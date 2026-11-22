//! MediaTek DA V6 (XML) session: the string command protocol the new
//! platforms use after the DA jump. Same packet framing as XFlash,
//! but commands are XML documents and the answers walk an OK /
//! CMD:END / CMD:START cycle. Framing, command set and the stage 2
//! boot flow follow mtkclient (Apache-2.0, Library/DA/xmlflash/) and
//! mtkclient-rs (Apache-2.0, da/xflash/protocol/xml.rs); NOTICE has
//! the attribution. Session scope: boot, boot mode and reboot -
//! partition IO stays on the V5 transport for now.

use std::time::Duration;

use crate::mediatek::brom::{read_exact_t, Brom};
use crate::mediatek::daldr::{pick_loader, stage_image};
use crate::transport::Link;

pub const XML_MAGIC: u32 = 0xFEEEEEEF;
pub const DT_PROTOCOL_FLOW: u32 = 1;
/// Log messages the DA streams aside: skipped by the reader.
pub const DT_MESSAGE: u32 = 2;

/// Sanity cap on any length field coming from the DA.
const MAX_XML_PACKET: usize = 64 * 1024 * 1024;

const BOOT_TIMEOUT: Duration = Duration::from_secs(30);

/// Boot target for `set_boot_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlBootMode {
    Fastboot,
    Meta,
}

impl XmlBootMode {
    fn as_str(self) -> &'static str {
        match self {
            XmlBootMode::Fastboot => "FASTBOOT",
            XmlBootMode::Meta => "META",
        }
    }
}

#[derive(Debug)]
pub enum XmlDaError {
    Io(std::io::Error),
    WrongMagic,
    Unexpected(&'static str),
    /// The DA answered ERR! with its message.
    Rejected(String),
    /// The loader file is not a V6 XML DA.
    NotV6Loader,
}

impl std::fmt::Display for XmlDaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmlDaError::Io(e) => write!(f, "xml da io: {e}"),
            XmlDaError::WrongMagic => write!(f, "xml da wrong packet magic"),
            XmlDaError::Unexpected(s) => write!(f, "xml da unexpected response: {s}"),
            XmlDaError::Rejected(s) => write!(f, "xml da rejected: {s}"),
            XmlDaError::NotV6Loader => write!(f, "loader file is not a V6 XML DA"),
        }
    }
}

impl std::error::Error for XmlDaError {}

impl From<std::io::Error> for XmlDaError {
    fn from(e: std::io::Error) -> Self {
        XmlDaError::Io(e)
    }
}

impl From<crate::transport::LinkError> for XmlDaError {
    fn from(e: crate::transport::LinkError) -> Self {
        XmlDaError::Io(std::io::Error::other(e.to_string()))
    }
}

impl From<crate::mediatek::brom::BromError> for XmlDaError {
    fn from(e: crate::mediatek::brom::BromError) -> Self {
        XmlDaError::Io(std::io::Error::other(e.to_string()))
    }
}

type Result<T> = std::result::Result<T, XmlDaError>;

/// One DOWNLOAD-FILE request from the DA: it wants the named payload
/// streamed back in packet_length chunks.
#[derive(Debug, Clone)]
pub struct XmlDownload {
    pub checksum: String,
    pub info: String,
    pub source_file: String,
    pub packet_length: usize,
}

/// The answer to one XML command.
#[derive(Debug)]
pub enum XmlReply {
    /// The command closed cleanly: CMD:END with the result, then
    /// CMD:START.
    Completed(String),
    /// An intermediate command result before CMD:END.
    Result { command: String, result: String },
    /// The DA asks the host to download a file.
    Download(XmlDownload),
    /// Raw data received through an OK@length transfer.
    Data(Vec<u8>),
    /// The DA announced itself (CMD:START).
    Start,
}

/// A DA V6 session over an open byte link.
pub struct XmlDa {
    link: Box<dyn Link>,
    /// The size answer for the FILE-SYS-OPERATION query of
    /// WRITE-PARTITION: the DA asks how big the source file is.
    pending_size: Option<usize>,
}

impl XmlDa {
    pub fn new(link: Box<dyn Link>) -> Self {
        Self {
            link,
            pending_size: None,
        }
    }

    fn read_exact(&mut self, buf: &mut [u8], timeout: Duration) -> Result<()> {
        read_exact_t(&mut *self.link, buf, timeout)?;
        Ok(())
    }

    fn send_packet(&mut self, body: &[u8], datatype: u32) -> Result<()> {
        let mut hdr = Vec::with_capacity(12 + body.len());
        hdr.extend_from_slice(&XML_MAGIC.to_le_bytes());
        hdr.extend_from_slice(&datatype.to_le_bytes());
        hdr.extend_from_slice(&(body.len() as u32).to_le_bytes());
        hdr.extend_from_slice(body);
        self.link.write_all(&hdr)?;
        Ok(())
    }

    /// Sends an XML document as a NUL-terminated string packet.
    fn send_str(&mut self, text: &str) -> Result<()> {
        let mut body = Vec::with_capacity(text.len() + 1);
        body.extend_from_slice(text.as_bytes());
        body.push(0);
        self.send_packet(&body, DT_PROTOCOL_FLOW)
    }

    /// `OK@0x...` ack used by the file transfer.
    fn ack_value(&mut self, value: usize) -> Result<()> {
        self.send_str(&format!("OK@{value:#x}"))
    }

    fn ack(&mut self) -> Result<()> {
        self.send_str("OK")
    }

