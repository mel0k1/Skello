use std::process::Command;

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
