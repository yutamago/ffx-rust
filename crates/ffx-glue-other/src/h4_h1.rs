//! Continuation round 6 of list `h4-glue-other`, batch 1: the
//! texture-animation records' copies (`TextureAnimationInfo`, its `Char`
//! subclass and `TextureAnimationMaterial`, one by one and as arrays) and
//! a group of small constructors (the escape menu's creation, the movie
//! player's semaphore blocks, a `PCluster`-based effect object, a mutex
//! block, a shader-source reader).
//!
//! The records ([`crate::h4_c1`] has their destructors): a `PString` is a
//! word whose bit 0 marks a static string; a copy duplicates the text
//! (`PhyreMalloc(len + 1, 2)` and the CRT `memcpy`), a static one too. A
//! word array is a count (top bit: not owned) and a block; a copy owns a
//! new block, and its count is 0 when the allocation failed. The SEH
//! frames are not kept: they only unwind partly built records.

use ffx_graphic::host::{Conv, Host};

use crate::h4_c1::{TEXTURE_ANIMATION_INFO_CHAR_VTABLE, TEXTURE_ANIMATION_INFO_VTABLE};
use crate::h4_e1::pass;

/// `Phyre_PhyreMalloc_0042fb80(bytes, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PhyreFree(block)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// The CRT `memcpy(dst, src, n)` (cdecl, `0x00949256`).
pub const MEMCPY: u32 = 0x0094_9256;
/// The CRT `memset(dst, c, n)` thunk (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre_PAnimation_copyWords_005281e0(n, src)` (cdecl).
pub const COPY_WORDS: u32 = 0x0052_81e0;
/// `PArray_copyNew_006affc0(n, src)` (cdecl).
pub const COPY_NEW_A: u32 = crate::h4_c1::COPY_NEW_A;
/// `PArray_copyNew_006b0010(n, src)` (cdecl).
pub const COPY_NEW_B: u32 = crate::h4_c1::COPY_NEW_B;
/// `TextureAnimationInfo_copy_006af0d0(this; src)` (thiscall, ret 4).
pub const INFO_COPY: u32 = 0x006a_f0d0;
/// `TextureAnimationInfoChar_copyArray_006aeb20(dst, src, n)` (cdecl).
pub const CHAR_COPY_ARRAY: u32 = 0x006a_eb20;

/// The `PString` word at `at` duplicated: 0 for a null text, else
/// `PhyreMalloc(len + 1, 2)` holding the text and its terminator (0 when
/// the allocation fails).
fn dup_string(h: &mut impl Host, at: u32) -> u32 {
    let word: u32 = h.read(at);
    let text = word.wrapping_sub(word & 1);
    if text == 0 {
        return 0;
    }
    let mut len = 0u32;
    while h.read::<u8>(text.wrapping_add(len)) != 0 {
        len = len.wrapping_add(1);
    }
    let n = len.wrapping_add(1);
    let block = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n, 2]);
    if block != 0 {
        h.call_sym(MEMCPY, Conv::Cdecl, &[block, text, n]);
    }
    block
}

/// The word array `(count, block)` at `src` copied into `dst` by the
/// inline loop: `PhyreMalloc(4n, 4)` for `n` the count without its top
/// bit, each word stored where the cursor is not null (the source moves
/// only then); `dst + 4` the block, `dst` `n` (0 without a block).
fn copy_words_inline(h: &mut impl Host, dst: u32, src: u32) {
    let n = h.read::<u32>(src) & 0x7fff_ffff;
    let mut from: u32 = h.read(src.wrapping_add(4));
    let mut block = 0;
    if n != 0 {
        block = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n << 2, 4]);
        if block != 0 {
            let mut to = block;
            for _ in 0..n {
                if to != 0 {
                    let w: u32 = h.read(from);
                    h.write(to, w);
                    from = from.wrapping_add(4);
                }
                to = to.wrapping_add(4);
            }
        }
    }
    h.write(dst.wrapping_add(4), block);
    h.write(dst, if block == 0 { 0 } else { n });
}

