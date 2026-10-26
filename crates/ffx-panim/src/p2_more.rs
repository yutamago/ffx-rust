//! The rest of list `p2-anim` (continuation round): the
//! `PAnimationClipBinding` descriptor header's callbacks and the clip's
//! fix-up after loading, `PAnimationNetworkInstance`'s constructor,
//! destructor and thread pool, `PAnimatableComponent`'s constructor and
//! destructor, and a 3x3 copy out of a 4x4 matrix.
//!
//! The originals' SEH frames only unwind half-built members; nothing on
//! these paths throws: dropped.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre::PhyreMalloc(n)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb40;
/// `Phyre::PFreeList::PFreeList(this; size, count, align, name)`
/// (thiscall, four words) -> `this`.
pub const FREE_LIST_CTOR: u32 = 0x0049_c480;
/// `PTypeDispenser::GetType<PUInt16>()` (cdecl).
pub const TYPE_U16: u32 = 0x0043_7e70;
/// The `PDeleteChildrenVisitor` the descriptor header's destruct callback
/// builds: its vtable, and the getter of its one word (cdecl, `()`).
pub const DELETE_CHILDREN_VTABLE: u32 = 0x00b1_fac0;
pub const DELETE_CHILDREN_ARG: u32 = 0x0059_e530;
/// `PAnimationClipBinding`'s fix-up (`0x00511980`, thiscall `(this;
/// entry)`, always 0).
pub const CLIP_BINDING_FIXUP: u32 = 0x0051_1980;
/// The `PClassDescriptor`s of `PAnimationClipBindingChannelMap` and
/// `PAnimationClipBindingDataBlockCache`.
pub const CHANNEL_MAP_DESC: u32 = 0x00ca_a428;
pub const DATA_BLOCK_CACHE_DESC: u32 = 0x00ca_a4c0;
/// `PAnimationNetworkInstance::InitializeThreadPool`'s pool (a global
/// pointer) and the pool's name, `"ANIThreadPoolJobs"`.
pub const THREAD_POOL: u32 = 0x00ca_cd10;
pub const THREAD_POOL_NAME: u32 = 0x00b2_ab8c;
/// `PAnimationNetworkInstance`'s vtable.
pub const NETWORK_INSTANCE_VTABLE: u32 = 0x00b2_a640;
/// `PAnimatableComponent`'s vtable and its parts' constructors and
/// destructors (thiscall).
pub const ANIMATABLE_VTABLE: u32 = 0x00b2_b5b8;
/// The `PObjectRef` at `+4`: its init (`(ref; descriptor)`) and release,
/// and the descriptor it is given.
pub const OBJECT_REF_INIT: u32 = 0x0044_d6a0;
pub const OBJECT_REF_RELEASE: u32 = 0x0044_d730;
pub const OBJECT_REF_DESC: u32 = 0x00ca_da40;
pub const WEIGHTED_BLENDER_CTOR: u32 = 0x0052_d660;
pub const WEIGHTED_BLENDER_DTOR: u32 = 0x0052_d6b0;
pub const TARGET_BLENDER_CTOR: u32 = 0x0053_0dd0;
pub const TARGET_BLENDER_DTOR: u32 = 0x0053_0e40;
pub const NETWORK_INSTANCE_CTOR: u32 = 0x0053_4450;
pub const NETWORK_INSTANCE_DTOR: u32 = 0x0053_4b10;
/// `Phyre_DeleteArray4c_00540b80` (cdecl `(data, n)`).
pub const DELETE_ARRAY_4C: u32 = 0x0054_0b80;

/// What a `shr eax, 0x1f; not al` leaves in `eax` for the word `w`.
const fn owned_test(w: u32) -> u32 {
    (w >> 31) ^ 0xff
}

#[allow(clippy::many_single_char_names)]
pub mod bridge {
    //! The originals over game memory.

    use super::{
        ANIMATABLE_VTABLE, CHANNEL_MAP_DESC, CLIP_BINDING_FIXUP, Conv, DATA_BLOCK_CACHE_DESC,
        DELETE_ARRAY_4C, DELETE_CHILDREN_ARG, DELETE_CHILDREN_VTABLE, FREE_LIST_CTOR, Host, Mem,
        NETWORK_INSTANCE_CTOR, NETWORK_INSTANCE_DTOR, NETWORK_INSTANCE_VTABLE, OBJECT_REF_DESC,
        OBJECT_REF_INIT, OBJECT_REF_RELEASE, PHYRE_FREE, PHYRE_MALLOC, TARGET_BLENDER_CTOR,
        TARGET_BLENDER_DTOR, THREAD_POOL, THREAD_POOL_NAME, TYPE_U16, WEIGHTED_BLENDER_CTOR,
        WEIGHTED_BLENDER_DTOR, owned_test,
    };

