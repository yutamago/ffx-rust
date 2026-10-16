//! The save-data state: the PS2-style `SaveData*` flags the menus poll and
//! the PC port's `SaveDataManager`.

/// `SaveDataManager::isBusy` for a manager state: false for the idle and
/// finished states (-1, 0, 2, 4, 6, 7, 8, 9, 10), true otherwise.
#[must_use]
pub const fn is_busy(state: i32) -> bool {
    !matches!(state, -1 | 0 | 2 | 4 | 6..=10)
}

/// Bytes of a save the CRC covers, from [`CRC_DATA`] (`TkCRC`'s length).
pub const CRC_LEN: i32 = 0x64b8;
/// Where the CRC'd bytes of a save start.
pub const CRC_DATA: u32 = 0x40;
/// The CRC as a word, inside the CRC'd bytes (cleared while computing).
pub const CRC_WORD: u32 = 0x64f4;

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `SaveDataManager::instance`'s object.
    pub const MANAGER: Global<u32> = Global::new(0x00ce_81f4);
    /// The event ID `SaveDataSetEventId` stores (nothing in the game reads
    /// it back).
    pub const EVENT_ID: Global<u32> = Global::new(0x00cc_b990);
    /// The save screen's state (0 = closed; `SaveDataSetScreenState`).
    pub const SCREEN_STATE: Global<u32> = Global::new(0x00cc_b994);
    /// The save dialog's state.
    pub const DIALOG_STATE: Global<u32> = Global::new(0x00cc_b998);
    /// 1 while the screen saves, else it loads (`SaveDataSetSave`).
    pub const IS_SAVE: Global<u32> = Global::new(0x00cc_b99c);
    /// 1 when the last load succeeded (`SaveDataToLoad`, `TkMenuPreSave`).
    pub const LOAD_RESULT: Global<u8> = Global::new(0x0159_7f74);

    /// `SaveDataManager_instance()` (`0x006f2070`).
    pub fn manager_instance(mem: &impl Mem) -> u32 {
        MANAGER.get(mem)
    }

    /// `SaveDataManager::getRefBuffer` (`0x006f1ed0`, `this` in ECX): `+0x350`.
    pub fn get_ref_buffer(mem: &impl Mem, this: u32) -> u32 {
        mem.read(this.wrapping_add(0x350))
    }

    /// `SaveDataManager::setCbResult(result)` (`0x006f2500`, thiscall): `+4`.
    pub fn set_cb_result(mem: &mut impl Mem, this: u32, result: u32) {
        mem.write(this.wrapping_add(4), result);
    }

    /// `SaveDataManager::isBusy` (`0x006f2090`, `this` in ECX): by the state
    /// at `+8`.
    pub fn is_busy(mem: &impl Mem, this: u32) -> u32 {
        u32::from(super::is_busy(mem.read(this.wrapping_add(8))))
    }

    /// `SaveDataSetEventId(id)` (`0x00648630`).
    pub fn set_event_id(mem: &mut impl Mem, id: u32) {
        EVENT_ID.set(mem, id);
    }

    /// `SaveDataGetScreenState()` (`0x00647f70`).
    pub fn get_screen_state(mem: &impl Mem) -> u32 {
        SCREEN_STATE.get(mem)
    }

    /// `SaveDataIsScreenClosed()` (`0x00648100`).
    pub fn is_screen_closed(mem: &impl Mem) -> u32 {
        u32::from(SCREEN_STATE.get(mem) == 0)
    }

    /// `SaveDataGetDialogState()` (`0x00647ee0`).
    pub fn get_dialog_state(mem: &impl Mem) -> u32 {
        DIALOG_STATE.get(mem)
    }

    /// `SaveDataSetDialogState(state)` (`0x00648620`).
    pub fn set_dialog_state(mem: &mut impl Mem, state: u32) {
        DIALOG_STATE.set(mem, state);
    }

    /// `SaveDataIsSave()` (`0x006480d0`).
    pub fn is_save(mem: &impl Mem) -> u32 {
        u32::from(IS_SAVE.get(mem) == 1)
    }

    /// `SaveDataSetSave(save)` (`0x006486b0`).
    pub fn set_save(mem: &mut impl Mem, save: u32) {
        IS_SAVE.set(mem, save);
    }

    /// `SaveDataIsLoadingSuccessful()` (`0x006480a0`).
    pub fn is_loading_successful(mem: &impl Mem) -> u32 {
        u32::from(LOAD_RESULT.get(mem) == 1)
    }

    /// Set: `SaveDataCheckCrc` passes every save without checking.
    pub const CRC_CHECK_OFF: Global<u32> = Global::new(0x00cc_b9a4);
    /// The Al Bhed fade value `SaveDataSetAlbhedFade` sets.
    pub const ALBHED_FADE: Global<u32> = Global::new(0x00c3_2308);

    /// `SaveDataIsBusy()` (`0x00648040`): the manager's `isBusy`, 0 without
    /// a manager.
    pub fn is_busy_now(mem: &impl Mem) -> u32 {
        match manager_instance(mem) {
            0 => 0,
            m => is_busy(mem, m),
        }
    }

    /// `SaveDataSetScreenState(state)` (`0x006486e0`): closing (0) also
    /// clears the dialog state and the manager's callback result.
    pub fn set_screen_state(mem: &mut impl Mem, state: u32) {
        SCREEN_STATE.set(mem, state);
        if state == 0 {
            DIALOG_STATE.set(mem, 0);
            let m = manager_instance(mem);
            set_cb_result(mem, m, 0);
        }
    }

    /// `SaveDataIsOperationCanceled()` (`0x006480c0`): the manager's
    /// `+0x358`.
    pub fn is_operation_canceled(mem: &impl Mem) -> u32 {
        mem.read(manager_instance(mem).wrapping_add(0x358))
    }

    /// `SaveDataIsLoadingAlbhed()` (`0x00648070`): the manager's `+0x360`
    /// is set.
    pub fn is_loading_albhed(mem: &impl Mem) -> u32 {
        u32::from(mem.read::<u32>(manager_instance(mem).wrapping_add(0x360)) != 0)
    }

    /// `SaveDataSetAlbhedFade(v)` (`0x006485e0`): only with a manager.
    pub fn set_albhed_fade(mem: &mut impl Mem, v: u32) {
        if manager_instance(mem) != 0 {
            ALBHED_FADE.set(mem, v);
        }
    }

    /// `SetLocalDataSlotAndTime(_, slot, a, b)` (`0x00648fa0`): into the
    /// manager (`+0x376` slot, `+0x380`, `+0x384`, `+0x388` = 2).
    pub fn set_local_data_slot_and_time(mem: &mut impl Mem, slot: u16, a: u32, b: u32) {
        let m = manager_instance(mem);
        mem.write(m.wrapping_add(0x376), slot);
        mem.write(m.wrapping_add(0x380), a);
        mem.write(m.wrapping_add(0x384), b);
        mem.write(m.wrapping_add(0x388), 2u16);
    }

    /// `SaveDataGetCrc(save)` (`0x00647e80`): clears the stored CRC word
    /// (`+0x64f4`) and returns the save's CRC, given `TkCRC(len, data)` (in
    /// `ffx-menu`; only `AX` is defined, as there).
    pub fn get_crc<M: Mem>(mem: &mut M, save: u32, crc: impl Fn(&M, i32, u32) -> u16) -> u16 {
        mem.write(save.wrapping_add(super::CRC_WORD), 0u32);
        crc(mem, super::CRC_LEN, save.wrapping_add(super::CRC_DATA))
    }

    /// `SaveDataWriteCrc(save)` (`0x00648f20`): the save's CRC, given
    /// `TkCRC(len, data)` (in `ffx-menu`), into `+0x1a` and the CRC word;
    /// returns 0.
    pub fn write_crc<M: Mem>(mem: &mut M, save: u32, crc: impl Fn(&M, i32, u32) -> u16) -> u32 {
        mem.write(save.wrapping_add(super::CRC_WORD), 0u32);
        mem.write(save.wrapping_add(super::CRC_WORD), 0u32);
        let c = crc(mem, super::CRC_LEN, save.wrapping_add(super::CRC_DATA));
        mem.write(save.wrapping_add(0x1a), c);
        mem.write(save.wrapping_add(super::CRC_WORD), u32::from(c));
        0
    }

    /// `SaveDataCheckCrc()` (`0x00647d70`): whether the manager's buffer
    /// holds a save whose CRC (`+0x1a`) matches, also left in its byte
    /// `+4`; given `TkCRC`.
    pub fn check_crc<M: Mem>(mem: &mut M, crc: impl Fn(&M, i32, u32) -> u16) -> u32 {
        let m = manager_instance(mem);
        let buf = get_ref_buffer(mem, m);
        let stored: u16 = mem.read(buf.wrapping_add(0x1a));
        mem.write(buf.wrapping_add(4), 0u8);
        let ok = CRC_CHECK_OFF.get(mem) != 0
            || (u32::from(stored) == mem.read::<u32>(buf.wrapping_add(super::CRC_WORD)) && {
                mem.write(buf.wrapping_add(super::CRC_WORD), 0u32);
                crc(mem, super::CRC_LEN, buf.wrapping_add(super::CRC_DATA)) == stored
            });
        if ok {
            mem.write(buf.wrapping_add(4), 1u8);
        }
        u32::from(ok)
    }

    /// The table `0x006f0fb0` sorts (`0x00ce7318`): 200 entries of three
    /// words (an id, then the two sort keys).
    pub const SORTED_TABLE: Global<[u32; 3]> = Global::new(0x00ce_7318);

    /// `0x006f0fb0()`: entries 1..200 of [`SORTED_TABLE`] selection-sorted
    /// by their second and third words, largest first (signed; on a tie the
    /// first found stays); entry 0 is left alone. Returns what `eax` ends
    /// as (from the last pass: the swapped entry's id, or the index found).
    pub fn sort_table(mem: &mut impl Mem) -> u32 {
        let base = SORTED_TABLE.ptr(mem).addr;
        let at = |i: i32| base.wrapping_add(i.cast_unsigned() * 12);
        let mut eax = 0;
        for i in 1..200 {
            let (mut best, mut best_id, mut best_b, mut best_c) = (-1i32, u32::MAX, -1i32, -1i32);
            for j in i..200 {
                let e = at(j);
                let b: i32 = mem.read(e + 4);
                let c: i32 = mem.read(e + 8);
                if b > best_b || (b == best_b && c > best_c) {
                    best_id = mem.read(e);
                    best_c = c;
                    best = j;
                    best_b = b;
                }
            }
            eax = best.cast_unsigned();
            if best > i {
                let (src, dest) = (at(i), at(best));
                let entry: [u32; 3] = mem.read(src);
                mem.write(dest, entry);
                mem.write(
                    src,
                    [best_id, best_b.cast_unsigned(), best_c.cast_unsigned()],
                );
                eax = best_id;
            }
        }
        eax
    }

    /// `0x00648640(v)`: word `+0x360` of the save-data manager set;
    /// returns the manager.
    pub fn set_manager_word360(mem: &mut impl Mem, v: u32) -> u32 {
        let m = manager_instance(mem);
        mem.write(m.wrapping_add(0x360), v);
        m
    }

    /// `0x006f1e40(id)` (`thiscall`, `ret 4`): the first of the manager's
    /// `+0x3a0` 0x20-byte entries (at `*(this + 0x368)`) whose half at `+6`
    /// is `id`, or 0.
    pub fn find_entry_by_id(mem: &impl Mem, this: u32, id: u32) -> u32 {
        let count: i32 = mem.read(this.wrapping_add(0x3a0));
        let base: u32 = mem.read(this.wrapping_add(0x368));
        for k in 0..count.max(0).cast_unsigned() {
            let e = base.wrapping_add(k << 5);
            if u32::from(mem.read::<u16>(e.wrapping_add(6))) == id {
                return e;
            }
        }
        0
    }

    /// `0x006fc390(key)` (`thiscall`, `ret 4`): every in-use (byte 1)
    /// 0x18-byte slot (`*(this + 4)`, count in the low 31 bits of `+0`)
    /// whose `+4` is `key` released (byte 0). Returns 0.
    pub fn release_slots(mem: &mut impl Mem, this: u32, key: u32) -> u32 {
        let count = mem.read::<u32>(this) & 0x7fff_ffff;
        for k in 0..count {
            let slots: u32 = mem.read(this.wrapping_add(4));
            let s = slots.wrapping_add(k * 0x18);
            if mem.read::<u8>(s) == 1 && mem.read::<u32>(s.wrapping_add(4)) == key {
                mem.write(s, 0u8);
            }
        }
        0
    }
}