/// A word array copied by a call `copy(n, block)` (cdecl): `dst + 4` its
/// result, `dst` the count without its top bit (0 for a null result).
fn copy_words_call(h: &mut impl Host, dst: u32, src: u32, copy: u32) {
    let n = h.read::<u32>(src) & 0x7fff_ffff;
    let data: u32 = h.read(src.wrapping_add(4));
    let block = h.call_sym(copy, Conv::Cdecl, &[n, data]);
    h.write(dst.wrapping_add(4), block);
    h.write(dst, if block == 0 { 0 } else { n });
}

/// The `TextureAnimationInfoChar` array `(count, block)` at `src` copied
/// into `dst`: `PhyreMalloc(64n, 4)` filled by
/// `TextureAnimationInfoChar_copyArray_006aeb20(block, data, n)` (called
/// only for a block); `dst + 4` the block, `dst` `n` (0 without one).
fn copy_chars(h: &mut impl Host, dst: u32, src: u32) {
    let n = h.read::<u32>(src) & 0x7fff_ffff;
    let data: u32 = h.read(src.wrapping_add(4));
    let mut block = 0;
    if n != 0 {
        block = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n << 6, 4]);
        if block != 0 {
            h.call_sym(CHAR_COPY_ARRAY, Conv::Cdecl, &[block, data, n]);
        }
    }
    h.write(dst.wrapping_add(4), block);
    h.write(dst, if block == 0 { 0 } else { n });
}

/// The float word at `src` stored at `dst` through `fld`/`fstp dword`.
fn copy_float(h: &mut impl Host, dst: u32, src: u32) {
    let v: u32 = h.read(src);
    h.write(dst, pass(v));
}

/// `TextureAnimationInfoChar`'s own fields from `src` into `this`
/// (`+0x20`..`+0x40`): the name duplicated, four floats, a byte, two
/// words.
fn char_fields(h: &mut impl Host, this: u32, src: u32) {
    let name = dup_string(h, src.wrapping_add(0x20));
    h.write(this.wrapping_add(0x20), name);
    for off in [0x24, 0x28, 0x2c, 0x30] {
        copy_float(h, this.wrapping_add(off), src.wrapping_add(off));
    }
    let b: u8 = h.read(src.wrapping_add(0x34));
    h.write(this.wrapping_add(0x34), b);
    for off in [0x38, 0x3c] {
        let w: u32 = h.read(src.wrapping_add(off));
        h.write(this.wrapping_add(off), w);
    }
}

/// `TextureAnimationInfo_copy_006af0d0(this; src)` (thiscall, ret 4): the
/// copy constructor. Its vtable, the name `+4` duplicated, `+8` cleared,
/// the frame words `+8`/`+0xc` copied by
/// `Phyre_PAnimation_copyWords_005281e0` (count 0 for a null block), the
/// floats `+0x10`, `+0x14`, the word `+0x18` and the byte `+0x1c`.
/// Returns `this`.
pub fn info_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
    let vtable = h.rebase(TEXTURE_ANIMATION_INFO_VTABLE);
    h.write(this, vtable);
    let name = dup_string(h, src.wrapping_add(4));
    h.write(this.wrapping_add(4), name);
    h.write(this.wrapping_add(8), 0u32);
    copy_words_call(h, this.wrapping_add(8), src.wrapping_add(8), COPY_WORDS);
    copy_float(h, this.wrapping_add(0x10), src.wrapping_add(0x10));
    copy_float(h, this.wrapping_add(0x14), src.wrapping_add(0x14));
    let w: u32 = h.read(src.wrapping_add(0x18));
    h.write(this.wrapping_add(0x18), w);
    let b: u8 = h.read(src.wrapping_add(0x1c));
    h.write(this.wrapping_add(0x1c), b);
    this
}

