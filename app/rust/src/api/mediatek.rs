use base64::Engine as _;

use crate::api::adb_usb::adb_usb_shell;
use crate::api::log;
use skello_chipsets::{LockState, SecCfgInfo, SecCfgError, parse, patch};

const SECCFG_DEVICE: &str = "/dev/block/by-name/seccfg";
const TMP_PATH: &str = "/data/local/tmp/skello_seccfg.img";

/// Parsed seccfg state of a MediaTek device.
#[derive(Clone)]
pub struct SeccfgReport {
    pub version: String,
    pub state: String,
    pub detail: String,
}

fn run_shell(serial: &str, keys_dir: &str, command: &str) -> anyhow::Result<String> {
    match adb_usb_shell(serial.to_owned(), keys_dir.to_owned(), command.to_owned()) {
        Ok(out) => Ok(out),
        Err(first) => {
            // Not running as root: retry through su.
            let su_cmd = format!("su -c '{command}'");
            match adb_usb_shell(serial.to_owned(), keys_dir.to_owned(), su_cmd) {
                Ok(out) => Ok(out),
                Err(_) => Err(anyhow::anyhow!(
                    "shell command failed: {first}; su fallback failed too"
                )),
            }
        }
    }
}

fn is_rooted(serial: &str, keys_dir: &str) -> anyhow::Result<bool> {
    // Through run_shell: adb without root but with a working su still
    // reaches the seccfg partition, a bare "id" would underreport.
    let out = run_shell(serial, keys_dir, "id")?;
    Ok(out.contains("uid=0"))
}

fn pull_seccfg(serial: &str, keys_dir: &str) -> anyhow::Result<Vec<u8>> {
    let out = run_shell(
        serial,
        keys_dir,
        &format!("dd if={SECCFG_DEVICE} | base64"),
    )?;
    let flat: String = out.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD
        .decode(&flat)
        .map_err(|e| anyhow::anyhow!("base64 decode of seccfg failed: {e}"))
}

fn push_seccfg(serial: &str, keys_dir: &str, data: &[u8]) -> anyhow::Result<()> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    let chunks: Vec<&str> = {
        let mut all = Vec::new();
        let mut rest = encoded.as_str();
        while rest.len() > 1800 {
            let (chunk, tail) = rest.split_at(1800);
            all.push(chunk);
            rest = tail;
        }
        all.push(rest);
        all
    };
    for (index, chunk) in chunks.iter().enumerate() {
        let redirect = if index == 0 { ">" } else { ">>" };
        run_shell(
            serial,
            keys_dir,
            &format!("echo '{chunk}' | base64 -d {redirect} {TMP_PATH}"),
        )?;
    }
    run_shell(
        serial,
        keys_dir,
        &format!("dd if={TMP_PATH} of={SECCFG_DEVICE}"),
    )?;
    run_shell(serial, keys_dir, &format!("rm {TMP_PATH}"))?;
    run_shell(serial, keys_dir, "sync")?;
    Ok(())
}

fn report_of(info: &SecCfgInfo) -> SeccfgReport {
    let version = match info.version {
        skello_chipsets::SecCfgVersion::V3 => "V3",
        skello_chipsets::SecCfgVersion::V4 => "V4",
    };
    let state = match info.state {
        LockState::Locked => "locked",
        LockState::Unlocked => "unlocked",
        LockState::Unknown(v) => {
            return SeccfgReport {
                version: version.to_owned(),
                state: format!("unknown ({v:#x})"),
                detail: info.detail.clone(),
            };
        }
    };
    SeccfgReport {
        version: version.to_owned(),
        state: state.to_owned(),
        detail: info.detail.clone(),
    }
}

/// Reads the seccfg partition over adb (rooted device required for the
/// block device) and reports the lock state.
pub fn mediatek_seccfg_status(serial: String, keys_dir: String) -> anyhow::Result<SeccfgReport> {
    if !is_rooted(&serial, &keys_dir)? {
        anyhow::bail!("root access required to read {SECCFG_DEVICE}");
    }
    let data = pull_seccfg(&serial, &keys_dir)?;
    log::core_log(
        "mediatek",
        &format!("seccfg pulled, {} bytes", data.len()),
    );
    let info = parse(&data).map_err(seccfg_err)?;
    Ok(report_of(&info))
}

/// Patches seccfg to the unlocked state and writes it back through adb.
/// Only V3 software-SEJ partitions are supported; hardware-keyed ones
/// need the Download Agent stage.
pub fn mediatek_seccfg_unlock(serial: String, keys_dir: String) -> anyhow::Result<String> {
    if !is_rooted(&serial, &keys_dir)? {
        anyhow::bail!("root access required to write {SECCFG_DEVICE}");
    }
    let data = pull_seccfg(&serial, &keys_dir)?;
    let patched = patch(&data, true).map_err(seccfg_err)?;
    push_seccfg(&serial, &keys_dir, &patched)?;
    log::core_log(
        "mediatek",
        &format!("seccfg patched and written back on {serial}"),
    );
    Ok("seccfg patched to unlocked; reboot and verify".to_owned())
}

fn seccfg_err(e: SecCfgError) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}
