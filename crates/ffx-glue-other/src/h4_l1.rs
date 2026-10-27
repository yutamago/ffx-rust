//! Continuation round 10 of list `h4-glue-other`, batch 1: the shapes'
//! VFX render data buffers (`createVFXRenderDataBufferShp`).

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

use crate::h4_k4::{
    ALPHA_FUNCTION, CHR_FROM_CONTENT, DBG_PRINTF, GET_PTR32, GET_RENDER_DATA, MEMSET,
    MONSTER_TEXTURE_NAME, SPRINTF, VFX_CREATE, VFX_CREATE_DATA_BUFFER,
};

/// `MagicFile_GetMagicId()` (`0x009da3b0`, cdecl).
pub const MAGIC_ID: u32 = 0x009d_a3b0;
/// `graphicLevelGetNameHash()` (`0x00641da0`, cdecl).
pub const LEVEL_NAME_HASH: u32 = 0x0064_1da0;
/// `graphicVFXIsTextureWithBpLoaded(number, bpp, 0, 0, w, h, name)`
/// (`0x00645450`, cdecl): writes the texture's name.
pub const IS_TEXTURE_WITH_BP_LOADED: u32 = 0x0064_5450;
/// `g_CurrentMapNo` (`0x012fbc40`).
pub const CURRENT_MAP: u32 = 0x012f_bc40;
/// `g_VfxShapeEnlargeBufferFactor` (`0x00c3a4d8`).
pub const ENLARGE: u32 = 0x00c3_a4d8;
/// `"empty"` (`0x00b4ff8c`, six bytes with its terminator): the name
/// before the first texture.
pub const EMPTY: u32 = 0x00b4_ff8c;
/// The two-textures message (`0x00b4ff98`), the texture-not-loaded
/// message (`0x00b4ffe0`), the no-vertex message (`0x00b50030`).
pub const TWO_TEXTURES: u32 = 0x00b4_ff98;
/// See [`TWO_TEXTURES`].
pub const NOT_LOADED: u32 = 0x00b4_ffe0;
/// See [`TWO_TEXTURES`].
pub const NO_VERTEX: u32 = 0x00b5_0030;
/// Bytes of [`create_shp`]'s frame, from the original's `ebp - 0x68e0`
/// (its cookie excluded): the data buffers' flags (`+0x2c`, 100 words,
/// the first 1), the vertex counts (`+0x1bc`) and index counts (`+0x34c`),
/// 100 words each (only the first is counted; the index words are doubled
/// one per shape), then the name (`+0x4dc`, 0x6400 bytes). The words
/// below `+0x2c` are the original's temporaries, which no callee sees; the
/// name's last word is where the original's `__chkstk` call left `ecx`.
pub const SHP_FRAME: u32 = 0x68dc;
/// Bytes zeroed after each array's first word.
const REST: u32 = 0x18c;

/// The (number, depth, width, height) of a shape's texture: the 64-bit
/// `(+0x14 << 32) + sext(+0x10)`'s low 14 bits, bits 20..25, and the
/// powers of two of bits 26..29 and 30..33.
fn texture(h: &impl Host, sub: u32) -> (u32, u32, u32, u32) {
    let size_lo: u32 = h.read(sub.wrapping_add(0x10));
    let size_hi: u32 = h.read(sub.wrapping_add(0x14));
    let bits = (u64::from(size_hi) << 32).wrapping_add(i64::from(size_lo as i32) as u64);
    let depth = (bits >> 20) as u32 & 0x3f;
    let width = 1u32 << ((bits >> 26) as u32 & 0xf);
    let height = 1u32 << ((bits >> 30) as u32 & 0xf);
    (bits as u32 & 0x3fff, depth, width, height)
}

/// The shape count (`+6`, i16).
fn count(h: &impl Host, shape: u32) -> i32 {
    i32::from(h.read::<i16>(shape.wrapping_add(6)))
}

/// Shape `i`'s record: `shape` + the i16 offset at `+0x10 + 8 i`.
fn record(h: &impl Host, shape: u32, i: u32) -> u32 {
    let entry = shape.wrapping_add(0x10).wrapping_add(i.wrapping_mul(8));
    (i32::from(h.read::<i16>(entry)) as u32).wrapping_add(shape)
}

/// A record's primitive count (`+2`, i16).
fn primitives(h: &impl Host, sub: u32) -> i32 {
    i32::from(h.read::<i16>(sub.wrapping_add(2)))
}

/// A record's alpha modes: byte `+4` of each 8-byte primitive, which
/// start at `sub + [sub + 4] + 4`.
fn alpha_of(h: &impl Host, sub: u32) -> u32 {
    h.read::<u32>(sub.wrapping_add(4))
        .wrapping_add(4)
        .wrapping_add(sub)
}

