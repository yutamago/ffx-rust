//! M5 finish, WP5 part 7 (wp5h): the developers' save loader, which the
//! harness's `devsave` command runs (docs/harness.md, "Developer saves"):
//! `TkDebugLoadViewerSave` (`0x008b55e0`) and the file reader it calls,
//! `debugLoadForViewer` (`0x00649040`). Both are reached in the game only
//! from the PC tweak panel's "Game Section" entry
//! (`BasicDebugTweakUtil_handleInput`, `0x006b7767`); `ffx-rust` needs
//! them because the harness loads developer saves through them.
//!
//! `debugLoadForViewer` opens its file with the C runtime's `fopen`,
//! which cannot see into `FFX_Data.vbf`, so in the shipped game it always
//! fails; the harness replaces it with an archive reader (`ffx-hook`'s and
//! `ffx-host`'s `devsave`), and outside the harness its port runs.

/// The originals over game memory.
#[allow(clippy::cast_possible_truncation)] // C strings in 32-bit memory
pub mod bridge {
    use ffx_bridge::Mem;

    /// `TkDebugLoadViewerSave(n)`.
    pub const TK_DEBUG_LOAD_VIEWER_SAVE: u32 = 0x008b_55e0;
    /// `debugLoadForViewer(n, buffer, size)`.
    pub const DEBUG_LOAD_FOR_VIEWER: u32 = 0x0064_9040;
    /// `MsGetSaveEventAddress()`: `&g_SaveData`.
    pub const MS_GET_SAVE_EVENT_ADDRESS: u32 = 0x0078_5240;
    /// `AtelSetEventJump(map, entrance)`.
    pub const ATEL_SET_EVENT_JUMP: u32 = 0x0086_fec0;
    /// `crt_memcpy_thunk` and `crt_memset_thunk` (`jmp [import]`).
    pub const MEMCPY: u32 = 0x0094_9256;
    pub const MEMSET: u32 = 0x0094_964c;
    /// `FmodFile_getDataDirectory_00679a10(key, buf, n)`.
    pub const GET_DATA_DIRECTORY: u32 = 0x0067_9a10;
    /// `crossAssertOutput(text)` (a bare `ret` in this build).
    pub const CROSS_ASSERT_OUTPUT: u32 = 0x006e_f310;
    /// `sprintf`, `fopen`, `fread`, `fclose` (`MSVCR110` import slots,
    /// cdecl).
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    pub const FOPEN_SLOT: u32 = 0x00b0_c3f0;
    pub const FREAD_SLOT: u32 = 0x00b0_c3f4;
    pub const FCLOSE_SLOT: u32 = 0x00b0_c3ec;
    /// `"%d"`, `"rb"`, `"/FFX_Data/GameData/PS3Data/savesforviewer/"`
    /// and `"SKS_DEBUG: File \"%s\" could not be opened\n"`.
    pub const FMT_INT: u32 = 0x00b0_f604;
    pub const MODE_RB: u32 = 0x00b3_f69c;
    pub const FOLDER: u32 = 0x00b4_bab8;
    pub const MSG_CANNOT_OPEN: u32 = 0x00b4_1e94;

    /// `g_SaveFileBuffer` (0x6900 bytes, a PC save file) and its game
    /// block (`+0x40`, 0x68c0 bytes: what `g_SaveData` holds).
    pub const SAVE_FILE_BUFFER: u32 = 0x0159_7f70;
    pub const SAVE_FILE_SIZE: u32 = 0x6900;
    pub const SAVE_FILE_BLOCK: u32 = 0x0159_7fb0;
    pub const SAVE_BLOCK_SIZE: u32 = 0x68c0;
    /// The last developer save loaded (`TkDebugLoadViewerSave` skips a
    /// repeat).
    pub const VIEWER_SAVE_LAST: u32 = 0x00c5_a038;
    /// `g_SaveData + 0xb8`: the entrance (`u16`) and, at `+0xba`, the map
    /// a loaded save starts on.
    pub const SAVE_JUMP: u32 = 0x0112_cb48;
    /// `g_SaveScenario` (`g_SaveData + 0xbec`, `u16`).
    pub const SAVE_SCENARIO: u32 = 0x0112_d67c;
    /// The save `TkDebugLoadViewerSave` sends to map `0xf6` with scenario
    /// `0x7c15` whatever the file says.
    pub const SPECIAL_SAVE: u32 = 0x97;

    /// Bytes of `debugLoadForViewer`'s frame below its `ebp` (the stack
    /// cookie at `ebp - 4` included): the buffer argument's copy at
    /// `ebp - 0x40c`, the number at `- 0x408`, the message at `- 0x404`
    /// (0x200), the folder at `- 0x204` (0x100), the path at `- 0x104`
    /// (0x100).
    pub const FRAME: u32 = 0x40c;
    const BUFFER_COPY: u32 = 0x40c;
    const NUMBER: u32 = 0x408;
    const MESSAGE: u32 = 0x404;
    const DIRECTORY: u32 = 0x204;
    const PATH: u32 = 0x104;
    const PATH_SIZE: u32 = 0x100;

