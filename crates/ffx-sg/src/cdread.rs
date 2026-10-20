//! The read queue of the PS2 CD-library emulation (`libmscd.c`): the 64
//! request records at `g_CdChannels` (`0x01127c80`, 0x38 bytes each) and
//! the functions in `g_CdLib`'s table that queue and cancel them.
//!
//! A record: size at `+0` (and again at `+0x2c`), buffer at `+4`, PS2
//! file number at `+8` (-1 for a read by sector), the caller's four words
//! at `+0xc..+0x1c`, start sector at `+0x1c`, callback at `+0x20`, request
//! id at `+0x24`, `+0x28` -1 for a read by sector, the busy byte at
//! `+0x31`, the cancel byte at `+0x33`, a word at `+0x34`. The ids run
//! 1..=0xffff from the counter at `0x01128a80`.

/// The requests in [`bridge::CHANNELS`] (`READ_Q_CNT`).
pub const READ_Q_CNT: u32 = 64;

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Host, Mem};
    pub use ffx_system::cd::bridge::{CHANNEL_KEEP_NAMES, CHANNEL_NAMES, CHANNELS};

    use super::READ_Q_CNT;

    /// The last request id handed out (`0x01128a80`).
    pub const READ_ID: Global<i32> = Global::new(0x0112_8a80);
    /// `movie_err(expr, file, line)` (glue: the assertion report).
    pub const MOVIE_ERR: u32 = 0x0076_efa0;
    /// `"q_cnt != READ_Q_CNT"`.
    pub const STR_Q_CNT: u32 = 0x00b5_1f04;
    /// `"..\\program\\lib\\src\\libmscd.c"`.
    pub const STR_LIBMSCD: u32 = 0x00b5_1d30;
    /// `"DVD FILE"`, the name a sector read gets.
    pub const STR_DVD_FILE: u32 = 0x00b5_1f24;

    fn record(mem: &impl Mem, i: u32) -> u32 {
        CHANNELS.ptr(mem).addr.wrapping_add(i.wrapping_mul(0x38))
    }

    /// `cd_read_q_clear(id)` (`0x0076e130`, `g_CdLib + 0x1c`): sets the
    /// cancel byte (`+0x33`) of every busy request (`+0x31`) whose id
    /// (`+0x24`) is `id`, or of every busy request when `id < 1`.
    pub fn cd_read_q_clear(mem: &mut impl Mem, id: i32) {
        for i in 0..READ_Q_CNT {
            let r = record(mem, i);
            let busy = mem.read::<u8>(r + 0x31) != 0;
            if busy && (id < 1 || mem.read::<i32>(r + 0x24) == id) {
                mem.write(r + 0x33, 1u8);
            }
        }
    }

    /// `CDfileReadSector_CD(sector, buffer, size, callback, opts)`
    /// (`0x0076bdd0`, `g_CdLib + 0x78`): queues a read of `size` bytes from
    /// `sector` into `buffer` in the first free request and returns its new
    /// id. `opts` (or 0) supplies the words at `+0xc..+0x1c`. The name
    /// becomes `"DVD FILE"` unless [`CHANNEL_KEEP_NAMES`]. With all 64 busy
    /// the assertion goes to `movie_err` and request 64 is written anyway
    /// (over [`READ_ID`] and what follows).
    pub fn cd_file_read_sector(
        h: &mut impl Host,
        sector: u32,
        buffer: u32,
        size: u32,
        callback: u32,
        opts: u32,
    ) -> i32 {
        let base = CHANNELS.ptr(h).addr;
        let i = (0..READ_Q_CNT)
            .find(|&i| h.read::<u8>(base + 0x38 * i + 0x31) == 0)
            .unwrap_or(READ_Q_CNT);
        if i == READ_Q_CNT {
            let args = [h.rebase(STR_Q_CNT), h.rebase(STR_LIBMSCD), 0x6a7];
            let at = h.rebase(MOVIE_ERR);
            h.call(at, &args);
        }
        let r = record(h, i);
        h.write(r + 0x31, 1u8);
        h.write(r + 4, buffer);
        h.write(r + 8, u32::MAX);
        h.write(r + 0x28, u32::MAX);
        for k in 0..4 {
            let v = if opts == 0 {
                0
            } else {
                h.read::<u32>(opts.wrapping_add(0xc + 4 * k))
            };
            h.write(r + 0xc + 4 * k, v);
        }
        let mut id = READ_ID.get(h).wrapping_add(1);
        h.write(r + 0x20, callback);
        READ_ID.set(h, id);
        if id > 0xffff {
            id = 1;
            READ_ID.set(h, id);
        }
        let keep = CHANNEL_KEEP_NAMES.get(h);
        h.write(r, size);
        h.write(r + 0x2c, size);
        h.write(r + 0x24, id);
        h.write(r + 0x1c, sector);
        if keep == 0 {
            let name = CHANNEL_NAMES.ptr(h).addr.wrapping_add(i << 7);
            let src = h.rebase(STR_DVD_FILE);
            for k in 0..0x80 {
                let c = h.read::<u8>(src + k);
                h.write(name + k, c);
                if c == 0 {
                    break;
                }
            }
        }
        h.write(r + 0x34, 0u16);
        id
    }

    /// `pc_ReadWholeFile(path, &size)` (`0x00a42a00`, platform: the file
    /// read whole into a new block).
    pub const READ_WHOLE_FILE: u32 = 0x00a4_2a00;
    /// `MsSetCountryName(out)` (`0x00782c00`).
    pub const SET_COUNTRY_NAME: u32 = 0x0078_2c00;
    /// The paths: `".../sizetbl.vita.bin"`, `".../modulesize.bin"`,
    /// `".../sizetbl.bin"` (under `host0:/ffx/proj/prog/cdidx/jp/`).
    pub const SIZETBL_EX_PATH: u32 = 0x00c8_5e74;
    pub const MODULE_SIZE_PATH: u32 = 0x00c8_5e44;
    pub const SIZETBL_PATH: u32 = 0x00c8_5e18;
    /// The 0x10-byte buffer [`pc_sizetbl_set`] hands `MsSetCountryName`.
    pub const COUNTRY_WORK: u32 = 0x10;

    /// `pc_sizetbl_set_Ex()` (`0x00a43100`, from `CdEnvInit`): reads
    /// `sizetbl.vita.bin` with its byte size into `g_Cd_ExSizeCount`, then
    /// makes that a count of words (`/ 4`, toward zero); returns the block.
    pub fn pc_sizetbl_set_ex(h: &mut impl Host) -> u32 {
        let count = ffx_system::cd::bridge::EX_SIZE_COUNT;
        let (f, path, at) = (
            h.rebase(READ_WHOLE_FILE),
            h.rebase(SIZETBL_EX_PATH),
            h.rebase(0x0112_8a94),
        );
        let data = h.call(f, &[path, at]);
        let bytes = count.get(h);
        count.set(h, bytes / 4);
        data
    }

    /// `pc_sizetbl_set()` (`0x00a430c0`) and `pc_module_sizetbl_set()`
    /// (`0x00a43080`), from `CdEnvInit`, the same code but the path:
    /// `MsSetCountryName` into a buffer (`work`, [`COUNTRY_WORK`] bytes,
    /// not used after), then `pc_ReadWholeFile(path, 0)` returned.
    pub fn pc_sizetbl_set(h: &mut impl Host, path: u32, work: u32) -> u32 {
        let f = h.rebase(SET_COUNTRY_NAME);
        h.call(f, &[work]);
        let (f, path) = (h.rebase(READ_WHOLE_FILE), h.rebase(path));
        h.call(f, &[path, 0])
    }
}
