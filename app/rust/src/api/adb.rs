use skello_adb::server::AdbServerClient;

pub fn adb_server_version() -> anyhow::Result<u32> {
    let mut client = AdbServerClient::connect_default()?;
    Ok(client.server_version()?)
}

pub fn adb_devices() -> anyhow::Result<String> {
    let mut client = AdbServerClient::connect_default()?;
    Ok(client.devices()?)
}
