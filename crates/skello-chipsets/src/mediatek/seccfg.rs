//! MediaTek seccfg partition format (V3 and V4) and the software SEJ mode.
//!
//! Format and constants follow mtkclient (Apache-2.0, bkerler/mtkclient,
//! mtkclient/Library/Hardware/seccfg.py); see NOTICE for attribution.
//! V3 devices with the software SEJ key are patched locally; anything
//! hardware-keyed (SEJ HW, V2/V3/V4 variants) reports `NeedsSej` - that
//! path requires the Download Agent stage and is not implemented yet.

use std::fmt;

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use sha2::{Digest, Sha256};

/// Magic of the V3/V4 seccfg structure ("MMMM").
const MAGIC: u32 = 0x4D4D_4D4D;
/// End marker ("EEEE").
const ENDFLAG: u32 = 0x4545_4545;
/// V3 info header prefix: "AND_SECCFG_v" plus four zero bytes.
const V3_INFO_HEADER: [u8; 16] = *b"AND_SECCFG_v\0\0\0\0";
const V3_HEADER_SZ: usize = 0x2C;
const V3_IMGINFO_SZ: usize = 0x68 * 20;

/// V3 lock marker: enc_len 0x01000000 means locked, 0x07F20000 unlocked.
const V3_ENC_LEN_LOCKED: u32 = 0x0100_0000;
const V3_ENC_LEN_UNLOCKED: u32 = 0x07F2_0000;

/// Body status markers ("CCCC" complete / "IIII" incomplete).
const STATUS_COMPLETE: u32 = 0x4343_4343;
const STATUS_INCOMPLETE: u32 = 0x4949_4949;

/// V3 body attributes.
const ATTR_DEFAULT: u32 = 0x3333_3333;
const ATTR_UNLOCK: u32 = 0x4444_4444;
const ATTR_LOCK: u32 = 0x6000;
const ATTR_VERIFIED: u32 = 0x6001;
const ATTR_CUSTOM: u32 = 0x6002;
const ATTR_MP_DEFAULT: u32 = 0x6003;

