//! Byte-link abstraction for the vendor transports: a plain read/write
//! channel with timeouts, implemented over USB bulk endpoints and
//! mocked in tests.

use std::fmt;
use std::io::{Read as _, Write as _};
use std::time::Duration;

use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, In, Out};
use nusb::{Device, Interface, MaybeFuture};
use serial2::SerialPort;

/// One byte stream to a bootloader port: CDC-ACM style serial over
/// bulk endpoints or a vendor-specific EDL interface.
pub trait Link {
    /// Reads whatever arrives within `timeout`; an empty result means
    /// timeout or link closed.
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> std::io::Result<usize>;
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()>;
}

#[derive(Debug)]
pub enum LinkError {
    Enumeration(String),
    NotFound(String),
    Endpoints(String),
    Io(String),
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkError::Enumeration(s) => write!(f, "usb enumeration failed: {s}"),
            LinkError::NotFound(s) => write!(f, "bootloader port not found: {s}"),
            LinkError::Endpoints(s) => write!(f, "bulk endpoints missing: {s}"),
            LinkError::Io(s) => write!(f, "usb transfer failed: {s}"),
        }
    }
}

impl std::error::Error for LinkError {}

/// Serial numbers of USB devices with one of the given `pids` at the
/// given vendor id: the bootloader ports Skello talks to.
pub fn usb_serial_devices(vid: u16, pids: &[u16]) -> Result<Vec<String>, LinkError> {
    let list = nusb::list_devices()
        .wait()
        .map_err(|e| LinkError::Enumeration(e.to_string()))?;
    Ok(list
        .filter(|info| info.vendor_id() == vid && pids.contains(&info.product_id()))
        .filter_map(|info| info.serial_number().map(str::to_owned))
        .collect())
}

/// Opens the device `vid:pid` and hands back a byte link on the first
/// interface that owns both bulk endpoints: CDC data interfaces and
/// vendor EDL interfaces both look like this.
pub fn open_usb_serial(vid: u16, pid: u16) -> Result<Box<dyn Link>, LinkError> {
    let mut list = nusb::list_devices()
        .wait()
        .map_err(|e| LinkError::Enumeration(e.to_string()))?;
    let info = list
        .find(|info| info.vendor_id() == vid && info.product_id() == pid)
        .ok_or_else(|| LinkError::NotFound(format!("{vid:04x}:{pid:04x}")))?;
    let device = info
        .open()
        .wait()
        .map_err(|e| LinkError::Io(e.to_string()))?;
    // Walk the interfaces and pick the first one owning both bulk
    // endpoints: CDC data interfaces and vendor EDL interfaces both
    // look like this.
    for num in 0..4u8 {
        let Ok(interface) = device.claim_interface(num).wait() else {
            continue;
        };
        let mut in_addr = None;
        let mut out_addr = None;
        if let Some(desc) = interface.descriptor() {
            for ep in desc.endpoints() {
                match ep.direction() {
                    nusb::transfer::Direction::In => in_addr = Some(ep.address()),
                    nusb::transfer::Direction::Out => out_addr = Some(ep.address()),
                }
            }
        }
        if let (Some(i), Some(o)) = (in_addr, out_addr) {
            let write = interface
                .endpoint::<Bulk, Out>(o)
                .map_err(|e| LinkError::Endpoints(e.to_string()))?
                .writer(16 * 1024);
            let read = interface
                .endpoint::<Bulk, In>(i)
                .map_err(|e| LinkError::Endpoints(e.to_string()))?
                .reader(64 * 1024);
            return Ok(Box::new(UsbSerialLink {
                _device: device,
                _interface: interface,
                write,
                read,
            }));
        }
    }
    Err(LinkError::Endpoints(format!("{vid:04x}:{pid:04x}")))
}

