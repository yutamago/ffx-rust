//! M5 stage 1, list `phyre-core`, batch 1: `PhyreEngine`'s small core
//! members the class-gated model reaches and no coverage run did.
//!
//! - `PString`'s destructor, the owned-array resets (`[+0]` count with the
//!   external flag in bit 31, `[+4]` data) and the mesh segments' reset;
//! - `PUtility`'s and `PAnnotation`'s destructors (list unlinks), the
//!   thread pools' destructors, the job vtable store;
//! - the thread primitives: wait for a thread's exit, close a handle,
//!   the pool's thread entry and state constructor;
//! - float setters (`fld`/`fstp dword`: exact, a signalling NaN comes out
//!   quiet), `Phyre_maxFloat`, the four-float row store;
//! - small forwarders (a context switch's 16-bit index flag, a segment
//!   context copy, the morph manager's teardown, the red-black tree erase).
//!
//! Calls to code outside the list stay calls to the original at its
//! address. The originals' SEH frames only unwind; dropped.

/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `operator delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `GetDefaultThreadName`'s text.
pub const DEFAULT_THREAD_NAME: u32 = 0x00b0_e060;
/// KERNEL32 import slots: `WaitForSingleObject`, `GetExitCodeThread`,
/// `CloseHandle` (stdcall).
pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
pub const GET_EXIT_CODE_THREAD: u32 = 0x00b0_c110;
pub const CLOSE_HANDLE: u32 = 0x00b0_c0dc;
/// `PMeshSegmentEntries_destroy_00434210(data, n)` (cdecl).
pub const SEGMENTS_DESTROY: u32 = 0x0043_4210;
/// `Phyre_PUtility_vftable` and the utilities' list head.
pub const UTILITY_VTABLE: u32 = 0x00b0_e078;
pub const UTILITY_LIST: u32 = 0x00c9_01a8;
/// `Phyre_PAnnotation_vftable`.
pub const ANNOTATION_VTABLE: u32 = 0x00b3_0a00;
/// The class record's register `0x0043d6a0(class; object, flag)`.
pub const CLASS_REGISTER: u32 = 0x0043_d6a0;
/// `PhyreCtor_zeroWord_0042fcc0(this)` (thiscall).
pub const ZERO_WORD_CTOR: u32 = 0x0042_fcc0;
/// `Phyre_PThreadPool_vftable`, `PThreadPoolImmediate`'s, and the pool's
/// destructor body `0x0044d070` (thiscall) and its vtable-storing
/// destructor `0x0044cc60`.
pub const THREAD_POOL_VTABLE: u32 = 0x00b1_0250;
pub const IMMEDIATE_VTABLE: u32 = 0x00b3_13b4;
pub const THREAD_POOL_DTOR_BODY: u32 = 0x0044_d070;
pub const THREAD_POOL_DTOR: u32 = 0x0044_cc60;
/// `PMatrix4_toWorldMatrix3x4(dst; m)` (thiscall, `ret 4`).
pub const TO_WORLD_MATRIX: u32 = 0x0045_1190;
/// `PMatrix4_Copy_00453340(out; m)` (thiscall, `ret 4`).
pub const MATRIX_COPY: u32 = 0x0045_3340;
/// `Phyre_PThreadPoolJob_vftable`.
pub const JOB_VTABLE: u32 = 0x00b2_a2cc;
/// `eh vector destructor iterator(p, size, n, dtor)` (stdcall) and the
/// element destructor it gets from `0x005cd490`.
pub const VECTOR_DTOR: u32 = 0x0094_96d2;
pub const ANIMATABLE_ELEMENT_DTOR: u32 = 0x005c_d4c0;
/// `Phyre_PRedBlackTree_find_00576ef0(tree; &key)`,
/// `Phyre_PRedBlackTree_remove(tree; node)`,
/// `Phyre_PFreeList_deallocate(list; p)` (thiscall).
pub const TREE_FIND: u32 = 0x0057_6ef0;
pub const TREE_REMOVE: u32 = 0x0057_5000;
pub const FREE_LIST_DEALLOCATE: u32 = 0x0049_c980;
/// `Phyre_PSemaphoreWin32_PSemaphoreWin32(this)`,
/// `Phyre_PCriticalSectionSimple_PCriticalSectionSimple(this)` and the
/// timeline base's destructor `0x0060ec70` (thiscall).
pub const SEMAPHORE_CTOR: u32 = 0x0042_fc90;
pub const CRITICAL_SECTION_CTOR: u32 = 0x0043_1690;
pub const TIMELINE_DTOR: u32 = 0x0060_ec70;
/// The class descriptor `0x005d3e90` looks for in a chain (`+0x40` links).
pub const RIGID_BODY_CLASS: u32 = 0x00ca_f780;
/// `g_MorphManager` and its destructor `0x006abc20` (thiscall).
pub const MORPH_MANAGER: u32 = 0x00cc_ca90;
pub const MORPH_MANAGER_DTOR: u32 = 0x006a_bc20;

/// The `ForType<T>` descriptor destructors of this list: `[this] = vtable`,
/// then a tail jump to `Phyre_PClassDescriptor_Destruct(this)` (the body
/// of `ffx_prender::m5r_f`'s), address and vtable.
pub const DESTRUCT: &[(u32, u32)] = &[
    (0x0044_5360, 0x00b0_f61c),
    (0x0044_aee0, 0x00b1_006c),
    (0x0044_aef0, 0x00b0_fe54),
    (0x0045_1030, 0x00b1_0814),
    (0x0045_32e0, 0x00b1_0abc),
    (0x0045_32f0, 0x00b1_0b9c),
    (0x0045_e860, 0x00b1_1d9c),
    (0x0049_9da0, 0x00b1_6edc),
    (0x0049_9db0, 0x00b1_6cdc),
    (0x004c_9890, 0x00b1_f08c),
    (0x004c_98a0, 0x00b1_eb9c),
    (0x004c_98b0, 0x00b1_ec74),
    (0x0055_df20, 0x00b2_ed44),
    (0x0055_df30, 0x00b2_ef54),
    (0x0055_df40, 0x00b2_ee4c),
    (0x0055_df50, 0x00b2_e1c4),
    (0x0055_df60, 0x00b2_e2b4),
    (0x0055_df70, 0x00b2_e39c),
    (0x0055_df80, 0x00b2_e48c),
    (0x0055_df90, 0x00b2_e57c),
    (0x005a_2820, 0x00b3_4c6c),
    (0x005a_2830, 0x00b3_4b64),
    (0x005a_2840, 0x00b3_47c4),
    (0x005a_2850, 0x00b3_46e4),
];

/// The owned-array resets (byte-identical copies of `PArray_Reset_00633d90`).
pub const RESET: &[u32] = &[
    0x0043_1b60,
    0x0043_1d20,
    0x0043_38d0,
    0x0043_3960,
    0x0043_39d0,
    0x0044_4a80,
    0x0044_4ab0,
    0x0049_9d20,
    0x005c_d430,
    0x005c_d460,
];

/// The vtable of [`DESTRUCT`] copy `f`.
#[must_use]
pub const fn vtable_of(f: u32) -> u32 {
    let mut i = 0;
    while i < DESTRUCT.len() {
        if DESTRUCT[i].0 == f {
            return DESTRUCT[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// What `shr eax, 0x1f; not al` leaves in `eax` for the word `w`.
const fn owned_test(w: u32) -> u32 {
    (w >> 31) ^ 0xff
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn free(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p])
    }

    fn delete(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p])
    }

    fn thiscall(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// Calls the KERNEL32 function in import slot `slot` (stdcall).
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// Copies the float at `from` to `to` through `fld`/`fstp dword`.
    fn copy_float(h: &mut impl Host, from: u32, to: u32) {
        let v: u32 = h.read(from);
        h.write(to, x87::store(x87::load(v)));
    }

    /// `Phyre_PString_dtor_PString(this)`: frees the text unless bit 0 of
    /// the pointer says it is borrowed.
    pub fn string_dtor(h: &mut impl Host, this: u32) {
        let p: u32 = h.read(this);
        if p & 1 == 0 {
            free(h, p);
        }
    }

    /// `Phyre_PThreadInitializationBase_GetDefaultThreadName()` -> the
    /// text.
    pub fn default_thread_name(h: &impl Host) -> u32 {
        h.rebase(DEFAULT_THREAD_NAME)
    }

    /// `0x004301b0(thread; &code)` -> 0: waits for the thread
    /// (`WaitForSingleObject(handle, INFINITE)`), then `[code] =` its exit
    /// code (`GetExitCodeThread` into the local at `local`).
    pub fn thread_join(h: &mut impl Host, this: u32, code: u32, local: u32) -> u32 {
        let handle: u32 = h.read(this);
        import(h, WAIT_FOR_SINGLE_OBJECT, &[handle, u32::MAX]);
        let handle: u32 = h.read(this);
        import(h, GET_EXIT_CODE_THREAD, &[handle, local]);
        let v: u32 = h.read(local);
        h.write(code, v);
        0
    }

    /// `CloseHandle([this])` -> 0 (`0x00430240`, `0x00430250`).
    pub fn close_handle(h: &mut impl Host, this: u32) -> u32 {
        let handle: u32 = h.read(this);
        import(h, CLOSE_HANDLE, &[handle]);
        0
    }

    /// The owned-array reset `(this)` -> `eax`: unless the count's bit 31
    /// (external) is set, frees the data when there is some; then data and
    /// count are 0. `eax` is `PhyreFree`'s, 0 for no data, or the flag
    /// test's `0xfe`.
    pub fn owned_reset(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = owned_test(count);
        if count >> 31 == 0 {
            let data: u32 = h.read(this + 4);
            eax = data;
            if data != 0 {
                eax = free(h, data);
            }
        }
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// `0x00433990(this)` -> `eax`: the mesh segments' reset: unless
    /// external, `PMeshSegmentEntries_destroy(data, count)` (its `eax`,
    /// else the flag test's `0xfe`); then count and data are 0.
    pub fn segments_reset(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = owned_test(count);
        if count >> 31 == 0 {
            let data: u32 = h.read(this + 4);
            eax = h.call_sym(SEGMENTS_DESTROY, Conv::Cdecl, &[data, count & 0x7fff_ffff]);
        }
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        eax
    }

    /// `Phyre_PUtility`'s destructor `0x00431d50(this)`: the vtable, then
    /// the utility leaves the list at `0x00c901a8` (links at `+4`) when it
    /// is on it.
    pub fn utility_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(UTILITY_VTABLE);
        h.write(this, vt);
        let mut slot = h.rebase(UTILITY_LIST);
        if h.read::<u32>(slot) == 0 {
            return;
        }
        loop {
            let node: u32 = h.read(slot);
            if node == this {
                break;
            }
            slot = node.wrapping_add(4);
            if h.read::<u32>(slot) == 0 {
                break;
            }
        }
        if h.read::<u32>(slot) != 0 {
            let next: u32 = h.read(this + 4);
            h.write(slot, next);
            h.write(this + 4, 0u32);
        }
    }

    /// The annotation's unlink: the vtable, the node at `+4` leaves its
    /// ring (`+4` next, `+8` previous) and links to itself; `eax` is the
    /// previous node.
    fn annotation_unlink(h: &mut impl Host, this: u32) -> u32 {
        let link = this + 4;
        let vt = h.rebase(ANNOTATION_VTABLE);
        h.write(this, vt);
        let next: u32 = h.read(link);
        let prev: u32 = h.read(link + 4);
        h.write(next.wrapping_add(4), prev);
        h.write(prev, next);
        h.write(link, link);
        h.write(link + 4, link);
        prev
    }

    /// `Phyre_PAnnotation_dtor_PAnnotation(this)` -> `eax` (the previous
    /// node).
    pub fn annotation_dtor(h: &mut impl Host, this: u32) -> u32 {
        annotation_unlink(h, this)
    }

    /// `PAnnotation`'s scalar deleting destructor `(this; flags)` ->
    /// `this`: the destructor, `operator delete(this)` for bit 0.
    pub fn annotation_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        annotation_unlink(h, this);
        if flags & 1 != 0 {
            delete(h, this);
        }
        this
    }

    /// `0x0044a450(descriptor; offset, flag)` -> `al` 1 over the
    /// register's `eax`: `0x0043d6a0(descriptor; [descriptor + 0x88] +
    /// offset, flag)`.
    pub fn register_member(h: &mut impl Host, this: u32, offset: u32, flag: u32) -> u32 {
        let base: u32 = h.read(this + 0x88);
        let eax = thiscall(h, CLASS_REGISTER, this, &[base.wrapping_add(offset), flag]);
        eax & !0xff | 1
    }

    /// `0x0044cc20(this)` -> `this`: the pool's thread state: two zero
    /// words, `PhyreCtor_zeroWord(this + 8)`, the flags and the word
    /// after them zero.
    pub fn pool_state_init(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        thiscall(h, ZERO_WORD_CTOR, this + 8, &[]);
        h.write(this + 0xc, 0u8);
        h.write(this + 0x10, 0u32);
        h.write(this + 0x14, 0u8);
        this
    }

    /// `Phyre_PThreadPool_dtor_PThreadPool(this)`: the vtable, then the
    /// destructor body's `eax` (a tail jump).
    pub fn pool_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(THREAD_POOL_VTABLE);
        h.write(this, vt);
        thiscall(h, THREAD_POOL_DTOR_BODY, this, &[])
    }

    /// `Phyre_PThreadPool_deleting_dtor_0044ccd0(this; flags)` -> `this`.
    pub fn pool_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(THREAD_POOL_VTABLE);
        h.write(this, vt);
        thiscall(h, THREAD_POOL_DTOR_BODY, this, &[]);
        if flags & 1 != 0 {
            delete(h, this);
        }
        this
    }

    /// `PThreadPoolImmediate`'s scalar deleting destructor
    /// `0x0057cf20(this; flags)` -> `this`: its vtable, the pool's
    /// destructor, `operator delete` for bit 0.
    pub fn immediate_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(IMMEDIATE_VTABLE);
        h.write(this, vt);
        thiscall(h, THREAD_POOL_DTOR, this, &[]);
        if flags & 1 != 0 {
            delete(h, this);
        }
        this
    }

    /// `0x0057d2e0(this; flags)` -> `this`: the profile timeline's
    /// destructor `0x0060ec70`, `operator delete` for bit 0.
    pub fn timeline_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        thiscall(h, TIMELINE_DTOR, this, &[]);
        if flags & 1 != 0 {
            delete(h, this);
        }
        this
    }

    /// The pool's thread entry `0x0044ce30(arg)` -> its `eax`: slot 0 of
    /// the pool `[arg]` (`workerThread`, thiscall `(pool; arg)`).
    pub fn thread_entry(h: &mut impl Host, arg: u32) -> u32 {
        let pool: u32 = h.read(arg);
        let vt: u32 = h.read(pool);
        let f: u32 = h.read(vt);
        h.call(f, Conv::Thiscall(pool), &[arg])
    }

    /// `0x00452730(dst; m)`: a tail jump to `PMatrix4_toWorldMatrix3x4`.
    pub fn to_world_matrix(h: &mut impl Host, this: u32, m: u32) -> u32 {
        thiscall(h, TO_WORLD_MATRIX, this, &[m])
    }

    /// A float setter `(this; v)` -> `this`: `[this + offset] = v`.
    pub fn set_float(h: &mut impl Host, this: u32, offset: u32, v: u32) -> u32 {
        h.write(this.wrapping_add(offset), x87::store(x87::load(v)));
        this
    }

    /// A projection setter `(this; v)` -> slot 1's `eax`: `[this +
    /// offset] = v`, then the projection's update (vtable `+4`, thiscall
    /// `(this)`).
    pub fn set_float_update(h: &mut impl Host, this: u32, offset: u32, v: u32) -> u32 {
        let vt: u32 = h.read(this);
        h.write(this.wrapping_add(offset), x87::store(x87::load(v)));
        let f: u32 = h.read(vt.wrapping_add(4));
        h.call(f, Conv::Thiscall(this), &[])
    }

    /// `0x004d43f0(a0, a1, a2, cursor, a4, shader)` (stdcall, six words)
    /// -> 0: writes 1 at the cursor when the shader `+0x1c`'s format byte
    /// `+0x3c` is 12 (16-bit indices), else 0, and moves it on a word.
    pub fn index16_flag(h: &mut impl Host, cursor: u32, shader: u32) -> u32 {
        let mut set = false;
        if shader != 0 {
            let p: u32 = h.read(shader + 0x1c);
            if p != 0 {
                set = h.read::<u8>(p.wrapping_add(0x3c)) == 0xc;
            }
        }
        let at: u32 = h.read(cursor);
        h.write(at, u32::from(set));
        // `add [cursor], 4` reads the cursor again (the store may hit it)
        let at: u32 = h.read(cursor);
        h.write(cursor, at.wrapping_add(4));
        0
    }

    /// `0x004f8e80(array; i, m)` -> `eax`: for `i` below the count
    /// (`+8`, less bit 31) `PMatrix4_Copy(data + 64 i; m)` (data `+0xc`);
    /// else the count.
    pub fn matrix_set(h: &mut impl Host, this: u32, i: u32, m: u32) -> u32 {
        let n = h.read::<u32>(this + 8) & 0x7fff_ffff;
        if i >= n {
            return n;
        }
        let data: u32 = h.read(this + 0xc);
        thiscall(h, MATRIX_COPY, data.wrapping_add(i << 6), &[m])
    }

    /// `0x004f9260(this; &v)`: with `v` and `[this + 0x1c]` set, that
    /// object's `+0x1c` becomes `this` when `0 < v`, else 0 (NaN too).
    /// The original's `eax` is the FPU status word's (never read).
    pub fn set_owner_if_positive(h: &mut impl Host, this: u32, v: u32) {
        if v == 0 {
            return;
        }
        let t: u32 = h.read(this + 0x1c);
        if t == 0 {
            return;
        }
        let x = x87::load(h.read(v));
        h.write(t.wrapping_add(0x1c), if 0.0 < x { this } else { 0 });
    }

    /// `Phyre_maxFloat_005126e0(a, b)` -> `ST0`: `a` when `b < a`, else
    /// `b` (also when unordered), through a `fstp dword`.
    #[must_use]
    pub fn max_float(a: u32, b: u32) -> f32 {
        let (x, y) = (x87::load(a), x87::load(b));
        let r = if y < x { x } else { y };
        f32::from_bits(x87::store(r))
    }

    /// `0x005cd3b0` / `0x005cd3c0(this)`: `[this] = Phyre_PThreadPoolJob_vftable`.
    pub fn job_vtable(h: &mut impl Host, this: u32) {
        let vt = h.rebase(JOB_VTABLE);
        h.write(this, vt);
    }

    /// `0x005cd490(this)` -> `eax`: with an array at `[this]` (its count
    /// in the word before it), destroys each 0x11c-byte element
    /// (`eh vector destructor iterator` with `0x005cd4c0`), then
    /// `PhyreFree` of the block (its `eax`); 0 for none.
    pub fn element_array_delete(h: &mut impl Host, this: u32) -> u32 {
        let p: u32 = h.read(this);
        if p == 0 {
            return 0;
        }
        let block = p.wrapping_sub(4);
        let n: u32 = h.read(block);
        let dtor = h.rebase(ANIMATABLE_ELEMENT_DTOR);
        h.call_sym(VECTOR_DTOR, Conv::Stdcall, &[p, 0x11c, n, dtor]);
        free(h, block)
    }

    /// `0x00574680(this; i, &v)` -> `this`: the four floats of `v` to row
    /// `i` (`this + 16 i`).
    pub fn set_row(h: &mut impl Host, this: u32, i: u32, v: u32) -> u32 {
        let row = this.wrapping_add(i.wrapping_mul(16));
        for k in 0..4 {
            copy_float(h, v.wrapping_add(4 * k), row.wrapping_add(4 * k));
        }
        this
    }

    /// `0x0057cb70(tree; &key)` -> `eax`: the node with that key leaves
    /// the tree and goes back to the free list at `+0x18`
    /// (`Phyre_PFreeList_deallocate`'s `eax`); 0 when there is none.
    pub fn tree_erase(h: &mut impl Host, this: u32, key: u32) -> u32 {
        let node = thiscall(h, TREE_FIND, this, &[key]);
        if node == 0 {
            return 0;
        }
        thiscall(h, TREE_REMOVE, this, &[node]);
        thiscall(h, FREE_LIST_DEALLOCATE, this + 0x18, &[node])
    }

    /// `0x0057d050(this)` -> `PhyreFree`'s `eax`: frees `[this + 0xc]` and
    /// clears it.
    pub fn free_buffer(h: &mut impl Host, this: u32) -> u32 {
        let p: u32 = h.read(this + 0xc);
        let eax = free(h, p);
        h.write(this + 0xc, 0u32);
        eax
    }

    /// `0x0057d070(this; job)` -> 0: the job's slot 1 (thiscall `(job;
    /// [this + 0xc], [this + 0x10], 0)`).
    pub fn run_job(h: &mut impl Host, this: u32, job: u32) -> u32 {
        let vt: u32 = h.read(job);
        let a: u32 = h.read(this + 0xc);
        let b: u32 = h.read(this + 0x10);
        let f: u32 = h.read(vt.wrapping_add(4));
        h.call(f, Conv::Thiscall(job), &[a, b, 0]);
        0
    }

    /// `0x0057d0f0(this)` -> `this`: `PSemaphoreWin32(this)`, the byte at
    /// `+4` zero.
    pub fn semaphore_flag_init(h: &mut impl Host, this: u32) -> u32 {
        thiscall(h, SEMAPHORE_CTOR, this, &[]);
        h.write(this + 4, 0u8);
        this
    }

    /// `0x005a1030(dst, src, n)`: copies `n` 12-byte elements; a null
    /// destination slot is skipped without moving the source (the
    /// original's placement-new test). The original's `eax` is the last
    /// slot's address (unset for `n` = 0), never read.
    pub fn copy12(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let mut s = src;
        for k in 0..n {
            let d = dst.wrapping_add(k.wrapping_mul(12));
            if d != 0 {
                for w in 0..3 {
                    let v: u32 = h.read(s.wrapping_add(4 * w));
                    h.write(d.wrapping_add(4 * w), v);
                }
                s = s.wrapping_add(12);
            }
        }
    }

    /// `0x005ccfb0(this)` -> `this`: `PCriticalSectionSimple(this)` and
    /// six zero words from `+0x18`.
    pub fn locked_list_init(h: &mut impl Host, this: u32) -> u32 {
        thiscall(h, CRITICAL_SECTION_CTOR, this, &[]);
        for k in 0..6 {
            h.write(this + 0x18 + 4 * k, 0u32);
        }
        this
    }

    /// `0x005cd0c0(end, &cursor)` -> 0: moves the cursor along the links
    /// (`+0`) past nodes whose `+4` is 0, stopping at `end` or a node with
    /// one.
    pub fn skip_empty(h: &mut impl Host, end: u32, cursor: u32) -> u32 {
        let mut at: u32 = h.read(cursor);
        while at != end {
            if h.read::<u32>(at.wrapping_add(4)) != 0 {
                break;
            }
            at = h.read(at);
            h.write(cursor, at);
        }
        0
    }

    /// `0x005d3e90(this)` -> `eax`: when the class chain at `+8` (links
    /// `+0x40`) holds the rigid-body descriptor and `[this + 4] - 4` is an
    /// object, that object's slot `+0x20` (thiscall, a tail jump); else 0.
    pub fn rigid_body_slot8(h: &mut impl Host, this: u32) -> u32 {
        let want = h.rebase(RIGID_BODY_CLASS);
        let mut class: u32 = h.read(this + 8);
        loop {
            if class == 0 {
                return 0;
            }
            if class == want {
                break;
            }
            class = h.read(class.wrapping_add(0x40));
        }
        let p: u32 = h.read(this + 4);
        if p == 0 {
            return 0;
        }
        let obj = p.wrapping_sub(4);
        if obj == 0 {
            return 0;
        }
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt.wrapping_add(0x20));
        h.call(f, Conv::Thiscall(obj), &[])
    }

    /// `0x006abeb0()`: destroys and deletes `g_MorphManager` when set, then
    /// clears it. The original's `eax` is `operator delete`'s, or the
    /// caller's when there is none (an `atexit` destructor: never read).
    pub fn morph_manager_destroy(h: &mut impl Host) {
        let g = h.rebase(MORPH_MANAGER);
        let m: u32 = h.read(g);
        if m == 0 {
            return;
        }
        thiscall(h, MORPH_MANAGER_DTOR, m, &[]);
        delete(h, m);
        h.write(g, 0u32);
    }
}
