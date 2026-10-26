//! List p6-render-c, continuation round 2 (batch 11): the small engine
//! functions left of the list: thunks, a modifier network's sync, the
//! renderer's ring room check, the visibility check's job bookkeeping,
//! post-effect deleting destructors and render-target helpers, the
//! bitmap text's colour, the application's timer step and input update,
//! a file seek and the Iggy start-up.
//!
//! Each bridge's doc names its original and the calling convention read
//! from the disassembly; `eax` is returned wherever the original defines
//! it, also where Ghidra says `void`.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyreOS_Sleep(ms)` (cdecl).
pub const SLEEP: u32 = 0x0043_0030;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// The CRT's `atexit(f)` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// `PRenderRingBuffer::check(ring; cursor, size, &wrapped)` (thiscall,
/// `ret 0xc`).
pub const RING_CHECK: u32 = 0x0061_3f00;
/// `PThreadPool::GetSingleton()` (cdecl).
pub const THREAD_POOL: u32 = 0x0044_cd30;

/// The thunks: (address, target, stack words). Each tail-jumps to its
/// target with `ecx` and the stack as it found them.
pub const THUNKS: &[(u32, u32, u32)] = &[
    // renderMeshInstanceList(this; a, b, c)
    (0x005b_a360, 0x005b_3df0, 3),
    // PLegacyGlowGPUBase's 0x00a1a2c0(this; a, b, c)
    (0x00a1_7980, 0x00a1_a2c0, 3),
    // PFXAABase::initializeCommonShaders(this; a, b)
    (0x00a1_a810, 0x00a1_8870, 2),
];

/// The thunk at `addr` ([`THUNKS`]): (target, words).
#[must_use]
pub const fn thunk(addr: u32) -> (u32, u32) {
    let mut i = 0;
    while i < THUNKS.len() {
        if THUNKS[i].0 == addr {
            return (THUNKS[i].1, THUNKS[i].2);
        }
        i += 1;
    }
    panic!("no such thunk")
}

/// `PApplication::isKeyDown`'s target `0x00628470(input; key, i)`
/// (thiscall on `+0x2b8`, `ret 8`, a float in `ST0`).
pub const KEY_VALUE: u32 = 0x0062_8470;

/// The post-effect deleting destructors: (address, vtable, render
/// target field, base destructor).
pub const DELETING: &[(u32, u32, u32, u32)] = &[
    (0x00a0_27a0, 0x00b7_4590, 0xfb0, 0x009f_dd30),
    (0x00a0_2850, 0x00b7_4590, 0xfb0, 0x009f_dd30),
    (0x00a0_2e90, 0x00b7_4928, 0xd4c, 0x009f_e640),
    (0x00a0_2f40, 0x00b7_4928, 0xd4c, 0x009f_e640),
];

/// The deleting destructor at `addr` ([`DELETING`]).
#[must_use]
pub const fn deleting(addr: u32) -> (u32, u32, u32) {
    let mut i = 0;
    while i < DELETING.len() {
        if DELETING[i].0 == addr {
            return (DELETING[i].1, DELETING[i].2, DELETING[i].3);
        }
        i += 1;
    }
    panic!("no such destructor")
}

/// The render target's release `0x00a0e520(target)` (cdecl).
pub const TARGET_RELEASE: u32 = 0x00a0_e520;
/// The texture kinds' `isBound` (thiscall, `al`) and `unbind`
/// (thiscall) by the target's kind byte `+4`: 2D, cube, 3D.
pub const IS_BOUND: [u32; 3] = [0x0059_5f90, 0x0059_5fa0, 0x0059_5fb0];
pub const UNBIND: [u32; 3] = [0x0059_cc40, 0x0059_cf00, 0x0059_d0f0];
/// `PTexture2DBase::setDimensions(tex; w, h, format, 0)` (thiscall).
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;

/// `PEffect`'s parameter lookup `0x0056cbf0(effect; name)` (thiscall).
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
/// `PDepthOfFieldBase::initializeCommonShaders(this; a, effect)`.
pub const DOF_COMMON: u32 = 0x00a1_85a0;
/// Its three parameter names and where they go.
pub const DOF_PARAMETERS: [(u32, u32); 3] = [
    (0x00b7_6734, 0xfb4), // "FocusPlaneDistance"
    (0x00b7_6748, 0xfb8), // "FocusRange"
    (0x00b7_6754, 0xfbc), // "FocusBlurRange"
];
/// "textColor" and `ShaderParamBuffer::SetValue(buffer; parameter, value)`.
pub const TEXT_COLOR: u32 = 0x00b7_8cbc;
pub const SET_VALUE: u32 = 0x0063_8780;
/// The bitmap text's 0x00a2a110(this; a) and createMeshGeometry(this; n).
pub const TEXT_SET: u32 = 0x00a2_a110;
pub const CREATE_MESH: u32 = 0x00a2_9770;

