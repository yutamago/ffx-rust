//! M5 stage 1, list `scene`, batch 13: the shader LOD context switch's
//! define string (`getStringsFromPackedState`). Conventions as in
//! [`crate::m5p_scene_b`]; the string builder lives in a frame (the
//! original's `ebp-0x1c`, three words) handed to the builder's methods.

use ffx_graphic::host::{Conv, Host};

pub use crate::m5p_scene_b::PHYRE_FREE;
pub use crate::m5p_scene_c::PHYRE_MALLOC;
pub use crate::m5p_scene_e::MEMCPY;

/// `PStringBuilder`: the constructor from a C string (thiscall, `ret 4`),
/// `operator+=` of a C string (thiscall, `ret 4`), the append of an
/// integer (thiscall, `ret 4`) and the destructor (thiscall).
pub const BUILDER_CTOR: u32 = 0x0044_49e0;
pub const BUILDER_APPEND: u32 = 0x0044_4bf0;
pub const BUILDER_APPEND_INT: u32 = 0x0044_4b90;
pub const BUILDER_DTOR: u32 = 0x0044_4ab0;
/// `PArray_releaseTaggedWords_00434360(data, count)` (cdecl): a
/// `PArray<PString>`'s release.
pub const STRINGS_RELEASE: u32 = 0x0043_4360;
/// The define prefix (`"-D"`) and the separator (`"="`).
pub const DEFINE: u32 = 0x00b1_f99c;
pub const EQUALS: u32 = 0x00b7_ca94;

pub mod bridge {
    //! The ports over game memory.

    use super::{
        BUILDER_APPEND, BUILDER_APPEND_INT, BUILDER_CTOR, BUILDER_DTOR, Conv, DEFINE, EQUALS, Host,
        MEMCPY, PHYRE_FREE, PHYRE_MALLOC, STRINGS_RELEASE,
    };

    const EXTERNAL: u32 = 0x8000_0000;

    /// `PContextSwitchShaderLODLevel::getStringsFromPackedState` (`0x004d4ba0`,
    /// thiscall `(&cursor, unused, strings)`, `ret 0xc`): the next packed
    /// word (`*cursor`, advanced); for a level other than 0, `strings` made
    /// a one-string array (a fresh buffer unless it holds one; an owned old
    /// one other than it released) whose string becomes `"-D" + name
    /// (this + 4) + "=" + level` (a copy of the builder's text,
    /// `PhyreMalloc(len, 2)`; the old string freed unless unowned). 0, or
    /// 0xd when an allocation failed.
    pub fn lod_level_strings(
        h: &mut impl Host,
        this: u32,
        cursor: u32,
        strings: u32,
        frame: u32,
    ) -> u32 {
        let at: u32 = h.read(cursor);
        h.write(cursor, at.wrapping_add(4));
        let level: u32 = h.read(at);
        if level == 0 {
            return 0;
        }
        let count: u32 = h.read(strings);
        if count != 1 {
            let buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[4, 4]);
            if buf == 0 {
                return 0xd;
            }
            h.write(buf, 0u32);
            let header: u32 = h.read(strings);
            let old: u32 = h.read(strings + 4);
            if old != buf && header & EXTERNAL == 0 {
                h.call_sym(STRINGS_RELEASE, Conv::Cdecl, &[old, header & !EXTERNAL]);
            }
            h.write(strings + 4, buf);
            h.write(strings, 1u32);
        }
        let define = h.rebase(DEFINE);
        h.call_sym(BUILDER_CTOR, Conv::Thiscall(frame), &[define]);
        let name: u32 = h.read(this + 4);
        h.call_sym(BUILDER_APPEND, Conv::Thiscall(frame), &[name]);
        let equals = h.rebase(EQUALS);
        h.call_sym(BUILDER_APPEND, Conv::Thiscall(frame), &[equals]);
        h.call_sym(BUILDER_APPEND_INT, Conv::Thiscall(frame), &[level]);
        let len: u32 = h.read(frame + 8);
        let text: u32 = if len == 0 { 0 } else { h.read(frame + 4) };
        let size = if len == 0 { 0 } else { len - 1 } + 1;
        let slot: u32 = h.read(strings + 4);
        let mut copy = 0;
        if text != 0 {
            copy = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 2]);
            if copy != 0 {
                h.call_sym(MEMCPY, Conv::Cdecl, &[copy, text, size]);
            }
            if copy == 0 {
                h.call_sym(BUILDER_DTOR, Conv::Thiscall(frame), &[]);
                return 0xd;
            }
        }
        let lo: u8 = h.read(slot);
        if lo & 1 == 0 {
            let old: u32 = h.read(slot);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(slot, copy);
        h.call_sym(BUILDER_DTOR, Conv::Thiscall(frame), &[]);
        0
    }
}
