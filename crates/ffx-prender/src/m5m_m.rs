//! M5 stage 1, list `map`, round 5 batch 9: the guide maps' binary save
//! (`rcbgSaveBinNormal`, `host:<dir>/bin/map2d.bin`): a file chunk, the
//! map system's header, each guide (`rcuiGuideSave_009397f0`), an end
//! chunk, then the host's `chmod 666` on it.
//!
//! The path, command, reply and chunk header live in the original's frame
//! and come in through its `ebp` (the shim passes a `StackFrame`).

pub use crate::m5m_e::{RCUI_WORK, YN_MAP_SYSTEM};

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x318;
/// `host:%s/bin/map2d.bin`, `Error in %s, line %d: `, the source file's
/// name, `Save(Bin.) Failed.`, `chmod 666 %s`, `system %s ret:%d`.
pub const PATH_FORMAT: u32 = 0x00b6_84e8;
pub const ERROR_FORMAT: u32 = 0x00b6_7878;
pub const SOURCE_FILE: u32 = 0x00b6_892c;
pub const FAILED_TEXT: u32 = 0x00b6_894c;
pub const CHMOD_FORMAT: u32 = 0x00b6_8a3c;
pub const SYSTEM_FORMAT: u32 = 0x00b6_8a4c;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `sceOpen(name, mode)` -> fd or -1, `sceWrite(fd, buf, n)`,
/// `sceClose(fd)`, `rcbgChunkTag(i)`, `rcbgSaveChunkHeader_00938f40(size,
/// fd)`, `rcuiGuideSave_009397f0(g, fd)`, `SystemOnHost(command, out)`,
/// `rcPrint(fmt, ...)` (cdecl).
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const SCE_WRITE: u32 = 0x0062_fdc0;
pub const SCE_CLOSE: u32 = 0x0062_f610;
pub const CHUNK_TAG: u32 = 0x0092_1b90;
pub const SAVE_CHUNK_HEADER: u32 = 0x0093_8f40;
pub const GUIDE_SAVE: u32 = 0x0093_97f0;
pub const SYSTEM_ON_HOST: u32 = 0x0084_ede0;
pub const RC_PRINT: u32 = 0x0092_7550;

pub mod bridge {
    use super::{
        CHMOD_FORMAT, CHUNK_TAG, ERROR_FORMAT, FAILED_TEXT, GUIDE_SAVE, PATH_FORMAT, RC_PRINT,
        RCUI_WORK, SAVE_CHUNK_HEADER, SCE_CLOSE, SCE_OPEN, SCE_WRITE, SOURCE_FILE, SPRINTF_SLOT,
        SYSTEM_FORMAT, SYSTEM_ON_HOST, YN_MAP_SYSTEM,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn sprintf(h: &mut impl Host, args: &[u32]) {
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        h.call(f, Conv::Cdecl, args);
    }

    /// The map system's first word (`*g_YnMapSystem`), 0 without one.
    fn map_system(h: &mut impl Host) -> u32 {
        let sys = YN_MAP_SYSTEM.get(h);
        if sys == 0 { 0 } else { h.read(sys) }
    }

    /// The chunk header: `rcbgChunkTag(i)`'s tag over the zeroed 16 bytes,
    /// written.
    fn chunk(h: &mut impl Host, header: u32, fd: u32, i: u32) {
        let tag_at = cdecl(h, CHUNK_TAG, &[i]);
        let tag: u32 = h.read(tag_at);
        h.write(header, tag);
        cdecl(h, SCE_WRITE, &[fd, header, 0x10]);
    }

    /// `rcbgSaveBinNormal()` -> the last `rcPrint`'s `eax`: the file
    /// `host:` + the rcui work's directory (`+0x174`) + `/bin/map2d.bin`
    /// opened (mode 0x602; failing: an error and "Save(Bin.) Failed."
    /// printed); chunk 0, the map system's header
    /// (`rcbgSaveChunkHeader` of its first word's word), each guide
    /// (`+4`, 0x110 bytes apart, the count `+0` read each time), chunk
    /// 0xc, closed; then `chmod 666` of the path after `host:` run on the
    /// host and its result printed.
    pub fn save_bin_normal(h: &mut impl Host, ebp: u32) -> u32 {
        let guides = map_system(h);
        let path = ebp - 0x104;
        let dir = RCUI_WORK.get(h).wrapping_add(0x174);
        let fmt = h.rebase(PATH_FORMAT);
        sprintf(h, &[path, fmt, dir]);
        let fd = cdecl(h, SCE_OPEN, &[path, 0x602]);
        h.write(ebp - 0x318, fd);
        if fd == u32::MAX {
            let (fmt, file) = (h.rebase(ERROR_FORMAT), h.rebase(SOURCE_FILE));
            cdecl(h, RC_PRINT, &[fmt, file, 0x59]);
            let text = h.rebase(FAILED_TEXT);
            return cdecl(h, RC_PRINT, &[text]);
        }
        let header = ebp - 0x314;
        for k in 0..4 {
            h.write(header + 4 * k, 0_u32);
        }
        chunk(h, header, fd, 0);
        let sys = map_system(h);
        let size: u32 = h.read(sys);
        cdecl(h, SAVE_CHUNK_HEADER, &[size, fd]);
        let mut i = 0_i32;
        let mut at = 0_u32;
        while i < h.read::<i32>(guides) {
            let base: u32 = h.read(guides + 4);
            let fd: u32 = h.read(ebp - 0x318);
            cdecl(h, GUIDE_SAVE, &[base.wrapping_add(at), fd]);
            i += 1;
            at = at.wrapping_add(0x110);
        }
        let fd: u32 = h.read(ebp - 0x318);
        chunk(h, header, fd, 0xc);
        cdecl(h, SCE_CLOSE, &[fd]);
        let command = ebp - 0x204;
        let fmt = h.rebase(CHMOD_FORMAT);
        sprintf(h, &[command, fmt, path + 5]);
        let r = cdecl(h, SYSTEM_ON_HOST, &[command, ebp - 0x304]);
        let fmt = h.rebase(SYSTEM_FORMAT);
        cdecl(h, RC_PRINT, &[fmt, command, r])
    }
}
