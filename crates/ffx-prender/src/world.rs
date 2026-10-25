//! A world's culled and sorted render: `PApplication::renderWorld`
//! `0x0062be70`, `PRenderer::renderWorldCulledSorted` `0x009f3c30` and
//! `PVisibilityCheck::mergeResults` `0x009f3660` (docs/draw.md, "The map
//! path").
//!
//! `renderWorldCulledSorted(renderer, world, camera, ctx, pass)` (thiscall,
//! `ret 0x10`, returns 0) builds three objects in its frame: two world
//! cursors (`PMeshInstanceBounds` `0x004ff6e0`, `POccluderGeometryInstance`
//! `0x009f23f0`) and the `PVisibilityCheck` ([`crate::visibility`]); starts
//! and builds the occluders (`0x009f20f0`, `0x009f2fd0`; none on the field
//! maps), starts the culling jobs
//! (`startJobsForAllBounds` `0x009f2250`), waits for them and counts their
//! entries (`0x009f30d0`), merges the jobs' sorted entries into one list of
//! instances in the preprocess ring (`mergeResults`), renders that list
//! (`renderMeshInstanceList` through its thunk `0x005ba360`) and frees the
//! jobs (`0x009f26c0`). The original's SEH frame only runs `0x009f26c0`
//! when a callee throws; nothing on this path throws, so the port has
//! none.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::visibility;

/// `PWorldCursor_MeshInstanceBounds` (thiscall `(cursor, world)`).
pub const BOUNDS_CURSOR: u32 = 0x004f_f6e0;
/// `PWorldCursor_OccluderGeometryInstance` (thiscall `(cursor, world)`).
pub const OCCLUDER_CURSOR: u32 = 0x009f_23f0;
/// `PVisibilityCheck_startOccluderJobs` (thiscall `(check, cursor)`).
pub const START_OCCLUDERS: u32 = 0x009f_20f0;
/// `PVisibilityCheck_buildOccluders` (thiscall `(check)`).
pub const BUILD_OCCLUDERS: u32 = 0x009f_2fd0;
/// `startJobsForAllBounds` (thiscall `(check, cursor)`).
pub const START_JOBS: u32 = 0x009f_2250;
/// `0x009f30d0` (thiscall `(check)` -> entries): waits for the jobs.
pub const WAIT_JOBS: u32 = 0x009f_30d0;
/// `Phyre_PPreprocessRingBuffer_GetSingleton` (no arguments).
pub const RING_SINGLETON: u32 = 0x005c_d010;
/// `Phyre_PPreprocessRingBuffer_allocate` (thiscall `(ring, size)`).
pub const RING_ALLOCATE: u32 = 0x005c_d0f0;
/// `renderMeshInstanceList`'s thunk (thiscall `(renderer, first, last,
/// ctx)`, `ret 0xc`).
pub const RENDER_LIST: u32 = 0x005b_a360;
/// `0x009f26c0` (thiscall `(check)`): frees the jobs.
pub const FREE_JOBS: u32 = 0x009f_26c0;
/// `renderWorldCulledSorted`.
pub const CULLED_SORTED: u32 = 0x009f_3c30;

