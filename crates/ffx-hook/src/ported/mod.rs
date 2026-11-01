//! Functions rewritten in Rust behind `ffx-bridge` (`docs/rewrite-plan.md`):
//! each line names the original, its address and signature, and the Rust
//! that replaces it. The logic lives in the module crates (portable, checked
//! by `ffx-oracle` against the original on synthetic inputs and snapshots);
//! a shim here only adapts the calling convention and hands over the game's
//! memory as [`RawMem`].
//!
//! A replacement jumps from the original's first five bytes, so it needs a
//! function of at least five bytes; the build check (`check_build`) already
//! pins every address to this exact `FFX.exe`.

//!
//! One file per module crate (`battle`, `kernel`, ...), each with its own
//! `ported!` list, so work on different modules does not touch the same
//! file.
//!
//! The same files build `ffx-host`'s function table (docs/host.md, "One
//! source of truth"): `ffx-host-ports` compiles this module a second time
//! with `cfg(ffx_host)`, where [`ported!`] makes table entries instead of
//! thunks, [`mem`] is the arena and [`StackFrame`] lives on the game
//! thread's stack in the arena. Every call into original code goes through
//! `ffx_bridge::route` (the helpers below), which calls natively in the
//! hooked game and through the function table in `ffx-host`.

use ffx_bridge::RawMem;

use crate::Hook;
use ffx_bridge::route;

mod auto;
mod battle;
mod c2_new;
mod chara;
mod h1_render_a;
mod h2_render_b;
mod h3_glue_render;
mod h3b_glue_render;
mod h3c_glue_render;
mod h3d_glue_render;
mod h3e_glue_render;
mod h3f_glue_render;
mod h4_glue_other;
mod h4b_glue_other;
mod kernel;
mod m4_rest;
mod m5_battle;
mod m5_main;
mod m5_main3;
mod m5_main4;
mod m5_map;
mod m5_menu;
mod m5_movie;
mod m5_pc_ui;
mod m5_ps2_emu;
mod m5_render;
mod m5_script;
mod m5_sound;
mod m5_static_init_a;
mod m5_static_init_b;
mod m5_static_init_c;
mod m5_static_init_d;
mod m5_system_a;
mod m5_system_a_df;
mod m5_system_a_stubs;
mod m5_system_b;
mod m5_vfx;
mod m5p_anim;
mod m5p_anim_left;
mod m5p_core;
mod m5p_core_b;
mod m5p_core_c;
mod m5p_core_d;
mod m5p_core_e;
mod m5p_core_f;
mod m5p_core_g;
mod m5p_framework_input;
mod m5p_framework_input_bp2;
mod m5p_framework_input_df;
mod m5p_framework_input_left2;
mod m5p_physics;
mod m5p_physics_stubs;
mod m5p_registry_b;
mod m5p_registry_b_bp2;
mod m5p_registry_load_a;
mod m5p_registry_load_a_bp3;
mod m5p_registry_load_a_left;
mod m5p_render_a;
mod m5p_render_b;
mod m5p_render_b_bp2;
mod m5p_render_b_bp3;
mod m5p_render_b_df;
mod m5p_render_b_left2;
mod m5p_scene;
mod m5p_scene_b;
mod m5p_scene_bp2;
mod m5p_scene_bp3;
mod m5p_scene_c;
mod m5p_scene_d;
mod m5p_scene_df;
mod m5p_scene_e;
mod m5p_scene_f;
mod m5p_scene_g;
mod m5p_scene_h;
mod m5p_scene_i;
mod m5p_scene_j;
mod m5p_scene_k;
mod m5p_scene_l;
mod m5p_scene_m;
mod m5p_scene_n;
mod m5p_scripting;
mod m5p_scripting_bp2;
mod m5p_scripting_bp3;
mod m5p_scripting_df;
mod m5p_scripting_left2;
mod m5p_stubs;
mod m5p_tmpl_abstract_ab;
mod m5p_tmpl_abstract_cd;
mod m5p_tmpl_concrete;
mod m5p_tmpl_fortype;
mod m5p_tmpl_members_dynamic;
mod m5p_tmpl_members_phyrearray;
mod main;
mod map;
mod menu;
mod movie;
mod p1_load;
mod p2_anim;
mod p2_anim_c;
mod p3_other_a;
mod p4_other_b;
mod p5_core;
mod p5_core_c;
mod p6_render_c;
mod p6_render_c2;
mod p6_render_c3;
mod p6_render_c4;
mod p6_render_c5;
mod p6_render_c6;
mod p6_render_c7;
mod p6_render_c8;
mod p6_render_c9;
mod pc_ui;
mod phyre_render;
mod ps2_emu;
mod render;
mod script;
mod sound;
mod system;
mod text;
mod vfx;
mod vfx_hi;
mod vfx_ppp;
mod vfx_shape;

#[cfg(not(ffx_host))]
pub(crate) use battle::ms_screen_process;
pub(crate) use m5_battle::{clock_seed as clock_seed_port, clock_seed_on};
pub(crate) use m5_main::{
    animate as ffx_application_animate, animate_on as ffx_application_animate_on,
};
#[cfg(not(ffx_host))]
pub(crate) use m5_system_a_df::{key_held, key_held_on};
pub(crate) use m5_system_b::{
    cd_file_size_cd_port, cd_file_size_pc_port, loader_thread_main, loader_thread_main_on,
};
#[cfg(not(ffx_host))]
pub(crate) use m5p_framework_input_df::{get_time, get_time_on, open_file, open_file_on};
#[cfg(not(ffx_host))]
pub(crate) use m5p_render_b_df::{
    FLIP_NAME, PRESENT_FLIP_NAME, flip, flip_on, present_flip, present_flip_on,
};
#[cfg(not(ffx_host))]
pub(crate) use map::rcbg_guide_update_ex;
#[cfg(not(ffx_host))]
pub(crate) use phyre_render::{culled_sorted, culled_sorted_on};

