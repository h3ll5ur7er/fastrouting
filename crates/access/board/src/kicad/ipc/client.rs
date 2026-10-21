use super::messages::{
    Any, ApiRequest, ApiResponse, DOCUMENT_PCB, Document, GetOpenDocuments,
    GetOpenDocumentsResponse, RequestHeader, StatusCode,
};
use super::nng::{self, Connection};
use crate::BoardError;
use prost::{Message, Name};
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// How to reach KiCad, as the process's environment says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Environment {
    pub socket: PathBuf,
    /// Proves to KiCad that a plugin is talking to the instance that started it.
    pub token: String,
    /// The project's folder, which KiCad passes to plugins.
    pub project_folder: Option<PathBuf>,
}

impl Environment {
    /// `KICAD_API_SOCKET`, `KICAD_API_TOKEN` and `KIPRJMOD`, which KiCad sets for
    /// plugins; KiCad's default socket for a traxide started by hand.
    pub fn of_process() -> Self {
        Self {
            socket: std::env::var_os("KICAD_API_SOCKET")
                .map(socket_path)
                .unwrap_or_else(default_socket_path),
            token: std::env::var("KICAD_API_TOKEN").unwrap_or_default(),
            project_folder: std::env::var_os("KIPRJMOD")
                .filter(|folder| !folder.is_empty())
                .map(PathBuf::from),
        }
    }
}

/// KiCad hands plugins an NNG address (`ipc:///tmp/kicad/api.sock`), not a path.
fn socket_path(address: OsString) -> PathBuf {
    match address.to_str().and_then(|a| a.strip_prefix("ipc://")) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(address),
    }
}

/// `kicad/api.sock` in the temporary folder, as KiCad's documentation describes;
/// a second KiCad appends its process id, which only `KICAD_API_SOCKET` reveals.
fn default_socket_path() -> PathBuf {
    std::env::temp_dir().join("kicad").join("api.sock")
}

/// KiCad answers on its GUI thread: while a dialog is open it is busy, and just
/// after starting it is not ready. Its documentation asks clients to retry.
const PATIENCE: Duration = Duration::from_secs(10);
const RETRY_PAUSE: Duration = Duration::from_millis(100);
/// The longest KiCad may take over one request, such as refilling large zones.
const REPLY_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub(crate) struct Kicad {
    connection: Connection,
    token: String,
    client_name: String,
}

impl Kicad {
    pub fn connect(environment: &Environment) -> Result<Self, BoardError> {
        let connection = nng::connect(&environment.socket, REPLY_TIMEOUT).map_err(|error| {
            BoardError::KicadUnreachable {
                socket: environment.socket.clone(),
                reason: error.to_string(),
            }
        })?;
        Ok(Self {
            connection,
            token: environment.token.clone(),
            client_name: format!("traxide-{}", std::process::id()),
        })
    }

    /// Sends `request` and returns KiCad's reply, retrying while KiCad is busy.
    pub fn call<Request, Reply>(&mut self, request: &Request) -> Result<Reply, BoardError>
    where
        Request: Message + Name,
        Reply: Message + Name + Default,
    {
        let envelope = ApiRequest {
            header: Some(RequestHeader {
                kicad_token: self.token.clone(),
                client_name: self.client_name.clone(),
            }),
            message: Some(Any::of(request)),
        }
        .encode_to_vec();
        let failed = |message: String| BoardError::Kicad(format!("{}: {message}", Request::NAME));
        let started = Instant::now();
        loop {
            let reply = self
                .connection
                .request(&envelope)
                .map_err(|error| failed(error.to_string()))?;
            let response = ApiResponse::decode(reply.as_slice())
                .map_err(|error| failed(format!("an unreadable reply: {error}")))?;
            let status = response.status.unwrap_or_default();
            match StatusCode::try_from(status.status).unwrap_or(StatusCode::Unknown) {
                StatusCode::Ok => {
                    let message = response.message.unwrap_or_default();
                    return message.to::<Reply>().ok_or_else(|| {
                        failed(format!(
                            "expected {}, KiCad answered {}",
                            Reply::NAME,
                            message.type_url
                        ))
                    });
                }
                StatusCode::Busy | StatusCode::NotReady if started.elapsed() < PATIENCE => {
                    std::thread::sleep(RETRY_PAUSE);
                }
                StatusCode::Busy | StatusCode::NotReady => {
                    return Err(failed(format!(
                        "KiCad stayed busy for {} s; close any dialog open in KiCad and try again",
                        PATIENCE.as_secs()
                    )));
                }
                StatusCode::TokenMismatch => {
                    return Err(failed(
                        "KiCad refused the token: the KiCad that started traxide has closed".into(),
                    ));
                }
                code => {
                    return Err(failed(format!(
                        "KiCad answered {code:?}: {}",
                        status.error_message
                    )));
                }
            }
        }
    }

