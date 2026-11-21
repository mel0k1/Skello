use std::fmt;
use std::io::{Read as _, Write as _};
use std::time::Duration;

use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, In, Out};
use nusb::{Device, Interface, MaybeFuture};

use crate::{download as download_cmd, parse_response, Response, RESP_TIMEOUT_SECS};

/// Android fastboot interface class / subclass / protocol from the
/// Google USB driver definitions.
pub const FASTBOOT_CLASS: u8 = 0xFF;
pub const FASTBOOT_SUBCLASS: u8 = 0x42;
pub const FASTBOOT_PROTOCOL: u8 = 0x03;

/// Timeout for one response packet read.
const READ_TIMEOUT: Duration = Duration::from_secs(RESP_TIMEOUT_SECS);

/// Size of one payload write chunk; the kernel splits it into
/// max-packet-size USB transfers.
const WRITE_CHUNK: usize = 16 * 1024;

#[derive(Debug)]
pub enum UsbError {
    Enumeration(String),
    NotFound(String),
    Endpoints(String),
    Io(String),
    Device(String),
}

impl fmt::Display for UsbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbError::Enumeration(s) => write!(f, "usb enumeration failed: {s}"),
            UsbError::NotFound(s) => write!(f, "fastboot device not found: {s}"),
            UsbError::Endpoints(s) => write!(f, "fastboot endpoints missing: {s}"),
            UsbError::Io(s) => write!(f, "usb transfer failed: {s}"),
            UsbError::Device(s) => write!(f, "device refused: {s}"),
        }
    }
}

impl std::error::Error for UsbError {}

/// Serial numbers of USB devices exposing the fastboot interface.
pub fn usb_fastboot_devices() -> Result<Vec<String>, UsbError> {
    let list = nusb::list_devices()
        .wait()
        .map_err(|e| UsbError::Enumeration(e.to_string()))?;
    Ok(list
        .filter(|info: &nusb::DeviceInfo| has_fastboot_interface(info))
        .filter_map(|info| info.serial_number().map(str::to_owned))
        .collect())
}

fn has_fastboot_interface(info: &nusb::DeviceInfo) -> bool {
    info.interfaces().any(|iface| {
        iface.class() == FASTBOOT_CLASS
            && iface.subclass() == FASTBOOT_SUBCLASS
            && iface.protocol() == FASTBOOT_PROTOCOL
    })
}

/// An open fastboot session over USB bulk endpoints.
pub struct UsbFastboot {
    write: EndpointWrite<Bulk>,
    read: EndpointRead<Bulk>,
    _device: Device,
    _interface: Interface,
}