/// Every replacement, for the hook table.
#[must_use]
// one line per module: it grows with every merged port list
#[allow(clippy::too_many_lines)]
pub fn hooks() -> Vec<Hook> {
    let mut all = Vec::new();
    all.extend(auto::hooks());
    all.extend(battle::hooks());
    all.extend(c2_new::hooks());
    all.extend(chara::hooks());
    all.extend(h2_render_b::hooks());
    all.extend(h1_render_a::hooks());
    all.extend(h4_glue_other::hooks());
    all.extend(h4b_glue_other::hooks());
    all.extend(kernel::hooks());
    all.extend(m4_rest::hooks());
    all.extend(m5_battle::hooks());
    all.extend(m5_map::hooks());
    all.extend(m5_main::hooks());
    all.extend(m5_main3::hooks());
    all.extend(m5_main4::hooks());
    all.extend(m5_pc_ui::hooks());
    all.extend(m5_menu::hooks());
    all.extend(m5_render::hooks());
    all.extend(m5_script::hooks());
    all.extend(m5_static_init_a::hooks());
    all.extend(m5_static_init_b::hooks());
    all.extend(m5_static_init_c::hooks());
    all.extend(m5_static_init_d::hooks());
    all.extend(m5p_tmpl_fortype::hooks());
    all.extend(m5p_registry_b::hooks());
    all.extend(m5p_registry_b_bp2::hooks());
    all.extend(m5_system_a::hooks());
    all.extend(m5_system_a_df::hooks());
    all.extend(m5_system_a_stubs::hooks());
    all.extend(m5_system_b::hooks());
    all.extend(m5_vfx::hooks());
    all.extend(m5p_anim::hooks());
    all.extend(m5p_framework_input::hooks());
    all.extend(m5p_framework_input_bp2::hooks());
    all.extend(m5p_framework_input_df::hooks());
    all.extend(m5p_framework_input_left2::hooks());
    all.extend(m5p_anim_left::hooks());
    all.extend(m5p_registry_load_a::hooks());
    all.extend(m5p_registry_load_a_bp3::hooks());
    all.extend(m5p_registry_load_a_left::hooks());
    all.extend(m5p_core::hooks());
    all.extend(m5p_core_b::hooks());
    all.extend(m5p_core_c::hooks());
    all.extend(m5p_core_d::hooks());
    all.extend(m5p_core_e::hooks());
    all.extend(m5p_core_f::hooks());
    all.extend(m5p_core_g::hooks());
    all.extend(m5p_render_b::hooks());
    all.extend(m5p_render_b_bp2::hooks());
    all.extend(m5p_render_b_bp3::hooks());
    all.extend(m5p_render_b_df::hooks());
    all.extend(m5p_render_b_left2::hooks());
    all.extend(m5_sound::hooks());
    all.extend(m5_movie::hooks());
    all.extend(m5_ps2_emu::hooks());
    all.extend(m5p_tmpl_abstract_ab::hooks());
    all.extend(m5p_tmpl_abstract_cd::hooks());
    all.extend(m5p_tmpl_concrete::hooks());
    all.extend(m5p_tmpl_members_dynamic::hooks());
    all.extend(m5p_tmpl_members_phyrearray::hooks());
    all.extend(m5p_render_a::hooks());
    all.extend(m5p_stubs::hooks());
    all.extend(m5p_physics::hooks());
    all.extend(m5p_physics_stubs::hooks());
    all.extend(m5p_scene::hooks());
    all.extend(m5p_scene_df::hooks());
    all.extend(m5p_scene_b::hooks());
    all.extend(m5p_scene_bp2::hooks());
    all.extend(m5p_scene_bp3::hooks());
    all.extend(m5p_scene_c::hooks());
    all.extend(m5p_scene_d::hooks());
    all.extend(m5p_scene_e::hooks());
    all.extend(m5p_scene_f::hooks());
    all.extend(m5p_scene_g::hooks());
    all.extend(m5p_scene_h::hooks());
    all.extend(m5p_scene_i::hooks());
    all.extend(m5p_scene_j::hooks());
    all.extend(m5p_scene_k::hooks());
    all.extend(m5p_scene_l::hooks());
    all.extend(m5p_scene_m::hooks());
    all.extend(m5p_scene_n::hooks());
    all.extend(m5p_scripting::hooks());
    all.extend(m5p_scripting_bp2::hooks());
    all.extend(m5p_scripting_bp3::hooks());
    all.extend(m5p_scripting_df::hooks());
    all.extend(m5p_scripting_left2::hooks());
    all.extend(main::hooks());
    all.extend(map::hooks());
    all.extend(menu::hooks());
    all.extend(movie::hooks());
    all.extend(p1_load::hooks());
    all.extend(p3_other_a::hooks());
    all.extend(p4_other_b::hooks());
    all.extend(p5_core::hooks());
    all.extend(p5_core_c::hooks());
    all.extend(pc_ui::hooks());
    all.extend(h3_glue_render::hooks());
    all.extend(h3b_glue_render::hooks());
    all.extend(h3c_glue_render::hooks());
    all.extend(h3d_glue_render::hooks());
    all.extend(h3f_glue_render::hooks());
    all.extend(h3e_glue_render::hooks());
    all.extend(p2_anim::hooks());
    all.extend(p2_anim_c::hooks());
    all.extend(phyre_render::hooks());
    all.extend(p6_render_c::hooks());
    all.extend(p6_render_c2::hooks());
    all.extend(p6_render_c3::hooks());
    all.extend(p6_render_c4::hooks());
    all.extend(p6_render_c5::hooks());
    all.extend(p6_render_c6::hooks());
    all.extend(p6_render_c7::hooks());
    all.extend(p6_render_c8::hooks());
    all.extend(p6_render_c9::hooks());
    all.extend(ps2_emu::hooks());
    all.extend(render::hooks());
    all.extend(script::hooks());
    all.extend(sound::hooks());
    all.extend(system::hooks());
    all.extend(text::hooks());
    all.extend(vfx::hooks());
    all.extend(vfx_hi::hooks());
    all.extend(vfx_ppp::hooks());
    all.extend(vfx_shape::hooks());
    all
}

