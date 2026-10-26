//! `ffx-prender`: `PhyreEngine`'s map-world render chain (the M4 pilot,
//! `~/ffx-work/m4/pilot-report.md`): `renderWorld`, the culling and sort,
//! the merge.
//!
//! Threads: `PCheckVisibleBoundsJob_execute` and the sort it calls run on
//! the `PhyreEngine` job workers (and on the main thread when it waits for
//! a job it then runs itself); the rest on the main thread. Each shim's
//! frame is a [`super::StackFrame`] on its own stack (at most `0x210`
//! bytes), never shared memory.

use super::{StackFrame, mem, ported};
use ffx_prender::{mesh, sort, thread, visibility, world};

/// Words of the culling job's frame.
const EXECUTE_WORDS: usize = visibility::EXECUTE_FRAME as usize / 4;
/// Words of `renderWorldCulledSorted`'s frame.
const CULLED_WORDS: usize = world::FRAME as usize / 4;

ported! {
    /// `PCheckVisibleBoundsJob_compareEntries(a, b)`: `al`, as the sort's
    /// `test al, al` reads it.
    PCheckVisibleBoundsJob_compareEntries @ 0x009f_2d70 => extern "C" fn compare_entries(a: u32, b: u32) -> u8 {
        u8::from(sort::bridge::compare_entries(&mem(), a, b))
    }
    /// `std_sort_VisibleEntries(first, last, ideal, pred)`.
    std_sort_VisibleEntries @ 0x009f_17c0 => extern "C" fn sort_entries(first: u32, last: u32, ideal: i32, pred: u32) {
        let mut tmp = StackFrame::<4>::new();
        sort::bridge::sort(&mut mem(), tmp.at(), first, last, ideal, pred);
    }
    /// `std_insertion_sort_VisibleEntries(first, last, pred, unused)`.
    std_insertion_sort_VisibleEntries @ 0x009f_0d20 => extern "C" fn insertion_sort(first: u32, last: u32, pred: u32, _unused: u32) {
        let mut tmp = StackFrame::<4>::new();
        sort::bridge::insertion_sort(&mut mem(), tmp.at(), first, last, pred);
    }
    /// `std_med3_VisibleEntries(a, b, c, pred)`.
    std_med3_VisibleEntries @ 0x009f_10e0 => extern "C" fn median3(a: u32, b: u32, c: u32, pred: u32) {
        sort::bridge::median3(&mut mem(), a, b, c, pred);
    }
    /// `std_median_VisibleEntries(first, mid, last, pred)`.
    std_median_VisibleEntries @ 0x009f_1230 => extern "C" fn median(first: u32, mid: u32, last: u32, pred: u32) {
        sort::bridge::median(&mut mem(), first, mid, last, pred);
    }
    /// `std_unguarded_partition_VisibleEntries(out, first, last, pred)` ->
    /// `out`.
    std_unguarded_partition_VisibleEntries @ 0x009f_1c60 => extern "C" fn partition(out: u32, first: u32, last: u32, pred: u32) -> u32 {
        sort::bridge::partition_out(&mut mem(), out, first, last, pred)
    }
    /// `std_push_heap_VisibleEntries(first, hole, top, val, pred)`.
    std_push_heap_VisibleEntries @ 0x009f_1630 => extern "C" fn push_heap(first: u32, hole: i32, top: i32, val: u32, pred: u32) {
        sort::bridge::push_heap(&mut mem(), first, hole, top, val, pred);
    }
    /// `std_adjust_heap_VisibleEntries(first, hole, bottom, val, pred)`.
    std_adjust_heap_VisibleEntries @ 0x009f_0b60 => extern "C" fn adjust_heap(first: u32, hole: i32, bottom: i32, val: u32, pred: u32) {
        sort::bridge::adjust_heap(&mut mem(), first, hole, bottom, val, pred);
    }
    /// `std_make_heap_VisibleEntries(first, last, pred, 0, 0)`.
    std_make_heap_VisibleEntries @ 0x009f_0f50 => extern "C" fn make_heap(first: u32, last: u32, pred: u32, _a: u32, _b: u32) {
        let mut tmp = StackFrame::<4>::new();
        sort::bridge::make_heap(&mut mem(), tmp.at(), first, last, pred);
    }
    /// `std_sort_heap_VisibleEntries(first, last, pred)`.
    std_sort_heap_VisibleEntries @ 0x009f_1970 => extern "C" fn sort_heap(first: u32, last: u32, pred: u32) {
        let mut tmp = StackFrame::<4>::new();
        sort::bridge::sort_heap(&mut mem(), tmp.at(), first, last, pred);
    }
    /// `PVisibilityCheck::PVisibilityCheck(this, camera, pass)` -> `this`.
    Phyre_PRendering_PVisibilityCheck_PVisibilityCheck @ 0x009f_2580 => extern "thiscall" fn visibility_check(this: u32, camera: u32, pass: i32) -> u32 {
        let mut frame = StackFrame::<16>::new();
        visibility::bridge::init(&mut mem(), frame.at(), this, camera, pass)
    }
    /// `PCheckVisibleBoundsJob_execute(job, ...)` (vtable slot 1; pops
    /// three words it does not read).
    PCheckVisibleBoundsJob_execute @ 0x009f_3160 => extern "thiscall" fn culling_job(job: u32, _a: u32, _b: u32, _c: u32) {
        let mut frame = StackFrame::<EXECUTE_WORDS>::new();
        visibility::bridge::execute(&mut mem(), frame.at(), job);
    }
    /// `PVisibilityCheck::mergeResults(check, out, range)` -> `out`.
    Phyre_PRendering_PVisibilityCheck_mergeResults @ 0x009f_3660 => extern "thiscall" fn merge_results(check: u32, out: u32, range: u32) -> u32 {
        world::bridge::merge(&mut mem(), check, out, range)
    }
    /// `PRenderThread_runMeshCommand(ctx, cmd)` (render thread).
    PRenderThread_runMeshCommand @ 0x0061_44f0 => extern "C" fn run_mesh_command(ctx: u32, cmd: u32) {
        let mut frame = StackFrame::<2>::new();
        thread::bridge::run_mesh_command(&mut mem(), frame.at(), ctx, cmd);
    }
    /// `PRenderThread_runMeshCommand2(ctx, cmd)` (render thread).
    PRenderThread_runMeshCommand2 @ 0x0061_4610 => extern "C" fn run_mesh_command2(ctx: u32, cmd: u32) {
        let mut frame = StackFrame::<2>::new();
        thread::bridge::run_mesh_command2(&mut mem(), frame.at(), ctx, cmd);
    }
    /// `PShaderPassStateD3D11_bind(state, ctx)` -> 0 (render thread): the
    /// `ID3D11DeviceContext` calls stay calls through its vtable.
    PShaderPassStateD3D11_bind @ 0x0058_c6c0 => extern "thiscall" fn bind_pass_state(state: u32, ctx: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        thread::bridge::bind_pass_state(&mut mem(), frame.at(), state, ctx)
    }
    /// `PEffect_findTechniqueByPass(effect, pass)` -> the technique or 0.
    PEffect_findTechniqueByPass_004e71d0 @ 0x004e_71d0 => extern "thiscall" fn find_technique(effect: u32, pass: u32) -> u32 {
        mesh::bridge::find_technique(&mut mem(), effect, pass)
    }
    /// `PRendererBase::selectTechniquePass(renderer, out, a, b, effect,
    /// material, pass, node, segment)` -> 0 or 7.
    Phyre_PRendering_PRendererBase_selectTechniquePass @ 0x005b_a690 => extern "thiscall" fn select_technique_pass(renderer: u32, out: u32, a: u32, b: u32, effect: u32, material: u32, pass: u32, node: u32, segment: u32) -> u32 {
        mesh::bridge::select_technique_pass(&mut mem(), renderer, out, a, b, effect, material, pass, node, segment)
    }
    /// `PRendererBase::allocateAndFillParameterBuffers(renderer, out,
    /// flags, material, node, a, b, c, dynamic)` -> 0 or an error.
    Phyre_PRendering_PRendererBase_allocateAndFillParameterBuffers @ 0x005b_5a50 => extern "thiscall" fn allocate_and_fill(renderer: u32, out: u32, flags: u32, material: u32, node: u32, a: u32, b: u32, c: u32, dynamic: u32) -> u32 {
        mesh::bridge::allocate_and_fill(&mut mem(), renderer, out, flags, material, node, a, b, c, dynamic)
    }
    /// `PApplication::renderWorld(app, world, camera, pass)`.
    Phyre_PFramework_PApplication_renderWorld @ 0x0062_be70 => extern "thiscall" fn render_world(app: u32, world_: u32, camera: u32, pass: u32) -> u32 {
        world::bridge::render_world(&mut mem(), app, world_, camera, pass)
    }
}

