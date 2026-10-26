//! The render-side glue, twentieth batch (M4 port round 6, list
//! `h3-glue-render`): the cross menu's scroll bar (x87), a character
//! instance's mesh alpha and the map's YIQ parameters. The material
//! parameter records are written in the original's frame and copied from
//! there, so a parameter larger than its record copies the same frame
//! words the original's does.

pub use crate::glue::SCENE;

/// `graphicUiRemapX2(x)` / `graphicUiRemapY2(y)` (cdecl, `ST0`) and
/// `TODrawCrossBoxXYWHC2(x, y, w, h, c0, c1)` (cdecl).
pub const UI_REMAP_X2: u32 = 0x0064_47e0;
pub const UI_REMAP_Y2: u32 = 0x0064_4820;
pub const DRAW_CROSS_BOX: u32 = 0x008f_4b20;
/// The scroll bar's track colour and its thumb's two.
pub const TRACK_COLOUR: u32 = 0x801a_1a1a;
pub const THUMB_COLOURS: (u32, u32) = (0x80bb_a09f, 0x8097_6c6f);

/// `ShaderParams_FindByName(shader, name)`, `PhyreScene_setMeshVisible(this,
/// mesh, visible)`, `PhyreScene_ForceTransparent(material, on)` (cdecl)
/// and `"MaterialColour"`.
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
pub const SET_MESH_VISIBLE: u32 = 0x0066_d910;
pub const FORCE_TRANSPARENT: u32 = 0x0065_1400;
pub const MATERIAL_COLOUR: u32 = 0x00b4_1980;

/// `PhyreScene_getYIQParam(this, 6 pointers)`, `PArray_Data_004d45a0(this)`,
/// `Phyre_PRedBlackTree_successor(this, node)`, `"YIQscale"` and
/// `"YIQshift"`.
pub const GET_YIQ_PARAM: u32 = 0x0065_e9e0;
pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
pub const YIQ_SCALE: u32 = 0x00b4_3c98;
pub const YIQ_SHIFT: u32 = 0x00b4_3ca4;