/// `TextureAnimationInfoChar_copy_006af270(this; src)` (thiscall, ret 4):
/// the base copied (`TextureAnimationInfo_copy_006af0d0`), its own vtable,
/// then its fields `+0x20`..`+0x40`. Returns `this`.
pub fn info_char_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
    h.call_sym(INFO_COPY, Conv::Thiscall(this), &[src]);
    let vtable = h.rebase(TEXTURE_ANIMATION_INFO_CHAR_VTABLE);
    h.write(this, vtable);
    char_fields(h, this, src);
    this
}

/// `TextureAnimationInfo_copyArray_006ae9c0(dst, src, n)` (cdecl): `n`
/// records of 0x20 bytes copy-constructed, [`info_copy`] inline but with
/// the word array copied by the inline loop. A null destination slot is
/// skipped and the source stays where it is. Returns `dst + 0x20 * n`;
/// for `n == 0` the original's `eax` is junk (callers don't read it).
pub fn info_copy_array(h: &mut impl Host, dst: u32, src: u32, n: u32) -> u32 {
    let (mut to, mut from) = (dst, src);
    for _ in 0..n {
        if to != 0 {
            let vtable = h.rebase(TEXTURE_ANIMATION_INFO_VTABLE);
            h.write(to, vtable);
            let name = dup_string(h, from.wrapping_add(4));
            h.write(to.wrapping_add(4), name);
            h.write(to.wrapping_add(8), 0u32);
            copy_words_inline(h, to.wrapping_add(8), from.wrapping_add(8));
            copy_float(h, to.wrapping_add(0x10), from.wrapping_add(0x10));
            copy_float(h, to.wrapping_add(0x14), from.wrapping_add(0x14));
            let w: u32 = h.read(from.wrapping_add(0x18));
            h.write(to.wrapping_add(0x18), w);
            let b: u8 = h.read(from.wrapping_add(0x1c));
            h.write(to.wrapping_add(0x1c), b);
            from = from.wrapping_add(0x20);
        }
        to = to.wrapping_add(0x20);
    }
    to
}

/// `TextureAnimationInfoChar_copyArray_006aeb20(dst, src, n)` (cdecl): `n`
/// records of 0x40 bytes, each [`info_char_copy`] inline (the base by
/// `TextureAnimationInfo_copy_006af0d0`). A null destination slot is
/// skipped and the source stays. Returns `dst + 0x40 * n` (junk for
/// `n == 0`).
pub fn info_char_copy_array(h: &mut impl Host, dst: u32, src: u32, n: u32) -> u32 {
    let (mut to, mut from) = (dst, src);
    for _ in 0..n {
        if to != 0 {
            info_char_copy(h, to, from);
            from = from.wrapping_add(0x40);
        }
        to = to.wrapping_add(0x40);
    }
    to
}

/// `TextureAnimationMaterial_copy_006aeee0(this; src)` (thiscall, ret 4):
/// the copy constructor. The word arrays `+0` (`PArray_copyNew_006affc0`)
/// and `+0x10` (`PArray_copyNew_006b0010`), the `Char` array `+8`, the
/// names `+0x18`, `+0x1c` duplicated. Returns `this`.
pub fn material_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
    h.write(this, 0u32);
    copy_words_call(h, this, src, COPY_NEW_A);
    h.write(this.wrapping_add(8), 0u32);
    copy_chars(h, this.wrapping_add(8), src.wrapping_add(8));
    h.write(this.wrapping_add(0x10), 0u32);
    copy_words_call(
        h,
        this.wrapping_add(0x10),
        src.wrapping_add(0x10),
        COPY_NEW_B,
    );
    for off in [0x18, 0x1c] {
        let name = dup_string(h, src.wrapping_add(off));
        h.write(this.wrapping_add(off), name);
    }
    this
}

