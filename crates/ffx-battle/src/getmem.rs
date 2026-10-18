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
}