    /// Reads one protocol packet, draining the DT_MESSAGE log stream
    /// the DA emits alongside.
    fn read_packet(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        loop {
            let mut hdr = [0u8; 12];
            self.read_exact(&mut hdr, timeout)?;
            let magic = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
            if magic != XML_MAGIC {
                return Err(XmlDaError::WrongMagic);
            }
            let datatype = u32::from_le_bytes(hdr[4..8].try_into().unwrap());
            let length = u32::from_le_bytes(hdr[8..12].try_into().unwrap()) as usize;
            if length > MAX_XML_PACKET {
                return Err(XmlDaError::Unexpected("oversized packet length"));
            }
            if datatype == DT_MESSAGE {
                // priority is 4 bytes when the packet is long enough;
                // a short packet must not swallow the next header
                let priority_len = length.min(4);
                let mut skip = vec![0u8; priority_len];
                self.read_exact(&mut skip, timeout)?;
                let mut msg = vec![0u8; length - priority_len];
                self.read_exact(&mut msg, timeout)?;
                continue;
            }
            let mut body = vec![0u8; length];
            self.read_exact(&mut body, timeout)?;
            return Ok(body);
        }
    }

    /// The previous packet body as text (NUL stripped).
    fn get_response(&mut self) -> Result<String> {
        let body = self.read_packet(BOOT_TIMEOUT)?;
        Ok(String::from_utf8_lossy(&body)
            .trim_end_matches('\0')
            .to_owned())
    }

    fn get_response_data(&mut self) -> Result<Vec<u8>> {
        self.read_packet(BOOT_TIMEOUT)
    }

    /// Parses the DA's command answer XML into a reply.
    fn get_command_result(&mut self) -> Result<XmlReply> {
        let data = self.get_response()?;
        let command = xml_field(&data, "command").unwrap_or_default();
        if command.is_empty() && data.contains("OK@") {
            // data transfer: OK@0x<len>, then ack, then raw chunks
            let tmp = data.split('@').nth(1).unwrap_or("");
            let length = usize::from_str_radix(tmp.trim_start_matches("0x"), 16).unwrap_or(0);
            self.ack()?;
            let resp = self.get_response()?;
            if !resp.contains("OK") {
                return Err(XmlDaError::Unexpected("transfer ack"));
            }
            self.ack()?;
            // The length comes from the DA's own OK@ string; cap the
            // allocation and the loop before trusting it.
            if length > MAX_XML_PACKET {
                return Err(XmlDaError::Unexpected("oversized transfer length"));
            }
            let mut out = Vec::with_capacity(length);
            let mut iters = 0u32;
            while out.len() < length {
                iters += 1;
                if iters > 1_000_000 {
                    return Err(XmlDaError::Unexpected("transfer made no progress"));
                }
                out.extend_from_slice(&self.get_response_data()?);
            }
            self.ack()?;
            return Ok(XmlReply::Data(out[..length].to_vec()));
        }
        match command.as_str() {
            "CMD:START" => {
                self.ack()?;
                Ok(XmlReply::Start)
            }
            "CMD:PROGRESS-REPORT" => {
                self.ack()?;
                loop {
                    let line = self.get_response()?;
                    self.ack()?;
                    if line == "OK!EOT" {
                        break;
                    }
                }
                self.get_command_result()
            }
            "CMD:UPLOAD-FILE" => {
                // the DA wants to send data: acknowledge, then the
                // size answer follows as OK@0x<len>
                self.ack()?;
                self.get_command_result()
            }
            "CMD:FILE-SYS-OPERATION" => {
                // the DA asks for the file operation (the source
                // size for WRITE-PARTITION): answer the pending size
                self.ack()?;
                let payload = match self.pending_size {
                    Some(size) => format!("OK@{size:#x}"),
                    None => "OK@NOT-EXISTS".to_owned(),
                };
                self.send_str(&payload)?;
                self.get_command_result()
            }
            "CMD:DOWNLOAD-FILE" => {
                let packet_length = usize::from_str_radix(
                    xml_field(&data, "packet_length")
                        .unwrap_or_default()
                        .trim_start_matches("0x"),
                    16,
                )
                .unwrap_or(0x1000);
                self.ack()?;
                Ok(XmlReply::Download(XmlDownload {
                    checksum: xml_field(&data, "checksum").unwrap_or_default(),
                    info: xml_field(&data, "info").unwrap_or_default(),
                    source_file: xml_field(&data, "source_file").unwrap_or_default(),
                    packet_length,
                }))
            }
            "CMD:END" => {
                let result = xml_field(&data, "result").unwrap_or_default();
                Ok(XmlReply::Result { command, result })
            }
            other => Ok(XmlReply::Result {
                command: other.to_owned(),
                result: xml_field(&data, "result").unwrap_or_default(),
            }),
        }
    }

    /// Sends one XML command and walks the OK / CMD:END / CMD:START
    /// cycle. Returns the final answer; DOWNLOAD-FILE replies come
    /// back for the caller to stream into.
    pub fn send_command(&mut self, xml: &str) -> Result<XmlReply> {
        self.send_str(xml)?;
        let resp = self.get_response()?;
        if !resp.contains("OK") {
            if resp.starts_with("ERR!") {
                return Err(XmlDaError::Rejected(resp));
            }
            return Err(XmlDaError::Unexpected("command ack"));
        }
        match self.get_command_result()? {
            XmlReply::Result { command, result } if command == "CMD:END" => {
                if result != "OK" {
                    return Err(XmlDaError::Rejected(result));
                }
                self.ack()?;
                match self.get_command_result()? {
                    XmlReply::Start => Ok(XmlReply::Completed(result)),
                    XmlReply::Result { command, result } => {
                        Ok(XmlReply::Result { command, result })
                    }
                    other => Ok(other),
                }
            }
            other => Ok(other),
        }
    }

