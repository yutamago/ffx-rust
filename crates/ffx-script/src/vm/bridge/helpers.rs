//! Atel VM helpers that are not natives (script round 13): the worker's
//! turn blocks, its binary header's tables, the sound and menu-window
//! tables, the bind masks. Each reads and writes memory as the original
//! does and returns the `EAX` it leaves.

// m, w: the memory and the worker, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Mem, x87};

/// `g_ScriptContext` (`0x01326b28`): the current ctrl work.
const SCRIPT_CONTEXT: u32 = 0x0132_6b28;
/// `g_AtelMenuExecStatus` (`0x01326b82`, `i8`): the window bank.
const MENU_EXEC_STATUS: u32 = 0x0132_6b82;
/// The message windows' status words (`0x01326d84`, 0x2c bytes each,
/// eight per bank).
const WINDOWS: u32 = 0x0132_6d84;
/// `g_AtelSnd3dWork` (`0x01328318`): 16 records of 0x24 bytes.
const SND3D_WORK: u32 = 0x0132_8318;
/// The bind masks (`0x01326b98`, four rows of four words) and their union
/// (`0x01326b94`).
const BIND_MASKS: u32 = 0x0132_6b98;
const BIND_MASK_ALL: u32 = 0x0132_6b94;

/// The worker's type (its header's first byte).
fn kind(m: &impl Mem, w: u32) -> u8 {
    let h = m.read::<u32>(w);
    m.read::<u8>(h)
}

/// A level clamped as the turn/move helpers do: negative to 0, at most 1
/// for worker types 5 and 6, else at most 8.
fn clamp_level(m: &impl Mem, w: u32, l: u32) -> u32 {
    let l = l.cast_signed().max(0).cast_unsigned();
    let max = if kind(m, w).wrapping_sub(5) < 2 { 1 } else { 8 };
    l.min(max)
}

/// `fld m32`/`fstp m32` of one word.
fn copy_f32(m: &mut impl Mem, from: u32, to: u32) {
    let v = x87::store(x87::load(m.read(from)));
    m.write(to, v);
}

/// `Script_StopTurn(worker, level)` (`0x00863fe0`; -1 = the current
/// level): every level sharing that level's turn block (`+0x174 +
/// level*0x4c`) gets its own block at `+0x864 + i*0x44`, with `+2` = 0 and
/// `+0x20`..`+0x34`, `+0xc`, `+8` copied from the shared one. Returns the
/// `EAX` the loop leaves (the last block pointer it wrote, or the header
/// pointer with its low byte the worker type minus 5).
pub fn script_stop_turn(m: &mut impl Mem, w: u32, level: u32) -> u32 {
    let level = if level.cast_signed() < 0 {
        u32::from(m.read::<u8>(w + 0x32))
    } else {
        level
    };
    let slot = |l: u32| w + 0x174 + l * 0x4c;
    let l = clamp_level(m, w, level);
    let shared = m.read::<u32>(slot(l));
    let mut eax = 0;
    for i in 0..9 {
        let blk = w + 0x864 + i * 0x44;
        let h = m.read::<u32>(w);
        eax = h & !0xff | u32::from(m.read::<u8>(h).wrapping_sub(5));
        let k = clamp_level(m, w, i);
        if m.read::<u32>(slot(k)) != shared {
            continue;
        }
        m.write(slot(k), blk);
        m.write(blk + 2, 0_u16);
        for off in [0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0xc, 8] {
            let to = m.read::<u32>(slot(k));
            copy_f32(m, shared + off, to + off);
            eax = to;
        }
    }
    eax
}

/// `0x0086f780(worker, a f, b f, c f)`: the current level's `+0x5e0`..
/// `+0x5e8` = a, b, c and `+0x5d4`..`+0x5dc` = the worker's position
/// (`+0x284` for types 5 and 6, else `+0x558`; type 4 reads address 0, as
/// the original does). Returns the `EAX` it leaves, level * 0x4c.
pub fn atel_set_level_vectors(m: &mut impl Mem, w: u32, a: u32, b: u32, c: u32) -> u32 {
    let t = kind(m, w);
    let src = if t == 4 {
        0
    } else if t.wrapping_sub(5) < 2 {
        w + 0x284
    } else {
        w + 0x558
    };
    let off = u32::from(m.read::<u8>(w + 0x32)) * 0x4c;
    let base = w + off;
    for (i, v) in [a, b, c].into_iter().enumerate() {
        let to = base + 0x5e0 + 4 * u32::try_from(i).unwrap_or(0);
        m.write(to, x87::store(x87::load(v)));
    }
    for i in 0..3 {
        copy_f32(m, src + 4 * i, base + 0x5d4 + 4 * i);
    }
    off
}

