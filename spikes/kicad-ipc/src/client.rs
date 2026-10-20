use crate::messages::{ApiRequest, ApiResponse, RequestHeader, StatusCode};
use crate::nng::ReqSocket;
use anyhow::{Context, Result, bail};
use prost::{Message, Name};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// KiCad answers on its GUI thread, so it may be busy with the user; the docs ask
/// clients to retry rather than fail.
const BUSY_RETRIES: u32 = 50;
const BUSY_PAUSE: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub struct Kicad {
    socket: ReqSocket,
    token: String,
    client_name: String,
}

impl Kicad {
    /// Connects where a plugin launched by KiCad is told to, else to KiCad's default socket.
    pub fn connect_from_environment() -> Result<Self> {
        let socket_path = std::env::var_os("KICAD_API_SOCKET")
            .map(socket_path)
            .unwrap_or_else(default_socket_path);
        let token = std::env::var("KICAD_API_TOKEN").unwrap_or_default();
        Self::connect(&socket_path, token)
    }

    pub fn connect(socket_path: &Path, token: String) -> Result<Self> {
        let socket =
            ReqSocket::connect(socket_path, Duration::from_secs(10)).with_context(|| {
                "KiCad's API is not reachable: is the PCB editor open, and is \
             Preferences → Plugins → Enable KiCad API ticked?"
            })?;
        Ok(Self {
            socket,
            token,
            client_name: format!("traxide-spike-{}", std::process::id()),
        })
    }

    pub fn call<Request: Name, Reply: Name + Default>(
        &mut self,
        request: &Request,
    ) -> Result<Reply> {
        let envelope = ApiRequest {
            header: Some(RequestHeader {
                kicad_token: self.token.clone(),
                client_name: self.client_name.clone(),
            }),
            message: Some(prost_types::Any::from_msg(request)?),
        }
        .encode_to_vec();
        for _ in 0..BUSY_RETRIES {
            let response = ApiResponse::decode(self.socket.request(&envelope)?.as_slice())?;
            let status = response.status.unwrap_or_default();
            match StatusCode::try_from(status.status) {
                Ok(StatusCode::Ok) => {
                    let message = response.message.unwrap_or_default();
                    return message.to_msg().with_context(|| {
                        format!("{} answered with {}", Request::NAME, message.type_url)
                    });
                }
                Ok(StatusCode::Busy | StatusCode::NotReady) => std::thread::sleep(BUSY_PAUSE),
                code => bail!(
                    "{} failed: {code:?} {}",
                    Request::NAME,
                    status.error_message
                ),
            }
        }
        bail!("KiCad stayed busy; {} was not handled", Request::NAME)
    }
}

/// KiCad hands plugins an NNG address (`ipc:///tmp/kicad/api.sock`), not a path.
fn socket_path(address: OsString) -> PathBuf {
    match address.to_str().and_then(|a| a.strip_prefix("ipc://")) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(address),
    }
}

/// `api.sock` in the temporary directory, as KiCad's documentation describes; a second
/// KiCad instance appends its process id, which only `KICAD_API_SOCKET` reveals.
fn default_socket_path() -> PathBuf {
    std::env::temp_dir().join("kicad").join("api.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_socket_path_from_the_nng_url_kicad_passes_to_plugins() {
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
}
