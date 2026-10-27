//! The render-side glue, fiftieth batch (M4 port round 14, list
//! `h3-glue-render`): a loaded character's instance. The cookie is left
//! out, as in [`crate::glue10`].

/// The MSVC string's `substr(this; out, pos, n)` (`ret 0xc`, returns
/// `out`).
pub const STRING_SUBSTR: u32 = 0x0040_2e20;
/// `operator new` and `operator delete` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `FUN_006a0d00(character; instance)` (`ret 4`).
pub const CHARACTER_ADD_INSTANCE: u32 = 0x006a_0d00;
/// The scene cursor's `begin` and `next` (`this`, no arguments) over
/// objects of the mesh-instance class.
pub const CURSOR_BEGIN: u32 = 0x0063_7730;
pub const CURSOR_NEXT: u32 = 0x0063_77a0;
pub const INSTANCE_CLASS: u32 = 0x00ca_7228;
/// `PhyreScene_instantiateMesh_0066f7d0(scene; model, key, shared)`
/// (`ret 0xc`).
pub const INSTANTIATE_MESH: u32 = 0x0066_f7d0;
/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl) and
/// `PArray_PushInto_00637ec0(array; buffer, &value)` (`ret 8`).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PARRAY_PUSH_INTO: u32 = 0x0063_7ec0;
/// `Phyre_PhyreFree` (cdecl), which the push calls on the old data.
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `FUN_006a1320(instance list; mesh instance)` (`ret 4`).
pub const INSTANCE_ADD_MESH: u32 = 0x006a_1320;
/// `Phyre_CharacterLighting_GetInstance()` (cdecl) and
/// `instantiateUniqueMaterialParameters(lighting; mesh instance)` (`ret
/// 4`).
pub const LIGHTING_INSTANCE: u32 = 0x006a_6e30;
pub const LIGHTING_INSTANTIATE: u32 = 0x006a_96b0;
/// `ClassLoadedCharacterTable_GetInstance()` (cdecl) and `FUN_006a2220
/// (table)` (its name tree).
pub const TABLE_INSTANCE: u32 = 0x0069_f6c0;
pub const TABLE_TREE: u32 = 0x006a_2220;
/// `TextureAnimationManager_GetInstance()` (cdecl) and
/// `registerCharacterInstance(manager; name, instance)` (`ret 8`).
pub const TEXTURE_ANIMATION_INSTANCE: u32 = 0x006b_02e0;
pub const REGISTER_CHARACTER_INSTANCE: u32 = 0x006b_29c0;
/// `DepthBiasHooks_onCharacterInstantiated(name, array)` (cdecl; the
/// array `{count, data}` by value).
pub const DEPTH_BIAS_ON_INSTANTIATED: u32 = 0x006e_4100;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::SCENE;
    use crate::h3_glue14::bridge::{cursor_over, walk};
    use crate::h3_glue49::bridge::{clear_string, move_string, rfind, successor, text};
    use ffx_graphic::host::{Conv, Host};

    /// `FUN_0063c540(character, key)` (cdecl; frame from `ebp - 0x54`)
    /// -> the new instance list, or 0 without a character. The list (0x20
    /// bytes from `operator new`: a `PArray` of mesh instances, `+0x10`
    /// the character, `+0x18` the key) goes to the character; each mesh
    /// of the character's model (`+4`, its objects of the mesh-instance
    /// class, cursor at `ebp - 0x3c`) is instantiated in the scene
    /// (shared when `+0x20` is set), pushed (`ebp - 0x44`), added and its
    /// material parameters made unique. The loaded-character table's
    /// names whose value (`+0x28`) is this character give the instance
    /// its name (`ebp - 0x1c`): the part between the last `/` and the
    /// last `.`, the last match winning. The name and list go to the
    /// texture animations and, with a copy of the mesh array, to the
    /// depth-bias hooks.
    pub fn instantiate_character(h: &mut impl Host, ebp: u32, character: u32, key: u32) -> u32 {
        h.write(ebp - 0x54, character);
        h.write(ebp - 0x4c, key);
        if character == 0 {
            return 0;
        }
        let list = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x20]);
        h.write(ebp - 0x48, list);
        if list != 0 {
            for k in 0..4 {
                h.write(list + 4 * k, 0u32);
            }
        }
        h.write(list.wrapping_add(0x18), key);
        h.write(list.wrapping_add(0x10), character);
        h.call_sym(CHARACTER_ADD_INSTANCE, Conv::Thiscall(character), &[list]);
        let model: u32 = h.read(character.wrapping_add(4));
        let cursor = ebp - 0x3c;
        cursor_over(h, cursor, model, INSTANCE_CLASS);
        h.call_sym(CURSOR_BEGIN, Conv::Thiscall(cursor), &[]);
        walk(h, cursor, CURSOR_NEXT, |h, base| {
            let scene = SCENE.get(h);
            let shared = u32::from(h.read::<u32>(character.wrapping_add(0x20)) != 0);
            let offset: u32 = h.read(cursor + 0x10);
            let mesh = h.call_sym(
                INSTANTIATE_MESH,
                Conv::Thiscall(scene),
                &[offset.wrapping_add(base), key, shared],
            );
            h.write(ebp - 0x44, mesh);
            let count = h.read::<u32>(list) & 0x7fff_ffff;
            let size = count.wrapping_add(1).wrapping_mul(4);
            let buffer = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 4]);
            h.call_sym(
                PARRAY_PUSH_INTO,
                Conv::Thiscall(list),
                &[buffer, ebp - 0x44],
            );
            h.call_sym(INSTANCE_ADD_MESH, Conv::Thiscall(list), &[mesh]);
            let lighting = h.call_sym(LIGHTING_INSTANCE, Conv::Cdecl, &[]);
            h.call_sym(LIGHTING_INSTANTIATE, Conv::Thiscall(lighting), &[mesh]);
        });
        let name = ebp - 0x1c;
        clear_string(h, name);
        let table = h.call_sym(TABLE_INSTANCE, Conv::Cdecl, &[]);
        let tree = h.call_sym(TABLE_TREE, Conv::Thiscall(table), &[]);
        h.write(ebp - 0x44, tree);
        let head: u32 = h.read(tree);
        let mut node: u32 = h.read(head);
        while node != h.read::<u32>(tree) {
            if h.read::<u32>(node.wrapping_add(0x28)) == character {
                name_from(h, ebp, node.wrapping_add(0x10));
            }
            node = successor(h, node);
        }
        let name_text = text(h, name);
        let manager = h.call_sym(TEXTURE_ANIMATION_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(
            REGISTER_CHARACTER_INSTANCE,
            Conv::Thiscall(manager),
            &[name_text, list],
        );
        // the mesh array by value (the original keeps the address of its
        // argument words, `ebp - 0x68`, and the count in its frame): a
        // copy of its words, or none
        h.write(ebp - 0x50, ebp - 0x68);
        let count = h.read::<u32>(list) & 0x7fff_ffff;
        h.write(ebp - 0x44, count);
        let mut from: u32 = h.read(list.wrapping_add(4));
        let mut data = 0;
        if count != 0 {
            data = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[count.wrapping_mul(4), 4]);
            if data != 0 {
                for k in 0..count {
                    let word: u32 = h.read(from);
                    h.write(data.wrapping_add(4 * k), word);
                    from = from.wrapping_add(4);
                }
            }
        }
        let count = if data == 0 { 0 } else { count };
        let name_text = text(h, name);
        h.call_sym(
            DEPTH_BIAS_ON_INSTANTIATED,
            Conv::Cdecl,
            &[name_text, count, data],
        );
        if h.read::<u32>(name + 0x14) >= 0x10 {
            let data: u32 = h.read(name);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
        }
        list
    }

    /// The name `s` cut to the part between its last `/` and its last
    /// `.` (`substr` into `ebp - 0x34`) and moved to `ebp - 0x1c`.
    fn name_from(h: &mut impl Host, ebp: u32, s: u32) {
        h.write(ebp - 0x40, b'.');
        let dot = rfind(h, s, b'.');
        h.write(ebp - 0x4c, dot);
        h.write(ebp - 0x40, b'/');
        let slash = rfind(h, s, b'/');
        h.write(ebp - 0x50, slash);
        let start = rfind(h, s, b'/');
        let temp = ebp - 0x34;
        let name = ebp - 0x1c;
        let cut = h.call_sym(
            STRING_SUBSTR,
            Conv::Thiscall(s),
            &[
                temp,
                start.wrapping_add(1),
                dot.wrapping_sub(slash).wrapping_sub(1),
            ],
        );
        if cut != name {
            move_string(h, name, cut);
        }
        if h.read::<u32>(temp + 0x14) >= 0x10 {
            let data: u32 = h.read(temp);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
        }
    }
}
