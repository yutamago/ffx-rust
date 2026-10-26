//! `PRenderThread::processRenderCommands` (`0x00615550`, list
//! p6-render-c; `m4-rest`): the render thread plays a run of 72-byte
//! render commands, the dynamic-geometry jobs issued one step ahead.
//!
//! `thiscall(thread; first, end)`, `ret 8` -> 0. The frame (the bridge
//! takes the original's `ebp`; the callees see these addresses):
//! - `ebp - 0x4058`: the render scope lock (`PRenderScopeLock_enter` /
//!   `_leave`);
//! - `ebp - 0x4084`: a pooled handle `{0, object}` (the object at `- 0x4080`
//!   built by `0x00583b80` with the render context `0x00c94f00`), the
//!   shader pass state every command handler gets;
//! - `ebp - 0x402c`: two 0x200c-byte job lists (`0x00613f70` /
//!   `0x00614200`, built and destroyed by the CRT's `eh vector`
//!   iterators), byte `+0` their index, byte `+1` "jobs issued";
//! - `ebp - 0x4060`: `{context, word}` handed to a callback command.
//!
//! The commands (type at `+0`): 1 and 2 (a mesh draw: skipped when its
//! mesh's stream blocks or index block are already bound, else the pass
//! set to `+8` and `PRenderThread_runMeshCommand(2)` called through the
//! original's call sites (which the Rust map path re-points); then the
//! reference counts listed at `+0x38` are decremented), 3 (a target
//! switch into the thread's two alternating records at `+0x190`), 0x2c (a
//! callback `[+8](&{context, [+0xc]})`), anything else
//! `handleNonRenderInstanceCommand`.

use ffx_graphic::host::{Conv, Host};

/// `PRenderScopeLock_enter_00463d50` / `_leave_00463e90` (thiscall
/// `(lock)`).
pub const LOCK_ENTER: u32 = 0x0046_3d50;
pub const LOCK_LEAVE: u32 = 0x0046_3e90;
/// The render context singleton and its construction guard (bit 0).
pub const CONTEXT: u32 = 0x00c9_4f00;
pub const CONTEXT_GUARD: u32 = 0x00c9_628c;
/// `Phyre_PRendering_Construct583580_004fee70` (thiscall `(context)`).
pub const CONTEXT_CTOR: u32 = 0x004f_ee70;
/// `atexit` (cdecl `(f)`) and the context's exit destructor.
pub const ATEXIT: u32 = 0x0094_9207;
pub const CONTEXT_DTOR: u32 = 0x00af_b710;
/// `Phyre_PRendering_Ctor_00583b80` (thiscall `(object, context)`,
/// `ret 4`).
pub const PASS_CTOR: u32 = 0x0058_3b80;
/// The pass object's destructor (thiscall, `ret`).
pub const PASS_DTOR: u32 = 0x0058_52f0;
/// The CRT's `eh vector constructor iterator` (stdcall `(p, size, n,
/// ctor, dtor)`) and `eh vector destructor iterator` (stdcall `(p, size,
/// n, dtor)`).
pub const VECTOR_CTOR: u32 = 0x0094_966a;
pub const VECTOR_DTOR: u32 = 0x0094_96d2;
/// The job lists' constructor and destructor.
pub const LIST_CTOR: u32 = 0x0061_3f70;
pub const LIST_DTOR: u32 = 0x0061_4200;
/// `PRenderThread_issueDynamicJobs_006148d0` (thiscall `(list, context,
/// pass, first, end)`, `ret 0x10`) -> the command it stopped at.
pub const ISSUE_JOBS: u32 = 0x0061_48d0;
/// `PRenderThread_flushReleases_00614800` (thiscall `(list, context)`,
/// `ret 4`) -> the command the jobs reached.
pub const FLUSH: u32 = 0x0061_4800;
/// `Phyre_PGeometry_PDynamicMesh_getDataBlockCount` (thiscall `(mesh,
/// segment)`, `ret 4`).
pub const BLOCK_COUNT: u32 = 0x0049_8770;
/// `PRenderThread_setShaderPass_0061c400` (thiscall `(pass, x)`, `ret
/// 4`).
pub const SET_PASS: u32 = 0x0061_c400;
/// `PRenderThread_runMeshCommand` / `2` (cdecl `(context, command + 8)`).
pub const RUN_MESH: u32 = 0x0061_44f0;
pub const RUN_MESH2: u32 = 0x0061_4610;
/// The original's calls of those two (`call rel32` at these sites): the
/// Rust map path (`ffx-hook` `gfx::scene`, `FFX_RUST_MAP`) re-points
/// them, so the port calls whatever they call now.
pub const HANDLER_SITES: [u32; 2] = [0x0061_57cf, 0x0061_5898];
/// `0x00592b00` (thiscall `(context, a, b, c)`, `ret 0xc`).
pub const SET_TARGETS: u32 = 0x0059_2b00;
/// `Phyre_PRendering_PooledHandle_release_0061c890` (thiscall
/// `(handle)`).
pub const RELEASE: u32 = 0x0061_c890;
/// `PRenderThread::handleNonRenderInstanceCommand` (thiscall `(thread,
/// context, command, pass)`, `ret 0xc`).
pub const OTHER_COMMAND: u32 = 0x0061_4c60;
/// `InterlockedExchangeAdd`'s import slot (stdcall `(p, v)`).
pub const INTERLOCKED_ADD: u32 = 0x00b0_c128;
/// The frame's size below `ebp`.
pub const FRAME: u32 = 0x4084;
/// A command's size.
pub const COMMAND: u32 = 0x48;
/// A job list's size.
pub const LIST: u32 = 0x200c;

