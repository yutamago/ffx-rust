//! M5 stage 1, list `phyre-core`, batch 2: the containers.
//!
//! A `PArray<T>` is a count word (bit 31: the storage is external, not
//! the array's own) and a data pointer; a `PSharray<T>` keeps a single
//! element in the data word itself (`+4`). Most functions here are
//! copies of one template:
//!
//! - `resize(n)` ([`RESIZE`]): the same count does nothing; else a block
//!   from a reallocator `(n, data, keep)` or from `PhyreMalloc(n * size, 4)`
//!   (then the kept elements copied and the rest constructed by the
//!   element type's helpers), 0xd when that fails for `n` > 0, the old
//!   storage released unless external (`PhyreFree`, with or without a
//!   null test, or the type's destroy `(data, count)`), then data and
//!   count stored (the count without the external flag);
//! - the element helpers: copy-constructions over `n` elements that skip a
//!   null slot without moving the source (the placement-new test),
//!   default constructions, the `PSharray` copy constructor;
//! - `PSharray<PString>::setToExternalBuffer`, the `PThreadPool`
//!   destructor body, `PStringBuilder`'s constructor and `+= int`.

/// `Phyre::PhyreFree(p)` and `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The CRT's `memcpy` thunk and `sprintf_s`'s import slot (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
pub const SPRINTF_S: u32 = 0x00b0_c394;
/// `PStringBuilder += int`'s format (`"%d"`) and buffer size.
pub const INT_FORMAT: u32 = 0x00b0_f604;
pub const INT_BUFFER: u32 = 0x32;
/// `Phyre_PStringBuilder_append_00444ce0(builder; s, n)` and its reserve
/// `0x00444e10(builder; n)` (thiscall).
pub const BUILDER_APPEND: u32 = 0x0044_4ce0;
pub const BUILDER_RESERVE: u32 = 0x0044_4e10;
/// `Phyre_PString_copy_0044ae10(this; &src)` and
/// `PArray_releaseTaggedWords_00434360(data, count)`.
pub const STRING_COPY: u32 = 0x0044_ae10;
pub const RELEASE_TAGGED_WORDS: u32 = 0x0043_4360;
/// The object `0x0044ca80` asks for a validating descriptor (vtable
/// `+4`, thiscall `()`), and that descriptor's
/// `Phyre_PClassDescriptor_applyDefaultBufferWithValidation(d; p, 1)`.
pub const STRING_SHARRAY_TYPE: u32 = 0x00c9_1048;
pub const APPLY_DEFAULT_BUFFER: u32 = 0x0043_c160;
/// `PSharray<PString>::setToExternalBuffer` (thiscall `(this; data, n)`).
pub const STRING_SHARRAY_SET: u32 = 0x0044_c960;
/// `Phyre_releaseThreeArrays_00479930(this)` (thiscall).
pub const RELEASE_THREE: u32 = 0x0047_9930;
/// `Phyre_Ctor_0055dc40(this)` (thiscall), a 64-byte element's.
pub const CTOR_55DC40: u32 = 0x0055_dc40;
/// The thread pool's members: the join `0x004301b0(thread; &code)`, the
/// close `0x00430260(thread)` (thiscall), `eh vector destructor
/// iterator(p, size, n, dtor)` (stdcall) and the thread record's
/// destructor `0x0044cc70`.
pub const THREAD_JOIN: u32 = 0x0043_01b0;
pub const THREAD_CLOSE: u32 = 0x0043_0260;
pub const VECTOR_DTOR: u32 = 0x0094_96d2;
pub const THREAD_RECORD_DTOR: u32 = 0x0044_cc70;
/// `PMatrix4_toWorldMatrix3x4(dst; m)` (thiscall).
pub const TO_WORLD_MATRIX: u32 = 0x0045_1190;
/// The mesh instance segment context's `+0x1c` copy `0x004f2740(dst;
/// &src)` (thiscall).
pub const CONTEXT_TAIL_COPY: u32 = 0x004f_2740;
/// The LOD level's `+4` copy `0x005a2380(dst; &src)`, its `PSharray`
/// copy constructor `0x005a2440(dst; &src)` (thiscall), the default
/// level's `+0x10` float and the levels' storage setter `0x005a7d90(this;
/// block, n)` (thiscall).
pub const LOD_LEVEL_4_COPY: u32 = 0x005a_2380;
pub const LOD_SHARRAY_COPY: u32 = 0x005a_2440;
pub const LOD_LEVEL_FLOAT: u32 = 0x00b1_0f4c;
pub const LOD_LEVELS_SET: u32 = 0x005a_7d90;
pub const LOD_LEVELS_COPY: u32 = 0x005a_0f80;
pub const LOD_LEVELS_DEFAULT: u32 = 0x005a_6280;
/// `Phyre_uninitializedCopy12_005a1030(dst, src, n)` (cdecl).
pub const COPY12: u32 = 0x005a_1030;
/// `PSharray<T *>`'s reallocator `0x0048bbc0(this; n, old, keep)` (thiscall, `ret 0xc`; the inline word for up to one element).
pub const SHARRAY_REALLOC: u32 = 0x0048_bbc0;

