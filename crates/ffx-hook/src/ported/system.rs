//! `ffx-system`: the PS2 CD layer, save data, singletons, configuration,
//! text codes, the `TO` window, pad, battle-window and scene accessors,
//! the archive digest map, the `cdc` matrix math.

use ffx_bridge::Mem;
use ffx_menu::scene::bridge as menu_scene;
use ffx_system::{
    accessor, btl, cd, config, container, daps, dtor, files, fixup, heap, instance, math, menu,
    msg, names, pad, pkt, render, save, scene, sort, stl, stub, sync, tolib, vbf, window,
};

use super::{call0, mem, ported};

/// `Phyre_PhyrePrintf(level, format)` (`0x004352f0`, cdecl, varargs), the
/// engine's printf, for bridges that print.
fn phyre_printf(level: u32, format: u32) {
    // SAFETY: the engine's cdecl printf, a format without arguments.
    unsafe {
        let f: unsafe extern "C" fn(u32, u32) = std::mem::transmute(crate::rebase(0x0043_52f0));
        f(level, format);
    }
}

/// Calls the cdecl function at `f` (an absolute address, as the game's
/// function tables hold them) with up to four arguments.
fn call_ptr<M>(_mem: &mut M, f: u32, args: &[u32]) -> u32 {
    super::call_cdecl(f, args)
}

/// Calls the original cdecl function at `sym` (an `ffx.sym` address).
fn call_sym<M>(mem: &mut M, sym: u32, args: &[u32]) -> u32 {
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    call_ptr(mem, crate::rebase(sym) as u32, args)
}

/// `Phyre::PSerialization::PStreamFile::read(buf, n)` (`0x00608090`,
/// thiscall), not ported yet.
fn stream_read<M>(_mem: &mut M, stream: u32, buf: u32, n: u32) -> u32 {
    // SAFETY: the game's thiscall stream read.
    unsafe {
        let f: unsafe extern "thiscall" fn(u32, u32, u32) -> u32 =
            std::mem::transmute(crate::rebase(0x0060_8090));
        f(stream, buf, n)
    }
}

/// `GetFileSizeEx(handle, &size)` through its import slot.
fn get_file_size_ex<M>(_mem: &mut M, handle: u32) -> Option<u64> {
    let mut size = 0u64;
    // SAFETY: GetFileSizeEx, stdcall, on the stream's handle.
    let ok = unsafe {
        let f: unsafe extern "stdcall" fn(u32, *mut u64) -> u32 =
            std::mem::transmute(GameOs::import(0x00b0_c13c));
        f(handle, &raw mut size)
    };
    (ok != 0).then_some(size)
}

/// `Phyre::PRedBlackTree::remove(node)` (`0x00575000`, thiscall, `ret 4`),
/// the engine's, not ported here.
fn red_black_remove<M>(_mem: &mut M, tree: u32, node: u32) {
    // SAFETY: the engine's thiscall tree removal.
    unsafe {
        let f: unsafe extern "thiscall" fn(u32, u32) =
            std::mem::transmute(crate::rebase(0x0057_5000));
        f(tree, node);
    }
}

/// A `Phyre::PClassDescriptorForType<T>` destructor: the vtable, then the
/// parent's destructor (`0x0043b3e0`, thiscall, not ported).
fn class_descriptor_dtor(this: u32, vtable: u32) -> u32 {
    dtor::bridge::chain(&mut mem(), this, vtable, |_, this| {
        // SAFETY: the engine's thiscall PClassDescriptor destructor.
        unsafe {
            let f: unsafe extern "thiscall" fn(u32) -> u32 =
                std::mem::transmute(crate::rebase(dtor::CLASS_DESCRIPTOR_DTOR));
            f(this)
        }
    })
}

/// `operator delete(p)` (ported), for the container helpers.
fn delete<M: Mem>(m: &mut M, p: u32) {
    heap::operator_delete(m, &mut GameOs, p, red_black_remove);
}

/// `PhyreFree(p)` (ported), for the container helpers.
fn phyre_free_cb<M: Mem>(m: &mut M, p: u32) {
    heap::phyre_free(m, &mut GameOs, p);
}

/// Runs `f` with a sorter of `size`-byte records over the game's
/// comparator `pred` (cdecl, two record addresses, `bool` in `al`) and a
/// scratch record on this stack.
fn with_sorter<R>(
    size: u32,
    pred: u32,
    f: impl FnOnce(
        &mut super::RawMem,
        &mut sort::bridge::Sorter<fn(&mut super::RawMem, u32, u32) -> bool>,
    ) -> R,
) -> R {
    thread_local! {
        static PRED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }
    fn less(m: &mut super::RawMem, a: u32, b: u32) -> bool {
        let pred = PRED.with(std::cell::Cell::get);
        call_ptr(m, pred, &[a, b]) & 0xff != 0
    }
    let mut tmp = [0u32; 7];
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    let at = tmp.as_mut_ptr() as u32;
    let saved = PRED.with(|c| c.replace(pred));
    let mut sorter = sort::bridge::Sorter {
        size,
        tmp: at,
        less: less as fn(&mut super::RawMem, u32, u32) -> bool,
    };
    let r = f(&mut mem(), &mut sorter);
    PRED.with(|c| c.set(saved));
    std::hint::black_box(&mut tmp);
    r
}

/// A name lookup through a kernel or battle table (`get(id, &strings)`):
/// the record's `u16` offset at `field`, the next word's with `kana` and
/// the hiragana setting on (`MsGetSaveConfigHiragana`).
fn table_name(
    id: u32,
    field: u32,
    kana: bool,
    get: impl FnOnce(&mut super::RawMem, u32, u32) -> u32,
) -> u32 {
    let m = &mut mem();
    let field = if kana && ffx_battle::save::bridge::ms_get_save_config_hiragana(m) != 0 {
        field + 4
    } else {
        field
    };
    let mut tmp = 0u32;
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    let at = std::ptr::addr_of_mut!(tmp) as u32;
    let r = tolib::bridge::table_name(m, id, field, at, get);
    std::hint::black_box(&mut tmp);
    r
}

/// The menu, script and sound calls of the TO library, through the
/// originals' addresses (their hooks where those are ported).
struct GameMenu;

impl<M> tolib::bridge::SaveMenu<M> for GameMenu {
    fn set_debug_lock(&mut self, mem: &mut M, v: u32) {
        call_sym(mem, 0x0088_8420, &[v]);
    }
    fn reset_toggles(&mut self, mem: &mut M) {
        call_sym(mem, 0x008b_4eb0, &[]);
    }
    fn set_screen_x2(&mut self, mem: &mut M, v: u32) {
        call_sym(mem, 0x008a_b020, &[v]);
    }
    fn set_frame_ratio_diff(&mut self, mem: &mut M, v: i32) -> u32 {
        call_sym(mem, 0x008a_b260, &[v.cast_unsigned()])
    }
}

impl<M> tolib::bridge::MainWindows<M> for GameMenu {
    fn alloc(&mut self, mem: &mut M) -> u32 {
        call_sym(mem, 0x008a_a1a0, &[])
    }
    fn regist(&mut self, mem: &mut M, w: u32) -> u32 {
        call_sym(mem, 0x008a_ab00, &[w])
    }
}

impl<M> tolib::bridge::ChrStatus<M> for GameMenu {
    fn cursor_flag(&mut self, mem: &mut M, chr: u32) -> u32 {
        call_sym(mem, 0x0079_4ca0, &[chr])
    }
    fn event_flag(&mut self, mem: &mut M, chr: u32, command: u32) -> u32 {
        call_sym(mem, 0x0079_4f20, &[chr, command])
    }
    fn dist(&mut self, mem: &mut M, chr: u32) -> u32 {
        call_sym(mem, 0x0079_4e00, &[chr])
    }
    fn dead(&mut self, mem: &mut M, chr: u32) -> u32 {
        call_sym(mem, 0x0079_5780, &[chr])
    }
}

impl<M> tolib::bridge::SummonList<M> for GameMenu {
    fn status_num(&mut self, mem: &mut M, id: u32, strings: u32) -> u32 {
        call_sym(mem, 0x0079_0200, &[id, strings])
    }
    fn command_max(&mut self, mem: &mut M, list: u32, kind: u32, a: u32, b: u32) -> i32 {
        call_sym(mem, 0x008d_2d80, &[list, kind, a, b]).cast_signed()
    }
    fn command_offset(&mut self, mem: &mut M, list: u32, kind: u32, a: u32, b: u32) -> i32 {
        call_sym(mem, 0x008d_2dc0, &[list, kind, a, b]).cast_signed()
    }
    fn joined(&mut self, mem: &mut M, chr: u32) -> u32 {
        call_sym(mem, 0x0078_5380, &[chr])
    }
}

impl<M> tolib::bridge::PlayerList<M> for GameMenu {
    fn in_party(&mut self, mem: &mut M, count: u32) -> u32 {
        call_sym(mem, 0x0078_5270, &[count])
    }
    fn out_party(&mut self, mem: &mut M, count: u32) -> u32 {
        call_sym(mem, 0x0078_52d0, &[count])
    }
    fn joined(&mut self, mem: &mut M, chr: u32) -> u32 {
        call_sym(mem, 0x0078_53a0, &[chr])
    }
}

impl<M: Mem> tolib::bridge::LimitOpenCalls<M> for GameMenu {
    fn limit_time(&mut self, mem: &mut M, who: tolib::bridge::Limit, arg: u32) -> f32 {
        let m = &*mem;
        match who {
            tolib::bridge::Limit::Auron => {
                ffx_field::nkz::bridge::nkz_get_auron_limit_time(m, arg.cast_signed())
            }
            tolib::bridge::Limit::Lulu => ffx_field::nkz::bridge::nkz_get_lulu_limit_time(m),
            tolib::bridge::Limit::Tidus => {
                ffx_field::nkz::bridge::nkz_get_tidus_limit_time(m, arg.cast_signed())
            }
        }
    }
    fn auron_buttons(&mut self, mem: &mut M, level: u32) -> u32 {
        call_sym(mem, 0x00a5_ca60, &[level])
    }
    fn lulu_table(&mut self, mem: &mut M, group: u32) {
        call_sym(mem, 0x00a5_cc70, &[group]);
    }
    fn tidus_table(&mut self, mem: &mut M, b: bool, level: u32) -> u32 {
        call_sym(mem, if b { 0x00a5_cb80 } else { 0x00a5_cb30 }, &[level])
    }
}

impl<M: Mem> tolib::bridge::LimitWindowCalls<M> for GameMenu {
    fn seq_num(&mut self, mem: &mut M, i: u32, name: u32, out: u32) {
        call_sym(mem, 0x008a_e410, &[i, name, out]);
    }
    fn seq_point(&mut self, mem: &mut M, i: u32, name: u32, x: u32, y: u32) {
        call_sym(mem, 0x008a_e450, &[i, name, x, y]);
    }
    fn seq_color(&mut self, mem: &mut M, i: u32, name: u32, out: u32) {
        call_sym(mem, 0x008a_e360, &[i, name, out]);
    }
    fn init_pad(&mut self, mem: &mut M) {
        tolib::bridge::init_pad_data(mem);
    }
    fn init_limit_timer(&mut self, mem: &mut M) {
        call_sym(mem, 0x0089_7f00, &[]);
    }
    fn remap(&mut self, _mem: &mut M, y: bool, v: f32) -> f32 {
        if y {
            ffx_graphic::ui::remap_y2(v)
        } else {
            ffx_graphic::ui::remap_x2(v)
        }
    }
}

/// A CRT `_CI*` intrinsic through the game's import thunk at `sym`: `x`
/// in ST0 (`fld qword`), the result read back from ST0 (`fstp qword`).
fn intrinsic(sym: u32, x: f64) -> f64 {
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    let f = crate::rebase(sym) as u32;
    // SAFETY: the thunk jumps to the CRT intrinsic, which takes its
    // argument in ST0 and leaves the result there (eax, ecx, edx, the x87
    // and SSE registers clobbered; edi kept).
    #[cfg(target_arch = "x86")]
    unsafe {
        let mut out = 0f64;
        std::arch::asm!(
            "fld qword ptr [{x}]",
            "call {f}",
            "fstp qword ptr [edi]",
            x = in(reg) &raw const x,
            f = in(reg) f,
            in("edi") &raw mut out,
            clobber_abi("C"),
        );
        out
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = (f, x);
        f64::NAN
    }
}

/// A cdecl function of no arguments at `sym` that returns a float in ST0,
/// read back with `fstp qword`.
fn st0_result(sym: u32) -> f64 {
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    let f = crate::rebase(sym) as u32;
    // SAFETY: a cdecl function of no arguments; it leaves its result in
    // ST0 and keeps edi.
    #[cfg(target_arch = "x86")]
    unsafe {
        let mut out = 0f64;
        std::arch::asm!(
            "call {f}",
            "fstp qword ptr [edi]",
            f = in(reg) f,
            in("edi") &raw mut out,
            clobber_abi("C"),
        );
        out
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = f;
        f64::NAN
    }
}

impl<M: Mem> tolib::bridge::PadInput<M> for GameMenu {
    fn cur_time(&mut self, _mem: &mut M) -> f64 {
        st0_result(0x0063_0c60)
    }
    fn hist(&mut self, mem: &mut M, a: u32, b: u32, c: u32) -> u32 {
        call_sym(mem, 0x0088_8d70, &[a, b, c])
    }
    fn analog(&mut self, mem: &mut M, a: u32, b: u32, axis: u32, d: u32) -> u32 {
        call_sym(mem, 0x0088_8c20, &[a, b, axis, d])
    }
}

impl<M: Mem> tolib::bridge::NearTarget<M> for GameMenu {
    fn btl_pos(&mut self, mem: &mut M, chr: u32, vec: u32) {
        call_sym(mem, 0x0079_48e0, &[chr, vec]);
    }
    fn dist(&mut self, mem: &mut M, chr: u32) -> u32 {
        call_sym(mem, 0x0079_4e00, &[chr])
    }
    fn sqrt(&mut self, _mem: &mut M, x: f64) -> f64 {
        intrinsic(0x0094_97b8, x)
    }
    fn acos(&mut self, _mem: &mut M, x: f64) -> f64 {
        intrinsic(0x0094_98da, x)
    }
}

impl<M: Mem> menu::bridge::MagicFileCalls<M> for GameMenu {
    fn printf(&mut self, mem: &mut M, fmt: u32, arg: Option<u32>) {
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let slot = crate::rebase(0x00b0_c380) as u32;
        let f: u32 = mem.read(slot);
        match arg {
            Some(a) => call_ptr(mem, f, &[fmt, a]),
            None => call_ptr(mem, f, &[fmt]),
        };
    }
    fn game_over_notify(&mut self, mem: &mut M, on: u32) {
        call_sym(mem, 0x0064_3260, &[on]);
    }
    fn init_magic(&mut self, mem: &mut M, imports: u32) {
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let slot = crate::rebase(0x022f_b6a0) as u32;
        let f: u32 = mem.read(slot);
        call_ptr(mem, f, &[imports]);
    }
    fn fog_fade(&mut self, mem: &mut M, on: u32) {
        call_sym(mem, 0x0064_2f10, &[on]);
    }
    fn reset_zanmato(&mut self, mem: &mut M) {
        call_sym(mem, 0x007a_8f20, &[]);
    }
}

impl<M: Mem> tolib::bridge::TextureName<M> for GameMenu {
    fn battle_status(&mut self, mem: &mut M) -> u32 {
        call_sym(mem, 0x0078_16f0, &[])
    }
    fn tk_lang(&mut self, mem: &mut M) -> u32 {
        call_sym(mem, 0x0088_7c30, &[])
    }
    fn ffx_lang(&mut self, mem: &mut M) -> u32 {
        instance::bridge::ffx_lang(mem, &mut phyre_printf)
    }
    fn print(&mut self, mem: &mut M, fmt: u32) {
        call_sym(mem, 0x0062_f500, &[fmt]);
    }
}

impl<M: Mem> tolib::bridge::FontWidth<M> for GameMenu {
    fn screen_wh(&mut self, mem: &mut M) -> (i32, i32) {
        font_screen_wh(mem)
    }
    fn lang(&mut self, mem: &mut M) -> u32 {
        instance::bridge::ffx_lang(mem, &mut phyre_printf)
    }
    fn width(&mut self, mem: &mut M, width: f32, scale: f32) -> f32 {
        font_width(mem, width, scale)
    }
    fn space(&mut self, mem: &mut M, kind: u32) -> f32 {
        font_space(mem, kind)
    }
}

impl<M> tolib::bridge::MenuMusic<M> for GameMenu {
    fn keep_white_in_out(&mut self, mem: &mut M) -> u32 {
        call_sym(mem, 0x0086_bc30, &[])
    }
    fn keep_clear11(&mut self, mem: &mut M) -> u32 {
        call_sym(mem, 0x0086_bc50, &[])
    }
    fn music_number(&mut self, mem: &mut M) -> u32 {
        call_sym(mem, 0x0087_3b00, &[])
    }
    fn regist_music_number(&mut self, mem: &mut M, n: u32) {
        call_sym(mem, 0x0087_3d00, &[n]);
    }
    fn now_playing(&mut self, mem: &mut M) -> u32 {
        call_sym(mem, 0x0088_63f0, &[])
    }
    fn cross_start(&mut self, mem: &mut M, n: u32, frames: u32) -> u32 {
        call_sym(mem, 0x0088_6830, &[n, frames])
    }
    fn fade_out(&mut self, mem: &mut M, n: u32, frames: u32) -> u32 {
        call_sym(mem, 0x0088_6af0, &[n, frames])
    }
}

/// `graphicFontGetScreenWH(&w, &h)` (the original, or its hook).
fn font_screen_wh<M>(m: &mut M) -> (i32, i32) {
    let mut wh = [0i32; 2];
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    let at = wh.as_mut_ptr() as u32;
    call_sym(m, 0x0064_0db0, &[at, at + 4]);
    std::hint::black_box(&mut wh);
    (wh[0], wh[1])
}

/// `SndIsSepPlay(id)` or, with `stop`, `SndSepStop(id)` (their hooks).
fn limit_sound<M>(m: &mut M, stop: bool, id: u32) -> u32 {
    call_sym(m, if stop { 0x0088_6eb0 } else { 0x0088_6680 }, &[id])
}

/// `TOGetFontWidthInPiexl(a, b)` (ported), for the TO library's callers.
fn font_width<M: Mem>(m: &mut M, a: f32, b: f32) -> f32 {
    tolib::bridge::font_width_in_pixel(m, a, b, font_screen_wh)
}

/// `TOGetFontSpaceInPiexl(kind)` (ported).
fn font_space<M: Mem>(m: &mut M, kind: u32) -> f32 {
    tolib::bridge::font_space_in_pixel(
        m,
        kind,
        |m| instance::bridge::ffx_lang(m, &mut phyre_printf),
        font_screen_wh,
    )
}

/// The heap's Win32 calls, through the game's own import slots (so they are
/// the calls the originals make, to the same critical sections).
struct GameOs;

impl GameOs {
    /// The function in import slot `slot`.
    fn import(slot: u32) -> usize {
        // SAFETY: an import slot of this build, filled by the loader.
        unsafe { (crate::rebase(slot) as *const usize).read() }
    }

    /// The one-argument stdcall import in `slot`; its `eax` (what a `void`
    /// import leaves is read as a `u32`, as the originals pass it on).
    fn stdcall1(slot: u32, x: u32) -> u32 {
        // SAFETY: a one-argument stdcall KERNEL32 import of this build.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32) -> u32 = std::mem::transmute(Self::import(slot));
            f(x)
        }
    }
}

/// The thread and synchronisation wrappers' KERNEL32 calls, through the
/// same import slots; each returns the import's `eax`.
impl<M> sync::Kernel<M> for GameOs {
    fn sleep(&mut self, _mem: &mut M, ms: u32) -> u32 {
        Self::stdcall1(0x00b0_c100, ms)
    }
    fn wait(&mut self, _mem: &mut M, handle: u32, ms: u32) -> u32 {
        // SAFETY: WaitForSingleObject(handle, ms), stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32, u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c0f8));
            f(handle, ms)
        }
    }
    fn release_semaphore(&mut self, _mem: &mut M, handle: u32, count: u32, previous: u32) -> u32 {
        // SAFETY: ReleaseSemaphore(handle, count, previous), stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32, u32, u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c0f4));
            f(handle, count, previous)
        }
    }
    fn create_semaphore(
        &mut self,
        _mem: &mut M,
        attributes: u32,
        initial: u32,
        maximum: u32,
        name: u32,
    ) -> u32 {
        // SAFETY: CreateSemaphoreW(attributes, initial, maximum, name), stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32, u32, u32, u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c1e0));
            f(attributes, initial, maximum, name)
        }
    }
    fn close_handle(&mut self, _mem: &mut M, handle: u32) -> u32 {
        Self::stdcall1(0x00b0_c0dc, handle)
    }
    fn set_event(&mut self, _mem: &mut M, handle: u32) -> u32 {
        Self::stdcall1(0x00b0_c0ec, handle)
    }
    fn reset_event(&mut self, _mem: &mut M, handle: u32) -> u32 {
        Self::stdcall1(0x00b0_c0f0, handle)
    }
    fn initialize_critical_section(&mut self, _mem: &mut M, cs: u32) -> u32 {
        Self::stdcall1(0x00b0_c150, cs)
    }
    fn enter_critical_section(&mut self, _mem: &mut M, cs: u32) -> u32 {
        Self::stdcall1(0x00b0_c154, cs)
    }
    fn leave_critical_section(&mut self, _mem: &mut M, cs: u32) -> u32 {
        Self::stdcall1(0x00b0_c158, cs)
    }
    fn delete_critical_section(&mut self, _mem: &mut M, cs: u32) -> u32 {
        Self::stdcall1(0x00b0_c15c, cs)
    }
}

impl heap::Os for GameOs {
    fn enter_critical_section(&mut self, cs: u32) {
        // SAFETY: EnterCriticalSection(cs), stdcall; `cs` is the game's.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32) = std::mem::transmute(Self::import(0x00b0_c154));
            f(cs);
        }
    }
    fn leave_critical_section(&mut self, cs: u32) {
        // SAFETY: LeaveCriticalSection(cs), stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32) = std::mem::transmute(Self::import(0x00b0_c158));
            f(cs);
        }
    }
    fn virtual_alloc(&mut self, addr: u32, size: u32, alloc_type: u32, protect: u32) -> u32 {
        // SAFETY: VirtualAlloc, stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32, u32, u32, u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c09c));
            f(addr, size, alloc_type, protect)
        }
    }
    fn virtual_free(&mut self, addr: u32, size: u32, free_type: u32) -> u32 {
        // SAFETY: VirtualFree, stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32, u32, u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c098));
            f(addr, size, free_type)
        }
    }
    fn report_free_failure(&mut self) {
        // As the originals: FormatMessageA(ALLOCATE_BUFFER | FROM_SYSTEM,
        // 0, GetLastError(), 0, &message, 0x1000, &args), then the error
        // hook at 0x0193cd4c with the message, when one is set.
        // SAFETY: GetLastError and FormatMessageA (stdcall) through their
        // import slots; the hook is the game's cdecl callback.
        unsafe {
            let last: unsafe extern "stdcall" fn() -> u32 =
                std::mem::transmute(Self::import(0x00b0_c030));
            let format: unsafe extern "stdcall" fn(
                u32,
                u32,
                u32,
                u32,
                *mut u32,
                u32,
                *mut u32,
            ) -> u32 = std::mem::transmute(Self::import(0x00b0_c070));
            let mut message = 0u32;
            let mut args = 0u32;
            format(
                0x1100,
                0,
                last(),
                0,
                &raw mut message,
                0x1000,
                &raw mut args,
            );
            let hook = (crate::rebase(0x0193_cd4c) as *const usize).read();
            if hook != 0 {
                let f: unsafe extern "C" fn(u32) = std::mem::transmute(hook);
                f(message);
            }
        }
    }
    fn initialize_critical_section(&mut self, cs: u32) {
        // SAFETY: InitializeCriticalSection(cs), stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32) = std::mem::transmute(Self::import(0x00b0_c150));
            f(cs);
        }
    }
    fn crt_malloc(&mut self, size: u32) -> u32 {
        // SAFETY: the C runtime's malloc, cdecl.
        unsafe {
            let f: unsafe extern "C" fn(u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c468));
            f(size)
        }
    }
    fn crt_free(&mut self, p: u32) {
        // SAFETY: the C runtime's free, cdecl.
        unsafe {
            let f: unsafe extern "C" fn(u32) = std::mem::transmute(Self::import(0x00b0_c370));
            f(p);
        }
    }
    fn mem_system_init(&mut self) {
        // the original slow path of 0x009428a0 (cdecl, no arguments)
        call0(0x0094_28b0);
    }
    fn wait_for_single_object(&mut self, handle: u32, ms: u32) -> u32 {
        // SAFETY: WaitForSingleObject, stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32, u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c0f8));
            f(handle, ms)
        }
    }
    fn release_mutex(&mut self, handle: u32) -> u32 {
        // SAFETY: ReleaseMutex, stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c1c0));
            f(handle)
        }
    }
}

ported! {
    /// `set_ptr32(p)`: `p`.
    set_ptr32 @ 0x00a4_4700 => extern "C" fn set_ptr32(p: u32) -> u32 {
        stub::ptr32(p)
    }
    /// `get_ptr32_func(p)`: `p`.
    get_ptr32_func @ 0x00a4_4650 => extern "C" fn get_ptr32_func(p: u32) -> u32 {
        stub::ptr32(p)
    }
    /// `is_ptr32_null(p)`.
    is_ptr32_null @ 0x00a4_46a0 => extern "C" fn is_ptr32_null(p: u32) -> u32 {
        stub::is_ptr32_null(p)
    }
    /// `sceCdDiskReady(mode)`: 2.
    sceCdDiskReady @ 0x0062_f550 => extern "C" fn sce_cd_disk_ready() -> u32 {
        stub::CD_DISK_READY
    }
    /// `sceCdRead(...)`: 1.
    sceCdRead @ 0x00a4_47c0 => extern "C" fn sce_cd_read() -> u32 {
        stub::CD_READ
    }
    /// `sceSifSetDma(...)`: 1.
    sceSifSetDma @ 0x00a4_4790 => extern "C" fn sce_sif_set_dma() -> u32 {
        stub::SIF_SET_DMA
    }
    /// `scePadGetState(port, slot)`: 6.
    scePadGetState @ 0x0062_fb30 => extern "C" fn sce_pad_get_state() -> u32 {
        stub::PAD_GET_STATE
    }
    /// `sceMcSync(mode, &cmd, &result)`.
    sceMcSync @ 0x0062_f9f0 => extern "C" fn sce_mc_sync(_mode: u32, cmd: u32, result: u32) -> i32 {
        stub::bridge::mc_sync(&mut mem(), cmd, result)
    }
    /// `ModelBuffer_AllocateBuffer(...)`: 0xffff.
    ModelBuffer_AllocateBuffer @ 0x0072_ac20 => extern "C" fn model_buffer_allocate_buffer() -> u32 {
        stub::MODEL_BUFFER_ALLOCATE
    }

    /// `GetCD()`: `&g_CdLib`.
    GetCD @ 0x0076_d0d0 => extern "C" fn get_cd() -> u32 {
        cd::bridge::get_cd(&mem())
    }
    /// `cd_file_sector(n)`.
    cd_file_sector @ 0x0076_d510 => extern "C" fn cd_file_sector(n: i32) -> u32 {
        cd::bridge::cd_file_sector(&mem(), n)
    }
    /// `CDfileSize_CD_EX(n)`.
    CDfileSize_CD_EX @ 0x0076_c410 => extern "C" fn cd_file_size_cd_ex(n: i32) -> u32 {
        cd::bridge::cd_file_size_ex(&mem(), n)
    }
    /// `check_ex_file_size(archive, n)`.
    check_ex_file_size @ 0x0076_d660 => extern "C" fn check_ex_file_size(archive: u32, n: i32) -> u32 {
        cd::bridge::check_ex_file_size(&mem(), archive, n)
    }
    /// `disk_open_check_set(flag)` (a byte).
    disk_open_check_set @ 0x0076_d6b0 => extern "C" fn disk_open_check_set(flag: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores AL
        cd::bridge::disk_open_check_set(&mut mem(), flag as u8);
    }
    /// `hdd_mount_flg_get()`.
    hdd_mount_flg_get @ 0x0077_9560 => extern "C" fn hdd_mount_flg_get() -> i32 {
        cd::bridge::hdd_mount_flg_get(&mem())
    }
    /// `hdd_rmode_get(n)`.
    hdd_rmode_get @ 0x0077_9570 => extern "C" fn hdd_rmode_get(n: i32) -> i32 {
        cd::bridge::hdd_rmode_get(&mem(), n)
    }
    /// `hdd_head_reset()`.
    hdd_head_reset @ 0x0077_89e0 => extern "C" fn hdd_head_reset() {
        cd::bridge::hdd_head_reset(&mut mem());
    }
    /// `clear_read_chgname(i)`.
    clear_read_chgname @ 0x00a4_2f30 => extern "C" fn clear_read_chgname(i: i32) {
        cd::bridge::clear_read_chgname(&mut mem(), i);
    }
    /// `setFullArenaLocalizationEnable()`.
    setFullArenaLocalizationEnable @ 0x00a4_31f0 => extern "C" fn set_full_arena_localization_enable() {
        cd::bridge::set_full_arena_localization_enable(&mut mem());
    }

    /// `SaveDataManager::instance()`.
    SaveDataManager_instance @ 0x006f_2070 => extern "C" fn save_data_manager_instance() -> u32 {
        save::bridge::manager_instance(&mem())
    }
    /// `SaveDataManager::getRefBuffer()`.
    SaveDataManager_getRefBuffer @ 0x006f_1ed0 => extern "thiscall" fn save_data_manager_get_ref_buffer(this: u32) -> u32 {
        save::bridge::get_ref_buffer(&mem(), this)
    }
    /// `SaveDataManager::setCbResult(result)`.
    SaveDataManager_setCbResult @ 0x006f_2500 => extern "thiscall" fn save_data_manager_set_cb_result(this: u32, result: u32) {
        save::bridge::set_cb_result(&mut mem(), this, result);
    }
    /// `SaveDataManager::isBusy()`.
    SaveDataManager_isBusy @ 0x006f_2090 => extern "thiscall" fn save_data_manager_is_busy(this: u32) -> u32 {
        save::bridge::is_busy(&mem(), this)
    }
    /// `SaveDataSetEventId(id)`.
    SaveDataSetEventId @ 0x0064_8630 => extern "C" fn save_data_set_event_id(id: u32) {
        save::bridge::set_event_id(&mut mem(), id);
    }
    /// `SaveDataGetScreenState()`.
    SaveDataGetScreenState @ 0x0064_7f70 => extern "C" fn save_data_get_screen_state() -> u32 {
        save::bridge::get_screen_state(&mem())
    }
    /// `SaveDataIsScreenClosed()`.
    SaveDataIsScreenClosed @ 0x0064_8100 => extern "C" fn save_data_is_screen_closed() -> u32 {
        save::bridge::is_screen_closed(&mem())
    }
    /// `SaveDataGetDialogState()`.
    SaveDataGetDialogState @ 0x0064_7ee0 => extern "C" fn save_data_get_dialog_state() -> u32 {
        save::bridge::get_dialog_state(&mem())
    }
    /// `SaveDataSetDialogState(state)`.
    SaveDataSetDialogState @ 0x0064_8620 => extern "C" fn save_data_set_dialog_state(state: u32) {
        save::bridge::set_dialog_state(&mut mem(), state);
    }
    /// `SaveDataIsSave()`.
    SaveDataIsSave @ 0x0064_80d0 => extern "C" fn save_data_is_save() -> u32 {
        save::bridge::is_save(&mem())
    }
    /// `SaveDataSetSave(save)`.
    SaveDataSetSave @ 0x0064_86b0 => extern "C" fn save_data_set_save(save: u32) {
        save::bridge::set_save(&mut mem(), save);
    }
    /// `SaveDataIsLoadingSuccessful()`.
    SaveDataIsLoadingSuccessful @ 0x0064_80a0 => extern "C" fn save_data_is_loading_successful() -> u32 {
        save::bridge::is_loading_successful(&mem())
    }

    /// `ClusterManager::GetInstance()`.
    ClusterManager_GetInstance @ 0x0069_a1a0 => extern "C" fn cluster_manager_get_instance() -> u32 {
        instance::bridge::cluster_manager(&mem())
    }
    /// `AsyncLoadManager::sGetInstance()`.
    AsyncLoadManager_sGetInstance @ 0x0069_1b40 => extern "C" fn async_load_manager_s_get_instance() -> u32 {
        instance::bridge::async_load_manager(&mem())
    }
    /// `AsyncLoadManager::isAllRequestDone()`.
    AsyncLoadManager_isAllRequestDone @ 0x0069_1630 => extern "thiscall" fn async_load_manager_is_all_request_done(this: u32) -> bool {
        instance::bridge::is_all_request_done(&mem(), this)
    }
    /// `BigFileStream::get()`.
    BigFileStream_get @ 0x0061_bdb0 => extern "C" fn big_file_stream_get() -> u32 {
        instance::bridge::big_file_stream(&mem())
    }
    /// `MagicFile_GetMagicId()`.
    MagicFile_GetMagicId @ 0x009d_a3b0 => extern "C" fn magic_file_get_magic_id() -> u32 {
        instance::bridge::magic_id(&mem())
    }
    /// `MagicFile_GetHandle()`.
    MagicFile_GetHandle @ 0x009d_a390 => extern "C" fn magic_file_get_handle() -> u32 {
        instance::bridge::magic_handle(&mem())
    }
    /// `MagicFile_IsNexData()`.
    MagicFile_IsNexData @ 0x009d_a420 => extern "C" fn magic_file_is_nex_data() -> u32 {
        instance::bridge::magic_is_nex(&mem())
    }
    /// `AssetListReader::getAssetCount()`.
    AssetListReader_getAssetCount @ 0x006f_4170 => extern "thiscall" fn asset_list_reader_get_asset_count(this: u32) -> i32 {
        instance::bridge::asset_count(&mem(), this)
    }

    /// `FFXIsDebug()`.
    FFXIsDebug @ 0x0088_7c50 => extern "C" fn ffx_is_debug() -> i32 {
        config::bridge::is_debug(&mem())
    }
    /// `FFXGetAsiaMode()`.
    FFXGetAsiaMode @ 0x0088_7c20 => extern "C" fn ffx_get_asia_mode() -> u32 {
        config::bridge::get_asia_mode(&mem())
    }
    /// `FFXSetAsiaMode(mode)` (a byte).
    FFXSetAsiaMode @ 0x0088_7c70 => extern "C" fn ffx_set_asia_mode(mode: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores AL
        config::bridge::set_asia_mode(&mut mem(), mode as u8);
    }
    /// `TOGetFFXLangSwapFlag()`.
    TOGetFFXLangSwapFlag @ 0x008a_c390 => extern "C" fn to_get_ffx_lang_swap_flag() -> u32 {
        config::bridge::lang_swap_flag(&mem())
    }
    /// `TOGetFFXVideoMode()`.
    TOGetFFXVideoMode @ 0x008a_c3a0 => extern "C" fn to_get_ffx_video_mode() -> u32 {
        config::bridge::video_mode(&mem())
    }
    /// `TOGetSystemConfirmButtonAssign()`.
    TOGetSystemConfirmButtonAssign @ 0x008a_cf30 => extern "C" fn to_get_system_confirm_button_assign() -> i32 {
        config::bridge::confirm_button_assign(&mut mem())
    }
    /// `isEternalCalmScene(n)`.
    isEternalCalmScene @ 0x0067_ab50 => extern "C" fn is_eternal_calm_scene(n: i32) -> u32 {
        config::bridge::is_eternal_calm(&mem(), n)
    }
    /// `isEternalCalmMgrp(n)`.
    isEternalCalmMgrp @ 0x0067_ab30 => extern "C" fn is_eternal_calm_mgrp(n: i32) -> u32 {
        config::bridge::is_eternal_calm(&mem(), n)
    }
    /// `getEternalCalmSceneName(n)`.
    getEternalCalmSceneName @ 0x0067_aac0 => extern "C" fn get_eternal_calm_scene_name(n: i32) -> u32 {
        config::bridge::eternal_calm_scene_name(&mem(), n)
    }
    /// `setBattleEnable(on)`.
    setBattleEnable @ 0x0078_2f70 => extern "C" fn set_battle_enable(on: u32) {
        config::bridge::set_battle_enable(&mut mem(), on);
    }

    /// `TOSkipCode(p)`.
    TOSkipCode @ 0x008d_fd80 => extern "C" fn to_skip_code(p: u32) -> u32 {
        msg::bridge::skip_code(&mem(), p)
    }
    /// `TOGetGroup(code)`.
    TOGetGroup @ 0x008d_f610 => extern "C" fn to_get_group(code: u32) -> u32 {
        msg::group(code)
    }

    /// `cdc::Mul4x4(dst, m, v)`: a 4-vector times a matrix.
    cdc_Mul4x4 @ 0x0070_58f0 => extern "C" fn cdc_mul4x4(dst: u32, m: u32, v: u32) -> u32 {
        math::bridge::mul4x4_vector(&mut mem(), dst, m, v);
        dst // left in eax; FFXVu0ApplyMatrix returns it
    }
    /// `cdc::Mul4x4(dst, a, b)`: `b × a`.
    cdc_Mul4x4_007059b0 @ 0x0070_59b0 => extern "C" fn cdc_mul4x4_matrix(dst: u32, a: u32, b: u32) {
        math::bridge::mul4x4(&mut mem(), dst, a, b);
    }
    /// `cdc::Mul3x4(dst, m, v)`: a point transformed.
    cdc_Mul3x4 @ 0x0070_5850 => extern "C" fn cdc_mul3x4(dst: u32, m: u32, v: u32) -> u32 {
        math::bridge::mul3x4(&mut mem(), dst, m, v);
        dst // left in eax; FFXVu0RotTransPers and the envelope code use it
    }
    /// `cdc::Transpose(dst, m)`.
    cdc_Transpose @ 0x0070_63f0 => extern "C" fn cdc_transpose(dst: u32, m: u32) {
        math::bridge::transpose(&mut mem(), dst, m);
    }
    /// `cdc::OrthonormalInverse3x4(dst, m)`.
    cdc_OrthonormalInverse3x4 @ 0x0070_5db0 => extern "C" fn cdc_orthonormal_inverse3x4(dst: u32, m: u32) {
        math::bridge::orthonormal_inverse3x4(&mut mem(), dst, m);
    }
    /// `cdc::Matrix::RotX(sin, cos)`.
    cdc_Matrix_RotX @ 0x0070_5ed0 => extern "thiscall" fn cdc_matrix_rot_x(this: u32, sin: f32, cos: f32) -> u32 {
        math::bridge::rotate(&mut mem(), this, 1, sin, cos)
    }
    /// `cdc::Matrix::RotY(sin, cos)`.
    cdc_Matrix_RotY @ 0x0070_5fa0 => extern "thiscall" fn cdc_matrix_rot_y(this: u32, sin: f32, cos: f32) -> u32 {
        math::bridge::rotate_y(&mut mem(), this, sin, cos)
    }
    /// `cdc::Matrix::RotZ(sin, cos)`.
    cdc_Matrix_RotZ @ 0x0070_6070 => extern "thiscall" fn cdc_matrix_rot_z(this: u32, sin: f32, cos: f32) -> u32 {
        math::bridge::rotate(&mut mem(), this, 0, sin, cos)
    }
    /// `cdc::Vector::CanNormalize3()` (`this` in ECX): `al` 1 when the
    /// square sum is a finite number above `FLT_MIN`; the rest of `eax` as
    /// the original leaves it.
    cdc_Vector_CanNormalize3 @ 0x0070_3bc0 => extern "thiscall" fn cdc_vector_can_normalize3(this: u32) -> u32 {
        math::bridge::can_normalize3(&mem(), this)
    }
    /// `cdc::Vector::SafeNormalize3()` (`this` in ECX, fastcall).
    cdc_Vector_SafeNormalize3 @ 0x0070_4200 => extern "fastcall" fn cdc_vector_safe_normalize3(v: u32) {
        math::bridge::safe_normalize3(&mut mem(), v);
    }
    /// `cdc::Inverse4x4(dst, m)`: the general inverse (cofactors).
    cdc_Inverse4x4 @ 0x0070_4dd0 => extern "C" fn cdc_inverse4x4(dst: u32, m: u32) -> u32 {
        math::bridge::inverse4x4(&mut mem(), dst, m)
    }
    /// `sceVu0InversMatrix4x4(dst, m)`.
    sceVu0InversMatrix4x4 @ 0x006e_d460 => extern "C" fn sce_vu0_invers_matrix4x4(dst: u32, m: u32) -> u32 {
        math::bridge::invers_matrix4x4(&mut mem(), dst, m)
    }
    /// `sceVu0VectorLengthSquared(v)`.
    sceVu0VectorLengthSquared @ 0x006e_d9b0 => extern "C" fn sce_vu0_vector_length_squared(v: u32) -> f32 {
        math::bridge::length_squared(&mem(), v)
    }

    /// `TOGetWindowStructAdrs(i)`.
    TOGetWindowStructAdrs @ 0x0090_6fb0 => extern "C" fn to_get_window_struct_adrs(i: i32) -> u32 {
        window::bridge::window(&mem(), i)
    }
    /// `TOGetWindowType(i)`.
    TOGetWindowType @ 0x0090_6c40 => extern "C" fn to_get_window_type(i: i32) -> i32 {
        window::bridge::window_type(&mem(), i)
    }
    /// `TOGetWindowStatus(i)`.
    TOGetWindowStatus @ 0x0090_6c20 => extern "C" fn to_get_window_status(i: i32) -> i32 {
        window::bridge::window_status(&mem(), i)
    }
    /// `TOCloseWindow(i)`.
    TOCloseWindow @ 0x0090_6ba0 => extern "C" fn to_close_window(i: i32) {
        window::bridge::close_window(&mut mem(), i);
    }
    /// `TOGetMessageStatus(i)`.
    TOGetMessageStatus @ 0x008b_7ca0 => extern "C" fn to_get_message_status(i: i32) -> i32 {
        window::bridge::message_status(&mem(), i)
    }
    /// `TOCloseMessage(i)`.
    TOCloseMessage @ 0x008b_5810 => extern "C" fn to_close_message(i: i32) {
        window::bridge::close_message(&mut mem(), i);
    }
    /// `TOSetMessageFunc(i, n)`.
    TOSetMessageFunc @ 0x008b_8af0 => extern "C" fn to_set_message_func(i: i32, n: i32) {
        window::bridge::set_message_func(&mut mem(), i, n);
    }
    /// `TOBtlSetMacroCommandType(i, n, type)` (a byte).
    TOBtlSetMacroCommandType @ 0x008b_57c0 => extern "C" fn to_btl_set_macro_command_type(i: i32, n: i32, ty: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores CL
        window::bridge::set_macro_command_type(&mut mem(), i, n, ty as u8);
    }
    /// `TOBtlSetMacroCommandValue(i, n, value)`.
    TOBtlSetMacroCommandValue @ 0x008b_57f0 => extern "C" fn to_btl_set_macro_command_value(i: i32, n: i32, value: u32) {
        window::bridge::set_macro_command_value(&mut mem(), i, n, value);
    }
    /// `TOInitMacroCmd(i)`.
    TOInitMacroCmd @ 0x008b_81d0 => extern "C" fn to_init_macro_cmd(i: i32) {
        window::bridge::init_macro_cmd(&mut mem(), i);
    }
    /// `MacroDic_Section(n, table)`.
    MacroDic_Section @ 0x008b_70a0 => extern "C" fn macro_dic_section(n: i32, table: i32) -> u32 {
        window::bridge::macro_dic_section(&mem(), n, table)
    }
    /// `TOGetSaveDicAdrs(table)`.
    TOGetSaveDicAdrs @ 0x008b_7f60 => extern "C" fn to_get_save_dic_adrs(table: i32) -> u32 {
        window::bridge::save_dic(&mem(), table)
    }

    /// `TOMenuGetControlPad()`.
    TOMenuGetControlPad @ 0x008b_e410 => extern "C" fn to_menu_get_control_pad() -> u32 {
        let flag = ffx_menu::tk::bridge::tk_menu_get_dummy_controler_flag(&mem());
        u32::from(pad::bridge::control_pad(&mem(), 0, flag))
    }
    /// `TOMenuGetControlPadTrg()`.
    TOMenuGetControlPadTrg @ 0x008b_e4b0 => extern "C" fn to_menu_get_control_pad_trg() -> u32 {
        let flag = ffx_menu::tk::bridge::tk_menu_get_dummy_controler_flag(&mem());
        u32::from(pad::bridge::control_pad(&mem(), 1, flag))
    }
    /// `TOMenuGetControlPadRep()`.
    TOMenuGetControlPadRep @ 0x008b_e470 => extern "C" fn to_menu_get_control_pad_rep() -> u32 {
        let flag = ffx_menu::tk::bridge::tk_menu_get_dummy_controler_flag(&mem());
        u32::from(pad::bridge::control_pad(&mem(), 2, flag))
    }
    /// `TOMenuGetControlPad2()` (only `AX` defined).
    TOMenuGetControlPad2 @ 0x008b_e440 => extern "C" fn to_menu_get_control_pad2() -> u32 {
        u32::from(pad::bridge::control_pad2(&mem()))
    }
    /// `TOMenuGetDummyAnalogControler(axis)` (only `AL` defined).
    TOMenuGetDummyAnalogControler @ 0x008c_a320 => extern "C" fn to_menu_get_dummy_analog_controler(axis: u32) -> u32 {
        u32::from(pad::bridge::dummy_analog(&mem(), axis))
    }
    /// `SetTOPadGetCtrl(v)` (a byte).
    SetTOPadGetCtrl @ 0x008a_b580 => extern "C" fn set_to_pad_get_ctrl(v: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores AL
        pad::bridge::set_get_ctrl(&mut mem(), v as u8);
    }
    /// `TODebugAddPadPrintFlag()`.
    TODebugAddPadPrintFlag @ 0x008a_bd00 => extern "C" fn to_debug_add_pad_print_flag() {
        pad::bridge::add_print_flag(&mut mem());
    }

    /// `TOBtlCloseHelpWin()`.
    TOBtlCloseHelpWin @ 0x0089_0eb0 => extern "C" fn to_btl_close_help_win() {
        btl::bridge::close_help_win(&mut mem());
    }
    /// `TOBtlInitAnm()`.
    TOBtlInitAnm @ 0x008a_0cd0 => extern "C" fn to_btl_init_anm() {
        btl::bridge::init_anm(&mut mem());
    }
    /// `TOBtlCloseSignWindow()`.
    TOBtlCloseSignWindow @ 0x0089_0ee0 => extern "C" fn to_btl_close_sign_window() {
        btl::bridge::close_sign_window(&mut mem());
    }
    /// `TOBtlCloseSimpleHelpMes()`.
    TOBtlCloseSimpleHelpMes @ 0x0089_0ef0 => extern "C" fn to_btl_close_simple_help_mes() {
        btl::bridge::close_simple_help(&mut mem());
    }
    /// `TOGetBtlWindowControlEnable()`.
    TOGetBtlWindowControlEnable @ 0x0089_b590 => extern "C" fn to_get_btl_window_control_enable() -> u32 {
        btl::bridge::window_control_enable(&mem())
    }
    /// `TOBtlSetCurPos(x, y, z)` (16-bit each).
    TOBtlSetCurPos @ 0x0089_99f0 => extern "C" fn to_btl_set_cur_pos(x: u32, y: u32, z: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores AX
        btl::bridge::set_cur_pos(&mut mem(), x as u16, y as u16, z as u16);
    }
    /// `TOBtlSetLuluLimitTbl(digits)`.
    TOBtlSetLuluLimitTbl @ 0x0089_a2f0 => extern "C" fn to_btl_set_lulu_limit_tbl(digits: u32) {
        btl::bridge::set_lulu_limit_table(&mut mem(), digits);
    }
    /// `TOBtlSetTargetCallback(w)`.
    TOBtlSetTargetCallback @ 0x008a_1090 => extern "C" fn to_btl_set_target_callback(w: u32) {
        btl::bridge::set_target_callback(&mut mem(), w);
    }

    /// `TOSeqGetNum(i, name, &n)`.
    TOSeqGetNum @ 0x008a_e410 => extern "C" fn to_seq_get_num(i: i32, name: u32, out: u32) {
        scene::bridge::get_num(&mut mem(), i, name, out, menu_scene::tk_menu_get_param_handle, menu_scene::tk_menu_get_param_offset);
    }
    /// `TOSeqGetColor(i, name, &rgba)`.
    TOSeqGetColor @ 0x008a_e360 => extern "C" fn to_seq_get_color(i: i32, name: u32, out: u32) {
        scene::bridge::get_color(&mut mem(), i, name, out, menu_scene::tk_menu_get_param_handle, menu_scene::tk_menu_get_param_offset);
    }
    /// `TOSeqGetPoint(i, name, &x, &y)`.
    TOSeqGetPoint @ 0x008a_e450 => extern "C" fn to_seq_get_point(i: i32, name: u32, x: u32, y: u32) {
        scene::bridge::get_point(&mut mem(), i, name, [x, y], menu_scene::tk_menu_get_param_handle, menu_scene::tk_menu_get_param_offset);
    }
    /// `TOSeqGetRect(i, name, &x, &y, &w, &h)`.
    TOSeqGetRect @ 0x008a_e4b0 => extern "C" fn to_seq_get_rect(i: i32, name: u32, x: u32, y: u32, w: u32, h: u32) {
        scene::bridge::get_rect(&mut mem(), i, name, [x, y, w, h], menu_scene::tk_menu_get_param_handle, menu_scene::tk_menu_get_param_offset);
    }

    /// `TOMenuGetFirstReadEndFlag()`.
    TOMenuGetFirstReadEndFlag @ 0x008a_8e50 => extern "C" fn to_menu_get_first_read_end_flag() -> i32 {
        menu::bridge::first_read_end(&mem())
    }
    /// `TOMenuHelpSetExtMode(mode, arg)`.
    TOMenuHelpSetExtMode @ 0x008b_ef10 => extern "C" fn to_menu_help_set_ext_mode(mode: u32, arg: u32) {
        menu::bridge::help_set_ext_mode(&mut mem(), mode, arg);
    }
    /// `TOMenuGetAbilitymapWorkAdrs()`.
    TOMenuGetAbilitymapWorkAdrs @ 0x008b_ea30 => extern "C" fn to_menu_get_abilitymap_work_adrs() -> u32 {
        menu::bridge::abilitymap_work(&mem())
    }
    /// `TOAmapGetItemListMax()`.
    TOAmapGetItemListMax @ 0x008c_8c30 => extern "C" fn to_amap_get_item_list_max() -> u32 {
        menu::bridge::amap_item_list_max(&mem())
    }
    /// `TOAmapGetItemType(i)`.
    TOAmapGetItemType @ 0x008c_6710 => extern "C" fn to_amap_get_item_type(i: i32) -> u32 {
        u32::from(menu::bridge::amap_item_type(&mem(), i))
    }
    /// `IsInTkMenuPause()`.
    IsInTkMenuPause @ 0x008a_8e10 => extern "C" fn is_in_tk_menu_pause() -> u32 {
        menu::bridge::is_in_pause(&mem())
    }
    /// `SetInvokedFromSelectCursor(v)`.
    SetInvokedFromSelectCursor @ 0x008e_89c0 => extern "C" fn set_invoked_from_select_cursor(v: u32) {
        menu::bridge::set_invoked_from_select_cursor(&mut mem(), v);
    }
    /// `TOGetMessageWindowMode()`.
    TOGetMessageWindowMode @ 0x008a_c7b0 => extern "C" fn to_get_message_window_mode() -> u32 {
        menu::bridge::message_window_mode(&mem())
    }
    /// `TOGetFFXToAlbhedTblAdrs()`.
    TOGetFFXToAlbhedTblAdrs @ 0x0090_6b20 => extern "C" fn to_get_ffx_to_albhed_tbl_adrs() -> u32 {
        menu::bridge::albhed_table(&mem())
    }
    /// `TOGetFontWidthTblAdrs(font)`.
    TOGetFontWidthTblAdrs @ 0x008a_c3d0 => extern "C" fn to_get_font_width_tbl_adrs(font: i32) -> u32 {
        menu::bridge::font_width_table(&mem(), font)
    }
    /// `TOGetLineFeedHeight()`: 16.
    TOGetLineFeedHeight @ 0x008b_7c60 => extern "C" fn to_get_line_feed_height() -> u32 {
        stub::LINE_FEED_HEIGHT
    }
    /// `TOFFXToSJIS(text, out)`.
    TOFFXToSJIS @ 0x008b_6200 => extern "C" fn to_ffx_to_sjis(_text: u32, out: u32) {
        menu::bridge::ffx_to_sjis(&mut mem(), out);
    }
    /// `TkMenuGetImportantName(id)`.
    TkMenuGetImportantName @ 0x008d_dee0 => extern "C" fn to_menu_get_get_important_name(id: u32) -> u32 {
        important_text(id, 0)
    }
    /// `TkMenuGetImportantHelp(id)`.
    TkMenuGetImportantHelp @ 0x008d_dea0 => extern "C" fn to_menu_get_get_important_help(id: u32) -> u32 {
        important_text(id, 8)
    }

    /// `TOInitAddPkt()`.
    TOInitAddPkt @ 0x0090_4650 => extern "C" fn to_init_add_pkt() {
        pkt::bridge::init_add_pkt(&mut mem());
    }
    /// `TOSetAddPkt()`.
    TOSetAddPkt @ 0x008f_ae10 => extern "C" fn to_set_add_pkt() {
        pkt::bridge::set_add_pkt(&mut mem());
    }
    /// `TOSetPktBuffAdrs(base, a, b)`.
    TOSetPktBuffAdrs @ 0x0090_47c0 => extern "C" fn to_set_pkt_buff_adrs(base: u32, a: u32, b: u32) {
        pkt::bridge::set_pkt_buff(&mut mem(), base, a, b);
    }
    /// `TOGetScreenWH(&w, &h)`.
    TOGetScreenWH @ 0x008f_a900 => extern "C" fn to_get_screen_wh(w: u32, h: u32) {
        pkt::bridge::screen_wh(&mut mem(), w, h);
    }

    /// `Fixup_ReadLeb128(&cursor)`.
    Fixup_ReadLeb128 @ 0x0060_ac00 => extern "C" fn fixup_read_leb128(cursor: u32) -> u32 {
        fixup::bridge::read_leb128(&mut mem(), cursor)
    }
    /// `Fixup_SetObject(record, index)`.
    Fixup_SetObject @ 0x0060_ae40 => extern "C" fn fixup_set_object(record: u32, index: u32) {
        fixup::bridge::set_object(&mut mem(), record, index);
    }
    /// `Fixup_SetArrayObject(record, index)`.
    Fixup_SetArrayObject @ 0x0060_ae30 => extern "C" fn fixup_set_array_object(record: u32, index: u32) {
        fixup::bridge::set_object(&mut mem(), record, index);
    }
    /// `Md5Digest_Less(b)`.
    Md5Digest_Less @ 0x0061_d6e0 => extern "thiscall" fn md5_digest_less(this: u32, b: u32) -> bool {
        vbf::bridge::less(&mem(), this, b)
    }
    /// `Vbf_MapLookup(digest)`.
    Vbf_MapLookup @ 0x0061_d140 => extern "thiscall" fn vbf_map_lookup(this: u32, digest: u32) -> u32 {
        vbf::bridge::map_lookup(&mem(), this, digest)
    }

    /// `AssetListReader::getAssetPath(i)`.
    AssetListReader_getAssetPath @ 0x006f_41b0 => extern "thiscall" fn asset_list_reader_get_asset_path(this: u32, i: i32) -> u32 {
        instance::bridge::asset_path(&mem(), this, i)
    }
    /// `AssetListReader::isModel(id)`.
    AssetListReader_isModel @ 0x006f_46b0 => extern "thiscall" fn asset_list_reader_is_model(this: u32, id: u32) -> bool {
        instance::bridge::is_model(&mem(), this, id)
    }
    /// `AssetListReader::getSymbolById(id)`.
    AssetListReader_getSymbolById @ 0x006f_42d0 => extern "thiscall" fn asset_list_reader_get_symbol_by_id(this: u32, id: u32) -> u32 {
        instance::bridge::symbol_by_id(&mem(), this, id)
    }
    /// `AbmapManager::ToDestroy()`.
    AbmapManager_ToDestroy @ 0x0068_57b0 => extern "thiscall" fn abmap_manager_to_destroy(this: u32) {
        instance::bridge::abmap_to_destroy(&mut mem(), this);
    }
    /// `ChrResourceCacheManager::isChrDelayedRelease()`.
    ChrResourceCacheManager_isChrDelayedRelease @ 0x0069_b7b0 => extern "thiscall" fn chr_resource_cache_manager_is_chr_delayed_release(this: u32) -> u32 {
        instance::bridge::is_chr_delayed_release(&mem(), this)
    }
    /// `AsyncLoadNewCharacter()`.
    AsyncLoadNewCharacter @ 0x0078_2f80 => extern "C" fn async_load_new_character() -> u32 {
        instance::bridge::async_load_new_character(&mut mem())
    }
    /// `LoadingScreen::setParameterManually(&a, &b, &c)`.
    LoadingScreen_setParameterManually @ 0x006e_4ff0 => extern "thiscall" fn loading_screen_set_parameter_manually(this: u32, a: u32, b: u32, c: u32) {
        instance::bridge::loading_screen_set_parameter(&mut mem(), this, a, b, c);
    }
    /// `abmCalcChrMoveCurMoveCheck(chr)`.
    abmCalcChrMoveCurMoveCheck @ 0x00a4_5000 => extern "C" fn abm_calc_chr_move_cur_move_check(chr: u32) -> u32 {
        instance::bridge::chr_move_cur_move_check(&mem(), chr)
    }

    /// `useExtendedAlpha(on)`.
    useExtendedAlpha @ 0x0071_e460 => extern "C" fn use_extended_alpha(on: u32) {
        render::bridge::use_extended_alpha(&mut mem(), on);
    }
    /// `useShaderPointLight()`.
    useShaderPointLight @ 0x0071_e6f0 => extern "C" fn use_shader_point_light() -> u32 {
        render::bridge::use_shader_point_light(&mem())
    }
    /// `DepthBiasHooks::clearDepthBias()`.
    DepthBiasHooks_clearDepthBias @ 0x006e_40b0 => extern "C" fn depth_bias_hooks_clear_depth_bias() {
        render::bridge::clear_depth_bias(&mut mem());
    }
    /// `JobSchedule::pppPart::clearFillBufferRequest()`.
    JobSchedule_pppPart_clearFillBufferRequest @ 0x0071_ecd0 => extern "C" fn job_schedule_ppp_part_clear_fill_buffer_request() {
        render::bridge::clear_fill_buffer_request(&mut mem());
    }

    /// `SaveDataIsBusy()`.
    SaveDataIsBusy @ 0x0064_8040 => extern "C" fn save_data_is_busy() -> u32 {
        save::bridge::is_busy_now(&mem())
    }
    /// `SaveDataSetScreenState(state)`.
    SaveDataSetScreenState @ 0x0064_86e0 => extern "C" fn save_data_set_screen_state(state: u32) {
        save::bridge::set_screen_state(&mut mem(), state);
    }
    /// `SaveDataIsOperationCanceled()`.
    SaveDataIsOperationCanceled @ 0x0064_80c0 => extern "C" fn save_data_is_operation_canceled() -> u32 {
        save::bridge::is_operation_canceled(&mem())
    }
    /// `SaveDataIsLoadingAlbhed()`.
    SaveDataIsLoadingAlbhed @ 0x0064_8070 => extern "C" fn save_data_is_loading_albhed() -> u32 {
        save::bridge::is_loading_albhed(&mem())
    }
    /// `SaveDataSetAlbhedFade(v)`.
    SaveDataSetAlbhedFade @ 0x0064_85e0 => extern "C" fn save_data_set_albhed_fade(v: u32) {
        save::bridge::set_albhed_fade(&mut mem(), v);
    }
    /// `SetLocalDataSlotAndTime(_, slot, a, b)` (slot 16-bit).
    SetLocalDataSlotAndTime @ 0x0064_8fa0 => extern "C" fn set_local_data_slot_and_time(_unused: u32, slot: u32, a: u32, b: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores CX
        save::bridge::set_local_data_slot_and_time(&mut mem(), slot as u16, a, b);
    }
    /// `SaveDataGetCrc(save)`.
    SaveDataGetCrc @ 0x0064_7e80 => extern "C" fn save_data_get_crc(save_buf: u32) -> u32 {
        u32::from(save::bridge::get_crc(&mut mem(), save_buf, ffx_menu::tk::bridge::tk_crc))
    }
    /// `SaveDataWriteCrc(save)`.
    SaveDataWriteCrc @ 0x0064_8f20 => extern "C" fn save_data_write_crc(save_buf: u32) -> u32 {
        save::bridge::write_crc(&mut mem(), save_buf, ffx_menu::tk::bridge::tk_crc)
    }
    /// `SaveDataCheckCrc()`.
    SaveDataCheckCrc @ 0x0064_7d70 => extern "C" fn save_data_check_crc() -> u32 {
        save::bridge::check_crc(&mut mem(), ffx_menu::tk::bridge::tk_crc)
    }
    /// `Virtuos::InputManager::getInstance()`.
    Virtuos_InputManager_getInstance @ 0x0063_1300 => extern "C" fn virtuos_input_manager_get_instance() -> u32 {
        instance::bridge::input_manager(&mem())
    }
    /// `Virtuos::InputManager::getVibrationFlag()` (`this` in ECX).
    Virtuos_InputManager_getVibrationFlag @ 0x0063_1320 => extern "fastcall" fn virtuos_input_manager_get_vibration_flag(this: u32) -> bool {
        instance::bridge::vibration_flag(&mem(), this)
    }
    /// `Input_KeyDown(key, pad)` (`this` the input manager).
    Input_KeyDown @ 0x0062_9ca0 => extern "thiscall" fn input_key_down(this: u32, key: u32, pad: u32) -> u32 {
        instance::bridge::key_down(&mem(), this, key, pad)
    }
    /// `Instance_006dc800_Get()`.
    Instance_006dc800_Get @ 0x006d_c8a0 => extern "C" fn instance_006dc800_get() -> u32 {
        instance::bridge::morph_manager(&mem())
    }
    /// `TrophyManager::Instance()`.
    TrophyManager_Instance @ 0x0067_ac90 => extern "C" fn trophy_manager_instance() -> u32 {
        instance::bridge::trophy_manager(&mem())
    }
    /// `FFXGetVideoMode()`.
    FFXGetVideoMode @ 0x0088_7c40 => extern "C" fn ffx_get_video_mode() -> i32 {
        config::bridge::ffx_video_mode(&mem())
    }
    /// `sceGsSetDefDrawEnv2(...)`.
    sceGsSetDefDrawEnv2 @ 0x0088_7c60 => extern "C" fn sce_gs_set_def_draw_env2() -> i32 {
        config::bridge::gs_set_def_draw_env2(&mem())
    }
    /// `MsGetSavePlayerPtr_thunk(chr)`.
    MsGetSavePlayerPtr_thunk @ 0x0077_6f50 => extern "C" fn ms_get_save_player_ptr_thunk(chr: u32) -> u32 {
        names::bridge::save_player_ptr(&mem(), chr)
    }
    /// `hdd_state_clear()`.
    hdd_state_clear @ 0x0077_7420 => extern "C" fn hdd_state_clear() {
        cd::bridge::hdd_state_clear(&mut mem());
    }
    /// `hdd_set_pass_names()`.
    hdd_set_pass_names @ 0x0077_7940 => extern "C" fn hdd_set_pass_names() {
        cd::bridge::hdd_set_pass_names(&mut mem());
    }
    /// `get_hdd_err()`.
    get_hdd_err @ 0x0077_7e80 => extern "C" fn get_hdd_err() -> i32 {
        cd::bridge::hdd_err(&mem())
    }
    /// `get_hdd_status()`.
    get_hdd_status @ 0x0077_7e90 => extern "C" fn get_hdd_status() -> i32 {
        cd::bridge::hdd_status(&mem())
    }
    /// `TOCloseMessageWindow(i)`.
    TOCloseMessageWindow @ 0x008a_b8f0 => extern "C" fn to_close_message_window(i: i32) {
        window::bridge::close_message_window(&mut mem(), i);
    }
    /// `TOSetMessageWindowActive(i)`.
    TOSetMessageWindowActive @ 0x008a_f7d0 => extern "C" fn to_set_message_window_active(i: i32) -> u32 {
        window::bridge::set_message_window_active(&mut mem(), i)
    }
    /// `TOSetMessageWindowValue(i, v)`.
    TOSetMessageWindowValue @ 0x008a_fb90 => extern "C" fn to_set_message_window_value(i: i32, v: u32) -> u32 {
        window::bridge::set_message_window_value(&mut mem(), i, v)
    }
    /// `TODrawScissor(x0, y0, x1, y1)`.
    TODrawScissor @ 0x008f_91f0 => extern "C" fn to_draw_scissor(x0: i32, y0: i32, x1: i32, y1: i32) {
        pkt::bridge::make_pkt_scissor(&mut mem(), x0, y0, x1, y1);
    }
    /// `TOMakePktScissor(x0, y0, x1, y1)`.
    TOMakePktScissor @ 0x008f_dee0 => extern "C" fn to_make_pkt_scissor(x0: i32, y0: i32, x1: i32, y1: i32) {
        pkt::bridge::make_pkt_scissor(&mut mem(), x0, y0, x1, y1);
    }
    /// `TOGetGsRegWord()`: a constant in `edx:eax`.
    TOGetGsRegWord @ 0x008f_a7b0 => extern "C" fn to_get_gs_reg_word() -> u64 {
        pkt::bridge::GS_WORD_008FA7B0
    }
    /// `TouchManager::GetInstance()`.
    TouchManager_GetInstance @ 0x0068_98c0 => extern "C" fn touch_manager_get_instance() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::TOUCH_MANAGER)
    }
    /// `AchievementManager_GetInstance()`.
    AchievementManager_GetInstance @ 0x006f_15b0 => extern "C" fn achievement_manager_get_instance() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::ACHIEVEMENT_MANAGER)
    }
    /// `MagicFile_GetToBeDeleteMagicId()`.
    MagicFile_GetToBeDeleteMagicId @ 0x009d_a3d0 => extern "C" fn magic_file_get_to_be_delete_magic_id() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::MAGIC_TO_DELETE)
    }
    /// `get_hdd_word_518()`.
    get_hdd_word_518 @ 0x0077_87b0 => extern "C" fn get_hdd_word_518() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::HDD_WORD_518)
    }
    /// `TOGetWord_01841e08()`.
    TOGetWord_01841e08 @ 0x008a_c2d0 => extern "C" fn to_get_word_01841e08() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::TO_GET_WORDS[0])
    }
    /// `TOGetWord_01841d98()`.
    TOGetWord_01841d98 @ 0x008a_c3f0 => extern "C" fn to_get_word_01841d98() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::TO_GET_WORDS[1])
    }
    /// `TOGetWord_01841e00()`.
    TOGetWord_01841e00 @ 0x008a_c790 => extern "C" fn to_get_word_01841e00() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::TO_GET_WORDS[2])
    }
    /// `TOGetWord_01841d9c()`.
    TOGetWord_01841d9c @ 0x008a_cf10 => extern "C" fn to_get_word_01841d9c() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::TO_GET_WORDS[3])
    }
    /// `Table60_Get(i)` (`this` in ECX).
    Table60_Get @ 0x0063_11b0 => extern "thiscall" fn table60_get(this: u32, i: u32) -> u32 {
        instance::bridge::table_60(&mem(), this, i)
    }
    /// `EighthAtLeastOne(n, _)`.
    EighthAtLeastOne @ 0x006d_b870 => extern "stdcall" fn eighth_at_least_one(n: i32, _unused: u32) -> i32 {
        instance::bridge::eighth_at_least_one(n)
    }
    /// `TOSetMessageWindowMode(v)`.
    TOSetMessageWindowMode @ 0x008a_f840 => extern "C" fn to_set_message_window_mode(v: u32) -> u32 {
        window::bridge::set_message_window_mode(&mut mem(), v)
    }
    /// `TOIsMessageStatus3Pressed(i)`.
    TOIsMessageStatus3Pressed @ 0x008b_8530 => extern "C" fn to_is_message_status3_pressed(i: i32) -> u32 {
        window::bridge::is_message_status3_pressed(&mem(), i)
    }
    /// `TOMenuSet_018762ee(i, v)`.
    TOMenuSet_018762ee @ 0x0090_0f60 => extern "C" fn to_menu_set_018762ee(i: i32, v: u32) {
        window::bridge::menu_set_018762ee(&mut mem(), i, v);
    }
    /// `user_free(p)`: the original leaves `LeaveCriticalSection`'s `eax`,
    /// which is `RtlLeaveCriticalSection`'s `STATUS_SUCCESS`, 0 (what
    /// `tools/check-clobber.py` sees read after `0x007783cb` is the end of a
    /// function body Ghidra cut short; the caller clears `eax`).
    user_free @ 0x006f_b8b0 => extern "C" fn user_free(p: u32) -> u32 {
        heap::user_free(&mut mem(), &mut GameOs, p);
        0
    }
    /// `UserHeap_Free(p)` (`this` the allocator).
    UserHeap_Free @ 0x0094_3130 => extern "thiscall" fn user_heap_free(this: u32, p: u32) -> bool {
        heap::heap_free(&mut mem(), &mut GameOs, this, p)
    }
    /// `UserHeap_InsertFree(node, arena)`.
    UserHeap_InsertFree @ 0x0094_34c0 => extern "thiscall" fn user_heap_insert_free(this: u32, node: u32, arena: u32) {
        heap::insert_free(&mut mem(), this, node, arena);
    }
    /// `UserHeap_RemoveFree(node)`: leaves `node` in eax.
    UserHeap_RemoveFree @ 0x0094_35a0 => extern "thiscall" fn user_heap_remove_free(_this: u32, node: u32) -> u32 {
        heap::remove_free(&mut mem(), node);
        node
    }
    /// `UserHeap_BinIndex(size)`: in edx:eax.
    UserHeap_BinIndex @ 0x0094_32e0 => extern "thiscall" fn user_heap_bin_index(this: u32, size: u32) -> i64 {
        heap::bin_index(&mem(), this, size)
    }
    /// `UserHeap_ReleaseArena(arena)`.
    UserHeap_ReleaseArena @ 0x0094_3050 => extern "thiscall" fn user_heap_release_arena(this: u32, arena: u32) {
        heap::release_arena(&mut mem(), &mut GameOs, this, arena);
    }
    /// `UserHeap_NodeAlloc()` (the pool in ECX).
    UserHeap_NodeAlloc @ 0x0094_85b0 => extern "fastcall" fn user_heap_node_alloc(pool: u32) -> u32 {
        heap::node_alloc(&mut mem(), &mut GameOs, pool)
    }
    /// `UserHeap_NodeNewPage()` (the pool in ECX).
    UserHeap_NodeNewPage @ 0x0094_85f0 => extern "fastcall" fn user_heap_node_new_page(pool: u32) -> u32 {
        heap::node_new_page(&mut mem(), &mut GameOs, pool)
    }
    /// `UserHeap_NodeFree(node)` (`this` the pool).
    UserHeap_NodeFree @ 0x0094_86a0 => extern "thiscall" fn user_heap_node_free(this: u32, node: u32) {
        heap::node_free(&mut mem(), &mut GameOs, this, node);
    }
    /// `PageManager_Alloc(size, align)`.
    PageManager_Alloc @ 0x0094_8130 => extern "thiscall" fn page_manager_alloc(this: u32, size: u32, align: u32) -> u32 {
        heap::page_alloc(&mut mem(), &mut GameOs, this, size, align)
    }
    /// `PageAlloc(size, align)`: on `*g_PageManager`.
    PageAlloc @ 0x0094_8110 => extern "C" fn page_alloc(size: u32, align: u32) -> u32 {
        let m = &mut mem();
        let mgr = heap::PAGE_MANAGER.get(m);
        heap::page_alloc(m, &mut GameOs, mgr, size, align)
    }
    /// `PageManager_Commit(addr, size)`.
    PageManager_Commit @ 0x0094_7590 => extern "thiscall" fn page_manager_commit(this: u32, addr: u32, size: u32) -> bool {
        heap::commit(&mut mem(), &mut GameOs, this, addr, size)
    }
    /// `PageCommit(addr, size)`: on `*g_PageManager`.
    PageCommit @ 0x0094_7570 => extern "C" fn page_commit(addr: u32, size: u32) -> bool {
        let m = &mut mem();
        let mgr = heap::PAGE_MANAGER.get(m);
        heap::commit(m, &mut GameOs, mgr, addr, size)
    }
    /// `PageManager_CommitRange(&addr, &size)`.
    PageManager_CommitRange @ 0x0094_83d0 => extern "thiscall" fn page_manager_commit_range(this: u32, pa: u32, ps: u32) -> bool {
        heap::commit_range_at(&mut mem(), &mut GameOs, this, pa, ps)
    }
    /// `PageManager_QueueDecommit(addr, size)`.
    PageManager_QueueDecommit @ 0x0094_7690 => extern "thiscall" fn page_manager_queue_decommit(this: u32, addr: u32, size: u32) {
        heap::queue_decommit(&mut mem(), &mut GameOs, this, addr, size);
    }
    /// `PageQueueDecommit(addr, size)`: on `*g_PageManager`.
    PageQueueDecommit @ 0x0094_7670 => extern "C" fn page_queue_decommit(addr: u32, size: u32) {
        let m = &mut mem();
        let mgr = heap::PAGE_MANAGER.get(m);
        heap::queue_decommit(m, &mut GameOs, mgr, addr, size);
    }
    /// `PageManager_Release(addr)`.
    PageManager_Release @ 0x0094_7ec0 => extern "thiscall" fn page_manager_release(this: u32, addr: u32) {
        heap::release_pages(&mut mem(), &mut GameOs, this, addr);
    }
    /// `PageRelease(addr)`: on `*g_PageManager`.
    PageRelease @ 0x0094_7e80 => extern "C" fn page_release(addr: u32) {
        let m = &mut mem();
        let mgr = heap::PAGE_MANAGER.get(m);
        heap::release_pages(m, &mut GameOs, mgr, addr);
    }
    /// `PageManager_Flush(force)` (a byte).
    PageManager_Flush @ 0x0094_78e0 => extern "thiscall" fn page_manager_flush(this: u32, force: u32) {
        heap::flush(&mut mem(), &mut GameOs, this, force.to_le_bytes()[0]);
    }
    /// `PageHash_Insert(&key, &data)`.
    PageHash_Insert @ 0x0094_7270 => extern "thiscall" fn page_hash_insert(this: u32, key: u32, data: u32) {
        heap::hash_insert_at(&mut mem(), &mut GameOs, this, key, data);
    }
    /// `PageHash_Remove(&out, &key)`: returns `out`.
    PageHash_Remove @ 0x0094_8000 => extern "thiscall" fn page_hash_remove(this: u32, out: u32, key: u32) -> u32 {
        heap::hash_remove_at(&mut mem(), this, out, key)
    }
    /// `PageHash_Rehash(n)`.
    PageHash_Rehash @ 0x0094_82c0 => extern "thiscall" fn page_hash_rehash(this: u32, n: i32) {
        heap::hash_rehash(&mut mem(), &mut GameOs, this, n);
    }
    /// `PageHash_NodeAlloc()` (the table in ECX).
    PageHash_NodeAlloc @ 0x0094_7430 => extern "fastcall" fn page_hash_node_alloc(table: u32) -> u32 {
        heap::alloc_hash_node(&mut mem(), &mut GameOs, table)
    }
    /// `PageManager_QueueNodeAlloc()` (the pool in ECX).
    PageManager_QueueNodeAlloc @ 0x0094_7350 => extern "fastcall" fn page_manager_queue_node_alloc(pool: u32) -> u32 {
        heap::alloc_queue_node(&mut mem(), &mut GameOs, pool)
    }
    /// `PageManager_CacheNodeAlloc()` (the pool in ECX).
    PageManager_CacheNodeAlloc @ 0x0094_73c0 => extern "fastcall" fn page_manager_cache_node_alloc(pool: u32) -> u32 {
        heap::alloc_cache_node(&mut mem(), &mut GameOs, pool)
    }
    /// `VirtualAlloc_Commit_RW(addr, size)`.
    VirtualAlloc_Commit_RW @ 0x0094_38b0 => extern "C" fn virtual_alloc_commit_rw(addr: u32, size: u32) -> u32 {
        heap::commit_rw(&mut mem(), &mut GameOs, addr, size)
    }
    /// `VirtualFree_Decommit(addr, size)`.
    VirtualFree_Decommit @ 0x0094_38e0 => extern "C" fn virtual_free_decommit(addr: u32, size: u32) {
        heap::decommit(&mut mem(), &mut GameOs, addr, size);
    }
    /// `VirtualFree_Release(addr, size)`: returns `size`.
    VirtualFree_Release @ 0x0094_3940 => extern "C" fn virtual_free_release(addr: u32, size: u32) -> u32 {
        heap::release(&mut mem(), &mut GameOs, addr, size)
    }
    /// `VirtualAlloc_Reserve_NA(size)`.
    VirtualAlloc_Reserve_NA @ 0x0094_39a0 => extern "C" fn virtual_alloc_reserve_na(size: u32) -> u32 {
        heap::reserve(&mut mem(), &mut GameOs, size)
    }
    /// `user_malloc(size)`: a jump to `UserHeap_Malloc`.
    user_malloc @ 0x0068_7020 => extern "C" fn user_malloc(size: u32) -> u32 {
        heap::user_malloc(&mut mem(), &mut GameOs, size)
    }
    /// `UserHeap_Malloc(size)`.
    UserHeap_Malloc @ 0x006f_b910 => extern "C" fn user_heap_malloc(size: u32) -> u32 {
        heap::user_malloc(&mut mem(), &mut GameOs, size)
    }
    /// `user_aligned_malloc(size, align)`: a jump to `UserHeap_AlignedMalloc`.
    user_aligned_malloc @ 0x0068_6ff0 => extern "C" fn user_aligned_malloc(size: u32, align: u32) -> u32 {
        heap::user_aligned_malloc(&mut mem(), &mut GameOs, size, align)
    }
    /// `UserHeap_AlignedMalloc(size, align)`.
    UserHeap_AlignedMalloc @ 0x006f_b770 => extern "C" fn user_heap_aligned_malloc(size: u32, align: u32) -> u32 {
        heap::user_aligned_malloc(&mut mem(), &mut GameOs, size, align)
    }
    /// `user_mem_size(p)`: a jump to `UserHeap_MemSize`.
    user_mem_size @ 0x0068_7050 => extern "C" fn user_mem_size(p: u32) -> u32 {
        heap::user_mem_size(&mut mem(), &mut GameOs, p)
    }
    /// `UserHeap_MemSize(p)`.
    UserHeap_MemSize @ 0x006f_ba90 => extern "C" fn user_heap_mem_size(p: u32) -> u32 {
        heap::user_mem_size(&mut mem(), &mut GameOs, p)
    }
    /// `user_realloc(p, size)`: a jump to `UserHeap_Realloc`.
    user_realloc @ 0x0068_7060 => extern "C" fn user_realloc(p: u32, size: u32) -> u32 {
        heap::user_realloc(&mut mem(), &mut GameOs, p, size)
    }
    /// `UserHeap_Realloc(p, size)`.
    UserHeap_Realloc @ 0x006f_bb00 => extern "C" fn user_heap_realloc(p: u32, size: u32) -> u32 {
        heap::user_realloc(&mut mem(), &mut GameOs, p, size)
    }
    /// `UserHeap_Alloc(size)` (`this` the allocator).
    UserHeap_Alloc @ 0x0094_2b60 => extern "thiscall" fn user_heap_alloc(this: u32, size: u32) -> u32 {
        heap::heap_alloc(&mut mem(), &mut GameOs, this, size)
    }
    /// `UserHeap_FindFree(size, &arena)`.
    UserHeap_FindFree @ 0x0094_3380 => extern "thiscall" fn user_heap_find_free(this: u32, size: u32, arena: u32) -> u32 {
        heap::find_free_at(&mut mem(), &mut GameOs, this, size, arena)
    }
    /// `UserHeap_NewArena()` (the allocator in ECX).
    UserHeap_NewArena @ 0x0094_2dc0 => extern "fastcall" fn user_heap_new_arena(this: u32) -> u32 {
        heap::new_arena(&mut mem(), &mut GameOs, this)
    }
    /// `UserHeap_BlockSize(p)`.
    UserHeap_BlockSize @ 0x0094_3480 => extern "thiscall" fn user_heap_block_size(this: u32, p: u32) -> u32 {
        heap::block_size(&mem(), this, p)
    }
    /// `LocalizationManager_GetInstance()`.
    LocalizationManager_GetInstance @ 0x006d_b070 => extern "C" fn localization_manager_get_instance() -> u32 {
        instance::bridge::localization_manager(&mem(), &mut phyre_printf)
    }
    /// `TOGetFFXLang()`.
    TOGetFFXLang @ 0x0064_10e0 => extern "C" fn to_get_ffx_lang() -> u32 {
        instance::bridge::ffx_lang(&mem(), &mut phyre_printf)
    }
    /// `BufferAt8_dtor_00633b70()` (`this` in ECX).
    BufferAt8_dtor_00633b70 @ 0x0063_3b70 => extern "thiscall" fn buffer_at8_dtor_00633b70(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633b80()` (`this` in ECX).
    BufferAt8_dtor_00633b80 @ 0x0063_3b80 => extern "thiscall" fn buffer_at8_dtor_00633b80(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633b90()` (`this` in ECX).
    BufferAt8_dtor_00633b90 @ 0x0063_3b90 => extern "thiscall" fn buffer_at8_dtor_00633b90(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633ba0()` (`this` in ECX).
    BufferAt8_dtor_00633ba0 @ 0x0063_3ba0 => extern "thiscall" fn buffer_at8_dtor_00633ba0(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633bb0()` (`this` in ECX).
    BufferAt8_dtor_00633bb0 @ 0x0063_3bb0 => extern "thiscall" fn buffer_at8_dtor_00633bb0(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633bc0()` (`this` in ECX).
    BufferAt8_dtor_00633bc0 @ 0x0063_3bc0 => extern "thiscall" fn buffer_at8_dtor_00633bc0(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633bd0()` (`this` in ECX).
    BufferAt8_dtor_00633bd0 @ 0x0063_3bd0 => extern "thiscall" fn buffer_at8_dtor_00633bd0(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633bf0()` (`this` in ECX).
    BufferAt8_dtor_00633bf0 @ 0x0063_3bf0 => extern "thiscall" fn buffer_at8_dtor_00633bf0(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633c00()` (`this` in ECX).
    BufferAt8_dtor_00633c00 @ 0x0063_3c00 => extern "thiscall" fn buffer_at8_dtor_00633c00(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633c10()` (`this` in ECX).
    BufferAt8_dtor_00633c10 @ 0x0063_3c10 => extern "thiscall" fn buffer_at8_dtor_00633c10(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633c20()` (`this` in ECX).
    BufferAt8_dtor_00633c20 @ 0x0063_3c20 => extern "thiscall" fn buffer_at8_dtor_00633c20(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633c30()` (`this` in ECX).
    BufferAt8_dtor_00633c30 @ 0x0063_3c30 => extern "thiscall" fn buffer_at8_dtor_00633c30(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00690a20()` (`this` in ECX).
    BufferAt8_dtor_00690a20 @ 0x0069_0a20 => extern "thiscall" fn buffer_at8_dtor_00690a20(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `Identity_008f2f70(x)`.
    Identity_008f2f70 @ 0x008f_2f70 => extern "C" fn identity_008f2f70(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// `Identity_008ed040(x)`.
    Identity_008ed040 @ 0x008e_d040 => extern "C" fn identity_008ed040(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// `TOAdpWords4(p, a, b, c, d)`.
    TOAdpWords4 @ 0x008e_f8e0 => extern "C" fn to_adp_words4(p: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        pkt::bridge::adp_words4(&mut mem(), p, [a, b, c, d])
    }
    /// `TOAdpXY2(p, x, y)`.
    TOAdpXY2 @ 0x008e_fb90 => extern "C" fn to_adp_xy2(p: u32, x: u32, y: u32) -> u32 {
        pkt::bridge::adp_xy2(&mut mem(), p, x, y)
    }
    /// `TOAdpXYZ2Offset(p, x, y)`.
    TOAdpXYZ2Offset @ 0x008e_fbf0 => extern "C" fn to_adp_xyz2_offset(p: u32, x: u32, y: u32) -> u32 {
        pkt::bridge::adp_xyz2_offset(&mut mem(), p, x, y)
    }
    /// `TOAdpGsDirectRGBA(p, rgba)`.
    TOAdpGsDirectRGBA @ 0x008e_f790 => extern "C" fn to_adp_gs_direct_rgba(p: u32, rgba: u32) -> u32 {
        pkt::bridge::adp_gs_direct_rgba(&mut mem(), p, rgba)
    }
    /// `OpenFontScissor(x0, y0, x1, y1)`.
    OpenFontScissor @ 0x008e_8940 => extern "C" fn open_font_scissor(x0: f32, y0: f32, x1: f32, y1: f32) {
        pkt::bridge::open_font_scissor(&mut mem(), [x0, y0, x1, y1]);
    }
    /// `TOGetBtlMenuTexAdrs()`.
    TOGetBtlMenuTexAdrs @ 0x008a_c2c0 => extern "C" fn to_get_btl_menu_tex_adrs() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::BTL_MENU_TEX)
    }
    /// `TOMenuGetYesNoWindow()`.
    TOMenuGetYesNoWindow @ 0x008a_9b60 => extern "C" fn to_menu_get_yes_no_window() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::YES_NO_WINDOW)
    }
    /// `TOMacroSetType(win, n, t)`.
    TOMacroSetType @ 0x008b_8960 => extern "C" fn to_macro_set_type(win: i32, n: i32, t: u32) {
        window::bridge::macro_set_type(&mut mem(), win, n, t.to_le_bytes()[0]);
    }
    /// `TOMacroSetValue(win, n, v)`.
    TOMacroSetValue @ 0x008b_8980 => extern "C" fn to_macro_set_value(win: i32, n: i32, v: u32) -> u32 {
        window::bridge::macro_set_value(&mut mem(), win, n, v)
    }
    /// `TOGetTable_01868a90(i)`.
    TOGetTable_01868a90 @ 0x008b_7c40 => extern "C" fn to_get_table_01868a90(i: i32) -> u32 {
        window::bridge::table_01868a90(&mem(), i)
    }
    /// `TOCloseBtlWindow()`.
    TOCloseBtlWindow @ 0x0089_ade0 => extern "C" fn to_close_btl_window() -> u32 {
        btl::bridge::close_btl_window(&mut mem())
    }
    /// `hdd_set_pass_names_x()`.
    hdd_set_pass_names_x @ 0x0077_9a80 => extern "C" fn hdd_set_pass_names_x() {
        cd::bridge::hdd_set_pass_names_x(&mut mem());
    }
    /// `cd_module_file_sector(m, n)`.
    cd_module_file_sector @ 0x0076_c270 => extern "C" fn cd_module_file_sector(m: i32, n: i32) -> u32 {
        cd::bridge::module_file_sector(&mem(), m, n)
    }
    /// `std::_Uninitialized_copy` of 12-byte elements.
    std_Uninitialized_copy12_0059e830 @ 0x0059_e830 => extern "C" fn uninitialized_copy12(first: u32, last: u32, dest: u32) -> u32 {
        stl::bridge::uninitialized_copy12(&mut mem(), first, last, dest)
    }
    /// `memcmp` as compiled into the game.
    memcmp_00401020 @ 0x0040_1020 => extern "C" fn memcmp_00401020(a: u32, b: u32, n: u32) -> i32 {
        stl::bridge::memcmp(&mem(), a, b, n)
    }
    /// `SetWordAt_00402a00(i, v)` (`this` in ECX).
    SetWordAt_00402a00 @ 0x0040_2a00 => extern "thiscall" fn set_word_at(this: u32, i: u32, v: u32) -> u32 {
        stl::bridge::set_word(&mut mem(), this, i, v)
    }
    /// `MapFindU32_0062fe50(&key)` (`this` in ECX).
    MapFindU32_0062fe50 @ 0x0062_fe50 => extern "thiscall" fn map_find_u32(this: u32, key: u32) -> u32 {
        stl::bridge::map_find_u32(&mem(), this, key)
    }
    /// `cd_channel_clear(n)`.
    cd_channel_clear @ 0x0076_d530 => extern "C" fn cd_channel_clear(n: u32) {
        cd::bridge::cd_channel_clear(&mut mem(), n);
    }
    /// `hdd_install_find(archive, n)`.
    hdd_install_find @ 0x0076_d600 => extern "C" fn hdd_install_find(archive: u32, n: u32) -> u32 {
        cd::bridge::hdd_install_find(&mem(), archive, n)
    }
    /// `TOSetWindowRGBA(i, r, g, b, a)`.
    TOSetWindowRGBA @ 0x0090_6d90 => extern "C" fn to_set_window_rgba(i: i32, r: u32, g: u32, b: u32, a: u32) {
        window::bridge::set_window_rgba(&mut mem(), i, [r, g, b, a].map(|v| v.to_le_bytes()[0]));
    }
    /// `dvd_cd_unk8_lang3()`.
    dvd_cd_unk8_lang3 @ 0x0077_a600 => extern "C" fn dvd_cd_unk8_lang3() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a600)]
    }
    /// `dvd_cd_fat_data_size_lang2()`.
    dvd_cd_fat_data_size_lang2 @ 0x0077_a610 => extern "C" fn dvd_cd_fat_data_size_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a610)]
    }
    /// `dvd_cd_fat_sector_lang2()`.
    dvd_cd_fat_sector_lang2 @ 0x0077_a620 => extern "C" fn dvd_cd_fat_sector_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a620)]
    }
    /// `dvd_cd_fat_size_lang2()`.
    dvd_cd_fat_size_lang2 @ 0x0077_a630 => extern "C" fn dvd_cd_fat_size_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a630)]
    }
    /// `dvd_cd_fid_sector_lang2()`.
    dvd_cd_fid_sector_lang2 @ 0x0077_a640 => extern "C" fn dvd_cd_fid_sector_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a640)]
    }
    /// `dvd_cd_fid_size_lang2()`.
    dvd_cd_fid_size_lang2 @ 0x0077_a650 => extern "C" fn dvd_cd_fid_size_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a650)]
    }
    /// `dvd_cd_id_sector_lang2()`.
    dvd_cd_id_sector_lang2 @ 0x0077_a660 => extern "C" fn dvd_cd_id_sector_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a660)]
    }
    /// `dvd_cd_id_size_lang2()`.
    dvd_cd_id_size_lang2 @ 0x0077_a670 => extern "C" fn dvd_cd_id_size_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a670)]
    }
    /// `dvd_cd_unk7_lang2()`.
    dvd_cd_unk7_lang2 @ 0x0077_a680 => extern "C" fn dvd_cd_unk7_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a680)]
    }
    /// `dvd_cd_unk8_lang2()`.
    dvd_cd_unk8_lang2 @ 0x0077_a690 => extern "C" fn dvd_cd_unk8_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a690)]
    }
    /// `dvd_cd_fat_data_size_lang5()`.
    dvd_cd_fat_data_size_lang5 @ 0x0077_a6a0 => extern "C" fn dvd_cd_fat_data_size_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6a0)]
    }
    /// `dvd_cd_fat_sector_lang5()`.
    dvd_cd_fat_sector_lang5 @ 0x0077_a6b0 => extern "C" fn dvd_cd_fat_sector_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6b0)]
    }
    /// `dvd_cd_fat_size_lang5()`.
    dvd_cd_fat_size_lang5 @ 0x0077_a6c0 => extern "C" fn dvd_cd_fat_size_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6c0)]
    }
    /// `dvd_cd_fid_sector_lang5()`.
    dvd_cd_fid_sector_lang5 @ 0x0077_a6d0 => extern "C" fn dvd_cd_fid_sector_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6d0)]
    }
    /// `dvd_cd_fid_size_lang5()`.
    dvd_cd_fid_size_lang5 @ 0x0077_a6e0 => extern "C" fn dvd_cd_fid_size_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6e0)]
    }
    /// `dvd_cd_id_sector_lang5()`.
    dvd_cd_id_sector_lang5 @ 0x0077_a6f0 => extern "C" fn dvd_cd_id_sector_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6f0)]
    }
    /// `dvd_cd_id_size_lang5()`.
    dvd_cd_id_size_lang5 @ 0x0077_a700 => extern "C" fn dvd_cd_id_size_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a700)]
    }
    /// `dvd_cd_unk7_lang5()`.
    dvd_cd_unk7_lang5 @ 0x0077_a710 => extern "C" fn dvd_cd_unk7_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a710)]
    }
    /// `dvd_cd_unk8_lang5()`.
    dvd_cd_unk8_lang5 @ 0x0077_a720 => extern "C" fn dvd_cd_unk8_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a720)]
    }
    /// `dvd_cd_fat_data_size_lang_other()`.
    dvd_cd_fat_data_size_lang_other @ 0x0077_a730 => extern "C" fn dvd_cd_fat_data_size_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a730)]
    }
    /// `dvd_cd_fat_sector_lang_other()`.
    dvd_cd_fat_sector_lang_other @ 0x0077_a740 => extern "C" fn dvd_cd_fat_sector_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a740)]
    }
    /// `dvd_cd_fat_size_lang_other()`.
    dvd_cd_fat_size_lang_other @ 0x0077_a750 => extern "C" fn dvd_cd_fat_size_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a750)]
    }
    /// `dvd_cd_fid_sector_lang_other()`.
    dvd_cd_fid_sector_lang_other @ 0x0077_a760 => extern "C" fn dvd_cd_fid_sector_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a760)]
    }
    /// `dvd_cd_fid_size_lang_other()`.
    dvd_cd_fid_size_lang_other @ 0x0077_a770 => extern "C" fn dvd_cd_fid_size_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a770)]
    }
    /// `dvd_cd_id_sector_lang_other()`.
    dvd_cd_id_sector_lang_other @ 0x0077_a780 => extern "C" fn dvd_cd_id_sector_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a780)]
    }
    /// `dvd_cd_id_size_lang_other()`.
    dvd_cd_id_size_lang_other @ 0x0077_a790 => extern "C" fn dvd_cd_id_size_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a790)]
    }
    /// `dvd_cd_unk7_lang_other()`.
    dvd_cd_unk7_lang_other @ 0x0077_a7a0 => extern "C" fn dvd_cd_unk7_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7a0)]
    }
    /// `dvd_cd_unk8_lang_other()`.
    dvd_cd_unk8_lang_other @ 0x0077_a7b0 => extern "C" fn dvd_cd_unk8_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7b0)]
    }
    /// `dvd_cd_fat_data_size_lang18()`.
    dvd_cd_fat_data_size_lang18 @ 0x0077_a7c0 => extern "C" fn dvd_cd_fat_data_size_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7c0)]
    }
    /// `dvd_cd_fat_sector_lang18()`.
    dvd_cd_fat_sector_lang18 @ 0x0077_a7d0 => extern "C" fn dvd_cd_fat_sector_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7d0)]
    }
    /// `dvd_cd_fat_size_lang18()`.
    dvd_cd_fat_size_lang18 @ 0x0077_a7e0 => extern "C" fn dvd_cd_fat_size_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7e0)]
    }
    /// `dvd_cd_fid_sector_lang18()`.
    dvd_cd_fid_sector_lang18 @ 0x0077_a7f0 => extern "C" fn dvd_cd_fid_sector_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7f0)]
    }
    /// `dvd_cd_fid_size_lang18()`.
    dvd_cd_fid_size_lang18 @ 0x0077_a800 => extern "C" fn dvd_cd_fid_size_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a800)]
    }
    /// `dvd_cd_id_sector_lang18()`.
    dvd_cd_id_sector_lang18 @ 0x0077_a810 => extern "C" fn dvd_cd_id_sector_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a810)]
    }
    /// `dvd_cd_id_size_lang18()`.
    dvd_cd_id_size_lang18 @ 0x0077_a820 => extern "C" fn dvd_cd_id_size_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a820)]
    }
    /// `dvd_cd_unk7_lang18()`.
    dvd_cd_unk7_lang18 @ 0x0077_a830 => extern "C" fn dvd_cd_unk7_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a830)]
    }
    /// `dvd_cd_unk8_lang18()`.
    dvd_cd_unk8_lang18 @ 0x0077_a840 => extern "C" fn dvd_cd_unk8_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a840)]
    }
    /// `dvd_cd_fat_data_size_english()`.
    dvd_cd_fat_data_size_english @ 0x0077_a850 => extern "C" fn dvd_cd_fat_data_size_english() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a850)]
    }
    /// `dvd_cd_fat_data_size()`: the same for every language.
    dvd_cd_fat_data_size @ 0x0076_d6c0 => extern "C" fn dvd_cd_fat_data_size() -> u32 {
        cd::DVD_CD_LAYOUT[0]
    }
    /// `dvd_cd_fat_sector()`: the same for every language.
    dvd_cd_fat_sector @ 0x0076_d740 => extern "C" fn dvd_cd_fat_sector() -> u32 {
        cd::DVD_CD_LAYOUT[1]
    }
    /// `dvd_cd_fat_size()`: the same for every language.
    dvd_cd_fat_size @ 0x0076_d7c0 => extern "C" fn dvd_cd_fat_size() -> u32 {
        cd::DVD_CD_LAYOUT[2]
    }
    /// `dvd_cd_fid_sector()`: the same for every language.
    dvd_cd_fid_sector @ 0x0076_d840 => extern "C" fn dvd_cd_fid_sector() -> u32 {
        cd::DVD_CD_LAYOUT[3]
    }
    /// `dvd_cd_fid_size()`: the same for every language.
    dvd_cd_fid_size @ 0x0076_d8c0 => extern "C" fn dvd_cd_fid_size() -> u32 {
        cd::DVD_CD_LAYOUT[4]
    }
    /// `dvd_cd_id_sector()`: the same for every language.
    dvd_cd_id_sector @ 0x0076_d940 => extern "C" fn dvd_cd_id_sector() -> u32 {
        cd::DVD_CD_LAYOUT[5]
    }
    /// `dvd_cd_id_size()`: the same for every language.
    dvd_cd_id_size @ 0x0076_d9c0 => extern "C" fn dvd_cd_id_size() -> u32 {
        cd::DVD_CD_LAYOUT[6]
    }
    /// `dvd_cd_unk7()`: the same for every language.
    dvd_cd_unk7 @ 0x0076_da40 => extern "C" fn dvd_cd_unk7() -> u32 {
        cd::DVD_CD_LAYOUT[7]
    }
    /// `dvd_cd_unk8()`: the same for every language.
    dvd_cd_unk8 @ 0x0076_dac0 => extern "C" fn dvd_cd_unk8() -> u32 {
        cd::DVD_CD_LAYOUT[8]
    }
    /// `() -> 0x80`.
    ReturnConst_00405ba0 @ 0x0040_5ba0 => extern "C" fn returnconst_00405ba0() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x0040_5ba0)
    }
    /// `() -> 0x4`.
    ReturnConst_0059e5c0 @ 0x0059_e5c0 => extern "C" fn returnconst_0059e5c0() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x0059_e5c0)
    }
    /// `() -> 0x6900`.
    ReturnConst_00647cb0 @ 0x0064_7cb0 => extern "C" fn returnconst_00647cb0() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x0064_7cb0)
    }
    /// `() -> 0x98`.
    ReturnConst_006f1580 @ 0x006f_1580 => extern "C" fn returnconst_006f1580() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x006f_1580)
    }
    /// `() -> 0x18`.
    ReturnConst_006f1590 @ 0x006f_1590 => extern "C" fn returnconst_006f1590() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x006f_1590)
    }
    /// `() -> 0x10`.
    ReturnConst_006f15a0 @ 0x006f_15a0 => extern "C" fn returnconst_006f15a0() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x006f_15a0)
    }
    /// `() -> 0x640064`.
    ReturnConst_008b7cc0 @ 0x008b_7cc0 => extern "C" fn returnconst_008b7cc0() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x008b_7cc0)
    }
    /// `() -> &0x00b7c79c`.
    ReturnAddress_004013a0 @ 0x0040_13a0 => extern "C" fn returnaddress_004013a0() -> u32 {
        accessor::bridge::address(&mem(), 0x0040_13a0)
    }
    /// `() -> &0x00b7c7b4`.
    ReturnAddress_00401480 @ 0x0040_1480 => extern "C" fn returnaddress_00401480() -> u32 {
        accessor::bridge::address(&mem(), 0x0040_1480)
    }
    /// `() -> &0x00b7c7d8`.
    ReturnAddress_00401530 @ 0x0040_1530 => extern "C" fn returnaddress_00401530() -> u32 {
        accessor::bridge::address(&mem(), 0x0040_1530)
    }
    /// `() -> &0x00b42a34`.
    ReturnAddress_00679a00 @ 0x0067_9a00 => extern "C" fn returnaddress_00679a00() -> u32 {
        accessor::bridge::address(&mem(), 0x0067_9a00)
    }
    /// `() -> &0x00c41748`.
    ReturnAddress_00777230 @ 0x0077_7230 => extern "C" fn returnaddress_00777230() -> u32 {
        accessor::bridge::address(&mem(), 0x0077_7230)
    }
    /// `() -> &0x023cc480`.
    TOMenuGetHelpWorkAdrs @ 0x008b_ec70 => extern "C" fn returnaddress_008bec70() -> u32 {
        accessor::bridge::address(&mem(), 0x008b_ec70)
    }
    /// `() -> *0x01325ac4`.
    GetU32_01325ac4 @ 0x0064_7c90 => extern "C" fn getu32_01325ac4() -> u32 {
        accessor::bridge::global(&mem(), 0x0064_7c90)
    }
    /// `() -> *0x0112a010`.
    GetU32_0112a010 @ 0x0077_4670 => extern "C" fn getu32_0112a010() -> u32 {
        accessor::bridge::global(&mem(), 0x0077_4670)
    }
    /// `() -> *0x01841d30`.
    GetU32_01841d30 @ 0x008a_cf20 => extern "C" fn getu32_01841d30() -> u32 {
        accessor::bridge::global(&mem(), 0x008a_cf20)
    }
    /// `() -> *0x025d16c4`.
    GetU32_025d16c4 @ 0x008b_9130 => extern "C" fn getu32_025d16c4() -> u32 {
        accessor::bridge::global(&mem(), 0x008b_9130)
    }
    /// `() -> *0x00c5a084`.
    GetU32_00c5a084 @ 0x008b_e320 => extern "C" fn getu32_00c5a084() -> u32 {
        accessor::bridge::global(&mem(), 0x008b_e320)
    }
    /// `() -> *0x018763e0`.
    GetU32_018763e0 @ 0x0090_6730 => extern "C" fn getu32_018763e0() -> u32 {
        accessor::bridge::global(&mem(), 0x0090_6730)
    }
    /// `() -> *0x01940ab4`.
    GetU32_01940ab4 @ 0x009d_a2c0 => extern "C" fn getu32_01940ab4() -> u32 {
        accessor::bridge::global(&mem(), 0x009d_a2c0)
    }
    /// `() -> *0x00c64ca4`.
    GetU32_00c64ca4 @ 0x009d_a3a0 => extern "C" fn getu32_00c64ca4() -> u32 {
        accessor::bridge::global(&mem(), 0x009d_a3a0)
    }
    /// `() -> *0x01a85ba8`.
    GetFullArenaLocalization @ 0x00a4_2fa0 => extern "C" fn getfullarenalocalization() -> u32 {
        accessor::bridge::global(&mem(), 0x00a4_2fa0)
    }
    /// `() -> *0x023cbeb0`.
    GetU32_023cbeb0 @ 0x00a7_ae90 => extern "C" fn getu32_023cbeb0() -> u32 {
        accessor::bridge::global(&mem(), 0x00a7_ae90)
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_0061a110 @ 0x0061_a110 => extern "stdcall" fn returnminus1_0061a110(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_006765d0 @ 0x0067_65d0 => extern "stdcall" fn returnminus1_006765d0(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_00677080 @ 0x0067_7080 => extern "stdcall" fn returnminus1_00677080(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_00677c30 @ 0x0067_7c30 => extern "stdcall" fn returnminus1_00677c30(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_00678b50 @ 0x0067_8b50 => extern "stdcall" fn returnminus1_00678b50(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_00a42490 @ 0x00a4_2490 => extern "stdcall" fn returnminus1_00a42490(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// Its argument.
    Identity_0060ab50 @ 0x0060_ab50 => extern "C" fn identity_0060ab50(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060ab60 @ 0x0060_ab60 => extern "C" fn identity_0060ab60(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060b850 @ 0x0060_b850 => extern "C" fn identity_0060b850(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060b860 @ 0x0060_b860 => extern "C" fn identity_0060b860(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c010 @ 0x0060_c010 => extern "C" fn identity_0060c010(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c020 @ 0x0060_c020 => extern "C" fn identity_0060c020(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c700 @ 0x0060_c700 => extern "C" fn identity_0060c700(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c710 @ 0x0060_c710 => extern "C" fn identity_0060c710(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c720 @ 0x0060_c720 => extern "C" fn identity_0060c720(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c730 @ 0x0060_c730 => extern "C" fn identity_0060c730(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c870 @ 0x0060_c870 => extern "C" fn identity_0060c870(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c880 @ 0x0060_c880 => extern "C" fn identity_0060c880(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0061e720 @ 0x0061_e720 => extern "C" fn identity_0061e720(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0061e730 @ 0x0061_e730 => extern "C" fn identity_0061e730(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_006332d0 @ 0x0063_32d0 => extern "C" fn identity_006332d0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_006332e0 @ 0x0063_32e0 => extern "C" fn identity_006332e0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_006332f0 @ 0x0063_32f0 => extern "C" fn identity_006332f0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00633300 @ 0x0063_3300 => extern "C" fn identity_00633300(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00633310 @ 0x0063_3310 => extern "C" fn identity_00633310(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00633320 @ 0x0063_3320 => extern "C" fn identity_00633320(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00633330 @ 0x0063_3330 => extern "C" fn identity_00633330(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00686cb0 @ 0x0068_6cb0 => extern "C" fn identity_00686cb0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00686f80 @ 0x0068_6f80 => extern "C" fn identity_00686f80(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0069a3d0 @ 0x0069_a3d0 => extern "C" fn identity_0069a3d0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0069bda0 @ 0x0069_bda0 => extern "C" fn identity_0069bda0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_006f4870 @ 0x006f_4870 => extern "C" fn identity_006f4870(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_006f4880 @ 0x006f_4880 => extern "C" fn identity_006f4880(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_008ed020 @ 0x008e_d020 => extern "C" fn identity_008ed020(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_008ed030 @ 0x008e_d030 => extern "C" fn identity_008ed030(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_008ed050 @ 0x008e_d050 => extern "C" fn identity_008ed050(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00a41900 @ 0x00a4_1900 => extern "C" fn identity_00a41900(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00a41910 @ 0x00a4_1910 => extern "C" fn identity_00a41910(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_00686c50 @ 0x0068_6c50 => extern "stdcall" fn identitystd_00686c50(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_0069a350 @ 0x0069_a350 => extern "stdcall" fn identitystd_0069a350(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_0069a360 @ 0x0069_a360 => extern "stdcall" fn identitystd_0069a360(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_0069a390 @ 0x0069_a390 => extern "stdcall" fn identitystd_0069a390(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_00a41850 @ 0x00a4_1850 => extern "stdcall" fn identitystd_00a41850(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_00a41860 @ 0x00a4_1860 => extern "stdcall" fn identitystd_00a41860(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_00a41870 @ 0x00a4_1870 => extern "stdcall" fn identitystd_00a41870(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_00a418f0 @ 0x00a4_18f0 => extern "stdcall" fn identitystd_00a418f0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its second argument.
    Second_00401000 @ 0x0040_1000 => extern "C" fn second_00401000(a: u32, b: u32) -> u32 {
        accessor::second(a, b)
    }
    /// Its argument's low byte (only `al` set).
    IdentityU8_0060ab40 @ 0x0060_ab40 => extern "C" fn identityu8_0060ab40(x: u8) -> u8 {
        accessor::first_u8(x)
    }
    /// Its argument's low half (only `ax` set).
    IdentityU16_0061e710 @ 0x0061_e710 => extern "C" fn identityu16_0061e710(x: u16) -> u16 {
        accessor::first_u16(x)
    }
    /// Its argument as a `u64`.
    ZeroExtend64_0060ab70 @ 0x0060_ab70 => extern "C" fn zeroextend64_0060ab70(x: u32) -> u64 {
        accessor::zero_extend(x)
    }
    /// `a + b`.
    Add_0060ab80 @ 0x0060_ab80 => extern "C" fn add_0060ab80(a: u32, b: u32) -> u32 {
        accessor::add(a, b)
    }
    /// `a + b`.
    Add_0061d110 @ 0x0061_d110 => extern "C" fn add_0061d110(a: u32, b: u32) -> u32 {
        accessor::add(a, b)
    }
    /// `b - a`.
    Difference_0061e6e0 @ 0x0061_e6e0 => extern "C" fn difference_0061e6e0(a: u32, b: u32) -> u32 {
        accessor::difference(a, b)
    }
    /// `() -> 0x18` (`ret 4`).
    ReturnConstStd_006770b0 @ 0x0067_70b0 => extern "stdcall" fn returnconststd_006770b0(_a: u32) -> u32 {
        accessor::SIZE_0X18
    }
    /// Stores `Phyre::PInputs::PInputSource::vftable` in `this`.
    PInputSource_setVftable @ 0x009e_24f0 => extern "thiscall" fn pinputsource_setvftable(this: u32) {
        accessor::bridge::set_vftable(&mut mem(), this, accessor::bridge::PINPUT_SOURCE_VFTABLE);
    }
    /// Stores `Phyre::PWorldMatrixController::vftable` in `this`.
    PWorldMatrixController_setVftable @ 0x0061_a730 => extern "thiscall" fn pworldmatrixcontroller_setvftable(this: u32) {
        accessor::bridge::set_vftable(&mut mem(), this, accessor::bridge::PWORLD_MATRIX_CONTROLLER_VFTABLE);
    }
    /// `this[0] = 1`.
    SetWord0_006f1db0 @ 0x006f_1db0 => extern "thiscall" fn setword0_006f1db0(this: u32) {
        accessor::bridge::set_word0(&mut mem(), this);
    }
    /// Byte `+0x364` of `this` set.
    SetByte364_006f1dc0 @ 0x006f_1dc0 => extern "thiscall" fn setbyte364_006f1dc0(this: u32) {
        accessor::bridge::set_byte364(&mut mem(), this);
    }
    /// Whether the state (`+8`) is 6.
    SaveDataManager_isDeletingFinished @ 0x006f_20d0 => extern "thiscall" fn savedatamanager_isdeletingfinished(this: u32) -> u32 {
        accessor::bridge::state_is(&mem(), this, 6)
    }
    /// Whether the state (`+8`) is 4.
    SaveDataManager_isLoadingFinished @ 0x006f_2150 => extern "thiscall" fn savedatamanager_isloadingfinished(this: u32) -> u32 {
        accessor::bridge::state_is(&mem(), this, 4)
    }
    /// Whether the state (`+8`) is 2.
    SaveDataManager_isSavingFinished @ 0x006f_21a0 => extern "thiscall" fn savedatamanager_issavingfinished(this: u32) -> u32 {
        accessor::bridge::state_is(&mem(), this, 2)
    }
    /// The 64-bit size at `+8` of `*(this + 0xc)`.
    PStreamFile_entrySize64 @ 0x0061_acb0 => extern "thiscall" fn pstreamfile_entrysize64(this: u32) -> u64 {
        accessor::bridge::entry_size64(&mem(), this)
    }
    /// `this + 0x8c`.
    Virtuos_InputManager_getStickInfo @ 0x0063_1310 => extern "thiscall" fn getstickinfo_00631310(this: u32) -> u32 {
        accessor::bridge::stick_info(this)
    }
    /// Word `+0x64` of `g_TkMenuWorldMapWindow`.
    TkMenuWorldMapWindowWord64 @ 0x008e_2d50 => extern "C" fn tkmenuworldmapwindowword64() -> u32 {
        accessor::bridge::world_map_window_word64(&mem())
    }
    /// `TODapsInit()`.
    TODapsInit @ 0x0089_dc40 => extern "C" fn to_daps_init() -> u32 {
        daps::bridge::init(&mut mem())
    }
    /// `TODapsDraw()`: the same code as `TODapsInit`.
    TODapsDraw @ 0x0089_dc10 => extern "C" fn to_daps_draw() -> u32 {
        daps::bridge::init(&mut mem())
    }
    /// `TODapsString(s)`.
    TODapsString @ 0x0089_dec0 => extern "C" fn to_daps_string(s: u32) -> u32 {
        daps::bridge::string(&mut mem(), s)
    }
    /// `TODapsSetPos(x, y)`.
    TODapsSetPos @ 0x0089_de30 => extern "C" fn to_daps_set_pos(x: i32, y: i32) -> u32 {
        daps::bridge::set_pos(&mut mem(), x, y)
    }
    /// `TOAdpTEST(p, ate, atst, aref, afail, date, datm, zte, ztst)`.
    TOAdpTEST @ 0x008e_f970 => extern "C" fn to_adp_test(p: u32, ate: i32, atst: i32, aref: i32, afail: i32, date: i32, datm: i32, zte: i32, ztst: i32) -> u32 {
        pkt::bridge::adp_test(&mut mem(), p, [ate, atst, aref, afail, date, datm, zte, ztst])
    }
    /// `TouchRecords_Init(base, n)`.
    TouchRecords_Init @ 0x0068_9a10 => extern "C" fn touch_records_init(base: u32, n: u32) {
        pad::bridge::touch_records_init(&mut mem(), base, n);
    }
    /// `std::map<u32>::equal_range(&out, &key)` (`this` in ECX).
    std_map_u32_equal_range_0069a250 @ 0x0069_a250 => extern "thiscall" fn map_u32_equal_range(this: u32, out: u32, key: u32) -> u32 {
        stl::bridge::equal_range_u32(&mut mem(), this, out, key)
    }
    /// `TOFontGlyphSprite(s, out)`.
    TOFontGlyphSprite @ 0x0089_e360 => extern "C" fn to_font_glyph_sprite(s: u32, out: u32) -> u32 {
        msg::bridge::glyph_sprite(&mut mem(), s, out)
    }
    /// `TOSeqCheckTermScene(i, name)`.
    TOSeqCheckTermScene @ 0x008a_e070 => extern "C" fn to_seq_check_term_scene(i: i32, name: u32) -> u32 {
        scene::bridge::check_term_scene(&mut mem(), i, name, menu_scene::tk_menu_is_term_scene)
    }
    /// `Fixup_ReadRecord(&cursor, flags)` (the record in ECX).
    Fixup_ReadRecord @ 0x0060_e730 => extern "thiscall" fn fixup_read_record(record: u32, cursor: u32, flags: u32) -> u32 {
        fixup::bridge::read_record(&mut mem(), record, cursor, flags)
    }
    /// `SortSaveTable_006f0fb0()`.
    SortSaveTable_006f0fb0 @ 0x006f_0fb0 => extern "C" fn sort_save_table() -> u32 {
        save::bridge::sort_table(&mut mem())
    }
    /// `ResetRecords_00774740()`.
    ResetRecords_00774740 @ 0x0077_4740 => extern "C" fn reset_records_00774740() -> u32 {
        instance::bridge::reset_records(&mut mem())
    }
    /// `TOSeqStartScene(i, name)`.
    TOSeqStartScene @ 0x008a_f0d0 => extern "C" fn to_seq_start_scene(i: i32, name: u32) -> u32 {
        scene::bridge::start_scene(&mut mem(), i, name, false, menu_scene::tk_menu_get_scene_handle)
    }
    /// `TOSeqStartSceneDrawContinue(i, name)`.
    TOSeqStartSceneDrawContinue @ 0x008a_f1b0 => extern "C" fn to_seq_start_scene_draw_continue(i: i32, name: u32) -> u32 {
        scene::bridge::start_scene(&mut mem(), i, name, true, menu_scene::tk_menu_get_scene_handle)
    }
    /// `TOSeqStopScene(i, name)`.
    TOSeqStopScene @ 0x008a_f4f0 => extern "C" fn to_seq_stop_scene(i: i32, name: u32) -> u32 {
        scene::bridge::stop_scene(&mut mem(), i, name)
    }
    /// `TOSeqInitAllScene(i)`.
    TOSeqInitAllScene @ 0x008a_e5a0 => extern "C" fn to_seq_init_all_scene(i: i32) -> u32 {
        scene::bridge::init_all_scene(&mut mem(), i)
    }
    /// `TOBtlSetPrevAction()`.
    TOBtlSetPrevAction @ 0x0089_a630 => extern "C" fn to_btl_set_prev_action() -> u32 {
        btl::bridge::set_prev_action(&mut mem())
    }
    /// `JobSchedule_pppPart_getFillBufferRequest(size)`.
    JobSchedule_pppPart_getFillBufferRequest @ 0x0072_8660 => extern "C" fn get_fill_buffer_request(size: i32) -> u32 {
        render::bridge::get_fill_buffer_request(&mut mem(), size)
    }
    /// `TouchTracks_Init(base, n)`.
    TouchTracks_Init @ 0x0068_98d0 => extern "C" fn touch_tracks_init(base: u32, n: u32) -> u32 {
        pad::bridge::touch_tracks_init(&mut mem(), base, n)
    }
    /// `std_map_cstr_find_006dc010(&key)` (`this` in ECX).
    std_map_cstr_find_006dc010 @ 0x006d_c010 => extern "thiscall" fn map_cstr_find(this: u32, key: u32) -> u32 {
        stl::bridge::map_find_cstr(&mem(), this, key)
    }
    /// `GetTable_00c8a538(i)` (`ret 4`).
    GetTable_00c8a538 @ 0x0040_2be0 => extern "stdcall" fn get_table_00c8a538(i: u32) -> u32 {
        instance::bridge::table_00c8a538(&mem(), i)
    }
    /// `LocalizationManager_GetWord0()`.
    LocalizationManager_GetWord0 @ 0x0064_1100 => extern "C" fn localization_manager_get_word0() -> u32 {
        instance::bridge::localization_manager_word0(&mem(), &mut phyre_printf)
    }
    /// `SetTable_02310f20(i, v)`.
    SetTable_02310f20 @ 0x008b_dfc0 => extern "C" fn set_table_02310f20(i: u32, v: u32) -> u32 {
        instance::bridge::set_table_02310f20(&mut mem(), i, v)
    }
    /// `SaveDataManager_SetWord360(v)`.
    SaveDataManager_SetWord360 @ 0x0064_8640 => extern "C" fn save_data_manager_set_word360(v: u32) -> u32 {
        save::bridge::set_manager_word360(&mut mem(), v)
    }
    /// `TOGetTableKind_008b7fb0(i)`.
    TOGetTableKind_008b7fb0 @ 0x008b_7fb0 => extern "C" fn to_get_table_kind(i: i32) -> i32 {
        window::bridge::table_kind(&mem(), i)
    }
    /// `TOGetTableKind2Data_008b7fd0(i)`.
    TOGetTableKind2Data_008b7fd0 @ 0x008b_7fd0 => extern "C" fn to_get_table_kind2_data(i: i32) -> u32 {
        window::bridge::table_kind2_data(&mem(), i)
    }
    /// `TOGetTableKind2Byte7_008b7ff0(i)`.
    TOGetTableKind2Byte7_008b7ff0 @ 0x008b_7ff0 => extern "C" fn to_get_table_kind2_byte7(i: i32) -> i32 {
        window::bridge::table_kind2_byte7(&mem(), i)
    }
    /// `cd_module_file_sector_rel(n)`.
    cd_module_file_sector_rel @ 0x0076_c250 => extern "C" fn cd_module_file_sector_rel(n: i32) -> u32 {
        cd::bridge::cd_module_file_sector_rel(&mem(), n)
    }
    /// `TOMacroSetByte1(win, n, v)`.
    TOMacroSetByte1 @ 0x008b_84f0 => extern "C" fn to_macro_set_byte1(win: i32, n: i32, v: u8) {
        window::bridge::macro_set_byte1(&mut mem(), win, n, v);
    }
    /// `TOMacroSetValue_008b8510(win, n, v)`: the same code as `TOMacroSetValue`.
    TOMacroSetValue_008b8510 @ 0x008b_8510 => extern "C" fn to_macro_set_value_008b8510(win: i32, n: i32, v: u32) -> u32 {
        window::bridge::macro_set_value(&mut mem(), win, n, v)
    }
    /// `TOAllCloseMessageWindow()`.
    TOAllCloseMessageWindow @ 0x008a_b7b0 => extern "C" fn to_all_close_message_window() {
        window::bridge::all_close_message_window(&mut mem());
    }
    /// `Kernel_GetAbilityName(id)`.
    Kernel_GetAbilityName @ 0x0077_4720 => extern "C" fn kernel_get_ability_name(id: u32) -> u32 {
        names::bridge::ability_name(&mut mem(), id, ffx_menu::tk::bridge::tk_mn2_get_ability_mes)
    }
    /// `user_aligned_free(p)`.
    user_aligned_free @ 0x006f_b6c0 => extern "C" fn user_aligned_free(p: u32) {
        heap::user_aligned_free(&mut mem(), &mut GameOs, p);
    }
    /// `UserHeap_Calloc(n, size)`.
    UserHeap_Calloc @ 0x006f_b820 => extern "C" fn user_heap_calloc(n: u32, size: u32) -> u32 {
        heap::user_calloc(&mut mem(), &mut GameOs, n, size)
    }
    /// `user_calloc(n, size)`: the jump to `UserHeap_Calloc`.
    user_calloc @ 0x0068_7000 => extern "C" fn user_calloc(n: u32, size: u32) -> u32 {
        heap::user_calloc(&mut mem(), &mut GameOs, n, size)
    }
    /// `malloc_pool_init()`.
    malloc_pool_init @ 0x006f_b9b0 => extern "C" fn malloc_pool_init() {
        heap::malloc_pool_init(&mut mem(), &mut GameOs);
    }
    /// `MemSystem_Ensure()`.
    MemSystem_Ensure @ 0x0094_28a0 => extern "C" fn mem_system_ensure() {
        heap::mem_system_ensure(&mem(), &mut GameOs);
    }
    /// `UserHeap_Init(arena, a, b)` (`this` in ECX).
    UserHeap_Init @ 0x0094_2a40 => extern "thiscall" fn user_heap_init(this: u32, arena: u32, a: u32, b: u32) -> u32 {
        heap::heap_init(&mut mem(), this, arena, a, b)
    }
    /// `Phyre::PhyreMalloc(size)`.
    Phyre_PhyreMalloc @ 0x0042_fb40 => extern "C" fn phyre_malloc(size: u32) -> u32 {
        heap::phyre_malloc(&mut mem(), &mut GameOs, size)
    }
    /// `Phyre::PhyreMalloc(size, align)`.
    Phyre_PhyreMalloc_0042fb80 @ 0x0042_fb80 => extern "C" fn phyre_malloc_aligned(size: u32, align: u32) -> u32 {
        heap::phyre_malloc_aligned(&mut mem(), &mut GameOs, size, align)
    }
    /// `Phyre::PhyreFree(p)`.
    Phyre_PhyreFree @ 0x0042_fb20 => extern "C" fn phyre_free(p: u32) {
        heap::phyre_free(&mut mem(), &mut GameOs, p);
    }
    /// `operator new(size)`: `user_malloc`.
    operator_new @ 0x0063_0310 => extern "C" fn operator_new(size: u32) -> u32 {
        heap::user_malloc(&mut mem(), &mut GameOs, size)
    }
    /// `operator new[](size)`: `user_malloc`.
    operator_new_006304c0 @ 0x0063_04c0 => extern "C" fn operator_new_array(size: u32) -> u32 {
        heap::user_malloc(&mut mem(), &mut GameOs, size)
    }
    /// `memclear(p, n)`.
    memclear @ 0x0062_f4e0 => extern "C" fn memclear(p: u32, n: u32) -> u32 {
        heap::memclear(&mut mem(), p, n)
    }
    /// `PStreamFileWin32::isOpen()` (`this` in ECX).
    Phyre_PSerialization_PStreamFileWin32_isOpen @ 0x0060_7f20 => extern "thiscall" fn stream_is_open(this: u32) -> bool {
        files::bridge::stream_is_open(&mem(), this)
    }
    /// `PStreamFile::getFileSize()` (`this` in ECX).
    Phyre_PSerialization_PStreamFile_getFileSize @ 0x0060_7dc0 => extern "thiscall" fn stream_file_size(this: u32) -> u64 {
        files::bridge::stream_file_size(&mut mem(), this, get_file_size_ex)
    }
    /// `PhyreFIOSStreamFile::getFileSize()` (`this` in ECX).
    PhyreFIOSStreamFile_getFileSize @ 0x006f_9d50 => extern "thiscall" fn fios_stream_file_size(this: u32) -> u64 {
        files::bridge::fios_stream_file_size(&mut mem(), this, get_file_size_ex)
    }
    /// `fiosFileSize(stream)`.
    fiosFileSize @ 0x0067_9570 => extern "C" fn fios_file_size(stream: u32) -> u32 {
        files::bridge::fios_file_size(&mut mem(), stream, get_file_size_ex)
    }
    /// `sceFileSize(stream)`.
    sceFileSize @ 0x0062_f760 => extern "C" fn sce_file_size(stream: u32) -> u32 {
        files::bridge::fios_file_size(&mut mem(), stream, get_file_size_ex)
    }
    /// `fiosRead(stream, buf, n)`.
    fiosRead @ 0x0067_9800 => extern "C" fn fios_read(stream: u32, buf: u32, n: u32) -> u32 {
        files::bridge::fios_read(&mut mem(), stream, buf, n, stream_read)
    }
    /// `sceRead(fd, buf, n)`.
    sceRead @ 0x0062_fc00 => extern "C" fn sce_read(fd: u32, buf: u32, n: u32) -> u32 {
        files::bridge::sce_read(&mut mem(), fd, buf, n, stream_read)
    }
    /// `PhyreFIOS::isFileExist(name)`: `File_Probe`.
    PhyreFIOS_isFileExist @ 0x006f_9d70 => extern "C" fn phyre_fios_is_file_exist(name: u32) -> u32 {
        call_sym(&mut mem(), 0x0060_7e00, &[name])
    }
    /// `fiosIsFileExist(name)`.
    fiosIsFileExist @ 0x0067_96f0 => extern "C" fn fios_is_file_exist(name: u32) -> u32 {
        files::bridge::fios_is_file_exist(&mut mem(), name, |m, n| call_sym(m, 0x0060_7e00, &[n]))
    }
    /// `TODVDFileRead(file, buf, len)`.
    TODVDFileRead @ 0x008a_ba30 => extern "C" fn to_dvd_file_read(file: u32, buf: u32, len: u32) -> u32 {
        files::bridge::dvd_file_read(&mut mem(), file, buf, len, call_ptr)
    }
    /// `TOCDGetFileName(file, out)`.
    TOCDGetFileName @ 0x008a_b7d0 => extern "C" fn to_cd_get_file_name(file: u32, out: u32) -> u32 {
        files::bridge::cd_get_file_name(&mut mem(), file, out, call_ptr)
    }
    /// `CDfileName_PC(n)`.
    CDfileName_PC @ 0x00a4_2830 => extern "C" fn cd_file_name_pc(n: u32) -> u32 {
        files::bridge::cd_file_name_pc(&mut mem(), n, |m, at| call_sym(m, 0x00a4_2c00, &[at]))
    }
    /// `GetCDFileName_PC(n)`.
    GetCDFileName_PC @ 0x00a4_29d0 => extern "C" fn get_cd_file_name_pc(n: u32) -> u32 {
        files::bridge::get_cd_file_name_pc(&mut mem(), n, |m, at| call_sym(m, 0x00a4_2c00, &[at]))
    }
    /// `set_read_chgname(from, to)`.
    set_read_chgname @ 0x00a4_3220 => extern "C" fn set_read_chgname(from: u32, to: u32) -> u32 {
        cd::bridge::set_read_chgname(&mut mem(), from, to, |m, e, f, l| {
            call_sym(m, 0x0076_efa0, &[e, f, l]);
        })
    }
    /// `VFile::assign(handle)` (`this` in ECX).
    VFile_assign @ 0x0061_aa10 => extern "thiscall" fn vfile_assign(this: u32, handle: u32) -> u32 {
        files::bridge::vfile_assign(&mut mem(), this, handle)
    }
    /// `VFile::fileRead(buf, n)` (`this` in ECX).
    VFile_fileRead @ 0x0061_aa60 => extern "thiscall" fn vfile_file_read(this: u32, buf: u32, n: u32) -> u32 {
        files::bridge::vfile_file_read(&mut mem(), this, buf, n, |_, t, b, c| {
            // SAFETY: FileSys_Read (thiscall, ret 8), not ported yet.
            unsafe {
                let f: unsafe extern "thiscall" fn(u32, u32, u32) -> u32 =
                    std::mem::transmute(crate::rebase(0x0061_aa90));
                f(t, b, c)
            }
        })
    }
    /// `BigFileStream::setStreamPrefix(path)` (`this` in ECX).
    BigFileStream_setStreamPrefix @ 0x0061_c3a0 => extern "thiscall" fn set_stream_prefix(this: u32, path: u32) -> u32 {
        files::bridge::set_stream_prefix(&mut mem(), &mut GameOs, this, path)
    }
    /// `TOControlWindow(i)`.
    TOControlWindow @ 0x0090_6bd0 => extern "C" fn to_control_window(i: i32) -> u32 {
        window::bridge::control_window(&mut mem(), i, call_ptr)
    }
    /// `TODrawWindow(i)`.
    TODrawWindow @ 0x0090_6bf0 => extern "C" fn to_draw_window(i: i32) -> u32 {
        window::bridge::draw_window(&mut mem(), i, call_ptr)
    }
    /// `TOControlMessage(i)`.
    TOControlMessage @ 0x008b_5850 => extern "C" fn to_control_message(i: i32) -> u32 {
        window::bridge::control_message(&mut mem(), i, call_ptr)
    }
    /// `TOMessageCallF8_008b8ca0(i)`.
    TOMessageCallF8_008b8ca0 @ 0x008b_8ca0 => extern "C" fn to_message_call_f8(i: i32) -> u32 {
        window::bridge::message_call_f8(&mut mem(), i, call_ptr)
    }
    /// `TOGetControlPadStatus(i)`.
    TOGetControlPadStatus @ 0x008b_6b70 => extern "C" fn to_get_control_pad_status(i: i32) -> u32 {
        window::bridge::control_pad_status(&mut mem(), i, call_ptr)
    }
    /// `TOMessageTableByte2_008b8010(i)`.
    TOMessageTableByte2_008b8010 @ 0x008b_8010 => extern "C" fn to_message_table_byte2(i: i32) -> i32 {
        window::bridge::message_table_byte2(&mem(), i)
    }
    /// `TOSetPolyG4A_XYWH(p, x, y, w, h)`.
    TOSetPolyG4A_XYWH @ 0x0090_48a0 => extern "C" fn to_set_poly_g4a_xywh(p: u32, x: u32, y: u32, width: u32, height: u32) {
        window::bridge::set_poly_g4a_xywh(&mut mem(), p, x, y, width, height);
    }
    /// `cd_size_from_table(n)`.
    cd_size_from_table @ 0x0076_dc20 => extern "C" fn cd_size_from_table(n: i32) -> u32 {
        cd::bridge::cd_size_from_table(&mem(), n)
    }
    /// `TOAdpALPHA(p, A, B, C, D, FIX)`.
    TOAdpALPHA @ 0x008e_f670 => extern "C" fn toadpalpha(p: u32, f0: i32, f1: i32, f2: i32, f3: i32, f4: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f670, p, &[f0, f1, f2, f3, f4])
    }
    /// `TOAdpCLAMP(p, WMS, WMT, MINU, MAXU, MINV, MAXV)`.
    TOAdpCLAMP @ 0x008e_f6e0 => extern "C" fn toadpclamp(p: u32, f0: i32, f1: i32, f2: i32, f3: i32, f4: i32, f5: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f6e0, p, &[f0, f1, f2, f3, f4, f5])
    }
    /// `TOAdpCOLCLAMP(p, value)`.
    TOAdpCOLCLAMP @ 0x008e_f760 => extern "C" fn toadpcolclamp(p: u32, f0: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f760, p, &[f0])
    }
    /// `TOAdpFBA(p, value)`.
    TOAdpFBA @ 0x008e_f7d0 => extern "C" fn toadpfba(p: u32, f0: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f7d0, p, &[f0])
    }
    /// `TOAdpFRAME(p, FBP, FBW, PSM, FBMSK)`.
    TOAdpFRAME @ 0x008e_f880 => extern "C" fn toadpframe(p: u32, f0: i32, f1: i32, f2: i32, f3: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f880, p, &[f0, f1, f2, f3])
    }
    /// `TOAdpSCISSOR(p, SCAX0, SCAX1, SCAY0, SCAY1)`.
    TOAdpSCISSOR @ 0x008e_f910 => extern "C" fn toadpscissor(p: u32, f0: i32, f1: i32, f2: i32, f3: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f910, p, &[f0, f1, f2, f3])
    }
    /// `TOAdpTEX1(p, LCM, MXL, MMAG, MMIN, MTBA, L, K)`.
    TOAdpTEX1 @ 0x008e_fad0 => extern "C" fn toadptex1(p: u32, f0: i32, f1: i32, f2: i32, f3: i32, f4: i32, f5: i32, f6: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_fad0, p, &[f0, f1, f2, f3, f4, f5, f6])
    }
    /// `TOAdpXYOFFSET(p, OFX, OFY)`.
    TOAdpXYOFFSET @ 0x008e_fc60 => extern "C" fn toadpxyoffset(p: u32, f0: i32, f1: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_fc60, p, &[f0, f1])
    }
    /// `TOAdpFOG`.
    TOAdpFOG @ 0x008e_f800 => extern "C" fn toadpfog(p: u32, fog: u32) -> u32 {
        pkt::bridge::adp_fog(&mut mem(), p, fog)
    }
    /// `TOAdpTEXFLUSH`.
    TOAdpTEXFLUSH @ 0x008e_fb60 => extern "C" fn toadptexflush(p: u32) -> u32 {
        pkt::bridge::adp_texflush(&mut mem(), p)
    }
    /// `TOAdpXYFrac`.
    TOAdpXYFrac @ 0x008e_fbc0 => extern "C" fn toadpxyfrac(p: u32, x: u32, fx: u32, y: u32, fy: u32) -> u32 {
        pkt::bridge::adp_xy_frac(&mut mem(), p, [x, fx, y, fy])
    }
    /// `TOAdpWords2Reg10`.
    TOAdpWords2Reg10 @ 0x008e_fc30 => extern "C" fn toadpwords2reg10(p: u32, a: u32, b: u32) -> u32 {
        pkt::bridge::adp_words2_reg10(&mut mem(), p, a, b)
    }
    /// `TOAdpWords3`.
    TOAdpWords3 @ 0x008e_fcd0 => extern "C" fn toadpwords3(p: u32, a: u32, b: u32, c: u32) -> u32 {
        pkt::bridge::adp_words3(&mut mem(), p, a, b, c)
    }
    /// `TOAdpXYZOffset`.
    TOAdpXYZOffset @ 0x008e_fc90 => extern "C" fn toadpxyzoffset(p: u32, x: u32, y: u32, z: u32) -> u32 {
        pkt::bridge::adp_xyz_offset(&mut mem(), p, x, y, z, None)
    }
    /// `TOAdpXYZWOffset`.
    TOAdpXYZWOffset @ 0x008e_fd00 => extern "C" fn toadpxyzwoffset(p: u32, x: u32, y: u32, z: u32, w: u32) -> u32 {
        pkt::bridge::adp_xyz_offset(&mut mem(), p, x, y, z, Some(w))
    }
    /// `Virtuos::InputManager::isHoldButton(mask)` (`this` in ECX).
    Virtuos_InputManager_isHoldButton @ 0x0063_1340 => extern "thiscall" fn is_hold_button(this: u32, mask: u32) -> u32 {
        pad::bridge::is_hold_button(&mem(), this, mask)
    }
    /// `Virtuos::InputManager::isPressButton(mask)` (`this` in ECX).
    Virtuos_InputManager_isPressButton @ 0x0063_1360 => extern "thiscall" fn is_press_button(this: u32, mask: u32) -> bool {
        pad::bridge::is_press_button(&mem(), this, mask)
    }
    /// `Virtuos_InputManager_isReleaseButton(mask)` (`this` in ECX).
    Virtuos_InputManager_isReleaseButton @ 0x0063_1380 => extern "thiscall" fn is_release_button(this: u32, mask: u32) -> bool {
        pad::bridge::is_release_button(&mem(), this, mask)
    }
    /// `TOSetPair_008bdfe0(prev, cur)`.
    TOSetPair_008bdfe0 @ 0x008b_dfe0 => extern "C" fn to_set_pair(prev: u32, cur: u32) -> u32 {
        window::bridge::set_pair(&mut mem(), prev, cur)
    }
    /// `TOClearEntry_008ff140(i)`.
    TOClearEntry_008ff140 @ 0x008f_f140 => extern "C" fn to_clear_entry(i: i32) -> u32 {
        window::bridge::clear_entry_18762e0(&mut mem(), i)
    }
    /// `TOSetScreenWH(w, h)`.
    TOSetScreenWH @ 0x0090_48f0 => extern "C" fn to_set_screen_wh(w: u32, h: u32) {
        window::bridge::set_screen_wh(&mut mem(), w, h);
    }
    /// `TOAdpTEX0(p, TBP0, TBW, PSM, TW, TH, TCC, TFX, CBP, CPSM, CSM, CSA, CLD)`.
    TOAdpTEX0 @ 0x008e_fa00 => extern "C" fn toadptex0(p: u32, f0: i32, f1: i32, f2: i32, f3: i32, f4: i32, f5: i32, f6: i32, f7: i32, f8: i32, f9: i32, f10: i32, f11: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_fa00, p, &[f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11])
    }
    /// `SaveData_ClearWord8_006f24d0`.
    SaveData_ClearWord8_006f24d0 @ 0x006f_24d0 => extern "C" fn savedata_clearword8_006f24d0() -> u32 {
        accessor::bridge::clear_word8_of_00ce81f4(&mut mem())
    }
    /// `SetMemberC_006f24f0`.
    SetMemberC_006f24f0 @ 0x006f_24f0 => extern "thiscall" fn setmemberc_006f24f0(this: u32, v: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0xc, v)
    }
    /// `SetMember394_006f2520`.
    SetMember394_006f2520 @ 0x006f_2520 => extern "thiscall" fn setmember394_006f2520(this: u32, v: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0x394, v)
    }
    /// `SetGlobal_01128ac4`.
    SetGlobal_01128ac4 @ 0x0076_cfc0 => extern "C" fn setglobal_01128ac4(v: u32) -> u32 {
        accessor::bridge::set_word_global(&mut mem(), 0x0076_cfc0, v)
    }
    /// `SetGlobal_01841e0c`.
    SetGlobal_01841e0c @ 0x008a_f690 => extern "C" fn setglobal_01841e0c(v: u32) -> u32 {
        accessor::bridge::set_word_global(&mut mem(), 0x008a_f690, v)
    }
    /// `SetGlobal_01a85ba4`.
    SetGlobal_01a85ba4 @ 0x00a4_3210 => extern "C" fn setglobal_01a85ba4(v: u32) -> u32 {
        accessor::bridge::set_word_global(&mut mem(), 0x00a4_3210, v)
    }
    /// `SetByte_01128ac0`.
    SetByte_01128ac0 @ 0x0076_dc60 => extern "C" fn setbyte_01128ac0(v: u8) {
        accessor::bridge::set_byte_01128ac0(&mut mem(), v);
    }
    /// `TOGetMessageWindow_008b9640`.
    TOGetMessageWindow_008b9640 @ 0x008b_9640 => extern "C" fn togetmessagewindow_008b9640(i: i32) -> u32 {
        window::bridge::message_window(&mem(), i)
    }
    /// `TOGetTableEntry_008b9a50`.
    TOGetTableEntry_008b9a50 @ 0x008b_9a50 => extern "C" fn togettableentry_008b9a50(i: i32) -> u32 {
        window::bridge::table_01868a90_entry(&mem(), i)
    }
    /// `Flag_01326b28_008729c0`.
    Flag_01326b28_008729c0 @ 0x0087_29c0 => extern "C" fn flag_01326b28_008729c0() -> u32 {
        accessor::bridge::flag_01326b28(&mut mem())
    }
    /// `Check_01326b28_008729e0`.
    Check_01326b28_008729e0 @ 0x0087_29e0 => extern "C" fn check_01326b28_008729e0() -> u32 {
        accessor::bridge::check_01326b28(&mem())
    }
    /// `TkMenuWorldMapWindowSetWord68`.
    TkMenuWorldMapWindowSetWord68 @ 0x008e_2dd0 => extern "C" fn tkmenuworldmapwindowsetword68(v: u32) -> u32 {
        accessor::bridge::set_world_map_window_word68(&mut mem(), v)
    }
    /// `SaveDataManager_takeByte364`.
    SaveDataManager_takeByte364 @ 0x006f_2120 => extern "thiscall" fn savedatamanager_takebyte364(this: u32) -> u8 {
        accessor::bridge::take_byte364(&mut mem(), this)
    }
    /// `SaveDataManager_setFlag3bc`.
    SaveDataManager_setFlag3bc @ 0x006f_2430 => extern "thiscall" fn savedatamanager_setflag3bc(this: u32, i: u32) -> u32 {
        accessor::bridge::set_flag_3bc(&mut mem(), this, i)
    }
    /// `Reset_006e4fd0`.
    Reset_006e4fd0 @ 0x006e_4fd0 => extern "thiscall" fn reset_006e4fd0(this: u32) {
        accessor::bridge::reset_e4fd0(&mut mem(), this);
    }
    /// `CopyFloats2_007037e0`.
    CopyFloats2_007037e0 @ 0x0070_37e0 => extern "thiscall" fn copyfloats2_007037e0(this: u32, src: u32) -> u32 {
        accessor::bridge::copy_floats(&mut mem(), this, src, 0x14, 2)
    }
    /// `CopyFloats4_00703750`.
    CopyFloats4_00703750 @ 0x0070_3750 => extern "thiscall" fn copyfloats4_00703750(this: u32, src: u32) -> u32 {
        accessor::bridge::copy_floats(&mut mem(), this, src, 4, 4)
    }
    /// `CdSetC40e19_0076cfa0`.
    CdSetC40e19_0076cfa0 @ 0x0076_cfa0 => extern "C" fn cdsetc40e19_0076cfa0(n: u32) {
        accessor::bridge::set_c40e19(&mut mem(), n);
    }
    /// `VFile_reset`.
    VFile_reset @ 0x0061_acf0 => extern "thiscall" fn vfile_reset(this: u32) {
        accessor::bridge::vfile_reset(&mut mem(), this);
    }
    /// `RecordStateIs8_006dbf30`.
    RecordStateIs8_006dbf30 @ 0x006d_bf30 => extern "thiscall" fn recordstateis8_006dbf30(this: u32, i: u32) -> u32 {
        accessor::bridge::record_state_is8(&mem(), this, i)
    }
    /// `hdd_head_sectors`.
    hdd_head_sectors @ 0x0077_8960 => extern "C" fn hdd_head_sectors(i: u32) -> i32 {
        accessor::bridge::hdd_head_sectors(&mem(), i)
    }
    /// `std_Tree_Min_00a419b0`.
    std_Tree_Min_00a419b0 @ 0x00a4_19b0 => extern "C" fn std_tree_min_00a419b0(node: u32) -> u32 {
        accessor::bridge::tree_extreme(&mem(), node, 0)
    }
    /// `std_Tree_Max_00a41990`.
    std_Tree_Max_00a41990 @ 0x00a4_1990 => extern "C" fn std_tree_max_00a41990(node: u32) -> u32 {
        accessor::bridge::tree_extreme(&mem(), node, 8)
    }
    /// `Table_00c334d4_0067aa50`.
    Table_00c334d4_0067aa50 @ 0x0067_aa50 => extern "C" fn table_00c334d4_0067aa50(x: u32) -> u32 {
        accessor::bridge::table_00c334d4(&mem(), x)
    }
    /// `AbmapManager_GetMeshInstace`.
    AbmapManager_GetMeshInstace @ 0x0068_4ca0 => extern "thiscall" fn entrywordc_00684ca0(this: u32, i: u32) -> u32 {
        accessor::bridge::entry_word_c(&mem(), this, i)
    }
    /// `UnlinkRecord_00776910`.
    UnlinkRecord_00776910 @ 0x0077_6910 => extern "C" fn unlinkrecord_00776910(rec: u32) -> u32 {
        accessor::bridge::unlink_record(&mut mem(), rec)
    }
    /// `Copy_009da750`.
    Copy_009da750 @ 0x009d_a750 => extern "C" fn copy_009da750() {
        accessor::bridge::copy_9da750(&mut mem());
    }
    /// `CopyTable_00c8a538`.
    CopyTable_00c8a538 @ 0x0040_2b50 => extern "thiscall" fn copytable_00c8a538(this: u32) -> u32 {
        accessor::bridge::copy_c8a538(&mut mem(), this)
    }
    /// `TOAdpFOGCOL`.
    TOAdpFOGCOL @ 0x008e_f830 => extern "C" fn toadpfogcol(p: u32, fcr: i32, fcg: i32, fcb: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f830, p, &[fcr, fcg, fcb])
    }
    /// `TOAdpZBUF`.
    TOAdpZBUF @ 0x008e_fd40 => extern "C" fn toadpzbuf(p: u32, zbp: i32, psm: i32, zmsk: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_fd40, p, &[zbp, psm, zmsk])
    }
    /// `OpenScissor`.
    OpenScissor @ 0x008e_8980 => extern "C" fn openscissor(x: f32, y: f32, w: f32, h: f32) {
        pkt::bridge::open_scissor(&mut mem(), [x, y, w, h]);
    }
    /// `TOSetRectDirect`.
    TOSetRectDirect @ 0x008b_8cd0 => extern "C" fn tosetrectdirect(p: u32, x: i32, y: i32, w: i32, h: i32) -> u32 {
        pkt::bridge::set_rect_direct(&mut mem(), p, [x, y, w, h])
    }
    /// `SetByte_0186a429_008ca370`.
    SetByte_0186a429_008ca370 @ 0x008c_a370 => extern "C" fn setbyte_0186a429_008ca370(i: u32, v: u8) {
        window::bridge::set_byte_186a429(&mut mem(), i, v);
    }
    /// `StepSwitch_018763e0`.
    StepSwitch_018763e0 @ 0x0090_6740 => extern "C" fn stepswitch_018763e0() -> i32 {
        window::bridge::step_switch(&mut mem())
    }
    /// `PushEntry_013402b8`.
    PushEntry_013402b8 @ 0x0089_e4f0 => extern "C" fn pushentry_013402b8(a: u32, b: u32, c: u32) -> u32 {
        window::bridge::push_entry_13402b8(&mut mem(), a, b, c)
    }
    /// `SetEntries_01869f20`.
    SetEntries_01869f20 @ 0x008b_e050 => extern "C" fn setentries_01869f20(v: u32) {
        window::bridge::set_entries_1869f20(&mut mem(), v);
    }
    /// `TOBtlCtbAnimStep_008a2ad0`.
    TOBtlCtbAnimStep_008a2ad0 @ 0x008a_2ad0 => extern "C" fn tobtlctbanimstep_008a2ad0(rec: u32) -> u32 {
        window::bridge::anim_step_8a2ad0(&mut mem(), rec)
    }
    /// `TOBtlCtrlActiveChrCursor`.
    TOBtlCtrlActiveChrCursor @ 0x0089_0f30 => extern "C" fn tobtladvanceangle_00890f30() {
        window::bridge::advance_angle(&mut mem());
    }
    /// `cd_file_size_bytes`.
    cd_file_size_bytes @ 0x0076_c440 => extern "C" fn cd_file_size_bytes(n: i32) -> u32 {
        cd::bridge::cd_file_size_bytes(&mem(), n)
    }
    /// `SaveDataManager_findEntry_006f1e40`.
    SaveDataManager_findEntry_006f1e40 @ 0x006f_1e40 => extern "thiscall" fn savedatamanager_findentry_006f1e40(this: u32, id: u32) -> u32 {
        save::bridge::find_entry_by_id(&mem(), this, id)
    }
    /// `ReleaseSlots_006fc390`.
    ReleaseSlots_006fc390 @ 0x006f_c390 => extern "thiscall" fn releaseslots_006fc390(this: u32, key: u32) -> u32 {
        save::bridge::release_slots(&mut mem(), this, key)
    }
    /// `MapFindU32_0060c6b0`.
    MapFindU32_0060c6b0 @ 0x0060_c6b0 => extern "thiscall" fn mapfindu32_0060c6b0(this: u32, key: u32) -> u32 {
        stl::bridge::map_find_u32(&mem(), this, key)
    }
    /// `SwapWords3_00647440`.
    SwapWords3_00647440 @ 0x0064_7440 => extern "C" fn swapwords3_00647440(p: u32) -> u32 {
        accessor::bridge::swap3(&mut mem(), p)
    }
    /// `SwapWords3Out_00647490`.
    SwapWords3Out_00647490 @ 0x0064_7490 => extern "C" fn swapwords3out_00647490(p: u32, out: u32) -> u32 {
        accessor::bridge::swap3_out(&mut mem(), p, out)
    }
    /// `Settings_CopyDefaultTables`.
    Settings_CopyDefaultTables @ 0x0040_2b00 => extern "thiscall" fn settings_copydefaulttables(this: u32) -> u32 {
        accessor::bridge::copy_default_tables(&mut mem(), this)
    }
    /// `InputManager_SwapState_00632930`.
    InputManager_SwapState_00632930 @ 0x0063_2930 => extern "thiscall" fn inputmanager_swapstate_00632930(this: u32, out: u32, x: u32, y: u32, z: u32) -> u32 {
        accessor::bridge::swap_state_ac(&mut mem(), this, out, [x, y, z])
    }
    /// `SelectId_006fb320`.
    SelectId_006fb320 @ 0x006f_b320 => extern "thiscall" fn selectid_006fb320(this: u32, n: u32) -> u32 {
        accessor::bridge::select_id(&mut mem(), this, n)
    }
    /// `SetPairFloats_006e4020`.
    SetPairFloats_006e4020 @ 0x006e_4020 => extern "thiscall" fn setpairfloats_006e4020(this: u32, key: u32, x: f32, y: f32) -> u32 {
        accessor::bridge::set_pair_floats(&mut mem(), this, key, x, y)
    }
    /// `Fixup_RunAll`.
    Fixup_RunAll @ 0x0060_ca70 => extern "thiscall" fn fixup_runall(this: u32, f: u32, src: u32, ctx: u32, dst: u32) -> u32 {
        accessor::bridge::fixup_run_all(&mut mem(), this, [f, src, ctx, dst], 0x1c, call_ptr)
    }
    /// `Fixup_RunAll16`.
    Fixup_RunAll16 @ 0x0060_ca10 => extern "thiscall" fn fixup_runall16(this: u32, f: u32, src: u32, ctx: u32, dst: u32) -> u32 {
        accessor::bridge::fixup_run_all(&mut mem(), this, [f, src, ctx, dst], 0x10, call_ptr)
    }
    /// `std_Tree_Lrotate_00a41930`.
    std_Tree_Lrotate_00a41930 @ 0x00a4_1930 => extern "thiscall" fn std_tree_lrotate_00a41930(this: u32, node: u32) {
        stl::bridge::tree_rotate_left(&mut mem(), this, node);
    }
    /// `std_Tree_Rrotate_00a41b40`.
    std_Tree_Rrotate_00a41b40 @ 0x00a4_1b40 => extern "thiscall" fn std_tree_rrotate_00a41b40(this: u32, node: u32) {
        stl::bridge::tree_rotate_right(&mut mem(), this, node);
    }
    /// `TOOpenWindow_00906c80`.
    TOOpenWindow_00906c80 @ 0x0090_6c80 => extern "C" fn toopenwindow_00906c80(i: i32) -> u32 {
        window::bridge::open_window(&mut mem(), i, call_ptr)
    }
    /// `TOOpenMessage_008b8560`.
    TOOpenMessage_008b8560 @ 0x008b_8560 => extern "C" fn toopenmessage_008b8560(i: i32) -> u32 {
        window::bridge::open_message(&mut mem(), i, call_ptr)
    }
    /// `TOClearMessageSlot_008ab800`.
    TOClearMessageSlot_008ab800 @ 0x008a_b800 => extern "C" fn toclearmessageslot_008ab800(i: u32) -> u32 {
        window::bridge::clear_message_slot(&mut mem(), i)
    }
    /// `Virtuos_InputManager_isRepButton`.
    Virtuos_InputManager_isRepButton @ 0x0063_13a0 => extern "thiscall" fn virtuos_inputmanager_isrepbutton(this: u32, mask: u32) -> bool {
        pad::bridge::is_rep_button(&mut mem(), this, mask)
    }
    /// `PcReadFunc_set`.
    PcReadFunc_set @ 0x00a4_2ba0 => extern "C" fn pcreadfunc_set() {
        cd::bridge::pc_read_func_set(&mut mem());
    }
    /// `TOXyoffsetPacket_008ff020`.
    TOXyoffsetPacket_008ff020 @ 0x008f_f020 => extern "C" fn toxyoffsetpacket_008ff020(x: u32, y: u32) -> u32 {
        pkt::bridge::xyoffset_packet(&mut mem(), x, y)
    }
    /// `CopyCuttingLines_00a7b1a0`.
    CopyCuttingLines_00a7b1a0 @ 0x00a7_b1a0 => extern "C" fn copycuttinglines_00a7b1a0(dst: u32, src: u32) -> u32 {
        msg::bridge::copy_cutting_lines(&mut mem(), dst, src)
    }
    /// `TONumberFontString_0089e600`.
    TONumberFontString_0089e600 @ 0x0089_e600 => extern "C" fn tonumberfontstring_0089e600(src: u32, dst: u32) {
        msg::bridge::number_font_string(&mut mem(), src, dst);
    }
    /// `MagicFile_FindPreloaded_009dad70`.
    MagicFile_FindPreloaded_009dad70 @ 0x009d_ad70 => extern "C" fn magicfile_findpreloaded_009dad70(id: i32, out: u32) -> u32 {
        instance::bridge::find_preloaded(&mut mem(), id, out)
    }
    /// `PStreamFile::GetStreamPrefix()`.
    Phyre_PSerialization_PStreamFile_GetStreamPrefix @ 0x0060_7d30 => extern "C" fn get_stream_prefix() -> u32 {
        files::bridge::stream_prefix(&mem())
    }
    /// `fiosUnifyFilename(src, out, size)`.
    fiosUnifyFilename @ 0x0067_9820 => extern "C" fn fios_unify_filename(src: u32, out: u32, size: u32) -> u32 {
        files::bridge::unify_filename(&mut mem(), src, out, size, |m, level, fmt, arg| {
            call_sym(m, 0x0043_52f0, &[level, fmt, arg]);
        })
    }
    /// `get_hdd_name(n)`.
    get_hdd_name @ 0x0077_7eb0 => extern "C" fn get_hdd_name(n: i32) -> u32 {
        cd::bridge::get_hdd_name(&mut mem(), n)
    }
    /// `AllocTracker_Release(p)`.
    AllocTracker_Release @ 0x0063_0a60 => extern "C" fn alloc_tracker_release(p: u32) {
        heap::tracker_release(&mut mem(), &mut GameOs, p, red_black_remove);
    }
    /// `operator delete(p)`.
    operator_delete @ 0x0063_0390 => extern "C" fn operator_delete(p: u32) {
        heap::operator_delete(&mut mem(), &mut GameOs, p, red_black_remove);
    }
    /// `operator delete[](p)`.
    operator_delete_00630540 @ 0x0063_0540 => extern "C" fn operator_delete_array(p: u32) {
        heap::operator_delete_array(&mut mem(), &mut GameOs, p, red_black_remove);
    }
    /// `ResetRecords18_006fbdc0`.
    ResetRecords18_006fbdc0 @ 0x006f_bdc0 => extern "C" fn resetrecords18_006fbdc0(base: u32, n: u32) -> u32 {
        accessor::bridge::reset_records_18(&mut mem(), base, n)
    }
    /// `SwapHalves_006473d0`.
    SwapHalves_006473d0 @ 0x0064_73d0 => extern "C" fn swaphalves_006473d0(p: u32) -> u32 {
        accessor::bridge::swap_halves(&mut mem(), p)
    }
    /// `TouchRecord_Reset`.
    TouchRecord_Reset @ 0x0068_9510 => extern "thiscall" fn touchrecord_reset(this: u32) -> u32 {
        pad::bridge::touch_record_reset(&mut mem(), this)
    }
    /// `MapFindU32_00691dc0`.
    MapFindU32_00691dc0 @ 0x0069_1dc0 => extern "thiscall" fn mapfindu32_00691dc0(this: u32, key: u32) -> u32 {
        stl::bridge::map_find_u32(&mem(), this, key)
    }
    /// `TOSeqClearSlots_008adf20`.
    TOSeqClearSlots_008adf20 @ 0x008a_df20 => extern "C" fn toseqclearslots_008adf20() -> u32 {
        scene::bridge::clear_slots(&mut mem())
    }
    /// `TOSeqGetSceneState_008ae530`.
    TOSeqGetSceneState_008ae530 @ 0x008a_e530 => extern "C" fn toseqgetscenestate_008ae530(i: i32, name: u32) -> i32 {
        scene::bridge::scene_state(&mem(), i, name)
    }
    /// `TOSetPolyG4A_RGBA`.
    TOSetPolyG4A_RGBA @ 0x0090_47f0 => extern "C" fn tosetquadrgba_009047f0(p: u32, k: u32, r: u8, g: u8, b: u8, a: u8) {
        window::bridge::set_quad_rgba(&mut mem(), p, k, [r, g, b, a]);
    }
    /// `TOSetMessageArea_008b8b20`.
    TOSetMessageArea_008b8b20 @ 0x008b_8b20 => extern "C" fn tosetmessagearea_008b8b20(i: i32, x: i32, y: i32, w: i32, h: i32) {
        window::bridge::set_message_area(&mut mem(), i, [x, y, w, h]);
    }
    /// `VFile_fileSeek`.
    VFile_fileSeek @ 0x0061_ac30 => extern "thiscall" fn vfile_fileseek(this: u32, offset: i32, whence: u32) -> u32 {
        files::bridge::vfile_seek(&mut mem(), this, offset, whence)
    }
    /// `SwapHeaderF00_00647220`.
    SwapHeaderF00_00647220 @ 0x0064_7220 => extern "C" fn swapheaderf00_00647220(p: u32) -> u32 {
        accessor::bridge::swap_header_f00(&mut mem(), p)
    }
    /// `SwapHeader84_00647560`.
    SwapHeader84_00647560 @ 0x0064_7560 => extern "C" fn swapheader84_00647560(p: u32) -> u32 {
        accessor::bridge::swap_header_84(&mut mem(), p)
    }
    /// `Init68_00687190`.
    Init68_00687190 @ 0x0068_7190 => extern "thiscall" fn init68_00687190(this: u32, x: u32, y: u32) -> u32 {
        accessor::bridge::init_68(&mut mem(), this, x, y)
    }
    /// `CompareI64Pairs_0061d730`.
    CompareI64Pairs_0061d730 @ 0x0061_d730 => extern "stdcall" fn comparei64pairs_0061d730(x: u32, y: u32) -> i32 {
        accessor::bridge::compare_i64_pairs(&mem(), x, y)
    }
    /// `TaggedTreeInsert_0060c990`.
    TaggedTreeInsert_0060c990 @ 0x0060_c990 => extern "thiscall" fn taggedtreeinsert_0060c990(this: u32, node: u32) -> u32 {
        stl::bridge::tagged_tree_insert(&mut mem(), this, node)
    }
    /// `TaggedTreeInsert_00691e60`.
    TaggedTreeInsert_00691e60 @ 0x0069_1e60 => extern "thiscall" fn taggedtreeinsert_00691e60(this: u32, node: u32) -> u32 {
        stl::bridge::tagged_tree_insert(&mut mem(), this, node)
    }
    /// `TOMoveEntry_00900f80`.
    TOMoveEntry_00900f80 @ 0x0090_0f80 => extern "C" fn tomoveentry_00900f80(i: i32, x: i32, y: i32, d: i32) {
        window::bridge::move_entry_18762e0(&mut mem(), i, x, y, d);
    }
    /// `Difference_0061e6f0`.
    Difference_0061e6f0 @ 0x0061_e6f0 => extern "C" fn difference_0061e6f0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_0061e700`.
    Difference_0061e700 @ 0x0061_e700 => extern "C" fn difference_0061e700(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_00675c70`.
    Difference_00675c70 @ 0x0067_5c70 => extern "C" fn difference_00675c70(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_00676960`.
    Difference_00676960 @ 0x0067_6960 => extern "C" fn difference_00676960(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_006772d0`.
    Difference_006772d0 @ 0x0067_72d0 => extern "C" fn difference_006772d0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_00678010`.
    Difference_00678010 @ 0x0067_8010 => extern "C" fn difference_00678010(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddb30`.
    Difference_009ddb30 @ 0x009d_db30 => extern "C" fn difference_009ddb30(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddb40`.
    Difference_009ddb40 @ 0x009d_db40 => extern "C" fn difference_009ddb40(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddb50`.
    Difference_009ddb50 @ 0x009d_db50 => extern "C" fn difference_009ddb50(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddb60`.
    Difference_009ddb60 @ 0x009d_db60 => extern "C" fn difference_009ddb60(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddb70`.
    Difference_009ddb70 @ 0x009d_db70 => extern "C" fn difference_009ddb70(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddb80`.
    Difference_009ddb80 @ 0x009d_db80 => extern "C" fn difference_009ddb80(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddb90`.
    Difference_009ddb90 @ 0x009d_db90 => extern "C" fn difference_009ddb90(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddba0`.
    Difference_009ddba0 @ 0x009d_dba0 => extern "C" fn difference_009ddba0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddbb0`.
    Difference_009ddbb0 @ 0x009d_dbb0 => extern "C" fn difference_009ddbb0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddbc0`.
    Difference_009ddbc0 @ 0x009d_dbc0 => extern "C" fn difference_009ddbc0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddbd0`.
    Difference_009ddbd0 @ 0x009d_dbd0 => extern "C" fn difference_009ddbd0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddbe0`.
    Difference_009ddbe0 @ 0x009d_dbe0 => extern "C" fn difference_009ddbe0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddbf0`.
    Difference_009ddbf0 @ 0x009d_dbf0 => extern "C" fn difference_009ddbf0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddc00`.
    Difference_009ddc00 @ 0x009d_dc00 => extern "C" fn difference_009ddc00(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddc10`.
    Difference_009ddc10 @ 0x009d_dc10 => extern "C" fn difference_009ddc10(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddc20`.
    Difference_009ddc20 @ 0x009d_dc20 => extern "C" fn difference_009ddc20(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddc30`.
    Difference_009ddc30 @ 0x009d_dc30 => extern "C" fn difference_009ddc30(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddc40`.
    Difference_009ddc40 @ 0x009d_dc40 => extern "C" fn difference_009ddc40(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddc50`.
    Difference_009ddc50 @ 0x009d_dc50 => extern "C" fn difference_009ddc50(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddc60`.
    Difference_009ddc60 @ 0x009d_dc60 => extern "C" fn difference_009ddc60(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddc70`.
    Difference_009ddc70 @ 0x009d_dc70 => extern "C" fn difference_009ddc70(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddc80`.
    Difference_009ddc80 @ 0x009d_dc80 => extern "C" fn difference_009ddc80(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddc90`.
    Difference_009ddc90 @ 0x009d_dc90 => extern "C" fn difference_009ddc90(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddca0`.
    Difference_009ddca0 @ 0x009d_dca0 => extern "C" fn difference_009ddca0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddcb0`.
    Difference_009ddcb0 @ 0x009d_dcb0 => extern "C" fn difference_009ddcb0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddcc0`.
    Difference_009ddcc0 @ 0x009d_dcc0 => extern "C" fn difference_009ddcc0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Difference_009ddcd0`.
    Difference_009ddcd0 @ 0x009d_dcd0 => extern "C" fn difference_009ddcd0(x: u32, y: u32) -> u32 {
        accessor::difference(x, y)
    }
    /// `Add_0061e740`.
    Add_0061e740 @ 0x0061_e740 => extern "C" fn add_0061e740(x: u32, y: u32) -> u32 {
        accessor::add(x, y)
    }
    /// `Add_006331a0`.
    Add_006331a0 @ 0x0063_31a0 => extern "C" fn add_006331a0(x: u32, y: u32) -> u32 {
        accessor::add(x, y)
    }
    /// `Add_0069c4b0`.
    Add_0069c4b0 @ 0x0069_c4b0 => extern "C" fn add_0069c4b0(x: u32, y: u32) -> u32 {
        accessor::add(x, y)
    }
    /// `Add_0069c4d0`.
    Add_0069c4d0 @ 0x0069_c4d0 => extern "C" fn add_0069c4d0(x: u32, y: u32) -> u32 {
        accessor::add(x, y)
    }
    /// `Add_006e45c0`.
    Add_006e45c0 @ 0x006e_45c0 => extern "C" fn add_006e45c0(x: u32, y: u32) -> u32 {
        accessor::add(x, y)
    }
    /// `Add_006f5370`.
    Add_006f5370 @ 0x006f_5370 => extern "C" fn add_006f5370(x: u32, y: u32) -> u32 {
        accessor::add(x, y)
    }
    /// `Add_006f5a00`.
    Add_006f5a00 @ 0x006f_5a00 => extern "C" fn add_006f5a00(x: u32, y: u32) -> u32 {
        accessor::add(x, y)
    }
    /// `Add_00702fd0`.
    Add_00702fd0 @ 0x0070_2fd0 => extern "C" fn add_00702fd0(x: u32, y: u32) -> u32 {
        accessor::add(x, y)
    }
    /// `Add_009ddce0`.
    Add_009ddce0 @ 0x009d_dce0 => extern "C" fn add_009ddce0(x: u32, y: u32) -> u32 {
        accessor::add(x, y)
    }
    /// `Add_009ddcf0`.
    Add_009ddcf0 @ 0x009d_dcf0 => extern "C" fn add_009ddcf0(x: u32, y: u32) -> u32 {
        accessor::add(x, y)
    }
    /// `AddU16_006331b0`.
    AddU16_006331b0 @ 0x0063_31b0 => extern "C" fn addu16_006331b0(x: u32, y: u32) -> u32 {
        accessor::add(x, y & 0xffff)
    }
    /// `AddU16_0069c4c0`.
    AddU16_0069c4c0 @ 0x0069_c4c0 => extern "C" fn addu16_0069c4c0(x: u32, y: u32) -> u32 {
        accessor::add(x, y & 0xffff)
    }
    /// `AddU16_006e45d0`.
    AddU16_006e45d0 @ 0x006e_45d0 => extern "C" fn addu16_006e45d0(x: u32, y: u32) -> u32 {
        accessor::add(x, y & 0xffff)
    }
    /// `AddU16_006f5380`.
    AddU16_006f5380 @ 0x006f_5380 => extern "C" fn addu16_006f5380(x: u32, y: u32) -> u32 {
        accessor::add(x, y & 0xffff)
    }
    /// `AddU16_006f5750`.
    AddU16_006f5750 @ 0x006f_5750 => extern "C" fn addu16_006f5750(x: u32, y: u32) -> u32 {
        accessor::add(x, y & 0xffff)
    }
    /// `AddU16_00702fe0`.
    AddU16_00702fe0 @ 0x0070_2fe0 => extern "C" fn addu16_00702fe0(x: u32, y: u32) -> u32 {
        accessor::add(x, y & 0xffff)
    }
    /// `AddU8_0060ab90`.
    AddU8_0060ab90 @ 0x0060_ab90 => extern "C" fn addu8_0060ab90(x: u32, y: u32) -> u32 {
        accessor::add(x, y & 0xff)
    }
    /// `AddU8_0061d120`.
    AddU8_0061d120 @ 0x0061_d120 => extern "C" fn addu8_0061d120(x: u32, y: u32) -> u32 {
        accessor::add(x, y & 0xff)
    }
    /// `AddU8_0069c4e0`.
    AddU8_0069c4e0 @ 0x0069_c4e0 => extern "C" fn addu8_0069c4e0(x: u32, y: u32) -> u32 {
        accessor::add(x, y & 0xff)
    }
    /// `ReturnZeroStd_006765c0`.
    ReturnZeroStd_006765c0 @ 0x0067_65c0 => extern "stdcall" fn returnzerostd_006765c0(_x: u32) -> u32 {
        0
    }
    /// `ReturnZeroStd_00677060`.
    ReturnZeroStd_00677060 @ 0x0067_7060 => extern "stdcall" fn returnzerostd_00677060(_x: u32) -> u32 {
        0
    }
    /// `ReturnZeroStd_00677c20`.
    ReturnZeroStd_00677c20 @ 0x0067_7c20 => extern "stdcall" fn returnzerostd_00677c20(_x: u32) -> u32 {
        0
    }
    /// `ReturnZeroStd_00678b40`.
    ReturnZeroStd_00678b40 @ 0x0067_8b40 => extern "stdcall" fn returnzerostd_00678b40(_x: u32) -> u32 {
        0
    }
    /// `ReturnZeroStd_00a42360`.
    ReturnZeroStd_00a42360 @ 0x00a4_2360 => extern "stdcall" fn returnzerostd_00a42360(_x: u32) -> u32 {
        0
    }
    /// `StoreRetThis_006338e0`.
    StoreRetThis_006338e0 @ 0x0063_38e0 => extern "thiscall" fn storeretthis_006338e0(this: u32, v: u32, _unused: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0, v);
        this
    }
    /// `StoreRetThis_006338f0`.
    StoreRetThis_006338f0 @ 0x0063_38f0 => extern "thiscall" fn storeretthis_006338f0(this: u32, v: u32, _unused: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0, v);
        this
    }
    /// `StoreRetThis_00633900`.
    StoreRetThis_00633900 @ 0x0063_3900 => extern "thiscall" fn storeretthis_00633900(this: u32, v: u32, _unused: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0, v);
        this
    }
    /// `StoreRetThis_00633910`.
    StoreRetThis_00633910 @ 0x0063_3910 => extern "thiscall" fn storeretthis_00633910(this: u32, v: u32, _unused: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0, v);
        this
    }
    /// `StoreRetThis_00633920`.
    StoreRetThis_00633920 @ 0x0063_3920 => extern "thiscall" fn storeretthis_00633920(this: u32, v: u32, _unused: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0, v);
        this
    }
    /// `StoreRetThis_00633930`.
    StoreRetThis_00633930 @ 0x0063_3930 => extern "thiscall" fn storeretthis_00633930(this: u32, v: u32, _unused: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0, v);
        this
    }
    /// `StoreRetThis_00633940`.
    StoreRetThis_00633940 @ 0x0063_3940 => extern "thiscall" fn storeretthis_00633940(this: u32, v: u32, _unused: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0, v);
        this
    }
    /// `StoreRetThis_00633950`.
    StoreRetThis_00633950 @ 0x0063_3950 => extern "thiscall" fn storeretthis_00633950(this: u32, v: u32, _unused: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0, v);
        this
    }
    /// `StoreRetThis_00633960`.
    StoreRetThis_00633960 @ 0x0063_3960 => extern "thiscall" fn storeretthis_00633960(this: u32, v: u32, _unused: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 0, v);
        this
    }
    /// `CopyThisTo_00686ce0`.
    CopyThisTo_00686ce0 @ 0x0068_6ce0 => extern "thiscall" fn copythisto_00686ce0(this: u32, p: u32) -> u32 {
        accessor::bridge::copy_member_to(&mut mem(), this, 0, p)
    }
    /// `CopyThisTo_00686e80`.
    CopyThisTo_00686e80 @ 0x0068_6e80 => extern "thiscall" fn copythisto_00686e80(this: u32, p: u32) -> u32 {
        accessor::bridge::copy_member_to(&mut mem(), this, 0, p)
    }
    /// `CopyThisTo_0069af30`.
    CopyThisTo_0069af30 @ 0x0069_af30 => extern "thiscall" fn copythisto_0069af30(this: u32, p: u32) -> u32 {
        accessor::bridge::copy_member_to(&mut mem(), this, 0, p)
    }
    /// `CopyThisTo_006f3fe0`.
    CopyThisTo_006f3fe0 @ 0x006f_3fe0 => extern "thiscall" fn copythisto_006f3fe0(this: u32, p: u32) -> u32 {
        accessor::bridge::copy_member_to(&mut mem(), this, 0, p)
    }
    /// `CopyThis4To_006f4090`.
    CopyThis4To_006f4090 @ 0x006f_4090 => extern "thiscall" fn copythis4to_006f4090(this: u32, p: u32) -> u32 {
        accessor::bridge::copy_member_to(&mut mem(), this, 4, p)
    }
    /// `CopyThis4To_006f40a0`.
    CopyThis4To_006f40a0 @ 0x006f_40a0 => extern "thiscall" fn copythis4to_006f40a0(this: u32, p: u32) -> u32 {
        accessor::bridge::copy_member_to(&mut mem(), this, 4, p)
    }
    /// `SetMember8_00647bb0`.
    SetMember8_00647bb0 @ 0x0064_7bb0 => extern "thiscall" fn setmember8_00647bb0(this: u32, v: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 8, v)
    }
    /// `SetMember8_006fc340`.
    SetMember8_006fc340 @ 0x006f_c340 => extern "thiscall" fn setmember8_006fc340(this: u32, v: u32) -> u32 {
        accessor::bridge::set_member(&mut mem(), this, 8, v)
    }
    /// `Index4At8_00690bd0`.
    Index4At8_00690bd0 @ 0x0069_0bd0 => extern "thiscall" fn index4at8_00690bd0(this: u32, i: u32) -> u32 {
        accessor::bridge::index_member(&mem(), this, 8, i, 4)
    }
    /// `Index4At8_00690be0`.
    Index4At8_00690be0 @ 0x0069_0be0 => extern "thiscall" fn index4at8_00690be0(this: u32, i: u32) -> u32 {
        accessor::bridge::index_member(&mem(), this, 8, i, 4)
    }
    /// `Index4At8_00690bf0`.
    Index4At8_00690bf0 @ 0x0069_0bf0 => extern "thiscall" fn index4at8_00690bf0(this: u32, i: u32) -> u32 {
        accessor::bridge::index_member(&mem(), this, 8, i, 4)
    }
    /// `Index4At8_00690c00`.
    Index4At8_00690c00 @ 0x0069_0c00 => extern "thiscall" fn index4at8_00690c00(this: u32, i: u32) -> u32 {
        accessor::bridge::index_member(&mem(), this, 8, i, 4)
    }
    /// `Index4At4_009e2cb0`.
    Index4At4_009e2cb0 @ 0x009e_2cb0 => extern "thiscall" fn index4at4_009e2cb0(this: u32, i: u32) -> u32 {
        accessor::bridge::index_member(&mem(), this, 4, i, 4)
    }
    /// `Index4At4_009e2cc0`.
    Index4At4_009e2cc0 @ 0x009e_2cc0 => extern "thiscall" fn index4at4_009e2cc0(this: u32, i: u32) -> u32 {
        accessor::bridge::index_member(&mem(), this, 4, i, 4)
    }
    /// `Index4At4_009e2cd0`.
    Index4At4_009e2cd0 @ 0x009e_2cd0 => extern "thiscall" fn index4at4_009e2cd0(this: u32, i: u32) -> u32 {
        accessor::bridge::index_member(&mem(), this, 4, i, 4)
    }
    /// `Index8At8_00631170`.
    Index8At8_00631170 @ 0x0063_1170 => extern "thiscall" fn index8at8_00631170(this: u32, i: u32) -> u32 {
        accessor::bridge::index_member(&mem(), this, 8, i, 8)
    }
    /// `Index8At8_00631180`.
    Index8At8_00631180 @ 0x0063_1180 => extern "thiscall" fn index8at8_00631180(this: u32, i: u32) -> u32 {
        accessor::bridge::index_member(&mem(), this, 8, i, 8)
    }
    /// `SetGlobal_00cc9a38`.
    SetGlobal_00cc9a38 @ 0x0061_9e70 => extern "C" fn setglobal_00cc9a38(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x0061_9e70, v)
    }
    /// `SetGlobal_00ccb9a0`.
    SetGlobal_00ccb9a0 @ 0x0064_8680 => extern "C" fn setglobal_00ccb9a0(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x0064_8680, v)
    }
    /// `SetGlobal_00ccc890`.
    SetGlobal_00ccc890 @ 0x0069_1b50 => extern "C" fn setglobal_00ccc890(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x0069_1b50, v)
    }
    /// `SetGlobal_00ccc894`.
    SetGlobal_00ccc894 @ 0x0069_1b60 => extern "C" fn setglobal_00ccc894(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x0069_1b60, v)
    }
    /// `SetGlobal_02305844`.
    SetGlobal_02305844 @ 0x0076_65a0 => extern "C" fn setglobal_02305844(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x0076_65a0, v)
    }
    /// `SetGlobal_02305840`.
    SetGlobal_02305840 @ 0x0076_65b0 => extern "C" fn setglobal_02305840(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x0076_65b0, v)
    }
    /// `SetGlobal_0112a018`.
    SetGlobal_0112a018 @ 0x0077_5810 => extern "C" fn setglobal_0112a018(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x0077_5810, v)
    }
    /// `SetGlobal_00c58ef8`.
    SetGlobal_00c58ef8 @ 0x0089_bc50 => extern "C" fn setglobal_00c58ef8(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x0089_bc50, v)
    }
    /// `SetGlobal_00c59f38`.
    SetGlobal_00c59f38 @ 0x008a_b570 => extern "C" fn setglobal_00c59f38(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x008a_b570, v)
    }
    /// `SetGlobal_00c5a084`.
    SetGlobal_00c5a084 @ 0x008b_e9b0 => extern "C" fn setglobal_00c5a084(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x008b_e9b0, v)
    }
    /// `SetGlobal_00c64ca4`.
    SetGlobal_00c64ca4 @ 0x009d_a790 => extern "C" fn setglobal_00c64ca4(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x009d_a790, v)
    }
    /// `SetGlobal_0193fdac`.
    SetGlobal_0193fdac @ 0x009d_afd0 => extern "C" fn setglobal_0193fdac(v: u32) -> u32 {
        accessor::bridge::set_word_global2(&mut mem(), 0x009d_afd0, v)
    }
    /// `SetByte_012fb8fb`.
    SetByte_012fb8fb @ 0x0082_1a20 => extern "C" fn setbyte_012fb8fb(v: u8) {
        accessor::bridge::set_byte_global(&mut mem(), 0x0082_1a20, v);
    }
    /// `SetByte_012fb8fc`.
    SetByte_012fb8fc @ 0x0082_1a30 => extern "C" fn setbyte_012fb8fc(v: u8) {
        accessor::bridge::set_byte_global(&mut mem(), 0x0082_1a30, v);
    }
    /// `SetByte_0133d782`.
    SetByte_0133d782 @ 0x0089_a850 => extern "C" fn setbyte_0133d782(v: u8) {
        accessor::bridge::set_byte_global(&mut mem(), 0x0089_a850, v);
    }
    /// `SetByte_00c58ef4`.
    SetByte_00c58ef4 @ 0x0089_bc60 => extern "C" fn setbyte_00c58ef4(v: u8) {
        accessor::bridge::set_byte_global(&mut mem(), 0x0089_bc60, v);
    }
    /// `SleepThreadInMillisecond(ms)`: `Sleep(ms)`, 0.
    SleepThreadInMillisecond @ 0x006f_2d20 => extern "C" fn sleep_thread_in_millisecond(ms: u32) -> u32 {
        sync::bridge::sleep_ms(&mut mem(), &mut GameOs, ms)
    }
    /// `SleepThread()`: `Sleep(10)`, 0.
    SleepThread @ 0x006f_2d10 => extern "C" fn sleep_thread() -> u32 {
        sync::bridge::sleep_thread(&mut mem(), &mut GameOs)
    }
    /// `Thread_Sleep(ms)`: `Sleep(ms)`.
    Thread_Sleep @ 0x006f_2a40 => extern "C" fn thread_sleep(ms: u32) -> u32 {
        sync::bridge::sleep(&mut mem(), &mut GameOs, ms)
    }
    /// `MutexHandle_Close()`: `CloseHandle(*this)`.
    MutexHandle_Close @ 0x0063_4830 => extern "thiscall" fn mutex_handle_close(this: u32) -> u32 {
        sync::bridge::close_member(&mut mem(), &mut GameOs, this, 0)
    }
    /// `Worker_CloseSemaphore()`: `CloseHandle(this->+0x2c)`.
    Worker_CloseSemaphore @ 0x006d_b6e0 => extern "thiscall" fn worker_close_semaphore(this: u32) -> u32 {
        sync::bridge::close_member(&mut mem(), &mut GameOs, this, 0x2c)
    }
    /// `CriticalSection_Init()`: returns `this`.
    CriticalSection_ctor @ 0x006f_b500 => extern "thiscall" fn critical_section_init(this: u32) -> u32 {
        sync::bridge::cs_init(&mut mem(), &mut GameOs, this)
    }
    /// `CsLock_Construct(cs)`: returns `this`.
    CriticalSectionLock_ctor @ 0x006f_b510 => extern "thiscall" fn cs_lock_construct(this: u32, cs: u32) -> u32 {
        sync::bridge::cs_guard(&mut mem(), &mut GameOs, this, cs)
    }
    /// `CriticalSection_Delete()`.
    CriticalSection_dtor @ 0x006f_b530 => extern "thiscall" fn critical_section_delete(this: u32) -> u32 {
        sync::bridge::cs_delete(&mut mem(), &mut GameOs, this)
    }
    /// `CsLock_Destruct()`.
    CriticalSectionLock_dtor @ 0x006f_b540 => extern "thiscall" fn cs_lock_destruct(this: u32) -> u32 {
        sync::bridge::cs_guard_drop(&mut mem(), &mut GameOs, this)
    }
    /// `Event_Reset(&event)`.
    Event_Reset @ 0x006f_2be0 => extern "C" fn event_reset(event: u32) -> u32 {
        sync::bridge::event_set(&mut mem(), &mut GameOs, event, false)
    }
    /// `Event_Set(&event)`.
    Event_Set @ 0x006f_2bf0 => extern "C" fn event_set(event: u32) -> u32 {
        sync::bridge::event_set(&mut mem(), &mut GameOs, event, true)
    }
    /// `SignalSema(&sema)`.
    SignalSema @ 0x006f_2ce0 => extern "C" fn signal_sema(sema: u32) -> u32 {
        sync::bridge::signal_sema(&mut mem(), &mut GameOs, sema)
    }
    /// `WaitSema(&sema)`: without end.
    WaitSema @ 0x006f_2d80 => extern "C" fn wait_sema(sema: u32) -> u32 {
        sync::bridge::wait_sema(&mut mem(), &mut GameOs, sema, sync::INFINITE)
    }
    /// `WaitSemaTimeout(&sema, ms)`.
    WaitSemaTimeout @ 0x006f_2da0 => extern "C" fn wait_sema_timeout(sema: u32, ms: u32) -> u32 {
        sync::bridge::wait_sema(&mut mem(), &mut GameOs, sema, ms)
    }
    /// `Queue_PostRequest(request)`.
    Queue_PostRequest @ 0x0061_b500 => extern "thiscall" fn queue_post_request(this: u32, request: u32) -> u32 {
        sync::bridge::queue_post(&mut mem(), &mut GameOs, this, request)
    }
    /// `Request_Wait(request)` (`this` unused).
    Request_Wait @ 0x0061_ba60 => extern "thiscall" fn request_wait(_this: u32, request: u32) -> u32 {
        sync::bridge::request_wait(&mut mem(), &mut GameOs, request)
    }
    /// `MutexLock_Construct(mutex)`: returns `this`.
    MutexLock_Construct @ 0x0063_3a80 => extern "thiscall" fn mutex_lock_construct(this: u32, mutex: u32) -> u32 {
        sync::bridge::mutex_guard(&mut mem(), &mut GameOs, this, mutex)
    }
    /// `Semaphore_Construct(n)`: returns `this`.
    Semaphore_Construct @ 0x0067_9ab0 => extern "thiscall" fn semaphore_construct(this: u32, n: u32) -> u32 {
        sync::bridge::semaphore_ctor(&mut mem(), &mut GameOs, this, n)
    }
    /// `Worker_ResetCounters()`.
    Worker_ResetCounters @ 0x006d_b7a0 => extern "thiscall" fn worker_reset_counters(this: u32) -> u32 {
        sync::bridge::reset_counters(&mut mem(), &mut GameOs, this)
    }
    /// `Thread_JoinClose()`.
    Thread_JoinClose @ 0x0070_3a90 => extern "thiscall" fn thread_join_close(this: u32) -> u32 {
        sync::bridge::join_close(&mut mem(), &mut GameOs, this)
    }
    /// `Phyre::PClassDescriptorForType<RadialLineDynamicMesh>` destructor.
    PClassDescriptorForType_RadialLineDynamicMesh_Destruct @ 0x0067_5f30 => extern "thiscall" fn cd_dtor_00675f30(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b4_466c)
    }
    /// `Phyre::PClassDescriptorForType<BrokenScreenPolygonDynamicMeshInstance>` destructor.
    PClassDescriptorForType_BrokenScreenPolygonDynamicMeshInstance_Destruct @ 0x0067_6ba0 => extern "thiscall" fn cd_dtor_00676ba0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b4_483c)
    }
    /// `Phyre::PClassDescriptorForType<BrokenScreenPolygonDynamicMesh>` destructor.
    PClassDescriptorForType_BrokenScreenPolygonDynamicMesh_Destruct @ 0x0067_7590 => extern "thiscall" fn cd_dtor_00677590(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b4_49bc)
    }
    /// `Phyre::PClassDescriptorForType<ShadowDynamicMesh>` destructor.
    PClassDescriptorForType_ShadowDynamicMesh_Destruct @ 0x0067_82f0 => extern "thiscall" fn cd_dtor_006782f0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b4_4ba4)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PArray_Phyre_PInputs_PInputAction_4>` destructor.
    PClassDescriptorForType_Phyre_PArray_Phyre_PInputs_PInputAction_4_Destruct @ 0x009e_2160 => extern "thiscall" fn cd_dtor_009e2160(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_3a2c)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PArray_Phyre_PInputs_PInputSource_4>` destructor.
    PClassDescriptorForType_Phyre_PArray_Phyre_PInputs_PInputSource_4_Destruct @ 0x009e_2170 => extern "thiscall" fn cd_dtor_009e2170(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_38f4)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputAction>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputAction_Destruct @ 0x009e_2180 => extern "thiscall" fn cd_dtor_009e2180(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1fac)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputMap>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputMap_Destruct @ 0x009e_2190 => extern "thiscall" fn cd_dtor_009e2190(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_209c)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSource>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSource_Destruct @ 0x009e_21a0 => extern "thiscall" fn cd_dtor_009e21a0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_08ac)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceJoypadAxis>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceJoypadAxis_Destruct @ 0x009e_21b0 => extern "thiscall" fn cd_dtor_009e21b0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_0b54)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceJoypadButton>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceJoypadButton_Destruct @ 0x009e_21c0 => extern "thiscall" fn cd_dtor_009e21c0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_0a54)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceKey>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceKey_Destruct @ 0x009e_21d0 => extern "thiscall" fn cd_dtor_009e21d0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_095c)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMotionAngularVelocityX>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionAngularVelocityX_Destruct @ 0x009e_21e0 => extern "thiscall" fn cd_dtor_009e21e0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1964)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMotionAngularVelocityY>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionAngularVelocityY_Destruct @ 0x009e_21f0 => extern "thiscall" fn cd_dtor_009e21f0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1a6c)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMotionAngularVelocityZ>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionAngularVelocityZ_Destruct @ 0x009e_2200 => extern "thiscall" fn cd_dtor_009e2200(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1b74)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMotionLinearAccelerationX>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionLinearAccelerationX_Destruct @ 0x009e_2210 => extern "thiscall" fn cd_dtor_009e2210(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1c7c)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMotionLinearAccelerationY>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionLinearAccelerationY_Destruct @ 0x009e_2220 => extern "thiscall" fn cd_dtor_009e2220(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1d8c)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMotionLinearAccelerationZ>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionLinearAccelerationZ_Destruct @ 0x009e_2230 => extern "thiscall" fn cd_dtor_009e2230(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1e9c)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMotionQuatW>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatW_Destruct @ 0x009e_2240 => extern "thiscall" fn cd_dtor_009e2240(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1864)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMotionQuatX>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatX_Destruct @ 0x009e_2250 => extern "thiscall" fn cd_dtor_009e2250(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1564)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMotionQuatY>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatY_Destruct @ 0x009e_2260 => extern "thiscall" fn cd_dtor_009e2260(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1664)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMotionQuatZ>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatZ_Destruct @ 0x009e_2270 => extern "thiscall" fn cd_dtor_009e2270(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1764)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMouseButton>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMouseButton_Destruct @ 0x009e_2280 => extern "thiscall" fn cd_dtor_009e2280(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_0c54)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMouseDeltaX>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMouseDeltaX_Destruct @ 0x009e_2290 => extern "thiscall" fn cd_dtor_009e2290(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_0d54)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceMouseDeltaY>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceMouseDeltaY_Destruct @ 0x009e_22a0 => extern "thiscall" fn cd_dtor_009e22a0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_0e54)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceTouchDragX>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchDragX_Destruct @ 0x009e_22b0 => extern "thiscall" fn cd_dtor_009e22b0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1364)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceTouchDragY>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchDragY_Destruct @ 0x009e_22c0 => extern "thiscall" fn cd_dtor_009e22c0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1464)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceTouchPinch>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchPinch_Destruct @ 0x009e_22d0 => extern "thiscall" fn cd_dtor_009e22d0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1054)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceTouchRotate>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchRotate_Destruct @ 0x009e_22e0 => extern "thiscall" fn cd_dtor_009e22e0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_0f54)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceTouchTwoFingerDragX>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchTwoFingerDragX_Destruct @ 0x009e_22f0 => extern "thiscall" fn cd_dtor_009e22f0(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_1154)
    }
    /// `Phyre::PClassDescriptorForType<Phyre_PInputs_PInputSourceTouchTwoFingerDragY>` destructor.
    PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchTwoFingerDragY_Destruct @ 0x009e_2300 => extern "thiscall" fn cd_dtor_009e2300(this: u32) -> u32 {
        class_descriptor_dtor(this, 0x00b7_125c)
    }
    /// `TOAdpALPHA_AtCursor(...)`: `TOAdpALPHA` on the packet cursor.
    TOGsALPHA @ 0x008f_aa20 => extern "C" fn adp_cursor_008faa20(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f670, &[a0, a1, a2, a3, a4])
    }
    /// `TOAdpCLAMP_AtCursor(...)`: `TOAdpCLAMP` on the packet cursor.
    TOGsCLAMP @ 0x008f_aa50 => extern "C" fn adp_cursor_008faa50(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f6e0, &[a0, a1, a2, a3, a4, a5])
    }
    /// `TOAdpCOLCLAMP_AtCursor(...)`: `TOAdpCOLCLAMP` on the packet cursor.
    TOGsCOLCLAMP @ 0x008f_aa80 => extern "C" fn adp_cursor_008faa80(a0: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f760, &[a0])
    }
    /// `TOAdpGsDirectRGBA_AtCursor(...)`: `TOAdpGsDirectRGBA` on the packet cursor.
    TOGsGsDirectRGBA @ 0x008f_aaa0 => extern "C" fn adp_cursor_008faaa0(a0: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f790, &[a0])
    }
    /// `TOAdpFBA_AtCursor(...)`: `TOAdpFBA` on the packet cursor.
    TOGsFBA @ 0x008f_aac0 => extern "C" fn adp_cursor_008faac0(a0: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f7d0, &[a0])
    }
    /// `TOAdpFOG_AtCursor(...)`: `TOAdpFOG` on the packet cursor.
    TOGsFOG @ 0x008f_aae0 => extern "C" fn adp_cursor_008faae0(a0: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f800, &[a0])
    }
    /// `TOAdpFOGCOL_AtCursor(...)`: `TOAdpFOGCOL` on the packet cursor.
    TOGsFOGCOL @ 0x008f_ab00 => extern "C" fn adp_cursor_008fab00(a0: u32, a1: u32, a2: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f830, &[a0, a1, a2])
    }
    /// `TOAdpFRAME_AtCursor(...)`: `TOAdpFRAME` on the packet cursor.
    TOGsFRAME @ 0x008f_ab30 => extern "C" fn adp_cursor_008fab30(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f880, &[a0, a1, a2, a3])
    }
    /// `TOAdpGsDirectRGBA_AtCursor_008fab60(...)`: `TOAdpGsDirectRGBA` on the packet cursor.
    TOGsGsDirectRGBA_008fab60 @ 0x008f_ab60 => extern "C" fn adp_cursor_008fab60(a0: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f790, &[a0])
    }
    /// `TOAdpWords4_AtCursor(...)`: `TOAdpWords4` on the packet cursor.
    TOGsWords4 @ 0x008f_ab80 => extern "C" fn adp_cursor_008fab80(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f8e0, &[a0, a1, a2, a3])
    }
    /// `TOAdpSCISSOR_AtCursor(...)`: `TOAdpSCISSOR` on the packet cursor.
    TOGsSCISSOR @ 0x008f_abb0 => extern "C" fn adp_cursor_008fabb0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f910, &[a0, a1, a2, a3])
    }
    /// `TOAdpTEST_AtCursor(...)`: `TOAdpTEST` on the packet cursor.
    TOGsTEST @ 0x008f_abe0 => extern "C" fn adp_cursor_008fabe0(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f970, &[a0, a1, a2, a3, a4, a5, a6, a7])
    }
    /// `TOAdpTEX0_AtCursor(...)`: `TOAdpTEX0` on the packet cursor.
    TOGsTEX0 @ 0x008f_ac10 => extern "C" fn adp_cursor_008fac10(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fa00, &[a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11])
    }
    /// `TOAdpTEX1_AtCursor(...)`: `TOAdpTEX1` on the packet cursor.
    TOGsTEX1 @ 0x008f_ac50 => extern "C" fn adp_cursor_008fac50(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fad0, &[a0, a1, a2, a3, a4, a5, a6])
    }
    /// `TOAdpTEXFLUSH_AtCursor(...)`: `TOAdpTEXFLUSH` on the packet cursor.
    TOGsTEXFLUSH @ 0x008f_ac80 => extern "C" fn adp_cursor_008fac80() -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fb60, &[])
    }
    /// `TOAdpXY2_AtCursor(...)`: `TOAdpXY2` on the packet cursor.
    TOGsXY2 @ 0x008f_aca0 => extern "C" fn adp_cursor_008faca0(a0: u32, a1: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fb90, &[a0, a1])
    }
    /// `TOAdpXYFrac_AtCursor(...)`: `TOAdpXYFrac` on the packet cursor.
    TOGsXYFrac @ 0x008f_acc0 => extern "C" fn adp_cursor_008facc0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fbc0, &[a0, a1, a2, a3])
    }
    /// `TOAdpXYZ2Offset_AtCursor(...)`: `TOAdpXYZ2Offset` on the packet cursor.
    TOGsXYZ2Offset @ 0x008f_acf0 => extern "C" fn adp_cursor_008facf0(a0: u32, a1: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fbf0, &[a0, a1])
    }
    /// `TOAdpWords2Reg10_AtCursor(...)`: `TOAdpWords2Reg10` on the packet cursor.
    TOGsWords2Reg10 @ 0x008f_ad10 => extern "C" fn adp_cursor_008fad10(a0: u32, a1: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fc30, &[a0, a1])
    }
    /// `TOAdpXYOFFSET_AtCursor(...)`: `TOAdpXYOFFSET` on the packet cursor.
    TOGsXYOFFSET @ 0x008f_ad30 => extern "C" fn adp_cursor_008fad30(a0: u32, a1: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fc60, &[a0, a1])
    }
    /// `TOAdpXYZOffset_AtCursor(...)`: `TOAdpXYZOffset` on the packet cursor.
    TOGsXYZOffset @ 0x008f_ad50 => extern "C" fn adp_cursor_008fad50(a0: u32, a1: u32, a2: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fc90, &[a0, a1, a2])
    }
    /// `TOAdpWords3_AtCursor(...)`: `TOAdpWords3` on the packet cursor.
    TOGsWords3 @ 0x008f_ad80 => extern "C" fn adp_cursor_008fad80(a0: u32, a1: u32, a2: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fcd0, &[a0, a1, a2])
    }
    /// `TOAdpXYZWOffset_AtCursor(...)`: `TOAdpXYZWOffset` on the packet cursor.
    TOGsXYZWOffset @ 0x008f_adb0 => extern "C" fn adp_cursor_008fadb0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fd00, &[a0, a1, a2, a3])
    }
    /// `TOAdpZBUF_AtCursor(...)`: `TOAdpZBUF` on the packet cursor.
    TOGsZBUF @ 0x008f_ade0 => extern "C" fn adp_cursor_008fade0(a0: u32, a1: u32, a2: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fd40, &[a0, a1, a2])
    }
    /// `TOAdpALPHA_AtCursor_009023b0(...)`: `TOAdpALPHA` on the packet cursor.
    TOAdpALPHA_AtCursor_009023b0 @ 0x0090_23b0 => extern "C" fn adp_cursor_009023b0(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f670, &[a0, a1, a2, a3, a4])
    }
    /// `TOAdpCLAMP_AtCursor_009023e0(...)`: `TOAdpCLAMP` on the packet cursor.
    TOAdpCLAMP_AtCursor_009023e0 @ 0x0090_23e0 => extern "C" fn adp_cursor_009023e0(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f6e0, &[a0, a1, a2, a3, a4, a5])
    }
    /// `TOAdpCOLCLAMP_AtCursor_00902410(...)`: `TOAdpCOLCLAMP` on the packet cursor.
    TOAdpCOLCLAMP_AtCursor_00902410 @ 0x0090_2410 => extern "C" fn adp_cursor_00902410(a0: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f760, &[a0])
    }
    /// `TOAdpGsDirectRGBA_AtCursor_00902430(...)`: `TOAdpGsDirectRGBA` on the packet cursor.
    TOAdpGsDirectRGBA_AtCursor_00902430 @ 0x0090_2430 => extern "C" fn adp_cursor_00902430(a0: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f790, &[a0])
    }
    /// `TOAdpFBA_AtCursor_00902450(...)`: `TOAdpFBA` on the packet cursor.
    TOAdpFBA_AtCursor_00902450 @ 0x0090_2450 => extern "C" fn adp_cursor_00902450(a0: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f7d0, &[a0])
    }
    /// `TOAdpFRAME_AtCursor_00902470(...)`: `TOAdpFRAME` on the packet cursor.
    TOAdpFRAME_AtCursor_00902470 @ 0x0090_2470 => extern "C" fn adp_cursor_00902470(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f880, &[a0, a1, a2, a3])
    }
    /// `TOAdpWords4_AtCursor_009024a0(...)`: `TOAdpWords4` on the packet cursor.
    TOAdpWords4_AtCursor_009024a0 @ 0x0090_24a0 => extern "C" fn adp_cursor_009024a0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f8e0, &[a0, a1, a2, a3])
    }
    /// `TOAdpSCISSOR_AtCursor_009024d0(...)`: `TOAdpSCISSOR` on the packet cursor.
    TOAdpSCISSOR_AtCursor_009024d0 @ 0x0090_24d0 => extern "C" fn adp_cursor_009024d0(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f910, &[a0, a1, a2, a3])
    }
    /// `TOAdpTEST_AtCursor_00902500(...)`: `TOAdpTEST` on the packet cursor.
    TOAdpTEST_AtCursor_00902500 @ 0x0090_2500 => extern "C" fn adp_cursor_00902500(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_f970, &[a0, a1, a2, a3, a4, a5, a6, a7])
    }
    /// `TOAdpTEX0_AtCursor_00902530(...)`: `TOAdpTEX0` on the packet cursor.
    TOAdpTEX0_AtCursor_00902530 @ 0x0090_2530 => extern "C" fn adp_cursor_00902530(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fa00, &[a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11])
    }
    /// `TOAdpTEX1_AtCursor_00902570(...)`: `TOAdpTEX1` on the packet cursor.
    TOAdpTEX1_AtCursor_00902570 @ 0x0090_2570 => extern "C" fn adp_cursor_00902570(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fad0, &[a0, a1, a2, a3, a4, a5, a6])
    }
    /// `TOAdpXY2_AtCursor_009025c0(...)`: `TOAdpXY2` on the packet cursor.
    TOAdpXY2_AtCursor_009025c0 @ 0x0090_25c0 => extern "C" fn adp_cursor_009025c0(a0: u32, a1: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fb90, &[a0, a1])
    }
    /// `TOAdpXYZ2Offset_AtCursor_009025e0(...)`: `TOAdpXYZ2Offset` on the packet cursor.
    TOAdpXYZ2Offset_AtCursor_009025e0 @ 0x0090_25e0 => extern "C" fn adp_cursor_009025e0(a0: u32, a1: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fbf0, &[a0, a1])
    }
    /// `TOAdpXYOFFSET_AtCursor_00902600(...)`: `TOAdpXYOFFSET` on the packet cursor.
    TOAdpXYOFFSET_AtCursor_00902600 @ 0x0090_2600 => extern "C" fn adp_cursor_00902600(a0: u32, a1: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fc60, &[a0, a1])
    }
    /// `TOAdpXYZOffset_AtCursor_00902620(...)`: `TOAdpXYZOffset` on the packet cursor.
    TOAdpXYZOffset_AtCursor_00902620 @ 0x0090_2620 => extern "C" fn adp_cursor_00902620(a0: u32, a1: u32, a2: u32) -> u32 {
        pkt::bridge::adp_at_cursor(&mut mem(), 0x008e_fc90, &[a0, a1, a2])
    }
    /// `std::string::_Tidy(true)` copy.
    std_string_dtor @ 0x0040_2da0 => extern "thiscall" fn string_tidy_00402da0(this: u32) {
        container::bridge::string_tidy(&mut mem(), this, delete);
    }
    /// `std::string::_Tidy(true)` copy.
    StdString_Tidy_00634410 @ 0x0063_4410 => extern "thiscall" fn string_tidy_00634410(this: u32) {
        container::bridge::string_tidy(&mut mem(), this, delete);
    }
    /// `std::string::_Tidy(true)` copy.
    StdString_Tidy_00634440 @ 0x0063_4440 => extern "thiscall" fn string_tidy_00634440(this: u32) {
        container::bridge::string_tidy(&mut mem(), this, delete);
    }
    /// `std::vector::_Tidy` copy.
    StdVector_Tidy_00634470 @ 0x0063_4470 => extern "thiscall" fn vector_tidy_00634470(this: u32) {
        container::bridge::vector_tidy(&mut mem(), this, delete);
    }
    /// `std::vector::_Tidy` copy.
    StdVector_Tidy_006344a0 @ 0x0063_44a0 => extern "thiscall" fn vector_tidy_006344a0(this: u32) {
        container::bridge::vector_tidy(&mut mem(), this, delete);
    }
    /// `std::vector::_Tidy` copy.
    StdVector_Tidy_006344d0 @ 0x0063_44d0 => extern "thiscall" fn vector_tidy_006344d0(this: u32) {
        container::bridge::vector_tidy(&mut mem(), this, delete);
    }
    /// Phyre owned array reset.
    PArray_Reset_00633c40 @ 0x0063_3c40 => extern "thiscall" fn owned_reset_00633c40(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, false, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_00633c70 @ 0x0063_3c70 => extern "thiscall" fn owned_reset_00633c70(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, false, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_00633ca0 @ 0x0063_3ca0 => extern "thiscall" fn owned_reset_00633ca0(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, false, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_00633cd0 @ 0x0063_3cd0 => extern "thiscall" fn owned_reset_00633cd0(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, false, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_00633d00 @ 0x0063_3d00 => extern "thiscall" fn owned_reset_00633d00(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, false, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_00633d30 @ 0x0063_3d30 => extern "thiscall" fn owned_reset_00633d30(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, false, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_00633d60 @ 0x0063_3d60 => extern "thiscall" fn owned_reset_00633d60(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, false, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_00633d90 @ 0x0063_3d90 => extern "thiscall" fn owned_reset_00633d90(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, true, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_0067b1d0 @ 0x0067_b1d0 => extern "thiscall" fn owned_reset_0067b1d0(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, false, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_0067b200 @ 0x0067_b200 => extern "thiscall" fn owned_reset_0067b200(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, true, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_006895c0 @ 0x0068_95c0 => extern "thiscall" fn owned_reset_006895c0(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, true, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_006895f0 @ 0x0068_95f0 => extern "thiscall" fn owned_reset_006895f0(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, true, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_006fbca0 @ 0x006f_bca0 => extern "thiscall" fn owned_reset_006fbca0(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, true, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_006fbcd0 @ 0x006f_bcd0 => extern "thiscall" fn owned_reset_006fbcd0(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, true, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_009e1d70 @ 0x009e_1d70 => extern "thiscall" fn owned_reset_009e1d70(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, false, phyre_free_cb);
    }
    /// Phyre owned array reset.
    PArray_Reset_009e1da0 @ 0x009e_1da0 => extern "thiscall" fn owned_reset_009e1da0(this: u32) {
        container::bridge::owned_reset(&mut mem(), this, false, phyre_free_cb);
    }
    /// `0x0069a130(items, n)`.
    FreeOwnedArray_0069a130 @ 0x0069_a130 => extern "C" fn free_owned_array_0069a130(items: u32, n: u32) {
        container::bridge::free_owned_array(&mut mem(), items, n, phyre_free_cb);
    }
    /// `std::list<std::string>::clear`.
    StdStringList_Clear @ 0x0068_6da0 => extern "thiscall" fn string_list_clear(this: u32) {
        container::bridge::string_list_clear(&mut mem(), this, delete);
    }
    /// A `std::map` head node.
    StdMap_NewHead_0069a1b0 @ 0x0069_a1b0 => extern "C" fn map_head_new_0069a1b0() -> u32 {
        container::bridge::map_head_new(
            &mut mem(),
            |m, size| heap::user_malloc(m, &mut GameOs, size),
            |_| {
                // SAFETY: std::_Xbad_alloc through its MSVCP110 import slot; it throws.
                unsafe {
                    let f: unsafe extern "C" fn() -> u32 = std::mem::transmute(GameOs::import(0x00b0_c2a0));
                    f()
                }
            },
        )
    }
    /// `0x0061c270()`.
    Parser_Reset_0061c270 @ 0x0061_c270 => extern "thiscall" fn parser_reset_0061c270(this: u32) {
        container::bridge::parser_reset(&mut mem(), this, delete);
    }
    /// `TOAllClearMessageWindow()`.
    TOAllClearMessageWindow @ 0x008a_b590 => extern "C" fn to_all_clear_message_window() -> u32 {
        window::bridge::all_clear_message_windows(&mut mem())
    }
    /// `TOOpenWindowAndMessage(i)`.
    TOOpenWindowAndMessage @ 0x008a_de10 => extern "C" fn to_open_window_and_message(i: i32) -> u32 {
        window::bridge::open_window_and_message(&mut mem(), i, call_ptr)
    }
    /// `TOMenuCloseYesNoWindow()`.
    TOMenuCloseYesNoWindow @ 0x008e_2de0 => extern "C" fn to_menu_close_yes_no_window() -> u32 {
        window::bridge::menu_close_yes_no_window(&mut mem())
    }
    /// `TkMenuAddHPCapped(chr, delta)`.
    TkMenuAddHPCapped @ 0x008c_8d10 => extern "C" fn tk_menu_add_hp_capped(chr: u32, delta: u32) -> u32 {
        menu::bridge::add_hp_capped(
            &mut mem(),
            chr,
            delta,
            |m, c| ffx_menu::tk::bridge::tk_menu_get_ply(m, c, 0x1c),
            |m, c| ffx_menu::tk::bridge::tk_menu_get_ply(m, c, 0x24),
            ffx_menu::tk::bridge::tk_menu_set_hp,
        )
    }
    /// `TOMenuGetControlPadAnalogHist(pad, port, k, i)`.
    TOMenuGetControlPadAnalogHist @ 0x008b_e450 => extern "C" fn to_menu_get_control_pad_analog_hist(pad: u32, port: u32, k: u32, i: u32) -> u32 {
        menu::bridge::control_pad_analog_hist(
            &mut mem(),
            [pad, port, k, i],
            |m| ffx_menu::tk::bridge::tk_menu_get_dummy_controler_flag(m),
            |m, [pad, port, k, i]| {
                ffx_menu::tk::bridge::tk_get_controler_analog_hist(m, pad.cast_signed(), port.cast_signed(), k, i.cast_signed())
            },
        )
    }
    /// `GetSaveProductCode()`: by `TkGetLang()`.
    GetSaveProductCode @ 0x0077_9ca0 => extern "C" fn get_save_product_code() -> u32 {
        let m = mem();
        menu::bridge::save_product_code(&m, ffx_menu::tk::bridge::tk_get_lang(&m))
    }
    /// `TOSwapInternationalEnvInit(english)`.
    TOSwapInternationalEnvInit @ 0x008b_0920 => extern "C" fn to_swap_international_env_init(english: u32) {
        menu::bridge::swap_international_env_init(&mut mem(), english, |m, e| call_sym(m, 0x0078_5ed0, &[e]));
    }
    /// `TOSkipCodesAbove2(p)`.
    TOSkipCodesAbove2 @ 0x008b_7f40 => extern "C" fn to_skip_codes_above_2(p: u32) -> u32 {
        msg::bridge::skip_codes_above_2(&mem(), p)
    }
    /// `TODVDFileLoad(file, name)`.
    TODVDFileLoad @ 0x008a_ba60 => extern "C" fn to_dvd_file_load(file: u32, name: u32) -> u32 {
        files::bridge::dvd_file_load(&mut mem(), file, name, call_ptr, |m, size| heap::user_malloc(m, &mut GameOs, size))
    }
    /// `TODVDFileStartRead(file, name, buf)`.
    TODVDFileStartRead @ 0x008a_bab0 => extern "C" fn to_dvd_file_start_read(file: u32, name: u32, buf: u32) -> u32 {
        files::bridge::dvd_file_start_read(&mut mem(), file, name, buf, call_ptr)
    }
    /// `RatioSquared(a, b)`.
    RatioSquared @ 0x008f_38c0 => extern "C" fn ratio_squared(a: i32, b: i32) -> f32 {
        math::bridge::ratio_squared(a, b)
    }
    /// `Counters_Flush(k)`.
    Counters_Flush @ 0x0082_1a40 => extern "C" fn counters_flush(k: i32) -> u32 {
        save::bridge::counters_flush(&mut mem(), k)
    }
    /// `Counters_Snapshot()`.
    Counters_Snapshot @ 0x0082_1a70 => extern "C" fn counters_snapshot() -> u32 {
        save::bridge::counters_snapshot(&mut mem())
    }
    /// `std::_Adjust_heap(first, hole, bottom, &val, less)`, 16-byte records.
    StdAdjustHeap16 @ 0x0060_aeb0 => extern "C" fn sort16_0060aeb0(first: u32, hole: i32, bottom: i32, val: u32, less: u32) {
        with_sorter(16, less, |m, s| s.adjust_heap(m, first, hole, bottom, val));
    }
    /// `std::_Insertion_sort(first, last, less)`, 16-byte records.
    StdInsertionSort16 @ 0x0060_b0f0 => extern "C" fn sort16_0060b0f0(first: u32, last: u32, less: u32) {
        with_sorter(16, less, |m, s| s.insertion_sort(m, first, last));
    }
    /// `std::_Insertion_sort(first, last, less)`, 16-byte records.
    StdInsertionSort16_0060b2d0 @ 0x0060_b2d0 => extern "C" fn sort16_0060b2d0(first: u32, last: u32, less: u32) {
        with_sorter(16, less, |m, s| s.insertion_sort(m, first, last));
    }
    /// `std::_Make_heap(first, last, less)`, 16-byte records.
    StdMakeHeap16 @ 0x0060_b310 => extern "C" fn sort16_0060b310(first: u32, last: u32, less: u32) {
        with_sorter(16, less, |m, s| s.make_heap(m, first, last));
    }
    /// `std::_Med3(a, b, c, less)`, 16-byte records.
    StdMed316 @ 0x0060_b550 => extern "C" fn sort16_0060b550(a: u32, b: u32, c: u32, less: u32) {
        with_sorter(16, less, |m, s| s.med3(m, a, b, c));
    }
    /// `std::_Median(first, mid, last, less), last inclusive; median of nine above 40 records`, 16-byte records.
    StdMedian16 @ 0x0060_b6f0 => extern "C" fn sort16_0060b6f0(first: u32, mid: u32, last: u32, less: u32) {
        with_sorter(16, less, |m, s| s.median(m, first, mid, last));
    }
    /// `std::_Pop_heap(first, last, dest, &val, less)`, 16-byte records.
    StdPopHeapTo16 @ 0x0060_b950 => extern "C" fn sort16_0060b950(first: u32, last: u32, dest: u32, val: u32, less: u32) {
        with_sorter(16, less, |m, s| s.pop_heap_to(m, first, last, dest, val));
    }
    /// `std::_Pop_heap(first, last, less): top to the last record`, 16-byte records.
    StdPopHeap16 @ 0x0060_b990 => extern "C" fn sort16_0060b990(first: u32, last: u32, less: u32) {
        with_sorter(16, less, |m, s| s.pop_heap(m, first, last));
    }
    /// `std::_Pop_heap(first, last, less): top to the last record`, 16-byte records.
    StdPopHeap16_0060baa0 @ 0x0060_baa0 => extern "C" fn sort16_0060baa0(first: u32, last: u32, less: u32) {
        with_sorter(16, less, |m, s| s.pop_heap(m, first, last));
    }
    /// `std::_Push_heap(first, hole, top, &val, less)`, 16-byte records.
    StdPushHeap16 @ 0x0060_bb80 => extern "C" fn sort16_0060bb80(first: u32, hole: i32, top: i32, val: u32, less: u32) {
        with_sorter(16, less, |m, s| s.push_heap(m, first, hole, top, val));
    }
    /// `std::_Sort(first, last, ideal, less): introsort, 32-record insertion cutoff`, 16-byte records.
    StdSort16 @ 0x0060_bc90 => extern "C" fn sort16_0060bc90(first: u32, last: u32, ideal: i32, less: u32) {
        with_sorter(16, less, |m, s| s.sort(m, first, last, ideal));
    }
    /// `std::sort_heap(first, last, less)`, 16-byte records.
    StdSortHeap16 @ 0x0060_bed0 => extern "C" fn sort16_0060bed0(first: u32, last: u32, less: u32) {
        with_sorter(16, less, |m, s| s.sort_heap(m, first, last));
    }
    /// `std::_Unguarded_partition(&result, first, last, less) -> &result (pfirst, plast)`, 16-byte records.
    StdUnguardedPartition16 @ 0x0060_c030 => extern "C" fn sort16_0060c030(result: u32, first: u32, last: u32, less: u32) -> u32 {
        let (lo, hi) = with_sorter(16, less, |m, s| s.partition(m, first, last));
        mem().write(result, [lo, hi]);
        result
    }
    /// `std::_Adjust_heap(first, hole, bottom, &val, less)`, 28-byte records.
    StdAdjustHeap28 @ 0x0060_afc0 => extern "C" fn sort28_0060afc0(first: u32, hole: i32, bottom: i32, val: u32, less: u32) {
        with_sorter(28, less, |m, s| s.adjust_heap(m, first, hole, bottom, val));
    }
    /// `std::_Insertion_sort(first, last, less)`, 28-byte records.
    StdInsertionSort28 @ 0x0060_b1f0 => extern "C" fn sort28_0060b1f0(first: u32, last: u32, less: u32) {
        with_sorter(28, less, |m, s| s.insertion_sort(m, first, last));
    }
    /// `std::_Insertion_sort(first, last, less)`, 28-byte records.
    StdInsertionSort28_0060b2f0 @ 0x0060_b2f0 => extern "C" fn sort28_0060b2f0(first: u32, last: u32, less: u32) {
        with_sorter(28, less, |m, s| s.insertion_sort(m, first, last));
    }
    /// `std::_Make_heap(first, last, less)`, 28-byte records.
    StdMakeHeap28 @ 0x0060_b420 => extern "C" fn sort28_0060b420(first: u32, last: u32, less: u32) {
        with_sorter(28, less, |m, s| s.make_heap(m, first, last));
    }
    /// `std::_Med3(a, b, c, less)`, 28-byte records.
    StdMed328 @ 0x0060_b640 => extern "C" fn sort28_0060b640(a: u32, b: u32, c: u32, less: u32) {
        with_sorter(28, less, |m, s| s.med3(m, a, b, c));
    }
    /// `std::_Median(first, mid, last, less), last inclusive; median of nine above 40 records`, 28-byte records.
    StdMedian28 @ 0x0060_b790 => extern "C" fn sort28_0060b790(first: u32, mid: u32, last: u32, less: u32) {
        with_sorter(28, less, |m, s| s.median(m, first, mid, last));
    }
    /// `std::_Pop_heap(first, last, dest, &val, less)`, 28-byte records.
    StdPopHeapTo28 @ 0x0060_b9f0 => extern "C" fn sort28_0060b9f0(first: u32, last: u32, dest: u32, val: u32, less: u32) {
        with_sorter(28, less, |m, s| s.pop_heap_to(m, first, last, dest, val));
    }
    /// `std::_Pop_heap(first, last, less): top to the last record`, 28-byte records.
    StdPopHeap28 @ 0x0060_ba40 => extern "C" fn sort28_0060ba40(first: u32, last: u32, less: u32) {
        with_sorter(28, less, |m, s| s.pop_heap(m, first, last));
    }
    /// `std::_Pop_heap(first, last, less): top to the last record`, 28-byte records.
    StdPopHeap28_0060bb00 @ 0x0060_bb00 => extern "C" fn sort28_0060bb00(first: u32, last: u32, less: u32) {
        with_sorter(28, less, |m, s| s.pop_heap(m, first, last));
    }
    /// `std::_Push_heap(first, hole, top, &val, less)`, 28-byte records.
    StdPushHeap28 @ 0x0060_bc00 => extern "C" fn sort28_0060bc00(first: u32, hole: i32, top: i32, val: u32, less: u32) {
        with_sorter(28, less, |m, s| s.push_heap(m, first, hole, top, val));
    }
    /// `std::_Sort(first, last, ideal, less): introsort, 32-record insertion cutoff`, 28-byte records.
    StdSort28 @ 0x0060_bd80 => extern "C" fn sort28_0060bd80(first: u32, last: u32, ideal: i32, less: u32) {
        with_sorter(28, less, |m, s| s.sort(m, first, last, ideal));
    }
    /// `std::sort_heap(first, last, less)`, 28-byte records.
    StdSortHeap28 @ 0x0060_bf60 => extern "C" fn sort28_0060bf60(first: u32, last: u32, less: u32) {
        with_sorter(28, less, |m, s| s.sort_heap(m, first, last));
    }
    /// `std::_Unguarded_partition(&result, first, last, less) -> &result (pfirst, plast)`, 28-byte records.
    StdUnguardedPartition28 @ 0x0060_c390 => extern "C" fn sort28_0060c390(result: u32, first: u32, last: u32, less: u32) -> u32 {
        let (lo, hi) = with_sorter(28, less, |m, s| s.partition(m, first, last));
        mem().write(result, [lo, hi]);
        result
    }
    /// `std::sort(first, last, less)`, 16-byte records.
    StdSort16_std @ 0x0060_c890 => extern "C" fn std_sort16(first: u32, last: u32, less: u32) {
        with_sorter(16, less, |m, s| s.std_sort(m, first, last));
    }
    /// `std::sort(first, last, less)`, 28-byte records.
    StdSort28_std @ 0x0060_c8b0 => extern "C" fn std_sort28(first: u32, last: u32, less: u32) {
        with_sorter(28, less, |m, s| s.std_sort(m, first, last));
    }
    /// Phyre array append into a new buffer.
    PArray_PushInto_00629b90 @ 0x0062_9b90 => extern "thiscall" fn parray_push_into_00629b90(this: u32, dst: u32, value: u32) -> u32 {
        container::bridge::parray_push_into(&mut mem(), this, dst, value, phyre_free_cb)
    }
    /// Phyre array append into a new buffer.
    PArray_PushInto_0069b530 @ 0x0069_b530 => extern "thiscall" fn parray_push_into_0069b530(this: u32, dst: u32, value: u32) -> u32 {
        container::bridge::parray_push_into(&mut mem(), this, dst, value, phyre_free_cb)
    }
    /// Phyre array append into a new buffer.
    PArray_PushInto_0069b5c0 @ 0x0069_b5c0 => extern "thiscall" fn parray_push_into_0069b5c0(this: u32, dst: u32, value: u32) -> u32 {
        container::bridge::parray_push_into(&mut mem(), this, dst, value, phyre_free_cb)
    }
    /// Phyre array append into a new buffer.
    PArray_PushInto_0069b650 @ 0x0069_b650 => extern "thiscall" fn parray_push_into_0069b650(this: u32, dst: u32, value: u32) -> u32 {
        container::bridge::parray_push_into(&mut mem(), this, dst, value, phyre_free_cb)
    }
    /// `0x00689620()`: two owned arrays reset.
    ResetArrays_00689620 @ 0x0068_9620 => extern "thiscall" fn reset_arrays_00689620(this: u32) {
        container::bridge::reset_arrays_689620(&mut mem(), this, phyre_free_cb);
    }
    /// `0x009e2420()`: a Phyre array of objects destroyed.
    PArray_DeleteAll_009e2420 @ 0x009e_2420 => extern "thiscall" fn parray_delete_all_009e2420(this: u32) {
        container::bridge::parray_delete_all(
            &mut mem(),
            this,
            |m, obj| {
                // SAFETY: the object's scalar deleting destructor (vtable
                // slot 0, thiscall, flag 1), as the original calls it.
                unsafe {
                    let vtable: u32 = m.read(obj);
                    let f: u32 = m.read(vtable);
                    let f: unsafe extern "thiscall" fn(u32, u32) -> u32 = std::mem::transmute(f as usize);
                    f(obj, 1);
                }
            },
            phyre_free_cb,
        );
    }
    /// `TOSeqSetScenePosByName(i, name, x, y)`.
    TOSeqSetScenePosByName @ 0x008a_f060 => extern "C" fn to_seq_set_scene_pos_by_name(i: i32, name: u32, x: u32, y: u32) {
        #[allow(clippy::cast_possible_truncation)] // the menu takes u16 positions
        scene::bridge::set_scene_pos_by_name(&mut mem(), i, name, x, y, |m, i, h, x, y| {
            ffx_menu::scene::bridge::tk_menu_set_scene_pos(m, i, h, x as u16, y as u16);
        });
    }
    /// `ScreenProjection(m)`.
    ScreenProjection @ 0x0076_6ae0 => extern "C" fn screen_projection(m: u32) {
        math::bridge::screen_projection(&mut mem(), m, |mm, p| {
            ffx_ps2::vu0::bridge::matrix_identity(mm, p);
        });
    }
    /// `TOGetFFXLangStr()`.
    TOGetFFXLangStr @ 0x008a_c300 => extern "C" fn to_get_ffx_lang_str() -> u32 {
        let m = mem();
        let lang = instance::bridge::ffx_lang(&m, &mut phyre_printf);
        menu::bridge::ffx_lang_str(&m, lang)
    }
    /// `TOSwapInternationalEnvInit_008b0970(english)`.
    TOSwapInternationalEnvInit_008b0970 @ 0x008b_0970 => extern "C" fn to_swap_international_env_init_008b0970(english: u32) {
        menu::bridge::swap_international_env_init2(&mut mem(), english, |m, e| call_sym(m, 0x0078_5ed0, &[e]));
    }
    /// `TOMenuAtelReq(work, entry, level)`.
    TOMenuAtelReq @ 0x008b_e370 => extern "C" fn to_menu_atel_req(work: u32, entry: u32, level: u32) -> u32 {
        use ffx_script::vm::bridge as atel;
        menu::bridge::menu_atel_req(
            &mut mem(),
            [work, entry, level],
            atel::atel_change_current_ctrl,
            |m, [from, to, kind, lvl, ent]| {
                atel::signal::atel_regist_signal_without_same_tag(m, atel::signal::Request::new(from, to, kind, lvl, ent))
            },
            atel::atel_set_current_ctrl,
        )
    }
    /// `TOSeqGetColorRGBA(i, name, &r, &g, &b, &a)`.
    TOSeqGetColorRGBA @ 0x008a_e3a0 => extern "C" fn to_seq_get_color_rgba(i: i32, name: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
        scene::bridge::get_color_rgba(
            &mut mem(),
            i,
            name,
            [r, g, b, a],
            |m, i, n| ffx_menu::scene::bridge::tk_menu_get_param_handle(m, i, n),
            |m, i, h, n| ffx_menu::scene::bridge::tk_menu_get_param_offset(m, i, h.cast_signed(), n.cast_signed()),
        )
    }
    /// `TOShiftListedId(id)`.
    TOShiftListedId @ 0x0090_66f0 => extern "C" fn to_shift_listed_id(id: i32) -> i32 {
        msg::bridge::shift_listed_id(&mem(), id)
    }
    /// `TOBtlSetDamageValue(a, b, c, d, e, f, g, h)`.
    TOBtlSetDamageValue @ 0x0089_9a40 => extern "C" fn to_btl_set_damage_value(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32, h: u32) -> i32 {
        btl::bridge::set_damage_value(&mut mem(), [a, b, c, d, e, f, g, h])
    }
    /// `TOBtlMakeTargetListBit()`.
    TOBtlMakeTargetListBit @ 0x0089_85b0 => extern "C" fn to_btl_make_target_list_bit() -> i32 {
        btl::bridge::make_target_list_bit(&mut mem())
    }
    /// `TOControlMessageWindowForMenu()`.
    TOControlMessageWindowForMenu @ 0x008a_b9a0 => extern "C" fn to_control_message_window_for_menu() -> u32 {
        window::bridge::control_message_window_for_menu(&mut mem(), call_ptr)
    }
    /// `TOSetPktSide(x)`.
    TOSetPktSide_009047a0 @ 0x0090_47a0 => extern "C" fn to_set_pkt_side(x: u32) -> u32 {
        tolib::bridge::set_pkt_side(&mut mem(), x)
    }
    /// `TOMenuAtelRelease()`.
    TOMenuAtelRelease_008be350 @ 0x008b_e350 => extern "C" fn to_menu_atel_release() {
        tolib::bridge::menu_atel_release(&mut mem(), ffx_script::vm::bridge::atel_remove_ctrl_work);
    }
    /// `TOBtlPadScrollUp(w)`.
    TOBtlPadScrollUp @ 0x0089_8e20 => extern "C" fn to_btl_pad_scroll_up(w: u32) -> u32 {
        tolib::bridge::btl_pad_scroll(&mut mem(), w, stub::LINE_FEED_HEIGHT, false)
    }
    /// `TOBtlPadScrollDown(w)`.
    TOBtlPadScrollDown @ 0x0089_8df0 => extern "C" fn to_btl_pad_scroll_down(w: u32) -> u32 {
        tolib::bridge::btl_pad_scroll(&mut mem(), w, stub::LINE_FEED_HEIGHT, true)
    }
    /// `0x008a2ba0(p)`.
    Tick4_008a2ba0 @ 0x008a_2ba0 => extern "C" fn tick4_008a2ba0(p: u32) -> u32 {
        tolib::bridge::tick4(&mut mem(), p)
    }
    /// `0x008dfe70(w)`.
    MessageHold_008dfe70 @ 0x008d_fe70 => extern "C" fn message_hold_008dfe70(w: u32) -> u32 {
        tolib::bridge::message_hold(&mut mem(), w)
    }
    /// `TOInitPadData()`.
    TOInitPadData @ 0x008a_dbd0 => extern "C" fn to_init_pad_data() -> u32 {
        tolib::bridge::init_pad_data(&mut mem())
    }
    /// `TOMenuClearControlPadBuff()`.
    TOMenuClearControlPadBuff @ 0x008b_e3d0 => extern "C" fn to_menu_clear_control_pad_buff() -> u32 {
        tolib::bridge::init_pad_data(&mut mem())
    }
    /// `TOMenuGetDBuffDC(&out)`.
    TOMenuGetDBuffDC @ 0x008a_8e30 => extern "C" fn to_menu_get_dbuff_dc(out: u32) -> u32 {
        tolib::bridge::menu_get_dbuff_dc(&mut mem(), out, ffx_sg::state::bridge::get_dbuff_dc)
    }
    /// `TOInitMenuGraph()`.
    TOInitMenuGraph @ 0x008b_e330 => extern "C" fn to_init_menu_graph() -> u32 {
        tolib::bridge::init_menu_graph(&mut mem(), ffx_sg::state::bridge::get_dbuff_dc)
    }
    /// `TOMenuOpenPktBuffTmp()`: `TOOpenPktbuff` (`0x00904600`) not ported.
    TOMenuOpenPktBuffTmp @ 0x008b_ef30 => extern "C" fn to_menu_open_pkt_buff_tmp() -> u32 {
        tolib::bridge::menu_open_pkt_buff_tmp(&mut mem(), |m| call_sym(m, 0x0090_4600, &[]))
    }
    /// `TOCloseHelpWindow()`.
    TOCloseHelpWindow_008b3790 @ 0x008b_3790 => extern "C" fn to_close_help_window() -> u32 {
        tolib::bridge::close_help_window(&mut mem())
    }
    /// `TOGetCommandName(id)` (`MsGetComData`).
    TOGetComName_008b8d70 @ 0x008b_8d70 => extern "C" fn to_get_com_name(id: u32) -> u32 {
        table_name(id, 0, false, ffx_battle::stat::bridge::ms_get_com_data)
    }
    /// `TOGetAbilityName(id)` (`MsGetRomAbility`).
    TOGetAbilityName_008b8d90 @ 0x008b_8d90 => extern "C" fn to_get_ability_name(id: u32) -> u32 {
        table_name(id, 0, false, |m, id, t| ffx_kernel::bridge::rom(m, ffx_kernel::bridge::A_ABILITY, id, t))
    }
    /// `MsGetRomPlyCommand` name (kana-aware).
    TOMenuGetGetCommandName @ 0x008b_ebb0 => extern "C" fn to_get_ply_command_name(id: u32) -> u32 {
        table_name(id, 0, true, |m, id, t| ffx_kernel::bridge::rom(m, ffx_kernel::bridge::COMMAND, id, t))
    }
    /// `MsGetRomPlyCommand` help text (kana-aware).
    TOMenuGetGetCommandHelp @ 0x008b_eb70 => extern "C" fn to_get_ply_command_help(id: u32) -> u32 {
        table_name(id, 8, true, |m, id, t| ffx_kernel::bridge::rom(m, ffx_kernel::bridge::COMMAND, id, t))
    }
    /// `MsImportantPtr` name (kana-aware).
    TOMenuGetGetImportantName @ 0x008b_ec30 => extern "C" fn to_get_important_name(id: u32) -> u32 {
        table_name(id, 0, true, ffx_battle::save::bridge::ms_important_ptr)
    }
    /// `MsImportantPtr` help text (kana-aware).
    TOMenuGetGetImportantHelp @ 0x008b_ebf0 => extern "C" fn to_get_important_help(id: u32) -> u32 {
        table_name(id, 8, true, ffx_battle::save::bridge::ms_important_ptr)
    }
    /// `0x008b3730(_, i)`.
    TOSaveSlotShown_008b3730 @ 0x008b_3730 => extern "C" fn to_save_slot_shown(_x: u32, i: u32) -> u32 {
        tolib::bridge::save_slot_shown(&mem(), i)
    }
    /// `0x008b7f80(p)`.
    TOCountLineBreaks_008b7f80 @ 0x008b_7f80 => extern "C" fn to_count_line_breaks(p: u32) -> u32 {
        tolib::bridge::count_line_breaks(&mem(), p)
    }
    /// `0x008df690(p, n)`.
    TOSkipCountedCodes_008df690 @ 0x008d_f690 => extern "C" fn to_skip_counted_codes(p: u32, n: u32) -> u32 {
        tolib::bridge::skip_counted_codes(&mem(), p, n)
    }
    /// `0x008b89a0(i, text)`.
    TOSetMessageText_008b89a0 @ 0x008b_89a0 => extern "C" fn to_set_message_text(i: i32, text: u32) -> u32 {
        tolib::bridge::set_message_text(&mut mem(), i, text)
    }
    /// `0x00890e30(tag, end, flags)`.
    TOBtlPacketTag_00890e30 @ 0x0089_0e30 => extern "C" fn to_btl_packet_tag(tag: u32, end: u32, flags: u32) -> u32 {
        tolib::bridge::gif_tag(&mut mem(), tag, end, flags)
    }
    /// `0x008a8f00(bits, i)`.
    TOBit16_008a8f00 @ 0x008a_8f00 => extern "C" fn to_bit16(bits: u32, i: i32) -> u32 {
        tolib::bridge::bit16(&mem(), bits, i)
    }
    /// `TOSeqHasScene(i, name)`.
    TOSeqHasScene_008ae000 @ 0x008a_e000 => extern "C" fn to_seq_has_scene(i: i32, name: u32) -> u32 {
        scene::bridge::has_scene(&mem(), i, name)
    }
    /// `TOResetMessageVars(i)`.
    TOResetMessageVars_008b8680 @ 0x008b_8680 => extern "C" fn to_reset_message_vars(i: i32) -> u32 {
        tolib::bridge::reset_message_vars(&mut mem(), i)
    }
    /// `TOInitMessage(i)`.
    TOInitMessage @ 0x008b_8210 => extern "C" fn to_init_message(i: i32) -> u32 {
        tolib::bridge::init_message(&mut mem(), i)
    }
    /// `TOSelectMacroDic()`.
    TOSelectMacroDic_008b56d0 @ 0x008b_56d0 => extern "C" fn to_select_macro_dic() -> u32 {
        let m = &mut mem();
        let lang = instance::bridge::ffx_lang(m, &mut phyre_printf);
        tolib::bridge::select_macro_dic(m, lang)
    }
    /// `TOJoinsNeighbours(p)`.
    TOJoinsNeighbours_008b5760 @ 0x008b_5760 => extern "C" fn to_joins_neighbours(p: u32) -> u32 {
        let m = mem();
        let lang = instance::bridge::ffx_lang(&m, &mut phyre_printf);
        tolib::bridge::joins_neighbours(&m, p, lang)
    }
    /// `TOBtlInitLimitTimer()`.
    TOBtlInitLimitTimer @ 0x0089_7f00 => extern "C" fn to_btl_init_limit_timer() -> u32 {
        tolib::bridge::btl_init_limit_timer(
            &mut mem(),
            |m, i, n, x, y| {
                scene::bridge::get_point(m, i.cast_signed(), n, [x, y], menu_scene::tk_menu_get_param_handle, menu_scene::tk_menu_get_param_offset);
            },
            limit_sound,
        )
    }
    /// `TOLoadMenuOpen(menu)`.
    TOLoadMenuOpen_008b3510 @ 0x008b_3510 => extern "C" fn to_load_menu_open(menu: u32) -> u32 {
        tolib::bridge::save_menu_open(&mut mem(), menu, true, &mut GameMenu)
    }
    /// `TOSaveMenuOpen(menu)`.
    TOSaveMenuOpen_008b3580 @ 0x008b_3580 => extern "C" fn to_save_menu_open(menu: u32) -> u32 {
        tolib::bridge::save_menu_open(&mut mem(), menu, false, &mut GameMenu)
    }
    /// `TOMenuFieldMusic()`.
    TOMenuFieldMusic_008bdf60 @ 0x008b_df60 => extern "C" fn to_menu_field_music() -> u32 {
        tolib::bridge::menu_music(&mut mem(), &mut GameMenu)
    }
    /// `TOBtlCtrlLibraWindow()`.
    TOBtlCtrlLibraWindow @ 0x0089_1a30 => extern "C" fn to_btl_ctrl_libra_window() -> u32 {
        tolib::bridge::btl_ctrl_libra_window(&mut mem(), |m, id| call_sym(m, 0x0088_6ba0, &[id]))
    }
    /// `TOBtlInit()`.
    TOBtlInit @ 0x0089_77f0 => extern "C" fn to_btl_init() -> u32 {
        tolib::bridge::btl_init(&mut mem())
    }
    /// `TOMenuCloseMainWindows(owner)`.
    TOMenuCloseMainWindows_008a9230 @ 0x008a_9230 => extern "C" fn to_menu_close_main_windows(owner: u32) -> u32 {
        tolib::bridge::close_main_windows(&mut mem(), owner, call_ptr, |m, w| call_sym(m, 0x008a_a3f0, &[w]))
    }
    /// `TOSetWindowRect(i, x, y, w, h)`.
    TOSetWindowRect_00906dd0 @ 0x0090_6dd0 => extern "C" fn to_set_window_rect(i: i32, x: i32, y: i32, w: i32, h: i32) -> u32 {
        tolib::bridge::set_window_rect(&mut mem(), i, [x, y, w, h])
    }
    /// `TOBtlSetDamageValueShort(a, b, d, e, f, g, h)`.
    TOBtlSetDamageValue_0089baa0 @ 0x0089_baa0 => extern "C" fn to_btl_set_damage_value_short(a: u32, b: u32, d: u32, e: u32, f: u32, g: u32, h: u32) -> i32 {
        btl::bridge::set_damage_value_short(&mut mem(), [a, b, d, e, f, g, h])
    }
    /// `TOMoveWindowEntry(i, x, y, n)`.
    TOMoveWindowEntry_00901090 @ 0x0090_1090 => extern "C" fn to_move_window_entry(i: i32, x: i32, y: i32, n: u32) {
        tolib::bridge::move_window_entry(&mut mem(), i, x, y, n);
    }
    /// `TOGetButtonIconWidth(code)`.
    TOGetButtonIconWidth @ 0x008b_65e0 => extern "C" fn to_get_button_icon_width(code: u32) -> u32 {
        tolib::bridge::button_icon_width(&mem(), code)
    }
    /// `TOSeqClearScene(i)`.
    TOSeqClearScene_008adf80 @ 0x008a_df80 => extern "C" fn to_seq_clear_scene(i: u32) -> u32 {
        tolib::bridge::clear_scene_slots(&mut mem(), i)
    }
    /// `TOMessageSlotMode1(i, a, b, c)`.
    TOMessageSlotMode1_008b0020 @ 0x008b_0020 => extern "C" fn to_message_slot_mode1(i: u32, a: u32, b: u32, c: u32) -> u32 {
        tolib::bridge::message_slot_mode1(&mut mem(), i, a, b, c)
    }
    /// `TOMessageSlotCursor(i, x, y, v)`.
    TOMessageSlotCursor_008af730 @ 0x008a_f730 => extern "C" fn to_message_slot_cursor(i: u32, x: i32, y: i32, v: u32) -> u32 {
        tolib::bridge::message_slot_cursor(&mut mem(), i, x, y, v)
    }
    /// `TOGsFRAMEPacket(fbp, fbw, psm, fbmsk)`.
    TOGsFramePacket_008fd1f0 @ 0x008f_d1f0 => extern "C" fn to_gs_frame_packet(a: i32, b: i32, c: i32, d: i32) -> u32 {
        tolib::bridge::frame_packet(&mut mem(), [a, b, c, d])
    }
    /// `TOMenuMainWindowOpen()`.
    TOMenuMainWindowOpen_008a8e70 @ 0x008a_8e70 => extern "C" fn to_menu_main_window_open() -> u32 {
        tolib::bridge::menu_main_window_open(&mut mem(), &mut GameMenu)
    }
    /// `TOStartWindowEntry(i, x, y, n)`.
    TOStartWindowEntry_00901000 @ 0x0090_1000 => extern "C" fn to_start_window_entry(i: i32, x: i32, y: i32, n: i32) {
        tolib::bridge::start_window_entry(&mut mem(), i, x, y, n);
    }
    /// `TOSetExeWindowArea(bank)`.
    TOSetExeWindowArea @ 0x0090_6ce0 => extern "C" fn to_set_exe_window_area(bank: u32) {
        tolib::bridge::set_exe_window_area(&mut mem(), bank);
    }
    /// `TOBtlMakeTargetCommandBit()`.
    TOBtlMakeTargetCommandBit @ 0x0089_83f0 => extern "C" fn to_btl_make_target_command_bit() -> u32 {
        let mut tmp = 0u32;
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = std::ptr::addr_of_mut!(tmp) as u32;
        let r = tolib::bridge::btl_make_target_command_bit(&mut mem(), at, ffx_battle::stat::bridge::ms_get_com_data);
        std::hint::black_box(&mut tmp);
        r
    }
    /// `TOBtlMakeTargetChrStatusBit()`.
    TOBtlMakeTargetChrStatusBit @ 0x0089_8330 => extern "C" fn to_btl_make_target_chr_status_bit() {
        tolib::bridge::btl_make_target_chr_status_bit(&mut mem(), &mut GameMenu);
    }
    /// `TOGetFontWidthInPiexl(a, b)`.
    TOGetFontWidthInPiexl @ 0x008f_a6b0 => extern "C" fn to_get_font_width_in_piexl(a: f32, b: f32) -> f32 {
        tolib::bridge::font_width_in_pixel(&mut mem(), a, b, font_screen_wh)
    }
    /// `TOWideWindowX(i, x)`.
    TOWideWindowX_008b8430 @ 0x008b_8430 => extern "C" fn to_wide_window_x(i: i32, x: f32) -> f32 {
        tolib::bridge::wide_window_x(&mut mem(), i, x, font_screen_wh)
    }
    /// `TOGetFontSpaceInPiexl(kind)`.
    TOGetFontSpaceInPiexl @ 0x008f_a570 => extern "C" fn to_get_font_space_in_piexl(kind: u32) -> f32 {
        tolib::bridge::font_space_in_pixel(
            &mut mem(),
            kind,
            |m| instance::bridge::ffx_lang(m, &mut phyre_printf),
            font_screen_wh,
        )
    }
    /// `TOInitMessageSlot(i)`.
    TOInitMessageSlot_008ada30 @ 0x008a_da30 => extern "C" fn to_init_message_slot(i: i32) -> u32 {
        tolib::bridge::init_message_slot(&mut mem(), i)
    }
    /// `TOSeqInitScene(i)`.
    TOSeqInitScene @ 0x008a_eec0 => extern "C" fn to_seq_init_scene(i: i32) -> u32 {
        scene::bridge::init_scene(&mut mem(), i)
    }
    /// `TOGetImageWH(id, &w, &h)`.
    TOGetImageWH @ 0x008a_c400 => extern "C" fn to_get_image_wh(id: u32, w: u32, h: u32) -> u32 {
        tolib::bridge::get_image_wh(
            &mut mem(),
            id,
            [w, h],
            |m| ffx_menu::tk::bridge::tk_get_lang(m).cast_unsigned(),
            |m| call_sym(m, 0x0078_16f0, &[]),
        )
    }
    /// `TOBtlLimitTimerStop()` (two copies).
    TOBtlLimitTimerStop_0089ab20 @ 0x0089_ab20 => extern "C" fn to_btl_limit_timer_stop_0089ab20() -> u32 {
        tolib::bridge::limit_timer_stop(&mut mem(), limit_sound)
    }
    /// `TOBtlLimitTimerStop()`, second copy.
    TOBtlLimitTimerStop_0089aba0 @ 0x0089_aba0 => extern "C" fn to_btl_limit_timer_stop_0089aba0() -> u32 {
        tolib::bridge::limit_timer_stop(&mut mem(), limit_sound)
    }
    /// `TOBtlLimitTimerStart()`.
    TOBtlLimitTimerStart_0089ab50 @ 0x0089_ab50 => extern "C" fn to_btl_limit_timer_start() {
        tolib::bridge::limit_timer_start(&mut mem(), |m, play, id| call_sym(m, if play { 0x0088_6e80 } else { 0x0088_6680 }, &[id]));
    }
    /// `TOCopyWindowLayout(table, i)`.
    TOCopyWindowLayout_00905e90 @ 0x0090_5e90 => extern "C" fn to_copy_window_layout(table: u32, i: u32) -> u32 {
        tolib::bridge::copy_window_layout(&mut mem(), table, i)
    }
    /// `TOMenuInit()`.
    TOMenuInit @ 0x008b_e800 => extern "C" fn to_menu_init() -> u32 {
        tolib::bridge::menu_init(&mut mem())
    }
    /// `TOMenuInitBrush()`.
    TOMenuInitBrush @ 0x0090_0f40 => extern "C" fn to_menu_init_brush() -> u32 {
        tolib::bridge::menu_init_brush(&mut mem())
    }
    /// `TOAmapMakeItemList(k)`.
    TOAmapMakeItemList @ 0x008c_6730 => extern "C" fn to_amap_make_item_list(k: u32) -> u32 {
        tolib::bridge::amap_make_item_list(&mut mem(), k, |m, mode| call_sym(m, 0x008c_8e60, &[mode]))
    }
    /// `TOGetShapTextureName(id)`.
    TOGetShapTextureName @ 0x008a_c8c0 => extern "C" fn to_get_shap_texture_name(id: u32) -> u32 {
        tolib::bridge::shap_texture_name(&mut mem(), id, |m| instance::bridge::ffx_lang(m, &mut phyre_printf))
    }
    /// `TOSeqInitParam()`.
    TOSeqInitParam @ 0x008a_e6f0 => extern "C" fn to_seq_init_param() {
        let mut local = 0u32;
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = std::ptr::addr_of_mut!(local) as u32;
        scene::bridge::init_param(&mut mem(), at, menu_scene::tk_menu_get_param_handle, menu_scene::tk_menu_get_param_offset);
        std::hint::black_box(&mut local);
    }
    /// `TOCopyWindowLayout` from the table at `0x00c5b460`.
    TOCopyWindowLayout_008fa7d0 @ 0x008f_a7d0 => extern "C" fn to_copy_window_layout_008fa7d0(i: u32) -> u32 {
        tolib::bridge::copy_layout_from(&mut mem(), 0x00c5_b460, i)
    }
    /// `TOCopyWindowLayout` from the table at `0x00c5c2c8`.
    TOCopyWindowLayout_008fa450 @ 0x008f_a450 => extern "C" fn to_copy_window_layout_008fa450(i: u32) -> u32 {
        tolib::bridge::copy_layout_from(&mut mem(), 0x00c5_c2c8, i)
    }
    /// `TOCopyWindowLayout` from the table at `0x00c5d578`.
    TOCopyWindowLayout_008fa790 @ 0x008f_a790 => extern "C" fn to_copy_window_layout_008fa790(i: u32) -> u32 {
        tolib::bridge::copy_layout_from(&mut mem(), 0x00c5_d578, i)
    }
    /// `TOGetFontAdvance(kind, font, size)`.
    TOGetFontAdvance_008fa740 @ 0x008f_a740 => extern "C" fn to_get_font_advance(kind: u32, font: u32, size: f32) -> f32 {
        tolib::bridge::font_advance(&mut mem(), kind, font, size, font_width, font_space)
    }
    /// `TOAddFontWidthSpace(glyph, font, size)`.
    TOAddFontWidthSpace @ 0x008e_8ed0 => extern "C" fn to_add_font_width_space(glyph: u32, font: u32, size: f32) {
        tolib::bridge::add_font_width_space(&mut mem(), glyph, font, size, font_screen_wh, font_width, font_space);
    }
    /// `TOWideWindowRect(i, out, x, y, w, h)`.
    TOWideWindowRect_008b8d10 @ 0x008b_8d10 => extern "C" fn to_wide_window_rect(i: i32, out: u32, x: i32, y: i32, w: i32, h: i32) -> i32 {
        tolib::bridge::wide_window_rect(&mut mem(), i, out, [x, y, w, h], |m, i, x| {
            tolib::bridge::wide_window_x(m, i, x, font_screen_wh)
        })
    }
    /// `TOInternName(name)`.
    TOInternName_008ab440 @ 0x008a_b440 => extern "C" fn to_intern_name(name: u32) -> u32 {
        tolib::bridge::intern_name(&mut mem(), name)
    }
    /// `TOSummonListInit()`.
    TOSummonListInit_008a9090 @ 0x008a_9090 => extern "C" fn to_summon_list_init() {
        tolib::bridge::summon_list_init(&mut mem(), &mut GameMenu);
    }
    /// `TOFTCTransfer(file)`.
    TOFTCTransfer @ 0x008a_c130 => extern "C" fn to_ftc_transfer(file: u32) -> u32 {
        tolib::bridge::ftc_transfer(&mut mem(), file)
    }
    /// `TOPlayerListInit(flags)`.
    TOPlayerListInit_008a8f40 @ 0x008a_8f40 => extern "C" fn to_player_list_init(flags: u32) {
        let mut count = 0i32;
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = std::ptr::addr_of_mut!(count) as u32;
        tolib::bridge::player_list_init(&mut mem(), flags, at, &mut GameMenu);
        std::hint::black_box(&mut count);
    }
    /// `TOBtlInitInfoWinMes()`.
    TOBtlInitInfoWinMes @ 0x0089_7bd0 => extern "C" fn to_btl_init_info_win_mes() -> u32 {
        tolib::bridge::btl_init_info_win_mes(&mut mem(), |_, y, v| {
            if y { ffx_graphic::ui::remap_y2(v) } else { ffx_graphic::ui::remap_x2(v) }
        })
    }
    /// `TOSortMessageWindowPriority(out)`.
    TOSortMessageWindowPriority @ 0x008b_00a0 => extern "C" fn to_sort_message_window_priority(out: u32) -> u32 {
        tolib::bridge::sort_message_window_priority(&mut mem(), out)
    }
    /// `TOGetFontWidth(text, kind, &width, table)`.
    TOGetFontWidth @ 0x008b_9330 => extern "C" fn to_get_font_width(text: u32, kind: u32, out: u32, table: u32) -> u32 {
        tolib::bridge::font_width(&mut mem(), text, kind, out, table, &mut GameMenu)
    }
    /// `TOGetWindowLayout(id)`.
    TOGetWindowLayout @ 0x008f_a4f0 => extern "C" fn to_get_window_layout(id: u32) -> u32 {
        tolib::bridge::window_layout_of(&mut mem(), id)
    }
    /// `IsLoadTexture()`.
    IsLoadTexture @ 0x009d_a280 => extern "C" fn is_load_texture() -> u32 {
        menu::bridge::is_load_texture(&mut mem(), |m, id| {
            call_sym(m, 0x0064_20b0, &[id]);
        })
    }
    /// `getSa_MenuTextureName(out, n)`.
    getSa_MenuTextureName @ 0x0077_4ae0 => extern "C" fn get_sa_menu_texture_name(out: u32, n: i32) {
        let mut m = mem();
        let lang = instance::bridge::ffx_lang(&m, &mut phyre_printf);
        files::bridge::help_texture_name(&mut m, out, n, lang);
    }
    /// `TOGetSaveGilAddress()`.
    TOGetSaveGilAddress @ 0x008a_8e20 => extern "C" fn to_get_save_gil_address() -> u32 {
        tolib::bridge::save_gil_address(&mem())
    }
    /// `TOGetTextureName(id, nibbles, index, high, &size)`.
    TOGetTextureName @ 0x008a_cf50 => extern "C" fn to_get_texture_name(id: u32, nibbles: u32, index: u32, high: u32, size: u32) -> u32 {
        tolib::bridge::texture_name(&mut mem(), id, nibbles, index, high, size, &mut GameMenu)
    }
    /// `MagicFile_SetMagicId(id)`.
    MagicFile_SetMagicId @ 0x009d_a7a0 => extern "C" fn magic_file_set_magic_id(id: u32) {
        menu::bridge::magic_file_set_magic_id(&mut mem(), id, &mut GameMenu);
    }
    /// `MagicFile_Start()`.
    MagicFile_Start @ 0x009d_a7f0 => extern "C" fn magic_file_start() -> u32 {
        menu::bridge::magic_file_start(&mut mem(), &mut GameMenu)
    }
    /// `TOPlaceMessageWindow(i, x, y, w, h)`.
    TOPlaceMessageWindow @ 0x008a_fbb0 => extern "C" fn to_place_message_window(i: i32, x: i32, y: i32, w: i32, h: i32) {
        tolib::bridge::place_message_window(&mut mem(), i, [x, y, w, h], font_screen_wh);
    }
    /// `TOSetMessageWindowPosition(i, anchor, x, y)`.
    TOSetMessageWindowPosition @ 0x008a_f850 => extern "C" fn to_set_message_window_position(i: i32, anchor: u32, x: i32, y: i32) {
        tolib::bridge::set_message_window_position(&mut mem(), i, anchor, x, y, font_screen_wh);
    }
    /// `TOSetMessageWindowRect(i, anchor, x, y, w, h)`.
    TOSetMessageWindowRect @ 0x008a_fa90 => extern "C" fn to_set_message_window_rect(i: i32, anchor: u32, x: i32, y: i32, w: i32, h: i32) {
        tolib::bridge::set_message_window_rect(&mut mem(), i, anchor, [x, y, w, h], font_screen_wh);
    }
    /// `TOBtlInitAuronLimitWindow()`.
    TOBtlInitAuronLimitWindow @ 0x0089_7970 => extern "C" fn to_btl_init_auron_limit_window() {
        tolib::bridge::btl_init_auron_limit_window(&mut mem(), &mut GameMenu);
    }
    /// `TOBtlInitLuluLimitWindow()`.
    TOBtlInitLuluLimitWindow @ 0x0089_7f70 => extern "C" fn to_btl_init_lulu_limit_window() {
        tolib::bridge::btl_init_lulu_limit_window(&mut mem(), &mut GameMenu);
    }
    /// `TOBtlInitTidusLimitWindow()`.
    TOBtlInitTidusLimitWindow @ 0x0089_8180 => extern "C" fn to_btl_init_tidus_limit_window() {
        tolib::bridge::btl_init_tidus_limit_window(&mut mem(), &mut GameMenu);
    }
    /// `TOBtlInitActiveChrCursor()`.
    TOBtlInitActiveChrCursor @ 0x0089_7860 => extern "C" fn to_btl_init_active_chr_cursor() {
        tolib::bridge::btl_init_active_chr_cursor(&mut mem(), &mut GameMenu);
    }
    /// `TOBtlOpenAuronLimitWindow(cmd)`.
    TOBtlOpenAuronLimitWindow @ 0x0089_8ad0 => extern "C" fn to_btl_open_auron_limit_window(cmd: u32) -> u32 {
        tolib::bridge::btl_open_auron_limit_window(&mut mem(), cmd, &mut GameMenu)
    }
    /// `TOBtlOpenLuluLimitWindow(cmd)`.
    TOBtlOpenLuluLimitWindow @ 0x0089_8bf0 => extern "C" fn to_btl_open_lulu_limit_window(cmd: u32) -> u32 {
        tolib::bridge::btl_open_lulu_limit_window(&mut mem(), cmd, &mut GameMenu)
    }
    /// `TOBtlOpenTidusLimitWindow(cmd)`.
    TOBtlOpenTidusLimitWindow @ 0x0089_8ca0 => extern "C" fn to_btl_open_tidus_limit_window(cmd: u32) -> u32 {
        tolib::bridge::btl_open_tidus_limit_window(&mut mem(), cmd, &mut GameMenu)
    }
    /// `TOBtlCtrlLimitTimer()`.
    TOBtlCtrlLimitTimer @ 0x0089_1ac0 => extern "C" fn to_btl_ctrl_limit_timer() {
        tolib::bridge::btl_ctrl_limit_timer(&mut mem(), |m| call_sym(m, 0x0079_89a0, &[]));
    }
    /// `TOBtlMemoryCursorPos()`.
    TOBtlMemoryCursorPos @ 0x0089_89d0 => extern "C" fn to_btl_memory_cursor_pos() {
        tolib::bridge::btl_memory_cursor_pos(&mut mem());
    }
    /// `TOBtlMakeTargetGroupBit()`.
    TOBtlMakeTargetGroupBit @ 0x0089_8490 => extern "C" fn to_btl_make_target_group_bit() {
        let mut tmp = 0u32;
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = std::ptr::addr_of_mut!(tmp) as u32;
        tolib::bridge::btl_make_target_group_bit(&mut mem(), at, ffx_battle::stat::bridge::ms_get_com_data, |m, chr| {
            call_sym(m, 0x0079_5040, &[chr])
        });
        std::hint::black_box(&mut tmp);
    }
    /// `TOBtlMakeTargetTargetBit()`.
    TOBtlMakeTargetTargetBit @ 0x0089_8640 => extern "C" fn to_btl_make_target_target_bit() {
        let mut tmp = [0u32; 3];
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = tmp.as_mut_ptr() as u32;
        tolib::bridge::btl_make_target_target_bit(&mut mem(), at, ffx_battle::stat::bridge::ms_get_com_data);
        std::hint::black_box(&mut tmp);
    }
    /// `TOBtlStartLimitTimer(t)`.
    TOBtlStartLimitTimer @ 0x0089_a2a0 => extern "C" fn to_btl_start_limit_timer(t: f32) {
        tolib::bridge::btl_start_limit_timer(&mut mem(), t);
    }
    /// `TOBtlSetLimitTimerPos(x, y)`.
    TOBtlSetLimitTimerPos @ 0x0089_a2d0 => extern "C" fn to_btl_set_limit_timer_pos(x: u32, y: u32) {
        tolib::bridge::btl_set_limit_timer_pos(&mut mem(), x, y);
    }
    /// `TOBtlSetDefaultCursorPos()`.
    TOBtlSetDefaultCursorPos @ 0x0089_9ac0 => extern "C" fn to_btl_set_default_cursor_pos() {
        tolib::bridge::btl_set_default_cursor_pos(&mut mem());
    }
    /// `TOReset2()`.
    TOReset2 @ 0x008a_dee0 => extern "C" fn to_reset2() -> u32 {
        tolib::bridge::reset2(&mut mem(), call_sym)
    }
    /// `TOStdGetItemName(item)`.
    TOStdGetItemName @ 0x008b_8db0 => extern "C" fn to_std_get_item_name(item: u32) -> u32 {
        let mut tmp = 0u32;
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = std::ptr::addr_of_mut!(tmp) as u32;
        let r = tolib::bridge::std_get_item_name(&mut mem(), item, at, call_sym);
        std::hint::black_box(&mut tmp);
        r
    }
    /// `TOStdGetWeaponName(id)`.
    TOStdGetWeaponName @ 0x008b_8dd0 => extern "C" fn to_std_get_weapon_name(id: u32) -> u32 {
        tolib::bridge::std_get_weapon_name(&mut mem(), id, call_sym)
    }
    /// `TOBtlEndSetFlagMark(i, k)`.
    TOBtlEndSetFlagMark @ 0x008b_e020 => extern "C" fn to_btl_end_set_flag_mark(i: u32, k: u32) {
        tolib::bridge::btl_end_set_flag_f(&mut mem(), i, k, call_sym);
    }
    /// `TOMenuTermHelp()`.
    TOMenuTermHelp @ 0x008b_f0b0 => extern "C" fn to_menu_term_help() {
        tolib::bridge::menu_term_help(&mut mem(), call_sym);
    }
    /// `TOMenuTermAbilityMap2()`.
    TOMenuTermAbilityMap2 @ 0x008e_27e0 => extern "C" fn to_menu_term_ability_map2() {
        tolib::bridge::menu_term_ability_map2(&mut mem(), call_sym);
    }
    /// `TOSetWindowKind(i, kind)`.
    TOSetWindowKind @ 0x0090_6e60 => extern "C" fn to_set_window_kind(i: u32, kind: u32) {
        tolib::bridge::set_window_kind(&mut mem(), i, kind, call_sym);
    }
    /// `MagicFile_Cleanup()`.
    MagicFile_Cleanup @ 0x009d_a2d0 => extern "C" fn magic_file_cleanup() {
        menu::bridge::magic_file_cleanup(&mut mem(), call_sym);
    }
    /// `MagicFile_NeedRestoreCamera()`.
    MagicFile_NeedRestoreCamera @ 0x009d_a700 => extern "C" fn magic_file_need_restore_camera() -> u32 {
        menu::bridge::magic_file_need_restore_camera(&mut mem(), call_sym)
    }
    /// `MagicFile_RestoreScreenSetup()`.
    MagicFile_RestoreScreenSetup @ 0x009d_a780 => extern "C" fn magic_file_restore_screen_setup() -> u32 {
        menu::bridge::magic_file_restore_screen_setup(&mut mem(), call_sym)
    }
    /// `MagicFile_isPrxPreloaded(id)`.
    MagicFile_isPrxPreloaded @ 0x009d_ae60 => extern "C" fn magic_file_is_prx_preloaded(id: u32) -> u32 {
        let mut tmp = 0u32;
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = std::ptr::addr_of_mut!(tmp) as u32;
        let r = menu::bridge::magic_file_is_prx_preloaded(&mut mem(), id, at, call_sym);
        std::hint::black_box(&mut tmp);
        r
    }
    /// `TOCheckMapMessage(id)`.
    TOCheckMapMessage @ 0x008a_f6a0 => extern "C" fn to_check_map_message(id: u32) -> u32 {
        tolib::bridge::check_map_message(&mut mem(), id, call_sym)
    }
    /// `TOCountMapVisits()`.
    TOCountMapVisits @ 0x008b_5680 => extern "C" fn to_count_map_visits() {
        tolib::bridge::count_map_visits(&mut mem(), call_sym);
    }
    /// `TOSetCollectBit(n)`.
    TOSetCollectBit @ 0x0090_6b30 => extern "C" fn to_set_collect_bit(n: u32) {
        tolib::bridge::collect_bit(&mut mem(), n, call_sym);
    }
    /// `TOClearBlock_008b3610()`.
    TOClearBlock_008b3610 @ 0x008b_3610 => extern "C" fn to_clear_block_008b3610() {
        tolib::bridge::clear_block_159e468(&mut mem(), call_sym);
    }
    /// `TOGetFontWidthForLang(text, kind, out, _)`.
    TOGetFontWidthForLang @ 0x008b_7c00 => extern "C" fn to_get_font_width_for_lang(text: u32, kind: u32, out: u32) -> u32 {
        tolib::bridge::font_width_for_lang(
            &mut mem(),
            |m| instance::bridge::ffx_lang(m, &mut phyre_printf),
            |m, table| tolib::bridge::font_width(m, text, kind, out, table, &mut GameMenu),
        )
    }
    /// `TOBtlSearchRevNearTarget()`.
    TOBtlSearchRevNearTarget @ 0x0089_90f0 => extern "C" fn to_btl_search_rev_near_target() -> u32 {
        let mut v = [0f32; 4];
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = v.as_mut_ptr() as u32;
        let r = tolib::bridge::btl_search_rev_near_target(&mut mem(), at, &mut GameMenu);
        std::hint::black_box(&mut v);
        r
    }
    /// `TOInitSceneMessageWindow()`.
    TOInitSceneMessageWindow @ 0x008a_dbf0 => extern "C" fn to_init_scene_message_window() {
        tolib::bridge::init_scene_message_window(&mut mem());
    }
    /// `TOSetExeMessageArea(bank)`.
    TOSetExeMessageArea @ 0x008b_86e0 => extern "C" fn to_set_exe_message_area(bank: u32) {
        tolib::bridge::set_exe_message_area(&mut mem(), bank);
    }
    /// `TOInitMessageCallBack(msg)`.
    TOInitMessageCallBack @ 0x008d_f6d0 => extern "C" fn to_init_message_call_back(msg: u32) {
        tolib::bridge::init_message_call_back(&mut mem(), msg);
    }
    /// `TOMenuMakeSaveFileName()`.
    TOMenuMakeSaveFileName @ 0x008b_3a40 => extern "C" fn to_menu_make_save_file_name() {
        let mut m = mem();
        let lang = instance::bridge::ffx_lang(&m, &mut phyre_printf);
        tolib::bridge::menu_make_save_file_name(&mut m, lang);
    }
    /// `TOFuncMacroCmdValue(cmd)`.
    TOFuncMacroCmdValue @ 0x008b_6210 => extern "C" fn to_func_macro_cmd_value(cmd: u32) -> u32 {
        tolib::bridge::func_macro_cmd_value(&mut mem(), cmd)
    }
    /// `TOGetPadStatus()`.
    TOGetPadStatus @ 0x008b_7cd0 => extern "C" fn to_get_pad_status() {
        tolib::bridge::get_pad_status(&mut mem(), &mut GameMenu);
    }
    /// `TOMenuGetPadStatus()`.
    TOMenuGetPadStatus @ 0x008b_e500 => extern "C" fn to_menu_get_pad_status() {
        tolib::bridge::menu_get_pad_status(&mut mem(), |_| st0_result(0x0063_0c60), call_sym);
    }
    /// `KernelGetSaveItemNum_007724c0(id)`.
    KernelGetSaveItemNum_007724c0 @ 0x0077_24c0 => extern "C" fn kernel_forward_007724c0(id: u32) -> u32 {
        tolib::bridge::kernel_forward(&mut mem(), 0x0079_04b0, id, call_sym)
    }
    /// `KernelImportantGet_007724f0(id)`.
    KernelImportantGet_007724f0 @ 0x0077_24f0 => extern "C" fn kernel_forward_007724f0(id: u32) -> u32 {
        tolib::bridge::kernel_forward(&mut mem(), 0x0079_0800, id, call_sym)
    }
    /// `KernelPlayerWord_50_00772370(chr)`.
    KernelPlayerWord_50_00772370 @ 0x0077_2370 => extern "C" fn kernel_player_word_00772370(chr: u32) -> u32 {
        tolib::bridge::kernel_player_word(&mut mem(), chr, 0x50, call_sym)
    }
    /// `KernelPlayerWord_58_007723e0(chr)`.
    KernelPlayerWord_58_007723e0 @ 0x0077_23e0 => extern "C" fn kernel_player_word_007723e0(chr: u32) -> u32 {
        tolib::bridge::kernel_player_word(&mut mem(), chr, 0x58, call_sym)
    }
    /// `KernelPlayerWord_54_00772450(chr)`.
    KernelPlayerWord_54_00772450 @ 0x0077_2450 => extern "C" fn kernel_player_word_00772450(chr: u32) -> u32 {
        tolib::bridge::kernel_player_word(&mut mem(), chr, 0x54, call_sym)
    }
    /// `KernelSaveCommand_30b8_00771c60(chr)`.
    KernelSaveCommand_30b8_00771c60 @ 0x0077_1c60 => extern "C" fn kernel_save_command_00771c60(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30b8, call_sym)
    }
    /// `KernelSaveCommand_30b9_00771c80(chr)`.
    KernelSaveCommand_30b9_00771c80 @ 0x0077_1c80 => extern "C" fn kernel_save_command_00771c80(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30b9, call_sym)
    }
    /// `KernelSaveCommand_30ba_00771cd0(chr)`.
    KernelSaveCommand_30ba_00771cd0 @ 0x0077_1cd0 => extern "C" fn kernel_save_command_00771cd0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30ba, call_sym)
    }
    /// `KernelSaveCommand_30bb_00771d20(chr)`.
    KernelSaveCommand_30bb_00771d20 @ 0x0077_1d20 => extern "C" fn kernel_save_command_00771d20(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30bb, call_sym)
    }
    /// `KernelSaveCommand_30bc_00771d70(chr)`.
    KernelSaveCommand_30bc_00771d70 @ 0x0077_1d70 => extern "C" fn kernel_save_command_00771d70(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30bc, call_sym)
    }
    /// `KernelSaveCommand_30bd_00771dc0(chr)`.
    KernelSaveCommand_30bd_00771dc0 @ 0x0077_1dc0 => extern "C" fn kernel_save_command_00771dc0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30bd, call_sym)
    }
    /// `KernelSaveCommand_30be_00771e10(chr)`.
    KernelSaveCommand_30be_00771e10 @ 0x0077_1e10 => extern "C" fn kernel_save_command_00771e10(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30be, call_sym)
    }
    /// `KernelSaveCommand_30bf_00771e30(chr)`.
    KernelSaveCommand_30bf_00771e30 @ 0x0077_1e30 => extern "C" fn kernel_save_command_00771e30(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30bf, call_sym)
    }
    /// `KernelSaveCommand_30c0_00771e80(chr)`.
    KernelSaveCommand_30c0_00771e80 @ 0x0077_1e80 => extern "C" fn kernel_save_command_00771e80(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30c0, call_sym)
    }
    /// `KernelSaveCommand_30c1_00771ed0(chr)`.
    KernelSaveCommand_30c1_00771ed0 @ 0x0077_1ed0 => extern "C" fn kernel_save_command_00771ed0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30c1, call_sym)
    }
    /// `KernelSaveCommand_30c2_00771f20(chr)`.
    KernelSaveCommand_30c2_00771f20 @ 0x0077_1f20 => extern "C" fn kernel_save_command_00771f20(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30c2, call_sym)
    }
    /// `KernelSaveCommand_30c3_00771f70(chr)`.
    KernelSaveCommand_30c3_00771f70 @ 0x0077_1f70 => extern "C" fn kernel_save_command_00771f70(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30c3, call_sym)
    }
    /// `KernelSaveCommand_30c4_00771fc0(chr)`.
    KernelSaveCommand_30c4_00771fc0 @ 0x0077_1fc0 => extern "C" fn kernel_save_command_00771fc0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30c4, call_sym)
    }
    /// `KernelSaveCommand_30c5_00771fe0(chr)`.
    KernelSaveCommand_30c5_00771fe0 @ 0x0077_1fe0 => extern "C" fn kernel_save_command_00771fe0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30c5, call_sym)
    }
    /// `KernelSaveCommand_30c6_00772030(chr)`.
    KernelSaveCommand_30c6_00772030 @ 0x0077_2030 => extern "C" fn kernel_save_command_00772030(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30c6, call_sym)
    }
    /// `KernelSaveCommand_30c7_00772080(chr)`.
    KernelSaveCommand_30c7_00772080 @ 0x0077_2080 => extern "C" fn kernel_save_command_00772080(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30c7, call_sym)
    }
    /// `KernelSaveCommand_30c8_007720d0(chr)`.
    KernelSaveCommand_30c8_007720d0 @ 0x0077_20d0 => extern "C" fn kernel_save_command_007720d0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30c8, call_sym)
    }
    /// `KernelSaveCommand_30c9_00772120(chr)`.
    KernelSaveCommand_30c9_00772120 @ 0x0077_2120 => extern "C" fn kernel_save_command_00772120(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30c9, call_sym)
    }
    /// `KernelSaveCommand_30ca_00772170(chr)`.
    KernelSaveCommand_30ca_00772170 @ 0x0077_2170 => extern "C" fn kernel_save_command_00772170(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30ca, call_sym)
    }
    /// `KernelSaveCommand_30cb_00772190(chr)`.
    KernelSaveCommand_30cb_00772190 @ 0x0077_2190 => extern "C" fn kernel_save_command_00772190(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30cb, call_sym)
    }
    /// `KernelSaveCommand_30cc_007721e0(chr)`.
    KernelSaveCommand_30cc_007721e0 @ 0x0077_21e0 => extern "C" fn kernel_save_command_007721e0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30cc, call_sym)
    }
    /// `KernelSaveCommand_30cd_00772230(chr)`.
    KernelSaveCommand_30cd_00772230 @ 0x0077_2230 => extern "C" fn kernel_save_command_00772230(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30cd, call_sym)
    }
    /// `KernelSaveCommand_30ce_00772280(chr)`.
    KernelSaveCommand_30ce_00772280 @ 0x0077_2280 => extern "C" fn kernel_save_command_00772280(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30ce, call_sym)
    }
    /// `KernelSaveCommand_30cf_007722f0(chr)`.
    KernelSaveCommand_30cf_007722f0 @ 0x0077_22f0 => extern "C" fn kernel_save_command_007722f0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30cf, call_sym)
    }
    /// `KernelSaveCommand_30d0_00772350(chr)`.
    KernelSaveCommand_30d0_00772350 @ 0x0077_2350 => extern "C" fn kernel_save_command_00772350(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30d0, call_sym)
    }
    /// `KernelSaveCommand_30d1_00772390(chr)`.
    KernelSaveCommand_30d1_00772390 @ 0x0077_2390 => extern "C" fn kernel_save_command_00772390(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30d1, call_sym)
    }
    /// `KernelSaveCommand_30d2_00772400(chr)`.
    KernelSaveCommand_30d2_00772400 @ 0x0077_2400 => extern "C" fn kernel_save_command_00772400(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30d2, call_sym)
    }
    /// `KernelSaveCommand_30d3_00772470(chr)`.
    KernelSaveCommand_30d3_00772470 @ 0x0077_2470 => extern "C" fn kernel_save_command_00772470(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30d3, call_sym)
    }
    /// `KernelSaveCommand_30d4_007724d0(chr)`.
    KernelSaveCommand_30d4_007724d0 @ 0x0077_24d0 => extern "C" fn kernel_save_command_007724d0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30d4, call_sym)
    }
    /// `KernelSlotAbilityName_11e_00771c30(i)`.
    KernelSlotAbilityName_11e_00771c30 @ 0x0077_1c30 => extern "C" fn kernel_slot_ability_name_00771c30(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x11e, call_sym)
    }
    /// `KernelSlotAbilityName_120_00771ca0(i)`.
    KernelSlotAbilityName_120_00771ca0 @ 0x0077_1ca0 => extern "C" fn kernel_slot_ability_name_00771ca0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x120, call_sym)
    }
    /// `KernelSlotAbilityName_122_00771cf0(i)`.
    KernelSlotAbilityName_122_00771cf0 @ 0x0077_1cf0 => extern "C" fn kernel_slot_ability_name_00771cf0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x122, call_sym)
    }
    /// `KernelSlotAbilityName_124_00771d40(i)`.
    KernelSlotAbilityName_124_00771d40 @ 0x0077_1d40 => extern "C" fn kernel_slot_ability_name_00771d40(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x124, call_sym)
    }
    /// `KernelSlotAbilityName_126_00771d90(i)`.
    KernelSlotAbilityName_126_00771d90 @ 0x0077_1d90 => extern "C" fn kernel_slot_ability_name_00771d90(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x126, call_sym)
    }
    /// `KernelSlotAbilityName_128_00771de0(i)`.
    KernelSlotAbilityName_128_00771de0 @ 0x0077_1de0 => extern "C" fn kernel_slot_ability_name_00771de0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x128, call_sym)
    }
    /// `KernelSlotAbilityName_12a_00771e50(i)`.
    KernelSlotAbilityName_12a_00771e50 @ 0x0077_1e50 => extern "C" fn kernel_slot_ability_name_00771e50(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x12a, call_sym)
    }
    /// `KernelSlotAbilityName_12c_00771ea0(i)`.
    KernelSlotAbilityName_12c_00771ea0 @ 0x0077_1ea0 => extern "C" fn kernel_slot_ability_name_00771ea0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x12c, call_sym)
    }
    /// `KernelSlotAbilityName_12e_00771ef0(i)`.
    KernelSlotAbilityName_12e_00771ef0 @ 0x0077_1ef0 => extern "C" fn kernel_slot_ability_name_00771ef0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x12e, call_sym)
    }
    /// `KernelSlotAbilityName_130_00771f40(i)`.
    KernelSlotAbilityName_130_00771f40 @ 0x0077_1f40 => extern "C" fn kernel_slot_ability_name_00771f40(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x130, call_sym)
    }
    /// `KernelSlotAbilityName_132_00771f90(i)`.
    KernelSlotAbilityName_132_00771f90 @ 0x0077_1f90 => extern "C" fn kernel_slot_ability_name_00771f90(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x132, call_sym)
    }
    /// `KernelSlotAbilityName_134_00772000(i)`.
    KernelSlotAbilityName_134_00772000 @ 0x0077_2000 => extern "C" fn kernel_slot_ability_name_00772000(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x134, call_sym)
    }
    /// `KernelSlotAbilityName_136_00772050(i)`.
    KernelSlotAbilityName_136_00772050 @ 0x0077_2050 => extern "C" fn kernel_slot_ability_name_00772050(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x136, call_sym)
    }
    /// `KernelSlotAbilityName_138_007720a0(i)`.
    KernelSlotAbilityName_138_007720a0 @ 0x0077_20a0 => extern "C" fn kernel_slot_ability_name_007720a0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x138, call_sym)
    }
    /// `KernelSlotAbilityName_13a_007720f0(i)`.
    KernelSlotAbilityName_13a_007720f0 @ 0x0077_20f0 => extern "C" fn kernel_slot_ability_name_007720f0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x13a, call_sym)
    }
    /// `KernelSlotAbilityName_13c_00772140(i)`.
    KernelSlotAbilityName_13c_00772140 @ 0x0077_2140 => extern "C" fn kernel_slot_ability_name_00772140(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x13c, call_sym)
    }
    /// `KernelSlotAbilityName_13e_007721b0(i)`.
    KernelSlotAbilityName_13e_007721b0 @ 0x0077_21b0 => extern "C" fn kernel_slot_ability_name_007721b0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x13e, call_sym)
    }
    /// `KernelSlotAbilityName_140_00772200(i)`.
    KernelSlotAbilityName_140_00772200 @ 0x0077_2200 => extern "C" fn kernel_slot_ability_name_00772200(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x140, call_sym)
    }
    /// `KernelSlotAbilityName_142_00772250(i)`.
    KernelSlotAbilityName_142_00772250 @ 0x0077_2250 => extern "C" fn kernel_slot_ability_name_00772250(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x142, call_sym)
    }
    /// `KernelSlotAbilityName_144_007722c0(i)`.
    KernelSlotAbilityName_144_007722c0 @ 0x0077_22c0 => extern "C" fn kernel_slot_ability_name_007722c0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x144, call_sym)
    }
    /// `KernelSlotAbilityName_146_00772320(i)`.
    KernelSlotAbilityName_146_00772320 @ 0x0077_2320 => extern "C" fn kernel_slot_ability_name_00772320(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x146, call_sym)
    }
    /// `KernelSlotAbilityName_148_007723b0(i)`.
    KernelSlotAbilityName_148_007723b0 @ 0x0077_23b0 => extern "C" fn kernel_slot_ability_name_007723b0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x148, call_sym)
    }
    /// `KernelSlotAbilityName_14a_00772420(i)`.
    KernelSlotAbilityName_14a_00772420 @ 0x0077_2420 => extern "C" fn kernel_slot_ability_name_00772420(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x14a, call_sym)
    }
    /// `KernelSlotAbilityName_14c_00772490(i)`.
    KernelSlotAbilityName_14c_00772490 @ 0x0077_2490 => extern "C" fn kernel_slot_ability_name_00772490(i: u32) -> u32 {
        tolib::bridge::kernel_slot_ability_name(&mut mem(), i, 0x14c, call_sym)
    }
    /// `TOSetExeMessageWindowArea(bank)`.
    TOSetExeMessageWindowArea @ 0x008a_f5c0 => extern "C" fn to_set_exe_message_window_area(bank: u32) {
        tolib::bridge::set_exe_message_window_area(&mut mem(), bank);
    }
    /// `TOCheckBtlCommandTarget(chr, cmd)`.
    TOCheckBtlCommandTarget @ 0x0089_abd0 => extern "C" fn to_check_btl_command_target(chr: u32, cmd: u32) -> u32 {
        tolib::bridge::btl_check_command_target(&mut mem(), chr, cmd, call_sym)
    }
    /// `TOControlMessageWindow()`.
    TOControlMessageWindow @ 0x008a_b910 => extern "C" fn to_control_message_window() {
        tolib::bridge::control_message_window(&mut mem(), call_sym);
    }
    /// `TOSetMessageType(i, type)`.
    TOSetMessageType @ 0x008b_8bb0 => extern "C" fn to_set_message_type(i: u32, kind: u32) -> u32 {
        tolib::bridge::set_message_type(&mut mem(), i, kind, call_sym)
    }
    /// `TOGetTextWidth(text, arg)`.
    TOGetTextWidth @ 0x008b_70c0 => extern "C" fn to_get_text_width(text: u32, arg: u32) -> f32 {
        let mut w = 0f32;
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = std::ptr::addr_of_mut!(w) as u32;
        let r = tolib::bridge::text_width(&mut mem(), text, arg, at, |m, p, out, arg| {
            call_sym(m, 0x008b_7c00, &[p, 0, out, arg])
        });
        std::hint::black_box(&mut w);
        r
    }
    /// `KernelMonsterCapture_00772790(id)`.
    KernelMonsterCapture_00772790 @ 0x0077_2790 => extern "C" fn kernel_forward_00772790(id: u32) -> u32 {
        tolib::bridge::kernel_forward(&mut mem(), 0x0079_0af0, id, call_sym)
    }
    /// `KernelSaveCommand_30d5_00772530(chr)`.
    KernelSaveCommand_30d5_00772530 @ 0x0077_2530 => extern "C" fn kernel_save_command_00772530(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30d5, call_sym)
    }
    /// `KernelSaveCommand_30d6_00772590(chr)`.
    KernelSaveCommand_30d6_00772590 @ 0x0077_2590 => extern "C" fn kernel_save_command_00772590(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30d6, call_sym)
    }
    /// `KernelSaveCommand_30d7_007725d0(chr)`.
    KernelSaveCommand_30d7_007725d0 @ 0x0077_25d0 => extern "C" fn kernel_save_command_007725d0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30d7, call_sym)
    }
    /// `KernelSaveCommand_30d8_00772660(chr)`.
    KernelSaveCommand_30d8_00772660 @ 0x0077_2660 => extern "C" fn kernel_save_command_00772660(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30d8, call_sym)
    }
    /// `KernelSaveCommand_30d9_007726b0(chr)`.
    KernelSaveCommand_30d9_007726b0 @ 0x0077_26b0 => extern "C" fn kernel_save_command_007726b0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30d9, call_sym)
    }
    /// `KernelSaveCommand_30da_00772710(chr)`.
    KernelSaveCommand_30da_00772710 @ 0x0077_2710 => extern "C" fn kernel_save_command_00772710(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30da, call_sym)
    }
    /// `KernelSaveCommand_30db_00772770(chr)`.
    KernelSaveCommand_30db_00772770 @ 0x0077_2770 => extern "C" fn kernel_save_command_00772770(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30db, call_sym)
    }
    /// `KernelSaveCommand_30dc_007727d0(chr)`.
    KernelSaveCommand_30dc_007727d0 @ 0x0077_27d0 => extern "C" fn kernel_save_command_007727d0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30dc, call_sym)
    }
    /// `KernelSaveCommand_30dd_00772810(chr)`.
    KernelSaveCommand_30dd_00772810 @ 0x0077_2810 => extern "C" fn kernel_save_command_00772810(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30dd, call_sym)
    }
    /// `KernelSaveCommand_30de_00772890(chr)`.
    KernelSaveCommand_30de_00772890 @ 0x0077_2890 => extern "C" fn kernel_save_command_00772890(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30de, call_sym)
    }
    /// `KernelSaveCommand_30df_007728e0(chr)`.
    KernelSaveCommand_30df_007728e0 @ 0x0077_28e0 => extern "C" fn kernel_save_command_007728e0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30df, call_sym)
    }
    /// `KernelSaveCommand_30e0_00772950(chr)`.
    KernelSaveCommand_30e0_00772950 @ 0x0077_2950 => extern "C" fn kernel_save_command_00772950(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30e0, call_sym)
    }
    /// `KernelSaveCommand_30e1_007729b0(chr)`.
    KernelSaveCommand_30e1_007729b0 @ 0x0077_29b0 => extern "C" fn kernel_save_command_007729b0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30e1, call_sym)
    }
    /// `KernelSaveCommand_3000_007729e0(chr)`.
    KernelSaveCommand_3000_007729e0 @ 0x0077_29e0 => extern "C" fn kernel_save_command_007729e0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3000, call_sym)
    }
    /// `KernelSaveCommand_30e2_00772a30(chr)`.
    KernelSaveCommand_30e2_00772a30 @ 0x0077_2a30 => extern "C" fn kernel_save_command_00772a30(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30e2, call_sym)
    }
    /// `KernelSaveCommand_3001_00772a50(chr)`.
    KernelSaveCommand_3001_00772a50 @ 0x0077_2a50 => extern "C" fn kernel_save_command_00772a50(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3001, call_sym)
    }
    /// `KernelSaveCommand_30e3_00772a70(chr)`.
    KernelSaveCommand_30e3_00772a70 @ 0x0077_2a70 => extern "C" fn kernel_save_command_00772a70(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30e3, call_sym)
    }
    /// `KernelSaveCommand_3002_00772ac0(chr)`.
    KernelSaveCommand_3002_00772ac0 @ 0x0077_2ac0 => extern "C" fn kernel_save_command_00772ac0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3002, call_sym)
    }
    /// `KernelSaveCommand_30e4_00772ae0(chr)`.
    KernelSaveCommand_30e4_00772ae0 @ 0x0077_2ae0 => extern "C" fn kernel_save_command_00772ae0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30e4, call_sym)
    }
    /// `KernelSaveCommand_3003_00772b30(chr)`.
    KernelSaveCommand_3003_00772b30 @ 0x0077_2b30 => extern "C" fn kernel_save_command_00772b30(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3003, call_sym)
    }
    /// `KernelSaveCommand_30e5_00772b50(chr)`.
    KernelSaveCommand_30e5_00772b50 @ 0x0077_2b50 => extern "C" fn kernel_save_command_00772b50(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30e5, call_sym)
    }
    /// `KernelSaveCommand_3004_00772ba0(chr)`.
    KernelSaveCommand_3004_00772ba0 @ 0x0077_2ba0 => extern "C" fn kernel_save_command_00772ba0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3004, call_sym)
    }
    /// `KernelSaveCommand_30e6_00772bc0(chr)`.
    KernelSaveCommand_30e6_00772bc0 @ 0x0077_2bc0 => extern "C" fn kernel_save_command_00772bc0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30e6, call_sym)
    }
    /// `KernelSaveCommand_3005_00772c10(chr)`.
    KernelSaveCommand_3005_00772c10 @ 0x0077_2c10 => extern "C" fn kernel_save_command_00772c10(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3005, call_sym)
    }
    /// `KernelSaveCommand_30e7_00772c30(chr)`.
    KernelSaveCommand_30e7_00772c30 @ 0x0077_2c30 => extern "C" fn kernel_save_command_00772c30(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30e7, call_sym)
    }
    /// `KernelSaveCommand_3006_00772c50(chr)`.
    KernelSaveCommand_3006_00772c50 @ 0x0077_2c50 => extern "C" fn kernel_save_command_00772c50(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3006, call_sym)
    }
    /// `KernelSaveCommand_30e8_00772ca0(chr)`.
    KernelSaveCommand_30e8_00772ca0 @ 0x0077_2ca0 => extern "C" fn kernel_save_command_00772ca0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30e8, call_sym)
    }
    /// `KernelSaveCommand_3007_00772cc0(chr)`.
    KernelSaveCommand_3007_00772cc0 @ 0x0077_2cc0 => extern "C" fn kernel_save_command_00772cc0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3007, call_sym)
    }
    /// `KernelSaveCommand_30e9_00772ce0(chr)`.
    KernelSaveCommand_30e9_00772ce0 @ 0x0077_2ce0 => extern "C" fn kernel_save_command_00772ce0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30e9, call_sym)
    }
    /// `KernelSaveCommand_3008_00772d30(chr)`.
    KernelSaveCommand_3008_00772d30 @ 0x0077_2d30 => extern "C" fn kernel_save_command_00772d30(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3008, call_sym)
    }
    /// `KernelSaveCommand_30ea_00772d50(chr)`.
    KernelSaveCommand_30ea_00772d50 @ 0x0077_2d50 => extern "C" fn kernel_save_command_00772d50(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30ea, call_sym)
    }
    /// `KernelSaveCommand_3009_00772da0(chr)`.
    KernelSaveCommand_3009_00772da0 @ 0x0077_2da0 => extern "C" fn kernel_save_command_00772da0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3009, call_sym)
    }
    /// `KernelSaveCommand_300a_00772e20(chr)`.
    KernelSaveCommand_300a_00772e20 @ 0x0077_2e20 => extern "C" fn kernel_save_command_00772e20(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x300a, call_sym)
    }
    /// `KernelSaveCommand_300b_00772ea0(chr)`.
    KernelSaveCommand_300b_00772ea0 @ 0x0077_2ea0 => extern "C" fn kernel_save_command_00772ea0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x300b, call_sym)
    }
    /// `KernelSaveCommand_30ff_00772ec0(chr)`.
    KernelSaveCommand_30ff_00772ec0 @ 0x0077_2ec0 => extern "C" fn kernel_save_command_00772ec0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x30ff, call_sym)
    }
    /// `KernelSaveCommand_300c_00772ee0(chr)`.
    KernelSaveCommand_300c_00772ee0 @ 0x0077_2ee0 => extern "C" fn kernel_save_command_00772ee0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x300c, call_sym)
    }
    /// `KernelSaveCommand_3100_00772f30(chr)`.
    KernelSaveCommand_3100_00772f30 @ 0x0077_2f30 => extern "C" fn kernel_save_command_00772f30(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3100, call_sym)
    }
    /// `KernelSaveCommand_300d_00772f50(chr)`.
    KernelSaveCommand_300d_00772f50 @ 0x0077_2f50 => extern "C" fn kernel_save_command_00772f50(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x300d, call_sym)
    }
    /// `KernelSaveCommand_3101_00772fa0(chr)`.
    KernelSaveCommand_3101_00772fa0 @ 0x0077_2fa0 => extern "C" fn kernel_save_command_00772fa0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3101, call_sym)
    }
    /// `KernelSaveCommand_300e_00772fc0(chr)`.
    KernelSaveCommand_300e_00772fc0 @ 0x0077_2fc0 => extern "C" fn kernel_save_command_00772fc0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x300e, call_sym)
    }
    /// `KernelSaveCommand_3102_00773010(chr)`.
    KernelSaveCommand_3102_00773010 @ 0x0077_3010 => extern "C" fn kernel_save_command_00773010(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3102, call_sym)
    }
    /// `KernelSaveCommand_300f_00773030(chr)`.
    KernelSaveCommand_300f_00773030 @ 0x0077_3030 => extern "C" fn kernel_save_command_00773030(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x300f, call_sym)
    }
    /// `KernelSaveCommand_3103_00773080(chr)`.
    KernelSaveCommand_3103_00773080 @ 0x0077_3080 => extern "C" fn kernel_save_command_00773080(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3103, call_sym)
    }
    /// `KernelSaveCommand_3010_007730a0(chr)`.
    KernelSaveCommand_3010_007730a0 @ 0x0077_30a0 => extern "C" fn kernel_save_command_007730a0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3010, call_sym)
    }
    /// `KernelSaveCommand_3104_007730c0(chr)`.
    KernelSaveCommand_3104_007730c0 @ 0x0077_30c0 => extern "C" fn kernel_save_command_007730c0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3104, call_sym)
    }
    /// `KernelSaveCommand_3011_00773110(chr)`.
    KernelSaveCommand_3011_00773110 @ 0x0077_3110 => extern "C" fn kernel_save_command_00773110(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3011, call_sym)
    }
    /// `KernelSaveCommand_3105_00773130(chr)`.
    KernelSaveCommand_3105_00773130 @ 0x0077_3130 => extern "C" fn kernel_save_command_00773130(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3105, call_sym)
    }
    /// `KernelSaveCommand_3012_00773150(chr)`.
    KernelSaveCommand_3012_00773150 @ 0x0077_3150 => extern "C" fn kernel_save_command_00773150(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3012, call_sym)
    }
    /// `KernelSaveCommand_3106_007731a0(chr)`.
    KernelSaveCommand_3106_007731a0 @ 0x0077_31a0 => extern "C" fn kernel_save_command_007731a0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3106, call_sym)
    }
    /// `KernelSaveCommand_3013_007731c0(chr)`.
    KernelSaveCommand_3013_007731c0 @ 0x0077_31c0 => extern "C" fn kernel_save_command_007731c0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3013, call_sym)
    }
    /// `KernelSaveCommand_3107_00773210(chr)`.
    KernelSaveCommand_3107_00773210 @ 0x0077_3210 => extern "C" fn kernel_save_command_00773210(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3107, call_sym)
    }
    /// `KernelSaveCommand_3014_00773230(chr)`.
    KernelSaveCommand_3014_00773230 @ 0x0077_3230 => extern "C" fn kernel_save_command_00773230(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3014, call_sym)
    }
    /// `KernelSaveCommand_3108_00773280(chr)`.
    KernelSaveCommand_3108_00773280 @ 0x0077_3280 => extern "C" fn kernel_save_command_00773280(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3108, call_sym)
    }
    /// `KernelSaveCommand_3015_007732a0(chr)`.
    KernelSaveCommand_3015_007732a0 @ 0x0077_32a0 => extern "C" fn kernel_save_command_007732a0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3015, call_sym)
    }
    /// `KernelSaveCommand_3109_007732f0(chr)`.
    KernelSaveCommand_3109_007732f0 @ 0x0077_32f0 => extern "C" fn kernel_save_command_007732f0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3109, call_sym)
    }
    /// `KernelSaveCommand_3016_00773310(chr)`.
    KernelSaveCommand_3016_00773310 @ 0x0077_3310 => extern "C" fn kernel_save_command_00773310(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3016, call_sym)
    }
    /// `KernelSaveCommand_311f_00773330(chr)`.
    KernelSaveCommand_311f_00773330 @ 0x0077_3330 => extern "C" fn kernel_save_command_00773330(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x311f, call_sym)
    }
    /// `KernelSaveCommand_3017_00773380(chr)`.
    KernelSaveCommand_3017_00773380 @ 0x0077_3380 => extern "C" fn kernel_save_command_00773380(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3017, call_sym)
    }
    /// `KernelSaveCommand_3120_007733a0(chr)`.
    KernelSaveCommand_3120_007733a0 @ 0x0077_33a0 => extern "C" fn kernel_save_command_007733a0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3120, call_sym)
    }
    /// `KernelSaveCommand_3018_007733c0(chr)`.
    KernelSaveCommand_3018_007733c0 @ 0x0077_33c0 => extern "C" fn kernel_save_command_007733c0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3018, call_sym)
    }
    /// `KernelSaveCommand_3113_00773410(chr)`.
    KernelSaveCommand_3113_00773410 @ 0x0077_3410 => extern "C" fn kernel_save_command_00773410(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3113, call_sym)
    }
    /// `KernelSaveCommand_3019_00773430(chr)`.
    KernelSaveCommand_3019_00773430 @ 0x0077_3430 => extern "C" fn kernel_save_command_00773430(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3019, call_sym)
    }
    /// `KernelSaveCommand_3114_00773480(chr)`.
    KernelSaveCommand_3114_00773480 @ 0x0077_3480 => extern "C" fn kernel_save_command_00773480(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3114, call_sym)
    }
    /// `KernelSaveCommand_301a_007734a0(chr)`.
    KernelSaveCommand_301a_007734a0 @ 0x0077_34a0 => extern "C" fn kernel_save_command_007734a0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x301a, call_sym)
    }
    /// `KernelSaveCommand_3115_007734f0(chr)`.
    KernelSaveCommand_3115_007734f0 @ 0x0077_34f0 => extern "C" fn kernel_save_command_007734f0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3115, call_sym)
    }
    /// `KernelSaveCommand_301b_00773510(chr)`.
    KernelSaveCommand_301b_00773510 @ 0x0077_3510 => extern "C" fn kernel_save_command_00773510(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x301b, call_sym)
    }
    /// `KernelSaveCommand_3116_00773540(chr)`.
    KernelSaveCommand_3116_00773540 @ 0x0077_3540 => extern "C" fn kernel_save_command_00773540(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3116, call_sym)
    }
    /// `KernelSaveCommand_301c_00773560(chr)`.
    KernelSaveCommand_301c_00773560 @ 0x0077_3560 => extern "C" fn kernel_save_command_00773560(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x301c, call_sym)
    }
    /// `KernelSaveCommand_3117_00773580(chr)`.
    KernelSaveCommand_3117_00773580 @ 0x0077_3580 => extern "C" fn kernel_save_command_00773580(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3117, call_sym)
    }
    /// `KernelSaveCommand_301d_007735a0(chr)`.
    KernelSaveCommand_301d_007735a0 @ 0x0077_35a0 => extern "C" fn kernel_save_command_007735a0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x301d, call_sym)
    }
    /// `KernelSaveCommand_3118_007735c0(chr)`.
    KernelSaveCommand_3118_007735c0 @ 0x0077_35c0 => extern "C" fn kernel_save_command_007735c0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3118, call_sym)
    }
    /// `KernelSaveCommand_301e_007735e0(chr)`.
    KernelSaveCommand_301e_007735e0 @ 0x0077_35e0 => extern "C" fn kernel_save_command_007735e0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x301e, call_sym)
    }
    /// `KernelSaveCommand_3119_00773600(chr)`.
    KernelSaveCommand_3119_00773600 @ 0x0077_3600 => extern "C" fn kernel_save_command_00773600(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3119, call_sym)
    }
    /// `KernelSaveCommand_301f_00773620(chr)`.
    KernelSaveCommand_301f_00773620 @ 0x0077_3620 => extern "C" fn kernel_save_command_00773620(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x301f, call_sym)
    }
    /// `KernelSaveCommand_311a_00773640(chr)`.
    KernelSaveCommand_311a_00773640 @ 0x0077_3640 => extern "C" fn kernel_save_command_00773640(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x311a, call_sym)
    }
    /// `KernelSaveCommand_3020_00773660(chr)`.
    KernelSaveCommand_3020_00773660 @ 0x0077_3660 => extern "C" fn kernel_save_command_00773660(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3020, call_sym)
    }
    /// `KernelSaveCommand_311b_00773680(chr)`.
    KernelSaveCommand_311b_00773680 @ 0x0077_3680 => extern "C" fn kernel_save_command_00773680(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x311b, call_sym)
    }
    /// `KernelSaveCommand_3021_007736a0(chr)`.
    KernelSaveCommand_3021_007736a0 @ 0x0077_36a0 => extern "C" fn kernel_save_command_007736a0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3021, call_sym)
    }
    /// `KernelSaveCommand_311c_007736c0(chr)`.
    KernelSaveCommand_311c_007736c0 @ 0x0077_36c0 => extern "C" fn kernel_save_command_007736c0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x311c, call_sym)
    }
    /// `KernelSaveCommand_3022_007736e0(chr)`.
    KernelSaveCommand_3022_007736e0 @ 0x0077_36e0 => extern "C" fn kernel_save_command_007736e0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3022, call_sym)
    }
    /// `KernelSaveCommand_311d_00773700(chr)`.
    KernelSaveCommand_311d_00773700 @ 0x0077_3700 => extern "C" fn kernel_save_command_00773700(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x311d, call_sym)
    }
    /// `KernelSaveCommand_3023_00773720(chr)`.
    KernelSaveCommand_3023_00773720 @ 0x0077_3720 => extern "C" fn kernel_save_command_00773720(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3023, call_sym)
    }
    /// `KernelSaveCommand_311e_00773740(chr)`.
    KernelSaveCommand_311e_00773740 @ 0x0077_3740 => extern "C" fn kernel_save_command_00773740(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x311e, call_sym)
    }
    /// `KernelSaveCommand_3024_00773760(chr)`.
    KernelSaveCommand_3024_00773760 @ 0x0077_3760 => extern "C" fn kernel_save_command_00773760(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3024, call_sym)
    }
    /// `KernelSaveCommand_3025_007737c0(chr)`.
    KernelSaveCommand_3025_007737c0 @ 0x0077_37c0 => extern "C" fn kernel_save_command_007737c0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3025, call_sym)
    }
    /// `KernelSaveCommand_3026_007737e0(chr)`.
    KernelSaveCommand_3026_007737e0 @ 0x0077_37e0 => extern "C" fn kernel_save_command_007737e0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3026, call_sym)
    }
    /// `KernelSaveCommand_3027_00773840(chr)`.
    KernelSaveCommand_3027_00773840 @ 0x0077_3840 => extern "C" fn kernel_save_command_00773840(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3027, call_sym)
    }
    /// `KernelSaveCommand_3028_007738c0(chr)`.
    KernelSaveCommand_3028_007738c0 @ 0x0077_38c0 => extern "C" fn kernel_save_command_007738c0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3028, call_sym)
    }
    /// `KernelSaveCommand_3029_00773900(chr)`.
    KernelSaveCommand_3029_00773900 @ 0x0077_3900 => extern "C" fn kernel_save_command_00773900(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3029, call_sym)
    }
    /// `KernelSaveCommand_302b_00773950(chr)`.
    KernelSaveCommand_302b_00773950 @ 0x0077_3950 => extern "C" fn kernel_save_command_00773950(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x302b, call_sym)
    }
    /// `KernelSaveCommand_302c_007739a0(chr)`.
    KernelSaveCommand_302c_007739a0 @ 0x0077_39a0 => extern "C" fn kernel_save_command_007739a0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x302c, call_sym)
    }
    /// `KernelSaveCommand_302d_00773a10(chr)`.
    KernelSaveCommand_302d_00773a10 @ 0x0077_3a10 => extern "C" fn kernel_save_command_00773a10(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x302d, call_sym)
    }
    /// `KernelSaveCommand_302e_00773a80(chr)`.
    KernelSaveCommand_302e_00773a80 @ 0x0077_3a80 => extern "C" fn kernel_save_command_00773a80(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x302e, call_sym)
    }
    /// `KernelSaveCommand_302f_00773af0(chr)`.
    KernelSaveCommand_302f_00773af0 @ 0x0077_3af0 => extern "C" fn kernel_save_command_00773af0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x302f, call_sym)
    }
    /// `KernelSaveCommand_3030_00773b40(chr)`.
    KernelSaveCommand_3030_00773b40 @ 0x0077_3b40 => extern "C" fn kernel_save_command_00773b40(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3030, call_sym)
    }
    /// `KernelSaveCommand_3031_00773ba0(chr)`.
    KernelSaveCommand_3031_00773ba0 @ 0x0077_3ba0 => extern "C" fn kernel_save_command_00773ba0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3031, call_sym)
    }
    /// `KernelSaveCommand_3032_00773be0(chr)`.
    KernelSaveCommand_3032_00773be0 @ 0x0077_3be0 => extern "C" fn kernel_save_command_00773be0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3032, call_sym)
    }
    /// `KernelSaveCommand_3033_00773c20(chr)`.
    KernelSaveCommand_3033_00773c20 @ 0x0077_3c20 => extern "C" fn kernel_save_command_00773c20(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3033, call_sym)
    }
    /// `KernelSaveCommand_3034_00773c90(chr)`.
    KernelSaveCommand_3034_00773c90 @ 0x0077_3c90 => extern "C" fn kernel_save_command_00773c90(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3034, call_sym)
    }
    /// `KernelSaveCommand_3035_00773cd0(chr)`.
    KernelSaveCommand_3035_00773cd0 @ 0x0077_3cd0 => extern "C" fn kernel_save_command_00773cd0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3035, call_sym)
    }
    /// `KernelSaveCommand_3036_00773d10(chr)`.
    KernelSaveCommand_3036_00773d10 @ 0x0077_3d10 => extern "C" fn kernel_save_command_00773d10(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3036, call_sym)
    }
    /// `KernelSaveCommand_3037_00773d50(chr)`.
    KernelSaveCommand_3037_00773d50 @ 0x0077_3d50 => extern "C" fn kernel_save_command_00773d50(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3037, call_sym)
    }
    /// `KernelSaveCommand_3038_00773d90(chr)`.
    KernelSaveCommand_3038_00773d90 @ 0x0077_3d90 => extern "C" fn kernel_save_command_00773d90(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3038, call_sym)
    }
    /// `KernelSaveCommand_3039_00773dd0(chr)`.
    KernelSaveCommand_3039_00773dd0 @ 0x0077_3dd0 => extern "C" fn kernel_save_command_00773dd0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3039, call_sym)
    }
    /// `KernelSaveCommand_303a_00773e10(chr)`.
    KernelSaveCommand_303a_00773e10 @ 0x0077_3e10 => extern "C" fn kernel_save_command_00773e10(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x303a, call_sym)
    }
    /// `KernelSaveCommand_303b_00773e50(chr)`.
    KernelSaveCommand_303b_00773e50 @ 0x0077_3e50 => extern "C" fn kernel_save_command_00773e50(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x303b, call_sym)
    }
    /// `KernelSaveCommand_303c_00773e90(chr)`.
    KernelSaveCommand_303c_00773e90 @ 0x0077_3e90 => extern "C" fn kernel_save_command_00773e90(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x303c, call_sym)
    }
    /// `KernelSaveCommand_303d_00773ed0(chr)`.
    KernelSaveCommand_303d_00773ed0 @ 0x0077_3ed0 => extern "C" fn kernel_save_command_00773ed0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x303d, call_sym)
    }
    /// `KernelSaveCommand_303e_00773f10(chr)`.
    KernelSaveCommand_303e_00773f10 @ 0x0077_3f10 => extern "C" fn kernel_save_command_00773f10(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x303e, call_sym)
    }
    /// `KernelSaveCommand_303f_00773f50(chr)`.
    KernelSaveCommand_303f_00773f50 @ 0x0077_3f50 => extern "C" fn kernel_save_command_00773f50(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x303f, call_sym)
    }
    /// `KernelSaveCommand_3040_00773f90(chr)`.
    KernelSaveCommand_3040_00773f90 @ 0x0077_3f90 => extern "C" fn kernel_save_command_00773f90(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3040, call_sym)
    }
    /// `KernelSaveCommand_3041_00773fd0(chr)`.
    KernelSaveCommand_3041_00773fd0 @ 0x0077_3fd0 => extern "C" fn kernel_save_command_00773fd0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3041, call_sym)
    }
    /// `KernelSaveCommand_3042_00774010(chr)`.
    KernelSaveCommand_3042_00774010 @ 0x0077_4010 => extern "C" fn kernel_save_command_00774010(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3042, call_sym)
    }
    /// `KernelSaveCommand_3043_00774050(chr)`.
    KernelSaveCommand_3043_00774050 @ 0x0077_4050 => extern "C" fn kernel_save_command_00774050(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3043, call_sym)
    }
    /// `KernelSaveCommand_3044_00774090(chr)`.
    KernelSaveCommand_3044_00774090 @ 0x0077_4090 => extern "C" fn kernel_save_command_00774090(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3044, call_sym)
    }
    /// `KernelSaveCommand_3045_007740d0(chr)`.
    KernelSaveCommand_3045_007740d0 @ 0x0077_40d0 => extern "C" fn kernel_save_command_007740d0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3045, call_sym)
    }
    /// `KernelSaveCommand_3046_00774110(chr)`.
    KernelSaveCommand_3046_00774110 @ 0x0077_4110 => extern "C" fn kernel_save_command_00774110(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3046, call_sym)
    }
    /// `KernelSaveCommand_3047_00774150(chr)`.
    KernelSaveCommand_3047_00774150 @ 0x0077_4150 => extern "C" fn kernel_save_command_00774150(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3047, call_sym)
    }
    /// `KernelSaveCommand_3048_00774190(chr)`.
    KernelSaveCommand_3048_00774190 @ 0x0077_4190 => extern "C" fn kernel_save_command_00774190(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3048, call_sym)
    }
    /// `KernelSaveCommand_3049_007741d0(chr)`.
    KernelSaveCommand_3049_007741d0 @ 0x0077_41d0 => extern "C" fn kernel_save_command_007741d0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3049, call_sym)
    }
    /// `KernelSaveCommand_304a_00774210(chr)`.
    KernelSaveCommand_304a_00774210 @ 0x0077_4210 => extern "C" fn kernel_save_command_00774210(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x304a, call_sym)
    }
    /// `KernelSaveCommand_304b_00774250(chr)`.
    KernelSaveCommand_304b_00774250 @ 0x0077_4250 => extern "C" fn kernel_save_command_00774250(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x304b, call_sym)
    }
    /// `KernelSaveCommand_304c_00774290(chr)`.
    KernelSaveCommand_304c_00774290 @ 0x0077_4290 => extern "C" fn kernel_save_command_00774290(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x304c, call_sym)
    }
    /// `KernelSaveCommand_304d_007742d0(chr)`.
    KernelSaveCommand_304d_007742d0 @ 0x0077_42d0 => extern "C" fn kernel_save_command_007742d0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x304d, call_sym)
    }
    /// `KernelSaveCommand_304e_00774310(chr)`.
    KernelSaveCommand_304e_00774310 @ 0x0077_4310 => extern "C" fn kernel_save_command_00774310(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x304e, call_sym)
    }
    /// `KernelSaveCommand_304f_00774350(chr)`.
    KernelSaveCommand_304f_00774350 @ 0x0077_4350 => extern "C" fn kernel_save_command_00774350(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x304f, call_sym)
    }
    /// `KernelSaveCommand_3050_00774370(chr)`.
    KernelSaveCommand_3050_00774370 @ 0x0077_4370 => extern "C" fn kernel_save_command_00774370(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3050, call_sym)
    }
    /// `KernelSaveCommand_3051_007743c0(chr)`.
    KernelSaveCommand_3051_007743c0 @ 0x0077_43c0 => extern "C" fn kernel_save_command_007743c0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3051, call_sym)
    }
    /// `KernelSaveCommand_3052_00774410(chr)`.
    KernelSaveCommand_3052_00774410 @ 0x0077_4410 => extern "C" fn kernel_save_command_00774410(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3052, call_sym)
    }
    /// `KernelSaveCommand_3053_00774430(chr)`.
    KernelSaveCommand_3053_00774430 @ 0x0077_4430 => extern "C" fn kernel_save_command_00774430(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3053, call_sym)
    }
    /// `KernelSaveCommand_3054_00774480(chr)`.
    KernelSaveCommand_3054_00774480 @ 0x0077_4480 => extern "C" fn kernel_save_command_00774480(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3054, call_sym)
    }
    /// `KernelSaveCommand_3055_007744d0(chr)`.
    KernelSaveCommand_3055_007744d0 @ 0x0077_44d0 => extern "C" fn kernel_save_command_007744d0(chr: u32) -> u32 {
        tolib::bridge::kernel_save_command(&mut mem(), chr, 0x3055, call_sym)
    }
    /// `KernelSlotAbilityName_14e_00772500(i)`.
    KernelSlotAbilityName_14e_00772500 @ 0x0077_2500 => extern "C" fn kernel_slot_name_00772500(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x14e, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_150_00772560(i)`.
    KernelSlotAbilityName_150_00772560 @ 0x0077_2560 => extern "C" fn kernel_slot_name_00772560(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x150, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_152_007725f0(i)`.
    KernelSlotAbilityName_152_007725f0 @ 0x0077_25f0 => extern "C" fn kernel_slot_name_007725f0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x152, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_154_00772680(i)`.
    KernelSlotAbilityName_154_00772680 @ 0x0077_2680 => extern "C" fn kernel_slot_name_00772680(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x154, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_156_007726e0(i)`.
    KernelSlotAbilityName_156_007726e0 @ 0x0077_26e0 => extern "C" fn kernel_slot_name_007726e0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x156, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_158_00772740(i)`.
    KernelSlotAbilityName_158_00772740 @ 0x0077_2740 => extern "C" fn kernel_slot_name_00772740(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x158, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_15a_007727a0(i)`.
    KernelSlotAbilityName_15a_007727a0 @ 0x0077_27a0 => extern "C" fn kernel_slot_name_007727a0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x15a, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_15c_00772830(i)`.
    KernelSlotAbilityName_15c_00772830 @ 0x0077_2830 => extern "C" fn kernel_slot_name_00772830(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x15c, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_15e_007728b0(i)`.
    KernelSlotAbilityName_15e_007728b0 @ 0x0077_28b0 => extern "C" fn kernel_slot_name_007728b0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x15e, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_160_00772920(i)`.
    KernelSlotAbilityName_160_00772920 @ 0x0077_2920 => extern "C" fn kernel_slot_name_00772920(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x160, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_162_00772980(i)`.
    KernelSlotAbilityName_162_00772980 @ 0x0077_2980 => extern "C" fn kernel_slot_name_00772980(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x162, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_164_00772a00(i)`.
    KernelSlotAbilityName_164_00772a00 @ 0x0077_2a00 => extern "C" fn kernel_slot_name_00772a00(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x164, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_166_00772a90(i)`.
    KernelSlotAbilityName_166_00772a90 @ 0x0077_2a90 => extern "C" fn kernel_slot_name_00772a90(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x166, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_168_00772b00(i)`.
    KernelSlotAbilityName_168_00772b00 @ 0x0077_2b00 => extern "C" fn kernel_slot_name_00772b00(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x168, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_16a_00772b70(i)`.
    KernelSlotAbilityName_16a_00772b70 @ 0x0077_2b70 => extern "C" fn kernel_slot_name_00772b70(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x16a, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_16c_00772be0(i)`.
    KernelSlotAbilityName_16c_00772be0 @ 0x0077_2be0 => extern "C" fn kernel_slot_name_00772be0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x16c, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_16e_00772c70(i)`.
    KernelSlotAbilityName_16e_00772c70 @ 0x0077_2c70 => extern "C" fn kernel_slot_name_00772c70(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x16e, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_170_00772d00(i)`.
    KernelSlotAbilityName_170_00772d00 @ 0x0077_2d00 => extern "C" fn kernel_slot_name_00772d00(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x170, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_172_00772d70(i)`.
    KernelSlotAbilityName_172_00772d70 @ 0x0077_2d70 => extern "C" fn kernel_slot_name_00772d70(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x172, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_174_00772df0(i)`.
    KernelSlotAbilityName_174_00772df0 @ 0x0077_2df0 => extern "C" fn kernel_slot_name_00772df0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x174, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_176_00772e70(i)`.
    KernelSlotAbilityName_176_00772e70 @ 0x0077_2e70 => extern "C" fn kernel_slot_name_00772e70(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x176, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_178_00772f00(i)`.
    KernelSlotAbilityName_178_00772f00 @ 0x0077_2f00 => extern "C" fn kernel_slot_name_00772f00(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x178, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_17a_00772f70(i)`.
    KernelSlotAbilityName_17a_00772f70 @ 0x0077_2f70 => extern "C" fn kernel_slot_name_00772f70(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x17a, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_17c_00772fe0(i)`.
    KernelSlotAbilityName_17c_00772fe0 @ 0x0077_2fe0 => extern "C" fn kernel_slot_name_00772fe0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x17c, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_17e_00773050(i)`.
    KernelSlotAbilityName_17e_00773050 @ 0x0077_3050 => extern "C" fn kernel_slot_name_00773050(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x17e, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_180_007730e0(i)`.
    KernelSlotAbilityName_180_007730e0 @ 0x0077_30e0 => extern "C" fn kernel_slot_name_007730e0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x180, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_182_00773170(i)`.
    KernelSlotAbilityName_182_00773170 @ 0x0077_3170 => extern "C" fn kernel_slot_name_00773170(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x182, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_184_007731e0(i)`.
    KernelSlotAbilityName_184_007731e0 @ 0x0077_31e0 => extern "C" fn kernel_slot_name_007731e0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x184, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_186_00773250(i)`.
    KernelSlotAbilityName_186_00773250 @ 0x0077_3250 => extern "C" fn kernel_slot_name_00773250(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x186, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_188_007732c0(i)`.
    KernelSlotAbilityName_188_007732c0 @ 0x0077_32c0 => extern "C" fn kernel_slot_name_007732c0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x188, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_18a_00773350(i)`.
    KernelSlotAbilityName_18a_00773350 @ 0x0077_3350 => extern "C" fn kernel_slot_name_00773350(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x18a, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_18c_007733e0(i)`.
    KernelSlotAbilityName_18c_007733e0 @ 0x0077_33e0 => extern "C" fn kernel_slot_name_007733e0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x18c, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_18e_00773450(i)`.
    KernelSlotAbilityName_18e_00773450 @ 0x0077_3450 => extern "C" fn kernel_slot_name_00773450(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x18e, 0x0077_4720, call_sym)
    }
    /// `KernelSlotAbilityName_190_007734c0(i)`.
    KernelSlotAbilityName_190_007734c0 @ 0x0077_34c0 => extern "C" fn kernel_slot_name_007734c0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x190, 0x0077_4720, call_sym)
    }
    /// `KernelSlotItemName_08c_00774390(i)`.
    KernelSlotItemName_08c_00774390 @ 0x0077_4390 => extern "C" fn kernel_slot_name_00774390(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x8c, 0x0077_67a0, call_sym)
    }
    /// `KernelSlotItemName_08e_007743e0(i)`.
    KernelSlotItemName_08e_007743e0 @ 0x0077_43e0 => extern "C" fn kernel_slot_name_007743e0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x8e, 0x0077_67a0, call_sym)
    }
    /// `KernelSlotItemName_090_00774450(i)`.
    KernelSlotItemName_090_00774450 @ 0x0077_4450 => extern "C" fn kernel_slot_name_00774450(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x90, 0x0077_67a0, call_sym)
    }
    /// `KernelSlotItemName_092_007744a0(i)`.
    KernelSlotItemName_092_007744a0 @ 0x0077_44a0 => extern "C" fn kernel_slot_name_007744a0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x92, 0x0077_67a0, call_sym)
    }
    /// `KernelSlotItemName_098_007744f0(i)`.
    KernelSlotItemName_098_007744f0 @ 0x0077_44f0 => extern "C" fn kernel_slot_name_007744f0(i: u32) -> u32 {
        tolib::bridge::kernel_slot_name(&mut mem(), i, 0x98, 0x0077_67a0, call_sym)
    }
    /// `KernelPostRequest1_007739c0(v)`.
    KernelPostRequest1_007739c0 @ 0x0077_39c0 => extern "C" fn kernel_post_request_007739c0(v: u32) -> u32 {
        tolib::bridge::kernel_post_request(&mut mem(), v, 1)
    }
    /// `KernelPostRequest0_00773a30(v)`.
    KernelPostRequest0_00773a30 @ 0x0077_3a30 => extern "C" fn kernel_post_request_00773a30(v: u32) -> u32 {
        tolib::bridge::kernel_post_request(&mut mem(), v, 0)
    }
    /// `KernelPostRequest3_00773aa0(v)`.
    KernelPostRequest3_00773aa0 @ 0x0077_3aa0 => extern "C" fn kernel_post_request_00773aa0(v: u32) -> u32 {
        tolib::bridge::kernel_post_request(&mut mem(), v, 3)
    }
    /// `KernelSlotWord194_00772dc0(i)`.
    KernelSlotWord194_00772dc0 @ 0x0077_2dc0 => extern "C" fn kernel_slot_word_194(i: u32) -> u32 {
        tolib::bridge::kernel_slot_word_194(&mem(), i)
    }
    /// `KernelGilAfter_00772e40(i)`.
    KernelGilAfter_00772e40 @ 0x0077_2e40 => extern "C" fn kernel_gil_after(i: u32) -> u32 {
        tolib::bridge::kernel_gil_after(&mut mem(), i, call_sym)
    }
    /// `KernelPlaySfx_00773b10(sfx)`.
    KernelPlaySfx_00773b10 @ 0x0077_3b10 => extern "C" fn kernel_play_sfx(sfx: u32) -> u32 {
        tolib::bridge::kernel_play_sfx(&mut mem(), sfx, call_sym)
    }
    /// `KernelEventByte_007725b0(i)`.
    KernelEventByte_007725b0 @ 0x0077_25b0 => extern "C" fn kernel_event_byte(i: u32) -> u32 {
        tolib::bridge::kernel_event_byte(&mut mem(), i, call_sym)
    }
    /// `KernelSetEventByte_00773800(i, v)`.
    KernelSetEventByte_00773800 @ 0x0077_3800 => extern "C" fn kernel_set_event_byte(i: u32, v: u32) -> u32 {
        tolib::bridge::kernel_set_event_byte(&mut mem(), i, v, call_sym)
    }
    /// `KernelJoined_00772900(chr)`.
    KernelJoined_00772900 @ 0x0077_2900 => extern "C" fn kernel_joined(chr: u32) -> u32 {
        tolib::bridge::kernel_joined(&mut mem(), chr, call_sym)
    }
    /// `KernelEventBit_00772620(n)`.
    KernelEventBit_00772620 @ 0x0077_2620 => extern "C" fn kernel_event_bit(n: u32) -> u32 {
        tolib::bridge::kernel_event_bit(&mut mem(), n, call_sym)
    }
    /// `KernelSetEventBit_00773880(n, on)`.
    KernelSetEventBit_00773880 @ 0x0077_3880 => extern "C" fn kernel_set_event_bit(n: u32, on: u32) -> u32 {
        tolib::bridge::kernel_set_event_bit(&mut mem(), n, on, call_sym)
    }
    /// `MagicFile_Stop()`.
    MagicFile_Stop @ 0x009d_a880 => extern "C" fn magic_file_stop() -> u32 {
        menu::bridge::magic_file_stop(&mut mem(), call_sym, |m, fmt, arg| {
            menu::bridge::MagicFileCalls::printf(&mut GameMenu, m, fmt, arg);
        })
    }
    /// `MagicFile_SetAsyncLoad()`.
    MagicFile_SetAsyncLoad @ 0x009d_afe0 => extern "C" fn magic_file_set_async_load() -> u32 {
        menu::bridge::magic_file_async_load(&mut mem(), call_sym)
    }
    /// `MagicFile_ResetPreloaded()`.
    MagicFile_ResetPreloaded @ 0x009d_adc0 => extern "C" fn magic_file_reset_preloaded() -> u32 {
        menu::bridge::magic_file_reset_preloaded(&mut mem())
    }
    /// `CheckPreviousLevel_00872a00()`.
    CheckPreviousLevel_00872a00 @ 0x0087_2a00 => extern "C" fn check_previous_level() -> u32 {
        tolib::bridge::previous_level_check(&mut mem(), call_sym)
    }
    /// `Fixup_WriteLeb128(value, &cursor)`.
    Fixup_WriteLeb128 @ 0x0060_aba0 => extern "C" fn fixup_write_leb128(value: u32, cursor: u32) {
        fixup::bridge::write_leb128(&mut mem(), value, cursor);
    }
    /// `TrophyManager::Install(arg)`: 0.
    TrophyManager_Install @ 0x0059_e520 => extern "thiscall" fn trophy_manager_install(_this: u32, _arg: u32) -> u32 {
        stub::TROPHY_INSTALL
    }
    /// `std::map` iterator `++` (fastcall).
    std_Tree_const_iterator_inc_006352b0 @ 0x0063_52b0 => extern "fastcall" fn tree_iterator_inc_006352b0(it: u32) -> u32 {
        stl::bridge::tree_iterator_inc(&mut mem(), it)
    }
    /// `std::map` iterator `++` (fastcall).
    std_Tree_const_iterator_inc_00635300 @ 0x0063_5300 => extern "fastcall" fn tree_iterator_inc_00635300(it: u32) -> u32 {
        stl::bridge::tree_iterator_inc(&mut mem(), it)
    }
    /// `std::map` iterator `++` (fastcall).
    std_Tree_const_iterator_inc_00635350 @ 0x0063_5350 => extern "fastcall" fn tree_iterator_inc_00635350(it: u32) -> u32 {
        stl::bridge::tree_iterator_inc(&mut mem(), it)
    }
    /// `std::map` iterator `++` (fastcall).
    std_Tree_const_iterator_inc_00699ee0 @ 0x0069_9ee0 => extern "fastcall" fn tree_iterator_inc_00699ee0(it: u32) -> u32 {
        stl::bridge::tree_iterator_inc(&mut mem(), it)
    }
    /// `TODrawScissorXYWH(x, y, w, h)`.
    TODrawScissorXYWH @ 0x008f_9230 => extern "C" fn to_draw_scissor_xywh(x: i32, y: i32, w: i32, h: i32) -> u32 {
        pkt::bridge::draw_scissor_xywh(&mut mem(), x, y, w, h)
    }
    /// `Kernel_GetItemName(id)`.
    Kernel_GetItemName @ 0x0077_67a0 => extern "C" fn kernel_get_item_name(id: u32) -> u32 {
        names::bridge::item_name(&mem(), id)
    }
}

/// `TOMenuGetGetImportant*`: the key item's text at `field`, with the
/// callees in `ffx-battle` (`MsImportantPtr` stores the strings base into a
/// local, as the original's).
fn important_text(id: u32, field: u32) -> u32 {
    let hiragana = ffx_battle::save::bridge::ms_get_save_config_hiragana(&mem());
    let mut strings = 0u32;
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    let out = std::ptr::from_mut(&mut strings) as usize as u32;
    let record = ffx_battle::save::bridge::ms_important_ptr(&mut mem(), id, out);
    let strings = mem().read::<u32>(out);
    menu::bridge::important_text(&mem(), field, hiragana, record, strings)
}