pub mod bridge {
    //! The original over game memory.

    use super::{
        ATEXIT, BLOCK_COUNT, COMMAND, CONTEXT, CONTEXT_CTOR, CONTEXT_DTOR, CONTEXT_GUARD, Conv,
        FLUSH, HANDLER_SITES, Host, INTERLOCKED_ADD, ISSUE_JOBS, LIST, LIST_CTOR, LIST_DTOR,
        LOCK_ENTER, LOCK_LEAVE, OTHER_COMMAND, PASS_CTOR, PASS_DTOR, RELEASE, SET_PASS,
        SET_TARGETS, VECTOR_CTOR, VECTOR_DTOR,
    };

    /// The frame at the original's `ebp`.
    struct Frame(u32);

    impl Frame {
        fn at(&self, off: u32) -> u32 {
            self.0.wrapping_sub(off)
        }
        /// The pass state handle.
        fn pass(&self) -> u32 {
            self.at(0x4084)
        }
        fn list(&self, slot: u32) -> u32 {
            self.at(0x402c).wrapping_add(slot.wrapping_mul(LIST))
        }
    }

    /// Decrements each reference count the list at `list` names (`+0`
    /// the count, re-read each time; then the counters' addresses).
    fn release_refs(h: &mut impl Host, f: &Frame, list: u32) {
        h.write(f.at(0x4038), list);
        if list == 0 {
            return;
        }
        let add: u32 = h.read(h.rebase(INTERLOCKED_ADD));
        let mut i = 0;
        while i < h.read::<u32>(list) {
            let counter: u32 = h.read(list.wrapping_add(4 + 4 * i));
            h.call(add, Conv::Stdcall, &[counter, u32::MAX]);
            i += 1;
        }
    }

    /// Whether a mesh command's data is bound already: one of its stream
    /// blocks (`[cmd + 0x10]`, 0x40 bytes each, `+0x38` set and byte
    /// `+0x14`) or its index block (`[cmd + 0x3c]` when the segment has
    /// indices: `+0x34` set and byte `+0xe`).
    fn bound(h: &mut impl Host, f: &Frame, cmd: u32, mesh: u32) -> bool {
        h.write(f.at(0x4038), mesh);
        let segment = u32::from(h.read::<u16>(cmd + 0x24));
        let blocks_n = h.call_sym(BLOCK_COUNT, Conv::Thiscall(mesh), &[segment]);
        let segment = u32::from(h.read::<u16>(cmd + 0x24));
        let segments: u32 = h.read(mesh + 0x14);
        let index_block = if h.read::<u32>(segments.wrapping_add(segment * 16) + 0xc) == 0 {
            0
        } else {
            h.read(cmd + 0x3c)
        };
        let blocks: u32 = h.read(cmd + 0x10);
        if blocks != 0 {
            for i in 0..blocks_n {
                let b = blocks.wrapping_add(0x14 + 0x40 * i);
                if h.read::<u32>(b + 0x24) != 0 && h.read::<u8>(b) != 0 {
                    return true;
                }
            }
        }
        index_block != 0
            && h.read::<u32>(index_block + 0x34) != 0
            && h.read::<u8>(index_block + 0xe) != 0
    }

    /// Where the original's call at `site` goes now (`call rel32`).
    fn call_target(h: &impl Host, site: u32) -> u32 {
        let at = h.rebase(site);
        let rel: u32 = h.read(at + 1);
        at.wrapping_add(5).wrapping_add(rel)
    }