/// Where `resize` gets its block.
#[derive(Clone, Copy, Debug)]
pub enum Alloc {
    /// A reallocator `(n, data, keep)` (cdecl).
    Realloc(u32),
    /// `PhyreMalloc(n * stride, 4)`, then `copy(block, data, keep)` and
    /// `construct(block + keep * stride, n - keep)` (cdecl; 0: none).
    Malloc {
        stride: u32,
        copy: u32,
        construct: u32,
    },
}

/// How `resize` releases the old storage (unless external).
#[derive(Clone, Copy, Debug)]
pub enum Release {
    /// `PhyreFree(data)`, skipped for null `data` when `null_check`.
    Free { null_check: bool },
    /// The element type's destroy `(data, count)` (cdecl).
    Destroy(u32),
}

/// One `resize` copy.
#[derive(Clone, Copy, Debug)]
pub struct Resize {
    pub alloc: Alloc,
    pub release: Release,
}

const fn realloc(f: u32, null_check: bool) -> Resize {
    Resize {
        alloc: Alloc::Realloc(f),
        release: Release::Free { null_check },
    }
}

const fn malloc(stride: u32, copy: u32, construct: u32, release: Release) -> Resize {
    Resize {
        alloc: Alloc::Malloc {
            stride,
            copy,
            construct,
        },
        release,
    }
}

const FREE_CHECKED: Release = Release::Free { null_check: true };

/// The `PArray<T>::resize` copies of this list (thiscall `(this; n)`,
/// `ret 4`).
pub const RESIZE: &[(u32, Resize)] = &[
    (0x0044_a080, realloc(0x0044_7d50, true)),
    (0x0047_ef50, malloc(4, 0, 0, FREE_CHECKED)),
    (0x0048_9740, realloc(0x0048_7920, true)),
    (
        0x0049_9030,
        malloc(16, 0x0049_5bc0, 0x0049_7db0, FREE_CHECKED),
    ),
    (
        0x004b_dc40,
        malloc(
            0x254,
            0x004b_aed0,
            0x004b_d1e0,
            Release::Destroy(0x004b_c850),
        ),
    ),
    (
        0x004f_8980,
        malloc(
            0x24,
            0x004f_0950,
            0x004f_6380,
            Release::Destroy(0x004f_4fc0),
        ),
    ),
    (0x0050_ae70, malloc(4, 0x0050_8f00, 0, FREE_CHECKED)),
    (0x0052_a860, realloc(0x0052_8270, true)),
    (
        0x0059_93a0,
        malloc(
            0x14,
            0x0057_e190,
            0x0058_afd0,
            Release::Destroy(0x0058_8190),
        ),
    ),
    (0x005b_3040, realloc(0x005b_1320, false)),
    (
        0x00a2_d9e0,
        malloc(0x30, 0x00a2_adc0, 0x00a2_d1a0, FREE_CHECKED),
    ),
];

