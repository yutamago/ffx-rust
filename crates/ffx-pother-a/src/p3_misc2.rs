//! The list's second round, second batch: arrays (push with a new
//! buffer, resize, element releases), the scheduler's class descriptor,
//! constructor and destructor, the gameplay components' and LOD objects'
//! constructors, destructors and callbacks, the named semantic
//! descriptors' getters.
//!
//! An array here is `{count | bit 31 (not owned), data}`; the originals
//! test bit 31 with `shr 31; not al; test al, 1`. Exception frames and
//! stack cookies are left out: nothing on these paths throws.

use crate::{cdecl, free, thiscall};
use ffx_bridge::x87;
use ffx_graphic::host::Host;

/// `Phyre_PPhysics_PPhysicsPlaneBullet_vftable`,
/// `Phyre_PPhysics_PPhysicsShapeBase_vftable`.
pub const PLANE_BULLET_VFTABLE: u32 = 0x00b3_a9d8;
pub const SHAPE_BASE_VFTABLE: u32 = 0x00b3_9508;
/// `0x0044d6a0` (thiscall `(ref, descriptor)`), `0x0044d730` (thiscall
/// `(ref)`).
pub const REF_INIT: u32 = 0x0044_d6a0;
pub const REF_RELEASE: u32 = 0x0044_d730;
/// `0x005d3860` (thiscall `(this)`): `{0, 0, PBase's descriptor}`.
pub const BASE_REF_CTOR: u32 = 0x005d_3860;
/// The descriptors the receiver data (`0x00551da0`) and the attachable
/// component (`0x00559530`) initialise their references with.
pub const RECEIVER_DATA_DESCRIPTOR: u32 = 0x00ca_f388;
pub const ATTACHABLE_DESCRIPTOR: u32 = 0x00cb_0ac0;
/// `Phyre_PNamedSemanticDescriptor_PNamedSemanticDescriptor` (thiscall
/// `(this, a, b, c, d, e)`).
pub const NAMED_SEMANTIC_CTOR: u32 = 0x005a_0200;
/// `atexit` (cdecl `(f)`).
pub const ATEXIT: u32 = 0x0094_9207;

/// A named semantic descriptor made once by its getter: the once flag
/// (bit 0), the object, the constructor's five words (`args[0]`, `[3]` and
/// `[4]` are addresses), the vtable stored after it and its `atexit`
/// destructor.
#[derive(Debug, Clone, Copy)]
pub struct Semantic {
    pub once: u32,
    pub object: u32,
    pub args: [u32; 5],
    pub vtable: u32,
    pub dtor: u32,
}

/// `0x00544d70` and `0x00544e00` (the same code): the locator shape type's.
pub const LOCATOR_SHAPE_SEMANTIC: Semantic = Semantic {
    once: 0x00ca_e618,
    object: 0x00ca_e5e4,
    args: [0x00b2_bef8, 0xc, 4, 0x0054_5aa0, 0x0054_5e60],
    vtable: 0x00b2_bf10,
    dtor: 0x00b0_19a0,
};
/// `0x0054ce90`: the trigger type's.
pub const TRIGGER_SEMANTIC: Semantic = Semantic {
    once: 0x00ca_ed50,
    object: 0x00ca_ed1c,
    args: [0x00b2_c5d8, 0xc, 4, 0x0054_d2b0, 0x0054_d320],
    vtable: 0x00b2_c61c,
    dtor: 0x00b0_1e10,
};

