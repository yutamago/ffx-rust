//! Continuation round 8 of list `h4-glue-other`, batch 5: the text
//! utility's asset hook-up and the character shader's render data.

use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;

/// `PApplication::GetApplication()` (`0x00623b90`, cdecl).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `PApplication_FixupClusters(clusters, n)` (`0x00623580`, cdecl).
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
/// `PhyreSceneCursor_Begin_006f6300(cursor)` (thiscall).
pub const CURSOR_BEGIN: u32 = 0x006f_6300;
/// The bitmap font class the cursor looks for (`0x01a84d20`).
pub const FONT_CLASS: u32 = 0x01a8_4d20;
/// `Phyre_PhyreMalloc(n)` (`0x0042fb40`, cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb40;
/// `Phyre_PhyreFree(p)` (`0x0042fb20`, cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `SamplerHolder_ctor_006f5ae0(this; owner)` (thiscall, ret 4) -> this.
pub const MATERIAL_CTOR: u32 = 0x006f_5ae0;
/// `0x006f5c40(this)` (thiscall): the material's destructor.
pub const MATERIAL_DTOR: u32 = 0x006f_5c40;
/// `Phyre_PText_PBitmapTextMaterial_initialize(material; font, 0)`
/// (`0x00a29ee0`, thiscall, ret 8) -> 0 or an error.
pub const MATERIAL_INITIALIZE: u32 = 0x00a2_9ee0;
/// `Phyre_PText_PBitmapTextMaterial_setTextColor_00a2a650(material;
/// rgb)` (thiscall, ret 4).
pub const SET_TEXT_COLOR: u32 = 0x00a2_a650;
/// `Phyre_PText_PBitmapFontText_PBitmapFontText_00a29360(text; material,
/// 0)` (thiscall, ret 8) -> text.
pub const FONT_TEXT_CTOR: u32 = 0x00a2_9360;
/// `Phyre_PString_Assign(string; text)` (`0x00445cf0`, thiscall, ret 4).
pub const STRING_ASSIGN: u32 = 0x0044_5cf0;
/// `"Debug Info:"` (`0x00b4bef0`).
pub const DEBUG_INFO: u32 = 0x00b4_bef0;
/// `Phyre_PStringBuilder_operator+=(builder; text)` (`0x00444bf0`,
/// thiscall, ret 4).
pub const BUILDER_APPEND: u32 = 0x0044_4bf0;
/// `Phyre_PText_PBitmapFontText_setText_00a2ad80(text; n)` (thiscall,
/// ret 4).
pub const SET_TEXT: u32 = 0x00a2_ad80;
/// The text utilities' registry (`0x00ce8260`).
pub const TEXT_UTILS: u32 = 0x00ce_8260;
/// `0x006f5de0(registry; util)` (thiscall, ret 4): registers a text
/// utility.
pub const REGISTER: u32 = 0x006f_5de0;
/// Bytes of [`on_all_assets_loaded`]'s frame, from the original's `ebp -
/// 0x30`: the scene cursor (8 words), whose last three the text colour
/// (1, 0, 1) overwrites later (`ebp - 0x20`).
pub const ASSETS_FRAME: u32 = 0x20;