impl UsbFastboot {
    pub fn connect(serial: &str) -> Result<Self, UsbError> {
        let list = nusb::list_devices()
            .wait()
            .map_err(|e| UsbError::Enumeration(e.to_string()))?;
        let mut match_info = None;
        for info in list {
            if info.serial_number() != Some(serial) {
                continue;
            }
            let fastboot_iface = info
                .interfaces()
                .find(|iface| {
                    iface.class() == FASTBOOT_CLASS
                        && iface.subclass() == FASTBOOT_SUBCLASS
                        && iface.protocol() == FASTBOOT_PROTOCOL
                })
                .cloned();
            if let Some(iface) = fastboot_iface {
                match_info = Some((info, iface.interface_number()));
                break;
            }
        }
        let Some((info, interface_number)) = match_info else {
            return Err(UsbError::NotFound(serial.to_owned()));
        };
        let device = info
            .open()
            .wait()
            .map_err(|e| UsbError::Io(e.to_string()))?;
        let interface = device
            .detach_and_claim_interface(interface_number)
            .wait()
            .map_err(|e| UsbError::Io(e.to_string()))?;
        let mut out_addr = None;
        let mut in_addr = None;
        let mut max_packet = 512;
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
            return Err(UsbError::Endpoints(serial.to_owned()));
        };
        let write = interface
            .endpoint::<Bulk, Out>(out_addr)
            .map_err(|e| UsbError::Endpoints(e.to_string()))?
            .writer(WRITE_CHUNK);
        let mut read = interface
            .endpoint::<Bulk, In>(in_addr)
            .map_err(|e| UsbError::Endpoints(e.to_string()))?
            .reader(max_packet);
        read.set_read_timeout(READ_TIMEOUT);
        Ok(Self {
            write,
            read,
            _device: device,
            _interface: interface,
        })
    }

    /// Overrides the response read timeout, e.g. for `flashing:unlock`
    /// where the device waits for an on-phone confirmation.
    pub fn set_read_timeout(&mut self, timeout: Duration) {
        self.read.set_read_timeout(timeout);
    }

    /// Runs a command to completion collecting every packet as raw
    /// (header, body) pairs: vendor bootloaders answer with
    /// non-standard headers (Unisoc `SN` for get_identifier_token).
    /// OKAY closes the loop, FAIL errors out.
    pub fn command_raw(&mut self, cmd: &str) -> Result<(String, Vec<(String, String)>), UsbError> {
        let mut frame = cmd.as_bytes().to_vec();
        frame.push(0);
        self.send_raw(&frame)?;
        let mut raw = Vec::new();
        loop {
            let mut buf = vec![0u8; crate::RESPONSE_SZ];
            let n = self
                .read
                .read(&mut buf)
                .map_err(|e| UsbError::Io(e.to_string()))?;
            if n == 0 {
                return Err(UsbError::Io("empty response packet".into()));
            }
            let packet = &buf[..n];
            let head: String = String::from_utf8_lossy(&packet[..packet.len().min(4)])
                .trim_end_matches('\0')
                .to_owned();
            if packet.len() >= 4
                && matches!(head.as_str(), "OKAY" | "FAIL" | "INFO" | "TEXT" | "DATA")
            {
                match parse_response(packet).map_err(|e| UsbError::Device(e.to_string()))? {
                    Response::Ok(payload) => return Ok((payload, raw)),
                    Response::Fail(payload) => return Err(UsbError::Device(payload)),
                    Response::Info(text) => raw.push(("INFO".into(), text)),
                    Response::Text(text) => raw.push(("TEXT".into(), text)),
                    Response::Data(_) => {
                        return Err(UsbError::Device("unexpected DATA response".into()))
                    }
                }
            } else {
                let body = String::from_utf8_lossy(packet)
                    .trim_end_matches(['\0', '\n', '\r'])
                    .to_owned();
                raw.push((head, body));
            }
        }
    }

    fn send_raw(&mut self, data: &[u8]) -> Result<(), UsbError> {
        self.write
            .write_all(data)
            .map_err(|e| UsbError::Io(e.to_string()))
    }

    fn read_packet(&mut self) -> Result<Response, UsbError> {
        let mut buf = vec![0u8; crate::RESPONSE_SZ];
        let n = self
            .read
            .read(&mut buf)
            .map_err(|e| UsbError::Io(e.to_string()))?;
        if n == 0 {
            return Err(UsbError::Io("empty response packet".into()));
        }
        parse_response(&buf[..n]).map_err(|e| UsbError::Device(e.to_string()))
    }

    /// Runs a command to completion, collecting INFO/TEXT lines.
    /// Returns the final payload on OKAY, an error on FAIL.
    pub fn command(&mut self, cmd: &str) -> Result<(String, Vec<String>), UsbError> {
        let mut frame = cmd.as_bytes().to_vec();
        frame.push(0);
        self.send_raw(&frame)?;
        let mut infos = Vec::new();
        loop {
            match self.read_packet()? {
                Response::Ok(payload) => return Ok((payload, infos)),
                Response::Fail(payload) => return Err(UsbError::Device(payload)),
                Response::Info(text) => infos.push(text),
                Response::Text(text) => infos.push(text),
                Response::Data(_) => {
                    return Err(UsbError::Device("unexpected DATA response".into()))
                }
            }
        }
    }

    /// Runs `download:<size>` and streams the payload, reporting progress.
    /// Then continues the transaction to the final OKAY / FAIL.
    pub fn download(
        &mut self,
        payload: &[u8],
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<(String, Vec<String>), UsbError> {
        let size = u32::try_from(payload.len())
            .map_err(|_| UsbError::Device("payload too large".into()))?;
        let mut frame = download_cmd(size).into_bytes();
        frame.push(0);
        self.send_raw(&frame)?;
        loop {
            match self.read_packet()? {
                Response::Data(size) => {
                    if size as usize != payload.len() {
                        return Err(UsbError::Device(format!(
                            "device expects {size} bytes, payload is {}",
                            payload.len()
                        )));
                    }
                    self.send_payload(payload, progress)?;
                }
                Response::Ok(rest) => return Ok((rest, Vec::new())),
                Response::Fail(rest) => return Err(UsbError::Device(rest)),
                Response::Info(_text) => {}
                Response::Text(_text) => {}
            }
        }
    }

    fn send_payload(
        &mut self,
        payload: &[u8],
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<(), UsbError> {
        let total = payload.len();
        for start in (0..total).step_by(WRITE_CHUNK) {
            let end = (start + WRITE_CHUNK).min(total);
            self.send_raw(&payload[start..end])?;
            progress(end as u64, total as u64);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_download_size;

    #[test]
    fn command_frame_appends_zero_byte() {
        let mut frame = crate::flash("boot").into_bytes();
        frame.push(0);
        assert_eq!(frame, b"flash:boot\0");
    }

    #[test]
    fn download_size_gate_uses_parsed_max() {
        let max = parse_download_size("0x1fe00000").unwrap();
        let image: u64 = 64 * 1024 * 1024;
        assert!(image <= max);
        let small = parse_download_size("16M").unwrap();
        assert!(image > small);
    }

    #[test]
    fn usb_error_messages_are_readable() {
        assert!(UsbError::NotFound("ABC".into()).to_string().contains("ABC"));
    }
}
