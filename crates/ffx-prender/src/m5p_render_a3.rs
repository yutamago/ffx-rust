//! M5 stage 1, `PhyreEngine` list `render-a`, batch 3: the mesh classes'
//! copy constructors and element loops (`PMeshSegment`, `PMesh`, the
//! shader parameter records), the `PSharray<T>` storage of the D3D11
//! buffers (resize, external buffer, the member descriptor's setter),
//! array truncation and `PTexture3DBase::setDimensions`.

use ffx_graphic::host::{Conv, Host};

use crate::m5p_render_a::{PHYRE_FREE, PHYRE_MALLOC_ALIGNED};

/// The CRT's `memcpy(dst, src, n)` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;

/// The element loops `(dst, src, count)` (cdecl) that copy-construct each
/// element with `ctor(dst; src)` (thiscall): (address, element size,
/// ctor).
pub const COPY_CONSTRUCTS: &[(u32, u32, u32)] = &[
    (0x0048_c960, 0x6c, 0x0048_f0d0),
    (0x004b_aed0, 0x254, 0x004b_bcb0),
];

/// The `PSharray<T>` resizes of 20-byte elements `(this; n)` (`ret 4`):
/// (address, the copy `(dst, src, count)`, the zeroing `(dst, count)`).
pub const SHARRAY_RESIZES: &[(u32, u32, u32)] = &[
    (0x0056_3df0, 0x0048_0b60, 0x0056_0df0),
    (0x0056_3ed0, 0x0048_4a30, 0x0056_0e40),
];

/// The `PSharray<T>::setToExternalBuffer` copies of 20-byte elements.
pub const SHARRAY_EXTERNALS: &[u32] = &[0x0056_4160, 0x0056_4210];

/// The member descriptors' external-buffer setters `(this; buffer,
/// count)`: (address, the class object whose virtual 1 gives the
/// relocator, the sharray setter).
pub const MEMBER_EXTERNALS: &[(u32, u32, u32)] = &[
    (0x0056_42e0, 0x00cb_11b8, 0x0056_4160),
    (0x0056_4330, 0x00cb_1088, 0x0056_4210),
];

