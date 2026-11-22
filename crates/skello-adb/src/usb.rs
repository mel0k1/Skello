use std::fmt;
use std::io::{Read as _, Write as _};
use std::path::Path;
use std::time::{Duration, Instant};

use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, In, Out};
use nusb::{Device, Interface, MaybeFuture};

use crate::auth::{AdbKeys, AuthError};
use crate::{
    Message, AUTH_RSAPUBLICKEY, AUTH_SIGNATURE, AUTH_TOKEN, A_AUTH, A_CLSE, A_CNXN, A_OKAY, A_OPEN,
    A_VERSION, A_WRTE, HEADER_SIZE, MAX_PAYLOAD, TOKEN_SIZE,
};

/// Android adb interface class / subclass / protocol.
pub const ADB_CLASS: u8 = 0xFF;
pub const ADB_SUBCLASS: u8 = 0x42;
pub const ADB_PROTOCOL: u8 = 0x01;

/// Time to wait for the user to accept the debugging dialog on the phone.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(20);
/// Timeout of one USB transfer while waiting for protocol messages.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum AdbError {
    Enumeration(String),
    NotFound(String),
    Endpoints(String),
    Io(String),
    Protocol(String),
    Auth(String),
    Unauthorized,
    Timeout,
}

impl fmt::Display for AdbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdbError::Enumeration(s) => write!(f, "usb enumeration failed: {s}"),
            AdbError::NotFound(s) => write!(f, "adb device not found: {s}"),
            AdbError::Endpoints(s) => write!(f, "adb endpoints missing: {s}"),
            AdbError::Io(s) => write!(f, "usb transfer failed: {s}"),
            AdbError::Protocol(s) => write!(f, "adb protocol error: {s}"),
            AdbError::Auth(s) => write!(f, "adb auth error: {s}"),
            AdbError::Unauthorized => write!(f, "device did not accept the adb key"),
            AdbError::Timeout => write!(f, "device did not answer in time"),
        }
    }
}

impl std::error::Error for AdbError {}

impl From<AuthError> for AdbError {
    fn from(e: AuthError) -> Self {
        AdbError::Auth(e.to_string())
    }
}

/// Serial numbers of USB devices exposing an adb interface.
pub fn adb_usb_serials() -> Result<Vec<String>, AdbError> {
    let list = nusb::list_devices()
        .wait()
        .map_err(|e| AdbError::Enumeration(e.to_string()))?;
    Ok(list
        .filter(|info: &nusb::DeviceInfo| has_adb_interface(info))
        .filter_map(|info| info.serial_number().map(str::to_owned))
        .collect())
}

fn has_adb_interface(info: &nusb::DeviceInfo) -> bool {
    info.interfaces().any(|iface| {
        iface.class() == ADB_CLASS
            && iface.subclass() == ADB_SUBCLASS
            && iface.protocol() == ADB_PROTOCOL
    })
}

/// An open adb connection to one device over USB, without the adb server.
pub struct UsbAdb {
    write: EndpointWrite<Bulk>,
    read: EndpointRead<Bulk>,
    maxdata: usize,
    local_id: u32,
    remote_id: Option<u32>,
    _device: Device,
    _interface: Interface,
}

impl UsbAdb {
    pub fn connect(serial: &str, keys: &AdbKeys) -> Result<Self, AdbError> {
        let list = nusb::list_devices()
            .wait()
            .map_err(|e| AdbError::Enumeration(e.to_string()))?;
        let mut match_info = None;
        for info in list {
            if info.serial_number() != Some(serial) {
                continue;
            }
            let adb_iface = info
                .interfaces()
                .find(|iface| {
                    iface.class() == ADB_CLASS
                        && iface.subclass() == ADB_SUBCLASS
                        && iface.protocol() == ADB_PROTOCOL
                })
                .cloned();
            if let Some(iface) = adb_iface {
                match_info = Some((info, iface.interface_number()));
                break;
            }
        }
        let Some((info, interface_number)) = match_info else {
            return Err(AdbError::NotFound(serial.to_owned()));
        };
        let device = info
            .open()
            .wait()
            .map_err(|e| AdbError::Io(e.to_string()))?;
        let interface = device
            .detach_and_claim_interface(interface_number)
            .wait()
            .map_err(|e| AdbError::Io(e.to_string()))?;
        let mut out_addr = None;
        let mut in_addr = None;
        let mut max_packet = 512usize;
        if let Some(desc) = interface.descriptor() {
            for ep in desc.endpoints() {
                match ep.direction() {
                    nusb::transfer::Direction::Out => out_addr = Some(ep.address()),
                    nusb::transfer::Direction::In => {
                        in_addr = Some(ep.address());
                        max_packet = ep.max_packet_size();
                    }
                }
            }
        }
        let (Some(out_addr), Some(in_addr)) = (out_addr, in_addr) else {
            return Err(AdbError::Endpoints(serial.to_owned()));
        };
        let write = interface
            .endpoint::<Bulk, Out>(out_addr)
            .map_err(|e| AdbError::Endpoints(e.to_string()))?
            .writer(max_packet);
        let mut read = interface
            .endpoint::<Bulk, In>(in_addr)
            .map_err(|e| AdbError::Endpoints(e.to_string()))?
            .reader(max_packet);
        read.set_read_timeout(READ_TIMEOUT);
        let mut adb = Self {
            write,
            read,
            maxdata: MAX_PAYLOAD as usize,
            local_id: 1,
            remote_id: None,
            _device: device,
            _interface: interface,
        };
        adb.handshake(keys)?;
        Ok(adb)
    }