/// `0x008712a0(worker, on)`: the worker's `+0x34` bit 9 (`0x200`) set or
/// cleared; returns the bit.
pub fn atel_set_worker_flag_200(m: &mut impl Mem, w: u32, on: u32) -> u32 {
    let v = m.read::<u16>(w + 0x34);
    let v = if on != 0 { v | 0x200 } else { v & 0xfdff };
    m.write(w + 0x34, v);
    u32::from(v >> 9 & 1)
}

/// The worker's binary header table at `+0x2c` (relative to the worker
/// block) and its `+0xc` (relative too), 0 when either is 0.
fn bin_table(m: &impl Mem, b: u32) -> Option<u32> {
    let o = m.read::<u32>(b + 0x2c);
    if o == 0 || o.wrapping_add(b) == 0 {
        return None;
    }
    let t = m.read::<u32>(o.wrapping_add(b) + 0xc);
    (t != 0).then_some(t)
}

/// `0x0086ac60(bin)`: the first word of the binary's table (`+0x2c`, then
/// `+0xc`), 0 without one.
pub fn atel_bin_table_count(m: &impl Mem, b: u32) -> u32 {
    bin_table(m, b).map_or(0, |t| m.read(t.wrapping_add(b)))
}

/// `0x0086ac30(bin, i)`: entry i's first word (`+4 + 8i`), 0 without a
/// table.
pub fn atel_bin_table_entry_a(m: &impl Mem, b: u32, i: u32) -> u32 {
    bin_table(m, b).map_or(0, |t| {
        m.read(
            t.wrapping_add(i.wrapping_mul(8))
                .wrapping_add(4)
                .wrapping_add(b),
        )
    })
}

/// `0x0086ac90(bin, i)`: entry i's second word (`+8 + 8i`), 0 without a
/// table.
pub fn atel_bin_table_entry_b(m: &impl Mem, b: u32, i: u32) -> u32 {
    bin_table(m, b).map_or(0, |t| {
        m.read(
            t.wrapping_add(i.wrapping_mul(8))
                .wrapping_add(8)
                .wrapping_add(b),
        )
    })
}

/// `AtelGetSnd3dWork(i)` (`0x008773f0`): record i (negative 0, at most
/// 15) of `g_AtelSnd3dWork`.
pub fn atel_get_snd3d_work(m: &impl Mem, i: u32) -> u32 {
    let i = i.cast_signed().clamp(0, 0xf).cast_unsigned();
    m.rebase(SND3D_WORK) + i * 0x24
}

/// `AtelSoundTableGet_00876860(kind, i)`: byte i of the table for kind 1
/// (`0x00c52ac0`), 2 and 3 (`0x00c52ac8`), 5 and 6 (`0x00c52ad0`); 0xff
/// for any other kind.
pub fn atel_sound_table_get(m: &impl Mem, kind: u32, i: u32) -> u32 {
    let t = match kind {
        1 => 0x00c5_2ac0,
        2 | 3 => 0x00c5_2ac8,
        5 | 6 => 0x00c5_2ad0,
        _ => return 0xff,
    };
    u32::from(m.read::<u8>(m.rebase(t).wrapping_add(i)))
}

/// Message window `i` (clamped to 0..7) of the current bank.
fn window(m: &impl Mem, i: u32) -> u32 {
    let i = i.cast_signed().clamp(0, 7);
    let bank = i32::from(m.read::<i8>(m.rebase(MENU_EXEC_STATUS)));
    let k = i.wrapping_add(bank.wrapping_mul(8)).cast_unsigned();
    m.rebase(WINDOWS).wrapping_add(k.wrapping_mul(0x2c))
}

/// `0x0086be60(i)`: message window i's status word (`u16`).
pub fn atel_get_window_status(m: &impl Mem, i: u32) -> u32 {
    u32::from(m.read::<u16>(window(m, i)))
}

/// `0x0086c8b0()`: over the current bank's eight windows, 2 for the first
/// open one (status 1 or 2) with `+9` bits `0x22`, 1 for one with `+2` set
/// or `+9` bit `0x10`, else 0.
pub fn atel_windows_busy(m: &impl Mem) -> u32 {
    for i in 0..8 {
        let p = window(m, i);
        let st = m.read::<u16>(p);
        if st != 1 && st != 2 {
            continue;
        }
        let f = m.read::<u8>(p + 9);
        if f & 0x22 != 0 {
            return 2;
        }
        if m.read::<u16>(p + 2) != 0 || f & 0x10 != 0 {
            return 1;
        }
    }
    0
}

/// `0x0086c530(strict)`: 1 when the current ctrl work's `+0` has bits
/// `0x22`, unless `strict` and `+1` bit 4 or `+3` bits `0x84` are set.
pub fn atel_ctrl_running(m: &impl Mem, strict: u32) -> u32 {
    let c = m.read::<u32>(m.rebase(SCRIPT_CONTEXT));
    let on = m.read::<u8>(c) & 0x22 != 0;
    if strict != 0 && (m.read::<u8>(c + 1) & 4 != 0 || m.read::<u8>(c + 3) & 0x84 != 0) {
        return 0;
    }
    u32::from(on)
}