    /// One command.
    fn play(h: &mut impl Host, f: &Frame, thread: u32, cmd: u32) {
        let context = h.rebase(CONTEXT);
        match h.read::<u32>(cmd) {
            kind @ (1 | 2) => {
                let mesh: u32 = h.read(cmd + 0xc);
                h.write(f.at(0x4038), mesh);
                if mesh == 0 || !bound(h, f, cmd, mesh) {
                    h.call_sym(SET_PASS, Conv::Thiscall(f.pass()), &[cmd + 8]);
                    let run = call_target(h, HANDLER_SITES[kind as usize - 1]);
                    h.call(run, Conv::Cdecl, &[context, cmd + 8]);
                }
                let list: u32 = h.read(cmd + 0x38);
                release_refs(h, f, list);
            }
            3 => {
                let s: u32 = h.read(thread + 0x208);
                let rec = thread.wrapping_add((s.wrapping_mul(15).wrapping_add(100)) << 2);
                h.write(thread + 0x208, 1_u32.wrapping_sub(s));
                for (to, from) in [
                    (0x14, 8),
                    (0x20, 0xc),
                    (0x24, 0x10),
                    (0x28, 0x14),
                    (0x2c, 0x18),
                ] {
                    let w: u32 = h.read(cmd + from);
                    h.write(rec.wrapping_add(to), w);
                }
                h.call_sym(SET_PASS, Conv::Thiscall(f.pass()), &[rec]);
                let a: u32 = h.read(cmd + 0x20);
                let b: u32 = h.read(cmd + 0x24);
                let c: u32 = h.read(cmd + 0x28);
                h.call_sym(SET_TARGETS, Conv::Thiscall(context), &[a, b, c]);
                let list: u32 = h.read(cmd + 0x1c);
                release_refs(h, f, list);
            }
            0x2c => {
                h.call_sym(RELEASE, Conv::Thiscall(f.pass()), &[]);
                let word: u32 = h.read(cmd + 0xc);
                h.write(f.at(0x405c), word);
                h.write(f.at(0x4060), context);
                let callback: u32 = h.read(cmd + 8);
                h.call(callback, Conv::Cdecl, &[f.at(0x4060)]);
            }
            _ => {
                h.call_sym(
                    OTHER_COMMAND,
                    Conv::Thiscall(thread),
                    &[context, cmd, f.pass()],
                );
            }
        }
    }

    /// `processRenderCommands(thread; first, end)` with the original's
    /// frame at `ebp`: see the module. Returns 0.
    pub fn process(h: &mut impl Host, thread: u32, first: u32, end: u32, ebp: u32) -> u32 {
        let f = Frame(ebp);
        let context = h.rebase(CONTEXT);
        h.write(f.at(0x404c), thread);
        h.write(f.at(0x4044), first);
        h.write(f.at(0x403c), end);
        h.call_sym(LOCK_ENTER, Conv::Thiscall(f.at(0x4058)), &[]);
        let guard = h.rebase(CONTEXT_GUARD);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            h.call_sym(CONTEXT_CTOR, Conv::Thiscall(context), &[]);
            let dtor = h.rebase(CONTEXT_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        h.write(f.pass(), 0_u32);
        h.call_sym(PASS_CTOR, Conv::Thiscall(f.at(0x4080)), &[context]);
        let mut slot: u32 = h.read(thread + 0x158);
        h.write(f.at(0x4030), f.pass());
        h.write(f.at(0x4034), context);
        let ctor = h.rebase(LIST_CTOR);
        let dtor = h.rebase(LIST_DTOR);
        h.call_sym(
            VECTOR_CTOR,
            Conv::Stdcall,
            &[f.list(0), LIST, 2, ctor, dtor],
        );
        h.write(f.at(0x14), slot);
        for i in 0..2_u32 {
            h.write(f.list(i), i as u8);
        }
        // the first jobs, one step ahead of the commands
        let list = f.list(slot);
        h.write(f.at(0x4048), list);
        let mut next = h.call_sym(
            ISSUE_JOBS,
            Conv::Thiscall(list),
            &[context, f.pass(), first, end],
        );
        let mut previous = list;
        if h.read::<u8>(list + 1) == 0 {
            previous = 0;
            h.write(f.at(0x4048), 0_u32);
        } else {
            slot = 1 - slot;
            h.write(f.at(0x14), slot);
        }
        let mut cur = first;
        while cur != end {
            let list = f.list(slot);
            let after = h.call_sym(
                ISSUE_JOBS,
                Conv::Thiscall(list),
                &[context, f.pass(), next, end],
            );
            h.write(f.at(0x4054), after);
            let issued = if h.read::<u8>(list + 1) == 0 {
                0
            } else {
                slot = 1 - slot;
                h.write(f.at(0x14), slot);
                list
            };
            let mut stop = end;
            h.write(f.at(0x4050), stop);
            if previous != 0 {
                stop = h.call_sym(FLUSH, Conv::Thiscall(previous), &[context]);
                h.write(f.at(0x4050), stop);
            }
            h.write(f.at(0x4048), issued);
            previous = issued;
            if cur != stop {
                h.write(f.at(0x4040), cur + 0x3c);
                loop {
                    play(h, &f, thread, cur);
                    cur = cur.wrapping_add(COMMAND);
                    h.write(f.at(0x4044), cur);
                    h.write(f.at(0x4040), cur.wrapping_add(0x3c));
                    if cur == h.read::<u32>(f.at(0x4050)) {
                        break;
                    }
                }
            }
            next = after;
        }
        h.write(thread + 0x158, slot);
        h.call_sym(RELEASE, Conv::Thiscall(f.pass()), &[]);
        h.call_sym(VECTOR_DTOR, Conv::Stdcall, &[f.list(0), LIST, 2, dtor]);
        h.call_sym(RELEASE, Conv::Thiscall(f.pass()), &[]);
        h.call_sym(PASS_DTOR, Conv::Thiscall(f.at(0x4080)), &[]);
        h.call_sym(LOCK_LEAVE, Conv::Thiscall(f.at(0x4058)), &[]);
        0
    }
}
