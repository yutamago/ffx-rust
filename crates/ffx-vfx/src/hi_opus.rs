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

    /// `f64` 1000 (`0x00b0dba0`).
    const K_1000: f64 = 1000.0;
    /// `f64` 3 (`0x00b2c5c8`).
    const K_3: f64 = 3.0;
    /// `f64` 0.333333 (`0x00b501f8`, an `f32` 0.333333 widened).
    const K_THIRD: u32 = 0x00b5_01f8;

    /// `opus_0437_poly_check(list, b, dots, m, limit)` (`0x00a6ed40`, from
    /// `opp_0437_cmd_a6dbe0`): the polygons of a model list that sank to
    /// `limit` become `opus_0437` polygon entries.
    ///
    /// For each of the `u16 list +4` packets (`ptr32 list +0x10 + 0x10 k`,
    /// its matrix the `ptr32` after it): the matrix's translation times
    /// 1000 (`f32`), `op_mul_m(frame - 0x48, m, matrix)`, divided back
    /// (`f32`); `buf = user_malloc(i16 packet +0x12 << 4)` gets every
    /// vertex (16 bytes from `packet + [packet +8]`) through
    /// `FFXVu0ApplyMatrix`. Then each tag of type 0x10 (from `packet +
    /// [packet +4]`, `i16 +2` polygons of 0x1c bytes from `tag +0x20`,
    /// three `u16` vertex numbers `a, b, c` at `+0`), with `idx` the
    /// polygon's number over the whole list: when `u16 [b +0xc][idx]` is
    /// 0xffff and `y = f32(f32(f32(B.y + A.y) + C.y) / 3)` (`A` = vertex
    /// `b`, `B` = `a`, `C` = `c`) is `<= limit` (never for a NaN), entry
    /// `s = u16 b +2` (0x60 bytes from `b +0x30`) takes it.
    ///
    /// The entry: `[b +0xc][idx] = s`; the polygon it held (`i16 +0xc`,
    /// when not negative) gets 0xfffe unless negative; `u16 b +2 = (s + 1)
    /// % u16 b +0` (`idiv`, which faults on 0); the centre `f32((A + B + C)
    /// x 0.333333)` per axis (one rounding) at `+0`, the emulated `r0` at
    /// `+0x50`, `u16 +0x1c` 0, `u16 +0xc` `idx`, `u8 +0xe` 0, `u16 +0x5e`
    /// 0x1000.
    ///
    /// With `dots`, eight dots more, each the next of `dots[1] % dots[0]`
    /// (32 bytes from `dots + 0x10`; `dots[1]` up by one): `r =
    /// op_random(4)`, `op_xyz_scale(dot, v, 1.0)` with `v = (x + r, r + y,
    /// r + z)` at `frame - 0x60` (its `w` at `frame - 0x54` never written:
    /// `junk[0]`), then `(0, f32((op_random(16) - 20) x 0.0625), 0,
    /// junk[1])` into the emulated `v0` (`0x00c8f528`; written z, w, x, y)
    /// and `dot +0x10`, whose `u16 +0x1c` is then 0. `junk[1]` is the
    /// original's frame word `ebp - 0xa4`, never written either. Last
    /// `user_free_thunk(buf)`.
    ///
    /// Callees through `call` but `op_random` (ours). Returns `eax` as
    /// left: `u16 list +4`.
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    pub fn opus_0437_poly_check<M: Mem>(
        mem: &mut M,
        list: u32,
        b: u32,
        dots: u32,
        m: u32,
        limit: f32,
        junk: [u32; 2],
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        let ptr32 = ffx_system::stub::ptr32;
        let (local, v) = (frame.wrapping_sub(0x48), frame.wrapping_sub(0x60));
        mem.write(v.wrapping_add(0xc), junk[0]);
        let r0 = mem.rebase(EE_R0);
        let v0 = mem.rebase(EE_COPY);
        let third: f64 = mem.read(mem.rebase(K_THIRD));
        let rd = |mem: &M, at: u32| load(mem.read::<u32>(at));
        let mut idx = 0u32;
        let mut e = list.wrapping_add(0x10);
        let mut k = 0u32;
        while k < u32::from(mem.read::<u16>(list.wrapping_add(4))) {
            let packet = ptr32(mem.read(e));
            let verts = packet.wrapping_add(mem.read::<u32>(packet.wrapping_add(8)));
            for o in [0x30u32, 0x34, 0x38] {
                let t = ptr32(mem.read(e.wrapping_add(4))).wrapping_add(o);
                mem.write(t, store(mul(rd(mem, t), K_1000)));
            }
            let tm = ptr32(mem.read(e.wrapping_add(4)));
            go(mem, 0x007e_83b0, &[local, m, tm], call);
            for o in [0x30u32, 0x34, 0x38] {
                let t = ptr32(mem.read(e.wrapping_add(4))).wrapping_add(o);
                mem.write(t, store(div(rd(mem, t), K_1000)));
            }
            let size = i32::from(mem.read::<i16>(packet.wrapping_add(0x12))) << 4;
            let buf = go(mem, 0x0068_7020, &[size.cast_unsigned()], call);
            let mut i = 0i32;
            while i < i32::from(mem.read::<i16>(packet.wrapping_add(0x12))) {
                let o = i.cast_unsigned() << 4;
                let args = [buf.wrapping_add(o), local, verts.wrapping_add(o)];
                go(mem, 0x006e_d2e0, &args, call);
                i += 1;
            }
            let vertex = |mem: &M, at: u32| buf.wrapping_add(u32::from(mem.read::<u16>(at)) << 4);
            let mut tag = packet.wrapping_add(mem.read::<u32>(packet.wrapping_add(4)));
            while mem.read::<u8>(tag.wrapping_add(1)) == 0x10 {
                let mut poly = tag.wrapping_add(0x20);
                let mut j = 0i32;
                while j < i32::from(mem.read::<i16>(tag.wrapping_add(2))) {
                    let table: u32 = mem.read(b.wrapping_add(0xc));
                    if mem.read::<u16>(table.wrapping_add(idx.wrapping_mul(2))) == 0xffff {
                        let pa = vertex(mem, poly.wrapping_add(2));
                        let pb = vertex(mem, poly);
                        let pc = vertex(mem, poly.wrapping_add(4));
                        let y = store(add(
                            rd(mem, pa.wrapping_add(4)),
                            rd(mem, pb.wrapping_add(4)),
                        ));
                        let y = store(add(load(y), rd(mem, pc.wrapping_add(4))));
                        let y = f32::from_bits(store(div(load(y), K_3)));
                        if y <= limit {
                            let slot: u16 = mem.read(b.wrapping_add(2));
                            mem.write(table.wrapping_add(idx.wrapping_mul(2)), slot);
                            let ent = b.wrapping_add(0x30 + 0x60 * u32::from(slot));
                            let prev = mem.read::<i16>(ent.wrapping_add(0xc));
                            if prev >= 0 {
                                let table: u32 = mem.read(b.wrapping_add(0xc));
                                let at = table.wrapping_add(i32::from(prev).cast_unsigned() * 2);
                                if mem.read::<i16>(at) >= 0 {
                                    mem.write(at, 0xfffeu16);
                                }
                            }
                            let n = i32::from(mem.read::<u16>(b.wrapping_add(2))) + 1;
                            let d = i32::from(mem.read::<u16>(b));
                            // dx; a 0 count faults in the original
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            mem.write(b.wrapping_add(2), n.checked_rem(d).unwrap_or(0) as u16);
                            for c in 0..3u32 {
                                let s = add(
                                    add(
                                        rd(mem, pa.wrapping_add(4 * c)),
                                        rd(mem, pb.wrapping_add(4 * c)),
                                    ),
                                    rd(mem, pc.wrapping_add(4 * c)),
                                );
                                mem.write(ent.wrapping_add(4 * c), store(mul(s, third)));
                            }
                            let r: [u32; 4] = mem.read(r0);
                            mem.write(ent.wrapping_add(0x50), r);
                            mem.write(ent.wrapping_add(0x1c), 0u16);
                            #[allow(clippy::cast_possible_truncation)] // the low half
                            mem.write(ent.wrapping_add(0xc), idx as u16);
                            mem.write(ent.wrapping_add(0xe), 0u8);
                            mem.write(ent.wrapping_add(0x5e), 0x1000u16);
                            if dots != 0 {
                                for _ in 0..8 {
                                    let r = crate::rand::bridge::op_random(mem, 4.0);
                                    let r = f64::from(r);
                                    mem.write(v, store(add(rd(mem, ent), r)));
                                    mem.write(
                                        v.wrapping_add(4),
                                        store(add(r, rd(mem, ent.wrapping_add(4)))),
                                    );
                                    mem.write(
                                        v.wrapping_add(8),
                                        store(add(r, rd(mem, ent.wrapping_add(8)))),
                                    );
                                    let r2 = f64::from(crate::rand::bridge::op_random(mem, 16.0));
                                    let w = store(mul(sub(r2, 20.0), 0.0625));
                                    let n: u32 = mem.read(dots);
                                    let at: u32 = mem.read(dots.wrapping_add(4));
                                    // a 0 count faults in the original
                                    let i = at.checked_rem(n).unwrap_or(0);
                                    let dot = dots.wrapping_add(0x10).wrapping_add(i << 5);
                                    go(mem, 0x007e_a850, &[dot, v, 0x3f80_0000], call);
                                    mem.write(v0.wrapping_add(8), 0u32);
                                    mem.write(v0.wrapping_add(0xc), junk[1]);
                                    mem.write(v0, 0u32);
                                    mem.write(v0.wrapping_add(4), w);
                                    mem.write(dot.wrapping_add(0x10), [0, w, 0, junk[1]]);
                                    mem.write(dot.wrapping_add(0x1c), 0u16);
                                    let at = dots.wrapping_add(4);
                                    mem.write(at, mem.read::<u32>(at).wrapping_add(1));
                                }
                            }
                        }
                    }
                    idx = idx.wrapping_add(1);
                    poly = poly.wrapping_add(0x1c);
                    j += 1;
                }
                tag = poly;
            }
            go(mem, 0x0068_7010, &[buf], call);
            e = e.wrapping_add(0x10);
            k += 1;
        }
        u32::from(mem.read::<u16>(list.wrapping_add(4)))
    }

    /// `f64` 2048 (`0x00b51bc0`).
    const K_2048: u32 = 0x00b5_1bc0;
    /// `f64` 256 (`0x00b922f0`).
    const K_256: u32 = 0x00b9_22f0;
    /// `f64` 208 (`0x00b54c70`).
    const K_208: u32 = 0x00b5_4c70;
    /// `f64` 1/512 (`0x00b41cc0`).
    const K_512TH: u32 = 0x00b4_1cc0;
    /// `f32` 0.8125 (`0x00b582f8`).
    const K_0_8125: u32 = 0x00b5_82f8;
    /// `f64` 1792 (`0x00b583b8`).
    const K_1792: u32 = 0x00b5_83b8;
    /// `f64` 1840 (`0x00b51bb8`).
    const K_1840: u32 = 0x00b5_1bb8;
    /// `f32` 0.001 (`0x00b368a4`).
    const K_0_001: u32 = 0x00b3_68a4;
    /// `f32` 0.01 (`0x00b1f2b0`).
    const K_0_01: u32 = 0x00b1_f2b0;
    const OP_RANDOM_P: u32 = 0x007e_6780;

    /// The map buffer's entries: 0x740 of them, 0x58 bytes each, from
    /// `b + 0x20`.
    const MAP_ENTRIES: u32 = 0x740;
    const MAP_ENTRY: u32 = 0x58;

    /// `opp_map_draw_init_00a753f0(b, pos)` (`0x00a753f0`; kind 0 of
    /// `opp_map_draw_a74c60`): `[b]` = 0, then with `X = f32(pos.x +
    /// 2048)`, `Y = f32(pos.y + 2048)` and `k` = 1/512 the 0x740 entries
    /// `e` (0x58 bytes from `b + 0x20`) round a 512 x 416 rectangle, each
    /// side's `i` counted in `f32` (`x0 = X - 256`, `y0 = Y - 208` kept
    /// unrounded until the store):
    /// - 0x200 bottom ones: `+0x10` `x0 + i`, `+0x14` `f32(y0)`, `+0x18`
    ///   0, `+0x38` `i k`, `+0x3c` 0;
    /// - 0x1a0 right: `+0x10` `f32(X + 256)`, `+0x14` `y0 + i`, `+0x18` 0,
    ///   `+0x38` 1, `+0x3c` `i k`;
    /// - 0x200 top, `i` from 0x1ff down: `+0x10` `x0 + i`, `+0x14` `f32(Y +
    ///   208)`, `+0x18` 0, `+0x38` `i k`, `+0x3c` 0.8125;
    /// - 0x1a0 left, `i` from 0x19f down: `+0x10` `f32(x0)`, `+0x14` `y0 +
    ///   i`, `+0x18` 0, and the next entry's `+0x38` 0 and `+0x3c` `i k`
    ///   (the original's off-by-one: the side's first entry keeps its old
    ///   uv, and the last pair lands 8 bytes past the 0x27e20-byte
    ///   buffer).
    ///
    /// Then every entry: `+0` `X`, `+4` `Y`, `+8` 0, `+0x20` and `+0x24`
    /// 0x80808080, `+0x30` `f32((X - 1792) k)`, `+0x34` `f32(k (Y -
    /// 1840))`, `+0x40..+0x48` 0, `op_random_p(0.001)` (dropped; through
    /// `call`), `+0x4c` 0.01, `+0x50` and `+0x54` 0. Returns `eax` as the
    /// last `op_random_p` left it.
    pub fn opp_map_draw_init<M: Mem>(
        mem: &mut M,
        b: u32,
        pos: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87;
        let f64_at = |mem: &M, sym: u32| -> f64 { mem.read(mem.rebase(sym)) };
        let f32_at = |mem: &M, sym: u32| -> u32 { mem.read(mem.rebase(sym)) };
        mem.write(b, 0u32);
        let k2048 = f64_at(mem, K_2048);
        let x = x87::store(x87::add(x87::load(mem.read(pos)), k2048));
        let y = x87::store(x87::add(k2048, x87::load(mem.read(pos.wrapping_add(4)))));
        let (xf, yf) = (x87::load(x), x87::load(y));
        let x0 = x87::sub(xf, f64_at(mem, K_256));
        let y0 = x87::sub(yf, f64_at(mem, K_208));
        let k = f64_at(mem, K_512TH);
        let entry = |j: u32| b.wrapping_add(0x20 + MAP_ENTRY * j);
        let at = |e: u32, o: u32, v: u32, mem: &mut M| mem.write(e.wrapping_add(o), v);
        // an f32 count, exact
        #[allow(clippy::cast_precision_loss)]
        let fi = |i: u32| f64::from(i as f32);
        let zero = 0u32;
        let one = 1.0f32.to_bits();
        let y0f = x87::store(y0);
        let mut j = 0u32;
        for i in 0..0x200u32 {
            let e = entry(j);
            at(e, 0x10, x87::store(x87::add(fi(i), x0)), mem);
            at(e, 0x14, y0f, mem);
            at(e, 0x18, zero, mem);
            at(e, 0x38, x87::store(x87::mul(fi(i), k)), mem);
            at(e, 0x3c, zero, mem);
            j += 1;
        }
        let right = x87::store(x87::add(f64_at(mem, K_256), xf));
        for i in 0..0x1a0u32 {
            let e = entry(j);
            at(e, 0x10, right, mem);
            at(e, 0x14, x87::store(x87::add(fi(i), y0)), mem);
            at(e, 0x18, zero, mem);
            at(e, 0x38, one, mem);
            at(e, 0x3c, x87::store(x87::mul(fi(i), k)), mem);
            j += 1;
        }
        let top = x87::store(x87::add(yf, f64_at(mem, K_208)));
        let v = f32_at(mem, K_0_8125);
        for i in (0..0x200u32).rev() {
            let e = entry(j);
            at(e, 0x10, x87::store(x87::add(fi(i), x0)), mem);
            at(e, 0x14, top, mem);
            at(e, 0x18, zero, mem);
            at(e, 0x38, x87::store(x87::mul(fi(i), k)), mem);
            at(e, 0x3c, v, mem);
            j += 1;
        }
        let left = x87::store(x0);
        for i in (0..0x1a0u32).rev() {
            let e = entry(j);
            at(e, 0x10, left, mem);
            at(e, 0x14, x87::store(x87::add(fi(i), y0)), mem);
            at(e, 0x18, zero, mem);
            let next = entry(j + 1);
            at(next, 0x38, zero, mem);
            at(next, 0x3c, x87::store(x87::mul(fi(i), k)), mem);
            j += 1;
        }
        let u = x87::store(x87::mul(x87::sub(xf, f64_at(mem, K_1792)), k));
        let w = x87::store(x87::mul(k, x87::sub(yf, f64_at(mem, K_1840))));
        let (small, tiny) = (f32_at(mem, K_0_001), f32_at(mem, K_0_01));
        let mut eax = 0;
        for j in 0..MAP_ENTRIES {
            let e = entry(j);
            at(e, 0, x, mem);
            at(e, 0x20, 0x8080_8080, mem);
            at(e, 0x24, 0x8080_8080, mem);
            at(e, 4, y, mem);
            at(e, 8, zero, mem);
            at(e, 0x30, u, mem);
            at(e, 0x34, w, mem);
            for o in [0x40u32, 0x44, 0x48] {
                at(e, o, zero, mem);
            }
            eax = go(mem, OP_RANDOM_P, &[small], call);
            at(e, 0x4c, tiny, mem);
            at(e, 0x50, zero, mem);
            at(e, 0x54, zero, mem);
        }
        eax
    }

    /// `f64` 1/4096 (`0x00b50190`).
    const K_4096TH: u32 = 0x00b5_0190;
    /// `op_deg_get` (`0x007e7ed0`), which returns its angle in `ST0`.
    const OP_DEG_GET: u32 = 0x007e_7ed0;

    /// `opp_md_break_pieces_00a744e0(src, out, rec)` (`0x00a744e0`, name
    /// ours; from `opp_md_break_init_a74410`): the pieces of a breaking
    /// model. `out[1]` = `n = i16 src +0x10`, `out[0]` = `src[0]`,
    /// `out[2..4]` = `src +0x28..`, `u16 out +0x10` = `u16 src +0x16`.
    ///
    /// Piece `k` (0x20 bytes from `src +0x30`, out 0x100 bytes from `out
    /// +0x20`): its words `+0..+8` and `+0x14..+0x1c` copied to `+0` and
    /// `+0x40`; the three vertices `i16 +0xc`, `+0xe`, `+0x10` (`i16`
    /// triples, 6 bytes each, from `src + [src +8]`) as `f32`s `A`, `B`,
    /// `C`; the centre `m = f32((B + A + C) / 3)` per axis at `+0x50`, each
    /// vertex minus it (`f32`) at `+0x10`, `+0x20`, `+0x30`. Twelve random
    /// values `f32(op_random(f32(i16 rec + r)) + f32(i16 rec + b))` at
    /// `+0x60..+0x68`, `+0x70..+0x78` (`b` from `rec +6`, `r` from `rec
    /// +0xc`, two bytes on each) and `+0x90..+0x98`, `+0xa0..+0xa8` (`b`
    /// from `rec +0x1e`, `r` from `rec +0x24`); `+0x80..+0x88` 0; `+0xb0`
    /// `i16 rec +0x36`; `+0xb4` `op_random_per(i16 rec +0x3a) + i16 rec
    /// +0x38`, whose other three words are the last `op_random` result
    /// (which it leaves in its argument's slot) and the caller's `edi` and
    /// `esi` (`junk`). With `u16 out +0x12` 1:
    /// `op_deg_get(0, 0, m.x, m.z, frame - 0x2c)` (through `fcall`) gives
    /// `+0x58` and the length it stores, `+0x50`. Then with `s = f32(f32(i16
    /// rec +4) / 4096)`: kind 0 scales `+0x60..+0x68` and `+0x70..+0x78`
    /// by `s`, kind 1 `+0x60`, `+0x64`, `+0x70`, `+0x74`. Returns `eax` as
    /// left: `i16 rec +4` after a piece, else `src + [src +8]`.
    #[allow(clippy::too_many_lines)]
    pub fn opp_md_break_pieces<M: Mem>(
        mem: &mut M,
        src: u32,
        out: u32,
        rec: u32,
        junk: [u32; 2],
        frame: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f32,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        let n = i32::from(mem.read::<i16>(src.wrapping_add(0x10)));
        mem.write(out.wrapping_add(4), n);
        mem.write(out, mem.read::<u32>(src));
        mem.write(out.wrapping_add(8), mem.read::<u32>(src.wrapping_add(0x28)));
        mem.write(
            out.wrapping_add(0xc),
            mem.read::<u32>(src.wrapping_add(0x2c)),
        );
        mem.write(
            out.wrapping_add(0x10),
            mem.read::<u16>(src.wrapping_add(0x16)),
        );
        let verts = src.wrapping_add(mem.read::<u32>(src.wrapping_add(8)));
        let half = |mem: &M, at: u32| f64::from(mem.read::<i16>(at));
        let k4096: f64 = mem.read(mem.rebase(K_4096TH));
        let mut eax = verts;
        let mut e = src.wrapping_add(0x30);
        let mut o = out.wrapping_add(0x20);
        for _ in 0..n.max(0) {
            for w in [0x14u32, 0x18, 0x1c] {
                mem.write(o.wrapping_add(w + 0x2c), mem.read::<u32>(e.wrapping_add(w)));
            }
            for w in [0u32, 4, 8] {
                mem.write(o.wrapping_add(w), mem.read::<u32>(e.wrapping_add(w)));
            }
            // the vertices' i16s are exact as f32s
            let vertex = |mem: &M, at: u32| {
                let v = verts.wrapping_add(
                    (i32::from(mem.read::<i16>(e.wrapping_add(at))) * 6).cast_unsigned(),
                );
                [
                    half(mem, v),
                    half(mem, v.wrapping_add(2)),
                    half(mem, v.wrapping_add(4)),
                ]
            };
            let (a, b, c) = (vertex(mem, 0xc), vertex(mem, 0xe), vertex(mem, 0x10));
            let m: [u32; 3] = std::array::from_fn(|i| store(div(add(add(b[i], a[i]), c[i]), 3.0)));
            for (i, &v) in (0u32..).zip(&m) {
                mem.write(o.wrapping_add(0x50 + 4 * i), v);
            }
            for (base, p) in [(0x10u32, a), (0x20, b), (0x30, c)] {
                for (i, &v) in (0u32..).zip(&m) {
                    let d = store(sub(p[i as usize], load(v)));
                    mem.write(o.wrapping_add(base + 4 * i), d);
                }
            }
            let mut last = 0u32;
            for (to, from) in [
                (0x60u32, 6u32),
                (0x64, 8),
                (0x68, 0xa),
                (0x70, 0x12),
                (0x74, 0x14),
                (0x78, 0x16),
                (0x90, 0x1e),
                (0x94, 0x20),
                (0x98, 0x22),
                (0xa0, 0x2a),
                (0xa4, 0x2c),
                (0xa8, 0x2e),
            ] {
                if to == 0x90 {
                    for z in [0x88u32, 0x84, 0x80] {
                        mem.write(o.wrapping_add(z), 0f32);
                    }
                }
                let base = half(mem, rec.wrapping_add(from));
                #[allow(clippy::cast_possible_truncation)] // exact: an i16
                let range = half(mem, rec.wrapping_add(from + 6)) as f32;
                // op_random leaves its result in its argument's slot
                let r = crate::rand::bridge::op_random(mem, range);
                last = r.to_bits();
                let r = f64::from(r);
                mem.write(o.wrapping_add(to), store(add(r, base)));
            }
            mem.write(
                o.wrapping_add(0xb0),
                i32::from(mem.read::<i16>(rec.wrapping_add(0x36))),
            );
            let per = i32::from(mem.read::<i16>(rec.wrapping_add(0x3a))).cast_unsigned();
            let r = crate::rand::bridge::op_random_per(mem, [per, last, junk[0], junk[1]]);
            let plus = i32::from(mem.read::<i16>(rec.wrapping_add(0x38))).cast_unsigned();
            mem.write(o.wrapping_add(0xb4), r.wrapping_add(plus));
            let kind = |mem: &M| mem.read::<u16>(out.wrapping_add(0x12));
            if kind(mem) == 1 {
                let len = frame.wrapping_sub(0x2c);
                let f = mem.rebase(OP_DEG_GET);
                let deg = fcall(mem, f, &[0, 0, m[0], m[2], len]);
                mem.write(o.wrapping_add(0x50), mem.read::<u32>(len));
                mem.write(o.wrapping_add(0x58), deg);
            }
            let s = store(mul(half(mem, rec.wrapping_add(4)), k4096));
            let scaled: &[u32] = match kind(mem) {
                0 => &[0x60, 0x64, 0x68, 0x70, 0x74, 0x78],
                1 => &[0x60, 0x64, 0x70, 0x74],
                _ => &[],
            };
            for &at in scaled {
                let v = load(mem.read(o.wrapping_add(at)));
                mem.write(o.wrapping_add(at), store(mul(v, load(s))));
            }
            eax = i32::from(mem.read::<i16>(rec.wrapping_add(4))).cast_unsigned();
            e = e.wrapping_add(0x20);
            o = o.wrapping_add(0x100);
        }
        eax
    }
}
