//! MediaTek flows: BROM transport, DA loader, XFlash protocol, the
//! XML DA V6 session, the SEJ engine driver, seccfg format and the
//! GPT reader.

pub mod brom;
pub mod daldr;
pub mod gpt;
pub mod seccfg;
pub mod sej;
pub mod xflash;

pub use brom::{Brom, BromError};
pub use daldr::{DaLoader, DaLoaderError};
pub use gpt::{GptError, GptPartition};
pub use seccfg::{
    detect_v4_mode, patch_v4, LockState, SecCfgError, SecCfgInfo, SecCfgVersion, SejMode, V4Fields,
    V4Signer,
};
pub use xflash::{XFlash, XFlashError};
pub use xml::{XmlBootMode, XmlDa, XmlDaError};
