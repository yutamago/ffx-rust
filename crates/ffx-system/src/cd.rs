//! The PS2 CD layer the PC port emulates (`libmscd.c`): file numbers,
//! sectors and sizes from the tables `CdEnvInit` (`0x0076c560`) loads, the
//! HDD-install bookkeeping, and the read-name substitution table
//! (`read_debug.c`).
//!
//! A PS2 file is known by its number. `cdrom.mdg` (at `g_CdLib + 0x28`)
//! holds one `u32` per file: the start sector in the low 22 bits, bit 22 when
//! the size comes from `sizetbl.bin`, bit 23 when the file is absent, and the
//! unused bytes at the end over 8 in the top byte (`CDfileSize_CD`). The
//! size functions take numbers relative to the current module, whose first
//! number is at `g_CdLib + 0x20`.

/// A sector-table entry's start sector, with the disc's base added.
#[must_use]
pub const fn start_sector(entry: u32, base: u32) -> u32 {
    (entry & 0x3f_ffff).wrapping_add(base)
}

/// The DVD layout the nine `dvd_cd_*` functions (`0x0076d6c0` +
/// `0x80 * kind`) report, by kind: FAT data size, FAT sector, FAT size, FID
/// sector, FID size, ID sector, ID size, and two more (`0x0076da40`,
/// `0x0076dac0`). Each function picks one of seven getters by
/// `MsGetSaveConfigEnglish` and `TkGetLang` (`0x0077a4f0` + `0x90 *
/// variant` + `0x10 * kind`), but every variant returns the same value,
/// so the choice does not matter on PC.
pub const DVD_CD_LAYOUT: [u32; 9] = [2, 0x118, 0x2_0000, 0x158, 0x82, 0x117, 0x30, 0xf, 0x10];

