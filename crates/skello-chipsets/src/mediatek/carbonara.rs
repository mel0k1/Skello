//! Carbonara: the stock DA2 patch and the DA1 hash re-marking that
//! lets a patched stage 2 boot on secure devices. The check patterns,
//! hash position search and the patch table follow mtkclient
//! (Apache-2.0, Library/Exploit/carbonara.py, Library/DA/
//! mtk_daloader.py, Library/mtk_class.py); NOTICE has the
//! attribution. The in-memory hash overwrite runs over the DA
//! session BOOT-TO command, exactly like the reference flow.

use sha1::Sha1;
use sha2::{Digest, Sha256};

/// The anti-Carbonara markers vendors baked into newer loaders: a
/// patched DA1 with any of these cannot be re-marked.
pub fn check_carbonara_patched(da1: &[u8]) -> bool {
    const V6_MARKS: [&[u8]; 4] = [
        b"\x01\x01\x54\xE3\x01\x14\xA0\xE3",
        b"\x08\x00\xa8\x52\xff\x02\x08\xeb",
        b"\x01\x01\x50\xE3\x01\x14\xA0\xE3",
        b"2nd DA address is invalid",
    ];
    for mark in V6_MARKS {
        if find(da1, mark).is_some() {
            return true;
        }
    }
    find(da1, b"\x06\x9B\x4F\xF0\x80\x40\x02\xA9").is_some()
}

/// Finds a byte pattern (no wildcards here; the vendor patches use
/// exact sequences).
fn find(data: &[u8], pattern: &[u8]) -> Option<usize> {
    if pattern.is_empty() || pattern.len() > data.len() {
        return None;
    }
    (0..=data.len() - pattern.len()).find(|&i| &data[i..i + pattern.len()] == pattern)
}

/// The hash digest over the patched DA2 prefix, per the mode the DA1
/// hash table uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashMode {
    Md5,
    Sha1,
    Sha256,
}

pub fn digest(mode: HashMode, data: &[u8]) -> Vec<u8> {
    match mode {
        HashMode::Md5 => {
            use md5::Digest as _;
            md5::Md5::digest(data).to_vec()
        }
        HashMode::Sha1 => Sha1::digest(data).to_vec(),
        HashMode::Sha256 => Sha256::digest(data).to_vec(),
    }
}

/// Searches for the digest of `da2[..hashlen]` inside `da1`.
/// Returns (mode, offset) like the mtkclient calc_da_hash.
pub fn calc_da_hash(da1: &[u8], da2: &[u8]) -> Option<(HashMode, usize)> {
    for (mode, len) in [
        (HashMode::Md5, 16usize),
        (HashMode::Sha1, 20),
        (HashMode::Sha256, 32),
    ] {
        let d = digest(mode, da2);
        if let Some(idx) = find(da1, &d) {
            let _ = len;
            return Some((mode, idx));
        }
    }
    None
}

/// The V5 fallback: the hash sits right before the "MMU MAP: VA"
/// marker, 0x30 bytes (sha256) or 0x14 (sha1).
pub fn find_da_hash_v5(da1: &[u8]) -> Option<(usize, HashMode)> {
    let marker = find(da1, b"MMU MAP: VA")?;
    let hashed = da1.get(marker.saturating_sub(0x30)..marker)?;
    if hashed.len() == 0x30 && hashed[0x2C..] == [0, 0, 0, 0] {
        return Some((marker - 0x30, HashMode::Sha256));
    }
    Some((marker - 0x14, HashMode::Sha1))
}

/// The V6 fallback: the last 0x30 bytes before the signature.
pub fn find_da_hash_v6(da1: &[u8], siglen: usize) -> Option<(usize, HashMode)> {
    let pos = da1.len().checked_sub(siglen)?.checked_sub(0x30)?;
    let window = da1.get(pos..pos + 0x30)?;
    if window[0x2C..] == [0, 0, 0, 0] {
        return Some((pos, HashMode::Sha256));
    }
    None
}