    /// Runtime parameters, host capabilities and the hardware init
    /// notification. SLA/auth is not attempted; devices with it active
    /// reject the session with an ERR! later.
    pub fn init_session(&mut self) -> Result<()> {
        self.send_command(&build_command(
            "SET-RUNTIME-PARAMETER",
            "1.1",
            Some(
                "<checksum_level>NONE</checksum_level>\
                 <battery_exist>AUTO-DETECT</battery_exist>\
                 <da_log_level>INFO</da_log_level>\
                 <log_channel>USB</log_channel>\
                 <system_os>LINUX</system_os>",
            ),
            Some("<initialize_dram>YES</initialize_dram>"),
        ))?
        .completed()?;
        self.send_command(&build_command(
            "HOST-SUPPORTED-COMMANDS",
            "1.0",
            Some("<host_capability>CMD:DOWNLOAD-FILE^1@CMD:FILE-SYS-OPERATION^1@CMD:PROGRESS-REPORT^1@CMD:UPLOAD-FILE^1@</host_capability>"),
            None,
        ))?
        .completed()?;
        self.send_command(&build_command("NOTIFY-INIT-HW", "1.0", None, None))?
            .completed()?;
        Ok(())
    }

    /// Boots stage 2 through BOOT-TO: the DA answers with a
    /// DOWNLOAD-FILE request and the image streams in chunks.
    pub fn boot_to(&mut self, addr: u32, data: &[u8]) -> Result<()> {
        let xml = build_command(
            "BOOT-TO",
            "1.0",
            Some(&format!(
                "<at_address>{addr:#x}</at_address>\
                 <jmp_address>{addr:#x}</jmp_address>\
                 <source_file>MEM://{:#x}:{:#x}</source_file>",
                0usize,
                data.len()
            )),
            None,
        );
        let download = match self.send_command(&xml)? {
            XmlReply::Download(dl) => dl,
            _ => return Err(XmlDaError::Unexpected("boot_to download request")),
        };
        self.stream_download(download, data)
    }

    /// Streams `data` into the device through the DOWNLOAD-FILE
    /// handshake the DA opened, then closes the command cycle.
    fn stream_download(&mut self, dl: XmlDownload, data: &[u8]) -> Result<()> {
        let packet_length = dl.packet_length.max(0x100);
        self.pending_size = Some(data.len());
        self.ack_value(data.len())?;
        if !self.get_response()?.contains("OK") {
            return Err(XmlDaError::Unexpected("upload length ack"));
        }
        let mut pos = 0;
        while pos < data.len() {
            self.ack_value(0)?;
            if !self.get_response()?.contains("OK") {
                return Err(XmlDaError::Unexpected("upload chunk ack"));
            }
            let end = (pos + packet_length).min(data.len());
            self.send_packet(&data[pos..end], DT_PROTOCOL_FLOW)?;
            if !self.get_response()?.contains("OK") {
                return Err(XmlDaError::Unexpected("upload chunk"));
            }
            pos = end;
        }
        self.finish_end_start()
    }

    /// Closes a command whose data phase ended: reads CMD:END,
    /// checks the result, acks and consumes the next CMD:START.
    fn finish_end_start(&mut self) -> Result<()> {
        match self.get_command_result()? {
            XmlReply::Result { command, result } if command == "CMD:END" => {
                if result != "OK" {
                    return Err(XmlDaError::Rejected(result));
                }
                self.ack()?;
                match self.get_command_result()? {
                    XmlReply::Start => Ok(()),
                    _ => Err(XmlDaError::Unexpected("post-command start")),
                }
            }
            _ => Err(XmlDaError::Unexpected("command end")),
        }
    }

    /// Chunked flash read with session recovery: on a transport
    /// failure `reopen` builds a fresh XmlDa and reading continues at
    /// the exact offset. `sink` receives (offset, chunk). Returns the
    /// total bytes read.
    pub fn read_flash_resumable(
        total_len: usize,
        chunk_len: usize,
        mut sink: impl FnMut(usize, &[u8]) -> Result<()>,
        mut reopen: impl FnMut() -> Result<XmlDa>,
    ) -> Result<usize> {
        let mut session = reopen()?;
        let mut pos = 0usize;
        let mut attempts = 0usize;
        while pos < total_len {
            let want = chunk_len.min(total_len - pos);
            match session.read_flash_region("user", pos as u64, want) {
                Ok(data) => {
                    sink(pos, &data)?;
                    pos += want;
                    attempts = 0;
                }
                Err(e) => {
                    attempts += 1;
                    if attempts > 3 {
                        return Err(e);
                    }
                    // the transport died: rebuild and continue here
                    session = reopen()?;
                }
            }
        }
        Ok(pos)
    }