/// `AtelAllClearBindMask()` (`0x00860f50`): the sixteen bind-mask words
/// and their union cleared. Returns the `EAX` it leaves (the end address
/// of the loop).
pub fn atel_all_clear_bind_mask(m: &mut impl Mem) -> u32 {
    let base = m.rebase(BIND_MASKS);
    for i in 0..16 {
        m.write(base + 4 * i, 0_u32);
    }
    m.write(m.rebase(BIND_MASK_ALL), 0_u32);
    base + 0x48
}

// ---- round 14 ----

/// The effect flags (`0x01326b70`, `0x01326b74`), the love-parameter
/// mask (`0x01326b90`) and the guide-map flag byte (`0x01325ba3`).
const EFFECT_FLAGS: u32 = 0x0132_6b70;
const EFFECT_FLAGS2: u32 = 0x0132_6b74;
const LOVE_DISABLE: u32 = 0x0132_6b90;
const GUIDE_MAP_FLAGS: u32 = 0x0132_5ba3;
/// `g_AtelForceBattleMapId` (`0x00c5273c`).
const FORCE_BATTLE_MAP: u32 = 0x00c5_273c;
/// The 128 sprite records (`0x01327148`, 0x1c bytes each).
const SPRITES: u32 = 0x0132_7148;
/// The empty string the header getters return for none (`0x00b8fbeb`).
const EMPTY: u32 = 0x00b8_fbeb;

/// `1 << (n & 31)` (`shl eax, cl`).
fn bit(n: u32) -> u32 {
    1_u32.wrapping_shl(n)
}

fn and_word(m: &mut impl Mem, at: u32, mask: u32) {
    let p = m.rebase(at);
    let v = m.read::<u32>(p);
    m.write(p, v & mask);
}

fn or_word(m: &mut impl Mem, at: u32, mask: u32) {
    let p = m.rebase(at);
    let v = m.read::<u32>(p);
    m.write(p, v | mask);
}

/// `AtelClearEffectFlag_00863860(n)`: bit n of `0x01326b70` cleared;
/// `EAX` the mask.
pub fn atel_clear_effect_flag(m: &mut impl Mem, n: u32) -> u32 {
    and_word(m, EFFECT_FLAGS, !bit(n));
    !bit(n)
}

/// `0x008702f0(n)`: bit n of `0x01326b74` set; `EAX` the bit.
pub fn atel_set_effect_flag2(m: &mut impl Mem, n: u32) -> u32 {
    or_word(m, EFFECT_FLAGS2, bit(n));
    bit(n)
}

/// `0x00863af0(n)`: bit n of `0x01326b74` cleared; `EAX` the mask.
pub fn atel_clear_effect_flag2(m: &mut impl Mem, n: u32) -> u32 {
    and_word(m, EFFECT_FLAGS2, !bit(n));
    !bit(n)
}

/// `AtelSetLoveParamDisable(i, on)` (`0x00870250`): bit i (< 8) of
/// `0x01326b90` set or cleared; `EAX` the bit or its mask (i itself for
/// i >= 8).
pub fn atel_set_love_param_disable(m: &mut impl Mem, i: u32, on: u32) -> u32 {
    if i.cast_signed() >= 8 {
        return i;
    }
    if on != 0 {
        or_word(m, LOVE_DISABLE, bit(i));
        bit(i)
    } else {
        and_word(m, LOVE_DISABLE, !bit(i));
        !bit(i)
    }
}

/// Bit `k` of the guide-map flag byte = on != 0 (`AtelSetGuideMapDraw*`);
/// `EAX` the old bit.
fn guide_map_flag(m: &mut impl Mem, k: u32, on: u32) -> u32 {
    let p = m.rebase(GUIDE_MAP_FLAGS);
    let v = m.read::<u8>(p);
    let old = u32::from(v >> k & 1);
    let b = u8::from(on != 0) << k;
    m.write(p, v & !(1 << k) | b);
    old
}

/// `AtelSetGuideMapDrawBlizFlag(on)` (`0x00870160`): bit 3.
pub fn atel_set_guide_map_draw_bliz_flag(m: &mut impl Mem, on: u32) -> u32 {
    guide_map_flag(m, 3, on)
}

/// `AtelSetGuideMapDrawFlag(on)` (`0x00870190`): bit 5.
pub fn atel_set_guide_map_draw_flag(m: &mut impl Mem, on: u32) -> u32 {
    guide_map_flag(m, 5, on)
}

