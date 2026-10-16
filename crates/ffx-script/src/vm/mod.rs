//! The event-script VM ("Atel") as `FFX.exe` keeps it in memory
//! (`docs/formats/script.md`, "The VM in memory"), and the logic of its
//! accessors on plain values.
//!
//! - A **ctrl work** ([`ctrl`]) is one running script block: seven of them,
//!   `0x238` bytes each, at `0x01325ba0` (`AtelGetCtrlWork`); the current
//!   one is `g_ScriptContext` (`AtelGetCurCtrlWork`), right after them.
//! - Its **workers** ([`worker`]) lie back to back from `ctrl + 0x1c`, in
//!   five size classes ([`worker_offset`], from `AtelGetBasicWork`).
//! - Each worker has a value **stack** ([`stack`]) of 19 tagged slots and
//!   up to nine **levels** ([`level`]), one per request priority, each
//!   with its own program counter.
//! - **Variables** ([`var`]) are 8-byte descriptors in the worker header's
//!   table: type, storage area and offset in one word, array length after.
//!
//! [`bridge`] runs the original accessors over game memory.

pub mod bridge;
pub mod exec;

/// Ctrl work fields (offsets from its start).
pub mod ctrl {
    /// Size of one ctrl work (`AtelGetCtrlWork`).
    pub const SIZE: u32 = 0x238;
    /// How many there are (`g_ScriptContext` follows the array).
    pub const COUNT: u32 = 7;
    /// `u8`: bit 0 = in use (`AtelGetEventActorTotal`, cleared by
    /// `AtelRemoveCtrlWork`).
    pub const FLAGS: u32 = 0x00;
    /// `u8`: bits 0-1 = run mode `Script_Run` tests (1 skips every worker,
    /// 2 all but those with flag `0x8000` at worker `+0x36`).
    pub const MODE: u32 = 0x03;
    /// `u16`: the player's worker, `0xffff` for none
    /// (`AtelGetNowCtrlTopId`).
    pub const TOP_ID: u32 = 0x0a;
    /// `u16`: workers with a full work area (`AtelGetActorType`'s bound;
    /// `AtelGetCtrlworkActorTotal`).
    pub const ACTORS: u32 = 0x0c;
    /// `u16`: every worker of the block, the last ones with a `0x30`-byte
    /// work only (`AtelGetBasicWork`'s bound).
    pub const WORKERS: u32 = 0x10;
    /// `u16`: workers of the `0x5b8`-byte class.
    pub const CLASS_5B8: u32 = 0x14;
    /// `u16`: workers of the `0x558`-byte class.
    pub const CLASS_558: u32 = 0x16;
    /// `u16`: workers of the `0x2e8`-byte class.
    pub const CLASS_2E8: u32 = 0x18;
    /// `u16`: this ctrl work's number (0 = the event's); `AtelGetBasicWork`
    /// keys its cache on it.
    pub const ID: u32 = 0x1a;
    /// `u32`: the first worker.
    pub const WORK: u32 = 0x1c;
    /// `u32`: base of storage 0 variables (the event's: the save data's
    /// script area in the field).
    pub const STORAGE0: u32 = 0x2c;
    /// `u32`: base of storage 1 variables.
    pub const STORAGE1: u32 = 0x30;
    /// `u32`: `AtelSetVibrateStreamExt`'s value.
    pub const VIBRATE_STREAM_EXT: u32 = 0x44;
    /// `u32`: the event's script block (the workers'
    /// [`super::worker::BLOCK`]) in ctrl work 0, null in the battle's other
    /// ctrl works (snapshots); `AtelGetEventMapCameraId` reads its `+0x18`
    /// (`u16`), `AtelGetGideMapActorNumber` its expansion header.
    pub const BLOCK: u32 = 0x48;
    /// `u32`: `AtelInitActor`'s callback, `(ctrl)`.
    pub const INIT_ACTOR: u32 = 0x50;
    /// `u32`: storage 3's address callback, `(worker, offset) -> address`
    /// (`AtelGetRealVarAdrs`); null = the worker header's `+0x2c` area.
    pub const VAR_ADDRESS: u32 = 0x54;
    /// `u32`: called by `Script_Run` before a worker runs, `(worker, level)`.
    pub const RUN_BEGIN: u32 = 0x58;
    /// `u32`: called by `Script_Run` after, `(worker, level)`.
    pub const RUN_END: u32 = 0x5c;
    /// `u32`: cleared by `AtelRemoveCtrlWork`.
    pub const REMOVE_CLEARS: u32 = 0x1f4;
    /// `u32`: field state bits; bit 0 is `AtelGetBattleStatus`
    /// (`0x0086a9b0`), bit 3 native `0x0241`.
    pub const FIELD_STATE: u32 = 0x210;
}

