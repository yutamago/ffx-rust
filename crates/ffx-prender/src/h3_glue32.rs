//! The render-side glue, thirty-second batch (M4 port round 8, list
//! `h3-glue-render`): a VFX's render data and the debug map list's text.
//! The cookie is left out, as in [`crate::glue10`].

use ffx_bridge::Global;

/// `g_VFXRenderDataTable`, its lookup `GetDynamicGeometryByInstance(this;
/// a, b)` and pool `GetVFXDynamicGeometryFromPool(this; &data, &geometry)`
/// (`ret 8`), `operator new` (cdecl), the geometry's constructor,
/// `Insert(this; a, b, geometry)` (`ret 0xc`), the render data's far
/// corners (-10000, 10000) and `g_VfxCurrentType` (a byte).
pub const VFX_RENDER_DATA_TABLE: Global<u32> = crate::h3_glue15::VFX_RENDER_DATA_TABLE;
pub const GET_DYNAMIC_GEOMETRY: u32 = crate::h3_glue15::GET_DYNAMIC_GEOMETRY;
pub const GEOMETRY_FROM_POOL: u32 = 0x0069_f750;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const GEOMETRY_CTOR: u32 = 0x0069_e3e0;
pub const VFX_TABLE_INSERT: u32 = crate::h3_glue15::VFX_TABLE_INSERT;
pub const FAR_LOW: u32 = 0x00b4_1db0;
pub const FAR_HIGH: u32 = 0x00b1_0f4c;
pub const VFX_CURRENT_TYPE: Global<u8> = Global::new(0x00c3_22aa);

