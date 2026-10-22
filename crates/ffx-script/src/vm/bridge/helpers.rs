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

// ---- round 16 ----

/// `g_AtelCallbackQue` (`0x01327f50`): queues of 0x100 bytes, fifteen
/// 16-byte entries (`+0` the callback, `+4` its word, `+8` a 7-character
/// name) and a sixteenth callback slot at `+0xf0`.
const CALLBACK_QUE: u32 = 0x0132_7f50;

/// `Script_LineActorSide(worker, key)` (`0x008632b0`): for worker types 5
/// and 6, the status byte (`+4`) of the entry of the worker's line table
/// (`+0x2e4`, eight 8-byte entries) whose `+2` is the bound actor's id
/// (`*(+0x90) + 8`) and, for a non-negative key, whose `+0` is the key;
/// -1 for none, a status of 3 or other types.
pub fn script_line_actor_side(m: &impl Mem, w: u32, key: u32) -> u32 {
    if kind(m, w).wrapping_sub(5) > 1 {
        return u32::MAX;
    }
    let t = m.read::<u32>(w + 0x2e4);
    let a = m.read::<u32>(w + 0x90);
    if t == 0 || a == 0 {
        return u32::MAX;
    }
    let id = m.read::<u16>(a + 8);
    let hit = (0..8).find(|&i| {
        let e = t + i * 8;
        m.read::<u16>(e + 2) == id && (key.cast_signed() < 0 || u32::from(m.read::<u16>(e)) == key)
    });
    match hit.map(|i| u32::from(m.read::<u8>(t + i * 8 + 4))) {
        Some(v) if v != 3 => v,
        _ => u32::MAX,
    }
}

/// `0x00877660(s)`: the hexadecimal number at s (after an optional `0x`
/// or `$`), up to the first character that is no hex digit.
pub fn atel_parse_hex(m: &impl Mem, s: u32) -> u32 {
    let mut p = s;
    if m.read::<u8>(p) == b'0' && m.read::<u8>(p + 1) == b'x' {
        p += 2;
    } else if m.read::<u8>(p) == b'$' {
        p += 1;
    }
    let mut v = 0_u32;
    loop {
        let c = m.read::<u8>(p);
        match c {
            b'0'..=b'9' => v = (v << 4).wrapping_add(u32::from(c - b'0')),
            b'A'..=b'F' | b'a'..=b'f' => {
                v = (v << 4)
                    .wrapping_sub(0x37)
                    .wrapping_add(u32::from(c & 0xdf));
            }
            _ => return v,
        }
        p = p.wrapping_add(1);
    }
}

/// `0x0086f460(n)`: bind-mask bit n (below 0x200; signed division, as the
/// original) set and the union `0x01326b94` recomputed. Returns the `EAX`
/// it leaves (the loop's end address; n itself for n >= 0x200, where the
/// original leaves the caller's).
pub fn atel_set_bind_mask(m: &mut impl Mem, n: u32) -> u32 {
    let k = n.cast_signed();
    if k >= 0x200 {
        return n;
    }
    let idx = k / 32;
    let bit = 1_u32.wrapping_shl((k % 32).cast_unsigned());
    let at = m
        .rebase(BIND_MASKS)
        .wrapping_add(idx.cast_unsigned().wrapping_mul(4));
    let v = m.read::<u32>(at);
    m.write(at, v | bit);
    let base = m.rebase(BIND_MASKS);
    let mut all = 0;
    for i in 0..16 {
        all |= m.read::<u32>(base + 4 * i);
    }
    m.write(m.rebase(BIND_MASK_ALL), all);
    base + 0x48
}

/// `0x0086ab80(block, i)` (float): the expansion header's `+0x3c` record's
/// `+0x44 + 4i`, else the header's `+0x14`, else 0.0 (`f32`).
pub fn atel_get_bin_hdr_float(m: &impl Mem, b: u32, i: u32) -> f64 {
    let Some(e) = expansion(m, b) else { return 0.0 };
    let r = m.read::<u32>(e + 0x3c);
    let at = if r != 0 && r.wrapping_add(b) != 0 {
        r.wrapping_add(b)
            .wrapping_add(0x44)
            .wrapping_add(i.wrapping_mul(4))
    } else {
        e + 0x14
    };
    x87::load(x87::store(x87::load(m.read(at))))
}

/// A size rounded up to 16 as the original does (`(x + 15)`, then toward
/// zero to a multiple of 16).
fn round16(x: u32) -> u32 {
    let v = x.wrapping_add(15).cast_signed();
    (v.wrapping_add((v >> 31) & 15) & !15).cast_unsigned()
}

/// `AtelGetMyBufferSize(block)` (`0x0086c0c0`): the sum of the block's
/// workers' sizes (each worker header's `+0x10`, from the offsets at
/// `+0x38`, `+0x36` of them), each rounded up to 16.
pub fn atel_get_my_buffer_size(m: &impl Mem, b: u32) -> u32 {
    let n = u32::from(m.read::<u16>(b + 0x36));
    (0..n).fold(0_u32, |s, j| {
        let h = m.read::<u32>(b + 0x38 + j * 4);
        s.wrapping_add(round16(m.read::<u32>(h.wrapping_add(0x10).wrapping_add(b))))
    })
}

/// `AtelRegistCallbackQue2(q, f, name)` (`0x008751b0`): the first free of
/// queue q's fifteen entries gets callback f, word 0 and the name
/// (`strncpy` of 7, 0-ended); the next entry's callback is cleared.
/// `EAX` the name's address, or the last entry's when the queue is full.
pub fn atel_regist_callback_que2(m: &mut impl Mem, q: u32, f: u32, name: u32) -> u32 {
    let base = m.rebase(CALLBACK_QUE).wrapping_add(q.wrapping_mul(0x100));
    let Some(k) = (0..15).find(|&k| m.read::<u32>(base + k * 0x10) == 0) else {
        return base + 15 * 0x10;
    };
    let e = base + k * 0x10;
    m.write(e, f);
    m.write(e + 4, 0_u32);
    let mut end = false;
    for i in 0..7 {
        let c = if end {
            0
        } else {
            m.read::<u8>(name.wrapping_add(i))
        };
        end |= c == 0;
        m.write(e + 8 + i, c);
    }
    m.write(e + 0xf, 0_u8);
    m.write(e + 0x10, 0_u32);
    e + 8
}

/// `AtelExecCallbackQue(q)` (`0x008750b0`): calls queue q's first
/// callback with its word's address; when it returns non-zero the
/// entries move up one (callback, word and name) and the last slot is
/// cleared. 1 when the queue is (then) empty.
pub fn atel_exec_callback_que(h: &mut impl ffx_bridge::Host, q: u32) -> u32 {
    let base = h.rebase(CALLBACK_QUE).wrapping_add(q.wrapping_mul(0x100));
    let f = h.read::<u32>(base);
    if f == 0 {
        return 1;
    }
    if h.call(f, &[base + 4]) != 0 {
        for k in 0..15 {
            let e = base + k * 0x10;
            let nf = h.read::<u32>(e + 0x10);
            h.write(e, nf);
            let nw = h.read::<u32>(e + 0x14);
            h.write(e + 4, nw);
            let mut i = 0;
            loop {
                let c = h.read::<u8>(e + 0x18 + i);
                h.write(e + 8 + i, c);
                i += 1;
                if c == 0 {
                    break;
                }
            }
        }
        h.write(base + 0xf0, 0_u32);
    }
    u32::from(h.read::<u32>(base) == 0)
}

/// `AtelClearMesWinWork()` (`0x00863b10`): the 16 message windows (two
/// banks of eight, 0x2c bytes from `g_AtelMsgWindows` `0x01326d70`)
/// reset: `+0`, `+0x14`, `+0x1c`, `+0x28` = 0, `+4` = `0x01a00200`, `+8`
/// = `g_MsgNone` (`0x01325b74`), `+0x10` = `0x50000080`. Returns 0.
pub fn atel_clear_mes_win_work(m: &mut impl Mem) -> u32 {
    let base = m.rebase(0x0132_6d70);
    // 0x01a00200 is a constant the image has no relocation for
    let (buf, none) = (0x01a0_0200, m.rebase(0x0132_5b74));
    for k in 0..16 {
        let w = base + k * 0x2c;
        for (off, v) in [
            (0, 0),
            (4, buf),
            (0x10, 0x5000_0080),
            (0x28, 0),
            (8, none),
            (0x14, 0),
            (0x1c, 0),
        ] {
            m.write(w + off, v);
        }
    }
    0
}

// ---- round 17 ----

/// `0x008683e0(ctrl)`: the ctrl work's default start callback (`+0x50`,
/// set by `AtelRequireCtrlWork`): runs each worker (`Script_Run(worker,
/// ctrl +8, 1)`) until its `+0x34` bit 4 says its init is done, then
/// decodes every worker's signals (`AtelDecodeSignal(i, -1)`) and sets the
/// ctrl work's bit 3. `EAX` the ctrl work (0 without workers).
pub fn atel_ctrl_start<H: super::run::NativeHost>(h: &mut H, c: u32) -> u32 {
    let n = u32::from(h.read::<u16>(c + 0xc));
    let mode = u32::from(h.read::<u8>(c + 8));
    for i in 0..n {
        let w = super::atel_get_basic_work(h, i.cast_signed());
        loop {
            super::run::script_run(h, &mut super::run::OriginalNatives, w, mode, 1);
            if h.read::<u8>(w + 0x34) & 0x10 != 0 {
                break;
            }
        }
    }
    for i in 0..n {
        super::signal::atel_decode_signal(h, i, -1);
    }
    let b = h.read::<u8>(c);
    h.write(c, b | 8);
    if n == 0 { 0 } else { c }
}

// ---- round 19 ----

/// `g_AtelCtrlWorks` (`0x01325ba0`): ctrl work 0, the event's.
const CTRL_WORKS: u32 = 0x0132_5ba0;
/// `MsGetSaveEventAddress()` (`0x0088c830`): the save data's event record.
const MS_GET_SAVE_EVENT_ADDRESS: u32 = 0x0088_c830;
/// `TkSetFadeOut(frames)` (`0x0088eb50`).
const TK_SET_FADE_OUT: u32 = 0x0088_eb50;
/// `TkGetNowEventJumpIndex()` (`0x0088d670`): the entrance index.
const TK_GET_NOW_EVENT_JUMP_INDEX: u32 = 0x0088_d670;

/// `AtelDecodeCommand(i, mode)` (`0x008641c0`): `Script_Run(worker i,
/// mode, 1)`; its result.
pub fn atel_decode_command<H: super::run::NativeHost>(h: &mut H, i: u32, mode: u32) -> u32 {
    let w = super::atel_get_basic_work(h, i.cast_signed());
    super::run::script_run(h, &mut super::run::OriginalNatives, w, mode, 1)
}

/// `AtelSetSaveDicIndex(i)` (`0x00870ef0`): the save event record's `+0xe`
/// = the event block's (ctrl work 0's `+0x48`) dictionary word `+0x1e`,
/// or, when its bit 15 is set, entry i (0 from the count in the low 15
/// bits on, signed) of the table at the block's `+0x28`; 0 without a
/// block. `g_ScriptContext` is ctrl work 0 meanwhile. `EAX` the record.
pub fn atel_set_save_dic_index(h: &mut impl ffx_bridge::Host, i: u32) -> u32 {
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let blk = h.read::<u32>(h.rebase(CTRL_WORKS + 0x48));
    let old = h.read::<u32>(ctx);
    h.write(ctx, h.rebase(CTRL_WORKS));
    let v = if blk == 0 {
        0
    } else {
        let v = h.read::<u16>(blk + 0x1e);
        if v & 0x8000 == 0 {
            v
        } else {
            let n = u32::from(v & 0x7fff).cast_signed();
            let k = if i.cast_signed() >= n { 0 } else { i };
            let t = h.read::<u32>(blk + 0x28).wrapping_add(blk);
            h.read::<u16>(t.wrapping_add(k.wrapping_mul(2)))
        }
    };
    let e = host_call(h, MS_GET_SAVE_EVENT_ADDRESS, &[]);
    h.write(e + 0xe, v);
    h.write(ctx, old);
    e
}

/// The jump shared by `AtelSetEventJump` and `AtelSetEventJump2`: with the
/// ctrl work's bit 7 (or `FFXIsDebug`, `0x00887c50`): `AtelUcOff`,
/// `TkSetEventJump(map, entrance)` (`0x0088ea60`), the hook at
/// `0x01327070` when set, `MsBattleExeClear` (`0x00781530`), the ctrl work's
/// bit 7 cleared and `+0x210` bit 1 set, `AtelSetSnd3dVolumeDiffRatio`
/// (`0x00877450`) of the float at `0x00b5be88`. `false` when it does not
/// jump (`EAX` 0 then).
fn event_jump(h: &mut impl ffx_bridge::Host, map: u32, entrance: u32) -> bool {
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let c = h.read::<u32>(ctx);
    if h.read::<u8>(c) & 0x80 == 0 && host_call(h, 0x0088_7c50, &[]) == 0 {
        return false;
    }
    atel_uc_off(h);
    host_call(h, 0x0088_ea60, &[map, entrance]);
    let hook = h.read::<u32>(h.rebase(0x0132_7070));
    if hook != 0 {
        h.call(hook, &[]);
    }
    host_call(h, 0x0078_1530, &[]);
    let c = h.read::<u32>(ctx);
    let b = h.read::<u8>(c);
    h.write(c, b & 0x7f);
    let f = h.read::<u32>(c + 0x210);
    h.write(c + 0x210, f | 2);
    let ratio = x87::store(x87::load(h.read(h.rebase(0x00b5_be88))));
    host_call(h, 0x0087_7450, &[ratio]);
    true
}

/// `TkSetFadeOut` of 30 frames on map `0x109` (the save event record's
/// `+2`, which `TkSetEventJump` has just set to the map jumped to), else
/// 0; its `EAX`.
fn event_fade(h: &mut impl ffx_bridge::Host) -> u32 {
    let e = host_call(h, MS_GET_SAVE_EVENT_ADDRESS, &[]);
    let frames = if h.read::<u16>(e + 2) == 0x109 {
        0x1e
    } else {
        0
    };
    host_call(h, TK_SET_FADE_OUT, &[frames])
}

/// `AtelSetEventJump(map, entrance)` (`0x0086fec0`): the map jump (see
/// `event_jump`) and its fade-out. `EAX` `TkSetFadeOut`'s, 0 without a
/// jump.
pub fn atel_set_event_jump(h: &mut impl ffx_bridge::Host, map: u32, entrance: u32) -> u32 {
    if !event_jump(h, map, entrance) {
        return 0;
    }
    event_fade(h)
}

/// `AtelSetEventJump2(map, entrance, own)` (`0x0086ff40`): as
/// `AtelSetEventJump`; with `own` the fade takes the save event record's
/// frames (`+0x12`) and its kind (`+0x11`: 0 `Sg_FadeOut`, 1
/// `Sg_FadeOutW`, `0x0082cbc0`/`0x0082cbe0`). `EAX` the last call's (the
/// kind minus 1 for another kind, 0 without a jump).
pub fn atel_set_event_jump2(
    h: &mut impl ffx_bridge::Host,
    map: u32,
    entrance: u32,
    own: u32,
) -> u32 {
    if !event_jump(h, map, entrance) {
        return 0;
    }
    if own == 0 {
        return event_fade(h);
    }
    let e = host_call(h, MS_GET_SAVE_EVENT_ADDRESS, &[]);
    let frames = u32::from(h.read::<u8>(e + 0x12));
    host_call(h, TK_SET_FADE_OUT, &[frames]);
    let e = host_call(h, MS_GET_SAVE_EVENT_ADDRESS, &[]);
    match h.read::<u8>(e + 0x11) {
        0 => host_call(h, 0x0082_cbc0, &[frames]),
        1 => host_call(h, 0x0082_cbe0, &[frames]),
        k => u32::from(k) - 1,
    }
}

/// `AtelUcOn()` (`0x00871ab0`): unless the ctrl work's `+1` bit 2 or `+3`
/// bits `0x84` are set, every type-1 worker of it bound to a character
/// (`+0xaa` 1, `+0x40` not -1) whose control bit is set (`+0x36` bit 3 when
/// the ctrl work's bit 5 is set, else `+0x34` bit 9) gets the player's
/// control (`+0x34` bit 5); the one whose id is the ctrl work's `+0xa`
/// with a model (`+0x9c`) becomes the player (`Sg_SetPlayer(model)`,
/// `0x0082dad0`). Without one, `Sg_SetPlayer(0)`. Either way the ctrl
/// work's bit 1 is set. (`EAX` is the last call's leftover; no caller
/// reads it.)
pub fn atel_uc_on(h: &mut impl ffx_bridge::Host) {
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let mut c = h.read::<u32>(ctx);
    let n = u32::from(h.read::<u16>(c + 0xc));
    let mut found = false;
    if h.read::<u8>(c + 1) & 4 == 0 && h.read::<u8>(c + 3) & 0x84 == 0 {
        for i in 0..n {
            if i >= u32::from(h.read::<u16>(c + 0xc)) {
                continue;
            }
            let w = super::atel_get_basic_work(h, i.cast_signed());
            if kind(h, w) != 1 {
                continue;
            }
            let w = super::atel_get_basic_work(h, i.cast_signed());
            let on = if h.read::<u8>(c) & 0x20 == 0 {
                h.read::<u16>(w + 0x34) >> 9
            } else {
                h.read::<u16>(w + 0x36) >> 3
            } & 1;
            if h.read::<u8>(w + 0xaa) != 1 || h.read::<u32>(w + 0x40) == u32::MAX || on == 0 {
                continue;
            }
            let f = h.read::<u16>(w + 0x34);
            h.write(w + 0x34, f | 0x20);
            let model = h.read::<u32>(w + 0x9c);
            if h.read::<u16>(w + 0x2e) == h.read::<u16>(c + 0xa) && model != 0 {
                host_call(h, 0x0082_dad0, &[model]);
                c = h.read::<u32>(ctx);
                found = true;
            }
        }
        if found {
            let b = h.read::<u8>(c);
            h.write(c, b | 2);
            return;
        }
    }
    host_call(h, 0x0082_dad0, &[0]);
    let c = h.read::<u32>(ctx);
    let b = h.read::<u8>(c);
    h.write(c, b | 2);
}

/// `Script_StopMove(worker, level)` (`0x00863e10`; -1 = the current
/// level): when that level's move block (`+0x170 + level*0x4c`) is turning
/// too (`+2` bit 14), `Script_StopTurn(worker, level)` first; then every
/// level sharing the block gets its own at `+0x5b8 + i*0x4c`, with `+2` = 0
/// and `+0x28`, `+0x2c`, `+0x30`, `+0x38`, `+0x44`, `+0x48`, `+8`, `+0xc`
/// copied from the shared one. `EAX` as the loop leaves it (see
/// `script_stop_turn`).
pub fn script_stop_move(m: &mut impl Mem, w: u32, level: u32) -> u32 {
    let level = if level.cast_signed() < 0 {
        u32::from(m.read::<u8>(w + 0x32))
    } else {
        level
    };
    let slot = |l: u32| w + 0x170 + l * 0x4c;
    let l = clamp_level(m, w, level);
    let shared = m.read::<u32>(slot(l));
    if m.read::<u16>(shared + 2) & 0x4000 != 0 {
        script_stop_turn(m, w, level);
    }
    let mut eax = 0;
    for i in 0..9 {
        let blk = w + 0x5b8 + i * 0x4c;
        let h = m.read::<u32>(w);
        eax = h & !0xff | u32::from(m.read::<u8>(h).wrapping_sub(5));
        let k = clamp_level(m, w, i);
        if m.read::<u32>(slot(k)) != shared {
            continue;
        }
        m.write(slot(k), blk);
        m.write(blk + 2, 0_u16);
        for off in [0x28, 0x2c, 0x30, 0x38, 0x44, 0x48, 8, 0xc] {
            let to = m.read::<u32>(slot(k));
            copy_f32(m, shared + off, to + off);
            eax = to;
        }
    }
    eax
}

/// `AtelGetBattleIdAdrs(i)` (`0x0086a970`): worker i's `+0xc0`.
pub fn atel_get_battle_id_adrs(m: &mut impl Mem, i: u32) -> u32 {
    super::atel_get_basic_work(m, i.cast_signed()).wrapping_add(0xc0)
}

/// `AtelSetMoveHokanMode(worker, mode)` (`0x008703b0`):
/// `Ch_EnRotHokan(model, mode)` (`0x0082aae0`, a tail jump) for a worker
/// bound to a character (`+0xaa` 1) with a model (`+0x9c`). `EAX` its
/// result, else `+0xaa` minus 1 or the null model.
pub fn atel_set_move_hokan_mode(h: &mut impl ffx_bridge::Host, w: u32, mode: u32) -> u32 {
    let k = u32::from(h.read::<u8>(w + 0xaa)).wrapping_sub(1);
    if k != 0 {
        return k;
    }
    let model = h.read::<u32>(w + 0x9c);
    if model == 0 {
        return 0;
    }
    host_call(h, 0x0082_aae0, &[model, mode])
}

/// Runs `f` with ctrl work 0 (the event's) current, as the `...FromID`
/// getters do, and puts the previous one back.
fn with_event_ctrl<M: Mem, T>(m: &mut M, f: impl FnOnce(&mut M) -> T) -> T {
    let ctx = m.rebase(SCRIPT_CONTEXT);
    let old = m.read::<u32>(ctx);
    m.write(ctx, m.rebase(CTRL_WORKS));
    let r = f(m);
    m.write(ctx, old);
    r
}

/// `AtelGetEventCharHandleFromID(i)` (`0x0086b3a0`): the model (`+0x9c`)
/// of the event's worker i.
pub fn atel_get_event_char_handle_from_id(m: &mut impl Mem, i: u32) -> u32 {
    with_event_ctrl(m, |m| {
        let w = super::atel_get_basic_work(m, i.cast_signed());
        m.read::<u32>(w + 0x9c)
    })
}

/// `AtelGetEventActorPos(i, x, y, z)` (`0x0086b1e0`): the position of the
/// event's worker i (the one `0x01326b2c` names for an index outside
/// `0..WORKERS`): `+0x284` for types 5 and 6, else `+0x558` (type 4 reads
/// address 0, as the original). Each non-null out pointer gets one word
/// (`fld`/`fstp`); `EAX` the position's address.
pub fn atel_get_event_actor_pos(m: &mut impl Mem, i: u32, x: u32, y: u32, z: u32) -> u32 {
    let ctx = m.rebase(SCRIPT_CONTEXT);
    let old = m.read::<u32>(ctx);
    m.write(ctx, m.rebase(CTRL_WORKS));
    let n = i32::from(m.read::<u16>(m.rebase(CTRL_WORKS + 0xc)));
    let i = if i.cast_signed() < 0 || i.cast_signed() >= n {
        m.read::<u32>(m.rebase(0x0132_6b2c))
    } else {
        i
    };
    let w = super::atel_get_basic_work(m, i.cast_signed());
    let t = kind(m, w);
    let p = if t == 4 {
        0
    } else if t.wrapping_sub(5) < 2 {
        w + 0x284
    } else {
        w + 0x558
    };
    if x != 0 {
        copy_f32(m, p, x);
    }
    if y != 0 {
        copy_f32(m, p + 4, y);
    }
    m.write(ctx, old);
    if z != 0 {
        copy_f32(m, p + 8, z);
    }
    p
}

/// Entrance `idx` of a worker's script block (`+4`: its entrances at
/// block `+4`, their end at `+8`, block-relative, 0x20 bytes each; past
/// the end the first), `0x01325b50` without entrances.
fn entrance(m: &impl Mem, idx: u32, t: u32, first: u32, end: u32) -> u32 {
    if first == 0 {
        return m.rebase(0x0132_5b50);
    }
    let mut at = idx.wrapping_shl(5).wrapping_add(first);
    if at.cast_signed() >= end.cast_signed() {
        at = first;
    }
    at.wrapping_add(t)
}

/// `Event_GetEntrance(worker)` (`0x0086bfe0`): the current entrance's
/// record (`TkGetNowEventJumpIndex`) in the worker's script block.
pub fn event_get_entrance(h: &mut impl ffx_bridge::Host, w: u32) -> u32 {
    let t = h.read::<u32>(w + 4);
    let (first, end) = (h.read::<u32>(t + 4), h.read::<u32>(t + 8));
    let idx = host_call(h, TK_GET_NOW_EVENT_JUMP_INDEX, &[]);
    entrance(h, idx, t, first, end)
}

/// `Event_GetEntranceAt(worker, i)` (`0x0086c020`): entrance i's record
/// (the current one for a negative i).
pub fn event_get_entrance_at(h: &mut impl ffx_bridge::Host, w: u32, i: u32) -> u32 {
    let t = h.read::<u32>(w + 4);
    let (first, end) = (h.read::<u32>(t + 4), h.read::<u32>(t + 8));
    let idx = if i.cast_signed() < 0 {
        host_call(h, TK_GET_NOW_EVENT_JUMP_INDEX, &[])
    } else {
        i
    };
    entrance(h, idx, t, first, end)
}

/// `AtelGetHelpEventFlag(i)` (`0x0086b9e0`): bit i (negative 0, byte at
/// most `0x7f`) of `MsGetSaveEventFlag()`'s bytes (`0x00785250`), 0 or 1.
pub fn atel_get_help_event_flag(h: &mut impl ffx_bridge::Host, i: u32) -> u32 {
    let f = host_call(h, 0x0078_5250, &[]);
    let i = i.cast_signed().max(0).cast_unsigned();
    let byte = (i >> 3).min(0x7f);
    u32::from(h.read::<u8>(f.wrapping_add(byte)) & (1 << (i & 7)) != 0)
}

/// `AtelSetUpCtrlWork(i)` (`0x00871190`): ctrl work i's bit 0 set,
/// `g_AtelError` (`0x013270d8`) cleared when its `+0x1c` is 0, and its
/// unset callbacks given their defaults: `+0x20` `AtelGetBasicWork`,
/// `+0x2c` the save event record's `+0x1ec`, `+0x4c` `0x00867710`, `+0x58`
/// `0x00866800`, `+0x5c` `0x008666e0`, `+0x60` `0x0086c4c0`. `EAX` the
/// error mask, or the `+0x2c` it set.
pub fn atel_set_up_ctrl_work(h: &mut impl ffx_bridge::Host, i: u32) -> u32 {
    let c = h.rebase(CTRL_WORKS).wrapping_add(i.wrapping_mul(0x238));
    let b = h.read::<u8>(c);
    h.write(c, b | 1);
    let mut eax = if h.read::<u32>(c + 0x1c) == 0 {
        0
    } else {
        u32::MAX
    };
    let err = h.rebase(0x0132_70d8);
    let e = h.read::<u32>(err);
    h.write(err, e & eax);
    if h.read::<u32>(c + 0x20) == 0 {
        h.write(c + 0x20, h.rebase(0x0086_a830));
    }
    if h.read::<u32>(c + 0x2c) == 0 {
        eax = host_call(h, MS_GET_SAVE_EVENT_ADDRESS, &[]).wrapping_add(0x1ec);
        h.write(c + 0x2c, eax);
    }
    for (off, f) in [
        (0x4c, 0x0086_7710),
        (0x58, 0x0086_6800),
        (0x5c, 0x0086_66e0),
        (0x60, 0x0086_c4c0),
    ] {
        if h.read::<u32>(c + off) == 0 {
            h.write(c + off, h.rebase(f));
        }
    }
    eax
}

/// `AtelSkipReqLevel2(i, level)` (`0x00871730`): `AtelCancelSignals(worker
/// i, level, 1)`.
pub fn atel_skip_req_level2(m: &mut impl Mem, i: u32, level: u32) {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    super::signal::atel_cancel_signals(m, w, level.cast_signed(), 1);
}

/// `AtelGetSummonNameforFinalBattleSelector(i)` (`0x00873b50`):
/// `MsGetSaveChrName(i)` (`0x00784fb0`) below 15, else
/// `TkMn2GetCommandName(0x30ff)` (`0x008c1a20`, whatever i is).
pub fn atel_get_summon_name_for_final_battle_selector(
    h: &mut impl ffx_bridge::Host,
    i: u32,
) -> u32 {
    if i.cast_signed() >= 0xf {
        host_call(h, 0x008c_1a20, &[0x30ff])
    } else {
        host_call(h, 0x0078_4fb0, &[i])
    }
}

/// `AtelMapLoad(map)` (`0x0086dca0`): `TkSetEventJumpMapNo(map, 0)`
/// (`0x0088eaf0`) and, for a positive map, `yiFieldMapRead(map, 0)`
/// (`0x009085d0`); the last one's `EAX`.
pub fn atel_map_load(h: &mut impl ffx_bridge::Host, map: u32) -> u32 {
    let r = host_call(h, 0x0088_eaf0, &[map, 0]);
    if map.cast_signed() > 0 {
        return host_call(h, 0x0090_85d0, &[map, 0]);
    }
    r
}

/// `AtelGetCharHandleFromBindPC(chr)` (`0x0086ae90`): the model (`+0x9c`)
/// of the actor bound to character `chr` (`AtelFindActorByChar`) when it is
/// bound (`+0xaa` 1) and has one, else 0.
pub fn atel_get_char_handle_from_bind_pc(m: &mut impl Mem, chr: u32) -> u32 {
    let i = super::signal::atel_find_actor_by_char(m, chr);
    if i.cast_signed() < 0 {
        return 0;
    }
    let a = super::atel_get_basic_work(m, i.cast_signed());
    let w = super::atel_get_basic_work(m, i.cast_signed());
    if m.read::<u8>(w + 0xaa) == 1 && m.read::<u32>(w + 0x9c) != 0 {
        m.read::<u32>(a + 0x9c)
    } else {
        0
    }
}

/// `AtelBindEff2OffSome(worker, which)` (`0x00862680`): switches off the
/// model's (`+0x9c`) bound effect `which` (0 walk `0x007fb7f0`, 1 breath
/// `0x007fb470`, 2 water `0x007fb890`) and clears its bit of `+0x38` (5, 6,
/// 7); when none of bits 5-7 is left, the drawing and the whole bound
/// effect too (`0x007fb530`, `0x007fb6d0`). `EAX` the last call's, else
/// the flags.
pub fn atel_bind_eff2_off_some(h: &mut impl ffx_bridge::Host, w: u32, which: u32) -> u32 {
    let off = match which {
        0 => Some((0x007f_b7f0, 0xffdf_u16)),
        1 => Some((0x007f_b470, 0xffbf)),
        2 => Some((0x007f_b890, 0xff7f)),
        _ => None,
    };
    if let Some((f, mask)) = off {
        let model = h.read::<u32>(w + 0x9c);
        host_call(h, f, &[model]);
        let v = h.read::<u16>(w + 0x38);
        h.write(w + 0x38, v & mask);
    }
    let v = h.read::<u16>(w + 0x38);
    #[allow(clippy::cast_possible_truncation)] // the low byte, as `cl`
    let al = v as u8;
    let cl = (((al >> 1) | al) >> 1) | al;
    if cl & 0x20 != 0 {
        return u32::from(v);
    }
    let model = h.read::<u32>(w + 0x9c);
    host_call(h, 0x007f_b530, &[model]);
    let model = h.read::<u32>(w + 0x9c);
    host_call(h, 0x007f_b6d0, &[model])
}

/// `AtelCheckCaptMonster(mode, id, min)` (`0x00872aa0`): how many of the
/// `NkzGetMonsterListNum()` (`0x00a5cb20`) monsters of the list at
/// `0x00c86708` (8 bytes each) have at least `min` captures
/// (`MsGetMonsterCapture(i)`, `0x00790af0`, signed): mode 0 those whose
/// `+2` is `id`, mode 1 those whose `+4` is, mode 2 all; 0 for any other
/// mode.
pub fn atel_check_capt_monster(h: &mut impl ffx_bridge::Host, mode: u32, id: u32, min: u32) -> u32 {
    let n = host_call(h, 0x00a5_cb20, &[]).cast_signed();
    let field = match mode {
        0 => Some(2),
        1 => Some(4),
        2 => None,
        _ => return 0,
    };
    let list = h.rebase(0x00c8_6708);
    let mut count = 0_u32;
    for i in 0..n.max(0).cast_unsigned() {
        if let Some(f) = field
            && u32::from(h.read::<u16>(list + i * 8 + f)) != id
        {
            continue;
        }
        if host_call(h, 0x0079_0af0, &[i]).cast_signed() >= min.cast_signed() {
            count += 1;
        }
    }
    count
}

/// `AtelRequestTrigger_008764f0(from, to, kind)`: with ctrl work 0
/// current, a request from `from` to its worker `to` (kind 0) for trigger
/// `kind`, when the worker has that trigger enabled (`+0xab` bit `kind`):
/// at the level `AtelSoundTableGet_00876860(type, kind)` gives and the
/// entry `TkGetVibrateCurve(type, kind, worker +0x44)` (`0x008763e0`)
/// gives, unless they are `0xff`/`0xffff`. The request's result, else 0.
pub fn atel_request_trigger(h: &mut impl ffx_bridge::Host, from: u32, to: u32, kind: u32) -> u32 {
    let old = super::atel_change_current_ctrl(h, 0);
    let w = super::atel_get_basic_work(h, to.cast_signed());
    let t = super::atel_get_actor_type(h, to.cast_signed());
    let lvl = atel_sound_table_get(h, t, kind);
    let curve = h.read::<u32>(w + 0x44);
    let entry = host_call(h, 0x0087_63e0, &[t, kind, curve]);
    let mut r = 0;
    #[allow(clippy::cast_possible_truncation)] // `dl`, the bit's low byte
    let bit = 1_u32.wrapping_shl(kind) as u8;
    if lvl != 0xff && entry != 0xffff && h.read::<u8>(w + 0xab) & bit != 0 {
        r = super::signal::atel_request(h, super::signal::Request::new(from, to, 0, lvl, entry));
    }
    super::atel_set_current_ctrl(h, old);
    r
}

// ---- script round 22 ----

/// `Atel_ClearBindMask(n)` (`0x00863650`): [`atel_set_bind_mask`]'s
/// counterpart, bind-mask bit n (below 0x200, signed division) cleared and
/// the union `0x01326b94` recomputed. Returns the `EAX` it leaves (the
/// loop's end address; n itself for n >= 0x200, where the original leaves
/// the caller's).
pub fn atel_clear_bind_mask(m: &mut impl Mem, n: u32) -> u32 {
    let k = n.cast_signed();
    if k >= 0x200 {
        return n;
    }
    let idx = k / 32;
    let bit = 1_u32.wrapping_shl((k % 32).cast_unsigned());
    let at = m
        .rebase(BIND_MASKS)
        .wrapping_add(idx.cast_unsigned().wrapping_mul(4));
    let v = m.read::<u32>(at);
    m.write(at, v & !bit);
    let base = m.rebase(BIND_MASKS);
    let mut all = 0;
    for i in 0..16 {
        all |= m.read::<u32>(base + 4 * i);
    }
    m.write(m.rebase(BIND_MASK_ALL), all);
    base + 0x48
}

/// `g_EventSprites` (`0x01327148`): 128 records of `0x1c` bytes.
const EVENT_SPRITES: u32 = 0x0132_7148;

/// A sprite slot clamped to `0..=0x7f`, and a byte value to `0..=0xff`.
fn sprite_slot(i: u32) -> u32 {
    i.cast_signed().clamp(0, 0x7f).cast_unsigned()
}

fn byte_value(v: u32) -> u8 {
    // the clamp makes it fit
    u8::try_from(v.cast_signed().clamp(0, 0xff)).unwrap_or(0)
}

/// `Atel_SetSpriteAlpha(i, alpha)` (`0x00874ad0`): sprite i's `+1` (alpha)
/// = alpha, i clamped to `0..=0x7f` and alpha to `0..=0xff`. Returns the
/// `EAX` it leaves, `i * 7`.
pub fn atel_set_sprite_alpha(m: &mut impl Mem, i: u32, alpha: u32) -> u32 {
    let i = sprite_slot(i);
    let at = m.rebase(EVENT_SPRITES) + i * 0x1c + 1;
    m.write(at, byte_value(alpha));
    i * 7
}

/// `Atel_SetSpriteColor(i, r, g, b)` (`0x008749f0`): sprite i's `+0x10`,
/// `+0x11`, `+0x12` = r, g, b, each clamped to `0..=0xff` (i to
/// `0..=0x7f`). Returns the `EAX` it leaves, `i * 7`.
pub fn atel_set_sprite_color(m: &mut impl Mem, i: u32, r: u32, g: u32, b: u32) -> u32 {
    let i = sprite_slot(i);
    let at = m.rebase(EVENT_SPRITES) + i * 0x1c;
    let (r, g, b) = (byte_value(r), byte_value(g), byte_value(b));
    m.write(at + 0x11, g);
    m.write(at + 0x10, r);
    m.write(at + 0x12, b);
    i * 7
}

/// `Atel_GetVoiceStreamParams(id, &a, &b)` (`0x00885ba0`): the voice id's
/// decimal digits 4-5 of `id >> 12` (`(id >> 12) / 10000 % 100`) below 21
/// give `a` = 0x19, `b` = 1, else `a` = 0x1b, `b` = 0. Returns the `EAX` it
/// leaves, `&b`.
pub fn atel_get_voice_stream_params(m: &mut impl Mem, id: u32, a: u32, b: u32) -> u32 {
    let low = (id >> 12) / 10000 % 100 < 0x15;
    m.write(a, if low { 0x19_u32 } else { 0x1b });
    m.write(b, u32::from(low));
    b
}

/// One byte of `Atel_NameToLang1` (`0x00875220`): `0x5a..=0x73` down by 10,
/// `0x74..=0x8d` down by 4, `>` `R` `S` to `<` `j` `l`, the rest kept.
fn name_to_lang1(c: u8) -> u8 {
    match c {
        0x5a..=0x73 => c - 10,
        0x74..=0x8d => c - 4,
        b'>' => b'<',
        b'R' => b'j',
        b'S' => b'l',
        _ => c,
    }
}

/// One byte of `Atel_NameFromLang1` (`0x00875370`), the inverse:
/// `0x50..=0x69` up by 10, `0x70..=0x89` up by 4, `<` `j` `l` to `>` `R`
/// `S`, the rest kept.
fn name_from_lang1(c: u8) -> u8 {
    match c {
        0x50..=0x69 => c + 10,
        0x70..=0x89 => c + 4,
        b'<' => b'>',
        b'j' => b'R',
        b'l' => b'S',
        _ => c,
    }
}

/// Copies the C string at `src` to `dst` through `f`, byte by byte (the
/// read of each byte follows the write of the one before, as in the
/// original, so overlapping buffers behave alike). Returns the address of
/// the terminating NUL written, the `EAX` the originals leave.
fn map_name(m: &mut impl Mem, src: u32, dst: u32, f: fn(u8) -> u8) -> u32 {
    let mut i = 0_u32;
    loop {
        let c = m.read::<u8>(src.wrapping_add(i));
        let at = dst.wrapping_add(i);
        if c == 0 {
            m.write(at, 0_u8);
            return at;
        }
        m.write(at, f(c));
        i = i.wrapping_add(1);
    }
}

/// `Atel_NameToLang1(src, dst)` (`0x00875220`): a character name copied
/// into language 1's encoding (`setDefaultPlayerName`, `0x00787470`).
pub fn atel_name_to_lang1(m: &mut impl Mem, src: u32, dst: u32) -> u32 {
    map_name(m, src, dst, name_to_lang1)
}

/// `Atel_NameFromLang1(src, dst)` (`0x00875370`): the inverse, for
/// languages 9 and 10.
pub fn atel_name_from_lang1(m: &mut impl Mem, src: u32, dst: u32) -> u32 {
    map_name(m, src, dst, name_from_lang1)
}

/// Whether the worker is of type 5 or 6, and the `EAX` the type test
/// leaves otherwise (the header pointer with its low byte the type minus
/// 5).
fn kind56(m: &impl Mem, w: u32) -> Result<(), u32> {
    let h = m.read::<u32>(w);
    let t = m.read::<u8>(h).wrapping_sub(5);
    if t < 2 {
        Ok(())
    } else {
        Err(h & !0xff | u32::from(t))
    }
}

/// `Atel_ResetLineTable(worker)` (`0x0086d110`): for types 5 and 6 with a
/// table at `+0x2e4`, its eight `(key u32, value u8)` slots (8 bytes each,
/// the ones opcode `7a` sets) = `(0xffff, 3)`. Returns the `EAX` it leaves:
/// the table, or the type test's.
pub fn atel_reset_line_table(m: &mut impl Mem, w: u32) -> u32 {
    if let Err(eax) = kind56(m, w) {
        return eax;
    }
    let t = m.read::<u32>(w + 0x2e4);
    if t != 0 {
        for i in 0..8 {
            m.write(t + i * 8, 0xffff_u32);
            m.write(t + i * 8 + 4, 3_u8);
        }
    }
    t
}

/// `Atel_FreeLineSignals(worker)` (`0x0086d1c0`): for types 5 and 6 with a
/// table at `+0x2e4`, its 16 signal nodes (`+0x40`, 0x18 bytes each) are
/// pushed one by one onto the worker's free list (`+0x88`): the node's
/// `next` = the old head, its `prev` = the list head, the old head's `prev`
/// = the node. Returns the `EAX` it leaves: the table `+ 0x1dc`, 0 without
/// one, or the type test's.
pub fn atel_free_line_signals(m: &mut impl Mem, w: u32) -> u32 {
    if let Err(eax) = kind56(m, w) {
        return eax;
    }
    let t = m.read::<u32>(w + 0x2e4);
    if t == 0 {
        return 0;
    }
    let head = w + 0x88;
    for i in 0..16 {
        let node = t + 0x40 + i * 0x18;
        let old = m.read::<u32>(head);
        if old != 0 {
            m.write(old + 4, node);
        }
        m.write(head, node);
        m.write(node + 4, head);
        m.write(node, old);
    }
    t + 0x1dc
}

/// `Atel_SetLineTable(worker, table)` (`0x0086d190`): for types 5 and 6
/// `+0x2e4` = table; then [`atel_free_line_signals`] and
/// [`atel_reset_line_table`], whose `EAX` it returns.
pub fn atel_set_line_table(m: &mut impl Mem, w: u32, table: u32) -> u32 {
    if kind56(m, w).is_ok() {
        m.write(w + 0x2e4, table);
    }
    atel_free_line_signals(m, w);
    atel_reset_line_table(m, w)
}

/// The move or turn variant of the block helpers below.
struct BlockKind {
    /// Where a level block (`+0x12c + level*0x4c`) keeps the pointer.
    slot: u32,
    /// The worker's own blocks: `+base + level*stride`.
    base: u32,
    stride: u32,
    /// The mode bits that mark a block busy (`+2`).
    busy: u16,
}

const MOVE_BLOCK: BlockKind = BlockKind {
    slot: 0x44,
    base: 0x5b8,
    stride: 0x4c,
    busy: 0xff80,
};
const TURN_BLOCK: BlockKind = BlockKind {
    slot: 0x48,
    base: 0x864,
    stride: 0x44,
    busy: 0xffe0,
};

/// `0x00870440`/`0x00871090(worker, level, mask, mode)`: the level's own
/// block (`level` unclamped) becomes the clamped level's block, and that of
/// every level `i` up to `level` (7 with mode bit `0x20`) whose bit is in
/// `mask | 1 << level`; with mode bit `0x100` only levels whose current
/// block is idle (`+2 & busy` clear). Returns the `EAX` the loop leaves.
fn share_block(m: &mut impl Mem, k: &BlockKind, w: u32, level: u32, mask: u32, mode: u32) -> u32 {
    let slot = |l: u32| w + 0x12c + l * 0x4c + k.slot;
    let block = w
        .wrapping_add(k.base)
        .wrapping_add(level.wrapping_mul(k.stride));
    let at = slot(clamp_level(m, w, level));
    m.write(at, block);
    let mask = mask | 1 << (level & 31);
    let last = if mode & 0x20 != 0 { 7 } else { level };
    let mut eax = 1 << (level & 31);
    let mut i = 0_u32;
    while i.cast_signed() <= last.cast_signed() {
        let l = clamp_level(m, w, i);
        eax = l * 0x4c;
        if mask & 1_u32.rotate_left(i) != 0 {
            let idle = if mode & 0x100 == 0 {
                true
            } else {
                eax = m.read::<u32>(slot(l));
                m.read::<u16>(eax + 2) & k.busy == 0
            };
            if idle {
                eax = block;
                m.write(slot(l), block);
            }
        }
        i = i.wrapping_add(1);
    }
    eax
}

/// `Atel_ShareMoveBlock` (`0x00870440`): [`share_block`] for the move
/// blocks (`+0x5b8 + level*0x4c`, level block `+0x44`, busy `0xff80`).
pub fn atel_share_move_block(m: &mut impl Mem, w: u32, level: u32, mask: u32, mode: u32) -> u32 {
    share_block(m, &MOVE_BLOCK, w, level, mask, mode)
}

/// `Atel_ShareTurnBlock` (`0x00871090`): [`share_block`] for the turn
/// blocks (`+0x864 + level*0x44`, level block `+0x48`, busy `0xffe0`).
pub fn atel_share_turn_block(m: &mut impl Mem, w: u32, level: u32, mask: u32, mode: u32) -> u32 {
    share_block(m, &TURN_BLOCK, w, level, mask, mode)
}

/// `0x008703e0`/`0x00871030(worker, mask, mode, param)`: the current
/// level's own block gets `+0` = param, `+2` = mode, `+6` = 0 (16 bits
/// each), then is shared with the levels in `mask` ([`share_block`]).
fn start_block(m: &mut impl Mem, k: &BlockKind, w: u32, mask: u32, mode: u32, param: u32) -> u32 {
    let level = u32::from(m.read::<u8>(w + 0x32));
    let block = w + k.base + level * k.stride;
    #[allow(clippy::cast_possible_truncation)] // 16-bit fields
    {
        m.write(block, param as u16);
        m.write(block + 2, mode as u16);
    }
    m.write(block + 6, 0_u16);
    share_block(m, k, w, level, mask, mode)
}

/// `Atel_StartMoveBlock` (`0x008703e0`, `Native_MoveStart`'s tail).
pub fn atel_start_move_block(m: &mut impl Mem, w: u32, mask: u32, mode: u32, param: u32) -> u32 {
    start_block(m, &MOVE_BLOCK, w, mask, mode, param)
}

/// `Atel_StartTurnBlock` (`0x00871030`, `Native_TurnStart`'s tail).
pub fn atel_start_turn_block(m: &mut impl Mem, w: u32, mask: u32, mode: u32, param: u32) -> u32 {
    start_block(m, &TURN_BLOCK, w, mask, mode, param)
}

/// `g_AtelSepLoadCount` (`0x01325b78`, `i32`) and `g_AtelSepLoadList`
/// (`0x01327030`, 8 x `(id, flags)`): the sound separates an event asks
/// for; `g_AtelSepGroupMask` (`0x01326bdc`, `u8`) the groups selected.
const SEP_LOAD_COUNT: u32 = 0x0132_5b78;
const SEP_LOAD_LIST: u32 = 0x0132_7030;
const SEP_GROUP_MASK: u32 = 0x0132_6bdc;

/// `Atel_AddSepLoad(id, flags)` (`0x0086e250`): with fewer than 8 entries,
/// -1 when `id` is listed already; else, when the flags' group bits
/// (`0x7e0000`, none = group 0) meet the selected groups (`mask | 1` from
/// bit 16), appends `(id, flags | 0x8000)` and returns 0. Else 1.
pub fn atel_add_sep_load(m: &mut impl Mem, id: u32, flags: u32) -> u32 {
    let count = m.rebase(SEP_LOAD_COUNT);
    let n = m.read::<i32>(count);
    if n >= 8 {
        return 1;
    }
    let base = m.rebase(SEP_LOAD_LIST);
    let list = |i: i32| base + i.cast_unsigned() * 8;
    if (0..n).any(|i| m.read::<u32>(list(i)) == id) {
        return u32::MAX;
    }
    let groups = if flags & 0x7e_0000 == 0 {
        flags | 0x1_0000
    } else {
        flags
    };
    let selected = u32::from(m.read::<u8>(m.rebase(SEP_GROUP_MASK)) | 1) << 16;
    if groups & selected == 0 {
        return 1;
    }
    m.write(list(n), id);
    m.write(list(n) + 4, flags | 0x8000);
    m.write(count, n + 1);
    0
}

/// The worker's position block: `+0x284` for types 5 and 6, `+0x558`
/// otherwise (`AtelGetPosWorkAdrs`), and 0 for type 4.
fn pos_block(m: &impl Mem, w: u32) -> u32 {
    match kind(m, w) {
        4 => 0,
        5 | 6 => w.wrapping_add(0x284),
        _ => w.wrapping_add(0x558),
    }
}

const YI_SET_ROT_TRANS_GROUP: u32 = 0x0090_a830;
const YI_SET_ROT_TRANS_DOBJ: u32 = 0x0090_a750;

/// `AtelSetBindObject(worker)` (`0x0086f4c0`): the map object the worker
/// is bound to (`+0xa8`, `u16`; `+0xaa` 4 a group, 5 one object) gets the
/// position block's rotation (`+0x30`..`+0x38`, w 0) and translation
/// (`+0`..`+8`, w 1) through `yiSetRotTransGroup`/`yiSetRotTransDObj`,
/// built in `frame` (32 bytes: the original's locals). Returns that
/// call's `EAX`, else the block pointer with `+0xaa` in its low byte.
/// Type 4 workers have no block (the original reads through 0).
pub fn atel_set_bind_object(h: &mut impl ffx_bridge::Host, w: u32, frame: u32) -> u32 {
    let p = pos_block(h, w);
    let (rot, trans) = (frame, frame + 0x10);
    for k in 0..3 {
        copy_f32(h, p.wrapping_add(0x30 + 4 * k), rot + 4 * k);
    }
    h.write(rot + 0xc, 0_u32);
    for k in 0..3 {
        copy_f32(h, p.wrapping_add(4 * k), trans + 4 * k);
    }
    h.write(trans + 0xc, x87::store(1.0));
    let bind = h.read::<u8>(w + 0xaa);
    let f = match bind {
        4 => YI_SET_ROT_TRANS_GROUP,
        5 => YI_SET_ROT_TRANS_DOBJ,
        _ => return p & !0xff | u32::from(bind),
    };
    let id = u32::from(h.read::<u16>(w + 0xa8));
    host_call(h, f, &[id, rot, trans])
}

const MS_CAMERA_SET_RECT: u32 = 0x007b_f940;
const CH_SET_POS3: u32 = 0x0082_b480;
const CH_CLOTH_RESET: u32 = 0x0082_4650;
const CH_ID_GET_ENCOUNT: u32 = 0x0083_d820;
const CH_ID_GET_DICTIONARY: u32 = 0x0083_d7a0;
const TK_GET_SAVE_EVENT_ADDRESS: u32 = 0x0088_c830;

/// `AtelSetPos3d(worker, x, y, z, snap)` (`0x00870b20`): the position
/// block's `+0`..`+8` = (x, y, z) (`f32`), then by `+0xaa`: 1 the
/// character (`+0x9c`, when set) `Ch_SetPos3` and `Ch_ClothReset`, 2 the
/// camera `MsCameraSetRect(+0x3c, 1, &(x, y, z, 1))` (in `frame`, 16
/// bytes). For the player worker (`+0x2e` = the ctrl work's `+0xa`) the
/// ctrl work's position (`+0x218`, w `+0x224`) moves to the previous one
/// (`+0x228`) and becomes the block's, w 1; without `+0x34` bit 4, or with
/// `snap`, the previous one too; the save event's encounter byte (`+0x10`)
/// and dictionary index come from the character (0 without one). Without
/// bit 4, or with `snap`, the block's `+0x10`..`+0x18` = the position too.
/// The ctrl work's `+2` bit 4 is set. Returns `snap` (the header's
/// pointer with its type in the low byte for type 4, which is left alone).
pub fn atel_set_pos_3d(
    h: &mut impl ffx_bridge::Host,
    w: u32,
    pos: [u32; 3],
    snap: u32,
    frame: u32,
) -> u32 {
    let hdr = h.read::<u32>(w);
    let t = h.read::<u8>(hdr);
    if t == 4 {
        return hdr & !0xff | u32::from(t);
    }
    let p = pos_block(h, w);
    if p == 0 {
        return hdr & !0xff | u32::from(t.wrapping_sub(5));
    }
    let [x, y, z] = pos.map(|v| x87::store(x87::load(v)));
    h.write(p, x);
    h.write(p + 4, y);
    h.write(p + 8, z);
    let one = x87::store(1.0);
    match h.read::<u8>(w + 0xaa) {
        1 => {
            let c = h.read::<u32>(w + 0x9c);
            if c != 0 {
                host_call(h, CH_SET_POS3, &[c, x, y, z]);
                let c = h.read::<u32>(w + 0x9c);
                host_call(h, CH_CLOTH_RESET, &[c]);
            }
        }
        2 => {
            h.write(frame, [x, y, z, one]);
            let cam = h.read::<u32>(w + 0x3c);
            host_call(h, MS_CAMERA_SET_RECT, &[cam, 1, frame]);
        }
        _ => {}
    }
    // the original tests +0x34 twice, around the save-event calls
    let keep = |h: &mut _| ffx_bridge::Mem::read::<u8>(h, w + 0x34) & 0x10 != 0 && snap == 0;
    let mut ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    if h.read::<u16>(w + 0x2e) == h.read::<u16>(ctx + 0xa) {
        for k in 0..4 {
            copy_f32(h, ctx + 0x218 + 4 * k, ctx + 0x228 + 4 * k);
        }
        for k in 0..3 {
            copy_f32(h, p + 4 * k, ctx + 0x218 + 4 * k);
        }
        h.write(ctx + 0x224, one);
        if !keep(h) {
            for k in 0..3 {
                copy_f32(h, ctx + 0x218 + 4 * k, ctx + 0x228 + 4 * k);
            }
            h.write(ctx + 0x234, one);
        }
        let c = h.read::<u32>(w + 0x9c);
        let dic = if c == 0 {
            let e = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
            h.write(e + 0x10, 0_u8);
            0
        } else {
            #[allow(clippy::cast_possible_truncation)] // the original stores BL
            let enc = host_call(h, CH_ID_GET_ENCOUNT, &[c]) as u8;
            let e = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
            h.write(e + 0x10, enc);
            let c = h.read::<u32>(w + 0x9c);
            host_call(h, CH_ID_GET_DICTIONARY, &[c])
        };
        atel_set_save_dic_index(h, dic);
        ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    }
    if !keep(h) {
        for k in 0..3 {
            copy_f32(h, p + 4 * k, p + 0x10 + 4 * k);
        }
    }
    let f = h.read::<u8>(ctx + 2);
    h.write(ctx + 2, f | 0x10);
    snap
}

/// Whether the player may be given control back: the ctrl work's `+1`
/// bit 2 and `+3` bits `0x84` clear (read after `AtelUcOff`), and it had
/// control before (`+0` bits `0x22`).
fn uc_may_resume(h: &impl Mem, had: bool) -> bool {
    let c = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    h.read::<u8>(c + 1) & 4 == 0 && h.read::<u8>(c + 3) & 0x84 == 0 && had
}

/// `Atel_UcResume()` (`0x0086a130`): `AtelUcOff`, then `AtelUcOn` when
/// the player had control (ctrl work `+0` bits `0x22`) and the ctrl work's
/// `+1` bit 2 and `+3` bits `0x84` are clear. (`EAX` is a leftover; no
/// caller reads it.)
pub fn atel_uc_resume(h: &mut impl ffx_bridge::Host) {
    let c = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    let had = h.read::<u8>(c) & 0x22 != 0;
    atel_uc_off(h);
    if uc_may_resume(h, had) {
        atel_uc_on(h);
    }
}

/// `Atel_UcRestore()` (`0x0086a0e0`): [`atel_uc_resume`], then the ctrl
/// work's `+0` bit 1 is what it was before. (`EAX` is a leftover; no
/// caller reads it.)
pub fn atel_uc_restore(h: &mut impl ffx_bridge::Host) {
    let c = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    let flags = h.read::<u8>(c);
    atel_uc_off(h);
    if uc_may_resume(h, flags & 0x22 != 0) {
        atel_uc_on(h);
    }
    let c = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    let now = h.read::<u8>(c);
    h.write(c, now & !2 | flags & 2);
}

/// `Atel_GetWorkerRotY(i)` (`0x0086a6e0`): `ST0` = the position block's
/// `+0x34` (`f32`) of worker `i` (`AtelGetBasicWork`). Type 4 workers have
/// no block (the original reads through 0).
pub fn atel_get_worker_rot_y(m: &mut impl Mem, i: u32) -> f32 {
    let w = super::super::bridge::atel_get_basic_work(m, i.cast_signed());
    f32::from_bits(m.read(pos_block(m, w).wrapping_add(0x34)))
}

/// `Atel_SyncChrDir(worker)` (`0x00869df0`): a worker bound to its
/// character (`+0xaa` 1, `+0x9c` set) takes the character's direction
/// (`Ch_GetDir`, `dir`) into its position block's `+0x20` (`fstp m32`).
pub fn atel_sync_chr_dir<M: Mem>(m: &mut M, w: u32, dir: impl FnOnce(&mut M, u32) -> f64) {
    let p = pos_block(m, w);
    let c = m.read::<u32>(w + 0x9c);
    if m.read::<u8>(w + 0xaa) == 1 && c != 0 {
        let d = x87::store(dir(m, c));
        m.write(p.wrapping_add(0x20), d);
    }
}

const OP_ET_BINDEFF2: u32 = 0x007f_b360;
const OP_ET_BINDEFF2_DRAW_ON: u32 = 0x007f_b5a0;
const OP_ET_BINDEFF2_WALK_ON: u32 = 0x007f_b830;
const OP_ET_BINDEFF2_BREATH_ON: u32 = 0x007f_b4c0;
const OP_ET_BINDEFF2_WATER_ON: u32 = 0x007f_b8e0;

/// `AtelBindEff2On(worker, kind)` (`0x00862700`): the character's
/// (`+0x9c`) bind effect 2 (`op_et_bindeff2`, then `_draw_on`), and for
/// kind 0 `_walk_on` (`+0x38` bit 5), 1 `_breath_on` (bit 6), 2
/// `_water_on` (bit 7). `EAX` the last call's, 0x80 for kind 2, else
/// `kind - 2`.
pub fn atel_bind_eff2_on(h: &mut impl ffx_bridge::Host, w: u32, kind: u32) -> u32 {
    for f in [OP_ET_BINDEFF2, OP_ET_BINDEFF2_DRAW_ON] {
        let c = h.read::<u32>(w + 0x9c);
        host_call(h, f, &[c]);
    }
    let (f, bit) = match kind {
        0 => (OP_ET_BINDEFF2_WALK_ON, 0x20),
        1 => (OP_ET_BINDEFF2_BREATH_ON, 0x40),
        2 => (OP_ET_BINDEFF2_WATER_ON, 0x80),
        _ => return kind.wrapping_sub(2),
    };
    let c = h.read::<u32>(w + 0x9c);
    let eax = host_call(h, f, &[c]);
    let v = h.read::<u16>(w + 0x38);
    h.write(w + 0x38, v | bit);
    if kind == 2 { bit.into() } else { eax }
}

/// `g_AtelSepQueueCount` (`0x0132707c`), then the queue from 1:
/// `g_AtelSepQueueIds` (`0x01327080`) and `g_AtelSepQueueFades`
/// (`0x013270a0`), 8 each.
const SEP_QUEUE_COUNT: u32 = 0x0132_707c;
const SEP_QUEUE_FADES: u32 = 0x0132_709c;

/// The event header's sound-separate table (`+0x2c`, then its `+0xc`:
/// `n`, then `n` `(id, flags)` pairs; offsets from the header), or 0.
fn sep_table(m: &impl Mem, b: u32) -> u32 {
    let t = m.read::<u32>(b + 0x2c);
    if t == 0 || b.wrapping_add(t) == 0 {
        return 0;
    }
    let u = m.read::<u32>(b.wrapping_add(t) + 0xc);
    if u == 0 { 0 } else { b.wrapping_add(u) }
}

/// `Atel_QueueSepLoads(header)` (`0x008638c0`): the queue is emptied, then
/// every pending entry of `g_AtelSepLoadList` (flags bit `0x8000`) whose id
/// is not in the event header's separates table is queued with a fade of
/// 0xf0 frames (0x78 with flag `0x2000`, 0x168 with `0x4000`) and loses
/// the pending bit. Returns the table's count (0 without one).
pub fn atel_queue_sep_loads(m: &mut impl Mem, b: u32) -> u32 {
    let table = sep_table(m, b);
    let n = if table == 0 { 0 } else { m.read::<i32>(table) };
    let count = m.rebase(SEP_QUEUE_COUNT);
    let fades = m.rebase(SEP_QUEUE_FADES);
    let list = m.rebase(SEP_LOAD_LIST);
    let loads = m.rebase(SEP_LOAD_COUNT);
    m.write(count, 0_u32);
    let mut i = 0_u32;
    while i.cast_signed() < m.read::<i32>(loads) {
        let entry = list + i * 8;
        let flags = m.read::<u32>(entry + 4);
        let id = m.read::<u32>(entry);
        let listed = flags & 0x8000 == 0
            || (0..n).any(|j| {
                let t = sep_table(m, b);
                let tid = if t == 0 {
                    0
                } else {
                    m.read::<u32>(t.wrapping_add(j.cast_unsigned() * 8 + 4))
                };
                tid == m.read::<u32>(entry)
            });
        if !listed {
            let fade = if flags & 0x4000 != 0 {
                0x168_u32
            } else if flags & 0x2000 != 0 {
                0x78
            } else {
                0xf0
            };
            let k = m.read::<u32>(count).wrapping_add(1);
            m.write(count.wrapping_add(k * 4), id);
            m.write(fades.wrapping_add(k * 4), fade);
            m.write(count, k);
            m.write(entry + 4, flags & !0x8000);
        }
        i += 1;
    }
    n.cast_unsigned()
}

// ---- script round 23 (this machine) ----

/// The event's worker `i` (ctrl work 0 current meanwhile, as the
/// `...FromID` accessors do).
fn event_worker(m: &mut impl Mem, i: u32) -> u32 {
    with_event_ctrl(m, |m| super::atel_get_basic_work(m, i.cast_signed()))
}

/// `AtelGetEventActorLinkWork(i)` (`0x0086b1b0`): the link byte `+0x33`
/// of the event's worker i (`mov al`: `EAX` keeps the worker's upper
/// bytes).
pub fn atel_get_event_actor_link_work(m: &mut impl Mem, i: u32) -> u32 {
    let w = event_worker(m, i);
    w & !0xff | u32::from(m.read::<u8>(w + 0x33))
}

/// `AtelSetEventActorLinkWork(i, v)` (`0x0086fc40`): the event's worker
/// i's `+0x33` = the low byte of v; `EAX` the worker.
pub fn atel_set_event_actor_link_work(m: &mut impl Mem, i: u32, v: u32) -> u32 {
    let w = event_worker(m, i);
    m.write(w + 0x33, v.to_le_bytes()[0]);
    w
}

/// `AtelGetEventBindIdFromID(i)` (`0x0086b370`): the binding byte `+0xaa`
/// of the event's worker i.
pub fn atel_get_event_bind_id_from_id(m: &mut impl Mem, i: u32) -> u32 {
    let w = event_worker(m, i);
    u32::from(m.read::<u8>(w + 0xaa))
}

/// `AtelGetEventCharIDFromID(i)` (`0x0086b3d0`): the character id
/// `+0xa8` (`u16`) of the event's worker i.
pub fn atel_get_event_char_id_from_id(m: &mut impl Mem, i: u32) -> u32 {
    let w = event_worker(m, i);
    u32::from(m.read::<u16>(w + 0xa8))
}

/// `AtelGetEventElemCharIDFromID(i, k)` (`0x0086b400`): the character id
/// of the event's worker i's equipment element k (`u16` at `+0xb04 + 2k`).
pub fn atel_get_event_elem_char_id_from_id(m: &mut impl Mem, i: u32, k: u32) -> u32 {
    let w = event_worker(m, i);
    u32::from(m.read::<u16>(w.wrapping_add(k.wrapping_mul(2)) + 0xb04))
}

/// `AtelGetEventElemHandleFromID(i, k)` (`0x0086b440`): the model of the
/// event's worker i's element k (`+0xaf4 + 4k`, what
/// `AtelSetEventElemHandleFromID` stores).
pub fn atel_get_event_elem_handle_from_id(m: &mut impl Mem, i: u32, k: u32) -> u32 {
    let w = event_worker(m, i);
    m.read::<u32>(w.wrapping_add(k.wrapping_mul(4)) + 0xaf4)
}

/// `AtelGetEventElemPosFromID(i, k)` (`0x0086b480`): the attach word of
/// the event's worker i's element k (`+0xafc + 4k`, what
/// `AtelSetEventElemPosFromID` stores).
pub fn atel_get_event_elem_pos_from_id(m: &mut impl Mem, i: u32, k: u32) -> u32 {
    let w = event_worker(m, i);
    m.read::<u32>(w.wrapping_add(k.wrapping_mul(4)) + 0xafc)
}

/// `AtelSetEventElemHandleFromID(i, k, v)` (`0x0086fde0`): `+0xaf4 + 4k`
/// of the event's worker i = v; `EAX` the worker.
pub fn atel_set_event_elem_handle_from_id(m: &mut impl Mem, i: u32, k: u32, v: u32) -> u32 {
    let w = event_worker(m, i);
    m.write(w.wrapping_add(k.wrapping_mul(4)) + 0xaf4, v);
    w
}

/// `AtelSetEventElemPosFromID(i, k, v)` (`0x0086fe20`): `+0xafc + 4k` of
/// the event's worker i = v; `EAX` the worker.
pub fn atel_set_event_elem_pos_from_id(m: &mut impl Mem, i: u32, k: u32, v: u32) -> u32 {
    let w = event_worker(m, i);
    m.write(w.wrapping_add(k.wrapping_mul(4)) + 0xafc, v);
    w
}

/// `AtelSetEventElemCharIDFromID(i, k, v)` (`0x0086fda0`): `+0xb04 + 2k`
/// of the event's worker i = the low half of v; `EAX` the worker.
pub fn atel_set_event_elem_char_id_from_id(m: &mut impl Mem, i: u32, k: u32, v: u32) -> u32 {
    let w = event_worker(m, i);
    let [lo, hi, _, _] = v.to_le_bytes();
    m.write(
        w.wrapping_add(k.wrapping_mul(2)) + 0xb04,
        u16::from_le_bytes([lo, hi]),
    );
    w
}

/// `AtelSetEventCharHandleFromID(i, model)` (`0x0086fd60`): the model
/// `+0x9c` of the event's worker i, when it is bound to a character
/// (`+0xaa` 1); `EAX` the worker.
pub fn atel_set_event_char_handle_from_id(m: &mut impl Mem, i: u32, model: u32) -> u32 {
    let w = event_worker(m, i);
    if m.read::<u8>(w + 0xaa) == 1 {
        m.write(w + 0x9c, model);
    }
    w
}

/// `AtelGetEventElemStatus(i, k)` (`0x0086b4c0`): 1 when bit k (mod 32)
/// of the event's worker i's element mask `+0xb14` (`u16`) is set, else 0.
pub fn atel_get_event_elem_status(m: &mut impl Mem, i: u32, k: u32) -> u32 {
    let w = event_worker(m, i);
    let mask = u32::from(m.read::<u16>(w + 0xb14));
    u32::from(mask & 1_u32 << (k & 31) != 0)
}

/// `AtelSetEventElemStatus(i, k, on)` (`0x0086fe60`): sets (on non-zero)
/// or clears bit k of the event's worker i's element mask `+0xb14`
/// (a 16-bit `shl`: k mod 32 at 16 and above is no bit). `EAX` the worker
/// when setting, else the inverted mask word.
pub fn atel_set_event_elem_status(m: &mut impl Mem, i: u32, k: u32, on: u32) -> u32 {
    let w = event_worker(m, i);
    let bit = 1_u16.checked_shl(k & 31).unwrap_or(0);
    let at = w + 0xb14;
    let old = m.read::<u16>(at);
    if on != 0 {
        m.write(at, old | bit);
        w
    } else {
        m.write(at, old & !bit);
        u32::from(!bit)
    }
}

/// `AtelGetEventActorType(i)` (`0x0086b320`): the type byte of the
/// event's worker i, 7 outside `0..ACTORS` of ctrl work 0.
pub fn atel_get_event_actor_type(m: &mut impl Mem, i: u32) -> u32 {
    with_event_ctrl(m, |m| {
        let n = i32::from(m.read::<u16>(m.rebase(CTRL_WORKS + 0xc)));
        if i.cast_signed() < 0 || i.cast_signed() >= n {
            return 7;
        }
        let w = super::atel_get_basic_work(m, i.cast_signed());
        u32::from(kind(m, w))
    })
}

/// `AtelGetHideStatusFromID(i)` (`0x0086ba40`): `+0xa0` of the current
/// ctrl work's worker i.
pub fn atel_get_hide_status_from_id(m: &mut impl Mem, i: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    m.read::<u32>(w + 0xa0)
}

/// `AtelGetKeepBattlePartyFromID(i)` (`0x0086bbf0`): bit 8 of the
/// current ctrl work's worker i's flags `+0x38` (`u16`).
pub fn atel_get_keep_battle_party_from_id(m: &mut impl Mem, i: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    u32::from(m.read::<u16>(w + 0x38)) >> 8 & 1
}

/// `AtelGetRequestLevel(i)` (`0x0086c3f0`): the request level byte
/// `+0x32` of the current ctrl work's worker i.
pub fn atel_get_request_level(m: &mut impl Mem, i: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    u32::from(m.read::<u8>(w + 0x32))
}

/// The work size `+0x10` of worker w's header, rounded up to 16 as the
/// original's signed `cdq` rounding does.
fn my_work_size(m: &impl Mem, w: u32) -> i32 {
    let hdr = m.read::<u32>(w);
    let t = m.read::<i32>(hdr + 0x10).wrapping_add(15);
    t.wrapping_add(if t < 0 { 15 } else { 0 }) & !15
}

/// `AtelGetMyWorkSize(i)` (`0x0086c160`): the current ctrl work's worker
/// i's work size (its header's `+0x10`) rounded up to 16 (towards zero
/// for negative sizes).
pub fn atel_get_my_work_size(m: &mut impl Mem, i: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    my_work_size(m, w).cast_unsigned()
}

/// `AtelSetMyWorkAdrs(i, at)` (`0x00870870`): the current ctrl work's
/// worker i's own work `+0xb8` = at, then the work cleared: `size`
/// (header `+0x10`, unrounded) bytes at `at`, or without one at the
/// worker's binary (`+4`) plus the header's offset `+0x2c` when that is
/// set. `EAX` 0 after clearing, else the worker.
pub fn atel_set_my_work_adrs(m: &mut impl Mem, i: u32, at: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    let hdr = m.read::<u32>(w);
    m.write(w + 0xb8, at);
    let (mut dst, mut n) = (0_u32, 0_i32);
    let off = m.read::<u32>(hdr + 0x2c);
    if off != 0 {
        dst = m.read::<u32>(w + 4).wrapping_add(off);
        n = m.read::<i32>(hdr + 0x10);
    }
    if at != 0 {
        n = m.read::<i32>(hdr + 0x10);
        dst = at;
    }
    if dst == 0 || n <= 0 {
        return w;
    }
    for k in 0..n.cast_unsigned() {
        m.write(dst.wrapping_add(k), 0_u8);
    }
    0
}

/// `MsGetSaveChrArmorModel(chr)` (`0x007abb70`, battle).
const MS_GET_SAVE_CHR_ARMOR_MODEL: u32 = 0x007a_bb70;
/// `MsGetSaveChrWeaponModel(chr)` (`0x007abba0`, battle).
const MS_GET_SAVE_CHR_WEAPON_MODEL: u32 = 0x007a_bba0;

/// `AtelGetElemId(chr, kind)` (`0x0086b150`): the model of the save
/// character's armor for kind 1, else of its weapon (through the game).
pub fn atel_get_elem_id(h: &mut impl ffx_bridge::Host, chr: u32, kind: u32) -> u32 {
    let f = if kind == 1 {
        MS_GET_SAVE_CHR_ARMOR_MODEL
    } else {
        MS_GET_SAVE_CHR_WEAPON_MODEL
    };
    host_call(h, f, &[chr])
}

/// The save event record (`TkGetSaveEventAddress`, `0x0088c830`, a jump to
/// `MsGetSaveEventAddress`).
fn save_event(h: &mut impl ffx_bridge::Host) -> u32 {
    host_call(h, MS_GET_SAVE_EVENT_ADDRESS, &[])
}

/// The low half of an argument the original stores as `cx`.
fn low16(v: u32) -> u16 {
    let [lo, hi, _, _] = v.to_le_bytes();
    u16::from_le_bytes([lo, hi])
}

/// `AtelSetWaterBattleEffect(on)` (`0x008715f0`): the save event record's
/// `+0xd7` = on != 0; `EAX` the record.
pub fn atel_set_water_battle_effect(h: &mut impl ffx_bridge::Host, on: u32) -> u32 {
    let e = save_event(h);
    h.write(e + 0xd7, u8::from(on != 0));
    e
}

/// `Atel_SetSaveEventD1(on)` (`0x0086f060`, behind `0x0194`): the save
/// event record's `+0xd1` = on != 0; `EAX` the record.
pub fn atel_set_save_event_d1(h: &mut impl ffx_bridge::Host, on: u32) -> u32 {
    let e = save_event(h);
    h.write(e + 0xd1, u8::from(on != 0));
    e
}

/// `Atel_SetSaveEvent13Bit1(on)` (`0x0086f430`, behind `0x00e2`): bit 1 of
/// the save event record's `+0x13` = on != 0; `EAX` the record.
pub fn atel_set_save_event_13_bit1(h: &mut impl ffx_bridge::Host, on: u32) -> u32 {
    let e = save_event(h);
    let b = h.read::<u8>(e + 0x13);
    h.write(e + 0x13, b & !2 | u8::from(on != 0) << 1);
    e
}

/// `AtelSetForcePlaceId(id)` (`0x00870140`): the save event record's
/// `+0xd4` (`u16`) = id, `+0xd6` = 1 (the forced place set); `EAX` the
/// record.
pub fn atel_set_force_place_id(h: &mut impl ffx_bridge::Host, id: u32) -> u32 {
    let e = save_event(h);
    h.write(e + 0xd4, low16(id));
    h.write(e + 0xd6, 1_u8);
    e
}

/// The save event record setters of one `u16` word: `0x00870ed0` (`+0xe`,
/// the dictionary index `AtelSetSaveDicIndex` computes; behind `0x00ac`),
/// `0x00873dd0` (`+0xe8`, behind `0x01c4`/`0x0227`) and `0x00870fa0`
/// (`+0xea`, behind `0x020d`). `EAX` the record.
pub fn atel_set_save_event_word(h: &mut impl ffx_bridge::Host, off: u32, v: u32) -> u32 {
    let e = save_event(h);
    h.write(e + off, low16(v));
    e
}

/// `Atel_SetSaveEventE4(v)` (`0x00874030`, behind `0x01c3`): the save event
/// record's `+0xe4` = v; `EAX` the record.
pub fn atel_set_save_event_e4(h: &mut impl ffx_bridge::Host, v: u32) -> u32 {
    let e = save_event(h);
    h.write(e + 0xe4, v);
    e
}

/// `AtelSetHelpEventFlag(i, on)` (`0x008701f0`): sets (on non-zero) or
/// clears bit i of `MsGetSaveEventFlag()`'s bytes (`0x00785250`), as
/// [`atel_get_help_event_flag`] reads it (negative i is bit 0, the byte at
/// most `0x7f`); `EAX` the flags.
pub fn atel_set_help_event_flag(h: &mut impl ffx_bridge::Host, i: u32, on: u32) -> u32 {
    let f = host_call(h, 0x0078_5250, &[]);
    let i = i.cast_signed().max(0).cast_unsigned();
    let at = f.wrapping_add((i >> 3).min(0x7f));
    let bit = 1_u8 << (i & 7);
    let b = h.read::<u8>(at);
    h.write(at, if on != 0 { b | bit } else { b & !bit });
    f
}

/// `NkzVersion()` (`0x00a5cf00`): the build date string `0x00b7b7dc`.
const NKZ_VERSION_STRING: u32 = 0x00b7_b7dc;
/// `g_AtelSepLoadCount` (`0x01325b78`).
const ATEL_SEP_LOAD_COUNT: u32 = 0x0132_5b78;
/// `g_AtelSepGroupMask` (`0x01326bdc`, a byte).
const ATEL_SEP_GROUP_MASK: u32 = 0x0132_6bdc;

/// `AtelInitSaveWork()` (`0x0086d4e0`): the save event record's script
/// variables (`+0x1ec`, 0x1000 bytes) cleared, `g_AtelSepLoadCount` 0 and
/// `g_AtelSepGroupMask` 0; a tail jump to `NkzVersion`, so `EAX` its
/// string.
pub fn atel_init_save_work(h: &mut impl ffx_bridge::Host) -> u32 {
    let e = save_event(h);
    h.write_bytes(e + 0x1ec, &[0; 0x1000]);
    h.write(h.rebase(ATEL_SEP_LOAD_COUNT), 0_u32);
    h.write(h.rebase(ATEL_SEP_GROUP_MASK), 0_u8);
    h.rebase(NKZ_VERSION_STRING)
}

/// `TkSetEventJumpMapNo(map, entrance)` (`0x0088eaf0`, menu).
const TK_SET_EVENT_JUMP_MAP_NO: u32 = 0x0088_eaf0;

/// `AtelMapLoadBtl(map)` (`0x0086dcd0`, behind `0xc031`): as
/// [`atel_map_load`] with `yiBattleMapRead(map, 0)` (`0x00908370`).
pub fn atel_map_load_btl(h: &mut impl ffx_bridge::Host, map: u32) -> u32 {
    let r = host_call(h, TK_SET_EVENT_JUMP_MAP_NO, &[map, 0]);
    if map.cast_signed() > 0 {
        return host_call(h, 0x0090_8370, &[map, 0]);
    }
    r
}

/// `Atel_JumpToEntrance(e)` (`0x00874000`, behind `0x8006`):
/// `g_AtelMapIdPage` = 1, its `+1` = e's low byte and `+2` = `0xff`, then
/// `TkSetEventJumpMapNo(TkGetNowEventJumpMapNo(), e)` (`0x0088d690`): a
/// jump to entrance e of the current map. `EAX` the last call's.
pub fn atel_jump_to_entrance(h: &mut impl ffx_bridge::Host, e: u32) -> u32 {
    let p = h.rebase(0x0132_711c);
    h.write(p, 1_u8);
    h.write(p + 1, e.to_le_bytes()[0]);
    h.write(p + 2, 0xff_u8);
    let map = host_call(h, 0x0088_d690, &[]);
    host_call(h, TK_SET_EVENT_JUMP_MAP_NO, &[map, e])
}

/// `AtelSetDolby(on)` (`0x0086f9e0`, behind `0x023a`): bit 7 of the save
/// menu word (`MsGetSaveMenu`, `0x00785290`) = on, then
/// `SndSetSoundMode` (`0x00887180`) 0 for Dolby, else 1 for bit 0 (mono)
/// or 2, and `TkSetControlerActuatorMode(0, 0, bit 4)` (`0x008894d0`, the
/// vibration). `EAX` the last call's.
pub fn atel_set_dolby(h: &mut impl ffx_bridge::Host, on: u32) -> u32 {
    let p = host_call(h, 0x0078_5290, &[]);
    let v = h.read::<u32>(p);
    h.write(p, if on != 0 { v | 0x80 } else { v & !0x80 });
    let p = host_call(h, 0x0078_5290, &[]);
    let flags = h.read::<u8>(p);
    let mode = if flags & 0x80 != 0 {
        0
    } else if flags & 1 != 0 {
        1
    } else {
        2
    };
    host_call(h, 0x0088_7180, &[mode]);
    let p = host_call(h, 0x0078_5290, &[]);
    let vib = u32::from(h.read::<u8>(p) & 0x10) >> 4;
    host_call(h, 0x0088_94d0, &[0, 0, vib])
}

// ---- script round 23 f (this machine): worker positions, signals, effect polls ----

/// An `f32` through `fld`/`fstp` (signalling NaNs come out quiet).
fn through(m: &impl Mem, at: u32) -> u32 {
    x87::store(x87::load(m.read(at)))
}

/// The two-point workers (types 2 and 5) whose second point sits at the
/// block's `+0x30`.
fn two_point(t: u8) -> bool {
    t == 2 || t == 5
}

/// `AtelGetActorStartPos(i, x, y, z)` (`0x0086a720`): the position block's
/// first point (`+0`, `+4`, `+8`) of the current ctrl work's worker i into
/// the three out words. Two-point workers read y and z before storing x.
/// `EAX` the z pointer.
pub fn atel_get_actor_start_pos(m: &mut impl Mem, i: u32, x: u32, y: u32, z: u32) -> u32 {
    actor_pos(m, i, [x, y, z], 0)
}

/// `AtelGetActorEndPos(i, x, y, z)` (`0x0086a390`): as
/// [`atel_get_actor_start_pos`], the second point (`+0x30`) for two-point
/// workers.
pub fn atel_get_actor_end_pos(m: &mut impl Mem, i: u32, x: u32, y: u32, z: u32) -> u32 {
    actor_pos(m, i, [x, y, z], 0x30)
}

fn actor_pos(m: &mut impl Mem, i: u32, out: [u32; 3], second: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    let t = kind(m, w);
    let p = pos_block(m, w);
    if two_point(t) {
        let p = p.wrapping_add(second);
        let (vy, vz) = (through(m, p + 4), through(m, p + 8));
        let vx = through(m, p);
        m.write(out[0], vx);
        m.write(out[1], vy);
        m.write(out[2], vz);
    } else {
        for (k, o) in (0..3).zip(out) {
            let v = through(m, p.wrapping_add(k * 4));
            m.write(o, v);
        }
    }
    out[2]
}

/// `Script_GetWorkerPos(i, x, y, z)` (`0x0086a2f0`): as
/// [`atel_get_actor_start_pos`], but two-point workers give the midpoint
/// of their two points (`(b + a) * 0.5` per axis on the x87, y and z first).
pub fn script_get_worker_pos(m: &mut impl Mem, i: u32, x: u32, y: u32, z: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    let t = kind(m, w);
    let p = pos_block(m, w);
    if !two_point(t) {
        return actor_pos(m, i, [x, y, z], 0);
    }
    let (vy, vz) = (midpoint(m, p, 4), midpoint(m, p, 8));
    let vx = midpoint(m, p, 0);
    m.write(x, vx);
    m.write(y, vy);
    m.write(z, vz);
    z
}

/// `(p[0x30 + k] + p[k]) * 0.5` on the x87, stored as an `f32`.
fn midpoint(m: &impl Mem, p: u32, k: u32) -> u32 {
    let s = x87::add(x87::load(m.read(p + 0x30 + k)), x87::load(m.read(p + k)));
    x87::store(x87::mul(s, 0.5))
}

/// `Event_SetPlayer(id)` (`0x0086f6a0`): the current ctrl work's player
/// (`+0xa`, `u16`) = id; for an id other than `0xffff`, worker id's
/// position (the block as [`atel_get_actor_start_pos`] picks it) into
/// `+0x218` and `+0x228` (x, y, z each). `EAX` the block, or the id.
pub fn event_set_player(m: &mut impl Mem, id: u32) -> u32 {
    let ctx = m.read::<u32>(m.rebase(SCRIPT_CONTEXT));
    m.write(ctx + 0xa, low16(id));
    if id == 0xffff {
        return id;
    }
    let w = super::atel_get_basic_work(m, id.cast_signed());
    let p = pos_block(m, w);
    let v = [through(m, p), through(m, p + 4), through(m, p + 8)];
    for (k, at) in [
        (0, 0x228),
        (1, 0x22c),
        (2, 0x230),
        (2, 0x220),
        (0, 0x218),
        (1, 0x21c),
    ] {
        m.write(ctx + at, v[k]);
    }
    p
}

/// `AtelIsBind(i)` (`0x0086d950`): 1 when the current ctrl work's worker i
/// is bound to a character (`+0xaa` 1) with a model (`+0x9c`), else 0.
pub fn atel_is_bind(m: &mut impl Mem, i: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    u32::from(m.read::<u8>(w + 0xaa) == 1 && m.read::<u32>(w + 0x9c) != 0)
}

/// `Atel_HasSignalFromLevel(i, level)` (`0x00863520`, the `0x00f6`/`0x00f7`
/// polls): 1 when worker i's signal queue (`+0x80`, linked through `+0`)
/// holds an entry other than kind 3 (`+0xf`) whose level (`+0xe`, low 4
/// bits) is at least `level` (signed), else 0.
pub fn atel_has_signal_from_level(m: &mut impl Mem, i: u32, level: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    let mut e = m.read::<u32>(w + 0x80);
    while e != 0 {
        let next = m.read::<u32>(e);
        if m.read::<u8>(e + 0xf) != 3
            && i32::from(m.read::<u8>(e + 0xe) & 0xf) >= level.cast_signed()
        {
            return 1;
        }
        e = next;
    }
    0
}

/// `MsEventEffectGetStat()` (`0x00788bd0`, battle): the event effect's state.
const MS_EVENT_EFFECT_GET_STAT: u32 = 0x0078_8bd0;

/// `Atel_EffectStartPoll(arg)` (`0x008633e0`, the `0x00dc` poll; a jump
/// table on the effect state): state 3 starts the effect
/// (`MsEventEffectStart(arg)`, `0x00788d60`), state 5 clears
/// `g_AtelEffectState` (`0x01326b58`) and is done (5); else 0.
pub fn atel_effect_start_poll(h: &mut impl ffx_bridge::Host, arg: u32) -> u32 {
    match host_call(h, MS_EVENT_EFFECT_GET_STAT, &[]) {
        3 => {
            host_call(h, 0x0078_8d60, &[arg]);
            0
        }
        5 => {
            h.write(h.rebase(0x0132_6b58), 0_u32);
            5
        }
        _ => 0,
    }
}

/// `Atel_EffectIdlePoll()` (`0x008633a0`): 5 (done) while the event effect
/// is idle (state 0), else 0.
pub fn atel_effect_idle_poll(h: &mut impl ffx_bridge::Host) -> u32 {
    if host_call(h, MS_EVENT_EFFECT_GET_STAT, &[]) == 0 {
        5
    } else {
        0
    }
}

/// `Atel_EffectEndPoll()` (`0x00863350`, the `0x00de` poll): for the effect
/// states 0 and 6, `g_AtelOverlayStatus` (`0x01326b80`) = 4 and 5 (done);
/// else 0.
pub fn atel_effect_end_poll(h: &mut impl ffx_bridge::Host) -> u32 {
    if matches!(host_call(h, MS_EVENT_EFFECT_GET_STAT, &[]), 0 | 6) {
        h.write(h.rebase(0x0132_6b80), 4_u8);
        5
    } else {
        0
    }
}

/// `Atel_SetLevelVectorsXZ(worker, a, c)` (`0x0086f750`, behind `0x0014`):
/// [`atel_set_level_vectors`] with a, 0 and c (each through `fld`/`fstp`).
pub fn atel_set_level_vectors_xz(m: &mut impl Mem, w: u32, a: u32, c: u32) -> u32 {
    let q = |v: u32| x87::store(x87::load(v));
    atel_set_level_vectors(m, w, q(a), 0, q(c))
}

/// `Atel_GetWorkerBits10_12(i)` (`0x0086a440`): bits 10..12 of the current
/// ctrl work's worker i's `+0x34`, at most 5.
pub fn atel_get_worker_bits10_12(m: &mut impl Mem, i: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    (u32::from(m.read::<u16>(w + 0x34)) >> 10 & 7).min(5)
}

/// `AtelRecoverPreBgPage()` (`0x00873cc0`): `yiSetPreBgPage` (`0x0090a6f0`)
/// with `g_AtelMapIdPage`'s `+1` and `yiSetIDPage` (`0x0090a4a0`) with its
/// `+2`, each when not negative.
pub fn atel_recover_pre_bg_page(h: &mut impl ffx_bridge::Host) {
    let p = h.rebase(0x0132_711c);
    for (k, f) in [(1, 0x0090_a6f0), (2, 0x0090_a4a0)] {
        let v = h.read::<i8>(p + k);
        if v >= 0 {
            host_call(h, f, &[i32::from(v).cast_unsigned()]);
        }
    }
}

/// `AtelExecReturn2(i, level, reset)` (`0x00869350`, the battle's request
/// cancel/run): `AtelExecReturn(i, level, reset)` (a tail jump, the
/// caller's three words) when worker i has a request level (`+0x32`).
pub fn atel_exec_return2(h: &mut impl ffx_bridge::Host, i: u32, level: u32, reset: u32) {
    let w = super::atel_get_basic_work(h, i.cast_signed());
    if h.read::<u8>(w + 0x32) != 0 {
        super::signal::atel_exec_return(h, i, level, reset);
    }
}

// ---- script round 23 g (this machine): event set-up helpers ----

/// `AtelInitSourceCodeDebugWork(i, src)` (`0x0086d670`): with a source
/// record, worker i's `+0xc + 4k` = the record's `+4 + 12k` for k from
/// `n = [src] - 1` (clamped to 0..=7) down to 0.
pub fn atel_init_source_code_debug_work(m: &mut impl Mem, i: u32, src: u32) {
    if src == 0 {
        return;
    }
    let w = super::atel_get_basic_work(m, i.cast_signed());
    let n = m.read::<u32>(src).wrapping_sub(1).cast_signed().clamp(0, 7);
    for k in (0..=n.cast_unsigned()).rev() {
        let v = m.read::<u32>(src + 4 + k * 12);
        m.write(w + 0xc + k * 4, v);
    }
}

/// Ctrl work 0's guide map flag (bit 7 of `+1`).
const GUIDE_MAP_FLAG: u32 = CTRL_WORKS + 1;

/// `AtelEventRemoveGuideMap()` (`0x008661a0`): clears ctrl work 0's guide
/// map flag (`Yn_GuideMapEnd`, `0x00919c60`, is an empty function).
pub fn atel_event_remove_guide_map(m: &mut impl Mem) {
    with_event_ctrl(m, |m| {
        let at = m.rebase(GUIDE_MAP_FLAG);
        let f = m.read::<u8>(at);
        if f & 0x80 != 0 {
            m.write(at, f & 0x7f);
        }
    });
}

/// `AtelEventSetupGuideMap()` (`0x00866490`): sets ctrl work 0's guide map
/// flag (`Yn_GuideMapInit`, `0x00919cd0`, is empty), then
/// `Yn_GuideMapSetMode(0)` (`0x00919e20`) with the context back; its `EAX`.
pub fn atel_event_setup_guide_map(h: &mut impl ffx_bridge::Host) -> u32 {
    with_event_ctrl(h, |m| {
        let at = m.rebase(GUIDE_MAP_FLAG);
        let f = m.read::<u8>(at);
        if f & 0x80 == 0 {
            m.write(at, f | 0x80);
        }
    });
    host_call(h, 0x0091_9e20, &[0])
}

/// The event header's sound table (its `+0x2c` offset from the header),
/// none for a zero offset or one that wraps to address 0.
fn header_sound(m: &impl Mem, hdr: u32) -> Option<u32> {
    let t = m.read::<u32>(hdr + 0x2c);
    let at = t.wrapping_add(hdr);
    (t != 0 && at != 0).then_some(at)
}

/// `AtelEventSetUpFoot3d(hdr)` (`0x00866210`): `SndEventSetFoot3dCtrlp`
/// (`0x00885fc0`, a tail jump) with the sound table's foot table (its
/// `+0x2c` offset from the header), or 0; its `EAX`.
pub fn atel_event_set_up_foot_3d(h: &mut impl ffx_bridge::Host, hdr: u32) -> u32 {
    let p = header_sound(h, hdr).map_or(0, |t| {
        let off = h.read::<u32>(t + 0x2c);
        if off == 0 { 0 } else { hdr.wrapping_add(off) }
    });
    host_call(h, 0x0088_5fc0, &[p])
}

/// `AtelEventLoadMusic(hdr)` (`0x00865a60`): when the sound table's music
/// word (`+0x38`) is negative, `SndMusicRead(its low half)` (`0x00886960`)
/// and `SndSyncMusicStandby(1)` (`0x00887900`, a tail jump; the debug
/// strings around them go to the empty `AtelPs2DebugString`). `EAX` the
/// last call's, else the header.
pub fn atel_event_load_music(h: &mut impl ffx_bridge::Host, hdr: u32) -> u32 {
    let music = header_sound(h, hdr).map_or(0, |t| h.read::<i32>(t + 0x38));
    if music >= 0 {
        return hdr;
    }
    host_call(h, 0x0088_6960, &[music.cast_unsigned() & 0xffff]);
    host_call(h, 0x0088_7900, &[1])
}

/// `AtelInitCalcSound()` (`0x00877420`): `AtelClearSnd3dWorkAll`, then
/// the 16 bytes at `0x00c0a004` (`g_Vu0_Vf00`'s) to `0x01328278`; `EAX`
/// the last word.
pub fn atel_init_calc_sound(m: &mut impl Mem) -> u32 {
    super::globals::atel_clear_snd3d_work_all(m);
    let (from, to) = (m.rebase(0x00c0_a004), m.rebase(0x0132_8278));
    let mut v = 0;
    for k in 0..4 {
        v = m.read::<u32>(from + k * 4);
        m.write(to + k * 4, v);
    }
    v
}

/// `AtelGetGuideMapAngEx(i)` (`0x0086b9a0`, `ST0`): with ctrl work 0 in use
/// (bit 0) and its event block (`+0x48`), `Atel_GetBinHdrFloat(block, i)`
/// rounded to an `f32`, else 0.
pub fn atel_get_guide_map_ang_ex(m: &impl Mem, i: u32) -> f32 {
    let c = m.rebase(CTRL_WORKS);
    let blk = m.read::<u32>(c + 0x48);
    if m.read::<u8>(c) & 1 == 0 || blk == 0 {
        return 0.0;
    }
    f32::from_bits(x87::store(atel_get_bin_hdr_float(m, blk, i)))
}

/// `AtelInitInternalActorWork(i)` (`0x0086d260`): worker i's `+0x48` to
/// `+0x7f` zeroed (words, then `fldz` stores); `EAX` the worker.
pub fn atel_init_internal_actor_work(m: &mut impl Mem, i: u32) -> u32 {
    let w = super::atel_get_basic_work(m, i.cast_signed());
    m.write_bytes(w + 0x48, &[0; 0x38]);
    w
}

/// `AtelSetOnMemoryMovieFileWait(movie, flags)` (`0x00873f70`):
/// `graphicSetTextureVideoInfo(movie, flags & 0x80)` (`0x00643aa0`), then
/// the save event record's `+0xe0` = 1, `+0xd8` = movie, `+0xdc` = flags;
/// `EAX` the record.
pub fn atel_set_on_memory_movie_file_wait(
    h: &mut impl ffx_bridge::Host,
    movie: u32,
    flags: u32,
) -> u32 {
    host_call(h, 0x0064_3aa0, &[movie, flags & 0x80]);
    let e = save_event(h);
    h.write(e + 0xe0, 1_u32);
    h.write(e + 0xd8, movie);
    h.write(e + 0xdc, flags);
    e
}

/// `AtelRecoverFromMenuCallback()` (`0x0086e700`): back from the menu:
/// `g_AtelMenuExecStatus` 0, `Yn_GuideMapSetDispProg(1)` (`0x00919d70`),
/// the save event record's forced place (`+0xd6`) off,
/// `TkSetVibrateMask(k, 1)` (`0x00875f60`) for k 1..=7 and
/// `g_AtelOverlayStatus` 6. `EAX` the last call's.
pub fn atel_recover_from_menu_callback(h: &mut impl ffx_bridge::Host) -> u32 {
    h.write(h.rebase(MENU_EXEC_STATUS), 0_u8);
    host_call(h, 0x0091_9d70, &[1]);
    let e = save_event(h);
    h.write(e + 0xd6, 0_u8);
    let mut r = 0;
    for k in 1..=7 {
        r = host_call(h, 0x0087_5f60, &[k, 1]);
    }
    h.write(h.rebase(0x0132_6b80), 6_u8);
    r
}

// ---- script round 23 h (this machine): the FP (effect point) bindings ----

/// The four FP slots of a worker (`+0xb18`, 0x10 bytes each): `+0` the
/// FP id (low 15 bits) with bit 15 for a set position, `+2` the bone or
/// mode, `+4..+0xc` the position.
const FP_SLOTS: u32 = 0xb18;
/// `pppFpSetStickChr(fp, model, bone)` (`0x0072a530`, vfx).
const PPP_FP_SET_STICK_CHR: u32 = 0x0072_a530;
/// `pppFpSetStickMap(fp, group)` (`0x0072a590`, vfx).
const PPP_FP_SET_STICK_MAP: u32 = 0x0072_a590;
/// `pppFpSetStickMapDObj(fp, dobj)` (`0x0072a5e0`, vfx).
const PPP_FP_SET_STICK_MAP_DOBJ: u32 = 0x0072_a5e0;
/// `pppFpResetStick(fp)` (`0x0072a120`, vfx).
const PPP_FP_RESET_STICK: u32 = 0x0072_a120;
/// `pppFpSetStickPos(fp, &vec4)` (`0x0072a630`, vfx).
const PPP_FP_SET_STICK_POS: u32 = 0x0072_a630;

/// Whether worker w is of type 1 (the only one with FP slots).
fn fp_worker(m: &impl Mem, w: u32) -> bool {
    kind(m, w) == 1
}

/// `AtelBindFp(worker, fp, bone)` (`0x00862800`, behind `0x8009` and
/// `0x0196`): the first free FP slot (id 0) gets the fp id or-ed in and
/// the bone, then the FP is stuck to what the worker is bound to:
/// its character (`+0xaa` 1, with a model) by `pppFpSetStickChr`, its map
/// group (4) by `pppFpSetStickMap`, its map object (5) by
/// `pppFpSetStickMapDObj`. Nothing without a free slot.
#[allow(clippy::verbose_bit_mask)] // the id bits, as the original tests them
pub fn atel_bind_fp(h: &mut impl ffx_bridge::Host, w: u32, fp: u32, bone: u32) {
    if !fp_worker(h, w) {
        return;
    }
    let Some(slot) = (0..4)
        .map(|k| w + FP_SLOTS + k * 0x10)
        .find(|&s| h.read::<u16>(s) & 0x7fff == 0)
    else {
        return;
    };
    let id = h.read::<u16>(slot);
    h.write(slot, id | low16(fp));
    h.write(slot + 2, low16(bone));
    match h.read::<u8>(w + 0xaa) {
        1 => {
            let model = h.read::<u32>(w + 0x9c);
            if model != 0 {
                host_call(h, PPP_FP_SET_STICK_CHR, &[fp, model, bone]);
            }
        }
        4 => {
            host_call(h, PPP_FP_SET_STICK_MAP, &[fp, bone]);
        }
        5 => {
            host_call(h, PPP_FP_SET_STICK_MAP_DOBJ, &[fp, bone]);
        }
        _ => {}
    }
}

/// `AtelBindFpOff(worker, fp)` (`0x008628a0`, behind `0x800b`): fp 0 frees
/// every slot in use with `pppFpResetStick(0)` each; another fp frees the
/// slots holding it (bit 15 left out) with `pppFpResetStick(fp)`.
pub fn atel_bind_fp_off(h: &mut impl ffx_bridge::Host, w: u32, fp: u32) {
    if !fp_worker(h, w) {
        return;
    }
    for k in 0..4 {
        let slot = w + FP_SLOTS + k * 0x10;
        let id = u32::from(h.read::<u16>(slot) & 0x7fff);
        let hit = if fp == 0 { id != 0 } else { id == fp };
        if hit {
            h.write(slot, 0_u16);
            host_call(h, PPP_FP_RESET_STICK, &[fp]);
        }
    }
}

/// `pppFpSetStickPos(fp, (x, y, z, 1.0))` with the vector in `frame` (the
/// original's local, 0x10 bytes).
fn fp_set_stick_pos(h: &mut impl ffx_bridge::Host, fp: u32, pos: [u32; 3], frame: u32) {
    for (k, v) in (0..).zip(pos) {
        h.write(frame + k * 4, v);
    }
    h.write(frame + 12, 1f32.to_bits());
    host_call(h, PPP_FP_SET_STICK_POS, &[fp, frame]);
}

/// `AtelBindFpPos(worker, fp, x, y, z)` (`0x00862920`, behind `0x800a`):
/// every slot holding fp gets the position (`+4..+0xc`) and bit 15, and
/// `pppFpSetStickPos(fp, (x, y, z, 1))` with the vector in `frame`.
pub fn atel_bind_fp_pos(h: &mut impl ffx_bridge::Host, w: u32, fp: u32, pos: [u32; 3], frame: u32) {
    if !fp_worker(h, w) {
        return;
    }
    let pos = pos.map(|v| x87::store(x87::load(v)));
    for k in 0..4 {
        let slot = w + FP_SLOTS + k * 0x10;
        let id = h.read::<u16>(slot);
        if u32::from(id & 0x7fff) != fp {
            continue;
        }
        for (j, v) in (0..).zip(pos) {
            h.write(slot + 4 + j * 4, v);
        }
        h.write(slot, id | 0x8000);
        fp_set_stick_pos(h, fp, pos, frame);
    }
}

/// `AtelRecoverFPFromBattle(worker)` (`0x0086e4c0`): back from a battle,
/// every slot in use is stuck again as [`atel_bind_fp`] does (fp, bone),
/// and its position set again when bit 15 says it has one. A binding other
/// than 1, 4 and 5 ends it at the first slot in use.
pub fn atel_recover_fp_from_battle(h: &mut impl ffx_bridge::Host, w: u32, frame: u32) {
    if !fp_worker(h, w) {
        return;
    }
    for k in 0..4 {
        let slot = w + FP_SLOTS + k * 0x10;
        let fp = u32::from(h.read::<u16>(slot) & 0x7fff);
        if fp == 0 {
            continue;
        }
        let bone = u32::from(h.read::<u16>(slot + 2));
        match h.read::<u8>(w + 0xaa) {
            1 => {
                let model = h.read::<u32>(w + 0x9c);
                if model != 0 {
                    host_call(h, PPP_FP_SET_STICK_CHR, &[fp, model, bone]);
                }
            }
            4 => {
                host_call(h, PPP_FP_SET_STICK_MAP, &[fp, bone]);
            }
            5 => {
                host_call(h, PPP_FP_SET_STICK_MAP_DOBJ, &[fp, bone]);
            }
            _ => return,
        }
        if h.read::<u16>(slot) & 0x8000 != 0 {
            let pos = [4, 8, 12].map(|o| through(h, slot + o));
            fp_set_stick_pos(h, fp, pos, frame);
        }
    }
}

// ---- script round 23 i (this machine): radius tests, map index, music, map camera ----

/// `Math_InRadius(r, dx, dy, dz)` (`0x0088b730`, menu).
const MATH_IN_RADIUS: u32 = 0x0088_b730;

/// `a - b` of two `f32` words on the x87, stored as an `f32`.
fn sub_f32(a: u32, b: u32) -> u32 {
    x87::store(x87::sub(x87::load(a), x87::load(b)))
}

/// `Atel_InRadiusXZ(r, worker, x, y, z)` (`0x008634b0`): whether (x, z) is
/// within r of the worker's position block (as [`atel_get_actor_start_pos`]
/// picks it): `Math_InRadius(r, x - px, 0, z - pz)`; y is not used. Its
/// `EAX`.
pub fn atel_in_radius_xz(h: &mut impl ffx_bridge::Host, r: u32, w: u32, x: u32, z: u32) -> u32 {
    let p = pos_block(h, w);
    let dx = sub_f32(x, h.read(p));
    let dz = sub_f32(z, h.read(p + 8));
    let r = x87::store(x87::load(r));
    host_call(h, MATH_IN_RADIUS, &[r, dx, 0, dz])
}

/// `Atel_InRadiusXYZ(r, worker, x, y, z)` (`0x00863440`): as
/// [`atel_in_radius_xz`] with y as well.
pub fn atel_in_radius_xyz(h: &mut impl ffx_bridge::Host, r: u32, w: u32, pos: [u32; 3]) -> u32 {
    let p = pos_block(h, w);
    let d = [0, 4, 8].map(|k| h.read::<u32>(p + k));
    let [dx, dy, dz] = [0, 1, 2].map(|k| sub_f32(pos[k], d[k]));
    let r = x87::store(x87::load(r));
    host_call(h, MATH_IN_RADIUS, &[r, dx, dy, dz])
}

/// `AtelGetMapIndex(hdr, i)` (`0x0086bd90`): -1 without the header's
/// entrance table (`+4`); else, when the sound table (`+0x2c`) has a map
/// record (`+0x3c`) with a language table (`+0xc`), that table's word for
/// the language (1 with the English config, else `TkGetLang()` when in
/// 0..16, else 0); else entrance i's map (`i16` at the table's `+32i`).
/// The `dbgPrintf` calls are empty on PC.
pub fn atel_get_map_index(h: &mut impl ffx_bridge::Host, hdr: u32, i: u32) -> u32 {
    let tab = h.read::<u32>(hdr + 4);
    if tab == 0 {
        return u32::MAX;
    }
    let map = i32::from(h.read::<i16>(i.wrapping_mul(32).wrapping_add(tab).wrapping_add(hdr)));
    let rec = header_sound(h, hdr).and_then(|t| {
        let r = h.read::<u32>(t + 0x3c);
        let at = r.wrapping_add(hdr);
        (r != 0 && at != 0 && h.read::<u32>(at + 0xc) != 0).then_some(at)
    });
    let Some(rec) = rec else {
        return map.cast_unsigned();
    };
    let lang = host_call(h, 0x0088_7c30, &[]).cast_signed();
    let k = if host_call(h, 0x0078_51d0, &[]) != 0 {
        1
    } else if lang >= 0x10 {
        0
    } else {
        lang.max(0).cast_unsigned()
    };
    let t = h.read::<u32>(rec + 0xc);
    h.read::<u32>(t.wrapping_add(k * 4).wrapping_add(hdr))
}

/// `SndGetNowPlayMusic()` (`0x008863f0`).
const SND_GET_NOW_PLAY_MUSIC: u32 = 0x0088_63f0;

/// `AtelRegistMusicNumber(n)` (`0x00873d00`), with the `EAX` it leaves (n).
fn regist_music(m: &mut impl Mem, n: u32) -> u32 {
    super::globals::atel_regist_music_number(m, n);
    n
}

/// `AtelEventPlayMusic(hdr)` (`0x00865ac0`): for a negative music word in
/// the sound table (`+0x38`: the number in the low half, the fade in bits
/// 16..30): a number from `0x100` up fades the playing music out
/// (`SndMusicVolumeFadeOut(now, fade)`) and registers -1; else the number
/// starts (`SndMusicStart`) when nothing plays, cross-fades in
/// (`SndMusicCrossStart(n, fade)`) when other music plays, and is
/// registered (`AtelRegistMusicNumber`); the same music goes on. `EAX`
/// the last call's, or the music word.
pub fn atel_event_play_music(h: &mut impl ffx_bridge::Host, hdr: u32) -> u32 {
    let music = header_sound(h, hdr).map_or(0, |t| h.read::<u32>(t + 0x38));
    if music.cast_signed() >= 0 {
        return music;
    }
    let fade = music >> 16 & 0x7fff;
    let n = music & 0xffff;
    let now = host_call(h, SND_GET_NOW_PLAY_MUSIC, &[]);
    if n >= 0x100 {
        if now.cast_signed() >= 0 {
            let now = host_call(h, SND_GET_NOW_PLAY_MUSIC, &[]);
            host_call(h, 0x0088_6af0, &[now, fade]);
        }
        return regist_music(h, u32::MAX);
    }
    if now.cast_signed() >= 0 {
        let now = host_call(h, SND_GET_NOW_PLAY_MUSIC, &[]);
        if now == n {
            return now;
        }
        if host_call(h, SND_GET_NOW_PLAY_MUSIC, &[]).cast_signed() >= 0 {
            host_call(h, 0x0088_6830, &[n, fade]);
            return regist_music(h, n);
        }
    }
    host_call(h, 0x0088_6a20, &[n]);
    regist_music(h, n)
}

/// The entrance record `AtelRequestMapCamera` falls back on without a
/// table (`0x01325b50`).
const DEFAULT_ENTRANCE: u32 = 0x0132_5b50;

/// `AtelRequestMapCamera(set)` (`0x0086ed80`): with a camera worker
/// (`+0x18`, `u16`, not `0xffff`), the current entrance's record
/// (`TkGetNowEventJumpIndex`, 32 bytes each in the table at `+4`, the
/// first one past the end `+8`; [`DEFAULT_ENTRANCE`] without a table)
/// names a camera (`+6`, `i16`); a non-zero one is requested from the
/// worker as signal (`0xffff`, worker, 0, 1, camera) (`AtelRegistSignalMain`),
/// and 1. Else 0.
pub fn atel_request_map_camera(h: &mut impl ffx_bridge::Host, set: u32) -> u32 {
    let to = u32::from(h.read::<u16>(set + 0x18));
    if to == 0xffff {
        return 0;
    }
    let end = h.read::<i32>(set + 8);
    let tab = h.read::<u32>(set + 4);
    let j = host_call(h, 0x0088_d670, &[]);
    let rec = if tab == 0 {
        h.rebase(DEFAULT_ENTRANCE)
    } else {
        let at = j.wrapping_shl(5).wrapping_add(tab);
        let at = if at.cast_signed() < end { at } else { tab };
        at.wrapping_add(set)
    };
    let cam = h.read::<i16>(rec + 6);
    if cam == 0 {
        return 0;
    }
    let r = super::signal::Request::new(0xffff, to, 0, 1, i32::from(cam).cast_unsigned());
    super::signal::atel_regist_signal_main(h, r);
    1
}

// ---- round 24 ----

/// The bytes of the C string at `p` (without its NUL).
fn c_string(m: &impl Mem, p: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = p;
    loop {
        let c = m.read::<u8>(at);
        if c == 0 {
            return out;
        }
        out.push(c);
        at = at.wrapping_add(1);
    }
}

/// `strstr`'s offset of `needle` in `hay` (an empty needle is at 0).
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Writes `text` and a NUL at `at`.
fn write_c_string(m: &mut impl Mem, at: u32, text: &[u8]) {
    m.write_bytes(at, text);
    let end = u32::try_from(text.len()).unwrap_or(u32::MAX);
    m.write(at.wrapping_add(end), 0u8);
}

/// `g_EventName` (`0x023cbc60`): the current event's directory name.
const EVENT_NAME: u32 = 0x023c_bc60;
/// The event directories' language prefixes (`0x00c52768`, a word per
/// language: `""` but `fr/`, `sp/`, `de/`, `it/`).
const EVENT_LANG_DIRS: u32 = 0x00c5_2768;
/// `g_CurrentMapNo` (`0x012fbc40`).
const CURRENT_MAP_NO: u32 = 0x012f_bc40;
/// The battle-tutorial events whose texture lists are per language
/// (`bltz0000`, `bltz0005`, `bltz0006`, `bltz0200`, `bltz0201`).
const BLTZ_NAMES: [u32; 5] = [
    0x00b5_bf78,
    0x00b5_bf84,
    0x00b5_bf90,
    0x00b5_bf9c,
    0x00b5_bfa8,
];
/// The language directories the `bltz` paths get by language 0..=10
/// (the original's jump table at `0x00874f84`): `""`, `us/`, `fr/`,
/// `sp/`, `de/`, `it/`, `us/` (6-8), `kr/`, `ch/`; `us/` past 10.
const BLTZ_LANG_DIRS: [u32; 11] = [
    0x00b8_fbeb,
    0x00b5_bfc8,
    0x00b5_bf34,
    0x00b5_bf38,
    0x00b5_bf3c,
    0x00b5_bf40,
    0x00b5_bfc8,
    0x00b5_bfc8,
    0x00b5_bfc8,
    0x00b5_bfcc,
    0x00b5_bfd0,
];

/// `getEventLocTexListPath(out)` (`0x00874c50`): the current event's
/// texture list path into `out`.
///
/// The event directory is `obj/<event>` (`g_EventName`), behind the
/// language prefix (`0x00c52768[lang]`, `""` for the event names holding
/// `sins05`; no directory at all past language 10) unless the name holds
/// `bltz`. The path is
/// `/FFX_Data/GameData/PS3Data/event/<dir>/tex/TexList.txt`; for the
/// `bltz` events whose name is one of [`BLTZ_NAMES`] the language
/// directory ([`BLTZ_LANG_DIRS`]) goes in seven bytes before that name
/// (`strncpy` + `sprintf("%s%s%s")`). Then `fiosUnifyFilename(out, frame,
/// 0x100)` (`frame` holds the original's 256-byte local) and the result
/// copied back (`sprintf_s`), whose length is the `EAX` returned.
///
/// Language 11 on map `0x79` first points the prefix of language 11 at
/// `cn/` (a word past the ten the path reads).
pub fn get_event_loc_tex_list_path(h: &mut impl ffx_bridge::Host, out: u32, frame: u32) -> u32 {
    let lang = host_call(h, 0x0064_10e0, &[]);
    if host_call(h, 0x008a_c2f0, &[]) == 0xb && h.read::<u32>(h.rebase(CURRENT_MAP_NO)) == 0x79 {
        let at = h.rebase(EVENT_LANG_DIRS).wrapping_add(lang.wrapping_mul(4));
        let cn = h.rebase(0x00b5_bfb4);
        h.write(at, cn);
    }
    let name = c_string(h, h.rebase(EVENT_NAME));
    let obj = c_string(h, h.rebase(0x00b5_bfc0));
    let event_dir = |prefix: Vec<u8>| -> Vec<u8> {
        let mut dir = prefix;
        dir.extend_from_slice(&obj);
        dir.extend_from_slice(&name);
        dir
    };
    let fmt = |dir: &[u8]| -> Vec<u8> {
        let mut p = b"/FFX_Data/GameData/PS3Data/event/".to_vec();
        p.extend_from_slice(dir);
        p.extend_from_slice(b"/tex/TexList.txt");
        p
    };
    if find(&name, b"bltz").is_none() {
        let dir = if lang < 0xb {
            let k = if find(&name, b"sins05").is_some() {
                0
            } else {
                lang
            };
            let table = h.rebase(EVENT_LANG_DIRS);
            let p = h.read::<u32>(table + k * 4);
            let prefix = c_string(h, p);
            event_dir(prefix)
        } else {
            Vec::new()
        };
        write_c_string(h, out, &fmt(&dir));
    } else {
        let dir = event_dir(Vec::new());
        let path = fmt(&dir);
        write_c_string(h, out, &path);
        for n in BLTZ_NAMES {
            let needle = c_string(h, h.rebase(n));
            let Some(at) = find(&path, &needle) else {
                continue;
            };
            let cut = at.saturating_sub(7);
            let ld = *BLTZ_LANG_DIRS.get(lang as usize).unwrap_or(&0x00b5_bfc8);
            let mut text = path[..cut].to_vec();
            text.extend_from_slice(&c_string(h, h.rebase(ld)));
            text.extend_from_slice(&path[cut..]);
            write_c_string(h, out, &text);
            break;
        }
    }
    host_call(h, 0x0067_9820, &[out, frame, 0x100]);
    let unified = c_string(h, frame);
    write_c_string(h, out, &unified);
    u32::try_from(unified.len()).unwrap_or(u32::MAX)
}

/// `AtelStopSoundAll()` (`0x00871890`): every sound separate 1..=0x77
/// stopped (`SndSepStop(n | 0x80000000)`, a `TkSyncV(0)` + `TkSndUpdate`
/// every 16), the music stopped and cleared (`SndMusicStop(-1)`,
/// `SndMusicClear(-1)`), FMOD's music too (`iMusicStopAll`,
/// `iMusicClearAll`), 32 more sync-and-update pairs, the music once more,
/// then `SqsAllVoiceTerminate` (`0x008872e0`). Its `EAX` is the last
/// `TkSndUpdate`'s (a tail jump).
pub fn atel_stop_sound_all(h: &mut impl ffx_bridge::Host) -> u32 {
    const SND_SEP_STOP: u32 = 0x0088_6eb0;
    const TK_SYNC_V: u32 = 0x0088_7f00;
    const TK_SND_UPDATE: u32 = 0x0088_7b60;
    const SND_MUSIC_STOP: u32 = 0x0088_6a60;
    const SND_MUSIC_CLEAR: u32 = 0x0088_6800;
    let sync = |h: &mut _| {
        host_call(h, TK_SYNC_V, &[0]);
        host_call(h, TK_SND_UPDATE, &[])
    };
    let music = |h: &mut _| {
        host_call(h, SND_MUSIC_STOP, &[u32::MAX]);
        host_call(h, SND_MUSIC_CLEAR, &[u32::MAX]);
        sync(h);
    };
    for n in 1..0x78u32 {
        host_call(h, SND_SEP_STOP, &[n | 0x8000_0000]);
        if n.trailing_zeros() >= 4 {
            sync(h);
        }
    }
    music(h);
    host_call(h, 0x0067_a230, &[]);
    host_call(h, 0x0067_a030, &[]);
    for n in 0..0x100u32 {
        if n.trailing_zeros() >= 3 {
            sync(h);
        }
    }
    sync(h);
    music(h);
    host_call(h, 0x0088_72e0, &[]);
    sync(h)
}

/// `AtelGetHyoukaMenuEnable()` (`0x0086bae0`): whether the evaluation
/// menu may open (ctrl work 0's `+1` bit 4), with the current ctrl work
/// switched to ctrl work 0 meanwhile: not while `TkGetTGS`, in battle,
/// in an Eternal Calm scene (`isEternalCalmScene(save map)`), unless
/// ctrl work 0 has `+0` bits 0 and 1 and none of `+1` bit 2, `+3` bits
/// `0x87`, `+2` bit 5; not while a sound reads (`SndIsReadStatus`) or a
/// movie runs (`movie_run_check`). 0 too with `g_AtelOverlayStatus` 1.
pub fn atel_get_hyouka_menu_enable(h: &mut impl ffx_bridge::Host) -> u32 {
    let context = h.rebase(SCRIPT_CONTEXT);
    let saved = h.read::<u32>(context);
    let works = h.rebase(CTRL_WORKS);
    let bit = u32::from(h.read::<u8>(works + 1) >> 4) & 1;
    h.write(context, works);
    let mut on = if host_call(h, 0x0088_d6d0, &[]) == 0 {
        bit
    } else {
        0
    };
    let blocked = host_call(h, 0x0078_0c90, &[]) != 0 || {
        let map = u32::from(h.read::<u16>(h.rebase(0x0112_ca90)));
        host_call(h, 0x0067_ab50, &[map]) != 0 || {
            let c = h.read::<u32>(context);
            let f = h.read::<u8>(c);
            f & 1 == 0
                || f & 2 == 0
                || h.read::<u8>(c + 1) & 4 != 0
                || h.read::<u8>(c + 3) & 0x87 != 0
                || h.read::<u8>(c + 2) & 0x20 != 0
        }
    };
    if blocked {
        on = 0;
    }
    if host_call(h, 0x0088_6650, &[]) != 0 || host_call(h, 0x0076_f060, &[]) != 0 {
        on = 0;
    }
    let overlay = h.read::<u8>(h.rebase(0x0132_6b80));
    h.write(context, saved);
    if overlay == 1 { 0 } else { on }
}

/// `Atel_SetRunInBattle_008630c0()` (name ours; called by
/// `AtelBattleEndCallback`): with ctrl work 0's `+0` bit 0, every one of
/// its actors (`+0xc`) gets `+0x34` bit 2 (runs while in battle), looked
/// up as `AtelGetBasicWork` does (its cache, inlined). Returns the last
/// worker (the `EAX` the loop leaves), 0 without actors or the bit (the
/// original leaves its caller's `EAX` then).
pub fn atel_set_run_in_battle(m: &mut impl Mem) -> u32 {
    let c = m.rebase(CTRL_WORKS);
    if m.read::<u8>(c) & 1 == 0 {
        return 0;
    }
    let n = m.read::<u16>(c + super::super::ctrl::ACTORS);
    let mut w = 0;
    for i in 0..i32::from(n) {
        w = super::atel_get_basic_work_in(m, c, i);
        let f = m.read::<u16>(w + 0x34);
        m.write(w + 0x34, f | 4);
    }
    w
}

/// Pushes the 24-byte signal node `node` onto the list headed at `head`
/// (a worker's free nodes, `+0x88`): the old first node's `prev` (`+4`)
/// is `node`, the head `node`, `node`'s `prev` the head and its `next`
/// the old first node (stored in that order).
fn push_signal_node(m: &mut impl Mem, head: u32, node: u32) {
    let first = m.read::<u32>(head);
    if first != 0 {
        m.write(first + 4, node);
    }
    m.write(head, node);
    m.write(node + 4, head);
    m.write(node, first);
}

/// `Atel_InitFreeSignals_0086d510(worker)` (name ours; called by
/// `AtelInitBasicWork`): the worker's own signal nodes go onto its free
/// list (`+0x88`): for types 5 and 6 the line table (`+0x2e4`) is cleared
/// and the eight nodes from `+0x1c4` are pushed, else the 16 from
/// `+0x3d8` (24 bytes each). Returns the `EAX` it leaves: the list head's
/// address for types 5 and 6, else the last node.
pub fn atel_init_free_signals(m: &mut impl Mem, w: u32) -> u32 {
    let head = w + 0x88;
    if kind(m, w).wrapping_sub(5) < 2 {
        m.write(w + 0x2e4, 0_u32);
        for k in 0..8 {
            push_signal_node(m, head, w + 0x1c4 + k * 0x18);
        }
        head
    } else {
        let mut node = 0;
        for k in 0..16 {
            node = w + 0x3d8 + k * 0x18;
            push_signal_node(m, head, node);
        }
        node
    }
}

/// `Atel_ResetMsgWindowBank_00863c90(bank)` (name ours; called by
/// `AtelEnterMenuCallback`): the eight message windows of bank `bank`
/// (`i8`, eight windows of 0x2c bytes from `g_AtelMsgWindows`) reset as
/// [`atel_clear_mes_win_work`] does; `g_AtelMenuExecStatus` is stored back
/// as it was read. Returns 0.
pub fn atel_reset_msg_window_bank(m: &mut impl Mem, bank: u32) -> u32 {
    let bank = i32::from(bank.to_le_bytes()[0].cast_signed());
    let base = m
        .rebase(0x0132_6d70)
        .wrapping_add((bank * 8 * 0x2c).cast_unsigned());
    let status = m.rebase(MENU_EXEC_STATUS);
    let s = m.read::<u8>(status);
    // 0x01a00200 is a constant the image has no relocation for
    let (buf, none) = (0x01a0_0200, m.rebase(0x0132_5b74));
    for k in 0..8 {
        let w = base.wrapping_add(k * 0x2c);
        for (off, v) in [
            (4, buf),
            (0x10, 0x5000_0080),
            (8, none),
            (0, 0),
            (0x28, 0),
            (0x14, 0),
            (0x1c, 0),
        ] {
            m.write(w.wrapping_add(off), v);
        }
    }
    m.write(status, s);
    0
}

/// `SqsSeSeparateMaskClear` (`0x00886dc0`, eight ids).
const SQS_SE_SEPARATE_MASK_CLEAR: u32 = 0x0088_6dc0;
/// `SndSepClearAll` (`0x00886db0`).
const SND_SEP_CLEAR_ALL: u32 = 0x0088_6db0;

/// `Atel_LoadEventSeps_00866520(_, mode, group)` (name ours; callers
/// `0x00862470`, `AtelEventRemoveSep`, `0x00866500`): rebuilds
/// `g_AtelSepLoadList` (emptied first) from the separates table
/// ([`sep_table`]) of the first ctrl work's script block (`+0x48`,
/// `0x01325be8`). An entry
/// `(id, flags)` is added (`Atel_AddSepLoad`) when, with `mode` 0, flags
/// bit 1 is set without bit 2, or bit 3 without bit 1; with `mode` set
/// and `group` (`>= 0`) one of the selected groups (`mask | 1`), never
/// (the entry is kept when its flags have none of the group's bits: the
/// group bit rotated into the top half); otherwise when flags bit 0 is
/// set. Added ones not refused as full or unselected (1) are kept too:
/// the first 16 non-negative ids kept (the 16th slot takes the later
/// ones). The group's bit then leaves `g_AtelSepGroupMask`. With nothing
/// kept every separate is cleared (`SndSepClearAll`), else the first
/// eight kept ones stay (`SqsSeSeparateMaskClear`); the group mask ends
/// 0. Returns that call's `EAX`.
pub fn atel_load_event_seps<H: super::run::NativeHost>(h: &mut H, mode: u32, group: u32) -> u32 {
    let hdr = h.read::<u32>(h.rebase(0x0132_5be8));
    let mut keep = [u32::MAX; 16];
    let mut kept = 0usize;
    let table = sep_table(h, hdr);
    let n = if table == 0 { 0 } else { h.read::<i32>(table) };
    let rot = 1u32.rotate_left(group & 0x1f);
    let group_bits = rot.rotate_left(16);
    let mask_at = h.rebase(SEP_GROUP_MASK);
    h.write(h.rebase(SEP_LOAD_COUNT), 0_u32);
    for i in 0..n.max(0).cast_unsigned() {
        let t = sep_table(h, hdr);
        let (id, flags) = if t == 0 {
            (0, 0)
        } else {
            (
                h.read::<u32>(t.wrapping_add(i * 8 + 4)),
                h.read::<u32>(t.wrapping_add(i * 8 + 8)),
            )
        };
        let selected = || {
            let bit = (1u32 << (group & 0x1f)).to_le_bytes()[0];
            group.cast_signed() >= 0 && bit & (h.read::<u8>(mask_at) | 1) != 0
        };
        // (add it, keep it without adding)
        let (add, keep_only) = if mode == 0 {
            if flags & 2 != 0 {
                (flags & 4 == 0, false)
            } else {
                (flags & 8 != 0, false)
            }
        } else if selected() {
            (false, flags & group_bits == 0)
        } else {
            (flags & 1 != 0, false)
        };
        let keep_it = keep_only || (add && atel_add_sep_load(h, id, flags) != 1);
        if keep_it && id.cast_signed() >= 0 {
            keep[kept] = id;
            if kept < 15 {
                kept += 1;
            }
        }
    }
    if group.cast_signed() >= 0 {
        let bit = (1u32 << (group & 0x1f)).to_le_bytes()[0];
        let v = h.read::<u8>(mask_at) & !bit;
        h.write(mask_at, v);
    }
    let r = if keep[0] == u32::MAX {
        let f = h.rebase(SND_SEP_CLEAR_ALL);
        h.call(f, &[])
    } else {
        let f = h.rebase(SQS_SE_SEPARATE_MASK_CLEAR);
        h.call_n(f, &keep[..8])
    };
    h.write(mask_at, 0_u8);
    r
}

/// `g_MoviePictureOn` (`0x0112a00c`; next to `g_WaitFramesByTime`): a movie
/// picture to destroy.
const MOVIE_PICTURE_ON: u32 = 0x0112_a00c;
/// `g_WaitFramesByTime` (`0x0112a008`).
const WAIT_FRAMES_BY_TIME: u32 = 0x0112_a008;

/// `destroyMoviePicture()` (`0x0076ece0`): with a movie picture up
/// ([`MOVIE_PICTURE_ON`]), the time-counted frame wait off, `0x00c40e2c` =
/// 1, the picture flag cleared and `graphicDestroyFmv` (a tail jump: its
/// `EAX`). Else nothing (the caller's `EAX` stays; 0 here).
pub fn destroy_movie_picture(h: &mut impl ffx_bridge::Host) -> u32 {
    let on = h.rebase(MOVIE_PICTURE_ON);
    if h.read::<u32>(on) == 0 {
        return 0;
    }
    h.write(h.rebase(WAIT_FRAMES_BY_TIME), 0_u32);
    h.write(h.rebase(0x00c4_0e2c), 1_u32);
    h.write(on, 0_u32);
    host_call(h, 0x0063_df30, &[])
}

/// `Movie_IsBusy_0076ed10()` (name ours; callers `Sg_MainCheckMenu`,
/// `0x00873620`): 1 unless a video plays (`graphicIsVideoPlaying` or
/// `0x00641ce0`) with neither `movie_with3DScene` nor a movie picture up
/// ([`MOVIE_PICTURE_ON`]), which gives 0. All three are asked first.
pub fn movie_is_busy(h: &mut impl ffx_bridge::Host) -> u32 {
    let playing = host_call(h, 0x0064_1cf0, &[]);
    let other = host_call(h, 0x0064_1ce0, &[]);
    let scene =
        host_call(h, 0x0076_f0c0, &[]) != 0 || h.read::<u32>(h.rebase(MOVIE_PICTURE_ON)) != 0;
    u32::from(!((playing != 0 || other != 0) && !scene))
}

/// `TkMenuSaveLoad_PollCard_008b3d20()` (name ours; callers
/// `TkMenuCtrlSaveLoad`, `TkMenuCloseSaveLoadScreen`): the save screen's
/// memory card poll by its step (`0x0186626b`): 0 moves to 1; 1 asks
/// `TkMcGetInfo(0, &0x0186626c, 0, 0)` and sets `0x0186626c` to 2, done
/// when that returns 0; 2 is done. Done: the save choice
/// (`g_TkMenuSaveChoiceEnabled`) is 1 (from step 2 only when
/// `0x0186626c` is 2, else 0), the step 0 and `g_TkMenuSaveLoadToggles`
/// flipped (to `== 0`). Returns the `EAX` it leaves: `TkMcGetInfo`'s when
/// not 0, the step minus 2 for steps past 2, else 0.
pub fn tk_menu_save_load_poll_card(h: &mut impl ffx_bridge::Host) -> u32 {
    let step_at = h.rebase(0x0186_626b);
    let info = h.rebase(0x0186_626c);
    let choice = h.rebase(0x0186_6268);
    let step = h.read::<u8>(step_at);
    let enabled = match step {
        0 | 1 => {
            if step == 0 {
                h.write(step_at, 1_u8);
            }
            let r = host_call(h, 0x0088_82c0, &[0, info, 0, 0]);
            h.write(info, 2_u32);
            if r != 0 {
                return r;
            }
            1
        }
        2 => u8::from(h.read::<u32>(info) == 2),
        _ => return u32::from(step) - 2,
    };
    h.write(choice, enabled);
    let toggles = h.rebase(0x0186_626a);
    h.write(step_at, 0_u8);
    let t = h.read::<u8>(toggles);
    h.write(toggles, u8::from(t == 0));
    0
}

/// `Atel_SetVarSlot_00878140(worker, args, stack)` (name ours; the core of
/// `Native_101c`/`Native_101d`): pops a variable word and a slot (0..=7,
/// clamped) and points the current ctrl work's slot record (`+0x1a4 +
/// slot*8`) at the variable (`AtelGetRealVarAdrs`; 0 for word 0). Returns
/// the record's address.
pub fn atel_set_var_slot(h: &mut impl ffx_bridge::Host, w: u32, s: u32) -> u32 {
    let word = super::atel_pop_stack_integer(h, s);
    let slot = super::atel_pop_stack_integer(h, s)
        .cast_signed()
        .clamp(0, 7);
    let c = super::atel_get_cur_ctrl_work(h);
    let rec = c.wrapping_add(slot.cast_unsigned() * 8) + 0x1a4;
    let v = if word == 0 {
        0
    } else {
        super::atel_get_real_var_adrs(h, w, word)
    };
    h.write(rec, v);
    rec
}

/// `Atel_GetEventActorRot_0086b270(i, x, y, z)` (name ours; callers
/// `Snd3dFootRequest`, `AtelDebugCtrl`): [`atel_get_event_actor_pos`]'s
/// twin for the position block's rotation (`+0x30`, `+0x34`, `+0x38`): x,
/// z, the current ctrl work back, then y. `EAX` the block's address.
pub fn atel_get_event_actor_rot(m: &mut impl Mem, i: u32, x: u32, y: u32, z: u32) -> u32 {
    let ctx = m.rebase(SCRIPT_CONTEXT);
    let old = m.read::<u32>(ctx);
    m.write(ctx, m.rebase(CTRL_WORKS));
    let n = i32::from(m.read::<u16>(m.rebase(CTRL_WORKS + 0xc)));
    let i = if i.cast_signed() < 0 || i.cast_signed() >= n {
        m.read::<u32>(m.rebase(0x0132_6b2c))
    } else {
        i
    };
    let w = super::atel_get_basic_work(m, i.cast_signed());
    let p = pos_block(m, w);
    if x != 0 {
        copy_f32(m, p + 0x30, x);
    }
    if z != 0 {
        copy_f32(m, p + 0x38, z);
    }
    m.write(ctx, old);
    if y != 0 {
        copy_f32(m, p + 0x34, y);
    }
    p
}

/// `Atel_TransSepGroup_00871780(hdr, data, sep_only, group, flag)` (name
/// ours; callers `0x00862220`, `AtelEventSetUpSep`,
/// `AtelSndTransSepSPKDataGroup`): waits out the sound transfer
/// (`SndSyncTrans(0)`, a `TkSyncV(0)` + `TkSndUpdate2` per turn); a
/// negative `group` queues the event's pending separates
/// (`Atel_QueueSepLoads(hdr)`), any other sets `flag`. Without `data`, 0.
/// Else a `data` word with bit 30 set and bit 31 clear holds the first
/// group's size (low 30 bits), the next words the later groups' sizes, the
/// groups themselves from `data + 0x20`: group `group` (the sizes before
/// it skipped, summed in pairs as the original does) of a non-zero size
/// goes to `SndSepTransSepOnly` (with `flag` and `sep_only`) or
/// `SndSepTransNonblock`, and 1. A `group >= 0` is then selected in
/// `g_AtelSepGroupMask`. Returns 1 for a transfer, else 0.
pub fn atel_trans_sep_group(
    h: &mut impl ffx_bridge::Host,
    hdr: u32,
    data: u32,
    sep_only: u32,
    group: u32,
    flag: u32,
) -> u32 {
    while host_call(h, 0x0088_7910, &[0]) != 0 {
        host_call(h, 0x0088_7f00, &[0]);
        host_call(h, 0x0088_7b80, &[]);
    }
    let g = group.cast_signed();
    let flag = if g < 0 {
        host_call(h, 0x0086_38c0, &[hdr]);
        flag
    } else {
        1
    };
    if data == 0 {
        return 0;
    }
    let head = h.read::<u32>(data);
    let mut sent = 0;
    if head & 0xc000_0000 != 0 && head.cast_signed() >= 0 {
        let mut size = head & 0x3fff_ffff;
        let mut at = data + 4;
        let mut base = data + 0x20;
        let (mut even, mut odd) = (0u32, 0u32);
        let mut rest = g;
        if g >= 2 {
            let pairs = ((g - 2).cast_unsigned() >> 1) + 1;
            rest = g.wrapping_sub((pairs * 2).cast_signed());
            for _ in 0..pairs {
                odd = odd.wrapping_add(h.read::<u32>(at));
                even = even.wrapping_add(size);
                size = h.read::<u32>(at + 4);
                at += 8;
            }
        }
        if rest > 0 {
            base = base.wrapping_add(size);
            size = h.read::<u32>(at);
        }
        let from = base.wrapping_add(even.wrapping_add(odd));
        if size != 0 {
            sent = 1;
            let f = if flag != 0 && sep_only != 0 {
                0x0088_6f30
            } else {
                0x0088_6f00
            };
            host_call(h, f, &[from, size]);
        }
    }
    if g >= 0 {
        let mask = h.rebase(SEP_GROUP_MASK);
        let bit = (1u32 << (group & 0x1f)).to_le_bytes()[0];
        let v = h.read::<u8>(mask) | bit;
        h.write(mask, v);
    }
    sent
}

/// `AtelGetGuideMapActorInfo(i, out)` (`0x0086b740`): the guide map's
/// record of the event's marker `i` (ctrl work 0 current meanwhile).
/// `out` (`+0x18` and `+0x2c` cleared first) gets, for an `i` below the
/// count of the script block's marker table (`+0x2c` -> `+0x18`) while
/// ctrl work 0 runs (`+0` bit 0): the worker (`Atel_GetBinHdrTable18B`)
/// `+0`, the marker (`Atel_GetBinHdrTable18A`, or the worker's `+0xbc`
/// without its sign bit when that is negative; `| 0x800` for the ctrl
/// work's top worker while ctrl work 0's `+2` bit 7) `+4`, the worker's
/// position (`Script_GetWorkerPos`) `+8`..`+0x10`, its `+0xb4` `+0x18`,
/// its start and end x, z (`AtelGetActorStartPos`/`EndPos`; y into
/// `frame`) `+0x1c`..`+0x28`, its direction `+0x14` (0 for types 2, 3, 5
/// and 6; `Atel_GetWorkerRotY` else), `+0x2c` 1 for types 2 and 5.
/// Returns the worker's `+0x36` bit 11. Past the count every field is 0
/// (and 0 returned).
pub fn atel_get_guide_map_actor_info(m: &mut impl Mem, i: u32, out: u32, frame: u32) -> u32 {
    let ctx = m.rebase(SCRIPT_CONTEXT);
    let old = m.read::<u32>(ctx);
    let works = m.rebase(CTRL_WORKS);
    m.write(ctx, works);
    if out != 0 {
        m.write(out + 0x18, 0_u32);
        m.write(out + 0x2c, 0_u8);
    }
    let b = m.read::<u32>(works + 0x48);
    let mut ret = 0;
    if m.read::<u8>(works) & 1 != 0 && b != 0 && out != 0 {
        let t = m.read::<u32>(b + 0x2c);
        let n = if t == 0 || b.wrapping_add(t) == 0 {
            0
        } else {
            let u = m.read::<u32>(b.wrapping_add(t) + 0x18);
            if u == 0 {
                0
            } else {
                m.read::<i32>(u.wrapping_add(b))
            }
        };
        if i.cast_signed() < n {
            let id = atel_get_bin_hdr_table18_b(m, b, i);
            let mut marker = atel_get_bin_hdr_table18_a(m, b, i);
            let w = super::atel_get_basic_work(m, id.cast_signed());
            ret = u32::from(m.read::<u16>(w + 0x36) >> 11) & 1;
            m.write(out, id);
            m.write(out + 0x18, w + 0xb4);
            let over = m.read::<u32>(w + 0xbc);
            if over.cast_signed() < 0 {
                marker = over & 0x7fff_ffff;
            }
            if m.read::<u8>(works + 2) & 0x80 != 0
                && id == super::globals::atel_event_get_now_ctrl_top_id(m)
            {
                marker |= 0x800;
            }
            m.write(out + 4, marker);
            let k = u32::from(m.read::<u16>(w + 0x2e));
            script_get_worker_pos(m, k, out + 8, out + 0xc, out + 0x10);
            atel_get_actor_start_pos(m, k, out + 0x1c, frame, out + 0x20);
            atel_get_actor_end_pos(m, k, out + 0x24, frame, out + 0x28);
            match kind(m, w) {
                2 | 5 => {
                    m.write(out + 0x14, 0_u32);
                    m.write(out + 0x2c, 1_u8);
                }
                3 | 6 => m.write(out + 0x14, 0_u32),
                _ => {
                    let r = atel_get_worker_rot_y(m, k);
                    m.write(out + 0x14, x87::store(x87::load(r.to_bits())));
                }
            }
        } else {
            for off in [0, 8, 4, 0xc, 0x10, 0x1c, 0x20, 0x24, 0x28, 0x14] {
                m.write(out + off, 0_u32);
            }
        }
    }
    m.write(ctx, old);
    ret
}

/// `AtelSndTransSepSPKDataGroup(hdr, data, group)` (`0x00871750`):
/// [`atel_trans_sep_group`]`(hdr, data, 1, group, 0)`, then
/// `SndSyncTrans(1)`; returns the first's result.
pub fn atel_snd_trans_sep_spk_data_group(
    h: &mut impl ffx_bridge::Host,
    hdr: u32,
    data: u32,
    group: u32,
) -> u32 {
    let r = atel_trans_sep_group(h, hdr, data, 1, group, 0);
    host_call(h, 0x0088_7910, &[1]);
    r
}

/// `AtelEventRemoveSep(mode)` (`0x008661d0`): `SndStopStream`, then with
/// ctrl work 0's script block ([`CTRL_WORKS`] `+0x48`) and ctrl work 0
/// current [`atel_load_event_seps`]`(block, mode, -1)` and the current
/// ctrl work back. Without a block ctrl work 0 stays current (as in the
/// original) and 0 is returned; else the reload's `EAX`.
pub fn atel_event_remove_sep<H: super::run::NativeHost>(h: &mut H, mode: u32) -> u32 {
    host_call(h, 0x0088_7780, &[]);
    let works = h.rebase(CTRL_WORKS);
    let b = h.read::<u32>(works + 0x48);
    let context = h.rebase(SCRIPT_CONTEXT);
    let saved = h.read::<u32>(context);
    h.write(context, works);
    if b == 0 {
        return 0;
    }
    let r = atel_load_event_seps(h, mode, u32::MAX);
    h.write(context, saved);
    r
}

/// `AtelEnterMenuCallback()` (`0x008655e0`): the guide map hidden
/// (`Yn_GuideMapSetDispProg(0)`), vibration off for pads 1..=7
/// (`TkSetVibrateMask(pad, 0)`), `MsEventEffectFree`, message bank 1
/// (`g_AtelMenuExecStatus` = 1, [`atel_reset_msg_window_bank`]`(1)`),
/// `TkMenuClearDummyControler`, `g_AtelOverlayStatus` = 3. Returns the
/// last call's `EAX`.
pub fn atel_enter_menu_callback(h: &mut impl ffx_bridge::Host) -> u32 {
    host_call(h, 0x0091_9d70, &[0]);
    for pad in 1..8 {
        host_call(h, 0x0087_5f60, &[pad, 0]);
    }
    host_call(h, 0x0078_8b90, &[]);
    h.write(h.rebase(MENU_EXEC_STATUS), 1_u8);
    atel_reset_msg_window_bank(h, 1);
    let r = host_call(h, 0x008c_a540, &[]);
    h.write(h.rebase(0x0132_6b80), 3_u8);
    r
}

/// `AtelEventSetUpSep(hdr, data, sep_only)` (`0x00866430`): with ctrl work
/// 0 current, `sep_only` 1 keeps `data` in ctrl work 0's `+0x204`;
/// [`atel_trans_sep_group`]`(hdr, data, sep_only, -1, 0)`,
/// `SndSyncTrans(1)`, the current ctrl work back, then the 3D sound
/// ratios: total 0, difference 1/60 (`0x3c888889`). Returns the last
/// call's `EAX`.
pub fn atel_event_set_up_sep(
    h: &mut impl ffx_bridge::Host,
    hdr: u32,
    data: u32,
    sep_only: u32,
) -> u32 {
    let context = h.rebase(SCRIPT_CONTEXT);
    let saved = h.read::<u32>(context);
    let works = h.rebase(CTRL_WORKS);
    h.write(context, works);
    if sep_only == 1 {
        h.write(works + 0x204, data);
    }
    atel_trans_sep_group(h, hdr, data, sep_only, u32::MAX, 0);
    host_call(h, 0x0088_7910, &[1]);
    h.write(context, saved);
    host_call(h, 0x0087_7460, &[0]);
    let sixtieth = h.read::<u32>(h.rebase(0x00b3_04f4));
    host_call(h, 0x0087_7450, &[x87::store(x87::load(sixtieth))])
}

/// `Atel_RequestBattleEndEntries_0086e8d0(tag)` (name ours; callers
/// `AtelBattleEndCallback`, `0x008617d0`): the current ctrl work's script
/// block's battle-end table (`+0x24`, relative to the block: a count, then
/// 16-byte entries from `+4`: tag, worker `u16`, entry `u16`): every
/// entry of `tag` (every entry with `AtelGetBattleEndTagAlways`) has its
/// worker signalled to run the entry (`(0xffff, worker, 0, 8, entry)`,
/// not for worker `0xffff`) and its signals decoded
/// (`AtelDecodeSignal(worker, -1)`). Returns the `EAX` it leaves: the
/// table, the block without a table, 0 without a block.
pub fn atel_request_battle_end_entries(h: &mut impl ffx_bridge::Host, tag: u32) -> u32 {
    let c = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    let b = h.read::<u32>(c + 0x48);
    if b == 0 {
        return 0;
    }
    let t = h.read::<u32>(b + 0x24);
    if t == 0 {
        return b;
    }
    let tab = b.wrapping_add(t);
    let mut i = 0u32;
    while i < h.read::<u32>(tab) {
        let e = tab + 4 + i * 0x10;
        if h.read::<u32>(e) == tag || super::globals::atel_get_battle_end_tag_always(h) != 0 {
            let worker = u32::from(h.read::<u16>(e + 4));
            let entry = u32::from(h.read::<u16>(e + 6));
            if worker != 0xffff {
                let r = super::signal::Request::new(0xffff, worker, 0, 8, entry);
                super::signal::atel_regist_signal_main(h, r);
            }
            super::signal::atel_decode_signal(h, worker, -1);
        }
        i += 1;
    }
    tab
}

/// `Atel_SetLevelTarget_0086f940(worker, i)` (name ours; callers
/// `Native_003b`, `0x00868990`): the current level's turn block (`+0x170 +
/// level*0x4c`, the level clamped as the turn helpers do) gets `i` (`u16`
/// at `+0`), and [`atel_set_level_vectors`] the position of worker `i`
/// (`AtelGetBasicWork`; type 4 reads through 0). Returns that call's
/// `EAX`.
pub fn atel_set_level_target(m: &mut impl Mem, w: u32, i: u32) -> u32 {
    let level = clamp_level(m, w, u32::from(m.read::<u8>(w + 0x32)));
    let target = super::atel_get_basic_work(m, i.cast_signed());
    let p = pos_block(m, target);
    let block = m.read::<u32>(w + 0x170 + level * 0x4c);
    m.write(
        block,
        u16::from_le_bytes([i.to_le_bytes()[0], i.to_le_bytes()[1]]),
    );
    let [x, y, z] = [0, 4, 8].map(|k| through(m, p.wrapping_add(k)));
    atel_set_level_vectors(m, w, x, y, z)
}

/// `0x007ba3c0` and `0x007ba460` (worker, args, stack, side): pop three
/// variable words (z, y, x: the first popped is the last output), give the
/// worker's camera (`AtelGetCameraWorkAdrs`, `+0` taken from
/// `MsAtelGetCameraID(5, worker is the event map camera, 0)` when 0) to
/// `get(camera, side, &x, &y, &z)`: `Cam_GetPos` (`0x007bd420`) for the
/// first, `Cam_GetRefPos_007bd4b0` for the second. Returns its `EAX`.
pub fn atel_cam_get_vars(
    h: &mut impl ffx_bridge::Host,
    get: u32,
    w: u32,
    s: u32,
    side: u32,
) -> u32 {
    let mut vars = [0u32; 3];
    for v in &mut vars {
        let word = super::atel_pop_stack_integer(h, s);
        *v = super::atel_get_real_var_adrs(h, w, word);
    }
    let cam = super::atel_get_camera_work_adrs(w);
    let mut id = h.read::<u32>(cam);
    if id == 0 {
        let actor = super::atel_get_actor_id(h, w);
        let is_map = u32::from(super::atel_get_event_map_camera_id(h) == actor);
        id = host_call(h, 0x007b_a780, &[5, is_map, 0]);
        h.write(cam, id);
    }
    host_call(h, get, &[id, side, vars[2], vars[1], vars[0]])
}

/// `Vibrate_UpdateVolume_00875b40()` (name ours; callers `0x00872bd0`,
/// `AtelExecVibrate`): unless vibration is off (`g_TkVibrateOff`), the
/// two channel levels of every pad allowed by `g_TkVibrateMask`
/// (`Vibrate_EntryLevel(pad, 0)` and `(pad, 1)`, 0x24-byte pads from
/// `g_TkVibratePads`) after ending pads whose state (`+0x22`) is 1 with
/// both channels at `0x200` or 2 with both at `0xfffe` (`+6`, `+0x16`):
/// their maxima become `g_TkVibrateVolume`. Returns the `EAX` the loop
/// leaves (the last allowed pad's second level, or the mask), 0 when off.
pub fn vibrate_update_volume(h: &mut impl ffx_bridge::Host) -> u32 {
    if h.read::<u8>(h.rebase(0x0132_8272)) != 0 {
        return 0;
    }
    let pads = h.rebase(0x0132_8150);
    let (mut lo, mut hi) = (0u32, 0u32);
    let mut eax = 0;
    for pad in 0..8u32 {
        let mask = u32::from(h.read::<u16>(h.rebase(0x0132_8274)));
        if mask & (1 << pad) == 0 {
            eax = mask;
            continue;
        }
        let p = pads + pad * 0x24;
        let state = h.read::<u8>(p + 0x22);
        let end = match state {
            1 => 0x200,
            2 => 0xfffe,
            _ => 0,
        };
        if end != 0 && h.read::<u16>(p + 6) == end && h.read::<u16>(p + 0x16) == end {
            h.write(p + 0x22, 0_u8);
        }
        let a = host_call(h, 0x0087_5c20, &[p, 0]);
        let b = host_call(h, 0x0087_5c20, &[p, 1]);
        lo = lo.max(a);
        hi = hi.max(b);
        eax = b;
    }
    let vol = h.rebase(0x0132_8270);
    h.write(vol, lo.to_le_bytes()[0]);
    h.write(vol + 1, hi.to_le_bytes()[0]);
    eax
}

/// `Atel_RequestOwnerTrigger_008764a0(owner, kind)` (name ours; caller
/// `0x0083d3b0`): with ctrl work 0 current, the worker bound to `owner`
/// (`Atel_FindWorkerByOwner`) gets the trigger request
/// ([`atel_request_trigger`]`(0xffff, worker, kind)`); the current ctrl
/// work back. Returns the request's result, 0 without a worker.
pub fn atel_request_owner_trigger(h: &mut impl ffx_bridge::Host, owner: u32, kind: u32) -> u32 {
    let old = super::atel_change_current_ctrl(h, 0);
    let i = host_call(h, 0x0087_6440, &[owner]);
    let r = if i.cast_signed() < 0 {
        0
    } else {
        atel_request_trigger(h, 0xffff, i, kind)
    };
    super::atel_set_current_ctrl(h, old);
    r
}

/// `TkMenuSaveLoad_ClearLoadPos_008b3cd0(slot)` (name ours; caller
/// `AtelAllClearCallbackQue`): `TkMcSetLoadFilePos(slot, -1)`, its `EAX`.
pub fn tk_menu_save_load_clear_load_pos(h: &mut impl ffx_bridge::Host, slot: u32) -> u32 {
    host_call(h, 0x0088_8480, &[slot, u32::MAX])
}

/// `Atel_InitMovieState_00873c00()` (name ours; caller `AtelInitTotal`):
/// the save event's `+0xe0` and the three movie buffer pointers
/// (`g_AtelMovieBuffers`) cleared. Returns the save event's address.
pub fn atel_init_movie_state(h: &mut impl ffx_bridge::Host) -> u32 {
    let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
    h.write(save.wrapping_add(0xe0), 0_u32);
    let bufs = h.rebase(0x0132_7110);
    for k in 0..3 {
        h.write(bufs + k * 4, 0_u32);
    }
    save
}

/// `Atel_QueueBattleEndEffect_00861550(kind, v)` (name ours; caller
/// `AtelBattleEndCallback`): `0x01326b40` = kind, `0x01326b44` = v, the
/// callback `0x00861f90` queued as `"EFF"` (`AtelRegistCallbackQue2(0, f,
/// name)`); unless kind is `0x121`, `g_AtelEffectState` = 1. Returns the
/// queue's `EAX`.
pub fn atel_queue_battle_end_effect(m: &mut impl Mem, kind: u32, v: u32) -> u32 {
    m.write(m.rebase(0x0132_6b40), kind);
    m.write(m.rebase(0x0132_6b44), v);
    let f = m.rebase(0x0086_1f90);
    let name = m.rebase(0x00b5_9944);
    let r = atel_regist_callback_que2(m, 0, f, name);
    if kind != 0x121 {
        m.write(m.rebase(0x0132_6b58), 1_u32);
    }
    r
}

/// `Atel_FadeOutQueuedSeps_00868460()` (name ours; caller
/// `AtelEventKickBGSep`): every queued separate (`g_AtelSepQueueCount`,
/// ids and fades from index 0 here) fades out
/// (`SndSepVolumeFadeOut(id, fade)`), and the queue empties. Returns the
/// last fade's `EAX` (the caller's with nothing queued; 0 here).
pub fn atel_fade_out_queued_seps(h: &mut impl ffx_bridge::Host) -> u32 {
    let count = h.rebase(SEP_QUEUE_COUNT);
    let ids = h.rebase(0x0132_7080);
    let fades = h.rebase(SEP_QUEUE_FADES + 4);
    let mut r = 0;
    let mut i = 0u32;
    while i.cast_signed() < h.read::<i32>(count) {
        let id = h.read::<u32>(ids + i * 4);
        let fade = h.read::<u32>(fades + i * 4);
        r = host_call(h, 0x0088_6f90, &[id, fade]);
        i += 1;
    }
    h.write(count, 0_u32);
    r
}

/// A menu window (`TkMenuMainAllocWindow`) with its callbacks (`+8`,
/// `+0xc`, `+0x10`, `+0x14`; code addresses, 0 for none) and mode
/// (`+0x3e`), kept in `slot`, then registered (`TkMenuMainRegistWindow`).
fn menu_window(
    h: &mut impl ffx_bridge::Host,
    slot: u32,
    calls: [u32; 4],
    mode: u8,
    arg: Option<u32>,
) -> u32 {
    let w = host_call(h, 0x008a_a1a0, &[]);
    if arg.is_none() {
        h.write(h.rebase(slot), w);
    }
    for (k, f) in (0u32..).zip(calls) {
        let v = if f == 0 { 0 } else { h.rebase(f) };
        h.write(w + 8 + k * 4, v);
    }
    if arg.is_none() {
        h.write(w + 0x1c, 0_u32);
    }
    h.write(w + 0x3e, mode);
    if let Some(a) = arg {
        h.write(h.rebase(slot), w);
        h.write(w + 0x58, a);
    }
    host_call(h, 0x008a_ab00, &[w]);
    w
}

/// `TkMenuSaveLoad_OpenWindow_008b3c20()` (name ours; caller
/// `TkMenuCtrlSaveLoad`; a near-copy of `MenuNameEntry_RegistWindow`): the
/// save screen's window (`0x01866260`): draw `0x008b37c0`, `+0x1c` 0,
/// mode 4. Returns it.
pub fn tk_menu_save_load_open_window(h: &mut impl ffx_bridge::Host) -> u32 {
    menu_window(h, 0x0186_6260, [0, 0, 0x008b_37c0, 0], 4, None)
}

/// `TkMenuSaveLoad_OpenChoice_008b3c70(arg)` (name ours; caller
/// `TkMenuCtrlSaveLoad`; a near-copy of `MenuSave_OpenWindow2`): the save
/// choice window (`0x0186625c`): `TkMenuInitSaveChoice_008b3680`,
/// `0x008b37b0`, `TkMenuDrawSaveChoice_008b3860`, `0x008b3cc0`, mode 1,
/// `+0x58` = arg. Returns it.
pub fn tk_menu_save_load_open_choice(h: &mut impl ffx_bridge::Host, arg: u32) -> u32 {
    menu_window(
        h,
        0x0186_625c,
        [0x008b_3680, 0x008b_37b0, 0x008b_3860, 0x008b_3cc0],
        1,
        Some(arg),
    )
}

/// `Atel_ShowWaterFps_008682a0(on)` (name ours; caller `0x00865740`): the
/// four effect ids of `g_AtelMCFPInWater` (`0x01326cf0`; negative = none)
/// restart and show (`Fp_Restart`, `pppFpIDdispOn`) or hide
/// (`pppFpIDdispOff`). Returns the last call's `EAX` (0 without one).
pub fn atel_show_water_fps(h: &mut impl ffx_bridge::Host, on: u32) -> u32 {
    let ids = h.rebase(0x0132_6cf0);
    let mut r = 0;
    for k in 0..4 {
        let id = h.read::<u32>(ids + k * 4);
        if id.cast_signed() < 0 {
            continue;
        }
        r = if on == 0 {
            host_call(h, 0x0072_9570, &[id])
        } else {
            host_call(h, 0x0072_9700, &[id]);
            host_call(h, 0x0072_95b0, &[id])
        };
    }
    r
}

/// `Atel_SetSaveEventBit_008700e0(n)` (name ours; caller `Native_00fc`):
/// bit `n` of the save event's two flag words (`+0x18`, `TkGetSaveEventAddress`;
/// C division and remainder, so a negative `n` reaches below them); 1, or 0
/// for an `n` from 64 on.
pub fn atel_set_save_event_bit(h: &mut impl ffx_bridge::Host, n: u32) -> u32 {
    let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
    let n = n.cast_signed();
    let q = n / 32;
    if q >= 2 {
        return 0;
    }
    let bit = 1u32 << ((n % 32).cast_unsigned() & 0x1f);
    let at = save
        .wrapping_add(0x18)
        .wrapping_add(q.cast_unsigned().wrapping_mul(4));
    let v = h.read::<u32>(at) | bit;
    h.write(at, v);
    1
}

// ---- round 25 ----

/// `AtelExecSoftWareReset()` (`0x008738d0`; callers `Sg_MainLoop`,
/// `0x00872bd0`): with ctrl work 0 current, a soft reset is allowed when
/// its flags allow one (`+0` bit 0 set, `+3` bit `0x80` and `+0x210`
/// bit 2 clear, `+0x210` bit `0x40` set), no movie plays (`0x0086d940`
/// returns 0, or `+3` has bits `0x3` clear), the map is not `0x15c`
/// (`0x0086c1c0`) and no voice is on standby or loading. The ctrl work
/// back, `TkSoftWareReset` runs when the reset mode word
/// (`0x01327108`) is 2, or when allowed and `TkCheckSoftWareReset`
/// says so. Returns the last callee's `EAX`.
pub fn atel_exec_soft_ware_reset(h: &mut impl ffx_bridge::Host) -> u32 {
    let old = super::atel_change_current_ctrl(h, 0);
    let c = super::atel_get_cur_ctrl_work(h);
    let mut ok = h.read::<u8>(c) & 1 != 0;
    if host_call(h, 0x0086_d940, &[]) != 0 || h.read::<u8>(c + 3) & 3 != 0 {
        ok = false;
    }
    ok &= h.read::<u8>(c + 3) & 0x80 == 0;
    ok &= h.read::<u8>(c + 0x210) & 2 == 0;
    ok &= host_call(h, 0x0086_c1c0, &[]) != 0x15c;
    ok &= h.read::<u8>(c + 0x210) & 0x40 != 0;
    ok &= host_call(h, 0x0088_7b20, &[]) == 0;
    ok &= host_call(h, 0x0088_7b10, &[]) == 0;
    let r = super::atel_set_current_ctrl(h, old);
    if h.read::<u32>(h.rebase(0x0132_7108)) == 2 {
        return host_call(h, 0x0087_2550, &[]);
    }
    if !ok {
        return r;
    }
    let r = host_call(h, 0x0087_2350, &[]);
    if r == 0 {
        r
    } else {
        host_call(h, 0x0087_2550, &[])
    }
}

/// `AtelClearShape()` (`0x008742b0`; callers `AtelInitTotal`,
/// `AtelEventRemove`): every event sprite (`g_EventSprites`) reset (`+0`
/// = `0x8000`, `+2` = 0, `+4` = 0, scale `0x100`/`0x100`, colour
/// `0x80808080`; `is_ptr32_null(+0x14)` asked and ignored) and its six
/// effect handles freed (`graphicVFXDestroy(i + k*10000, sprite)`, a
/// 64-bit id with the sprite's address high). With a texture list loaded
/// (`0x01327140`), the event's list path (`getEventLocTexListPath`, into
/// the first 256 bytes of `frame`; its own local in the next 256) is
/// unloaded (`graphicAtelUnloadTexture`). Then the list flag, `0x01327144`,
/// `0x023cbc5c` = 0, `0x01327f48` = -0xa0, `0x01327f4c` = 1. Returns the
/// last callee's `EAX`.
pub fn atel_clear_shape(h: &mut impl ffx_bridge::Host, frame: u32) -> u32 {
    let base = h.rebase(EVENT_SPRITES);
    let mut r = 0;
    for i in 0..0x80u32 {
        let s = base + i * 0x1c;
        h.write(s, 0x8000_u16);
        h.write(s + 2, 0_u8);
        h.write(s + 4, 0_u32);
        h.write(s + 8, 0x0100_0100_u32);
        h.write(s + 0x10, 0x8080_8080_u32);
        let data = h.read::<u32>(s + 0x14);
        host_call(h, 0x00a4_46a0, &[data]);
        for k in 0..6u32 {
            r = host_call(h, 0x0064_5280, &[k * 10_000 + i, s]);
        }
    }
    let loaded = h.rebase(0x0132_7140);
    if h.read::<u32>(loaded) != 0 {
        get_event_loc_tex_list_path(h, frame, frame + 0x100);
        r = host_call(h, 0x0063_9330, &[frame]);
    }
    h.write(loaded, 0_u32);
    h.write(h.rebase(0x0132_7144), 0_u32);
    h.write(h.rebase(0x0132_7f48), 0xffff_ff60_u32);
    h.write(h.rebase(0x023c_bc5c), 0_u32);
    h.write(h.rebase(0x0132_7f4c), 1_u32);
    r
}

/// `Atel_TranslateBGSeps_00861020(volume, frames, all)` (name ours;
/// callers `AtelBGSepFade`, `0x00862470`): with ctrl work 0 current and
/// running (`+0` bit 0), every entry of its event's separates table
/// (`+0x48` header) that is a background separate (flags bit 2, or bits
/// 2 or 8 with `all`; not `0x30`) gets `SndSepVolumeTranslate(id,
/// volume, t)`: t is `frames`, halved (`frames*128/256`, C division) with
/// the save event's `+0x28` set; a flagged `0x80` entry gets 0 instead
/// when `+0x28` is clear, the full `frames` when it and `0x40` are set. The ctrl work back. Returns that work, or the table's
/// count when it is 0 or less, or the save event's address when ctrl
/// work 0 is idle.
pub fn atel_translate_bg_seps(
    h: &mut impl ffx_bridge::Host,
    volume: u32,
    frames: u32,
    all: u32,
) -> u32 {
    let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let works = h.rebase(CTRL_WORKS);
    let old = h.read::<u32>(ctx);
    h.write(ctx, works);
    if h.read::<u8>(works) & 1 == 0 {
        h.write(ctx, old);
        return save;
    }
    let hdr = h.read::<u32>(works + 0x48);
    let t = sep_table(h, hdr);
    let n = if t == 0 { 0 } else { h.read::<i32>(t) };
    if n <= 0 {
        h.write(ctx, old);
        return n.cast_unsigned();
    }
    for i in 0..n.cast_unsigned() {
        let t = sep_table(h, hdr);
        let id = if t == 0 {
            0
        } else {
            h.read::<u32>(t + i * 8 + 4)
        };
        let t = sep_table(h, hdr);
        let flags = if t == 0 {
            0
        } else {
            h.read::<u32>(t + i * 8 + 8)
        };
        let mask = if all != 0 { 0xa } else { 2 };
        if flags & 0x30 != 0 || flags & mask == 0 {
            continue;
        }
        let fixed = if flags & 0x80 == 0 {
            None
        } else if h.read::<u8>(save + 0x28) == 0 {
            Some(0)
        } else if flags & 0x40 != 0 {
            Some(frames)
        } else {
            None
        };
        let t = fixed.unwrap_or_else(|| {
            let s = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
            if h.read::<u8>(s + 0x28) == 0 {
                frames
            } else {
                (frames.cast_signed().wrapping_shl(7) / 256).cast_unsigned()
            }
        });
        host_call(h, 0x0088_6ff0, &[id, volume, t]);
    }
    h.write(ctx, old);
    old
}

/// `AtelEventKickBGSep(header, fade)` (`0x008658d0`; callers
/// `0x00862220`, `AtelEventSetUp`): with `fade`, the queued separates
/// fade out first ([`atel_fade_out_queued_seps`]). Then every entry of the
/// event header's separates table that is a background separate (flags
/// bit 2) to start now (with `fade`: not flagged `0x80`; without: flagged
/// 4), not `0x300`, and not pending in `g_AtelSepLoadList` under the
/// same id, plays (`SndSepPlay(id, 0x3f, v)`): v the volume (0 with
/// flags `0x30`, else `0x3f` with the save event's `+0x28`, `0x7f`
/// without), 0 when flagged `0x1c00`; with a fade-in flag (`0x400` 0xf0,
/// `0x800` 0x78, `0x1000` 0x168 frames) its volume goes to the volume
/// (`SndSepVolumeTranslate(id, frames, volume)`).
pub fn atel_event_kick_bg_sep(h: &mut impl ffx_bridge::Host, hdr: u32, fade: u32) {
    let t = sep_table(h, hdr);
    let n = if t == 0 { 0 } else { h.read::<i32>(t) };
    if fade != 0 {
        atel_fade_out_queued_seps(h);
    }
    let loads = h.rebase(SEP_LOAD_COUNT);
    let list = h.rebase(SEP_LOAD_LIST);
    for i in 0..n.max(0).cast_unsigned() {
        let t = sep_table(h, hdr);
        let id = if t == 0 {
            0
        } else {
            h.read::<u32>(t + i * 8 + 4)
        };
        let t = sep_table(h, hdr);
        let flags = if t == 0 {
            0
        } else {
            h.read::<u32>(t + i * 8 + 8)
        };
        if flags & 2 == 0 {
            continue;
        }
        let on = if fade != 0 {
            flags & 0x80 == 0
        } else {
            flags & 4 != 0
        };
        if flags & 0x300 != 0 || !on {
            continue;
        }
        let volume = if flags & 0x30 != 0 {
            0
        } else {
            let s = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
            if h.read::<u8>(s + 0x28) == 0 {
                0x7f
            } else {
                0x3f
            }
        };
        let start = if flags & 0x1c00 == 0 { volume } else { 0 };
        let mut k = 0;
        let mut pending = false;
        while k < h.read::<i32>(loads) {
            let e = list + k.cast_unsigned() * 8;
            if h.read::<u32>(e + 4) & 0x8000 != 0 && h.read::<u32>(e) == id {
                pending = true;
                break;
            }
            k += 1;
        }
        if pending {
            continue;
        }
        host_call(h, 0x0088_6e50, &[id, 0x3f, start]);
        let mut frames = 0;
        if flags & 0x400 != 0 {
            frames = 0xf0;
        }
        if flags & 0x800 != 0 {
            frames = 0x78;
        }
        if flags & 0x1000 != 0 {
            frames = 0x168;
        }
        if frames != 0 {
            host_call(h, 0x0088_6ff0, &[id, frames, volume]);
        }
    }
}

/// The position block of worker `w` by its kind (`*w`'s first byte: 5 or
/// 6 at `+0x284`, others at `+0x558`; 4 has none, 0).
fn worker_pos(h: &impl Mem, w: u32) -> u32 {
    match h.read::<u8>(h.read::<u32>(w)) {
        4 => 0,
        5 | 6 => w + 0x284,
        _ => w + 0x558,
    }
}

/// `Atel_SetWorkerPosXZ_00870970(worker, x, z, keep)` (name ours; callers
/// `Native_0012`, `Native_0125`): the worker's position block (by its
/// kind) = `(x, 0, z)`; a bound character (`+0xaa` 1, `+0x9c`) moves
/// there (`Ch_SetPos`) and its cloth resets. When the worker is the
/// current ctrl work's player (`+0x2e` = ctx `+0xa`), the ctx's camera
/// target (`+0x218..+0x224`) moves to `+0x228`, the position (w 1) takes
/// its place, and back to `+0x228` too unless the worker has flag `0x10`
/// at `+0x34` and `keep` is 0; the save event's encounter byte (`+0x10`)
/// and dictionary index follow the character (0 without one). Unless the
/// same flag and `keep` = 0, the position is also the block's start
/// (`+0x10`). Then the ctx's `+2` bit `0x10`.
pub fn atel_set_worker_pos_xz(h: &mut impl ffx_bridge::Host, w: u32, x: u32, z: u32, keep: u32) {
    let p = worker_pos(h, w);
    put_f32(h, p, x);
    h.write(p + 4, 0_u32);
    put_f32(h, p + 8, z);
    let c = h.read::<u32>(w + 0x9c);
    if h.read::<u8>(w + 0xaa) == 1 && c != 0 {
        let (x, z) = (x87::store(x87::load(x)), x87::store(x87::load(z)));
        host_call(h, CH_SET_POS, &[c, x, z]);
        let c = h.read::<u32>(w + 0x9c);
        host_call(h, CH_CLOTH_RESET, &[c]);
    }
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    if h.read::<u16>(w + 0x2e) == h.read::<u16>(ctx + 0xa) {
        let p = worker_pos(h, w);
        for k in 0..4 {
            copy_f32(h, ctx + 0x218 + k * 4, ctx + 0x228 + k * 4);
        }
        for k in 0..3 {
            copy_f32(h, p + k * 4, ctx + 0x218 + k * 4);
        }
        h.write(ctx + 0x224, 1f32.to_bits());
        if h.read::<u8>(w + 0x34) & 0x10 == 0 || keep != 0 {
            for k in 0..3 {
                copy_f32(h, ctx + 0x218 + k * 4, ctx + 0x228 + k * 4);
            }
            h.write(ctx + 0x234, 1f32.to_bits());
        }
        let c = h.read::<u32>(w + 0x9c);
        if c == 0 {
            let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
            h.write(save + 0x10, 0_u8);
            atel_set_save_dic_index(h, 0);
        } else {
            let enc = host_call(h, CH_ID_GET_ENCOUNT, &[c]);
            let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
            h.write(save + 0x10, enc.to_le_bytes()[0]);
            let c = h.read::<u32>(w + 0x9c);
            let dic = host_call(h, CH_ID_GET_DICTIONARY, &[c]);
            atel_set_save_dic_index(h, dic);
        }
    }
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    if h.read::<u8>(w + 0x34) & 0x10 == 0 || keep != 0 {
        for k in 0..3 {
            copy_f32(h, p + k * 4, p + 0x10 + k * 4);
        }
    }
    let f = h.read::<u8>(ctx + 2);
    h.write(ctx + 2, f | 0x10);
}

/// `Ch_SetPos(chr, x, z)` (`0x0082b440`).
const CH_SET_POS: u32 = 0x0082_b440;

/// `AtelExecCtrlEventInit_008677a0(ctrl, battle)`: the event start of a
/// ctrl work. With `battle`, `MsBattleExeClear`. The work's skip bit
/// (`+0` bit 4) is set when the pad's newly pressed buttons (mapped,
/// `Pad_MapButtons(0, 0, g_PadPressed)`; none from ctx `+8` = 2 on) hold
/// `0x20` and the windows are busy ([`atel_windows_busy`] = 1). Each
/// worker that has not finished its init (`+0x34` bit `0x10`) runs it
/// (`Script_Run(w, 0, 1)`, ctx `+4` = its index); the work's `+0` bit 8
/// says whether all have. Then every worker decodes its signals
/// (`AtelDecodeSignal(i, -1)`), `op_event_effect_init`,
/// [`atel_clear_pause_flag`](super::globals::atel_clear_pause_flag) and
/// `+0x210` bit `0x10`; or, when one is still busy, the init count
/// `+0x20c` goes up. The debug strings (`AtelPs2DebugString`, a bare
/// `ret`, and the `sprintf`s into a local for it) are left out.
pub fn atel_exec_ctrl_event_init(h: &mut impl ffx_bridge::Host, c: u32, battle: u32) {
    if battle != 0 {
        host_call(h, 0x0078_1530, &[]);
    }
    let f = h.read::<u8>(c);
    h.write(c, f & 0xfb);
    let n = u32::from(h.read::<u16>(c + 0xc));
    let pad = if h.read::<i32>(h.rebase(SCRIPT_CONTEXT + 8)) < 2 {
        let pressed = u32::from(h.read::<u16>(h.rebase(0x0132_70d0)));
        host_call(h, 0x0088_8c40, &[0, 0, pressed]) & 0xffff
    } else {
        0
    };
    if pad & 0x20 != 0 && atel_windows_busy(h) == 1 {
        let f = h.read::<u8>(c);
        h.write(c, f | 4);
    }
    let index = h.rebase(SCRIPT_CONTEXT + 4);
    let mut done = true;
    for i in 0..n {
        h.write(index, i);
        let k = h.read::<i32>(index);
        let w = super::atel_get_basic_work(h, k);
        if h.read::<u8>(w + 0x34) & 0x10 == 0 {
            host_call(h, 0x0086_41e0, &[w, 0, 1]);
        }
        done &= h.read::<u8>(w + 0x34) & 0x10 != 0;
    }
    let f = h.read::<u8>(c);
    h.write(c, (f & !8) | (u8::from(done) << 3));
    if done {
        for i in 0..n {
            host_call(h, 0x0086_53f0, &[i, u32::MAX]);
        }
        host_call(h, 0x007f_cc20, &[]);
        super::globals::atel_clear_pause_flag(h);
        let f = h.read::<u32>(c + 0x210);
        h.write(c + 0x210, f | 0x10);
    } else {
        let k = h.read::<u32>(c + 0x20c);
        h.write(c + 0x20c, k.wrapping_add(1));
    }
}

/// `AtelEventSetUpForBattleMusic(…)` (`0x00866240`; callers
/// `MsBattleExe`, `Native_c024`; their argument unread): the event set-up
/// for a battle. No voice on standby; outside an encounter
/// (`MsGetEncountType` 0) the battle-end status takes the save event's
/// `+0xe4` over its bit 16; the music-keep byte `0x01326bdd` = status bit
/// 10 clear; the `"MUSI"` callback `0x00862470` queued; vibration off on
/// pads 1-3; the foot status pushed and cleared; no overlay;
/// `yiPushRenderState`. With ctrl work 0 current, each worker loses its
/// `+0x34` bit 4 and keeps its character's field and gravity modes
/// (`+0xac`, `+0xb0`) and swap work (`+0xa4`; `+0x38` bit 8 sets `+0x33`
/// bit `0x80`); a bound worker of type `0x5001` hides its four effects
/// (`pppFpIDdispOff(+0xb18 + 16k & 0x7fff)`); ctrl work 0 gets `+3` bit
/// `0x80`. Back on the old ctrl work, the UC goes off ([`atel_uc_off`]),
/// and on again ([`atel_uc_on`]) when the old work had `+0` bits `0x22`
/// and neither `+1` bit 4 nor `+3` bits `0x84` of the current one are
/// set; the old work's `+0` bit 2 comes back. Last, ctrl work 0 current,
/// an open guide map (`+1` bit `0x80`) closes (`Yn_GuideMapEnd`).
pub fn atel_event_set_up_for_battle_music(h: &mut impl ffx_bridge::Host) {
    host_call(h, 0x0088_7b50, &[0]);
    let status = h.rebase(0x0132_6b48);
    let s = if host_call(h, 0x0078_2a10, &[]) == 0 {
        let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
        let s = (h.read::<u32>(status) & 0xfffe_ffff) | h.read::<u32>(save + 0xe4);
        h.write(status, s);
        s
    } else {
        h.read::<u32>(status)
    };
    h.write(h.rebase(0x0132_6bdd), u8::from((s >> 10) & 1 == 0));
    let f = h.rebase(0x0086_2470);
    let name = h.rebase(0x00b5_bdf4);
    atel_regist_callback_que2(h, 0, f, name);
    for pad in 1..4 {
        host_call(h, 0x0087_5f60, &[pad, 0]);
    }
    host_call(h, 0x0088_6c40, &[]);
    host_call(h, 0x0088_5e50, &[]);
    h.write(h.rebase(0x0132_6b80), 0_u8);
    host_call(h, 0x0090_9c60, &[]);
    let works = h.rebase(CTRL_WORKS);
    let n = h.read::<u16>(works + 0xc);
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let old = h.read::<u32>(ctx);
    h.write(ctx, works);
    for i in 0..i32::from(n) {
        let w = super::atel_get_basic_work(h, i);
        let f = h.read::<u16>(w + 0x34);
        h.write(w + 0x34, f & 0xfffb);
        let c = h.read::<u32>(w + 0x9c);
        if c == 0 {
            continue;
        }
        let mode = host_call(h, 0x0082_ab90, &[c]);
        h.write(w + 0xac, mode.to_le_bytes()[0]);
        let c = h.read::<u32>(w + 0x9c);
        let grav = host_call(h, 0x0082_abc0, &[c]);
        h.write(w + 0xb0, grav.to_le_bytes()[0]);
        let c = h.read::<u32>(w + 0x9c);
        let swap = host_call(h, 0x0082_8b30, &[c]);
        let bit = h.read::<u8>(w + 0x38) & 8 != 0;
        h.write(w + 0xa4, swap);
        if bit {
            let f = h.read::<u8>(w + 0x33);
            h.write(w + 0x33, f | 0x80);
        }
    }
    for i in 0..i32::from(n) {
        let w = super::atel_get_basic_work(h, i);
        if h.read::<u32>(w + 0x9c) == 0 || h.read::<u16>(w + 0xa8) != 0x5001 {
            continue;
        }
        for k in 0..4 {
            let id = u32::from(h.read::<u16>(w + FP_SLOTS + k * 0x10)) & 0x7fff;
            if id != 0 {
                host_call(h, 0x0072_9570, &[id]);
            }
        }
    }
    let c = h.read::<u32>(ctx);
    let f = h.read::<u8>(c + 3);
    h.write(c + 3, f | 0x80);
    h.write(ctx, old);
    let b = h.read::<u8>(old);
    let keep = b & 2;
    let on = b & 0x22 != 0;
    atel_uc_off(h);
    let mut cur = h.read::<u32>(ctx);
    if h.read::<u8>(cur + 1) & 4 == 0 && h.read::<u8>(cur + 3) & 0x84 == 0 && on {
        atel_uc_on(h);
        cur = h.read::<u32>(ctx);
    }
    h.write(ctx, works);
    let f = h.read::<u8>(cur);
    h.write(cur, (f & !2) | keep);
    let g = h.read::<u8>(works + 1);
    if g & 0x80 != 0 {
        h.write(works + 1, g & 0x7f);
        host_call(h, 0x0091_9c60, &[]);
    }
    h.write(ctx, cur);
}

/// `AtelEventRegistFootInfo(header)` (`0x00865b80`; callers
/// `AtelBattleEndCallback`, `AtelEventSetUp`): the footstep separates of
/// the event header (`+0x2c` sub-header, its tables `+0x1c`, `+0x20`,
/// `+0x30`, `+0x34`: 16 `u16` offsets each, to a count and that many
/// 16-byte entries of four separates) go to `SndSetFootSep(kind, party,
/// variation, sep)`, with kinds (0, 1, 4, 5), (2, 3, 6, 7), (8, 9, 4, 0xd)
/// and (0xa, 0xb, 0xe, 0xf); kind 4 only for variation 0. Returns the
/// header.
pub fn atel_event_regist_foot_info(h: &mut impl ffx_bridge::Host, hdr: u32) -> u32 {
    const TABLES: [(u32, [u32; 4]); 4] = [
        (0x1c, [0, 1, 4, 5]),
        (0x20, [2, 3, 6, 7]),
        (0x30, [8, 9, 4, 0xd]),
        (0x34, [0xa, 0xb, 0xe, 0xf]),
    ];
    for (off, kinds) in TABLES {
        for party in 0..16u32 {
            let sub = h.read::<u32>(hdr + 0x2c);
            let sub = if sub == 0 { 0 } else { sub.wrapping_add(hdr) };
            if sub == 0 {
                continue;
            }
            let t = h.read::<u32>(sub + off);
            if t == 0 {
                continue;
            }
            let t = t.wrapping_add(hdr);
            let e = u32::from(h.read::<u16>(t + party * 2));
            let p = e.wrapping_add(t);
            if e == 0 || p == 0 {
                continue;
            }
            let n = h.read::<i32>(p);
            for v in 0..n.max(0).cast_unsigned() {
                let entry = p + 4 + v * 0x10;
                for (k, &kind) in (0u32..).zip(&kinds) {
                    if kind == 4 && v != 0 {
                        continue;
                    }
                    let sep = h.read::<u32>(entry + k * 4);
                    host_call(h, 0x0088_70e0, &[kind, party, v, sep]);
                }
            }
        }
    }
    hdr
}

/// `AtelAllClearCallbackQue()` (`0x008b3cf0`; caller `AtelInitTotal`):
/// both card slots' load positions reset
/// ([`tk_menu_save_load_clear_load_pos`] 0, 1). Returns the last one's
/// `EAX`.
pub fn atel_all_clear_callback_que(h: &mut impl ffx_bridge::Host) -> u32 {
    tk_menu_save_load_clear_load_pos(h, 0);
    tk_menu_save_load_clear_load_pos(h, 1)
}

/// `AtelExecVibrate(update)` (`0x008739a0`; caller `0x008a9ca0`): with
/// `update` bit 0 the volumes are updated first
/// ([`vibrate_update_volume`]); then pad 0's two actuators get twice the
/// two vibration volumes (`TkGetVibrateVolume(&a, &b)` into `frame`, 8
/// bytes: `Pad_SetActuatorFF(0, 0, 2a)`, `Pad_SetActuatorFE(0, 0, 2b)`).
/// Returns the last call's `EAX`.
pub fn atel_exec_vibrate(h: &mut impl ffx_bridge::Host, update: u32, frame: u32) -> u32 {
    if update & 1 != 0 {
        vibrate_update_volume(h);
    }
    host_call(h, 0x0087_5e10, &[frame, frame + 4]);
    let a = h.read::<u32>(frame);
    host_call(h, 0x0088_9490, &[0, 0, a.wrapping_add(a)]);
    let b = h.read::<u32>(frame + 4);
    host_call(h, 0x0088_94b0, &[0, 0, b.wrapping_add(b)])
}

/// `Atel_SetChrAltParts_008720e0(chr, on)` (name ours; caller
/// `Native_7080`): with a character, `on` hides its model part 1 and shows
/// part 2 (`Ch_ModelSetHide(chr, 1, 1)`, `(chr, 2, 0)`), else the other
/// way round.
pub fn atel_set_chr_alt_parts(h: &mut impl ffx_bridge::Host, chr: u32, on: u32) {
    if chr == 0 {
        return;
    }
    let on = u32::from(on != 0);
    host_call(h, 0x0082_aea0, &[chr, 1, on]);
    host_call(h, 0x0082_aea0, &[chr, 2, on ^ 1]);
}

/// `Atel_ApplyChrDir_00870360(worker)` (name ours; caller `0x008666e0`):
/// a worker bound to its character (`+0xaa` 1, `+0x9c`) turns it to its
/// position block's direction (`+0x20`, `Ch_SetDir`).
pub fn atel_apply_chr_dir(h: &mut impl ffx_bridge::Host, w: u32) {
    let p = pos_block(h, w);
    let c = h.read::<u32>(w + 0x9c);
    if h.read::<u8>(w + 0xaa) == 1 && c != 0 {
        let d = x87::store(x87::load(h.read(p + 0x20)));
        host_call(h, 0x0082_b190, &[c, d]);
    }
}

/// `Atel_SetSpriteScale_00874a80(i, x, y)` (name ours; caller
/// `Native_SpriteSetScale`): event sprite i (clamped to 0..=0x7f) gets
/// the scale `(x*256, y*256)` as `u16` (`ftol2_sse`).
pub fn atel_set_sprite_scale(m: &mut impl Mem, i: u32, x: u32, y: u32) {
    let i = i.cast_signed().clamp(0, 0x7f).cast_unsigned();
    let s = m.rebase(EVENT_SPRITES) + i * 0x1c;
    let sx = x87::ftol(m, x87::mul(x87::load(x), 256.0));
    m.write(s + 8, lo16(sx));
    let sy = x87::ftol(m, x87::mul(256.0, x87::load(y)));
    m.write(s + 0xa, lo16(sy));
}

/// `Atel_SyncChrRot_00869f90(worker)` (name ours; caller `0x008666e0`):
/// a worker bound to its character takes the character's rotation,
/// wrapped (`Math_WrapAngle(Ch_GetRot(chr))`, each through `fstp m32`),
/// into its position block's `+0x34`. `fcall(m, f, args)` calls the
/// original at `f` (an `ffx.sym` address) and returns its `ST0`.
pub fn atel_sync_chr_rot<M: Mem>(
    m: &mut M,
    w: u32,
    mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
) {
    let p = pos_block(m, w);
    let c = m.read::<u32>(w + 0x9c);
    if m.read::<u8>(w + 0xaa) == 1 && c != 0 {
        let r = x87::store(fcall(m, 0x0082_ace0, &[c]));
        let r = x87::store(fcall(m, 0x0088_bc20, &[r]));
        m.write(p + 0x34, r);
    }
}

/// The low 16 bits of `v` (a `mov word` of a 32-bit register).
fn lo16(v: u32) -> u16 {
    let [a, b, _, _] = v.to_le_bytes();
    u16::from_le_bytes([a, b])
}

/// The event's saved lights (`0x01326d00`): four colours, then three
/// directions, 16 bytes each.
const SAVED_LIGHTS: u32 = 0x0132_6d00;

/// `Atel_SaveLights_0086db20()` (name ours; caller
/// `AtelEventRemoveForBattle`): the four light colours
/// (`Sg_LightGetColor(k, &saved[k])`) and three directions
/// (`Sg_LightGetDir`) kept for the battle's end. Returns the last call's
/// `EAX`.
pub fn atel_save_lights(h: &mut impl ffx_bridge::Host) -> u32 {
    let at = h.rebase(SAVED_LIGHTS);
    for k in 0..4 {
        host_call(h, 0x0083_60f0, &[k, at + k * 0x10]);
    }
    let mut r = 0;
    for k in 0..3 {
        r = host_call(h, 0x0083_61c0, &[k, at + 0x40 + k * 0x10]);
    }
    r
}

/// `Atel_RestoreLights_0086e770()` (name ours; caller
/// `AtelBattleEndCallback`): the lights [`atel_save_lights`] kept, back
/// (`Sg_LightSetColor`, `Sg_LightSetDir`). Returns the last call's `EAX`.
pub fn atel_restore_lights(h: &mut impl ffx_bridge::Host) -> u32 {
    let at = h.rebase(SAVED_LIGHTS);
    for k in 0..4 {
        host_call(h, 0x0083_6310, &[k, at + k * 0x10]);
    }
    let mut r = 0;
    for k in 0..3 {
        r = host_call(h, 0x0083_6430, &[k, at + 0x40 + k * 0x10]);
    }
    r
}

/// `Atel_ResetTextLanguage_0086ece0()` (name ours; caller
/// `AtelJumpGameOver`): the text language back to the build's
/// (`MsSetSaveConfigEnglish(FFXGetAsiaMode != 0)`); when that changed it,
/// the international data swaps (`TOSwapInternationalEnvInit(english)`,
/// then `TOSwapInternationalEnvExec` until it returns non-zero, with
/// `TkSyncV(0)` and `FileReadMain` between tries).
pub fn atel_reset_text_language(h: &mut impl ffx_bridge::Host) {
    let was = host_call(h, 0x0078_51d0, &[]);
    let asia = host_call(h, 0x0078_26a0, &[]);
    host_call(h, 0x0078_5ed0, &[u32::from(asia != 0)]);
    if host_call(h, 0x0078_51d0, &[]) == was {
        return;
    }
    let english = host_call(h, 0x0078_51d0, &[]);
    host_call(h, 0x008b_0920, &[english]);
    if host_call(h, 0x008b_0340, &[]) != 0 {
        return;
    }
    loop {
        host_call(h, 0x0088_7f00, &[0]);
        host_call(h, 0x0076_cfd0, &[]);
        if host_call(h, 0x008b_0340, &[]) != 0 {
            return;
        }
    }
}

/// `Atel_StepSpriteAnim_00874bf0(sprite)` (name ours; caller
/// `0x008743e0`): an event sprite's animation (`get_ptr32_func(+0x14)`:
/// frame count `i16` at `+4`, 8-byte frames with their length `i16` at
/// `+0x12` and flags at `+0x14`) advances its timer (`+0x18`) by `0x100`;
/// past the frame's length the next frame starts (`+0xc`, timer 0), and
/// past the last frame the first, the sprite turning off (`+0` = 0)
/// unless the last frame loops (flag `0x80`).
pub fn atel_step_sprite_anim(h: &mut impl ffx_bridge::Host, s: u32) {
    let p = h.read::<u32>(s + 0x14);
    let data = host_call(h, 0x00a4_4650, &[p]);
    let f = u32::from(h.read::<u16>(s + 0xc));
    let t = h.read::<u32>(s + 0x18).wrapping_add(0x100);
    let frame = data.wrapping_add(f * 8);
    if t.cast_signed() < i32::from(h.read::<i16>(frame + 0x12)) {
        h.write(s + 0x18, t);
        return;
    }
    let mut next = f + 1;
    if next.cast_signed() >= i32::from(h.read::<i16>(data + 4)) {
        next = 0;
        if h.read::<u8>(frame + 0x14) & 0x80 == 0 {
            h.write(s, 0_u8);
        }
    }
    h.write(s + 0xc, lo16(next));
    h.write(s + 0x18, 0_u32);
}

/// `Atel_LoadEventTexList_00874b70(on, x, y)` (name ours; caller
/// `Native_011a`): `0x01327144` = on, `0x01327f48` = `x - 0xa0 + on`,
/// `0x023cbc5c` = y, `0x01327f4c` = 1; with `on` the event's texture list
/// loads (`0x00639310` of `getEventLocTexListPath`, into `frame`'s first
/// 256 bytes, its own local in the next) and `g_AtelEventTexListLoaded`
/// is set.
pub fn atel_load_event_tex_list(
    h: &mut impl ffx_bridge::Host,
    on: u32,
    x: u32,
    y: u32,
    frame: u32,
) {
    h.write(h.rebase(0x0132_7f48), x.wrapping_sub(0xa0).wrapping_add(on));
    h.write(h.rebase(0x0132_7144), on);
    h.write(h.rebase(0x023c_bc5c), y);
    h.write(h.rebase(0x0132_7f4c), 1_u32);
    if on != 0 {
        get_event_loc_tex_list_path(h, frame, frame + 0x100);
        host_call(h, 0x0063_9310, &[frame]);
        h.write(h.rebase(0x0132_7140), 1_u32);
    }
}

/// `AtelBGSepFade(volume, frames)` (`0x00861000`; callers
/// `MsBattleProcess_Status_007917d0`, `Native_0143`, `Native_0144`): the
/// background separates only ([`atel_translate_bg_seps`] with `all` 0).
pub fn atel_bg_sep_fade(h: &mut impl ffx_bridge::Host, volume: u32, frames: u32) -> u32 {
    atel_translate_bg_seps(h, volume, frames, 0)
}

/// `Atel_StepSprites_008743e0()` (name ours; caller `0x00872bd0`): with
/// the event sprites shown (`0x01327144`, `0x01327f4c`) and no movie
/// (`0x0086d940` 0), every active sprite's animation steps
/// ([`atel_step_sprite_anim`]).
pub fn atel_step_sprites(h: &mut impl ffx_bridge::Host) {
    if h.read::<u32>(h.rebase(0x0132_7144)) == 0 || host_call(h, 0x0086_d940, &[]) != 0 {
        return;
    }
    if h.read::<u32>(h.rebase(0x0132_7f4c)) == 0 {
        return;
    }
    let base = h.rebase(EVENT_SPRITES);
    for i in 0..0x80 {
        let s = base + i * 0x1c;
        if h.read::<u8>(s) != 0 {
            atel_step_sprite_anim(h, s);
        }
    }
}

/// `AtelDrawSemiTrans()` (`0x00872ca0`; caller `Sg_MainLoop`): with ctrl
/// work 0 running (`+0` bit 0) and no movie (`0x0086d940` 0), the guide
/// map is drawn (`Yn_GuideMapProcEx(show)`): show when the work's `+3`
/// has bit `0x20` without `0x10`, or `0x40`, and the save menu's first
/// byte has bit `0x40`; or `+3` has `8`; and never before the work's
/// `+0x1f4` reaches 3.
pub fn atel_draw_semi_trans(h: &mut impl ffx_bridge::Host) {
    let c = super::atel_get_ctrl_work(h, 0);
    if h.read::<u8>(c) & 1 == 0 || host_call(h, 0x0086_d940, &[]) != 0 {
        return;
    }
    let f = h.read::<u8>(c + 3);
    let mut show = f & 0x20 != 0 && f & 0x10 == 0;
    if f & 0x40 != 0 {
        show = true;
    }
    let menu = host_call(h, 0x0078_5290, &[]);
    show &= h.read::<u8>(menu) & 0x40 != 0;
    if h.read::<u8>(c + 3) & 8 != 0 {
        show = true;
    }
    show &= h.read::<u32>(c + 0x1f4) >= 3;
    host_call(h, 0x0091_9d00, &[u32::from(show)]);
}

/// `Atel_SetPartyMask_008623b0()` (name ours; caller
/// `AtelBattleEndCallback`): the party mask `0x01326b7c` = the bits of the
/// three party members (`MsGetSavePartyMember(&a, &b, &c)` into `frame`,
/// 12 bytes; `0xff` for an empty slot, bits taken mod 32).
pub fn atel_set_party_mask(h: &mut impl ffx_bridge::Host, frame: u32) {
    let mask = h.rebase(0x0132_6b7c);
    h.write(mask, 0_u32);
    host_call(h, 0x0078_52f0, &[frame, frame + 4, frame + 8]);
    let mut m = h.read::<u32>(mask);
    for k in 0..3 {
        let v = h.read::<u32>(frame + k * 4);
        if v != 0xff {
            m |= 1u32.wrapping_shl(v);
            h.write(mask, m);
        }
    }
}

/// `Atel_RegistVibrateSignal_00876590(from, i, k)` (name ours; caller
/// `0x00867c10`): with ctrl work 0 current, worker `i`'s sound `k`
/// (`AtelSoundTableGet_00876860(type, k)`) and vibration curve
/// (`TkGetVibrateCurve(type, k, +0x44)`); when both exist and the
/// worker's `+0xab` has bit `k` (of the low byte), a signal from `from`
/// (`AtelRegistSignalWithoutSameTag(from, i, 0, sound, curve)`). The ctrl
/// work back. Returns the signal's result, 0 without one.
pub fn atel_regist_vibrate_signal(h: &mut impl ffx_bridge::Host, from: u32, i: u32, k: u32) -> u32 {
    vibrate_signal(h, 0x0086_ec40, from, i, k, None)
}

/// `Native_DrawBarSegment_00855a30(pos, seg)` (name ours; caller
/// `caseD_8011`): one segment of a bar: its width is `table(seg[4]) *
/// 0x013259f0 / 0x013259e8` (`ReturnTableU32_00820580`, C division; 0
/// where the original divides by zero), less the cursor `0x013259f4` for
/// kind 0 (other kinds add the table value to `0x013259f8`); drawn in
/// the segment's colour (`Gu_SetRgbaq(seg[5..8], 0xff)`) from the cursor
/// to cursor + width (`Gu_SetXyzf2`, at `pos` (`i16` x, y) and `pos[3]`
/// below), and the cursor moves on. Returns the width.
pub fn native_draw_bar_segment(h: &mut impl ffx_bridge::Host, pos: u32, seg: u32) -> u32 {
    let kind = h.read::<u8>(seg + 4);
    let k = host_call(h, 0x0082_0580, &[u32::from(kind)]).cast_signed();
    let num = h.read::<i32>(h.rebase(0x0132_59f0)).wrapping_mul(k);
    let den = h.read::<i32>(h.rebase(0x0132_59e8));
    let mut width = if den == 0 { 0 } else { num.wrapping_div(den) };
    let cursor = h.rebase(0x0132_59f4);
    if h.read::<u8>(seg + 4) == 0 {
        width = width.wrapping_sub(h.read::<i32>(cursor));
    } else {
        let at = h.rebase(0x0132_59f8);
        let v = h.read::<i32>(at).wrapping_add(k);
        h.write(at, v);
    }
    let rgb = [5, 6, 7].map(|o| u32::from(h.read::<u8>(seg + o)));
    host_call(h, 0x0084_84c0, &[rgb[0], rgb[1], rgb[2], 0xff]);
    let x = i32::from(h.read::<i16>(pos));
    let y = i32::from(h.read::<i16>(pos + 2));
    let at = x.wrapping_add(h.read::<i32>(cursor));
    host_call(h, 0x0084_8680, &[at.cast_unsigned(), y.cast_unsigned()]);
    let y = i32::from(h.read::<i16>(pos + 2));
    let bottom = i32::from(h.read::<i16>(pos + 6)).wrapping_add(y);
    let next = h.read::<i32>(cursor).wrapping_add(width);
    let x = i32::from(h.read::<i16>(pos)).wrapping_add(next);
    h.write(cursor, next);
    host_call(h, 0x0084_8680, &[x.cast_unsigned(), bottom.cast_unsigned()]);
    width.cast_unsigned()
}

/// `Atel_BattleEndMapStart_008615b0(map)` (name ours; caller
/// `AtelBattleEndCallback`): back on another map than the event jump's
/// (`TkGetNowEventJumpMapNo`), `0x01325b88` = 1 and the callback
/// `0x00861650` queued as `"MAPR"`. On the same map, `0x01325b88` = 0, the
/// render state popped, the map id page (`TkGetNowEventJumpMapIdNo`),
/// guide map number (`0x01326b68`), FP state (`pppFpLoadStat(0x01326be0)`)
/// and draw mask (`0x01326b78`) restored; with ctrl work 0 current a
/// guide map not open yet opens (`+1` bit `0x80`, `Yn_GuideMapInit`);
/// then `Yn_BattleEnd`. Returns the last call's `EAX`.
pub fn atel_battle_end_map_start(h: &mut impl ffx_bridge::Host, map: u32) -> u32 {
    let flag = h.rebase(0x0132_5b88);
    if map != host_call(h, 0x0088_d690, &[]) {
        h.write(flag, 1_u32);
        let f = h.rebase(0x0086_1650);
        let name = h.rebase(0x00b5_be0c);
        return atel_regist_callback_que2(h, 0, f, name);
    }
    h.write(flag, 0_u32);
    host_call(h, 0x0090_9b90, &[]);
    let page = host_call(h, 0x0088_d680, &[]);
    host_call(h, 0x0087_3e60, &[page]);
    let no = h.read::<u32>(h.rebase(0x0132_6b68));
    host_call(h, 0x0091_9da0, &[no]);
    let stat = h.rebase(0x0132_6be0);
    host_call(h, 0x0072_9a10, &[stat]);
    let mask = h.read::<u32>(h.rebase(0x0132_6b78));
    host_call(h, 0x0090_a2b0, &[mask]);
    let works = h.rebase(CTRL_WORKS);
    let g = h.read::<u8>(works + 1);
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let old = h.read::<u32>(ctx);
    h.write(ctx, works);
    if g & 0x80 == 0 {
        h.write(works + 1, g | 0x80);
        host_call(h, 0x0091_9cd0, &[]);
    }
    h.write(ctx, old);
    host_call(h, 0x0091_9a40, &[])
}

/// `Atel_SetMovieTexture_00873eb0(id, flags)` (name ours; caller
/// `Native_01b0`): by the save event's movie state (`+0xe0`): outside 2..=5
/// the movie texture is set up (`graphicSetTextureVideoInfo(id, flags &
/// 0x80)`) and the state becomes 1 with the id and flags (`+0xd8`,
/// `+0xdc`); inside, another id waits for its file
/// (`AtelSetOnMemoryMovieFileWait(id, flags)`), and the same id with new
/// flags sets state 3 and the flags. Returns the wait's `EAX`, else the
/// save event's address.
pub fn atel_set_movie_texture(h: &mut impl ffx_bridge::Host, id: u32, flags: u32) -> u32 {
    let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
    if h.read::<u32>(save + 0xe0).wrapping_sub(2) > 3 {
        host_call(h, 0x0064_3aa0, &[id, flags & 0x80]);
        let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
        h.write(save + 0xe0, 1_u32);
        h.write(save + 0xd8, id);
        h.write(save + 0xdc, flags);
        return save;
    }
    if id != h.read::<u32>(save + 0xd8) {
        return host_call(h, 0x0087_3f70, &[id, flags]);
    }
    if flags != h.read::<u32>(save + 0xdc) {
        h.write(save + 0xe0, 3_u32);
        h.write(save + 0xdc, flags);
    }
    save
}

/// What [`atel_regist_vibrate_signal`] and
/// [`atel_regist_vibrate_signal_with`] share: with ctrl work 0 current,
/// worker `i`'s sound `k` and vibration curve (`curve`, or for a negative
/// one `TkGetVibrateCurve(type, k, +0x44)`); with both and the worker's
/// `+0xab` bit `k`, the signal `regist(from, i, 0, sound, curve)`.
fn vibrate_signal(
    h: &mut impl ffx_bridge::Host,
    regist: u32,
    from: u32,
    i: u32,
    k: u32,
    curve: Option<u32>,
) -> u32 {
    let old = super::atel_change_current_ctrl(h, 0);
    let w = super::atel_get_basic_work(h, i.cast_signed());
    let t = super::atel_get_actor_type(h, i.cast_signed());
    let snd = atel_sound_table_get(h, t, k);
    let curve = match curve {
        Some(c) if c.cast_signed() >= 0 => c,
        _ => {
            let id = h.read::<u32>(w + 0x44);
            host_call(h, 0x0087_63e0, &[t, k, id])
        }
    };
    let mut r = 0;
    let bit = 1u32.wrapping_shl(k).to_le_bytes()[0];
    if snd != 0xff && curve != 0xffff && h.read::<u8>(w + 0xab) & bit != 0 {
        r = host_call(h, regist, &[from, i, 0, snd, curve]);
    }
    super::atel_set_current_ctrl(h, old);
    r
}

/// `Atel_RegistVibrateSignalWith_008766d0(from, i, k, curve)` (name ours;
/// caller `0x008680a0`): [`atel_regist_vibrate_signal`] with a given
/// vibration curve (the worker's own for a negative one) and
/// `AtelRegistSignal`.
pub fn atel_regist_vibrate_signal_with(
    h: &mut impl ffx_bridge::Host,
    from: u32,
    i: u32,
    k: u32,
    curve: u32,
) -> u32 {
    vibrate_signal(h, 0x0086_e990, from, i, k, Some(curve))
}

/// `AtelSetSndCameraPos()` (`0x00877470`; caller `AtelExecSound`): the
/// camera matrix copied (`MsCopyCameraMatrix` into `0x013282c8`), the
/// camera-matrix flag cleared (`g_AtelCameraMatrixFlag` = -1), and the
/// sound listener matrix (`0x01328288`) made: with ctrl work 0's player
/// worker (`+0xa` not `0xffff`), the inverse of the camera matrix with
/// the player's position as translation, inverted again
/// (`FFXVu0InversMatrix`, through `frame`, 64 bytes); without one, the
/// camera matrix itself (`FFXVu0CopyMatrix`).
pub fn atel_set_snd_camera_pos(h: &mut impl ffx_bridge::Host, frame: u32) {
    let cam = h.rebase(0x0132_82c8);
    let listener = h.rebase(0x0132_8288);
    host_call(h, 0x007c_0070, &[cam]);
    h.write(h.rebase(0x0132_8308), u32::MAX);
    let old = super::atel_change_current_ctrl(h, 0);
    let c = super::atel_get_cur_ctrl_work(h);
    let player = h.read::<u16>(c + 0xa);
    if player == 0xffff {
        host_call(h, 0x006e_d360, &[listener, cam]);
    } else {
        host_call(h, 0x006e_d450, &[frame, cam]);
        let player = h.read::<u16>(c + 0xa);
        let w = super::atel_get_basic_work(h, i32::from(player));
        let p = pos_block(h, w);
        for k in 0..3 {
            copy_f32(h, p + k * 4, frame + 0x30 + k * 4);
        }
        host_call(h, 0x006e_d450, &[listener, frame]);
    }
    super::atel_set_current_ctrl(h, old);
}

/// `Atel_TurnToPoint_0086f7f0(worker, x, y, z)` (name ours; caller
/// `0x0086f8a0`): the yaw and pitch from the worker's position to `(x, y,
/// z)` (`Math_YawPitchTo(&yaw, &pitch, pos, point)`, the two results in
/// `frame`, 8 bytes; `yaw_pitch(h, args)` calls it with its eight words)
/// into its current level's turn block (`+0x174` of level `+0x32`, at
/// most 1 for types 5 and 6, 8 otherwise): `+0x20` yaw, `+0x24` pitch.
pub fn atel_turn_to_point<H: ffx_bridge::Host>(
    h: &mut H,
    w: u32,
    point: [u32; 3],
    frame: u32,
    yaw_pitch: impl FnOnce(&mut H, &[u32]) -> u32,
) {
    let level = u32::from(h.read::<u8>(w + 0x32));
    let kind = h.read::<u8>(h.read::<u32>(w));
    let level = if matches!(kind, 5 | 6) {
        level.min(1)
    } else {
        level.min(8)
    };
    let turn = h.read::<u32>(w + level * 0x4c + 0x174);
    let p = pos_block(h, w);
    let mut args = [frame, frame + 4, 0, 0, 0, 0, 0, 0];
    for k in 0..3 {
        args[2 + k] = x87::store(x87::load(h.read(p + 4 * u32::try_from(k).unwrap_or(0))));
        args[5 + k] = x87::store(x87::load(point[k]));
    }
    yaw_pitch(h, &args);
    copy_f32(h, frame, turn + 0x20);
    copy_f32(h, frame + 4, turn + 0x24);
}

/// `Atel_ShowSprite_008748a0(i, tex)` (name ours; caller
/// `Native_SpriteShow`): event sprite i (clamped to 0..=0x7f) shows
/// texture `tex` (`op_anm_tex0_set(tex, 0x1a00, 0x1b00)`, its data
/// `set_ptr32(tex)` at `+0x14`): `+0` = `0xff01` (on, alpha `0xff`), `+3`
/// = 1, colour `0x80808080`, frame and timer 0. With `0x023cbc5c` set and
/// the texture list not loaded, it loads (as in
/// [`atel_load_event_tex_list`]).
pub fn atel_show_sprite(h: &mut impl ffx_bridge::Host, i: u32, tex: u32, frame: u32) {
    let i = i.cast_signed().clamp(0, 0x7f).cast_unsigned();
    let s = h.rebase(EVENT_SPRITES) + i * 0x1c;
    host_call(h, 0x007e_cf50, &[tex, 0x1a00, 0x1b00]);
    let data = host_call(h, 0x00a4_4700, &[tex]);
    h.write(s + 0x14, data);
    h.write(s, 0xff01_u16);
    h.write(s + 0x10, 0x8080_8080_u32);
    h.write(s + 3, 1_u8);
    h.write(s + 0x18, 0_u32);
    h.write(s + 0xc, 0_u16);
    let loaded = h.rebase(0x0132_7140);
    if h.read::<u32>(h.rebase(0x023c_bc5c)) != 0 && h.read::<u32>(loaded) == 0 {
        get_event_loc_tex_list_path(h, frame, frame + 0x100);
        host_call(h, 0x0063_9310, &[frame]);
        h.write(loaded, 1_u32);
    }
}

/// `AtelUpdateWaterStatus()` (`0x00871e50`; caller `AtelDrawNoTrans`):
/// with ctrl work 0 current, the "in water" byte `0x01326b83`: outside a
/// battle the map's status bit 0 (`yiGetMapStatus`, when not negative)
/// goes to the save event's `+0x29`, and set it means 1; else, when ctrl
/// work 0 has water (`+2` bit 0), 1 when the camera
/// (`MsCameraGetPosNow(1, frame)`, 16 bytes) is not above the water level
/// (`+0x208`: level <= camera y), else 0. The ctrl work back.
pub fn atel_update_water_status(h: &mut impl ffx_bridge::Host, frame: u32) {
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let old = h.read::<u32>(ctx);
    h.write(ctx, h.rebase(CTRL_WORKS));
    let flag = h.rebase(0x0132_6b83);
    let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
    if host_call(h, 0x0078_0c90, &[]) == 0 {
        let st = host_call(h, 0x0090_8dd0, &[]);
        if st.cast_signed() >= 0 {
            h.write(save + 0x29, st.to_le_bytes()[0] & 1);
        }
        if h.read::<u8>(save + 0x29) != 0 {
            h.write(flag, 1_u8);
            h.write(ctx, old);
            return;
        }
    }
    let c = h.read::<u32>(ctx);
    if h.read::<u8>(c + 2) & 1 == 0 {
        h.write(flag, 0_u8);
        h.write(ctx, old);
        return;
    }
    host_call(h, 0x007b_d470, &[1, frame]);
    let y = x87::load(h.read(frame + 4));
    let c = h.read::<u32>(ctx);
    let level = x87::load(h.read(c + 0x208));
    h.write(ctx, old);
    h.write(flag, u8::from(level <= y));
}

/// `AtelExec_Internal_871d10()` (`0x00871d70`; caller `0x008688f0`): the
/// pad words of the frames since the last sync (`TkGetLastSyncVCounter`
/// of them, history index `1 - n` up to 0) merged (`|`) into the words from `0x013270c0`: pad 1
/// and pad 0 held (`TkGetControlerHist`), repeat, repeat 2, newly pressed
/// (`g_PadPressed`) and trigger-repeat, each cleared first. (The original
/// tests a constant code address first; its else branch never runs.)
pub fn atel_exec_internal_871d10(h: &mut impl ffx_bridge::Host) {
    let n = host_call(h, 0x0088_7dc0, &[]);
    let words = h.rebase(0x0132_70c0);
    for off in [4u32, 0, 8, 0xc, 0x10, 0x14] {
        h.write(words + off, 0_u16);
    }
    let mut i = 1u32.wrapping_sub(n).cast_signed();
    while i <= 0 {
        let k = i.cast_unsigned();
        for (off, f, pad) in [
            (4u32, 0x0088_8d70, 1u32),
            (0, 0x0088_8d70, 0),
            (8, 0x0088_8e50, 0),
            (0xc, 0x0088_8e30, 0),
            (0x10, 0x0088_8e80, 0),
            (0x14, 0x0088_8ea0, 0),
        ] {
            let v = lo16(host_call(h, f, &[pad, 0, k]));
            let w = h.read::<u16>(words + off);
            h.write(words + off, w | v);
        }
        i += 1;
    }
}

/// `Atel_SetEventPathLang_00879430(to)` (name ours; caller
/// `AtelUpdateFilePath`): the event path's language replacement
/// (`g_AtelEventPathChg`, `0x013285b4`) cleared (`clear_read_chgname`) and
/// set again for the text language (`TkGetLang`: 1 us, 2 fr, 3 es, 4 de,
/// 5 it, 18 uk, any other jp; `set_read_chgname("/ffx/proj/event/xx/",
/// to)`).
pub fn atel_set_event_path_lang(h: &mut impl ffx_bridge::Host, to: u32) {
    let cell = h.rebase(0x0132_85b4);
    let old = h.read::<u32>(cell);
    host_call(h, 0x00a4_2f30, &[old]);
    let dir = match host_call(h, 0x0088_7d00, &[]) {
        1 => 0x00b5_c110,
        2 => 0x00b5_c138,
        3 => 0x00b5_c160,
        4 => 0x00b5_c124,
        5 => 0x00b5_c14c,
        18 => 0x00b5_c174,
        _ => 0x00b5_c188,
    };
    let from = h.rebase(dir);
    let r = host_call(h, 0x00a4_3220, &[from, to]);
    h.write(cell, r);
}

/// `Atel_AddLoveParam_00865650(i, skip)` (name ours; caller
/// `0x00867950`): in an event that counts affection (ctx `+2` bit `0x40`)
/// and without `skip`, a bound worker `i` (`+0xaa` 1, a model) whose
/// party member (`+0x40`, below 8, not disabled in
/// `g_AtelLoveParamDisable`) is a woman or a man (`MsGetSaveChrFemale`,
/// `MsGetSaveChrMan`) gains the event's affection (ctx `+4` or `+6`, `u16`)
/// in the save event's `+0x2c + 4*member`, once per event jump id (bit
/// `id` of the save event's words from `+0x4c`, 20 of them); ctx `+0x1e7`
/// = the member.
pub fn atel_add_love_param(h: &mut impl ffx_bridge::Host, i: u32, skip: u32) {
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let c = h.read::<u32>(ctx);
    if h.read::<u8>(c + 2) & 0x40 == 0 || skip != 0 {
        return;
    }
    let w = super::atel_get_basic_work(h, i.cast_signed());
    if h.read::<u8>(w + 0xaa) != 1 || h.read::<u32>(w + 0x9c) == 0 {
        return;
    }
    let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
    let w = super::atel_get_basic_work(h, i.cast_signed());
    let m = h.read::<u32>(w + 0x40);
    if m >= 8 || h.read::<u32>(h.rebase(0x0132_6b90)) & (1 << m) != 0 {
        return;
    }
    let mut add = 0;
    if host_call(h, 0x0078_4f60, &[m]) == 1 {
        let c = h.read::<u32>(ctx);
        add = u32::from(h.read::<u16>(c + 4));
    }
    if host_call(h, 0x0078_4f90, &[m]) == 1 {
        let c = h.read::<u32>(ctx);
        add = u32::from(h.read::<u16>(c + 6));
    }
    let id = host_call(h, 0x0088_d660, &[]).cast_signed();
    let word = id / 32;
    if word >= 20 {
        return;
    }
    let bit = 1u32.wrapping_shl((id % 32).cast_unsigned());
    let at = save
        .wrapping_add(word.cast_unsigned().wrapping_mul(4))
        .wrapping_add(0x4c);
    let v = h.read::<u32>(at);
    if v & bit != 0 {
        return;
    }
    let c = h.read::<u32>(ctx);
    h.write(at, v | bit);
    let love = save + 0x2c + m * 4;
    let l = h.read::<u32>(love);
    h.write(love, l.wrapping_add(add));
    h.write(c + 0x1e7, m.to_le_bytes()[0]);
}

/// `Atel_FindVarSlot_0087bc90(rec)` (name ours; caller `0x00884cc0`): for
/// a variable reference `rec` (`+2` worker, `+4` kind in the top 4 bits
/// and offset), finds the entry of the worker's level variable table
/// (script header `+4`, level `+0x2e`'s `+0x38` offset, its `+2` count
/// and `+0x14` 8-byte entries: kind and base, `u16` count at `+4`) of the
/// same kind holding the offset (`base <= off < base + count * size`,
/// `AtelGetValueIncValue(kind)`; kind-3 entries of storage 6 below the
/// actor's limit skipped); then `rec+1` bit `0x80` and `+4` = entry index
/// << 16 | element index. (A size of 0 traps in the original; 0 here.)
pub fn atel_find_var_slot(h: &mut impl ffx_bridge::Host, rec: u32) {
    let w = super::atel_get_basic_work(h, i32::from(h.read::<u16>(rec + 2)));
    let hdr = h.read::<u32>(w + 4);
    let level = u32::from(h.read::<u16>(w + 0x2e));
    let c = h.read::<u32>(hdr + level * 4 + 0x38);
    let n = i32::from(h.read::<u16>(c.wrapping_add(hdr) + 2));
    let mut e = h.read::<u32>(c.wrapping_add(hdr) + 0x14).wrapping_add(hdr);
    let v = h.read::<u32>(rec + 4);
    let kind = v >> 28;
    let off = v & 0x0fff_ffff;
    for j in 0..n {
        let ent = h.read::<u32>(e);
        if ent >> 28 == kind {
            let base = ent & 0x0fff_ffff;
            let size = host_call(h, 0x0086_c6e0, &[kind]);
            let end = u32::from(h.read::<u16>(e + 4))
                .wrapping_mul(size)
                .wrapping_add(base);
            let mut skip = false;
            if off & 0x0e00_0000 == 0x0600_0000 {
                let wc = h.read::<u32>(w + 8);
                if wc != 0 {
                    let l = u32::from(h.read::<u16>(w + 0x2e)) << 4;
                    let at = l.wrapping_add(h.read::<u32>(wc + 0x54)).wrapping_add(wc) + 0xc;
                    skip = j < i32::from(h.read::<u16>(at));
                }
            }
            if !skip && base <= off && end > off {
                let q = (off & 0xf0ff_ffff).wrapping_sub(base & 0xf0ff_ffff);
                let q = q.checked_div(size).unwrap_or(0);
                let f = h.read::<u8>(rec + 1);
                h.write(rec + 1, f | 0x80);
                h.write(rec + 4, (q & 0xffff) | (j.cast_unsigned() << 16));
                return;
            }
        }
        e = e.wrapping_add(8);
    }
}

/// `Atel_TurnToWorker_0086f8a0(worker, target)` (name ours; callers
/// `Native_003c`, `0x00869510`): the worker's current turn block (as in
/// [`atel_turn_to_point`]) records the target (`u16` at `+0`) and the
/// worker turns to the target worker's position.
pub fn atel_turn_to_worker<H: ffx_bridge::Host>(
    h: &mut H,
    w: u32,
    target: u32,
    frame: u32,
    yaw_pitch: impl FnOnce(&mut H, &[u32]) -> u32,
) {
    let level = u32::from(h.read::<u8>(w + 0x32));
    let kind = h.read::<u8>(h.read::<u32>(w));
    let level = if matches!(kind, 5 | 6) {
        level.min(1)
    } else {
        level.min(8)
    };
    let turn = h.read::<u32>(w + level * 0x4c + 0x174);
    h.write(turn, lo16(target));
    let t = super::atel_get_basic_work(h, target.cast_signed());
    let p = pos_block(h, t);
    let point = [0, 4, 8].map(|o| x87::store(x87::load(h.read(p + o))));
    atel_turn_to_point(h, w, point, frame, yaw_pitch);
}

/// `Atel_SyncTurnEnd_00870dc0(worker, level)` (name ours; caller
/// `0x008666e0`): a bound worker's character takes its rotation
/// (`Ch_SetRot(chr, pos+0x34)`); then the level's turn (`level+0x48`:
/// mode `u16` at `+2`, low 5 bits) ends (`Script_StopTurn(worker, -1)`,
/// mode masked to `0x5000`, unless bit `0x4000`) when, for modes 3 and 4,
/// the turn's `+0xc` is below its `+8`, or (any mode but 5) when the
/// worker's direction (`pos+0x34`) is the turn's target `+0x20`, and with
/// mode bits `0x80` / `0x40` also `+0x30`/`+0x24` and `+0x38`/`+0x28`.
pub fn atel_sync_turn_end(h: &mut impl ffx_bridge::Host, w: u32, lvl: u32) {
    let p = pos_block(h, w);
    let t = h.read::<u32>(lvl + 0x48);
    let mode = u32::from(h.read::<u16>(t + 2));
    let c = h.read::<u32>(w + 0x9c);
    if h.read::<u8>(w + 0xaa) == 1 && c != 0 {
        let r = x87::store(x87::load(h.read(p + 0x34)));
        host_call(h, 0x0082_b520, &[c, r]);
    }
    let m = mode & 0x1f;
    if m == 5 {
        return;
    }
    if m == 3 || m == 4 {
        let a = x87::load(h.read(t + 0xc));
        let b = x87::load(h.read(t + 8));
        if a < b {
            stop_turn_unless_held(h, w, t);
        }
    }
    if !same_f32(h, t + 0x20, p + 0x34) {
        return;
    }
    let f = h.read::<u16>(t + 2);
    if f & 0x80 != 0 && !same_f32(h, t + 0x24, p + 0x30) {
        return;
    }
    if f & 0x40 != 0 && !same_f32(h, t + 0x28, p + 0x38) {
        return;
    }
    stop_turn_unless_held(h, w, t);
}

/// Whether the `f32`s at `a` and `b` compare equal (`fucompp`: NaN never,
/// -0 and 0 alike).
#[allow(clippy::float_cmp)] // the original's exact compare
fn same_f32(m: &impl Mem, a: u32, b: u32) -> bool {
    x87::load(m.read(a)) == x87::load(m.read(b))
}

/// Unless the turn `t`'s mode (`+2`) has bit `0x4000`, the mode keeps only
/// `0x5000` and the worker's turn stops (`Script_StopTurn(w, -1)`).
fn stop_turn_unless_held(h: &mut impl ffx_bridge::Host, w: u32, t: u32) {
    let f = h.read::<u16>(t + 2);
    if f & 0x4000 == 0 {
        h.write(t + 2, f & 0x5000);
        host_call(h, 0x0086_3fe0, &[w, u32::MAX]);
    }
}

/// Copies the NUL-terminated string at `from` to `to`, its NUL included.
fn copy_cstr(m: &mut impl Mem, from: u32, to: u32) {
    let mut k = 0;
    loop {
        let c = m.read::<u8>(from.wrapping_add(k));
        m.write(to.wrapping_add(k), c);
        if c == 0 {
            return;
        }
        k += 1;
    }
}

/// `Atel_StreamSeek_00885bf0(start, count, sector, id, pos)` (name ours;
/// caller `Native_01d6`): a sound stream's file name
/// (`SndStreamPCName(id, frame)`, 16 bytes) becomes the CD's current
/// stream name (`0x013300c0`); for `pos` -1 also the upper-cased DVD name
/// (`0x013300a0`, `SndToUpper`), `0x01330098` = `0x78`, `*sector` = `0x19`,
/// a count of 1 and the position `SndStreamDVDPos(0x504)`. Then the CD
/// object (`GetCD`) selects `*sector` (its `+0x10`) and seeks twice (its
/// `+4`: 0, then `(pos >> 16) + count`); `*start` = that sector and
/// `*count` = the count (1 without one). Returns the second seek's result
/// plus `pos & 0xffff`.
pub fn atel_stream_seek(
    h: &mut impl ffx_bridge::Host,
    start: u32,
    count: u32,
    sector: u32,
    id: u32,
    mut pos: u32,
    frame: u32,
) -> u32 {
    let mut n = if count == 0 { 1 } else { h.read::<u32>(count) };
    host_call(h, 0x0088_7870, &[id, frame]);
    let name = h.rebase(0x0133_00c0);
    if pos == u32::MAX {
        h.write(h.rebase(0x0133_0098), 0x78_u32);
        let dvd = h.rebase(0x0133_00a0);
        copy_cstr(h, frame, dvd);
        copy_cstr(h, frame, name);
        host_call(h, 0x0088_7940, &[dvd]);
        pos = host_call(h, 0x0088_7790, &[0x504]);
        h.write(sector, 0x19_u32);
        n = 1;
    } else {
        copy_cstr(h, frame, name);
    }
    let cd = host_call(h, 0x0076_d0d0, &[]);
    let select = h.read::<u32>(cd + 0x10);
    let s = h.read::<u32>(sector);
    h.call(select, &[s]);
    let at = (pos >> 16).wrapping_add(n);
    let seek = h.read::<u32>(cd + 4);
    h.call(seek, &[0]);
    let seek = h.read::<u32>(cd + 4);
    let r = h.call(seek, &[at]).wrapping_add(pos & 0xffff);
    if start != 0 {
        h.write(start, at);
    }
    if count != 0 {
        h.write(count, n);
    }
    r
}

/// `AtelJumpGameOver()` (`0x0086da10`; caller `AtelBattleEndCallback`):
/// the game-over jump: the menus and battle reset (`TOReset1`,
/// `MsBtlGetInit`), vibration stopped and masked, the event's and the
/// playing music's replay points cleared (the playing music faded out
/// over 0x78 frames), music `0x8a` stopped and cleared, no music number,
/// every separate and sound stopped and the waves cleared
/// (`SndClearWave(-2)`), no separates to load, no group mask, memory
/// movie 6; ctrl work 0 flagged (`+0` bit `0x80`, `+0x210` bit 2) and
/// current for the focus, blur and filter resets, the ripple effect off,
/// the text language reset, `TOReset2`, `g_TkScalePair` = (2, 2) and the
/// jump to map `0x17` (`AtelSetEventJump(0x17, 0)`); the ctrl work back.
pub fn atel_jump_game_over(h: &mut impl ffx_bridge::Host) {
    for f in [0x008a_de50, 0x0079_8b60, 0x0087_62f0] {
        host_call(h, f, &[]);
    }
    host_call(h, 0x0087_5f60, &[u32::MAX, 1]);
    if host_call(h, 0x0087_3b00, &[]).cast_signed() >= 0 {
        let n = host_call(h, 0x0087_3b00, &[]);
        host_call(h, 0x0088_69e0, &[n]);
    }
    if host_call(h, SND_GET_NOW_PLAY_MUSIC, &[]).cast_signed() >= 0 {
        let n = host_call(h, SND_GET_NOW_PLAY_MUSIC, &[]);
        host_call(h, 0x0088_69e0, &[n]);
        let n = host_call(h, SND_GET_NOW_PLAY_MUSIC, &[]);
        host_call(h, 0x0088_6af0, &[n, 0x78]);
    }
    host_call(h, 0x0088_6a60, &[0x8a]);
    host_call(h, 0x0088_6800, &[0x8a]);
    host_call(h, 0x0087_3d00, &[u32::MAX]);
    host_call(h, 0x0088_6db0, &[]);
    atel_stop_sound_all(h);
    host_call(h, 0x0088_5ec0, &[0xffff_fffe]);
    h.write(h.rebase(SEP_LOAD_COUNT), 0_u32);
    h.write(h.rebase(SEP_GROUP_MASK), 0_u8);
    host_call(h, 0x0087_3e90, &[6]);
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let old = h.read::<u32>(ctx);
    let works = h.rebase(CTRL_WORKS);
    let f = h.read::<u8>(works);
    h.write(works, f | 0x80);
    let f = h.read::<u32>(works + 0x210);
    h.write(works + 0x210, f | 2);
    h.write(ctx, works);
    host_call(h, 0x0082_cb60, &[0]);
    host_call(h, 0x0082_bee0, &[0]);
    host_call(h, 0x0082_cc70, &[]);
    host_call(h, 0x0090_8450, &[]);
    atel_reset_text_language(h);
    host_call(h, 0x008a_dee0, &[]);
    let two = 2f32.to_bits();
    h.write(h.rebase(0x0132_5b98), two);
    h.write(h.rebase(0x0132_5b9c), two);
    host_call(h, 0x0086_fec0, &[0x17, 0]);
    h.write(ctx, old);
}

/// `Atel_KickBGSepsOnFlag_00865780(header, on)` (name ours; caller
/// `0x00865740`): every background separate (flags bit 2) of the event
/// header's table: one flagged `0x80` (starts later) stops without `on`
/// and plays (`SndSepPlay(id, 0x3f, 0x7f)`) with it; one not flagged
/// fades to `0x3f` with `on` and `0x7f` without over 12 frames
/// (`SndSepVolumeTranslate`, not for flag `0x40`). In a running event
/// (ctx `+2` bit 8, `+0x1f4` past 3), flag `0x100` without `on` or `0x200`
/// with it plays the separate instead.
pub fn atel_kick_bg_seps_on_flag(h: &mut impl ffx_bridge::Host, hdr: u32, on: u32) {
    let t = sep_table(h, hdr);
    let n = if t == 0 { 0 } else { h.read::<i32>(t) };
    for i in 0..n.max(0).cast_unsigned() {
        let t = sep_table(h, hdr);
        let id = if t == 0 {
            0
        } else {
            h.read::<u32>(t + i * 8 + 4)
        };
        let t = sep_table(h, hdr);
        let flags = if t == 0 {
            0
        } else {
            h.read::<u32>(t + i * 8 + 8)
        };
        if flags & 2 == 0 {
            continue;
        }
        let c = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
        let mut force = false;
        if h.read::<u8>(c + 2) & 8 != 0 && h.read::<u32>(c + 0x1f4) > 3 {
            if flags & 0x100 != 0 && on == 0 {
                force = true;
            }
            if flags & 0x200 != 0 && on != 0 {
                force = true;
            }
        }
        let play = if flags & 0x80 != 0 {
            if on == 0 {
                host_call(h, 0x0088_6eb0, &[id]);
                force
            } else {
                if flags & 0x40 == 0 {
                    host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
                }
                true
            }
        } else if force {
            true
        } else {
            if flags & 0x40 == 0 {
                let v = if on == 0 { 0x7f } else { 0x3f };
                host_call(h, 0x0088_6ff0, &[id, 0xc, v]);
            }
            false
        };
        if play {
            host_call(h, 0x0088_6e50, &[id, 0x3f, 0x7f]);
        }
    }
}

/// `Native_GetChrOptPos_00a7a980(worker, _, stack, mode)` (name ours;
/// caller `Native_5036`, mode 0): pops three variable words (z, y, x
/// order: the last popped is x's), for mode 0 an attachment index, and a
/// worker index; that worker's character gives a position (mode 0:
/// `Ch_OptposGet(chr, index, frame)`; 1: point 0; 2: its `+0x6b4`
/// vector) stored (`fstp m32`) into the variables that have an address
/// (`AtelGetRealVarAdrs`). Without a character nothing is stored. (For
/// another mode the original stores its uninitialised locals; `frame`
/// then.) Returns 0.
pub fn native_get_chr_opt_pos(
    h: &mut impl ffx_bridge::Host,
    w: u32,
    s: u32,
    mode: u32,
    frame: u32,
) -> u32 {
    let vz = super::atel_pop_stack_integer(h, s);
    let vy = super::atel_pop_stack_integer(h, s);
    let vx = super::atel_pop_stack_integer(h, s);
    let index = if mode == 0 {
        super::atel_pop_stack_integer(h, s)
    } else {
        0
    };
    let wi = super::atel_pop_stack_integer(h, s);
    let ax = super::atel_get_real_var_adrs(h, w, vx);
    let ay = super::atel_get_real_var_adrs(h, w, vy);
    let az = super::atel_get_real_var_adrs(h, w, vz);
    let t = super::atel_get_basic_work(h, wi.cast_signed());
    let chr = super::atel_get_char_handle(h, t);
    if chr == 0 {
        return 0;
    }
    match mode {
        0 => {
            host_call(h, 0x0083_54f0, &[chr, index, frame]);
        }
        1 => {
            host_call(h, 0x0083_54f0, &[chr, 0, frame]);
        }
        2 => {
            for k in 0..4 {
                let v = h.read::<u32>(chr + 0x6b4 + k * 4);
                h.write(frame + k * 4, v);
            }
        }
        _ => {}
    }
    for (k, at) in (0u32..).zip([ax, ay, az]) {
        if at != 0 {
            copy_f32(h, frame + k * 4, at);
        }
    }
    0
}

/// `Atel_ApplyPolarMove_008629c0(worker)` (name ours; caller
/// `0x008666e0`): a worker bound as type 2..5 (`+0xaa`) moves by its
/// polar step (`Math_PolarToVector(&d, r, yaw, pitch)`: r its position
/// block's `+0xc`, a tenth for types 4 and 5; yaw `+0x20`, pitch `+0x24`;
/// `d` in `frame`, 32 bytes) added to its position; type 2 then moves its
/// camera there (`MsCameraSetRect(+0x3c, 1, &(x, y, z, 1))`, the vector at
/// `frame + 0x10`) when its level's turn blocks (`+0x44`, `+0x48`) have a
/// mode; types 4 and 5 re-bind (`AtelSetBindObject`).
pub fn atel_apply_polar_move(h: &mut impl ffx_bridge::Host, w: u32, frame: u32) {
    let b = h.read::<u8>(w + 0xaa);
    if !(2..=5).contains(&b) {
        return;
    }
    let level = u32::from(h.read::<u8>(w + 0x32));
    let lv = super::script_worker_level(h, w, level.cast_signed());
    let p = pos_block(h, w);
    let mut r = x87::store(x87::load(h.read(p + 0xc)));
    if b == 4 || b == 5 {
        r = x87::store(x87::div(x87::load(r), 10.0));
    }
    let yaw = x87::store(x87::load(h.read(p + 0x20)));
    let pitch = x87::store(x87::load(h.read(p + 0x24)));
    host_call(h, 0x0088_a3a0, &[frame, r, yaw, pitch]);
    let mut v = [0u32; 3];
    for k in 0..3u32 {
        let s = x87::add(
            x87::load(h.read(p + k * 4)),
            x87::load(h.read(frame + k * 4)),
        );
        v[k as usize] = x87::store(s);
        h.write(p + k * 4, v[k as usize]);
    }
    match h.read::<u8>(w + 0xaa) {
        2 => {
            let a = h.read::<u32>(lv + 0x44);
            let c = h.read::<u32>(lv + 0x48);
            if h.read::<u16>(a + 2) != 0 || h.read::<u16>(c + 2) != 0 {
                let rect = frame + 0x10;
                for (k, x) in (0u32..).zip(v) {
                    h.write(rect + k * 4, x);
                }
                h.write(rect + 12, 1f32.to_bits());
                let cam = h.read::<u32>(w + 0x3c);
                host_call(h, MS_CAMERA_SET_RECT, &[cam, 1, rect]);
            }
        }
        4 | 5 => {
            host_call(h, 0x0086_f4c0, &[w]);
        }
        _ => {}
    }
}

/// `Atel_SetEventWaterFlag_00865740(on)` (name ours; caller
/// `0x008682f0`): when the save event's "in water" byte (`+0x28`) is not
/// `on` yet, it becomes `on` and the event's delayed background
/// separates (ctx `+0x48`, [`atel_kick_bg_seps_on_flag`]) and water
/// effects ([`atel_show_water_fps`]) follow.
pub fn atel_set_event_water_flag(h: &mut impl ffx_bridge::Host, on: u32) {
    let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
    if on == u32::from(h.read::<u8>(save + 0x28)) {
        return;
    }
    h.write(save + 0x28, on.to_le_bytes()[0]);
    let c = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    let hdr = h.read::<u32>(c + 0x48);
    if hdr != 0 {
        atel_kick_bg_seps_on_flag(h, hdr, on);
    }
    atel_show_water_fps(h, on);
}

/// `AtelUpdateWindowStatus()` (`0x00871f30`; caller `0x008688f0`): each
/// message window of the bank takes the window system's status
/// (`TOGetWindowStatus(i)`, 4 read as 0); one shown with mode flag 2 makes
/// the windows busy, the last open one (status 2) with mode flags `0x32`
/// becomes the active one (`TOSetMessageWindowActive`, 0 without). With
/// no message running (ctx `+0x1a` 0), the ctx's window countdown
/// `+0x1ea` steps outside menus and battles (at 1 it clears `+3` bit
/// `0x80` and ends), the ctx's `+1` bit 4 follows the busy state (cleared
/// at once without `+1` bit 8), and on a change or the countdown's end
/// the UC is turned off and back on as the ctx allows; `+3` bit `0x10`
/// says whether the player's control is off. Returns the activation's
/// `EAX`.
pub fn atel_update_window_status(h: &mut impl ffx_bridge::Host) -> u32 {
    let mut busy = false;
    let mut active = 0u32;
    for i in 0..8u32 {
        // the window's status word (its `+0x14`; mode flags 9 bytes on)
        let win = window(h, i);
        let st = host_call(h, 0x0090_6c20, &[i]);
        h.write(win, lo16(st));
        if lo16(st) == 4 {
            h.write(win, 0_u16);
        }
        let s = h.read::<u16>(win);
        if s != 0 && h.read::<u8>(win + 9) & 2 != 0 {
            busy = true;
        }
        if s == 2 && h.read::<u8>(win + 9) & 0x32 != 0 {
            active = i;
        }
    }
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let mut c = h.read::<u32>(ctx);
    if h.read::<u16>(c + 0x1a) == 0 {
        let mut ended = false;
        if h.read::<u8>(c + 0x1ea) != 0 {
            if host_call(h, 0x0087_2b50, &[]) == 0
                && h.read::<u8>(h.rebase(MENU_EXEC_STATUS)) == 0
                && host_call(h, 0x0078_0c90, &[]) == 0
            {
                c = h.read::<u32>(ctx);
                let v = h.read::<u8>(c + 0x1ea);
                if v == 1 {
                    let f = h.read::<u8>(c + 3);
                    h.write(c + 3, f & 0x7f);
                    ended = true;
                    h.write(c + 0x1ea, 0_u8);
                } else {
                    h.write(c + 0x1ea, v.saturating_sub(1));
                }
            } else {
                c = h.read::<u32>(ctx);
            }
        }
        let f = h.read::<u8>(c + 1);
        let was = f & 4 != 0;
        if was != busy || ended {
            if was != busy {
                let f = if f & 8 == 0 {
                    f & 0xfb
                } else {
                    (f & !4) | (u8::from(busy) << 2)
                };
                h.write(c + 1, f);
            }
            let b = h.read::<u8>(c);
            let keep = b & 2;
            let on = b & 0x22 != 0;
            atel_uc_off(h);
            c = h.read::<u32>(ctx);
            if h.read::<u8>(c + 1) & 4 == 0 && h.read::<u8>(c + 3) & 0x84 == 0 && on {
                atel_uc_on(h);
                c = h.read::<u32>(ctx);
            }
            let f = h.read::<u8>(c);
            h.write(c, (f & !2) | keep);
        }
        let off = h.read::<u8>(c + 1) & 4 != 0 || h.read::<u8>(c) & 2 == 0;
        let f = h.read::<u8>(c + 3);
        h.write(c + 3, (f & !0x10) | (u8::from(off) << 4));
    }
    host_call(h, 0x008a_f7d0, &[active])
}

/// `AtelUpdateBattleStatus()` (`0x00871bc0`; caller `0x00872bd0`): with
/// ctrl work 0 current (left so) and running: a battle requested by its
/// `+0x210` bit 2 is cleared first (`MsBattleExeClear`); the distance the
/// ctx's camera target moved (`+0x218` against `+0x228`,
/// `CIsqrt(Math_LengthSq3(d))`, `fcall` for those `ST0` results) is the
/// walk step: for a player in control (`+0xa` set, `+0` bits 2 and 8,
/// not `+0` bit `0x10`, not `+1` bits 1 and 4, not `+3` bits `0x84`, the
/// event past frame 6) it counts toward the walk abilities
/// (`MsWalkAbilityProcess`) and, with encounters on (`+0x210` bit 1
/// without 4, the save event's `+0x13` bits 6), toward an encounter:
/// outside battles, menus (`+3` bits 3), the sound's busy state
/// (`0x00886630`) and CD reads (`GetCD()->+8(1)`), `MsBattleEncountExe(map
/// (the forced one when set), the save event's group, step)`. Last, ctx
/// `+1` loses bits 1 and 2.
#[allow(clippy::verbose_bit_mask)] // the original's `+3` bits 0..1 test
pub fn atel_update_battle_status<H: ffx_bridge::Host>(
    h: &mut H,
    mut fcall: impl FnMut(&mut H, u32, &[u32]) -> f64,
) {
    let cd = host_call(h, 0x0076_d0d0, &[]);
    let mut amount = 0u32;
    let mut encount = false;
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let works = h.rebase(CTRL_WORKS);
    h.write(ctx, works);
    if h.read::<u8>(works) & 1 != 0 {
        let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
        let mut c = h.read::<u32>(ctx);
        if h.read::<u8>(c + 0x210) & 2 != 0 {
            host_call(h, 0x0078_1530, &[]);
            c = h.read::<u32>(ctx);
        }
        let d = [0u32, 4, 8].map(|k| {
            x87::store(x87::sub(
                x87::load(h.read(c + 0x228 + k)),
                x87::load(h.read(c + 0x218 + k)),
            ))
        });
        let l = x87::store(fcall(h, 0x0088_c3c0, &d));
        let dist = x87::store(fcall(h, 0x0088_c400, &[l]));
        let c = h.read::<u32>(ctx);
        let f0 = h.read::<u8>(c);
        if h.read::<u16>(c + 0xa) != 0xffff && f0 & 2 != 0 {
            let f1 = h.read::<u8>(c + 1);
            if f1 & 4 == 0
                && h.read::<u8>(c + 3) & 0x84 == 0
                && f0 & 0x10 == 0
                && f1 & 1 == 0
                && f0 & 8 != 0
                && h.read::<u32>(c + 0x1f4) > 6
            {
                let fl = h.read::<u32>(c + 0x210);
                if fl & 1 != 0 && h.read::<u8>(save + 0x13) & 6 != 0 && fl & 4 == 0 {
                    amount = dist;
                    encount = true;
                }
                if host_call(h, 0x0078_0c90, &[]) == 0 {
                    let c = h.read::<u32>(ctx);
                    if h.read::<u8>(c + 3) & 3 == 0 {
                        host_call(h, 0x0078_2d60, &[dist]);
                    }
                }
            }
        }
        let go = host_call(h, 0x0078_0c90, &[]) == 0
            && {
                let c = h.read::<u32>(ctx);
                h.read::<u8>(c + 3) & 3 == 0
            }
            && host_call(h, 0x0088_6630, &[]) == 0
            && {
                let f = h.read::<u32>(cd + 8);
                h.call(f, &[1]) == 0
            }
            && encount;
        if go {
            let mut map = host_call(h, 0x0088_d690, &[]);
            let forced = h.read::<i32>(h.rebase(FORCE_BATTLE_MAP));
            if forced > 0 {
                map = forced.cast_unsigned();
            }
            let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
            let group = u32::from(h.read::<u8>(save + 0x10));
            host_call(h, 0x0078_0d10, &[map, group, amount]);
        }
    }
    let c = h.read::<u32>(ctx);
    let f = h.read::<u8>(c + 1);
    h.write(c + 1, f & 0xfc);
}

/// `Atel_InitWorkerLevels_0086d300(worker, a, b, c)` (name ours; caller
/// `AtelInitBasicWork`): each level (2 for types 5 and 6, 9 otherwise)
/// gets `a`, `b`, `c` at `+0x16c`, `+0x16a`, `+0x168` (`u16`) and is reset
/// (`AtelResetLevel`); a type-1 worker's levels also get their move block
/// (`+0x5b8 + 0x4c*l`, at `+0x170`: speed `0x5a`, `+0x10` 0, `+0x38` 1,
/// `+0x3c`/`+0x40` pi) and turn block (`+0x864 + 0x44*l`, at `+0x174`:
/// speed `0x5a`, `+0x10..+0x1c` pi/64).
pub fn atel_init_worker_levels(h: &mut impl ffx_bridge::Host, w: u32, a: u32, b: u32, c: u32) {
    let kind = |h: &mut _| -> u8 { kind_of(h, w) };
    let set = |h: &mut _, l: u32| {
        let at = w + l * 0x4c;
        write_u16(h, at + 0x16c, a);
        write_u16(h, at + 0x16a, b);
        write_u16(h, at + 0x168, c);
    };
    if matches!(kind(h), 5 | 6) {
        for i in 0..2 {
            set(h, i.min(1));
            host_call(h, 0x0086_39e0, &[w, i]);
        }
        return;
    }
    // (the original clamps the level by the type again here; it is not 5
    // or 6 on this path, so level i is i)
    for i in 0..9u32 {
        let l = i;
        set(h, l);
        host_call(h, 0x0086_39e0, &[w, i]);
        if kind(h) != 1 {
            continue;
        }
        let at = w + l * 0x4c;
        let mv = w + 0x5b8 + i * 0x4c;
        let turn = w + 0x864 + i * 0x44;
        h.write(at + 0x174, turn);
        h.write(at + 0x170, mv);
        h.write(mv + 4, 0x5a_u16);
        h.write(mv + 6, 0_u16);
        h.write(mv + 0x10, 0_u32);
        let pi = h.read::<u32>(h.rebase(0x00b9_22d0));
        let pi = x87::store(x87::load(pi));
        h.write(mv + 0x3c, pi);
        h.write(mv + 0x40, pi);
        let step = x87::store(x87::load(h.read(h.rebase(0x00b5_be78))));
        for k in 0..4 {
            h.write(turn + 0x10 + k * 4, step);
        }
        h.write(turn + 4, 0x5a_u16);
        h.write(turn + 6, 0_u16);
        h.write(mv + 0x38, 1f32.to_bits());
    }
}

/// The worker's type (its header's first byte).
fn kind_of(m: &impl Mem, w: u32) -> u8 {
    m.read::<u8>(m.read::<u32>(w))
}

/// The low 16 bits of `v` at `at`.
fn write_u16(m: &mut impl Mem, at: u32, v: u32) {
    m.write(at, lo16(v));
}

/// `Atel_CheckLineCross_008680a0(worker, _, target, rec)` (name ours;
/// caller `0x00868020`): whether the line the target (a worker, or for
/// `0xffff` the ctx's camera target) moved along since last frame (from
/// its position block's `+0x10` to `+0`; the ctx's `+0x228` to `+0x218`)
/// crosses the worker's line (`+0` to `+0x30`, `Math_SegmentsCross`; the
/// four points in `frame`, 64 bytes): not checked when the target did not
/// move in x and z unless the worker's `+0x36` bit 0 asks (then cleared).
/// A crossing counts when the worker's height `+0x1c` is not positive or
/// the crossing's height gap (`Math_CrossHeightGap`, through `fcall`) is
/// at most it: without `rec`, `+0x36` bits 2/4 record the side and the
/// worker's trigger 4 is requested (`AtelRequestTrigger_008764f0`); with
/// one, `rec+4` = the side and a vibration signal is sent
/// (`Atel_RegistVibrateSignalWith_008766d0`, curve `rec+2`; both called
/// through the game).
pub fn atel_check_line_cross<H: ffx_bridge::Host>(
    h: &mut H,
    w: u32,
    target: u32,
    rec: u32,
    frame: u32,
    mut fcall: impl FnMut(&mut H, u32, &[u32]) -> f64,
) {
    let p = pos_block(h, w);
    let (from, to) = if target == 0xffff {
        let c = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
        (c + 0x228, c + 0x218)
    } else {
        let t = super::atel_get_basic_work(h, target.cast_signed());
        let q = pos_block(h, t);
        (q + 0x10, q)
    };
    let (a, b, c, d) = (frame, frame + 0x10, frame + 0x20, frame + 0x30);
    for (dst, src) in [(a, from), (b, to), (c, p), (d, p + 0x30)] {
        for k in 0..3 {
            copy_f32(h, src + k * 4, dst + k * 4);
        }
    }
    let height = x87::store(x87::load(h.read(p + 0x1c)));
    let still = same_f32(h, b, a) && same_f32(h, b + 8, a + 8);
    let f = h.read::<u16>(w + 0x36);
    if still && f & 1 == 0 {
        return;
    }
    h.write(w + 0x36, f & 0xfffe);
    let side = host_call(h, 0x0088_9bb0, &[a, b, c, d]).cast_signed();
    if side == 0 {
        return;
    }
    let gap = x87::load(x87::store(fcall(h, 0x0088_9e90, &[a, b, c, d])));
    let hgt = x87::load(height);
    if !(hgt <= 0.0 || gap <= hgt) {
        return;
    }
    let id = u32::from(h.read::<u16>(w + 0x2e));
    if rec == 0 {
        let f = h.read::<u16>(w + 0x36);
        let f = if side > 0 {
            (f & 0xfffb) | 2
        } else {
            f & 0xfff9
        };
        h.write(w + 0x36, f);
        host_call(h, 0x0087_64f0, &[0xffff, id, 4]);
    } else {
        h.write(rec + 4, u8::from(side > 0));
        let curve = u32::from(h.read::<u16>(rec + 2));
        host_call(h, 0x0087_66d0, &[0xffff, id, 4, curve]);
    }
}

// ---- script round 26 ----

/// `g_AtelBattleEndStatus` (`0x01326b48`): `+0` the status (low byte the
/// end kind, bit 9 no game over, bit 14 no camera reset), `+4` its copy
/// taken at the battle end, `+8`/`+0xc` the movie or effect arguments.
const BATTLE_END_STATUS: u32 = 0x0132_6b48;
/// `AtelRegistCallbackQue2` (`0x008751b0`).
const ATEL_REGIST_CALLBACK_QUE2: u32 = 0x0087_51b0;

/// `AtelRegistCallbackQue2(0, f, name)` with `f` and `name` image
/// addresses.
fn queue_callback(h: &mut impl ffx_bridge::Host, f: u32, name: u32) -> u32 {
    let (f, name) = (h.rebase(f), h.rebase(name));
    host_call(h, ATEL_REGIST_CALLBACK_QUE2, &[0, f, name])
}

/// `AtelBattleEndCallback(mode, v, map, end)` (`0x00861160`): the field's
/// side of a battle's end. It resets the party mask, the effect state, the
/// lights, the separates lock, the stream, the standby voice and the
/// vibration of pads 1..3, and stores `end` in the save event (`+0xd0`).
/// End type 6 resets the renderer (`PostReset(1)`) and fades to opaque
/// black. Otherwise, with ctrl work 0 current: end type 1 without status
/// bit 9 jumps to the game over; anything else queues the callbacks that
/// bring the field back (`mode` 1 = after a battle: `CAMERA` unless status
/// bit 14, then by the status's low byte `MOVIE` (2) or the end effect
/// (3, 4), and `EFFDONE` for 3; other `mode`s `CAMERA` only), `SNDCLR`,
/// `SNDREG`, the map start, `CHRREG`, `DONE` and `DONE2`, or instead of
/// `DONE2` (`mode` not 1, or an end kind outside 1..4) the menu reserve
/// `0x40010002` and message window area 2. The status becomes `0x10001`
/// and the old ctrl work current again. Returns 1. `EmptyStub_008750a0(0)`
/// at the end is skipped.
pub fn atel_battle_end_callback(
    h: &mut impl ffx_bridge::Host,
    mode: u32,
    v: u32,
    map: u32,
    end: u32,
) -> u32 {
    host_call(h, 0x0086_23b0, &[]);
    let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
    let effect_state = h.rebase(0x0132_6b58);
    h.write(effect_state, 0_u32);
    host_call(h, 0x0086_e770, &[]);
    host_call(h, 0x0088_6dd0, &[0]);
    host_call(h, 0x0088_7780, &[]);
    host_call(h, 0x0088_7b50, &[0]);
    for pad in 1..=3 {
        host_call(h, 0x0087_5f60, &[pad, 1]);
    }
    h.write(save + 0xd0, end.to_le_bytes()[0]);
    if end == 6 {
        host_call(h, 0x0042_fa70, &[1]);
        host_call(h, 0x0064_3900, &[0, 0, 0, 1.0_f32.to_bits()]);
        host_call(h, 0x0064_3940, &[3]);
        host_call(h, 0x0064_1e20, &[]);
        return 1;
    }
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let works = h.rebase(CTRL_WORKS);
    let status = h.rebase(BATTLE_END_STATUS);
    let saved = h.read::<u32>(ctx);
    let s = h.read::<u32>(status);
    h.write(ctx, works);
    h.write(status + 4, s);
    if end == 1 && s & 0x200 == 0 {
        host_call(h, 0x0086_da10, &[]);
    } else {
        battle_end_queue(h, mode, v, map);
    }
    h.write(status, 0x0001_0001_u32);
    h.write(ctx, saved);
    1
}

/// The callback queue of [`atel_battle_end_callback`] (no game over).
fn battle_end_queue(h: &mut impl ffx_bridge::Host, mode: u32, v: u32, map: u32) {
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let works = h.rebase(CTRL_WORKS);
    let status = h.rebase(BATTLE_END_STATUS);
    host_call(h, 0x0087_3e80, &[]);
    let mut c = h.read::<u32>(ctx);
    h.write(ctx, works);
    host_call(h, 0x0086_e8d0, &[v]);
    h.write(ctx, c);
    host_call(h, 0x0086_30c0, &[]);
    let foot = h.read::<u32>(c + 0x48);
    if foot != 0 {
        host_call(h, 0x0086_5b80, &[foot]);
        host_call(h, 0x0088_6c10, &[]);
        c = h.read::<u32>(ctx);
    }
    let flags = h.read::<u8>(c + 3);
    // the field menu comes back by the reserve instead of DONE2
    let mut reserve = false;
    if mode == 1 {
        h.write(c + 3, flags & 0xfe | 2);
        if h.read::<u32>(status) & 0x4000 == 0 {
            queue_callback(h, 0x0086_1800, 0x00b5_b378); // "CAMERA"
        }
        match h.read::<u8>(status) {
            1 => {}
            2 => {
                host_call(h, 0x0087_3d00, &[u32::MAX]);
                // the movie's id and flags for the MOVIE callback
                // (`0x01326b38`, `0x01326b3c`)
                let movie = h.rebase(0x0132_6b38);
                let id = h.read::<u32>(status + 8);
                h.write(movie, id);
                let flags = h.read::<u32>(status + 0xc);
                h.write(movie + 4, flags);
                queue_callback(h, 0x0086_2000, 0x00b5_be2c); // "MOVIE"
            }
            3 | 4 => {
                let (a, b) = (h.read::<u32>(status + 8), h.read::<u32>(status + 0xc));
                host_call(h, 0x0086_1550, &[a, b]);
            }
            _ => reserve = true,
        }
    } else {
        reserve = true;
        h.write(c + 3, flags & 0xfd | 1);
        h.write(status + 4, 0_u32);
        queue_callback(h, 0x0086_1800, 0x00b5_b378); // "CAMERA"
    }
    queue_callback(h, 0x0086_2120, 0x00b5_bdfc); // "SNDCLR"
    queue_callback(h, 0x0086_2220, 0x00b5_be04); // "SNDREG"
    host_call(h, 0x0086_15b0, &[map]);
    queue_callback(h, 0x0086_1850, 0x00b5_be14); // "CHRREG"
    if mode == 1 && h.read::<u8>(status) == 3 {
        queue_callback(h, 0x0086_1f20, 0x00b5_be44); // "EFFDONE"
    }
    queue_callback(h, 0x0086_1480, 0x00b5_be1c); // "DONE"
    if reserve {
        host_call(h, 0x0082_1750, &[0x4001_0002]);
        host_call(h, 0x008a_f5c0, &[2]);
        h.write(status + 4, 0_u8);
    } else {
        queue_callback(h, 0x0086_1510, 0x00b5_be24); // "DONE2"
    }
}

/// `Atel_RunCtrlWorkers_00867950(event, input, clear)` (name ours; callers
/// `AtelExecCtrlEvent` and `0x00867710`): one pass of the current ctrl
/// work's scripts, repeated while a script asks for another. With `input`
/// set it first clears the choice (`+0x1e4`) and `event` bit 2, maps the
/// pressed buttons (`Pad_MapButtons`, nothing from pad count 2 up, the
/// held words mapped too and dropped), clears the encounter flag, sets the
/// save event's `+0x13` bit 1 (bit 2 clear) and clears ctx bit 4; with
/// buttons `0xa0` pressed and no window busy (and ctx bits `0x22`,
/// `0x00c526d4` set) a choice is opened (`+0x1e6` = no cross pressed,
/// `+0x1e4` = 1, `+0x1e8` = `0xffff`); with busy windows (1) the cross sets
/// `event` bit 2. Then every worker (`event +0xc` of them, `+4` the
/// index) runs (`Script_Run(w, event +8, 1)`; without `input` only workers
/// with `+0x36` bits `0x180`, level 0), ctx `+2` bit 4 is cleared and a
/// chosen worker (`+0x1e8`) gets the affection and its trigger. Last every
/// worker's signals are decoded (without `input` only with `+0x36` bits
/// `0x180`, which then drop one level; pending levels `0x600` move down to
/// `0x180`) and `clear` drops `+0x36` bit 12. A result with bit 15 sets ctx
/// `+8` = 3; any bit but 14 and 15 runs the pass again. Returns the last
/// pass's results without bit 15.
pub fn atel_run_ctrl_workers(
    h: &mut impl ffx_bridge::Host,
    event: u32,
    input: u32,
    clear: u32,
) -> u32 {
    let ctx_at = h.rebase(SCRIPT_CONTEXT);
    let save = host_call(h, TK_GET_SAVE_EVENT_ADDRESS, &[]);
    let count = u32::from(h.read::<u16>(event + 0xc));
    let level = u32::from(h.read::<u8>(event + 8));
    // `ebx`: the context as last read
    let mut c = h.read::<u32>(ctx_at);
    loop {
        if input != 0 {
            c = ctrl_pass_input(h, event, save, c);
        }
        let mut result = 0;
        if count != 0 {
            for i in 0..count {
                h.write(ctx_at + 4, i);
                let w = host_call(h, 0x0086_a830, &[i]);
                result |= if input != 0 {
                    host_call(h, 0x0086_41e0, &[w, level, 1])
                } else if h.read::<u16>(w + 0x36) & 0x180 != 0 {
                    host_call(h, 0x0086_41e0, &[w, level, 0])
                } else {
                    0
                };
            }
            c = h.read::<u32>(ctx_at);
        }
        let f = h.read::<u8>(c + 2);
        h.write(c + 2, f & 0xef);
        let chosen = u32::from(h.read::<u16>(c + 0x1e8));
        if h.read::<u8>(c + 0x1e4) == 1
            && chosen != 0xffff
            && h.read::<u8>(c) & 0x22 != 0
            && input != 0
        {
            let member = u32::from(h.read::<u8>(c + 0x1e6));
            host_call(h, 0x0086_5650, &[chosen, member]);
            let c2 = h.read::<u32>(ctx_at);
            let member = u32::from(h.read::<u8>(c2 + 0x1e6));
            let chosen = u32::from(h.read::<u16>(c2 + 0x1e8));
            host_call(h, 0x0087_64f0, &[0xffff, chosen, member]);
            c = h.read::<u32>(ctx_at);
        }
        if count != 0 {
            ctrl_pass_signals(h, count, input, clear);
            c = h.read::<u32>(ctx_at);
        }
        if result & 0x8000 != 0 {
            h.write(c + 8, 3_u8);
            result &= 0xffff_7fff;
        }
        if result & 0xffff_3fff == 0 {
            return result;
        }
    }
}

/// The input part of [`atel_run_ctrl_workers`]: the choice cleared, the
/// buttons mapped, the encounter flag and the flags, starting from the
/// context `c` as last read; returns the context as read then.
fn ctrl_pass_input(h: &mut impl ffx_bridge::Host, event: u32, save: u32, c: u32) -> u32 {
    let ctx_at = h.rebase(SCRIPT_CONTEXT);
    h.write(c + 0x1e4, 0_u8);
    let e = h.read::<u8>(event);
    h.write(event, e & 0xfb);
    let pressed = if h.read::<i32>(ctx_at + 8) < 2 {
        let p = u32::from(h.read::<u16>(h.rebase(0x0132_70d0)));
        let pressed = host_call(h, 0x0088_8c40, &[0, 0, p]) & 0xffff;
        for held in [0x0132_70c0, 0x0132_70c4] {
            let v = u32::from(h.read::<u16>(h.rebase(held)));
            host_call(h, 0x0088_8c40, &[0, 0, v]);
        }
        pressed
    } else {
        0
    };
    host_call(h, 0x0078_0fe0, &[0]);
    let c = h.read::<u32>(ctx_at);
    let f = h.read::<u8>(save + 0x13);
    h.write(save + 0x13, f & 0xfb | 2);
    let flags = h.read::<u8>(c) & 0xef;
    h.write(c, flags);
    if pressed & 0xa0 != 0 {
        match host_call(h, 0x0086_c8b0, &[]) {
            0 => {
                if flags & 0x22 != 0 && h.read::<u32>(h.rebase(0x00c5_26d4)) != 0 {
                    h.write(c + 0x1e6, u8::from(pressed & 0x20 == 0));
                    h.write(c + 0x1e4, 1_u8);
                    h.write(c + 0x1e8, 0xffff_u16);
                }
            }
            1 if pressed & 0x20 != 0 => {
                let e = h.read::<u8>(event);
                h.write(event, e | 4);
            }
            _ => {}
        }
    }
    c
}

/// The signal part of [`atel_run_ctrl_workers`] over `count` workers.
fn ctrl_pass_signals(h: &mut impl ffx_bridge::Host, count: u32, input: u32, clear: u32) {
    for i in 0..count {
        let w = host_call(h, 0x0086_a830, &[i]);
        // pending levels (bits 9, 10) down to the running ones (7, 8)
        let lower = |f: u16| f >> 2 & 0x180 | f & 0xf87f;
        if input != 0 {
            host_call(h, 0x0086_53f0, &[i, u32::MAX]);
            let f = h.read::<u16>(w + 0x36);
            h.write(w + 0x36, lower(f));
        } else {
            let f = h.read::<u16>(w + 0x36);
            if f & 0x600 != 0 {
                h.write(w + 0x36, lower(f));
            }
            if h.read::<u16>(w + 0x36) & 0x180 != 0 {
                host_call(h, 0x0086_53f0, &[i, u32::MAX]);
                let f = h.read::<u16>(w + 0x36);
                // bits 7, 8 count down by one
                let down = ((f >> 7).wrapping_sub(1) << 7 ^ f) & 0x180 ^ f;
                h.write(w + 0x36, down);
            }
        }
        if clear != 0 {
            let f = h.read::<u16>(w + 0x36);
            h.write(w + 0x36, f & 0xefff);
        }
    }
}

/// `AtelUpdateFilePath()` (`0x00879f00`): the event file directory for the
/// event mode (`AtelGetEventMode`: 0 map, 2 master PC, 3 sound, 4 motion,
/// 5 light, else the plain event directory) and the language (`TkGetLang`:
/// 0 jp, 1 us, 2 fr, 3 es, 4 de, 5 it, `0x12` uk), stored by
/// `Atel_SetEventPathLang_00879430`. An unknown language takes the mode's
/// base directory (`/ffx/proj/event/map/` for mode 0, else
/// `/ffx/proj/event/jp/`). Returns the path (an `.rdata` string).
pub fn atel_update_file_path(h: &mut impl ffx_bridge::Host) -> u32 {
    // by language jp, us, fr, es, de, it, uk
    const PATHS: [[u32; 7]; 6] = [
        // /ffx/proj/event/map/<lang>/
        [
            0x00b5_c1b4,
            0x00b5_c1cc,
            0x00b5_c1fc,
            0x00b5_c22c,
            0x00b5_c1e4,
            0x00b5_c214,
            0x00b5_c244,
        ],
        // /ffx/proj/event/<lang>/
        [
            0x00b5_c188,
            0x00b5_c110,
            0x00b5_c138,
            0x00b5_c160,
            0x00b5_c124,
            0x00b5_c14c,
            0x00b5_c174,
        ],
        // /ffx/master/<lang>pc/event/
        [
            0x00b5_c25c,
            0x00b5_c274,
            0x00b5_c2a4,
            0x00b5_c2d4,
            0x00b5_c28c,
            0x00b5_c2bc,
            0x00b5_c2ec,
        ],
        // /ffx/proj/event/sound/<lang>/
        [
            0x00b5_c304,
            0x00b5_c320,
            0x00b5_c358,
            0x00b5_c390,
            0x00b5_c33c,
            0x00b5_c374,
            0x00b5_c3ac,
        ],
        // /ffx/proj/event/motion/<lang>/
        [
            0x00b5_c3c8,
            0x00b5_c3e4,
            0x00b5_c41c,
            0x00b5_c454,
            0x00b5_c400,
            0x00b5_c438,
            0x00b5_c470,
        ],
        // /ffx/proj/event/light/<lang>/
        [
            0x00b5_c48c,
            0x00b5_c4a8,
            0x00b5_c4e0,
            0x00b5_c518,
            0x00b5_c4c4,
            0x00b5_c4fc,
            0x00b5_c534,
        ],
    ];
    let mode = host_call(h, 0x0087_3aa0, &[]);
    let (row, base) = match mode {
        0 => (0, 0x00b5_c19c),
        2..=5 => (mode as usize, 0x00b5_c188),
        _ => (1, 0x00b5_c188),
    };
    let lang = host_call(h, 0x0088_7d00, &[]);
    let path = match lang {
        0..=5 => PATHS[row][lang as usize],
        0x12 => PATHS[row][6],
        _ => base,
    };
    let path = h.rebase(path);
    host_call(h, 0x0087_9430, &[path]);
    path
}

/// `g_AtelPauseFlag` (`0x01327138`) and the pause delay after it.
const PAUSE_FLAG: u32 = 0x0132_7138;

/// `Atel_CheckPause_00873620()` (name ours; caller `0x00872bd0`): with
/// ctrl work 0 current, decides whether START opens the pause menu. The
/// delay (`g_AtelPauseFlag + 4`) counts down; START (`0x800`) held and
/// newly pressed with the delay run out sets the pause flag. Every
/// blocker clears the flag and restarts the delay at 12 (0 for no actors
/// and for `+0x210` bit 3 clear): ctx bit 0 or 3 clear (bit 0 also
/// forbids the menu), a movie (`movie_run_check`), `0x0086d940` or ctx
/// `+3` bits `0x3`, a busy effect (`Atel_EffectIdlePoll(0)` = 0), ctx `+3`
/// bit 7 or `+1` bit 2, no actors, overlay status 1..3, `+0x210` bit 4 or
/// 3 clear, `+0x210` bit 1. Without blockers or a pause, an HDD error
/// (`get_hdd_err`, not yet flagged) pauses with the error flag. When
/// `Movie_IsBusy` reports 1 and the TGS flag is clear, a pause opens the
/// menu (`Sg_MenuSetReserve(0x41000000 | hdd)`, ctx `+0x210` bit 2,
/// `0x013270f4` = 1); then, and whenever `Movie_IsBusy` is 0, the flag
/// and delay reset. The ctrl work is put back; returns its `EAX`.
pub fn atel_check_pause(h: &mut impl ffx_bridge::Host) -> u32 {
    let pad = host_call(h, 0x0086_af70, &[]) & 0xfff;
    let pad_t = host_call(h, 0x0086_af30, &[]) & 0xfff;
    let old = host_call(h, 0x0086_2d30, &[0]);
    let c = host_call(h, 0x0086_aff0, &[]);
    let flag = h.rebase(PAUSE_FLAG);
    let delay = flag + 4;
    let block = |h: &mut _, d: u32| {
        ffx_bridge::Mem::write(h, flag, 0_u32);
        ffx_bridge::Mem::write(h, delay, d);
    };
    let f = h.read::<u32>(c + 0x210);
    h.write(c + 0x210, f & !4);
    let mut d = h.read::<u32>(delay);
    if d != 0 {
        d -= 1;
        h.write(delay, d);
    }
    let mut hdd = 0;
    let b0 = h.read::<u8>(c);
    let mut ok = true;
    if b0 & 1 == 0 {
        d = 0xc;
        block(h, 0xc);
        ok = false;
    }
    if pad == 0x800 && pad_t == 0x800 && d == 0 {
        h.write(flag, 1_u32);
    }
    if b0 & 8 == 0 {
        block(h, 0xc);
        ok = false;
    }
    if host_call(h, 0x0076_f060, &[]) != 0 {
        block(h, 0xc);
        ok = false;
    }
    if host_call(h, 0x0086_d940, &[]) != 0 || h.read::<u8>(c + 3) & 3 != 0 {
        block(h, 0xc);
        ok = false;
    }
    if host_call(h, 0x0086_33a0, &[0]) == 0 {
        block(h, 0xc);
        ok = false;
    }
    if h.read::<u8>(c + 3) & 0x80 != 0 || h.read::<u8>(c + 1) & 4 != 0 {
        block(h, 0xc);
        ok = false;
    }
    if host_call(h, 0x0086_c510, &[]) == 0 {
        block(h, 0);
        ok = false;
    }
    if host_call(h, 0x0086_c200, &[]).wrapping_sub(1) <= 2 {
        block(h, 0xc);
        ok = false;
    }
    let g = h.read::<u32>(c + 0x210);
    if g & 0x10 == 0 {
        block(h, 0xc);
        ok = false;
    }
    if g & 8 == 0 {
        block(h, 0);
        ok = false;
    }
    if g & 2 != 0 {
        block(h, 0xc);
        ok = false;
    } else if ok
        && h.read::<u32>(flag) == 0
        && host_call(h, 0x0088_d5f0, &[]) == 0
        && host_call(h, 0x0077_7e80, &[]) != 0
    {
        hdd = 1;
        h.write(flag, 1_u32);
        host_call(h, 0x0088_eb70, &[1]);
    }
    let menu = host_call(h, 0x0088_d6d0, &[]) == 0 && ok;
    if host_call(h, 0x0076_ed10, &[]) != 0 {
        if !menu || h.read::<u32>(flag) == 0 {
            return host_call(h, 0x0086_f730, &[old]);
        }
        host_call(h, 0x0082_1750, &[hdd | 0x4100_0000]);
        let f = h.read::<u32>(c + 0x210);
        h.write(c + 0x210, f | 4);
        h.write(h.rebase(0x0132_70f4), 1_u32);
    }
    h.write(delay, 0xc_u32);
    h.write(flag, 0_u32);
    host_call(h, 0x0086_f730, &[old])
}

/// `AtelInitTotal()` (`0x0086d6d0`; caller `TkInit`): the script system's
/// reset. The scale pair 2, 2; the callback queue cleared; shop ratio 100;
/// the four in-water words -1; pause, event mode `0xff`, the window bank,
/// the records, the basic-work cache, `0x00874830`, the BG page, the
/// treasure flag, the Al Bhed dictionary, the forced battle map (-1), the
/// bind masks, the separates queue; half-turn angle 179; vibration,
/// expand FTC 0, music -1, movie state, `TkMenuDummy`; overlay status,
/// error and `0x01327078` 0; the sync-V callback 2 = `0x0086ca20`; the
/// battle-end status `0x10001` and effect state 0, `0x01327070`/`74` 0;
/// native tables 0..15 empty, then groups 1, 0, 12, 6, 9, 5, 4, 11, 7, 8,
/// 13 set; game speed 1; the seven ctrl works' bit 0 and `+0x48` cleared
/// and ctrl work 0 current; `0x00873ba0`, `AtelInitCalcSound` and
/// `Atel_ParseHex("0x0")`, whose `EAX` it returns.
pub fn atel_init_total(h: &mut impl ffx_bridge::Host) -> u32 {
    let two = 2.0_f32.to_bits();
    for at in [0x0132_5b98, 0x0132_5b9c] {
        h.write(h.rebase(at), two);
    }
    host_call(h, 0x008b_3cf0, &[]);
    h.write(h.rebase(0x0132_6b6c), 100_u32);
    for k in 0..4 {
        h.write(h.rebase(0x0132_6cf0) + 4 * k, u32::MAX);
    }
    host_call(h, 0x0087_2b70, &[]);
    host_call(h, 0x0087_3e40, &[0xff]);
    h.write(h.rebase(MENU_EXEC_STATUS), 0_u8);
    host_call(h, 0x0087_4fb0, &[]);
    h.write(h.rebase(BASIC_WORK_CACHE), 0_u32);
    host_call(h, 0x0087_4830, &[]);
    host_call(h, 0x0087_2b90, &[]);
    host_call(h, 0x0087_2b60, &[]);
    h.write(h.rebase(0x0132_5b90), 0_u32);
    h.write(h.rebase(0x0132_5b94), 0_u32);
    h.write(h.rebase(FORCE_BATTLE_MAP), u32::MAX);
    host_call(h, 0x0086_0f50, &[]);
    h.write(h.rebase(SEP_QUEUE_COUNT), 0_u32);
    host_call(h, 0x0087_3e50, &[179.0_f32.to_bits()]);
    host_call(h, 0x0087_5e80, &[]);
    host_call(h, 0x0087_3cf0, &[0]);
    host_call(h, 0x0087_3d00, &[u32::MAX]);
    host_call(h, 0x0087_3c00, &[]);
    host_call(h, 0x008a_9710, &[]);
    h.write(h.rebase(0x0132_6b80), 0_u8);
    h.write(h.rebase(0x0132_70d8), 0_u32);
    h.write(h.rebase(0x0132_7078), 0_u32);
    let sync = h.rebase(0x0086_ca20);
    host_call(h, 0x0088_8790, &[2, sync]);
    h.write(h.rebase(BATTLE_END_STATUS), 0x0001_0001_u32);
    h.write(h.rebase(0x0132_6b58), 0_u32);
    h.write(h.rebase(0x0132_7070), 0_u32);
    h.write(h.rebase(0x0132_7074), 0_u32);
    for group in 0..0x10 {
        host_call(h, 0x0087_7880, &[group, 0]);
    }
    for (group, table) in [
        (1, 0x00c5_2be0),
        (0, 0x00c5_0050),
        (0xc, 0x00c5_2dd8),
        (6, 0x00c4_3998),
        (9, 0x00c5_d8c0),
        (5, 0x00c8_91f8),
        (4, 0x00c8_8d88),
        (0xb, 0x00c4_0e30),
        (7, 0x00c4_2628),
        (8, 0x00c5_dc90),
        (0xd, 0x00c8_5eb0),
    ] {
        let table = h.rebase(table);
        host_call(h, 0x0087_7880, &[group, table]);
    }
    let works = h.rebase(CTRL_WORKS);
    let c = |i: u32| works + i * 0x238;
    for i in 0..7 {
        let f = h.read::<u8>(c(i));
        h.write(c(i), f & 0xfe);
    }
    h.write(h.rebase(0x0132_6bd8), 1.0_f32.to_bits());
    for i in 0..7 {
        h.write(c(i) + 0x48, 0_u32);
    }
    h.write(h.rebase(SCRIPT_CONTEXT), works);
    host_call(h, 0x0087_3ba0, &[]);
    host_call(h, 0x0087_7420, &[]);
    let zero = h.rebase(0x00b5_bd9c);
    host_call(h, 0x0087_7660, &[zero])
}

/// Whether the `f32` at `a` is below the one at `b` (`fcompp` on
/// `fld b; fld a`: false for NaN).
fn below_f32(m: &impl Mem, a: u32, b: u32) -> bool {
    x87::load(m.read(a)) < x87::load(m.read(b))
}

/// `fld m32`/`fstp m32` of `at`'s word: the value as passed.
fn arg_f32(m: &impl Mem, at: u32) -> u32 {
    x87::store(x87::load(m.read(at)))
}

/// `Atel_MoveStep_00870540(worker, level)` (name ours; caller
/// `0x008666e0`): one frame of the level's move (`level +0x44`, mode `u16
/// +2`) for the worker's character. A bound character (`+0xaa` 1, `+0x9c`)
/// takes the position block's speed (`+0xc`, 0 for mode 0) through
/// `Ch_SetSp`, after, by `Ch_GetFieldMode` and the move kind (`+2 &
/// 0x7f`), a vertical part (`Ch_SetVelY`: sin of `+0x24` times the speed,
/// the speed times its cos, for field mode 2 and kinds 2, 4; for kinds 5,
/// 6 cos times speed, its sin times 10, the speed times 10) or the
/// block's position (`TkCh_SetPos`, speed 0, kinds 7, 9). Then by kind:
/// 0 puts the bound character at the move's target (`+0x28`, mode bit
/// 11, which it clears; bindings 4, 5 `AtelSetBindObject`); 5, 6 stop
/// when the time (`+0xc`) is below the length (`+8`), clearing the mode
/// and `+0x14`; 7, 9 the same, putting the target into the position
/// block; the rest stop inside the radius `+0x38` around the target
/// (`Atel_InRadiusXYZ` with mode bit 10, else `Atel_InRadiusXZ`), and mode
/// bit 12 stops a move kind left (keeping bit 11).
pub fn atel_move_step<H: super::run::NativeHost>(h: &mut H, w: u32, level: u32) {
    let p = pos_block(h, w);
    let mv = h.read::<u32>(level + 0x44);
    let mut sp = if h.read::<u16>(mv + 2) == 0 {
        0
    } else {
        arg_f32(h, p + 0xc)
    };
    let chr = h.read::<u32>(w + 0x9c);
    if h.read::<u8>(w + 0xaa) == 1 && chr != 0 {
        let sin = h.rebase(0x0094_98c8);
        let cos = h.rebase(0x0094_98c2);
        let angle = |h: &H| x87::load(h.read(p + 0x24));
        let mode = host_call(h, 0x0082_ab90, &[chr]);
        let kind = h.read::<u16>(mv + 2) & 0x7f;
        let step = match (mode, kind) {
            (2, _) | (0, 2 | 4) => 1,
            (0, 5 | 6) => 2,
            (0, 7 | 9) => 3,
            _ => 0,
        };
        match step {
            1 => {
                let s = x87::store(h.call_intrinsic1(sin, angle(h)));
                let vy = x87::store(x87::mul(x87::load(s), x87::load(sp)));
                let c = x87::store(h.call_intrinsic1(cos, angle(h)));
                sp = x87::store(x87::mul(x87::load(c), x87::load(sp)));
                host_call(h, 0x0082_b920, &[chr, vy]);
            }
            2 => {
                let c = x87::store(h.call_intrinsic1(cos, angle(h)));
                let u = x87::store(x87::mul(x87::load(c), x87::load(sp)));
                let s = x87::store(h.call_intrinsic1(sin, angle(h)));
                let ten = h.read::<f64>(h.rebase(0x00b9_22b0));
                let vy = x87::store(x87::mul(x87::mul(x87::load(s), x87::load(u)), ten));
                sp = x87::store(x87::mul(x87::load(u), ten));
                host_call(h, 0x0082_b920, &[chr, vy]);
            }
            3 => {
                sp = 0;
                let (x, y, z) = (arg_f32(h, p), arg_f32(h, p + 4), arg_f32(h, p + 8));
                host_call(h, 0x0087_2310, &[chr, x, y, z]);
            }
            _ => {}
        }
        let chr = h.read::<u32>(w + 0x9c);
        host_call(h, 0x0082_b840, &[chr, sp]);
    }
    move_step_end(h, w, mv, p);
}

/// The second half of [`atel_move_step`]: the move's end by its kind.
fn move_step_end(h: &mut impl ffx_bridge::Host, w: u32, mv: u32, p: u32) {
    let f = h.read::<u16>(mv + 2);
    match f & 0x7f {
        0 => match h.read::<u8>(w + 0xaa) {
            1 => {
                let chr = h.read::<u32>(w + 0x9c);
                if chr != 0 && f & 0x800 != 0 {
                    let (x, y, z) = (
                        arg_f32(h, mv + 0x28),
                        arg_f32(h, mv + 0x2c),
                        arg_f32(h, mv + 0x30),
                    );
                    host_call(h, 0x0087_2310, &[chr, x, y, z]);
                    let f = h.read::<u16>(mv + 2);
                    h.write(mv + 2, f & 0xf7ff);
                }
            }
            4 | 5 => {
                host_call(h, 0x0086_f4c0, &[w]);
            }
            _ => {}
        },
        5 | 6 => {
            if below_f32(h, mv + 0xc, mv + 8) {
                host_call(h, 0x0086_3e10, &[w, u32::MAX]);
                h.write(mv + 2, 0_u16);
                h.write(mv + 0x14, 0_u32);
            }
        }
        7 | 9 => {
            if below_f32(h, mv + 0xc, mv + 8) {
                host_call(h, 0x0086_3e10, &[w, u32::MAX]);
                h.write(mv + 2, 0_u16);
                for k in [0, 4, 8] {
                    let v = arg_f32(h, mv + 0x28 + k);
                    h.write(p + k, v);
                }
            }
        }
        _ => {
            let r = arg_f32(h, mv + 0x38);
            let (x, y, z) = (
                arg_f32(h, mv + 0x28),
                arg_f32(h, mv + 0x2c),
                arg_f32(h, mv + 0x30),
            );
            let test = if f & 0x400 != 0 {
                0x0086_3440
            } else {
                0x0086_34b0
            };
            if host_call(h, test, &[r, w, x, y, z]) != 0 {
                if h.read::<u16>(mv + 2) & 0x7f != 0 {
                    host_call(h, 0x0086_3e10, &[w, u32::MAX]);
                }
                h.write(mv + 2, 0_u16);
            }
            let f = h.read::<u16>(mv + 2);
            if f & 0x1000 != 0 && f & 0x7f != 0 {
                host_call(h, 0x0086_3e10, &[w, u32::MAX]);
                let f = h.read::<u16>(mv + 2);
                h.write(mv + 2, f & 0x800);
            }
        }
    }
}

/// `AtelBindElemOff(worker, i)` (`0x00862780`): frees bound element `i`
/// (`+0xaf4 + 4i`: `Ch_Dispose` when `Ch_CheckHandle` says it is valid,
/// then 0); for element 0 of a binding-7 worker (`+0xa8`) the character's
/// model part 1 shows again (`Ch_ModelSetHide(chr, 1, 0)`). Clears bit
/// `i` of `+0xb14` (a 16-bit shift: from 16 up nothing). Returns the mask
/// it applied with `and`.
pub fn atel_bind_elem_off(h: &mut impl ffx_bridge::Host, w: u32, i: u32) -> u32 {
    let slot = w.wrapping_add(i.wrapping_mul(4)).wrapping_add(0xaf4);
    let e = h.read::<u32>(slot);
    if e != 0 {
        if host_call(h, 0x0082_61b0, &[e]) == 0 {
            host_call(h, 0x0082_66f0, &[e]);
        }
        h.write(slot, 0_u32);
    }
    if i == 0 && h.read::<u16>(w + 0xa8) == 7 {
        let chr = h.read::<u32>(w + 0x9c);
        if chr != 0 {
            host_call(h, 0x0082_aea0, &[chr, 1, 0]);
        }
    }
    // `shl ax, cl`: the count taken mod 32, a 16-bit result
    let bit = 1u16.checked_shl(i & 0x1f).unwrap_or(0);
    let mask = !bit;
    let f = h.read::<u16>(w + 0xb14);
    h.write(w + 0xb14, f & mask);
    u32::from(mask)
}

/// `t` clamped to `0..=1` as `0x00862b20` does (`fcom` against 0, then
/// 1; NaN passes), as the `f32` bits it passes on.
fn clamp_unit(t: u32) -> u32 {
    let x = x87::load(t);
    if x < 0.0 {
        0
    } else if 1.0 < x {
        1.0_f32.to_bits()
    } else {
        x87::store(x)
    }
}

/// `Atel_CalcSpline_00862b20(out, curve, t)` (name ours; callers
/// `Native_0099`, `0x00868990`, `0x0087de30`):
/// `TkMtCalcSpline(out, curve + 0x10, curve's count, t in 0..=1)`;
/// returns its `EAX`.
pub fn atel_calc_spline(h: &mut impl ffx_bridge::Host, out: u32, curve: u32, t: u32) -> u32 {
    let n = h.read::<u32>(curve);
    host_call(h, 0x0088_a4a0, &[out, curve + 0x10, n, clamp_unit(t)])
}

/// `Atel_CalcSpline3_00862b70(out, curve, t)` (name ours; callers
/// `Native_009c`, `0x00868990`, `0x0087de30`): [`atel_calc_spline`] with
/// `TkMtCalcSpline3`.
pub fn atel_calc_spline3(h: &mut impl ffx_bridge::Host, out: u32, curve: u32, t: u32) -> u32 {
    let n = h.read::<u32>(curve);
    host_call(h, 0x0088_a4d0, &[out, curve + 0x10, n, clamp_unit(t)])
}

/// `Atel_ExecCtrlWorks_008688f0(from, to)` (name ours; callers `AtelExec`,
/// `AtelMenuExec`): the pad and window update (`0x00871d70`), then ctrl
/// works `from..to` in turn current (`g_ScriptContext`, `+8` of it the
/// index): a work in state (`+8`) 0..2 runs its callback (`+0x4c`, when
/// bit 0 is set; bit 3 of the then current work counts `+0x1f4` up), and
/// states 1 and 2 become 3. The old ctrl work back, the window status
/// update; returns its `EAX`.
pub fn atel_exec_ctrl_works(h: &mut impl ffx_bridge::Host, from: u32, to: u32) -> u32 {
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let saved = h.read::<u32>(ctx);
    host_call(h, 0x0087_1d70, &[]);
    let works = h.rebase(CTRL_WORKS);
    let mut i = from;
    while i.cast_signed() < to.cast_signed() {
        let c = works.wrapping_add(i.wrapping_mul(0x238));
        h.write(ctx, c);
        h.write(ctx + 8, i);
        if h.read::<u8>(c + 8) <= 2 {
            let mut e = c;
            if h.read::<u8>(c) & 1 != 0 {
                let f = h.read::<u32>(c + 0x4c);
                h.call(f, &[c]);
                e = h.read::<u32>(ctx);
                if h.read::<u8>(e) & 8 != 0 {
                    let n = h.read::<u32>(e + 0x1f4);
                    h.write(e + 0x1f4, n.wrapping_add(1));
                }
            }
            if matches!(h.read::<u8>(e + 8), 1 | 2) {
                h.write(e + 8, 3_u8);
            }
        }
        i = i.wrapping_add(1);
    }
    h.write(ctx, saved);
    host_call(h, 0x0087_1f30, &[])
}

/// `Atel_CheckLineCrosses_00868020(worker, x)` (name ours; caller
/// `0x008666e0`): `Atel_CheckLineCross_008680a0` for the worker's own
/// target (the position block's `+0x28`, no record), then for each of the
/// eight line records at `+0x2e4` (8 bytes: target, a non-zero `+2`) with
/// a target. Returns the `EAX` the last step leaves.
pub fn atel_check_line_crosses(h: &mut impl ffx_bridge::Host, w: u32, x: u32) -> u32 {
    let p = pos_block(h, w);
    let id = u32::from(h.read::<u16>(p.wrapping_add(0x28)));
    let mut eax = host_call(h, 0x0086_80a0, &[w, x, id, 0]);
    let list = h.read::<u32>(w + 0x2e4);
    if list == 0 {
        return eax;
    }
    for k in 0..8 {
        let rec = list + 8 * k;
        let id = u32::from(h.read::<u16>(rec));
        eax = id;
        if id != 0xffff && h.read::<u16>(rec + 2) != 0 {
            eax = host_call(h, 0x0086_80a0, &[w, x, id, rec]);
        }
    }
    eax
}

/// `AtelEventGetFilePath2(dirs, sub, alt, name, ext)` (`0x008795b0`):
/// builds a path in `0x01328858` and returns it. `name` and `alt` are cut
/// at their first `.`; the directory part comes from `alt`, or `name`
/// without one. With `dirs` the path starts with `AtelUpdateFilePath()`,
/// `sub` and `/xy/<base>/` (`xy` the base's first two characters); then
/// the cut name and `ext`. `frame` holds the original's three 256-byte
/// locals (the cut `alt`, the cut `name`, the directory part).
pub fn atel_event_get_file_path2(h: &mut impl ffx_bridge::Host, args: [u32; 5], frame: u32) -> u32 {
    let [dirs, sub, alt, name, ext] = args;
    let out = h.rebase(0x0132_8858);
    let (cut_alt, cut_name, dir) = (frame, frame + 0x100, frame + 0x200);
    h.write(out, 0_u8);
    let cut = |h: &mut _, from: u32, to: u32| {
        let mut k = 0;
        loop {
            let c = ffx_bridge::Mem::read::<u8>(h, from + k);
            if c == 0 || c == b'.' {
                break;
            }
            ffx_bridge::Mem::write(h, to + k, c);
            k += 1;
        }
        ffx_bridge::Mem::write(h, to + k, 0_u8);
    };
    cut(h, name, cut_name);
    let base = if alt == 0 {
        cut_name
    } else {
        cut(h, alt, cut_alt);
        cut_alt
    };
    let path = host_call(h, 0x0087_9f00, &[]);
    let slash = h.rebase(0x00b8_f88c);
    if dirs != 0 {
        host_call(h, 0x0087_1980, &[out, path]);
        host_call(h, 0x0087_1950, &[out, sub]);
        let (a, b) = (h.read::<u8>(base), h.read::<u8>(base + 1));
        h.write(dir + 1, a);
        h.write(dir + 2, b);
        h.write(dir, b'/');
        h.write(dir + 3, 0_u8);
        host_call(h, 0x0087_1950, &[dir, slash]);
        host_call(h, 0x0087_1950, &[dir, base]);
        host_call(h, 0x0087_1950, &[dir, slash]);
        host_call(h, 0x0087_1950, &[out, dir]);
    }
    host_call(h, 0x0087_1950, &[out, cut_name]);
    host_call(h, 0x0087_1950, &[out, ext]);
    out
}

/// The offsets of a vector's x, y, z.
const XYZ: [u32; 3] = [0, 4, 8];

/// `(a² + b²) + c²` of three `f32` words, in the original's order.
fn sum_sq(a: u32, b: u32, c: u32) -> f64 {
    let sq = |v: u32| x87::mul(x87::load(v), x87::load(v));
    x87::add(x87::add(sq(a), sq(b)), sq(c))
}

/// `spline_get_near_point_init(spline, x, y, z, cubic)` (`0x00a44420`,
/// Fahrenheit's name; callers `MsCameraMoveStep_007bcab0`,
/// `MsCameraNearPoints_007c0d40`, `0x0093f900`): walks the spline (`+0`
/// count, points from `+0x10`; `TkMtCalcSpline3` when `cubic`) in steps of
/// `1 / (10 n)` from 0 while `t` stays at most 1, summing the chord
/// lengths into `g_SplineNearSamples` and keeping the `t` of the sample
/// nearest `(x, y, z)` (squared distance, starting from 999999) in
/// `g_SplineNearT`; then `spline_get_near_point` refines it (its `ST0`
/// dropped, the point into `frame + 0x20..0x2c`) and the length goes to
/// `scePrintf` (its callers read no `EAX`). `frame` holds the original's
/// locals: the previous point (`+0`), the next (`+0x10`), the offsets
/// (`+0x20`). `fcall` calls an original that returns in `ST0` (rebased
/// address, eight words).
pub fn spline_get_near_point_init<H: ffx_bridge::Host>(
    h: &mut H,
    spline: u32,
    p: [u32; 3],
    cubic: u32,
    frame: u32,
    mut fcall: impl FnMut(&mut H, u32, &[u32]) -> f64,
) {
    let samples = h.rebase(0x01a8_5f34);
    let near_t = h.rebase(0x01a8_5f30);
    let calc = if cubic == 0 { 0x0088_a4a0 } else { 0x0088_a4d0 };
    let (prev, next, off) = (frame, frame + 0x10, frame + 0x20);
    h.write(samples, 0_u32);
    let mut best = x87::load(h.read(h.rebase(0x00b7_af48)));
    let n = h.read::<u32>(spline);
    let points = spline + 0x10;
    let count = x87::store(f64::from(n.wrapping_mul(10).cast_signed()));
    let step = x87::store(x87::div(1.0, x87::load(count)));
    let mut t = 0_u32;
    h.write(near_t, t);
    host_call(h, calc, &[prev, points, n, t]);
    loop {
        host_call(h, calc, &[next, points, n, t]);
        let q: [u32; 3] = XYZ.map(|o| h.read(next + o));
        let d: [u32; 3] = core::array::from_fn(|k| {
            let a = x87::load(q[k]);
            x87::store(x87::sub(a, x87::load(h.read(prev + XYZ[k]))))
        });
        let len = x87::store(x87::sqrt(sum_sq(d[0], d[1], d[2])));
        let total = x87::store(x87::add(x87::load(len), x87::load(h.read(samples))));
        h.write(samples, total);
        let e: [u32; 3] =
            core::array::from_fn(|k| x87::store(x87::sub(x87::load(q[k]), x87::load(p[k]))));
        for (k, &v) in (0u32..).zip(&e) {
            h.write(off + 4 * k, v);
        }
        let dist = x87::load(x87::store(sum_sq(e[0], e[1], e[2])));
        if best > dist {
            best = dist;
            h.write(near_t, x87::store(x87::load(t)));
        }
        for (k, &v) in (0u32..).zip(&q) {
            h.write(prev + 4 * k, x87::store(x87::load(v)));
        }
        t = x87::store(x87::add(x87::load(t), x87::load(step)));
        if 1.0 < x87::load(t) {
            break;
        }
    }
    let [x, y, z] = p.map(|v| x87::store(x87::load(v)));
    let near = h.rebase(0x00a4_4120);
    fcall(h, near, &[spline, x, y, z, off, off + 4, off + 8, cubic]);
    let total = x87::load(h.read(samples)).to_bits();
    let fmt = h.rebase(0x00b7_af28);
    #[allow(clippy::cast_possible_truncation)] // the two halves
    let (lo, hi) = (total as u32, (total >> 32) as u32);
    host_call(h, 0x0062_fbf0, &[fmt, lo, hi]);
}

/// `AtelExec()` (`0x008666d0`, Fahrenheit's name; caller `0x00872bd0`):
/// [`atel_exec_ctrl_works`] over ctrl works 0..6; `EAX` its result.
pub fn atel_exec(h: &mut impl ffx_bridge::Host) -> u32 {
    atel_exec_ctrl_works(h, 0, 6)
}

/// `AtelMenuExec()` (`0x0086dd60`, Fahrenheit's name; caller
/// `AtelExecMenuCtrl`): [`atel_exec_ctrl_works`]
/// over ctrl work 6, the menu's; `EAX` its result.
pub fn atel_menu_exec(h: &mut impl ffx_bridge::Host) -> u32 {
    atel_exec_ctrl_works(h, 6, 7)
}

/// `AtelLoadSourceData(slot, arg)` (`0x00879a70`, Fahrenheit's name;
/// caller `AtelEventSetUp`): clears the slot's eight sizes
/// (`0x01328778 + 0x20 * slot`), then for each of the first (at most 7)
/// names in the event's file table (header `0x01328598[slot]`, table at
/// `+0x40`: its first word is the byte length, the words are offsets from
/// the table; the first names the directory) builds `debug/<dir>/<name>.src`
/// (`AtelEventGetFilePath2`) unless the name starts with `/`, prints it
/// (`TK:EVENTNAME`) and loads it (`0x00879b60`, the PS2 host-file loader,
/// through the game). `EAX` 1, or 0 for an empty table.
pub fn atel_load_source_data(h: &mut impl ffx_bridge::Host, slot: u32, arg: u32) -> u32 {
    let hdr = h.read::<u32>(h.rebase(0x0132_8598).wrapping_add(slot.wrapping_mul(4)));
    let sizes = h.rebase(0x0132_8778).wrapping_add(slot.wrapping_mul(0x20));
    for k in 0..8 {
        h.write(sizes + 4 * k, 0_u32);
    }
    let table = |h: &mut _| hdr.wrapping_add(Mem::read::<u32>(h, hdr + 0x40));
    let t = table(h);
    let count = (h.read::<u32>(t) >> 2).min(7);
    if count == 0 {
        return 0;
    }
    for i in 0..count {
        let t = table(h);
        let mut name = h.read::<u32>(t + 4 * i).wrapping_add(t);
        let dir = h.read::<u32>(t).wrapping_add(t);
        if h.read::<u8>(name) != b'/' {
            let (debug, src) = (h.rebase(0x00b5_9148), h.rebase(0x00b5_c56c));
            name = host_call(h, 0x0087_95b0, &[1, debug, dir, name, src]);
        }
        let fmt = h.rebase(0x00b5_c574);
        host_call(h, 0x0062_f500, &[fmt, i, name]);
        host_call(h, 0x0087_9b60, &[slot, arg, i, name]);
    }
    1
}

/// `AtelInitBasicWork(setup, clear)` (`0x0086cb00`, Fahrenheit's name;
/// callers `AtelActorSetUp2`, `AtelBattleSetUp`, `AtelCommonSetUp`). The
/// setup record holds eight words: active, worker index, three level
/// values, header, code base, `+0x1c`. The worker (`AtelGetBasicWork`)
/// gets header `+0`, code `+4`, `+8`, its index (`u16 +0x2e`) and
/// `+0x0c..+0x28` zeroed; an active one is reset in full: its levels
/// (`Atel_InitWorkerLevels_0086d300` with the three values, the low two
/// also in byte `+0xae`), the flags (`+0x36`: battle bit `0x40` from the
/// ctrl work's `+0x1a`, `0x800`; `+0x38`), scale 1 (`+0x94`), `+0xb4`
/// -10000, `+0x40` -1, the move block (`+0x558`, types 5/6 `+0x284`; type 1
/// also its `+0xae0..+0xb54` part) for types 1, 2, 3, 5, 6, the `+0xab`
/// mask by type, the free signals (`Atel_InitFreeSignals_0086d510`), the
/// variables zeroed when `clear` (the private area at `+0xb8`, else the
/// code's at header `+0x2c`, header `+0x10` bytes), then signals 7 and 0
/// to itself (`AtelRegistSignalMain`) and a decode at level 10. Its `EAX`
/// (the index for an inactive worker, else `AtelDecodeSignal`'s leftover)
/// is read by no caller.
#[allow(clippy::too_many_lines)] // one store per field, as the original
pub fn atel_init_basic_work(h: &mut impl ffx_bridge::Host, setup: u32, clear: u32) {
    let mut rec = [0_u32; 8];
    for (k, v) in (0_u32..).zip(&mut rec) {
        *v = h.read(setup + 4 * k);
    }
    let [active, index, l0, l1, l2, header, code, x1c] = rec;
    let w = super::atel_get_basic_work(h, index.cast_signed());
    h.write(w + 4, code);
    h.write(w, header);
    h.write(w + 8, x1c);
    write_u16(h, w + 0x2e, index);
    for o in (0x0c..=0x28).step_by(4) {
        h.write(w + o, 0_u32);
    }
    if active == 0 {
        return;
    }
    let (zero, one) = (0_u32, 1.0_f32.to_bits());
    h.write(w + 0xb8, 0_u32);
    h.write(w + 0xc4, 0_u32);
    #[allow(clippy::cast_possible_truncation)] // two nibbles
    h.write(w + 0xae, ((l1 << 4) | (l0 & 0xf)) as u8);
    for o in (0x48..=0x7c).step_by(4) {
        h.write(w + o, zero);
    }
    atel_init_worker_levels(h, w, l0, l1, l2);
    let w2 = super::atel_get_basic_work(h, index.cast_signed());
    h.write(w2 + 0x94, one);
    let flags = u32::from(h.read::<u16>(w + 0x36)) & 0xffc6 | 6;
    write_u16(
        h,
        w + 0x38,
        u32::from(h.read::<u16>(w + 0x38)) & 0xe020 | 0x20,
    );
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    for o in [0x84, 0x80, 0x8c, 0x88] {
        h.write(w + o, 0_u32);
    }
    h.write(w + 0x32, 0x040e_0000_u32);
    h.write(w + 0x90, 0_u32);
    write_u16(h, w + 0x36, flags);
    h.write(w + 0xbc, 0_u32);
    let flags = if h.read::<u16>(ctx + 0x1a) < 2 {
        flags & 0xffbf
    } else {
        flags | 0x40
    };
    h.write(w + 0x94, one);
    write_u16(h, w + 0x36, flags);
    h.write(w + 0x98, zero);
    h.write(w + 0xb4, (-10000.0_f32).to_bits());
    write_u16(h, w + 0x36, flags & 0x7f | 0x800);
    write_u16(h, w + 0xa8, 0);
    h.write(w + 0x9c, 0_u32);
    h.write(w + 0x3c, 0_u32);
    h.write(w + 0xad, 0_u8);
    h.write(w + 0x44, 0_u32);
    h.write(w + 0xaf, 0xf_u8);
    h.write(w + 0xaa, 0_u8);
    h.write(w + 0x40, u32::MAX);
    let kind = h.read::<u8>(h.read::<u32>(w));
    let mv = match kind {
        4 => 0,
        5 | 6 => w + 0x284,
        _ => w + 0x558,
    };
    if kind == 4 {
        write_u16(h, w + 0x34, 0x48e);
    }
    if kind == 1 {
        for k in 0..2 {
            h.write(w + 0xaf4 + 4 * k, 0_u32);
            h.write(w + 0xafc + 4 * k, 0_u32);
            write_u16(h, w + 0xb04 + 2 * k, 0);
            h.write(w + 0xb08 + 4 * k, 1_u32);
        }
        write_u16(h, w + 0xb14, 0);
        write_u16(h, w + 0x34, u32::from(h.read::<u16>(w + 0x34)) | 0x80);
        for o in (0xb18..=0xb54).step_by(4) {
            h.write(w + o, zero);
        }
    }
    if matches!(kind, 1 | 2 | 3 | 5 | 6) {
        for o in (0..=0x24).step_by(4) {
            h.write(mv + o, zero);
        }
        write_u16(h, mv + 0x28, 0xffff);
        h.write(mv + 0x2c, (-1.0_f32).to_bits());
        for o in [0x30, 0x34, 0x38] {
            h.write(mv + o, zero);
        }
        // pi / 3
        h.write(mv + 0x3c, 0x3f86_0a92_u32);
        h.write(mv + 0x4c, 0x3f86_0a92_u32);
        let [x44, x40, x48, x1c]: [f32; 4] = if kind == 1 {
            [1.0, 2.0, 1.5, 5.0]
        } else {
            [0.0, 0.0, 1.0, -1.0]
        };
        h.write(mv + 0x44, x44.to_bits());
        h.write(mv + 0x40, x40.to_bits());
        h.write(mv + 0x48, x48.to_bits());
        h.write(mv + 0x1c, x1c.to_bits());
        if kind == 1 {
            write_u16(h, w + 0xb16, 10);
            for o in (0xae0..=0xaf0).step_by(4) {
                h.write(w + o, 0_u32);
            }
        }
    }
    match h.read::<u8>(header) {
        1 => h.write(w + 0xab, 0xf7_u8),
        2 | 3 | 5 | 6 => h.write(w + 0xab, 0xff_u8),
        4 => h.write(w + 0xab, 0_u8),
        _ => {}
    }
    atel_init_free_signals(h, w);
    if clear != 0 {
        let hdr = h.read::<u32>(w);
        let (mut at, mut len) = (0_u32, 0_u32);
        let vars = h.read::<u32>(hdr + 0x2c);
        if vars != 0 {
            at = h.read::<u32>(w + 4).wrapping_add(vars);
            len = h.read::<u32>(hdr + 0x10);
        }
        // the private area: always 0 here (stored above, and neither
        // callee since writes it), kept as the original tests it
        let private = h.read::<u32>(w + 0xb8);
        if private != 0 {
            len = h.read::<u32>(hdr + 0x10);
            at = private;
        }
        if at != 0 && len.cast_signed() > 0 {
            for k in 0..len {
                h.write(at.wrapping_add(k), 0_u8);
            }
        }
    }
    let levels = [0x16c, 0x16a, 0x168].map(|o| u32::from(h.read::<u16>(w + o)));
    let mut req = super::signal::Request([0xffff, index, 0, 7, 0, levels[0], levels[1], levels[2]]);
    if index != 0xffff {
        super::signal::atel_regist_signal_main(h, req);
    }
    req.0[4] = 1;
    req.0[3] = 0;
    if index != 0xffff {
        super::signal::atel_regist_signal_main(h, req);
    }
    super::signal::atel_decode_signal(h, index, 10);
}

/// `AtelEventRemoveForBattle()` (`0x00865dc0`, Fahrenheit's name; callers
/// `MsBtlReadSetScene`, native `0xc024`): before a battle, unlocks the
/// separates (`SndSepLock(0)`), `Yn_BattleBegin`, hides the guide map,
/// keeps `yiGetDrawMask` (`0x01326b78`), the effect stick state
/// (`pppFpSaveStat` into the zeroed `0x01326be0`, 0x100 bytes), the lights
/// (`Atel_SaveLights_0086db20`) and the guide map number (`0x01326b68`).
/// When ctrl work 0 is in use it becomes current (`SndStopStream`) and each
/// of its actors loses `+0x34` bit 2; the character-bound ones (`+0xaa` 1)
/// release their model: speed 0 (`Ch_SetSp`, unless `+0x36` bit 6), flags
/// cleared, the system motion groups noted in `+0x36` (`0x2000`/`0x4000`),
/// the hide event kept in `+0xa0` (0 with `+0x38` bit 12), `Ch_SetResHide`
/// (unless `+0x38` bit 11), the four effect sticks reset (`+0xb18`, every
/// 0x10), then the motion groups (`+0x33` bit 1), the two attached models
/// (`+0xaf4`/`+0xaf8`, bits 4 and 5) and with bit 0 the model itself
/// (`+0x9c`) disposed, their data too when `MsBattleGetChrDisposeFlag`.
/// The old ctrl work comes back. `EAX` the guide map number when ctrl work
/// 0 is unused, else the old ctrl work.
#[allow(clippy::too_many_lines)] // the original's three disposal steps
pub fn atel_event_remove_for_battle(h: &mut impl ffx_bridge::Host) -> u32 {
    host_call(h, 0x0088_6dd0, &[0]);
    let dispose_data = host_call(h, 0x0078_1630, &[]);
    host_call(h, 0x0091_9a20, &[]);
    host_call(h, 0x0091_9d70, &[0]);
    let mask = host_call(h, 0x0090_8960, &[]);
    h.write(h.rebase(0x0132_6b78), mask);
    let sticks = h.rebase(0x0132_6be0);
    for k in 0..0x40 {
        h.write(sticks + 4 * k, 0_u32);
    }
    host_call(h, 0x0072_a180, &[sticks]);
    host_call(h, 0x0086_db20, &[]);
    let map = host_call(h, 0x0091_9c70, &[]);
    h.write(h.rebase(0x0132_6b68), map);
    let works = h.rebase(0x0132_5ba0);
    if h.read::<u8>(works) & 1 == 0 {
        return map;
    }
    let ctx = h.rebase(SCRIPT_CONTEXT);
    let saved = h.read::<u32>(ctx);
    let actors = h.read::<u16>(works + 0x0c);
    h.write(ctx, works);
    host_call(h, 0x0088_7780, &[]);
    // `Ch_CheckHandle` 0 means a live model: dispose it, and its data
    let release = |h: &mut _, chr: u32, data: u32| {
        if host_call(h, 0x0082_61b0, &[chr]) == 0 {
            host_call(h, 0x0082_66f0, &[chr]);
        }
        if dispose_data != 0 {
            host_call(h, 0x0082_4f00, &[data, 0]);
        }
    };
    let id = |h: &mut _, w: u32| u32::from(Mem::read::<u16>(h, w + 0xa8));
    let and8 = |h: &mut _, at: u32, m: u8| {
        let v = Mem::read::<u8>(h, at);
        Mem::write(h, at, v & m);
    };
    for i in 0..i32::from(actors) {
        let w = super::atel_get_basic_work(h, i);
        write_u16(h, w + 0x34, u32::from(h.read::<u16>(w + 0x34)) & 0xfffb);
        if h.read::<u8>(w + 0xaa) != 1 {
            continue;
        }
        let chr = h.read::<u32>(w + 0x9c);
        if chr != 0 && h.read::<u8>(w + 0x36) & 0x40 == 0 {
            host_call(h, 0x0082_b840, &[chr, 0]);
        }
        and8(h, w + 0x33, 0xfb);
        let v = h.read::<u32>(w + 0xb10);
        h.write(w + 0xb10, v & !1);
        write_u16(h, w + 0x36, u32::from(h.read::<u16>(w + 0x36)) & 0x9fff);
        h.write(w + 0xa0, 0_u32);
        if h.read::<u32>(w + 0x9c) == 0 {
            continue;
        }
        for (group, bit) in [(0, 0x2000), (2, 0x4000)] {
            let g = id(h, w);
            if host_call(h, 0x0083_6850, &[g, group]) != 0 {
                write_u16(h, w + 0x36, u32::from(h.read::<u16>(w + 0x36)) | bit);
            }
        }
        let chr = h.read::<u32>(w + 0x9c);
        let hide = host_call(h, 0x0082_ac20, &[chr]);
        h.write(w + 0xa0, hide);
        let f38 = h.read::<u16>(w + 0x38);
        if f38 & 0x1000 != 0 {
            h.write(w + 0xa0, 0_u32);
        }
        if f38 & 0x800 == 0 {
            let chr = h.read::<u32>(w + 0x9c);
            host_call(h, 0x0082_b4e0, &[chr, 0]);
        }
        write_u16(h, w + 0x38, u32::from(h.read::<u16>(w + 0x38)) & 0xf7ff);
        for k in 0..4 {
            let stick = u32::from(h.read::<u16>(w + 0xb18 + 0x10 * k)) & 0x7fff;
            if stick != 0 {
                host_call(h, 0x0072_a120, &[stick]);
            }
        }
        // `+0x9c` is tested again before each step, as the original does;
        // none of the calls since the test above can clear it (they are
        // not handed the worker), so these re-tests never fire
        if h.read::<u32>(w + 0x9c) == 0 {
            continue;
        }
        if h.read::<u8>(w + 0x33) & 2 != 0 {
            if h.read::<u8>(w + 0x36) & 0x40 == 0 {
                for group in [0, 2] {
                    let g = id(h, w);
                    if host_call(h, 0x0083_6850, &[g, group]) != 0 {
                        let g = id(h, w);
                        host_call(h, 0x0083_6830, &[g, group]);
                    }
                }
            }
            and8(h, w + 0x33, 0xfd);
        }
        // the two attached models, bits 4 and 5
        for (k, bit) in [(0_u32, 0x10_u8), (1, 0x20)] {
            if h.read::<u32>(w + 0x9c) == 0 {
                break;
            }
            if h.read::<u8>(w + 0x33) & bit != 0 {
                let at = w + 0xaf4 + 4 * k;
                let m = h.read::<u32>(at);
                if h.read::<u8>(w + 0x36) & 0x40 == 0 && m != 0 {
                    let data = u32::from(h.read::<u16>(w + 0xb04 + 2 * k));
                    release(h, m, data);
                    h.write(at, 0_u32);
                }
                and8(h, w + 0x33, !bit);
            }
        }
        if h.read::<u32>(w + 0x9c) != 0
            && h.read::<u8>(w + 0x33) & 1 != 0
            && h.read::<u8>(w + 0x36) & 0x40 == 0
        {
            and8(h, w + 0x33, 0xfe);
            for k in 0..2 {
                let at = w + 0xaf4 + 4 * k;
                let m = h.read::<u32>(at);
                if m != 0 {
                    let data = u32::from(h.read::<u16>(w + 0xb04 + 2 * k));
                    release(h, m, data);
                    h.write(at, 0_u32);
                }
            }
            let chr = h.read::<u32>(w + 0x9c);
            let data = id(h, w);
            release(h, chr, data);
            h.write(w + 0x9c, 0_u32);
        }
    }
    h.write(ctx, saved);
    saved
}

/// The move block of worker `w` (`+0x558`, types 5 and 6 `+0x284`, none
/// for type 4).
fn move_block(m: &impl Mem, w: u32) -> u32 {
    match kind(m, w) {
        4 => 0,
        5 | 6 => w + 0x284,
        _ => w + 0x558,
    }
}

/// `Atel_CheckTouchTalk_00866980(w, _, player)` (name ours; caller
/// `0x008666e0`, per actor): for a character-bound actor (`+0xaa` 1, else
/// `EAX` 0) against the ctrl work's player position (`+0x218..+0x220`).
/// Touch: within the height band (`|dy| < ` move `+0x1c`) and the radius
/// (`LengthSq3(dx, 0, dz) <= (+0x48)²`) it sends itself trigger 3
/// (`AtelRequestTrigger_008764f0(0xffff, id, 3)`, the `EAX` returned);
/// `+0x34` bit 6 says whether it did, `+0x36` bit 0 is cleared. Talk (with
/// `+0xab` bit 0 and the ctrl work's `+0x1e4` 1): within
/// `(ctrl +0x1f0 + move +0x40)²` (squared distance with `dy` only when
/// move `+0x2c` is negative, else the height band `+0x2c` applies), not
/// hidden (`+0x34` bits 5 and 9, `+0x36` bit 3), nearer than the
/// candidate so far (`+0x1ec`, none when `+0x1e8` is `0xffff`) and facing
/// it (`Math_FacingWithin(player move +0x34, +0x3c, ...)`) it becomes the
/// candidate: id into `+0x1e8`, distance into `+0x1ec`. `fcall` calls an
/// original that returns in `ST0` (`Math_LengthSq3`, rebased address).
#[allow(clippy::too_many_lines)] // the two checks
pub fn atel_check_touch_talk<H: ffx_bridge::Host>(
    h: &mut H,
    w: u32,
    player: u32,
    mut fcall: impl FnMut(&mut H, u32, &[u32]) -> f64,
) -> u32 {
    let copy = |v: u32| x87::store(x87::load(v));
    let mv = move_block(h, w);
    let pmv = move_block(h, player);
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    let pos = |h: &mut H, ctx: u32| -> [u32; 6] {
        [
            copy(h.read(mv)),
            copy(h.read(mv + 4)),
            copy(h.read(mv + 8)),
            copy(h.read(ctx + 0x218)),
            copy(h.read(ctx + 0x21c)),
            copy(h.read(ctx + 0x220)),
        ]
    };
    let [px, py, pz, cx, cy, cz] = pos(h, ctx);
    let band = copy(h.read(mv + 0x1c));
    if h.read::<u8>(w + 0xaa) != 1 {
        return 0;
    }
    let length_sq3 = h.rebase(0x0088_c3c0);
    let diff = |a: u32, b: u32| x87::store(x87::sub(x87::load(a), x87::load(b)));
    write_u16(h, w + 0x36, u32::from(h.read::<u16>(w + 0x36)) & 0xfffe);
    let dy = x87::store(x87::sub(x87::load(cy), x87::load(py)).abs());
    let mut ret = 0;
    let mut touched = 0;
    let mut ctx = ctx;
    if x87::load(band) > x87::load(dy) {
        let (dz, dx) = (diff(cz, pz), diff(cx, px));
        let l = x87::store(fcall(h, length_sq3, &[dx, 0, dz]));
        let r = x87::load(h.read(mv + 0x48));
        let r2 = x87::store(x87::mul(r, r));
        if x87::load(r2) >= x87::load(l) {
            touched = 1;
            let id = u32::from(h.read::<u16>(w + 0x2e));
            ret = host_call(h, 0x0087_64f0, &[0xffff, id, 3]);
        }
        ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    }
    let f = u32::from(h.read::<u16>(w + 0x34));
    write_u16(h, w + 0x34, f & !0x40 | touched << 6);
    if h.read::<u8>(w + 0xab) & 1 == 0 || h.read::<u8>(ctx + 0x1e4) != 1 {
        return ret;
    }
    let [px, py, pz, cx, cy, cz] = pos(h, ctx);
    let level = x87::load(h.read(mv + 0x2c));
    let below = level < 0.0;
    let (dz, ey) = (diff(cz, pz), if below { diff(cy, py) } else { 0 });
    let dx = diff(cx, px);
    let l = x87::store(fcall(h, length_sq3, &[dx, ey, dz]));
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    let reach = x87::store(x87::add(
        x87::load(h.read(ctx + 0x1f0)),
        x87::load(h.read(mv + 0x40)),
    ));
    let reach = x87::store(x87::mul(x87::load(reach), x87::load(reach)));
    let (l64, flags) = (x87::load(l), h.read::<u16>(w + 0x34));
    // every compare is false when unordered, as `fcomp` with `test ah, 0x41`
    let in_reach = x87::load(reach) > l64
        && flags & 0x20 == 0
        && flags & 0x200 == 0
        && h.read::<u8>(w + 0x36) & 8 == 0;
    if !in_reach {
        return ret;
    }
    if !below {
        let dy = x87::store(x87::sub(x87::load(cy), x87::load(py)).abs());
        let in_band = level > x87::load(dy);
        if !in_band {
            return ret;
        }
    }
    let nearer = h.read::<u16>(ctx + 0x1e8) == 0xffff || x87::load(h.read(ctx + 0x1ec)) > l64;
    if !nearer {
        return ret;
    }
    let heading = copy(h.read(pmv + 0x34));
    let tol = copy(h.read(mv + 0x3c));
    if host_call(h, 0x0088_b780, &[heading, tol, cx, cz, px, pz]) == 0 {
        return ret;
    }
    let id = h.read::<u16>(w + 0x2e);
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    h.write(ctx + 0x1ec, copy(l));
    h.write(ctx + 0x1e8, id);
    ret
}

/// `AtelExecMenuCtrl()` (`0x00873600`, Fahrenheit's name; caller
/// `0x008a9ca0`): `AtelMenuExec`, then ctrl work 0's callback queue
/// (`AtelExecCallbackQue(0)` when `AtelCheckCallbackQue(0)`); `EAX` the
/// last call's.
pub fn atel_exec_menu_ctrl(h: &mut impl ffx_bridge::Host) -> u32 {
    host_call(h, 0x0086_dd60, &[]);
    let queued = host_call(h, 0x0087_5000, &[0]);
    if queued == 0 {
        return queued;
    }
    host_call(h, 0x0087_50b0, &[0])
}

/// Writes an `AtelInitBasicWork` setup record (eight words) at `rec`.
fn write_setup(h: &mut impl Mem, rec: u32, words: [u32; 8]) {
    for (k, v) in (0_u32..).zip(words) {
        h.write(rec + 4 * k, v);
    }
}

/// `AtelCommonSetUp(ctrl, a, block, x34, x8, debug)` (`0x008759b0`,
/// Fahrenheit's name; caller `TOMenuAtelStart`): registers the debug
/// sources (`debug`: a count, then three words each, through
/// `AtelSetUpSourceDebug`, flag `0x100000` for ctrl work 0 else
/// `0x20000`), takes ctrl work `ctrl` for `block`
/// (`AtelRequireCtrlWork(ctrl, block, a, -1)`, `+0x34` = `x34`), sets it up
/// and makes it current, then sets up the block's workers
/// (`AtelInitBasicWork` with clear, record `(active, i, ctrl, 0, i,
/// AtelGetIndexWork(block, i), block, x8)`; the actors active, the other
/// namespace entries not), each with its source debug work
/// (`AtelInitSourceCodeDebugWork(i, debug)`), and `AtelInitActor`. `rec`
/// is the record's frame (eight words); `EAX` `AtelInitActor`'s.
pub fn atel_common_set_up(h: &mut impl ffx_bridge::Host, args: [u32; 6], rec: u32) -> u32 {
    let [ctrl, a, block, x34, x8, debug] = args;
    let actors = host_call(h, 0x0086_a650, &[block]).cast_signed();
    let names = host_call(h, 0x0086_c190, &[block]).cast_signed();
    if debug != 0 {
        let mut j = 0_u32;
        while j.cast_signed() < h.read::<i32>(debug) {
            let e = debug + 4 + 12 * j;
            let (e0, e1, e2) = (h.read::<u32>(e), h.read::<u32>(e + 4), h.read::<u32>(e + 8));
            let flag = if ctrl == 0 { 0x0010_0000 } else { 0x0002_0000 };
            host_call(h, 0x0088_5b00, &[e1, e2, x8, e0, flag]);
            j += 1;
        }
    }
    let c = host_call(h, 0x0086_ee10, &[ctrl, block, a, u32::MAX]);
    h.write(c + 0x34, x34);
    host_call(h, 0x0087_1190, &[ctrl]);
    host_call(h, 0x0086_2d30, &[ctrl]);
    let mut i = 0_i32;
    for (active, end) in [(1, actors), (0, names)] {
        while i < end {
            let k = i.cast_unsigned();
            let hdr = host_call(h, 0x0086_bb80, &[block, k]);
            write_setup(h, rec, [active, k, ctrl, 0, k, hdr, block, x8]);
            host_call(h, 0x0086_cb00, &[rec, 1]);
            host_call(h, 0x0086_d670, &[k, debug]);
            i += 1;
        }
    }
    host_call(h, 0x0086_cae0, &[])
}

/// `AtelBattleSetUp(blocks, x8s, n, x30, work, a, ctrl, callback)`
/// (`0x008756d0`, Fahrenheit's name; caller `MsAtelSetUp`, which reads no
/// `EAX`):
/// takes ctrl work `ctrl` (`AtelRequireCtrlWork(ctrl, 0, a, -1)`; `+0x34`
/// 0, `+0x54` the field getter `0x008756b0`) for the `n` script blocks in
/// `blocks`, summing their actor counts into `+0x0c`, namespaces `+0x10`,
/// `+0x14`/`+0x18`/`+0x16` from the block header getters; `+0x1f8` the
/// blocks, `+0x1fc` `x8s`, `+0x30` `x30`, `+0x4c` `callback`. Set up and
/// current, it gives every block's actors their basic work (record
/// `(1, index, ctrl, block number, i, AtelGetIndexWork, block, x8s[k] or
/// 0)`, no clear) and their variables (`AtelSetMyWorkAdrs` from `work`,
/// advanced by `AtelGetMyWorkSize`), then the other namespace entries
/// (active 0), then `MsAtelChrInit(ctrl)`, `AtelInitActor` and ctrl work 0
/// current again (`EAX` that call's). `rec` is the record's frame.
pub fn atel_battle_set_up(h: &mut impl ffx_bridge::Host, args: [u32; 8], rec: u32) -> u32 {
    let [blocks, x8s, n, x30, work, a, ctrl, callback] = args;
    let n = n.cast_signed();
    let c = host_call(h, 0x0086_ee10, &[ctrl, 0, a, u32::MAX]);
    h.write(c + 0x34, 0_u32);
    h.write(c + 0x54, h.rebase(0x0087_56b0));
    // actors, +0x14, +0x18, +0x16, namespaces
    let mut sums = [0_u32; 5];
    for k in 0..n.max(0).cast_unsigned() {
        let b = h.read::<u32>(blocks.wrapping_add(4 * k));
        for (s, f) in sums.iter_mut().zip([
            0x0086_a650,
            0x0086_bcb0,
            0x0086_b020,
            0x0086_a9e0,
            0x0086_c190,
        ]) {
            *s = s.wrapping_add(host_call(h, f, &[b]));
        }
    }
    let [actors, x14, x18, x16, names] = sums;
    write_u16(h, c + 0x0c, actors);
    write_u16(h, c + 0x10, names);
    h.write(c + 0x4c, callback);
    write_u16(h, c + 0x16, x16);
    h.write(c + 0x1f8, blocks);
    write_u16(h, c + 0x14, x14);
    write_u16(h, c + 0x18, x18);
    h.write(c + 0x1fc, x8s);
    h.write(c + 0x30, x30);
    host_call(h, 0x0087_1190, &[ctrl]);
    host_call(h, 0x0086_2d30, &[ctrl]);
    let block_of = |h: &mut _, k: u32| {
        let b = Mem::read::<u32>(h, blocks.wrapping_add(4 * k));
        let x8 = if x8s == 0 {
            0
        } else {
            Mem::read::<u32>(h, x8s.wrapping_add(4 * k))
        };
        (b, x8)
    };
    let mut index = 0_u32;
    let mut work = work;
    for k in 0..n.max(0).cast_unsigned() {
        let (b, x8) = block_of(h, k);
        let actors = host_call(h, 0x0086_a650, &[b]).cast_signed();
        host_call(h, 0x0086_c190, &[b]);
        for i in 0..actors.max(0).cast_unsigned() {
            let hdr = host_call(h, 0x0086_bb80, &[b, i]);
            write_setup(h, rec, [1, index, ctrl, k, i, hdr, b, x8]);
            host_call(h, 0x0086_cb00, &[rec, 0]);
            host_call(h, 0x0087_0870, &[index, work]);
            work = work.wrapping_add(host_call(h, 0x0086_c160, &[index]));
            index += 1;
        }
    }
    for k in 0..n.max(0).cast_unsigned() {
        let (b, x8) = block_of(h, k);
        let mut i = host_call(h, 0x0086_a650, &[b]).cast_signed();
        let names = host_call(h, 0x0086_c190, &[b]).cast_signed();
        while i < names {
            let hdr = host_call(h, 0x0086_bb80, &[b, i.cast_unsigned()]);
            write_setup(h, rec, [0, index, ctrl, k, i.cast_unsigned(), hdr, b, x8]);
            host_call(h, 0x0086_cb00, &[rec, 0]);
            i += 1;
            index += 1;
        }
    }
    host_call(h, 0x0079_76a0, &[ctrl]);
    host_call(h, 0x0086_cae0, &[]);
    host_call(h, 0x0086_2d30, &[0])
}

/// `Atel_CheckBoxTouch_00867c10(w, _)` (name ours; caller `0x008666e0`'s
/// type-5 case): a box-shaped trigger. Two points relative to the actor's
/// move block position: A (ctrl work `+0x228..+0x230`, or the target
/// worker's move block `+0x10..+0x18` when move `+0x28` is not `0xffff`)
/// and B, the player (`+0x218..+0x220`, or the target's position), turned
/// by `-(move +0x34)` (`_CIsin`/`_CIcos`) into the box's frame. Each gets
/// outcode bits against the box: x against `±(+0x30)` (1, 2), z against
/// `±(+0x38)` (4, 8), y against `±(+0x1c)` when that is positive (0x10,
/// 0x20). `+0x36` bit 0 is cleared. B inside sets `+0x34` bit 8 and counts
/// as touching; outside, bit 8 is cleared and B touches when its height is
/// within the band (or there is no band) and `LengthSq3` from B to its
/// clamp onto the box is at most `(+0x48)²`. `+0x34` bit 6 keeps the
/// touch; the actor's vibrate signals go to itself
/// (`Atel_RegistVibrateSignal_00876590(0xffff, id, k)`): 5 on a new touch,
/// 5 then 6 when the segment A..B crosses the box (no common outside bit)
/// without a touch before, 6 when a touch ends. `EAX` the last signal's,
/// else 0. `fcall` calls an original returning in `ST0`
/// (`Math_LengthSq3`).
#[allow(clippy::too_many_lines, clippy::similar_names)] // the original's two outcode passes
pub fn atel_check_box_touch<H: super::run::NativeHost>(
    h: &mut H,
    w: u32,
    mut fcall: impl FnMut(&mut H, u32, &[u32]) -> f64,
) -> u32 {
    let copy = |v: u32| x87::store(x87::load(v));
    let ld = x87::load;
    let mv = move_block(h, w);
    let [px, py, pz] = [0, 4, 8].map(|o| copy(h.read(mv + o)));
    let hz = copy(h.read(mv + 0x38));
    let hx = copy(h.read(mv + 0x30));
    let angle = x87::store(x87::mul(ld(h.read(mv + 0x34)), -1.0));
    let hy = copy(h.read(mv + 0x1c));
    let target = h.read::<u16>(mv + 0x28);
    let (a, b): ([u32; 3], [u32; 3]) = if target == 0xffff {
        let c = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
        (
            [0x228, 0x22c, 0x230].map(|o| copy(h.read(c + o))),
            [0x218, 0x21c, 0x220].map(|o| copy(h.read(c + o))),
        )
    } else {
        let o = super::atel_get_basic_work(h, i32::from(target));
        let om = move_block(h, o);
        (
            [0x10, 0x14, 0x18].map(|k| copy(h.read(om + k))),
            [0, 4, 8].map(|k| copy(h.read(om + k))),
        )
    };
    let rel = |v: u32, p: u32| x87::store(x87::sub(ld(v), ld(p)));
    let (ax, ay, az) = (rel(a[0], px), rel(a[1], py), rel(a[2], pz));
    let (bx, by, bz) = (rel(b[0], px), rel(b[1], py), rel(b[2], pz));
    let (sin, cos) = (h.rebase(0x0094_98c8), h.rebase(0x0094_98c2));
    let s = x87::store(h.call_intrinsic1(sin, ld(angle)));
    let c = x87::store(h.call_intrinsic1(cos, ld(angle)));
    let ns = s ^ 0x8000_0000;
    // into the box's frame: x' = c x - s z, z' = s x + c z
    let az2 = x87::store(x87::add(x87::mul(ld(s), ld(ax)), x87::mul(ld(c), ld(az))));
    let ax2 = x87::store(x87::add(x87::mul(ld(ax), ld(c)), x87::mul(ld(az), ld(ns))));
    let bz2 = x87::store(x87::add(x87::mul(ld(s), ld(bx)), x87::mul(ld(c), ld(bz))));
    let bx2 = x87::store(x87::add(x87::mul(ld(bz), ld(ns)), x87::mul(ld(c), ld(bx))));
    write_u16(h, w + 0x36, u32::from(h.read::<u16>(w + 0x36)) & 0xfffe);
    // every compare is false when unordered
    let (hx64, hz64, hy64) = (ld(hx), ld(hz), ld(hy));
    let mut out_a = 0_u32;
    if ld(ax2) >= hx64 {
        out_a |= 1;
    }
    if -hx64 >= ld(ax2) {
        out_a |= 2;
    }
    if ld(az2) >= hz64 {
        out_a |= 4;
    }
    if -hz64 >= ld(az2) {
        out_a |= 8;
    }
    if hy64 > 0.0 {
        if ld(ay) >= hy64 {
            out_a |= 0x10;
        }
        if -hy64 >= ld(ay) {
            out_a |= 0x20;
        }
    }
    let (mut cx, mut cz) = (bx2, bz2);
    let mut out_b = 0_u32;
    if ld(bx2) >= hx64 {
        out_b |= 1;
        cx = hx;
    }
    let nhx = x87::mul(hx64, -1.0);
    if ld(bx2) <= nhx {
        out_b |= 2;
        cx = x87::store(nhx);
    }
    if ld(bz2) >= hz64 {
        out_b |= 4;
        cz = hz;
    }
    let nhz = x87::mul(hz64, -1.0);
    if nhz >= ld(bz2) {
        out_b |= 8;
        cz = x87::store(nhz);
    }
    if hy64 > 0.0 {
        if ld(by) >= hy64 {
            out_b |= 0x10;
        }
        if ld(by) <= x87::mul(-1.0, hy64) {
            out_b |= 0x20;
        }
    }
    let crossing = out_a & out_b == 0 && out_a | out_b != 0;
    let f34 = u32::from(h.read::<u16>(w + 0x34));
    let was = f34 >> 6 & 1;
    let touched = if out_b == 0 {
        write_u16(h, w + 0x34, f34 | 0x100);
        true
    } else {
        write_u16(h, w + 0x34, f34 & 0xfeff);
        let in_band = hy64 <= 0.0 || (hy64 >= ld(ay) && -hy64 <= ld(ay));
        if in_band {
            let dz = x87::store(x87::sub(ld(bz2), ld(cz)));
            let dx = x87::store(x87::sub(ld(bx2), ld(cx)));
            let l = x87::store(fcall(h, h.rebase(0x0088_c3c0), &[dx, 0, dz]));
            let r = ld(h.read(mv + 0x48));
            let r2 = x87::store(x87::mul(r, r));
            ld(r2) >= ld(l)
        } else {
            false
        }
    };
    let f34 = u32::from(h.read::<u16>(w + 0x34));
    write_u16(h, w + 0x34, f34 & !0x40 | u32::from(touched) << 6);
    let signal = |h: &mut H, k: u32| {
        let id = u32::from(Mem::read::<u16>(h, w + 0x2e));
        host_call(h, 0x0087_6590, &[0xffff, id, k])
    };
    match (touched, was, crossing) {
        (true, 0, _) => signal(h, 5),
        (false, 0, true) => {
            signal(h, 5);
            signal(h, 6)
        }
        (false, 1, _) => signal(h, 6),
        _ => 0,
    }
}

/// `Atel_CheckLineTouch_008684b0(w, _, player)` (name ours; caller
/// `0x008666e0`, the ctrl work's `+0x5c` callback, whose `EAX` its caller
/// drops): a line-shaped trigger from the actor's move position P to move
/// `+0x30` Q, against the player's step from A (ctrl work `+0x228`) to B
/// (`+0x218`). When the player moved in x or z (or `+0x36` bit 0 asks), the
/// step crossing the line (`Math_SegmentsCross(A, B, P, Q)`, its sign into
/// `+0x36` bits 1/2) within the height band (`Math_CrossHeightGap` at most
/// move `+0x1c`, any when that is not positive) sends trigger 4. Then B's
/// nearest point N on P..Q (`Math_SegmentNearestT`): within `+0x48`
/// (`Math_LengthSq3`, no y) and the band around N's height it touches
/// (`+0x34` bit 6, trigger 3, trigger 5 on a new touch); else a crossing
/// sends 5, and a crossing or an ended touch 6. Talk (`+0xab` bit 0, ctrl
/// `+0x1e4` 1): within `(move +0x40 + ctrl +0x1f0)²` of N and the band, a
/// candidate distance (`+0x1ec`, none when `+0x1e8` is `0xffff`) above
/// that radius², and `Math_FacingWithin(player heading, +0x3c, B, N)`
/// make it the candidate (`+0x1e8` id, `+0x1ec` the radius², as the
/// original stores). `frame` holds the four vectors (16 words: A, Q, P, B,
/// `w` 0) the math helpers get pointers to; `fcall` calls an original
/// returning in `ST0` (rebased address).
#[allow(clippy::too_many_lines, clippy::similar_names)] // the original's three steps
pub fn atel_check_line_touch<H: ffx_bridge::Host>(
    h: &mut H,
    w: u32,
    player: u32,
    frame: u32,
    mut fcall: impl FnMut(&mut H, u32, &[u32]) -> f64,
) {
    let copy = |v: u32| x87::store(x87::load(v));
    let ld = x87::load;
    let mv = move_block(h, w);
    let pmv = move_block(h, player);
    let (va, vq, vp, vb) = (frame, frame + 0x10, frame + 0x20, frame + 0x30);
    for k in 0..16 {
        h.write(frame + 4 * k, 0_u32);
    }
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    let p = [0, 4, 8].map(|o| copy(h.read(mv + o)));
    let q = [0x30, 0x34, 0x38].map(|o| copy(h.read(mv + o)));
    let a = [0x228, 0x22c, 0x230].map(|o| copy(h.read(ctx + o)));
    let b = [0x218, 0x21c, 0x220].map(|o| copy(h.read(ctx + o)));
    for (base, v) in [(vp, p), (vq, q), (va, a), (vb, b)] {
        for (k, x) in (0_u32..).zip(v) {
            h.write(base + 4 * k, x);
        }
    }
    let band = copy(h.read(mv + 0x1c));
    let band64 = ld(band);
    // `fucompp` with `test ah, 0x44`: unordered counts as moved
    #[allow(clippy::float_cmp)] // the original's exact compare
    let still = ld(a[0]) == ld(b[0]) && ld(a[2]) == ld(b[2]);
    let mut crossed = false;
    if !still || h.read::<u8>(w + 0x36) & 1 != 0 {
        write_u16(h, w + 0x36, u32::from(h.read::<u16>(w + 0x36)) & 0xfffe);
        let side = host_call(h, 0x0088_9bb0, &[va, vb, vp, vq]).cast_signed();
        if side != 0 {
            let gap = x87::store(fcall(h, h.rebase(0x0088_9e90), &[va, vb, vp, vq]));
            let within = band64 <= 0.0 || ld(gap) <= band64;
            if within {
                let f = u32::from(h.read::<u16>(w + 0x36));
                let f = if side > 0 { f & 0xfffb | 2 } else { f & 0xfff9 };
                write_u16(h, w + 0x36, f);
                let id = u32::from(h.read::<u16>(w + 0x2e));
                host_call(h, 0x0087_64f0, &[0xffff, id, 4]);
                crossed = true;
            }
        }
    }
    let t = x87::store(fcall(h, h.rebase(0x0088_a040), &[vb, vp, vq]));
    let t64 = ld(t);
    let nx = x87::store(x87::add(
        ld(p[0]),
        x87::mul(x87::sub(ld(q[0]), ld(p[0])), t64),
    ));
    let ny = x87::store(x87::add(
        x87::mul(x87::sub(ld(q[1]), ld(p[1])), t64),
        ld(p[1]),
    ));
    let nz = x87::store(x87::add(
        x87::mul(t64, x87::sub(ld(q[2]), ld(p[2]))),
        ld(p[2]),
    ));
    let [bx, by, bz] = b;
    let dz = x87::store(x87::sub(ld(nz), ld(bz)));
    let dx = x87::store(x87::sub(ld(nx), ld(bx)));
    let l = x87::store(fcall(h, h.rebase(0x0088_c3c0), &[dx, 0, dz]));
    let f34 = u32::from(h.read::<u16>(w + 0x34));
    let r = ld(h.read(mv + 0x48));
    let r2 = x87::store(x87::mul(r, r));
    let was = f34 >> 6 & 1;
    // B's height within the band around N's (any when the band is not
    // positive); false on NaN
    let in_band = |band64: f64| {
        band64 <= 0.0 || (x87::add(ld(ny), band64) >= ld(by) && ld(by) >= x87::sub(ld(ny), band64))
    };
    let id = |h: &mut H| u32::from(Mem::read::<u16>(h, w + 0x2e));
    if ld(r2) >= ld(l) && in_band(band64) {
        write_u16(h, w + 0x34, f34 | 0x40);
        let i = id(h);
        host_call(h, 0x0087_64f0, &[0xffff, i, 3]);
        if was == 0 {
            let i = id(h);
            host_call(h, 0x0087_64f0, &[0xffff, i, 5]);
        }
    } else {
        write_u16(h, w + 0x34, f34 & 0xffbf);
        if crossed {
            let i = id(h);
            host_call(h, 0x0087_64f0, &[0xffff, i, 5]);
        }
        if was == 1 || crossed {
            let i = id(h);
            host_call(h, 0x0087_64f0, &[0xffff, i, 6]);
        }
    }
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    if h.read::<u8>(w + 0xab) & 1 == 0 || h.read::<u8>(ctx + 0x1e4) != 1 {
        return;
    }
    let reach = x87::store(x87::add(ld(h.read(mv + 0x40)), ld(h.read(ctx + 0x1f0))));
    let reach2 = x87::store(x87::mul(ld(reach), ld(reach)));
    let near = ld(reach2) >= ld(l) && in_band(band64);
    if !near {
        return;
    }
    let free = h.read::<u16>(ctx + 0x1e8) == 0xffff || ld(h.read(ctx + 0x1ec)) > ld(reach2);
    if !free {
        return;
    }
    let heading = copy(h.read(pmv + 0x34));
    let tol = copy(h.read(mv + 0x3c));
    if host_call(h, 0x0088_b780, &[heading, tol, bx, bz, nx, nz]) == 0 {
        return;
    }
    let i = h.read::<u16>(w + 0x2e);
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    h.write(ctx + 0x1ec, copy(reach2));
    h.write(ctx + 0x1e8, i);
}

/// `Atel_CheckBoxTrigger_00866cc0(w, _, player)` (name ours; caller
/// `0x008666e0`, whose `EAX` its caller drops): the box-shaped request
/// trigger, the request-and-talk sibling of [`atel_check_box_touch`]. A
/// (ctrl work `+0x228`) and B (the player, `+0x218`) relative to the move
/// position, turned by `-(move +0x34)` (cosine first:
/// `x' = x c - z s`, `z' = c z + s x`), get the same outcodes against
/// `±(+0x30)`, `±(+0x38)` and `±(+0x1c)`. A crossing (no common outcode,
/// not both inside) sends trigger 4 unless `+0x34` bit 8 (inside last
/// time) is set. B inside sets bit 8 and touches; outside, bit 8 is
/// cleared and it touches within the band (A's height) and `(+0x48)²` of
/// its clamp. A touch sends 3 (and 5 when new); else a crossing without an
/// earlier touch sends 5 then 6, an ended touch 6. Talk (`+0xab` bit 0,
/// ctrl `+0x1e4` 1): B inside, or in the band and within
/// `(move +0x40 + ctrl +0x1f0)²` of its clamp; a candidate distance above
/// that radius² (any for `+0x1e8` `0xffff`) and
/// `Math_FacingWithin(player heading, +0x3c, player x, z, actor x, z)`
/// make it the candidate (`+0x1e8` id, `+0x1ec` the radius²). `fcall`
/// calls an original returning in `ST0` (`Math_LengthSq3`).
#[allow(clippy::too_many_lines, clippy::similar_names)] // the original's three steps
pub fn atel_check_box_trigger<H: super::run::NativeHost>(
    h: &mut H,
    w: u32,
    player: u32,
    mut fcall: impl FnMut(&mut H, u32, &[u32]) -> f64,
) {
    let copy = |v: u32| x87::store(x87::load(v));
    let ld = x87::load;
    let mv = move_block(h, w);
    let pmv = move_block(h, player);
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    let [px, py, pz] = [0, 4, 8].map(|o| copy(h.read(mv + o)));
    let hz = copy(h.read(mv + 0x38));
    let hx = copy(h.read(mv + 0x30));
    let angle = copy(h.read(mv + 0x34)) ^ 0x8000_0000;
    let hy = copy(h.read(mv + 0x1c));
    let rel = |h: &mut H, o: u32, p: u32| x87::store(x87::sub(ld(h.read(ctx + o)), ld(p)));
    let (ax, ay, az) = (rel(h, 0x228, px), rel(h, 0x22c, py), rel(h, 0x230, pz));
    let (bx, by, bz) = (rel(h, 0x218, px), rel(h, 0x21c, py), rel(h, 0x220, pz));
    let (sin, cos) = (h.rebase(0x0094_98c8), h.rebase(0x0094_98c2));
    let c = x87::store(h.call_intrinsic1(cos, ld(angle)));
    let s = x87::store(h.call_intrinsic1(sin, ld(angle)));
    let az2 = x87::store(x87::add(x87::mul(ld(c), ld(az)), x87::mul(ld(s), ld(ax))));
    let ax2 = x87::store(x87::sub(x87::mul(ld(ax), ld(c)), x87::mul(ld(az), ld(s))));
    let bz2 = x87::store(x87::add(x87::mul(ld(c), ld(bz)), x87::mul(ld(s), ld(bx))));
    write_u16(h, w + 0x36, u32::from(h.read::<u16>(w + 0x36)) & 0xfffe);
    let bx2 = x87::store(x87::sub(x87::mul(ld(c), ld(bx)), x87::mul(ld(s), ld(bz))));
    // every compare is false when unordered
    let (hx64, hz64, hy64) = (ld(hx), ld(hz), ld(hy));
    let (nhx, nhz) = (hx ^ 0x8000_0000, hz ^ 0x8000_0000);
    let mut out_a = 0_u32;
    if ld(ax2) >= hx64 {
        out_a |= 1;
    }
    if ld(nhx) >= ld(ax2) {
        out_a |= 2;
    }
    if ld(az2) >= hz64 {
        out_a |= 4;
    }
    if ld(nhz) >= ld(az2) {
        out_a |= 8;
    }
    if hy64 > 0.0 {
        if ld(ay) >= hy64 {
            out_a |= 0x10;
        }
        if -hy64 >= ld(ay) {
            out_a |= 0x20;
        }
    }
    let (mut cx, mut cz) = (bx2, bz2);
    let mut out_b = 0_u32;
    if ld(bx2) >= hx64 {
        out_b |= 1;
        cx = hx;
    }
    if ld(nhx) >= ld(bx2) {
        out_b |= 2;
        cx = nhx;
    }
    if ld(bz2) >= hz64 {
        out_b |= 4;
        cz = hz;
    }
    if ld(nhz) >= ld(bz2) {
        out_b |= 8;
        cz = nhz;
    }
    if hy64 > 0.0 {
        if ld(by) >= hy64 {
            out_b |= 0x10;
        }
        if x87::mul(hy64, -1.0) >= ld(by) {
            out_b |= 0x20;
        }
    }
    let id = |h: &mut H| u32::from(Mem::read::<u16>(h, w + 0x2e));
    let crossing = out_a & out_b == 0 && out_a | out_b != 0;
    if crossing && h.read::<u8>(w + 0x35) & 1 == 0 {
        let i = id(h);
        host_call(h, 0x0087_64f0, &[0xffff, i, 4]);
    }
    let f34 = u32::from(h.read::<u16>(w + 0x34));
    let was = f34 >> 6 & 1;
    // A's height within the band (any when the band is not positive)
    let in_band = hy64 <= 0.0 || (ld(ay) <= hy64 && -hy64 <= ld(ay));
    let length_sq3 = h.rebase(0x0088_c3c0);
    let mut dist = |h: &mut H| {
        let dz = x87::store(x87::sub(ld(bz2), ld(cz)));
        let dx = x87::store(x87::sub(ld(bx2), ld(cx)));
        x87::store(fcall(h, length_sq3, &[dx, 0, dz]))
    };
    let touched = if out_b == 0 {
        write_u16(h, w + 0x34, f34 | 0x100);
        true
    } else {
        write_u16(h, w + 0x34, f34 & 0xfeff);
        in_band && {
            let l = dist(h);
            let r = ld(h.read(mv + 0x48));
            ld(x87::store(x87::mul(r, r))) >= ld(l)
        }
    };
    let f34 = u32::from(h.read::<u16>(w + 0x34));
    write_u16(h, w + 0x34, f34 & !0x40 | u32::from(touched) << 6);
    if touched {
        let i = id(h);
        host_call(h, 0x0087_64f0, &[0xffff, i, 3]);
        if was == 0 {
            let i = id(h);
            host_call(h, 0x0087_64f0, &[0xffff, i, 5]);
        }
    } else if crossing && was == 0 {
        let i = id(h);
        host_call(h, 0x0087_64f0, &[0xffff, i, 5]);
        let i = id(h);
        host_call(h, 0x0087_64f0, &[0xffff, i, 6]);
    } else if was == 1 {
        let i = id(h);
        host_call(h, 0x0087_64f0, &[0xffff, i, 6]);
    }
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    if h.read::<u8>(w + 0xab) & 1 == 0 || h.read::<u8>(ctx + 0x1e4) != 1 {
        return;
    }
    let reach = x87::store(x87::add(ld(h.read(mv + 0x40)), ld(h.read(ctx + 0x1f0))));
    let reach2 = x87::store(x87::mul(ld(reach), ld(reach)));
    if out_b != 0 {
        if !in_band {
            return;
        }
        let l = dist(h);
        let near = ld(reach2) >= ld(l);
        if !near {
            return;
        }
    }
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    let free = h.read::<u16>(ctx + 0x1e8) == 0xffff || ld(h.read(ctx + 0x1ec)) > ld(reach2);
    if !free {
        return;
    }
    let args = [
        copy(h.read(pmv + 0x34)),
        copy(h.read(mv + 0x3c)),
        copy(h.read(ctx + 0x218)),
        copy(h.read(ctx + 0x220)),
        copy(h.read(mv)),
        copy(h.read(mv + 8)),
    ];
    if host_call(h, 0x0088_b780, &args) == 0 {
        return;
    }
    let i = h.read::<u16>(w + 0x2e);
    let ctx = h.read::<u32>(h.rebase(SCRIPT_CONTEXT));
    h.write(ctx + 0x1ec, copy(reach2));
    h.write(ctx + 0x1e8, i);
}