/// `createVFXRenderDataBufferShp(lo, hi, shapes, unused, flags)`
/// (`0x00715170`, cdecl; `frame` see [`SHP_FRAME`]) -> 1, or 0 when a
/// texture is not loaded, a monster is missing (-1), no shape has
/// primitives or the render data is missing. The shapes
/// (`get_ptr32_func([shapes])`, `+6` count, records at the i16 offsets
/// `+0x10 + 8 i`) give one vertex count (the largest primitive count `+2`
/// times `g_VfxShapeEnlargeBufferFactor` times 12) and index count (times
/// 6, then doubled); every new (number, depth) texture must be loaded
/// (numbers from 0x3f00 name a monster's texture; magic 0x241 maps 0x3cc0
/// to 0x34c0 depth 0x13, level hash 0x1ed9 maps 0x3720 to 0x3721), and
/// a second one is reported. When every primitive's alpha function (byte
/// `+4` of the 8-byte primitives at `+4 +[+4]`) matches the first, or the
/// magic is 7, one instance (`flags` | 8; | 0x800 for magic 0x1e3 on map
/// 0x71) is made, its render data marked (`+0xc0` 0, `+0xc4` 1);
/// otherwise six, ids 10000 apart, one per alpha bucket (`+0xc0` 1,
/// `+0xc4` 6). Either way every render entry's `+0x24` becomes 2.
#[allow(clippy::too_many_lines)] // the textures, then the instances
pub fn create_shp(h: &mut impl Host, frame: u32, lo: u32, hi: u32, shapes: u32, flags: u32) -> u32 {
    let flag_words = frame + 0x2c;
    let vertices = frame + 0x1bc;
    let indices = frame + 0x34c;
    let name = frame + 0x4dc;
    h.write(vertices, 0u32);
    h.call_sym(MEMSET, Conv::Cdecl, &[vertices + 4, 0, REST]);
    h.write(indices, 0u32);
    h.call_sym(MEMSET, Conv::Cdecl, &[indices + 4, 0, REST]);
    h.write(flag_words, 1u32);
    h.call_sym(MEMSET, Conv::Cdecl, &[flag_words + 4, 0, REST]);
    let magic = h.call_sym(MAGIC_ID, Conv::Cdecl, &[]);
    let mut flags = flags;
    if magic == 0x1e3 && h.read::<u32>(h.rebase(CURRENT_MAP)) == 0x71 {
        flags |= 0x800;
    }
    let empty = h.rebase(EMPTY);
    let head: u32 = h.read(empty);
    let tail: u16 = h.read(empty + 4);
    h.write(name, head);
    h.write(name + 4, tail);
    let pointer: u32 = h.read(shapes);
    let shape = h.call_sym(GET_PTR32, Conv::Cdecl, &[pointer]);
    let (mut current, mut current_depth) = (0u32, 0u32);
    let mut i = 0i32;
    while i < count(h, shape) {
        let sub = record(h, shape, i as u32);
        let factor: i32 = h.read(h.rebase(ENLARGE));
        let v = primitives(h, sub).wrapping_mul(factor).wrapping_mul(12);
        if h.read::<i32>(vertices) < v {
            h.write(vertices, v);
        }
        let x = primitives(h, sub).wrapping_mul(factor).wrapping_mul(6);
        if h.read::<i32>(indices) < x {
            h.write(indices, x);
        }
        let (mut number, mut depth, width, height) = texture(h, sub);
        if h.call_sym(MAGIC_ID, Conv::Cdecl, &[]) == 0x241 && number == 0x3cc0 {
            number = 0x34c0;
            depth = 0x13;
        } else if h.call_sym(LEVEL_NAME_HASH, Conv::Cdecl, &[]) == 0x1ed9 && number == 0x3720 {
            number = 0x3721;
        }
        if number != current || depth != current_depth {
            if current != 0 {
                let text = h.rebase(TWO_TEXTURES);
                h.call_sym(DBG_PRINTF, Conv::Cdecl, &[text]);
            }
            let mut copied = false;
            if number >= 0x3f00 {
                let chr = h.call_sym(CHR_FROM_CONTENT, Conv::Cdecl, &[shape]);
                if chr == u32::MAX {
                    return 0;
                }
                if chr != 0 {
                    let text = h.call_sym(MONSTER_TEXTURE_NAME, Conv::Cdecl, &[chr, number]);
                    if text != 0 {
                        // the inlined strcpy
                        let mut k = 0u32;
                        loop {
                            let b: u8 = h.read(text.wrapping_add(k));
                            h.write(name.wrapping_add(k), b);
                            if b == 0 {
                                break;
                            }
                            k = k.wrapping_add(1);
                        }
                        copied = true;
                    }
                }
            }
            if !copied {
                let loaded = h.call_sym(
                    IS_TEXTURE_WITH_BP_LOADED,
                    Conv::Cdecl,
                    &[number, depth, 0, 0, width, height, name],
                );
                if loaded == 0 {
                    let text = h.rebase(NOT_LOADED);
                    h.call_sym(
                        DBG_PRINTF,
                        Conv::Cdecl,
                        &[text, number, depth, width, height],
                    );
                    return 0;
                }
            }
            current = number;
            current_depth = depth;
        }
        i += 1;
    }
    if h.read::<u32>(vertices) == 0 || h.read::<u32>(indices) == 0 {
        let text = h.rebase(NO_VERTEX);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[text]);
        return 0;
    }
    flags |= 8;
    // one index word doubled per shape, past the array for more than 100
    let mut c = 0i32;
    while c < count(h, shape) {
        let at = indices.wrapping_add(4 * c as u32);
        let n: u32 = h.read(at);
        h.write(at, n.wrapping_add(n));
        c += 1;
    }
    let first = record(h, shape, 0);
    let mode = u32::from(h.read::<u8>(alpha_of(h, first)));
    let alpha = h.call_sym(ALPHA_FUNCTION, Conv::Cdecl, &[mode]);
    let mut mixed = false;
    let mut i = 0i32;
    'shapes: while i < count(h, shape) {
        let sub = record(h, shape, i as u32);
        let mut at = alpha_of(h, sub);
        let mut j = 0i32;
        while j < primitives(h, sub) {
            let mode = u32::from(h.read::<u8>(at));
            if h.call_sym(ALPHA_FUNCTION, Conv::Cdecl, &[mode]) != alpha {
                mixed = magic != 7;
                break 'shapes;
            }
            j += 1;
            at = at.wrapping_add(8);
        }
        i += 1;
    }
    let (marked, kinds) = if mixed {
        let f = flags & 0xffff_fccd;
        let mut id = u64::from(hi) << 32 | u64::from(lo);
        for bits in [0, 2, 0x10, 0x20, 0x100, 0x200] {
            let (id_lo, id_hi) = (id as u32, (id >> 32) as u32);
            h.call_sym(VFX_CREATE, Conv::Cdecl, &[id_lo, id_hi, f | bits]);
            h.call_sym(
                VFX_CREATE_DATA_BUFFER,
                Conv::Cdecl,
                &[id_lo, id_hi, 1, vertices, indices, name, flag_words],
            );
            id = id.wrapping_add(10_000);
        }
        (1u8, 6u16)
    } else {
        h.call_sym(VFX_CREATE, Conv::Cdecl, &[lo, hi, flags]);
        h.call_sym(
            VFX_CREATE_DATA_BUFFER,
            Conv::Cdecl,
            &[lo, hi, 1, vertices, indices, name, flag_words],
        );
        (0u8, 1u16)
    };
    let data = h.call_sym(GET_RENDER_DATA, Conv::Cdecl, &[lo, hi]);
    if data == 0 {
        return 0;
    }
    h.write(data.wrapping_add(0xc0), marked);
    h.write(data.wrapping_add(0xc4), kinds);
    let mut k = 0i32;
    while k < h.read::<i32>(data.wrapping_add(0x90)) {
        let entries: u32 = h.read(data.wrapping_add(0x94));
        h.write(entries.wrapping_add(0x6c * k as u32 + 0x24), 2u32);
        k += 1;
    }
    1
}

