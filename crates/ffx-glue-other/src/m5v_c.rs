//! M5 stage 1, list `vfx`, batch 3: the VFX shader manager's string-keyed
//! tree insert and texture-name builder, the particle programs' VFX
//! instance setups (`InitVFXRenderData*`), the filter program's
//! destructor, the PS2 packet layer's wait and the background thread's
//! semaphore wait.
//!
//! Stack cookies are left out. Where the original hands a callee a frame
//! buffer, the bridge takes the original's `ebp` (the shim passes a
//! `StackFrame`).

use ffx_bridge::Global;

/// `g_pppCurrentProgram`: the running program's index, the VFX
/// instance's low word.
pub const CURRENT_PROGRAM: Global<u32> = Global::new(0x0230_fd20);
/// `g_TkBgThreadSema`.
pub const BG_THREAD_SEMA: Global<u32> = Global::new(0x0133_0250);
/// `g_pppWaitDone`.
pub const WAIT_DONE: Global<u32> = Global::new(0x010e_7640);
/// The VFX segment divisor (`0x00c3a484`, 2 in the image) the shape
/// setups divide each segment's `+8` by.
pub const SEGMENT_DIVISOR: Global<u32> = Global::new(0x00c3_a484);

/// `pppSendWait`'s message (`scePrintf` format).
pub const SEND_WAIT_FORMAT: u32 = 0x00b5_1be0;
/// The texture folder `fiosUnifyFilename` resolves, and the name format.
pub const TEXTURE_FOLDER: u32 = 0x00b4_3944;
pub const NAME_FORMAT: u32 = 0x00b4_92b4;
/// `MSVCR110!sprintf` (the import slot; cdecl, varargs).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `graphicVFXDestroy(lo, hi)` (cdecl).
pub const VFX_DESTROY: u32 = 0x0064_5280;
/// `WaitSema(sema)` (cdecl).
pub const WAIT_SEMA: u32 = 0x006f_2d80;
/// `sceGsSyncV(mode)` / `scePrintf(format, ...)` (cdecl).
pub const GS_SYNC_V: u32 = 0x0062_f890;
pub const SCE_PRINTF: u32 = 0x0062_fbf0;
/// `createVFXRenderDataBufferRects(lo, hi, n, arg)` (cdecl).
pub const CREATE_RECTS: u32 = 0x0071_4c50;
/// `createVFXRenderDataBufferShp_OShapeVu1(lo, hi, shapes, arg)` (cdecl).
pub const CREATE_SHP_VU1: u32 = 0x0071_5810;
/// `graphicVFXGetRenderDataByInstance(lo, hi)` (cdecl).
pub const GET_RENDER_DATA: u32 = 0x0064_53d0;
/// `graphicVFX2DSetup(lo, hi)` / `graphicVFXSetup(lo, hi)` (cdecl).
pub const VFX_2D_SETUP: u32 = 0x0064_4b50;
pub const VFX_SETUP: u32 = 0x0064_5780;
/// `updateVFXRenderDataRects(lo, hi, p)` (cdecl).
pub const UPDATE_RECTS: u32 = 0x0071_a330;
/// `updateVFXRenderDataShp_OShapeVu1(lo, hi, a, b)` (cdecl).
pub const UPDATE_SHP_VU1: u32 = 0x0071_c0b0;
/// `graphicVFX2DUpdate`, `graphicVFXPath3Update`, `graphicVFXUpdate`
/// (`lo, hi`, cdecl).
pub const VFX_2D_UPDATE: u32 = 0x0064_4bc0;
pub const VFX_PATH3_UPDATE: u32 = 0x0064_5600;
pub const VFX_UPDATE: u32 = 0x0064_58a0;
/// `fiosUnifyFilename(src, out, size)` (cdecl).
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
/// `operator_new_006304c0(n)` / `operator_delete_00630540(p)` (cdecl).
pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
/// `ReturnAddress_00679a00()` (cdecl): the level folder string.
pub const LEVEL_FOLDER: u32 = 0x0067_9a00;