/// The full hash position search: digest match first, then the
/// layout fallbacks (mtkclient compute_hash_pos). Returns
/// (offset, mode, hashlen).
pub fn compute_hash_pos(
    da1: &[u8],
    da2: &[u8],
    da2sig_len: usize,
    v6: bool,
) -> Option<(usize, HashMode, usize)> {
    let hashlen = da2.len().saturating_sub(da2sig_len);
    if let Some((mode, idx)) = calc_da_hash(da1, &da2[..hashlen]) {
        return Some((idx, mode, hashlen));
    }
    let hashlen = da2.len();
    if let Some((mode, idx)) = calc_da_hash(da1, &da2[..hashlen]) {
        return Some((idx, mode, hashlen));
    }
    if v6 {
        let (idx, mode) = find_da_hash_v6(da1, da2sig_len)?;
        return Some((idx, mode, hashlen));
    }
    let (idx, mode) = find_da_hash_v5(da1)?;
    Some((idx, mode, hashlen))
}

/// Rewrites the DA1 hash table bytes for the patched DA2: returns a
/// copy of `da1` with the hash at `hashpos` replaced (the bootrom
/// stage overwrites it in memory through the exploit primitive).
pub fn fix_hash(da1: &[u8], da2: &[u8], hashpos: usize, mode: HashMode, hashlen: usize) -> Vec<u8> {
    let mut out = da1.to_vec();
    let d = digest(mode, &da2[..hashlen.min(da2.len())]);
    if hashpos + d.len() <= out.len() {
        out[hashpos..hashpos + d.len()].copy_from_slice(&d);
    }
    out
}

/// The DA2 security patch table (mtkclient mtk_class.py), applied by
/// exact byte match: DA signature checks, usb download checks, the
/// sec_img_auth early return and the vfy policy stub.
pub fn patch_da2(da2: &[u8]) -> Vec<u8> {
    const PATCHES: [(&str, &str, &str); 8] = [
        ("A3687BB12846", "0123A3602846", "oppo security"),
        (
            "B3F5807F01D1",
            "B3F5807F01D14FF000004FF000007047",
            "mt6739 c30",
        ),
        (
            "B3F5807F04BF4FF4807305F011B84FF0FF307047",
            "B3F5807F04BF4FF480734FF000004FF000007047",
            "regular",
        ),
        ("10B50C680268", "10B5012010BD", "ram blacklist"),
        (
            "08B5104B7B441B681B68",
            "00207047000000000000",
            "seclib_sec_usbdl_enabled",
        ),
        (
            "5072656C6F61646572205374617274",
            "50617463686564204C205374617274",
            "Patched loader msg",
        ),
        (
            "F0B58BB002AE20250C460746",
            "002070470000000000205374617274",
            "sec_img_auth",
        ),
        ("FFC0F3400008BD", "FF4FF0000008BD", "get_vfy_policy"),
    ];
    let mut out = da2.to_vec();
    let mut patched = false;
    for (pattern, replacement, name) in PATCHES {
        let Ok(pattern) = hex::decode(pattern) else {
            continue;
        };
        let Ok(replacement) = hex::decode(replacement) else {
            continue;
        };
        if let Some(idx) = find(&out, &pattern) {
            if idx + replacement.len() <= out.len() {
                out[idx..idx + replacement.len()].copy_from_slice(&replacement);
                patched = true;
                let _ = name;
            }
        }
    }
    if !patched {
        // nothing matched: leave the image untouched, the caller
        // reports honestly
    }
    out
}