/// `AtelSetGuideMapForceDrawFlag(on)` (`0x008701c0`): bit 6.
pub fn atel_set_guide_map_force_draw_flag(m: &mut impl Mem, on: u32) -> u32 {
    guide_map_flag(m, 6, on)
}

/// `0x0086f3d0(on)`: bit 0 of `0x01325db0` = on != 0; `EAX` the old bit
/// xor the new one.
pub fn atel_set_ctrl_flag_1325db0(m: &mut impl Mem, on: u32) -> u32 {
    let p = m.rebase(0x0132_5db0);
    let v = m.read::<u32>(p);
    let x = (v ^ u32::from(on != 0)) & 1;
    m.write(p, v ^ x);
    x
}

/// `0x00862d50(map)`: `g_AtelForceBattleMapId` when it is positive, else
/// map.
pub fn atel_get_battle_map_id(m: &impl Mem, map: u32) -> u32 {
    let f = m.read::<u32>(m.rebase(FORCE_BATTLE_MAP));
    if f.cast_signed() < 1 { map } else { f }
}

/// `0x00870fe0(worker, on)`: the worker's `+0x36` bit 3 set or cleared;
/// returns the bit.
pub fn atel_set_worker_flag36_8(m: &mut impl Mem, w: u32, on: u32) -> u32 {
    let v = m.read::<u16>(w + 0x36);
    let v = if on != 0 { v | 8 } else { v & 0xfff7 };
    m.write(w + 0x36, v);
    u32::from(v >> 3 & 1)
}

/// The script block's expansion header (`+0x2c`, relative), if any.
fn expansion(m: &impl Mem, b: u32) -> Option<u32> {
    let o = m.read::<u32>(b + 0x2c);
    if o == 0 {
        return None;
    }
    let e = o.wrapping_add(b);
    (e != 0).then_some(e)
}

/// `AtelGetBinHdrExpandSpFlags(block)` (`0x0086adc0`): the expansion
/// header's `+8`, 0 without one.
pub fn atel_get_bin_hdr_expand_sp_flags(m: &impl Mem, b: u32) -> u32 {
    expansion(m, b).map_or(0, |e| m.read(e + 8))
}

/// `0x0086acc0(block)`: the expansion header's `+0x24` table address
/// (relative to the block), 0 without one.
pub fn atel_get_bin_hdr_expand_24(m: &impl Mem, b: u32) -> u32 {
    let Some(e) = expansion(m, b) else { return 0 };
    let t = m.read::<u32>(e + 0x24);
    if t == 0 { 0 } else { t.wrapping_add(b) }
}

/// `0x0086aa70(block)`: the expansion header's `+0x10` string (relative
/// to the block), the empty string `0x00b8fbeb` without one.
pub fn atel_get_bin_hdr_expand_name(m: &impl Mem, b: u32) -> u32 {
    let Some(e) = expansion(m, b) else {
        return m.rebase(EMPTY);
    };
    let t = m.read::<u32>(e + 0x10);
    if t == 0 {
        m.rebase(EMPTY)
    } else {
        b.wrapping_add(t)
    }
}

/// `AtelGetBindpList()` (`0x0086ae20`): the first ctrl work's script
/// block's expansion `+0x28` list, `0x01325b80` without one.
pub fn atel_get_bindp_list(m: &impl Mem) -> u32 {
    let none = m.rebase(0x0132_5b80);
    let b = m.read::<u32>(m.rebase(0x0132_5be8));
    if b == 0 {
        return none;
    }
    let Some(e) = expansion(m, b) else {
        return none;
    };
    let t = m.read::<u32>(e + 0x28);
    if t == 0 { none } else { b.wrapping_add(t) }
}

/// The expansion header's `+0x18` table entry i (8 bytes, relative to
/// the block).
fn table18(m: &impl Mem, b: u32, i: u32) -> Option<u32> {
    let e = expansion(m, b)?;
    let t = m.read::<u32>(e + 0x18);
    (t != 0).then(|| t.wrapping_add(i.wrapping_mul(8)).wrapping_add(b))
}

/// `0x0086aaf0(block, i)`: entry i's word at `+4` of the `+0x18` table, 0
/// without one.
pub fn atel_get_bin_hdr_table18_a(m: &impl Mem, b: u32, i: u32) -> u32 {
    table18(m, b, i).map_or(0, |p| m.read(p + 4))
}

/// `0x0086aac0(block, i)`: entry i's `u16` at `+8` of the `+0x18` table,
/// 0 without one.
pub fn atel_get_bin_hdr_table18_b(m: &impl Mem, b: u32, i: u32) -> u32 {
    table18(m, b, i).map_or(0, |p| u32::from(m.read::<u16>(p + 8)))
}