    fn read_exact_usb(&mut self, buf: &mut [u8]) -> Result<(), AdbError> {
        let mut filled = 0;
        while filled < buf.len() {
            let n = self
                .read
                .read(&mut buf[filled..])
                .map_err(|e| AdbError::Io(e.to_string()))?;
            if n == 0 {
                return Err(AdbError::Io("device closed the endpoint".into()));
            }
            filled += n;
        }
        Ok(())
    }

    pub fn read_message(&mut self) -> Result<(Message, Vec<u8>), AdbError> {
        let mut header = [0u8; HEADER_SIZE];
        self.read_exact_usb(&mut header)?;
        let msg = Message::decode(&header).map_err(|e| AdbError::Protocol(e.to_string()))?;
        if msg.data_length as usize > self.maxdata {
            return Err(AdbError::Protocol(format!(
                "payload {} bytes exceeds negotiated maxdata {}",
                msg.data_length, self.maxdata
            )));
        }
        let mut data = vec![0u8; msg.data_length as usize];
        if !data.is_empty() {
            self.read_exact_usb(&mut data)?;
        }
        Ok((msg, data))
    }

    pub fn write_message(
        &mut self,
        command: u32,
        arg0: u32,
        arg1: u32,
        payload: &[u8],
    ) -> Result<(), AdbError> {
        let msg = Message::new(command, arg0, arg1, payload);
        self.write
            .write_all(&msg.encode())
            .map_err(|e| AdbError::Io(e.to_string()))?;
        if !payload.is_empty() {
            self.write
                .write_all(payload)
                .map_err(|e| AdbError::Io(e.to_string()))?;
        }
        Ok(())
    }

