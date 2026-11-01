//! M5 stage 1, list `phyre-scene`, round big-phyre3
//! (`ffx_prender::m5p_scene_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_bp3_*`).
//!
//! Functions that hand callees the addresses of their locals take `ebp`,
//! where the original's frame pointer is, and keep those locals at the
//! original's offsets below it; the shim passes the top of a
//! `StackFrame`, the tests the original's frame address. The string
//! builders the originals keep in their frames are made, appended to and
//! destroyed by the original `PStringBuilder` code in the same order,
//! error paths included; none of it throws, so the exception frames that
//! would destroy them on unwinding have no counterpart.

/// `0x004d4810`'s callees: `decodePairs_004d3d30` (cdecl, four words),
/// `PhyreMalloc(size, align)`, `PArray_releaseTaggedWords_00434360`
/// (cdecl `(data, n)`), `PStringBuilder(text)` (thiscall), `operator+=`
/// (thiscall `(builder; text)`), `appendInt` (thiscall), `memcpy`,
/// `PhyreFree(p)`, the light and shadow type tables (`0x0049e4e0`,
/// `0x004cb750`, cdecl one word), `~PStringBuilder` (`ecx`),
/// `PArray_truncateTagged_004d5940` (thiscall, one word).
pub const LIGHT_STRING_CALLS: [u32; 12] = [
    0x004d_3d30,
    0x0042_fb80,
    0x0043_4360,
    0x0044_49e0,
    0x0044_4bf0,
    0x0044_4b90,
    0x0094_9256,
    0x0042_fb20,
    0x0049_e4e0,
    0x004c_b750,
    0x0044_4ab0,
    0x004d_5940,
];
/// The texts: the count's prefix, `"-DLIGHTTYPE_"`, `"-DSHADOWTYPE_"`,
/// `"="`.
pub const LIGHT_STRING_TEXTS: [u32; 4] = [0x00b1_f99c, 0x00b1_f9a0, 0x00b1_f9b0, 0x00b7_ca94];
/// `0x004d4810`'s frame below `ebp` (down to the main builder at
/// `ebp - 0x88`), in words.
pub const LIGHT_STRINGS_FRAME_WORDS: u32 = 0x8c / 4;

pub mod light_strings {
    //! `0x004d4810` over game memory.

    use super::{LIGHT_STRING_CALLS, LIGHT_STRING_TEXTS};
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The builder's text (`[b + 4]`, `[b + 8]` bytes with the NUL) copied
    /// into a new block (`PhyreMalloc(n, 2)`, n = the length or 1) for the
    /// tagged string slot `slot` (its old owned text freed); `false` when
    /// the allocation fails (nothing stored then).
    fn store(h: &mut impl Host, builder: u32, slot: u32) -> bool {
        let [_, malloc, _, _, _, _, memcpy, free, ..] = LIGHT_STRING_CALLS;
        let len: u32 = h.read(builder + 8);
        let text = if len == 0 { 0 } else { h.read(builder + 4) };
        let size = if len == 0 { 1 } else { len };
        let mut copy = 0;
        if text != 0 {
            copy = h.call_sym(malloc, Conv::Cdecl, &[size, 2]);
            if copy == 0 {
                return false;
            }
            h.call_sym(memcpy, Conv::Cdecl, &[copy, text, size]);
            if copy == 0 {
                return false;
            }
        }
        if h.read::<u8>(slot) & 1 == 0 {
            let old: u32 = h.read(slot);
            h.call_sym(free, Conv::Cdecl, &[old]);
        }
        h.write(slot, copy);
        true
    }

    /// A builder at `builder` of `prefix`, the index, `"="` and `name`.
    fn define(h: &mut impl Host, builder: u32, prefix: u32, i: u32, name: u32) {
        let [_, _, _, ctor, append, append_int, ..] = LIGHT_STRING_CALLS;
        let equals = h.rebase(LIGHT_STRING_TEXTS[3]);
        let prefix = h.rebase(prefix);
        this_call(h, ctor, builder, &[prefix]);
        this_call(h, append_int, builder, &[i]);
        this_call(h, append, builder, &[equals]);
        this_call(h, append, builder, &[name]);
    }

