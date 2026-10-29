//! M5 stage 1, `PhyreEngine` list `render-a`, batch 2: array storage
//! (allocate-and-copy helpers, resizes), element copies and initialisers,
//! the sampler/texture setters that refuse a locked object, the sprite
//! collection's per-frame setters, static descriptor getters and the
//! small destructors and constructors around them.
//!
//! Status codes the engine returns: 0 success, 5 bad argument, 9 not
//! ready, 0xd out of memory, 0x16 locked.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_render_a::{OPERATOR_DELETE, PHYRE_FREE, PHYRE_MALLOC_ALIGNED, f32_move};

/// `atexit(f)` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// `0x0043d6a0(class; object, flag)`: the class record's register.
pub const CLASS_REGISTER: u32 = 0x0043_d6a0;
/// The CRT's `__ehvec_dtor(array, size, count, dtor)` (stdcall).
pub const EHVEC_DTOR: u32 = 0x0094_96d2;
/// `Phyre_PFreeList_reset(list)`.
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;

/// A static `PClassDescriptorConcrete<PArray<T>>` getter that registers
/// an object: the descriptor `d` built on first use (`ctor(d)`, flag bit
/// 1 of `d + 0x90` cleared, its vtable, the atexit destructor), then
/// `CLASS_REGISTER(d; p, flag)`.
#[derive(Debug, Clone, Copy)]
pub struct ConcreteRegister {
    pub d: u32,
    pub ctor: u32,
    pub vtable: u32,
    pub atexit: u32,
    pub flag: u32,
}

/// The [`ConcreteRegister`] copies by address.
pub const CONCRETE_REGISTERS: &[(u32, ConcreteRegister)] = &[
    (
        0x0048_1130,
        reg(0x00c9_8a98, 0x0048_1470, 0x00b1_43fc, 0x00af_c550, 1),
    ),
    (
        0x0048_1200,
        reg(0x00c9_8a98, 0x0048_1470, 0x00b1_43fc, 0x00af_c550, 0),
    ),
    (
        0x0048_a000,
        reg(0x00c9_9a30, 0x0048_a310, 0x00b1_5624, 0x00af_c840, 1),
    ),
    (
        0x0048_a0b0,
        reg(0x00c9_9a30, 0x0048_a310, 0x00b1_5624, 0x00af_c840, 0),
    ),
    (
        0x0049_62c0,
        reg(0x00c9_aa10, 0x0049_6610, 0x00b1_6b44, 0x00af_cc60, 0),
    ),
    (
        0x004b_b6c0,
        reg(0x00ca_26d8, 0x004b_b9f0, 0x00b1_d424, 0x00af_dbf0, 0),
    ),
    (
        0x004f_aef0,
        reg(0x00ca_80f0, 0x004f_b1e0, 0x00b2_3a34, 0x00af_f830, 1),
    ),
    (
        0x004f_afb0,
        reg(0x00ca_80f0, 0x004f_b1e0, 0x00b2_3a34, 0x00af_f830, 0),
    ),
];

const fn reg(d: u32, ctor: u32, vtable: u32, atexit: u32, flag: u32) -> ConcreteRegister {
    ConcreteRegister {
        d,
        ctor,
        vtable,
        atexit,
        flag,
    }
}

/// The [`ConcreteRegister`] of a copy (a zero record for another address).
#[must_use]
pub const fn concrete_register(addr: u32) -> ConcreteRegister {
    let mut i = 0;
    while i < CONCRETE_REGISTERS.len() {
        if CONCRETE_REGISTERS[i].0 == addr {
            return CONCRETE_REGISTERS[i].1;
        }
        i += 1;
    }
    reg(0, 0, 0, 0, 0)
}

/// The conditional destructors `(this)`: `if test(this) { release(this) }`
/// then `dtor(this)` (all thiscall), its `eax`: (address, test, release,
/// dtor).
pub const RELEASE_DTORS: &[(u32, u32, u32, u32)] = &[
    (0x0048_18b0, 0x0056_2da0, 0x0056_46d0, 0x0055_dfa0),
    (0x004c_3010, 0x0059_5f90, 0x0059_cc40, 0x0058_5840),
    (0x0050_1cd0, 0x0059_5f80, 0x0059_ca60, 0x0058_57b0),
];

