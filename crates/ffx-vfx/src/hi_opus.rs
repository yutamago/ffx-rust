//! The `opus_0437`, `opus_0582` and `opus_0283` effects' helpers and
//! draws at `0x00a6d000..0x00a78000` (`docs/formats/vfx.md`, "High
//! range").

#![allow(clippy::many_single_char_names)]

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::Mem;

    /// Calls the game function at `sym` (rebased) through `call`.
    fn go<M: Mem>(
        mem: &mut M,
        sym: u32,
        args: &[u32],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(sym);
        call(mem, f, args)
    }

    /// The emulated EE's `r0` (`g_Ee_Gpr`, `0x00c8f508`, 16 bytes).
    const EE_R0: u32 = 0x00c8_f508;
    /// The emulated EE's `v0` (`g_Ee_Gpr + 0x20`), which the dot set copies
    /// through.
    const EE_COPY: u32 = 0x00c8_f528;

    /// `opus_0437_dot_init_00a6e540(b, n)` (`0x00a6e540`): the `opus_0437`
    /// dot buffer's header (`b[0]` = `n`, `b[1]` = 0) and its `n` 32-byte
    /// entries from `b + 0x10`: both halves the emulated `r0`, then `u16
    /// +0x1c` = 0x20, `u16 +0xc` = 0x8080, `u8 +0xf` = 0x80 and `u8 +0xe`
    /// = `op_random_per_call_007e5d40(0x40)` + 0x40 (through `call`). The
    /// count is reread from `b[0]` each entry. Returns `eax` as left: 0
    /// for no entries, else the last random draw with 0x40 added to its
    /// low byte.
    ///
    /// `op_random_per` takes four words and this passes one: the other
    /// three are the `edi`, `esi` and `ebx` the original pushed on entry
    /// (its caller's), which only reach the emulated VU0 registers
    /// (`0x00c8f974..0x00c8f97c` and the copies from them). `stale` gives
    /// them; the hook, which cannot see the caller's registers, passes 0.
    pub fn opus_0437_dot_init<M: Mem>(
        mem: &mut M,
        b: u32,
        n: u32,
        stale: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mem.write(b, n);
        mem.write(b.wrapping_add(4), 0u32);
        if n == 0 {
            return 0;
        }
        let r0 = mem.rebase(EE_R0);
        let mut e = b.wrapping_add(0x10);
        let mut i = 0u32;
        let mut eax;
        loop {
            for half in [0u32, 0x10] {
                for k in 0..4u32 {
                    let w: u32 = mem.read(r0.wrapping_add(4 * k));
                    mem.write(e.wrapping_add(half + 4 * k), w);
                }
            }
            mem.write(e.wrapping_add(0x1c), 0x20u16);
            mem.write(e.wrapping_add(0xc), 0x8080u16);
            mem.write(e.wrapping_add(0xf), 0x80u8);
            let [x, y, z] = stale;
            let r = go(mem, 0x007e_5d40, &[0x40, x, y, z], call);
            eax = (r & !0xff) | (r.wrapping_add(0x40) & 0xff);
            #[allow(clippy::cast_possible_truncation)] // the byte stored
            mem.write(e.wrapping_add(0xe), eax as u8);
            i += 1;
            e = e.wrapping_add(0x20);
            if i >= mem.read::<u32>(b) {
                return eax;
            }
        }
    }

    /// `opus_0437_dot_set(b, pos, colour)` (`0x00a6e5d0`): the next entry
    /// of the dot buffer `b` (entry `b[1] % b[0]`, 32 bytes from `b +
    /// 0x10`): its first half `op_xyz_scale(e, pos, 1.0)` (through
    /// `call`), its second the 16 bytes at `colour` (copied through the
    /// emulated `v0`, `0x00c8f528`, which keeps them), then `u16 +0x1c`
    /// cleared and `b[1]` counted up. Returns 0. (An empty buffer divides
    /// by zero in the original; here it takes entry 0.)
    pub fn opus_0437_dot_set<M: Mem>(
        mem: &mut M,
        b: u32,
        pos: u32,
        colour: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let n: u32 = mem.read(b);
        let i: u32 = mem.read(b.wrapping_add(4));
        let e = b
            .wrapping_add(0x10)
            .wrapping_add(i.checked_rem(n).unwrap_or(0).wrapping_shl(5));
        go(mem, 0x007e_a850, &[e, pos, 1.0f32.to_bits()], call);
        let reg = mem.rebase(EE_COPY);
        for k in 0..4u32 {
            let w: u32 = mem.read(colour.wrapping_add(4 * k));
            mem.write(reg.wrapping_add(4 * k), w);
        }
        for k in 0..4u32 {
            let w: u32 = mem.read(reg.wrapping_add(4 * k));
            mem.write(e.wrapping_add(0x10 + 4 * k), w);
        }
        mem.write(e.wrapping_add(0x1c), 0u16);
        let i: u32 = mem.read(b.wrapping_add(4));
        mem.write(b.wrapping_add(4), i.wrapping_add(1));
        0
    }
}
