//! The camera tracks of the movies played over a 3D scene
//! (`g_CameraData`): `LoadCameraData` reads one from
//! `/FFX_Data/GameData/PS3Data/Video/<movie>.dat` when
//! `PhyreScene_fieldMapLoad` enters one of the four maps that have one;
//! `movie_have_camera` and the frame code at `0x0076ee38` read it.

/// The size of one camera record (one movie frame) in the file and in
/// `g_CameraData`.
pub const RECORD: u32 = 0xa8;

/// The block `LoadCameraData` allocates for movie `movie` (bytes) and the
/// word count it stores in `g_CameraDataWords` (bytes / 4); `None` for a
/// movie without a fixed block (the buffer stays null).
#[must_use]
pub fn block(movie: u32) -> Option<(u32, u32)> {
    match movie {
        0x2e => Some((0x3_8658, 0xe196)),
        0x48 => Some((0xfff0, 0x3ffc)),
        0x20 => Some((0x3_ba30, 0xee8c)),
        0x46 => Some((0x2_9388, 0xa4e2)),
        _ => None,
    }
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{RECORD, block};

    /// `g_CameraData`: the loaded camera records (`operator new`), null
    /// when none.
    pub const CAMERA_DATA: Global<u32> = Global::new(0x00ce_82ac);
    /// `g_CameraDataWords`: the block's size in words (records = words /
    /// 42 at `0x0076ee38`).
    pub const CAMERA_DATA_WORDS: Global<u32> = Global::new(0x00c3_8e18);
    /// `operator_delete_00630540(p)` and `operator_new_006304c0(size)`.
    pub const OPERATOR_DELETE: u32 = 0x0063_0540;
    pub const OPERATOR_NEW: u32 = 0x0063_04c0;
    /// `fiosUnifyFilename(src, out, size)`, `fiosOpen(path, mode)`,
    /// `fiosRead(file, buffer, size)`, `fiosClose(file)`.
    pub const FIOS_UNIFY: u32 = 0x0067_9820;
    pub const FIOS_OPEN: u32 = 0x0067_9730;
    pub const FIOS_READ: u32 = 0x0067_9800;
    pub const FIOS_CLOSE: u32 = 0x0067_9510;
    /// `dbgPrintf(fmt, ...)`.
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// The C runtime's `sprintf` (import slot).
    pub const SPRINTF_SLOT: Global<u32> = Global::new(0x00b0_c3dc);
    /// `"/FFX_Data/GameData/PS3Data/Video/"`, `"%s%d%s"`, `".dat"` and
    /// `"Virtuos Error: Cannot open camera.dat file!!!"`.
    pub const VIDEO_DIR: u32 = 0x00b4_bf18;
    pub const FMT_PATH: u32 = 0x00b4_1f68;
    pub const DAT_SUFFIX: u32 = 0x00b4_bf3c;
    pub const MSG_CANNOT_OPEN: u32 = 0x00b4_bf44;

    /// Bytes of work [`load_camera_data`] needs, laid out as the
    /// original's frame from `ebp - 0x284`: the record read buffer
    /// (0x100), the unified directory (0x100), the path (0x80).
    pub const WORK: u32 = 0x280;
    const WORK_UNIFIED: u32 = 0x100;
    const WORK_PATH: u32 = 0x200;

    /// `LoadCameraData(movie)` (`0x006f6dc0`, Fahrenheit's name; from
    /// `PhyreScene_fieldMapLoad`): the old block freed (`operator
    /// delete`) and `g_CameraData` cleared; for the four movies of
    /// [`block`] a new zeroed block (`operator new`, then `memset`) and
    /// `g_CameraDataWords`. The path `<unified video dir><movie>.dat`
    /// (`fiosUnifyFilename` into 0x100 bytes, `sprintf "%s%d%s"`) opened
    /// with mode 1; if that fails a `dbgPrintf` and nothing else, else
    /// [`RECORD`]-byte reads copied one after another to `g_CameraData`
    /// (re-read for each) while a read returns the full record, and the
    /// file closed. Any other movie leaves the block null, so a file that
    /// opens would be copied to address 0 (the original faults too).
    /// `eax` is the stack cookie check's (callers read none).
    pub fn load_camera_data<M, C>(mem: &mut M, movie: u32, work: u32, call: &mut C)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let old = CAMERA_DATA.get(mem);
        if old != 0 {
            let f = mem.rebase(OPERATOR_DELETE);
            call(mem, f, &[old]);
        }
        CAMERA_DATA.set(mem, 0);
        if let Some((size, words)) = block(movie) {
            let f = mem.rebase(OPERATOR_NEW);
            let p = call(mem, f, &[size]);
            CAMERA_DATA.set(mem, p);
            mem.write_bytes(p, &vec![0u8; size as usize]);
            CAMERA_DATA_WORDS.set(mem, words);
        }
        let (buf, unified, path) = (work, work + WORK_UNIFIED, work + WORK_PATH);
        let (f, dir) = (mem.rebase(FIOS_UNIFY), mem.rebase(VIDEO_DIR));
        call(mem, f, &[dir, unified, 0x100]);
        let f = SPRINTF_SLOT.get(mem);
        let (fmt, dat) = (mem.rebase(FMT_PATH), mem.rebase(DAT_SUFFIX));
        call(mem, f, &[path, fmt, unified, movie, dat]);
        let f = mem.rebase(FIOS_OPEN);
        let file = call(mem, f, &[path, 1]);
        if file == 0 {
            let (f, msg) = (mem.rebase(DBG_PRINTF), mem.rebase(MSG_CANNOT_OPEN));
            call(mem, f, &[msg]);
            return;
        }
        let mut at = 0u32;
        loop {
            let f = mem.rebase(FIOS_READ);
            if call(mem, f, &[file, buf, RECORD]) != RECORD {
                break;
            }
            let mut record = [0u8; RECORD as usize];
            mem.read_bytes(buf, &mut record);
            let to = CAMERA_DATA.get(mem).wrapping_add(at);
            mem.write_bytes(to, &record);
            at = at.wrapping_add(RECORD);
        }
        let f = mem.rebase(FIOS_CLOSE);
        call(mem, f, &[file]);
    }
}