/// Script block fields the VM reads (`docs/formats/script.md`, "Script
/// block"), from the block's start.
pub mod block {
    /// `u32`: offset of the storage 6 variables, shared by the block's
    /// workers (`AtelGetRealVarAdrs`).
    pub const STORAGE6: u32 = 0x20;
    /// `u32`: offset of the expansion header, 0 for none
    /// (`AtelGetBinHdrExpandAddChr`: its `+4` is the extra character count).
    pub const EXPAND: u32 = 0x2c;
    /// `u32`: code start.
    pub const CODE: u32 = 0x30;
    /// `u16`: workers (`AtelGetNamespaceNumber`).
    pub const WORKERS: u32 = 0x34;
    /// `u16`: workers with a full work area, the first ones
    /// (`AtelGetActorNumber`).
    pub const ACTORS: u32 = 0x36;
    /// `u32` × workers: worker header offsets (`AtelGetIndexWork`).
    pub const HEADERS: u32 = 0x38;
}

/// Worker header fields the VM reads (from the header's start; offsets in
/// it are relative to the block).
pub mod header {
    /// `u32`: the variable table ([`super::var`]).
    pub const VARS: u32 = 0x14;
    /// `u32`: the integer constants (opcode `2d`).
    pub const INTS: u32 = 0x18;
    /// `u32`: the float constants (opcode `2f`).
    pub const FLOATS: u32 = 0x1c;
    /// `u32`: storage 2 variables.
    pub const STORAGE2: u32 = 0x28;
    /// `u32`: storage 3 variables (without a ctrl work callback).
    pub const STORAGE3: u32 = 0x2c;
    /// `u32`: storage 4 variables (right after the worker's code).
    pub const STORAGE4: u32 = 0x30;
}

/// Worker fields (offsets from its start).
pub mod worker {
    /// `u32`: its header in the script block (`docs/formats/script.md`);
    /// the first byte is the worker type.
    pub const HEADER: u32 = 0x00;
    /// `u32`: the script block.
    pub const BLOCK: u32 = 0x04;
    /// `u16`: its index (`AtelGetActorId`).
    pub const ID: u32 = 0x2e;
    /// `u8`: the level running (`AtelPopStackFloat`'s error report).
    pub const LEVEL: u32 = 0x32;
    /// `u16`: flags `Script_Run` tests (2: runs, 4, ...).
    pub const FLAGS: u32 = 0x34;
    /// `u16`: more flags `Script_Run` tests (`0x40`, `0x8000`).
    pub const FLAGS2: u32 = 0x36;
    /// Camera handle cache of the group 6 natives (`AtelGetCameraWorkAdrs`).
    pub const CAMERA: u32 = 0x3c;
    /// `u32`: the bound program counter (`AtelGetBindPC`, `0x0086adf0`;
    /// `AtelGetBindPCFromID`).
    pub const BIND_PC: u32 = 0x40;
    /// Storage 5 variables: inside the worker itself.
    pub const STORAGE5: u32 = 0x48;
    /// `u32`: field character object (`AtelGetCharHandle`).
    pub const CHAR_HANDLE: u32 = 0x9c;
    /// `AtelGetBattleIdAdrs2`.
    pub const BATTLE_ID: u32 = 0xc0;
    /// The value stack ([`super::stack`]); `Script_Run` passes this.
    pub const STACK: u32 = 0xc4;
    /// The first level ([`super::level`]).
    pub const LEVELS: u32 = 0x12c;
    /// Position block of types 5 and 6 (`AtelGetPosWorkAdrs`).
    pub const POS_SMALL: u32 = 0x284;
    /// Position block of the other types.
    pub const POS: u32 = 0x558;
}

/// The value stack (at worker `+0xc4`).
pub mod stack {
    /// Slots.
    pub const SLOTS: i32 = 19;
    /// `i32`: slots in use.
    pub const DEPTH: u32 = 0x00;
    /// `u32` × 19: the values.
    pub const VALUES: u32 = 0x04;
    /// `u8` × 19: their kinds ([`INT`], [`FLOAT`]).
    pub const KINDS: u32 = 0x54;
    /// Kind of an integer slot.
    pub const INT: u8 = 1;
    /// Kind of a float slot.
    pub const FLOAT: u8 = 2;
}