/// `0x005a29d0` (thiscall `(element)`): see [`bridge::lod_element_dtor`].
pub const LOD_ELEMENT_DTOR: u32 = 0x005a_29d0;
/// `0x005a2700` (thiscall `(array)`): the 0x24-byte array's release.
pub const ARRAY24_RELEASE: u32 = 0x005a_2700;
/// `Phyre_PGameplay_PScriptableComponent_PScriptableComponent` (thiscall
/// `(this, descriptor)`) and the descriptor `0x00554600` passes.
pub const SCRIPTABLE_INIT: u32 = 0x0055_2c00;
pub const SCRIPTED_DESCRIPTOR: u32 = 0x00ca_f8d0;
/// `Phyre_PGameplay_PScriptedComponent_vftable`.
pub const SCRIPTED_VFTABLE: u32 = 0x00b2_d410;
/// `0x00552c60` (thiscall `(component)`): `PScriptableComponent`'s
/// destructor.
pub const SCRIPTABLE_DTOR: u32 = 0x0055_2c60;
/// `0x005d5450` (cdecl `(handle)`): releases a script handle.
pub const RELEASE_HANDLE: u32 = 0x005d_5450;
/// The constants `0x005a2520` and `0x005a2630` copy (`f32`).
pub const LIGHT_A: u32 = 0x00b1_7504;
pub const LIGHT_B: u32 = 0x00b1_f2d0;
pub const LIGHT_C: u32 = 0x00b1_f2cc;
pub const LOD_LEVEL_DEFAULT: u32 = 0x00b1_0f4c;
/// The two words `0x005a2520` stores at `+0x34`, `+0x38` (addresses).
pub const LIGHT_WORDS: [u32; 2] = [0x00c2_9794, 0x00c2_98c8];

/// `Phyre_PBase_GetClassDescriptor`, `Phyre_PNamespace_GetGlobalNamespace`
/// (cdecl `()`).
pub const PBASE_GET_DESCRIPTOR: u32 = 0x0043_5740;
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// `Phyre_PClassDescriptor_PClassDescriptor_0043b0a0` (thiscall `(d, ns,
/// name, size, align, base, flags)`).
pub const CLASS_DESCRIPTOR_INIT6: u32 = 0x0043_b0a0;
/// The descriptor's buffer setters (thiscall `(d, buffer)`).
pub const SET_DEFAULT_BUFFER: u32 = 0x0043_d510;
pub const SET_VALIDATION_BUFFER: u32 = 0x0043_d5b0;
pub const SET_WRITE_MASK_BUFFER: u32 = 0x0043_d5c0;
/// `PScheduler`'s class descriptor: name, vtable, buffers.
pub const SCHEDULER_NAME: u32 = 0x00b3_8adc;
pub const SCHEDULER_DESCRIPTOR_VFTABLE: u32 = 0x00b3_8a3c;
pub const SCHEDULER_BUFFERS: [u32; 3] = [0x00cb_db40, 0x00cb_ddd0, 0x00cb_e060];
/// `Phyre_PScripting_PScheduler_vftable`.
pub const SCHEDULER_VFTABLE: u32 = 0x00b3_8ac8;
/// `Phyre_PFreeList_PFreeList` (thiscall `(list, size, count, align,
/// name)`) and `~PFreeList` (thiscall `(list)`).
pub const FREE_LIST_INIT: u32 = 0x0049_c480;
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
/// The scheduler's free list's name.
pub const SCHEDULER_LIST_NAME: u32 = 0x00b3_8b90;
/// `PAsyncProcessBuffer`'s constructor and destructor (thiscall).
pub const ASYNC_BUFFER_CTOR: u32 = 0x005c_e4b0;
pub const ASYNC_BUFFER_DTOR: u32 = 0x005c_e550;
/// `crt_memset_thunk` (cdecl `(p, v, n)`).
pub const MEMSET: u32 = 0x0094_964c;
/// `0x005d18f0` (thiscall `(scheduler)`): the scheduler's shutdown.
pub const SCHEDULER_SHUTDOWN: u32 = 0x005d_18f0;
/// `0x005cfa60` (cdecl `(data, count)`): see [`bridge::free_pair_array`].
pub const FREE_PAIR_ARRAY: u32 = 0x005c_fa60;
/// `Phyre_PGameplay_PSpline_vftable`.
pub const SPLINE_VFTABLE: u32 = 0x00b2_c158;
/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const MALLOC: u32 = 0x0042_fb80;
/// `0x0044a470` (cdecl `(to, from, count)`): moves strings.
pub const MOVE_STRINGS: u32 = 0x0044_a470;
/// `Phyre_PString_copy_0044ae10` (thiscall `(to, &from)`).
pub const STRING_COPY: u32 = 0x0044_ae10;
/// `PArray_releaseTaggedWords_00434360` (cdecl `(data, count)`).
pub const RELEASE_TAGGED_WORDS: u32 = 0x0043_4360;

