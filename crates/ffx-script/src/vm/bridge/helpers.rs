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
