//! M5 finish, WP5 part 6 (wp5g): small `FFX.exe` functions in gaps of the
//! dump that ported code stores as callbacks or calls, across modules:
//! the event debug memory's save/restore pair, the system motions'
//! disposal tail, the semaphore queue's destructor, the battle position
//! lists' allocations, the field scene's wave draws, the sound standby's
//! end, a request list's destructor, the PS2 debug list's release, the
//! HDD-mode refresh, the battle menu's scene steps, the sphere grid's
//! draw jobs, states and draw hooks, two PC UI pass steps, the read
//! task's step and the field scene's step. None is in the dump; each was
//! read from the machine code. Callbacks whose callers read no result
//! (the job runner, the grid's state and hook calls, the tasks) are void.

pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn global(h: &impl Host, g: u32) -> u32 {
        h.read(h.rebase(g))
    }

    fn set_global(h: &mut impl Host, g: u32, v: u32) {
        let p = h.rebase(g);
        h.write(p, v);
    }

    pub const TK_GET_SAVE_EVENT_ADDRESS: u32 = 0x0088_c830;
    pub const MEMCPY: u32 = 0x0094_9256;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// The event debug memory's copy (`0x0132d570`, 0x1000 bytes) and the
    /// save event's area it mirrors (`+0x1ec`).
    pub const EVENT_DEBUG_COPY: u32 = 0x0132_d570;

    /// `0x0087e6f0()` (restore) / `0x0087e720()` (save), the pair
    /// `SetGlobals_0086e970` registers: 0x1000 bytes between the save
    /// event's `+0x1ec` and [`EVENT_DEBUG_COPY`] (into the event for
    /// `restore`), then a debug print (`dbgPrintf`, an empty `ret` in the
    /// game: its `eax`, the copy's destination, is the result).
    pub fn event_debug_copy(h: &mut impl Host, restore: bool) -> u32 {
        let e = cdecl(h, TK_GET_SAVE_EVENT_ADDRESS, &[]).wrapping_add(0x1ec);
        let copy = h.rebase(EVENT_DEBUG_COPY);
        let (to, from, text) = if restore {
            (e, copy, 0x00b5_ccdc)
        } else {
            (copy, e, 0x00b5_ccc8)
        };
        cdecl(h, MEMCPY, &[to, from, 0x1000]);
        let text = h.rebase(text);
        cdecl(h, DBG_PRINTF, &[text])
    }

    pub const SG_DISPOSE_MOTION_GROUP: u32 = 0x0083_6e20;
    pub const CACHE_DISPOSE: u32 = 0x0084_0250;

    /// `ChnSystemMgrpReset_tail_00837690()`: the system motion group
    /// disposed (`SG_DisposeMotionGroup(2, 0, 0)`) and its cache entry
    /// (`Cache_Dispose(2, 0x903e7, 1)`); the latter's `eax`.
    pub fn system_mgrp_dispose(h: &mut impl Host) -> u32 {
        cdecl(h, SG_DISPOSE_MOTION_GROUP, &[2, 0, 0]);
        cdecl(h, CACHE_DISPOSE, &[2, 0x0009_03e7, 1])
    }

    /// The handle import `0x006db6f0` closes four handles with
    /// (`[0x00b0c0dc]`, stdcall) and `eh_vector_destructor_iterator`.
    pub const CLOSE_HANDLE_SLOT: u32 = 0x00b0_c0dc;
    pub const EH_VECTOR_DTOR: u32 = 0x0094_96d2;
    pub const QUEUE_RECORD_DTOR: u32 = 0x0070_3900;

    /// `SemaphoreQueue_dtor_006db6f0(this)` (thiscall): the handles at
    /// `+0x1e4`, `+0x1e0`, `+0x1dc`, `+0x1ac` closed, then the eight
    /// 0x30-byte records from `this` destroyed (`0x00703900` each, last
    /// first); the iterator's `eax`.
    pub fn semaphore_queue_dtor(h: &mut impl Host, this: u32) -> u32 {
        let close = global(h, CLOSE_HANDLE_SLOT);
        for o in [0x1e4u32, 0x1e0, 0x1dc, 0x1ac] {
            let handle: u32 = h.read(this.wrapping_add(o));
            h.call(close, Conv::Stdcall, &[handle]);
        }
        let dtor = h.rebase(QUEUE_RECORD_DTOR);
        h.call_sym(EH_VECTOR_DTOR, Conv::Stdcall, &[this, 0x30, 8, dtor])
    }

    /// The battle position system (`0x0113e27c`), `ms_malloc_007cf970(n)`
    /// and `MsStructClear(p, n)`.
    pub const BTL_POS: u32 = 0x0113_e27c;
    pub const MS_MALLOC: u32 = 0x007c_f970;
    pub const MS_STRUCT_CLEAR: u32 = 0x0079_f230;

    /// `0x007c91a0()` (`count` `+0x30`, words `+0x34`/`+0x38` cleared,
    /// lists `+0x4c`/`+0x50`) / `0x007cc180()` (`+0xc`, `+0x10`/`+0x14`,
    /// `+0x28`/`+0x2c`): two lists of `count` 12-byte entries allocated
    /// and cleared; the second clear's `eax`.
    pub fn btl_pos_alloc_lists(h: &mut impl Host, count: u32, clear: u32, lists: u32) -> u32 {
        let p = global(h, BTL_POS);
        h.write(p.wrapping_add(clear), 0u32);
        h.write(p.wrapping_add(clear + 4), 0u32);
        let mut r = 0;
        for k in 0..2 {
            let p = global(h, BTL_POS);
            let n = h.read::<u32>(p.wrapping_add(count)).wrapping_mul(12);
            let a = cdecl(h, MS_MALLOC, &[n]);
            let p = global(h, BTL_POS);
            h.write(p.wrapping_add(lists + 4 * k), a);
            let n = h.read::<u32>(p.wrapping_add(count)).wrapping_mul(12);
            r = cdecl(h, MS_STRUCT_CLEAR, &[a, n]);
        }
        r
    }

    /// The field scene's sound record list (`0x0193bf80`).
    pub const SCENE_WAVES: u32 = 0x0193_bf80;

    /// `0x009215e0()` / `0x00921610()`: `draw` (`rcbgSceneDraw` /
    /// `rcbgSceneSetChadist_thunk`) of the list's first record's `+8` (0
    /// without a list or a record); its `eax`.
    pub fn scene_wave_draw(h: &mut impl Host, draw: u32) -> u32 {
        let list = global(h, SCENE_WAVES);
        let rec = if list == 0 { 0 } else { h.read::<u32>(list) };
        let id = if rec == 0 {
            0
        } else {
            h.read(rec.wrapping_add(8))
        };
        cdecl(h, draw, &[id])
    }

    /// `0x00921a10()` / `0x00921a30()`: with a list holding a record, a
    /// tail jump to `body` (`0x009215e0` / `0x00921610`); else the list
    /// pointer (0 or the empty list, the original's `eax`).
    pub fn scene_wave_draw_if_any(h: &mut impl Host, body: u32) -> u32 {
        let list = global(h, SCENE_WAVES);
        if list == 0 || h.read::<u32>(list) == 0 {
            return list;
        }
        cdecl(h, body, &[])
    }

    /// `SqsStandbyDone_0081fcb0()`: the standby flag (`0x012f4e74` bit 0)
    /// cleared and the word the pointer `0x00c49464` holds kept in
    /// `0x012f4150`; that word.
    pub fn sqs_standby_done(h: &mut impl Host) -> u32 {
        let p = global(h, 0x00c4_9464);
        let flags = global(h, 0x012f_4e74);
        set_global(h, 0x012f_4e74, flags & !1);
        let v: u32 = h.read(p);
        set_global(h, 0x012f_4150, v);
        v
    }

    pub const USER_FREE: u32 = 0x0068_7010;

    /// `RequestList_dtor_00690a30(this)` (thiscall): the buffer `+8` freed
    /// when there is one (`user_free_thunk`); its `eax`, or 0.
    pub fn request_list_dtor(h: &mut impl Host, this: u32) -> u32 {
        let p: u32 = h.read(this.wrapping_add(8));
        if p == 0 {
            return 0;
        }
        cdecl(h, USER_FREE, &[p])
    }

    pub const TK_IS_DEBUG: u32 = 0x0088_7d10;
    pub const ATEL_REMOVE_PS2_DEBUG: u32 = 0x0087_9e70;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const TK_FREE: u32 = 0x0088_d520;
    /// The PS2 debug list's two blocks (`0x023cbd78`, `0x023cbd7c`).
    pub const DEBUG_BLOCKS: u32 = 0x023c_bd78;

    /// `AtelReleaseDebugList_00873d60()`: in debug mode
    /// `AtelRemovePS2Debug()`; each of the two blocks freed (`TkFree`,
    /// between two `FlushCache(0)`, empty `ret`s in the game) and cleared.
    /// The original's `eax`: the last flush's (the free's in the game),
    /// else the debug calls'.
    pub fn release_debug_list(h: &mut impl Host) -> u32 {
        let mut r = cdecl(h, TK_IS_DEBUG, &[]);
        if r != 0 {
            r = cdecl(h, ATEL_REMOVE_PS2_DEBUG, &[]);
        }
        let blocks = h.rebase(DEBUG_BLOCKS);
        for k in 0..2u32 {
            let slot = blocks.wrapping_add(4 * k);
            let b: u32 = h.read(slot);
            if b != 0 {
                cdecl(h, FLUSH_CACHE, &[0]);
                cdecl(h, TK_FREE, &[b]);
                r = cdecl(h, FLUSH_CACHE, &[0]);
                h.write(slot, 0u32);
            }
        }
        r
    }

    pub const HDD_MODE_REFRESH: u32 = 0x0062_f640;

    /// `0x00885560()` (the HDD mode's refresh): with `0x0132f570` positive
    /// (signed), `code_0062f640(it)`'s `eax`; else 1.
    pub fn hdd_mode_refresh(h: &mut impl Host) -> u32 {
        let v = global(h, 0x0132_f570);
        if v.cast_signed() > 0 {
            return cdecl(h, HDD_MODE_REFRESH, &[v]);
        }
        1
    }

    pub const TO_SEQ_STOP_EXEC_SCENE: u32 = 0x008a_f380;
    pub const TO_SEQ_START_SCENE: u32 = 0x008a_f0d0;
    pub const TO_BTL_DRAW_TARGET_CURSOR: u32 = 0x0089_c870;

    /// The battle menu's scene stops (`0x008a2a40` .. `0x008a2ab0`, eight
    /// copies, 16 bytes apart): `TOSeqStopExecScene(2)`; 1.
    pub fn btl_scene_stop(h: &mut impl Host) -> u32 {
        cdecl(h, TO_SEQ_STOP_EXEC_SCENE, &[2]);
        1
    }

    /// The battle menu's scene starts (`0x008a86b0`, `0x008a86f0`,
    /// `0x008a8710`, `0x008a8730` with `0x00b5f088`, `0x008a8780` with
    /// `0x00b5f094`): `TOSeqStartScene(2, name)`; 1.
    pub fn btl_scene_start(h: &mut impl Host, name: u32) -> u32 {
        let name = h.rebase(name);
        cdecl(h, TO_SEQ_START_SCENE, &[2, name]);
        1
    }

    /// `0x008a84c0()`: `TOBtlDrawTargetCursor()`; 0.
    pub fn btl_target_cursor_0(h: &mut impl Host) -> u32 {
        cdecl(h, TO_BTL_DRAW_TARGET_CURSOR, &[]);
        0
    }

    /// The sphere grid's work (`0x02305834`).
    pub const ABMAP: u32 = 0x0230_5834;
    pub const RESET_LAYERS: u32 = 0x0063_90d0;
    pub const DRAW_LINKS: u32 = 0x00a4_b4b0;
    pub const DRAW_NODE_HALOS: u32 = 0x00a5_0290;
    pub const DRAW_CHR_RINGS: u32 = 0x00a4_c8d0;
    pub const DRAW_NODE_SPHERES: u32 = 0x00a5_0ed0;
    pub const DRAW_SPHERE_JOB_TAIL: u32 = 0x00a5_0b50;
    pub const DRAW_NODE_HIGHLIGHT: u32 = 0x00a5_1560;
    pub const DRAW_NODES_EAMAP: u32 = 0x00a5_1340;
    pub const DRAW_NODE_SHAPES_TAIL: u32 = 0x00a4_ffd0;
    pub const DRAW_NODE_SHAPES: u32 = 0x00a4_fe40;

    fn abmap(h: &impl Host) -> u32 {
        global(h, ABMAP)
    }

    fn word_at(h: &impl Host, o: u32) -> u32 {
        let g = abmap(h);
        h.read(g.wrapping_add(o))
    }

    fn set_word_at(h: &mut impl Host, o: u32, v: u32) {
        let g = abmap(h);
        h.write(g.wrapping_add(o), v);
    }

    /// The grid's draw job 0 (`0x00a53450`, `eiAbmDraw`'s first kick):
    /// with a node picked (`+0x116a8` not -1), by the link fade state
    /// `+0x116ac`: 1 resets layer 0, draws the links and moves to 2; 3
    /// goes back to 0; any other positive state counts up. (No result:
    /// the job runner reads none.)
    pub fn abmap_job_links(h: &mut impl Host) {
        if word_at(h, 0x116a8) == u32::MAX {
            return;
        }
        let state = word_at(h, 0x116ac);
        if state == 1 {
            cdecl(h, RESET_LAYERS, &[0]);
            cdecl(h, DRAW_LINKS, &[]);
            set_word_at(h, 0x116ac, 2);
        } else if state == 3 {
            set_word_at(h, 0x116ac, 0);
        } else if state.cast_signed() > 0 {
            set_word_at(h, 0x116ac, state.wrapping_add(1));
        }
    }

    /// Draw job 1 (`0x00a534b0`): the node halos, then the character
    /// rings (a tail jump).
    pub fn abmap_job_halos(h: &mut impl Host) {
        cdecl(h, DRAW_NODE_HALOS, &[]);
        cdecl(h, DRAW_CHR_RINGS, &[]);
    }

    /// Draw job 2 (`0x00a534c0`): the picked node's spheres (`+0x116a8`
    /// not negative), layer 2 or 3 reset for the zoom state `+0x116b0`
    /// -2 or 2, and with that state positive `0x00a50b50` (a tail jump).
    pub fn abmap_job_spheres(h: &mut impl Host) {
        let node = word_at(h, 0x116a8);
        if node.cast_signed() >= 0 {
            cdecl(h, DRAW_NODE_SPHERES, &[node]);
        }
        let zoom = word_at(h, 0x116b0);
        if zoom == 0xffff_fffe {
            cdecl(h, RESET_LAYERS, &[2]);
        } else if zoom == 2 {
            cdecl(h, RESET_LAYERS, &[3]);
        }
        if word_at(h, 0x116b0).cast_signed() > 0 {
            cdecl(h, DRAW_SPHERE_JOB_TAIL, &[]);
        }
    }

    /// Draw job 3 (`0x00a53510`): unless `+0x116a8` is -2, the node's
    /// highlight (not negative) or layer 1 reset and the eamap nodes;
    /// then by the shape state byte `+0x115c3`: 1 `0x00a4ffd0` (a tail
    /// jump), 0 the node shapes and the state 2.
    pub fn abmap_job_shapes(h: &mut impl Host) {
        let node = word_at(h, 0x116a8);
        if node != 0xffff_fffe {
            if node.cast_signed() >= 0 {
                cdecl(h, DRAW_NODE_HIGHLIGHT, &[node]);
            } else {
                cdecl(h, RESET_LAYERS, &[1]);
                cdecl(h, DRAW_NODES_EAMAP, &[]);
            }
        }
        let g = abmap(h);
        let state: u8 = h.read(g.wrapping_add(0x115c3));
        if state == 1 {
            cdecl(h, DRAW_NODE_SHAPES_TAIL, &[]);
        } else if state == 0 {
            cdecl(h, DRAW_NODE_SHAPES, &[]);
            let g = abmap(h);
            h.write(g.wrapping_add(0x115c3), 2u8);
        }
    }

    /// The grid's "get node" panel state (`0x00a48280`): unless the panel
    /// word `0x01a8607e` is set, the saved state and draw hook (`+0x115b0`,
    /// `+0x115b4`) put back (`+0x115a8`, `+0x115ac`) and cleared.
    pub fn abmap_restore_state(h: &mut impl Host) {
        if h.read::<u16>(h.rebase(0x01a8_607e)) != 0 {
            return;
        }
        let s = word_at(h, 0x115b0);
        set_word_at(h, 0x115a8, s);
        set_word_at(h, 0x115b0, 0);
        let d = word_at(h, 0x115b4);
        set_word_at(h, 0x115ac, d);
        set_word_at(h, 0x115b4, 0);
    }

    pub const HELP_MESSAGE_STEP: u32 = 0x00a5_9350;
    pub const RUN_PART: u32 = 0x00a4_5930;
    /// The help screen's message part (`0x01a86060`).
    pub const HELP_PART: u32 = 0x01a8_6060;

    /// The panels' draw hook (`0x00a51700`): the help message stepped
    /// (`AbmapHelpMessageStep(0)`) and its part run (`AbmapRunPart`); the
    /// latter's `eax`.
    pub fn abmap_help_draw(h: &mut impl Host) -> u32 {
        cdecl(h, HELP_MESSAGE_STEP, &[0]);
        let part = h.rebase(HELP_PART);
        cdecl(h, RUN_PART, &[part])
    }

    /// The warp's draw hook (`0x00a4c430`): for the warp mode byte
    /// `+0x1164c` 0 or 2, [`abmap_help_draw`]'s steps.
    pub fn abmap_warp_draw(h: &mut impl Host) {
        let g = abmap(h);
        let mode: u8 = h.read(g.wrapping_add(0x1164c));
        if mode == 0 || mode == 2 {
            abmap_help_draw(h);
        }
    }

    pub const STATE_HOME_ON_NODE: u32 = 0x00a5_9e80;
    pub const MARK_EMPTY_MENUS: u32 = 0x00a5_aca0;
    /// The grid's menu (`0x01a86108`) and its pad handler.
    pub const ABMAP_MENU: u32 = 0x01a8_6108;
    pub const PAD_COMMAND_HANDLER: u32 = 0x00a5_6330;

    /// The select-homing state (`0x00a5a020`): `AbmapState_HomeOnNode()`;
    /// once the homing's progress `+0x11620` reaches 1.0 (`fcomp`; not for
    /// NaN) the empty menus marked and the menu opened: `+0x16e3` =
    /// 0x0301, `+0x16e5` = 1, `+0x2700` = 7, the pad handler at `+0x2704`,
    /// `+0x16e4` bit 3.
    pub fn abmap_select_homing(h: &mut impl Host) {
        cdecl(h, STATE_HOME_ON_NODE, &[]);
        let t = ffx_bridge::x87::load(word_at(h, 0x11620));
        // `fcomp`'s C0/C3: on only when t >= 1.0 (a NaN is unordered)
        if t.partial_cmp(&1.0).is_none_or(std::cmp::Ordering::is_lt) {
            return;
        }
        cdecl(h, MARK_EMPTY_MENUS, &[]);
        let m = global(h, ABMAP_MENU);
        h.write(m.wrapping_add(0x16e3), 0x0301u16);
        h.write(m.wrapping_add(0x16e5), 1u8);
        h.write(m.wrapping_add(0x2700), 7u16);
        let handler = h.rebase(PAD_COMMAND_HANDLER);
        h.write(m.wrapping_add(0x2704), handler);
        let f: u8 = h.read(m.wrapping_add(0x16e4));
        h.write(m.wrapping_add(0x16e4), f | 8);
    }

    /// The PC UI's work (`0x01a858b4`).
    pub const PCUI: u32 = 0x01a8_58b4;
    /// `ID3D11DeviceContext::ClearDepthStencilView`'s vtable slot.
    pub const CLEAR_DSV_SLOT: u32 = 0xd4;

    /// `0x00a373c0()` (a PC UI pass's `+0x18` step): the context (`+4`)
    /// clears the depth of the view `+0x784` and, when there is one,
    /// `+0x788` (`ClearDepthStencilView(view, D3D11_CLEAR_DEPTH, 1.0,
    /// 0)`, stdcall COM).
    pub fn pcui_clear_depth(h: &mut impl Host) {
        let ui = global(h, PCUI);
        let ctx: u32 = h.read(ui.wrapping_add(4));
        let one = 1.0f32.to_bits();
        let f: u32 = h.read(h.read::<u32>(ctx).wrapping_add(CLEAR_DSV_SLOT));
        let view: u32 = h.read(ui.wrapping_add(0x784));
        h.call(f, Conv::Stdcall, &[ctx, view, 1, one, 0]);
        let ui = global(h, PCUI);
        let second: u32 = h.read(ui.wrapping_add(0x788));
        if second != 0 {
            let ctx: u32 = h.read(ui.wrapping_add(4));
            let f: u32 = h.read(h.read::<u32>(ctx).wrapping_add(CLEAR_DSV_SLOT));
            h.call(f, Conv::Stdcall, &[ctx, second, 1, one, 0]);
        }
    }

    pub const NODE_REQUEUE_ALL: u32 = 0x00a3_9f60;

    /// `0x00a3b090(_)` (stdcall, `ret 4`; a PC UI pass's `+0x44` step):
    /// `op_node_requeue_all` of the lists `+0xab0` and `+0xab4`; the
    /// second's `eax`.
    pub fn pcui_requeue_nodes(h: &mut impl Host) -> u32 {
        let ui = global(h, PCUI);
        let a: u32 = h.read(ui.wrapping_add(0xab0));
        cdecl(h, NODE_REQUEUE_ALL, &[a]);
        let b: u32 = h.read(ui.wrapping_add(0xab4));
        cdecl(h, NODE_REQUEUE_ALL, &[b])
    }

    pub const CD_DECODE_DATA: u32 = 0x0076_d440;
    pub const HD_FILE_READ_TH_MAIN: u32 = 0x0077_86f0;
    /// The read thread's request byte (`0x01128aad`) and the CD data it
    /// decodes (`0x02310c2c`).
    pub const READ_REQUEST: u32 = 0x0112_8aad;

    /// The read task's step (`0x0076e5f0`): request 1 decodes the CD data
    /// (`cd_decode_data([0x02310c2c])`), request 3 runs the HDD file read
    /// (`hd_file_read_th_main`); either clears the request and returns its
    /// call's `eax`; any other request `r` leaves `r - 3` (signed byte) in
    /// `eax`, as the original's switch does.
    pub fn read_task_step(h: &mut impl Host) -> u32 {
        let at = h.rebase(READ_REQUEST);
        let r = i32::from(h.read::<i8>(at));
        match r {
            1 => {
                let data = global(h, 0x0231_0c2c);
                let e = cdecl(h, CD_DECODE_DATA, &[data]);
                h.write(at, 0u8);
                e
            }
            3 => {
                let e = cdecl(h, HD_FILE_READ_TH_MAIN, &[]);
                h.write(at, 0u8);
                e
            }
            _ => r.wrapping_sub(3).cast_unsigned(),
        }
    }

    pub const FFX_IS_DEBUG: u32 = 0x0088_7c50;
    pub const DEBUG_DRAW_ENV: u32 = 0x0088_7c60;
    pub const YN_BATTLE_CAPTURE_NUM: u32 = 0x0092_1910;
    pub const RCBG_PRE_ADD_FP_OBJECTS: u32 = 0x0092_2010;
    pub const YN_SCENE_PROC: u32 = 0x0092_17b0;
    /// The scene's debug overlay (`0x0092cf10`, debug builds' only path).
    pub const SCENE_DEBUG_OVERLAY: u32 = 0x0092_cf10;

    /// `Yn_SceneStep_00922060()`: in debug with the draw environment, the
    /// battle capture count; the field objects added once (`+8` of the
    /// scene list unset); the scene proc when the list has a record; in
    /// debug again the debug overlay (a tail jump).
    pub fn yn_scene_step(h: &mut impl Host) {
        if cdecl(h, FFX_IS_DEBUG, &[]) != 0 && cdecl(h, DEBUG_DRAW_ENV, &[]) != 0 {
            cdecl(h, YN_BATTLE_CAPTURE_NUM, &[]);
        }
        let mut list = global(h, SCENE_WAVES);
        if h.read::<u32>(list.wrapping_add(8)) == 0 {
            cdecl(h, RCBG_PRE_ADD_FP_OBJECTS, &[]);
            list = global(h, SCENE_WAVES);
        }
        if list != 0 && h.read::<u32>(list) != 0 {
            cdecl(h, YN_SCENE_PROC, &[]);
        }
        if cdecl(h, FFX_IS_DEBUG, &[]) != 0 && cdecl(h, DEBUG_DRAW_ENV, &[]) != 0 {
            cdecl(h, SCENE_DEBUG_OVERLAY, &[]);
        }
    }

    fn konst(h: &impl Host, a: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(a)))
    }

    pub const MAKE_NODE_MATRIX: u32 = 0x00a5_ad30;
    pub const VU0_MUL_MATRIX: u32 = 0x006e_d5c0;
    pub const SHAPE_DRAW_EAMAP: u32 = 0x007f_49e0;
    /// The sine table (`f32`, 4096 entries) and the node colours (20-byte
    /// records, the colour word first).
    pub const SINE_TABLE: u32 = 0x00c4_4bf0;
    pub const NODE_COLOURS: u32 = 0x00c8_65bc;
    /// The emulated VU0 register `vf0`'s four words (`0x00c8f508`).
    pub const VF0: u32 = 0x00c8_f508;

    /// Frame bytes `0x00a4ffd0` uses below its `ebp` (`sub esp, 0x118`).
    pub const NODE_SHAPES_FRAME: u32 = 0x118;

    /// `AbmapDrawNodeShapesPulse_00a4ffd0()` (draw job 3's tail): every
    /// node (`i16 +2` of them, 0x28 bytes from `+0x808`) but the empty
    /// ones (`u16 +6` = 0xffff) drawn as its link's eamap shape
    /// (`op_shape_draw_eamap(link +8, &request, i - count, +0x116a4)`,
    /// `i` counting down from `count - 1`) through its matrix
    /// (`AbmapMakeNodeMatrix(m, node, 1.0)`, `FFXVu0MulMatrix(m', +0x113e0,
    /// m)`); the request's colour is the node colour (`+0x115bc`'s record)
    /// scaled by a pulse (`ftol((sin(+0x1169c) + a) * b + c)`, faded in by
    /// `+0x116a0` / 16 below 16), halved, for nodes with bit 1 of `+0x22`,
    /// else 0. Returns 0x14 with no nodes, else 0xffff (the original's
    /// `eax`). `frame` is the original's (aligned) `ebp`; the request
    /// words `-0x100` (2 bytes) and `-0xec` are never written (junk read:
    /// 0, brief 2026-10-24).
    #[allow(clippy::too_many_lines)]
    pub fn abmap_draw_node_shapes_pulse(h: &mut impl Host, frame: u32) -> u32 {
        use ffx_bridge::x87::{add, ftol, load, mul};
        let l = |o: u32| frame.wrapping_sub(o);
        let g = abmap(h);
        let at = |o: u32| g.wrapping_add(o);
        let links = at(0xf828);
        let phase: u32 = h.read(at(0x1169c));
        let idx = ((phase << 11).wrapping_sub(0x3ff1).cast_signed() >> 4).cast_unsigned() & 0xfff;
        let s = load(h.read(h.rebase(SINE_TABLE).wrapping_add(4 * idx)));
        let pulse = add(
            mul(add(s, konst(h, 0x00b9_2248)), konst(h, 0x00b5_83a8)),
            konst(h, 0x00b4_c170),
        );
        let mut v = ftol(h, pulse);
        let fade: u32 = h.read(at(0x116a0));
        if fade.cast_signed() < 0x10 {
            v = (v.wrapping_mul(fade).cast_signed() / 16).cast_unsigned();
        }
        let v = v & 0xff;
        let z: u32 = h.read(h.rebase(0x00b7_b708));
        h.write(l(0xf8), 0u32);
        h.write(l(0xf4), 0u32);
        let kind = u32::from(h.read::<u8>(at(0x115bc)));
        let c: u32 = h.read(h.rebase(NODE_COLOURS).wrapping_add(20 * kind));
        let part =
            |c: u32, shift: u32| ((((c >> shift) & 0xff).wrapping_mul(v)) >> 7) & 0x01ff_ffff;
        let packed = (((part(c, 24) << 8).wrapping_add(part(c, 16)) << 8).wrapping_add(part(c, 8))
            << 8)
            .wrapping_add(part(c, 0));
        h.write(l(0x104), packed);
        let (req, xform, node_m, base) = (l(0x100), l(0x88), l(0xc8), l(0x48));
        h.write(l(0xf0), xform);
        h.write(l(0xe4), base);
        let vf0: [u32; 4] = h.read(h.rebase(VF0));
        h.write(base, vf0);
        h.write(base.wrapping_add(0x10), vf0);
        h.write(base.wrapping_add(0x20), vf0);
        let one = 1.0f32.to_bits();
        h.write(
            base.wrapping_add(0x30),
            [vf0[0], vf0[1], ffx_bridge::x87::store(load(z)), one],
        );
        h.write(l(0xe8), 0u8);
        h.write(l(0xfe), 0x14u16);
        let count = i32::from(h.read::<i16>(at(2))).cast_unsigned();
        if count == 0 {
            return 0x14;
        }
        let mut left = count;
        let mut node = at(0x808);
        loop {
            left = left.wrapping_sub(1);
            let e = h.read::<u16>(node.wrapping_add(6));
            if e != 0xffff {
                h.call_sym(MAKE_NODE_MATRIX, Conv::Cdecl, &[node_m, node, one]);
                let g = abmap(h);
                h.call_sym(
                    VU0_MUL_MATRIX,
                    Conv::Cdecl,
                    &[xform, g.wrapping_add(0x113e0), node_m],
                );
                let e = u32::from(h.read::<u16>(node.wrapping_add(6)));
                let link = links.wrapping_add(48 * e);
                let lit = h.read::<u8>(node.wrapping_add(0x22)) & 2 != 0;
                let colour = if lit { packed } else { 0 };
                let halved = (colour >> 1) & 0x7f7f_7f7f;
                h.write(l(0xfc), halved);
                let g = abmap(h);
                let count_now = i32::from(h.read::<i16>(g.wrapping_add(2))).cast_unsigned();
                let slot = left.wrapping_sub(count_now);
                let key: u32 = h.read(g.wrapping_add(0x116a4));
                let shape: u32 = h.read(link.wrapping_add(8));
                h.call_sym(SHAPE_DRAW_EAMAP, Conv::Cdecl, &[shape, req, slot, key]);
            }
            node = node.wrapping_add(0x28);
            if left == 0 {
                return 0xffff;
            }
        }
    }

    pub const MAKE_LINK_MATRIX: u32 = 0x00a5_a360;
    pub const DRAW_EIABM_SPHERE: u32 = 0x00a6_8140;
    /// The node sphere colours (7 words, `0x00c8659c`).
    pub const SPHERE_COLOURS: u32 = 0x00c8_659c;
    /// Frame bytes `0x00a50b50` uses below its `ebp` (`sub esp, 0x1c8`).
    pub const NODE_SPHERES_FRAME: u32 = 0x1c8;

    /// `AbmapDrawNodeSpheresLit_00a50b50()` (draw job 2's tail): the seven
    /// sphere colours dimmed by a pulse (`0x6c - ftol(sin(x) * a * b)`,
    /// `x` from `+0x11698` and the first node's `+0x26`) into four words
    /// each (packed, then the green, blue and alpha parts); then for every
    /// node with a link (`u16 +6`) whose `+0x14` word is not 0x1000, each
    /// of the 7 sphere slots the mask `+0x115c9 & node +0x21` has (bit
    /// `k`) drawn (`op1_md_draw_eiabm_sphe([0x023057ec], &request, i, w)`
    /// with `i = count - left - 1`, `w = -1 - k`) with the node matrix
    /// (`AbmapMakeNodeMatrix(m, node, link +0x10)`), the slot's link
    /// matrix (`AbmapMakeLinkMatrix(m, node, link +0x14 + 4k, s)`) and the
    /// grid's (`FFXVu0MulMatrix`). No result (its caller, draw job 2,
    /// reads none). `frame` is the
    /// original's aligned `ebp`; the request words `-0x180`, `-0x17c` and
    /// the byte `-0x1a5` are never written (junk read: 0, brief
    /// 2026-10-24).
    #[allow(clippy::too_many_lines)]
    pub fn abmap_draw_node_spheres_lit(h: &mut impl Host, frame: u32) {
        use ffx_bridge::x87::{ftol, load, mul};
        let l = |o: u32| frame.wrapping_sub(o);
        let g = abmap(h);
        let count = i32::from(h.read::<i16>(g.wrapping_add(2))).cast_unsigned();
        let nodes = g.wrapping_add(0x808);
        let phase: u32 = h.read(g.wrapping_add(0x11698));
        let first = u32::from(h.read::<u16>(nodes.wrapping_add(0x26)));
        let idx = ((phase << 11).wrapping_add(first).cast_signed() >> 4).cast_unsigned() & 0xfff;
        let s = load(h.read(h.rebase(SINE_TABLE).wrapping_add(4 * idx)));
        let v = ftol(h, mul(mul(s, konst(h, 0x00b5_9870)), konst(h, 0x00b7_b718)));
        let dim = 0x6cu32.wrapping_sub(v) & 0xff;
        let recs = l(0x138);
        for k in 0..7u32 {
            let c: u32 = h.read(h.rebase(SPHERE_COLOURS).wrapping_add(4 * k));
            let part = |shift: u32| ((((c >> shift) & 0xff).wrapping_mul(dim)) >> 7) & 0x01ff_ffff;
            let (a, b, gg, r) = (part(24), part(16), part(8), part(0));
            let packed = (((a << 8).wrapping_add(b) << 8).wrapping_add(gg) << 8).wrapping_add(r);
            h.write(recs.wrapping_add(0x10 * k), [packed, gg, b, a]);
        }
        let (req, xform, node_m, base) = (l(0x1b0), l(0xc8), l(0x88), l(0x48));
        let vf0: [u32; 4] = h.read(h.rebase(VF0));
        let one = 1.0f32.to_bits();
        h.write(l(0x190), xform);
        h.write(l(0x16c), base);
        h.write(l(0x184), node_m);
        for o in [
            0x194u32, 0x174, 0x178, 0x1a4, 0x1a0, 0x19c, 0x170, 0x188, 0x18c,
        ] {
            h.write(l(o), 0u32);
        }
        h.write(l(0x1a6), 0u8);
        h.write(l(0x1ac), h.rebase(0x0174_0870));
        h.write(base, vf0);
        h.write(base.wrapping_add(0x10), vf0);
        h.write(base.wrapping_add(0x20), vf0);
        h.write(base.wrapping_add(0x30), [vf0[0], vf0[1], vf0[2], one]);
        let mut left = count;
        let mut node = nodes;
        while left != 0 {
            let e = h.read::<u16>(node.wrapping_add(6));
            left = left.wrapping_sub(1);
            if e != 0xffff {
                let g = abmap(h);
                let link = g.wrapping_add(0xf828 + 48 * u32::from(e));
                if h.read::<u16>(link.wrapping_add(0x14)) != 0x1000 {
                    let mut mask = h.read::<u8>(g.wrapping_add(0x115c9))
                        & h.read::<u8>(node.wrapping_add(0x21));
                    h.write(l(0x1a8), 0x8064u16);
                    for k in 0..7u32 {
                        if mask & 1 != 0 {
                            let colour: u32 = h.read(recs.wrapping_add(0x10 * k));
                            h.write(base.wrapping_add(0x38), one);
                            h.write(l(0x198), colour);
                            h.write(l(0x195), 0x80u8);
                            h.write(req, 0x2cu32);
                            let scale =
                                ffx_bridge::x87::store(load(h.read(link.wrapping_add(0x10))));
                            h.call_sym(MAKE_NODE_MATRIX, Conv::Cdecl, &[node_m, node, scale]);
                            let kk = ffx_bridge::x87::store(load(h.read(h.rebase(0x00b7_b5d4))));
                            let slot = link.wrapping_add(0x14 + 4 * k);
                            h.call_sym(MAKE_LINK_MATRIX, Conv::Cdecl, &[node_m, node, slot, kk]);
                            let g = abmap(h);
                            h.call_sym(
                                VU0_MUL_MATRIX,
                                Conv::Cdecl,
                                &[xform, g.wrapping_add(0x113e0), node_m],
                            );
                            let g = abmap(h);
                            let count_now =
                                i32::from(h.read::<i16>(g.wrapping_add(2))).cast_unsigned();
                            let i = count_now.wrapping_sub(left).wrapping_sub(1);
                            let sphere: u32 = h.read(h.rebase(0x0230_57ec));
                            let which = (-1i32 - k.cast_signed()).cast_unsigned();
                            h.call_sym(DRAW_EIABM_SPHERE, Conv::Cdecl, &[sphere, req, i, which]);
                        }
                        mask >>= 1;
                    }
                }
            }
            node = node.wrapping_add(0x28);
        }
    }
}
