//! The Atel accessors where the game keeps the VM, through
//! [`ffx_bridge::Mem`] (`docs/rewrite-plan.md`, M1). One function per
//! original, with its arguments as the original takes them (addresses as
//! `u32`) and its full `EAX` (or `f32` bits for `ST0`) as the result.

/// How the VM reaches original code: the ctrl work's callbacks, and the
/// callees not ported yet.
pub use ffx_bridge::Host;
use ffx_bridge::{Global, Mem};

pub mod globals;
pub mod helpers;
pub mod native;
pub mod native_btl;
pub mod native_btl2;
pub mod native_cam;
pub mod native_chr;
pub mod native_map;
pub mod native_mods;
pub mod native_move;
pub mod native_msg;
pub mod native_sys;
pub mod run;
pub mod signal;

use super::{
    Classes, block, buffer_size, ctrl, header, is_small, level, level_index, slot_float, slot_int,
    stack, store_float, store_int, value_float, value_int, var, worker, worker_offset,
};

/// `g_ScriptContext`: the current ctrl work.
pub const CONTEXT: Global<u32> = Global::new(0x0132_6b28);
/// `g_AtelCtrlWorks`: the seven ctrl works.
pub const CTRL_WORKS: Global<u8> = Global::new(0x0132_5ba0);
/// `g_AtelError`: zeroed where the PS2 code reported a script error.
pub const ERROR: Global<u32> = Global::new(0x0132_70d8);
/// `g_AtelBasicWorkCacheCtrl`: ctrl work number of the cached worker.
pub const CACHE_CTRL: Global<u16> = Global::new(0x0132_70dc);
/// `g_AtelBasicWorkCacheIndex`: index of the cached worker.
pub const CACHE_INDEX: Global<u16> = Global::new(0x0132_70e0);
/// `g_AtelBasicWorkCache`: the cached worker, 0 for none.
pub const CACHE_WORK: Global<u32> = Global::new(0x0132_70e4);
/// `g_AtelMsgWindows`: two banks of eight message windows (not the
/// system's `g_MsgWindows` at `0x018676f0`).
pub const ATEL_MSG_WINDOWS: Global<u8> = Global::new(0x0132_6d70);
/// `g_AtelMenuExecStatus`: the message window bank in use.
pub const MENU_EXEC_STATUS: Global<i8> = Global::new(0x0132_6b82);
/// The build date `AtelGetVersion` returns.
pub const VERSION: Global<u8> = Global::new(0x00b5_b9fc);

/// Size of a message window.
pub const MSG_WINDOW_SIZE: u32 = 0x2c;

fn ctx(mem: &impl Mem) -> u32 {
    CONTEXT.get(mem)
}

/// The type byte of a worker (the first byte of its header).
fn worker_type(mem: &impl Mem, w: u32) -> u8 {
    let h = mem.read::<u32>(w + worker::HEADER);
    mem.read::<u8>(h)
}

/// `AtelGetBasicWork(index)` (`0x0086a830`): worker `index` of the current
/// ctrl work. An index out of `0..WORKERS` gives worker 0 (and bumps
/// [`ERROR`] past the end). The last result is cached per ctrl work
/// number and index, while [`CACHE_WORK`] is not cleared.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // 16-bit cache keys
pub fn atel_get_basic_work(mem: &mut impl Mem, index: i32) -> u32 {
    let c = ctx(mem);
    atel_get_basic_work_in(mem, c, index)
}