/// `AtelGetBinHdrExpandSoundEvent(block)` (`0x0086ad20`): the expansion
/// header's `+0x3c` record's `+8` (relative to the block), 0 without one.
pub fn atel_get_bin_hdr_expand_sound_event(m: &impl Mem, b: u32) -> u32 {
    let Some(e) = expansion(m, b) else { return 0 };
    let t = m.read::<u32>(e + 0x3c);
    if t == 0 {
        return 0;
    }
    let r = t.wrapping_add(b);
    if r == 0 {
        return 0;
    }
    m.read::<u32>(r + 8).wrapping_add(b)
}

/// `0x008858c0(ctrl, i)`: string i of the ctrl work's `+8` block's `+0x44`
/// table (offsets relative to it), the empty string without one.
pub fn atel_get_block_string(m: &impl Mem, c: u32, i: u32) -> u32 {
    let e = m.read::<u32>(c + 8);
    if e == 0 {
        return m.rebase(EMPTY);
    }
    let t = m.read::<u32>(e + 0x44).wrapping_add(e);
    if t == 0 {
        return m.rebase(EMPTY);
    }
    m.read::<u32>(t.wrapping_add(i.wrapping_mul(4)))
        .wrapping_add(t)
}

/// Sprite record i clamped to 0..127 (`0x1c` bytes from `0x01327148`).
fn sprite(m: &impl Mem, i: u32) -> u32 {
    let i = i.cast_signed().clamp(0, 0x7f).cast_unsigned();
    m.rebase(SPRITES) + i * 0x1c
}

/// `0x008747b0(i)`: sprite record i's address.
pub fn atel_get_sprite(m: &impl Mem, i: u32) -> u32 {
    sprite(m, i)
}

/// `0x008747f0(i)`: sprite record i's first byte cleared (for a negative
/// i, record 0 gets 0 too); `EAX` 7i or 0x37d (`lea`/`sub`).
pub fn atel_clear_sprite(m: &mut impl Mem, i: u32) -> u32 {
    let p = sprite(m, i);
    m.write(p, 0_u8);
    (p - m.rebase(SPRITES)) / 4
}

/// `0x00874970(i, on)`: sprite i's `+3` byte = on != 0 (1 or 0); `EAX` on.
pub fn atel_set_sprite_flag(m: &mut impl Mem, i: u32, on: u32) -> u32 {
    let p = sprite(m, i);
    m.write(p + 3, u8::from(on != 0));
    on
}

/// `0x008749b0(i, v)`: sprite i's `+0x13` byte = v; `EAX` the clamped
/// index with v in its low byte.
pub fn atel_set_sprite_13(m: &mut impl Mem, i: u32, v: u32) -> u32 {
    let k = i.cast_signed().clamp(0, 0x7f).cast_unsigned();
    let p = sprite(m, i);
    let b = v.to_le_bytes()[0];
    m.write(p + 0x13, b);
    k & !0xff | u32::from(b)
}

/// `0x00874b30(i, x, y)`: sprite i's `+4` and `+6` (`u16`) = x, y; `EAX`
/// the clamped index with y in its low half.
pub fn atel_set_sprite_pos(m: &mut impl Mem, i: u32, x: u32, y: u32) -> u32 {
    let k = i.cast_signed().clamp(0, 0x7f).cast_unsigned();
    let p = sprite(m, i);
    let [x0, x1, ..] = x.to_le_bytes();
    let [y0, y1, ..] = y.to_le_bytes();
    m.write(p + 4, u16::from_le_bytes([x0, x1]));
    m.write(p + 6, u16::from_le_bytes([y0, y1]));
    k & !0xffff | u32::from(u16::from_le_bytes([y0, y1]))
}

/// `AtelClearSnd3dWork(i)` (`0x008772f0`): record i's `+0`, `+4` and
/// `+0x20` (`u16`) cleared; `EAX` 9i (clamped to 0..15).
pub fn atel_clear_snd3d_work(m: &mut impl Mem, i: u32) -> u32 {
    let k = i.cast_signed().clamp(0, 0xf).cast_unsigned();
    let p = m.rebase(SND3D_WORK) + k * 0x24;
    m.write(p, 0_u32);
    m.write(p + 4, 0_u32);
    m.write(p + 0x20, 0_u16);
    k * 9
}

/// `0x00874fb0()`: the 32 records at `0x01327f50` (16 bytes) get `+0`,
/// `+4` and the byte `+8` cleared; `EAX` the end `0x01328154`.
pub fn atel_clear_records_1327f50(m: &mut impl Mem) -> u32 {
    let base = m.rebase(0x0132_7f50);
    for i in 0..32 {
        let p = base + i * 0x10;
        m.write(p, 0_u32);
        m.write(p + 4, 0_u32);
        m.write(p + 8, 0_u8);
    }
    base + 0x204
}

