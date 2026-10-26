//! Continuation round 8 of list `h4-glue-other`, batch 6: the guild map's
//! dynamic geometry update.

use ffx_bridge::x87::{load, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;

/// `g_PhyreScene` (`0x00ccb9d8`).
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// `g_CurrentMapNo` (`0x012fbc40`).
pub const CURRENT_MAP: u32 = 0x012f_bc40;
/// `ClassDynamicMesh_fillDrawTable_006732a0(mesh; data)` (thiscall, ret
/// 4).
pub const FILL_DRAW_TABLE: u32 = 0x0067_32a0;
/// `PhyreScene_setMeshInstanceWorldMatrix(scene; instance, data)`
/// (`0x0066d810`, thiscall, ret 8).
pub const SET_WORLD_MATRIX: u32 = 0x0066_d810;
/// `Phyre_PRendering_PMeshInstanceBounds_setMinAndMax(bounds; min, max)`
/// (`0x00500bd0`, thiscall, ret 8).
pub const SET_MIN_AND_MAX: u32 = 0x0050_0bd0;
/// `PhyreScene_is3DScene(scene)` (`0x006601d0`, thiscall).
pub const IS_3D_SCENE: u32 = 0x0066_01d0;
/// `ShaderParams_FindByName(shader; name)` (`0x0056cbf0`, thiscall, ret
/// 4).
pub const FIND_BY_NAME: u32 = 0x0056_cbf0;
/// `ShaderParamBuffer_SetValue2(buffer; parameter, value)`
/// (`0x006386c0`, thiscall, ret 8).
pub const SET_VALUE: u32 = 0x0063_86c0;
/// `PhyreScene_addUniqueToArray386c_00656a00(scene; instance)`
/// (thiscall, ret 4).
pub const ADD_UNIQUE: u32 = 0x0065_6a00;
/// `"OffsetX"`, `"OffsetY"` (`0x00b47fb4`, `0x00b47fbc`).
pub const OFFSET_X: u32 = 0x00b4_7fb4;
/// See [`OFFSET_X`].
pub const OFFSET_Y: u32 = 0x00b4_7fbc;
/// The offsets' constants (f32 at `0x00c34180`..`0x00c34198`): x for 3D
/// scenes, x on maps 0x66 and 0xdf, y, x with the scene flag, y with it,
/// x and y for 2D scenes.
pub const X_3D: u32 = 0x00c3_4180;
/// See [`X_3D`].
pub const X_3D_GUILD: u32 = 0x00c3_4184;
/// See [`X_3D`].
pub const Y_3D: u32 = 0x00c3_4188;
/// See [`X_3D`].
pub const X_FLAGGED: u32 = 0x00c3_418c;
/// See [`X_3D`].
pub const Y_FLAGGED: u32 = 0x00c3_4190;
/// See [`X_3D`].
pub const X_2D: u32 = 0x00c3_4194;
/// See [`X_3D`].
pub const Y_2D: u32 = 0x00c3_4198;
/// The offset values handed to the shader (f32 globals `0x00ccca5c` x,
/// `0x00ccca60` y, `0x00ccca64` 2D x, `0x00ccca68` 2D y).
pub const OFFSET_VALUES: u32 = 0x00cc_ca5c;
/// Bytes of [`update_render_data`]'s frame, from the original's `ebp -
/// 0x2c`: the bounds' minimum (`+0`) and maximum (`+0x10`), 3 floats
/// each.
pub const GUILD_FRAME: u32 = 0x1c;

/// An `f32` through `fld`/`fchs`/`fstp`.
fn negated(bits: u32) -> u32 {
    store(-load(bits))
}

/// `GuildMapDynamicGeometry_updateRenderData(this; unused)` (`0x006a5070`,
/// thiscall, ret 4; a vtable entry; `frame` see [`GUILD_FRAME`]): with
/// render data (`+0x24`): the mesh's draw table filled (`+8`), the mesh
/// instance (`+0xc`) placed and its bounds (`+0x1c`) set from the data's
/// `+0x98`..`+0xac`; for each instance of the array `+0x14` the shader
/// (`+0x28`) parameters `OffsetX`/`OffsetY` set on its `+4` buffer to the
/// image's constants (negated by the scene's mode `+0x74`: 3..4 for x, 2
/// or 4 for y; x picked by map 0x66/0xdf; overridden while the scene's
/// first byte is set; separate 2D constants when the scene is not 3D),
/// through the globals at [`OFFSET_VALUES`]; the instance added to the
/// scene's unique array. Returns 0.
pub fn update_render_data(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    let data: u32 = h.read(at(0x24));
    if data == 0 {
        return 0;
    }
    let mesh: u32 = h.read(at(8));
    if mesh != 0 {
        h.call_sym(FILL_DRAW_TABLE, Conv::Thiscall(mesh), &[data]);
    }
    let instance: u32 = h.read(at(0xc));
    if instance != 0 {
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        let data: u32 = h.read(at(0x24));
        h.call_sym(SET_WORLD_MATRIX, Conv::Thiscall(scene), &[instance, data]);
        let instance: u32 = h.read(at(0xc));
        let bounds: u32 = h.read(instance.wrapping_add(0x1c));
        if bounds != 0 {
            let data: u32 = h.read(at(0x24));
            for k in 0..3u32 {
                let low = pass(h.read(data.wrapping_add(0x98 + 4 * k)));
                let high = pass(h.read(data.wrapping_add(0xa4 + 4 * k)));
                h.write(frame + 4 * k, low);
                h.write(frame + 0x10 + 4 * k, high);
            }
            h.call_sym(
                SET_MIN_AND_MAX,
                Conv::Thiscall(bounds),
                &[frame, frame + 0x10],
            );
        }
    }
    if h.read::<u32>(at(0x14)) != 0 {
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        let flat = h.call_sym(IS_3D_SCENE, Conv::Thiscall(scene), &[]) == 0;
        let array: u32 = h.read(at(0x14));
        let n = h.read::<u32>(array) & 0x7fff_ffff;
        for i in 0..n {
            let array: u32 = h.read(at(0x14));
            let count = h.read::<u32>(array) & 0x7fff_ffff;
            let items = if count > 1 {
                h.read(array.wrapping_add(4))
            } else if count != 0 {
                array.wrapping_add(4)
            } else {
                0
            };
            let item: u32 = h.read(items.wrapping_add(4 * i));
            for (name, x) in [(OFFSET_X, true), (OFFSET_Y, false)] {
                let shader: u32 = h.read(at(0x28));
                let name = h.rebase(name);
                let p = h.call_sym(FIND_BY_NAME, Conv::Thiscall(shader), &[name]);
                if p != 0 {
                    let value = offset_value(h, flat, x);
                    let buffer: u32 = h.read(item.wrapping_add(4));
                    h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, value]);
                }
            }
        }
    }
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let instance: u32 = h.read(at(0xc));
    h.call_sym(ADD_UNIQUE, Conv::Thiscall(scene), &[instance]);
    0
}

