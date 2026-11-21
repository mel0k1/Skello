use std::process::Command;
use std::time::Duration;

use super::util::run_with_timeout;

pub fn fastboot_devices() -> anyhow::Result<String> {
    let output = Command::new("fastboot")
        .arg("devices")
        .output()
        .map_err(|e| anyhow::anyhow!("fastboot binary not found: {e}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "fastboot devices failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn fastboot_getvar(serial: String, name: String) -> anyhow::Result<String> {
    let mut cmd = Command::new("fastboot");
    cmd.arg("-s").arg(&serial).arg("getvar").arg(&name);
    let text = run_with_timeout(cmd, "fastboot getvar", Duration::from_secs(30))?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(&format!("{name}:")) {
            return Ok(rest.trim().to_string());
        }
    }
    anyhow::bail!("variable {name} not found in fastboot output")
}

pub fn fastboot_reboot(serial: String) -> anyhow::Result<String> {
    let mut cmd = Command::new("fastboot");
    cmd.arg("-s").arg(&serial).arg("reboot");
    run_with_timeout(cmd, "fastboot reboot", Duration::from_secs(60))
}

pub fn fastboot_flashing_unlock(serial: String) -> anyhow::Result<String> {
    let mut cmd = Command::new("fastboot");
    cmd.arg("-s").arg(&serial).arg("flashing").arg("unlock");
    run_with_timeout(cmd, "fastboot flashing unlock", Duration::from_secs(120))
}

pub fn fastboot_flash(serial: String, partition: String, image: String) -> anyhow::Result<String> {
    let mut cmd = Command::new("fastboot");
    cmd.arg("-s")
        .arg(&serial)
        .arg("flash")
        .arg(&partition)
        .arg(&image);
    run_with_timeout(cmd, "fastboot flash", Duration::from_secs(600))
}