/// The job ring: `PPreprocessRingBuffer::GetSingleton()` (cdecl) and
/// `allocate(ring; size)` (thiscall).
pub const RING_SINGLETON: u32 = 0x005c_d010;
pub const RING_ALLOCATE: u32 = 0x005c_d0f0;
/// `PCheckVisibleBoundsJob`'s vtable and its job-type word.
pub const JOB_VTABLE: u32 = 0x00b7_3d68;
pub const JOB_TYPE: u32 = 0x00b2_a2e0;

/// The visibility check's job starters' callees (thiscall): the
/// cursor's count, its advance, the job start.
pub const OCCLUDER_COUNT: u32 = 0x009f_3830;
pub const OCCLUDER_ADVANCE: u32 = 0x009f_3e50;
pub const OCCLUDER_START: u32 = 0x009f_2f20;
pub const OBJECT_COUNT: u32 = 0x009f_3810;
pub const OBJECT_ADVANCE: u32 = 0x009f_3dc0;
pub const OBJECT_START: u32 = 0x009f_4000;
/// The occluder cursor's set-up: the world lookup `0x004437e0(world;
/// type)`, the type `0x01a85420`, `0x00443ef0(cursor)`, `0x00441dc0(_;
/// p)` and `skipEmpty` `0x009f3930(cursor)`.
pub const CURSOR_LOOKUP: u32 = 0x0044_37e0;
pub const CURSOR_TYPE: u32 = 0x01a8_5420;
pub const CURSOR_FIRST: u32 = 0x0044_3ef0;
pub const CURSOR_NOTE: u32 = 0x0044_1dc0;
pub const CURSOR_SKIP: u32 = 0x009f_3930;
/// `PVisibilityCheck`'s job release `0x009f3590(this)`.
pub const RELEASE_JOBS: u32 = 0x009f_3590;

/// `PTimerWin32::GetTime()` (cdecl, a double in `ST0`) and the start
/// time it is taken from (a double).
pub const GET_TIME: u32 = 0x0042_fd80;
pub const START_TIME: u32 = 0x00c9_0278;

/// `PInput::Update`'s globals: the "reacquire" byte, the device (COM),
/// its enumeration callback and the input list's head.
pub const INPUT_DIRTY: u32 = 0x00cc_9cd0;
pub const INPUT_DEVICE: u32 = 0x00cc_9cd4;
pub const INPUT_ENUM: u32 = 0x0062_4d40;
pub const INPUT_LIST: u32 = 0x00c3_0fa8;

/// `operator new(n)` (cdecl) and `std::_Xbad_alloc`'s import slot.
pub const NEW: u32 = 0x0063_0310;
pub const BAD_ALLOC: u32 = 0x00b0_c2a0;

/// `0x00626c40`'s lookup `0x0043cbb0(list)` on `0x00cc9d20` and its call
/// `0x0061fdd0(p; q)`.
pub const LOOKUP: u32 = 0x0043_cbb0;
pub const LOOKUP_LIST: u32 = 0x00cc_9d20;
pub const LOOKUP_USE: u32 = 0x0061_fdd0;

/// The file's virtual seek `0x0061ac30(vfile; distance, method)` and
/// `KERNEL32!SetFilePointer`'s import slot.
pub const VFILE_SEEK: u32 = 0x0061_ac30;
pub const SET_FILE_POINTER: u32 = 0x00b0_c148;

/// `0x006141a0`'s element destructor `0x004818b0(e)` (thiscall), stride.
pub const ELEMENT_DTOR: u32 = 0x0048_18b0;
/// `0x00614800`'s finishing call `0x0058e5b0(context; byte)`.
pub const FINISH: u32 = 0x0058_e5b0;

/// `GetType<PModifier>`'s descriptor, guard, constructor `0x005a0200`
/// (thiscall, 5 words), its arguments, vtable and `atexit` destructor.
pub const MODIFIER_TYPE: u32 = 0x00cb_d34c;
pub const MODIFIER_GUARD: u32 = 0x00cb_d380;
pub const NAMED_SEMANTIC_CTOR: u32 = 0x005a_0200;
pub const MODIFIER_NAME: u32 = 0x00b3_8288;
pub const MODIFIER_CALLBACKS: [u32; 2] = [0x005c_8580, 0x005c_86e0];
pub const MODIFIER_VTABLE: u32 = 0x00b3_8298;
pub const MODIFIER_ATEXIT: u32 = 0x00b0_4750;

/// `0x005c5e50`'s visitor: the singleton `0x0059e530()` and
/// `PDeleteChildrenVisitor`'s vtable.
pub const VISITOR_SINGLETON: u32 = 0x0059_e530;
pub const DELETE_VISITOR_VTABLE: u32 = 0x00b1_fac0;

