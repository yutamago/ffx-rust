//! List `p5-core`, round 4: `0x004d4140` (stdcall, 7 words, `ret 0x1c`,
//! `eax` 0), the per-instance light selection of the mesh-segment
//! context code (name ours: it packs the lights an instance's masks
//! select into a bit stream; its error message names
//! `GetContextSwitchesForInstance`).
//!
//! Arguments: `(masks, segments, instance, &words, out, _, mode)`.
//! - Unless `mode` is the global at `0x00c160ec`, each context switch of
//!   `segments` (`+0x10`: a `PArray` of switch pointers, inline when one)
//!   with a parameter block of at least 0x40 bytes (`+0x14`) copies the
//!   instance's buffer (`+0xac`) into the switch's data (`+4`) at the
//!   block's offset (`+8`, from byte 4, length `+0xa` & 0x1fff, if it
//!   fits the data's size word), and the light list is the instance's
//!   `+0xa4` instead of `+0x9c`. A null switch prints the error to
//!   `stderr` and writes to address 0 (the original's deliberate crash).
//! - Each light of the list whose type (`+0x18`) has a bit of the first
//!   mask (`+0xc`) is kept as `(light, key)`: the key is the type's
//!   `+0x10`, or'ed with its shadow's `+0x10` shifted by 5 when the
//!   light's `+0x14` has a shadow (`+4`) with a bit of the second mask.
//!   The pairs live in a block the size of the list (the original's
//!   `_alloca` of 8 bytes a light).
//! - At most `masks + 0x38` are used. Unless `mode` is the global: none
//!   writes a 0 word at `*words` and moves it on; more than one sorts
//!   the pairs by key (`std::sort` `0x004d21c0` with `0x004d3700`).
//! - The bit stream at `*words` (cleared, `(10 n + 0x23) / 32` words):
//!   the count in 4 bits, then each key in 10; the lights go to `out`,
//!   the count to `out + 0x10`, and `*words` moves past the stream.

use ffx_graphic::host::{Conv, Host};

/// The mode that skips the switches and the sort (a global's word).
const SKIP_MODE: u32 = 0x00c1_60ec;
/// `std::sort` of the pairs `(first, last, count, less)` (cdecl) and
/// the key comparator.
const SORT: u32 = 0x004d_21c0;
const BY_KEY: u32 = 0x004d_3700;
/// `PBitArray::setBits(value, bit, count)` (thiscall on `&words`).
const SET_BITS: u32 = 0x004d_55d0;
/// The CRT's `memset` and `memcpy` (cdecl).
const MEMSET: u32 = 0x0094_964c;
const MEMCPY: u32 = 0x0094_9256;
/// The CRT imports `__iob_func`, `fprintf`, `fflush` (slots).
const IOB_FUNC: u32 = 0x00b0_c350;
const FPRINTF: u32 = 0x00b0_c38c;
const FFLUSH: u32 = 0x00b0_c3bc;
/// The error's three strings (colour on, message, colour off).
const ERROR_TEXT: [u32; 3] = [0x00b1_f84c, 0x00b1_f9cc, 0x00b1_f854];

/// The ports over game memory.
pub mod bridge {
    use super::{
        BY_KEY, Conv, ERROR_TEXT, FFLUSH, FPRINTF, Host, IOB_FUNC, MEMCPY, MEMSET, SET_BITS,
        SKIP_MODE, SORT,
    };

    /// The elements of a `PArray` of words at `arr` (`{count | flag,
    /// data}`, the one element inline).
    fn elements(h: &mut impl Host, arr: u32) -> u32 {
        let n = h.read::<u32>(arr) & 0x7fff_ffff;
        match n {
            0 => 0,
            1 => arr + 4,
            _ => h.read(arr + 4),
        }
    }

