use std::fmt;

pub mod server;

pub const A_SYNC: u32 = 0x434e_5953;
pub const A_CNXN: u32 = 0x4e58_4e43;
pub const A_OPEN: u32 = 0x4e45_504f;
pub const A_OKAY: u32 = 0x5941_4b4f;
pub const A_CLSE: u32 = 0x4553_4c43;
pub const A_WRTE: u32 = 0x4554_5257;
pub const A_AUTH: u32 = 0x4854_5541;
pub const A_STLS: u32 = 0x534c_5453;

pub const A_VERSION: u32 = 0x0100_0001;
pub const A_VERSION_MIN: u32 = 0x0100_0000;

pub const MAX_PAYLOAD_V1: u32 = 4 * 1024;
pub const MAX_PAYLOAD: u32 = 1024 * 1024;

pub const TOKEN_SIZE: usize = 20;

pub const HEADER_SIZE: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub command: u32,
    pub arg0: u32,
    pub arg1: u32,
    pub data_length: u32,
    pub data_check: u32,
    pub magic: u32,
}

impl Message {
    pub fn new(command: u32, arg0: u32, arg1: u32, payload: &[u8]) -> Self {
        Message {
            command,
            arg0,
            arg1,
            data_length: payload.len().min(u32::MAX as usize) as u32,
            data_check: crc32fast::hash(payload),
            magic: !command,
        }
    }

    pub fn encode(&self) -> [u8; HEADER_SIZE] {
        let mut out = [0u8; HEADER_SIZE];
        out[0..4].copy_from_slice(&self.command.to_le_bytes());
        out[4..8].copy_from_slice(&self.arg0.to_le_bytes());
        out[8..12].copy_from_slice(&self.arg1.to_le_bytes());
        out[12..16].copy_from_slice(&self.data_length.to_le_bytes());
        out[16..20].copy_from_slice(&self.data_check.to_le_bytes());
        out[20..24].copy_from_slice(&self.magic.to_le_bytes());
        out
    }

    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < HEADER_SIZE {
            return Err(DecodeError::TooShort(buf.len()));
        }
        let read = |off: usize| u32::from_le_bytes(buf[off..off + 4].try_into().unwrap());
        let msg = Message {
            command: read(0),
            arg0: read(4),
            arg1: read(8),
            data_length: read(12),
            data_check: read(16),
            magic: read(20),
        };
        if msg.magic != !msg.command {
            return Err(DecodeError::BadMagic);
        }
        Ok(msg)
    }

    pub fn command_name(&self) -> &'static str {
        match self.command {
            A_SYNC => "SYNC",
            A_CNXN => "CNXN",
            A_OPEN => "OPEN",
            A_OKAY => "OKAY",
            A_CLSE => "CLSE",
            A_WRTE => "WRTE",
            A_AUTH => "AUTH",
            A_STLS => "STLS",
            _ => "?",
        }
    }
}

#[derive(Debug)]
pub enum DecodeError {
    TooShort(usize),
    BadMagic,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::TooShort(n) => write!(f, "header is {n} bytes, need {HEADER_SIZE}"),
            DecodeError::BadMagic => write!(f, "magic does not match command"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_with_payload() {
        let payload = b"host:devices";
        let msg = Message::new(A_OPEN, 1, 0, payload);
        let decoded = Message::decode(&msg.encode()).unwrap();
        assert_eq!(msg, decoded);
        assert_eq!(decoded.data_length as usize, payload.len());
    }

    #[test]
    fn header_bytes_are_little_endian() {
        let msg = Message::new(A_CNXN, A_VERSION, MAX_PAYLOAD, &[]);
        let hdr = msg.encode();
        assert_eq!(&hdr[0..4], &A_CNXN.to_le_bytes());
        assert_eq!(&hdr[4..8], &A_VERSION.to_le_bytes());
        assert_eq!(&hdr[20..24], &(!A_CNXN).to_le_bytes());
    }

    #[test]
    fn bad_magic_is_rejected() {
        let mut hdr = Message::new(A_WRTE, 0, 0, b"x").encode();
        hdr[20] ^= 0xff;
        assert!(matches!(Message::decode(&hdr), Err(DecodeError::BadMagic)));
    }

    #[test]
    fn short_buffer_is_rejected() {
        assert!(matches!(
            Message::decode(&[0u8; 8]),
            Err(DecodeError::TooShort(8))
        ));
    }

    #[test]
    fn command_names() {
        for (cmd, name) in [
            (A_CNXN, "CNXN"),
            (A_AUTH, "AUTH"),
            (A_OPEN, "OPEN"),
            (A_OKAY, "OKAY"),
            (A_CLSE, "CLSE"),
            (A_WRTE, "WRTE"),
        ] {
            assert_eq!(Message::new(cmd, 0, 0, &[]).command_name(), name);
        }
    }
}
