//! The emulated R5900 and VU0 registers (`docs/hooking.md`, "The PS2
//! emulation registers"): the argument setters, `vf00`, the VU0 status
//! flags.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `g_Ee_Gpr`: 32 registers of 16 bytes.
    pub const GPR: Global<[u32; 4]> = Global::new(0x00c8_f508);
    /// The first argument register, `a0` (r4).
    pub const A0: u32 = 4;
    /// `g_Vu0_Vf00`: `vf00`, (0, 0, 0, 1).
    pub const VF00: Global<[f32; 4]> = Global::new(0x00c0_a004);
    /// VU0's status flags (`setZSFlag` sets bit 6, `setSSFlag` bit 7).
    pub const STATUS: Global<u32> = Global::new(0x00c8_fac4);

    /// `AsmEmuSetFuncParam0`..`3(v)` (`0x0042eec0` + 0x10 n): the low word
    /// of `a0`..`a3`; returns `v` (left in `eax`).
    pub fn set_func_param(mem: &mut impl Mem, n: u32, v: u32) -> u32 {
        let r = GPR.ptr(mem).at(A0 + n).addr;
        mem.write(r, v);
        v
    }

    /// `restoreVf00Register()` (`0x0042ef00`).
    pub fn restore_vf00(mem: &mut impl Mem) {
        let z = ffx_bridge::x87::Reg::ZERO.store();
        let one = ffx_bridge::x87::Reg::ONE.store();
        VF00.set(mem, [z, z, z, one]);
    }

    /// `setZSFlag()` (`0x0042ef30`): the sticky zero flag.
    pub fn set_zs_flag(mem: &mut impl Mem) {
        let v = STATUS.get(mem) | 0x40;
        STATUS.set(mem, v);
    }

    /// `setSSFlag()` (`0x0042ef20`): the sticky sign flag.
    pub fn set_ss_flag(mem: &mut impl Mem) {
        let v = STATUS.get(mem) | 0x80;
        STATUS.set(mem, v);
    }
}