    /// The strings after the count: per light its type define (builder at
    /// `ebp - 0x70`) and, with a shadow, its shadow define (`ebp - 0x7c`),
    /// into the slots from index 1; `None` when an allocation fails (the
    /// builders made so far destroyed), else the next index.
    fn defines(h: &mut impl Host, ebp: u32, out: u32, n: u32) -> Option<u32> {
        let [.., light_table, shadow_table, dtor, _] = LIGHT_STRING_CALLS;
        let [_, light_prefix, shadow_prefix, _] = LIGHT_STRING_TEXTS;
        let mut idx = 1u32;
        for i in 0..n {
            let kind: u32 = h.read(ebp - 0x50 + 4 * i);
            let entry = h.call_sym(light_table, Conv::Cdecl, &[kind]);
            let name: u32 = h.read(entry + 4);
            let light = ebp - 0x70;
            define(h, light, light_prefix, i, name);
            let slot = h.read::<u32>(out + 4).wrapping_add(4 * idx);
            if !store(h, light, slot) {
                this_call(h, dtor, light, &[]);
                return None;
            }
            idx += 1;
            let shadow: u32 = h.read(ebp - 0x30 + 4 * i);
            if shadow != 0 {
                let entry = h.call_sym(shadow_table, Conv::Cdecl, &[shadow]);
                let name: u32 = h.read(entry + 4);
                let builder = ebp - 0x7c;
                define(h, builder, shadow_prefix, i, name);
                let slot = h.read::<u32>(out + 4).wrapping_add(4 * idx);
                if !store(h, builder, slot) {
                    this_call(h, dtor, builder, &[]);
                    this_call(h, dtor, light, &[]);
                    return None;
                }
                idx += 1;
                this_call(h, dtor, builder, &[]);
            }
            this_call(h, dtor, light, &[]);
        }
        Some(idx)
    }

    /// `Phyre_PRendering_PContextSwitchLights_getStringsFromPackedState(this;
    /// a, b, out)` (thiscall, ret 0xc; Fahrenheit's name): the light and
    /// shadow types decoded (`decodePairs(ebp - 0x50, ebp - 0x30, a, b)`,
    /// n lights); the string array `out` sized to 2n + 1 tagged words
    /// (a new zeroed block, 0xd when it can't be had; the old owned words
    /// released); string 0 the count define (`prefix`, `[this + 4]`, `=`,
    /// n; builder at `ebp - 0x88`), then the light and shadow defines
    /// ([`defines`]), the array cut to the strings made
    /// (`truncateTagged`). 0, or 0xd when a string's allocation fails.
    pub fn get_strings(h: &mut impl Host, ebp: u32, this: u32, a: u32, b: u32, out: u32) -> u32 {
        let [
            decode,
            malloc,
            release,
            ctor,
            append,
            append_int,
            _,
            _,
            _,
            _,
            dtor,
            truncate,
        ] = LIGHT_STRING_CALLS;
        let n = h.call_sym(decode, Conv::Cdecl, &[ebp - 0x50, ebp - 0x30, a, b]);
        let want = n.wrapping_mul(2).wrapping_add(1);
        if want != h.read::<u32>(out) {
            let mut data = 0;
            if want != 0 {
                data = h.call_sym(malloc, Conv::Cdecl, &[want.wrapping_mul(4), 4]);
                if data == 0 {
                    return 0xd;
                }
                for k in 0..want {
                    h.write(data.wrapping_add(4 * k), 0u32);
                }
            }
            let head: u32 = h.read(out);
            let old: u32 = h.read(out + 4);
            if old != data && head >> 31 == 0 {
                h.call_sym(release, Conv::Cdecl, &[old, head & 0x7fff_ffff]);
            }
            h.write(out + 4, data);
            h.write(out, want);
        }
        let main = ebp - 0x88;
        let prefix = h.rebase(LIGHT_STRING_TEXTS[0]);
        let equals = h.rebase(LIGHT_STRING_TEXTS[3]);
        this_call(h, ctor, main, &[prefix]);
        let name: u32 = h.read(this + 4);
        this_call(h, append, main, &[name]);
        this_call(h, append, main, &[equals]);
        this_call(h, append_int, main, &[n]);
        let slot: u32 = h.read(out + 4);
        let result = if store(h, main, slot) {
            match defines(h, ebp, out, n) {
                Some(idx) => {
                    this_call(h, truncate, out, &[idx]);
                    0
                }
                None => 0xd,
            }
        } else {
            0xd
        };
        this_call(h, dtor, main, &[]);
        result
    }
}
