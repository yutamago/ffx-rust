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

    /// `g_CdLib_SectorTable` and the two words after it: the file
    /// tables `pc_fat_index_filename_set` reads.
    pub const FAT_TABLES: [u32; 3] = [0x0231_0c68, 0x0231_0c6c, 0x0231_0c70];
    /// `g_FFXHddMode` (`u8`).
    pub const HDD_MODE: Global<u8> = Global::new(0x0133_c913);
    /// `0x00a42ab0(path, &size)`: [`READ_WHOLE_FILE`]'s twin for the third
    /// table.
    pub const READ_WHOLE_FILE_2: u32 = 0x00a4_2ab0;
    /// The CD build's three paths (`host0:/ffx/proj/battle/jp/cddata/...`)
    /// and the HDD build's two (`host0:/ffx/proj/prog/cdidx/jp/...`), whose
    /// bytes `+0x1a`, `+0x1b` (the `jp`) take the country name.
    pub const FAT_CD_PATHS: [u32; 3] = [0x00c8_5d44, 0x00c8_5d70, 0x00c8_5d9c];
    pub const FAT_HDD_PATHS: [u32; 2] = [0x00c8_5dc8, 0x00c8_5df0];
    /// The country name buffer's size in `pc_fat_index_filename_set`'s
    /// frame.
    pub const FAT_COUNTRY_WORK: u32 = 0x10;

    /// `pc_fat_index_filename_set()` (`0x00a42fc0`, from `CdEnvInit`):
    /// `MsSetCountryName` into `work` ([`FAT_COUNTRY_WORK`] bytes); without
    /// [`HDD_MODE`] the three CD tables (`pc_ReadWholeFile` twice, then
    /// [`READ_WHOLE_FILE_2`]); with it the country's two letters into both
    /// HDD paths, their two tables and 0 for the third. Each read takes
    /// `(path, 0)`; returns the last read's block.
    pub fn pc_fat_index_filename_set(h: &mut impl Host, work: u32) -> u32 {
        fn read(h: &mut impl Host, f: u32, path: u32, table: u32) -> u32 {
            let (f, path, table) = (h.rebase(f), h.rebase(path), h.rebase(table));
            let block = h.call(f, &[path, 0]);
            h.write(table, block);
            block
        }
        let f = h.rebase(SET_COUNTRY_NAME);
        h.call(f, &[work]);
        if HDD_MODE.get(h) == 0 {
            read(h, READ_WHOLE_FILE, FAT_CD_PATHS[0], FAT_TABLES[0]);
            read(h, READ_WHOLE_FILE, FAT_CD_PATHS[1], FAT_TABLES[1]);
            return read(h, READ_WHOLE_FILE_2, FAT_CD_PATHS[2], FAT_TABLES[2]);
        }
        let country: [u8; 2] = h.read(work);
        for path in FAT_HDD_PATHS {
            let at = h.rebase(path) + 0x1a;
            h.write(at, country[0]);
            h.write(at + 1, country[1]);
        }
        read(h, READ_WHOLE_FILE, FAT_HDD_PATHS[0], FAT_TABLES[0]);
        let block = read(h, READ_WHOLE_FILE, FAT_HDD_PATHS[1], FAT_TABLES[1]);
        let at = h.rebase(FAT_TABLES[2]);
        h.write(at, 0u32);
        block
    }

    /// `sceCdSeek(sector)`, `sceCdRead()` (both `xor eax, eax; ret` on
    /// PC), `task_wait(n)` (a `dbgPrintf`), `0x0076d5a0()` (the HDD read
    /// step: 0 when done).
    pub const CD_SEEK: u32 = 0x00a4_47e0;
    pub const CD_READ: u32 = 0x00a4_47a0;
    pub const TASK_WAIT: u32 = 0x0077_a4e0;
    pub const HDD_READ_STEP: u32 = 0x0076_d5a0;

    /// The busy requests (`+0x31` set) among the 64.
    fn busy_count(h: &impl Mem) -> u32 {
        (0..READ_Q_CNT)
            .map(|i| u32::from(h.read::<u8>(record(h, i) + 0x31) != 0))
            .sum()
    }

    /// `0x0076e4b0(cd, request, n)` (`cd_read_thread_sector`, name ours;
    /// from the CD-mode read thread `0x0076e780` for a request whose
    /// `+0x30` is 1, a read by sector). Off the HDD mode, `cd + 0x74` = `n`
    /// and, when this is the only busy request: up to 16 `sceCdSeek(start
    /// sector +0x1c)` until one answers 1, a `task_wait(1)` after each
    /// miss; after a hit, while it is still the only busy one,
    /// `0x0076d5a0()` until it answers 0 (`task_wait(1)` between),
    /// otherwise `sceCdRead()` until it answers nonzero (`task_wait(1)`
    /// between). Always ends with `cd + 0x74` and `+0x5c` -1, `+0x70` 0,
    /// the request's busy byte and id (`+0x24`) 0. `eax` is left over
    /// (the entry's own on the HDD path): its caller ignores it.
    pub fn cd_read_thread_sector(h: &mut impl Host, cd: u32, request: u32, n: u32) {
        if HDD_MODE.get(h) == 0 {
            h.write(cd.wrapping_add(0x74), n);
            if busy_count(h) == 1 {
                let sector: u32 = h.read(request.wrapping_add(0x1c));
                let mut hit = false;
                for _ in 0..16 {
                    let f = h.rebase(CD_SEEK);
                    if h.call(f, &[sector]) == 1 {
                        hit = true;
                        break;
                    }
                    let f = h.rebase(TASK_WAIT);
                    h.call(f, &[1]);
                }
                if hit {
                    loop {
                        if busy_count(h) != 1 {
                            let f = h.rebase(CD_READ);
                            while h.call(f, &[]) == 0 {
                                let w = h.rebase(TASK_WAIT);
                                h.call(w, &[1]);
                            }
                            break;
                        }
                        let f = h.rebase(HDD_READ_STEP);
                        if h.call(f, &[]) == 0 {
                            break;
                        }
                        let f = h.rebase(TASK_WAIT);
                        h.call(f, &[1]);
                    }
                }
            }
        }
        h.write(cd.wrapping_add(0x74), u32::MAX);
        h.write(cd.wrapping_add(0x5c), u32::MAX);
        h.write(cd.wrapping_add(0x70), 0u32);
        h.write(request.wrapping_add(0x31), 0u8);
        h.write(request.wrapping_add(0x24), 0u32);
    }

    /// `hdd_install_find(archive, file)`, `hd_file_read_ex_th_main(path,
    /// buffer, a, b)`, `get_hdd_err()`, `TkRefreshHDDMode()`,
    /// `Sa_CdRead_task(sector, size, buffer, mode, archive)`,
    /// `FlushCache(0)`, the C runtime's `memcpy` (import thunk),
    /// `TkGetFFXCofingAdrs()`, `TkSignalSemaBGThread()`,
    /// `cd_decode_data(request)`, `sa_read_callback(request)`.
    pub const HDD_INSTALL_FIND: u32 = 0x0076_d600;
    pub const HD_FILE_READ: u32 = 0x0077_8550;
    pub const GET_HDD_ERR: u32 = 0x0077_7e80;
    pub const TK_REFRESH_HDD_MODE: u32 = 0x0088_e930;
    pub const SA_CD_READ_TASK: u32 = 0x0076_d200;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const MEMCPY_THUNK: u32 = 0x0094_9256;
    pub const TK_CONFIG: u32 = 0x0088_d5e0;
    pub const TK_SIGNAL_BG: u32 = 0x0088_8820;
    pub const DECODE_DATA: u32 = 0x0076_d440;
    pub const READ_CALLBACK: u32 = 0x0076_e640;
    /// The C runtime's `sprintf` (import slot).
    pub const SPRINTF_SLOT: Global<u32> = Global::new(0x00b0_c3dc);
    /// `"pfs0:/ex/%s"` and the path it builds (`0x01128ae8`).
    pub const FMT_EX_PATH: u32 = 0x00b5_1ee0;
    pub const EX_PATH: u32 = 0x0112_8ae8;
    /// The read mode block every `Sa_CdRead_task` here gets.
    pub const READ_MODE: u32 = 0x00c4_0e18;
    /// The clock block whose `+0x1c` times the read (`0x02310c10`).
    pub const CLOCK: Global<u32> = Global::new(0x0231_0c10);
    /// The sector buffer (`get_read_buffer`).
    pub const READ_BUFFER: Global<u32> = Global::new(0x0112_8ac8);
    /// The read's time and `0x01128ad0` copied after it.
    pub const READ_TIME: Global<u32> = Global::new(0x0112_8a88);
    pub const READ_LAST: Global<u32> = Global::new(0x0112_8a8c);
    pub const READ_LAST_SRC: Global<u32> = Global::new(0x0112_8ad0);
    /// The background decode handshake: busy byte, request.
    pub const BG_BUSY: Global<u8> = Global::new(0x0112_8aad);
    pub const BG_REQUEST: Global<u32> = Global::new(0x0231_0c2c);

    /// `0x0076e1c0(cd, request, n)` (`cd_read_thread_file`, name ours;
    /// from the CD-mode read thread `0x0076e780` for a request whose
    /// `+0x30` and `+0x33` are 0). Nothing while `cd + 0x70` is set; else
    /// `+0x70` 1, `+0x74` `n`, `+0x5c` 0 and the clock's `+0x1c` noted.
    /// An HDD-installed copy (`hdd_install_find(archive +0x28, file +8)`)
    /// is read from `"pfs0:/ex/<name +0x24>"` into the buffer (`+4`) with
    /// the entry's `+0x10` and `+0xc`; on an HDD error the HDD mode goes
    /// off (`TkRefreshHDDMode`) and the disc read follows. The disc read
    /// takes `size` (`+0x2c`) bytes from `sector` (`+0x1c`): whole
    /// sectors into the buffer, the tail through the sector buffer. A
    /// packed request (`+0x34`) shorter than `size` reads its `+0` + 5
    /// bytes, and when the first byte read is 0 (stored) drops the 5-byte
    /// header by copying; otherwise (and for every packed request after a
    /// full read) the data is decoded, by `cd_decode_data` or, with the
    /// config's `+0x10`, by the background thread (handshake on
    /// `0x01128aad`, `task_wait(1)` while busy). Then the time taken,
    /// `0x01128ad0` noted, the callback (`+0x20`) through
    /// `sa_read_callback`, and the request and `cd` released as in
    /// [`cd_read_thread_sector`].
    #[allow(clippy::too_many_lines)] // one read with three layouts
    #[allow(clippy::many_single_char_names)] // host, function, request word
    pub fn cd_read_thread_file<H: Host>(h: &mut H, cd: u32, request: u32, n: u32) {
        /// The shared tail, after the tail sector's read: the first
        /// `size & 0x7ff` bytes of the sector buffer after the whole
        /// sectors (the buffer, `+4`, read after the flush).
        fn tail(h: &mut impl Host, request: u32, size: u32) {
            let f = h.rebase(FLUSH_CACHE);
            h.call(f, &[0]);
            let rest = size & 0x7ff;
            let buffer: u32 = h.read(request.wrapping_add(4));
            let (f, from) = (h.rebase(MEMCPY_THUNK), READ_BUFFER.get(h));
            h.call(
                f,
                &[buffer.wrapping_sub(rest).wrapping_add(size), from, rest],
            );
        }
        if h.read::<u32>(cd.wrapping_add(0x70)) != 0 {
            return;
        }
        h.write(cd.wrapping_add(0x70), 1u32);
        h.write(cd.wrapping_add(0x74), n);
        h.write(cd.wrapping_add(0x5c), 0u32);
        let field = |h: &mut H, o: u32| -> u32 { h.read(request.wrapping_add(o)) };
        let start: u32 = h.read(CLOCK.get(h).wrapping_add(0x1c));
        let mut size = field(h, 0x2c);
        let f = h.rebase(HDD_INSTALL_FIND);
        let (archive, file) = (field(h, 0x28), field(h, 8));
        let entry = h.call(f, &[archive, file]);
        let mut done = false;
        if entry != 0 {
            let (path, fmt) = (h.rebase(EX_PATH), h.rebase(FMT_EX_PATH));
            let sprintf = SPRINTF_SLOT.get(h);
            h.call(sprintf, &[path, fmt, entry.wrapping_add(0x24)]);
            let buffer = field(h, 4);
            let (x, y) = (
                h.read(entry.wrapping_add(0x10)),
                h.read(entry.wrapping_add(0xc)),
            );
            let f = h.rebase(HD_FILE_READ);
            h.call(f, &[path, buffer, x, y]);
            let f = h.rebase(GET_HDD_ERR);
            if h.call(f, &[]) == 0 {
                done = true;
            } else {
                HDD_MODE.set(h, 0);
                let f = h.rebase(TK_REFRESH_HDD_MODE);
                h.call(f, &[]);
            }
        }
        if !done {
            // the request's words are read where the original reads them,
            // the sizes once
            let read = |h: &mut H, at: u32, len: u32, to: u32| {
                let (f, mode) = (h.rebase(SA_CD_READ_TASK), h.rebase(READ_MODE));
                let archive = field(h, 0x28);
                h.call(f, &[at, len, to, mode, archive]);
            };
            let flush = |h: &mut H| {
                let f = h.rebase(FLUSH_CACHE);
                h.call(f, &[0]);
            };
            let memcpy = |h: &mut H, to: u32, from: u32, len: u32| {
                let f = h.rebase(MEMCPY_THUNK);
                h.call(f, &[to, from, len]);
            };
            // the tail sector into the sector buffer, after `sectors`
            let read_tail = |h: &mut H, sectors: u32| {
                let (at, to) = (field(h, 0x1c).wrapping_add(sectors), READ_BUFFER.get(h));
                read(h, at, 0x800, to);
            };
            let mut decode = true;
            let packed = h.read::<u8>(request.wrapping_add(0x34)) != 0;
            let stored = field(h, 0);
            if packed && stored < size {
                size = stored.wrapping_add(5);
                let sectors = (size.cast_signed() / 0x800).cast_unsigned();
                if sectors != 0 && size.cast_signed() > 0 {
                    let (at, to) = (field(h, 0x1c), field(h, 4));
                    read(h, at, size, to);
                    flush(h);
                    let buffer = field(h, 4);
                    if h.read::<u8>(buffer) == 0 {
                        let whole = sectors << 11;
                        memcpy(h, buffer, buffer.wrapping_add(5), whole);
                        read_tail(h, sectors);
                        flush(h);
                        let (to, from) = (field(h, 4).wrapping_add(whole), READ_BUFFER.get(h));
                        memcpy(h, to, from, size.wrapping_sub(whole).wrapping_sub(5));
                        decode = false;
                    } else if size & 0x7ff != 0 {
                        read_tail(h, sectors);
                        tail(h, request, size);
                    } else {
                        flush(h);
                    }
                } else {
                    read_tail(h, 0);
                    flush(h);
                    let from = READ_BUFFER.get(h);
                    if h.read::<u8>(from) == 0 {
                        let to = field(h, 4);
                        memcpy(h, to, from.wrapping_add(5), size.wrapping_sub(5));
                        decode = false;
                    } else {
                        tail(h, request, size);
                    }
                }
            } else {
                let sectors = (size.cast_signed() / 0x800).cast_unsigned();
                if sectors == 0 {
                    read_tail(h, 0);
                    tail(h, request, size);
                } else {
                    let (at, len, to) = (field(h, 0x1c), field(h, 0x2c), field(h, 4));
                    read(h, at, len, to);
                    if size & 0x7ff != 0 {
                        read_tail(h, sectors);
                        tail(h, request, size);
                    } else {
                        flush(h);
                    }
                }
            }
            if decode && h.read::<u8>(request.wrapping_add(0x34)) != 0 {
                let f = h.rebase(TK_CONFIG);
                let config = h.call(f, &[]);
                if h.read::<u8>(config.wrapping_add(0x10)) == 0 {
                    let f = h.rebase(DECODE_DATA);
                    h.call(f, &[request]);
                } else {
                    let wait = |h: &mut H| {
                        while BG_BUSY.get(h) != 0 {
                            let f = h.rebase(TASK_WAIT);
                            h.call(f, &[1]);
                        }
                    };
                    wait(h);
                    BG_BUSY.set(h, 1);
                    BG_REQUEST.set(h, request);
                    let f = h.rebase(TK_SIGNAL_BG);
                    h.call(f, &[]);
                    wait(h);
                }
            }
        }
        let now: u32 = h.read(CLOCK.get(h).wrapping_add(0x1c));
        READ_TIME.set(h, now.wrapping_sub(start));
        let last = READ_LAST_SRC.get(h);
        READ_LAST.set(h, last);
        if field(h, 0x20) != 0 {
            let f = h.rebase(READ_CALLBACK);
            h.call(f, &[request]);
        }
        h.write(cd.wrapping_add(0x74), u32::MAX);
        h.write(cd.wrapping_add(0x5c), u32::MAX);
        h.write(cd.wrapping_add(0x70), 0u32);
        h.write(request.wrapping_add(0x31), 0u8);
        h.write(request.wrapping_add(0x24), 0u32);
    }
}