/// Software SEJ key and IV from mtkclient (`sej_sec_cfg_sw`), used by
/// devices whose seccfg was written without a hardware-derived key.
const SEJ_SW_KEY: &[u8; 32] = b"25A1763A21BC854CD569DC23B4782B63";
const SEJ_SW_IV: [u8; 16] = [
    0x57, 0x32, 0x5A, 0x5A, 0x12, 0x54, 0x97, 0x66, 0x12, 0x54, 0x97, 0x66, 0x57, 0x32, 0x5A, 0x5A,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecCfgError {
    NotSecCfg,
    Truncated(&'static str),
    BadHeader(&'static str),
    /// The partition is hardware-SEJ-protected; patching needs the
    /// Download Agent stage with the device's SEJ hardware.
    NeedsSej(&'static str),
    /// The DA SEJ engine returned an error.
    Engine(String),
    BadLength(usize),
}

impl fmt::Display for SecCfgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecCfgError::NotSecCfg => write!(f, "image is not a seccfg partition"),
            SecCfgError::Truncated(s) => write!(f, "seccfg truncated at {s}"),
            SecCfgError::BadHeader(s) => write!(f, "seccfg bad header: {s}"),
            SecCfgError::NeedsSej(s) => {
                write!(f, "SEJ hardware required, not available locally: {s}")
            }
            SecCfgError::Engine(s) => write!(f, "SEJ engine: {s}"),
            SecCfgError::BadLength(n) => write!(f, "seccfg data length {n} is not 16-byte aligned"),
        }
    }
}

impl std::error::Error for SecCfgError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecCfgVersion {
    V3,
    V4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    Locked,
    Unlocked,
    Unknown(u32),
}

impl LockState {
    fn from_v4(value: u32) -> Self {
        match value {
            1 => LockState::Locked,
            3 => LockState::Unlocked,
            other => LockState::Unknown(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecCfgInfo {
    pub version: SecCfgVersion,
    pub state: LockState,
    /// Human-readable detail: for V3 the body attribute, for V4 the raw
    /// lock field and verity state.
    pub detail: String,
}

/// Raw AES-256-CBC without padding, the mtkclient software SEJ mode.
pub fn sej_sw(data: &[u8], encrypt: bool) -> Result<Vec<u8>, SecCfgError> {
    if !data.len().is_multiple_of(16) {
        return Err(SecCfgError::BadLength(data.len()));
    }
    let cipher = aes::Aes256::new(GenericArray::from_slice(SEJ_SW_KEY));
    let mut iv = SEJ_SW_IV;
    let mut out = vec![0u8; data.len()];
    for (index, block) in data.chunks(16).enumerate() {
        let mut buf = [0u8; 16];
        buf.copy_from_slice(block);
        let ga = GenericArray::from_mut_slice(&mut buf);
        if encrypt {
            for (a, b) in ga.iter_mut().zip(iv.iter()) {
                *a ^= b;
            }
            cipher.encrypt_block(ga);
            iv.copy_from_slice(ga);
        } else {
            let mut saved = [0u8; 16];
            saved.copy_from_slice(ga);
            cipher.decrypt_block(ga);
            for (a, b) in ga.iter_mut().zip(iv.iter()) {
                *a ^= b;
            }
            iv.copy_from_slice(&saved);
        }
        out[index * 16..index * 16 + 16].copy_from_slice(ga);
    }
    Ok(out)
}

fn read32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(data[off..off + 4].try_into().unwrap())
}

fn attr_name(attr: u32) -> String {
    match attr {
        ATTR_DEFAULT => "default".to_owned(),
        ATTR_UNLOCK => "unlocked".to_owned(),
        ATTR_LOCK => "locked".to_owned(),
        ATTR_VERIFIED => "verified".to_owned(),
        ATTR_CUSTOM => "custom".to_owned(),
        ATTR_MP_DEFAULT => "mp default".to_owned(),
        other => format!("unknown ({other:#x})"),
    }
}

/// Parses a seccfg partition image and reports version and lock state.
pub fn parse(indata: &[u8]) -> Result<SecCfgInfo, SecCfgError> {
    if indata.len() >= 16 && indata[..16] == V3_INFO_HEADER {
        parse_v3(indata).map(|(state, detail)| SecCfgInfo {
            version: SecCfgVersion::V3,
            state,
            detail,
        })
    } else if indata.len() >= 28 && read32(indata, 0) == MAGIC {
        parse_v4(indata)
    } else {
        Err(SecCfgError::NotSecCfg)
    }
}

fn parse_v3(indata: &[u8]) -> Result<(LockState, String), SecCfgError> {
    if indata.len() < V3_HEADER_SZ + 4 {
        return Err(SecCfgError::Truncated("header"));
    }
    let magic = read32(indata, 16);
    let _ver = read32(indata, 20);
    let size = read32(indata, 24);
    let _enc_offset = read32(indata, 28);
    let enc_len = read32(indata, 32);
    let body_len = (size as usize)
        .checked_sub(V3_HEADER_SZ + 4)
        .ok_or(SecCfgError::Truncated("body"))?;
    let end_pos = V3_HEADER_SZ + body_len;
    if indata.len() < end_pos {
        return Err(SecCfgError::Truncated("body"));
    }
    if magic != MAGIC || read32(indata, end_pos) != ENDFLAG {
        return Err(SecCfgError::BadHeader("magic or end flag"));
    }
    let body = &indata[V3_HEADER_SZ..end_pos];
    let plain = sej_sw(body, false)?;
    if plain.len() < V3_IMGINFO_SZ + 12 {
        return Err(SecCfgError::Truncated("v3 imginfo"));
    }
    let status = read32(&plain, V3_IMGINFO_SZ + 4);
    let attr = read32(&plain, V3_IMGINFO_SZ + 8);
    let marker_ok = status == STATUS_COMPLETE || status == STATUS_INCOMPLETE;
    if !marker_ok {
        return Err(SecCfgError::NeedsSej(
            "body is not software-SEJ encrypted (hardware key)",
        ));
    }
    let state = match (enc_len, attr) {
        (V3_ENC_LEN_UNLOCKED, ATTR_UNLOCK) => LockState::Unlocked,
        (V3_ENC_LEN_LOCKED, ATTR_UNLOCK) => LockState::Unlocked,
        _ => {
            if attr == ATTR_LOCK || attr == ATTR_DEFAULT || attr == ATTR_MP_DEFAULT {
                LockState::Locked
            } else {
                LockState::Unknown(attr)
            }
        }
    };
    Ok((state, format!("body attr {}", attr_name(attr))))
}

fn parse_v4(indata: &[u8]) -> Result<SecCfgInfo, SecCfgError> {
    let size = read32(indata, 8) as usize;
    let hash_pos = size.saturating_sub(0x20);
    if indata.len() < hash_pos + 0x20 {
        return Err(SecCfgError::Truncated("hash"));
    }
    if read32(indata, 24) != ENDFLAG {
        return Err(SecCfgError::BadHeader("end flag"));
    }
    let lock_state = read32(indata, 12);
    let dm_verity = read32(indata, 16);
    Ok(SecCfgInfo {
        version: SecCfgVersion::V4,
        state: LockState::from_v4(lock_state),
        detail: format!("dm-verity {dm_verity}, lock field {lock_state}"),
    })
}

/// Rewrites the lock state of a seccfg image. Only V3 software-SEJ
/// partitions are supported locally; anything else returns `NeedsSej`.
pub fn patch(indata: &[u8], unlock: bool) -> Result<Vec<u8>, SecCfgError> {
    if indata.len() >= 16 && indata[..16] == V3_INFO_HEADER {
        patch_v3(indata, unlock)
    } else {
        Err(SecCfgError::NeedsSej(
            "V4 and hardware-keyed V3 partitions are patched through the DA stage",
        ))
    }
}

fn patch_v3(indata: &[u8], unlock: bool) -> Result<Vec<u8>, SecCfgError> {
    let (state, _detail) = parse_v3(indata)?;
    if unlock && state == LockState::Unlocked {
        return Err(SecCfgError::BadHeader("already unlocked"));
    }
    if !unlock && state == LockState::Locked {
        return Err(SecCfgError::BadHeader("already locked"));
    }
    let size = read32(indata, 24) as usize;
    let body_len = size - V3_HEADER_SZ - 4;
    let body = &indata[V3_HEADER_SZ..V3_HEADER_SZ + body_len];
    let mut plain = sej_sw(body, false)?;
    let mut header = indata[..V3_HEADER_SZ].to_vec();
    header[32..36].copy_from_slice(&if unlock {
        V3_ENC_LEN_UNLOCKED.to_le_bytes()
    } else {
        V3_ENC_LEN_LOCKED.to_le_bytes()
    });
    let attr_offset = V3_IMGINFO_SZ + 8;
    plain[attr_offset..attr_offset + 4].copy_from_slice(&if unlock {
        ATTR_UNLOCK.to_le_bytes()
    } else {
        ATTR_DEFAULT.to_le_bytes()
    });
    let encrypted = sej_sw(&plain, true)?;
    let mut out = Vec::with_capacity(size + 0x200);
    out.extend_from_slice(&header);
    out.extend_from_slice(&encrypted);
    out.extend_from_slice(&ENDFLAG.to_le_bytes());
    while out.len() % 0x200 != 0 {
        out.push(0);
    }
    Ok(out)
}

/// Which SEJ mode encrypted the trailing hash of a V4 seccfg.
/// `V3` is the plain hash variant, `V4Legacy` the older derivation
/// (mtkclient hwtype names "V3" and "V4"); the true per-device keys
/// (mtkclient "HW"/"HWXOR") need the otp and stay out of scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SejMode {
    Sw,
    V2,
    V3,
    V4Legacy,
}

/// The raw V4 fields, kept for the patch path.
#[derive(Debug, Clone)]
pub struct V4Fields {
    pub magic: u32,
    pub ver: u32,
    pub size: u32,
    pub lock_state: u32,
    pub dm_verity: u32,
    pub sboot: u32,
    /// The stored hash, SEJ-encrypted SHA256 of the 28-byte header.
    pub enc_hash: Vec<u8>,
}

impl V4Fields {
    /// The 28-byte header this structure hashes over.
    fn header_bytes(&self, lock_state: u32, dm_verity: u32) -> [u8; 28] {
        let mut out = [0u8; 28];
        out[0..4].copy_from_slice(&self.magic.to_le_bytes());
        out[4..8].copy_from_slice(&self.ver.to_le_bytes());
        out[8..12].copy_from_slice(&self.size.to_le_bytes());
        out[12..16].copy_from_slice(&lock_state.to_le_bytes());
        out[16..20].copy_from_slice(&dm_verity.to_le_bytes());
        out[20..24].copy_from_slice(&self.sboot.to_le_bytes());
        out[24..28].copy_from_slice(&ENDFLAG.to_le_bytes());
        out
    }

    fn expected_hash(&self, lock_state: u32, dm_verity: u32) -> [u8; 32] {
        Sha256::digest(self.header_bytes(lock_state, dm_verity)).into()
    }
}

/// The V4 hash modes beyond `Sw` need the device SEJ engine: the key
/// is HUID-bound, so the round trips run through the DA registers.
pub trait V4Signer {
    /// Decrypts a stored hash with the hardware engine for the mode.
    fn decrypt(&mut self, data: &[u8], mode: SejMode) -> Result<Vec<u8>, SecCfgError>;
    /// Encrypts a fresh hash with the hardware engine for the mode.
    fn encrypt(&mut self, data: &[u8], mode: SejMode) -> Result<Vec<u8>, SecCfgError>;
}

fn parse_v4_fields(indata: &[u8]) -> Result<V4Fields, SecCfgError> {
    if indata.len() < 28 {
        return Err(SecCfgError::Truncated("v4 header"));
    }
    let fields = V4Fields {
        magic: read32(indata, 0),
        ver: read32(indata, 4),
        size: read32(indata, 8),
        lock_state: read32(indata, 12),
        dm_verity: read32(indata, 16),
        sboot: read32(indata, 20),
        enc_hash: Vec::new(),
    };
    if fields.magic != MAGIC || read32(indata, 24) != ENDFLAG {
        return Err(SecCfgError::BadHeader("v4 magic or end flag"));
    }
    let size = fields.size as usize;
    let hash_pos = size.saturating_sub(0x20);
    if size < 0x20 || indata.len() < hash_pos + 0x20 {
        return Err(SecCfgError::Truncated("v4 hash"));
    }
    let mut fields = fields;
    fields.enc_hash = indata[hash_pos..hash_pos + 0x20].to_vec();
    Ok(fields)
}

/// Detects which SEJ mode encrypted the stored V4 hash: `Sw` runs in
/// software, the rest through the `signer` (the DA SEJ engine).
pub fn detect_v4_mode(
    fields: &V4Fields,
    signer: &mut Option<&mut dyn V4Signer>,
) -> Result<SejMode, SecCfgError> {
    let expected = fields.expected_hash(fields.lock_state, fields.dm_verity);
    let sw_plain = sej_sw(&fields.enc_hash, false)?;
    if sw_plain[..32] == expected {
        return Ok(SejMode::Sw);
    }
    let Some(signer) = signer.as_mut() else {
        return Err(SecCfgError::NeedsSej(
            "V4 hash is not software-SEJ; the DA SEJ engine is required",
        ));
    };
    for mode in [SejMode::V3, SejMode::V4Legacy, SejMode::V2] {
        let plain = signer.decrypt(&fields.enc_hash, mode)?;
        if plain.len() >= 32 && plain[..32] == expected {
            return Ok(mode);
        }
    }
    Err(SecCfgError::NeedsSej(
        "V4 hash matches no known SEJ mode (device-keyed hash)",
    ))
}

/// Rewrites the lock state of a V4 seccfg image: recompute the header
/// SHA256 and re-encrypt it with the detected SEJ mode. `Sw` signs in
/// software; other modes go through the `signer` (DA SEJ engine).
pub fn patch_v4(
    indata: &[u8],
    unlock: bool,
    signer: &mut Option<&mut dyn V4Signer>,
) -> Result<Vec<u8>, SecCfgError> {
    let fields = parse_v4_fields(indata)?;
    let mode = detect_v4_mode(&fields, signer)?;
    match (unlock, fields.lock_state) {
        (true, 3) => return Err(SecCfgError::BadHeader("already unlocked")),
        (false, 1) => return Err(SecCfgError::BadHeader("already locked")),
        _ => {}
    }
    let new_lock = if unlock { 3u32 } else { 1u32 };
    let fresh = fields.expected_hash(new_lock, fields.dm_verity);
    let enc_hash = match mode {
        SejMode::Sw => sej_sw(&fresh, true)?,
        other => {
            let Some(s) = signer.as_mut() else {
                return Err(SecCfgError::NeedsSej(
                    "hash needs the DA SEJ engine, none attached",
                ));
            };
            s.encrypt(&fresh, other)?
        }
    };
    // The V4 hash lives at size-0x20 (the mtkclient layout); the
    // rebuilt image is 0x3C bytes: 28 header + 32 hash. Force the
    // size field so the on-device check looks where the hash is.
    let mut header = fields.header_bytes(new_lock, fields.dm_verity);
    header[8..12].copy_from_slice(&0x3Cu32.to_le_bytes());
    let mut out = Vec::with_capacity(0x3C + 0x200);
    out.extend_from_slice(&header);
    out.extend_from_slice(&enc_hash);
    while out.len() % 0x200 != 0 {
        out.push(0);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v3_image(attr: u32, enc_len: u32) -> Vec<u8> {
        let mut plain = vec![0u8; 0x820 + 12 + 0x1004];
        plain[V3_IMGINFO_SZ + 8..V3_IMGINFO_SZ + 12].copy_from_slice(&attr.to_le_bytes());
        plain[V3_IMGINFO_SZ + 4..V3_IMGINFO_SZ + 8].copy_from_slice(&STATUS_COMPLETE.to_le_bytes());
        let body = sej_sw(&plain, true).unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(&V3_INFO_HEADER);
        out.extend_from_slice(&MAGIC.to_le_bytes());
        out.extend_from_slice(&3u32.to_le_bytes());
        out.extend_from_slice(&((V3_HEADER_SZ + body.len() + 4) as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&enc_len.to_le_bytes());
        out.extend_from_slice(&[0u8, 0]);
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&body);
        out.extend_from_slice(&ENDFLAG.to_le_bytes());
        out
    }

    #[test]
    fn sej_sw_roundtrip() {
        let data: Vec<u8> = (0..64).map(|i| (i * 7) as u8).collect();
        let enc = sej_sw(&data, true).unwrap();
        assert_ne!(enc, data);
        let dec = sej_sw(&enc, false).unwrap();
        assert_eq!(dec, data);
        assert!(matches!(
            sej_sw(&data[..10], true),
            Err(SecCfgError::BadLength(10))
        ));
    }

    #[test]
    fn parses_v3_sw_state() {
        let locked = parse(&v3_image(ATTR_DEFAULT, V3_ENC_LEN_LOCKED)).unwrap();
        assert_eq!(locked.version, SecCfgVersion::V3);
        assert_eq!(locked.state, LockState::Locked);
        let unlocked = parse(&v3_image(ATTR_UNLOCK, V3_ENC_LEN_UNLOCKED)).unwrap();
        assert_eq!(unlocked.state, LockState::Unlocked);
        assert!(unlocked.detail.contains("unlocked"));
    }

    #[test]
    fn patches_v3_sw_unlock_and_lock() {
        let image = v3_image(ATTR_DEFAULT, V3_ENC_LEN_LOCKED);
        let patched = patch(&image, true).unwrap();
        let info = parse(&patched).unwrap();
        assert_eq!(info.state, LockState::Unlocked);
        assert_eq!(patched.len() % 0x200, 0);
        let back = patch(&patched, false).unwrap();
        let info = parse(&back).unwrap();
        assert_eq!(info.state, LockState::Locked);
    }

    #[test]
    fn refuses_noop_patch() {
        let unlocked = v3_image(ATTR_UNLOCK, V3_ENC_LEN_UNLOCKED);
        assert!(matches!(
            patch(&unlocked, true),
            Err(SecCfgError::BadHeader("already unlocked"))
        ));
    }

    #[test]
    fn hardware_sej_reports_needs_sej() {
        // A V4 image: parse works, patch is out of scope without SEJ hardware.
        let mut v4 = Vec::new();
        v4.extend_from_slice(&MAGIC.to_le_bytes());
        v4.extend_from_slice(&4u32.to_le_bytes());
        v4.extend_from_slice(&0x40u32.to_le_bytes());
        v4.extend_from_slice(&3u32.to_le_bytes());
        v4.extend_from_slice(&0u32.to_le_bytes());
        v4.extend_from_slice(&0u32.to_le_bytes());
        v4.extend_from_slice(&ENDFLAG.to_le_bytes());
        v4.extend_from_slice(&[0u8; 4]);
        v4.extend_from_slice(&[0xEE; 0x20]);
        let info = parse(&v4).unwrap();
        assert_eq!(info.version, SecCfgVersion::V4);
        assert_eq!(info.state, LockState::Unlocked);
        assert!(matches!(patch(&v4, true), Err(SecCfgError::NeedsSej(_))));
        // Hardware-keyed V3 body: corrupt the block carrying the status
        // marker so software decrypt cannot produce valid markers.
        let mut v3hw = v3_image(ATTR_DEFAULT, V3_ENC_LEN_LOCKED);
        v3hw[V3_HEADER_SZ + V3_IMGINFO_SZ + 4] ^= 0xFF;
        assert!(matches!(parse(&v3hw), Err(SecCfgError::NeedsSej(_))));
    }

    #[test]
    fn rejects_foreign_images() {
        assert!(matches!(parse(b"nope"), Err(SecCfgError::NotSecCfg)));
    }

    /// Builds a V4 image whose hash is SEJ-encrypted with the given
    /// signer-equivalent software function (Sw or the hw modes).
    fn v4_image(lock_state: u32, dm_verity: u32, enc: &dyn Fn(&[u8]) -> Vec<u8>) -> Vec<u8> {
        let header = {
            let mut out = [0u8; 28];
            out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
            out[4..8].copy_from_slice(&4u32.to_le_bytes());
            out[8..12].copy_from_slice(&0x3Cu32.to_le_bytes());
            out[12..16].copy_from_slice(&lock_state.to_le_bytes());
            out[16..20].copy_from_slice(&dm_verity.to_le_bytes());
            out[20..24].copy_from_slice(&0u32.to_le_bytes());
            out[24..28].copy_from_slice(&ENDFLAG.to_le_bytes());
            out
        };
        let hash: [u8; 32] = Sha256::digest(header).into();
        let mut out = Vec::new();
        out.extend_from_slice(&header);
        out.extend_from_slice(&enc(&hash));
        while out.len() % 0x200 != 0 {
            out.push(0);
        }
        out
    }

    /// Software stand-in for the DA SEJ engine: AES-128-CBC with the
    /// zero key over the seccfg IV (the `sej_hw_soft` mode used by
    /// chips with the key binding disabled).
    struct SoftSigner;

    impl SoftSigner {
        fn hw(&self, data: &[u8], encrypt: bool) -> Vec<u8> {
            crate::mediatek::sej::sej_hw_soft(data, encrypt).unwrap()
        }
    }

    impl V4Signer for SoftSigner {
        fn decrypt(&mut self, data: &[u8], _mode: SejMode) -> Result<Vec<u8>, SecCfgError> {
            Ok(self.hw(data, false))
        }
        fn encrypt(&mut self, data: &[u8], _mode: SejMode) -> Result<Vec<u8>, SecCfgError> {
            Ok(self.hw(data, true))
        }
    }

    #[test]
    fn v4_sw_hash_patch_unlocks() {
        let image = v4_image(1, 0, &|h| sej_sw(h, true).unwrap());
        let info = parse(&image).unwrap();
        assert_eq!(info.version, SecCfgVersion::V4);
        assert_eq!(info.state, LockState::Locked);
        let patched = patch_v4(&image, true, &mut None).unwrap();
        let info = parse(&patched).unwrap();
        assert_eq!(info.state, LockState::Unlocked);
        assert_eq!(patched.len() % 0x200, 0);
        // round trip back to locked
        let back = patch_v4(&patched, false, &mut None).unwrap();
        assert_eq!(parse(&back).unwrap().state, LockState::Locked);
    }

    #[test]
    fn v4_hw_hash_needs_signer_then_unlocks() {
        // A hash encrypted with the hw mode: without a signer the patch
        // refuses; with the software stand-in it detects V3 and unlocks.
        let image = v4_image(1, 0, &|h| {
            crate::mediatek::sej::sej_hw_soft(h, true).unwrap()
        });
        assert!(matches!(
            patch_v4(&image, true, &mut None),
            Err(SecCfgError::NeedsSej(_))
        ));
        let patched = patch_v4(&image, true, &mut Some(&mut SoftSigner)).unwrap();
        // the patched hash is V3-mode again; the fields flipped to unlock
        assert_eq!(read32(&patched, 12), 3);
        let info = parse(&patched).unwrap();
        assert_eq!(info.state, LockState::Unlocked);
    }

    #[test]
    fn v4_noop_patch_refused() {
        let image = v4_image(3, 0, &|h| sej_sw(h, true).unwrap());
        assert!(matches!(
            patch_v4(&image, true, &mut None),
            Err(SecCfgError::BadHeader("already unlocked"))
        ));
    }
}
