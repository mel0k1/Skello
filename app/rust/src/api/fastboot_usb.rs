use std::fs;

use skello_fastboot::UsbFastboot;

use crate::frb_generated::StreamSink;

/// Progress event for the USB flash stream; `phase` is "download" while the
/// image streams to the device and "done" with the final message at the end.
#[derive(Clone)]
pub struct FlashProgress {
    pub sent: u64,
    pub total: u64,
    pub phase: String,
    pub message: String,
}

pub fn fastboot_usb_devices() -> anyhow::Result<Vec<String>> {
    Ok(skello_fastboot::usb_fastboot_devices()?)
}

pub fn fastboot_usb_getvar(serial: String, name: String) -> anyhow::Result<String> {
    let mut usb = UsbFastboot::connect(&serial)?;
    let cmd = format!("getvar:{name}");
    let (payload, infos) = usb.command(&cmd)?;
    let mut lines = infos;
    if !payload.is_empty() {
        lines.push(payload);
    }
    Ok(lines.join("\n"))
}

pub fn fastboot_usb_reboot(serial: String) -> anyhow::Result<()> {
    let mut usb = UsbFastboot::connect(&serial)?;
    usb.command("reboot")?;
    Ok(())
}

/// `flashing:unlock` over USB; the device may wait for an on-phone
/// confirmation, so the read timeout is extended to 120 seconds.
pub fn fastboot_usb_unlock(serial: String) -> anyhow::Result<String> {
    let mut usb = UsbFastboot::connect(&serial)?;
    usb.set_read_timeout(std::time::Duration::from_secs(120));
    let (payload, infos) = usb.command(&skello_fastboot::flashing_unlock())?;
    let mut lines = infos;
    if !payload.is_empty() {
        lines.push(payload);
    }
    Ok(lines.join("\n"))
}

pub fn fastboot_usb_erase(serial: String, partition: String) -> anyhow::Result<String> {
    let mut usb = UsbFastboot::connect(&serial)?;
    let (payload, infos) = usb.command(&skello_fastboot::erase(&partition))?;
    let mut lines = infos;
    if !payload.is_empty() {
        lines.push(payload);
    }
    Ok(lines.join("\n"))
}

pub fn fastboot_flash_usb(
    serial: String,
    partition: String,
    image_path: String,
    progress: StreamSink<FlashProgress>,
) -> anyhow::Result<String> {
    let data = fs::read(&image_path)?;
    let mut usb = UsbFastboot::connect(&serial)?;
    let total = data.len() as u64;
    let mut last_emitted = 0u64;
    usb.download(&data, &mut |sent, total| {
        let step = total / 100;
        if sent == total || sent - last_emitted >= step.max(1) {
            last_emitted = sent;
            let _ = progress.add(FlashProgress {
                sent,
                total,
                phase: "download".to_owned(),
                message: String::new(),
            });
        }
    })?;
    let (payload, infos) = usb.command(&skello_fastboot::flash(&partition))?;
    let mut lines = infos;
    if !payload.is_empty() {
        lines.push(payload);
    }
    let message = if lines.is_empty() {
        "OKAY".to_owned()
    } else {
        format!("OKAY {}", lines.join("\n"))
    };
    let _ = progress.add(FlashProgress {
        sent: total,
        total,
        phase: "done".to_owned(),
        message: message.clone(),
    });
    Ok(message)
}