/// The allocate-and-copy helpers `(n, src, count)` (cdecl) -> block:
/// (address, element size, alignment, the copy `(dst, src, count)` or 0
/// for plain words, the words zeroed per element past `count`, or `None`
/// for none).
pub const ALLOC_COPIES: &[(u32, u32, u32, u32, Option<u32>)] = &[
    (0x0049_00c0, 4, 4, 0, None),
    (0x0048_7920, 4, 2, 0, None),
    (0x0049_01f0, 4, 4, 0x0044_a470, Some(4)),
    (0x004b_6220, 2, 2, 0x004b_2d00, Some(2)),
    (0x004e_b260, 8, 4, 0x004e_7d70, Some(8)),
];

/// The resizes `(this; n)` (`ret 4`) through an allocate-and-copy helper
/// `(n, data, min(n, count))`: (address, the helper, whether a null old
/// block is skipped).
pub const RESIZE_VIA: &[(u32, u32, bool)] = &[
    (0x004f_cac0, 0x004f_b850, false),
    (0x004b_a640, 0x004b_6220, true),
];

/// The resizes that drop the contents `(this; n)` (`ret 4`): (address,
/// element size).
pub const RESIZE_DISCARD: &[(u32, u32)] = &[(0x0049_4c20, 0x40), (0x0049_bfc0, 4)];

/// The sprite collection's per-frame float setters `(this; frame, v...)`:
/// (address, offset in the 48-byte frame record, floats).
pub const SPRITE_SETTERS: &[(u32, u32, u32)] = &[
    (0x0056_e4c0, 0x28, 1),
    (0x0056_e6e0, 0, 2),
    (0x0056_e800, 0x20, 2),
    (0x0056_e910, 0x10, 2),
    (0x0056_ea20, 0x18, 2),
];

/// The locked-object word setters `(this; v)` (`ret 4`) -> 0, or 0x16
/// when bit 7 of `+0x1c` is set: (address, offset, a float).
pub const LOCKED_SETTERS: &[(u32, u32, bool)] = &[
    (0x004c_0680, 0x14, false),
    (0x004c_06b0, 0x10, false),
    (0x004c_07c0, 0x18, false),
    (0x004c_0720, 8, true),
    (0x004c_0790, 0xc, true),
];

