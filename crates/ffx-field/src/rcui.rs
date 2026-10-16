//! `rcui`: the field UI helpers (mouse drag state, number input, vector and
//! matrix copies).

use ffx_bridge::Global;

/// The UI work area (`rcuiMouseMove*`).
pub const WORK: Global<u32> = Global::new(0x0193_c0f0);

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
}
