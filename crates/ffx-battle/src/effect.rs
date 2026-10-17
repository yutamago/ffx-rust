//! A command's effect, from its start to its end (`docs/rewrite-plan.md`,
//! M2): `MsEffectStart` (`0x00788480`) runs each actor's effect state
//! (`+0xdfb`) once a frame from `MsBattleProcess`, and these are its
//! steps: the overdrive start (`MsEffectLimitCheck`), the effect's camera
//! script (`MsAtelCamRegist`), the reflect statuses as the effect lands,
//! the wait for the actor's motion, the end.
//!
//! Other modules' functions (script, the system's windows and magic files,
//! the graphics filters, sound) go to the game's code through `call`/`s.call`.
//! `FlushCache` is empty in this build (`ret`); the port leaves it out.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, the script calls
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;
    use crate::exec::bridge::Call;
    use crate::target::bridge::{Dm, Host};

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn half(mem: &impl Mem, p: u32, o: u32) -> u32 {
        u32::from(mem.read::<u16>(p.wrapping_add(o)))
    }

    fn word(mem: &impl Mem, p: u32, o: u32) -> u32 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        call(mem, f, args)
    }

    /// `TOBtlInitTidusLimitWindow` (system).
    pub const TO_BTL_INIT_TIDUS_LIMIT_WINDOW: u32 = 0x0089_8180;
    /// `TOBtlOpenTidusLimitWindow(kind)` (system).
    pub const TO_BTL_OPEN_TIDUS_LIMIT_WINDOW: u32 = 0x0089_8ca0;
    /// `TOBtlInitAuronLimitWindow` (system).
    pub const TO_BTL_INIT_AURON_LIMIT_WINDOW: u32 = 0x0089_7970;
    /// `TOBtlOpenAuronLimitWindow(kind)` (system).
    pub const TO_BTL_OPEN_AURON_LIMIT_WINDOW: u32 = 0x0089_8ad0;
    /// `TOBtlInitLuluLimitWindow` (system).
    pub const TO_BTL_INIT_LULU_LIMIT_WINDOW: u32 = 0x0089_7f70;
    /// `TOBtlOpenLuluLimitWindow(command)` (system).
    pub const TO_BTL_OPEN_LULU_LIMIT_WINDOW: u32 = 0x0089_8bf0;
    /// `Sg_FilterSub(r, g, b, a)` (graphics).
    pub const SG_FILTER_SUB: u32 = 0x0082_cc80;
    /// `Sg_FilterReset` (graphics).
    pub const SG_FILTER_RESET: u32 = 0x0082_cc70;
    /// `Sg_EffectRoutineEnable(on)` (graphics).
    pub const SG_EFFECT_ROUTINE_ENABLE: u32 = 0x0082_0380;
    /// `AtelInitInternalActorWork(work)` (script).
    pub const ATEL_INIT_INTERNAL_ACTOR_WORK: u32 = 0x0086_d260;
    /// `AtelGetActorBufferSize(script)` (script).
    pub const ATEL_GET_ACTOR_BUFFER_SIZE: u32 = 0x0086_a290;
    /// `AtelGetMyBufferSize(script)` (script).
    pub const ATEL_GET_MY_BUFFER_SIZE: u32 = 0x0086_c0c0;
    /// `AtelBattleSetUp(scripts, words, n, works, my, actors, kind, f)`
    /// (script).
    pub const ATEL_BATTLE_SET_UP: u32 = 0x0087_56d0;
    /// `user_malloc(size)` (system).
    pub const USER_MALLOC: u32 = 0x0068_7020;

    /// The camera script's work start for the managers (`AtelBattleSetUp`'s
    /// fourth argument).
    pub const ATEL_WORKS: Global<u8> = Global::new(0x0113_4570);
    /// Manager 2's (the camera script's) actor and own buffers: fixed, 0x800
    /// bytes from here.
    pub const ATEL_CAM_BUFFER: Global<u8> = Global::new(0x0113_5570);
    /// The callbacks `AtelBattleSetUp` gets for managers 0, 1 and 2.
    pub const ATEL_SET_UP_CALLBACKS: [u32; 3] = [0x0079_72e0, 0x0079_73b0, 0x0079_7350];
    /// `MsBattleError`'s message when manager 2's buffers pass 0x800 bytes.
    pub const ATEL_CAM_BUFFER_ERROR: u32 = 0x00b5_4c80;

    /// Set once a script camera request is running (`0x007981e0`).
    pub const CAMERA_ONCE: Global<u8> = Global::new(0x0112_a8e5);
    /// The debug menu's effect hold (`0x007c6e20`): no effect starts.
    pub const DEBUG_EFFECT_HOLD: Global<u8> = Global::new(0x0112_a91c);
    /// The overdrive start's darkening (0-6 steps of 8: `Sg_FilterSub`).
    pub const LIMIT_FADE: Global<i8> = Global::new(0x0112_ca07);
    /// Models whose motion stops (`ms_motion_trans` 0) during magic
    /// 0x1b2 or 0x262: a count, then the models from `0x0113339c`.
    pub const MOTION_STOP_CHRS: Global<i32> = Global::new(0x0113_3398);

    /// `ms_motion_trans(actor, kind)` (`0x007a8c80`): the motion an actor
    /// plays for `kind` (1 its `+0x3f1`; 2-5 by its weapon kind `+0x408`,
    /// 12 and 13 as 4 and 5 by `+0x409`; 6 0x5d; 0x82 0x2a when `+0x3f0`
    /// is 0x29; else `kind` itself), 0 while magic 0x1b2 or 0x262 runs
    /// for one of [`MOTION_STOP_CHRS`] (`+0x3f3` set then).
    pub fn ms_motion_trans<M: Mem>(mem: &mut M, call: Call<'_, M>, actor: u32, kind: u32) -> u32 {
        let (w, w2) = (byte(mem, actor, 0x408), byte(mem, actor, 0x409));
        let by = |w: u8, t: [u32; 6]| match w {
            1..=5 => t[usize::from(w)],
            _ => t[0],
        };
        let mut r = match kind {
            1 => u32::from(byte(mem, actor, 0x3f1)),
            2 => by(w, [0x58, u32::MAX, 0xb0, 0xb8, 0xc0, 0xc8]),
            3 => by(w, [0x59, 0x5c, 0xb1, 0xb9, 0xc1, 0xc9]),
            4 => by(w, [0x5a, 0x5a, 0xb2, 0xba, 0xc2, 0xca]),
            0xc => by(w2, [0x5a, 0x5a, 0xb2, 0xba, 0xc2, 0xca]),
            5 => by(w, [0x5b, 0x5b, 0xb3, 0xbb, 0xc3, 0xcb]),
            0xd => by(w2, [0x5b, 0x5b, 0xb3, 0xbb, 0xc3, 0xcb]),
            6 => 0x5d,
            0x82 if byte(mem, actor, 0x3f0) == 0x29 => 0x2a,
            _ => kind,
        };
        if MOTION_STOP_CHRS.get(mem) > 0 {
            let get = crate::process::bridge::MAGIC_FILE_GET_MAGIC_ID;
            let hit = game(mem, &mut *call, get, &[]) == 0x1b2
                || game(mem, &mut *call, get, &[]) == 0x262;
            if hit {
                let at = MOTION_STOP_CHRS.ptr(mem).addr;
                let mut n = MOTION_STOP_CHRS.get(mem);
                let mut k = 1;
                while k <= n {
                    let chr: u32 = mem.read(at.wrapping_add(k.cast_unsigned().wrapping_mul(4)));
                    if chr == word(mem, actor, 0) {
                        mem.write(actor.wrapping_add(0x3f3), 1u8);
                        n = MOTION_STOP_CHRS.get(mem);
                        r = 0;
                    }
                    k += 1;
                }
            }
        }
        r
    }

    /// `0x00787650(i, actor)` (`MsEffectMotionDone`, name ours): 1 once the
    /// actor is still (`+0x508`, `+0x436` clear) and its motion (`+0x3f0`)
    /// is none of its attack ones (kinds 12, 13, 2-5 of
    /// [`ms_motion_trans`]).
    pub fn ms_effect_motion_done<M: Mem>(mem: &mut M, call: Call<'_, M>, actor: u32) -> u32 {
        let now = u32::from(byte(mem, actor, 0x3f0));
        if byte(mem, actor, 0x508) != 0 || byte(mem, actor, 0x436) != 0 {
            return 0;
        }
        for kind in [0xc, 0xd, 2, 3, 4, 5] {
            if ms_motion_trans(mem, &mut *call, actor, kind) == now {
                return 0;
            }
        }
        1
    }

    /// `0x00787d70(i, actor)` (`MsEffectBackToIdle`, name ours): the actor
    /// asks its turn's end (`MsActionRequest(i, 0xff, 7, 0, 2)`), goes back
    /// to its place, the command title closes, its idle motion `+0xe00`
    /// plays and the magic camera `+0xe04`/`+0xe08` of its command
    /// `+0xf5c` runs (its result returned).
    pub fn ms_effect_back_to_idle<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
    ) -> i32 {
        crate::action::bridge::ms_action_request(mem, host, s, i, 0xff, 7, 0, 2, 0);
        crate::actor::bridge::ms_pop_btl_pos(mem, actor);
        let title = crate::parse::bridge::PARSED_COMMAND.get(mem);
        crate::exec::bridge::ms_menu_open_title_window(mem, &mut *s.call, title, 0x3c);
        let motion = i32::from(byte(mem, actor, 0xe00));
        mem.write(actor.wrapping_add(0x3f2), 0xffu8);
        crate::atel::bridge::ms_set_motion(mem, s, i, motion, 1, 1, 1, 0, 0);
        let (id, a, b) = (
            half(mem, actor, 0xf5c),
            half(mem, actor, 0xe04),
            half(mem, actor, 0xe08),
        );
        crate::atel::bridge::ms_atel_request_magic_camera(mem, s, 0, i, id, a, b, 1, 1, 0, 0)
    }

    /// `0x00788ef0(actor)` (`MsEffectCamRegist`, name ours): the effect's
    /// camera script ([`ms_atel_cam_regist`]): the magic file's
    /// ([`crate::parse::bridge::EFFECT_READ_STAT`]) or else the command's
    /// (`+0xf8c`'s `+0x1c`).
    pub fn ms_effect_cam_regist<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, actor: u32) -> u32 {
        let cam = crate::parse::bridge::EFFECT_READ_STAT.get(mem);
        let e = word(mem, actor, 0xf8c);
        if cam != 0 {
            return ms_atel_cam_regist(mem, s, cam);
        }
        if e == 0 {
            return 0;
        }
        let c = word(mem, e, 0x1c);
        if c == 0 {
            return c;
        }
        ms_atel_cam_regist(mem, s, c)
    }

    /// `0x00788f30(i, actor)` (`MsEffectCameraEnd2`, name ours): with an
    /// effect (`+0xf8c`) the magic camera `+0xe06`/`+0xe0a` of the command
    /// `+0xf5c` (its slot to `+0xdfe`), then the effect camera mode 1
    /// (`MsEffectCameraCall(-1, 1)`, its result returned).
    pub fn ms_effect_camera_end2<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        i: u32,
        actor: u32,
    ) -> u32 {
        if word(mem, actor, 0xf8c) != 0 {
            let (id, a, b) = (
                half(mem, actor, 0xf5c),
                half(mem, actor, 0xe06),
                half(mem, actor, 0xe0a),
            );
            let slot = crate::atel::bridge::ms_atel_request_magic_camera(
                mem, s, 0, i, id, a, b, 1, 1, 0, 0,
            );
            mem.write(actor.wrapping_add(0xdfe), slot.to_le_bytes()[0]);
        }
        crate::minor::bridge::ms_effect_camera_call(mem, -1, 1)
    }

    /// `0x00789030(i, actor)` (`MsEffectReflect`, name ours): once per
    /// effect (`+0xe0d` set at the end), while the actor is not moving
    /// (`+0x432`) or is (`+0x429`): the targets of its cue entry's hit
    /// (`MsGetChrCommand(i)`, the hit `+2`'s target bits at `+0x14`)
    /// get, the first time (`+5` 0 → 1), their head target reset, the
    /// overdrive status check, the reflect effect and sound 0x3b; then
    /// (`+5` 1 → 2) once none of them still shows it (`+0xea4`), the end:
    /// with no targets at once. The end runs [`ms_effect_camera_end2`]
    /// unless `+0xe14`.
    pub fn ms_effect_reflect<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
    ) {
        if byte(mem, actor, 0xe0d) != 0 {
            return;
        }
        let cue = crate::actor::bridge::ms_get_chr_command(mem, i);
        let hit = u32::from(byte(mem, cue, 2));
        let targets = word(mem, cue, 0x14 + hit * 0x10).cast_signed();
        if byte(mem, actor, 0x432) != 0 && byte(mem, actor, 0x429) == 0 {
            return;
        }
        let end = |mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>| {
            mem.write(actor.wrapping_add(0xe0d), 1u8);
            if byte(mem, actor, 0xe14) == 0 {
                ms_effect_camera_end2(mem, s, i, actor);
            }
        };
        if targets == 0 {
            end(mem, s);
            return;
        }
        match byte(mem, cue, 5) {
            0 => {
                for j in 0..0x1f {
                    if (targets >> j) & 1 == 0 {
                        continue;
                    }
                    let a = ms_get_chr(mem, j);
                    crate::atel::bridge::ms_reset_head_target(mem, &mut *s.call, a);
                    crate::after_damage::bridge::ms_limit_type_status_check(
                        mem, i, actor, j, a, 0, 2,
                    );
                    ms_status_effect_reflect(mem, &mut *s.call, j);
                    crate::after_damage::bridge::ms_reg_se_play2(
                        mem,
                        host.trig,
                        &mut *s.call,
                        j,
                        0x3b,
                    );
                }
                mem.write(cue.wrapping_add(5), 1u8);
            }
            1 => {
                let shown = (0..0x1f)
                    .any(|j| (targets >> j) & 1 != 0 && word(mem, ms_get_chr(mem, j), 0xea4) != 0);
                if !shown {
                    mem.write(cue.wrapping_add(5), 2u8);
                    end(mem, s);
                }
            }
            _ => {}
        }
    }

    /// `MsStatusEffectReflect(i)` (`0x0079f160`): actor `i`'s reflect
    /// effect (`MsGetEffID(+0xe26, 9, i)`, the counter on), started
    /// (`op_et_eff(owner, 0, 0xf, id)`) and kept at `+0xea4` unless one is
    /// there already; the ID, else 0.
    pub fn ms_status_effect_reflect<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32) -> u32 {
        let a = ms_get_chr(mem, i);
        if word(mem, a, 0xea4) != 0 {
            return 0;
        }
        let n = mem.read::<u16>(a.wrapping_add(0xe26));
        let id = crate::util::bridge::ms_get_eff_id(u32::from(n), 9, i);
        mem.write(a.wrapping_add(0xe26), n.wrapping_add(1));
        let owner = crate::status::bridge::EFFECT_OWNER.get(mem);
        game(
            mem,
            call,
            crate::death::bridge::OP_ET_EFF,
            &[owner, 0, 0xf, id],
        );
        mem.write(a.wrapping_add(0xea4), id);
        id
    }

    /// `0x00788f90(actor, x, motion, flag)` (`MsEffectEnd`, name ours): the
    /// actor back at its place, `+0x405` = `flag`; with `motion` 1 its
    /// end motion `+0xe01` (none for 0xff), else 0x33, plays
    /// (`MsSetMotion(chr, m, 1, x, 1)`); the reflect step
    /// ([`ms_effect_reflect`]), `+0x414` set unless `+0x413`, the camera's
    /// end with `+0xe14`, and its sound `+0xf76` fades over 0x14 frames
    /// (`MsRegSEfade`'s result returned).
    pub fn ms_effect_end<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        actor: u32,
        x: u32,
        motion: u32,
        flag: u32,
    ) -> i32 {
        let i = u32::from(byte(mem, actor, 0xc));
        crate::actor::bridge::ms_pop_btl_pos(mem, actor);
        let end = byte(mem, actor, 0xe01);
        mem.write(actor.wrapping_add(0x405), flag.to_le_bytes()[0]);
        let m = if motion == 1 {
            (end != 0xff).then_some(i32::from(end))
        } else {
            Some(0x33)
        };
        if let Some(m) = m {
            crate::atel::bridge::ms_set_motion(mem, s, i, m, 1, x.to_le_bytes()[0], 1, 0, 0);
        }
        ms_effect_reflect(mem, host, s, i, actor);
        if byte(mem, actor, 0x413) == 0 {
            mem.write(actor.wrapping_add(0x414), 1u8);
        }
        if byte(mem, actor, 0xe14) != 0 {
            ms_effect_camera_end2(mem, s, i, actor);
        }
        let se = half(mem, actor, 0xf76);
        crate::damage_end::bridge::ms_reg_se_fade(mem, i, se, 0x14)
    }

    /// `MsEffectReady(id, x)` (`0x00788200`): an effect reports itself
    /// ready: for an actor's (`id >> 8` 2, the actor in the low byte)
    /// whose state is 4 (waiting), its end ([`ms_effect_end`] with motion
    /// for `+0x413` clear, `x`), effect camera `id` mode 0, state 5; for
    /// the event effect (0x80) state 4 → 5. EAX as the original leaves it.
    pub fn ms_effect_ready<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        id: u32,
        x: u32,
    ) -> u32 {
        let kind = (id >> 8) & 0xff;
        match kind {
            2 => {
                let a = ms_get_chr(mem, id & 0xff);
                if byte(mem, a, 0xdfb) != 4 {
                    return a;
                }
                let motion = u32::from(byte(mem, a, 0x413) == 0);
                ms_effect_end(mem, host, s, a, motion, x, 0);
                let r = crate::minor::bridge::ms_effect_camera_call(mem, id.cast_signed(), 0);
                mem.write(a.wrapping_add(0xdfb), 5u8);
                r
            }
            0x80 => {
                let stat = crate::state::bridge::EVENT_EFFECT_STAT;
                if stat.get(mem) == 4 {
                    stat.set(mem, 5);
                }
                0
            }
            _ => kind.wrapping_sub(0x80),
        }
    }

    /// `MsEffectLimitStart(i)` (`0x007afd30`): actor `i`'s overdrive
    /// starts (`+0xd24` 1): Tidus's, Auron's and Lulu's open their
    /// windows (`TOBtl*LimitWindow`; Lulu's gets the command ID
    /// `MsGetCommand` found for `+0xf5c`, the others its `+0x58`), then
    /// the magic camera 0x87. 0.
    pub fn ms_effect_limit_start<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, i: u32) -> u32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        mem.write(a.wrapping_add(0xd24), 1u16);
        let given = half(mem, a, 0xf5c);
        let (c, id) = crate::command::bridge::get_command(mem, u32::MAX, 0, given);
        let windows = match i {
            0 => Some((
                TO_BTL_INIT_TIDUS_LIMIT_WINDOW,
                TO_BTL_OPEN_TIDUS_LIMIT_WINDOW,
                false,
            )),
            2 => Some((
                TO_BTL_INIT_AURON_LIMIT_WINDOW,
                TO_BTL_OPEN_AURON_LIMIT_WINDOW,
                false,
            )),
            5 => Some((
                TO_BTL_INIT_LULU_LIMIT_WINDOW,
                TO_BTL_OPEN_LULU_LIMIT_WINDOW,
                true,
            )),
            _ => None,
        };
        if let Some((init, open, lulu)) = windows {
            game(mem, &mut *s.call, init, &[]);
            let v = if lulu {
                id
            } else {
                u32::from(byte(mem, c, 0x58))
            };
            game(mem, &mut *s.call, open, &[v]);
        }
        crate::atel::bridge::ms_atel_request_magic_camera(
            mem, s, 0, i, 0, 0x87, 0xffff, 0, 1, 0, 0,
        );
        0
    }

    /// `MsEffectLimitCheck(i, actor)` (`0x007afc90`): the effect state
    /// after the command's start: 0xc for an overdrive command (its
    /// `+0x58` 0x20, 0x25, 0x30 or 0x40: the darkening reset and
    /// [`ms_effect_limit_start`]), else 0xb.
    pub fn ms_effect_limit_check<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        i: u32,
        actor: u32,
    ) -> u32 {
        let i = i & 0xff;
        let given = half(mem, actor, 0xf5c);
        let (c, _) = crate::command::bridge::get_command(mem, u32::MAX, 0, given);
        if matches!(byte(mem, c, 0x58), 0x20 | 0x25 | 0x30 | 0x40) {
            LIMIT_FADE.set(mem, 0);
            ms_effect_limit_start(mem, s, i);
            return 0xc;
        }
        0xb
    }

    /// `0x00797490(kind, i, n)` (`MsAtelActorWorkInit`): handler `n` of
    /// script owner `kind`/`i` gets its work reset
    /// (`AtelInitInternalActorWork(byte + base)`), under that manager's
    /// control (`AtelChangeCurrentCtrl`, then the previous back).
    pub fn ms_atel_actor_work_init<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        kind: u32,
        i: u32,
        n: u32,
    ) -> u32 {
        let prev = game(
            mem,
            &mut *call,
            crate::atel::bridge::ATEL_CHANGE_CURRENT_CTRL,
            &[kind],
        );
        let (h, base) = crate::atel::bridge::atel_get_handler(mem, kind, i, n.cast_signed());
        if h != 0 {
            let w = u32::from(mem.read::<u8>(h)).wrapping_add(base.unwrap_or(0));
            game(mem, &mut *call, ATEL_INIT_INTERNAL_ACTOR_WORK, &[w]);
        }
        game(
            mem,
            call,
            crate::atel::bridge::ATEL_SET_CURRENT_CTRL,
            &[prev],
        )
    }

    /// `MsAtelSetUp(kind)` (`0x00798390`): manager `kind`'s scripts' buffer
    /// sizes summed (actor sizes at `+8`, own at `+0x10`); manager 2's
    /// buffers are the fixed [`ATEL_CAM_BUFFER`] (each rounded up to 16
    /// bytes; `MsBattleError` past 0x800), the others' allocated
    /// (`user_malloc` into `+4` and `+0xc`). Managers 0-2 are then set up
    /// (`AtelBattleSetUp`, kinds 2-4 and their callbacks). EAX as the
    /// original leaves it.
    pub fn ms_atel_set_up<M: Mem>(mem: &mut M, call: Call<'_, M>, kind: u32) -> u32 {
        let m = crate::minor::bridge::ATEL_MANAGERS
            .ptr(mem)
            .addr
            .wrapping_add(kind.wrapping_mul(0x54));
        let mut k = 0i32;
        while k < mem.read::<i32>(m) {
            let script: u32 = mem.read(m.wrapping_add(0x14).wrapping_add(k.cast_unsigned() * 4));
            let n = game(mem, &mut *call, ATEL_GET_ACTOR_BUFFER_SIZE, &[script]);
            mem.write(m.wrapping_add(8), word(mem, m, 8).wrapping_add(n));
            let script: u32 = mem.read(m.wrapping_add(0x14).wrapping_add(k.cast_unsigned() * 4));
            let n = game(mem, &mut *call, ATEL_GET_MY_BUFFER_SIZE, &[script]);
            mem.write(m.wrapping_add(0x10), word(mem, m, 0x10).wrapping_add(n));
            k += 1;
        }
        let mut r;
        if kind == 2 {
            // `(n + 15) & ~15` with the original's signed rounding
            let up = |n: u32| {
                let n = n.cast_signed().wrapping_add(0xf);
                (n.wrapping_add((n >> 31) & 0xf) & !0xf).cast_unsigned()
            };
            let buf = ATEL_CAM_BUFFER.ptr(mem).addr;
            mem.write(m.wrapping_add(4), buf);
            let first = up(word(mem, m, 8));
            let second = up(word(mem, m, 0x10));
            mem.write(m.wrapping_add(0xc), buf.wrapping_add(first));
            r = second.wrapping_add(first);
            if r.cast_signed() > 0x800 {
                let msg = code(mem, ATEL_CAM_BUFFER_ERROR);
                r = game(
                    mem,
                    &mut *call,
                    crate::parse::bridge::MS_BATTLE_ERROR,
                    &[msg],
                );
            }
        } else {
            let actors = game(mem, &mut *call, USER_MALLOC, &[word(mem, m, 8)]);
            mem.write(m.wrapping_add(4), actors);
            r = game(mem, &mut *call, USER_MALLOC, &[word(mem, m, 0x10)]);
            mem.write(m.wrapping_add(0xc), r);
        }
        if kind > 2 {
            return r;
        }
        let f = code(mem, ATEL_SET_UP_CALLBACKS[kind as usize]);
        let works = ATEL_WORKS.ptr(mem).addr;
        let args = [
            m.wrapping_add(0x14),
            m.wrapping_add(0x34),
            word(mem, m, 0),
            works,
            word(mem, m, 0xc),
            word(mem, m, 4),
            kind + 2,
            f,
        ];
        game(mem, call, ATEL_BATTLE_SET_UP, &args)
    }

    /// `MsAtelCamRegist(file)` (`0x00797550`): a camera script file (its
    /// script and data at the offsets `+4` and `+8`,
    /// [`crate::minor::bridge::ATEL_CAM`]) on manager 2: the three camera
    /// handlers' works reset, the script registered and set up,
    /// [`crate::minor::bridge::ATEL_CAM_ON`] 0xff. -1; 0 for no file.
    pub fn ms_atel_cam_regist<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, file: u32) -> u32 {
        if file == 0 {
            return 0;
        }
        for n in 0..3 {
            ms_atel_actor_work_init(mem, &mut *s.call, 2, 0, n);
        }
        let script = word(mem, file, 4).wrapping_add(file);
        let data = word(mem, file, 8).wrapping_add(file);
        crate::minor::bridge::ATEL_CAM.set(mem, [script, data]);
        crate::minor::bridge::ms_atel_regist(mem, 2, script);
        ms_atel_set_up(mem, &mut *s.call, 2);
        crate::minor::bridge::ATEL_CAM_ON.set(mem, 0xff);
        u32::MAX
    }

    /// `MsEffectStart()` (`0x00788480`): each actor's effect state
    /// (`+0xdfb`) one step:
    ///
    /// | State | Then |
    /// |---|---|
    /// | 1 | by `+0xdea` (1 the party swapped, 8 an overdrive's party, both once no motion runs; 2, 3 [`crate::damage_end::bridge::DYING_WAIT`]); once still (`+0x508`, `+0x436`, `+0x435`): the turn's end asked, voice 9, the title closed, 2, the start motion `+0xe00` (none: 3), the weak pose, a weapon special's motion 8 (`+0xdfc`), the script camera `+0xe04`/`+0xe08` with an effect |
    /// | 3 | once nothing waits (`+0xdfd`, the targets' motions, the reset wait, sound reads and streams, the system message, the debug hold, a running script camera): [`ms_effect_limit_check`], the camera script, 0xb at once |
    /// | 5 | [`ms_effect_reflect`] |
    /// | 7, 8, 15 | once [`ms_effect_motion_done`]: [`ms_effect_back_to_idle`] (7, 15: 0, 7 with `+0x405`; 8: 9) |
    /// | 9 | after `+0xf65` frames, unless moving: `MsWeaponRead(i, +0xf64, +0xf66)` |
    /// | 10 | unless `+0xdfd`: [`ms_effect_end`] with motion, 6 |
    /// | 11 | no effect: [`ms_effect_end`] (motion by `+0x413`), 6; else 4 and the effect starts (its function `+0xf8c`'s first, ID `MsGetEffID(+0xe1c, 2, i)`) |
    /// | 12 | darkening while the overdrive's window is up, then 13 |
    /// | 13 | lightening, then 11 |
    /// | 14 | 0, the last magic read (`MsEffectRead`) |
    #[allow(clippy::too_many_lines)] // one state machine
    pub fn ms_effect_start<'d>(mem: &mut Dm<'d>, host: &Host<'_>, s: &mut Script<'_, Dm<'d>>) {
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            match byte(mem, a, 0xdfb) {
                1 => effect_first(mem, host, s, i, a),
                3 => {
                    if byte(mem, a, 0xdfd) == 0
                        && crate::flow::bridge::ms_check_start_effect(mem, i, word(mem, a, 0xdc0))
                            == 0
                        && crate::process::bridge::RESET_WAIT2.get(mem) == 0
                        && game(
                            mem,
                            &mut *s.call,
                            crate::process::bridge::SND_IS_READ_STATUS,
                            &[],
                        ) == 0
                        && game(
                            mem,
                            &mut *s.call,
                            crate::process::bridge::SND_GET_STREAM_STATE,
                            &[],
                        ) == 0
                        && crate::exec::bridge::ms_system_message_check(mem, &mut *s.call) == 0
                        && DEBUG_EFFECT_HOLD.get(mem) == 0
                    {
                        if crate::atel::bridge::CAMERA_STATE.get(mem) != 0
                            && CAMERA_ONCE.get(mem) == 1
                        {
                            continue;
                        }
                        let next = ms_effect_limit_check(mem, s, i, a);
                        mem.write(a.wrapping_add(0xdfb), next.to_le_bytes()[0]);
                        ms_effect_cam_regist(mem, s, a);
                    }
                    if byte(mem, a, 0xdfb) == 0xb {
                        effect_go(mem, host, s, i, a);
                    }
                }
                5 => ms_effect_reflect(mem, host, s, i, a),
                7 | 8 => {
                    if ms_effect_motion_done(mem, &mut *s.call, a) != 0 {
                        if byte(mem, a, 0xdfb) == 7 {
                            mem.write(a.wrapping_add(0xdfb), 0u8);
                            mem.write(a.wrapping_add(0x405), 1u8);
                        } else {
                            mem.write(a.wrapping_add(0xdfb), 9u8);
                        }
                        ms_effect_back_to_idle(mem, host, s, i, a);
                    }
                }
                9 => {
                    let wait = byte(mem, a, 0xf65);
                    if wait != 0 {
                        mem.write(a.wrapping_add(0xf65), wait - 1);
                    } else if byte(mem, a, 0x432) == 0 {
                        let (k, id) = (u32::from(byte(mem, a, 0xf64)), half(mem, a, 0xf66));
                        crate::save_param::bridge::ms_weapon_read(mem, s, i, k, id);
                    }
                }
                10 => {
                    if byte(mem, a, 0xdfd) == 0 {
                        ms_effect_end(mem, host, s, a, 1, 1, 0);
                        mem.write(a.wrapping_add(0xdfb), 6u8);
                    }
                }
                0xb => effect_go(mem, host, s, i, a),
                0xc => {
                    if crate::flow::bridge::ms_effect_limit_sync(mem, i) < 0 {
                        let f = LIMIT_FADE.get(mem);
                        let f = if f < 6 {
                            LIMIT_FADE.set(mem, f + 1);
                            f + 1
                        } else {
                            f
                        };
                        limit_filter(mem, &mut *s.call, f);
                    } else {
                        mem.write(a.wrapping_add(0xdfb), 0xdu8);
                    }
                }
                0xd => {
                    let f = LIMIT_FADE.get(mem);
                    if f > 0 {
                        LIMIT_FADE.set(mem, f - 1);
                        limit_filter(mem, &mut *s.call, f - 1);
                    } else {
                        game(mem, &mut *s.call, SG_FILTER_RESET, &[]);
                        mem.write(a.wrapping_add(0xdfb), 0xbu8);
                    }
                }
                0xe => {
                    mem.write(a.wrapping_add(0xdfb), 0u8);
                    let id = crate::parse::bridge::LAST_MAGIC.get(mem);
                    crate::parse::bridge::ms_effect_read(mem, s, i, id);
                }
                0xf if ms_effect_motion_done(mem, &mut *s.call, a) != 0 => {
                    mem.write(a.wrapping_add(0xdfb), 0u8);
                    ms_effect_back_to_idle(mem, host, s, i, a);
                }
                _ => {}
            }
        }
    }

    /// `Sg_FilterSub(0, 0, 0, (16 - f) * 8)`.
    fn limit_filter<M: Mem>(mem: &mut M, call: Call<'_, M>, f: i8) {
        let a = (0x10 - i32::from(f)).wrapping_mul(8).cast_unsigned();
        game(mem, call, SG_FILTER_SUB, &[0, 0, 0, a]);
    }

    /// [`ms_effect_start`]'s state 1.
    fn effect_first<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        a: u32,
    ) {
        match byte(mem, a, 0xdea) {
            1 | 8 if crate::body::bridge::ms_check_chr_motion_all(mem, 1) != 0 => return,
            1 => crate::flow::bridge::ms_exchange_party(mem),
            2 | 3 => crate::damage_end::bridge::DYING_WAIT.set(mem, 1),
            8 => crate::flow::bridge::ms_set_limit_party(mem, i, a),
            _ => {}
        }
        if byte(mem, a, 0x508) != 0 || byte(mem, a, 0x436) != 0 || byte(mem, a, 0x435) != 0 {
            return;
        }
        crate::action::bridge::ms_action_request(mem, host, s, i, 0xff, 7, 0, 2, 0);
        crate::atel::bridge::ms_set_voice_check(mem, s, i, 9, 1, 0, 0);
        let title = crate::parse::bridge::PARSED_COMMAND.get(mem);
        crate::exec::bridge::ms_menu_open_title_window(mem, &mut *s.call, title, 0x3c);
        mem.write(a.wrapping_add(0xdfb), 2u8);
        mem.write(a.wrapping_add(0x3f2), 0xffu8);
        let start = byte(mem, a, 0xe00);
        if byte(mem, a, 0x413) != 0 {
            crate::input::bridge::ms_set_chr_weak(mem, i, 0);
            if 1u32 << i != word(mem, a, 0xdc0) {
                mem.write(a.wrapping_add(0x450), 1u8);
            }
        }
        if byte(mem, a, 0xdfc) != 0 {
            mem.write(a.wrapping_add(0xdfc), 0u8);
            let w = crate::actor::bridge::ms_get_weapon_special(mem, i, 0);
            if w != 0 {
                let chr = u32::from(byte(mem, w, 0xc));
                crate::atel::bridge::ms_set_motion(mem, s, chr, 8, 1, 0, 1, 0, 0);
            }
        }
        if start == 0xff {
            mem.write(a.wrapping_add(0xdfb), 3u8);
        } else {
            crate::atel::bridge::ms_set_motion(mem, s, i, i32::from(start), 1, 0, 1, 0, 0);
        }
        if word(mem, a, 0xf8c) != 0 {
            let (x, y) = (half(mem, a, 0xe04), half(mem, a, 0xe08));
            crate::atel::bridge::ms_atel_request_camera(mem, s, 0, i, x, y, 0, 1, 0, 0);
        }
    }

    /// [`ms_effect_start`]'s state 11: the end without an effect, else the
    /// effect's start.
    fn effect_go<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        a: u32,
    ) {
        let e = word(mem, a, 0xf8c);
        if e == 0 {
            let motion = u32::from(byte(mem, a, 0x413) != 0);
            ms_effect_end(mem, host, s, a, 0, motion, 0);
            mem.write(a.wrapping_add(0xdfb), 6u8);
            return;
        }
        mem.write(a.wrapping_add(0xdfb), 4u8);
        let sleep = crate::damage_end::bridge::DEBUG_CAMERA_SLEEP;
        let [on, _] = sleep.get(mem);
        crate::damage_end::bridge::MAGIC_STATE.set(mem, i.to_le_bytes()[0]);
        sleep.set(mem, [0, on]);
        let n = mem.read::<u16>(a.wrapping_add(0xe1c));
        let id = crate::util::bridge::ms_get_eff_id(u32::from(n), 2, i);
        mem.write(a.wrapping_add(0xe1c), n.wrapping_add(1));
        crate::flow::bridge::ms_set_chr_move_flag_all(mem, u32::from(byte(mem, a, 0x413)));
        crate::chr_damage::bridge::ms_magic_effect_end(mem, &mut *s.call, i);
        let f: u32 = mem.read(e);
        (s.call)(mem, f, &[id]);
        game(mem, &mut *s.call, SG_EFFECT_ROUTINE_ENABLE, &[1]);
    }
}
