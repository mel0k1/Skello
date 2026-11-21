//! MediaTek BROM/Preloader byte protocol over the USB CDC port:
//! handshake, hardware code query, DA upload and jump.
//! Steps and constants follow mtkclient (Apache-2.0), NOTICE has the
//! attribution.

use std::io;

use crate::transport::Link;

/// Handshake bytes: the device echoes each byte inverted.
pub const HANDSHAKE: [u8; 4] = [0xA0, 0x0A, 0x50, 0x05];

/// Bootloader command bytes (mtkclient `Cmd`).
pub const CMD_JUMP_DA: u8 = 0xD5;
pub const CMD_SEND_DA: u8 = 0xD7;
pub const CMD_GET_HW_CODE: u8 = 0xFD;

/// USB product ids of the bootloader ports (vendor 0x0E8D).
pub const VID_MEDIATEK: u16 = 0x0E8D;
pub const PID_BROM: u16 = 0x0003;
pub const PID_PRELOADER: u16 = 0x2000;
pub const PID_DA: u16 = 0x2001;

/// Timeouts: commands are quick, DA upload and jump are slower.
const CMD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
const DA_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const JUMP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

#[derive(Debug)]
pub enum BromError {
    Io(std::io::Error),
    /// Handshake echo did not invert correctly within the retries.
    NoHandshake,
    /// The device returned a non-zero status or wrong echo.
    Unexpected(&'static str),
}

impl std::fmt::Display for BromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BromError::Io(e) => write!(f, "brom io: {e}"),
            BromError::NoHandshake => write!(f, "brom handshake failed"),
            BromError::Unexpected(s) => write!(f, "brom unexpected response: {s}"),
        }
    }
}

impl std::error::Error for BromError {}

impl From<std::io::Error> for BromError {
    fn from(e: std::io::Error) -> Self {
        BromError::Io(e)
    }
}

impl From<crate::transport::LinkError> for BromError {
    fn from(e: crate::transport::LinkError) -> Self {
        BromError::Io(std::io::Error::other(e.to_string()))
    }
}

type Result<T> = std::result::Result<T, BromError>;

/// A BROM or Preloader session over an open byte link.
pub struct Brom {
    link: Box<dyn Link>,
}

impl Brom {
    pub fn new(link: Box<dyn Link>) -> Self {
        Self { link }
    }

    /// Runs the four-byte handshake: every byte must come back
    /// inverted, up to 10 attempts.
    pub fn handshake(&mut self) -> Result<()> {
        for _ in 0..10 {
            let mut ok = true;
            for byte in HANDSHAKE {
                self.link.write_all(&[byte])?;
                let mut echo = [0u8; 1];
                let n = self.link.read(&mut echo, CMD_TIMEOUT)?;
                if n != 1 || echo[0] != !byte {
                    ok = false;
                    break;
                }
            }
            if ok {
                return Ok(());
            }
        }
        Err(BromError::NoHandshake)
    }

    fn echo_block(&mut self, data: &[u8], timeout: std::time::Duration) -> Result<()> {
        self.link.write_all(data)?;
        let mut echo = vec![0u8; data.len()];
        let n = self.link.read(&mut echo, timeout)?;
        if n != data.len() || echo != data {
            return Err(BromError::Unexpected("echo mismatch"));
        }
        Ok(())
    }

