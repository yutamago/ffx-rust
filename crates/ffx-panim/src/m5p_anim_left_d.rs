//! List `phyre-anim-left`, batch 4 (M5 stage 1): more of the animation
//! sets' and components' arrays, the scene bind's key tree, the shader
//! parameter bind and two float helpers.
//!
//! - `PSharray`-style word arrays that keep one element in place:
//!   resize and "set to external buffer";
//! - array resizes that copy the kept elements and construct the rest
//!   (16-byte slot entries, `0x4c`-byte source records), the source
//!   records' copy constructor, a named-target constructor;
//! - `PAnimationSet`'s clip lookup by name (exact, then `strstr`);
//! - the scene bind's key order and tree insert (as
//!   `PAnimationSetTargetLookup`'s, with its own compare);
//! - `PAnimationChannelTargetBindShaderParameter::bindTargets`;
//! - the component's source count and clip-set changes;
//! - a 3x3 rows-times-scale and the inverse-lerp blend of
//!   `FFXApplication`.
//!
//! Callees outside the list stay calls to the original; no state.

#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE};

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The default name `0x0051bb80` gives a target (cdecl, no arguments).
pub const DEFAULT_NAME: u32 = 0x0043_5740;
/// The small word array's allocating copy (thiscall `(total, src, n)`: one element goes in place at `this + 4`).
pub const SMALL_ALLOC_COPY: u32 = 0x0051_c270;
/// MSVCR110's `strstr` import slot.
pub const STRSTR: u32 = 0x00b0_c378;
/// The scene-key compare `0x00617260` (in this list).
pub const SCENE_KEY_COMPARE: u32 = 0x0061_7260;
/// The two descriptors a shader-parameter target may name its object by.
pub const SHADER_DESC_A: u32 = 0x00ca_5040;
/// See [`SHADER_DESC_A`] (its object holds the material at `+4`).
pub const SHADER_DESC_B: u32 = 0x00ca_5268;
/// The material's parameter lookup by name (thiscall on `[material +
/// 4]`, `(name)`).
pub const PARAM_LOOKUP: u32 = 0x0056_cbf0;
/// The material's parameter binding (thiscall on the material,
/// `(param)`).
pub const PARAM_BIND: u32 = 0x005b_8b00;
/// The record time set (thiscall on `record + 0x38`, `(ctl)`).
pub const RECORD_SET_TIME: u32 = 0x0052_65f0;
/// The record clip bind (thiscall on `record + 0x38`, `(clip)`).
pub const RECORD_BIND: u32 = 0x0052_6590;
/// The blender's source set (thiscall on `component + 0x1c`, `(i, src)`).
pub const BLENDER_SET_SOURCE: u32 = 0x0052_cec0;
/// The blender's weight set (thiscall on `component + 0x1c`, `(i, w)`).
pub const BLENDER_SET_WEIGHT: u32 = 0x0052_e660;
/// The component's refresh (thiscall).
pub const COMPONENT_REFRESH: u32 = 0x0054_1c00;
/// The component data's clear `0x0053d100` (in this list).
pub const DATA_CLEAR: u32 = 0x0053_d100;
/// The clip set's channel-target bind (thiscall on `component + 0x38`,
/// `(component + 0x1c)`).
pub const CLIP_SET_BIND: u32 = 0x0053_1a10;
/// The source records' resize `0x00543bb0` (in this list).
pub const RECORDS_RESIZE: u32 = 0x0054_3bb0;
/// The f32 threshold `0x00a2f560` compares the difference with.
pub const BLEND_EPSILON: u32 = 0x00b6_f3cc;

/// A resize that copies the kept elements and constructs the rest
/// (`0x0051d670`, `0x00543bb0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyResize {
    pub size: u32,
    /// cdecl `(new, old, kept)`.
    pub copy: u32,
    /// cdecl `(at, n)`.
    pub construct: u32,
    /// cdecl `(old, count)` for the old storage when not the new one and
    /// owned; 0: `PhyreFree` when also set.
    pub release: u32,
}

/// The [`CopyResize`] copies.
pub const COPY_RESIZES: &[(u32, CopyResize)] = &[
    (
        0x0051_d670,
        CopyResize {
            size: 0x10,
            copy: 0x0051_7900,
            construct: 0x0051_bc30,
            release: 0,
        },
    ),
    (
        0x0054_3bb0,
        CopyResize {
            size: 0x4c,
            copy: 0x0053_d210,
            construct: 0x0054_1780,
            release: 0x0054_0b80,
        },
    ),
];

