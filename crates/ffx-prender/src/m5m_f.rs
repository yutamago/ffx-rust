//! M5 stage 1, list `map`, round 5 batch 2: the debug map tool's text
//! loads and saves: the FP texture count, the guide-map model load and a
//! texture's binary save (`rcbg*`).
//!
//! None of it ran in a coverage run. Callees stay calls to the originals,
//! cdecl as their `ret` says. Stack cookies are left out; frame buffers
//! handed to callees come in as addresses (the shims pass a `StackFrame`).

use ffx_bridge::Global;

/// `g_YnMapSystem`; `g_RcPrim2DPacket` (the 64 KiB host reply buffer).
pub const YN_MAP_SYSTEM: Global<u32> = Global::new(0x0193_bf80);
pub const PRIM_2D_PACKET: Global<u32> = Global::new(0x00c4_98dc);
/// The count's host command format, the numbered name format.
pub const LIST_COMMAND_FORMAT: u32 = 0x00b6_8e24;
pub const NUMBERED_NAME_FORMAT: u32 = 0x00b6_8e00;
/// The guide-map model path format.
pub const GUIDE_MAP_PATH_FORMAT: u32 = 0x00b6_89b0;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `crt_memset_thunk(p, v, n)`, `SystemOnHost(command, out)`,
/// `Gu_MakeFileList(list, pattern)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
pub const SYSTEM_ON_HOST: u32 = 0x0084_ede0;
pub const MAKE_FILE_LIST: u32 = 0x0084_98e0;
/// `rcuiGetStageName(id, out)`, `rcbgModelNew()`, `rcbgModelDelete(m)`,
/// `rcbgModelLoadByExt_0093ad60(m, name)` (cdecl).
pub const GET_STAGE_NAME: u32 = 0x0093_7940;
pub const MODEL_NEW: u32 = 0x0092_83b0;
pub const MODEL_DELETE: u32 = 0x0092_8000;
pub const MODEL_LOAD_BY_EXT: u32 = 0x0093_ad60;
/// `sceWrite(fd, buf, n)` (cdecl).
pub const SCE_WRITE: u32 = 0x0062_fdc0;

