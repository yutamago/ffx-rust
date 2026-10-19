//! `ffx-battle`: random numbers, the save block, actors, the battle camera,
//! battle globals and helpers, damage.

use ffx_battle::action::bridge as action;
use ffx_battle::after_damage::bridge as afterdmg;
use ffx_battle::atel::bridge as atel;
use ffx_battle::battle_end::bridge as bend;
use ffx_battle::chr_damage::bridge as chrdmg;
use ffx_battle::damage_end::bridge as dmgend;
use ffx_battle::death::bridge as death;
use ffx_battle::effect::bridge as effect;
use ffx_battle::exchange::bridge as exchange;
use ffx_battle::exe::bridge as exe;
use ffx_battle::exec::bridge as exec;
use ffx_battle::free::bridge as free;
use ffx_battle::gear::bridge as gear;
use ffx_battle::getmem::bridge as getmem;
use ffx_battle::init::bridge as init;
use ffx_battle::input::bridge as input;
use ffx_battle::limit::bridge as limit;
use ffx_battle::motion::bridge as motion;
use ffx_battle::parse::bridge as parse;
use ffx_battle::predict::bridge as predict;
use ffx_battle::process::bridge as process;
use ffx_battle::read::bridge as read;
use ffx_battle::reward::bridge as reward;
use ffx_battle::save_param::bridge as sparam;
use ffx_battle::screen::bridge as screen;
use ffx_battle::shake::bridge as shake;
use ffx_battle::special::bridge as special;
use ffx_battle::status::bridge as status;
use ffx_battle::view::bridge as view;
use ffx_battle::visible::bridge as visible;
use ffx_battle::{actor::bridge as actor, camera::bridge as camera, command::bridge as cmd};
use ffx_battle::{
    body::bridge as body, menu::bridge as menu, target::bridge as target, vector::bridge as vector,
};
use ffx_battle::{
    cue::bridge as cue, flow::bridge as flow, pos::bridge as pos, turn::bridge as turn,
};
use ffx_battle::{damage::bridge as dmg, state::bridge as state, util::bridge as util};
use ffx_battle::{minor::bridge as minor, misc::bridge as misc, save::bridge as save};

use super::{call0, mem, ported};

/// `TOGetFFXLang` (system), which `DmgCalc_Bribe` asks.
const TO_GET_FFX_LANG: u32 = 0x0064_10e0;
/// `TOBtlCloseSignWindow` (system), which the message cue calls.
const TO_BTL_CLOSE_SIGN_WINDOW: u32 = 0x0089_0ee0;
/// `achievementUnlockAchievement(n)` (main), which `Battle_Steal` calls.
const ACHIEVEMENT_UNLOCK: u32 = 0x0082_2380;

/// Calls an original cdecl function of one argument by its `ffx.sym`
/// address.
fn call1(sym: u32, a: u32) -> u32 {
    // SAFETY: the address is a cdecl function of this build taking one u32.
    unsafe {
        let f: unsafe extern "C" fn(u32) -> u32 = std::mem::transmute(crate::rebase(sym));
        f(a)
    }
}

/// Calls an original cdecl function of four arguments by its `ffx.sym`
/// address.
#[allow(clippy::many_single_char_names)]
fn call4(sym: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
    // SAFETY: the address is a cdecl function of this build taking four u32s.
    unsafe {
        let f: unsafe extern "C" fn(u32, u32, u32, u32) -> u32 =
            std::mem::transmute(crate::rebase(sym));
        f(a, b, c, d)
    }
}

/// Calls the game's cdecl code at a run-time address (`exec::Call`).
fn call_ptr<M>(_: &mut M, f: u32, args: &[u32]) -> u32 {
    super::call_cdecl(f, args)
}

/// `op_3d_vol_pan_get(pos, vol, &out)` with `out` on this thread's stack
/// (`MsSEP3Dprocess`'s frame local): the pan and the volume.
fn sep_vol_pan<M>(_: &mut M, pos: u32, vol: u32) -> (u32, u32) {
    let mut out = 0u32;
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    let (at, f) = (
        std::ptr::addr_of_mut!(out) as u32,
        crate::rebase(afterdmg::OP_3D_VOL_PAN_GET) as u32,
    );
    let pan = super::call_cdecl(f, &[pos, vol, at]);
    // SAFETY: `out` is this frame's local, written through `at` above.
    (pan, unsafe { std::ptr::read_volatile(&raw const out) })
}

/// The calls `MsCheckRikkuLimit` makes: the CD library's read queue gets
/// a block whose `+0xc` is the actor index (the original's other words
/// are stack garbage; zeros here).
fn cd_call<M>(m: &mut M, f: u32, args: &[u32]) -> u32 {
    if let [file, dest, done, i] = *args {
        let block = [0u32, 0, 0, i, 0, 0, 0];
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = block.as_ptr() as u32;
        call_ptr(m, f, &[file, dest, done, at])
    } else {
        call_ptr(m, f, args)
    }
}

/// `AtelRegistSignal2` with the signal block on this stack, as the
/// original builds it.
fn regist_signal<M>(_: &mut M, block: [u32; 8]) -> u32 {
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    call1(0x0086_e9e0, block.as_ptr() as u32)
}

/// The script calls of `ffx_battle::atel` (the script module's code).
macro_rules! script {
    ($s:ident) => {
        let mut call = call_ptr;
        let mut signal = regist_signal;
        let mut off = |m: &mut _, e| ffx_vfx::et::bridge::op_et_eff_off_signal(m, e);
        let mut $s = atel::Script {
            call: &mut call,
            signal: &mut signal,
            off: &mut off,
        };
    };
}

/// MSVCR110's `_CIcos`, `_CIsin` or `_CIatan` through the game's import
/// slot (`0x00b0c3b4`, `0x00b0c3b8`, `0x00b0c450`, behind the thunks
/// `0x009498c2`, `0x009498c8`, `0x00949d5c`): the argument (`f32` bits)
/// loaded with `fld dword`, the result stored with `fstp dword`, as the
/// battle code does.
fn crt_trig(t: pos::Trig, x: u32) -> u32 {
    let slot = match t {
        pos::Trig::Cos => 0x00b0_c3b4,
        pos::Trig::Sin => 0x00b0_c3b8,
        pos::Trig::Atan => 0x00b0_c450,
    };
    // SAFETY: the slot holds the address of the CRT's intrinsic, which
    // takes its argument in ST0 and returns in ST0 (eax, ecx, edx, the x87
    // and SSE registers clobbered).
    #[cfg(target_arch = "x86")]
    unsafe {
        let f = (crate::rebase(slot) as *const usize).read();
        let bits: u32;
        std::arch::asm!(
            "fld dword ptr [{x}]",
            "call {f}",
            "sub esp, 4",
            "fstp dword ptr [esp]",
            "pop eax",
            x = in(reg) &raw const x,
            f = in(reg) f,
            lateout("eax") bits,
            clobber_abi("C"),
        );
        bits
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = (slot, x);
        unreachable!("the CRT's trigonometry runs in the 32-bit game")
    }
}

/// MSVCR110's `_CIcos` or `_CIsin` through the game's import slot on a
/// double (an angle the caller summed at 53 bits): loaded with `fld qword`,
/// the result stored with `fstp dword`, as `MsCameraShakeProcess` does.
fn crt_trig_f64(t: pos::Trig, x: f64) -> u32 {
    let slot = match t {
        pos::Trig::Cos => 0x00b0_c3b4,
        pos::Trig::Sin => 0x00b0_c3b8,
        pos::Trig::Atan => 0x00b0_c450,
    };
    // SAFETY: the slot holds the address of the CRT's intrinsic, which
    // takes its argument in ST0 and returns in ST0 (eax, ecx, edx, the x87
    // and SSE registers clobbered).
    #[cfg(target_arch = "x86")]
    unsafe {
        let f = (crate::rebase(slot) as *const usize).read();
        let bits: u32;
        std::arch::asm!(
            "fld qword ptr [{x}]",
            "call {f}",
            "sub esp, 4",
            "fstp dword ptr [esp]",
            "pop eax",
            x = in(reg) &raw const x,
            f = in(reg) f,
            lateout("eax") bits,
            clobber_abi("C"),
        );
        bits
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = (slot, x);
        0
    }
}

/// MSVCR110's `_CIcos` through the game's import slot (`0x00b0c3b4`,
/// behind the thunk `0x009498c2`) on a double, as `MsCameraMoveCalc`
/// calls it: loaded with `fld qword`, the result stored with `fstp qword`.
fn crt_cos(x: f64) -> f64 {
    #[cfg(target_arch = "x86")]
    {
        let mut out = 0f64;
        // SAFETY: the slot holds the address of the CRT's intrinsic, which
        // takes its argument in ST0 and returns in ST0 (eax, ecx, edx, the
        // x87 and SSE registers clobbered); edi, which holds the result's
        // address, is kept by the callee.
        unsafe {
            let f = (crate::rebase(0x00b0_c3b4) as *const usize).read();
            std::arch::asm!(
                "fld qword ptr [{x}]",
                "call {f}",
                "fstp qword ptr [edi]",
                x = in(reg) &raw const x,
                f = in(reg) f,
                in("edi") &raw mut out,
                clobber_abi("C"),
            );
        }
        out
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = x;
        f64::NAN
    }
}

/// `TkMtCalcSpline` or `TkMtCalcSplineTangent` (`f`) into `out`, which
/// may be a [`target::Dm`] frame local: through four words on this stack,
/// holding `out`'s words first (the spline writes nothing for some point
/// counts, leaving the caller's).
#[allow(clippy::many_single_char_names)] // the spline's arguments
fn spline_via_stack(m: &mut target::Dm<'_>, f: u32, out: u32, points: u32, n: u32, t: u32) {
    use ffx_bridge::Mem;
    let mut v = super::StackFrame::<4>::new();
    let at = v.at();
    let w: [u32; 4] = m.read(out);
    m.write(at, w);
    call4(f, at, points, n, t);
    let w: [u32; 4] = m.read(at);
    m.write(out, w);
}

/// What `ffx_battle::target` calls in other modules: the CRT through
/// [`crt_trig`], script's ported functions, the originals of the rest.
fn host() -> target::Host<'static> {
    fn spline(m: &mut target::Dm<'_>, out: u32, points: u32, n: u32, t: u32) {
        spline_via_stack(m, 0x0088_a4a0, out, points, n, t);
    }
    fn tangent(m: &mut target::Dm<'_>, out: u32, points: u32, n: u32, t: u32) {
        spline_via_stack(m, 0x0088_b3b0, out, points, n, t);
    }
    fn neck_enable(m: &mut target::Dm<'_>, chr: u32, on: u32) {
        ffx_chara::chr::bridge::ch_neck_set_enable(
            m,
            chr,
            on,
            ffx_sg::quat::bridge::matrix_to_quaternion,
        );
    }
    fn neck_target(m: &mut target::Dm<'_>, chr: u32, to: u32) {
        ffx_chara::chr::bridge::ch_neck_set_target(
            m,
            chr,
            to,
            ffx_sg::quat::bridge::matrix_to_quaternion,
        );
    }
    fn total(m: &mut target::Dm<'_>) -> u32 {
        ffx_script::vm::bridge::atel_get_total_actor_number2(m)
    }
    fn top(m: &mut target::Dm<'_>) -> u32 {
        ffx_script::vm::bridge::atel_get_now_ctrl_top_id(m)
    }
    fn is_bind(_: &mut target::Dm<'_>, i: u32) -> u32 {
        call1(0x0086_d950, i)
    }
    fn dir(_: &mut target::Dm<'_>, i: u32) -> u32 {
        // SAFETY: a cdecl function of this build taking a byte index and
        // returning an f32 in ST0.
        unsafe {
            let f: unsafe extern "C" fn(u32) -> f32 =
                std::mem::transmute(crate::rebase(0x0086_a6e0));
            f(i).to_bits()
        }
    }
    fn pos(_: &mut target::Dm<'_>, i: u32, x: u32, y: u32, z: u32) {
        call4(0x0086_b1e0, i, x, y, z);
    }
    target::Host {
        trig: &crt_trig,
        spline: &spline,
        spline_tangent: &tangent,
        cos: &crt_cos,
        neck_enable: &neck_enable,
        neck_target: &neck_target,
        atel_total: &total,
        atel_top: &top,
        atel_is_bind: &is_bind,
        atel_dir: &dir,
        atel_pos: &pos,
    }
}

/// `MsGetSummonerPos` in Rust with the CRT's own cosine and sine.
fn summoner_pos(m: &mut impl ffx_bridge::Mem, actor: u32, index: i32, k: i32, vec: u32) {
    pos::ms_get_summoner_pos(
        m,
        &crt_trig,
        actor,
        index.cast_unsigned(),
        k.cast_unsigned(),
        vec,
    );
}

