//! The field's particle effects ("FP", `pppFp*` and `Fp_*`).
//!
//! `g_FpData` points at the current field's FP table: a `u16` count at
//! `+4`, and 0x50-byte entries whose `u16` ID is at `+0x64` (entry `i`'s
//! at `+0x64 + 0x50 i`). Each entry has two runtime records:
//! - `g_FpRun` (0x80 bytes each): the frame counter at `+0` (`-0x1000`
//!   while not started) and the status byte at `+0x1d` (0 running, 1
//!   ended, 2 restarted in mode 2);
//! - `g_FpDisp` (0x4c bytes each): flags at `+0` (0x10 hidden, 2 stuck to
//!   a map object, 8 stuck to a display object, the low four bits the
//!   sticking), the stick target at `+4` and a word at `+8` that sticking
//!   clears.
//!
//! The functions taking an `i16` ID change every entry with that ID; the
//! ones taking a full word compare it with the zero-extended ID and change
//! the first match.

/// `pppFpSaveStat`'s packing: two bits per entry with a nonzero ID, four
/// entries a byte from bit 0 (bit 0 running, bit 1 hidden). The byte after
/// a full one is started (cleared) even when no entry follows; `out`
/// receives every byte written.
pub fn save_stat(entries: impl IntoIterator<Item = (bool, bool)>, out: &mut Vec<u8>) {
    out.clear();
    out.push(0);
    let mut bit = 0;
    for (running, hidden) in entries {
        let last = out.len() - 1;
        if running {
            out[last] |= 1 << bit;
        }
        if hidden {
            out[last] |= 1 << (bit + 1);
        }
        bit += 2;
        if bit > 7 {
            bit = 0;
            out.push(0);
        }
    }
}