/// [`atel_get_basic_work`] on ctrl work `c` instead of the current one
/// (the copy inlined into `0x008630c0`), with the same cache.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // 16-bit cache keys
pub fn atel_get_basic_work_in(mem: &mut impl Mem, c: u32, index: i32) -> u32 {
    let id = mem.read::<u16>(c + ctrl::ID);
    let cached = CACHE_WORK.get(mem);
    if cached != 0
        && i32::from(id) == i32::from(CACHE_CTRL.get(mem) as i16)
        && index == i32::from(CACHE_INDEX.get(mem) as i16)
    {
        return cached;
    }
    CACHE_CTRL.set(mem, id);
    CACHE_INDEX.set(mem, index.cast_unsigned() as u16);
    let mut i = index;
    if i < 0 {
        i = 0;
        ERROR.set(mem, 0);
    }
    if i >= i32::from(mem.read::<u16>(c + ctrl::WORKERS)) {
        i = 0;
        let e = ERROR.get(mem).wrapping_add(1) & 0xff;
        ERROR.set(mem, e);
    }
    let classes = Classes {
        actors: mem.read(c + ctrl::ACTORS),
        c5b8: mem.read(c + ctrl::CLASS_5B8),
        c558: mem.read(c + ctrl::CLASS_558),
        c2e8: mem.read(c + ctrl::CLASS_2E8),
    };
    let w = mem
        .read::<u32>(c + ctrl::WORK)
        .wrapping_add(worker_offset(i, classes));
    CACHE_WORK.set(mem, w);
    w
}

/// `AtelGetActorType(index)` (`0x0086a7b0`): worker `index`'s type, 7 when
/// the index is outside `0..ACTORS`.
pub fn atel_get_actor_type(mem: &mut impl Mem, index: i32) -> u32 {
    let c = ctx(mem);
    if index < 0 || index >= i32::from(mem.read::<u16>(c + ctrl::ACTORS)) {
        return 7;
    }
    let w = atel_get_basic_work(mem, index);
    u32::from(worker_type(mem, w))
}

/// `AtelGetActorId(worker)` (`0x0086a5d0`).
pub fn atel_get_actor_id(mem: &impl Mem, w: u32) -> u32 {
    u32::from(mem.read::<u16>(w + worker::ID))
}

/// `AtelGetCameraWorkAdrs(worker)` (`0x0086ae50`).
#[must_use]
pub const fn atel_get_camera_work_adrs(w: u32) -> u32 {
    w.wrapping_add(worker::CAMERA)
}

/// `AtelGetBattleIdAdrs2(worker)` (`0x0086a990`).
#[must_use]
pub const fn atel_get_battle_id_adrs2(w: u32) -> u32 {
    w.wrapping_add(worker::BATTLE_ID)
}

/// `AtelGetCharHandle(worker)` (`0x0086ae80`).
pub fn atel_get_char_handle(mem: &impl Mem, w: u32) -> u32 {
    mem.read(w + worker::CHAR_HANDLE)
}

/// `AtelGetPosWorkAdrs(worker)` (`0x0086c2b0`): null for type 4.
pub fn atel_get_pos_work_adrs(mem: &impl Mem, w: u32) -> u32 {
    match worker_type(mem, w) {
        4 => 0,
        k if is_small(k) => w + worker::POS_SMALL,
        _ => w + worker::POS,
    }
}

/// `Script_WorkerLevel(worker, level)` (`0x00872650`): the level block
/// ([`level`]), the index clamped by [`level_index`].
pub fn script_worker_level(mem: &impl Mem, w: u32, lvl: i32) -> u32 {
    let k = worker_type(mem, w);
    w + worker::LEVELS + level_index(k, lvl) * level::SIZE
}

/// `AtelGetActorNumber(block)` (`0x0086a650`).
pub fn atel_get_actor_number(mem: &impl Mem, b: u32) -> u32 {
    u32::from(mem.read::<u16>(b + block::ACTORS))
}

/// `AtelGetNamespaceNumber(block)` (`0x0086c190`).
pub fn atel_get_namespace_number(mem: &impl Mem, b: u32) -> u32 {
    u32::from(mem.read::<u16>(b + block::WORKERS))
}

