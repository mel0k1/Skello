//! Chipset-specific formats and flows for Skello.
//!
//! Currently: MediaTek seccfg (V3 software-SEJ patching, V4 state read).
//! Unisoc FDL transport and Qualcomm Sahara/Firehose follow the plan in
//! CHIPSETS.md and live here once implemented.

pub mod mediatek;

pub use mediatek::{parse, patch, sej_sw, LockState, SecCfgError, SecCfgInfo, SecCfgVersion};