/// The count of a `PArray`-style header.
const COUNT_MASK: u32 = 0x7fff_ffff;
const ONE: u32 = 0x3f80_0000;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn remap(h: &mut impl Host, f: u32) -> f64 {
        h.call_sym_st0_f64(f, Conv::Cdecl, &[ONE])
    }

    /// `DrawCrossMenuScrollParts(x, y, w, h, first, shown, total)` (cdecl;
    /// `slots` the address of its arguments, the floats as bits): with
    /// more lines than shown, the track box, and inside its remapped
    /// one-unit border the thumb from line `first` for `shown` lines (to
    /// the bottom when they reach the end). The original stores `shown` as
    /// a float over `total`'s slot and the thumb's width over `y`'s.
    #[allow(clippy::too_many_arguments)] // the original's seven and the slots
    #[allow(clippy::cast_precision_loss)] // fild + fstp dword: the same rounding
    pub fn draw_cross_menu_scroll_parts(
        h: &mut impl Host,
        slots: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
        first: i32,
        shown: i32,
        total: i32,
    ) {
        let left = x87::store(x87::add(remap(h, UI_REMAP_X2), x87::load(x)));
        let top = x87::store(x87::add(remap(h, UI_REMAP_Y2), x87::load(y)));
        let right_edge = x87::add(x87::load(x), x87::load(w));
        let right = x87::store(x87::sub(right_edge, remap(h, UI_REMAP_X2)));
        let bottom_edge = x87::add(x87::load(y), x87::load(ht));
        let bottom = x87::store(x87::sub(bottom_edge, remap(h, UI_REMAP_Y2)));
        if total <= shown {
            return;
        }
        let total_f = (total as f32).to_bits();
        h.write(slots + 0x18, total_f);
        let step = x87::store(x87::div(
            x87::sub(x87::load(bottom), x87::load(top)),
            x87::load(total_f),
        ));
        let first_f = (first as f32).to_bits();
        h.write(slots + 0x18, first_f);
        let y0 = x87::store(x87::add(
            x87::load(top),
            x87::mul(x87::load(first_f), x87::load(step)),
        ));
        let shown_f = (shown as f32).to_bits();
        h.write(slots + 0x18, shown_f);
        let mut y1 = x87::store(x87::add(
            x87::mul(x87::load(step), x87::load(shown_f)),
            x87::load(y0),
        ));
        if first.wrapping_add(shown) >= total {
            y1 = x87::store(x87::sub(bottom_edge, remap(h, UI_REMAP_Y2)));
        }
        let copy = |bits| x87::store(x87::load(bits));
        h.call_sym(
            DRAW_CROSS_BOX,
            Conv::Cdecl,
            &[
                copy(x),
                copy(y),
                copy(w),
                copy(ht),
                TRACK_COLOUR,
                TRACK_COLOUR,
            ],
        );
        let thumb_h = x87::store(x87::sub(x87::load(y1), x87::load(y0)));
        h.write(slots + 4, thumb_h);
        let thumb_w = x87::store(x87::sub(x87::load(right), x87::load(left)));
        h.write(slots + 4, thumb_w);
        h.call_sym(
            DRAW_CROSS_BOX,
            Conv::Cdecl,
            &[left, y0, thumb_w, thumb_h, THUMB_COLOURS.0, THUMB_COLOURS.1],
        );
    }

    /// `PArray`-style element pointer: the data `+4` with a count over 1,
    /// the inline element `+4` with 1, none with 0.
    fn array_data(h: &impl Host, array: u32) -> u32 {
        match h.read::<u32>(array) & COUNT_MASK {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        }
    }

    /// Copies `record` (in the frame) into the material's parameter
    /// buffer (`+4`, its size the first word) at the parameter's offset
    /// (`+8`, 16 bits) for its size (`+0xa`, 13 bits) when it fits past
    /// the size word; `at_offset`/`at_size`: frame words that get them.
    fn copy_record(
        h: &mut impl Host,
        parameter: u32,
        buffer: u32,
        record: u32,
        at_offset: Option<u32>,
        at_size: Option<u32>,
    ) {
        let offset = u32::from(h.read::<u16>(parameter + 8));
        let size = u32::from(h.read::<u16>(parameter + 0xa) & 0x1fff);
        if let Some(at) = at_size {
            h.write(at, size);
        }
        if let Some(at) = at_offset {
            h.write(at, offset);
        }
        if offset + size > h.read::<u32>(buffer) || offset < 4 {
            return;
        }
        let mut bytes = vec![0; size as usize];
        h.read_bytes(record, &mut bytes);
        h.write_bytes(buffer.wrapping_add(offset), &bytes);
    }

    /// Every mesh of the instance shown or hidden.
    fn set_all_visible(h: &mut impl Host, chr: u32, visible: u32) {
        let mut n = 0;
        while n < h.read::<u32>(chr) & COUNT_MASK {
            let meshes: u32 = h.read(chr + 4);
            let mesh: u32 = h.read(meshes.wrapping_add(4 * n));
            let scene = SCENE.get(h);
            h.call_sym(SET_MESH_VISIBLE, Conv::Thiscall(scene), &[mesh, visible]);
            n += 1;
        }
    }

    /// `graphicCharacterInstanceSetMeshAlpha(chr, _, alpha)` (cdecl; frame:
    /// the colour record `{1, 1, 1, alpha}` at `ebp - 0x24`, then the
    /// element list `- 0x14`, its count `- 0x10`, the parameter offset
    /// `- 0xc`, the element offset `- 8` and index `- 4`): for each mesh
    /// (`+0/+4`) each draw element (0x6c bytes, its material index first)
    /// gets its material's (the mesh's `+0x10` array)
    /// `"MaterialColour"` set and is forced transparent below 1.0. A
    /// positive alpha on an instance flagged at `+0x14` shows every mesh
    /// first (each element); a zero alpha clears the flag, hides every
    /// mesh and stops.
    pub fn character_set_mesh_alpha(h: &mut impl Host, ebp: u32, chr: u32, alpha_word: u32) {
        let alpha = x87::load(alpha_word);
        if chr == 0 || h.read::<u32>(chr) & COUNT_MASK == 0 {
            return;
        }
        let alpha_bits = x87::store(alpha);
        let mut i = 0u32;
        loop {
            h.write(ebp - 4, 0u32);
            let meshes: u32 = h.read(chr + 4);
            let mesh: u32 = h.read(meshes.wrapping_add(4 * i));
            let list: u32 = h.read(mesh);
            h.write(ebp - 0x14, list);
            let count = h.read::<u32>(list) & COUNT_MASK;
            h.write(ebp - 0x10, count);
            if count != 0 {
                h.write(ebp - 8, 0u32);
                let mut k = 0u32;
                let mut offset = 0u32;
                loop {
                    let elements: u32 = h.read(list + 4);
                    let element = elements.wrapping_add(offset);
                    if alpha > 0.0 && h.read::<u8>(chr + 0x14) != 0 {
                        h.write(chr + 0x14, 1u8);
                        set_all_visible(h, chr, 1);
                    }
                    if alpha == 0.0 {
                        h.write(chr + 0x14, 0u8);
                        set_all_visible(h, chr, 0);
                        return;
                    }
                    let index: u32 = h.read(element);
                    let meshes: u32 = h.read(chr + 4);
                    let mesh: u32 = h.read(meshes.wrapping_add(4 * i));
                    let materials = array_data(h, h.read(mesh + 0x10));
                    let material: u32 = h.read(materials.wrapping_add(4 * index));
                    let shader: u32 = h.read(material);
                    let buffer: u32 = h.read(material + 4);
                    let name = h.rebase(MATERIAL_COLOUR);
                    let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
                    if parameter != 0 {
                        let record = ebp - 0x24;
                        for (k, v) in [ONE, ONE, ONE, alpha_bits].into_iter().enumerate() {
                            h.write(record + 4 * k as u32, v);
                        }
                        copy_record(h, parameter, buffer, record, Some(ebp - 0xc), None);
                        let below = u32::from(alpha < 1.0);
                        h.call_sym(FORCE_TRANSPARENT, Conv::Cdecl, &[material, below]);
                    }
                    k += 1;
                    offset = offset.wrapping_add(0x6c);
                    h.write(ebp - 4, k);
                    h.write(ebp - 8, offset);
                    if (k as i32) >= (count as i32) {
                        break;
                    }
                }
            }
            i += 1;
            if i >= h.read::<u32>(chr) & COUNT_MASK {
                return;
            }
        }
    }

    /// `PhyreScene_updateMapYIQ(this)` (frame: the shift record `{0, 0,
    /// 0, 0}` at `ebp - 0x3c`, the scale record `{1, 1, 1, 1}` at `-
    /// 0x2c`, the tree head `- 0x1c`, the node's list `- 0x18`, the
    /// scale parameter's offset and size `- 0x14`/`- 0x10`, the list index
    /// `- 0xc`, the material array `- 8` and the next node `- 4`): with a
    /// map cluster (`+0x10d4`) the scene's YIQ values fill the records
    /// (three scale and three shift words), then every material of every
    /// mesh in the map tree (`+0x1198`; its first node the end of the
    /// chain from `+0x1198 + 0xc`) gets `"YIQscale"` and `"YIQshift"`.
    pub fn scene_update_map_yiq(h: &mut impl Host, ebp: u32, this: u32) {
        if h.read::<u32>(this + 0x10d4) == 0 {
            return;
        }
        let scale = ebp - 0x2c;
        let shift = ebp - 0x3c;
        for k in 0..4 {
            h.write(scale + 4 * k, ONE);
            h.write(shift + 4 * k, 0u32);
        }
        h.call_sym(
            GET_YIQ_PARAM,
            Conv::Thiscall(this),
            &[scale, scale + 4, scale + 8, shift, shift + 4, shift + 8],
        );
        let head = this + 0x1198;
        h.write(ebp - 0x1c, head);
        let mut node = head + 0xc;
        let mut at: u32 = h.read(head + 0xc);
        if at == head || at == 0 {
            node = at;
        } else {
            while at != head {
                node = at;
                at = h.read(at);
            }
        }
        h.write(ebp - 4, node);
        if node == head {
            return;
        }
        let next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(head), &[node]);
        h.write(ebp - 4, next);
        loop {
            let list: u32 = h.read(node.wrapping_add(0x10));
            h.write(ebp - 0x18, list);
            h.write(ebp - 0xc, 0u32);
            let mut k = 0u32;
            while k < h.read::<u32>(list) {
                let items: u32 = h.read(list + 8);
                let item: u32 = h.read(items.wrapping_add(4 * k));
                let array: u32 = h.read(item + 0x10);
                h.write(ebp - 8, array);
                let mut j = 0u32;
                while j < h.read::<u32>(array) & COUNT_MASK {
                    let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(array), &[]);
                    let material: u32 = h.read(data.wrapping_add(4 * j));
                    let shader: u32 = h.read(material);
                    let name = h.rebase(YIQ_SCALE);
                    let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
                    if parameter != 0 {
                        let buffer: u32 = h.read(material + 4);
                        copy_record(
                            h,
                            parameter,
                            buffer,
                            scale,
                            Some(ebp - 0x14),
                            Some(ebp - 0x10),
                        );
                    }
                    let name = h.rebase(YIQ_SHIFT);
                    let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
                    if parameter != 0 {
                        let buffer: u32 = h.read(material + 4);
                        copy_record(h, parameter, buffer, shift, None, None);
                    }
                    j += 1;
                }
                k += 1;
                h.write(ebp - 0xc, k);
            }
            let next: u32 = h.read(ebp - 4);
            node = next;
            let after = if next == head {
                next
            } else {
                h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(head), &[next])
            };
            h.write(ebp - 4, after);
            if node == head {
                return;
            }
        }
    }
}
