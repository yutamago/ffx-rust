//! M5 stage 1, list `map`, round 5 batch 17: the guide maps' save of an
//! area (`rcbgGuideMapSaveSub`): each map of the area (its two digits
//! over the end of the rcui work's directory and map names) loaded as a
//! scene and saved into `map2d.bin`, then the names put back and the map
//! system's scene loaded again.
//!
//! The scene, names, path and header live in the original's frame and
//! come in through its `ebp` (the shim passes a `StackFrame`).

pub use crate::m5m_e::{RCUI_WORK, YN_MAP_SYSTEM};

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x534;
/// `host:%s/bin/map2d.bin`, `chmod 666 %s`, `system %s ret:%d`.
pub const PATH_FORMAT: u32 = 0x00b6_84e8;
pub const CHMOD_FORMAT: u32 = 0x00b6_8a3c;
pub const SYSTEM_FORMAT: u32 = 0x00b6_8a4c;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `rcuiSetNumber(n)`, `sceOpen(name, mode)`, `sceWrite(fd, buf, n)`,
/// `sceClose(fd)`, `rcbgChunkTag(i)`, `rcbgSaveChunkHeader_00938f40(size,
/// fd)`, `rcbgSceneInit(s)`, `rcbgSceneLoadAll_0093aec0(s)`,
/// `rcuiGuideSave_009397f0(g, fd)`, `rcbgSceneFini(s)`,
/// `SystemOnHost(command, out)`, `rcPrint(fmt, ...)` (cdecl).
pub const SET_NUMBER: u32 = 0x0093_7f70;
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const SCE_WRITE: u32 = 0x0062_fdc0;
pub const SCE_CLOSE: u32 = 0x0062_f610;
pub const CHUNK_TAG: u32 = 0x0092_1b90;
pub const SAVE_CHUNK_HEADER: u32 = 0x0093_8f40;
pub const SCENE_INIT: u32 = 0x0092_09e0;
pub const SCENE_LOAD_ALL: u32 = 0x0093_aec0;
pub const GUIDE_SAVE: u32 = 0x0093_97f0;
pub const SCENE_FINI: u32 = 0x0092_0870;
pub const SYSTEM_ON_HOST: u32 = 0x0084_ede0;
pub const RC_PRINT: u32 = 0x0092_7550;