/// `W` zeroed words on this stack, for a port whose original keeps locals
/// in its frame and passes their addresses ([`Self::at`]) to its bridge or
/// callees. It lives until the end of the statement or block that holds it,
/// like the original's frame.
///
/// Scratch memory for a shim must be per call (this) or per thread
/// (`thread_local!`), never a process-wide `static`: the game runs ports on
/// several threads at once (job-scheduler workers, docs/hooking.md,
/// "Threads"), and a shared buffer gets overwritten mid-call
/// (`tools/check-ported.py` rejects one).
#[cfg(not(ffx_host))]
pub(crate) struct StackFrame<const W: usize>([u32; W]);

#[cfg(not(ffx_host))]
impl<const W: usize> StackFrame<W> {
    pub(crate) const fn new() -> Self {
        Self([0; W])
    }

    /// The frame's address (32-bit process).
    pub(crate) fn at(&mut self) -> u32 {
        u32::try_from(self.0.as_mut_ptr() as usize).unwrap_or(0)
    }
}

#[cfg(not(ffx_host))]
impl<const W: usize> Drop for StackFrame<W> {
    fn drop(&mut self) {
        // the bridge and the originals wrote it through its address
        std::hint::black_box(&mut self.0);
    }
}

/// In `ffx-host`: `W` zeroed words on the game thread's stack in the arena
/// (the router's [`route::Router::frame_alloc`]), released when dropped,
/// so the originals in Unicorn can reach them.
#[cfg(ffx_host)]
pub(crate) struct StackFrame<const W: usize>(u32);

#[cfg(ffx_host)]
impl<const W: usize> StackFrame<W> {
    pub(crate) fn new() -> Self {
        #[allow(clippy::cast_possible_truncation)] // frames are small
        let bytes = (4 * W) as u32;
        Self(route::with(|r| r.frame_alloc(bytes)).unwrap_or(0))
    }

    /// The frame's game address.
    pub(crate) fn at(&mut self) -> u32 {
        self.0
    }
}

#[cfg(ffx_host)]
impl<const W: usize> Drop for StackFrame<W> {
    fn drop(&mut self) {
        let at = self.0;
        let _ = route::with(|r| r.frame_free(at));
    }
}

/// A frame sized at run time, for an original that carves its own with
/// `_alloca_probe_16` (or `_alloca_probe_8`, `_alloca_probe`): `f` gets the
/// address of at least `bytes` zeroed bytes, 16-byte aligned as
/// `_alloca_probe_16` leaves them, which live until `f` returns, like the
/// original's buffer until its `ret` (docs/porting.md, "Run-time frames
/// and varargs"). Per call, so threads and nested calls each get their
/// own: an inner `with_frame` lies below the outer one, as a second alloca
/// lies below the first.
///
/// In the hook the bytes come from this thread's stack: the smallest of a
/// few fixed sizes that holds them (up to 16 KB, the scripting and
/// file-name buffers are a few dozen bytes), in its own call so only that
/// one is reserved; beyond that from the heap (the original would need as
/// much stack, which the 32 KB threads don't have).
#[cfg(not(ffx_host))]
pub(crate) fn with_frame<R>(bytes: u32, f: impl FnOnce(u32) -> R) -> R {
    // three spare words to align the start to 16
    let words = usize::try_from(bytes)
        .unwrap_or(usize::MAX)
        .div_ceil(4)
        .saturating_add(3);
    match words {
        0..=16 => in_frame::<16, R>(f),
        17..=64 => in_frame::<64, R>(f),
        65..=256 => in_frame::<256, R>(f),
        257..=1024 => in_frame::<1024, R>(f),
        1025..=4096 => in_frame::<4096, R>(f),
        _ => {
            let mut heap = Vec::new();
            if heap.try_reserve_exact(words).is_err() {
                return f(0);
            }
            heap.resize(words, 0u32);
            let at = u32::try_from(heap.as_mut_ptr() as usize).unwrap_or(0);
            let r = f(at.wrapping_add(15) & !15);
            std::hint::black_box(&mut heap);
            r
        }
    }
}

/// [`with_frame`]'s fixed sizes: `W` words in this call's own frame.
#[cfg(not(ffx_host))]
#[inline(never)]
fn in_frame<const W: usize, R>(f: impl FnOnce(u32) -> R) -> R {
    let mut frame = StackFrame::<W>::new();
    let at = frame.at();
    f(at.wrapping_add(15) & !15)
}

/// In `ffx-host`: the router's frame on the game thread's stack in the
/// arena ([`route::Router::frame_alloc`]: zeroed, 16-byte aligned),
/// released when `f` returns.
#[cfg(ffx_host)]
pub(crate) fn with_frame<R>(bytes: u32, f: impl FnOnce(u32) -> R) -> R {
    let at = route::with(|r| r.frame_alloc(bytes)).unwrap_or(0);
    let r = f(at);
    let _ = route::with(|r| r.frame_free(at));
    r
}

