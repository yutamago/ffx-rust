//! Small methods of the input devices, the character-lighting and fog
//! managers and the remaster's dynamic meshes: getters, setters and
//! one-call forwarders.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// The `CharacterLighting` singleton pointer.
pub const G_CHARACTER_LIGHTING: u32 = 0x00cc_ca84;
/// The `CharacterLightingTweakUtil` singleton pointer.
pub const G_CHARACTER_LIGHTING_TWEAK: u32 = 0x00cc_ca88;
/// The `MorphManager` singleton pointer.
pub const G_MORPH_MANAGER: u32 = 0x00cc_ca90;
/// The `FogEffectManager` singleton pointer.
pub const G_FOG_EFFECT_MANAGER: u32 = 0x00ce_5170;
/// The vertex-buffer upload `0x00a0e4b0` (cdecl `(buffer, data, size,
/// 1)`).
pub const UPLOAD: u32 = 0x00a0_e4b0;
/// `graphicDynGeoMemManagerGetCurrentBufferId` (cdecl `()`).
pub const DYN_GEO_BUFFER_ID: u32 = 0x0063_ff50;
/// The character shader loads `CharacterShaderManager_loadShader` makes.
pub const SHADER_LOADS: [u32; 3] = [0x006d_f3d0, 0x006d_f470, 0x006d_f510];

pub mod bridge {
    //! The originals over game memory.

    use super::{Conv, DYN_GEO_BUFFER_ID, Host, UPLOAD, x87};

    /// A singleton getter: the pointer at `global` (an `ffx.sym` address).
    pub fn singleton(h: &mut impl Host, global: u32) -> u32 {
        let at = h.rebase(global);
        h.read(at)
    }

    /// `PInputDeviceKeyboard_getButton(kb, i)`: the key byte
    /// `[kb + 0x1fc + i]` for `i >= 0x1c` (signed), else 0; the rest of
    /// `eax` is `i`'s (the original loads `i` into `eax` and sets `al`).
    pub fn keyboard_button(h: &mut impl Host, kb: u32, i: u32) -> u32 {
        let b = if (i as i32) >= 0x1c {
            h.read::<u8>(i.wrapping_add(kb).wrapping_add(0x1fc))
        } else {
            0
        };
        (i & !0xff) | u32::from(b)
    }

    /// The mouse's `getButton(mouse, i)` (`0x006289d0`): the button byte
    /// `[mouse + 0x10d + i]` for `i` in `0xb..=0xd`, else 0; the rest of
    /// `eax` is `i`'s.
    pub fn mouse_button(h: &mut impl Host, mouse: u32, i: u32) -> u32 {
        let b = if i.wrapping_sub(0xb) <= 2 {
            h.read::<u8>(i.wrapping_add(mouse).wrapping_add(0x10d))
        } else {
            0
        };
        (i & !0xff) | u32::from(b)
    }

    /// `CharacterLighting_isInitSuccessful(cl)`: the byte `+0x1cc`.
    pub fn is_init_successful(h: &mut impl Host, cl: u32) -> u8 {
        h.read(cl + 0x1cc)
    }

    /// `0x006a9a80(it)` -> `al`: whether the iterator `it` is usable: no
    /// current node (`+0x18`) is fine; else the node's list (`+0x2c`,
    /// linked by `+0x40`) must hold the iterator's key (`+0x1c`) and the
    /// node's `+0x40` be non-zero.
    pub fn iterator_valid(h: &mut impl Host, it: u32) -> u8 {
        let node: u32 = h.read(it + 0x18);
        if node == 0 {
            return 1;
        }
        let mut e: u32 = h.read(node + 0x2c);
        if e == 0 {
            return 0;
        }
        let key: u32 = h.read(it + 0x1c);
        while e != key {
            e = h.read(e + 0x40);
            if e == 0 {
                return 0;
            }
        }
        u8::from(h.read::<u32>(node + 0x40) != 0)
    }

    /// `0x006aa920(cl, i, value)`: when the byte `[cl]` is set, stores
    /// `value` (through the x87, so a signalling NaN is quieted) at
    /// `cl + 0xcc + 4 i`.
    pub fn set_coefficient(h: &mut impl Host, cl: u32, i: u32, value: u32) {
        if h.read::<u8>(cl) != 0 {
            let v = x87::store(x87::load(value));
            h.write(cl.wrapping_add(i.wrapping_mul(4)).wrapping_add(0xcc), v);
        }
    }

    /// `CharacterLighting_setMapIdFlagChanged(cl, a, b)` -> `b`: `+0x224 =
    /// a`, `+0x228 = b`.
    pub fn set_map_id_flag_changed(h: &mut impl Host, cl: u32, a: u32, b: u32) -> u32 {
        h.write(cl + 0x224, a);
        h.write(cl + 0x228, b);
        b
    }

    /// `0x006e2b10(mesh, value, i)` -> `value`: `[mesh + 0xa10 + 4 i] =
    /// value`.
    pub fn set_slot(h: &mut impl Host, mesh: u32, value: u32, i: u32) -> u32 {
        h.write(
            mesh.wrapping_add(i.wrapping_mul(4)).wrapping_add(0xa10),
            value,
        );
        value
    }

    /// `0x006e0800(this)` -> `this`: clears `+0`, `+4`, `+8` and `+0x10`.
    pub fn clear(h: &mut impl Host, this: u32) -> u32 {
        for at in [0, 4, 8, 0x10] {
            h.write(this + at, 0u32);
        }
        this
    }

    /// `ClothInstancingDynamicMeshInstance` vtable slot 1 `(inst, x)`
    /// (`0x006e23d0`): its mesh `+0xc`'s `vtable[1](x)` plus 8.
    pub fn mesh_slot1_plus8(h: &mut impl Host, inst: u32, x: u32) -> u32 {
        let mesh: u32 = h.read(inst + 0xc);
        h.call_virtual(mesh, 1, &[x]).wrapping_add(8)
    }

    /// `0x006f89b0(this, data, size)` -> 0: uploads through `UPLOAD(this
    /// +0x14's buffer, data, size, 1)`.
    pub fn upload(h: &mut impl Host, this: u32, data: u32, size: u32) -> u32 {
        let buffer: u32 = h.read(this + 0x14);
        let f = h.rebase(UPLOAD);
        h.call(f, Conv::Cdecl, &[buffer, data, size, 1]);
        0
    }

    /// `CharacterShaderManager_loadShader()`: the three shader loads
    /// `0x006df3d0`, `0x006df470` and `0x006df510` (cdecl, no arguments),
    /// the last one's `eax` returned (a tail jump).
    pub fn load_shaders(h: &mut impl Host) -> u32 {
        let mut eax = 0;
        for f in super::SHADER_LOADS {
            let f = h.rebase(f);
            eax = h.call(f, Conv::Cdecl, &[]);
        }
        eax
    }

    /// `ClassDynamicMeshInstance_populatePreprocessData(inst, out, x)` ->
    /// 0: `out +0x10` = the mesh's (`+0xc`) `+0x24`, `out +0x14` = the
    /// current dynamic-geometry buffer id.
    pub fn populate_preprocess_data(h: &mut impl Host, inst: u32, out: u32) -> u32 {
        let mesh: u32 = h.read(inst + 0xc);
        let v: u32 = h.read(mesh + 0x24);
        h.write(out + 0x10, v);
        let f = h.rebase(DYN_GEO_BUFFER_ID);
        let id = h.call(f, Conv::Cdecl, &[]);
        h.write(out + 0x14, id);
        0
    }
}
