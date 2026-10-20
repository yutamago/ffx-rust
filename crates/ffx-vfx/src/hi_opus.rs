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

    /// `opus_0582_dot_set_00a73740(b, rec)` (`0x00a73740`,
    /// `opp_0582_cmd_a70910` kind 0x2000): the next entry `e` of the
    /// `opus_0582` dot buffer `b` (entry `b[1] % b[0]`, 32 bytes from `b +
    /// 0x10`, the layout `opus_0582_dot_draw` reads) from the part `rec`:
    /// `+0`, `+8` the radii `rec +0x40`, `+0x48` / 256, `+4` the height
    /// `rec +0x94`, `+0xc` the scale `rec +0x3c` / 16, `u16 +0x10` the
    /// angle `ftol(rec +0x30)`, `+0x12` the step `ftol(ftol(rec +0x34) /
    /// 256)` (the integer back through an `f32`), `+0x14` the spread
    /// `ftol(rec +0x38)` (0 when negative as `i16`), `+0x16` the life
    /// `ftol(rec +0x4c) + 16`, `+0x18` the colour `rec +0xc8`, `u8 +0x1c`
    /// the fade-in 8; then `b[1]` counted up. Returns the colour (`eax`).
    /// (An empty buffer divides by zero in the original; here it takes
    /// entry 0.)
    pub fn opus_0582_dot_set(mem: &mut impl Mem, b: u32, rec: u32) -> u32 {
        use ffx_bridge::x87;
        const K256: f64 = 0.003_906_25;
        fn fld(mem: &impl Mem, at: u32) -> f64 {
            x87::load(mem.read(at))
        }
        #[allow(clippy::cast_possible_truncation)] // the u16 stores
        fn word(v: u32) -> u16 {
            v as u16
        }
        let n: u32 = mem.read(b);
        let i: u32 = mem.read(b.wrapping_add(4));
        let e = b
            .wrapping_add(0x10)
            .wrapping_add(i.checked_rem(n).unwrap_or(0).wrapping_shl(5));
        let r = |o: u32| rec.wrapping_add(o);
        let height = x87::store(fld(mem, r(0x94)));
        mem.write(e.wrapping_add(4), height);
        let v = x87::store(x87::mul(fld(mem, r(0x40)), K256));
        mem.write(e, v);
        let v = x87::store(x87::mul(fld(mem, r(0x48)), K256));
        mem.write(e.wrapping_add(8), v);
        let v = x87::store(x87::mul(fld(mem, r(0x3c)), 0.0625));
        mem.write(e.wrapping_add(0xc), v);
        let v = x87::ftol(mem, fld(mem, r(0x30)));
        mem.write(e.wrapping_add(0x10), word(v));
        let step = x87::ftol(mem, fld(mem, r(0x34)));
        let step = x87::load(x87::store(f64::from(step.cast_signed())));
        let v = x87::ftol(mem, x87::mul(K256, step));
        mem.write(e.wrapping_add(0x12), word(v));
        let v = word(x87::ftol(mem, fld(mem, r(0x38))));
        mem.write(e.wrapping_add(0x14), v);
        if f64::from(v.cast_signed()) < 0.0 {
            mem.write(e.wrapping_add(0x14), 0u16);
        }
        let v = x87::ftol(mem, fld(mem, r(0x4c))).wrapping_add(0x10);
        mem.write(e.wrapping_add(0x16), word(v));
        let colour: u32 = mem.read(r(0xc8));
        mem.write(e.wrapping_add(0x18), colour);
        mem.write(e.wrapping_add(0x1c), 8u8);
        let i: u32 = mem.read(b.wrapping_add(4));
        mem.write(b.wrapping_add(4), i.wrapping_add(1));
        colour
    }

    /// `opus_0437_dot_draw(b)` (`0x00a6e2e0`, `opp_0437_cmd_a6dbe0` kind
    /// 0x2000): the dot buffer `b` handed to the renderer's fill job.
    /// First the PS2 packet the original still builds in a `0x1100`-byte
    /// `op_spr_alloc` scratch: a DMA tag block at `+0x1000` (`0x70000002`,
    /// `0x8001`/`0x10000000`, the `TEST`/`ALPHA` register writes `0x48`,
    /// `0x42`) sent with `sceGsSyncPath(0, 0)` and `sceDmaSend([0x023328e0],
    /// block | 0x80000000)`, and two quadwords at `+0` (`0x70000000`; the
    /// `GIF` tag `0x8000`, `0x20244000`, `0x41`) copied to `+0x800` through
    /// the EE's `v0` (`0x00c8f528`; the first's high words are the
    /// scratch's). Then three line instances `(1000 + i, 0x1b5)`: made with
    /// `createVFXRenderDataBufferLine(id, 3000, 2)` when
    /// `graphicVFXGetRenderDataByInstance` has none (then `+0xb8` = 1 and
    /// `graphicVFXSetup`), each needing render data and
    /// `graphicDynGeoMemManagerGetBufferForSegment(rd, 0, geo +4, geo +8)`
    /// (`geo` = `rd +0x94`), else it stops; `FFXVu0UnitMatrix(rd)`,
    /// `graphicVFXSetVisible(id, 1)`, `graphicVFXPath3Update(id)`. When
    /// all three are there and `graphicDynGeoFillBufferIsValid`, the
    /// request `JobSchedule_pppPart_getFillBufferRequest(b[0])` gets kind
    /// 4, the three `geo`s, `b + 0x10` and `b[0]`. `op_spr_free(0x1100)`
    /// at the end; its `eax` is returned. All callees through `call`.
    pub fn opus_0437_dot_draw<M: Mem>(
        mem: &mut M,
        b: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let p = go(mem, 0x007e_6a00, &[0x1100], call);
        let q = p.wrapping_add(0x1000);
        for (k, v) in (0u32..).zip([
            0x7000_0002u32,
            0,
            0,
            0,
            0x8001,
            0x1000_0000,
            0xeeee_eeee,
            0,
            0x48,
            0,
            0x42,
            0,
        ]) {
            mem.write(q.wrapping_add(4 * k), v);
        }
        go(mem, 0x0062_f880, &[0, 0], call);
        let channel: u32 = mem.read(mem.rebase(0x0233_28e0));
        go(mem, 0x0062_f710, &[channel, q | 0x8000_0000], call);
        mem.write(p, 0x7000_0000u32);
        mem.write(p.wrapping_add(4), 0u32);
        let first: u32 = mem.read(p);
        for (k, v) in (4u32..).zip([0x8000u32, 0x2024_4000, 0x41, 0]) {
            mem.write(p.wrapping_add(4 * k), v);
        }
        let v0 = mem.rebase(EE_COPY);
        for half in [0u32, 0x10] {
            for k in 0..4u32 {
                let w: u32 = if half == 0 && k == 0 {
                    first
                } else {
                    mem.read(p.wrapping_add(half + 4 * k))
                };
                mem.write(v0.wrapping_add(4 * k), w);
            }
            for k in 0..4u32 {
                let w: u32 = mem.read(p.wrapping_add(half + 4 * k));
                mem.write(p.wrapping_add(0x800 + half + 4 * k), w);
            }
        }
        let (list, count) = (b.wrapping_add(0x10), mem.read::<u32>(b));
        let mut geo = [0u32; 3];
        let mut ready = true;
        for (i, g) in (0u32..).zip(geo.iter_mut()) {
            let id = [1000 + i, 0x1b5];
            let fresh = go(mem, 0x0064_53d0, &id, call) == 0;
            if fresh {
                go(mem, 0x0071_4b60, &[id[0], id[1], 3000, 2], call);
            }
            let rd = go(mem, 0x0064_53d0, &id, call);
            if rd == 0 {
                ready = false;
                break;
            }
            if fresh {
                mem.write(rd.wrapping_add(0xb8), 1u32);
                go(mem, 0x0064_5780, &id, call);
            }
            let d: u32 = mem.read(rd.wrapping_add(0x94));
            let (a, c): (u32, u32) = (mem.read(d.wrapping_add(4)), mem.read(d.wrapping_add(8)));
            if go(mem, 0x0063_fdb0, &[rd, 0, a, c], call) == 0 {
                ready = false;
                break;
            }
            *g = mem.read(rd.wrapping_add(0x94));
            go(mem, 0x006e_d9a0, &[rd], call);
            go(mem, 0x0064_5710, &[id[0], id[1], 1], call);
            go(mem, 0x0064_5600, &id, call);
        }
        if ready && go(mem, 0x0063_fd70, &[], call) != 0 {
            let r = go(mem, 0x0072_8660, &[count], call);
            mem.write(r, 4u8);
            for (k, g) in (0u32..).zip(geo) {
                mem.write(r.wrapping_add(0x10 + 4 * k), g);
            }
            mem.write(r.wrapping_add(0x1c), list);
            mem.write(r.wrapping_add(0x20), count);
        }
        go(mem, 0x007e_6a60, &[0x1100], call)
    }

    /// `opus_0437_poly_init_00a6e1b0(b, list, m, k)` (`0x00a6e1b0`,
    /// `opp_0437_cmd_a6dbe0` kind 0x4000): the `opus_0437` polygon
    /// buffer's header: `u16 b +0` = `k`, `u16 +2` = 0, `u16 +4` =
    /// `op_md_list_count10_00a6a2b0(list)`; `k` 0x60-byte entries from
    /// `b +0x30`, each `u16 +0xc` = 0xffff; `ptr32 b +0xc` = their end,
    /// from which `op_md_list_count10(list)` (asked again) `u16`s are
    /// 0xffff (two at a time, as words); then
    /// `op_md_list_bounds_00a6a0c0(list, m, b +0x10, b +0x20)`, whose
    /// `eax` it returns. Callees through `call`.
    pub fn opus_0437_poly_init<M: Mem>(
        mem: &mut M,
        b: u32,
        list: u32,
        m: u32,
        k: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low half
        mem.write(b, k as u16);
        mem.write(b.wrapping_add(2), 0u16);
        let n = go(mem, 0x00a6_a2b0, &[list], call);
        #[allow(clippy::cast_possible_truncation)]
        mem.write(b.wrapping_add(4), n as u16);
        let mut at = b.wrapping_add(0x30);
        for _ in 0..k.cast_signed().max(0) {
            mem.write(at.wrapping_add(0xc), 0xffffu16);
            at = at.wrapping_add(0x60);
        }
        mem.write(b.wrapping_add(0xc), at);
        let n = go(mem, 0x00a6_a2b0, &[list], call);
        if n.cast_signed() > 0 {
            for _ in 0..n >> 1 {
                mem.write(at, u32::MAX);
                at = at.wrapping_add(4);
            }
            if n & 1 != 0 {
                mem.write(at, 0xffffu16);
            }
        }
        go(
            mem,
            0x00a6_a0c0,
            &[list, m, b.wrapping_add(0x10), b.wrapping_add(0x20)],
            call,
        )
    }
}