    /// Calls the CRT import in `slot` (cdecl).
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let slot = h.rebase(slot);
        let f: u32 = h.read(slot);
        h.call(f, Conv::Cdecl, args)
    }

    /// The null-switch error: three lines to `stderr`, a flush, then the
    /// original's write to address 0.
    fn fail(h: &mut impl Host) {
        for text in ERROR_TEXT {
            let s = h.rebase(text);
            let iob = import(h, IOB_FUNC, &[]);
            import(h, FPRINTF, &[iob.wrapping_add(0x40), s]);
        }
        let iob = import(h, IOB_FUNC, &[]);
        import(h, FFLUSH, &[iob.wrapping_add(0x40)]);
        h.write(0, 0u32);
    }

    /// The switches' parameter copies; the light list to use.
    fn copy_parameters(h: &mut impl Host, segments: u32, instance: u32) -> u32 {
        let mut list = instance.wrapping_add(0x9c);
        let arr: u32 = h.read(segments + 0x10);
        let n = h.read::<u32>(arr) & 0x7fff_ffff;
        for i in 0..n {
            let arr: u32 = h.read(segments + 0x10);
            let first = elements(h, arr);
            let switch: u32 = h.read(first.wrapping_add(4 * i));
            if switch == 0 {
                fail(h);
            }
            let block: u32 = h.read(switch.wrapping_add(0x14));
            if block < 0x40 {
                continue;
            }
            let data: u32 = h.read(switch.wrapping_add(4));
            list = instance.wrapping_add(0xa4);
            let src = instance.wrapping_add(0xac);
            if src == 0 {
                continue;
            }
            let offset = u32::from(h.read::<u16>(block + 8));
            let len = u32::from(h.read::<u16>(block + 0xa) & 0x1fff);
            let size: u32 = h.read(data);
            if offset.wrapping_add(len) <= size && offset >= 4 {
                h.call_sym(MEMCPY, Conv::Cdecl, &[data.wrapping_add(offset), src, len]);
            }
        }
        list
    }

    /// The lights of `list` the masks select, as `(light, key)` pairs at
    /// `pairs` (8 bytes each); their count.
    fn select(h: &mut impl Host, masks: u32, list: u32, pairs: u32) -> u32 {
        let first_mask: u32 = h.read(masks);
        let second_mask: u32 = h.read(masks + 4);
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        let mut kept = 0u32;
        for i in 0..n {
            let lights: u32 = h.read(list + 4);
            let light: u32 = h.read(lights.wrapping_add(4 * i));
            let ty: u32 = h.read(light + 0x18);
            if h.read::<u32>(ty + 0xc) & first_mask == 0 {
                continue;
            }
            let holder: u32 = h.read(light + 0x14);
            let shadow: u32 = if holder == 0 { 0 } else { h.read(holder + 4) };
            let extra = if shadow != 0 && h.read::<u32>(shadow + 0xc) & second_mask != 0 {
                h.read(shadow + 0x10)
            } else {
                0
            };
            let ty: u32 = h.read(light + 0x18);
            let key = (extra << 5) | h.read::<u32>(ty + 0x10);
            let at = pairs.wrapping_add(8 * kept);
            h.write(at, light);
            h.write(at + 4, key);
            kept += 1;
        }
        kept
    }

    /// The bytes [`pack_lights`] needs at `pairs`: 8 a light of the list
    /// it will use (the larger of the two lists' counts covers both).
    pub fn pairs_len(h: &mut impl Host, instance: u32) -> u32 {
        let a = h.read::<u32>(instance.wrapping_add(0x9c)) & 0x7fff_ffff;
        let b = h.read::<u32>(instance.wrapping_add(0xa4)) & 0x7fff_ffff;
        a.max(b).wrapping_mul(8)
    }

    /// `0x004d4140(masks, segments, instance, words, out, _, mode)`: 0.
    /// `pairs` is the pairs' block ([`pairs_len`] bytes), `cursor` a word
    /// the bit writer gets the address of.
    pub fn pack_lights(h: &mut impl Host, args: [u32; 7], pairs: u32, cursor: u32) -> u32 {
        let [masks, segments, instance, words, out, _, mode] = args;
        let skip_at = h.rebase(SKIP_MODE);
        let skip = mode == h.read::<u32>(skip_at);
        let list = if skip {
            instance.wrapping_add(0x9c)
        } else {
            copy_parameters(h, segments, instance)
        };
        let kept = select(h, masks, list, pairs);
        let limit: u32 = h.read(masks + 0x38);
        let n = kept.min(limit);
        let skip = mode == h.read::<u32>(skip_at);
        if !skip {
            if n == 0 {
                let at: u32 = h.read(words);
                h.write(at, 0u32);
                h.write(words, at.wrapping_add(4));
                return 0;
            }
            if n != 1 {
                let end = pairs.wrapping_add(8 * kept);
                let less = h.rebase(BY_KEY);
                h.call_sym(
                    SORT,
                    Conv::Cdecl,
                    &[
                        pairs,
                        end,
                        ((end.wrapping_sub(pairs)) as i32 >> 3) as u32,
                        less,
                    ],
                );
            }
        }
        let bytes = (n.wrapping_mul(10).wrapping_add(0x23) >> 5) << 2;
        let stream: u32 = h.read(words);
        h.call_sym(MEMSET, Conv::Cdecl, &[stream, 0, bytes]);
        let stream: u32 = h.read(words);
        h.write(cursor, stream);
        h.call_sym(SET_BITS, Conv::Thiscall(cursor), &[n, 0, 4]);
        let mut bit = 4u32;
        for i in 0..n {
            let at = pairs.wrapping_add(8 * i);
            let light: u32 = h.read(at);
            let key: u32 = h.read(at + 4);
            h.write(out.wrapping_add(4 * i), light);
            h.call_sym(SET_BITS, Conv::Thiscall(cursor), &[key, bit, 10]);
            bit = bit.wrapping_add(10);
        }
        let at: u32 = h.read(words);
        h.write(words, at.wrapping_add(bytes));
        h.write(out + 0x10, n);
        0
    }
}
