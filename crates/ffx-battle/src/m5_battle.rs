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
#[allow(clippy::many_single_char_names)] // indices, bytes and the original's registers
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

    // ---- batch 2: the debug files, the position script reader ----

    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const USER_FREE_THUNK: u32 = 0x0068_7010;
    pub const DEBUG_EXIT_TRACE: u32 = 0x0062_f510;
    pub const OP_FILE_PUTS: u32 = 0x007c_f860;
    pub const MS_GET_BATTLE_NAME: u32 = 0x007c_5fc0;
    pub const SG_GET_USER_NAME: u32 = 0x0082_0610;
    pub const MS_BTL_POS_INIT: u32 = 0x007c_93d0;
    pub const MS_FREE: u32 = 0x007c_f8f0;
    pub const MS_MALLOC: u32 = 0x007c_f970;
    pub const MS_STRUCT_CLEAR: u32 = 0x0079_f230;
    pub const MS_STR_COPY: u32 = 0x007c_f6c0;
    pub const MS_CAMERA_GET_NUM: u32 = 0x007b_d340;
    pub const READ_SKIP_SPACE: u32 = 0x007c_f540;
    pub const READ_TOKEN: u32 = 0x007c_f180;
    pub const PATH_BASE_NAME: u32 = 0x007c_efb0;
    pub const DEBUG_WIN_ALLOC: u32 = 0x007c_8bb0;
    /// `sscanf`'s and `malloc`'s import table slots.
    pub const SSCANF_SLOT: u32 = 0x00b0_c448;
    pub const MALLOC_SLOT: u32 = 0x00b0_c468;
    /// The debug memory trace: on when set, its file.
    pub const MEM_TRACE: u32 = 0x0113_e490;
    pub const MEM_TRACE_FILE: u32 = 0x0231_14e0;
    /// The memory trace's messages (`.rdata`).
    pub const MSG_MALLOC_FAILED: u32 = 0x00b5_5c4c;
    pub const FMT_MALLOC: u32 = 0x00b5_5c6c;
    pub const MSG_FREE_NULL: u32 = 0x00b5_5c7c;
    pub const FMT_FREE: u32 = 0x00b5_5ca4;
    /// The debug files' messages and the position file names (`.rdata`).
    pub const MSG_FILE_ERROR: u32 = 0x00b5_54f8;
    pub const MSG_FILE_READ: u32 = 0x00b5_550c;
    pub const MSG_FILE_WRITE: u32 = 0x00b5_54e4;
    pub const FMT_POS_NAME: u32 = 0x00b5_551c;
    pub const FMT_POS_NAME_USER: u32 = 0x00b5_5544;
    /// The position script parser's state: the file being read
    /// (`0x0113e27c`, a record of 0x74 bytes), the position table
    /// (`0x0113e278`), its sub-record cursor, the line and file name of
    /// errors, the error code, the version.
    pub const POS_TABLE: u32 = 0x0113_e278;
    pub const POS_RECORD: u32 = 0x0113_e27c;
    pub const POS_SUB: u32 = 0x0113_e284;
    pub const POS_LINE: u32 = 0x0113_e288;
    pub const POS_FILE_NAME: u32 = 0x0113_e28c;
    pub const OP_LAST_ERROR: u32 = 0x0113_e238;
    pub const POS_VERSION: u32 = 0x0113_e270;
    pub const POS_VERSION_SUB: u32 = 0x0113_e274;
    /// The parser's messages: the file name, the line, the version and
    /// missing value errors; the int and float formats.
    pub const MSG_POS_FILE: u32 = 0x00b5_5904;
    pub const MSG_POS_LINE: u32 = 0x00b5_590c;
    pub const MSG_POS_VERSION: u32 = 0x00b5_59f4;
    pub const MSG_POS_VALUE: u32 = 0x00b5_59bc;
    pub const FMT_FLOAT: u32 = 0x00b4_0274;
    pub const FMT_INT: u32 = 0x00b0_f604;
    /// The debug windows: 0x80 records of 0x34 bytes from `0x0113c800`
    /// (in use when the byte at `+0x20` is set), and the four the info
    /// screen keeps (`0x0113e494`) with their colour flag.
    pub const DEBUG_WINS: u32 = 0x0113_c800;
    pub const DEBUG_WINS_END: u32 = 0x0113_e220;
    pub const INFO_WINS: u32 = 0x0113_e494;
    pub const INFO_WIN_FLAGS: u32 = 0x0113_e4cc;
    /// `g_DebugCamera`, `g_Cameras`.
    pub const DEBUG_CAMERA: u32 = 0x0113_70c1;
    pub const CAMERAS: u32 = 0x0113_7540;

    /// `MsCheckFile(name)` (`0x007c8a80`): the file's size rounded up to
    /// 4 (`sceOpen`, `sceLseek` to the end and back, `sceClose`); -1 when
    /// it doesn't open.
    pub fn ms_check_file<M: Mem>(mem: &mut M, call: Call<'_, M>, name: u32) -> u32 {
        let fd = game(mem, &mut *call, SCE_OPEN, &[name, 1]);
        if fd == u32::MAX {
            return u32::MAX;
        }
        let size = game(mem, &mut *call, SCE_LSEEK, &[fd, 0, 2]).cast_signed();
        game(mem, &mut *call, SCE_LSEEK, &[fd, 0, 0]);
        let x = size.wrapping_add(3);
        let rounded = x.wrapping_add((x >> 31) & 3) & !3;
        game(mem, call, SCE_CLOSE, &[fd]);
        rounded.cast_unsigned()
    }

    /// `MsFileIn(name, buf, n)` (`0x007c8b00`) and `MsFileOut(name, buf,
    /// n)` (`0x007c8b50`, `out`): the file read into or written from
    /// `buf`, with a debug line; a debug line when it doesn't open.
    pub fn ms_file_io<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        out: bool,
        name: u32,
        buf: u32,
        n: u32,
    ) {
        let mode = if out { 0x602 } else { 1 };
        let fd = game(mem, &mut *call, SCE_OPEN, &[name, mode]);
        if fd.cast_signed() < 0 {
            let msg = mem.rebase(MSG_FILE_ERROR);
            game(mem, &mut *call, DBG_PRINTF, &[msg, name]);
            return;
        }
        let msg = mem.rebase(if out { MSG_FILE_WRITE } else { MSG_FILE_READ });
        game(mem, &mut *call, DBG_PRINTF, &[msg, name]);
        let f = if out { SCE_WRITE } else { SCE_READ };
        game(mem, &mut *call, f, &[fd, buf, n]);
        game(mem, call, SCE_CLOSE, &[fd]);
    }

    /// `0x007c8bb0(slot)` (`MsDebugWinAlloc`, name ours): `*slot` when
    /// set; else the first free debug window record (marked in use:
    /// `+0x20` = 0x101, `+0x22` = 6), or the end of the table when none is
    /// free, stored into `*slot` (when `slot` isn't null) and returned.
    pub fn ms_debug_win_alloc(mem: &mut impl Mem, slot: u32) -> u32 {
        if slot != 0 {
            let p: u32 = mem.read(slot);
            if p != 0 {
                return p;
            }
        }
        let base = mem.rebase(DEBUG_WINS);
        let end = mem.rebase(DEBUG_WINS_END);
        let mut k = 0u32;
        let mut at = base + 0x20;
        let mut found = false;
        while at < end {
            if mem.read::<u8>(at) == 0 {
                found = true;
                break;
            }
            at += 0x34;
            k += 1;
        }
        if found {
            mem.write(base + 0x20 + k * 0x34, 0x101u16);
            mem.write(base + 0x22 + k * 0x34, 6u8);
        }
        let rec = base + k * 0x34;
        if slot != 0 {
            mem.write(slot, rec);
        }
        rec
    }

    /// `0x007cf970(size)` (`ms_malloc`, name ours): `user_malloc(size)`;
    /// a debug line and `debug_exit_trace(1)` when it fails; with the
    /// memory trace on, a line for the block into its file (printed in the
    /// frame buffer `buf`, 0x100 bytes at the original's `ebp - 0x104`).
    /// Returns the block.
    pub fn ms_malloc<M: Mem>(mem: &mut M, call: Call<'_, M>, buf: u32, size: u32) -> u32 {
        let p = game(mem, &mut *call, USER_MALLOC, &[size]);
        if p == 0 {
            let msg = mem.rebase(MSG_MALLOC_FAILED);
            game(mem, &mut *call, DBG_PRINTF, &[msg]);
            game(mem, &mut *call, DEBUG_EXIT_TRACE, &[1]);
        }
        trace_line(mem, call, buf, FMT_MALLOC, p);
        p
    }

    /// The memory trace's line for `p` (format `fmt`), when the trace is
    /// on.
    fn trace_line<M: Mem>(mem: &mut M, call: Call<'_, M>, buf: u32, fmt: u32, p: u32) {
        if r32(mem, MEM_TRACE) == 0 {
            return;
        }
        let fmt = mem.rebase(fmt);
        import(mem, &mut *call, SPRINTF_SLOT, &[buf, fmt, p]);
        let file = mem.rebase(MEM_TRACE_FILE);
        game(mem, call, OP_FILE_PUTS, &[file, buf]);
    }

    /// `ms_free(p)` (`0x007cf8f0`): a debug line for null; else the trace
    /// line and `user_free(p)`. `buf` as in [`ms_malloc`].
    pub fn ms_free<M: Mem>(mem: &mut M, call: Call<'_, M>, buf: u32, p: u32) {
        if p == 0 {
            let msg = mem.rebase(MSG_FREE_NULL);
            game(mem, &mut *call, DBG_PRINTF, &[msg]);
            return;
        }
        trace_line(mem, &mut *call, buf, FMT_FREE, p);
        game(mem, call, USER_FREE_THUNK, &[p]);
    }

    /// `MsDebugBattlePosInit(out, a, b)` (`0x007c90f0`): the position
    /// file's name: the battle name (`MsGetBattleName(0, name)`, `name`
    /// the frame buffer, 0x100 bytes at the original's `ebp - 0x104`)
    /// printed into `out` with the country and `a`, `b`; in a debug build
    /// with a user name, that name first.
    pub fn ms_debug_battle_pos_init<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        name: u32,
        args: [u32; 3],
    ) {
        let [out, a, b] = args;
        game(mem, &mut *call, MS_GET_BATTLE_NAME, &[0, name]);
        let country = mem.rebase(COUNTRY_NAME);
        if game(mem, &mut *call, FFX_IS_DEBUG, &[]) != 0 {
            let user = game(mem, &mut *call, SG_GET_USER_NAME, &[]);
            if user != 0 {
                let fmt = mem.rebase(FMT_POS_NAME_USER);
                import(
                    mem,
                    call,
                    SPRINTF_SLOT,
                    &[out, fmt, user, country, name, a, name, b],
                );
                return;
            }
        }
        let fmt = mem.rebase(FMT_POS_NAME);
        import(
            mem,
            call,
            SPRINTF_SLOT,
            &[out, fmt, country, name, a, name, b],
        );
    }

    /// `0x007c9210()` (`MsBtlPosFree`, name ours): every block the
    /// position table holds freed with `ms_free` (per file record of 0x74
    /// bytes: `+0x28`, `+0x2c`, `+0x4c`, `+0x50`, `+0x60`, `+0x64`, the
    /// `+0x68` sub-records' `+0x10` blocks and the sub-record array
    /// `+0x70`; then the table's `+0x2c` and record array `+0x30`), the
    /// cursors kept in `POS_RECORD`/`POS_SUB` as the original does; then
    /// `MsBtlPosInit()`.
    pub fn ms_btl_pos_free<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let table = r32(mem, POS_TABLE);
        if table != 0 && mem.read::<u32>(table.wrapping_add(0x30)) != 0 {
            let mut i = 0i32;
            while i < mem.read::<i32>(r32(mem, POS_TABLE).wrapping_add(0x1c)) {
                let recs: u32 = mem.read(r32(mem, POS_TABLE).wrapping_add(0x30));
                let mut e = recs.wrapping_add(i.cast_unsigned().wrapping_mul(0x74));
                w32(mem, POS_RECORD, e);
                for off in [0x28, 0x2c, 0x4c, 0x50, 0x60, 0x64] {
                    let p: u32 = mem.read(e.wrapping_add(off));
                    if p != 0 {
                        game(mem, &mut *call, MS_FREE, &[p]);
                        e = r32(mem, POS_RECORD);
                    }
                }
                if mem.read::<u32>(e.wrapping_add(0x70)) != 0 {
                    let mut k = 0i32;
                    while k < mem.read::<i32>(e.wrapping_add(0x68)) {
                        let sub = mem
                            .read::<u32>(e.wrapping_add(0x70))
                            .wrapping_add(k.cast_unsigned().wrapping_mul(0x14));
                        w32(mem, POS_SUB, sub);
                        let p: u32 = mem.read(sub.wrapping_add(0x10));
                        if p != 0 {
                            game(mem, &mut *call, MS_FREE, &[p]);
                            e = r32(mem, POS_RECORD);
                        }
                        k += 1;
                    }
                    let subs: u32 = mem.read(e.wrapping_add(0x70));
                    game(mem, &mut *call, MS_FREE, &[subs]);
                }
                i += 1;
            }
            let p: u32 = mem.read(r32(mem, POS_TABLE).wrapping_add(0x2c));
            if p != 0 {
                game(mem, &mut *call, MS_FREE, &[p]);
            }
            let recs: u32 = mem.read(r32(mem, POS_TABLE).wrapping_add(0x30));
            game(mem, &mut *call, MS_FREE, &[recs]);
        }
        game(mem, call, MS_BTL_POS_INIT, &[]);
    }

    /// The parser's error lines: the file name and line when set, then
    /// `msg` with `arg`; the error code `code`.
    fn pos_error<M: Mem>(mem: &mut M, call: Call<'_, M>, msg: u32, arg: u32, code: u32) {
        let file = r32(mem, POS_FILE_NAME);
        if file != 0 {
            let m = mem.rebase(MSG_POS_FILE);
            game(mem, &mut *call, DBG_PRINTF, &[m, file]);
        }
        let line = r32(mem, POS_LINE);
        if line != 0 {
            let m = mem.rebase(MSG_POS_LINE);
            game(mem, &mut *call, DBG_PRINTF, &[m, line]);
        }
        let m = mem.rebase(msg);
        game(mem, call, DBG_PRINTF, &[m, arg]);
        w32(mem, OP_LAST_ERROR, code);
    }

    /// `0x007cc1f0(name, version)` (`MsBtlPosCheckVersion`, name ours):
    /// 1 when the file's version is at least `version` (signed); else the
    /// error lines, error 7, 0.
    pub fn ms_btl_pos_check_version<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        name: u32,
        version: u32,
    ) -> u32 {
        if r32(mem, POS_VERSION).cast_signed() >= version.cast_signed() {
            return 1;
        }
        pos_error(mem, call, MSG_POS_VERSION, name, 7);
        0
    }

    /// `0x007cc250(name, version, sub)` (`MsBtlPosCheckVersion2`, name
    /// ours): 1 when the version is at least `version` and the
    /// sub-version is `sub`; else as [`ms_btl_pos_check_version`].
    pub fn ms_btl_pos_check_version2<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        name: u32,
        version: u32,
        sub: u32,
    ) -> u32 {
        if r32(mem, POS_VERSION).cast_signed() >= version.cast_signed()
            && r32(mem, POS_VERSION_SUB) == sub
        {
            return 1;
        }
        pos_error(mem, call, MSG_POS_VERSION, name, 7);
        0
    }

    /// `0x007cf540(reader)` (`MsReadSkipSpace`, name ours): a text
    /// reader (`+0` buffer, `+0xc` position, `+0x10` length) past tabs
    /// and spaces; the position ends on the first other byte (past the
    /// end: one further, then back one). Returns how many it skipped.
    pub fn ms_read_skip_space(mem: &mut impl Mem, reader: u32) -> u32 {
        let pos_at = reader.wrapping_add(0xc);
        let len: i32 = mem.read(reader.wrapping_add(0x10));
        let mut n = 0u32;
        let pos = loop {
            let pos: i32 = mem.read(pos_at);
            if pos >= len {
                mem.write(pos_at, pos.wrapping_add(1));
                break mem.read::<i32>(pos_at);
            }
            let buf: u32 = mem.read(reader);
            let c: u8 = mem.read(buf.wrapping_add(pos.cast_unsigned()));
            let pos = pos.wrapping_add(1);
            mem.write(pos_at, pos);
            if c == 9 || c == 0x20 {
                n += 1;
                continue;
            }
            break pos;
        };
        if pos > 0 {
            mem.write(pos_at, pos - 1);
        }
        n
    }

    /// The next byte of a text reader, the position advanced (stored);
    /// `Err(pos)` past the end (the position then one further, stored and
    /// read back).
    fn reader_next(mem: &mut impl Mem, reader: u32) -> Result<u8, i32> {
        let pos_at = reader.wrapping_add(0xc);
        let pos: i32 = mem.read(pos_at);
        if pos >= mem.read::<i32>(reader.wrapping_add(0x10)) {
            mem.write(pos_at, pos.wrapping_add(1));
            return Err(mem.read(pos_at));
        }
        let buf: u32 = mem.read(reader);
        let c: u8 = mem.read(buf.wrapping_add(pos.cast_unsigned()));
        mem.write(pos_at, pos.wrapping_add(1));
        Ok(c)
    }

    /// `0x007cf180(reader, out, size)` (`MsReadToken`, name ours): after
    /// the reader's spaces (`0x007cf540`), a token into `out` (at most
    /// `size - 1` bytes and a NUL): up to a space or tab outside double
    /// quotes, a line end, a NUL or 0xff, the quotes and single quotes
    /// left out, Shift-JIS pairs kept whole. The reader stops on the byte
    /// that ended it. The original keeps the quote flag in its `reader`
    /// argument slot (`slots`). Returns the token's length.
    pub fn ms_read_token<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        slots: u32,
        reader: u32,
        out: u32,
        size: u32,
    ) -> u32 {
        game(mem, &mut *call, READ_SKIP_SPACE, &[reader]);
        let max = size.wrapping_sub(1).cast_signed();
        let mut at = out;
        let mut n = 0i32;
        mem.write(slots, 0u32);
        let pos_at = reader.wrapping_add(0xc);
        let pos = loop {
            let c = match reader_next(mem, reader) {
                Ok(c) => c,
                Err(pos) => break pos,
            };
            if c == 0xff || c == 0 {
                break mem.read::<i32>(pos_at);
            }
            if c >= 0x80 {
                if n < max {
                    mem.write(at, c);
                    at = at.wrapping_add(1);
                    n += 1;
                }
                let c2 = match reader_next(mem, reader) {
                    Ok(c) => c,
                    Err(pos) => break pos,
                };
                if c2 == 0xff || c2 == 0 {
                    break mem.read::<i32>(pos_at);
                }
                if n < max {
                    mem.write(at, c2);
                    at = at.wrapping_add(1);
                    n += 1;
                }
                continue;
            }
            if c == b'"' {
                let q: u32 = mem.read(slots);
                mem.write(slots, 1u32.wrapping_sub(q));
            } else if ((c == b' ' || c == b'\t') && mem.read::<u32>(slots) == 0)
                || c == b'\r'
                || c == b'\n'
            {
                break mem.read::<i32>(pos_at);
            }
            if n >= max || c == b'"' || c == b'\'' {
                continue;
            }
            mem.write(at, c);
            at = at.wrapping_add(1);
            n += 1;
        };
        if pos > 0 {
            mem.write(pos_at, pos - 1);
        }
        mem.write(at, 0u8);
        n.cast_unsigned()
    }

    /// `0x007cd360(reader, name, out)` (`MsReadFloat`, name ours, `float`:
    /// `"%f"`) and `0x007cd420` (`MsReadInt`, `"%d"`): the next token
    /// (into the frame buffer `tok`, 0x100 bytes at the original's
    /// `ebp - 0x104`) scanned into `out` with `sscanf`; an empty
    /// token gives the error lines and error 5.
    pub fn ms_read_value<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        tok: u32,
        float: bool,
        args: [u32; 3],
    ) {
        let [reader, name, out] = args;
        game(mem, &mut *call, READ_TOKEN, &[reader, tok, 0x100]);
        if mem.read::<u8>(tok) == 0 {
            pos_error(mem, call, MSG_POS_VALUE, name, 5);
            return;
        }
        let fmt = mem.rebase(if float { FMT_FLOAT } else { FMT_INT });
        import(mem, call, SSCANF_SLOT, &[tok, fmt, out]);
    }

    /// `0x007cd4e0()` (`MsBtlPosAllocPoints`, name ours): the current
    /// file record's two point arrays (`+0x60`, `+0x64`: `+0x54` entries
    /// of 12 bytes) allocated with `ms_malloc` and cleared, its counts
    /// `+0x58`, `+0x5c` zeroed.
    pub fn ms_btl_pos_alloc_points<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let rec = r32(mem, POS_RECORD);
        mem.write(rec.wrapping_add(0x58), 0u32);
        mem.write(rec.wrapping_add(0x5c), 0u32);
        for off in [0x60u32, 0x64] {
            let rec = r32(mem, POS_RECORD);
            let n = mem.read::<u32>(rec.wrapping_add(0x54)).wrapping_mul(12);
            let p = game(mem, &mut *call, MS_MALLOC, &[n]);
            let rec = r32(mem, POS_RECORD);
            mem.write(rec.wrapping_add(off), p);
            let n = mem.read::<u32>(rec.wrapping_add(0x54)).wrapping_mul(12);
            game(mem, &mut *call, MS_STRUCT_CLEAR, &[p, n]);
        }
    }

    /// A byte lower-cased (`A`..`Z` only).
    fn lower(c: u8) -> u8 {
        if c.wrapping_sub(0x41) <= 0x19 {
            c.wrapping_add(0x20)
        } else {
            c
        }
    }

    /// `0x007ceeb0(name, table)` (`MsKeywordLookup`, name ours): the
    /// value (`+4`) of the 12-byte entry (`+0` name, `+8` hash; the table
    /// ends at a null name) whose hash and name match `name` without
    /// regard to case; 0 when none. The hash: the sum of each lower-cased
    /// (sign-extended) byte xor `0x6f * index`.
    pub fn ms_keyword_lookup(mem: &impl Mem, name: u32, table: u32) -> u32 {
        let mut hash = 0u32;
        let mut k = 0u32;
        let mut p = name;
        loop {
            let c: u8 = mem.read(p);
            if c == 0 {
                break;
            }
            let v = i32::from(lower(c).cast_signed()).cast_unsigned();
            hash = hash.wrapping_add(v ^ k);
            k = k.wrapping_add(0x6f);
            p = p.wrapping_add(1);
        }
        let mut i = 0u32;
        loop {
            let e = table.wrapping_add(12u32.wrapping_mul(i));
            let key: u32 = mem.read(e);
            if key == 0 {
                return 0;
            }
            if hash == mem.read::<u32>(e.wrapping_add(8)) {
                let (mut a, mut b) = (name, key);
                loop {
                    let x = lower(mem.read(a));
                    let y = lower(mem.read(b));
                    if x != y {
                        break;
                    }
                    if x == 0 {
                        return mem.read(e.wrapping_add(4));
                    }
                    a = a.wrapping_add(1);
                    b = b.wrapping_add(1);
                }
            }
            i = i.wrapping_add(1);
        }
    }

    /// `0x007cf380(reader, line)` (`MsReadSkipLine`, name ours): the
    /// reader past the rest of the line (Shift-JIS pairs whole) and its
    /// `\n`; `*line` counted up when `line` isn't null. Returns the bytes
    /// skipped, or -1 at the end of the text (a NUL, 0xff or the end; the
    /// reader then back on it).
    pub fn ms_read_skip_line(mem: &mut impl Mem, reader: u32, line: u32) -> u32 {
        let pos_at = reader.wrapping_add(0xc);
        let len: i32 = mem.read(reader.wrapping_add(0x10));
        let mut n = 0u32;
        let end = loop {
            let pos: i32 = mem.read(pos_at);
            if pos >= len {
                mem.write(pos_at, pos.wrapping_add(1));
                break true;
            }
            let buf: u32 = mem.read(reader);
            let c: u8 = mem.read(buf.wrapping_add(pos.cast_unsigned()));
            let pos = pos.wrapping_add(1);
            mem.write(pos_at, pos);
            if c == 0xff || c == 0 {
                break true;
            }
            if c < 0x80 {
                if c == b'\n' {
                    break false;
                }
                n = n.wrapping_add(1);
                continue;
            }
            n = n.wrapping_add(1);
            if pos >= len {
                mem.write(pos_at, pos.wrapping_add(1));
                break true;
            }
            let c2: u8 = mem.read(buf.wrapping_add(pos.cast_unsigned()));
            mem.write(pos_at, pos.wrapping_add(1));
            if c2 == 0xff || c2 == 0 {
                break true;
            }
            n = n.wrapping_add(1);
        };
        if end {
            n = u32::MAX;
            let pos: i32 = mem.read(pos_at);
            if pos > 0 {
                mem.write(pos_at, pos - 1);
            }
        }
        if line != 0 {
            let v: u32 = mem.read(line);
            mem.write(line, v.wrapping_add(1));
        }
        n
    }

    /// `0x007cd5a0(reader, name)` (`MsReadOpen`, name ours): the file
    /// `name` read whole into a `malloc` buffer of its size rounded up to
    /// 16 (cleared first): the reader's `+0` buffer, `+4` 0, `+8` the
    /// file (negative when it doesn't open; 0 after the read), `+0xc`
    /// position 0, `+0x10` length.
    pub fn ms_read_open<M: Mem>(mem: &mut M, call: Call<'_, M>, reader: u32, name: u32) {
        let fd = game(mem, &mut *call, SCE_OPEN, &[name, 1]);
        mem.write(reader.wrapping_add(8), fd);
        if fd.cast_signed() < 0 {
            return;
        }
        mem.write(reader.wrapping_add(4), 0u32);
        let size = game(mem, &mut *call, SCE_LSEEK, &[fd, 0, 2]).cast_signed();
        let fd: u32 = mem.read(reader.wrapping_add(8));
        game(mem, &mut *call, SCE_LSEEK, &[fd, 0, 0]);
        let x = size.wrapping_add(0xf);
        let len = x.wrapping_add((x >> 31) & 0xf) & !0xf;
        mem.write(reader.wrapping_add(0x10), len);
        mem.write(reader.wrapping_add(0xc), 0u32);
        let buf = import(mem, &mut *call, MALLOC_SLOT, &[len.cast_unsigned()]);
        mem.write(reader, buf);
        let mut i = 0i32;
        while i < mem.read::<i32>(reader.wrapping_add(0x10)) {
            let b: u32 = mem.read(reader);
            mem.write(b.wrapping_add(i.cast_unsigned()), 0u8);
            i += 1;
        }
        let len: u32 = mem.read(reader.wrapping_add(0x10));
        let buf: u32 = mem.read(reader);
        let fd: u32 = mem.read(reader.wrapping_add(8));
        game(mem, &mut *call, SCE_READ, &[fd, buf, len]);
        let fd: u32 = mem.read(reader.wrapping_add(8));
        game(mem, call, SCE_CLOSE, &[fd]);
        mem.write(reader.wrapping_add(8), 0u32);
    }

    /// `0x007cefb0(path, ext)` (`MsPathBaseName`, name ours): the file
    /// name after the last `\` or `/` (Shift-JIS pairs whole), and in
    /// `*ext` (when `ext` isn't null) the index of its `.` (or of its
    /// end).
    pub fn ms_path_base_name(mem: &mut impl Mem, path: u32, ext: u32) -> u32 {
        let mut p = path;
        let mut base = path;
        loop {
            let mut c: u8 = mem.read(p);
            p = p.wrapping_add(1);
            if c >= 0x80 {
                c = mem.read(p);
                p = p.wrapping_add(1);
            } else if c == b'\\' || c == b'/' {
                base = p;
            }
            if c == 0 {
                break;
            }
        }
        let mut i = 0u32;
        loop {
            let mut c: u8 = mem.read(base.wrapping_add(i));
            if c >= 0x80 {
                c = mem.read(base.wrapping_add(i).wrapping_add(1));
                i = i.wrapping_add(1);
            } else if c == b'.' {
                break;
            }
            if c == 0 {
                break;
            }
            i = i.wrapping_add(1);
        }
        if ext != 0 {
            mem.write(ext, i);
        }
        base
    }

    /// `0x007cf330(a, b)` (`MsStrEqualNoCase`, name ours): 1 when the
    /// strings are equal without regard to case (`A`..`Z`), else 0.
    pub fn ms_str_equal_no_case(mem: &impl Mem, a: u32, b: u32) -> u32 {
        let (mut a, mut b) = (a, b);
        loop {
            let x = lower(mem.read(a));
            let y = lower(mem.read(b));
            a = a.wrapping_add(1);
            b = b.wrapping_add(1);
            if x != y {
                return 0;
            }
            if x == 0 {
                return 1;
            }
        }
    }

    /// Bytes up to a NUL, `\r`, `\n` or 0xff.
    fn line_len(mem: &impl Mem, p: u32) -> u32 {
        let mut n = 0u32;
        let mut c: u8 = mem.read(p);
        while c != 0xff && c != 0 && c != b'\r' && c != b'\n' {
            n = n.wrapping_add(1);
            c = mem.read(p.wrapping_add(n));
        }
        n
    }

    /// `0x007cf590(path, src, n, force)` (`MsPathSetExt`, name ours): the
    /// extension of `path`'s file name replaced by `src`
    /// (`MsStrCopy(src, ext, n + ext_len - path_len)`, a `.` put in first
    /// where the name has none; a `.` after a Shift-JIS lead byte doesn't
    /// count), when it has none or `force` is 1: 1; else 0. The original
    /// lets `0x007cefb0` store the extension's index into its own `path`
    /// argument slot (`slots`).
    pub fn ms_path_set_ext<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        slots: u32,
        args: [u32; 4],
    ) -> u32 {
        let [path, src, n, force] = args;
        let mut total = line_len(mem, path);
        let base = game(mem, &mut *call, PATH_BASE_NAME, &[path, slots]);
        let mut i: u32 = mem.read(slots);
        if mem.read::<u8>(base.wrapping_add(i)) == b'.' {
            i = i.wrapping_add(1);
        }
        let mut ext = base.wrapping_add(i);
        let ext_len = line_len(mem, ext);
        if force != 1 && ext_len != 0 {
            return 0;
        }
        let keep = mem.read::<u8>(ext.wrapping_sub(1)) == b'.'
            && (total.wrapping_sub(ext_len).cast_signed() <= 1
                || mem.read::<i8>(ext.wrapping_sub(2)) >= 0);
        if !keep {
            mem.write(ext, b'.');
            ext = ext.wrapping_add(1);
            total = total.wrapping_add(1);
        }
        let count = ext_len.wrapping_sub(total).wrapping_add(n);
        game(mem, call, MS_STR_COPY, &[src, ext, count]);
        1
    }

    /// `0x007cfaa0(kind)` (`MsCameraKindChar`, name ours): the letter of
    /// a camera kind (1 `M`, 3 `E`, 5 `B`, 7 `V`, 17 `m`, 19 `e`, 21 `b`,
    /// 23 `v`, else `-`) in `al`, the rest of `eax` as the original leaves
    /// it (0, or `kind - 1`'s upper bytes out of range). The letter is
    /// found through the original's two jump tables.
    pub fn ms_camera_kind_char(mem: &impl Mem, kind: u32) -> u32 {
        let k = kind.wrapping_sub(1);
        if k > 0x16 {
            return (k & !0xff) | u32::from(b'-');
        }
        let idx = u32::from(mem.read::<u8>(mem.rebase(0x007c_fb04) + k));
        let target: u32 = mem.read(mem.rebase(0x007c_fae0) + 4 * idx);
        let first = mem.rebase(0x007c_faba);
        let letters = *b"MEBVmebv-";
        let at = (target.wrapping_sub(first) / 4) as usize;
        u32::from(letters.get(at).copied().unwrap_or(b'-'))
    }

    /// `MsDebugSetCameraMode(side, mode)` (`0x007cfb60`): the debug
    /// camera's channel (`MsCameraGetNum`) mode byte (`+0xf2`) on side
    /// `side`.
    pub fn ms_debug_set_camera_mode<M: Mem>(mem: &mut M, call: Call<'_, M>, side: u32, mode: u32) {
        let cam = u32::from(r8(mem, DEBUG_CAMERA));
        let ch = game(mem, call, MS_CAMERA_GET_NUM, &[cam]);
        let at = mem
            .rebase(CAMERAS + 0xf2)
            .wrapping_add(cam.wrapping_mul(0xda0))
            .wrapping_add(ch.wrapping_mul(0x130))
            .wrapping_add(side.wrapping_mul(0x61c));
        #[allow(clippy::cast_possible_truncation)] // the byte argument
        mem.write(at, mode as u8);
    }

    /// `0x007cfba0(i, mark, pos)` (`MsInfoWinSet`, name ours): info
    /// window `i` (0..3, when set): mark `+0x21`, position `pos` (three
    /// floats copied through the x87), `+0x22` = 2 and colours `+0x28`..
    /// `+0x30` by `i`'s parity and the colour flag.
    pub fn ms_info_win_set(mem: &mut impl Mem, i: u32, mark: u32, pos: u32) {
        let slot = mem.rebase(INFO_WINS).wrapping_add(i.wrapping_mul(4));
        let win: u32 = mem.read(slot);
        if win == 0 || i.cast_signed() >= 4 {
            return;
        }
        #[allow(clippy::cast_possible_truncation)] // the byte argument
        mem.write(win.wrapping_add(0x21), mark as u8);
        for k in 0..3 {
            let win: u32 = mem.read(slot);
            let v = ffx_bridge::x87::round_trip(f32::from_bits(mem.read(pos.wrapping_add(4 * k))));
            mem.write(win.wrapping_add(4 * k), v.to_bits());
        }
        let win: u32 = mem.read(slot);
        mem.write(win.wrapping_add(0x22), 2u8);
        let flag = r8(mem, INFO_WIN_FLAGS) & 1 != 0;
        let colours: [u32; 3] = match (i & 1 != 0, flag) {
            (true, true) => [0xff, 0, 0],
            (true, false) => [0xd2, 0x20, 0],
            (false, true) => [0, 0xff, 0],
            (false, false) => [0, 0xd2, 0x20],
        };
        for (k, c) in (0u32..).zip(colours) {
            let win: u32 = mem.read(slot);
            mem.write(win.wrapping_add(0x28 + 4 * k), c);
        }
    }

    pub const MS_GET_BATTLE_END_STATUS: u32 = 0x0079_28d0;
    pub const TK_SND_REPORT_STATUS: u32 = 0x0088_7b30;
    pub const MS_SEP_TRANS_CHECK_WAVE: u32 = 0x007a_04d0;
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const MS_BTL_READ_CUE_LEVEL: u32 = 0x007a_d180;
    pub const TO_DAPS_SET_POS: u32 = 0x0089_de30;
    pub const TO_DAPS_STRING: u32 = 0x0089_dec0;
    pub const SND_SQS_IOP_SOUND_ERROR_STATUS: u32 = 0x0088_7360;
    /// `0x008873b0`: a jump to `SqsIopSoundSystemStatus`.
    pub const SQS_IOP_SOUND_SYSTEM_STATUS_JMP: u32 = 0x0088_73b0;
    pub const MS_GET_SUMMON_ENERGY: u32 = 0x007b_2b00;
    pub const MS_GET_YOUJINBOU_COM: u32 = 0x007b_2b20;
    pub const MS_MENU_WEAK_ELEMENT: u32 = 0x007a_9a20;
    pub const MS_CHECK_CHR_MOTION_ALL: u32 = 0x0079_1630;
    pub const SND_GET_STREAM_STATE: u32 = 0x0088_64b0;
    pub const SND_IS_READ_STATUS: u32 = 0x0088_6650;
    pub const MS_CHECK_START_EFFECT: u32 = 0x007a_92c0;
    /// The debug status lines' formats (`.rdata`).
    pub const FMT_STATUS: [u32; 9] = [
        0x00b5_4bac,
        0x00b5_4bc8,
        0x00b5_4bdc,
        0x00b5_4be8,
        0x00b5_4bfc,
        0x00b5_4c10,
        0x00b5_4c28,
        0x00b5_4c34,
        0x00b5_4c4c,
    ];
    pub const FMT_STATUS_COUNT: u32 = 0x00b5_4c6c;
    /// The globals the status lines show.
    pub const BTL_STATUS: u32 = 0x0112_a8e0;
    pub const BTL_SCENE_NAME: u32 = 0x0112_c25a;
    pub const BTL_END_HOLD: u32 = 0x0112_ca01;
    pub const EFFECT_CAMERA_BUF_20: u32 = 0x0112_c24c;
    pub const BTL_MESSAGE_LAST: u32 = 0x0112_c250;
    pub const BTL_DIR_FREE: u32 = 0x0112_c9e7;
    pub const BTL_PARSED_COMMAND2: u32 = 0x0112_c8dc;
    pub const BTL_COMMAND_ACTOR: u32 = 0x0112_c9e0;
    pub const BTL_CAMERA_STATE: u32 = 0x0112_a8e4;
    pub const BTL_CAMERA_ONCE: u32 = 0x0112_a8e5;
    pub const BTL_LAST_MAGIC: u32 = 0x0112_c8ec;
    pub const BTL_RESET_WAIT2: u32 = 0x0112_c9d6;
    pub const BTL_STATUS_COUNTER: u32 = 0x0112_c8fc;

    fn sx8(mem: &impl Mem, at: u32) -> u32 {
        i32::from(mem.read::<i8>(at)).cast_unsigned()
    }

    /// `0x00792f20()` (`MsBattleProcess_DebugStatus`): the battle's debug
    /// status lines (`sprintf` into the frame buffer `buf`, 0x40 bytes at
    /// the original's `ebp - 0x44`, then `TODapsSetPos` and
    /// `TODapsString`): the battle and sound states, the scene name, the
    /// sound errors, the effect camera and message, the summon energies
    /// (with `g_BtlDirFree`), the command, the camera and effect waits,
    /// the commanding actor's state (with a magic), and a frame counter
    /// (when not negative, counted up).
    #[allow(clippy::too_many_lines)] // one straight run of lines, as the original
    pub fn ms_battle_debug_status<M: Mem>(mem: &mut M, call: Call<'_, M>, buf: u32) {
        let fmt = |mem: &M, k: usize| mem.rebase(FMT_STATUS[k]);
        let line = |mem: &mut M, call: Call<'_, M>, x: u32, y: u32, text: u32| {
            game(mem, &mut *call, TO_DAPS_SET_POS, &[x, y]);
            game(mem, call, TO_DAPS_STRING, &[text]);
        };
        let end = game(mem, &mut *call, MS_GET_BATTLE_END_STATUS, &[0]);
        let snd = game(mem, &mut *call, TK_SND_REPORT_STATUS, &[end]);
        let wave = game(mem, &mut *call, MS_SEP_TRANS_CHECK_WAVE, &[]);
        let cd = game(mem, &mut *call, GET_CD, &[]);
        let f: u32 = mem.read(cd.wrapping_add(8));
        let level = call(mem, f, &[1]);
        let cue = game(mem, &mut *call, MS_BTL_READ_CUE_LEVEL, &[level]);
        let status = sx8(mem, mem.rebase(BTL_STATUS));
        let f0 = fmt(mem, 0);
        import(
            mem,
            &mut *call,
            SPRINTF_SLOT,
            &[buf, f0, status, cue, level, wave, snd, end],
        );
        line(mem, &mut *call, 0xbc, 8, buf);
        let name = mem.rebase(BTL_SCENE_NAME);
        line(mem, &mut *call, 0x188, 8, name);
        let hold = sx8(mem, mem.rebase(BTL_END_HOLD));
        let e1 = game(mem, &mut *call, SND_SQS_IOP_SOUND_ERROR_STATUS, &[hold]);
        let e2 = game(mem, &mut *call, SQS_IOP_SOUND_SYSTEM_STATUS_JMP, &[e1]);
        let f1 = fmt(mem, 1);
        import(mem, &mut *call, SPRINTF_SLOT, &[buf, f1, e2, e1, hold]);
        line(mem, &mut *call, 0xec, 0x15, buf);
        let (cam, msg) = (r32(mem, EFFECT_CAMERA_BUF_20), r32(mem, BTL_MESSAGE_LAST));
        let f2 = fmt(mem, 2);
        import(mem, &mut *call, SPRINTF_SLOT, &[buf, f2, cam, msg]);
        line(mem, &mut *call, 0x140, 0x22, buf);
        let mut row = 3u32;
        if r8(mem, BTL_DIR_FREE) != 0 {
            let s17 = game(mem, &mut *call, MS_GET_SUMMON_ENERGY, &[0x11]);
            let s16 = game(mem, &mut *call, MS_GET_SUMMON_ENERGY, &[0x10]);
            let s15 = game(mem, &mut *call, MS_GET_SUMMON_ENERGY, &[0xf]);
            let y = game(mem, &mut *call, MS_GET_YOUJINBOU_COM, &[]);
            let f3 = fmt(mem, 3);
            import(mem, &mut *call, SPRINTF_SLOT, &[buf, f3, y, s15, s16, s17]);
            line(mem, &mut *call, 0x110, 0x2f, buf);
            row = 4;
        }
        let ypos = |row: u32| row.wrapping_mul(0xd).wrapping_add(8);
        if r32(mem, BTL_PARSED_COMMAND2) != 0xff {
            let actor = u32::from(r8(mem, BTL_COMMAND_ACTOR));
            let weak = game(mem, &mut *call, MS_MENU_WEAK_ELEMENT, &[actor]);
            let command = r32(mem, BTL_PARSED_COMMAND2);
            let actor = u32::from(r8(mem, BTL_COMMAND_ACTOR));
            let f4 = fmt(mem, 4);
            import(
                mem,
                &mut *call,
                SPRINTF_SLOT,
                &[buf, f4, actor, command, weak],
            );
            line(mem, &mut *call, 0xc8, ypos(row), buf);
        }
        let vals = [
            sx8(mem, mem.rebase(BTL_CAMERA_STATE)),
            sx8(mem, mem.rebase(BTL_CAMERA_ONCE)),
            sx8(mem, mem.rebase(EFFECT_WAIT)),
            u32::from(r8(mem, EFFECT_WAIT + 1)),
        ];
        let f5 = fmt(mem, 5);
        import(
            mem,
            &mut *call,
            SPRINTF_SLOT,
            &[buf, f5, vals[0], vals[1], vals[2], vals[3]],
        );
        line(mem, &mut *call, 0x164, ypos(row), buf);
        row += 1;
        if r32(mem, BTL_LAST_MAGIC).cast_signed() >= 0 {
            let idx = u32::from(r8(mem, BTL_COMMAND_ACTOR));
            let a = game(mem, &mut *call, MS_GET_CHR, &[idx]);
            let magic = r32(mem, BTL_LAST_MAGIC);
            let f6 = fmt(mem, 6);
            let v = sx8(mem, a.wrapping_add(0xdfb));
            import(mem, &mut *call, SPRINTF_SLOT, &[buf, f6, v, magic]);
            line(mem, &mut *call, 0x104, ypos(row), buf);
            let b412 = u32::from(mem.read::<u8>(a.wrapping_add(0x412)));
            let b432 = sx8(mem, a.wrapping_add(0x432));
            let b436 = sx8(mem, a.wrapping_add(0x436));
            let b508 = sx8(mem, a.wrapping_add(0x508));
            let m = game(mem, &mut *call, MS_CHECK_CHR_MOTION_ALL, &[0]);
            let f7 = fmt(mem, 7);
            import(
                mem,
                &mut *call,
                SPRINTF_SLOT,
                &[buf, f7, m, b508, b436, b432, b412],
            );
            line(mem, &mut *call, 0x158, ypos(row), buf);
            let st = game(mem, &mut *call, SND_GET_STREAM_STATE, &[]);
            let rd = game(mem, &mut *call, SND_IS_READ_STATUS, &[st]);
            let wait = sx8(mem, mem.rebase(BTL_RESET_WAIT2));
            let set: u32 = mem.read(a.wrapping_add(0xdc0));
            let chr = u32::from(mem.read::<u8>(a.wrapping_add(0xc)));
            let start = game(mem, &mut *call, MS_CHECK_START_EFFECT, &[chr, set]);
            let dfd = sx8(mem, a.wrapping_add(0xdfd));
            let f8 = fmt(mem, 8);
            import(
                mem,
                &mut *call,
                SPRINTF_SLOT,
                &[buf, f8, dfd, start, wait, rd, st],
            );
            line(mem, &mut *call, 0x110, ypos(row + 1), buf);
            row += 2;
        }
        let n = r32(mem, BTL_STATUS_COUNTER);
        if n.cast_signed() >= 0 {
            let n = n.wrapping_add(1);
            w32(mem, BTL_STATUS_COUNTER, n);
            let f = mem.rebase(FMT_STATUS_COUNT);
            import(mem, &mut *call, SPRINTF_SLOT, &[buf, f, n]);
            line(mem, call, 0x1c4, ypos(row), buf);
        }
    }

    /// `0x007cfca0()` (`MsInfoWinsInit`, name ours): the four info
    /// windows allocated (`0x007c8bb0(0)`), their marks cleared.
    pub fn ms_info_wins_init<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        for k in 0..4u32 {
            let w = game(mem, &mut *call, DEBUG_WIN_ALLOC, &[0]);
            let slot = mem.rebase(INFO_WINS) + 4 * k;
            mem.write(slot, w);
            mem.write(w.wrapping_add(0x21), 0u8);
        }
    }

    // ---- batch 3: the camera and battle debug screens' helpers ----

    pub const MS_BTL_POS_FREE: u32 = 0x007c_9210;
    pub const MS_GET_CHR_BTL_POS: u32 = 0x0079_48e0;
    pub const CH_SET_POS3: u32 = 0x0082_b480;
    pub const MS_STR_EQUAL_NO_CASE: u32 = 0x007c_f330;
    pub const ATEL_GET_TOTAL_ACTOR_NUMBER2: u32 = 0x0086_c510;
    pub const MS_CAMERA_GET_ID: u32 = 0x007b_d300;
    pub const MS_DEBUG_GET_CAMERA_MODE: u32 = 0x007c_fb20;
    pub const MS_DEBUG_CAMERA_SYNC: u32 = 0x007d_7130;
    pub const MS_POS_PARSE_TOKEN: u32 = 0x007c_c420;
    pub const READ_SKIP_LINE: u32 = 0x007c_f380;
    pub const READ_OPEN: u32 = 0x007c_d5a0;
    pub const MS_POS_PARSE: u32 = 0x007c_d0f0;
    /// `free`'s import table slot.
    pub const FREE_SLOT: u32 = 0x00b0_c370;
    /// The position file editor's buffer (`ms_malloc`ed), freed with the
    /// table.
    pub const POS_EDIT_BUF: u32 = 0x0113_f510;
    /// The debug screens' scene path: the scene's directory name and the
    /// path built from it (`0x0113e4e8`), its cursor.
    pub const DBG_SCENE_DIR: u32 = 0x0113_f410;
    pub const DBG_SCENE_ROOT: u32 = 0x00c4_47a0;
    pub const DBG_SCENE_PATH: u32 = 0x0113_e4e8;
    pub const DBG_SCENE_PATH_POS: u32 = 0x0113_e4e0;
    pub const FMT_PATH_DIR: u32 = 0x00b5_377c;
    /// The reference point editor: its value in 0..1 (`0x0113ed48`), the
    /// point and the direction it shows.
    pub const REF_VALUE: u32 = 0x0113_ed48;
    pub const REF_POINT: u32 = 0x0113_ed28;
    pub const REF_DIR: u32 = 0x0113_f3d4;
    /// The constants `0x007d23d0` scales by (`.rdata`, 0.1 and 10.0).
    pub const REF_STEP: u32 = 0x00b5_51e0;
    pub const REF_SCALE: u32 = 0x00b9_22b0;
    /// The debug camera screen: the camera channel copied (0x18c words
    /// at `0x0113ed7c`) and the channel it came from.
    pub const DBG_CAMERA_COPY: u32 = 0x0113_ed7c;
    pub const DBG_CAMERA_FROM: u32 = 0x0113_e660;
    pub const DBG_CAMERA_SCREEN: u32 = 0x0113_9f10;
    /// The debug save files' names (`.rdata`).
    pub const FMT_MON_FILE: u32 = 0x00b5_7270;
    pub const USER_MASAKI: u32 = 0x00b5_62f0;
    pub const FMT_PLY_FILES: [u32; 4] = [0x00b5_71c0, 0x00b5_71e8, 0x00b5_7214, 0x00b5_7240];
    pub const BTL_UNDERWATER: u32 = 0x0112_c9ce;
    pub const MSG_POS_OPEN: u32 = 0x00b5_5938;

    /// `0x007d7450()` (`MsBtlPosEditFree`, name ours): the position
    /// editor's buffer freed (`ms_free`, cleared), then the tail call of
    /// `MsBtlPosFree_007c9210`.
    pub fn ms_btl_pos_edit_free<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let p = r32(mem, POS_EDIT_BUF);
        if p != 0 {
            game(mem, &mut *call, MS_FREE, &[p]);
            w32(mem, POS_EDIT_BUF, 0);
        }
        game(mem, call, MS_BTL_POS_FREE, &[]);
    }

    /// `0x007d79e0(kind)` (`MsDebugIsTextMode`, name ours): -1 for 3..8
    /// and 0xb, else 0.
    #[must_use]
    pub fn ms_debug_is_text_mode(kind: u32) -> u32 {
        if matches!(kind, 3..=8 | 0xb) {
            u32::MAX
        } else {
            0
        }
    }

    /// `0x007d70d0()` (`MsDebugScenePath`, name ours): the scene path
    /// `"<root><dir>/"` (or the root alone without a directory) and its
    /// cursor 0.
    pub fn ms_debug_scene_path<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let (path, root) = (mem.rebase(DBG_SCENE_PATH), mem.rebase(DBG_SCENE_ROOT));
        if r8(mem, DBG_SCENE_DIR) == 0 {
            let fmt = mem.rebase(FMT_S);
            import(mem, call, SPRINTF_SLOT, &[path, fmt, root]);
        } else {
            let (fmt, dir) = (mem.rebase(FMT_PATH_DIR), mem.rebase(DBG_SCENE_DIR));
            import(mem, call, SPRINTF_SLOT, &[path, fmt, root, dir]);
        }
        w32(mem, DBG_SCENE_PATH_POS, 0);
    }

    /// `0x007d73e0()` (`MsDebugPlaceAll`, name ours): every actor 0..0x1e
    /// with a model goes to its battle position (`MsGetChrBtlPos(i, pos)`,
    /// `pos` the frame's 3 floats at the original's `ebp - 0x14`, passed
    /// to `Ch_SetPos3(model, x, y, z)` through the x87).
    pub fn ms_debug_place_all<M: Mem>(mem: &mut M, call: Call<'_, M>, pos: u32) {
        for i in 0..0x1f {
            let actor = game(mem, &mut *call, MS_GET_CHR, &[i]);
            let model: u32 = mem.read(actor);
            if model == 0 {
                continue;
            }
            game(mem, &mut *call, MS_GET_CHR_BTL_POS, &[i, pos]);
            let v: [u32; 3] = mem.read(pos);
            let w = v.map(|b| ffx_bridge::x87::round_trip(f32::from_bits(b)).to_bits());
            let model: u32 = mem.read(actor);
            game(mem, &mut *call, CH_SET_POS3, &[model, w[0], w[1], w[2]]);
        }
    }

    /// `0x007d2f10(name, list)` (`MsDebugFindName`, name ours): the index
    /// of `name` in a null-ended list of strings (`MsStrEqualNoCase`), or
    /// -1.
    pub fn ms_debug_find_name<M: Mem>(mem: &mut M, call: Call<'_, M>, name: u32, list: u32) -> u32 {
        let mut i = 0u32;
        loop {
            let s: u32 = mem.read(list.wrapping_add(4 * i));
            if s == 0 {
                return u32::MAX;
            }
            if game(mem, &mut *call, MS_STR_EQUAL_NO_CASE, &[name, s]) != 0 {
                return i;
            }
            i = i.wrapping_add(1);
        }
    }

    /// The reference point and direction (6 floats) set through the x87
    /// from `src`.
    fn set_ref_vectors(mem: &mut impl Mem, src: [u32; 6]) {
        let dst = [
            REF_POINT,
            REF_POINT + 4,
            REF_POINT + 8,
            REF_DIR,
            REF_DIR + 4,
            REF_DIR + 8,
        ];
        for (to, v) in dst.into_iter().zip(src) {
            let v = ffx_bridge::x87::round_trip(f32::from_bits(v));
            w32(mem, to, v.to_bits());
        }
    }

    /// `0x007d36b0(cam)` (`MsDebugRefFromCamera`, name ours): the
    /// reference point and direction from camera `cam`'s side 1 (its look
    /// point `+0x30`, its direction `+0x20` in the record at
    /// `0x01137b8c + 0xda0 cam`).
    pub fn ms_debug_ref_from_camera(mem: &mut impl Mem, cam: u32) {
        let rec = mem
            .rebase(0x0113_7b8c)
            .wrapping_add(cam.wrapping_mul(0xda0));
        let src =
            [0x30u32, 0x34, 0x38, 0x20, 0x24, 0x28].map(|o| mem.read::<u32>(rec.wrapping_add(o)));
        set_ref_vectors(mem, src);
    }

    /// A float clamped to 0..1 on the x87 (`fcom`): below 0 gives 0,
    /// above 1 gives 1, else (NaN too) the value as loaded.
    fn clamp01(bits: u32) -> (u32, bool) {
        let x = ffx_bridge::x87::load(bits);
        if x < 0.0 {
            (0, true)
        } else if 1.0 < x {
            (1.0f32.to_bits(), true)
        } else {
            (ffx_bridge::x87::store(x), false)
        }
    }

    /// `0x007d3420(value, point[3], dir[3])` (`MsDebugRefSet`, name ours):
    /// the reference value (clamped to 0..1; the original stores the
    /// clamped value into its own `value` argument slot, `slots`), point
    /// and direction set. Returns the value (`ST0`), as `f32` bits.
    pub fn ms_debug_ref_set(mem: &mut impl Mem, slots: u32, args: [u32; 7]) -> u32 {
        let (v, clamped) = clamp01(args[0]);
        if clamped {
            mem.write(slots, v);
        }
        w32(mem, REF_VALUE, v);
        let mut src = [0u32; 6];
        src.copy_from_slice(&args[1..]);
        set_ref_vectors(mem, src);
        v
    }

    /// `0x007d2f50(delta)` (`MsDebugRefAdd`, name ours): the reference
    /// value plus `delta` (x87, 53 bits), clamped to 0..1, stored and
    /// returned (`f32` bits); a `delta` of 0 returns the value untouched.
    pub fn ms_debug_ref_add(mem: &mut impl Mem, delta: u32) -> u32 {
        let old = r32(mem, REF_VALUE);
        let d = ffx_bridge::x87::load(delta);
        if d == 0.0 {
            return ffx_bridge::x87::round_trip(f32::from_bits(old)).to_bits();
        }
        let sum = ffx_bridge::x87::store(ffx_bridge::x87::add(d, ffx_bridge::x87::load(old)));
        w32(mem, REF_VALUE, sum);
        let (v, clamped) = clamp01(sum);
        if clamped {
            w32(mem, REF_VALUE, v);
        }
        v
    }

    /// `0x007d23d0()` (`MsDebugRefRound`, name ours): the reference value
    /// rounded to a tenth toward zero (`/ 0.1`, `ftol2_sse`, `fild`, `/
    /// 10.0`, each rounding as the x87 and the `f32` stores do).
    pub fn ms_debug_ref_round(mem: &mut impl Mem) {
        let step: f64 = mem.read(mem.rebase(REF_STEP));
        let scale: f64 = mem.read(mem.rebase(REF_SCALE));
        let x = ffx_bridge::x87::div(ffx_bridge::x87::load(r32(mem, REF_VALUE)), step);
        let n = ffx_bridge::x87::ftol(mem, x).cast_signed();
        #[allow(clippy::cast_precision_loss)] // fild then fstp dword: the f32 rounding
        let f = n as f32;
        let v = ffx_bridge::x87::store(ffx_bridge::x87::div(f64::from(f), scale));
        w32(mem, REF_VALUE, v);
    }

    /// `MsSetFileName(out, n)` (`0x007dce80`): a monster motion file's
    /// name (`n & 0x3ff` twice, the user name in a debug build, else
    /// null, and the country).
    pub fn ms_set_file_name<M: Mem>(mem: &mut M, call: Call<'_, M>, out: u32, n: u32) {
        let n = n & 0x3ff;
        let user = if game(mem, &mut *call, FFX_IS_DEBUG, &[]) != 0 {
            game(mem, &mut *call, SG_GET_USER_NAME, &[])
        } else {
            0
        };
        let (fmt, country) = (mem.rebase(FMT_MON_FILE), mem.rebase(COUNTRY_NAME));
        import(mem, call, SPRINTF_SLOT, &[out, fmt, user, country, n, n]);
    }

    /// `MsGetPlySaveName(out, name)` (`0x007dcec0`): a party motion
    /// file's name: the user's (null outside a debug build) program
    /// directory for user `masaki`, else the battle directory with the
    /// country; the underwater set (`g_BtlUnderwater`) for underwater.
    pub fn ms_get_ply_save_name<M: Mem>(mem: &mut M, call: Call<'_, M>, out: u32, name: u32) {
        let user = if game(mem, &mut *call, FFX_IS_DEBUG, &[]) != 0 {
            game(mem, &mut *call, SG_GET_USER_NAME, &[])
        } else {
            0
        };
        let water = usize::from(r8(mem, BTL_UNDERWATER) != 0);
        let masaki = mem.rebase(USER_MASAKI);
        let program = game(mem, &mut *call, MS_STR_EQUAL_NO_CASE, &[user, masaki]) != 0;
        if program {
            let fmt = mem.rebase(FMT_PLY_FILES[2 * water]);
            import(mem, call, SPRINTF_SLOT, &[out, fmt, user, name, name]);
        } else {
            let (fmt, country) = (
                mem.rebase(FMT_PLY_FILES[2 * water + 1]),
                mem.rebase(COUNTRY_NAME),
            );
            import(
                mem,
                call,
                SPRINTF_SLOT,
                &[out, fmt, user, country, name, name],
            );
        }
    }

    /// `0x007d9ab0(id, kind)` (`MsDebugActorCount`, name ours): for a
    /// battle actor id (`id & 0xf0000 == 0x50000`) 0x1f actors and kind 0;
    /// else the event's actor count (at least 1) and kind 1. `*kind` set
    /// when `kind` isn't null.
    pub fn ms_debug_actor_count<M: Mem>(mem: &mut M, call: Call<'_, M>, id: u32, kind: u32) -> u32 {
        let (n, k) = if id & 0xf_0000 == 0x5_0000 {
            (0x1f, 0)
        } else {
            let n = game(mem, call, ATEL_GET_TOTAL_ACTOR_NUMBER2, &[]);
            (if n.cast_signed() > 0 { n } else { 1 }, 1)
        };
        if kind != 0 {
            mem.write(kind, k);
        }
        n
    }

    /// `0x007d12a0(path, base)` (`MsDebugPathAfterBase`, name ours): past
    /// the part of `path` it shares with `base`, the rest after its next
    /// `/`; where there is none, the end of the shared part.
    pub fn ms_debug_path_after_base(mem: &impl Mem, path: u32, base: u32) -> u32 {
        let mut p = path;
        let mut c: u8 = mem.read(p);
        if c != 0 {
            let off = base.wrapping_sub(path);
            while c == mem.read::<u8>(p.wrapping_add(off)) {
                p = p.wrapping_add(1);
                c = mem.read(p);
                if c == 0 {
                    break;
                }
            }
        }
        let mut q = p;
        let mut c: u8 = mem.read(q);
        while c != 0 {
            if c == b'/' {
                return q.wrapping_add(1);
            }
            q = q.wrapping_add(1);
            c = mem.read(q);
        }
        p
    }

    /// `0x007d1220()` (`MsDebugCameraCopy`, name ours): in the camera
    /// modes 8 and 9, the debug camera channel's data (`+8` of its
    /// `+0x70c` record: 0x18c words) copied for the camera screen, and
    /// when it came from elsewhere than last time, `0x007d7130` run; the
    /// source's end kept.
    pub fn ms_debug_camera_copy<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let screen = r32(mem, DBG_CAMERA_SCREEN);
        let cam_at = mem
            .rebase(DEBUG_CAMERA)
            .wrapping_add(screen.wrapping_mul(0x1c0));
        let cam = u32::from(mem.read::<u8>(cam_at));
        game(mem, &mut *call, MS_CAMERA_GET_ID, &[cam]);
        let ch = game(mem, &mut *call, MS_CAMERA_GET_NUM, &[cam]);
        let rec = mem
            .rebase(0x0113_7c4c)
            .wrapping_add(cam.wrapping_mul(0xda0))
            .wrapping_add(ch.wrapping_mul(0x130));
        let mode = game(mem, &mut *call, MS_DEBUG_GET_CAMERA_MODE, &[1]);
        if mode.wrapping_sub(8) > 1 {
            return;
        }
        let mut src: u32 = mem.read(rec.wrapping_add(8));
        let dst = mem.rebase(DBG_CAMERA_COPY);
        for k in 0..0x18c {
            let v: u32 = mem.read(src);
            mem.write(dst + 4 * k, v);
            src = src.wrapping_add(4);
        }
        if src != r32(mem, DBG_CAMERA_FROM) {
            game(mem, &mut *call, MS_DEBUG_CAMERA_SYNC, &[]);
        }
        w32(mem, DBG_CAMERA_FROM, src);
    }

    /// `0x007cd0f0(reader)` (`MsBtlPosParse`, name ours): every token of
    /// the reader (into the frame buffer `tok`, 0x100 bytes at the
    /// original's `ebp - 0x104`) handed to `0x007cc420(reader, tok)`, the
    /// empty ones' lines skipped (counted in `POS_LINE`) until the end.
    pub fn ms_btl_pos_parse<M: Mem>(mem: &mut M, call: Call<'_, M>, tok: u32, reader: u32) {
        if reader == 0 {
            return;
        }
        loop {
            game(mem, &mut *call, READ_TOKEN, &[reader, tok, 0x100]);
            if mem.read::<u8>(tok) != 0 {
                game(mem, &mut *call, MS_POS_PARSE_TOKEN, &[reader, tok]);
                continue;
            }
            let line = mem.rebase(POS_LINE);
            if game(mem, &mut *call, READ_SKIP_LINE, &[reader, line]).cast_signed() < 0 {
                return;
            }
        }
    }

    /// `0x007ca4a0(name)` (`MsBtlPosLoad`, name ours): the position script
    /// `name` read (`0x007cd5a0` into the frame's reader at the
    /// original's `ebp - 0x14`) and parsed (`0x007cd0f0`, with `name` as
    /// the error file name and the line count from 0); a reader that kept
    /// its file and a position past 0 writes the text back; the buffer
    /// freed, the file closed: 1. A file that doesn't open: the error
    /// lines, error 2, 0.
    pub fn ms_btl_pos_load<M: Mem>(mem: &mut M, call: Call<'_, M>, reader: u32, name: u32) -> u32 {
        game(mem, &mut *call, READ_OPEN, &[reader, name]);
        if mem.read::<i32>(reader.wrapping_add(8)) < 0 {
            pos_error(mem, call, MSG_POS_OPEN, name, 2);
            return 0;
        }
        let file_name = r32(mem, POS_FILE_NAME);
        w32(mem, POS_FILE_NAME, name);
        w32(mem, POS_LINE, 0);
        game(mem, &mut *call, MS_POS_PARSE, &[reader]);
        let pos: i32 = mem.read(reader.wrapping_add(0xc));
        w32(mem, POS_FILE_NAME, file_name);
        if pos > 0 && mem.read::<u32>(reader.wrapping_add(4)) != 0 {
            let (buf, fd): (u32, u32) = (mem.read(reader), mem.read(reader.wrapping_add(8)));
            game(mem, &mut *call, SCE_WRITE, &[fd, buf, pos.cast_unsigned()]);
        }
        let buf: u32 = mem.read(reader);
        if buf != 0 {
            import(mem, &mut *call, FREE_SLOT, &[buf]);
        }
        let fd: i32 = mem.read(reader.wrapping_add(8));
        if fd > 0 {
            game(mem, call, SCE_CLOSE, &[fd.cast_unsigned()]);
        }
        1
    }
}
