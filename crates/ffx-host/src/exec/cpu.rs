//! The executor's CPU state (docs/host.md, "Without Unicorn").
//!
//! With `dev-unicorn` it is the Unicorn instance itself: the registers the
//! emulated code runs on. Without it (the release) no `FFX.exe` code runs,
//! so the executor keeps only a register file with the same accessors:
//! `esp` (the frames the ports carve on the game stack, the words a call
//! from Rust puts there), `eax`/`ecx`/`edx` as a call leaves them, the
//! x87 and SSE control words, an empty x87 stack. The thread block's
//! segment and control registers are kept too, unused, so the code that
//! sets them up is the same in both builds.

#[cfg(feature = "dev-unicorn")]
pub(crate) use unicorn_engine::RegisterX86;

/// The executor's CPU: Unicorn's x86 instance.
#[cfg(feature = "dev-unicorn")]
pub(crate) type Cpu = unicorn_engine::Unicorn<'static, super::State>;

#[cfg(not(feature = "dev-unicorn"))]
pub(crate) use file::{Cpu, RegisterX86};

#[cfg(not(feature = "dev-unicorn"))]
mod file {
    use crate::Result;

    /// The registers the executor reads and writes (Unicorn's names).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[allow(clippy::upper_case_acronyms)] // Unicorn's names, one source for both builds
    pub(crate) enum RegisterX86 {
        EAX,
        ECX,
        EDX,
        EBX,
        ESP,
        EBP,
        ESI,
        EDI,
        EIP,
        EFLAGS,
        CS,
        SS,
        DS,
        ES,
        FS,
        GS,
        CR0,
        CR4,
        FPCW,
        FPSW,
        FPTAG,
        MXCSR,
        ST0,
        ST1,
        ST2,
        ST3,
        ST4,
        ST5,
        ST6,
        ST7,
        XMM0,
        GDTR,
    }

    /// Word registers (`EAX` .. `MXCSR`), in the enum's order.
    const WORDS: usize = RegisterX86::MXCSR as usize + 1;

    /// A register file in place of the emulator: what a register was last
    /// set to, nothing more.
    #[derive(Debug)]
    pub(crate) struct Cpu {
        words: [u64; WORDS],
        /// `ST0` .. `ST7` as 80-bit values (physical order is not modelled:
        /// the x87 stack stays empty, nothing pushes onto it).
        st: [[u8; 10]; 8],
        xmm0: [u8; 16],
        gdtr: [u8; 24],
        data: super::super::State,
    }

    impl Cpu {
        /// A fresh register file holding `data` (the executor's counters):
        /// all zero, the x87 stack empty (tags `0xffff`).
        pub(crate) fn new(data: super::super::State) -> Self {
            let mut words = [0; WORDS];
            words[RegisterX86::FPTAG as usize] = 0xffff;
            words[RegisterX86::MXCSR as usize] = 0x1f80;
            Self {
                words,
                st: [[0; 10]; 8],
                xmm0: [0; 16],
                gdtr: [0; 24],
                data,
            }
        }

        fn st_index(r: RegisterX86) -> Option<usize> {
            let i = (r as usize).checked_sub(RegisterX86::ST0 as usize)?;
            (i < 8).then_some(i)
        }

        /// A word register (`ST*`, `XMM0` and `GDTR`: their low 64 bits).
        #[allow(clippy::unnecessary_wraps)] // Unicorn's signature
        pub(crate) fn reg_read(&self, r: RegisterX86) -> Result<u64> {
            let i = r as usize;
            if i < WORDS {
                return Ok(self.words[i]);
            }
            let long = self.reg_read_long(r)?;
            let mut low = [0u8; 8];
            for (d, s) in low.iter_mut().zip(long.iter()) {
                *d = *s;
            }
            Ok(u64::from_le_bytes(low))
        }

        /// Sets a word register (32 bits kept, as the x86's own).
        #[allow(clippy::unnecessary_wraps)] // Unicorn's signature
        pub(crate) fn reg_write(&mut self, r: RegisterX86, v: u64) -> Result<()> {
            let i = r as usize;
            if i < WORDS {
                self.words[i] = v & 0xffff_ffff;
                return Ok(());
            }
            self.reg_write_long(r, &v.to_le_bytes())
        }

        /// An x87, SSE or descriptor-table register's bytes.
        #[allow(clippy::unnecessary_wraps)] // Unicorn's signature
        pub(crate) fn reg_read_long(&self, r: RegisterX86) -> Result<Vec<u8>> {
            Ok(match (r, Self::st_index(r)) {
                (_, Some(i)) => self.st[i].to_vec(),
                (RegisterX86::XMM0, _) => self.xmm0.to_vec(),
                (RegisterX86::GDTR, _) => self.gdtr.to_vec(),
                _ => self.reg_read(r)?.to_le_bytes().to_vec(),
            })
        }

        /// Sets an x87, SSE or descriptor-table register's bytes (as many
        /// as it holds).
        #[allow(clippy::unnecessary_wraps)] // Unicorn's signature
        pub(crate) fn reg_write_long(&mut self, r: RegisterX86, v: &[u8]) -> Result<()> {
            let put = |d: &mut [u8]| {
                d.fill(0);
                for (d, s) in d.iter_mut().zip(v) {
                    *d = *s;
                }
            };
            match (r, Self::st_index(r)) {
                (_, Some(i)) => put(&mut self.st[i]),
                (RegisterX86::XMM0, _) => put(&mut self.xmm0),
                (RegisterX86::GDTR, _) => put(&mut self.gdtr),
                _ => {
                    let mut w = [0u8; 8];
                    put(&mut w);
                    self.reg_write(r, u64::from_le_bytes(w))?;
                }
            }
            Ok(())
        }

        /// The executor's counters (nothing counts them without the
        /// emulator).
        pub(crate) fn get_data(&self) -> &super::super::State {
            &self.data
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{Cpu, RegisterX86};

        #[test]
        fn registers_keep_what_was_written() {
            let mut c = Cpu::new(super::super::super::State::default());
            assert_eq!(c.reg_read(RegisterX86::FPTAG).unwrap(), 0xffff);
            assert_eq!(c.reg_read(RegisterX86::MXCSR).unwrap(), 0x1f80);
            c.reg_write(RegisterX86::ESP, 0x1_0300_0ff0).unwrap();
            assert_eq!(c.reg_read(RegisterX86::ESP).unwrap(), 0x0300_0ff0);
            c.reg_write_long(RegisterX86::ST3, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])
                .unwrap();
            assert_eq!(
                c.reg_read_long(RegisterX86::ST3).unwrap(),
                [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
            );
            assert_eq!(c.reg_read_long(RegisterX86::ST2).unwrap(), [0; 10]);
            c.reg_write_long(RegisterX86::XMM0, &7u64.to_le_bytes())
                .unwrap();
            assert_eq!(c.reg_read(RegisterX86::XMM0).unwrap(), 7);
            assert_eq!(c.reg_read_long(RegisterX86::XMM0).unwrap().len(), 16);
        }
    }
}