/// A level: one request priority's execution state, `0x4c` bytes, the
/// first at worker `+0x12c`.
pub mod level {
    /// Size of one.
    pub const SIZE: u32 = 0x4c;
    /// `u32`: the program counter (an address in the block).
    pub const PC: u32 = 0x18;
    /// `u8`: call depth of opcode `33` (at most 3; return addresses at
    /// `+0`, worker IDs at `+0x10`).
    pub const CALL_DEPTH: u32 = 0x1d;
    /// `u8`: request state `Script_Run` waits on (0 = none).
    pub const STATE: u32 = 0x1f;
    /// `u32`: register A.
    pub const A: u32 = 0x20;
    /// `u32`: register B.
    pub const B: u32 = 0x24;
    /// `f32`: the float accumulator (opcodes `25`/`26`).
    pub const FACC: u32 = 0x28;
}

/// Variable descriptors: 8 bytes each, from the worker header's `+0x14`
/// (relative to the block).
pub mod var {
    /// Size of a descriptor.
    pub const SIZE: u32 = 8;
    /// `u32`: type (bits 28-31), storage (bits 25-27), offset (bits 0-23).
    pub const WORD: u32 = 0;
    /// `u16`: array length (`AtelGetVarArrayMax`, `AtelGetVarArrayNumber`).
    pub const LENGTH: u32 = 4;
    /// `u16`: `AtelGetVarArrayRange`.
    pub const RANGE: u32 = 6;

    /// Type: `u8`.
    pub const U8: u32 = 0;
    /// Type: `i8`.
    pub const I8: u32 = 1;
    /// Type: `u16`.
    pub const U16: u32 = 2;
    /// Type: `i16`.
    pub const I16: u32 = 3;
    /// Type: `u32`.
    pub const U32: u32 = 4;
    /// Type: `i32`.
    pub const I32: u32 = 5;
    /// Type: `f32`.
    pub const F32: u32 = 6;

    /// The type (bits 28-31).
    #[must_use]
    pub const fn kind(word: u32) -> u32 {
        word >> 28
    }

    /// The storage area (bits 25-27; `AtelGetRealVarAdrs`).
    #[must_use]
    pub const fn storage(word: u32) -> u32 {
        (word >> 25) & 7
    }

    /// The offset in its storage area.
    #[must_use]
    pub const fn offset(word: u32) -> u32 {
        word & 0x00ff_ffff
    }

    /// Bytes per element (`AtelGetValueIncValue`): 1 for types 0, 1 and
    /// 7, 2 for 2 and 3, 4 for 4 to 6, 0 for the rest.
    #[must_use]
    pub const fn element_size(kind: u32) -> u32 {
        match kind {
            0 | 1 | 7 => 1,
            2 | 3 => 2,
            4..=6 => 4,
            _ => 0,
        }
    }

    /// An array index clamped as the accessors do: negative to 0, past the
    /// end to `length - 1` (so -1 for an empty array).
    #[must_use]
    pub fn clamp_index(index: i32, length: u16) -> i32 {
        let i = index.max(0);
        let n = i32::from(length);
        if i >= n { n - 1 } else { i }
    }
}

/// Worker counts by class, from the ctrl work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Classes {
    /// [`ctrl::ACTORS`].
    pub actors: u16,
    /// [`ctrl::CLASS_5B8`].
    pub c5b8: u16,
    /// [`ctrl::CLASS_558`].
    pub c558: u16,
    /// [`ctrl::CLASS_2E8`].
    pub c2e8: u16,
}

/// Work sizes of the five classes, in the order they are laid out.
pub const CLASS_SIZES: [u32; 5] = [0xb58, 0x558, 0x5b8, 0x2e8, 0x30];

/// Where worker `index` (already in `0..WORKERS`) starts, from the first
/// worker (`AtelGetBasicWork`): `actors - c558 - c2e8 - c5b8` workers of
/// `0xb58` bytes, then `c558` of `0x558`, `c5b8` of `0x5b8`, `c2e8` of
/// `0x2e8`, and the rest of `0x30`. Computed in 32 bits with signed
/// comparisons, like the original, so inconsistent counts give the same
/// (wrong) addresses.
#[must_use]
#[allow(clippy::cast_sign_loss)] // the original mixes signed and unsigned
pub fn worker_offset(index: i32, counts: Classes) -> u32 {
    let third = i32::from(counts.c5b8);
    let second = i32::from(counts.c558);
    let fourth = i32::from(counts.c2e8);
    let full = i32::from(counts.actors)
        .wrapping_sub(second)
        .wrapping_sub(fourth)
        .wrapping_sub(third);
    if index < full {
        return (index as u32).wrapping_mul(0xb58);
    }
    let mut off = (full as u32).wrapping_mul(0xb58);
    let mut rest = index.wrapping_sub(full);
    if rest < second {
        return off.wrapping_add((rest as u32).wrapping_mul(0x558));
    }
    rest = rest.wrapping_sub(second);
    off = off.wrapping_add((second as u32).wrapping_mul(0x558));
    if rest < third {
        return off.wrapping_add((rest as u32).wrapping_mul(0x5b8));
    }
    rest = rest.wrapping_sub(third);
    off = off.wrapping_add((third as u32).wrapping_mul(0x5b8));
    if rest < fourth {
        return off.wrapping_add((rest as u32).wrapping_mul(0x2e8));
    }
    // rest still counts the 0x2e8 class: fourth * 0x2b8 + rest * 0x30 = fourth * 0x2e8 + (rest - fourth) * 0x30
    off.wrapping_add((fourth as u32).wrapping_mul(0x2b8))
        .wrapping_add((rest as u32).wrapping_mul(0x30))
}

