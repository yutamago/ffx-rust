//! The battle loop's per-frame pieces (`docs/rewrite-plan.md`, M2):
//! what `MsBattleProcess` (`0x00790c10`) checks and runs each frame
//! besides the command cue ([`crate::parse`]).
//!
//! Sound, the CD library and the voice status are other modules'; those go
//! to the game's code through the caller's `call`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::exec::bridge::Call;

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// Counts down each frame; the battle waits while it runs.
    pub const PAUSE: Global<u8> = Global::new(0x0112_c9d0);
    /// Set while an actor plays a hit motion (`MsCheckChrHitMotionAll`).
    pub const HIT_MOTION: Global<u8> = Global::new(0x0112_c9ea);
    /// Not 0: a soft reset waits.
    pub const RESET_WAIT: Global<u32> = Global::new(0x0112_c024);
    /// Not 0: a soft reset waits.
    pub const RESET_WAIT2: Global<u8> = Global::new(0x0112_c9d6);

    /// `SndGetNowPlayMusic`, `SndMusicStop`, `SndMusicClear` (sound).
    pub const SND_GET_NOW_PLAY_MUSIC: u32 = 0x0088_63f0;
    pub const SND_MUSIC_STOP: u32 = 0x0088_6a60;
    pub const SND_MUSIC_CLEAR: u32 = 0x0088_6800;
    /// The voice status's sound and CD calls (`msVoiceCheckStanby`).
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const TK_SND_REPORT_STATUS: u32 = 0x0088_7b30;
    pub const SND_IS_READ_STATUS: u32 = 0x0088_6650;
    pub const SND_GET_STREAM_STATE: u32 = 0x0088_64b0;
    pub const SND_IS_READ_SHOUT_VOICE_STATUS: u32 = 0x0088_6640;

    /// `MsNowMusicStop()` (`0x007a00b0`): the music playing
    /// (`SndGetNowPlayMusic`) stopped and cleared; what `SndMusicClear`
    /// returns.
    pub fn ms_now_music_stop<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        let f = code(mem, SND_GET_NOW_PLAY_MUSIC);
        let m = call(mem, f, &[]);
        let f = code(mem, SND_MUSIC_STOP);
        call(mem, f, &[m]);
        let f = code(mem, SND_MUSIC_CLEAR);
        call(mem, f, &[m])
    }

    /// `MsResetNormalCalc()` (`0x00795c40`): every actor with `+0x3d` set
    /// has it cleared and its normals recalculated (`Ch_SetCalcNorm2`);
    /// how many.
    pub fn ms_reset_normal_calc(mem: &mut impl Mem) -> u32 {
        let mut n = 0;
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if mem.read::<u8>(a.wrapping_add(0x3d)) != 0 {
                n += 1;
                mem.write(a.wrapping_add(0x3d), 0u8);
                ffx_chara::parts::bridge::ch_set_calc_norm2_holder(mem, a);
            }
        }
        n
    }

    /// `0x00791190()` (`MsBattleCanProgress`, name ours): -1 when nothing
    /// holds the battle (no pause, hit motion, turn ending, message, camera
    /// state, overdrive learnt, stone effect or command hold), else 0.
    pub fn ms_battle_can_progress(mem: &impl Mem) -> i32 {
        let quiet = PAUSE.get(mem) == 0
            && HIT_MOTION.get(mem) == 0
            && crate::action::bridge::TURN_END_ACTOR.get(mem).cast_signed() < 0
            && crate::state::bridge::MESSAGE_CUE_COUNT.get(mem) == 0
            && crate::atel::bridge::CAMERA_STATE.get(mem) == 0
            && crate::flow::bridge::LIMIT_LEARNT.get(mem) == 0
            && crate::state::bridge::STONE_EFFECT.get(mem)[0] == 0
            && crate::parse::bridge::COMMAND_HOLD.get(mem) == 0;
        -i32::from(quiet)
    }

    /// `0x00791760()` (`MsSetDeathAll`, name ours): `MsSetDeath(i, 1)` for
    /// every actor (the battle's own, through `call`).
    pub fn ms_set_death_all<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let f = code(mem, 0x0078_dc30);
        for i in 0..0x1f {
            call(mem, f, &[i, 1]);
        }
    }

    /// `msVoiceCheckStanby(any)` (`0x007b2c10`): -1 while a voice is not
    /// ready: the CD library busy (its `+8` with 1), the sound report 1
    /// (`TkSndReportStatus`), a sound read running (`SndIsReadStatus`,
    /// with `any`), a stream or a shout voice reading; else 0.
    pub fn ms_voice_check_stanby<M: Mem>(mem: &mut M, call: Call<'_, M>, any: u32) -> i32 {
        let f = code(mem, GET_CD);
        let cd = call(mem, f, &[]);
        let f: u32 = mem.read(cd.wrapping_add(8));
        if call(mem, f, &[1]) != 0 {
            return -1;
        }
        let f = code(mem, TK_SND_REPORT_STATUS);
        if call(mem, f, &[]) == 1 {
            return -1;
        }
        let f = code(mem, SND_IS_READ_STATUS);
        if call(mem, f, &[]) != 0 && any != 0 {
            return -1;
        }
        let f = code(mem, SND_GET_STREAM_STATE);
        if call(mem, f, &[]) != 0 {
            return -1;
        }
        let f = code(mem, SND_IS_READ_SHOUT_VOICE_STATUS);
        if call(mem, f, &[]) != 0 {
            return -1;
        }
        0
    }

    /// `0x00792e80()` (`MsCheckSoftResetOk`, name ours): -1 when a soft
    /// reset may end the battle: no read queued, no sound effect in
    /// transfer, the voices ready ([`ms_voice_check_stanby`]), the battle
    /// running (status 1), no effect, nothing waiting, no stone effect;
    /// else 0.
    pub fn ms_check_soft_reset_ok<M: Mem>(mem: &mut M, call: Call<'_, M>) -> i32 {
        use crate::state::bridge as st;
        let ok = st::ms_btl_read_cue_level(mem) == 0
            && st::ms_sep_trans_amount(mem) == 0
            && ms_voice_check_stanby(mem, call, 1) == 0
            && st::STATUS.get(mem) == 1
            && crate::minor::bridge::ms_effect_check(mem) == 0
            && RESET_WAIT.get(mem) == 0
            && RESET_WAIT2.get(mem) == 0
            && st::ms_stone_effect_check(mem) == 0;
        -i32::from(ok)
    }
}