/// Opens the device and returns the handle, the claimed interface
/// and the bulk link: control transfers go through the interface
/// (the only way on Windows, nusb platform note), the byte protocol
/// over the bulk endpoints.
pub fn open_device_with_link(
    vid: u16,
    pid: u16,
) -> Result<(Device, Interface, Box<dyn Link>), LinkError> {
    let mut list = nusb::list_devices()
        .wait()
        .map_err(|e| LinkError::Enumeration(e.to_string()))?;
    let info = list
        .find(|info| info.vendor_id() == vid && info.product_id() == pid)
        .ok_or_else(|| LinkError::NotFound(format!("{vid:04x}:{pid:04x}")))?;
    let device = info
        .open()
        .wait()
        .map_err(|e| LinkError::Io(e.to_string()))?;
    for num in 0..4u8 {
        let Ok(interface) = device.claim_interface(num).wait() else {
            continue;
        };
        let mut in_addr = None;
        let mut out_addr = None;
        if let Some(desc) = interface.descriptor() {
            for ep in desc.endpoints() {
                match ep.direction() {
                    nusb::transfer::Direction::In => in_addr = Some(ep.address()),
                    nusb::transfer::Direction::Out => out_addr = Some(ep.address()),
                }
            }
        }
        if let (Some(i), Some(o)) = (in_addr, out_addr) {
            let write = interface
                .endpoint::<Bulk, Out>(o)
                .map_err(|e| LinkError::Endpoints(e.to_string()))?
                .writer(16 * 1024);
            let read = interface
                .endpoint::<Bulk, In>(i)
                .map_err(|e| LinkError::Endpoints(e.to_string()))?
                .reader(64 * 1024);
            return Ok((
                device.clone(),
                interface.clone(),
                Box::new(UsbSerialLink {
                    _device: device,
                    _interface: interface,
                    write,
                    read,
                }),
            ));
        }
    }
    Err(LinkError::Endpoints(format!("{vid:04x}:{pid:04x}")))
}

/// A live USB byte link: blocking bulk reads and writes through nusb.
pub struct UsbSerialLink {
    _device: Device,
    _interface: Interface,
    write: EndpointWrite<Bulk>,
    read: EndpointRead<Bulk>,
}

impl Link for UsbSerialLink {
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> std::io::Result<usize> {
        self.read.set_read_timeout(timeout);
        self.read.read(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        // nusb buffers writes and only submits the transfer once the
        // buffer fills or a flush happens; protocol exchanges are
        // smaller than the 16 KiB buffer, so flush every write.
        self.write.set_write_timeout(Duration::from_secs(10));
        self.write.write_all(buf)?;
        self.write.flush()
    }
}

/// A raw serial port link: the EDL transport over UART. Setup follows
/// qdlrs (BSD-3-Clause, serial.rs): raw mode, 8N1, 115200 baud by
/// default, per-call read timeouts; NOTICE has the attribution.
pub struct SerialLink {
    port: serial2::SerialPort,
}

/// Opens a serial port as a byte link for the EDL-over-UART path.
pub fn open_serial_link(path: &str, baud: u32) -> Result<Box<dyn Link>, LinkError> {
    let port = SerialPort::open(path, baud).map_err(|e| LinkError::Io(e.to_string()))?;
    Ok(Box::new(SerialLink { port }))
}

/// Names of the serial ports present on the system: the EDL UART
/// candidates (/dev/ttyUSB*, /dev/ttyACM*, COM*).
pub fn serial_port_names() -> Result<Vec<String>, LinkError> {
    Ok(SerialPort::available_ports()
        .map_err(|e| LinkError::Enumeration(e.to_string()))?
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

impl Link for SerialLink {
    fn read(&mut self, buf: &mut [u8], timeout: Duration) -> std::io::Result<usize> {
        self.port.set_read_timeout(timeout)?;
        match self.port.read(buf) {
            Ok(n) => Ok(n),
            Err(e)
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock =>
            {
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.port.set_write_timeout(Duration::from_secs(5))?;
        self.port.write_all(buf)?;
        Ok(())
    }
}

/// In-memory loopback link for tests: a `device` closure reacts to
/// every host write and queues its answers, so the protocol layers
/// run against a fake bootloader without any real hardware.
/// Device-side callback: gets the answer queue and the host write.
#[doc(hidden)]
pub type DeviceFn = Box<dyn FnMut(&mut std::collections::VecDeque<u8>, &[u8]) + Send>;

#[doc(hidden)]
pub struct MockLink {
    pub to_device: Vec<u8>,
    pub from_device: std::collections::VecDeque<u8>,
    pub device: Option<DeviceFn>,
}

impl MockLink {
    pub fn new() -> Self {
        Self {
            to_device: Vec::new(),
            from_device: std::collections::VecDeque::new(),
            device: None,
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        for b in data {
            self.from_device.push_back(*b);
        }
    }
}

impl Default for MockLink {
    fn default() -> Self {
        Self::new()
    }
}

impl Link for MockLink {
    fn read(&mut self, buf: &mut [u8], _timeout: Duration) -> std::io::Result<usize> {
        let n = buf.len().min(self.from_device.len());
        for (i, b) in self.from_device.drain(..n).enumerate() {
            buf[i] = b;
        }
        Ok(n)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if let Some(device) = &mut self.device {
            device(&mut self.from_device, buf);
        }
        self.to_device.extend_from_slice(buf);
        Ok(())
    }
}
