//! M5 `main`, round 3: the PS2 CD library emulation's read queue
//! (`libmscd.c`, slot 0 of `g_CdLib` at `0x02310c40`).
//!
//! `g_CdChannels` (`0x01127c80`): 64 requests of 0x38 bytes:
//! `+0x00` size, `+0x04` buffer, `+0x08` file number, `+0x0c..+0x18` the
//! caller's options (`+0x0c..+0x18` of its block), `+0x1c` sector, `+0x20`
//! callback, `+0x24` id, `+0x28` module, `+0x2c` size in bytes, `+0x30`
//! a byte cleared, `+0x31` busy, `+0x32` queued, `+0x34` "size from the
//! table", `+0x35` cleared. `g_CdChannelNames` (`0x022fb6c0`): a 0x80-byte
//! name per request.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)]
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    pub const CHANNELS: u32 = 0x0112_7c80;
    pub const CHANNEL: u32 = 0x38;
    pub const NAMES: u32 = 0x022f_b6c0;
    /// The last request id (1..=0xffff).
    pub const READ_ID: u32 = 0x0112_8a80;
    /// The current module (`CdSetModule`).
    pub const MODULE: u32 = 0x0231_0c64;
    /// 0 the PC files, 1 the DVD image, else no lookup at all.
    pub const MODE: u32 = 0x0231_0c58;
    /// Set: the HDD build's files (sector and size from the tables).
    pub const HDD: u32 = 0x0133_c913;
    pub const GRAPHIC_MAP_ID: u32 = 0x0133_0844;
    /// The save's u16 that picks the c101 -> c307 swap, and its value.
    pub const SWAP_KEY: u32 = 0x0112_ca90;
    pub const SWAP_VALUE: u32 = 0x00cc_c7c8;
    /// Pointers to `host0:/ffx/master/jppc/chr/pc/c101/mdl/c101.chr`
    /// and the `c307` path that replaces it.
    pub const SWAP_FROM: u32 = 0x00b4_4e5c;
    pub const SWAP_TO: u32 = 0x00b4_4e60;
    /// `"DVD FILE"`, `"HDD FILE"`.
    pub const DVD_FILE: u32 = 0x00b5_1f24;
    pub const HDD_FILE: u32 = 0x00b5_1f18;
    /// `movie_err`'s message, file and line for a full queue.
    pub const Q_CNT: u32 = 0x00b5_1f04;
    pub const LIBMSCD_C: u32 = 0x00b5_1d30;
    /// `dbgPrintf` formats and `yiAssert`'s strings of the movie check.
    pub const FMT_MODULE: u32 = 0x00b5_1f30;
    pub const FMT_NUM: u32 = 0x00b5_1f40;
    pub const FMT_NAME: u32 = 0x00b5_1f4c;
    pub const ASSERT_FMT: u32 = 0x00b5_1f58;
    pub const ASSERT_TEXT: u32 = 0x00b5_1f74;

    pub const MOVIE_ERR: u32 = 0x0076_efa0;
    pub const CD_FILE_SIZE_CD: u32 = 0x0076_c320;
    pub const CD_FILE_SIZE_PC: u32 = 0x00a4_28a0;
    pub const CD_FILE_NAME_PC: u32 = 0x00a4_2830;
    pub const MODULE_FILE_SECTOR_REL: u32 = 0x0076_c250;
    pub const FILE_SIZE_BYTES: u32 = 0x0076_c440;
    pub const IS_ETERNAL_CALM_SCENE: u32 = 0x0067_ab50;
    pub const IS_ETERNAL_CALM_MGRP: u32 = 0x0067_ab30;
    pub const ETERNAL_CALM_MGRP_NAME: u32 = 0x0067_aa50;
    pub const ETERNAL_CALM_SCENE_NAME: u32 = 0x0067_aac0;
    pub const TK_SYNC_V: u32 = 0x0088_7f00;
    pub const MEMSET: u32 = 0x0094_964c;
    pub const MEMCPY: u32 = 0x0094_9256;
    pub const MOVIE_RUN_CHECK: u32 = 0x0076_f060;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const YI_ASSERT: u32 = 0x0090_ae50;
    pub const CD_SIZE_FROM_TABLE: u32 = 0x0076_dc20;
    pub const REGIST_ASYNC_TASK: u32 = 0x0064_2950;
    /// The async task that reads a queued request (`0x00a43740`).
    pub const READ_TASK: u32 = 0x00a4_3740;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn strlen(mem: &impl Mem, p: u32) -> u32 {
        let mut n = 0u32;
        while mem.read::<u8>(at(p, n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The first request with neither `+0x31` nor `+0x32` set (64 when
    /// none is free).
    fn free_slot(mem: &impl Mem) -> u32 {
        let base = mem.rebase(CHANNELS);
        (0..64)
            .find(|&i| {
                let e = at(base, i * CHANNEL);
                mem.read::<u8>(at(e, 0x31)) == 0 && mem.read::<u8>(at(e, 0x32)) == 0
            })
            .unwrap_or(64)
    }

    /// `Cd_QueueRead(file, buffer, callback, opts)` (`0x0076b970`,
    /// cdecl): queues a read of file `file` (of the current module) into
    /// `buffer` in the first free request (`movie_err` when all 64 are
    /// busy, then request 64 past the table, as the original). By
    /// [`MODE`]: the PC files (size from `CDfileSize_PC`, or the DVD's for
    /// the Eternal Calm scene; the HDD build takes sector and byte size
    /// from the tables; the name from `CDfileName_PC`, else the Eternal
    /// Calm movie group's or scene's name for files 18k+3 / 18k; one
    /// `TkSyncV(2)`), the DVD image (size, sector and byte size; named
    /// "DVD FILE"), or no lookup at all. With the save's swap key set, a
    /// name that is a prefix of the c101 model path becomes the c307
    /// path. The name goes to the request's name slot; then the movie
    /// check's report, the "size from table" byte, and the read task
    /// registered. Returns the request's id.
    ///
    /// The original uses its `buffer` argument slot as scratch (the
    /// quotient `file / 18`, then a string cursor); its callers call
    /// through `g_CdLib` and pop the words after.
    pub fn queue_read(h: &mut impl Host, file: u32, buffer: u32, callback: u32, opts: u32) -> u32 {
        let i = free_slot(h);
        if i == 0x40 {
            let (msg, src) = (h.rebase(Q_CNT), h.rebase(LIBMSCD_C));
            cdecl(h, MOVIE_ERR, &[msg, src, 0x5b1]);
        }
        let e = at(h.rebase(CHANNELS), i.wrapping_mul(CHANNEL));
        h.write(at(e, 4), buffer);
        h.write(at(e, 8), file);
        let module: u32 = h.read(h.rebase(MODULE));
        h.write(at(e, 0x31), 0x101u16);
        h.write(at(e, 0x28), module);
        let words: [u32; 4] = if opts == 0 {
            [0; 4]
        } else {
            h.read(at(opts, 0xc))
        };
        h.write(at(e, 0xc), words);
        h.write(at(e, 0x20), callback);
        let id_at = h.rebase(READ_ID);
        let mut id = h.read::<u32>(id_at).wrapping_add(1);
        h.write(at(e, 0x30), 0u8);
        h.write(id_at, id);
        if id.cast_signed() > 0xffff {
            id = 1;
            h.write(id_at, id);
        }
        h.write(at(e, 0x24), id);
        let mode: u32 = h.read(h.rebase(MODE));
        let mut name = 0u32;
        if mode == 1 {
            let size = cdecl(h, CD_FILE_SIZE_CD, &[file]);
            h.write(e, size);
            let sector = cdecl(h, MODULE_FILE_SECTOR_REL, &[file]);
            h.write(at(e, 0x1c), sector);
            let bytes = cdecl(h, FILE_SIZE_BYTES, &[file]);
            h.write(at(e, 0x2c), bytes);
            name = h.rebase(DVD_FILE);
        } else if mode == 0 {
            name = pc_file(h, i, e, file);
        }
        if cdecl(h, MOVIE_RUN_CHECK, &[]) != 0 {
            let (fm, fnum, fname) = (h.rebase(FMT_MODULE), h.rebase(FMT_NUM), h.rebase(FMT_NAME));
            let module: u32 = h.read(h.rebase(MODULE));
            cdecl(h, DBG_PRINTF, &[fm, module]);
            cdecl(h, DBG_PRINTF, &[fnum, file]);
            cdecl(h, DBG_PRINTF, &[fname, name]);
            let module: u32 = h.read(h.rebase(MODULE));
            let (text, fmt) = (h.rebase(ASSERT_TEXT), h.rebase(ASSERT_FMT));
            cdecl(h, YI_ASSERT, &[1, text, fmt, name, module, file]);
        }
        let from_table = cdecl(h, CD_SIZE_FROM_TABLE, &[file]);
        #[allow(clippy::cast_possible_truncation)] // `al`
        h.write(at(e, 0x34), from_table as u8);
        h.write(at(e, 0x35), 0u8);
        h.write(at(e, 0x32), 0u8);
        let task = h.rebase(READ_TASK);
        cdecl(h, REGIST_ASYNC_TASK, &[task, i]);
        h.read(id_at)
    }

    /// [`queue_read`]'s PC-files part for request `e`: sizes, sector, the
    /// name (with the c101 swap), the name copied. Returns the name.
    fn pc_file(h: &mut impl Host, i: u32, e: u32, file: u32) -> u32 {
        let hdd = h.rebase(HDD);
        let size = if h.read::<u8>(hdd) == 0 {
            let map: u32 = h.read(h.rebase(GRAPHIC_MAP_ID));
            let f = if cdecl(h, IS_ETERNAL_CALM_SCENE, &[map]) == 0 {
                CD_FILE_SIZE_PC
            } else {
                CD_FILE_SIZE_CD
            };
            let size = cdecl(h, f, &[file]);
            h.write(at(e, 0x2c), size);
            size
        } else {
            cdecl(h, CD_FILE_SIZE_CD, &[file])
        };
        h.write(e, size);
        h.write(at(e, 0x1c), 0u32);
        let name = if h.read::<u8>(hdd) == 0 {
            let mut name = cdecl(h, CD_FILE_NAME_PC, &[file]);
            let n = file.cast_signed();
            let (q, r) = ((n / 18).cast_unsigned(), n % 18);
            if name == 0 {
                if r == 3 {
                    if cdecl(h, IS_ETERNAL_CALM_MGRP, &[q]) != 0 {
                        name = cdecl(h, ETERNAL_CALM_MGRP_NAME, &[q]);
                    }
                } else if r == 0 && cdecl(h, IS_ETERNAL_CALM_SCENE, &[q]) != 0 {
                    name = cdecl(h, ETERNAL_CALM_SCENE_NAME, &[q]);
                }
            }
            cdecl(h, TK_SYNC_V, &[2]);
            name
        } else {
            let sector = cdecl(h, MODULE_FILE_SECTOR_REL, &[file]);
            h.write(at(e, 0x1c), sector);
            let bytes = cdecl(h, FILE_SIZE_BYTES, &[file]);
            h.write(at(e, 0x2c), bytes);
            h.rebase(HDD_FILE)
        };
        let key = u32::from(h.read::<u16>(h.rebase(SWAP_KEY)));
        if key == h.read::<u32>(h.rebase(SWAP_VALUE)) {
            let from: u32 = h.read(h.rebase(SWAP_FROM));
            let len = strlen(h, name);
            let same = (0..len).all(|k| h.read::<u8>(at(name, k)) == h.read::<u8>(at(from, k)));
            if same {
                cdecl(h, MEMSET, &[name, 0, len]);
                let to: u32 = h.read(h.rebase(SWAP_TO));
                let n = strlen(h, to);
                cdecl(h, MEMCPY, &[name, to, n]);
                let to: u32 = h.read(h.rebase(SWAP_TO));
                let n = strlen(h, to);
                h.write(at(name, n), 0u8);
            }
        }
        let slot = at(h.rebase(NAMES), i.wrapping_mul(0x80));
        for k in 0..0x80 {
            let c: u8 = h.read(at(name, k));
            h.write(at(slot, k), c);
            if c == 0 {
                break;
            }
        }
        name
    }
}
