//! [`step`]: one [`Op`] on a Unicorn x86 CPU, in place of the emulator's
//! own version.

use unicorn_engine::{RegisterX86, Unicorn, uc_error};

use crate::x87::{Ext, Op, execute};

/// Runs the [`Op`] at `addr` (its two bytes) on `uc`'s x87 registers, as
/// the FPU does it: `ST0`/`ST1`, the stack top and tags for a push or a
/// pop, the condition codes and exception flags in the status word; then
/// sets `eip` past it. Call it from a code hook at `addr` or an interrupt
/// hook for an `int3` patched over it.
///
/// # Errors
/// When Unicorn refuses a register access.
pub fn step<D>(uc: &mut Unicorn<'_, D>, addr: u64, op: Op) -> Result<(), uc_error> {
    let st = |uc: &Unicorn<'_, D>, r| -> Result<Ext, uc_error> {
        let v = uc.reg_read_long(r)?;
        let mut b = [0u8; 10];
        b.copy_from_slice(&v[..10]);
        Ok(Ext::from_bytes(&b))
    };
    let word = |uc: &Unicorn<'_, D>, r| -> Result<u16, uc_error> {
        #[allow(clippy::cast_possible_truncation)] // 16-bit registers
        Ok(uc.reg_read(r)? as u16)
    };
    let (st0, st1) = (st(uc, RegisterX86::ST0)?, st(uc, RegisterX86::ST1)?);
    let cw = word(uc, RegisterX86::FPCW)?;
    let mut sw = word(uc, RegisterX86::FPSW)?;
    let e = execute(op, st0, st1, cw);
    // condition codes C0, C1, C2 (bits 8..10) and C3 (bit 14)
    for (k, bit) in [8u16, 9, 10, 14].into_iter().enumerate() {
        if let Some(v) = e.c[k] {
            sw = (sw & !(1 << bit)) | (u16::from(v) << bit);
        }
    }
    sw |= e.flags;
    let top = (sw >> 11) & 7;
    let mut tag = word(uc, RegisterX86::FPTAG)?;
    let set_top = |sw: u16, t: u16| (sw & !0x3800) | ((t & 7) << 11);
    if e.push {
        let t = top.wrapping_sub(1) & 7;
        uc.reg_write(RegisterX86::FPSW, u64::from(set_top(sw, t)))?;
        tag &= !(3 << (2 * t)); // the new ST0 is in use
        uc.reg_write(RegisterX86::FPTAG, u64::from(tag))?;
    } else {
        uc.reg_write(RegisterX86::FPSW, u64::from(sw))?;
    }
    if let Some(v) = e.st0 {
        uc.reg_write_long(RegisterX86::ST0, &v.to_bytes())?;
    }
    if let Some(v) = e.st1 {
        uc.reg_write_long(RegisterX86::ST1, &v.to_bytes())?;
    }
    if e.pop {
        tag |= 3 << (2 * top); // the old ST0 is empty
        uc.reg_write(RegisterX86::FPTAG, u64::from(tag))?;
        uc.reg_write(RegisterX86::FPSW, u64::from(set_top(sw, top + 1)))?;
    }
    uc.reg_write(RegisterX86::EIP, addr + 2)?;
    Ok(())
}
