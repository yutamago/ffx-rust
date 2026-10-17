//! Pieces of the damage step (`docs/rewrite-plan.md`, M2):
//! `MsBattleProcess_ChrDamage` (`0x007911e0`) ends each actor's command
//! (`+0xdf0`), and these are what it and `MsSetDamage` lean on: when a
//! command may end, Poison at a turn's end, running away, the camera and
//! focus resets, the prayer effects.
//!
//! The focus (`Sg_EnFocus`) is the graphics module's and the prayer
//! effects vfx's: those go to the game's code through `call`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;
    use crate::exec::bridge::Call;
    use crate::target::bridge::{Dm, Host};

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// `Sg_EnFocus` (graphics).
    pub const SG_EN_FOCUS: u32 = 0x0082_cb60;
    /// `op_et_prayeff_send_read_through` (vfx).
    pub const OP_ET_PRAYEFF_SEND_READ_THROUGH: u32 = 0x007f_c920;
    /// A dword `0x00791690` waits on (not 0: the command does not end).
    pub const COMMAND_HOLD2: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0112_c8f8);

    /// The turn-end actor (`MsDeleteInputCue`'s), sign-extended as the
    /// originals compare it.
    fn turn_end_actor(mem: &impl Mem) -> i32 {
        i32::from(crate::action::bridge::TURN_END_ACTOR.get(mem).cast_signed())
    }

    /// `MsScreenFocusResetAll()` (`0x007bbda0`): the three screens' focus
    /// gone (`MsScreenFocusReset`) and the focus off (`Sg_EnFocus(0)`).
    pub fn ms_screen_focus_reset_all<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        crate::misc::bridge::ms_screen_focus_reset(mem);
        let f = code(mem, SG_EN_FOCUS);
        call(mem, f, &[0]);
    }

    /// `0x00792ee0()` (`MsCheckPartyDying`, name ours): -1 while a party
    /// member's death still plays (`+0x436`), else 0.
    pub fn ms_check_party_dying(mem: &impl Mem) -> i32 {
        let party = crate::menu::bridge::PARTY.get(mem);
        for &p in &party[..3] {
            if p != 0xff && byte(mem, ms_get_chr(mem, u32::from(p)), 0x436) != 0 {
                return -1;
            }
        }
        0
    }

    /// `0x00791780(i, actor)` (`MsCheckRestMotion`, name ours): 1 when the
    /// actor goes back to its standing motion after its command: hurt
    /// (`+0x41c`) with `+0x41d`, no command entry left (`+0xde7`), not busy
    /// (`+0x508`, `+0x432`) and not the turn-end actor; else 0.
    pub fn ms_check_rest_motion(mem: &impl Mem, i: u32, actor: u32) -> u32 {
        u32::from(
            byte(mem, actor, 0x41c) != 0
                && byte(mem, actor, 0xde7) == 0
                && byte(mem, actor, 0x508) == 0
                && byte(mem, actor, 0x432) == 0
                && byte(mem, actor, 0x41d) != 0
                && turn_end_actor(mem) != i.cast_signed(),
        )
    }

    /// `MsEffectCameraEnd(keep)` (`0x00787760`): the effect cameras
    /// (`minor::EFFECT_CAMERA_BUF`) freed (`MsCameraFree`); unless `keep`,
    /// in battle, a command's camera started (`parse::COMMAND_STARTED`)
    /// gives way to camera 0x1c.
    pub fn ms_effect_camera_end<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, keep: u32) {
        use crate::parse::bridge::COMMAND_STARTED;
        let at = crate::minor::bridge::EFFECT_CAMERA_BUF.ptr(mem).addr;
        for k in 0..8 {
            let p = at + 4 * k;
            let id: u32 = mem.read(p);
            if id != 0 {
                crate::camera::bridge::ms_camera_free(mem, id);
                mem.write(p, 0u32);
            }
        }
        if keep == 0 && crate::state::bridge::STATUS.get(mem) != 0 && COMMAND_STARTED.get(mem) != 0
        {
            COMMAND_STARTED.set(mem, 0);
            crate::atel::bridge::ms_atel_request_camera(mem, s, 0, 0, 0x1c, 0xffff, 1, 1, 0, 0);
        }
    }

    /// `0x0078ee80(a, b, i)` (`MsCheckHitPending`, name ours): 1 when either
    /// of actor `i`'s two records at `+0x774` and `+0xa4c` (count `+0`
    /// below `+1`) is for `(a, b)` (`+2`, `+3`); else 0.
    pub fn ms_check_hit_pending(mem: &impl Mem, a: u32, b: u32, i: u32) -> u32 {
        let p = ms_get_chr(mem, i);
        let hit = |o: u32| {
            u32::from(byte(mem, p, o + 2)) == a
                && u32::from(byte(mem, p, o + 3)) == b
                && byte(mem, p, o) < byte(mem, p, o + 1)
        };
        u32::from(hit(0x774) || hit(0xa4c))
    }

    /// `MsPoisonProcess(i, actor)` (`0x007afab0`): `+0xdf8` cleared; for the
    /// turn-end actor (cleared to 0xff after) Poison bites: `+0xdd0`
    /// cleared, its moves reset (`MsResetDirMove`), `+0x41c` set, `+0x5ba`
    /// percent of its max HP lost (unsigned; number style 0x82), its
    /// action 8, its pose and automatic statuses redone, `+0xded` itself
    /// and its hit motion 8 unless that killed it
    /// ([`crate::death::bridge::ms_damage_check_death`]).
    pub fn ms_poison_process<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
    ) {
        use crate::action::bridge::TURN_END_ACTOR;
        mem.write(actor.wrapping_add(0xdf8), 0u8);
        if i.cast_signed() == turn_end_actor(mem) {
            mem.write(actor.wrapping_add(0xdd0), 0u8);
            crate::misc::bridge::ms_reset_dir_move(mem, actor);
            let max: u32 = mem.read(actor.wrapping_add(0x594));
            let n = (u32::from(byte(mem, actor, 0x5ba)).wrapping_mul(max) / 100).cast_signed();
            mem.write(actor.wrapping_add(0x41c), 1u8);
            crate::input::bridge::ms_sub_hp(mem, i, actor, n, n, 0, 1, 0x82);
            crate::action::bridge::ms_action_request(mem, host, s, i, i, 8, 0, 1, 0);
            crate::input::bridge::ms_set_chr_weak(mem, i, u32::MAX);
            crate::status::bridge::ms_check_auto_status(mem, &mut *s.call, &mut *s.off, i, actor);
            mem.write(actor.wrapping_add(0xded), i.to_le_bytes()[0]);
            if crate::death::bridge::ms_damage_check_death(mem, host, s, i, i, 0, 1) == 0 {
                crate::status::bridge::ms_damage_set_motion(mem, host, s, i, 8, 1);
            }
        }
        TURN_END_ACTOR.set(mem, 0xff);
    }

    /// `0x00791690(actor)` (`MsCheckCommandEnd`, name ours): whether the
    /// actor's command may end: with numbers of kind 1 showing, `+0xe13` or
    /// [`COMMAND_HOLD2`] it waits (`+0xdf0` 5): 0. Commands that show a
    /// title (0x3056, 0x3057, 0x312c, 0x604d, 0x60f1, 0x3022, 0x3055,
    /// 0x3105-0x3108, 0x403c, 0x60bd, 0x4086) wait while it shows
    /// (`TITLE_WINDOW` above 0, 0x2d for 0x3103): 0. Else `+0xdf0` 1: 1.
    pub fn ms_check_command_end(mem: &mut impl Mem, actor: u32) -> u32 {
        if crate::minor::bridge::NUMBERS_KIND1.get(mem) != 0
            || byte(mem, actor, 0xe13) != 0
            || COMMAND_HOLD2.get(mem) != 0
        {
            mem.write(actor.wrapping_add(0xdf0), 5u8);
            return 0;
        }
        let cmd = crate::parse::bridge::PARSED_COMMAND.get(mem);
        let titled = [
            0x3056, 0x3057, 0x312c, 0x604d, 0x60f1, 0x3022, 0x3055, 0x3105, 0x3107, 0x3108, 0x3106,
            0x403c, 0x60bd, 0x4086,
        ];
        let title = crate::state::bridge::TITLE_WINDOW.get(mem)[0];
        let shows = if titled.contains(&cmd) {
            title > 0
        } else {
            cmd == 0x3103 && title > 0x2d
        };
        if shows {
            return 0;
        }
        mem.write(actor.wrapping_add(0xdf0), 1u8);
        1
    }

    /// `MsSetEscapeFlag(i, actor, away)` (`0x0078de40`): the actor has run
    /// away (`+0xdcd` set, `+0xdc8` cleared) or, not `away`, is back
    /// (`+0xdcd` cleared, `+0xdc8` set, shown:
    /// [`crate::visible::bridge::ms_set_chr_visible_appear`]).
    pub fn ms_set_escape_flag<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        actor: u32,
        away: u32,
    ) {
        let away = away != 0;
        mem.write(actor.wrapping_add(0xdcd), u8::from(away));
        mem.write(actor.wrapping_add(0xdc8), u8::from(!away));
        if !away {
            crate::visible::bridge::ms_set_chr_visible_appear(mem, call, i, 1);
        }
    }

    /// `MsSetEscapeFlagAll(away)` (`0x0078de80`): [`ms_set_escape_flag`]
    /// for actors 0-0x11.
    pub fn ms_set_escape_flag_all<M: Mem>(mem: &mut M, call: Call<'_, M>, away: u32) {
        for i in 0..0x12 {
            let a = ms_get_chr(mem, i);
            ms_set_escape_flag(mem, &mut *call, i, a, away);
        }
    }

    /// `MsMagicEffectEnd(i)` (`0x0079e830`): actor `i`'s prayer effect
    /// (`+0xea0`) ends (`op_et_prayeff_send_read_through`, cleared): -1; an
    /// actor of kind 8 (`+0xdea`) ends actors 0x10's and 0x11's first. 0
    /// otherwise, and for 0xff.
    pub fn ms_magic_effect_end<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32) -> i32 {
        if i == 0xff {
            return 0;
        }
        let a = ms_get_chr(mem, i);
        if byte(mem, a, 0xdea) == 8 {
            ms_magic_effect_end(mem, &mut *call, 0x10);
            ms_magic_effect_end(mem, &mut *call, 0x11);
        }
        let e: u32 = mem.read(a.wrapping_add(0xea0));
        if e == 0 {
            return 0;
        }
        let f = code(mem, OP_ET_PRAYEFF_SEND_READ_THROUGH);
        call(mem, f, &[e]);
        mem.write(a.wrapping_add(0xea0), 0u32);
        -1
    }
}