    /// Chunked flash write with session recovery: erases once per
    /// fresh session, then writes the remaining bytes at the exact
    /// offset. Returns the total bytes written.
    pub fn write_flash_resumable(
        source: &mut dyn std::io::Read,
        total_len: usize,
        chunk_len: usize,
        mut reopen: impl FnMut() -> Result<XmlDa>,
    ) -> Result<usize> {
        let mut session = reopen()?;
        let mut pos = 0usize;
        let mut attempts = 0usize;
        while pos < total_len {
            let want = chunk_len.min(total_len - pos);
            let mut chunk = vec![0u8; want];
            source.read_exact(&mut chunk)?;
            // the wipe runs exactly once, before the first chunk:
            // a resumed write must not erase what already landed
            let step = if pos == 0 {
                session.erase_flash_region("user", 0, total_len)
            } else {
                Ok(())
            };
            match step.and_then(|()| session.write_flash_region("user", pos as u64, &chunk)) {
                Ok(()) => {
                    pos += want;
                    attempts = 0;
                }
                Err(e) => {
                    attempts += 1;
                    if attempts > 3 {
                        return Err(e);
                    }
                    session = reopen()?;
                }
            }
        }
        Ok(pos)
    }

    /// Asks the running DA to reboot into fastboot or meta.
    pub fn set_boot_mode(&mut self, mode: XmlBootMode) -> Result<()> {
        self.send_command(&build_command(
            "SET-BOOT-MODE",
            "1.0",
            Some(&format!(
                "<mode>{}</mode>\
                 <connect_type>USB</connect_type>\
                 <mobile_log>ON</mobile_log>\
                 <adb>ON</adb>",
                mode.as_str()
            )),
            None,
        ))?
        .completed()?;
        Ok(())
    }

    /// Reboots the device; `disconnect` drops the USB link first.
    pub fn reboot(&mut self, disconnect: bool) -> Result<()> {
        let action = if disconnect {
            "DISCONNECT"
        } else {
            "IMMEDIATE"
        };
        self.send_command(&build_command(
            "REBOOT",
            "1.0",
            Some(&format!("<action>{action}</action>")),
            None,
        ))?
        .completed()?;
        Ok(())
    }