/// `TextureAnimationMaterial_copyArray_006ae730(dst, src, n)` (cdecl):
/// `n` records of 0x20 bytes, each [`material_copy`] inline with the two
/// word arrays copied by the inline loop. A null destination slot is
/// skipped and the source stays. Returns `dst + 0x20 * n` (junk for
/// `n == 0`).
pub fn material_copy_array(h: &mut impl Host, dst: u32, src: u32, n: u32) -> u32 {
    let (mut to, mut from) = (dst, src);
    for _ in 0..n {
        if to != 0 {
            h.write(to, 0u32);
            copy_words_inline(h, to, from);
            h.write(to.wrapping_add(8), 0u32);
            copy_chars(h, to.wrapping_add(8), from.wrapping_add(8));
            h.write(to.wrapping_add(0x10), 0u32);
            copy_words_inline(h, to.wrapping_add(0x10), from.wrapping_add(0x10));
            for off in [0x18, 0x1c] {
                let name = dup_string(h, from.wrapping_add(off));
                h.write(to.wrapping_add(off), name);
            }
            from = from.wrapping_add(0x20);
        }
        to = to.wrapping_add(0x20);
    }
    to
}

// ---- small constructors

/// `eh_vector_constructor_iterator(base, size, n, ctor, dtor)` (the CRT
/// import thunk; stdcall, five words).
pub const EH_VECTOR_CTOR: u32 = 0x0094_966a;
/// `operator_new(bytes)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `KERNEL32!CreateSemaphoreW`'s import slot (stdcall, four words).
pub const CREATE_SEMAPHORE_IAT: u32 = 0x00b0_c1e0;
/// `KERNEL32!CreateMutexW`'s import slot (stdcall, three words).
pub const CREATE_MUTEX_IAT: u32 = 0x00b0_c198;
/// `Phyre_PPair_init_004677d0(pair)` (thiscall).
pub const PAIR_INIT: u32 = 0x0046_77d0;
/// `Phyre_PCluster_operator_new(this)` (thiscall; `PCluster`'s
/// constructor despite the name).
pub const PCLUSTER_CTOR: u32 = 0x0057_6630;
/// `PSamplerStateD3D11_ctor(this)` (thiscall).
pub const SAMPLER_STATE_CTOR: u32 = crate::h4_c1::SAMPLER_STATE_CTOR;
/// `FFEscMenu_ctor_0068dfd0(this; a, f0, f1, f2, f3)` (thiscall, ret 0x14).
pub const ESC_MENU_CTOR: u32 = 0x0068_dfd0;
/// `g_FFEscMenu` (`0x00ccc870`).
pub const ESC_MENU: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00cc_c870);
/// `0x00654ea0()` (cdecl): a new empty tree head (`operator_new(0x2c)`).
pub const NEW_TREE_HEAD: u32 = 0x0065_4ea0;
/// `AssetListReader_clear(this)` (thiscall).
pub const ASSET_LIST_CLEAR: u32 = 0x006f_4030;
/// `std::string::assign(this; text, n)` (`0x00404270`, thiscall, ret 8).
pub const STRING_ASSIGN: u32 = 0x0040_4270;
/// `"#ifdef PHYRE_RENDERING_PLATFORM_D3D11"` (`0x00b46f48`, 0x25 bytes).
pub const D3D11_IFDEF: u32 = 0x00b4_6f48;

/// One `eh_vector_constructor_iterator(base, size, n, ctor, dtor)` with
/// image addresses `ctor` and `dtor`.
fn vector_ctor(h: &mut impl Host, base: u32, size: u32, n: u32, ctor: u32, dtor: u32) {
    let (ctor, dtor) = (h.rebase(ctor), h.rebase(dtor));
    h.call_sym(EH_VECTOR_CTOR, Conv::Stdcall, &[base, size, n, ctor, dtor]);
}

/// `CreateSemaphoreW(0, 1, 1, 0)`.
fn semaphore(h: &mut impl Host) -> u32 {
    let create: u32 = h.read(h.rebase(CREATE_SEMAPHORE_IAT));
    h.call(create, Conv::Stdcall, &[0, 1, 1, 0])
}

/// A semaphore in a block of its own (`operator_new(4)`), read back: for
/// a null block the original reads address 0 too.
fn boxed_semaphore(h: &mut impl Host) -> u32 {
    let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[4]);
    if block != 0 {
        let s = semaphore(h);
        h.write(block, s);
    }
    h.read(block)
}

