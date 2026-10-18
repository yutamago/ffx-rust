//! The battle's teardown leaves (`docs/rewrite-plan.md`, M2): the buffers
//! `MsBattleFree` (`0x00781590`) and `MsBattleAllFree` (`0x00780b40`) give
//! back at the end of a battle, each freed and its pointer cleared.
//!
//! The heap (`user_free_thunk`, `Sg_FreeA`), vfx and `Yn_SeaResetMatrix`
//! (map) go to the game's code through `call`; script's
//! `AtelRemoveCtrlWork` comes from the caller.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::exec::bridge::Call;

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        call(mem, f, args)
    }

    /// `user_free_thunk(p)` (the game's heap).
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `Sg_FreeA(p, pool)` (main).
    pub const SG_FREE_A: u32 = 0x0083_f090;
    /// `op_et_battle_effect_free()` (vfx).
    pub const OP_ET_BATTLE_EFFECT_FREE: u32 = 0x007f_b170;
    /// `Yn_SeaResetMatrix()` (map).
    pub const YN_SEA_RESET_MATRIX: u32 = 0x0091_ae30;

    /// The battle's special stage (`MsBattleSpecial`): 1 the shaking
    /// screen, 2 and 3 the sea; 0 none.
    pub const SPECIAL_KIND: Global<i8> = Global::new(0x0112_a8e6);
    /// The special stage's work (`+0x810` an up vector it set).
    pub const SPECIAL_MEM: Global<u32> = Global::new(0x0113_60a4);
    /// The two buffers `MsFieldItemFree` gives back.
    pub const FIELD_ITEM: Global<u32> = Global::new(0x0113_5ff4);
    pub const FIELD_ITEM2: Global<u32> = Global::new(0x0113_5ffc);

    /// Frees `p` unless null; true when freed.
    fn free<M: Mem>(mem: &mut M, call: Call<'_, M>, p: u32) -> bool {
        if p == 0 {
            return false;
        }
        game(mem, call, USER_FREE, &[p]);
        true
    }

    /// Frees the buffer `g` points at (unless null) and clears `g`.
    fn free_global<M: Mem>(mem: &mut M, call: Call<'_, M>, g: Global<u32>) {
        let p = g.get(mem);
        if free(mem, call, p) {
            g.set(mem, 0);
        }
    }

    /// `MsBtlChrInitMem()` (`0x00793630`): no actors, monsters or weapon
    /// records.
    pub fn ms_btl_chr_init_mem(mem: &mut impl Mem) {
        use crate::actor::bridge::{CHR_TOP, MON_TOP, WEAPON_TOP};
        CHR_TOP.set(mem, 0);
        MON_TOP.set(mem, 0);
        WEAPON_TOP.set(mem, 0);
    }

    /// `MsBtlChrFreeMem()` (`0x00793500`): the actors' and the weapon
    /// records' blocks freed (the monster pointer is only cleared), then
    /// [`ms_btl_chr_init_mem`].
    pub fn ms_btl_chr_free_mem<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        use crate::actor::bridge::{CHR_TOP, WEAPON_TOP};
        let p = CHR_TOP.get(mem);
        free(mem, call, p);
        let p = WEAPON_TOP.get(mem);
        free(mem, call, p);
        ms_btl_chr_init_mem(mem);
    }

    /// `MsNumberFreeMem()` (`0x0079f990`): the damage numbers' pool.
    pub fn ms_number_free_mem<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        free_global(mem, call, crate::misc::bridge::NUMBERS);
    }

    /// `MsEffectEnvFree()` (`0x00787a40`): the effect environment's
    /// buffer (`g_BtlEffectReadStat`).
    pub fn ms_effect_env_free<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        free_global(mem, call, crate::parse::bridge::EFFECT_READ_STAT);
    }

    /// `MsEffectBufferFree_00782750()`: the effect buffer
    /// (`MsEffectGetBufferAddress`).
    pub fn ms_effect_buffer_free<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        free_global(mem, call, crate::misc::bridge::EFFECT_BUFFER);
    }

    /// `MsBtlFreeItemMem()` (`0x00799f70`): the two item buffers.
    pub fn ms_btl_free_item_mem<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        free_global(mem, &mut *call, crate::misc::bridge::ITEM_MEM);
        free_global(mem, call, crate::misc::bridge::ITEM_MEM2);
    }

    /// `MsMemCheckFree()` (`0x00799ec0`): the second memory check block.
    pub fn ms_mem_check_free<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let g = crate::state::bridge::MEM_CHECK2;
        let p = g.get(mem).cast_unsigned();
        if free(mem, call, p) {
            g.set(mem, 0);
        }
    }

    /// `MsFieldItemFree()` (`0x00798f90`): [`FIELD_ITEM`] and
    /// [`FIELD_ITEM2`].
    pub fn ms_field_item_free<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        free_global(mem, &mut *call, FIELD_ITEM);
        free_global(mem, call, FIELD_ITEM2);
    }

    /// `MsBtlScriptRecordFree_00782640(i)`: script record `i`'s `+4`
    /// buffer (`Sg_FreeA(p, 5)`) and the pointer cleared; `i` scales by
    /// 0x20 with wrap-around, as the original's shift.
    pub fn ms_btl_script_record_free<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32) {
        let at = crate::atel::bridge::SCRIPT_RECORDS
            .ptr(mem)
            .addr
            .wrapping_add(i << 5)
            .wrapping_add(4);
        let p: u32 = mem.read(at);
        if p != 0 {
            game(mem, call, SG_FREE_A, &[p, 5]);
            mem.write(at, 0u32);
        }
    }

    /// `MsAtelRegistFree()` (`0x00797a90`): both script managers' `+4`
    /// and `+0xc` buffers freed (not cleared), the script camera
    /// (`MsAtelCamFree`), ctrl works 2 and 3 removed (`remove`) and the
    /// managers reset (`MsAtelRegistInit`).
    pub fn ms_atel_regist_free<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        mut remove: impl FnMut(&mut M, u32),
    ) {
        atel_regist_free(mem, call, &mut |m, _, i| remove(m, i));
    }

    /// [`ms_atel_regist_free`] with `remove` given the heap's `call` too.
    fn atel_regist_free<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        remove: &mut dyn FnMut(&mut M, Call<'_, M>, u32),
    ) {
        let m = crate::minor::bridge::ATEL_MANAGERS.ptr(mem).addr;
        for k in 0..2 {
            for o in [4, 0xc] {
                let p = mem.read(m + 0x54 * k + o);
                free(mem, &mut *call, p);
            }
        }
        crate::flow::bridge::ms_atel_cam_free(mem, |m, i| remove(m, &mut *call, i));
        remove(mem, &mut *call, 2);
        remove(mem, &mut *call, 3);
        crate::flow::bridge::ms_atel_regist_init(mem);
    }

    /// `MsEtEffectStop()` (`0x0079e7b0`): with the battle's effects read
    /// (`g_BtlEtEffectRead`), `op_et_battle_effect_free` (the original
    /// jumps there; its result is the original's, which no caller reads).
    pub fn ms_et_effect_stop<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        if crate::misc::bridge::ET_EFFECT_READ.get(mem) != 0 {
            game(mem, call, OP_ET_BATTLE_EFFECT_FREE, &[]);
        }
    }

    /// `MsBattleSpecialFree()` (`0x0079dde0`): the special stage undone.
    /// Kind 1 stops screen 0's shake, kinds 2 and 3 reset the sea's
    /// matrix (`Yn_SeaResetMatrix`); then the up vector at the work's
    /// `+0x810` is set back (`MsSetNormalUpVector`) and the work freed.
    /// The kind ends 0 in every case.
    pub fn ms_battle_special_free<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let kind = SPECIAL_KIND.get(mem);
        if (1..=3).contains(&kind) {
            if kind == 1 {
                crate::camera::bridge::ms_screen_reset_shake(mem, 0);
            } else {
                game(mem, &mut *call, YN_SEA_RESET_MATRIX, &[]);
            }
            let w = SPECIAL_MEM.get(mem);
            let up = mem.read(w.wrapping_add(0x810));
            crate::vector::bridge::ms_set_normal_up_vector(mem, up);
            let w = SPECIAL_MEM.get(mem);
            game(mem, call, USER_FREE, &[w]);
            SPECIAL_MEM.set(mem, 0);
        }
        SPECIAL_KIND.set(mem, 0);
    }

    /// `MsMenuFreeText(i)` (`0x0078fb00`): menu text `i` (below 17,
    /// unsigned; `g_BtlMenuText`, 12 bytes each) frees its buffer (`+8`)
    /// when it has one and its `+0` byte is 0: 0, else -1. The entry stays
    /// as it was: the original's `MsStructClear` of 12 bytes lands on its
    /// own argument slot and the two words above it (its caller's stack,
    /// which the port does not write; for `MsBattleAllFree`'s first call
    /// they hold its return address).
    pub fn ms_menu_free_text<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32) -> i32 {
        if i > 0x10 {
            return -1;
        }
        let e = crate::flow::bridge::MENU_TEXT.ptr(mem).addr + 12 * i;
        let p: u32 = mem.read(e + 8);
        if p == 0 || mem.read::<u8>(e) != 0 {
            return -1;
        }
        game(mem, call, USER_FREE, &[p]);
        0
    }

    /// `Ch_Dispose(model)` (chara).
    pub const CH_DISPOSE: u32 = 0x0082_66f0;
    /// `Ch_ModelCreatePacket(model)` (chara).
    pub const CH_MODEL_CREATE_PACKET: u32 = 0x0082_7280;

    /// `MsBtlChrFreeAll()` (`0x00793480`): every model of the 0x5d actors
    /// and records that is no ride (`+0x12`) disposed (`Ch_Dispose`), then
    /// each one's share flag `+0x3e` cleared and the actor disposed
    /// (`MsDebugFullCommand_00796dc0`, flags 2 with `+0x22`, 4 with
    /// `+0x21`), and the extra block (`g_BtlChrExtra`) freed and cleared.
    pub fn ms_btl_chr_free_all<'d>(
        mem: &mut crate::target::bridge::Dm<'d>,
        s: &mut crate::atel::bridge::Script<'_, crate::target::bridge::Dm<'d>>,
    ) {
        use crate::actor::bridge::ms_get_chr;
        for i in 0..0x5d {
            let a = ms_get_chr(mem, i);
            let model: u32 = mem.read(a);
            if model != 0 && mem.read::<u8>(a.wrapping_add(0x12)) == 0 {
                game(mem, &mut *s.call, CH_DISPOSE, &[model]);
            }
        }
        for i in 0..0x5d {
            let a = ms_get_chr(mem, i);
            mem.write(a.wrapping_add(0x3e), 0u8);
            let mut flags = 0;
            if mem.read::<u8>(a.wrapping_add(0x22)) != 0 {
                flags = 2;
            }
            if mem.read::<u8>(a.wrapping_add(0x21)) != 0 {
                flags |= 4;
            }
            crate::damage_end::bridge::ms_chr_dispose(mem, s, i, a, flags);
        }
        let extra = crate::flow::bridge::CHR_EXTRA;
        let p = extra.get(mem);
        game(mem, &mut *s.call, USER_FREE, &[p]);
        extra.set(mem, 0);
    }

    /// `MsChrCreatePacket(actor)` (`0x00793d90`): with its packet request
    /// (`+0x3a`) and a model, the model's packets are made
    /// (`Ch_ModelCreatePacket`): 1; else 0. The request ends cleared.
    pub fn ms_chr_create_packet<M: Mem>(mem: &mut M, call: Call<'_, M>, a: u32) -> u32 {
        let model: u32 = mem.read(a);
        let asked = mem.read::<u8>(a.wrapping_add(0x3a)) != 0;
        let made = asked && model != 0;
        if made {
            game(mem, call, CH_MODEL_CREATE_PACKET, &[model]);
        }
        mem.write(a.wrapping_add(0x3a), 0u8);
        u32::from(made)
    }

    /// The event script's actor accessors `MsBtlBridgeBtl2Event` uses
    /// (script; `(e)` or `(e, k[, v])`).
    pub const ATEL_GET_EVENT_ACTOR_TOTAL: u32 = 0x0086_b300;
    pub const ATEL_GET_EVENT_BIND_ID: u32 = 0x0086_b370;
    pub const ATEL_GET_EVENT_CHAR_ID: u32 = 0x0086_b3d0;
    pub const ATEL_GET_EVENT_CHAR_HANDLE: u32 = 0x0086_b3a0;
    pub const ATEL_GET_EVENT_ACTOR_LINK_WORK: u32 = 0x0086_b1b0;
    pub const ATEL_GET_BIND_PC: u32 = 0x0086_ae00;
    pub const ATEL_GET_HIDE_STATUS: u32 = 0x0086_ba40;
    pub const ATEL_GET_KEEP_BATTLE_PARTY: u32 = 0x0086_bbf0;
    pub const ATEL_GET_EVENT_ELEM_STATUS: u32 = 0x0086_b4c0;
    pub const ATEL_SET_EVENT_ELEM_HANDLE: u32 = 0x0086_fde0;
    pub const ATEL_SET_EVENT_ELEM_POS: u32 = 0x0086_fe20;
    pub const ATEL_SET_EVENT_ELEM_CHAR_ID: u32 = 0x0086_fda0;
    pub const ATEL_SET_EVENT_ELEM_STATUS: u32 = 0x0086_fe60;
    pub const ATEL_SET_EVENT_CHAR_HANDLE: u32 = 0x0086_fd60;
    pub const ATEL_SET_EVENT_ACTOR_LINK_WORK: u32 = 0x0086_fc40;
    /// `graphicLevelGetNameHash()` (graphic).
    pub const GRAPHIC_LEVEL_GET_NAME_HASH: u32 = 0x0064_1da0;
    /// The battle scene and level `MsBtlBridgeBtl2Event` keeps its models
    /// shown in (`MsGetBattleScene`, `graphicLevelGetNameHash`).
    pub const SHOWN_SCENE: u32 = 0x0237_0000;
    pub const SHOWN_LEVEL_HASH: u32 = 0x21e8;

    /// Whether a model handed to the event stays hidden (`Ch_SetHideEvent`'s
    /// flag): 1 with `hide` set, else only in scene [`SHOWN_SCENE`] on level
    /// [`SHOWN_LEVEL_HASH`].
    fn event_hide<M: Mem>(mem: &mut M, call: Call<'_, M>, hide: u32) -> u32 {
        if hide != 0 {
            return 1;
        }
        u32::from(
            crate::state::bridge::ms_get_battle_scene(mem) == SHOWN_SCENE
                && game(mem, call, GRAPHIC_LEVEL_GET_NAME_HASH, &[]) == SHOWN_LEVEL_HASH,
        )
    }

    /// `MsBtlBridgeBtl2Event()` (`0x007821f0`): at the end of a battle the
    /// event script takes the battle's models over. Every actor's status
    /// effects end (`MsStatusEffectClear`), every model of the 0x5d actors
    /// and records without a ride (`+0x12`) is reset (thickness -1,
    /// texture timer on, CLUT off and freed, shown in battle and effects),
    /// actor 6's arm shown (`MsHideArm`). Then each event actor `e` bound
    /// to a battle character (bind ID 1, a handle) finds its actor: its
    /// bind PC (below 0x1c, unsigned) if that one's model is there with the
    /// event's character ID (`+4`), else the first of the 0x1f such (none:
    /// link bit 2). That actor's two weapons (event elements whose status
    /// is set) hand their model (`+0`), position (`+0xd`) and ID (`+4`) to
    /// the element (or clear it), get their packets
    /// ([`ms_chr_create_packet`]) and leave the battle (`+0`, `+0x22`
    /// cleared), as does the actor's model (the event's handle); the models
    /// stay hidden ([`event_hide`]) when the event's hide status says so
    /// (0 for a kept party member found in the party, -2 for one not; 0
    /// while `+0x1c`; -1 when `+0x1b` is clear), which also sets link
    /// bit 6. A link with bit 7 clears the actor's `+0x21`. The link is
    /// written back for every event actor.
    #[allow(clippy::too_many_lines)]
    pub fn ms_btl_bridge_btl2_event<M: Mem>(
        mem: &mut M,
        s: &mut crate::atel::bridge::Script<'_, M>,
    ) {
        use crate::actor::bridge::{ms_get_chr, ms_get_weapon};
        use ffx_chara::chr::bridge as c;
        use ffx_chara::texture::bridge as tx;
        for i in 0..0x1f {
            crate::flow::bridge::ms_status_effect_clear(mem, i, &mut *s.off);
        }
        for i in 0..0x5d {
            let a = ms_get_chr(mem, i);
            let model: u32 = mem.read(a);
            if model == 0 || mem.read::<u8>(a.wrapping_add(0x12)) != 0 {
                continue;
            }
            let minus_one = crate::vector::bridge::MINUS_ONE.get(mem);
            c::ch_set_thickness(mem, model, f32::from_bits(minus_one));
            ffx_chara::texanim::bridge::ch_texture_set_anim_timer(mem, model, 1);
            tx::ch_clut_enable(mem, model, 0);
            tx::ch_clut_dispose(mem, model, &mut s.call);
            c::ch_set_hide_battle(mem, model, 0);
            c::ch_set_hide_effect(mem, model, 0);
        }
        crate::status::bridge::ms_hide_arm(mem, &mut *s.call, 6, 0);
        let total = game(mem, &mut *s.call, ATEL_GET_EVENT_ACTOR_TOTAL, &[]).cast_signed();
        for e in 0..total.max(0).cast_unsigned() {
            let g = |mem: &mut M,
                     s: &mut crate::atel::bridge::Script<'_, M>,
                     f: u32,
                     args: &[u32]| { game(mem, &mut *s.call, f, args) };
            let bind = g(mem, s, ATEL_GET_EVENT_BIND_ID, &[e]);
            let id = g(mem, s, ATEL_GET_EVENT_CHAR_ID, &[e]);
            let handle = g(mem, s, ATEL_GET_EVENT_CHAR_HANDLE, &[e]);
            let mut link = g(mem, s, ATEL_GET_EVENT_ACTOR_LINK_WORK, &[e]) & 0xff;
            let pc = g(mem, s, ATEL_GET_BIND_PC, &[e]);
            let mut hide = g(mem, s, ATEL_GET_HIDE_STATUS, &[e]);
            let keep = g(mem, s, ATEL_GET_KEEP_BATTLE_PARTY, &[e]);
            if bind == 1 && handle != 0 {
                let ours = |mem: &M, k: u32| {
                    let a = ms_get_chr(mem, k);
                    (mem.read::<u32>(a) != 0 && mem.read::<u32>(a.wrapping_add(4)) == id)
                        .then_some(a)
                };
                let found = if pc != u32::MAX && pc <= 0x1b {
                    ours(mem, pc).map(|a| (pc, a))
                } else {
                    None
                }
                .or_else(|| (0..0x1f).find_map(|k| ours(mem, k).map(|a| (k, a))));
                if let Some((k, a)) = found {
                    if keep != 0 {
                        let slot: u8 = mem.read(a.wrapping_add(0xc));
                        let party = crate::ability::bridge::PARTY.get(mem);
                        hide = if party[..3].contains(&slot) {
                            0
                        } else {
                            0xffff_fffe
                        };
                    }
                    let mut stay = if mem.read::<u8>(a.wrapping_add(0x1c)) != 0 {
                        0
                    } else {
                        hide
                    };
                    if mem.read::<u8>(a.wrapping_add(0x1b)) == 0 {
                        stay = u32::MAX;
                    }
                    for n in 0..2u32 {
                        if g(mem, s, ATEL_GET_EVENT_ELEM_STATUS, &[e, n]) == 0 {
                            continue;
                        }
                        let wp = ms_get_weapon(mem, k, n.cast_signed());
                        let model: u32 = mem.read(wp);
                        let pos = u32::from(mem.read::<u8>(wp.wrapping_add(0xd)));
                        let wid: u32 = mem.read(wp.wrapping_add(4));
                        if model == 0 || wid == 0 {
                            for f in [
                                ATEL_SET_EVENT_ELEM_HANDLE,
                                ATEL_SET_EVENT_ELEM_POS,
                                ATEL_SET_EVENT_ELEM_CHAR_ID,
                                ATEL_SET_EVENT_ELEM_STATUS,
                            ] {
                                g(mem, s, f, &[e, n, 0]);
                            }
                            continue;
                        }
                        g(mem, s, ATEL_SET_EVENT_ELEM_HANDLE, &[e, n, model]);
                        g(mem, s, ATEL_SET_EVENT_ELEM_POS, &[e, n, pos]);
                        g(mem, s, ATEL_SET_EVENT_ELEM_CHAR_ID, &[e, n, wid]);
                        g(mem, s, ATEL_SET_EVENT_ELEM_STATUS, &[e, n, 1]);
                        ms_chr_create_packet(mem, &mut *s.call, wp);
                        let h = event_hide(mem, &mut *s.call, stay);
                        c::ch_set_hide_event(mem, model, h, &mut s.call);
                        mem.write(wp, 0u32);
                        mem.write(wp.wrapping_add(0x22), 0u8);
                    }
                    let model: u32 = mem.read(a);
                    g(mem, s, ATEL_SET_EVENT_CHAR_HANDLE, &[e, model]);
                    ms_chr_create_packet(mem, &mut *s.call, a);
                    let model: u32 = mem.read(a);
                    mem.write(a, 0u32);
                    mem.write(a.wrapping_add(0x22), 0u8);
                    if stay != 0 {
                        link |= 0x40;
                    }
                    let h = event_hide(mem, &mut *s.call, stay);
                    c::ch_set_hide_event(mem, model, h, &mut s.call);
                    if link & 0x80 != 0 {
                        mem.write(a.wrapping_add(0x21), 0u8);
                    }
                } else {
                    link |= 4;
                }
            }
            game(
                mem,
                &mut *s.call,
                ATEL_SET_EVENT_ACTOR_LINK_WORK,
                &[e, link],
            );
        }
    }

    /// `GetCD()` (system), `graphicSetIsBattleMap(v)`,
    /// `graphicMenuUnloadSpecialTexture(k)` (render),
    /// `Sg_IdSetEffectTableIndex(v)` (main), `TOClearBtlWindow()` (system),
    /// `TrophyManager_DestroyInstance()` (system).
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const GRAPHIC_SET_IS_BATTLE_MAP: u32 = 0x0064_3490;
    pub const GRAPHIC_MENU_UNLOAD_SPECIAL_TEXTURE: u32 = 0x0064_2390;
    pub const SG_ID_SET_EFFECT_TABLE_INDEX: u32 = 0x0083_e9d0;
    pub const TO_CLEAR_BTL_WINDOW: u32 = 0x0089_ad90;
    pub const TROPHY_MANAGER_DESTROY_INSTANCE: u32 = 0x007a_0cb0;

    /// `MsBattleFree()` (`0x00781590`): in battle (`g_BtlStatus`), the CD
    /// object's third method with 0 (`GetCD()`), not a battle map any more,
    /// the encounter count saved (`MsSaveEncountCheck`), effect table 0,
    /// then the teardown leaves in the original's order (the effect
    /// buffer, numbers, items, the special stage, 3D sounds, the scene's
    /// read buffer, effects, the debug camera, the battle window, the
    /// models to the event ([`ms_btl_bridge_btl2_event`]), script record
    /// 1, every model ([`ms_btl_chr_free_all`]), the script managers, the
    /// effect environment, the actor blocks), the special menu texture 8
    /// unloaded and Zanmato's list reset; overdrive cue, status and the
    /// item in use cleared. 0 (the original's `eax`; out of battle the
    /// status byte's test leaves it as it came, here 0 too).
    pub fn ms_battle_free<'d>(
        mem: &mut crate::target::bridge::Dm<'d>,
        s: &mut crate::atel::bridge::Script<'_, crate::target::bridge::Dm<'d>>,
    ) -> u32 {
        if crate::state::bridge::STATUS.get(mem) == 0 {
            return 0;
        }
        let cd = game(mem, &mut *s.call, GET_CD, &[]);
        let method: u32 = mem.read(cd.wrapping_add(8));
        (s.call)(mem, method, &[0]);
        game(mem, &mut *s.call, GRAPHIC_SET_IS_BATTLE_MAP, &[0]);
        crate::minor::bridge::ms_save_encount_check(mem);
        game(mem, &mut *s.call, SG_ID_SET_EFFECT_TABLE_INDEX, &[0]);
        ms_effect_buffer_free(mem, &mut *s.call);
        ms_number_free_mem(mem, &mut *s.call);
        ms_btl_free_item_mem(mem, &mut *s.call);
        ms_battle_special_free(mem, &mut *s.call);
        crate::battle_end::bridge::ms_sep_battle_clear(mem, &mut *s.call);
        crate::battle_end::bridge::ms_btl_read_scene_free(mem, &mut *s.call);
        ms_et_effect_stop(mem, &mut *s.call);
        crate::misc::bridge::ms_debug_camera_sleep_init(mem);
        game(mem, &mut *s.call, TO_CLEAR_BTL_WINDOW, &[]);
        ms_btl_bridge_btl2_event(mem, s);
        ms_btl_script_record_free(mem, &mut *s.call, 1);
        ms_btl_chr_free_all(mem, s);
        atel_regist_free(mem, &mut *s.call, &mut |m, c, i| {
            game(m, c, crate::damage_end::bridge::ATEL_REMOVE_CTRL_WORK, &[i]);
        });
        ms_effect_env_free(mem, &mut *s.call);
        ms_btl_chr_free_mem(mem, &mut *s.call);
        game(mem, &mut *s.call, GRAPHIC_MENU_UNLOAD_SPECIAL_TEXTURE, &[8]);
        crate::minor::bridge::reset_zanmato_sep_list(mem);
        crate::process::bridge::LIMIT_TYPE_CUE.set(mem, 0);
        crate::state::bridge::STATUS.set(mem, 0);
        crate::save::bridge::USE_ITEM.set(mem, 0);
        0
    }

    /// The menu texts `MsBattleAllFree` frees, in its order.
    pub const ALL_FREE_TEXTS: [u32; 15] = [0, 1, 3, 4, 5, 6, 8, 10, 11, 12, 7, 14, 9, 15, 16];
    /// The kernel tables and files `MsBattleAllFree` frees last, in its
    /// order (freed whatever they hold, not cleared): the field list, the
    /// command, monster magic 1 and 2, auto-ability, summon floors, player
    /// ROM, `0x0112a93c`, item, effect files, status ROM, motion binary,
    /// Kaizou and summon growth tables.
    pub const ALL_FREE_TABLES: [u32; 14] = [
        0x0112_a9c4,
        0x0112_a92c,
        0x0112_a930,
        0x0112_a934,
        0x0112_a944,
        0x0112_a948,
        0x0112_a938,
        0x0112_a93c,
        0x0112_a940,
        0x0112_a950,
        0x0112_a958,
        0x0112_a954,
        0x0112_a964,
        0x0112_a960,
    ];

    /// `MsBattleAllFree()` (`0x00780b40`): [`ms_battle_free`], script
    /// record 0, the menu texts [`ALL_FREE_TEXTS`]
    /// ([`ms_menu_free_text`]), the field items, the trophy manager
    /// (`TrophyManager_DestroyInstance`), the memory check block, then
    /// [`ALL_FREE_TABLES`]. Returns the last `user_free`'s `eax`.
    pub fn ms_battle_all_free<'d>(
        mem: &mut crate::target::bridge::Dm<'d>,
        s: &mut crate::atel::bridge::Script<'_, crate::target::bridge::Dm<'d>>,
    ) -> u32 {
        ms_battle_free(mem, s);
        ms_btl_script_record_free(mem, &mut *s.call, 0);
        for i in ALL_FREE_TEXTS {
            ms_menu_free_text(mem, &mut *s.call, i);
        }
        ms_field_item_free(mem, &mut *s.call);
        game(mem, &mut *s.call, TROPHY_MANAGER_DESTROY_INSTANCE, &[]);
        ms_mem_check_free(mem, &mut *s.call);
        let mut eax = 0;
        for g in ALL_FREE_TABLES {
            let p = Global::<u32>::new(g).get(mem);
            eax = game(mem, &mut *s.call, USER_FREE, &[p]);
        }
        eax
    }
}
