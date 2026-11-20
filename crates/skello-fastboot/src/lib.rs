//! Fastboot protocol: command strings and response parsing.
//!
//! Command spellings and status prefixes follow the AOSP host driver
//! (platform/system/core/fastboot, `constants.h` and `fastboot_driver.cpp`).

use std::fmt;

/// One response line is at most this many bytes, prefix included.
pub const RESPONSE_SZ: usize = 256;
/// The official driver rejects longer commands before sending them.
pub const COMMAND_SZ: usize = 4096;
/// Bulk payload is written in chunks of this size.
pub const TRANSPORT_CHUNK_SIZE: usize = 1024;
/// Seconds the official driver waits for a device response.
pub const RESP_TIMEOUT_SECS: u64 = 30;

pub const CMD_GETVAR: &str = "getvar";
pub const CMD_DOWNLOAD: &str = "download";
pub const CMD_UPLOAD: &str = "upload";
pub const CMD_FLASH: &str = "flash";
pub const CMD_ERASE: &str = "erase";
pub const CMD_BOOT: &str = "boot";
pub const CMD_SET_ACTIVE: &str = "set_active";
pub const CMD_CONTINUE: &str = "continue";
pub const CMD_REBOOT: &str = "reboot";
pub const CMD_OEM: &str = "oem";
pub const CMD_FETCH: &str = "fetch";
pub const CMD_SNAPSHOT_UPDATE: &str = "snapshot-update";

pub const RESP_OKAY: &str = "OKAY";
pub const RESP_FAIL: &str = "FAIL";
pub const RESP_DATA: &str = "DATA";
pub const RESP_INFO: &str = "INFO";
pub const RESP_TEXT: &str = "TEXT";

/// `getvar` keys used by the official tool and useful for safety checks.
pub const VAR_VERSION: &str = "version";
pub const VAR_VERSION_BOOTLOADER: &str = "version-bootloader";
pub const VAR_PRODUCT: &str = "product";
pub const VAR_SERIALNO: &str = "serialno";
pub const VAR_SECURE: &str = "secure";
pub const VAR_UNLOCKED: &str = "unlocked";
pub const VAR_CURRENT_SLOT: &str = "current-slot";
pub const VAR_SLOT_COUNT: &str = "slot-count";
pub const VAR_HAS_SLOT: &str = "has-slot";
pub const VAR_MAX_DOWNLOAD_SIZE: &str = "max-download-size";
pub const VAR_PARTITION_SIZE: &str = "partition-size";
pub const VAR_PARTITION_TYPE: &str = "partition-type";
pub const VAR_IS_LOGICAL: &str = "is-logical";
pub const VAR_BATTERY_SOC_OK: &str = "battery-soc-ok";
pub const VAR_BATTERY_VOLTAGE: &str = "battery-voltage";

#[derive(Debug, PartialEq, Eq)]
pub enum Response {
    /// `OKAY`: command finished, text after the prefix is optional.
    Ok(String),
    /// `FAIL`: command failed, the rest is the error message.
    Fail(String),
    /// `INFO`: progress line, safe to show the user as-is.
    Info(String),
    /// `TEXT`: diagnostics without progress semantics.
    Text(String),
    /// `DATA`: hex size of the payload the device expects next.
    Data(u32),
}

