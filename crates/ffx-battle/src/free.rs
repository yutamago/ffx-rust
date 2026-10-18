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
        let m = crate::minor::bridge::ATEL_MANAGERS.ptr(mem).addr;
        for k in 0..2 {
            for o in [4, 0xc] {
                let p = mem.read(m + 0x54 * k + o);
                free(mem, &mut *call, p);
            }
        }
        crate::flow::bridge::ms_atel_cam_free(mem, &mut remove);
        remove(mem, 2);
        remove(mem, 3);
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
}
