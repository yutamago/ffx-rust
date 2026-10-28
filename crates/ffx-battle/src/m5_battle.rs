//! M5 stage 1, list `battle`: the battle code the static call graph
//! reaches that no coverage run did. Most of it is the battle's debug
//! side: the debug menu's helpers (full items, full party, weak
//! enemies), the scene picker's list walk, the debug status line, the
//! environment file and its writer; the rest is the PS2 CD layer the
//! movie player keeps (the disc checks and the sector read with its
//! one-sector cache) and a few small battle helpers.
//!
//! Callees that are still the game's (and the ported ones in other
//! modules) are called through a [`Call`] callback to the game's code,
//! CRT imports through their import table slots.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    pub use crate::exec::bridge::Call;

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = mem.rebase(sym);
        call(mem, f, args)
    }

    /// Calls the CRT import whose import table slot is `slot`.
    fn import<M: Mem>(mem: &mut M, call: Call<'_, M>, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = mem.read(mem.rebase(slot));
        call(mem, f, args)
    }

    fn r8(mem: &impl Mem, sym: u32) -> u8 {
        mem.read(mem.rebase(sym))
    }
    fn r16(mem: &impl Mem, sym: u32) -> u16 {
        mem.read(mem.rebase(sym))
    }
    fn r32(mem: &impl Mem, sym: u32) -> u32 {
        mem.read(mem.rebase(sym))
    }
    fn w8(mem: &mut impl Mem, sym: u32, v: u8) {
        let at = mem.rebase(sym);
        mem.write(at, v);
    }
    fn w32(mem: &mut impl Mem, sym: u32, v: u32) {
        let at = mem.rebase(sym);
        mem.write(at, v);
    }

    // ---- callees (ffx.sym addresses) ----

    /// `sprintf`'s import table slot.
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// `memcpy`'s import thunk.
    pub const MEMCPY: u32 = 0x0094_9256;
    pub const TO_GET_FFX_LANG_JMP: u32 = 0x008a_c2f0;
    pub const TK_MENU_ITEM_REFRESH: u32 = 0x008c_8e60;
    /// `FUN_00a44ea0`, `MsDebugFullItem`'s tail call.
    pub const FULL_ITEM_TAIL: u32 = 0x00a4_4ea0;
    pub const FFX_IS_DEBUG: u32 = 0x0088_7c50;
    pub const MS_SET_COUNTRY_NAME: u32 = 0x0078_2c00;
    pub const PC_FILE_TBL_SET: u32 = 0x00a4_2b70;
    pub const SCE_CD_STATUS: u32 = 0x00a4_4810;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const AUDIO_STOP: u32 = 0x0077_b170;
    pub const SCE_CD_DISK_READY: u32 = 0x0062_f550;
    pub const READ_ERR_DISP_SET: u32 = 0x0076_e5e0;
    pub const MOVIE_WAIT: u32 = 0x0077_daa0;
    pub const MOVIE_CSC: u32 = 0x0077_d5e0;
    pub const TK_SYNC_V: u32 = 0x0088_7f00;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const MOVIE_ERR: u32 = 0x0076_efa0;
    pub const HD_FILE_READ_NO_WAIT: u32 = 0x0077_85e0;
    pub const HDD_STATE_CLEAR: u32 = 0x0077_7420;
    pub const SCE_CD_READ: u32 = 0x00a4_47c0;
    /// `0x00a447a0`, the read's reset (Ghidra names it `sceCdRead` too).
    pub const SCE_CD_READ_RESET: u32 = 0x00a4_47a0;
    pub const CD_DISC_WAIT: u32 = 0x0077_6f60;
    pub const MS_SET_SAVE_PLY_JOIN: u32 = 0x0078_69b0;
    pub const MS_SAVE_ENCOUNT_CHECK: u32 = 0x0078_5b10;
    pub const MS_GET_CHR_TOP: u32 = 0x0079_5970;
    pub const MS_GET_CHR: u32 = 0x0079_4020;
    pub const MS_SET_CHR_WEAK: u32 = 0x0078_d7f0;
    pub const FIOS_OPEN: u32 = 0x0067_9730;
    pub const SCE_LSEEK: u32 = 0x0062_f8e0;
    pub const TK_MALLOC: u32 = 0x0088_e650;
    pub const SCE_READ: u32 = 0x0062_fc00;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const TK_FREE: u32 = 0x0088_d520;
    pub const RND: u32 = 0x0079_89a0;
    pub const MS_COPY_POINTS_OFFSET: u32 = 0x007a_beb0;
    pub const MS_CAMERA_FREE: u32 = 0x007b_d000;
    pub const MS_VECTOR_RECT2_POLAR: u32 = 0x007c_4990;
    pub const MS_BTL_LIST_FIELD: u32 = 0x0079_d190;
    pub const MS_BTL_LIST_ENCOUNT: u32 = 0x0079_d170;
    pub const MS_BTL_LIST_GROUP: u32 = 0x0079_d210;
    pub const MS_DEBUG_SCENE_WALK: u32 = 0x007c_5dd0;
    pub const MS_DEBUG_SCENE_GROUP_BASE: u32 = 0x007c_5f00;
    pub const MS_GET_SAVE_PLY_JOIN: u32 = 0x0078_5380;
    pub const SCE_WRITE: u32 = 0x0062_fdc0;

    // ---- globals ----

    /// `g_SaveItemIds` (0x70 x u16) and `g_SaveItemCounts` (0x70 x u8).
    pub const SAVE_ITEM_IDS: u32 = 0x0113_095c;
    pub const SAVE_ITEM_COUNTS: u32 = 0x0113_0b5c;
    /// `g_BtlExeKeep + 1`: the environment file's kind (1, 2).
    pub const ENV_KIND: u32 = 0x0112_a8f6;
    /// The country name buffer `MsSetCountryName` fills.
    pub const COUNTRY_NAME: u32 = 0x0231_1420;
    /// The two environment file formats (`.rdata`).
    pub const ENV_FORMAT_1: u32 = 0x00b5_46e0;
    pub const ENV_FORMAT_2: u32 = 0x00b5_4710;
    /// The CD layer's messages (`.rdata`).
    pub const MSG_DISC_STATUS: u32 = 0x00b5_1e64;
    pub const MSG_DISC_OPEN: u32 = 0x00b5_1e08;
    pub const MSG_DISC_CLOSED: u32 = 0x00b5_1e1c;
    pub const MSG_READ_FILE: u32 = 0x00b5_34cc;
    pub const MSG_READ_SOURCE: u32 = 0x00b5_34ac;
    pub const MSG_READ_ALIGN: u32 = 0x00b5_34d8;
    pub const MSG_READ_ERROR: u32 = 0x00b5_3508;
    pub const MSG_READ_RETRY_1: u32 = 0x00b5_3524;
    pub const MSG_READ_RETRY_2: u32 = 0x00b5_3544;
    /// `g_CdDiscOpen`: set when a disc check saw the tray open.
    pub const CD_DISC_OPEN: u32 = 0x0112_a4ec;
    /// The sector read's state: the request (sector, count, buffer,
    /// mode), the last read (first sector, count) and the last sector's
    /// copy.
    pub const CD_REQ_SECTOR: u32 = 0x0112_a4dc;
    pub const CD_LAST_SECTOR: u32 = 0x0112_a4e0;
    pub const CD_REQ_COUNT: u32 = 0x0112_a4e4;
    pub const CD_LAST_COUNT: u32 = 0x0112_a4e8;
    pub const CD_REQ_BUFFER: u32 = 0x0112_a4f0;
    pub const CD_LAST_BLOCK: u32 = 0x0112_a4f4;
    pub const CD_REQ_MODE: u32 = 0x0112_a4f8;
    /// `g_Hdd_State` (2 words).
    pub const HDD_STATE: u32 = 0x0112_a4fc;
    /// `g_PlySave` (18 x 0x94 bytes from `0x01132044`).
    pub const PLY_SAVE: u32 = 0x0113_2044;
    /// `g_BtlEffectWait` (2 x u8).
    pub const EFFECT_WAIT: u32 = 0x0112_ca08;
    /// `g_SepTransState` and `g_BtlSepTrans`; the queue's 12-byte records.
    pub const SEP_TRANS_STATE: u32 = 0x0113_6188;
    pub const SEP_TRANS: u32 = 0x0113_618c;
    pub const SEP_TRANS_QUEUE: u32 = 0x0113_60d0;
    /// The battle's script records and position file; the ground byte.
    pub const BTL_SCRIPT_RECORDS: u32 = 0x0112_a990;
    pub const BTL_POS_FILE: u32 = 0x0112_a9b0;
    pub const BTL_GROUND_GROUP: u32 = 0x0112_bdee;
    /// `g_AtelCamera`: 4 x 3 camera ids.
    pub const ATEL_CAMERA: u32 = 0x0113_6f88;
    /// `g_Camera`.
    pub const CAMERA: u32 = 0x0231_1430;
    /// The debug scene picker: field index, the field's encounter list
    /// and group entries, the picked encounter (global, group, in group),
    /// the scene's field and formation numbers, the list pointers and the
    /// formation's name.
    pub const DBG_FIELD: u32 = 0x00c4_4248;
    pub const BTL_FIELD_COUNT: u32 = 0x0112_a9d0;
    pub const DBG_ENCOUNT: u32 = 0x0113_9e44;
    pub const DBG_GROUP: u32 = 0x0113_9e48;
    pub const DBG_IN_GROUP: u32 = 0x0113_9e4c;
    pub const DBG_SCENE_FIELD: u32 = 0x0113_9e50;
    pub const DBG_SCENE_FORMATION: u32 = 0x0113_9e54;
    pub const DBG_FIELD_REC: u32 = 0x0113_9e94;
    pub const DBG_ENCOUNT_LIST: u32 = 0x0113_9e98;
    pub const DBG_GROUP_REC: u32 = 0x0113_9e9c;
    pub const DBG_FORMATION: u32 = 0x0113_9ea0;
    pub const DBG_SCENE_NAME: u32 = 0x0113_9ea4;
    /// `"%s"`, `"%s_%02d"` and the two fixed scene names (`.rdata`,
    /// `.data`).
    pub const FMT_S: u32 = 0x00b0_e160;
    pub const FMT_SCENE: u32 = 0x00b5_4788;
    pub const SCENE_FIXED_A: u32 = 0x00c4_4a00;
    pub const SCENE_FIXED_B: u32 = 0x00c4_4a08;
    /// `g_SaveInParty` (3 x u8).
    pub const SAVE_IN_PARTY: u32 = 0x0113_07e8;

    /// A 5-byte `jmp target` thunk: `target(args)`, its `eax`.
    pub fn jmp_thunk<M: Mem>(mem: &mut M, call: Call<'_, M>, target: u32, args: &[u32]) -> u32 {
        game(mem, call, target, args)
    }

    /// The jump thunks' targets: `FFXGetAsiaMode` (`0x007826a0`),
    /// `MsEtEffectRead` (`0x00782740`), and the magic DLLs' imports
    /// `MsSetChrPos`, `MsSetChrVisible`, `MsEffectSetMotion`
    /// (`0x0092e980`, `0x0092e990`, `0x0092eb10`).
    pub const FFX_GET_ASIA_MODE: u32 = 0x0088_7c20;
    pub const MS_ET_EFFECT_READ: u32 = 0x0079_e6a0;
    pub const MS_SET_CHR_POS: u32 = 0x0079_6150;
    pub const MS_SET_CHR_VISIBLE: u32 = 0x0079_6660;
    pub const MS_EFFECT_SET_MOTION: u32 = 0x0078_82c0;

    /// `MsDebugFullItem()` (`0x007849d0`): every item 0x2000..0x206f
    /// into the inventory with 99 (slot `i` for item `i`; where the item
    /// already is, that slot gets 99 too), and in language 0xb item 0x2065
    /// becomes 10 of item 0x2000; then `TkMenuItemRefresh(0)` and a tail
    /// call of `0x00a44ea0`, whose `eax` it returns.
    pub fn ms_debug_full_item<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        let id_at = |mem: &M, i: u32| mem.rebase(SAVE_ITEM_IDS) + 2 * i;
        let n_at = |mem: &M, i: u32| mem.rebase(SAVE_ITEM_COUNTS) + i;
        for i in 0..0x70u32 {
            let item = i + 0x2000;
            for j in 0..0x70u32 {
                let id = mem.read::<u16>(id_at(mem, j));
                if u32::from(id) == item {
                    let at = n_at(mem, j);
                    mem.write(at, 99u8);
                    break;
                }
                if game(mem, &mut *call, TO_GET_FFX_LANG_JMP, &[]) == 0xb
                    && mem.read::<u16>(id_at(mem, j)) == 0x2065
                {
                    let (a, b) = (id_at(mem, j), n_at(mem, j));
                    mem.write(a, 0x2000u16);
                    mem.write(b, 10u8);
                    break;
                }
            }
            let (a, b) = (id_at(mem, i), n_at(mem, i));
            #[allow(clippy::cast_possible_truncation)] // below 0x2070
            mem.write(a, item as u16);
            mem.write(b, 99u8);
            if game(mem, &mut *call, TO_GET_FFX_LANG_JMP, &[]) == 0xb
                && mem.read::<u16>(a) == 0x2065
            {
                mem.write(a, 0x2000u16);
                mem.write(b, 10u8);
            }
        }
        game(mem, &mut *call, TK_MENU_ITEM_REFRESH, &[0]);
        game(mem, call, FULL_ITEM_TAIL, &[])
    }

    /// `MsEnvInit()` (`0x00782770`): in a debug build the country name,
    /// then for environment kind 1 or 2 its file name printed into the
    /// frame buffer at `buf` (0x84 bytes at the original's `ebp - 0x84`)
    /// and handed to `PcFileTblSet(buf, 0xd)`.
    pub fn ms_env_init<M: Mem>(mem: &mut M, call: Call<'_, M>, buf: u32) {
        if game(mem, &mut *call, FFX_IS_DEBUG, &[]) == 0 {
            return;
        }
        let name = mem.rebase(COUNTRY_NAME);
        game(mem, &mut *call, MS_SET_COUNTRY_NAME, &[name]);
        let fmt = match r8(mem, ENV_KIND) {
            1 => ENV_FORMAT_1,
            2 => ENV_FORMAT_2,
            _ => return,
        };
        let fmt = mem.rebase(fmt);
        import(mem, &mut *call, SPRINTF_SLOT, &[buf, fmt, name]);
        game(mem, call, PC_FILE_TBL_SET, &[buf, 0xd]);
    }

    /// `0x00776f60(ctx)` (`CdDiscWait`, name ours): with no hard disk
    /// (`ctx + 0x5075d` clear) and the tray open (`sceCdStatus() == 1`),
    /// the audio is stopped and the movie loop waits until the disc is
    /// ready and the tray closed: 0. A status other than 1: printed, 1.
    pub fn cd_disc_wait<M: Mem>(mem: &mut M, call: Call<'_, M>, ctx: u32) -> u32 {
        if mem.read::<u8>(ctx.wrapping_add(0x0005_075d)) != 0 {
            return 0;
        }
        let status = game(mem, &mut *call, SCE_CD_STATUS, &[]);
        if status != 1 {
            let msg = mem.rebase(MSG_DISC_STATUS);
            game(mem, &mut *call, DBG_PRINTF, &[msg, status]);
            return 1;
        }
        w8(mem, CD_DISC_OPEN, 1);
        let msg = mem.rebase(MSG_DISC_OPEN);
        game(mem, &mut *call, DBG_PRINTF, &[msg]);
        game(mem, &mut *call, AUDIO_STOP, &[ctx]);
        wait_disc_closed(mem, call, None)
    }

    /// The loop [`cd_disc_wait`] and [`cd_open_check_sub`] share: until
    /// `sceCdDiskReady(1) == 2` with the tray closed, the error display
    /// and a frame (`TkSyncV(0)` for `sync` 0, else `movie_wait`, and for
    /// `None` also `movie_csc`).
    fn wait_disc_closed<M: Mem>(mem: &mut M, call: Call<'_, M>, sync: Option<u32>) -> u32 {
        loop {
            if game(mem, &mut *call, SCE_CD_DISK_READY, &[1]) == 2
                && game(mem, &mut *call, SCE_CD_STATUS, &[]) != 1
            {
                let msg = mem.rebase(MSG_DISC_CLOSED);
                game(mem, &mut *call, DBG_PRINTF, &[msg]);
                return 0;
            }
            game(mem, &mut *call, READ_ERR_DISP_SET, &[1]);
            match sync {
                Some(0) => {
                    game(mem, &mut *call, TK_SYNC_V, &[0]);
                }
                Some(_) => {
                    game(mem, &mut *call, MOVIE_WAIT, &[]);
                }
                None => {
                    game(mem, &mut *call, MOVIE_WAIT, &[]);
                    game(mem, &mut *call, MOVIE_CSC, &[]);
                }
            }
        }
    }

    /// `CdOpenCheck_sub(sync)` (`0x00777000`): like [`cd_disc_wait`]
    /// without the context's checks; it waits with `TkSyncV(0)` when
    /// `sync` is 0, else with `movie_wait`.
    pub fn cd_open_check_sub<M: Mem>(mem: &mut M, call: Call<'_, M>, sync: u32) -> u32 {
        let status = game(mem, &mut *call, SCE_CD_STATUS, &[]);
        if status != 1 {
            let msg = mem.rebase(MSG_DISC_STATUS);
            game(mem, &mut *call, DBG_PRINTF, &[msg, status]);
            return 1;
        }
        let msg = mem.rebase(MSG_DISC_OPEN);
        game(mem, &mut *call, DBG_PRINTF, &[msg]);
        wait_disc_closed(mem, call, Some(sync))
    }

    /// `0x00777080(sector, count, buf, mode, ctx)` (`CdReadSectors`,
    /// name ours): reads `count` sectors of 0x800 bytes from `sector`
    /// into `buf`. A read that starts at the sector after the last one
    /// read takes that sector from its copy (`CD_LAST_BLOCK`) when no disc
    /// check came in between; the original then stores the advanced
    /// sector into its own argument slot (`slots`, the caller's argument
    /// words). From the hard disk (`ctx + 0x5075d`) through
    /// `hd_file_read_no_wait` (its `eax`); from the disc through
    /// `sceCdRead`, retried with a disc wait and a movie frame, every
    /// 60 tries with a reset, until it succeeds (1).
    pub fn cd_read_sectors<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        slots: u32,
        args: [u32; 5],
    ) -> u32 {
        let [mut sector, mut count, mut buf, mode, ctx] = args;
        w32(mem, CD_REQ_BUFFER, buf);
        w32(mem, CD_REQ_MODE, mode);
        w32(mem, CD_REQ_SECTOR, sector);
        w32(mem, CD_REQ_COUNT, count);
        if r8(mem, CD_DISC_OPEN) == 0
            && sector
                == r32(mem, CD_LAST_COUNT)
                    .wrapping_sub(1)
                    .wrapping_add(r32(mem, CD_LAST_SECTOR))
        {
            game(mem, &mut *call, FLUSH_CACHE, &[0]);
            let from = r32(mem, CD_LAST_BLOCK);
            for k in 0..0x200u32 {
                let v: u32 = mem.read(from.wrapping_add(4 * k));
                mem.write(buf.wrapping_add(4 * k), v);
            }
            buf = buf.wrapping_add(0x800);
            count = count.wrapping_sub(1);
            sector = sector.wrapping_add(1);
            mem.write(slots, sector);
        }
        w8(mem, CD_DISC_OPEN, 1);
        w32(mem, CD_LAST_COUNT, count);
        w32(mem, CD_LAST_SECTOR, sector);
        let bytes = count << 11;
        if mem.read::<u8>(ctx.wrapping_add(0x0005_075d)) != 0 {
            w32(
                mem,
                CD_LAST_BLOCK,
                buf.wrapping_sub(0x800).wrapping_add(bytes),
            );
            let file: u32 = mem.read(ctx.wrapping_add(0x0005_0544));
            if file.cast_signed() < 0 {
                let (f, s) = (mem.rebase(MSG_READ_FILE), mem.rebase(MSG_READ_SOURCE));
                game(mem, &mut *call, MOVIE_ERR, &[f, s, 0xc9]);
            }
            let file: u32 = mem.read(ctx.wrapping_add(0x0005_0544));
            let kind = i32::from(mem.read::<i8>(ctx.wrapping_add(0x0005_0764))).cast_unsigned();
            return game(
                mem,
                call,
                HD_FILE_READ_NO_WAIT,
                &[sector, buf, bytes, kind, file],
            );
        }
        game(mem, &mut *call, READ_ERR_DISP_SET, &[0]);
        loop {
            for _ in 0..0x3c {
                if game(mem, &mut *call, SCE_CD_DISK_READY, &[1]) == 2 {
                    game(mem, &mut *call, HDD_STATE_CLEAR, &[]);
                    let busy = r32(mem, HDD_STATE + 4);
                    w32(
                        mem,
                        CD_LAST_BLOCK,
                        bytes.wrapping_sub(0x800).wrapping_add(buf),
                    );
                    w32(mem, HDD_STATE, 0);
                    if busy == 0 {
                        if buf & 0x3f != 0 {
                            let msg = mem.rebase(MSG_READ_ALIGN);
                            game(mem, &mut *call, DBG_PRINTF, &[msg, buf]);
                        }
                        game(mem, &mut *call, FLUSH_CACHE, &[0]);
                        let first: u32 = mem.read(slots);
                        let mode: u32 = mem.read(slots + 0xc);
                        if game(mem, &mut *call, SCE_CD_READ, &[first, count, buf, mode]) == 1 {
                            return 1;
                        }
                    }
                    let msg = mem.rebase(MSG_READ_ERROR);
                    game(mem, &mut *call, DBG_PRINTF, &[msg]);
                    game(mem, &mut *call, READ_ERR_DISP_SET, &[2]);
                }
                let ctx: u32 = mem.read(slots + 0x10);
                game(mem, &mut *call, CD_DISC_WAIT, &[ctx]);
                game(mem, &mut *call, MOVIE_WAIT, &[]);
                game(mem, &mut *call, MOVIE_CSC, &[]);
            }
            let (a, b) = (mem.rebase(MSG_READ_RETRY_1), mem.rebase(MSG_READ_RETRY_2));
            game(mem, &mut *call, DBG_PRINTF, &[a]);
            game(mem, &mut *call, DBG_PRINTF, &[b]);
            game(mem, &mut *call, SCE_CD_READ_RESET, &[]);
            game(mem, &mut *call, MOVIE_WAIT, &[]);
            game(mem, &mut *call, MOVIE_CSC, &[]);
        }
    }

    /// `MsDebugFullMember()` (`0x00784ac0`): every character 0..0x11
    /// joins (`MsSetSavePlyJoin(i, 1)`); then the tail call of
    /// `MsSaveEncountCheck`, whose `eax` it returns.
    pub fn ms_debug_full_member<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        for i in 0..0x12 {
            game(mem, &mut *call, MS_SET_SAVE_PLY_JOIN, &[i, 1]);
        }
        game(mem, call, MS_SAVE_ENCOUNT_CHECK, &[])
    }

    /// `0x00784b00()` (`MsDebugFullGil`, name ours): every character's
    /// save record gets the debug maximum of HP and MP (99999, 9999 at
    /// `+0x1c`/`+0x20`, the bases at `+4`/`+8` and `+0x24`/`+0x28`) and
    /// -1 at `+0xc`, `+0x10`.
    pub fn ms_debug_full_stats(mem: &mut impl Mem) {
        let first = mem.rebase(0x0113_2078);
        let end = mem.rebase(0x0113_2ae0);
        let mut p = first;
        while p < end {
            mem.write(p - 0x18, 99_999u32);
            mem.write(p, 99_999u32);
            mem.write(p - 0x14, 9999u32);
            mem.write(p + 4, 9999u32);
            mem.write(p + 8, 99_999u32);
            mem.write(p + 0xc, 9999u32);
            mem.write(p - 0x10, u32::MAX);
            mem.write(p - 0xc, u32::MAX);
            p += 0x94;
        }
    }

    /// `0x00784b80()` (`MsDebugWeakAll`, name ours): in a battle, every
    /// actor 0..0x1e gets `+0x5d0` = 1 and `MsSetChrWeak(i, -1)`.
    pub fn ms_debug_weak_all<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        if game(mem, &mut *call, MS_GET_CHR_TOP, &[]) == 0 {
            return;
        }
        for i in 0..0x1f {
            let actor = game(mem, &mut *call, MS_GET_CHR, &[i]);
            mem.write(actor.wrapping_add(0x5d0), 1u32);
            game(mem, &mut *call, MS_SET_CHR_WEAK, &[i, u32::MAX]);
        }
    }

    /// The actors `first..end` get `+0x5d0` = 1 when `on`, else their
    /// `+0x594`, and `MsSetChrWeak(i, -1)`, in a battle.
    fn weak_range<M: Mem>(mem: &mut M, call: Call<'_, M>, first: u32, end: u32, on: u32) {
        if game(mem, &mut *call, MS_GET_CHR_TOP, &[]) == 0 {
            return;
        }
        for i in first..end {
            let actor = game(mem, &mut *call, MS_GET_CHR, &[i]);
            let v = if on != 0 {
                1
            } else {
                mem.read::<u32>(actor.wrapping_add(0x594))
            };
            mem.write(actor.wrapping_add(0x5d0), v);
            game(mem, &mut *call, MS_SET_CHR_WEAK, &[i, u32::MAX]);
        }
    }

    /// `0x00784bc0(on)` (`MsDebugWeakMonsters`, name ours): the monster
    /// actors 0x14..0x1b ([`weak_range`]).
    pub fn ms_debug_weak_monsters<M: Mem>(mem: &mut M, call: Call<'_, M>, on: u32) {
        weak_range(mem, call, 0x14, 0x1c, on);
    }

    /// `0x00784c80(on)` (`MsDebugWeakParty`, name ours): the party
    /// actors 0..0x11 ([`weak_range`]).
    pub fn ms_debug_weak_party<M: Mem>(mem: &mut M, call: Call<'_, M>, on: u32) {
        weak_range(mem, call, 0, 0x12, on);
    }

    /// `0x00784c20()` (`MsDebugAllItemsOne`, name ours): slot `i` holds
    /// one of item `0x2000 + i`.
    pub fn ms_debug_all_items_one(mem: &mut impl Mem) {
        for i in 0..0x70u32 {
            let (a, b) = (
                mem.rebase(SAVE_ITEM_IDS) + 2 * i,
                mem.rebase(SAVE_ITEM_COUNTS) + i,
            );
            #[allow(clippy::cast_possible_truncation)] // below 0x2070
            mem.write(a, (i + 0x2000) as u16);
            mem.write(b, 1u8);
        }
    }

    /// `0x00784cd0()` (`MsDebugClearItems`, name ours): every slot empty
    /// (item 0xff, count 0).
    pub fn ms_debug_clear_items(mem: &mut impl Mem) {
        for i in 0..0x70u32 {
            let (a, b) = (
                mem.rebase(SAVE_ITEM_IDS) + 2 * i,
                mem.rebase(SAVE_ITEM_COUNTS) + i,
            );
            mem.write(a, 0xffu16);
            mem.write(b, 0u8);
        }
    }

    /// `0x00784c40()` (`MsDebugAbilityOne`, name ours): in each of the
    /// seven save records from `0x011320bc` (stride 0x94), every one of
    /// the 0x14 words that isn't 0 becomes 1.
    pub fn ms_debug_ability_one(mem: &mut impl Mem) {
        let end = mem.rebase(0x0113_24c8);
        let mut rec = mem.rebase(0x0113_20bc);
        while rec < end {
            for k in 0..0x14 {
                let at = rec + 2 * k;
                if mem.read::<u16>(at) != 0 {
                    mem.write(at, 1u16);
                }
            }
            rec += 0x94;
        }
    }

    /// `0x00790430(name, dst)` (`PasswordFileRead`, name ours): the file
    /// `name` read whole into a `TkMalloc` buffer and copied to `dst`;
    /// its size, 0 when it doesn't open.
    pub fn password_file_read<M: Mem>(mem: &mut M, call: Call<'_, M>, name: u32, dst: u32) -> u32 {
        let f = game(mem, &mut *call, FIOS_OPEN, &[name, 1]);
        if f == 0 {
            return 0;
        }
        let size = game(mem, &mut *call, SCE_LSEEK, &[f, 0, 2]);
        game(mem, &mut *call, SCE_LSEEK, &[f, 0, 0]);
        let buf = game(mem, &mut *call, TK_MALLOC, &[size]);
        game(mem, &mut *call, SCE_READ, &[f, buf, size]);
        game(mem, &mut *call, SCE_CLOSE, &[f]);
        game(mem, &mut *call, MEMCPY, &[dst, buf, size]);
        game(mem, call, TK_FREE, &[buf]);
        size
    }

    /// `MsCheckChrBtlPosAll()` (`0x007915f0`): the first actor 0..0x1e
    /// with `+0x508` set becomes the effect's wait actor (motion 0xfe): 1;
    /// else 0.
    pub fn ms_check_chr_btl_pos_all<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        for i in 0..0x1fu8 {
            let actor = game(mem, &mut *call, MS_GET_CHR, &[u32::from(i)]);
            if mem.read::<u8>(actor.wrapping_add(0x508)) != 0 {
                w8(mem, EFFECT_WAIT, i);
                w8(mem, EFFECT_WAIT + 1, 0xfe);
                return 1;
            }
        }
        0
    }

    /// `MsSEPtransCheckWave()` (`0x007a04d0`): 0 with nothing in
    /// transfer, else 1, or 2 when the current queue record's first byte
    /// is set.
    pub fn ms_sep_trans_check_wave(mem: &impl Mem) -> u32 {
        if r32(mem, SEP_TRANS).cast_signed() <= 0 {
            return 0;
        }
        let i = r32(mem, SEP_TRANS_STATE);
        let at = mem.rebase(SEP_TRANS_QUEUE).wrapping_add(i.wrapping_mul(12));
        1 + u32::from(mem.read::<u8>(at) != 0)
    }

    /// `0x007abf40(group)` (`MsPickGroundGroup`, name ours): among the
    /// script records (`g_BtlScriptRecords`: an i8 count at `+1`, records
    /// of 0x20 from `+4`, the group an i8 at `+0`), from the first of
    /// `group` up to the last record of another group after it (or the
    /// end), one is picked with `rnd()`; the ground byte gets it plus 1
    /// and `MsCopyPointsOffset(records, pos_file, pick)` copies its
    /// points.
    pub fn ms_pick_ground_group<M: Mem>(mem: &mut M, call: Call<'_, M>, group: u32) {
        let recs = r32(mem, BTL_SCRIPT_RECORDS);
        let n = i32::from(mem.read::<i8>(recs.wrapping_add(1)));
        let group = group.cast_signed();
        let mut first = -1i32;
        let mut last = -1i32;
        let mut k = 0i32;
        if n > 0 {
            while k < n {
                let at = recs.wrapping_add(4).wrapping_add(k.cast_unsigned() * 0x20);
                let g = i32::from(mem.read::<i8>(at));
                if g == group {
                    if first < 0 {
                        first = k;
                    }
                } else if first >= 0 {
                    last = k;
                }
                k += 1;
            }
        }
        if first < 0 {
            first = 0;
        }
        if last < 0 {
            last = k;
        }
        let span = last.wrapping_sub(first);
        if span != 0 {
            let r = game(mem, &mut *call, RND, &[]).cast_signed();
            first = first.wrapping_add(r.wrapping_rem(span));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the byte
        w8(mem, BTL_GROUND_GROUP, first.wrapping_add(1) as u8);
        let pos = r32(mem, BTL_POS_FILE);
        game(
            mem,
            call,
            MS_COPY_POINTS_OFFSET,
            &[recs, pos, first.cast_unsigned()],
        );
    }

    /// `MsAtelFreeCameraID(kind, slot)` (`0x007ba6d0`): kind 0..3 and
    /// slot 0..2 name a camera id of `g_AtelCamera` (kind 0 is the second
    /// row, 1 the first); it is freed (`MsCameraFree`) and cleared.
    pub fn ms_atel_free_camera_id<M: Mem>(mem: &mut M, call: Call<'_, M>, kind: u32, slot: u32) {
        if slot > 2 {
            return;
        }
        let row = match kind {
            0 => 0xc,
            1 => 0,
            2 => 0x18,
            3 => 0x24,
            _ => return,
        };
        let at = mem.rebase(ATEL_CAMERA) + row + 4 * slot;
        let id: u32 = mem.read(at);
        game(mem, &mut *call, MS_CAMERA_FREE, &[id]);
        mem.write(at, 0u32);
    }

    /// `MsCameraGetAngle(yaw, pitch, len)` (`0x007bd2d0`): the active
    /// camera's look-at point (`+0x50`) to its eye (`+0x66c`) in polar
    /// form (`MsVectorRect2Polar`).
    pub fn ms_camera_get_angle<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        yaw: u32,
        pitch: u32,
        len: u32,
    ) {
        let cam = r32(mem, CAMERA);
        game(
            mem,
            call,
            MS_VECTOR_RECT2_POLAR,
            &[
                cam.wrapping_add(0x50),
                cam.wrapping_add(0x66c),
                yaw,
                pitch,
                len,
            ],
        );
    }

    /// `index` when below `count` (signed), else 0.
    fn wrap_index(index: i32, count: i32) -> u32 {
        if index >= count {
            0
        } else {
            index.cast_unsigned()
        }
    }

    /// `0x007c5dd0()` (`MsDebugSceneWalk`, name ours): the debug scene
    /// picker's selection made valid (a negative field or encounter
    /// index wraps to the last, past the end to 0), its field and
    /// encounter list looked up, the encounter's group and place in it
    /// found by walking the groups, and the scene's field number, group
    /// record, formation pointer and formation number set.
    pub fn ms_debug_scene_walk<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let fields = r16(mem, BTL_FIELD_COUNT).cast_signed();
        let mut field = r32(mem, DBG_FIELD).cast_signed();
        if field < 0 {
            field = i32::from(fields) - 1;
        }
        let field = wrap_index(field, i32::from(fields));
        w32(mem, DBG_FIELD, field);
        let rec = game(mem, &mut *call, MS_BTL_LIST_FIELD, &[field]);
        w32(mem, DBG_FIELD_REC, rec);
        let field = r32(mem, DBG_FIELD);
        let list = game(mem, &mut *call, MS_BTL_LIST_ENCOUNT, &[field]);
        w32(mem, DBG_ENCOUNT_LIST, list);
        let n = i32::from(mem.read::<u8>(list));
        let mut enc = r32(mem, DBG_ENCOUNT).cast_signed();
        if enc < 0 {
            enc = n - 1;
        }
        w32(mem, DBG_ENCOUNT, wrap_index(enc, n));
        let field = r32(mem, DBG_FIELD);
        let mut group = game(mem, &mut *call, MS_BTL_LIST_GROUP, &[field, 0]);
        w32(mem, DBG_GROUP_REC, group);
        w32(mem, DBG_GROUP, 0);
        let mut start = 0i32;
        let mut seen = 0i32;
        let mut enc = r32(mem, DBG_ENCOUNT).cast_signed();
        if mem.read::<u8>(list.wrapping_add(1)) > 0 {
            let mut g = 0u32;
            loop {
                let field = r32(mem, DBG_FIELD);
                group = game(mem, &mut *call, MS_BTL_LIST_GROUP, &[field, g]);
                enc = r32(mem, DBG_ENCOUNT).cast_signed();
                w32(mem, DBG_GROUP_REC, group);
                let size = i32::from(mem.read::<u8>(group));
                if enc >= seen && enc < size.wrapping_add(seen) {
                    start = seen;
                    break;
                }
                seen = seen.wrapping_add(size);
                g = r32(mem, DBG_GROUP).wrapping_add(1);
                w32(mem, DBG_GROUP, g);
                let list = r32(mem, DBG_ENCOUNT_LIST);
                let groups = i32::from(mem.read::<u8>(list.wrapping_add(1)));
                if g.cast_signed() >= groups {
                    break;
                }
            }
        }
        let rec = r32(mem, DBG_FIELD_REC);
        let in_group = enc.wrapping_sub(start).cast_unsigned();
        w32(mem, DBG_IN_GROUP, in_group);
        let field_no = i32::from(mem.read::<i16>(rec)).cast_unsigned();
        w32(mem, DBG_SCENE_FIELD, field_no);
        let formation = in_group.wrapping_mul(2).wrapping_add(5).wrapping_add(group);
        let number = mem.read::<u8>(formation);
        w32(mem, DBG_FORMATION, formation);
        w32(mem, DBG_SCENE_FORMATION, u32::from(number));
    }

    /// `0x007c5f00()` (`MsDebugSceneGroupBase`, name ours): the picked
    /// encounter's global index: the sizes of the groups before
    /// `DBG_GROUP` plus `DBG_IN_GROUP`; stored in `DBG_ENCOUNT` and
    /// returned.
    pub fn ms_debug_scene_group_base<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        let mut total = 0u32;
        w32(mem, DBG_ENCOUNT, 0);
        let mut g = 0u32;
        while g.cast_signed() < r32(mem, DBG_GROUP).cast_signed() {
            let field = r32(mem, DBG_FIELD);
            let rec = game(mem, &mut *call, MS_BTL_LIST_GROUP, &[field, g]);
            w32(mem, DBG_GROUP_REC, rec);
            total = r32(mem, DBG_ENCOUNT).wrapping_add(u32::from(mem.read::<u8>(rec)));
            g += 1;
            w32(mem, DBG_ENCOUNT, total);
        }
        total = total.wrapping_add(r32(mem, DBG_IN_GROUP));
        w32(mem, DBG_ENCOUNT, total);
        total
    }

    /// `MsDebugSetScene(field, group, in_group, scene_field, formation)`
    /// (`0x007c5f60`): the picker's selection set, then the tail call of
    /// [`ms_debug_scene_group_base`] (its `eax`).
    pub fn ms_debug_set_scene<M: Mem>(mem: &mut M, call: Call<'_, M>, args: [u32; 5]) -> u32 {
        let [field, group, in_group, scene_field, formation] = args;
        w32(mem, DBG_FIELD, field);
        w32(mem, DBG_GROUP, group);
        w32(mem, DBG_IN_GROUP, in_group);
        w32(mem, DBG_SCENE_FIELD, scene_field);
        w32(mem, DBG_SCENE_FORMATION, formation);
        game(mem, call, MS_DEBUG_SCENE_GROUP_BASE, &[])
    }

    /// `MsGetBattleName(full, out)` (`0x007c5fc0`): after the picker's
    /// walk (`0x007c5dd0`), the field record's 8 name bytes (`+6`) into
    /// `DBG_SCENE_NAME`; `out` gets `"<name>_<formation>"`, or with
    /// `full` set on field 0 one of the two fixed names (by whether the
    /// formation byte is 0). Returns `out`.
    pub fn ms_get_battle_name<M: Mem>(mem: &mut M, call: Call<'_, M>, full: u32, out: u32) -> u32 {
        game(mem, &mut *call, MS_DEBUG_SCENE_WALK, &[]);
        let rec = r32(mem, DBG_FIELD_REC);
        let name = mem.rebase(DBG_SCENE_NAME);
        for k in 0..8 {
            let b: u8 = mem.read(rec.wrapping_add(6 + k));
            mem.write(name + k, b);
        }
        mem.write(name + 8, 0u8);
        let formation = r32(mem, DBG_FORMATION);
        if full != 0 && r32(mem, DBG_FIELD) == 0 {
            let fixed = if mem.read::<u8>(formation) == 0 {
                SCENE_FIXED_A
            } else {
                SCENE_FIXED_B
            };
            let (fmt, fixed) = (mem.rebase(FMT_S), mem.rebase(fixed));
            import(mem, call, SPRINTF_SLOT, &[out, fmt, fixed]);
            return out;
        }
        let number = u32::from(mem.read::<u8>(formation));
        let fmt = mem.rebase(FMT_SCENE);
        import(mem, call, SPRINTF_SLOT, &[out, fmt, name, number]);
        out
    }

    /// `0x007c60a0(slot, step)` (`MsDebugNextMember`, name ours): party
    /// slot `slot`'s character stepped (`step` < 0 down, wrapping from
    /// 0 to 0x11; > 0 up, 0xff past 0x11) to the next one who has joined
    /// and isn't in another party slot. 0xff when the slot is empty.
    pub fn ms_debug_next_member<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        slot: u32,
        step: u32,
    ) -> u32 {
        let party = mem.rebase(SAVE_IN_PARTY);
        let mut c: u8 = mem.read(party.wrapping_add(slot));
        let step = step.cast_signed();
        loop {
            if step < 0 {
                // the original skips the empty check after the wrap to
                // 0x11, which can't be 0xff anyway
                c = c.wrapping_sub(1);
                if c == 0xfe {
                    c = 0x11;
                }
            } else if step > 0 {
                c = c.wrapping_add(1);
                if c >= 0x12 {
                    return 0xff;
                }
            }
            if c == 0xff {
                return 0xff;
            }
            if game(mem, &mut *call, MS_GET_SAVE_PLY_JOIN, &[u32::from(c)]) == 0 {
                continue;
            }
            let taken = (0..3u32).any(|k| k != slot && mem.read::<u8>(party + k) == c);
            if !taken {
                return u32::from(c);
            }
        }
    }

    /// `0x007ce9f0(writer, text)` (`MsEnvWriteText`, name ours): the
    /// string `text` appended to the writer's buffer (`+0` buffer, `+8`
    /// file, `+0xc` fill, `+0x10` size), the full buffer written out with
    /// `sceWrite(file, buffer, size)` whenever it fills. The original
    /// keeps the chunk length in its `text` argument slot (`slots + 4`).
    pub fn ms_env_write_text<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        slots: u32,
        writer: u32,
        text: u32,
    ) {
        if mem.read::<u8>(text) == 0 {
            return;
        }
        let mut len = 0i32;
        while mem.read::<u8>(text.wrapping_add(len.cast_unsigned())) != 0 {
            len += 1;
        }
        let mut rem = len;
        let mut src = text;
        while rem > 0 {
            let fill_at = writer.wrapping_add(0xc);
            let size: i32 = mem.read(writer.wrapping_add(0x10));
            let space = size.wrapping_sub(mem.read(fill_at));
            let chunk = if rem > space { space } else { rem };
            mem.write(slots + 4, chunk);
            for k in 0..chunk.max(0).cast_unsigned() {
                // both words read again for each byte, as the original does
                let fill: u32 = mem.read(fill_at);
                let buf: u32 = mem.read(writer);
                let b: u8 = mem.read(src);
                mem.write(fill.wrapping_add(buf).wrapping_add(k), b);
                src = src.wrapping_add(1);
            }
            let fill: i32 = mem.read(fill_at);
            mem.write(fill_at, fill.wrapping_add(chunk));
            rem = rem.wrapping_sub(chunk);
            let size: i32 = mem.read(writer.wrapping_add(0x10));
            if mem.read::<i32>(fill_at) >= size {
                let buf: u32 = mem.read(writer);
                mem.write(fill_at, 0u32);
                let out: u32 = mem.read(writer.wrapping_add(8));
                game(
                    mem,
                    &mut *call,
                    SCE_WRITE,
                    &[out, buf, size.cast_unsigned()],
                );
            }
        }
    }

    /// `0x007cf140(table, key)` (`MsEnvFindKey`, name ours): the first
    /// word of the 12-byte entry whose second word is `key`, in a table
    /// that ends at an entry whose first word is 0; 0 when none.
    pub fn ms_env_find_key(mem: &impl Mem, table: u32, key: u32) -> u32 {
        let mut i = 0u32;
        if mem.read::<u32>(table) == 0 {
            return 0;
        }
        loop {
            let e = table.wrapping_add(12 * i);
            if mem.read::<u32>(e.wrapping_add(4)) == key {
                return mem.read(e);
            }
            i += 1;
            if mem.read::<u32>(table.wrapping_add(12 * i)) == 0 {
                return 0;
            }
        }
    }
}
