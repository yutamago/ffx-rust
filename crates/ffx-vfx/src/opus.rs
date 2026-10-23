//! The "opus" effect objects (`opus_*`, and the `opu_*`/`opp_*` script
//! commands that step them).
//!
//! An opus object has a draw list at `+0x1c` (`i16` part numbers, ended by
//! a negative one) and its 0x100-byte parts at `+0x20`; a part's group
//! byte is at `+0x1e` and its draw kind at `+0xbb`. The script commands
//! take the object and the command's address and return the next
//! command's.

/// `opus_encount0`'s particle grid, `nz` planes of `ny` rows of `nx`
/// particles in the order the original lays them out: each is `[x, y, z,
/// 1, t, 0, 0, 1]`, `x` from 0 down by 64 along a row, `y` from 0 down by
/// 64 per row, `z` from 0 down by 64 per plane, and `t` from 2112 up by 64
/// per particle. Each step is the x87's: the `f32` value plus 64.0 in 53
/// bits, rounded to `f32` where the original stores it.
pub fn encount0_grid(nx: u16, ny: u16, nz: u16, mut put: impl FnMut([f32; 8])) {
    #[allow(clippy::cast_possible_truncation)] // the original's fstp
    let step = |v: f32, d: f64| (f64::from(v) + d) as f32;
    let (mut t, mut z) = (2112.0f32, 0.0f32);
    for _ in 0..nz {
        let mut y = 0.0f32;
        for _ in 0..ny {
            let mut x = 0.0f32;
            for _ in 0..nx {
                put([x, y, z, 1.0, t, 0.0, 0.0, 1.0]);
                t = step(t, 64.0);
                x = step(x, -64.0);
            }
            y = step(y, -64.0);
        }
        z = step(z, -64.0);
    }
}

/// The objects where the game keeps them.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    use crate::et::bridge::{EFF, OPUS_BATTLE};
    use crate::x87::{ADD, MUL, Reg};

    /// `g_OpusWeapons`: the four weapon objects.
    pub const WEAPONS: Global<u32> = Global::new(0x0119_fea0);
    /// `g_OpusMonsters`: the eight monster objects.
    pub const MONSTERS: Global<u32> = Global::new(0x0119_fed0);

    /// `FFXVu0ApplyMatrix` on values: `v` times the row-major `m`.
    fn apply_rows(m: &[[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
        std::array::from_fn(|i| {
            let col = [m[0][i], m[1][i], m[2][i], m[3][i]];
            #[allow(clippy::cast_possible_truncation)] // 0..4
            ffx_system::math::mul4x4_component(i as u32, v, col)
        })
    }

    /// The most vertices `opus_wave_light_007f60b0`'s frame holds colours
    /// for (0x4400 bytes of them; more overrun the original's frame).
    pub const WAVE_LIGHT_MAX: usize = 0x440;

    /// `opus_wave_light_setup_0080e2f0(unit, _, list, src, nos)`
    /// (`0x0080e2f0`, name ours; called by `0x0080df10`): the four lights
    /// [`opus_wave_light`] reads. The colour (`unit +0x24`) and position
    /// (`unit +0x28`) blocks, 4 x 16 bytes each, start as `r0` (the
    /// emulated EE's zero register). For each `u16` of `nos +2..+0xa` that
    /// is not 0, the unit `L` it names ([`opus_no_get`] from `src`):
    /// position = `L +0x90` less `src +0x90` (`f32`, through the VU0
    /// registers), then `restoreVf00Register`; its intensity (the
    /// position's `w`) = `vf05.x / 256 x L +0x3c`, `vf05` reset to `(1, 0,
    /// t1.zw)` when `L +0x4c` (through `g_pppColorConShift`) and `t1.y`
    /// equal `r0.xy`'s words; the colour = the bytes of `L +0xc8`
    /// widened to words by `pextlb` and `pextlh` with `r0` (`t1`'s other
    /// words in the upper half) as `f32` / 64. Last the unit's base and
    /// gain bytes `+0x4c` / `+0x4d`: `ftol(src +0x30)`, then `src +0x4c`,
    /// halved toward zero and held in `0..=0x80`. Returns what the
    /// original leaves in `eax` (the last `ftol` when held at 0x80, else
    /// the byte).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opus_wave_light_setup<M: Mem>(
        mem: &mut M,
        unit: u32,
        list: u32,
        src: u32,
        nos: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load, mul, store, sub};
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let (r0, t1, shift) = (
            g(mem, 0x00c8_f508),
            g(mem, 0x00c8_f528),
            g(mem, 0x00c8_b500),
        );
        // the VU0 registers: vf02 vf03 vf05 vf06 vf07, and the move register
        let (vf02, vf03, vf05, vf06, vf07, mv) = (
            g(mem, 0x00c8_f920),
            g(mem, 0x00c8_f930),
            g(mem, 0x00c8_f950),
            g(mem, 0x00c8_f960),
            g(mem, 0x00c8_f970),
            g(mem, 0x00c8_f980),
        );
        let restore = g(mem, 0x0042_ef00);
        let r: [u32; 4] = mem.read(r0);
        let (mut colour, mut pos) = (
            rd(mem, unit.wrapping_add(0x24)),
            rd(mem, unit.wrapping_add(0x28)),
        );
        for k in 0..4 {
            mem.write(colour.wrapping_add(16 * k), r);
            mem.write(pos.wrapping_add(16 * k), r);
        }
        for k in 0..4u32 {
            let no = u32::from(mem.read::<u16>(nos.wrapping_add(2 + 2 * k)));
            if no != 0 {
                let l = opus_no_get(mem, list, src, no);
                let from: [u32; 4] = mem.read(src.wrapping_add(0x90));
                mem.write(mv, from);
                let to: [u32; 4] = mem.read(l.wrapping_add(0x90));
                mem.write(vf07, to);
                let d = [0usize, 1, 2, 3].map(|j| store(sub(load(to[j]), load(from[j]))));
                mem.write(vf06, d);
                call(mem, restore, &[]);
                let d: [u32; 4] = mem.read(vf06);
                mem.write(pos, d);
                let sh = rd(mem, shift);
                let tag = rd(mem, sh.wrapping_add(l).wrapping_add(0x4c));
                mem.write(shift, 0u32);
                let t: [u32; 4] = mem.read(t1);
                if tag == r[0] && t[1] == r[1] {
                    mem.write(vf05, [1f32.to_bits(), 0, t[2], t[3]]);
                }
                let v5: [u32; 4] = mem.read(vf05);
                let third = 0x3b80_0000u32; // 1/256
                mem.write(vf03, [third, 0, t[2], t[3]]);
                let scaled = store(mul(load(third), load(v5[0])));
                let size = rd(mem, l.wrapping_add(0x3c));
                mem.write(vf02, [size, 0, t[2], t[3]]);
                let w = store(mul(load(scaled), load(size)));
                mem.write(vf05, w);
                mem.write(pos.wrapping_add(0xc), w);
                // pextlb t1, r0, t1 on (L +0xc8, vf05.y), then pextlh t1, r0,
                // t1: only the low half of the bytes' result (from L +0xc8 and
                // r0.x) reaches the words
                let c = rd(mem, l.wrapping_add(0xc8));
                let bytes = |w: u32, z: u32, half: u32| {
                    let b = |x: u32, i: u32| (x >> (8 * i)) & 0xff;
                    let (i, j) = (2 * half, 2 * half + 1);
                    b(w, i) | b(z, i) << 8 | b(w, j) << 16 | b(z, j) << 24
                };
                let wide = [bytes(c, r[0], 0), bytes(c, r[0], 1)];
                let halves = |w: u32, z: u32, half: u32| {
                    let h = |x: u32| (x >> (16 * half)) & 0xffff;
                    h(w) | h(z) << 16
                };
                let words = [
                    halves(wide[0], r[0], 0),
                    halves(wide[0], r[0], 1),
                    halves(wide[1], r[1], 0),
                    halves(wide[1], r[1], 1),
                ];
                let sixty_fourth = 0x3c80_0000u32;
                mem.write(t1, [sixty_fourth, 0, words[2], words[3]]);
                mem.write(vf07, [sixty_fourth, 0, words[2], words[3]]);
                let x = load(sixty_fourth);
                let out = words.map(|v| {
                    let f = store(f64::from(v.cast_signed()));
                    store(mul(x, load(f)))
                });
                mem.write(mv, out);
                mem.write(colour, out);
            }
            pos = pos.wrapping_add(0x10);
            colour = colour.wrapping_add(0x10);
        }
        let mut eax = 0;
        for (from, to) in [(0x30u32, 0x4cu32), (0x4c, 0x4d)] {
            let v = ftol(mem, load(rd(mem, src.wrapping_add(from)))).cast_signed();
            let q = v / 2;
            let b = if q >= 0x80 {
                eax = v.cast_unsigned();
                0x80
            } else {
                let b = q.max(0).cast_unsigned();
                eax = b;
                b
            };
            #[allow(clippy::cast_possible_truncation)] // at most 0x80
            mem.write(unit.wrapping_add(to), b as u8);
        }
        eax
    }

    /// `opus_wave_light_007f60b0(_, shape, unit)` (`0x007f60b0`, name ours;
    /// `opus_0154_wave_draw`'s): vertex colours from four point lights.
    /// The model matrix (`unit +0x2c`) with its first three rows scaled to
    /// unit length (each `f32` sum of squares, x, y, z, and its root) is the normal
    /// basis. For each of the shape's `i16 +0x12` vertices (16 bytes each
    /// at shape + `+8`), the vertex through the model matrix, then for each
    /// light (`unit +0x28`: 16 bytes, position and intensity) the
    /// direction `d` to it (`f32` per component) and `|d|^2` (summed as
    /// `f32`, x, y, then z): `d * intensity / |d|^2`; the normal (shape +
    /// `+0xc`, 16 bytes each) through the basis-times-directions matrix
    /// gives a term per light, negative ones 0, then through the colour
    /// matrix (`unit +0x24`), and with `base` = byte `+0x4c` / 128, `gain`
    /// = byte `+0x4d` / 128 (`f32` each) each of r, g, b becomes `128 (c
    /// gain 2 + base)` (`128 base` below 0), at most 128. Then each of the
    /// strips' records (shape + `+4`: `i16 +2` of 0x28 bytes from `+0x10`)
    /// has four `u16` vertex indices at `+0x10` and gets their colours
    /// (truncated, low bytes) as `r g b` at `+0`, `+4`, `+8`, `+0xc`.
    /// Index past the colours made (stack garbage in the original) read
    /// as 0, and colours past [`WAVE_LIGHT_MAX`] are kept (the original
    /// overruns its frame).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opus_wave_light<M: Mem>(mem: &mut M, shape: u32, unit: u32) {
        use ffx_bridge::x87::{add, cvttsd2si, div, load, mul, sqrt, store, sub};
        let rows = |mem: &M, at: u32| -> [[f32; 4]; 4] {
            std::array::from_fn(|r| {
                #[allow(clippy::cast_possible_truncation)] // 0..4
                let r = r as u32;
                mem.read(at.wrapping_add(16 * r))
            })
        };
        let f = |x: f32| load(x.to_bits());
        let model_at: u32 = mem.read(unit.wrapping_add(0x2c));
        let model = rows(mem, model_at);
        // the normal basis: rows 0..2 scaled to unit length
        let mut basis = model;
        for row in basis.iter_mut().take(3) {
            let [m0, m1, m2, _] = *row;
            let s = store(add(
                add(mul(f(m0), f(m0)), mul(f(m1), f(m1))),
                mul(f(m2), f(m2)),
            ));
            let q = store(sqrt(load(s)));
            for v in row.iter_mut().take(3) {
                *v = f32::from_bits(store(div(f(*v), load(q))));
            }
        }
        let normals = shape.wrapping_add(mem.read(shape.wrapping_add(0xc)));
        let verts = shape.wrapping_add(mem.read(shape.wrapping_add(8)));
        let strips = shape.wrapping_add(mem.read(shape.wrapping_add(4)));
        let byte = |mem: &M, off: u32| f64::from(mem.read::<u8>(unit.wrapping_add(off)));
        let base = store(mul(byte(mem, 0x4c), 0.007_812_5));
        let gain = store(mul(0.007_812_5, byte(mem, 0x4d)));
        let count = |mem: &M| i32::from(mem.read::<i16>(shape.wrapping_add(0x12)));
        let mut colours: Vec<[f32; 4]> = Vec::new();
        let mut i = 0i32;
        while i < count(mem) {
            let at = i.cast_unsigned().wrapping_mul(16);
            let model = rows(mem, mem.read(unit.wrapping_add(0x2c)));
            let v = apply_rows(&model, mem.read(verts.wrapping_add(at)));
            let lights: u32 = mem.read(unit.wrapping_add(0x28));
            let mut dirs = [[0f32; 4]; 4];
            for (k, dir) in (0u32..).zip(dirs.iter_mut()) {
                let light: [f32; 4] = mem.read(lights.wrapping_add(16 * k));
                let d: [u32; 3] = std::array::from_fn(|c| store(sub(f(light[c]), f(v[c]))));
                let sq = |x: u32| mul(load(x), load(x));
                let t = store(add(sq(d[0]), 0.0));
                let t = store(add(load(t), sq(d[1])));
                let dist = store(add(load(t), sq(d[2])));
                let s = f(light[3]);
                for (c, out) in dir.iter_mut().take(3).enumerate() {
                    let p = store(mul(load(d[c]), s));
                    *out = f32::from_bits(store(div(load(p), load(dist))));
                }
            }
            // row r, column k: basis row r dotted with direction k
            let mut lit = [[0f32; 4]; 4];
            for (row, b) in lit.iter_mut().zip(basis.iter()).take(3) {
                for (out, d) in row.iter_mut().zip(dirs.iter()) {
                    let x = add(
                        add(mul(f(d[0]), f(b[0])), mul(f(d[1]), f(b[1]))),
                        mul(f(d[2]), f(b[2])),
                    );
                    *out = f32::from_bits(store(x));
                }
            }
            let mut c = apply_rows(&lit, mem.read(normals.wrapping_add(at)));
            for x in &mut c {
                if *x < 0.0 {
                    *x = 0.0;
                }
            }
            let colour_at: u32 = mem.read(unit.wrapping_add(0x24));
            let mut c = apply_rows(&rows(mem, colour_at), c);
            for x in c.iter_mut().take(3) {
                let t = if 0.0 <= *x || x.is_nan() {
                    add(mul(mul(f(*x), load(gain)), 2.0), load(base))
                } else {
                    load(base)
                };
                *x = f32::from_bits(store(mul(t, 128.0)));
                if *x > 128.0 {
                    *x = 128.0;
                }
            }
            colours.push(c);
            i = (i + 1) & 0xffff;
        }
        let strip_count = |mem: &M| i32::from(mem.read::<i16>(strips.wrapping_add(2)));
        let mut s = 0i32;
        while s < strip_count(mem) {
            let rec = strips
                .wrapping_add(0x10)
                .wrapping_add(s.cast_unsigned().wrapping_mul(0x28));
            for j in 0..4u32 {
                let idx: u16 = mem.read(rec.wrapping_add(0x10 + 2 * j));
                let c = colours.get(usize::from(idx)).copied().unwrap_or_default();
                for (k, x) in (0u32..).zip(c.iter().take(3)) {
                    #[allow(clippy::cast_possible_truncation)] // the low byte
                    let b = cvttsd2si(f(*x)) as u8;
                    mem.write(rec.wrapping_add(4 * j + k), b);
                }
            }
            s += 1;
        }
    }

    /// `opus_encount0_init(obj)` (`0x007f8480`): `f32 +0x20` = 16, `+0x24`
    /// = 32, `+0x28` = 0, and the 0x20-byte particles from `+0x30` laid
    /// out by [`super::encount0_grid`] for the `u16` counts `+2` (x), `+4`
    /// (y) and `+6` (z). Returns `eax` as left: the z count (0 for none).
    pub fn opus_encount0_init(mem: &mut impl Mem, obj: u32) -> u32 {
        mem.write(obj.wrapping_add(0x20), 16.0f32);
        mem.write(obj.wrapping_add(0x24), 32.0f32);
        mem.write(obj.wrapping_add(0x28), 0.0f32);
        let count = |at: u32| mem.read::<u16>(obj.wrapping_add(at));
        let (nx, ny, nz) = (count(2), count(4), count(6));
        let mut at = obj.wrapping_add(0x30);
        super::encount0_grid(nx, ny, nz, |p| {
            mem.write(at, p);
            at = at.wrapping_add(0x20);
        });
        u32::from(nz)
    }

    /// `opus_mimic_check(chr)` (`0x0080f630`): the `g_EtChr` record of
    /// character `chr` (`opus_get_chr_status_index(chr, 0, 0)`, the record
    /// before the table for none) when its number (`+4`) is one of the
    /// mimic models 0x10fb, 0x10fc, 0x10fd or 0x1100; else 0.
    pub fn opus_mimic_check(mem: &mut impl Mem, chr: u32) -> u32 {
        let i = crate::et::bridge::opus_get_chr_status_index(mem, chr, 0, 0, &mut |_, _, _| 0);
        let rec = crate::et::bridge::CHR
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(0x20));
        let no: u32 = mem.read(rec.wrapping_add(4));
        if matches!(no, 0x10fb..=0x10fd | 0x1100) {
            rec
        } else {
            0
        }
    }

    /// The mimic's drawing unit (`opus_get_osu_by_draw(0, 0, 0x12, 1)`,
    /// through `call`) and its source record (`+0xbc`), or `None`.
    fn mimic_source<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        let osu = call(mem, g(mem, 0x0080_0f60), &[0, 0, 0x12, 1]);
        (osu != 0).then(|| mem.read(osu.wrapping_add(0xbc)))
    }

    /// `opus_mdc_size_get_mimic(mode)` (`0x0080f580`): the bytes the
    /// mimic's effect packets take: for each of the source record's models
    /// (`u16 src +0xe`, reread per model; model numbers `u16` at `src
    /// +0x124`, 0x60 apart) `Ch_ModelCalcSizeEffectPacket(chr, no)` of the
    /// character `[[src +4]]`, rounded up to 16 (masked to 28 bits), mode 2
    /// adding `(size / 28) * 12` rounded the same way. 0 without the unit.
    /// Callees through `call`.
    pub fn opus_mdc_size_get_mimic<M: Mem>(
        mem: &mut M,
        mode: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let Some(src) = mimic_source(mem, call) else {
            return 0;
        };
        let chr: u32 = mem.read(mem.read::<u32>(src.wrapping_add(4)));
        let mut total = 0u32;
        let mut model = src.wrapping_add(0x124);
        let mut i = 0i32;
        while i < i32::from(mem.read::<u16>(src.wrapping_add(0xe))) {
            let no = u32::from(mem.read::<u16>(model));
            let size = call(mem, g(mem, 0x0083_0450), &[chr, no]);
            let rounded = size.wrapping_add(0xf) & 0x0fff_fff0;
            let extra = if mode == 2 {
                ((rounded / 28) * 0xc + 0xf) & 0x0fff_fff0
            } else {
                0
            };
            total = total.wrapping_add(rounded.wrapping_add(extra));
            i += 1;
            model = model.wrapping_add(0x60);
        }
        total
    }

    /// `opus_mdc_make_mimic(out, mode, table)` (`0x0080f460`): the mimic's
    /// mdc: `out +0` = `[[src]]`, byte `out +6` = `mode`, then per model of
    /// the source record (`u16 src +0xe`, reread per model; model records
    /// at `src +0x124`, 0x60 apart, number `u16 +0`) a 16-byte head at `out
    /// +0x10`: word `+8` gets bit 0 when `Ch_ModelGetHide(chr, no)`, `+0`
    /// `set_ptr32(packet)`, `+4` `set_ptr32(model +0x1c)`, `u16 +0xa` the
    /// number; the packet (from `out +0x10 + 16 n`) is
    /// `Ch_ModelCreateEffectPacket(chr, no, packet)`, the next one
    /// `Ch_ModelCalcSizeEffectPacket(chr, no)` rounded up to 16 further,
    /// for mode 2 `op_md_vn_index(packet, that, table)`. `u16 out +4` =
    /// the model count. Returns `out`, 0 without the unit (nothing
    /// written). `chr` = `[[src +4]]`. Callees through `call`.
    pub fn opus_mdc_make_mimic<M: Mem>(
        mem: &mut M,
        out: u32,
        mode: u32,
        table: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let Some(src) = mimic_source(mem, call) else {
            return 0;
        };
        let chr: u32 = mem.read(mem.read::<u32>(src.wrapping_add(4)));
        let first: u32 = mem.read(mem.read::<u32>(src));
        mem.write(out, first);
        mem.write(out.wrapping_add(6), mode.to_le_bytes()[0]);
        let n = u32::from(mem.read::<u16>(src.wrapping_add(0xe)));
        let mut head = out.wrapping_add(0x10);
        let mut packet = head.wrapping_add(n << 4);
        let mut model = src.wrapping_add(0x124);
        let mut count = 0u16;
        if n != 0 {
            let mut i = 0i32;
            loop {
                // the number is reread for every callee, as the original does
                let number = |mem: &M| u32::from(mem.read::<u16>(model));
                if call(mem, g(mem, 0x0082_add0), &[chr, number(mem)]) != 0 {
                    let w: u16 = mem.read(head.wrapping_add(8));
                    mem.write(head.wrapping_add(8), w | 1);
                }
                let p = call(mem, g(mem, 0x00a4_4700), &[packet]);
                mem.write(head, p);
                let p = call(mem, g(mem, 0x00a4_4700), &[model.wrapping_add(0x1c)]);
                mem.write(head.wrapping_add(4), p);
                let no: u16 = mem.read(model);
                mem.write(head.wrapping_add(0xa), no);
                let size = call(mem, g(mem, 0x0083_0450), &[chr, number(mem)]);
                call(mem, g(mem, 0x0083_0490), &[chr, number(mem), packet]);
                let mut next = (size.wrapping_add(0xf) & 0x0fff_fff0).wrapping_add(packet);
                if mode == 2 {
                    next = call(mem, g(mem, 0x00a6_9f10), &[packet, next, table]);
                }
                i += 1;
                count = count.wrapping_add(1);
                model = model.wrapping_add(0x60);
                head = head.wrapping_add(0x10);
                packet = next;
                if i >= i32::from(mem.read::<u16>(src.wrapping_add(0xe))) {
                    break;
                }
            }
        }
        mem.write(out.wrapping_add(4), count);
        out
    }

    /// `Ch_GetRootGlobalMatrix(chr, out)`, called through `call`.
    const CH_GET_ROOT_GLOBAL_MATRIX: u32 = 0x0083_3f80;

    /// `opus_chara_root_m_get(out, chr)` (`0x00805400`): a character's
    /// original matrix (`Ch_GetOrgMatrix`: the pointer at `chr + 0x10c`)
    /// copied to `out` when it has one (`op_m_copy`), else its root
    /// global matrix (`Ch_GetRootGlobalMatrix(chr, out)`). Returns `eax`
    /// as left: the original matrix's address, or the callee's result.
    pub fn opus_chara_root_m_get<M: Mem>(
        mem: &mut M,
        out: u32,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let org: u32 = mem.read(chr.wrapping_add(0x10c));
        if org != 0 {
            crate::op::bridge::op_m_copy(mem, out, org);
            return org;
        }
        let f = Global::<u32>::new(CH_GET_ROOT_GLOBAL_MATRIX).ptr(mem).addr;
        call(mem, f, &[chr, out])
    }

    /// `Ch_OptposGetElement(chr, i, out)`, called through `call`.
    const CH_OPTPOS_GET_ELEMENT: u32 = 0x0083_5660;
    /// `Ch_ElemGetBaseMatrix(chr, e, out)`, called through `call`.
    const CH_ELEM_GET_BASE_MATRIX: u32 = 0x0083_3b80;

    /// `opus_get_bone_matrix(out, chr, bone)` (`0x00805430`): the base
    /// matrix of a character's element into `out` (`Ch_ElemGetBaseMatrix`).
    /// With bit 0x8000 the bone is an option position (`bone & 0xff`):
    /// `Ch_OptposGetElement` gives its element and an offset (16 bytes at
    /// `frame`, the original's local), and `out + 0x30` becomes that offset
    /// through the matrix (`op_apply_m_w1`). Otherwise the emulated EE
    /// register `g_Ee_Gpr` (`0x00c8f508..`, 16 bytes) is copied to `out +
    /// 0x40`. Returns `eax` as left: `0x00c8f93c` (`op_apply_m_w1`'s last
    /// load), or `0x00c8f514`; no caller reads it.
    pub fn opus_get_bone_matrix<M: Mem>(
        mem: &mut M,
        out: u32,
        chr: u32,
        bone: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let base = Global::<u32>::new(CH_ELEM_GET_BASE_MATRIX).ptr(mem).addr;
        if bone & 0x8000 != 0 {
            let f = Global::<u32>::new(CH_OPTPOS_GET_ELEMENT).ptr(mem).addr;
            let e = call(mem, f, &[chr, bone & 0xff, frame]);
            call(mem, base, &[chr, e, out]);
            crate::op::bridge::op_apply_m_w1(mem, out.wrapping_add(0x30), out, frame);
            return Global::<u32>::new(0x00c8_f93c).get(mem);
        }
        call(mem, base, &[chr, bone, out]);
        let gpr: [u32; 4] = mem.read(Global::<u32>::new(0x00c8_f508).ptr(mem).addr);
        mem.write(out.wrapping_add(0x40), gpr);
        gpr[3]
    }

    /// `MsGetChrScale(chr, out)`, called through `call`.
    const MS_GET_CHR_SCALE: u32 = 0x0079_5720;
    /// `Ch_GetScaleX/Y/Z(h)` and `Ch_GetDataScale(h)`, called through
    /// `fcall` (they return in `ST0`).
    const CH_GET_SCALE: [u32; 3] = [0x0082_ad30, 0x0082_ad40, 0x0082_ad50];
    const CH_GET_DATA_SCALE: u32 = 0x0082_ab60;

    /// `opus_rt_m_chara(m, chr, k, angles, trans, battle)` (`0x008062c0`):
    /// `m` becomes a character's scale matrix, then turned and moved.
    /// The scale (`frame - 0x20`, the original's 16-byte local) is the
    /// battle character's (`MsGetChrScale(chr, local)`) when `battle` is set
    /// and `chr < 0x20` (signed), else `Ch_GetScaleX/Y/Z` of its handle
    /// (`g_EtChrHandles[chr]`); it is scaled by `Ch_GetDataScale * k *
    /// 0.001f * (1/256)` (53 bits, stored as an `f32`, `op_xyz_scale`) and
    /// set as `m`'s diagonal (`op_unit_m_scale2`). Then `angles` and
    /// `trans` (either may be 0) through `op_rt_m`, `op_rot_m` or
    /// `op_trans_m`. No caller reads `eax`.
    #[allow(clippy::too_many_arguments, clippy::many_single_char_names)] // the original's six, the local and two calls
    pub fn opus_rt_m_chara<M: Mem>(
        mem: &mut M,
        m: u32,
        chr: u32,
        k: f32,
        angles: u32,
        trans: u32,
        battle: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use crate::op::bridge as op;
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        let v = frame.wrapping_sub(0x20);
        #[allow(clippy::cast_possible_wrap)] // cmp eax, 0x20; jge
        if battle == 0 || chr as i32 >= 0x20 {
            for (i, f) in CH_GET_SCALE.into_iter().enumerate() {
                let f = Global::<u32>::new(f).ptr(mem).addr;
                // fstp m32
                let x = ffx_bridge::x87::store(fcall(mem, f, &[h]));
                #[allow(clippy::cast_possible_truncation)] // 0..3
                mem.write(v.wrapping_add(4 * i as u32), x);
            }
        } else {
            let f = Global::<u32>::new(MS_GET_CHR_SCALE).ptr(mem).addr;
            call(mem, f, &[chr, v]);
        }
        let f = Global::<u32>::new(CH_GET_DATA_SCALE).ptr(mem).addr;
        // an fld m32 in the callee: exact
        let ds = f32::from_bits(ffx_bridge::x87::store(fcall(mem, f, &[h])));
        let milli: f64 = mem.read(Global::<u32>::new(0x00b4_7c68).ptr(mem).addr);
        let byte: f64 = mem.read(Global::<u32>::new(0x00b5_0198).ptr(mem).addr);
        let s = Reg::load_f32(ds)
            .mem(MUL, k)
            .constant(MUL, milli)
            .constant(MUL, byte)
            .store();
        // fstp [ebp - 0x24]; fld; fstp [esp]
        let s = crate::x87::load_f32(s);
        op::op_xyz_scale(mem, v, v, s.to_bits());
        op::op_unit_m_scale2(mem, m, v);
        match (angles, trans) {
            (0, 0) => {}
            (0, t) => {
                op::op_trans_m(mem, m, m, t);
            }
            (a, 0) => op::op_rot_m(mem, m, m, a),
            (a, t) => op::op_rt_m_zyx(mem, m, m, a, t),
        }
    }

    /// `0x0080e160(pk, mats, obj, unit, tex)` and `0x0080e230` (`clear`
    /// false): the 0x50-byte `op1_mdc_draw` packet header at `pk` for an
    /// opus model unit: `+0` the unit's `u16 +0xc0`, `+4` `obj +0x28`, `u16
    /// +8` the unit's `+0xc2`, `+0xc` 0, `+0x10` 0, `+0x18` RGB 0x80 with the
    /// unit's `+0xcb` as A, `+0x1c` the unit's `+0xc8` with its top byte
    /// cleared, `+0x20`/`+0x24`/`+0x28`/`+0x2c` the four 0x40-byte matrices
    /// of `mats` (`+0xc0`, `+0`, `+0x40`, `+0x80`), `+0x34` `obj +0x54`,
    /// `+0x38` 0, `+0x40` the unit's `+0xc4`, `+0x44` 0, `u16 +0x4c`
    /// 0x8080. `0x0080e160` also clears `+0x14` and `+0x3c`. Returns the
    /// model, `op_oef2_a_get(obj +0x60 [unit +0x1e], 4, tex)`, or 0 when
    /// `tex` is -1.
    #[allow(clippy::many_single_char_names)] // packet fields
    pub fn opus_mdc_packet_set(
        mem: &mut impl Mem,
        pk: u32,
        mats: u32,
        obj: u32,
        unit: u32,
        tex: u32,
        clear: bool,
    ) -> u32 {
        let at = |o: u32| pk.wrapping_add(o);
        let c: u32 = mem.read(unit.wrapping_add(0xc8));
        let a: u8 = mem.read(unit.wrapping_add(0xcb));
        mem.write(at(0x1c), c & 0x00ff_ffff);
        mem.write(at(0x18), [0x80u8, 0x80, 0x80, a]);
        if clear {
            mem.write(at(0x14), 0u32);
        }
        mem.write(at(0x10), 0u32);
        let n: u16 = mem.read(unit.wrapping_add(0xc0));
        mem.write(at(0), u32::from(n));
        let v: u32 = mem.read(obj.wrapping_add(0x28));
        mem.write(at(4), v);
        mem.write(at(0xc), 0u32);
        let v: u32 = mem.read(unit.wrapping_add(0xc4));
        mem.write(at(0x40), v);
        mem.write(at(0x38), 0u32);
        if clear {
            mem.write(at(0x3c), 0u32);
        }
        mem.write(at(0x4c), 0x8080u16);
        let v: u32 = mem.read(obj.wrapping_add(0x54));
        mem.write(at(0x34), v);
        let v: u16 = mem.read(unit.wrapping_add(0xc2));
        mem.write(at(8), v);
        mem.write(at(0x28), mats.wrapping_add(0x40));
        mem.write(at(0x2c), mats.wrapping_add(0x80));
        mem.write(at(0x24), mats);
        mem.write(at(0x20), mats.wrapping_add(0xc0));
        mem.write(at(0x44), 0u32);
        if tex == u32::MAX {
            return 0;
        }
        let g = u32::from(mem.read::<u8>(unit.wrapping_add(0x1e)));
        let oef: u32 = mem.read(obj.wrapping_add(0x60 + 4 * g));
        crate::op::bridge::op_oef2_a_get(mem, oef, 4, tex)
    }

    /// `opus_no_get(obj, unit, no)`, called through `call`.
    const OPUS_NO_GET: u32 = 0x0080_1060;

    /// `0x0080e900(pk, mats, obj, unit, nos)`: the four lights of an
    /// `op1_mdc_draw` packet (`0x0080e030`). Both light matrices, `pk +0x24`
    /// (colours) and `pk +0x28` (positions), start as four rows of the
    /// emulated EE register `g_Ee_Gpr` (`0x00c8f508`, 16 bytes); then for
    /// each unit number `u16 nos[1..=4]` that is not 0, the light unit
    /// (`opus_no_get(obj, unit, no)`, through `call`) gives row `i`: its
    /// position (`+0x90..`) with `w` = `+0x4c * 0.0625`, and its colour from
    /// `+0xc8`: R, G, B each `+0x3c * (1/32768) * c` (53 bits), A `a *
    /// (1/256)`. Last, `pk +0x4c` and `+0x4d` are `ftol(unit +0x30) / 2` and
    /// `ftol(unit +0x4c) / 2`, at most 0x80 and negative as 0. No caller
    /// reads `eax`.
    #[allow(clippy::many_single_char_names)] // packet fields
    pub fn opus_mdc_light_set<M: Mem>(
        mem: &mut M,
        pk: u32,
        obj: u32,
        unit: u32,
        nos: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let col: u32 = mem.read(pk.wrapping_add(0x24));
        let pos: u32 = mem.read(pk.wrapping_add(0x28));
        let gpr = Global::<u32>::new(0x00c8_f508).ptr(mem).addr;
        for m in [col, pos] {
            for row in 0..4u32 {
                for w in 0..4u32 {
                    let v: u32 = mem.read(gpr + 4 * w);
                    mem.write(m.wrapping_add(0x10 * row + 4 * w), v);
                }
            }
        }
        let get = Global::<u32>::new(OPUS_NO_GET).ptr(mem).addr;
        let k16: f64 = mem.read(Global::<u32>::new(0x00b9_21e0).ptr(mem).addr);
        let k15: f64 = mem.read(Global::<u32>::new(0x00b1_70b0).ptr(mem).addr);
        let k8: f64 = mem.read(Global::<u32>::new(0x00b5_0198).ptr(mem).addr);
        for i in 0..4u32 {
            let no: u16 = mem.read(nos.wrapping_add(2 + 2 * i));
            if no == 0 {
                continue;
            }
            let l = call(mem, get, &[obj, unit, u32::from(no)]);
            let (p, c) = (pos.wrapping_add(0x10 * i), col.wrapping_add(0x10 * i));
            let xyz: [u32; 3] = mem.read(l.wrapping_add(0x90));
            mem.write(p, xyz);
            let r: f32 = mem.read(l.wrapping_add(0x4c));
            mem.write(
                p.wrapping_add(0xc),
                Reg::load_f32(r).constant(MUL, k16).store(),
            );
            let rgba: [u8; 4] = mem.read(l.wrapping_add(0xc8));
            let s: f32 = mem.read(l.wrapping_add(0x3c));
            // (s * 1/32768) at 53 bits, times the channel (exact as f32)
            let sk = Reg::load_f32(s).constant(MUL, k15);
            for (j, &ch) in rgba[..3].iter().enumerate() {
                let v = sk.mem(MUL, f32::from(ch)).store();
                #[allow(clippy::cast_possible_truncation)] // 0..3
                mem.write(c.wrapping_add(4 * j as u32), v);
            }
            let a = Reg::load_f32(f32::from(rgba[3])).constant(MUL, k8).store();
            mem.write(c.wrapping_add(0xc), a);
        }
        for (src, dst) in [(0x30u32, 0x4cu32), (0x4c, 0x4d)] {
            let x: f32 = mem.read(unit.wrapping_add(src));
            let v =
                ffx_bridge::x87::ftol(mem, crate::x87::Reg::load_f32(x).value()).cast_signed() / 2;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=0x80
            let b = v.clamp(0, 0x80) as u8;
            mem.write(pk.wrapping_add(dst), b);
        }
    }

    /// `opus_get_weapon_index(obj)` (`0x00801000`): its slot, or -1.
    pub fn opus_get_weapon_index(mem: &impl Mem, obj: u32) -> i32 {
        (0..4)
            .find(|&i| WEAPONS.ptr(mem).at(i).get(mem) == obj)
            .map_or(-1, u32::cast_signed)
    }

    /// `g_OpusWeaponNos`: the weapon numbers of the four weapon objects
    /// (bytes, 0xff free).
    pub const WEAPON_NOS: Global<u8> = Global::new(0x0119_fe9c);

    /// `opus_get_weapon_index_by_no(no)` (`0x00801030`): the slot whose
    /// weapon number is `no`, or -1.
    pub fn opus_get_weapon_index_by_no(mem: &impl Mem, no: u32) -> i32 {
        (0..4)
            .find(|&i| u32::from(WEAPON_NOS.ptr(mem).at(i).get(mem)) == no)
            .map_or(-1, u32::cast_signed)
    }

    /// `opus_get_weapon_by_no(no)` (`0x00800fd0`): the object of the slot
    /// whose weapon number is `no`, or 0.
    pub fn opus_get_weapon_by_no(mem: &impl Mem, no: u32) -> u32 {
        (0..4)
            .find(|&i| u32::from(WEAPON_NOS.ptr(mem).at(i).get(mem)) == no)
            .map_or(0, |i| WEAPONS.ptr(mem).at(i).get(mem))
    }

    /// `opus_get_monster_index(obj)` (`0x00800e80`): its slot, or -1 (also
    /// for null).
    pub fn opus_get_monster_index(mem: &impl Mem, obj: u32) -> i32 {
        if obj == 0 {
            return -1;
        }
        (0..8)
            .find(|&i| MONSTERS.ptr(mem).at(i).get(mem) == obj)
            .map_or(-1, u32::cast_signed)
    }

    /// `opus_draw_off(obj, part)` (`0x00800d10`): takes the first `part`
    /// (16 bits) out of the draw list.
    pub fn opus_draw_off(mem: &mut impl Mem, obj: u32, part: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original compares 16 bits
        let part = part as i16;
        let mut p = Ptr::<i16>::new(mem.read(obj.wrapping_add(0x1c)));
        loop {
            let v = p.get(mem);
            if v < 0 {
                return;
            }
            if v == part {
                break;
            }
            p = p.at(1);
        }
        loop {
            let next = p.at(1).get(mem);
            if next < 0 {
                break;
            }
            p.set(mem, next);
            p = p.at(1);
        }
        p.set(mem, -1);
    }

    /// `opus_get_osu_by_draw(obj, like, kind, any)` (`0x00800f60`): the
    /// first drawn part of draw kind `kind` (a byte) and, unless `any`, of
    /// `like`'s group; 0 if none. A null `obj` means the battle's object
    /// with its first part and `any`.
    pub fn opus_get_osu_by_draw(mem: &impl Mem, obj: u32, like: u32, kind: u32, any: u32) -> u32 {
        let (obj, like, any) = if obj == 0 {
            let b = OPUS_BATTLE.get(mem);
            (b, mem.read(b.wrapping_add(0x20)), 1)
        } else {
            (obj, like, any)
        };
        let group = mem.read::<u8>(like.wrapping_add(0x1e));
        let parts = mem.read::<u32>(obj.wrapping_add(0x20));
        let mut list = Ptr::<i16>::new(mem.read(obj.wrapping_add(0x1c)));
        #[allow(clippy::cast_possible_truncation)] // the original compares a byte
        let kind = kind as u8;
        loop {
            let n = list.get(mem);
            if n < 0 {
                return 0;
            }
            let part = (i32::from(n) << 8).cast_unsigned().wrapping_add(parts);
            if mem.read::<u8>(part.wrapping_add(0xbb)) == kind
                && (any != 0 || mem.read::<u8>(part.wrapping_add(0x1e)) == group)
            {
                return part;
            }
            list = list.at(1);
        }
    }

    /// `opus_fullauto_no_get(obj)` (`0x00800df0`): steps the `i16` at `+2`,
    /// which wraps to -0x8000 from -16 up, and returns the value before
    /// the step, zero-extended (`opu_et_unit` takes it as its number).
    pub fn opus_fullauto_no_get(mem: &mut impl Mem, obj: u32) -> u32 {
        let p = Ptr::<i16>::new(obj.wrapping_add(2));
        let old = p.get(mem);
        let v = old.wrapping_add(1);
        p.set(mem, if v >= -16 { i16::MIN } else { v });
        u32::from(old.cast_unsigned())
    }

    /// `opu_wait(obj, cmd)` (`0x0080c500`): `+0x210` = the `u16` at
    /// `+0x21a` over 512.
    pub fn opu_wait(mem: &mut impl Mem, obj: u32, cmd: u32) -> u32 {
        let v = mem.read::<u16>(obj.wrapping_add(0x21a)) >> 9;
        #[allow(clippy::cast_possible_truncation)] // below 128
        mem.write(obj.wrapping_add(0x210), v as u8);
        cmd.wrapping_add(2)
    }

    /// `opu_part_kill(host, cmd)` (`0x0080b7d0`): the host's part driver
    /// (`[host +0x350]`) is told through its function at `+0x54` to end the
    /// current unit (`host +0x220`). Returns `cmd + 2`.
    pub fn opu_part_kill<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let driver: u32 = mem.read(host.wrapping_add(0x350));
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let f: u32 = mem.read(driver.wrapping_add(0x54));
        call(mem, f, &[unit]);
        cmd.wrapping_add(2)
    }

    /// `opu_local_link(host, cmd)` (`0x00801a50`): for flag nibble 0 (the
    /// top four bits of `u16 host +0x21a`) the unit's link `u16 +0xb8` =
    /// the command's `+2` and bit 7 of its byte `+0x19` cleared; for 1 the
    /// same with the bit set; other nibbles change nothing. Returns `cmd +
    /// 4`.
    pub fn opu_local_link(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let kind = mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000;
        if kind == 0 || kind == 0x1000 {
            let v: u16 = mem.read(cmd.wrapping_add(2));
            let b: u8 = mem.read(unit.wrapping_add(0x19));
            let b = if kind == 0 { b & 0x7f } else { b | 0x80 };
            mem.write(unit.wrapping_add(0x19), b);
            mem.write(unit.wrapping_add(0xb8), v);
        }
        cmd.wrapping_add(4)
    }

    /// `opu_etc(host, cmd)` (`0x00816610`): the unit's work area at `+0xa8`
    /// plus the command's `i16 +2` takes a value from `+4` by the flag
    /// nibble: 0 a byte, 1 a `u16` (both return `cmd + 6`), 2 a word
    /// (`cmd + 8`). Other nibbles write nothing and return `cmd` (the
    /// interpreter would run it again).
    pub fn opu_etc(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let off = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let at = off.wrapping_add(unit).wrapping_add(0xa8);
        match mem.read::<u16>(host.wrapping_add(0x21a)) >> 12 {
            0 => {
                let v: u8 = mem.read(cmd.wrapping_add(4));
                mem.write(at, v);
                cmd.wrapping_add(6)
            }
            1 => {
                let v: u16 = mem.read(cmd.wrapping_add(4));
                mem.write(at, v);
                cmd.wrapping_add(6)
            }
            2 => {
                let v: u32 = mem.read(cmd.wrapping_add(4));
                mem.write(at, v);
                cmd.wrapping_add(8)
            }
            _ => cmd,
        }
    }

    /// `opu_jloop(host, cmd)` (`0x00816870`): the unit's loop counter
    /// (byte `+0xb4` + the flag nibble, set by `opu_sloop`) down by one;
    /// while not 0 back by the command's `i16 +2` (from `cmd`), else `cmd +
    /// 4`.
    pub fn opu_jloop(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let k = u32::from(mem.read::<u16>(host.wrapping_add(0x21a)) >> 12);
        let at = k.wrapping_add(unit).wrapping_add(0xb4);
        let c = mem.read::<u8>(at).wrapping_sub(1);
        mem.write(at, c);
        if c == 0 {
            return cmd.wrapping_add(4);
        }
        i32::from(mem.read::<i16>(cmd.wrapping_add(2)))
            .cast_unsigned()
            .wrapping_add(cmd)
    }

    /// `opu_unit_max(host, cmd)` (`0x00817530`): the unit count `u16 host
    /// +0x334` becomes the command's `u16 +2`; each unit from the old count
    /// up to the new one (0x100 bytes from `[host +0x320]`, while the
    /// difference as an `i16` stays positive) has its four program
    /// pointers cleared. Returns `cmd + 4`.
    pub fn opu_unit_max(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let n: u16 = mem.read(cmd.wrapping_add(2));
        let old: u16 = mem.read(host.wrapping_add(0x334));
        let units: u32 = mem.read(host.wrapping_add(0x320));
        let mut at = (u32::from(old) << 8).wrapping_add(units);
        let mut left = n.wrapping_sub(old).cast_signed();
        while left > 0 {
            mem.write(at, [0u32; 4]);
            at = at.wrapping_add(0x100);
            left -= 1;
        }
        mem.write(host.wrapping_add(0x334), n);
        cmd.wrapping_add(4)
    }

    /// `0x00800ab0(dst, src)` and `0x00800af0(src, dst)`: 0x400 bytes
    /// copied word by word upward (the save and restore of a 1 KB block
    /// around `0x00826950` in `opu_motion_etc`). Return `src + 0x400`.
    pub fn copy_1k(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
        for i in 0..0x100u32 {
            let v: u32 = mem.read(src.wrapping_add(4 * i));
            mem.write(dst.wrapping_add(4 * i), v);
        }
        src.wrapping_add(0x400)
    }

    /// The `f64` constant 1/16 (`0x00b921e0`).
    const SIXTEENTH: f64 = 0.0625;

    /// `ftol(angle) & 0xfff` and the two table entries the `opf_cir*`
    /// callbacks read: `0x00c43bf0[i + 0x400]` (= `g_pppSinTable[i]`) and
    /// `g_pppSinTable[(i + 0x400) & 0xfff]`.
    fn opf_sin_cos(mem: &impl Mem, angle: f32) -> (f32, f32) {
        let a = ffx_bridge::x87::ftol(mem, Reg::load_f32(angle).value());
        let i = (a & 0xfff) + 0x400;
        let t1 = f(mem, g(mem, 0x00c4_3bf0).wrapping_add(4 * i));
        let t2 = f(mem, g(mem, 0x00c4_4bf0).wrapping_add(4 * (i & 0xfff)));
        (t1, t2)
    }

    /// The circle of `opf_cir`, `opf_cir2` and `opf_cir_y` around the
    /// base unit `b`: angle `+0x48`, radius `+0x40`, height `+0x44`:
    /// `+0x90 = t1 r / 16 + b.x`, `+0x98 = t2 r / 16 + b.z`, `+0x94 =
    /// y / 16` plus `b.y` when `add_y`.
    fn opf_circle(mem: &mut impl Mem, unit: u32, base: u32, add_y: bool) {
        let (t1, t2) = opf_sin_cos(mem, f(mem, unit.wrapping_add(0x48)));
        let radius = f(mem, unit.wrapping_add(0x40));
        let x = Reg::load_f32(t1)
            .mem(MUL, radius)
            .constant(MUL, SIXTEENTH)
            .reg(ADD, Reg::load_f32(f(mem, base.wrapping_add(0x90))))
            .store();
        mem.write(unit.wrapping_add(0x90), x);
        let z = Reg::load_f32(t2)
            .mem(MUL, radius)
            .constant(MUL, SIXTEENTH)
            .mem(ADD, f(mem, base.wrapping_add(0x98)))
            .store();
        mem.write(unit.wrapping_add(0x98), z);
        let y = Reg::Num(SIXTEENTH).mem(MUL, f(mem, unit.wrapping_add(0x44)));
        let y = if add_y {
            y.mem(ADD, f(mem, base.wrapping_add(0x94)))
        } else {
            y
        };
        mem.write(unit.wrapping_add(0x94), y.store());
    }

    /// `opf_cir(host, ptr)` (`0x00814c90`): the unit circles the unit
    /// `opus_no_get(host +0x300, unit, u16 +0xa0)` (see [`opf_circle`]).
    /// Returns `ptr`.
    pub fn opf_cir(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let no = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let b = opus_no_get(mem, host.wrapping_add(0x300), unit, no);
        opf_circle(mem, unit, b, true);
        ptr
    }

    /// `opf_cir_y(host, ptr)` (`0x00814f00`): as [`opf_cir`] with the
    /// height not offset by the base's.
    pub fn opf_cir_y(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let no = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let b = opus_no_get(mem, host.wrapping_add(0x300), unit, no);
        opf_circle(mem, unit, b, false);
        ptr
    }

    /// `opf_cir2(host, ptr)` (`0x00814d20`): as [`opf_cir`] around the unit
    /// of `u16 +0xa0`; then with the horizontal distance `d` from it to the
    /// unit of `u16 +0xa2` (`sqrt |dx^2 + dz^2|`, each step rounded to
    /// `f32`), bit 0x2000 of `u16 +0xb0` is cleared when `+0x30 / 16 + d`
    /// (`f32`) is above `+0x40 / 16`, else set (also for a NaN).
    pub fn opf_cir2(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let list = host.wrapping_add(0x300);
        let n0 = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let n1 = u32::from(mem.read::<u16>(unit.wrapping_add(0xa2)));
        let base = opus_no_get(mem, list, unit, n0);
        let other = opus_no_get(mem, list, unit, n1);
        opf_circle(mem, unit, base, true);
        let dx = crate::x87::sub(
            f(mem, other.wrapping_add(0x90)),
            f(mem, base.wrapping_add(0x90)),
        );
        let dz = crate::x87::sub(
            f(mem, other.wrapping_add(0x98)),
            f(mem, base.wrapping_add(0x98)),
        );
        let (dz, dx) = (Reg::load_f32(dz), Reg::load_f32(dx));
        let d2 = dx.reg(MUL, dx).reg(ADD, dz.reg(MUL, dz)).store();
        // fabs, _CIsqrt, fstp m32
        let dist = match Reg::load_f32(d2) {
            Reg::Num(v) => Reg::Num(ffx_bridge::x87::sqrt(v.abs())),
            Reg::Nan(bits) => Reg::Nan(bits & 0x7fff_ffff),
        }
        .store();
        let reach = Reg::load_f32(f(mem, unit.wrapping_add(0x30)))
            .constant(MUL, SIXTEENTH)
            .mem(ADD, dist)
            .store();
        let lim = Reg::load_f32(f(mem, unit.wrapping_add(0x40))).constant(MUL, SIXTEENTH);
        let above =
            matches!((Reg::load_f32(reach), lim), (Reg::Num(v), Reg::Num(limit)) if v > limit);
        let fl: u16 = mem.read(unit.wrapping_add(0xb0));
        let fl = if above { fl & 0xdfff } else { fl | 0x2000 };
        mem.write(unit.wrapping_add(0xb0), fl);
        ptr
    }

    /// The circle of `opf_cir_gw` and `opf_cirz_gw`: the angle, radius and
    /// height are the unit's floats at the signed byte offsets `+0xa0`,
    /// `+0xa1` and `+0xa2` (0.0 for a 0 offset); `in_x` puts the sine
    /// term on `+0x90`/`+0x40` (else on `+0x94`/`+0x44`).
    fn opf_circle_gw(mem: &mut impl Mem, unit: u32, in_x: bool) {
        let at = |mem: &dyn Fn(u32) -> i8, o: u32| {
            i32::from(mem(unit.wrapping_add(o)))
                .cast_unsigned()
                .wrapping_add(unit)
        };
        let byte = |a: u32| mem.read::<i8>(a);
        let (pa, pb) = (at(&byte, 0xa0), at(&byte, 0xa1));
        let pc = (byte(unit.wrapping_add(0xa2)) != 0).then(|| at(&byte, 0xa2));
        let angle = crate::x87::load_f32(f(mem, pa));
        let r = crate::x87::load_f32(f(mem, pb));
        let h = pc.map_or(0.0, |p| crate::x87::load_f32(f(mem, p)));
        let (t1, t2) = opf_sin_cos(mem, angle);
        // the sine term's axis, the cosine term's axis, the height's axis
        let (s_at, c_at, h_at) = if in_x {
            (0x40u32, 0x48u32, 0x44u32)
        } else {
            (0x44, 0x40, 0x48)
        };
        let v = Reg::load_f32(t1)
            .reg(MUL, Reg::load_f32(r))
            .reg(ADD, Reg::load_f32(f(mem, unit.wrapping_add(s_at))))
            .constant(MUL, SIXTEENTH)
            .store();
        mem.write(unit.wrapping_add(s_at + 0x50), v);
        let v = Reg::load_f32(r)
            .mem(MUL, t2)
            .mem(ADD, f(mem, unit.wrapping_add(c_at)))
            .constant(MUL, SIXTEENTH)
            .store();
        mem.write(unit.wrapping_add(c_at + 0x50), v);
        let v = Reg::load_f32(f(mem, unit.wrapping_add(h_at)))
            .mem(ADD, h)
            .constant(MUL, SIXTEENTH)
            .store();
        mem.write(unit.wrapping_add(h_at + 0x50), v);
    }

    /// `opf_cir_gw(host, ptr)` (`0x00814e50`): the unit circles its own
    /// `+0x40` (x) and `+0x48` (z): `+0x90 = (t1 r + x) / 16`, `+0x98 =
    /// (r t2 + z) / 16`, `+0x94 = (+0x44 + h) / 16`, the angle, radius and
    /// height `h` read at the unit's signed byte offsets `+0xa0..+0xa2`
    /// (see [`opf_circle_gw`]). Returns `ptr`.
    pub fn opf_cir_gw(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        opf_circle_gw(mem, unit, true);
        ptr
    }

    /// `opf_cirz_gw(host, ptr)` (`0x00814fa0`): as [`opf_cir_gw`] with the
    /// circle in y and x: `+0x94` from `+0x44` and the sine, `+0x90` from
    /// `+0x40` and the cosine, the height on `+0x98` from `+0x48`.
    pub fn opf_cirz_gw(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        opf_circle_gw(mem, unit, false);
        ptr
    }

    /// `opf_sph(host, ptr)` (`0x008155b0`): the unit on a sphere around the
    /// unit `opus_no_get(host +0x300, unit, u16 +0xa0)`: the host's matrix
    /// (`+0`) = identity turned by the unit's angles `+0x30` and moved to
    /// that unit's `+0x90` (`op_unit_m`, `op_rt_m`); `host +0x40` = the
    /// unit's `+0x40` / 16 (`op_xyz_scale`) through that matrix
    /// (`op_apply_m_w1`), copied to the emulated `v0` (`0x00c8f528`) and
    /// to the unit's `+0x90..+0x98` (the first two at the byte offset
    /// `g_pppColorConShift`, which is then cleared). Returns `ptr`.
    pub fn opf_sph(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        use crate::op::bridge as op;
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let no = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let base = opus_no_get(mem, host.wrapping_add(0x300), unit, no);
        crate::small::bridge::op_unit_m(mem, host);
        op::op_rt_m_zyx(
            mem,
            host,
            host,
            unit.wrapping_add(0x30),
            base.wrapping_add(0x90),
        );
        let k: u32 = mem.read(g(mem, 0x00b5_8354));
        let v = host.wrapping_add(0x40);
        op::op_xyz_scale(mem, v, unit.wrapping_add(0x40), crate::x87::load(k));
        op::op_apply_m_w1(mem, v, host, v);
        let w: [u32; 4] = mem.read(v);
        mem.write(g(mem, 0x00c8_f528), w);
        let shift = crate::rand::bridge::COLOR_CON_SHIFT.get(mem);
        mem.write(shift.wrapping_add(unit).wrapping_add(0x90), w[0]);
        let shift = crate::rand::bridge::COLOR_CON_SHIFT.get(mem);
        let w1: u32 = mem.read(g(mem, 0x00c8_f52c));
        mem.write(shift.wrapping_add(unit).wrapping_add(0x94), w1);
        let w2: u32 = mem.read(g(mem, 0x00c8_f530));
        crate::rand::bridge::COLOR_CON_SHIFT.set(mem, 0);
        mem.write(unit.wrapping_add(0x98), w2);
        ptr
    }

    /// `opf_spl_gw(host, ptr)` (`0x00815670`): the unit on the spline of
    /// the unit `opus_no_get(host +0x300, unit, u16 +0xa0)` (its record at
    /// `+0xbc`: `i16` point count `+0`, flag byte `+7`, points `+8`, tangents
    /// `+0xc`) at `t` = the unit's float at the signed byte offset `+0xa2`
    /// / 256 (rounded to `f32`): `op_spline_xyz` into the host's `+0`, then
    /// scaled into the unit's `+0x90` by 16 when the flag is set, else 1
    /// (`op_xyz_scale`). Returns 0.
    pub fn opf_spl_gw(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        use crate::op::bridge as op;
        let _ = ptr;
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let no = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let base = opus_no_get(mem, host.wrapping_add(0x300), unit, no);
        let sp: u32 = mem.read(base.wrapping_add(0xbc));
        let n = i32::from(mem.read::<i16>(sp)).cast_unsigned();
        let off = i32::from(mem.read::<i8>(unit.wrapping_add(0xa2))).cast_unsigned();
        let c: f64 = mem.read(g(mem, 0x00b5_0198));
        let t = Reg::load_f32(f(mem, off.wrapping_add(unit)))
            .constant(MUL, c)
            .store();
        let (pts, tan): (u32, u32) = (mem.read(sp.wrapping_add(8)), mem.read(sp.wrapping_add(0xc)));
        op::op_spline_xyz(mem, host, crate::x87::load_f32(t), pts, tan, n);
        let k = if mem.read::<u8>(sp.wrapping_add(7)) != 0 {
            crate::x87::load(mem.read(g(mem, 0x00b9_231c)))
        } else {
            1.0f32.to_bits()
        };
        op::op_xyz_scale(mem, unit.wrapping_add(0x90), host, k);
        0
    }

    /// The step `opf_rel` and `opf_rel2` share, in the emulated VU
    /// registers: `0x00c8f8d0` = the unit's `+0x40..+0x4c`, `0x00c8f8f0` =
    /// the base point `from` (16 bytes), `0x00c8f900` = `to +0x90..`;
    /// `0x00c8f970` = `to - from` (x, y, z), its `w` side `0x00c8f980..98c`
    /// = 1/256 and `0x00c8f52c..534`; `restoreVf00Register`; then each of
    /// x, y, z times the unit's `+0x40`, times 1/256, plus `from`, each
    /// rounded to `f32`, into `0x00c8f970..978` and `v0` (`0x00c8f528..530`,
    /// `0x00c8f534` = `0x00c8f97c`). The first two also go to the unit's
    /// `+0x90`/`+0x94` at the byte offset `g_pppColorConShift`. Returns
    /// the new `v0` z.
    fn opf_rel_step(mem: &mut impl Mem, unit: u32, from: u32, to: u32) -> u32 {
        let k256 = 0x3b80_0000u32;
        let v0 = g(mem, 0x00c8_f528);
        mem.write(v0, k256);
        let w: [u32; 4] = mem.read(unit.wrapping_add(0x40));
        mem.write(g(mem, 0x00c8_f8d0), w);
        let w: [u32; 4] = mem.read(from);
        mem.write(g(mem, 0x00c8_f8f0), w);
        let w: [u32; 4] = mem.read(to.wrapping_add(0x90));
        mem.write(g(mem, 0x00c8_f900), w);
        let (vf, base, dst) = (
            g(mem, 0x00c8_f970),
            g(mem, 0x00c8_f8f0),
            g(mem, 0x00c8_f900),
        );
        let d0 = crate::x87::sub(f(mem, dst), f(mem, base));
        mem.write(vf, d0);
        let d1 = crate::x87::sub(f(mem, dst + 4), f(mem, base + 4));
        let t: u32 = mem.read(v0 + 4);
        mem.write(vf + 0x14, t);
        let t: u32 = mem.read(v0 + 8);
        mem.write(vf + 0x18, t);
        mem.write(vf + 4, d1);
        let d2 = crate::x87::sub(f(mem, dst + 8), f(mem, base + 8));
        let t: u32 = mem.read(v0 + 0xc);
        mem.write(vf + 0x10, k256);
        mem.write(vf + 0x1c, t);
        mem.write(vf + 8, d2);
        ffx_ps2::ee::bridge::restore_vf00(mem);
        for scale in [g(mem, 0x00c8_f8d0), vf + 0x10] {
            let k = Reg::load(mem.read(scale));
            for i in 0..3 {
                let v = k.mem(MUL, f(mem, vf + 4 * i)).store();
                mem.write(vf + 4 * i, v);
            }
        }
        for i in 0..3 {
            let v = crate::x87::add(f(mem, vf + 4 * i), f(mem, base + 4 * i));
            mem.write(vf + 4 * i, v);
            mem.write(v0 + 4 * i, v);
        }
        let t: u32 = mem.read(vf + 0xc);
        mem.write(v0 + 0xc, t);
        let x: u32 = mem.read(vf);
        let shift = crate::rand::bridge::COLOR_CON_SHIFT.get(mem);
        mem.write(shift.wrapping_add(unit).wrapping_add(0x90), x);
        let shift = crate::rand::bridge::COLOR_CON_SHIFT.get(mem);
        let y: u32 = mem.read(v0 + 4);
        mem.write(shift.wrapping_add(unit).wrapping_add(0x94), y);
        mem.read(v0 + 8)
    }

    /// `opf_rel(host, ptr)` (`0x00815120`): the unit placed between the
    /// units of `u16 +0xa0` and `+0xa2` (`opus_no_get`): their `+0x90`
    /// difference times the unit's `+0x40` / 256 from the first
    /// ([`opf_rel_step`]); `+0x98` from `v0`, `g_pppColorConShift`
    /// cleared. Returns `ptr`.
    pub fn opf_rel(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let list = host.wrapping_add(0x300);
        let n0 = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let n1 = u32::from(mem.read::<u16>(unit.wrapping_add(0xa2)));
        let base = opus_no_get(mem, list, unit, n0);
        let other = opus_no_get(mem, list, unit, n1);
        let z = opf_rel_step(mem, unit, base.wrapping_add(0x90), other);
        crate::rand::bridge::COLOR_CON_SHIFT.set(mem, 0);
        mem.write(unit.wrapping_add(0x98), z);
        ptr
    }

    /// `opf_rel2(host, ptr)` (`0x00815350`): as [`opf_rel`] from the point
    /// the host keeps at `+0` (`+0`, `+4`, `+8` = the unit's `+0xa4`,
    /// `+0x44`, `+0x48`) toward the unit of `u16 +0xa0`, with `v0`'s `y`
    /// 0 to start. Afterwards `v0` = (old `v0` z, `0x00c8f510`, `v0` w,
    /// `0x00c8f514`), the unit's `+0x98` = that z. Returns `ptr`.
    pub fn opf_rel2(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let n0 = u32::from(mem.read::<u16>(unit.wrapping_add(0xa0)));
        let other = opus_no_get(mem, host.wrapping_add(0x300), unit, n0);
        for (k, o) in [0xa4u32, 0x44, 0x48].into_iter().enumerate() {
            let v: u32 = mem.read(unit.wrapping_add(o));
            #[allow(clippy::cast_possible_truncation)] // 0..3
            mem.write(host.wrapping_add(4 * k as u32), crate::x87::load(v));
        }
        let v0 = g(mem, 0x00c8_f528);
        mem.write(v0 + 4, 0u32);
        let z = opf_rel_step(mem, unit, host, other);
        let gpr = g(mem, 0x00c8_f508);
        let (a, b): (u32, u32) = (mem.read(gpr + 8), mem.read(gpr + 0xc));
        let w: u32 = mem.read(v0 + 0xc);
        mem.write(v0 + 8, w);
        mem.write(v0 + 4, a);
        mem.write(v0 + 0xc, b);
        mem.write(v0, z);
        mem.write(unit.wrapping_add(0x98), z);
        crate::rand::bridge::COLOR_CON_SHIFT.set(mem, 0);
        ptr
    }

    /// `opu_vzero(host, cmd)` (`0x0081afa0`): clears parts of the unit's
    /// motion by the flag nibble: 0 the position `+0x30` and `+0x40`
    /// (16 bytes each) to the emulated `r0` (`g_Ee_Gpr`, `0x00c8f508`),
    /// 1 `+0x40..+0x48` and 2 `+0x30..+0x38` to 0; after those three the
    /// unit's motion callback runs (`0x00c492c8[byte +0xba]`, through
    /// `call` with `(host, cmd)`). 4 sets `+0x50..+0x8c` to `r0`, 5 zeroes
    /// `+0x60..+0x68` and `+0x80..+0x88`, 6 `+0x50..+0x58` and
    /// `+0x70..+0x78`; other nibbles change nothing. Returns `cmd + 2`.
    pub fn opu_vzero<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let gpr = g(mem, 0x00c8_f508);
        let zero3 = |mem: &mut M, at: u32| mem.write(unit.wrapping_add(at), [0u32; 3]);
        let copy = |mem: &mut M, at: u32| {
            let r0: [u32; 4] = mem.read(gpr);
            mem.write(unit.wrapping_add(at), r0);
        };
        match mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000 {
            k @ (0 | 0x1000 | 0x2000) => {
                match k {
                    0 => {
                        copy(mem, 0x30);
                        copy(mem, 0x40);
                    }
                    0x1000 => zero3(mem, 0x40),
                    _ => zero3(mem, 0x30),
                }
                let cb = u32::from(mem.read::<u8>(unit.wrapping_add(0xba)));
                let f: u32 = mem.read(g(mem, 0x00c4_92c8) + 4 * cb);
                call(mem, f, &[host, cmd]);
            }
            0x4000 => {
                for at in [0x50, 0x60, 0x70, 0x80] {
                    copy(mem, at);
                }
            }
            0x5000 => {
                zero3(mem, 0x60);
                zero3(mem, 0x80);
            }
            0x6000 => {
                zero3(mem, 0x50);
                zero3(mem, 0x70);
            }
            _ => {}
        }
        cmd.wrapping_add(2)
    }

    /// `op_spline_xyz_table(x, pts, n)` (`0x007ea2d0`), called through
    /// `call`.
    const OP_SPLINE_XYZ_TABLE: u32 = 0x007e_a2d0;
    /// `0x007e7330(dst, a, b)`, called through `call`.
    const OP_RT_TO: u32 = 0x007e_7330;

    /// `opu_spl(host, cmd)` (`0x0081b560`): spline motion, by the flag
    /// nibble.
    /// - 0: a new spline record for the unit (`+0xbc`; the old one freed,
    ///   `op_free`/`op_malloc` on the heap `[host +0x3a8]`): `n` (`i16
    ///   cmd+2`) points of 16 bytes after a 16-byte head (`+0` = `n`,
    ///   `+8` the points, `+0xc` the knots after them, `+7` = 1 when the
    ///   `u16 cmd+6` is 0), each point the `+0x40` (or, with that flag,
    ///   `+0x90`) x, y, z of the unit `opus_no_get` gives for the `u16`s
    ///   at `cmd + i16 cmd+4`; then `op_spline_xyz_table(knots, points,
    ///   n)`. Returns `cmd + 8`.
    /// - 1: the unit's `+0x40` = the spline of the unit `u16 cmd+2` (0:
    ///   itself) at `t = +0x4c / 256` (`op_spline_xyz` into the host's
    ///   `+0`, scaled by 16 when `u16 cmd+4` is set, else 1), and `+0x90`
    ///   = `+0x40` / 16. Returns `cmd + 6`.
    /// - 2: as 1, and a second point at `t + +0x6c` (or `t + 1e-6` for a
    ///   zero `+0x6c`) into the host's `+0x10` (scaled the same only with
    ///   the flag, then / 16); `0x007e7330(host +0x40, +0x90, host +0x10)`
    ///   and the unit's matrix `+0xc0` = `host +0x280` times `host
    ///   +0x40` (`op_mul_m`). Returns `cmd + 6`.
    /// - 3: the motion callback `+0xba` = 0xa, `u16 +0xa0`/`+0xa2` =
    ///   `cmd+2`/`+4`. Returns `cmd + 6`.
    /// - Any other nibble returns `cmd`; the original also pops the
    ///   caller's `esi`, `edi` and `ebx` off the wrong stack slots there
    ///   (not reproduced).
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::similar_names,
        clippy::redundant_guards
    )]
    pub fn opu_spl<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::op::bridge as op;
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let list = host.wrapping_add(0x300);
        let kind = mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000;
        let k256: f64 = mem.read(g(mem, 0x00b5_0198));
        let k16: u32 = mem.read(g(mem, 0x00b9_231c));
        let k16 = crate::x87::load(k16);
        let k_16: u32 = mem.read(g(mem, 0x00b5_8354));
        let k_16 = crate::x87::load(k_16);
        // the unit `u16 cmd+2` names (0: the unit itself) and its spline
        let source = |mem: &M| {
            let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
            if no == 0 {
                unit
            } else {
                opus_no_get(mem, list, unit, no)
            }
        };
        let at_t = |mem: &mut M, sp: u32, out: u32, t: f32| {
            let n = i32::from(mem.read::<i16>(sp)).cast_unsigned();
            let pts: u32 = mem.read(sp.wrapping_add(8));
            let x: u32 = mem.read(sp.wrapping_add(0xc));
            op::op_spline_xyz(mem, out, crate::x87::load_f32(t), pts, x, n);
        };
        match kind {
            0x1000 => {
                let s = source(mem);
                let sp: u32 = mem.read(s.wrapping_add(0xbc));
                let t = Reg::load_f32(f(mem, unit.wrapping_add(0x4c)))
                    .constant(MUL, k256)
                    .store();
                at_t(mem, sp, host, t);
                let k = if mem.read::<u16>(cmd.wrapping_add(4)) != 0 {
                    k16
                } else {
                    1.0f32.to_bits()
                };
                op::op_xyz_scale(mem, unit.wrapping_add(0x40), host, k);
                op::op_xyz_scale(mem, unit.wrapping_add(0x90), unit.wrapping_add(0x40), k_16);
                cmd.wrapping_add(6)
            }
            0 => {
                let heap: u32 = mem.read(host.wrapping_add(0x3a8));
                let old: u32 = mem.read(unit.wrapping_add(0xbc));
                op::op_free(mem, heap, old, call);
                let n: u16 = mem.read(cmd.wrapping_add(2));
                let count = i32::from(n.cast_signed());
                let heap: u32 = mem.read(host.wrapping_add(0x3a8));
                let size = (count.cast_unsigned() << 5).wrapping_add(0x10);
                let rec = op::op_malloc(mem, heap, size, call);
                mem.write(unit.wrapping_add(0xbc), rec);
                mem.write(rec, n);
                let pts = rec.wrapping_add(0x10);
                mem.write(
                    rec.wrapping_add(0xc),
                    (count.cast_unsigned() << 4).wrapping_add(pts),
                );
                mem.write(rec.wrapping_add(8), pts);
                let nos = i32::from(mem.read::<i16>(cmd.wrapping_add(4)))
                    .cast_unsigned()
                    .wrapping_add(cmd);
                let flag: u16 = mem.read(cmd.wrapping_add(6));
                let from = if flag == 0 { 0x40u32 } else { 0x90 };
                let mut i = 0i16;
                while i32::from(i) < count {
                    let k = i32::from(i).cast_unsigned();
                    let no = u32::from(mem.read::<u16>(nos.wrapping_add(2 * k)));
                    let s = opus_no_get(mem, list, unit, no);
                    for c in 0..3u32 {
                        let v: u32 = mem.read(s.wrapping_add(from + 4 * c));
                        mem.write(pts.wrapping_add(16 * k + 4 * c), crate::x87::load(v));
                    }
                    i = i.wrapping_add(1);
                }
                let (x, p): (u32, u32) = (
                    mem.read(rec.wrapping_add(0xc)),
                    mem.read(rec.wrapping_add(8)),
                );
                let f = Global::<u32>::new(OP_SPLINE_XYZ_TABLE).ptr(mem).addr;
                call(mem, f, &[x, p, count.cast_unsigned()]);
                mem.write(rec.wrapping_add(7), u8::from(flag == 0));
                cmd.wrapping_add(8)
            }
            0x2000 => {
                let s = source(mem);
                let sp: u32 = mem.read(s.wrapping_add(0xbc));
                let a = crate::x87::load_f32(f(mem, unit.wrapping_add(0x4c)));
                let t = Reg::load_f32(a).constant(MUL, k256).store();
                at_t(mem, sp, host, t);
                let e: f32 = f(mem, unit.wrapping_add(0x6c));
                let a2 = match Reg::load_f32(e) {
                    Reg::Num(z) if z == 0.0 => {
                        let c: f64 = mem.read(g(mem, 0x00b3_68a8));
                        Reg::load_f32(a).constant(crate::x87::ADD, c).store()
                    }
                    _ => Reg::load_f32(e).mem(crate::x87::ADD, a).store(),
                };
                let t2 = Reg::load_f32(a2).constant(MUL, k256).store();
                let second = host.wrapping_add(0x10);
                at_t(mem, sp, second, t2);
                if mem.read::<u16>(cmd.wrapping_add(4)) == 0 {
                    op::op_xyz_scale(mem, unit.wrapping_add(0x40), host, 1.0f32.to_bits());
                } else {
                    op::op_xyz_scale(mem, unit.wrapping_add(0x40), host, k16);
                    op::op_xyz_scale(mem, second, second, k16);
                }
                op::op_xyz_scale(mem, unit.wrapping_add(0x90), unit.wrapping_add(0x40), k_16);
                op::op_xyz_scale(mem, second, second, k_16);
                let m = host.wrapping_add(0x40);
                let f = Global::<u32>::new(OP_RT_TO).ptr(mem).addr;
                call(mem, f, &[m, unit.wrapping_add(0x90), second]);
                op::op_mul_m(mem, unit.wrapping_add(0xc0), host.wrapping_add(0x280), m);
                cmd.wrapping_add(6)
            }
            0x3000 => {
                mem.write(unit.wrapping_add(0xba), 0xau8);
                let a: [u16; 2] = mem.read(cmd.wrapping_add(2));
                mem.write(unit.wrapping_add(0xa0), a);
                cmd.wrapping_add(6)
            }
            _ => cmd,
        }
    }

    /// `opus_get_bone_vertex(out, chr, bone)` (`0x008054d0`): with bit
    /// 0x8000 the option position `bone & 0x7fff` (`Ch_OptposGet(chr, i,
    /// out)`), else the element's position (`Ch_ElemGetPos(chr, bone, 0,
    /// out)`), through `call`. Returns the callee's result.
    pub fn opus_get_bone_vertex<M: Mem>(
        mem: &mut M,
        out: u32,
        chr: u32,
        bone: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if bone & 0x8000 != 0 {
            let f = g(mem, 0x0083_54f0);
            call(mem, f, &[chr, bone & 0x7fff, out])
        } else {
            let f = g(mem, 0x0083_3d10);
            call(mem, f, &[chr, bone, 0, out])
        }
    }

    /// `opus_cheight_get(chr, battle)` (`0x00800b70`): the character's
    /// height, `MsGetChrHeight(chr)` in battle, else
    /// `Ch_GetFixHeight(g_EtChrHandles[chr])` (through `fcall`), rounded to
    /// `f32` (returned in `ST0`).
    pub fn opus_cheight_get<M: Mem>(
        mem: &mut M,
        chr: u32,
        battle: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> f32 {
        let v = if battle != 0 {
            let f = g(mem, 0x0079_50f0);
            fcall(mem, f, &[chr])
        } else {
            let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
            let f = g(mem, 0x0082_aba0);
            fcall(mem, f, &[h])
        };
        f32::from_bits(crate::x87::load(ffx_bridge::x87::store(v)))
    }

    /// `opus_get_chr_pos(chr, out)` (`0x0081b1e0`): `out` = the emulated
    /// `r0` (16 bytes), then the character's position
    /// (`Ch_GetPos(g_EtChrHandles[chr], out, out + 4, out + 8)`, through
    /// `call`) when it has a handle. Returns the callee's result, or `chr`.
    pub fn opus_get_chr_pos<M: Mem>(
        mem: &mut M,
        chr: u32,
        out: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
        mem.write(out, r0);
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        if h == 0 {
            return chr;
        }
        let f = g(mem, 0x0082_ac90);
        call(mem, f, &[h, out, out.wrapping_add(4), out.wrapping_add(8)])
    }

    /// `0x00802450(head, mats, set, part, tex)` (`opus_mdc_head_set`): a
    /// model draw head: `+0` = the part's `u16 +0xc0`, `+4` = the set's
    /// `+0x28`, `u16 +8` = the part's `+0xc2`, `+0xc`/`+0x10`/`+0x14`/
    /// `+0x1c` = 0, `+0x18` = the part's `+0xc8`, `+0x20`..`+0x2c` = `mats +
    /// 0xc0`, `mats`, `mats + 0x40`, `mats + 0x80`, `+0x34` = the set's
    /// `+0x54`, `+0x38`..`+0x44` = 0, `u16 +0x4c` = 0x8080. For a
    /// non-negative `tex` returns the part's oef (`[set +0x60 + 4 group]`,
    /// group byte `+0x1e`) table-4 entry `tex` (`op_oef2_a_get`), else 0.
    pub fn opus_mdc_head_set(
        mem: &mut impl Mem,
        head: u32,
        mats: u32,
        set: u32,
        part: u32,
        tex: u32,
    ) -> u32 {
        let at = |o: u32| head.wrapping_add(o);
        let v: u32 = mem.read(part.wrapping_add(0xc8));
        mem.write(at(0x18), v);
        mem.write(at(0x1c), 0u32);
        mem.write(at(0x10), [0u32; 2]);
        let v = u32::from(mem.read::<u16>(part.wrapping_add(0xc0)));
        mem.write(at(0), v);
        let v: u32 = mem.read(set.wrapping_add(0x28));
        mem.write(at(4), v);
        mem.write(at(0xc), 0u32);
        mem.write(at(0x40), 0u32);
        mem.write(at(0x38), [0u32; 2]);
        mem.write(at(0x4c), 0x8080u16);
        let v: u32 = mem.read(set.wrapping_add(0x54));
        mem.write(at(0x34), v);
        let v: u16 = mem.read(part.wrapping_add(0xc2));
        mem.write(at(8), v);
        mem.write(at(0x28), mats.wrapping_add(0x40));
        mem.write(at(0x2c), mats.wrapping_add(0x80));
        mem.write(at(0x24), mats);
        mem.write(at(0x20), mats.wrapping_add(0xc0));
        mem.write(at(0x44), 0u32);
        if tex.cast_signed() < 0 {
            return 0;
        }
        let g = u32::from(mem.read::<u8>(part.wrapping_add(0x1e)));
        let oef: u32 = mem.read(set.wrapping_add(0x60 + 4 * g));
        crate::op::bridge::op_oef2_a_get(mem, oef, 4, tex)
    }

    /// `Ch_SetShadeColEffect`, `Ch_SetShadeCol`, `Ch_SetTransparent` and
    /// `MsGetWeaponHandle`, called through `call`.
    const CH_SET_SHADE_COL_EFFECT: u32 = 0x0082_b6e0;
    const CH_SET_SHADE_COL: u32 = 0x0082_b5d0;
    const CH_SET_TRANSPARENT: u32 = 0x0082_b900;
    const MS_GET_WEAPON_HANDLE: u32 = 0x0079_5b40;

    /// The `g_EtChr` record of character `chr` (`opus_get_chr_status_index`;
    /// the record before the table for none).
    fn chr_record(mem: &mut impl Mem, chr: u32) -> u32 {
        let i = crate::et::bridge::opus_get_chr_status_index(mem, chr, 0, 0, &mut |_, _, _| 0);
        crate::et::bridge::CHR
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(0x20))
    }

    /// `opus_chara_status_rgb_set(chr, weapons)` (`0x0080aa00`): the
    /// character's shade colour effect (`Ch_SetShadeColEffect(chr, r, g,
    /// b)`) from its `g_EtChr` record: each channel the product of the bytes
    /// `+0x18..+0x1a` and `+0x10..+0x12` / 16384. With `weapons`, also the
    /// battle weapons of the record's `u16 +0xe` (`MsGetWeaponHandle(n, 0)`
    /// and `(n, 1)`, when present). Callees through `call`.
    pub fn opus_chara_status_rgb_set<M: Mem>(
        mem: &mut M,
        chr: u32,
        weapons: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let rec = chr_record(mem, chr);
        let n = u32::from(mem.read::<u16>(rec.wrapping_add(0xe)));
        let scale: f64 = mem.read(g(mem, 0x00b5_1550));
        let mut rgb = [0u32; 3];
        for (i, c) in (0u32..).zip(rgb.iter_mut()) {
            let status = u32::from(mem.read::<u8>(rec.wrapping_add(0x18 + i)));
            let base = u32::from(mem.read::<u8>(rec.wrapping_add(0x10 + i)));
            #[allow(clippy::cast_precision_loss)] // fild; small products are exact
            let v = (status * base) as f32;
            *c = Reg::load_f32(v).constant(MUL, scale).store().to_bits();
        }
        let set = g(mem, CH_SET_SHADE_COL_EFFECT);
        call(mem, set, &[chr, rgb[0], rgb[1], rgb[2]]);
        if weapons == 0 {
            return;
        }
        let get = g(mem, MS_GET_WEAPON_HANDLE);
        for hand in 0..2 {
            let w = call(mem, get, &[n, hand]);
            if w != 0 {
                call(mem, set, &[w, rgb[0], rgb[1], rgb[2]]);
            }
        }
    }

    /// `opus_chara_rgba_set(chr, rgba, weapons)` (`0x0080a560`): the
    /// character's shade colour and transparency from the bytes of `rgba`,
    /// through the emulated VU0 as the original moves them: the bytes are
    /// unpacked with the emulated `r0` (`0x00c8f508`) into the four words of
    /// `v0` (`0x00c8f528`: byte 0 from `rgba`, the rest from `r0`; plain
    /// bytes when `r0` is 0), each converted to a float, times 1/128, into
    /// `0x00c8f980..98c`; `0x00c8f970..97c` = VF00 minus itself;
    /// `restoreVf00Register`; each value of 255 or more (or a NaN) becomes
    /// 255; then at least `0x00c8f970..` (`Vec4Max`). `Ch_SetShadeCol(chr,
    /// r, g, b, 0)` and `Ch_SetTransparent(chr, a, 0)`, and with `weapons`
    /// the same for the battle weapons of the record's `u16 +0xe` when it
    /// is below 0x20 (`MsGetWeaponHandle(n, 0/1)`). The emulated registers
    /// keep what the original leaves: `v0` = 255.0 and the unpacked words,
    /// `0x00c8f538` and `0x00c8f960` = 1/128, `0x00c8f78c` = 255.0,
    /// `0x00c8f964..96c` = `0x00c8f53c..544`. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opus_chara_rgba_set<M: Mem>(
        mem: &mut M,
        chr: u32,
        rgba: u32,
        weapons: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let rec = chr_record(mem, chr);
        let n = u32::from(mem.read::<u16>(rec.wrapping_add(0xe)));
        // the image's rebase, for the emulated registers' addresses
        let delta = g(mem, 0x00c8_f528).wrapping_sub(0x00c8_f528);
        let r = move |a: u32| a.wrapping_add(delta);
        let (v0, vf, k128) = (r(0x00c8_f528), r(0x00c8_f980), 0x3c00_0000u32);
        let g0: [u8; 4] = mem.read(r(0x00c8_f508));
        let g1: [u8; 4] = mem.read(r(0x00c8_f50c));
        let p = rgba.to_le_bytes();
        let words = [
            [p[0], g0[0], g0[0], g0[1]],
            [p[1], g0[1], g0[2], g0[3]],
            [p[2], g0[2], g1[0], g1[1]],
            [p[3], g0[3], g1[2], g1[3]],
        ]
        .map(u32::from_le_bytes);
        mem.write(v0, words);
        mem.write(r(0x00c8_f538), k128);
        let next: [u32; 3] = mem.read(r(0x00c8_f53c));
        mem.write(r(0x00c8_f964), next);
        mem.write(v0, 0x437f_0000u32);
        mem.write(r(0x00c8_f78c), 255.0f32);
        mem.write(r(0x00c8_f960), k128);
        for (i, w) in (0u32..).zip(words) {
            #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
            let f = w.cast_signed() as f32;
            let v = Reg::load_f32(f).reg(MUL, Reg::load(k128)).store();
            mem.write(vf + 4 * i, v);
        }
        let zero = r(0x00c8_f970);
        let vf00 = r(0x00c0_a004);
        for i in 0..4 {
            let x = Reg::load(mem.read(vf00 + 4 * i));
            mem.write(zero + 4 * i, x.reg(crate::x87::SUB, x).store());
        }
        ffx_ps2::ee::bridge::restore_vf00(mem);
        for i in 0..4 {
            let v = Reg::load(mem.read(vf + 4 * i));
            if !matches!(v, Reg::Num(x) if x < 255.0) {
                mem.write(vf + 4 * i, 255.0f32);
            }
        }
        let k: [u32; 4] = mem.read(zero);
        crate::vec::bridge::vec4_max(mem, vf, vf, k);
        let c: [u32; 4] = mem.read(vf);
        let c = c.map(crate::x87::load);
        let shade = g(mem, CH_SET_SHADE_COL);
        let alpha = g(mem, CH_SET_TRANSPARENT);
        call(mem, shade, &[chr, c[0], c[1], c[2], 0]);
        call(mem, alpha, &[chr, c[3], 0]);
        if weapons == 0 || n >= 0x20 {
            return;
        }
        let get = g(mem, MS_GET_WEAPON_HANDLE);
        for hand in 0..2 {
            let w = call(mem, get, &[n, hand]);
            if w != 0 {
                call(mem, shade, &[w, c[0], c[1], c[2], 0]);
                call(mem, alpha, &[w, c[3], 0]);
            }
        }
    }

    /// `0x00802bd0(head, _, list, unit, nos)` (`opus_mdc_lights_get`): the
    /// four light colours `[head +0x24]` and positions `[head +0x28]` (16
    /// bytes each) start as the emulated `r0`; for each nonzero `u16` at
    /// `nos + 2 k` (k = 1..4) the unit `opus_no_get(list, unit, no)` gives
    /// position `k - 1` (its `+0x90..+0x98`, `w` = its `+0x4c` / 16) and
    /// colour (its bytes `+0xc8..+0xcb` / 128). Then the head's bytes
    /// `+0x4c` and `+0x4d` = `ftol` of the unit's `+0x30` and `+0x4c`, halved
    /// (toward 0), clamped to 0..=0x80.
    pub fn opus_mdc_lights_get(mem: &mut impl Mem, head: u32, list: u32, unit: u32, nos: u32) {
        let cols: u32 = mem.read(head.wrapping_add(0x24));
        let pos: u32 = mem.read(head.wrapping_add(0x28));
        let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
        for k in 0..4 {
            mem.write(cols.wrapping_add(16 * k), r0);
        }
        for k in 0..4 {
            mem.write(pos.wrapping_add(16 * k), r0);
        }
        let k16: f64 = mem.read(g(mem, 0x00b9_21e0));
        let k128: f64 = mem.read(g(mem, 0x00b1_70b8));
        for k in 0..4u32 {
            let no = u32::from(mem.read::<u16>(nos.wrapping_add(2 + 2 * k)));
            if no == 0 {
                continue;
            }
            let rec = opus_no_get(mem, list, unit, no);
            let at = pos.wrapping_add(16 * k);
            let xyz: [u32; 3] = mem.read(rec.wrapping_add(0x90));
            mem.write(at, xyz);
            let w = Reg::load_f32(f(mem, rec.wrapping_add(0x4c)))
                .constant(MUL, k16)
                .store();
            mem.write(at.wrapping_add(0xc), w);
            let col = cols.wrapping_add(16 * k);
            for i in 0..4u32 {
                let b = f64::from(mem.read::<u8>(rec.wrapping_add(0xc8 + i)));
                let v = Reg::Num(b).constant(MUL, k128).store();
                mem.write(col.wrapping_add(4 * i), v);
            }
        }
        for (o, byte) in [(0x30u32, 0x4cu32), (0x4c, 0x4d)] {
            let v = ffx_bridge::x87::ftol(mem, Reg::load_f32(f(mem, unit.wrapping_add(o))).value());
            let half = v.cast_signed() / 2;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=0x80
            let b = half.clamp(0, 0x80) as u8;
            mem.write(head.wrapping_add(byte), b);
        }
    }

    /// `0x00807b70(out, src, n, time, sat, val, mix)` (`opus_hsv_rgba`):
    /// `n` colours from the 8-byte entries at `src` (`i16` hue in 0x300
    /// steps, saturation, value, a byte alpha at `+6`) into the 4-byte RGBA
    /// words at `out`; a zero first word gives 0. Each hue moves toward
    /// the reference hue `ftol(f32(time) / 5.3333335) % 0x300` (the shorter
    /// way round, the reference itself raised by 0x300 for the rest when
    /// it lags) by `mix` / 256 (`mix` clamped to 0..=0x100), wrapped into
    /// 0..0x300; saturation `+ sat` is clamped to 0..=0x100 and value `+
    /// val` to 0..=0xff; then the six 0x80-wide hue sectors pick red,
    /// green and blue from the value and the three products (integer
    /// division toward 0). A hue outside the sectors keeps the previous
    /// colour (initially `mix` as given, `out` and 0; the original's third
    /// is an uninitialised local). Returns the entry after the last (`src`
    /// for `n` <= 0).
    #[allow(clippy::too_many_arguments, clippy::many_single_char_names)]
    pub fn opus_hsv_rgba(
        mem: &mut impl Mem,
        out: u32,
        src: u32,
        n: i32,
        time: i32,
        sat: i32,
        val: i32,
        mix: i32,
    ) -> u32 {
        let k: f64 = mem.read(g(mem, 0x00b5_8950));
        #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
        let t = Reg::load_f32(time as f32).constant(|a, b| a / b, k).store();
        let mut reference = ffx_bridge::x87::ftol(mem, f64::from(t)).cast_signed() % 0x300;
        let weight = mix.clamp(0, 0x100);
        if n < 1 {
            return src;
        }
        // the previous colour, for hues outside the sectors
        let (mut r, mut gr, mut b) = (mix.cast_unsigned(), out, 0u32);
        let (mut e, mut o) = (src, out);
        for _ in 0..n {
            if mem.read::<u32>(e) == 0 {
                mem.write(o, 0u32);
            } else {
                let mut h = i32::from(mem.read::<i16>(e));
                if reference > h {
                    if reference - h >= 0x180 {
                        h += 0x300;
                    }
                } else if h - reference >= 0x180 {
                    reference += 0x300;
                }
                h = h.wrapping_add(reference.wrapping_sub(h).wrapping_mul(weight) / 256);
                if h >= 0x300 {
                    h -= 0x300;
                } else if h < 0 {
                    h += 0x300;
                }
                let s = (i32::from(mem.read::<i16>(e.wrapping_add(2))) + sat).clamp(0, 0x100);
                let v = (i32::from(mem.read::<i16>(e.wrapping_add(4))) + val).clamp(0, 0xff);
                let (sector, frac) = (h / 128, h % 128);
                let p = ((0x100 - s) * v / 256).cast_unsigned();
                let q = (0x8000i32.wrapping_sub(frac.wrapping_mul(s)).wrapping_mul(v) / 0x8000)
                    .cast_unsigned();
                let t2 = (0x8000i32
                    .wrapping_sub((0x80 - frac).wrapping_mul(s))
                    .wrapping_mul(v)
                    / 0x8000)
                    .cast_unsigned();
                let v = v.cast_unsigned();
                (r, gr, b) = match sector {
                    0 => (v, t2, p),
                    1 => (q, v, p),
                    2 => (p, v, t2),
                    3 => (p, q, v),
                    4 => (t2, p, v),
                    5 => (v, p, q),
                    _ => (r, gr, b),
                };
                let a: u8 = mem.read(e.wrapping_add(6));
                #[allow(clippy::cast_possible_truncation)] // the low bytes
                mem.write(o, [r as u8, gr as u8, b as u8, a]);
            }
            e = e.wrapping_add(8);
            o = o.wrapping_add(4);
        }
        e
    }

    /// `opus_mdc_free(oef, chr, heap, second)` (`0x008061a0`): for a
    /// character with a status record (`opus_get_chr_status_index(chr, 0,
    /// 0)`), its mdc number (byte `+0x14`, or `+0x15` with `second`) taken
    /// (set to 0xff), its texture CLUT and image bases reset
    /// (`Ch_TextureSetClutBase`, `Ch_TextureSetImageBase` with -1, -1),
    /// and an mdc it had in `oef`'s table 4 freed (`op_free(heap, it)`,
    /// `op_oef2_a_set(oef, 4, no, 0)`). Callees other than the table lookup
    /// through `call`.
    pub fn opus_mdc_free<M: Mem>(
        mem: &mut M,
        oef: u32,
        chr: u32,
        heap: u32,
        second: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if chr == 0 {
            return;
        }
        let index = g(mem, 0x007f_e970);
        let Ok(i) = u32::try_from(call(mem, index, &[chr, 0, 0]).cast_signed()) else {
            return;
        };
        let rec = crate::et::bridge::CHR.ptr(mem).at(i).addr;
        let at = rec.wrapping_add(if second == 0 { 0x14 } else { 0x15 });
        let no = mem.read::<u8>(at);
        mem.write(at, 0xffu8);
        for f in [0x0083_13c0, 0x0083_1390] {
            let f = g(mem, f);
            call(mem, f, &[chr, u32::MAX, u32::MAX]);
        }
        if no == 0xff {
            return;
        }
        let no = u32::from(no);
        let mdc = crate::op::bridge::op_oef2_a_get(mem, oef, 4, no);
        if mdc == 0 {
            return;
        }
        let free = g(mem, 0x007f_f1d0);
        call(mem, free, &[heap, mdc]);
        let set = g(mem, 0x007e_38a0);
        call(mem, set, &[oef, 4, no, 0]);
    }

    /// `opf_normal(host, ptr)` (`0x00815050`): the motion callback for a
    /// plain unit (slot 0 of `0x00c492c8`): the emulated VU registers
    /// `0x00c8f934..93c` take `0x00c8f53c..544`, `0x00c8f538` and
    /// `0x00c8f930` = 1/16, and the unit's `+0x40..+0x4c` times 1/16 go to
    /// `0x00c8f8e0..8ec` and the unit's `+0x90..+0x9c`. Returns `ptr`.
    pub fn opf_normal(mem: &mut impl Mem, host: u32, ptr: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let saved: [u32; 3] = mem.read(g(mem, 0x00c8_f53c));
        mem.write(g(mem, 0x00c8_f934), saved);
        let k = 0x3d80_0000u32;
        mem.write(g(mem, 0x00c8_f538), k);
        mem.write(g(mem, 0x00c8_f930), k);
        for i in 0..4u32 {
            let x: f32 = mem.read(unit.wrapping_add(0x40 + 4 * i));
            let v = crate::x87::mul(x, f32::from_bits(k));
            mem.write(g(mem, 0x00c8_f8e0) + 4 * i, v);
            mem.write(unit.wrapping_add(0x90 + 4 * i), v);
        }
        ptr
    }

    /// `0x008178f0(host, cmd)` (`opu_eff_flag`): the flags word `+0xc` of
    /// the unit's `g_EtEff` record (`u16 unit +0x14`) against the mask
    /// `i16 +4 << 16 | i16 +2` (both sign-extended), by the flag nibble's
    /// low three bits: 0 set (`cmd + 6`), 1 clear (`cmd + 6`), 2/3 jump by
    /// `i16 +6` when any/none is set, else `cmd + 8`, 4/5 wait (`host
    /// +0x210` = 1, returning `cmd`) while any/none is set, else `cmd + 6`;
    /// 6 and 7 return `cmd`.
    pub fn opu_eff_flag(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let e = u32::from(mem.read::<u16>(unit.wrapping_add(0x14)));
        let at = EFF.ptr(mem).at(e).addr.wrapping_add(0xc);
        let hi = i32::from(mem.read::<i16>(cmd.wrapping_add(4))).cast_unsigned();
        let lo = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let mask = hi << 16 | lo;
        let kind = (i32::from(mem.read::<i16>(host.wrapping_add(0x21a))) >> 12) & 7;
        let flags: u32 = mem.read(at);
        let any = flags & mask != 0;
        let jump = || {
            i32::from(mem.read::<i16>(cmd.wrapping_add(6)))
                .cast_unsigned()
                .wrapping_add(cmd)
        };
        match kind {
            0 => {
                mem.write(at, flags | mask);
                cmd.wrapping_add(6)
            }
            1 => {
                mem.write(at, flags & !mask);
                cmd.wrapping_add(6)
            }
            2 | 3 if any == (kind == 2) => jump(),
            2 | 3 => cmd.wrapping_add(8),
            4 | 5 if any == (kind == 4) => {
                mem.write(host.wrapping_add(0x210), 1u8);
                cmd
            }
            4 | 5 => cmd.wrapping_add(6),
            _ => cmd,
        }
    }

    /// `fldz; fld m32; fsubr st, st(1); fstp m32`: `0 - x` in place.
    fn neg0(mem: &mut impl Mem, at: u32) {
        let x: f32 = mem.read(at);
        let r = Reg::load_f32(0.0).mem(crate::x87::SUB, x).store();
        mem.write(at, r);
    }

    /// `opu_v_rev(host, cmd)` (`0x00819a20`): negates (as `0 - x`, so +0
    /// stays +0) the flagged x, y, z, w (flags 0x1000..0x8000) of the
    /// unit's vector at the `i16` offset `0x00c49388[cmd+2 & 0xf]`; with
    /// bit 15 of `cmd+2` also the flagged x, y, z of the unit's `+0x90`.
    /// Returns `cmd + 4`.
    pub fn opu_v_rev(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let fl: u16 = mem.read(host.wrapping_add(0x21a));
        let arg: u16 = mem.read(cmd.wrapping_add(2));
        let off: i16 = mem.read(g(mem, 0x00c4_9388) + 2 * u32::from(arg & 0xf));
        let v = unit.wrapping_add(i32::from(off).cast_unsigned());
        for (k, bit) in [0x1000u16, 0x2000, 0x4000, 0x8000].into_iter().enumerate() {
            if fl & bit != 0 {
                #[allow(clippy::cast_possible_truncation)] // 0..4
                neg0(mem, v.wrapping_add(4 * k as u32));
            }
        }
        if arg & 0x8000 != 0 {
            for (k, bit) in [0x1000u16, 0x2000, 0x4000].into_iter().enumerate() {
                if fl & bit != 0 {
                    #[allow(clippy::cast_possible_truncation)] // 0..3
                    neg0(mem, unit.wrapping_add(0x90 + 4 * k as u32));
                }
            }
        }
        cmd.wrapping_add(4)
    }

    /// `0x008182f0(host, cmd)` (`opu_v_stop`): the unit's velocity to stop
    /// in `n` frames (`u16 +2`, as `f32`): for flag bits 9..11 = 0 each
    /// flagged x, y, z, w (0x1000..0x8000) of `+0x70` = `-(+0x50 / n)`, for
    /// 1 those of `+0x80` = `-(+0x60 / n)` (`fchs`, so a NaN's sign flips
    /// too). Returns `cmd + 4`.
    pub fn opu_v_stop(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        const DIV: crate::x87::Op = |a, b| a / b;
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        #[allow(clippy::cast_precision_loss)] // fild, fstp m32
        let n = f32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        let fl: u16 = mem.read(host.wrapping_add(0x21a));
        let (from, to) = match fl & 0xe00 {
            0 => (0x50u32, 0x70u32),
            0x200 => (0x60, 0x80),
            _ => return cmd.wrapping_add(4),
        };
        for (k, bit) in [0x1000u16, 0x2000, 0x4000, 0x8000].into_iter().enumerate() {
            // each flag is read again from the host
            if mem.read::<u16>(host.wrapping_add(0x21a)) & bit != 0 {
                #[allow(clippy::cast_possible_truncation)] // 0..4
                let k = 4 * k as u32;
                let x: f32 = mem.read(unit.wrapping_add(from + k));
                let q = Reg::load_f32(x).reg(DIV, Reg::load_f32(n)).store();
                mem.write(unit.wrapping_add(to + k), q.to_bits() ^ 0x8000_0000);
            }
        }
        cmd.wrapping_add(4)
    }

    /// `0x0081b4d0(host, cmd)` (`opu_motion_cb`): the unit's motion
    /// callback byte `+0xba` = byte `cmd+2`, `u16 +0xa0`/`+0xa2` = `cmd+4`
    /// /`+6`; for callbacks 8 and 9 the unit's `+0x40..+0x48` = its
    /// `+0x90..+0x98` times 16, for 0xb `+0xa4`, `+0x44`, `+0x48` take
    /// `+0x90..+0x98` (through the FPU). Returns `cmd + 8`.
    pub fn opu_motion_cb(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let cb: u8 = mem.read(cmd.wrapping_add(2));
        mem.write(unit.wrapping_add(0xba), cb);
        let a: [u16; 2] = mem.read(cmd.wrapping_add(4));
        mem.write(unit.wrapping_add(0xa0), a);
        let cb: u8 = mem.read(unit.wrapping_add(0xba));
        match cb {
            8 | 9 => {
                for k in 0..3u32 {
                    let x: f32 = mem.read(unit.wrapping_add(0x90 + 4 * k));
                    let v = Reg::load_f32(x).constant(MUL, 16.0).store();
                    mem.write(unit.wrapping_add(0x40 + 4 * k), v);
                }
            }
            0xb => {
                for (from, to) in [(0x90u32, 0xa4u32), (0x94, 0x44), (0x98, 0x48)] {
                    let x: u32 = mem.read(unit.wrapping_add(from));
                    mem.write(unit.wrapping_add(to), crate::x87::load(x));
                }
            }
            _ => {}
        }
        cmd.wrapping_add(8)
    }

    /// `opu_et_no(obj, cmd)` (`0x008165d0`): the part at `+0x220` takes its
    /// effect's number (the effect's `u16` at `+0x1a`, bits 4-14) plus the
    /// command's `i16`.
    pub fn opu_et_no(mem: &mut impl Mem, obj: u32, cmd: u32) -> u32 {
        let part = mem.read::<u32>(obj.wrapping_add(0x220));
        let e = u32::from(mem.read::<u16>(part.wrapping_add(0x14)));
        let eff = mem.read::<u32>(EFF.ptr(mem).at(e).addr.wrapping_add(4));
        let v = (mem.read::<u16>(eff.wrapping_add(0x1a)) & 0x7ff0)
            .wrapping_add(mem.read::<u16>(cmd.wrapping_add(2)));
        mem.write(part.wrapping_add(0x1a), v);
        cmd.wrapping_add(4)
    }

    /// `opp_anmm_fin_en_bone3x(_, _, cmd, part)` (`0x00810ed0`): mode 5,
    /// and the command's `i16` as a float at `+0x40`.
    pub fn opp_anmm_fin_en_bone3x(mem: &mut impl Mem, cmd: u32, part: u32) -> u32 {
        let v = mem.read::<i16>(cmd.wrapping_add(2));
        mem.write(part.wrapping_add(0x4e), 5u8);
        mem.write(part.wrapping_add(0x40), f32::from(v));
        cmd.wrapping_add(4)
    }

    /// `opus_monster_index(obj)` (`0x007fe540`): the slot of `obj` among
    /// the 8 `g_OpusMonsters` (null included), or -1.
    pub fn opus_monster_index(mem: &impl Mem, obj: u32) -> i32 {
        (0..8)
            .find(|&i| MONSTERS.ptr(mem).at(i).get(mem) == obj)
            .map_or(-1, u32::cast_signed)
    }

    /// `g_OpusMonsterFlags` (`0x0119fef0`): a byte per monster slot.
    pub const MONSTER_FLAGS: Global<u8> = Global::new(0x0119_fef0);

    fn monster_variant(mem: &impl Mem, slot: u32) -> Option<u32> {
        let f: u8 = mem.read(MONSTER_FLAGS.ptr(mem).addr.wrapping_add(slot));
        (f != 0).then_some(if f & 1 != 0 { 0 } else { 2 })
    }

    /// `opus_monster_clut_pos(slot)` (`0x00800e20`): 0 without flags,
    /// else 0x3fc0 (flag bit 0) or 0x3fd0.
    pub fn opus_monster_clut_pos(mem: &impl Mem, slot: u32) -> u32 {
        monster_variant(mem, slot).map_or(0, |k| k * 8 + 0x3fc0)
    }

    /// `opus_monster_tex_pos(slot)` (`0x00800e50`): 0 without flags,
    /// else 0x3f40 (flag bit 0) or 0x3fc0.
    pub fn opus_monster_tex_pos(mem: &impl Mem, slot: u32) -> u32 {
        monster_variant(mem, slot).map_or(0, |k| (k + 0xfd) << 6)
    }

    /// `opus_osu_init_group(i, n)` (`0x00801480`): the first 16 bytes of the
    /// first `n` 0x100-byte parts of opus object `i` (the table at
    /// `g_OpusBattle`) zeroed.
    pub fn opus_osu_init_group(mem: &mut impl Mem, i: u32, n: u32) {
        let obj: u32 = mem.read(OPUS_BATTLE.ptr(mem).addr.wrapping_add(4 * (i & 0xffff)));
        let mut p: u32 = mem.read(obj.wrapping_add(0x20));
        for _ in 0..(n & 0xffff) {
            mem.write(p, [0u32; 4]);
            p = p.wrapping_add(0x100);
        }
    }

    /// The table `opus_no_get` reads a unit's weapon number from (32-byte
    /// entries, a pointer at `+4`; `0x012a4104`).
    pub const NO_TABLE: Global<u32> = Global::new(0x012a_4104);

    /// `opus_no_get(list, unit, no)` (`0x00801060`): the 0x100-byte unit
    /// of the list (parts at `list +0x20`) that `no` (16 bits) names
    /// relative to `unit`:
    /// - 0xff00: `unit` itself.
    /// - 0xffff, or another negative `-k` outside the ranges below: `k`
    ///   steps up the parent chain (`u16 +0x1c`); that unit, or the first
    ///   part when its first word is null (`is_ptr32_null`).
    /// - 0xb001..=0xcfff: `no + 0x4000 + unit +0x1a`; 0xfe00..=0xfeff: the
    ///   low nibble over `+0x1a & 0x7ff0` of the unit `NO_TABLE[unit
    ///   +0x14]` points at; a non-negative `no` as it is: the unit with
    ///   that number (`u16 +0x1a`) and `unit`'s group (`+0x1e`) in the
    ///   index lists `+0x14`, `+0x10`, `+0x18` (`i16`, ended by a negative
    ///   one), else the first part.
    pub fn opus_no_get(mem: &impl Mem, list: u32, unit: u32, no: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the u16 argument
        let mut n = no as u16;
        let base = |mem: &dyn Fn(u32) -> u32| mem(list.wrapping_add(0x20));
        let rd = |a: u32| mem.read::<u32>(a);
        let up = |at: u32| {
            u32::from(mem.read::<u16>(at.wrapping_add(0x1c)))
                .wrapping_shl(8)
                .wrapping_add(base(&rd))
        };
        if n & 0x8000 != 0 {
            if n == 0xff00 {
                return unit;
            }
            let steps = if n == 0xffff {
                Some(1)
            } else if n.wrapping_add(0x4fff) <= 0x1ffe {
                n = n
                    .wrapping_add(0x4000)
                    .wrapping_add(mem.read::<u16>(unit.wrapping_add(0x1a)));
                None
            } else if n & 0xff00 == 0xfe00 {
                let i = u32::from(mem.read::<u16>(unit.wrapping_add(0x14)));
                let e: u32 = mem.read(NO_TABLE.ptr(mem).addr.wrapping_add(32 * i));
                n = (mem.read::<u16>(e.wrapping_add(0x1a)) & 0x7ff0) | (n & 0xf);
                None
            } else {
                Some(n.wrapping_neg())
            };
            if let Some(k) = steps {
                let mut u = unit;
                for _ in 0..k {
                    u = up(u);
                }
                return if ffx_system::stub::is_ptr32_null(rd(u)) == 0 {
                    u
                } else {
                    base(&rd)
                };
            }
        }
        let want = i32::from(n.cast_signed());
        let group: u8 = mem.read(unit.wrapping_add(0x1e));
        for at in [0x14u32, 0x10, 0x18] {
            let mut e: u32 = mem.read(list.wrapping_add(at));
            loop {
                let i: i16 = mem.read(e);
                if i < 0 {
                    break;
                }
                let u = (i32::from(i) << 8).cast_unsigned().wrapping_add(base(&rd));
                if i32::from(mem.read::<u16>(u.wrapping_add(0x1a))) == want
                    && mem.read::<u8>(u.wrapping_add(0x1e)) == group
                {
                    return u;
                }
                e = e.wrapping_add(2);
            }
        }
        base(&rd)
    }

    /// `opus_parent_get(obj, unit)` (`0x00801f00`): the matrix (`+0xc0`)
    /// of the unit `u16 unit +0xb8` names, among the object's units (parts
    /// at `obj +0x320`, index lists `+0x314`, `+0x310`, `+0x318`): 0 gives
    /// the object's own matrix (`obj + 0x280`), 0xff00 `unit`'s, a
    /// negative `-k` the unit `k` steps up the parent chain (`u16
    /// +0x1c`), 0xfe00..=0xfeff the number from `NO_TABLE` as in
    /// `opus_no_get`, a non-negative one the unit with that number (`u16
    /// +0x1a`), else the first unit's.
    pub fn opus_parent_get(mem: &impl Mem, obj: u32, unit: u32) -> u32 {
        let mut n: u16 = mem.read(unit.wrapping_add(0xb8));
        let base = || mem.read::<u32>(obj.wrapping_add(0x320));
        if n == 0 {
            return obj.wrapping_add(0x280);
        }
        if n & 0x8000 != 0 {
            if n == 0xff00 {
                return unit.wrapping_add(0xc0);
            }
            if n & 0xff00 == 0xfe00 && n != 0xffff {
                let i = u32::from(mem.read::<u16>(unit.wrapping_add(0x14)));
                let e: u32 = mem.read(NO_TABLE.ptr(mem).addr.wrapping_add(32 * i));
                n = (mem.read::<u16>(e.wrapping_add(0x1a)) & 0x7ff0) | (n & 0xf);
            } else {
                let mut u = unit;
                for _ in 0..n.wrapping_neg() {
                    u = u32::from(mem.read::<u16>(u.wrapping_add(0x1c)))
                        .wrapping_shl(8)
                        .wrapping_add(base());
                }
                return u.wrapping_add(0xc0);
            }
        }
        let want = i32::from(n.cast_signed());
        for at in [0x314u32, 0x310, 0x318] {
            let mut e: u32 = mem.read(obj.wrapping_add(at));
            loop {
                let i: i16 = mem.read(e);
                if i < 0 {
                    break;
                }
                let u = (i32::from(i) << 8).cast_unsigned().wrapping_add(base());
                if i32::from(mem.read::<u16>(u.wrapping_add(0x1a))) == want {
                    return u.wrapping_add(0xc0);
                }
                e = e.wrapping_add(2);
            }
        }
        base().wrapping_add(0xc0)
    }

    /// `opu_end(obj, cmd)` (`0x0080c3f0`): unless the object is held
    /// (`u16 +0x216`), its part (`+0x220`) is cleared (first 16 bytes);
    /// if its `g_EtEff` record (`EFF[u16 part +0x14]`) still points at it
    /// with a type nibble of 4, the object waits (`+0x210` = 0x78) and
    /// `cmd` comes back; otherwise unit `u16 +0x214` of the list `obj +
    /// 0x300` is shut down (`opus_prog_off`, `opus_draw_off`,
    /// `yonExtMapDeleteId`, through `call`) and `obj +0x218` decremented.
    /// Then `+0x210` = 1 and 0 is returned.
    pub fn opu_end<M: Mem>(
        mem: &mut M,
        obj: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u16>(obj.wrapping_add(0x216)) == 0 {
            let part: u32 = mem.read(obj.wrapping_add(0x220));
            mem.write(part, [0u32; 4]);
            let i = u32::from(mem.read::<u16>(part.wrapping_add(0x14)));
            let e = EFF.ptr(mem).at(i).addr;
            if mem.read::<u32>(e + 4) == part && mem.read::<u32>(e) & 0xf000_0000 == 0x4000_0000 {
                mem.write(obj.wrapping_add(0x210), 0x78u8);
                return cmd;
            }
            let list = obj.wrapping_add(0x300);
            let no = u32::from(mem.read::<u16>(obj.wrapping_add(0x214)));
            for (f, args) in [(0x0080_14c0u32, [list, no]), (0x0080_0d10, [list, no])] {
                let f = Global::<u32>::new(f).ptr(mem).addr;
                call(mem, f, &args);
            }
            let id = i32::from(mem.read::<i16>(obj.wrapping_add(0x214))).cast_unsigned();
            let f = Global::<u32>::new(0x0080_7830).ptr(mem).addr;
            call(mem, f, &[id]);
            let n: u16 = mem.read(obj.wrapping_add(0x218));
            mem.write(obj.wrapping_add(0x218), n.wrapping_sub(1));
        }
        mem.write(obj.wrapping_add(0x210), 1u8);
        0
    }

    /// `opus_chr_width_00800cd0(chr, battle)` (`0x00800cd0`): a character's
    /// width / 6: `MsGetChrWidth(chr)` when `battle` is set, else
    /// `Ch_GetThickness(g_EtChrHandles[chr])` (both float results, through
    /// `fcall`), stored as `f32` before and after the division. Returns
    /// it in `ST0`.
    pub fn opus_chr_width<M: Mem>(
        mem: &mut M,
        chr: u32,
        battle: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> f32 {
        use crate::x87::Reg;
        let w = if battle == 0 {
            let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
            let f = Global::<u32>::new(0x0082_ad70).ptr(mem).addr;
            fcall(mem, f, &[h])
        } else {
            let f = Global::<u32>::new(0x0079_5a20).ptr(mem).addr;
            fcall(mem, f, &[chr])
        };
        let div: crate::x87::Op = |a, b| a / b;
        let six: f64 = mem.read(Global::<u32>::new(0x00b3_eb88).ptr(mem).addr);
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let w = w as f32;
        Reg::load_f32(w).constant(div, six).store()
    }

    /// `opus_clip_off_all_in_battle()` (`0x00800bb0`): when
    /// `MsBattleChrStart()` is not 0, every character handle in the first
    /// 32 `g_EtChrHandles` slots gets `Ch_EnClip(h, 0)` (through `call`).
    /// Returns what the original leaves in `eax`: the last call's result,
    /// or the last handle read.
    pub fn opus_clip_off_all_in_battle<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = Global::<u32>::new(0x0078_0cb0).ptr(mem).addr;
        let mut eax = call(mem, f, &[]);
        if eax == 0 {
            return 0;
        }
        let clip = Global::<u32>::new(0x0082_aa30).ptr(mem).addr;
        for i in 0..32 {
            let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(i).get(mem);
            eax = h;
            if h != 0 {
                eax = call(mem, clip, &[h, 0]);
            }
        }
        eax
    }

    /// `opus_cscale_get(chr, battle)` (`0x00800c10`): the larger of a
    /// character's height and width (`MsGetChrHeight`/`MsGetChrWidth(chr)`
    /// in battle, else `Ch_GetFixHeight`/`Ch_GetThickness` of
    /// `g_EtChrHandles[chr]`; float results through `fcall`, each stored
    /// as `f32`; the height on a tie or NaN) divided by the `f64` at
    /// `0x00b587d8`. Returns it in `ST0`.
    #[allow(clippy::many_single_char_names)]
    pub fn opus_cscale_get<M: Mem>(
        mem: &mut M,
        chr: u32,
        battle: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> f32 {
        use crate::x87::Reg;
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        let (fa, fb, arg) = if battle == 0 {
            (0x0082_aba0u32, 0x0082_ad70u32, h)
        } else {
            (0x0079_50f0, 0x0079_5a20, chr)
        };
        let fa = Global::<u32>::new(fa).ptr(mem).addr;
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let a = fcall(mem, fa, &[arg]) as f32;
        let fb = Global::<u32>::new(fb).ptr(mem).addr;
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let b = fcall(mem, fb, &[arg]) as f32;
        let a = crate::x87::load_f32(a);
        let b = crate::x87::load_f32(b);
        let m = if b > a { b } else { a };
        let div: crate::x87::Op = |x, y| x / y;
        let k: f64 = mem.read(Global::<u32>::new(0x00b5_87d8).ptr(mem).addr);
        Reg::load_f32(m).constant(div, k).store()
    }

    /// `opus_osu_init(out)` (`0x00801200`): the 0x100 bytes at `out`
    /// filled with `g_Ee_Gpr`'s `r0` (16 bytes at a time, read anew each
    /// time), then `+0xc8` = `0x80808080`. Returns `r0`'s last word (left
    /// in `eax`).
    pub fn opus_osu_init(mem: &mut impl Mem, out: u32) -> u32 {
        let r0 = crate::ppp::bridge::R0.ptr(mem).addr;
        let mut last = 0;
        for i in 0..0x40u32 {
            last = mem.read::<u32>(r0 + 4 * (i % 4));
            mem.write(out.wrapping_add(4 * i), last);
        }
        mem.write(out.wrapping_add(0xc8), 0x8080_8080u32);
        last
    }

    /// `opus_osu_copy_00800eb0(set, dst, src)`: `src`'s words `+0x30`..
    /// `+0x4c` and `+0x90`..`+0x9c`, `u16 +0x14` and bytes `+0x18`,
    /// `+0x1e`, `+0xba`, `+0xb3` into `dst`, and `dst.u16[+0x1c]` =
    /// `(src - set[+0x20]) >> 8` (`src`'s index among `set`'s 0x100-byte
    /// records). Returns the byte `+0xb3` (left in `eax`).
    pub fn opus_osu_copy(mem: &mut impl Mem, set: u32, dst: u32, src: u32) -> u32 {
        for o in (0x30u32..0x50).step_by(4).chain((0x90u32..0xa0).step_by(4)) {
            let x: u32 = mem.read(src.wrapping_add(o));
            mem.write(dst.wrapping_add(o), x);
        }
        let base: u32 = mem.read(set.wrapping_add(0x20));
        #[allow(clippy::cast_possible_truncation)] // the u16 store
        mem.write(dst.wrapping_add(0x1c), (src.wrapping_sub(base) >> 8) as u16);
        let b: u8 = mem.read(src.wrapping_add(0x1e));
        mem.write(dst.wrapping_add(0x1e), b);
        let h: u16 = mem.read(src.wrapping_add(0x14));
        mem.write(dst.wrapping_add(0x14), h);
        let mut last = 0;
        for o in [0x18u32, 0xba, 0xb3] {
            last = mem.read::<u8>(src.wrapping_add(o));
            mem.write(dst.wrapping_add(o), last);
        }
        u32::from(last)
    }

    /// `0x00801780(set, id, list)`: `id` into the first free (negative)
    /// `i16` of `set`'s list `[set +0x10 + 4 list]`, among `u16 +0x34` - 1
    /// entries (the index an `i16`), and `u16 +0x38` up by one: 0; -1 when
    /// none is free.
    pub fn opus_list_add(mem: &mut impl Mem, set: u32, id: u16, list: u32) -> u32 {
        let n = u32::from(mem.read::<u16>(set.wrapping_add(0x34)).wrapping_sub(1));
        let t: u32 = mem.read(set.wrapping_add(0x10).wrapping_add(list.wrapping_mul(4)));
        let mut i = 0i16;
        while i32::from(i) < n.cast_signed() {
            let at = t.wrapping_add(i32::from(i).cast_unsigned().wrapping_mul(2));
            if mem.read::<i16>(at) < 0 {
                mem.write(at, id);
                let c: u16 = mem.read(set.wrapping_add(0x38));
                mem.write(set.wrapping_add(0x38), c.wrapping_add(1));
                return 0;
            }
            i = i.wrapping_add(1);
        }
        u32::MAX
    }

    /// `opus_prog_on(set, list)` (`0x00801700`): like
    /// [`opus_list_add`], but the id put into the first free (negative)
    /// `i16` of list `list` is the first part (0x100 bytes from `[set +
    /// 0x20]`, among the same `u16 +0x34` - 1) whose first word is 0
    /// (`is_ptr32_null`), and `u16 +0x38` up by one. Both indexes are
    /// `i16`. Returns that part's number in `ax` over the list address's
    /// high half (`eax` as left); -1 when no list entry or no part is
    /// free.
    pub fn opus_prog_on(mem: &mut impl Mem, set: u32, list: u32) -> u32 {
        let count =
            u32::from(mem.read::<u16>(set.wrapping_add(0x34)).wrapping_sub(1)).cast_signed();
        let table: u32 = mem.read(set.wrapping_add(0x10).wrapping_add(list.wrapping_mul(4)));
        let idx = |i: i16| i32::from(i).cast_unsigned();
        let mut i = 0i16;
        while i32::from(i) < count {
            let at = table.wrapping_add(idx(i).wrapping_mul(2));
            if mem.read::<i16>(at) < 0 {
                let mut k = 0i16;
                while i32::from(k) < count {
                    let parts: u32 = mem.read(set.wrapping_add(0x20));
                    let first: u32 = mem.read(parts.wrapping_add(idx(k) << 8));
                    if first == 0 {
                        mem.write(at, k);
                        let c: u16 = mem.read(set.wrapping_add(0x38));
                        mem.write(set.wrapping_add(0x38), c.wrapping_add(1));
                        return table & 0xffff_0000 | u32::from(k.cast_unsigned());
                    }
                    k = k.wrapping_add(1);
                }
                return u32::MAX;
            }
            i = i.wrapping_add(1);
        }
        u32::MAX
    }

    /// `opus_osu_set_0080b520(set, src, ptr, list, _)` (`0x0080b520`): a
    /// free unit of `set` on index list `list` ([`opus_prog_on`]; 0 when
    /// none) set up from `src` ([`opus_osu_init`], [`opus_osu_copy`]),
    /// numbered (`+0x1a` = the set's `u16 +2`, which counts on and wraps
    /// from 0xfff0 to 0x8000) and pointed at `ptr` (`+0` =
    /// `set_ptr32(ptr)`, through `call`). Returns the unit. With `list` 1
    /// the original also hands the unit to `yonExtMapRegister`, a bare
    /// `ret` in this build, with a 0x4c-byte record by value (the fifth
    /// argument in it); that call is left out.
    pub fn opus_osu_set<M: Mem>(
        mem: &mut M,
        set: u32,
        src: u32,
        ptr: u32,
        list: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the u16 index in ax
        let i = (opus_prog_on(mem, set, list) as u16).cast_signed();
        if i < 0 {
            return 0;
        }
        osu_take(mem, set, i, src, ptr, call)
    }

    /// Unit `i` of `set` (0x100 bytes from `+0x20`, `i` sign-extended) set
    /// up from `src` ([`opus_osu_init`], [`opus_osu_copy`]), numbered
    /// (`+0x1a` = the set's `u16 +2`, which counts on and wraps from 0xfff0
    /// to 0x8000) and pointed at `ptr` (`+0` = `set_ptr32(ptr)`, through
    /// `call`); returns the unit.
    fn osu_take<M: Mem>(
        mem: &mut M,
        set: u32,
        i: i16,
        src: u32,
        ptr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let units: u32 = mem.read(set.wrapping_add(0x20));
        let unit = units.wrapping_add((i32::from(i) << 8).cast_unsigned());
        opus_osu_init(mem, unit);
        opus_osu_copy(mem, set, unit, src);
        let serial: u16 = mem.read(set.wrapping_add(2));
        mem.write(unit.wrapping_add(0x1a), serial);
        let next = serial.wrapping_add(1);
        mem.write(
            set.wrapping_add(2),
            if next >= 0xfff0 { 0x8000 } else { next },
        );
        let p = call(mem, g(mem, 0x00a4_4700), &[ptr]);
        mem.write(unit, p);
        unit
    }

    /// `opu_osu_load_0080ae30(host, cmd)` (`0x0080ae30`): with `u16 host
    /// +0x212` set, `host +0` = `host` only. Otherwise a unit of the host's
    /// set (`host + 0x300`) on index list `(i16 host +0x21a >> 12) & 3`
    /// ([`opus_prog_on`]; its result is used unchecked, so none free gives
    /// unit -1, 0x100 bytes before the first) is set up from the host's
    /// unit (`+0x220`) as [`opus_osu_set`] does, pointed at `cmd + i16 cmd
    /// +2`, and `host +0` = the unit. The original also fills a record
    /// named `LoadedByOPU_UNIT` for `yonExtMapRegister` (list 1), a bare
    /// `ret` in this build: left out. Returns `cmd + 4`.
    pub fn opu_osu_load<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u16>(host.wrapping_add(0x212)) != 0 {
            mem.write(host, host);
            return cmd.wrapping_add(4);
        }
        let src: u32 = mem.read(host.wrapping_add(0x220));
        let list = u32::from((mem.read::<i16>(host.wrapping_add(0x21a)) >> 12).cast_unsigned() & 3);
        let set = host.wrapping_add(0x300);
        #[allow(clippy::cast_possible_truncation)] // the u16 index in ax
        let i = (opus_prog_on(mem, set, list) as u16).cast_signed();
        let off = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let unit = osu_take(mem, set, i, src, cmd.wrapping_add(off), call);
        mem.write(host, unit);
        cmd.wrapping_add(4)
    }

    /// `opus_hd_ptr_get_00803200(base, i)` (`0x00803200`): `get_ptr32` of
    /// the word at `base + 16 (i + 1)` (through `call`).
    pub fn opus_hd_ptr_get<M: Mem>(
        mem: &mut M,
        base: u32,
        i: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w: u32 = mem.read(base.wrapping_add(i.wrapping_add(1).wrapping_mul(16)));
        call(mem, g(mem, 0x00a4_4650), &[w])
    }

    /// `op_part_init_check(part)` (`0x0080b650`): 1 when `get_ptr32(part
    /// +0x14)` is at least 0x100000 (a real pointer), else 0.
    pub fn op_part_init_check<M: Mem>(
        mem: &mut M,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w: u32 = mem.read(part.wrapping_add(0x14));
        u32::from(call(mem, g(mem, 0x00a4_4650), &[w]) >= 0x10_0000)
    }

    /// `g_OpusFfxFocus` (`0x011a0068`) and the focus it replaced
    /// (`0x011a0069`): `opus_ffxfocus_off` turns `Sg_EnFocus` off and
    /// keeps the old state once, `opus_ffxfocus_free` puts it back.
    pub const FFX_FOCUS: Global<[u8; 2]> = Global::new(0x011a_0068);

    /// `opus_ffxfocus_off()` (`0x0080abe0`): `Sg_EnFocus(0)`; the first time
    /// (no focus kept) its old state is kept. Returns `Sg_EnFocus`'s eax.
    pub fn opus_ffxfocus_off<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r = call(mem, g(mem, 0x0082_cb60), &[0]);
        if FFX_FOCUS.get(mem)[0] == 0 {
            FFX_FOCUS.set(mem, [1, r.to_le_bytes()[0]]);
        }
        r
    }

    /// `opus_ffxfocus_free()` (`0x0080abb0`): with a focus kept, it is put
    /// back (`Sg_EnFocus(old)`) and no longer kept.
    pub fn opus_ffxfocus_free<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [kept, old] = FFX_FOCUS.get(mem);
        if kept != 0 {
            call(mem, g(mem, 0x0082_cb60), &[u32::from(old)]);
            FFX_FOCUS.set(mem, [0, old]);
        }
    }

    /// `opus_chr_org_set_008053a0(chr, m)` (`0x008053a0`): for a
    /// character, `Ch_SetOrgMatrix(chr, m)` and `Ch_EnClip(chr, 0)`
    /// (through `call`).
    pub fn opus_chr_org_set<M: Mem>(
        mem: &mut M,
        chr: u32,
        m: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if chr != 0 {
            call(mem, g(mem, 0x0082_b3e0), &[chr, m]);
            call(mem, g(mem, 0x0082_aa30), &[chr, 0]);
        }
    }

    /// Where [`opus_mdc_break_make`]'s callees see its locals, as offsets in
    /// its `frame` (0x180 bytes, the original's `ebp - 0x184` up): the
    /// image size words, `Ch_GetBigImageSize`'s buffer, the piece matrix,
    /// the entry's scaled matrix and the piece matrix without translation.
    const BREAK_SIZE: u32 = 0;
    const BREAK_BUFFER: u32 = 0x70;
    const BREAK_PIECE_M: u32 = 0xc0;
    const BREAK_ENTRY_M: u32 = 0x100;
    const BREAK_TURN_M: u32 = 0x140;

    /// `opus_mdc_break_make_VitaSafe(mdc, out, params, view)`
    /// (`0x008059c0`): breaks a model into flying triangles. `out` gets
    /// `+0xc` = `params`, `+4` = 0 (the piece count), `+0x20` = the float
    /// `view +8`, `+8` = `mdc`, `+0x10..+0x1f` the four words at `[view
    /// +4]`, after `Ch_GetBigImageSize([mdc], ...)` into the frame. Each of
    /// the `u16 mdc +4` entries of 16 bytes from `mdc + 0x10` ([`break_entry`])
    /// adds its pieces, 0xc0 bytes each from `out + 0x30`. Callees through
    /// `call` (`op_random` through `fcall`); `frame` stands for the
    /// original's locals (0x180 bytes). `op_irandom_p` reads four
    /// arguments where one is pushed: the other three are `op_random`'s
    /// last result (left in its argument slot) and the caller's `edi` and
    /// `esi` as the original saved them (`saved`); they only reach the
    /// emulated VU0 registers.
    pub fn opus_mdc_break_make<M: Mem>(
        mem: &mut M,
        args: [u32; 4],
        frame: u32,
        saved: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let [mdc, out, params, view] = args;
        let camera: u32 = mem.read(view);
        mem.write(out.wrapping_add(0xc), params);
        mem.write(out.wrapping_add(4), 0u32);
        let scale = crate::x87::load(mem.read(view.wrapping_add(8)));
        mem.write(out.wrapping_add(0x20), scale);
        mem.write(out.wrapping_add(8), mdc);
        let id: u32 = mem.read(mdc);
        let size = frame + BREAK_SIZE;
        call(
            mem,
            g(mem, 0x0083_3e60),
            &[id, size + 4, size, frame + BREAK_BUFFER],
        );
        let pos: u32 = mem.read(view.wrapping_add(4));
        let p: [u32; 4] = mem.read(pos);
        mem.write(out.wrapping_add(0x10), p);
        let mut piece = out.wrapping_add(0x30);
        let ctx = Break {
            out,
            params,
            view,
            camera,
            frame,
            saved,
        };
        let mut i = 0u32;
        while i < u32::from(mem.read::<u16>(mdc.wrapping_add(4))) {
            let entry = mdc.wrapping_add(0x10 + 0x10 * i);
            piece = break_entry(mem, &ctx, entry, piece, call, fcall);
            i += 1;
        }
    }

    /// What [`break_entry`] and [`break_piece`] share.
    struct Break {
        out: u32,
        params: u32,
        view: u32,
        camera: u32,
        frame: u32,
        saved: [u32; 2],
    }

    /// One entry of [`opus_mdc_break_make`]: its matrix (`get_ptr32_func`
    /// of `+4`, copied, the translation times 1000) after the camera
    /// (`op_mul_m(piece_m, camera, entry_m)`), then the parts of its model
    /// (`get_ptr32_func` of `+0`, vertices of 16 bytes at `[+8]` past it,
    /// parts from `+0x20` until a kind (`+1`) of 0xff; kinds other than
    /// 0x10 are reported and `debug_exit_trace(1)` called). A part of `i16
    /// +2` triangles of 0x1c bytes from `+0x20` makes its pieces while the
    /// total stays below 4096, else is skipped. Returns the next piece.
    fn break_entry<M: Mem>(
        mem: &mut M,
        ctx: &Break,
        entry: u32,
        mut piece: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        let get = g(mem, 0x00a4_4650);
        let (entry_m, piece_m) = (ctx.frame + BREAK_ENTRY_M, ctx.frame + BREAK_PIECE_M);
        let w: u32 = mem.read(entry.wrapping_add(4));
        let m = call(mem, get, &[w]);
        call(mem, g(mem, 0x006e_d360), &[entry_m, m]);
        for k in 0..3 {
            let at = entry_m + 0x30 + 4 * k;
            let v = ffx_bridge::x87::load(mem.read(at));
            mem.write(at, ffx_bridge::x87::store(ffx_bridge::x87::mul(v, 1000.0)));
        }
        call(mem, g(mem, 0x007e_83b0), &[piece_m, ctx.camera, entry_m]);
        let w: u32 = mem.read(entry);
        let model = call(mem, get, &[w]);
        let verts = mem.read::<u32>(model.wrapping_add(8)).wrapping_add(model);
        let mut part = model.wrapping_add(0x20);
        let total_at = ctx.out.wrapping_add(4);
        while mem.read::<u8>(part.wrapping_add(1)) != 0xff {
            if mem.read::<u8>(part.wrapping_add(1)) != 0x10 {
                let msg = g(mem, 0x00b5_87f8);
                call(mem, g(mem, 0x0062_f500), &[msg]);
                call(mem, g(mem, 0x0062_f510), &[1]);
            }
            let count = |mem: &M| i32::from(mem.read::<i16>(part.wrapping_add(2)));
            let n = count(mem);
            let total = mem.read::<u32>(total_at).cast_signed().wrapping_add(n);
            let mut tri = part.wrapping_add(0x20);
            if total < 0x1000 {
                mem.write(total_at, total);
                let mut k = 0;
                while k < count(mem) {
                    break_piece(mem, ctx, piece, tri, part, verts, call, fcall);
                    piece = piece.wrapping_add(0xc0);
                    tri = tri.wrapping_add(0x1c);
                    k += 1;
                }
            } else {
                tri = tri.wrapping_add(n.wrapping_mul(0x1c).cast_unsigned());
            }
            part = tri;
        }
        // the original fetches the model twice more and drops it
        let w: u32 = mem.read(entry);
        call(mem, get, &[w]);
        let w: u32 = mem.read(entry);
        call(mem, get, &[w]);
        piece
    }

    /// One piece of [`break_entry`] from the triangle `tri` (`i16`
    /// vertex indices at `+0`, `+2`, `+4`; six `u16` texture coordinates
    /// from `+8`, over 4096 into `+0x30`, `+0x34`, `+0x40`, `+0x44`,
    /// `+0x50`, `+0x54`). Its centre `c` (the three vertices' mean, at
    /// x87 precision) goes to `+0x60` (`w` 1 at `+0x6c`), the vertices less
    /// `c` to `+0`, `+0x10`, `+0x20` (`w` 1); the centre through the piece
    /// matrix and the vertices through it without translation
    /// (`FFXVu0ApplyMatrix`). Then `+0xb0` = the centre's `y`, the unit
    /// direction from `[view +4]` to the centre at `+0x70` and its length
    /// at `+0x7c`, `g_Ee_Gpr`'s r0 into `+0x90` and `+0xb4`; `params`
    /// (`i16` pairs) give `+0x80`, `+0x84`, `+0xb8`, `+0xbc` = `(a +
    /// op_random(b)) / 256`, `+0xa0..+0xa8` = `op_random(+0x16..+0x1a)`,
    /// `u16 +0x48` = `op_irandom_p(+0x1e) + 0x10 + +0x1c`, `+0x4a` 0; the
    /// part gives `+0x38`, `+0x3c` (its words `+8`, `+0xc`) and the bytes
    /// `+0x58..+0x5b` (its `+4..+7`).
    #[allow(clippy::too_many_arguments)]
    fn break_piece<M: Mem>(
        mem: &mut M,
        ctx: &Break,
        piece: u32,
        tri: u32,
        part: u32,
        verts: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87::{add, div, load, mul, sqrt, store, sub};
        let at = |off: u32| piece.wrapping_add(off);
        for (k, off) in (0u32..).zip([0x30, 0x34, 0x40, 0x44, 0x50, 0x54]) {
            let v = mem.read::<u16>(tri.wrapping_add(8 + 2 * k));
            mem.write(at(off), f32::from(v) / 4096.0);
        }
        // the three vertices as the original copies them (fld/fstp)
        let vertex = |mem: &M, k: u32| -> [f64; 3] {
            let i = i32::from(mem.read::<i16>(tri.wrapping_add(2 * k)));
            let v = verts.wrapping_add(i.wrapping_mul(16).cast_unsigned());
            [0, 4, 8].map(|o| load(mem.read(v.wrapping_add(o))))
        };
        let (v0, v1, v2) = (vertex(mem, 0), vertex(mem, 1), vertex(mem, 2));
        let mut centre = [0f64; 3];
        for j in 0..3 {
            let sum = add(add(v1[j], v0[j]), v2[j]);
            centre[j] = load(store(div(sum, 3.0)));
        }
        let one = 1.0f32.to_bits();
        for (j, &c) in (0u32..).zip(&centre) {
            mem.write(at(0x60 + 4 * j), store(c));
            mem.write(at(4 * j), store(sub(v0[j as usize], c)));
            mem.write(at(0x10 + 4 * j), store(sub(v1[j as usize], c)));
            mem.write(at(0x20 + 4 * j), store(sub(v2[j as usize], c)));
        }
        for off in [0xc, 0x1c, 0x2c, 0x6c] {
            mem.write(at(off), one);
        }
        let (piece_m, turn_m) = (ctx.frame + BREAK_PIECE_M, ctx.frame + BREAK_TURN_M);
        call(mem, g(mem, 0x006e_d360), &[turn_m, piece_m]);
        mem.write(turn_m + 0x30, [0u32, 0, 0, one]);
        let apply = g(mem, 0x006e_d2e0);
        call(mem, apply, &[at(0x60), piece_m, at(0x60)]);
        for off in [0, 0x10, 0x20] {
            call(mem, apply, &[at(off), turn_m, at(off)]);
        }
        let centre_y: u32 = mem.read(at(0x64));
        mem.write(at(0xb0), crate::x87::load(centre_y));
        // (the original clears +0xb4..+0xbc here; all three are written below)
        let pos: u32 = mem.read(ctx.view.wrapping_add(4));
        let dir: [f64; 3] = [0, 4, 8].map(|o| {
            let c = load(mem.read(at(0x60 + o)));
            load(store(sub(c, load(mem.read(pos.wrapping_add(o))))))
        });
        let squares = add(
            add(mul(dir[1], dir[1]), mul(dir[0], dir[0])),
            mul(dir[2], dir[2]),
        );
        let len = load(store(sqrt(squares)));
        for (j, &dj) in (0u32..).zip(&dir) {
            mem.write(at(0x70 + 4 * j), store(div(dj, len)));
        }
        mem.write(at(0x7c), store(len));
        let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
        mem.write(at(0xb4), r0);
        mem.write(at(0x90), r0);
        let int = |mem: &M, off: u32| f32::from(mem.read::<i16>(ctx.params.wrapping_add(off)));
        let random = g(mem, 0x007e_6700);
        for (lo, off) in [(4, 0x80), (8, 0x84), (0xc, 0xb8), (0x10, 0xbc)] {
            let base = f64::from(int(mem, lo));
            let spread = int(mem, lo + 2);
            let r = fcall(mem, random, &[spread.to_bits()]);
            mem.write(at(off), store(mul(add(r, base), 1.0 / 256.0)));
        }
        let mut last = 0;
        for (k, off) in [(0x16, 0xa0), (0x18, 0xa4), (0x1a, 0xa8)] {
            let spread = int(mem, k);
            let r = fcall(mem, random, &[spread.to_bits()]);
            last = store(r);
            mem.write(at(off), last);
        }
        let spread = i32::from(mem.read::<i16>(ctx.params.wrapping_add(0x1e))).cast_unsigned();
        let [edi, esi] = ctx.saved;
        #[allow(clippy::cast_possible_truncation)] // the 16-bit sum
        let n = call(mem, g(mem, 0x007e_5d50), &[spread, last, edi, esi]) as u16;
        let base: u16 = mem.read(ctx.params.wrapping_add(0x1c));
        mem.write(at(0x48), [n.wrapping_add(0x10).wrapping_add(base), 0]);
        let words: [u32; 2] = mem.read(part.wrapping_add(8));
        mem.write(at(0x38), words);
        let bytes: [u8; 4] = mem.read(part.wrapping_add(4));
        mem.write(at(0x58), bytes);
    }

    /// `opus_mdc_rec_init_00813950(rec, n, a, b)` (`0x00813950`): a record
    /// head: `u16 +0` 0, `+2` `n`, `+4` `a`, `+6` `b`, `+8` 0; `+0x18` =
    /// `set_ptr32(rec + 0x20)`, `+0x1c` = `set_ptr32(get_ptr32(+0x18) + 32
    /// n)` (through `call`). Returns the last `set_ptr32`'s eax.
    pub fn opus_mdc_rec_init<M: Mem>(
        mem: &mut M,
        rec: u32,
        n: u32,
        a: u32,
        b: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the u16 stores
        let head = [0u16, n as u16, a as u16, b as u16, 0];
        mem.write(rec, head);
        let set = g(mem, 0x00a4_4700);
        let first = call(mem, set, &[rec.wrapping_add(0x20)]);
        mem.write(rec.wrapping_add(0x18), first);
        let p = call(mem, g(mem, 0x00a4_4650), &[first]);
        let end = call(mem, set, &[p.wrapping_add(n << 5)]);
        mem.write(rec.wrapping_add(0x1c), end);
        end
    }

    /// `opu_mdc_0x1000_hide(_)` (`0x00804150`): by the magic file
    /// (`MagicFile_GetMagicId`, through `call`): 0 for 0x280, 0x1e5, 0x2c6,
    /// 0x250 and 0x276..=0x27d; 3 for the nine ids listed at `0x00b587e0`
    /// (`i16`); else 1.
    pub fn opu_mdc_0x1000_hide<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id = call(mem, g(mem, 0x009d_a3b0), &[]);
        if matches!(id, 0x280 | 0x1e5 | 0x2c6 | 0x250) || id.wrapping_sub(0x276) <= 7 {
            return 0;
        }
        let list: [i16; 9] = mem.read(g(mem, 0x00b5_87e0));
        if list.iter().any(|&x| i32::from(x).cast_unsigned() == id) {
            3
        } else {
            1
        }
    }

    /// `opus_map_rgba_free()` (`0x0080ac00`): the map's look back to
    /// normal: `yiRenderDisable(4)`, `yiSetColorScale` of (1, 1, 1, 1) (a
    /// local vector at `frame`, 16 bytes), and with a back colour kept
    /// (`0x011a006f`) `yiSetBackColor(0x011a0070)` and the flag cleared;
    /// the colour scale at `0x00c48e64` 1.0 four times. Callees through
    /// `call`.
    pub fn opus_map_rgba_free<M: Mem>(
        mem: &mut M,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let one = 1.0f32.to_bits();
        mem.write(frame, [one; 4]);
        call(mem, g(mem, 0x0090_9e10), &[4]);
        call(mem, g(mem, 0x0090_a270), &[frame]);
        let kept = g(mem, 0x011a_006f);
        if mem.read::<u8>(kept) != 0 {
            call(mem, g(mem, 0x0090_a040), &[g(mem, 0x011a_0070)]);
            mem.write(kept, 0u8);
        }
        mem.write(g(mem, 0x00c4_8e64), [one; 4]);
    }

    /// `opus_copy_buffer_event_00808a60()` (`0x00808a60`): in the two
    /// magic files 0x11b and 0x1a6 (`MagicFile_GetMagicId`), a copy-buffer
    /// event (`graphicVFXAddCopyBufferEventWithClearRGB(0, 1)`), through
    /// `call`.
    pub fn opus_copy_buffer_event<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let id = g(mem, 0x009d_a3b0);
        if call(mem, id, &[]) == 0x11b || call(mem, id, &[]) == 0x1a6 {
            call(mem, g(mem, 0x0064_4c40), &[0, 1]);
        }
    }

    /// `opus_all_clut_dispose()` (`0x00808a20`): `Ch_ClutDispose` of the
    /// first word of each of the first 0xd0 character records that has one
    /// (through `call`).
    pub fn opus_all_clut_dispose<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        for i in 0..0xd0 {
            let h: u32 = mem.read(crate::et::bridge::CHR.ptr(mem).at(i).addr);
            if h != 0 {
                call(mem, g(mem, 0x0083_c810), &[h]);
            }
        }
    }

    /// `opus_clip_on_all_in_battle()` (`0x00800be0`): in battle
    /// (`MsBattleChrStart()` nonzero) every handle of the first 32 in
    /// `g_EtChrHandles` gets `Ch_EnClip(h, 1)` (through `call`).
    pub fn opus_clip_on_all_in_battle<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if call(mem, g(mem, 0x0078_0cb0), &[]) == 0 {
            return;
        }
        for i in 0..0x20 {
            let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(i).get(mem);
            if h != 0 {
                call(mem, g(mem, 0x0082_aa30), &[h, 1]);
            }
        }
    }

    /// `opus_chr_thickness_00800c90(chr, battle)` (`0x00800c90`): in
    /// battle `MsGetChrWidth(chr)`, else `Ch_GetThickness` of its handle in
    /// `g_EtChrHandles` (through `fcall`), rounded to `f32`.
    pub fn opus_chr_thickness<M: Mem>(
        mem: &mut M,
        chr: u32,
        battle: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> f32 {
        let v = if battle == 0 {
            let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
            fcall(mem, g(mem, 0x0082_ad70), &[h])
        } else {
            fcall(mem, g(mem, 0x0079_5a20), &[chr])
        };
        f32::from_bits(ffx_bridge::x87::store(v))
    }

    /// `opus_ffxdeg_to_normaldeg(x)` (`0x0081b170`): an angle in radians
    /// to the game's 4096-step units, flipped: `n = ftol(x * k1 / k2)`
    /// (`f64` constants at `0x00b51bc0` and `0x00b582f0`, rounded to `f32`
    /// first), then `(0x400 - n) & 0xfff` as a float.
    pub fn opus_ffxdeg_to_normaldeg(mem: &impl Mem, x: f32) -> f32 {
        use ffx_bridge::x87::{div, load, mul, store};
        let k1: f64 = mem.read(g(mem, 0x00b5_1bc0));
        let k2: f64 = mem.read(g(mem, 0x00b5_82f0));
        let v = store(div(mul(load(x.to_bits()), k1), k2));
        let n = ffx_bridge::x87::ftol(mem, load(v));
        #[allow(clippy::cast_precision_loss)] // 0..0x1000: exact
        let r = (0x400u32.wrapping_sub(n) & 0xfff) as f32;
        r
    }

    /// `opus_normaldeg_to_ffxdeg_0081b230(x)` (`0x0081b230`): the way
    /// back: `(0x400 - ftol(x)) & 0xfff` as a float times the `f64`
    /// constants at `0x00b582f0` and `0x00b58590`, rounded to `f32`.
    pub fn opus_normaldeg_to_ffxdeg(mem: &impl Mem, x: f32) -> f32 {
        use ffx_bridge::x87::{load, mul, store};
        let k1: f64 = mem.read(g(mem, 0x00b5_82f0));
        let k2: f64 = mem.read(g(mem, 0x00b5_8590));
        let n = ffx_bridge::x87::ftol(mem, load(x.to_bits()));
        #[allow(clippy::cast_precision_loss)] // 0..0x1000: exact
        let r = f64::from((0x400u32.wrapping_sub(n) & 0xfff) as f32);
        f32::from_bits(store(mul(mul(r, k1), k2)))
    }

    /// `opus_set_chr_dirf(chr, dir)` (`0x0081b270`): in battle
    /// `MsSetChrDirF(chr, dir)`; elsewhere `Ch_SetDirF` on its handle in
    /// `g_EtChrHandles`, when it has one (0 otherwise). Through `call`;
    /// returns the callee's eax.
    pub fn opus_set_chr_dirf<M: Mem>(
        mem: &mut M,
        chr: u32,
        dir: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let dir = crate::x87::load(dir);
        if call(mem, g(mem, 0x0078_0cb0), &[]) != 0 {
            return call(mem, g(mem, 0x0079_5e20), &[chr, dir]);
        }
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        if h == 0 {
            return 0;
        }
        call(mem, g(mem, 0x0082_b1b0), &[h, dir])
    }

    /// `opus_set_chr_pos(chr, pos)` (`0x0081b2b0`): in battle
    /// `MsSetChrPos(chr, pos)`; elsewhere `Ch_SetPos3(h, x, y, z)` on its
    /// handle, when it has one. Through `call`; returns the callee's eax
    /// (the handle, 0, without one).
    pub fn opus_set_chr_pos<M: Mem>(
        mem: &mut M,
        chr: u32,
        pos: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if call(mem, g(mem, 0x0078_0cb0), &[]) != 0 {
            return call(mem, g(mem, 0x0079_6150), &[chr, pos]);
        }
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        if h == 0 {
            return chr;
        }
        let xyz: [u32; 3] = mem.read(pos);
        let [x, y, z] = xyz.map(crate::x87::load);
        call(mem, g(mem, 0x0082_b480), &[h, x, y, z])
    }

    /// `opu_matrix_bone(host, cmd)` (`0x00803940`): the unit's matrix
    /// from its character's bone. `h` = `g_EtChrHandles[unit +0x18]`;
    /// the element `e` is `i16 cmd +2` for nibble 0, `ftol(f32 unit + u8
    /// cmd +2)` for 1; other nibbles return cmd at once. `host +0` =
    /// `Ch_GetOrgMatrix(h)` (`op_m_copy`) or `Ch_GetRootGlobalMatrix(h,
    /// host)`; `host +0x40` = `Ch_ElemGetBaseMatrix(h, e', host +0x40)`
    /// with `e'` = `Ch_OptposGetElement(h, e & 0xff, vec)` and its row 3 =
    /// `op_apply_m_w1(host +0x70, host +0x40, vec)` for bit 0x8000, else
    /// `e` and `host +0x80` = the emulated `r0`; `host` = `host x
    /// host+0x40`; `host +0x80` = scale 999.99994 rotated (`op_rt_m`) by
    /// unit `+0x30` at `host +0xc0` = unit `+0x90` scaled (`op_xyz_scale`);
    /// `host` x= it. Then the emulated VU0 normalises `host`'s three rows
    /// (`0x00c8f528` the row, `0x00c8f538` the count, the move register
    /// `0x00c8f980`, squares and sums `0x00c8f970..`, `Q`, results
    /// `0x00c8f960..`, whose `w` `0x00c8f96c` each row takes), and unit
    /// `+0xc0` = `host +0x280 x host`. Returns cmd+4. Callees through
    /// `call`; `vec` is the original's 4-word local.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_matrix_bone<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        vec: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{Reg, ftol, load};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let chr = u32::from(mem.read::<u8>(unit.wrapping_add(0x18)));
        let h: u32 = mem.read(g(mem, 0x012a_8100).wrapping_add(4 * chr));
        let e = match mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000 {
            0 => i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned(),
            0x1000 => {
                let at = u32::from(mem.read::<u8>(cmd.wrapping_add(2)));
                ftol(mem, load(mem.read(at.wrapping_add(unit))))
            }
            _ => return cmd,
        };
        let org = go(mem, 0x0082_ac60, &[h]);
        if org == 0 {
            go(mem, 0x0083_3f80, &[h, host]);
        } else {
            go(mem, 0x007e_8010, &[host, org]);
        }
        let base = host.wrapping_add(0x40);
        if e & 0x8000 == 0 {
            go(mem, 0x0083_3b80, &[h, e, base]);
            let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
            mem.write(host.wrapping_add(0x80), r0);
        } else {
            let el = go(mem, 0x0083_5660, &[h, e & 0xff, vec]);
            go(mem, 0x0083_3b80, &[h, el, base]);
            go(mem, 0x007e_79a0, &[host.wrapping_add(0x70), base, vec]);
        }
        go(mem, 0x007e_83b0, &[host, host, base]);
        let s = mem.read::<u32>(g(mem, 0x00b5_88f8));
        let s = ffx_bridge::x87::round_trip(f32::from_bits(s)).to_bits();
        let (m80, mc0) = (host.wrapping_add(0x80), host.wrapping_add(0xc0));
        go(mem, 0x007e_a700, &[m80, s]);
        go(mem, 0x007e_a850, &[mc0, unit.wrapping_add(0x90), s]);
        go(mem, 0x007e_9850, &[m80, m80, unit.wrapping_add(0x30), mc0]);
        go(mem, 0x007e_83b0, &[host, host, m80]);
        // the emulated VU0's normalisation of the three rows
        let reg = |mem: &M, a: u32| g(mem, a);
        let (row_at, count) = (reg(mem, 0x00c8_f528), reg(mem, 0x00c8_f538));
        let (mv, sq, out) = (
            reg(mem, 0x00c8_f980),
            reg(mem, 0x00c8_f970),
            reg(mem, 0x00c8_f960),
        );
        let vf00: [f32; 4] = mem.read(reg(mem, 0x00c0_a004));
        mem.write(row_at, host);
        mem.write(count, 3u32);
        loop {
            let row: u32 = mem.read(row_at);
            let v: [f32; 4] = mem.read(row);
            mem.write(mv, v);
            let sx = Reg::load(v[0]).mul_m32(v[0]).store();
            let sy = Reg::load(v[1]).mul_m32(v[1]).store();
            let sz = Reg::load(v[2]).mul_m32(v[2]).store();
            let s1 = Reg::load(sy).add_m32(sx).store();
            let s2 = Reg::load(sz).add_m32(s1).store();
            let root = Reg::load(f32::from_bits(s2.to_bits() & 0x7fff_ffff))
                .sqrt()
                .store();
            let len = Reg::load(root).add_m32(vf00[0]).store();
            mem.write(sq, [len, sy, sz]);
            let q = Reg::load(vf00[3]).div_m32(len).store();
            mem.write(reg(mem, 0x00c8_f788), q);
            let qr = Reg::load(q);
            let n = [
                (Reg::load(v[0]) * qr).store(),
                (Reg::load(v[1]) * qr).store(),
                qr.mul_m32(v[2]).store(),
            ];
            mem.write(out, n);
            let w: u32 = mem.read(out.wrapping_add(0xc));
            mem.write(row, n);
            mem.write(row.wrapping_add(0xc), w);
            let left = mem.read::<u32>(count).wrapping_sub(1);
            let next = mem.read::<u32>(row_at).wrapping_add(0x10);
            mem.write(count, left);
            mem.write(row_at, next);
            if left == 0 {
                break;
            }
        }
        go(
            mem,
            0x007e_83b0,
            &[unit.wrapping_add(0xc0), host.wrapping_add(0x280), host],
        );
        cmd.wrapping_add(4)
    }

    /// `opu_se_00814150(host, cmd)` (ours; opus table `0x00c48f74`): the
    /// sound commands, by the nibble `host u16 +0x21a >> 12`. With `unit`
    /// = `host +0x220`, `oef` = `host +0x360 + 4 unit +0x1e` and `ch` = 8
    /// for flag 0x800, else `0x10000 << unit +0x1e` (0 from 32), `a(n)` =
    /// `op_oef2_a_get(oef, 6, u16 cmd +2)`: 0 `op_oef2_se_on(a, 0x40,
    /// 0x100)` -> cmd+4; 1 `op_oef2_sound_cue_multi(oef, 0, 1000, ch)`;
    /// 2 `op_oef2_sep_free_all(ch)`; 3 `op_oef2_sound_cue_check(ch)`:
    /// nonzero sets `host +0x210` = 1 and returns cmd; 4 / 5
    /// `op_oef2_se_3d_on` / `_move(a, unit + 0x90, 0x100)` -> cmd+6; 6
    /// `op_oef2_stream_cue(a, i16 cmd +4, ch)` -> cmd+6; 7 / 8
    /// `MsChrSEfoot(unit +0x18, 1 / 0)`; 9 / 0xa the same for each of the
    /// first 32 characters with a handle (`g_EtChrHandles`); 0xb
    /// `op_oef2_se_volume(a, i16 cmd +4)` -> cmd+6; 0xc
    /// `SndMusicVolumeTranslate(music, i16 cmd +2, 0)` when
    /// `SndGetNowPlayMusic()` is not negative -> cmd+4. 1, 2, 3 and 7..=0xa
    /// return cmd+2, others cmd. All callees through `call`.
    pub fn opu_se<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let k = u32::from(mem.read::<u8>(unit.wrapping_add(0x1e)));
        let flags = u32::from(mem.read::<u16>(host.wrapping_add(0x21a)));
        let oef: u32 = mem.read(host.wrapping_add(0x360 + 4 * k));
        let ch = if flags & 0x800 != 0 {
            8
        } else {
            0x1_0000u32.checked_shl(k).unwrap_or(0)
        };
        let next = cmd.wrapping_add(2);
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let a = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let n = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
            go(mem, 0x007e_3810, &[oef, 6, n])
        };
        let word =
            |mem: &M, at: u32| i32::from(mem.read::<i16>(cmd.wrapping_add(at))).cast_unsigned();
        let foot = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, v: u32| {
            let handles = g(mem, 0x012a_8100);
            for i in 0..0x20u32 {
                if mem.read::<u32>(handles.wrapping_add(4 * i)) != 0 {
                    go(mem, 0x0079_fd20, &[i, v]);
                }
            }
        };
        match flags & 0xf000 {
            0 => {
                let c = a(mem, &mut go);
                go(mem, 0x007f_fe10, &[c, 0x40, 0x100]);
                cmd.wrapping_add(4)
            }
            0x1000 => {
                go(mem, 0x0080_02c0, &[oef, 0, 1000, ch]);
                next
            }
            0x2000 => {
                go(mem, 0x007f_ff30, &[ch]);
                next
            }
            0x3000 => {
                if go(mem, 0x0080_00f0, &[ch]) == 0 {
                    return next;
                }
                mem.write(host.wrapping_add(0x210), 1u8);
                cmd
            }
            f @ (0x4000 | 0x5000) => {
                let c = a(mem, &mut go);
                let at = if f == 0x4000 {
                    0x007f_fd20
                } else {
                    0x007f_fc60
                };
                go(mem, at, &[c, unit.wrapping_add(0x90), 0x100]);
                cmd.wrapping_add(6)
            }
            0x6000 => {
                let c = a(mem, &mut go);
                let w = word(mem, 4);
                go(mem, 0x0080_0390, &[c, w, ch]);
                cmd.wrapping_add(6)
            }
            f @ (0x7000 | 0x8000) => {
                let chr = u32::from(mem.read::<u8>(unit.wrapping_add(0x18)));
                go(mem, 0x0079_fd20, &[chr, u32::from(f == 0x7000)]);
                next
            }
            f @ (0x9000 | 0xa000) => {
                foot(mem, &mut go, u32::from(f == 0x9000));
                next
            }
            0xb000 => {
                let c = a(mem, &mut go);
                let w = word(mem, 4);
                go(mem, 0x007f_fde0, &[c, w]);
                cmd.wrapping_add(6)
            }
            0xc000 => {
                let music = go(mem, 0x0088_63f0, &[]);
                if music.cast_signed() >= 0 {
                    let w = word(mem, 2);
                    go(mem, 0x0088_6b40, &[music, w, 0]);
                }
                cmd.wrapping_add(4)
            }
            _ => cmd,
        }
    }

    /// `opp_md(host, set)` (`0x008020a0`, opp table entry `0x00c48e80`):
    /// the unit's model draw. Nothing when the unit's flags `+0xb3` share
    /// a bit with its character's `g_EtChrDrawFlags` byte (`u8 +0x18`;
    /// that byte is returned). Else `host +0x18` = unit `+0xc8`, `+0x10`
    /// and `+0x14` cleared, and for the selectors `i8 +0xa8` (value `f32
    /// +0x3c`) then `+0xa9` (`+0x4c`), two jump tables: 1 sets the scale
    /// (else `g_AlphaScaleNormal`, 256), 2 the alpha byte `+0x1b` (0xff
    /// above 255, 0 below 0, else truncated: a NaN gives 0), 3 `+0x10` and
    /// 4 `+0x14` = `ftol(value) << 16`. Then `+8`/`+9` = unit bytes
    /// `+0xc2`/`+0xc3`, `+0` = `u16 +0xc0`, `+0xc` = `+0xd0`; the matrix `m`
    /// = `host +0x80`: `op_unit_m_scale(m, f32(scale / 2^21))`,
    /// `op_rt_m(m, m, unit +0x30, unit +0x90)`, `op_mul_m(m,
    /// opus_parent_get(host, unit), m)`; `+0x1c`, `+0x38`, `+0x3c`, `+0x44`
    /// = 0, `+0x40` = unit `+0xc4`, `+0x20` = `+0x30` = `m`, `+4` = `set
    /// +0x28`, `+0x34` = `set +0x54`. The part record `part` (the
    /// original's 0x20-byte local; only `+0x1c` is written, the shape
    /// `set_ptr32(unit +0xe0)`) goes to instance `(0, unit)`:
    /// `updateVFXRenderData` and `graphicVFXUpdate` (its eax returned)
    /// when it has render data, else `InitVFXRenderData` with flags
    /// `alpha function(+8) | z(+0x44) unless +0 bit 0x2000 | 0x40 for bit
    /// 3` (its eax returned). All callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opp_md<M: Mem>(
        mem: &mut M,
        host: u32,
        set: u32,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load, mul, round_trip, store};
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let u = |k: u32| unit.wrapping_add(k);
        let h = |k: u32| host.wrapping_add(k);
        let chr = u32::from(mem.read::<u8>(u(0x18)));
        let flags: u8 = mem.read(g(mem, 0x012a_84c0).wrapping_add(chr));
        if mem.read::<u8>(u(0xb3)) & flags != 0 {
            return u32::from(flags);
        }
        let w: u32 = mem.read(u(0xc8));
        mem.write(h(0x18), w);
        let mut scale = round_trip(f(mem, g(mem, 0x00b5_01c0)));
        mem.write(h(0x10), 0u32);
        mem.write(h(0x14), 0u32);
        for (sel, at) in [(0xa8, 0x3c), (0xa9, 0x4c)] {
            let x = f(mem, u(at));
            match mem.read::<i8>(u(sel)) {
                1 => scale = round_trip(x),
                2 => {
                    let a = if x > 255.0 {
                        0xff
                    } else if x < 0.0 || x.is_nan() {
                        0
                    } else {
                        // fistp with the rounding control set to chop
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let t = x as u8;
                        t
                    };
                    mem.write(h(0x1b), a);
                }
                3 => mem.write(h(0x10), ftol(mem, load(x.to_bits())) << 16),
                4 => mem.write(h(0x14), ftol(mem, load(x.to_bits())) << 16),
                _ => {}
            }
        }
        let b: u8 = mem.read(u(0xc2));
        mem.write(h(8), b);
        let s = store(mul(load(scale.to_bits()), 4.768_371_582_031_25e-7));
        let b: u8 = mem.read(u(0xc3));
        mem.write(h(9), b);
        let w = u32::from(mem.read::<u16>(u(0xc0)));
        mem.write(host, w);
        let w: u32 = mem.read(u(0xd0));
        mem.write(h(0xc), w);
        let m = h(0x80);
        call(mem, g(mem, 0x007e_a700), &[m, s]);
        call(mem, g(mem, 0x007e_9850), &[m, m, u(0x30), u(0x90)]);
        let parent = call(mem, g(mem, 0x0080_1f00), &[host, unit]);
        call(mem, g(mem, 0x007e_83b0), &[m, parent, m]);
        mem.write(h(0x1c), 0u32);
        let w: u32 = mem.read(u(0xc4));
        mem.write(h(0x40), w);
        for k in [0x38, 0x3c] {
            mem.write(h(k), 0u32);
        }
        mem.write(h(0x20), m);
        mem.write(h(0x44), 0u32);
        let w: u32 = mem.read(set.wrapping_add(0x28));
        mem.write(h(4), w);
        mem.write(h(0x30), m);
        let w: u32 = mem.read(set.wrapping_add(0x54));
        mem.write(h(0x34), w);
        let shape: u32 = mem.read(u(0xe0));
        let shape = call(mem, g(mem, 0x00a4_4700), &[shape]);
        mem.write(part.wrapping_add(0x1c), shape);
        if call(mem, g(mem, 0x0064_53d0), &[0, unit]) != 0 {
            call(mem, g(mem, 0x0071_8610), &[0, unit, part, host]);
            return call(mem, g(mem, 0x0064_58a0), &[0, unit]);
        }
        let a = u32::from(mem.read::<u8>(h(8)));
        let mut fl = call(mem, g(mem, 0x0071_2c30), &[a]);
        let top: u32 = mem.read(host);
        if top & 0x2000 == 0 {
            let z: u32 = mem.read(h(0x44));
            fl |= call(mem, g(mem, 0x0071_2d00), &[z]);
        }
        if top & 8 != 0 {
            fl |= 0x40;
        }
        call(mem, g(mem, 0x0071_1180), &[0, unit, part, host, fl])
    }

    /// `opus_encount0_draw(host, unit, grid)` (`0x007f81e0`, from
    /// `opu_encount`): the encounter grid's particle matrices. `host`'s
    /// draw head: `+0x118` 0x80808080, `+0x110` 0, `u16 +0x108` = unit
    /// `+0xc2`, `+0x100` = `u16 +0xc0`, `+0x10c` = `+0xd0`, `+0x11c`,
    /// `+0x138`, `+0x144` 0, `+0x140` = `+0xc4`, `+0x120` = `host +0xc0`,
    /// `+0x104` = `host +0x328`. `host +0x40` = `op_unit_m_scale(f32(unit
    /// +0x3c / 2^21))`, its `+0x54` x= `unit +0x4c / 256`. The grid (`u16
    /// +2`, `+4`, `+6` its sides, `n` = `+2` x 64) has 32-byte records
    /// from `+0x30`: unless `host u16 +0x212`, each moves by the unit's
    /// `+0x60` (position) and `+0x50` (angles at `+0x10`). The record's
    /// position wraps into `[-n/2, n/2)` (`ftol` mod `n`) at `host +0`
    /// (`w` 1), `host +0x80` = `op_rt_m(host +0x40, angles, host)` x
    /// `opus_parent_get(host, unit)`, `host +0xc0` = `host +0x2c0` x that,
    /// `+0x130` = `host +0x80`, `+0x134` = `host +0x354`. Returns the
    /// original's eax: `u16 grid +4` after the last record, else 0.
    #[allow(clippy::many_single_char_names)]
    pub fn opus_encount0_draw<M: Mem>(
        mem: &mut M,
        host: u32,
        unit: u32,
        grid: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{Reg, ftol, round_trip};
        let h = |k: u32| host.wrapping_add(k);
        let u = |k: u32| unit.wrapping_add(k);
        let fl = |mem: &M, at: u32| f32::from_bits(mem.read(at));
        mem.write(h(0x118), 0x8080_8080u32);
        mem.write(h(0x110), 0u32);
        let scale = round_trip(fl(mem, u(0x3c)));
        let w: u16 = mem.read(u(0xc2));
        mem.write(h(0x108), w);
        let w = u32::from(mem.read::<u16>(u(0xc0)));
        mem.write(h(0x100), w);
        let w: u32 = mem.read(u(0xd0));
        mem.write(h(0x10c), w);
        mem.write(h(0x11c), 0u32);
        let w: u32 = mem.read(u(0xc4));
        mem.write(h(0x140), w);
        mem.write(h(0x138), 0u32);
        mem.write(h(0x144), 0u32);
        mem.write(h(0x120), h(0xc0));
        let w: u32 = mem.read(h(0x328));
        mem.write(h(0x104), w);
        let mv: [f32; 6] = [0x60, 0x64, 0x68, 0x50, 0x54, 0x58].map(|k| round_trip(fl(mem, u(k))));
        let s = (Reg::load(scale) * Reg::constant(4.768_371_582_031_25e-7)).store();
        call(mem, g(mem, 0x007e_a700), &[h(0x40), s.to_bits()]);
        let y = (Reg::load(fl(mem, u(0x4c))) * Reg::constant(0.003_906_25))
            .mul_m32(fl(mem, h(0x54)))
            .store();
        mem.write(h(0x54), y);
        let side = |mem: &M, k: u32| u32::from(mem.read::<u16>(grid.wrapping_add(k)));
        let n = side(mem, 2) << 6;
        let half = n.cast_signed() / 2;
        let count = |mem: &M| {
            side(mem, 6)
                .wrapping_mul(side(mem, 4))
                .wrapping_mul(side(mem, 2))
        };
        let mut ret = 0;
        let mut rec = grid.wrapping_add(0x30);
        let mut i = 0u32;
        while i.cast_signed() < count(mem).cast_signed() {
            if mem.read::<u16>(h(0x212)) == 0 {
                for (k, at) in [0u32, 4, 8, 0x10, 0x14, 0x18].into_iter().enumerate() {
                    let p = rec.wrapping_add(at);
                    let v = Reg::load(mv[k]).add_m32(fl(mem, p)).store();
                    mem.write(p, v);
                }
            }
            for k in 0..3u32 {
                let v =
                    ftol(mem, Reg::load(fl(mem, rec.wrapping_add(4 * k))).value()).cast_signed();
                let mut r = v.wrapping_rem(n.cast_signed());
                if r < 0 {
                    r = r.wrapping_add(n.cast_signed());
                }
                #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
                let f = r.wrapping_sub(half) as f32;
                mem.write(h(4 * k), f);
            }
            mem.write(h(0xc), 1.0f32);
            call(
                mem,
                g(mem, 0x007e_9850),
                &[h(0x80), h(0x40), rec.wrapping_add(0x10), host],
            );
            let parent = call(mem, g(mem, 0x0080_1f00), &[host, unit]);
            call(mem, g(mem, 0x007e_83b0), &[h(0x80), parent, h(0x80)]);
            call(mem, g(mem, 0x007e_83b0), &[h(0xc0), h(0x2c0), h(0x80)]);
            i += 1;
            mem.write(h(0x130), h(0x80));
            let w: u32 = mem.read(h(0x354));
            mem.write(h(0x134), w);
            ret = side(mem, 4);
            rec = rec.wrapping_add(0x20);
        }
        ret
    }

    /// `opus_encount5_make(set, pos)` (`0x007f9280`): the encounter
    /// swirl's 0x740 particles, a ring of 96-byte records from `set +0x20`
    /// round a 512 x 416 rectangle centred on `A = f32(pos.x + 2048)`, `B =
    /// f32(pos.y + 2048)`. `g_BtlReadHold` = 0x5a, `set +0` = 0. With `k` =
    /// 1/512, `x0 = A - 256`, `y0 = B - 208` (unrounded): the bottom edge
    /// (512, `i` up) at `+0x10` (`f32(i + x0)`, `f32(y0)`, 0) and texture
    /// `+0x38` (`i k`, 0); the right edge (416) (`f32(A + 256)`, `f32(i +
    /// y0)`, 0) and (1, `i k`); the top (512, `i` down) (`f32(i + x0)`,
    /// `f32(B + 208)`, 0) and (`i k`, 0.8125); the left (416, `i` down)
    /// (`f32(x0)`, `f32(i + y0)`, 0) with its texture pair (0, `i k`) one
    /// record on, so the left edge's first record keeps its old `+0x38` and
    /// the record after the last gets one. Then every record: `+0` (`A`,
    /// `B`, 0), colours `+0x20`/`+0x24` 0x80808080, `+0x30` (`f32((A -
    /// 1792) k)`, `f32(k (B - 1840))`), `+0x40..+0x50` 0 and `+0x54` =
    /// `f32(op_random_p(0.001) + 0.002)`. Returns
    /// `graphicActivateRadialBlur()` through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opus_encount5_make<M: Mem>(
        mem: &mut M,
        set: u32,
        pos: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        const K: f64 = 0.001_953_125;
        const N: [u32; 4] = [0x200, 0x1a0, 0x200, 0x1a0];
        Global::<u32>::new(0x0112_ca64).set(mem, 0x5a);
        mem.write(set, 0u32);
        let a = store(add(load(mem.read(pos)), 2048.0));
        let b = store(add(2048.0, load(mem.read(pos.wrapping_add(4)))));
        let x0 = sub(load(a), 256.0);
        let y0 = sub(load(b), 208.0);
        let rec = |j: u32| set.wrapping_add(0x20).wrapping_add(0x60 * j);
        let put = |mem: &mut M, j: u32, at: u32, v: u32| mem.write(rec(j).wrapping_add(at), v);
        #[allow(clippy::cast_precision_loss)] // below 0x200: exact
        let fi = |i: u32| f64::from(i as f32);
        let (zero, one) = (0u32, 1.0f32.to_bits());
        let mut j = 0;
        // bottom, right, top, left
        for (edge, n) in (0..).zip(N) {
            let (right, top) = (store(add(load(a), 256.0)), store(add(load(b), 208.0)));
            for step in 0..n {
                let i = if edge < 2 { step } else { n - 1 - step };
                let (x, y, u, v) = match edge {
                    0 => (store(add(fi(i), x0)), store(y0), store(mul(fi(i), K)), zero),
                    1 => (right, store(add(fi(i), y0)), one, store(mul(fi(i), K))),
                    2 => (
                        store(add(fi(i), x0)),
                        top,
                        store(mul(fi(i), K)),
                        0x3f50_0000,
                    ),
                    _ => (store(x0), store(add(fi(i), y0)), zero, store(mul(fi(i), K))),
                };
                put(mem, j, 0x10, x);
                put(mem, j, 0x14, y);
                put(mem, j, 0x18, zero);
                let t = if edge == 3 { j + 1 } else { j };
                put(mem, t, 0x38, u);
                put(mem, t, 0x3c, v);
                j += 1;
            }
        }
        let u = store(mul(sub(load(a), 1792.0), K));
        let v = store(mul(K, sub(load(b), 1840.0)));
        // the f64 0x00b4c168 is the f32 0.002 widened
        let speed = f64::from(0.002f32);
        for k in 0..j {
            put(mem, k, 0, a);
            put(mem, k, 0x20, 0x8080_8080);
            put(mem, k, 0x24, 0x8080_8080);
            put(mem, k, 4, b);
            put(mem, k, 8, zero);
            put(mem, k, 0x30, u);
            put(mem, k, 0x34, v);
            for at in (0x40..=0x50).step_by(4) {
                put(mem, k, at, zero);
            }
            let r = crate::rand::bridge::op_random_p(mem, f32::from_bits(0x3a83_126f));
            put(mem, k, 0x54, store(add(load(r.to_bits()), speed)));
        }
        call(mem, g(mem, 0x0063_9140), &[])
    }

    /// `opus_encount4_size_get(oef, first, n)` (`0x007f8e30`): the packet
    /// size of `n` encounter parts: `(7 n + 0x11 + the sum of i16 +0x12 of
    /// op_oef2_a_get(oef, 4, first + i)) * 16`.
    pub fn opus_encount4_size_get(mem: &impl Mem, oef: u32, first: u32, n: i32) -> u32 {
        let mut sum = 0u32;
        for i in 0..n.max(0).cast_unsigned() {
            let p = crate::op::bridge::op_oef2_a_get(mem, oef, 4, first.wrapping_add(i));
            sum =
                sum.wrapping_add(i32::from(mem.read::<i16>(p.wrapping_add(0x12))).cast_unsigned());
        }
        n.cast_unsigned()
            .wrapping_mul(7)
            .wrapping_add(sum)
            .wrapping_add(0x11)
            << 4
    }

    fn g(mem: &impl Mem, a: u32) -> u32 {
        Global::<u32>::new(a).ptr(mem).addr
    }

    fn f(mem: &impl Mem, a: u32) -> f32 {
        mem.read(a)
    }

    /// `opu_main_host`'s 64-byte copy through the emulated EE registers
    /// `v0`..`a1` (`0x00c8f528`.., [`crate::md::bridge::SPR_BOUNCE`]):
    /// words 1..15 (and word 0 with `w0`) into the registers, then all of
    /// them to `dst`, word 0 straight from `src`.
    fn ee_copy64(mem: &mut impl Mem, src: u32, dst: u32, w0: bool) {
        let regs = crate::md::bridge::SPR_BOUNCE.ptr(mem).addr;
        let first: u32 = mem.read(src);
        if w0 {
            mem.write(regs, first);
        }
        for i in 1..16u32 {
            let x: u32 = mem.read(src.wrapping_add(4 * i));
            mem.write(regs + 4 * i, x);
        }
        mem.write(dst, first);
        for i in 1..16u32 {
            let x: u32 = mem.read(regs + 4 * i);
            mem.write(dst.wrapping_add(4 * i), x);
        }
    }

    /// The emulated VU registers `opu_main_host` integrates a unit's
    /// motion in (`0x00c8f8d0`..`0x00c8f93c`).
    const VF: u32 = 0x00c8_f8d0;

    /// `opu_main_host`'s motion step of unit record `u` (and its draw
    /// callback): position/velocity/acceleration in the emulated VU
    /// registers, unless `+0xb2` bit 1 (or the host's `+0x212` is set):
    /// velocity += acceleration, position += velocity; then, unless
    /// `+0xb2` bit 2, `+0x90`.. = the second position row / 16 and the
    /// callback `0x00c492c8[+0xba]` (1..11) with `(host, ptr)`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn motion<M: Mem>(
        mem: &mut M,
        host: u32,
        u: u32,
        ptr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let vf = g(mem, VF);
        let regs = crate::md::bridge::SPR_BOUNCE.ptr(mem).addr;
        let r = |o: u32| vf + (o - VF);
        let at = |o: u32| u.wrapping_add(o);
        let flags = mem.read::<u8>(at(0xb2));
        mem.write(regs, 0x3d80_0000u32);
        for i in 0..8u32 {
            let x: u32 = mem.read(at(0x30 + 4 * i));
            mem.write(r(0x00c8_f8d0) + 4 * i, x);
        }
        for (d, s) in [(0x00c8_f934u32, 4u32), (0x00c8_f938, 8)] {
            let x: u32 = mem.read(regs + s);
            mem.write(r(d), x);
        }
        let x: u32 = mem.read(regs + 0xc);
        mem.write(r(0x00c8_f930), 0x3d80_0000u32);
        mem.write(r(0x00c8_f93c), x);
        if flags & 2 == 0 {
            let k = i32::from(mem.read::<i16>(host.wrapping_add(0x212)));
            mem.write(regs, k);
            for i in 0..4u32 {
                let x: u32 = mem.read(at(0x50 + 4 * i));
                mem.write(r(0x00c8_f8f0) + 4 * i, x);
            }
            if k == 0 {
                for i in 0..12u32 {
                    let x: u32 = mem.read(at(0x60 + 4 * i));
                    mem.write(r(0x00c8_f900) + 4 * i, x);
                }
                // velocity (+0x50, 8 floats) += acceleration (+0x70)
                for i in 0..8u32 {
                    let v = r(0x00c8_f8f0) + 4 * i;
                    let a = r(0x00c8_f910) + 4 * i;
                    let x = crate::x87::add(f(mem, v), f(mem, a));
                    mem.write(v, x);
                }
                for i in 0..8u32 {
                    let x: u32 = mem.read(r(0x00c8_f8f0) + 4 * i);
                    mem.write(at(0x50 + 4 * i), x);
                }
                // position (+0x30, 8 floats) += velocity
                for i in 0..8u32 {
                    let p = r(0x00c8_f8d0) + 4 * i;
                    let v = r(0x00c8_f8f0) + 4 * i;
                    let x = crate::x87::add(f(mem, p), f(mem, v));
                    mem.write(p, x);
                }
                for i in 0..8u32 {
                    let x: u32 = mem.read(r(0x00c8_f8d0) + 4 * i);
                    mem.write(at(0x30 + 4 * i), x);
                }
            }
        }
        if mem.read::<u8>(at(0xb2)) & 4 != 0 {
            return;
        }
        let k = Reg::load_f32(f(mem, r(0x00c8_f930)));
        let p = r(0x00c8_f8e0);
        let x0 = Reg::load_f32(f(mem, p)).reg(MUL, k).store();
        mem.write(p, x0);
        let x1 = Reg::load_f32(f(mem, p + 4)).reg(MUL, k).store();
        mem.write(p + 4, x1);
        let x2 = k.mem(MUL, f(mem, p + 8)).store();
        mem.write(p + 8, x2);
        let x3 = k.mem(MUL, f(mem, p + 12)).store();
        mem.write(p + 12, x3);
        for i in 0..4u32 {
            let x: u32 = mem.read(p + 4 * i);
            mem.write(at(0x90 + 4 * i), x);
        }
        let cb = mem.read::<u8>(at(0xba));
        if cb != 0 && cb < 0xc {
            let table = g(mem, 0x00c4_92c8);
            let fp: u32 = mem.read(table + 4 * u32::from(cb));
            call(mem, fp, &[host, ptr]);
        }
    }

    /// `opp_main(battle)` (`0x0080bd50`, called by `op_effect_run_after`):
    /// runs the draw list of battle `battle` (`g_OpusBattle[battle]`, low
    /// 16 bits; `g_VfxPass` = 1 for battle 1, 0 at the end). The host
    /// record comes from the SPR stack (`op_spr_alloc(0x400)`): `g_Ee_Gpr`'s
    /// `r0` at `+0x210`, `+0x212` = 1 when both bytes `0x02322780`/`81`
    /// share a bit or `Sg_GetCurExecFrames` is 0, `+0x21e` = 1 when
    /// `MsBattleChrStart`, the listener (`0x0113fcd0`) at `+0x280` and the
    /// matrix at `0x0113fd10` at `+0x2c0` (both through the emulated
    /// registers at `0x00c8f528`, the first without their word 0).
    ///
    /// The draw list `[battle +0x1c]` holds `i16` part numbers ending at a
    /// negative one, indexed by the host's `+0x218` (re-read each time:
    /// the callbacks may move it). For part `n` (`+0x214` = `n`, `+0x220` =
    /// `[battle +0x20] + 0x100 n`), with part byte `+0x19` bit 7 the matrix
    /// at `0x0113fd50` goes in at `+0x2c0` for its call and the one at
    /// `0x0113fd10` comes back after. Part byte `+0xbb` picks the callback
    /// `(host, battle)`: with bit 7 `[[battle +0x50] +0x78][low 7 bits]`
    /// (no null check), else `0x00c48e78[byte]`, which when null only
    /// prints a warning (`dbgPrintf`).
    ///
    /// Callees through `call`. Returns `eax` as the original leaves it:
    /// the terminating list word, zero-extended (`op_spr_free` keeps
    /// `eax`; the caller does not read it).
    #[allow(clippy::many_single_char_names)]
    pub fn opp_main<M: Mem>(
        mem: &mut M,
        battle: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let battle = battle & 0xffff;
        let pass = g(mem, 0x0113_335c);
        if battle == 1 {
            mem.write(pass, 1u32);
        }
        let alloc = g(mem, 0x007e_6a00);
        let host = call(mem, alloc, &[0x400]);
        let h = |o: u32| host.wrapping_add(o);
        let ob = OPUS_BATTLE.ptr(mem).addr;
        let b: u32 = mem.read(ob + 4 * battle);
        let parts: u32 = mem.read(b.wrapping_add(0x20));
        let r0 = crate::ppp::bridge::R0.ptr(mem).addr;
        for i in 0..4u32 {
            let x: u32 = mem.read(r0 + 4 * i);
            mem.write(h(0x210 + 4 * i), x);
        }
        let s0: u8 = mem.read(g(mem, 0x0232_2780));
        let s1: u8 = mem.read(g(mem, 0x0232_2781));
        let still = if s0 & s1 != 0 {
            true
        } else {
            let frames = g(mem, 0x0082_d7e0);
            call(mem, frames, &[]) == 0
        };
        mem.write(h(0x212), u16::from(still));
        let start = g(mem, 0x0078_0cb0);
        if call(mem, start, &[]) != 0 {
            mem.write(h(0x21e), 1u8);
        }
        let (listener, normal, marked) = (
            g(mem, 0x0113_fcd0),
            g(mem, 0x0113_fd10),
            g(mem, 0x0113_fd50),
        );
        ee_copy64(mem, listener, h(0x280), false);
        ee_copy64(mem, normal, h(0x2c0), true);
        let list: u32 = mem.read(b.wrapping_add(0x1c));
        let own = g(mem, 0x00c4_8e78);
        let end = loop {
            let idx = i32::from(mem.read::<i16>(h(0x218))).cast_unsigned();
            let v: u16 = mem.read(list.wrapping_add(idx.wrapping_mul(2)));
            if v.cast_signed() < 0 {
                break v;
            }
            mem.write(h(0x214), v);
            let part = parts.wrapping_add(i32::from(v.cast_signed()).cast_unsigned() << 8);
            mem.write(h(0x220), part);
            if mem.read::<u8>(part.wrapping_add(0x19)) & 0x80 != 0 {
                ee_copy64(mem, marked, h(0x2c0), true);
            }
            let sel: u8 = mem.read(part.wrapping_add(0xbb));
            let fp = if sel & 0x80 != 0 {
                let e: u32 = mem.read(b.wrapping_add(0x50));
                let t: u32 = mem.read(e.wrapping_add(0x78));
                Some(mem.read::<u32>(t.wrapping_add(4 * u32::from(sel & 0x7f))))
            } else {
                Some(mem.read::<u32>(own + 4 * u32::from(sel))).filter(|&f| f != 0)
            };
            if let Some(fp) = fp {
                call(mem, fp, &[host, b]);
            } else {
                let text = g(mem, 0x00b5_8a50);
                let dbg = g(mem, 0x0062_f500);
                call(mem, dbg, &[text]);
            }
            let i: u16 = mem.read(h(0x218));
            mem.write(h(0x218), i.wrapping_add(1));
            if mem.read::<u8>(part.wrapping_add(0x19)) & 0x80 != 0 {
                ee_copy64(mem, normal, h(0x2c0), true);
            }
        };
        let free = g(mem, 0x007e_6a60);
        call(mem, free, &[0x400]);
        mem.write(pass, 0u32);
        u32::from(end)
    }

    /// `opu_main_host(battle, list)` (`0x0080cc00`): runs the opus units
    /// of battle `battle` (`g_OpusBattle[battle]`, `g_VfxPass` = 1 for
    /// battle 1). The host record comes from the SPR stack
    /// (`op_spr_alloc(0x400)`, [`super::bridge`]'s `g_OpusBattleList`
    /// while it runs): the listener (`0x0113fcd0`, 64 bytes) at `+0x280`,
    /// the block at `0x0113fd10` at `+0x2c0`, the battle's first 0x100
    /// bytes at `+0x300` (copied back at the end), `g_Ee_Gpr`'s `r0` at
    /// `+0x210` with `+0x21c` = `list`, the unit order `+0x224` =
    /// `[+0x310 + 4 list]` (`i16`s ending at -1), `+0x21e` = 1 when
    /// `MsBattleChrStart`.
    ///
    /// The order is walked twice, the second time (`+0x21d` = 0xff) only
    /// for slots marked to run once more. Each unit (0x100 bytes from
    /// `+0x320`) has four program slots (`+0` pointers, `+0x10` wait
    /// bytes, `+0x1f` mark bits): a slot runs its commands (`i16`, low
    /// byte the handler in `0x00c48ec8`, or with 0x100 the battle's
    /// overlay table `[[battle +0x50] +0x7c]`) until a handler sets the
    /// host's `+0x210`, which then adds `+0x210 - 1` to the wait. After the
    /// first pass's slots comes the unit's [`motion`] step (straight away
    /// with `+0x21f`).
    ///
    /// Callees through `call`. Returns the last copied word (`eax` after
    /// `op_spr_free`).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_main_host<M: Mem>(
        mem: &mut M,
        battle: u32,
        list: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let battle = battle & 0xffff;
        let pass = g(mem, 0x0113_335c);
        if battle == 1 {
            mem.write(pass, 1u32);
        }
        let alloc = g(mem, 0x007e_6a00);
        let host = call(mem, alloc, &[0x400]);
        let ob = OPUS_BATTLE.ptr(mem).addr;
        let b: u32 = mem.read(ob + 4 * battle);
        let src1 = g(mem, 0x0113_fcd0);
        ee_copy64(mem, src1, host.wrapping_add(0x280), false);
        let hl = g(mem, 0x012a_12fc);
        let src2 = g(mem, 0x0113_fd10);
        ee_copy64(mem, src2, host.wrapping_add(0x2c0), true);
        for k in 0..4u32 {
            ee_copy64(
                mem,
                b.wrapping_add(0x40 * k),
                host.wrapping_add(0x300 + 0x40 * k),
                true,
            );
        }
        mem.write(hl, host);
        let h = |o: u32| host.wrapping_add(o);
        let r0 = crate::ppp::bridge::R0.ptr(mem).addr;
        for i in 0..4u32 {
            let x: u32 = mem.read(r0 + 4 * i);
            mem.write(h(0x210 + 4 * i), x);
        }
        #[allow(clippy::cast_possible_truncation)] // the byte argument
        let lb = list as u8;
        mem.write(h(0x21c), lb);
        let order: u32 = mem.read(h(0x310 + 4 * u32::from(lb)));
        mem.write(h(0x224), order);
        let start = g(mem, 0x0078_0cb0);
        if call(mem, start, &[]) != 0 {
            mem.write(h(0x21e), 1u8);
        }
        let get = g(mem, 0x00a4_4650);
        let set = g(mem, 0x00a4_4700);
        let handlers = g(mem, 0x00c4_8ec8);
        let cur = g(mem, 0x012f_4118);
        let mut ptr = b;
        loop {
            let order: u32 = mem.read(h(0x224));
            let idx = i32::from(mem.read::<i16>(h(0x218)));
            let mut v: u16 = mem.read(order.wrapping_add(idx.cast_unsigned().wrapping_mul(2)));
            while v == 0xffff {
                if mem.read::<u8>(h(0x21d)) != 0 {
                    return finish(mem, host, b, pass, call);
                }
                mem.write(h(0x218), 0u16);
                mem.write(h(0x21d), 0xffu8);
                v = mem.read(order);
            }
            let units: u32 = mem.read(h(0x320));
            let u = (i32::from(v.cast_signed()).cast_unsigned() << 8).wrapping_add(units);
            mem.write(h(0x214), v);
            mem.write(h(0x220), u);
            mem.write(h(0x216), 0u16);
            let mut run_motion = false;
            loop {
                if mem.read::<u8>(h(0x21f)) != 0 && mem.read::<u8>(h(0x21d)) == 0 {
                    run_motion = true;
                    break;
                }
                let s = i32::from(mem.read::<i16>(h(0x216)));
                let su = s.cast_unsigned();
                let slot_at = u.wrapping_add(su.wrapping_mul(4));
                let sp: u32 = mem.read(slot_at);
                ptr = call(mem, get, &[sp]);
                'slot: {
                    if ptr == 0 {
                        break 'slot;
                    }
                    let bit = 1u8.wrapping_shl(su & 0x1f);
                    let bit32 = 1u32.wrapping_shl(su & 0x1f);
                    let marks: u8 = mem.read(u.wrapping_add(0x1f));
                    if mem.read::<u8>(h(0x21d)) == 0 {
                        if u32::from(marks) & bit32 != 0 {
                            break 'slot;
                        }
                    } else {
                        if u32::from(marks) & bit32 == 0 {
                            break 'slot;
                        }
                        mem.write(u.wrapping_add(0x1f), !bit & marks & 0xf);
                    }
                    let s = i32::from(mem.read::<i16>(h(0x216))).cast_unsigned();
                    let wait_at = u.wrapping_add(0x10).wrapping_add(s);
                    let w: u8 = mem.read(wait_at);
                    if w != 0 {
                        mem.write(wait_at, w.wrapping_sub(1));
                        break 'slot;
                    }
                    loop {
                        let c = if ptr == 0 {
                            0u32
                        } else {
                            i32::from(mem.read::<i16>(ptr)).cast_unsigned()
                        };
                        mem.write(cur, ptr);
                        #[allow(clippy::cast_possible_truncation)] // the u16 store
                        mem.write(h(0x21a), c as u16);
                        let fp = if c & 0x100 != 0 {
                            let e: u32 = mem.read(b.wrapping_add(0x50));
                            let t: u32 = mem.read(e.wrapping_add(0x7c));
                            if t == 0 {
                                let fmt = g(mem, 0x00b5_8aa0);
                                let dbg = g(mem, 0x0062_f500);
                                let cx =
                                    i32::from(u16::try_from(c & 0xffff).unwrap_or(0).cast_signed())
                                        .cast_unsigned();
                                call(mem, dbg, &[fmt, cx]);
                                break;
                            }
                            mem.read::<u32>(t.wrapping_add(4 * (c & 0xff)))
                        } else {
                            let fp: u32 = mem.read(handlers + 4 * (c & 0xff));
                            if fp == 0 {
                                break;
                            }
                            fp
                        };
                        ptr = call(mem, fp, &[host, ptr]);
                        if mem.read::<u8>(h(0x210)) != 0 {
                            break;
                        }
                    }
                    let d: u8 = mem.read(h(0x210));
                    let s = i32::from(mem.read::<i16>(h(0x216))).cast_unsigned();
                    let wait_at = u.wrapping_add(0x10).wrapping_add(s);
                    let w: u8 = mem.read(wait_at);
                    mem.write(wait_at, w.wrapping_add(d.wrapping_sub(1)));
                    mem.write(h(0x210), 0u8);
                    if ptr != 0 && mem.read::<u8>(h(0x21d)) != 0 {
                        let s = mem.read::<u8>(h(0x216));
                        let m: u8 = mem.read(u.wrapping_add(0x1f));
                        mem.write(
                            u.wrapping_add(0x1f),
                            m | 1u8.wrapping_shl(u32::from(s & 0x1f)),
                        );
                    }
                    let saved = call(mem, set, &[ptr]);
                    let s = i32::from(mem.read::<i16>(h(0x216))).cast_unsigned();
                    mem.write(u.wrapping_add(s.wrapping_mul(4)), saved);
                }
                let s: u16 = mem.read(h(0x216));
                mem.write(h(0x216), s.wrapping_add(1));
                if s.wrapping_add(1).cast_signed() >= 4 {
                    break;
                }
            }
            if run_motion || mem.read::<u8>(h(0x21d)) == 0 {
                motion(mem, host, u, ptr, call);
            }
            let i: u16 = mem.read(h(0x218));
            mem.write(h(0x218), i.wrapping_add(1));
        }
    }

    /// `opu_main_host`'s end: the battle's 0x100 bytes back, the list and
    /// the SPR block released, `g_VfxPass` 0.
    fn finish<M: Mem>(
        mem: &mut M,
        host: u32,
        b: u32,
        pass: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        for k in 0..4u32 {
            ee_copy64(
                mem,
                host.wrapping_add(0x300 + 0x40 * k),
                b.wrapping_add(0x40 * k),
                true,
            );
        }
        let regs = crate::md::bridge::SPR_BOUNCE.ptr(mem).addr;
        let last: u32 = mem.read(regs + 0x3c);
        let hl = g(mem, 0x012a_12fc);
        mem.write(hl, 0u32);
        let free = g(mem, 0x007e_6a60);
        call(mem, free, &[0x400]);
        mem.write(pass, 0u32);
        last
    }

    /// `opu_mdc_0x2000_ambient(rec)` (`0x008041c0`): for the magic file's
    /// id (`MagicFile_GetMagicId`, through `call`) among the seven words at
    /// `0x00c48e44` or in 0x26e..=0x275, the record's ambient (`u16 +0x1d`,
    /// byte `+0x1c`) all 0x80. Returns 0.
    pub fn opu_mdc_0x2000_ambient<M: Mem>(
        mem: &mut M,
        rec: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id = call(mem, g(mem, 0x009d_a3b0), &[]);
        let ids: [u32; 7] = mem.read(g(mem, 0x00c4_8e44));
        if ids.contains(&id) || id.wrapping_sub(0x26e) <= 7 {
            mem.write(rec.wrapping_add(0x1d), 0x8080u16);
            mem.write(rec.wrapping_add(0x1c), 0x80u8);
        }
        0
    }

    /// The unit `host +0x220`.
    fn unit(mem: &impl Mem, host: u32) -> u32 {
        mem.read(host.wrapping_add(0x220))
    }

    /// The flag nibble's bits (`u16 host +0x21a & 0xf000`).
    fn nibble(mem: &impl Mem, host: u32) -> u16 {
        mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000
    }

    /// `opp_et_ovl(host)` (`0x00813460`): the unit's word `+0xac`
    /// (`get_ptr32_func`, the identity here).
    pub fn opp_et_ovl(mem: &impl Mem, host: u32) -> u32 {
        ffx_system::stub::ptr32(mem.read(unit(mem, host).wrapping_add(0xac)))
    }

    /// `0x00817280` (`opu_return`, ours): the address [`opu_call`] saved
    /// in the unit's slot `i16 host +0x216` (`+0x20 + 4 i`), through
    /// `get_ptr32_func`.
    pub fn opu_return(mem: &impl Mem, host: u32) -> u32 {
        let i = i32::from(mem.read::<i16>(host.wrapping_add(0x216))).cast_unsigned();
        let slot = unit(mem, host)
            .wrapping_add(0x20)
            .wrapping_add(i.wrapping_mul(4));
        ffx_system::stub::ptr32(mem.read(slot))
    }

    /// `0x00815760` (`opu_call`, ours): `cmd + 4` (`set_ptr32`) into the
    /// unit's slot `i16 host +0x216`, then a jump by the command's `i16 +2`
    /// (from `cmd`, read after the store).
    pub fn opu_call(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let u = unit(mem, host);
        let back = ffx_system::stub::ptr32(cmd.wrapping_add(4));
        let i = i32::from(mem.read::<i16>(host.wrapping_add(0x216))).cast_unsigned();
        mem.write(u.wrapping_add(0x20).wrapping_add(i.wrapping_mul(4)), back);
        let d = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        d.wrapping_add(cmd)
    }

    /// `0x0080c650` (`opu_end_no`, ours): when unit `u16 cmd +2` of the
    /// list `host +0x300` ([`opus_no_get`]) is the list's base `[host
    /// +0x320]` (none found), [`opu_end`]; else the host waits (`+0x210` =
    /// 1) and `cmd` comes back.
    pub fn opu_end_no<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        let list = host.wrapping_add(0x300);
        let u = opus_no_get(mem, list, unit(mem, host), no);
        if u == mem.read::<u32>(list.wrapping_add(0x20)) {
            return opu_end(mem, host, cmd, call);
        }
        mem.write(host.wrapping_add(0x210), 1u8);
        cmd
    }

    /// `0x00815710` (`opu_kind8`, ours): the unit's `+0xa8` = `cmd + 4`,
    /// its draw kind `+0xbb` = 8, the host's unit number `u16 +0x214`
    /// added to the list `host +0x300` (`opu_id_list_add`). Returns `cmd +
    /// 6 + 2 n` for the command's `u16 +2` as an `i16` (read first).
    pub fn opu_kind8(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let u = unit(mem, host);
        let n = mem.read::<i16>(cmd.wrapping_add(2));
        mem.write(u.wrapping_add(0xa8), cmd.wrapping_add(4));
        mem.write(u.wrapping_add(0xbb), 8u8);
        let id: u16 = mem.read(host.wrapping_add(0x214));
        crate::op::bridge::opu_id_list_add(mem, host.wrapping_add(0x300), id);
        let skip = i32::from(n).cast_unsigned().wrapping_mul(2);
        cmd.wrapping_add(6).wrapping_add(skip)
    }

    /// `opu_signal(host, cmd)` (`0x008172b0`): with no flag bits
    /// `AtelSetEffectFlag(u16 cmd +2)` and `cmd + 4`; with nibble 1 `cmd +
    /// 4` once `AtelGetEffectFlag` of it is set, else the host waits
    /// (`+0x210` = 1) on `cmd`; other nibbles return `cmd` (both through
    /// `call`).
    pub fn opu_signal<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let bit = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        match nibble(mem, host) {
            0 => {
                call(mem, g(mem, 0x0086_fa40), &[bit]);
                cmd.wrapping_add(4)
            }
            0x1000 => {
                if call(mem, g(mem, 0x0086_b030), &[bit]) != 0 {
                    return cmd.wrapping_add(4);
                }
                mem.write(host.wrapping_add(0x210), 1u8);
                cmd
            }
            _ => cmd,
        }
    }

    /// `opu_vibration(host, cmd)` (`0x008143e0`): with no flag bits, the
    /// pad's vibration stream: table `i16 cmd +2` of kind 5 of the unit's
    /// oef (`[host +0x360 + 4 * byte unit +0x1e]`, `op_oef2_a_get`), then
    /// `TkStopVibrate(7)` and `TkSetVibrateStream(7, i16 cmd +4, table)`
    /// (through `call`); returns `cmd + 6`. Otherwise `cmd`.
    pub fn opu_vibration<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if nibble(mem, host) != 0 {
            return cmd;
        }
        let k = u32::from(mem.read::<u8>(unit(mem, host).wrapping_add(0x1e)));
        let oef: u32 = mem.read(host.wrapping_add(0x360).wrapping_add(4 * k));
        let n = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let table = crate::op::bridge::op_oef2_a_get(mem, oef, 5, n);
        call(mem, g(mem, 0x0087_6290), &[7]);
        let curve = i32::from(mem.read::<i16>(cmd.wrapping_add(4))).cast_unsigned();
        call(mem, g(mem, 0x0087_5fb0), &[7, curve, table]);
        cmd.wrapping_add(6)
    }

    /// `0x00814440` (`opu_stone`, ours): the unit's character (`byte +0x18`
    /// of `g_EtChrHandles`) found in `g_EtChr` (`opus_get_chr_status_index(h,
    /// 0, 0)`, -1 for none takes the record before) gets its stone bit
    /// (`+0x17` bit 0) set with no flag bits or cleared with nibble 1 (both
    /// `cmd + 4`); other nibbles return `cmd`.
    pub fn opu_stone<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let chr = u32::from(mem.read::<u8>(unit(mem, host).wrapping_add(0x18)));
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        let i = crate::et::bridge::opus_get_chr_status_index(mem, h, 0, 0, call);
        let base = crate::et::bridge::CHR.ptr(mem).addr;
        let at = (i.cast_unsigned() << 5)
            .wrapping_add(base)
            .wrapping_add(0x17);
        match nibble(mem, host) {
            0 => mem.write(at, mem.read::<u8>(at) | 1),
            0x1000 => mem.write(at, mem.read::<u8>(at) & 0xfe),
            _ => return cmd,
        }
        cmd.wrapping_add(4)
    }

    /// `0x0080adc0` (`opu_prog_clear`, ours): for flag nibble `k` not 0 the
    /// unit's program pointer `+4 k` = 0 (`set_ptr32(0)`) and its byte
    /// `+0x1f` loses bit `k` (`!(1 << k)` in a byte, 0 from bit 8 up, kept
    /// to the low four bits); for 0 the pointers `+4`, `+8`, `+0xc` = 0 and
    /// `+0x1f` keeps bit 0. Returns `cmd + 2`.
    pub fn opu_prog_clear(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let u = unit(mem, host);
        let k = u32::from(mem.read::<u16>(host.wrapping_add(0x21a)) >> 12);
        let zero = ffx_system::stub::ptr32(0);
        let flags = u.wrapping_add(0x1f);
        if k == 0 {
            mem.write(flags, mem.read::<u8>(flags) & 1);
            for off in [0xc, 8, 4] {
                mem.write(u.wrapping_add(off), zero);
            }
        } else {
            mem.write(u.wrapping_add(4 * k), zero);
            // read after the store: slot 7 holds the flags byte
            let bit = 1u8.checked_shl(k).unwrap_or(0);
            mem.write(flags, mem.read::<u8>(flags) & !bit & 0xf);
        }
        cmd.wrapping_add(2)
    }

    /// `0x008144b0` (`opu_chr_switch`, ours): with no flag bits, a jump
    /// through the command's table (`cmd + i16 cmd +2`) by the unit's
    /// character's effect table number (`Ch_IdGetEffectTable` of its
    /// `g_EtChrHandles` entry, `byte unit +0x18`, through `call`; from 10
    /// up the original prints and calls `debug_exit_trace(1)` first): `t +
    /// 2 i + i16 [t + 2 i]`. Otherwise `cmd`.
    pub fn opu_chr_switch<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let chr = u32::from(mem.read::<u8>(unit(mem, host).wrapping_add(0x18)));
        if nibble(mem, host) != 0 {
            return cmd;
        }
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        let i = call(mem, g(mem, 0x0083_d7e0), &[h]);
        if i.cast_signed() >= 10 {
            let fmt = g(mem, 0x00b5_8b2c);
            call(mem, g(mem, 0x0062_f500), &[fmt, i]);
            call(mem, g(mem, 0x0062_f510), &[1]);
        }
        let t = i32::from(mem.read::<i16>(cmd.wrapping_add(2)))
            .cast_unsigned()
            .wrapping_add(cmd);
        let e = t.wrapping_add(i.wrapping_mul(2));
        i32::from(mem.read::<i16>(e))
            .cast_unsigned()
            .wrapping_add(e)
    }

    /// Points `g_pppEnvSlot`'s target at the op layer's environment
    /// (`0x012f2070`).
    fn op_env(mem: &mut impl Mem) {
        let slot = crate::heap::bridge::ENV_SLOT.get(mem);
        mem.write(slot, g(mem, 0x012f_2070));
    }

    /// `op_part_free(part)` (`0x008009c0`): a particle part (draw kind
    /// `+0xbb` 3) with a ppp part at `+0xbc` frees its objects
    /// (`pppAllFreePObject`, under the op environment), clears its entry in
    /// `0x012e9c70` (by the ppp part's `u16 +0x1e`) and is reset (`+0xbb`,
    /// `+0xbc` = 0). Returns 0.
    pub fn op_part_free<M: Mem>(
        mem: &mut M,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u8>(part.wrapping_add(0xbb)) != 3
            || mem.read::<u32>(part.wrapping_add(0xbc)) == 0
        {
            return 0;
        }
        op_env(mem);
        let ppp: u32 = mem.read(part.wrapping_add(0xbc));
        op_env(mem);
        crate::part::bridge::ppp_all_free_pobject(mem, ppp, call);
        let i = u32::from(mem.read::<u16>(ppp.wrapping_add(0x1e)));
        mem.write(g(mem, 0x012e_9c70).wrapping_add(4 * i), 0u32);
        mem.write(part.wrapping_add(0xbb), 0u8);
        mem.write(part.wrapping_add(0xbc), 0u32);
        0
    }

    /// `op_part_fp_init(part)` (`0x00800700`): for a loaded ppp part
    /// (`[part +0xd0]`, first byte not 0) the op environment's three tables
    /// (`0x012f2090`..`98`) = its words `+0x14`, `+0x18`, `+0x1c`
    /// (`get_ptr32_func`); unless [`op_part_init_check`] says it is set up,
    /// `pppInitData(ppp, g_pppProgramTable)`. Returns 0.
    pub fn op_part_fp_init<M: Mem>(
        mem: &mut M,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let ppp: u32 = mem.read(part.wrapping_add(0xd0));
        if mem.read::<u8>(ppp) == 0 {
            return 0;
        }
        for k in 0..3 {
            let v = ffx_system::stub::ptr32(mem.read(ppp.wrapping_add(0x14 + 4 * k)));
            mem.write(g(mem, 0x012f_2090).wrapping_add(4 * k), v);
        }
        op_env(mem);
        if op_part_init_check(mem, ppp, call) == 0 {
            let table = g(mem, 0x00c3_a508);
            crate::part::bridge::ppp_init_data(mem, ppp, table);
        }
        0
    }

    /// `0x00807f90` (`opu_blur`, ours): with no flag bits the unit's draw
    /// kind `+0xbb` = 7, `u16 +0xa8` = 0 and the host's unit number added
    /// to the list `host +0x300` (`opu_id_list_add`); with nibble 1
    /// `opus_blur_path3(unit)` (through `call`); both `cmd + 2`. Other
    /// nibbles return `cmd`.
    pub fn opu_blur<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        match nibble(mem, host) {
            0 => {
                mem.write(u.wrapping_add(0xbb), 7u8);
                mem.write(u.wrapping_add(0xa8), 0u16);
                let id: u16 = mem.read(host.wrapping_add(0x214));
                crate::op::bridge::opu_id_list_add(mem, host.wrapping_add(0x300), id);
            }
            0x1000 => {
                call(mem, g(mem, 0x0080_f7e0), &[u]);
            }
            _ => return cmd,
        }
        cmd.wrapping_add(2)
    }

    /// `0x0080c490` (`opu_eff_end`, ours): the unit's `g_EtEff` record (`u16
    /// unit +0x14`) with a battle effect id (`+0` with the top four bits
    /// clear) not yet ended (byte `+0x1d`) is marked (`+0x1d` = 1) and
    /// `MsEffectTerminate(id)` called (through `call`). Then with flag bits
    /// `cmd + 4`; without, `opu_eff_free_by_id(id)` and [`opu_end`].
    pub fn opu_eff_end<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let i = u32::from(mem.read::<u16>(unit(mem, host).wrapping_add(0x14)));
        let e = EFF.ptr(mem).at(i).addr;
        let id: u32 = mem.read(e);
        if id & 0xf000_0000 == 0 && mem.read::<u8>(e + 0x1d) == 0 {
            mem.write(e + 0x1d, 1u8);
            call(mem, g(mem, 0x0078_8920), &[id]);
        }
        if nibble(mem, host) != 0 {
            return cmd.wrapping_add(4);
        }
        crate::et::bridge::opu_eff_free_by_id(mem, id);
        opu_end(mem, host, cmd, call)
    }

    /// `opu_damage_motion(host, cmd)` (`0x00815e10`):
    /// `MsDamageSetMotion(chr, kind, 1)` (through `call`): `kind` 5 with
    /// no flag bits, 6 with nibble 1, else the host's address (as the
    /// original leaves it); `chr` the unit's byte `+0x18` with flag bit
    /// 0x800, else byte `+0x10` of its `g_EtEff` record. Returns `cmd + 2`.
    pub fn opu_damage_motion<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        let i = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
        let e = EFF.ptr(mem).at(i).addr;
        let kind = match flags & 0xf000 {
            0 => 5,
            0x1000 => 6,
            _ => host,
        };
        let chr = if flags & 0x800 == 0 {
            mem.read::<u8>(e + 0x10)
        } else {
            mem.read::<u8>(u.wrapping_add(0x18))
        };
        call(mem, g(mem, 0x0078_ca20), &[u32::from(chr), kind, 1]);
        cmd.wrapping_add(2)
    }

    /// `0x00817ee0` (`opu_length`, ours): with no flag bits, the unit's
    /// float at byte offset `u16 cmd +4` = the distance from its `+0x90` to
    /// unit `u16 cmd +2`'s (`opus_no_get`, `op_length_get`) times
    /// `g_Vu0FixedScale` (16), rounded to `f32`; `cmd + 6`. Otherwise
    /// `cmd`.
    pub fn opu_length(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let u = unit(mem, host);
        if nibble(mem, host) != 0 {
            return cmd;
        }
        let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        let off = u32::from(mem.read::<u16>(cmd.wrapping_add(4)));
        let other = opus_no_get(mem, host.wrapping_add(0x300), u, no);
        let d =
            crate::op::bridge::op_length_get(mem, u.wrapping_add(0x90), other.wrapping_add(0x90));
        let scale: f64 = mem.read(g(mem, 0x00b3_13a0));
        let v = crate::x87::Reg::load_f32(d)
            .constant(crate::x87::MUL, scale)
            .store();
        mem.write(off.wrapping_add(u), v);
        cmd.wrapping_add(6)
    }

    /// `opu_et_ovl(host, cmd)` (`0x00814790`): with nibble 1 the unit's
    /// `u16 +0xa8` = the command's `u16 +2`, `+0xac` = `cmd + i16 cmd +4`
    /// (`set_ptr32`), draw kind `+0xbb` = 0xb and the host's unit number
    /// added to the list `host +0x300`. Returns `cmd + 6`.
    pub fn opu_et_ovl(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let to = i32::from(mem.read::<i16>(cmd.wrapping_add(4)))
            .cast_unsigned()
            .wrapping_add(cmd);
        let u = unit(mem, host);
        if nibble(mem, host) == 0x1000 {
            let v: u16 = mem.read(cmd.wrapping_add(2));
            mem.write(u.wrapping_add(0xa8), v);
            mem.write(u.wrapping_add(0xac), ffx_system::stub::ptr32(to));
            mem.write(u.wrapping_add(0xbb), 0xbu8);
            let id: u16 = mem.read(host.wrapping_add(0x214));
            crate::op::bridge::opu_id_list_add(mem, host.wrapping_add(0x300), id);
        }
        cmd.wrapping_add(6)
    }

    /// `0x0080aca0` (`opu_et_battle`, ours): units of the battle's oef2
    /// (`g_OpBattleOef2`) under id -2: no flag bits `opu_et_unit(oef, i16
    /// cmd +2, i16 cmd +4, -2)` (`cmd + 6`); nibble 1
    /// `op_et_eff_signal_or(-2, 0)` (`cmd + 4`); nibble 2
    /// `op_et_eff_off_signal(-2)` (`cmd + 2`); else `cmd`.
    pub fn opu_et_battle<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id = (-2i32).cast_unsigned();
        match nibble(mem, host) {
            0 => {
                let part = i32::from(mem.read::<i16>(cmd.wrapping_add(4))).cast_unsigned();
                let group = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
                let oef = crate::et::bridge::BATTLE_OEF2.get(mem);
                call(mem, g(mem, 0x007f_d7f0), &[oef, group, part, id]);
                cmd.wrapping_add(6)
            }
            0x1000 => {
                crate::et::bridge::op_et_eff_signal_or(mem, id, 0);
                cmd.wrapping_add(4)
            }
            0x2000 => {
                crate::et::bridge::op_et_eff_off_signal(mem, id);
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `0x00817e60` (`opu_ground_branch`, ours): the unit's character's
    /// ground height (`MsGetChrGround(byte unit +0x18)`, an `f32` through
    /// `fcall`) against its `f32 +0x94` (x87 compares, NaN unordered):
    /// with no flag bits a jump by the command's `i16 +2` unless the
    /// ground is below (or NaN), with nibble 1 unless it is above (or
    /// NaN), else `cmd + 4`. Other nibbles return `cmd`.
    pub fn opu_ground_branch<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        let u = unit(mem, host);
        let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let ground = fcall(mem, g(mem, 0x0079_4fb0), &[chr]) as f32;
        let y: f32 = mem.read(u.wrapping_add(0x94));
        let jump = match nibble(mem, host) {
            0 => ground >= y,
            0x1000 => ground <= y,
            _ => return cmd,
        };
        if jump {
            let d = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
            cmd.wrapping_add(d)
        } else {
            cmd.wrapping_add(4)
        }
    }

    /// `opp_flash(host)` (`0x00812fa0`): `graphicAddScreenFadeTask(r, g,
    /// b, a, f)` from the unit's floats: `r g b` = `+0x40`, `+0x44`,
    /// `+0x48` times 1/128 and `a` = `+0x4c` / 255 (`f32` each), `f` =
    /// `calculateVFXSP_AlphaFunction(i8 +0xa8)`; then
    /// `graphicsRegisterResetScreenFadeDelegation(0x801)` (both through
    /// `call`), whose eax it returns.
    #[allow(clippy::many_single_char_names)] // the colour channels
    pub fn opp_flash<M: Mem>(
        mem: &mut M,
        host: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{MUL, Reg};
        let u = unit(mem, host);
        let mode = i32::from(mem.read::<i8>(u.wrapping_add(0xa8))).cast_unsigned();
        let f = crate::op::bridge::calculate_vfx_sp_alpha_function(mode);
        let rd = |mem: &M, off: u32| mem.read::<f32>(u.wrapping_add(off));
        let div: crate::x87::Op = |a, b| a / b;
        let max: f64 = mem.read(g(mem, 0x00b1_70c0));
        let k: f64 = mem.read(g(mem, 0x00b1_70b8));
        let a = Reg::load_f32(rd(mem, 0x4c)).constant(div, max).store();
        let b = Reg::load_f32(rd(mem, 0x48)).constant(MUL, k).store();
        let gr = Reg::load_f32(rd(mem, 0x44)).constant(MUL, k).store();
        let r = Reg::Num(k).mem(MUL, rd(mem, 0x40)).store();
        let args = [r, gr, b, a].map(f32::to_bits);
        call(
            mem,
            g(mem, 0x0063_9240),
            &[args[0], args[1], args[2], args[3], f],
        );
        call(mem, g(mem, 0x0064_5e60), &[0x801])
    }

    /// `opu_damage(host, cmd)` (`0x00815d90`): for the unit's `g_EtEff`
    /// record `e` (`u16 unit +0x14`): no flag bits `MsSetDamage([e], byte
    /// e +0x10, b)`, nibble 1 `MsSetDamage([e], byte unit +0x18, b)` (`b` =
    /// 2 with flag bit 0x800, else 0), nibble 2 `MsMergeDamage([e], byte
    /// unit +0x18)` (through `call`), each `cmd + 2`; other nibbles `cmd`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_damage<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        let i = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
        let e = EFF.ptr(mem).at(i).addr;
        let b = u32::from(flags & 0x800) >> 10;
        let chr = |mem: &M| u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        match flags & 0xf000 {
            0 => {
                let c = u32::from(mem.read::<u8>(e + 0x10));
                let id: u32 = mem.read(e);
                call(mem, g(mem, 0x0078_d980), &[id, c, b]);
            }
            0x1000 => {
                let c = chr(mem);
                let id: u32 = mem.read(e);
                call(mem, g(mem, 0x0078_d980), &[id, c, b]);
            }
            0x2000 => {
                let c = chr(mem);
                let id: u32 = mem.read(e);
                call(mem, g(mem, 0x0079_aff0), &[id, c]);
            }
            _ => return cmd,
        }
        cmd.wrapping_add(2)
    }

    /// `opu_motion_etc(host, cmd)` (`0x008047b0`): for the unit's character
    /// (`byte +0x18`): no flag bits `Ch_SetMotionSpeed(g_EtChrHandles[c],
    /// u16 cmd +2)` (`cmd + 4`); nibble 1 `MsEffectEndMotion(c, 0)` (`cmd +
    /// 2`); nibble 2 `MsEffectEndMotion(c, [0x00c48e28 + 4 u16 cmd +2])`
    /// (`cmd + 4`); all through `call`. Other nibbles `cmd`.
    pub fn opu_motion_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let c = u32::from(mem.read::<u8>(unit(mem, host).wrapping_add(0x18)));
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(c).get(mem);
        let n = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        match nibble(mem, host) {
            0 => {
                call(mem, g(mem, 0x0082_b380), &[h, n]);
                cmd.wrapping_add(4)
            }
            0x1000 => {
                call(mem, g(mem, 0x0078_7950), &[c, 0]);
                cmd.wrapping_add(2)
            }
            0x2000 => {
                let kind: u32 = mem.read(g(mem, 0x00c4_8e28).wrapping_add(4 * n));
                call(mem, g(mem, 0x0078_7950), &[c, kind]);
                cmd.wrapping_add(4)
            }
            _ => cmd,
        }
    }

    /// `opu_motion(host, cmd)` (`0x008044e0`): the motion of the unit's
    /// character `c` (byte `+0x18`, `h` = `g_EtChrHandles[c]`), by the
    /// host's flag nibble. 0 and 1: `m` = `u16 cmd +4` / `+2`; in battle
    /// (`MsBattleChrStart()`) `MsEffectSetMotion(c, m)` for `c` < 0x20,
    /// else when `h` is set `Ch_ResetMotion(h)`, `Ch_SetSysMotion(h, 1,
    /// m)`, `Ch_GetSeqFrame(h)` (`cmd + 6` / `+ 4`). 3: the same with `m`
    /// = `u16 cmd +2`, storing the result (or `m` without `h`, 0 outside
    /// battle) as `f32` at unit `+ i16 cmd +4` (`cmd + 6`). 2: waits while
    /// the motion runs: `MsEffectCheckMotion2(c)` when host byte `+0x21e`
    /// is set and `c` < 0x20, else `Ch_GetWaitMotion(h)` != 0 sets host
    /// byte `+0x210` = 1 and returns `cmd`; done is `cmd + 2`. 4: outside
    /// battle with `h`, `Ch_ResetMotion`, `Ch_SetSequence(h, u32 cmd +2)`,
    /// `Ch_GetSeqFrame` (`cmd + 6`). 5: `Ch_MotionSetNextHokan(h, 0)`,
    /// `Ch_ClothReset2(h)` (`cmd + 2`). Others `cmd`. Through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_motion<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let c = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(c).get(mem);
        let half = |mem: &M, k: u32| u32::from(mem.read::<u16>(cmd.wrapping_add(k)));
        match nibble(mem, host) {
            n @ (0 | 0x1000) => {
                let (m, next) = if n == 0 {
                    (half(mem, 4), 6)
                } else {
                    (half(mem, 2), 4)
                };
                if go(mem, 0x0078_0cb0, &[]) != 0 {
                    set_motion(mem, &mut go, c, h, m);
                }
                cmd.wrapping_add(next)
            }
            0x3000 => {
                let to = i32::from(mem.read::<i16>(cmd.wrapping_add(4))).cast_unsigned();
                let m = half(mem, 2);
                let v = if go(mem, 0x0078_0cb0, &[]) != 0 {
                    set_motion(mem, &mut go, c, h, m).map_or(m.cast_signed(), u32::cast_signed)
                } else {
                    0
                };
                #[allow(clippy::cast_precision_loss)] // fild, one rounding at the fstp
                mem.write(to.wrapping_add(u), v as f32);
                cmd.wrapping_add(6)
            }
            0x2000 => {
                let busy = if mem.read::<u8>(host.wrapping_add(0x21e)) != 0 && c < 0x20 {
                    go(mem, 0x0078_78a0, &[c]) == 0
                } else {
                    go(mem, 0x0083_92c0, &[h]) != 0
                };
                if !busy {
                    return cmd.wrapping_add(2);
                }
                mem.write(host.wrapping_add(0x210), 1u8);
                cmd
            }
            0x4000 => {
                let seq = half(mem, 4).wrapping_shl(16).wrapping_add(half(mem, 2));
                if go(mem, 0x0078_0cb0, &[]) == 0 && h != 0 {
                    go(mem, 0x0082_b080, &[h]);
                    go(mem, 0x0083_7ac0, &[h, seq]);
                    go(mem, 0x0083_6af0, &[h]);
                }
                cmd.wrapping_add(6)
            }
            0x5000 => {
                go(mem, 0x0082_b010, &[h, 0]);
                go(mem, 0x0082_4670, &[h]);
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `0x0080e030` (`opp_mdc`, ours; opp table entry 13, `0x00c48eb4`),
    /// `(host, set)` -> the draw's result, also at `set +0x54`: the unit's
    /// (`host +0x220`) `+0x30..+0x40` to the emulated EE register at
    /// `0x00c8f528` (`g_Ee_Gpr +0x20`) and host `+0..+0x10`, host `+0` then
    /// 0.0; `opus_rt_m_chara(host +0x80,
    /// byte unit +0x18, f32 unit +0x3c, host, unit +0x90, i8 host +0x21e)`,
    /// host `+0xc0` = `opus_parent_get(host, unit)` x `host +0x80`, then
    /// `host +0x2c0` x it (`op_mul_m`); the mdc packet
    /// (`opus_mdc_packet_set(host +0x100, host, set, unit, i16 *[unit
    /// +0xa8])`), host `+0x100` |= 0x40, its lights
    /// (`opus_mdc_light_set(.., [unit +0xa8])`) and `op1_mdc_draw(packet,
    /// host +0x100, host +0x80, host +0x280, host +0x2c0)`. Callees through
    /// `call`.
    pub fn opp_mdc<M: Mem>(
        mem: &mut M,
        host: u32,
        set: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mdc_draw(mem, host, set, false, call)
    }

    /// `0x0080df10` (`opp_mdc_wave_0080df10`, ours; opp table entry 12,
    /// `0x00c48eb0`), `(host, set)`: [`opp_mdc`] with the wave lights
    /// (`opus_wave_light_setup_0080e2f0(host +0x100, host, set, unit,
    /// [unit +0xa8])`) in place of `opus_mdc_light_set`, and host `+0x100`
    /// left as the packet set-up leaves it. Callees through `call`.
    pub fn opp_mdc_wave<M: Mem>(
        mem: &mut M,
        host: u32,
        set: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mdc_draw(mem, host, set, true, call)
    }

    /// The body of [`opp_mdc`] (`wave` false) and [`opp_mdc_wave`].
    #[allow(clippy::many_single_char_names)]
    fn mdc_draw<M: Mem>(
        mem: &mut M,
        host: u32,
        set: u32,
        wave: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{load, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let h = |k: u32| host.wrapping_add(k);
        let shape: u32 = mem.read(u.wrapping_add(0xa8));
        let v: [u32; 4] = mem.read(u.wrapping_add(0x30));
        mem.write(g(mem, 0x00c8_f528), v);
        mem.write(host, v);
        mem.write(host, 0u32);
        let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let k = store(load(mem.read(u.wrapping_add(0x3c))));
        let battle = i32::from(mem.read::<i8>(h(0x21e))).cast_unsigned();
        go(
            mem,
            0x0080_62c0,
            &[h(0x80), chr, k, host, u.wrapping_add(0x90), battle],
        );
        let parent = go(mem, 0x0080_1f00, &[host, u]);
        go(mem, 0x007e_83b0, &[h(0xc0), parent, h(0x80)]);
        go(mem, 0x007e_83b0, &[h(0xc0), h(0x2c0), h(0xc0)]);
        let first = i32::from(mem.read::<i16>(shape)).cast_unsigned();
        let packet = go(mem, 0x0080_e160, &[h(0x100), host, set, u, first]);
        if wave {
            go(mem, 0x0080_e2f0, &[h(0x100), host, set, u, shape]);
        } else {
            let w: u32 = mem.read(h(0x100));
            mem.write(h(0x100), w | 0x40);
            go(mem, 0x0080_e900, &[h(0x100), host, set, u, shape]);
        }
        let r = go(
            mem,
            0x00a6_8c90,
            &[packet, h(0x100), h(0x80), h(0x280), h(0x2c0)],
        );
        mem.write(set.wrapping_add(0x54), r);
        r
    }

    /// `0x00814800` (`opu_dobj_draw_00814800`, ours; opus table entry 0x54,
    /// `0x00c49018`), `(host, cmd)`: by the flag nibble, 0:
    /// `yiDObjToOMD(0x6800000, 0)` (plain numbers, not image addresses),
    /// `cmd + 2`; 0x1000: host `+0x80` = the unit scale `f32 unit +0x3c /
    /// 1024` (`op_unit_m_scale`; the original keeps that `f32` in its
    /// `host` argument slot, which its only caller, the opus command loop,
    /// pushes afresh), turned by unit `+0x30` and moved by `+0x90`
    /// (`op_rt_m`), times `opus_parent_get(host, unit)` (`op_mul_m`); host
    /// `+0xc0` = host `+0x2c0` x it, and `op_md_x64_draw_00a69300(0x6800000,
    /// host +0xc0, 0x1f00000)`, `cmd + 2`; else `cmd`. Callees through
    /// `call`.
    pub fn opu_dobj_draw<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{load, mul, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        match nibble(mem, host) {
            0 => {
                go(mem, 0x0090_8420, &[0x0680_0000, 0]);
                cmd.wrapping_add(2)
            }
            0x1000 => {
                let m = host.wrapping_add(0x80);
                // `fmul qword` 1/1024, `fstp dword` into the slot, reloaded
                let s = store(mul(load(mem.read(u.wrapping_add(0x3c))), 0.000_976_562_5));
                go(mem, 0x007e_a700, &[m, s]);
                go(
                    mem,
                    0x007e_9850,
                    &[m, m, u.wrapping_add(0x30), u.wrapping_add(0x90)],
                );
                let parent = go(mem, 0x0080_1f00, &[host, u]);
                go(mem, 0x007e_83b0, &[m, parent, m]);
                let out = host.wrapping_add(0xc0);
                go(mem, 0x007e_83b0, &[out, host.wrapping_add(0x2c0), m]);
                go(mem, 0x00a6_9300, &[0x0680_0000, out, 0x01f0_0000]);
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `opu_world(host, cmd)` (`0x00801d00`; opus table): the host matrix
    /// scaled, by flag 0x400 the `i16`s at `cmd +2..+6` / 256
    /// (`op_unit_m_scale2`, `cmd + 8`; the scale also in host `+0..+8`),
    /// else 1.0 (`op_unit_m_scale`, `cmd + 2`); unit byte `+0x19` = the
    /// rotation order `(flags >> 12) & 7`, then `op_rt_m_6(host, host,
    /// unit +0x30, unit +0x90, order)`. Unit `+0xc0` by flags 0x800/0x200
    /// (reread): none: `host +0x280` x host (`op_mul_m`); 0x200: host
    /// `+0x30` through `host +0x280` (`op_apply_m_w1`), host copied;
    /// 0x800: host copied; both: `op_mul_m33(unit +0xc0, host +0x280,
    /// host)` and host `+0x30..+0x3c` into the emulated `v0` (`0x00c8f528`)
    /// and unit `+0xf0`. Returns the next command. Callees through `call`.
    pub fn opu_world<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{mul, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let next = if mem.read::<u16>(host.wrapping_add(0x21a)) & 0x400 != 0 {
            for k in 0..3u32 {
                let n = mem.read::<i16>(cmd.wrapping_add(2 + 2 * k));
                // `fild`, times the `f64` 1/256 (exact), `fstp dword`
                let x = store(mul(f64::from(n), 0.003_906_25));
                mem.write(host.wrapping_add(4 * k), x);
            }
            go(mem, 0x007e_a730, &[host, host]);
            cmd.wrapping_add(8)
        } else {
            go(mem, 0x007e_a700, &[host, 0x3f80_0000]);
            cmd.wrapping_add(2)
        };
        let order = u32::from(mem.read::<u16>(host.wrapping_add(0x21a)) >> 12) & 7;
        #[allow(clippy::cast_possible_truncation)] // 0..8
        mem.write(u.wrapping_add(0x19), order as u8);
        go(
            mem,
            0x007e_98b0,
            &[
                host,
                host,
                u.wrapping_add(0x30),
                u.wrapping_add(0x90),
                order,
            ],
        );
        let flags = mem.read::<u16>(host.wrapping_add(0x21a));
        let (out, view) = (u.wrapping_add(0xc0), host.wrapping_add(0x280));
        match (flags & 0x800 != 0, flags & 0x200 != 0) {
            (false, false) => {
                go(mem, 0x007e_83b0, &[out, view, host]);
            }
            (false, true) => {
                let t = host.wrapping_add(0x30);
                go(mem, 0x007e_79a0, &[t, view, t]);
                go(mem, 0x007e_8010, &[out, host]);
            }
            (true, false) => {
                go(mem, 0x007e_8010, &[out, host]);
            }
            (true, true) => {
                go(mem, 0x007e_8b00, &[out, view, host]);
                let v: [u32; 4] = mem.read(host.wrapping_add(0x30));
                mem.write(g(mem, 0x00c8_f528), v);
                mem.write(u.wrapping_add(0xf0), v);
            }
        }
        next
    }

    /// `opu_en_on(host, cmd)` (`0x008161f0`; opus table): character
    /// visibility by the flag nibble, `c` = byte unit `+0x18`, `h` =
    /// `g_EtChrHandles[c]`, `e` = the `g_EtEff` record `u16 unit +0x14`,
    /// `b` = flag bit 12. 0, 1, 0xc, 0xd: without byte host `+0x21e` or for
    /// `c` >= 0x20 `Ch_SetHideEffect(h, b)`; else `v = !b`:
    /// `MsSetChrVisible(c, v)` unless `GetU32_00c64ca4()` is 0x290, for
    /// 0xc/0xd `MsSetChrVisibleAppear(c, v)` unless magic 0x77, and in
    /// battle scene 0x1050032 under magic 0x94 or 0x253 with nibble 0 and
    /// `Ch_GetId(h)` 2, `g_MagicHiddenBtlChr = c` and `MsSetChrVisible(c,
    /// 0)`. 2/3: `Ch_SetHideEffect(g_EtChrHandles[byte e +0x18], 0/1)`;
    /// 4/5: with `+0x21e` `MsSetAllVisible(!b)`, else each of the first 32
    /// handles `Ch_SetHideEffect(h, b)`; 6/7: `Ch_DrawEnEffect(h, 0/1)`;
    /// 8: `m` = `i16 cmd +2`, `o` = byte `cmd +4`: below 2
    /// `Ch_SetHideEvent(h, m)` (its result as an `f32` to unit `+o` for a
    /// nonzero `o`), else `Ch_SetHideEvent(h, ftol(f32 unit +o))`, `cmd +
    /// 6`; 9/0xa: `MsSetChrVisible(c, 1/0)`; 0xb: host `+0..+0x20` = the
    /// emulated `r0` twice, host byte `[byte e +0x10 + i]` = 1 for `i` below
    /// byte `e +8`, then `MsSetChrVisible(i, 0)` for each of 32 characters
    /// with a handle, no mark and not `c`. `cmd + 2`, `cmd` for 0xe/0xf.
    /// Callees through `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_en_on<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::et::bridge::CHR_HANDLES;
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let flags = mem.read::<u16>(host.wrapping_add(0x21a));
        let c = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let handle = |mem: &M, i: u32| CHR_HANDLES.ptr(mem).at(i).get(mem);
        let h = handle(mem, c);
        let e = EFF
            .ptr(mem)
            .at(u32::from(mem.read::<u16>(u.wrapping_add(0x14))))
            .addr;
        let b = u32::from(flags >> 12) & 1;
        let no = |mem: &M| u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let linked = mem.read::<u8>(host.wrapping_add(0x21e)) != 0;
        match flags & 0xf000 {
            0 | 0x1000 | 0xc000 | 0xd000 => {
                if !linked || c >= 0x20 {
                    go(mem, 0x0082_b2a0, &[h, b]);
                    return cmd.wrapping_add(2);
                }
                let v = u32::from(b == 0);
                if go(mem, 0x009d_a3a0, &[]) != 0x290 {
                    let n = no(mem);
                    go(mem, 0x0079_6660, &[n, v]);
                }
                let kind = mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000;
                if matches!(kind, 0xc000 | 0xd000) && go(mem, 0x009d_a3b0, &[]) != 0x77 {
                    let n = no(mem);
                    go(mem, 0x0079_66a0, &[n, v]);
                }
                if go(mem, 0x0078_2970, &[]) == 0x0105_0032
                    && (go(mem, 0x009d_a3b0, &[]) == 0x94 || go(mem, 0x009d_a3b0, &[]) == 0x253)
                    && mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000 == 0
                    && go(mem, 0x0082_6b80, &[h]) == 2
                {
                    let n = no(mem);
                    ffx_system::menu::bridge::MAGIC_HIDDEN_BTL_CHR.set(mem, n);
                    let n = no(mem);
                    go(mem, 0x0079_6660, &[n, 0]);
                }
            }
            k @ (0x2000 | 0x3000) => {
                let t = u32::from(mem.read::<u8>(e.wrapping_add(0x18)));
                let t = handle(mem, t);
                go(mem, 0x0082_b2a0, &[t, u32::from(k == 0x3000)]);
            }
            0x4000 | 0x5000 => {
                if linked {
                    go(mem, 0x0079_6680, &[u32::from(b == 0)]);
                } else {
                    for i in 0..32 {
                        let t = handle(mem, i);
                        if t != 0 {
                            go(mem, 0x0082_b2a0, &[t, b]);
                        }
                    }
                }
            }
            k @ (0x6000 | 0x7000) => {
                go(mem, 0x0082_eb60, &[h, u32::from(k == 0x7000)]);
            }
            0x8000 => {
                let m = mem.read::<i16>(cmd.wrapping_add(2));
                let o = u32::from(mem.read::<u8>(cmd.wrapping_add(4)));
                let at = o.wrapping_add(u);
                if m < 2 {
                    let r = go(mem, 0x0082_b2c0, &[h, i32::from(m).cast_unsigned()]);
                    if o != 0 {
                        #[allow(clippy::cast_precision_loss)] // fild, fstp m32
                        let x = r.cast_signed() as f32;
                        mem.write(at, x.to_bits());
                    }
                } else {
                    let x = ffx_bridge::x87::load(mem.read(at));
                    let v = ffx_bridge::x87::ftol(mem, x);
                    go(mem, 0x0082_b2c0, &[h, v]);
                }
                return cmd.wrapping_add(6);
            }
            k @ (0x9000 | 0xa000) => {
                go(mem, 0x0079_6660, &[c, u32::from(k == 0x9000)]);
            }
            0xb000 => {
                let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
                mem.write(host, r0);
                mem.write(host.wrapping_add(0x10), r0);
                let mut i = 0u32;
                while i < u32::from(mem.read::<u8>(e.wrapping_add(8))) {
                    let k = u32::from(mem.read::<u8>(e.wrapping_add(0x10 + i)));
                    mem.write(host.wrapping_add(k), 1u8);
                    i += 1;
                }
                for i in 0..32 {
                    if handle(mem, i) != 0
                        && mem.read::<u8>(host.wrapping_add(i)) == 0
                        && no(mem) != i
                    {
                        go(mem, 0x0079_6660, &[i, 0]);
                    }
                }
            }
            _ => return cmd,
        }
        cmd.wrapping_add(2)
    }

    /// `opp_anm(host, set)` (`0x00811120`): an animated shape for the
    /// unit `u` = `[host +0x220]`. Host `+4` = `u +0xc8`; the scale `s` =
    /// `i16 u +0xf2`, by mode byte `u +0xa8` then `u +0xa9`: 1 `s` = `f32 u
    /// +0x3c` / `+0x4c`, 2 host byte `+7` = that float clamped to 0..255
    /// (above 255 the `f32` at `0x00b480d4`, below 0 zero, NaN kept) and
    /// truncated (`fistp` under a round-toward-zero control word, NaN's
    /// indefinite giving 0). `u16 host +2` = `u16 u +0xc0`, `host +8` = `u
    /// +0xd0`, `host +0x80` = `op_unit_m_scale(s / 4096)`, turned by
    /// `op_rot_m(u +0x30)` when byte `u +0xaa`, else by `op_rot_m_z(i16 u
    /// +0xf4)` when that is non-zero; `host +0x40` = `u +0x90` (through
    /// `g_Ee_Gpr +0x20`), `host +0xb0` = it through `opus_parent_get`
    /// (`op_apply_m_w1`), plus `u +0x30.. / 16` with `u16 u +0xc0` bit
    /// 0x100. `host +0x10` = `host +0x80`, `+0x14` = `[set +0x54]`, `+0x1c`
    /// = 0 or (with `i16 u +0xc6`) `host +0x100`, a copy of `+0x80` with
    /// `+0x138` raised by `i16 u +0xc6 / 16`; `u16 host +0xc` = `u16 u
    /// +0xc4`. The frame timer `u16 u +0xee` -= `u16 u +0xf0`; at 0 the
    /// frame `u16 u +0xec` steps through the animation `d` = `[u +0xe0]`
    /// (`i16 d +4` frames, 8-byte entries from `d +0x10`): past the end
    /// it loops when the last entry's byte `+4` has bit 7 or `u +0xc0` bit
    /// 0x8000, else `opus_draw_off(set, u16 host +0x214)`, `u16 host
    /// +0x218` - 1, `u16 u +0xb0` |= 0x8000 and done (`eax` 0x8000);
    /// `u +0xe4` = the entry, `u +0xe8` = `d` + its `i16 +0`, `u16 u +0xee`
    /// = its `+2`. Unless the unit's draw bits (`u8 +0xb3`) meet
    /// `g_EtChrDrawFlags[byte u +0x18]` (`eax` that byte): `[slot]` =
    /// `set_ptr32(d)` (`slot` the original's first argument slot) and the
    /// render data of instance `(0, u)`: none, `InitVFXRenderDataShp(0, u,
    /// slot, host, frame, calculateVFXSP_AlphaFunction(byte e +4 + [e
    /// +4]))` (`e` = `[u +0xe8]`) between `setVfxShapeEnlargeBufferFactor(8)`
    /// and its reset (whose `eax` it returns); with `i16 e +2` above 0
    /// `updateVFXRenderDataShp(0, u, slot, host, frame)` and
    /// `graphicVFXUpdate(0, u)` (its `eax`); else `e`. Callees through
    /// `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opp_anm<M: Mem>(
        mem: &mut M,
        host: u32,
        set: u32,
        slot: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, cvttsd2si, load, mul, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let h = |k: u32| host.wrapping_add(k);
        let ua = |k: u32| u.wrapping_add(k);
        let w: u32 = mem.read(ua(0xc8));
        mem.write(h(4), w);
        // `fild`, stored as f32 (exact for an i16)
        let short = |mem: &M, at: u32| store(f64::from(mem.read::<i16>(at)));
        let mut scale = short(mem, ua(0xf2));
        let angle = short(mem, ua(0xf4));
        let top: u32 = mem.read(g(mem, 0x00b4_80d4));
        let k255: f64 = mem.read(g(mem, 0x00b1_70c0));
        for (mode_at, value_at) in [(0xa8, 0x3c), (0xa9, 0x4c)] {
            match mem.read::<u8>(ua(mode_at)) {
                1 => scale = store(load(mem.read(ua(value_at)))),
                2 => {
                    let v = load(store(load(mem.read(ua(value_at)))));
                    // above 255 (ordered) the constant, below 0 zero; a NaN
                    // passes both compares
                    let c = if v > k255 {
                        load(store(load(top)))
                    } else if v < 0.0 {
                        0.0
                    } else {
                        v
                    };
                    mem.write(h(7), cvttsd2si(c).to_le_bytes()[0]);
                }
                _ => {}
            }
        }
        let tag: u16 = mem.read(ua(0xc0));
        mem.write(h(2), tag);
        let d0: u32 = mem.read(ua(0xd0));
        mem.write(h(8), d0);
        let k4096: f64 = mem.read(g(mem, 0x00b5_0190));
        let s = store(mul(load(scale), k4096));
        let m = h(0x80);
        go(mem, 0x007e_a700, &[m, s]);
        if mem.read::<u8>(ua(0xaa)) != 0 {
            go(mem, 0x007e_9530, &[m, m, ua(0x30)]);
        } else if load(angle) != 0.0 {
            go(mem, 0x007e_9760, &[m, m, angle]);
        }
        let pos: [u32; 4] = mem.read(ua(0x90));
        mem.write(g(mem, 0x00c8_f528), pos);
        mem.write(h(0x40), pos);
        let parent = go(mem, 0x0080_1f00, &[host, u]);
        go(mem, 0x007e_79a0, &[h(0xb0), parent, h(0x40)]);
        let k16: f64 = mem.read(g(mem, 0x00b9_21e0));
        if mem.read::<u16>(ua(0xc0)) & 0x100 != 0 {
            for i in 0..3 {
                let at = h(0xb0 + 4 * i);
                let lift = mul(load(mem.read(ua(0x30 + 4 * i))), k16);
                let v = store(add(lift, load(mem.read(at))));
                mem.write(at, v);
            }
        }
        mem.write(h(0x10), m);
        let q: u32 = mem.read(set.wrapping_add(0x54));
        mem.write(h(0x14), q);
        if mem.read::<i16>(ua(0xc6)) == 0 {
            mem.write(h(0x1c), 0u32);
        } else {
            mem.write(h(0x1c), h(0x100));
            go(mem, 0x007e_8010, &[h(0x100), m]);
            let lift = mul(load(short(mem, ua(0xc6))), k16);
            let v = store(add(lift, load(mem.read(h(0x138)))));
            mem.write(h(0x138), v);
        }
        let c4: u16 = mem.read(ua(0xc4));
        mem.write(h(0xc), c4);
        let timer = mem
            .read::<u16>(ua(0xee))
            .wrapping_sub(mem.read::<u16>(ua(0xf0)));
        mem.write(ua(0xee), timer);
        let d: u32 = mem.read(ua(0xe0));
        if timer == 0 {
            let frame = mem.read::<u16>(ua(0xec)).wrapping_add(1);
            mem.write(ua(0xec), frame);
            if i32::from(frame) >= i32::from(mem.read::<i16>(d.wrapping_add(4))) {
                let last = u32::from(frame.wrapping_sub(1));
                let looped = mem.read::<u8>(d.wrapping_add(0x14 + last * 8)) & 0x80 != 0;
                if !looped && mem.read::<u16>(ua(0xc0)) & 0x8000 == 0 {
                    let id = u32::from(mem.read::<u16>(h(0x214)));
                    go(mem, 0x0080_0d10, &[set, id]);
                    let n: u16 = mem.read(h(0x218));
                    mem.write(h(0x218), n.wrapping_sub(1));
                    let b0: u16 = mem.read(ua(0xb0));
                    mem.write(ua(0xb0), b0 | 0x8000);
                    return 0x8000;
                }
                mem.write(ua(0xec), 0u16);
            }
            let entry = d.wrapping_add((u32::from(mem.read::<u16>(ua(0xec))) + 2) * 8);
            mem.write(ua(0xe4), entry);
            let start = i32::from(mem.read::<i16>(entry))
                .cast_unsigned()
                .wrapping_add(d);
            mem.write(ua(0xe8), start);
            let t: u16 = mem.read(entry.wrapping_add(2));
            mem.write(ua(0xee), t);
        }
        let chr = u32::from(mem.read::<u8>(ua(0x18)));
        let flags: u8 = mem.read(g(mem, 0x012a_84c0).wrapping_add(chr));
        if mem.read::<u8>(ua(0xb3)) & flags != 0 {
            return u32::from(flags);
        }
        let p = go(mem, 0x00a4_4700, &[d]);
        mem.write(slot, p);
        let rd = go(mem, 0x0064_53d0, &[0, u]);
        let e: u32 = mem.read(ua(0xe8));
        if rd == 0 {
            let at: u32 = mem.read(e.wrapping_add(4));
            let kind = u32::from(mem.read::<u8>(e.wrapping_add(at).wrapping_add(4)));
            let alpha = go(mem, 0x0071_2c30, &[kind]);
            go(mem, 0x0071_7610, &[8]);
            let n = u32::from(mem.read::<u16>(ua(0xec)));
            go(mem, 0x0071_1460, &[0, u, slot, host, n, alpha]);
            return go(mem, 0x0071_7600, &[]);
        }
        if mem.read::<i16>(e.wrapping_add(2)) > 0 {
            let n = u32::from(mem.read::<u16>(ua(0xec)));
            go(mem, 0x0071_b8a0, &[0, u, slot, host, n]);
            return go(mem, 0x0064_58a0, &[0, u]);
        }
        e
    }

    /// `opp_dtt(host, set)` (`0x00812b50`): a distortion sprite for the
    /// unit `u` = `[host +0x220]`, unless the unit's draw bits (`u8 +0xb3`)
    /// meet `g_EtChrDrawFlags[byte u +0x18]` (then `eax` is that flags
    /// byte). The sprite record is `host +0x100`: first `g_Ee_Gpr`'s xyz
    /// with `w` 1; the colour `u +0xc8` (offset by `g_pppColorConShift`,
    /// then cleared) through the EE's `pextlb`/`pextlh` with `r0` into
    /// `g_Ee_Gpr +0x20`, its four words as `f32` into `g_Vu0_VfMove` and
    /// `+0x110`; `host +0x80` = `op_unit_m_scale(1/16)`, `host +0x40` = `u
    /// +0x90` (through `g_Ee_Gpr +0x20`), `host +0xb0` = it through
    /// `opus_parent_get` (`op_apply_m_w1`), copied to `+0x100` (w 1);
    /// host = `op_rot_m(unit, u +0x30)`, `host +0x40` = `g_Ee_Gpr`'s with
    /// `z` = `u +0x4c / 16`, put through host, `host +0xb0` = `g_Ee_Gpr`'s,
    /// host = `host +0x80` times `host +0x40`; `+0x120`/`+0x128`
    /// `set_ptr32(host +0x2c0)`, `+0x130..` host's xyz (w 0), `+0x140` the
    /// 64-bit word `0x2_2402_0d00` (`0x2_2402_0000` when `0x011a0730 &
    /// 0x1ff`), `+0x148`/`+0x150` the words at `0x011a0760`/`0x011a0750`,
    /// `+0x124` `set_ptr32(q + 0x10)` with `q` = `[set +0x54]`. Then
    /// `KeDttSpr_MkGifPk_Pre`, `KeDttSpr_MkGifPk`; when that is non-zero
    /// `[set +0x54]` = `get_ptr32_func([host +0x124])` = `n` and
    /// `op_ot_add(q, (n - q) >> 4, i32 [host +0x12c] / 0x3000, 0)`, whose
    /// `eax` is returned (else 0). Callees through `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opp_dtt<M: Mem>(
        mem: &mut M,
        host: u32,
        set: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{load, mul, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let h = |k: u32| host.wrapping_add(k);
        let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let flags: u8 = mem.read(g(mem, 0x012a_84c0).wrapping_add(chr));
        if mem.read::<u8>(u.wrapping_add(0xb3)) & flags != 0 {
            return u32::from(flags);
        }
        let gpr = g(mem, 0x00c8_f508);
        let r: [u32; 3] = mem.read(gpr);
        mem.write(h(0x100), r);
        mem.write(h(0x10c), 1.0f32);
        let shift = g(mem, 0x00c8_b500);
        let off: u32 = mem.read(shift);
        let colour: u32 = mem.read(off.wrapping_add(u).wrapping_add(0xc8));
        let r0: [u8; 8] = mem.read(gpr);
        let words = crate::hi_draw::bridge::expand_colour(colour, r0);
        let wide = g(mem, 0x00c8_f528);
        mem.write(wide, words);
        mem.write(shift, 0u32);
        // `fild` of each word, stored as f32
        let floats = words.map(|w| store(f64::from(w.cast_signed())));
        let vf_move = g(mem, 0x00c8_f980);
        mem.write(vf_move, floats);
        mem.write(h(0x110), floats);
        go(mem, 0x007e_a700, &[h(0x80), 0.0625f32.to_bits()]);
        let pos: [u32; 4] = mem.read(u.wrapping_add(0x90));
        mem.write(wide, pos);
        mem.write(h(0x40), pos);
        let parent = go(mem, 0x0080_1f00, &[host, u]);
        go(mem, 0x007e_79a0, &[h(0xb0), parent, h(0x40)]);
        let at: [u32; 3] = mem.read(h(0xb0));
        mem.write(h(0x100), at);
        mem.write(h(0x10c), 1.0f32);
        go(mem, 0x007e_a6d0, &[host]);
        go(mem, 0x007e_9530, &[host, host, u.wrapping_add(0x30)]);
        let r: [u32; 4] = mem.read(gpr);
        mem.write(h(0x40), r);
        let k: f64 = mem.read(g(mem, 0x00b9_21e0));
        let z = store(mul(load(mem.read(u.wrapping_add(0x4c))), k));
        mem.write(h(0x48), z);
        go(mem, 0x007e_79a0, &[h(0x40), host, h(0x40)]);
        let r: [u32; 4] = mem.read(gpr);
        mem.write(h(0xb0), r);
        go(mem, 0x007e_79a0, &[host, h(0x80), h(0x40)]);
        let p = go(mem, 0x00a4_4700, &[h(0x2c0)]);
        mem.write(h(0x120), p);
        let p = go(mem, 0x00a4_4700, &[h(0x2c0)]);
        mem.write(h(0x128), p);
        // `fld; fstp` (a signalling NaN comes out quiet)
        let copy = |mem: &M, at: u32| store(load(mem.read(at)));
        let (x, y) = (copy(mem, host), copy(mem, h(4)));
        mem.write(h(0x130), x);
        mem.write(h(0x134), y);
        mem.write(h(0x13c), 0u32);
        let z = copy(mem, h(8));
        mem.write(h(0x138), z);
        let low = if mem.read::<u32>(g(mem, 0x011a_0730)) & 0x1ff != 0 {
            0
        } else {
            0xd00
        };
        mem.write(h(0x140), 0x2_2402_0000u64 | low);
        let a: [u32; 2] = mem.read(g(mem, 0x011a_0760));
        mem.write(h(0x148), a);
        let b: [u32; 2] = mem.read(g(mem, 0x011a_0750));
        mem.write(h(0x150), b);
        let q: u32 = mem.read(set.wrapping_add(0x54));
        let p = go(mem, 0x00a4_4700, &[q.wrapping_add(0x10)]);
        mem.write(h(0x124), p);
        go(mem, 0x0076_8d30, &[h(0x100)]);
        if go(mem, 0x0076_7790, &[h(0x100)]) == 0 {
            return 0;
        }
        let w: u32 = mem.read(h(0x124));
        let n = go(mem, 0x00a4_4650, &[w]);
        mem.write(set.wrapping_add(0x54), n);
        let depth = mem.read::<u32>(h(0x12c)).cast_signed() / 0x3000;
        let count = n.wrapping_sub(q).cast_signed() >> 4;
        go(
            mem,
            0x007e_c420,
            &[q, count.cast_unsigned(), depth.cast_unsigned(), 0],
        )
    }

    /// A unit's colour `+0xc8` as the EE leaves it after `pextlb` then
    /// `pextlh` with `r0` (`g_Ee_Gpr`): 16 bytes from the colour's first
    /// word and `r0`'s low 8 bytes.
    fn gline_colour(mem: &mut impl Mem, u: u32) {
        let r0: [u8; 8] = mem.read(g(mem, 0x00c8_f508));
        let c: u32 = mem.read(u.wrapping_add(0xc8));
        let v = crate::hi_draw::bridge::expand_colour(c, r0);
        mem.write(u.wrapping_add(0xc8), v);
    }

    /// `opp_gline_00813020(host, set)` (`0x00813020`; opp table entry 4,
    /// `0x00c48e90`; the Switch's otherwise unused `opp_gline`, a line
    /// packet): `L = opus_no_get(set, unit, u16 unit +0xa8)`, the packet
    /// `P = [set +0x54]`. Host = `host +0x2c0` x `opus_parent_get(host,
    /// unit)` (`op_mul_m`); `P +0x10` = 0x8001, `+0x18` = 0x4141e, byte
    /// `+0x20` = byte unit `+0xc2`, byte `+0x34` = byte `+0xc3`, `+0x28` =
    /// 0x42. The unit's colour through the EE ([`gline_colour`]) to `P
    /// +0x30`, its point `op_rtp_w1(P +0x40, host, unit +0x90)`; `L`'s the
    /// same to `P +0x50` / `+0x60`. Then with `q` = `[set +0x54]` again,
    /// `op_ot_add(q, (P + 0x70 - q) / 16, 0, 0)`, whose `eax` is returned,
    /// and `[set +0x54]` = `P + 0x70`. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opp_gline<M: Mem>(
        mem: &mut M,
        host: u32,
        set: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let no = u32::from(mem.read::<u16>(u.wrapping_add(0xa8)));
        let other = go(mem, 0x0080_1060, &[set, u, no]);
        let p: u32 = mem.read(set.wrapping_add(0x54));
        let parent = go(mem, 0x0080_1f00, &[host, u]);
        go(mem, 0x007e_83b0, &[host, host.wrapping_add(0x2c0), parent]);
        mem.write(p.wrapping_add(0x10), 0x8001u32);
        mem.write(p.wrapping_add(0x18), 0x0004_141eu32);
        let b: u8 = mem.read(u.wrapping_add(0xc2));
        mem.write(p.wrapping_add(0x20), b);
        let b: u8 = mem.read(u.wrapping_add(0xc3));
        mem.write(p.wrapping_add(0x34), b);
        mem.write(p.wrapping_add(0x28), 0x42u32);
        for (src, at) in [(u, 0x30u32), (other, 0x50)] {
            gline_colour(mem, src);
            let v: [u32; 4] = mem.read(src.wrapping_add(0xc8));
            mem.write(p.wrapping_add(at), v);
            go(
                mem,
                0x007e_9b50,
                &[p.wrapping_add(at + 0x10), host, src.wrapping_add(0x90)],
            );
        }
        let end = p.wrapping_add(0x70);
        let cur: u32 = mem.read(set.wrapping_add(0x54));
        // `cdq; and edx, 0xf; add; sar 4`: rounded toward zero
        let n = (end.wrapping_sub(cur).cast_signed() / 16).cast_unsigned();
        let r = go(mem, 0x007e_c420, &[cur, n, 0, 0]);
        mem.write(set.wrapping_add(0x54), end);
        r
    }

    /// `opu_weapon_etc_008139b0(host, cmd)` (`0x008139b0`, name ours; opus
    /// table `0x00c490dc`): weapon commands by the flag nibble (jump table
    /// `0x00813bf8`), `c` = byte unit `+0x18`. 0: unit byte `+0xbb` = byte
    /// `cmd +2`, `n = ftol(f32 unit +0x40)` as a `u16` (8 for 0) to `+0xd0`,
    /// `u16 cmd +4`/`+6` to `+0xd2`/`+0xd4`, `+0xbc` = `op_malloc([host
    /// +0x3a8], (n << 9) + 0x120)` set up by `opus_mdc_rec_init(it, 8,
    /// 0x10, n)`, `opu_id_list_add(host +0x300, u16 host +0x214)`, `cmd +
    /// 8`. 1: with `u16 host +0x212` clear, the weapon `w` (below 0x14
    /// `MsGetWeaponHandle(c, 0)`, else `g_EtChrHandles[c]`): host =
    /// `Ch_GetRootGlobalMatrix`, `+0x40`/`+0xa0` its bones `u16 unit
    /// +0xd2`/`+0xd4` (`opus_get_bone_matrix`) each times host, then
    /// `op_2point_shadow([unit +0xbc], host +0x70, host +0xd0)`. 2: unit
    /// `+0xa8` = `op_et_weapon_unit(g_EtChrHandles[c], 0 x6)`, `u16 host
    /// +0x338` + 1 when `[host +0x324]` is `g_OpusBattle`'s word, `cmd +
    /// 0x12`. 3: `op_et_weapon_afterimage_fade([unit +0xa8])`. 4/5: below
    /// 8, `MsDetachWeapon(c, 0/1)`. 6/7: `MsSetChrShadow(c, 2/0)`. `cmd +
    /// 2`, `cmd` past 7. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_weapon_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::et::bridge::CHR_HANDLES;
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let h = |k: u32| host.wrapping_add(k);
        let c = |mem: &M| u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let half = |mem: &M, at: u32| u32::from(mem.read::<u16>(at));
        match mem.read::<u16>(h(0x21a)) >> 12 {
            0 => {
                let x = ffx_bridge::x87::load(mem.read(u.wrapping_add(0x40)));
                let b: u8 = mem.read(cmd.wrapping_add(2));
                mem.write(u.wrapping_add(0xbb), b);
                #[allow(clippy::cast_possible_truncation)] // `movzx ecx, ax`
                let n = match ffx_bridge::x87::ftol(mem, x) as u16 {
                    0 => 8,
                    n => n,
                };
                mem.write(u.wrapping_add(0xd0), n);
                let a: u16 = mem.read(cmd.wrapping_add(4));
                mem.write(u.wrapping_add(0xd2), a);
                let a: u16 = mem.read(cmd.wrapping_add(6));
                mem.write(u.wrapping_add(0xd4), a);
                let heap: u32 = mem.read(h(0x3a8));
                let p = go(mem, 0x007f_f780, &[heap, (u32::from(n) << 9) + 0x120]);
                mem.write(u.wrapping_add(0xbc), p);
                let n = half(mem, u.wrapping_add(0xd0));
                go(mem, 0x0081_3950, &[p, 8, 0x10, n]);
                let id = half(mem, h(0x214));
                go(mem, 0x0080_0d60, &[h(0x300), id]);
                return cmd.wrapping_add(8);
            }
            1 => {
                if mem.read::<u16>(h(0x212)) == 0 {
                    let n = c(mem);
                    let w = if n < 0x14 {
                        go(mem, 0x0079_5b40, &[n, 0])
                    } else {
                        CHR_HANDLES.ptr(mem).at(n).get(mem)
                    };
                    if w != 0 {
                        go(mem, 0x0083_3f80, &[w, host]);
                        let b = half(mem, u.wrapping_add(0xd2));
                        go(mem, 0x0080_5430, &[h(0x40), w, b]);
                        let b = half(mem, u.wrapping_add(0xd4));
                        go(mem, 0x0080_5430, &[h(0xa0), w, b]);
                        go(mem, 0x007e_83b0, &[h(0x40), host, h(0x40)]);
                        go(mem, 0x007e_83b0, &[h(0xa0), host, h(0xa0)]);
                        let sh: u32 = mem.read(u.wrapping_add(0xbc));
                        go(mem, 0x0081_3480, &[sh, h(0x70), h(0xd0)]);
                    }
                }
            }
            2 => {
                let n = c(mem);
                let w = CHR_HANDLES.ptr(mem).at(n).get(mem);
                let r = go(mem, 0x007f_c9b0, &[w, 0, 0, 0, 0, 0, 0]);
                mem.write(u.wrapping_add(0xa8), r);
                if mem.read::<u32>(h(0x324)) == OPUS_BATTLE.get(mem) {
                    let k: u16 = mem.read(h(0x338));
                    mem.write(h(0x338), k.wrapping_add(1));
                }
                return cmd.wrapping_add(0x12);
            }
            3 => {
                let id: u32 = mem.read(u.wrapping_add(0xa8));
                go(mem, 0x007f_ca40, &[id]);
            }
            k @ (4 | 5) => {
                let n = c(mem);
                if n < 8 {
                    go(mem, 0x0079_3f10, &[n, u32::from(k == 5)]);
                }
            }
            k @ (6 | 7) => {
                let n = c(mem);
                go(mem, 0x0079_62b0, &[n, if k == 6 { 2 } else { 0 }]);
            }
            _ => return cmd,
        }
        cmd.wrapping_add(2)
    }

    /// `opu_encount(host, cmd)` (`0x007f7e30`; the battle-start
    /// "encount" screen effects): by the `i16` command at `cmd +2` (jump
    /// table `0x007f819c`), unit `u` = `[host +0x220]`, `oef` = host
    /// `+0x360[byte u +0x1e]`, the heap `[host +0x3a8]`, `p` = `[u +0xbc]`.
    /// 0: `p` = `op_malloc(heap, 0xd830)`, `u16 p +6` = 12, `p +2` =
    /// 0x000c000c, `p +8` = `op_oef2_a_get(oef, 4, i16 cmd +4)`,
    /// `opus_encount0_init(p)`, `cmd + 6`. 1: `opus_encount0_draw(host, u,
    /// p)`. 2: `p` made (0xe0 bytes) when 0, `p +0` = `[host +0x324]`, `+4`
    /// `u`, `+8` `opus_no_get(host +0x300, u, u16 cmd +4)`, `+0xc` `oef`,
    /// `+0x10` `cmd`; `op_unit_m_scale(p +0x20, 0.0625)`, `p +0x60` = the
    /// parent's matrix (`opus_parent_get`, `op_m_copy`), `p +0xa0` =
    /// `FFXSamp0ViewScreenMatrix(512, 0.8, 0.87285, f32 u +0x40 + 2048, f32
    /// u +0x44 + 2048, 1, 16777215, 1, 65536)` (sums rounded to `f32`), the
    /// noise draw `0x007f73d0` queued after the 3D OT
    /// (`op_func_after_ot3d_set(it, p, i16 host +0x214, 1)`), `cmd + 0x12`.
    /// 4: `MsBattleStartReady()`, or else byte host `+0x210` = 1 and `cmd`.
    /// 8: `p` = `op_malloc(heap, opus_encount4_size_get(oef, i16 cmd +4,
    /// i16 cmd +6))`, `graphicBrokenScreenReset()`, `opus_encount4_init(p,
    /// oef, [host +0x324], u, cmd)`, `cmd + 10`. 9: `opus_encount4_calc(p,
    /// host, host +0x300, u, opus_no_get(host +0x300, u, u16 cmd +4), cmd)`;
    /// 0 sets bit 0x8000 of `u16 u +0xb0`; `cmd + 6`. 10/11: `p +0x60` = the
    /// parent's matrix, `op_func_final_set(0x007f75e0 / 0x007f7a40, p, cmd,
    /// 1)`, `cmd + 10`. 12/13: host `+0..+0x10` = `g_Ee_Gpr`'s first 16
    /// bytes with the first two floats 0 / 2048, then
    /// `opus_encount5_make(0x01540870, host)` / `opus_encount5_draw(host,
    /// u, 0x01540870, u +0x90, [host +0x328])`, `cmd + 2`. Others `cmd + 4`
    /// (1, 4), or `cmd` (3, 5..7, past 13, negative). Callees through
    /// `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_encount<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let op = mem.read::<i16>(cmd.wrapping_add(2));
        let u = unit(mem, host);
        let h = |k: u32| host.wrapping_add(k);
        let slot = u32::from(mem.read::<u8>(u.wrapping_add(0x1e)));
        let oef: u32 = mem.read(h(0x360 + 4 * slot));
        let half = |mem: &M, at: u32| u32::from(mem.read::<u16>(at));
        let short = |mem: &M, at: u32| i32::from(mem.read::<i16>(at)).cast_unsigned();
        let heap = |mem: &M| mem.read::<u32>(h(0x3a8));
        match op {
            0 => {
                let hp = heap(mem);
                let p = go(mem, 0x007f_f780, &[hp, 0xd830]);
                mem.write(u.wrapping_add(0xbc), p);
                mem.write(p.wrapping_add(6), 0xcu16);
                mem.write(p.wrapping_add(2), 0x000c_000cu32);
                let n = short(mem, cmd.wrapping_add(4));
                let a = go(mem, 0x007e_3810, &[oef, 4, n]);
                mem.write(p.wrapping_add(8), a);
                go(mem, 0x007f_8480, &[p]);
                cmd.wrapping_add(6)
            }
            1 => {
                let p: u32 = mem.read(u.wrapping_add(0xbc));
                go(mem, 0x007f_81e0, &[host, u, p]);
                cmd.wrapping_add(4)
            }
            2 => {
                if mem.read::<u32>(u.wrapping_add(0xbc)) == 0 {
                    let hp = heap(mem);
                    let p = go(mem, 0x007f_f780, &[hp, 0xe0]);
                    mem.write(u.wrapping_add(0xbc), p);
                }
                let p: u32 = mem.read(u.wrapping_add(0xbc));
                let w: u32 = mem.read(h(0x324));
                mem.write(p, w);
                mem.write(p.wrapping_add(4), u);
                let no = half(mem, cmd.wrapping_add(4));
                let n = go(mem, 0x0080_1060, &[h(0x300), u, no]);
                mem.write(p.wrapping_add(8), n);
                mem.write(p.wrapping_add(0xc), oef);
                mem.write(p.wrapping_add(0x10), cmd);
                go(
                    mem,
                    0x007e_a700,
                    &[p.wrapping_add(0x20), 0.0625f32.to_bits()],
                );
                let parent = go(mem, 0x0080_1f00, &[host, u]);
                go(mem, 0x007e_8010, &[p.wrapping_add(0x60), parent]);
                // `fadd` of the f64 2048 to each f32 centre, `fstp` to f32
                #[allow(clippy::cast_possible_truncation)]
                let centre =
                    |mem: &M, at: u32| (f64::from(f(mem, u.wrapping_add(at))) + 2048.0) as f32;
                let cy = centre(mem, 0x44);
                let cx = centre(mem, 0x40);
                let scrz: u32 = mem.read(g(mem, 0x00b4_1d9c));
                let args = [
                    p.wrapping_add(0xa0),
                    scrz,
                    0.8f32.to_bits(),
                    0.872_85f32.to_bits(),
                    cx.to_bits(),
                    cy.to_bits(),
                    1.0f32.to_bits(),
                    16_777_215.0f32.to_bits(),
                    1.0f32.to_bits(),
                    65536.0f32.to_bits(),
                ];
                go(mem, 0x0093_d960, &args);
                let noise = g(mem, 0x007f_73d0);
                let ot = short(mem, h(0x214));
                go(mem, 0x007e_b320, &[noise, p, ot, 1]);
                cmd.wrapping_add(0x12)
            }
            4 => {
                if go(mem, 0x0078_1650, &[]) != 0 {
                    return cmd.wrapping_add(4);
                }
                mem.write(h(0x210), 1u8);
                cmd
            }
            8 => {
                let (a, b) = (
                    short(mem, cmd.wrapping_add(4)),
                    short(mem, cmd.wrapping_add(6)),
                );
                let size = go(mem, 0x007f_8e30, &[oef, a, b]);
                let hp = heap(mem);
                let p = go(mem, 0x007f_f780, &[hp, size]);
                mem.write(u.wrapping_add(0xbc), p);
                go(mem, 0x0063_9490, &[]);
                let p: u32 = mem.read(u.wrapping_add(0xbc));
                let w: u32 = mem.read(h(0x324));
                go(mem, 0x007f_8930, &[p, oef, w, u, cmd]);
                cmd.wrapping_add(10)
            }
            9 => {
                let no = half(mem, cmd.wrapping_add(4));
                let n = go(mem, 0x0080_1060, &[h(0x300), u, no]);
                let p: u32 = mem.read(u.wrapping_add(0xbc));
                if go(mem, 0x007f_8570, &[p, host, h(0x300), u, n, cmd]) == 0 {
                    let fl: u16 = mem.read(u.wrapping_add(0xb0));
                    mem.write(u.wrapping_add(0xb0), fl | 0x8000);
                }
                cmd.wrapping_add(6)
            }
            10 | 11 => {
                let p: u32 = mem.read(u.wrapping_add(0xbc));
                let parent = go(mem, 0x0080_1f00, &[host, u]);
                go(mem, 0x007e_8010, &[p.wrapping_add(0x60), parent]);
                let step = g(mem, if op == 10 { 0x007f_75e0 } else { 0x007f_7a40 });
                go(mem, 0x007e_b410, &[step, p, cmd, 1]);
                cmd.wrapping_add(10)
            }
            12 | 13 => {
                let gpr: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
                mem.write(host, gpr);
                let v = if op == 12 { 0.0f32 } else { 2048.0 };
                mem.write(host, v.to_bits());
                mem.write(h(4), v.to_bits());
                let set = g(mem, 0x0154_0870);
                if op == 12 {
                    go(mem, 0x007f_9280, &[set, host]);
                } else {
                    let w: u32 = mem.read(h(0x328));
                    go(mem, 0x007f_8e80, &[host, u, set, u.wrapping_add(0x90), w]);
                }
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `opu_vran_etc(host, cmd)` (`0x0081abe0`): random placement by the
    /// flag nibble, unit `u` = `[host +0x220]`; each float goes through
    /// the x87 (`f32` at every store). 0: host = `op_unit_m`, `host
    /// +0x80..+0x88` = three `op_random(4096)`, host = `op_rt_m(host, host
    /// +0x80, u +0x40)`, `host +0x90` = `g_Ee_Gpr`'s first 16 bytes with
    /// `+0x98` = `f32 u +0x30[cmd +2 & 0xf]`, `u +0x40` =
    /// `op_apply_m_w1(host, host +0x90)`, `cmd + 4`. 0x1000: `host +0..+8`
    /// = `u +0x40..` plus `op_random(u +0x30..)` each, `cmd + 2`. 0x2000:
    /// `host +0..+8` = `op_spline_draw_path3(i16 cmd +2/+4/+6, i16 cmd
    /// +8/+0xa/+0xc / 256)`, turned by `op_rot_m_y_apply(host, host, a)`
    /// (`a` the `f32` at `u` plus `u16 cmd +0xe`); the sums `host + u +0x40` (x, y, z) land in
    /// `g_Vu0_VfMove` (`0x00c8f980`, with `u +0x4c`) and `g_Ee_Gpr +0x20`
    /// (z and x swapped at the end), x and y at `u +0x40 + 0x44` offset by
    /// `g_pppColorConShift` (then cleared), z at `u +0x48`; `0x00c8f970`
    /// keeps `host +0..+0xc`; `cmd + 0x10`. Every case but the others
    /// (`cmd`) ends with `op_xyz_scale(u +0x90, u +0x40 or host, 1/16)`.
    /// Callees through `call`, the float results through `fcall`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_vran_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let h = |k: u32| host.wrapping_add(k);
        let ua = |k: u32| u.wrapping_add(k);
        // `fld m32; fstp m32`
        let copy = |mem: &M, at: u32| store(load(mem.read(at)));
        let sixteenth = 0.0625f32.to_bits();
        match nibble(mem, host) {
            0 => {
                go(mem, 0x007e_a6d0, &[host]);
                for k in 0..3 {
                    let f = g(mem, 0x007e_6700);
                    let r = store(fcall(mem, f, &[4096.0f32.to_bits()]));
                    mem.write(h(0x80 + 4 * k), r);
                }
                go(mem, 0x007e_9850, &[host, host, h(0x80), ua(0x40)]);
                let i = u32::from(mem.read::<u16>(cmd.wrapping_add(2)) & 0xf);
                let gpr: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
                mem.write(h(0x90), gpr);
                let v = copy(mem, ua(0x30 + 4 * i));
                mem.write(h(0x98), v);
                go(mem, 0x007e_79a0, &[ua(0x40), host, h(0x90)]);
                go(mem, 0x007e_a850, &[ua(0x90), ua(0x40), sixteenth]);
                cmd.wrapping_add(4)
            }
            0x1000 => {
                for k in 0..3 {
                    let x = copy(mem, ua(0x30 + 4 * k));
                    let f = g(mem, 0x007e_6700);
                    let r = store(fcall(mem, f, &[x]));
                    let v = store(add(load(mem.read(ua(0x40 + 4 * k))), load(r)));
                    mem.write(h(4 * k), v);
                }
                go(mem, 0x007e_a850, &[ua(0x90), host, sixteenth]);
                cmd.wrapping_add(2)
            }
            0x2000 => {
                let short = |mem: &M, at: u32| f64::from(mem.read::<i16>(cmd.wrapping_add(at)));
                let k: f64 = mem.read(g(mem, 0x00b5_0198));
                let spread = [8, 0xa, 0xc].map(|at| store(mul(short(mem, at), k)));
                for (i, at) in [2u32, 4, 6].into_iter().enumerate() {
                    let x = store(short(mem, at));
                    let f = g(mem, 0x007e_67f0);
                    let r = store(fcall(mem, f, &[x, spread[i]]));
                    mem.write(h(2 * at - 4), r);
                }
                let off = u32::from(mem.read::<u16>(cmd.wrapping_add(0xe)));
                let angle = copy(mem, ua(off));
                go(mem, 0x007e_7be0, &[host, host, angle]);
                let pos: [u32; 4] = mem.read(ua(0x40));
                let vf_move = g(mem, 0x00c8_f980);
                mem.write(vf_move, pos);
                let at: [u32; 4] = mem.read(host);
                mem.write(g(mem, 0x00c8_f970), at);
                let x = store(add(load(at[0]), load(pos[0])));
                let y = store(add(load(at[1]), load(pos[1])));
                let z = store(add(load(pos[2]), load(at[2])));
                mem.write(vf_move, [x, y, z]);
                let gpr = g(mem, 0x00c8_f528);
                mem.write(gpr, [x, y, z, pos[3]]);
                let shift = g(mem, 0x00c8_b500);
                // reread after each store, as the original does
                let off: u32 = mem.read(shift);
                mem.write(off.wrapping_add(ua(0x40)), x);
                let off: u32 = mem.read(shift);
                let y: u32 = mem.read(gpr.wrapping_add(4));
                mem.write(off.wrapping_add(ua(0x44)), y);
                let (z, x): (u32, u32) = (mem.read(gpr.wrapping_add(8)), mem.read(gpr));
                mem.write(gpr, z);
                mem.write(gpr.wrapping_add(8), x);
                mem.write(ua(0x48), z);
                mem.write(shift, 0u32);
                go(mem, 0x007e_a850, &[ua(0x90), ua(0x40), sixteenth]);
                cmd.wrapping_add(0x10)
            }
            _ => cmd,
        }
    }

    /// `opu_clut_load(host, cmd)` (`0x00808000`; opus table `0x00c48f68`):
    /// CLUT work by the flag nibble, `chr` = `g_EtChrHandles[byte unit
    /// +0x18]`, `oef` = host `+0x360[byte unit +0x1e]`, `buf` = unit
    /// `+0xbc`, the heap `[host +0x3a8]`, `hsv` = `ftol` of unit `f32 +0x40,
    /// +0x44, +0x48, +0x3c`. 0: `op_oef2_clut_load(op_oef2_a_get(oef, 3, i16
    /// cmd +2), 1)`, `cmd + 4`. 1: `Ch_ClutCreate(chr)`; with
    /// `op_chara_hsv_size_get(chr)` bytes, `buf` freed, made anew and
    /// filled (`op_chara_hsv_make`), else 0. 2: with `buf`,
    /// `op_chara_hsv(chr, buf, hsv)`. 3: `buf` freed (and 0),
    /// `Ch_ClutDispose(chr)`. 4: `n` bytes (`Ch_TexGetClut(chr, i16 cmd +2,
    /// &n)`, the count in the local at `count`) copied 16 at a time from
    /// `op_oef2_a_get(oef, 9, i16 cmd +4)` to the CLUT it returns, `cmd +
    /// 6` (without texture data `Ch_TexGetClut` leaves the local alone and
    /// returns 0: the original then copies its stack leftover's worth to
    /// address 0, the shim's zeroed local nothing). 5: with `buf`,
    /// `op_chara_hsv` as 2 with bit 0 of the
    /// character's `g_EtChr` byte `+0x17` cleared meanwhile. 6: the CLUT
    /// `c = op_oef2_a_get(oef, 3, k)` (`k` = `i16 cmd +2`) recoloured in a
    /// 0x20000-byte `op_malloc` block `b = + (k & 7) << 14`: its 16-byte
    /// head to `b +0x2000`, `op_rgb_to_hsv(b, c's colours, u16 c +2 >> 2)`,
    /// `opus_hsv_rgba(b +0x1000, b, that count, hsv)`, the head's colours
    /// pointed at `b +0x1000` and `op_oef2_clut_load(b +0x2000, 1)`, the
    /// block freed, `cmd + 4`. Others `cmd + 2`, `cmd` for 7 and up.
    /// Callees through `call`.
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::similar_names
    )]
    pub fn opu_clut_load<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        count: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::et::bridge::{CHR, CHR_HANDLES};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let chr = CHR_HANDLES
            .ptr(mem)
            .at(u32::from(mem.read::<u8>(u.wrapping_add(0x18))))
            .get(mem);
        let slot = u32::from(mem.read::<u8>(u.wrapping_add(0x1e)));
        let oef: u32 = mem.read(host.wrapping_add(0x360 + 4 * slot));
        let buf = u.wrapping_add(0xbc);
        let heap = |mem: &M| mem.read::<u32>(host.wrapping_add(0x3a8));
        let hsv = |mem: &M| {
            [0x40u32, 0x44, 0x48, 0x3c].map(|o| {
                let x = ffx_bridge::x87::load(mem.read(u.wrapping_add(o)));
                ffx_bridge::x87::ftol(mem, x)
            })
        };
        let half =
            |mem: &M, at: u32| i32::from(mem.read::<i16>(cmd.wrapping_add(at))).cast_unsigned();
        match mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000 {
            0 => {
                let k = half(mem, 2);
                let c = go(mem, 0x007e_3810, &[oef, 3, k]);
                go(mem, 0x007e_46b0, &[c, 1]);
                return cmd.wrapping_add(4);
            }
            0x1000 => {
                go(mem, 0x0083_c710, &[chr]);
                let size = go(mem, 0x0080_7b20, &[chr]);
                if size == 0 {
                    mem.write(buf, 0u32);
                } else {
                    let (h, old) = (heap(mem), mem.read::<u32>(buf));
                    go(mem, 0x007f_f1d0, &[h, old]);
                    let h = heap(mem);
                    let p = go(mem, 0x007f_f780, &[h, size]);
                    mem.write(buf, p);
                    go(mem, 0x0080_79f0, &[chr, p]);
                }
            }
            0x2000 => {
                let b: u32 = mem.read(buf);
                if b != 0 {
                    let [h, s, v, m] = hsv(mem);
                    go(mem, 0x0080_7870, &[chr, b, h, s, v, m]);
                }
            }
            0x3000 => {
                let b: u32 = mem.read(buf);
                if b != 0 {
                    let h = heap(mem);
                    go(mem, 0x007f_f1d0, &[h, b]);
                    mem.write(buf, 0u32);
                }
                go(mem, 0x0083_c810, &[chr]);
            }
            0x4000 => {
                let k = half(mem, 4);
                let src = go(mem, 0x007e_3810, &[oef, 9, k]);
                let i = half(mem, 2);
                let dst = go(mem, 0x0083_cd20, &[chr, i, count]);
                let mut done = 0i32;
                while done < mem.read::<i32>(count) {
                    let at = done.cast_unsigned();
                    let v: [u32; 4] = mem.read(src.wrapping_add(at));
                    mem.write(dst.wrapping_add(at), v);
                    done = done.wrapping_add(0x10);
                }
                return cmd.wrapping_add(6);
            }
            0x5000 => {
                if mem.read::<u32>(buf) != 0 {
                    let i = go(mem, 0x007f_e970, &[chr, 0, 0]);
                    let flags = CHR.ptr(mem).at(i).addr.wrapping_add(0x17);
                    let old: u8 = mem.read(flags);
                    mem.write(flags, old & 0xfe);
                    let [h, s, v, m] = hsv(mem);
                    let b: u32 = mem.read(buf);
                    go(mem, 0x0080_7870, &[chr, b, h, s, v, m]);
                    mem.write(flags, old);
                }
            }
            0x6000 => {
                let k = half(mem, 2);
                let c = go(mem, 0x007e_3810, &[oef, 3, k]);
                let h = heap(mem);
                let block = go(mem, 0x007f_f780, &[h, 0x2_0000]);
                let b = block.wrapping_add((k & 7) << 14);
                let head = b.wrapping_add(0x2000);
                let v: [u32; 4] = mem.read(c);
                mem.write(head, v);
                let n = u32::from(mem.read::<u16>(c.wrapping_add(2))) >> 2;
                let colours = ffx_system::stub::ptr32(mem.read(c.wrapping_add(4)));
                go(mem, 0x0080_7e90, &[b, colours, n]);
                let [hh, s, v, m] = hsv(mem);
                let n = u32::from(mem.read::<u16>(c.wrapping_add(2))) >> 2;
                let out = b.wrapping_add(0x1000);
                go(mem, 0x0080_7b70, &[out, b, n, hh, s, v, m]);
                mem.write(head.wrapping_add(4), out);
                go(mem, 0x007e_46b0, &[head, 1]);
                let h = heap(mem);
                go(mem, 0x007f_f1d0, &[h, block]);
                return cmd.wrapping_add(4);
            }
            _ => return cmd,
        }
        cmd.wrapping_add(2)
    }

    /// The two `opu_v` setters' differences.
    #[derive(Debug, Clone, Copy)]
    pub struct VSet {
        /// The scale table (four words a kind).
        pub scales: u32,
        /// `0x00818a30`: the kind is flag bits 9..11 (else the low nine
        /// bits less 10 as an `i16`), and its last test compares the
        /// emulated `t?` register `0x00c8f538` (never set here) with `r0`.
        pub stale: bool,
    }

    /// `opu_v_set_00818410`.
    pub const V_SET: VSet = VSet {
        scales: 0x00c4_9398,
        stale: false,
    };
    /// `opu_v_set_sel_00818a30`.
    pub const V_SET_SEL: VSet = VSet {
        scales: 0x00c4_93f8,
        stale: true,
    };

    /// `opu_v_set_00818410(host, cmd)` / `opu_v_set_sel_00818a30(host, cmd)`
    /// (names ours; opus commands recompiled from the EE, 64-bit register
    /// compares included): for kind `k` (see [`VSet::stale`]) the unit's
    /// vector at `unit + i16 0x00c49388[k]` takes the `i16`s from `cmd +2`
    /// times the kind's scale words, each through `f32` / 4096 into
    /// `g_Vu0_VfMove` (`0x00c8f980`; the products also into the emulated
    /// `v0`..`v1` words `0x00c8f528..+0x34`). Each of flag bits 12..15 that
    /// differs from `r0` (`g_Ee_Gpr`, as a 64-bit value) takes the next
    /// lane: word `k` of the vector is the first lane, the four rotate, and
    /// the command pointer (from `cmd + 2`) moves 2 (the first bit's
    /// lane moves it in all cases but the fourth's comes only from bit
    /// 15). `0x00c8f558..+0x64` = the lanes after bit 14; then
    /// `0x00c8f528` = the pointer, `0x00c8f548` = host; with flag 0x200
    /// (or, `stale`, `0x00c8f538/c`) equal to `r0` the pointer is returned,
    /// else the unit's motion callback (`0x00c492c8[byte +0xba]`) runs on
    /// `(host, pointer)` and its `eax` is returned. Callees through `call`.
    #[allow(clippy::too_many_lines)]
    pub fn opu_v_set<M: Mem>(
        mem: &mut M,
        v: &VSet,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{mul, store};
        let u = unit(mem, host);
        let flags = u32::from(mem.read::<u16>(host.wrapping_add(0x21a)));
        let kind = if v.stale {
            (flags >> 9) & 7
        } else {
            // `sub eax, 10; movzx eax, ax; movsx edx, ax`
            #[allow(clippy::cast_possible_truncation)]
            let k = ((flags & 0x1ff).wrapping_sub(10) as u16).cast_signed();
            i32::from(k).cast_unsigned()
        };
        let scale: [u32; 4] = mem.read(g(mem, v.scales).wrapping_add(kind.wrapping_mul(16)));
        let off = mem.read::<i16>(g(mem, 0x00c4_9388).wrapping_add(kind.wrapping_mul(2)));
        let dst = u.wrapping_add(i32::from(off).cast_unsigned());
        let src = cmd.wrapping_add(2);
        let p: [u32; 4] = std::array::from_fn(|i| {
            #[allow(clippy::cast_possible_truncation)] // 0..4
            let n = i32::from(mem.read::<i16>(src.wrapping_add(2 * i as u32)));
            scale[i].wrapping_mul(n.cast_unsigned())
        });
        let v0 = g(mem, 0x00c8_f528);
        mem.write(v0, p[0]);
        mem.write(v0.wrapping_add(4), p[1]);
        mem.write(v0.wrapping_add(8), p[2]);
        mem.write(v0.wrapping_add(0xc), p[3]);
        // `fild`, `fstp dword`, times the `f64` 1/4096, `fstp dword`
        #[allow(clippy::cast_precision_loss)]
        let mut lanes = p.map(|w| store(mul(f64::from(w.cast_signed() as f32), 0.000_244_140_625)));
        let mov = g(mem, 0x00c8_f980);
        let r0: u64 = mem.read(g(mem, 0x00c8_f508));
        let differs = |bit: u32| u64::from(flags & bit) != r0;
        let mut ptr = if differs(0x1000) {
            src.wrapping_add(2)
        } else {
            src
        };
        for (k, bit) in [(0u32, 0x1000u32), (4, 0x2000), (8, 0x4000)] {
            if differs(bit) {
                mem.write(dst.wrapping_add(k), lanes[0]);
                lanes.rotate_left(1);
                if bit != 0x1000 {
                    ptr = ptr.wrapping_add(2);
                }
            }
        }
        mem.write(mov, lanes);
        let tail = g(mem, 0x00c8_f558);
        mem.write(tail, lanes);
        if !v.stale {
            mem.write(v0.wrapping_add(4), 0u32);
        }
        if differs(0x8000) {
            ptr = ptr.wrapping_add(2);
            mem.write(dst.wrapping_add(0xc), lanes[0]);
        }
        let low = flags & 0x200;
        let done = if v.stale {
            mem.write(v0, low);
            mem.write(g(mem, 0x00c8_f548), host);
            mem.write(v0.wrapping_add(4), 0u32);
            mem.read::<u64>(v0.wrapping_add(0x10)) == r0
        } else {
            mem.write(v0, ptr);
            mem.write(v0.wrapping_add(0x10), low);
            mem.write(v0.wrapping_add(0x14), 0u32);
            mem.write(g(mem, 0x00c8_f548), host);
            u64::from(low) == r0
        };
        if done {
            return ptr;
        }
        mem.write(tail, ptr);
        let cb = u32::from(mem.read::<u8>(u.wrapping_add(0xba)));
        let table = g(mem, 0x00c4_92c8);
        // `shld; shl 2; add; adc`: the 64-bit entry address into `v0`
        let at = u64::from(cb << 2) + u64::from(table);
        #[allow(clippy::cast_possible_truncation)] // the halves
        let (lo, hi) = (at as u32, (at >> 32) as u32);
        let f: u32 = mem.read(lo);
        mem.write(v0, f);
        mem.write(v0.wrapping_add(4), hi);
        call(mem, f, &[host, ptr])
    }

    /// `opu_v_en_move(host, cmd)` (`0x008190e0`; opus table): a battle
    /// character's position, direction and moves by the flag nibble; `c` =
    /// byte unit `+0x18`, `e` = the `g_EtEff` record `u16 unit +0x14`, `t`
    /// = byte `e +0x18` (the target), `w` = byte `e +0x10`. 0:
    /// `opus_set_chr_pos(c, unit +0x90)`; 2: `opus_set_chr_dirf(c,
    /// opus_normaldeg_to_ffxdeg(f32 unit +0x34))`; 3/4:
    /// `MsSetChrMoveFlag(t with flag 0x800 else c, 1/0)`; 5: host =
    /// `MsGetChrBtlPos(c)`, `opus_set_chr_pos(c, host)`,
    /// `opus_set_chr_dirf(c, MsGetChrBtlDir(c))`; 6/7:
    /// `MsGetMoveTargetDist` with the speed `i16 cmd +4 / 16` and the
    /// frame count `n` (`i16 cmd +2`, with 0x800 `ftol` of the unit's
    /// `f32` at that offset): `(t, 2, w, 0.0, n, 0, 0, speed, 1.0, 0.0,
    /// 0.0)` / `(t, 1, t, 0.0, n, ...)`, `cmd + 6`; 8/9:
    /// `MsSetChrDirTarget(t, 2/1, 0, w, 0, n)` with `n` as for 6, except
    /// that 9 with 0x800 takes `ftol` of the command pointer's own bits
    /// read as an `f32` (as the original does), `cmd + 4`; 0xa:
    /// `MsSetChrPos2D(c, unit +0x90)`; 0xb: `MsMotionSpeedResetChr([e])`.
    /// `cmd + 2`, `cmd` for 1 and 0xc..0xf. Callees through `call`, the
    /// two float ones through `fcall` (their `ST0`).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_v_en_move<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load, mul, store};
        let u = unit(mem, host);
        let flags = mem.read::<u16>(host.wrapping_add(0x21a));
        let flagged = flags & 0x800 != 0;
        let e = EFF
            .ptr(mem)
            .at(u32::from(mem.read::<u16>(u.wrapping_add(0x14))))
            .addr;
        let byte = |mem: &M, at: u32| u32::from(mem.read::<u8>(at));
        let c = |mem: &M| byte(mem, u.wrapping_add(0x18));
        let half =
            |mem: &M, at: u32| i32::from(mem.read::<i16>(cmd.wrapping_add(at))).cast_unsigned();
        // the frame count: the command's `i16`, with flag 0x800 `ftol` of
        // the unit's `f32` at that offset
        let count = |mem: &M| {
            let n = half(mem, 2);
            if flagged {
                ftol(mem, load(mem.read(u.wrapping_add(n))))
            } else {
                n
            }
        };
        // `fild`, `fstp qword`, times the `f64` 1/16, `fstp dword`
        let speed = |mem: &M| store(mul(f64::from(half(mem, 4).cast_signed()), 0.0625));
        let (zero, one) = (0u32, 0x3f80_0000u32);
        match flags & 0xf000 {
            0 => {
                let n = c(mem);
                call(mem, g(mem, 0x0081_b2b0), &[n, u.wrapping_add(0x90)]);
            }
            0x2000 => {
                let x = store(load(mem.read(u.wrapping_add(0x34))));
                let d = store(fcall(mem, g(mem, 0x0081_b230), &[x]));
                let n = c(mem);
                call(mem, g(mem, 0x0081_b270), &[n, d]);
            }
            k @ (0x3000 | 0x4000) => {
                let n = if flagged {
                    byte(mem, e.wrapping_add(0x18))
                } else {
                    c(mem)
                };
                call(mem, g(mem, 0x0079_6110), &[n, u32::from(k == 0x3000)]);
            }
            0x5000 => {
                let n = c(mem);
                call(mem, g(mem, 0x0079_48e0), &[n, host]);
                let n = c(mem);
                call(mem, g(mem, 0x0081_b2b0), &[n, host]);
                let n = c(mem);
                let d = store(fcall(mem, g(mem, 0x0079_48a0), &[n]));
                let n = c(mem);
                call(mem, g(mem, 0x0081_b270), &[n, d]);
            }
            k @ (0x6000 | 0x7000) => {
                let s = speed(mem);
                let n = count(mem);
                let t = byte(mem, e.wrapping_add(0x18));
                let (kind, with) = if k == 0x6000 {
                    (2, byte(mem, e.wrapping_add(0x10)))
                } else {
                    (1, t)
                };
                call(
                    mem,
                    g(mem, 0x007a_b540),
                    &[t, kind, with, zero, n, 0, 0, s, one, zero, zero],
                );
                return cmd.wrapping_add(6);
            }
            k @ (0x8000 | 0x9000) => {
                let n = if k == 0x9000 && flagged {
                    ftol(mem, load(cmd))
                } else {
                    count(mem)
                };
                let w = byte(mem, e.wrapping_add(0x10));
                let t = byte(mem, e.wrapping_add(0x18));
                let kind = if k == 0x8000 { 2 } else { 1 };
                call(mem, g(mem, 0x0079_5eb0), &[t, kind, 0, w, 0, n]);
                return cmd.wrapping_add(4);
            }
            0xa000 => {
                let n = c(mem);
                call(mem, g(mem, 0x0079_61d0), &[n, u.wrapping_add(0x90)]);
            }
            0xb000 => {
                let chr: u32 = mem.read(e);
                call(mem, g(mem, 0x007a_aff0), &[chr]);
            }
            _ => return cmd,
        }
        cmd.wrapping_add(2)
    }

    /// `opu_dvd(host, cmd)` (`0x008148c0`; opus table `0x00c492c0`):
    /// character data loads by `i16 cmd +2` (jump table `0x008149e8`),
    /// `a`/`b` the `i16`s at `cmd +4`/`+6`. 0: `Ch_DataRead(a)`, `cmd + 6`;
    /// 1: `Ch_DataReadSync(a)`, 1 (loaded) gives `cmd` back with byte host
    /// `+0x210` = 1 (wait), else `cmd + 6`; 2: `Ch_MgrpRead_thunk(a, b)`,
    /// `cmd + 8`; 3: `ChN_ReadSystemMGRPsync(a, b)` as 1, else
    /// `ChN_ReadSystemMGRP(a, b)` and `cmd + 8`; 4: `Ch_Allocate(a)`, the
    /// new `g_EtChr[op_et_chr_index_by_no(a)]` character hidden
    /// (`graphicCharacterInstanceSetVisible(Ch_GetChrRenderInstance(chr),
    /// 0)`), `cmd + 6`; 5: that character (when there is one)
    /// `Ch_Dispose`d with `ChN_DataDispose(a, 1)` and
    /// `ChN_DisposeSystemMGRP(a, b)`, `cmd + 8`; else `cmd`. Callees
    /// through `call`.
    pub fn opu_dvd<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let half =
            |mem: &M, at: u32| i32::from(mem.read::<i16>(cmd.wrapping_add(at))).cast_unsigned();
        let wait = |mem: &mut M| {
            mem.write(host.wrapping_add(0x210), 1u8);
            cmd
        };
        let chr = |mem: &M, i: u32| crate::et::bridge::CHR.ptr(mem).at(i).addr;
        match mem.read::<i16>(cmd.wrapping_add(2)) {
            0 => {
                let a = half(mem, 4);
                go(mem, 0x0082_9ef0, &[a]);
                cmd.wrapping_add(6)
            }
            1 => {
                let a = half(mem, 4);
                if go(mem, 0x0082_a040, &[a]) == 1 {
                    return wait(mem);
                }
                cmd.wrapping_add(6)
            }
            2 => {
                let (a, b) = (half(mem, 4), half(mem, 6));
                go(mem, 0x0083_6a40, &[a, b]);
                cmd.wrapping_add(8)
            }
            3 => {
                let (a, b) = (half(mem, 4), half(mem, 6));
                if go(mem, 0x0083_6a50, &[a, b]) == 1 {
                    return wait(mem);
                }
                let (a, b) = (half(mem, 4), half(mem, 6));
                go(mem, 0x0083_6870, &[a, b]);
                cmd.wrapping_add(8)
            }
            4 => {
                let a = half(mem, 4);
                go(mem, 0x0082_4f90, &[a]);
                let i = go(mem, 0x007f_eaa0, &[a]);
                let c: u32 = mem.read(chr(mem, i));
                if c != 0 {
                    let inst = go(mem, 0x0082_6b70, &[c]);
                    go(mem, 0x0063_c390, &[inst, 0]);
                }
                cmd.wrapping_add(6)
            }
            5 => {
                let a = half(mem, 4);
                let i = go(mem, 0x007f_eaa0, &[a]);
                let c: u32 = mem.read(chr(mem, i));
                if c != 0 {
                    go(mem, 0x0082_66f0, &[c]);
                    go(mem, 0x0082_4f00, &[a, 1]);
                    let b = half(mem, 6);
                    go(mem, 0x0083_6830, &[a, b]);
                }
                cmd.wrapping_add(8)
            }
            _ => cmd,
        }
    }

    /// `opu_en_lock(host, cmd)` (`0x00816030`; opus table `0x00c49058`):
    /// the unit's character number (byte `unit +0x18`) by the host's flag
    /// nibble (`u16 host +0x21a >> 12`, jump table `0x008161bc`); `e` =
    /// the `g_EtEff` record `u16 unit +0x14`, `c(i)` = byte `+0xe` of
    /// `g_EtChr[i]`. 0: `e +0x10` (or `e +0x18` when that is 0xff), `cmd +
    /// 2`; 1: byte `cmd +2` when byte host `+0x21e` is set, `cmd + 4`; 2:
    /// without `+0x21e`, `g_EtChrHandles[n]` =
    /// `AtelGetCharHandleFromBindPC2(n)` for `n` = `u16 cmd +2`, `cmd + 4`;
    /// 3: only in debug mode (`g_OpDebugMode`), byte `cmd +2`, or when its
    /// handle is none `c(opus_get_chr_status_index(opus_chr_handle_nth(0),
    /// 0, 0))`, `cmd + 4`; 5: `c(op_et_chr_index_by_no(i16 cmd +2))` for a
    /// found one, `cmd + 4`; 6: `MsGetSummonerNum(number)`, `cmd + 4`
    /// below 0x20, else the jump `cmd + i16 cmd +2`; 8: `e +0x18`, `cmd +
    /// 2`; 9 and 10: `MsGetWeaponHandle(number, 0 / 1)`, the jump without
    /// one, else `c(opus_get_chr_status_index(weapon, 0, 0))` for a found
    /// one and `cmd + 4`; else `cmd`. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_en_lock<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::et::bridge::{CHR, CHR_HANDLES};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let no = u.wrapping_add(0x18);
        let e = EFF
            .ptr(mem)
            .at(u32::from(mem.read::<u16>(u.wrapping_add(0x14))))
            .addr;
        let c = |mem: &M, i: u32| mem.read::<u8>(CHR.ptr(mem).at(i).addr.wrapping_add(0xe));
        let jump = |mem: &M| {
            cmd.wrapping_add(i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned())
        };
        let kind = mem.read::<u16>(host.wrapping_add(0x21a)) >> 12;
        match kind {
            0 => {
                let mut v: u8 = mem.read(e.wrapping_add(0x10));
                if v == 0xff {
                    v = mem.read(e.wrapping_add(0x18));
                }
                mem.write(no, v);
                cmd.wrapping_add(2)
            }
            1 => {
                if mem.read::<u8>(host.wrapping_add(0x21e)) != 0 {
                    let v: u8 = mem.read(cmd.wrapping_add(2));
                    mem.write(no, v);
                }
                cmd.wrapping_add(4)
            }
            2 => {
                if mem.read::<u8>(host.wrapping_add(0x21e)) == 0 {
                    let n = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
                    let h = go(mem, 0x0086_aee0, &[n]);
                    CHR_HANDLES.ptr(mem).at(n).set(mem, h);
                }
                cmd.wrapping_add(4)
            }
            3 => {
                if crate::op::bridge::DEBUG_MODE.get(mem) != 0 {
                    let mut v: u8 = mem.read(cmd.wrapping_add(2));
                    if CHR_HANDLES.ptr(mem).at(u32::from(v)).get(mem) == 0 {
                        let h = go(mem, 0x007f_cfb0, &[0]);
                        let i = go(mem, 0x007f_e970, &[h, 0, 0]);
                        v = c(mem, i);
                    }
                    mem.write(no, v);
                }
                cmd.wrapping_add(4)
            }
            5 => {
                let n = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
                let i = go(mem, 0x007f_eaa0, &[n]);
                if i.cast_signed() >= 0 {
                    let v = c(mem, i);
                    mem.write(no, v);
                }
                cmd.wrapping_add(4)
            }
            6 => {
                let n = u32::from(mem.read::<u8>(no));
                let v = go(mem, 0x0078_8e10, &[n]).to_le_bytes()[0];
                mem.write(no, v);
                if v < 0x20 {
                    cmd.wrapping_add(4)
                } else {
                    jump(mem)
                }
            }
            8 => {
                let v: u8 = mem.read(e.wrapping_add(0x18));
                mem.write(no, v);
                cmd.wrapping_add(2)
            }
            9 | 10 => {
                let n = u32::from(mem.read::<u8>(no));
                let w = go(mem, 0x0079_5b40, &[n, u32::from(kind == 10)]);
                if w == 0 {
                    return jump(mem);
                }
                let i = go(mem, 0x007f_e970, &[w, 0, 0]);
                if i.cast_signed() >= 0 {
                    let v = c(mem, i);
                    mem.write(no, v);
                }
                cmd.wrapping_add(4)
            }
            _ => cmd,
        }
    }

    /// `0x00804840` (`opu_matrix_chara`, ours; opus table `0x00c48f30`),
    /// `(host, cmd)` -> `cmd + 2` (`cmd` for nibbles 3 and 8..): the host
    /// matrix from the unit's character `h` (`g_EtChrHandles[unit +0x18]`,
    /// whose `Ch_GetDataScale` is read and dropped). `p` = the parent
    /// (`opus_parent_get`) with flag 0x800, else the unit. Nibbles 2 and 5
    /// take the parent and set the flags to 0x800 / 0x4800; with 0 and 4
    /// they build `opus_rt_m_chara(host, c, f32 unit +0x3c, 0, 0, i8 host
    /// +0x21e)` turned by unit `+0x30` (`op_rot_m` with flag 0x4000, else
    /// `op_rt_m` moved by `+0x90`); 7 scales by `unit +0x3c/+0x4c / 256 /
    /// 1000` (x and z alike, `op_unit_m_scale2`) and `op_rt_m`s. Those
    /// end with unit `+0xc0` = `p x host` (flag 0x800) or a copy, and
    /// `Ch_SetOrgMatrix(h, unit +0xc0)`, `Ch_EnClip(h, 0)`. Nibble 1:
    /// `Ch_SetOrgMatrix(h, 0)`, `Ch_EnClip(h, 1)`. Nibble 6: the rotation
    /// at host `+0x100` turned into host (`op_rt_m`, times `p` with 0x800),
    /// host `+0x80` = the base matrix of `h`'s element 1
    /// (`Ch_OptposGetElement`, `Ch_ElemGetBaseMatrix`), host `+0xb0` =
    /// that point (`op_apply_m_w1`); unless `opus_chara_center_check(c,
    /// host +0xb0)` (the second word is the stack's leftover), host `+0x50`
    /// / `+0x40` = that point through host `+0x100` / host, and host `+0x30`
    /// pulled half way back from `+0x40` (x and z; y plus half of `+0x54`);
    /// unit `+0xc0` = host, and `h` set up as above. Without `h` the
    /// character calls are skipped. Callees through `call`
    /// (`Ch_GetDataScale` through `fcall`); `local` is the original's
    /// 16-byte vector.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_matrix_chara<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        // the image's f64s: 1/256, 0.001 (as an f32) and 0.5
        let konst = |mem: &M, a: u32| mem.read::<f64>(g(mem, a));
        let (k256, k1000, half) = (
            konst(mem, 0x00b5_0198),
            konst(mem, 0x00b4_7c68),
            konst(mem, 0x00b9_2220),
        );
        let u = unit(mem, host);
        let c = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(c).get(mem);
        let at = |k: u32| host.wrapping_add(k);
        let ua = |k: u32| u.wrapping_add(k);
        let f = g(mem, 0x0082_ab60);
        fcall(mem, f, &[h]);
        let flags = |mem: &M| mem.read::<u16>(at(0x21a));
        let mut parent = if flags(mem) & 0x800 != 0 {
            go(mem, 0x0080_1f00, &[host, u])
        } else {
            u
        };
        // opus_rt_m_chara on `m` from the unit's scale word
        let chara = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, m: u32| {
            let battle = i32::from(mem.read::<i8>(at(0x21e))).cast_unsigned();
            let k = store(load(mem.read(ua(0x3c))));
            go(mem, 0x0080_62c0, &[m, c, k, 0, 0, battle]);
        };
        let set_up = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, m: u32| {
            if h != 0 {
                go(mem, 0x0082_b3e0, &[h, m]);
                go(mem, 0x0082_aa30, &[h, 0]);
            }
        };
        let rt_m = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, src: u32| {
            go(mem, 0x007e_9850, &[host, src, ua(0x30), ua(0x90)]);
        };
        match flags(mem) & 0xf000 {
            0x1000 => {
                if h != 0 {
                    go(mem, 0x0082_b3e0, &[h, 0]);
                    go(mem, 0x0082_aa30, &[h, 1]);
                }
                return cmd.wrapping_add(2);
            }
            n @ (0 | 0x4000 | 0x2000 | 0x5000) => {
                if n == 0x2000 || n == 0x5000 {
                    parent = go(mem, 0x0080_1f00, &[host, u]);
                    let to: u16 = if n == 0x2000 { 0x800 } else { 0x4800 };
                    mem.write(at(0x21a), to);
                }
                chara(mem, &mut go, host);
                if flags(mem) & 0x4000 != 0 {
                    go(mem, 0x007e_9530, &[host, host, ua(0x30)]);
                } else {
                    rt_m(mem, &mut go, host);
                }
            }
            0x7000 => {
                let k = mul(mul(load(mem.read(ua(0x3c))), k256), k1000);
                let xz = store(k);
                mem.write(at(0x48), xz);
                mem.write(at(0x40), xz);
                let y = mul(k1000, mul(k256, load(mem.read(ua(0x4c)))));
                mem.write(at(0x44), store(y));
                go(mem, 0x007e_a730, &[host, at(0x40)]);
                rt_m(mem, &mut go, host);
            }
            0x6000 => {
                chara(mem, &mut go, at(0x100));
                rt_m(mem, &mut go, at(0x100));
                if flags(mem) & 0x800 != 0 {
                    go(mem, 0x007e_83b0, &[host, parent, host]);
                }
                let e = go(mem, 0x0083_5660, &[h, 1, local]);
                go(mem, 0x0083_3b80, &[h, e, at(0x80)]);
                go(mem, 0x007e_79a0, &[at(0xb0), at(0x80), local]);
                if go(mem, 0x0080_0b40, &[c, at(0xb0)]) == 0 {
                    go(mem, 0x007e_79a0, &[at(0x50), at(0x100), at(0xb0)]);
                    go(mem, 0x007e_79a0, &[at(0x40), host, at(0xb0)]);
                    let r = |mem: &M, k: u32| load(mem.read(at(k)));
                    // each pulled back by half its distance to the point
                    let back = |a: f64, b: f64| sub(a, mul(sub(b, a), half));
                    let x = back(r(mem, 0x30), r(mem, 0x40));
                    mem.write(at(0x30), store(x));
                    let z = back(r(mem, 0x38), r(mem, 0x48));
                    mem.write(at(0x38), store(z));
                    let y = load(store(back(r(mem, 0x34), r(mem, 0x44))));
                    let y = add(mul(half, r(mem, 0x54)), y);
                    mem.write(at(0x34), store(y));
                }
                go(mem, 0x007e_8010, &[ua(0xc0), host]);
                set_up(mem, &mut go, ua(0xc0));
                return cmd.wrapping_add(2);
            }
            _ => return cmd,
        }
        if flags(mem) & 0x800 != 0 {
            go(mem, 0x007e_83b0, &[ua(0xc0), parent, host]);
        } else {
            go(mem, 0x007e_8010, &[ua(0xc0), host]);
        }
        set_up(mem, &mut go, ua(0xc0));
        cmd.wrapping_add(2)
    }

    /// `opus_encount4_init(enc, oef, a, b, conf)` (`0x007f8930`), from
    /// `opu_encount`: sets up the broken-screen encounter. `enc+0` = `i16
    /// conf+8` (the mode), `u16 +4`/`+6` (first record, block count) from
    /// `conf+4`/`+6`, the bounds `+0xe0..+0xe8` = -65536 and `+0xf0..+0xf8`
    /// = 65536, `+0x100` = 0, `+0x10`/`+0x14` = `a`/`b`. The old buffer
    /// `0x012e962c` is freed (`user_free_thunk`), `0x012e9634..+0x3c` = 0,
    /// and `opus_0706_acc_draw(enc, oef, &size)` gives the new one; the
    /// renderer's block buffer size starts at `0x00639450(count, buffer)`.
    /// Each block (0x70 bytes from `+0x110`; dword `+0` = 1) takes record
    /// `op_oef2_a_get(oef, 4, first + i)` (`+4`): its `i16 +0x12` vertices
    /// (i16 triples at `+[+8]`) give the centre (`+0x10..+0x18`, `y` also
    /// at `+0x68`; the bounds track it, the maxima all compared with
    /// `+0xe0` as in the original), `op_deg_get(0, 0, x, z, +0x60)` the
    /// angle (`+0x64`; `+0x100` the largest `+0x60`), the vertices less
    /// the centre (`w` 1, at the shared cursor from the block area's end,
    /// `+8`), each with `graphicBrokenScreenUpdatePolygonData(i, 1, j, 0
    /// x6, &u, &v)` (`u`/`v` = x/z / 14000 / 2 + 0.5); its `i16 +0x14`
    /// normals (`+[+0xc]`, / 4096, `+0xc`); three copies of the emulated
    /// GPR `r0` (`+0x20..+0x4c`) and three `op_spline_draw_path3(16, 0.5)`
    /// (`+0x50..+0x58`, through `fcall`). When the block size ends past
    /// the buffer plus `size`, `fprintf(stderr, "Not Enough buffer size
    /// ...", size)` through the CRT imports. Then `0x00639370` and
    /// `0x012e9630` = 1; returns 0. `local` is the original's `u`, `v`,
    /// `size` (3 words).
    #[allow(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::similar_names
    )]
    pub fn opus_encount4_init<M: Mem>(
        mem: &mut M,
        enc: u32,
        oef: u32,
        a: u32,
        b: u32,
        conf: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        let word = |mem: &M, at: u32| mem.read::<u32>(g(mem, at));
        let konst = |mem: &M, at: u32| mem.read::<f64>(g(mem, at));
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let e = |k: u32| enc.wrapping_add(k);
        let (u, v, size_out) = (local, local.wrapping_add(4), local.wrapping_add(8));
        mem.write(size_out, 0u32);
        let mode = i32::from(mem.read::<i16>(conf.wrapping_add(8)));
        mem.write(enc, mode);
        let first = mem.read::<u16>(conf.wrapping_add(4));
        mem.write(e(4), first);
        let count = mem.read::<u16>(conf.wrapping_add(6));
        mem.write(e(6), count);
        let low = word(mem, 0x00b5_83f0);
        mem.write(e(0xe0), [low; 3]);
        let high = word(mem, 0x00b4_3ddc);
        mem.write(e(0xf0), [high; 3]);
        mem.write(e(0x100), 0u32);
        mem.write(e(0x10), a);
        mem.write(e(0x14), b);
        let count = u32::from(mem.read::<u16>(e(6)));
        let mut out = e(0x110 + 0x70 * count);
        let buffer = g(mem, 0x012e_962c);
        let old: u32 = mem.read(buffer);
        if old != 0 {
            go(mem, 0x0068_7010, &[old]);
            mem.write(buffer, 0u32);
        }
        mem.write(g(mem, 0x012e_9634), [0u32; 3]);
        let made = go(mem, 0x007f_7360, &[enc, oef, size_out]);
        mem.write(buffer, made);
        let n = u32::from(mem.read::<u16>(e(6)));
        let mut blocks = go(mem, 0x0063_9450, &[n, made]);
        let mut idx = u32::from(mem.read::<u16>(e(4)));
        let k14000 = konst(mem, 0x00b5_83d0);
        let half = konst(mem, 0x00b9_2220);
        let k4096 = konst(mem, 0x00b5_0190);
        let mut i = 0u32;
        while i < u32::from(mem.read::<u16>(e(6))) {
            let blk = e(0x110 + 0x70 * i);
            let at = |k: u32| blk.wrapping_add(k);
            mem.write(blk, 1u32);
            let rec = go(mem, 0x007e_3810, &[oef, 4, idx]);
            mem.write(at(4), rec);
            let nv16 = mem.read::<i16>(rec.wrapping_add(0x12));
            let verts = rec.wrapping_add(mem.read(rec.wrapping_add(8)));
            let nv = i32::from(nv16);
            blocks = go(mem, 0x0063_9470, &[i, nv.cast_unsigned(), blocks]);
            let (mut sx, mut sy, mut sz) = (0u32, 0u32, 0u32);
            let short = |mem: &M, at: u32| f64::from(mem.read::<i16>(at));
            for j in 0..nv.max(0).cast_unsigned() {
                let p = verts.wrapping_add(6 * j);
                sx = store(add(short(mem, p), load(sx)));
                sy = store(add(short(mem, p.wrapping_add(2)), load(sy)));
                sz = store(add(load(sz), short(mem, p.wrapping_add(4))));
            }
            let nf = f64::from(nv);
            let cx = store(div(load(sx), nf));
            let cy = store(div(load(sy), nf));
            let cz = store(div(load(sz), nf));
            mem.write(at(0x10), cx);
            mem.write(at(0x68), cy);
            mem.write(at(0x14), cy);
            mem.write(at(0x18), cz);
            let bound = |mem: &M, k: u32| load(mem.read(e(k)));
            for (k, c) in [(0xe0u32, cx), (0xe4, cy), (0xe8, cz)] {
                if bound(mem, 0xe0) < load(c) {
                    mem.write(e(k), c);
                }
            }
            for (k, c) in [(0xf0u32, cx), (0xf4, cy), (0xf8, cz)] {
                if bound(mem, 0xf0) > load(c) {
                    mem.write(e(k), c);
                }
            }
            let f = g(mem, 0x007e_7ed0);
            let deg = fcall(mem, f, &[0, 0, cx, cz, at(0x60)]);
            mem.write(at(0x64), store(deg));
            let len = load(mem.read(at(0x60)));
            if bound(mem, 0x100) < len {
                mem.write(e(0x100), store(len));
            }
            let verts = rec.wrapping_add(mem.read(rec.wrapping_add(8)));
            mem.write(at(8), out);
            let mut j = 0i32;
            while j < i32::from(mem.read::<i16>(rec.wrapping_add(0x12))) {
                let p = verts.wrapping_add(6 * j.cast_unsigned());
                let (x, y, z) = (
                    short(mem, p),
                    short(mem, p.wrapping_add(2)),
                    short(mem, p.wrapping_add(4)),
                );
                mem.write(out, store(sub(x, load(cx))));
                mem.write(out.wrapping_add(4), store(sub(y, load(cy))));
                mem.write(out.wrapping_add(8), store(sub(z, load(cz))));
                mem.write(out.wrapping_add(0xc), 1.0f32.to_bits());
                mem.write(u, store(add(mul(div(x, k14000), half), half)));
                mem.write(v, store(add(half, mul(div(z, k14000), half))));
                let args = [i, 1, j.cast_unsigned(), 0, 0, 0, 0, 0, 0, u, v];
                go(mem, 0x0063_9500, &args);
                out = out.wrapping_add(0x10);
                j += 1;
            }
            let normals = rec.wrapping_add(mem.read(rec.wrapping_add(0xc)));
            mem.write(at(0xc), out);
            let mut j = 0i32;
            while j < i32::from(mem.read::<i16>(rec.wrapping_add(0x14))) {
                let p = normals.wrapping_add(6 * j.cast_unsigned());
                for k in 0..3u32 {
                    let n = short(mem, p.wrapping_add(2 * k));
                    mem.write(out.wrapping_add(4 * k), store(mul(n, k4096)));
                }
                mem.write(out.wrapping_add(0xc), 1.0f32.to_bits());
                out = out.wrapping_add(0x10);
                j += 1;
            }
            let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
            mem.write(at(0x20), [r0; 3]);
            let spline = g(mem, 0x007e_67f0);
            let args = [word(mem, 0x00b9_231c), word(mem, 0x00b9_2194)];
            for k in 0..3u32 {
                let r = fcall(mem, spline, &args);
                mem.write(at(0x50 + 4 * k), store(r));
            }
            i += 1;
            idx = idx.wrapping_add(1);
        }
        let size: u32 = mem.read(size_out);
        if blocks > mem.read::<u32>(buffer).wrapping_add(size) {
            let iob = word(mem, 0x00b0_c350);
            let stderr = call(mem, iob, &[]).wrapping_add(0x40);
            let fprintf = word(mem, 0x00b0_c38c);
            let format = g(mem, 0x00b5_8358);
            call(mem, fprintf, &[stderr, format, size]);
        }
        let f = g(mem, 0x0063_9370);
        call(mem, f, &[]);
        mem.write(g(mem, 0x012e_9630), 1u32);
        0
    }

    /// `opus_encount4_calc(enc, host, _, eff, scale)` (`0x007f8570`), from
    /// `opu_encount`: one frame of the "broken screen" encounter blocks
    /// (`u16 enc+6` of them, 0x70-byte records from `enc+0x110`; byte `+0`
    /// alive, bit 0 of `+1` falling). Unless `eff`'s byte `+0xb0` bit 0 is
    /// clear or `host`'s `i16 +0x212` is set, the band between `a` =
    /// `f32(enc+0xf0 x 1.2)` and `b` = `f32(enc+0xe0 x 1.2)` is narrowed
    /// by `scale+0x40 x f32(b - a) / 256` (mode `enc+0` 0/1 moves `b` up
    /// from `a`, 2/3 moves `a` down from `b`), and each alive, resting
    /// block whose `+0x10` lies in `[a, b]` starts falling: its velocity
    /// `+0x40` = (∓48 for modes 0/2 and 1/3) + `op_random(16)`, `+0x34` =
    /// -512 + `op_random(128)`, `+0x38` = `op_random(64)` (through
    /// `fcall`). Each falling block counts `u16 +2` up, its velocity
    /// `+0x30..+0x38` gains `+0x40..+0x48` and its offset `+0x20..+0x28`
    /// the new velocity. Then every block's position: `+0x10`/`+0x18` =
    /// the sine table at `ftol(+0x64)` (cosine and sine) x `+0x60` x
    /// `scale+0x3c` / 4096 plus `+0x20`/`+0x28`, `+0x14` = `+0x68 +
    /// +0x24`; a block whose `+0x10` leaves ±20000 dies. Over 100 alive
    /// blocks `graphicBrokenScreenStartKeepFrame`, else `0x00639390`; then
    /// `graphicBrokenScreenUpdateOpacity((n - 60) / 46)` over 60 (else 0)
    /// and, after a fall this frame, `Sg_Blur` (1 once when `0x012e9630` is
    /// positive, clearing it; else `n`, or `max(2n - 60, 0)` for 60 or
    /// fewer). Returns the alive bytes or-ed together: `0x00639360` when
    /// nonzero, else `Sg_Blur(0)`.
    #[allow(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::many_single_char_names
    )]
    pub fn opus_encount4_calc<M: Mem>(
        mem: &mut M,
        enc: u32,
        host: u32,
        eff: u32,
        scale: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
        let konst = |mem: &M, a: u32| mem.read::<f64>(g(mem, a));
        let word = |mem: &M, a: u32| mem.read::<u32>(g(mem, a));
        let count = |mem: &M| u32::from(mem.read::<u16>(enc.wrapping_add(6)));
        let block = |i: u32| enc.wrapping_add(0x110 + 0x70 * i);
        let rnd = g(mem, 0x007e_6700);
        let mut fell = false;
        if mem.read::<u8>(eff.wrapping_add(0xb0)) & 1 != 0
            && mem.read::<i16>(host.wrapping_add(0x212)) == 0
        {
            let k12 = konst(mem, 0x00b4_8340);
            let k256 = konst(mem, 0x00b5_0198);
            let mut a = store(mul(load(mem.read(enc.wrapping_add(0xf0))), k12));
            let mut b = store(mul(k12, load(mem.read(enc.wrapping_add(0xe0)))));
            let diff = load(store(sub(load(b), load(a))));
            let mode: u32 = mem.read(enc);
            let c = |mem: &M| load(mem.read(scale.wrapping_add(0x40)));
            if mode < 2 {
                b = store(add(mul(mul(c(mem), diff), k256), load(a)));
            }
            if mode == 2 || mode == 3 {
                a = store(sub(load(b), mul(k256, mul(diff, c(mem)))));
            }
            let (lo, hi) = (load(a), load(b));
            let mut i = 0u32;
            while i < count(mem) {
                let r = block(i);
                let at = |k: u32| r.wrapping_add(k);
                if mem.read::<u8>(r) != 0 {
                    let flags = mem.read::<u8>(at(1));
                    let v = load(mem.read(at(0x10)));
                    if flags & 1 == 0 && lo <= v && v <= hi {
                        let mode: u32 = mem.read(enc);
                        if mode <= 3 {
                            let side = if mode & 1 == 0 {
                                0x00b5_83e4
                            } else {
                                0x00b5_83d8
                            };
                            mem.write(at(0x40), word(mem, side));
                            mem.write(at(1), flags | 1);
                            let x = fcall(mem, rnd, &[word(mem, 0x00b9_231c)]);
                            let vx = store(add(x, load(mem.read(at(0x40)))));
                            mem.write(at(0x40), vx);
                            mem.write(at(0x34), word(mem, 0x00b5_83e8));
                            let y = fcall(mem, rnd, &[word(mem, 0x00b5_01bc)]);
                            let vy = store(add(y, load(mem.read(at(0x34)))));
                            mem.write(at(0x34), vy);
                            let z = fcall(mem, rnd, &[word(mem, 0x00b5_83dc)]);
                            mem.write(at(0x38), store(z));
                        }
                    }
                    if mem.read::<u8>(at(1)) & 1 != 0 {
                        let n = mem.read::<u16>(at(2)).wrapping_add(1);
                        mem.write(at(2), n);
                        fell = true;
                        for k in 0..3u32 {
                            let acc = load(mem.read(at(0x40 + 4 * k)));
                            let vel = store(add(acc, load(mem.read(at(0x30 + 4 * k)))));
                            mem.write(at(0x30 + 4 * k), vel);
                            let off = store(add(load(vel), load(mem.read(at(0x20 + 4 * k)))));
                            mem.write(at(0x20 + 4 * k), off);
                        }
                    }
                }
                i += 1;
            }
        }
        let k4096 = konst(mem, 0x00b5_0190);
        let (far, near) = (load(word(mem, 0x00b4_80d8)), load(word(mem, 0x00b5_83ec)));
        let (cos_at, sin_at) = (g(mem, 0x00c4_3bf0), g(mem, 0x00c4_4bf0));
        let mut i = 0u32;
        while i < count(mem) {
            let r = block(i);
            let at = |k: u32| r.wrapping_add(k);
            let t = (ftol(mem, load(mem.read(at(0x64)))) & 0x3fff) + 0x400;
            let q = load(mem.read(scale.wrapping_add(0x3c)));
            let radius = |mem: &M| load(mem.read(at(0x60)));
            let c = store(mul(load(mem.read(cos_at.wrapping_add(4 * t))), radius(mem)));
            mem.write(at(0x10), store(mul(mul(load(c), q), k4096)));
            let s = load(mem.read(sin_at.wrapping_add(4 * (t & 0xfff))));
            let s = store(mul(s, radius(mem)));
            let q = load(mem.read(scale.wrapping_add(0x3c)));
            mem.write(at(0x18), store(mul(mul(load(s), q), k4096)));
            let x = store(add(load(mem.read(at(0x20))), load(mem.read(at(0x10)))));
            mem.write(at(0x10), x);
            let y = store(add(load(mem.read(at(0x68))), load(mem.read(at(0x24)))));
            mem.write(at(0x14), y);
            let z = store(add(load(mem.read(at(0x28))), load(mem.read(at(0x18)))));
            mem.write(at(0x18), z);
            let x = load(x);
            if x > far || x < near {
                mem.write(r, 0u8);
            }
            i += 1;
        }
        let mut alive = 0u32;
        let mut n = 0u32;
        for i in 0..count(mem) {
            let b = mem.read::<u8>(block(i));
            alive |= u32::from(b);
            if b != 0 {
                n += 1;
            }
        }
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        go(mem, if n > 100 { 0x0063_94a0 } else { 0x0063_9390 }, &[]);
        let blur_once = g(mem, 0x012e_9630);
        if n > 60 {
            let k60 = konst(mem, 0x00b4_a180);
            let k46 = konst(mem, 0x00b5_83b0);
            let nf = load(store(f64::from(n)));
            let o = store(div(sub(nf, k60), k46));
            go(mem, 0x0063_94e0, &[o]);
        } else {
            go(mem, 0x0063_94e0, &[0]);
        }
        if fell {
            if n <= 60 {
                go(mem, 0x0082_bee0, &[(2 * n).saturating_sub(60)]);
            } else if mem.read::<i32>(blur_once) > 0 {
                go(mem, 0x0082_bee0, &[1]);
                mem.write(blur_once, 0u32);
            } else {
                go(mem, 0x0082_bee0, &[n]);
            }
        }
        if alive == 0 {
            go(mem, 0x0082_bee0, &[0]);
        } else {
            go(mem, 0x0063_9360, &[]);
        }
        alive
    }

    /// `opus_camera_limit(host)` (`0x00817590`), from `opu_camera`: while
    /// `g_OpusCameraLimit` (`i16 0x011a006c`) is set, the battle camera's
    /// segment from its position (`MsCameraGetPosNow(1)`) to its target
    /// (`(0)`) is cut by the circle of radius 120 around the map centre
    /// (`MsGetMapCenter`, x/z; `op_segment_circle_cross`); on a crossing
    /// the camera position moves to the crossing (at the old height,
    /// `MsCameraSetRect(host +0x5c, 1, ..)`) and the screen depth
    /// (`MsCameraGetScrDpt`, through `fcall`) shrinks by `(limit - 0x1000)
    /// x |crossing - position| / 256` (`MsCameraSetScrDpt`). In op debug
    /// mode with pad byte `0x0232154b` set, `QuickRecoverMenuReserve(
    /// 0x80800000, 0x48, 0)`. `local` is the original's 0x50-byte frame:
    /// `+0` the second crossing, `+0x10` the centre, `+0x20` the target,
    /// `+0x30` the crossing, `+0x40` the position.
    pub fn opus_camera_limit<M: Mem>(
        mem: &mut M,
        host: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87::{load, mul, store, sub};
        let limit = |mem: &M| mem.read::<i16>(g(mem, 0x011a_006c));
        if limit(mem) == 0 {
            return;
        }
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let (cross2, centre, target, cross, pos) = (
            local,
            local.wrapping_add(0x10),
            local.wrapping_add(0x20),
            local.wrapping_add(0x30),
            local.wrapping_add(0x40),
        );
        go(mem, 0x0078_2ae0, &[centre]);
        go(mem, 0x007b_d470, &[1, pos]);
        go(mem, 0x007b_d470, &[0, target]);
        let get_depth = g(mem, 0x007b_d510);
        let depth = store(fcall(mem, get_depth, &[]));
        // fld/fstp of each float argument
        let word = |mem: &M, a: u32| store(load(mem.read(a)));
        let radius = word(mem, g(mem, 0x00b5_8cd8));
        let args = [
            word(mem, centre),
            word(mem, centre.wrapping_add(8)),
            radius,
            word(mem, pos),
            word(mem, pos.wrapping_add(8)),
            word(mem, target),
            word(mem, target.wrapping_add(4)),
            cross,
            cross2,
        ];
        if go(mem, 0x007e_8fd0, &args) == 0 {
            return;
        }
        let y = word(mem, pos.wrapping_add(4));
        mem.write(cross.wrapping_add(4), y);
        let id: u32 = mem.read(host.wrapping_add(0x5c));
        let length = g(mem, 0x007e_7fc0);
        let len = store(fcall(mem, length, &[pos, cross]));
        let k256: f64 = mem.read(g(mem, 0x00b5_0198));
        let n = store(f64::from(i32::from(limit(mem)) - 0x1000));
        let cut = store(mul(mul(load(n), load(len)), k256));
        go(mem, 0x007b_f940, &[id, 1, cross]);
        let d = store(sub(load(depth), load(cut)));
        go(mem, 0x007b_fbd0, &[id, d]);
        if mem.read::<u8>(g(mem, 0x0232_2790)) != 0 && mem.read::<u8>(g(mem, 0x0232_154b)) != 0 {
            go(mem, 0x007e_adb0, &[0x8080_0000, 0x48, 0]);
        }
    }

    /// `opu_tc_load(host, cmd)` (`0x008085a0`) -> the next command: the
    /// texture/CLUT cues of the unit's oef (`host +0x360 + 4 u8 unit
    /// +0x1e`), `n` = `u16 cmd +2`, the cue mask 8 with flag 0x800 (else
    /// 0x80000000). Nibble 0: `op_oef2_tex_clut_cue_multi(oef, n, n,
    /// mask)` (`+4`); 4: with `u16 cmd +4` as the last (`+6`); 5:
    /// `op_oef2_texture_load` / `op_oef2_clut_load` (with 1) of its kind 2
    /// / 3 record `i16 n` (`+4`); 6: `op_oef2_tex_clut_ovl_clear(mask)`
    /// (`+2`); 3: `op_oef2_tex_clut_cue_check(mask)` set: host byte
    /// `+0x210` = 1 and `cmd`, else `+2`. 1: the oef's monster slot
    /// (`opus_get_monster_index`) with no flags takes the first of the
    /// byte `0x00c48e60[i16 n]` and twice it that no slot uses
    /// (`g_OpusMonsterFlags`, 8 bytes), its two record pairs at
    /// `0x0119ff10`/`0x0119ff50` cleared; flags already set leave it
    /// (`+4`). 2: a slot without bit 7 whose texture and CLUT positions
    /// (`opus_monster_tex_pos`/`_clut_pos`, `u16`) are set: unless byte
    /// `+0x1f` of the kind 2 record `t`, `op_oef2_tex_clut_reset2(oef,
    /// *t & 0x3fff, *c, op_oef2_tex_pos_set(t, tex),
    /// op_oef2_clut_change_monster(c, clut))` (`c` the kind 3 record);
    /// `t`/`c` go into its first free pair (`+4`); bit 7 set: `+4`.
    /// Without a slot (or a free bit, or positions) it is
    /// `dbgPrintf` and `+4` in debug mode (`g_OpDebugMode`), else
    /// `dbgPrintf`, `debug_exit_trace(1)` and on as nibble 3. Others
    /// `cmd`. Callees through `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_tc_load<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let n = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        let wide = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let k = u32::from(mem.read::<u8>(unit(mem, host).wrapping_add(0x1e)));
        let oef: u32 = mem.read(host.wrapping_add(0x360).wrapping_add(4 * k));
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        let mask = if flags & 0x800 != 0 { 8 } else { 0x8000_0000 };
        let monster_flags = g(mem, 0x0119_fef0);
        let pairs = |mem: &M, slot: u32| {
            let at = |a: u32| g(mem, a).wrapping_add(slot.wrapping_mul(8));
            (at(0x0119_ff10), at(0x0119_ff50))
        };
        let found = match flags & 0xf000 {
            0 => {
                go(mem, 0x007e_4830, &[oef, n, n, mask]);
                return cmd.wrapping_add(4);
            }
            0x1000 => 'slot: {
                let slot = go(mem, 0x0080_0e80, &[oef]);
                if slot.cast_signed() < 0 {
                    break 'slot false;
                }
                let own = monster_flags.wrapping_add(slot);
                if mem.read::<u8>(own) != 0 {
                    return cmd.wrapping_add(4);
                }
                let used = (0..8).fold(0u8, |a, i| a | mem.read::<u8>(monster_flags + i));
                let mut bit: u8 = mem.read(g(mem, 0x00c4_8e60).wrapping_add(wide));
                for _ in 0..2 {
                    if used & bit == 0 {
                        mem.write(own, bit);
                        let (a, b) = pairs(mem, slot);
                        mem.write(a, [0u32; 2]);
                        mem.write(b, [0u32; 2]);
                        return cmd.wrapping_add(4);
                    }
                    bit = bit.wrapping_add(bit);
                }
                false
            }
            0x2000 => 'slot: {
                let slot = go(mem, 0x0080_0e80, &[oef]);
                if slot.cast_signed() < 0 {
                    break 'slot false;
                }
                if mem.read::<u8>(monster_flags.wrapping_add(slot)) & 0x80 != 0 {
                    return cmd.wrapping_add(4);
                }
                let tex = go(mem, 0x0080_0e50, &[slot]) & 0xffff;
                if tex == 0 {
                    break 'slot false;
                }
                let clut = go(mem, 0x0080_0e20, &[slot]) & 0xffff;
                if clut == 0 {
                    break 'slot false;
                }
                let t = go(mem, 0x007e_3810, &[oef, 2, wide]);
                let c = go(mem, 0x007e_3810, &[oef, 3, wide]);
                if mem.read::<u8>(t.wrapping_add(0x1f)) == 0 {
                    let pos = u32::from(mem.read::<u16>(t) & 0x3fff);
                    let new_tex = go(mem, 0x007e_4ff0, &[t, tex]) & 0xffff;
                    let first = u32::from(mem.read::<u16>(c));
                    let new_clut = go(mem, 0x007e_45e0, &[c, clut]) & 0xffff;
                    go(mem, 0x007e_3e70, &[oef, pos, first, new_tex, new_clut]);
                }
                let (a, b) = pairs(mem, slot);
                for i in 0..2u32 {
                    if mem.read::<u32>(a.wrapping_add(4 * i)) == 0 {
                        mem.write(a.wrapping_add(4 * i), t);
                        mem.write(b.wrapping_add(4 * i), c);
                        break;
                    }
                }
                return cmd.wrapping_add(4);
            }
            0x3000 => true,
            0x4000 => {
                let last = u32::from(mem.read::<u16>(cmd.wrapping_add(4)));
                go(mem, 0x007e_4830, &[oef, n, last, mask]);
                return cmd.wrapping_add(6);
            }
            0x5000 => {
                let t = go(mem, 0x007e_3810, &[oef, 2, wide]);
                go(mem, 0x007e_5030, &[t, 1]);
                let c = go(mem, 0x007e_3810, &[oef, 3, wide]);
                go(mem, 0x007e_46b0, &[c, 1]);
                return cmd.wrapping_add(4);
            }
            0x6000 => {
                go(mem, 0x007e_4e40, &[mask]);
                return cmd.wrapping_add(2);
            }
            _ => return cmd,
        };
        if !found {
            // no monster slot: a debug print, and in a release game the
            // debug exit (which returns here)
            if mem.read::<u8>(g(mem, 0x0232_2790)) != 0 {
                let text = g(mem, 0x00b5_88fc);
                go(mem, 0x0062_f500, &[text]);
                return cmd.wrapping_add(4);
            }
            let text = g(mem, 0x00b5_892c);
            go(mem, 0x0062_f500, &[text]);
            go(mem, 0x0062_f510, &[1]);
        }
        if go(mem, 0x007e_46d0, &[mask]) != 0 {
            mem.write(host.wrapping_add(0x210), 1u8);
            return cmd;
        }
        cmd.wrapping_add(2)
    }

    /// `opu_motion`'s motion change: `MsEffectSetMotion(c, m)` for `c` <
    /// 0x20, else with a handle `Ch_ResetMotion(h)`, `Ch_SetSysMotion(h, 1,
    /// m)` and `Ch_GetSeqFrame(h)`; the frame count either returns (None:
    /// neither ran).
    fn set_motion<M: Mem>(
        mem: &mut M,
        go: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        c: u32,
        h: u32,
        m: u32,
    ) -> Option<u32> {
        if c < 0x20 {
            Some(go(mem, 0x0078_82c0, &[c, m]))
        } else if h != 0 {
            go(mem, 0x0082_b080, &[h]);
            go(mem, 0x0083_7d00, &[h, 1, m]);
            Some(go(mem, 0x0083_6af0, &[h]))
        } else {
            None
        }
    }

    /// `0x0081c460` (`opu_kind2`, ours): the unit's draw kind `+0xbb` = 2,
    /// `+0xe0` = table `u16 cmd +2` of kind 4 of its oef (`op_oef2_a_get`,
    /// `set_ptr32`), `+0xc0` = 0x00480080 (two halves, 0x80 and 0x48: not relocated), byte `+0xa8`
    /// = flag bits 13..15, `+0xa9` = bits 9..11, and the host's unit number
    /// added to the list `host +0x300`. Returns `cmd + 4`.
    pub fn opu_kind2(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let n = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        let u = unit(mem, host);
        let k = u32::from(mem.read::<u8>(u.wrapping_add(0x1e)));
        let oef: u32 = mem.read(host.wrapping_add(0x360).wrapping_add(4 * k));
        let t = crate::op::bridge::op_oef2_a_get(mem, oef, 4, n);
        mem.write(u.wrapping_add(0xbb), 2u8);
        mem.write(u.wrapping_add(0xe0), ffx_system::stub::ptr32(t));
        // a constant, not an address: no relocation
        mem.write(u.wrapping_add(0xc0), 0x0048_0080u32);
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        #[allow(clippy::cast_possible_truncation)] // three bits
        mem.write(u.wrapping_add(0xa8), (flags >> 13 & 7) as u8);
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        #[allow(clippy::cast_possible_truncation)] // three bits
        mem.write(u.wrapping_add(0xa9), (flags >> 9 & 7) as u8);
        let id: u16 = mem.read(host.wrapping_add(0x214));
        crate::op::bridge::opu_id_list_add(mem, host.wrapping_add(0x300), id);
        cmd.wrapping_add(4)
    }

    /// `0x0080ad20` (`opu_prog_set`, ours): program slot `k` = flag bits
    /// 12..13 of the unit (`[unit +4 k]`); slot 0 takes the first null one
    /// of 1..3 (`is_ptr32_null`), none returning `cmd + 4` at once. The slot
    /// = `cmd + i16 cmd +2` (`set_ptr32`), the unit's byte `+0x1f` gets bit
    /// `k` with flag bit 15, its byte `+0x10 + k` = 0. Returns `cmd + 4`.
    pub fn opu_prog_set(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        let u = unit(mem, host);
        let mut k = u32::from(flags >> 12 & 3);
        if k == 0 {
            match (1..4)
                .find(|&i| ffx_system::stub::is_ptr32_null(mem.read(u.wrapping_add(4 * i))) != 0)
            {
                Some(i) => k = i,
                None => return cmd.wrapping_add(4),
            }
        }
        let to = i32::from(mem.read::<i16>(cmd.wrapping_add(2)))
            .cast_unsigned()
            .wrapping_add(cmd);
        mem.write(u.wrapping_add(4 * k), ffx_system::stub::ptr32(to));
        let bit = u8::from(flags & 0x8000 != 0) << k & 0xf;
        let at = u.wrapping_add(0x1f);
        mem.write(at, mem.read::<u8>(at) | bit);
        mem.write(u.wrapping_add(0x10 + k), 0u8);
        cmd.wrapping_add(4)
    }

    /// `0x0080b460` (`opu_osu_load_pos`, ours): without flag bits 8..11
    /// [`opu_osu_load`], then the eight `i16` at `d = cmd + i16 cmd +4` as
    /// floats into the host's object (`[host]`) `+0x30..+0x4c` (the load
    /// through `call`). Returns
    /// `cmd + 6`; with the bits `cmd`.
    pub fn opu_osu_load_pos<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf00 != 0 {
            return cmd;
        }
        call(mem, g(mem, 0x0080_ae30), &[host, cmd]);
        let d = i32::from(mem.read::<i16>(cmd.wrapping_add(4)))
            .cast_unsigned()
            .wrapping_add(cmd);
        let obj: u32 = mem.read(host);
        for k in 0..8 {
            let v = f32::from(mem.read::<i16>(d.wrapping_add(2 * k)));
            mem.write(obj.wrapping_add(0x30 + 4 * k), v);
        }
        cmd.wrapping_add(6)
    }

    /// `yonExtMapDeleteId(id)` (`0x00807830`), a bare `ret` here.
    pub const YON_EXT_MAP_DELETE_ID: u32 = 0x0080_7830;
    /// `yonExtMapRegister(id, struct)` (`0x00807860`), a bare `ret` here;
    /// the struct (`0x4c` bytes, 19 words) is passed by value.
    pub const YON_EXT_MAP_REGISTER: u32 = 0x0080_7860;
    /// The `sprintf` format `opu_unit_etc` copies into its struct
    /// (`"LoadedByOPU_UNIT_ETC"`, no conversions).
    const LOADED_BY: u32 = 0x00b5_8984;

    /// `opu_unit_etc(host, cmd)` (`0x0080b180`), by the flag nibble, with
    /// `u` the host's unit (`+0x220`), `list = host + 0x300`:
    /// - 0: `o = opus_no_get(list, u, u16 cmd +2)`; jumps (`cmd + i16 cmd
    ///   +4`) when `o` is not the first part (`[list +0x20]`) and not `u`,
    ///   or is the first part, `u16 cmd +2` is 0, `o` is not `u` and its
    ///   `u16 +0x1a` is 0; else `cmd + 6`.
    /// - 0x1000: the `i16` list `[list +0x10]` (to a negative one) is
    ///   searched for `n = u16((u - first part) >> 8)`; none: `cmd + 2`.
    ///   Found: [`opus_prog_off`](crate::op::bridge::opus_prog_off)`(list,
    ///   n)`, [`opus_list_add`]`(list, n, 1)`, `yonExtMapDeleteId(n)` and
    ///   `yonExtMapRegister(n, s)` with `s` a 0x4c-byte struct passed by
    ///   value: words 0 0, 1 `u`, 2 `list`, 3 `n`, 4 `junk[0]`, 5 0, then
    ///   0x34 bytes of `sprintf` buffer holding `"LoadedByOPU_UNIT_ETC"`
    ///   (21 bytes with the NUL); the rest of the buffer is never written:
    ///   the three bytes after the NUL are `junk[1] >> 8`, the last seven
    ///   words `junk[2..9]` (the original's uninitialised frame). Then the
    ///   host's `u16 +0x218` down by one, `+0x216` 0, byte `+0x210` 1;
    ///   `cmd + 2`.
    /// - 0x2000: `u16 [opus_no_get(list, u, u16 cmd +2) + i16 cmd +4]` =
    ///   `u16 host +0x214`; `cmd + 6`.
    /// - 0x3000: `i = i16 [u + i16 cmd +2]`; nonzero: with `k = cmd +4 &
    ///   3` and `e = (i << 8) + [host +0x320]`, `[e + 4k]` =
    ///   `set_ptr32(cmd + i16 cmd +6 + ([e + 4k] & 0x80000000))` and byte
    ///   `e + 0x10 + k` 1; `cmd + 8`.
    /// - 0x4000: `op_func_before_ot_set` / `after_ot3d` / `before_wot` /
    ///   `final` for `u16 cmd +2` 0..3 with `([[host +0x350] +0x74], 0, 0,
    ///   1)`; `cmd + 4` (also for other values).
    /// - 0x5000: `op_wot_triger(flags & 0x800)`; `cmd + 2`.
    /// - Anything else: `cmd`.
    ///
    /// The yonExtMap calls go through `call`, `yonExtMapRegister` with 20
    /// words (the hook passes them with `call_cdecl_words`).
    #[allow(clippy::too_many_lines)]
    pub fn opu_unit_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        junk: [u32; 9],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        let u = unit(mem, host);
        let list = host.wrapping_add(0x300);
        let first: u32 = mem.read(list.wrapping_add(0x20));
        let half = |mem: &M, at: u32| i32::from(mem.read::<i16>(cmd.wrapping_add(at)));
        match flags & 0xf000 {
            0 => {
                let no = mem.read::<u16>(cmd.wrapping_add(2));
                let o = opus_no_get(mem, list, u, u32::from(no));
                let jump = if o == first {
                    no == 0 && u != o && mem.read::<u16>(o.wrapping_add(0x1a)) == 0
                } else {
                    u != o
                };
                if jump {
                    cmd.wrapping_add(half(mem, 4).cast_unsigned())
                } else {
                    cmd.wrapping_add(6)
                }
            }
            0x1000 => {
                #[allow(clippy::cast_possible_truncation)] // the low half
                let want = (u.wrapping_sub(first) >> 8) as u16;
                let mut at: u32 = mem.read(list.wrapping_add(0x10));
                let n = loop {
                    let v: u16 = mem.read(at);
                    if v & 0x8000 != 0 {
                        return cmd.wrapping_add(2);
                    }
                    if v == want {
                        break v;
                    }
                    at = at.wrapping_add(2);
                };
                crate::op::bridge::opus_prog_off(mem, list, u32::from(n), call);
                opus_list_add(mem, list, n, 1);
                let id = u32::from(n);
                call(mem, g(mem, YON_EXT_MAP_DELETE_ID), &[id]);
                let mut s = [0u32; 20];
                s[0] = id;
                s[1..7].copy_from_slice(&[0, u, list, id, junk[0], 0]);
                // the sprintf buffer, words 6..18 of the struct
                let mut buf = [0u8; 0x34];
                buf[0x14..0x18].copy_from_slice(&junk[1].to_le_bytes());
                for (k, w) in junk[2..].iter().enumerate() {
                    buf[0x18 + 4 * k..0x1c + 4 * k].copy_from_slice(&w.to_le_bytes());
                }
                let fmt = g(mem, LOADED_BY);
                for (k, b) in (0u32..).zip(buf.iter_mut()) {
                    *b = mem.read(fmt.wrapping_add(k));
                    if *b == 0 {
                        break;
                    }
                }
                for (k, w) in buf.as_chunks::<4>().0.iter().enumerate() {
                    s[7 + k] = u32::from_le_bytes(*w);
                }
                call(mem, g(mem, YON_EXT_MAP_REGISTER), &s);
                let left = host.wrapping_add(0x218);
                mem.write(left, mem.read::<u16>(left).wrapping_sub(1));
                mem.write(host.wrapping_add(0x216), 0u16);
                mem.write(host.wrapping_add(0x210), 1u8);
                cmd.wrapping_add(2)
            }
            0x2000 => {
                let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
                let o = opus_no_get(mem, list, u, no);
                let id: u16 = mem.read(host.wrapping_add(0x214));
                mem.write(o.wrapping_add(half(mem, 4).cast_unsigned()), id);
                cmd.wrapping_add(6)
            }
            0x3000 => {
                let i = mem.read::<i16>(u.wrapping_add(half(mem, 2).cast_unsigned()));
                if i != 0 {
                    let k = u32::from(mem.read::<u16>(cmd.wrapping_add(4)) & 3);
                    let base: u32 = mem.read(host.wrapping_add(0x320));
                    let e = (i32::from(i) << 8).cast_unsigned().wrapping_add(base);
                    let slot = e.wrapping_add(4 * k);
                    let to = (mem.read::<u32>(slot) & 0x8000_0000)
                        .wrapping_add(cmd)
                        .wrapping_add(half(mem, 6).cast_unsigned());
                    mem.write(slot, ffx_system::stub::ptr32(to));
                    mem.write(e.wrapping_add(0x10 + k), 1u8);
                }
                cmd.wrapping_add(8)
            }
            0x4000 => {
                use crate::op::bridge as op;
                let lists: u32 = mem.read(list.wrapping_add(0x50));
                let func: u32 = mem.read(lists.wrapping_add(0x74));
                match mem.read::<u16>(cmd.wrapping_add(2)) {
                    0 => _ = op::op_func_before_ot_set(mem, func, 0, 0, 1),
                    1 => _ = op::op_func_after_ot3d_set(mem, func, 0, 0, 1),
                    2 => _ = op::op_func_before_wot_set(mem, func, 0, 0, 1),
                    3 => _ = op::op_func_final_set(mem, func, 0, 0, 1),
                    _ => {}
                }
                cmd.wrapping_add(4)
            }
            0x5000 => {
                crate::op::bridge::op_wot_triger(mem, u32::from(flags & 0x800));
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `op_part_run(part)` (`0x008008b0`): the op environment's tables from
    /// the ppp part data (`[part +0xd0]`, as [`op_part_fp_init`]); for a
    /// loaded one `pppRunPart([part +0xbc], 0x30)` (through `call`): when
    /// that is done (not 0), its entry in `0x012e9c70` cleared and
    /// `pppAllFreePObject`. Returns `pppRunPart`'s result, or 0 (an unloaded
    /// one prints a message, `dbgPrintf`).
    pub fn op_part_run<M: Mem>(
        mem: &mut M,
        part: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let data: u32 = mem.read(part.wrapping_add(0xd0));
        let ppp: u32 = mem.read(part.wrapping_add(0xbc));
        for k in 0..3 {
            let v = ffx_system::stub::ptr32(mem.read(data.wrapping_add(0x14 + 4 * k)));
            mem.write(g(mem, 0x012f_2090).wrapping_add(4 * k), v);
        }
        op_env(mem);
        if mem.read::<u8>(data) == 0 {
            call(mem, g(mem, 0x0062_f500), &[g(mem, 0x00b5_8778)]);
            return 0;
        }
        op_env(mem);
        let done = call(mem, g(mem, 0x0071_2250), &[ppp, 0x30]);
        if done != 0 {
            let i = u32::from(mem.read::<u16>(ppp.wrapping_add(0x1e)));
            mem.write(g(mem, 0x012e_9c70).wrapping_add(4 * i), 0u32);
            op_env(mem);
            crate::part::bridge::ppp_all_free_pobject(mem, ppp, call);
        }
        done
    }

    /// `opu_fin_type(host, cmd)` (`0x008167b0`): no flag bits: the unit's
    /// `+0x48` = the direction (`op_deg_get(x, z, other x, other z, unit
    /// +0x40)`, which also stores the distance at `+0x40`) to unit `u16 cmd
    /// +2` ([`opus_no_get`]), `cmd + 4`; nibble 1: byte `+0xba` = 0 and
    /// `+0x40 = op_xyz_scale(+0x90, 16)`, `cmd + 4`; else `cmd`. The unit's
    /// byte `+0xb2` loses bit 2 in every case.
    pub fn opu_fin_type<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        atan2: &mut impl FnMut(&mut M, f64, f64) -> f64,
    ) -> u32 {
        use crate::x87::load_f32;
        let u = unit(mem, host);
        let next = match nibble(mem, host) {
            0 => {
                let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
                let other = opus_no_get(mem, host.wrapping_add(0x300), u, no);
                let rd = |mem: &M, at: u32| load_f32(mem.read(at));
                let at = u.wrapping_add(0x40);
                let p = [
                    rd(mem, at),
                    rd(mem, u.wrapping_add(0x48)),
                    rd(mem, other.wrapping_add(0x40)),
                    rd(mem, other.wrapping_add(0x48)),
                ];
                let d = crate::op::bridge::op_deg_get(mem, p, at, atan2);
                mem.write(u.wrapping_add(0x48), d);
                cmd.wrapping_add(4)
            }
            0x1000 => {
                let s: f32 = mem.read(g(mem, 0x00b9_231c));
                mem.write(u.wrapping_add(0xba), 0u8);
                crate::op::bridge::op_xyz_scale(
                    mem,
                    u.wrapping_add(0x40),
                    u.wrapping_add(0x90),
                    s.to_bits(),
                );
                cmd.wrapping_add(4)
            }
            _ => cmd,
        };
        let b = u.wrapping_add(0xb2);
        mem.write(b, mem.read::<u8>(b) & 0xfb);
        next
    }

    /// `opu_en_etc(host, cmd)` (`0x00815f60`): by the command's `i16 +2`:
    /// 0 sets the wind of the unit's character (`g_EtChrHandles[byte unit
    /// +0x18]`): `Ch_WindSet(h, +0x30 / 256, angle, +0x38 +
    /// op_random(10))` with `angle` = `((ftol(+0x34) + 0x400) & 0xfff) /
    /// 4096 * 360 - 90` (`f32` each); 1 `Ch_WindSetEnable(h, 0)`; both
    /// `cmd + 4` (callees through `call`). Other values return `cmd`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_en_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{MUL, Reg, SUB};
        let u = unit(mem, host);
        let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        match mem.read::<i16>(cmd.wrapping_add(2)) {
            0 => {
                let rd = |mem: &M, at: u32| mem.read::<f32>(u.wrapping_add(at));
                let k = |mem: &M, at: u32| mem.read::<f64>(g(mem, at));
                let turn = ffx_bridge::x87::ftol(mem, Reg::load_f32(rd(mem, 0x34)).value());
                #[allow(clippy::cast_precision_loss)] // below 4096: fild is exact
                let t = (turn.wrapping_add(0x400) & 0xfff) as f32;
                let angle = Reg::load_f32(t)
                    .constant(MUL, k(mem, 0x00b5_0190))
                    .constant(MUL, k(mem, 0x00b4_8380))
                    .constant(SUB, k(mem, 0x00b5_8cd0))
                    .store();
                let w = crate::x87::load_f32(rd(mem, 0x38));
                let ten: f32 = mem.read(g(mem, 0x00b9_2318));
                let r = crate::rand::bridge::op_random(mem, ten);
                let b = Reg::load_f32(r).mem(crate::x87::ADD, w).store();
                let s = Reg::load_f32(rd(mem, 0x30))
                    .constant(MUL, k(mem, 0x00b5_0198))
                    .store();
                call(
                    mem,
                    g(mem, 0x0082_4c90),
                    &[h, s.to_bits(), angle.to_bits(), b.to_bits()],
                );
                cmd.wrapping_add(4)
            }
            1 => {
                call(mem, g(mem, 0x0082_4cc0), &[h, 0]);
                cmd.wrapping_add(4)
            }
            _ => cmd,
        }
    }

    /// The flag word an [`opu_flag_etc`] command works on.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FlagWord {
        /// `0x008176f0`: the host's word `+0x304`, the command's bits
        /// shifted up 16 with flag bit 15.
        Host,
        /// `0x008177f0`: the unit's `u16 +0xb0` (the signals).
        Unit,
        /// `0x008179b0`: the signals of unit `u16 cmd +2` ([`opus_no_get`]);
        /// the bits at `cmd +4`, the jump at `+6`, everything 2 bytes on.
        Other,
    }

    /// `0x008176f0`, `0x008177f0` and `0x008179b0` (`opu_flag_etc`,
    /// `opu_signal_etc`, `opu_signal_other`, ours): the flag word
    /// ([`FlagWord`]) and the command's `u16 +2` bits, by flag bits 12..14:
    /// 0 sets them, 1 clears them (`cmd + 4`); 2 jumps by `i16 cmd +4`
    /// (from the command) when any is set (else `cmd + 6`), 3 when none is;
    /// 4 waits (`+0x210` = 1, `cmd`) while any is set, 5 while none is (else
    /// `cmd + 4`); 6 sets them when the unit's effect part index
    /// (`opu_eff_part_index`) plus one is its `g_EtEff` record's byte `+8`
    /// (`cmd + 4`); 7 returns `cmd`. `Other` reads all of these 2 bytes on.
    pub fn opu_flag_etc(mem: &mut impl Mem, which: FlagWord, host: u32, cmd: u32) -> u32 {
        let (u, cmd2) = if which == FlagWord::Other {
            let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
            let me = unit(mem, host);
            (
                opus_no_get(mem, host.wrapping_add(0x300), me, no),
                cmd.wrapping_add(2),
            )
        } else {
            (unit(mem, host), cmd)
        };
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        let mut bits = u32::from(mem.read::<u16>(cmd2.wrapping_add(2)));
        let (word, wide) = match which {
            FlagWord::Host => {
                if flags & 0x8000 != 0 {
                    bits <<= 16;
                }
                (host.wrapping_add(0x304), true)
            }
            FlagWord::Unit | FlagWord::Other => (u.wrapping_add(0xb0), false),
        };
        // the command's length: `Other` has the unit number in front
        let start = cmd;
        let cmd = cmd2;
        match flags >> 12 & 7 {
            0 => {
                let v = load_word(mem, word, wide);
                store_word(mem, word, wide, v | bits);
                cmd.wrapping_add(4)
            }
            1 => {
                let v = load_word(mem, word, wide);
                store_word(mem, word, wide, v & !bits);
                cmd.wrapping_add(4)
            }
            k @ (2 | 3) => {
                let set = load_word(mem, word, wide) & bits != 0;
                if set == (k == 2) {
                    i32::from(mem.read::<i16>(cmd.wrapping_add(4)))
                        .cast_unsigned()
                        .wrapping_add(start)
                } else {
                    cmd.wrapping_add(6)
                }
            }
            k @ (4 | 5) => {
                let set = load_word(mem, word, wide) & bits != 0;
                if set == (k == 4) {
                    mem.write(host.wrapping_add(0x210), 1u8);
                    // the command itself again
                    start
                } else {
                    cmd.wrapping_add(4)
                }
            }
            6 => {
                let idx = crate::et::bridge::opu_eff_part_index(mem, u) & 0xff;
                let i = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
                let e = EFF.ptr(mem).at(i).addr;
                if u32::from(mem.read::<u8>(e + 8)) == idx + 1 {
                    let v = load_word(mem, word, wide);
                    store_word(mem, word, wide, v | bits);
                }
                cmd.wrapping_add(4)
            }
            _ => start,
        }
    }

    fn load_word(mem: &impl Mem, at: u32, wide: bool) -> u32 {
        if wide {
            mem.read(at)
        } else {
            u32::from(mem.read::<u16>(at))
        }
    }

    fn store_word(mem: &mut impl Mem, at: u32, wide: bool, v: u32) {
        if wide {
            mem.write(at, v);
        } else {
            #[allow(clippy::cast_possible_truncation)] // the u16 word
            mem.write(at, v as u16);
        }
    }

    /// `opu_v_get(host, cmd)` (`0x00819940`): from unit `u16 cmd +4`
    /// ([`opus_no_get`]) into the host's unit: vector `(u16 cmd +2 & 7)`
    /// (16 bytes at `+0x30` up) word by word for its flag bits 12..15 (x, y,
    /// z, w); with bit 4 also the floats `+0x90`, `+0x94`, `+0x98` for bits
    /// 12..14 (through the FPU). Returns `cmd + 6`.
    pub fn opu_v_get(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let u = unit(mem, host);
        let no = u32::from(mem.read::<u16>(cmd.wrapping_add(4)));
        let other = opus_no_get(mem, host.wrapping_add(0x300), u, no);
        let bits: u16 = mem.read(cmd.wrapping_add(2));
        let v = u32::from(bits & 7) << 4;
        for k in 0..4 {
            if bits & (0x1000 << k) != 0 {
                let w: u32 = mem.read(other.wrapping_add(v + 0x30 + 4 * k));
                mem.write(u.wrapping_add(v + 0x30 + 4 * k), w);
            }
        }
        if bits & 0x10 != 0 {
            for k in 0..3 {
                if bits & (0x1000 << k) != 0 {
                    let f = crate::x87::load(mem.read(other.wrapping_add(0x90 + 4 * k)));
                    mem.write(u.wrapping_add(0x90 + 4 * k), f);
                }
            }
        }
        cmd.wrapping_add(6)
    }

    /// `opu_mem(host, cmd)` (`0x008170c0`): with no flag bits the unit's
    /// buffer `+0xbc` is freed (`op_free([host +0x3a8], p)`) and cleared
    /// (`cmd + 2`). With nibble 1 the host's set `s = host + 0x300` gets a
    /// heap: sizes `a`, `b`, `c` = the command's `i16 +2`, `+4`, `+6` times
    /// 0x100; `s +0xa8` = `op_heap_init([s +0xc], a)`, `+0xa0` = `[s +0xc] +
    /// a`, `+0xac` = `a`, `+0xa4` = `b`, `+0x54`, `+0x58` = `+0xa0` + `b`,
    /// `+0xb0` = `c`, `+0x28`, `+0x2c` = `[0x00c498dc]`, then its driver's
    /// `[[s +0x50] +0x60](s, heap, a)` (through `call`); `cmd + 8`. Other
    /// nibbles return `cmd`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_mem<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        let s = host.wrapping_add(0x300);
        match nibble(mem, host) {
            0 => {
                let p: u32 = mem.read(u.wrapping_add(0xbc));
                if p != 0 {
                    let heap: u32 = mem.read(host.wrapping_add(0x3a8));
                    crate::op::bridge::op_free(mem, heap, p, call);
                    mem.write(u.wrapping_add(0xbc), 0u32);
                }
                cmd.wrapping_add(2)
            }
            0x1000 => {
                let size = |mem: &M, at: u32| {
                    (i32::from(mem.read::<i16>(cmd.wrapping_add(at))) << 8).cast_unsigned()
                };
                let (a, b, c) = (size(mem, 2), size(mem, 4), size(mem, 6));
                let heap: u32 = mem.read(s.wrapping_add(0xc));
                let r = crate::op::bridge::op_heap_init(mem, heap, a);
                mem.write(s.wrapping_add(0xa8), r);
                let base = mem.read::<u32>(s.wrapping_add(0xc)).wrapping_add(a);
                mem.write(s.wrapping_add(0xa0), base);
                mem.write(s.wrapping_add(0xac), a);
                mem.write(s.wrapping_add(0xa4), b);
                let end = base.wrapping_add(b);
                mem.write(s.wrapping_add(0x54), end);
                mem.write(s.wrapping_add(0x58), end);
                mem.write(s.wrapping_add(0xb0), c);
                for at in [0x28, 0x2c] {
                    let v: u32 = mem.read(g(mem, 0x00c4_98dc));
                    mem.write(s.wrapping_add(at), v);
                }
                let driver: u32 = mem.read(s.wrapping_add(0x50));
                let f: u32 = mem.read(driver.wrapping_add(0x60));
                call(mem, f, &[s, heap, a]);
                cmd.wrapping_add(8)
            }
            _ => cmd,
        }
    }

    /// `opu_r_u(host, cmd)` (`0x00818200`): the direction from the host's
    /// unit to unit `u16 cmd +2` ([`opus_no_get`]) in x/z (`op_deg_get(x,
    /// z, other x, other z, host)`, which stores the distance at `[host]`),
    /// plus `[0x00b51bc0]` with flag bit 0x800 (`f32`), into the unit's
    /// float at byte `cmd +4`; with nibble 1 also the distance times
    /// `g_Vu0FixedScale` at byte `cmd +5`. Returns `cmd + 6` (other
    /// nibbles: `cmd`, after the call).
    pub fn opu_r_u<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        atan2: &mut impl FnMut(&mut M, f64, f64) -> f64,
    ) -> u32 {
        use crate::x87::{ADD, MUL, Reg, load_f32};
        let u = unit(mem, host);
        let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        let other = opus_no_get(mem, host.wrapping_add(0x300), u, no);
        let rd = |mem: &M, at: u32| load_f32(mem.read(at));
        let p = [
            rd(mem, u.wrapping_add(0x90)),
            rd(mem, u.wrapping_add(0x98)),
            rd(mem, other.wrapping_add(0x90)),
            rd(mem, other.wrapping_add(0x98)),
        ];
        let mut d = crate::op::bridge::op_deg_get(mem, p, host, atan2);
        let (at4, at5) = (
            mem.read::<u8>(cmd.wrapping_add(4)),
            mem.read::<u8>(cmd.wrapping_add(5)),
        );
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        let nib = flags & 0xf000;
        if nib > 0x1000 {
            return cmd;
        }
        if flags & 0x800 != 0 {
            let k: f64 = mem.read(g(mem, 0x00b5_1bc0));
            d = Reg::load_f32(d).constant(ADD, k).store();
        }
        mem.write(u.wrapping_add(u32::from(at4)), d);
        if nib == 0x1000 {
            let scale: f64 = mem.read(g(mem, 0x00b3_13a0));
            let len = Reg::load_f32(mem.read(host)).constant(MUL, scale).store();
            mem.write(u.wrapping_add(u32::from(at5)), len);
        }
        cmd.wrapping_add(6)
    }

    /// `opu_r_etc(host, cmd)` (`0x00818110`): with nibble 0 or 1 (the same
    /// code), whether the unit's angle at byte `cmd +2` is within `w` of
    /// its angle at byte `cmd +3` (`op_deg_in_check(a, b - w, b + w)`, `w`
    /// the command's `i16 +4`, `f32` each): a jump by `i16 cmd +6` when it
    /// is, else `cmd + 8`. Other nibbles return `cmd`.
    pub fn opu_r_etc(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        use crate::x87::{ADD, Reg, SUB, load_f32};
        let u = unit(mem, host);
        if nibble(mem, host) > 0x1000 {
            return cmd;
        }
        let a = load_f32(mem.read(u.wrapping_add(u32::from(mem.read::<u8>(cmd.wrapping_add(2))))));
        let b = load_f32(mem.read(u.wrapping_add(u32::from(mem.read::<u8>(cmd.wrapping_add(3))))));
        let w = f32::from(mem.read::<i16>(cmd.wrapping_add(4)));
        let (wr, br) = (Reg::load_f32(w), Reg::load_f32(b));
        let hi = wr.reg(ADD, br).store();
        let lo = br.reg(SUB, wr).store();
        if crate::op::bridge::op_deg_in_check(mem, a, lo, hi) != 0 {
            i32::from(mem.read::<i16>(cmd.wrapping_add(6)))
                .cast_unsigned()
                .wrapping_add(cmd)
        } else {
            cmd.wrapping_add(8)
        }
    }

    /// `0x00802350` (`opu_mdc_matrix_set`, ours; `(host, set)`): the host's
    /// draw head for its unit `u`: the unit's `+0x30..+0x3c` through the
    /// emulated `r` registers (`0x00c8f528`..`534`) into the host's first
    /// 16 bytes, `+0` then 0.0; `host +0x80` = the unit's scale `f32 +0x3c`
    /// times `[0x00b58398]` (`op_unit_m_scale`) rotated and moved by the
    /// unit (`op_rt_m(+0x80, +0x80, u +0x30, u +0x90)`); `host +0xc0` = its
    /// parent's matrix (`opus_parent_get`) times that, then `host +0x2c0`
    /// times it (`op_mul_m`); the head `host +0x100` from
    /// `opus_mdc_head_set(head, host, set, u, i16 [[u +0xa8] + 2])` with
    /// bit 6 of its first word set, and its lights
    /// (`opus_mdc_lights_get(head, set, u, [u +0xa8] + 2)`).
    #[allow(clippy::many_single_char_names)]
    pub fn opu_mdc_matrix_set(mem: &mut impl Mem, host: u32, set: u32) {
        use crate::op::bridge as op;
        use crate::x87::{MUL, Reg};
        let u = unit(mem, host);
        let nos = mem.read::<u32>(u.wrapping_add(0xa8)).wrapping_add(2);
        let scale = crate::x87::load(mem.read(u.wrapping_add(0x3c)));
        let v: [u32; 4] = mem.read(u.wrapping_add(0x30));
        let gpr = g(mem, 0x00c8_f528);
        mem.write(gpr, v);
        mem.write(host, v[0]);
        for k in 1..4 {
            let w: u32 = mem.read(gpr.wrapping_add(4 * k));
            mem.write(host.wrapping_add(4 * k), w);
        }
        mem.write(host, 0f32);
        let k: f64 = mem.read(g(mem, 0x00b5_8398));
        let s = Reg::load(scale).constant(MUL, k).store();
        let m = host.wrapping_add(0x80);
        let out = host.wrapping_add(0xc0);
        op::op_unit_m_scale(mem, m, s);
        op::op_rt_m_zyx(mem, m, m, u.wrapping_add(0x30), u.wrapping_add(0x90));
        let parent = opus_parent_get(mem, host, u);
        op::op_mul_m(mem, out, parent, m);
        op::op_mul_m(mem, out, host.wrapping_add(0x2c0), out);
        let head = host.wrapping_add(0x100);
        let tex = i32::from(mem.read::<i16>(nos)).cast_unsigned();
        opus_mdc_head_set(mem, head, host, set, u, tex);
        let w: u32 = mem.read(head);
        mem.write(head, w | 0x40);
        opus_mdc_lights_get(mem, head, set, u, nos);
    }

    // ---- round 34 a ----

    /// `opu_end_all(host)` (`0x007fd720`): the host's unit gets its first
    /// 16 bytes cleared, the host halts (`byte +0x210` = 1) with `u16
    /// +0x214` and `+0x218` = 0, then `opus_end_all_sub(u16 unit +0x14)`
    /// (through `call`). Returns 0.
    pub fn opu_end_all<M: Mem>(
        mem: &mut M,
        host: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        mem.write(u, [0u32; 4]);
        mem.write(host.wrapping_add(0x210), 1u8);
        mem.write(host.wrapping_add(0x214), 0u16);
        mem.write(host.wrapping_add(0x218), 0u16);
        let k = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
        call(mem, g(mem, 0x007f_e6c0), &[k]);
        0
    }

    /// `opu_ffx(host, cmd)` (`0x00816680`): sub-command `k = i16 cmd +2`
    /// on the effect record of the host's unit (`g_EtEff[u16 unit +0x14]`,
    /// its ID `+0`). 1, 2: `g_OpPrayEffect` = 1, 0. 3: while
    /// `g_OpPrayEffect` is set the host halts (`byte +0x210` = 1, `cmd`
    /// again). 4, 6: `MsEffectReady(id, 1)`, `(id, 0)`. 5: halts while
    /// `MsEffectMotionLevel(id)` is below 1. 7, 8: `Sg_EnableCalcMap`,
    /// `Sg_EnableDrawMap`, `Sg_EnableDrawMenu` and `Sg_EnableDrawGui` with
    /// 1, 0. Callees through `call`. Returns `cmd + 4`, `cmd` when it halts
    /// or for any other `k`.
    pub fn opu_ffx<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        let rec = EFF
            .ptr(mem)
            .addr
            .wrapping_add(u32::from(mem.read::<u16>(u.wrapping_add(0x14))) << 5);
        let pray = g(mem, crate::op::bridge::PRAY_EFFECT.sym);
        let next = cmd.wrapping_add(4);
        let halt = |mem: &mut M| {
            mem.write(host.wrapping_add(0x210), 1u8);
            cmd
        };
        let id = |mem: &M| mem.read::<u32>(rec);
        let enable = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, on: u32| {
            for f in [0x0082_0440, 0x0082_0400, 0x0082_0460, 0x0082_0420] {
                call(mem, g(mem, f), &[on]);
            }
        };
        match mem.read::<i16>(cmd.wrapping_add(2)) {
            1 => mem.write(pray, 1u8),
            2 => mem.write(pray, 0u8),
            3 => {
                if mem.read::<u8>(pray) != 0 {
                    return halt(mem);
                }
            }
            4 => {
                let id = id(mem);
                call(mem, g(mem, 0x0078_8200), &[id, 1]);
            }
            5 => {
                let id = id(mem);
                if call(mem, g(mem, 0x0078_7ca0), &[id]).cast_signed() < 1 {
                    return halt(mem);
                }
            }
            6 => {
                let id = id(mem);
                call(mem, g(mem, 0x0078_8200), &[id, 0]);
            }
            7 => enable(mem, call, 1),
            8 => enable(mem, call, 0),
            _ => return cmd,
        }
        next
    }

    /// `opu_v_2d(host, cmd)` (`0x00818dc0`): unless the host's flag nibble
    /// is set (then `cmd`), places the host's unit on the screen at another
    /// unit `o` (`u16 cmd +2`, [`opus_no_get`]): the host's matrix `+0x80`
    /// = the parent's matrix (`opus_parent_get(host, o)`) times a move by
    /// `o +0x90`; `host +0xc0` = `host +0x2c0` times that; `host +0..+0x10`
    /// = the emulated `r0` (`0x00c8f508`), then `op_rtp_w1(host, host
    /// +0xc0, host)`. Its screen x, y (`(i32 +0 - 0x8000) >> 4`, `+4`)
    /// rounded to `f32`, times the unit's scale `+0x40` and `[0x00b50198]`
    /// (1/256), go to the unit's `+0x90`, `+0x94`. Returns `cmd + 4`.
    pub fn opu_v_2d<M: Mem>(mem: &mut M, host: u32, cmd: u32) -> u32 {
        use crate::op::bridge as op;
        let me = unit(mem, host);
        if nibble(mem, host) != 0 {
            return cmd;
        }
        let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        let other = opus_no_get(mem, host.wrapping_add(0x300), me, no);
        let mat = host.wrapping_add(0x80);
        crate::small::bridge::op_unit_m(mem, mat);
        op::op_trans_m(mem, mat, mat, other.wrapping_add(0x90));
        let parent = opus_parent_get(mem, host, other);
        op::op_mul_m(mem, mat, parent, mat);
        let out = host.wrapping_add(0xc0);
        op::op_mul_m(mem, out, host.wrapping_add(0x2c0), mat);
        let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
        mem.write(host, r0);
        op::op_rtp_w1(mem, host, out, host);
        let screen = |mem: &M, k: u32| {
            let v = mem
                .read::<i32>(host.wrapping_add(4 * k))
                .wrapping_sub(0x8000)
                >> 4;
            #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
            let v = v as f32;
            v
        };
        let (x, y) = (screen(mem, 0), screen(mem, 1));
        let k: f64 = mem.read(g(mem, 0x00b5_0198));
        let scale: f32 = mem.read(me.wrapping_add(0x40));
        let x = Reg::load_f32(x).mem(MUL, scale).constant(MUL, k).store();
        mem.write(me.wrapping_add(0x90), x);
        let y = Reg::load_f32(y).mem(MUL, scale).constant(MUL, k).store();
        mem.write(me.wrapping_add(0x94), y);
        cmd.wrapping_add(4)
    }

    // ---- round 34 b ----

    /// The unit-type hook a float command ends with: when the host's flag
    /// bit 9 is set, `[0x00c492c8 + 4 byte unit +0xba](host, next)`
    /// (through `call`).
    fn unit_type_hook<M: Mem>(
        mem: &mut M,
        host: u32,
        unit: u32,
        next: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if mem.read::<u16>(host.wrapping_add(0x21a)) & 0x200 != 0 {
            let i = u32::from(mem.read::<u8>(unit.wrapping_add(0xba)));
            let f: u32 = mem.read(g(mem, 0x00c4_92c8).wrapping_add(4 * i));
            call(mem, f, &[host, next]);
        }
    }

    /// `0x0080c6a0(host, cmd)` (`opu_float_calc`, ours): with flag bit 10
    /// the command names another unit `o` (`u16 cmd +2`, [`opus_no_get`];
    /// the rest of the command follows two bytes later), else `o` is the
    /// host's unit `u`. The target base `t` is `u`, or `o` when the flags'
    /// low byte is 0xa9. With flag bit 11 `a = f32 t[c +3]`, `b = f32 o[c
    /// +4]` into `t[c +2]` (byte offsets), else `b = i16 c +2` as a float,
    /// `a = o[c +5]` into `o[c +4]`. The nibble picks `b + a` (0), `a - b`
    /// (1), `b * a` (2) or `a / b` (3, 0 for a `b` of 0), stored as `f32`;
    /// others store nothing. Then [`unit_type_hook`] on `t`. Returns the
    /// command's end (`+6`).
    #[allow(clippy::many_single_char_names)]
    pub fn opu_float_calc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::x87::{ADD, SUB};
        let flags = |mem: &M| u32::from(mem.read::<u16>(host.wrapping_add(0x21a)));
        let u = unit(mem, host);
        let mut c = cmd;
        let (src, base) = if flags(mem) & 0x400 != 0 {
            let no = u32::from(mem.read::<u16>(c.wrapping_add(2)));
            c = c.wrapping_add(2);
            let o = opus_no_get(mem, host.wrapping_add(0x300), u, no);
            let u = unit(mem, host);
            (o, if flags(mem) & 0xff == 0xa9 { o } else { u })
        } else {
            (u, u)
        };
        let f = flags(mem);
        let byte = |mem: &M, k: u32| u32::from(mem.read::<u8>(c.wrapping_add(k)));
        let (a, b, dst) = if f & 0x800 != 0 {
            let a = crate::x87::load(mem.read(base.wrapping_add(byte(mem, 3))));
            let b = crate::x87::load(mem.read(src.wrapping_add(byte(mem, 4))));
            (a, b, base.wrapping_add(byte(mem, 2)))
        } else {
            #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
            let b = (f32::from(mem.read::<i16>(c.wrapping_add(2)))).to_bits();
            let a = crate::x87::load(mem.read(src.wrapping_add(byte(mem, 5))));
            (a, b, base.wrapping_add(byte(mem, 4)))
        };
        let r = match f & 0xf000 {
            0 => Some(Reg::load(b).mem(ADD, f32::from_bits(a))),
            0x1000 => Some(Reg::load(a).mem(SUB, f32::from_bits(b))),
            0x2000 => Some(Reg::load(b).mem(MUL, f32::from_bits(a))),
            // fldz; fucom: a `b` of 0 (either sign) gives 0, a NaN goes on
            // to `fdivr` (st = m32 / st)
            0x3000 => Some(if Reg::load(b).value() == 0.0 {
                Reg::Num(0.0)
            } else {
                Reg::load(b).mem(|st, m| m / st, f32::from_bits(a))
            }),
            _ => None,
        };
        if let Some(r) = r {
            mem.write(dst, r.store());
        }
        let next = c.wrapping_add(6);
        unit_type_hook(mem, host, base, next, call);
        next
    }

    /// `0x0080c520(host, cmd)` (`opu_wait_cond`, ours): the flag nibble
    /// picks what goes into the host's halt byte `+0x210` (`cmd + 4`
    /// unless said otherwise): 0 `op_irandom_p(i16 cmd +2)` (its low byte;
    /// `junk` are the three stack words it reads past its argument, the
    /// caller's `edi`, `esi` and `ebp`), 1 `u8 cmd +2 x
    /// opu_eff_part_index(u)` (8-bit), 2 `ftol(f32 u[i16 cmd +2])`; 3 halts
    /// (`1`, `cmd`) unless `op_et_chr_index_by_no(i16 cmd +2)` finds the
    /// character; 8 halts unless unit `u16 cmd +2` ([`opus_no_get`]) is
    /// the host's first part (`host +0x320`); 9 halts unless
    /// `g_OpEventEffect` (then `cmd + 2`). Others return `cmd`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_wait_cond(mem: &mut impl Mem, host: u32, cmd: u32, junk: [u32; 3]) -> u32 {
        let halt = host.wrapping_add(0x210);
        let wide = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let u = unit(mem, host);
        #[allow(clippy::cast_possible_truncation)] // the low byte
        let v = match nibble(mem, host) {
            0 => crate::rand::bridge::op_irandom_core(mem, [wide, junk[0], junk[1], junk[2]]) as u8,
            0x1000 => {
                let n = crate::et::bridge::opu_eff_part_index(mem, u) as u8;
                mem.read::<u8>(cmd.wrapping_add(2)).wrapping_mul(n)
            }
            0x2000 => {
                let x = ffx_bridge::x87::load(mem.read(u.wrapping_add(wide)));
                ffx_bridge::x87::ftol(mem, x) as u8
            }
            0x3000 => {
                if crate::et::bridge::op_et_chr_index_by_no(mem, wide) < 0 {
                    mem.write(halt, 1u8);
                    return cmd;
                }
                return cmd.wrapping_add(4);
            }
            0x8000 => {
                let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
                let o = opus_no_get(mem, host.wrapping_add(0x300), u, no);
                if o == mem.read::<u32>(host.wrapping_add(0x320)) {
                    return cmd.wrapping_add(4);
                }
                mem.write(halt, 1u8);
                return cmd;
            }
            0x9000 => {
                if crate::op::bridge::EVENT_EFFECT.get(mem) != 0 {
                    return cmd.wrapping_add(2);
                }
                mem.write(halt, 1u8);
                return cmd;
            }
            _ => return cmd,
        };
        mem.write(halt, v);
        cmd.wrapping_add(4)
    }

    /// `0x0080c940(host, cmd)` (`opu_float_move`, ours): with flag bit 10
    /// the command names another unit `o` (`u16 cmd +2`, [`opus_no_get`];
    /// the rest follows two bytes later as `c`), else `o` is the host's
    /// unit `u`. The bytes `lo`, `hi` of `u16 c +2` are float offsets; the
    /// nibble: 0 `o[i16 c +2] = i16 c +4` (`c + 6`), 1 `o[lo] = o[hi]`, 2
    /// swaps them, 3 `u[lo] = o[hi]`, 4 `o[hi] = u[lo]`, 5 `o[lo] =
    /// -u[lo]` (`c + 4`); others move nothing (`c`). Floats through the
    /// FPU. Then [`unit_type_hook`] on `o`. Returns the end.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_float_move<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{load, store};
        let u = unit(mem, host);
        let mut c = cmd;
        let o = if mem.read::<u16>(host.wrapping_add(0x21a)) & 0x400 != 0 {
            let no = u32::from(mem.read::<u16>(c.wrapping_add(2)));
            c = c.wrapping_add(2);
            opus_no_get(mem, host.wrapping_add(0x300), u, no)
        } else {
            u
        };
        let pair: u16 = mem.read(c.wrapping_add(2));
        let (lo, hi) = (u32::from(pair & 0xff), u32::from(pair >> 8));
        let f = |mem: &M, a: u32| load(mem.read(a));
        let next = match nibble(mem, host) {
            0 => {
                let v = f32::from(mem.read::<i16>(c.wrapping_add(4)));
                let at = i32::from(pair.cast_signed()).cast_unsigned();
                mem.write(o.wrapping_add(at), v);
                c.wrapping_add(6)
            }
            0x1000 => {
                let v = f(mem, o.wrapping_add(hi));
                mem.write(o.wrapping_add(lo), store(v));
                c.wrapping_add(4)
            }
            0x2000 => {
                let t = store(f(mem, o.wrapping_add(lo)));
                let v = f(mem, o.wrapping_add(hi));
                mem.write(o.wrapping_add(lo), store(v));
                mem.write(o.wrapping_add(hi), store(load(t)));
                c.wrapping_add(4)
            }
            0x3000 => {
                let v = f(mem, o.wrapping_add(hi));
                mem.write(u.wrapping_add(lo), store(v));
                c.wrapping_add(4)
            }
            0x4000 => {
                let v = f(mem, u.wrapping_add(lo));
                mem.write(o.wrapping_add(hi), store(v));
                c.wrapping_add(4)
            }
            0x5000 => {
                let v = f(mem, u.wrapping_add(lo));
                mem.write(o.wrapping_add(lo), store(-v));
                c.wrapping_add(4)
            }
            _ => c,
        };
        unit_type_hook(mem, host, o, next, call);
        next
    }

    // ---- round 34 c ----

    /// `op_part_init(unit)` (`0x00800770`): the op environment's three
    /// tables (`0x012f2090`..`98`) from the unit's ppp part `d = [unit
    /// +0xd0]` (`get_ptr32` of `+0x14`, `+0x18`, `+0x1c`), the environment
    /// made current; the unit's key `byte +0x1e << 16 + u16 +0x1a` gets a
    /// slot of the 256 at `0x012e9c70`: its own (freed first:
    /// `pppAllFreePObject` of slot `i`'s part `0x012ea070 + 0x80 i`), else
    /// the first empty one, else slot 0x100 (freed, as the original's
    /// loops fall through). The slot keeps the key and `unit +0xbc` = its
    /// part. For a loaded `d` (first byte not 0) the part starts
    /// (`pppStartPartFromYonish(part, 0x1000, [get_ptr32(d +0x10) + 4 byte
    /// unit +0xd4], 1)`, and a fifth word, `byte d +3 == 1`, that the
    /// callee does not read), else `dbgPrintf(0x00b58718)`. Callees
    /// through `call`. Returns 0.
    pub fn op_part_init<M: Mem>(
        mem: &mut M,
        unit: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_system::stub::ptr32;
        let d: u32 = mem.read(unit.wrapping_add(0xd0));
        for k in 0..3 {
            let v = ptr32(mem.read(d.wrapping_add(0x14 + 4 * k)));
            mem.write(g(mem, 0x012f_2090).wrapping_add(4 * k), v);
        }
        op_env(mem);
        let key = (u32::from(mem.read::<u8>(unit.wrapping_add(0x1e))) << 16)
            .wrapping_add(u32::from(mem.read::<u16>(unit.wrapping_add(0x1a))));
        let slots = g(mem, 0x012e_9c70);
        let parts = g(mem, 0x012e_a070);
        let find =
            |mem: &M, want: u32| (0..0x100u32).find(|&i| mem.read::<u32>(slots + 4 * i) == want);
        let (i, free) = match find(mem, key) {
            Some(i) => (i, true),
            None => find(mem, 0).map_or((0x100, true), |i| (i, false)),
        };
        if free {
            op_env(mem);
            call(mem, g(mem, 0x0071_1f20), &[parts.wrapping_add(i << 7)]);
        }
        mem.write(slots.wrapping_add(4 * i), key);
        let part = parts.wrapping_add(i << 7);
        mem.write(unit.wrapping_add(0xbc), part);
        if mem.read::<u8>(d) == 0 {
            call(mem, g(mem, 0x0062_f500), &[g(mem, 0x00b5_8718)]);
            return 0;
        }
        op_env(mem);
        let d: u32 = mem.read(unit.wrapping_add(0xd0));
        let once = u32::from(mem.read::<u8>(d.wrapping_add(3)) == 1);
        let data = ptr32(mem.read(d.wrapping_add(0x10)));
        let k = u32::from(mem.read::<u8>(unit.wrapping_add(0xd4)));
        let set: u32 = mem.read(data.wrapping_add(4 * k));
        call(mem, g(mem, 0x0071_2560), &[part, 0x1000, set, 1, once]);
        0
    }

    // ---- round 35 a ----

    /// `opus_mdc_break_draw(brk, id, _, _, still)` (`0x00805510`), the
    /// shattered model's pieces (`i32 brk+4` of them, 0xc0 bytes from
    /// `brk+0x30`) into instance `(0, id)`'s segment-0 buffer (`3n`
    /// vertices and indices; the original assumes both exist). Scratch
    /// from `op_spr_alloc(0x1800)`: the matrix at `+0x1440`, two counters
    /// at `+0x1600`/`+0x1604` started from the emulated `r0` (the second
    /// counts drawn pieces). Unless `still`, each piece moves (`+0x80` +=
    /// `+0x84`, then the distance `+0x7c` += `+0x80`; `+0xb8` += `+0xbc`,
    /// then the drop `+0xb4` += `+0xb8`; the spin `+0x90..+0x98` +=
    /// `+0xa0..+0xa8`; its life `u16 +0x48` down to 0 with alpha byte
    /// `+0x5b` = `min(8 life, 0x80)`); the number still
    /// alive (with `still`: `id`) decides: none sets the break flags `u16
    /// [brk+0xc]` into `id+0xb0` and skips the pieces. Each piece's
    /// position `+0x60..+0x68` = direction `+0x70..+0x78` x distance
    /// `+0x7c` plus the centre `brk+0x10..+0x18` (`y` also `+ +0xb4`); a
    /// piece at or below the floor `brk+0x20` turns its `+0xb8` around.
    /// Unless the renderer skips the frame (when the piece pointer does
    /// not move on, as in the original), `opus_mdc_break_matrix` and,
    /// with a non-zero alpha, the colour (`rgba +0x58..+0x5b` / 256 on
    /// three vertices), the triangle `+0..+0x2c` (3 vectors in `local`,
    /// `w` 1.0, `FFXVu0ApplyMatrix`), its texture coordinates (`+0x30`,
    /// `+0x34`, `+0x40`, `+0x44`, `+0x50`, `+0x54`) and the indices
    /// `3k..3k+2` at those same slots. Then `sceGsSyncPath(0, 0)`, the
    /// unused indices zeroed, the unit matrix, `+0x80` = 0, the
    /// ±9999999 box, `graphicVFXSetVisible(0, id, 1)`, `graphicVFXUpdate`
    /// and `op_spr_free`.
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn opus_mdc_break_draw<M: Mem>(
        mem: &mut M,
        brk: u32,
        id: u32,
        still: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let count = |mem: &M| mem.read::<u32>(brk.wrapping_add(4)).cast_signed();
        let n = mem.read::<u32>(brk.wrapping_add(4));
        let mut rd = go(mem, 0x0064_53d0, &[0, id]);
        if rd != 0 && go(mem, 0x0063_fdb0, &[rd, 0, 3 * n, 3 * n]) == 0 {
            rd = 0;
        }
        let spr = go(mem, 0x007e_6a00, &[0x1800]);
        let base = spr.wrapping_add(0x1400);
        let r0: [u32; 4] = mem.read(g(mem, 0x00c8_f508));
        mem.write(base.wrapping_add(0x200), r0);
        let drawn = base.wrapping_add(0x204);
        let w = |mem: &M, at: u32| load(mem.read(at));
        let moving = if still == 0 {
            let mut alive = 0u32;
            let mut i = 0i32;
            while i < count(mem) {
                let p = brk.wrapping_add(0xb0 + 0xc0 * i.cast_unsigned());
                let at = |k: i32| p.wrapping_add_signed(k);
                let life = i32::from(mem.read::<u16>(at(-0x38))) - 1;
                let v = store(add(w(mem, at(4)), w(mem, at(0))));
                mem.write(at(0), v);
                mem.write(at(-4), store(add(load(v), w(mem, at(-4)))));
                let v = store(add(w(mem, at(0x3c)), w(mem, at(0x38))));
                mem.write(at(0x38), v);
                mem.write(at(0x34), store(add(load(v), w(mem, at(0x34)))));
                for k in [0x10, 0x14, 0x18] {
                    mem.write(at(k), store(add(w(mem, at(k + 0x10)), w(mem, at(k)))));
                }
                let life = if life > 0 {
                    alive += 1;
                    life
                } else {
                    0
                };
                mem.write(at(-0x38), life as u16);
                mem.write(at(-0x25), (life * 8).min(0x80) as u8);
                i += 1;
            }
            alive
        } else {
            id
        };
        if moving == 0 {
            let flags = mem.read::<u16>(mem.read(brk.wrapping_add(0xc)));
            let at = id.wrapping_add(0xb0);
            mem.write(at, mem.read::<u16>(at) | flags);
        } else {
            let k256: f64 = mem.read(g(mem, 0x00b5_0198));
            let mat = base.wrapping_add(0x40);
            let mut piece = brk.wrapping_add(0x30);
            let mut i = 0i32;
            while i < count(mem) {
                let at = |k: u32| piece.wrapping_add(k);
                let b = |k: u32| brk.wrapping_add(k);
                let dist = w(mem, at(0x7c));
                mem.write(
                    at(0x60),
                    store(add(mul(w(mem, at(0x70)), dist), w(mem, b(0x10)))),
                );
                let y = add(
                    add(mul(w(mem, at(0x74)), dist), w(mem, b(0x14))),
                    w(mem, at(0xb4)),
                );
                mem.write(at(0x64), store(y));
                mem.write(
                    at(0x68),
                    store(add(mul(w(mem, at(0x78)), dist), w(mem, b(0x18)))),
                );
                if w(mem, b(0x20)) <= w(mem, at(0x64)) {
                    let flip = store(load(mem.read(at(0xb8)))) ^ 0x8000_0000;
                    mem.write(at(0xb8), flip);
                }
                if mem.read::<u32>(g(mem, 0x012f_b7d0)) == 0 {
                    go(mem, 0x0080_2e20, &[mat, piece]);
                    let alpha = mem.read::<u8>(at(0x5b));
                    if alpha != 0 {
                        let rgba = [0x58u32, 0x59, 0x5a]
                            .map(|k| store(mul(f64::from(mem.read::<u8>(at(k))), k256)));
                        let a = store(mul(k256, f64::from(alpha)));
                        let geo: u32 = mem.read(rd.wrapping_add(0x94));
                        for j in 0..3u32 {
                            let k = mem.read::<u32>(drawn);
                            let col = mem
                                .read::<u32>(geo.wrapping_add(0x14))
                                .wrapping_add(16 * (j + 3 * k));
                            mem.write(col, [rgba[0], rgba[1], rgba[2], a]);
                        }
                        for v in 0..3u32 {
                            go(mem, 0x006e_d370, &[local.wrapping_add(16 * v), at(16 * v)]);
                        }
                        for v in 0..3u32 {
                            let vec = local.wrapping_add(16 * v);
                            mem.write(vec.wrapping_add(0xc), 1.0f32.to_bits());
                            go(mem, 0x006e_d2e0, &[vec, mat, vec]);
                        }
                        let k = mem.read::<u32>(drawn);
                        let geo: u32 = mem.read(rd.wrapping_add(0x94));
                        let pos = mem.read::<u32>(geo.wrapping_add(0xc)).wrapping_add(36 * k);
                        for v in 0..3u32 {
                            for c in 0..3u32 {
                                let x = store(w(mem, local.wrapping_add(16 * v + 4 * c)));
                                mem.write(pos.wrapping_add(12 * v + 4 * c), x);
                            }
                        }
                        let uv = mem.read::<u32>(geo.wrapping_add(0x18)).wrapping_add(24 * k);
                        for (j, o) in [0x30u32, 0x34, 0x40, 0x44, 0x50, 0x54]
                            .into_iter()
                            .enumerate()
                        {
                            let x = store(w(mem, at(o)));
                            mem.write(uv.wrapping_add(4 * j as u32), x);
                        }
                        let idx: u32 = mem.read(geo.wrapping_add(0x1c));
                        for j in 0..3u16 {
                            let v = (k as u16).wrapping_mul(3).wrapping_add(j);
                            mem.write(idx.wrapping_add(2 * u32::from(v)), v);
                        }
                        mem.write(drawn, k.wrapping_add(1));
                    }
                    let c = mem.read::<u32>(base.wrapping_add(0x200));
                    mem.write(base.wrapping_add(0x200), c.wrapping_add(1));
                    piece = piece.wrapping_add(0xc0);
                }
                i += 1;
            }
        }
        go(mem, 0x0062_f880, &[0, 0]);
        let k = mem.read::<u32>(drawn);
        let geo: u32 = mem.read(rd.wrapping_add(0x94));
        let idx: u32 = mem.read(geo.wrapping_add(0x1c));
        let left = mem
            .read::<u32>(brk.wrapping_add(4))
            .wrapping_sub(k)
            .cast_signed();
        let start = idx.wrapping_add(k.wrapping_mul(6));
        for b in 0..(6 * left.max(0)).cast_unsigned() {
            mem.write(start.wrapping_add(b), 0u8);
        }
        go(mem, 0x006e_d9a0, &[rd]);
        mem.write(rd.wrapping_add(0x80), 0u32);
        let lo_box: u32 = mem.read(g(mem, 0x00b5_1660));
        let hi_box: u32 = mem.read(g(mem, 0x00b5_1658));
        mem.write(rd.wrapping_add(0x98), [lo_box; 3]);
        mem.write(rd.wrapping_add(0xa4), [hi_box; 3]);
        go(mem, 0x0064_5710, &[0, id, 1]);
        go(mem, 0x0064_58a0, &[0, id]);
        go(mem, 0x007e_6a60, &[0x1800]);
    }

    /// `0x00802e20` (`opus_mdc_break_matrix`, ours; `(out, src)`, the
    /// matrix `opus_mdc_break_draw` draws a piece with): the angles `f32
    /// src +0x90`, `+0x94`, `+0x98` through `ftol` (a turn being 0x1000)
    /// pick sines `sa`, `sb`, `sc` and cosines (index `+0x400`) `ca`, `cb`,
    /// `cc` from `g_pppSinTable`; the rows (each element rounded to `f32`)
    /// are `(cc cb, sc ca + sa (cc sb), sc sa - (cc sb) ca, 0)`,
    /// `(-sc cb, cc ca - sa (sc sb), (sc sb) ca + cc sa, 0)` and
    /// `(sb, -cb sa, cb ca, 0)`; the position `src +0x60..+0x68` and 1.0.
    /// Returns `out` (left in eax).
    #[allow(clippy::many_single_char_names)]
    pub fn opus_mdc_break_matrix(mem: &mut impl Mem, out: u32, src: u32) -> u32 {
        use ffx_bridge::x87::{add, ftol, load, mul, store, sub};
        fn entry(mem: &impl Mem, i: u32) -> f64 {
            let sin = crate::ppp::bridge::SIN.ptr(mem).addr;
            load(mem.read(sin.wrapping_add(4 * (i & 0xfff))))
        }
        let a = ftol(mem, load(mem.read(src.wrapping_add(0x90))));
        let b = ftol(mem, load(mem.read(src.wrapping_add(0x94))));
        let c = ftol(mem, load(mem.read(src.wrapping_add(0x98))));
        let (sa, ca) = (entry(mem, a), entry(mem, a.wrapping_add(0x400)));
        let (sb, cb) = (entry(mem, b), entry(mem, b.wrapping_add(0x400)));
        let (sc, cc) = (entry(mem, c), entry(mem, c.wrapping_add(0x400)));
        let at = |k: u32| out.wrapping_add(4 * k);
        mem.write(at(0), store(mul(cc, cb)));
        let ccsb = mul(cc, sb);
        mem.write(at(1), store(add(mul(sc, ca), mul(sa, ccsb))));
        mem.write(at(2), store(sub(mul(sc, sa), mul(ccsb, ca))));
        mem.write(at(4), store(mul(-sc, cb)));
        let scsb = mul(sc, sb);
        mem.write(at(5), store(sub(mul(cc, ca), mul(sa, scsb))));
        mem.write(at(6), store(add(mul(scsb, ca), mul(cc, sa))));
        mem.write(at(8), store(sb));
        mem.write(at(9), store(mul(sa, -cb)));
        mem.write(at(10), store(mul(cb, ca)));
        for k in 0..3 {
            let v = load(mem.read(src.wrapping_add(0x60 + 4 * k)));
            mem.write(at(12 + k), store(v));
        }
        for k in [11, 7, 3] {
            mem.write(at(k), 0f32);
        }
        mem.write(at(15), 1f32);
        out
    }

    /// `0x0080f680` (`opus_mimic_screen_set`, ours; `(host)`): for the
    /// host's unit `u`: magic 0x25c or 0x267 (`MagicFile_GetMagicId`,
    /// asked twice when the first is not 0x25c) clears the word `u +0xc8`;
    /// then `graphicActivateBlurEffectEx(0)`, `0x00643830(r, g, b, a)` with
    /// the bytes `u +0xc8..+0xcb` times `[0x00b170b8]` (1/128),
    /// `0x00643890(x, y)` with `x = (f32 u +0x30 x [0x00b41cc0]) x 2` and
    /// `y = (f32 u +0x34 x 2) x [0x00b41cc0]` (1/512),
    /// `0x00643860(f32 u +0x90 x [0x00b41cc0] + 0.5, f32 u +0x94 /
    /// [0x00b41d30] (416) + 0.5, 0, 0)` and
    /// `graphicSetFlipVsnc(calculateVFXSP_AlphaFunction(u +0xc2))`, whose
    /// eax is returned. Every argument rounded to `f32`; callees through
    /// `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opus_mimic_screen_set<M: Mem>(
        mem: &mut M,
        host: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, load, mul, store};
        let u = unit(mem, host);
        let magic = g(mem, 0x009d_a3b0);
        if call(mem, magic, &[]) == 0x25c || call(mem, magic, &[]) == 0x267 {
            mem.write(u.wrapping_add(0xc8), 0u32);
        }
        call(mem, g(mem, 0x0063_9110), &[0]);
        let k: f64 = mem.read(g(mem, 0x00b1_70b8));
        let mut rgba = [0u32; 4];
        for (i, c) in (0..4u32).zip(rgba.iter_mut()).rev() {
            let b = mem.read::<u8>(u.wrapping_add(0xc8 + i));
            *c = store(mul(f64::from(b), k));
        }
        call(mem, g(mem, 0x0064_3830), &rgba);
        let two: f64 = mem.read(g(mem, 0x00b9_2270));
        let k: f64 = mem.read(g(mem, 0x00b4_1cc0));
        let f = |mem: &M, at: u32| load(mem.read(u.wrapping_add(at)));
        let y = store(mul(mul(f(mem, 0x34), two), k));
        let x = store(mul(mul(k, f(mem, 0x30)), two));
        call(mem, g(mem, 0x0064_3890), &[x, y]);
        let (h, half): (f64, f64) = (mem.read(g(mem, 0x00b4_1d30)), mem.read(g(mem, 0x00b9_2220)));
        let y = store(add(div(f(mem, 0x94), h), half));
        let x = store(add(half, mul(f(mem, 0x90), k)));
        call(mem, g(mem, 0x0064_3860), &[x, y, 0, 0]);
        let alpha = u32::from(mem.read::<u8>(u.wrapping_add(0xc2)));
        let a = call(mem, g(mem, 0x0071_2c30), &[alpha]);
        call(mem, g(mem, 0x0064_3810), &[a])
    }

    /// `0x008088c0` (`opu_tex_ctrl`, ours; `(host, cmd)` -> the next
    /// command): for the unit `u`'s character `h = g_EtChrHandles[u +0x18]`
    /// and its oef2 `o = host +0x360 + 4 (u +0x1e)`, by the flag nibble:
    /// 0: `op_oef2_texture_load(op_oef2_a_get(o, 2, i16 cmd +2), 1)`,
    /// `cmd + 4`; 0x1000 / 0x2000: `op_et_chr_show(1 / 0)`, `cmd + 2`; 0x4000:
    /// `Ch_TexGetImage(h, i16 cmd +2, &w, &h)` (`w`, `h` in the original's
    /// locals at `frame - 4` and `- 8`, `frame` being its `ebp`) gets
    /// `((w h - 1) >> 4) + 1` 16-byte blocks copied over it, word by word,
    /// from `op_oef2_a_get(o, 9, i16 cmd +4)` (none when `w h <= 0`), then
    /// magic 0x276..=0x27d (`MagicFile_GetMagicId`, asked again when the
    /// first is at least 0x276) `graphicCharacterReplaceSummonTexture(h)`,
    /// `cmd + 6`; 0x5000: `Ch_TextureAnimSetEnable(h, 0)`, `cmd + 2`;
    /// others: `cmd`. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_tex_ctrl<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        let slot = u32::from(mem.read::<u8>(u.wrapping_add(0x1e)));
        let oef: u32 = mem.read(host.wrapping_add(0x360).wrapping_add(4 * slot));
        let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        let arg =
            |mem: &M, at: u32| i32::from(mem.read::<i16>(cmd.wrapping_add(at))).cast_unsigned();
        let a_get = g(mem, 0x007e_3810);
        match nibble(mem, host) {
            0 => {
                let n = arg(mem, 2);
                let a = call(mem, a_get, &[oef, 2, n]);
                call(mem, g(mem, 0x007e_5030), &[a, 1]);
                cmd.wrapping_add(4)
            }
            on @ (0x1000 | 0x2000) => {
                call(mem, g(mem, 0x007f_f090), &[u32::from(on == 0x1000)]);
                cmd.wrapping_add(2)
            }
            0x4000 => {
                let (i, n) = (arg(mem, 2), arg(mem, 4));
                let src = call(mem, a_get, &[oef, 9, n]);
                let (pw, ph) = (frame.wrapping_sub(4), frame.wrapping_sub(8));
                let dst = call(mem, g(mem, 0x0083_cd50), &[h, i, pw, ph]);
                let size = mem.read::<i32>(pw).wrapping_mul(mem.read::<i32>(ph));
                if size > 0 {
                    let blocks = ((size.cast_unsigned() - 1) >> 4) + 1;
                    let rel = src.wrapping_sub(dst);
                    for b in 0..blocks {
                        for k in 0..4 {
                            let at = dst.wrapping_add(16 * b + 4 * k);
                            let w: u32 = mem.read(rel.wrapping_add(at));
                            mem.write(at, w);
                        }
                    }
                }
                let magic = g(mem, 0x009d_a3b0);
                if call(mem, magic, &[]).cast_signed() >= 0x276
                    && call(mem, magic, &[]).cast_signed() <= 0x27d
                {
                    call(mem, g(mem, 0x0063_d400), &[h]);
                }
                cmd.wrapping_add(6)
            }
            0x5000 => {
                call(mem, g(mem, 0x0083_0d10), &[h, 0]);
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `opu_level(host, cmd)` (`0x00817ad0`) -> the next command. With bit
    /// 0x8000 of `u16 host +0x21a` clear, `lvl` is the low byte of
    /// `Ch_EffectGetEffectLevel(g_EtChrHandles[u +0x18])` (and `sig` the
    /// unit pointer); set, `sig = MsGetChrMotionSignal(u +0x18)` and `lvl`
    /// is the unit pointer's top byte (what the original leaves in `al`).
    /// With `v = u16 cmd +2`, a jump being `cmd + i16 cmd +4`, a halt
    /// `byte host +0x210 = 1` and `cmd`, by the flag nibble: 0 jump when
    /// `lvl == v` else `cmd + 6`; 0x1000 jump when `lvl != v`; 0x2000 jump
    /// when `lvl & v` (low bytes); 0x3000 jump when not; 0x4000 `cmd + 4`
    /// when `lvl != v` else halt; 0x5000 the reverse; 0x6000
    /// `Ch_EffectSetEffectLevel(handle, v)`, `cmd + 4`; 0x8000 jump when
    /// `sig & v`; 0x9000 jump when not; 0xa000 halt when `sig & v` else
    /// `cmd + 4`; 0xb000 the reverse; others `cmd`. Callees through `call`.
    pub fn opu_level<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let handle = |mem: &M| crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        let (lvl, sig) = if mem.read::<u16>(host.wrapping_add(0x21a)) & 0x8000 == 0 {
            let h = handle(mem);
            let l = call(mem, g(mem, 0x0082_a9a0), &[h]);
            (l.to_le_bytes()[0], u)
        } else {
            let s = call(mem, g(mem, 0x0079_5660), &[chr]);
            (u.to_le_bytes()[3], s)
        };
        let v = mem.read::<u16>(cmd.wrapping_add(2));
        let jump = |mem: &M| {
            let d = i32::from(mem.read::<i16>(cmd.wrapping_add(4))).cast_unsigned();
            cmd.wrapping_add(d)
        };
        let same = u16::from(lvl) == v;
        let bits = lvl & v.to_le_bytes()[0] != 0;
        let sig_bits = sig & u32::from(v) != 0;
        let halt_unless = |mem: &mut M, go_on: bool| {
            if go_on {
                cmd.wrapping_add(4)
            } else {
                mem.write(host.wrapping_add(0x210), 1u8);
                cmd
            }
        };
        let jump_if = |mem: &M, j: bool| if j { jump(mem) } else { cmd.wrapping_add(6) };
        match nibble(mem, host) {
            0 => jump_if(mem, same),
            0x1000 => jump_if(mem, !same),
            0x2000 => jump_if(mem, bits),
            0x3000 => jump_if(mem, !bits),
            0x4000 => halt_unless(mem, !same),
            0x5000 => halt_unless(mem, same),
            0x6000 => {
                let h = handle(mem);
                call(mem, g(mem, 0x0082_a9d0), &[h, u32::from(v)]);
                cmd.wrapping_add(4)
            }
            0x8000 => jump_if(mem, sig_bits),
            0x9000 => jump_if(mem, !sig_bits),
            0xa000 => halt_unless(mem, !sig_bits),
            0xb000 => halt_unless(mem, sig_bits),
            _ => cmd,
        }
    }

    // ---- round 35 b ----

    /// `opu_part_init(host, cmd)` (`0x0080b670`) -> the next command; `o`
    /// the unit `u`'s oef2 (`host +0x360 + 4 (u +0x1e)`). By the flag
    /// nibble: 0: `u +0xd0 = op_oef2_a_get(o, 8, i16 cmd +2)`, byte `u
    /// +0xd4` = the low byte of `cmd +4`, `f32 u +0xdc` = `i16 cmd +6 x
    /// [0x00b50198]` (1/256); a null particle set only prints the Virtuos
    /// warning (`dbgPrintf(0x00b589a0)`), else the host's table `host
    /// +0x350` runs its `+0x40` and `+0x44` on `u` and byte `u +0xbb` = 3;
    /// `cmd + 8`. 0x1000: `f32 u +0xdc` times (the larger of
    /// `MsGetChrWidth` and `MsGetChrHeight` of `u +0x18`, height unless
    /// width is above it, both `f32`) `/ [0x00b587d8]` (18, rounded to
    /// `f32`); `cmd + 2`. 0x2000: `op_oef2_a_set(o, 4, i16 cmd +6,
    /// op_oef2_entry_link(op_oef2_a_get(o, 8, i16 cmd +2), i16 cmd +4))`,
    /// `cmd + 8`. Others: `cmd`. Callees through `call`, the two float
    /// ones through `fcall`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_part_init<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{div, load, mul, store};
        let u = unit(mem, host);
        let slot = u32::from(mem.read::<u8>(u.wrapping_add(0x1e)));
        let oef: u32 = mem.read(host.wrapping_add(0x360).wrapping_add(4 * slot));
        let arg =
            |mem: &M, at: u32| i32::from(mem.read::<i16>(cmd.wrapping_add(at))).cast_unsigned();
        let a_get = g(mem, 0x007e_3810);
        match nibble(mem, host) {
            0 => {
                let (n, which) = (arg(mem, 2), mem.read::<u8>(cmd.wrapping_add(4)));
                let set = call(mem, a_get, &[oef, 8, n]);
                mem.write(u.wrapping_add(0xd0), set);
                mem.write(u.wrapping_add(0xd4), which);
                let k: f64 = mem.read(g(mem, 0x00b5_0198));
                let scale = f64::from(mem.read::<i16>(cmd.wrapping_add(6)));
                mem.write(u.wrapping_add(0xdc), store(mul(scale, k)));
                let table: u32 = mem.read(host.wrapping_add(0x350));
                if set == 0 {
                    call(mem, g(mem, 0x0062_f500), &[g(mem, 0x00b5_89a0)]);
                } else {
                    for at in [0x40, 0x44] {
                        let f: u32 = mem.read(table.wrapping_add(at));
                        call(mem, f, &[u]);
                    }
                    mem.write(u.wrapping_add(0xbb), 3u8);
                }
                cmd.wrapping_add(8)
            }
            0x1000 => {
                let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
                let h = load(store(fcall(mem, g(mem, 0x0079_50f0), &[chr])));
                let w = load(store(fcall(mem, g(mem, 0x0079_5a20), &[chr])));
                // fcom: width only when above (ordered)
                let m = if w.partial_cmp(&h) == Some(std::cmp::Ordering::Greater) {
                    w
                } else {
                    h
                };
                let k: f64 = mem.read(g(mem, 0x00b5_87d8));
                let r = load(store(div(m, k)));
                let v = mul(r, load(mem.read(u.wrapping_add(0xdc))));
                mem.write(u.wrapping_add(0xdc), store(v));
                cmd.wrapping_add(2)
            }
            0x2000 => {
                let n = arg(mem, 2);
                let set = call(mem, a_get, &[oef, 8, n]);
                let i = arg(mem, 4);
                let e = call(mem, g(mem, 0x00a6_a320), &[set, i]);
                let at = arg(mem, 6);
                call(mem, g(mem, 0x007e_38a0), &[oef, 4, at, e]);
                cmd.wrapping_add(8)
            }
            _ => cmd,
        }
    }

    /// `opu_sloop(host, cmd)` (`0x00817300`) -> the next command: sets
    /// the loop counter byte `u +0xb4 + (u16 host +0x21a >> 12)` by bits
    /// `0xe00` of the flags, `v` being `u16 cmd +2`. 0: the low byte of
    /// `v`, `cmd + 4`. 0x200: the low byte of
    /// `MsGetDamageInfo(g_EtEff[u16 u +0x14] +0, u +0x18, 0, 0, 0)`, then
    /// `cmd + 4` when its low half is not 0, else the jump `cmd + i16 v`.
    /// 0x400: `op_irandom_p(i16 cmd +4) + v` in bytes (`junk` the three
    /// stack words it reads past its argument, the caller's saved `edi`,
    /// `esi`, `ebx`), `cmd + 6`. 0x600: `ftol(f32 u[i16 v])`, `cmd + 4`.
    /// 0x800: `ftol(opus_cscale_get(u +0x18, i8 host +0x21e) x i16 v)` (the
    /// product rounded to `f32`) as an `i16`, at most `i16 cmd +4` and at
    /// least 1, `cmd + 6`. Others: `cmd`. Callees through `call`, `opus_cscale_get`'s `ST0` through
    /// `fcall`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_sloop<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        junk: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load, mul, store};
        let flags = mem.read::<u16>(host.wrapping_add(0x21a));
        let u = unit(mem, host);
        let v = mem.read::<u16>(cmd.wrapping_add(2));
        let counter = u.wrapping_add(0xb4).wrapping_add(u32::from(flags >> 12));
        let [lo, _] = v.to_le_bytes();
        match flags & 0xe00 {
            0 => {
                mem.write(counter, lo);
                cmd.wrapping_add(4)
            }
            0x200 => {
                let i = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
                let id: u32 = mem.read(EFF.ptr(mem).addr.wrapping_add(i << 5));
                let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
                let d = call(mem, g(mem, 0x0079_a680), &[id, chr, 0, 0, 0]);
                mem.write(counter, d.to_le_bytes()[0]);
                if d & 0xffff != 0 {
                    cmd.wrapping_add(4)
                } else {
                    cmd.wrapping_add(i32::from(v.cast_signed()).cast_unsigned())
                }
            }
            0x400 => {
                let n = i32::from(mem.read::<i16>(cmd.wrapping_add(4))).cast_unsigned();
                let r = crate::rand::bridge::op_irandom_core(mem, [n, junk[0], junk[1], junk[2]]);
                mem.write(counter, r.to_le_bytes()[0].wrapping_add(lo));
                cmd.wrapping_add(6)
            }
            0x600 => {
                let at = u.wrapping_add(i32::from(v.cast_signed()).cast_unsigned());
                let n = ftol(mem, load(mem.read(at)));
                mem.write(counter, n.to_le_bytes()[0]);
                cmd.wrapping_add(4)
            }
            0x800 => {
                let limit = mem.read::<i16>(cmd.wrapping_add(4));
                let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
                let battle = i32::from(mem.read::<i8>(host.wrapping_add(0x21e))).cast_unsigned();
                let s = fcall(mem, g(mem, 0x0080_0c10), &[chr, battle]);
                let p = load(store(mul(s, f64::from(v.cast_signed()))));
                #[allow(clippy::cast_possible_truncation)] // the low half
                let mut c = ftol(mem, p) as i16;
                if c > limit {
                    c = limit;
                }
                if c <= 0 {
                    c = 1;
                }
                mem.write(counter, c.to_le_bytes()[0]);
                cmd.wrapping_add(6)
            }
            _ => cmd,
        }
    }

    /// `0x0080c7c0` (`opu_float_cmp`, ours; `(host, cmd)` -> the next
    /// command): compares two `f32` and goes on when the test holds. With
    /// flag bit 0x400 the unit is `opus_no_get(host +0x300, host +0x220, u16
    /// cmd +4)` and the operands start at `c = cmd + 2`, else the host's
    /// unit and `c = cmd`. Bit 0x800: `b = u[u8 c +4]`, `a = u[u8 c +5]`,
    /// step 6; else `b = u[u16 c +4]`, `a = i16 c +6`, step 8. The nibble
    /// tests, as `fcom`/`fucom` see unordered: 0 `a == b`; 0x1000 `a != b`
    /// or unordered; 0x2000 `a < b` or unordered; 0x3000 `a <= b` or
    /// unordered; 0x4000 `a > b` or unordered; 0x5000 `a >= b` or
    /// unordered; others fail. Holding: `c + step`. Failing: with bit 0x200
    /// the host halts (byte `+0x210` = 1) and `cmd` is returned, else the
    /// jump `cmd + i16 cmd +2`. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_float_cmp<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use std::cmp::Ordering::{Equal, Greater, Less};
        let flags = |mem: &M| mem.read::<u16>(host.wrapping_add(0x21a));
        let (u, c) = if flags(mem) & 0x400 == 0 {
            (unit(mem, host), cmd)
        } else {
            let no = u32::from(mem.read::<u16>(cmd.wrapping_add(4)));
            let args = [host.wrapping_add(0x300), unit(mem, host), no];
            (call(mem, g(mem, 0x0080_1060), &args), cmd.wrapping_add(2))
        };
        let f = flags(mem);
        let at = |mem: &M, off: u32| ffx_bridge::x87::load(mem.read(u.wrapping_add(off)));
        let (a, b, step) = if f & 0x800 == 0 {
            let a = f64::from(mem.read::<i16>(c.wrapping_add(6)));
            let b = at(mem, u32::from(mem.read::<u16>(c.wrapping_add(4))));
            (a, b, 8)
        } else {
            let b = at(mem, u32::from(mem.read::<u8>(c.wrapping_add(4))));
            let a = at(mem, u32::from(mem.read::<u8>(c.wrapping_add(5))));
            (a, b, 6)
        };
        let o = a.partial_cmp(&b);
        let holds = match f & 0xf000 {
            0 => o == Some(Equal),
            0x1000 => o != Some(Equal),
            0x2000 => o != Some(Greater) && o != Some(Equal),
            0x3000 => o != Some(Greater),
            0x4000 => o != Some(Less) && o != Some(Equal),
            0x5000 => o != Some(Less),
            _ => false,
        };
        if holds {
            c.wrapping_add(step)
        } else if f & 0x200 != 0 {
            mem.write(host.wrapping_add(0x210), 1u8);
            cmd
        } else {
            let d = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
            cmd.wrapping_add(d)
        }
    }

    /// `opu_gw_etc(host, cmd)` (`0x0080ca80`) -> the next command: sets the
    /// float `p = f32 u[u8 cmd +2]` by the flag nibble, `e` being the unit's
    /// `g_EtEff[u16 u +0x14]` record: 0 the byte `e +8` (`cmd + 4`); 0x1000
    /// adds `op_random(i16 cmd +4 x [0x00b50198])` (1/256, the argument and
    /// result rounded to `f32`; `cmd + 6`); 0x2000
    /// `MsGetDamageInfo(e +0, u +0x18, 0, 0, 0)` (`cmd + 4`); 0x3000 picks
    /// from the `i16` list at `t = cmd + i16 cmd +4` (`u16 t +0` entries
    /// from `t + 2`) by `op_irandom_p(i16 t +0)`, at most the last (`junk`
    /// its three extra stack words, the caller's saved `ebx`, `edi`,
    /// `esi`; `cmd + 6`); 0x4000 clamps `p` to `[i16 cmd +4, i16 cmd +6]`
    /// (the low bound only when above `p`, the high one only when below the
    /// result, as `fcom` sees unordered; `cmd + 8`); others `cmd`. Callees
    /// through `call`, `op_random`'s `ST0` through `fcall`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_gw_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        junk: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store};
        use std::cmp::Ordering::{Greater, Less};
        let u = unit(mem, host);
        let i = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
        let e = EFF.ptr(mem).addr.wrapping_add(i << 5);
        let p = u.wrapping_add(u32::from(mem.read::<u8>(cmd.wrapping_add(2))));
        let half = |mem: &M, at: u32| mem.read::<i16>(cmd.wrapping_add(at));
        // fild, rounded by the store
        let int = |n: i32| store(f64::from(n));
        match nibble(mem, host) {
            0 => {
                let b = mem.read::<u8>(e.wrapping_add(8));
                mem.write(p, int(i32::from(b)));
                cmd.wrapping_add(4)
            }
            0x1000 => {
                let k: f64 = mem.read(g(mem, 0x00b5_0198));
                let x = store(mul(f64::from(half(mem, 4)), k));
                let r = load(store(fcall(mem, g(mem, 0x007e_6700), &[x])));
                let v = store(add(load(mem.read(p)), r));
                mem.write(p, v);
                cmd.wrapping_add(6)
            }
            0x2000 => {
                let id: u32 = mem.read(e);
                let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
                let d = call(mem, g(mem, 0x0079_a680), &[id, chr, 0, 0, 0]);
                mem.write(p, int(d.cast_signed()));
                cmd.wrapping_add(4)
            }
            0x3000 => {
                let t = cmd.wrapping_add(i32::from(half(mem, 4)).cast_unsigned());
                let n = mem.read::<i16>(t);
                let wide = i32::from(n).cast_unsigned();
                let r =
                    crate::rand::bridge::op_irandom_core(mem, [wide, junk[0], junk[1], junk[2]]);
                #[allow(clippy::cast_possible_truncation)] // the low half
                let mut c = r as i16;
                if c >= n {
                    c = n.wrapping_sub(1);
                }
                let at = t
                    .wrapping_add(2)
                    .wrapping_add(i32::from(c).cast_unsigned().wrapping_mul(2));
                let v = mem.read::<i16>(at);
                mem.write(p, int(i32::from(v)));
                cmd.wrapping_add(6)
            }
            0x4000 => {
                let x = load(mem.read(p));
                let (lo, hi) = (f64::from(half(mem, 4)), f64::from(half(mem, 6)));
                let m = if lo.partial_cmp(&x) == Some(Greater) {
                    lo
                } else {
                    x
                };
                let r = if hi.partial_cmp(&m) == Some(Less) {
                    hi
                } else {
                    m
                };
                mem.write(p, store(r));
                cmd.wrapping_add(8)
            }
            _ => cmd,
        }
    }

    /// `opus_chara_center_check(chr, on)` (`0x00800b40`): 0 when `on` is
    /// 0, else 1 when `MsGetChrFly(chr)` or `MsBattleWater()` is not 0
    /// (callees through `call`).
    pub fn opus_chara_center_check<M: Mem>(
        mem: &mut M,
        chr: u32,
        on: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if on == 0 {
            return 0;
        }
        if call(mem, g(mem, 0x0079_4f70), &[chr]) != 0 {
            return 1;
        }
        u32::from(call(mem, g(mem, 0x0078_21e0), &[]) != 0)
    }

    /// `opu_death(host, cmd)` (`0x00815f10`) -> the next command: with a
    /// flag nibble `cmd`; else `cmd + 2` when
    /// `MsGetChrByte3f4_00795a60(u8 u +0x18)` is not 0, and when it is 0 the
    /// host halts (byte `+0x210` = 1) on `cmd` (callee through `call`).
    pub fn opu_death<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if nibble(mem, host) != 0 {
            return cmd;
        }
        let chr = u32::from(mem.read::<u8>(unit(mem, host).wrapping_add(0x18)));
        if call(mem, g(mem, 0x0079_5a60), &[chr]) != 0 {
            cmd.wrapping_add(2)
        } else {
            mem.write(host.wrapping_add(0x210), 1u8);
            cmd
        }
    }

    /// `opu_sys(host, cmd)` (`0x00817470`) -> `cmd + 2`: by the flag
    /// nibble, on the unit's byte `+0xb2`: 0 `MsEffectWaitFlag(1)` and sets
    /// bit 0; 0x1000 `MsEffectWaitFlag(0)` and clears it; 0x2000 / 0x3000
    /// set / clear bit 1; 0x4000 / 0x5000 set / clear bit 2 (callee through
    /// `call`).
    pub fn opu_sys<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let flags = unit(mem, host).wrapping_add(0xb2);
        let (set, clear) = match nibble(mem, host) {
            0 => {
                call(mem, g(mem, 0x0078_8b20), &[1]);
                (1, 0)
            }
            0x1000 => {
                call(mem, g(mem, 0x0078_8b20), &[0]);
                (0, 1)
            }
            0x2000 => (2, 0),
            0x3000 => (0, 2),
            0x4000 => (4, 0),
            0x5000 => (0, 4),
            _ => return cmd.wrapping_add(2),
        };
        let b = mem.read::<u8>(flags);
        mem.write(flags, (b | set) & !clear);
        cmd.wrapping_add(2)
    }

    /// `opu_v_scale(host, cmd)` (`0x00819af0`) -> `cmd + 4`: scales unit
    /// floats by `s`, picked by flag bits `0xe000`: 0
    /// `opus_cscale_get(u8 u +0x18, i8 host +0x21e)`; 0x2000 1.0 outside
    /// battle (byte `+0x21e` 0), else `|f32 host +4|` after
    /// `MsGetChrScale(u8 u +0x18, host)` (a NaN kept as it is); 0x4000
    /// `opus_chr_width(u8 u +0x18, i8 host +0x21e)`; others the bits of the
    /// `cmd` pointer itself as an `f32` (the original reads its argument
    /// slot). Each `f32`. With flag 0x1000 the bits 4, 5 and 6 of `m = u16
    /// cmd +2` scale `u +0x40`, `+0x44`, `+0x48` by `s x 16` (rounded to
    /// `f32`), else bits 0..7 scale `u +0x30`..`+0x4c`. Callees through
    /// `call`, the floats through `fcall`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_v_scale<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{load, mul, store};
        let m = mem.read::<u16>(cmd.wrapping_add(2));
        let u = unit(mem, host);
        let chr = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let battle = i32::from(mem.read::<i8>(host.wrapping_add(0x21e))).cast_unsigned();
        let flags = |mem: &M| mem.read::<u16>(host.wrapping_add(0x21a));
        let s = match flags(mem) & 0xe000 {
            0 => load(store(fcall(mem, g(mem, 0x0080_0c10), &[chr, battle]))),
            0x2000 if battle == 0 => 1.0,
            0x2000 => {
                call(mem, g(mem, 0x0079_5720), &[chr, host]);
                let x = load(store(load(mem.read(host.wrapping_add(4)))));
                if x < 0.0 { load(store(-x)) } else { x }
            }
            0x4000 => load(store(fcall(mem, g(mem, 0x0080_0cd0), &[chr, battle]))),
            _ => load(cmd),
        };
        let scale = |mem: &mut M, at: u32, s: f64, first: bool| {
            let a = u.wrapping_add(at);
            let v = load(mem.read(a));
            let r = if first { mul(s, v) } else { mul(v, s) };
            mem.write(a, store(r));
        };
        let bit = |k: u32| u32::from(m) >> k & 1 != 0;
        if flags(mem) & 0x1000 == 0 {
            for k in 0..8 {
                if bit(k) {
                    scale(mem, 0x30 + 4 * k, s, k == 7);
                }
            }
        } else {
            let s16: f64 = mem.read(g(mem, 0x00b3_13a0));
            let s16 = load(store(mul(s, s16)));
            for k in 4..7 {
                if bit(k) {
                    scale(mem, 0x30 + 4 * k, s16, k == 6);
                }
            }
        }
        cmd.wrapping_add(4)
    }

    /// `opu_r_en(host, cmd)` (`0x00817f50`) -> the next command: first the
    /// host's `f32 +0` = `Ch_GetRot(g_EtChrHandles[u8 u +0x18])` when that
    /// handle is set, and `r = (0x400 - ftol(f32 (host +0 x 2048 /
    /// 3.141592...))) & 0xfff` as an `f32`. By the flag nibble: 0 `u +0x30`,
    /// `+0x34`, `+0x38` = 0, `r`, 0; 0x1000 `u +0x48` = `r`; 0x2000 `u
    /// +0x34` = `r`; 0x3000 `MsSetChrDirTarget(byte g_EtEff[u16 u +0x14]
    /// +0x18, 2, 0, u8 u +0x18, 0, 0)`; all `cmd + 2`. 0x4000:
    /// `op_et_eff_target_center(&g_EtEff[u16 u +0x14], host, 0)`, then
    /// `u[u8 cmd +2]` = `op_deg_get(u +0x90, u +0x98, host +0, host +8, host
    /// +0x10)` and with flag 0x800 `u[u8 cmd +3]` = `host +0x10`; 0x6000
    /// `r = opus_ffxdeg_to_normaldeg(MsGetChrBtlDir(u8 u +0x18))` and on as
    /// 0x5000: `u[u16 cmd +2]` = `r`; these `cmd + 4`. Others `cmd`.
    /// Callees through `call`, the floats through `fcall`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_r_en<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{div, ftol, load, mul, store};
        let u = unit(mem, host);
        let first = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(first).get(mem);
        if h != 0 {
            let rot = fcall(mem, g(mem, 0x0082_ace0), &[h]);
            mem.write(host, store(rot));
        }
        let k: f64 = mem.read(g(mem, 0x00b5_1bc0));
        let pi: f64 = mem.read(g(mem, 0x00b5_82f0));
        let a = load(store(div(mul(load(mem.read(host)), k), pi)));
        let turn = 0x400u32.wrapping_sub(ftol(mem, a)) & 0xfff;
        #[allow(clippy::cast_precision_loss)] // 12 bits: exact
        let mut r = store(f64::from(turn));
        let field =
            |mem: &M, at: u32| u.wrapping_add(u32::from(mem.read::<u8>(cmd.wrapping_add(at))));
        let eff = |mem: &M| {
            let i = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
            crate::et::bridge::EFF.ptr(mem).addr.wrapping_add(i << 5)
        };
        let chr = |mem: &M| u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        match nibble(mem, host) {
            0 => {
                mem.write(u.wrapping_add(0x30), 0u32);
                mem.write(u.wrapping_add(0x34), r);
                mem.write(u.wrapping_add(0x38), 0u32);
                cmd.wrapping_add(2)
            }
            0x1000 => {
                mem.write(u.wrapping_add(0x48), r);
                cmd.wrapping_add(2)
            }
            0x2000 => {
                mem.write(u.wrapping_add(0x34), r);
                cmd.wrapping_add(2)
            }
            0x3000 => {
                let id = u32::from(mem.read::<u8>(eff(mem).wrapping_add(0x18)));
                let c = chr(mem);
                call(mem, g(mem, 0x0079_5eb0), &[id, 2, 0, c, 0, 0]);
                cmd.wrapping_add(2)
            }
            0x4000 => {
                let e = eff(mem);
                call(mem, g(mem, 0x0081_b3e0), &[e, host, 0]);
                let f = |mem: &M, at: u32| mem.read::<u32>(at);
                let args = [
                    f(mem, u.wrapping_add(0x90)),
                    f(mem, u.wrapping_add(0x98)),
                    f(mem, host),
                    f(mem, host.wrapping_add(8)),
                    host.wrapping_add(0x10),
                ];
                let d = store(fcall(mem, g(mem, 0x007e_7ed0), &args));
                let to = field(mem, 2);
                let len_to = field(mem, 3);
                mem.write(to, d);
                if mem.read::<u16>(host.wrapping_add(0x21a)) & 0x800 != 0 {
                    let len: u32 = mem.read(host.wrapping_add(0x10));
                    mem.write(len_to, len);
                }
                cmd.wrapping_add(4)
            }
            n @ (0x5000 | 0x6000) => {
                if n == 0x6000 {
                    let c = chr(mem);
                    let dir = store(fcall(mem, g(mem, 0x0079_48a0), &[c]));
                    r = store(fcall(mem, g(mem, 0x0081_b170), &[dir]));
                }
                let at = u.wrapping_add(u32::from(mem.read::<u16>(cmd.wrapping_add(2))));
                mem.write(at, r);
                cmd.wrapping_add(4)
            }
            _ => cmd,
        }
    }

    /// `opu_init(no, data, owner, obj, n, _, show)` (`0x007fda80`, called by
    /// `opu_et_init`): sets up opus object `obj` as number `no`
    /// (`g_OpusBattle[u16 no]`; a non-negative `show` to `0x023227b4`, and
    /// `show` to `0x011a0084` for number 1): `obj`'s first 0x100 bytes
    /// cleared (`memset`), four `u16 n`-entry `i16` arrays after them
    /// (`+0x10`, `+0x14`, `+0x18`, `+0x1c`; their first `n & !7` entries
    /// -1), the unit at `+0x20` behind them, `+0x24` = `obj`, `u16 +0` =
    /// `no`, `+2` = 0x8000, `+8` = `data`, `u16 +0x34` = `n`, `+0x36` =
    /// 0x10001; `owner` gets a slot among the 16 words at `+0x60` (its own,
    /// else the first free one; 0xff when none or `owner` is 0; the clear
    /// has just emptied them, so a set `owner` always gets slot 0); then
    /// `opus_osu_init_group(no, n)` (after the first array's first entry is
    /// 0), `opus_osu_init(unit)` (`g_OpusLastUnit` = it), the unit's `+0` =
    /// `data`, `u16 +0x1a` = 0, byte `+0x1e` = the slot, `u16 +0x14` =
    /// `opus_get_eff_status_index(show, slot, 1)` = `i`, `g_EtEff[i] +4` =
    /// the unit and its byte `+0x18` = record byte `+0x10` (`+0x18` when
    /// that is 0xff). Callees through `call`. Returns `obj`.
    #[allow(clippy::too_many_arguments, clippy::cast_possible_truncation)]
    pub fn opu_init<M: Mem>(
        mem: &mut M,
        no: u32,
        data: u32,
        owner: u32,
        obj: u32,
        n: u32,
        show: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let no16 = no & 0xffff;
        mem.write(OPUS_BATTLE.ptr(mem).addr.wrapping_add(no16 << 2), obj);
        if show.cast_signed() >= 0 {
            mem.write(g(mem, 0x0232_27b4), show);
        }
        if no16 == 1 {
            mem.write(g(mem, 0x011a_0084), show);
        }
        call(mem, g(mem, 0x0094_964c), &[obj, 0, 0x100]);
        let len = (n & 0xffff) * 2;
        let arrays = [0u32, 1, 2, 3].map(|k| obj.wrapping_add(0x100 + k * len));
        let unit = obj.wrapping_add(0x100 + 4 * len);
        for (k, a) in (0u32..).zip(arrays) {
            mem.write(obj.wrapping_add(0x10 + 4 * k), a);
        }
        mem.write(obj.wrapping_add(0x20), unit);
        for _ in 0..len / 16 {
            for k in 0..4 {
                let at = obj.wrapping_add(0x10 + 4 * k);
                let a: u32 = mem.read(at);
                mem.write(a, [u32::MAX; 4]);
            }
            for k in 0..4 {
                let at = obj.wrapping_add(0x10 + 4 * k);
                let a: u32 = mem.read(at);
                mem.write(at, a.wrapping_add(0x10));
            }
        }
        mem.write(obj.wrapping_add(0x18), arrays[2]);
        mem.write(obj.wrapping_add(0x1c), arrays[3]);
        mem.write(obj.wrapping_add(2), 0x8000u16);
        mem.write(obj.wrapping_add(0x14), arrays[1]);
        mem.write(obj.wrapping_add(0x34), n as u16);
        mem.write(obj.wrapping_add(0x10), arrays[0]);
        mem.write(obj.wrapping_add(0x24), obj);
        mem.write(obj, no as u16);
        mem.write(obj.wrapping_add(0x36), 0x0001_0001u32);
        mem.write(obj.wrapping_add(8), data);
        let slots = obj.wrapping_add(0x60);
        let slot = if owner == 0 {
            0xff
        } else if let Some(k) = (0..16u32).find(|&k| mem.read::<u32>(slots + 4 * k) == owner) {
            k
        } else if let Some(k) = (0..16u32).find(|&k| mem.read::<u32>(slots + 4 * k) == 0) {
            mem.write(slots + 4 * k, owner);
            k
        } else {
            0xff
        };
        let first: u32 = mem.read(obj.wrapping_add(0x10));
        mem.write(first, 0u16);
        call(mem, g(mem, 0x0080_1480), &[no, n]);
        let unit: u32 = mem.read(obj.wrapping_add(0x20));
        call(mem, g(mem, 0x0080_1200), &[unit]);
        mem.write(g(mem, 0x012a_12f8), unit);
        let d: u32 = mem.read(obj.wrapping_add(8));
        mem.write(unit, ffx_system::stub::ptr32(d));
        mem.write(unit.wrapping_add(0x1a), 0u16);
        mem.write(unit.wrapping_add(0x1e), slot as u8);
        let i = call(mem, g(mem, 0x007f_eba0), &[show, slot, 1]);
        mem.write(unit.wrapping_add(0x14), i as u16);
        let rec = EFF.ptr(mem).addr.wrapping_add(i << 5);
        mem.write(rec.wrapping_add(4), unit);
        let mut c: u8 = mem.read(rec.wrapping_add(0x10));
        if c == 0xff {
            c = mem.read(rec.wrapping_add(0x18));
        }
        mem.write(unit.wrapping_add(0x18), c);
        obj
    }

    /// `opus_unit_on(host, cmd)` (`0x0080af70`, opus command table
    /// `0x00c4903c`) -> the next command. The host's unit `u` (`+0x220`)
    /// names its effect record `r` = `g_EtEff[u16 u +0x14]`. By the top
    /// flag bits (`u16 host +0x21a & 0xc000`): 0 loads `r`'s byte `+8`
    /// units, each as [`opu_osu_load`] does (the host itself with `u16
    /// host +0x212` set), with byte `+0x18` = `r`'s byte `+0x10 + i` and
    /// `u16 +0x1a` = `base + i`, `base` = the host's serial `u16 +0x302`
    /// rounded up to the next 16 (0x8000 from 0xff80); the serial is
    /// `base + 0x10` after, and the result `cmd + 4`. 0x4000:
    /// [`opu_osu_load`], then the loaded unit's `u16 +0x1a` = `u16 cmd +4`
    /// plus `u`'s `u16 +0x1a`; `cmd + 6`. Others: `cmd`. As in
    /// [`opu_osu_load`] the `LoadedByOPU_UNIT` record for
    /// `yonExtMapRegister` (a bare `ret`) is left out.
    #[allow(clippy::many_single_char_names)]
    pub fn opus_unit_on<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        let i = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
        let rec = EFF.ptr(mem).addr.wrapping_add(i << 5);
        match mem.read::<u16>(host.wrapping_add(0x21a)) & 0xc000 {
            0 => {}
            0x4000 => {
                opu_osu_load(mem, host, cmd, call);
                let first: u32 = mem.read(host);
                let v = mem
                    .read::<u16>(cmd.wrapping_add(4))
                    .wrapping_add(mem.read(u.wrapping_add(0x1a)));
                mem.write(first.wrapping_add(0x1a), v);
                return cmd.wrapping_add(6);
            }
            _ => return cmd,
        }
        let serial = host.wrapping_add(0x302);
        let mut base = mem.read::<u16>(serial).wrapping_add(0x10) & 0xfff0;
        if base >= 0xff80 {
            base = 0x8000;
        }
        let mut k = 0u16;
        while u32::from(k) < u32::from(mem.read::<u8>(rec.wrapping_add(8))) {
            let o = if mem.read::<u16>(host.wrapping_add(0x212)) != 0 {
                host
            } else {
                let src = unit(mem, host);
                let list = u32::from(
                    (mem.read::<i16>(host.wrapping_add(0x21a)) >> 12).cast_unsigned() & 3,
                );
                let set = host.wrapping_add(0x300);
                #[allow(clippy::cast_possible_truncation)] // the u16 index in ax
                let n = (opus_prog_on(mem, set, list) as u16).cast_signed();
                let off = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
                osu_take(mem, set, n, src, cmd.wrapping_add(off), call)
            };
            mem.write(host, o);
            let b: u8 = mem.read(rec.wrapping_add(0x10 + u32::from(k)));
            mem.write(o.wrapping_add(0x18), b);
            mem.write(o.wrapping_add(0x1a), k.wrapping_add(base));
            k += 1;
        }
        mem.write(serial, base.wrapping_add(0x10));
        cmd.wrapping_add(4)
    }

    /// `opu_v_en(host, cmd)` (`0x00818ed0`, opus command table
    /// `0x00c49044`) -> `cmd + 2`: the host's vector (`f32 +0`..`+8`) from
    /// the unit `u`'s character (`c` = byte `u +0x18`) by the flag nibble:
    /// 0 its position (`opus_get_chr_pos(c, host)`); 0x1000 the position,
    /// lowered by half its height (`opus_cheight_get(c, i8 host +0x21e)`
    /// x 0.5 as an `f32`) when that byte is set and the character neither
    /// flies (`MsGetChrFly`) nor is in water (`MsBattleWater`), then
    /// `opus_get_bone_vertex(host + 0x10, g_EtChrHandles[c], 0x8003)`
    /// and `x`, `z` from that; 0x2000/0x5000 `op_et_eff_target_center(
    /// &g_EtEff[u16 u +0x14], host, 0/1)`; 0x3000 `MsGetChrBtlPos(c,
    /// host)`; 0x4000 the position raised by half the height when
    /// `opus_chara_center_check(c, c)` (its second word is the height
    /// call's first: 0 for character 0); 0x6000/0x7000 bone vertex
    /// 0x8010/0x8011; 0x8000 `op_et_battle_target_center(host)`. Those
    /// set `u +0x40` = 16 x the vector (`op_xyz_scale`; 0x1000 too);
    /// every nibble then `u +0x90` = `u +0x40` / 16. Callees through
    /// `call`, the height through `fcall`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_v_en<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, round_trip, store, sub};
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        let u = unit(mem, host);
        let chr = |mem: &M| u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let low = |mem: &M| i32::from(mem.read::<i8>(host.wrapping_add(0x21e))).cast_unsigned();
        let handle = |mem: &M| {
            crate::et::bridge::CHR_HANDLES
                .ptr(mem)
                .at(chr(mem))
                .get(mem)
        };
        let half = |mem: &mut M, fcall: &mut dyn FnMut(&mut M, u32, &[u32]) -> f64| {
            let args = [chr(mem), low(mem)];
            let h = fcall(mem, g(mem, 0x0080_0b70), &args);
            store(mul(h, mem.read(g(mem, 0x00b9_2220))))
        };
        let y = host.wrapping_add(4);
        let scaled = match flags & 0xf000 {
            0 => {
                call(mem, g(mem, 0x0081_b1e0), &[chr(mem), host]);
                true
            }
            0x1000 => {
                let h = half(mem, fcall);
                call(mem, g(mem, 0x0081_b1e0), &[chr(mem), host]);
                if mem.read::<u8>(host.wrapping_add(0x21e)) != 0
                    && call(mem, g(mem, 0x0079_4f70), &[chr(mem)]) == 0
                    && call(mem, g(mem, 0x0078_21e0), &[]) == 0
                {
                    let v = store(sub(load(mem.read(y)), load(h)));
                    mem.write(y, v);
                }
                let bone = host.wrapping_add(0x10);
                call(mem, g(mem, 0x0080_54d0), &[bone, handle(mem), 0x8003]);
                let x = round_trip(mem.read(bone));
                mem.write(host, x);
                let z = round_trip(mem.read(host.wrapping_add(0x18)));
                mem.write(host.wrapping_add(8), z);
                let s: u32 = mem.read(g(mem, 0x00b9_231c));
                call(mem, g(mem, 0x007e_a850), &[u.wrapping_add(0x40), host, s]);
                false
            }
            n @ (0x2000 | 0x5000) => {
                let i = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
                let eff = EFF.ptr(mem).addr.wrapping_add(i << 5);
                call(
                    mem,
                    g(mem, 0x0081_b3e0),
                    &[eff, host, u32::from(n == 0x5000)],
                );
                true
            }
            0x3000 => {
                call(mem, g(mem, 0x0079_48e0), &[chr(mem), host]);
                true
            }
            0x4000 => {
                call(mem, g(mem, 0x0081_b1e0), &[chr(mem), host]);
                let h = half(mem, fcall);
                let c = chr(mem);
                if call(mem, g(mem, 0x0080_0b40), &[c, c]) != 0 {
                    let v = store(add(load(mem.read(y)), load(h)));
                    mem.write(y, v);
                }
                true
            }
            n @ (0x6000 | 0x7000) => {
                let bone = 0x8010 | u32::from(n >> 12) & 1;
                call(mem, g(mem, 0x0080_54d0), &[host, handle(mem), bone]);
                true
            }
            0x8000 => {
                call(mem, g(mem, 0x0081_b300), &[host]);
                true
            }
            _ => false,
        };
        if scaled {
            let s: u32 = mem.read(g(mem, 0x00b9_231c));
            call(mem, g(mem, 0x007e_a850), &[u.wrapping_add(0x40), host, s]);
        }
        let s: u32 = mem.read(g(mem, 0x00b5_8354));
        call(
            mem,
            g(mem, 0x007e_a850),
            &[u.wrapping_add(0x90), u.wrapping_add(0x40), s],
        );
        cmd.wrapping_add(2)
    }

    /// `0x008171a0` (`opu_trail_point`, ours; opus command table
    /// `0x00c49014`) -> `cmd + 4`: the unit `u` (`+0x220`) adds its point
    /// (`u + 0x90`) to a trail unless `u16 host +0x212` is set. The unit
    /// `t` = `opus_no_get(host + 0x300, u, u16 cmd +2)`: of draw kind 0xa
    /// (byte `t +0xbb`), `op_trail_add(t +0xbc, u + 0x90, u == t, u == t ?
    /// 0 : (u - host +0x320) >> 8)`; of kind 0x11, `op_trail_add(u +0xbc,
    /// u + 0x90, 0, 0)`.
    pub fn opu_trail_point(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        let u = unit(mem, host);
        let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        let t = opus_no_get(mem, host.wrapping_add(0x300), u, no);
        let on = mem.read::<u16>(host.wrapping_add(0x212)) == 0;
        let point = u.wrapping_add(0x90);
        match mem.read::<u8>(t.wrapping_add(0xbb)) {
            0xa => {
                let (fill, life) = if u == t {
                    (1, 0)
                } else {
                    let units: u32 = mem.read(host.wrapping_add(0x320));
                    (0, u.wrapping_sub(units) >> 8)
                };
                if on {
                    let ring: u32 = mem.read(t.wrapping_add(0xbc));
                    crate::small::bridge::op_trail_add(mem, ring, point, fill, life);
                }
            }
            0x11 if on => {
                let ring: u32 = mem.read(u.wrapping_add(0xbc));
                crate::small::bridge::op_trail_add(mem, ring, point, 0, 0);
            }
            _ => {}
        }
        cmd.wrapping_add(4)
    }

    /// `opu_et_init()` (`0x007fd760`, from `op_init_ffx`): the effect
    /// records reset (`op_et_eff_init_all`, with `0x012e9650` = 1 meanwhile),
    /// opus object 0 at `*0x00c88a90` set up with 0x200 entries
    /// (`opu_init(0, 0x012e9654, 0x012e9660, obj, 0x200, -1)`), its heap
    /// `+0xa8` = `op_heap_init(0x0113fe10, 0x60000)`, `+0x58` = 0, `+0x28`
    /// and `+0x2c` = `0x00c498dc`'s word; `opus_prog_off(obj, 0)`, the
    /// first unit's `+0` = 0 and `u16 +0x38` = 0. `yonExtMapDeleteId(0)` is
    /// a bare `ret`: left out. Callees of the callees through `call`.
    /// Returns 0 (`eax` as left).
    pub fn opu_et_init<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let obj: u32 = mem.read(g(mem, 0x00c8_8a90));
        let busy = g(mem, 0x012e_9650);
        mem.write(busy, 1u8);
        crate::et::bridge::op_et_eff_init_all(mem);
        let (data, owner) = (g(mem, 0x012e_9654), g(mem, 0x012e_9660));
        let o = opu_init(mem, 0, data, owner, obj, 0x200, u32::MAX, call);
        let heap = crate::op::bridge::op_heap_init(mem, g(mem, 0x0113_fe10), 0x60000);
        mem.write(o.wrapping_add(0xa8), heap);
        mem.write(o.wrapping_add(0x58), 0u32);
        let w: u32 = mem.read(g(mem, 0x00c4_98dc));
        mem.write(o.wrapping_add(0x28), w);
        let w: u32 = mem.read(g(mem, 0x00c4_98dc));
        mem.write(o.wrapping_add(0x2c), w);
        mem.write(busy, 0u8);
        crate::op::bridge::opus_prog_off(mem, o, 0, call);
        let first: u32 = mem.read(o.wrapping_add(0x20));
        mem.write(first, ffx_system::stub::ptr32(0));
        mem.write(o.wrapping_add(0x38), 0u16);
        0
    }

    /// `0x00808380` (`opu_gs_etc`, ours; opus command table `0x00c49174`)
    /// -> the next command, by the flag nibble (`u` = the host's unit
    /// `+0x220`): 0 `host +0x354` = `0x007e5390(host +0x354, op_oef2_a_get(
    /// host[0x360 + 4 group], 2, i16 cmd +2), n, word at cmd + u16 cmd +6,
    /// i16 cmd +8, ftol(u f32 +0x40), ftol(u f32 +0x44), u +0xc8, u8 u
    /// +0xc2, u8 u +0xc3)` with `n` = `u16 cmd +4` (`(n & 0x3fff) + 0x1a00`
    /// when its top bit is set), `cmd + 10`; 0x1000
    /// `opus_copy_buffer_event(host, u, i16 cmd +2)`, `cmd + 4`; 0x2000
    /// `0x007eadc0(0, 0)`, `cmd + 2`; 0x3000 `op_zbuf_clear(0)` and
    /// `opus_ffxfocus_off()`, `cmd + 2`; 0x4000 `0x007e4420(cmd + 4, i16 cmd
    /// +2 << 16)` and 0x5000 `op_gs_store` alike, `cmd + 0x12`; 0x6000
    /// `op_func_final_set(op_zbuf_clear, 0, 0, 1)`, `cmd + 2`; 0x7000
    /// `op_zbuf_clear(ftol(f32 u + i16 cmd +2) << 8)`, `cmd + 4`; others
    /// `cmd`. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_gs_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load};
        let u = unit(mem, host);
        let i16_at =
            |mem: &M, at: u32| i32::from(mem.read::<i16>(cmd.wrapping_add(at))).cast_unsigned();
        let ftol_at = |mem: &M, at: u32| ftol(mem, load(mem.read(at)));
        let zbuf = g(mem, 0x007e_c7e0);
        match nibble(mem, host) {
            0 => {
                let mut n: u16 = mem.read(cmd.wrapping_add(4));
                if n & 0x8000 != 0 {
                    n = (n & 0x3fff) + 0x1a00;
                }
                let off = u32::from(mem.read::<u16>(cmd.wrapping_add(6)));
                let group = u32::from(mem.read::<u8>(u.wrapping_add(0x1e)));
                let oef: u32 = mem.read(host.wrapping_add(0x360 + 4 * group));
                let no = i16_at(mem, 2);
                let a = call(mem, g(mem, 0x007e_3810), &[oef, 2, no]);
                let word: u32 = mem.read(cmd.wrapping_add(off));
                let x = ftol_at(mem, u.wrapping_add(0x40));
                let y = ftol_at(mem, u.wrapping_add(0x44));
                let args = [
                    mem.read(host.wrapping_add(0x354)),
                    a,
                    u32::from(n),
                    word,
                    i16_at(mem, 8),
                    x,
                    y,
                    mem.read(u.wrapping_add(0xc8)),
                    u32::from(mem.read::<u8>(u.wrapping_add(0xc2))),
                    u32::from(mem.read::<u8>(u.wrapping_add(0xc3))),
                ];
                let r = call(mem, g(mem, 0x007e_5390), &args);
                mem.write(host.wrapping_add(0x354), r);
                cmd.wrapping_add(10)
            }
            0x1000 => {
                let no = i16_at(mem, 2);
                call(mem, g(mem, 0x0080_8a60), &[host, u, no]);
                cmd.wrapping_add(4)
            }
            0x2000 => {
                call(mem, g(mem, 0x007e_adc0), &[0, 0]);
                cmd.wrapping_add(2)
            }
            0x3000 => {
                call(mem, zbuf, &[0]);
                call(mem, g(mem, 0x0080_abe0), &[]);
                cmd.wrapping_add(2)
            }
            n @ (0x4000 | 0x5000) => {
                let f = g(
                    mem,
                    if n == 0x4000 {
                        0x007e_4420
                    } else {
                        0x007e_4540
                    },
                );
                let hi = i16_at(mem, 2) << 16;
                call(mem, f, &[cmd.wrapping_add(4), hi]);
                cmd.wrapping_add(0x12)
            }
            0x6000 => {
                call(mem, g(mem, 0x007e_b410), &[zbuf, 0, 0, 1]);
                cmd.wrapping_add(2)
            }
            0x7000 => {
                let z = ftol_at(mem, u.wrapping_add(i16_at(mem, 2)));
                call(mem, zbuf, &[z << 8]);
                cmd.wrapping_add(4)
            }
            _ => cmd,
        }
    }

    /// `0x00814520` (`opu_bind_etc`, ours; opus command table
    /// `0x00c49010`) -> the next command, by `i16 cmd +2` (`u` = the
    /// host's unit, `c` = byte `u +0x18`, `h` = `g_EtChrHandles[c]`): 1
    /// and 2 step the host's byte `+0x33a` up (to 0xfa at most) and down
    /// (not below 0); 3 branches when it is 10 or more; 4 `host +0..+0xc`
    /// = r0 with `f32 +4` = `u +0x34`, then `op_et_weapon_effect_on(c, u +
    /// 0x90, host)`; 5 `op_et_bindeff_send_signal(h, u16 cmd +4)` (`cmd +
    /// 6`); 6..0xb the bind effect's draw on/off, off, on, flags
    /// save/restore of `h`; 0xc `u +0xb3` = 0x7f; 0xd branches when the
    /// weapon (`opus_get_weapon_index_by_no(c)`) exists and its counter
    /// (`g_OpusWeaponCounter`, signed) is above 0x20; 0xe..0x12 branch when
    /// `h` is hidden (`Ch_GetHide`), has no status record
    /// (`opus_get_chr_status_index(h, 0, 0)` negative) or the record's bind
    /// flags (`g_EtChr[i] +0xc` | `g_EtBindDefault`) meet `0x4020` |
    /// 0x100/0x200/0x2000/0x1000/0x800; 0x13/0x14 branch on bit 0/1 of the
    /// byte at `0x011a006e`. A branch is `cmd + i16 cmd +4`, a test that
    /// fails `cmd + 6`, the plain steps `cmd + 4`; other values `cmd`.
    /// Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_bind_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        let c = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(c).get(mem);
        let jump = |mem: &M| {
            cmd.wrapping_add(i32::from(mem.read::<i16>(cmd.wrapping_add(4))).cast_unsigned())
        };
        let next = cmd.wrapping_add(4);
        let fail = cmd.wrapping_add(6);
        let count = host.wrapping_add(0x33a);
        let mut on = |mem: &mut M, f: u32| {
            let f = g(mem, f);
            call(mem, f, &[h]);
            next
        };
        let bits: u32 = match mem.read::<i16>(cmd.wrapping_add(2)) {
            1 => {
                let n = mem.read::<u8>(count).wrapping_add(1);
                mem.write(count, n.min(0xfa));
                return next;
            }
            2 => {
                let n: u8 = mem.read(count);
                if n != 0 {
                    mem.write(count, n - 1);
                }
                return next;
            }
            3 => {
                return if mem.read::<u8>(count) >= 10 {
                    jump(mem)
                } else {
                    fail
                };
            }
            4 => {
                let r0: [u32; 4] = mem.read(crate::ppp::bridge::R0.ptr(mem).addr);
                mem.write(host, r0);
                let y = ffx_bridge::x87::round_trip(mem.read(u.wrapping_add(0x34)));
                mem.write(host.wrapping_add(4), y);
                let c = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
                let f = g(mem, 0x007f_ca80);
                call(mem, f, &[c, u.wrapping_add(0x90), host]);
                return next;
            }
            5 => {
                let bits = u32::from(mem.read::<u16>(cmd.wrapping_add(4)));
                let f = g(mem, 0x007f_bef0);
                call(mem, f, &[h, bits]);
                return fail;
            }
            6 => return on(mem, 0x007f_bab0),
            7 => return on(mem, 0x007f_ba40),
            8 => return on(mem, 0x007f_bb90),
            9 => return on(mem, 0x007f_b290),
            0xa => return on(mem, 0x007f_c190),
            0xb => return on(mem, 0x007f_c130),
            0xc => {
                mem.write(u.wrapping_add(0xb3), 0x7fu8);
                return next;
            }
            0xd => {
                let f = g(mem, 0x0080_1030);
                let i = call(mem, f, &[c]);
                let w = WEAPONS.ptr(mem).addr.wrapping_add(i.wrapping_mul(4));
                let counter = g(mem, 0x0119_fefc).wrapping_add(i);
                return if mem.read::<u32>(w) != 0 && mem.read::<i8>(counter) > 0x20 {
                    jump(mem)
                } else {
                    fail
                };
            }
            0xe => 0x100,
            0xf => 0x200,
            0x10 => 0x2000,
            0x11 => 0x1000,
            0x12 => 0x800,
            n @ (0x13 | 0x14) => {
                let b: u8 = mem.read(g(mem, 0x011a_006e));
                let bit = if n == 0x13 { 1 } else { 2 };
                return if b & bit != 0 { jump(mem) } else { fail };
            }
            _ => return cmd,
        };
        if call(mem, g(mem, 0x0082_abd0), &[h]) != 0 {
            return jump(mem);
        }
        let i = call(mem, g(mem, 0x007f_e970), &[h, 0, 0]);
        if i.cast_signed() < 0 {
            return jump(mem);
        }
        let rec = g(mem, 0x012a_22cc).wrapping_add(i << 5);
        let flags =
            u32::from(mem.read::<u16>(rec)) | u32::from(mem.read::<u16>(g(mem, 0x011a_0080)));
        if (bits | 0x4020) & flags != 0 {
            jump(mem)
        } else {
            fail
        }
    }

    /// `opu_jump_etc2(host, cmd)` (`0x00816e50`, opus command table
    /// `0x00c49150`) -> the next command: tests that branch to `cmd + i16
    /// cmd +2` or go on to `cmd + 4`, by the flag nibble (`u` = the host's
    /// unit, `c` = byte `u +0x18`, `h` = `g_EtChrHandles[c]`): 0 on
    /// `MsGetChrFly(c)` set, or clear with flag 0x800; 0x1000 on
    /// `MsGetDamageResult(g_EtEff[u16 u +0x14] +0, c, 0, 0, host)` = 1;
    /// 0x2000 on `c` below 8; 0x4000 when a character of the effect record
    /// (bytes `+0x10..` of its `+8` count) has a status record
    /// (`opus_get_chr_status_index(handle, 0, 0)`) whose number (`g_EtChr
    /// +4`) is 0x10b2..0x10b4 or 0x1121..0x1123; 0x5000 when
    /// `Sg_MemGet(0)`, `(1)` and `(2)` are all below 0x1e0000; 0x6000 on
    /// bit 0 of `h`'s status record byte `+0x17` (its index unchecked).
    /// 0x3000 follows `h`'s field mode (`Ch_GetFieldMode`) into its status
    /// record's byte `+0x16`: unless `h` is hidden, has no record or the
    /// mode is the same, a new mode 2 stores 2 and returns `cmd + i16 cmd
    /// +2` from 0, else `cmd`; mode 0 stores 0 and returns `cmd + i16 cmd
    /// +4` from 2, else `cmd`; another mode stores 0; those `cmd + 6`.
    /// Other nibbles `cmd`. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_jump_etc2<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        let flags: u16 = mem.read(host.wrapping_add(0x21a));
        let i = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
        let rec = EFF.ptr(mem).addr.wrapping_add(i << 5);
        let rel = |mem: &M, at: u32| {
            cmd.wrapping_add(i32::from(mem.read::<i16>(cmd.wrapping_add(at))).cast_unsigned())
        };
        let chr = |mem: &M| u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let handle = |mem: &M, c: u32| crate::et::bridge::CHR_HANDLES.ptr(mem).at(c).get(mem);
        let chrs = g(mem, 0x012a_22c0);
        let status = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, h: u32| {
            let f = g(mem, 0x007f_e970);
            call(mem, f, &[h, 0, 0])
        };
        let jump = match flags & 0xf000 {
            0 => {
                let c = chr(mem);
                let fly = call(mem, g(mem, 0x0079_4f70), &[c]) != 0;
                fly == (flags & 0x800 == 0)
            }
            0x1000 => {
                let id: u32 = mem.read(rec);
                let c = chr(mem);
                call(mem, g(mem, 0x0079_a920), &[id, c, 0, 0, host]) == 1
            }
            0x2000 => chr(mem) < 8,
            0x3000 => {
                let h = handle(mem, chr(mem));
                if call(mem, g(mem, 0x0082_abd0), &[h]) != 0 {
                    return cmd.wrapping_add(6);
                }
                let s = status(mem, call, h);
                if s.cast_signed() < 0 {
                    return cmd.wrapping_add(6);
                }
                let at = chrs.wrapping_add(s << 5).wrapping_add(0x16);
                let mode = call(mem, g(mem, 0x0082_ab90), &[h]);
                let old: u8 = mem.read(at);
                if mode == u32::from(old) {
                    return cmd.wrapping_add(6);
                }
                return match mode {
                    2 => {
                        mem.write(at, 2u8);
                        if old == 0 { rel(mem, 2) } else { cmd }
                    }
                    0 => {
                        mem.write(at, 0u8);
                        if old == 2 { rel(mem, 4) } else { cmd }
                    }
                    _ => {
                        mem.write(at, 0u8);
                        cmd.wrapping_add(6)
                    }
                };
            }
            0x4000 => {
                let mut k = 0u32;
                let mut hit = false;
                while k < u32::from(mem.read::<u8>(rec.wrapping_add(8))) {
                    let c = u32::from(mem.read::<u8>(rec.wrapping_add(0x10 + k)));
                    let h = handle(mem, c);
                    let s = status(mem, call, h);
                    if s.cast_signed() >= 0 {
                        let no: u32 = mem.read(chrs.wrapping_add(s << 5).wrapping_add(4));
                        if matches!(no, 0x10b2..=0x10b4 | 0x1121..=0x1123) {
                            hit = true;
                            break;
                        }
                    }
                    k += 1;
                }
                hit
            }
            0x5000 => {
                let f = g(mem, 0x0083_bff0);
                (0..3).all(|k| call(mem, f, &[k]).cast_signed() < 0x1e_0000)
            }
            0x6000 => {
                let h = handle(mem, chr(mem));
                let s = status(mem, call, h);
                let b: u8 = mem.read(chrs.wrapping_add(s << 5).wrapping_add(0x17));
                b & 1 != 0
            }
            _ => return cmd,
        };
        if jump {
            rel(mem, 2)
        } else {
            cmd.wrapping_add(4)
        }
    }

    /// `opu_matrix_etc(host, cmd)` (`0x00801ab0`, opus command table
    /// `0x00c490f0`) -> the next command: the unit `u`'s matrix (`+0xc0`)
    /// and point (`+0x90`) by the flag nibble (`h` = `g_EtChrHandles[byte
    /// u +0x18]`; `host +0` and `+0x40` serve as scratch matrices): 0 the
    /// point into the space of the parent `i16 cmd +2` names
    /// (`opus_parent_get` with `u16 u +0xb8` set to it meanwhile;
    /// `FFXVu0InversMatrix(host, parent)`, `op_apply_m_w1`) and `u +0x40` =
    /// 16 x the point, `cmd + 4`; 0x1000 the matrix = bone `i16 cmd +2`
    /// (`opus_get_bone_matrix`), `cmd + 4`; 0x2000 `u +0xf0` = the point
    /// through the matrix; 0x3000 matrix x `host +0x280`; 0x4000 the matrix
    /// moved by the point (`op_trans_m`); 0x5000 the matrix = the root
    /// matrix x the bone (`cmd + 4`); 0x6000 `opus_chr_org_set(h, matrix)`;
    /// 0x7000 the matrix x the rotation of `u +0x30`; 0x8000 the matrix x
    /// the root matrix; these `cmd + 2`, others `cmd`. Callees through
    /// `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_matrix_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let u = unit(mem, host);
        let c = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(c).get(mem);
        let (m, p) = (u.wrapping_add(0xc0), u.wrapping_add(0x90));
        let no = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let mut go = |mem: &mut M, f: u32, args: &[u32]| {
            let f = g(mem, f);
            call(mem, f, args)
        };
        match nibble(mem, host) {
            0 => {
                let parent = u.wrapping_add(0xb8);
                let keep: u16 = mem.read(parent);
                let n: u16 = mem.read(cmd.wrapping_add(2));
                mem.write(parent, n);
                let pm = go(mem, 0x0080_1f00, &[host, u]);
                go(mem, 0x006e_d450, &[host, pm]);
                go(mem, 0x007e_79a0, &[p, host, p]);
                let s: u32 = mem.read(g(mem, 0x00b9_231c));
                go(mem, 0x007e_a850, &[u.wrapping_add(0x40), p, s]);
                mem.write(parent, keep);
                cmd.wrapping_add(4)
            }
            0x1000 => {
                go(mem, 0x0080_5430, &[m, h, no]);
                cmd.wrapping_add(4)
            }
            0x2000 => {
                go(mem, 0x007e_79a0, &[u.wrapping_add(0xf0), m, p]);
                cmd.wrapping_add(2)
            }
            0x3000 => {
                go(mem, 0x007e_83b0, &[m, m, host.wrapping_add(0x280)]);
                cmd.wrapping_add(2)
            }
            0x4000 => {
                go(mem, 0x007e_a550, &[m, m, p]);
                cmd.wrapping_add(2)
            }
            0x5000 => {
                go(mem, 0x0080_5400, &[host, h]);
                let b = host.wrapping_add(0x40);
                go(mem, 0x0080_5430, &[b, h, no]);
                go(mem, 0x007e_83b0, &[m, host, b]);
                cmd.wrapping_add(4)
            }
            0x6000 => {
                go(mem, 0x0080_53a0, &[h, m]);
                cmd.wrapping_add(2)
            }
            0x7000 => {
                go(mem, 0x007e_a6d0, &[host]);
                go(mem, 0x007e_9530, &[host, host, u.wrapping_add(0x30)]);
                go(mem, 0x007e_83b0, &[m, m, host]);
                cmd.wrapping_add(2)
            }
            0x8000 => {
                go(mem, 0x0080_5400, &[host, h]);
                go(mem, 0x007e_83b0, &[m, host, m]);
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `opu_local_etc(host, cmd)` (`0x008017d0`, opus command table
    /// `0x00c49064`) -> `cmd + 6`: a local frame between two units (`a`, `b`
    /// = `opus_no_get` of `u16 cmd +2` and `+4` from the host's unit `u`),
    /// through the emulated registers as the original does: `at` (`0x00c8f528`)
    /// = (1, 0), `0x00c8f970` = `b`'s point (`+0x90`), the move register
    /// (`0x00c8f980`) = `b - a` (`f32`, all four), `restoreVf00Register`;
    /// `host +0` = that vector and `[g_pppColorConShift + host + 0xc]` =
    /// `at.x` (the shift then 0). With flag 0x400 `host +0x10` = r0 and
    /// `op_vector_m(host + 0x40, host + 0x10, host)`; else
    /// `op_vector_m(host + 0x40, u + 0x90, host)`, `at` = (1, 0), the move
    /// register = `u`'s point, `host +0x70` = it and `[shift + host +
    /// 0x7c]` = `at.x` (the shift 0). Then `u +0xc0` = `host +0x280` x that
    /// matrix with flag 0x8000, else `FFXVu0CameraMatrix(u + 0xc0, u +
    /// 0x90, host, 0x00c48ca0)` with flag 0x200, else the matrix
    /// (`op_m_copy`). Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_local_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{load, store, sub};
        let u = unit(mem, host);
        let list = host.wrapping_add(0x300);
        let (na, nb) = (
            u32::from(mem.read::<u16>(cmd.wrapping_add(2))),
            u32::from(mem.read::<u16>(cmd.wrapping_add(4))),
        );
        let a = opus_no_get(mem, list, u, na);
        let b = opus_no_get(mem, list, u, nb);
        let (at, mv, tmp) = (
            g(mem, 0x00c8_f528),
            g(mem, 0x00c8_f980),
            g(mem, 0x00c8_f970),
        );
        let shift = g(mem, 0x00c8_b500);
        mem.write(at, [1.0f32.to_bits(), 0]);
        let pa: [u32; 4] = mem.read(a.wrapping_add(0x90));
        mem.write(mv, pa);
        let pb: [u32; 4] = mem.read(b.wrapping_add(0x90));
        mem.write(tmp, pb);
        let d: [u32; 4] = std::array::from_fn(|k| store(sub(load(pb[k]), load(pa[k]))));
        mem.write(mv, d);
        ffx_ps2::ee::bridge::restore_vf00(mem);
        let v: [u32; 4] = mem.read(mv);
        mem.write(host, v);
        let put_at = |mem: &mut M, off: u32| {
            let s: u32 = mem.read(shift);
            let x: u32 = mem.read(at);
            mem.write(s.wrapping_add(host).wrapping_add(off), x);
            mem.write(shift, 0u32);
        };
        put_at(mem, 0xc);
        let m = host.wrapping_add(0x40);
        let flags = |mem: &M| mem.read::<u16>(host.wrapping_add(0x21a));
        if flags(mem) & 0x400 != 0 {
            let r0: [u32; 4] = mem.read(crate::ppp::bridge::R0.ptr(mem).addr);
            mem.write(host.wrapping_add(0x10), r0);
            call(
                mem,
                g(mem, 0x007e_a760),
                &[m, host.wrapping_add(0x10), host],
            );
        } else {
            let p = u.wrapping_add(0x90);
            call(mem, g(mem, 0x007e_a760), &[m, p, host]);
            mem.write(at, [1.0f32.to_bits(), 0]);
            let pu: [u32; 4] = mem.read(p);
            mem.write(mv, pu);
            mem.write(host.wrapping_add(0x70), pu);
            put_at(mem, 0x7c);
        }
        let um = u.wrapping_add(0xc0);
        let f = flags(mem);
        if f & 0x8000 != 0 {
            call(mem, g(mem, 0x007e_83b0), &[um, host.wrapping_add(0x280), m]);
        } else if f & 0x200 != 0 {
            let c = g(mem, 0x00c4_8ca0);
            call(
                mem,
                g(mem, 0x0062_fc80),
                &[um, u.wrapping_add(0x90), host, c],
            );
        } else {
            call(mem, g(mem, 0x007e_8010), &[um, m]);
        }
        cmd.wrapping_add(6)
    }

    /// `0x008187b0` (`opu_v_lerp`, ours; opus command table `0x00c49110`)
    /// -> `cmd + 6`: the unit `u`'s point between two units (`a`, `b` =
    /// `opus_no_get` of `u16 cmd +2` and `+4`), `t` = `u f32 +0x40`, through
    /// the emulated registers as the original does: `0x00c8f8d0` = `u
    /// +0x40..`, `0x00c8f8f0` = `a`'s point, `0x00c8f900` = `b`'s, `d` at
    /// `0x00c8f970` = `f32(b - a)` (x, y, z; `w` kept), `at` (`0x00c8f528`)
    /// = (1/256, 0), the move register (`0x00c8f980`) = (1/256, 0, `at.z`,
    /// `at.w`); `restoreVf00Register`; then `d` = `f32(f32(t d) x move.x) +
    /// a` per axis, `at` = `d`; `[g_pppColorConShift + u + 0x90]` and
    /// `+0x94` = `d.x`, `d.y`, `u +0x98` = `d.z`; finally `at` = (`d.z`,
    /// r0.z, `d.w`, r0.w) and the shift 0.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_v_lerp(mem: &mut impl Mem, host: u32, cmd: u32) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let u = unit(mem, host);
        let list = host.wrapping_add(0x300);
        let (na, nb) = (
            u32::from(mem.read::<u16>(cmd.wrapping_add(2))),
            u32::from(mem.read::<u16>(cmd.wrapping_add(4))),
        );
        let a = opus_no_get(mem, list, u, na);
        let b = opus_no_get(mem, list, u, nb);
        let at = g(mem, 0x00c8_f528);
        let (pt, ra, rb, d, mv) = (
            g(mem, 0x00c8_f8d0),
            g(mem, 0x00c8_f8f0),
            g(mem, 0x00c8_f900),
            g(mem, 0x00c8_f970),
            g(mem, 0x00c8_f980),
        );
        let step = 0x3b80_0000u32;
        mem.write(at, [step, 0]);
        let p: [u32; 4] = mem.read(u.wrapping_add(0x40));
        mem.write(pt, p);
        let pa: [u32; 4] = mem.read(a.wrapping_add(0x90));
        mem.write(ra, pa);
        let pb: [u32; 4] = mem.read(b.wrapping_add(0x90));
        mem.write(rb, pb);
        for (k, at) in (0..3).zip((d..).step_by(4)) {
            mem.write(at, store(sub(load(pb[k]), load(pa[k]))));
        }
        let hi: [u32; 2] = mem.read(at.wrapping_add(8));
        mem.write(mv, [step, 0, hi[0], hi[1]]);
        ffx_ps2::ee::bridge::restore_vf00(mem);
        let t = load(mem.read(pt));
        for k in 0..3u32 {
            let v = store(mul(t, load(mem.read(d + 4 * k))));
            mem.write(d + 4 * k, v);
        }
        let s = load(mem.read(mv));
        for k in 0..3u32 {
            let v = store(mul(s, load(mem.read(d + 4 * k))));
            mem.write(d + 4 * k, v);
        }
        let mut r = [0u32; 4];
        for (k, at_k) in (0..3).zip((d..).step_by(4)) {
            let v = store(add(load(mem.read(at_k)), load(pa[k])));
            mem.write(at_k, v);
            r[k] = v;
        }
        r[3] = mem.read(d + 0xc);
        mem.write(at, r);
        let shift = g(mem, 0x00c8_b500);
        for (k, off) in [(0usize, 0x90u32), (1, 0x94)] {
            let sh: u32 = mem.read(shift);
            mem.write(sh.wrapping_add(u).wrapping_add(off), r[k]);
        }
        let r0: [u32; 4] = mem.read(crate::ppp::bridge::R0.ptr(mem).addr);
        mem.write(at, [r[2], r0[2], r[3], r0[3]]);
        mem.write(u.wrapping_add(0x98), r[2]);
        mem.write(shift, 0u32);
        cmd.wrapping_add(6)
    }

    /// `opu_mimic(host, cmd)` (`0x0080f210`; opus command, no direct
    /// caller): the "mimic" model copy, by the flag nibble (`u16 host
    /// +0x21a & 0xf000`), unit `u` = `[host +0x220]` (read first). 0: `n`
    /// = `i16 cmd +2`, `p` = `op_malloc([host +0x3a8], (n + 3) * 0x60)` at
    /// `u +0xbc`, `u16 p +0xc` = `n`, `u16 p +0xe` = 0, byte `p +0x10` =
    /// 0, `p +0` / `+4` = the `g_EtChr` records of `op_et_chr_index_by_no(i16
    /// cmd +4 / +6)` (both read before the first call), byte `u +0xbb` =
    /// 0x12, `opu_id_list_add(host +0x300, u16 host +0x214)`, `cmd + 8`.
    /// 0x1000: `p` = `[opus_no_get(host +0x300, u, u16 cmd +2) +0xbc]`,
    /// entry `e` = `p + (u16 p +0xe + 3) * 0x60`: `e +0` = `u`, `u16 e +4`
    /// / `+6` = `ftol(f32 u +0x40 / +0x4c)`, the emulated register
    /// `0x00c8f528..` = `u +0x30..+0x3c`, `e +0x10..+0x18` = its first
    /// three words (the last two read back from it), `e +0x1c` = 0.0,
    /// `u16 p +0xe` + 1, `cmd + 4`. 0x2000: `p` =
    /// `[opus_get_osu_by_draw(0, 0, 0x12, 1) +0xbc]`, `chr` = `[[p +4]]`;
    /// `Ch_TextureSetClutBase(chr, -1, 0x3c00)`,
    /// `Ch_TextureSetImageBase(chr, -1, 12000)`; `op_mdc_size_get_main(chr,
    /// 2, 1, 0)` 0 gives `cmd + 6`; else `buf` =
    /// `MsEffectGetBufferAddress(0)` or `0x02333500`, `p +8` = `buf`,
    /// `op_mdc_all_make_main(buf, chr, 2, 0, 0, [host +0x328])`,
    /// `op_mdc_mem_transfer_cue_free(chr)`, then for each model `i <
    /// Ch_GetNbModel(chr)` (signed) `p +0x20 + 4i` = `Ch_ModelGetVtexPtr`
    /// and `p +0xa0 + 4i` = `Ch_ModelGetNromPtr`; `cmd + 2`. Other nibbles
    /// `cmd`. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_mimic<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let h = |k: u32| host.wrapping_add(k);
        match nibble(mem, host) {
            0 => {
                let n = mem.read::<i16>(cmd.wrapping_add(2));
                let size = (i32::from(n) + 3).wrapping_mul(0x60).cast_unsigned();
                let heap: u32 = mem.read(h(0x3a8));
                let p = go(mem, 0x007f_f780, &[heap, size]);
                mem.write(u.wrapping_add(0xbc), p);
                mem.write(p.wrapping_add(0xc), n);
                mem.write(p.wrapping_add(0xe), 0u16);
                mem.write(p.wrapping_add(0x10), 0u8);
                let a = i32::from(mem.read::<i16>(cmd.wrapping_add(4))).cast_unsigned();
                let b = i32::from(mem.read::<i16>(cmd.wrapping_add(6))).cast_unsigned();
                let recs = g(mem, 0x012a_22c0);
                let i = go(mem, 0x007f_eaa0, &[a]);
                mem.write(p, (i << 5).wrapping_add(recs));
                let i = go(mem, 0x007f_eaa0, &[b]);
                mem.write(p.wrapping_add(4), (i << 5).wrapping_add(recs));
                mem.write(u.wrapping_add(0xbb), 0x12u8);
                let id = u32::from(mem.read::<u16>(h(0x214)));
                go(mem, 0x0080_0d60, &[h(0x300), id]);
                cmd.wrapping_add(8)
            }
            0x1000 => {
                let no = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
                let o = go(mem, 0x0080_1060, &[h(0x300), u, no]);
                let p: u32 = mem.read(o.wrapping_add(0xbc));
                let k = u32::from(mem.read::<u16>(p.wrapping_add(0xe)));
                let e = (k + 3).wrapping_mul(0x60).wrapping_add(p);
                mem.write(e, u);
                #[allow(clippy::cast_possible_truncation)] // mov word, ax
                let x = ftol(mem, load(mem.read(u.wrapping_add(0x40)))) as u16;
                mem.write(e.wrapping_add(4), x);
                #[allow(clippy::cast_possible_truncation)] // mov word, ax
                let z = ftol(mem, load(mem.read(u.wrapping_add(0x4c)))) as u16;
                mem.write(e.wrapping_add(6), z);
                let reg = g(mem, 0x00c8_f528);
                let w0: u32 = mem.read(u.wrapping_add(0x30));
                mem.write(reg, w0);
                for k in 1..4u32 {
                    let w: u32 = mem.read(u.wrapping_add(0x30 + 4 * k));
                    mem.write(reg + 4 * k, w);
                }
                mem.write(e.wrapping_add(0x10), w0);
                for k in 1..3u32 {
                    let w: u32 = mem.read(reg + 4 * k);
                    mem.write(e.wrapping_add(0x10 + 4 * k), w);
                }
                mem.write(e.wrapping_add(0x1c), 0u32);
                let c: u16 = mem.read(p.wrapping_add(0xe));
                mem.write(p.wrapping_add(0xe), c.wrapping_add(1));
                cmd.wrapping_add(4)
            }
            0x2000 => {
                let o = go(mem, 0x0080_0f60, &[0, 0, 0x12, 1]);
                let p: u32 = mem.read(o.wrapping_add(0xbc));
                let chr: u32 = mem.read(mem.read::<u32>(p.wrapping_add(4)));
                go(mem, 0x0083_13c0, &[chr, u32::MAX, 0x3c00]);
                go(mem, 0x0083_1390, &[chr, u32::MAX, 12000]);
                if go(mem, 0x0080_3880, &[chr, 2, 1, 0]) == 0 {
                    return cmd.wrapping_add(6);
                }
                let mut buf = go(mem, 0x007b_7eb0, &[0]);
                if buf == 0 {
                    buf = g(mem, 0x0233_3500);
                }
                mem.write(p.wrapping_add(8), buf);
                let table: u32 = mem.read(h(0x328));
                go(mem, 0x0080_34b0, &[buf, chr, 2, 0, 0, table]);
                go(mem, 0x0080_3730, &[chr]);
                let n = go(mem, 0x0083_0410, &[chr]).cast_signed();
                let mut i = 0i32;
                while i < n {
                    let k = i.cast_unsigned();
                    let v = go(mem, 0x0082_ae10, &[chr, k]);
                    mem.write(p.wrapping_add(0x20).wrapping_add(4 * k), v);
                    let v = go(mem, 0x0082_adf0, &[chr, k]);
                    mem.write(p.wrapping_add(0xa0).wrapping_add(4 * k), v);
                    i += 1;
                }
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `opu_mdc_break(host, cmd)` (`0x00804270`; opus command, no direct
    /// caller): the shattering model, by the flag nibble, unit `u` =
    /// `[host +0x220]`, `chr` = `g_EtChrHandles[u8 u +0x18]`, the slot
    /// bits byte `host +0x33b`, the three slot buffers at `0x012f20a4`. 0:
    /// the first clear bit `k < 3` (none: byte `host +0x210` = 1, `cmd`);
    /// it is set, slot `k` = `user_aligned_malloc(0x120000, 16)`,
    /// `Ch_TextureSetClutBase(chr, -1, 0x3c00)`,
    /// `Ch_TextureSetImageBase(chr, -1, 12000)`, `Ch_SetCalcNorm(chr, 1)`,
    /// `op_mdc_all_make_delhide(g_AbilitymapWork, chr, 0, [host +0x328])`,
    /// the first of the 8 `g_OpMdcTransferCues` keyed by `chr` cleared;
    /// byte `u +0xad` = `k`, `u +0xa8` = the buffer,
    /// `opus_rt_m_chara(host, u8 u +0x18, f32 u +0x3c, u +0x30, u +0x90, i8
    /// host +0x21e)`, `Ch_OptposGet(chr, 16, host +0x40)`; the original's
    /// 16-byte local (at `frame`) = (`host`, `host +0x40`,
    /// `f32 MsGetChrGround(u8 u +0x18)`, the buffer) goes to
    /// `opus_mdc_break_make(g_AbilitymapWork, buffer, cmd + i16 cmd +2,
    /// frame)`, and `createU16MeshBreakRenderDataBuffer(0, u, buffer, chr,
    /// 0x4000)` != 0 runs `graphicVFXSetup(0, u)`; `cmd + 4`. 0x1000:
    /// `Ch_TextureLoadImage(chr, -1)`, `Ch_TextureLoadClut(chr, -1)`,
    /// `opus_mdc_break_draw([u +0xa8], u, host +0x280, host +0x2c0, i16 host
    /// +0x212)`, `MsEffectWaitFlag(1)`, bit 0 of byte `u +0xb2` set, `cmd +
    /// 2`. 0x2000: both texture bases -1, `graphicVFXDestroy(0, u)`, slot
    /// `k` = `i8 u +0xad` freed (`user_aligned_free`) and cleared, its bit
    /// (`u8` 1 doubled `k` times, 1 for `k <= 0`) cleared in `host +0x33b`,
    /// bit 0 of `u +0xb2` cleared, `MsEffectWaitFlag(0)` when no slot is
    /// left; `cmd + 2`. Other nibbles `cmd`. Callees through `call`, the
    /// ground through `fcall`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_mdc_break<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{load, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let u = unit(mem, host);
        let h = |k: u32| host.wrapping_add(k);
        let who = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        let chr = crate::et::bridge::CHR_HANDLES.ptr(mem).at(who).get(mem);
        let slots = g(mem, 0x012f_20a4);
        match nibble(mem, host) {
            0 => {
                let used: u8 = mem.read(h(0x33b));
                let Some(k) = (0..3u32).find(|&k| used & (1 << k) == 0) else {
                    mem.write(h(0x210), 1u8);
                    return cmd;
                };
                mem.write(h(0x33b), used | (1 << k));
                let buf = go(mem, 0x0068_6ff0, &[0x0012_0000, 0x10]);
                mem.write(slots.wrapping_add(4 * k), buf);
                go(mem, 0x0083_13c0, &[chr, u32::MAX, 0x3c00]);
                go(mem, 0x0083_1390, &[chr, u32::MAX, 12000]);
                go(mem, 0x0083_5750, &[chr, 1]);
                let work = g(mem, 0x0164_0870);
                let table: u32 = mem.read(h(0x328));
                go(mem, 0x0080_3360, &[work, chr, 0, table]);
                let cues = crate::op::bridge::MDC_TRANSFER_CUES.ptr(mem).addr;
                if let Some(c) = (0..8u32)
                    .map(|i| cues.wrapping_add(0x10 * i))
                    .find(|&c| mem.read::<u32>(c) == chr)
                {
                    mem.write(c, 0u32);
                }
                let step = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
                let next = cmd.wrapping_add(step);
                #[allow(clippy::cast_possible_truncation)] // k < 3
                mem.write(u.wrapping_add(0xad), k as u8);
                mem.write(u.wrapping_add(0xa8), buf);
                let battle = i32::from(mem.read::<i8>(h(0x21e))).cast_unsigned();
                // fld dword u +0x3c; fstp dword [esp]
                let k3c = store(load(mem.read(u.wrapping_add(0x3c))));
                let who = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
                go(
                    mem,
                    0x0080_62c0,
                    &[
                        host,
                        who,
                        k3c,
                        u.wrapping_add(0x30),
                        u.wrapping_add(0x90),
                        battle,
                    ],
                );
                go(mem, 0x0083_54f0, &[chr, 0x10, h(0x40)]);
                let who = u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
                let ground = g(mem, 0x0079_4fb0);
                let ground = store(fcall(mem, ground, &[who]));
                mem.write(frame, [host, h(0x40), ground, buf]);
                go(mem, 0x0080_59c0, &[work, buf, next, frame]);
                if go(mem, 0x0080_2fb0, &[0, u, buf, chr, 0x4000]) != 0 {
                    go(mem, 0x0064_5780, &[0, u]);
                }
                cmd.wrapping_add(4)
            }
            0x1000 => {
                let id: u32 = mem.read(u.wrapping_add(0xa8));
                go(mem, 0x0083_1300, &[chr, u32::MAX]);
                go(mem, 0x0083_11d0, &[chr, u32::MAX]);
                let still = i32::from(mem.read::<i16>(h(0x212))).cast_unsigned();
                go(mem, 0x0080_5510, &[id, u, h(0x280), h(0x2c0), still]);
                go(mem, 0x0078_8b20, &[1]);
                let b: u8 = mem.read(u.wrapping_add(0xb2));
                mem.write(u.wrapping_add(0xb2), b | 1);
                cmd.wrapping_add(2)
            }
            0x2000 => {
                go(mem, 0x0083_13c0, &[chr, u32::MAX, u32::MAX]);
                go(mem, 0x0083_1390, &[chr, u32::MAX, u32::MAX]);
                go(mem, 0x0064_5280, &[0, u]);
                let k = i32::from(mem.read::<i8>(u.wrapping_add(0xad)));
                let at = slots.wrapping_add((4 * k).cast_unsigned());
                let buf: u32 = mem.read(at);
                go(mem, 0x0068_6fe0, &[buf]);
                mem.write(at, 0u32);
                let mut bit = 1u8;
                for _ in 0..k.max(0) {
                    bit = bit.wrapping_add(bit);
                }
                let used: u8 = mem.read(h(0x33b));
                mem.write(h(0x33b), used & !bit);
                let b: u8 = mem.read(u.wrapping_add(0xb2));
                mem.write(u.wrapping_add(0xb2), b & 0xfe);
                if mem.read::<u8>(h(0x33b)) == 0 {
                    go(mem, 0x0078_8b20, &[0]);
                }
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `opu_ffxlight(host, cmd)` (`0x00808cf0`), `frame` the original's
    /// `ebp` (two vectors at `frame - 0x24` and `- 0x14`). With `unit` =
    /// host `+0x220`, `saved` = unit `+0xbc`, by the flag nibble (host `u16
    /// +0x21a`): 0 `saved` = `op_malloc([host +0x3a8], 0x80)` holding the
    /// game's lights (`Sg_LightGetDir(i, saved + 0x20 i)` for 0..3,
    /// `Sg_LightGetColor(i, saved + 0x10 + 0x20 i)` for 0..4), `cmd + 2`.
    /// 1: those put back (`Sg_LightSetDir`/`SetColor`, the same order),
    /// `saved` freed and 0, `cmd + 2`. 2 (with `saved`): for light `k` of 4
    /// at `d = host + 0x20 k` with the source `n` = `i16 cmd +2 +2k` (read
    /// as it goes): 0 copies the saved direction and colour to `cmd + 2k`
    /// and `cmd + 0x10 + 2k` (the command, not `d`, which keeps what it
    /// had: an original slip that overwrites the next sources) through the
    /// EE registers `0x00c8f528..` (left holding the colour); else `src =
    /// opus_no_get(host +0x300, unit, n)`, `d` = unit `+0x90` - src `+0x90`
    /// and `d +0x10` = src `+0x30` times its own `w` times 1/32768. Then
    /// `d` normalised and both moved from the saved light towards the new
    /// one by `w / 256` (`w` the last source's, else the unit's, `+0x4c`;
    /// `FFXVu0Sub`/`Scale`/`AddVector`), `w` = 1; the colour clamped to
    /// 0..1 (NaN kept). Then the lights set from `host` (directions 0..3,
    /// colours 0..4 interleaved), `cmd + 0xa` (also without `saved`). 3:
    /// [`opus_ffxfocus_off`], 4: [`opus_ffxfocus_free`], both `cmd + 2`;
    /// others `cmd`. Callees through `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_ffxlight<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{load, store};
        const SUB: u32 = 0x006e_d940;
        const COPY: u32 = 0x006e_d370;
        const SCALE: u32 = 0x006e_d8f0;
        const ADD_V: u32 = 0x006e_d290;
        const SET_DIR: u32 = 0x0083_6430;
        const SET_COLOR: u32 = 0x0083_6310;
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let saved_at = unit.wrapping_add(0xbc);
        let heap = |mem: &M| mem.read::<u32>(host.wrapping_add(0x3a8));
        match mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000 {
            0 => {
                let saved = go(mem, 0x007f_f780, &[heap(mem), 0x80]);
                mem.write(saved_at, saved);
                for i in 0..3u32 {
                    go(mem, 0x0083_61c0, &[i, saved.wrapping_add(0x20 * i)]);
                }
                for i in 0..4u32 {
                    go(mem, 0x0083_60f0, &[i, saved.wrapping_add(0x10 + 0x20 * i)]);
                }
                cmd.wrapping_add(2)
            }
            0x1000 => {
                let saved: u32 = mem.read(saved_at);
                for i in 0..3u32 {
                    go(mem, SET_DIR, &[i, saved.wrapping_add(0x20 * i)]);
                }
                for i in 0..4u32 {
                    go(mem, SET_COLOR, &[i, saved.wrapping_add(0x10 + 0x20 * i)]);
                }
                go(mem, 0x007f_f1d0, &[heap(mem), mem.read(saved_at)]);
                mem.write(saved_at, 0u32);
                cmd.wrapping_add(2)
            }
            0x2000 => {
                let saved: u32 = mem.read(saved_at);
                if saved == 0 {
                    return cmd.wrapping_add(0xa);
                }
                let (v, t) = (frame.wrapping_sub(0x24), frame.wrapping_sub(0x14));
                let ee = g(mem, 0x00c8_f528);
                let mut src = unit;
                for k in 0..4u32 {
                    let d = host.wrapping_add(0x20 * k);
                    let (dir, colour) = (
                        saved.wrapping_add(0x20 * k),
                        saved.wrapping_add(0x20 * k + 0x10),
                    );
                    let n = mem.read::<i16>(cmd.wrapping_add(2 + 2 * k));
                    if n == 0 {
                        // into the command itself (at its 2k and 0x10 + 2k),
                        // not the host's light: the original writes through
                        // the command pointer here
                        let c = cmd.wrapping_add(2 * k);
                        for (from, to) in [(dir, c), (colour, c.wrapping_add(0x10))] {
                            let w: [u32; 4] = mem.read(from);
                            mem.write(ee, w);
                            mem.write(to, w);
                        }
                    } else {
                        let list = host.wrapping_add(0x300);
                        src = go(
                            mem,
                            OPUS_NO_GET,
                            &[list, unit, i32::from(n).cast_unsigned()],
                        );
                        go(
                            mem,
                            SUB,
                            &[v, unit.wrapping_add(0x90), src.wrapping_add(0x90)],
                        );
                        go(mem, COPY, &[d, v]);
                        go(mem, COPY, &[v, src.wrapping_add(0x30)]);
                        // the factors go through fld/fstp
                        let w = store(load(mem.read(v.wrapping_add(0xc))));
                        go(mem, SCALE, &[v, v, w]);
                        go(mem, SCALE, &[v, v, 0x3800_0000]);
                        go(mem, COPY, &[d.wrapping_add(0x10), v]);
                    }
                    go(mem, 0x006e_d6f0, &[d, d]);
                    for (to, from) in [(d, dir), (d.wrapping_add(0x10), colour)] {
                        go(mem, SUB, &[t, to, from]);
                        let w = store(load(mem.read(src.wrapping_add(0x4c))));
                        go(mem, SCALE, &[t, t, w]);
                        go(mem, SCALE, &[t, t, 0x3b80_0000]);
                        go(mem, ADD_V, &[t, t, from]);
                        if to != d {
                            for i in 0..4u32 {
                                let x = load(mem.read(t.wrapping_add(4 * i)));
                                if x > 1.0 {
                                    mem.write(t.wrapping_add(4 * i), 1f32);
                                }
                            }
                            for i in 0..4u32 {
                                let x = load(mem.read(t.wrapping_add(4 * i)));
                                if x < 0.0 {
                                    mem.write(t.wrapping_add(4 * i), 0f32);
                                }
                            }
                        }
                        mem.write(t.wrapping_add(0xc), 1f32);
                        go(mem, COPY, &[to, t]);
                    }
                }
                for i in 0..3u32 {
                    go(mem, SET_DIR, &[i, host.wrapping_add(0x20 * i)]);
                    go(mem, SET_COLOR, &[i, host.wrapping_add(0x10 + 0x20 * i)]);
                }
                go(mem, SET_COLOR, &[3, host.wrapping_add(0x70)]);
                cmd.wrapping_add(0xa)
            }
            0x3000 => {
                opus_ffxfocus_off(mem, call);
                cmd.wrapping_add(2)
            }
            0x4000 => {
                opus_ffxfocus_free(mem, call);
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `opu_rgba_set_00809180(host, cmd)` (`0x00809180`, name ours; opus
    /// command, next to `opu_ffxlight`): a colour into `g_Vu0_VfMove`
    /// (`0x00c8f980`) by the flag nibble (host `u16 +0x21a >> 12`), with
    /// `unit` = host `+0x220` and `src` = `opus_no_get(host +0x300, unit, u16
    /// cmd +2)`: 0 src `+0x40..`, `cmd + 4`; 1 src `+0x40..` with x, y, z
    /// times its `w` and then times `[0x00c8f78c]` (first set to 1/128),
    /// host `+0xc` (through `g_pppColorConShift`) = 128.0 and `w` = that,
    /// `0x00c8f970..` = host `+0..+0xc`, `cmd + 4`; 2 host `+0..+0xc` =
    /// the `i16` at `cmd +2..+8` and the colour them as floats, `cmd +
    /// 0xa`; 3 unit `+0x30..`, `cmd + 2`; 4 src `+0x30..`, `cmd + 4`; 5
    /// host `+0..+0xc` = unit bytes `+0xc8..+0xca` and `i16 cmd +2`, the
    /// colour them as floats, `cmd + 4`; others leave it and return `cmd`.
    /// Then `0x00c8f528` and `0x00c8f78c` = 255.0, `0x00c8f970..` =
    /// `vf00 - vf00` (`0x00c0a004..`), `restoreVf00Register`,
    /// `vfx_vec4_min(m, m, 255)`, `Vec4Max(m, m, those four)`, each
    /// component `ftol`'d, packed with `r0`'s low halves through
    /// `pppEePpacb` into `0x00c8f528`, which goes to unit `+0xc8`. Callees
    /// through `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_rgba_set<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load, mul, store, sub};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let vf = g(mem, 0x00c8_f980);
        let (ee, k, zero, r0) = (
            g(mem, 0x00c8_f528),
            g(mem, 0x00c8_f78c),
            g(mem, 0x00c8_f970),
            g(mem, 0x00c8_f508),
        );
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let list = host.wrapping_add(0x300);
        let src = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let n = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
            go(mem, OPUS_NO_GET, &[list, unit, n])
        };
        // the four words as floats (fild, fstp)
        #[allow(clippy::cast_precision_loss)] // the original's fild
        let floats = |mem: &mut M, w: [u32; 4]| {
            mem.write(host, w);
            mem.write(vf, w);
            mem.write(vf, w.map(|n| (n.cast_signed() as f32).to_bits()));
        };
        let next = match mem.read::<u16>(host.wrapping_add(0x21a)) >> 12 {
            0 => {
                let s = src(mem, &mut go);
                let c: [u32; 4] = mem.read(s.wrapping_add(0x40));
                mem.write(vf, c);
                cmd.wrapping_add(4)
            }
            1 => {
                let s = src(mem, &mut go);
                mem.write(ee, 0x3c00_0000u32);
                mem.write(k, 0x3c00_0000u32);
                let c: [u32; 4] = mem.read(s.wrapping_add(0x40));
                mem.write(vf, c);
                let x = load(mem.read(vf));
                let w = load(mem.read(s.wrapping_add(0x4c)));
                let shift = g(mem, 0x00c8_b500);
                let sh: u32 = mem.read(shift);
                mem.write(ee, 0x4300_0000u32);
                mem.write(vf, store(mul(x, w)));
                let y = load(mem.read(vf + 4));
                mem.write(vf + 4, store(mul(y, w)));
                let z = load(mem.read(vf + 8));
                mem.write(vf + 8, store(mul(w, z)));
                let s = load(mem.read(k));
                for i in 0..3u32 {
                    let v = load(mem.read(vf + 4 * i));
                    mem.write(vf + 4 * i, store(mul(v, s)));
                }
                mem.write(host.wrapping_add(sh).wrapping_add(0xc), 0x4300_0000u32);
                mem.write(shift, 0u32);
                let h: [u32; 4] = mem.read(host);
                mem.write(zero, h);
                mem.write(vf + 0xc, store(load(h[3])));
                cmd.wrapping_add(4)
            }
            2 => {
                let w = [2u32, 4, 6, 8]
                    .map(|o| i32::from(mem.read::<i16>(cmd.wrapping_add(o))).cast_unsigned());
                floats(mem, w);
                cmd.wrapping_add(0xa)
            }
            3 => {
                let c: [u32; 4] = mem.read(unit.wrapping_add(0x30));
                mem.write(vf, c);
                cmd.wrapping_add(2)
            }
            4 => {
                let s = src(mem, &mut go);
                let c: [u32; 4] = mem.read(s.wrapping_add(0x30));
                mem.write(vf, c);
                cmd.wrapping_add(4)
            }
            5 => {
                let b = |mem: &M, o: u32| u32::from(mem.read::<u8>(unit.wrapping_add(o)));
                let a = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
                let w = [b(mem, 0xc8), b(mem, 0xc9), b(mem, 0xca), a];
                floats(mem, w);
                cmd.wrapping_add(4)
            }
            _ => cmd,
        };
        let v00 = g(mem, 0x00c0_a004);
        mem.write(ee, 0x437f_0000u32);
        mem.write(k, 0x437f_0000u32);
        for i in 0..4u32 {
            let x = load(mem.read(v00 + 4 * i));
            mem.write(zero + 4 * i, store(sub(x, x)));
        }
        go(mem, 0x0042_ef00, &[]);
        let s = store(load(mem.read(k)));
        go(mem, 0x0080_8c10, &[vf, vf, s]);
        let z: [u32; 4] = mem.read(zero);
        go(mem, 0x0080_8b70, &[vf, vf, z[0], z[1], z[2], z[3]]);
        let c: [u32; 4] = mem.read(vf);
        let n = c.map(|x| ftol(mem, load(x)));
        mem.write(vf, n);
        let r: [u32; 4] = mem.read(r0);
        let lo = |a: u32, b: u32| a & 0xffff | b << 16;
        mem.write(
            ee,
            [
                lo(n[0], n[1]),
                lo(n[2], n[3]),
                lo(r[0], r[1]),
                lo(r[2], r[3]),
            ],
        );
        go(mem, 0x0073_75d0, &[ee, r0, ee]);
        let packed: u32 = mem.read(ee);
        mem.write(unit.wrapping_add(0xc8), packed);
        next
    }

    /// `opu_v_add_00819fd0(host, cmd)` (`0x00819fd0`, name ours; opus
    /// command recompiled from the EE, 64-bit register compares included):
    /// as [`opu_v_set`] with kind `k` = the low nine flag bits less 0xd6
    /// (an `i16`) and scales `0x00c49398`, but each lane is added to the
    /// vector's word (`unit + i16 0x00c49388[k]`, through the emulated
    /// `0x00c8f760` + `0x00c8f75c`) instead of stored, every taken bit
    /// (12..15, each compared with `r0` as 64 bits) moves the command
    /// pointer 2 from `cmd + 2`, and the lanes rotate after bits 12..14.
    /// The products stay in `0x00c8f530`/`+0x34`, the lanes before bit 15
    /// in `0x00c8f558..+0x64`; then `0x00c8f528` = the pointer,
    /// `0x00c8f538` = flag 0x200 (with a zero high word), `0x00c8f548` =
    /// host; with flag 0x200 equal to `r0` the pointer is returned, else
    /// the unit's motion callback (`0x00c492c8[byte +0xba]`) runs on
    /// `(host, pointer)` and its `eax` is returned. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_v_add<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store};
        let u = unit(mem, host);
        let flags = u32::from(mem.read::<u16>(host.wrapping_add(0x21a)));
        // `sub eax, 0xd6; movzx eax, ax; movsx edx, ax`
        #[allow(clippy::cast_possible_truncation)]
        let k = ((flags & 0x1ff).wrapping_sub(0xd6) as u16).cast_signed();
        let kind = i32::from(k).cast_unsigned();
        let scale: [u32; 4] = mem.read(g(mem, 0x00c4_9398).wrapping_add(kind.wrapping_mul(16)));
        let off = mem.read::<i16>(g(mem, 0x00c4_9388).wrapping_add(kind.wrapping_mul(2)));
        let dst = u.wrapping_add(i32::from(off).cast_unsigned());
        let src = cmd.wrapping_add(2);
        let p: [u32; 4] = std::array::from_fn(|i| {
            #[allow(clippy::cast_possible_truncation)] // 0..4
            let n = i32::from(mem.read::<i16>(src.wrapping_add(2 * i as u32)));
            scale[i].wrapping_mul(n.cast_unsigned())
        });
        let v0 = g(mem, 0x00c8_f528);
        mem.write(v0, p);
        // `fild`, `fstp dword`, times the `f64` 1/4096, `fstp dword`
        #[allow(clippy::cast_precision_loss)]
        let mut lanes = p.map(|w| store(mul(f64::from(w.cast_signed() as f32), 0.000_244_140_625)));
        let mov = g(mem, 0x00c8_f980);
        mem.write(mov, lanes);
        let r0: u64 = mem.read(g(mem, 0x00c8_f508));
        let (tail, sum, old) = (
            g(mem, 0x00c8_f558),
            g(mem, 0x00c8_f75c),
            g(mem, 0x00c8_f760),
        );
        let mut ptr = src;
        for (j, bit) in (0u32..).zip([0x1000u32, 0x2000, 0x4000, 0x8000]) {
            mem.write(v0, flags & bit);
            mem.write(v0.wrapping_add(4), 0u32);
            mem.write(tail, lanes);
            if u64::from(flags & bit) == r0 {
                continue;
            }
            let at = dst.wrapping_add(4 * j);
            let w: u32 = mem.read(at);
            mem.write(old, w);
            let s = store(add(load(w), load(lanes[0])));
            mem.write(sum, s);
            mem.write(at, s);
            ptr = ptr.wrapping_add(2);
            if bit != 0x8000 {
                lanes.rotate_left(1);
                mem.write(mov, lanes);
            }
        }
        let low = flags & 0x200;
        mem.write(v0, ptr);
        mem.write(v0.wrapping_add(0x10), low);
        mem.write(v0.wrapping_add(0x14), 0u32);
        mem.write(g(mem, 0x00c8_f548), host);
        if u64::from(low) == r0 {
            return ptr;
        }
        mem.write(tail, ptr);
        let cb = u32::from(mem.read::<u8>(u.wrapping_add(0xba)));
        let table = g(mem, 0x00c4_92c8);
        // `shld; shl 2; add; adc`: the 64-bit entry address into `v0`
        let at = u64::from(cb << 2) + u64::from(table);
        #[allow(clippy::cast_possible_truncation)] // the halves
        let (lo, hi) = (at as u32, (at >> 32) as u32);
        let f: u32 = mem.read(lo);
        mem.write(v0, f);
        mem.write(v0.wrapping_add(4), hi);
        call(mem, f, &[host, ptr])
    }

    /// `opu_v_etc(host, cmd)` (`0x00819450`): the opus vector commands
    /// picked by the flag nibble (`u16 host +0x21a & 0xf000`); `u` is the
    /// unit (`host +0x220`), `host` doubles as scratch. Returns the next
    /// command (`cmd` itself for an unknown nibble). Callees through `call`
    /// (`fcall` for those that return in `ST0`).
    ///
    /// - 0x0000: `MsGetMapCenter(host)`; `u +0x44` = its y x 16, `u +0x94`
    ///   = y. 0x4000 and 0x7000 the same with `MsGetChrLiftHeight(u8 u
    ///   +0x18)` and `Sg_GetSuimen()` (the `f32` result).
    /// - 0x1000: `u +0x30 + 4 i16 cmd+2` = `u16 cmd+4` x
    ///   `opus_cheight_get(u8 u +0x18, i8 host +0x21e)` / 16
    ///   (`opus_chr_thickness` is called too, its result dropped); 6 bytes.
    /// - 0x2000: the unit's motion callback (`0x00c492c8[u8 u +0xba]`) on
    ///   `(host, cmd)`, its result dropped.
    /// - 0x3000: angles from the unit floats at the `i8` offsets `cmd
    ///   +0xa..+0xc` into `host`, `r0` at `host +0x10`, the matrix `host
    ///   +0x40` = `op_rt_m_6(unit, angles, r0, order i16 cmd+2)`, the `i16`
    ///   vector `cmd +4..+8` through it (`op_apply_m_w1` into `host +0x30`)
    ///   added to the unit floats at the offsets `cmd +0xd..+0xf`, then
    ///   `op_xyz_scale(u +0x90, u +0x40, 1/16)`; 16 bytes.
    /// - 0x5000: the emulated VU0 step: the key `(u16 +6 - 1) % u16 +4 + 1`
    ///   (signed) of the table `u +0xbc` (16 bytes each) into
    ///   `g_Vu0_VfMove`, `u +0x60..+0x68` = 16 x (`u +0x90..` - it), `u
    ///   +0x6c` kept, the work registers `0x00c8f950..0x00c8f97c` and
    ///   `0x00c8f528..` left as the original leaves them; 4 bytes.
    /// - 0x6000: `MsGetMapCenter(host)` into `g_Vu0_VfMove` and `0x00c8f528`
    ///   (x and z swapped there after), x, y to `u +0x90 +
    ///   g_pppColorConShift`, z to `u +0x98`, the shift cleared, then
    ///   `op_xyz_scale(u +0x40, host, 16)`.
    /// - 0x8000: `opus_get_chr_pos(0x012a4110[32 u16 u +0x14], host)`; when
    ///   `i16 cmd+2 / 16` (an `f32`) is at most `op_length_get(u +0x90,
    ///   host)` the command jumps by `i16 cmd+4`, else 6 bytes (NaN too).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opu_v_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let u = unit(mem, host);
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let rg = |mem: &M, a: u32| mem.read::<u32>(g(mem, a));
        let wg = |mem: &mut M, a: u32, v: u32| {
            let at = g(mem, a);
            mem.write(at, v);
        };
        let at_i8 = |mem: &M, k: u32| {
            u.wrapping_add(i32::from(mem.read::<i8>(cmd.wrapping_add(k))).cast_unsigned())
        };
        let chr = |mem: &M| u32::from(mem.read::<u8>(u.wrapping_add(0x18)));
        // `fstp dword` of a result, `fld`, x 16 into +0x44 and itself into
        // +0x94 (in the order given)
        let height = |mem: &mut M, h: f64, scaled_first: bool| {
            let h = store(h);
            let scaled = store(mul(load(h), 16.0));
            if scaled_first {
                mem.write(u.wrapping_add(0x44), scaled);
                mem.write(u.wrapping_add(0x94), store(load(h)));
            } else {
                mem.write(u.wrapping_add(0x94), store(load(h)));
                mem.write(u.wrapping_add(0x44), scaled);
            }
            cmd.wrapping_add(2)
        };
        match mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000 {
            0x0000 => {
                let f = g(mem, 0x0078_2ae0);
                call(mem, f, &[host]);
                let y = rd(mem, host.wrapping_add(4));
                mem.write(u.wrapping_add(0x44), store(mul(load(y), 16.0)));
                let y = rd(mem, host.wrapping_add(4));
                mem.write(u.wrapping_add(0x94), store(load(y)));
                cmd.wrapping_add(2)
            }
            0x1000 => {
                let i = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
                let side = i32::from(mem.read::<i8>(host.wrapping_add(0x21e))).cast_unsigned();
                let (b, f) = (chr(mem), g(mem, 0x0080_0b70));
                let h = store(fcall(mem, f, &[b, side]));
                let side = i32::from(mem.read::<i8>(host.wrapping_add(0x21e))).cast_unsigned();
                let (b, f) = (chr(mem), g(mem, 0x0080_0c90));
                fcall(mem, f, &[b, side]);
                let n = f64::from(mem.read::<u16>(cmd.wrapping_add(4)));
                let v = mul(mul(n, load(store(load(h)))), 0.0625);
                mem.write(
                    u.wrapping_add(0x30).wrapping_add(i.wrapping_mul(4)),
                    store(v),
                );
                cmd.wrapping_add(6)
            }
            0x2000 => {
                let k = u32::from(mem.read::<u8>(u.wrapping_add(0xba)));
                let f = rd(mem, g(mem, 0x00c4_92c8).wrapping_add(k.wrapping_mul(4)));
                call(mem, f, &[host, cmd]);
                cmd.wrapping_add(2)
            }
            0x3000 => {
                let (a, b, c) = (at_i8(mem, 0xa), at_i8(mem, 0xb), at_i8(mem, 0xc));
                for (k, from) in (0u32..).zip([a, b, c]) {
                    let v = store(load(rd(mem, from)));
                    mem.write(host.wrapping_add(4 * k), v);
                }
                for k in 0..4u32 {
                    let w = rg(mem, 0x00c8_f508 + 4 * k);
                    mem.write(host.wrapping_add(0x10 + 4 * k), w);
                }
                let m = host.wrapping_add(0x40);
                let f = g(mem, 0x007e_a6d0);
                call(mem, f, &[m]);
                let order = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
                let f = g(mem, 0x007e_98b0);
                call(mem, f, &[m, m, host, host.wrapping_add(0x10), order]);
                let v = [4u32, 6, 8].map(|k| mem.read::<i16>(cmd.wrapping_add(k)));
                for (k, n) in (0u32..).zip(v) {
                    mem.write(host.wrapping_add(0x20 + 4 * k), store(f64::from(n)));
                }
                let f = g(mem, 0x007e_79a0);
                call(
                    mem,
                    f,
                    &[host.wrapping_add(0x30), m, host.wrapping_add(0x20)],
                );
                let to = [0xdu32, 0xe, 0xf].map(|k| at_i8(mem, k));
                for (k, to) in (0u32..).zip(to) {
                    let x = load(rd(mem, host.wrapping_add(0x30 + 4 * k)));
                    let s = store(add(x, load(rd(mem, to))));
                    mem.write(to, s);
                }
                let f = g(mem, 0x007e_a850);
                call(
                    mem,
                    f,
                    &[u.wrapping_add(0x90), u.wrapping_add(0x40), 0x3d80_0000],
                );
                cmd.wrapping_add(0x10)
            }
            0x4000 => {
                let (b, f) = (chr(mem), g(mem, 0x0079_4990));
                let h = fcall(mem, f, &[b]);
                height(mem, h, true)
            }
            0x5000 => {
                let keys = rd(mem, u.wrapping_add(0xbc));
                let n = i32::from(mem.read::<u16>(keys.wrapping_add(6))) - 1;
                let d = i32::from(mem.read::<u16>(keys.wrapping_add(4)));
                // `idiv`: the original faults on a zero count; here key 1
                let r = n.checked_rem(d).unwrap_or(0);
                let key = keys.wrapping_add((r + 1).cast_unsigned().wrapping_mul(16));
                wg(mem, 0x00c8_f52c, 0);
                wg(mem, 0x00c8_f954, 0);
                let w = rg(mem, 0x00c8_f530);
                wg(mem, 0x00c8_f958, w);
                let w = rg(mem, 0x00c8_f534);
                wg(mem, 0x00c8_f528, 0x4180_0000);
                wg(mem, 0x00c8_f950, 0x4180_0000);
                wg(mem, 0x00c8_f95c, w);
                for k in 0..4u32 {
                    let w = rd(mem, key.wrapping_add(4 * k));
                    wg(mem, 0x00c8_f980 + 4 * k, w);
                }
                // vf = u +0x90..+0x9c, vf' = u +0x60..+0x6c
                let w = rd(mem, u.wrapping_add(0x90));
                wg(mem, 0x00c8_f970, w);
                let x = sub(load(rg(mem, 0x00c8_f970)), load(rg(mem, 0x00c8_f980)));
                for k in 1..4u32 {
                    let w = rd(mem, u.wrapping_add(0x90 + 4 * k));
                    wg(mem, 0x00c8_f970 + 4 * k, w);
                }
                for k in 0..4u32 {
                    let w = rd(mem, u.wrapping_add(0x60 + 4 * k));
                    wg(mem, 0x00c8_f960 + 4 * k, w);
                }
                wg(mem, 0x00c8_f960, store(x));
                for k in 1..3u32 {
                    let a = load(rg(mem, 0x00c8_f970 + 4 * k));
                    let d = store(sub(a, load(rg(mem, 0x00c8_f980 + 4 * k))));
                    wg(mem, 0x00c8_f960 + 4 * k, d);
                }
                let f = g(mem, 0x0042_ef00);
                call(mem, f, &[]);
                let s = load(rg(mem, 0x00c8_f950));
                for k in 0..3u32 {
                    let d = store(mul(s, load(rg(mem, 0x00c8_f960 + 4 * k))));
                    wg(mem, 0x00c8_f960 + 4 * k, d);
                }
                for k in 0..4u32 {
                    let w = rg(mem, 0x00c8_f960 + 4 * k);
                    mem.write(u.wrapping_add(0x60 + 4 * k), w);
                }
                cmd.wrapping_add(4)
            }
            0x6000 => {
                let f = g(mem, 0x0078_2ae0);
                call(mem, f, &[host]);
                let x = rd(mem, host);
                wg(mem, 0x00c8_f980, x);
                let y = rd(mem, host.wrapping_add(4));
                wg(mem, 0x00c8_f984, y);
                let z = rd(mem, host.wrapping_add(8));
                wg(mem, 0x00c8_f988, z);
                let w = rd(mem, host.wrapping_add(0xc));
                wg(mem, 0x00c8_f98c, w);
                wg(mem, 0x00c8_f534, w);
                let shift = rg(mem, 0x00c8_b500);
                wg(mem, 0x00c8_f530, z);
                wg(mem, 0x00c8_f528, x);
                wg(mem, 0x00c8_f52c, y);
                mem.write(shift.wrapping_add(u).wrapping_add(0x90), x);
                let shift = rg(mem, 0x00c8_b500);
                let y = rg(mem, 0x00c8_f52c);
                mem.write(shift.wrapping_add(u).wrapping_add(0x94), y);
                let (x, z) = (rg(mem, 0x00c8_f528), rg(mem, 0x00c8_f530));
                wg(mem, 0x00c8_f530, x);
                wg(mem, 0x00c8_f528, z);
                mem.write(u.wrapping_add(0x98), z);
                wg(mem, 0x00c8_b500, 0);
                let f = g(mem, 0x007e_a850);
                call(mem, f, &[u.wrapping_add(0x40), host, 0x4180_0000]);
                cmd.wrapping_add(2)
            }
            0x7000 => {
                let f = g(mem, 0x0083_60a0);
                let h = fcall(mem, f, &[]);
                height(mem, h, false)
            }
            0x8000 => {
                let i = u32::from(mem.read::<u16>(u.wrapping_add(0x14)));
                let no = u32::from(mem.read::<u8>(g(mem, 0x012a_4110).wrapping_add(i << 5)));
                let f = g(mem, 0x0081_b1e0);
                call(mem, f, &[no, host]);
                let n = mem.read::<i16>(cmd.wrapping_add(2));
                let f = g(mem, 0x007e_7fc0);
                let len = fcall(mem, f, &[u.wrapping_add(0x90), host]);
                // `fild`, `fstp dword`, x the `f64` 1/16, `fstp dword`
                let t = load(store(mul(f64::from(n), 0.0625)));
                if t <= len {
                    let jump = i32::from(mem.read::<i16>(cmd.wrapping_add(4))).cast_unsigned();
                    cmd.wrapping_add(jump)
                } else {
                    cmd.wrapping_add(6)
                }
            }
            _ => cmd,
        }
    }

    /// The EE register image `opu_md_etc` leaves for the four `i16` at `p`
    /// (`a`, `b`, `c`, `d`): `0x00c8f528` = `b | d << 16`, `+4`, `+8`,
    /// `+0xc` their sign halves, `+0x10` `d` sign-extended to 64 bits.
    fn md_etc_regs(mem: &mut impl Mem, p: u32) {
        let b = mem.read::<u16>(p.wrapping_add(2));
        let d = mem.read::<u16>(p.wrapping_add(6));
        let sign = |h: u16| if h & 0x8000 != 0 { 0xffff_u32 } else { 0 };
        let halves = sign(b) | sign(d) << 16;
        let at = g(mem, 0x00c8_f528);
        mem.write(
            at,
            [
                u32::from(b) | u32::from(d) << 16,
                halves,
                halves,
                halves,
                i32::from(d.cast_signed()).cast_unsigned(),
                sign(d) | sign(d) << 16,
            ],
        );
    }

    /// `opu_md_etc(host, cmd)` (`0x0081c4f0`): the opus model-packet
    /// commands. `oef` is the host's oef2 (`host +0x360 + 4 u8 unit +0x1e`),
    /// `pk` its packet `op_oef2_a_get(oef, 4, u16 cmd+2)`; by the flag
    /// nibble (`u16 host +0x21a & 0xf000`):
    ///
    /// - 0x0000: a copy of `pk` from the host's heap (`host +0x3a8`,
    ///   `op_md_packet_size`, `op_malloc`, `op_md_copy`) set as packet `i16
    ///   cmd+4`; 6 bytes. 0x1000: `op_free(heap, pk)`; 4 bytes.
    /// - 0x2000: `op_md_tex0_change(pk, w, w >> 31)` with `w` the word at
    ///   `op_oef2_a_get(oef, 2, i16 cmd+4)`; 6 bytes. 0x3000:
    ///   `op_md_tex_clut_change(pk, u16 cmd+4, u16 cmd+6)`; 8 bytes.
    /// - 0x4000: the four `i16` at `cmd + i16 cmd+4` (`a`..`d`) go into
    ///   the EE registers ([`md_etc_regs`]) and `op_md_packet_set_addr(pk, a
    ///   | b << 16, c | d << 16)`; 6 bytes. 0x6000: the same at `cmd + i16
    ///   cmd+6`, then `SqsOefSetPacketAddrRange(oef, i16 cmd+2, i16 cmd+4, a
    ///   | b << 16)`; 8 bytes.
    /// - 0x5000: the draw: packet `op_oef2_a_get(oef, 4, i16 cmd+2)`, its
    ///   vertex/normal index built once (`op_md_vn_index(.., 0x06800000,
    ///   0x06c00000)` while `0x012f4120` is 0, then counted), the host's
    ///   matrix scaled by `f32 unit +0x3c x 2^-21` (`op_unit_m_scale`),
    ///   `host +0x40` = `0x0113fd10` x `g_Op3dListener`, `host +0xc0` =
    ///   `0x0113fd10` x parent (`opus_parent_get`) x host matrix, then
    ///   `opu_md_etc_draw(packet, host +0xc0, host, host +0x40, host
    ///   +0x328)`; 4 bytes.
    ///
    /// Any other nibble returns `cmd`. Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn opu_md_etc<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{load, mul, store};
        let mut fun = |mem: &mut M, f: u32, args: &[u32]| {
            let f = g(mem, f);
            call(mem, f, args)
        };
        let i16_at =
            |mem: &M, k: u32| i32::from(mem.read::<i16>(cmd.wrapping_add(k))).cast_unsigned();
        let u16_at = |mem: &M, k: u32| u32::from(mem.read::<u16>(cmd.wrapping_add(k)));
        let u = unit(mem, host);
        let slot = u32::from(mem.read::<u8>(u.wrapping_add(0x1e)));
        let oef: u32 = mem.read(host.wrapping_add(0x360).wrapping_add(slot.wrapping_mul(4)));
        let n = u16_at(mem, 2);
        let pk = fun(mem, 0x007e_3810, &[oef, 4, n]);
        match mem.read::<u16>(host.wrapping_add(0x21a)) & 0xf000 {
            0x0000 => {
                let size = fun(mem, 0x00a6_98c0, &[pk]);
                let heap = mem.read(host.wrapping_add(0x3a8));
                let copy = fun(mem, 0x007f_f780, &[heap, size]);
                fun(mem, 0x00a6_9290, &[pk, copy]);
                let i = i16_at(mem, 4);
                fun(mem, 0x007e_38a0, &[oef, 4, i, copy]);
                cmd.wrapping_add(6)
            }
            0x1000 => {
                let heap = mem.read(host.wrapping_add(0x3a8));
                fun(mem, 0x007f_f1d0, &[heap, pk]);
                cmd.wrapping_add(4)
            }
            0x2000 => {
                let i = i16_at(mem, 4);
                let at = fun(mem, 0x007e_3810, &[oef, 2, i]);
                let w = mem.read::<i32>(at);
                fun(
                    mem,
                    0x00a6_9c80,
                    &[pk, w.cast_unsigned(), (w >> 31).cast_unsigned()],
                );
                cmd.wrapping_add(6)
            }
            0x3000 => {
                let (a, b) = (u16_at(mem, 4), u16_at(mem, 6));
                fun(mem, 0x00a6_9e60, &[pk, a, b]);
                cmd.wrapping_add(8)
            }
            0x4000 => {
                let p = cmd.wrapping_add(i16_at(mem, 4));
                md_etc_regs(mem, p);
                let (lo, hi) = (mem.read(p), mem.read(p.wrapping_add(4)));
                fun(mem, 0x00a6_9de0, &[pk, lo, hi]);
                cmd.wrapping_add(6)
            }
            0x5000 => {
                let i = i16_at(mem, 2);
                let pk = fun(mem, 0x007e_3810, &[oef, 4, i]);
                if mem.read::<u32>(g(mem, 0x012f_4120)) == 0 {
                    fun(mem, 0x00a6_9f10, &[pk, 0x0680_0000, 0x06c0_0000]);
                    let at = g(mem, 0x012f_4120);
                    let n = mem.read::<u32>(at).wrapping_add(1);
                    mem.write(at, n);
                }
                // `fmul qword` 2^-21, `fstp dword`, `fld`, `fstp dword`
                let scale = store(load(store(mul(
                    load(mem.read(u.wrapping_add(0x3c))),
                    4.768_371_582_031_25e-7,
                ))));
                fun(mem, 0x007e_a700, &[host, scale]);
                let parent = fun(mem, 0x0080_1f00, &[host, u]);
                let (view, world) = (host.wrapping_add(0x40), host.wrapping_add(0xc0));
                let (cam, listener) = (g(mem, 0x0113_fd10), g(mem, 0x0113_fcd0));
                fun(mem, 0x007e_83b0, &[view, cam, listener]);
                fun(mem, 0x007e_83b0, &[world, parent, host]);
                let cam = g(mem, 0x0113_fd10);
                fun(mem, 0x007e_83b0, &[world, cam, world]);
                let e = mem.read(host.wrapping_add(0x328));
                fun(mem, 0x00a6_93b0, &[pk, world, host, view, e]);
                cmd.wrapping_add(4)
            }
            0x6000 => {
                let p = cmd.wrapping_add(i16_at(mem, 6));
                md_etc_regs(mem, p);
                let lo = mem.read(p);
                let (a, b) = (i16_at(mem, 2), i16_at(mem, 4));
                fun(mem, 0x0081_ca20, &[oef, a, b, lo]);
                cmd.wrapping_add(8)
            }
            _ => cmd,
        }
    }
}