/// Whether the port of a wrapper-owned address runs (docs/hooking.md, "The
/// hook table"): a hand-written wrapper (harness, mods, captures) holds the
/// address and calls the port where it called its trampoline, unless
/// `FFX_HOOK_DISABLE` names `name` or `replacements`, or `FFX_HOOK_ONLY`
/// leaves `name` out. Each port caches the answer.
#[cfg(not(ffx_host))]
pub(crate) fn wrapper_port_on(name: &str) -> bool {
    let off = std::env::var("FFX_HOOK_DISABLE").unwrap_or_default();
    let only = crate::hook_only().is_none_or(|names| names.iter().any(|n| n == name));
    only && !off
        .split(',')
        .map(str::trim)
        .any(|n| n == "replacements" || n == name)
}

/// The game's memory, for the bridges.
#[cfg(not(ffx_host))]
fn mem() -> RawMem {
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    RawMem {
        image: crate::GAME_BASE.load(std::sync::atomic::Ordering::Acquire) as u32,
    }
}

/// The game's memory, for the bridges: the arena (`FFX.exe` at its
/// preferred base).
#[cfg(ffx_host)]
fn mem() -> RawMem {
    RawMem {
        image: ffx_bridge::IMAGE_BASE,
    }
}

/// The address an import slot of `FFX.exe` holds (the slot's `ffx.sym`
/// address): the DLL function the loader bound, or `ffx-host`'s import
/// stub. Call it through `ffx_bridge::route` with the import's convention.
#[allow(clippy::cast_possible_truncation)] // a game address
fn import_slot(sym: u32) -> u32 {
    ffx_bridge::Mem::read(&mem(), crate::rebase(sym) as u32)
}

/// Calls an original function by its `ffx.sym` address (cdecl, no
/// arguments), for bridges that still need one.
#[allow(clippy::cast_possible_truncation)] // a game address
fn call0(sym: u32) -> u32 {
    // a cdecl function of this build taking nothing
    route::cdecl(crate::rebase(sym) as u32, &[])
}

/// The scratch registers a shim's thunk saves for the caller, by the
/// shim's return type: what it returns in stays the shim's, the rest keeps
/// the caller's value.
///
/// The originals were compiled with whole-program optimisation, which lets
/// a caller keep `eax`, `ecx` or `edx` live across a call to a function it
/// knows leaves them alone (`tools/check-clobber.py`: `op_spline_xyz_table`
/// returns the `eax` of a call before `op_spr_free`, which only decrements
/// a byte). A Rust function clobbers all three, as the ABI allows, so every
/// replacement goes through a thunk that keeps them.
#[cfg(not(ffx_host))]
pub(crate) trait Saved {
    /// Whether `eax` is saved (nothing returned in it).
    const EAX: bool;
    /// Whether `edx` is saved (no 64-bit result).
    const EDX: bool;
}

#[cfg(not(ffx_host))]
macro_rules! saved {
    ($($t:ty => $eax:literal, $edx:literal;)*) => {
        $(impl Saved for $t {
            const EAX: bool = $eax;
            const EDX: bool = $edx;
        })*
    };
}
#[cfg(not(ffx_host))]
saved! {
    () => true, true;
    f32 => true, true;
    f64 => true, true;
    bool => false, true;
    u8 => false, true;
    i8 => false, true;
    u16 => false, true;
    i16 => false, true;
    u32 => false, true;
    i32 => false, true;
    u64 => false, false;
    i64 => false, false;
}

/// The pseudo-ABI of [`ported!`] for originals that take their first two
/// arguments in `ecx` and `edx` like fastcall, the rest on the stack, and
/// leave those stack words to the caller like cdecl (`ret`, the caller's
/// `add esp`). MSVC's link-time code generation makes this custom
/// convention for internal functions (`0x00401690` and
/// `Settings_ValidateBindings` under `Settings_SetValue`). Rust has no such
/// ABI: the shim is declared `extern "C"` with the two register arguments
/// first, and its thunk pushes `edx`, then `ecx`, below the copied stack
/// words.
pub(crate) const FASTCALL_CALLER_POPS: &str = "fastcall-caller-pops";

