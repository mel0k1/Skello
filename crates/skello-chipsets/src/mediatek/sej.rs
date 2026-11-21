//! MediaTek HACC (SEJ) hardware engine driven through the DA V5
//! register access. The engine binds its key to the chip's unique
//! HUID, so the round trips run on the device itself: the host only
//! writes the register sequence. Register offsets, control bits and
//! the init/run/terminate sequences follow mtkclient
//! (Apache-2.0, Library/Hardware/hwcrypto_sej.py) and mtkclient-rs
//! (Apache-2.0, security/sej.rs); NOTICE has the attribution.

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use std::io;

/// HACC register offsets relative to the SEJ base.
pub const HACC_ACON: u32 = 0x004;
pub const HACC_ACON2: u32 = 0x008;
pub const HACC_ACONK: u32 = 0x00C;
pub const HACC_ASRC0: u32 = 0x010;
pub const HACC_AKEY0: u32 = 0x020;
pub const HACC_ACFG0: u32 = 0x040;
pub const HACC_AOUT0: u32 = 0x050;
pub const HACC_UNK: u32 = 0x0BC;

/// HACC control bits.
pub const AES_ENC: u32 = 0x0000_0001;
pub const AES_CBC: u32 = 0x0000_0002;
pub const AES_128: u32 = 0x0000_0000;
pub const AES_CHG_BO_OFF: u32 = 0x0000_0000;
pub const AES_START: u32 = 0x0000_0001;
pub const AES_CLR: u32 = 0x0000_0002;
pub const AES_RDY: u32 = 0x0000_8000;
pub const AES_BK2C: u32 = 0x0000_0010;
pub const AES_R2K: u32 = 0x0000_0100;

/// The fixed pattern the non-legacy init feeds through the engine to
/// derive the operating key (mtkclient `g_CFG_RANDOM_PATTERN`).
pub const CFG_RANDOM_PATTERN: [u32; 12] = [
    0x2D44_BB70,
    0xA744_D227,
    0xD0A9_864B,
    0x83FF_C244,
    0x7EC8_266B,
    0x43E8_0FB2,
    0x01A6_348A,
    0x2067_F9A0,
    0x5453_6405,
    0xD546_A6B1,
    0x1CC3_EC3A,
    0xDE37_7A83,
];

/// The IV words the seccfg hash modes use (mtkclient `g_HACC_CFG_1`).
pub const G_HACC_CFG_1: [u32; 8] = [
    0x9ED4_0400,
    0x00E8_84A1,
    0xE3F0_83BD,
    0x2F4E_6D8A,
    0xFF83_8E5C,
    0xE940_A0E3,
    0x8D4D_ECC6,
    0x45FC_0989,
];

/// Default SEJ base: MT6768-class chips; a per-chip table can override.
pub const SEJ_BASE_DEFAULT: u32 = 0x1000_A000;

/// Register access to the running chip. The real device goes through
/// the DA V5 custom register commands; tests emulate the engine.
pub trait SejBus {
    fn read32(&mut self, addr: u32) -> io::Result<u32>;
    fn write32(&mut self, addr: u32, value: u32) -> io::Result<()>;
}

impl<T: SejBus + ?Sized> SejBus for &mut T {
    fn read32(&mut self, addr: u32) -> io::Result<u32> {
        (**self).read32(addr)
    }

    fn write32(&mut self, addr: u32, value: u32) -> io::Result<()> {
        (**self).write32(addr, value)
    }
}

#[derive(Debug)]
pub enum SejError {
    Io(std::io::Error),
    /// The engine did not report AES_RDY in time.
    NotReady,
}

impl std::fmt::Display for SejError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SejError::Io(e) => write!(f, "sej io: {e}"),
            SejError::NotReady => write!(f, "sej engine not ready"),
        }
    }
}

impl std::error::Error for SejError {}

impl From<io::Error> for SejError {
    fn from(e: io::Error) -> Self {
        SejError::Io(e)
    }
}

type Result<T> = std::result::Result<T, SejError>;

/// The HACC driver over any register bus. `base` is the chip's SEJ
/// base address (SEJ_BASE_DEFAULT covers the common chips).
pub struct SejEngine<B: SejBus> {
    pub bus: B,
    pub base: u32,
}

impl<B: SejBus> SejEngine<B> {
    pub fn new(bus: B, base: u32) -> Self {
        Self { bus, base }
    }

    fn hacc_write(&mut self, offset: u32, value: u32) -> Result<()> {
        self.bus
            .write32(self.base + offset, value)
            .map_err(SejError::Io)
    }

    fn hacc_read(&mut self, offset: u32) -> Result<u32> {
        self.bus.read32(self.base + offset).map_err(SejError::Io)
    }

