//! List p6-render-c, continuation round 2 (batch 12): the renderer's and
//! the render thread's set-up (constructors, ring sizes, `initialize`,
//! `sync`, the viewport setters with their size assert), the material
//! parameter capture buffer, the visibility check's occluder build,
//! job wait and object jobs, and the world cursor's advance.
//!
//! Conventions are read from the disassembly; `eax` is returned wherever
//! the original defines it.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::p6_encode;

/// `Phyre_PhyreFree(p)` (cdecl) and the CRT's `memcpy(dst, src, n)`.
pub const FREE: u32 = 0x0042_fb20;
pub const MEMCPY: u32 = 0x0094_9256;
/// `PThreadPool::GetSingleton()` (cdecl); the preprocess ring's
/// `GetSingleton()` (cdecl) and `allocate(ring; size)` (thiscall).
pub const THREAD_POOL: u32 = 0x0044_cd30;
pub const RING_SINGLETON: u32 = 0x005c_d010;
pub const RING_ALLOCATE: u32 = 0x005c_d0f0;

/// The CRT's `__iob_func()`, `fprintf` and `fflush` import slots
/// (cdecl); `stderr` is `__iob_func() + 0x40`.
pub const IOB: u32 = 0x00b0_c350;
pub const FPRINTF: u32 = 0x00b0_c38c;
pub const FFLUSH: u32 = 0x00b0_c3bc;
/// The assert's colour strings around its message.
pub const RED: u32 = 0x00b1_f84c;
pub const RESET: u32 = 0x00b1_f854;
/// The messages: `PRendererBase::setViewport`'s ("wyq Viewport size
/// error ...") and `0x00615de0`'s ("Viewport size error ...").
pub const RENDERER_VIEWPORT_MSG: u32 = 0x00b3_684c;
pub const THREAD_VIEWPORT_MSG: u32 = 0x00b3_f020;
/// The largest viewport side either accepts.
pub const VIEWPORT_MAX: u32 = 0x8000;
/// `0x00615de0`'s target `0x0059bbf0(state; x, y, w, h)` (thiscall).
pub const SET_VIEWPORT_STATE: u32 = 0x0059_bbf0;
/// `setViewport`'s opcode.
pub const VIEWPORT_OPCODE: u32 = 4;

/// The renderer's rings: `PRenderRingBufferSegmented`'s constructor
/// (thiscall), `initialize(ring; base, size, segment)` (thiscall) and
/// `nextSegment` (thiscall); `allocate(ring; size)` (thiscall).
pub const RING_CTOR: u32 = 0x0061_3d90;
pub const RING_INIT: u32 = 0x0061_3e80;
pub const RING_NEXT: u32 = 0x0061_3eb0;
pub const RING_ALLOC: u32 = 0x0061_3e10;
/// `PSemaphoreWin32`: constructor, `initialize(sem; count, max)`,
/// `wait`, all thiscall.
pub const SEMAPHORE_CTOR: u32 = 0x0042_fc90;
pub const SEMAPHORE_INIT: u32 = 0x0043_0080;
pub const SEMAPHORE_WAIT: u32 = 0x0043_02b0;
/// `PEventFlagWin32`: `initialize(flag; manual)`, `signal`, `wait`.
pub const EVENT_INIT: u32 = 0x0043_0050;
pub const EVENT_SIGNAL: u32 = 0x0043_0200;
pub const EVENT_WAIT: u32 = 0x0043_02a0;

/// `PRendererBase`'s and `PRenderer`'s vtables.
pub const RENDERER_BASE_VTABLE: u32 = 0x00b3_63d8;
pub const RENDERER_VTABLE: u32 = 0x00b3_63f8;
/// The MSVC `eh vector constructor iterator(p, size, n, ctor, dtor)`
/// (stdcall) and the render surface's constructor and destructor.
pub const VECTOR_CTOR: u32 = 0x0094_966a;
pub const SURFACE_CTOR: u32 = 0x005b_3620;
pub const SURFACE_DTOR: u32 = 0x005b_3640;
/// The ring sizes' default word for `+0x54`.
pub const RING_DEFAULT: u32 = 0x00ca_8510;