    /// `READ-FLASH`: one region read by byte offset over the
    /// running DA. The DA streams the data back through
    /// UPLOAD-FILE; short reads are an honest error.
    pub fn read_flash_region(
        &mut self,
        section: &str,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>> {
        let xml = build_command(
            "READ-FLASH",
            "1.0",
            Some(&format!(
                "<partition>{section}</partition>\
                 <target_file>MEM</target_file>\
                 <length>{length:#x}</length>\
                 <offset>{offset:#x}</offset>"
            )),
            None,
        );
        let data = match self.send_command(&xml)? {
            XmlReply::Data(d) => d,
            _ => return Err(XmlDaError::Unexpected("read-flash upload")),
        };
        if data.len() != length {
            return Err(XmlDaError::Unexpected("read-flash short read"));
        }
        self.finish_end_start()?;
        Ok(data)
    }

    /// `WRITE-FLASH`: one region write by byte offset, data streamed
    /// through the DOWNLOAD-FILE handshake.
    pub fn write_flash_region(&mut self, section: &str, offset: u64, data: &[u8]) -> Result<()> {
        let xml = build_command(
            "WRITE-FLASH",
            "1.0",
            Some(&format!(
                "<partition>{section}</partition>\
                 <source_file>MEM:\\0x0:0x{:X}</source_file>\
                 <offset>{offset:#x}</offset>",
                data.len()
            )),
            None,
        );
        match self.send_command(&xml)? {
            XmlReply::Download(dl) => self.stream_download(dl, data),
            _ => Err(XmlDaError::Unexpected("write-flash download request")),
        }
    }

    /// `ERASE-FLASH`: wipes a region; the DA reports progress until
    /// it answers EOT.
    pub fn erase_flash_region(&mut self, section: &str, offset: u64, length: usize) -> Result<()> {
        let xml = build_command(
            "ERASE-FLASH",
            "1.0",
            Some(&format!(
                "<partition>{section}</partition>\
                 <length>{length:#x}</length>\
                 <offset>{offset:#x}</offset>"
            )),
            None,
        );
        self.send_command(&xml)?.completed()?;
        Ok(())
    }

    /// `ERASE-PARTITION`: wipes the whole named partition.
    pub fn erase_partition_named(&mut self, name: &str) -> Result<()> {
        let xml = build_command(
            "ERASE-PARTITION",
            "1.0",
            Some(&format!("<partition>{name}</partition>")),
            None,
        );
        self.send_command(&xml)?.completed()?;
        Ok(())
    }

    /// `READ-PARTITION`: reads the whole named partition.
    pub fn read_partition_named(&mut self, name: &str) -> Result<Vec<u8>> {
        let xml = build_command(
            "READ-PARTITION",
            "1.0",
            Some(&format!(
                "<partition>{name}</partition><target_file>{name}.bin</target_file>"
            )),
            None,
        );
        let data = match self.send_command(&xml)? {
            XmlReply::Data(d) => d,
            _ => return Err(XmlDaError::Unexpected("read-partition upload")),
        };
        self.finish_end_start()?;
        Ok(data)
    }

    /// `WRITE-PARTITION`: writes the whole named partition. The DA
    /// pre-erases, asks the source size (FILE-SYS) and streams the
    /// data through DOWNLOAD-FILE.
    pub fn write_partition_named(&mut self, name: &str, data: &[u8]) -> Result<()> {
        let xml = build_command(
            "WRITE-PARTITION",
            "1.0",
            Some(&format!(
                "<partition>{name}</partition><source_file>{name}.bin</source_file>"
            )),
            None,
        );
        self.pending_size = Some(data.len());
        match self.send_command(&xml)? {
            XmlReply::Download(dl) => self.stream_download(dl, data),
            _ => Err(XmlDaError::Unexpected("write-partition download request")),
        }
    }

    /// Boots the DA for flash operations: like the full V6 boot but
    /// without the boot-mode switch and the reboot tail, so read and
    /// write commands run right after. Also the session builder the
    /// recovery loop uses to reopen after a transport failure.
    pub fn flash_session_with(
        brom: &mut Brom,
        loader_path: &str,
        da_open: &mut dyn FnMut()
            -> std::result::Result<Box<dyn Link>, crate::transport::LinkError>,
    ) -> Result<XmlDa> {
        brom.handshake()?;
        let (hw_code, _sub) = brom.get_hw_code()?;
        let (loader, bytes) = pick_loader(loader_path, hw_code)
            .map_err(|e| XmlDaError::Io(std::io::Error::other(e.to_string())))?;
        if !loader.v6 {
            return Err(XmlDaError::NotV6Loader);
        }
        let s1 = loader
            .stage1()
            .ok_or(XmlDaError::Unexpected("no stage 1"))?;
        let s2 = loader
            .stage2()
            .ok_or(XmlDaError::Unexpected("no stage 2"))?;
        let da1 = stage_image(&bytes, s1)
            .map_err(|e| XmlDaError::Io(std::io::Error::other(e.to_string())))?;
        let da2 = stage_image(&bytes, s2)
            .map_err(|e| XmlDaError::Io(std::io::Error::other(e.to_string())))?;
        brom.send_da(s1.start_addr, s1.sig_len, &da1)?;
        brom.jump_da(s1.start_addr)?;
        let link = da_open()?;
        let mut da = XmlDa::new(link);
        match da.get_command_result()? {
            XmlReply::Start => {}
            _ => return Err(XmlDaError::Unexpected("xml da start")),
        }
        da.init_session()?;
        da.boot_to(s2.start_addr, &da2)?;
        Ok(da)
    }

    /// Full V6 boot over real hardware: BROM handshake, stage 1
    /// upload, XML session on the re-enumerated DA port, stage 2
    /// through BOOT-TO and the requested boot mode.
    pub fn boot_v6(loader_path: &str, mode: XmlBootMode) -> Result<String> {
        let mut brom = Brom::new(crate::transport::open_usb_serial(
            crate::mediatek::brom::VID_MEDIATEK,
            crate::mediatek::brom::PID_BROM,
        )?);
        Self::boot_v6_with(&mut brom, loader_path, mode, &mut || {
            crate::transport::open_usb_serial(
                crate::mediatek::brom::VID_MEDIATEK,
                crate::mediatek::brom::PID_DA,
            )
        })
    }

    /// Same flow over an established BROM link with a replaceable DA
    /// port opener (tests substitute a mock device).
    pub fn boot_v6_with(
        brom: &mut Brom,
        loader_path: &str,
        mode: XmlBootMode,
        da_open: &mut dyn FnMut()
            -> std::result::Result<Box<dyn Link>, crate::transport::LinkError>,
    ) -> Result<String> {
        brom.handshake()?;
        let (hw_code, _sub) = brom.get_hw_code()?;
        let (loader, bytes) = pick_loader(loader_path, hw_code)
            .map_err(|e| XmlDaError::Io(std::io::Error::other(e.to_string())))?;
        if !loader.v6 {
            return Err(XmlDaError::NotV6Loader);
        }
        let s1 = loader
            .stage1()
            .ok_or(XmlDaError::Unexpected("no stage 1"))?;
        let s2 = loader
            .stage2()
            .ok_or(XmlDaError::Unexpected("no stage 2"))?;
        let da1 = stage_image(&bytes, s1)
            .map_err(|e| XmlDaError::Io(std::io::Error::other(e.to_string())))?;
        let da2 = stage_image(&bytes, s2)
            .map_err(|e| XmlDaError::Io(std::io::Error::other(e.to_string())))?;
        brom.send_da(s1.start_addr, s1.sig_len, &da1)?;
        brom.jump_da(s1.start_addr)?;
        let link = da_open()?;
        let mut da = XmlDa::new(link);
        match da.get_command_result()? {
            XmlReply::Start => {}
            _ => return Err(XmlDaError::Unexpected("xml da start")),
        }
        da.init_session()?;
        da.boot_to(s2.start_addr, &da2)?;
        da.set_boot_mode(mode)?;
        da.reboot(true)?;
        Ok(format!(
            "DA V6 booted, device set to {}",
            mode.as_str().to_lowercase()
        ))
    }
}

impl XmlReply {
    /// Waits for a clean completion when the caller does not care
    /// about the result text.
    fn completed(self) -> Result<String> {
        match self {
            XmlReply::Completed(result) => Ok(result),
            XmlReply::Start => Ok("START".to_owned()),
            _ => Err(XmlDaError::Unexpected("expected command completion")),
        }
    }
}

/// Extracts the text between `<tag>` and `</tag>`.
pub fn xml_field(data: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = data.find(&open)? + open.len();
    let end = data[start..].find(&close)? + start;
    Some(data[start..end].to_string())
}

/// Builds one DA XML command document.
fn build_command(cmd: &str, version: &str, arg: Option<&str>, adv: Option<&str>) -> String {
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><da><version>{version}</version><command>CMD:{cmd}</command>"
    );
    if let Some(arg) = arg {
        s.push_str("<arg>");
        s.push_str(arg);
        s.push_str("</arg>");
    }
    if let Some(adv) = adv {
        s.push_str("<adv>");
        s.push_str(adv);
        s.push_str("</adv>");
    }
    s.push_str("</da>");
    s
}