// ---- MenuManager_GetMenuVertexInfo

/// `MenuManager_gatherMenuBuffers_0067b530(this; chain)` (thiscall, ret 4).
pub const GATHER: u32 = 0x0067_b530;
/// `MenuManager_TakeMultiRecord_0067c170(this; chain)` (thiscall, ret 4).
pub const TAKE_MULTI: u32 = 0x0067_c170;
/// `MenuManager_TakeSingleRecord_0067c210(this; chain)` (thiscall, ret 4).
pub const TAKE_SINGLE: u32 = 0x0067_c210;
/// `MenuVertexInfo_move_0067dc70(from, to)` (stdcall).
pub const MOVE: u32 = 0x0067_dc70;
/// `MenuManager_findNameIndex_0067e310(this; name)` (thiscall, ret 4):
/// -1 when the name is not in the table.
pub const FIND_NAME: u32 = 0x0067_e310;
/// `MenuManager_SeveralStoresCopyIntoTable_0067e4a0(this; slot, name)`
/// (thiscall, ret 8).
pub const STORE_NAME: u32 = 0x0067_e4a0;
/// `g_MenuVertexNameUses` (`0x00ccc7ec`): how often each of the six
/// table names was handed out since it was stored.
pub const NAME_USES: u32 = 0x00cc_c7ec;
/// `"Unitialized TextureName"` (`0x00b45ba0`).
pub const UNINITIALIZED: u32 = 0x00b4_5ba0;

/// A chain's multi-record pool and draw list, as the original addresses
/// them: chain 0 keeps its pool at `+0x15e8` (count, high bit masked) and
/// `+0x15ec` (records), its used count at `+0x2bfc`, its single records'
/// used count at `+0x2bf8` and its draw list's last entry at `-4 + 4 n`
/// (n `+0x2c50`); chain 1 at `+0x4240`, `+0x4244`, `+0x583c`, `+0x5838`
/// and `+0x2c54 + 4 n` (n `+0x5840`).
struct Chain {
    pool: u32,
    records: u32,
    used: u32,
    singles: u32,
    list: u32,
    count: u32,
}

