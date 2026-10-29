//! M5 stage 1, list `scene`, batch 2: the small methods of the scene,
//! gameplay and LOD classes: accessors, `PArray`/`PSharray` clears and
//! external-buffer setters, destructors, forwarders to sub-objects and the
//! context-switch packers. Each port names its original; the shims in
//! `crates/ffx-hook/src/ported/m5p_scene.rs` pass the original's
//! arguments, and every port returns the original's `eax` where it
//! defines one.
//!
//! **Arrays.** A `PArray`/`PSharray` header is two words: the count, its
//! bit 31 set when the buffer is external (not owned), and the buffer.
//! The clears free an owned buffer (`PhyreFree`, or the element type's
//! release) and zero both words; the setters free an owned buffer that
//! isn't the new one and store `(buffer, count | 0x8000_0000)`.
//!
//! **Floats.** Every float here is copied through the x87 (`fld`/`fstp`
//! of `f32`): exact, but a signalling NaN comes out quiet
//! ([`ffx_bridge::x87::round_trip`]).
//!
//! Image addresses are rebased before use. Nothing here keeps state of
//! its own (worker threads: per-call scratch only).

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

/// `PhyreFree(block)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `operator delete(block)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `PMatrix4_Copy` (thiscall `(out; m)`, `ret 4`).
pub const MATRIX_COPY: u32 = 0x0045_3340;
/// `PMatrix4_toWorldMatrix3x4` (thiscall `(dst; m)`, `ret 4`).
pub const TO_WORLD_MATRIX: u32 = 0x0045_1190;
/// `PScriptableComponent::initialize` (thiscall, three words, `ret 0xc`).
pub const SCRIPTABLE_INITIALIZE: u32 = 0x0055_31d0;
/// The `PWorldMatrixFlyController` methods two forwarders jump to
/// (thiscall, one word, `ret 4`).
pub const FLY_SET: [u32; 2] = [0x005d_6f00, 0x005d_77a0];
/// `Phyre_FindNamed_00545840(name)` (cdecl): the locator shape type of
/// that name.
pub const FIND_LOCATOR_SHAPE: u32 = 0x0054_5840;
/// `0x0054d1e0(name)` (cdecl): the gameplay type of that name.
pub const FIND_GAMEPLAY_TYPE: u32 = 0x0054_d1e0;
/// `Phyre_FindNamed_0054d130(name)` (cdecl).
pub const FIND_NAMED: u32 = 0x0054_d130;
/// The list head `0x0054d040` registers into and `0x0054d190` searches.
pub const NAMED_LIST: u32 = 0x00ca_ece8;
/// The import slot of the CRT's `_stricmp` (cdecl; case-insensitive).
pub const STRICMP_SLOT: u32 = 0x00b0_c398;
/// `PInput_GetDevice(type, n)` (cdecl).
pub const GET_DEVICE: u32 = 0x0062_3c90;
/// `Phyre_PClassDescriptor_setOverallocation` (thiscall `(a, b)`, `ret 8`).
pub const SET_OVERALLOCATION: u32 = 0x0043_d590;
/// `Phyre_PCluster_deleteObject` (thiscall `(cluster; obj)`, `ret 4`).
pub const DELETE_OBJECT: u32 = 0x0044_1240;
/// `0x004533b0` (thiscall `(dst; src)`, `ret 4`): the camera base's copy.
pub const CAMERA_COPY: u32 = 0x0045_33b0;
/// `0x005a7d90`, `PSharray<PLODLevel>::setToExternalBuffer` (thiscall
/// `(buffer, count)`, `ret 8`).
pub const LOD_SET_EXTERNAL: u32 = 0x005a_7d90;
/// `Phyre_PNamespace_GetGlobalNamespace` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// `Phyre_PNamespace_getClassDescriptor` (thiscall `(name)`, `ret 4`).
pub const GET_CLASS_DESCRIPTOR: u32 = 0x0043_e950;
/// `0x00443550` (thiscall `(descriptor)`, `ret 4`): `PWorld`'s instance
/// count of a class.
pub const WORLD_INSTANCE_COUNT: u32 = 0x0044_3550;
/// `0x004334e0` (cdecl): the gameplay utility's list clean-up.
pub const GAMEPLAY_CLEANUP: u32 = 0x0043_34e0;
/// What `0x00432ac0` runs, in order (cdecl, no arguments).
pub const RENDERING_TERMINATE: [u32; 6] = [
    0x0043_26f0,
    0x0043_2680,
    0x004f_dca0,
    0x004f_f0a0,
    0x004e_68d0,
    0x004c_1480,
];
/// `0x0054b300` (thiscall): a spline's segment count.
pub const SPLINE_SEGMENTS: u32 = 0x0054_b300;
/// `0x0054bdb0` (thiscall `(i)`, `ret 4`, a float in `ST0`): segment
/// `i`'s length.
pub const SPLINE_SEGMENT_LENGTH: u32 = 0x0054_bdb0;
/// `0x005d5450(p)` (cdecl): the timer's release.
pub const TIMER_RELEASE: u32 = 0x005d_5450;
/// The camera projection class's vtable (its destructors store it).
pub const CAMERA_PROJECTION_VTABLE: u32 = 0x00b1_0f5c;
/// `PRendering`'s globals `0x004e68d0` resets: a word, a byte and a
/// descriptor.
pub const RENDER_RESET: [u32; 3] = [0x00ca_5fec, 0x00ca_5fe8, 0x00ca_5ff0];
/// The double `0x005b8d80` compares a light's range with (infinite).
pub const RANGE_INFINITE: u32 = 0x00b3_68b0;
/// The global `PArray` `0x005d6c80` clears (count, buffer).
pub const SCRIPT_ARRAY: u32 = 0x00cb_e6d8;
/// The vtable `0x004db1e0` stores (a data address in `.data`).
pub const RENDER_LIST_VTABLE: u32 = 0x00c1_6bd4;

