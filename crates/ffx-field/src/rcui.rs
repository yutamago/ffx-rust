//! `rcui`: the field UI helpers (mouse drag state, number input, vector and
//! matrix copies).

use ffx_bridge::Global;

/// The UI work area (`rcuiMouseMove*`).
pub const WORK: Global<u32> = Global::new(0x0193_c0f0);
/// The map directory names (`0x00c60e30`, "yanagi", …).
pub const MAP_DIRS: u32 = 0x00c6_0e30;
/// "/ffx/proj/map/%s/" (`0x00b688e0`).
pub const BASE_PATH_FORMAT: u32 = 0x00b6_88e0;
/// `Gu_WindowCloseTree(w)` (`0x0084ab80`).
pub const GU_WINDOW_CLOSE_TREE: u32 = 0x0084_ab80;
/// The motion table (`0x00c60a78`, 0x69 entries of 8 bytes, the number
/// first).
pub const MOTIONS: u32 = 0x00c6_0a78;
/// The motion group starts (`0x00c60dc0`, 27 ranges, each ending at the
/// next).
pub const MOTION_GROUPS: u32 = 0x00c6_0dc0;

/// Offsets in the work area.
pub mod work {
    /// `+0xbe8`: the ID being dragged.
    pub const MOVE_ID: u32 = 0xbe8;
    /// `+0xbec`: the ID `rcuiMouseMoveGetID` returns.
    pub const GET_ID: u32 = 0xbec;
    /// `+0xbf0`: non-zero while a drag runs (cleared by `rcuiMouseMoveEnd`).
    pub const MOVING: u32 = 0xbf0;
}

/// The VU0 register `rcuiVu0CopyVector` goes through.
pub const VF_COPY: Global<[u32; 4]> = Global::new(0x00c8_f980);

/// `+0x28` of a number input: set once a number was entered
/// (`rcuiNumberIsOK` reads and clears it).
pub const NUMBER_OK: u32 = 0x28;

/// A value's type for `rcuiValueGet`/`rcuiValueSet`: 1 `i8`, 2 `u8`,
/// 3 `i16`, 4 `u16`, 5 and 6 32 bits; anything else reads 0 and writes
/// nothing.
#[must_use]
pub const fn value_size(kind: u32) -> u32 {
    match kind {
        1 | 2 => 1,
        3 | 4 => 2,
        5 | 6 => 4,
        _ => 0,
    }
}

