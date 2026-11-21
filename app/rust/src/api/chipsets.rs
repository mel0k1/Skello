use crate::api::log;
use skello_chipsets::mediatek::brom::{Brom, PID_BROM, PID_DA, VID_MEDIATEK};
use skello_chipsets::mediatek::xflash::XFlash;
use skello_chipsets::mediatek::xml::{XmlBootMode, XmlDa};
use skello_chipsets::qualcomm::{Firehose, Sahara, PID_EDL, VID_QUALCOMM};
use skello_chipsets::transport::{open_serial_link, open_usb_serial, serial_port_names, usb_serial_devices};
use skello_chipsets::unisoc::{Bsl, PID_BROM as UNISOC_PID_BROM, VID_UNISOC};

/// Bootloader ports visible right now, grouped by chipset family.
#[derive(Clone)]
pub struct ChipsetPorts {
    pub mediatek: Vec<String>,
    pub unisoc: Vec<String>,
    pub qualcomm: Vec<String>,
}

/// Lists the MediaTek BROM/Preloader, Unisoc BROM and Qualcomm EDL
/// USB ports without opening them.
pub fn chipset_ports() -> anyhow::Result<ChipsetPorts> {
    let mediatek = usb_serial_devices(VID_MEDIATEK, &[PID_BROM, skello_chipsets::mediatek::brom::PID_PRELOADER, PID_DA])
        .map_err(usb_err)?;
    let unisoc = usb_serial_devices(VID_UNISOC, &[UNISOC_PID_BROM]).map_err(usb_err)?;
    let qualcomm = usb_serial_devices(VID_QUALCOMM, &[PID_EDL]).map_err(usb_err)?;
    Ok(ChipsetPorts {
        mediatek,
        unisoc,
        qualcomm,
    })
}

/// MediaTek unlock without root: BROM handshake, stock DA from the
/// loader file, seccfg read through the DA, V3 software-SEJ patch or
/// the V4 patch signed by the on-chip SEJ engine through the DA
/// registers.
pub fn mediatek_da_unlock(loader_path: String) -> anyhow::Result<String> {
    let mut brom = Brom::new(open_usb_serial(VID_MEDIATEK, PID_BROM)?);
    let result = XFlash::unlock_seccfg_with(&mut brom, &loader_path, &mut || {
        open_usb_serial(VID_MEDIATEK, PID_DA)
    })?;
    log::core_log("mediatek-da", &format!("unlock: {result}"));
    Ok(result)
}

/// Boots a V6 XML Download Agent and moves the device into fastboot
/// or meta: stage 1 over BROM, stage 2 through BOOT-TO, then the
/// boot mode and a disconnecting reboot. The loader file must carry
/// a V6 entry for the chip.
pub fn mediatek_v6_boot(loader_path: String, mode: String) -> anyhow::Result<String> {
    let boot_mode = match mode.as_str() {
        "fastboot" => XmlBootMode::Fastboot,
        "meta" => XmlBootMode::Meta,
        other => anyhow::bail!("unknown boot mode: {other}"),
    };
    let result = XmlDa::boot_v6(&loader_path, boot_mode)?;
    log::core_log("mediatek-v6", &format!("boot: {result}"));
    Ok(result)
}

/// One row of the Unisoc partition table: the name and the raw size
/// value the FDL2 reports (in its own sector units).
#[derive(Clone)]
pub struct UnisocPartition {
    pub name: String,
    pub size_sectors: u64,
}

/// Reads the partition table over the Unisoc FDL: boots FDL1/FDL2
/// from the model firmware, turns the transcode off and parses the
/// READ_PARTITION reply.
pub fn unisoc_partition_list(
    fdl1_path: String,
    fdl2_path: String,
    fdl1_addr: u32,
    fdl2_addr: u32,
) -> anyhow::Result<Vec<UnisocPartition>> {
    let fdl1 = std::fs::read(&fdl1_path)?;
    let fdl2 = std::fs::read(&fdl2_path)?;
    let mut bsl = Bsl::new(open_usb_serial(VID_UNISOC, UNISOC_PID_BROM)?, true);
    let table = bsl.partition_table(&fdl1, fdl1_addr, &fdl2, fdl2_addr)?;
    log::core_log("unisoc", &format!("table: {} partitions", table.len()));
    Ok(table
        .into_iter()
        .map(|p| UnisocPartition {
            name: p.name,
            size_sectors: p.size_sectors,
        })
        .collect())
}

/// Serial ports present on the system: the EDL-over-UART candidates.
pub fn qualcomm_serial_ports() -> anyhow::Result<Vec<String>> {
    Ok(serial_port_names().map_err(usb_err)?)
}

/// Qualcomm EDL flash over a serial port: Sahara with the silent
/// HELLO workaround, Firehose rawprogram run - same as the USB path
/// but on the UART link (115200 8N1, qdlrs setup).
pub fn qualcomm_edl_flash_serial(
    programmer_path: String,
    entries: Vec<ProgramEntry>,
    port_path: String,
) -> anyhow::Result<String> {
    let blob = std::fs::read(&programmer_path)?;
    if blob.starts_with(b"<?xml") {
        anyhow::bail!("the programmer file is an ELF or cpio archive, plain binary expected");
    }
    let link = open_serial_link(&port_path, 115_200).map_err(usb_err)?;
    Sahara::boot_programmer_with(link, &[blob])?;
    let link = open_serial_link(&port_path, 115_200).map_err(usb_err)?;
    let mut fh = Firehose::new(link);
    fh.configure("ufs")?;
    for entry in &entries {
        let mut data = std::fs::read(&entry.file_path)?;
        if skello_fastboot::sparse::is_sparse(&data) {
            data = skello_fastboot::sparse::sparse_expand(&data)
                .map_err(|e| anyhow::anyhow!("sparse expansion failed: {e}"))?;
        }
        log::core_log(
            "qualcomm",
            &format!(
                "serial program {} at sector {}, {} bytes",
                entry.label, entry.start_sector,
                data.len()
            ),
        );
        fh.program(entry.phys_part, &entry.start_sector, &data)?;
    }
    fh.reset()?;
    Ok(format!("{} entries flashed over serial", entries.len()))
}

