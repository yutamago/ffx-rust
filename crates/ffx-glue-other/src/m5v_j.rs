//! M5 stage 1, list `vfx`, round 5 batch 5: effect debug helpers: the
//! opus debug command, the debug keys' effect-character bind off/on, the
//! effect number printer's GIF tags, and the battle debug file reader's
//! character classes.

use ffx_bridge::Global;

/// `g_OpEtBindOff` (byte), `g_EtChr` (0x20-byte records; the first 0xcf
/// walked, ids looked up in all 0xf0), `g_OpusBattle`.
pub const OP_ET_BIND_OFF: Global<u8> = Global::new(0x011a_0065);
pub const ET_CHR: u32 = 0x012a_22c0;
pub const OPUS_BATTLE: Global<u32> = Global::new(0x012a_40c0);
/// The opus debug command's point copy (3 floats).
pub const DBG_POINT: u32 = 0x00c4_8cc0;
/// `g_OpdNumPal` (byte), `g_OpdNumFrames`, and the number printer's
/// texture base (`0x00c87880`) and CBP word (`0x00c87870`), both i16.
pub const OPD_NUM_PAL: Global<u8> = Global::new(0x0228_615c);
pub const OPD_NUM_FRAMES: Global<u32> = Global::new(0x0228_81d4);
pub const OPD_NUM_TBP: Global<u16> = Global::new(0x00c8_7880);
pub const OPD_NUM_CBP: Global<u16> = Global::new(0x00c8_7870);

// ---- callees ----

/// `opu_unit_free_by_id(list, id)`, `opus_get_monster_index(chr)`,
/// `op_et_eff(chr, 0, kind, id)` (cdecl).
pub const OPU_UNIT_FREE_BY_ID: u32 = 0x007f_de30;
pub const OPUS_GET_MONSTER_INDEX: u32 = 0x0080_0e80;
pub const OP_ET_EFF: u32 = 0x007f_c2e0;

