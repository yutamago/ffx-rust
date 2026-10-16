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

    /// `0x0076c250(n)`: [`cd_file_sector`] of `n` relative to the current
    /// module ([`FILE_BASE`] added; a tail jump).
    pub fn cd_module_file_sector_rel(mem: &impl Mem, n: i32) -> u32 {
        cd_file_sector(mem, n.wrapping_add(FILE_BASE.get(mem)))
    }
}