const CHAINS: [Chain; 2] = [
    Chain {
        pool: 0x15e8,
        records: 0x15ec,
        used: 0x2bfc,
        singles: 0x2bf8,
        list: 0xffff_fffc,
        count: 0x2c50,
    },
    Chain {
        pool: 0x4240,
        records: 0x4244,
        used: 0x583c,
        singles: 0x5838,
        list: 0x2c54,
        count: 0x5840,
    },
];

/// The two C strings at `a` and `b` are equal.
fn same_text(h: &impl Host, a: u32, b: u32) -> bool {
    let mut k = 0u32;
    loop {
        let x: u8 = h.read(a.wrapping_add(k));
        if x != h.read::<u8>(b.wrapping_add(k)) {
            return false;
        }
        if x == 0 {
            return true;
        }
        k = k.wrapping_add(1);
    }
}

/// A multi-record of `chain`: chain 0 through `TakeMultiRecord`, chain 1
/// inline (clears `+0x5b74`, then the pool's next record, or 0 when it is
/// used up, appended to the draw list).
fn take_multi(h: &mut impl Host, this: u32, chain: u32) -> u32 {
    if chain == 0 {
        return h.call_sym(TAKE_MULTI, Conv::Thiscall(this), &[0]);
    }
    let at = |off: u32| this.wrapping_add(off);
    h.write(at(0x5b74), 0u8);
    let used: u32 = h.read(at(0x583c));
    if used >= h.read::<u32>(at(0x4240)) & 0x7fff_ffff {
        return 0;
    }
    let records: u32 = h.read(at(0x4244));
    let record: u32 = h.read(records.wrapping_add(used.wrapping_mul(4)));
    let count: u32 = h.read(at(0x5840));
    h.write(at(0x2c58).wrapping_add(count.wrapping_mul(4)), record);
    let used: u32 = h.read(at(0x583c));
    h.write(at(0x583c), used.wrapping_add(1));
    let count: u32 = h.read(at(0x5840));
    h.write(at(0x5840), count.wrapping_add(1));
    record
}