/// `AtelGetIndexWork(block, i)` (`0x0086bb80`): worker header `i`.
pub fn atel_get_index_work(mem: &impl Mem, b: u32, i: u32) -> u32 {
    let at = b
        .wrapping_add(block::HEADERS)
        .wrapping_add(i.wrapping_mul(4));
    mem.read::<u32>(at).wrapping_add(b)
}

/// `AtelGetBinHdrExpandAddChr(block)` (`0x0086aa20`): the expansion
/// header's extra character count, 0 without one.
pub fn atel_get_bin_hdr_expand_add_chr(mem: &impl Mem, b: u32) -> u32 {
    let off = mem.read::<u32>(b + block::EXPAND);
    let e = b.wrapping_add(off);
    if off == 0 || e == 0 {
        return 0;
    }
    mem.read(e + 4)
}

/// `AtelGetActorBufferSize(block)` (`0x0086a290`).
pub fn atel_get_actor_buffer_size(mem: &impl Mem, b: u32) -> u32 {
    buffer_size(
        mem.read(b + block::WORKERS),
        Classes {
            actors: mem.read(b + block::ACTORS),
            c5b8: mem.read(b + 0x14),
            c558: mem.read(b + 0x16),
            c2e8: mem.read(b + 0x1c),
        },
    )
}

/// `AtelGetCtrlWork(i)` (`0x0086afa0`).
pub fn atel_get_ctrl_work(mem: &impl Mem, i: u32) -> u32 {
    CTRL_WORKS
        .ptr(mem)
        .addr
        .wrapping_add(i.wrapping_mul(ctrl::SIZE))
}

/// `AtelGetCurCtrlWork()` (`0x0086aff0`).
pub fn atel_get_cur_ctrl_work(mem: &impl Mem) -> u32 {
    ctx(mem)
}

/// `AtelChangeCurrentCtrl(i)` (`0x00862d30`): makes ctrl work `i` current;
/// returns the previous one.
pub fn atel_change_current_ctrl(mem: &mut impl Mem, i: u32) -> u32 {
    let w = atel_get_ctrl_work(mem, i);
    atel_set_current_ctrl(mem, w)
}

/// `AtelSetCurrentCtrl(ctrl)` (`0x0086f730`): returns the previous one.
pub fn atel_set_current_ctrl(mem: &mut impl Mem, c: u32) -> u32 {
    let old = ctx(mem);
    CONTEXT.set(mem, c);
    old
}

/// `AtelGetCtrlworkActorTotal(ctrl)` (`0x0086afe0`).
pub fn atel_get_ctrlwork_actor_total(mem: &impl Mem, c: u32) -> u32 {
    u32::from(mem.read::<u16>(c + ctrl::ACTORS))
}

/// `AtelGetNowCtrlTopId()` (`0x0086c1a0`): the player's worker, -1 for
/// none.
pub fn atel_get_now_ctrl_top_id(mem: &impl Mem) -> u32 {
    match mem.read::<u16>(ctx(mem) + ctrl::TOP_ID) {
        0xffff => u32::MAX,
        id => u32::from(id),
    }
}

/// `AtelGetTotalActorNumber2()` (`0x0086c510`): workers of the `0xb58`
/// class in the current ctrl work.
pub fn atel_get_total_actor_number2(mem: &impl Mem) -> u32 {
    let c = ctx(mem);
    let f = |o| u32::from(mem.read::<u16>(c + o));
    f(ctrl::ACTORS)
        .wrapping_sub(f(ctrl::CLASS_2E8))
        .wrapping_sub(f(ctrl::CLASS_558))
        .wrapping_sub(f(ctrl::CLASS_5B8))
}

/// `AtelGetEventActorTotal()` (`0x0086b300`): ctrl work 0's actor count, 0
/// when it is not in use.
pub fn atel_get_event_actor_total(mem: &impl Mem) -> u32 {
    let c = CTRL_WORKS.ptr(mem).addr;
    if mem.read::<u8>(c + ctrl::FLAGS) & 1 == 0 {
        return 0;
    }
    u32::from(mem.read::<u16>(c + ctrl::ACTORS))
}

