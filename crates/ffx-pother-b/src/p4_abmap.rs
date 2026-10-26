//! The sphere grid's render object (`g_GraphicAbmap`, 0xe8 bytes): its
//! constructor.
//!
//! Layout: the owner word (`+0`), two words set to 1 (`+4`, `+8`), twelve
//! `MenuDynamicGeometry` pointers sign-extended to 64 bits (`+0x10..0x88`,
//! eight made inline, four by `MenuDynamicGeometry_create`), and 23
//! render data (`+0x88..0xe0`, from `0x00681c10`), each with its segment
//! count (`+0x90`) and 0x6c-byte segments (`+0x94`).

use ffx_graphic::host::{Conv, Host};

use crate::p4_mesh::{GEOMETRY_HIDE, MENU_GEOMETRY_CTOR, OPERATOR_NEW, PHYRE_SCENE};

/// `0x00681c10` (thiscall `(abmap, kind)`, `ret 4`) -> new render data.
pub const CREATE_RENDER_DATA: u32 = 0x0068_1c10;
/// `MenuDynamicGeometry_create_00681970` (stdcall `(owner, kind)`, `ret
/// 8`; `ecx` unread) -> the geometry in `edx:eax`, sign-extended.
pub const MENU_GEOMETRY_CREATE: u32 = 0x0068_1970;

/// The render data the constructor creates: (kind, field), in call
/// order.
const RENDER_DATA: [(u32, u32); 23] = [
    (0, 0x88),
    (1, 0x8c),
    (2, 0x90),
    (3, 0x94),
    (4, 0x98),
    (4, 0xb0),
    (0xb, 0x9c),
    (0xb, 0xb4),
    (5, 0xa0),
    (5, 0xb8),
    (6, 0xa4),
    (6, 0xbc),
    (0xc, 0xa8),
    (0xc, 0xc0),
    (0xd, 0xac),
    (0xd, 0xc4),
    (7, 0xc8),
    (8, 0xcc),
    (9, 0xd0),
    (9, 0xd4),
    (9, 0xd8),
    (9, 0xdc),
    (0xa, 0xe0),
];

/// One geometry the constructor makes.
#[derive(Clone, Copy)]
enum Geometry {
    /// Inline: (render data field, the scene's hidden byte, its value
    /// word).
    Inline(u32, u32, u32),
    /// `MenuDynamicGeometry_create(render data field's word, 1)`.
    Create(u32),
}

/// The geometries in order, with the 64-bit field each goes to.
const GEOMETRIES: [(Geometry, u32); 12] = [
    (Geometry::Inline(0x88, 0xfa9f, 0xfa90), 0x10),
    (Geometry::Create(0x8c), 0x18),
    (Geometry::Create(0x90), 0x20),
    (Geometry::Create(0x94), 0x28),
    (Geometry::Inline(0xcc, 0xfa9a, 0xfa78), 0x30),
    (Geometry::Inline(0xd0, 0xfa98, 0xfa70), 0x38),
    (Geometry::Create(0xd4), 0x40),
    (Geometry::Inline(0xe0, 0xfa99, 0xfa74), 0x48),
    (Geometry::Inline(0xc8, 0xfa9a, 0xfa78), 0x50),
    (Geometry::Inline(0x98, 0xfa9a, 0xfa78), 0x58),
    (Geometry::Create(0xa0), 0x60),
    (Geometry::Inline(0x9c, 0xfaa0, 0xfa94), 0x68),
];

/// The rest after [`GEOMETRIES`]: two more pairs.
const GEOMETRIES2: [(Geometry, u32); 3] = [
    (Geometry::Create(0xa4), 0x70),
    (Geometry::Inline(0xa8, 0xfaa0, 0xfa94), 0x78),
    (Geometry::Create(0xac), 0x80),
];

/// The render data whose segments the constructor clears, in order.
const CLEARED: [u32; 23] = [
    0x88, 0x8c, 0x90, 0x94, 0x98, 0x9c, 0xa0, 0xa4, 0xac, 0xa8, 0xb0, 0xb4, 0xb8, 0xbc, 0xc4, 0xc0,
    0xc8, 0xcc, 0xd0, 0xd4, 0xd8, 0xdc, 0xe0,
];

pub mod bridge {
    //! The originals over game memory.

