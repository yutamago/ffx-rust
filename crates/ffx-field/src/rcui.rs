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
/// `Ch_GetNbElem(chr)` (`0x008303f0`).
pub const CH_GET_NB_ELEM: u32 = 0x0083_03f0;
/// `Ch_SetMotionSpeed(chr, speed)` (`0x0082b380`).
pub const CH_SET_MOTION_SPEED: u32 = 0x0082_b380;
/// "Load - %s" (`0x00b68500`).
pub const LOAD_TITLE_FORMAT: u32 = 0x00b6_8500;
/// `Gu_WindowSetTitle(w, title)` (`0x0084b6a0`).
pub const GU_WINDOW_SET_TITLE: u32 = 0x0084_b6a0;
/// `MsGetChrHandle(id)` (`0x00795080`).
pub const MS_GET_CHR_HANDLE: u32 = 0x0079_5080;
/// The VU0 register at `0x00c8f970`: `rcuiVu0AddVector`'s second operand.
pub const VF_ADD_B: Global<[u32; 4]> = Global::new(0x00c8_f970);
/// `rcuiCopyVector(dst, src)` (`0x0093ce50`).
pub const RCUI_COPY_VECTOR: u32 = 0x0093_ce50;
/// `rcuiCopyMatrix(dst, src)` (`0x0093ce10`).
pub const RCUI_COPY_MATRIX: u32 = 0x0093_ce10;
/// `rcuiApplyMatrix(dst, m, v)` (`0x0093cbf0`).
pub const RCUI_APPLY_MATRIX: u32 = 0x0093_cbf0;
/// The `.rdata` double `rcuiProjectToInt` scales by (`0x00b313a0`).
pub const PROJECT_SCALE: u32 = 0x00b3_13a0;

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
    use ffx_system::stub::ptr32;

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

    // ---- round 12 ----

    /// `rcuiChrMotionSetup()` (`0x00932470`): for the selected character
    /// (handle `+0x870 + 4 * +0x8b0`, -1 none) its element count to
    /// `+0x8b8`, `+0x8bc`..`+0x8c4` cleared, the element buffer (`+0x8c8`)
    /// freed and reallocated at 16 bytes per element, `+0x8cc` 0x16,
    /// `+0x8d0` 0, motion speed 0x100 (`Ch_SetMotionSpeed`, kept at
    /// `+0x8d4`). Returns the work pointer, or the selection index with no
    /// character.
    pub fn rcui_chr_motion_setup<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let w = WORK.get(mem);
        let sel = mem.read::<u32>(w.wrapping_add(0x8b0));
        let h = mem.read::<u32>(w.wrapping_add(0x870).wrapping_add(sel.wrapping_mul(4)));
        if h == u32::MAX {
            return sel;
        }
        let f = mem.rebase(super::CH_GET_NB_ELEM);
        let n = call(mem, f, &[h]);
        let w = WORK.get(mem);
        mem.write(w.wrapping_add(0x8b8), n);
        for off in [0x8bcu32, 0x8c0, 0x8c4] {
            let w = WORK.get(mem);
            mem.write(w.wrapping_add(off), 0u32);
        }
        let w = WORK.get(mem);
        let p = ptr32(mem.read(w.wrapping_add(0x8c8)));
        if p != 0 {
            crate::model::bridge::rc_free(mem, call, p);
        }
        let w = WORK.get(mem);
        mem.write(w.wrapping_add(0x8c8), ptr32(0));
        let size = mem.read::<u32>(w.wrapping_add(0x8b8)) << 4;
        let b = crate::model::bridge::rc_alloc(mem, call, size);
        let w = WORK.get(mem);
        mem.write(w.wrapping_add(0x8c8), ptr32(b));
        mem.write(w.wrapping_add(0x8cc), 0x16u32);
        let w = WORK.get(mem);
        mem.write(w.wrapping_add(0x8d0), 0u32);
        let f = mem.rebase(super::CH_SET_MOTION_SPEED);
        call(mem, f, &[h, 0x100]);
        let w = WORK.get(mem);
        mem.write(w.wrapping_add(0x8d4), 0x100u32);
        w
    }

    /// `rcuiLoadSetTitle(buf)` (`0x00934fb0`): "Load - <dir>" (the map
    /// directory by `+0x64`) into `buf` (64 bytes, the original's stack)
    /// and `Gu_WindowSetTitle(+4, buf)`; returns that.
    pub fn rcui_load_set_title<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        buf: u32,
        call: &mut C,
    ) -> u32 {
        let w = WORK.get(mem);
        let i = mem.read::<u32>(w.wrapping_add(0x64));
        let dirs = mem.rebase(super::MAP_DIRS);
        let dir = mem.read::<u32>(dirs.wrapping_add(i.wrapping_mul(4)));
        let fmt = mem.rebase(super::LOAD_TITLE_FORMAT);
        let sprintf = mem.read::<u32>(mem.rebase(crate::yi::CRT_SPRINTF));
        call(mem, sprintf, &[buf, fmt, dir]);
        let w = WORK.get(mem);
        let win = mem.read::<u32>(w.wrapping_add(4));
        let f = mem.rebase(super::GU_WINDOW_SET_TITLE);
        call(mem, f, &[win, buf])
    }

    /// `rcuiCollectChrHandlesByGroup()` (`0x009323b0`): characters 0x14
    /// to 0x1b (counted at `+0x828`), then 0 to 0xe (`+0x82c`), each with
    /// a handle (`MsGetChrHandle`) added to the work's lists (index
    /// `+0x830`, handle `+0x870`, count `+0x824`).
    pub fn rcui_collect_chr_handles_by_group<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) {
        let f = mem.rebase(super::MS_GET_CHR_HANDLE);
        for (ids, counter) in [(0x14u32..0x1c, 0x828u32), (0..0xf, 0x82c)] {
            for id in ids {
                let h = call(mem, f, &[id]);
                if h == 0 {
                    continue;
                }
                let w = WORK.get(mem);
                let c = mem.read::<u32>(w.wrapping_add(counter)).wrapping_add(1);
                mem.write(w.wrapping_add(counter), c);
                let n = mem.read::<u32>(w.wrapping_add(0x824));
                mem.write(w.wrapping_add(0x830).wrapping_add(n.wrapping_mul(4)), id);
                let n = mem.read::<u32>(w.wrapping_add(0x824));
                mem.write(w.wrapping_add(0x870).wrapping_add(n.wrapping_mul(4)), h);
                let n = mem.read::<u32>(w.wrapping_add(0x824)).wrapping_add(1);
                mem.write(w.wrapping_add(0x824), n);
            }
        }
    }

    /// `rcuiVu0AddVector(dst, a, b)` (`0x00937540`): `dst = (a.xyz + b.xyz,
    /// a.w)` through the VU0 registers at `0x00c8f980` (`a`, then the sum)
    /// and `0x00c8f970` (`b`).
    pub fn rcui_vu0_add_vector(mem: &mut impl Mem, dst: u32, a: u32, b: u32) {
        let va = mem.read::<[u32; 4]>(a);
        super::VF_COPY.set(mem, va);
        let vb = mem.read::<[u32; 4]>(b);
        super::VF_ADD_B.set(mem, vb);
        let s = |i: usize| x87::store(x87::load(va[i]) + x87::load(vb[i]));
        let r = [s(0), s(1), s(2), va[3]];
        super::VF_COPY.set(mem, r);
        mem.write(dst, r);
    }

    // ---- round 16 ----

    /// `rcuiVu0SubVector(dst, a, b)` (`0x009376b0`): `dst = (a.xyz -
    /// b.xyz, a.w)` through the VU0 registers as [`rcui_vu0_add_vector`],
    /// `vf00` restored before the store.
    pub fn rcui_vu0_sub_vector(mem: &mut impl Mem, dst: u32, a: u32, b: u32) {
        let va = mem.read::<[u32; 4]>(a);
        super::VF_COPY.set(mem, va);
        let vb = mem.read::<[u32; 4]>(b);
        super::VF_ADD_B.set(mem, vb);
        let s = |i: usize| x87::store(x87::load(va[i]) - x87::load(vb[i]));
        let r = [s(0), s(1), s(2), va[3]];
        super::VF_COPY.set(mem, r);
        ffx_ps2::ee::bridge::restore_vf00(mem);
        let r = super::VF_COPY.get(mem);
        mem.write(dst, r);
    }

    /// `rcuiClampVector(dst, v, lo, hi)` (`0x0093cd20`): each of the four
    /// floats clamped to `lo..=hi` (`lo` wins below, `hi` above, NaN kept)
    /// into `work` (16 bytes, the original's stack), then
    /// `rcuiCopyVector(dst, work)`; returns that.
    pub fn rcui_clamp_vector<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        dst: u32,
        v: u32,
        (lo, hi): (u32, u32),
        work: u32,
        call: &mut C,
    ) -> u32 {
        let (l, h) = (f32::from_bits(lo), f32::from_bits(hi));
        for i in 0..4u32 {
            let x = mem.read::<f32>(v.wrapping_add(4 * i));
            let r = if l > x {
                lo
            } else if x > h {
                hi
            } else {
                x.to_bits()
            };
            mem.write(work.wrapping_add(4 * i), x87::copy(r));
        }
        let f = mem.rebase(super::RCUI_COPY_VECTOR);
        call(mem, f, &[dst, work])
    }

    /// `rcuiInvertRigidMatrix(dst, m, work)` (`0x0093d090`): the inverse of
    /// a rotation-and-translation matrix: `m` transposed into `work` (64
    /// bytes), its last column replaced by `-(R^T t)` (each `-(t.y r1 +
    /// r0 t.x + t.z r2)`, `t` = `m`'s `+0xc`/`+0x1c`/`+0x2c`), the bottom
    /// row (0, 0, 0, 1); then `rcuiCopyMatrix(dst, work)`; returns that.
    pub fn rcui_invert_rigid_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        dst: u32,
        m: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        for i in 0..4u32 {
            for j in 0..4u32 {
                let v = x87::copy(mem.read(m.wrapping_add(16 * j + 4 * i)));
                mem.write(work.wrapping_add(16 * i + 4 * j), v);
            }
        }
        let ld = |mem: &M, at: u32| x87::load(mem.read(at));
        for r in 0..3u32 {
            let row = work.wrapping_add(16 * r);
            let s = ld(mem, m.wrapping_add(0x1c)) * ld(mem, row.wrapping_add(4))
                + ld(mem, row) * ld(mem, m.wrapping_add(0xc))
                + ld(mem, m.wrapping_add(0x2c)) * ld(mem, row.wrapping_add(8));
            mem.write(row.wrapping_add(0xc), x87::store(-s));
        }
        mem.write(work.wrapping_add(0x38), 0f32);
        mem.write(work.wrapping_add(0x34), 0f32);
        mem.write(work.wrapping_add(0x30), 0f32);
        mem.write(work.wrapping_add(0x3c), 1f32);
        let f = mem.rebase(super::RCUI_COPY_MATRIX);
        call(mem, f, &[dst, work])
    }

    /// `rcuiProjectToInt(out, m, v, raw, work)` (`0x0093d710`):
    /// `rcuiApplyMatrix(work, m, v)`, x/y/z divided by w (through `1/w`
    /// stored as a float), then the four as integers (`ftol`): times the
    /// `.rdata` double at `0x00b313a0` unless `raw` (w unchanged).
    /// Returns `1/w`, which the original leaves in `ST0`.
    pub fn rcui_project_to_int<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        out: u32,
        mv: (u32, u32),
        raw: u32,
        work: u32,
        call: &mut C,
    ) -> f32 {
        let f = mem.rebase(super::RCUI_APPLY_MATRIX);
        call(mem, f, &[work, mv.0, mv.1]);
        let w = x87::load(mem.read(work.wrapping_add(0xc)));
        let inv = x87::load(x87::store(1.0 / w));
        for i in 0..3u32 {
            let at = work.wrapping_add(4 * i);
            let v = x87::load(mem.read(at));
            mem.write(at, x87::store(v * inv));
        }
        let k = if raw == 0 {
            Some(mem.read::<f64>(mem.rebase(super::PROJECT_SCALE)))
        } else {
            None
        };
        for i in 0..3u32 {
            let v = x87::load(mem.read(work.wrapping_add(4 * i)));
            let r = ffx_bridge::x87::ftol(mem, k.map_or(v, |k| v * k));
            mem.write(out.wrapping_add(4 * i), r);
        }
        let r = ffx_bridge::x87::ftol(mem, k.map_or(w, |k| w * k));
        mem.write(out.wrapping_add(0xc), r);
        #[allow(clippy::cast_possible_truncation)] // a stored float
        let inv = inv as f32;
        inv
    }

    // ---- round 17 ----

    /// `FFXSamp0TransMatrix(dst, m, t)` (`0x0093d830`): `dst = m` (16
    /// floats through the x87 stack, row by row) with the translation
    /// (`+0x30`..`+0x38`) moved by `t`'s x, y, z. Returns `t`.
    pub fn ffx_samp0_trans_matrix(mem: &mut impl Mem, dst: u32, m: u32, t: u32) -> u32 {
        for i in 0..16u32 {
            let v = x87::copy(mem.read(m.wrapping_add(4 * i)));
            mem.write(dst.wrapping_add(4 * i), v);
        }
        for i in 0..3u32 {
            let a = x87::load(mem.read(m.wrapping_add(0x30 + 4 * i)));
            let b = x87::load(mem.read(t.wrapping_add(4 * i)));
            mem.write(dst.wrapping_add(0x30 + 4 * i), x87::store(a + b));
        }
        t
    }

    /// `rcuiScaleVec4(dst, v, s)` (`0x0093d7d0`): the four floats times
    /// `s`. Returns `dst`.
    pub fn rcui_scale_vec4(mem: &mut impl Mem, dst: u32, v: u32, s: u32) -> u32 {
        let s = x87::load(s);
        for i in 0..4u32 {
            let x = x87::load(mem.read(v.wrapping_add(4 * i)));
            mem.write(dst.wrapping_add(4 * i), x87::store(x * s));
        }
        dst
    }

    // ---- field round 41: 2D primitives ----

    /// `rcPrimDraw2D(kind, verts, n, colour)` (`0x00925f80`, glue: the
    /// renderer's 2D primitive).
    pub const RC_PRIM_DRAW_2D: u32 = 0x0092_5f80;
    /// `cgraphicSetNowUIType(type)` (`0x00638d90`).
    pub const SET_NOW_UI_TYPE: u32 = 0x0063_8d90;
    /// 256.0 (`f32`, `.rdata`): the depth of the 2D primitives' vertices.
    pub const PRIM_DEPTH: u32 = 0x00b5_01c0;

    /// `rcPrimLine(x0, y0, x1, y1, colour, ui)` (`0x00926c00`): the two
    /// vertices `(x, y, 256.0, 0.0)` (`f32`, through the x87 stack) at
    /// `work` (the original's locals, 0x20 bytes), then `rcPrimDraw2D(0x41,
    /// work, 2, colour)`, with `cgraphicSetNowUIType(2)` before it when
    /// `ui` is set and `cgraphicSetNowUIType(0)` after, whose `eax` it
    /// returns (the stack-cookie check leaves it).
    pub fn rc_prim_line<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 6],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let [x0, y0, x1, y1, colour, ui] = args;
        let depth = ffx_bridge::x87::round_trip(mem.read::<f32>(mem.rebase(PRIM_DEPTH)));
        let f = |b: u32| ffx_bridge::x87::round_trip(f32::from_bits(b));
        mem.write(work, [f(x0), f(y0), depth, 0.0, f(x1), f(y1), depth, 0.0]);
        draw_2d(mem, [0x41, work, 2, colour], ui, call)
    }

    /// `rcPrimDraw2D(kind, verts, n, colour)` between
    /// `cgraphicSetNowUIType(2)` (when `ui` is set) and
    /// `cgraphicSetNowUIType(0)`, whose `eax` it returns.
    fn draw_2d<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        draw: [u32; 4],
        ui: u32,
        call: &mut C,
    ) -> u32 {
        draw_2d_with(mem, SET_NOW_UI_TYPE, draw, ui, call)
    }

    /// [`draw_2d`] with the UI type set through `set_ui`.
    fn draw_2d_with<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        set_ui: u32,
        draw: [u32; 4],
        ui: u32,
        call: &mut C,
    ) -> u32 {
        let ui_type = mem.rebase(set_ui);
        if ui != 0 {
            call(mem, ui_type, &[2]);
        }
        let f = mem.rebase(RC_PRIM_DRAW_2D);
        call(mem, f, &draw);
        call(mem, ui_type, &[0])
    }

    /// The circle's height factor: 1.1666666 (`f64`, `0x00b67698`) times
    /// 416.0 (`0x00b41d30`) over 448.0 (`0x00b50210`).
    pub const CIRCLE_FACTORS: [u32; 3] = [0x00b6_7698, 0x00b4_1d30, 0x00b5_0210];

    /// `rcPrimCircleLine(x, y, r, colour, ui)` (`0x00925ea0`): a circle
    /// for the renderer as 65 vertices at `work` (0x410 bytes): its box
    /// `(x - r, y - k)` and `(x + r, y + k)` (depth 256.0, `w` 0), then 63
    /// zero vertices, `k = r * 1.1666666 * 416 / 448` (each step in `f64`,
    /// at the x87's 53 bits), drawn as `rcPrimDraw2D(0x42, work, 0x41,
    /// colour)` with the UI type as in [`rc_prim_line`].
    pub fn rc_prim_circle_line<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 5],
        work: u32,
        call: &mut C,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        let [x, y, r, colour, ui] = args;
        let [c1, c2, c3] = CIRCLE_FACTORS.map(|a| mem.read::<f64>(mem.rebase(a)));
        let (x, y, r) = (load(x), load(y), load(r));
        let k = div(mul(mul(c1, r), c2), c3);
        let depth = ffx_bridge::x87::round_trip(mem.read::<f32>(mem.rebase(PRIM_DEPTH))).to_bits();
        let v = [
            store(sub(x, r)),
            store(sub(y, k)),
            depth,
            0,
            store(add(r, x)),
            store(add(k, y)),
            depth,
            0,
        ];
        mem.write(work, v);
        mem.write(work.wrapping_add(0x20), [0u32; 63 * 4]);
        draw_2d(mem, [0x42, work, 0x41, colour], ui, call)
    }

    /// The `ppp` sine table (4096 `f32`, a full turn).
    pub const SIN_TABLE: u32 = 0x00c4_4bf0;
    /// `rcPrimTri`'s angle scale: times `0x00b51558`, over `0x00b39378`
    /// (`f64` each), truncated to the table's 16.4 index.
    pub const TRI_ANGLE: [u32; 2] = [0x00b5_1558, 0x00b3_9378];
    /// The depth of `rcPrimTri`'s second and third vertices (`f32`).
    pub const TRI_DEPTH: u32 = 0x00b6_64d8;

    /// `rcPrimTri(x, y, r, angle, colour, ui)` (`0x009270c0`): a triangle
    /// inscribed in the circle of [`rc_prim_circle_line`], its corners at
    /// `angle` and a third of a turn on either side. With `i` the angle as
    /// a table index (`angle * A / B`, `_ftol2_sse`) and `t(o)` the sine
    /// table at `(i + o) >> 4 & 0xfff` (arithmetic shift), each corner is
    /// `x + t(a) * r`, `y + t(b) * r * 1.1666666 * 416 / 448`, for `(a, b)` =
    /// (0x4000, 0x8000), (0xa38e, 0xe38e), (0xdc71, 0x11c71): every step at
    /// the x87's 53 bits in the original's operand order, rounded to `f32`
    /// at the store. Depths: 256.0 for the first, `0x00b664d8` for the
    /// others; `w` 0. Drawn as `rcPrimDraw2D(0x4c, work, 3, colour)` with
    /// the UI type calls of [`rc_prim_line`].
    pub fn rc_prim_tri<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 6],
        work: u32,
        call: &mut C,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, ftol, load, mul, store};
        let [x, y, r, angle, colour, ui] = args;
        let rd = |mem: &M, a: u32| mem.read::<f64>(mem.rebase(a));
        let (sa, sb) = (rd(mem, TRI_ANGLE[0]), rd(mem, TRI_ANGLE[1]));
        let i = ftol(mem, div(mul(load(angle), sa), sb)).cast_signed();
        let table = mem.rebase(SIN_TABLE);
        let t = |mem: &M, o: i32| -> f64 {
            let k = (i.wrapping_add(o) >> 4) & 0xfff;
            load(mem.read::<u32>(table + 4 * k.cast_unsigned()))
        };
        let [c1, c2, c3] = CIRCLE_FACTORS.map(|a| rd(mem, a));
        let (x, y, r) = (load(x), load(y), load(r));
        let depth = ffx_bridge::x87::round_trip(mem.read::<f32>(mem.rebase(PRIM_DEPTH))).to_bits();
        let depth2 = ffx_bridge::x87::round_trip(mem.read::<f32>(mem.rebase(TRI_DEPTH))).to_bits();
        let v0x = add(mul(t(mem, 0x4000), r), x);
        let v0y = add(div(mul(mul(mul(r, t(mem, 0x8000)), c1), c2), c3), y);
        let v1x = add(x, mul(t(mem, 0xa38e), r));
        let v1y = add(div(mul(mul(mul(t(mem, 0xe38e), r), c1), c2), c3), y);
        let v2x = add(mul(t(mem, 0xdc71), r), x);
        let v2y = add(y, div(mul(c2, mul(c1, mul(t(mem, 0x1_1c71), r))), c3));
        let v = [
            store(v0x),
            store(v0y),
            depth,
            0,
            store(v1x),
            store(v1y),
            depth2,
            0,
            store(v2x),
            store(v2y),
            depth2,
            0,
        ];
        mem.write(work, v);
        draw_2d(mem, [0x4c, work, 3, colour], ui, call)
    }

    /// `graphicSetNowUIType(type)` (`0x006437c0`), the setter `rcPrimBox`
    /// uses.
    pub const GRAPHIC_SET_NOW_UI_TYPE: u32 = 0x0064_37c0;
    /// 0.5 (`f64`, `0x00b92220`): `rcPrimBox`'s half.
    pub const BOX_HALF: u32 = 0x00b9_2220;

    /// `rcPrimBox(x, y, w, h, colour, ui)` (`0x00925b60`): the box of
    /// size `w` by `h` centred on `(x, y)` as four vertices at `work`:
    /// `(x - w/2, y - h/2)`, `(x + w/2, y - h/2)`, `(x - w/2, y + h/2)`,
    /// `(x + w/2, y + h/2)` (depth 256.0, `w` 0). The halves are `f32(w *
    /// 0.5)` and `f32(0.5 * h)`, each corner coordinate is stored as `f32`.
    /// Drawn as `rcPrimDraw2D(0x44, work, 4, colour)` between
    /// `graphicSetNowUIType(2)` (when `ui` is set) and
    /// `graphicSetNowUIType(0)`, whose `eax` it returns.
    pub fn rc_prim_box<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 6],
        work: u32,
        call: &mut C,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let [x, y, w, h, colour, ui] = args;
        let k = mem.read::<f64>(mem.rebase(BOX_HALF));
        let hw = load(store(mul(load(w), k)));
        let hh = load(store(mul(k, load(h))));
        let (x, y) = (load(x), load(y));
        let depth = ffx_bridge::x87::round_trip(mem.read::<f32>(mem.rebase(PRIM_DEPTH))).to_bits();
        let (left, top) = (store(sub(x, hw)), store(sub(y, hh)));
        let (right, bottom) = (store(add(x, hw)), store(add(y, hh)));
        let v = [
            left, top, depth, 0, right, top, depth, 0, left, bottom, depth, 0, right, bottom,
            depth, 0,
        ];
        mem.write(work, v);
        draw_2d_with(
            mem,
            GRAPHIC_SET_NOW_UI_TYPE,
            [0x44, work, 4, colour],
            ui,
            call,
        )
    }

    /// 0.6666667 (`f64`, `0x00b5def8`): `rcPrimBatu`'s inner corners.
    pub const BATU_INNER: u32 = 0x00b5_def8;

    /// `rcPrimBatu(x, y, size, colour, ui)` (`0x009259c0`): a cross (the
    /// "batsu" mark) of two quads at `work` (0x40 bytes), drawn with
    /// `rcPrimDraw2D(0x44, work, 4, colour)` each between
    /// `cgraphicSetNowUIType(2)` (when `ui` is set) and
    /// `cgraphicSetNowUIType(0)`, whose `eax` it returns. With `h =
    /// f32(size * 0.5)`, `a = f32(h * 0.6666667)` and the heights `p`, `q`
    /// of `h` and `a` (each `f32(v * 1.1666666 * 416 / 448)`, in `f64`), the
    /// first quad is `(x - h, y - q)`, `(x - a, y - p)`, `(x + a, y + p)`,
    /// `(x + h, y + q)` (depth 256.0, `w` 0), the second `(x + a, y - p)`,
    /// `(x + h, y - q)`, `(x - h, y + q)`, `(x - a, y + p)` (its `w` words
    /// left as the first draw leaves them). Coordinates are stored as
    /// `f32`.
    pub fn rc_prim_batu<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 5],
        work: u32,
        call: &mut C,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let [x, y, size, colour, ui] = args;
        let k = |mem: &M, at: u32| mem.read::<f64>(mem.rebase(at));
        let h = load(store(mul(load(size), k(mem, BOX_HALF))));
        let a = load(store(mul(h, k(mem, BATU_INNER))));
        let [kb, kc, kd] = CIRCLE_FACTORS.map(|at| k(mem, at));
        let height = |v: f64| load(store(mul(mul(v, kb), kc) / kd));
        let (p, q) = (height(h), height(a));
        let (x, y) = (load(x), load(y));
        let depth = ffx_bridge::x87::round_trip(mem.read::<f32>(mem.rebase(PRIM_DEPTH))).to_bits();
        let (left, right) = (store(sub(x, h)), store(add(x, h)));
        let (inner_left, inner_right) = (store(sub(x, a)), store(add(x, a)));
        let (top, bottom) = (store(sub(y, q)), store(add(y, q)));
        let (inner_top, inner_bottom) = (store(sub(y, p)), store(add(y, p)));
        let first = [
            [left, top],
            [inner_left, inner_top],
            [inner_right, inner_bottom],
            [right, bottom],
        ];
        for (i, [vx, vy]) in (0u32..).zip(first) {
            mem.write(work.wrapping_add(16 * i), [vx, vy, depth, 0]);
        }
        let ui_type = mem.rebase(SET_NOW_UI_TYPE);
        if ui != 0 {
            call(mem, ui_type, &[2]);
        }
        let draw = mem.rebase(RC_PRIM_DRAW_2D);
        call(mem, draw, &[0x44, work, 4, colour]);
        let second = [
            [inner_right, inner_top],
            [right, top],
            [left, bottom],
            [inner_left, inner_bottom],
        ];
        for (i, [vx, vy]) in (0u32..).zip(second) {
            mem.write(work.wrapping_add(16 * i), [vx, vy, depth]);
        }
        call(mem, draw, &[0x44, work, 4, colour]);
        call(mem, ui_type, &[0])
    }

    // ---- field r13, axis rotations ----

    /// `rcuiMulMatrix(out, a, b)` (`0x0093d1b0`).
    pub const MUL_MATRIX: u32 = 0x0093_d1b0;

    /// An axis for [`rcui_rotate`]: where `cos`, `-sin`, `sin`, `cos` go
    /// in the 4x4 matrix (word indices).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Axis {
        /// `0x0093d5a0`: rows/columns 1 and 2.
        X,
        /// `0x0093d620`: rows/columns 0 and 2, `-sin` above the diagonal.
        Y,
        /// `0x0093d690`: rows/columns 0 and 1.
        Z,
    }

    impl Axis {
        /// The word indices of `cos`, `-sin`, `sin` and `cos`.
        const fn slots(self) -> [u32; 4] {
            match self {
                Self::X => [5, 6, 9, 10],
                Self::Y => [0, 2, 8, 10],
                Self::Z => [0, 1, 4, 5],
            }
        }
    }

    /// `rcuiRotate{X,Y,Z}(out, m, angle)` (`0x0093d5a0`, `0x0093d620`,
    /// `0x0093d690`, from `0x0093d490` and `0x0093d550`; names ours): the
    /// rotation by `angle` about `axis` (an identity in `work`, the
    /// original's stack matrix, with `cos` and `sin` from `_CIcos` and
    /// `_CIsin`, each stored as an `f32`, and `-sin`) times `m` into `out`
    /// (`rcuiMulMatrix(out, rotation, m)`), whose `eax` it returns.
    pub fn rcui_rotate<M: Mem, C, F>(
        mem: &mut M,
        axis: Axis,
        [out, m, angle]: [u32; 3],
        work: u32,
        call: &mut C,
        fcall: &mut F,
    ) -> u32
    where
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        F: FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    {
        for i in 0..16u32 {
            let one = if i % 5 == 0 { 1.0f32 } else { 0.0 };
            mem.write(work.wrapping_add(4 * i), one);
        }
        let a = Reg::load(f32::from_bits(angle)).value();
        let f = mem.rebase(crate::vu0::CI_COS);
        let c = Reg::constant(fcall(mem, f, crate::camera::FArgs::St0(a))).store();
        let f = mem.rebase(crate::vu0::CI_SIN);
        let s = Reg::constant(fcall(mem, f, crate::camera::FArgs::St0(a))).store();
        let minus_s = (-Reg::load(s)).store();
        let [i0, i1, i2, i3] = axis.slots();
        for (i, v) in [(i0, c), (i1, minus_s), (i2, s), (i3, c)] {
            mem.write(work.wrapping_add(4 * i), v);
        }
        let f = mem.rebase(MUL_MATRIX);
        call(mem, f, &[out, work, m])
    }

    /// `rcuiRotateX`, `rcuiRotateY` and `rcuiRotateZ` (`0x0093d5a0`,
    /// `0x0093d620`, `0x0093d690`): [`rcui_rotate_zyx`]'s steps.
    pub const ROTATE_XYZ: [u32; 3] = [0x0093_d5a0, 0x0093_d620, 0x0093_d690];

    /// `rcuiRotateZYX_0093d550(out, m, angles)` (`0x0093d550`; name ours):
    /// `m` rotated about z by `angles[2]` into `out`, then `out` about y by
    /// `angles[1]` and about x by `angles[0]` (three `f32`s, each copied
    /// through the x87 and read just before its step, after the step
    /// before has written `out`). Returns `rcuiRotateX`'s `eax`.
    pub fn rcui_rotate_zyx<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [out, m, angles]: [u32; 3],
        call: &mut C,
    ) -> u32 {
        let [x, y, z] = ROTATE_XYZ.map(|f| mem.rebase(f));
        let angle = |mem: &M, i: u32| crate::x87::copy(mem.read::<u32>(angles.wrapping_add(4 * i)));
        let a = angle(mem, 2);
        call(mem, z, &[out, m, a]);
        let a = angle(mem, 1);
        call(mem, y, &[out, out, a]);
        let a = angle(mem, 0);
        call(mem, x, &[out, out, a])
    }
}