/// The Iggy start-up's globals (the allocator block) and import slots
/// (stdcall): `IggyInit`, `IggySetWarningCallback`,
/// `IggySetTraceCallbackUTF8`, with the callbacks.
pub const IGGY_BLOCK: u32 = 0x01a8_58e4;
pub const IGGY_ALLOC: [u32; 2] = [0x00a3_e5a0, 0x00a3_e5c0];
pub const IGGY_INIT: u32 = 0x00b0_c804;
pub const IGGY_WARNING: u32 = 0x00b0_c80c;
pub const IGGY_TRACE: u32 = 0x00b0_c810;
pub const IGGY_CALLBACKS: [u32; 2] = [0x00a3_e600, 0x00a3_e5e0];

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ATEXIT, BAD_ALLOC, CREATE_MESH, CURSOR_FIRST, CURSOR_LOOKUP, CURSOR_NOTE, CURSOR_SKIP,
        CURSOR_TYPE, Conv, DELETE_VISITOR_VTABLE, DOF_COMMON, DOF_PARAMETERS, ELEMENT_DTOR,
        FIND_PARAMETER, FINISH, FREE, GET_TIME, Host, IGGY_ALLOC, IGGY_BLOCK, IGGY_CALLBACKS,
        IGGY_INIT, IGGY_TRACE, IGGY_WARNING, INPUT_DEVICE, INPUT_DIRTY, INPUT_ENUM, INPUT_LIST,
        IS_BOUND, JOB_TYPE, JOB_VTABLE, KEY_VALUE, LOOKUP, LOOKUP_LIST, LOOKUP_USE,
        MODIFIER_ATEXIT, MODIFIER_CALLBACKS, MODIFIER_GUARD, MODIFIER_NAME, MODIFIER_TYPE,
        MODIFIER_VTABLE, NAMED_SEMANTIC_CTOR, NEW, OBJECT_ADVANCE, OBJECT_COUNT, OBJECT_START,
        OCCLUDER_ADVANCE, OCCLUDER_COUNT, OCCLUDER_START, RELEASE_JOBS, RING_ALLOCATE, RING_CHECK,
        RING_SINGLETON, SET_DIMENSIONS, SET_FILE_POINTER, SET_VALUE, SLEEP, START_TIME,
        TARGET_RELEASE, TEXT_COLOR, TEXT_SET, THREAD_POOL, UNBIND, VFILE_SEEK, VISITOR_SINGLETON,
        x87,
    };

    /// A thunk ([`super::THUNKS`]): the target's result, `this` and the
    /// words passed on.
    pub fn thunk(h: &mut impl Host, target: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(target, Conv::Thiscall(this), args)
    }

    /// `PApplication::isKeyDown(this; key, i)` (`0x00628460`): the
    /// application's input (`+0x2b8`) asked [`KEY_VALUE`]; its float.
    pub fn is_key_down(h: &mut impl Host, app: u32, key: u32, i: u32) -> f32 {
        let input: u32 = h.read(app.wrapping_add(0x2b8));
        h.call_sym_st0(KEY_VALUE, Conv::Thiscall(input), &[key, i])
    }

    /// `PModifierNetworkDynamicMesh::sync(mesh)` (`0x005afca0`, stdcall):
    /// for a mesh, waits (`Sleep(1)`) until the counter `*[+0x28]`
    /// reaches `+0x2c` (no counter, no wait), then clears `+4`. `eax`
    /// is not defined (the caller's or `Sleep`'s).
    pub fn sync(h: &mut impl Host, mesh: u32) {
        if mesh == 0 {
            return;
        }
        let counter: u32 = h.read(mesh + 0x28);
        if counter != 0 {
            let target: u32 = h.read(mesh + 0x2c);
            while h.read::<u32>(counter) != target {
                h.call_sym(SLEEP, Conv::Cdecl, &[1]);
            }
        }
        h.write(mesh + 4, 0u32);
    }

    /// Whether a ring (cursor `ring + 8`, end `ring + 0xc`) is full for
    /// `size` more bytes: the check's new cursor `r` and wrapped cursor
    /// `o` (through `out`); with the cursor not past the end, a wrap or
    /// `r` past the end. Returns (full, r).
    fn full(h: &mut impl Host, ring: u32, size: u32, out: u32) -> (bool, u32) {
        let cur: u32 = h.read(ring + 8);
        let end: u32 = h.read(ring + 0xc);
        let r = h.call_sym(RING_CHECK, Conv::Thiscall(ring), &[cur, size, out]);
        let o: u32 = h.read(out);
        (cur <= end && (o < cur || r > end), r)
    }

    /// `0x005b6a70(renderer; bytes, commands)` (thiscall, `ret 8`):
    /// whether the data ring (`+0x1c`) has room for `bytes` and the
    /// command ring (`+0x30`) for `commands` commands of 0x48 bytes:
    /// `al` 1 or 0 over the last check's result. The checks write their
    /// wrapped cursor through `frame` (the original's `ebp - 4`) and
    /// `arg` (its first argument's slot, `ebp + 8`).
    pub fn has_room(
        h: &mut impl Host,
        renderer: u32,
        frame: u32,
        arg: u32,
        bytes: u32,
        commands: u32,
    ) -> u32 {
        let (data_full, r) = full(h, renderer + 0x1c, bytes, frame);
        if data_full {
            return r & !0xff;
        }
        let (full2, r) = full(h, renderer + 0x30, commands.wrapping_mul(0x48), arg);
        r & !0xff | u32::from(!full2)
    }

    /// `0x005c5e50(node; a)` (thiscall, `ret 4`): visits `node` with a
    /// `PDeleteChildrenVisitor` built in the frame (`frame`, the
    /// original's `ebp - 0x14`: the vtable, then `0x0059e530()`'s
    /// singleton) through virtual slot 15 `(a, &visitor)`; `al` 1 over
    /// that call's result.
    pub fn visit_delete(h: &mut impl Host, node: u32, frame: u32, a: u32) -> u32 {
        let single = h.call_sym(VISITOR_SINGLETON, Conv::Thiscall(node), &[]);
        h.write(frame + 4, single);
        let vt = h.rebase(DELETE_VISITOR_VTABLE);
        h.write(frame, vt);
        let r = h.call_virtual(node, 15, &[a, frame]);
        r & !0xff | 1
    }

    /// `PTypeDispenser::GetType<PModifier>()` (`0x005c7fe0`, cdecl): the
    /// named semantic descriptor, built on first use (guard bit 0; the
    /// constructor, the vtable, `atexit`; the SEH frame dropped).
    pub fn modifier_type(h: &mut impl Host) -> u32 {
        let guard = h.rebase(MODIFIER_GUARD);
        let g: u32 = h.read(guard);
        let desc = h.rebase(MODIFIER_TYPE);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let name = h.rebase(MODIFIER_NAME);
            let c0 = h.rebase(MODIFIER_CALLBACKS[0]);
            let c1 = h.rebase(MODIFIER_CALLBACKS[1]);
            h.call_sym(
                NAMED_SEMANTIC_CTOR,
                Conv::Thiscall(desc),
                &[name, 0x40, 4, c0, c1],
            );
            let vt = h.rebase(MODIFIER_VTABLE);
            h.write(desc, vt);
            let f = h.rebase(MODIFIER_ATEXIT);
            h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
        }
        desc
    }

    /// The file's seek `0x006080e0(file; distance, method)` (thiscall,
    /// `ret 8`): through the virtual file `+4` when there is one
    /// (`0x0061ac30`), else `SetFilePointer(handle, distance, 0, method)`
    /// on the handle `+0`, or 0 for an invalid handle.
    pub fn file_seek(h: &mut impl Host, file: u32, distance: u32, method: u32) -> u32 {
        let vfile: u32 = h.read(file + 4);
        if vfile != 0 {
            return h.call_sym(VFILE_SEEK, Conv::Thiscall(vfile), &[distance, method]);
        }
        let handle: u32 = h.read(file);
        if handle == u32::MAX {
            return 0;
        }
        let f: u32 = h.read(h.rebase(SET_FILE_POINTER));
        h.call(f, Conv::Stdcall, &[handle, distance, 0, method])
    }

    /// `0x006141a0(array)` (thiscall): an owned array (bit 31 of the
    /// count word clear) of 0x40-byte elements destroyed
    /// (`0x004818b0`) and freed; then its count and data cleared. `eax`:
    /// 0xfe for a view, 0 with no data, else the free's.
    pub fn array_destroy(h: &mut impl Host, array: u32) -> u32 {
        let word: u32 = h.read(array);
        let mut eax = 0xfe;
        if word >> 31 == 0 {
            let data: u32 = h.read(array + 4);
            eax = data;
            if data != 0 {
                let mut e = data;
                for _ in 0..word & 0x7fff_ffff {
                    h.call_sym(ELEMENT_DTOR, Conv::Thiscall(e), &[]);
                    e = e.wrapping_add(0x40);
                }
                eax = h.call_sym(FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(array + 4, 0u32);
        h.write(array, 0u32);
        eax
    }

    /// `0x00614800(list; context)` (thiscall, `ret 4`): when open (byte
    /// `+1`), each recorded (object, word) pair from `+8` (count
    /// `+0x2008`, read each round) has virtual slot 7 `(word)` called;
    /// the count cleared, `0x0058e5b0(context; byte +0)`, the list
    /// closed. Returns the word `+4` and clears it.
    pub fn flush(h: &mut impl Host, list: u32, context: u32) -> u32 {
        if h.read::<u8>(list + 1) != 0 {
            let mut i = 0u32;
            while i < h.read::<u32>(list + 0x2008) {
                let at = list + 8 + 8 * i;
                let obj: u32 = h.read(at);
                let word: u32 = h.read(at + 4);
                h.call_virtual(obj, 7, &[word]);
                i += 1;
            }
            let b: u8 = h.read(list);
            h.write(list + 0x2008, 0u32);
            h.call_sym(FINISH, Conv::Thiscall(context), &[u32::from(b)]);
            h.write(list + 1, 0u8);
        }
        let r: u32 = h.read(list + 4);
        h.write(list + 4, 0u32);
        r
    }

    /// `PInput::Update()` (`0x00625770`, cdecl): when flagged, the device
    /// re-enumerated (COM slot 4: `(0, 0x00624d40, 0, 1)`; 9 when it
    /// fails); then each input of the list (links at `+4` of each, the
    /// list's head its own end) updated (virtual slot 5). 0.
    pub fn input_update(h: &mut impl Host) -> u32 {
        let dirty = h.rebase(INPUT_DIRTY);
        if h.read::<u8>(dirty) != 0 {
            let device: u32 = h.read(h.rebase(INPUT_DEVICE));
            h.write(dirty, 0u8);
            let vt: u32 = h.read(device);
            let f: u32 = h.read(vt + 0x10);
            let cb = h.rebase(INPUT_ENUM);
            let r = h.call(f, Conv::Stdcall, &[device, 0, cb, 0, 1]);
            if (r as i32) < 0 {
                return 9;
            }
        }
        let head = h.rebase(INPUT_LIST);
        let mut link: u32 = h.read(head);
        while link != head && link != 0 && link != 4 {
            let input = link - 4;
            h.call_virtual(input, 5, &[]);
            link = h.read(input + 4);
        }
        0
    }

    /// `0x00625880(value)` (`ecx` = a pointer to a word, no stack words):
    /// a new 0x14-byte node (`operator new`) whose first three words are
    /// the word (read for each). With no memory it calls
    /// `std::_Xbad_alloc`, which does not return; 0 if it did.
    pub fn new_node(h: &mut impl Host, value: u32) -> u32 {
        let node = h.call_sym(NEW, Conv::Cdecl, &[0x14]);
        if node == 0 {
            let f: u32 = h.read(h.rebase(BAD_ALLOC));
            h.call(f, Conv::Cdecl, &[]);
            return 0;
        }
        for k in 0..3u32 {
            let at = node.wrapping_add(4 * k);
            if k == 0 || at != 0 {
                let w: u32 = h.read(value);
                h.write(at, w);
            }
        }
        node
    }

    /// `0x00626c40(p)` (stdcall, one word): the entry of the list at
    /// `0x00cc9d20` (`0x0043cbb0`, its block `- 4`); with one, `p`'s
    /// `0x0061fdd0(p; entry)` when `p` is set, and `al` 1; else `al` 0
    /// (`eax` 0).
    pub fn lookup_use(h: &mut impl Host, p: u32) -> u32 {
        let list = h.rebase(LOOKUP_LIST);
        let found = h.call_sym(LOOKUP, Conv::Thiscall(list), &[]);
        if found == 0 || found == 4 {
            return 0;
        }
        let entry = found - 4;
        let r = if p != 0 {
            h.call_sym(LOOKUP_USE, Conv::Thiscall(p), &[entry])
        } else {
            entry
        };
        r & !0xff | 1
    }

    /// `0x00628df0(timer)` (thiscall): the frame step. A fixed step
    /// (`+0x3c` above 0): the step to `+0x24`, the time `+0x20` advanced
    /// by it. Else the clock: `GetTime() - start - pause (+0x28)`, the
    /// step from `+0x20` to it, kept in `+0x20`. Either way the time is
    /// copied to `+0x120`.
    pub fn timer_step(h: &mut impl Host, timer: u32) {
        let fixed: u32 = h.read(timer + 0x3c);
        let f = x87::load(fixed);
        // fldz; fcomp: taken only for 0 < step (not for NaN)
        if 0.0 < f {
            h.write(timer + 0x24, x87::store(f));
            let t: u32 = h.read(timer + 0x20);
            let sum = x87::Reg::load(f32::from_bits(t)).add_m32(f32::from_bits(fixed));
            h.write(timer + 0x20, sum.store().to_bits());
        } else {
            let now = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
            let start: f64 = h.read(h.rebase(START_TIME));
            let pause: u32 = h.read(timer + 0x28);
            let t = x87::Reg::constant(x87::sub(now, start)).sub_m32(f32::from_bits(pause));
            let old: u32 = h.read(timer + 0x20);
            let step = t - x87::Reg::load(f32::from_bits(old));
            h.write(timer + 0x24, step.store().to_bits());
            h.write(timer + 0x20, t.store().to_bits());
        }
        let t: u32 = h.read(timer + 0x20);
        h.write(timer + 0x120, x87::round_trip(f32::from_bits(t)).to_bits());
    }

    /// `PCheckVisibleBoundsJob`'s creation `0x009f09e0()` (cdecl): 0x48
    /// bytes from the preprocess ring, waiting for room (the thread
    /// pool's slot 6, then `Sleep(1)` until there is); the job at
    /// `+0x10` (vtable, type word, 0, the block). The original's
    /// unreachable no-job path (a block at `-0x10`) writes the block to
    /// address 0xc; so does this.
    pub fn job_create(h: &mut impl Host) -> u32 {
        let mut block = alloc_job(h);
        if block == 0 {
            let pool = h.call_sym(THREAD_POOL, Conv::Cdecl, &[]);
            h.call_virtual(pool, 6, &[]);
            loop {
                h.call_sym(SLEEP, Conv::Cdecl, &[1]);
                block = alloc_job(h);
                if block != 0 {
                    break;
                }
            }
        }
        let job = block.wrapping_add(0x10);
        if job == 0 {
            h.write(0xc, block);
            return 0;
        }
        let ty = h.rebase(JOB_TYPE);
        h.write(job + 4, ty);
        h.write(job + 8, 0u32);
        let vt = h.rebase(JOB_VTABLE);
        h.write(job, vt);
        h.write(job + 0xc, block);
        job
    }

    fn alloc_job(h: &mut impl Host) -> u32 {
        // the 0x48 pushed before GetSingleton is allocate's argument
        let ring = h.call_sym(RING_SINGLETON, Conv::Cdecl, &[]);
        h.call_sym(RING_ALLOCATE, Conv::Thiscall(ring), &[0x48])
    }

    /// The job starters' loop over a cursor: while it has entries
    /// (`+0x18`), the start index (`+0x1c` + `+0xc`), the count
    /// (`count`), the batch word (`[[[cursor+8]+0xc]+0x1c]`), the cursor
    /// advanced by the count (`advance`), the jobs started (`start(this;
    /// index, count, word)`). Returns the last start's `eax`, or `None`.
    fn start_jobs(
        h: &mut impl Host,
        this: u32,
        cursor: u32,
        [count, advance, start]: [u32; 3],
    ) -> Option<u32> {
        let mut last = None;
        while h.read::<u32>(cursor + 0x18) != 0 {
            let a: u32 = h.read(cursor + 0x1c);
            let b: u32 = h.read(cursor + 0xc);
            let index = a.wrapping_add(b);
            let n = h.call_sym(count, Conv::Thiscall(cursor), &[]);
            let p: u32 = h.read(cursor + 8);
            let q: u32 = h.read(p + 0xc);
            let word: u32 = h.read(q + 0x1c);
            h.call_sym(advance, Conv::Thiscall(cursor), &[n]);
            last = Some(h.call_sym(start, Conv::Thiscall(this), &[index, n, word]));
        }
        last
    }

    /// `PVisibilityCheck::startOccluderJobs(this; cursor)` (`0x009f20f0`,
    /// `ret 4`). `eax`: the last start's (the caller's with no
    /// entries: not read by its caller).
    pub fn start_occluder_jobs(h: &mut impl Host, this: u32, cursor: u32) -> u32 {
        start_jobs(
            h,
            this,
            cursor,
            [OCCLUDER_COUNT, OCCLUDER_ADVANCE, OCCLUDER_START],
        )
        .unwrap_or(0)
    }

    /// `PVisibilityCheck::startJobsForAllBounds(this; cursor)`
    /// (`0x009f2250`, `ret 4`): the object jobs, then the words at
    /// `[this+0x24]` and `[this+0x28]` cleared. `eax`: the pointer
    /// `+0x28`.
    pub fn start_jobs_for_all_bounds(h: &mut impl Host, this: u32, cursor: u32) -> u32 {
        start_jobs(
            h,
            this,
            cursor,
            [OBJECT_COUNT, OBJECT_ADVANCE, OBJECT_START],
        );
        let p: u32 = h.read(this + 0x24);
        h.write(p, 0u32);
        let q: u32 = h.read(this + 0x28);
        h.write(q, 0u32);
        q
    }

    /// The occluder cursor's constructor `0x009f23f0(cursor; world)`
    /// (thiscall, `ret 4`) -> `cursor`: the world's block of the occluder
    /// type (`0x004437e0`); its entries from `+0x14`, count `+0x10` (-1
    /// with none); `0x00443ef0(cursor)`; `+0xc..+0x20` zeroed;
    /// `0x00441dc0(cursor + 0xc; block - 0x20)` for a block; then
    /// `skipEmpty`.
    pub fn occluder_cursor(h: &mut impl Host, cursor: u32, world: u32) -> u32 {
        let ty = h.rebase(CURSOR_TYPE);
        let block = h.call_sym(CURSOR_LOOKUP, Conv::Thiscall(world), &[ty]);
        h.write(cursor, block);
        h.write(cursor + 8, block);
        if block == 0 {
            h.write(cursor + 4, u32::MAX);
        } else {
            let n: u32 = h.read(block + 0x10);
            h.write(cursor + 4, n);
            h.write(cursor, block.wrapping_add(0x14));
        }
        h.call_sym(CURSOR_FIRST, Conv::Thiscall(cursor), &[]);
        for k in 0..5 {
            h.write(cursor + 0xc + 4 * k, 0u32);
        }
        let b: u32 = h.read(cursor + 8);
        if b != 0 {
            h.call_sym(
                CURSOR_NOTE,
                Conv::Thiscall(cursor + 0xc),
                &[b.wrapping_sub(0x20)],
            );
        }
        h.call_sym(CURSOR_SKIP, Conv::Thiscall(cursor), &[]);
        cursor
    }

    /// Clears a list of (next, object) links: each object's `+4`.
    fn unlink_all(h: &mut impl Host, mut node: u32) {
        while node != 0 {
            let next: u32 = h.read(node);
            let obj: u32 = h.read(node + 4);
            h.write(obj + 4, 0u32);
            node = next;
        }
    }

    /// `PVisibilityCheck::~PVisibilityCheck(this)` (`0x009f26c0`): the
    /// jobs released (`0x009f3590`), the two link lists `+0x1c`/`+0x20`
    /// unlinked and emptied (their tails `+0x28`/`+0x2c` back at the
    /// heads), the entry `+0x38` unlinked. `eax`: that entry.
    pub fn visibility_dtor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(RELEASE_JOBS, Conv::Thiscall(this), &[]);
        let first: u32 = h.read(this + 0x1c);
        unlink_all(h, first);
        let second: u32 = h.read(this + 0x20);
        h.write(this + 0x1c, 0u32);
        h.write(this + 0x28, this + 0x1c);
        unlink_all(h, second);
        let entry: u32 = h.read(this + 0x38);
        h.write(this + 0x20, 0u32);
        h.write(this + 0x2c, this + 0x20);
        if entry != 0 {
            h.write(entry + 4, 0u32);
            h.write(this + 0x38, 0u32);
            h.write(this + 0x3c, 0u32);
        }
        entry
    }

    /// `0x009f3590(this)` (thiscall): each job of the list `+0x18` (next
    /// at `+0x40`, read first) handed to the thread pool's slot 7 and its
    /// owner link `[+0xc]+4` cleared; the list emptied (tail `+0x24` at
    /// the head). `eax`: `this`, or the pool for an empty list.
    pub fn release_jobs(h: &mut impl Host, this: u32) -> u32 {
        let pool = h.call_sym(THREAD_POOL, Conv::Cdecl, &[]);
        let head = this + 0x18;
        let mut job: u32 = h.read(head);
        if job == 0 {
            h.write(this + 0x24, head);
            h.write(head, 0u32);
            return pool;
        }
        while job != 0 {
            let next: u32 = h.read(job + 0x40);
            h.call_virtual(pool, 7, &[job]);
            let owner: u32 = h.read(job + 0xc);
            if owner != 0 {
                h.write(owner + 4, 0u32);
            }
            job = next;
        }
        h.write(head, 0u32);
        h.write(this + 0x24, head);
        this
    }

    /// A post effect's deleting destructor (`this; flags`, `ret 4`):
    /// its vtable, the render target `+field` released
    /// (`0x00a0e520`), the base destructor, freed for flag bit 0.
    /// Returns `this`.
    pub fn deleting_dtor(
        h: &mut impl Host,
        this: u32,
        flags: u32,
        (vtable, field, base): (u32, u32, u32),
    ) -> u32 {
        let vt = h.rebase(vtable);
        h.write(this, vt);
        let target: u32 = h.read(this.wrapping_add(field));
        h.call_sym(TARGET_RELEASE, Conv::Cdecl, &[target]);
        h.call_sym(base, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `0x00a0e4b0(target, w, h, force)` (cdecl): resizes a 2D render
    /// target (kind byte `+4` 0, texture `+0`; else 1): when forced or
    /// the size (`+0x1c`, `+0x20`) differs, unbound if bound (its error
    /// returned) and `setDimensions(w, h, format +0, 0)` (its error
    /// returned). 0 when done.
    pub fn target_resize(h: &mut impl Host, target: u32, w: u32, ht: u32, force: u32) -> u32 {
        if target == 0 || h.read::<u8>(target + 4) != 0 {
            return 1;
        }
        let tex: u32 = h.read(target);
        if tex == 0 {
            return 1;
        }
        if force as u8 == 0 && h.read::<u32>(tex + 0x1c) == w && h.read::<u32>(tex + 0x20) == ht {
            return 0;
        }
        let bound = h.call_sym(IS_BOUND[0], Conv::Thiscall(tex), &[]);
        if bound as u8 != 0 {
            let r = h.call_sym(UNBIND[0], Conv::Thiscall(tex), &[]);
            if r != 0 {
                return r;
            }
        }
        let format: u32 = h.read(tex);
        h.call_sym(SET_DIMENSIONS, Conv::Thiscall(tex), &[w, ht, format, 0])
    }

    /// `0x00a0e520(target)` (cdecl): a render target's texture (kind byte
    /// `+4`: 2D, cube or 3D) unbound when bound. `eax`: the unbind's, the
    /// bound test's (`al` 0), the target for no texture or another kind,
    /// 0 for none.
    pub fn target_release(h: &mut impl Host, target: u32) -> u32 {
        if target == 0 {
            return 0;
        }
        let kind: u8 = h.read(target + 4);
        if kind > 2 {
            return target;
        }
        let tex: u32 = h.read(target);
        if tex == 0 {
            return target;
        }
        let k = usize::from(kind);
        let bound = h.call_sym(IS_BOUND[k], Conv::Thiscall(tex), &[]);
        if bound as u8 == 0 {
            return bound;
        }
        h.call_sym(UNBIND[k], Conv::Thiscall(tex), &[])
    }

    /// `0x00a1a6c0(this; a, effect)` (thiscall, `ret 8`):
    /// `PDepthOfFieldBase::initializeCommonShaders(a, effect)`; on success
    /// the three focus parameters looked up in `effect` into
    /// `+0xfb4..+0xfbc`, and 0; else its error.
    pub fn dof_shaders(h: &mut impl Host, this: u32, a: u32, effect: u32) -> u32 {
        let r = h.call_sym(DOF_COMMON, Conv::Thiscall(this), &[a, effect]);
        if r != 0 {
            return r;
        }
        for (name, field) in DOF_PARAMETERS {
            let n = h.rebase(name);
            let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(effect), &[n]);
            h.write(this.wrapping_add(field), p);
        }
        0
    }

    /// `0x00a2a650(material; rgb)` (thiscall, `ret 4`): the colour
    /// `(r, g, b, 1)` built in the frame (`frame`, the original's
    /// `ebp - 0x10`; each through `fld`/`fstp`) and set as the effect's
    /// "textColor" (`[material]`: effect `+0`, buffer `+4`); 9 with no
    /// effect or no such parameter.
    pub fn text_color(h: &mut impl Host, material: u32, frame: u32, rgb: u32) -> u32 {
        let fx: u32 = h.read(material);
        for k in 0..3 {
            let c: u32 = h.read(rgb + 4 * k);
            h.write(frame + 4 * k, x87::round_trip(f32::from_bits(c)).to_bits());
        }
        h.write(frame + 12, 1.0f32.to_bits());
        if fx == 0 {
            return 9;
        }
        let effect: u32 = h.read(fx);
        let name = h.rebase(TEXT_COLOR);
        let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(effect), &[name]);
        if p == 0 {
            return 9;
        }
        let buffer: u32 = h.read(fx + 4);
        h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, frame])
    }

    /// `0x00a2ad80(text; a)` (thiscall, `ret 4`): `0x00a2a110(a)`; on
    /// success the mesh rebuilt for the string `+0x160`'s length. `eax`:
    /// the last call's.
    pub fn text_set(h: &mut impl Host, text: u32, a: u32) -> u32 {
        let r = h.call_sym(TEXT_SET, Conv::Thiscall(text), &[a]);
        if r != 0 {
            return r;
        }
        let s: u32 = h.read(text + 0x160);
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        h.call_sym(CREATE_MESH, Conv::Thiscall(text), &[n])
    }

    /// The Iggy start-up `0x00a3e6d0()` (cdecl): the allocator block
    /// (0, then the two callbacks) to `IggyInit`, the warning and trace
    /// callbacks set. 0.
    pub fn iggy_init(h: &mut impl Host) -> u32 {
        let block = h.rebase(IGGY_BLOCK);
        let a0 = h.rebase(IGGY_ALLOC[0]);
        let a1 = h.rebase(IGGY_ALLOC[1]);
        h.write(block + 4, a0);
        h.write(block + 8, a1);
        h.write(block, 0u32);
        let f: u32 = h.read(h.rebase(IGGY_INIT));
        h.call(f, Conv::Stdcall, &[block]);
        let cb = h.rebase(IGGY_CALLBACKS[0]);
        let f: u32 = h.read(h.rebase(IGGY_WARNING));
        h.call(f, Conv::Stdcall, &[cb, 0]);
        let cb = h.rebase(IGGY_CALLBACKS[1]);
        let f: u32 = h.read(h.rebase(IGGY_TRACE));
        h.call(f, Conv::Stdcall, &[cb, 0]);
        0
    }
}
