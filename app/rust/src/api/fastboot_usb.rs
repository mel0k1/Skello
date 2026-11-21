use std::fs;

use skello_fastboot::UsbFastboot;

use crate::api::log;
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
    log::core_log("fastboot-usb", &format!(">> flashing:unlock on {serial}"));
    let mut usb = UsbFastboot::connect(&serial)?;
    usb.set_read_timeout(std::time::Duration::from_secs(120));
    let (payload, infos) = usb.command(&skello_fastboot::flashing_unlock())?;
    let mut lines = infos;
    if !payload.is_empty() {
        lines.push(payload);
    }
    log::core_log("fastboot-usb", &format!("<< {}", lines.join(" | ")));
    Ok(lines.join("\n"))
}

pub fn fastboot_usb_erase(serial: String, partition: String) -> anyhow::Result<String> {
    log::core_log(
        "fastboot-usb",
        &format!(">> erase:{partition} on {serial}"),
    );
    let mut usb = UsbFastboot::connect(&serial)?;
    let (payload, infos) = usb.command(&skello_fastboot::erase(&partition))?;
    let mut lines = infos;
    if !payload.is_empty() {
        lines.push(payload);
    }
    log::core_log("fastboot-usb", &format!("<< {}", lines.join(" | ")));
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
    log::core_log(
        "flash-usb",
        &format!(">> flash {partition} on {serial}, image {total} bytes"),
    );
    let max = match usb.command("getvar:max-download-size") {
        Ok((payload, _)) => skello_fastboot::parse_download_size(payload.trim())
            .unwrap_or(u64::MAX),
        Err(_) => u64::MAX,
    };
    log::core_log("flash-usb", &format!("max-download-size {max}"));
    let parts: Vec<Vec<u8>> = if total > max && skello_fastboot::sparse::is_sparse(&data) {
        let parts =
            skello_fastboot::sparse::sparse_plan(&data, max.min(usize::MAX as u64) as usize)?;
        log::core_log(
            "flash-usb",
            &format!("sparse image split into {} parts", parts.len()),
        );
        parts
    } else if total > max {
        anyhow::bail!(
            "image is {total} bytes, max-download-size is {max} and the image is not sparse"
        );
    } else {
        vec![data]
    };
    let parts_total: u64 = parts.iter().map(|p| p.len() as u64).sum();
    let mut all_lines: Vec<String> = Vec::new();
    let mut sent_base = 0u64;
    let mut last_emitted = 0u64;
    for (index, part) in parts.iter().enumerate() {
        let part_note = if parts.len() > 1 {
            format!("part {}/{} ({} bytes)", index + 1, parts.len(), part.len())
        } else {
            String::new()
        };
        if !part_note.is_empty() {
            log::core_log("flash-usb", &format!(">> download {part_note}"));
            let _ = progress.add(FlashProgress {
                sent: sent_base,
                total: parts_total,
                phase: "download".to_owned(),
                message: part_note.clone(),
            });
        }
        usb.download(part, &mut |sent, _| {
            let sent = sent_base + sent;
            let step = parts_total / 100;
            if sent == parts_total || sent - last_emitted >= step.max(1) {
                last_emitted = sent;
                let _ = progress.add(FlashProgress {
                    sent,
                    total: parts_total,
                    phase: "download".to_owned(),
                    message: part_note.clone(),
                });
            }
        })?;
        sent_base += part.len() as u64;
        let (payload, infos) = usb.command(&skello_fastboot::flash(&partition))?;
        log::core_log(
            "flash-usb",
            &format!(
                "<< {}{}",
                infos.join(" | "),
                if payload.is_empty() {
                    String::new()
                } else {
                    format!(" | {payload}")
                }
            ),
        );
        all_lines.extend(infos);
        if !payload.is_empty() {
            all_lines.push(payload);
        }
    }
    let message = if all_lines.is_empty() {
        "OKAY".to_owned()
    } else {
        format!("OKAY {}", all_lines.join("\n"))
    };
    let _ = progress.add(FlashProgress {
        sent: parts_total,
        total: parts_total,
        phase: "done".to_owned(),
        message: message.clone(),
    });
    Ok(message)
}
