//! The render-side glue, thirty-sixth batch (M4 port round 8, list
//! `h3-glue-render`): the character lighting's object lists from the two
//! map worlds.

/// The scene cursor's first step `0x0065eb60(this)` and next
/// `PhyreSceneCursor_Next2(this)`, the class walked, the array resize
/// `0x0066c240(array; n)` (`ret 4`), the aligned `PhyreMalloc(n, 4)` and
/// `PhyreFree` (cdecl), `Phyre_CharacterLighting_GetInstance` and
/// `isInitSuccessful(this)` (`al`).
pub const CURSOR_BEGIN: u32 = 0x0065_eb60;
pub const CURSOR_NEXT: u32 = 0x0065_ed80;
pub const OBJECT_CLASS: u32 = 0x00c9_af80;
pub const ARRAY_RESIZE: u32 = 0x0066_c240;
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const LIGHTING_INSTANCE: u32 = 0x006a_6e30;
pub const LIGHTING_READY: u32 = 0x006a_9ad0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::h3_glue14::bridge::{cursor_over, walk};
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The objects of `world`'s class walked with the cursor at `cursor`:
    /// their count.
    fn count(h: &mut impl Host, cursor: u32, world: u32) -> u32 {
        cursor_over(h, cursor, world, OBJECT_CLASS);
        h.call_sym(CURSOR_BEGIN, Conv::Thiscall(cursor), &[]);
        let mut n = 0u32;
        walk(h, cursor, CURSOR_NEXT, |_, _| n = n.wrapping_add(1));
        n
    }

    /// The same walk storing each object's address (element + the
    /// cursor's offset) from `out` on.
    fn collect(h: &mut impl Host, cursor: u32, world: u32, out: u32) {
        cursor_over(h, cursor, world, OBJECT_CLASS);
        h.call_sym(CURSOR_BEGIN, Conv::Thiscall(cursor), &[]);
        let mut at = out;
        walk(h, cursor, CURSOR_NEXT, |h, base| {
            let offset: u32 = h.read(cursor + 0x10);
            h.write(at, offset.wrapping_add(base));
            at = at.wrapping_add(4);
        });
    }

    /// The light colour (`+0xc0` .. `+0xc8` of the scene's `+0x4f8`)
    /// copied to the character lighting record (`+0xcc`), as `fld` /
    /// `fstp` copy floats.
    fn copy_light(h: &mut impl Host, this: u32, light: u32) {
        let out: u32 = h.read(this + 0xcc);
        for k in 0..3 {
            let v = x87::store(x87::load(h.read(light + 0xc0 + 4 * k)));
            h.write(out + 4 * k, v);
        }
    }

    /// No objects: the light colour when the character lighting started
    /// and the scene has a light, else zero.
    fn light_or_zero(h: &mut impl Host, this: u32) {
        let lighting = h.call_sym(LIGHTING_INSTANCE, Conv::Cdecl, &[]);
        let ready = h.call_sym(LIGHTING_READY, Conv::Thiscall(lighting), &[]) as u8;
        let light: u32 = h.read(this + 0x4f8);
        if ready != 0 && light != 0 {
            copy_light(h, this, light);
        } else {
            let out: u32 = h.read(this + 0xcc);
            for k in 0..3 {
                h.write(out + 4 * k, 0f32);
            }
        }
    }

    /// `0x00651de0(this)` (thiscall; frame: the cursor at `ebp - 0x28`, a
    /// new array at `- 8`, `this` at `- 4`): for each map world (`+0x10d4`,
    /// then `+0x1128`) the objects of class `0x00c9af80` are counted, the
    /// character lighting record's object array (`+0xcc`, `+0x9c` count,
    /// `+0xa0` data) sized to them (the first world through `0x0066c240`,
    /// the second inline, keeping the old array when the allocation
    /// fails) and filled with their addresses; then the scene light's
    /// colour is copied. Without objects the colour is copied only when
    /// the lighting started, else zeroed. Returns 0.
    pub fn lighting_objects(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let cursor = ebp - 0x28;
        h.write(ebp - 4, this);
        let world: u32 = h.read(this + 0x10d4);
        if world != 0 {
            let n = count(h, cursor, world);
            if n == 0 {
                light_or_zero(h, this);
            } else {
                let record: u32 = h.read(this + 0xcc);
                h.call_sym(ARRAY_RESIZE, Conv::Thiscall(record + 0x9c), &[n]);
                let world: u32 = h.read(this + 0x10d4);
                let record: u32 = h.read(this + 0xcc);
                let out: u32 = h.read(record + 0xa0);
                collect(h, cursor, world, out);
                let light: u32 = h.read(this + 0x4f8);
                if light != 0 {
                    copy_light(h, this, light);
                }
            }
        }
        let world: u32 = h.read(this + 0x1128);
        if world == 0 {
            return 0;
        }
        let n = count(h, cursor, world);
        if n == 0 {
            light_or_zero(h, this);
            return 0;
        }
        let record: u32 = h.read(this + 0xcc);
        if n != h.read::<u32>(record + 0x9c) {
            let data = h.call_sym(PHYRE_MALLOC_ALIGNED, Conv::Cdecl, &[n.wrapping_mul(4), 4]);
            h.write(ebp - 8, data);
            if data != 0 {
                for k in 0..n {
                    h.write(data.wrapping_add(4 * k), 0u32);
                }
                let old: u32 = h.read(record + 0xa0);
                let borrowed = h.read::<u32>(record + 0x9c) & 0x8000_0000 != 0;
                if old != data && !borrowed {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
                h.write(record + 0xa0, data);
                h.write(record + 0x9c, n);
            }
        }
        let world: u32 = h.read(this + 0x1128);
        let record: u32 = h.read(this + 0xcc);
        let out: u32 = h.read(record + 0xa0);
        collect(h, cursor, world, out);
        let light: u32 = h.read(this + 0x4f8);
        if light != 0 {
            copy_light(h, this, light);
        }
        0
    }
}
