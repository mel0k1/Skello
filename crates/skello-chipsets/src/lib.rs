//! Chipset-specific transports and formats for Skello: the MediaTek
//! BROM/DA V5 flow, the Unisoc FDL transport with transcode, the
//! Qualcomm Sahara/Firehose pair and the seccfg format.

pub mod mediatek;
pub mod qualcomm;
pub mod transport;
pub mod unisoc;
pub mod unisoc_pac;

pub use mediatek::seccfg::{
    detect_v4_mode, parse, patch, patch_v4, sej_sw, LockState, SecCfgError, SecCfgInfo,
    SecCfgVersion, SejMode, V4Fields, V4Signer,
};
pub use mediatek::xml::XmlBootMode;
pub use unisoc::{parse_partition_list, UniPartition};
pub use unisoc_pac::{parse_pac, PacError, PacFirmware};