/// Bytes the workers of a script block take (`AtelGetActorBufferSize`),
/// from its header's counts: `+0x34` workers, `+0x36` with a full work
/// area ([`Classes::actors`]), `+0x14` of the `0x5b8` class, `+0x16` of
/// `0x558`, `+0x1c` of `0x2e8`.
#[must_use]
pub fn buffer_size(workers: u16, counts: Classes) -> u32 {
    let full = u32::from(counts.actors)
        .wrapping_sub(u32::from(counts.c558))
        .wrapping_sub(u32::from(counts.c2e8))
        .wrapping_sub(u32::from(counts.c5b8));
    full.wrapping_mul(0xb58)
        .wrapping_add(
            u32::from(workers)
                .wrapping_sub(u32::from(counts.actors))
                .wrapping_mul(0x30),
        )
        .wrapping_add(u32::from(counts.c5b8) * 0x5b8)
        .wrapping_add(u32::from(counts.c558) * 0x558)
        .wrapping_add(u32::from(counts.c2e8) * 0x2e8)
}

/// Whether a worker type uses the small layout (types 5 and 6: two levels,
/// position at `+0x284`).
#[must_use]
pub const fn is_small(kind: u8) -> bool {
    kind.wrapping_sub(5) < 2
}

/// The level index the accessors use for `level` (`Script_WorkerLevel`):
/// negative to 0, then at most 1 for the small types and 8 for the rest.
#[must_use]
#[allow(clippy::cast_sign_loss)] // non-negative here
pub fn level_index(kind: u8, level: i32) -> u32 {
    let l = level.max(0);
    let max = if is_small(kind) { 1 } else { 8 };
    l.min(max) as u32
}

/// MSVC's `_ftol2_sse` (`0x009497e0`) on an `f32`, as the game's SSE2 path
/// does it ([`ffx_bridge::x87::cvttsd2si`]: truncation toward zero,
/// `0x80000000` for NaN and out-of-range values). Every CPU the game runs
/// on has SSE2; the x87 path (`ffx_bridge::x87::ftol2`) is the oracle's on
/// an image that has not run its start-up code.
#[must_use]
pub fn ftol(x: f32) -> u32 {
    ffx_bridge::x87::cvttsd2si(f64::from(x))
}

/// An `f32` after an x87 `fld`/`fstp` round trip: signalling NaNs come back
/// quiet; everything else unchanged.
#[must_use]
pub const fn through_x87(bits: u32) -> u32 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    }
}

/// A stack slot read as an integer (`AtelPopStackInteger`): integer slots
/// as they are, anything else as an `f32` truncated.
#[must_use]
pub fn slot_int(kind: u8, raw: u32) -> u32 {
    if kind == stack::INT {
        raw
    } else {
        ftol(f32::from_bits(raw))
    }
}

/// A stack slot read as a float (`AtelPopStackFloat`), as `f32` bits: float
/// slots through the x87, anything else as an `i32` converted.
#[must_use]
#[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)] // as the original
pub fn slot_float(kind: u8, raw: u32) -> u32 {
    if kind == stack::FLOAT {
        through_x87(raw)
    } else {
        ((raw as i32) as f32).to_bits()
    }
}

/// A variable's value as an integer (`AtelGetValueInteger`), from the
/// 4 bytes at its element (only the low bytes count for narrow types).
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // narrowing is the point
pub fn value_int(kind: u32, raw: u32) -> u32 {
    match kind {
        var::U8 => raw & 0xff,
        var::I8 => i32::from((raw as u8).cast_signed()).cast_unsigned(),
        var::U16 => raw & 0xffff,
        var::I16 => i32::from((raw as u16).cast_signed()).cast_unsigned(),
        var::U32 | var::I32 => raw,
        var::F32 => ftol(f32::from_bits(raw)),
        _ => 0,
    }
}