/// `AtelGetEventMapCameraId()` (`0x0086b5d0`).
pub fn atel_get_event_map_camera_id(mem: &impl Mem) -> u32 {
    let m = mem.read::<u32>(CTRL_WORKS.ptr(mem).addr + ctrl::BLOCK);
    u32::from(mem.read::<u16>(m + 0x18))
}

/// `AtelRemoveCtrlWork(i)` (`0x0086ed40`): marks ctrl work `i` free and
/// drops the worker cache.
pub fn atel_remove_ctrl_work(mem: &mut impl Mem, i: u32) {
    let c = atel_get_ctrl_work(mem, i);
    CACHE_WORK.set(mem, 0);
    let f = mem.read::<u8>(c);
    mem.write(c, f & 0xfe);
    mem.write(c + ctrl::REMOVE_CLEARS, 0u32);
}

/// `AtelGetMesWinWork(i)` (`0x0086be90`): message window `i` (clamped to
/// 0..=7) of the bank [`MENU_EXEC_STATUS`] selects.
#[allow(clippy::cast_sign_loss)] // wrapping, as the original
pub fn atel_get_mes_win_work(mem: &impl Mem, i: i32) -> u32 {
    let bank = i32::from(MENU_EXEC_STATUS.get(mem));
    let n = i.clamp(0, 7).wrapping_add(bank.wrapping_mul(8)) as u32;
    ATEL_MSG_WINDOWS
        .ptr(mem)
        .addr
        .wrapping_add(n.wrapping_mul(MSG_WINDOW_SIZE))
}

// ---- the stack ----

fn kind_at(s: u32, i: i32) -> u32 {
    s.wrapping_add(stack::KINDS).wrapping_add_signed(i)
}

fn value_at(s: u32, i: i32) -> u32 {
    s.wrapping_add(stack::VALUES)
        .wrapping_add_signed(i.wrapping_mul(4))
}

fn push(mem: &mut impl Mem, s: u32, kind: u8, value: u32) {
    let d = mem.read::<i32>(s + stack::DEPTH);
    if d >= stack::SLOTS {
        ERROR.set(mem, 0);
        return;
    }
    mem.write(kind_at(s, d), kind);
    mem.write(value_at(s, d), value);
    mem.write(s + stack::DEPTH, d.wrapping_add(1));
}

/// `AtelPushStackInteger(worker, stack, value)` (`0x0086e3f0`): a full
/// stack drops the value. Like the original, a negative depth is not
/// checked.
pub fn atel_push_stack_integer(mem: &mut impl Mem, s: u32, value: u32) {
    push(mem, s, stack::INT, value);
}

/// `AtelPushStackFloat(worker, stack, value)` (`0x0086e3c0`), the value as
/// `f32` bits (stored as they are).
pub fn atel_push_stack_float(mem: &mut impl Mem, s: u32, bits: u32) {
    push(mem, s, stack::FLOAT, bits);
}

/// The top slot, `(kind, raw)`, and the depth below it; `None` (and
/// [`ERROR`] zeroed) when the stack is empty.
fn top(mem: &mut impl Mem, s: u32) -> Option<(u8, u32, i32)> {
    let d = mem.read::<i32>(s + stack::DEPTH);
    if d <= 0 {
        ERROR.set(mem, 0);
        return None;
    }
    let d = d - 1;
    Some((mem.read(kind_at(s, d)), mem.read(value_at(s, d)), d))
}

/// `AtelPopStackInteger(worker, stack)` (`0x0086df00`): the top as an
/// integer ([`super::slot_int`]); 0 from an empty stack.
pub fn atel_pop_stack_integer(mem: &mut impl Mem, s: u32) -> u32 {
    script_top_int(mem, 1, s)
}