/// The render thread: the thread set-up `PThreadInitializationWin32`
/// (thiscall; entry), its entry `0x00614710`, the name, and the thread
/// start `0x004300b0(thread; &init)` (thiscall).
pub const THREAD_INIT_CTOR: u32 = 0x0042_fca0;
pub const THREAD_ENTRY: u32 = 0x0061_4710;
pub const THREAD_NAME: u32 = 0x00b3_f14c;
pub const THREAD_START: u32 = 0x0043_00b0;
/// `getEventFlag` and its release, the sync callback.
pub const GET_EVENT_FLAG: u32 = 0x0061_4b70;
pub const RELEASE_EVENT_FLAG: u32 = 0x0061_5ac0;
pub const SYNC_CALLBACK: u32 = 0x0061_4700;
/// The command `sync` queues (the callback opcode).
pub const SYNC_OPCODE: u32 = 0x2c;

/// The visibility check's sort `0x009f16c0(begin, end, n, 0, 0)` and
/// occluder set-up `0x00a2f7d0(out, entries, n, word)` (cdecl).
pub const SORT_OCCLUDERS: u32 = 0x009f_16c0;
pub const BUILD_OCCLUDERS: u32 = 0x00a2_f7d0;
/// The object job's name and the depth bias globals (count, table).
pub const JOB_NAME: u32 = 0x00b7_3d70;
pub const DEPTH_BIAS_COUNT: u32 = 0x0194_2998;
pub const DEPTH_BIAS: u32 = 0x0194_299c;
/// `PCheckVisibleBoundsJob_create()` (cdecl).
pub const JOB_CREATE: u32 = 0x009f_09e0;
/// Objects per job.
pub const JOB_OBJECTS: u32 = 400;
/// The cursor's `skipEmpty` (`0x005009f0`, `ecx`).
pub const SKIP_EMPTY: u32 = 0x0050_09f0;

/// `PModifierNetworkDynamicMesh`'s `constructInstance` (`0x005ada70`,
/// `p6_descriptor::bridge::construct_with_zeroed`): its base constructor,
/// vtable and zeroed words.
pub const MESH_BASE_CTOR: u32 = 0x0049_68c0;
pub const MESH_VTABLE: u32 = 0x00b3_5168;
pub const MESH_ZEROS: [u32; 8] = [0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38];

/// The serialization visitor's names of a node's three arrays.
pub const VISIT_NAMES: [u32; 3] = [0x00cb_c1c0, 0x00cb_c258, 0x00cb_c2f0];