/// The [`COPY_RESIZES`] entry of `f`.
#[must_use]
pub const fn copy_resize_of(f: u32) -> CopyResize {
    let mut i = 0;
    while i < COPY_RESIZES.len() {
        if COPY_RESIZES[i].0 == f {
            return COPY_RESIZES[i].1;
        }
        i += 1;
    }
    panic!("not a copy resize")
}

/// The vtables `0x0053d210` writes into a copied source record (the time
/// controller at `+0xc`, its source at `+0x24`, the bind at `+0x38`),
/// each after the base vtable `0x00b28150` (`+0x38`: and `0x00b28870`).
pub const RECORD_VTABLES: [u32; 5] = [
    0x00b2_8150,
    0x00b2_84a0,
    0x00b2_86b8,
    0x00b2_8870,
    0x00b2_8c48,
];

pub mod bridge {
    use super::{
        BLEND_EPSILON, BLENDER_SET_SOURCE, BLENDER_SET_WEIGHT, CLIP_SET_BIND, COMPONENT_REFRESH,
        Conv, CopyResize, DATA_CLEAR, DEFAULT_NAME, Host, Mem, NOT_OWNED, PARAM_BIND, PARAM_LOOKUP,
        PHYRE_FREE, PHYRE_MALLOC, RECORD_BIND, RECORD_SET_TIME, RECORD_VTABLES, RECORDS_RESIZE,
        SHADER_DESC_A, SHADER_DESC_B, SMALL_ALLOC_COPY, STRSTR, x87,
    };
    use crate::m5p_anim_left_a::bridge::lerp4;
    use crate::m5p_anim_left_a::{strcmp, text};

    fn count(m: &impl Mem, at: u32) -> u32 {
        m.read::<u32>(at) & !NOT_OWNED
    }

    fn free(h: &mut impl Host, p: u32) {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
    }

    /// The storage of a `PArray` with one element in place: the pointer
    /// past one element, `a + 4` for one, 0 for none.
    fn small_data(m: &impl Mem, a: u32) -> u32 {
        let n = count(m, a);
        if n > 1 {
            m.read(a.wrapping_add(4))
        } else if n != 0 {
            a.wrapping_add(4)
        } else {
            0
        }
    }

    /// `0x0051bb80` (cdecl `(dst, n)`): `n` `0x1c`-byte named targets
    /// constructed: the default name ([`DEFAULT_NAME`]) then six zero
    /// words (null slots skipped).
    pub fn construct_named(h: &mut impl Host, dst: u32, n: u32) {
        for i in 0..n {
            let p = dst.wrapping_add(i.wrapping_mul(0x1c));
            if p != 0 {
                let name = h.call_sym(DEFAULT_NAME, Conv::Cdecl, &[]);
                h.write(p, name);
                for k in 1..7u32 {
                    h.write(p.wrapping_add(4 * k), 0u32);
                }
            }
        }
    }

    /// `0x0051d720` (`thiscall(n)`, `ret 4`): a word array with one
    /// element in place resized: nothing when `n` is the raw count word;
    /// else [`SMALL_ALLOC_COPY`] `(n, old, kept)` (0xd when it gives 0 for
    /// `n` != 0), the old storage freed when it is neither the new one nor
    /// in place and owned (no null check), the pointer stored unless in
    /// place, the count; 0.
    pub fn small_resize(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let raw: u32 = h.read(this);
        if n == raw {
            return 0;
        }
        let have = raw & !NOT_OWNED;
        let old = small_data(h, this);
        let new = h.call_sym(
            SMALL_ALLOC_COPY,
            Conv::Thiscall(this),
            &[n, old, n.min(have)],
        );
        if new == 0 && n != 0 {
            return 0xd;
        }
        let inline = this.wrapping_add(4);
        if old != new && old != inline && h.read::<u32>(this) & NOT_OWNED == 0 {
            free(h, old);
        }
        if new != inline {
            h.write(inline, new);
        }
        h.write(this, n);
        0
    }

