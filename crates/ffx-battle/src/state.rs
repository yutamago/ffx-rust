//! The battle's own globals (`docs/rewrite-plan.md`, M2): whether a battle
//! runs and which, the work memory tally, the effect request queue, the
//! command and input cues, the action order.

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
    /// The battle is under water (`MsBattleWater`; the formation's byte 3,
    /// set in underwater fields such as `cdsp00`); no voice events then
    /// (`MsGetVoiceEventFlag`).
    pub const UNDERWATER: Global<i8> = Global::new(0x0112_c9ce);
    /// Map circle shown (`MsBattleGetMapCircleOn`), signed.
    pub const MAP_CIRCLE_ON: Global<i8> = Global::new(0x0112_a91b);
    /// Encounter type (`MsGetEncountType`).
    pub const ENCOUNT_TYPE: Global<u8> = Global::new(0x0112_a9d4);
    /// Two stone (petrify) effect counters `MsStoneEffectCheck` adds.
    pub const STONE_EFFECT: Global<[i8; 2]> = Global::new(0x0112_ca0b);
    /// Pad orientation, the cursor type (`MsBattleCursorType`), signed.
    pub const PAD_ORIENTATION: Global<i8> = Global::new(0x0112_c9cd);
    /// The input cue, 0x48-byte entries (`MsGetInputCue`).
    pub const INPUT_CUE: Global<u8> = Global::new(0x0112_aa80);
    /// The command cue, 0x48-byte entries: `+0` actor, `+1` command, `+7`
    /// high nibble its kind (`MsCheckCommandCue`, `MsCheckCommandCue2`).
    pub const COMMAND_CUE: Global<u8> = Global::new(0x0112_ac70);
    /// Entries in [`INPUT_CUE`], signed.
    pub const INPUT_CUE_COUNT: Global<i8> = Global::new(0x0112_bde0);
    /// Entries in [`COMMAND_CUE`], signed.
    pub const COMMAND_CUE_COUNT: Global<i8> = Global::new(0x0112_bde1);
    /// Bytes per cue entry.
    pub const CUE_SIZE: u32 = 0x48;
    /// Number of queued action orders (`MsOrderRegist`).
    pub const ORDER_COUNT: Global<i32> = Global::new(0x0113_33c0);
    /// The action orders, 31 bytes.
    pub const ORDERS: Global<u8> = Global::new(0x0113_33c4);
    /// Battle subtitles muted (`MsMuteBattleSubTitle`).
    pub const MUTE_SUBTITLE: Global<u32> = Global::new(0x0113_3354);
    /// Cleared with [`EVENT_EFFECT_STAT`] by `MsEventEffectInit`.
    pub const EVENT_EFFECT_A: Global<u32> = Global::new(0x0113_3360);
    /// Cleared with [`EVENT_EFFECT_STAT`] by `MsEventEffectInit`.
    pub const EVENT_EFFECT_B: Global<u32> = Global::new(0x0113_336c);
    /// Bytes of sound effect data in transfer (`MsSEPtransAmount`).
    pub const SEP_TRANS: Global<u32> = Global::new(0x0113_618c);
    /// The debug field number, high and low (`MsDebugGetFieldNum`).
    pub const DEBUG_FIELD: Global<[u32; 2]> = Global::new(0x0113_9e50);
    /// The status table, 4 bytes per status; byte 3 is what `MsCheckStat`
    /// returns.
    pub const STATUS_TABLE: Global<u32> = Global::new(0x00c4_23e8);
    /// The second status table (`MsCheckStat2`), from entry 14 of the first.
    pub const STATUS_TABLE2: Global<u32> = Global::new(0x00c4_2420);
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
        u32::from(on && UNDERWATER.get(mem) == 0)
    }

    /// `MsBattleWater` (`0x007821e0`).
    pub fn ms_battle_water(mem: &impl Mem) -> i32 {
        i32::from(UNDERWATER.get(mem))
    }

    /// `MsBattleGetMapCircleOn` (`0x007816a0`).
    pub fn ms_battle_get_map_circle_on(mem: &impl Mem) -> i32 {
        i32::from(MAP_CIRCLE_ON.get(mem))
    }

    /// `MsGetEncountType` (`0x00782a10`).
    pub fn ms_get_encount_type(mem: &impl Mem) -> u32 {
        u32::from(ENCOUNT_TYPE.get(mem))
    }

    /// `MsStoneEffectCheck` (`0x007a2670`): the two counters' sum.
    pub fn ms_stone_effect_check(mem: &impl Mem) -> i32 {
        let [a, b] = STONE_EFFECT.get(mem);
        i32::from(a) + i32::from(b)
    }

    /// `MsBattleCursorType` (`0x00780ce0`).
    pub fn ms_battle_cursor_type(mem: &impl Mem) -> i32 {
        i32::from(PAD_ORIENTATION.get(mem))
    }

    /// `MsBattleGetChrDisposeFlag` (`0x00781630`): always 1.
    #[must_use]
    pub fn ms_battle_get_chr_dispose_flag() -> u32 {
        1
    }

    /// `MsGetInputCue` (`0x007b22e0`): the input cue, null when empty.
    pub fn ms_get_input_cue(mem: &impl Mem) -> u32 {
        if INPUT_CUE_COUNT.get(mem) > 0 {
            INPUT_CUE.ptr(mem).addr
        } else {
            0
        }
    }

    /// `MsGetCommandCue` (`0x007b0a10`): the command cue, null when empty.
    pub fn ms_get_command_cue(mem: &impl Mem) -> u32 {
        if COMMAND_CUE_COUNT.get(mem) > 0 {
            COMMAND_CUE.ptr(mem).addr
        } else {
            0
        }
    }

    /// The first 8 bytes of each command cue entry.
    fn command_cue(mem: &impl Mem) -> impl Iterator<Item = [u8; 8]> + '_ {
        let base = COMMAND_CUE.ptr(mem).addr;
        let n = i32::from(COMMAND_CUE_COUNT.get(mem)).max(0).cast_unsigned();
        (0..n).map(move |i| mem.read(base + i * CUE_SIZE))
    }

    /// `MsCheckCommandCue(actor, command)` (`0x007b0720`): 0 when the
    /// command cue holds that actor's command, else 1.
    pub fn ms_check_command_cue(mem: &impl Mem, actor: u32, command: u32) -> u32 {
        u32::from(!command_cue(mem).any(|e| u32::from(e[0]) == actor && u32::from(e[1]) == command))
    }

    /// `MsCheckCommandCue2(actor, kind)` (`0x007b0770`): 0 when the command
    /// cue holds a command of that actor and kind (high nibble of `+7`).
    pub fn ms_check_command_cue2(mem: &impl Mem, actor: u32, kind: u32) -> u32 {
        u32::from(
            !command_cue(mem)
                .any(|e| u32::from(e[0]) == actor && u32::from(e[7] & 0xf0) == kind & 0xf0),
        )
    }

    /// `MsOrderInit` (`0x0078d4b0`).
    pub fn ms_order_init(mem: &mut impl Mem) {
        ORDER_COUNT.set(mem, 0);
    }

    /// `MsOrderRegist(chr)` (`0x0078d4c0`): queues an actor unless 31 are.
    pub fn ms_order_regist(mem: &mut impl Mem, chr: u8) {
        let n = ORDER_COUNT.get(mem);
        if n < 0x1f {
            ORDERS.ptr(mem).at(n.cast_unsigned()).set(mem, chr);
            ORDER_COUNT.set(mem, n + 1);
        }
    }

    /// `MsMuteBattleSubTitle(on)` (`0x00785990`).
    pub fn ms_mute_battle_sub_title(mem: &mut impl Mem, on: u32) {
        MUTE_SUBTITLE.set(mem, on);
    }

    /// `MsEventEffectInit` (`0x00788be0`).
    pub fn ms_event_effect_init(mem: &mut impl Mem) {
        EVENT_EFFECT_STAT.set(mem, 0);
        EVENT_EFFECT_A.set(mem, 0);
        EVENT_EFFECT_B.set(mem, 0);
    }

    /// `MsSEPtransAmount` (`0x007a04b0`).
    pub fn ms_sep_trans_amount(mem: &impl Mem) -> u32 {
        SEP_TRANS.get(mem)
    }

    /// `MsSEPtransCheck` (`0x007a04c0`): -1 while a transfer is pending,
    /// else 0.
    pub fn ms_sep_trans_check(mem: &impl Mem) -> i32 {
        -i32::from(SEP_TRANS.get(mem) != 0)
    }

    /// `MsDebugGetFieldNum` (`0x007c5f50`): `high << 16` + low.
    pub fn ms_debug_get_field_num(mem: &impl Mem) -> u32 {
        let [hi, lo] = DEBUG_FIELD.get(mem);
        (hi << 16).wrapping_add(lo)
    }

    fn status_byte(mem: &impl Mem, table: Global<u32>, i: i32) -> u32 {
        let at = table
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(4))
            .wrapping_add(3);
        u32::from(mem.read::<u8>(at))
    }

    /// `MsCheckStat(i)` (`0x0079e310`): byte 3 of status `i` (not checked).
    pub fn ms_check_stat(mem: &impl Mem, i: i32) -> u32 {
        status_byte(mem, STATUS_TABLE, i)
    }

    /// `MsCheckStat2(i)` (`0x0079e320`): byte 3 of entry `i` of the second
    /// table.
    pub fn ms_check_stat2(mem: &impl Mem, i: i32) -> u32 {
        status_byte(mem, STATUS_TABLE2, i)
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