/// The `dvd_cd_*` kind of a getter at `0x0077a4f0..0x0077a8e0`.
#[must_use]
pub const fn dvd_cd_getter_kind(addr: u32) -> usize {
    ((addr - 0x0077_a4f0) / 0x10 % 9) as usize
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `g_CdLib`: the CD library's state (its function table first).
    pub const CD_LIB: Global<u32> = Global::new(0x0231_0c40);
    /// `g_CdLib + 0x20`: the current module's first file number.
    pub const FILE_BASE: Global<i32> = Global::new(0x0231_0c60);
    /// `g_CdLib + 0x28`: the `cdrom.mdg` sector table.
    pub const SECTORS: Global<u32> = Global::new(0x0231_0c68);
    /// Added to every sector from the table (`CdEnvInit`: the disc's first
    /// sector, less the table's own).
    pub const SECTOR_BASE: Global<u32> = Global::new(0x0112_8ab4);
    /// The extra size table (`pc_sizetbl_set_Ex`), one `u32` per file number.
    pub const EX_SIZES: Global<u32> = Global::new(0x0112_8a90);
    /// Entries in [`EX_SIZES`].
    pub const EX_SIZE_COUNT: Global<i32> = Global::new(0x0112_8a94);
    /// Entries in the HDD-install table ([`HDD_INSTALL`]; `hdd_install_check`).
    pub const HDD_INSTALL_COUNT: Global<i32> = Global::new(0x0112_8ab8);
    /// The HDD-install table: 52-byte entries, archive at `+0` (`u8`), file
    /// number at `+8`, size at `+0x10`.
    pub const HDD_INSTALL: Global<u32> = Global::new(0x0112_8ae4);
    /// Set by `disk_open_check_set`, read by the read loop (`0x0076ccb0`).
    pub const DISK_OPEN_CHECK: Global<u8> = Global::new(0x0112_8aac);
    /// Whether the HDD is mounted (`hdd_mount`, `hdd_unmount`).
    pub const HDD_MOUNTED: Global<i8> = Global::new(0x0112_a508);
    /// The HDD heads: [`HDD_HEAD_COUNT`] entries of 12 bytes, read mode
    /// (`i8`) at `+8`.
    pub const HDD_HEADS: Global<[u8; 12]> = Global::new(0x0112_a528);
    /// Entries in [`HDD_HEADS`] (`hdd_head_reset` runs to `0x0112a790`).
    pub const HDD_HEAD_COUNT: u32 = 51;
    /// The read-name substitution table (`set_read_chgname(a, b)` puts `a`
    /// here and `b` in [`CHGNAME_TO`], in the first free of 16 slots).
    pub const CHGNAME_FROM: Global<u32> = Global::new(0x01a8_5bb0);
    /// The other half of [`CHGNAME_FROM`].
    pub const CHGNAME_TO: Global<u32> = Global::new(0x01a8_5bf0);
    /// Toggled by `setFullArenaLocalizationEnable`.
    pub const FULL_ARENA_LOCALIZATION: Global<u32> = Global::new(0x01a8_5ba8);

    /// `GetCD()` (`0x0076d0d0`): `&g_CdLib`.
    pub fn get_cd(mem: &impl Mem) -> u32 {
        CD_LIB.ptr(mem).addr
    }

    /// `cd_file_sector(n)` (`0x0076d510`): the absolute start sector of entry
    /// `n` of the sector table (`n` is not offset by the module).
    pub fn cd_file_sector(mem: &impl Mem, n: i32) -> u32 {
        let entry = SECTORS
            .get(mem)
            .wrapping_add(n.cast_unsigned().wrapping_mul(4));
        super::start_sector(mem.read(entry), SECTOR_BASE.get(mem))
    }

    /// `CDfileSize_CD_EX(n)` (`0x0076c410`): file `n`'s entry in the extra
    /// size table, or -1 past its end (the index is signed and not checked
    /// below 0).
    pub fn cd_file_size_ex(mem: &impl Mem, n: i32) -> u32 {
        let i = n.wrapping_add(FILE_BASE.get(mem));
        if i >= EX_SIZE_COUNT.get(mem) {
            return u32::MAX;
        }
        mem.read(
            EX_SIZES
                .get(mem)
                .wrapping_add(i.cast_unsigned().wrapping_mul(4)),
        )
    }

    /// `check_ex_file_size(archive, n)` (`0x0076d660`): the size the
    /// HDD-install table records for file `n` of `archive`, or 0. `archive` is
    /// compared whole with the entry's byte.
    pub fn check_ex_file_size(mem: &impl Mem, archive: u32, n: i32) -> u32 {
        let count = HDD_INSTALL_COUNT.get(mem);
        let mut e = HDD_INSTALL.get(mem);
        for _ in 0..count.max(0) {
            if u32::from(mem.read::<u8>(e)) == archive && mem.read::<i32>(e.wrapping_add(8)) == n {
                return mem.read(e.wrapping_add(0x10));
            }
            e = e.wrapping_add(0x34);
        }
        0
    }

    /// `disk_open_check_set(flag)` (`0x0076d6b0`).
    pub fn disk_open_check_set(mem: &mut impl Mem, flag: u8) {
        DISK_OPEN_CHECK.set(mem, flag);
    }

    /// `hdd_mount_flg_get()` (`0x00779560`).
    pub fn hdd_mount_flg_get(mem: &impl Mem) -> i32 {
        i32::from(HDD_MOUNTED.get(mem))
    }

    /// `hdd_rmode_get(n)` (`0x00779570`): head `n`'s read mode (not checked).
    pub fn hdd_rmode_get(mem: &impl Mem, n: i32) -> i32 {
        let head = HDD_HEADS.ptr(mem).addr;
        i32::from(
            mem.read::<i8>(
                head.wrapping_add(n.cast_unsigned().wrapping_mul(12))
                    .wrapping_add(8),
            ),
        )
    }

    /// `hdd_head_reset()` (`0x007789e0`): every head to (0, 0, read mode 1).
    pub fn hdd_head_reset(mem: &mut impl Mem) {
        let mut head = HDD_HEADS.ptr(mem).addr;
        for _ in 0..HDD_HEAD_COUNT {
            mem.write(head, 0u32);
            mem.write(head + 4, 0u32);
            mem.write(head + 8, 1u8);
            head += 12;
        }
    }

    /// `clear_read_chgname(i)` (`0x00a42f30`): frees slot `i` of the
    /// read-name substitution table (not checked).
    pub fn clear_read_chgname(mem: &mut impl Mem, i: i32) {
        let off = i.cast_unsigned().wrapping_mul(4);
        let from = CHGNAME_FROM.ptr(mem).addr.wrapping_add(off);
        let to = CHGNAME_TO.ptr(mem).addr.wrapping_add(off);
        mem.write(from, 0u32);
        mem.write(to, 0u32);
    }

    /// `setFullArenaLocalizationEnable()` (`0x00a431f0`): toggles the flag
    /// (any non-zero value becomes 0).
    pub fn set_full_arena_localization_enable(mem: &mut impl Mem) {
        let v = FULL_ARENA_LOCALIZATION.get(mem);
        FULL_ARENA_LOCALIZATION.set(mem, u32::from(v == 0));
    }

    /// The two words `hdd_state_clear` zeroes (before [`HDD_MOUNTED`]).
    pub const HDD_STATE: Global<[u32; 2]> = Global::new(0x0112_a4fc);
    /// The HDD status bytes: `+0` (`0x0112a509`) and the error `+1`
    /// (`0x0112a50a`), after [`HDD_MOUNTED`].
    pub const HDD_STATUS: Global<i8> = Global::new(0x0112_a509);
    /// The HDD error byte (`get_hdd_err`).
    pub const HDD_ERR: Global<i8> = Global::new(0x0112_a50a);
    /// The two 8-character names `hdd_set_pass_names` writes (12 bytes
    /// apart).
    pub const HDD_PASS_NAMES: Global<[u32; 2]> = Global::new(0x00c4_1758);

    /// `hdd_state_clear()` (`0x00777420`).
    pub fn hdd_state_clear(mem: &mut impl Mem) {
        HDD_STATE.set(mem, [0, 0]);
    }

    /// `get_hdd_err()` (`0x00777e80`): the error byte, sign-extended.
    pub fn hdd_err(mem: &impl Mem) -> i32 {
        i32::from(HDD_ERR.get(mem))
    }

    /// `get_hdd_status()` (`0x00777e90`): 2 on an error, else the status
    /// byte, sign-extended.
    pub fn hdd_status(mem: &impl Mem) -> i32 {
        if HDD_ERR.get(mem) != 0 {
            2
        } else {
            i32::from(HDD_STATUS.get(mem))
        }
    }

    /// `hdd_set_pass_names()` (`0x00777940`): `"ffxpassf"` and `"ffxpassr"`
    /// (as the four words the original stores).
    pub fn hdd_set_pass_names(mem: &mut impl Mem) {
        let p = HDD_PASS_NAMES.ptr(mem).addr;
        mem.write(p, *b"ffxpassf");
        mem.write(p + 12, *b"ffxpassr");
    }

    /// `g_CdLib + 0x2c`: a table of `i16` file-number offsets (by module).
    pub const MODULE_FILES: Global<u32> = Global::new(0x0231_0c6c);
    /// The scrambled names `hdd_set_pass_names_x` writes over
    /// [`HDD_PASS_NAMES`], as four words.
    pub const HDD_PASS_SCRAMBLED: [u32; 4] = [0x97a9_9d9e, 0xa89a_a09a, 0x9fa7_95a1, 0x6ba7_9da2];

    /// `cd_module_file_sector(m, n)` (`0x0076c270`): [`cd_file_sector`] of
    /// file `n` of module `m` (the module's first file from the `i16`
    /// table at `g_CdLib + 0x2c`).
    pub fn module_file_sector(mem: &impl Mem, m: i32, n: i32) -> u32 {
        let t = MODULE_FILES.get(mem);
        let first: i16 = mem.read(t.wrapping_add(m.cast_unsigned().wrapping_mul(2)));
        cd_file_sector(mem, i32::from(first).wrapping_add(n))
    }

    /// `hdd_set_pass_names_x()` (`0x00779a80`): the two 8-byte names from
    /// scrambled words, byte `i` of each then raised by `0xcd - i`.
    pub fn hdd_set_pass_names_x(mem: &mut impl Mem) {
        let p = HDD_PASS_NAMES.ptr(mem).addr;
        let w = HDD_PASS_SCRAMBLED;
        mem.write(p, [w[0], w[1]]);
        mem.write(p + 12, [w[2], w[3]]);
        for i in 0..8u8 {
            for at in [p, p + 12] {
                let a = at + u32::from(i);
                let b: u8 = mem.read(a);
                mem.write(a, b.wrapping_add(0xcd_u8.wrapping_sub(i)));
            }
        }
    }

    /// `hdd_install_find(archive, n)` (`0x0076d600`): the HDD-install
    /// entry of archive `archive` (its low byte compared) and file `n`, or
    /// 0 (also for -1 in either, or an empty table).
    pub fn hdd_install_find(mem: &impl Mem, archive: u32, n: u32) -> u32 {
        let count = HDD_INSTALL_COUNT.get(mem);
        if count == 0 || archive == u32::MAX || n == u32::MAX {
            return 0;
        }
        let mut e = HDD_INSTALL.get(mem);
        for _ in 0..count.max(0) {
            if u32::from(mem.read::<u8>(e)) == archive && mem.read::<u32>(e + 8) == n {
                return e;
            }
            e = e.wrapping_add(0x34);
        }
        0
    }

    /// The CD read channels' flag (`0x02310c58`): when 0,
    /// `cd_channel_clear` also clears the channel's name.
    pub const CHANNEL_KEEP_NAMES: Global<u32> = Global::new(0x0231_0c58);
    /// The channel names, 128 bytes each.
    pub const CHANNEL_NAMES: Global<[u8; 0x80]> = Global::new(0x022f_b6c0);
    /// The channels, 56 bytes each.
    pub const CHANNELS: Global<[u8; 0x38]> = Global::new(0x0112_7c80);

    /// `cd_channel_clear(n)` (`0x0076d530`): channel `n`'s words at `+8`,
    /// `+4`, `+0`, `+0x30`, `+0x24` and its byte at `+0x34` zeroed (and
    /// its name's first byte, unless [`CHANNEL_KEEP_NAMES`]).
    pub fn cd_channel_clear(mem: &mut impl Mem, n: u32) {
        if CHANNEL_KEEP_NAMES.get(mem) == 0 {
            let at = CHANNEL_NAMES.ptr(mem).addr.wrapping_add(n.wrapping_shl(7));
            mem.write(at, 0u8);
        }
        let c = CHANNELS.ptr(mem).addr.wrapping_add(n.wrapping_mul(0x38));
        for off in [8, 4, 0, 0x30, 0x24] {
            mem.write(c.wrapping_add(off), 0u32);
        }
        mem.write(c.wrapping_add(0x34), 0u8);
    }

    /// `g_CdLib + 0x70`: non-zero while a channel is being read.
    pub const READING: Global<u32> = Global::new(0x0231_0cb0);
    /// `g_CdLib + 0x74`: the channel being read.
    pub const READING_CHANNEL: Global<u32> = Global::new(0x0231_0cb4);

    /// `0x0076d0e0(file, archive)`: the first of the 64 [`CHANNELS`] in use (byte
    /// `+0x31` non-zero) for file `+0x28` and archive `+8` (64 channels):
    /// 2 when it is the one being read ([`READING`], [`READING_CHANNEL`]),
    /// 1 when it is only queued, 0 for none.
    pub fn channel_state(mem: &impl Mem, file: u32, archive: u32) -> u32 {
        let base = CHANNELS.ptr(mem).addr;
        for k in 0..64u32 {
            let ch = base.wrapping_add(k * 0x38);
            if mem.read::<u8>(ch.wrapping_add(0x31)) != 0
                && mem.read::<u32>(ch.wrapping_add(0x28)) == file
                && mem.read::<u32>(ch.wrapping_add(8)) == archive
            {
                return if READING.get(mem) != 0 && READING_CHANNEL.get(mem) == k {
                    2
                } else {
                    1
                };
            }
        }
        0
    }

    /// `0x0076c250(n)`: [`cd_file_sector`] of `n` relative to the current
    /// module ([`FILE_BASE`] added; a tail jump).
    pub fn cd_module_file_sector_rel(mem: &impl Mem, n: i32) -> u32 {
        cd_file_sector(mem, n.wrapping_add(FILE_BASE.get(mem)))
    }

    /// `set_read_chgname(from, to)` (`0x00a43220`): the pair in the first
    /// slot of [`CHGNAME_FROM`] that is 0; with all 16 taken, the assertion
    /// `i!=FILE_NAME_CHG_TBL_CNT` (`read_debug.c`, line 0x7e) goes to
    /// `movie_err` (passed in: expression, file, line) and slot 16 is
    /// written anyway. Returns the slot.
    pub fn set_read_chgname<M: Mem>(
        mem: &mut M,
        from: u32,
        to: u32,
        assert_fail: impl FnOnce(&mut M, u32, u32, u32),
    ) -> u32 {
        let table = CHGNAME_FROM.ptr(mem).addr;
        let mut i = 0u32;
        while i < 16 && mem.read::<u32>(table + 4 * i) != 0 {
            i += 1;
        }
        if i == 16 {
            let expr = mem.rebase(0x00b7_ad40);
            let file = mem.rebase(0x00b7_ad20);
            assert_fail(mem, expr, file, 0x7e);
        }
        mem.write(table + 4 * i, from);
        mem.write(CHGNAME_TO.ptr(mem).addr + 4 * i, to);
        i
    }

    /// `0x0076dc20(n)`: whether file `n` of the current module is present
    /// (bit 23 clear) with its size in `sizetbl.bin` (bit 22 set).
    pub fn cd_size_from_table(mem: &impl Mem, n: i32) -> u32 {
        let i = n.wrapping_add(FILE_BASE.get(mem)).cast_unsigned();
        let e: u32 = mem.read(SECTORS.get(mem).wrapping_add(i.wrapping_mul(4)));
        u32::from(e & 0x80_0000 == 0 && e & 0x40_0000 != 0)
    }

    /// `0x0076c440(n)`: file `n`'s size in bytes from the sector table:
    /// the sectors to the next entry times 0x800, less 8 times the entry's
    /// top byte; 0 for an absent file (bit 23).
    pub fn cd_file_size_bytes(mem: &impl Mem, n: i32) -> u32 {
        let i = n.wrapping_add(FILE_BASE.get(mem)).cast_unsigned();
        let at = SECTORS.get(mem).wrapping_add(i.wrapping_mul(4));
        let e: u32 = mem.read(at);
        if e & 0x80_0000 != 0 {
            return 0;
        }
        let next: u32 = mem.read(at.wrapping_add(4));
        let sectors = (next & 0x3f_ffff).wrapping_sub(e & 0x3f_ffff);
        (sectors << 8).wrapping_sub(e >> 24) << 3
    }

    /// `PcReadFunc_set()` (`0x00a42ba0`): the PC's read functions into
    /// `g_CdLib`'s table (offset, function).
    pub const PC_READ_FUNCS: [(u32, u32); 9] = [
        (0x14, 0x00a4_28a0),
        (0x58, 0x00a4_2830),
        (0x54, 0x00a4_2a00),
        (0x68, 0x00a4_32a0),
        (0x60, 0x00a4_2b70),
        (0x78, 0x00a4_2870),
        (0x88, 0x00a4_29c0),
        (0x8c, 0x00a4_29b0),
        (0x04, 0x00a4_2880),
    ];

    /// See [`PC_READ_FUNCS`].
    pub fn pc_read_func_set(mem: &mut impl Mem) {
        let table = CD_LIB.ptr(mem).addr;
        for (off, f) in PC_READ_FUNCS {
            let f = mem.rebase(f);
            mem.write(table + off, f);
        }
    }

    /// `get_hdd_name(n)` (`0x00777eb0`): `sprintf(0x0112a78c, "pfs0:ffx%d",
    /// n)`; returns the buffer.
    pub fn get_hdd_name(mem: &mut impl Mem, n: i32) -> u32 {
        let buf = mem.rebase(0x0112_a78c);
        let mut text = format!("pfs0:ffx{n}").into_bytes();
        text.push(0);
        mem.write_bytes(buf, &text);
        buf
    }

    /// The CD reader's buffer pointer (`0x01128ac8`, `get_read_buffer`).
    pub const READ_BUFFER: Global<u32> = Global::new(0x0112_8ac8);
    /// `sa_decode(src, dst)`.
    const SA_DECODE: u32 = 0x00a4_3de0;
    /// `sa_meth01dec_ex(packed, dst, n, window)` (a stand-in on PC).
    const SA_METH01DEC_EX: u32 = 0x00a4_3cb0;
    /// `sa_meth01dec2_ex(packed, dst, n, window)` (a stand-in on PC).
    const SA_METH01DEC2_EX: u32 = 0x00a4_3ca0;

    /// `memcpy(dst, src, n)`.
    fn copy(mem: &mut impl Mem, dst: u32, src: u32, n: u32) {
        let mut bytes = vec![0u8; n as usize];
        mem.read_bytes(src, &mut bytes);
        mem.write_bytes(dst, &bytes);
    }

    /// `cd_decode_data(req)` (`0x0076d440`): decodes the compressed file
    /// a read request (`+4` data, `+0x2c` size) brought in place. The
    /// unpacked size is the unaligned `u32` at data `+1`. Up to 0x1800
    /// bytes are copied to the [`READ_BUFFER`] whole and `sa_decode(data,
    /// buffer)` unpacks them back over the data. A larger file keeps its
    /// last 0x1800 bytes in the buffer and moves its first `size - 0x1800`
    /// bytes up (byte by byte from the end) to end at data `+ unpacked`;
    /// the moved block's first byte picks the decoder: 1
    /// `sa_meth01dec_ex(block, data, n, buffer)`, 2 `sa_meth01dec2_ex`,
    /// anything else none. Returns the decoder's `eax`, or for no decoder
    /// the method byte less 2. `call` takes absolute addresses.
    pub fn decode_data<M: Mem>(
        mem: &mut M,
        req: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let size: u32 = mem.read(req.wrapping_add(0x2c));
        let data: u32 = mem.read(req.wrapping_add(4));
        let unpacked: u32 = mem.read(data.wrapping_add(1));
        let buffer = READ_BUFFER.get(mem);
        if size <= 0x1800 {
            copy(mem, buffer, data, size);
            let f = mem.rebase(SA_DECODE);
            return call(mem, f, &[data, buffer]);
        }
        let packed = size - 0x1800;
        copy(mem, buffer, data.wrapping_add(packed), 0x1800);
        // the head moves up byte by byte, last byte first (the two ranges
        // may overlap)
        let end = data.wrapping_add(unpacked);
        let moved = if packed.cast_signed() > 0 { packed } else { 0 };
        for k in 1..=moved {
            let b: u8 = mem.read(data.wrapping_add(packed - k));
            mem.write(end.wrapping_sub(k), b);
        }
        let block = end.wrapping_sub(moved);
        let method: u8 = mem.read(block);
        let decoder = match method {
            1 => SA_METH01DEC_EX,
            2 => SA_METH01DEC2_EX,
            m => return u32::from(m).wrapping_sub(2),
        };
        let f = mem.rebase(decoder);
        call(mem, f, &[block, data, packed, buffer])
    }

    /// The file name `PcFileName_00a42c00` builds.
    pub const PC_FILE_NAME: u32 = 0x01a8_5c30;
    /// Set by `SetGlobal_01a85ba4` (a debug tweak): use the 200-thunder
    /// `kami0400.ebp`.
    pub const THUNDER_200: Global<u32> = Global::new(0x01a8_5ba4);
    /// `Sg_GetUserName2()`.
    pub const SG_GET_USER_NAME2: u32 = 0x0082_0620;
    /// `ReturnGlobalSx8_006bc960()`: the sign-extended byte at `0x00cccab8`.
    pub const RETURN_GLOBAL_SX8: u32 = 0x006b_c960;
    /// `dbgPrintf(format, ...)`.
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// `"Virtuos ERR: bad filename location (loc var?), loc = %d\n"`.
    pub const BAD_LOC_FORMAT: u32 = 0x00b7_ad5c;
    /// `"file name NULL !!!!!\n"`.
    pub const NAME_NULL_FORMAT: u32 = 0x00b7_ad98;
    /// The three name rewrites at the end of `PcFileName_00a42c00`: the
    /// file and the prefix put before it.
    pub const NAME_PREFIXES: [(&[u8], &[u8]); 3] = [
        (b"nagi0000.ebp", b"dbg_"),
        (b"kami0400.ebp", b"200thunder_"),
        (b"nagi0700.ebp", b"full_"),
    ];

    /// The C string at `s`: `out[j..]` gets every byte of it, the NUL
    /// included, each re-read after the store before it (the original's
    /// loop); returns `j` past the NUL.
    fn copy_name(mem: &mut impl Mem, out: u32, mut j: u32, s: u32) -> u32 {
        let first: u8 = mem.read(s);
        mem.write(out.wrapping_add(j), first);
        j = j.wrapping_add(1);
        if mem.read::<u8>(s) != 0 {
            let mut p = s;
            loop {
                let c: u8 = mem.read(p.wrapping_add(1));
                p = p.wrapping_add(1);
                mem.write(out.wrapping_add(j), c);
                j = j.wrapping_add(1);
                if mem.read::<u8>(p) == 0 {
                    break;
                }
            }
        }
        j
    }

    /// Whether `$USER$` starts at `at` (bytes read up to the first that
    /// differs).
    fn user_tag_at(mem: &impl Mem, at: u32) -> bool {
        (0..6).all(|k: u32| mem.read::<u8>(at.wrapping_add(k)) == b"$USER$"[k as usize])
    }

    /// `strstr(out, name)`, then `sprintf(out, "%s%s%s", head, prefix,
    /// name)` with `head` the bytes before the match: `prefix` goes in
    /// before the first `name` and whatever followed `name` is cut. The
    /// original copies `head` to a 256-byte local first (`strncpy` over a
    /// zeroed buffer; a head of 256 bytes or more overruns it).
    fn put_prefix(mem: &mut impl Mem, out: u32, name: &[u8], prefix: &[u8]) {
        let mut text = Vec::new();
        loop {
            let c: u8 = mem.read(out.wrapping_add(u32::try_from(text.len()).unwrap_or(u32::MAX)));
            if c == 0 {
                break;
            }
            text.push(c);
        }
        let Some(pos) = text.windows(name.len()).position(|w| w == name) else {
            return;
        };
        let mut at = out.wrapping_add(u32::try_from(pos).unwrap_or(u32::MAX));
        for &c in prefix.iter().chain(name).chain(&[0]) {
            mem.write(at, c);
            at = at.wrapping_add(1);
        }
    }

    /// `PcFileName_00a42c00(loc)`: the PC file name of entry `loc` of the
    /// file-name table (`[GetCD() + 0x30]`: a count, then offsets from the
    /// table; entry `loc` runs from offset `loc` to offset `loc + 1`),
    /// built in [`PC_FILE_NAME`], which it returns. `$USER$` becomes
    /// `Sg_GetUserName2()` (unless that is null) and each set read-name
    /// substitution its replacement; then `dbg_` goes before
    /// `nagi0000.ebp` when `ReturnGlobalSx8_006bc960()`, `200thunder_`
    /// before `kami0400.ebp` with [`THUNDER_200`], `full_` before
    /// `nagi0700.ebp` with [`FULL_ARENA_LOCALIZATION`]. Returns 0 after
    /// `dbgPrintf("file name NULL")` without a table, for an empty entry
    /// or one at address 0; a `loc` at or past the count (signed) prints
    /// the bad location first.
    ///
    /// Each substitution overwrites the name from the byte that matched
    /// (the copy's NUL included), and the scan goes on from the last byte
    /// matched and the last byte written: the later substitutions are
    /// tried from there, and the next byte is copied after them. `call`
    /// takes an `ffx.sym` address.
    pub fn pc_file_name<M: Mem>(
        mem: &mut M,
        loc: i32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table: u32 = mem.read(get_cd(mem).wrapping_add(0x30));
        let mut src = 0;
        if table != 0 {
            if loc < mem.read::<i32>(table) {
                let entry = table.wrapping_add(loc.cast_unsigned().wrapping_mul(4));
                let start: u32 = mem.read(entry.wrapping_add(4));
                let end: u32 = mem.read(entry.wrapping_add(8));
                if end.wrapping_sub(start).cast_signed() > 0 {
                    src = start.wrapping_add(table);
                }
            } else {
                let format = mem.rebase(BAD_LOC_FORMAT);
                call(mem, DBG_PRINTF, &[format, loc.cast_unsigned()]);
            }
        }
        if src == 0 {
            let format = mem.rebase(NAME_NULL_FORMAT);
            call(mem, DBG_PRINTF, &[format]);
            return 0;
        }
        let user = call(mem, SG_GET_USER_NAME2, &[]);
        let out = mem.rebase(PC_FILE_NAME);
        let first: u8 = mem.read(src);
        mem.write(out, first);
        // `out[j]` holds the copy of `src[i]`
        let (mut i, mut j) = (0u32, 0u32);
        if mem.read::<u8>(src) != 0 {
            loop {
                if user_tag_at(mem, src.wrapping_add(i)) {
                    if user != 0 {
                        j = copy_name(mem, out, j, user).wrapping_sub(2);
                        i = i.wrapping_add(5);
                    }
                } else {
                    let from_table = CHGNAME_FROM.ptr(mem).addr;
                    let to_table = CHGNAME_TO.ptr(mem).addr;
                    for t in 0..16u32 {
                        let from: u32 = mem.read(from_table.wrapping_add(4 * t));
                        if from == 0 {
                            continue;
                        }
                        let at = src.wrapping_add(i);
                        let mut k = 0u32;
                        if mem.read::<u8>(at) != 0 {
                            loop {
                                let c: u8 = mem.read(from.wrapping_add(k));
                                if c == 0 || mem.read::<u8>(at.wrapping_add(k)) != c {
                                    break;
                                }
                                k = k.wrapping_add(1);
                                if mem.read::<u8>(at.wrapping_add(k)) == 0 {
                                    break;
                                }
                            }
                        }
                        if mem.read::<u8>(from.wrapping_add(k)) == 0 {
                            let to: u32 = mem.read(to_table.wrapping_add(4 * t));
                            j = copy_name(mem, out, j, to).wrapping_sub(2);
                            i = i.wrapping_add(k).wrapping_sub(1);
                        }
                    }
                }
                i = i.wrapping_add(1);
                j = j.wrapping_add(1);
                let c: u8 = mem.read(src.wrapping_add(i));
                mem.write(out.wrapping_add(j), c);
                if mem.read::<u8>(src.wrapping_add(i)) == 0 {
                    break;
                }
            }
        }
        let [(nagi0000, dbg), (kami0400, thunder), (nagi0700, full)] = NAME_PREFIXES;
        if call(mem, RETURN_GLOBAL_SX8, &[]) != 0 {
            put_prefix(mem, out, nagi0000, dbg);
        }
        if THUNDER_200.get(mem) != 0 {
            put_prefix(mem, out, kami0400, thunder);
        }
        if FULL_ARENA_LOCALIZATION.get(mem) != 0 {
            put_prefix(mem, out, nagi0700, full);
        }
        out
    }
    /// `g_CdChannels`: the 64 read requests (0x38 bytes: busy byte
    /// `+0x31`); a 65th starts at [`READ_ID`].
    pub const CD_CHANNELS: u32 = 0x0112_7c80;
    /// `g_CdReadId`: the last read-request id (1..=0xffff).
    pub const READ_ID: Global<i32> = Global::new(0x0112_8a80);
    /// `movie_err(text, file, line)`.
    pub const MOVIE_ERR: u32 = 0x0076_efa0;
    /// `"q_cnt != READ_Q_CNT"` and `"..\program\lib\src\libmscd.c"`.
    pub const QUEUE_FULL_TEXT: u32 = 0x00b5_1f04;
    pub const LIBMSCD_FILE: u32 = 0x00b5_1d30;

    /// `CdQueueSectorRead_0076ce40(sector, a, b)`: takes the first free
    /// read request of [`CD_CHANNELS`] (request 64, over [`READ_ID`] and
    /// what follows, after `movie_err("q_cnt != READ_Q_CNT", libmscd.c,
    /// 0x96d)` when all are busy) and fills it: `+8 = b`, `+0x28 = a`,
    /// busy `+0x31` and `+0x30` set, `+0x1c = sector`, everything else it
    /// owns zeroed; the id `+0x24` is [`READ_ID`] plus 1 (1 past 0xffff),
    /// stored back and returned. The fields are written in the original's
    /// order, the old id read first (request 64's `+0` is the id).
    pub fn queue_sector_read<M: Mem>(
        mem: &mut M,
        [sector, a, b]: [u32; 3],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let channels = mem.rebase(CD_CHANNELS);
        let n = (0..64u32)
            .find(|&n| mem.read::<u8>(channels.wrapping_add(n * 0x38 + 0x31)) == 0)
            .unwrap_or(64);
        if n == 64 {
            let text = mem.rebase(QUEUE_FULL_TEXT);
            let file = mem.rebase(LIBMSCD_FILE);
            call(mem, MOVIE_ERR, &[text, file, 0x96d]);
        }
        let r = channels.wrapping_add(n * 0x38);
        let at = |off: u32| r.wrapping_add(off);
        mem.write(at(8), b);
        mem.write(at(0x28), a);
        let old = READ_ID.get(mem);
        mem.write(at(0x31), 1u8);
        mem.write(at(4), 0u32);
        for off in [0xc, 0x10, 0x14, 0x18, 0, 0x20] {
            mem.write(at(off), 0u32);
        }
        let mut id = old.wrapping_add(1);
        READ_ID.set(mem, id);
        if id > 0xffff {
            id = 1;
            READ_ID.set(mem, id);
        }
        mem.write(at(0x24), id);
        mem.write(at(0x30), 1u8);
        mem.write(at(0x34), 0u16);
        mem.write(at(0x2c), 0u32);
        mem.write(at(0x1c), sector);
        id.cast_unsigned()
    }

    /// `CdQueueSector_0076ce20(sector)`: [`queue_sector_read`] with `a`
    /// and `b` 0.
    pub fn queue_sector<M: Mem>(
        mem: &mut M,
        sector: u32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        queue_sector_read(mem, [sector, 0, 0], call)
    }

    /// The last direct read `CdReadDirect_0076d140` starts: sector, size,
    /// buffer and the mode's first word (`0x01128acc`..).
    pub const DIRECT_READ: u32 = 0x0112_8acc;
    /// `FlushCache(n)` and `sceCdRead(sector, sectors, buffer, mode)`.
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const SCE_CD_READ: u32 = 0x00a4_47c0;
    /// `g_FFXHddMode` (a byte).
    pub const FFX_HDD_MODE: u32 = 0x0133_c913;
    /// The rule, `"warning error !!! not 64byte align\n"` and
    /// `"sceCdRead()\n"` (`dbgPrintf` texts).
    pub const RULE_TEXT: u32 = 0x00b5_1d88;
    pub const ALIGN_TEXT: u32 = 0x00b5_1dd4;
    pub const CD_READ_TEXT: u32 = 0x00b5_1df8;

    /// `CdReadDirect_0076d140(sector, size, buffer, mode)`: clears the byte
    /// `0x01128a87` and the word `0x01128ab0`, prints a ruled "not 64byte
    /// align" warning for a buffer not on 64 bytes, keeps the read in
    /// [`DIRECT_READ`] (`[mode]` read first), runs `hdd_state_clear` and
    /// `FlushCache(0)`, then returns `sceCdRead(sector, size / 2048
    /// (signed, toward zero), buffer, mode)`; with [`FFX_HDD_MODE`] set a
    /// ruled `"sceCdRead()"` follows. `call` takes `ffx.sym` addresses.
    pub fn read_direct<M: Mem>(
        mem: &mut M,
        [sector, size, buffer, mode]: [u32; 4],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let ruled = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, text: u32| {
            let rule = mem.rebase(RULE_TEXT);
            let text = mem.rebase(text);
            call(mem, DBG_PRINTF, &[rule]);
            call(mem, DBG_PRINTF, &[text]);
            call(mem, DBG_PRINTF, &[rule]);
        };
        mem.write(mem.rebase(0x0112_8a87), 0u8);
        mem.write(mem.rebase(0x0112_8ab0), 0u32);
        if buffer & 0x3f != 0 {
            ruled(mem, &mut call, ALIGN_TEXT);
        }
        let mode_word: u32 = mem.read(mode);
        let last = mem.rebase(DIRECT_READ);
        mem.write(last, [sector, size, buffer, mode_word]);
        hdd_state_clear(mem);
        call(mem, FLUSH_CACHE, &[0]);
        let sectors = size.cast_signed() / 2048;
        let r = call(
            mem,
            SCE_CD_READ,
            &[sector, sectors.cast_unsigned(), buffer, mode],
        );
        if mem.read::<u8>(mem.rebase(FFX_HDD_MODE)) != 0 {
            ruled(mem, &mut call, CD_READ_TEXT);
        }
        r
    }
}