/// The UI state where the game keeps it, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{NUMBER_OK, WORK, work};
    use crate::x87;
    use ffx_bridge::x87::Reg;

    /// `rcuiMouseMoveEnd()` (`0x0093c050`); `rcuiMouseMoveReset`
    /// (`0x0093c110`) is the same code.
    pub fn rcui_mouse_move_end(mem: &mut impl Mem) {
        let w = WORK.get(mem);
        mem.write(w + work::MOVING, 0u32);
    }

    /// `rcuiMouseMoveGetID()` (`0x0093c100`).
    pub fn rcui_mouse_move_get_id(mem: &impl Mem) -> u32 {
        mem.read(WORK.get(mem) + work::GET_ID)
    }

    /// `rcuiMouseMoveIsID(id)` (`0x0093c120`): whether `id` is being
    /// dragged.
    pub fn rcui_mouse_move_is_id(mem: &impl Mem, id: u32) -> bool {
        let w = WORK.get(mem);
        mem.read::<u32>(w + work::MOVE_ID) == id && mem.read::<u32>(w + work::MOVING) != 0
    }

    /// `rcuiNumberIsOK(input)` (`0x0093c3d0`): the flag, cleared.
    pub fn rcui_number_is_ok(mem: &mut impl Mem, input: u32) -> u32 {
        let v = mem.read::<u32>(input + NUMBER_OK);
        mem.write(input + NUMBER_OK, 0u32);
        v
    }

    /// `rcuiValueGet(p, kind)` (`0x0093c210`): the value, sign- or
    /// zero-extended by its type.
    pub fn rcui_value_get(mem: &impl Mem, p: u32, kind: u32) -> u32 {
        match kind {
            1 => i32::from(mem.read::<i8>(p)).cast_unsigned(),
            2 => u32::from(mem.read::<u8>(p)),
            3 => i32::from(mem.read::<i16>(p)).cast_unsigned(),
            4 => u32::from(mem.read::<u16>(p)),
            5 | 6 => mem.read(p),
            _ => 0,
        }
    }

    /// `rcuiValueSet(p, kind, v)` (`0x0093c270`): the low bytes of `v`.
    pub fn rcui_value_set(mem: &mut impl Mem, p: u32, kind: u32, v: u32) {
        let n = super::value_size(kind) as usize;
        mem.write_bytes(p, &v.to_le_bytes()[..n]);
    }

    /// Reads a word of a UI item and clears it (`rcuiNumberIsOK` and the
    /// three at `0x0093c800`, `0x0093c820`, `0x0093c840`, for `+0x18`,
    /// `+0x1c`, `+0x14`).
    pub fn rcui_take(mem: &mut impl Mem, item: u32, offset: u32) -> u32 {
        let v = mem.read::<u32>(item + offset);
        mem.write(item + offset, 0u32);
        v
    }

    /// `rcuiItemInit(item)` (`0x0093c2c0`): `+0xc` = `0x30` (`u16`), `+0x1c`
    /// and `+0x24` cleared.
    pub fn rcui_item_init(mem: &mut impl Mem, item: u32) {
        mem.write(item + 0xc, 0x30u16);
        mem.write(item + 0x1c, 0u32);
        mem.write(item + 0x24, 0u32);
    }

    fn vec4_op(mem: &mut impl Mem, out: u32, a: u32, b: u32, op: fn(f64, f64) -> f64) {
        use ffx_bridge::x87::{load, store};
        for i in 0..4 {
            let r = op(load(mem.read(a + 4 * i)), load(mem.read(b + 4 * i)));
            mem.write(out + 4 * i, store(r));
        }
    }

    /// `rcuiAddVector(out, a, b)` (`0x0093cbc0`).
    pub fn rcui_add_vector(mem: &mut impl Mem, out: u32, a: u32, b: u32) {
        vec4_op(mem, out, a, b, ffx_bridge::x87::add);
    }

    /// `rcuiMulVector(out, a, b)` (`0x0093d290`): per component.
    pub fn rcui_mul_vector(mem: &mut impl Mem, out: u32, a: u32, b: u32) {
        vec4_op(mem, out, a, b, ffx_bridge::x87::mul);
    }

    /// `rcuiItoF(out, ints)` (`0x0093d030`): four `i32` to floats.
    pub fn rcui_itof(mem: &mut impl Mem, out: u32, ints: u32) {
        for i in 0..4 {
            let v = mem.read::<i32>(ints + 4 * i);
            #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
            mem.write(out + 4 * i, (v as f32).to_bits());
        }
    }

    /// `rcuiDot(a, b)` (`0x0093d060`): the same code as `rcVec3fDot`.
    pub fn rcui_dot(mem: &impl Mem, a: u32, b: u32) -> f32 {
        crate::geom::bridge::rc_vec3f_dot(mem, a, b)
    }

    /// `rcuiVu0CopyVector(dst, src)` (`0x009382e0`): through the VU0
    /// register at `0x00c8f980`, which keeps the copy.
    pub fn rcui_vu0_copy_vector(mem: &mut impl Mem, dst: u32, src: u32) {
        let v = mem.read::<[u32; 4]>(src);
        super::VF_COPY.set(mem, v);
        mem.write(dst, v);
    }

    fn copy_floats(mem: &mut impl Mem, dst: u32, src: u32, n: u32) {
        // one at a time, in order, as the original (they may overlap)
        for i in 0..n {
            let v = x87::copy(mem.read(src + 4 * i));
            mem.write(dst + 4 * i, v);
        }
    }

    /// `rcuiCopyVector(dst, src)` (`0x0093ce50`): four floats.
    pub fn rcui_copy_vector(mem: &mut impl Mem, dst: u32, src: u32) {
        copy_floats(mem, dst, src, 4);
    }

    /// `rcuiCopyMatrix(dst, src)` (`0x0093ce10`): sixteen floats.
    pub fn rcui_copy_matrix(mem: &mut impl Mem, dst: u32, src: u32) {
        copy_floats(mem, dst, src, 16);
    }

    /// `TextUtilList::TextUtilList(this)` (`0x0090e6d0`): 64 bytes cleared
    /// (`memset`, which returns `this`).
    pub fn text_util_list_text_util_list(mem: &mut impl Mem, this: u32) -> u32 {
        mem.write(this, [0u32; 16]);
        this
    }

    /// `rcuiCollectChrHandles()` (`0x00932570`): every character
    /// (`Ch_HandleRewind`, `Ch_HandleGetNext`) appended to the rcui work's
    /// lists: its index at `+0x830`, its handle at `+0x870`, the count at
    /// `+0x824`.
    pub fn rcui_collect_chr_handles(mem: &mut impl Mem) {
        use ffx_chara::handle::bridge as h;
        h::ch_handle_rewind(mem);
        let mut c = h::ch_handle_get_next(mem);
        while c != 0 {
            let w = WORK.get(mem);
            let n = mem.read::<u32>(w.wrapping_add(0x824));
            mem.write(w.wrapping_add(0x830).wrapping_add(n.wrapping_mul(4)), n);
            let w = WORK.get(mem);
            let n = mem.read::<u32>(w.wrapping_add(0x824));
            mem.write(w.wrapping_add(0x870).wrapping_add(n.wrapping_mul(4)), c);
            let w = WORK.get(mem);
            let n = mem.read::<u32>(w.wrapping_add(0x824));
            mem.write(w.wrapping_add(0x824), n.wrapping_add(1));
            c = h::ch_handle_get_next(mem);
        }
    }

    // ---- round 6: vectors, matrices, mouse drag ----

    /// `rcuiVecToInt(out, v)` (`0x0093cff0`): four floats through
    /// `_ftol2_sse`; returns the last.
    pub fn rcui_vec_to_int(mem: &mut impl Mem, out: u32, v: u32) -> u32 {
        let mut r = 0;
        for i in 0..4 {
            let x = mem.read::<u32>(v + 4 * i);
            r = ffx_bridge::x87::ftol(mem, x87::load(x));
            mem.write(out + 4 * i, r);
        }
        r
    }

    /// `rcuiApplyMatrix(out, m, v)` (`0x0093cbf0`): row `i` of `m` dotted
    /// with `v` (`((m1 v1 + m0 v0) + m2 v2) + m3 v3`, rounded once), then
    /// `rcuiCopyVector`.
    pub fn rcui_apply_matrix(mem: &mut impl Mem, out: u32, m: u32, v: u32) {
        let mv = mem.read::<[f32; 16]>(m);
        let vv = mem.read::<[f32; 4]>(v);
        let r: [f32; 4] = std::array::from_fn(|i| {
            let row = &mv[4 * i..4 * i + 4];
            let p = |k: usize| Reg::load(row[k]).mul_m32(vv[k]);
            (((p(1) + p(0)) + p(2)) + p(3)).store()
        });
        mem.write(out, r);
    }

    /// `rcuiMulMatrix(out, b, a)` (`0x0093d1b0`): `out[r][c] = ((a[r][1]
    /// b[1][c] + a[r][0] b[0][c]) + a[r][2] b[2][c]) + a[r][3] b[3][c]`,
    /// into a local, then `rcuiCopyMatrix` (so `out` may be an input).
    pub fn rcui_mul_matrix(mem: &mut impl Mem, out: u32, b: u32, a: u32) {
        let bm = mem.read::<[f32; 16]>(b);
        let am = mem.read::<[f32; 16]>(a);
        let r: [f32; 16] = std::array::from_fn(|k| {
            let (row, c) = (k / 4, k % 4);
            let p = |j: usize| Reg::load(am[4 * row + j]).mul_m32(bm[4 * j + c]);
            (((p(1) + p(0)) + p(2)) + p(3)).store()
        });
        mem.write(out, r);
    }

    /// `rcuiNormalize(out, v)` (`0x0093d3d0`): `v`'s x, y, z divided by its
    /// length (`_CIsqrt` of `(x² + y²) + z²`, stored as `f32`; the
    /// reciprocal stored too), w copied; nothing when the length is not
    /// above 0.
    pub fn rcui_normalize(mem: &mut impl Mem, out: u32, v: u32) {
        let [x, y, z, w] = mem.read::<[f32; 4]>(v);
        let (rx, ry, rz) = (Reg::load(x), Reg::load(y), Reg::load(z));
        let len = Reg::constant(((rx * rx + ry * ry) + rz * rz).value().sqrt()).store();
        let above = x87::load(len.to_bits()) > 0.0; // fcom: false when unordered
        if !above {
            return;
        }
        let inv = (Reg::constant(1.0) / Reg::load(len)).store();
        let r = Reg::load(inv);
        mem.write(out, (rx * r).store());
        mem.write(out + 4, r.mul_m32(y).store());
        mem.write(out + 8, r.mul_m32(z).store());
        mem.write(out + 0xc, x87::copy(w.to_bits()));
    }

    /// `rcuiMouseMoveBegin(id, arg)` (`0x0093c150`): starts dragging: the
    /// id and its argument (`+0xbe8`, `+0xbec`), the mouse captured
    /// (`Gu_MouseSetCapture`, `capture`), the dragging flag (`+0xbf0`) and
    /// where the drag started (`Gu_MouseGetX`/`Y`, `+0xbf4`, `+0xbf8`).
    pub fn rcui_mouse_move_begin<M: Mem>(
        mem: &mut M,
        id: u32,
        arg: u32,
        capture: impl FnOnce(&mut M, u32, u32),
        get_x: impl FnOnce(&mut M, u32) -> u32,
        get_y: impl FnOnce(&mut M, u32) -> u32,
    ) {
        let w = WORK.get(mem);
        mem.write(w + work::MOVE_ID, id);
        let w = WORK.get(mem);
        mem.write(w + work::GET_ID, arg);
        let w = WORK.get(mem);
        let id = mem.read::<u32>(w + work::MOVE_ID);
        capture(mem, id, arg);
        let w = WORK.get(mem);
        mem.write(w + work::MOVING, 1u32);
        let w = WORK.get(mem);
        let id = mem.read::<u32>(w + work::MOVE_ID);
        let x = get_x(mem, id);
        let w = WORK.get(mem);
        mem.write(w + 0xbf4, x);
        let w = WORK.get(mem);
        let id = mem.read::<u32>(w + work::MOVE_ID);
        let y = get_y(mem, id);
        let w = WORK.get(mem);
        mem.write(w + 0xbf8, y);
    }

    /// `rcuiCrossProduct(out, a, b)` (`0x0093d440`): `a x b` (x, y, z, each
    /// `p * q - r * s` on the x87 stack), w 0.0. Returns `out`.
    pub fn rcui_cross_product(mem: &mut impl Mem, out: u32, a: u32, b: u32) -> u32 {
        let ld = |mem: &_, v: u32, i: u32| {
            ffx_bridge::x87::load(Mem::read::<u32>(mem, v.wrapping_add(4 * i)))
        };
        let x = ld(&*mem, a, 1) * ld(&*mem, b, 2) - ld(&*mem, a, 2) * ld(&*mem, b, 1);
        mem.write(out, x87::store(x));
        let y = ld(&*mem, a, 2) * ld(&*mem, b, 0) - ld(&*mem, a, 0) * ld(&*mem, b, 2);
        mem.write(out.wrapping_add(4), x87::store(y));
        let z = ld(&*mem, b, 1) * ld(&*mem, a, 0) - ld(&*mem, b, 0) * ld(&*mem, a, 1);
        mem.write(out.wrapping_add(8), x87::store(z));
        mem.write(out.wrapping_add(0xc), 0u32);
        out
    }

    /// The map system's layer table (`*(*YnMapSystem + 8)`; 0 without a
    /// map system or map).
    fn layer_table(mem: &impl Mem) -> u32 {
        let s = crate::guide::YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return 0;
        }
        let m = mem.read::<u32>(s);
        if m == 0 {
            0
        } else {
            mem.read::<u32>(m.wrapping_add(8))
        }
    }

    /// The select flags of `rcuiClearSelect`/`rcuiSetSelect`: mode 1
    /// (`+0x28c`) changes bit 0 of the selected layer's `+0xb0` (the
    /// layer `+0x288`, `0xc0` bytes each); mode 2 bit 1 of each layer's
    /// first word (`skip_selected`: all but the selected one). Returns the
    /// `eax` the originals end with.
    fn select_flags(mem: &mut impl Mem, set: bool, skip_selected: bool) -> u32 {
        let t = layer_table(mem);
        let w = WORK.get(mem);
        let mode = mem.read::<u32>(w.wrapping_add(0x28c));
        let sel = mem.read::<u32>(w.wrapping_add(0x288));
        let apply = |v: u32, bit: u32| if set { v | bit } else { v & !bit };
        match mode {
            1 => {
                let layers = mem.read::<u32>(t.wrapping_add(8));
                let at = sel
                    .wrapping_mul(0xc0)
                    .wrapping_add(layers)
                    .wrapping_add(0xb0);
                let v = mem.read::<u32>(at);
                mem.write(at, apply(v, 1));
                layers
            }
            2 => {
                let mut last = 0;
                let mut i = 0i32;
                while i < mem.read::<i32>(t.wrapping_add(4)) {
                    let w = WORK.get(mem);
                    if !skip_selected || i.cast_unsigned() != mem.read::<u32>(w.wrapping_add(0x288))
                    {
                        last = mem.read::<u32>(t.wrapping_add(8));
                        let at = last.wrapping_add(i.cast_unsigned().wrapping_mul(0xc0));
                        let v = mem.read::<u32>(at);
                        mem.write(at, apply(v, 2));
                    }
                    i += 1;
                }
                if skip_selected { WORK.get(mem) } else { last }
            }
            _ if skip_selected => w,
            m => m.wrapping_sub(2),
        }
    }

    /// `rcuiClearSelect()` (`0x00934af0`): [`select_flags`] clearing the
    /// bits (every layer in mode 2).
    pub fn rcui_clear_select(mem: &mut impl Mem) -> u32 {
        select_flags(mem, false, false)
    }

    /// `rcuiSetSelect()` (`0x00934b50`): [`select_flags`] setting the bits
    /// (all but the selected layer in mode 2).
    pub fn rcui_set_select(mem: &mut impl Mem) -> u32 {
        select_flags(mem, true, true)
    }

    /// `rcuiMakeBasePath()` (`0x00937af0`): `/ffx/proj/map/<dir>/` into the
    /// work's `+0x74` (`sprintf`, the directory from the table at
    /// `0x00c60e30` by `+0x64`).
    pub fn rcui_make_base_path<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let w = WORK.get(mem);
        let i = mem.read::<u32>(w.wrapping_add(0x64));
        let dirs = mem.rebase(super::MAP_DIRS);
        let dir = mem.read::<u32>(dirs.wrapping_add(i.wrapping_mul(4)));
        let sprintf = mem.read::<u32>(mem.rebase(crate::yi::CRT_SPRINTF));
        let fmt = mem.rebase(super::BASE_PATH_FORMAT);
        call(mem, sprintf, &[w.wrapping_add(0x74), fmt, dir])
    }

    /// `rcuiCloseWindow(&w)` (`0x00937f50`): an open window closed
    /// (`Gu_WindowCloseTree`) and the handle cleared.
    pub fn rcui_close_window<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        at: u32,
        call: &mut C,
    ) {
        let w = mem.read::<u32>(at);
        if w != 0 {
            let f = mem.rebase(super::GU_WINDOW_CLOSE_TREE);
            call(mem, f, &[w]);
            mem.write(at, 0u32);
        }
    }

    /// `rcuiReset()` (`0x00936570`): the work's `+0x5c`, `+0x288`, `+0x308`,
    /// `+0x30c` cleared; the renderer's bit 0 disabled when set
    /// (`yiRenderDisable(1)`).
    pub fn rcui_reset(mem: &mut impl Mem) {
        for o in [0x5cu32, 0x288, 0x308, 0x30c] {
            let w = WORK.get(mem);
            mem.write(w.wrapping_add(o), 0u32);
        }
        if crate::yi::bridge::yi_get_render_status(mem) & 1 != 0 {
            crate::yi::bridge::yi_render_disable(mem, 1);
        }
    }

    /// `rcuiMotionStep(d)` (`0x00935690`): the motion selected (work
    /// `+0x8d8`, an index into the 0x69 motions at `0x00c60a78`, 8 bytes
    /// each) moved `d` groups: its group is the first of the 27 ranges at
    /// `0x00c60dc0` holding its number (else 0); the new group (negative:
    /// 26, past 26: 0) starts at a number whose motion becomes the
    /// selection (unchanged when none has it).
    pub fn rcui_motion_step(mem: &mut impl Mem, d: i32) {
        let w = WORK.get(mem);
        let motions = mem.rebase(super::MOTIONS);
        let groups = mem.rebase(super::MOTION_GROUPS);
        let cur = mem.read::<u32>(w.wrapping_add(0x8d8));
        let x = mem.read::<i32>(motions.wrapping_add(cur.wrapping_mul(8)));
        let found = (0..0x1bu32)
            .find(|&e| {
                mem.read::<i32>(groups + 4 * e) <= x && x < mem.read::<i32>(groups + 4 * e + 4)
            })
            .unwrap_or(0);
        let g = found.cast_signed().wrapping_add(d);
        let g = if g < 0 {
            0x1a
        } else if g.cast_unsigned() > 0x1a {
            0
        } else {
            g.cast_unsigned()
        };
        let target = mem.read::<i32>(groups + 4 * g);
        if let Some(i) =
            (0..0x69u32).find(|&i| mem.read::<i32>(motions.wrapping_add(8 * i)) == target)
        {
            mem.write(w.wrapping_add(0x8d8), i);
        }
    }

    /// `rcuiSetNumber(n)` (`0x00937f70`): `n`'s two decimal digits (tens
    /// truncated toward zero, plus `'0'`) over the last two characters of
    /// the work's strings at `+0x174` and `+0x27c`.
    pub fn rcui_set_number(mem: &mut impl Mem, n: i32) {
        let tens = n / 10;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // `mov byte`
        let (hi, lo) = ((tens + 0x30) as u8, (n - tens * 10 + 0x30) as u8);
        for s in [0x174u32, 0x27c] {
            let w = WORK.get(mem);
            let mut len = 0u32;
            while mem.read::<u8>(w.wrapping_add(s + len)) != 0 {
                len += 1;
            }
            let w = WORK.get(mem);
            mem.write(w.wrapping_add(s - 2 + len), hi);
            let w = WORK.get(mem);
            mem.write(w.wrapping_add(s - 1 + len), lo);
        }
    }

    /// `rcuiGuideSaveSize(g)` (`0x009396a0`): the bytes a guide map save
    /// takes: 0x10, per map (`+0x4` of them from `+0x8`, 0xc0 bytes each)
    /// 0x30 plus its two transfers' data (`+0xbc`, `+0x98 + 8`: entry size
    /// times count) and, with `+0x98` indices, 0xa0 + 4 each rounded to 16
    /// + 0x20, else 0xa0; then 0x38.
    pub fn rcui_guide_save_size(mem: &impl Mem, g: u32) -> u32 {
        let n = mem.read::<i32>(g + 4);
        let mut size = 0x10u32;
        let mut e = mem.read::<u32>(g + 8).wrapping_add(0x98);
        let data = |mem: &dyn Fn(u32) -> u32, t: u32| mem(t + 0x34).wrapping_mul(mem(t + 0x30));
        let rd = |a: u32| mem.read::<u32>(a);
        for _ in 0..n.max(0) {
            let k = mem.read::<i32>(e);
            let mut part = 0xa0u32;
            if k > 0 {
                let b = k.cast_unsigned().wrapping_shl(2);
                part = b.wrapping_add(0xa0);
                if b & 0xf != 0 {
                    part = part.wrapping_add(0x10 - (b & 0xf));
                }
                part = part
                    .wrapping_add(0x20)
                    .wrapping_add(data(&rd, mem.read::<u32>(e + 8)));
            }
            size = size
                .wrapping_add(0x30)
                .wrapping_add(data(&rd, mem.read::<u32>(e + 0x24)))
                .wrapping_add(part);
            e = e.wrapping_add(0xc0);
        }
        size.wrapping_add(0x38)
    }
}
