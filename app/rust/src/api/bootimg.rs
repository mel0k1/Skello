pub struct BootImageInfo {
    pub header_version: u32,
    pub kernel_size: u32,
    pub ramdisk_size: u32,
    pub page_size: Option<u32>,
    pub os_version: Option<String>,
    pub os_patch_level: Option<String>,
    pub file_size: u64,
    pub has_avb_footer: bool,
}

impl From<skello_bootimg::BootImageInfo> for BootImageInfo {
    fn from(core: skello_bootimg::BootImageInfo) -> Self {
        BootImageInfo {
            header_version: core.header_version,
            kernel_size: core.kernel_size,
            ramdisk_size: core.ramdisk_size,
            page_size: core.page_size,
            os_version: core.os_version,
            os_patch_level: core.os_patch_level,
            file_size: core.file_size,
            has_avb_footer: core.has_avb_footer,
        }
    }
}

pub fn boot_image_info(path: String) -> anyhow::Result<BootImageInfo> {
    Ok(skello_bootimg::read(&path)?.into())
}