/// Whether `abi` is [`FASTCALL_CALLER_POPS`].
const fn is_fastcall_caller_pops(abi: &str) -> bool {
    let (a, b) = (abi.as_bytes(), FASTCALL_CALLER_POPS.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The stack words a shim's caller pushed: every argument for cdecl and
/// stdcall, all but `this` for thiscall (`ecx`), all but the first two for
/// fastcall and [`FASTCALL_CALLER_POPS`] (`ecx`, `edx`).
pub(crate) const fn stack_words(abi: &str, sizes: &[usize]) -> usize {
    let skip = match abi.as_bytes() {
        b"thiscall" => 1,
        b"fastcall" => 2,
        _ if is_fastcall_caller_pops(abi) => 2,
        _ => 0,
    };
    let mut words = 0;
    let mut i = skip;
    while i < sizes.len() {
        words += sizes[i].div_ceil(4);
        i += 1;
    }
    words
}

/// Whether a shim may take the caller's argument slots (`ported!`): the
/// extra last argument must be a stack word. thiscall qualifies (`this`
/// stays in `ecx`, which the thunk saves and never changes; the slots word
/// follows the stack arguments, and the shim pops it with them); fastcall
/// does not.
pub(crate) const fn slots_abi(abi: &str) -> bool {
    matches!(abi.as_bytes(), b"C" | b"cdecl" | b"stdcall" | b"thiscall")
}

/// Whether the callee pops its stack arguments (everything but cdecl and
/// [`FASTCALL_CALLER_POPS`]).
pub(crate) const fn callee_pops(abi: &str) -> bool {
    !matches!(abi.as_bytes(), b"C" | b"cdecl") && !is_fastcall_caller_pops(abi)
}

// The conventions' arithmetic, checked at compile time (the thunks run only
// in the 32-bit game, so this is their host-side test).
const _: () = {
    assert!(is_fastcall_caller_pops("fastcall-caller-pops"));
    assert!(!is_fastcall_caller_pops("fastcall"));
    assert!(!is_fastcall_caller_pops("fastcall-caller-popz"));
    assert!(!callee_pops(FASTCALL_CALLER_POPS));
    assert!(callee_pops("fastcall") && callee_pops("thiscall") && callee_pops("stdcall"));
    assert!(!callee_pops("C") && !callee_pops("cdecl"));
    assert!(!slots_abi(FASTCALL_CALLER_POPS));
    // (ecx, edx; one stack word): Settings_ValidateBindings
    assert!(stack_words(FASTCALL_CALLER_POPS, &[4, 4, 4]) == 1);
    assert!(stack_words(FASTCALL_CALLER_POPS, &[4, 4]) == 0);
    assert!(stack_words(FASTCALL_CALLER_POPS, &[4, 4, 8, 4]) == 3);
    assert!(stack_words("fastcall", &[4, 4, 4]) == 1);
    assert!(stack_words("thiscall", &[4, 4, 4]) == 2);
    assert!(stack_words("C", &[4, 4, 4]) == 3);
};

/// Calls an original cdecl function by absolute address with 0 to 12 word
/// arguments, returning `eax`. cdecl leaves the stack to the caller, so
/// the call is the same for any count; more than 8 is a bug in the bridge
/// that asked.
pub(crate) fn call_cdecl(f: u32, a: &[u32]) -> u32 {
    cdecl12("call_cdecl", f, a)
}

/// [`call_cdecl`] for an original that returns its result in `ST0`
/// (`float` or `double` in C, often `void` in Ghidra: look for
/// `(float10)Name(` at the callers): the value at the x87's precision
/// (53 bits under the game's control word), which the declared `f64`
/// return pops off the x87 stack. Only for a callee that always leaves
/// `ST0` set: popping an empty x87 stack gives a NaN and unbalances it.
pub(crate) fn call_cdecl_f64(f: u32, a: &[u32]) -> f64 {
    cdecl12("call_cdecl_f64", f, a)
}

/// [`call_cdecl_f64`] rounded to `f32` (`fstp dword`), for an original
/// whose callers store its `ST0` as a `float` (`MsCameraGetYaw`,
/// `AtelPopStackFloat`).
pub(crate) fn call_cdecl_f32(f: u32, a: &[u32]) -> f32 {
    cdecl12("call_cdecl_f32", f, a)
}

/// [`call_cdecl`] for an original that returns 64 bits in `edx:eax`
/// (`graphicGetTimeInMicroseconds`).
pub(crate) fn call_cdecl_u64(f: u32, a: &[u32]) -> u64 {
    cdecl12("call_cdecl_u64", f, a)
}

/// The call behind [`call_cdecl`] and its float forms: 12 words, the
/// missing ones zero, `R` from `eax` (`u32`) or `ST0` (`f32`, `f64`).
fn cdecl12<R: route::Out>(helper: &str, f: u32, a: &[u32]) -> R {
    // No panic here: a panic in a function the game calls ends the process.
    if a.len() > 12 {
        crate::log::log(&format!(
            "{helper}: {} arguments for {f:#010x}, only 12 passed",
            a.len()
        ));
    }
    let mut w = [0u32; 12];
    let n = a.len().min(12);
    w[..n].copy_from_slice(&a[..n]);
    // a cdecl function of at most 12 word arguments returning `R` where
    // the ABI puts it (`eax`, or `ST0` for a float); the extra zero words
    // it doesn't read sit on the caller's side of the stack
    route::cdecl(f, &w)
}

/// The most stack words [`call_cdecl_words`] passes.
pub(crate) const WIDE_WORDS: usize = 32;

/// Calls an original cdecl function by absolute address with up to
/// [`WIDE_WORDS`] stack words, returning `eax`: for callees that take a
/// struct by value (`yonExtMapRegister(id, 0x4c-byte struct)` from
/// `opu_unit_etc`, 20 words), which [`call_cdecl`]'s 12 can't hold. `a` is
/// the stack as the original pushes it, lowest address first: a by-value
/// struct is its words in memory order, rounded up to whole words. cdecl
/// leaves the stack to the caller, so the zero words past `a` are only
/// unread stack. More than [`WIDE_WORDS`] would cut a struct short: that is
/// logged and the original is not called (0).
pub(crate) fn call_cdecl_words(f: u32, a: &[u32]) -> u32 {
    // No panic here: a panic in a function the game calls ends the process.
    if a.len() > WIDE_WORDS {
        crate::log::log(&format!(
            "call_cdecl_words: {} words for {f:#010x}, at most {WIDE_WORDS}; not called",
            a.len()
        ));
        return 0;
    }
    let mut w = [0u32; WIDE_WORDS];
    w[..a.len()].copy_from_slice(a);
    // a cdecl function of this build taking at most these stack words and
    // returning in `eax`; the zero words it doesn't read sit on the
    // caller's side of the stack, which the caller pops
    route::cdecl(f, &w)
}

/// Calls an original thiscall (or fastcall taking only `ecx`) function by
/// absolute address: `this` in `ecx`, 0 to 8 word arguments on the
/// stack, returning `eax`. The callee pops its stack arguments, so `a`
/// must hold exactly as many words as it takes; more than 8 is a bridge
/// bug, logged, and the original is not called (0).
pub(crate) fn call_thiscall(f: u32, this: u32, a: &[u32]) -> u32 {
    if a.len() > 8 {
        // No panic here: a panic in a function the game calls ends the
        // process. A wrong count would unbalance the stack: don't call.
        crate::log::log(&format!(
            "call_thiscall: {} arguments for {f:#010x}, at most 8; not called",
            a.len()
        ));
        return 0;
    }
    // a thiscall function of this build that takes these stack words (the
    // bridge's count) and pops them itself
    route::thiscall(f, this, a)
}

/// `ported! { /// doc  Name @ 0xADDR => extern "C" fn shim(args) -> ret { body } ... }`:
/// the shims, a register-saving thunk for each ([`Saved`]), and the
/// module's `hooks()`.
///
/// `fn shim(args; slots: u32)` also hands the shim the address of the
/// caller's stack arguments (`slots`, not an argument of the original): the
/// thunk passes the shim a copy of the arguments, so an original that
/// stores into its own argument slots (MSVC reuses them as scratch, and
/// `op_random` leaves its result there) needs its shim to write them
/// through `slots` for callers that read the word after the call
/// (`tools/check-argslots.py`). cdecl, stdcall and thiscall (there `slots`
/// is the address of the first stack argument, the word after `this`).
///
/// `extern "fastcall-caller-pops" fn shim(ecx_arg, edx_arg, stack..)`
/// ([`FASTCALL_CALLER_POPS`]): the original's custom convention, `ecx` and
/// `edx` plus stack words the caller pops; the shim itself becomes
/// `extern "C"` with the register arguments first. No `slots` there.
#[cfg(not(ffx_host))]
macro_rules! ported {
    ($( $(#[$doc:meta])* $name:ident @ $addr:literal => extern $abi:tt fn $shim:ident($($arg:ident: $ty:ty),* $(; $slots:ident: $sty:ty)?) $(-> $ret:ty)? $body:block )*) => {
        $(
            $crate::ported::shim_fn! {
                $abi, [$(#[$doc])*], $shim, ($($arg: $ty,)* $($slots: $sty)?), ($($ret)?), $body
            }
        )*

        /// This module's replacements.
        #[must_use]
        pub fn hooks() -> Vec<crate::Hook> {
            vec![$(crate::Hook {
                name: stringify!($name),
                address: $addr,
                prologue: &[],
                replacement: $crate::ported::thunk!($abi, $shim, ($($ty),*), ($($ret)?) $(, $slots)?),
                original: None,
                steal: 0,
            }),*]
        }
    };
}
#[cfg(not(ffx_host))]
use ported;

/// One shim of [`ported!`] in the ABI its thunk calls: the declared one, or
/// `extern "C"` for [`FASTCALL_CALLER_POPS`].
#[cfg(not(ffx_host))]
macro_rules! shim_fn {
    ("fastcall-caller-pops", [$($doc:tt)*], $shim:ident, ($($p:tt)*), ($($ret:ty)?), $body:block) => {
        $($doc)*
        unsafe extern "C" fn $shim($($p)*) $(-> $ret)? $body
    };
    ($abi:tt, [$($doc:tt)*], $shim:ident, ($($p:tt)*), ($($ret:ty)?), $body:block) => {
        $($doc)*
        unsafe extern $abi fn $shim($($p)*) $(-> $ret)? $body
    };
}
#[cfg(not(ffx_host))]
pub(crate) use shim_fn;

/// The address of `$shim`'s thunk: saves the scratch registers its return
/// type leaves alone, copies the caller's stack arguments, calls the shim
/// and returns as the original did (`ret n` for the callee-pops ABIs).
#[cfg(all(target_arch = "x86", not(ffx_host)))]
macro_rules! thunk {
    // `ecx`, `edx` and stack words the caller pops (FASTCALL_CALLER_POPS):
    // the cdecl shim gets the stack copy with `ecx` and `edx` pushed below
    // it, and the thunk returns with a plain `ret`
    ("fastcall-caller-pops", $shim:ident, ($($ty:ty),*), ($($ret:ty)?)) => {{
        type Ret = $crate::ported::thunk!(@ret $($ret)?);
        const WORDS: usize = $crate::ported::stack_words(
            $crate::ported::FASTCALL_CALLER_POPS,
            &[$(::core::mem::size_of::<$ty>()),*],
        );
        const EAX: usize = <Ret as $crate::ported::Saved>::EAX as usize;
        const EDX: usize = <Ret as $crate::ported::Saved>::EDX as usize;
        const SAVED: usize = 1 + EAX + EDX;
        const _: () = {
            let sizes = [$(::core::mem::size_of::<$ty>()),*];
            let mut i = 0;
            while i < sizes.len() && i < 2 {
                assert!(sizes[i] == 4, "the ecx and edx arguments are words");
                i += 1;
            }
        };
        #[unsafe(naked)]
        unsafe extern "C" fn thunk() {
            ::core::arch::naked_asm!(
                ".if {eax}", "push eax", ".endif",
                "push ecx",
                ".if {edx}", "push edx", ".endif",
                ".rept {words}", "push dword ptr [esp + {from}]", ".endr",
                "push edx",
                "push ecx",
                "call {shim}",
                "add esp, {bytes} + 8",
                ".if {edx}", "pop edx", ".endif",
                "pop ecx",
                ".if {eax}", "pop eax", ".endif",
                "ret",
                eax = const EAX,
                edx = const EDX,
                words = const WORDS,
                from = const 4 * (SAVED + WORDS),
                bytes = const 4 * WORDS,
                shim = sym $shim,
            )
        }
        thunk as *const () as usize
    }};
    ("fastcall-caller-pops", $shim:ident, ($($ty:ty),*), ($($ret:ty)?), $slots:ident) => {{
        compile_error!("argument slots only for cdecl, stdcall and thiscall shims")
    }};
    ($abi:literal, $shim:ident, ($($ty:ty),*), ($($ret:ty)?)) => {{
        type Ret = $crate::ported::thunk!(@ret $($ret)?);
        const WORDS: usize = $crate::ported::stack_words($abi, &[$(::core::mem::size_of::<$ty>()),*]);
        const EAX: usize = <Ret as $crate::ported::Saved>::EAX as usize;
        const EDX: usize = <Ret as $crate::ported::Saved>::EDX as usize;
        const SAVED: usize = 1 + EAX + EDX;
        const POPS: usize = $crate::ported::callee_pops($abi) as usize;
        #[unsafe(naked)]
        unsafe extern "C" fn thunk() {
            ::core::arch::naked_asm!(
                ".if {eax}", "push eax", ".endif",
                "push ecx",
                ".if {edx}", "push edx", ".endif",
                ".rept {words}", "push dword ptr [esp + {from}]", ".endr",
                "call {shim}",
                ".if {pops} == 0", ".if {words}", "add esp, {bytes}", ".endif", ".endif",
                ".if {edx}", "pop edx", ".endif",
                "pop ecx",
                ".if {eax}", "pop eax", ".endif",
                ".if {pops}", "ret {bytes}", ".else", "ret", ".endif",
                eax = const EAX,
                edx = const EDX,
                words = const WORDS,
                from = const 4 * (SAVED + WORDS),
                bytes = const 4 * WORDS,
                pops = const POPS,
                shim = sym $shim,
            )
        }
        thunk as *const () as usize
    }};
    // the same, with the address of the caller's arguments as the shim's
    // last (extra) stack argument: pushed first, at `esp + 4` on entry
    ($abi:literal, $shim:ident, ($($ty:ty),*), ($($ret:ty)?), $slots:ident) => {{
        type Ret = $crate::ported::thunk!(@ret $($ret)?);
        const WORDS: usize = $crate::ported::stack_words($abi, &[$(::core::mem::size_of::<$ty>()),*]);
        const EAX: usize = <Ret as $crate::ported::Saved>::EAX as usize;
        const EDX: usize = <Ret as $crate::ported::Saved>::EDX as usize;
        const SAVED: usize = 1 + EAX + EDX;
        const POPS: usize = $crate::ported::callee_pops($abi) as usize;
        const _: () = assert!(
            $crate::ported::slots_abi($abi),
            "argument slots only for cdecl, stdcall and thiscall shims"
        );
        #[unsafe(naked)]
        unsafe extern "C" fn thunk() {
            ::core::arch::naked_asm!(
                ".if {eax}", "push eax", ".endif",
                "push ecx",
                ".if {edx}", "push edx", ".endif",
                // esp as it was before this push, moved up to the first argument
                "push esp",
                "add dword ptr [esp], {to_args}",
                ".rept {words}", "push dword ptr [esp + {from}]", ".endr",
                "call {shim}",
                ".if {pops} == 0", "add esp, {bytes} + 4", ".endif",
                ".if {edx}", "pop edx", ".endif",
                "pop ecx",
                ".if {eax}", "pop eax", ".endif",
                ".if {pops}", "ret {bytes}", ".else", "ret", ".endif",
                eax = const EAX,
                edx = const EDX,
                words = const WORDS,
                to_args = const 4 * SAVED + 4,
                from = const 4 * (SAVED + WORDS + 1),
                bytes = const 4 * WORDS,
                pops = const POPS,
                shim = sym $shim,
            )
        }
        thunk as *const () as usize
    }};
    (@ret) => { () };
    (@ret $ret:ty) => { $ret };
}
/// Off the game's architecture (host builds, clippy) the shim itself.
#[cfg(all(not(target_arch = "x86"), not(ffx_host)))]
macro_rules! thunk {
    ($abi:literal, $shim:ident, ($($ty:ty),*), ($($ret:ty)?) $(, $slots:ident)?) => {
        $shim as *const () as usize
    };
}
#[cfg(not(ffx_host))]
pub(crate) use thunk;

/// `ported!` in `ffx-host`: each shim as a plain Rust function (no ABI: the
/// executor decodes the original's arguments), and the module's `hooks()`
/// as function-table entries (`crate::Hook`: name, address, convention,
/// stack words and the adapter from the caller's words to the shim).
#[cfg(ffx_host)]
macro_rules! ported {
    ($( $(#[$doc:meta])* $name:ident @ $addr:literal => extern $abi:tt fn $shim:ident($($arg:ident: $ty:ty),* $(; $slots:ident: $sty:ty)?) $(-> $ret:ty)? $body:block )*) => {
        $(
            $(#[$doc])*
            unsafe fn $shim($($arg: $ty,)* $($slots: $sty)?) $(-> $ret)? $body
        )*

        /// This module's replacements, as function-table entries.
        #[must_use]
        pub fn hooks() -> Vec<crate::Hook> {
            vec![$({
                const CONV: ::ffx_bridge::route::Conv = $crate::ported::host::conv($abi);
                #[allow(clippy::let_unit_value, clippy::unit_arg)] // void shims
                fn run(a: &::ffx_bridge::route::Args) -> ::ffx_bridge::route::Ret {
                    let mut w = $crate::ported::host::Words::new(CONV, a);
                    let _ = &mut w;
                    // SAFETY: the shim's arguments as its original's caller
                    // passed them, decoded by the original's convention.
                    let r = unsafe {
                        $shim($(<$ty as $crate::ported::host::FromWords>::take(&mut w),)* $({ let _ = stringify!($slots); a.slots })?)
                    };
                    $crate::ported::host::IntoRet::into_ret(r)
                }
                crate::Hook {
                    name: stringify!($name),
                    address: $addr,
                    conv: CONV,
                    words: $crate::ported::stack_words($abi, &[$(::core::mem::size_of::<$ty>()),*]),
                    run,
                }
            }),*]
        }
    };
}
#[cfg(ffx_host)]
use ported;

/// [`ported!`] for originals under five bytes (`docs/porting.md`,
/// "Functions under five bytes"): the hook's 5-byte `jmp` doesn't fit, so
/// the 32-bit game gets no hook from these lines (`hooks()` is empty
/// there and the original bytes keep running; the shims are still
/// compiled and linted), while `ffx-host` makes the same table entries as
/// [`ported!`]. `tools/check-ported.py` accepts an address under five
/// bytes only here, and only such addresses.
#[cfg(not(ffx_host))]
macro_rules! host_only {
    ($( $(#[$doc:meta])* $name:ident @ $addr:literal => extern $abi:tt fn $shim:ident($($arg:ident: $ty:ty),* $(; $slots:ident: $sty:ty)?) $(-> $ret:ty)? $body:block )*) => {
        $(
            $crate::ported::shim_fn! {
                $abi, [#[allow(dead_code)] $(#[$doc])*], $shim, ($($arg: $ty,)* $($slots: $sty)?), ($($ret)?), $body
            }
        )*

        /// This module's replacements in the 32-bit game: none (every
        /// original here is too short for the hook's `jmp`).
        #[must_use]
        pub fn hooks() -> Vec<crate::Hook> {
            Vec::new()
        }
    };
}
#[cfg(ffx_host)]
macro_rules! host_only {
    ($($t:tt)*) => {
        $crate::ported::ported! { $($t)* }
    };
}
use host_only;

/// What `ffx-host`'s [`ported!`] expands to: the convention, the argument
/// decoding and the result.
#[cfg(ffx_host)]
pub(crate) mod host {
    use ffx_bridge::route::{Args, Conv, Ret};

    /// The convention of a `ported!` ABI string.
    pub(crate) const fn conv(abi: &str) -> Conv {
        match abi.as_bytes() {
            b"C" | b"cdecl" => Conv::Cdecl,
            b"stdcall" => Conv::Stdcall,
            b"thiscall" => Conv::Thiscall,
            b"fastcall" => Conv::Fastcall,
            _ if super::is_fastcall_caller_pops(abi) => Conv::FastcallCallerPops,
            // the hook's build refuses any other ABI string (rustc's
            // `extern`), so a line that builds there is one of the above
            _ => Conv::Cdecl,
        }
    }

    /// The caller's words in argument order: the register ones, then the
    /// stack.
    pub(crate) struct Words<'a> {
        args: &'a Args,
        regs: usize,
        next: usize,
    }

    impl<'a> Words<'a> {
        pub(crate) const fn new(conv: Conv, args: &'a Args) -> Self {
            Self {
                args,
                regs: conv.register_args(),
                next: 0,
            }
        }

        fn word(&mut self) -> u32 {
            let i = self.next;
            self.next += 1;
            match (self.regs, i) {
                (1.., 0) => self.args.ecx,
                (2, 1) => self.args.edx,
                _ => self.args.stack.get(i - self.regs).copied().unwrap_or(0),
            }
        }
    }

    /// An argument type of a shim: decoded from the next word or two.
    pub(crate) trait FromWords {
        fn take(w: &mut Words<'_>) -> Self;
    }

    macro_rules! from_word {
        ($($t:ty),*) => {$(
            impl FromWords for $t {
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // the low bits, as the shim's ABI reads them
                fn take(w: &mut Words<'_>) -> Self {
                    w.word() as $t
                }
            }
        )*};
    }
    from_word!(u32, i32, u16, i16, u8, i8);

    impl FromWords for bool {
        fn take(w: &mut Words<'_>) -> Self {
            w.word() & 0xff != 0
        }
    }

    impl FromWords for f32 {
        fn take(w: &mut Words<'_>) -> Self {
            f32::from_bits(w.word())
        }
    }

    impl FromWords for u64 {
        fn take(w: &mut Words<'_>) -> Self {
            let lo = w.word();
            u64::from(w.word()) << 32 | u64::from(lo)
        }
    }

    impl FromWords for i64 {
        fn take(w: &mut Words<'_>) -> Self {
            u64::take(w).cast_signed()
        }
    }

    impl FromWords for f64 {
        fn take(w: &mut Words<'_>) -> Self {
            f64::from_bits(u64::take(w))
        }
    }

    /// A shim's result: where its original's caller reads it.
    pub(crate) trait IntoRet {
        fn into_ret(self) -> Ret;
    }

    impl IntoRet for () {
        fn into_ret(self) -> Ret {
            Ret::VOID
        }
    }

    macro_rules! into_eax {
        ($($t:ty),*) => {$(
            impl IntoRet for $t {
                #[allow(clippy::cast_sign_loss, clippy::cast_lossless)] // the register's bits
                fn into_ret(self) -> Ret {
                    Ret::eax(self as u32)
                }
            }
        )*};
    }
    into_eax!(u32, u16, u8);

    macro_rules! into_eax_signed {
        ($($t:ty),*) => {$(
            impl IntoRet for $t {
                fn into_ret(self) -> Ret {
                    Ret::eax(i32::from(self).cast_unsigned())
                }
            }
        )*};
    }
    into_eax_signed!(i32, i16, i8);

    impl IntoRet for bool {
        fn into_ret(self) -> Ret {
            Ret::eax(u32::from(self))
        }
    }

    impl IntoRet for u64 {
        fn into_ret(self) -> Ret {
            Ret::u64(self)
        }
    }

    impl IntoRet for i64 {
        fn into_ret(self) -> Ret {
            Ret::u64(self.cast_unsigned())
        }
    }

    impl IntoRet for f32 {
        fn into_ret(self) -> Ret {
            Ret::st0(f64::from(self))
        }
    }

    impl IntoRet for f64 {
        fn into_ret(self) -> Ret {
            Ret::st0(self)
        }
    }
}