    /// A virtual call: slot `off` of `obj`'s vtable (thiscall).
    fn virtual_call(h: &mut impl Host, obj: u32, off: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt.wrapping_add(off));
        h.call(f, Conv::Thiscall(obj), args)
    }

    /// The binding header's `+0x94` handler `(header, stream)`
    /// (`0x00510f90`, cdecl): unless the stream's `+0xc` says to skip
    /// (`al` set: that call's `eax`), three blocks of the header through
    /// the stream: the `u16` keys (`+8`, `(h6 + 3) & !3` of them), the
    /// channel maps after them (`h2 + h4`), and the data-block caches
    /// (`h2`) at the next 4-byte boundary; the second object's `+0xc`
    /// gives the caches' alignment. `eax` is the last call's.
    pub fn header_visit(h: &mut impl Host, header: u32, stream: u32) -> u32 {
        let skip = virtual_call(h, stream, 0xc, &[]);
        if skip & 0xff != 0 {
            return skip;
        }
        let other: u32 = h.read(stream + 4);
        let h4 = u32::from(h.read::<u16>(header + 4));
        let h2 = u32::from(h.read::<u16>(header + 2));
        let keys = (u32::from(h.read::<u16>(header + 6)) + 3) & !3;
        let tail = header.wrapping_add((keys + (h4 + h2) * 2 + 4) * 2);
        let vt: u32 = h.read(stream);
        let t = h.call_sym(TYPE_U16, Conv::Cdecl, &[]);
        let f: u32 = h.read(vt + 4);
        h.call(f, Conv::Thiscall(stream), &[header + 8, keys, t, 0]);
        let maps = header.wrapping_add((keys + 4) * 2);
        let d = h.rebase(CHANNEL_MAP_DESC);
        let vt: u32 = h.read(stream);
        let f: u32 = h.read(vt + 4);
        h.call(f, Conv::Thiscall(stream), &[maps, h4 + h2, d, 0]);
        let d = h.rebase(DATA_BLOCK_CACHE_DESC);
        let align = virtual_call(h, other, 0xc, &[d]);
        let h4 = u32::from(h.read::<u16>(header + 4));
        let h2b = u32::from(h.read::<u16>(header + 2));
        let keys = (u32::from(h.read::<u16>(header + 6)) + 3) & !3;
        let size = (keys + (h2b + h4) * 2) * 2;
        let end =
            header.wrapping_add(size.wrapping_add(7).wrapping_add(align) & !align.wrapping_sub(1));
        let vt: u32 = h.read(stream);
        let caches = tail.wrapping_add(tail.wrapping_neg() & 3);
        let f: u32 = h.read(vt + 8);
        h.call(f, Conv::Thiscall(stream), &[caches, h2, d, 0, end])
    }

    /// The binding header's `+0x3c` callback `(this; header, stream)`
    /// (`0x00511ac0`, thiscall): the handler at `this + 0x94` (cdecl), its
    /// `eax`.
    pub fn header_slot3c(h: &mut impl Host, this: u32, header: u32, stream: u32) -> u32 {
        let f: u32 = h.read(this + 0x94);
        h.call(f, Conv::Cdecl, &[header, stream])
    }

    /// The binding header's destruct callback `(this; object)`
    /// (`0x00511130`, thiscall): a `PDeleteChildrenVisitor` `{vtable,
    /// 0x0059e530()}` in `frame` (two words), handed to `this`'s `+0x3c`
    /// with the object; `al` = 1 over that call's `eax`.
    pub fn header_destruct(h: &mut impl Host, this: u32, object: u32, frame: u32) -> u32 {
        let arg = h.call_sym(DELETE_CHILDREN_ARG, Conv::Thiscall(this), &[]);
        h.write(frame + 4, arg);
        let vt = h.rebase(DELETE_CHILDREN_VTABLE);
        h.write(frame, vt);
        let eax = virtual_call(h, this, 0x3c, &[object, frame]);
        crate::al_one(eax)
    }

    /// `PAnimationClipBinding`'s fix-up `(this; entry)` (`0x00511980`,
    /// thiscall, always 0): `this`'s size word (`u16` at `+0`) from the
    /// entry's counts, then each of the entry's `[+4] & 0x7fffffff`
    /// records (16 bytes, after `this`'s key blocks at a 4-byte boundary)
    /// gets its target's two words: the record's `u16` at `+0xc` indexes
    /// the entry's table (`+8`); words `[[t + 0x24] + 8]` and `[t + 0x2c]`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn clip_binding_fixup(m: &mut impl Mem, this: u32, entry: u32) -> u32 {
        let n = m.read::<u32>(entry + 4) & 0x7fff_ffff;
        let c = m.read::<u16>(entry + 0xc);
        let h6 = m.read::<u16>(this + 6);
        let h4 = u32::from(m.read::<u16>(this + 4));
        let keys = h6.wrapping_add(3) & 0xfffc;
        let size = c
            // the original adds bx: the count's low 16 bits
            .wrapping_add(n as u16)
            .wrapping_mul(2)
            .wrapping_add(keys)
            .wrapping_mul(2)
            .wrapping_add(0x17)
            & 0xfff0;
        let h2 = u32::from(m.read::<u16>(this + 2));
        m.write(this, size);
        let words = (u32::from(h6) + 3) & !3;
        let p = this.wrapping_add((words + (h4 + h2) * 2 + 4) * 2);
        let mut rec = p.wrapping_add(p.wrapping_neg() & 3);
        for _ in 0..n {
            let i = u32::from(m.read::<u16>(rec + 0xc));
            // the table word is read again for every record
            let table: u32 = m.read(entry + 8);
            let t: u32 = m.read(table.wrapping_add(i * 4));
            let a: u32 = m.read(t + 0x24);
            let v: u32 = m.read(a + 8);
            m.write(rec, v);
            let v: u32 = m.read(t + 0x2c);
            m.write(rec + 4, v);
            rec = rec.wrapping_add(0x10);
        }
        0
    }

    /// `PAnimationClip`'s fix-objects-after-loading callback `(object)`
    /// (`0x00516370`, stdcall): the binding fix-up on each of the clip's
    /// `[+0x3c]` entries at `[+0x30]` (stride `[[+0x2c] + 0x1c]`) whose
    /// first word is set; the first non-zero result, else 0.
    pub fn clip_fix_after_loading(h: &mut impl Host, object: u32) -> u32 {
        let mut entry: u32 = h.read(object + 0x30);
        let mut left: u32 = h.read(object + 0x3c);
        let info: u32 = h.read(object + 0x2c);
        let stride: u32 = h.read(info + 0x1c);
        while left != 0 {
            let binding: u32 = h.read(entry);
            left -= 1;
            if binding != 0 {
                let r = h.call_sym(CLIP_BINDING_FIXUP, Conv::Thiscall(binding), &[entry]);
                if r != 0 {
                    return r;
                }
            }
            entry = entry.wrapping_add(stride);
        }
        0
    }

    /// `PAnimationNetworkInstance`'s constructor `(this)` (`0x00534450`)
    /// -> `this`: its vtable, the other 14 words 0.
    pub fn network_instance_ctor(m: &mut impl Mem, this: u32) -> u32 {
        let vt = m.rebase(NETWORK_INSTANCE_VTABLE);
        m.write(this, vt);
        for i in 1..15 {
            m.write(this + 4 * i, 0u32);
        }
        this
    }

    /// An owned `PArray` `{count | not-owned bit 31, data}` at `at`: the
    /// data freed when owned and set; both words cleared. Returns the
    /// `eax` the original leaves (`PhyreFree`'s, the data word 0, or the
    /// owned test's).
    fn free_array(h: &mut impl Host, at: u32) -> u32 {
        let count: u32 = h.read(at);
        let mut eax = owned_test(count);
        if eax & 1 != 0 {
            let data: u32 = h.read(at + 4);
            eax = data;
            if data != 0 {
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(at + 4, 0u32);
        h.write(at, 0u32);
        eax
    }

    /// `PAnimationNetworkInstance`'s destructor `(this)` (`0x00534b10`):
    /// its vtable, `PhyreFree([+0x34])`, `+0x14` = 0, then its owned arrays
    /// at `+0x2c`, `+0x20`, `+0xc` and `+4`. `eax` as the original leaves
    /// it.
    pub fn network_instance_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(NETWORK_INSTANCE_VTABLE);
        h.write(this, vt);
        let p: u32 = h.read(this + 0x34);
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        h.write(this + 0x14, 0u32);
        let mut eax = 0;
        for at in [0x2c, 0x20, 0xc, 4] {
            eax = free_array(h, this + at);
        }
        eax
    }

    /// `PAnimationNetworkInstance::InitializeThreadPool()` (`0x00539640`):
    /// once, a `PFreeList` of 0x80 16-byte jobs (align 4,
    /// `"ANIThreadPoolJobs"`) in a 0x18-byte block; 0xd when the block
    /// could not be had, else 0.
    pub fn initialize_thread_pool(h: &mut impl Host) -> u32 {
        let pool = h.rebase(THREAD_POOL);
        if h.read::<u32>(pool) != 0 {
            return 0;
        }
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x18]);
        let list = if p == 0 {
            0
        } else {
            let name = h.rebase(THREAD_POOL_NAME);
            h.call_sym(FREE_LIST_CTOR, Conv::Thiscall(p), &[0x10, 0x80, 4, name])
        };
        h.write(pool, list);
        if list == 0 { 0xd } else { 0 }
    }

    /// `PAnimatableComponent`'s constructor `(this)` (`0x0053e720`) ->
    /// `this`: the object ref at `+4`, its vtable, `+0x10..+0x18` = 0, the
    /// weighted blender (`+0x1c`), target blender (`+0x38`) and network
    /// instance (`+0x48`).
    pub fn animatable_ctor(h: &mut impl Host, this: u32) -> u32 {
        let d = h.rebase(OBJECT_REF_DESC);
        h.call_sym(OBJECT_REF_INIT, Conv::Thiscall(this + 4), &[d]);
        let vt = h.rebase(ANIMATABLE_VTABLE);
        h.write(this, vt);
        h.write(this + 0x10, 0u32);
        h.write(this + 0x14, 0u32);
        h.write(this + 0x18, 0u32);
        h.call_sym(WEIGHTED_BLENDER_CTOR, Conv::Thiscall(this + 0x1c), &[]);
        h.call_sym(TARGET_BLENDER_CTOR, Conv::Thiscall(this + 0x38), &[]);
        h.call_sym(NETWORK_INSTANCE_CTOR, Conv::Thiscall(this + 0x48), &[]);
        this
    }

    /// `PAnimatableComponent`'s destructor `(this)` (`0x0053ebd0`): its
    /// vtable, the parts' destructors in reverse, the owned array of
    /// 0x4c-byte elements at `+0x14` deleted, then the object ref's
    /// release (its `eax`).
    pub fn animatable_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(ANIMATABLE_VTABLE);
        h.write(this, vt);
        h.call_sym(NETWORK_INSTANCE_DTOR, Conv::Thiscall(this + 0x48), &[]);
        h.call_sym(TARGET_BLENDER_DTOR, Conv::Thiscall(this + 0x38), &[]);
        h.call_sym(WEIGHTED_BLENDER_DTOR, Conv::Thiscall(this + 0x1c), &[]);
        let count: u32 = h.read(this + 0x14);
        if owned_test(count) & 1 != 0 {
            let data: u32 = h.read(this + 0x18);
            h.call_sym(DELETE_ARRAY_4C, Conv::Cdecl, &[data, count & 0x7fff_ffff]);
        }
        h.write(this + 0x18, 0u32);
        h.write(this + 0x14, 0u32);
        h.call_sym(OBJECT_REF_RELEASE, Conv::Thiscall(this + 4), &[])
    }

    /// The upper 3x3 of a 4x4 matrix `(this; src)` (`0x00531a40`,
    /// thiscall) -> `this`: rows of 16 bytes, three floats each copied
    /// through the x87 stack (a signalling NaN comes out quiet), row by
    /// row (each row read whole before it is written).
    pub fn upper3x3(m: &mut impl Mem, this: u32, src: u32) -> u32 {
        for row in 0..3 {
            let s = src.wrapping_add(row * 0x10);
            let d = this.wrapping_add(row * 0x10);
            let v: [f32; 3] = [m.read(s), m.read(s + 4), m.read(s + 8)];
            for (i, x) in (0u32..).zip(v) {
                m.write(d + 4 * i, ffx_bridge::x87::round_trip(x));
            }
        }
        this
    }
}