/// The [`RESIZE`] entry of copy `f`.
#[must_use]
pub const fn resize_of(f: u32) -> Resize {
    let mut i = 0;
    while i < RESIZE.len() {
        if RESIZE[i].0 == f {
            return RESIZE[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::strlen;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn thiscall(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn free(h: &mut impl Host, p: u32) -> u32 {
        cdecl(h, PHYRE_FREE, &[p])
    }

    /// The smaller of two counts.
    const fn keep(n: u32, cap: u32) -> u32 {
        if n < cap { n } else { cap }
    }

    /// A `PSharray`'s storage for count word `word`: the data pointer for
    /// more than one element, the inline word for one, 0 for none.
    fn sharray_storage(h: &impl Host, this: u32, word: u32) -> u32 {
        match word & 0x7fff_ffff {
            0 => 0,
            1 => this + 4,
            _ => h.read(this + 4),
        }
    }

    /// `PArray<T>::resize(this; n)` -> 0, or 0xd when no block came for
    /// `n` > 0 (the array unchanged).
    pub fn resize(h: &mut impl Host, this: u32, n: u32, r: Resize) -> u32 {
        let word: u32 = h.read(this);
        if n == word {
            return 0;
        }
        let kept = keep(n, word & 0x7fff_ffff);
        let block = match r.alloc {
            Alloc::Realloc(f) => {
                let data: u32 = h.read(this + 4);
                cdecl(h, f, &[n, data, kept])
            }
            Alloc::Malloc {
                stride,
                copy,
                construct,
            } => {
                let old: u32 = h.read(this + 4);
                if n == 0 {
                    0
                } else {
                    let block = cdecl(h, PHYRE_MALLOC, &[n.wrapping_mul(stride), 4]);
                    if block != 0 {
                        if copy != 0 {
                            cdecl(h, copy, &[block, old, kept]);
                        }
                        if construct != 0 {
                            let rest = block.wrapping_add(kept.wrapping_mul(stride));
                            cdecl(h, construct, &[rest, n - kept]);
                        }
                    }
                    block
                }
            }
        };
        if block == 0 && n != 0 {
            return 0xd;
        }
        let data: u32 = h.read(this + 4);
        if data != block {
            let count: u32 = h.read(this);
            if count >> 31 == 0 {
                match r.release {
                    Release::Free { null_check } => {
                        if !null_check || data != 0 {
                            free(h, data);
                        }
                    }
                    Release::Destroy(f) => {
                        cdecl(h, f, &[data, count & 0x7fff_ffff]);
                    }
                }
            }
        }
        h.write(this + 4, block);
        h.write(this, n);
        0
    }

    /// `PSharray<T *>::resize(this; n)` (`0x0048c470`) -> 0 or 0xd: the
    /// reallocator `(n, storage, keep)`; the old storage freed unless
    /// inline or external; a block other than the inline word stored.
    pub fn sharray_resize(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let word: u32 = h.read(this);
        if n == word {
            return 0;
        }
        let old = sharray_storage(h, this, word);
        let kept = keep(n, word & 0x7fff_ffff);
        let block = thiscall(h, SHARRAY_REALLOC, this, &[n, old, kept]);
        if block == 0 && n != 0 {
            return 0xd;
        }
        if old != block && old != this + 4 && h.read::<u32>(this) >> 31 == 0 {
            free(h, old);
        }
        if block != this + 4 {
            h.write(this + 4, block);
        }
        h.write(this, n);
        0
    }

    /// `PArray<PLODLevel>::resize(this; n)` (`0x005a7b60`) -> 0 or 0xd:
    /// `PhyreMalloc(n * 0x24, 4)`, the kept levels copied, the rest
    /// default-constructed, then the storage setter `0x005a7d90(this;
    /// block, n)`.
    pub fn lod_levels_resize(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let word: u32 = h.read(this);
        if n == word {
            return 0;
        }
        let kept = keep(n, word & 0x7fff_ffff);
        let old: u32 = h.read(this + 4);
        let mut block = 0;
        if n != 0 {
            block = cdecl(h, PHYRE_MALLOC, &[n.wrapping_mul(0x24), 4]);
            if block == 0 {
                return 0xd;
            }
            cdecl(h, LOD_LEVELS_COPY, &[block, old, kept]);
            let rest = block.wrapping_add(kept.wrapping_mul(0x24));
            cdecl(h, LOD_LEVELS_DEFAULT, &[rest, n - kept]);
        }
        thiscall(h, LOD_LEVELS_SET, this, &[block, n]);
        0
    }

    /// `PStreamInputDescD3D11`'s copy over `n` 20-byte elements
    /// (`0x0057e190(dst, src, n)`): the name (`+0`, tag bit 0 dropped) as
    /// a fresh `PhyreMalloc(len + 1, 2)` copy (0 when none or when that
    /// fails), four words after it.
    pub fn copy_stream_descs(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let mut s = src;
        for k in 0..n {
            let d = dst.wrapping_add(k.wrapping_mul(0x14));
            if d == 0 {
                continue;
            }
            let tagged: u32 = h.read(s);
            let text = tagged - (tagged & 1);
            let copy = if text == 0 {
                0
            } else {
                let len = strlen(h, text).wrapping_add(1);
                let p = cdecl(h, PHYRE_MALLOC, &[len, 2]);
                if p != 0 {
                    cdecl(h, MEMCPY, &[p, text, len]);
                }
                p
            };
            h.write(d, copy);
            for w in 1..5 {
                let v: u32 = h.read(s.wrapping_add(4 * w));
                h.write(d.wrapping_add(4 * w), v);
            }
            s = s.wrapping_add(0x14);
        }
    }

    /// `0x0058afd0(dst, n)`: `n` 20-byte elements zeroed (a null slot
    /// skipped).
    pub fn zero20(h: &mut impl Host, dst: u32, n: u32) {
        for k in 0..n {
            let d = dst.wrapping_add(k.wrapping_mul(0x14));
            if d != 0 {
                for w in 0..5 {
                    h.write(d.wrapping_add(4 * w), 0u32);
                }
            }
        }
    }

    /// `PMeshInstanceSegmentContext`'s copy over `n` 0x24-byte elements
    /// (`0x004f0950(dst, src, n)`): seven words, then the `+0x1c` member's
    /// copy `0x004f2740(dst + 0x1c; src + 0x1c)`.
    pub fn copy_segment_contexts(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let mut s = src;
        for k in 0..n {
            let d = dst.wrapping_add(k.wrapping_mul(0x24));
            if d == 0 {
                continue;
            }
            for w in 0..7 {
                let v: u32 = h.read(s.wrapping_add(4 * w));
                h.write(d.wrapping_add(4 * w), v);
            }
            thiscall(
                h,
                CONTEXT_TAIL_COPY,
                d.wrapping_add(0x1c),
                &[s.wrapping_add(0x1c)],
            );
            s = s.wrapping_add(0x24);
        }
    }

    /// `PLODLevel`'s copy over `n` 0x24-byte elements (`0x005a0f80(dst,
    /// src, n)`): the word, the `+4` member (`0x005a2380`), two floats,
    /// the `+0x14` `PSharray` (`0x005a2440`).
    pub fn copy_lod_levels(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let mut s = src;
        for k in 0..n {
            let d = dst.wrapping_add(k.wrapping_mul(0x24));
            if d == 0 {
                continue;
            }
            let v: u32 = h.read(s);
            h.write(d, v);
            thiscall(h, LOD_LEVEL_4_COPY, d.wrapping_add(4), &[s.wrapping_add(4)]);
            for off in [0xc, 0x10] {
                let f: u32 = h.read(s.wrapping_add(off));
                h.write(d.wrapping_add(off), x87::store(x87::load(f)));
            }
            thiscall(
                h,
                LOD_SHARRAY_COPY,
                d.wrapping_add(0x14),
                &[s.wrapping_add(0x14)],
            );
            s = s.wrapping_add(0x24);
        }
    }

    /// `PLODLevel`'s default construction over `n` 0x24-byte elements
    /// (`0x005a6280(dst, n)`): three zero words, 0.0 and the image's
    /// float at `+0xc`/`+0x10`, then the `+0x14` `PSharray` with its one
    /// inline element (count 1, 1.0, 0.0, 4).
    pub fn default_lod_levels(h: &mut impl Host, dst: u32, n: u32) {
        let k: u32 = h.read(h.rebase(LOD_LEVEL_FLOAT));
        let k = x87::store(x87::load(k));
        let one = 1.0f32.to_bits();
        for i in 0..n {
            let e = dst.wrapping_add(i.wrapping_mul(0x24));
            if e == 0 {
                continue;
            }
            h.write(e, 0u32);
            h.write(e.wrapping_add(4), 0u32);
            h.write(e.wrapping_add(8), 0u32);
            h.write(e.wrapping_add(0xc), 0u32);
            h.write(e.wrapping_add(0x10), k);
            h.write(e.wrapping_add(0x14), 0u32);
            if e.wrapping_add(0x18) != 0 {
                h.write(e.wrapping_add(0x20), 4u32);
                h.write(e.wrapping_add(0x18), one);
                h.write(e.wrapping_add(0x1c), 0u32);
                h.write(e.wrapping_add(0x14), 1u32);
            }
        }
    }

    /// `PSharray<12-byte>`'s copy constructor `0x005a2440(this; &src)` ->
    /// `this`: the count 0, the source's elements copied into a fresh
    /// block (`PhyreMalloc(n * 12, 4)`) or the inline word, the (now
    /// empty) old storage released, count and storage stored.
    pub fn sharray12_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
        h.write(this, 0u32);
        let sw: u32 = h.read(src);
        let n = sw & 0x7fff_ffff;
        let from = sharray_storage(h, src, sw);
        let inline = this + 4;
        let block = if n > 1 {
            cdecl(h, PHYRE_MALLOC, &[n.wrapping_mul(12), 4])
        } else {
            inline
        };
        if block != 0 {
            cdecl(h, COPY12, &[block, from, n]);
        }
        let word: u32 = h.read(this);
        let old = sharray_storage(h, this, word);
        if old != block && old != inline && word >> 31 == 0 && old != 0 {
            free(h, old);
        }
        h.write(this, n);
        if block != inline {
            h.write(inline, block);
        }
        this
    }

    /// `PSharray<PString>::setToExternalBuffer(this; data, n)`
    /// (`0x0044c960`) -> 0, or 5 for a negative `n`. One element is
    /// copied into the inline word (`PString` copy, after freeing an
    /// inline string the array owned) and the count is 1 without the
    /// external flag; otherwise `data` becomes the storage, flagged
    /// external. The old storage goes unless it is the new one: an inline
    /// string freed unless borrowed, owned heap strings through
    /// `PArray_releaseTaggedWords`.
    pub fn string_sharray_set(h: &mut impl Host, this: u32, data: u32, n: u32) -> u32 {
        if (n as i32) < 0 {
            return 5;
        }
        let count = h.read::<u32>(this) & 0x7fff_ffff;
        let old = match count {
            0 => 0,
            1 => this + 4,
            _ => h.read(this + 4),
        };
        let inline = this + 4;
        let (store, word) = if n == 1 {
            if count == n {
                let s: u32 = h.read(inline);
                if s & 1 == 0 {
                    free(h, s);
                }
            }
            thiscall(h, STRING_COPY, inline, &[data]);
            (inline, 1)
        } else {
            (data, n | 0x8000_0000)
        };
        if old != store {
            if old == inline {
                let s: u32 = h.read(old);
                if s & 1 == 0 {
                    free(h, s);
                }
            } else if h.read::<u32>(this) >> 31 == 0 {
                cdecl(h, RELEASE_TAGGED_WORDS, &[old, count]);
            }
        }
        if store != inline {
            h.write(inline, store);
        }
        h.write(this, word);
        0
    }

    /// `0x0044ca80(this; data, n)` -> 0: a single element given as the
    /// array's own inline word goes through the validating descriptor
    /// (`[0x00c91048]`'s slot 1, then `applyDefaultBufferWithValidation(d;
    /// data, 1)`); anything else, or no descriptor, to
    /// `PSharray<PString>::setToExternalBuffer`.
    pub fn string_sharray_assign(h: &mut impl Host, this: u32, data: u32, n: u32) -> u32 {
        let g = h.rebase(STRING_SHARRAY_TYPE);
        let vt: u32 = h.read(g);
        let f: u32 = h.read(vt.wrapping_add(4));
        let d = h.call(f, Conv::Thiscall(g), &[]);
        if data == this + 4 && n == 1 && d != 0 {
            thiscall(h, APPLY_DEFAULT_BUFFER, d, &[this + 4, 1]);
        } else {
            thiscall(h, STRING_SHARRAY_SET, this, &[data, n]);
        }
        0
    }

    /// The owned-array reset of `+0` count / `+4` data (`eax` as the
    /// originals leave it: `PhyreFree`'s, 0, or the flag test's 0xfe).
    fn reset(h: &mut impl Host, at: u32) -> u32 {
        let count: u32 = h.read(at);
        let mut eax = (count >> 31) ^ 0xff;
        if count >> 31 == 0 {
            let data: u32 = h.read(at + 4);
            eax = data;
            if data != 0 {
                eax = free(h, data);
            }
        }
        h.write(at + 4, 0u32);
        h.write(at, 0u32);
        eax
    }

    /// `0x00431bf0(this)` -> `eax`: the destructor of a record with an
    /// array at `+4`, three at `+0xc` (`Phyre_releaseThreeArrays`) and one
    /// at `+0x30` (freed unless external, no null test).
    pub fn record_dtor(h: &mut impl Host, this: u32) -> u32 {
        let inner = this + 0xc;
        let count: u32 = h.read(inner + 0x24);
        if count >> 31 == 0 {
            let data: u32 = h.read(inner + 0x28);
            free(h, data);
        }
        h.write(inner + 0x28, 0u32);
        h.write(inner + 0x24, 0u32);
        thiscall(h, RELEASE_THREE, inner, &[]);
        reset(h, this + 4)
    }

    /// `0x004343c0(p, n)`: `Phyre_Ctor_0055dc40` on `n` 64-byte elements
    /// (a null slot skipped).
    pub fn construct64(h: &mut impl Host, p: u32, n: u32) {
        for i in 0..n {
            let e = p.wrapping_add(i << 6);
            if e != 0 {
                thiscall(h, CTOR_55DC40, e, &[]);
            }
        }
    }

    /// `PStringBuilder(this; s)` (`0x004449e0`) -> `this`: empty, then for
    /// a text its length with the terminator, room for `+ 0x80`, the text
    /// copied when the buffer came.
    pub fn builder_new(h: &mut impl Host, this: u32, s: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        if s == 0 {
            h.write(this + 8, 0u32);
            return this;
        }
        let n = strlen(h, s).wrapping_add(1);
        thiscall(h, BUILDER_RESERVE, this, &[n.wrapping_add(0x80)]);
        let p: u32 = h.read(this + 4);
        h.write(this + 8, n);
        if p != 0 {
            cdecl(h, MEMCPY, &[p, s, n]);
        }
        this
    }

    /// `PStringBuilder += int` (`0x00444b90(this; v)`) -> the append's
    /// `eax`: `sprintf_s(buffer, 0x32, "%d", v)` into the frame at `buf`
    /// (0x34 bytes), then `append(buffer, strlen)`.
    pub fn builder_append_int(h: &mut impl Host, this: u32, v: u32, buf: u32) -> u32 {
        let f: u32 = h.read(h.rebase(SPRINTF_S));
        let fmt = h.rebase(INT_FORMAT);
        h.call(f, Conv::Cdecl, &[buf, INT_BUFFER, fmt, v]);
        let n = strlen(h, buf);
        thiscall(h, BUILDER_APPEND, this, &[buf, n])
    }

    /// `PThreadPool`'s destructor body `0x0044d070(this)` -> the last
    /// `PhyreFree`'s `eax`: with thread records (`+4`, `+8` of them,
    /// 0x18 bytes each) each thread is joined (`&code` in the frame word
    /// `code`) and closed and its flag `+0xc` cleared; the records are
    /// destroyed and freed, then the buffers `+0x18` and `+0xc`.
    pub fn pool_dtor_body(h: &mut impl Host, this: u32, code: u32) -> u32 {
        if h.read::<u32>(this + 4) != 0 {
            let mut i = 0u32;
            while i < h.read::<u32>(this + 8) {
                let rec = h.read::<u32>(this + 4).wrapping_add(i.wrapping_mul(0x18));
                h.write(code, 0u32);
                thiscall(h, THREAD_JOIN, rec.wrapping_add(8), &[code]);
                thiscall(h, THREAD_CLOSE, rec.wrapping_add(8), &[]);
                h.write(rec.wrapping_add(0xc), 0u8);
                i += 1;
            }
        }
        let p: u32 = h.read(this + 4);
        if p != 0 {
            let block = p.wrapping_sub(4);
            let n: u32 = h.read(block);
            let dtor = h.rebase(THREAD_RECORD_DTOR);
            h.call_sym(VECTOR_DTOR, Conv::Stdcall, &[p, 0x18, n, dtor]);
            free(h, block);
        }
        let a: u32 = h.read(this + 0x18);
        free(h, a);
        let b: u32 = h.read(this + 0xc);
        let eax = free(h, b);
        h.write(this + 4, 0u32);
        h.write(this + 0x18, 0u32);
        h.write(this + 0xc, 0u32);
        eax
    }

    /// `Matrix3x4_Identity_00450ee0(m)` -> `m`: the 4x4 identity in the
    /// frame at `frame` (16 floats), then `PMatrix4_toWorldMatrix3x4(m;
    /// frame)`.
    pub fn identity_3x4(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        for i in 0..16 {
            let v = if i % 5 == 0 { 1.0f32 } else { 0.0 };
            h.write(frame + 4 * i, v.to_bits());
        }
        thiscall(h, TO_WORLD_MATRIX, this, &[frame]);
        this
    }
}
