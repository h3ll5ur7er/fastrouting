//! The client half of NNG's REQ/REP protocol over its IPC transport, which is all
//! KiCad's API needs: written from the scalability protocols' wire format
//! (an 8-byte SP header exchange, then length-prefixed messages) so that no C
//! library has to be built.

use anyhow::{Context, Result, ensure};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

const PROTOCOL_REQ: u16 = 0x30;
const PROTOCOL_REP: u16 = 0x31;
const IPC_MESSAGE: u8 = 0x01;
/// Marks the last (here the only) element of a request's backtrace.
const REQUEST_ID_END: u32 = 0x8000_0000;

#[derive(Debug)]
pub struct ReqSocket {
    stream: UnixStream,
    next_request_id: u32,
}

impl ReqSocket {
    pub fn connect(path: &Path, timeout: Duration) -> Result<Self> {
        let mut stream = UnixStream::connect(path)
            .with_context(|| format!("connecting to {}", path.display()))?;
        stream.set_read_timeout(Some(timeout))?;
        stream.write_all(&sp_header(PROTOCOL_REQ))?;
        let mut peer = [0u8; 8];
        stream
            .read_exact(&mut peer)
            .context("reading the peer's SP header")?;
        ensure!(
            peer == sp_header(PROTOCOL_REP),
            "the peer is not an NNG REP socket: {peer:02x?}"
        );
        Ok(Self {
            stream,
            next_request_id: 1,
        })
    }

    pub fn request(&mut self, body: &[u8]) -> Result<Vec<u8>> {
        let id = REQUEST_ID_END | self.next_request_id;
        self.next_request_id = (self.next_request_id + 1) & !REQUEST_ID_END;
        let mut payload = id.to_be_bytes().to_vec();
        payload.extend_from_slice(body);
        write_message(&mut self.stream, &payload)?;
        loop {
            let reply = read_message(&mut self.stream)?;
            ensure!(reply.len() >= 4, "a reply without a request id");
            // A reply to an earlier, timed-out request may still arrive; REQ drops it.
            if reply[..4] == id.to_be_bytes() {
                return Ok(reply[4..].to_vec());
            }
        }
    }
}

fn sp_header(protocol: u16) -> [u8; 8] {
    let [high, low] = protocol.to_be_bytes();
    [0x00, b'S', b'P', 0x00, high, low, 0x00, 0x00]
}

fn write_message(stream: &mut impl Write, payload: &[u8]) -> Result<()> {
    let mut frame = vec![IPC_MESSAGE];
    frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    frame.extend_from_slice(payload);
    stream.write_all(&frame)?;
    Ok(())
}

fn read_message(stream: &mut impl Read) -> Result<Vec<u8>> {
    let mut head = [0u8; 9];
    stream
        .read_exact(&mut head)
        .context("waiting for KiCad's reply")?;
    ensure!(head[0] == IPC_MESSAGE, "unknown IPC frame type {}", head[0]);
    let mut length = [0u8; 8];
    length.copy_from_slice(&head[1..]);
    let mut payload = vec![0u8; usize::try_from(u64::from_be_bytes(length))?];
    stream.read_exact(&mut payload)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::thread;

    fn echo_rep_peer(path: &Path, replies: usize) -> thread::JoinHandle<()> {
        let listener = UnixListener::bind(path).expect("a fresh socket path");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("one client");
            let mut header = [0u8; 8];
            stream
                .read_exact(&mut header)
                .expect("the client's SP header");
            assert_eq!(header, sp_header(PROTOCOL_REQ));
            stream
                .write_all(&sp_header(PROTOCOL_REP))
                .expect("our header");
            for _ in 0..replies {
                let mut request = read_message(&mut stream).expect("a request");
                request.extend_from_slice(b"!");
                write_message(&mut stream, &request).expect("the reply");
            }
        })
    }

    #[test]
    fn exchanges_requests_with_a_rep_peer() {
        let dir = std::env::temp_dir().join(format!("traxide-nng-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temporary directory");
        let path = dir.join("api.sock");
        let _ = std::fs::remove_file(&path);
        let peer = echo_rep_peer(&path, 2);

        let mut socket = ReqSocket::connect(&path, Duration::from_secs(5)).expect("connected");
        assert_eq!(socket.request(b"ping").expect("first reply"), b"ping!");
        assert_eq!(socket.request(b"pong").expect("second reply"), b"pong!");
        peer.join().expect("the peer finished");
    }

    #[test]
    fn frames_a_message_with_its_type_and_big_endian_length() {
        let mut frame = Vec::new();
        write_message(&mut frame, b"abc").expect("written");
        assert_eq!(frame, [1, 0, 0, 0, 0, 0, 0, 0, 3, b'a', b'b', b'c']);
    }
}