/// `TextUtil_doOnAllTextUtilAssetsLoaded(this)` (`0x006f60a0`, thiscall;
/// `frame` see [`ASSETS_FRAME`]; the C++ exception frame around the two
/// constructors is not kept): both clusters (`+4`, then `+0`) fixed up;
/// 0xa without a font cluster; a scene cursor over it for the bitmap font
/// class, 0x13 without one; `+8` = the font, a 0x38-byte material
/// (`PhyreMalloc`, its constructor with the font) at `+0xc` (0xe when the
/// allocation fails); its `initialize(+0, 0)` result returned when not 0
/// (the material deleted and `+0xc` cleared); else the text colour (1, 0,
/// 1), a 0x198-byte bitmap text over the material at `+0x10` (0xe and
/// the material deleted when that fails), `+0x7c` = `"Debug Info:"`,
/// `+0x1c` = 0, that text appended to the builder at `+0x14`, the text
/// set with `+0x1c` less one (0 stays 0), the utility registered;
/// returns 0.
pub fn on_all_assets_loaded(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    for cluster in [at(4), this] {
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[cluster, 1]);
    }
    let cluster: u32 = h.read(at(4));
    if cluster == 0 {
        return 0xa;
    }
    let end = cluster.wrapping_add(0x1c);
    let first: u32 = h.read(end);
    for k in 0..5u32 {
        h.write(frame + 4 * k, 0u32);
    }
    h.write(frame + 0x14, end);
    h.write(frame + 0x18, if first == end { 0 } else { first });
    let class = h.rebase(FONT_CLASS);
    h.write(frame + 0x1c, class);
    h.call_sym(CURSOR_BEGIN, Conv::Thiscall(frame), &[]);
    if h.read::<u32>(frame + 0xc) == 0 {
        return 0x13;
    }
    let font = h.read::<u32>(frame + 0x10).wrapping_add(h.read(frame));
    h.write(at(8), font);
    let block = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x38]);
    let material = if block == 0 {
        0
    } else {
        let owner: u32 = h.read(at(8));
        h.call_sym(MATERIAL_CTOR, Conv::Thiscall(block), &[owner])
    };
    h.write(at(0xc), material);
    if material == 0 {
        return 0xe;
    }
    let font_file: u32 = h.read(this);
    let r = h.call_sym(
        MATERIAL_INITIALIZE,
        Conv::Thiscall(material),
        &[font_file, 0],
    );
    if r != 0 {
        delete_material(h, this);
        return r;
    }
    let colour = frame + 0x10;
    h.write(colour, 1.0f32.to_bits());
    h.write(colour + 4, 0u32);
    h.write(colour + 8, 1.0f32.to_bits());
    let material: u32 = h.read(at(0xc));
    h.call_sym(SET_TEXT_COLOR, Conv::Thiscall(material), &[colour]);
    let block = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x198]);
    let text = if block == 0 {
        0
    } else {
        let material: u32 = h.read(at(0xc));
        h.call_sym(FONT_TEXT_CTOR, Conv::Thiscall(block), &[material, 0])
    };
    h.write(at(0x10), text);
    if text == 0 {
        delete_material(h, this);
        return 0xe;
    }
    let debug = h.rebase(DEBUG_INFO);
    h.call_sym(STRING_ASSIGN, Conv::Thiscall(at(0x7c)), &[debug]);
    h.write(at(0x1c), 0u32);
    let s: u32 = h.read(at(0x7c));
    h.call_sym(
        BUILDER_APPEND,
        Conv::Thiscall(at(0x14)),
        &[s.wrapping_sub(s & 1)],
    );
    let n: u32 = h.read(at(0x1c));
    let text: u32 = h.read(at(0x10));
    h.call_sym(SET_TEXT, Conv::Thiscall(text), &[n.saturating_sub(1)]);
    let registry = h.rebase(TEXT_UTILS);
    h.call_sym(REGISTER, Conv::Thiscall(registry), &[this]);
    0
}

/// The material at `+0xc` destroyed and freed (when set), `+0xc` cleared.
fn delete_material(h: &mut impl Host, this: u32) {
    let material: u32 = h.read(this.wrapping_add(0xc));
    if material != 0 {
        h.call_sym(MATERIAL_DTOR, Conv::Thiscall(material), &[]);
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[material]);
    }
    h.write(this.wrapping_add(0xc), 0u32);
}

// ---- ClassChrShaderRender_VertexInstanceSetup

/// `operator new(n)` (`0x00630310`, cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `operator new[](n)` (`0x006304c0`, cdecl).
pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
/// `crt_memset_thunk(p, c, n)` (`0x0094964c`, cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `"Shadowmap"` (`0x00b480c8`), the segments' name.
pub const SHADOWMAP: u32 = 0x00b4_80c8;
/// The bounds' minimum and maximum (`0x00b41db0` -10000.0, `0x00b480d8`
/// 20000.0, f32).
pub const BOUNDS_MIN: u32 = 0x00b4_1db0;
/// See [`BOUNDS_MIN`].
pub const BOUNDS_MAX: u32 = 0x00b4_80d8;

