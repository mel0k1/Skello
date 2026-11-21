//! MediaTek flows: BROM transport, DA loader, XFlash protocol, the
//! XML DA V6 session, the SEJ engine driver, seccfg format and the
//! GPT reader.

pub mod brom;
pub mod carbonara;
pub mod daldr;
pub mod exploit;
pub mod gpt;
pub mod scatter;
pub mod seccfg;
pub mod sej;
pub mod xflash;
pub mod xml;

pub use brom::{Brom, BromError};
pub use carbonara::{
    calc_da_hash, check_carbonara_patched, compute_hash_pos, fix_hash, patch_da2, patch_da2_vendor,
    HashMode,
};
pub use daldr::{DaLoader, DaLoaderError};
pub use exploit::{
    find_exploit_chip, BromExploit, CtrlBus, ExploitChip, ExploitError, NusbCtrlBus,
};
pub use gpt::{GptError, GptPartition};
pub use seccfg::{
    detect_v4_mode, patch_v4, LockState, SecCfgError, SecCfgInfo, SecCfgVersion, SejMode, V4Fields,
    V4Signer,
};
pub use xflash::{XFlash, XFlashError};
pub use xml::{XmlBootMode, XmlDa, XmlDaError};