#[derive(Debug)]
pub enum ParseError {
    Empty,
    Unknown(String),
    BadDataSize(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty response"),
            ParseError::Unknown(code) => write!(f, "unknown status {code:?}"),
            ParseError::BadDataSize(s) => write!(f, "DATA size {s:?} is not a hex number"),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_response(bytes: &[u8]) -> Result<Response, ParseError> {
    if bytes.len() < 4 {
        return Err(ParseError::Empty);
    }
    let body = String::from_utf8_lossy(&bytes[4..]).into_owned();
    match &bytes[..4] {
        b"OKAY" => Ok(Response::Ok(body)),
        b"FAIL" => Ok(Response::Fail(body)),
        b"INFO" => Ok(Response::Info(body)),
        b"TEXT" => Ok(Response::Text(body)),
        b"DATA" => u32::from_str_radix(body.trim(), 16)
            .map(Response::Data)
            .map_err(|_| ParseError::BadDataSize(body)),
        other => Err(ParseError::Unknown(
            String::from_utf8_lossy(other).into_owned(),
        )),
    }
}

pub fn getvar(key: &str) -> String {
    format!("{CMD_GETVAR}:{key}")
}

pub fn partition_var(key: &str, partition: &str) -> String {
    format!("{CMD_GETVAR}:{key}:{partition}")
}

pub fn download(size: u32) -> String {
    format!("{CMD_DOWNLOAD}:{size:08x}")
}

pub fn flash(partition: &str) -> String {
    format!("{CMD_FLASH}:{partition}")
}

pub fn erase(partition: &str) -> String {
    format!("{CMD_ERASE}:{partition}")
}

pub fn set_active(slot: &str) -> String {
    format!("{CMD_SET_ACTIVE}:{slot}")
}

pub fn reboot() -> String {
    CMD_REBOOT.to_owned()
}

pub fn continue_boot() -> String {
    CMD_CONTINUE.to_owned()
}

pub fn oem(args: &str) -> String {
    format!("{CMD_OEM} {args}")
}

pub fn fetch(partition: &str) -> String {
    format!("{CMD_FETCH}:{partition}")
}

pub fn snapshot_update(action: &str) -> String {
    format!("{CMD_SNAPSHOT_UPDATE}:{action}")
}

pub enum RebootTarget {
    Bootloader,
    Recovery,
    Fastboot,
}

impl RebootTarget {
    fn as_str(&self) -> &'static str {
        match self {
            RebootTarget::Bootloader => "bootloader",
            RebootTarget::Recovery => "recovery",
            RebootTarget::Fastboot => "fastboot",
        }
    }
}

pub fn reboot_to(target: RebootTarget) -> String {
    format!("{CMD_REBOOT}-{}", target.as_str())
}

pub fn flashing_unlock() -> String {
    "flashing:unlock".to_owned()
}

pub fn flashing_lock() -> String {
    "flashing:lock".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_statuses() {
        assert_eq!(
            parse_response(b"OKAY").unwrap(),
            Response::Ok(String::new())
        );
        assert_eq!(
            parse_response(b"FAILpartition not found").unwrap(),
            Response::Fail("partition not found".to_owned())
        );
        assert_eq!(
            parse_response(b"INFOwriting 'boot'").unwrap(),
            Response::Info("writing 'boot'".to_owned())
        );
        assert_eq!(
            parse_response(b"TEXTsome trace").unwrap(),
            Response::Text("some trace".to_owned())
        );
        assert_eq!(
            parse_response(b"DATA00001000").unwrap(),
            Response::Data(0x1000)
        );
    }

    #[test]
    fn rejects_unknown_status_and_garbage() {
        assert!(matches!(
            parse_response(b"OOPS"),
            Err(ParseError::Unknown(_))
        ));
        assert!(matches!(parse_response(b"OK"), Err(ParseError::Empty)));
        assert!(matches!(
            parse_response(b"DATAnothex"),
            Err(ParseError::BadDataSize(_))
        ));
    }

    #[test]
    fn builds_official_command_strings() {
        assert_eq!(getvar(VAR_UNLOCKED), "getvar:unlocked");
        assert_eq!(
            partition_var(VAR_PARTITION_SIZE, "boot"),
            "getvar:partition-size:boot"
        );
        assert_eq!(download(0x1000), "download:00001000");
        assert_eq!(flash("boot"), "flash:boot");
        assert_eq!(erase("userdata"), "erase:userdata");
        assert_eq!(set_active("a"), "set_active:a");
        assert_eq!(reboot(), "reboot");
        assert_eq!(reboot_to(RebootTarget::Bootloader), "reboot-bootloader");
        assert_eq!(reboot_to(RebootTarget::Recovery), "reboot-recovery");
        assert_eq!(reboot_to(RebootTarget::Fastboot), "reboot-fastboot");
        assert_eq!(oem("device-info"), "oem device-info");
        assert_eq!(fetch("boot"), "fetch:boot");
        assert_eq!(snapshot_update("cancel"), "snapshot-update:cancel");
        assert_eq!(flashing_unlock(), "flashing:unlock");
        assert_eq!(flashing_lock(), "flashing:lock");
        assert_eq!(continue_boot(), "continue");
    }

    #[test]
    fn constants_match_aosp_sizes() {
        assert_eq!(RESPONSE_SZ, 256);
        assert_eq!(COMMAND_SZ, 4096);
        assert_eq!(TRANSPORT_CHUNK_SIZE, 1024);
    }
}
