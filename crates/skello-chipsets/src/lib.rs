//! Chipset-specific transports and formats for Skello: the MediaTek
//! BROM/DA V5 flow, the Unisoc FDL transport with transcode, the
//! Qualcomm Sahara/Firehose pair and the seccfg format.

pub mod mediatek;
pub mod qualcomm;
pub mod transport;
pub mod unisoc;

pub use mediatek::seccfg::{
    parse, patch, sej_sw, LockState, SecCfgError, SecCfgInfo, SecCfgVersion,
};