/// `SemaphoreQueue_ctor_006db560(this)` (thiscall): eight 0x30-byte
/// entries (`eh_vector_constructor_iterator`, `0x007038f0`/`0x00703900`),
/// two queues (`+0x1a0` and `+0x1d0`: three words cleared, a boxed
/// semaphore `CreateSemaphoreW(0, 1, 1, 0)` at `+0xc`), two more
/// semaphores (`+0x1e0`, `+0x1e4`). Returns `this`.
pub fn semaphore_queue_ctor(h: &mut impl Host, this: u32) -> u32 {
    vector_ctor(h, this, 0x30, 8, 0x0070_38f0, 0x0070_3900);
    for q in [0x1a0u32, 0x1d0] {
        for k in 0..3u32 {
            h.write(this.wrapping_add(q + 4 * k), 0u32);
        }
        let s = boxed_semaphore(h);
        h.write(this.wrapping_add(q + 0xc), s);
    }
    for off in [0x1e0u32, 0x1e4] {
        let s = semaphore(h);
        h.write(this.wrapping_add(off), s);
    }
    this
}

/// `SemaphoreQueues_ctor_006db670(this)` (thiscall): four 0x14-byte
/// entries (`0x00703960`/`0x00703990`) and one 0x1e8-byte
/// [`semaphore_queue_ctor`] at `+0x50` (`0x006db560`/`0x006db6f0`), both
/// by `eh_vector_constructor_iterator`. Returns `this`.
pub fn semaphore_queues_ctor(h: &mut impl Host, this: u32) -> u32 {
    vector_ctor(h, this, 0x14, 4, 0x0070_3960, 0x0070_3990);
    vector_ctor(
        h,
        this.wrapping_add(0x50),
        0x1e8,
        1,
        0x006d_b560,
        0x006d_b6f0,
    );
    this
}

/// `PostProcessState_init_006ddbf0(this)` (thiscall): a pair
/// (`PhyreMalloc(8, 4)`, `Phyre_PPair_init_004677d0`) at `+0x10c` (0
/// without memory; the block is stored there first), 0x100 bytes from
/// `+0xc` cleared, byte `+1` set. Returns `memset`'s `eax`.
pub fn post_process_state_init(h: &mut impl Host, this: u32) -> u32 {
    let block = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[8, 4]);
    h.write(this.wrapping_add(0x10c), block);
    let pair = if block == 0 {
        0
    } else {
        h.call_sym(PAIR_INIT, Conv::Thiscall(block), &[])
    };
    h.write(this.wrapping_add(0x10c), pair);
    let r = h.call_sym(MEMSET, Conv::Cdecl, &[this.wrapping_add(0xc), 0, 0x100]);
    h.write(this.wrapping_add(1), 1u8);
    r
}

/// `DynGeoMemPool_ctor_006de2c0(this)` (thiscall): the block array
/// `+0xc`/`+0x10` cleared, a mutex (`CreateMutexW(0, 0, 0)`) at `+0x14`,
/// `+0` 0 and `+4` -1, the (empty) array's reset, `+8` 0. Returns
/// `this`.
pub fn dyn_geo_pool_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this.wrapping_add(0xc), 0u32);
    h.write(this.wrapping_add(0x10), 0u32);
    let create: u32 = h.read(h.rebase(CREATE_MUTEX_IAT));
    let mutex = h.call(create, Conv::Stdcall, &[0, 0, 0]);
    h.write(this.wrapping_add(0x14), mutex);
    h.write(this, 0u32);
    h.write(this.wrapping_add(4), 0xffff_ffffu32);
    let count: u32 = h.read(this.wrapping_add(0xc));
    if count != 0 {
        let data: u32 = h.read(this.wrapping_add(0x10));
        if data != 0 && count >> 31 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        h.write(this.wrapping_add(0x10), 0u32);
        h.write(this.wrapping_add(0xc), 0u32);
    }
    h.write(this.wrapping_add(8), 0u32);
    this
}