    use super::{
        CLEARED, CREATE_RENDER_DATA, Conv, GEOMETRIES, GEOMETRIES2, GEOMETRY_HIDE, Geometry, Host,
        MENU_GEOMETRY_CREATE, MENU_GEOMETRY_CTOR, OPERATOR_NEW, PHYRE_SCENE, RENDER_DATA,
    };

    /// A `MenuDynamicGeometry` made inline (as `MenuDynamicGeometry_create`
    /// does for a known kind): the scene's hidden byte and, shown, its
    /// value word (read before the allocation); a new geometry (0x3c
    /// bytes, constructed when the allocation succeeds) with `owner` at
    /// `+0x24`, then `vtable[1](value, scene + 0x1284)` and `vtable[2](0)`
    /// or `ClassDynamicGeometry_hide`. Returns it and the hidden byte.
    fn inline(h: &mut impl Host, owner: u32, flag: u32, word: u32) -> (u32, u8) {
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        let hidden: u8 = h.read(scene.wrapping_add(flag));
        let value: u32 = if hidden == 0 {
            h.read(scene.wrapping_add(word))
        } else {
            0
        };
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x3c]);
        let geom = if p == 0 {
            0
        } else {
            h.call_sym(MENU_GEOMETRY_CTOR, Conv::Thiscall(p), &[])
        };
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        h.write(geom.wrapping_add(0x24), owner);
        if hidden == 0 {
            h.call_virtual(geom, 1, &[value, scene.wrapping_add(0x1284)]);
            h.call_virtual(geom, 2, &[0]);
        } else {
            h.call_sym(GEOMETRY_HIDE, Conv::Thiscall(geom), &[]);
        }
        (geom, hidden)
    }

    /// `GraphicAbmap::GraphicAbmap(abmap, owner)` (`0x00680970`,
    /// thiscall, `ret 4`) -> `abmap`: `+0 = owner`, `+4 = +8 = 1`, the 23
    /// render data (`0x00681c10(abmap, kind)`), the twelve geometries
    /// (each sign-extended into its 64-bit field), then every render
    /// data's segments get their words `+4` and `+8` zeroed. Also returns
    /// the word the original leaves in its `owner` argument's slot: the
    /// last inline geometry's hidden byte in the top byte (the original
    /// keeps it at `[ebp + 0xb]`).
    pub fn ctor(h: &mut impl Host, abmap: u32, owner: u32) -> (u32, u32) {
        h.write(abmap, owner);
        h.write(abmap + 4, 1u32);
        h.write(abmap + 8, 1u32);
        for (kind, field) in RENDER_DATA {
            let r = h.call_sym(CREATE_RENDER_DATA, Conv::Thiscall(abmap), &[kind]);
            h.write(abmap + field, r);
        }
        let mut slot = owner;
        for (g, field) in GEOMETRIES.into_iter().chain(GEOMETRIES2) {
            let geom = match g {
                Geometry::Inline(data, flag, word) => {
                    let o: u32 = h.read(abmap + data);
                    let (geom, hidden) = inline(h, o, flag, word);
                    slot = (slot & 0x00ff_ffff) | (u32::from(hidden) << 24);
                    geom
                }
                Geometry::Create(data) => {
                    let o: u32 = h.read(abmap + data);
                    // `ecx` is the constructor's `this` at the call
                    h.call_sym(MENU_GEOMETRY_CREATE, Conv::Thiscall(abmap), &[o, 1])
                }
            };
            h.write(abmap + field, geom);
            // `cdq`: the high word is the sign
            h.write(abmap + field + 4, ((geom as i32) >> 31) as u32);
        }
        for field in CLEARED {
            let mut i = 0i32;
            loop {
                let data: u32 = h.read(abmap + field);
                if i >= h.read::<i32>(data + 0x90) {
                    break;
                }
                let segs: u32 = h.read(data + 0x94);
                h.write(
                    segs.wrapping_add((i as u32).wrapping_mul(0x6c))
                        .wrapping_add(4),
                    0u32,
                );
                let segs: u32 = h.read(data + 0x94);
                h.write(
                    segs.wrapping_add((i as u32).wrapping_mul(0x6c))
                        .wrapping_add(8),
                    0u32,
                );
                i += 1;
            }
        }
        (abmap, slot)
    }
}
