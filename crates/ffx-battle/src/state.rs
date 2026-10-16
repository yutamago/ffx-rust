//! The battle's own globals (`docs/rewrite-plan.md`, M2): whether a battle
//! runs and which, the work memory tally, the effect request queue.

/// Bytes `MsMemCheckRegist` books for a block of `size`: `size + 0x4f`
/// rounded toward zero to 64 bytes.
#[must_use]
pub const fn mem_check_size(size: i32) -> i32 {
    let x = size.wrapping_add(0x4f);
    x.wrapping_add((x >> 31) & 0x3f) & !0x3f
}

/// Requests `MsAtelRequestAdd` queues at most.
pub const ATEL_REQUESTS: i32 = 16;

/// The battle globals where the game keeps them, through
/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    use super::{ATEL_REQUESTS, mem_check_size};

    /// Battle status (`MsBattleGetStatus`), signed; not 0 in battle.
    pub const STATUS: Global<i8> = Global::new(0x0112_a8e0);
    /// Set while the battle runs commands (`MsBattleExeClear` keeps it only
    /// while [`EXE_KEEP`] is set).
    pub const EXE: Global<i8> = Global::new(0x0112_a8e2);
    /// Keeps [`EXE`] across `MsBattleExeClear`.
    pub const EXE_KEEP: Global<u8> = Global::new(0x0112_a8f5);
    /// Pointer to the voice event flag (`MsGetVoiceEventFlag`).
    pub const VOICE_EVENT: Global<u32> = Global::new(0x0112_a9c0);
    /// Pointer to the battle field's ID (`i16`, `MsGetBattleScene`).
    pub const SCENE_FIELD: Global<u32> = Global::new(0x0112_a9f0);
    /// Pointer to the encounter group's ID (`u8`, `MsGetBattleScene`).
    pub const SCENE_GROUP: Global<u32> = Global::new(0x0112_a9fc);
    /// Read cue level (`MsBtlReadCueLevel`).
    pub const READ_CUE_LEVEL: Global<u8> = Global::new(0x0112_c021);
    /// Mutes battle voice events (`MsGetVoiceEventFlag`).
    pub const VOICE_MUTE: Global<u8> = Global::new(0x0112_c9ce);
    /// Event effect state (`MsEventEffectGetStat`), signed.
    pub const EVENT_EFFECT_STAT: Global<i8> = Global::new(0x0113_3364);
    /// Number of queued effect requests.
    pub const ATEL_COUNT: Global<i32> = Global::new(0x0113_456c);
    /// The effect requests, 16 × four `u16`.
    pub const ATEL_REQUEST: Global<[u16; 4]> = Global::new(0x0113_5d70);
    /// Work memory booked (`MsMemCheckRegist`), from 0x40.
    pub const MEM_CHECK: Global<i32> = Global::new(0x0113_6000);
    /// Cleared with [`MEM_CHECK`] by `MsMemCheckInit`.
    pub const MEM_CHECK2: Global<i32> = Global::new(0x0113_6004);

    /// `MsBattleGetStatus` (`0x007816f0`).
    pub fn ms_battle_get_status(mem: &impl Mem) -> i32 {
        i32::from(STATUS.get(mem))
    }

    /// `MsGetBattleScene` (`0x00782970`): field ID `<< 16` + group ID while
    /// in battle, else 0 (battle scripts compare it with `0x02050000`).
    pub fn ms_get_battle_scene(mem: &impl Mem) -> u32 {
        let field = SCENE_FIELD.get(mem);
        if STATUS.get(mem) == 0 || field == 0 {
            return 0;
        }
        let f = i32::from(Ptr::<i16>::new(field).get(mem)).cast_unsigned();
        let g = Ptr::<u8>::new(SCENE_GROUP.get(mem)).get(mem);
        (f << 16).wrapping_add(u32::from(g))
    }

    /// `MsBattleExeClear` (`0x00781530`): returns [`EXE`] (signed) and
    /// clears it unless [`EXE_KEEP`] is set.
    pub fn ms_battle_exe_clear(mem: &mut impl Mem) -> i32 {
        let keep = EXE_KEEP.get(mem) != 0;
        let v = EXE.get(mem);
        EXE.set(mem, if keep { v } else { 0 });
        i32::from(v)
    }

    /// `MsEventEffectGetStat` (`0x00788bd0`).
    pub fn ms_event_effect_get_stat(mem: &impl Mem) -> i32 {
        i32::from(EVENT_EFFECT_STAT.get(mem))
    }

    /// `MsBtlReadCueLevel` (`0x007ad180`).
    pub fn ms_btl_read_cue_level(mem: &impl Mem) -> u32 {
        u32::from(READ_CUE_LEVEL.get(mem))
    }

    /// `MsGetVoiceEventFlag` (`0x007986a0`): 1 when the voice event flag is
    /// set and voices are not muted.
    pub fn ms_get_voice_event_flag(mem: &impl Mem) -> u32 {
        let on = Ptr::<u8>::new(VOICE_EVENT.get(mem)).get(mem) != 0;
        u32::from(on && VOICE_MUTE.get(mem) == 0)
    }

    /// `MsMemCheckInit` (`0x00799ee0`).
    pub fn ms_mem_check_init(mem: &mut impl Mem) {
        MEM_CHECK.set(mem, 0x40);
        MEM_CHECK2.set(mem, 0);
    }

    /// `MsMemCheckRegist(name, size)` (`0x00799f00`): books a block.
    pub fn ms_mem_check_regist(mem: &mut impl Mem, size: i32) {
        let v = MEM_CHECK.get(mem).wrapping_add(mem_check_size(size));
        MEM_CHECK.set(mem, v);
    }

    /// `MsAtelRequestInit` (`0x00797ff0`): empties the queue.
    pub fn ms_atel_request_init(mem: &mut impl Mem) {
        ATEL_COUNT.set(mem, 0);
    }

    /// `MsAtelRequestAdd(a, b, c, d)` (`0x00797b70`): queues a request
    /// unless 16 are queued.
    pub fn ms_atel_request_add(mem: &mut impl Mem, request: [u16; 4]) {
        let n = ATEL_COUNT.get(mem);
        if n < ATEL_REQUESTS {
            ATEL_REQUEST
                .ptr(mem)
                .at(n.cast_unsigned())
                .set(mem, request);
            ATEL_COUNT.set(mem, n + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_round_to_64() {
        assert_eq!(mem_check_size(0), 0x40);
        assert_eq!(mem_check_size(0x31), 0x80);
        assert_eq!(mem_check_size(-0x100), -0x80);
    }
}
