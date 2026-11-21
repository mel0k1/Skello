use std::path::Path;

use skello_adb::{AdbKeys, UsbAdb, adb_usb_device_state, adb_usb_serials};

/// State of one adb device found over USB: "device" after a successful
/// handshake, "unauthorized" while the key is not accepted, "offline"
/// when the connection failed.
#[derive(Clone)]
pub struct AdbUsbStatus {
    pub serial: String,
    pub state: String,
}

pub fn adb_usb_status(keys_dir: String) -> anyhow::Result<Vec<AdbUsbStatus>> {
    let serials = adb_usb_serials()?;
    let keys = AdbKeys::load_or_generate(Path::new(&keys_dir))?;
    let states = std::thread::scope(|scope| {
        let keys_ref = &keys;
        let handles: Vec<_> = serials
            .iter()
            .map(|serial| {
                let serial = serial.clone();
                scope.spawn(move || {
                    let state = adb_usb_device_state(&serial, keys_ref);
                    (serial, state)
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .map(|(serial, state)| AdbUsbStatus { serial, state })
            .collect()
    });
    Ok(states)
}

pub fn adb_usb_shell(
    serial: String,
    keys_dir: String,
    command: String,
) -> anyhow::Result<String> {
    let keys = AdbKeys::load_or_generate(Path::new(&keys_dir))?;
    let mut adb = UsbAdb::connect(&serial, &keys)?;
    let out = adb.shell(&command)?;
    let _ = adb.close();
    Ok(out)
}

pub fn adb_usb_reboot(
    serial: String,
    keys_dir: String,
    target: String,
) -> anyhow::Result<()> {
    let keys = AdbKeys::load_or_generate(Path::new(&keys_dir))?;
    let mut adb = UsbAdb::connect(&serial, &keys)?;
    adb.reboot(if target.is_empty() {
        None
    } else {
        Some(&target)
    })?;
    Ok(())
}