pub mod bridge {
    use super::{
        BG_THREAD_SEMA, CREATE_RECTS, CREATE_SHP_VU1, CURRENT_PROGRAM, GET_RENDER_DATA, GS_SYNC_V,
        LEVEL_FOLDER, NAME_FORMAT, OPERATOR_DELETE_ARRAY, OPERATOR_NEW_ARRAY, SCE_PRINTF,
        SEGMENT_DIVISOR, SEND_WAIT_FORMAT, SPRINTF_SLOT, TEXTURE_FOLDER, UNIFY_FILENAME,
        UPDATE_RECTS, UPDATE_SHP_VU1, VFX_2D_SETUP, VFX_2D_UPDATE, VFX_DESTROY, VFX_PATH3_UPDATE,
        VFX_SETUP, VFX_UPDATE, WAIT_DONE, WAIT_SEMA,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `pppDrawFilterDes(id)` -> `graphicVFXDestroy`'s `eax`: the VFX
    /// instance `(id << 32) + program` (the program sign-extended): low
    /// word the program, high word `id` plus the program's sign.
    pub fn draw_filter_des(h: &mut impl Host, id: u32) -> u32 {
        let prog = CURRENT_PROGRAM.get(h);
        let sign = ((prog as i32) >> 31) as u32;
        cdecl(h, VFX_DESTROY, &[prog, id.wrapping_add(sign)])
    }

    /// `TkWaitSemaBGThread_00888880()` -> `WaitSema(g_TkBgThreadSema)`.
    pub fn wait_sema_bg_thread(h: &mut impl Host) -> u32 {
        let sema = BG_THREAD_SEMA.get(h);
        cdecl(h, WAIT_SEMA, &[sema])
    }

    /// `pppSendWait()`: a v-sync; then until `g_pppWaitDone` is set, a
    /// v-sync and the wait message with the count of waits.
    pub fn send_wait(h: &mut impl Host) {
        cdecl(h, GS_SYNC_V, &[0]);
        let mut i = 0_u32;
        while WAIT_DONE.get(h) == 0 {
            cdecl(h, GS_SYNC_V, &[0]);
            let fmt = h.rebase(SEND_WAIT_FORMAT);
            cdecl(h, SCE_PRINTF, &[fmt, i]);
            i = i.wrapping_add(1);
        }
    }

    /// `InitVFXRenderDataRects(lo, hi, p, arg)`: the rectangles' buffer
    /// (`p`'s `+8` of them); then the render data's kind (`+0xb8` = 2),
    /// the 2D setup, its first update (`updateVFXRenderDataRects(lo, hi,
    /// p)`, `graphicVFX2DUpdate`); a failed buffer or setup destroys the
    /// instance.
    pub fn init_rects(h: &mut impl Host, lo: u32, hi: u32, p: u32, arg: u32) {
        let n: u32 = h.read(p + 8);
        if cdecl(h, CREATE_RECTS, &[lo, hi, n, arg]) == 0 {
            cdecl(h, VFX_DESTROY, &[lo, hi]);
            return;
        }
        let rd = cdecl(h, GET_RENDER_DATA, &[lo, hi]);
        if rd != 0 {
            h.write(rd + 0xb8, 2_u32);
        }
        let ok = cdecl(h, VFX_2D_SETUP, &[lo, hi]);
        cdecl(h, GET_RENDER_DATA, &[lo, hi]);
        if ok == 0 {
            cdecl(h, VFX_DESTROY, &[lo, hi]);
            return;
        }
        cdecl(h, UPDATE_RECTS, &[lo, hi, p]);
        cdecl(h, VFX_2D_UPDATE, &[lo, hi]);
    }

    /// `InitVFXRenderDataShp_OShapeVu1(lo, hi, a, b, c)` (`path3`) and
    /// `InitVFXRenderData_KaGra001` (`graphicVFXUpdate` last): the shape
    /// buffer (`a`, `c`), the VFX setup, each render-data segment's
    /// (`+0x94`, 0x6c bytes, `+0x90` of them) `+8` divided by the segment
    /// divisor, the first update (`a`, `b`); a failed buffer or setup
    /// destroys the instance.
    ///
    /// The original faults on a divisor of 0 (`div`); the port leaves the
    /// word as it is then (a shim must not panic). The image holds 2.
    pub fn init_shp_vu1(h: &mut impl Host, lo: u32, hi: u32, a: u32, b: u32, c: u32, path3: bool) {
        if cdecl(h, CREATE_SHP_VU1, &[lo, hi, a, c]) == 0 {
            cdecl(h, VFX_DESTROY, &[lo, hi]);
            return;
        }
        let ok = cdecl(h, VFX_SETUP, &[lo, hi]);
        let rd = cdecl(h, GET_RENDER_DATA, &[lo, hi]);
        if rd != 0 {
            let mut i = 0_i32;
            while i < h.read::<i32>(rd + 0x90) {
                let segs: u32 = h.read(rd + 0x94);
                let at = segs.wrapping_add(0x6c * i as u32).wrapping_add(8);
                let v: u32 = h.read(at);
                if let Some(q) = v.checked_div(SEGMENT_DIVISOR.get(h)) {
                    h.write(at, q);
                }
                i += 1;
            }
        }
        if ok == 0 {
            cdecl(h, VFX_DESTROY, &[lo, hi]);
            return;
        }
        cdecl(h, UPDATE_SHP_VU1, &[lo, hi, a, b]);
        cdecl(
            h,
            if path3 { VFX_PATH3_UPDATE } else { VFX_UPDATE },
            &[lo, hi],
        );
    }

    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0_u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `VfxShaderManager_combineTextureNameInCurrentLevel(name, out)`
    /// (stdcall; frame: the unified folder at `ebp - 0x104`, `out` at
    /// `ebp - 0x108`): `out` = folder, the name's directory, '/', the
    /// level folder (`ReturnAddress_00679a00`), the name's file part.
    /// The copy is `new[]`'d one byte short (strlen, then the copy with
    /// its terminator) and, for a name without '/', its terminator goes
    /// one byte before the block, as in the original.
    pub fn combine_texture_name(h: &mut impl Host, ebp: u32, name: u32, out: u32) {
        h.write(ebp - 0x108, out);
        let folder = h.rebase(TEXTURE_FOLDER);
        cdecl(h, UNIFY_FILENAME, &[folder, ebp - 0x104, 0x100]);
        let len = strlen(h, name);
        let mut slash = len as i32 - 1;
        while slash > -1 && h.read::<u8>(name.wrapping_add(slash as u32)) != b'/' {
            slash -= 1;
        }
        let buf = cdecl(h, OPERATOR_NEW_ARRAY, &[len]);
        let mut k = 0_u32;
        loop {
            let c: u8 = h.read(name.wrapping_add(k));
            h.write(buf.wrapping_add(k), c);
            if c == 0 {
                break;
            }
            k = k.wrapping_add(1);
        }
        let file = buf.wrapping_add((slash + 1) as u32);
        h.write(buf.wrapping_add(slash as u32), 0_u8);
        let level = cdecl(h, LEVEL_FOLDER, &[]);
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        let fmt = h.rebase(NAME_FORMAT);
        h.call(
            sprintf,
            Conv::Cdecl,
            &[out, fmt, ebp - 0x104, buf, level, file],
        );
        cdecl(h, OPERATOR_DELETE_ARRAY, &[buf]);
    }

    /// The node's string key: `+0xc` less its flag bit.
    fn key(h: &impl Host, node: u32) -> u32 {
        let w: u32 = h.read(node + 0xc);
        w.wrapping_sub(w & 1)
    }

    /// `strcmp(a, b) < 0` as the original's inline compare (unsigned
    /// bytes), or the pointers' order when either is 0.
    fn str_less(h: &impl Host, a: u32, b: u32) -> bool {
        if a == 0 || b == 0 {
            return a < b;
        }
        let mut k = 0_u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(k));
            let y: u8 = h.read(b.wrapping_add(k));
            if x != y {
                return x < y;
            }
            if x == 0 {
                return false;
            }
            k = k.wrapping_add(1);
        }
    }

    /// `VfxShaderTree_insert_006be9e0(tree; node)` -> 0 when inserted, or
    /// the node already holding the key: the node's `+8` flag bit set and
    /// its `+0`/`+4` pointed at the tree; from the root (`+0xc`) down by
    /// key (`+0` the smaller side, `+4` the larger; the tree itself is
    /// the leaf), the node hangs at the end (its `+8` = parent | flag).
    /// An empty tree takes it at `+0xc`, with `tree + 0xc` as the parent.
    pub fn tree_insert(h: &mut impl Host, tree: u32, node: u32) -> u32 {
        let w: u32 = h.read(node + 8);
        h.write(node + 8, w.wrapping_add(1 - (w & 1)));
        h.write(node, tree);
        h.write(node + 4, tree);
        let mut cur: u32 = h.read(tree + 0xc);
        let (mut parent, mut side) = (tree + 0xc, 0_u32);
        if cur != tree {
            let k = key(h, node);
            loop {
                let ck = key(h, cur);
                parent = cur;
                if str_less(h, k, ck) {
                    side = 0;
                } else if str_less(h, ck, k) {
                    side = 1;
                } else {
                    return cur;
                }
                cur = h.read(cur + 4 * side);
                if cur == tree {
                    break;
                }
            }
        }
        let w: u32 = h.read(node + 8);
        h.write(node + 8, (w & 1).wrapping_add(parent));
        h.write(parent + 4 * side, node);
        0
    }
}