/// The texture owner setters `(this; owner)` (`ret 4`): (address, the
/// owner kind byte).
pub const OWNER_SETTERS: &[(u32, u8)] = &[(0x004c_2920, 1), (0x004c_2950, 2)];

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

    /// A [`ConcreteRegister`] getter `(p)` (cdecl).
    pub fn concrete_register(h: &mut impl Host, p: u32, r: &ConcreteRegister) -> u32 {
        let d = h.rebase(r.d);
        let guard = d + 0x94;
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            this_call(h, r.ctor, d, &[]);
            let f: u32 = h.read(d + 0x90);
            h.write(d + 0x90, f & !2);
            let vt = h.rebase(r.vtable);
            h.write(d, vt);
            let at = h.rebase(r.atexit);
            cdecl(h, ATEXIT, &[at]);
        }
        this_call(h, CLASS_REGISTER, d, &[p, r.flag])
    }

    /// A [`RELEASE_DTORS`] destructor `(this)`.
    pub fn release_dtor(h: &mut impl Host, this: u32, test: u32, release: u32, dtor: u32) -> u32 {
        if this_call(h, test, this, &[]) & 0xff != 0 {
            this_call(h, release, this, &[]);
        }
        this_call(h, dtor, this, &[])
    }

    /// An [`ALLOC_COPIES`] helper `(n, src, count)` -> the block (0 for
    /// `n` 0 or no memory): `n` elements, the first `count` copied (by
    /// `copy`, or word by word skipping a slot at address 0), the rest
    /// zeroed when `zero` says so (a slot at address 0 skipped).
    #[allow(clippy::too_many_arguments)] // the copy's constants
    pub fn alloc_copy(
        h: &mut impl Host,
        n: u32,
        src: u32,
        count: u32,
        size: u32,
        align: u32,
        copy: u32,
        zero: Option<u32>,
    ) -> u32 {
        if n == 0 {
            return 0;
        }
        let block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[n.wrapping_mul(size), align]);
        if block == 0 {
            return 0;
        }
        if copy == 0 {
            let mut s = src;
            for i in 0..count {
                let d = block.wrapping_add(4 * i);
                if d != 0 {
                    let v: u32 = h.read(s);
                    h.write(d, v);
                    s = s.wrapping_add(4);
                }
            }
        } else {
            cdecl(h, copy, &[block, src, count]);
        }
        if let Some(bytes) = zero {
            for i in 0..n.wrapping_sub(count) {
                let d = block.wrapping_add(size.wrapping_mul(count.wrapping_add(i)));
                if d != 0 {
                    h.write_bytes(d, &[0u8; 8][..bytes as usize]);
                }
            }
        }
        block
    }

    /// `FUN_004878d0(n, src)` (cdecl) -> block: `n` words (alignment 2)
    /// copied from `src`, a slot at address 0 skipped.
    pub fn alloc_copy_all(h: &mut impl Host, n: u32, src: u32) -> u32 {
        alloc_copy(h, n, src, n, 4, 2, 0, None)
    }

    /// Frees the old block of an array header at `this` when it is owned
    /// and not the new one (with `skip_null`, not null either).
    fn free_old(h: &mut impl Host, this: u32, block: u32, skip_null: bool) {
        let old: u32 = h.read(this + 4);
        let header: u32 = h.read(this);
        if old != block && header & 0x8000_0000 == 0 && !(skip_null && old == 0) {
            cdecl(h, PHYRE_FREE, &[old]);
        }
    }

    /// A [`RESIZE_VIA`] resize `(this; n)` -> 0, or 0xd when the helper
    /// finds no memory for `n` elements; nothing when `n` equals the
    /// header word (an owned array of `n`).
    pub fn resize_via(h: &mut impl Host, this: u32, n: u32, helper: u32, skip_null: bool) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let count = (header & 0x7fff_ffff).min(n);
        let data: u32 = h.read(this + 4);
        let block = cdecl(h, helper, &[n, data, count]);
        if block == 0 && n != 0 {
            return 0xd;
        }
        free_old(h, this, block, skip_null);
        h.write(this + 4, block);
        h.write(this, n);
        0
    }

    /// A [`RESIZE_DISCARD`] resize `(this; n)` -> 0 or 0xd: a fresh block
    /// of `n` elements (alignment 4, contents not kept), the old one freed
    /// when owned, not null and not the new one.
    pub fn resize_discard(h: &mut impl Host, this: u32, n: u32, size: u32) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let block = if n == 0 {
            0
        } else {
            cdecl(h, PHYRE_MALLOC_ALIGNED, &[n.wrapping_mul(size), 4])
        };
        if block == 0 && n != 0 {
            return 0xd;
        }
        free_old(h, this, block, true);
        h.write(this + 4, block);
        h.write(this, n);
        0
    }

    /// The copy-and-initialise resize `FUN_004ba720(this; n)` -> 0 or
    /// 0xd: 16-byte elements (alignment 4), the kept ones copied by
    /// `FUN_004b2d70`, the new ones set up by `FUN_004b8cf0`.
    pub fn resize_copy_init16(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let count = (header & 0x7fff_ffff).min(n);
        let data: u32 = h.read(this + 4);
        let mut block = 0;
        if n != 0 {
            block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[n << 4, 4]);
            if block == 0 {
                return 0xd;
            }
            cdecl(h, 0x004b_2d70, &[block, data, count]);
            cdecl(h, 0x004b_8cf0, &[block + (count << 4), n - count]);
        }
        free_old(h, this, block, true);
        h.write(this + 4, block);
        h.write(this, n);
        0
    }

    /// `FUN_004e1150(this; n)` -> 0 or 0xd: 32-byte elements, the kept
    /// ones copied by `FUN_004d8a30`, the new ones built by
    /// `FUN_004df1d0`, then `Phyre_PArray_PShader_setStorage(this; block,
    /// n)`.
    pub fn resize_shader_infos(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let count = (header & 0x7fff_ffff).min(n);
        let data: u32 = h.read(this + 4);
        let mut block = 0;
        if n != 0 {
            block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[n << 5, 4]);
            if block == 0 {
                return 0xd;
            }
            cdecl(h, 0x004d_8a30, &[block, data, count]);
            cdecl(h, 0x004d_f1d0, &[block + (count << 5), n - count]);
        }
        this_call(h, 0x004e_13a0, this, &[block, n]);
        0
    }

    /// `FUN_004b2d00(dst, src, count)` (cdecl): 16-bit elements, a slot
    /// at address 0 skipped.
    pub fn copy_u16(h: &mut impl Host, dst: u32, src: u32, count: u32) {
        let mut s = src;
        for i in 0..count {
            let d = dst.wrapping_add(2 * i);
            if d != 0 {
                let v: u16 = h.read(s);
                h.write(d, v);
                s = s.wrapping_add(2);
            }
        }
    }

    /// `FUN_004b2d70(dst, src, count)` (cdecl): 16-byte records (a 16-bit
    /// word, two words, a byte at `+0xc`), a slot at address 0 skipped.
    pub fn copy_location16(h: &mut impl Host, dst: u32, src: u32, count: u32) {
        let mut s = src;
        for i in 0..count {
            let d = dst.wrapping_add(16 * i);
            if d != 0 {
                let a: u16 = h.read(s);
                h.write(d, a);
                let b: u32 = h.read(s + 4);
                h.write(d + 4, b);
                let c: u32 = h.read(s + 8);
                h.write(d + 8, c);
                let e: u8 = h.read(s + 0xc);
                h.write(d + 0xc, e);
                s = s.wrapping_add(16);
            }
        }
    }

    /// `FUN_004b8cf0(data, count)` (cdecl): 16-byte records set to
    /// `[0u16, 0, 0, 0x34u8]`, one at address 0 skipped.
    pub fn init_location16(h: &mut impl Host, data: u32, count: u32) {
        for i in 0..count {
            let d = data.wrapping_add(16 * i);
            if d != 0 {
                h.write(d, 0u16);
                h.write(d + 4, 0u32);
                h.write(d + 8, 0u32);
                h.write(d + 0xc, 0x34u8);
            }
        }
    }

    /// `FUN_0056bab0(data, count)` (cdecl): 20-byte records set to `[0,
    /// 0.0, 0.0, 1.0, 1.0]`, one at address 0 skipped.
    pub fn init_subtexture20(h: &mut impl Host, data: u32, count: u32) {
        for i in 0..count {
            let d = data.wrapping_add(20 * i);
            if d != 0 {
                h.write(d, 0u32);
                h.write(d + 4, 0u32);
                h.write(d + 8, 0u32);
                h.write(d + 0xc, 1.0f32.to_bits());
                h.write(d + 0x10, 1.0f32.to_bits());
            }
        }
    }

    /// `FUN_004bbbd0(this)` -> this: `[0xf, 0 x 7]`.
    pub fn init_shader_ref(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0xfu32);
        for i in 1..8 {
            h.write(this + 4 * i, 0u32);
        }
        this
    }

    /// `FUN_004d3720(headers, n)` (cdecl): each 8-byte array header's
    /// owned, non-null block freed and the header zeroed, then the
    /// headers' block freed; nothing for a null `headers`.
    pub fn free_headers(h: &mut impl Host, headers: u32, n: u32) {
        if headers == 0 {
            return;
        }
        for i in 0..n {
            let a = headers + 8 * i;
            let count: u32 = h.read(a);
            let data: u32 = h.read(a + 4);
            if count & 0x8000_0000 == 0 && data != 0 {
                cdecl(h, PHYRE_FREE, &[data]);
            }
            h.write(a + 4, 0u32);
            h.write(a, 0u32);
        }
        cdecl(h, PHYRE_FREE, &[headers]);
    }

    /// A [`LOCKED_SETTERS`] setter `(this; v)`.
    pub fn locked_set(h: &mut impl Host, this: u32, off: u32, float: bool, v: u32) -> u32 {
        if locked(h, this) {
            return 0x16;
        }
        h.write(this + off, if float { f32_move(v) } else { v });
        0
    }

    fn locked(h: &mut impl Host, this: u32) -> bool {
        h.read::<u32>(this + 0x1c) & 0x80 != 0
    }

    /// `FUN_004c06e0(this; on)` (`ret 4`) -> 0 or 0x16: bit 6 of `+0x1c`
    /// set for a non-zero byte, else cleared.
    pub fn locked_set_flag40(h: &mut impl Host, this: u32, on: u32) -> u32 {
        let f: u32 = h.read(this + 0x1c);
        if f & 0x80 != 0 {
            return 0x16;
        }
        h.write(
            this + 0x1c,
            if on & 0xff != 0 { f | 0x40 } else { f & !0x40 },
        );
        0
    }

    /// `FUN_004c0750(this; v)` (`ret 4`) -> 0, 5 for `v` above 1, or 0x16:
    /// the byte at `+1`.
    pub fn locked_set_byte1(h: &mut impl Host, this: u32, v: u32) -> u32 {
        if locked(h, this) {
            return 0x16;
        }
        if v > 1 {
            return 5;
        }
        h.write(this + 1, v as u8);
        0
    }

    /// An [`OWNER_SETTERS`] setter `(this; owner)` -> 0, or 5 when `+0x18`
    /// already holds an owner other than `this` (one of `kind` names its
    /// object at `+0`).
    pub fn set_owner(h: &mut impl Host, this: u32, owner: u32, kind: u8) -> u32 {
        let cur: u32 = h.read(this + 0x18);
        if cur != 0 {
            let of = if h.read::<u8>(cur + 4) == kind {
                h.read(cur)
            } else {
                0
            };
            if of != this {
                return 5;
            }
        }
        h.write(this + 0x18, owner);
        0
    }

    /// `FUN_004e0700(this; key, p)` (`ret 8`): the tree node of `key`
    /// (`Phyre_PRedBlackTree_find_004da4e0(this; &key)`, the key handed
    /// in `slot`, the original's own argument slot); with a node, word
    /// `(p - key[2]) >> 5` (arithmetic) of the node's block `+0x14`, else
    /// 0.
    pub fn node_block_entry(h: &mut impl Host, this: u32, key: u32, p: u32, slot: u32) -> u32 {
        h.write(slot, key);
        let node = this_call(h, 0x004d_a4e0, this, &[slot]);
        if node == 0 || node.wrapping_add(0x10) == 0 {
            return 0;
        }
        let base: u32 = h.read(key + 8);
        let i = (p.wrapping_sub(base) as i32) >> 5;
        let block: u32 = h.read(node + 0x14);
        block.wrapping_add((i as u32).wrapping_mul(4))
    }

    /// `FUN_004fb3c0(this; a)` (`ret 4`) -> this: the
    /// `PDynamicMeshInstance` base (`0x004fa420(this; a)`), the
    /// `PModifierNetworkDynamicMeshInstance` vtable, `+0xc` and `+0x10`
    /// zeroed.
    pub fn modifier_instance_ctor(h: &mut impl Host, this: u32, a: u32) -> u32 {
        this_call(h, 0x004f_a420, this, &[a]);
        let vt = h.rebase(0x00b2_3880);
        h.write(this, vt);
        h.write(this + 0xc, 0u32);
        h.write(this + 0x10, 0u32);
        this
    }

    /// `FUN_00563fb0(this; n)` (`ret 4`): at least one buffer; a mapped
    /// buffer (`+0x34` bit 0) released first (`0x005646d0`); a different
    /// count waits for the slots (`0x00564630`) and resizes the sharray at
    /// `+0x18` (`FUN_00563df0`), whose status it returns (0 otherwise).
    pub fn set_buffer_count(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let n = n.max(1);
        if h.read::<u8>(this + 0x34) & 1 != 0 {
            this_call(h, 0x0056_46d0, this, &[]);
        }
        let count = h.read::<u32>(this + 0x18) & 0x7fff_ffff;
        if count == n {
            return 0;
        }
        this_call(h, 0x0056_4630, this, &[]);
        this_call(h, 0x0056_3df0, this + 0x18, &[n])
    }

    /// `FUN_0056c110(a, b)` (cdecl): the sort order of the floats at
    /// `+0x28`: -1 when `b`'s is above `a`'s, 1 when below, 0 for equal
    /// or unordered.
    pub fn compare_depth(h: &mut impl Host, a: u32, b: u32) -> u32 {
        let x = x87::load(h.read(a + 0x28));
        let y = x87::load(h.read(b + 0x28));
        if y > x { 0xffff_ffff } else { u32::from(y < x) }
    }

    /// `FUN_0056df40(this)` -> 0: no current frame (`+0xc0`, `+0x20`);
    /// for each of the `+0x1c` frames, `[+0xc8][i] = i + 1` and `[+0xd0][i]
    /// = -1`.
    pub fn reset_frames(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 0xc0, 0u32);
        h.write(this + 0x20, 0u32);
        let mut i = 0u32;
        while i < h.read::<u32>(this + 0x1c) {
            let next: u32 = h.read(this + 0xc8);
            h.write(next.wrapping_add(4 * i), i + 1);
            let prev: u32 = h.read(this + 0xd0);
            h.write(prev.wrapping_add(4 * i), 0xffff_ffff_u32);
            i += 1;
        }
        0
    }

    /// The frame record of `frame` (`[+0xb8] + 48 * [+0xc8][frame]`), or
    /// the status: 9 without records, 5 for an index past `+0x20`.
    fn sprite_frame(h: &mut impl Host, this: u32, frame: u32) -> Result<u32, u32> {
        let records: u32 = h.read(this + 0xb8);
        if records == 0 {
            return Err(9);
        }
        let table: u32 = h.read(this + 0xc8);
        let idx: u32 = h.read(table.wrapping_add(frame.wrapping_mul(4)));
        if idx >= h.read::<u32>(this + 0x20) {
            return Err(5);
        }
        Ok(records.wrapping_add(idx.wrapping_mul(48)))
    }

    /// A [`SPRITE_SETTERS`] setter `(this; frame, v...)` -> 0, 9 or 5.
    pub fn sprite_set(h: &mut impl Host, this: u32, frame: u32, off: u32, v: &[u32]) -> u32 {
        match sprite_frame(h, this, frame) {
            Ok(rec) => {
                for (i, &x) in (0u32..).zip(v) {
                    h.write(rec + off + 4 * i, f32_move(x));
                }
                0
            }
            Err(e) => e,
        }
    }

    /// `FUN_0056e7b0(this; frame, src)` (`ret 8`) -> 0, 9 or 5: the
    /// frame's 48-byte record copied from `src`.
    pub fn sprite_set_record(h: &mut impl Host, this: u32, frame: u32, src: u32) -> u32 {
        match sprite_frame(h, this, frame) {
            Ok(rec) => {
                for i in 0..12 {
                    let v: u32 = h.read(src + 4 * i);
                    h.write(rec + 4 * i, v);
                }
                0
            }
            Err(e) => e,
        }
    }

    /// `FUN_0056ebf0(this; buffer, count)` (`ret 8`) -> 0: an owned old
    /// block other than `buffer` handed to
    /// `Phyre_FreeOwnedBuffers20_0056a700(old, count)`, then the external
    /// `[count | 0x80000000, buffer]`; nothing for a negative count.
    pub fn set_external_release(h: &mut impl Host, this: u32, buffer: u32, count: u32) -> u32 {
        if count & 0x8000_0000 != 0 {
            return 0;
        }
        let old: u32 = h.read(this + 4);
        if old != buffer {
            let header: u32 = h.read(this);
            if header & 0x8000_0000 == 0 {
                cdecl(h, 0x0056_a700, &[old, header & 0x7fff_ffff]);
            }
        }
        h.write(this + 4, buffer);
        h.write(this, count | 0x8000_0000);
        0
    }

    /// `FUN_00584e80(this)`: the 8 `0x17c`-byte elements at `+0x2c`
    /// destroyed by `0x004bc000` (`__ehvec_dtor`), its `eax`.
    pub fn destroy_elements(h: &mut impl Host, this: u32) -> u32 {
        let dtor = h.rebase(0x004b_c000);
        h.call_sym(EHVEC_DTOR, Conv::Stdcall, &[this + 0x2c, 0x17c, 8, dtor])
    }

    /// `FUN_0058aaa0(mode)` (cdecl): the jump table at `0x0058aaec`.
    #[must_use]
    pub fn map_mode(mode: u32) -> u32 {
        match mode {
            1 => 2,
            2 => 3,
            3 => 4,
            4 => 5,
            5 => 7,
            6 => 8,
            7 => 6,
            _ => 1,
        }
    }

    /// `PTexture3DD3D11_bind_0058db50(this)` -> 0: without a resource
    /// (the first word of the sharray at `+0x28`),
    /// `PTexture3DD3D11_create(this; 0, 1, bit 10 of +0x14, 0)`; then
    /// `+0x64` = 1. (An empty sharray reads address 0, as the original.)
    pub fn texture3d_bind(h: &mut impl Host, this: u32) -> u32 {
        let count = h.read::<u32>(this + 0x28) & 0x7fff_ffff;
        let first = if count > 1 {
            h.read(this + 0x2c)
        } else if count != 0 {
            this + 0x2c
        } else {
            0
        };
        if h.read::<u32>(first) == 0 {
            let srgb = (h.read::<u32>(this + 0x14) >> 10) & 1;
            this_call(h, 0x0059_03a0, this, &[0, 1, srgb, 0]);
        }
        h.write(this + 0x64, 1u32);
        0
    }

    /// `FUN_00592c10(this; topology, start, count, instances)` (`ret
    /// 0x10`) -> `al`: nothing (1) for no vertices or instances; else the
    /// D3D topology (`PRendererD3D11_mapPrimitiveTopology`, 0 fails: `al`
    /// 0), `IASetPrimitiveTopology` and `DrawInstanced(count, instances,
    /// start, 0)` on the context `+0x70` (COM slots 24 and 21).
    pub fn draw_instanced(
        h: &mut impl Host,
        this: u32,
        topology: u32,
        start: u32,
        count: u32,
        instances: u32,
    ) -> u8 {
        if count == 0 || instances == 0 {
            return 1;
        }
        let t = cdecl(h, 0x0058_ab60, &[topology]);
        if t == 0 {
            return 0;
        }
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt + 0x60);
        h.call(f, Conv::Stdcall, &[ctx, t]);
        let ctx: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt + 0x54);
        h.call(f, Conv::Stdcall, &[ctx, count, instances, start, 0]);
        1
    }

    /// `FUN_004985b0(_; renderer, _, mesh, segments, desc, a, b)` (`ret
    /// 0x1c`): the vertex count (the descriptor's `+8`, else the first
    /// segment's non-zero `+4` among the 64-byte segments of `segments`),
    /// then `FUN_00592de0(renderer; [mesh + 0x10], n, desc, a, b)` with a
    /// descriptor, else `FUN_00592c10(renderer; [mesh + 0x10], 0, n, b)`.
    pub fn draw_segment(
        h: &mut impl Host,
        renderer: u32,
        mesh: u32,
        segments: u32,
        desc: u32,
        a: u32,
        b: u32,
    ) -> u32 {
        let n = if desc != 0 {
            h.read(desc + 8)
        } else {
            let count = h.read::<u32>(segments) & 0x7fff_ffff;
            let data: u32 = h.read(segments + 4);
            let mut n = 0;
            for i in 0..count {
                n = h.read(data.wrapping_add(4 + 64 * i));
                if n != 0 {
                    break;
                }
            }
            n
        };
        let topology: u32 = h.read(mesh + 0x10);
        if desc != 0 {
            this_call(h, 0x0059_2de0, renderer, &[topology, n, desc, a, b])
        } else {
            this_call(h, 0x0059_2c10, renderer, &[topology, 0, n, b])
        }
    }

    /// The list prunes `FUN_00432680` / `FUN_004326f0` `()`: every node of
    /// the list at `head` (link `+link`) whose byte `+flag` is set is
    /// unlinked (found by its link from the cached head, re-read after
    /// each unlink), then `Phyre_PFreeList_reset` of `a` and of `b` (the
    /// second's `eax`).
    pub fn prune_list(h: &mut impl Host, head: u32, link: u32, flag: u32, a: u32, b: u32) -> u32 {
        let head = h.rebase(head);
        let mut cached: u32 = h.read(head);
        let mut node = cached;
        while node != 0 {
            if h.read::<u8>(node + flag) == 0 {
                node = h.read(node + link);
                continue;
            }
            let target = node + link;
            node = h.read(target);
            let mut prev = head;
            let mut cur = cached;
            while cur != 0 {
                if cur + link == target {
                    let next: u32 = h.read(cur + link);
                    h.write(prev, next);
                    h.write(cur + link, 0u32);
                    cached = h.read(head);
                    break;
                }
                prev = cur + link;
                cur = h.read(prev);
            }
        }
        let a = h.rebase(a);
        let b = h.rebase(b);
        this_call(h, FREE_LIST_RESET, a, &[]);
        this_call(h, FREE_LIST_RESET, b, &[])
    }

    /// `PUtilitySprite`'s destructor `FUN_00433dd0(this)`: its vtable, the
    /// mesh at `+0x10` destroyed (`Phyre_PGeometry_PMesh_dtor`), then the
    /// `PUtility` part (vtable, out of the utility list). `eax` unused
    /// (the deleting destructor and the atexit teardown drop it).
    pub fn utility_sprite_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(0x00b0_e140);
        h.write(this, vt);
        this_call(h, 0x0043_3af0, this + 0x10, &[]);
        let vt = h.rebase(crate::m5p_render_a::PUTILITY_VTABLE);
        h.write(this, vt);
        let head = h.rebase(crate::m5p_render_a::PUTILITY_LIST);
        if h.read::<u32>(head) == 0 {
            return;
        }
        let mut link = head;
        loop {
            let cur: u32 = h.read(link);
            if cur == this {
                break;
            }
            link = cur + 4;
            if h.read::<u32>(link) == 0 {
                break;
            }
        }
        if h.read::<u32>(link) != 0 {
            let next: u32 = h.read(this + 4);
            h.write(link, next);
            h.write(this + 4, 0u32);
        }
    }

    /// `PBuffer_unmapNextSlot_004350b0(this; a)` (`ret 4`): the next slot
    /// (`+0x10` + 1, 0 past the count `+0x18`) handed to
    /// `PIndexBufferD3D11_unmapCopy(this; slot, a)`; on 0, `+0x10` = the
    /// slot and the byte `+0x14` = 0. Its `eax`.
    pub fn unmap_next_slot(h: &mut impl Host, this: u32, a: u32) -> u32 {
        let next = h.read::<u32>(this + 0x10).wrapping_add(1);
        let count = h.read::<u32>(this + 0x18) & 0x7fff_ffff;
        let slot = if next == count { 0 } else { next };
        let r = this_call(h, 0x0056_4cb0, this, &[slot, a]);
        if r == 0 {
            h.write(this + 0x10, slot);
            h.write(this + 0x14, 0u8);
        }
        r
    }

    /// The default TTY callback's guard: the object `0x00c902d8` built
    /// once (vtable, atexit `0x00af9e20`).
    fn tty_default(h: &mut impl Host) {
        let guard = h.rebase(0x00c9_02dc);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let obj = h.rebase(0x00c9_02d8);
            let vt = h.rebase(0x00b0_e158);
            h.write(obj, vt);
            let at = h.rebase(0x00af_9e20);
            cdecl(h, ATEXIT, &[at]);
        }
    }

    /// `FUN_00435270()` -> `0x00c902e0`, the current TTY callback cell,
    /// set to the default callback the first time.
    pub fn tty_current(h: &mut impl Host) -> u32 {
        let guard = h.rebase(0x00c9_02e4);
        let cell = h.rebase(0x00c9_02e0);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            tty_default(h);
            let obj = h.rebase(0x00c9_02d8);
            h.write(cell, obj);
        }
        cell
    }

    /// `PTTYCallback`'s scalar deleting destructor `(this; flags)` (`ret
    /// 4`) -> this: when it is the current callback (`FUN_00435270()`),
    /// the default takes its place.
    pub fn tty_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(0x00b0_e158);
        h.write(this, vt);
        let cell = cdecl(h, 0x0043_5270, &[]);
        if h.read::<u32>(cell) == this {
            tty_default(h);
            let cell = cdecl(h, 0x0043_5270, &[]);
            let obj = h.rebase(0x00c9_02d8);
            h.write(cell, obj);
        }
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `FUN_00463cb0(this; header)` (`ret 4`) -> this: a cluster texture
    /// iterator over the list at `header + 0x1c` (the first node, 0 for an
    /// empty list), the type `0x00ca3ef0`, moved to its first match
    /// (`Phyre_PClusterHeader_iterSeek`).
    pub fn texture_iter_ctor(h: &mut impl Host, this: u32, header: u32) -> u32 {
        let list = header.wrapping_add(0x1c);
        for i in 0..5 {
            h.write(this + 4 * i, 0u32);
        }
        h.write(this + 0x14, list);
        let first: u32 = h.read(list);
        h.write(this + 0x18, if first == list { 0 } else { first });
        let ty = h.rebase(0x00ca_3ef0);
        h.write(this + 0x1c, ty);
        this_call(h, 0x0046_5010, this, &[]);
        this
    }

    /// `FUN_0048c1a0(this; i)` (`ret 4`): word `i` of the array (0 past
    /// the count; one element is stored inline at `+4`).
    pub fn array_word(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let count = h.read::<u32>(this) & 0x7fff_ffff;
        if i >= count {
            return 0;
        }
        let base = if count > 1 {
            h.read(this + 4)
        } else {
            this + 4
        };
        h.read(base.wrapping_add(i.wrapping_mul(4)))
    }

    /// The `PNamedSemanticDescriptorForDynamicType<PContextSwitch>`
    /// getter `FUN_004d0e20()` -> the descriptor `0x00ca4f0c`, built once.
    pub fn context_switch_semantic(h: &mut impl Host) -> u32 {
        let guard = h.rebase(0x00ca_4f40);
        let d = h.rebase(0x00ca_4f0c);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let name = h.rebase(0x00b1_f83c);
            let f1 = h.rebase(0x004d_3b30);
            let f2 = h.rebase(0x004d_3d10);
            this_call(h, 0x005a_0200, d, &[name, 0x18, 4, f1, f2]);
            let vt = h.rebase(0x00b1_f970);
            h.write(d, vt);
            let at = h.rebase(0x00af_e8b0);
            cdecl(h, ATEXIT, &[at]);
        }
        d
    }

    /// `FUN_004ff0a0()`: the static event owner `0x00c94ee4`, built once
    /// (`PhyreCtor_eventOwner`, atexit `0x00afb700`), then `0x00431950`
    /// on it, its `eax`.
    pub fn static_event_owner(h: &mut impl Host) -> u32 {
        let guard = h.rebase(0x00c9_4efc);
        let obj = h.rebase(0x00c9_4ee4);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            this_call(h, 0x0043_15e0, obj, &[]);
            let at = h.rebase(0x00af_b700);
            cdecl(h, ATEXIT, &[at]);
        }
        this_call(h, 0x0043_1950, obj, &[])
    }
}
