//! The save-data state: the PS2-style `SaveData*` flags the menus poll and
//! the PC port's `SaveDataManager`.

/// `SaveDataManager::isBusy` for a manager state: false for the idle and
/// finished states (-1, 0, 2, 4, 6, 7, 8, 9, 10), true otherwise.
#[must_use]
pub const fn is_busy(state: i32) -> bool {
    !matches!(state, -1 | 0 | 2 | 4 | 6..=10)
}

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
}