ported! {
    /// `rnd`: the global random generator.
    rnd @ 0x0079_89a0 => extern "C" fn rnd() -> u32 {
        crate::harness::log_call("rnd", 0);
        ffx_battle::rng::bridge::rnd(&mut mem())
    }
    /// `brnd(index)`: one of the 68 indexed generators.
    brnd @ 0x0079_88f0 => extern "C" fn brnd(index: u32) -> u32 {
        crate::harness::log_call("brnd", index);
        ffx_battle::rng::bridge::brnd(&mut mem(), index)
    }
    /// `MsRandMize(n)`: seeds every generator from `Rng_ClockSeed`.
    MsRandMize @ 0x0079_8890 => extern "C" fn ms_rand_mize(n: u32) {
        let clock = call0(0x0079_8950);
        ffx_battle::rng::bridge::ms_rand_mize(&mut mem(), clock, n);
    }

    // The save block.
    /// `MsGetSaveCommand(chr, cmd)`: command learnt.
    MsGetSaveCommand @ 0x0078_5020 => extern "C" fn ms_get_save_command(chr: i32, cmd: u32) -> u32 {
        save::ms_get_save_command(&mut mem(), chr, cmd)
    }
    /// `MsGetSaveCommandUse(chr, cmd)`: command used.
    MsGetSaveCommandUse @ 0x0078_5140 => extern "C" fn ms_get_save_command_use(chr: i32, cmd: u32) -> u32 {
        save::ms_get_save_command_use(&mut mem(), chr, cmd)
    }
    /// `MsGetSaveEventAddress`: `&g_SaveData`.
    MsGetSaveEventAddress @ 0x0078_5240 => extern "C" fn ms_get_save_event_address() -> u32 {
        save::ms_get_save_event_address(&mem())
    }
    /// `MsGetSaveMenu`: the menu settings word.
    MsGetSaveMenu @ 0x0078_5290 => extern "C" fn ms_get_save_menu() -> u32 {
        save::ms_get_save_menu(&mem())
    }
    /// `MsGetSaveConfigCursorSave`.
    MsGetSaveConfigCursorSave @ 0x0078_51c0 => extern "C" fn ms_get_save_config_cursor_save() -> u32 {
        save::ms_get_save_config_cursor_save(&mem())
    }
    /// `MsGetSaveConfigHiragana`.
    MsGetSaveConfigHiragana @ 0x0078_51f0 => extern "C" fn ms_get_save_config_hiragana() -> u32 {
        save::ms_get_save_config_hiragana(&mem())
    }
    /// `MsGetSavePlayerPtr(chr)`: a `ply_save` record.
    MsGetSavePlayerPtr @ 0x0078_5330 => extern "C" fn ms_get_save_player_ptr(chr: u32) -> u32 {
        save::ms_get_save_player_ptr(&mem(), chr)
    }
    /// `MsGetSavePlyJoin(chr)`.
    MsGetSavePlyJoin @ 0x0078_5380 => extern "C" fn ms_get_save_ply_join(chr: u32) -> u32 {
        save::ms_get_save_ply_join(&mem(), chr)
    }
    /// `MsGetSavePlyJoined(chr)`.
    MsGetSavePlyJoined @ 0x0078_53a0 => extern "C" fn ms_get_save_ply_joined(chr: u32) -> u32 {
        save::ms_get_save_ply_joined(&mem(), chr)
    }
    /// `MsGetSaveSP(chr)`.
    MsGetSaveSP @ 0x0078_53e0 => extern "C" fn ms_get_save_sp(chr: u32) -> u32 {
        save::ms_get_save_sp(&mem(), chr)
    }
    /// `getScenerioFlag`: the story progress (16 bits in AX).
    getScenerioFlag @ 0x0078_7360 => extern "C" fn get_scenerio_flag() -> u16 {
        save::get_scenerio_flag(&mem())
    }
    /// `getInterValefor` (8 bits in AL).
    getInterValefor @ 0x0078_7340 => extern "C" fn get_inter_valefor() -> u8 {
        save::get_inter_valefor(&mem())
    }
    /// `getDefaultChrName(chr)`.
    getDefaultChrName @ 0x0078_7320 => extern "C" fn get_default_chr_name(chr: u32) -> u32 {
        save::get_default_chr_name(&mem(), chr)
    }
    /// `MsGetSaveItemNum(item)`.
    MsGetSaveItemNum @ 0x0079_04b0 => extern "C" fn ms_get_save_item_num(item: u32) -> u32 {
        save::ms_get_save_item_num(&mut mem(), item)
    }
    /// `MsSetUseItem(item)`.
    MsSetUseItem @ 0x0079_0790 => extern "C" fn ms_set_use_item(item: u32) {
        save::ms_set_use_item(&mut mem(), item);
    }
    /// `MsSaveItemTypeBuffer(&count)`.
    MsSaveItemTypeBuffer @ 0x0079_0530 => extern "C" fn ms_save_item_type_buffer(count: u32) -> u32 {
        save::ms_save_item_type_buffer(&mut mem(), count)
    }
    /// `MsSaveItemNumBuffer(&count)`.
    MsSaveItemNumBuffer @ 0x0079_0510 => extern "C" fn ms_save_item_num_buffer(count: u32) -> u32 {
        save::ms_save_item_num_buffer(&mut mem(), count)
    }
    /// `MsGetSaveInParty(&count)`.
    MsGetSaveInParty @ 0x0078_5270 => extern "C" fn ms_get_save_in_party(count: u32) -> u32 {
        save::ms_get_save_in_party(&mut mem(), count)
    }
    /// `MsGetSaveOutParty(&count)`.
    MsGetSaveOutParty @ 0x0078_52d0 => extern "C" fn ms_get_save_out_party(count: u32) -> u32 {
        save::ms_get_save_out_party(&mut mem(), count)
    }
    /// `MsGetSavePartyMember(&a, &b, &c)`.
    MsGetSavePartyMember @ 0x0078_52f0 => extern "C" fn ms_get_save_party_member(a: u32, b: u32, c: u32) {
        save::ms_get_save_party_member(&mut mem(), a, b, c);
    }
    /// `MsSetSaveParty`.
    MsSetSaveParty @ 0x0078_6930 => extern "C" fn ms_set_save_party() {
        save::ms_set_save_party(&mut mem());
    }
    /// `MsGetGIL`.
    MsGetGIL @ 0x0078_4e80 => extern "C" fn ms_get_gil() -> u32 {
        save::ms_get_gil(&mem())
    }
    /// `MsDebugFullGill`.
    MsDebugFullGill @ 0x0078_49c0 => extern "C" fn ms_debug_full_gill() {
        save::ms_debug_full_gill(&mut mem());
    }
    /// `MsDebugAddGill(n)`.
    MsDebugAddGill @ 0x0078_4830 => extern "C" fn ms_debug_add_gill(n: i32) {
        save::ms_debug_add_gill(&mut mem(), n);
    }
    /// `MsGetSaveBattleCount`.
    MsGetSaveBattleCount @ 0x0078_4f50 => extern "C" fn ms_get_save_battle_count() -> u32 {
        save::ms_get_save_battle_count(&mem())
    }
    /// `MsGetMonsterCapture(id)`.
    MsGetMonsterCapture @ 0x0079_0af0 => extern "C" fn ms_get_monster_capture(id: u32) -> u32 {
        save::ms_get_monster_capture(&mem(), id)
    }
    /// `AddMonsterCapture(id, n)`.
    AddMonsterCapture @ 0x0079_0b90 => extern "C" fn add_monster_capture(id: u32, n: i32) -> i32 {
        save::add_monster_capture(&mut mem(), id, n)
    }
    /// `MsImportantGet(id)`: key item held.
    MsImportantGet @ 0x0079_0800 => extern "C" fn ms_important_get(id: u32) -> u32 {
        save::ms_important_get(&mem(), id)
    }
    /// `MsImportantSet(id)`.
    MsImportantSet @ 0x0079_0930 => extern "C" fn ms_important_set(id: u32) {
        save::ms_important_set(&mut mem(), id);
    }
    /// `MsImportantPtr(id, strings)`: the key item's kernel record.
    MsImportantPtr @ 0x0079_08a0 => extern "C" fn ms_important_ptr(id: u32, strings: u32) -> u32 {
        save::ms_important_ptr(&mut mem(), id, strings)
    }
    /// `MsSetItemCheck(id, table)`.
    MsSetItemCheck @ 0x0079_0670 => extern "C" fn ms_set_item_check(id: u32, table: u32) {
        save::ms_set_item_check(&mut mem(), id, table);
    }
    /// `MsSetMonsterCheck(id, table)`.
    MsSetMonsterCheck @ 0x0079_0be0 => extern "C" fn ms_set_monster_check(id: u32, table: u32) {
        save::ms_set_monster_check(&mut mem(), id, table);
    }
    /// `MsGetSaveAbilityMap`.
    MsGetSaveAbilityMap @ 0x0078_4f40 => extern "C" fn ms_get_save_ability_map() -> u32 {
        save::ms_get_save_ability_map(&mem())
    }
    /// `MsGetSaveWalkMap`.
    MsGetSaveWalkMap @ 0x0078_5400 => extern "C" fn ms_get_save_walk_map() -> u32 {
        save::ms_get_save_walk_map(&mem())
    }
    /// `MsGetSaveEventFlag`.
    MsGetSaveEventFlag @ 0x0078_5250 => extern "C" fn ms_get_save_event_flag() -> u32 {
        save::ms_get_save_event_flag(&mem())
    }
    /// `MsGetItemCheck`.
    MsGetItemCheck @ 0x0079_04a0 => extern "C" fn ms_get_item_check() -> u32 {
        save::ms_get_item_check(&mem())
    }
    /// `MsGetSaveItemUse`.
    MsGetSaveItemUse @ 0x0079_0500 => extern "C" fn ms_get_save_item_use() -> u32 {
        save::ms_get_save_item_use(&mem())
    }
    /// `MsGetSaveMonsterMeet`.
    MsGetSaveMonsterMeet @ 0x0079_0b20 => extern "C" fn ms_get_save_monster_meet() -> u32 {
        save::ms_get_save_monster_meet(&mem())
    }
    /// `MsGetSaveMonsterDefeat`.
    MsGetSaveMonsterDefeat @ 0x0079_0b10 => extern "C" fn ms_get_save_monster_defeat() -> u32 {
        save::ms_get_save_monster_defeat(&mem())
    }
    /// `MsGetSaveWeaponList(character, kind, unequipped, &count)`.
    MsGetSaveWeaponList @ 0x007a_bd10 => extern "C" fn ms_get_save_weapon_list(character: i32, kind: i32, unequipped: i32, count: u32) -> u32 {
        save::ms_get_save_weapon_list(&mut mem(), character, kind, unequipped, count)
    }
    /// `MsGetSaveWeaponAmount(&free)`.
    MsGetSaveWeaponAmount @ 0x007a_bc60 => extern "C" fn ms_get_save_weapon_amount(free: u32) -> u32 {
        save::ms_get_save_weapon_amount(&mut mem(), free)
    }
    /// `MsCalcWeaponCap(equipment)`.
    MsCalcWeaponCap @ 0x0079_8eb0 => extern "C" fn ms_calc_weapon_cap(equipment: u32) {
        save::ms_calc_weapon_cap(&mut mem(), equipment);
    }
    /// `MsGetSaveChrMan(chr)`.
    MsGetSaveChrMan @ 0x0078_4f90 => extern "C" fn ms_get_save_chr_man(chr: u32) -> u32 {
        save::ms_get_save_chr_man(&mut mem(), chr)
    }
    /// `MsGetSaveChrFemale(chr)`.
    MsGetSaveChrFemale @ 0x0078_4f60 => extern "C" fn ms_get_save_chr_female(chr: u32) -> u32 {
        save::ms_get_save_chr_female(&mut mem(), chr)
    }
    /// `MsGetSaveChrSummon(chr)`.
    MsGetSaveChrSummon @ 0x0078_4ff0 => extern "C" fn ms_get_save_chr_summon(chr: u32) -> u32 {
        save::ms_get_save_chr_summon(&mut mem(), chr)
    }
    /// `MsAddLoveParam(chr, n)`.
    MsAddLoveParam @ 0x0078_45e0 => extern "C" fn ms_add_love_param(chr: i32, n: i32) -> u32 {
        save::ms_add_love_param(&mut mem(), chr, n)
    }
    /// `MsGetYoujinbouCom`: Yojimbo's command.
    MsGetYoujinbouCom @ 0x007b_2b20 => extern "C" fn ms_get_youjinbou_com() -> u32 {
        save::ms_get_youjinbou_com(&mem())
    }

    // Actors.
    /// `MsGetChr(i)`: the actor of index `i`.
    MsGetChr @ 0x0079_4020 => extern "C" fn ms_get_chr(i: u32) -> u32 {
        actor::ms_get_chr(&mem(), i)
    }
    /// `MsGetChrTop`.
    MsGetChrTop @ 0x0079_5970 => extern "C" fn ms_get_chr_top() -> u32 {
        actor::ms_get_chr_top(&mem())
    }
    /// `MsGetMon(i)`.
    MsGetMon @ 0x0079_5aa0 => extern "C" fn ms_get_mon(i: u32) -> u32 {
        actor::ms_get_mon(&mem(), i)
    }
    /// `MsGetWeapon(i, second)`.
    MsGetWeapon @ 0x0079_5b10 => extern "C" fn ms_get_weapon(i: u32, second: i32) -> u32 {
        actor::ms_get_weapon(&mem(), i, second)
    }
    /// `MsBtlChrNumCheck(i)`.
    MsBtlChrNumCheck @ 0x0079_3650 => extern "C" fn ms_btl_chr_num_check(i: u32) -> u32 {
        actor::ms_btl_chr_num_check(i)
    }
    /// `MsGetRamChrMonster(i)`.
    MsGetRamChrMonster @ 0x0079_aef0 => extern "C" fn ms_get_ram_chr_monster(i: u32) -> u32 {
        actor::ms_get_ram_chr_monster(i)
    }
    /// `MsCalcWeakLevel(hp, max)`.
    MsCalcWeakLevel @ 0x0078_bf00 => extern "C" fn ms_calc_weak_level(hp: i32, max: i32) -> u32 {
        actor::ms_calc_weak_level(hp, max)
    }
    /// `MsAutoCommandStatusCheck(actor, need_allowed, no_400, no_800)`.
    MsAutoCommandStatusCheck @ 0x007b_24a0 => extern "C" fn ms_auto_command_status_check(p: u32, need_allowed: i32, no_400: i32, no_800: i32) -> u32 {
        actor::ms_auto_command_status_check(&mem(), p, need_allowed, no_400, no_800)
    }
    /// `MsCheckSetMotion(actor, mode)`.
    MsCheckSetMotion @ 0x007a_f320 => extern "C" fn ms_check_set_motion(p: u32, mode: i32) -> u32 {
        actor::ms_check_set_motion(&mem(), p, mode)
    }
    /// `MsPushBtlPos(actor)`.
    MsPushBtlPos @ 0x007a_c650 => extern "C" fn ms_push_btl_pos(p: u32) {
        actor::ms_push_btl_pos(&mut mem(), p);
    }
    /// `MsPopBtlPos(actor)`.
    MsPopBtlPos @ 0x007a_c600 => extern "C" fn ms_pop_btl_pos(p: u32) {
        actor::ms_pop_btl_pos(&mut mem(), p);
    }
    /// `MsSetAutoStatus(actor)`.
    MsSetAutoStatus @ 0x0079_b290 => extern "C" fn ms_set_auto_status(p: u32) {
        actor::ms_set_auto_status(&mut mem(), p);
    }
    /// `MsResetAutoStatus(actor)`.
    MsResetAutoStatus @ 0x0079_b1a0 => extern "C" fn ms_reset_auto_status(p: u32) {
        actor::ms_reset_auto_status(&mut mem(), p);
    }
    /// `MsSetEternalRelife(actor)`.
    MsSetEternalRelife @ 0x0078_deb0 => extern "C" fn ms_set_eternal_relife(p: u32) {
        actor::ms_set_eternal_relife(&mut mem(), p);
    }
    /// `MsMotionSpeedReset(actor)`.
    MsMotionSpeedReset @ 0x007a_b730 => extern "C" fn ms_motion_speed_reset(p: u32) {
        actor::ms_motion_speed_reset(&mut mem(), p);
    }
    /// `MsTerminateMotion(actor)`.
    MsTerminateMotion @ 0x007a_b6c0 => extern "C" fn ms_terminate_motion(p: u32) {
        actor::ms_terminate_motion(&mut mem(), p);
    }
    /// `msSetEndDir(actor)`.
    msSetEndDir @ 0x0079_3260 => extern "C" fn ms_set_end_dir(p: u32) {
        actor::ms_set_end_dir(&mut mem(), p);
    }
    /// `msCheckSameTarget(chr, actor, targets)`.
    msCheckSameTarget @ 0x0079_6d70 => extern "C" fn ms_check_same_target(chr: u32, p: u32, targets: u32) -> i32 {
        actor::ms_check_same_target(&mem(), chr, p, targets)
    }
    /// `MsBtlChrInfoFree(actor)`.
    MsBtlChrInfoFree @ 0x0079_3570 => extern "C" fn ms_btl_chr_info_free(p: u32) {
        actor::ms_btl_chr_info_free(&mut mem(), p);
    }

    // Battle globals.
    /// `MsBattleGetStatus`.
    MsBattleGetStatus @ 0x0078_16f0 => extern "C" fn ms_battle_get_status() -> i32 {
        state::ms_battle_get_status(&mem())
    }
    /// `MsGetBattleScene`: field and group of the battle.
    MsGetBattleScene @ 0x0078_2970 => extern "C" fn ms_get_battle_scene() -> u32 {
        state::ms_get_battle_scene(&mem())
    }
    /// `MsBattleExeClear`.
    MsBattleExeClear @ 0x0078_1530 => extern "C" fn ms_battle_exe_clear() -> i32 {
        state::ms_battle_exe_clear(&mut mem())
    }
    /// `MsEventEffectGetStat`.
    MsEventEffectGetStat @ 0x0078_8bd0 => extern "C" fn ms_event_effect_get_stat() -> i32 {
        state::ms_event_effect_get_stat(&mem())
    }
    /// `MsBtlReadCueLevel`.
    MsBtlReadCueLevel @ 0x007a_d180 => extern "C" fn ms_btl_read_cue_level() -> u32 {
        state::ms_btl_read_cue_level(&mem())
    }
    /// `MsGetVoiceEventFlag`.
    MsGetVoiceEventFlag @ 0x0079_86a0 => extern "C" fn ms_get_voice_event_flag() -> u32 {
        state::ms_get_voice_event_flag(&mem())
    }
    /// `MsMemCheckInit`.
    MsMemCheckInit @ 0x0079_9ee0 => extern "C" fn ms_mem_check_init() {
        state::ms_mem_check_init(&mut mem());
    }
    /// `MsMemCheckRegist(name, size)`.
    MsMemCheckRegist @ 0x0079_9f00 => extern "C" fn ms_mem_check_regist(_name: u32, size: i32) {
        state::ms_mem_check_regist(&mut mem(), size);
    }
    /// `MsAtelRequestInit`.
    MsAtelRequestInit @ 0x0079_7ff0 => extern "C" fn ms_atel_request_init() {
        state::ms_atel_request_init(&mut mem());
    }
    /// `MsAtelRequestAdd(a, b, c, d)`: the low halves of the four.
    MsAtelRequestAdd @ 0x0079_7b70 => extern "C" fn ms_atel_request_add(a: u32, b: u32, c: u32, d: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores 16 bits
        state::ms_atel_request_add(&mut mem(), [a as u16, b as u16, c as u16, d as u16]);
    }
    /// `MsBattleWater`: under water.
    MsBattleWater @ 0x0078_21e0 => extern "C" fn ms_battle_water() -> i32 {
        state::ms_battle_water(&mem())
    }
    /// `MsBattleGetMapCircleOn`.
    MsBattleGetMapCircleOn @ 0x0078_16a0 => extern "C" fn ms_battle_get_map_circle_on() -> i32 {
        state::ms_battle_get_map_circle_on(&mem())
    }
    /// `MsGetEncountType`.
    MsGetEncountType @ 0x0078_2a10 => extern "C" fn ms_get_encount_type() -> u32 {
        state::ms_get_encount_type(&mem())
    }
    /// `MsStoneEffectCheck`.
    MsStoneEffectCheck @ 0x007a_2670 => extern "C" fn ms_stone_effect_check() -> i32 {
        state::ms_stone_effect_check(&mem())
    }
    /// `MsBattleCursorType`.
    MsBattleCursorType @ 0x0078_0ce0 => extern "C" fn ms_battle_cursor_type() -> i32 {
        state::ms_battle_cursor_type(&mem())
    }
    /// `MsBattleGetChrDisposeFlag`: always 1.
    MsBattleGetChrDisposeFlag @ 0x0078_1630 => extern "C" fn ms_battle_get_chr_dispose_flag() -> u32 {
        state::ms_battle_get_chr_dispose_flag()
    }
    /// `MsGetInputCue`.
    MsGetInputCue @ 0x007b_22e0 => extern "C" fn ms_get_input_cue() -> u32 {
        state::ms_get_input_cue(&mem())
    }
    /// `MsGetCommandCue`.
    MsGetCommandCue @ 0x007b_0a10 => extern "C" fn ms_get_command_cue() -> u32 {
        state::ms_get_command_cue(&mem())
    }
    /// `MsCheckCommandCue(actor, command)`.
    MsCheckCommandCue @ 0x007b_0720 => extern "C" fn ms_check_command_cue(actor: u32, command: u32) -> u32 {
        state::ms_check_command_cue(&mem(), actor, command)
    }
    /// `MsCheckCommandCue2(actor, kind)`.
    MsCheckCommandCue2 @ 0x007b_0770 => extern "C" fn ms_check_command_cue2(actor: u32, kind: u32) -> u32 {
        state::ms_check_command_cue2(&mem(), actor, kind)
    }
    /// `MsOrderInit`.
    MsOrderInit @ 0x0078_d4b0 => extern "C" fn ms_order_init() {
        state::ms_order_init(&mut mem());
    }
    /// `MsOrderRegist(chr)`: the low byte.
    MsOrderRegist @ 0x0078_d4c0 => extern "C" fn ms_order_regist(chr: u32) {
        state::ms_order_regist(&mut mem(), chr.to_le_bytes()[0]);
    }
    /// `MsMuteBattleSubTitle(on)`.
    MsMuteBattleSubTitle @ 0x0078_5990 => extern "C" fn ms_mute_battle_sub_title(on: u32) {
        state::ms_mute_battle_sub_title(&mut mem(), on);
    }
    /// `MsEventEffectInit`.
    MsEventEffectInit @ 0x0078_8be0 => extern "C" fn ms_event_effect_init() {
        state::ms_event_effect_init(&mut mem());
    }
    /// `MsSEPtransAmount`.
    MsSEPtransAmount @ 0x007a_04b0 => extern "C" fn ms_sep_trans_amount() -> u32 {
        state::ms_sep_trans_amount(&mem())
    }
    /// `MsSEPtransCheck`.
    MsSEPtransCheck @ 0x007a_04c0 => extern "C" fn ms_sep_trans_check() -> i32 {
        state::ms_sep_trans_check(&mem())
    }
    /// `MsDebugGetFieldNum`.
    MsDebugGetFieldNum @ 0x007c_5f50 => extern "C" fn ms_debug_get_field_num() -> u32 {
        state::ms_debug_get_field_num(&mem())
    }
    /// `MsCheckStat(i)`.
    MsCheckStat @ 0x0079_e310 => extern "C" fn ms_check_stat(i: i32) -> u32 {
        state::ms_check_stat(&mem(), i)
    }
    /// `MsCheckStat2(i)`.
    MsCheckStat2 @ 0x0079_e320 => extern "C" fn ms_check_stat2(i: i32) -> u32 {
        state::ms_check_stat2(&mem(), i)
    }

    // Helpers.
    /// `MsStructClear(p, n)`.
    MsStructClear @ 0x0079_f230 => extern "C" fn ms_struct_clear(p: u32, n: i32) {
        util::ms_struct_clear(&mut mem(), p, n);
    }
    /// `MsStructFill(p, n, v)`.
    MsStructFill @ 0x0079_f260 => extern "C" fn ms_struct_fill(p: u32, n: i32, v: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original fills 16 bits
        util::ms_struct_fill(&mut mem(), p, n, v as u16);
    }
    /// `MsStrCopy(src, dst, max)`.
    MsStrCopy @ 0x007c_f6c0 => extern "C" fn ms_str_copy(src: u32, dst: u32, max: i32) -> i32 {
        util::ms_str_copy(&mut mem(), src, dst, max)
    }
    /// `MsGetEff2Chr(x)`.
    MsGetEff2Chr @ 0x0078_8de0 => extern "C" fn ms_get_eff2_chr(x: u32) -> u32 {
        util::ms_get_eff2_chr(x)
    }
    /// `MsCheckMegas(n)`.
    MsCheckMegas @ 0x007a_da40 => extern "C" fn ms_check_megas(n: i32) -> i32 {
        util::ms_check_megas(n)
    }
    /// `MsSetHashValue(list)`.
    MsSetHashValue @ 0x007c_f640 => extern "C" fn ms_set_hash_value(list: u32) {
        util::ms_set_hash_value(&mut mem(), list);
    }
    /// `MsGetEffID(group, a, b)`.
    MsGetEffID @ 0x0078_8df0 => extern "C" fn ms_get_eff_id(group: u32, a: u32, b: u32) -> u32 {
        util::ms_get_eff_id(group, a, b)
    }
    /// `MsAutoRepeatInit(p, first, next)`.
    MsAutoRepeatInit @ 0x007c_89a0 => extern "C" fn ms_auto_repeat_init(p: u32, first: i32, next: i32) {
        util::ms_auto_repeat_init(&mut mem(), p, first, next);
    }
    /// `MsAutoRepeatCheck(p, input)`.
    MsAutoRepeatCheck @ 0x007c_8950 => extern "C" fn ms_auto_repeat_check(p: u32, input: i32) -> i32 {
        util::ms_auto_repeat_check(&mut mem(), p, input)
    }

    // The battle camera.
    /// `MsCameraGetNum(cam)`.
    MsCameraGetNum @ 0x007b_d340 => extern "C" fn ms_camera_get_num(cam: u32) -> u32 {
        camera::ms_camera_get_num(&mem(), cam)
    }
    /// `MsCameraIDcheck(id)`.
    MsCameraIDcheck @ 0x007b_d560 => extern "C" fn ms_camera_id_check(id: u32) -> i32 {
        camera::ms_camera_id_check(&mem(), id)
    }
    /// `MsCameraGetPosNow(side, out)`.
    MsCameraGetPosNow @ 0x007b_d470 => extern "C" fn ms_camera_get_pos_now(side: u32, out: u32) {
        camera::ms_camera_get_pos_now(&mut mem(), side, out);
    }
    /// `MsCameraGetScrDpt`: a float, on the x87 stack.
    MsCameraGetScrDpt @ 0x007b_d510 => extern "C" fn ms_camera_get_scr_dpt() -> f32 {
        camera::ms_camera_get_scr_dpt(&mem())
    }
    /// `MsCameraPriority(id, bonus)`: the low byte of `bonus`.
    MsCameraPriority @ 0x007b_e0a0 => extern "C" fn ms_camera_priority(id: u32, bonus: u32) {
        camera::ms_camera_priority(&mut mem(), id, bonus.to_le_bytes()[0]);
    }
    /// `MsCameraPriorityMask(on)`.
    MsCameraPriorityMask @ 0x007b_e100 => extern "C" fn ms_camera_priority_mask(on: u32) {
        camera::ms_camera_priority_mask(&mut mem(), on);
    }
    /// `MsScreenResetShake(screen)`.
    MsScreenResetShake @ 0x007c_56c0 => extern "C" fn ms_screen_reset_shake(screen: u32) {
        camera::ms_screen_reset_shake(&mut mem(), screen);
    }
    /// `MsGetScreenMatrix`.
    MsGetScreenMatrix @ 0x007b_bd70 => extern "C" fn ms_get_screen_matrix() -> u32 {
        camera::ms_get_screen_matrix(&mem())
    }
    /// `MsGetCameraMatrix`.
    MsGetCameraMatrix @ 0x007c_04b0 => extern "C" fn ms_get_camera_matrix() -> u32 {
        camera::ms_get_camera_matrix(&mem())
    }

    // Damage, hits and statuses. DmgCalc and DmgCalc_InflictStatus_Extra
    // keep values in ECX or EDX across some of their callees
    // (DmgCalc_ApplyZombieDrainInteraction, DmgCalc_Alchemy,
    // DmgCalc_ShatterFail, DmgCalc_NoDamageDeath, DmgCalc_HitKind,
    // DmgCalc_IsDistill: their compiler knew those leave them alone); with
    // both callers replaced too, nothing relies on that any more.
    /// `MsGetRamChrSummon(chr)`.
    MsGetRamChrSummon @ 0x0079_af60 => extern "C" fn ms_get_ram_chr_summon(chr: u32) -> u32 {
        dmg::ms_get_ram_chr_summon(&mem(), chr)
    }
    /// `MsGetRndChr(chr, kind)`: the generator for an actor's rolls.
    MsGetRndChr @ 0x0078_d210 => extern "C" fn ms_get_rnd_chr(chr: u32, kind: i32) -> u32 {
        dmg::ms_get_rnd_chr(&mem(), chr, kind)
    }
    /// `MsCalcDamageCommand(user, target, command, formula, power, status, stat, vary, &def, &mdef, fallback)`.
    MsCalcDamageCommand @ 0x0078_9bf0 => extern "C" fn ms_calc_damage_command(user: u32, target: u32, command: u32, formula: i32, power: i32, status: u32, stat: i32, vary: i32, def: u32, mdef: u32, fallback: i32) -> i32 {
        dmg::ms_calc_damage_command(&mut mem(), user, target, command, formula, power, status, stat, vary, def, mdef, fallback)
    }
    /// `MsCalcEscape(chr)`.
    MsCalcEscape @ 0x0078_a780 => extern "C" fn ms_calc_escape(chr: u32) -> u32 {
        dmg::ms_calc_escape(&mut mem(), chr)
    }
    /// `DmgCalc_StatBoostAbility(user_id, target_id, command, damage)`.
    DmgCalc_StatBoostAbility @ 0x0078_91e0 => extern "C" fn dmg_calc_stat_boost_ability(user_id: u32, target_id: u32, command: u32, damage: i32) -> i32 {
        dmg::dmg_calc_stat_boost_ability(&mem(), user_id, target_id, command, damage)
    }
    /// `DmgCalc_PowerMagicBreak(user, command, damage)`.
    DmgCalc_PowerMagicBreak @ 0x0078_9290 => extern "C" fn dmg_calc_power_magic_break(user: u32, command: u32, damage: i32) -> i32 {
        dmg::dmg_calc_power_magic_break(&mem(), user, command, damage)
    }
    /// `DmgCalc_Crit(user, target, command, &flags, damage)`.
    DmgCalc_Crit @ 0x0078_9690 => extern "C" fn dmg_calc_crit(user: u32, target: u32, command: u32, flags: u32, damage: i32) -> i32 {
        dmg::dmg_calc_crit(&mut mem(), user, target, command, flags, damage)
    }
    /// `DmgCalc_Elem(target, command, elements, damage)`.
    DmgCalc_Elem @ 0x0078_a360 => extern "C" fn dmg_calc_elem(target: u32, _command: u32, elements: u32, damage: i32) -> i32 {
        dmg::dmg_calc_elem(&mem(), target, elements, damage)
    }
    /// `DmgCalc_Defend(target, command, &flags, info, damage)`.
    DmgCalc_Defend @ 0x0078_a7d0 => extern "C" fn dmg_calc_defend(target: u32, command: u32, flags: u32, info: u32, damage: i32) -> i32 {
        dmg::dmg_calc_defend(&mut mem(), target, command, flags, info, damage)
    }
    /// `DmgCalc_Armored(user, target, command, info, &armored, damage)`.
    DmgCalc_Armored @ 0x0078_a830 => extern "C" fn dmg_calc_armored(user: u32, target: u32, command: u32, info: u32, armored: u32, damage: i32) -> i32 {
        dmg::dmg_calc_armored(&mut mem(), user, target, command, info, armored, damage)
    }
    /// `DmgCalc_CheckHit(user, target, command, info, counter)`: 0 hit, 1 miss, 2 none.
    DmgCalc_CheckHit @ 0x0078_a890 => extern "C" fn dmg_calc_check_hit(user: u32, target: u32, command: u32, info: u32, counter: i32) -> u32 {
        dmg::dmg_calc_check_hit(&mut mem(), user, target, command, info, counter)
    }
    /// `DmgCalc_DmgImmunity(target, command, &what, mask, &nulled, damage)`.
    DmgCalc_DmgImmunity @ 0x0078_aac0 => extern "C" fn dmg_calc_dmg_immunity(target: u32, command: u32, what: u32, mask: u32, nulled: u32, damage: i32) -> i32 {
        dmg::dmg_calc_dmg_immunity(&mut mem(), target, command, what, mask, nulled, damage)
    }
    /// `DmgCalc_Protect(command, &flags, &hit, info, damage)`.
    DmgCalc_Protect @ 0x0078_ad40 => extern "C" fn dmg_calc_protect(command: u32, flags: u32, hit: u32, info: u32, damage: i32) -> i32 {
        dmg::dmg_calc_protect(&mut mem(), command, flags, hit, info, damage)
    }
    /// `DmgCalc_GravImmunity(target, formula, &what, mask, &nulled, damage)`.
    DmgCalc_GravImmunity @ 0x0078_ad80 => extern "C" fn dmg_calc_grav_immunity(target: u32, formula: i32, what: u32, mask: u32, nulled: u32, damage: i32) -> i32 {
        dmg::dmg_calc_grav_immunity(&mut mem(), target, formula, what, mask, nulled, damage)
    }
    /// `DmgCalc_Shell(command, &flags, &hit, info, damage)`.
    DmgCalc_Shell @ 0x0078_adc0 => extern "C" fn dmg_calc_shell(command: u32, flags: u32, hit: u32, info: u32, damage: i32) -> i32 {
        dmg::dmg_calc_shell(&mut mem(), command, flags, hit, info, damage)
    }
    /// `DmgCalc_Threaten(target, info, &what, mask, flags, counts, damage)`.
    DmgCalc_Threaten @ 0x0078_bd50 => extern "C" fn dmg_calc_threaten(_target: u32, info: u32, what: u32, mask: u32, flags: u32, counts: u32, damage: u32) -> u32 {
        dmg::dmg_calc_threaten(&mut mem(), info, what, mask, flags, counts, damage)
    }
    /// `DmgCalc_ReelBonus(user, damage)`: Wakka's reels.
    DmgCalc_ReelBonus @ 0x0078_bd90 => extern "C" fn dmg_calc_reel_bonus(user: u32, damage: i32) -> u32 {
        dmg::dmg_calc_reel_bonus(&mem(), user, damage)
    }
    /// `DmgCalc_MarkDamaged(user_id, user, command, damage, always)`.
    DmgCalc_MarkDamaged @ 0x0078_bf50 => extern "C" fn dmg_calc_mark_damaged(_user_id: u32, user: u32, command: u32, damage: u32, always: i32) -> u32 {
        dmg::dmg_calc_mark_damaged(&mut mem(), user, command, damage, always)
    }
    /// `DmgCalc_CheckNuls(command, elements, info)`.
    DmgCalc_CheckNuls @ 0x0078_bfb0 => extern "C" fn dmg_calc_check_nuls(_command: u32, elements: u32, info: u32) -> i32 {
        dmg::dmg_calc_check_nuls(&mut mem(), elements, info)
    }
    /// `DmgCalc_Berserk(user, damage)`.
    DmgCalc_Berserk @ 0x0078_c0b0 => extern "C" fn dmg_calc_berserk(user: u32, damage: i32) -> i32 {
        dmg::dmg_calc_berserk(&mem(), user, damage)
    }
    /// `DmgCalc_QueueRemoveStatusOnHit(target_id, target, command, removed, counts, info)`.
    DmgCalc_QueueRemoveStatusOnHit @ 0x0078_c0f0 => extern "C" fn dmg_calc_queue_remove_status_on_hit(_target_id: u32, target: u32, command: u32, _removed: u32, counts: u32, info: u32) {
        dmg::dmg_calc_queue_remove_status_on_hit(&mut mem(), target, command, counts, info);
    }
    /// `MsAdjustCTB(chr, ctb, haste, slow)`.
    MsAdjustCTB @ 0x0078_c150 => extern "C" fn ms_adjust_ctb(_chr: u32, ctb: i32, haste: i32, slow: i32) -> i32 {
        dmg::ms_adjust_ctb(ctb, haste, slow)
    }
    /// `DmgCalc_DelayImmunity(target, damage, counts, _, flags, delayed)`.
    DmgCalc_DelayImmunity @ 0x0078_c180 => extern "C" fn dmg_calc_delay_immunity(target: u32, damage: u32, counts: u32, _unused: u32, flags: u32, delayed: i32) -> u32 {
        dmg::dmg_calc_delay_immunity(&mut mem(), target, damage, counts, flags, delayed)
    }
    /// `DmgCalc_CheckCounter(apply, user_id, user, target_id, target, command)`.
    DmgCalc_CheckCounter @ 0x0078_c1d0 => extern "C" fn dmg_calc_check_counter(apply: i32, user_id: i32, user: u32, target_id: i32, target: u32, command: u32) -> i32 {
        dmg::dmg_calc_check_counter(&mut mem(), apply, user_id, user, target_id, target, command)
    }
    /// `DmgCalc_ShieldBoost(target, &flags, damage)`.
    DmgCalc_ShieldBoost @ 0x0078_c530 => extern "C" fn dmg_calc_shield_boost(target: u32, flags: u32, damage: i32) -> i32 {
        dmg::dmg_calc_shield_boost(&mut mem(), target, flags, damage)
    }
    /// `DmgCalc_MagicBooster(user, command, command_id, &damage)`.
    DmgCalc_MagicBooster @ 0x0078_c580 => extern "C" fn dmg_calc_magic_booster(user: u32, command: u32, command_id: i32, damage: u32) -> i32 {
        dmg::dmg_calc_magic_booster(&mut mem(), user, command, command_id, damage)
    }
    /// `MsGetNextCTB(chr, rank, haste, slow)`.
    MsGetNextCTB @ 0x0078_d1d0 => extern "C" fn ms_get_next_ctb(chr: u32, rank: i32, haste: i32, slow: i32) -> i32 {
        dmg::ms_get_next_ctb(&mut mem(), chr, rank, haste, slow)
    }
    /// `DebugCanTakeDamage(chr)`.
    DebugCanTakeDamage @ 0x0078_d3a0 => extern "C" fn debug_can_take_damage(chr: u32) -> u32 {
        dmg::debug_can_take_damage(&mem(), chr)
    }
    /// `DmgCalc_ApplyStatUpBuffs(user, target, command)`.
    DmgCalc_ApplyStatUpBuffs @ 0x0078_d730 => extern "C" fn dmg_calc_apply_stat_up_buffs(_user: u32, target: u32, command: u32) {
        dmg::dmg_calc_apply_stat_up_buffs(&mut mem(), target, command);
    }
    /// `SetUpDamageInfo(info, parent, target)`.
    SetUpDamageInfo @ 0x0078_dae0 => extern "C" fn set_up_damage_info(info: u32, parent: u32, target: u32) {
        dmg::set_up_damage_info(&mut mem(), info, parent, target);
    }
    /// `DmgCalc_Delay(target, command, flags, damage)`.
    DmgCalc_Delay @ 0x0078_e0f0 => extern "C" fn dmg_calc_delay(target: u32, command: u32, flags: u32, damage: u32) -> u32 {
        dmg::dmg_calc_delay(&mut mem(), target, command, flags, damage)
    }
    /// `MsCheckStatCount(i)`.
    MsCheckStatCount @ 0x0079_e330 => extern "C" fn ms_check_stat_count(i: i32) -> u32 {
        dmg::ms_check_stat_count(&mem(), i)
    }
    /// `MsCheckWrongStatus(command)`.
    MsCheckWrongStatus @ 0x0079_e340 => extern "C" fn ms_check_wrong_status(command: u32) -> i32 {
        dmg::ms_check_wrong_status(&mem(), command)
    }

    /// `DmgCalc_ApplyZombieDrainInteraction(user, command, info, damage)`.
    DmgCalc_ApplyZombieDrainInteraction @ 0x0078_a200 => extern "C" fn dmg_calc_apply_zombie_drain_interaction(user: u32, command: u32, info: u32, damage: i32) -> i32 {
        dmg::dmg_calc_apply_zombie_drain_interaction(&mem(), user, command, info, damage)
    }
    /// `DmgCalc_ShatterFail(target, info, status, &what, counts, damage)`.
    DmgCalc_ShatterFail @ 0x0078_ba30 => extern "C" fn dmg_calc_shatter_fail(_target: u32, info: u32, status: u32, what: u32, counts: u32, damage: u32) {
        dmg::dmg_calc_shatter_fail(&mut mem(), info, status, what, counts, damage);
    }
    /// `DmgCalc_NoDamageDeath(target, info, status, flags, damage)`.
    DmgCalc_NoDamageDeath @ 0x0078_c480 => extern "C" fn dmg_calc_no_damage_death(_target: u32, info: u32, status: u32, flags: u32, damage: u32) -> u32 {
        dmg::dmg_calc_no_damage_death(&mut mem(), info, status, flags, damage)
    }
    /// `DmgCalc_Alchemy(user, command, command_id, formula, damage)`.
    DmgCalc_Alchemy @ 0x0078_c5f0 => extern "C" fn dmg_calc_alchemy(user: u32, command: u32, command_id: u32, formula: i32, damage: i32) -> i32 {
        dmg::dmg_calc_alchemy(&mem(), user, command, command_id, formula, damage)
    }
    /// `DmgCalc_HitKind(flags, counts)`.
    DmgCalc_HitKind @ 0x0078_c640 => extern "C" fn dmg_calc_hit_kind(flags: i32, counts: u32) -> u32 {
        dmg::dmg_calc_hit_kind(&mem(), flags, counts)
    }
    /// `DmgCalc_IsDistill(status2)`.
    DmgCalc_IsDistill @ 0x0078_c670 => extern "C" fn dmg_calc_is_distill(status2: u32) -> i32 {
        dmg::dmg_calc_is_distill(status2)
    }
    /// `DmgCalc_InflictStatus(user_id, user, target_id, target, command, inflicted, removed, info, flags, damage, &delayed)`.
    DmgCalc_InflictStatus @ 0x0078_ae00 => extern "C" fn dmg_calc_inflict_status(user_id: u32, user: u32, target_id: u32, target: u32, command: u32, inflicted: u32, removed: u32, info: u32, flags: u32, damage: u32, delayed: u32) -> u32 {
        dmg::dmg_calc_inflict_status(&mut mem(), user_id, user, target_id, target, command, inflicted, removed, info, flags, damage, delayed)
    }
    /// `DmgCalc_InflictStatus_Extra(user_id, user, target_id, target, command, inflicted, removed, info, ret, damage, status2)`.
    DmgCalc_InflictStatus_Extra @ 0x0078_b4e0 => extern "C" fn dmg_calc_inflict_status_extra(user_id: u32, user: u32, _target_id: u32, target: u32, command: u32, inflicted: u32, removed: u32, info: u32, ret: u32, _damage: u32, status2: u32) -> u32 {
        dmg::dmg_calc_inflict_status_extra(&mut mem(), user_id, user, target, command, inflicted, removed, info, ret, status2)
    }
    /// `DmgCalc_ResultType(user_id, target_id, class, armored, def, mdef, damage, inflicted, removed, command, status, asleep, what, _, nulls)`.
    DmgCalc_ResultType @ 0x0078_c270 => extern "C" fn dmg_calc_result_type(user_id: u32, target_id: u32, class: i32, armored: i32, def: i32, mdef: i32, damage: u32, inflicted: u32, removed: u32, _command: u32, status: u32, asleep: i32, what: u32, _what2: u32, nulls: u32) -> u32 {
        dmg::dmg_calc_result_type(&mut mem(), user_id, target_id, class, armored, def, mdef, damage, inflicted, removed, status, asleep, what, nulls)
    }
    /// `DmgCalc_Bribe(target_id, target, command, counts, info)`; the language from `TOGetFFXLang`.
    DmgCalc_Bribe @ 0x0078_bdd0 => extern "C" fn dmg_calc_bribe(target_id: u32, target: u32, command: u32, counts: u32, info: u32) -> u32 {
        dmg::dmg_calc_bribe(&mut mem(), target_id, target, command, counts, info, |_| call0(TO_GET_FFX_LANG))
    }
    /// `DmgCalc(user_id, user, target_id, target, command, command_id, info, &damage, &hits, &misses, &changed)`: returns the HP damage.
    DmgCalc @ 0x0078_e630 => extern "C" fn dmg_calc(user_id: u32, user: u32, target_id: u32, target: u32, command: u32, command_id: u32, info: u32, damage: u32, hits: u32, misses: u32, changed: u32) -> i32 {
        let out = dmg::DmgCalcOut { damage, hits, misses, changed };
        dmg::dmg_calc(&mut mem(), user_id, user, target_id, target, command, command_id, info, out, |_| call0(TO_GET_FFX_LANG))
    }

    /// `MsCalcDamage(user_id, user, target_id, target, command, command_id, results, &damage, &hits, &misses, hits_n)`.
    MsCalcDamage @ 0x0078_9740 => extern "C" fn ms_calc_damage(user_id: u32, user: u32, target_id: u32, target: u32, command: u32, command_id: u32, results: u32, damage: u32, hits: u32, misses: u32, hits_n: i32) -> i32 {
        let out = dmg::DmgCalcOut { damage, hits, misses, changed: 0 };
        dmg::ms_calc_damage(&mut mem(), user_id, user, target_id, target, command, command_id, results, out, hits_n, |_| call0(TO_GET_FFX_LANG))
    }
    /// `MsCalcDrainResult(user_id, target, command, results, damage, none)`.
    MsCalcDrainResult @ 0x0078_a240 => extern "C" fn ms_calc_drain_result(user_id: u32, target: u32, command: u32, results: u32, damage: u32, none: i32) {
        dmg::ms_calc_drain_result(&mut mem(), user_id, target, command, results, damage, none);
    }

    // Commands.
    /// `MsGetCommand(chr, _, stop, list, &id)`: a command's kernel record.
    MsGetCommand @ 0x0078_ce50 => extern "C" fn ms_get_command(_chr: u32, _unused: u32, stop: u32, list: u32, id: u32) -> u32 {
        cmd::ms_get_command(&mut mem(), stop, list, id)
    }
    /// `MsGetCommandMP(chr, command)`.
    MsGetCommandMP @ 0x0078_cf70 => extern "C" fn ms_get_command_mp(chr: u32, command: u32) -> i32 {
        cmd::ms_get_command_mp(&mut mem(), chr, command)
    }
    /// `MsCheckUseCommand(chr, command, extra)`: its MP cost, or -1.
    MsCheckUseCommand @ 0x0078_c690 => extern "C" fn ms_check_use_command(chr: u32, command: u32, extra: i32) -> i32 {
        cmd::ms_check_use_command(&mut mem(), chr, command, extra)
    }
    /// `MsCalcMagic(cue, k)`.
    MsCalcMagic @ 0x0078_ab20 => extern "C" fn ms_calc_magic(cue: u32, k: u32) -> i32 {
        cmd::ms_calc_magic(&mut mem(), cue, k)
    }
    /// `MsCalcCostCTB(cue)`: the CTB rank.
    MsCalcCostCTB @ 0x0078_95b0 => extern "C" fn ms_calc_cost_ctb(cue: u32) -> u32 {
        cmd::ms_calc_cost_ctb(&mut mem(), cue)
    }
    /// `MsCalcDamageCTB(cue, ctb, rank, haste, slow)`.
    MsCalcDamageCTB @ 0x0078_98d0 => extern "C" fn ms_calc_damage_ctb(cue: u32, ctb: u32, rank: i32, haste: u32, slow: u32) {
        cmd::ms_calc_damage_ctb(&mut mem(), cue, ctb, rank, haste, slow);
    }

    // Actor getters.
    /// `MsGetRamChrHP(i)`.
    MsGetRamChrHP @ 0x0079_add0 => extern "C" fn ms_get_ram_chr_hp(i: u32) -> u32 {
        actor::ms_get_ram_chr_hp(&mem(), i)
    }
    /// `MsGetRamChrHPmax(i)`.
    MsGetRamChrHPmax @ 0x0079_adf0 => extern "C" fn ms_get_ram_chr_hp_max(i: u32) -> u32 {
        actor::ms_get_ram_chr_hp_max(&mem(), i)
    }
    /// `MsGetRamChrMP(i)`.
    MsGetRamChrMP @ 0x0079_ae50 => extern "C" fn ms_get_ram_chr_mp(i: u32) -> u32 {
        actor::ms_get_ram_chr_mp(&mem(), i)
    }
    /// `MsGetRamChrMPmax(i)`.
    MsGetRamChrMPmax @ 0x0079_ae70 => extern "C" fn ms_get_ram_chr_mp_max(i: u32) -> u32 {
        actor::ms_get_ram_chr_mp_max(&mem(), i)
    }
    /// `MsGetRamChrMag(i)`.
    MsGetRamChrMag @ 0x0079_ae90 => extern "C" fn ms_get_ram_chr_mag(i: u32) -> u32 {
        actor::ms_get_ram_chr_mag(&mem(), i)
    }
    /// `MsGetRamChrName(i)`.
    MsGetRamChrName @ 0x0079_af10 => extern "C" fn ms_get_ram_chr_name(i: u32) -> u32 {
        actor::ms_get_ram_chr_name(&mem(), i)
    }
    /// `MsGetRamChrIconNumber(i)`.
    MsGetRamChrIconNumber @ 0x0079_ae10 => extern "C" fn ms_get_ram_chr_icon_number(i: u32) -> u32 {
        actor::ms_get_ram_chr_icon_number(&mem(), i)
    }
    /// `MsGetRamChrIconType(i)`.
    MsGetRamChrIconType @ 0x0079_ae30 => extern "C" fn ms_get_ram_chr_icon_type(i: u32) -> u32 {
        actor::ms_get_ram_chr_icon_type(&mem(), i)
    }
    /// `MsGetRamChrMegas(i)`.
    MsGetRamChrMegas @ 0x0079_aed0 => extern "C" fn ms_get_ram_chr_megas(i: u32) -> u32 {
        actor::ms_get_ram_chr_megas(&mem(), i)
    }
    /// `MsGetRamChrDisableZan(i)`.
    MsGetRamChrDisableZan @ 0x0079_ad90 => extern "C" fn ms_get_ram_chr_disable_zan(i: u32) -> u32 {
        actor::ms_get_ram_chr_disable_zan(&mem(), i)
    }
    /// `MsGetRamChrCTBmax(i)`.
    MsGetRamChrCTBmax @ 0x0079_ad10 => extern "C" fn ms_get_ram_chr_ctb_max(i: u32) -> u32 {
        actor::ms_get_ram_chr_ctb_max(&mem(), i)
    }
    /// `MsGetChrStatDeath(i)`.
    MsGetChrStatDeath @ 0x0079_5780 => extern "C" fn ms_get_chr_stat_death(i: u32) -> u32 {
        actor::ms_get_chr_stat_death(&mem(), i)
    }
    /// `MsGetChrStatStone(i)`.
    MsGetChrStatStone @ 0x0079_5800 => extern "C" fn ms_get_chr_stat_stone(i: u32) -> u32 {
        actor::ms_get_chr_stat_stone(&mem(), i)
    }
    /// `MsGetChrStatWeak(i)`.
    MsGetChrStatWeak @ 0x0079_5840 => extern "C" fn ms_get_chr_stat_weak(i: u32) -> u32 {
        actor::ms_get_chr_stat_weak(&mem(), i)
    }
    /// `MsGetChrStatZombie(i)`.
    MsGetChrStatZombie @ 0x0079_5880 => extern "C" fn ms_get_chr_stat_zombie(i: u32) -> u32 {
        actor::ms_get_chr_stat_zombie(&mem(), i)
    }
    /// `MsGetChrStatPoison(i)`.
    MsGetChrStatPoison @ 0x0079_57c0 => extern "C" fn ms_get_chr_stat_poison(i: u32) -> u32 {
        actor::ms_get_chr_stat_poison(&mem(), i)
    }
    /// `MsGetChrLimitGauge(i)`.
    MsGetChrLimitGauge @ 0x0079_5550 => extern "C" fn ms_get_chr_limit_gauge(i: u32) -> u32 {
        actor::ms_get_chr_limit_gauge(&mem(), i)
    }
    /// `MsGetChrLimitGaugeMax(i)`.
    MsGetChrLimitGaugeMax @ 0x0079_5590 => extern "C" fn ms_get_chr_limit_gauge_max(i: u32) -> u32 {
        actor::ms_get_chr_limit_gauge_max(&mem(), i)
    }
    /// `MsGetChrHandle(i)`.
    MsGetChrHandle @ 0x0079_5080 => extern "C" fn ms_get_chr_handle(i: u32) -> u32 {
        actor::ms_get_chr_handle(&mem(), i)
    }
    /// `MsGetChrGroup(i)`.
    MsGetChrGroup @ 0x0079_5040 => extern "C" fn ms_get_chr_group(i: u32) -> u32 {
        actor::ms_get_chr_group(&mem(), i)
    }
    /// `MsGetChrCursorFlag(i)`.
    MsGetChrCursorFlag @ 0x0079_4ca0 => extern "C" fn ms_get_chr_cursor_flag(i: u32) -> u32 {
        actor::ms_get_chr_cursor_flag(&mem(), i)
    }
    /// `MsGetChrCommand(i)`.
    MsGetChrCommand @ 0x007b_09b0 => extern "C" fn ms_get_chr_command(i: u32) -> u32 {
        actor::ms_get_chr_command(&mem(), i)
    }
    /// `MsGetChrCommand2(i)`.
    MsGetChrCommand2 @ 0x007b_09e0 => extern "C" fn ms_get_chr_command2(i: u32) -> u32 {
        actor::ms_get_chr_command2(&mem(), i)
    }
    /// `MsBattleCheck`.
    MsBattleCheck @ 0x0078_0c90 => extern "C" fn ms_battle_check() -> u32 {
        state::ms_battle_check(&mem())
    }
    /// `MsBattleChrStart`.
    MsBattleChrStart @ 0x0078_0cb0 => extern "C" fn ms_battle_chr_start() -> u32 {
        state::ms_battle_chr_start(&mem())
    }
    /// `MsBattleGetNum`.
    MsBattleGetNum @ 0x0078_16b0 => extern "C" fn ms_battle_get_num() -> u32 {
        state::ms_battle_get_num(&mem())
    }

    // Vectors and matrices (VU0 code on the x87).
    /// `MsMulVector(v, s, b1, b2, b3)`: `v *= s` (floats as bits).
    MsMulVector @ 0x007c_2e10 => extern "C" fn ms_mul_vector(v: u32, s: u32, b1: u32, b2: u32, b3: u32) {
        vector::ms_mul_vector(&mut mem(), v, [s, b1, b2, b3]);
    }
    /// `MsDivVector(v, s, b1, b2, b3)`: `v *= 1 / s`.
    MsDivVector @ 0x007c_2520 => extern "C" fn ms_div_vector(v: u32, s: u32, b1: u32, b2: u32, b3: u32) {
        vector::ms_div_vector(&mut mem(), v, [s, b1, b2, b3]);
    }
    /// `MsAddPoint(out, a, b)`.
    MsAddPoint @ 0x007c_1480 => extern "C" fn ms_add_point(out: u32, a: u32, b: u32) {
        vector::ms_add_point(&mut mem(), out, a, b);
    }
    /// `MsSetNormalUpVector(v)`.
    MsSetNormalUpVector @ 0x007c_0ab0 => extern "C" fn ms_set_normal_up_vector(v: u32) {
        vector::ms_set_normal_up_vector(&mut mem(), v);
    }
    /// `MsApplyMatrix(out, m, v)`.
    MsApplyMatrix @ 0x007c_1550 => extern "C" fn ms_apply_matrix(out: u32, m: u32, v: u32) {
        vector::ms_apply_matrix(&mut mem(), out, m, v);
    }
    /// `MsNearRag(a, b)`: an angle, on the x87 stack.
    MsNearRag @ 0x007c_04c0 => extern "C" fn ms_near_rag(a: u32, b: u32) -> f32 {
        vector::ms_near_rag(&mem(), a, b)
    }

    // Battle messages.
    /// `MsMenuCloseTitleWindow(keep)`.
    MsMenuCloseTitleWindow @ 0x0078_fa30 => extern "C" fn ms_menu_close_title_window(keep: i32) {
        state::ms_menu_close_title_window(&mut mem(), keep, |_| {
            call0(TO_BTL_CLOSE_SIGN_WINDOW);
        });
    }
    /// `MsMessageCueRegist(kind, actor, message, b, c)`.
    MsMessageCueRegist @ 0x0079_cfd0 => extern "C" fn ms_message_cue_regist(kind: u32, actor: u32, message: u32, b: u32, c: u32) -> u32 {
        let extra = [b.to_le_bytes()[0], c.to_le_bytes()[0]];
        state::ms_message_cue_regist(&mut mem(), kind.to_le_bytes()[0], actor, message, extra, |_| {
            call0(TO_BTL_CLOSE_SIGN_WINDOW);
        })
    }
    /// `MsMonsterCapture(chr, monster)`.
    MsMonsterCapture @ 0x0079_0b30 => extern "C" fn ms_monster_capture(chr: u32, monster: u32) -> u32 {
        state::ms_monster_capture(&mut mem(), chr, monster, |_| {
            call0(TO_BTL_CLOSE_SIGN_WINDOW);
        })
    }

    /// `MsMulMatrix(out, a, b)`.
    MsMulMatrix @ 0x007c_2770 => extern "C" fn ms_mul_matrix(out: u32, a: u32, b: u32) {
        vector::ms_mul_matrix(&mut mem(), out, a, b);
    }
    /// `MsMiddlePoint(out, p, q, s, t, c, d, e)` (floats as bits).
    MsMiddlePoint @ 0x007c_25e0 => extern "C" fn ms_middle_point(out: u32, p: u32, q: u32, s: u32, t: u32, c: u32, d: u32, e: u32) {
        vector::ms_middle_point(&mut mem(), out, p, q, [s, t, c, d, e]);
    }
    /// `MsVectorLen(v)`: on the x87 stack.
    MsVectorLen @ 0x007c_4280 => extern "C" fn ms_vector_len(v: u32) -> f32 {
        vector::ms_vector_len(&mut mem(), v)
    }
    /// `MsVectorLen2D(v)`: on the x87 stack.
    MsVectorLen2D @ 0x007c_43a0 => extern "C" fn ms_vector_len_2d(v: u32) -> f32 {
        vector::ms_vector_len_2d(&mut mem(), v)
    }
    /// `MsVectorNormal(out, v)`.
    MsVectorNormal @ 0x007c_44a0 => extern "C" fn ms_vector_normal(out: u32, v: u32) {
        vector::ms_vector_normal(&mut mem(), out, v);
    }

    // Battle bookkeeping (ffx_battle::misc).
    /// `MsComBufInit`.
    MsComBufInit @ 0x0079_d290 => extern "C" fn ms_com_buf_init() {
        misc::ms_com_buf_init(&mut mem());
    }
    /// `MsEtEffectReadInit`.
    MsEtEffectReadInit @ 0x0079_e730 => extern "C" fn ms_et_effect_read_init() {
        misc::ms_et_effect_read_init(&mut mem());
    }
    /// `MsNumberInit`.
    MsNumberInit @ 0x0079_f9f0 => extern "C" fn ms_number_init() {
        misc::ms_number_init(&mut mem());
    }
    /// `MsActionInit`.
    MsActionInit @ 0x007a_ce90 => extern "C" fn ms_action_init() {
        misc::ms_action_init(&mut mem());
    }
    /// `MsEffectCameraReset`.
    MsEffectCameraReset @ 0x007b_bb00 => extern "C" fn ms_effect_camera_reset() {
        misc::ms_effect_camera_reset(&mut mem());
    }
    /// `MsBattleGetFontAddress`.
    MsBattleGetFontAddress @ 0x0078_1680 => extern "C" fn ms_battle_get_font_address() -> u32 {
        misc::ms_battle_get_font_address(&mem())
    }
    /// `MsRegMotionInit`.
    MsRegMotionInit @ 0x0079_ccd0 => extern "C" fn ms_reg_motion_init() {
        misc::ms_reg_motion_init(&mut mem());
    }
    /// `MsBtlStartAP`.
    MsBtlStartAP @ 0x0079_8e90 => extern "C" fn ms_btl_start_ap() {
        misc::ms_btl_start_ap(&mut mem());
    }
    /// `MsBtlInitItemMem`.
    MsBtlInitItemMem @ 0x0079_a000 => extern "C" fn ms_btl_init_item_mem() {
        misc::ms_btl_init_item_mem(&mut mem());
    }
    /// `MsBtlInitItemMem_007a0cd0`.
    MsBtlInitItemMem_007a0cd0 @ 0x007a_0cd0 => extern "C" fn ms_btl_init_item_mem2() {
        misc::ms_btl_init_item_mem2(&mut mem());
    }
    /// `MsMessageCueInit`.
    MsMessageCueInit @ 0x007a_2730 => extern "C" fn ms_message_cue_init() {
        misc::ms_message_cue_init(&mut mem());
    }
    /// `MsSplineInit`.
    MsSplineInit @ 0x007b_aaa0 => extern "C" fn ms_spline_init() {
        misc::ms_spline_init(&mut mem());
    }
    /// `MsBattleSpecialCameraFree`.
    MsBattleSpecialCameraFree @ 0x0079_dd90 => extern "C" fn ms_battle_special_camera_free() {
        misc::ms_battle_special_camera_free(&mut mem());
    }
    /// `MsBtlReadStatusComplete`.
    MsBtlReadStatusComplete @ 0x007a_d550 => extern "C" fn ms_btl_read_status_complete() {
        misc::ms_btl_read_status_complete(&mut mem());
    }
    /// `MsScreenFocusReset`.
    MsScreenFocusReset @ 0x007b_bd80 => extern "C" fn ms_screen_focus_reset() {
        misc::ms_screen_focus_reset(&mut mem());
    }
    /// `MsBtlListInit`.
    MsBtlListInit @ 0x0079_d250 => extern "C" fn ms_btl_list_init() {
        misc::ms_btl_list_init(&mut mem());
    }
    /// `MsBtlReadInit`.
    MsBtlReadInit @ 0x0078_3000 => extern "C" fn ms_btl_read_init() {
        misc::ms_btl_read_init(&mut mem());
    }
    /// `MsMoveTargetPos`.
    MsMoveTargetPos @ 0x007a_b160 => extern "C" fn ms_move_target_pos() -> u32 {
        misc::ms_move_target_pos(&mem())
    }
    /// `FinishReadPhyreBattleMap`.
    FinishReadPhyreBattleMap @ 0x0078_2fd0 => extern "C" fn finish_read_phyre_battle_map() {
        misc::finish_read_phyre_battle_map(&mut mem());
    }
    /// `MsDebugCameraSleepInit`.
    MsDebugCameraSleepInit @ 0x0079_2200 => extern "C" fn ms_debug_camera_sleep_init() {
        misc::ms_debug_camera_sleep_init(&mut mem());
    }
    /// `MsResetDirMove(p)`.
    MsResetDirMove @ 0x007a_b170 => extern "C" fn ms_reset_dir_move(p: u32) {
        misc::ms_reset_dir_move(&mut mem(), p);
    }
    /// `MsPaymentProcess(p)`.
    MsPaymentProcess @ 0x007a_fa60 => extern "C" fn ms_payment_process(p: u32) -> u32 {
        misc::ms_payment_process(&mem(), p)
    }
    /// `MsMotionSpeedSetNormal(p)`.
    MsMotionSpeedSetNormal @ 0x007a_a260 => extern "C" fn ms_motion_speed_set_normal(p: u32) {
        misc::ms_motion_speed_set_normal(&mut mem(), p);
    }
    /// `msMoveReset(p)`.
    msMoveReset @ 0x007a_a280 => extern "C" fn ms_move_reset(p: u32) {
        misc::ms_move_reset(&mut mem(), p);
    }
    /// `MsSetMenuCommand(p)`.
    MsSetMenuCommand @ 0x0079_2e50 => extern "C" fn ms_set_menu_command(p: u32) {
        misc::ms_set_menu_command(&mut mem(), p);
    }
    /// `MsSummonLimitSave(p)`.
    MsSummonLimitSave @ 0x007b_06e0 => extern "C" fn ms_summon_limit_save(p: u32) {
        misc::ms_summon_limit_save(&mut mem(), p);
    }
    /// `MsSummonLimitLoad(p)`.
    MsSummonLimitLoad @ 0x007b_06b0 => extern "C" fn ms_summon_limit_load(p: u32) {
        misc::ms_summon_limit_load(&mut mem(), p);
    }
    /// `MsEffectGetBufferAddress(p)`.
    MsEffectGetBufferAddress @ 0x007b_7eb0 => extern "C" fn ms_effect_get_buffer_address(p: u32) -> u32 {
        misc::ms_effect_get_buffer_address(&mut mem(), p)
    }
    /// `MsEffectGetLastMagicNum2(p)`.
    MsEffectGetLastMagicNum2 @ 0x0078_7bf0 => extern "C" fn ms_effect_get_last_magic_num2(p: u32) -> u32 {
        misc::ms_effect_get_last_magic_num2(&mem(), p)
    }
    /// `MsMoveSplineSet(p)`.
    MsMoveSplineSet @ 0x007a_b140 => extern "C" fn ms_move_spline_set(p: u32) {
        misc::ms_move_spline_set(&mut mem(), p);
    }
    /// `MsSaveUcID(id)`.
    MsSaveUcID @ 0x007d_42e0 => extern "C" fn ms_save_uc_id(id: u32) {
        misc::ms_save_uc_id(&mut mem(), id.to_le_bytes()[..2].try_into().map_or(0, u16::from_le_bytes));
    }
    /// `MsScreenInit(w, h)`.
    MsScreenInit @ 0x007c_0540 => extern "C" fn ms_screen_init(w: u32, h: u32) {
        misc::ms_screen_init(&mut mem(), w, h);
    }
    /// `MsBattleSpecialCameraPause(camera)`.
    MsBattleSpecialCameraPause @ 0x0079_ddb0 => extern "C" fn ms_battle_special_camera_pause(camera: u32) {
        misc::ms_battle_special_camera_pause(&mut mem(), camera.to_le_bytes()[0]);
    }
    /// `MsScreenOffset(x, y)` (floats as bits).
    MsScreenOffset @ 0x007b_c0c0 => extern "C" fn ms_screen_offset(x: u32, y: u32) {
        misc::ms_screen_offset(&mut mem(), x, y);
    }
    /// `MsScreenResetOffset(x, y)`.
    MsScreenResetOffset @ 0x007b_c720 => extern "C" fn ms_screen_reset_offset(x: u32, y: u32) {
        misc::ms_screen_reset_offset(&mut mem(), x, y);
    }
    /// `MsEffectSetCameraBufFloat(i, f)`.
    MsEffectSetCameraBufFloat @ 0x007b_7f50 => extern "C" fn ms_effect_set_camera_buf_float(i: u32, f: u32) {
        misc::ms_effect_set_camera_buf_float(&mut mem(), i, f);
    }

    /// `MsGetChrBit(chr, selector, all, any)`: a set of actors as bits.
    MsGetChrBit @ 0x0079_4330 => extern "C" fn ms_get_chr_bit(chr: u32, selector: u32, all: i32, any: i32) -> u32 {
        actor::ms_get_chr_bit(&mem(), chr, selector, all, any)
    }
    /// `MsGetChrWidth(i)`: on the x87 stack.
    MsGetChrWidth @ 0x0079_5a20 => extern "C" fn ms_get_chr_width(i: u32) -> f32 {
        actor::ms_get_chr_width(&mem(), i)
    }
    /// `MsGetChrScale(i, out)`.
    MsGetChrScale @ 0x0079_5720 => extern "C" fn ms_get_chr_scale(i: u32, out: u32) -> i32 {
        actor::ms_get_chr_scale(&mut mem(), i, out)
    }
    /// `MsGetChrDist(i)`.
    MsGetChrDist @ 0x0079_4e00 => extern "C" fn ms_get_chr_dist(i: u32) -> u32 {
        actor::ms_get_chr_dist(&mem(), i)
    }
    /// `MsGetChrBtlDir(i)`: on the x87 stack.
    MsGetChrBtlDir @ 0x0079_48a0 => extern "C" fn ms_get_chr_btl_dir(i: u32) -> f32 {
        actor::ms_get_chr_btl_dir(&mem(), i)
    }
    /// `MsGetChrGroundPos(i, &x, &y)`: EAX as the original leaves it.
    MsGetChrGroundPos @ 0x0079_4ff0 => extern "C" fn ms_get_chr_ground_pos(i: u32, x: u32, y: u32) -> u32 {
        actor::ms_get_chr_ground_pos(&mut mem(), i, x, y)
    }
    /// `MsGetChrTargetPos(i, &x, &y)`: EAX as the original leaves it.
    MsGetChrTargetPos @ 0x0079_5920 => extern "C" fn ms_get_chr_target_pos(i: u32, x: u32, y: u32) -> u32 {
        actor::ms_get_chr_target_pos(&mut mem(), i, x, y)
    }
    /// `MsGetChrEventFlag(i, event)`.
    MsGetChrEventFlag @ 0x0079_4f20 => extern "C" fn ms_get_chr_event_flag(i: u32, event: u32) -> u32 {
        actor::ms_get_chr_event_flag(&mem(), i, event)
    }
    /// `MsGetChrMotionSignal(i)`.
    MsGetChrMotionSignal @ 0x0079_5660 => extern "C" fn ms_get_chr_motion_signal(i: u32) -> u32 {
        actor::ms_get_chr_motion_signal(&mem(), i)
    }
    /// `MsGetChrModelID(i)`.
    MsGetChrModelID @ 0x0079_5610 => extern "C" fn ms_get_chr_model_id(i: u32) -> u32 {
        actor::ms_get_chr_model_id(&mem(), i)
    }
    /// `MsGetChrMotionSpeed(i)`.
    MsGetChrMotionSpeed @ 0x0079_56a0 => extern "C" fn ms_get_chr_motion_speed(i: u32) -> u32 {
        actor::ms_get_chr_motion_speed(&mem(), i)
    }
    /// `MsGetWeaponHandle(i, second)`.
    MsGetWeaponHandle @ 0x0079_5b40 => extern "C" fn ms_get_weapon_handle(i: u32, second: i32) -> u32 {
        actor::ms_get_weapon_handle(&mem(), i, second)
    }
    /// `MsGetWeaponSpecial(id, k)`.
    MsGetWeaponSpecial @ 0x0079_5bb0 => extern "C" fn ms_get_weapon_special(id: u32, k: u32) -> u32 {
        actor::ms_get_weapon_special(&mem(), id, k)
    }
    /// `MsCameraGetID(cam)`.
    MsCameraGetID @ 0x007b_d300 => extern "C" fn ms_camera_get_id(cam: u32) -> u32 {
        camera::ms_camera_get_id(&mem(), cam)
    }

    // Items and steals.
    /// `MsSetItemWindow`: the battle item lists.
    MsSetItemWindow @ 0x0079_06a0 => extern "C" fn ms_set_item_window() {
        save::ms_set_item_window(&mut mem());
    }
    /// `MsSaveItemUse(item, n)`.
    MsSaveItemUse @ 0x0079_0550 => extern "C" fn ms_save_item_use(item: u32, n: i32) -> i32 {
        save::ms_save_item_use(&mut mem(), item, n)
    }
    /// `Battle_Steal(user, target, spoils, info, fail)`.
    Battle_Steal @ 0x0078_b760 => extern "C" fn battle_steal(user: u32, target: u32, spoils: u32, info: u32, fail: i32) -> u32 {
        save::battle_steal(&mut mem(), user, target, spoils, info, fail, |_| call0(TO_GET_FFX_LANG), |_, n| {
            call1(ACHIEVEMENT_UNLOCK, n);
        })
    }
    /// `Battle_StealGil(user, target, spoils, info, fail)`: Mug's gil.
    Battle_StealGil @ 0x0078_b920 => extern "C" fn battle_steal_gil(_user: u32, target: u32, spoils: u32, info: u32, fail: i32) -> u32 {
        save::battle_steal_gil(&mut mem(), target, spoils, info, fail)
    }
    /// `MsGetActionBuffer(actor)`.
    MsGetActionBuffer @ 0x0078_d040 => extern "C" fn ms_get_action_buffer(actor: u32) -> u32 {
        save::ms_get_action_buffer(&mut mem(), actor)
    }

    /// `MsGetCommandFar(chr, command)`.
    MsGetCommandFar @ 0x0079_9690 => extern "C" fn ms_get_command_far(chr: i32, command: u32) -> u32 {
        actor::ms_get_command_far(&mem(), chr, command)
    }
    /// `MsGetCursorTarget(chr, selector, command, event)`.
    MsGetCursorTarget @ 0x0079_9820 => extern "C" fn ms_get_cursor_target(chr: u32, selector: u32, command: u32, event: u32) -> u32 {
        actor::ms_get_cursor_target(&mem(), chr, selector, command, event)
    }
    /// `MsGetGuardChr(user, target)`.
    MsGetGuardChr @ 0x0078_eef0 => extern "C" fn ms_get_guard_chr(user: u32, target: u32) -> u32 {
        actor::ms_get_guard_chr(&mut mem(), user, target)
    }
    /// `MsGetChrVigor(i)`.
    MsGetChrVigor @ 0x0078_cce0 => extern "C" fn ms_get_chr_vigor(i: u32) -> u32 {
        actor::ms_get_chr_vigor(&mem(), i)
    }
    /// `MsSetDirThetaMode(actor)`.
    MsSetDirThetaMode @ 0x0079_6940 => extern "C" fn ms_set_dir_theta_mode(actor: u32) -> u32 {
        minor::ms_set_dir_theta_mode(&mut mem(), actor)
    }
    /// `MsBtlListField(i)`.
    MsBtlListField @ 0x0079_d190 => extern "C" fn ms_btl_list_field(i: i32) -> u32 {
        minor::ms_btl_list_field(&mem(), i)
    }
    /// `MsBtlListFieldNum(map)`.
    MsBtlListFieldNum @ 0x0079_d1c0 => extern "C" fn ms_btl_list_field_num(map: i32) -> i32 {
        minor::ms_btl_list_field_num(&mem(), map)
    }
    /// `ResetEncountExe(all)`.
    ResetEncountExe @ 0x0078_0ff0 => extern "C" fn reset_encount_exe(all: u32) {
        minor::reset_encount_exe(&mut mem(), all);
    }
    /// `MsEffectCameraCall(camera, mode)`.
    MsEffectCameraCall @ 0x0078_7740 => extern "C" fn ms_effect_camera_call(camera: i32, mode: u32) -> u32 {
        minor::ms_effect_camera_call(&mut mem(), camera, mode)
    }
    /// `resetZanmatoSepList()`.
    resetZanmatoSepList @ 0x007a_8f20 => extern "C" fn reset_zanmato_sep_list() -> u32 {
        minor::reset_zanmato_sep_list(&mut mem())
    }
    /// `MsSetMotBinAddress(actor, i)`.
    MsSetMotBinAddress @ 0x0079_cd80 => extern "C" fn ms_set_mot_bin_address(actor: u32, i: i32) -> u32 {
        minor::ms_set_mot_bin_address(&mut mem(), actor, i)
    }
    /// `MsAtelRegist(manager, name, script)`.
    MsAtelRegist @ 0x0079_7a40 => extern "C" fn ms_atel_regist(manager: i32, _name: u32, script: u32) -> i32 {
        minor::ms_atel_regist(&mut mem(), manager, script)
    }
    /// `MsSEPtransInit()`.
    MsSEPtransInit @ 0x007a_0500 => extern "C" fn ms_sep_trans_init() -> u32 {
        minor::ms_sep_trans_init(&mut mem())
    }
    /// `MsAtelCamInit()`.
    MsAtelCamInit @ 0x0079_7500 => extern "C" fn ms_atel_cam_init() {
        minor::ms_atel_cam_init(&mut mem());
    }
    /// `Mon_FindHandler(file, i, &k)`.
    Mon_FindHandler @ 0x0079_7410 => extern "C" fn mon_find_handler(file: u32, i: i32, k: u32) -> u32 {
        minor::mon_find_handler(&mut mem(), file, i, k)
    }
    /// `MsMotBinFrame(_, actor, motion, frame)`.
    MsMotBinFrame @ 0x0079_cb40 => extern "C" fn ms_mot_bin_frame(_x: u32, actor: u32, motion: u32, frame: u32) -> u32 {
        minor::ms_mot_bin_frame(&mem(), actor, motion, frame)
    }
    /// `MsSEP3DfadeOutAll(time)`.
    MsSEP3DfadeOutAll @ 0x007a_08e0 => extern "C" fn ms_sep3d_fade_out_all(time: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        minor::ms_sep3d_fade_out_all(&mut mem(), time as u16)
    }
    /// `MsWeaponReadSet(actor, slot, weapon)`.
    MsWeaponReadSet @ 0x007a_d940 => extern "C" fn ms_weapon_read_set(actor: u32, slot: u32, weapon: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        minor::ms_weapon_read_set(&mut mem(), actor, slot as u8, weapon);
    }
    /// `MsEffectCameraBufInit()`.
    MsEffectCameraBufInit @ 0x0078_76e0 => extern "C" fn ms_effect_camera_buf_init() {
        minor::ms_effect_camera_buf_init(&mut mem());
    }
    /// `MsInitSaveWeaponEquipFlag()`.
    MsInitSaveWeaponEquipFlag @ 0x007a_be30 => extern "C" fn ms_init_save_weapon_equip_flag() -> u32 {
        minor::ms_init_save_weapon_equip_flag(&mut mem())
    }
    /// `MsStoneRegist(chr)`.
    MsStoneRegist @ 0x007a_2780 => extern "C" fn ms_stone_regist(chr: u32) -> u32 {
        minor::ms_stone_regist(&mut mem(), chr)
    }
    /// `MsSetSaveCommandUse(chr, command, on)`.
    MsSetSaveCommandUse @ 0x0078_5e00 => extern "C" fn ms_set_save_command_use(chr: i32, command: u32, on: u32) -> u32 {
        minor::ms_set_save_command_use(&mut mem(), chr, command, on)
    }
    /// `MsAtelCameraInit()`.
    MsAtelCameraInit @ 0x007b_a650 => extern "C" fn ms_atel_camera_init() {
        minor::ms_atel_camera_init(&mut mem());
    }
    /// `MsNumberCalcProcess()`.
    MsNumberCalcProcess @ 0x0079_f5e0 => extern "C" fn ms_number_calc_process() -> u32 {
        minor::ms_number_calc_process(&mut mem())
    }
    /// `MsGetNowSelectedCamraLevel()`.
    MsGetNowSelectedCamraLevel @ 0x007b_bc30 => extern "C" fn ms_get_now_selected_camra_level() -> i32 {
        camera::ms_get_now_selected_camra_level(&mem())
    }
    /// `MsCameraFree(id)`.
    MsCameraFree @ 0x007b_d000 => extern "C" fn ms_camera_free(id: u32) -> u32 {
        camera::ms_camera_free(&mut mem(), id)
    }
    /// `MsEffectCameraInit()`.
    MsEffectCameraInit @ 0x007b_ba70 => extern "C" fn ms_effect_camera_init() -> u32 {
        camera::ms_effect_camera_init(&mut mem())
    }
    /// `MsEffectCameraSet(look_at, eye, depth, roll)` (floats as bits).
    MsEffectCameraSet @ 0x007b_bb10 => extern "C" fn ms_effect_camera_set(look_at: u32, eye: u32, depth: u32, roll: u32) -> u32 {
        camera::ms_effect_camera_set(&mut mem(), look_at, eye, depth, roll)
    }
    /// `MsSetScreen(i, a, b, c, d, e, near, far, z0, z1)` (floats as bits).
    MsSetScreen @ 0x007b_c7f0 => extern "C" fn ms_set_screen(i: i32, a: u32, b: u32, c: u32, d: u32, e: u32, near: u32, far: u32, z0: u32, z1: u32) {
        camera::ms_set_screen(&mut mem(), i, [a, b, c, d, e, near, far, z0, z1]);
    }
    /// `MsVectorGetSide(ax, az, bx, bz)` (floats as bits).
    MsVectorGetSide @ 0x007c_4180 => extern "C" fn ms_vector_get_side(ax: u32, az: u32, bx: u32, bz: u32) -> i32 {
        vector::ms_vector_get_side(&mut mem(), [ax, az, bx, bz])
    }
    /// `Battle_FirstSelectedActor(chr, selector, all, any)`.
    Battle_FirstSelectedActor @ 0x0079_47f0 => extern "C" fn battle_first_selected_actor(chr: u32, selector: u32, all: i32, any: i32) -> i32 {
        body::battle_first_selected_actor(&mem(), chr, selector, all, any)
    }
    /// `msSetChrBodyHit(actor, part, on)`.
    msSetChrBodyHit @ 0x0079_6f20 => extern "C" fn ms_set_chr_body_hit(actor: u32, part: i32, on: i32) -> u32 {
        body::ms_set_chr_body_hit(&mut mem(), actor, part, on)
    }
    /// `MsGetChrHeight(i)`: on the x87 stack.
    MsGetChrHeight @ 0x0079_50f0 => extern "C" fn ms_get_chr_height(i: u32) -> f32 {
        body::ms_get_chr_height(&mem(), i)
    }
    /// `MsGetChrCamHeight(i)`: on the x87 stack.
    MsGetChrCamHeight @ 0x0079_4a30 => extern "C" fn ms_get_chr_cam_height(i: u32) -> f32 {
        body::ms_get_chr_cam_height(&mem(), i)
    }
    /// `MsGetChrCamWidth(i)`: on the x87 stack.
    MsGetChrCamWidth @ 0x0079_4a70 => extern "C" fn ms_get_chr_cam_width(i: u32) -> f32 {
        body::ms_get_chr_cam_width(&mem(), i)
    }
    /// `MsGetChrDir(i, &dir)`.
    MsGetChrDir @ 0x0079_4d20 => extern "C" fn ms_get_chr_dir(i: u32, dir: u32) -> i32 {
        body::ms_get_chr_dir(&mut mem(), i, dir)
    }
    /// `MsGetChrGround(i)`: on the x87 stack.
    MsGetChrGround @ 0x0079_4fb0 => extern "C" fn ms_get_chr_ground(i: u32) -> f32 {
        body::ms_get_chr_ground(&mem(), i)
    }
    /// `MsSetChrPos(i, pos)`.
    MsSetChrPos @ 0x0079_6150 => extern "C" fn ms_set_chr_pos(i: u32, pos: u32) -> i32 {
        body::ms_set_chr_pos(&mut mem(), i, pos)
    }
    /// `MsSetChrWeight(i, how)`.
    MsSetChrWeight @ 0x0079_68e0 => extern "C" fn ms_set_chr_weight(i: u32, how: i32) {
        body::ms_set_chr_weight(&mut mem(), i, how);
    }
    /// `msSetChrWeight(actor, w)` (a float as bits).
    msSetChrWeight @ 0x0079_72c0 => extern "C" fn ms_set_chr_weight2(actor: u32, w: u32) {
        body::ms_set_chr_weight2(&mut mem(), actor, w);
    }
    /// `MsSetChrMoveFlag(i, v)`.
    MsSetChrMoveFlag @ 0x0079_6110 => extern "C" fn ms_set_chr_move_flag(i: u32, v: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        body::ms_set_chr_move_flag(&mut mem(), i, v as u8)
    }
    /// `MsSetChrMotionLevel(i, v)`.
    MsSetChrMotionLevel @ 0x0079_60d0 => extern "C" fn ms_set_chr_motion_level(i: u32, v: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        body::ms_set_chr_motion_level(&mut mem(), i, v as u8)
    }
    /// `MsSetChrSpecular(i, v, frames)` (v a float as bits).
    MsSetChrSpecular @ 0x0079_6360 => extern "C" fn ms_set_chr_specular(i: u32, v: u32, frames: i32) -> u32 {
        body::ms_set_chr_specular(&mut mem(), i, v, frames)
    }
    /// `MsCheckChrMotionAll(mode)`.
    MsCheckChrMotionAll @ 0x0079_1630 => extern "C" fn ms_check_chr_motion_all(mode: i32) -> u32 {
        body::ms_check_chr_motion_all(&mut mem(), mode)
    }
    /// `MsEffectResetMotionDisable(actor)`.
    MsEffectResetMotionDisable @ 0x0078_8270 => extern "C" fn ms_effect_reset_motion_disable(actor: u32) -> u32 {
        minor::ms_effect_reset_motion_disable(&mut mem(), actor)
    }
    /// `MsGetNextAP(chr)`.
    MsGetNextAP @ 0x0078_4e90 => extern "C" fn ms_get_next_ap(chr: i32) -> i32 {
        menu::ms_get_next_ap(&mut mem(), chr)
    }
    /// `MsGetSaveChrName(chr)`.
    MsGetSaveChrName @ 0x0078_4fb0 => extern "C" fn ms_get_save_chr_name(chr: u32) -> u32 {
        menu::ms_get_save_chr_name(&mem(), chr)
    }
    /// `MsMenuStatusNum(id, strings)`.
    MsMenuStatusNum @ 0x0079_0200 => extern "C" fn ms_menu_status_num(id: i32, strings: u32) -> u32 {
        menu::ms_menu_status_num(&mut mem(), id, strings)
    }
    /// `MsWeaponName(id, k, second, &icon)`.
    MsWeaponName @ 0x007a_0c50 => extern "C" fn ms_weapon_name(id: u32, k: u32, second: u32, icon: u32) -> u32 {
        menu::ms_weapon_name(&mut mem(), id, k, second, icon)
    }
    /// `MsMenuGetChrSkipCommand(i)`.
    MsMenuGetChrSkipCommand @ 0x0078_fb80 => extern "C" fn ms_menu_get_chr_skip_command(i: u32) -> u32 {
        menu::ms_menu_get_chr_skip_command(&mem(), i)
    }
    /// `MsGetCheckEnglish()`: the language from `TkGetLang` (menu).
    MsGetCheckEnglish @ 0x0078_4e60 => extern "C" fn ms_get_check_english() -> i32 {
        menu::ms_get_check_english(ffx_menu::tk::bridge::tk_get_lang(&mem()))
    }
    /// `MsGetBattleEndStatus()`.
    MsGetBattleEndStatus @ 0x0079_28d0 => extern "C" fn ms_get_battle_end_status() -> u32 {
        menu::ms_get_battle_end_status(&mem())
    }
    /// `MsCameraSetRoll(id, roll)` (a float as bits).
    MsCameraSetRoll @ 0x007b_fa80 => extern "C" fn ms_camera_set_roll(id: u32, roll: u32) {
        camera::ms_camera_set_roll(&mut mem(), id, roll);
    }
    /// `MsCameraSetScrDpt(id, depth)` (a float as bits).
    MsCameraSetScrDpt @ 0x007b_fbd0 => extern "C" fn ms_camera_set_scr_dpt(id: u32, depth: u32) {
        camera::ms_camera_set_scr_dpt(&mut mem(), id, depth);
    }
    /// `MsDebugGetCameraMode(side)`.
    MsDebugGetCameraMode @ 0x007c_fb20 => extern "C" fn ms_debug_get_camera_mode(side: i32) -> u32 {
        camera::ms_debug_get_camera_mode(&mem(), side)
    }
    /// `MsCopyCameraMatrix(out)`.
    MsCopyCameraMatrix @ 0x007c_0070 => extern "C" fn ms_copy_camera_matrix(out: u32) -> u32 {
        camera::ms_copy_camera_matrix(&mut mem(), out)
    }
    /// `MsCopyScreenMatrix(out)`.
    MsCopyScreenMatrix @ 0x007b_ba50 => extern "C" fn ms_copy_screen_matrix(out: u32) -> u32 {
        camera::ms_copy_screen_matrix(&mut mem(), out)
    }
    /// `MsSubPoint(out, a, b)`.
    MsSubPoint @ 0x007c_40b0 => extern "C" fn ms_sub_point(out: u32, a: u32, b: u32) {
        vector::ms_sub_point(&mut mem(), out, a, b);
    }
    /// `MsPointLen(a, b)`: on the x87 stack.
    MsPointLen @ 0x007c_2ec0 => extern "C" fn ms_point_len(a: u32, b: u32) -> f32 {
        vector::ms_point_len(&mut mem(), a, b)
    }
    /// `MsPointLen2D(a, b)`: on the x87 stack.
    MsPointLen2D @ 0x007c_3030 => extern "C" fn ms_point_len_2d(a: u32, b: u32) -> f32 {
        vector::ms_point_len_2d(&mut mem(), a, b)
    }
    /// `Battle_ChrMoveFlags(actor)`.
    Battle_ChrMoveFlags @ 0x007b_4a70 => extern "C" fn battle_chr_move_flags(actor: u32) -> u32 {
        body::battle_chr_move_flags(&mem(), actor)
    }
    /// `MsMenuCountStatus(i)`.
    MsMenuCountStatus @ 0x0078_fa80 => extern "C" fn ms_menu_count_status(i: u32) -> u32 {
        minor::ms_menu_count_status(&mem(), i)
    }
    /// `MsMenuDefCountStatus(i)`.
    MsMenuDefCountStatus @ 0x0078_faa0 => extern "C" fn ms_menu_def_count_status(i: u32) -> u32 {
        minor::ms_menu_def_count_status(&mem(), i)
    }
    /// `MsMenuHalfElement(i)`.
    MsMenuHalfElement @ 0x0078_fe40 => extern "C" fn ms_menu_half_element(i: u32) -> u32 {
        minor::ms_menu_half_element(&mem(), i)
    }
    /// `MsMenuBitStatus(i)`.
    MsMenuBitStatus @ 0x0078_f950 => extern "C" fn ms_menu_bit_status(i: u32) -> u32 {
        minor::ms_menu_bit_status(&mem(), i)
    }
    /// `MsGetSummonerNum(i)`.
    MsGetSummonerNum @ 0x0078_8e10 => extern "C" fn ms_get_summoner_num(i: u32) -> u32 {
        minor::ms_get_summoner_num(&mem(), i)
    }
    /// `MsMenuWeakElement(i)`.
    MsMenuWeakElement @ 0x007a_9a20 => extern "C" fn ms_menu_weak_element(i: u32) -> u32 {
        minor::ms_menu_weak_element(&mem(), i)
    }
    /// `MsMenuCheckChrLive(i)`.
    MsMenuCheckChrLive @ 0x0078_f970 => extern "C" fn ms_menu_check_chr_live(i: u32) -> u32 {
        minor::ms_menu_check_chr_live(&mem(), i)
    }
    /// `MsGetSummonEnergy(i)`.
    MsGetSummonEnergy @ 0x007b_2b00 => extern "C" fn ms_get_summon_energy(i: u32) -> u32 {
        minor::ms_get_summon_energy(&mem(), i)
    }
    /// `MsEffectGetTargetFlag(i)`.
    MsEffectGetTargetFlag @ 0x0078_7c10 => extern "C" fn ms_effect_get_target_flag(i: u32) -> i32 {
        minor::ms_effect_get_target_flag(&mem(), i)
    }
    /// `MsSetMotionAttackWait(i)`.
    MsSetMotionAttackWait @ 0x007a_b500 => extern "C" fn ms_set_motion_attack_wait(i: u32) -> u32 {
        minor::ms_set_motion_attack_wait(&mut mem(), i)
    }
    /// `MsSetMotionReturn(i)`.
    MsSetMotionReturn @ 0x007a_b520 => extern "C" fn ms_set_motion_return(i: u32) -> u32 {
        minor::ms_set_motion_return(&mut mem(), i)
    }
    /// `MsDefensePosOff(actor)`.
    MsDefensePosOff @ 0x007a_f880 => extern "C" fn ms_defense_pos_off(actor: u32) -> u32 {
        minor::ms_defense_pos_off(&mut mem(), actor)
    }
    /// `MsPushBtlPosAll()`.
    MsPushBtlPosAll @ 0x007a_c680 => extern "C" fn ms_push_btl_pos_all() {
        minor::ms_push_btl_pos_all(&mut mem());
    }
    /// `MsInitParamData()`.
    MsInitParamData @ 0x0078_5960 => extern "C" fn ms_init_param_data() {
        minor::ms_init_param_data(&mut mem());
    }
    /// `MsInitMimic()`.
    MsInitMimic @ 0x0079_9ab0 => extern "C" fn ms_init_mimic() {
        minor::ms_init_mimic(&mut mem());
    }
    /// `MsInitChrAbilityMap()`.
    MsInitChrAbilityMap @ 0x0079_8820 => extern "C" fn ms_init_chr_ability_map() {
        minor::ms_init_chr_ability_map(&mut mem());
    }
    /// `MsBattleMemInit()`.
    MsBattleMemInit @ 0x0078_1d00 => extern "C" fn ms_battle_mem_init() {
        minor::ms_battle_mem_init(&mut mem());
    }
    /// `frnd()`: a float in [1, 2) on the x87 stack.
    frnd @ 0x0079_8930 => extern "C" fn frnd() -> f32 {
        minor::frnd(&mut mem())
    }
    /// `MsGetStatusThreat(i)`.
    MsGetStatusThreat @ 0x0079_afc0 => extern "C" fn ms_get_status_threat(i: u32) -> u32 {
        minor::ms_get_status_threat(&mem(), i)
    }
    /// `MsCheckChrHitMotionAll()`.
    MsCheckChrHitMotionAll @ 0x007a_f2b0 => extern "C" fn ms_check_chr_hit_motion_all() -> i32 {
        minor::ms_check_chr_hit_motion_all(&mem())
    }
    /// `MsSetPartyRelifeTime()`.
    MsSetPartyRelifeTime @ 0x0078_e0a0 => extern "C" fn ms_set_party_relife_time() {
        minor::ms_set_party_relife_time(&mut mem());
    }
    /// `MsCheckPlyBtlPosAll()`.
    MsCheckPlyBtlPosAll @ 0x0079_2170 => extern "C" fn ms_check_ply_btl_pos_all() -> u32 {
        minor::ms_check_ply_btl_pos_all(&mut mem())
    }
    /// `MsCommandStartChrStatusAll()`.
    MsCommandStartChrStatusAll @ 0x0079_3ef0 => extern "C" fn ms_command_start_chr_status_all() {
        minor::ms_command_start_chr_status_all(&mut mem());
    }
    /// `MsEffectCheck()`.
    MsEffectCheck @ 0x0078_7810 => extern "C" fn ms_effect_check() -> u32 {
        minor::ms_effect_check(&mem())
    }
    /// `MsEffectMotionLevel(id)`.
    MsEffectMotionLevel @ 0x0078_7ca0 => extern "C" fn ms_effect_motion_level(id: u32) -> i32 {
        minor::ms_effect_motion_level(&mem(), id)
    }
    /// `MsMenuStatusNumSize()`.
    MsMenuStatusNumSize @ 0x0079_0220 => extern "C" fn ms_menu_status_num_size() -> i32 {
        minor::ms_menu_status_num_size(&mem())
    }
    /// `MsProvOff(chr)`.
    MsProvOff @ 0x0078_d4e0 => extern "C" fn ms_prov_off(chr: u32) {
        minor::ms_prov_off(&mut mem(), chr);
    }
    /// `MsSaveEncountCheck()`.
    MsSaveEncountCheck @ 0x0078_5b10 => extern "C" fn ms_save_encount_check() -> u32 {
        minor::ms_save_encount_check(&mut mem())
    }
    /// `MsImportantName(id)`.
    MsImportantName @ 0x0079_0860 => extern "C" fn ms_important_name(id: u32) -> u32 {
        minor::ms_important_name(&mem(), id)
    }
    /// `MsSetSaveConfigEnglish(on)`: the language from `TkGetLang` (menu).
    MsSetSaveConfigEnglish @ 0x0078_5ed0 => extern "C" fn ms_set_save_config_english(on: u32) {
        let m = &mut mem();
        let lang = ffx_menu::tk::bridge::tk_get_lang(m);
        minor::ms_set_save_config_english(m, on, lang);
    }
    /// `MsBtlListEncount(i)`.
    MsBtlListEncount @ 0x0079_d170 => extern "C" fn ms_btl_list_encount(i: i32) -> u32 {
        minor::ms_btl_list_encount(&mem(), i)
    }
    /// `MsCheckInputCueExe(chr)`.
    MsCheckInputCueExe @ 0x007b_1fd0 => extern "C" fn ms_check_input_cue_exe(chr: u32) -> u32 {
        cue::ms_check_input_cue_exe(&mem(), chr)
    }
    /// `MsCheckCommandCueExe(chr)`.
    MsCheckCommandCueExe @ 0x007b_07d0 => extern "C" fn ms_check_command_cue_exe(chr: u32) -> u32 {
        cue::ms_check_command_cue_exe(&mem(), chr)
    }
    /// `MsCheckInputCuePos(chr)`.
    MsCheckInputCuePos @ 0x007b_2020 => extern "C" fn ms_check_input_cue_pos(chr: u32) {
        cue::ms_check_input_cue_pos(&mut mem(), chr);
    }
    /// `MsCheckTurnInputCue(chr)`.
    MsCheckTurnInputCue @ 0x007b_2060 => extern "C" fn ms_check_turn_input_cue(chr: u32) -> u32 {
        cue::ms_check_turn_input_cue(&mem(), chr)
    }
    /// `MsExeInputCue(chr, on)`.
    MsExeInputCue @ 0x007b_2290 => extern "C" fn ms_exe_input_cue(chr: u32, on: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        cue::ms_exe_input_cue(&mut mem(), chr, on as u8)
    }
    /// `MsExeComandCue(chr, on)`.
    MsExeComandCue @ 0x007b_0960 => extern "C" fn ms_exe_comand_cue(chr: u32, on: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        cue::ms_exe_comand_cue(&mut mem(), chr, on as u8)
    }
    /// `MsRegistInputCue(chr, prio, turn, flag)`.
    MsRegistInputCue @ 0x007b_2430 => extern "C" fn ms_regist_input_cue(chr: u32, prio: u32, turn: u32, flag: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // byte arguments
        cue::ms_regist_input_cue(&mut mem(), chr, prio as u8, turn as u8, flag as u8)
    }
    /// `MsInsertInputCue(chr, prio, turn, flag, after, on)`.
    MsInsertInputCue @ 0x007b_2300 => extern "C" fn ms_insert_input_cue(chr: u32, prio: u32, turn: u32, flag: u32, after: u32, on: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // byte arguments
        cue::ms_insert_input_cue(&mut mem(), chr, prio, turn as u8, flag, after, on as u8)
    }
    /// `MsDeleteCommandCue(chr, slot, keep)`.
    MsDeleteCommandCue @ 0x007b_0860 => extern "C" fn ms_delete_command_cue(chr: u32, slot: u32, keep: u32) -> i32 {
        cue::ms_delete_command_cue(&mut mem(), chr, slot, keep)
    }
    /// `MsGetDamageResult(chr, target, &damage, &kind, &more)`.
    MsGetDamageResult @ 0x0079_a920 => extern "C" fn ms_get_damage_result(chr: u32, target: u32, damage: u32, kind: u32, more: u32) -> i32 {
        cue::ms_get_damage_result(&mut mem(), chr, target, damage, kind, more)
    }
    /// `MsGetDamageInfo(chr, target, &a, &b, &c)`.
    MsGetDamageInfo @ 0x0079_a680 => extern "C" fn ms_get_damage_info(chr: u32, target: u32, a: u32, b: u32, c: u32) -> i32 {
        cue::ms_get_damage_info(&mut mem(), chr, target, [a, b, c])
    }
    /// `MsGetTargetList(chr, &n)`.
    MsGetTargetList @ 0x007b_7090 => extern "C" fn ms_get_target_list(chr: u32, n: u32) -> u32 {
        cue::ms_get_target_list(&mut mem(), chr, n)
    }
    /// `MsSetFirstCTB(mode)`.
    MsSetFirstCTB @ 0x0078_ded0 => extern "C" fn ms_set_first_ctb(mode: i32) -> i32 {
        turn::ms_set_first_ctb(&mut mem(), mode)
    }
    /// `MsUseItem(chr, command, n)`.
    MsUseItem @ 0x007b_0c20 => extern "C" fn ms_use_item(chr: u32, command: u32, n: i32) -> i32 {
        turn::ms_use_item(&mut mem(), chr, command, n)
    }
    /// `MsCheckStatus()`.
    MsCheckStatus @ 0x0079_a0e0 => extern "C" fn ms_check_status() {
        turn::ms_check_status(&mut mem());
    }
    /// `MsGetDamageStatus(chr, target, &added, &on, &added2, &removed, &off, &removed2)`.
    MsGetDamageStatus @ 0x0079_aa50 => extern "C" fn ms_get_damage_status(chr: u32, target: u32, p0: u32, p1: u32, p2: u32, p3: u32, p4: u32, p5: u32) -> u32 {
        turn::ms_get_damage_status(&mut mem(), chr, target, [p0, p1, p2, p3, p4, p5])
    }
    /// `MsApUp(chr, actor, ap, ret)`.
    MsApUp @ 0x0079_8a00 => extern "C" fn ms_ap_up(chr: u32, actor: u32, ap: u32, ret: u32) -> u32 {
        turn::ms_ap_up(&mut mem(), chr, actor, ap, ret)
    }
    /// `BattleRewards_AddItem(item, n, spoils)`.
    BattleRewards_AddItem @ 0x0079_8ac0 => extern "C" fn battle_rewards_add_item(item: u32, n: i32, spoils: u32) {
        turn::battle_rewards_add_item(&mut mem(), item, n, spoils);
    }
    /// `MsStatusEffectClear(i)`: `op_et_eff_off_signal` (vfx) for each.
    MsStatusEffectClear @ 0x0079_f110 => extern "C" fn ms_status_effect_clear(i: u32) -> u32 {
        flow::ms_status_effect_clear(&mut mem(), i, ffx_vfx::et::bridge::op_et_eff_off_signal)
    }
    /// `MsResetBindEffect(i)`: `op_et_bindeff_off_signal` (vfx).
    MsResetBindEffect @ 0x0078_8e60 => extern "C" fn ms_reset_bind_effect(i: u32) {
        flow::ms_reset_bind_effect(&mut mem(), i, ffx_vfx::et::bridge::op_et_bindeff_off_signal);
    }
    /// `MsResetAfterImage(actor)`: `op_et_weapon_afterimage_fade` (vfx).
    MsResetAfterImage @ 0x0078_8e30 => extern "C" fn ms_reset_after_image(actor: u32) {
        flow::ms_reset_after_image(&mut mem(), actor, ffx_vfx::et::bridge::op_et_weapon_afterimage_fade);
    }
    /// `MsEffectCheckMotion(i)`.
    MsEffectCheckMotion @ 0x0078_7840 => extern "C" fn ms_effect_check_motion(i: u32) -> i32 {
        flow::ms_effect_check_motion(&mut mem(), i)
    }
    /// `MsCheckStartEffect(chr, set)`.
    MsCheckStartEffect @ 0x007a_92c0 => extern "C" fn ms_check_start_effect(chr: u32, set: u32) -> u32 {
        flow::ms_check_start_effect(&mut mem(), chr, set)
    }
    /// `MsSetHeadTarget(chr, set)`.
    MsSetHeadTarget @ 0x007b_1e80 => extern "C" fn ms_set_head_target(chr: u32, set: u32) -> u32 {
        flow::ms_set_head_target(&mut mem(), chr, set)
    }
    /// `MsSaveAddInfoData(chr, kind)`.
    MsSaveAddInfoData @ 0x0078_5a00 => extern "C" fn ms_save_add_info_data(chr: i32, kind: u32) -> i32 {
        flow::ms_save_add_info_data(&mut mem(), chr, kind)
    }
    /// `MsLimitStatusProcess(_, actor, flags)`.
    MsLimitStatusProcess @ 0x0078_d270 => extern "C" fn ms_limit_status_process(_x: u32, actor: u32, flags: i32) -> u32 {
        flow::ms_limit_status_process(&mut mem(), actor, flags)
    }
    /// `MsMenuCheckLook()`.
    MsMenuCheckLook @ 0x0078_f9e0 => extern "C" fn ms_menu_check_look() -> i32 {
        flow::ms_menu_check_look(&mem())
    }
    /// `MsEffectCheckTarget(set)`.
    MsEffectCheckTarget @ 0x0078_7900 => extern "C" fn ms_effect_check_target(set: u32) -> u32 {
        flow::ms_effect_check_target(&mem(), set)
    }
    /// `MsReturnParty()`.
    MsReturnParty @ 0x007a_eec0 => extern "C" fn ms_return_party() {
        flow::ms_return_party(&mut mem());
    }
    /// `MsSetCameraMatrix(on, screen, camera, look_at, eye, roll)` (roll a float as bits).
    MsSetCameraMatrix @ 0x007c_06d0 => extern "C" fn ms_set_camera_matrix(on: u32, screen: u32, cam: u32, look_at: u32, eye: u32, roll: u32) -> u32 {
        camera::ms_set_camera_matrix(&mut mem(), on, screen, cam, look_at, eye, roll)
    }
    /// `MsSetChrImageDamageAttack`.
    MsSetChrImageDamageAttack @ 0x0078_d790 => extern "C" fn ms_set_chr_image_damage_attack(actor: u32) -> i32 {
        flow::ms_set_chr_image_damage_attack(&mut mem(), actor)
    }
    /// `MsSetChrHeight`.
    MsSetChrHeight @ 0x007a_b300 => extern "C" fn ms_set_chr_height(i: u32) {
        flow::ms_set_chr_height(&mut mem(), i);
    }
    /// `MsGetScreenData`.
    MsGetScreenData @ 0x007b_bd00 => extern "C" fn ms_get_screen_data(a: u32, b: u32, c: u32, d: u32) -> u32 {
        flow::ms_get_screen_data(&mut mem(), [a, b, c, d])
    }
    /// `MsBtlPosInit`.
    MsBtlPosInit @ 0x007c_93d0 => extern "C" fn ms_btl_pos_init() {
        flow::ms_btl_pos_init(&mut mem());
    }
    /// `MsAtelRegistInit`.
    MsAtelRegistInit @ 0x0079_7ae0 => extern "C" fn ms_atel_regist_init() {
        flow::ms_atel_regist_init(&mut mem());
    }
    /// `MsSetChrMotionInfo`.
    MsSetChrMotionInfo @ 0x007b_4a90 => extern "C" fn ms_set_chr_motion_info(i: u32, k: u32, value: u32) {
        flow::ms_set_chr_motion_info(&mut mem(), i, k, value);
    }
    /// `MsExchangeParty`.
    MsExchangeParty @ 0x007a_de10 => extern "C" fn ms_exchange_party() {
        flow::ms_exchange_party(&mut mem());
    }
    /// `MsChrSEfoot`.
    MsChrSEfoot @ 0x0079_fd20 => extern "C" fn ms_chr_se_foot(i: u32, v: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        flow::ms_chr_se_foot(&mut mem(), i, v as u8);
    }
    /// `MsSEP3Dinit`.
    MsSEP3Dinit @ 0x007a_0910 => extern "C" fn ms_sep3d_init() {
        flow::ms_sep3d_init(&mut mem());
    }
    /// `MsEffectLimitSync`.
    MsEffectLimitSync @ 0x007a_fde0 => extern "C" fn ms_effect_limit_sync(i: u32) -> i32 {
        flow::ms_effect_limit_sync(&mem(), i)
    }
    /// `MsMenuInitText`.
    MsMenuInitText @ 0x0078_fe60 => extern "C" fn ms_menu_init_text() {
        flow::ms_menu_init_text(&mut mem());
    }
    /// `MsAddYoujinbouCom`.
    MsAddYoujinbouCom @ 0x007b_2950 => extern "C" fn ms_add_youjinbou_com(n: i32) -> i32 {
        flow::ms_add_youjinbou_com(&mut mem(), n)
    }
    /// `MsMagicEffectSet`.
    MsMagicEffectSet @ 0x0079_e890 => extern "C" fn ms_magic_effect_set(chr: u32, actor: u32) {
        flow::ms_magic_effect_set(&mut mem(), chr, actor);
    }
    /// `MsMenuCheckChrLook`.
    MsMenuCheckChrLook @ 0x0078_f990 => extern "C" fn ms_menu_check_chr_look(i: u32) -> u32 {
        flow::ms_menu_check_chr_look(&mem(), i)
    }
    /// `MsSetChrMoveFlagAll`.
    MsSetChrMoveFlagAll @ 0x0078_8e90 => extern "C" fn ms_set_chr_move_flag_all(on: u32) {
        flow::ms_set_chr_move_flag_all(&mut mem(), on);
    }
    /// `MsGetFirstAttack`.
    MsGetFirstAttack @ 0x0078_d090 => extern "C" fn ms_get_first_attack(chance: i32) -> u32 {
        flow::ms_get_first_attack(&mut mem(), chance)
    }
    /// `MsDefenseProcess`.
    MsDefenseProcess @ 0x007a_f8a0 => extern "C" fn ms_defense_process() -> u32 {
        flow::ms_defense_process(&mut mem())
    }
    /// `MsBtlChrInit`.
    MsBtlChrInit @ 0x0079_35b0 => extern "C" fn ms_btl_chr_init() {
        flow::ms_btl_chr_init(&mut mem());
    }
    /// `MsLimitTidusLearn`.
    MsLimitTidusLearn @ 0x007b_0cd0 => extern "C" fn ms_limit_tidus_learn(chr: i32) -> u32 {
        flow::ms_limit_tidus_learn(&mut mem(), chr)
    }
    /// `MsBattleCameraFree`.
    MsBattleCameraFree @ 0x007b_a950 => extern "C" fn ms_battle_camera_free() {
        flow::ms_battle_camera_free(&mut mem());
    }
    /// `MsBindOffCamera(w)`: `AtelGetCameraWorkAdrs` (script) for the worker.
    MsBindOffCamera @ 0x007b_a9c0 => extern "C" fn ms_bind_off_camera(w: u32) {
        flow::ms_bind_off_camera(&mut mem(), w, |_, w| ffx_script::vm::bridge::atel_get_camera_work_adrs(w));
    }
    /// `MsCheckStatusBeforeActionDamage`.
    MsCheckStatusBeforeActionDamage @ 0x007a_f770 => extern "C" fn ms_check_status_before_action_damage(actor: u32) -> u32 {
        flow::ms_check_status_before_action_damage(&mem(), actor)
    }
    /// `MsLimitCommandProcess(chr, actor, command, k)`.
    MsLimitCommandProcess @ 0x007b_03e0 => extern "C" fn ms_limit_command_process(_chr: u32, actor: u32, command: u32, k: u32) -> u32 {
        flow::ms_limit_command_process(&mut mem(), actor, command, k)
    }
    /// `MsSetLimitParty(chr, actor)`.
    MsSetLimitParty @ 0x007b_1aa0 => extern "C" fn ms_set_limit_party(chr: u32, actor: u32) {
        flow::ms_set_limit_party(&mut mem(), chr, actor);
    }
    /// `MsSetChrTransparent(i, alpha, frames)` (alpha a float as bits).
    MsSetChrTransparent @ 0x0079_6480 => extern "C" fn ms_set_chr_transparent(i: u32, alpha: u32, frames: i32) -> u32 {
        flow::ms_set_chr_transparent(&mut mem(), i, alpha, frames)
    }
    /// `MsEffectSetMotionFlag(i)`.
    MsEffectSetMotionFlag @ 0x0078_83f0 => extern "C" fn ms_effect_set_motion_flag(i: u32) {
        flow::ms_effect_set_motion_flag(&mut mem(), i);
    }
    /// `MsGetTargetRandomList2(i, &n)`.
    MsGetTargetRandomList2 @ 0x007b_7280 => extern "C" fn ms_get_target_random_list2(i: u32, n: u32) -> u32 {
        flow::ms_get_target_random_list2(&mut mem(), i, n)
    }
    /// `MsAtelCamFree()`: `AtelRemoveCtrlWork` (script).
    MsAtelCamFree @ 0x0079_74e0 => extern "C" fn ms_atel_cam_free() {
        flow::ms_atel_cam_free(&mut mem(), ffx_script::vm::bridge::atel_remove_ctrl_work);
    }
    /// `MsGetMagicEffNum(chr, id, command)`: `TkGetLang` (menu).
    MsGetMagicEffNum @ 0x0079_d2f0 => extern "C" fn ms_get_magic_eff_num(chr: i32, id: u32, command: u32) -> u32 {
        flow::ms_get_magic_eff_num(&mut mem(), chr, id, command, |m| ffx_menu::tk::bridge::tk_get_lang(m))
    }
    /// `MsGetSummonerPos(actor, chr, k, vec)`: the CRT's `_CIcos` and
    /// `_CIsin` through their import slots.
    MsGetSummonerPos @ 0x007a_c2b0 => extern "C" fn ms_get_summoner_pos(actor: u32, chr: u32, k: u32, vec: u32) {
        pos::ms_get_summoner_pos(&mut mem(), &crt_trig, actor, chr, k, vec);
    }
    /// `MsGetChrStatInfo(chr, kind, selector, out)`.
    MsGetChrStatInfo @ 0x007b_2dc0 => extern "C" fn ms_get_chr_stat_info(chr: u32, kind: u32, selector: u32, out: u32) -> u32 {
        ffx_battle::stat::bridge::ms_get_chr_stat_info(&mut mem(), &crt_trig, chr, kind, selector, out)
    }
    /// `MsActTargetSearch(chr, selector, kind, _, rng, mode, resolved, pick)`.
    MsActTargetSearch @ 0x007a_cd40 => extern "C" fn ms_act_target_search(chr: u32, selector: u32, kind: u32, _unused: u32, rng: u32, mode: u32, resolved: u32, pick: u32) -> u32 {
        ffx_battle::stat::bridge::ms_act_target_search(&mut mem(), &crt_trig, chr, selector, kind, rng, mode, resolved, pick)
    }
    /// `MsCalcCommandTarget(user, hits, counts, command, event, &targets, &total)`.
    MsCalcCommandTarget @ 0x0078_ba70 => extern "C" fn ms_calc_command_target(user: u32, hits: u32, counts: u32, command: u32, event: u32, targets: u32, total: u32) -> u32 {
        ffx_battle::stat::bridge::ms_calc_command_target(&mut mem(), &crt_trig, user, hits, counts, command, event, targets, total)
    }
    /// `MsCalcCommand(cue, k)`: `TOGetFFXLang` and the achievement unlock
    /// (main's) through the originals.
    MsCalcCommand @ 0x0078_92e0 => extern "C" fn ms_calc_command(cue: u32, k: u32) -> u32 {
        ffx_battle::stat::bridge::ms_calc_command(&mut mem(), &crt_trig, cue, k, |_| call0(TO_GET_FFX_LANG), |_, n| {
            call1(ACHIEVEMENT_UNLOCK, n);
        })
    }
    /// `MsVectorRect2Polar(a, b, &yaw, &pitch, &len)`: `_CIatan` through
    /// its import slot.
    MsVectorRect2Polar @ 0x007c_4990 => extern "C" fn ms_vector_rect2_polar(a: u32, b: u32, yaw: u32, pitch: u32, len: u32) {
        vector::ms_vector_rect2_polar(&mut mem(), &crt_trig, a, b, yaw, pitch, len);
    }
    /// `MsVectorPolar2Rect(out, base, yaw, pitch, r)`.
    MsVectorPolar2Rect @ 0x007c_47c0 => extern "C" fn ms_vector_polar2_rect(out: u32, base: u32, yaw: u32, pitch: u32, r0: u32, r1: u32, r2: u32, r3: u32) {
        vector::ms_vector_polar2_rect(&mut mem(), &crt_trig, out, base, yaw, pitch, [r0, r1, r2, r3]);
    }
    /// `MsCameraMatrix(out, eye, target, up, roll)`: `_CIcos` and `_CIsin`
    /// through their import slots; `eax` as the original leaves it.
    MsCameraMatrix @ 0x007c_1be0 => extern "C" fn ms_camera_matrix(out: u32, eye: u32, target: u32, up: u32, roll: u32) -> u32 {
        view::ms_camera_matrix(&mut mem(), &crt_trig, out, eye, target, up, roll)
    }
    /// `MsScreenMatrix(out, screen)`.
    MsScreenMatrix @ 0x007c_3d60 => extern "C" fn ms_screen_matrix(out: u32, screen: u32) -> u32 {
        view::ms_screen_matrix(&mut mem(), out, screen)
    }
    /// `MsCameraCheckShake(cam, side)`.
    MsCameraCheckShake @ 0x007c_4f10 => extern "C" fn ms_camera_check_shake(cam: u32, side: u32) -> i32 {
        shake::ms_camera_check_shake(&mem(), cam, side)
    }
    /// `MsScreenCheckShake(i)`.
    MsScreenCheckShake @ 0x007c_5680 => extern "C" fn ms_screen_check_shake(i: u32) -> i32 {
        shake::ms_screen_check_shake(&mem(), i)
    }
    /// `MsCameraResetShake(id, side)`; `eax` as the original leaves it.
    MsCameraResetShake @ 0x007c_4f50 => extern "C" fn ms_camera_reset_shake(id: u32, side: u32) -> u32 {
        shake::ms_camera_reset_shake(&mut mem(), id, side)
    }
    /// `MsCameraShake(id, side, which, kind, value, length, amplitude, range)`;
    /// `eax` as the original leaves it.
    MsCameraShake @ 0x007c_4fc0 => extern "C" fn ms_camera_shake(id: u32, side: u32, which: u32, kind: u32, value: u32, length: u32, amplitude: u32, range: u32) -> u32 {
        let start = shake::Start { kind: kind.to_le_bytes()[0], value, length: u16::from_le_bytes([length.to_le_bytes()[0], length.to_le_bytes()[1]]), amplitude: u16::from_le_bytes([amplitude.to_le_bytes()[0], amplitude.to_le_bytes()[1]]), range: range.to_le_bytes()[0] };
        shake::ms_camera_shake(&mut mem(), id, side, which, start)
    }
    /// `MsScreenShake(i, which, kind, value, length, amplitude, range)`.
    MsScreenShake @ 0x007c_56f0 => extern "C" fn ms_screen_shake(i: u32, which: u32, kind: u32, value: u32, length: u32, amplitude: u32, range: u32) {
        let start = shake::Start { kind: kind.to_le_bytes()[0], value, length: u16::from_le_bytes([length.to_le_bytes()[0], length.to_le_bytes()[1]]), amplitude: u16::from_le_bytes([amplitude.to_le_bytes()[0], amplitude.to_le_bytes()[1]]), range: range.to_le_bytes()[0] };
        shake::ms_screen_shake(&mut mem(), i, which, start);
    }
    /// `MsScreenShake2(i, which, kind, steps, length, speed, range, pull, fade)`.
    MsScreenShake2 @ 0x007c_57b0 => extern "C" fn ms_screen_shake2(i: u32, which: u32, kind: u32, steps: u32, length: u32, speed: u32, range: i32, pull: u32, fade: u32) {
        shake::ms_screen_shake2(&mut mem(), i, which, kind.to_le_bytes()[0], steps.to_le_bytes()[0], length, speed, range, pull, fade.to_le_bytes()[0]);
    }
    /// `MsCameraShakeProcess(shake, angle, out)`: `_CIcos`/`_CIsin` through
    /// their import slots on a double.
    MsCameraShakeProcess @ 0x007c_50f0 => extern "C" fn ms_camera_shake_process(rec: u32, angle: u32, out: u32) {
        shake::ms_camera_shake_process(&mut mem(), &crt_trig_f64, rec, angle, out);
    }
    /// `MsScreenLinkSet(i, k, v)`.
    MsScreenLinkSet @ 0x007b_bdd0 => extern "C" fn ms_screen_link_set(i: u32, k: u32, v: u32) {
        screen::ms_screen_link_set(&mut mem(), i, k, v.to_le_bytes()[0]);
    }
    /// `MsScreenOffsetID(id, x, y)`; `eax` as the original leaves it.
    MsScreenOffsetID @ 0x007b_c0e0 => extern "C" fn ms_screen_offset_id(id: u32, x: u32, y: u32) -> u32 {
        screen::ms_screen_offset_id(&mut mem(), id, x, y)
    }
    /// `MsScreenMoveCheck(i, side)`.
    MsScreenMoveCheck @ 0x007b_bfc0 => extern "C" fn ms_screen_move_check(i: u32, side: u32) -> u32 {
        screen::ms_screen_move_check(&mem(), i, side)
    }
    /// `MsScreenMoveReset(i, side)`.
    MsScreenMoveReset @ 0x007b_c010 => extern "C" fn ms_screen_move_reset(i: u32, side: u32) {
        screen::ms_screen_move_reset(&mut mem(), i, side);
    }
    /// `MsScreenMove(i, side, total, kind)`.
    MsScreenMove @ 0x007b_bdf0 => extern "C" fn ms_screen_move(i: u32, side: u32, total: u32, kind: u32) {
        screen::ms_screen_move(&mut mem(), i, side, u16::from_le_bytes([total.to_le_bytes()[0], total.to_le_bytes()[1]]), u16::from_le_bytes([kind.to_le_bytes()[0], kind.to_le_bytes()[1]]));
    }
    /// `MsScreenMoveAcc(i, side, accel, cruise, brake)`.
    MsScreenMoveAcc @ 0x007b_beb0 => extern "C" fn ms_screen_move_acc(i: u32, side: u32, accel: u32, cruise: u32, brake: u32) {
        screen::ms_screen_move_acc(&mut mem(), i, side, u16::from_le_bytes([accel.to_le_bytes()[0], accel.to_le_bytes()[1]]), u16::from_le_bytes([cruise.to_le_bytes()[0], cruise.to_le_bytes()[1]]), u16::from_le_bytes([brake.to_le_bytes()[0], brake.to_le_bytes()[1]]));
    }
    /// `MsCameraResetFilter(id, side)`.
    MsCameraResetFilter @ 0x007b_ed10 => extern "C" fn ms_camera_reset_filter(id: u32, side: u32) {
        camera::ms_camera_reset_filter(&mut mem(), id, side);
    }
    /// `MsCameraMoveStat(id, side, v)`.
    MsCameraMoveStat @ 0x007b_df90 => extern "C" fn ms_camera_move_stat(id: u32, side: u32, v: u32) {
        camera::ms_camera_move_stat(&mut mem(), id, side, v.to_le_bytes()[0]);
    }
    /// `MsCameraMoveDivide(id, side, v0, v1, v2, v3)`.
    MsCameraMoveDivide @ 0x007b_dde0 => extern "C" fn ms_camera_move_divide(id: u32, side: u32, v0: u32, v1: u32, v2: u32, v3: u32) {
        camera::ms_camera_move_divide(&mut mem(), id, side, [v0, v1, v2, v3]);
    }
    /// `MsCameraSetSplineFilter(id, side, value, scale, kind)`.
    MsCameraSetSplineFilter @ 0x007b_fe70 => extern "C" fn ms_camera_set_spline_filter(id: u32, side: u32, value: u32, scale: u32, kind: u32) {
        camera::ms_camera_set_spline_filter(&mut mem(), id, side, value, scale, kind);
    }
    /// `MsCameraResetDirVector(id, side)`.
    MsCameraResetDirVector @ 0x007b_ec90 => extern "C" fn ms_camera_reset_dir_vector(id: u32, side: u32) {
        camera::ms_camera_reset_dir_vector(&mut mem(), id, side);
    }
    /// `MsCameraSetFocus(id, side, v)`.
    MsCameraSetFocus @ 0x007b_f280 => extern "C" fn ms_camera_set_focus(id: u32, side: u32, v: u32) {
        camera::ms_camera_set_focus(&mut mem(), id, side, v);
    }
    /// `MsCameraReset(id, side)`.
    MsCameraReset @ 0x007b_eae0 => extern "C" fn ms_camera_reset(id: u32, side: u32) -> u32 {
        camera::ms_camera_reset(&mut mem(), id, side)
    }
    /// `MsCameraSetStartPos(cam, side, mv)`: the CRT's trigonometry
    /// through its import slots.
    MsCameraSetStartPos @ 0x007b_ff10 => extern "C" fn ms_camera_set_start_pos(cam: u32, side: u32, mv: u32) -> u32 {
        camera::ms_camera_set_start_pos(&mut mem(), &crt_trig, cam, side, mv)
    }
    /// `MsCameraGetYaw()`: the yaw through a word of this stack (the
    /// original's local), the CRT's `_CIatan` through its import slot.
    MsCameraGetYaw @ 0x007b_cf20 => extern "C" fn ms_camera_get_yaw() -> f32 {
        let mut local = super::StackFrame::<1>::new();
        let at = local.at();
        camera::ms_camera_get_yaw(&mut mem(), &crt_trig, at)
    }
    /// `MsCameraFocusWeights_007c0ae0(side, mv)`.
    MsCameraFocusWeights_007c0ae0 @ 0x007c_0ae0 => extern "C" fn ms_camera_focus_weights(side: u32, mv: u32) {
        camera::ms_camera_focus_weights(&mut mem(), side, mv);
    }
    /// `MsScaleAboutPoint_007c4d40(v, origin, scale)`.
    MsScaleAboutPoint_007c4d40 @ 0x007c_4d40 => extern "C" fn ms_scale_about_point(v: u32, origin: u32, scale: u32) -> u32 {
        vector::ms_scale_about_point(&mut mem(), v, origin, scale)
    }
    /// `MsVectorNormal2(out, a, b, s)`.
    MsVectorNormal2 @ 0x007c_45e0 => extern "C" fn ms_vector_normal2(out: u32, a: u32, b: u32, s0: u32, s1: u32, s2: u32, s3: u32) {
        vector::ms_vector_normal2(&mut mem(), out, a, b, [s0, s1, s2, s3]);
    }
    /// `MsGetChrBottomPos(i, vec)`.
    MsGetChrBottomPos @ 0x0079_4830 => extern "C" fn ms_get_chr_bottom_pos(i: u32, vec: u32) -> i32 {
        target::ms_get_chr_bottom_pos(&mut mem(), &crt_trig, i, vec)
    }
    /// `MsGetChrCameraPos(i, vec)`.
    MsGetChrCameraPos @ 0x0079_4ab0 => extern "C" fn ms_get_chr_camera_pos(i: u32, vec: u32) -> i32 {
        target::ms_get_chr_camera_pos(&mut mem(), &crt_trig, i, vec)
    }
    /// `MsGetChrTargetDir(i, dir)`: on the x87 stack.
    MsGetChrTargetDir @ 0x0079_58c0 => extern "C" fn ms_get_chr_target_dir(i: u32, dir: u32) -> f32 {
        target::ms_get_chr_target_dir(&mem(), i, dir)
    }
    /// `MsTurnMoveVector(i, actor, vec, angle)`.
    MsTurnMoveVector @ 0x007a_b190 => extern "C" fn ms_turn_move_vector(i: u32, actor: u32, vec: u32, angle: u32) {
        let mut m = mem();
        target::ms_turn_move_vector(&mut target::Dm::new(&mut m), &host(), i, actor, vec, angle);
    }
    /// `MsGetChrInfoOne(i, kind, k, dir, out)`.
    MsGetChrInfoOne @ 0x0079_4060 => extern "C" fn ms_get_chr_info_one(i: u32, kind: u32, k: u32, dir: u32, out: u32) -> u32 {
        let mut m = mem();
        target::ms_get_chr_info_one(&mut target::Dm::new(&mut m), &host(), i, kind, k, dir, out)
    }
    /// `MsGetChrInfo(kind, bits, k, out)`.
    MsGetChrInfo @ 0x0079_5170 => extern "C" fn ms_get_chr_info(kind: u32, bits: u32, k: u32, out: u32) -> u32 {
        let mut m = mem();
        target::ms_get_chr_info(&mut target::Dm::new(&mut m), &host(), kind, bits, k, out, None)
    }
    /// `MsAtelChrInfo(selector, kind, out)`.
    MsAtelChrInfo @ 0x0079_75c0 => extern "C" fn ms_atel_chr_info(selector: u32, kind: u32, out: u32) -> u32 {
        let mut m = mem();
        target::ms_atel_chr_info(&mut target::Dm::new(&mut m), &host(), selector, kind, out)
    }
    /// `MsAtelChrInfoOne(i, kind, acc)`.
    MsAtelChrInfoOne @ 0x0079_84c0 => extern "C" fn ms_atel_chr_info_one(i: u32, kind: u32, acc: u32) {
        let mut m = mem();
        target::ms_atel_chr_info_one(&mut target::Dm::new(&mut m), &host(), i, kind, acc);
    }
    /// `MsGetChrTargetInfo(i, kind, &pos, &width, &height)`.
    MsGetChrTargetInfo @ 0x007a_a050 => extern "C" fn ms_get_chr_target_info(i: u32, kind: u32, pos: u32, width: u32, height: u32) -> i32 {
        let mut m = mem();
        target::ms_get_chr_target_info(&mut target::Dm::new(&mut m), &host(), i, kind, pos, width, height)
    }
    /// `MsGetMoveTargetPos(i, actor, kind, out, &done)`: on the x87 stack.
    MsGetMoveTargetPos @ 0x007a_9ac0 => extern "C" fn ms_get_move_target_pos(i: u32, actor: u32, kind: u32, out: u32, done: u32) -> f32 {
        let mut m = mem();
        f32::from_bits(target::ms_get_move_target_pos(&mut target::Dm::new(&mut m), &host(), i, actor, kind, out, done, None))
    }
    /// `MsGetChrMoveAccelPos(actor, n, speed, target, out)`.
    MsGetChrMoveAccelPos @ 0x007a_ac40 => extern "C" fn ms_get_chr_move_accel_pos(actor: u32, n: u32, speed: u32, to: u32, out: u32) -> u32 {
        let mut m = mem();
        target::ms_get_chr_move_accel_pos(&mut target::Dm::new(&mut m), &crt_trig, actor, n, speed, to, out, None)
    }
    /// `MsGetChrMovePos(i, out)`.
    MsGetChrMovePos @ 0x007a_93e0 => extern "C" fn ms_get_chr_move_pos(i: u32, out: u32) {
        let mut m = mem();
        target::ms_get_chr_move_pos(&mut target::Dm::new(&mut m), &host(), i, out, None);
    }
    /// `MsSetRikkuLimit(chr, actor, cue)`.
    MsSetRikkuLimit @ 0x007b_0520 => extern "C" fn ms_set_rikku_limit(chr: u32, actor: u32, cue: u32) -> u32 {
        let mut m = mem();
        target::ms_set_rikku_limit(&mut target::Dm::new(&mut m), &host(), chr, actor, cue)
    }
    /// `MsSetTargetDir(i, targets)`.
    MsSetTargetDir @ 0x0079_69f0 => extern "C" fn ms_set_target_dir(i: u32, targets: u32) {
        let mut m = mem();
        target::ms_set_target_dir(&mut target::Dm::new(&mut m), &host(), i, targets);
    }
    /// `MsGetChrCommandFlag(chr, id)`.
    MsGetChrCommandFlag @ 0x0079_ad30 => extern "C" fn ms_get_chr_command_flag(chr: u32, id: u32) -> u32 {
        ffx_battle::ability::bridge::ms_get_chr_command_flag(&mem(), chr, id)
    }
    /// `MsSetRamChrCommand(chr, id, on)`.
    MsSetRamChrCommand @ 0x0079_c070 => extern "C" fn ms_set_chr_command_flag(chr: u32, id: u32, on: u32) {
        ffx_battle::ability::bridge::ms_set_chr_command_flag(&mut mem(), chr, id, on);
    }
    /// `MsSortCommandList(list, keys, n)`.
    MsSortCommandList @ 0x007a_d970 => extern "C" fn ms_sort_command_list(list: u32, keys: u32, n: i32) {
        ffx_battle::ability::bridge::ms_sort_command_list(&mut mem(), list, keys, n);
    }
    /// `MsSortCommand(lists)`.
    MsSortCommand @ 0x007a_efb0 => extern "C" fn ms_sort_command(lists: u32) {
        ffx_battle::ability::bridge::ms_sort_command(&mut mem(), lists);
    }
    /// `MsGetBattleList(kind, &count)`.
    MsGetBattleList @ 0x007b_6c50 => extern "C" fn ms_get_battle_list(kind: i32, count: u32) -> u32 {
        ffx_battle::ability::bridge::ms_get_battle_list(&mut mem(), kind, count)
    }
    /// `MsSetRamChrAbility(chr, actor)`.
    MsSetRamChrAbility @ 0x0079_bb50 => extern "C" fn ms_set_ram_chr_ability(chr: u32, actor: u32) {
        ffx_battle::ability::bridge::ms_set_ram_chr_ability(&mut mem(), chr, actor);
    }
    /// `MsSetSaveCommand(chr, id, on)`.
    MsSetSaveCommand @ 0x0078_5c50 => extern "C" fn ms_set_save_command(chr: u32, id: u32, on: u32) -> u32 {
        ffx_battle::ability::bridge::ms_set_save_command(&mut mem(), chr, id, on)
    }
    /// `MsGetComData(id, &name)`.
    MsGetComData @ 0x0079_a4b0 => extern "C" fn ms_get_com_data(id: u32, name: u32) -> u32 {
        ffx_battle::stat::bridge::ms_get_com_data(&mut mem(), id, name)
    }
    /// `MsGetRandomGroupTarget(bits)`.
    MsGetRandomGroupTarget @ 0x0079_9910 => extern "C" fn ms_get_random_group_target(bits: u32) -> u32 {
        ffx_battle::stat::bridge::ms_get_random_group_target(&mut mem(), bits)
    }
    /// `MsGetCommandTarget(chr, id, side, event)`.
    MsGetCommandTarget @ 0x0079_96d0 => extern "C" fn ms_get_command_target(chr: u32, id: u32, side: i32, event: u32) -> u32 {
        ffx_battle::stat::bridge::ms_get_command_target(&mut mem(), &crt_trig, chr, id, side, event)
    }
    /// `MsGetSaveWeapon(id, &name)`.
    MsGetSaveWeapon @ 0x007a_bbd0 => extern "C" fn ms_get_save_weapon(id: u32, name: u32) -> u32 {
        save::ms_get_save_weapon(&mut mem(), id, name)
    }
    /// `MsGetSaveWeaponName(id)`.
    MsGetSaveWeaponName @ 0x007a_bdf0 => extern "C" fn ms_get_save_weapon_name(id: u32) -> u32 {
        save::ms_get_save_weapon_name(&mut mem(), id)
    }
    /// `MsGetChrBtlPos(i, vec)`.
    MsGetChrBtlPos @ 0x0079_48e0 => extern "C" fn ms_get_chr_btl_pos(i: u32, vec: u32) -> i32 {
        pos::ms_get_chr_btl_pos(&mut mem(), &crt_trig, i, vec)
    }
    /// `MsGetChrPos(i, vec)`.
    MsGetChrPos @ 0x0079_56e0 => extern "C" fn ms_get_chr_pos(i: u32, vec: u32) -> i32 {
        pos::ms_get_chr_pos(&mut mem(), &crt_trig, i, vec)
    }
    /// `msGetChrPos(actor, vec)`.
    msGetChrPos @ 0x0079_6ea0 => extern "C" fn ms_get_chr_pos_of(actor: u32, vec: u32) {
        pos::ms_get_chr_pos_of(&mut mem(), &crt_trig, actor, vec);
    }
    /// `MsGetTransChrPos(actor, vec)`.
    MsGetTransChrPos @ 0x0079_5ac0 => extern "C" fn ms_get_chr_pos_no_chr(actor: u32, vec: u32) {
        pos::ms_get_chr_pos_no_chr(&mut mem(), &crt_trig, actor, vec);
    }
    /// `MsCheckBtlPos(i, actor, flat)`.
    MsCheckBtlPos @ 0x007a_9230 => extern "C" fn ms_check_btl_pos(i: u32, actor: u32, flat: u32) -> u32 {
        pos::ms_check_btl_pos(&mut mem(), &crt_trig, i, actor, flat)
    }
    /// `MsBtlGetPos(write, actor, group, kind, index, vec)`.
    MsBtlGetPos @ 0x007a_bfe0 => extern "C" fn ms_btl_get_pos(write: u32, actor: u32, group: i32, kind: i32, index: i32, vec: u32) -> i32 {
        pos::ms_btl_get_pos(&mut mem(), write, actor, group, kind, index, vec, summoner_pos)
    }
    /// `MsRegistCommandCue(chr, command, turn, kind, flag)`.
    MsRegistCommandCue @ 0x007b_0b90 => extern "C" fn ms_regist_command_cue(chr: u32, command: u32, turn: u32, kind: u32, flag: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // byte arguments
        cue::ms_regist_command_cue(&mut mem(), chr, command, turn as u8, kind as u8, flag as u8)
    }
    /// `MsInsertCommandCue(chr, command, turn, kind, flag, on, after)`.
    MsInsertCommandCue @ 0x007b_0a30 => extern "C" fn ms_insert_command_cue(chr: u32, command: u32, turn: u32, kind: u32, flag: u32, on: u32, after: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // byte arguments
        cue::ms_insert_command_cue(&mut mem(), chr, command, turn as u8, kind, flag as u8, on as u8, after)
    }
    /// `MsCleanCommandCue(chr, keep)`.
    MsCleanCommandCue @ 0x007b_0830 => extern "C" fn ms_clean_command_cue(chr: u32, keep: u32) -> i32 {
        cue::ms_clean_command_cue(&mut mem(), chr, keep)
    }
    /// `MsBtlListGroup(field, group)`.
    MsBtlListGroup @ 0x0079_d210 => extern "C" fn ms_btl_list_group(field: i32, group: i32) -> u32 {
        flow::ms_btl_list_group(&mem(), field, group)
    }
    /// `MsGetChrEscapePos(i, vec)`.
    MsGetChrEscapePos @ 0x0079_4ec0 => extern "C" fn ms_get_chr_escape_pos(i: u32, vec: u32) -> i32 {
        flow::ms_get_chr_escape_pos(&mut mem(), i, vec, summoner_pos)
    }
    /// `MsUseChrMpLimit(i)`.
    MsUseChrMpLimit @ 0x0078_e5a0 => extern "C" fn ms_use_chr_mp_limit(i: u32) {
        flow::ms_use_chr_mp_limit(&mut mem(), i);
    }
    /// `MsBtlGetTerm()`.
    MsBtlGetTerm @ 0x0079_8b70 => extern "C" fn ms_btl_get_term() {
        flow::ms_btl_get_term(&mut mem());
    }
    /// `MsClearStatus(i, keep)`.
    MsClearStatus @ 0x0079_a190 => extern "C" fn ms_clear_status(i: u32, keep: u32) {
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        flow::ms_clear_status(&mut mem(), i, keep as u16);
    }
    /// `MsMoveCheckFlag(actor)`.
    MsMoveCheckFlag @ 0x007a_aae0 => extern "C" fn ms_move_check_flag(actor: u32) -> u32 {
        flow::ms_move_check_flag(&mem(), actor)
    }
    /// `MsTestCheckMotion(i)`: `MsEffectCheckMotion`.
    MsTestCheckMotion @ 0x0079_0260 => extern "C" fn ms_test_check_motion(i: u32) -> i32 {
        flow::ms_effect_check_motion(&mut mem(), i)
    }
    /// `MsLimitUp(chr, actor, n)`.
    MsLimitUp @ 0x007b_1590 => extern "C" fn ms_limit_up(chr: u32, actor: u32, n: i32) -> i32 {
        turn::ms_limit_up(&mut mem(), chr, actor, n)
    }
    /// `MsCameraGet(owner, cam, kind)`.
    MsCameraGet @ 0x007b_d110 => extern "C" fn ms_camera_get(owner: u32, cam: u32, kind: u32) -> u32 {
        camera::ms_camera_get(&mut mem(), owner, cam, kind)
    }
    /// `MsSetGIL(n)`.
    MsSetGIL @ 0x0078_5c20 => extern "C" fn ms_set_gil(n: i32) {
        exec::ms_set_gil(&mut mem(), n);
    }
    /// `MsPayGIL(n)`.
    MsPayGIL @ 0x0078_59a0 => extern "C" fn ms_pay_gil(n: i32) -> i32 {
        exec::ms_pay_gil(&mut mem(), n)
    }
    /// `MsCheckChrHold(i, actor, cue)`.
    MsCheckChrHold @ 0x0079_1170 => extern "C" fn ms_check_chr_hold(_i: u32, actor: u32, _cue: u32) -> u32 {
        exec::ms_check_chr_hold(&mem(), actor)
    }
    /// `MsCountChrReady(bits)`.
    MsCountChrReady @ 0x0079_2880 => extern "C" fn ms_count_chr_ready(bits: u32) -> u32 {
        exec::ms_count_chr_ready(&mem(), bits)
    }
    /// `MsSystemMessageCheck()`.
    MsSystemMessageCheck @ 0x0079_d050 => extern "C" fn ms_system_message_check() -> i32 {
        exec::ms_system_message_check(&mut mem(), &mut call_ptr)
    }
    /// `MsCheckRikkuLimit(i, actor, cue)`.
    MsCheckRikkuLimit @ 0x007a_fbd0 => extern "C" fn ms_check_rikku_limit(i: u32, actor: u32, cue: u32) -> u32 {
        exec::ms_check_rikku_limit(&mut mem(), &mut cd_call, i, actor, cue)
    }
    /// `MsCheckSameSide(i, bits)`.
    MsCheckSameSide @ 0x0079_95c0 => extern "C" fn ms_check_same_side(i: u32, bits: u32) -> u32 {
        exec::ms_check_same_side(i, bits)
    }
    /// `MsCheckTargetsGone(bits, command)`.
    MsCheckTargetsGone @ 0x0079_9610 => extern "C" fn ms_check_targets_gone(bits: u32, command: u32) -> u32 {
        exec::ms_check_targets_gone(&mem(), bits, command)
    }
    /// `MsSetChrMimic(i, actor, cue)`.
    MsSetChrMimic @ 0x0079_9b40 => extern "C" fn ms_set_chr_mimic(i: u32, actor: u32, cue: u32) -> u32 {
        exec::ms_set_chr_mimic(&mut mem(), &crt_trig, i, actor, cue)
    }
    /// `MsActionRequestSet(kind)`.
    MsActionRequestSet @ 0x007a_cb00 => extern "C" fn ms_action_request_set(kind: u32) -> i32 {
        exec::ms_action_request_set(&mut mem(), kind.to_le_bytes()[0])
    }
    /// `MsDefensePosOffAll()`.
    MsDefensePosOffAll @ 0x007a_f800 => extern "C" fn ms_defense_pos_off_all() {
        exec::ms_defense_pos_off_all(&mut mem());
    }
    /// `MsAddChrGain(i, actor)`.
    MsAddChrGain @ 0x007a_fb60 => extern "C" fn ms_add_chr_gain(_i: u32, actor: u32) {
        exec::ms_add_chr_gain(&mut mem(), actor);
    }
    /// `MsCheckCommand3029(ids)`.
    MsCheckCommand3029 @ 0x007a_e0f0 => extern "C" fn ms_check_command_3029(ids: u32) -> u32 {
        exec::ms_check_command_3029(&mem(), ids)
    }
    /// `MsCommandKindMode(kind, command)`.
    MsCommandKindMode @ 0x007a_e120 => extern "C" fn ms_command_kind_mode(kind: u32, command: u32) -> u32 {
        exec::ms_command_kind_mode(&mem(), kind, command)
    }
    /// `MsInsertScriptCommand(i, after, id)`.
    MsInsertScriptCommand @ 0x007a_e2e0 => extern "C" fn ms_insert_script_command(i: u32, after: u32, id: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the original stores the low half
        exec::ms_insert_script_command(&mut mem(), i, after, id as u16)
    }
    /// `MsAtelGetHandler(kind, i, n, &base)`.
    MsAtelGetHandler @ 0x0079_8590 => extern "C" fn ms_atel_get_handler(kind: u32, i: u32, n: i32, out: u32) -> u32 {
        atel::ms_atel_get_handler(&mut mem(), kind, i, n, out)
    }
    /// `MsAtelRequestExe(kind, i, n, flags, actor, &same)`.
    MsAtelRequestExe @ 0x0079_7d50 => extern "C" fn ms_atel_request_exe(kind: u32, i: u32, n: i32, flags: u32, actor: u32, out: u32) -> i32 {
        script!(s);
        atel::ms_atel_request_exe(&mut mem(), &mut s, kind, i, n, flags, actor, out)
    }
    /// `MsAtelCameraPriority(kind, i, n, bonus)`.
    MsAtelCameraPriority @ 0x0079_8760 => extern "C" fn ms_atel_camera_priority(kind: u32, i: u32, n: i32, bonus: u32) {
        script!(s);
        atel::ms_atel_camera_priority(&mut mem(), &mut s, kind, i, n, bonus.to_le_bytes()[0]);
    }
    /// `MsAtelBlur()`.
    MsAtelBlur @ 0x0079_86c0 => extern "C" fn ms_atel_blur() {
        script!(s);
        atel::ms_atel_blur(&mut mem(), &mut s);
    }
    /// `MsSetMotion(i, motion, wait, x, flags, actor, &same)`.
    MsSetMotion @ 0x007a_b360 => extern "C" fn ms_set_motion(i: u32, motion: i32, wait: u32, x: u32, flags: u32, actor: u32, out: u32) -> u32 {
        script!(s);
        atel::ms_set_motion(&mut mem(), &mut s, i, motion, wait, x.to_le_bytes()[0], flags, actor, out)
    }
    /// `msResetHeadTarget(actor)`: the neck through chara's code.
    msResetHeadTarget @ 0x007b_1f10 => extern "C" fn ms_reset_head_target(actor: u32) -> u32 {
        atel::ms_reset_head_target(&mut mem(), &mut call_ptr, actor)
    }
    /// `MsSetTargetsMotion(chr, cue, k, &far)`.
    MsSetTargetsMotion @ 0x0079_1fa0 => extern "C" fn ms_set_targets_motion(chr: u32, cue: u32, k: u32, out: u32) -> i32 {
        script!(s);
        atel::ms_set_targets_motion(&mut mem(), &mut s, chr, cue, k, out)
    }
    /// `MsPaymentProcessStart(actor)`.
    MsPaymentProcessStart @ 0x007a_fa80 => extern "C" fn ms_payment_process_start(actor: u32) {
        script!(s);
        atel::ms_payment_process_start(&mut mem(), &mut s, actor);
    }
    /// `MsSetVoiceCheck(i, voice, flags, actor, &same)`.
    MsSetVoiceCheck @ 0x007a_07b0 => extern "C" fn ms_set_voice_check(i: u32, voice: i32, flags: u32, actor: u32, out: u32) -> u32 {
        script!(s);
        atel::ms_set_voice_check(&mut mem(), &mut s, i, voice, flags, actor, out)
    }
    /// `MsAtelUpdate1(ctrl)`: script 1's per-frame update.
    MsAtelUpdate1 @ 0x0079_73b0 => extern "C" fn ms_atel_update1(ctrl: u32) -> u32 {
        script!(s);
        atel::ms_atel_update(&mut mem(), &mut s, ctrl)
    }
    /// `MsAtelUpdate2(ctrl)`: script 2's per-frame update.
    MsAtelUpdate2 @ 0x0079_7350 => extern "C" fn ms_atel_update2(ctrl: u32) -> u32 {
        script!(s);
        atel::ms_atel_update(&mut mem(), &mut s, ctrl)
    }
    /// `MsAtelParseSysTag(kind, i, n)`.
    MsAtelParseSysTag @ 0x0079_79d0 => extern "C" fn ms_atel_parse_sys_tag(kind: u32, i: u32, n: i32) -> i32 {
        script!(s);
        atel::ms_atel_parse_sys_tag(&mut mem(), &mut s, kind, i, n)
    }
    /// `MsDefenseStartProcess()`: the caller's registers in the VU scale
    /// words left as they were.
    MsDefenseStartProcess @ 0x007a_f920 => extern "C" fn ms_defense_start_process() {
        script!(s);
        atel::ms_defense_start_process(&mut mem(), &mut s, &crt_trig, None);
    }
    /// `MsDirTurn(actor, dir)` (an `f32` in ST0).
    MsDirTurn @ 0x007a_9650 => extern "C" fn ms_dir_turn(actor: u32, dir: f32) -> f32 {
        f32::from_bits(target::ms_dir_turn(&mem(), actor, dir.to_bits()))
    }
    /// `MsGetDirMode(i, actor, vec, follow, &mode, &plain)` (an `f32` in ST0).
    MsGetDirMode @ 0x007a_9710 => extern "C" fn ms_get_dir_mode(i: u32, actor: u32, vec: u32, follow: u32, mode: u32, plain: u32) -> f32 {
        let mut m = mem();
        f32::from_bits(target::ms_get_dir_mode(&mut target::Dm::new(&mut m), &host(), i, actor, vec, follow, mode, plain))
    }
    /// `MsSetChrDirTarget(i, mode, turn, target, a, b)`.
    MsSetChrDirTarget @ 0x0079_5eb0 => extern "C" fn ms_set_chr_dir_target(i: u32, mode: u32, turn: u32, target: u32, a: u32, b: u32) -> i32 {
        let mut m = mem();
        target::ms_set_chr_dir_target(&mut target::Dm::new(&mut m), &host(), i, mode, turn, target, a, b)
    }
    /// `MsHideArm(i, hide)`: `Ch_ModelSetHide` (chara).
    MsHideArm @ 0x0079_5c00 => extern "C" fn ms_hide_arm(i: u32, hide: u32) {
        status::ms_hide_arm(&mut mem(), &mut call_ptr, i, hide);
    }
    /// `MsResetChrPos(i, actor)`.
    MsResetChrPos @ 0x0078_d680 => extern "C" fn ms_reset_chr_pos(i: u32, actor: u32) -> u32 {
        status::ms_reset_chr_pos(&mut mem(), &crt_trig, i, actor)
    }
    /// `MsSetChrMovePos(i, actor)`.
    MsSetChrMovePos @ 0x0078_f040 => extern "C" fn ms_set_chr_move_pos(i: u32, actor: u32) -> i32 {
        status::ms_set_chr_move_pos(&mut mem(), &crt_trig, i, actor)
    }
    /// `MsAttachWeapon(i, k)`.
    MsAttachWeapon @ 0x0079_3330 => extern "C" fn ms_attach_weapon(i: u32, k: u32) -> u32 {
        let mut m = mem();
        status::ms_attach_weapon(&mut target::Dm::new(&mut m), &host(), &mut call_ptr, i, k)
    }
    /// `MsDamageSetMotion(i, kind, flags)`.
    MsDamageSetMotion @ 0x0078_ca20 => extern "C" fn ms_damage_set_motion(i: u32, kind: u32, flags: u32) {
        let mut m = mem();
        script!(s);
        status::ms_damage_set_motion(&mut target::Dm::new(&mut m), &host(), &mut s, i, kind, flags);
    }
    /// `MsStatusEffect(i, actor, rec, on, slots)`: `op_et_eff` through the
    /// game, `op_et_eff_off_signal` (vfx).
    MsStatusEffect @ 0x0079_ef40 => extern "C" fn ms_status_effect(i: u32, actor: u32, rec: u32, on: u32, slots: u32) {
        let mut off = |m: &mut _, e| ffx_vfx::et::bridge::op_et_eff_off_signal(m, e);
        status::ms_status_effect(&mut mem(), &mut call_ptr, &mut off, i, actor, rec, on, slots);
    }
    /// `MsGetStatusLook(i)`.
    MsGetStatusLook @ 0x0079_e7c0 => extern "C" fn ms_get_status_look(i: u32) -> u32 {
        status::ms_get_status_look(&mem(), i)
    }
    /// `MsStatusEffectCheck(i)`.
    MsStatusEffectCheck @ 0x0079_eff0 => extern "C" fn ms_status_effect_check(i: u32) {
        let mut off = |m: &mut _, e| ffx_vfx::et::bridge::op_et_eff_off_signal(m, e);
        status::ms_status_effect_check(&mut mem(), &mut call_ptr, &mut off, i);
    }
    /// `MsCheckAutoStatus(i, actor)`.
    MsCheckAutoStatus @ 0x007a_f260 => extern "C" fn ms_check_auto_status(i: u32, actor: u32) -> u32 {
        let mut off = |m: &mut _, e| ffx_vfx::et::bridge::op_et_eff_off_signal(m, e);
        status::ms_check_auto_status(&mut mem(), &mut call_ptr, &mut off, i, actor)
    }
    /// `MsCheckStatusAfterInput()`.
    MsCheckStatusAfterInput @ 0x007a_f4b0 => extern "C" fn ms_check_status_after_input() -> u32 {
        let mut off = |m: &mut _, e| ffx_vfx::et::bridge::op_et_eff_off_signal(m, e);
        status::ms_check_status_after_input(&mut mem(), &mut call_ptr, &mut off)
    }
    /// `MsCheckStatusAfterAction(i, actor)`.
    MsCheckStatusAfterAction @ 0x007a_f390 => extern "C" fn ms_check_status_after_action(i: u32, actor: u32) -> u32 {
        let mut m = mem();
        script!(s);
        status::ms_check_status_after_action(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor)
    }
    /// `MsSetBtlCommand(cue, gil, flag)`.
    MsSetBtlCommand @ 0x0079_2d60 => extern "C" fn ms_set_btl_command(cue: u32, gil: i32, flag: u32) -> i32 {
        let mut m = mem();
        script!(s);
        action::ms_set_btl_command(&mut target::Dm::new(&mut m), &host(), &mut s, cue, gil, flag)
    }
    /// `MsInsertBtlCommand(cue, prio, flag, after)`.
    MsInsertBtlCommand @ 0x0079_29b0 => extern "C" fn ms_insert_btl_command(cue: u32, prio: u32, flag: u32, after: u32) -> i32 {
        let mut m = mem();
        script!(s);
        action::ms_insert_btl_command(&mut target::Dm::new(&mut m), &host(), &mut s, cue, prio, flag, after)
    }
    /// `MsActionRequest(i, who, n, special, mode, out)`.
    MsActionRequest @ 0x007a_ceb0 => extern "C" fn ms_action_request(i: u32, who: u32, n: i32, special: u32, mode: u32, out: u32) -> i32 {
        let mut m = mem();
        script!(s);
        action::ms_action_request(&mut target::Dm::new(&mut m), &host(), &mut s, i, who, n, special, mode, out)
    }
    /// `MsActionTarget(bits, who, _, _, n, mode)`.
    MsActionTarget @ 0x007a_d0c0 => extern "C" fn ms_action_target(bits: u32, who: u32, _a: u32, _b: u32, n: i32, mode: u32) -> u32 {
        let mut m = mem();
        script!(s);
        action::ms_action_target(&mut target::Dm::new(&mut m), &host(), &mut s, bits, who, n, mode)
    }
    /// `MsCheckSummon(i, actor, x, cue)`.
    MsCheckSummon @ 0x007b_2980 => extern "C" fn ms_check_summon(i: u32, actor: u32, x: u32, cue: u32) -> i32 {
        let mut m = mem();
        script!(s);
        action::ms_check_summon(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor, x, cue)
    }
    /// `MsDeleteInputCue(chr, k, last, keep)`.
    MsDeleteInputCue @ 0x007b_20e0 => extern "C" fn ms_delete_input_cue(chr: u32, k: u32, last: u32, keep: u32) -> u32 {
        let mut m = mem();
        script!(s);
        action::ms_delete_input_cue(&mut target::Dm::new(&mut m), &host(), &mut s, chr, k, last, keep)
    }
    /// `msSetChrShadowReq(p, k, v)`.
    msSetChrShadowReq @ 0x0079_7020 => extern "C" fn ms_set_chr_shadow_req(p: u32, k: i32, v: u32) -> u32 {
        visible::ms_set_chr_shadow_req(&mut mem(), p, k, v.to_le_bytes()[0])
    }
    /// `msSetChrVisibleReq(i, k, v)`: `op_et_bindeff` (vfx) through the game.
    msSetChrVisibleReq @ 0x0079_7080 => extern "C" fn ms_set_chr_visible_req(i: u32, k: u32, v: u32) -> u32 {
        visible::ms_set_chr_visible_req(&mut mem(), &mut call_ptr, i, k, v)
    }
    /// `MsSetChrVisibleCamera(i, v)`.
    MsSetChrVisibleCamera @ 0x0079_67a0 => extern "C" fn ms_set_chr_visible_camera(i: u32, v: u32) -> u32 {
        visible::ms_set_chr_visible_camera(&mut mem(), &mut call_ptr, i, v)
    }
    /// `MsSetChrVisibleCameraAll(v)`.
    MsSetChrVisibleCameraAll @ 0x0079_67c0 => extern "C" fn ms_set_chr_visible_camera_all(v: u32) -> u32 {
        visible::ms_set_chr_visible_camera_all(&mut mem(), &mut call_ptr, v)
    }
    /// `MsReturnCameraVisible()`: `eax` as it was (the original leaves it
    /// alone when nothing was hidden; no caller reads it).
    MsReturnCameraVisible @ 0x0079_5c70 => extern "C" fn ms_return_camera_visible() {
        visible::ms_return_camera_visible(&mut mem(), &mut call_ptr);
    }
    /// `MsAtelRequestCamera(i, monster, a, b, show, flags, actor, &same)`.
    MsAtelRequestCamera @ 0x0079_7bc0 => extern "C" fn ms_atel_request_camera(i: u32, monster: u32, a: u32, b: u32, show: u32, flags: u32, actor: u32, out: u32) -> i32 {
        script!(s);
        atel::ms_atel_request_camera(&mut mem(), &mut s, i, monster, a, b, show, flags, actor, out)
    }
    /// `MsLimitTypeExpGet(i, k, force)`.
    MsLimitTypeExpGet @ 0x007b_10c0 => extern "C" fn ms_limit_type_exp_get(i: u32, k: u32, force: u32) -> u32 {
        exec::ms_limit_type_exp_get(&mut mem(), i, k, force)
    }
    /// `MsLimitTypeEscapeCheck(i, actor)`.
    MsLimitTypeEscapeCheck @ 0x007b_1090 => extern "C" fn ms_limit_type_escape_check(i: u32, actor: u32) -> u32 {
        exec::ms_limit_type_escape_check(&mut mem(), i, actor)
    }
    /// `MsMusicVolume(volume, frames)`: sound's functions through the game.
    MsMusicVolume @ 0x007a_0090 => extern "C" fn ms_music_volume(volume: u32, frames: u32) -> u32 {
        exec::ms_music_volume(&mut mem(), &mut call_ptr, volume, frames)
    }
    /// `MsMenuOpenTitleWindow(i, pos)`: system's window through the game.
    MsMenuOpenTitleWindow @ 0x0078_fea0 => extern "C" fn ms_menu_open_title_window(i: u32, pos: u32) -> u32 {
        exec::ms_menu_open_title_window(&mut mem(), &mut call_ptr, i, pos)
    }
    /// `MsAtelRequestMagicCamera(i, monster, id, a, b, show, flags, actor, &same)`.
    MsAtelRequestMagicCamera @ 0x0079_8000 => extern "C" fn ms_atel_request_magic_camera(i: u32, monster: u32, id: u32, a: u32, b: u32, show: u32, flags: u32, actor: u32, out: u32) -> i32 {
        script!(s);
        atel::ms_atel_request_magic_camera(&mut mem(), &mut s, i, monster, id, a, b, show, flags, actor, out)
    }
    /// `MsEscapeAll()`.
    MsEscapeAll @ 0x007a_da60 => extern "C" fn ms_escape_all() {
        script!(s);
        atel::ms_escape_all(&mut mem(), &mut s);
    }
    /// `MsGetEscapeDir(i)` (an `f32` in ST0).
    MsGetEscapeDir @ 0x007a_99b0 => extern "C" fn ms_get_escape_dir(i: u32) -> f32 {
        let mut m = mem();
        f32::from_bits(target::ms_get_escape_dir(&mut target::Dm::new(&mut m), &host(), i))
    }
    /// `MsCheckTargetsNear(i, actor, targets)`.
    MsCheckTargetsNear @ 0x007a_f0f0 => extern "C" fn ms_check_targets_near(i: u32, actor: u32, targets: u32) -> u32 {
        let mut m = mem();
        target::ms_check_targets_near(&mut target::Dm::new(&mut m), &host(), i, actor, targets)
    }
    /// `MsCheckChrNear(i, actor)`.
    MsCheckChrNear @ 0x007a_f230 => extern "C" fn ms_check_chr_near(i: u32, actor: u32) -> u32 {
        let mut m = mem();
        target::ms_check_chr_near(&mut target::Dm::new(&mut m), &host(), i, actor)
    }
    /// `MsEffectFileNumber(n, k)`.
    MsEffectFileNumber @ 0x0078_7a70 => extern "C" fn ms_effect_file_number(n: i32, k: i32) -> i32 {
        exec::ms_effect_file_number(&mem(), n, k)
    }
    /// `MsSEPchrClear(actor, shared)`: `SndSepClear` (sound) through the game.
    MsSEPchrClear @ 0x007a_0400 => extern "C" fn ms_sep_chr_clear(actor: u32, shared: u32) {
        exec::ms_sep_chr_clear(&mut mem(), &mut call_ptr, actor, shared);
    }
    /// `MsGetAeonCommand(i, id, kind, &mode, &value)`.
    MsGetAeonCommand @ 0x007a_f060 => extern "C" fn ms_get_aeon_command(i: u32, id: u32, kind: u32, mode: u32, value: u32) -> u32 {
        exec::ms_get_aeon_command(&mut mem(), i, id, kind, mode, value)
    }
    /// `MsInsertChrCommand(i, target, kind)`.
    MsInsertChrCommand @ 0x007a_e1a0 => extern "C" fn ms_insert_chr_command(i: u32, target: u32, kind: u32) -> u32 {
        action::ms_insert_chr_command(&mut mem(), i, target, kind)
    }
    /// `MsCleanInputCue(chr, keep)`.
    MsCleanInputCue @ 0x007b_20b0 => extern "C" fn ms_clean_input_cue(chr: u32, keep: u32) -> u32 {
        let mut m = mem();
        script!(s);
        action::ms_clean_input_cue(&mut target::Dm::new(&mut m), &host(), &mut s, chr, keep)
    }
    /// `MsGetSaveWeaponModel(id)`.
    MsGetSaveWeaponModel @ 0x007a_bdc0 => extern "C" fn ms_get_save_weapon_model(id: u32) -> u32 {
        exchange::ms_get_save_weapon_model(&mut mem(), id)
    }
    /// `MsCheckBoundWeapon(id)` (`0x007ad130`).
    MsCheckBoundWeapon @ 0x007a_d130 => extern "C" fn ms_check_bound_weapon(id: u32) -> u32 {
        exchange::ms_check_bound_weapon(&mem(), id)
    }
    /// `MsBtlReadCueManage()`: the loads through the game (chara, battle).
    MsBtlReadCueManage @ 0x007a_d190 => extern "C" fn ms_btl_read_cue_manage() {
        let mut m = mem();
        script!(s);
        exchange::ms_btl_read_cue_manage(&mut target::Dm::new(&mut m), &mut s);
    }
    /// `MsBtlReadCueRegist(kind, chr, owner, id, then)`.
    MsBtlReadCueRegist @ 0x007a_d3f0 => extern "C" fn ms_btl_read_cue_regist(kind: u32, chr: u32, owner: u32, id: u32, then: u32) {
        let mut m = mem();
        script!(s);
        exchange::ms_btl_read_cue_regist(&mut target::Dm::new(&mut m), &mut s, kind, chr, owner, id, then);
    }
    /// `MsBtlReadCueRegistWeapon(i, which)`.
    MsBtlReadCueRegistWeapon @ 0x007a_d470 => extern "C" fn ms_btl_read_cue_regist_weapon(i: u32, which: u32) {
        let mut m = mem();
        script!(s);
        exchange::ms_btl_read_cue_regist_weapon(&mut target::Dm::new(&mut m), &mut s, i, which);
    }
    /// `MsEffectWeaponFree(i)`: `op_et_weapon_free` (vfx) through the game.
    MsEffectWeaponFree @ 0x0078_8b30 => extern "C" fn ms_effect_weapon_free(i: u32) {
        script!(s);
        exchange::ms_effect_weapon_free(&mut mem(), &mut s, i);
    }
    /// `MsThreatEndMotion(i, actor)` (`0x0078f6a0`).
    MsThreatEndMotion @ 0x0078_f6a0 => extern "C" fn ms_threat_end_motion(i: u32, actor: u32) {
        script!(s);
        exchange::ms_threat_end_motion(&mut mem(), &mut s, i, actor);
    }
    /// `MsThreatProcess(i, actor)`.
    MsThreatProcess @ 0x0078_e460 => extern "C" fn ms_threat_process(i: u32, actor: u32) -> u32 {
        let mut m = mem();
        script!(s);
        exchange::ms_threat_process(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor)
    }
    /// `MsThreatOff(i, actor)`.
    MsThreatOff @ 0x0078_e410 => extern "C" fn ms_threat_off(i: u32, actor: u32) {
        let mut m = mem();
        script!(s);
        exchange::ms_threat_off(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor);
    }
    /// `MsGetChrCenter(first, n, out)` (`0x00794b70`; the divide's caller
    /// words as the VU registers had them).
    MsGetChrCenter @ 0x0079_4b70 => extern "C" fn ms_get_chr_center(first: u32, n: i32, out: u32) {
        let mut m = mem();
        exchange::ms_get_chr_center(&mut target::Dm::new(&mut m), &host(), first, n, out, None);
    }
    /// `MsCalcChrDir(i, actor)` (an `f32` in ST0; the caller words as the VU
    /// registers had them).
    MsCalcChrDir @ 0x0079_3ba0 => extern "C" fn ms_calc_chr_dir(i: u32, _actor: u32) -> f32 {
        let mut m = mem();
        f32::from_bits(exchange::ms_calc_chr_dir(&mut target::Dm::new(&mut m), &host(), i, None))
    }
    /// `MsChangeMonsterDir()`.
    MsChangeMonsterDir @ 0x0079_3cf0 => extern "C" fn ms_change_monster_dir() {
        let mut m = mem();
        script!(s);
        exchange::ms_change_monster_dir(&mut target::Dm::new(&mut m), &host(), &mut s);
    }
    /// `MsExchangeChr(out, in, mode, after)`.
    MsExchangeChr @ 0x007a_dae0 => extern "C" fn ms_exchange_chr(out: u32, inn: u32, mode: i32, after: u32) -> u32 {
        let mut m = mem();
        script!(s);
        exchange::ms_exchange_chr(&mut target::Dm::new(&mut m), &host(), &mut s, out, inn, mode, after)
    }
    /// `MsExchangeSummon(in, summon)`.
    MsExchangeSummon @ 0x007a_df90 => extern "C" fn ms_exchange_summon(inn: u32, summon: u32) {
        let mut m = mem();
        script!(s);
        exchange::ms_exchange_summon(&mut target::Dm::new(&mut m), &host(), &mut s, inn, summon);
    }
    /// `MsEffectRead(i, id)`: the magic files (system, vfx) through the game.
    MsEffectRead @ 0x0078_7f20 => extern "C" fn ms_effect_read(i: u32, id: u32) -> i32 {
        script!(s);
        parse::ms_effect_read(&mut mem(), &mut s, i, id)
    }
    /// `MsEffectReadComplete()`.
    MsEffectReadComplete @ 0x0078_8110 => extern "C" fn ms_effect_read_complete() -> u32 {
        script!(s);
        parse::ms_effect_read_complete(&mut mem(), &mut s)
    }
    /// `MsParseCommand(cue)`.
    MsParseCommand @ 0x007a_e370 => extern "C" fn ms_parse_command(cue: u32) -> i32 {
        let mut m = mem();
        script!(s);
        parse::ms_parse_command(&mut target::Dm::new(&mut m), &host(), &mut s, cue)
    }
    /// `MsCommandCueProcess()` (`0x00792210`).
    MsCommandCueProcess @ 0x0079_2210 => extern "C" fn ms_command_cue_process() -> u32 {
        let mut m = mem();
        script!(s);
        parse::ms_command_cue_process(&mut target::Dm::new(&mut m), &host(), &mut s)
    }
    /// `MsNowMusicStop()`: sound through the game.
    MsNowMusicStop @ 0x007a_00b0 => extern "C" fn ms_now_music_stop() -> u32 {
        process::ms_now_music_stop(&mut mem(), &mut call_ptr)
    }
    /// `MsResetNormalCalc()`.
    MsResetNormalCalc @ 0x0079_5c40 => extern "C" fn ms_reset_normal_calc() -> u32 {
        process::ms_reset_normal_calc(&mut mem())
    }
    /// `MsBattleCanProgress()` (`0x00791190`).
    MsBattleCanProgress @ 0x0079_1190 => extern "C" fn ms_battle_can_progress() -> i32 {
        process::ms_battle_can_progress(&mem())
    }
    /// `MsSetDeathAll()` (`0x00791760`).
    MsSetDeathAll @ 0x0079_1760 => extern "C" fn ms_set_death_all() {
        let mut m = mem();
        script!(s);
        process::ms_set_death_all(&mut target::Dm::new(&mut m), &host(), &mut s);
    }
    /// `msVoiceCheckStanby(any)`: the CD library and sound through the game.
    msVoiceCheckStanby @ 0x007b_2c10 => extern "C" fn ms_voice_check_stanby(any: u32) -> i32 {
        process::ms_voice_check_stanby(&mut mem(), &mut call_ptr, any)
    }
    /// `MsCheckSoftResetOk()` (`0x00792e80`).
    MsCheckSoftResetOk @ 0x0079_2e80 => extern "C" fn ms_check_soft_reset_ok() -> i32 {
        process::ms_check_soft_reset_ok(&mut mem(), &mut call_ptr)
    }
    /// `MsDebugCameraSleep(v)` (`0x007921c0`).
    MsDebugCameraSleep @ 0x0079_21c0 => extern "C" fn ms_debug_camera_sleep(v: u32) {
        camera::ms_debug_camera_sleep(&mut mem(), v);
    }
    /// `MsCameraInit()`.
    MsCameraInit @ 0x007b_d5d0 => extern "C" fn ms_camera_init() {
        camera::ms_camera_init(&mut mem());
    }
    /// `MsCheckDeathMotion(i, actor)`.
    MsCheckDeathMotion @ 0x0078_c4c0 => extern "C" fn ms_check_death_motion(i: u32, actor: u32) -> u32 {
        death::ms_check_death_motion(&mem(), i, actor)
    }
    /// `MsDeathTerminate(i, actor)`.
    MsDeathTerminate @ 0x0078_cca0 => extern "C" fn ms_death_terminate(i: u32, actor: u32) -> u32 {
        script!(s);
        death::ms_death_terminate(&mut mem(), &mut s, i, actor)
    }
    /// `MsSetDeathProcess(i, actor)`.
    MsSetDeathProcess @ 0x0078_ddc0 => extern "C" fn ms_set_death_process(i: u32, actor: u32) {
        let mut m = mem();
        script!(s);
        death::ms_set_death_process(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor);
    }
    /// `MsDeathEffectStart(i, kind)`: the sound and effect through the game.
    MsDeathEffectStart @ 0x0079_e3d0 => extern "C" fn ms_death_effect_start(i: u32, kind: u32) -> u32 {
        script!(s);
        death::ms_death_effect_start(&mut mem(), &mut s, i, kind)
    }
    /// `MsStoneEffectProcess()`.
    MsStoneEffectProcess @ 0x007a_2690 => extern "C" fn ms_stone_effect_process() -> u32 {
        script!(s);
        death::ms_stone_effect_process(&mut mem(), &mut s)
    }
    /// `MsSetDeath(i, flags)`.
    MsSetDeath @ 0x0078_dc30 => extern "C" fn ms_set_death(i: u32, flags: u32) -> u32 {
        let mut m = mem();
        script!(s);
        death::ms_set_death(&mut target::Dm::new(&mut m), &host(), &mut s, i, flags)
    }
    /// `MsMotionPosCheck(i, actor)`.
    MsMotionPosCheck @ 0x007a_a100 => extern "C" fn ms_motion_pos_check(i: u32, actor: u32) {
        let mut m = mem();
        process::ms_motion_pos_check(&mut target::Dm::new(&mut m), &host(), i, actor);
    }
    /// `MsMotionPosCheckAll()`.
    MsMotionPosCheckAll @ 0x007a_a140 => extern "C" fn ms_motion_pos_check_all() {
        let mut m = mem();
        process::ms_motion_pos_check_all(&mut target::Dm::new(&mut m), &host());
    }
    /// `MsCountChrActing()` (`0x007b0ca0`).
    MsCountChrActing @ 0x007b_0ca0 => extern "C" fn ms_count_chr_acting() -> u32 {
        process::ms_count_chr_acting(&mem())
    }
    /// `MsLimitTypeProcess()`: `MsMessageCueRegist` through the game.
    MsLimitTypeProcess @ 0x007b_1180 => extern "C" fn ms_limit_type_process() {
        process::ms_limit_type_process(&mut mem(), &mut call_ptr);
    }
    /// `MsEtEffectFree()`: vfx and the heap through the game.
    MsEtEffectFree @ 0x0079_e630 => extern "C" fn ms_et_effect_free() {
        process::ms_et_effect_free(&mut mem(), &mut call_ptr);
    }
    /// `MsFldGetScene(scene, &field, &group, &k)`.
    MsFldGetScene @ 0x0078_27f0 => extern "C" fn ms_fld_get_scene(scene: u32, field: u32, group: u32, slot: u32) -> u32 {
        process::ms_fld_get_scene(&mut mem(), scene, field, group, slot)
    }
    /// `MsBattleLabelExe(scene, a, b)`.
    MsBattleLabelExe @ 0x0078_1c90 => extern "C" fn ms_battle_label_exe(scene: u32, a: u32, b: u32) -> i32 {
        let mut m = mem();
        process::ms_battle_label_exe(&mut target::Dm::new(&mut m), scene, a, b)
    }
    /// `MsOrderKey(i)` (`0x0078f000`).
    MsOrderKey @ 0x0078_f000 => extern "C" fn ms_order_key(i: u32) -> u32 {
        process::ms_order_key(&mem(), i)
    }
    /// `MsOrderCalc(&n)`.
    MsOrderCalc @ 0x0078_d3e0 => extern "C" fn ms_order_calc(n: u32) -> u32 {
        process::ms_order_calc(&mut mem(), n)
    }
    /// `MsLimitTypeTurnCheck(i, actor)`.
    MsLimitTypeTurnCheck @ 0x007b_13c0 => extern "C" fn ms_limit_type_turn_check(i: u32, actor: u32) -> u32 {
        process::ms_limit_type_turn_check(&mut mem(), i, actor)
    }
    /// `MsBattleProcess_Order_00790fb0()`: `MagicFile_GetMagicId` through the game.
    MsBattleProcess_Order_00790fb0 @ 0x0079_0fb0 => extern "C" fn ms_battle_process_order() -> i32 {
        process::ms_battle_process_order(&mut mem(), &mut call_ptr)
    }
    /// `MsBattleEncountExe(map, group, step)` (`step` an `f32`): `Sg_FadeInW` through the game.
    MsBattleEncountExe @ 0x0078_0d10 => extern "C" fn ms_battle_encount_exe(map: i32, group: i32, step: u32) -> i32 {
        process::ms_battle_encount_exe(&mut mem(), &mut call_ptr, map, group, step)
    }
    /// `MsBattleError(msg)`: `dbgPrintf` through the game, then it hangs.
    MsBattleError @ 0x0079_f210 => extern "C" fn ms_battle_error(msg: u32) {
        input::ms_battle_error(&mut mem(), &mut call_ptr, msg)
    }
    /// `MsSetChrWeak(i, level)`.
    MsSetChrWeak @ 0x0078_d7f0 => extern "C" fn ms_set_chr_weak(i: u32, level: u32) -> u32 {
        input::ms_set_chr_weak(&mut mem(), i, level)
    }
    /// `0x0079fc00(n, &digits)` (`MsNumberDigits`).
    MsNumberDigits @ 0x0079_fc00 => extern "C" fn ms_number_digits(n: i32, digits: u32) -> i32 {
        input::ms_number_digits(&mut mem(), n, digits)
    }
    /// `0x0079fc60(...)` (`MsNumberPut`): the byte and halfword arguments
    /// take their dwords' low parts.
    MsNumberPut @ 0x0079_fc60 => extern "C" fn ms_number_put(i: u32, k: u32, ch: u32, x: u32, y: u32, w: u32, h: u32, life: u32, colour: u32, style: u32) -> u32 {
        let b = |v: u32| v.to_le_bytes()[0];
        let s = |v: u32| u16::from_le_bytes([v.to_le_bytes()[0], v.to_le_bytes()[1]]);
        input::ms_number_put(&mut mem(), b(i), b(k), b(ch), s(x), s(y), b(w), b(h), b(life), b(colour), b(style))
    }
    /// `MsNumberRegist(i, kind, value, other, x, flags, style)`.
    MsNumberRegist @ 0x0079_fa00 => extern "C" fn ms_number_regist(i: u32, kind: u32, value: i32, other: i32, x: u32, flags: u32, style: u32) -> i32 {
        input::ms_number_regist(&mut mem(), i, kind, value, other, x, flags, style)
    }
    /// `MsSubHP(i, actor, n, other, x, flags, style)`.
    MsSubHP @ 0x0078_e230 => extern "C" fn ms_sub_hp(i: u32, actor: u32, n: i32, other: i32, x: u32, flags: u32, style: u32) {
        input::ms_sub_hp(&mut mem(), i, actor, n, other, x, flags, style);
    }
    /// `MsCheatPlayerPowerup(i)`.
    MsCheatPlayerPowerup @ 0x0078_4720 => extern "C" fn ms_cheat_player_powerup(i: u32) {
        let mut m = mem();
        script!(s);
        input::ms_cheat_player_powerup(&mut target::Dm::new(&mut m), &host(), &mut s, i);
    }
    /// `0x007af840(i, actor)` (`MsStatusEndMotion`).
    MsStatusEndMotion @ 0x007a_f840 => extern "C" fn ms_status_end_motion(i: u32, actor: u32) {
        script!(s);
        input::ms_status_end_motion(&mut mem(), &mut s, i, actor);
    }
    /// `MsCheckStatusBeforeAction(i, actor)`.
    MsCheckStatusBeforeAction @ 0x007a_f4f0 => extern "C" fn ms_check_status_before_action(i: u32, actor: u32) -> u32 {
        let mut m = mem();
        script!(s);
        input::ms_check_status_before_action(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor)
    }
    /// `0x00792120(i, actor)` (`MsCheckAutoInput`).
    MsCheckAutoInput @ 0x0079_2120 => extern "C" fn ms_check_auto_input(i: u32, actor: u32) -> i32 {
        input::ms_check_auto_input(&mut mem(), i, actor)
    }
    /// `MsSetDeathSentence(i, actor, cmd, flag)`.
    MsSetDeathSentence @ 0x0079_9cd0 => extern "C" fn ms_set_death_sentence(i: u32, actor: u32, cmd: u32, flag: u32) -> u32 {
        let mut m = mem();
        script!(s);
        input::ms_set_death_sentence(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor, cmd, flag)
    }
    /// `MsSetBerserkCommand(i, actor, cmd, flag, side)`.
    MsSetBerserkCommand @ 0x0079_9ac0 => extern "C" fn ms_set_berserk_command(i: u32, actor: u32, cmd: u32, flag: u32, side: u32) -> i32 {
        let mut m = mem();
        script!(s);
        input::ms_set_berserk_command(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor, cmd, flag, side)
    }
    /// `MsSetProvokingCommand(i, actor, cmd, flag)`.
    MsSetProvokingCommand @ 0x0079_9d50 => extern "C" fn ms_set_provoking_command(i: u32, actor: u32, cmd: u32, flag: u32) -> i32 {
        let mut m = mem();
        script!(s);
        input::ms_set_provoking_command(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor, cmd, flag)
    }
    /// `MsBattleProcess_InputCue_00792a90()`: the menu through the game.
    MsBattleProcess_InputCue_00792a90 @ 0x0079_2a90 => extern "C" fn ms_battle_process_input_cue() {
        let mut m = mem();
        script!(s);
        input::ms_battle_process_input_cue(&mut target::Dm::new(&mut m), &host(), &mut s);
    }
    /// `MsLimitTypeDeathCheck(i, actor, j, victim)`.
    MsLimitTypeDeathCheck @ 0x007b_0f80 => extern "C" fn ms_limit_type_death_check(i: u32, actor: u32, j: u32, victim: u32) -> u32 {
        death::ms_limit_type_death_check(&mut mem(), i, actor, j, victim)
    }
    /// `0x007b2740(i, target, after, id, prio)` (`MsAutoInsertAbility`).
    MsAutoInsertAbility @ 0x007b_2740 => extern "C" fn ms_auto_insert_ability(i: u32, target: u32, after: u32, id: u32, prio: u32) {
        let mut m = mem();
        script!(s);
        death::ms_auto_insert_ability(&mut target::Dm::new(&mut m), &host(), &mut s, i & 0xff, target & 0xff, after, id, prio);
    }
    /// `MsAutoInsertCommand_007b26d0(i, target, after, item, prio)`.
    MsAutoInsertCommand_007b26d0 @ 0x007b_26d0 => extern "C" fn ms_auto_insert_command(i: u32, target: u32, after: u32, item: u32, prio: u32) {
        let mut m = mem();
        script!(s);
        death::ms_auto_insert_command(&mut target::Dm::new(&mut m), &host(), &mut s, i & 0xff, target & 0xff, after, item & 0xffff, prio);
    }
    /// `MsAutoRelifeProcess_007b2910(i, actor, killer, status)`.
    MsAutoRelifeProcess_007b2910 @ 0x007b_2910 => extern "C" fn ms_auto_relife_process(i: u32, actor: u32, killer: u32, status: u32) -> i32 {
        let mut m = mem();
        script!(s);
        death::ms_auto_relife_process(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor, killer, status)
    }
    /// `MsAutoPhenixProcess(i, actor, killer)`.
    MsAutoPhenixProcess @ 0x007b_27b0 => extern "C" fn ms_auto_phenix_process(i: u32, actor: u32, killer: u32) -> u32 {
        let mut m = mem();
        script!(s);
        death::ms_auto_phenix_process(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor, killer)
    }
    /// `MsDamageCheckDeath(i, j, no_reward, flags)`: `MsGetItem`, the sign
    /// window and `forceAutoBattleOff` through the game.
    MsDamageCheckDeath @ 0x0078_c740 => extern "C" fn ms_damage_check_death(i: u32, j: u32, no_reward: u32, flags: u32) -> u32 {
        let mut m = mem();
        script!(s);
        death::ms_damage_check_death(&mut target::Dm::new(&mut m), &host(), &mut s, i, j, no_reward, flags)
    }
    /// `MsSetChrVisible(i, v)`.
    MsSetChrVisible @ 0x0079_6660 => extern "C" fn ms_set_chr_visible(i: u32, v: u32) -> u32 {
        visible::ms_set_chr_visible(&mut mem(), &mut call_ptr, i, v)
    }
    /// `MsSetChrVisibleAppear(i, v)`.
    MsSetChrVisibleAppear @ 0x0079_66a0 => extern "C" fn ms_set_chr_visible_appear(i: u32, v: u32) -> u32 {
        visible::ms_set_chr_visible_appear(&mut mem(), &mut call_ptr, i, v)
    }
    /// `MsScreenFocusResetAll()`: `Sg_EnFocus` through the game.
    MsScreenFocusResetAll @ 0x007b_bda0 => extern "C" fn ms_screen_focus_reset_all() {
        chrdmg::ms_screen_focus_reset_all(&mut mem(), &mut call_ptr);
    }
    /// `0x00792ee0()` (`MsCheckPartyDying`).
    MsCheckPartyDying @ 0x0079_2ee0 => extern "C" fn ms_check_party_dying() -> i32 {
        chrdmg::ms_check_party_dying(&mem())
    }
    /// `0x00791780(i, actor)` (`MsCheckRestMotion`).
    MsCheckRestMotion @ 0x0079_1780 => extern "C" fn ms_check_rest_motion(i: u32, actor: u32) -> u32 {
        chrdmg::ms_check_rest_motion(&mem(), i, actor)
    }
    /// `MsEffectCameraEnd(keep)`.
    MsEffectCameraEnd @ 0x0078_7760 => extern "C" fn ms_effect_camera_end(keep: u32) {
        script!(s);
        chrdmg::ms_effect_camera_end(&mut mem(), &mut s, keep);
    }
    /// `0x0078ee80(a, b, i)` (`MsCheckHitPending`).
    MsCheckHitPending @ 0x0078_ee80 => extern "C" fn ms_check_hit_pending(a: u32, b: u32, i: u32) -> u32 {
        chrdmg::ms_check_hit_pending(&mem(), a, b, i)
    }
    /// `MsPoisonProcess(i, actor)`.
    MsPoisonProcess @ 0x007a_fab0 => extern "C" fn ms_poison_process(i: u32, actor: u32) {
        let mut m = mem();
        script!(s);
        chrdmg::ms_poison_process(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor);
    }
    /// `0x00791690(actor)` (`MsCheckCommandEnd`).
    MsCheckCommandEnd @ 0x0079_1690 => extern "C" fn ms_check_command_end(actor: u32) -> u32 {
        chrdmg::ms_check_command_end(&mut mem(), actor)
    }
    /// `MsSetEscapeFlag(i, actor, away)`.
    MsSetEscapeFlag @ 0x0078_de40 => extern "C" fn ms_set_escape_flag(i: u32, actor: u32, away: u32) {
        chrdmg::ms_set_escape_flag(&mut mem(), &mut call_ptr, i, actor, away);
    }
    /// `MsSetEscapeFlagAll(away)`.
    MsSetEscapeFlagAll @ 0x0078_de80 => extern "C" fn ms_set_escape_flag_all(away: u32) {
        chrdmg::ms_set_escape_flag_all(&mut mem(), &mut call_ptr, away);
    }
    /// `MsMagicEffectEnd(i)`: the prayer effect through the game.
    MsMagicEffectEnd @ 0x0079_e830 => extern "C" fn ms_magic_effect_end(i: u32) -> i32 {
        chrdmg::ms_magic_effect_end(&mut mem(), &mut call_ptr, i)
    }
    /// `MsSetChrDirF(i, dir)` (`dir` an `f32`).
    MsSetChrDirF @ 0x0079_5e20 => extern "C" fn ms_set_chr_dir_f(i: u32, dir: u32) -> i32 {
        chrdmg::ms_set_chr_dir_f(&mut mem(), i, dir)
    }
    /// `MsSetChrPosReset(i, pos)`.
    MsSetChrPosReset @ 0x0079_6220 => extern "C" fn ms_set_chr_pos_reset(i: u32, pos: u32) -> i32 {
        chrdmg::ms_set_chr_pos_reset(&mut mem(), i, pos)
    }
    /// `MsStoneMove(i)`.
    MsStoneMove @ 0x007a_2750 => extern "C" fn ms_stone_move(i: u32) {
        script!(s);
        chrdmg::ms_stone_move(&mut mem(), &mut s, i);
    }
    /// `MsEffectEndMotion(i, kind)`.
    MsEffectEndMotion @ 0x0078_7950 => extern "C" fn ms_effect_end_motion(i: u32, kind: u32) {
        script!(s);
        chrdmg::ms_effect_end_motion(&mut mem(), &mut s, i, kind);
    }
    /// `MsCommandReturnChrStatusAll(all)`.
    MsCommandReturnChrStatusAll @ 0x0079_3e00 => extern "C" fn ms_command_return_chr_status_all(all: u32) {
        let mut m = mem();
        script!(s);
        chrdmg::ms_command_return_chr_status_all(&mut target::Dm::new(&mut m), &host(), &mut s, all);
    }
    /// `MsRelife(i, actor, full)`.
    MsRelife @ 0x0078_d530 => extern "C" fn ms_relife(i: u32, actor: u32, full: u32) {
        chrdmg::ms_relife(&mut mem(), &mut call_ptr, i, actor, full);
    }
    /// `MsBlowProcess(i, actor)`.
    MsBlowProcess @ 0x0078_91b0 => extern "C" fn ms_blow_process(i: u32, actor: u32) -> u32 {
        chrdmg::ms_blow_process(&mut mem(), &mut call_ptr, i, actor)
    }
    /// `MsSubMP(i, actor, n, other, x, flags, style)`.
    MsSubMP @ 0x0078_e3b0 => extern "C" fn ms_sub_mp(i: u32, actor: u32, n: i32, other: i32, x: u32, flags: u32, style: u32) {
        afterdmg::ms_sub_mp(&mut mem(), i, actor, n, other, x, flags, style);
    }
    /// `MsSubCTB(i, actor, n, x, flags, style)`.
    MsSubCTB @ 0x0078_e1e0 => extern "C" fn ms_sub_ctb(i: u32, actor: u32, n: i32, x: u32, flags: u32, style: u32) {
        afterdmg::ms_sub_ctb(&mut mem(), i, actor, n, x, flags, style);
    }
    /// `MsAliveProcess(i, actor)`.
    MsAliveProcess @ 0x0078_9160 => extern "C" fn ms_alive_process(i: u32, actor: u32) -> u32 {
        script!(s);
        afterdmg::ms_alive_process(&mut mem(), &mut s, i, actor)
    }
    /// `MsAutoRelifeProcess(i, actor, by, helper)`.
    MsAutoRelifeProcess @ 0x0078_d8d0 => extern "C" fn ms_auto_relife_command(i: u32, actor: u32, by: u32, helper: u32) -> u32 {
        let mut m = mem();
        script!(s);
        afterdmg::ms_auto_relife_command(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor, by, helper)
    }
    /// `MsSetChrEffectNext_0079ebe0(i, a, b)`: `op_et_eff` through the game.
    MsSetChrEffectNext_0079ebe0 @ 0x0079_ebe0 => extern "C" fn ms_set_chr_effect_next(i: u32, a: u32, b: u32) -> u32 {
        afterdmg::ms_set_chr_effect_next(&mut mem(), &mut call_ptr, i, a, b)
    }
    /// `MsSetStealEffect(i, got)`.
    MsSetStealEffect @ 0x0079_ed00 => extern "C" fn ms_set_steal_effect(i: u32, got: u32) {
        afterdmg::ms_set_steal_effect(&mut mem(), &mut call_ptr, i, got);
    }
    /// `MsSetStealGillEffect(i, got)`.
    MsSetStealGillEffect @ 0x0079_ed20 => extern "C" fn ms_set_steal_gill_effect(i: u32, got: u32) {
        afterdmg::ms_set_steal_gill_effect(&mut mem(), &mut call_ptr, i, got);
    }
    /// `MsGetChrCenterPos(i, vec)`.
    MsGetChrCenterPos @ 0x0079_4c20 => extern "C" fn ms_get_chr_center_pos(i: u32, vec: u32) -> i32 {
        target::ms_get_chr_center_pos(&mut mem(), &crt_trig, i, vec)
    }
    /// `MsGetChrTopPos(i, vec)`.
    MsGetChrTopPos @ 0x0079_5980 => extern "C" fn ms_get_chr_top_pos(i: u32, vec: u32) -> i32 {
        target::ms_get_chr_top_pos(&mut mem(), &crt_trig, i, vec)
    }
    /// `0x007a0bb0(kind, i, vec)` (`MsSEP3DPos`).
    MsSEP3DPos @ 0x007a_0bb0 => extern "C" fn ms_sep_3d_pos(kind: u32, i: u32, vec: u32) {
        afterdmg::ms_sep_3d_pos(&mut mem(), &crt_trig, kind, i, vec);
    }
    /// `MsSEP3Dregist(kind, i, se, once, pos)`: the pan and the sound through the game.
    MsSEP3Dregist @ 0x007a_0a30 => extern "C" fn ms_sep_3d_regist(kind: u32, i: u32, se: u32, once: u32, pos: u32) -> u32 {
        afterdmg::ms_sep_3d_regist(&mut mem(), &crt_trig, &mut call_ptr, kind, i, se, once, pos)
    }
    /// `MsRegSEplay(i, se)`.
    MsRegSEplay @ 0x007a_0100 => extern "C" fn ms_reg_se_play(i: u32, se: u32) -> u32 {
        afterdmg::ms_reg_se_play(&mut mem(), &crt_trig, &mut call_ptr, i, se)
    }
    /// `MsRegSEplay2(i, se)`.
    MsRegSEplay2 @ 0x007a_0140 => extern "C" fn ms_reg_se_play2(i: u32, se: u32) -> u32 {
        afterdmg::ms_reg_se_play2(&mut mem(), &crt_trig, &mut call_ptr, i, se)
    }
    /// `MsStoneProcess(i, actor)`.
    MsStoneProcess @ 0x0078_e150 => extern "C" fn ms_stone_process(i: u32, actor: u32) -> u32 {
        afterdmg::ms_stone_process(&mut mem(), &crt_trig, &mut call_ptr, i, actor)
    }
    /// `MsStatusDefenseEffect(i, j, bits)`.
    MsStatusDefenseEffect @ 0x0079_ee20 => extern "C" fn ms_status_defense_effect(i: u32, j: u32, bits: u32) -> u32 {
        afterdmg::ms_status_defense_effect(&mut mem(), &mut call_ptr, i, j, bits)
    }
    /// `MsLimitTypeDamageCheck(i, by, j, actor, hp, raw, crit)`.
    MsLimitTypeDamageCheck @ 0x007b_0d50 => extern "C" fn ms_limit_type_damage_check(i: u32, by: u32, j: u32, actor: u32, hp: i32, raw: i32, crit: u32) -> u32 {
        afterdmg::ms_limit_type_damage_check(&mut mem(), i, by, j, actor, hp, raw, crit)
    }
    /// `MsLimitTypeStatusCheck(i, by, j, actor, n, bits)`.
    MsLimitTypeStatusCheck @ 0x007b_12c0 => extern "C" fn ms_limit_type_status_check(i: u32, by: u32, j: u32, actor: u32, n: i32, bits: u32) -> u32 {
        afterdmg::ms_limit_type_status_check(&mut mem(), i, by, j, actor, n, bits)
    }
    /// `MsAutoCureProcess(i, actor, by, poison, zombie, dark, silence)`.
    MsAutoCureProcess @ 0x007b_2510 => extern "C" fn ms_auto_cure_process(i: u32, actor: u32, by: u32, poison: u32, zombie: u32, dark: u32, silence: u32) -> u32 {
        let mut m = mem();
        script!(s);
        afterdmg::ms_auto_cure_process(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor, by, poison, zombie, dark, silence)
    }
    /// `MsAutoPotionProcess(i, actor, by)`.
    MsAutoPotionProcess @ 0x007b_2850 => extern "C" fn ms_auto_potion_process(i: u32, actor: u32, by: u32) -> u32 {
        let mut m = mem();
        script!(s);
        afterdmg::ms_auto_potion_process(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor, by)
    }
    /// `AfterDamageProcess(i, k, j, out, flags)`: the sign window, the
    /// effects, sounds and the achievement through the game.
    AfterDamageProcess @ 0x0078_f060 => extern "C" fn after_damage_process(i: u32, k: u32, j: u32, out: u32, flags: u32) -> u32 {
        let mut m = mem();
        script!(s);
        afterdmg::after_damage_process(&mut target::Dm::new(&mut m), &host(), &mut s, i, k, j, out, flags)
    }
    /// `MsSetDamage(i, sel, flags)`: its warded-status local in the
    /// port's frame.
    MsSetDamage @ 0x0078_d980 => extern "C" fn ms_set_damage(i: u32, sel: u32, flags: u32) -> u32 {
        let mut m = mem();
        script!(s);
        afterdmg::ms_set_damage(&mut target::Dm::new(&mut m), &host(), &mut s, i, sel, flags)
    }
    /// `0x007a0b90(se)` (`MsSEP3DFind`).
    MsSEP3DFind @ 0x007a_0b90 => extern "C" fn ms_sep_3d_find(se: u32) -> u32 {
        dmgend::ms_sep_3d_find(&mem(), se)
    }
    /// `MsSEP3DfadeOut(se, frames)`.
    MsSEP3DfadeOut @ 0x007a_08a0 => extern "C" fn ms_sep_3d_fade_out(se: u32, frames: u32) -> u32 {
        dmgend::ms_sep_3d_fade_out(&mut mem(), se, frames)
    }
    /// `MsRegSEfade(i, se, frames)`.
    MsRegSEfade @ 0x007a_00d0 => extern "C" fn ms_reg_se_fade(i: u32, se: u32, frames: u32) -> i32 {
        dmgend::ms_reg_se_fade(&mut mem(), i, se, frames)
    }
    /// `MsAtelRequestCancel(kind, i, n)`: the script calls through the
    /// game, the handler's base in the port's frame.
    MsAtelRequestCancel @ 0x0079_7cd0 => extern "C" fn ms_atel_request_cancel(kind: u32, i: u32, n: i32) -> i32 {
        let mut m = mem();
        script!(s);
        dmgend::ms_atel_request_cancel(&mut target::Dm::new(&mut m), &mut s, kind, i, n)
    }
    /// `MsEffectFree(i)`: script, vfx, sound, the CRT's free and the
    /// effect's own function through the game.
    MsEffectFree @ 0x0078_7af0 => extern "C" fn ms_effect_free(i: u32) {
        let mut m = mem();
        script!(s);
        dmgend::ms_effect_free(&mut target::Dm::new(&mut m), &mut s, i);
    }
    /// `MsSetChrPosAfterSummon(i)`: its position vector in the port's
    /// frame.
    MsSetChrPosAfterSummon @ 0x007a_c6a0 => extern "C" fn ms_set_chr_pos_after_summon(i: u32) {
        let mut m = mem();
        script!(s);
        dmgend::ms_set_chr_pos_after_summon(&mut target::Dm::new(&mut m), &host(), &mut s, i);
    }
    /// `MsDebugFullCommand_00796dc0(i, actor, flags)` (`MsChrDispose`):
    /// chara's disposal and `Sg_FreeA` through the game.
    MsDebugFullCommand_00796dc0 @ 0x0079_6dc0 => extern "C" fn ms_chr_dispose(i: u32, actor: u32, flags: u32) -> u32 {
        let mut m = mem();
        script!(s);
        dmgend::ms_chr_dispose(&mut target::Dm::new(&mut m), &mut s, i, actor, flags)
    }
    /// `MsWeaponDispose(i, k, flags, weapon)`.
    MsWeaponDispose @ 0x007a_d750 => extern "C" fn ms_weapon_dispose(i: u32, k: u32, flags: u32, weapon: u32) {
        let mut m = mem();
        script!(s);
        dmgend::ms_weapon_dispose(&mut target::Dm::new(&mut m), &mut s, i, k, flags, weapon);
    }
    /// `MsBtlChrFree(i, flags)`.
    MsBtlChrFree @ 0x0079_3410 => extern "C" fn ms_btl_chr_free(i: u32, flags: u32) {
        let mut m = mem();
        script!(s);
        dmgend::ms_btl_chr_free(&mut target::Dm::new(&mut m), &mut s, i, flags);
    }
    /// `0x007ae0a0(i)` (`MsFreePlayer`).
    MsFreePlayer @ 0x007a_e0a0 => extern "C" fn ms_free_player(i: u32) {
        let mut m = mem();
        script!(s);
        dmgend::ms_free_player(&mut target::Dm::new(&mut m), &host(), &mut s, i);
    }
    /// `MsExchangePlayer()`.
    MsExchangePlayer @ 0x007a_df10 => extern "C" fn ms_exchange_player() {
        let mut m = mem();
        script!(s);
        dmgend::ms_exchange_player(&mut target::Dm::new(&mut m), &host(), &mut s);
    }
    /// `MsReturnSummon(revive)`.
    MsReturnSummon @ 0x007a_ef20 => extern "C" fn ms_return_summon(revive: u32) {
        let mut m = mem();
        script!(s);
        dmgend::ms_return_summon(&mut target::Dm::new(&mut m), &host(), &mut s, revive);
    }
    /// `MsBattleProcess_ChrDamage_007911e0()`.
    MsBattleProcess_ChrDamage_007911e0 @ 0x0079_11e0 => extern "C" fn ms_battle_process_chr_damage() {
        let mut m = mem();
        script!(s);
        dmgend::ms_battle_process_chr_damage(&mut target::Dm::new(&mut m), &host(), &mut s);
    }
    /// `MsEffectForceFree(id)`: the overlay table's function or vfx.
    MsEffectForceFree @ 0x0078_7ab0 => extern "C" fn ms_effect_force_free(id: u32) -> u32 {
        bend::ms_effect_force_free(&mut mem(), &mut call_ptr, id)
    }
    /// `MsEffectSepClearAll()`.
    MsEffectSepClearAll @ 0x0078_82b0 => extern "C" fn ms_effect_sep_clear_all() -> u32 {
        bend::ms_effect_sep_clear_all(&mut mem(), &mut call_ptr)
    }
    /// `MsEventEffectRead(id)`.
    MsEventEffectRead @ 0x0078_8c60 => extern "C" fn ms_event_effect_read(id: u32) -> i32 {
        bend::ms_event_effect_read(&mut mem(), &mut call_ptr, id)
    }
    /// `MsEventEffectStart(b)`.
    MsEventEffectStart @ 0x0078_8d60 => extern "C" fn ms_event_effect_start(b: u32) -> i32 {
        bend::ms_event_effect_start(&mut mem(), &mut call_ptr, b)
    }
    /// `0x00793290()` (`MsSetEndDirAll`).
    MsSetEndDirAll @ 0x0079_3290 => extern "C" fn ms_set_end_dir_all() {
        bend::ms_set_end_dir_all(&mut mem());
    }
    /// `MsAtelRequestSystemCheckExe(v, flags, actor, out)`.
    MsAtelRequestSystemCheckExe @ 0x0079_82a0 => extern "C" fn ms_atel_request_system_check_exe(v: u32, flags: u32, actor: u32, out: u32) -> i32 {
        script!(s);
        bend::ms_atel_request_system_check_exe(&mut mem(), &mut s, v, flags, actor, out)
    }
    /// `MsSEPbattleClear()`.
    MsSEPbattleClear @ 0x007a_0370 => extern "C" fn ms_sep_battle_clear() {
        bend::ms_sep_battle_clear(&mut mem(), &mut call_ptr);
    }
    /// `MsMusicReadStart()`: `SndMusicRead`'s result.
    MsMusicReadStart @ 0x0079_fff0 => extern "C" fn ms_music_read_start() -> u32 {
        bend::ms_music_read_start(&mut mem(), &mut call_ptr)
    }
    /// `MsMusicSync(music)`.
    MsMusicSync @ 0x007a_0040 => extern "C" fn ms_music_sync(music: u32) -> i32 {
        bend::ms_music_sync(&mut mem(), &mut call_ptr, music)
    }
    /// `MsLimitTypeWinCheck()`.
    MsLimitTypeWinCheck @ 0x007b_1540 => extern "C" fn ms_limit_type_win_check() -> u32 {
        bend::ms_limit_type_win_check(&mut mem())
    }
    /// `MsDeathYoujinbou(i)`.
    MsDeathYoujinbou @ 0x007b_2ad0 => extern "C" fn ms_death_youjinbou(i: u32) -> i32 {
        let mut m = mem();
        script!(s);
        bend::ms_death_youjinbou(&mut target::Dm::new(&mut m), &host(), &mut s, i)
    }
    /// `MsBattleProcess_Status_007917d0()`.
    MsBattleProcess_Status_007917d0 @ 0x0079_17d0 => extern "C" fn ms_battle_process_status() {
        let mut m = mem();
        script!(s);
        bend::ms_battle_process_status(&mut target::Dm::new(&mut m), &host(), &mut s);
    }
    /// `MsEventEffectProcess()`.
    MsEventEffectProcess @ 0x0078_8c00 => extern "C" fn ms_event_effect_process() {
        script!(s);
        bend::ms_event_effect_process(&mut mem(), &mut s);
    }
    /// `MsBtlReadSceneFree()`.
    MsBtlReadSceneFree @ 0x0078_3e00 => extern "C" fn ms_btl_read_scene_free() {
        bend::ms_btl_read_scene_free(&mut mem(), &mut call_ptr);
    }
    /// `MsSEPtransProcess()`.
    MsSEPtransProcess @ 0x007a_0540 => extern "C" fn ms_sep_trans_process() -> i32 {
        bend::ms_sep_trans_process(&mut mem(), &mut call_ptr)
    }
    /// `MsSEP3Dprocess()`: `op_3d_vol_pan_get`'s volume in the shim's
    /// frame.
    MsSEP3Dprocess @ 0x007a_0930 => extern "C" fn ms_sep_3d_process() -> u32 {
        bend::ms_sep_3d_process(&mut mem(), &crt_trig, &mut call_ptr, &mut sep_vol_pan)
    }
    /// `ms_motion_trans(actor, kind)`.
    ms_motion_trans @ 0x007a_8c80 => extern "C" fn ms_motion_trans(actor: u32, kind: u32) -> u32 {
        effect::ms_motion_trans(&mut mem(), &mut call_ptr, actor, kind)
    }
    /// `0x00787650(i, actor)`.
    MsEffectMotionDone_00787650 @ 0x0078_7650 => extern "C" fn ms_effect_motion_done(_i: u32, actor: u32) -> u32 {
        effect::ms_effect_motion_done(&mut mem(), &mut call_ptr, actor)
    }
    /// `0x00787d70(i, actor)`.
    MsEffectBackToIdle_00787d70 @ 0x0078_7d70 => extern "C" fn ms_effect_back_to_idle(i: u32, actor: u32) -> i32 {
        let mut m = mem();
        script!(s);
        effect::ms_effect_back_to_idle(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor)
    }
    /// `0x00788ef0(actor)`.
    MsEffectCamRegist_00788ef0 @ 0x0078_8ef0 => extern "C" fn ms_effect_cam_regist(actor: u32) -> u32 {
        script!(s);
        effect::ms_effect_cam_regist(&mut mem(), &mut s, actor)
    }
    /// `0x00788f30(i, actor)`.
    MsEffectCameraEnd2_00788f30 @ 0x0078_8f30 => extern "C" fn ms_effect_camera_end2(i: u32, actor: u32) -> u32 {
        script!(s);
        effect::ms_effect_camera_end2(&mut mem(), &mut s, i, actor)
    }
    /// `0x00788f90(actor, x, motion, flag)`.
    MsEffectEnd_00788f90 @ 0x0078_8f90 => extern "C" fn ms_effect_end(actor: u32, x: u32, motion: u32, flag: u32) -> i32 {
        let mut m = mem();
        script!(s);
        effect::ms_effect_end(&mut target::Dm::new(&mut m), &host(), &mut s, actor, x, motion, flag)
    }
    /// `0x00789030(i, actor)`.
    MsEffectReflect_00789030 @ 0x0078_9030 => extern "C" fn ms_effect_reflect(i: u32, actor: u32) {
        let mut m = mem();
        script!(s);
        effect::ms_effect_reflect(&mut target::Dm::new(&mut m), &host(), &mut s, i, actor);
    }
    /// `MsStatusEffectReflect(i)`.
    MsStatusEffectReflect @ 0x0079_f160 => extern "C" fn ms_status_effect_reflect(i: u32) -> u32 {
        effect::ms_status_effect_reflect(&mut mem(), &mut call_ptr, i)
    }
    /// `MsEffectReady(id, x)`.
    MsEffectReady @ 0x0078_8200 => extern "C" fn ms_effect_ready(id: u32, x: u32) -> u32 {
        let mut m = mem();
        script!(s);
        effect::ms_effect_ready(&mut target::Dm::new(&mut m), &host(), &mut s, id, x)
    }
    /// `MsEffectLimitStart(i)`.
    MsEffectLimitStart @ 0x007a_fd30 => extern "C" fn ms_effect_limit_start(i: u32) -> u32 {
        script!(s);
        effect::ms_effect_limit_start(&mut mem(), &mut s, i)
    }
    /// `MsEffectLimitCheck(i, actor)`.
    MsEffectLimitCheck @ 0x007a_fc90 => extern "C" fn ms_effect_limit_check(i: u32, actor: u32) -> u32 {
        script!(s);
        effect::ms_effect_limit_check(&mut mem(), &mut s, i, actor)
    }
    /// `MsAtelActorWorkInit(kind, i, n)`.
    MsAtelActorWorkInit @ 0x0079_7490 => extern "C" fn ms_atel_actor_work_init(kind: u32, i: u32, n: u32) -> u32 {
        effect::ms_atel_actor_work_init(&mut mem(), &mut call_ptr, kind, i, n)
    }
    /// `MsAtelSetUp(kind)`.
    MsAtelSetUp @ 0x0079_8390 => extern "C" fn ms_atel_set_up(kind: u32) -> u32 {
        effect::ms_atel_set_up(&mut mem(), &mut call_ptr, kind)
    }
    /// `MsAtelCamRegist(file)`.
    MsAtelCamRegist @ 0x0079_7550 => extern "C" fn ms_atel_cam_regist(file: u32) -> u32 {
        script!(s);
        effect::ms_atel_cam_regist(&mut mem(), &mut s, file)
    }
    /// `MsEffectStart()`.
    MsEffectStart @ 0x0078_8480 => extern "C" fn ms_effect_start() {
        let mut m = mem();
        script!(s);
        effect::ms_effect_start(&mut target::Dm::new(&mut m), &host(), &mut s);
    }
    /// `0x007987f0(i)`.
    MsGetAbilityMap_007987f0 @ 0x0079_87f0 => extern "C" fn ms_get_ability_map(i: u32) -> u32 {
        sparam::ms_get_ability_map(&mem(), i)
    }
    /// `MsGetChrAbilityMap(i, out)`.
    MsGetChrAbilityMap @ 0x0078_5b60 => extern "C" fn ms_get_chr_ability_map(i: u32, out: u32) -> u32 {
        sparam::ms_get_chr_ability_map(&mut mem(), i, out)
    }
    /// `MsSetSaveParam(i)`.
    MsSetSaveParam @ 0x0078_60f0 => extern "C" fn ms_set_save_param(i: u32) -> u32 {
        sparam::ms_set_save_param(&mut mem(), i)
    }
    /// `MsSetSaveParamAll()`.
    MsSetSaveParamAll @ 0x0078_6900 => extern "C" fn ms_set_save_param_all() -> u32 {
        sparam::ms_set_save_param_all(&mut mem())
    }
    /// `MsSetSaveData()`.
    MsSetSaveData @ 0x0078_5fc0 => extern "C" fn ms_set_save_data() -> u32 {
        sparam::ms_set_save_data(&mut mem())
    }
    /// `MsEquipSaveWeapon(i, k, id)`.
    MsEquipSaveWeapon @ 0x007a_b970 => extern "C" fn ms_equip_save_weapon(i: u32, k: u32, id: u32) {
        sparam::ms_equip_save_weapon(&mut mem(), i, k, id);
    }
    /// `0x007ad570(i, k, frames)`.
    MsWeaponFadeIn_007ad570 @ 0x007a_d570 => extern "C" fn ms_weapon_fade_in(i: u32, k: u32, frames: u32) {
        sparam::ms_weapon_fade_in(&mut mem(), &mut call_ptr, i, k, frames);
    }
    /// `MsWeaponRead(i, k, id)`.
    MsWeaponRead @ 0x007a_d820 => extern "C" fn ms_weapon_read(i: u32, k: u32, id: u32) {
        let mut m = mem();
        script!(s);
        sparam::ms_weapon_read(&mut target::Dm::new(&mut m), &mut s, i, k, id);
    }
    /// `MsMessageCueProcess()`.
    MsMessageCueProcess @ 0x0079_cdf0 => extern "C" fn ms_message_cue_process() -> u32 {
        bend::ms_message_cue_process(&mut mem(), &crt_trig, &mut call_ptr)
    }
    /// `MsSetCheckChange(id, side, step, mode)`.
    MsSetCheckChange @ 0x007c_0790 => extern "C" fn ms_set_check_change(id: u32, side: u32, step: u32, mode: u32) -> u32 {
        camera::ms_set_check_change(&mut mem(), &mut call_ptr, id, side, step, mode)
    }
    /// `MsCameraSetRect(id, side, p)`.
    MsCameraSetRect @ 0x007b_f940 => extern "C" fn ms_camera_set_rect(id: u32, side: u32, p: u32) -> u32 {
        camera::ms_camera_set_rect(&mut mem(), &mut call_ptr, id, side, p)
    }
    /// `MsCameraResetMove(id, flags)`.
    MsCameraResetMove @ 0x007b_ed60 => extern "C" fn ms_camera_reset_move(id: u32, flags: u32) -> u32 {
        camera::ms_camera_reset_move(&mut mem(), &mut call_ptr, id, flags)
    }
    /// `MsAtelResetMoveCameraID(kind)`.
    MsAtelResetMoveCameraID @ 0x007b_a860 => extern "C" fn ms_atel_reset_move_camera_id(kind: u32) -> u32 {
        camera::ms_atel_reset_move_camera_id(&mut mem(), &mut call_ptr, kind)
    }
    /// `MsCheckNeckMotion(actor)`.
    MsCheckNeckMotion @ 0x007a_b700 => extern "C" fn ms_check_neck_motion(actor: u32) -> u32 {
        predict::ms_check_neck_motion(&mem(), actor)
    }
    /// `MsGetMoveTargetDist(i, kind, with, speed, n, accel, brake, a, b, c,
    /// gravity)`; the caller's `ebx` is not known here.
    MsGetMoveTargetDist @ 0x007a_b540 => extern "C" fn ms_get_move_target_dist(i: u32, kind: u32, with: u32, speed: u32, n: u32, accel: u32, brake: u32, a: u32, b: u32, c: u32, gravity: u32) {
        let mut m = mem();
        predict::ms_get_move_target_dist(&mut target::Dm::new(&mut m), &host(), i, kind, with, speed, n, accel, brake, [a, b, c], gravity, None);
    }
    /// `MsGetChrPredictPos(i)`: one frame of actor `i`'s move; the
    /// caller's registers (the radius words) are not known here.
    MsGetChrPredictPos @ 0x007a_a2a0 => extern "C" fn ms_get_chr_predict_pos(i: u32) {
        let mut m = mem();
        predict::ms_get_chr_predict_pos(&mut target::Dm::new(&mut m), &host(), i, None);
    }
    /// `MsMoveChr()`: every actor's move step and neck, each frame.
    MsMoveChr @ 0x007a_ab80 => extern "C" fn ms_move_chr() {
        let mut m = mem();
        predict::ms_move_chr(&mut target::Dm::new(&mut m), &host());
    }
    /// `MsSetBallSpline(i)`: rebuilds move spline 2 for actor `i`'s ball;
    /// the caller's registers (the by-value vectors' spare words) are not
    /// known here.
    MsSetBallSpline @ 0x007b_78c0 => extern "C" fn ms_set_ball_spline(i: u32) {
        let mut m = mem();
        predict::ms_set_ball_spline(&mut target::Dm::new(&mut m), &host(), i, None);
    }
    /// `MsMoveSplineRegist(k, i, kind, point)`; the caller's `ebx` is not
    /// known here.
    MsMoveSplineRegist @ 0x007a_b0b0 => extern "C" fn ms_move_spline_regist(k: u32, i: u32, kind: u32, point: u32) {
        let mut m = mem();
        predict::ms_move_spline_regist(&mut target::Dm::new(&mut m), &host(), k, i, kind, point, None);
    }
    /// `MsCameraMoveCalc(near, value, start, mv)`: the CRT's `_CIcos`.
    MsCameraMoveCalc @ 0x007b_da80 => extern "C" fn ms_camera_move_calc(near: u32, value: u32, start: u32, mv: u32) -> u32 {
        camera::ms_camera_move_calc(&mut mem(), &crt_cos, near, value, start, mv)
    }
    /// `MsBtlChrFreeMem()`: the heap through the game.
    MsBtlChrFreeMem @ 0x0079_3500 => extern "C" fn ms_btl_chr_free_mem() {
        free::ms_btl_chr_free_mem(&mut mem(), &mut call_ptr);
    }
    /// `MsNumberFreeMem()`: the heap through the game.
    MsNumberFreeMem @ 0x0079_f990 => extern "C" fn ms_number_free_mem() {
        free::ms_number_free_mem(&mut mem(), &mut call_ptr);
    }
    /// `MsAtelRegistFree()`: the heap through the game, `AtelRemoveCtrlWork`
    /// (script).
    MsAtelRegistFree @ 0x0079_7a90 => extern "C" fn ms_atel_regist_free() {
        free::ms_atel_regist_free(&mut mem(), &mut call_ptr, ffx_script::vm::bridge::atel_remove_ctrl_work);
    }
    /// `MsEtEffectStop()`: `op_et_battle_effect_free` (vfx).
    MsEtEffectStop @ 0x0079_e7b0 => extern "C" fn ms_et_effect_stop() {
        free::ms_et_effect_stop(&mut mem(), &mut call_ptr);
    }
    /// `MsBtlScriptRecordFree_00782640(i)`: `Sg_FreeA` (main).
    MsBtlScriptRecordFree_00782640 @ 0x0078_2640 => extern "C" fn ms_btl_script_record_free(i: u32) {
        free::ms_btl_script_record_free(&mut mem(), &mut call_ptr, i);
    }
    /// `MsEffectEnvFree()`: the heap through the game.
    MsEffectEnvFree @ 0x0078_7a40 => extern "C" fn ms_effect_env_free() {
        free::ms_effect_env_free(&mut mem(), &mut call_ptr);
    }
    /// `MsEffectBufferFree_00782750()`: the heap through the game.
    MsEffectBufferFree_00782750 @ 0x0078_2750 => extern "C" fn ms_effect_buffer_free() {
        free::ms_effect_buffer_free(&mut mem(), &mut call_ptr);
    }
    /// `MsBattleSpecialFree()`: `Yn_SeaResetMatrix` (map) and the heap
    /// through the game.
    MsBattleSpecialFree @ 0x0079_dde0 => extern "C" fn ms_battle_special_free() {
        free::ms_battle_special_free(&mut mem(), &mut call_ptr);
    }
    /// `MsBtlFreeItemMem()`: the heap through the game.
    MsBtlFreeItemMem @ 0x0079_9f70 => extern "C" fn ms_btl_free_item_mem() {
        free::ms_btl_free_item_mem(&mut mem(), &mut call_ptr);
    }
    /// `MsMemCheckFree()`: the heap through the game.
    MsMemCheckFree @ 0x0079_9ec0 => extern "C" fn ms_mem_check_free() {
        free::ms_mem_check_free(&mut mem(), &mut call_ptr);
    }
    /// `MsMenuFreeText(i)`: the heap through the game; the original's
    /// clear of its own argument slots (dead caller stack) is not done.
    MsMenuFreeText @ 0x0078_fb00 => extern "C" fn ms_menu_free_text(i: u32) -> i32 {
        free::ms_menu_free_text(&mut mem(), &mut call_ptr, i)
    }
    /// `MsFieldItemFree()`: the heap through the game.
    MsFieldItemFree @ 0x0079_8f90 => extern "C" fn ms_field_item_free() {
        free::ms_field_item_free(&mut mem(), &mut call_ptr);
    }
    /// `MsCalcMotionSpeed(actor)`.
    MsCalcMotionSpeed @ 0x0078_ac20 => extern "C" fn ms_calc_motion_speed(actor: u32) -> u32 {
        motion::ms_calc_motion_speed(&mut mem(), actor)
    }
    /// `MsCheckChrTouch_007ab750(i, actor)`: its two position vectors in
    /// the port's frame; the CRT through its import slots.
    MsCheckChrTouch_007ab750 @ 0x007a_b750 => extern "C" fn ms_check_chr_touch(i: u32, actor: u32) -> i32 {
        let mut m = mem();
        motion::ms_check_chr_touch(&mut target::Dm::new(&mut m), &crt_trig, i, actor)
    }
    /// `MsMotionProcess()`: script's callees through the game.
    MsMotionProcess @ 0x007a_a160 => extern "C" fn ms_motion_process() {
        let mut m = mem();
        script!(s);
        motion::ms_motion_process(&mut target::Dm::new(&mut m), &crt_trig, &mut s);
    }
    /// `MsRotMatrixX(out, m, angle)`: the CRT through its import slots;
    /// the rotation rows' spare lanes are left as they were.
    MsRotMatrixX @ 0x007c_3170 => extern "C" fn ms_rot_matrix_x(out: u32, m: u32, angle: u32) -> u32 {
        vector::ms_rot_matrix(&mut mem(), &crt_trig, vector::Axis::X, out, m, angle)
    }
    /// `MsRotMatrixY(out, m, angle)`: as `MsRotMatrixX`.
    MsRotMatrixY @ 0x007c_3480 => extern "C" fn ms_rot_matrix_y(out: u32, m: u32, angle: u32) -> u32 {
        vector::ms_rot_matrix(&mut mem(), &crt_trig, vector::Axis::Y, out, m, angle)
    }
    /// `MsRotMatrixZ(out, m, angle)`: as `MsRotMatrixX`.
    MsRotMatrixZ @ 0x007c_3790 => extern "C" fn ms_rot_matrix_z(out: u32, m: u32, angle: u32) -> u32 {
        vector::ms_rot_matrix(&mut mem(), &crt_trig, vector::Axis::Z, out, m, angle)
    }
    /// `MsSpecialSeaMatrix_0079df90(chr, id, work)`: `Yn_SeaSetMatrix`
    /// (map) through the game.
    MsSpecialSeaMatrix_0079df90 @ 0x0079_df90 => extern "C" fn ms_special_sea_matrix(chr: u32, id: u32, work: u32) {
        special::ms_special_sea_matrix(&mut mem(), &mut call_ptr, chr, id, work);
    }
    /// `MsSpecialChrMatrix_0079e030(angles, unused)`: its locals in the
    /// port's frame; the CRT through its import slots.
    MsSpecialChrMatrix_0079e030 @ 0x0079_e030 => extern "C" fn ms_special_chr_matrix(angles: u32, _unused: u32) {
        let mut m = mem();
        special::ms_special_chr_matrix(&mut target::Dm::new(&mut m), &crt_trig, angles);
    }
    /// `MsBtlChrFreeAll()`: chara's `Ch_Dispose`, the heap and script's
    /// callees through the game.
    MsBtlChrFreeAll @ 0x0079_3480 => extern "C" fn ms_btl_chr_free_all() {
        let mut m = mem();
        script!(s);
        free::ms_btl_chr_free_all(&mut target::Dm::new(&mut m), &mut s);
    }
    /// `MsChrCreatePacket(actor)`: `Ch_ModelCreatePacket` (chara) through
    /// the game.
    MsChrCreatePacket @ 0x0079_3d90 => extern "C" fn ms_chr_create_packet(actor: u32) -> u32 {
        free::ms_chr_create_packet(&mut mem(), &mut call_ptr, actor)
    }
    /// `MsBattleSpecial()`: its locals (which go to map code) in a frame
    /// on this stack; map's callees through the game, the CRT through its
    /// import slots.
    MsBattleSpecial @ 0x0079_d430 => extern "C" fn ms_battle_special() {
        let mut m = mem();
        let mut frame = super::StackFrame::<{ (special::SPECIAL_LOCALS / 4) as usize }>::new();
        let locals = frame.at();
        special::ms_battle_special(&mut target::Dm::new(&mut m), &mut call_ptr, &crt_trig, locals);
    }
    /// `MsAtelUpdate0_007972e0(ctrl)`: script 0's per-frame update, with
    /// `MsBattleSpecial`'s locals in a frame on this stack.
    MsAtelUpdate0_007972e0 @ 0x0079_72e0 => extern "C" fn ms_atel_update0(ctrl: u32) -> u32 {
        let mut m = mem();
        let mut frame = super::StackFrame::<{ (special::SPECIAL_LOCALS / 4) as usize }>::new();
        let locals = frame.at();
        script!(s);
        special::ms_atel_update0(&mut target::Dm::new(&mut m), &mut s, &crt_trig, locals, ctrl)
    }
    /// `MsBtlBridgeBtl2Event()`: script's event accessors, chara's and
    /// graphic's callees through the game.
    MsBtlBridgeBtl2Event @ 0x0078_21f0 => extern "C" fn ms_btl_bridge_btl2_event() {
        script!(s);
        free::ms_btl_bridge_btl2_event(&mut mem(), &mut s);
    }
    /// `MsBattleFree()`: the other modules' callees through the game.
    MsBattleFree @ 0x0078_1590 => extern "C" fn ms_battle_free() -> u32 {
        let mut m = mem();
        script!(s);
        free::ms_battle_free(&mut target::Dm::new(&mut m), &mut s)
    }
    /// `MsBattleAllFree()`: the other modules' callees through the game;
    /// `MsMenuFreeText`'s stack clear is not done (see its port).
    MsBattleAllFree @ 0x0078_0b40 => extern "C" fn ms_battle_all_free() -> u32 {
        let mut m = mem();
        script!(s);
        free::ms_battle_all_free(&mut target::Dm::new(&mut m), &mut s)
    }
    /// `MsNumberGetMem()`: the heap through the game.
    MsNumberGetMem @ 0x0079_f9b0 => extern "C" fn ms_number_get_mem() {
        getmem::ms_number_get_mem(&mut mem(), &mut call_ptr);
    }
    /// `MsBtlGetItemMem()`: the heap through the game.
    MsBtlGetItemMem @ 0x0079_9fb0 => extern "C" fn ms_btl_get_item_mem() {
        getmem::ms_btl_get_item_mem(&mut mem(), &mut call_ptr);
    }
    /// `MsBtlScriptRecordShift_00782670()`.
    MsBtlScriptRecordShift_00782670 @ 0x0078_2670 => extern "C" fn ms_btl_script_record_shift() {
        getmem::ms_btl_script_record_shift(&mut mem());
    }
    /// `MsBtlEncountEffect_007829b0(kind, &out)`:
    /// `AtelGetWaterBattleEffect` (script) through the game.
    MsBtlEncountEffect_007829b0 @ 0x0078_29b0 => extern "C" fn ms_btl_encount_effect(kind: u32, out: u32) -> u32 {
        getmem::ms_btl_encount_effect(&mut mem(), &mut call_ptr, kind, out)
    }
    /// `MsGetFileSize(dir, dir2, file)`: the CD object through the game.
    MsGetFileSize @ 0x0078_2a20 => extern "C" fn ms_get_file_size(dir: u32, dir2: u32, file: u32) -> u32 {
        getmem::ms_get_file_size(&mut mem(), &mut call_ptr, dir, dir2, file)
    }
    /// `MsBattleExeSetIndex(both, file)`: the CD object through the game.
    MsBattleExeSetIndex @ 0x0078_1550 => extern "C" fn ms_battle_exe_set_index(both: u32, file: u32) -> u32 {
        getmem::ms_battle_exe_set_index(&mut mem(), &mut call_ptr, both, file)
    }
    /// `MsEventEffectFree()`: `yiAssert` (map) through the game.
    MsEventEffectFree @ 0x0078_8b90 => extern "C" fn ms_event_effect_free() -> u32 {
        script!(s);
        getmem::ms_event_effect_free(&mut mem(), &mut s)
    }
    /// `MsBtlChrGetMem()`: the heap through the game.
    MsBtlChrGetMem @ 0x0079_3530 => extern "C" fn ms_btl_chr_get_mem() {
        getmem::ms_btl_chr_get_mem(&mut mem(), &mut call_ptr);
    }
    /// `MsBtlListCDpos(field, &group, &slot)`.
    MsBtlListCDpos @ 0x0079_d0e0 => extern "C" fn ms_btl_list_cd_pos(field: u32, group: u32, slot: u32) -> u32 {
        getmem::ms_btl_list_cd_pos(&mut mem(), field, group, slot)
    }
    /// `MsEtEffectEncount(kind, se)`: vfx and `MsRegSEplay2` through the game.
    MsEtEffectEncount @ 0x0079_e5e0 => extern "C" fn ms_et_effect_encount(kind: u32, se: u32) -> u32 {
        getmem::ms_et_effect_encount(&mut mem(), &mut call_ptr, kind, se)
    }
    /// `MsEtEffectSet()`: vfx through the game.
    MsEtEffectSet @ 0x0079_e7a0 => extern "C" fn ms_et_effect_set() -> u32 {
        getmem::ms_et_effect_set(&mut mem(), &mut call_ptr)
    }
    /// `MsBattleExe(mode, field, group, slot)`: the other modules' callees
    /// and `MsBtlReadManage` through the game; `group`, `slot` (outputs of
    /// `MsBtlListCDpos` in the original's own argument slots) in locals.
    MsBattleExe @ 0x0078_1020 => extern "C" fn ms_battle_exe(mode: u32, field: u32, group: u32, slot: u32) -> u32 {
        let mut m = mem();
        script!(s);
        exe::ms_battle_exe(&mut target::Dm::new(&mut m), &mut s, mode, field, group, slot)
    }
    /// `MsBtlReadScene(file)`: the CD object and `Sg_MallocA` through the
    /// game.
    MsBtlReadScene @ 0x0078_3d80 => extern "C" fn ms_btl_read_scene(file: u32) -> u32 {
        read::ms_btl_read_scene(&mut mem(), &mut call_ptr, file)
    }
    /// `MsBtlReadManage()`: the map's, graphics', sound's, script's and
    /// chara's callees, the CD object and `MsBtlReadSetScene`,
    /// `MsBtlReadMonster` through the game.
    MsBtlReadManage @ 0x0078_3020 => extern "C" fn ms_btl_read_manage() {
        let mut m = mem();
        script!(s);
        read::ms_btl_read_manage(&mut target::Dm::new(&mut m), &mut s, &host());
    }
    /// `MsMemCheckSum(top)`: the heap and `MsBattleError` through the
    /// game.
    MsMemCheckSum @ 0x0079_9f20 => extern "C" fn ms_mem_check_sum(top: i32) -> u32 {
        init::ms_mem_check_sum(&mut mem(), &mut call_ptr, top)
    }
    /// `MsBtlGetClear_007993e0(p, kind)`.
    MsBtlGetClear_007993e0 @ 0x0079_93e0 => extern "C" fn ms_btl_get_clear(p: u32, kind: u32) -> u32 {
        init::ms_btl_get_clear(&mut mem(), p, kind)
    }
    /// `MsBtlGetInit()`.
    MsBtlGetInit @ 0x0079_8b60 => extern "C" fn ms_btl_get_init() -> u32 {
        init::ms_btl_get_init(&mut mem())
    }
    /// `MsEtEffectNumber_0079f1c0(table, n)`: `MsBattleError` through the
    /// game.
    MsEtEffectNumber_0079f1c0 @ 0x0079_f1c0 => extern "C" fn ms_et_effect_number(table: u32, n: u32) -> u32 {
        init::ms_et_effect_number(&mut mem(), &mut call_ptr, table, n)
    }
    /// `MsEtEffectInit()`.
    MsEtEffectInit @ 0x0079_e670 => extern "C" fn ms_et_effect_init() -> u32 {
        init::ms_et_effect_init(&mut mem(), &mut call_ptr)
    }
    /// `MsFieldItemRead()`: the CD object and the heap through the game.
    MsFieldItemRead @ 0x0079_9020 => extern "C" fn ms_field_item_read() -> u32 {
        init::ms_field_item_read(&mut mem(), &mut call_ptr)
    }
    /// `MsWeaponNameRead()`: `getExcelBinSize` and the heap through the
    /// game.
    MsWeaponNameRead @ 0x007a_2600 => extern "C" fn ms_weapon_name_read() -> u32 {
        init::ms_weapon_name_read(&mut mem(), &mut call_ptr)
    }
    /// `MsImportantRead()`: as `MsWeaponNameRead`.
    MsImportantRead @ 0x0079_08c0 => extern "C" fn ms_important_read() -> u32 {
        init::ms_important_read(&mut mem(), &mut call_ptr)
    }
    /// `MsMenuReadText(i, load)`: the async read's request block in a frame
    /// on this stack (the original's local); the CD object, the heap, the
    /// pointer table and `getExcelBinSize` through the game.
    MsMenuReadText @ 0x0078_fef0 => extern "C" fn ms_menu_read_text(i: u32, load: u32) -> u32 {
        let mut request = super::StackFrame::<{ (init::MENU_TEXT_REQUEST / 4) as usize }>::new();
        let at = request.at();
        init::ms_menu_read_text(&mut mem(), &mut call_ptr, i, load, at)
    }
    /// `MsMenuReadStartText_00782b70(k)`: as `MsMenuReadText`.
    MsMenuReadStartText_00782b70 @ 0x0078_2b70 => extern "C" fn ms_menu_read_start_text(k: u32) -> u32 {
        let mut request = super::StackFrame::<{ (init::MENU_TEXT_REQUEST / 4) as usize }>::new();
        let at = request.at();
        init::ms_menu_read_start_text(&mut mem(), &mut call_ptr, k, at)
    }
    /// `MsBattleSpecialInit(kind)`: the heap and map through the game.
    MsBattleSpecialInit @ 0x0079_de60 => extern "C" fn ms_battle_special_init(kind: u32) -> u32 {
        init::ms_battle_special_init(&mut mem(), &mut call_ptr, kind)
    }
    /// `MsSetSaveStartGame()`: the CD object, main's heap, system, menu,
    /// sphere grid and `MsWeaponNameNum` through the game.
    MsSetSaveStartGame @ 0x0078_6b00 => extern "C" fn ms_set_save_start_game() -> u32 {
        init::ms_set_save_start_game(&mut mem(), &mut call_ptr)
    }
    /// `MsBattleInit()`: the CD texts' request block in a frame on this
    /// stack; the inits, the kernel reader, the heap, `MsBattleExe`, the
    /// scene and the other modules' callees through the game.
    MsBattleInit @ 0x0078_1700 => extern "C" fn ms_battle_init() -> u32 {
        let mut request = super::StackFrame::<{ (init::MENU_TEXT_REQUEST / 4) as usize }>::new();
        let at = request.at();
        init::ms_battle_init(&mut mem(), &mut call_ptr, at)
    }
    /// `MsBattleProcess()`: one frame of the battle module; every callee
    /// (the battle's hooked ports and the other modules') through the game.
    MsBattleProcess @ 0x0079_0c10 => extern "C" fn ms_battle_process() {
        process::ms_battle_process(&mut mem(), &mut call_ptr);
    }
    /// `MsBtlReadMonster()`: the monster file read's request block in a
    /// frame on this stack (the original's local); the CD object, the heap
    /// and the unported monster loaders through the game.
    MsBtlReadMonster @ 0x0078_3680 => extern "C" fn ms_btl_read_monster() -> u32 {
        let mut request = super::StackFrame::<{ (read::MONSTER_REQUEST / 4) as usize }>::new();
        let at = request.at();
        read::ms_btl_read_monster(&mut mem(), &mut call_ptr, at)
    }
    /// `MsGetRamChrCTBbasic()`.
    MsGetRamChrCTBbasic @ 0x0078_4220 => extern "C" fn ms_get_ram_chr_ctb_basic() -> i32 {
        read::ms_get_ram_chr_ctb_basic(&mut mem())
    }
    /// `MsPlaceMonsters_00784420()`: its position vector in the port's
    /// frame; chara through the game.
    MsPlaceMonsters_00784420 @ 0x0078_4420 => extern "C" fn ms_place_monsters() -> u32 {
        let mut m = mem();
        read::ms_place_monsters(&mut target::Dm::new(&mut m), &host(), &mut call_ptr)
    }
    /// `MsCalcChrDirAll()`: the caller's registers (the direction's VU
    /// words) are not known here.
    MsCalcChrDirAll @ 0x0079_3cb0 => extern "C" fn ms_calc_chr_dir_all() {
        let mut m = mem();
        read::ms_calc_chr_dir_all(&mut target::Dm::new(&mut m), &host(), None);
    }
    /// `MsSetChrVisibleEvent(i, flags)`: chara through the game.
    MsSetChrVisibleEvent @ 0x0079_67f0 => extern "C" fn ms_set_chr_visible_event(i: u32, flags: u32) -> u32 {
        read::ms_set_chr_visible_event(&mut mem(), &mut call_ptr, i, flags)
    }
    /// `MsBtlReadSetScene()`: the battle file's sections; the battle's
    /// set-up callees through the game.
    MsBtlReadSetScene @ 0x0078_3e20 => extern "C" fn ms_btl_read_set_scene() -> u32 {
        read::ms_btl_read_set_scene(&mut mem(), &mut call_ptr)
    }
    /// `MsGetItem(chr, actor, rewards, overkill, flags)`.
    MsGetItem @ 0x0079_90d0 => extern "C" fn ms_get_item(chr: u32, actor: u32, rewards: u32, overkill: u32, flags: u32) {
        reward::ms_get_item(&mut mem(), &mut call_ptr, chr, actor, rewards, overkill, flags);
    }
    /// `BattleRewards_AddGear(chr, rewards, spoils)`.
    BattleRewards_AddGear @ 0x0079_8c10 => extern "C" fn battle_rewards_add_gear(chr: u32, rewards: u32, spoils: u32) -> u32 {
        reward::battle_rewards_add_gear(&mut mem(), chr, rewards, spoils)
    }
    /// `BattleRewards_NewGear_00798bd0(spoils)`.
    BattleRewards_NewGear_00798bd0 @ 0x0079_8bd0 => extern "C" fn battle_rewards_new_gear(spoils: u32) -> i32 {
        reward::new_gear(&mut mem(), spoils)
    }
    /// `MsGetAbilityGroup_00798a90(ability)`.
    MsGetAbilityGroup_00798a90 @ 0x0079_8a90 => extern "C" fn ms_get_ability_group(ability: u32) -> u32 {
        reward::ability_group(&mem(), ability)
    }
    /// `MsGearHasAbility_007a0c20(gear, id)`.
    MsGearHasAbility_007a0c20 @ 0x007a_0c20 => extern "C" fn ms_gear_has_ability(at: u32, id: u32) -> u32 {
        gear::ms_gear_has_ability(&mem(), at, id)
    }
    /// `MsWeaponNameNum(gear)`.
    MsWeaponNameNum @ 0x007a_0cf0 => extern "C" fn ms_weapon_name_num(at: u32) -> u32 {
        gear::ms_weapon_name_num(&mem(), at)
    }
    /// `MsWeaponNameNum_Armor_007a0d10(gear)`.
    MsWeaponNameNum_Armor_007a0d10 @ 0x007a_0d10 => extern "C" fn ms_weapon_name_num_armor(at: u32) -> u32 {
        gear::ms_weapon_name_num_armor(&mem(), at)
    }
    /// `MsGetSaveWindow(chr, kind, &count)`: a menu window's list.
    MsGetSaveWindow @ 0x0078_5410 => extern "C" fn ms_get_save_window(chr: u32, kind: u32, count: u32) -> u32 {
        menu::ms_get_save_window(&mut mem(), chr, kind, count)
    }
    /// `MsGetChrStatusBits_0078f930(chr)`.
    MsGetChrStatusBits_0078f930 @ 0x0078_f930 => extern "C" fn ms_get_chr_status_bits(chr: u32) -> u32 {
        menu::ms_get_chr_status_bits(&mem(), chr)
    }
    /// `MsSetRamChrParam(chr)`: a party actor from the save.
    MsSetRamChrParam @ 0x0079_c5f0 => extern "C" fn ms_set_ram_chr_param(chr: u32) -> i32 {
        sparam::ms_set_ram_chr_param(&mut mem(), chr)
    }
    /// `MsGetTargetRandomList(chr, &n, shuffle)`: `MsBattleError`
    /// through the game.
    MsGetTargetRandomList @ 0x007b_7170 => extern "C" fn ms_get_target_random_list(chr: u32, n: u32, shuffle: u32) -> u32 {
        cue::ms_get_target_random_list(&mut mem(), &mut call_ptr, chr, n, shuffle)
    }
    /// `msFieldItemGet(i, spoils)`: `TOGetFFXLangJmp` through the game.
    msFieldItemGet @ 0x0079_9420 => extern "C" fn ms_field_item_get(i: u32, spoils: u32) {
        reward::ms_field_item_get(&mut mem(), &mut call_ptr, i, spoils);
    }
    /// `MsMergeDamage(chr, target)`.
    MsMergeDamage @ 0x0079_aff0 => extern "C" fn ms_merge_damage(chr: u32, target: u32) -> i32 {
        cue::ms_merge_damage(&mut mem(), chr, target)
    }
    /// `MsExchangeSaveParty(chr, slot)`.
    MsExchangeSaveParty @ 0x0078_4d00 => extern "C" fn ms_exchange_save_party(chr: u32, slot: i32) -> u32 {
        save::ms_exchange_save_party(&mut mem(), chr, slot)
    }
    /// `MsWeaponNameNum_Weapon_007a1c00(gear)`.
    MsWeaponNameNum_Weapon_007a1c00 @ 0x007a_1c00 => extern "C" fn ms_weapon_name_num_weapon(at: u32) -> u32 {
        gear::ms_weapon_name_num_weapon(&mem(), at)
    }
    /// `MsMenuGetText(i, id, second)`: a menu text's name.
    MsMenuGetText @ 0x0078_fcf0 => extern "C" fn ms_menu_get_text(i: u32, id: u32, second: u32) -> u32 {
        menu::ms_menu_get_text(&mem(), i, id, second)
    }
    /// `MsMenuGetHelp(i, id, second)`: a menu text's help line.
    MsMenuGetHelp @ 0x0078_fbb0 => extern "C" fn ms_menu_get_help(i: u32, id: u32, second: u32) -> u32 {
        menu::ms_menu_get_help(&mem(), i, id, second)
    }
    /// `MsGetMenuChrInfo(chr, k)`: a name or help line of an actor or ID.
    MsGetMenuChrInfo @ 0x0078_f7c0 => extern "C" fn ms_get_menu_chr_info(chr: u32, k: u32) -> u32 {
        menu::ms_get_menu_chr_info(&mut mem(), chr, k)
    }
    /// `MsEffectTerminate(id)`: an effect's end (`MsBattleError` through
    /// the game).
    MsEffectTerminate @ 0x0078_8920 => extern "C" fn ms_effect_terminate(id: u32) {
        effect::ms_effect_terminate(&mut mem(), &mut call_ptr, id);
    }
    /// `SwapSaveGear(first, second)`: two equipment slots trade places.
    SwapSaveGear @ 0x007a_b9f0 => extern "C" fn swap_save_gear(first: u32, second: u32) -> i32 {
        save::swap_save_gear(&mut mem(), first, second)
    }
    /// `Battle_ScriptPlayMotion(worker, args, stack, flag)`: a script's
    /// actor plays a motion (script and chara calls through the game).
    Battle_ScriptPlayMotion @ 0x007a_8fc0 => extern "C" fn battle_script_play_motion(worker: u32, _args: u32, stack: u32, flag: u32) -> u32 {
        motion::battle_script_play_motion(&mut mem(), &mut call_ptr, worker, stack, flag.to_le_bytes()[0])
    }
    /// `getExcelBinSize(name, buffer)`: a kernel file read whole (its
    /// path in a frame of ours; the language and file layer through the
    /// game).
    getExcelBinSize @ 0x0078_2df0 => extern "C" fn get_excel_bin_size(name: u32, buffer: u32) -> u32 {
        let mut path = super::StackFrame::<{ (init::KERNEL_PATH_SIZE / 4) as usize }>::new();
        init::get_excel_bin_size(&mut mem(), &mut call_ptr, name, buffer, path.at())
    }
    /// `MsMenuLimitSet(chr, user, count)`: an overdrive's hits handed
    /// out (its callees' out values in a frame of ours).
    MsMenuLimitSet @ 0x007a_fe00 => extern "C" fn ms_menu_limit_set(chr: u32, user: u32, count: i32) -> u32 {
        let mut frame = super::StackFrame::<{ limit::LIMIT_SCRATCH_WORDS as usize }>::new();
        limit::ms_menu_limit_set(&mut mem(), &mut call_ptr, chr, user, count, frame.at())
    }
    /// `MsMenuLimitSet_007b0470(chr, count, x, y)`: a minigame's result
    /// taken (the magic camera through the game).
    MsMenuLimitSet_007b0470 @ 0x007b_0470 => extern "C" fn ms_menu_limit_start(chr: u32, count: u32, x: u32, y: u32) -> u32 {
        let mut frame = super::StackFrame::<{ limit::LIMIT_SCRATCH_WORDS as usize }>::new();
        limit::ms_menu_limit_start(&mut mem(), &mut call_ptr, chr, count, x, y, frame.at())
    }
}