/// `0x008719c0(s)`: the length of the string at s.
pub fn atel_strlen(m: &impl Mem, s: u32) -> u32 {
    let mut n = 0_u32;
    while m.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `0x00877610(s, n, ch)`: drops up to n leading `ch` from the string in
/// place, as the original does (when all n match, the copy starts at the
/// last one); `EAX` the new terminator's address.
pub fn atel_str_trim_leading(m: &mut impl Mem, s: u32, n: u32, ch: u32) -> u32 {
    let mut src = s;
    let mut p = s;
    let mut k = 0_i32;
    if n.cast_signed() > 0 {
        loop {
            let c = i32::from(m.read::<i8>(p)).cast_unsigned();
            src = p;
            p = p.wrapping_add(1);
            if c != ch {
                break;
            }
            k += 1;
            if k >= n.cast_signed() {
                break;
            }
        }
    }
    let mut d = s;
    let mut c = m.read::<u8>(src);
    while c != 0 {
        m.write(d, c);
        c = m.read::<u8>(src.wrapping_add(d.wrapping_sub(s)).wrapping_add(1));
        d = d.wrapping_add(1);
    }
    m.write(d, 0_u8);
    d
}

/// One `f32` argument stored `fld m32`/`fstp m32`.
fn put_f32(m: &mut impl Mem, at: u32, v: u32) {
    m.write(at, x87::store(x87::load(v)));
}

/// `0x00870d10(worker, i, v f)`: the worker's `+(i + 0x14) * 0x4c` = v;
/// `EAX` the worker.
pub fn atel_set_level_f74(m: &mut impl Mem, w: u32, i: u32, v: u32) -> u32 {
    put_f32(
        m,
        w.wrapping_add(i.wrapping_add(0x14).wrapping_mul(0x4c)),
        v,
    );
    w
}

/// The current level's field `off` (`+level*0x4c`) = v (`0x0086f040`
/// `+0x5ec`, `0x008715c0` `+0x5c8`, `0x00871610` `+0x5c4`, `0x00871630`
/// `+0x5c0`); `EAX` level*0x4c.
pub fn atel_set_cur_level_f32(m: &mut impl Mem, w: u32, off: u32, v: u32) -> u32 {
    let o = u32::from(m.read::<u8>(w + 0x32)) * 0x4c;
    put_f32(m, w + o + off, v);
    o
}

/// Level i's field `off` (`+i*0x4c`) = v, left in `ST0` (`fst`):
/// `0x00871500` `+0x5f8`, `0x00871520` `+0x5f4`. Returns v.
pub fn atel_set_level_f32(m: &mut impl Mem, w: u32, i: u32, off: u32, v: u32) -> f64 {
    put_f32(m, w.wrapping_add(i.wrapping_mul(0x4c)).wrapping_add(off), v);
    x87::load(v)
}

/// Turn block i's field `off` (`+i*0x44`) = v, left in `ST0`:
/// `0x00871540` `+0x880`, `0x00871560` `+0x87c`, `0x00871580` `+0x874`,
/// `0x008715a0` `+0x878`. Returns v.
pub fn atel_set_turn_f32(m: &mut impl Mem, w: u32, i: u32, off: u32, v: u32) -> f64 {
    put_f32(m, w.wrapping_add(i.wrapping_mul(0x44)).wrapping_add(off), v);
    x87::load(v)
}

/// The same field of all nine levels (`+off + l*0x4c`) or turn blocks
/// (`+off + l*0x44`) = v (`0x0086f0d0` and the like; `0x0086f300`'s last
/// store is an `fst`, so its shim returns v in `ST0`); `EAX` the worker.
pub fn atel_set_all_f32(m: &mut impl Mem, w: u32, off: u32, step: u32, v: u32) -> u32 {
    for l in 0..9 {
        put_f32(m, w + off + l * step, v);
    }
    w
}

/// `0x007a3ac0()`: the battle word `0x0112c900` as `f32` (`fild`).
pub fn atel_get_battle_count_f(m: &impl Mem) -> f64 {
    let v = m.read::<u32>(m.rebase(0x0112_c900)).cast_signed();
    x87::load(x87::store(f64::from(v)))
}

// ---- round 15 ----

fn host_call(h: &mut impl ffx_bridge::Host, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call(f, args)
}

/// `AtelSetUpCallFunc(group, table)` (`0x00877880`): `g_NativeTables[group]`
/// = table, or the empty table `0x00c52b60` for 0; `EAX` the group.
pub fn atel_set_up_call_func(m: &mut impl Mem, group: u32, table: u32) -> u32 {
    let t = if table == 0 {
        m.rebase(0x00c5_2b60)
    } else {
        table
    };
    let at = m.rebase(0x0132_8558).wrapping_add(group.wrapping_mul(4));
    m.write(at, t);
    group
}

/// `AtelGetSaveDic()` (`0x0086c410`): the save event record's
/// (`MsGetSaveEventAddress`, `0x0088c830`) `+0xd4` when `+0xd6` is set,
/// else `+0xe` (`u16`).
pub fn atel_get_save_dic(h: &mut impl ffx_bridge::Host) -> u32 {
    let e = host_call(h, 0x0088_c830, &[]);
    let off = if h.read::<u8>(e + 0xd6) != 0 {
        0xd4
    } else {
        0xe
    };
    u32::from(h.read::<u16>(e + off))
}

/// `AtelGetSaveDicName(i, dic)` (`0x0086c430`): entry i (0 past the
/// count, the first `u16` / 4) of the dictionary `TOGetSaveDicAdrs(dic)`
/// (`0x008b7f60`), its second offset with hiragana on
/// (`MsGetSaveConfigHiragana`); the text's address.
pub fn atel_get_save_dic_name(h: &mut impl ffx_bridge::Host, i: u32, dic: u32) -> u32 {
    let hira = u32::from(host_call(h, 0x0078_51f0, &[]) != 0);
    let p = host_call(h, 0x008b_7f60, &[dic]);
    let n = u32::from(h.read::<u16>(p) >> 2);
    let k = if i.cast_signed() >= n.cast_signed() {
        0
    } else {
        i
    };
    let e = hira.wrapping_add(k.wrapping_mul(2));
    u32::from(h.read::<u16>(p.wrapping_add(e.wrapping_mul(2)))).wrapping_add(p)
}

/// `0x00877520(v, dst, base)`: v written as digits over the divisor table
/// at `0x00c52b18` (0-ended), each digit plus `base` (`'0'`); `EAX` what
/// is left of v (dst for an empty table).
pub fn atel_put_digits(m: &mut impl Mem, v: u32, dst: u32, base: u32) -> u32 {
    let mut t = m.rebase(0x00c5_2b18);
    let mut d = m.read::<u32>(t);
    if d == 0 {
        m.write(dst, 0_u8);
        return dst;
    }
    let (mut v, mut p) = (v, dst);
    let [b, ..] = base.to_le_bytes();
    while d != 0 {
        let mut n = 0_u8;
        t += 4;
        while v >= d {
            v -= d;
            n = n.wrapping_add(1);
        }
        m.write(p, n.wrapping_add(b));
        p = p.wrapping_add(1);
        d = m.read::<u32>(t);
    }
    m.write(p, 0_u8);
    v
}

/// `0x0086db80(worker, mask)`: the current level's turn block gets the
/// worker's position-block angles: bit 0 `+0x38` = `+0x34`, bit 1 `+0x3c`
/// = `+0x30`, bit 2 `+0x40` = `+0x38` (`fld`/`fstp`). `EAX` the mask.
pub fn atel_copy_turn_angles(m: &mut impl Mem, w: u32, mask: u32) -> u32 {
    let t = kind(m, w);
    let src = if t == 4 {
        0
    } else if t.wrapping_sub(5) < 2 {
        w + 0x284
    } else {
        w + 0x558
    };
    let l = clamp_level(m, w, u32::from(m.read::<u8>(w + 0x32)));
    let blk = m.read::<u32>(w + 0x174 + l * 0x4c);
    for (bit, from, to) in [(1, 0x34, 0x38), (2, 0x30, 0x3c), (4, 0x38, 0x40)] {
        if mask & bit != 0 {
            copy_f32(m, src + from, blk + to);
        }
    }
    mask
}

/// `AtelUcOff()` (`0x008719e0`): for each type-1 worker of the current
/// ctrl work that has the player's control (`+0xaa` 1, `+0x34` bit 5):
/// the bit cleared, its position block `+0xc` = 0.0 and
/// `Ch_SetSp(model, 0.0)` (`0x0082b840`) for a model; then
/// `Sg_SetPlayer(0)` (`0x0082dad0`) and the ctrl work's bit 1 cleared.
/// `EAX` the ctrl work.
pub fn atel_uc_off(h: &mut impl ffx_bridge::Host) -> u32 {
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let mut c = h.read::<u32>(ctx);
    let n = u32::from(h.read::<u16>(c + 0xc));
    for i in 0..n {
        if i >= u32::from(h.read::<u16>(c + 0xc)) {
            continue;
        }
        let w = super::atel_get_basic_work(h, i.cast_signed());
        if kind(h, w) != 1 {
            continue;
        }
        let w = super::atel_get_basic_work(h, i.cast_signed());
        let pos = w + 0x558;
        let flags = h.read::<u16>(w + 0x34);
        if h.read::<u8>(w + 0xaa) != 1 || flags & 0x20 == 0 {
            continue;
        }
        h.write(w + 0x34, flags & 0xffdf);
        h.write(pos + 0xc, 0_u32);
        let model = h.read::<u32>(w + 0x9c);
        if model != 0 {
            host_call(h, 0x0082_b840, &[model, 0]);
            c = h.read::<u32>(ctx);
        }
    }
    host_call(h, 0x0082_dad0, &[0]);
    let c = h.read::<u32>(ctx);
    let b = h.read::<u8>(c);
    h.write(c, b & 0xfd);
    c
}

/// `g_AtelBasicWorkCache` (`0x013270e4`).
const BASIC_WORK_CACHE: u32 = 0x0132_70e4;

/// `AtelRequireCtrlWork(i, block, a, b)` (`0x0086ee10`): ctrl work i
/// (`g_AtelCtrlWorks + i*0x238`) set up for a script block: its id, the
/// block (`+0x48`) and its counts (`+0x36`, `+0x34`, `+0x14`, `+0x1c`,
/// `+0x16` of the block, or zeros without one), `a` and `b` (`+0x1c`,
/// `+0x24`), the callbacks (`+0x50` = `0x008683e0`), the flag words
/// (`+0` 0x9880, `+2` 0xc8 keeping bit 4, `+3` 0x30, `+4` 0x20004,
/// `+0x210` `& !0x36 | 0x49`), two scales of 1.0 and cleared slots; also
/// `g_AtelBasicWorkCache` = 0. Returns the ctrl work.
#[allow(clippy::too_many_lines)] // one store per field
pub fn atel_require_ctrl_work(m: &mut impl Mem, i: u32, block: u32, a: u32, b: u32) -> u32 {
    let c = m.rebase(0x0132_5ba0).wrapping_add(i.wrapping_mul(0x238));
    let one = 1f32.to_bits();
    for (off, v) in [
        (0x228, 0),
        (0x22c, 0),
        (0x230, 0),
        (0x234, one),
        (0x224, one),
    ] {
        m.write(c + off, v);
    }
    m.write(c + 0x1a, u16::try_from(i & 0xffff).unwrap_or(0));
    m.write(c + 0x1c, a);
    m.write(c + 0x218, 0_u32);
    m.write(c + 0x24, b);
    m.write(c + 0x21c, 0_u32);
    m.write(c + 0x220, 0_u32);
    m.write(c + 0xa, 0xffff_u16);
    m.write(c + 0x1ec, 0_u32);
    m.write(c + 0x1e8, 0xffff_u16);
    m.write(c + 0x1f0, 0_u32);
    let f2 = m.read::<u8>(c + 2);
    m.write(c + 0x208, 0_u32);
    m.write(c + 2, f2 & 0x10 | 0xc8);
    let fl = m.read::<u32>(c + 0x210);
    for off in [0x20c, 0x20, 0x28, 0x2c, 0x30, 0x4c] {
        m.write(c + off, 0_u32);
    }
    m.write(c + 0x50, m.rebase(0x0086_83e0));
    for off in [0x58, 0x5c, 0x60, 0x34, 0x44] {
        m.write(c + off, 0_u32);
    }
    m.write(c + 0x1ea, 0_u8);
    for off in [0x200, 0x204, 0x1f8, 0x1fc, 0x1f4] {
        m.write(c + off, 0_u32);
    }
    m.write(c + 4, 0x0002_0004_u32);
    m.write(c + 0x1e7, 0xff_u8);
    m.write(c + 0x54, 0_u32);
    m.write(c + 8, 0_u8);
    m.write(c + 0x48, block);
    m.write(c + 0x1e4, 0_u8);
    m.write(c, 0x9880_u16);
    m.write(c + 3, 0x30_u8);
    m.write(c + 0x210, fl & 0xffff_ffc9 | 0x49);
    m.write(c + 0x214, u32::MAX);
    if block == 0 {
        m.write(c + 0xc, 0_u16);
        m.write(c + 0x10, 0_u32);
        m.write(c + 0x14, 0_u32);
        m.write(c + 0x18, 0_u16);
    } else {
        let n36 = m.read::<u16>(block + 0x36);
        m.write(c + 0xc, n36);
        let n34 = m.read::<u16>(block + 0x34);
        m.write(c + 0x10, n34);
        m.write(c + 0x12, n36);
        let n14 = m.read::<u16>(block + 0x14);
        m.write(c + 0x14, n14);
        let n1c = m.read::<u16>(block + 0x1c);
        m.write(c + 0x18, n1c);
        let n16 = m.read::<u16>(block + 0x16);
        m.write(c + 0x16, n16);
    }
    for off in [0x64, 0x74, 0x84, 0x94] {
        m.write(c + off, u32::MAX);
    }
    for k in 0..0x20 {
        m.write(c + 0xa4 + k * 8, 0_u16);
    }
    m.write(m.rebase(BASIC_WORK_CACHE), 0_u32);
    for k in 0..8 {
        m.write(c + 0x1a4 + k * 8, 0_u32);
    }
    c
}