/// The parameter kinds whose capture slot gets the texture's word.
pub const CAPTURE_KINDS: [u8; 8] = [0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12, 0x43, 0x44];

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BUILD_OCCLUDERS, CAPTURE_KINDS, Conv, DEPTH_BIAS, DEPTH_BIAS_COUNT, EVENT_INIT,
        EVENT_SIGNAL, EVENT_WAIT, FFLUSH, FPRINTF, FREE, GET_EVENT_FLAG, Host, IOB, JOB_CREATE,
        JOB_NAME, JOB_OBJECTS, MEMCPY, RED, RELEASE_EVENT_FLAG, RENDERER_BASE_VTABLE,
        RENDERER_VIEWPORT_MSG, RENDERER_VTABLE, RESET, RING_ALLOC, RING_ALLOCATE, RING_CTOR,
        RING_DEFAULT, RING_INIT, RING_NEXT, RING_SINGLETON, SEMAPHORE_CTOR, SEMAPHORE_INIT,
        SEMAPHORE_WAIT, SET_VIEWPORT_STATE, SKIP_EMPTY, SORT_OCCLUDERS, SURFACE_CTOR, SURFACE_DTOR,
        SYNC_CALLBACK, SYNC_OPCODE, THREAD_ENTRY, THREAD_INIT_CTOR, THREAD_NAME, THREAD_POOL,
        THREAD_START, THREAD_VIEWPORT_MSG, VECTOR_CTOR, VIEWPORT_MAX, VIEWPORT_OPCODE, VISIT_NAMES,
        p6_encode, x87,
    };

    /// `0x0059d320(array; i, _)` (thiscall, `ret 8`): marks slot `i` of
    /// a texture array (count `+0xc`, bit 31 a flag; one slot inline at
    /// `+0x10`, more through the pointer `+0x10`; 16-byte slots): 3 out
    /// of range, 0x15 for an empty slot, else its word `+8` set to 1 and
    /// 0. (An empty array reads slot `i` at address `16 i`.)
    pub fn mark_slot(h: &mut impl Host, array: u32, i: u32) -> u32 {
        let n = h.read::<u32>(array + 0xc) & 0x7fff_ffff;
        if i >= n.max(1) {
            return 3;
        }
        let base = match n {
            0 => 0,
            1 => array + 0x10,
            _ => h.read(array + 0x10),
        };
        let slot = base.wrapping_add(i.wrapping_mul(16));
        if h.read::<u32>(slot) == 0 {
            return 0x15;
        }
        h.write(slot.wrapping_add(8), 1u32);
        0
    }

    /// `0x005b0e40(obj)` (thiscall): two `PArray`s freed when owned (bit
    /// 31 of the count clear): `+0xc`/`+0x10` (only with data), then
    /// `+4`/`+8` (freed even when null); both cleared. `eax`: the last
    /// free's, or 0xfe for a view.
    pub fn free_arrays(h: &mut impl Host, obj: u32) -> u32 {
        let count: u32 = h.read(obj + 0xc);
        if count >> 31 == 0 {
            let data: u32 = h.read(obj + 0x10);
            if data != 0 {
                h.call_sym(FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(obj + 0x10, 0u32);
        h.write(obj + 0xc, 0u32);
        let count: u32 = h.read(obj + 4);
        let mut eax = 0xfe;
        if count >> 31 == 0 {
            let data: u32 = h.read(obj + 8);
            eax = h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        h.write(obj + 4, 0u32);
        h.write(obj + 8, 0u32);
        eax
    }

    /// `PRendererBase::PRendererBase(this; device)` (`0x005b4650`,
    /// `ret 4`) -> `this`: the vtable, `+4..+0x14` zero, the device at
    /// `+0x18`, the two rings (`+0x1c`, `+0x30`) constructed,
    /// `+0x44..+0x5c` zero (`+0x4c`, `+0x50` twice).
    pub fn renderer_base_ctor(h: &mut impl Host, this: u32, device: u32) -> u32 {
        let vt = h.rebase(RENDERER_BASE_VTABLE);
        h.write(this, vt);
        for k in 1..6 {
            h.write(this + 4 * k, 0u32);
        }
        h.write(this + 0x18, device);
        h.call_sym(RING_CTOR, Conv::Thiscall(this + 0x1c), &[]);
        h.call_sym(RING_CTOR, Conv::Thiscall(this + 0x30), &[]);
        for k in 0..7 {
            h.write(this + 0x44 + 4 * k, 0u32);
        }
        h.write(this + 0x4c, 0u32);
        h.write(this + 0x50, 0u32);
        this
    }

    /// `PRenderer::PRenderer(this; device)` (`0x005b45b0`, `ret 4`) ->
    /// `this`: the base constructor, the vtable, the semaphore `+0xe0`,
    /// the byte `+0xe4` cleared, four 16-byte render surfaces from
    /// `+0xe8` (the vector constructor) and one more at `+0x128`, the
    /// half-word `+0x138` and the word `+0x13c` cleared.
    pub fn renderer_ctor(h: &mut impl Host, this: u32, device: u32) -> u32 {
        h.call_sym(0x005b_4650, Conv::Thiscall(this), &[device]);
        let vt = h.rebase(RENDERER_VTABLE);
        h.write(this, vt);
        h.call_sym(SEMAPHORE_CTOR, Conv::Thiscall(this + 0xe0), &[]);
        let surfaces = this + 0xe8;
        h.write(this + 0xe4, 0u8);
        let ctor = h.rebase(SURFACE_CTOR);
        let dtor = h.rebase(SURFACE_DTOR);
        h.call_sym(VECTOR_CTOR, Conv::Stdcall, &[surfaces, 0x10, 4, ctor, dtor]);
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(surfaces + 0x40), &[]);
        h.write(surfaces + 0x50, 0u16);
        h.write(this + 0x13c, 0u32);
        this
    }

    /// `PRendererBase::generateMaterialParameterCaptureBuffer(this;
    /// desc)` (`0x005b7f30`, `ret 4`): a copy of the descriptor (its
    /// size the first word) in the data ring (`+0x1c`, rounded to 16);
    /// for each parameter (16-byte entries at `[desc+0xc]`, count
    /// `[desc+8]` without bit 31) of a texture kind (byte `+2`), the
    /// capture slot at offset `u16 +8`: its word `+4` = the texture's
    /// `+8` (`[slot+0xc]`) when the slot's kind is 0x42..0x44 and there
    /// is one, else 0. The copy, or 0 when the ring is full.
    pub fn material_capture(h: &mut impl Host, this: u32, desc: u32) -> u32 {
        let size: u32 = h.read(desc);
        let dst = h.call_sym(
            RING_ALLOC,
            Conv::Thiscall(this + 0x1c),
            &[size.wrapping_add(0xf) & !0xf],
        );
        if dst == 0 {
            return 0;
        }
        let n: u32 = h.read(desc);
        h.call_sym(MEMCPY, Conv::Cdecl, &[dst, desc, n]);
        let mut entry: u32 = h.read(desc + 0xc);
        let count = h.read::<u32>(desc + 8) & 0x7fff_ffff;
        for _ in 0..count {
            let kind: u8 = h.read(entry.wrapping_add(2));
            if CAPTURE_KINDS.contains(&kind) {
                let off: u16 = h.read(entry.wrapping_add(8));
                let slot = dst.wrapping_add(u32::from(off));
                let k: u32 = h.read(slot);
                let mut word = 0;
                if (0x42..=0x44).contains(&k) {
                    let tex: u32 = h.read(slot.wrapping_add(0xc));
                    if tex != 0 {
                        word = h.read(tex.wrapping_add(8));
                    }
                }
                h.write(slot.wrapping_add(4), word);
            }
            entry = entry.wrapping_add(16);
        }
        dst
    }

    /// `0x005b8e80(this; data, commands, data_size, command_size,
    /// segments)` (thiscall, `ret 0x14`): the default word at `+0x54`;
    /// the data ring (`+0x1c`) over `data` (size rounded down to 16, cut
    /// in `segments`), the command ring (`+0x30`) over `commands` (whole
    /// commands of 0x48 bytes, likewise); each set-up's error returned;
    /// then the semaphore `[+0x44]` waited on when there is one, the
    /// frame's commands (`+0x4c`, `+0x50`) cleared and both rings on
    /// their next segment. 0. (The original divides by `segments`; for 0
    /// it faults, the port uses 0.)
    pub fn init_rings(
        h: &mut impl Host,
        this: u32,
        data: u32,
        commands: u32,
        data_size: u32,
        command_size: u32,
        segments: u32,
    ) -> u32 {
        let n = command_size / 0x48;
        let size = data_size & !0xf;
        let def = h.rebase(RING_DEFAULT);
        h.write(this + 0x54, def);
        let r = h.call_sym(
            RING_INIT,
            Conv::Thiscall(this + 0x1c),
            &[data, size, size.checked_div(segments).unwrap_or(0)],
        );
        if r != 0 {
            return r;
        }
        let per = n.checked_div(segments).unwrap_or(0);
        let r = h.call_sym(
            RING_INIT,
            Conv::Thiscall(this + 0x30),
            &[commands, n.wrapping_mul(0x48), per.wrapping_mul(0x48)],
        );
        if r != 0 {
            return r;
        }
        let sem: u32 = h.read(this + 0x44);
        if sem != 0 {
            h.call_sym(SEMAPHORE_WAIT, Conv::Thiscall(sem), &[]);
        }
        h.write(this + 0x4c, 0u32);
        h.write(this + 0x50, 0u32);
        h.call_sym(RING_NEXT, Conv::Thiscall(this + 0x1c), &[]);
        h.call_sym(RING_NEXT, Conv::Thiscall(this + 0x30), &[]);
        0
    }

    /// `0x005c5170(node, visitor)` (cdecl): unless the visitor says skip
    /// (virtual slot 3, `al`), the node's three arrays (counts `+4`,
    /// `+8` and the half-words `+0xc` + `+0xe`; 32-byte entries from
    /// `+0x20`, one after another) visited with their names (slot 1,
    /// `(entries, n, name, 0)`) where not empty. `eax`: the last call's,
    /// or the first count.
    pub fn visit_arrays(h: &mut impl Host, node: u32, visitor: u32) -> u32 {
        let skip = h.call_virtual(visitor, 3, &[]);
        if skip as u8 != 0 {
            return skip;
        }
        let c: u16 = h.read(node + 0xc);
        let b: u32 = h.read(node + 8);
        let a: u32 = h.read(node + 4);
        let d: u16 = h.read(node + 0xe);
        let third = u32::from(d) + u32::from(c);
        let at2 = node.wrapping_add(a.wrapping_add(1) << 5);
        let at3 = node.wrapping_add(a.wrapping_add(1).wrapping_add(b) << 5);
        let mut eax: u32 = h.read(node + 4);
        if eax != 0 {
            let name = h.rebase(VISIT_NAMES[0]);
            eax = h.call_virtual(visitor, 1, &[node + 0x20, eax, name, 0]);
        }
        if b != 0 {
            let name = h.rebase(VISIT_NAMES[1]);
            eax = h.call_virtual(visitor, 1, &[at2, b, name, 0]);
        }
        if third != 0 {
            let name = h.rebase(VISIT_NAMES[2]);
            eax = h.call_virtual(visitor, 1, &[at3, third, name, 0]);
        }
        eax
    }

    /// `PRenderThread::initialize(this)` (`0x00615140`): once (`+0xc`
    /// 1 after): the two semaphores (`+0x15c` 16/16, `+0x164` 1/1; each
    /// marked set up, its error returned), 16 event flags from `+0x18`
    /// (20 bytes each, marked), then the thread: a
    /// `PThreadInitializationWin32` in the frame (`frame`, the original's
    /// `ebp - 0x18`: the entry `0x00614710`, stack 0x10000, 1, the name,
    /// `this`) started with `0x004300b0` (byte `+4` set; its error
    /// returned). 0.
    pub fn thread_initialize(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        if h.read::<u32>(this + 0xc) == 1 {
            return 0;
        }
        let r = h.call_sym(SEMAPHORE_INIT, Conv::Thiscall(this + 0x15c), &[0x10, 0x10]);
        h.write(this + 0x160, 1u8);
        if r != 0 {
            return r;
        }
        let r = h.call_sym(SEMAPHORE_INIT, Conv::Thiscall(this + 0x164), &[1, 1]);
        h.write(this + 0x168, 1u8);
        if r != 0 {
            return r;
        }
        for k in 0..16 {
            let flag = this + 0x18 + 0x14 * k;
            h.call_sym(EVENT_INIT, Conv::Thiscall(flag), &[0]);
            h.write(flag + 4, 1u8);
        }
        let entry = h.rebase(THREAD_ENTRY);
        h.call_sym(THREAD_INIT_CTOR, Conv::Thiscall(frame), &[entry]);
        h.write(frame + 0xc, 1u32);
        h.write(frame + 4, 0x10000u32);
        let name = h.rebase(THREAD_NAME);
        h.write(frame + 0x10, name);
        h.write(frame + 8, this);
        let r = h.call_sym(THREAD_START, Conv::Thiscall(this), &[frame]);
        h.write(this + 4, 1u8);
        if r != 0 {
            return r;
        }
        h.write(this + 0xc, 1u32);
        0
    }

    /// The viewport size assert: for a side above 0x8000, the message
    /// (`msg`, with `w` and `h`) on `stderr` between the colour strings,
    /// flushed, then a store to address 0 (the original's crash, kept).
    fn viewport_assert(h: &mut impl Host, msg: u32, w: u32, ht: u32) {
        if w <= VIEWPORT_MAX && ht <= VIEWPORT_MAX {
            return;
        }
        let iob: u32 = h.read(h.rebase(IOB));
        let fprintf: u32 = h.read(h.rebase(FPRINTF));
        let red = h.rebase(RED);
        let err = h.call(iob, Conv::Cdecl, &[]).wrapping_add(0x40);
        h.call(fprintf, Conv::Cdecl, &[err, red]);
        let msg = h.rebase(msg);
        let err = h.call(iob, Conv::Cdecl, &[]).wrapping_add(0x40);
        h.call(fprintf, Conv::Cdecl, &[err, msg, w, ht]);
        let reset = h.rebase(RESET);
        let err = h.call(iob, Conv::Cdecl, &[]).wrapping_add(0x40);
        h.call(fprintf, Conv::Cdecl, &[err, reset]);
        let err = h.call(iob, Conv::Cdecl, &[]).wrapping_add(0x40);
        let fflush: u32 = h.read(h.rebase(FFLUSH));
        h.call(fflush, Conv::Cdecl, &[err]);
        h.write(0, 0u32);
    }

    /// `0x00615de0(state; x, y, w, h)` (thiscall, `ret 0x10`): the size
    /// assert, then `0x0059bbf0(state; x, y, w, h)`'s result.
    pub fn thread_set_viewport(
        h: &mut impl Host,
        state: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
    ) -> u32 {
        viewport_assert(h, THREAD_VIEWPORT_MSG, w, ht);
        h.call_sym(SET_VIEWPORT_STATE, Conv::Thiscall(state), &[x, y, w, ht])
    }

    /// `PRendererBase::setViewport(this; x, y, w, h)` (`0x005bc060`, `ret
    /// 0x10`): the size assert, then a command (opcode 4, the four words
    /// from `+8`) as the encoders make one (`frame` the original's
    /// `ebp - 8`), the viewport kept at `+8..+0x14`. 0, or 0xd.
    pub fn renderer_set_viewport(
        h: &mut impl Host,
        this: u32,
        frame: u32,
        [x, y, w, ht]: [u32; 4],
    ) -> u32 {
        viewport_assert(h, RENDERER_VIEWPORT_MSG, w, ht);
        let cmd = p6_encode::bridge::begin_command(h, this, frame);
        if cmd == 0 {
            return p6_encode::OUT_OF_MEMORY;
        }
        h.write(cmd + 0xc, y);
        h.write(cmd + 0x10, w);
        h.write(cmd + 0x14, ht);
        h.write(cmd, VIEWPORT_OPCODE);
        h.write(cmd + 8, x);
        h.write(this + 0x10, w);
        h.write(this + 0x14, ht);
        h.write(this + 0xc, y);
        h.write(this + 8, x);
        0
    }

    /// `PRenderThread::sync(this)` (`0x00615ea0`): when running (`+0xc`
    /// 1), a pool flag; a callback command (opcode 0x2c, the callback
    /// `0x00614700` at `+8`, the flag at `+0xc`) built in the frame
    /// (`frame`, the original's `ebp - 0x4c`; 0x48 bytes, the rest never
    /// written) queued after the queue semaphore (`+0x15c`): entry
    /// `+0x154` of 16 (20 bytes from `+0x10`: the command, its end, 0 at
    /// `+0x20`; its event flag `+0x18` signalled); then the flag waited
    /// on and given back. 0.
    pub fn thread_sync(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        if h.read::<u32>(this + 0xc) != 1 {
            return 0;
        }
        let flag = h.call_sym(GET_EVENT_FLAG, Conv::Thiscall(this), &[]);
        h.write(frame, SYNC_OPCODE);
        let cb = h.rebase(SYNC_CALLBACK);
        h.write(frame + 8, cb);
        h.write(frame + 0xc, flag);
        h.call_sym(SEMAPHORE_WAIT, Conv::Thiscall(this + 0x15c), &[]);
        let i: u32 = h.read(this + 0x154);
        let entry = this.wrapping_add(i.wrapping_mul(0x14));
        let next = i.wrapping_add(1);
        h.write(entry + 0x10, frame);
        h.write(entry + 0x14, frame + 0x48);
        h.write(entry + 0x20, 0u32);
        h.write(this + 0x154, if next == 0x10 { 0 } else { next });
        h.call_sym(EVENT_SIGNAL, Conv::Thiscall(entry + 0x18), &[]);
        h.call_sym(EVENT_WAIT, Conv::Thiscall(flag), &[]);
        h.call_sym(RELEASE_EVENT_FLAG, Conv::Thiscall(this), &[flag]);
        0
    }

    fn ring_alloc(h: &mut impl Host, size: u32) -> u32 {
        // the size pushed before GetSingleton is allocate's argument
        let ring = h.call_sym(RING_SINGLETON, Conv::Cdecl, &[]);
        h.call_sym(RING_ALLOCATE, Conv::Thiscall(ring), &[size])
    }

    /// `PVisibilityCheck::buildOccluders(this)` (`0x009f2fd0`): the
    /// occluder lists `+0x20` (next `+0`, owner `+4`, count `+8` without
    /// bit 31, entries `+0xc`, 8 bytes each) copied into one block of the
    /// preprocess ring (after its 16-byte header), each owner's `+4`
    /// cleared, the list emptied; the entries sorted (`0x009f16c0`); a
    /// 0x410-byte block (`+0x38`) set up from them (`0x00a2f7d0`, with
    /// the word `+0`; the result at `+0x3c`); the first block's `+4`
    /// cleared. `eax`: that block, or 0 with nothing.
    pub fn build_occluders(h: &mut impl Host, this: u32) -> u32 {
        let first: u32 = h.read(this + 0x20);
        if first == 0 {
            return 0;
        }
        let mut total = 0u32;
        let mut node = first;
        while node != 0 {
            let n = h.read::<u32>(node + 8) & 0x7fff_ffff;
            total = total.wrapping_add(n);
            node = h.read(node);
        }
        if total == 0 {
            return 0;
        }
        let block = ring_alloc(h, total.wrapping_mul(8));
        let begin = block.wrapping_add(0x10);
        let mut end = begin;
        let mut node: u32 = h.read(this + 0x20);
        while node != 0 {
            let bytes = (h.read::<u32>(node + 8) & 0x7fff_ffff).wrapping_mul(8);
            let src: u32 = h.read(node + 0xc);
            h.call_sym(MEMCPY, Conv::Cdecl, &[end, src, bytes]);
            let owner: u32 = h.read(node + 4);
            h.write(owner + 4, 0u32);
            node = h.read(node);
            end = end.wrapping_add(bytes);
        }
        let n = (end.wrapping_sub(begin) as i32 >> 3) as u32;
        h.write(this + 0x20, 0u32);
        h.call_sym(SORT_OCCLUDERS, Conv::Cdecl, &[begin, end, n, 0, 0]);
        let out = ring_alloc(h, 0x410);
        h.write(this + 0x38, out);
        let word: u32 = h.read(this);
        let set = out.wrapping_add(0x10);
        h.call_sym(BUILD_OCCLUDERS, Conv::Cdecl, &[set, begin, n, word]);
        h.write(this + 0x3c, set);
        h.write(block.wrapping_add(4), 0u32);
        block
    }

    /// `PVisibilityCheck::waitAndCount(this)` (`0x009f30d0`): each job of
    /// the list `+0x18` (next `+0x40`, read first) waited for (the thread
    /// pool's slot 7); a job with no results (`+0x2c` = `+0x30`) is
    /// unlinked and its owner `[+0xc]+4` cleared; the others' results
    /// (12 bytes each, `(+0x2c - +0x30) / 12`, signed) counted. The
    /// count.
    pub fn wait_and_count(h: &mut impl Host, this: u32) -> u32 {
        let mut link = this + 0x18;
        let mut count = 0i32;
        let pool = h.call_sym(THREAD_POOL, Conv::Cdecl, &[]);
        let mut job: u32 = h.read(link);
        while job != 0 {
            let next: u32 = h.read(job + 0x40);
            h.call_virtual(pool, 7, &[job]);
            let end: u32 = h.read(job + 0x30);
            let cur: u32 = h.read(job + 0x2c);
            if cur == end {
                let after: u32 = h.read(job + 0x40);
                h.write(link, after);
                let owner: u32 = h.read(job + 0xc);
                if owner != 0 {
                    h.write(owner + 4, 0u32);
                }
            } else {
                count = count.wrapping_add(cur.wrapping_sub(end) as i32 / 12);
                link = job + 0x40;
            }
            job = next;
        }
        count as u32
    }

    /// The world cursor's advance `0x009f3dc0(cursor; n)` (thiscall,
    /// `ret 4`): `n` entries on (entry `+0xc`, block end `+0x10`, stride
    /// `+0x14`, left `+0x18`; the next block's start at the end's first
    /// word), a block at a time, `skipEmpty` after each. `eax` is not
    /// defined (its callers drop it). (A zero stride faults in the
    /// original; the port takes 0 entries from that block.)
    pub fn advance(h: &mut impl Host, cursor: u32, mut n: u32) {
        while n != 0 {
            let left: u32 = h.read(cursor + 0x18);
            let mut cur: u32 = h.read(cursor + 0xc);
            let mut end: u32 = h.read(cursor + 0x10);
            let mut take = left;
            if cur < end {
                let stride: u32 = h.read(cursor + 0x14);
                take = end.wrapping_sub(cur).checked_div(stride).unwrap_or(0);
            }
            take = take.min(n);
            if take == left && end == 0 {
                h.write(cursor + 0x18, 0u32);
            } else if take != 0 {
                let stride: u32 = h.read(cursor + 0x14);
                let mut rem: u32 = h.read(cursor + 0x18);
                let mut k = take;
                loop {
                    k = k.wrapping_sub(1);
                    loop {
                        cur = cur.wrapping_add(stride);
                        rem = rem.wrapping_sub(1);
                        h.write(cursor + 0xc, cur);
                        h.write(cursor + 0x18, rem);
                        if end != cur {
                            break;
                        }
                        end = h.read(end);
                        h.write(cursor + 0x10, end);
                        if rem == 0 {
                            break;
                        }
                    }
                    if k == 0 {
                        break;
                    }
                }
            }
            n = n.wrapping_sub(take);
            h.call_sym(SKIP_EMPTY, Conv::Thiscall(cursor), &[]);
        }
    }

    /// `PVisibilityCheck::startJobsForObjects(this; objects, n, stride)`
    /// (`0x009f4000`, `ret 0xc`): a result block for `n` 12-byte entries
    /// (with an 8-byte link: `+4` the block, linked at `[+0x28]`, which
    /// moves to it) from the preprocess ring; then a job per 400 objects
    /// (`PCheckVisibleBoundsJob_create`): its name, the four floats
    /// `+8..+0x14` (`fld`/`fstp`), the objects, count and stride, the
    /// depth bias count and table, the words `+4` and `+0x3c`, its
    /// result cursor; started (the thread pool's slot 5) and linked at
    /// `[+0x24]` (next at `+0x40`).
    pub fn start_jobs_for_objects(
        h: &mut impl Host,
        this: u32,
        mut objects: u32,
        mut n: u32,
        stride: u32,
    ) {
        let pool = h.call_sym(THREAD_POOL, Conv::Cdecl, &[]);
        let size = (n.wrapping_mul(12).wrapping_add(0xf) & !0xf).wrapping_add(8);
        let block = ring_alloc(h, size);
        let head = block.wrapping_add(0x10);
        let mut results = head.wrapping_add(8);
        let bias_count: u32 = h.read(h.rebase(DEPTH_BIAS_COUNT));
        h.write(head + 4, block);
        let tail: u32 = h.read(this + 0x28);
        h.write(tail, head);
        h.write(this + 0x28, head);
        let name = h.rebase(JOB_NAME);
        let bias = h.rebase(DEPTH_BIAS);
        while n != 0 {
            let job = h.call_sym(JOB_CREATE, Conv::Cdecl, &[]);
            let k = n.min(JOB_OBJECTS);
            h.write(job + 4, name);
            for (src, dst) in [(8, 0x1c), (0xc, 0x20), (0x10, 0x24), (0x14, 0x28)] {
                let v: f32 = h.read(this + src);
                h.write(job + dst, x87::round_trip(v));
            }
            n -= k;
            h.write(job + 0x10, objects);
            h.write(job + 0x38, bias_count);
            h.write(job + 0x14, k);
            h.write(job + 0x18, stride);
            h.write(job + 0x34, bias);
            let w4: u32 = h.read(this + 4);
            h.write(job + 0x44, w4);
            h.write(job + 0x30, results);
            let w3c: u32 = h.read(this + 0x3c);
            h.write(job + 0x3c, w3c);
            objects = objects.wrapping_add(k.wrapping_mul(stride));
            results = results.wrapping_add(k * 12);
            h.call_virtual(pool, 5, &[job]);
            let link: u32 = h.read(this + 0x24);
            h.write(link, job);
            h.write(this + 0x24, job + 0x40);
        }
    }
}
