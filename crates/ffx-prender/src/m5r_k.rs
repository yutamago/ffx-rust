//! M5 stage 1, list `render`, batch 11: the magic-file PRX load, the
//! auto-test manager's deleting destructor and a debug text panel's
//! constructor.

use crate::m5r_c::quiet;
use ffx_bridge::Global;

/// `g_PrxState`, `g_PrxModule`, the async load's path buffer (0x81
/// bytes) and its callback word.
pub const PRX_STATE: Global<u32> = Global::new(0x00ce_83c4);
pub const PRX_MODULE: Global<u32> = Global::new(0x00ce_83c0);
pub const PRX_PATH: u32 = 0x00ce_83d0;
pub const PRX_CALLBACK: Global<u32> = Global::new(0x00ce_8454);

pub const ASYNC_AVAILABLE: u32 = 0x0064_1c60;
pub const FILE_PROBE: u32 = 0x0060_7e00;
pub const MEMSET: u32 = 0x0094_964c;
pub const SPRINTF_S: u32 = 0x00b0_c394;
pub const LOAD_LIBRARY_A: u32 = 0x00b0_c07c;
/// `"%s"`.
pub const FORMAT_S: u32 = 0x00b0_e160;
pub const REGIST_ASYNC_TASK: u32 = 0x0064_2950;
pub const MAGIC_DLL_ASYNC_LOAD: u32 = 0x006f_9cc0;
/// `AutoTestManager_vftable`.
pub const VT_AUTO_TEST: u32 = 0x00b4_8e30;
pub const UNINIT_SAVELOAD: u32 = 0x006f_2ba0;
pub const STRING_VECTOR_DESTROY: u32 = 0x0063_5c40;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const TEXT_UTIL_CTOR: u32 = 0x006f_5b50;
/// The panel's default scale (f32 0.02).
pub const PANEL_SCALE: u32 = 0x00b4_82d8;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    /// `PhyreFIOS_LoadsPrx(path, x, callback)` (`x` is never read): with
    /// async loads available, a path that exists (`File_Probe`) is copied
    /// into the PRX buffer (cleared, `sprintf_s("%s")`), the callback
    /// kept and `MagicDLLAsyncLoad` registered as a task: 1 (0 when it
    /// doesn't exist). Otherwise `LoadLibraryA(path)` now (state 1, then
    /// 3) and, when it loaded, `callback(module)`: the module (0 if not).
    pub fn loads_prx(h: &mut impl Host, path: u32, callback: u32) -> u32 {
        PRX_MODULE.set(h, 0);
        if cdecl(h, ASYNC_AVAILABLE, &[]) != 0 {
            if cdecl(h, FILE_PROBE, &[path]).to_le_bytes()[0] == 0 {
                return 0;
            }
            let buf = h.rebase(PRX_PATH);
            PRX_STATE.set(h, 1);
            PRX_MODULE.set(h, 0);
            cdecl(h, MEMSET, &[buf, 0, 0x81]);
            let f: u32 = h.read(h.rebase(SPRINTF_S));
            let fmt = h.rebase(FORMAT_S);
            h.call(f, Conv::Cdecl, &[buf, 0x81, fmt, path]);
            PRX_CALLBACK.set(h, callback);
            let task = h.rebase(MAGIC_DLL_ASYNC_LOAD);
            cdecl(h, REGIST_ASYNC_TASK, &[task, buf]);
            return 1;
        }
        PRX_STATE.set(h, 1);
        let f: u32 = h.read(h.rebase(LOAD_LIBRARY_A));
        let m = h.call(f, Conv::Stdcall, &[path]);
        PRX_MODULE.set(h, m);
        PRX_STATE.set(h, 3);
        if m == 0 {
            return 0;
        }
        h.call(callback, Conv::Cdecl, &[m]);
        PRX_MODULE.get(h)
    }

    /// `AutoTestManager_scalarDeletingDtor_006bcbd0(this; flags)` (`ret
    /// 4`) -> `this`: the vtable stored, `nx_uninit_saveload(+0x124)`, the
    /// string vector `+0x118` (begin, end, capacity) destroyed and freed
    /// when it has storage, cleared; deleted with flag bit 0.
    pub fn auto_test_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(VT_AUTO_TEST);
        let x = word(h, this + 0x124);
        h.write(this, vt);
        cdecl(h, UNINIT_SAVELOAD, &[x]);
        let v = this + 0x118;
        let first = word(h, v);
        if first != 0 {
            let last = word(h, v + 4);
            h.call_sym(STRING_VECTOR_DESTROY, Conv::Thiscall(v), &[first, last]);
            let first = word(h, v);
            cdecl(h, OPERATOR_DELETE, &[first]);
            h.write(v, 0_u32);
            h.write(v + 4, 0_u32);
            h.write(v + 8, 0_u32);
        }
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `DebugTextPanel_ctor_006b53b0(this)` -> `this`: byte `+0x1c` 0, the
    /// text `+0x20` constructed, scale `+0x100` (0.02), `+0xf4`/`+0xf8`
    /// 0, byte `+0xfc` 0, size 1280 x 720, the three floats `+0x10c..`
    /// 0, `+0x118` 1, `+0`/`+4` 0, the 0x44 bytes at `+0xb0` cleared.
    pub fn debug_text_panel_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 0x1c, 0_u8);
        h.call_sym(TEXT_UTIL_CTOR, Conv::Thiscall(this + 0x20), &[]);
        let scale: u32 = h.read(h.rebase(PANEL_SCALE));
        h.write(this + 0x100, quiet(scale));
        h.write(this + 0xf4, 0_u32);
        h.write(this + 0xf8, 0_u32);
        h.write(this + 0xfc, 0_u8);
        h.write(this + 0x104, 0x500_u32);
        h.write(this + 0x108, 0x2d0_u32);
        h.write(this + 0x10c, 0_u32);
        h.write(this + 0x110, 0_u32);
        h.write(this + 0x114, 0_u32);
        h.write(this + 0x118, 1.0_f32.to_bits());
        h.write(this, 0_u32);
        h.write(this + 4, 0_u32);
        cdecl(h, MEMSET, &[this + 0xb0, 0, 0x44]);
        this
    }
}