    /// The board open in KiCad's PCB editor.
    pub fn open_board(&mut self) -> Result<Document, BoardError> {
        let open: GetOpenDocumentsResponse = self.call(&GetOpenDocuments {
            document_type: DOCUMENT_PCB,
        })?;
        open.documents
            .into_iter()
            .next()
            .ok_or_else(|| BoardError::Kicad("no board is open in KiCad's PCB editor".into()))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::messages::{GetNets, NetsResponse};
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn takes_the_socket_path_from_the_nng_address_kicad_passes_to_plugins() {
        assert_eq!(
            socket_path(OsString::from("ipc:///tmp/kicad/api.sock")),
            PathBuf::from("/tmp/kicad/api.sock")
        );
    }

    #[test]
    fn takes_a_bare_socket_path_as_it_is() {
        assert_eq!(
            socket_path(OsString::from("/tmp/kicad/api-42.sock")),
            PathBuf::from("/tmp/kicad/api-42.sock")
        );
    }

    fn reply(status: StatusCode, message: Option<Any>) -> Vec<u8> {
        ApiResponse {
            status: Some(super::super::messages::ResponseStatus {
                status: status as i32,
                error_message: "the reason".into(),
            }),
            message,
        }
        .encode_to_vec()
    }

    /// A KiCad that answers each request with the next of `replies`.
    fn kicad_answering(replies: Vec<Vec<u8>>) -> (tempfile::TempDir, Environment) {
        let folder = tempfile::tempdir().expect("a temporary folder");
        let socket = folder.path().join("api.sock");
        let mut replies = replies.into_iter();
        nng::serve(UnixListener::bind(&socket).expect("bound"), move |_| {
            replies.next().expect("no more requests than replies")
        });
        let environment = Environment {
            socket,
            token: String::new(),
            project_folder: None,
        };
        (folder, environment)
    }

    #[test]
    fn retries_while_kicad_is_busy() {
        let nets = NetsResponse::default();
        let (_folder, environment) = kicad_answering(vec![
            reply(StatusCode::NotReady, None),
            reply(StatusCode::Busy, None),
            reply(StatusCode::Ok, Some(Any::of(&nets))),
        ]);
        let mut kicad = Kicad::connect(&environment).expect("connected");
        let answer: NetsResponse = kicad.call(&GetNets::default()).expect("answered");
        assert_eq!(answer, nets);
    }

    #[test]
    fn says_why_kicad_refused_a_request() {
        let (_folder, environment) = kicad_answering(vec![reply(StatusCode::BadRequest, None)]);
        let mut kicad = Kicad::connect(&environment).expect("connected");
        let error = kicad
            .call::<_, NetsResponse>(&GetNets::default())
            .expect_err("refused");
        assert_eq!(
            error.to_string(),
            "KiCad: GetNets: KiCad answered BadRequest: the reason"
        );
    }

    #[test]
    fn refuses_a_reply_of_another_type() {
        let (_folder, environment) = kicad_answering(vec![reply(
            StatusCode::Ok,
            Some(Any::of(&GetNets::default())),
        )]);
        let mut kicad = Kicad::connect(&environment).expect("connected");
        let error = kicad
            .call::<_, NetsResponse>(&GetNets::default())
            .expect_err("the wrong type");
        assert!(
            error.to_string().contains("expected NetsResponse"),
            "{error}"
        );
    }

    #[test]
    fn explains_how_to_enable_the_api_when_kicad_is_not_listening() {
        let folder = tempfile::tempdir().expect("a temporary folder");
        let environment = Environment {
            socket: folder.path().join("api.sock"),
            token: String::new(),
            project_folder: None,
        };
        let error = Kicad::connect(&environment).expect_err("nobody listens");
        let message = error.to_string();
        assert!(message.contains("Enable KiCad API"), "{message}");
        assert!(message.contains("api.sock"), "{message}");
    }
}