    fn read_be16(&mut self, timeout: std::time::Duration) -> Result<u16> {
        let mut buf = [0u8; 2];
        read_exact_timeout(&mut *self.link, &mut buf, timeout)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// `GET_HW_CODE`: returns (hw_code, hw_sub_code) of the chip.
    pub fn get_hw_code(&mut self) -> Result<(u16, u16)> {
        self.echo_block(&[CMD_GET_HW_CODE], CMD_TIMEOUT)?;
        let mut buf = [0u8; 4];
        read_exact_timeout(&mut *self.link, &mut buf, CMD_TIMEOUT)?;
        Ok((
            u16::from_be_bytes([buf[0], buf[1]]),
            u16::from_be_bytes([buf[2], buf[3]]),
        ))
    }

    /// `SEND_DA`: uploads the DA image to `address` with a trailing
    /// signature; the device echoes every parameter and answers the
    /// XOR16 checksum plus a status word.
    pub fn send_da(&mut self, address: u32, sig_len: u32, data: &[u8]) -> Result<()> {
        self.echo_block(&[CMD_SEND_DA], CMD_TIMEOUT)?;
        self.echo_block(&address.to_be_bytes(), CMD_TIMEOUT)?;
        self.echo_block(&(data.len() as u32).to_be_bytes(), CMD_TIMEOUT)?;
        self.echo_block(&sig_len.to_be_bytes(), CMD_TIMEOUT)?;

        let status = self.read_be16(DA_TIMEOUT)?;
        if status != 0 {
            return Err(BromError::Unexpected("send_da status"));
        }
        let checksum = xor16(data);
        self.upload(data)?;
        let mut tail = [0u8; 4];
        read_exact_timeout(&mut *self.link, &mut tail, DA_TIMEOUT)?;
        let device_checksum = u16::from_be_bytes([tail[0], tail[1]]);
        let device_status = u16::from_be_bytes([tail[2], tail[3]]);
        if device_status != 0 {
            return Err(BromError::Unexpected("upload status"));
        }
        if device_checksum != checksum && device_checksum != 0 {
            return Err(BromError::Unexpected("upload checksum"));
        }
        Ok(())
    }

    fn upload(&mut self, data: &[u8]) -> Result<()> {
        for chunk in data.chunks(16 * 1024) {
            self.link.write_all(chunk)?;
        }
        Ok(())
    }

    /// `JUMP_DA`: hands control to the DA at `address`.
    pub fn jump_da(&mut self, address: u32) -> Result<()> {
        self.echo_block(&[CMD_JUMP_DA], CMD_TIMEOUT)?;
        self.link.write_all(&address.to_be_bytes())?;
        let mut buf = [0u8; 4];
        read_exact_timeout(&mut *self.link, &mut buf, JUMP_TIMEOUT)?;
        if u32::from_be_bytes(buf) != address {
            return Err(BromError::Unexpected("jump address echo"));
        }
        let status = self.read_be16(JUMP_TIMEOUT)?;
        if status != 0 {
            return Err(BromError::Unexpected("jump status"));
        }
        Ok(())
    }
}

fn read_exact_timeout(
    link: &mut dyn Link,
    buf: &mut [u8],
    timeout: std::time::Duration,
) -> io::Result<()> {
    let mut done = 0;
    let deadline = std::time::Instant::now() + timeout;
    while done < buf.len() {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "link read timeout"));
        }
        let n = link.read(&mut buf[done..], left)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "link closed"));
        }
        done += n;
    }
    Ok(())
}

/// The checksum the BROM expects for DA data: XOR of little-endian
/// u16 words over the whole buffer.
pub fn xor16(data: &[u8]) -> u16 {
    let mut crc = 0u16;
    let mut i = 0;
    while i + 1 < data.len() {
        crc ^= u16::from_le_bytes([data[i], data[i + 1]]);
        i += 2;
    }
    if data.len() & 1 != 0 {
        crc ^= data[data.len() - 1] as u16;
    }
    crc
}