pub mod bridge {
    use super::{
        CHMOD_FORMAT, CHUNK_TAG, GUIDE_SAVE, PATH_FORMAT, RC_PRINT, RCUI_WORK, SAVE_CHUNK_HEADER,
        SCE_CLOSE, SCE_OPEN, SCE_WRITE, SCENE_FINI, SCENE_INIT, SCENE_LOAD_ALL, SET_NUMBER,
        SPRINTF_SLOT, SYSTEM_FORMAT, SYSTEM_ON_HOST, YN_MAP_SYSTEM,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn sprintf(h: &mut impl Host, args: &[u32]) {
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        h.call(f, Conv::Cdecl, args);
    }

    fn strcpy(h: &mut impl Host, to: u32, from: u32) {
        let mut k = 0;
        loop {
            let c: u8 = h.read(from + k);
            h.write(to + k, c);
            if c == 0 {
                break;
            }
            k += 1;
        }
    }

    fn strlen(h: &impl Host, at: u32) -> u32 {
        let mut n = 0;
        while h.read::<u8>(at + n) != 0 {
            n += 1;
        }
        n
    }

    /// The chunk header: `rcbgChunkTag(i)`'s tag over its first word,
    /// written.
    fn chunk(h: &mut impl Host, header: u32, fd: u32, i: u32) {
        let tag_at = cdecl(h, CHUNK_TAG, &[i]);
        let tag: u32 = h.read(tag_at);
        h.write(header, tag);
        cdecl(h, SCE_WRITE, &[fd, header, 0x10]);
    }

    /// The map system's scene (`(*g_YnMapSystem) +8`), 0 without one.
    fn system_scene(h: &mut impl Host) -> u32 {
        let sys = YN_MAP_SYSTEM.get(h);
        if sys == 0 {
            return 0;
        }
        let first: u32 = h.read(sys);
        if first == 0 { 0 } else { h.read(first + 8) }
    }

    /// `rcbgGuideMapSaveSub(n, maps, number)` -> `rcbgSceneLoadAll`'s
    /// `eax`: the rcui work's directory (`+0x174`) and map (`+0x27c`) kept,
    /// `rcuiSetNumber(number)`; `map2d.bin` opened (no failure check),
    /// chunk 0, the header of `n`; per map byte `maps[i]` its two digits
    /// over the names' last two characters, a scene initialised, loaded,
    /// saved as a guide and finished; chunk 0xc, closed, `chmod 666` on
    /// the host; the names put back and the map system's scene
    /// initialised and loaded again.
    pub fn guide_map_save_sub(h: &mut impl Host, ebp: u32, n: u32, maps: u32, number: u32) -> u32 {
        let (dir_copy, map_copy) = (ebp - 0x30c, ebp - 0xc);
        let work = RCUI_WORK.get(h);
        strcpy(h, dir_copy, work + 0x174);
        strcpy(h, map_copy, work + 0x27c);
        cdecl(h, SET_NUMBER, &[number]);
        let path = ebp - 0x10c;
        let fmt = h.rebase(PATH_FORMAT);
        let dir = RCUI_WORK.get(h) + 0x174;
        sprintf(h, &[path, fmt, dir]);
        let fd = cdecl(h, SCE_OPEN, &[path, 0x602]);
        h.write(ebp - 0x534, fd);
        let header = ebp - 0x41c;
        for k in 0..4 {
            h.write(header + 4 * k, 0_u32);
        }
        chunk(h, header, fd, 0);
        cdecl(h, SAVE_CHUNK_HEADER, &[n, fd]);
        let scene = ebp - 0x52c;
        let mut i = 0_i32;
        while i < n.cast_signed() {
            let b = h.read::<u8>(maps.wrapping_add(i.cast_unsigned()));
            let (tens, ones) = (b / 10 + b'0', b % 10 + b'0');
            h.write(ebp - 0x530, u32::from(tens));
            for off in [0x174_u32, 0x27c] {
                let work = RCUI_WORK.get(h);
                let len = strlen(h, work + off);
                let end = work + off + len;
                h.write(end - 2, tens);
                let work = RCUI_WORK.get(h);
                h.write(work + off + len - 1, ones);
            }
            cdecl(h, SCENE_INIT, &[scene]);
            cdecl(h, SCENE_LOAD_ALL, &[scene]);
            let fd: u32 = h.read(ebp - 0x534);
            cdecl(h, GUIDE_SAVE, &[scene, fd]);
            cdecl(h, SCENE_FINI, &[scene]);
            i += 1;
        }
        let fd: u32 = h.read(ebp - 0x534);
        chunk(h, header, fd, 0xc);
        cdecl(h, SCE_CLOSE, &[fd]);
        let command = ebp - 0x20c;
        let fmt = h.rebase(CHMOD_FORMAT);
        sprintf(h, &[command, fmt, path + 5]);
        let r = cdecl(h, SYSTEM_ON_HOST, &[command, ebp - 0x40c]);
        let fmt = h.rebase(SYSTEM_FORMAT);
        cdecl(h, RC_PRINT, &[fmt, command, r]);
        let work = RCUI_WORK.get(h);
        strcpy(h, work + 0x174, dir_copy);
        let work = RCUI_WORK.get(h);
        strcpy(h, work + 0x27c, map_copy);
        let s = system_scene(h);
        cdecl(h, SCENE_INIT, &[s]);
        let s = system_scene(h);
        cdecl(h, SCENE_LOAD_ALL, &[s])
    }
}