/// Frame bytes [`bridge::culled_sorted`] needs.
pub const FRAME: u32 = 0x90 + visibility::INIT_FRAME;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BOUNDS_CURSOR, BUILD_OCCLUDERS, CULLED_SORTED, Conv, FREE_JOBS, Host, Mem, OCCLUDER_CURSOR,
        RENDER_LIST, RING_ALLOCATE, RING_SINGLETON, START_JOBS, START_OCCLUDERS, WAIT_JOBS,
    };
    use crate::visibility;

    /// `PApplication::renderWorld(app, world, camera, pass)` (thiscall,
    /// `ret 0xc`): `renderWorldCulledSorted(app + 0x124, world, camera,
    /// app + 0x40, pass)`, its `eax`. Calls the original's address (hooked
    /// or not), as the original does.
    pub fn render_world(h: &mut impl Host, app: u32, world: u32, camera: u32, pass: u32) -> u32 {
        let f = h.rebase(CULLED_SORTED);
        h.call(
            f,
            Conv::Thiscall(app.wrapping_add(0x124)),
            &[world, camera, app.wrapping_add(0x40), pass],
        )
    }

    /// Frame offsets: the port's own matrix first (`PVisibilityCheck`'s
    /// camera matrix, in the original a callee's frame), then the
    /// original's locals from its `ebp - 0x9c` on. The frame ends where the
    /// original's locals end, so a test can put it at their address
    /// without touching the caller's stack.
    const INIT: u32 = 0x00;
    const LOCALS: u32 = visibility::INIT_FRAME;
    const MESH_BOUNDS: u32 = LOCALS;
    const OCCLUDERS: u32 = LOCALS + 0x20;
    const CHECK: u32 = LOCALS + 0x40;
    const RANGE: u32 = LOCALS + 0x80;
    const OUT: u32 = LOCALS + 0x88;

    /// `renderWorldCulledSorted(renderer, world, camera, ctx, pass)`:
    /// 0. `frame` is [`super::FRAME`] bytes: `frame + 0x40` is the
    /// original's `ebp - 0x9c`.
    #[allow(clippy::cast_possible_wrap)] // the pass is a signed word
    pub fn culled_sorted(
        h: &mut impl Host,
        frame: u32,
        renderer: u32,
        world: u32,
        camera: u32,
        ctx: u32,
        pass: u32,
    ) -> u32 {
        let check = frame + CHECK;
        let f = h.rebase(BOUNDS_CURSOR);
        h.call(f, Conv::Thiscall(frame + MESH_BOUNDS), &[world]);
        let f = h.rebase(OCCLUDER_CURSOR);
        h.call(f, Conv::Thiscall(frame + OCCLUDERS), &[world]);
        visibility::bridge::init(h, frame + INIT, check, camera, pass as i32);
        let f = h.rebase(START_OCCLUDERS);
        h.call(f, Conv::Thiscall(check), &[frame + OCCLUDERS]);
        let f = h.rebase(BUILD_OCCLUDERS);
        h.call(f, Conv::Thiscall(check), &[]);
        let f = h.rebase(START_JOBS);
        h.call(f, Conv::Thiscall(check), &[frame + MESH_BOUNDS]);
        let f = h.rebase(WAIT_JOBS);
        let n = h.call(f, Conv::Thiscall(check), &[]);
        if n != 0 {
            let bytes = n.wrapping_mul(4);
            let f = h.rebase(RING_SINGLETON);
            let ring = h.call(f, Conv::Cdecl, &[]);
            let f = h.rebase(RING_ALLOCATE);
            let block = h.call(f, Conv::Thiscall(ring), &[bytes]);
            let first = block.wrapping_add(0x10);
            h.write(frame + RANGE, first);
            h.write(frame + RANGE + 4, first.wrapping_add(bytes));
            merge(h, check, frame + OUT, frame + RANGE);
            let (a, b): (u32, u32) = (h.read(frame + OUT), h.read(frame + OUT + 4));
            let f = h.rebase(RENDER_LIST);
            h.call(f, Conv::Thiscall(renderer), &[a, b, ctx.wrapping_add(8)]);
            h.write(block.wrapping_add(4), 0u32);
        }
        let f = h.rebase(FREE_JOBS);
        h.call(f, Conv::Thiscall(check), &[]);
        0
    }

    /// `PVisibilityCheck::mergeResults(check, out, range)` (thiscall,
    /// `ret 8`, returns `out`): takes the jobs' sorted entries (the job
    /// list at `check +0x18`, linked through `+0x40`; a job's entries run
    /// from `+0x30` to `+0x2c`) in order, the smallest head first: render
    /// class 1 before the rest, then the key strictly smaller, the earlier
    /// job on ties. Writes each entry's instance word from `*range` on,
    /// unlinks a job when it runs out (freeing its ring block, `+0xc`),
    /// then frees the done-job list (`+0x1c`). `out` gets the written range,
    /// or two zeros when nothing was written.
    pub fn merge(h: &mut impl Host, check: u32, out: u32, range: u32) -> u32 {
        let start: u32 = h.read(range);
        let mut dst = start;
        loop {
            let head = check + 0x18;
            let mut best: u32 = h.read(head);
            if best == 0 {
                break;
            }
            let mut link = head;
            let (mut key, mut class) = head_of(h, best);
            let mut prev = best + 0x40;
            let mut cand: u32 = h.read(prev);
            while cand != 0 {
                let (k, c) = head_of(h, cand);
                let take = if class == c { k < key } else { c };
                if take {
                    best = cand;
                    link = prev;
                    key = k;
                    class = c;
                }
                prev = cand + 0x40;
                cand = h.read(prev);
            }
            let at: u32 = h.read(best + 0x30);
            let inst: u32 = h.read(at);
            h.write(dst, inst);
            h.write(best + 0x30, at + 12);
            dst += 4;
            if at + 12 == h.read::<u32>(best + 0x2c) {
                let next: u32 = h.read(best + 0x40);
                h.write(link, next);
                let block: u32 = h.read(best + 0xc);
                if block != 0 {
                    h.write(block + 4, 0u32);
                }
            }
        }
        let mut p: u32 = h.read(check + 0x1c);
        while p != 0 {
            let next: u32 = h.read(p);
            let block: u32 = h.read(p + 4);
            h.write(block + 4, 0u32);
            p = next;
        }
        h.write(check + 0x1c, 0u32);
        h.write(check + 0x28, check + 0x1c);
        let (first, last) = if start == dst { (0, 0) } else { (start, dst) };
        h.write(out + 4, last);
        h.write(out, first);
        out
    }

    /// A job's head entry: its key (copied through the x87) and whether
    /// its render order is class 1.
    fn head_of(h: &impl Mem, job: u32) -> (f32, bool) {
        let e: u32 = h.read(job + 0x30);
        let key: f32 = h.read(e + 4);
        let order: i32 = h.read(e + 8);
        (
            ffx_bridge::x87::round_trip(key),
            order / crate::sort::CLASS == 1,
        )
    }
}
