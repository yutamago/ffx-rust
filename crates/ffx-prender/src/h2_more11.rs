//! h2-render-b continuation round, batch 5: the scene-wide parameters'
//! one-time set-up (`PSceneWideParameters::initialize`).

use ffx_graphic::host::{Conv, Host};

use crate::descriptor::PHYRE_FREE;

/// The parameter definitions (`PSharray` of 16-byte entries: count word,
/// storage, the 30 inline entries) and their packed names.
const DEFS_COUNT: u32 = 0x00ca_8510;
const DEFS_DATA: u32 = 0x00ca_8514;
const DEFS_INLINE: u32 = 0x00ca_8330;
const NAMES: u32 = 0x00ca_8768;
/// `g_SceneWideParameterTable`: (name, id, type) x 30.
const TABLE: u32 = 0x00b2_3bc8;
/// The buffer layout: the stride (u16), the CPU block, its count, a word.
const STRIDE: u32 = 0x00ca_8518;
const BLOCK: u32 = 0x00ca_851c;
const BLOCK_COUNT: u32 = 0x00ca_8520;
const BLOCK_WORD: u32 = 0x00ca_8544;
/// The constant-buffer array (`PArray<PConstantBuffer>`: count, data).
const BUFFERS: u32 = 0x00ca_8524;
const BUFFERS_DATA: u32 = 0x00ca_8528;
const CRITICAL_SECTION: u32 = 0x00ca_852c;
const CACHE: u32 = 0x00ca_8548;