/// A deleting destructor that unregisters the object: the vtable it
/// stores, the cdecl release it calls with `this` when `+0xc` is set, and
/// the table whose `[+0x10]` entry it clears (`0x0049d880`,
/// `0x004caa80`).
#[derive(Clone, Copy, Debug)]
pub struct TypeDtor {
    pub vtable: u32,
    pub release: u32,
    pub table: u32,
}

/// `PLightType`'s deleting destructor (`0x0049d880`).
pub const LIGHT_TYPE_DTOR: TypeDtor = TypeDtor {
    vtable: 0x00b1_70e8,
    release: 0x0049_e920,
    table: 0x00c9_af00,
};
/// `PShadowCasterType`'s deleting destructor (`0x004caa80`).
pub const SHADOW_CASTER_TYPE_DTOR: TypeDtor = TypeDtor {
    vtable: 0x00b1_eb90,
    release: 0x004c_c050,
    table: 0x00ca_43b0,
};

/// A deleting destructor: the vtable it stores, the base destructor it
/// runs (thiscall, or cdecl with `this`) and the free it calls for
/// `flags & 1`.
#[derive(Clone, Copy, Debug)]
pub struct DeletingDtor {
    pub vtable: u32,
    pub base: u32,
    pub base_cdecl: bool,
    pub free: u32,
}

/// `PLocatorShapeType`'s deleting destructor (`0x00545450`).
pub const LOCATOR_SHAPE_TYPE_DTOR: DeletingDtor = DeletingDtor {
    vtable: 0x00b2_be10,
    base: 0x0054_5e80,
    base_cdecl: true,
    free: OPERATOR_DELETE,
};
/// `PWorldMatrixFlyController`'s deleting destructor (`0x005d6d20`).
pub const FLY_CONTROLLER_DTOR: DeletingDtor = DeletingDtor {
    vtable: 0x00b3_9348,
    base: 0x0061_a730,
    base_cdecl: false,
    free: PHYRE_FREE,
};
/// `PCheckVisibleBoundsJob`'s deleting destructor (`0x009f2860`).
pub const VISIBLE_BOUNDS_JOB_DTOR: DeletingDtor = DeletingDtor {
    vtable: 0x00b7_3d68,
    base: 0x005c_d3b0,
    base_cdecl: false,
    free: OPERATOR_DELETE,
};

/// The cdecl element releases of `0x004e37f0` and `0x004f2a20`
/// (`(items, count)`).
pub const PAIRS_RELEASE: u32 = 0x004e_4df0;
pub const SEGMENT_CONTEXTS_RELEASE: u32 = 0x004f_4fc0;

pub mod bridge {
    //! The ports over game memory.

    use super::{
        CAMERA_COPY, CAMERA_PROJECTION_VTABLE, Conv, DELETE_OBJECT, DeletingDtor, FIND_NAMED,
        GAMEPLAY_CLEANUP, GET_CLASS_DESCRIPTOR, GET_DEVICE, GLOBAL_NAMESPACE, Host,
        LOD_SET_EXTERNAL, Mem, NAMED_LIST, OPERATOR_DELETE, PHYRE_FREE, RANGE_INFINITE,
        RENDER_LIST_VTABLE, RENDER_RESET, RENDERING_TERMINATE, SCRIPT_ARRAY, SCRIPTABLE_INITIALIZE,
        SET_OVERALLOCATION, SPLINE_SEGMENT_LENGTH, SPLINE_SEGMENTS, STRICMP_SLOT, TIMER_RELEASE,
        TypeDtor, WORLD_INSTANCE_COUNT, x87,
    };