    /// `TkDebugLoadViewerSave(n)` (`0x008b55e0`, cdecl): nothing when `n`
    /// is the last save loaded; else `debugLoadForViewer(n,
    /// g_SaveFileBuffer, 0x6900)`, and when that returns 0 the file's game
    /// block is copied over `g_SaveData` (`memcpy(MsGetSaveEventAddress(),
    /// buffer + 0x40, 0x68c0)`), `n` noted, and `AtelSetEventJump` called
    /// with the map and entrance at `g_SaveData + 0xba` / `+ 0xb8`; save
    /// `0x97` instead gets map `0xf6`, entrance 0 and scenario `0x7c15`
    /// written first. `eax`: `AtelSetEventJump`'s, `debugLoadForViewer`'s
    /// when it fails; on a repeat the caller's `eax` (its only caller
    /// reads none): 0 here.
    pub fn tk_debug_load_viewer_save<M, C>(mem: &mut M, n: u32, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let last = mem.rebase(VIEWER_SAVE_LAST);
        if mem.read::<u32>(last) == n {
            return 0;
        }
        let (f, buffer) = (
            mem.rebase(DEBUG_LOAD_FOR_VIEWER),
            mem.rebase(SAVE_FILE_BUFFER),
        );
        let r = call(mem, f, &[n, buffer, SAVE_FILE_SIZE]);
        if r != 0 {
            return r;
        }
        let f = mem.rebase(MS_GET_SAVE_EVENT_ADDRESS);
        let save = call(mem, f, &[]);
        let (f, block) = (mem.rebase(MEMCPY), mem.rebase(SAVE_FILE_BLOCK));
        call(mem, f, &[save, block, SAVE_BLOCK_SIZE]);
        mem.write(last, n);
        let jump = mem.rebase(SAVE_JUMP);
        let (map, entrance) = if n == SPECIAL_SAVE {
            mem.write(jump, 0x00f6_0000u32);
            mem.write(mem.rebase(SAVE_SCENARIO), 0x7c15u16);
            (0xf6, 0)
        } else {
            (
                u32::from(mem.read::<u16>(jump + 2)),
                u32::from(mem.read::<u16>(jump)),
            )
        };
        let f = mem.rebase(ATEL_SET_EVENT_JUMP);
        call(mem, f, &[map, entrance])
    }

    /// The C string at `at` with its NUL.
    fn c_string(mem: &impl Mem, at: u32) -> Vec<u8> {
        let mut s = Vec::new();
        loop {
            let b: u8 = mem.read(at.wrapping_add(s.len() as u32));
            s.push(b);
            if b == 0 {
                return s;
            }
        }
    }

    /// `strcat(to, from)` as the original's inlined copy (the end of `to`
    /// found first, then `from` with its NUL copied there).
    fn append(mem: &mut impl Mem, to: u32, from: u32) {
        let s = c_string(mem, from);
        let end = to.wrapping_add(c_string(mem, to).len() as u32 - 1);
        mem.write_bytes(end, &s);
    }

    /// `debugLoadForViewer(n, buffer, size)` (`0x00649040`, cdecl; its
    /// frame below `ebp`, [`FRAME`] bytes): the path (0x100 bytes,
    /// `memset` to 0) becomes the stream folder of
    /// `"/FFX_Data/GameData/PS3Data/savesforviewer/"`
    /// (`FmodFile_getDataDirectory_00679a10`) followed by `n` in decimal
    /// (`sprintf "%d"`), and is opened with `fopen(path, "rb")`: then
    /// `fread(buffer, 1, size, file)`, `fclose` and 0; when it doesn't
    /// open, the message `sprintf`'d and handed to `crossAssertOutput`,
    /// and -1.
    pub fn debug_load_for_viewer<M, C>(
        mem: &mut M,
        ebp: u32,
        n: u32,
        buffer: u32,
        size: u32,
        call: &mut C,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let slot = |mem: &M, s: u32| mem.read::<u32>(mem.rebase(s));
        let (number, message) = (ebp.wrapping_sub(NUMBER), ebp.wrapping_sub(MESSAGE));
        let (directory, path) = (ebp.wrapping_sub(DIRECTORY), ebp.wrapping_sub(PATH));
        mem.write(ebp.wrapping_sub(BUFFER_COPY), buffer);
        let f = mem.rebase(MEMSET);
        call(mem, f, &[path, 0, PATH_SIZE]);
        let (f, fmt) = (slot(mem, SPRINTF_SLOT), mem.rebase(FMT_INT));
        call(mem, f, &[number, fmt, n]);
        let (f, folder) = (mem.rebase(GET_DATA_DIRECTORY), mem.rebase(FOLDER));
        call(mem, f, &[folder, directory, PATH_SIZE]);
        append(mem, path, directory);
        append(mem, path, number);
        let (f, rb) = (slot(mem, FOPEN_SLOT), mem.rebase(MODE_RB));
        let file = call(mem, f, &[path, rb]);
        if file == 0 {
            let (f, msg) = (slot(mem, SPRINTF_SLOT), mem.rebase(MSG_CANNOT_OPEN));
            call(mem, f, &[message, msg, path]);
            let f = mem.rebase(CROSS_ASSERT_OUTPUT);
            call(mem, f, &[message]);
            return u32::MAX;
        }
        let f = slot(mem, FREAD_SLOT);
        call(mem, f, &[buffer, 1, size, file]);
        let f = slot(mem, FCLOSE_SLOT);
        call(mem, f, &[file]);
        0
    }
}