const RELEASE_TAGGED: u32 = 0x004a_b4d0;
const MEMCPY: u32 = 0x0094_9256;
const MEMSET: u32 = 0x0094_964c;
const MALLOC_ALIGNED: u32 = 0x0042_fb80;
const PLATFORM_KIND: u32 = 0x0059_e530;
const LAYOUT_SIZE: u32 = 0x005a_0600;
const CONSTRUCT_ARRAY: u32 = 0x004f_d960;
const SET_STORAGE: u32 = 0x004f_ed50;
const RESIZE_SLOTS: u32 = 0x0059_9a80;
const CRITICAL_SECTION_INIT: u32 = 0x0043_17f0;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BLOCK, BLOCK_COUNT, BLOCK_WORD, BUFFERS, BUFFERS_DATA, CACHE, CONSTRUCT_ARRAY,
        CRITICAL_SECTION, CRITICAL_SECTION_INIT, Conv, DEFS_COUNT, DEFS_DATA, DEFS_INLINE, Host,
        LAYOUT_SIZE, MALLOC_ALIGNED, MEMCPY, MEMSET, NAMES, PHYRE_FREE, PLATFORM_KIND,
        RELEASE_TAGGED, RESIZE_SLOTS, SET_STORAGE, STRIDE, TABLE,
    };

    /// The bytes of `initialize`'s frame (`ebp - 0xc` up to `ebp`).
    pub const INIT_FRAME: u32 = 0xc;

    /// `PSceneWideParameters::initialize(buffers)` (`0x004fd9e0`, cdecl):
    /// the 30 parameter definitions (their old storage's tagged elements
    /// released when owned) set inline from `g_SceneWideParameterTable`:
    /// each name copied (`memcpy`, with its NUL) into the packed buffer
    /// `0x00ca8768` at a 2-aligned offset and tagged (`+1`) into the
    /// entry (an untagged old pointer freed), the id and type bytes, the
    /// entry's word `+4` cleared and `+6` kept to its top three bits;
    /// then the layout's size (`0x005a0600` for the platform's kind, 16-
    /// aligned) as the stride, a zeroed CPU block of `stride * n` (`n` =
    /// `buffers` rounded up to even), the constant-buffer array sized to
    /// `n` (60-byte entries) with each buffer's slots made 8 and its size
    /// 0x144, the 0x220-byte cache cleared, the critical section set up.
    /// Returns 0. `frame` is the original's `ebp - 0xc`.
    #[allow(clippy::too_many_lines)]
    pub fn initialize(h: &mut impl Host, buffers: u32, frame: u32) -> u32 {
        let local = |off: u32| frame + 0xc - off;
        let count_at = h.rebase(DEFS_COUNT);
        let data_at = h.rebase(DEFS_DATA);
        let inline = h.rebase(DEFS_INLINE);
        let data: u32 = h.read(data_at);
        if data != inline {
            let count: u32 = h.read(count_at);
            if count >> 31 == 0 {
                h.call_sym(RELEASE_TAGGED, Conv::Cdecl, &[data, count & 0x7fff_ffff]);
            }
        }
        h.write(data_at, inline);
        h.write(count_at, 0x8000_001eu32);
        let mut cursor = h.rebase(NAMES);
        let table = h.rebase(TABLE);
        for k in 0..30u32 {
            let row = table + 12 * k;
            let entry = inline + 0x10 * k + 4;
            let name: u32 = h.read(row);
            cursor = cursor.wrapping_add(cursor.wrapping_neg() & 1);
            let mut len = 0u32;
            while h.read::<u8>(name.wrapping_add(len)) != 0 {
                len += 1;
            }
            let len = len + 1;
            h.write(local(8), name.wrapping_add(1));
            h.write(local(0xc), len);
            h.call_sym(MEMCPY, Conv::Cdecl, &[cursor, name, len]);
            if cursor & 1 == 0 {
                let tagged = if cursor != 0 { cursor + 1 } else { 0 };
                h.write(local(8), tagged);
                let old: u32 = h.read(entry);
                if old & 1 == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
                h.write(entry, tagged);
            }
            let id: u8 = h.read(row + 4);
            let kind: u8 = h.read(row + 8);
            cursor = cursor.wrapping_add(len);
            h.write(entry - 2, id);
            h.write(entry - 1, kind);
            h.write(entry + 4, 0u16);
            let w: u16 = h.read(entry + 6);
            h.write(entry + 6, w & 0xe000);
        }
        let platform = h.call_sym(PLATFORM_KIND, Conv::Cdecl, &[]);
        let count = h.read::<u32>(count_at) & 0x7fff_ffff;
        let data: u32 = h.read(data_at);
        h.call_sym(
            LAYOUT_SIZE,
            Conv::Cdecl,
            &[8, data, count, local(4), platform],
        );
        let size = h.read::<u32>(local(4)).wrapping_add(0xf) & !0xf;
        h.write(local(4), size);
        let n = buffers.wrapping_add(1) & !1;
        h.write(h.rebase(STRIDE), size as u16);
        let bytes = size.wrapping_mul(n);
        let block = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[bytes, 4]);
        h.write(h.rebase(BLOCK), block);
        h.write(h.rebase(BLOCK_COUNT), n);
        h.write(h.rebase(BLOCK_WORD), 0u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[block, 0, bytes]);
        let array = h.rebase(BUFFERS);
        let mut fill = n != 0;
        if n != h.read::<u32>(array) {
            let mut p = 0;
            let mut store = true;
            if n != 0 {
                p = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[n.wrapping_mul(60), 4]);
                if p != 0 {
                    h.call_sym(CONSTRUCT_ARRAY, Conv::Cdecl, &[p, n]);
                } else {
                    // no storage: the original goes straight to the
                    // buffers' set-up
                    store = false;
                }
            }
            if store {
                h.call_sym(SET_STORAGE, Conv::Thiscall(array), &[p, n]);
                fill = n != 0;
            }
        }
        if fill {
            for k in 0..n {
                let base: u32 = h.read(h.rebase(BUFFERS_DATA));
                let b = base.wrapping_add(60 * k);
                let c = h.read::<u32>(b + 0xc) & 0x7fff_ffff;
                if c != 8 && h.call_sym(RESIZE_SLOTS, Conv::Thiscall(b), &[8]) == 0 {
                    h.write(b + 4, 0u32);
                }
                h.write(b, 0x144u32);
            }
        }
        let cache = h.rebase(CACHE);
        h.call_sym(MEMSET, Conv::Cdecl, &[cache, 0, 0x220]);
        let cs = h.rebase(CRITICAL_SECTION);
        h.call_sym(CRITICAL_SECTION_INIT, Conv::Thiscall(cs), &[]);
        0
    }
}