    /// `0x0051dac0` (`thiscall(src, n)`, `ret 8`): the same array set to
    /// an external buffer: 5 for a negative `n`; one element is copied in
    /// place (count 1, owned), more are borrowed (count with the "not
    /// owned" flag); the old storage freed as in [`small_resize`]; 0.
    pub fn small_set_external(h: &mut impl Host, this: u32, src: u32, n: u32) -> u32 {
        if n.cast_signed() < 0 {
            return 5;
        }
        let old = small_data(h, this);
        let inline = this.wrapping_add(4);
        let (data, word) = if n == 1 {
            let v: u32 = h.read(src);
            h.write(inline, v);
            (inline, 1)
        } else {
            (src, n | NOT_OWNED)
        };
        if old != data && old != inline && h.read::<u32>(this) & NOT_OWNED == 0 {
            free(h, old);
        }
        if data != inline {
            h.write(inline, data);
        }
        h.write(this, word);
        0
    }

    /// [`super::COPY_RESIZES`] (`thiscall(n)`, `ret 4`): nothing when `n`
    /// is the raw count word; else new storage (0xd when it fails for `n`
    /// != 0), the kept elements copied, the rest constructed, the old
    /// storage released (`r.release`, or freed when set), `n` stored; 0.
    pub fn copy_resize(h: &mut impl Host, r: CopyResize, this: u32, n: u32) -> u32 {
        let raw: u32 = h.read(this);
        if n == raw {
            return 0;
        }
        let keep = n.min(raw & !NOT_OWNED);
        let old: u32 = h.read(this.wrapping_add(4));
        let mut p = 0;
        if n != 0 {
            p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(r.size), 4]);
            if p == 0 {
                return 0xd;
            }
            h.call_sym(r.copy, Conv::Cdecl, &[p, old, keep]);
            let rest = p.wrapping_add(keep.wrapping_mul(r.size));
            h.call_sym(r.construct, Conv::Cdecl, &[rest, n - keep]);
        }
        let cur: u32 = h.read(this.wrapping_add(4));
        if cur != p {
            let raw: u32 = h.read(this);
            if raw & NOT_OWNED == 0 {
                if r.release != 0 {
                    h.call_sym(r.release, Conv::Cdecl, &[cur, raw & !NOT_OWNED]);
                } else if cur != 0 {
                    free(h, cur);
                }
            }
        }
        h.write(this.wrapping_add(4), p);
        h.write(this, n);
        0
    }

    /// `0x0051ca40` (`thiscall(name)`, `ret 4`): `PAnimationSet`'s clip
    /// by name among its clips (one kept in place; names at `+0x1c`, bit 0
    /// off): an exact match first, then the first whose name contains
    /// `name` (`strstr`); 0 for none or a null `name`.
    pub fn find_clip(h: &mut impl Host, this: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let n = count(h, this);
        let data = if n > 1 {
            h.read(this.wrapping_add(4))
        } else {
            this.wrapping_add(4)
        };
        for i in 0..n {
            let clip: u32 = h.read(data.wrapping_add(i.wrapping_mul(4)));
            if clip != 0 {
                let s = text(h, clip.wrapping_add(0x1c));
                if s != 0 && strcmp(h, name, s) == 0 {
                    return clip;
                }
            }
        }
        let slot = h.rebase(STRSTR);
        let strstr: u32 = h.read(slot);
        for i in 0..n {
            if i >= count(h, this) {
                continue;
            }
            let data = small_data(h, this);
            let clip: u32 = h.read(data.wrapping_add(i.wrapping_mul(4)));
            if clip != 0 {
                let s = text(h, clip.wrapping_add(0x1c));
                if s != 0 && h.call(strstr, Conv::Cdecl, &[s, name]) != 0 {
                    return clip;
                }
            }
        }
        0
    }

    /// `0x00531310` (cdecl `(out, m, s)`): the three rows of `m` (x, y, z
    /// of `+0`, `+0x10`, `+0x20`) times `s[0..3]`, each product an `f32`;
    /// the fourth words of `out` untouched; `out`.
    pub fn scale_rows(m: &mut impl Mem, out: u32, mat: u32, s: u32) -> u32 {
        let mut rows = [[0u32; 3]; 3];
        for r in (0..3u32).rev() {
            let k = x87::load(m.read(s.wrapping_add(4 * r)));
            for c in 0..3u32 {
                let v = x87::load(m.read(mat.wrapping_add(0x10 * r + 4 * c)));
                rows[r as usize][c as usize] = x87::store(x87::mul(v, k));
            }
        }
        for (r, row) in (0u32..).zip(rows) {
            for (c, v) in (0u32..).zip(row) {
                m.write(out.wrapping_add(0x10 * r + 4 * c), v);
            }
        }
        out
    }

    /// Releases the `PArray` at `arr` (count, data): when the count word is
    /// set, the data freed when set and owned, both zeroed.
    fn release_array(h: &mut impl Host, arr: u32) {
        let raw: u32 = h.read(arr);
        if raw != 0 {
            let data: u32 = h.read(arr.wrapping_add(4));
            if data != 0 && raw & NOT_OWNED == 0 {
                free(h, data);
            }
            h.write(arr.wrapping_add(4), 0u32);
            h.write(arr, 0u32);
        }
    }

    /// `0x0053d100` (`thiscall`): the component data cleared: `+0x18`,
    /// `+0x1c`, `+0x28`, `+0x14` zero, the arrays at `+4`, `+0x20`,
    /// `+0x2c`, `+0xc` released, `+0x34` freed and zeroed; 0.
    pub fn clear_data(h: &mut impl Host, this: u32) -> u32 {
        h.write(this.wrapping_add(0x18), 0u32);
        release_array(h, this.wrapping_add(4));
        release_array(h, this.wrapping_add(0x20));
        h.write(this.wrapping_add(0x1c), 0u32);
        release_array(h, this.wrapping_add(0x2c));
        h.write(this.wrapping_add(0x28), 0u32);
        release_array(h, this.wrapping_add(0xc));
        let p: u32 = h.read(this.wrapping_add(0x34));
        h.write(this.wrapping_add(0x14), 0u32);
        free(h, p);
        h.write(this.wrapping_add(0x34), 0u32);
        0
    }

    /// `0x0053d210` (cdecl `(dst, src, n)`): the copy constructor of `n`
    /// `0x4c`-byte source records: words copied, floats through the x87
    /// (`+4`, `+8`, `+0x18..+0x20`, `+0x30`, `+0x34`), the embedded
    /// objects' vtables set as their constructors leave them
    /// ([`RECORD_VTABLES`]). A null destination takes nothing and the
    /// source stays.
    pub fn copy_records(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let vt = RECORD_VTABLES.map(|v| h.rebase(v));
        let (mut d, mut s) = (dst, src);
        for _ in 0..n {
            if d != 0 {
                copy_field(h, d, s, 0, false);
                copy_field(h, d, s, 4, true);
                copy_field(h, d, s, 8, true);
                h.write(d.wrapping_add(0xc), vt[0]);
                copy_field(h, d, s, 0x10, false);
                h.write(d.wrapping_add(0xc), vt[1]);
                copy_field(h, d, s, 0x14, false);
                copy_field(h, d, s, 0x18, true);
                copy_field(h, d, s, 0x1c, true);
                copy_field(h, d, s, 0x20, true);
                h.write(d.wrapping_add(0x24), vt[0]);
                copy_field(h, d, s, 0x28, false);
                h.write(d.wrapping_add(0x24), vt[2]);
                copy_field(h, d, s, 0x2c, false);
                copy_field(h, d, s, 0x30, true);
                copy_field(h, d, s, 0x34, true);
                h.write(d.wrapping_add(0x38), vt[0]);
                let w: u32 = h.read(s.wrapping_add(0x3c));
                h.write(d.wrapping_add(0x38), vt[3]);
                h.write(d.wrapping_add(0x3c), w);
                copy_field(h, d, s, 0x40, false);
                h.write(d.wrapping_add(0x38), vt[4]);
                copy_field(h, d, s, 0x44, false);
                copy_field(h, d, s, 0x48, false);
                s = s.wrapping_add(0x4c);
            }
            d = d.wrapping_add(0x4c);
        }
    }

    fn copy_field(h: &mut impl Host, d: u32, s: u32, off: u32, float: bool) {
        let v: u32 = h.read(s.wrapping_add(off));
        let v = if float { x87::store(x87::load(v)) } else { v };
        h.write(d.wrapping_add(off), v);
    }

    /// `0x005cdad0` (`thiscall(key, _)`, `ret 8`, `al` 1): `key` moved to
    /// the end of the `0x94`-byte records (`[this + 4]`, count `+8`): when
    /// no record starts with it, a record's first word past the end and
    /// the count one more; else the records after it moved down one and
    /// the last one's first word `key`. `eax` the records with `al` 1.
    pub fn move_to_end(m: &mut impl Mem, this: u32, key: u32) -> u32 {
        let n: u32 = m.read(this.wrapping_add(8));
        let base: u32 = m.read(this.wrapping_add(4));
        let mut i = 0;
        while i < n && m.read::<u32>(base.wrapping_add(i.wrapping_mul(0x94))) != key {
            i += 1;
        }
        if i == n {
            let base: u32 = m.read(this.wrapping_add(4));
            m.write(base.wrapping_add(n.wrapping_mul(0x94)), key);
            m.write(this.wrapping_add(8), n.wrapping_add(1));
        } else {
            while i < m.read::<u32>(this.wrapping_add(8)).wrapping_sub(1) {
                let base: u32 = m.read(this.wrapping_add(4));
                let to = base.wrapping_add(i.wrapping_mul(0x94));
                for k in 0..0x25u32 {
                    let w: u32 = m.read(to.wrapping_add(0x94 + 4 * k));
                    m.write(to.wrapping_add(4 * k), w);
                }
                i += 1;
            }
            let n: u32 = m.read(this.wrapping_add(8));
            let base: u32 = m.read(this.wrapping_add(4));
            m.write(
                base.wrapping_add(n.wrapping_mul(0x94)).wrapping_sub(0x94),
                key,
            );
        }
        let base: u32 = m.read(this.wrapping_add(4));
        base & !0xff | 1
    }

    /// `0x005cde50` (`thiscall(out, targets, n)`, `ret 0xc`):
    /// `PAnimationChannelTargetBindShaderParameter::bindTargets`: 0 for
    /// none, 1 without targets or outputs. Each unbound output (`+0` 0,
    /// 0x18 bytes) of a shader-parameter target (type `+0x10` 5, 0x1c
    /// bytes) is bound when the material (the bind's `+4`, else the
    /// target's object for descriptor A, or its `+4` for B) has the named
    /// parameter: `+0` 1, `+4` the binding, `+8` the parameter's size
    /// (`+0xa` low 13 bits) up to 16; 0.
    pub fn bind_shader_params(h: &mut impl Host, this: u32, out: u32, targets: u32, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        if targets == 0 || out == 0 {
            return 1;
        }
        let (da, db) = (h.rebase(SHADER_DESC_A), h.rebase(SHADER_DESC_B));
        for k in 0..n {
            let t = targets.wrapping_add(k.wrapping_mul(0x1c));
            let o = out.wrapping_add(k.wrapping_mul(0x18));
            if h.read::<u32>(t.wrapping_add(0x10)) != 5 || h.read::<u32>(o) != 0 {
                continue;
            }
            let mut mat: u32 = h.read(this.wrapping_add(4));
            if mat == 0 {
                let desc: u32 = h.read(t);
                mat = h.read(t.wrapping_add(4));
                if desc != da {
                    if desc != db || mat == 0 {
                        continue;
                    }
                    mat = h.read(mat.wrapping_add(4));
                }
                if mat == 0 {
                    continue;
                }
            }
            let name = text(h, t.wrapping_add(0x14));
            let params: u32 = h.read(mat.wrapping_add(4));
            let p = h.call_sym(PARAM_LOOKUP, Conv::Thiscall(params), &[name]);
            if p == 0 {
                continue;
            }
            let b = h.call_sym(PARAM_BIND, Conv::Thiscall(mat), &[p]);
            let size = u32::from(h.read::<u16>(p.wrapping_add(0xa))) & 0x1fff;
            if b != 0 {
                h.write(o, 1u32);
                h.write(o.wrapping_add(4), b);
                h.write(o.wrapping_add(8), size.min(0x10));
            }
        }
        0
    }

    /// `0x00617260` (cdecl `(a, b)`): the scene bind's key order: `+0`,
    /// then `+4` (the length) unsigned, then the `+8` bytes compared as
    /// `memcmp(b, a)`; -1, 0 or 1.
    pub fn compare_scene_keys(m: &impl Mem, a: u32, b: u32) -> u32 {
        for off in [0u32, 4] {
            let (x, y): (u32, u32) = (m.read(a.wrapping_add(off)), m.read(b.wrapping_add(off)));
            if x > y {
                return 1;
            }
            if x < y {
                return u32::MAX;
            }
        }
        let len: u32 = m.read(a.wrapping_add(4));
        let (pa, pb): (u32, u32) = (m.read(a.wrapping_add(8)), m.read(b.wrapping_add(8)));
        for i in 0..len {
            let (x, y): (u8, u8) = (m.read(pb.wrapping_add(i)), m.read(pa.wrapping_add(i)));
            if x != y {
                return if x < y { u32::MAX } else { 1 };
            }
        }
        0
    }

    /// `0x006170e0` (`thiscall(node)`, `ret 4`): the scene bind tree's
    /// insert: as `PAnimationSetTargetLookup`'s
    /// (`m5p_anim_left_a::bridge::tree_insert`) with
    /// [`compare_scene_keys`].
    pub fn scene_tree_insert(m: &mut impl Mem, tree: u32, node: u32) -> u32 {
        let pw = node.wrapping_add(8);
        let w: u32 = m.read(pw);
        m.write(pw, w.wrapping_add(1 - (w & 1)));
        m.write(node, tree);
        m.write(node.wrapping_add(4), tree);
        let mut parent = tree.wrapping_add(0xc);
        let mut cur: u32 = m.read(parent);
        let mut right = false;
        while cur != tree {
            parent = cur;
            let key = if cur == 0 { 0 } else { cur.wrapping_add(0xc) };
            let c = compare_scene_keys(m, key, node.wrapping_add(0xc));
            if c == 0 {
                return cur;
            }
            right = c.cast_signed() < 0;
            cur = m.read(cur.wrapping_add(u32::from(right) * 4));
        }
        let w: u32 = m.read(pw);
        m.write(pw, (w & 1).wrapping_add(parent));
        m.write(parent.wrapping_add(u32::from(right) * 4), node);
        0
    }

    /// `0x00a2f560` (cdecl `(out, b, a, f)`): with `f` a cdecl
    /// `float(vec)`, `d = f(a) - f(b)`: below [`BLEND_EPSILON`] `out = b`,
    /// else the lerp from `a` to `b` by `f(a) / d` (`0x0050ebf0`); `out`.
    pub fn inverse_blend(h: &mut impl Host, out: u32, b: u32, a: u32, f: u32) -> u32 {
        let fa = x87::load(h.call_st0(f, Conv::Cdecl, &[a]).to_bits());
        let fb = h.call_st0_f64(f, Conv::Cdecl, &[b]);
        let d = x87::load(x87::store(x87::sub(fa, fb)));
        let eps = h.rebase(BLEND_EPSILON);
        if d < x87::load(h.read(eps)) {
            for k in 0..4u32 {
                let v: u32 = h.read(b.wrapping_add(4 * k));
                h.write(out.wrapping_add(4 * k), v);
            }
            return out;
        }
        let t = x87::store(x87::div(fa, d));
        lerp4(h, out, t, a, b)
    }

    /// The clip a component's clip set names (`PArray` with one kept in
    /// place): its first element, 0 for none.
    fn first_clip(m: &impl Mem, set: u32) -> u32 {
        let n = count(m, set);
        if n == 0 {
            0
        } else if n > 1 {
            let data: u32 = m.read(set.wrapping_add(4));
            m.read(data)
        } else {
            m.read(set.wrapping_add(4))
        }
    }

    /// The record re-bind both `0x00543c60` and `0x00543e50` run on record
    /// `rec`: its controller (`+0x24`) set as its time source
    /// ([`RECORD_SET_TIME`], `+0x14` = the controller first); when that
    /// succeeds and its clip set (`+0x40`) changes to `set`: the set's
    /// first clip bound ([`RECORD_BIND`]) unless there is no set, and
    /// `+0` cleared. Returns whether it got to the clip set.
    fn rebind_record(h: &mut impl Host, rec: u32, set: u32) -> bool {
        let ctl = rec.wrapping_add(0x24);
        h.write(rec.wrapping_add(0x14), ctl);
        let r = h.call_sym(
            RECORD_SET_TIME,
            Conv::Thiscall(rec.wrapping_add(0x38)),
            &[ctl],
        );
        if r != 0 {
            return false;
        }
        let old: u32 = h.read(rec.wrapping_add(0x40));
        h.write(rec.wrapping_add(0x40), set);
        if old == set {
            return false;
        }
        if set != 0 {
            let clip = first_clip(h, set);
            h.call_sym(RECORD_BIND, Conv::Thiscall(rec.wrapping_add(0x38)), &[clip]);
        }
        h.write(rec, 0u32);
        true
    }

    /// `0x00543c60` (`thiscall(set)`, `ret 4`): the component's clip set
    /// changed: 7 for an empty set; the data cleared ([`DATA_CLEAR`] on
    /// `+0x48`), `+0x10`, `+0x24`, `+0x40` = `set`, the channel targets
    /// bound ([`CLIP_SET_BIND`], its error returned); each source record
    /// re-bound, and with a set the blender gets each source (weight 0
    /// first for records left without a clip; the source set's error
    /// returned); then with a set [`COMPONENT_REFRESH`]'s `eax`, else
    /// `+0x40` cleared and 0.
    pub fn set_clip_set(h: &mut impl Host, this: u32, set: u32) -> u32 {
        if set != 0 && count(h, set) < 1 {
            return 7;
        }
        h.call_sym(DATA_CLEAR, Conv::Thiscall(this.wrapping_add(0x48)), &[]);
        h.write(this.wrapping_add(0x10), set);
        h.write(this.wrapping_add(0x24), set);
        h.write(this.wrapping_add(0x40), set);
        let blender = this.wrapping_add(0x1c);
        if set != 0 {
            let r = h.call_sym(
                CLIP_SET_BIND,
                Conv::Thiscall(this.wrapping_add(0x38)),
                &[blender],
            );
            if r != 0 {
                return r;
            }
        }
        let n = count(h, this.wrapping_add(0x14));
        for i in 0..n {
            let recs: u32 = h.read(this.wrapping_add(0x18));
            let rec = recs.wrapping_add(i.wrapping_mul(0x4c));
            rebind_record(h, rec, set);
            if h.read::<u32>(this.wrapping_add(0x10)) == 0 {
                continue;
            }
            if h.read::<u32>(rec) == 0 {
                h.call_sym(BLENDER_SET_WEIGHT, Conv::Thiscall(blender), &[i, 0]);
            }
            let r = h.call_sym(
                BLENDER_SET_SOURCE,
                Conv::Thiscall(blender),
                &[i, rec.wrapping_add(0x38)],
            );
            if r != 0 {
                return r;
            }
        }
        if set != 0 {
            return h.call_sym(COMPONENT_REFRESH, Conv::Thiscall(this), &[]);
        }
        h.write(this.wrapping_add(0x40), 0u32);
        0
    }

    /// `0x00543e50` (`thiscall(n)`, `ret 4`): the component's source
    /// count set to `n`: the blender's resize (its slot 12, `(n)`) and the
    /// records' ([`RECORDS_RESIZE`]), their errors returned; new sources
    /// get weight 1.0 (`+4` and the blender); every record re-bound to the
    /// component's clip set (`+0x10`) and, with a set, handed to the
    /// blender (its error returned); a changed count with a set refreshes
    /// the component; 0.
    pub fn set_source_count(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let old = count(h, this.wrapping_add(0x14));
        let blender = this.wrapping_add(0x1c);
        let r = h.call_virtual(blender, 12, &[n]);
        if r != 0 {
            return r;
        }
        let r = h.call_sym(
            RECORDS_RESIZE,
            Conv::Thiscall(this.wrapping_add(0x14)),
            &[n],
        );
        if r != 0 {
            return r;
        }
        let one = 1.0f32.to_bits();
        for i in old..n {
            if i < count(h, this.wrapping_add(0x14)) {
                let recs: u32 = h.read(this.wrapping_add(0x18));
                h.write(recs.wrapping_add(i.wrapping_mul(0x4c)).wrapping_add(4), one);
                h.call_sym(BLENDER_SET_WEIGHT, Conv::Thiscall(blender), &[i, one]);
            }
        }
        for i in 0..n {
            let recs: u32 = h.read(this.wrapping_add(0x18));
            let rec = recs.wrapping_add(i.wrapping_mul(0x4c));
            let set: u32 = h.read(this.wrapping_add(0x10));
            rebind_record(h, rec, set);
            if h.read::<u32>(this.wrapping_add(0x10)) != 0 {
                let r = h.call_sym(
                    BLENDER_SET_SOURCE,
                    Conv::Thiscall(blender),
                    &[i, rec.wrapping_add(0x38)],
                );
                if r != 0 {
                    return r;
                }
            }
        }
        if h.read::<u32>(this.wrapping_add(0x10)) != 0 && old != n {
            h.call_sym(COMPONENT_REFRESH, Conv::Thiscall(this), &[]);
        }
        0
    }
}
