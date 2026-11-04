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

    /// `pppFpPointLight(obj, key, prog)` (`0x0075d7f0`): while
    /// `g_FpPointLightA` (`i`) is below `g_FpPointLightCap`, defines light
    /// `i`: `Sg_PLightDefine(i, x, y, z, radius, r, g, b, light)` with the
    /// position work area 2 (`obj + 0xa0 + [[prog + 0xc] + 8]`) plus
    /// `g_pppWorldMatrix`'s translation, the radius `f32 key +4`, the
    /// colour work area 1's `i16 +0..+4` times its `i16 +6 >> 7`, `>> 14`
    /// each, and the light `byte key +8`; then
    /// `Sg_PLightSetAttenuation(i, a)` with `f32 key +0xc`, or
    /// `[0x00b92268]` (2.0) when that is 0. In any case `g_FpPointLightB`
    /// gets bit `light & 31` and `g_FpPointLightA` counts up (from the
    /// value read last). Callees through `call`. Returns the bit.
    #[allow(clippy::many_single_char_names)]
    pub fn ppp_fp_point_light<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{ADD, Reg, load};
        let sym = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let offs: u32 = mem.read(prog.wrapping_add(0xc));
        let area = |mem: &M, k: u32| {
            obj.wrapping_add(0xa0)
                .wrapping_add(mem.read(offs.wrapping_add(4 * k)))
        };
        let (colour, pos) = (area(mem, 1), area(mem, 2));
        let mut i = POINT_LIGHT_A.get(mem);
        if i.cast_signed() < POINT_LIGHT_CAP.get(mem) {
            let half = |mem: &M, o: u32| i32::from(mem.read::<i16>(colour.wrapping_add(o)));
            let s = half(mem, 6) >> 7;
            let [r, g, b] = [0, 2, 4].map(|o| (half(mem, o).wrapping_mul(s) >> 14).cast_unsigned());
            let world = crate::mat::bridge::WORLD.ptr(mem).addr;
            let [x, y, z] = [0u32, 4, 8].map(|o| {
                let w: f32 = mem.read(world.wrapping_add(0x30 + o));
                Reg::load(mem.read(pos.wrapping_add(o)))
                    .mem(ADD, w)
                    .store()
                    .to_bits()
            });
            let radius = load(mem.read(key.wrapping_add(4)));
            let light = u32::from(mem.read::<u8>(key.wrapping_add(8)));
            call(
                mem,
                sym(mem, 0x0083_19b0),
                &[i, x, y, z, radius, r, g, b, light],
            );
            let mut a = load(mem.read(key.wrapping_add(0xc)));
            if Reg::load(a).value() == 0.0 {
                a = load(mem.read(sym(mem, 0x00b9_2268)));
            }
            let n = POINT_LIGHT_A.get(mem);
            call(mem, sym(mem, 0x0083_1bf0), &[n, a]);
            i = POINT_LIGHT_A.get(mem);
        }
        let bit = 1u32 << (mem.read::<u8>(key.wrapping_add(8)) & 31);
        let b = POINT_LIGHT_B.get(mem);
        POINT_LIGHT_B.set(mem, b | bit);
        POINT_LIGHT_A.set(mem, i.wrapping_add(1));
        bit
    }

    /// `pppFpPointLightModel(obj, key, ctrl)` (`0x0075d930`) and, with
    /// `scaled`, `pppFpPointLightModelScl` (`0x0075dd70`): for a model
    /// (`key +0x10` = `n`, not 0xffff: `get_ptr32_func` of word `+0x1c` of
    /// entry `n` of the environment's 0x20-byte table at `env +0x20`), one
    /// light per model point (`i16 model +0x12` of them, 6-byte `i16` points
    /// from `model + model[+8]`), as [`Lights::point`]. Callees through
    /// `call`. Returns what the original leaves in `eax`: the bit, or
    /// without a model `key` (`Scl`: the scale area's table word).
    pub fn ppp_fp_point_light_model<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        scaled: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let offs: u32 = mem.read(ctrl.wrapping_add(0xc));
        let n: u32 = mem.read(key.wrapping_add(0x10));
        if n == 0xffff {
            return if scaled {
                mem.read(offs.wrapping_add(0xc))
            } else {
                key
            };
        }
        let env: u32 = mem.read(crate::heap::bridge::ENV_SLOT.get(mem));
        let models: u32 = mem.read(env.wrapping_add(0x20));
        let model =
            ffx_system::stub::ptr32(mem.read(models.wrapping_add(n << 5).wrapping_add(0x1c)));
        let lights = Lights::new(mem, obj, key, offs, scaled);
        let mut p = model.wrapping_add(mem.read(model.wrapping_add(8)));
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(model.wrapping_add(0x12))) {
            lights.point(mem, p, call);
            p = p.wrapping_add(6);
            i += 1;
        }
        lights.done(mem)
    }

    /// `pppFpPointLightVsf(obj, key, ctrl)` (`0x0075db30`) and, with
    /// `scaled`, `pppFpPointLightVsfScl` (`0x0075df90`): as
    /// [`ppp_fp_point_light_model`] over a vertex selection: entry `i16 key
    /// +0x10` of the environment's 8-byte table at `env +0x28` (`i16 +0`
    /// the model, `i16 +2` the count, `+4` the `u16` vertex numbers),
    /// the vertices (6-byte `i16`) at `get_ptr32_func(obj +0x90)`, or when
    /// that is 0 at `get_ptr32_func` of word `+0x14` of the model's entry in
    /// the table at `env +0x20`. Callees through `call`. Returns what the
    /// original leaves in `eax`: the bit, or the table address when the
    /// entry's address is 0.
    pub fn ppp_fp_point_light_vsf<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        scaled: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_system::stub::ptr32;
        let offs: u32 = mem.read(ctrl.wrapping_add(0xc));
        let env = |mem: &M| mem.read::<u32>(crate::heap::bridge::ENV_SLOT.get(mem));
        let table: u32 = mem.read(env(mem).wrapping_add(0x28));
        let n = i32::from(mem.read::<i16>(key.wrapping_add(0x10))).cast_unsigned();
        let sel = table.wrapping_add(n << 3);
        if sel == 0 {
            return table;
        }
        let mut verts = ptr32(mem.read(obj.wrapping_add(0x90)));
        if verts == 0 {
            let models: u32 = mem.read(env(mem).wrapping_add(0x20));
            let m = i32::from(mem.read::<i16>(sel)).cast_unsigned();
            verts = ptr32(mem.read(models.wrapping_add(m << 5).wrapping_add(0x14)));
        }
        let lights = Lights::new(mem, obj, key, offs, scaled);
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(sel.wrapping_add(2))) {
            let list = ptr32(mem.read(sel.wrapping_add(4)));
            let at = list.wrapping_add(2 * i.cast_unsigned());
            let k = u32::from(mem.read::<u16>(at));
            lights.point(mem, verts.wrapping_add(k * 6), call);
            i += 1;
        }
        lights.done(mem)
    }

    /// What the four model point light steps share: the position (work
    /// area 2 of `ctrl`'s offsets plus `g_pppWorldMatrix`'s translation,
    /// each `f32`), the colour (work area 1's `i16 +0..+4` times its `i16
    /// +6 >> 7`, `>> 14` each), the attenuation (`f32 key +0xc`, or
    /// `[0x00b92268]` (2.0) when that is 0) and, for the `Scl` steps, the
    /// scale area (word 3).
    struct Lights {
        key: u32,
        pos: [f64; 3],
        rgb: [u32; 3],
        att: u32,
        scale: Option<u32>,
    }

    impl Lights {
        fn new(mem: &impl Mem, obj: u32, key: u32, offs: u32, scaled: bool) -> Self {
            use ffx_bridge::x87::{add, load, store};
            let area = |k: u32| {
                obj.wrapping_add(0xa0)
                    .wrapping_add(mem.read(offs.wrapping_add(4 * k)))
            };
            let (colour, pos) = (area(1), area(2));
            let half = |o: u32| i32::from(mem.read::<i16>(colour.wrapping_add(o)));
            let s = half(6) >> 7;
            let world = crate::mat::bridge::WORLD.ptr(mem).addr;
            let pos = [0u32, 4, 8].map(|o| {
                let w = load(mem.read(world.wrapping_add(0x30 + o)));
                load(store(add(load(mem.read(pos.wrapping_add(o))), w)))
            });
            let rgb = [0, 2, 4].map(|o| (half(o).wrapping_mul(s) >> 14).cast_unsigned());
            let mut att: u32 = mem.read(key.wrapping_add(0xc));
            if load(att) == 0.0 {
                att = mem.read(Global::<u32>::new(0x00b9_2268).ptr(mem).addr);
            }
            let scale = scaled.then(|| area(3));
            Self {
                key,
                pos,
                rgb,
                att,
                scale,
            }
        }

        /// One light at the `i16` point `at` (`Scl`: times the scale area's
        /// floats), each coordinate `f32`: while `g_FpPointLightA` is below
        /// the cap, `Sg_PLightDefine(A, x, y, z, radius f32 key +4, r, g, b,
        /// light u8 key +8)` and `Sg_PLightSetAttenuation(A, att)`; then
        /// `g_FpPointLightA` counts up.
        #[allow(clippy::many_single_char_names)]
        fn point<M: Mem>(
            &self,
            mem: &mut M,
            at: u32,
            call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        ) {
            use ffx_bridge::x87::{add, load, mul, store};
            let sym = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
            let mut xyz = [0u32; 3];
            for ((o, base), out) in (0u32..).step_by(2).zip(self.pos).zip(&mut xyz) {
                let v = f64::from(mem.read::<i16>(at.wrapping_add(o)));
                let d = match self.scale {
                    Some(s) => mul(v, load(mem.read(s.wrapping_add(2 * o)))),
                    None => v,
                };
                *out = store(add(base, d));
            }
            let lit = POINT_LIGHT_A.get(mem);
            if lit.cast_signed() < POINT_LIGHT_CAP.get(mem) {
                let light = u32::from(mem.read::<u8>(self.key.wrapping_add(8)));
                let radius: u32 = mem.read(self.key.wrapping_add(4));
                let [x, y, z] = xyz;
                let [r, g, b] = self.rgb;
                let args = [lit, x, y, z, radius, r, g, b, light];
                call(mem, sym(mem, 0x0083_19b0), &args);
                let at = POINT_LIGHT_A.get(mem);
                call(mem, sym(mem, 0x0083_1bf0), &[at, self.att]);
            }
            let next = POINT_LIGHT_A.get(mem).wrapping_add(1);
            POINT_LIGHT_A.set(mem, next);
        }

        /// `g_FpPointLightB` gets bit `light & 31`; returns the bit.
        fn done(&self, mem: &mut impl Mem) -> u32 {
            let bit = 1u32 << (mem.read::<u8>(self.key.wrapping_add(8)) & 31);
            let old = POINT_LIGHT_B.get(mem);
            POINT_LIGHT_B.set(mem, old | bit);
            bit
        }
    }

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

    /// `pppFpPointLightProc()` (`0x00729f90`): the scene's point lights
    /// follow [`POINT_LIGHT_A`] (how many the field effects want). More
    /// than [`POINT_LIGHT_CAP`]: the defined ones are saved (position,
    /// radius, colour, light, attenuation through `Sg_PLightGet*`, 0x24
    /// bytes each in a `user_malloc` block), disposed of
    /// (`Sg_PLightDispose_thunk`), the new number allocated
    /// (`Sg_PLightAllocate`) and taken as the cap, the saved ones defined
    /// again (`Sg_PLightDefine`, `Sg_PLightSetAttenuation`) and the rest
    /// switched off (`Sg_PLightSetEnable(i, 0)`), the block freed. Fewer:
    /// those past the wanted number switched off. Then with any defined,
    /// every character (`Ch_HandleRewind`, `Ch_HandleGetNext`) gets
    /// `Ch_PLightSetEnable(h, POINT_LIGHT_B)`. Callees through `call`
    /// (`fcall` for those returning in ST0). Returns the original's eax:
    /// 0 after the character walk, else the cap.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // counts as the original's signed words
    pub fn ppp_fp_point_light_proc<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use crate::x87::load;
        use ffx_bridge::x87::store;
        fn r(mem: &impl Mem, a: u32) -> u32 {
            Global::<u32>::new(a).ptr(mem).addr
        }
        let cap = POINT_LIGHT_CAP.get(mem);
        let want = POINT_LIGHT_A.get(mem) as i32;
        if want > cap {
            let n = cap;
            let mut saved = 0u32;
            let mut count = want as u32;
            if n > 0 {
                saved = call(mem, r(mem, 0x0068_7020), &[(n as u32).wrapping_mul(0x24)]);
                for i in 0..n as u32 {
                    let rec = saved.wrapping_add(0x24 * i);
                    call(mem, r(mem, 0x0083_1b30), &[i, rec, rec + 4, rec + 8]);
                    let radius = fcall(mem, r(mem, 0x0083_1b80), &[i]);
                    mem.write(rec + 0xc, store(radius));
                    call(
                        mem,
                        r(mem, 0x0083_1a60),
                        &[i, rec + 0x10, rec + 0x14, rec + 0x18],
                    );
                    let light = call(mem, r(mem, 0x0083_1b00), &[i]);
                    mem.write(rec + 0x1c, light);
                    let att = fcall(mem, r(mem, 0x0083_1a40), &[i]);
                    mem.write(rec + 0x20, store(att));
                }
                call(mem, r(mem, 0x0083_1a30), &[]);
                count = POINT_LIGHT_A.get(mem);
            }
            call(mem, r(mem, 0x0083_1950), &[count]);
            let now = POINT_LIGHT_A.get(mem);
            POINT_LIGHT_CAP.set(mem, now as i32);
            let mut i = 0i32;
            if n > 0 {
                for k in 0..n as u32 {
                    let rec = saved.wrapping_add(0x24 * k);
                    let w: [u32; 9] = mem.read(rec);
                    let args = [
                        k,
                        load(w[0]),
                        load(w[1]),
                        load(w[2]),
                        load(w[3]),
                        w[4],
                        w[5],
                        w[6],
                        w[7],
                    ];
                    call(mem, r(mem, 0x0083_19b0), &args);
                    call(mem, r(mem, 0x0083_1bf0), &[k, load(w[8])]);
                }
                i = n;
            }
            while i < POINT_LIGHT_A.get(mem) as i32 {
                call(mem, r(mem, 0x0083_1c60), &[i as u32, 0]);
                i += 1;
            }
            if saved != 0 {
                call(mem, r(mem, 0x0068_7010), &[saved]);
            }
        } else if want < cap {
            let mut i = want;
            loop {
                call(mem, r(mem, 0x0083_1c60), &[i as u32, 0]);
                i += 1;
                if i >= POINT_LIGHT_CAP.get(mem) {
                    break;
                }
            }
        }
        let cap = POINT_LIGHT_CAP.get(mem);
        if cap <= 0 {
            return cap as u32;
        }
        call(mem, r(mem, 0x0082_6db0), &[]);
        loop {
            let h = call(mem, r(mem, 0x0082_6d70), &[]);
            if h == 0 {
                return 0;
            }
            let on = POINT_LIGHT_B.get(mem);
            call(mem, r(mem, 0x0083_0ae0), &[h, on]);
        }
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

    /// `g_pppFillMatrix2` (`0x0230fc20`): the view-projection matrix
    /// `UpdatePhyreViewProjectionMatrix` makes.
    pub const FILL_MATRIX2: Global<[f32; 16]> = Global::new(0x0230_fc20);
    /// `g_pppFillMatrix3` (`0x0230fbe0`): its inverse.
    pub const FILL_MATRIX3: Global<[f32; 16]> = Global::new(0x0230_fbe0);

    /// `UpdatePhyreViewProjectionMatrix()` (`0x00711910`): the battle
    /// camera's matrix (`MsCopyCameraMatrix`), inverted
    /// (`sceVu0InversMatrix4x4`), its x and y columns negated (`fchs`) and
    /// its w column (0, 0, 0, 1); then `g_pppFillMatrix2` = the renderer's
    /// view-projection (`graphicCameraGetViewProjectionMatrix`) times it
    /// (`FFXVu0MulMatrix(fill2, vp, m)`), and `g_pppFillMatrix3` its
    /// inverse. Callees through `call`; `local` is the original's two
    /// matrices (`vp`, then `m`; 0x80 bytes).
    pub fn update_phyre_view_projection_matrix<M: Mem>(
        mem: &mut M,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{load, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = Global::<u32>::new(at).ptr(mem).addr;
            call(mem, f, args)
        };
        let (vp, m) = (local, local.wrapping_add(0x40));
        go(mem, 0x007c_0070, &[m]);
        go(mem, 0x006e_d460, &[m, m]);
        for row in 0..4u32 {
            let at = m.wrapping_add(0x10 * row);
            for k in 0..2u32 {
                let x: u32 = mem.read(at.wrapping_add(4 * k));
                mem.write(at.wrapping_add(4 * k), store(-load(x)));
            }
            let w = if row == 3 { 1.0f32 } else { 0.0 };
            mem.write(at.wrapping_add(0xc), w);
        }
        go(mem, 0x0063_9590, &[vp]);
        let (fill2, fill3) = (FILL_MATRIX2.ptr(mem).addr, FILL_MATRIX3.ptr(mem).addr);
        go(mem, 0x006e_d5c0, &[fill2, vp, m]);
        go(mem, 0x006e_d460, &[fill3, fill2]);
    }

    /// `0x00728ac0` (`pppFpLoopMatrix`, ours; from `pppFpLoop`) `(part,
    /// set, def)`: `g_pppWorldMatrix` for a looped field effect. First
    /// `pppFpSetRotMatrix(part, def +0x4b)`. With none of `set`'s flag bits
    /// 0..3: bit 0x100 copies `set +0xc` into it, else
    /// `pppFpSetRotMatrix` again. Else its translation is cleared and the
    /// local matrix `local` (the original's, 16 words) starts as the unit
    /// matrix; with a parent `set +4`, bit 0 takes `Ch_ElemGetMatrix(chr,
    /// set +8, local)` and scales its three rows' `xyz` by 1000 (through
    /// the emulated registers: `0x00c8f830` ends with row 2 scaled,
    /// `+0xc` its `w`, `0x00c8f820` = 1000.0), else bit 1
    /// `yiGetMatrixGroup` and bit 3 `yiGetMatrixDObj`. Bit 0x20 normalises
    /// the three rows. Bit 0x40: world = `set +0xc` x `local`
    /// (`FFXVu0CopyMatrix`, `FFXVu0MulMatrix`) and `part +0x60..+0x68` its
    /// row lengths (`pppVu0Length`); else world = `set +0xc` x world, then
    /// `local` x it. Then `pppFpSetDrawMatrix(def +0x4a)`. Callees
    /// through `call` (`fcall` for the lengths).
    #[allow(clippy::many_single_char_names, clippy::verbose_bit_mask)] // test al, 0xf
    pub fn ppp_fp_loop_matrix<M: Mem>(
        mem: &mut M,
        part: u32,
        set: u32,
        def: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::Reg;
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let rot = u32::from(mem.read::<u8>(def.wrapping_add(0x4b)));
        go(mem, 0x0072_a2e0, &[part, rot]);
        let world = g(mem, 0x0230_ffa0);
        let flags = |mem: &M| mem.read::<u32>(set);
        let m = set.wrapping_add(0xc);
        if flags(mem) & 0xf == 0 {
            if flags(mem) & 0x100 != 0 {
                go(mem, 0x006e_d360, &[world, m]);
            } else {
                let rot = u32::from(mem.read::<u8>(def.wrapping_add(0x4b)));
                go(mem, 0x0072_a2e0, &[part, rot]);
            }
        } else {
            for k in [0x38, 0x34, 0x30] {
                mem.write(world.wrapping_add(k), 0u32);
            }
            go(mem, 0x006e_d9a0, &[local]);
            let parent: u32 = mem.read(set.wrapping_add(4));
            if parent != 0 {
                let fl = flags(mem);
                if fl & 1 != 0 {
                    let e: u32 = mem.read(set.wrapping_add(8));
                    go(mem, 0x0083_3c30, &[parent, e, local]);
                    let k = Reg::constant(1000.0);
                    let regs = g(mem, 0x00c8_f830);
                    for row in 0..3u32 {
                        let at = local.wrapping_add(0x10 * row);
                        let v: [f32; 3] = mem.read(at);
                        let s = v.map(|x| (Reg::load(x) * k).store());
                        mem.write(regs, s);
                        mem.write(at, s);
                    }
                    let w: u32 = mem.read(local.wrapping_add(0x2c));
                    mem.write(regs.wrapping_add(0xc), w);
                    mem.write(g(mem, 0x00c8_f820), 1000.0f32.to_bits());
                } else if fl & 2 != 0 {
                    go(mem, 0x0090_8e30, &[parent, local]);
                } else if fl & 8 != 0 {
                    go(mem, 0x0090_8df0, &[parent, local]);
                }
            }
            if mem.read::<u8>(set) & 0x20 != 0 {
                for row in 0..3u32 {
                    let r = local.wrapping_add(0x10 * row);
                    go(mem, 0x006e_d6f0, &[r, r]);
                }
            }
            if mem.read::<u8>(set) & 0x40 != 0 {
                go(mem, 0x006e_d360, &[world, m]);
                go(mem, 0x006e_d5c0, &[world, local, world]);
                let len = g(mem, 0x0072_ab20);
                for row in 0..3u32 {
                    let v = fcall(mem, len, &[world.wrapping_add(0x10 * row)]);
                    mem.write(part.wrapping_add(0x60 + 4 * row), ffx_bridge::x87::store(v));
                }
            } else {
                go(mem, 0x006e_d5c0, &[world, m, world]);
                go(mem, 0x006e_d5c0, &[world, local, world]);
            }
        }
        let draw = u32::from(mem.read::<u8>(def.wrapping_add(0x4a)));
        go(mem, 0x0072_a260, &[draw])
    }

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

    /// `pppFpSetWoodMatrix()` (`0x0072a690`): `g_pppFpWoodMatrix`, the
    /// camera-facing rotation the "wood" effects draw with
    /// ([`ppp_fp_set_draw_matrix`] mode 2), built in `frame` (0x30 bytes,
    /// the original's three local vectors): row 1 = the camera matrix's
    /// row 1 (`w` 0) normalized, row 0 = its (`y`, `-x`, 0, 0)
    /// normalized, row 2 = row 0 x row 1 normalized, row 3 = the camera's
    /// position row (`FFXVu0Normalize`, `FFXVu0CopyVector`,
    /// `FFXVu0OuterProduct`). Returns the last copy's eax (the position's
    /// `w`).
    pub fn ppp_fp_set_wood_matrix(mem: &mut impl Mem, frame: u32) -> u32 {
        use crate::x87::load;
        use ffx_ps2::vu0::bridge as vu0;
        fn r(mem: &impl Mem, a: u32) -> u32 {
            Global::<u32>::new(a).ptr(mem).addr
        }
        let (side, up, fwd) = (frame, frame + 0x10, frame + 0x20);
        let wood = r(mem, FP_WOOD_MATRIX);
        let cam: [u32; 3] = mem.read(r(mem, CAMERA_MATRIX) + 0x10);
        mem.write(up, [load(cam[0]), load(cam[1]), load(cam[2]), 0]);
        vu0::normalize(mem, up, up);
        vu0::copy_vector(mem, wood + 0x10, up);
        let [x, y]: [u32; 2] = mem.read(up);
        mem.write(side, [load(y), load(x) ^ 0x8000_0000, 0, 0]);
        vu0::normalize(mem, side, side);
        vu0::copy_vector(mem, wood, side);
        vu0::outer_product(mem, fwd, side, up);
        vu0::normalize(mem, fwd, fwd);
        vu0::copy_vector(mem, wood + 0x20, fwd);
        let pos = r(mem, CAMERA_MATRIX) + 0x30;
        vu0::copy_vector(mem, wood + 0x30, pos)
    }

    /// `g_pppFpHeapDisplaySet` (`0x00cec218`): set by
    /// `pppFpInitHeapDisplay` and [`ppp_set_fp_pdt`]; `pppPartLoop` runs
    /// `pppFpInitHeapDisplay` while it is 0.
    pub const HEAP_DISPLAY_SET: Global<u32> = Global::new(0x00ce_c218);
    /// `g_pppFpPacketBuf` (`0x00cec220`): the field effects' packet buffer
    /// (`pppFpLoop` starts `g_PppPacketPos` there), allocated by
    /// `pppSetFpPdt` when there is none (the first table, and the first
    /// after [`ppp_fp_free`], which frees it when a map is torn down and
    /// on `yiLoadFieldParticle`'s debug path).
    pub const PACKET_BUF: Global<u32> = Global::new(0x00ce_c220);
    /// `g_pppFpHeap2Buf` (`0x00cec224`): the second heap's block, allocated
    /// by `pppSetFpPdt` when there is none and the table asks for one (the
    /// first such table, or the first after [`ppp_fp_free`]).
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
    /// 0x40000)`. When there is no second heap and the table asks for one
    /// (`byte +1` blocks of 8 KB: the first such table, or the first after
    /// [`ppp_fp_free`]) it is allocated (`user_malloc`) and made
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
    /// packet buffer is allocated when there is none (the first table, or
    /// the first after `pppFpFree`): 0xc0000 bytes, 0x100000 for `byte +2` 1,
    /// 0x140000 for 2; its use and peak reset. Callees through `call`.
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

    /// `pppFpInitHeapDisplay(heap)` (`0x007299e0`): the heap display
    /// redone (`op_heap_display_clear(0x601)`,
    /// `op_heap_display_set_par(heap, 0x8603)`) and [`HEAP_DISPLAY_SET`]
    /// set.
    pub fn ppp_fp_init_heap_display(mem: &mut impl Mem, heap: u32) {
        crate::op::bridge::op_heap_display_clear(mem, 0x601);
        crate::op::bridge::op_heap_display_set_par(mem, heap, 0x8603);
        HEAP_DISPLAY_SET.set(mem, 1);
    }

    /// `g_FpPointLightCap` (`0x00cec1f8`): how many point lights
    /// `pppFpPointLightProc` has defined (`Sg_PLightDefine`); `pppFpFree`
    /// disposes of them.
    pub const POINT_LIGHT_CAP: Global<i32> = Global::new(0x00ce_c1f8);

    /// `*g_pppEnvSlot` = `g_pppFpHeap`: the field effects' heap current.
    fn set_env(mem: &mut impl Mem) {
        let slot: u32 = mem.read(
            Global::<u32>::new(crate::draw::bridge::ENV_SLOT)
                .ptr(mem)
                .addr,
        );
        let heap = crate::heap::bridge::FP_HEAP.ptr(mem).addr;
        mem.write(slot, heap);
    }

    /// [`set_env`], then `pppAllFreePObject(run)`.
    fn free_objects<M: Mem>(
        mem: &mut M,
        run: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        set_env(mem);
        let free = Global::<u32>::new(0x0071_1f20).ptr(mem).addr;
        call(mem, free, &[run]);
    }

    /// `pppFpFree()` (`0x007291c0`): `op_fp_effect_free()`; with a table
    /// loaded, every effect's objects freed (`pppAllFreePObject` on its run
    /// record, `g_pppFpHeap` current) and [`DATA`] cleared; the second
    /// heap's block and the packet buffer freed (`user_free_thunk`) and
    /// cleared; defined point lights disposed of
    /// (`Sg_PLightDispose_thunk`) and [`POINT_LIGHT_CAP`] cleared. Callees
    /// through `call`.
    pub fn ppp_fp_free<M: Mem>(mem: &mut M, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) {
        fn r(mem: &impl Mem, a: u32) -> u32 {
            Global::<u32>::new(a).ptr(mem).addr
        }
        let effect_free = r(mem, 0x007f_cc40);
        call(mem, effect_free, &[]);
        let table = DATA.get(mem);
        if table != 0 {
            set_env(mem);
            let mut k = 0u32;
            let mut n = u32::from(mem.read::<u16>(table.wrapping_add(4)));
            while k < n {
                let run = RUN.ptr(mem).at(k).addr;
                free_objects(mem, run, call);
                k += 1;
                n = u32::from(mem.read::<u16>(DATA.get(mem).wrapping_add(4)));
            }
            DATA.set(mem, 0);
        }
        let user_free = r(mem, 0x0068_7010);
        for buf in [HEAP2_BUF, PACKET_BUF] {
            let p = buf.get(mem);
            if p != 0 {
                call(mem, user_free, &[p]);
                buf.set(mem, 0);
            }
        }
        if POINT_LIGHT_CAP.get(mem) > 0 {
            let dispose = r(mem, 0x0083_1a30);
            call(mem, dispose, &[]);
            POINT_LIGHT_CAP.set(mem, 0);
        }
    }

    /// `pppFpIDfree_007295f0(id)` (`0x007295f0`): with a table loaded,
    /// every effect with that id (`u16 +0x64`) has its objects freed
    /// (`pppAllFreePObject`, `g_pppFpHeap` current) and goes back to not
    /// started (counter -0x1000). Callees through `call`.
    pub fn ppp_fp_id_free<M: Mem>(
        mem: &mut M,
        id: u16,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        free_matching(mem, 0x64, id, call);
    }

    /// `pppFpGrpForceOff(group)` (`0x00729480`): [`ppp_fp_id_free`] for
    /// every effect of the group (`u16 +0x66`).
    pub fn ppp_fp_grp_force_off<M: Mem>(
        mem: &mut M,
        group: u16,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        free_matching(mem, 0x66, group, call);
    }

    /// The effects whose `u16` at `at` (in the entry) is `v`: objects
    /// freed, counter -0x1000.
    fn free_matching<M: Mem>(
        mem: &mut M,
        at: u32,
        v: u16,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let table = DATA.get(mem);
        if table == 0 {
            return;
        }
        set_env(mem);
        let n = u32::from(mem.read::<u16>(table.wrapping_add(4)));
        for k in 0..n {
            if mem.read::<u16>(table.wrapping_add(at + 0x50 * k)) == v {
                let run = RUN.ptr(mem).at(k).addr;
                free_objects(mem, run, call);
                mem.write(run, NOT_STARTED);
            }
        }
    }

    /// `pppFpLoadStat(stat)` (`0x00729a10`): the inverse of
    /// [`ppp_fp_save_stat`]: each effect with a nonzero id takes the next
    /// two bits of `stat` (four effects a byte, low bits first): the first
    /// restarts it (`Fp_Restart(id)`), the second is its hidden flag
    /// (display `+0` bit 0x10). Nothing without `stat` or a table.
    pub fn ppp_fp_load_stat(mem: &mut impl Mem, stat: u32) {
        let table = DATA.get(mem);
        if stat == 0 || table == 0 {
            return;
        }
        let n = u32::from(mem.read::<u16>(table.wrapping_add(4)));
        let (mut byte, mut bit) = (stat, 0u32);
        for k in 0..n {
            let id = mem.read::<u16>(table.wrapping_add(0x64 + 0x50 * k));
            if id == 0 {
                continue;
            }
            if mem.read::<u8>(byte) & (1 << bit) != 0 {
                fp_restart(mem, u32::from(id));
            }
            let d = DISP.ptr(mem).at(k).addr;
            let flags: u32 = mem.read(d);
            let hidden = mem.read::<u8>(byte) & (2 << bit) != 0;
            mem.write(d, if hidden { flags | 0x10 } else { flags & !0x10 });
            bit += 2;
            if bit >= 8 {
                bit = 0;
                byte = byte.wrapping_add(1);
            }
        }
    }

    /// `pppSetFpTex(oef)` (`0x0072aa40`): `op_fp_effect_init()`, the oef2
    /// file set (`op_oef2_set2`), then each of its `u16 +0x44` textures
    /// (`op_oef2_texture_load(op_oef2_a_get(oef, 2, i), 1)`) and `u16
    /// +0x46` palettes (`op_oef2_clut_load(op_oef2_a_get(oef, 3, i), 1)`)
    /// loaded. Callees other than the oef2 lookups through `call`.
    pub fn ppp_set_fp_tex<M: Mem>(
        mem: &mut M,
        oef: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use crate::op::bridge as op;
        fn r(mem: &impl Mem, a: u32) -> u32 {
            Global::<u32>::new(a).ptr(mem).addr
        }
        let init = r(mem, 0x007f_cc60);
        call(mem, init, &[]);
        op::op_oef2_set2(mem, oef);
        for (count, kind, load) in [(0x44u32, 2u32, 0x007e_5030u32), (0x46, 3, 0x007e_46b0)] {
            let load = r(mem, load);
            let mut i = 0u32;
            while i < u32::from(mem.read::<u16>(oef.wrapping_add(count))) {
                let a = op::op_oef2_a_get(mem, oef, kind, i);
                call(mem, load, &[a, 1]);
                i += 1;
            }
        }
    }

    // ---- round 49 b ----

    /// `g_VfxYiqMode` (`0x00c3a504`): the effect `pppFpLoop` runs (-1 off).
    pub const YIQ_MODE: Global<u32> = Global::new(0x00c3_a504);
    /// `g_pppFpCurrentId` (`0x00c3a500`): the running effect's `u16 +0x64`.
    pub const CURRENT_ID: Global<u32> = Global::new(0x00c3_a500);
    /// `g_PppPacketPos` (`0x0230fd2c`).
    const PACKET_POS: Global<u32> = Global::new(0x0230_fd2c);
    /// `g_CurrentMapNo` (`0x012fbc40`).
    const MAP_NO: Global<u32> = Global::new(0x012f_bc40);

    /// Whether `pppFpLoop` still runs effect `i`, an entry of ID 0 (`u16
    /// +0x44`) that `pppCheckViewCylinder` puts out of view (a non-zero
    /// result; the local `seen` in [`ppp_fp_loop`] holds it): on map 0x136
    /// all but 2, 6 and 10; on map 0x31 only effect 0x26, while `getScenerioFlag()` is at
    /// most 0x2c; on map 0x176 only effect 0xd; elsewhere none.
    fn runs_in_view<M: Mem>(
        mem: &mut M,
        i: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        match MAP_NO.get(mem) {
            0x136 => !matches!(i, 2 | 6 | 10),
            0x31 => {
                // (the original then tests map 0x176 and effect 0xd again,
                // which effect 0x26 never passes)
                if i != 0x26 {
                    return false;
                }
                let flag = call(mem, sym(mem, 0x0078_7360), &[]);
                #[allow(clippy::cast_possible_truncation)] // cmp ax
                let flag = flag as u16;
                flag <= 0x2c
            }
            0x176 => i == 0xd,
            _ => false,
        }
    }

    fn sym(mem: &impl Mem, a: u32) -> u32 {
        Global::<u32>::new(a).ptr(mem).addr
    }

    /// `pppFpLoop()` (`0x00729ab0`): the field effects' frame. With
    /// [`FP_DISP`] on: the screen and camera matrices copied
    /// (`MsCopyScreenMatrix(g_pppScreenMatrix)`,
    /// `MsCopyCameraMatrix(g_pppCameraMatrix)`), the camera's inverse into
    /// `g_pppFillMatrix` (`sceVu0InversMatrix4x4`),
    /// `UpdatePhyreViewProjectionMatrix()`, `pppFpSetWoodMatrix()` with
    /// [`WOOD`]; the packet position at [`PACKET_BUF`], the field heap
    /// current, both point-light words cleared. Then each effect `i` of
    /// [`DATA`] (the count re-read every pass): hidden (display record
    /// bit 0x10) it gets `hideDisplayOffParticle(run)`; else [`YIQ_MODE`]
    /// is `i` and a started one (counter not -0x1000) runs, unless it has
    /// ID 0 (`u16 +0x44`), lies out of view (`pppCheckViewCylinder(camera,
    /// run + 0x40, far, radius, height)` non-zero, with the run's floats
    /// `+0x20..+0x28`; a `far` below 0 turns the test off) and is not let
    /// through by [`runs_in_view`].
    /// Running: [`CURRENT_ID`] is its `+0x44`; a non-negative counter
    /// counts down, and on passing below 0 the objects are freed and the
    /// part restarted with `pppStartPart(run, 0x1000, p, 1)`, `p` being
    /// program `[entry + 0x34]` of the table's `+0x10` (a status `+0x1d`
    /// of 2 becomes 1); else the effect waits. Then `pppFpLoopMatrix(run,
    /// disp, entry)`, the colour scale (`yiGetColorScale(colour)`, each
    /// word clamped to at most 1.0, scaled by `[0x00b50224]` (4096) with
    /// `FFXVu0ScaleVector`) truncated into the run's `u16 +0x70..+0x76`,
    /// and `pppRunPartFp(run, 0x30)`; a nonzero result frees the objects
    /// and stops the effect. At the end: `pppFpPointLightProc()`, the
    /// packet use and peak (with two `dbgPrintf`s over the budget in
    /// debug), the heap use (the two heaps' `+0x1c` and `+0xc`) and peak,
    /// the packet position cleared; in debug `op_heap_display_set(used,
    /// budget, 0x601)` and `op_heap_display_set(heap, span, 0x603)`, the
    /// span adding `last - first + 0x10` of each heap with both words
    /// set. [`YIQ_MODE`] -1 in any case. `colour` is the original's local
    /// (its `ebp - 0x14`, 16 bytes). Callees through `call`.
    #[allow(clippy::too_many_lines)]
    pub fn ppp_fp_loop<M: Mem>(
        mem: &mut M,
        colour: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if FP_DISP.get(mem) == 0 {
            YIQ_MODE.set(mem, u32::MAX);
            return;
        }
        let camera = sym(mem, CAMERA_MATRIX);
        let screen = sym(mem, 0x0230_fe60);
        call(mem, sym(mem, 0x007b_ba50), &[screen]);
        call(mem, sym(mem, 0x007c_0070), &[camera]);
        let fill = sym(mem, 0x0230_fca0);
        call(mem, sym(mem, 0x006e_d460), &[fill, camera]);
        call(mem, sym(mem, 0x0071_1910), &[]);
        let wood = WOOD.get(mem);
        YIQ_MODE.set(mem, u32::MAX);
        if wood != 0 {
            call(mem, sym(mem, 0x0072_a690), &[]);
        }
        let buf = PACKET_BUF.get(mem);
        PACKET_POS.set(mem, buf);
        set_env(mem);
        POINT_LIGHT_A.set(mem, 0);
        POINT_LIGHT_B.set(mem, 0);
        let count = |mem: &M| u32::from(mem.read::<u16>(DATA.get(mem).wrapping_add(4)));
        let mut i = 0;
        while i < count(mem) {
            let (run, disp) = (run(mem, i), disp(mem, i));
            if mem.read::<u8>(disp) & 0x10 != 0 {
                call(mem, sym(mem, 0x0072_8e40), &[run]);
                i += 1;
                continue;
            }
            YIQ_MODE.set(mem, i);
            if mem.read::<u32>(run) == NOT_STARTED {
                i += 1;
                continue;
            }
            let mut entry = DATA.get(mem).wrapping_add(0x20 + 0x50 * i);
            if mem.read::<u16>(entry.wrapping_add(0x44)) == 0 {
                // fld m32 / fstp m32 onto the stack
                let float = |mem: &M, o: u32| crate::x87::load(mem.read(run.wrapping_add(o)));
                let (x, y, z) = (float(mem, 0x20), float(mem, 0x24), float(mem, 0x28));
                let args = [camera, run.wrapping_add(0x40), x, y, z];
                let seen = call(mem, sym(mem, 0x0072_8f10), &args);
                if seen != 0 && !runs_in_view(mem, i, call) {
                    i += 1;
                    continue;
                }
            }
            CURRENT_ID.set(mem, u32::from(mem.read::<u16>(entry.wrapping_add(0x44))));
            let counter = mem.read::<u32>(run).cast_signed();
            if counter >= 0 {
                let left = counter.wrapping_sub(1);
                mem.write(run, left);
                if left >= 0 {
                    i += 1;
                    continue;
                }
                let mode2 = mem.read::<u8>(run.wrapping_add(0x1d)) == 2;
                free_objects(mem, run, call);
                let data = DATA.get(mem);
                entry = data.wrapping_add(0x20 + 0x50 * i);
                set_env(mem);
                let programs = ffx_system::stub::ptr32(mem.read(data.wrapping_add(0x10)));
                let k = mem.read::<u32>(entry.wrapping_add(0x34));
                let program = mem.read::<u32>(programs.wrapping_add(k.wrapping_mul(4)));
                call(mem, sym(mem, 0x0071_23c0), &[run, 0x1000, program, 1]);
                if mode2 {
                    mem.write(run.wrapping_add(0x1d), 1u8);
                }
            }
            call(mem, sym(mem, 0x0072_8ac0), &[run, disp, entry]);
            call(mem, sym(mem, 0x0090_88d0), &[colour]);
            for k in 0..4 {
                let at = colour.wrapping_add(4 * k);
                if f32::from_bits(mem.read(at)) > 1.0 {
                    mem.write(at, 1.0f32);
                }
            }
            let scale = crate::x87::load(mem.read(sym(mem, 0x00b5_0224)));
            call(mem, sym(mem, 0x006e_d8f0), &[colour, colour, scale]);
            for k in 0..4 {
                let x = f32::from_bits(mem.read(colour.wrapping_add(4 * k)));
                let v = ffx_bridge::x87::cvttsd2si(f64::from(x));
                #[allow(clippy::cast_possible_truncation)] // the low half
                mem.write(run.wrapping_add(0x70 + 2 * k), v as u16);
            }
            set_env(mem);
            if call(mem, sym(mem, 0x0071_22f0), &[run, 0x30]) != 0 {
                free_objects(mem, run, call);
                mem.write(run, NOT_STARTED);
            }
            i += 1;
        }
        call(mem, sym(mem, 0x0072_9f90), &[]);
        let debug = call(mem, sym(mem, 0x0088_7c50), &[]);
        let [size, used_at, peak_at, heap_at, heap_peak_at] = STATS;
        let g = |mem: &M, a: u32| mem.read::<u32>(sym(mem, a)).cast_signed();
        let used = PACKET_POS
            .get(mem)
            .wrapping_sub(PACKET_BUF.get(mem))
            .cast_signed();
        if debug != 0 && used > g(mem, size) {
            let print = sym(mem, 0x0062_f500);
            let kb = (used / 1024).cast_unsigned();
            let format = sym(mem, 0x00b5_13d4);
            call(mem, print, &[format, kb]);
            // the first call's format is still pushed below the second's
            // (which prints no arguments)
            call(mem, print, &[sym(mem, 0x00b5_13fc), format]);
        }
        mem.write(sym(mem, used_at), used);
        if g(mem, peak_at) < used {
            mem.write(sym(mem, peak_at), used);
        }
        let heap = g(mem, 0x00ce_c248).wrapping_add(g(mem, 0x00ce_c238));
        PACKET_POS.set(mem, 0);
        let mut span = 0i32;
        for (first, last) in [(0x00ce_c230, 0x00ce_c234), (0x00ce_c240, 0x00ce_c244)] {
            let (a, b) = (g(mem, first), g(mem, last));
            if b != 0 && a != 0 {
                span = span.wrapping_add(b.wrapping_sub(a).wrapping_add(0x10));
            }
        }
        mem.write(sym(mem, heap_at), heap);
        if g(mem, heap_peak_at) < heap {
            mem.write(sym(mem, heap_peak_at), heap);
        }
        if debug != 0 {
            let set = sym(mem, 0x007f_f290);
            let budget = g(mem, size).cast_unsigned();
            call(mem, set, &[used.cast_unsigned(), budget, 0x601]);
            call(
                mem,
                set,
                &[heap.cast_unsigned(), span.cast_unsigned(), 0x603],
            );
        }
        YIQ_MODE.set(mem, u32::MAX);
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