/// The vendor markers the XFlash extension also strips: the Huawei
/// rma-state compare and the Oppo auth flag flips.
pub fn patch_da2_vendor(da2: &[u8]) -> Vec<u8> {
    let mut out = da2.to_vec();
    if let Some(idx) = find(&out, b"\x01\x2B\x03\xD1\x01\x23") {
        if idx + 4 <= out.len() {
            out[idx..idx + 4].copy_from_slice(&[0, 0, 0, 0]);
        }
    }
    let oppo_marks: [&[u8]; 3] = [
        b"\xFF\xFF\xFF\xFF\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03",
        b"\x63\x88\x74\x18\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03",
        b"\x01\x3B\x01\x2B\x08\xD9",
    ];
    for mark in oppo_marks {
        let mut pos = 0;
        while let Some(idx) = {
            let rel = find(&out[pos.min(out.len())..], mark);
            rel.map(|r| pos + r)
        } {
            match mark {
                b"\x01\x3B\x01\x2B\x08\xD9" => {
                    if idx + 4 <= out.len() {
                        out[idx..idx + 4].copy_from_slice(&[0x01, 0x20, 0x08, 0xBD]);
                    }
                }
                _ => {
                    if idx + 0x11 <= out.len() {
                        out[idx + 0x10] = 0x01;
                    }
                }
            }
            pos = idx + mark.len();
            if pos >= out.len() {
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anti_carbonara_marks_are_detected() {
        let mut da1 = vec![0x90u8; 0x100];
        assert!(!check_carbonara_patched(&da1));
        da1[0x40..0x48].copy_from_slice(b"\x01\x01\x54\xE3\x01\x14\xA0\xE3");
        assert!(check_carbonara_patched(&da1));
        let mut da2 = vec![0u8; 0x100];
        da2[0x10..0x29].copy_from_slice(b"2nd DA address is invalid");
        assert!(check_carbonara_patched(&da2));
    }

    #[test]
    fn hash_pos_found_by_digest_match() {
        // da1 carries the sha256 of the patched da2 prefix
        let da2: Vec<u8> = (0..0x200u32).map(|i| (i * 13 + 5) as u8).collect();
        let hash = digest(HashMode::Sha256, &da2);
        let mut da1 = vec![0xEEu8; 0x400];
        da1[0x300..0x320].copy_from_slice(&hash);
        let (pos, mode, len) = compute_hash_pos(&da1, &da2, 0, false).unwrap();
        assert_eq!(mode, HashMode::Sha256);
        assert_eq!(len, 0x200);
        assert_eq!(pos, 0x300);
        // the re-marked da1 carries the hash of the patched image
        let patched = patch_da2(&da2);
        let fixed = fix_hash(&da1, &patched, pos, mode, len);
        let new_hash = digest(HashMode::Sha256, &patched);
        assert_eq!(&fixed[0x300..0x320], &new_hash[..]);
    }

    #[test]
    fn v5_fallback_finds_the_marker_layout() {
        // da2 digest does not appear in da1: the V5 layout kicks in
        let da2: Vec<u8> = (0..0x200u32).map(|i| (i * 13 + 5) as u8).collect();
        let mut da1 = vec![0xEEu8; 0x400];
        let marker = 0x380;
        da1[marker..marker + 11].copy_from_slice(b"MMU MAP: VA");
        // the 0x30 window before the marker ends with four zeros
        da1[marker - 4..marker].copy_from_slice(&[0, 0, 0, 0]);
        let (pos, mode, _len) = compute_hash_pos(&da1, &da2, 0, false).unwrap();
        assert_eq!(pos, marker - 0x30);
        assert_eq!(mode, HashMode::Sha256);
    }

    #[test]
    fn v6_fallback_sits_before_the_signature() {
        let da2: Vec<u8> = (0..0x200u32).map(|i| (i * 13 + 5) as u8).collect();
        let mut da1 = vec![0xEEu8; 0x400];
        let sig = 0x100usize;
        let pos = da1.len() - sig - 0x30;
        da1[pos + 0x2C..pos + 0x30].copy_from_slice(&[0, 0, 0, 0]);
        let (found, mode, _len) = compute_hash_pos(&da1, &da2, sig, true).unwrap();
        assert_eq!(found, pos);
        assert_eq!(mode, HashMode::Sha256);
    }

    #[test]
    fn da2_patches_apply() {
        // a da2 carrying the regular check and the sec_img_auth prologue
        let mut da2: Vec<u8> = (0..0x100u32).map(|i| (i ^ 0xA5) as u8).collect();
        let regular = hex::decode("B3F5807F04BF4FF4807305F011B84FF0FF307047").unwrap();
        da2[0x40..0x40 + regular.len()].copy_from_slice(&regular);
        let patched = patch_da2(&da2);
        assert_ne!(patched, da2);
        let replacement = hex::decode("B3F5807F04BF4FF480734FF000004FF000007047").unwrap();
        assert_eq!(&patched[0x40..0x40 + replacement.len()], &replacement[..]);
        // the huawei marker goes silent through the vendor pass
        let mut huawei = vec![0u8; 0x40];
        huawei[0x10..0x16].copy_from_slice(b"\x01\x2B\x03\xD1\x01\x23");
        let out = patch_da2_vendor(&huawei);
        assert_eq!(&out[0x10..0x14], &[0, 0, 0, 0]);
    }
}