/// `TextUtil_init(this)`, `TextUtil_setEnabled(this; b)` and
/// `TextUtil_appendLine(this; text)` (`ret 4`), `sprintf_s` (cdecl,
/// through its import slot), the 42 map-name prefixes (8 bytes apart),
/// `"init failed"`, `"%s%s%02d"` and the row markers `">"` and `" "`.
pub const TEXT_INIT: u32 = 0x006f_6370;
pub const TEXT_SET_ENABLED: u32 = 0x006f_68f0;
pub const TEXT_APPEND_LINE: u32 = 0x006f_5e30;
pub const SPRINTF_S: u32 = 0x00b0_c394;
pub const MAP_PREFIXES: u32 = 0x00b4_8700;
pub const MAP_COUNT: u32 = 42;
pub const INIT_FAILED: u32 = 0x00b4_8244;
pub const ROW_FORMAT: u32 = 0x00b4_8850;
pub const CURRENT_MARK: u32 = 0x00b4_8250;
pub const OTHER_MARK: u32 = 0x00b2_0b30;
/// The rows shown.
pub const ROWS: u32 = 10;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// A float constant of the image as `fld dword` / `fst dword` copy
    /// it.
    fn constant(h: &impl Host, sym: u32) -> u32 {
        x87::store(x87::load(h.read(h.rebase(sym))))
    }

    /// `graphicVFXCreate(a, b, arg; slots)` (cdecl; frame: the geometry
    /// at `ebp - 4`): unless `a` and `b` are both 0 or the table already
    /// holds a geometry for them, render data (`b`'s slot) and a geometry
    /// come from the table's pool, or are made (0xd0 bytes, and a 0xf8
    /// byte `VFXDynamicGeometry`); the data gets two identity matrices,
    /// the far corners, its flags and defaults; the geometry gets the
    /// data (`+0x24`), `arg` (`+0x3c`) and the current VFX type
    /// (`+0x51`), and goes into the table under (`a`, `b`). `eax`: 0
    /// for no key, the geometry found, or the insert's.
    pub fn vfx_create(h: &mut impl Host, ebp: u32, slots: u32, a: u32, b: u32, arg: u32) -> u32 {
        if a | b == 0 {
            return 0;
        }
        let table = VFX_RENDER_DATA_TABLE.get(h);
        let found = h.call_sym(GET_DYNAMIC_GEOMETRY, Conv::Thiscall(table), &[a, b]);
        if found != 0 {
            return found;
        }
        let table = VFX_RENDER_DATA_TABLE.get(h);
        let pooled = h.call_sym(
            GEOMETRY_FROM_POOL,
            Conv::Thiscall(table),
            &[slots + 4, ebp - 4],
        );
        if pooled == 0 {
            let data = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xd0]);
            h.write(slots + 4, data);
            let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xf8]);
            let geometry = if p == 0 {
                0
            } else {
                h.call_sym(GEOMETRY_CTOR, Conv::Thiscall(p), &[])
            };
            h.write(ebp - 4, geometry);
        }
        let data: u32 = h.read(slots + 4);
        for m in [0, 0x40] {
            for k in 0..16 {
                let one = k % 5 == 0;
                h.write(data + m + 4 * k, if one { 1f32 } else { 0f32 });
            }
        }
        let low = constant(h, FAR_LOW);
        let high = constant(h, FAR_HIGH);
        for k in 0..3 {
            h.write(data + 0x98 + 4 * k, low);
            h.write(data + 0xa4 + 4 * k, high);
        }
        h.write(data + 0x80, 0u32);
        h.write(data + 0xb8, 2u32);
        h.write(data + 0xbc, 0u32);
        h.write(data + 0xc0, 0u8);
        h.write(data + 0xc4, 1u16);
        h.write(data + 0xc8, u32::MAX);
        h.write(data + 0xc1, 0u8);
        h.write(data + 0xc2, 0xffffu16);
        h.write(data + 0x84, 0u32);
        h.write(data + 0x8c, 0f32);
        h.write(data + 0x88, 0f32);
        h.write(data + 0xb6, 0u8);
        h.write(data + 0xb7, 0u8);
        let geometry: u32 = h.read(ebp - 4);
        h.write(geometry.wrapping_add(0x24), data);
        h.write(geometry.wrapping_add(0x3c), arg);
        let kind = VFX_CURRENT_TYPE.get(h);
        h.write(geometry.wrapping_add(0x51), kind);
        let table = VFX_RENDER_DATA_TABLE.get(h);
        h.call_sym(VFX_TABLE_INSERT, Conv::Thiscall(table), &[a, b, geometry])
    }

    /// `0x006b9e20(this)` (thiscall; frame: the text object at `ebp -
    /// 0x1c`, a 20-byte line at `- 0x18`): the debug map list. When its
    /// text (`+4`) initialises, the list is on (`+0`), gets the 42 map
    /// prefixes (`+0x94`), zero counts (`+0x13c`) and row 0 current
    /// (`+0x1e8`), and the rows from `+0x1ec` up to 10 become lines
    /// (`"%s%s%02d"`: `">"` on the current row, the prefix and the count);
    /// else the list is off. The text is then disabled. `eax`:
    /// `setEnabled`'s.
    pub fn map_list_init(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let text = this + 4;
        h.write(ebp - 0x1c, text);
        let failed = h.call_sym(TEXT_INIT, Conv::Thiscall(text), &[]);
        if failed != 0 {
            let r = h.call_sym(TEXT_SET_ENABLED, Conv::Thiscall(text), &[0]);
            h.write(this, 0u8);
            return r;
        }
        h.write(this, 1u8);
        for k in 0..MAP_COUNT {
            let name = h.rebase(MAP_PREFIXES + 8 * k);
            h.write(this + 0x94 + 4 * k, name);
        }
        for k in 0..MAP_COUNT {
            h.write(this + 0x13c + 4 * k, 0u32);
        }
        h.write(this + 0x1e8, 0u32);
        let first: u32 = h.read(this + 0x1ec);
        let mut row = first;
        while (row as i32) < ROWS as i32 {
            let line = ebp - 0x18;
            for k in 0..5 {
                h.write(line + 4 * k, 0u32);
            }
            let sprintf: u32 = h.read(h.rebase(SPRINTF_S));
            if h.read::<u8>(this) == 0 {
                let format = h.rebase(INIT_FAILED);
                h.call(sprintf, Conv::Cdecl, &[line, 0x14, format]);
            } else {
                let current: u32 = h.read(this + 0x1e8);
                let mark = h.rebase(if current == row {
                    CURRENT_MARK
                } else {
                    OTHER_MARK
                });
                let at = this + 0x94 + row.wrapping_mul(4);
                let name: u32 = h.read(at);
                let count: u32 = h.read(at + 0xa8);
                let format = h.rebase(ROW_FORMAT);
                h.call(
                    sprintf,
                    Conv::Cdecl,
                    &[line, 0x14, format, mark, name, count],
                );
            }
            h.call_sym(TEXT_APPEND_LINE, Conv::Thiscall(text), &[line]);
            row = row.wrapping_add(1);
        }
        h.call_sym(TEXT_SET_ENABLED, Conv::Thiscall(text), &[0])
    }
}