    const EXTERNAL: u32 = 0x8000_0000;

    /// A float word copied through the x87.
    fn copy_float(m: &mut impl Mem, from: u32, to: u32) {
        let f: f32 = m.read(from);
        m.write(to, x87::round_trip(f));
    }

    /// The `eax` the array clears leave when they free nothing: the
    /// count's bit 31 with `al` inverted (`shr eax, 31; not al`).
    const fn kept(count: u32) -> u32 {
        (count >> 31) ^ 0xff
    }

    // ---- array clears ----

    /// `0x004c97e0`, `0x00547440`, `0x00547470` (thiscall on the header):
    /// frees an owned, non-null buffer, zeroes the buffer then the count.
    pub fn array_clear_checked(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = kept(count);
        if count & EXTERNAL == 0 {
            let buffer: u32 = h.read(this + 4);
            eax = buffer;
            if buffer != 0 {
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer]);
            }
        }
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// `0x004e37c0`, `0x004f29c0` (thiscall on the header): frees an
    /// owned buffer (null or not), zeroes the count then the buffer.
    pub fn array_clear(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = kept(count);
        if count & EXTERNAL == 0 {
            let buffer: u32 = h.read(this + 4);
            eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer]);
        }
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        eax
    }

    /// `0x004e37f0`, `0x004f2a20` (thiscall on the header): an owned
    /// buffer goes to the element type's release `(buffer, count)`
    /// (cdecl); zeroes the count then the buffer.
    pub fn array_clear_release(h: &mut impl Host, this: u32, release: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = kept(count);
        if count & EXTERNAL == 0 {
            let buffer: u32 = h.read(this + 4);
            eax = h.call_sym(release, Conv::Cdecl, &[buffer, count & !EXTERNAL]);
        }
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        eax
    }

    /// `0x00577f20`, `0x005a2880` (thiscall on the header): frees an owned
    /// buffer of more than one element (a single element lives in the
    /// header's own storage); zeroes the buffer then the count.
    pub fn sharray_clear(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = kept(count);
        if count & EXTERNAL == 0 && count & !EXTERNAL > 1 {
            let buffer: u32 = h.read(this + 4);
            eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer]);
        }
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// `0x005a28c0` (thiscall on the header): as [`sharray_clear`] but
    /// a null buffer isn't freed; zeroes the count then the buffer.
    pub fn sharray_clear_checked(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = kept(count);
        if count & EXTERNAL == 0 && count & !EXTERNAL > 1 {
            let buffer: u32 = h.read(this + 4);
            eax = buffer;
            if buffer != 0 {
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer]);
            }
        }
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        eax
    }

    /// `0x00443f80` (thiscall on an array whose count is at `+8` and
    /// buffer at `+0xc`): when the count is not 0, frees an owned,
    /// non-null buffer and zeroes the buffer then the count. 0.
    pub fn array_clear_at8(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this + 8);
        if count != 0 {
            let buffer: u32 = h.read(this + 0xc);
            if buffer != 0 && count & EXTERNAL == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer]);
            }
            h.write(this + 0xc, 0u32);
            h.write(this + 8, 0u32);
        }
        0
    }

    /// `0x005d6c80` (cdecl): [`array_clear_at8`]'s logic on the global
    /// array at [`SCRIPT_ARRAY`]. 0.
    pub fn script_array_clear(h: &mut impl Host) -> u32 {
        let at = h.rebase(SCRIPT_ARRAY);
        let count: u32 = h.read(at);
        if count != 0 {
            let buffer: u32 = h.read(at + 4);
            if buffer != 0 && count & EXTERNAL == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer]);
            }
            h.write(at + 4, 0u32);
            h.write(at, 0u32);
        }
        0
    }

    /// `0x0045a110`, `0x0054cac0` (thiscall `(buffer, count)`, `ret 8`):
    /// `setToExternalBuffer`. A negative count changes nothing. Else an
    /// owned, non-null buffer other than `buffer` is freed, and the header
    /// becomes `(count | 0x8000_0000, buffer)`. 0.
    pub fn array_set_external(h: &mut impl Host, this: u32, buffer: u32, count: u32) -> u32 {
        if count & EXTERNAL != 0 {
            return 0;
        }
        let old: u32 = h.read(this + 4);
        let header: u32 = h.read(this);
        if old != buffer && header & EXTERNAL == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 4, buffer);
        h.write(this, count | EXTERNAL);
        0
    }

    /// `0x005a8230` (thiscall `(buffer, count)`, `ret 8`): for a count
    /// that isn't negative, `PSharray<PLODLevel>::setToExternalBuffer`
    /// with the count marked external. 0.
    pub fn lod_set_external(h: &mut impl Host, this: u32, buffer: u32, count: u32) -> u32 {
        if count & EXTERNAL == 0 {
            h.call_sym(
                LOD_SET_EXTERNAL,
                Conv::Thiscall(this),
                &[buffer, count | EXTERNAL],
            );
        }
        0
    }

    /// `0x00551670` (thiscall on a word whose bit 0 marks it unowned):
    /// frees the block it points at when bit 0 is clear. Nothing returned
    /// (the original leaves `al` changed, which no caller reads).
    pub fn free_unless_bit0(h: &mut impl Host, this: u32) {
        let lo: u8 = h.read(this);
        if lo & 1 == 0 {
            let p: u32 = h.read(this);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
    }

    // ---- destructors ----

    /// `0x004553e0`, `0x00456410` (thiscall): the camera projection's
    /// destructor: its vtable, then the block at `+0xd0` freed unless its
    /// bit 0 is set. Nothing returned (as [`free_unless_bit0`]).
    pub fn camera_projection_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(CAMERA_PROJECTION_VTABLE);
        h.write(this, vt);
        free_unless_bit0(h, this + 0xd0);
    }

    /// `0x004555a0` (thiscall `(flags)`, `ret 4`): the camera
    /// projection's deleting destructor. `this`.
    pub fn camera_projection_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        camera_projection_dtor(h, this);
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x0049d880`, `0x004caa80` (thiscall `(flags)`, `ret 4`): a type
    /// registry entry's deleting destructor: its vtable; when registered
    /// (`+0xc`), the release `(this)` and its table slot `[+0x10]`
    /// cleared; `operator delete` for `flags & 1`. `this`.
    pub fn type_delete(h: &mut impl Host, this: u32, flags: u32, d: &TypeDtor) -> u32 {
        let registered: u32 = h.read(this + 0xc);
        let vt = h.rebase(d.vtable);
        h.write(this, vt);
        if registered != 0 {
            h.call_sym(d.release, Conv::Cdecl, &[this]);
            let index: u32 = h.read(this + 0x10);
            let table = h.rebase(d.table);
            h.write(table.wrapping_add(index.wrapping_mul(4)), 0u32);
        }
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x00545450`, `0x005d6d20`, `0x009f2860` (thiscall `(flags)`,
    /// `ret 4`): its vtable, the base destructor, the free for
    /// `flags & 1`. `this`.
    pub fn deleting_dtor(h: &mut impl Host, this: u32, flags: u32, d: &DeletingDtor) -> u32 {
        let vt = h.rebase(d.vtable);
        h.write(this, vt);
        if d.base_cdecl {
            h.call_sym(d.base, Conv::Cdecl, &[this]);
        } else {
            h.call_sym(d.base, Conv::Thiscall(this), &[]);
        }
        if flags & 1 != 0 {
            h.call_sym(d.free, Conv::Cdecl, &[this]);
        }
        this
    }

    // ---- accessors ----

    /// `0x00459980` (thiscall `(i)`, `ret 4`): element `i` of the 8-byte
    /// array at `+0x10`.
    pub fn element8(m: &impl Mem, this: u32, i: u32) -> u32 {
        let base: u32 = m.read(this + 0x10);
        base.wrapping_add(i.wrapping_mul(8))
    }

    /// `0x004f8070` (thiscall `(i)`, `ret 4`): element `i` of the 64-byte
    /// array at `+0xc`.
    pub fn element64(m: &impl Mem, this: u32, i: u32) -> u32 {
        let base: u32 = m.read(this + 0xc);
        (i << 6).wrapping_add(base)
    }

    /// `0x005b8c30` (thiscall `(i)`, `ret 4`): word `i` of the array at
    /// `+0x70` (count, bit 31 aside) / `+0x74`, or 0 past its end.
    pub fn array_word(m: &impl Mem, this: u32, i: u32) -> u32 {
        let count: u32 = m.read(this + 0x70);
        if i < count & 0x7fff_ffff {
            let base: u32 = m.read(this + 0x74);
            m.read(base.wrapping_add(i.wrapping_mul(4)))
        } else {
            0
        }
    }

    /// `0x004f8100` (thiscall `(out)`, `ret 4`): the vector at `+0x10`
    /// copied to `out`. `out`.
    pub fn get_vector3(m: &mut impl Mem, this: u32, out: u32) -> u32 {
        for i in 0..3 {
            copy_float(m, this + 0x10 + 4 * i, out + 4 * i);
        }
        out
    }

    /// The setters `0x004f8ad0` (`+0x1c`), `0x00507400` (`+0xc`),
    /// `0x00558a30` (`+0xb4`) (thiscall `(v)`, `ret 4`): `v`.
    pub fn set_word(m: &mut impl Mem, this: u32, offset: u32, v: u32) -> u32 {
        m.write(this + offset, v);
        v
    }

    /// `0x005505f0` (thiscall `(b)`, `ret 4`): the byte at `+0x40`.
    /// Nothing returned (the original leaves `al` = `b`, which no caller
    /// reads).
    pub fn set_byte40(m: &mut impl Mem, this: u32, b: u32) {
        m.write(this + 0x40, b as u8);
    }

    /// `0x009f3830` (thiscall): `(end - start) / stride` of the ring at
    /// `+0xc`/`+0x10`/`+0x14` while `start < end`, else `+0x18`; `edx` is
    /// the remainder, or `start`. A zero stride faults in the original
    /// (a divide error); the port returns 0 there.
    pub fn ring_count(m: &impl Mem, this: u32) -> u64 {
        let start: u32 = m.read(this + 0xc);
        let end: u32 = m.read(this + 0x10);
        if start < end {
            let stride: u32 = m.read(this + 0x14);
            let span = end - start;
            let (q, r) = match (span.checked_div(stride), span.checked_rem(stride)) {
                (Some(q), Some(r)) => (q, r),
                _ => (0, 0),
            };
            (u64::from(r) << 32) | u64::from(q)
        } else {
            let other: u32 = m.read(this + 0x18);
            (u64::from(start) << 32) | u64::from(other)
        }
    }

    /// `0x005b8d80` (thiscall, `al`): whether the light's range (`+0x2c`)
    /// equals the double at [`RANGE_INFINITE`] (`fucompp`: false for a
    /// NaN).
    pub fn range_is_infinite(m: &impl Mem, this: u32) -> u8 {
        let range: f32 = m.read(this + 0x2c);
        let at = m.rebase(RANGE_INFINITE);
        let limit: f64 = m.read(at);
        #[allow(clippy::float_cmp)] // the exact compare of `fucompp`
        let same = f64::from(range) == limit;
        u8::from(same)
    }

    /// `0x00443ae0`, `0x009f3a10` (thiscall, `al`): whether the node's
    /// owner (`+0x18`) shows it: no owner, or `+0x1c` is in the owner's
    /// list (from `+0x2c`, linked through `+0x40`) and the owner's `+0x40`
    /// is not 0. 1 for no owner; 0 for an owner with an empty list or
    /// without the node.
    pub fn owner_lists(m: &impl Mem, this: u32) -> u8 {
        let owner: u32 = m.read(this + 0x18);
        if owner == 0 {
            return 1;
        }
        let mut p: u32 = m.read(owner + 0x2c);
        if p == 0 {
            return 0;
        }
        let want: u32 = m.read(this + 0x1c);
        while p != want {
            p = m.read(p + 0x40);
            if p == 0 {
                return 0;
            }
        }
        let next: u32 = m.read(owner + 0x40);
        u8::from(next != 0)
    }

    /// `0x004501d0` (thiscall `(out, x)`, `ret 8`): the first entry of
    /// the list at `this` (linked through `+0`) whose chain (from `+8`,
    /// linked through `+0x40`) holds `x`: `out = (entry, entry's +8)`,
    /// else `(0, x)`. `out`.
    pub fn find_holder(m: &mut impl Mem, this: u32, out: u32, x: u32) -> u32 {
        let mut entry: u32 = m.read(this);
        while entry != 0 {
            let mut p: u32 = m.read(entry + 8);
            while p != 0 {
                if p == x {
                    let first: u32 = m.read(entry + 8);
                    m.write(out, entry);
                    m.write(out + 4, first);
                    return out;
                }
                p = m.read(p + 0x40);
            }
            entry = m.read(entry);
        }
        m.write(out, 0u32);
        m.write(out + 4, x);
        out
    }

    /// `0x0044e8c0` (thiscall `(&key)`, `ret 4`): the node of the tree
    /// (head sentinel `this`, root `+0xc`) whose `+0xc` is `key[0]`
    /// (unsigned; below goes to child 0), or 0.
    pub fn tree_find1(m: &impl Mem, tree: u32, key: u32) -> u32 {
        find1(m, tree, |m| m.read(key))
    }

    /// The walk of [`tree_find1`], the key read (once) only for a tree
    /// that isn't empty.
    fn find1<M: Mem>(m: &M, tree: u32, key: impl Fn(&M) -> u32) -> u32 {
        let mut node: u32 = m.read(tree + 0xc);
        if node == tree {
            return 0;
        }
        let k = key(m);
        while node != tree {
            let n: u32 = m.read(node + 0xc);
            if n == k {
                return node;
            }
            node = m.read(node + if n > k { 0 } else { 4 });
        }
        0
    }

    /// `0x0044f750` (cdecl `(key)`): the word at `+0x10` of `key`'s node
    /// in the class tree at `0x00c0bb70`, or 0 (the original hands the
    /// tree its own argument slot as the key).
    pub fn entity_class_lookup(m: &impl Mem, key: u32) -> u32 {
        let tree = m.rebase(0x00c0_bb70);
        let node = find1(m, tree, |_| key);
        // the original tests the field's address too (never 0 for a node)
        if node == 0 || node.wrapping_add(0x10) == 0 {
            return 0;
        }
        m.read(node + 0x10)
    }

    /// `0x004db1e0` (thiscall): the render list's constructor: its vtable
    /// (a `.data` address), eight words and a byte zeroed. `this`.
    pub fn render_list_ctor(m: &mut impl Mem, this: u32) -> u32 {
        let vt = m.rebase(RENDER_LIST_VTABLE);
        m.write(this, vt);
        for i in 1..9 {
            m.write(this + 4 * i, 0u32);
        }
        m.write(this + 0x24, 0u8);
        this
    }

    /// `0x005d7400` (thiscall): the matrix at `+0x34` set to the identity.
    /// Nothing returned (`eax` untouched).
    pub fn identity_at34(m: &mut impl Mem, this: u32) {
        for i in 0..16 {
            let one = i % 5 == 0;
            m.write(this + 0x34 + 4 * i, if one { 1.0f32 } else { 0.0 });
        }
    }

    /// `0x004df180` (cdecl `(items, n)`): each 20-byte item's five words
    /// zeroed (a null slot skipped: the placement-new test). Nothing
    /// returned (the original's `eax` is a cursor, or its caller's for no
    /// items).
    pub fn zero_items20(m: &mut impl Mem, items: u32, n: u32) {
        let mut p = items;
        for _ in 0..n {
            if p != 0 {
                for i in 0..5 {
                    m.write(p + 4 * i, 0u32);
                }
            }
            p = p.wrapping_add(0x14);
        }
    }

    /// `0x00456da0` (cdecl `(dst, src, n)`): `n` 8-byte items copied, a
    /// null destination slot skipped without using up a source item.
    /// Nothing returned (the original's `eax` is a cursor, or its
    /// caller's for no items).
    pub fn copy_items8(m: &mut impl Mem, dst: u32, src: u32, n: u32) {
        let mut d = dst;
        let mut s = src;
        for _ in 0..n {
            if d != 0 {
                let a: u32 = m.read(s);
                m.write(d, a);
                let b: u32 = m.read(s + 4);
                m.write(d + 4, b);
                s = s.wrapping_add(8);
            }
            d = d.wrapping_add(8);
        }
    }

    // ---- context-switch packers ----

    /// `0x004d4070` (thiscall, seven words, `ret 0x1c`): appends the flag
    /// byte `+0x14` (as 0 or 1) to the word cursor `*cursor` (the fourth
    /// word) and advances it. 0.
    pub fn pack_flag(m: &mut impl Mem, this: u32, cursor: u32) -> u32 {
        let flag: u8 = m.read(this + 0x14);
        let at: u32 = m.read(cursor);
        m.write(at, u32::from(flag != 0));
        m.write(cursor, at.wrapping_add(4));
        0
    }

    /// `0x004d4090` (thiscall, seven words, `ret 0x1c`): appends whether
    /// the sixth word is a non-null object with `+0x18` set (instancing)
    /// to the cursor `*cursor` (the fourth word). 0.
    pub fn pack_instancing(m: &mut impl Mem, cursor: u32, object: u32) -> u32 {
        let on = object != 0 && {
            let w: u32 = m.read(object + 0x18);
            w != 0
        };
        let at: u32 = m.read(cursor);
        m.write(at, u32::from(on));
        m.write(cursor, at.wrapping_add(4));
        0
    }

    // ---- forwarders ----

    /// `0x005073f0`, `0x005588e0`, `0x00558a50`, `0x005d7790` (thiscall
    /// `(x)`, `ret 4`): a tail jump to `target` on the sub-object at
    /// `this + offset`; its `eax`.
    pub fn forward_sub(h: &mut impl Host, this: u32, offset: u32, target: u32, x: u32) -> u32 {
        h.call_sym(target, Conv::Thiscall(this.wrapping_add(offset)), &[x])
    }

    /// `0x00555d90` (thiscall, three words, `ret 0xc`): the timer's period
    /// (`+0x20`) copied to its countdown (`+0x24`), then a tail jump to
    /// `PScriptableComponent::initialize`; its `eax`.
    pub fn timer_initialize(h: &mut impl Host, this: u32, args: [u32; 3]) -> u32 {
        copy_float(h, this + 0x20, this + 0x24);
        h.call_sym(SCRIPTABLE_INITIALIZE, Conv::Thiscall(this), &args)
    }

    /// `0x005588f0` (thiscall `(f)`, `ret 4`): virtual slot 7 (`+0x1c`) of
    /// the sub-object at `+0x34` with `f`; its `eax`.
    pub fn sub_virtual1(h: &mut impl Host, this: u32, f: f32) -> u32 {
        let obj = this.wrapping_add(0x34);
        let vt: u32 = h.read(obj);
        let fun: u32 = h.read(vt + 0x1c);
        let f = x87::round_trip(f).to_bits();
        h.call(fun, Conv::Thiscall(obj), &[f])
    }

    /// `0x00558910` (slot `+0x20`), `0x005589b0` (`+0x24`) (thiscall
    /// `(a, b)`, `ret 8`): that virtual slot of the sub-object at `+0x34`
    /// with `(a, b)`; its `eax`.
    pub fn sub_virtual2(h: &mut impl Host, this: u32, slot: u32, a: f32, b: f32) -> u32 {
        let obj = this.wrapping_add(0x34);
        let vt: u32 = h.read(obj);
        let fun: u32 = h.read(vt + slot);
        let a = x87::round_trip(a).to_bits();
        let b = x87::round_trip(b).to_bits();
        h.call(fun, Conv::Thiscall(obj), &[a, b])
    }

    /// `0x00455f40` (thiscall `(p)`, `ret 4`): with a non-null `p`, the
    /// orthographic camera's size `*p` (`+0x154`) and `size / size`
    /// (`+0x150`), then virtual slot 1 (`+4`, no words); its `eax`. 0 for
    /// a null `p`.
    pub fn ortho_set_size(h: &mut impl Host, this: u32, p: u32) -> u32 {
        if p == 0 {
            return 0;
        }
        let bits: u32 = h.read(p);
        let size = x87::load(x87::round_trip(f32::from_bits(bits)).to_bits());
        h.write(this + 0x154, x87::store(size));
        h.write(this + 0x150, x87::store(x87::div(size, size)));
        let vt: u32 = h.read(this);
        let fun: u32 = h.read(vt + 4);
        h.call(fun, Conv::Thiscall(this), &[])
    }

    /// `0x00456430` (thiscall `(src)`, `ret 4`): the orthographic
    /// camera's copy: the base's copy (`0x004533b0` on `this + 4` with
    /// `src + 4`, or 0 for a null `src`), then the two floats at `+0x150`
    /// and `+0x154`. `this`.
    pub fn ortho_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
        let base = if src == 0 { 0 } else { src.wrapping_add(4) };
        h.call_sym(CAMERA_COPY, Conv::Thiscall(this.wrapping_add(4)), &[base]);
        copy_float(h, src.wrapping_add(0x150), this + 0x150);
        copy_float(h, src.wrapping_add(0x154), this + 0x154);
        this
    }

    /// `0x00628700` (`(n)`, `ret 4`): `PInput_GetDevice(4, n)`.
    pub fn input_device4(h: &mut impl Host, n: u32) -> u32 {
        h.call_sym(GET_DEVICE, Conv::Cdecl, &[4, n])
    }

    /// `0x004e68d0` (cdecl): resets `PRendering`'s globals: the word, the
    /// descriptor's overallocation `(0, 0)`, the byte; the descriptor
    /// call's `eax`.
    pub fn render_reset(h: &mut impl Host) -> u32 {
        let word = h.rebase(RENDER_RESET[0]);
        h.write(word, 0u32);
        let d = h.rebase(RENDER_RESET[2]);
        let eax = h.call_sym(SET_OVERALLOCATION, Conv::Thiscall(d), &[0, 0]);
        let byte = h.rebase(RENDER_RESET[1]);
        h.write(byte, 0u8);
        eax
    }

    /// `0x00432ac0` (cdecl): the rendering utility's terminate: six
    /// clean-ups in order. 0.
    pub fn rendering_terminate(h: &mut impl Host) -> u32 {
        for f in RENDERING_TERMINATE {
            h.call_sym(f, Conv::Cdecl, &[]);
        }
        0
    }

    /// `0x00433650` (cdecl): the gameplay utility's clean-up. 0.
    pub fn gameplay_terminate(h: &mut impl Host) -> u32 {
        h.call_sym(GAMEPLAY_CLEANUP, Conv::Cdecl, &[]);
        0
    }

    /// `0x00545e60`, `0x0054d320` (cdecl `(out, name)`): `*out = find(name)`
    /// (cdecl); 0 when found, 0x13 when not.
    pub fn find_into(h: &mut impl Host, find: u32, out: u32, name: u32) -> u32 {
        let r = h.call_sym(find, Conv::Cdecl, &[name]);
        h.write(out, r);
        if r == 0 { 0x13 } else { 0 }
    }

    /// `0x0054d190` (cdecl `(name)`): the entry of the list at
    /// [`NAMED_LIST`] (`{name, next}`) whose name `_stricmp`s equal (any case), or 0
    /// (also for a null name).
    pub fn find_named(h: &mut impl Host, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let head = h.rebase(NAMED_LIST);
        let mut e: u32 = h.read(head);
        if e == 0 {
            return 0;
        }
        let slot = h.rebase(STRICMP_SLOT);
        let stricmp: u32 = h.read(slot);
        while e != 0 {
            let s: u32 = h.read(e);
            if h.call(stricmp, Conv::Cdecl, &[s, name]) == 0 {
                return e;
            }
            e = h.read(e + 4);
        }
        0
    }

    /// `0x0054d040` (thiscall `(unused, name)`, `ret 8`): a named entry's
    /// constructor: `(name, 0, 1)`; a named entry whose name isn't listed
    /// yet ([`FIND_NAMED`]) is pushed on the list at [`NAMED_LIST`].
    /// `this`.
    pub fn named_entry_ctor(h: &mut impl Host, this: u32, name: u32) -> u32 {
        h.write(this, name);
        h.write(this + 4, 0u32);
        h.write(this + 8, 1u8);
        if name != 0 && h.call_sym(FIND_NAMED, Conv::Cdecl, &[name]) == 0 {
            let head = h.rebase(NAMED_LIST);
            let first: u32 = h.read(head);
            h.write(this + 4, first);
            h.write(head, this);
        }
        this
    }

    /// `0x0054d2b0` (cdecl `(&name, &out, &len)`): `*out = name`,
    /// `*len = strlen(name) + 1`. 0.
    pub fn name_and_size(m: &mut impl Mem, pname: u32, out: u32, len: u32) -> u32 {
        let s: u32 = m.read(pname);
        let mut n = 0u32;
        loop {
            let c: u8 = m.read(s.wrapping_add(n));
            if c == 0 {
                break;
            }
            n += 1;
        }
        m.write(len, n + 1);
        m.write(out, s);
        0
    }

    /// `0x00516cf0` (thiscall `(cluster)`, `ret 4`): the object at `this`
    /// deleted from `cluster` when both are set
    /// (`PCluster::deleteObject`), then the word cleared. Its `eax`: the
    /// call's, else the object (0 for none).
    pub fn delete_held(h: &mut impl Host, this: u32, cluster: u32) -> u32 {
        let obj: u32 = h.read(this);
        let mut eax = obj;
        if obj != 0 && cluster != 0 {
            eax = h.call_sym(DELETE_OBJECT, Conv::Thiscall(cluster), &[obj]);
        }
        h.write(this, 0u32);
        eax
    }

    /// `PTimerComponent::uninitialize` (`0x00555ef0`, thiscall): a held
    /// timer (`+0x1c`, owned when `+0x28`) released, both cleared. Its
    /// `eax`: the release's, else the timer word.
    pub fn timer_uninitialize(h: &mut impl Host, this: u32) -> u32 {
        let timer: u32 = h.read(this + 0x1c);
        if timer == 0 {
            return 0;
        }
        let owned: u8 = h.read(this + 0x28);
        if owned == 0 {
            return timer;
        }
        let eax = h.call_sym(TIMER_RELEASE, Conv::Cdecl, &[timer]);
        h.write(this + 0x28, 0u8);
        h.write(this + 0x1c, 0u32);
        eax
    }

    /// `PWorld::getInstanceCountForScript` (`0x004436a0`, thiscall
    /// `(name)`, `ret 4`): the class of that name in the global namespace,
    /// and the world's instance count of it ([`WORLD_INSTANCE_COUNT`]); 0
    /// for no such class.
    pub fn world_instance_count_for_script(h: &mut impl Host, this: u32, name: u32) -> u32 {
        let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        let d = h.call_sym(GET_CLASS_DESCRIPTOR, Conv::Thiscall(ns), &[name]);
        if d == 0 {
            return 0;
        }
        h.call_sym(WORLD_INSTANCE_COUNT, Conv::Thiscall(this), &[d])
    }

    /// `0x005498c0` (thiscall, a float in `ST0`): a spline's length, the
    /// sum of its segments' lengths, rounded to `f32` after each add.
    pub fn spline_length(h: &mut impl Host, this: u32) -> f32 {
        let mut sum = 0.0f32;
        let n = h.call_sym(SPLINE_SEGMENTS, Conv::Thiscall(this), &[]);
        let mut i = 0;
        while i < n {
            let len = h.call_sym_st0_f64(SPLINE_SEGMENT_LENGTH, Conv::Thiscall(this), &[i]);
            sum = f32::from_bits(x87::store(x87::add(len, x87::load(sum.to_bits()))));
            i += 1;
        }
        sum
    }
}
