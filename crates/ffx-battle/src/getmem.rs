//! The battle's setup leaves (`docs/rewrite-plan.md`, M2): buffers
//! `MsBattleExe` (`0x00781020`) and `MsBattleInit` (`0x00781700`) take
//! from the heap at the start of a battle, and small helpers they call.
//!
//! The heap (`user_malloc`) and script's `AtelGetWaterBattleEffect` go to
//! the game's code through `call`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::exec::bridge::Call;

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base());
        call(mem, f, args)
    }

    /// `user_malloc(n)` (the game's heap).
    pub const USER_MALLOC: u32 = 0x0068_7020;
    /// `AtelGetWaterBattleEffect()` (script).
    pub const ATEL_GET_WATER_BATTLE_EFFECT: u32 = 0x0086_c8a0;
    /// The second script record, which `0x00782670` moves to the first.
    pub const SCRIPT_RECORD1: Global<[u32; 8]> = Global::new(0x0112_a9a4);

    /// `MsGetSaveConfigEnglish()` (`0x007851d0`): 0 (the PC's
    /// configuration has no English switch). Three bytes, too small for
    /// the hook's jump: the original stays in place.
    #[must_use]
    pub const fn ms_get_save_config_english() -> u32 {
        0
    }

    /// `MsNumberGetMem()` (`0x0079f9b0`): without one, the damage numbers'
    /// pool (0xa00 bytes, `user_malloc`, cleared) and an empty number list
    /// (its tail the list's head).
    pub fn ms_number_get_mem<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        use crate::minor::bridge::{NUMBER_LIST, NUMBER_TAIL};
        if crate::misc::bridge::NUMBERS.get(mem) != 0 {
            return;
        }
        let p = game(mem, call, USER_MALLOC, &[0xa00]);
        crate::misc::bridge::NUMBERS.set(mem, p);
        crate::util::bridge::ms_struct_clear(mem, p, 0xa00);
        NUMBER_LIST.set(mem, 0);
        let head = NUMBER_LIST.ptr(mem).addr;
        NUMBER_TAIL.set(mem, head);
    }

    /// `MsBtlGetItemMem()` (`0x00799fb0`): the two item buffers
    /// (`user_malloc`): 0x21e8 bytes cleared and 0x8a88 bytes of 0xff
    /// halfwords, each when it came.
    pub fn ms_btl_get_item_mem<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        use crate::misc::bridge::{ITEM_MEM, ITEM_MEM2};
        let p = game(mem, &mut *call, USER_MALLOC, &[0x21e8]);
        ITEM_MEM.set(mem, p);
        if p != 0 {
            crate::util::bridge::ms_struct_clear(mem, p, 0x21e8);
        }
        let p = game(mem, call, USER_MALLOC, &[0x8a88]);
        ITEM_MEM2.set(mem, p);
        if p != 0 {
            crate::util::bridge::ms_struct_fill(mem, p, 0x8a88, 0xff);
        }
    }

    /// `MsBtlScriptRecordShift_00782670()`: script record 1 copied over
    /// record 0 (`g_BtlScriptRecords`), out of battle (`g_BtlStatus` 0),
    /// record 1's `+4` cleared.
    pub fn ms_btl_script_record_shift(mem: &mut impl Mem) {
        let r = SCRIPT_RECORD1.get(mem);
        let at = crate::atel::bridge::SCRIPT_RECORDS.ptr(mem).addr;
        mem.write(at, r);
        crate::state::bridge::STATUS.set(mem, 0);
        mem.write(SCRIPT_RECORD1.ptr(mem).addr + 4, 0u32);
    }

    /// `MsBtlEncountEffect_007829b0(kind, &out)`: for an encounter kind
    /// (-1 none; 1-3; others) and whether the battle is under water
    /// (`AtelGetWaterBattleEffect`): -1/0 for none, 1/0x65 (3/0x3f under
    /// water) for kinds 1-3, else 0/0x31 (2/0x31). The second value goes to
    /// `out` unless it is null; the first is returned.
    pub fn ms_btl_encount_effect<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        kind: u32,
        out: u32,
    ) -> u32 {
        let water = game(mem, call, ATEL_GET_WATER_BATTLE_EFFECT, &[]) != 0;
        let (r, v) = if kind == u32::MAX {
            (u32::MAX, 0)
        } else if kind.wrapping_sub(1) <= 2 {
            if water { (3, 0x3f) } else { (1, 0x65) }
        } else if water {
            (2, 0x31)
        } else {
            (0, 0x31)
        };
        if out != 0 {
            mem.write(out, v);
        }
        r
    }

    /// `GetCD()` (system), `MsGetCheckEnglish()`, `MsGetSaveConfigEnglish()`
    /// (both called through the game: the first asks the menu's language),
    /// `dbgPrintf(fmt, ...)` (debug).
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const MS_GET_CHECK_ENGLISH: u32 = 0x0078_4e60;
    pub const MS_GET_SAVE_CONFIG_ENGLISH: u32 = 0x0078_51d0;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// `MsGetFileSize`'s two debug formats (`.rdata`).
    pub const FILE_SIZE_FORMATS: [u32; 2] = [0x00b5_4790, 0x00b5_479c];

    /// The CD object's method at `slot` called with `args` (`GetCD()`'s
    /// object, its function pointers at `+0x10` select a directory, `+0x14`
    /// a file's size, `+0x58` its name).
    fn cd<M: Mem>(mem: &mut M, call: Call<'_, M>, slot: u32, args: &[u32]) -> u32 {
        let obj = game(mem, &mut *call, GET_CD, &[]);
        let f: u32 = mem.read(obj.wrapping_add(slot));
        call(mem, f, args)
    }

    /// `MsGetFileSize(dir, dir2, file)` (`0x00782a20`): the size of `file`
    /// in directory `dir` (CD methods `+0x10`, `+0x14`); while
    /// `MsGetCheckEnglish` the size in `dir2` too (directory `dir` again
    /// unless `MsGetSaveConfigEnglish`): the larger (signed). For `dir` 0xe
    /// each size is printed with the file's name (`dbgPrintf`).
    pub fn ms_get_file_size<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        dir: u32,
        dir2: u32,
        file: u32,
    ) -> u32 {
        let size_in = |mem: &mut M, call: Call<'_, M>, d: u32, k: usize| {
            cd(mem, &mut *call, 0x10, &[d]);
            let n = cd(mem, &mut *call, 0x14, &[file]);
            if dir == 0xe {
                let name = cd(mem, &mut *call, 0x58, &[file]);
                let fmt = FILE_SIZE_FORMATS[k]
                    .wrapping_sub(IMAGE_BASE)
                    .wrapping_add(mem.image_base());
                game(mem, &mut *call, DBG_PRINTF, &[fmt, name, n]);
            }
            n
        };
        let first = size_in(mem, &mut *call, dir, 0);
        let mut second = 0;
        if game(mem, &mut *call, MS_GET_CHECK_ENGLISH, &[]) != 0 {
            second = size_in(mem, &mut *call, dir2, 1);
            if game(mem, &mut *call, MS_GET_SAVE_CONFIG_ENGLISH, &[]) == 0 {
                cd(mem, &mut *call, 0x10, &[dir]);
            }
        }
        if first.cast_signed() > second.cast_signed() {
            first
        } else {
            second
        }
    }

    /// `MsBattleExeSetIndex(both, file)` (`0x00781550`): `file`'s size in
    /// directory 0xe ([`ms_get_file_size`] with 0xb as the second when
    /// `both`).
    pub fn ms_battle_exe_set_index<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        both: u32,
        file: u32,
    ) -> u32 {
        if both != 0 {
            return ms_get_file_size(mem, call, 0xe, 0xb, file);
        }
        cd(mem, &mut *call, 0x10, &[0xe]);
        cd(mem, call, 0x14, &[file])
    }

    /// `yiAssert(cond, msg, fmt, ...)` (map) and its two strings.
    pub const YI_ASSERT: u32 = 0x0090_ae50;
    pub const EFFECT_SYNC_ERROR: [u32; 2] = [0x00b5_4a24, 0x00b5_4a1c];

    /// `MsEventEffectFree()` (`0x00788b90`): an event effect still running
    /// (`g_BtlEventEffectStat`) is reported (`yiAssert(1, "EFFECT SYNC
    /// ERROR", "ID %x", id)`), put in state 6 and ended
    /// (`MsEventEffectProcess`): 1. Else 0.
    pub fn ms_event_effect_free<M: Mem>(
        mem: &mut M,
        s: &mut crate::atel::bridge::Script<'_, M>,
    ) -> u32 {
        use crate::state::bridge::{EVENT_EFFECT_B, EVENT_EFFECT_STAT};
        if EVENT_EFFECT_STAT.get(mem) == 0 {
            return 0;
        }
        let [msg, fmt] =
            EFFECT_SYNC_ERROR.map(|a| a.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base()));
        let id = EVENT_EFFECT_B.get(mem);
        game(mem, &mut *s.call, YI_ASSERT, &[1, msg, fmt, id]);
        EVENT_EFFECT_STAT.set(mem, 6);
        crate::battle_end::bridge::ms_event_effect_process(mem, s);
        1
    }

    /// `MsBtlChrGetMem()` (`0x00793530`): the 0x1f actors' block (0x1e270
    /// bytes, the monsters from `+0x13740`), the weapon records (0xdce0)
    /// and the extra block (0x24) from the heap, then `MsBtlChrInit`.
    pub fn ms_btl_chr_get_mem<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        use crate::actor::bridge::{CHR_TOP, MON_TOP, WEAPON_TOP};
        let p = game(mem, &mut *call, USER_MALLOC, &[0x1e270]);
        CHR_TOP.set(mem, p);
        MON_TOP.set(mem, p.wrapping_add(0x13740));
        let p = game(mem, &mut *call, USER_MALLOC, &[0xdce0]);
        WEAPON_TOP.set(mem, p);
        let p = game(mem, call, USER_MALLOC, &[0x24]);
        crate::flow::bridge::CHR_EXTRA.set(mem, p);
        crate::flow::bridge::ms_btl_chr_init(mem);
    }

    /// `MsBtlListCDpos(field, &group, &slot)` (`0x0079d0e0`): the formation
    /// number of `slot` in `group` of `field`'s encounters: the field
    /// record's first number (`+4`, `i16`) plus the sizes of the groups
    /// before plus `slot`; `group` and `slot` out of range become 0 first.
    pub fn ms_btl_list_cd_pos(mem: &mut impl Mem, field: u32, group: u32, slot: u32) -> u32 {
        use crate::flow::bridge::ms_btl_list_group;
        use crate::minor::bridge::{ms_btl_list_encount, ms_btl_list_field};
        let f = field.cast_signed();
        let rec = ms_btl_list_field(mem, f);
        let g: i32 = mem.read(group);
        if g < 0 || g >= i32::from(mem.read::<u8>(ms_btl_list_encount(mem, f).wrapping_add(1))) {
            mem.write(group, 0u32);
        }
        let g: i32 = mem.read(group);
        let mut sum = 0u32;
        for i in 0..g.max(0) {
            sum = sum.wrapping_add(u32::from(mem.read::<u8>(ms_btl_list_group(mem, f, i))));
        }
        let k: i32 = mem.read(slot);
        if k < 0 || k >= i32::from(mem.read::<u8>(ms_btl_list_group(mem, f, g))) {
            mem.write(slot, 0u32);
        }
        let k: u32 = mem.read(slot);
        let first = i32::from(mem.read::<i16>(rec.wrapping_add(4))).cast_unsigned();
        first.wrapping_add(k).wrapping_add(sum)
    }

    /// `MsRegSEplay2(i, se)` (battle, through the game) and
    /// `op_et_encount_effect_on(kind, id)` (vfx).
    pub const MS_REG_SE_PLAY2: u32 = 0x007a_0140;
    pub const OP_ET_ENCOUNT_EFFECT_ON: u32 = 0x007f_c590;
    /// The encounter effects started (`MsEtEffectEncount` counts them).
    pub const ENCOUNT_EFFECTS: Global<u8> = Global::new(0x0112_c028);

    /// `MsEtEffectEncount(kind, se)` (`0x0079e5e0`): unless a soft reset
    /// waits (`g_BtlResetWait`), sound `se` (`MsRegSEplay2(0xff, se)`) and
    /// the next encounter effect (`MsGetEffID(n, 0xa, 0)`, the count
    /// [`ENCOUNT_EFFECTS`] up) on (`op_et_encount_effect_on(kind, id)`); its
    /// ID is the wait and the result. Else 0.
    pub fn ms_et_effect_encount<M: Mem>(mem: &mut M, call: Call<'_, M>, kind: u32, se: u32) -> u32 {
        let wait = crate::process::bridge::RESET_WAIT;
        if wait.get(mem) != 0 {
            return 0;
        }
        game(mem, &mut *call, MS_REG_SE_PLAY2, &[0xff, se]);
        let n = ENCOUNT_EFFECTS.get(mem);
        let id = crate::util::bridge::ms_get_eff_id(u32::from(n), 0xa, 0);
        ENCOUNT_EFFECTS.set(mem, n.wrapping_add(1));
        game(mem, call, OP_ET_ENCOUNT_EFFECT_ON, &[kind, id]);
        wait.set(mem, id);
        id
    }

    /// `op_et_battle_effect_init()` (vfx).
    pub const OP_ET_BATTLE_EFFECT_INIT: u32 = 0x007f_b1e0;

    /// `MsEtEffectSet()` (`0x0079e7a0`): the battle's effects set up
    /// (`op_et_battle_effect_init`), their owner the read effects
    /// (`g_BtlEtEffectRead`, also the result).
    pub fn ms_et_effect_set<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        game(mem, call, OP_ET_BATTLE_EFFECT_INIT, &[]);
        let r = crate::misc::bridge::ET_EFFECT_READ.get(mem);
        crate::status::bridge::EFFECT_OWNER.set(mem, r);
        r
    }
}