/// Unisoc partition flash without root: wakes the BROM, boots FDL1
/// and FDL2 from the model firmware, writes the partition. Sparse
/// images are expanded to raw first (FDL2 takes raw data only).
pub fn unisoc_flash_partition(
    fdl1_path: String,
    fdl2_path: String,
    fdl1_addr: u32,
    fdl2_addr: u32,
    partition: String,
    image_path: String,
) -> anyhow::Result<String> {
    let fdl1 = std::fs::read(&fdl1_path)?;
    let fdl2 = std::fs::read(&fdl2_path)?;
    let mut image = std::fs::read(&image_path)?;
    if skello_fastboot::sparse::is_sparse(&image) {
        image = skello_fastboot::sparse::sparse_expand(&image)
            .map_err(|e| anyhow::anyhow!("sparse expansion failed: {e}"))?;
    }
    let mut bsl = Bsl::new(open_usb_serial(VID_UNISOC, UNISOC_PID_BROM)?, true);
    let ver = bsl.boot(&fdl1, fdl1_addr, &fdl2, fdl2_addr)?;
    log::core_log(
        "unisoc",
        &format!("boot: {ver}, writing {partition}, {} bytes", image.len()),
    );
    bsl.disable_transcode()?;
    bsl.write_partition(&partition, &image)?;
    bsl.power_off()?;
    Ok(format!("{partition} written, {ver}"))
}

/// Qualcomm EDL flash: serves the Firehose programmer over Sahara,
/// switches to command mode and runs a rawprogram XML where every
/// program entry flashes one file at its start sector. Sparse images
/// are expanded to raw first.
pub struct ProgramEntry {
    pub label: String,
    pub start_sector: String,
    pub num_sectors: u64,
    pub phys_part: u8,
    pub file_path: String,
}

/// Parses a rawprogram XML (firehose flasher layout) into entries.
pub fn parse_rawprogram(xml_path: String) -> anyhow::Result<Vec<ProgramEntry>> {
    let xml = std::fs::read_to_string(&xml_path)?;
    let dir = std::path::Path::new(&xml_path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    let mut out = Vec::new();
    for cap in program_entries(&xml) {
        let file = dir.join(&cap.file);
        out.push(ProgramEntry {
            label: cap.label,
            start_sector: cap.start_sector,
            num_sectors: cap.num_sectors,
            phys_part: cap.phys_part,
            file_path: file.to_string_lossy().to_string(),
        });
    }
    Ok(out)
}

struct RawEntry {
    label: String,
    start_sector: String,
    num_sectors: u64,
    phys_part: u8,
    file: String,
}

fn program_entries(xml: &str) -> Vec<RawEntry> {
    let mut out = Vec::new();
    for chunk in xml.split("<program") {
        let Some(rest) = chunk.split_once("/>") else {
            continue;
        };
        let tag = rest.0;
        let get = |key: &str| -> Option<String> {
            let pattern = format!("{key}=\"");
            let start = tag.find(&pattern)? + pattern.len();
            let end = tag[start..].find('"')? + start;
            Some(tag[start..end].to_string())
        };
        let Some(file) = get("filename") else {
            continue;
        };
        out.push(RawEntry {
            label: get("label").unwrap_or_else(|| file.clone()),
            start_sector: get("start_sector").unwrap_or_default(),
            num_sectors: get("num_partition_sectors")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            phys_part: get("physical_partition_number")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            file,
        });
    }
    out
}

/// Flash every entry of an already parsed rawprogram list.
pub fn qualcomm_edl_flash(
    programmer_path: String,
    entries: Vec<ProgramEntry>,
) -> anyhow::Result<String> {
    let blob = std::fs::read(&programmer_path)?;
    let mut images: Vec<Vec<u8>> = Vec::new();
    if blob.starts_with(b"<?xml") {
        anyhow::bail!("the programmer file is an ELF or cpio archive, plain binary expected");
    }
    images.push(blob);
    Sahara::boot_programmer(&images)?;
    let mut fh = Firehose::new(open_usb_serial(VID_QUALCOMM, PID_EDL)?);
    fh.configure("ufs")?;
    for entry in &entries {
        let mut data = std::fs::read(&entry.file_path)?;
        if skello_fastboot::sparse::is_sparse(&data) {
            data = skello_fastboot::sparse::sparse_expand(&data)
                .map_err(|e| anyhow::anyhow!("sparse expansion failed: {e}"))?;
        }
        log::core_log(
            "qualcomm",
            &format!(
                "program {} at sector {}, {} bytes",
                entry.label, entry.start_sector,
                data.len()
            ),
        );
        fh.program(entry.phys_part, &entry.start_sector, &data)?;
    }
    fh.reset()?;
    Ok(format!("{} entries flashed", entries.len()))
}

fn usb_err(e: skello_chipsets::transport::LinkError) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}
