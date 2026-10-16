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
}