/// `ShadowManager_ctor_006c7d80(this)` (thiscall): the `PCluster` at `+4`
/// constructed, the list head `+0x4c` pointing at itself, `+0x74`..`+0x80`
/// cleared, two pairs of 8-byte entries (`+0x290`, `+0x2a0`;
/// `0x006c7ca0`/`0x006c7e60`), two sampler states (`+0x3fc4`, `+0x3fe8`),
/// `+0x4060` 3 and byte `+0x4064` 0. Returns `this`.
pub fn shadow_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.call_sym(PCLUSTER_CTOR, Conv::Thiscall(this.wrapping_add(4)), &[]);
    let head = this.wrapping_add(0x4c);
    h.write(head, head);
    h.write(head.wrapping_add(4), head);
    for k in 0..4u32 {
        h.write(this.wrapping_add(0x74 + 4 * k), 0u32);
    }
    for off in [0x290u32, 0x2a0] {
        vector_ctor(h, this.wrapping_add(off), 8, 2, 0x006c_7ca0, 0x006c_7e60);
    }
    for off in [0x3fc4u32, 0x3fe8] {
        h.call_sym(
            SAMPLER_STATE_CTOR,
            Conv::Thiscall(this.wrapping_add(off)),
            &[],
        );
    }
    h.write(this.wrapping_add(0x4060), 3u32);
    h.write(this.wrapping_add(0x4064), 0u8);
    this
}

/// `ShaderSourceReader_ctor_006904c0(this)` (thiscall): `+0`/`+4`
/// cleared, a new tree head (`0x00654ea0`) at `+0`, nine words
/// `+8`..`+0x2c` cleared, an empty `std::string` at `+0x2c` (capacity
/// `+0x40` 15, length `+0x3c` 0, first byte 0), the reader cleared
/// (`AssetListReader_clear`), the string assigned
/// `"#ifdef PHYRE_RENDERING_PLATFORM_D3D11"`. Returns `this`.
pub fn shader_source_reader_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this, 0u32);
    h.write(this.wrapping_add(4), 0u32);
    let head = h.call_sym(NEW_TREE_HEAD, Conv::Cdecl, &[]);
    h.write(this, head);
    for k in 0..9u32 {
        h.write(this.wrapping_add(8 + 4 * k), 0u32);
    }
    h.write(this.wrapping_add(0x40), 15u32);
    h.write(this.wrapping_add(0x3c), 0u32);
    h.write(this.wrapping_add(0x2c), 0u8);
    h.call_sym(ASSET_LIST_CLEAR, Conv::Thiscall(this), &[]);
    let text = h.rebase(D3D11_IFDEF);
    h.call_sym(
        STRING_ASSIGN,
        Conv::Thiscall(this.wrapping_add(0x2c)),
        &[text, 0x25],
    );
    this
}

/// `FFEscMenu_create_0068e400(a, f0, f1, f2, f3)` (cdecl): with no escape
/// menu yet (`g_FFEscMenu`), `operator_new(0x238)` constructed by
/// `FFEscMenu_ctor_0068dfd0(a, f0..f3)` (the floats through
/// `fld`/`fstp`) into `g_FFEscMenu`, or 0 there without memory. Returns
/// the constructor's `eax`, 0 without memory, `None` when a menu exists
/// (the original's `eax` is then a stack address).
pub fn esc_menu_create(h: &mut impl Host, a: u32, f: [u32; 4]) -> Option<u32> {
    if ESC_MENU.get(h) != 0 {
        return None;
    }
    let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x238]);
    if block == 0 {
        ESC_MENU.set(h, 0);
        return Some(0);
    }
    let args = [a, pass(f[0]), pass(f[1]), pass(f[2]), pass(f[3])];
    let menu = h.call_sym(ESC_MENU_CTOR, Conv::Thiscall(block), &args);
    ESC_MENU.set(h, menu);
    Some(menu)
}
