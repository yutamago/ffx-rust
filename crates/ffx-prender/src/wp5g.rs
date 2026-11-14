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
}
