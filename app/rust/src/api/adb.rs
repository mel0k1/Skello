use std::process::Command;

use skello_adb::server::AdbServerClient;

use super::util::run_with_timeout;
use std::time::Duration;

pub fn adb_server_version() -> anyhow::Result<u32> {
    let mut client = AdbServerClient::connect_default()?;
    Ok(client.server_version()?)
}

pub fn adb_devices() -> anyhow::Result<String> {
    let mut client = AdbServerClient::connect_default()?;
    Ok(client.devices()?)
}

pub fn adb_reboot(serial: String, target: String) -> anyhow::Result<String> {
    let mut cmd = Command::new("adb");
    cmd.arg("-s").arg(&serial).arg("reboot");
    if !target.is_empty() {
        cmd.arg(&target);
    }
    run_with_timeout(cmd, "adb reboot", Duration::from_secs(30))
}