/// `Script_TopInt(worker, pop, stack)` (`0x0086df40`): the top as
/// an integer, popped only when `pop` is not 0.
pub fn script_top_int(mem: &mut impl Mem, pop: u32, s: u32) -> u32 {
    let Some((kind, raw, d)) = top(mem, s) else {
        return 0;
    };
    let v = slot_int(kind, raw);
    if pop != 0 {
        mem.write(s + stack::DEPTH, d);
    }
    v
}

/// `AtelPopStackFloat(worker, stack)` (`0x0086de50`): the top as `f32`
/// bits ([`super::slot_float`]); 0.0 from an empty stack (the original
/// then prints the worker and PC through the empty `dbgPrintf`).
pub fn atel_pop_stack_float(mem: &mut impl Mem, s: u32) -> u32 {
    let Some((kind, raw, d)) = top(mem, s) else {
        return 0;
    };
    mem.write(s + stack::DEPTH, d);
    slot_float(kind, raw)
}

// ---- variables ----

/// Where variable `v`'s descriptor is.
fn descriptor(mem: &impl Mem, w: u32, v: u32) -> u32 {
    let h = mem.read::<u32>(w + worker::HEADER);
    let table = mem
        .read::<u32>(h + header::VARS)
        .wrapping_add(mem.read::<u32>(w + worker::BLOCK));
    table.wrapping_add(v.wrapping_mul(var::SIZE))
}

/// `AtelGetVarInfoAdrs(worker, v)` (`0x0086c850`): variable `v`'s
/// location word.
pub fn atel_get_var_info_adrs(mem: &impl Mem, w: u32, v: u32) -> u32 {
    mem.read(descriptor(mem, w, v) + var::WORD)
}

/// `AtelGetVarType(worker, v)` (`0x0086c870`).
pub fn atel_get_var_type(mem: &impl Mem, w: u32, v: u32) -> u32 {
    var::kind(atel_get_var_info_adrs(mem, w, v))
}

/// `AtelGetVarArrayMax(worker, v)` (`0x0086c7f0`) and
/// `AtelGetVarArrayNumber` (`0x0086c810`, the same code): the array length.
pub fn atel_get_var_array_max(mem: &impl Mem, w: u32, v: u32) -> u32 {
    u32::from(mem.read::<u16>(descriptor(mem, w, v) + var::LENGTH))
}

/// `AtelGetVarArrayRange(worker, v)` (`0x0086c830`).
pub fn atel_get_var_array_range(mem: &impl Mem, w: u32, v: u32) -> u32 {
    u32::from(mem.read::<u16>(descriptor(mem, w, v) + var::RANGE))
}

/// `AtelGetValueIncValue(type)` (`0x0086c6e0`): bytes per element.
#[must_use]
pub const fn atel_get_value_inc_value(kind: u32) -> u32 {
    var::element_size(kind)
}

/// `AtelGetVersion()` (`0x0086c890`): the build date string.
pub fn atel_get_version(mem: &impl Mem) -> u32 {
    VERSION.ptr(mem).addr
}

/// `AtelGetRealVarAdrs(worker, word)` (`0x0086c2e0`): where the variable
/// a location word names is, by its storage area: 0 and 1 the ctrl
/// work's, 2 to 4 the worker header's areas, 5 inside the worker, 6 the
/// block's; storage 3 goes through the ctrl work's callback when it has
/// one. Storage 7 gives 0. (The original then calls
/// `MsGetSaveEventAddress` for ctrl work 0 and drops the result.)
pub fn atel_get_real_var_adrs(host: &mut impl Host, w: u32, word: u32) -> u32 {
    let off = var::offset(word);
    let c = ctx(host);
    // a header area: its offset (relative to the block) + off + the block
    let area = |mem: &dyn Fn(u32) -> u32, field: u32| {
        let base = mem(mem(w + worker::HEADER) + field);
        base.wrapping_add(off).wrapping_add(mem(w + worker::BLOCK))
    };
    match var::storage(word) {
        0 => off.wrapping_add(host.read::<u32>(c + ctrl::STORAGE0)),
        1 => off.wrapping_add(host.read::<u32>(c + ctrl::STORAGE1)),
        2 => area(&|a| host.read(a), header::STORAGE2),
        3 => match host.read::<u32>(c + ctrl::VAR_ADDRESS) {
            0 => area(&|a| host.read(a), header::STORAGE3),
            f => host.call(f, &[w, off]),
        },
        4 => area(&|a| host.read(a), header::STORAGE4),
        5 => w.wrapping_add(worker::STORAGE5).wrapping_add(off),
        6 => {
            let b = host.read::<u32>(w + worker::BLOCK);
            host.read::<u32>(b + block::STORAGE6)
                .wrapping_add(off)
                .wrapping_add(b)
        }
        _ => 0,
    }
}