/// The name `FFX_HOOK_DISABLE` takes to turn [`culled_sorted`] off (as
/// does `replacements`).
pub(crate) const CULLED_SORTED_NAME: &str =
    "Phyre_PRendering_PRenderer_renderWorldCulledSorted_port";

/// Whether `renderWorldCulledSorted` runs the port. Its address already
/// holds a wrapper (`world::render_world`, the capture's scene inputs),
/// which calls [`culled_sorted`] in place of its trampoline when this is
/// on: not a `ported!` entry, since two hooks cannot share an address.
pub(crate) fn culled_sorted_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        let off = std::env::var("FFX_HOOK_DISABLE").unwrap_or_default();
        let only =
            crate::hook_only().is_none_or(|names| names.iter().any(|n| n == CULLED_SORTED_NAME));
        only && !off
            .split(',')
            .map(str::trim)
            .any(|n| n == "replacements" || n == CULLED_SORTED_NAME)
    })
}

/// `PRenderer::renderWorldCulledSorted(renderer, world, camera, ctx,
/// pass)` -> 0, ported (`ffx_prender::world`); its frame on this stack.
pub(crate) fn culled_sorted(renderer: u32, world_: u32, camera: u32, ctx: u32, pass: u32) -> u32 {
    let mut frame = StackFrame::<CULLED_WORDS>::new();
    world::bridge::culled_sorted(&mut mem(), frame.at(), renderer, world_, camera, ctx, pass)
}
