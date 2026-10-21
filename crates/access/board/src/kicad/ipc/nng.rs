//! The client half of NNG's REQ/REP protocol over its IPC transport, which is all
//! KiCad's API needs: written from the scalability protocols' wire format (an
//! 8-byte header exchange, then length-prefixed messages) so that no C library
//! has to be built.

use std::io::{self, Read, Write};
use std::path::Path;
use std::time::Duration;

const PROTOCOL_REQ: u16 = 0x30;
const PROTOCOL_REP: u16 = 0x31;
const IPC_MESSAGE: u8 = 0x01;
/// Marks the last (here the only) element of a request's backtrace.
const REQUEST_ID_END: u32 = 0x8000_0000;
/// No reply KiCad sends comes near this; a larger length means a broken stream.
const LARGEST_REPLY: u64 = 1 << 30;

#[derive(Debug)]
pub(super) struct ReqSocket<Stream> {
    stream: Stream,
    next_request_id: u32,
}

#[cfg(unix)]
pub(super) type Connection = ReqSocket<std::os::unix::net::UnixStream>;
#[cfg(windows)]
pub(super) type Connection = ReqSocket<std::fs::File>;

/// Connects to the REP socket NNG listens on at `path`, waiting at most
/// `timeout` for any reply.
#[cfg(unix)]
pub(super) fn connect(path: &Path, timeout: Duration) -> io::Result<Connection> {
    let stream = std::os::unix::net::UnixStream::connect(path)?;
    stream.set_read_timeout(Some(timeout))?;
    ReqSocket::handshake(stream)
}

/// NNG serves `ipc://NAME` on Windows as the named pipe `\\.\pipe\NAME`. Untested:
/// a pipe opened as a file cannot time out, so a hung KiCad hangs the client.
#[cfg(windows)]
pub(super) fn connect(path: &Path, _timeout: Duration) -> io::Result<Connection> {
    let pipe = Path::new(r"\\.\pipe\").join(path);
    let stream = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(pipe)?;
    ReqSocket::handshake(stream)
}

impl<Stream: Read + Write> ReqSocket<Stream> {
    fn handshake(mut stream: Stream) -> io::Result<Self> {
        stream.write_all(&sp_header(PROTOCOL_REQ))?;
        let mut peer = [0u8; 8];
        stream.read_exact(&mut peer)?;
        if peer != sp_header(PROTOCOL_REP) {
            return Err(broken(format!(
                "the peer is not an NNG REP socket: {peer:02x?}"
            )));
        }
        Ok(Self {
            stream,
            next_request_id: 1,
        })
    }

    /// Sends `body` and waits for the reply to it.
    pub fn request(&mut self, body: &[u8]) -> io::Result<Vec<u8>> {
        let id = (REQUEST_ID_END | self.next_request_id).to_be_bytes();
        self.next_request_id = (self.next_request_id + 1) & !REQUEST_ID_END;
        let mut payload = id.to_vec();
        payload.extend_from_slice(body);
        write_message(&mut self.stream, &payload)?;
        loop {
            let reply = read_message(&mut self.stream)?;
            match reply.strip_prefix(&id) {
                Some(body) => return Ok(body.to_vec()),
                // A reply to an earlier, timed-out request may still arrive; REQ drops it.
                None if reply.len() >= id.len() => {}
                None => return Err(broken("a reply without a request id".into())),
            }
        }
    }
}

fn sp_header(protocol: u16) -> [u8; 8] {
    let [high, low] = protocol.to_be_bytes();
    [0x00, b'S', b'P', 0x00, high, low, 0x00, 0x00]
}

fn write_message(stream: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    let mut frame = vec![IPC_MESSAGE];
    frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    frame.extend_from_slice(payload);
    stream.write_all(&frame)
}

fn read_message(stream: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut kind = [0u8; 1];
    stream.read_exact(&mut kind)?;
    if kind[0] != IPC_MESSAGE {
        return Err(broken(format!("unknown IPC frame type {}", kind[0])));
    }
    let mut length = [0u8; 8];
    stream.read_exact(&mut length)?;
    let length = u64::from_be_bytes(length);
    if length > LARGEST_REPLY {
        return Err(broken(format!("a reply of {length} bytes")));
    }
    let mut payload = vec![0u8; length as usize];
    stream.read_exact(&mut payload)?;
    Ok(payload)
}

fn broken(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// A REP peer for tests: answers each request with what `answer` makes of its
/// body, for as long as the client stays connected.
#[cfg(all(test, unix))]
pub(super) fn serve(
    listener: std::os::unix::net::UnixListener,
    mut answer: impl FnMut(&[u8]) -> Vec<u8> + Send + 'static,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("one client");
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).expect("the client's header");
        assert_eq!(header, sp_header(PROTOCOL_REQ));
        stream
            .write_all(&sp_header(PROTOCOL_REP))
            .expect("our header");
        while let Ok(request) = read_message(&mut stream) {
            let (id, body) = request.split_at(4);
            let mut reply = id.to_vec();
            reply.extend(answer(body));
            write_message(&mut stream, &reply).expect("the reply");
        }
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn frames_a_message_with_its_type_and_big_endian_length() {
        let mut frame = Vec::new();
        write_message(&mut frame, b"abc").expect("written");
        assert_eq!(frame, [1, 0, 0, 0, 0, 0, 0, 0, 3, b'a', b'b', b'c']);
    }

    #[test]
    fn exchanges_requests_with_a_rep_peer() {
        let folder = tempfile::tempdir().expect("a temporary folder");
        let path = folder.path().join("api.sock");
        let peer = serve(UnixListener::bind(&path).expect("bound"), |body| {
            [body, b"!"].concat()
        });
        let mut socket = connect(&path, Duration::from_secs(5)).expect("connected");
        assert_eq!(socket.request(b"ping").expect("first reply"), b"ping!");
        assert_eq!(socket.request(b"pong").expect("second reply"), b"pong!");
        drop(socket);
        peer.join().expect("the peer finished");
    }

    /// A peer that plays back recorded replies and ignores requests.
    #[derive(Debug)]
    struct LateReplies {
        replies: io::Cursor<Vec<u8>>,
    }

    impl Read for LateReplies {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.replies.read(buffer)
        }
    }

    impl Write for LateReplies {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn drops_a_late_reply_to_an_earlier_request() {
        let mut replies = sp_header(PROTOCOL_REP).to_vec();
        for (id, body) in [(0x8000_0007_u32, b"old"), (0x8000_0001, b"new")] {
            write_message(&mut replies, &[&id.to_be_bytes()[..], body].concat()).expect("framed");
        }
        let mut socket = ReqSocket::handshake(LateReplies {
            replies: io::Cursor::new(replies),
        })
        .expect("handshake");
        assert_eq!(socket.request(b"ask").expect("the reply"), b"new");
    }

    #[test]
    fn refuses_a_peer_that_is_not_a_rep_socket() {
        let peer = LateReplies {
            replies: io::Cursor::new(sp_header(PROTOCOL_REQ).to_vec()),
        };
        let error = ReqSocket::handshake(peer).expect_err("a REQ peer");
        assert!(
            error.to_string().contains("not an NNG REP socket"),
            "{error}"
        );
    }
}