/// `AtelGetVarAdrs(worker, v)` (`0x0086c7d0`): variable `v`'s address.
pub fn atel_get_var_adrs(host: &mut impl Host, w: u32, v: u32) -> u32 {
    let word = atel_get_var_info_adrs(host, w, v);
    atel_get_real_var_adrs(host, w, word)
}

/// `AtelGetValueAdrs(worker, v, index)` (`0x0086c570`): the address of
/// element `index` ([`var::clamp_index`]) of variable `v`.
#[allow(clippy::cast_sign_loss)] // wrapping, as the original
pub fn atel_get_value_adrs(host: &mut impl Host, w: u32, v: u32, index: i32) -> u32 {
    let d = descriptor(host, w, v);
    let word = host.read::<u32>(d + var::WORD);
    let len = host.read::<u16>(d + var::LENGTH);
    let at = atel_get_real_var_adrs(host, w, word);
    let i = var::clamp_index(index, len) as u32;
    var::element_size(var::kind(word))
        .wrapping_mul(i)
        .wrapping_add(at)
}

/// The element's bytes as the getters read them: the type's width, nothing
/// for the types past `f32`.
fn read_element(mem: &impl Mem, kind: u32, at: u32) -> u32 {
    match kind {
        var::U8 | var::I8 => u32::from(mem.read::<u8>(at)),
        var::U16 | var::I16 => u32::from(mem.read::<u16>(at)),
        var::U32 | var::I32 | var::F32 => mem.read(at),
        _ => 0,
    }
}

fn write_element(mem: &mut impl Mem, at: u32, (bytes, width): (u32, u32)) {
    #[allow(clippy::cast_possible_truncation)] // the width says how much
    match width {
        1 => mem.write(at, bytes as u8),
        2 => mem.write(at, bytes as u16),
        _ => mem.write(at, bytes),
    }
}

/// `AtelGetValueInteger(worker, v, index)` (`0x0086c730`): the element as
/// an integer ([`super::value_int`]).
pub fn atel_get_value_integer(host: &mut impl Host, w: u32, v: u32, index: i32) -> u32 {
    let at = atel_get_value_adrs(host, w, v, index);
    let kind = atel_get_var_type(host, w, v);
    value_int(kind, read_element(host, kind, at))
}

/// `AtelGetValueFloat(worker, v, index)` (`0x0086c610`): the element as
/// `f32` bits ([`super::value_float`]).
pub fn atel_get_value_float(host: &mut impl Host, w: u32, v: u32, index: i32) -> u32 {
    let at = atel_get_value_adrs(host, w, v, index);
    let kind = atel_get_var_type(host, w, v);
    value_float(kind, read_element(host, kind, at))
}

/// `AtelSetValueInteger(worker, v, index, value, clamp)` (`0x00871400`):
/// stores into the element ([`super::store_int`]).
pub fn atel_set_value_integer(
    host: &mut impl Host,
    w: u32,
    v: u32,
    index: i32,
    value: i32,
    clamp: u32,
) {
    let at = atel_get_value_adrs(host, w, v, index);
    let kind = atel_get_var_type(host, w, v);
    if let Some(e) = store_int(kind, value, clamp != 0) {
        write_element(host, at, e);
    }
}