/// `MenuManager_GetMenuVertexInfo(this; name, kind)` (`0x0067ba50`,
/// thiscall, ret 8): the vertex record a menu draw of texture `name`
/// fills, from chain `+0x5b7c` (0 or 1; other chains get 0). Kinds 1, 2
/// and 10 take a single-quad record (chain 1 inline, returning `kind`
/// when its pool is used up). In batch mode (`+0x5b8c` 1) up to six
/// names share multi-records: a known name (`findNameIndex`) reuses its
/// record while used fewer than 12 times, a new one is stored with a
/// fresh record; past six names or 12 uses the batch is gathered and
/// starts over. Otherwise the same name as last time (`+0x5b88`) keeps
/// filling its record (up to 12 quads; when `+0x5b74` is set its single
/// record is moved into a multi-record first, or 0 when the pool is used
/// up); a new name takes a single record and sets `+0x5b74`. The name is
/// remembered in `+0x5b88`.
#[allow(clippy::too_many_lines)] // the original's cases one by one
pub fn get_menu_vertex_info(h: &mut impl Host, this: u32, name: u32, kind: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    let chain: u32 = h.read(at(0x5b7c));
    if chain > 1 {
        return 0;
    }
    let me = Conv::Thiscall(this);
    if kind == 1 || kind == 10 || kind == 2 {
        if h.read::<u32>(at(0x5844)) != 0 {
            h.call_sym(GATHER, me, &[chain]);
            h.write(at(0x5844), 0u32);
        }
        let text = h.rebase(UNINITIALIZED);
        h.write(at(0x5b88), text);
        if chain == 0 {
            return h.call_sym(TAKE_SINGLE, me, &[0]);
        }
        let used: u32 = h.read(at(0x5838));
        if used >= h.read::<u32>(at(0x4238)) & 0x7fff_ffff {
            return kind;
        }
        let records: u32 = h.read(at(0x423c));
        let record: u32 = h.read(records.wrapping_add(used.wrapping_mul(4)));
        let count: u32 = h.read(at(0x5840));
        h.write(at(0x2c58).wrapping_add(count.wrapping_mul(4)), record);
        let used: u32 = h.read(at(0x5838));
        let records: u32 = h.read(at(0x423c));
        let record: u32 = h.read(records.wrapping_add(used.wrapping_mul(4)));
        let count: u32 = h.read(at(0x5840));
        h.write(at(0x5840), count.wrapping_add(1));
        h.write(at(0x5838), used.wrapping_add(1));
        return record;
    }
    let c = &CHAINS[chain as usize];
    if h.read::<u32>(at(0x5b8c)) == 1 {
        let n: u32 = h.read(at(0x5844));
        if n == 0 {
            let record = take_multi(h, this, chain);
            h.call_sym(STORE_NAME, me, &[0, name]);
            let n: u32 = h.read(at(0x5844));
            h.write(at(0x5844), n.wrapping_add(1));
            h.write(at(0x5b88), name);
            return record;
        }
        if n < 6 {
            let index = h.call_sym(FIND_NAME, me, &[name]);
            if index == u32::MAX {
                let record = take_multi(h, this, chain);
                let slot: u32 = h.read(at(0x5844));
                h.call_sym(STORE_NAME, me, &[slot, name]);
                let n: u32 = h.read(at(0x5844));
                h.write(at(0x5844), n.wrapping_add(1));
                h.write(at(0x5b88), name);
                return record;
            }
            let uses = h.rebase(NAME_USES).wrapping_add(index.wrapping_mul(4));
            let used: i32 = h.read(uses);
            if used < 12 {
                let records: u32 = h.read(at(c.records));
                let k = h
                    .read::<u32>(at(c.used))
                    .wrapping_sub(n)
                    .wrapping_add(index);
                let record: u32 = h.read(records.wrapping_add(k.wrapping_mul(4)));
                h.write(uses, used.wrapping_add(1));
                h.write(at(0x5b88), name);
                return record;
            }
        }
        h.call_sym(GATHER, me, &[chain]);
        let record = take_multi(h, this, chain);
        h.call_sym(STORE_NAME, me, &[0, name]);
        h.write(at(0x5b88), name);
        h.write(at(0x5844), 1u32);
        return record;
    }
    let mut same = false;
    if h.read::<u32>(at(0x5844)) != 0 {
        h.call_sym(GATHER, me, &[chain]);
        h.write(at(0x5844), 0u32);
    } else {
        let last: u32 = h.read(at(0x5b88));
        same = same_text(h, last, name);
    }
    if same {
        if h.read::<u8>(at(0x5b74)) == 1 {
            let used: u32 = h.read(at(c.used));
            if used >= h.read::<u32>(at(c.pool)) & 0x7fff_ffff {
                h.write(at(0x5b88), name);
                return 0;
            }
            let records: u32 = h.read(at(c.records));
            let to: u32 = h.read(records.wrapping_add(used.wrapping_mul(4)));
            let count: u32 = h.read(at(c.count));
            let from: u32 = h.read(at(c.list).wrapping_add(count.wrapping_mul(4)));
            h.call_sym(MOVE, Conv::Stdcall, &[from, to]);
            let used: u32 = h.read(at(c.used));
            let records: u32 = h.read(at(c.records));
            let moved: u32 = h.read(records.wrapping_add(used.wrapping_mul(4)));
            let count: u32 = h.read(at(c.count));
            h.write(at(c.list).wrapping_add(count.wrapping_mul(4)), moved);
            let used: u32 = h.read(at(c.used));
            let records: u32 = h.read(at(c.records));
            let record: u32 = h.read(records.wrapping_add(used.wrapping_mul(4)));
            let singles: u32 = h.read(at(c.singles));
            h.write(at(c.singles), singles.wrapping_sub(1));
            let quads: u32 = h.read(at(0x5b78));
            h.write(at(0x5b78), quads.wrapping_add(2));
            h.write(at(c.used), used.wrapping_add(1));
            h.write(at(0x5b88), name);
            h.write(at(0x5b74), 0u8);
            return record;
        }
        let quads: u32 = h.read(at(0x5b78));
        if quads < 12 {
            let records: u32 = h.read(at(c.records));
            let used: u32 = h.read(at(c.used));
            let record: u32 = h.read(records.wrapping_add(used.wrapping_mul(4)).wrapping_sub(4));
            h.write(at(0x5b78), quads.wrapping_add(1));
            h.write(at(0x5b88), name);
            h.write(at(0x5b74), 0u8);
            return record;
        }
    }
    let record = h.call_sym(TAKE_SINGLE, me, &[chain]);
    h.write(at(0x5b74), 1u8);
    h.write(at(0x5b78), 1u32);
    h.write(at(0x5b88), name);
    record
}

// ---- ChrResourceCacheManager_applyDelayedRelease

/// `g_ClusterManager` (`0x00ccca4c`).
pub const CLUSTER_MANAGER: u32 = 0x00cc_ca4c;
/// `WaitForSingleObject`'s and `ReleaseMutex`'s import slots (stdcall).
pub const WAIT_FOR_SINGLE_OBJECT: Global<u32> = Global::new(0x00b0_c0f8);
/// See [`WAIT_FOR_SINGLE_OBJECT`].
pub const RELEASE_MUTEX: Global<u32> = Global::new(0x00b0_c1c0);
/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre_PArray_pushInto4_0054bc40(array; buffer, &value)` (thiscall,
/// ret 8).
pub const PUSH_KEY: u32 = 0x0054_bc40;
/// `PArray_PushInto_0069b5c0(array; buffer, &value)` (thiscall, ret 8).
pub const PUSH_VALUE: u32 = 0x0069_b5c0;
/// `Phyre_PCluster_dtor_PCluster(cluster)` (thiscall).
pub const CLUSTER_DTOR: u32 = 0x0043_fa30;
/// `operator_delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `std_map_u32_equal_range_0069a250(map; out, &key)` (thiscall, ret 8)
/// -> `out`.
pub const EQUAL_RANGE: u32 = 0x0069_a250;
/// `ClusterMap_eraseRange_0069af70(map; out, first, last)` (thiscall,
/// ret 0xc).
pub const ERASE_RANGE: u32 = 0x0069_af70;
/// Bytes of [`apply_delayed_release`]'s frame, from the original's `ebp -
/// 0x40`: the equal range (`+0`, two words), the released clusters'
/// array (`+8` count, `+0xc` data), the keys' array (`+0x10`, `+0x14`)
/// and the erase's result (`+0x18`).
pub const RELEASE_FRAME: u32 = 0x1c;