pub mod bridge {
    use super::{
        GET_STAGE_NAME, GUIDE_MAP_PATH_FORMAT, LIST_COMMAND_FORMAT, MAKE_FILE_LIST, MEMSET,
        MODEL_DELETE, MODEL_LOAD_BY_EXT, MODEL_NEW, NUMBERED_NAME_FORMAT, PRIM_2D_PACKET,
        SCE_WRITE, SPRINTF_SLOT, SYSTEM_ON_HOST, YN_MAP_SYSTEM,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn sprintf(h: &mut impl Host, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        h.call(f, Conv::Cdecl, args)
    }

    /// Whether the C strings at `a` and `b` are equal.
    fn same_text(h: &impl Host, a: u32, b: u32) -> bool {
        let mut k = 0_u32;
        loop {
            let (x, y): (u8, u8) = (h.read(a.wrapping_add(k)), h.read(b.wrapping_add(k)));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// `rcbgFpTrxTexLoadTextGetTrxCnt(path, base)` -> how many of
    /// `base0_00.tm2`, `base1_00.tm2`, ... exist in a row in `path`'s file
    /// list (`frame`: the original's `ebp - 0x128`: the base at `+0`, the
    /// 3-word list at `+4`, the count at `+0x10`, the host command at
    /// `+0x14` (0x100 bytes), the name at `+0x114`): the host's reply
    /// buffer cleared and the listing asked for; without a reply
    /// (`SystemOnHost` < 0) the list stays empty. A negative list size
    /// never ends, in the original as here.
    pub fn trx_count(h: &mut impl Host, frame: u32, path: u32, base: u32) -> u32 {
        let list = frame + 4;
        let command = frame + 0x14;
        let name = frame + 0x114;
        for k in 0..3 {
            h.write(list + 4 * k, 0_u32);
        }
        h.write(frame, base);
        let fmt = h.rebase(LIST_COMMAND_FORMAT);
        sprintf(h, &[command, fmt, path]);
        let reply = PRIM_2D_PACKET.get(h);
        cdecl(h, MEMSET, &[reply, 0, 0x1_0000]);
        let reply = PRIM_2D_PACKET.get(h);
        if cdecl(h, SYSTEM_ON_HOST, &[command, reply]).cast_signed() >= 0 {
            cdecl(h, MAKE_FILE_LIST, &[list, path]);
        }
        let mut count = 0_u32;
        h.write(frame + 0x10, count);
        loop {
            let fmt = h.rebase(NUMBERED_NAME_FORMAT);
            sprintf(h, &[name, fmt, base, count]);
            let n: u32 = h.read(list);
            let mut found = false;
            let mut i = 0_u32;
            while i.cast_signed() < n.cast_signed() {
                let names: u32 = h.read(list + 8);
                let entry: u32 = h.read(names.wrapping_add(4 * i));
                if same_text(h, entry, name) {
                    found = true;
                    break;
                }
                i += 1;
            }
            if found {
                count += 1;
                h.write(frame + 0x10, count);
            } else if n.cast_signed() >= 0 {
                return count;
            }
        }
    }

    /// `rcbgGuideMapLoadTextSub(id)` -> the load's result, 0 without an
    /// id or stage name: the stage's guide-map model (its 4-character name
    /// into `stage`, the original's `ebp - 0xc`; the path formatted into
    /// `path`, `ebp - 0x10c`) loaded into a new model; loaded with a map
    /// (`[g_YnMapSystem]`), its vertex words cleared (every 6 bytes from
    /// `+8` + 2 of its mesh `+0x14`, `+0x12` of them), `+0xa4` and `+0x94`
    /// 0.0, flags 6 set, and it becomes the map's guide (`[map + 8] +
    /// 0x1c`); otherwise the model is deleted.
    pub fn guide_map_load_text_sub(h: &mut impl Host, stage: u32, path: u32, id: u32) -> u32 {
        if id == 0 || cdecl(h, GET_STAGE_NAME, &[id, stage]) == 0 {
            return 0;
        }
        let c: [u32; 4] =
            core::array::from_fn(|k| i32::from(h.read::<i8>(stage + k as u32)).cast_unsigned());
        let fmt = h.rebase(GUIDE_MAP_PATH_FORMAT);
        sprintf(h, &[path, fmt, c[0], c[1], c[2], c[3], stage]);
        let model = cdecl(h, MODEL_NEW, &[]);
        let r = cdecl(h, MODEL_LOAD_BY_EXT, &[model, path]);
        let sys = YN_MAP_SYSTEM.get(h);
        if r != 0 && sys != 0 && h.read::<u32>(sys) != 0 {
            let mesh: u32 = h.read(model + 0x14);
            let n = i32::from(h.read::<i16>(mesh + 0x12));
            let mut at = mesh.wrapping_add(h.read::<u32>(mesh + 8)).wrapping_add(2);
            for _ in 0..n.max(0) {
                h.write(at, 0_u16);
                at = at.wrapping_add(6);
            }
            h.write(model + 0xa4, 0_u32);
            h.write(model + 0x94, 0_u32);
            let flags: u32 = h.read(model + 0x10);
            h.write(model + 0x10, flags | 6);
            let sys = YN_MAP_SYSTEM.get(h);
            let map = if sys == 0 { 0 } else { h.read::<u32>(sys) };
            let guide = if map == 0 { 0 } else { h.read::<u32>(map + 8) };
            h.write(guide + 0x1c, model);
        } else if model != 0 {
            cdecl(h, MODEL_DELETE, &[model]);
        }
        r
    }

    /// `rcbgTrxSave2_009399f0(t, fd)` -> the last `sceWrite`'s `eax`: the
    /// two 16-bit sizes (`+0x10`, `+0x12`), a 12-byte zero pad (`pad`, the
    /// original's `ebp - 0x14`), then two images as `rcbgTrxSave` writes
    /// one (`+0x38` and `+0x78`: three 8-byte fields, two 4-byte sizes,
    /// their product's bytes from `+0x20` past the fields).
    pub fn trx_save2(h: &mut impl Host, pad: u32, t: u32, fd: u32) -> u32 {
        for k in 0..4 {
            h.write(pad + 4 * k, 0_u32);
        }
        cdecl(h, SCE_WRITE, &[fd, t + 0x10, 2]);
        cdecl(h, SCE_WRITE, &[fd, t + 0x12, 2]);
        cdecl(h, SCE_WRITE, &[fd, pad, 0xc]);
        let mut r = 0;
        for image in [t + 0x38, t + 0x78] {
            for off in [0, 8, 0x10] {
                cdecl(h, SCE_WRITE, &[fd, image + off, 8]);
            }
            cdecl(h, SCE_WRITE, &[fd, image + 0x18, 4]);
            cdecl(h, SCE_WRITE, &[fd, image + 0x1c, 4]);
            let size = h
                .read::<u32>(image + 0x1c)
                .wrapping_mul(h.read(image + 0x18));
            let data: u32 = h.read(image + 0x20);
            r = cdecl(h, SCE_WRITE, &[fd, data, size]);
        }
        r
    }
}