pub mod bridge {
    use super::{
        DBG_POINT, ET_CHR, OP_ET_BIND_OFF, OP_ET_EFF, OPD_NUM_CBP, OPD_NUM_FRAMES, OPD_NUM_PAL,
        OPD_NUM_TBP, OPU_UNIT_FREE_BY_ID, OPUS_BATTLE, OPUS_GET_MONSTER_INDEX,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `opu_dbg(host, cmd)` -> the next command: kind (`i16 +2`) 1 jumps by
    /// `i16 +4` while `+0x212` is set; 2 copies the unit's (`[+0x220]`)
    /// point `+0x90..` to `0x00c48cc0` (4 bytes); 4 sets `+0x3b8` from
    /// `+4`; 1 (unset), 3 and 4 take 6 bytes; any other kind stays.
    pub fn opu_dbg(h: &mut impl Host, host: u32, cmd: u32) -> u32 {
        let unit: u32 = h.read(host + 0x220);
        match h.read::<i16>(cmd + 2) {
            1 => {
                if h.read::<u16>(host + 0x212) != 0 {
                    return cmd.wrapping_add(i32::from(h.read::<i16>(cmd + 4)).cast_unsigned());
                }
                cmd + 6
            }
            2 => {
                let at = h.rebase(DBG_POINT);
                for k in 0..3 {
                    let v: u32 = h.read(unit + 0x90 + 4 * k);
                    h.write(at + 4 * k, x87::store(x87::load(v)));
                }
                cmd + 4
            }
            3 => cmd + 6,
            4 => {
                let v: u16 = h.read(cmd + 4);
                h.write(host + 0x3b8, v);
                cmd + 6
            }
            _ => cmd,
        }
    }

    /// The first effect-character record (of 0xf0) holding `handle`.
    fn find_chr(h: &impl Host, chr: u32, handle: u32) -> Option<u32> {
        (0..0xf0).find(|&i| h.read::<u32>(chr + 0x20 * i) == handle)
    }

    /// `op_et_bind_all_off_007fbc20()` (a debug key): `g_OpEtBindOff` set,
    /// then for each of the first 0xcf effect characters with a handle, its
    /// record's low two bind bits (`+0xc`) cleared and its four bind
    /// effects (ids `0x40000400 + 0x100 k + i`, k 0..3) freed.
    pub fn bind_all_off(h: &mut impl Host) {
        OP_ET_BIND_OFF.set(h, 1);
        let chr = h.rebase(ET_CHR);
        for r in 0..0xcf {
            let handle: u32 = h.read(chr + 0x20 * r);
            if handle == 0 {
                continue;
            }
            let Some(i) = find_chr(h, chr, handle) else {
                continue;
            };
            let rec = chr + 0x20 * i;
            let flags: u16 = h.read(rec + 0xc);
            h.write(rec + 0xc, flags & 0xfffc);
            for k in 0..4 {
                let list = OPUS_BATTLE.get(h);
                cdecl(h, OPU_UNIT_FREE_BY_ID, &[list, 0x4000_0400 + 0x100 * k + i]);
            }
        }
    }

    /// `op_et_bind_all_on_007fb940()` (a debug key): with
    /// `g_OpEtBindOff` set, it is cleared and each of the first 0xcf
    /// effect characters with a handle (while the flag stays clear) whose
    /// record isn't bound yet (bit 0 of `+0xc`) and has a character (`+8`)
    /// that is a monster (`opus_get_monster_index` >= 0) gets its four bind
    /// effects (`op_et_eff(chr, 0, 4 + k, 0x40000400 + 0x100 k + i)`) and
    /// bit 0 set, bit 1 cleared.
    pub fn bind_all_on(h: &mut impl Host) {
        if OP_ET_BIND_OFF.get(h) == 0 {
            return;
        }
        OP_ET_BIND_OFF.set(h, 0);
        let chr = h.rebase(ET_CHR);
        for r in 0..0xcf {
            let handle: u32 = h.read(chr + 0x20 * r);
            if handle == 0 || OP_ET_BIND_OFF.get(h) != 0 {
                continue;
            }
            let Some(i) = find_chr(h, chr, handle) else {
                continue;
            };
            let rec = chr + 0x20 * i;
            if h.read::<u8>(rec + 0xc) & 1 != 0 {
                continue;
            }
            let who: u32 = h.read(rec + 8);
            if who == 0 || cdecl(h, OPUS_GET_MONSTER_INDEX, &[who]).cast_signed() < 0 {
                continue;
            }
            for k in 0..4 {
                cdecl(h, OP_ET_EFF, &[who, 0, 4 + k, 0x4000_0400 + 0x100 * k + i]);
            }
            let flags: u16 = h.read(rec + 0xc);
            h.write(rec + 0xc, (flags & 0xfffd) | 1);
        }
    }

    /// `opd_num_end_tag_00a5f2d0(p, end)` -> 0x50: the GIF packet's DMA tag
    /// at `p` (quadwords to `end` - 1, id 7) and its A+D register writes:
    /// TEXFLUSH, ALPHA (0x44), TEST (0x47) 0x3000d... as the original
    /// lays them.
    pub fn num_end_tag(h: &mut impl Host, p: u32, end: u32) -> u32 {
        let n = end.wrapping_sub(p).cast_signed();
        let q = ((n + ((n >> 31) & 0xf)) >> 4) - 1;
        h.write(p, q.cast_unsigned() | 0x7000_0000);
        let words: [(u32, u32); 11] = [
            (8, 0),
            (0x10, 1),
            (0x14, 0x3000_0000),
            (0x18, 0xeeee_eeee),
            (0x1c, 0),
            (0x20, 0x44),
            (0x28, 0x42),
            (0x30, 0x3_000d),
            (0x38, 0x47),
            (0x40, 1),
            (0x48, 0x14),
        ];
        for (off, v) in words {
            h.write(p + off, v);
        }
        0x50
    }

    /// `opd_num_tex_tag_00a5e7b0(p, tbw, pal)` -> 0x40: the number
    /// printer's texture GIF tag at `p`: TEX0 with the texture base and the
    /// palette (`g_OpdNumPal` when set, else `pal`; -1 animates with
    /// `g_OpdNumFrames`; 0..3 of it), the CLUT base and `tbw | 0x8000`.
    pub fn num_tex_tag(h: &mut impl Host, p: u32, tbw: u32, pal: u32) -> u32 {
        let fixed = OPD_NUM_PAL.get(h);
        let mut k = if fixed != 0 {
            i32::from(fixed.cast_signed()).cast_unsigned()
        } else {
            pal
        };
        let tbp = i32::from(OPD_NUM_TBP.get(h).cast_signed());
        if k == 0xffff_ffff {
            k = OPD_NUM_FRAMES.get(h);
        }
        let v = tbp.wrapping_add(((k & 3) * 4).cast_signed());
        h.write(p, 1_u32);
        h.write(p + 4, 0x2000_0000_u32);
        h.write(p + 8, 0x1e_u32);
        h.write(p + 0xc, 0_u32);
        let cbp = i32::from(OPD_NUM_CBP.get(h).cast_signed());
        let cbp_sign = (cbp >> 31).cast_unsigned();
        let tbw_sign = (tbw.cast_signed() >> 31).cast_unsigned();
        h.write(p + 0x14, (v.cast_unsigned() << 5) | cbp_sign | 0x2000_0005);
        h.write(p + 0x30, tbw | 0x8000);
        h.write(p + 0x10, cbp.cast_unsigned() | 0x9940_4000);
        for (off, x) in [
            (0x18, 6),
            (0x1c, 0),
            (0x28, 0x80),
            (0x2c, 0x80),
            (0x20, 0x80),
            (0x24, 0x80),
        ] {
            h.write(p + off, x as u32);
        }
        h.write(p + 0x34, tbw_sign | 0x408b_4000);
        h.write(p + 0x38, 0x4343_u32);
        h.write(p + 0x3c, 0_u32);
        0x40
    }

    /// `MsDbgFile_charClasses_007e12d0(table, s1..s7)`: the 256-byte class
    /// table: 5 below 0x80, 2 from 0x80; then each character (signed, so
    /// bytes from 0x80 land before the table) of the seven strings gets
    /// its class 2, 3, 4, 7, 1, 6, 8.
    pub fn char_classes(h: &mut impl Host, table: u32, strings: [u32; 7]) {
        h.write_bytes(table, &[5; 0x80]);
        h.write_bytes(table + 0x80, &[2; 0x80]);
        for (s, class) in strings.into_iter().zip([2_u8, 3, 4, 7, 1, 6, 8]) {
            let mut at = s;
            loop {
                let c: i8 = h.read(at);
                if c == 0 {
                    break;
                }
                h.write(table.wrapping_add(i32::from(c).cast_unsigned()), class);
                at = at.wrapping_add(1);
            }
        }
    }
}