pub(crate) fn read_exact_t(
    link: &mut dyn Link,
    buf: &mut [u8],
    timeout: std::time::Duration,
) -> io::Result<()> {
    read_exact_timeout(link, buf, timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockLink;
    use std::cell::RefCell;

    /// Device side of the BROM: inverted handshake echo, hardware code,
    /// DA upload with XOR16 checksum, jump echo.
    fn mock_brom() -> MockLink {
        let mut link = MockLink::new();
        let state = RefCell::new((0usize, 0u8, 0u8, 0usize, Vec::new()));
        link.device = Some(Box::new(move |out, input| {
            let mut st = state.borrow_mut();
            let (hs_pos, last_cmd, step, pending_size, payload) = &mut *st;
            if input.is_empty() {
                return;
            }
            // Handshake bytes arrive one per write, inverted echo back.
            if input.len() == 1 && *hs_pos < HANDSHAKE.len() && HANDSHAKE[*hs_pos] == input[0] {
                out.push_back(!input[0]);
                *hs_pos += 1;
                return;
            }
            *hs_pos = 0;
            // Echo stages of SEND_DA: cmd, address, size, sig_len.
            // The size parameter (stage 1) is remembered for the
            // payload checksum answer.
            if *last_cmd == CMD_SEND_DA && input.len() == 4 && *step < 3 {
                if *step == 1 {
                    *pending_size = u32::from_be_bytes(input.try_into().unwrap()) as usize;
                }
                out.extend(input);
                *step += 1;
                if *step == 3 {
                    out.extend(0u16.to_be_bytes());
                    *step = 4;
                }
                return;
            }
            match input[0] {
                CMD_GET_HW_CODE => {
                    *last_cmd = 0;
                    out.extend([CMD_GET_HW_CODE]);
                    out.extend([0x65, 0x72, 0x00, 0x02]);
                }
                CMD_SEND_DA => {
                    *last_cmd = CMD_SEND_DA;
                    *step = 0;
                    payload.clear();
                    out.extend([CMD_SEND_DA]);
                }
                CMD_JUMP_DA => {
                    *last_cmd = CMD_JUMP_DA;
                    *step = 0;
                    out.extend([CMD_JUMP_DA]);
                }
                _ => {
                    match *last_cmd {
                        CMD_SEND_DA if *step == 4 => {
                            // DA payload upload
                            payload.extend_from_slice(input);
                            if payload.len() == *pending_size {
                                out.extend(xor16(payload).to_be_bytes());
                                out.extend(0u16.to_be_bytes());
                                *last_cmd = 0;
                            }
                        }
                        CMD_JUMP_DA => {
                            // address echo + status
                            out.extend(input);
                            out.extend(0u16.to_be_bytes());
                            *last_cmd = 0;
                        }
                        _ => {}
                    }
                }
            }
        }));
        link
    }

    #[test]
    fn handshake_echoes_inverted() {
        let mock = mock_brom();
        let mut brom = Brom::new(Box::new(mock));
        assert!(brom.handshake().is_ok());
    }

    #[test]
    fn handshake_fails_on_wrong_echo() {
        let mut mock = MockLink::new();
        mock.push(&[0x5F, 0x5F, 0x5F, 0x5F]);
        let mut brom = Brom::new(Box::new(mock));
        assert!(matches!(brom.handshake(), Err(BromError::NoHandshake)));
    }

    #[test]
    fn hw_code_roundtrip() {
        let mock = mock_brom();
        let mut brom = Brom::new(Box::new(mock));
        brom.handshake().unwrap();
        let (code, sub) = brom.get_hw_code().unwrap();
        assert_eq!(code, 0x6572);
        assert_eq!(sub, 0x0002);
    }

    #[test]
    fn send_da_and_jump_roundtrip() {
        let mock = mock_brom();
        let mut brom = Brom::new(Box::new(mock));
        brom.handshake().unwrap();
        let payload: Vec<u8> = (0..300).map(|i| (i * 7) as u8).collect();
        brom.send_da(0x200000, 4, &payload).unwrap();
        brom.jump_da(0x200000).unwrap();
    }

    #[test]
    fn send_da_rejects_bad_checksum() {
        let mut mock = MockLink::new();
        let hs = RefCell::new((0usize, 0usize));
        let broken: crate::transport::DeviceFn = Box::new(move |out, input| {
            let mut st = hs.borrow_mut();
            if input.len() == 1 && st.0 < HANDSHAKE.len() && HANDSHAKE[st.0] == input[0] {
                out.push_back(!input[0]);
                st.0 += 1;
                return;
            }
            st.0 = 0;
            if input[0] == CMD_SEND_DA && input.len() == 1 {
                out.extend(input);
            } else if input.len() == 4 {
                // address, size, sig_len echoes; status after the third
                out.extend(input);
                st.1 += 1;
                if st.1 == 3 {
                    out.extend(0u16.to_be_bytes());
                }
            } else if input.len() > 4 {
                out.extend(0xBEEFu16.to_be_bytes());
                out.extend(0u16.to_be_bytes());
            }
        });
        mock.device = Some(broken);
        let mut brom = Brom::new(Box::new(mock));
        brom.handshake().unwrap();
        let payload: Vec<u8> = (0..300).map(|i| (i * 7) as u8).collect();
        assert!(matches!(
            brom.send_da(0x200000, 4, &payload),
            Err(BromError::Unexpected("upload checksum"))
        ));
    }

    #[test]
    fn xor16_matches_reference() {
        // XOR of LE words: 0x0102 ^ 0x0304 ^ (trailing 0x05)
        assert_eq!(xor16(&[1, 2, 3, 4, 5]), 0x0201 ^ 0x0403 ^ 0x05);
    }
}