/// `AtelSetValueFloat(worker, v, index, value, clamp)` (`0x008712e0`), the
/// value as `f32` bits ([`super::store_float`]).
pub fn atel_set_value_float(
    host: &mut impl Host,
    w: u32,
    v: u32,
    index: i32,
    bits: u32,
    clamp: u32,
) {
    let at = atel_get_value_adrs(host, w, v, index);
    let kind = atel_get_var_type(host, w, v);
    if let Some(e) = store_float(kind, bits, clamp != 0) {
        write_element(host, at, e);
    }
}

/// `Script_PushVar(worker, stack, v, index)` (`0x0086e440`, opcode `1f`):
/// pushes element `index` of variable `v`, as a float slot for `f32`
/// variables and an integer slot for the rest.
pub fn script_push_var(host: &mut impl Host, w: u32, s: u32, v: u32, index: i32) {
    if atel_get_var_type(host, w, v) == var::F32 {
        let x = atel_get_value_float(host, w, v, index);
        atel_push_stack_float(host, s, x);
    } else {
        let x = atel_get_value_integer(host, w, v, index);
        atel_push_stack_integer(host, s, x);
    }
}

/// `Script_PushVarElem(worker, stack, v)` (`0x0086e100`, opcode `22`):
/// pops an index, then [`script_push_var`].
#[allow(clippy::cast_possible_wrap)]
pub fn script_push_var_elem(host: &mut impl Host, w: u32, s: u32, v: u32) {
    let i = atel_pop_stack_integer(host, s) as i32;
    script_push_var(host, w, s, v, i);
}

/// `Script_StoreVar(worker, stack, v, mode)` (`0x0086dfb0`, opcodes `20`,
/// `21`, `23`, `24`): pops a value (and, with mode bit `0x80`, an index
/// below it) and stores it into variable `v`, saturating with mode bit 0.
/// Returns 0.
#[allow(clippy::cast_possible_wrap)]
pub fn script_store_var(host: &mut impl Host, work: u32, stk: u32, v: u32, mode: u32) -> u32 {
    let clamp = mode & 1;
    let loc = atel_get_var_info_adrs(host, work, v);
    // the original resolves the address once for nothing (the callback runs)
    atel_get_real_var_adrs(host, work, loc);
    if loc & 0xf000_0000 == 0x6000_0000 {
        let value = atel_pop_stack_float(host, stk);
        let at = pop_index(host, stk, mode);
        atel_set_value_float(host, work, v, at, value, clamp);
    } else {
        let value = atel_pop_stack_integer(host, stk).cast_signed();
        let at = pop_index(host, stk, mode);
        atel_set_value_integer(host, work, v, at, value, clamp);
    }
    0
}

/// The array index `Script_StoreVar` pops with mode bit `0x80`, else 0.
fn pop_index(mem: &mut impl Mem, stk: u32, mode: u32) -> i32 {
    if mode & 0x80 == 0 {
        0
    } else {
        atel_pop_stack_integer(mem, stk).cast_signed()
    }
}

/// `Script_PushVarRef(worker, stack, v)` (`0x0086e160`, opcode `27`): pops
/// an index and pushes variable `v`'s location word advanced to that
/// element (a reference in the same encoding, not an address).
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub fn script_push_var_ref(mem: &mut impl Mem, work: u32, stk: u32, v: u32) {
    let desc = descriptor(mem, work, v);
    let loc = mem.read::<u32>(desc + var::WORD);
    let index = atel_pop_stack_integer(mem, stk).cast_signed();
    let len = mem.read::<u16>(desc + var::LENGTH);
    let r = var::element_size(var::kind(loc))
        .wrapping_mul(var::clamp_index(index, len).cast_unsigned())
        .wrapping_add(loc);
    atel_push_stack_integer(mem, stk, r);
}