/// The record copies with a deep-copied `PString` at `+4` (16-byte
/// records; the other 12 bytes copied as they are).
pub const STRING_RECORD_COPIES: &[u32] = &[0x004a_3530, 0x004a_3620];

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn copy_words(h: &mut impl Host, dst: u32, src: u32, offs: &[u32]) {
        for &o in offs {
            let v: u32 = h.read(src + o);
            h.write(dst + o, v);
        }
    }

    fn copy_bytes(h: &mut impl Host, dst: u32, src: u32, offs: &[u32]) {
        for &o in offs {
            let v: u8 = h.read(src + o);
            h.write(dst + o, v);
        }
    }

    /// `FUN_00433c80(this)`: the sprite collection's destructor body: the
    /// members at `+0x30` (`0x0055e060`) and `+0x28` (`0x00433900`), then
    /// the owned array at `+8` (`reset_owned_nonnull`), whose `eax` it
    /// returns.
    pub fn sprite_collection_dtor(h: &mut impl Host, this: u32) -> u32 {
        this_call(h, 0x0055_e060, this + 0x30, &[]);
        this_call(h, 0x0043_3900, this + 0x28, &[]);
        crate::m5p_render_a::bridge::reset_owned_nonnull(h, this, 8)
    }

    /// An array copy: `count` elements of `size` bytes from `data` into a
    /// fresh block (alignment 4) by `copy(block, data, count)`; the
    /// block (0 for no elements or no memory) and the count to store (0
    /// without a block).
    fn copy_array(h: &mut impl Host, count: u32, data: u32, size: u32, copy: u32) -> (u32, u32) {
        if count == 0 {
            return (0, 0);
        }
        let block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[count.wrapping_mul(size), 4]);
        if block == 0 {
            return (0, 0);
        }
        cdecl(h, copy, &[block, data, count]);
        (block, count)
    }

    /// `FUN_004870b0(this; src)` (`ret 4`) -> this: `PMeshSegmentBase`'s
    /// copy: two words, the array of 12-byte elements at `+8` copied
    /// (`PhyreCopyElements12_00480bb0`), the word `+0x10` and the bytes
    /// `+0x14`, `+0x15`.
    pub fn segment_base_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
        copy_words(h, this, src, &[0, 4]);
        h.write(this + 8, 0u32);
        let count = h.read::<u32>(src + 8) & 0x7fff_ffff;
        let data: u32 = h.read(src + 0xc);
        let (block, n) = copy_array(h, count, data, 12, 0x0048_0bb0);
        h.write(this + 0xc, block);
        h.write(this + 8, n);
        copy_words(h, this, src, &[0x10]);
        copy_bytes(h, this, src, &[0x14, 0x15]);
        this
    }

    /// `FUN_00486120(dst, src, count)` (cdecl): 64-byte `PMeshSegment`s
    /// copy-constructed (`FUN_004870b0` and `0x00486ff0` on `+0x18`, four
    /// words at `+0x30`); a destination at address 0 skipped.
    pub fn segments_copy(h: &mut impl Host, dst: u32, src: u32, count: u32) {
        let mut s = src;
        for i in 0..count {
            let d = dst.wrapping_add(64 * i);
            if d != 0 {
                this_call(h, 0x0048_70b0, d, &[s]);
                this_call(h, 0x0048_6ff0, d + 0x18, &[s + 0x18]);
                copy_words(h, d, s, &[0x30, 0x34, 0x38, 0x3c]);
                s = s.wrapping_add(64);
            }
        }
    }

    /// A [`COPY_CONSTRUCTS`] loop `(dst, src, count)`.
    pub fn copy_construct(h: &mut impl Host, dst: u32, src: u32, count: u32, size: u32, ctor: u32) {
        let mut s = src;
        for i in 0..count {
            let d = dst.wrapping_add(size.wrapping_mul(i));
            if d != 0 {
                this_call(h, ctor, d, &[s]);
                s = s.wrapping_add(size);
            }
        }
    }

    /// `FUN_004bd1e0(dst, count)` (cdecl): `PShaderPassD3D11_ctor` on each
    /// 0x254-byte element, one at address 0 skipped.
    pub fn shader_pass_construct(h: &mut impl Host, dst: u32, count: u32) {
        for i in 0..count {
            let d = dst.wrapping_add(0x254u32.wrapping_mul(i));
            if d != 0 {
                this_call(h, 0x0058_3c00, d, &[]);
            }
        }
    }

    /// `FUN_0048bbc0(this; n, src, count)` (`ret 0xc`) -> the storage:
    /// `n` words (`PhyreMallocAligned(4n, 4)`, or the inline word at `+4`
    /// for at most one), `count` copied from `src` (a slot at address 0
    /// skipped), the rest zeroed; 0 without memory.
    pub fn sharray_words_alloc(h: &mut impl Host, this: u32, n: u32, src: u32, count: u32) -> u32 {
        let block = if n > 1 {
            cdecl(h, PHYRE_MALLOC_ALIGNED, &[n << 2, 4])
        } else {
            this + 4
        };
        if block == 0 {
            return 0;
        }
        let mut s = src;
        for i in 0..count {
            let d = block.wrapping_add(4 * i);
            if d != 0 {
                let v: u32 = h.read(s);
                h.write(d, v);
                s = s.wrapping_add(4);
            }
        }
        for i in count..n {
            h.write(block.wrapping_add(4 * i), 0u32);
        }
        block
    }

    /// The first element of a `PSharray` header at `a`: the block for
    /// more than one, the inline element at `+4` for one, else 0.
    fn sharray_data(h: &mut impl Host, a: u32) -> u32 {
        let count = h.read::<u32>(a) & 0x7fff_ffff;
        if count > 1 {
            h.read(a + 4)
        } else if count != 0 {
            a + 4
        } else {
            0
        }
    }

    /// Frees a sharray's old block when it is neither the new one nor the
    /// inline element, owned and not null.
    fn sharray_free_old(h: &mut impl Host, this: u32, old: u32, block: u32) {
        let header: u32 = h.read(this);
        if old != block && old != this + 4 && header & 0x8000_0000 == 0 && old != 0 {
            cdecl(h, PHYRE_FREE, &[old]);
        }
    }

    /// `FUN_0048eed0(this; src)` (`ret 4`) -> this: the copy of a
    /// `PSharray` of 20-byte elements (a block for more than one, the
    /// inline element otherwise), copied by `0x00484a30`. The header is
    /// zeroed first, so the old-block free never runs.
    pub fn sharray20_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
        h.write(this, 0u32);
        let count = h.read::<u32>(src) & 0x7fff_ffff;
        let data = sharray_data(h, src);
        let block = if count > 1 {
            cdecl(h, PHYRE_MALLOC_ALIGNED, &[count.wrapping_mul(20), 4])
        } else {
            this + 4
        };
        if block != 0 {
            cdecl(h, 0x0048_4a30, &[block, data, count]);
        }
        let old = sharray_data(h, this);
        sharray_free_old(h, this, old, block);
        h.write(this, count);
        if block != this + 4 {
            h.write(this + 4, block);
        }
        this
    }

    /// `FUN_0048f060(this; src)` (`ret 4`) -> this: `PMeshSegment`'s copy:
    /// two words, the word array at `+8` copied (`PArray_allocCopyAll`),
    /// the word `+0x10`, the byte `+0x14`, the words `+0x18..+0x24`.
    pub fn mesh_segment_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
        copy_words(h, this, src, &[0, 4]);
        h.write(this + 8, 0u32);
        let count = h.read::<u32>(src + 8) & 0x7fff_ffff;
        let data: u32 = h.read(src + 0xc);
        let block = cdecl(h, 0x0048_78d0, &[count, data]);
        h.write(this + 0xc, block);
        h.write(this + 8, if block == 0 { 0 } else { count });
        copy_words(h, this, src, &[0x10]);
        copy_bytes(h, this, src, &[0x14]);
        copy_words(h, this, src, &[0x18, 0x1c, 0x20, 0x24]);
        this
    }

    /// `FUN_0048f0d0(this; src)` (`ret 4`) -> this: `PMesh`'s copy: the
    /// segment part (`FUN_0048f060`), the 64-byte segment array at `+0x28`
    /// (`FUN_00486120`), words `+0x30..+0x38`, bytes `+0x3c..+0x3e`, word
    /// `+0x40`, the sharray at `+0x44` (`FUN_0048eed0`), words
    /// `+0x5c..+0x68`.
    pub fn mesh_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
        this_call(h, 0x0048_f060, this, &[src]);
        h.write(this + 0x28, 0u32);
        let count = h.read::<u32>(src + 0x28) & 0x7fff_ffff;
        let data: u32 = h.read(src + 0x2c);
        let (block, n) = copy_array(h, count, data, 64, 0x0048_6120);
        h.write(this + 0x2c, block);
        h.write(this + 0x28, n);
        copy_words(h, this, src, &[0x30, 0x34, 0x38]);
        copy_bytes(h, this, src, &[0x3c, 0x3d, 0x3e]);
        copy_words(h, this, src, &[0x40]);
        this_call(h, 0x0048_eed0, this + 0x44, &[src + 0x44]);
        copy_words(h, this, src, &[0x5c, 0x60, 0x64, 0x68]);
        this
    }

    /// A [`STRING_RECORD_COPIES`] copy `(dst, src, count)` (cdecl): the
    /// `PString` at `+4` (its text less the tag bit 0) duplicated into a
    /// fresh block (`PhyreMallocAligned(len + 1, 2)` and `memcpy`; 0 for
    /// none or no memory), the rest copied; a destination at address 0
    /// skipped.
    pub fn string_records_copy(h: &mut impl Host, dst: u32, src: u32, count: u32) {
        let mut s = src;
        for i in 0..count {
            let d = dst.wrapping_add(16 * i);
            if d == 0 {
                continue;
            }
            copy_bytes(h, d, s, &[0, 1, 2, 3]);
            let word: u32 = h.read(s + 4);
            let text = word - (word & 1);
            let mut block = 0;
            if text != 0 {
                let mut n = 0u32;
                while h.read::<u8>(text.wrapping_add(n)) != 0 {
                    n += 1;
                }
                let len = n + 1;
                block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[len, 2]);
                if block != 0 {
                    cdecl(h, MEMCPY, &[block, text, len]);
                }
            }
            h.write(d + 4, block);
            copy_words(h, d, s, &[8]);
            copy_bytes(h, d, s, &[0xc, 0xd, 0xe, 0xf]);
            s = s.wrapping_add(16);
        }
    }

    /// `Phyre_PTexture3DBase_setDimensions(this; w, h, d, format, mips)`
    /// (`ret 0x14`) -> 0, or 3 when `1 << mips` (the shift count taken
    /// mod 32, as `shl`) exceeds the larger of `w` and `h`: the format
    /// `+0`, sizes `+0x1c..+0x24` and mip count `+0xc` stored unless all
    /// equal; the level count `+0x10` from `0x004c2430(w, h, d)` when bit 1
    /// of `+0x14` asks for a full chain, else `mips`.
    pub fn set_dimensions(
        h: &mut impl Host,
        this: u32,
        w: u32,
        hh: u32,
        d: u32,
        format: u32,
        mips: u32,
    ) -> u32 {
        if mips != 0 && 1u32.wrapping_shl(mips) > w.max(hh) {
            return 3;
        }
        let same = w == h.read(this + 0x1c)
            && hh == h.read(this + 0x20)
            && d == h.read(this + 0x24)
            && format == h.read(this)
            && mips == h.read(this + 0xc);
        if same {
            return 0;
        }
        h.write(this, format);
        let flags: u32 = h.read(this + 0x14);
        h.write(this + 0x1c, w);
        h.write(this + 0x20, hh);
        h.write(this + 0x24, d);
        h.write(this + 0xc, mips);
        let levels = if (flags >> 1) & 1 != 0 {
            cdecl(h, 0x004c_2430, &[w, hh, d])
        } else {
            mips
        };
        if levels != h.read::<u32>(this + 0x10) {
            h.write(this + 0x10, levels);
        }
        0
    }

    /// The truncation `(this; n)` (`ret 4`) of an array of tagged words
    /// (`FUN_004d5940`): the count limited to `n` (the external flag
    /// kept); in an owned array, each dropped word with bit 0 clear
    /// freed, and the block freed (`+4` = 0) when nothing is left.
    pub fn truncate_tagged(h: &mut impl Host, this: u32, n: u32) {
        let header: u32 = h.read(this);
        let count = header & 0x7fff_ffff;
        let keep = n.min(count);
        let new = (header & 0x8000_0000) | keep;
        h.write(this, new);
        if new & 0x8000_0000 != 0 {
            return;
        }
        for i in keep..count {
            let data: u32 = h.read(this + 4);
            let w: u32 = h.read(data.wrapping_add(4 * i));
            if w & 1 == 0 {
                cdecl(h, PHYRE_FREE, &[w]);
            }
        }
        if h.read::<u32>(this) == 0 {
            let data: u32 = h.read(this + 4);
            cdecl(h, PHYRE_FREE, &[data]);
            h.write(this + 4, 0u32);
        }
    }

    /// The truncation `(this; n)` (`ret 4`) of an array of array headers
    /// (`FUN_004d58b0`): as [`truncate_tagged`], each dropped header's
    /// owned non-null block freed and the header zeroed.
    pub fn truncate_headers(h: &mut impl Host, this: u32, n: u32) {
        let header: u32 = h.read(this);
        let count = header & 0x7fff_ffff;
        let keep = n.min(count);
        let new = (header & 0x8000_0000) | keep;
        h.write(this, new);
        if new & 0x8000_0000 != 0 {
            return;
        }
        for i in keep..count {
            let data: u32 = h.read(this + 4);
            let e = data.wrapping_add(8 * i);
            let c: u32 = h.read(e);
            let p: u32 = h.read(e + 4);
            if c & 0x8000_0000 == 0 && p != 0 {
                cdecl(h, PHYRE_FREE, &[p]);
            }
            h.write(e + 4, 0u32);
            h.write(e, 0u32);
        }
        if h.read::<u32>(this) == 0 {
            let data: u32 = h.read(this + 4);
            cdecl(h, PHYRE_FREE, &[data]);
            h.write(this + 4, 0u32);
        }
    }

    /// A [`SHARRAY_RESIZES`] resize `(this; n)` -> 0 or 0xd: nothing when
    /// `n` equals the header word; else the storage for `n` (a block for
    /// more than one, the inline element otherwise), the kept elements
    /// copied and the new ones zeroed, the old block freed (owned, not
    /// inline, not null, not the new one), `[n, storage]`.
    pub fn sharray20_resize(h: &mut impl Host, this: u32, n: u32, copy: u32, zero: u32) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let count = header & 0x7fff_ffff;
        let old = sharray_data(h, this);
        let keep = count.min(n);
        let block = if n > 1 {
            cdecl(h, PHYRE_MALLOC_ALIGNED, &[n.wrapping_mul(20), 4])
        } else {
            this + 4
        };
        if block == 0 {
            return 0xd;
        }
        cdecl(h, copy, &[block, old, keep]);
        cdecl(h, zero, &[block + 20 * keep, n - keep]);
        sharray_free_old(h, this, old, block);
        if block != this + 4 {
            h.write(this + 4, block);
        }
        h.write(this, n);
        0
    }

    /// A [`SHARRAY_EXTERNALS`] setter `(this; buffer, count)` (`ret 8`) ->
    /// 0, or 5 for a negative count: one element is copied inline (the
    /// header 1, owned); more become the external `[count | 0x80000000,
    /// buffer]`; the old block freed (owned, not inline, not null, not
    /// the new storage).
    pub fn sharray20_external(h: &mut impl Host, this: u32, buffer: u32, count: u32) -> u32 {
        if count & 0x8000_0000 != 0 {
            return 5;
        }
        let old = sharray_data(h, this);
        let (storage, header) = if count == 1 {
            let inl = this + 4;
            for i in 0..5 {
                let v: u32 = h.read(buffer + 4 * i);
                h.write(inl + 4 * i, v);
            }
            (inl, 1)
        } else {
            (buffer, count | 0x8000_0000)
        };
        sharray_free_old(h, this, old, storage);
        if storage != this + 4 {
            h.write(this + 4, storage);
        }
        h.write(this, header);
        0
    }

    /// A [`MEMBER_EXTERNALS`] setter `(this; buffer, count)` (`ret 8`) ->
    /// 0: the class object's virtual 1 (`relocator`); a single element
    /// handed in place (`buffer` the inline element) to the relocator
    /// (`0x0043c160(relocator; this + 4, 1)`), else the sharray setter.
    pub fn member_external(
        h: &mut impl Host,
        this: u32,
        buffer: u32,
        count: u32,
        class: u32,
        setter: u32,
    ) -> u32 {
        let obj = h.rebase(class);
        let relocator = h.call_virtual(obj, 1, &[]);
        if buffer == this + 4 && count == 1 && relocator != 0 {
            this_call(h, 0x0043_c160, relocator, &[this + 4, count]);
        } else {
            this_call(h, setter, this, &[buffer, count]);
        }
        0
    }
}
