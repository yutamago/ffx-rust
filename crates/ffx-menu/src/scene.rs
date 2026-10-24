//! Menu scene data (`TkMenu*Scene*`, `TkMenu*Param*`): up to eight loaded
//! blocks (`g_TkMenuSceneData`), each with two name-hashed tables:
//!
//! - `+0x00`: 16 offsets of parameter buckets; a bucket is a `u32` count
//!   and that many records, each a word `(extra << 24) | name offset`
//!   followed by `extra` more words;
//! - `+0x40`: 16 offsets of scene buckets; a bucket is a `u32` count and
//!   12-byte records whose first word is the name's offset.
//!
//! Offsets are from the block's start; 0 is an empty bucket. A name's
//! bucket is the XOR of its bytes, low four bits. Handles are offsets of
//! records from the block's start.

/// The bucket of a name: the XOR of its bytes, low nibble.
#[must_use]
pub fn bucket(name: &[u8]) -> u32 {
    u32::from(name.iter().fold(0u8, |h, &b| h ^ b) & 0xf)
}

/// The originals, on the loaded blocks (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::bucket;
    use crate::gu::bridge::str_eq;

    /// The loaded scene data blocks, eight pointers.
    pub const SCENE_DATA: Global<u32> = Global::new(0x022f_b4dc);

    /// Blocks.
    pub const SCENES: u32 = 8;

    fn block(mem: &impl Mem, i: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        SCENE_DATA.ptr(mem).at(i as u32).get(mem)
    }

    /// The bytes of the C string at `p` (without the 0).
    fn c_str(mem: &impl Mem, p: u32) -> Vec<u8> {
        let mut v = Vec::new();
        loop {
            let b = mem.read::<u8>(p.wrapping_add(u32::try_from(v.len()).unwrap_or(u32::MAX)));
            if b == 0 {
                return v;
            }
            v.push(b);
        }
    }

    /// `TkMenuSetSceneDataAdrs(i, data)` (`0x00a7c7f0`), unchecked.
    pub fn tk_menu_set_scene_data_adrs(mem: &mut impl Mem, i: i32, data: u32) {
        #[allow(clippy::cast_sign_loss)]
        let p = SCENE_DATA.ptr(mem).at(i as u32);
        p.set(mem, data);
    }

    /// `TkMenuGetSceneDataAdrs(i)` (`0x00a7b910`), unchecked.
    pub fn tk_menu_get_scene_data_adrs(mem: &impl Mem, i: i32) -> u32 {
        block(mem, i)
    }

    /// `TkMenuGetSceneRecord(i, handle)` (`0x00a7b8f0`): the record at
    /// `handle` in block `i`, 0 for a 0 handle or no block.
    pub fn tk_menu_get_scene_record(mem: &impl Mem, i: i32, handle: u32) -> u32 {
        if handle == 0 {
            return 0;
        }
        let b = block(mem, i);
        if b == 0 { 0 } else { b.wrapping_add(handle) }
    }

    /// `TkMenuGetSceneOffsetX(i, handle)` (`0x00a7b9d0`) and `TkMenuGetSceneOffsetY`
    /// (`0x00a7ba00`): the record's `i16` at `+8` or `+0xa`, 0 without one.
    pub fn tk_menu_get_scene_field(mem: &impl Mem, i: i32, handle: u32, field: u32) -> i32 {
        match tk_menu_get_scene_record(mem, i, handle) {
            0 => 0,
            r => i32::from(mem.read::<i16>(r.wrapping_add(field))),
        }
    }

    /// `TkMenuSetSceneOffset(i, handle, x, y)` (`0x00a7c810`): the record's
    /// `+8` and `+0xa`.
    pub fn tk_menu_set_scene_pos(mem: &mut impl Mem, i: i32, handle: u32, x: u16, y: u16) {
        let r = tk_menu_get_scene_record(mem, i, handle);
        if r != 0 {
            mem.write(r.wrapping_add(8), x);
            mem.write(r.wrapping_add(0xa), y);
        }
    }

    /// `g_TkMenuSceneOffsetX`/`Y` (`0x022fb4fc`/`0x022fb500`, names ours):
    /// the running scene's `+8`/`+0xa` offsets, as `i32`, for the draws.
    pub const SCENE_OFFSET_X: u32 = 0x022f_b4fc;
    pub const SCENE_OFFSET_Y: u32 = 0x022f_b500;

    /// `MenuScene_TrackStep_00a7b510`.
    pub const TRACK_STEP: u32 = 0x00a7_b510;

    /// `TkMenuExecScene(i, handle, out)` (`0x00a7b460`): one frame of the
    /// scene at `handle` in block `i` (unchecked index): 0 for a 0 handle,
    /// no block, or a record whose word `+4` lacks bit 31 (not started).
    /// Else its offsets go to [`SCENE_OFFSET_X`]/`Y`, and each of the
    /// tracks of its list (`+4` low 30 bits from the block: a count, then
    /// 20-byte tracks) steps (`MenuScene_TrackStep(block, list, track,
    /// block + block[+0x80], out)`); the OR of their results is returned,
    /// and when it is 0 (also with no tracks) the record's bit 30 is
    /// cleared (the scene ended). The original keeps the block in its
    /// `handle` argument slot (`slot`: the caller's slots' address; written
    /// whenever the handle is not 0).
    pub fn tk_menu_exec_scene<M, C>(
        mem: &mut M,
        i: u32,
        handle: u32,
        out: u32,
        slot: u32,
        call: &mut C,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        if handle == 0 {
            return 0;
        }
        let base = SCENE_DATA.ptr(mem).at(i).get(mem);
        mem.write(slot.wrapping_add(4), base);
        if base == 0 {
            return 0;
        }
        let record = base.wrapping_add(handle);
        if record == 0 {
            return 0;
        }
        let flags = mem.read::<u32>(record.wrapping_add(4));
        if flags.cast_signed() >= 0 {
            return 0;
        }
        let x = i32::from(mem.read::<i16>(record.wrapping_add(8)));
        let at = mem.rebase(SCENE_OFFSET_X);
        mem.write(at, x);
        let y = i32::from(mem.read::<i16>(record.wrapping_add(0xa)));
        let at = mem.rebase(SCENE_OFFSET_Y);
        mem.write(at, y);
        let list = (flags & 0x3fff_ffff).wrapping_add(base);
        let table = mem.read::<u32>(base.wrapping_add(0x80)).wrapping_add(base);
        let mut left = mem.read::<i32>(list);
        let mut track = list.wrapping_add(4);
        let mut any = 0;
        let step = mem.rebase(TRACK_STEP);
        while left > 0 {
            any |= call(mem, step, &[base, list, track, table, out]);
            left -= 1;
            track = track.wrapping_add(0x14);
        }
        if any != 0 {
            return any;
        }
        let v = mem.read::<u32>(record.wrapping_add(4)) & 0xbfff_ffff;
        mem.write(record.wrapping_add(4), v);
        0
    }

    /// `TkMenuSceneHash(name)` (`0x008e5cf0`): a name's bucket.
    pub fn tk_menu_scene_hash(mem: &impl Mem, name: u32) -> u32 {
        bucket(&c_str(mem, name))
    }

    /// `TkMenuInitSeq()` (`0x00a7bb00`): no blocks.
    pub fn tk_menu_init_seq(mem: &mut impl Mem) {
        let p = SCENE_DATA.ptr(mem).addr;
        mem.write(p, [0u32; SCENES as usize]);
    }

    /// `TkMenuGetParamOffset(i, offset, n)` (`0x00a7b8c0`): word `n + 1`
    /// from `offset` in block `i` (unchecked), 0 for a negative offset or
    /// no block.
    pub fn tk_menu_get_param_offset(mem: &impl Mem, i: i32, offset: i32, n: i32) -> u32 {
        if offset < 0 {
            return 0;
        }
        let b = block(mem, i);
        if b == 0 {
            return 0;
        }
        #[allow(clippy::cast_sign_loss)]
        let at = b
            .wrapping_add((n as u32).wrapping_mul(4))
            .wrapping_add(4)
            .wrapping_add(offset as u32);
        mem.read(at)
    }

    /// `TkMenuIsTermScene(i, handle)` (`0x00a7bb90`): the scene record at
    /// `handle` in block `i` (unchecked) when its word `+4` lacks bit 30,
    /// else 0 (also for a 0 handle or no block).
    pub fn tk_menu_is_term_scene(mem: &impl Mem, i: i32, handle: u32) -> u32 {
        if handle == 0 {
            return 0;
        }
        let b = block(mem, i);
        if b == 0 {
            return 0;
        }
        let p = b.wrapping_add(handle);
        if p == 0 || mem.read::<u32>(p.wrapping_add(4)) & 0x4000_0000 != 0 {
            return 0;
        }
        p
    }

    /// `TkMenuGetParamHandle(i, name)` (`0x00a7b7d0`): the parameter
    /// record named `name` in block `i` (clamped to 0..=7), 0 when none.
    pub fn tk_menu_get_param_handle(mem: &impl Mem, i: i32, name: u32) -> u32 {
        let base = block(mem, i.clamp(0, 7));
        if base == 0 {
            return 0;
        }
        let hash = bucket(&c_str(mem, name));
        let offset = mem.read::<u32>(base.wrapping_add(hash * 4));
        if offset == 0 {
            return 0;
        }
        let mut record = base.wrapping_add(offset).wrapping_add(4);
        let mut left = mem.read::<u32>(base.wrapping_add(offset));
        while left != 0 {
            let word = mem.read::<u32>(record);
            if str_eq(mem, base.wrapping_add(word & 0x00ff_ffff), name) {
                return record.wrapping_sub(base);
            }
            record = record.wrapping_add((word >> 24) * 4 + 4);
            left -= 1;
        }
        0
    }

    /// `TkMenuGetSceneHandle(i, name)` (`0x00a7b920`): the scene record
    /// named `name` in block `i` (clamped to 0..=7), 0 when none.
    pub fn tk_menu_get_scene_handle(mem: &impl Mem, i: i32, name: u32) -> u32 {
        let base = block(mem, i.clamp(0, 7));
        if base == 0 {
            return 0;
        }
        let hash = bucket(&c_str(mem, name));
        let offset = mem.read::<u32>(base.wrapping_add(0x40 + hash * 4));
        if offset == 0 {
            return 0;
        }
        let mut record = base.wrapping_add(offset).wrapping_add(4);
        let mut left = mem.read::<u32>(base.wrapping_add(offset));
        while left != 0 {
            let name_at = base.wrapping_add(mem.read::<u32>(record));
            if str_eq(mem, name_at, name) {
                return record.wrapping_sub(base);
            }
            record = record.wrapping_add(12);
            left -= 1;
        }
        0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn buckets() {
        assert_eq!(super::bucket(b""), 0);
        assert_eq!(super::bucket(b"ab"), u32::from((b'a' ^ b'b') & 0xf));
    }
}
