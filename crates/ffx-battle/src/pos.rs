//! Battle positions (`docs/rewrite-plan.md`, M2): the battle map's
//! position file (`*0x0112a9b0`) that `MsBtlGetPos` reads and writes.
//!
//! The file starts with a flag byte (set: one record of 0x20 bytes per
//! group, only kind 3 readable; clear: three records of 0x20 per group)
//! and an `i8` group count. A record has `i8` counts at `+2`-`+6` and `+9`,
//! file offsets of position tables at `+0x10`-`+0x24` and `+0x2c`, and its
//! own vectors at `+0x30`, `+0x40` and `+0x50`. Positions are vec4 (16
//! bytes); kind 11 is a table of single floats.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // file, record, pointer, value
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The position file.
    pub const POS_FILE: Global<u32> = Global::new(0x0112_a9b0);

    fn i8_at(mem: &impl Mem, p: u32, o: u32) -> i32 {
        i32::from(mem.read::<i8>(p.wrapping_add(o)))
    }

    fn quiet(v: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(v))
    }

    /// `MsBtlGetPos(write, actor, group, kind, index, vec)` (`0x007abfe0`):
    /// reads (or with `write` writes) position `index` of kind `kind` of
    /// `group` through `vec` (floats through the x87). An actor in a
    /// defense position (`+0x6d4` not 0xff) reads its own (`+0x3b0`) instead.
    /// Kinds: 2, 9, 10 the record's own vectors (9 and 10 return its
    /// counts at `+2`, `+3`); 3-8 the tables (counts `+4`, `+5`, `+6`); 11 a
    /// float; 12-14 a summoner's position (`MsGetSummonerPos(actor, index,
    /// k, vec)`, from the caller). -1 for a bad group, kind or index.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_btl_get_pos<M: Mem>(
        mem: &mut M,
        write: u32,
        actor: u32,
        group: i32,
        kind: i32,
        index: i32,
        vec: u32,
        summoner: impl FnOnce(&mut M, u32, i32, i32, u32),
    ) -> i32 {
        let file = POS_FILE.get(mem);
        if group < 0 || group >= i8_at(mem, file, 1) {
            return -1;
        }
        let rec = if mem.read::<u8>(file) != 0 {
            if kind != 3 {
                return -1;
            }
            file.wrapping_add(group.cast_unsigned() * 0x20)
        } else {
            file.wrapping_add(group.cast_unsigned() * 0x60)
        };
        if actor != 0 && vec != 0 && mem.read::<u8>(actor.wrapping_add(0x6d4)) != 0xff {
            let v: [u32; 4] = mem.read(actor.wrapping_add(0x3b0));
            crate::camera::bridge::EE_MOVE.set(mem, v);
            mem.write(vec, v);
            return 0;
        }
        let table = |mem: &M, count: u32, at: u32| -> Option<u32> {
            if index < 0 || index >= i8_at(mem, rec, count) {
                return None;
            }
            let o: u32 = mem.read(rec.wrapping_add(at));
            Some(
                o.wrapping_add(index.cast_unsigned() * 16)
                    .wrapping_add(file),
            )
        };
        let mut ret = 0;
        let p = match kind {
            2 => Some(rec.wrapping_add(0x30)),
            3 => table(mem, 4, 0x10),
            6 => table(mem, 4, 0x14),
            4 => table(mem, 5, 0x18),
            7 => table(mem, 5, 0x1c),
            5 => table(mem, 6, 0x20),
            8 => table(mem, 6, 0x24),
            9 => {
                ret = i8_at(mem, rec, 2);
                Some(rec.wrapping_add(0x40))
            }
            10 => {
                ret = i8_at(mem, rec, 3);
                Some(rec.wrapping_add(0x50))
            }
            11 => {
                if index < 0 || index >= i8_at(mem, rec, 9) {
                    return -1;
                }
                let o: u32 = mem.read(rec.wrapping_add(0x2c));
                let f = o.wrapping_add(index.cast_unsigned() * 4).wrapping_add(file);
                if f == 0 {
                    return -1;
                }
                if vec != 0 {
                    if write != 0 {
                        let v = quiet(mem.read(vec));
                        mem.write(f, v);
                    } else {
                        let v = quiet(mem.read(f));
                        mem.write(vec, v);
                    }
                }
                return 0;
            }
            12..=14 => {
                if vec != 0 {
                    summoner(mem, actor, index, kind - 12, vec);
                }
                return 0;
            }
            _ => return -1,
        };
        let Some(p) = p else {
            return -1;
        };
        let here: [u32; 4] = mem.read(p);
        if vec == 0 {
            return ret;
        }
        if write != 0 {
            let v: [u32; 4] = mem.read(vec);
            mem.write(p, v.map(quiet));
        } else {
            mem.write(vec, here.map(quiet));
        }
        ret
    }
}