/// The in-order successor of `node` in an MSVC tree (left `+0`, parent
/// `+4`, right `+8`, nil flag `+0xd`); a nil node stays.
fn successor(h: &impl Host, node: u32) -> u32 {
    let nil = |n: u32| h.read::<u8>(n.wrapping_add(0xd)) != 0;
    if nil(node) {
        return node;
    }
    let right: u32 = h.read(node.wrapping_add(8));
    if !nil(right) {
        let mut n = right;
        loop {
            let left: u32 = h.read(n);
            if nil(left) {
                return n;
            }
            n = left;
        }
    }
    let mut n = node;
    let mut parent: u32 = h.read(node.wrapping_add(4));
    while !nil(parent) && n == h.read::<u32>(parent.wrapping_add(8)) {
        n = parent;
        parent = h.read(parent.wrapping_add(4));
    }
    parent
}

/// `ChrResourceCacheManager_applyDelayedRelease(this)` (`0x0069a6b0`,
/// `ecx`, plain ret; `frame` see [`RELEASE_FRAME`]) -> `this` (its `eax`;
/// the SEH frame is not kept). In states 0 and 1 (`+0`), under the
/// cluster manager's mutex (`[[+0x18]]`), every map entry (`+0x10`, key
/// `+0x10`, info `+0x14`) whose info has no users (`+4` < 1) and is
/// pending (`+0xc`) is collected (key and info pushed into two arrays),
/// and every info's pending flag is cleared. Then each collected cluster
/// (`[info]`) is destroyed and freed, the info returned to the manager's
/// pool (`+0x1c`, 0x18-byte slots, free flags `+0x601c`, lowest free
/// `+0x641c`) or deleted, and its key erased from the map. The arrays are
/// freed when owned; the state becomes 2 with `+4` 0.0. The original
/// also looks each key up in `this`'s table (`+8` count, `+0xc` pairs)
/// and drops the result; the port skips that read-only loop.
pub fn apply_delayed_release(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let state: u32 = h.read(this);
    if state <= 1 {
        let manager: u32 = h.read(h.rebase(CLUSTER_MANAGER));
        let range = frame;
        let infos = frame + 8;
        let keys = frame + 0x10;
        let erased = frame + 0x18;
        for at in [infos, infos + 4, keys, keys + 4] {
            h.write(at, 0u32);
        }
        let (mut key_cap, mut info_cap) = (0u32, 0u32);
        let mutex: u32 = h.read(manager.wrapping_add(0x18));
        let handle: u32 = h.read(mutex);
        let wait = WAIT_FOR_SINGLE_OBJECT.get(h);
        h.call(wait, Conv::Stdcall, &[handle, u32::MAX]);
        let map = manager.wrapping_add(0x10);
        let head: u32 = h.read(map);
        let mut node: u32 = h.read(head);
        if node != head {
            loop {
                let info: u32 = h.read(node.wrapping_add(0x14));
                if h.read::<i32>(info.wrapping_add(4)) < 1
                    && h.read::<u32>(info.wrapping_add(0xc)) != 0
                {
                    let size = (key_cap & 0x7fff_ffff).wrapping_mul(4).wrapping_add(4);
                    let buffer = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 4]);
                    h.call_sym(
                        PUSH_KEY,
                        Conv::Thiscall(keys),
                        &[buffer, node.wrapping_add(0x10)],
                    );
                    let size = (info_cap & 0x7fff_ffff).wrapping_mul(4).wrapping_add(4);
                    let buffer = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 4]);
                    h.call_sym(
                        PUSH_VALUE,
                        Conv::Thiscall(infos),
                        &[buffer, node.wrapping_add(0x14)],
                    );
                    key_cap = h.read(keys);
                    info_cap = h.read(infos);
                }
                let info: u32 = h.read(node.wrapping_add(0x14));
                h.write(info.wrapping_add(0xc), 0u32);
                node = successor(h, node);
                if node == h.read::<u32>(map) {
                    break;
                }
            }
        }
        let mut n = key_cap & 0x7fff_ffff;
        if n != 0 {
            let mut key: u32 = h.read(keys + 4);
            let delta = h.read::<u32>(infos + 4).wrapping_sub(key);
            while n != 0 {
                let info: u32 = h.read(key.wrapping_add(delta));
                let cluster: u32 = h.read(info);
                if cluster != 0 {
                    h.call_sym(CLUSTER_DTOR, Conv::Thiscall(cluster), &[]);
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[cluster]);
                }
                let pool: u32 = h.read(h.rebase(CLUSTER_MANAGER));
                let info: u32 = h.read(key.wrapping_add(delta));
                if info >= pool.wrapping_add(0x1c) && info < pool.wrapping_add(0x601c) {
                    let slot = info.wrapping_sub(pool).wrapping_sub(0x1c) / 0x18;
                    h.write(pool.wrapping_add(0x601c).wrapping_add(slot), 0u8);
                    let lowest = pool.wrapping_add(0x641c);
                    if (slot as i32) < h.read::<i32>(lowest) {
                        h.write(lowest, slot);
                    }
                } else {
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[info]);
                }
                let out = h.call_sym(EQUAL_RANGE, Conv::Thiscall(map), &[range, key]);
                let first: u32 = h.read(out);
                let last: u32 = h.read(out.wrapping_add(4));
                h.call_sym(ERASE_RANGE, Conv::Thiscall(map), &[erased, first, last]);
                key = key.wrapping_add(4);
                n -= 1;
            }
        }
        let mutex: u32 = h.read(manager.wrapping_add(0x18));
        let handle: u32 = h.read(mutex);
        let release = RELEASE_MUTEX.get(h);
        h.call(release, Conv::Stdcall, &[handle]);
        if info_cap >> 31 == 0 {
            let data: u32 = h.read(infos + 4);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        if key_cap >> 31 == 0 {
            let data: u32 = h.read(keys + 4);
            if data != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
        }
    }
    h.write(this.wrapping_add(4), 0u32);
    h.write(this, 2u32);
    this
}