/// `1.0f`.
const ONE: u32 = 0x3f80_0000;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ARRAY24_RELEASE, ASYNC_BUFFER_CTOR, ASYNC_BUFFER_DTOR, ATEXIT, ATTACHABLE_DESCRIPTOR,
        BASE_REF_CTOR, CLASS_DESCRIPTOR_INIT6, FREE_LIST_DTOR, FREE_LIST_INIT, FREE_PAIR_ARRAY,
        GLOBAL_NAMESPACE, Host, LIGHT_A, LIGHT_B, LIGHT_C, LIGHT_WORDS, LOD_LEVEL_DEFAULT, MALLOC,
        MEMSET, MOVE_STRINGS, NAMED_SEMANTIC_CTOR, ONE, PBASE_GET_DESCRIPTOR, PLANE_BULLET_VFTABLE,
        RECEIVER_DATA_DESCRIPTOR, REF_INIT, REF_RELEASE, RELEASE_HANDLE, RELEASE_TAGGED_WORDS,
        SCHEDULER_BUFFERS, SCHEDULER_DESCRIPTOR_VFTABLE, SCHEDULER_LIST_NAME, SCHEDULER_NAME,
        SCHEDULER_SHUTDOWN, SCHEDULER_VFTABLE, SCRIPTABLE_DTOR, SCRIPTABLE_INIT,
        SCRIPTED_DESCRIPTOR, SCRIPTED_VFTABLE, SET_DEFAULT_BUFFER, SET_VALIDATION_BUFFER,
        SET_WRITE_MASK_BUFFER, SHAPE_BASE_VFTABLE, SPLINE_VFTABLE, STRING_COPY, Semantic, cdecl,
        free, thiscall, x87,
    };

    /// Whether an array word's bit 31 is clear (the array owns its data).
    fn owned(w: u32) -> bool {
        w >> 31 == 0
    }

    /// The word at `at` freed unless its bit 0 is set.
    fn free_unless_tagged(h: &mut impl Host, at: u32) {
        let w: u32 = h.read(at);
        if w & 1 == 0 {
            free(h, w);
        }
    }

    /// An array's data freed when owned and not null; both words cleared
    /// (data first).
    fn release_owned(h: &mut impl Host, count: u32, data: u32) {
        let w: u32 = h.read(count);
        if owned(w) {
            let p: u32 = h.read(data);
            if p != 0 {
                free(h, p);
            }
        }
        h.write(data, 0u32);
        h.write(count, 0u32);
    }

    /// As [`release_owned`], for arrays whose single element sits in the
    /// data word (freed only when the count is above 1).
    fn release_owned_gt1(h: &mut impl Host, count: u32, data: u32) {
        let w: u32 = h.read(count);
        if owned(w) && w & 0x7fff_ffff > 1 {
            let p: u32 = h.read(data);
            if p != 0 {
                free(h, p);
            }
        }
        h.write(data, 0u32);
        h.write(count, 0u32);
    }

    /// `0x005e6d50` / `0x005e6e00` (thiscall `(shape, flags)`, the same
    /// code): `PPhysicsPlaneBullet`'s scalar deleting destructor: its
    /// vtable, the Bullet object at `+0x6c` deleted (its slot 0, flags 1)
    /// and cleared, `PPhysicsShapeBase`'s vtable, freed when `flags & 1`;
    /// `this`.
    pub fn plane_bullet_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(PLANE_BULLET_VFTABLE);
        h.write(this, vt);
        let obj: u32 = h.read(this + 0x6c);
        if obj != 0 {
            h.call_virtual(obj, 0, &[1]);
        }
        h.write(this + 0x6c, 0u32);
        let vt = h.rebase(SHAPE_BASE_VFTABLE);
        h.write(this, vt);
        if flags & 1 != 0 {
            free(h, this);
        }
        this
    }

    /// `0x00551da0` (create callback, stdcall `(p)`): unless null, the
    /// reference, `+0xc = +0x10 = 0`, two base references at `+0x14` and
    /// `+0x20`; `al = 1`.
    pub fn create_receiver_data(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            let d = h.rebase(RECEIVER_DATA_DESCRIPTOR);
            thiscall(h, REF_INIT, p, &[d]);
            h.write(p + 0xc, 0u32);
            h.write(p + 0x10, 0u32);
            thiscall(h, BASE_REF_CTOR, p + 0x14, &[]);
            thiscall(h, BASE_REF_CTOR, p + 0x20, &[]);
        }
        1
    }

    /// `0x00551e20` (destroy callback, stdcall `(p)`): the words at
    /// `+0x20` and `+0x14` freed unless tagged, the reference released;
    /// `al = 1`.
    pub fn destroy_receiver_data(h: &mut impl Host, p: u32) -> u8 {
        free_unless_tagged(h, p + 0x20);
        free_unless_tagged(h, p + 0x14);
        thiscall(h, REF_RELEASE, p, &[]);
        1
    }

    /// A named semantic descriptor's getter (cdecl `()`,
    /// [`super::Semantic`]): made on the first call; the object.
    pub fn semantic_getter(h: &mut impl Host, s: &Semantic) -> u32 {
        let once = h.rebase(s.once);
        let flags: u32 = h.read(once);
        let obj = h.rebase(s.object);
        if flags & 1 == 0 {
            h.write(once, flags | 1);
            let [a, b, c, d, e] = s.args;
            let args = [h.rebase(a), b, c, h.rebase(d), h.rebase(e)];
            thiscall(h, NAMED_SEMANTIC_CTOR, obj, &args);
            let dtor = h.rebase(s.dtor);
            let vt = h.rebase(s.vtable);
            h.write(obj, vt);
            cdecl(h, ATEXIT, &[dtor]);
        }
        obj
    }

    /// `0x0054bc40` / `0x005d6a60` (thiscall `(array, buffer, &value)`,
    /// `ret 8`): appends `value` (`words` words) to the array through the
    /// new `buffer`: the old elements copied over (skipping a null
    /// destination, as the originals do), the value after them, the old
    /// data freed when owned, not null and not `buffer`; the count + 1
    /// (bit 31 dropped). 0, or 0xd for a null buffer.
    pub fn push_into(h: &mut impl Host, this: u32, buffer: u32, value: u32, words: u32) -> u32 {
        if buffer == 0 {
            return 0xd;
        }
        let size = 4 * words;
        let n = h.read::<u32>(this) & 0x7fff_ffff;
        let mut from: u32 = h.read(this + 4);
        let mut to = buffer;
        for _ in 0..n {
            if to != 0 {
                for k in 0..words {
                    let w: u32 = h.read(from + 4 * k);
                    h.write(to + 4 * k, w);
                }
                from = from.wrapping_add(size);
            }
            to = to.wrapping_add(size);
        }
        let slot = buffer.wrapping_add(n.wrapping_mul(size));
        if slot != 0 {
            for k in 0..words {
                let w: u32 = h.read(value + 4 * k);
                h.write(slot + 4 * k, w);
            }
        }
        let old: u32 = h.read(this + 4);
        if old != buffer && owned(h.read(this)) && old != 0 {
            free(h, old);
        }
        h.write(this + 4, buffer);
        h.write(this, n.wrapping_add(1));
        0
    }

    /// `0x005a6e30` (destroy callback, stdcall `(p)`): the 0x24-byte
    /// array at `+0x3c` released, the array at `+0x20` freed when owned
    /// with more than one element; `al = 1`.
    pub fn destroy_lod_group(h: &mut impl Host, p: u32) -> u8 {
        thiscall(h, ARRAY24_RELEASE, p + 0x3c, &[]);
        release_owned_gt1(h, p + 0x20, p + 0x24);
        1
    }

    /// `0x005a29d0` (thiscall `(element)`): the arrays at `+0x14` and `+4`
    /// freed when owned with more than one element (the first's data only
    /// when not null), all four words cleared.
    pub fn lod_element_dtor(h: &mut impl Host, this: u32) {
        release_owned_gt1(h, this + 0x14, this + 0x18);
        let w: u32 = h.read(this + 4);
        if owned(w) && w & 0x7fff_ffff > 1 {
            let p: u32 = h.read(this + 8);
            free(h, p);
        }
        h.write(this + 8, 0u32);
        h.write(this + 4, 0u32);
    }

    /// `0x00554600` (create callback, stdcall `(p)`): unless null, a
    /// `PScriptedComponent` with its descriptor (`0x00553980`'s stores);
    /// `al = 1`.
    pub fn create_scripted(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            let d = h.rebase(SCRIPTED_DESCRIPTOR);
            thiscall(h, SCRIPTABLE_INIT, p, &[d]);
            let vt = h.rebase(SCRIPTED_VFTABLE);
            h.write(p, vt);
            for off in [0x20, 0x24, 0x14, 0x18, 0x1c] {
                h.write(p + off, 0u32);
            }
            h.write(p + 0x28, 1u32);
            h.write(p + 0x2c, 0u16);
            h.write(p + 0x2e, 0u8);
        }
        1
    }

    /// The `f32` at `at` through `fld`/`fstp`.
    fn copy_f32(h: &impl Host, at: u32) -> u32 {
        x87::store(x87::load(h.read(h.rebase(at))))
    }

    /// `0x005a2520` (thiscall `(this)`): the defaults of a light-like
    /// object (two colour triples from the image, an empty one-element
    /// array at `+0x20`/`+0x24`, 1.0, two addresses, a third constant);
    /// `this`.
    pub fn light_defaults(h: &mut impl Host, this: u32) -> u32 {
        let a = copy_f32(h, LIGHT_A);
        for off in [0, 4, 8] {
            h.write(this + off, a);
        }
        let b = copy_f32(h, LIGHT_B);
        for off in [0x10, 0x14, 0x18] {
            h.write(this + off, b);
        }
        h.write(this + 0x20, 0u32);
        if this.wrapping_add(0x24) != 0 {
            h.write(this + 0x24, 0u32);
            h.write(this + 0x20, 1u32);
        }
        h.write(this + 0x30, 0u32);
        h.write(this + 0x28, ONE);
        let w0 = h.rebase(LIGHT_WORDS[0]);
        h.write(this + 0x34, w0);
        let w1 = h.rebase(LIGHT_WORDS[1]);
        h.write(this + 0x38, w1);
        let c = copy_f32(h, LIGHT_C);
        h.write(this + 0x2c, c);
        h.write(this + 0x3c, 0u32);
        h.write(this + 0x40, 0u32);
        h.write(this + 0x44, 1u8);
        this
    }

    /// `0x005a2630` (thiscall `(this)`): `{0, 0, 0, 0.0, constant, 0}`,
    /// then a one-element array at `+0x14` holding `{1.0, 0.0, 4}` at
    /// `+0x18`; `this`.
    pub fn lod_level_defaults(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0u32);
        let c = copy_f32(h, LOD_LEVEL_DEFAULT);
        h.write(this + 0x10, c);
        h.write(this + 0x14, 0u32);
        let e = this.wrapping_add(0x18);
        if e != 0 {
            h.write(e + 8, 4u32);
            h.write(e, ONE);
            h.write(e + 4, 0u32);
            h.write(this + 0x14, 1u32);
        }
        this
    }

    /// `0x005cefd0` (thiscall `(d)`): `PScheduler`'s class descriptor:
    /// `PClassDescriptor(d, GetGlobalNamespace(), name, 0x284, 4, PBase's
    /// descriptor, 4)` (`PBase`'s fetched first), `+0x90 |= 2`, the vtable,
    /// the three buffers; `d`.
    pub fn scheduler_descriptor_init(h: &mut impl Host, d: u32) -> u32 {
        let base = cdecl(h, PBASE_GET_DESCRIPTOR, &[]);
        let name = h.rebase(SCHEDULER_NAME);
        let ns = cdecl(h, GLOBAL_NAMESPACE, &[]);
        thiscall(h, CLASS_DESCRIPTOR_INIT6, d, &[ns, name, 0x284, 4, base, 4]);
        let flags: u32 = h.read(d + 0x90);
        h.write(d + 0x90, flags | 2);
        let vt = h.rebase(SCHEDULER_DESCRIPTOR_VFTABLE);
        h.write(d, vt);
        let [b0, b1, b2] = SCHEDULER_BUFFERS.map(|b| h.rebase(b));
        thiscall(h, SET_DEFAULT_BUFFER, d, &[b0]);
        thiscall(h, SET_VALIDATION_BUFFER, d, &[b1]);
        thiscall(h, SET_WRITE_MASK_BUFFER, d, &[b2]);
        d
    }

    /// `0x005cf1e0` (thiscall `(scheduler)`): `PScheduler`'s constructor:
    /// the vtable, a free list at `+4` (0x20-byte blocks, 100 of them,
    /// align 4), `+0x1c..+0x38` zero, the async process buffer at `+0x3c`,
    /// 0x200 zero bytes at `+0x84`; `this`.
    pub fn scheduler_ctor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(SCHEDULER_VFTABLE);
        h.write(this, vt);
        let name = h.rebase(SCHEDULER_LIST_NAME);
        thiscall(h, FREE_LIST_INIT, this + 4, &[0x20, 0x64, 4, name]);
        for off in (0x1c..=0x38).step_by(4) {
            h.write(this + off, 0u32);
        }
        thiscall(h, ASYNC_BUFFER_CTOR, this + 0x3c, &[]);
        cdecl(h, MEMSET, &[this + 0x84, 0, 0x200]);
        this
    }

    /// `0x005cfa60` (cdecl `(data, count)`): unless `data` is null, each
    /// 0x14-byte element's two arrays (`+8`/`+0xc`, then `+0`/`+4`)
    /// released when owned, then `data` freed.
    pub fn free_pair_array(h: &mut impl Host, data: u32, count: u32) {
        if data == 0 {
            return;
        }
        let mut e = data;
        for _ in 0..count {
            release_owned(h, e + 8, e + 0xc);
            release_owned(h, e, e + 4);
            e = e.wrapping_add(0x14);
        }
        free(h, data);
    }

    /// `0x005cf420` (thiscall `(scheduler)`): `PScheduler`'s destructor:
    /// the vtable, the shutdown, the async buffer at `+0x3c`, the array at
    /// `+0x34`, the pair array at `+0x28` (when owned), the free list.
    pub fn scheduler_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(SCHEDULER_VFTABLE);
        h.write(this, vt);
        thiscall(h, SCHEDULER_SHUTDOWN, this, &[]);
        thiscall(h, ASYNC_BUFFER_DTOR, this + 0x3c, &[]);
        release_owned(h, this + 0x34, this + 0x38);
        let w: u32 = h.read(this + 0x28);
        if owned(w) {
            let data: u32 = h.read(this + 0x2c);
            cdecl(h, FREE_PAIR_ARRAY, &[data, w & 0x7fff_ffff]);
        }
        h.write(this + 0x2c, 0u32);
        h.write(this + 0x28, 0u32);
        thiscall(h, FREE_LIST_DTOR, this + 4, &[])
    }

    /// `0x00547580` (thiscall `(spline)`): `PSpline`'s destructor: the
    /// vtable, its three arrays (`+0x24`, `+0xc`, `+4`) released.
    pub fn spline_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(SPLINE_VFTABLE);
        h.write(this, vt);
        release_owned(h, this + 0x24, this + 0x28);
        release_owned(h, this + 0xc, this + 0x10);
        release_owned(h, this + 4, this + 8);
    }

    /// `0x00553a70` (thiscall `(component)`): `PScriptedComponent`'s
    /// destructor: the vtable, its script handle (`+0x1c`, when the flag
    /// at `+0x2d` is set) released, the words at `+0x24` and `+0x20` freed
    /// unless tagged, then `PScriptableComponent`'s destructor (its
    /// `eax`).
    pub fn scripted_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(SCRIPTED_VFTABLE);
        h.write(this, vt);
        if h.read::<u8>(this + 0x2d) != 0 {
            let handle: u32 = h.read(this + 0x1c);
            if handle != 0 {
                cdecl(h, RELEASE_HANDLE, &[handle]);
                h.write(this + 0x2d, 0u8);
                h.write(this + 0x1c, 0u32);
            }
        }
        free_unless_tagged(h, this + 0x24);
        free_unless_tagged(h, this + 0x20);
        thiscall(h, SCRIPTABLE_DTOR, this, &[])
    }

    /// `0x0048c3b0` (thiscall `(array, n)`, `ret 4`): resizes a word array
    /// with inline storage for one element to `n` zeroed words: the same
    /// count word does nothing; otherwise the new storage (the inline word
    /// for `n <= 1`, else `PhyreMalloc(4 n, 4)`) zeroed, the old block
    /// freed when owned and neither the new one nor inline, the data word
    /// set (unless inline) and the count stored. 0, or 0xd when the
    /// allocation fails.
    pub fn resize_inline(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let w: u32 = h.read(this);
        if n == w {
            return 0;
        }
        let c = w & 0x7fff_ffff;
        let old = if c > 1 {
            h.read(this + 4)
        } else if c != 0 {
            this + 4
        } else {
            0
        };
        let inline = this + 4;
        let mut buf = inline;
        if n > 1 {
            buf = cdecl(h, MALLOC, &[n.wrapping_mul(4), 4]);
        }
        if buf == 0 {
            if n != 0 {
                return 0xd;
            }
        } else {
            for i in 0..n {
                h.write(buf.wrapping_add(4 * i), 0u32);
            }
        }
        if old != buf && old != inline && owned(h.read(this)) {
            free(h, old);
        }
        if buf != inline {
            h.write(inline, buf);
        }
        h.write(this, n);
        0
    }

    /// `0x005d6b00` (thiscall `(array, buffer, &string)`, `ret 8`): appends
    /// a copy of `string` to a `PString` array through the new `buffer`:
    /// the old strings moved (`0x0044a470`), the copy after them (skipped
    /// for a null slot), the old strings released when owned and not
    /// `buffer`; count + 1 (bit 31 dropped). 0, or 0xd for a null buffer.
    pub fn push_string(h: &mut impl Host, this: u32, buffer: u32, string: u32) -> u32 {
        if buffer == 0 {
            return 0xd;
        }
        let n = h.read::<u32>(this) & 0x7fff_ffff;
        let data: u32 = h.read(this + 4);
        cdecl(h, MOVE_STRINGS, &[buffer, data, n]);
        let slot = buffer.wrapping_add(n.wrapping_mul(4));
        if slot != 0 {
            thiscall(h, STRING_COPY, slot, &[string]);
        }
        let old: u32 = h.read(this + 4);
        if old != buffer {
            let w: u32 = h.read(this);
            if owned(w) {
                cdecl(h, RELEASE_TAGGED_WORDS, &[old, w & 0x7fff_ffff]);
            }
        }
        h.write(this, n.wrapping_add(1));
        h.write(this + 4, buffer);
        0
    }

    /// `0x00559530` (thiscall `(this)`): the attachable component's
    /// constructor: its reference, a 4x4 identity at `+0xc`, `+0x4c = 0`,
    /// two base references (`PBase`'s descriptor at `+0x50` and `+0x64`)
    /// around `{0, 0, -1, 0}`, `+0x68 = 1` (byte); `this`.
    pub fn attachable_ctor(h: &mut impl Host, this: u32) -> u32 {
        let d = h.rebase(ATTACHABLE_DESCRIPTOR);
        thiscall(h, REF_INIT, this, &[d]);
        h.write(this + 0xc, ONE);
        h.write(this + 0x4c, 0u32);
        for (off, v) in [
            (0x10, 0),
            (0x14, 0),
            (0x18, 0),
            (0x1c, 0),
            (0x24, 0),
            (0x28, 0),
            (0x20, ONE),
            (0x2c, 0),
            (0x30, 0),
            (0x38, 0),
            (0x34, ONE),
            (0x3c, 0),
            (0x40, 0),
            (0x44, 0),
            (0x48, ONE),
        ] {
            h.write(this + off, v);
        }
        let d = cdecl(h, PBASE_GET_DESCRIPTOR, &[]);
        h.write(this + 0x50, d);
        h.write(this + 0x54, 0u32);
        h.write(this + 0x58, 0u32);
        h.write(this + 0x5c, u32::MAX);
        h.write(this + 0x60, 0u32);
        let d = cdecl(h, PBASE_GET_DESCRIPTOR, &[]);
        h.write(this + 0x64, d);
        h.write(this + 0x68, 1u8);
        this
    }
}