/// A variable's value as `f32` bits (`AtelGetValueFloat`); 0.0 for the
/// types past `f32`.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]
pub fn value_float(kind: u32, raw: u32) -> u32 {
    let v = match kind {
        var::U8 => f32::from(raw as u8),
        var::I8 => f32::from((raw as u8).cast_signed()),
        var::U16 => f32::from(raw as u16),
        var::I16 => f32::from((raw as u16).cast_signed()),
        var::U32 => raw as f32,
        var::I32 => (raw as i32) as f32,
        var::F32 => return through_x87(raw),
        _ => 0.0,
    };
    v.to_bits()
}

/// What storing `value` into a variable of type `kind` writes
/// (`AtelSetValueInteger`): `(bytes, width)`, `None` for types that store
/// nothing. With `clamp`, narrow types saturate to their range; without,
/// they keep the low bytes.
#[must_use]
#[allow(clippy::cast_sign_loss, clippy::cast_precision_loss)]
pub fn store_int(kind: u32, value: i32, clamp: bool) -> Option<(u32, u32)> {
    let sat = |lo: i32, hi: i32| if clamp { value.clamp(lo, hi) } else { value };
    Some(match kind {
        var::U8 => (sat(0, 0xff) as u32 & 0xff, 1),
        var::I8 => (sat(-0x80, 0x7f) as u32 & 0xff, 1),
        var::U16 => (sat(0, 0xffff) as u32 & 0xffff, 2),
        var::I16 => (sat(-0x8000, 0x7fff) as u32 & 0xffff, 2),
        var::U32 | var::I32 => (value as u32, 4),
        var::F32 => ((value as f32).to_bits(), 4),
        _ => return None,
    })
}

/// What storing the `f32` `bits` writes (`AtelSetValueFloat`): the value
/// truncated ([`ftol`]) for integer types, as [`store_int`]; the float
/// itself (through the x87) for `f32`.
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub fn store_float(kind: u32, bits: u32, clamp: bool) -> Option<(u32, u32)> {
    if kind == var::F32 {
        return Some((through_x87(bits), 4));
    }
    store_int(kind, ftol(f32::from_bits(bits)) as i32, clamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workers_by_class() {
        // the field snapshot's event: 35 actors, 3 of 0x558, 3 of 0x2e8
        let c = Classes {
            actors: 35,
            c5b8: 0,
            c558: 3,
            c2e8: 3,
        };
        assert_eq!(worker_offset(1, c), 0xb58);
        assert_eq!(worker_offset(29, c), 29 * 0xb58);
        assert_eq!(worker_offset(30, c), 29 * 0xb58 + 0x558);
        assert_eq!(worker_offset(32, c), 29 * 0xb58 + 3 * 0x558);
        assert_eq!(worker_offset(35, c), 29 * 0xb58 + 3 * 0x558 + 3 * 0x2e8);
        assert_eq!(
            worker_offset(36, c),
            29 * 0xb58 + 3 * 0x558 + 3 * 0x2e8 + 0x30
        );
        assert_eq!(
            buffer_size(36, c),
            29 * 0xb58 + 3 * 0x558 + 3 * 0x2e8 + 0x30
        );
    }

    #[test]
    fn conversions() {
        assert_eq!(ftol(-1.75), (-1i32).cast_unsigned());
        assert_eq!(ftol(f32::NAN), 0x8000_0000);
        assert_eq!(ftol(3e9), 0x8000_0000);
        assert_eq!(ftol(-2_147_483_648.0), 0x8000_0000);
        assert_eq!(through_x87(0x7f80_0001), 0x7fc0_0001);
        assert_eq!(through_x87(0x7f80_0000), 0x7f80_0000);
        assert_eq!(value_int(var::I8, 0x1234_56f0), (-16i32).cast_unsigned());
        assert_eq!(
            value_float(var::U32, 0xffff_ffff),
            4_294_967_296f32.to_bits()
        );
        assert_eq!(store_int(var::U8, 300, true), Some((0xff, 1)));
        assert_eq!(store_int(var::U8, 300, false), Some((0x2c, 1)));
        assert_eq!(store_int(var::I16, -40000, true), Some((0x8000, 2)));
        assert_eq!(store_int(7, 1, true), None);
        assert_eq!(var::clamp_index(5, 0), -1);
        assert_eq!(level_index(5, 3), 1);
        assert_eq!(level_index(2, 30), 8);
    }
}