/// The offset `x` (else `y`) written to its global; returns the global's
/// address.
fn offset_value(h: &mut impl Host, flat: bool, x: bool) -> u32 {
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let mode: u32 = h.read(scene.wrapping_add(0x74));
    let (cell, bits) = if x {
        let flip = mode.wrapping_sub(3) <= 1;
        if flat {
            (
                8,
                if flip {
                    negated(image_word(h, X_2D))
                } else {
                    pass(image_word(h, X_2D))
                },
            )
        } else {
            let map: u32 = h.read(h.rebase(CURRENT_MAP));
            let sym = if map == 0x66 || map == 0xdf {
                X_3D_GUILD
            } else {
                X_3D
            };
            (
                0,
                if flip {
                    negated(image_word(h, sym))
                } else {
                    pass(image_word(h, sym))
                },
            )
        }
    } else {
        let flip = mode == 2 || mode == 4;
        let sym = if flat { Y_2D } else { Y_3D };
        (
            if flat { 0xc } else { 4 },
            if flip {
                negated(image_word(h, sym))
            } else {
                pass(image_word(h, sym))
            },
        )
    };
    let at = h.rebase(OFFSET_VALUES) + cell;
    h.write(at, bits);
    if !flat && h.read::<u8>(scene) != 0 {
        let sym = if x { X_FLAGGED } else { Y_FLAGGED };
        let v = pass(image_word(h, sym));
        h.write(at, v);
    }
    at
}

/// The word at image address `sym`.
fn image_word(h: &impl Host, sym: u32) -> u32 {
    h.read(h.rebase(sym))
}