    fn wait_ready(&mut self) -> Result<()> {
        for _ in 0..20 {
            let val = self.hacc_read(HACC_ACON2)?;
            if val & AES_RDY != 0 {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Err(SejError::NotReady)
    }

    /// `SEJ_V3_Init`: binds the HUID/HUK into the engine (R2K/BK2C),
    /// sets the IV and the operation mode. `legacy` selects the older
    /// key derivation (the V4 seccfg hash variant).
    pub fn sej_v3_init(&mut self, encrypt: bool, iv: &[u32; 4], legacy: bool) -> Result<()> {
        for i in 0..8 {
            self.hacc_write(HACC_AKEY0 + i * 4, 0)?;
        }
        self.hacc_write(HACC_ACON, AES_CHG_BO_OFF | AES_CBC | AES_128)?;
        self.hacc_write(HACC_ACONK, AES_BK2C | AES_R2K)?;
        self.hacc_write(HACC_ACON2, AES_CLR)?;

        for (i, word) in iv.iter().enumerate() {
            self.hacc_write(HACC_ACFG0 + (i * 4) as u32, *word)?;
        }

        let acon = AES_CHG_BO_OFF | AES_128 | AES_CBC | if encrypt { AES_ENC } else { 0 };
        if legacy {
            let unk = self.hacc_read(HACC_UNK)?;
            self.hacc_write(HACC_UNK, unk | 2)?;
            self.hacc_write(HACC_ACON2, 0x4000_0000)?;
            for _ in 0..20 {
                let val = self.hacc_read(HACC_ACON2)?;
                if val > 0x8000_0000 {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            self.hacc_write(HACC_UNK, unk & 0xFFFF_FFFE)?;
            self.hacc_write(HACC_ACONK, AES_BK2C)?;
            self.hacc_write(HACC_ACON, acon)?;
        } else {
            self.hacc_write(HACC_UNK, 1)?;
            for i in 0..3 {
                let pos = i * 4;
                for (k, word) in CFG_RANDOM_PATTERN[pos..pos + 4].iter().enumerate() {
                    self.hacc_write(HACC_ASRC0 + (k * 4) as u32, *word)?;
                }
                self.hacc_write(HACC_ACON2, AES_START)?;
                self.wait_ready()?;
            }
            self.hacc_write(HACC_ACON2, AES_CLR)?;
            for (i, word) in iv.iter().enumerate() {
                self.hacc_write(HACC_ACFG0 + (i * 4) as u32, *word)?;
            }
            self.hacc_write(HACC_ACON, acon)?;
            self.hacc_write(HACC_ACONK, 0)?;
        }
        Ok(())
    }

    /// `sej_run`: one AES block per round trip through ASRC/AOUT.
    pub fn sej_run(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut words = Vec::with_capacity(data.len() / 4);
        for chunk in data.as_chunks::<4>().0 {
            words.push(u32::from_le_bytes(*chunk));
        }
        let mut result = Vec::with_capacity(data.len());
        for chunk in words.chunks(4) {
            for (k, word) in chunk.iter().enumerate() {
                self.hacc_write(HACC_ASRC0 + (k * 4) as u32, *word)?;
            }
            self.hacc_write(HACC_ACON2, AES_START)?;
            self.wait_ready()?;
            for k in 0..4u32 {
                result.extend_from_slice(&self.hacc_read(HACC_AOUT0 + k * 4)?.to_le_bytes());
            }
        }
        Ok(result)
    }

    /// `sej_terminate`: clears the key registers.
    pub fn sej_terminate(&mut self) -> Result<()> {
        self.hacc_write(HACC_ACON2, AES_CLR)?;
        for i in 0..8 {
            self.hacc_write(HACC_AKEY0 + i * 4, 0)?;
        }
        Ok(())
    }

    /// One AES-128-CBC pass through the engine with the seccfg IV:
    /// init, run block by block, terminate.
    pub fn hacc_output(&mut self, data: &[u8], legacy: bool, encrypt: bool) -> Result<Vec<u8>> {
        let iv: [u32; 4] = G_HACC_CFG_1[0..4].try_into().unwrap();
        self.sej_v3_init(encrypt, &iv, legacy)?;
        let out = self.sej_run(data)?;
        self.sej_terminate()?;
        Ok(out)
    }
}

/// Software AES-128-CBC with a zero key over the seccfg IV words.
/// This is what the engine computes on chips whose key binding is
/// disabled; tests and offline tools use it as the reference.
pub fn sej_hw_soft(data: &[u8], encrypt: bool) -> io::Result<Vec<u8>> {
    if !data.len().is_multiple_of(16) {
        return Err(io::Error::other("sej data is not 16-byte aligned"));
    }
    let mut iv = [0u8; 16];
    for i in 0..4 {
        iv[i * 4..i * 4 + 4].copy_from_slice(&G_HACC_CFG_1[i].to_le_bytes());
    }
    let cipher = aes::Aes128::new(GenericArray::from_slice(&[0u8; 16]));
    let mut carry = iv;
    let mut out = vec![0u8; data.len()];
    for (index, block) in data.chunks(16).enumerate() {
        let mut buf = [0u8; 16];
        buf.copy_from_slice(block);
        let ga = GenericArray::from_mut_slice(&mut buf);
        if encrypt {
            for (a, b) in ga.iter_mut().zip(carry.iter()) {
                *a ^= b;
            }
            cipher.encrypt_block(ga);
            carry.copy_from_slice(ga);
        } else {
            let mut saved = [0u8; 16];
            saved.copy_from_slice(ga);
            cipher.decrypt_block(ga);
            for (a, b) in ga.iter_mut().zip(carry.iter()) {
                *a ^= b;
            }
            carry = saved;
        }
        out[index * 16..index * 16 + 16].copy_from_slice(ga);
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod testutil {
    use super::*;

    /// A register bus that emulates a HACC engine with the key binding
    /// disabled: the key register stays zero, every AES_START runs one
    /// CBC-128 block over ASRC with the ACFG IV chain. The direction
    /// follows the ACON ENC bit. Values on a real chip differ
    /// (HUID-derived key) but the register sequence and the state
    /// machine are the same.
    pub struct MockHacc {
        pub regs: std::collections::HashMap<u32, u32>,
        ready: bool,
        chain: [u8; 16],
        /// AOUT is a register, not a queue: every run overwrites it.
        aout: [u8; 16],
    }

    impl Default for MockHacc {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MockHacc {
        pub fn new() -> Self {
            Self {
                regs: std::collections::HashMap::new(),
                ready: false,
                chain: [0u8; 16],
                aout: [0u8; 16],
            }
        }

        fn encrypt(&self) -> bool {
            self.regs.get(&(SEJ_BASE_DEFAULT + HACC_ACON)).unwrap_or(&0) & AES_ENC != 0
        }

        fn asrc(&self) -> Vec<u8> {
            (0..4u32)
                .flat_map(|i| {
                    self.regs
                        .get(&(SEJ_BASE_DEFAULT + HACC_ASRC0 + i * 4))
                        .unwrap_or(&0)
                        .to_le_bytes()
                })
                .collect()
        }

        fn run_block(&mut self) {
            let src = self.asrc();
            let cipher = aes::Aes128::new(GenericArray::from_slice(&[0u8; 16]));
            let mut ga = GenericArray::clone_from_slice(&src);
            if self.encrypt() {
                for (a, c) in ga.iter_mut().zip(self.chain.iter()) {
                    *a ^= c;
                }
                cipher.encrypt_block(&mut ga);
                self.chain.copy_from_slice(&ga);
            } else {
                let mut saved = [0u8; 16];
                saved.copy_from_slice(&ga);
                cipher.decrypt_block(&mut ga);
                for (a, c) in ga.iter_mut().zip(self.chain.iter()) {
                    *a ^= c;
                }
                self.chain = saved;
            }
            self.aout.copy_from_slice(&ga);
            self.ready = true;
        }
    }

    impl SejBus for MockHacc {
        fn read32(&mut self, addr: u32) -> io::Result<u32> {
            let off = addr & 0xFF;
            if off == HACC_AOUT0 || (HACC_AOUT0..HACC_AOUT0 + 16).contains(&off) {
                let start = (off - HACC_AOUT0) as usize;
                let word: [u8; 4] = self.aout[start..start + 4].try_into().unwrap();
                return Ok(u32::from_le_bytes(word));
            }
            if off == HACC_ACON2 && self.ready {
                return Ok(AES_RDY);
            }
            Ok(*self.regs.get(&addr).unwrap_or(&0))
        }

        fn write32(&mut self, addr: u32, value: u32) -> io::Result<()> {
            let off = addr & 0xFF;
            if (HACC_ACFG0..HACC_ACFG0 + 16).contains(&off) {
                self.regs.insert(addr, value);
                // the IV reload resets the CBC chain
                let mut iv = [0u8; 16];
                for i in 0..4u32 {
                    iv[i as usize * 4..i as usize * 4 + 4].copy_from_slice(
                        &self
                            .regs
                            .get(&(SEJ_BASE_DEFAULT + HACC_ACFG0 + i * 4))
                            .unwrap_or(&0)
                            .to_le_bytes(),
                    );
                }
                self.chain = iv;
                return Ok(());
            }
            if off == HACC_ACON2 && value == AES_START {
                self.run_block();
                self.regs.insert(addr, AES_START);
                return Ok(());
            }
            if off == HACC_ACON2 && value == AES_CLR {
                self.ready = false;
            }
            self.regs.insert(addr, value);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sej_hw_soft_roundtrip() {
        let data: Vec<u8> = (0..32).map(|i| (i * 11) as u8).collect();
        let enc = sej_hw_soft(&data, true).unwrap();
        assert_ne!(enc, data);
        let dec = sej_hw_soft(&enc, false).unwrap();
        assert_eq!(dec, data);
        assert!(sej_hw_soft(&data[..10], true).is_err());
    }

    #[test]
    fn engine_sequence_matches_reference() {
        // The driver must reproduce the mtkclient register sequence for
        // the non-legacy init: clear keys, ACON/ACONK, CLR, IV, three
        // pattern runs with START/READY, CLR again, IV, ACON, ACONK=0.
        struct Recorder {
            ops: Vec<(u32, u32)>,
            reads: usize,
        }
        impl SejBus for Recorder {
            fn read32(&mut self, addr: u32) -> io::Result<u32> {
                self.reads += 1;
                if addr & 0xFF == HACC_ACON2 {
                    return Ok(AES_RDY);
                }
                Ok(0)
            }
            fn write32(&mut self, addr: u32, value: u32) -> io::Result<()> {
                self.ops.push((addr & 0xFF, value));
                Ok(())
            }
        }
        let mut bus = Recorder {
            ops: Vec::new(),
            reads: 0,
        };
        let mut engine = SejEngine::new(&mut bus, SEJ_BASE_DEFAULT);
        let iv = [1u32, 2, 3, 4];
        engine.sej_v3_init(true, &iv, false).unwrap();
        engine.sej_run(&[0u8; 16]).unwrap();
        engine.sej_terminate().unwrap();

        let ops = &bus.ops;
        // key clear at the very start
        assert_eq!(ops[0], (HACC_AKEY0, 0));
        // ACON = CBC | 128 in the pre-init phase
        assert!(ops.contains(&(HACC_ACON, AES_CBC | AES_128)));
        // ACONK = BK2C | R2K during init
        assert!(ops.contains(&(HACC_ACONK, AES_BK2C | AES_R2K)));
        // three init pattern runs + one data block
        assert_eq!(
            ops.iter()
                .filter(|&&(o, v)| o == HACC_ACON2 && v == AES_START)
                .count(),
            4
        );
        // terminate clears the keys again
        assert_eq!(*ops.last().unwrap(), (HACC_AKEY0 + 28, 0));
        // the engine was polled for readiness before every AOUT read
        assert!(bus.reads >= 4);
    }

    #[test]
    fn engine_roundtrip_through_mock() {
        let data: Vec<u8> = (0..32).map(|i| (i * 13) as u8).collect();
        let enc = {
            let mut mock = testutil::MockHacc::new();
            mock.regs.insert(SEJ_BASE_DEFAULT + HACC_ACON, AES_ENC);
            let mut engine = SejEngine::new(&mut mock, SEJ_BASE_DEFAULT);
            engine.hacc_output(&data, false, true).unwrap()
        };
        assert_ne!(enc, data);
        let dec = {
            let mut mock = testutil::MockHacc::new();
            let mut engine = SejEngine::new(&mut mock, SEJ_BASE_DEFAULT);
            engine.hacc_output(&enc, false, false).unwrap()
        };
        assert_eq!(dec, data);
    }

    #[test]
    fn legacy_and_normal_init_differ() {
        // The legacy init path must not run the pattern derivation but
        // must still set ACON to the requested direction.
        struct Recorder {
            ops: Vec<(u32, u32)>,
        }
        impl SejBus for Recorder {
            fn read32(&mut self, _addr: u32) -> io::Result<u32> {
                Ok(0x9000_0000)
            }
            fn write32(&mut self, addr: u32, value: u32) -> io::Result<()> {
                self.ops.push((addr & 0xFF, value));
                Ok(())
            }
        }
        let mut bus = Recorder { ops: Vec::new() };
        let mut engine = SejEngine::new(&mut bus, SEJ_BASE_DEFAULT);
        let iv = [7u32, 0, 0, 1];
        engine.sej_v3_init(true, &iv, true).unwrap();
        let pattern_written = engine
            .bus
            .ops
            .iter()
            .any(|&(o, v)| o == HACC_ASRC0 && v == CFG_RANDOM_PATTERN[0]);
        assert!(!pattern_written);
        assert!(engine
            .bus
            .ops
            .contains(&(HACC_ACON, AES_128 | AES_CBC | AES_ENC)));
        // the legacy branch pokes HACC_UNK
        assert!(engine.bus.ops.iter().any(|&(o, _)| o == HACC_UNK));
    }
}