// ---- FmodVoice_dataChange

/// `FmodVoice_releaseData(this)` (thiscall).
pub const RELEASE_DATA: u32 = 0x0070_b0b0;
/// `getFullFilename(out, path)` (cdecl).
pub const FULL_FILENAME: u32 = 0x0070_7aa0;
/// `ERRCHECK(result, file, line)` (cdecl).
pub const ERRCHECK: u32 = 0x0070_79d0;
/// `printToOutput(prefix, format, ...)` (cdecl).
pub const PRINT_TO_OUTPUT: u32 = 0x0070_7b20;
/// `FMOD::EventSystem::load(system, name, info, &project)`'s thunk
/// (`0x00948c94`, stdcall).
pub const EVENT_SYSTEM_LOAD: u32 = 0x0094_8c94;
/// `g_FmodVoiceNoPlay` (`0x00cec184`) and `g_FmodMusicNoLoad`
/// (`0x00cec174`).
pub const VOICE_NO_PLAY: u32 = 0x00ce_c184;
/// See [`VOICE_NO_PLAY`].
pub const MUSIC_NO_LOAD: u32 = 0x00ce_c174;
/// `"Voice/US/"` (`0x00b4fa74`), `"Voice/JP/"` (`0x00b4fa80`),
/// `"%s%s.fev"` (`0x00b4fa8c`), the source file's name (`0x00b4f8c0`),
/// `"voice"` (`0x00b4f98c`), the null-project message (`0x00b4fa98`)
/// and `"[Fmod Error]"` (`0x00b4c30c`).
pub const VOICE_US: u32 = 0x00b4_fa74;
/// See [`VOICE_US`].
pub const VOICE_JP: u32 = 0x00b4_fa80;
/// See [`VOICE_US`].
pub const FEV_FORMAT: u32 = 0x00b4_fa8c;
/// See [`VOICE_US`].
pub const SOURCE_FILE: u32 = 0x00b4_f8c0;
/// See [`VOICE_US`].
pub const VOICE_GROUP: u32 = 0x00b4_f98c;
/// See [`VOICE_US`].
pub const NULL_PROJECT: u32 = 0x00b4_fa98;
/// See [`VOICE_US`].
pub const FMOD_ERROR: u32 = 0x00b4_c30c;
/// The banks whose event data loads at once, by voice data: 0x170 the
/// two at `0x00b4fae0` and `0x00b4faf0`, 0x10e those at `0x00b4fb00` and
/// `0x00b4fb10`, 0x7f the one at `0x00b4fb20`.
pub const PRELOAD: [(u32, &[u32]); 3] = [
    (0x170, &[0x00b4_fae0, 0x00b4_faf0]),
    (0x10e, &[0x00b4_fb00, 0x00b4_fb10]),
    (0x7f, &[0x00b4_fb20]),
];
/// Bytes of [`data_change`]'s frame, from the original's `ebp - 0x204`:
/// the voice folder (`+0`) and the bank's file name (`+0x100`), 0x100
/// bytes each.
pub const VOICE_FRAME: u32 = 0x200;

