use std::fmt;
use std::io::{Error as IoError, Read, Write};
use std::net::TcpStream;

pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:5037";

#[derive(Debug)]
pub enum AdbServerError {
    Io(IoError),
    Protocol(String),
    ServerError(String),
}

impl fmt::Display for AdbServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdbServerError::Io(e) => write!(f, "connection error: {e}"),
            AdbServerError::Protocol(s) => write!(f, "unexpected server reply: {s}"),
            AdbServerError::ServerError(s) => write!(f, "adb server error: {s}"),
        }
    }
}

impl std::error::Error for AdbServerError {}

impl From<IoError> for AdbServerError {
    fn from(e: IoError) -> Self {
        AdbServerError::Io(e)
    }
}

pub struct AdbServerClient {
    stream: TcpStream,
}

impl AdbServerClient {
    pub fn connect(addr: &str) -> Result<Self, AdbServerError> {
        Ok(AdbServerClient {
            stream: TcpStream::connect(addr)?,
        })
    }

    pub fn connect_default() -> Result<Self, AdbServerError> {
        Self::connect(DEFAULT_SERVER_ADDR)
    }

    pub fn server_version(&mut self) -> Result<u32, AdbServerError> {
        let body = self.host_service("version")?;
        u32::from_str_radix(body.trim(), 16)
            .map_err(|e| AdbServerError::Protocol(format!("bad version {body:?}: {e}")))
    }

    pub fn devices(&mut self) -> Result<String, AdbServerError> {
        self.host_service("devices")
    }

    pub fn host_service(&mut self, service: &str) -> Result<String, AdbServerError> {
        self.exchange(&format!("host:{service}"))
    }

    fn exchange(&mut self, request: &str) -> Result<String, AdbServerError> {
        let frame = format!("{:04x}{}", request.len(), request);
        self.stream.write_all(frame.as_bytes())?;

        match self.read_exact_str(4)?.as_str() {
            "OKAY" => {}
            "FAIL" => {
                let msg = self.read_length_prefixed()?;
                return Err(AdbServerError::ServerError(msg));
            }
            other => {
                return Err(AdbServerError::Protocol(format!("status {other:?}")));
            }
        }
        self.read_length_prefixed()
    }

    fn read_length_prefixed(&mut self) -> Result<String, AdbServerError> {
        let len_hex = self.read_exact_str(4)?;
        let len = usize::from_str_radix(&len_hex, 16)
            .map_err(|e| AdbServerError::Protocol(format!("bad length {len_hex:?}: {e}")))?;
        self.read_exact_str(len)
    }

    fn read_exact_str(&mut self, n: usize) -> Result<String, AdbServerError> {
        let mut buf = vec![0u8; n];
        self.stream.read_exact(&mut buf)?;
        String::from_utf8(buf).map_err(|e| AdbServerError::Protocol(format!("utf8: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn reads_okay_and_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut buf = [0u8; 16];
            sock.read_exact(&mut buf).unwrap();
            assert_eq!(String::from_utf8_lossy(&buf), "000chost:version");
            sock.write_all(b"OKAY0004dead").unwrap();
        });

        let mut client = AdbServerClient::connect(&addr).unwrap();
        assert_eq!(client.server_version().unwrap(), 0xdead);
        server.join().unwrap();
    }

    #[test]
    fn reports_fail_body_as_error() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut buf = [0u8; 16];
            sock.read_exact(&mut buf).unwrap();
            sock.write_all(b"FAIL0009no server").unwrap();
        });

        let mut client = AdbServerClient::connect(&addr).unwrap();
        let err = client.host_service("devices").unwrap_err();
        assert!(err.to_string().contains("no server"));
        server.join().unwrap();
    }
}