    fn handshake(&mut self, keys: &AdbKeys) -> Result<(), AdbError> {
        let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
        self.write_message(A_CNXN, A_VERSION, MAX_PAYLOAD, b"host::skello\0")?;
        let mut signature_sent = false;
        let mut pubkey_sent = false;
        loop {
            match self.read_message() {
                Ok((msg, data)) => match msg.command {
                    A_CNXN => {
                        self.maxdata = (msg.arg1 as usize).clamp(4096, MAX_PAYLOAD as usize);
                        return Ok(());
                    }
                    A_AUTH => {
                        if msg.arg0 != AUTH_TOKEN || data.len() != TOKEN_SIZE {
                            return Err(AdbError::Protocol(format!(
                                "unexpected AUTH type {} with {} bytes",
                                msg.arg0,
                                data.len()
                            )));
                        }
                        if signature_sent && !pubkey_sent {
                            let key_payload = keys.public_key_payload();
                            self.write_message(A_AUTH, AUTH_RSAPUBLICKEY, 0, &key_payload)?;
                            pubkey_sent = true;
                        } else {
                            let signature = keys.sign_token(&data);
                            self.write_message(A_AUTH, AUTH_SIGNATURE, 0, &signature)?;
                            signature_sent = true;
                        }
                    }
                    _ => {
                        return Err(AdbError::Protocol(format!(
                            "unexpected {} during handshake",
                            msg.command_name()
                        )))
                    }
                },
                Err(AdbError::Io(_)) => {
                    // A per-read timeout inside the handshake window
                    // only means the device has not answered yet: the
                    // user may still be looking at the auth dialog on
                    // the phone. Only the deadline ends the wait.
                    if Instant::now() < deadline {
                        continue;
                    }
                    return Err(if pubkey_sent {
                        AdbError::Unauthorized
                    } else {
                        AdbError::Timeout
                    });
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Opens a service stream ("shell:...", "reboot", ...) and waits for
    /// the device acknowledgement.
    pub fn open_service(&mut self, service: &str) -> Result<(), AdbError> {
        let mut payload = service.as_bytes().to_vec();
        payload.push(0);
        self.write_message(A_OPEN, self.local_id, 0, &payload)?;
        let deadline = Instant::now() + READ_TIMEOUT * 2;
        loop {
            let timed_out = Instant::now() > deadline;
            match self.read_message() {
                Ok((msg, _)) => match msg.command {
                    A_OKAY if msg.arg1 == self.local_id => {
                        self.remote_id = Some(msg.arg0);
                        return Ok(());
                    }
                    A_CLSE if msg.arg1 == self.local_id => {
                        return Err(AdbError::Protocol(format!(
                            "device refused service {service:?}"
                        )));
                    }
                    A_WRTE if msg.arg1 == self.local_id => {
                        self.write_message(A_OKAY, self.local_id, msg.arg0, &[])?;
                    }
                    _ => {}
                },
                Err(AdbError::Io(_)) if timed_out => {
                    return Err(AdbError::Timeout);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Reads the service stream until the device closes it, acknowledging
    /// every WRTE message.
    pub fn read_until_close(&mut self) -> Result<Vec<u8>, AdbError> {
        let mut out = Vec::new();
        loop {
            let (msg, data) = self.read_message()?;
            match msg.command {
                A_WRTE if msg.arg1 == self.local_id => {
                    out.extend_from_slice(&data);
                    self.write_message(A_OKAY, self.local_id, msg.arg0, &[])?;
                }
                A_CLSE if msg.arg1 == self.local_id => {
                    self.remote_id = None;
                    return Ok(out);
                }
                A_OKAY if msg.arg1 == self.local_id => {}
                _ => {}
            }
        }
    }

    /// Runs a shell command and returns its combined output.
    pub fn shell(&mut self, command: &str) -> Result<String, AdbError> {
        self.open_service(&format!("shell:{command}"))?;
        let out = self.read_until_close()?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    /// Runs the reboot service; the device reboots right after acknowledging.
    pub fn reboot(&mut self, target: Option<&str>) -> Result<(), AdbError> {
        let service = match target {
            None | Some("") => "reboot".to_owned(),
            Some(t) => format!("reboot,{t}"),
        };
        self.open_service(&service)?;
        let _ = self.close();
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), AdbError> {
        if let Some(remote) = self.remote_id.take() {
            self.write_message(A_CLSE, self.local_id, remote, &[])?;
        }
        Ok(())
    }
}

/// Handshake result for one device: "device", "unauthorized" or "offline".
pub fn adb_usb_device_state(serial: &str, keys: &AdbKeys) -> String {
    match UsbAdb::connect(serial, keys) {
        Ok(mut adb) => {
            let _ = adb.close();
            "device".to_owned()
        }
        Err(AdbError::Unauthorized) => "unauthorized".to_owned(),
        Err(_) => "offline".to_owned(),
    }
}

/// Loads (or creates) the adb host keys in `keys_dir`.
pub fn load_adb_keys(keys_dir: &Path) -> Result<AdbKeys, AdbError> {
    AdbKeys::load_or_generate(keys_dir).map_err(AdbError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_frame_has_host_identity() {
        let msg = Message::new(A_CNXN, A_VERSION, MAX_PAYLOAD, b"host::skello\0");
        let frame = msg.encode();
        let decoded = Message::decode(&frame).unwrap();
        assert_eq!(decoded.command, A_CNXN);
        assert_eq!(decoded.data_length, 13);
        assert_eq!(decoded.data_check, crate::data_checksum(b"host::skello\0"));
    }

    #[test]
    fn reboot_service_string_variants() {
        let make = |target: Option<&str>| match target {
            None | Some("") => "reboot".to_owned(),
            Some(t) => format!("reboot,{t}"),
        };
        assert_eq!(make(None), "reboot");
        assert_eq!(make(Some("")), "reboot");
        assert_eq!(make(Some("bootloader")), "reboot,bootloader");
    }

    #[test]
    fn open_payload_is_null_terminated() {
        let mut payload = b"shell:getprop ro.product.model".to_vec();
        payload.push(0);
        let msg = Message::new(A_OPEN, 1, 0, &payload);
        assert_eq!(msg.data_length as usize, payload.len());
        assert_eq!(payload.last(), Some(&0));
    }

    #[test]
    fn usb_error_messages_are_readable() {
        assert!(AdbError::Unauthorized.to_string().contains("adb key"));
        assert!(AdbError::NotFound("ABC".into()).to_string().contains("ABC"));
    }
}