/// Builds a V6 loader file: the "MTK_DA_v6" marker and one DA entry
/// with three regions (boot, stage1, stage2) in the 0xDC layout.
#[cfg(test)]
fn build_loader_v6(hw_code: u16, stage2: &[u8]) -> Vec<u8> {
    let mut data = vec![0u8; 0xE00];
    data[..9].copy_from_slice(b"MTK_DA_v6");
    data[0x68..0x6C].copy_from_slice(&1u32.to_le_bytes());
    data[0x6C] = 0x69;
    data[0x6D] = 0x01;
    data[0x6E..0x70].copy_from_slice(&hw_code.to_le_bytes());
    data[0x6C + 0x12..0x6C + 0x14].copy_from_slice(&3u16.to_le_bytes());
    let region_base = 0x6C + 0x14;
    for (i, (buf, len, addr)) in [
        (0x200u32, 0x100u32, 0x100u32),
        (0x300, 0x400, 0x200000),
        (0x700, stage2.len() as u32, 0x40000000),
    ]
    .iter()
    .enumerate()
    {
        let ro = region_base + i * 20;
        data[ro..ro + 4].copy_from_slice(&buf.to_le_bytes());
        data[ro + 4..ro + 8].copy_from_slice(&len.to_le_bytes());
        data[ro + 8..ro + 12].copy_from_slice(&addr.to_le_bytes());
    }
    // stage payloads live after the header at their buf offsets
    for i in 0x148..0x700u32 {
        data[i as usize] = (i * 3) as u8;
    }
    data[0x700..0x700 + stage2.len()].copy_from_slice(stage2);
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mediatek::brom::mock_brom;
    use crate::transport::MockLink;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    const END_OK: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?><host><version>1.0</version><command>CMD:END</command><arg><result>OK</result></arg></host>";

    fn dev_xml(cmd: &str, extra: &str) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><host><version>1.0</version><command>{cmd}</command>{extra}</host>"
        )
    }

    fn mock_xml_da() -> MockLink {
        mock_xml_da_muted(0)
    }

    /// The same DA mock that goes silent (link dead) after `mute`
    /// successful READ-FLASH answers.
    fn mock_xml_da_muted(mute: usize) -> MockLink {
        let mut link = MockLink::new();
        // the DA announces itself with CMD:START right after the jump
        let mut q = std::collections::VecDeque::new();
        MockState::queue(&mut q, &dev_xml("CMD:START", ""));
        let start: Vec<u8> = q.into_iter().collect();
        link.push(&start);
        let state = RefCell::new(MockState {
            mute_after_reads: mute,
            ..Default::default()
        });
        link.device = Some(Box::new(move |out, input| {
            let mut st = state.borrow_mut();
            st.in_buf.extend_from_slice(input);
            st.process(out);
        }));
        link
    }

    #[derive(Default)]
    struct MockState {
        in_buf: Vec<u8>,
        expect_len: usize,
        uploaded: Vec<u8>,
        seen: Vec<String>,
        /// The host just answered the FILE-SYS size query: the next
        /// OK@ reply carries the partition size, DOWNLOAD follows.
        after_filesys: bool,
        /// READ-FLASH answers served so far.
        reads_done: usize,
        /// When nonzero, the mock goes silent after this many reads.
        mute_after_reads: usize,
    }

    impl MockState {
        fn queue(out: &mut VecDeque<u8>, text: &str) {
            let mut body = text.as_bytes().to_vec();
            body.push(0);
            Self::queue_packet(out, &body);
        }

        fn queue_packet(out: &mut VecDeque<u8>, body: &[u8]) {
            out.extend(XML_MAGIC.to_le_bytes());
            out.extend(DT_PROTOCOL_FLOW.to_le_bytes());
            out.extend((body.len() as u32).to_le_bytes());
            out.extend(body.iter().copied());
        }

        fn process(&mut self, out: &mut VecDeque<u8>) {
            if self.mute_after_reads > 0 && self.reads_done >= self.mute_after_reads {
                // the link went dark: no answers, reads time out
                return;
            }
            loop {
                if self.in_buf.len() < 12 {
                    break;
                }
                let magic = u32::from_le_bytes(self.in_buf[0..4].try_into().unwrap());
                if magic != XML_MAGIC {
                    break;
                }
                let length = u32::from_le_bytes(self.in_buf[8..12].try_into().unwrap()) as usize;
                if self.in_buf.len() < 12 + length {
                    break;
                }
                let body: Vec<u8> = self.in_buf.drain(..12 + length).skip(12).collect();
                self.handle(&body, out);
            }
        }

        fn handle(&mut self, body: &[u8], out: &mut VecDeque<u8>) {
            if body.starts_with(b"<?xml") {
                let text = String::from_utf8_lossy(body)
                    .trim_end_matches('\0')
                    .to_owned();
                let cmd = xml_field(&text, "command").unwrap_or_default();
                self.seen.push(cmd.clone());
                match cmd.as_str() {
                    "CMD:NOTIFY-INIT-HW" => {
                        Self::queue(out, "OK");
                        Self::queue(
                            out,
                            &dev_xml(
                                "CMD:PROGRESS-REPORT",
                                "<arg><message>init-hw</message></arg>",
                            ),
                        );
                        Self::queue(out, "OK!EOT");
                        Self::queue(out, END_OK);
                        Self::queue(out, &dev_xml("CMD:START", ""));
                    }
                    "CMD:READ-FLASH" | "CMD:READ-PARTITION" => {
                        let len_field = xml_field(&text, "length").unwrap_or_default();
                        let len = usize::from_str_radix(len_field.trim_start_matches("0x"), 16)
                            .unwrap_or(0x800);
                        let off_field = xml_field(&text, "offset").unwrap_or_default();
                        let off = usize::from_str_radix(off_field.trim_start_matches("0x"), 16)
                            .unwrap_or(0);
                        self.reads_done += 1;
                        let data: Vec<u8> = (0..len).map(|i| ((off + i) * 7 % 253) as u8).collect();
                        Self::queue(out, "OK");
                        Self::queue(
                            out,
                            &dev_xml(
                                "CMD:UPLOAD-FILE",
                                "<arg><checksum>CHK_NO</checksum><info>read</info><target_file>MEM</target_file><packet_length>0x100</packet_length></arg>",
                            ),
                        );
                        Self::queue(out, &format!("OK@{len:#x}"));
                        Self::queue(out, "OK");
                        Self::queue_packet(out, &data);
                        Self::queue(out, END_OK);
                        Self::queue(out, &dev_xml("CMD:START", ""));
                    }
                    "CMD:WRITE-FLASH" => {
                        let source = xml_field(&text, "source_file").unwrap_or_default();
                        // MEM:\0x0:0x<len>
                        let len_part = source.rsplit(':').next().unwrap_or("0x0");
                        self.expect_len =
                            usize::from_str_radix(len_part.trim_start_matches("0x"), 16)
                                .unwrap_or(0);
                        self.uploaded.clear();
                        Self::queue(out, "OK");
                        Self::queue(
                            out,
                            &dev_xml(
                                "CMD:DOWNLOAD-FILE",
                                "<arg><checksum>CHK_NO</checksum><info>write-flash</info><source_file>MEM</source_file><packet_length>0x100</packet_length></arg>",
                            ),
                        );
                    }
                    "CMD:WRITE-PARTITION" => {
                        Self::queue(out, "OK");
                        Self::queue(
                            out,
                            &dev_xml(
                                "CMD:PROGRESS-REPORT",
                                "<arg><message>pre-erase</message></arg>",
                            ),
                        );
                        Self::queue(out, "OK!EOT");
                        Self::queue(
                            out,
                            &dev_xml(
                                "CMD:FILE-SYS-OPERATION",
                                "<arg><key>MEM</key><operation>GetSize</operation></arg>",
                            ),
                        );
                        self.after_filesys = true;
                    }
                    "CMD:ERASE-FLASH" | "CMD:ERASE-PARTITION" => {
                        Self::queue(out, "OK");
                        Self::queue(
                            out,
                            &dev_xml(
                                "CMD:PROGRESS-REPORT",
                                "<arg><message>erasing</message></arg>",
                            ),
                        );
                        Self::queue(out, "OK!EOT");
                        Self::queue(out, END_OK);
                        Self::queue(out, &dev_xml("CMD:START", ""));
                    }
                    "CMD:BOOT-TO" => {
                        let source = xml_field(&text, "source_file").unwrap_or_default();
                        // MEM://0x0:0x<len>
                        let len_part = source.rsplit(':').next().unwrap_or("0x0");
                        self.expect_len =
                            usize::from_str_radix(len_part.trim_start_matches("0x"), 16)
                                .unwrap_or(0);
                        self.uploaded.clear();
                        Self::queue(out, "OK");
                        Self::queue(
                            out,
                            &dev_xml(
                                "CMD:DOWNLOAD-FILE",
                                "<arg><checksum>CHK_NO</checksum><info>2nd-DA</info><source_file>MEM://0x0:0x0</source_file><packet_length>0x100</packet_length></arg>",
                            ),
                        );
                    }
                    _ => {
                        Self::queue(out, "OK");
                        Self::queue(out, END_OK);
                        Self::queue(out, &dev_xml("CMD:START", ""));
                    }
                }
                return;
            }
            if body.starts_with(b"OK@") {
                let text = String::from_utf8_lossy(body)
                    .trim_end_matches('\0')
                    .to_owned();
                if self.after_filesys {
                    // the partition size answer: open the download
                    self.after_filesys = false;
                    self.expect_len =
                        usize::from_str_radix(text.trim_start_matches("OK@0x"), 16).unwrap_or(0);
                    self.uploaded.clear();
                    Self::queue(
                        out,
                        &dev_xml(
                            "CMD:DOWNLOAD-FILE",
                            "<arg><checksum>CHK_NO</checksum><info>partition</info><source_file>partition.bin</source_file><packet_length>0x100</packet_length></arg>",
                        ),
                    );
                } else {
                    // host transfer acks: answer OK, chunks follow
                    Self::queue(out, "OK");
                }
                return;
            }
            if body == b"OK\0" {
                // plain ack: consumed silently
                return;
            }
            // raw upload chunk
            self.uploaded.extend_from_slice(body);
            Self::queue(out, "OK");
            if !self.uploaded.is_empty() && self.uploaded.len() >= self.expect_len {
                Self::queue(out, END_OK);
                Self::queue(out, &dev_xml("CMD:START", ""));
            }
        }
    }

    #[test]
    fn xml_field_extracts_tag() {
        let data = "<host><command>CMD:END</command><result>OK</result></host>";
        assert_eq!(xml_field(data, "command").unwrap(), "CMD:END");
        assert_eq!(xml_field(data, "result").unwrap(), "OK");
        assert!(xml_field(data, "missing").is_none());
    }

    #[test]
    fn session_init_boot_mode_reboot_cycle() {
        let mut da = XmlDa::new(Box::new(mock_xml_da()));
        // the DA announces itself with CMD:START
        assert!(matches!(da.get_command_result().unwrap(), XmlReply::Start));
        da.init_session().unwrap();
        da.set_boot_mode(XmlBootMode::Fastboot).unwrap();
        da.reboot(true).unwrap();
    }

    #[test]
    fn boot_v6_full_flow() {
        let stage2: Vec<u8> = (0..0x400u32).map(|i| (i * 5) as u8).collect();
        let loader_path = std::env::temp_dir().join("skello_test_loader_v6.bin");
        std::fs::write(&loader_path, build_loader_v6(0x6572, &stage2)).unwrap();

        let mut brom = Brom::new(Box::new(mock_brom()));
        let mut da_open = || -> std::result::Result<Box<dyn Link>, crate::transport::LinkError> {
            Ok(Box::new(mock_xml_da()))
        };
        let result = XmlDa::boot_v6_with(
            &mut brom,
            loader_path.to_str().unwrap(),
            XmlBootMode::Fastboot,
            &mut da_open,
        )
        .unwrap();
        assert!(result.contains("fastboot"));
        let _ = std::fs::remove_file(&loader_path);
    }

    #[test]
    fn v5_loader_refused_for_xml() {
        // A loader without the V6 marker must not start the XML flow.
        let stage2 = vec![0u8; 0x100];
        let loader_path = std::env::temp_dir().join("skello_test_loader_v5.bin");
        std::fs::write(&loader_path, build_loader_v6(0x6572, &stage2)).unwrap();
        let mut bytes = std::fs::read(&loader_path).unwrap();
        bytes[..9].copy_from_slice(b"MTK_AllIn");
        std::fs::write(&loader_path, &bytes).unwrap();

        let mut brom = Brom::new(Box::new(mock_brom()));
        let mut da_open = || -> std::result::Result<Box<dyn Link>, crate::transport::LinkError> {
            Ok(Box::new(mock_xml_da()))
        };
        assert!(matches!(
            XmlDa::boot_v6_with(
                &mut brom,
                loader_path.to_str().unwrap(),
                XmlBootMode::Meta,
                &mut da_open
            ),
            Err(XmlDaError::NotV6Loader)
        ));
        let _ = std::fs::remove_file(&loader_path);
    }

    /// Opens a synced V6 session over the mock: the DA announces
    /// itself, then the runtime parameters run through.
    fn synced_xml_session() -> XmlDa {
        let mut da = XmlDa::new(Box::new(mock_xml_da()));
        // consume the post-jump START announcement first
        match da.get_command_result().unwrap() {
            XmlReply::Start => {}
            _ => panic!("expected the da start"),
        }
        da.init_session().unwrap();
        da
    }

    #[test]
    fn v6_read_flash_streams_data() {
        let mut da = synced_xml_session();
        let data = da.read_flash_region("user", 0, 0x800).unwrap();
        assert_eq!(data.len(), 0x800);
        assert_eq!(data[0], 0);
        assert_eq!(data[0x101], (0x101 * 7 % 253) as u8);
    }

    #[test]
    fn v6_read_partition_streams_data() {
        let mut da = synced_xml_session();
        let data = da.read_partition_named("boot").unwrap();
        assert_eq!(data.len(), 0x800);
    }

    #[test]
    fn v6_write_flash_streams_chunks() {
        let mut da = synced_xml_session();
        let data: Vec<u8> = (0..0x300).map(|i| (i * 11 % 251) as u8).collect();
        da.write_flash_region("user", 0x1_0000, &data).unwrap();
    }

    #[test]
    fn v6_write_partition_answers_the_size_query() {
        let mut da = synced_xml_session();
        let data = vec![0x5Au8; 0x500];
        da.write_partition_named("userdata", &data).unwrap();
    }

    #[test]
    fn v6_erase_reports_progress() {
        let mut da = synced_xml_session();
        da.erase_partition_named("cache").unwrap();
        da.erase_flash_region("user", 0x4000, 0x1000).unwrap();
    }

    #[test]
    fn v6_read_resumes_after_a_silent_link() {
        let mut collected: Vec<u8> = Vec::new();
        let opens = std::cell::Cell::new(0usize);

        // session 1 answers one read and goes silent
        let mut first_da = {
            let mut da = XmlDa::new(Box::new(mock_xml_da_muted(1)));
            da.get_command_result().unwrap();
            da.init_session().unwrap();
            Some(da)
        };

        let mut reopen = || -> Result<XmlDa> {
            opens.set(opens.get() + 1);
            if let Some(da) = first_da.take() {
                return Ok(da);
            }
            let mut da = XmlDa::new(Box::new(mock_xml_da()));
            da.get_command_result().unwrap();
            da.init_session().unwrap();
            Ok(da)
        };
        let mut sink = |pos: usize, data: &[u8]| {
            let _ = pos;
            collected.extend_from_slice(data);
            Ok(())
        };
        let read = XmlDa::read_flash_resumable(0x300, 0x100, &mut sink, &mut reopen).unwrap();
        assert_eq!(read, 0x300);
        assert_eq!(collected.len(), 0x300);
        // the offset-aware pattern proves the continuation started
        // exactly where the first session dropped
        assert_eq!(collected[0x200], (0x200 * 7 % 253) as u8);
        assert_eq!(collected[0x2FF], (0x2FF * 7 % 253) as u8);
        assert_eq!(opens.get(), 2);
    }
}