/// `ClassChrShaderRender_VertexInstanceSetup(this)` (`0x006a0100`,
/// thiscall): two render-data blocks at `+0` and `+4`, each a 0xd0-byte
/// `operator new` block (`+0xb8` = 2, `+0xbc` = 0) with one 0x6c-byte
/// segment at `+0x94` (`+4`, `+8` = 0; buffers from `operator new[]`,
/// cleared: `+0x14` 0x1f40 bytes, `+0x1c` 0xbb8, `+0x10` 0x1770, `+0x18`
/// 0xfa0, `+0xc` 0x1770; `+0x24` = 1, `+0x20` = `"Shadowmap"`; `+0x2c`,
/// `+0x48`, `+0x30`, `+0x4c`, `+0x34`, `+0x50` = 0), `+0x90` = 1, the
/// identity at `+0`, the bounds from the image's -10000 (`+0x98`..) to
/// 20000 (`+0xa4`..). Returns the second segment (left in `eax`).
pub fn vertex_instance_setup(h: &mut impl Host, this: u32) -> u32 {
    let mut segment = 0;
    for i in 0..2u32 {
        let slot = this.wrapping_add(4 * i);
        let data = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xd0]);
        h.write(slot, data);
        h.write(data.wrapping_add(0xb8), 2u32);
        let data: u32 = h.read(slot);
        h.write(data.wrapping_add(0xbc), 0u32);
        let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x6c]);
        let data: u32 = h.read(slot);
        h.write(data.wrapping_add(0x94), block);
        for off in [4u32, 8] {
            let seg = segment_of(h, slot);
            h.write(seg.wrapping_add(off), 0u32);
        }
        for (off, size) in BUFFERS {
            let buffer = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[size]);
            let seg = segment_of(h, slot);
            h.write(seg.wrapping_add(off), buffer);
        }
        let seg = segment_of(h, slot);
        h.write(seg.wrapping_add(0x24), 1u32);
        let name = h.rebase(SHADOWMAP);
        let seg = segment_of(h, slot);
        h.write(seg.wrapping_add(0x20), name);
        for (off, size) in BUFFERS {
            let seg = segment_of(h, slot);
            let buffer: u32 = h.read(seg.wrapping_add(off));
            h.call_sym(MEMSET, Conv::Cdecl, &[buffer, 0, size]);
        }
        let data: u32 = h.read(slot);
        h.write(data.wrapping_add(0x90), 1u32);
        for k in 0..16u32 {
            let one = if k % 5 == 0 { 1.0f32 } else { 0.0 };
            h.write(data.wrapping_add(4 * k), one.to_bits());
        }
        let low = pass(h.read(h.rebase(BOUNDS_MIN)));
        let high = pass(h.read(h.rebase(BOUNDS_MAX)));
        for k in 0..3u32 {
            h.write(data.wrapping_add(0x98 + 4 * k), low);
            h.write(data.wrapping_add(0xa4 + 4 * k), high);
        }
        h.write(data.wrapping_add(0xb8), 2u32);
        for off in [0x2cu32, 0x48, 0x30, 0x4c, 0x34, 0x50] {
            segment = segment_of(h, slot);
            h.write(segment.wrapping_add(off), 0u32);
        }
    }
    segment
}

/// The buffers of a segment: (offset, bytes), in the order allocated and
/// cleared.
const BUFFERS: [(u32, u32); 5] = [
    (0x14, 0x1f40),
    (0x1c, 0xbb8),
    (0x10, 0x1770),
    (0x18, 0xfa0),
    (0xc, 0x1770),
];

/// The segment of the render data in `slot`: `[[slot] + 0x94]`.
fn segment_of(h: &impl Host, slot: u32) -> u32 {
    let data: u32 = h.read(slot);
    h.read(data.wrapping_add(0x94))
}