/// The length of the string at `s`.
fn text_len(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `FmodVoice_dataChange(this; id, system)` (`0x0070a630`, thiscall, ret
/// 8; `frame` see [`VOICE_FRAME`]) -> 1; 0 when `id` is not 1..500,
/// `system` is null or `id` is the current data (`+0`); -1 when a bank's
/// project is missing. Makes `id` current and releases the old data;
/// ids 0x189..0x18e set `g_FmodVoiceNoPlay`, which (set) also sets
/// `g_FmodMusicNoLoad`. Each entry of the voice table (`+0x28`: `"ML"`,
/// count, then (id, name offset) pairs) for `id` loads the bank
/// `"<folder><name>.fev"` (folder by language `+4`) into project slot `k`
/// (`+0x18`): 0 with `g_FmodVoiceNoPlay` set or data 0x10e, else the
/// name's last two characters as a decimal number. Its `"voice"` group
/// goes to `+0xc` slot `k`, and the [`PRELOAD`] banks load their event
/// data. Every FMOD result goes through `ERRCHECK`.
#[allow(clippy::too_many_lines)] // the bank loop
pub fn data_change(h: &mut impl Host, frame: u32, this: u32, id: u32, system: u32) -> u32 {
    if id.wrapping_sub(1) > 499 || system == 0 || id == h.read::<u32>(this) {
        return 0;
    }
    h.write(this, id);
    h.call_sym(RELEASE_DATA, Conv::Thiscall(this), &[]);
    let no_play = h.rebase(VOICE_NO_PLAY);
    if (0x189..=0x18e).contains(&id) {
        h.write(no_play, 1u32);
        let music = h.rebase(MUSIC_NO_LOAD);
        h.write(music, 1u32);
    } else if h.read::<u32>(no_play) == 1 {
        let music = h.rebase(MUSIC_NO_LOAD);
        h.write(music, 1u32);
    }
    let folder = frame;
    let file = frame + 0x100;
    h.write(folder, 0u8);
    h.call_sym(MEMSET, Conv::Cdecl, &[folder + 1, 0, 0xff]);
    let language = if h.read::<u8>(this.wrapping_add(4)) == 0 {
        VOICE_JP
    } else {
        VOICE_US
    };
    let language = h.rebase(language);
    h.call_sym(FULL_FILENAME, Conv::Cdecl, &[folder, language]);
    let table: u32 = h.read(this.wrapping_add(0x28));
    let magic: u32 = h.read(table);
    if magic != u32::from_le_bytes(*b"ML\0\0") {
        return 1;
    }
    let count: u32 = h.read(table.wrapping_add(4));
    let source = h.rebase(SOURCE_FILE);
    let mut entry = table.wrapping_add(8);
    for _ in 0..count {
        if h.read::<u32>(entry) == id {
            let data: u32 = h.read(this.wrapping_add(0x28));
            let name = data.wrapping_add(h.read(entry.wrapping_add(4)));
            let slot = if h.read::<u32>(no_play) == 1 || h.read::<u32>(this) == 0x10e {
                0
            } else {
                // the last two characters as a two-digit number
                let len = text_len(h, name);
                let tens = i32::from(h.read::<i8>(name.wrapping_add(len).wrapping_sub(2)));
                let ones = i32::from(h.read::<i8>(name.wrapping_add(len).wrapping_sub(1)));
                (ones + (tens * 5 - 0x108) * 2) as u32
            };
            h.write(file, 0u8);
            h.call_sym(MEMSET, Conv::Cdecl, &[file + 1, 0, 0xff]);
            let sprintf = SPRINTF.get(h);
            let format = h.rebase(FEV_FORMAT);
            h.call(sprintf, Conv::Cdecl, &[file, format, folder, name]);
            let offset = slot.wrapping_mul(4);
            let projects: u32 = h.read(this.wrapping_add(0x18));
            let result = h.call_sym(
                EVENT_SYSTEM_LOAD,
                Conv::Stdcall,
                &[system, file, 0, projects.wrapping_add(offset)],
            );
            h.call_sym(ERRCHECK, Conv::Cdecl, &[result, source, 0x22f]);
            let projects: u32 = h.read(this.wrapping_add(0x18));
            let project: u32 = h.read(projects.wrapping_add(offset));
            if project == 0 {
                let prefix = h.rebase(FMOD_ERROR);
                let text = h.rebase(NULL_PROJECT);
                h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, text, slot, file]);
                return u32::MAX;
            }
            let vtable: u32 = h.read(project);
            let get_group: u32 = h.read(vtable.wrapping_add(8));
            let groups: u32 = h.read(this.wrapping_add(0xc));
            let group_name = h.rebase(VOICE_GROUP);
            let result = h.call(
                get_group,
                Conv::Stdcall,
                &[project, group_name, 0, groups.wrapping_add(offset)],
            );
            h.call_sym(ERRCHECK, Conv::Cdecl, &[result, source, 0x238]);
            let current: u32 = h.read(this);
            let preload = PRELOAD
                .iter()
                .find(|(data, _)| *data == current)
                .is_some_and(|(_, banks)| {
                    banks.iter().any(|&b| {
                        let bank = h.rebase(b);
                        same_text(h, name, bank)
                    })
                });
            if preload {
                let groups: u32 = h.read(this.wrapping_add(0xc));
                let group: u32 = h.read(groups.wrapping_add(offset));
                let vtable: u32 = h.read(group);
                let load: u32 = h.read(vtable.wrapping_add(4));
                let result = h.call(load, Conv::Stdcall, &[group, 1, 0]);
                h.call_sym(ERRCHECK, Conv::Cdecl, &[result, source, 0x23f]);
            }
        }
        entry = entry.wrapping_add(8);
    }
    1
}