/// The FP tables where the game keeps them.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    /// `g_FpData`: the current field's FP table (0 when none).
    pub const DATA: Global<u32> = Global::new(0x00d2_c258);
    /// `g_FpRun`: 0x80-byte run records.
    pub const RUN: Global<[u8; 0x80]> = Global::new(0x00d2_c260);
    /// `g_FpDisp`: 0x4c-byte display records.
    pub const DISP: Global<[u8; 0x4c]> = Global::new(0x00d3_0260);
    /// `g_pppStopStatus` (see [`crate::ppp::bridge::STOP`]).
    pub const STOP: Global<u32> = crate::ppp::bridge::STOP;
    /// `g_FpPointLightA`: cleared by `pppFpPointLightInit` and `pppFpLoop`,
    /// read by `pppFpPointLightProc`.
    pub const POINT_LIGHT_A: Global<u32> = Global::new(0x00ce_c1f4);
    /// `g_FpPointLightB`: likewise.
    pub const POINT_LIGHT_B: Global<u32> = Global::new(0x00ce_c1fc);

    const NOT_STARTED: u32 = 0xffff_f000;

    /// The table's entry count and the address of entry 0's ID.
    fn table(mem: &impl Mem) -> Option<(u32, u32)> {
        let t = DATA.get(mem);
        if t == 0 {
            return None;
        }
        Some((u32::from(mem.read::<u16>(t + 4)), t + 0x64))
    }

    fn id(mem: &impl Mem, ids: u32, i: u32) -> u16 {
        mem.read(ids.wrapping_add(0x50 * i))
    }

    fn run(mem: &impl Mem, i: u32) -> u32 {
        RUN.ptr(mem).at(i).addr
    }

    fn disp(mem: &impl Mem, i: u32) -> u32 {
        DISP.ptr(mem).at(i).addr
    }

    /// Every entry whose ID is the low 16 bits of `id16`.
    fn matching(mem: &impl Mem, id16: u32) -> Vec<u32> {
        let Some((n, ids)) = table(mem) else {
            return Vec::new();
        };
        #[allow(clippy::cast_possible_truncation)] // the original compares 16 bits
        let want = id16 as u16;
        (0..n).filter(|&i| id(mem, ids, i) == want).collect()
    }

    fn first(mem: &impl Mem, id32: u32) -> Option<u32> {
        let (n, ids) = table(mem)?;
        (0..n).find(|&i| u32::from(id(mem, ids, i)) == id32)
    }

    fn or_flags(mem: &mut impl Mem, at: u32, set: u32, keep: u32) {
        let p = Ptr::<u32>::new(at);
        let v = (p.get(mem) & keep) | set;
        p.set(mem, v);
    }

    fn restart(mem: &mut impl Mem, id: u32, status: u8) {
        for i in matching(mem, id) {
            let r = run(mem, i);
            mem.write(r, 0u32);
            mem.write(r + 0x1d, status);
        }
    }

    /// `Fp_Restart(id)` (`0x00729700`): counter 0, status running.
    pub fn fp_restart(mem: &mut impl Mem, id: u32) {
        restart(mem, id, 0);
    }

    /// `Fp_RestartMode2(id)` (`0x00729810`): counter 0, status 2.
    pub fn fp_restart_mode2(mem: &mut impl Mem, id: u32) {
        restart(mem, id, 2);
    }

    /// Every entry whose `u16` at `+0x66` (after the ID) is the low 16
    /// bits of `group`.
    fn in_group(mem: &impl Mem, group: u32) -> Vec<u32> {
        let Some((n, ids)) = table(mem) else {
            return Vec::new();
        };
        #[allow(clippy::cast_possible_truncation)] // the original compares 16 bits
        let want = group as u16;
        (0..n)
            .filter(|&i| mem.read::<u16>(ids.wrapping_add(2 + 0x50 * i)) == want)
            .collect()
    }

    /// `Fp_EndGroup(group)` (`0x007294f0`): as `Fp_End` for every entry
    /// of the group (`+0x66`).
    pub fn fp_end_group(mem: &mut impl Mem, group: u32) {
        for i in in_group(mem, group) {
            let r = run(mem, i);
            mem.write(r + 0x1d, 1u8);
        }
    }

    /// `Fp_RestartGroup(group)` (`0x00729530`): as `Fp_Restart` for every
    /// entry of the group.
    pub fn fp_restart_group(mem: &mut impl Mem, group: u32) {
        for i in in_group(mem, group) {
            let r = run(mem, i);
            mem.write(r, 0u32);
            mem.write(r + 0x1d, 0u8);
        }
    }

    /// `Fp_End(id)` (`0x00729660`): status ended.
    pub fn fp_end(mem: &mut impl Mem, id: u32) {
        for i in matching(mem, id) {
            let r = run(mem, i);
            mem.write(r + 0x1d, 1u8);
        }
    }

    /// `pppFpIDdispOff(id)` (`0x00729570`): hidden.
    pub fn ppp_fp_id_disp_off(mem: &mut impl Mem, id: u32) {
        for i in matching(mem, id) {
            let d = disp(mem, i);
            or_flags(mem, d, 0x10, !0);
        }
    }

    /// `pppFpIDdispOn(id)` (`0x007295b0`): shown.
    pub fn ppp_fp_id_disp_on(mem: &mut impl Mem, id: u32) {
        for i in matching(mem, id) {
            let d = disp(mem, i);
            or_flags(mem, d, 0, !0x10);
        }
    }

    /// `pppFpGetMngPtr(id, &run, &disp)` (`0x007292f0`): the first entry's
    /// records, each output cleared first and set when found (outputs may
    /// be null).
    pub fn ppp_fp_get_mng_ptr(mem: &mut impl Mem, id: u32, run_out: u32, disp_out: u32) {
        if run_out != 0 {
            mem.write(run_out, 0u32);
        }
        if disp_out != 0 {
            mem.write(disp_out, 0u32);
        }
        let Some(i) = first(mem, id) else { return };
        if run_out != 0 {
            let r = run(mem, i);
            mem.write(run_out, r);
        }
        if disp_out != 0 {
            let d = disp(mem, i);
            mem.write(disp_out, d);
        }
    }

    fn stick(mem: &mut impl Mem, id: u32, set: u32, keep: u32, target: u32) {
        let Some(i) = first(mem, id) else { return };
        let d = disp(mem, i);
        or_flags(mem, d, set, keep);
        mem.write(d + 4, target);
        mem.write(d + 8, 0u32);
    }

    /// `pppFpSetStickChr(id, chr, bone)` (`0x0072a530`): when
    /// `Ch_CheckHandle(chr)` (through `call`) is not -1, the first entry
    /// with that ID sticks to the character: display flag 1, `+4` = `chr`,
    /// `+8` = `bone`. Returns what the original leaves in `eax`: -1, the
    /// check's result without a table, the last ID compared, or `bone`.
    pub fn ppp_fp_set_stick_chr<M: Mem>(
        mem: &mut M,
        id: u32,
        chr: u32,
        bone: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = Global::<u32>::new(0x0082_61b0).ptr(mem).addr;
        let r = call(mem, f, &[chr]);
        if r == u32::MAX {
            return r;
        }
        let Some((n, ids)) = table(mem) else { return r };
        let mut last = r;
        for i in 0..n {
            last = u32::from(self::id(mem, ids, i));
            if last == id {
                let d = disp(mem, i);
                or_flags(mem, d, 1, !0);
                mem.write(d + 4, chr);
                mem.write(d + 8, bone);
                return bone;
            }
        }
        last
    }

    /// `pppFpSetStickPos(id, pos)` (`0x0072a630`): the first entry with
    /// that ID gets an identity matrix at display `+0xc`
    /// (`FFXVu0UnitMatrix`, through `call`) and the point `pos.xyz` at
    /// `+0x3c`.
    pub fn ppp_fp_set_stick_pos<M: Mem>(
        mem: &mut M,
        id: u32,
        pos: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let Some(i) = first(mem, id) else { return };
        let d = disp(mem, i);
        let f = Global::<u32>::new(0x006e_d9a0).ptr(mem).addr;
        call(mem, f, &[d + 0xc]);
        for k in 0..3 {
            let x: u32 = mem.read(pos.wrapping_add(4 * k));
            mem.write(d + 0x3c + 4 * k, crate::x87::load(x));
        }
    }

    /// `pppFpResetStick(id)` (`0x0072a120`): no sticking.
    pub fn ppp_fp_reset_stick(mem: &mut impl Mem, id: u32) {
        stick(mem, id, 0, !0xf, 0);
    }

    /// `pppFpSetStickMap(id, target)` (`0x0072a590`).
    pub fn ppp_fp_set_stick_map(mem: &mut impl Mem, id: u32, target: u32) {
        stick(mem, id, 2, !0, target);
    }

    /// `pppFpSetStickMapDObj(id, target)` (`0x0072a5e0`).
    pub fn ppp_fp_set_stick_map_dobj(mem: &mut impl Mem, id: u32, target: u32) {
        stick(mem, id, 8, !0, target);
    }

    /// `pppFpStopStatus(stop)` (`0x0072a750`).
    pub fn ppp_fp_stop_status(mem: &mut impl Mem, stop: u32) {
        STOP.set(mem, stop);
    }

    /// `pppFpPointLightInit()` (`0x00729f70`).
    pub fn ppp_fp_point_light_init(mem: &mut impl Mem) {
        POINT_LIGHT_A.set(mem, 0);
        POINT_LIGHT_B.set(mem, 0);
    }

    /// `pppFpSaveStat(out)` (`0x0072a180`): every entry's state as two
    /// bits ([`super::save_stat`]); running is the first entry with the
    /// same ID started and not ended.
    pub fn ppp_fp_save_stat(mem: &mut impl Mem, out: u32) {
        if out == 0 {
            return;
        }
        let Some((n, ids)) = table(mem) else { return };
        let mut entries = Vec::new();
        for i in 0..n {
            let me = id(mem, ids, i);
            if me == 0 {
                continue;
            }
            let running = (0..n).find(|&j| id(mem, ids, j) == me).is_some_and(|j| {
                let r = run(mem, j);
                mem.read::<u32>(r) != NOT_STARTED && mem.read::<u8>(r + 0x1d) == 0
            });
            let hidden = mem.read::<u32>(disp(mem, i)) & 0x10 != 0;
            entries.push((running, hidden));
        }
        let mut bytes = Vec::new();
        super::save_stat(entries, &mut bytes);
        mem.write_bytes(out, &bytes);
    }

    /// `g_pppFpDisp`: the field effects' display switch (`pppFpSetDisp`,
    /// the field effect debug listing reads it).
    pub const FP_DISP: Global<u32> = Global::new(0x00ce_c1f0);

    /// `pppFpSetDisp(on)` (`0x00729050`): returns `on` (left in `eax`).
    pub fn ppp_fp_set_disp(mem: &mut impl Mem, on: u32) -> u32 {
        FP_DISP.set(mem, on);
        on
    }
    /// The field effects' packet and heap statistics `pppFpLoop` keeps:
    /// the packet budget (`0x00cec200`), the last frame's packet use
    /// (`0x00cec204`) and its peak (`0x00cec208`), the heap use
    /// (`0x00cec20c`) and its peak (`0x00cec210`).
    pub const STATS: [u32; 5] = [
        0x00ce_c200,
        0x00ce_c204,
        0x00ce_c208,
        0x00ce_c20c,
        0x00ce_c210,
    ];

    /// The getters of [`STATS`]: `pppFpGetPacketSize` (`0x007293e0`),
    /// `pppFpGetPacketUsed` (`0x007293f0`), `pppFpGetPacketPeak`
    /// (`0x007293d0`), `pppFpGetHeapUsed` (`0x007292d0`),
    /// `pppFpGetHeapMaxUseSize` (`0x00729280`).
    pub fn ppp_fp_stat(mem: &impl Mem, i: usize) -> u32 {
        Global::<u32>::new(STATS[i]).get(mem)
    }

    /// `pppFpGroupDispOff(group)` (`0x00729400`): `pppFpIDdispOff` for
    /// every entry of the group (`+0x66`).
    pub fn ppp_fp_group_disp_off(mem: &mut impl Mem, group: u32) {
        for i in in_group(mem, group) {
            let d = disp(mem, i);
            or_flags(mem, d, 0x10, !0);
        }
    }

    /// `pppFpGroupDispOn(group)` (`0x00729440`).
    pub fn ppp_fp_group_disp_on(mem: &mut impl Mem, group: u32) {
        for i in in_group(mem, group) {
            let d = disp(mem, i);
            or_flags(mem, d, 0, !0x10);
        }
    }

    /// `0x0072a760(id)`: the run state of the `g_FpData` entry whose `i16
    /// +0x64` is `id` (the first): 0 not started (`g_FpRun` `+0` is
    /// `-0x1000`), 1 running, 2 ended (`+0x1d` set); -1 without a table or
    /// entry.
    pub fn fp_id_state<M: Mem>(mem: &M, id: u16) -> u32 {
        let data = DATA.get(mem);
        if data == 0 {
            return u32::MAX;
        }
        let mut run = RUN.ptr(mem).addr;
        let mut e = data.wrapping_add(0x64);
        let mut left = u32::from(mem.read::<u16>(data.wrapping_add(4)));
        while left != 0 {
            left -= 1;
            if mem.read::<u16>(e) == id {
                if mem.read::<u32>(run) == 0xffff_f000 {
                    return 0;
                }
                return 1 + u32::from(mem.read::<u8>(run.wrapping_add(0x1d)) != 0);
            }
            e = e.wrapping_add(0x50);
            run = run.wrapping_add(0x80);
        }
        u32::MAX
    }

    /// `g_pppCameraMatrix` (`0x0230ff60`).
    const CAMERA_MATRIX: u32 = 0x0230_ff60;
    /// `g_pppWorldMatrix` (`0x0230ffa0`), its translation at `+0x30`.
    const WORLD_MATRIX: u32 = 0x0230_ffa0;
    /// `g_pppFpWoodMatrix`: the billboard rotation `pppFpSetWoodMatrix`
    /// builds from the camera; `pppFpSetDrawMatrix` mode 2 views through it.
    const FP_WOOD_MATRIX: u32 = 0x0230_fea0;
    /// `g_pppViewMatrix`.
    const VIEW_MATRIX: u32 = 0x0230_ff20;
    /// `g_pppViewMatrixCopy`.
    const VIEW_MATRIX_COPY: u32 = 0x0230_fe20;

    /// `pppFpSetDrawMatrix(mode)` (`0x0072a260`): `g_pppViewMatrix` for a
    /// field effect's draws (the mode's low byte): 1 keeps
    /// `g_pppWorldMatrix`'s rotation with its position moved by
    /// `g_pppCameraMatrix`; 2 is `g_pppWorldMatrix x g_pppFpWoodMatrix`;
    /// anything else `g_pppWorldMatrix x g_pppCameraMatrix`. Then copied to
    /// `g_pppViewMatrixCopy`, which it returns (`FFXVu0CopyMatrix`'s eax).
    pub fn ppp_fp_set_draw_matrix(mem: &mut impl Mem, mode: u32) -> u32 {
        use ffx_ps2::vu0::bridge as vu0;
        use ffx_system::math::bridge as math;
        fn r(mem: &impl Mem, a: u32) -> u32 {
            Global::<u32>::new(a).ptr(mem).addr
        }
        let (view, world, camera) = (
            r(mem, VIEW_MATRIX),
            r(mem, WORLD_MATRIX),
            r(mem, CAMERA_MATRIX),
        );
        match mode & 0xff {
            1 => {
                vu0::matrix_copy(mem, view, world);
                vu0::apply_matrix(mem, view + 0x30, camera, world + 0x30);
            }
            2 => math::mul4x4(mem, view, r(mem, FP_WOOD_MATRIX), world),
            _ => math::mul4x4(mem, view, camera, world),
        }
        vu0::matrix_copy(mem, r(mem, VIEW_MATRIX_COPY), view)
    }

    /// `g_pppFpHeapDisplaySet` (`0x00cec218`): set by
    /// `pppFpInitHeapDisplay` and [`ppp_set_fp_pdt`]; `pppPartLoop` runs
    /// `pppFpInitHeapDisplay` while it is 0.
    pub const HEAP_DISPLAY_SET: Global<u32> = Global::new(0x00ce_c218);
    /// `g_pppFpPacketBuf` (`0x00cec220`): the field effects' packet buffer
    /// (`pppFpLoop` starts `g_PppPacketPos` there), allocated once.
    pub const PACKET_BUF: Global<u32> = Global::new(0x00ce_c220);
    /// `g_pppFpHeap2Buf` (`0x00cec224`): the second heap's block, allocated
    /// once.
    pub const HEAP2_BUF: Global<u32> = Global::new(0x00ce_c224);
    /// `g_pppFpWide` (`0x00cec228`): set for a table of 0x28 effects or the
    /// level whose name hash is 0x234; 0x00756740 then uses 1.1 for some
    /// of its 1.0 bounds.
    pub const WIDE: Global<u32> = Global::new(0x00ce_c228);
    /// `g_pppFpEnv` (`0x00cec258`): the field effects' environment
    /// `pppInitEnv` fills.
    pub const ENV: Global<u32> = Global::new(0x00ce_c258);
    /// `g_pppFpWood` (`0x00d2c25c`): some effect has kind byte 2, so
    /// `pppFpLoop` sets up the wood matrix (`pppFpSetWoodMatrix`).
    pub const WOOD: Global<u32> = Global::new(0x00d2_c25c);

    /// `pppSetFpPdt(data)` (`0x0072a7f0`): installs a field's effect table:
    /// [`FP_DISP`] on, [`DATA`] = `data`, `pppInitData(data,
    /// g_pppProgramTable, 0)`, `pppInitEnv(g_pppFpHeap, data, ENV,
    /// 0x40000)`. The first time a table asks for a second heap (`byte
    /// +1` blocks of 8 KB) it is allocated (`user_malloc`) and made
    /// (`pppCreateHeap(g_pppFpHeap2, ..)`). The heap use and peak are reset,
    /// the heap display redone (`op_heap_display_clear(0x601)`,
    /// `op_heap_display_set_par(g_pppFpHeap, 0x8603)`), [`HEAP_DISPLAY_SET`]
    /// set. Each of the `u16 +4` effects (0x50 bytes from `+0x20`) gets
    /// its run record: `+0` its `+0x30`, `+0x20..+0x28` its floats
    /// `+0x38..+0x40`, `+0x30` a fresh `set_ptr32(0)`, `+0x3c` 0,
    /// `+0x40..+0x6c` its `+0..+0x2c`; and its display record: three zero
    /// words and a unit matrix (`FFXVu0UnitMatrix`). An effect with kind
    /// byte `+0x4a` 2 sets [`WOOD`]. [`WIDE`] for 0x28 effects or level
    /// hash 0x234 (`graphicLevelGetNameHash`, not asked for 0x28). The
    /// packet buffer is allocated once: 0xc0000 bytes, 0x100000 for `byte
    /// +2` 1, 0x140000 for 2; its use and peak reset. Callees through
    /// `call`.
    pub fn ppp_set_fp_pdt<M: Mem>(
        mem: &mut M,
        data: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        fn r(mem: &impl Mem, a: u32) -> u32 {
            Global::<u32>::new(a).ptr(mem).addr
        }
        let stat = |i: usize| Global::<u32>::new(STATS[i]);
        FP_DISP.set(mem, 1);
        DATA.set(mem, data);
        let init_data = r(mem, 0x0071_1fa0);
        let programs = r(mem, 0x00c3_a508);
        call(mem, init_data, &[data, programs, 0]);
        let heap = crate::heap::bridge::FP_HEAP.ptr(mem).addr;
        let init_env = r(mem, 0x0071_69d0);
        let (table, env) = (DATA.get(mem), ENV.ptr(mem).addr);
        call(mem, init_env, &[heap, table, env, 0x4_0000]);
        let malloc = r(mem, 0x0068_7020);
        if HEAP2_BUF.get(mem) == 0 {
            let blocks = mem.read::<u8>(DATA.get(mem).wrapping_add(1));
            if blocks != 0 {
                let size = u32::from(blocks) << 13;
                let buf = call(mem, malloc, &[size]);
                HEAP2_BUF.set(mem, buf);
                let heap2 = crate::heap::bridge::FP_HEAP2.ptr(mem).addr;
                let create = r(mem, 0x0072_c480);
                call(mem, create, &[heap2, buf, size]);
            }
        }
        stat(4).set(mem, 0);
        stat(3).set(mem, 0);
        crate::op::bridge::op_heap_display_clear(mem, 0x601);
        crate::op::bridge::op_heap_display_set_par(mem, heap, 0x8603);
        HEAP_DISPLAY_SET.set(mem, 1);
        WOOD.set(mem, 0);
        let set_ptr32 = r(mem, 0x00a4_4700);
        let unit_matrix = r(mem, 0x006e_d9a0);
        let mut k = 0u32;
        while k < u32::from(mem.read::<u16>(DATA.get(mem).wrapping_add(4))) {
            // the table is read before the calls, as the original keeps it
            let fx = DATA.get(mem).wrapping_add(0x20 + 0x50 * k);
            let run = RUN.ptr(mem).at(k).addr;
            let disp = DISP.ptr(mem).at(k).addr;
            let ptr = call(mem, set_ptr32, &[0]);
            mem.write(run.wrapping_add(0x30), ptr);
            mem.write(run.wrapping_add(0x3c), 0u32);
            let words: [u32; 12] = mem.read(fx);
            mem.write(run.wrapping_add(0x40), words);
            let first: u32 = mem.read(fx.wrapping_add(0x30));
            mem.write(run, first);
            for i in 0..3 {
                let v: u32 = mem.read(fx.wrapping_add(0x38 + 4 * i));
                mem.write(run.wrapping_add(0x20 + 4 * i), crate::x87::load(v));
            }
            mem.write(disp, [0u32; 3]);
            call(mem, unit_matrix, &[disp.wrapping_add(0xc)]);
            if mem.read::<u8>(fx.wrapping_add(0x4a)) == 2 {
                WOOD.set(mem, 1);
            }
            k += 1;
        }
        WIDE.set(mem, 0);
        let wide = mem.read::<u16>(DATA.get(mem).wrapping_add(4)) == 0x28 || {
            let hash = r(mem, 0x0064_1da0);
            call(mem, hash, &[]) == 0x234
        };
        if wide {
            WIDE.set(mem, 1);
        }
        if PACKET_BUF.get(mem) != 0 {
            return;
        }
        let size = match mem.read::<u8>(DATA.get(mem).wrapping_add(2)) {
            1 => 0x10_0000,
            2 => 0x14_0000,
            _ => 0xc_0000,
        };
        stat(0).set(mem, size);
        let buf = call(mem, malloc, &[size]);
        PACKET_BUF.set(mem, buf);
        stat(2).set(mem, 0);
        stat(1).set(mem, 0);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn packs_four_a_byte() {
        let mut out = Vec::new();
        super::save_stat(
            [(true, false), (false, true), (true, true), (false, false)],
            &mut out,
        );
        assert_eq!(out, [0b0011_1001, 0]);
        super::save_stat([], &mut out);
        assert_eq!(out, [0]);
    }
}
