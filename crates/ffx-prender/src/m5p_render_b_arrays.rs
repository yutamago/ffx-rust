//! M5 stage 1, list `phyre-render-b`: `PArray`/`PSharray` helpers of the
//! render classes (element accessors, the class descriptors' array
//! allocate-and-copy and resize functions, external buffers), the render
//! thread's command slot hand-over, the utilities' list unlink, and the
//! fog manager's setters.
//!
//! A `PSharray` keeps its count (bit 31: the buffer is not owned) and,
//! for more than one element, a buffer pointer; a single element sits in
//! the pointer's word itself.

use ffx_bridge::x87;

/// `Phyre_PhyreMalloc_0042fb80(size, align)` and `Phyre_PhyreFree(p)`
/// (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre_ZeroArray28_0058b070(items, n)` (cdecl).
pub const ZERO_ARRAY_28: u32 = 0x0058_b070;
/// `Phyre_PEventFlagWin32_signal` (thiscall).
pub const EVENT_SIGNAL: u32 = 0x0043_0200;
/// `Phyre_PRendering_PRenderThread_GetInstance` (cdecl).
pub const RENDER_THREAD_INSTANCE: u32 = 0x0061_4410;
/// `0x006152a0`: the render thread's command slot hand-over (thiscall,
/// four words).
pub const HAND_OVER: u32 = 0x0061_52a0;
/// `Phyre_PRendering_PRenderRingBufferSegmented_nextSegment_00613eb0`
/// (thiscall).
pub const NEXT_SEGMENT: u32 = 0x0061_3eb0;
/// `0x005aafb0`: the `PArray<PUInt8 *>` allocate-and-copy (cdecl).
pub const ALLOC_COPY_U8P: u32 = 0x005a_afb0;
/// `0x00603dc0`: `PSharray<PPhysicsShape *>`'s external buffer (thiscall).
pub const SHAPES_SET_EXTERNAL: u32 = 0x0060_3dc0;
/// `Phyre_PClassDescriptor_applyDefaultBufferWithValidation` (thiscall,
/// `(buffer, count)`).
pub const APPLY_DEFAULT_BUFFER: u32 = 0x0043_c160;
/// The `PSharray<PPhysicsShape *>` member's descriptor, whose slot 1 the
/// `0x00603e50` setter asks for the type.
pub const SHAPES_MEMBER: u32 = 0x00cb_f358;
/// The utilities' registry list head and the `PUtility` base vtable.
pub const UTILITY_LIST: u32 = 0x00c9_01a8;
pub const UTILITY_VTABLE: u32 = 0x00b0_e078;
/// `operator_delete` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `FogEffectManager_isMoving_006e3480` (fastcall `(fog)` -> `al`).
pub const FOG_IS_MOVING: u32 = 0x006e_3480;
/// `PSamplerStateD3D11_release_0059c770` and `PSpriteArrays_release_00495a10`
/// (thiscall).
pub const SAMPLER_RELEASE: u32 = 0x0059_c770;
pub const SPRITE_ARRAYS_RELEASE: u32 = 0x0049_5a10;

/// A `PSharray` element accessor: the array's count word at `count`
/// (the buffer word after it), elements of `stride` bytes, the field at
/// `field`; `bounded` returns 0 for `i` past the count; `flag` a byte
/// whose bit 0 must be set too; `address` returns the field's address
/// instead of reading it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShField {
    pub count: u32,
    pub stride: u32,
    pub field: u32,
    pub bounded: bool,
    pub flag: Option<u32>,
    pub address: bool,
}

const fn read(count: u32, stride: u32, field: u32) -> ShField {
    ShField {
        count,
        stride,
        field,
        bounded: true,
        flag: None,
        address: false,
    }
}

const fn flagged(count: u32, field: u32, flag: u32) -> ShField {
    ShField {
        count,
        stride: 20,
        field,
        bounded: true,
        flag: Some(flag),
        address: false,
    }
}

/// The accessors `(address, shape)`.
pub const SH_FIELDS: [(u32, ShField); 12] = [
    (0x0059_3ca0, read(0xc, 16, 0)),
    (0x0059_3d10, read(0x14, 20, 0)),
    (0x0059_3d60, read(0x14, 20, 4)),
    (0x0059_3de0, read(0x28, 20, 4)),
    (0x0059_3eb0, read(0x28, 20, 0)),
    (0x0059_3f00, read(0x20, 20, 0)),
    (0x0059_3f90, read(0x14, 20, 8)),
    (0x0059_4010, read(0x28, 20, 8)),
    (0x0059_4730, flagged(0x18, 4, 0x34)),
    (0x0059_4780, flagged(0x14, 4, 0x30)),
    (0x0059_4830, flagged(0x18, 8, 0x34)),
    (
        0x005b_8840,
        ShField {
            count: 0x28,
            stride: 20,
            field: 0x10,
            bounded: false,
            flag: None,
            address: true,
        },
    ),
];

/// The [`SH_FIELDS`] shape of `f`.
#[must_use]
pub const fn sh_field_of(f: u32) -> ShField {
    let mut i = 0;
    while i < SH_FIELDS.len() {
        if SH_FIELDS[i].0 == f {
            return SH_FIELDS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// How an allocate-and-copy fills the elements past the copied ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    /// 4-byte elements copied inline, the rest zeroed (`rep stosd`).
    Words,
    /// 8-byte elements copied by `copy` (cdecl), the rest set to
    /// `{value, value}` each where its address is not null.
    Pairs { copy: u32, value: u32 },
}

/// The allocate-and-copy functions `(address, fill)`: `(n, src, m)`
/// (cdecl) -> a new buffer of `n` elements with the first `m` from `src`.
pub const ALLOC_COPIES: [(u32, Fill); 4] = [
    (0x005a_afb0, Fill::Words),
    (0x005b_1320, Fill::Words),
    (
        0x005a_b070,
        Fill::Pairs {
            copy: 0x005a_8840,
            value: 0,
        },
    ),
    (
        0x005c_0720,
        Fill::Pairs {
            copy: 0x005b_c660,
            value: 0xffff_ffff,
        },
    ),
];

/// The [`ALLOC_COPIES`] fill of `f`.
#[must_use]
pub const fn fill_of(f: u32) -> Fill {
    let mut i = 0;
    while i < ALLOC_COPIES.len() {
        if ALLOC_COPIES[i].0 == f {
            return ALLOC_COPIES[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The fog manager's one-float setters `(address, source field)`.
pub const FOG_SETTERS: [(u32, u32); 3] = [
    (0x006e_3760, 0x28),
    (0x006e_3a30, 0x24),
    (0x006e_3b10, 0x20),
];

/// The [`FOG_SETTERS`] field of `f`.
#[must_use]
pub const fn fog_field_of(f: u32) -> u32 {
    let mut i = 0;
    while i < FOG_SETTERS.len() {
        if FOG_SETTERS[i].0 == f {
            return FOG_SETTERS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// A `float` through `fld dword; fstp dword` (a signalling NaN quieted).
fn quiet(bits: u32) -> u32 {
    x87::round_trip(f32::from_bits(bits)).to_bits()
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// Where a `PSharray`'s elements are: the buffer for more than one,
    /// the word itself for one, 0 for none.
    fn sharray_data(h: &mut impl Host, at: u32, count: u32) -> u32 {
        if count > 1 {
            h.read(at + 4)
        } else if count != 0 {
            at + 4
        } else {
            0
        }
    }

    /// [`super::SH_FIELDS`]: field `e.field` of element `i`.
    pub fn sh_field(h: &mut impl Host, this: u32, i: u32, e: ShField) -> u32 {
        let at = this + e.count;
        let count = h.read::<u32>(at) & 0x7fff_ffff;
        if e.bounded && i >= count {
            return 0;
        }
        if let Some(flag) = e.flag
            && h.read::<u8>(this + flag) & 1 == 0
        {
            return 0;
        }
        let data = sharray_data(h, at, count);
        let p = data
            .wrapping_add(i.wrapping_mul(e.stride))
            .wrapping_add(e.field);
        if e.address { p } else { h.read(p) }
    }

    /// [`super::ALLOC_COPIES`]: `(n, src, m)` -> the new buffer or 0.
    pub fn alloc_copy(h: &mut impl Host, n: u32, src: u32, m: u32, fill: Fill) -> u32 {
        if n == 0 {
            return 0;
        }
        let size = match fill {
            Fill::Words => 4,
            Fill::Pairs { .. } => 8,
        };
        let p = cdecl(h, PHYRE_MALLOC, &[n.wrapping_mul(size), 4]);
        if p == 0 {
            return 0;
        }
        match fill {
            Fill::Words => {
                let mut s = src;
                for k in 0..m {
                    let d = p.wrapping_add(4 * k);
                    if d != 0 {
                        let w: u32 = h.read(s);
                        h.write(d, w);
                        s = s.wrapping_add(4);
                    }
                }
                for k in m..n {
                    h.write(p.wrapping_add(4 * k), 0u32);
                }
            }
            Fill::Pairs { copy, value } => {
                cdecl(h, copy, &[p, src, m]);
                for k in m..n {
                    let d = p.wrapping_add(8 * k);
                    if d != 0 {
                        h.write(d, [value, value]);
                    }
                }
            }
        }
        p
    }

    /// `0x005af000(this; n)`: the `PArray<PUInt8 *>` resized to `n`: a new
    /// buffer with the old elements that fit ([`ALLOC_COPY_U8P`]), the old
    /// one freed when owned; `0xd` when the allocation fails, else 0.
    pub fn resize_u8p(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let word: u32 = h.read(this);
        if n == word {
            return 0;
        }
        let count = word & 0x7fff_ffff;
        let keep = if n < count { n } else { count };
        let old: u32 = h.read(this + 4);
        let p = cdecl(h, ALLOC_COPY_U8P, &[n, old, keep]);
        if p == 0 && n != 0 {
            return 0xd;
        }
        let old: u32 = h.read(this + 4);
        if old != p && h.read::<u32>(this) & 0x8000_0000 == 0 {
            cdecl(h, PHYRE_FREE, &[old]);
        }
        h.write(this + 4, p);
        h.write(this, n);
        0
    }

    /// `0x0058c590(this; n, src, m)`: storage for `n` 0x1c-byte elements
    /// (the word at `this + 4` for one, else `PhyreMalloc`), the first `m`
    /// copied from `src`, the rest zeroed; the storage, or 0.
    pub fn alloc_copy_28(h: &mut impl Host, this: u32, n: u32, src: u32, m: u32) -> u32 {
        let p = if n > 1 {
            cdecl(h, PHYRE_MALLOC, &[n.wrapping_mul(28), 4])
        } else {
            this + 4
        };
        if p == 0 {
            return 0;
        }
        let mut s = src;
        for k in 0..m {
            let d = p.wrapping_add(28 * k);
            if d != 0 {
                for w in 0..7 {
                    let v: u32 = h.read(s + 4 * w);
                    h.write(d + 4 * w, v);
                }
                s = s.wrapping_add(28);
            }
        }
        cdecl(
            h,
            ZERO_ARRAY_28,
            &[p.wrapping_add(28 * m), n.wrapping_sub(m)],
        );
        p
    }

    /// `0x006041e0(this; src)`: 16 words from `src` to `this + 0x10`;
    /// `eax` the last.
    pub fn copy_16_words(h: &mut impl Host, this: u32, src: u32) -> u32 {
        let mut last = 0;
        for k in 0..16 {
            last = h.read(src + 4 * k);
            h.write(this + 0x10 + 4 * k, last);
        }
        last
    }

    /// `0x009fc970(this)`: the ten words at `+0x3040` zeroed; `this`.
    pub fn zero_3040(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 0x3040, [0u32; 10]);
        this
    }

    /// `0x005b4440(this; a, b, c)`: `{a, b, c}`, the first seven words of
    /// each of the 256 0x20-byte records from `+0xc`, and the six words
    /// at `+0x230c` zeroed; `this`.
    pub fn init_records(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        h.write(this, [a, b, c]);
        for k in 0..256 {
            h.write(this + 0xc + 0x20 * k, [0u32; 7]);
        }
        h.write(this + 0x230c, [0u32; 6]);
        this
    }

    /// `0x006152a0(thread; slot, b, c, d)`: the command in `*slot` gets
    /// `{c, d}` at `+0` and `b` at `+0x10`, its event (`+8`) is signalled
    /// and the slot cleared; 0, or 5 for an empty slot.
    pub fn hand_over(h: &mut impl Host, slot: u32, b: u32, c: u32, d: u32) -> u32 {
        let cmd: u32 = h.read(slot);
        if cmd == 0 {
            return 5;
        }
        h.write(cmd, c);
        let cmd: u32 = h.read(slot);
        h.write(cmd + 4, d);
        let cmd: u32 = h.read(slot);
        h.write(cmd + 0x10, b);
        let cmd: u32 = h.read(slot);
        this_call(h, EVENT_SIGNAL, cmd + 8, &[]);
        h.write(slot, 0u32);
        0
    }

    /// `0x005b6ca0(this)`: with a pending command slot (`+0xe0`), the
    /// render thread takes it ([`HAND_OVER`] with `+0x44`, `+0x4c`,
    /// `+0x50`); on success those are cleared and both ring buffers
    /// (`+0x1c`, `+0x30`) move to their next segment. Its non-zero
    /// result, else 0.
    pub fn flush_commands(h: &mut impl Host, this: u32) -> u32 {
        let slot: u32 = h.read(this + 0xe0);
        if slot == 0 {
            return 0;
        }
        let [b, c, d] = [0x44, 0x4c, 0x50].map(|at| h.read::<u32>(this + at));
        let thread = cdecl(h, RENDER_THREAD_INSTANCE, &[]);
        let r = this_call(h, HAND_OVER, thread, &[slot, b, c, d]);
        if r != 0 {
            return r;
        }
        h.write(this + 0x44, 0u32);
        h.write(this + 0x4c, 0u32);
        h.write(this + 0x50, 0u32);
        this_call(h, NEXT_SEGMENT, this + 0x1c, &[]);
        this_call(h, NEXT_SEGMENT, this + 0x30, &[]);
        0
    }

    /// `0x00a23be0(this; buffer, n)`: a `PArray`'s external buffer: for
    /// `n >= 0` (signed) the owned old buffer freed when it differs, then
    /// `{n | 0x80000000, buffer}`; 0.
    pub fn set_external(h: &mut impl Host, this: u32, buffer: u32, n: u32) -> u32 {
        if (n as i32) < 0 {
            return 0;
        }
        let old: u32 = h.read(this + 4);
        if old != buffer && h.read::<u32>(this) & 0x8000_0000 == 0 {
            cdecl(h, PHYRE_FREE, &[old]);
        }
        h.write(this + 4, buffer);
        h.write(this, n | 0x8000_0000);
        0
    }

    /// `0x00603dc0(this; buffer, n)`: a `PSharray`'s external buffer: 5
    /// for `n < 0` (signed); one element is copied into the array's own
    /// word (count 1, owned); else `{n | 0x80000000, buffer}`. The old
    /// owned buffer is freed when it is neither the new one nor the
    /// array's word; 0.
    pub fn sharray_set_external(h: &mut impl Host, this: u32, buffer: u32, n: u32) -> u32 {
        if (n as i32) < 0 {
            return 5;
        }
        let count = h.read::<u32>(this) & 0x7fff_ffff;
        let cur = sharray_data(h, this, count);
        let own = this + 4;
        let (data, word) = if n == 1 {
            let v: u32 = h.read(buffer);
            h.write(own, v);
            (own, 1)
        } else {
            (buffer, n | 0x8000_0000)
        };
        if cur != data && cur != own && h.read::<u32>(this) & 0x8000_0000 == 0 {
            cdecl(h, PHYRE_FREE, &[cur]);
        }
        if data != own {
            h.write(own, data);
        }
        h.write(this, word);
        0
    }

    /// `0x00603e50(this; buffer, n)`: the member's own word with one
    /// element goes through the descriptor's validated default buffer
    /// (when the member descriptor's slot 1 returns one), anything else
    /// through [`SHAPES_SET_EXTERNAL`]; 0.
    pub fn shapes_set_buffer(h: &mut impl Host, this: u32, buffer: u32, n: u32) -> u32 {
        let member = h.rebase(SHAPES_MEMBER);
        let desc = h.call_virtual(member, 1, &[]);
        let own = this + 4;
        if buffer == own && n == 1 && desc != 0 {
            this_call(h, APPLY_DEFAULT_BUFFER, desc, &[own, 1]);
        } else {
            this_call(h, SHAPES_SET_EXTERNAL, this, &[buffer, n]);
        }
        0
    }

    /// The utilities' deleting destructors (`0x00a03140`, `0x00a3e800`,
    /// `(this; flags)`): the `PUtility` vtable, the utility unlinked from
    /// the registry list (next at `+4`), `operator delete` when
    /// `flags & 1`; `this`.
    pub fn utility_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(UTILITY_VTABLE);
        h.write(this, vt);
        let mut link = h.rebase(UTILITY_LIST);
        if h.read::<u32>(link) != 0 {
            loop {
                let node: u32 = h.read(link);
                if node == this {
                    break;
                }
                let next: u32 = h.read(node + 4);
                link = node + 4;
                if next == 0 {
                    break;
                }
            }
            if h.read::<u32>(link) != 0 {
                let next: u32 = h.read(this + 4);
                h.write(link, next);
                h.write(this + 4, 0u32);
            }
        }
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `0x00a24c90(this)`: the samplers at `+0xf40` and `+0xf64` released
    /// when bit 7 of their `+0x1c` word is set, the sprite arrays of
    /// `[[this + 0xf38]]` released when set, `+0xf38` cleared.
    pub fn release_post_resources(h: &mut impl Host, this: u32) {
        for at in [0xf40, 0xf64] {
            if (h.read::<u32>(this + at + 0x1c) >> 7) & 1 != 0 {
                this_call(h, SAMPLER_RELEASE, this + at, &[]);
            }
        }
        let p: u32 = h.read(this + 0xf38);
        if p != 0 {
            let arrays: u32 = h.read(p);
            this_call(h, SPRITE_ARRAYS_RELEASE, arrays, &[]);
        }
        h.write(this + 0xf38, 0u32);
    }

    /// The fog manager's parameter copy: the source's (`+0xa0`) colour
    /// `+0x30..0x38` to `+0..8`, `+0x10..0x18` as they are, `+0x28` to
    /// `+0x2c`, `+0x24`, `+0x20`, `+0x2c` to `+0x28` (each through the
    /// x87); `eax` the source.
    fn fog_copy(h: &mut impl Host, fog: u32) -> u32 {
        let src: u32 = h.read(fog + 0xa0);
        for (from, to) in [
            (0x30, 0),
            (0x34, 4),
            (0x38, 8),
            (0x10, 0x10),
            (0x14, 0x14),
            (0x18, 0x18),
            (0x28, 0x2c),
            (0x24, 0x24),
            (0x20, 0x20),
            (0x2c, 0x28),
        ] {
            let v: u32 = h.read(src + from);
            h.write(fog + to, quiet(v));
        }
        src
    }

    /// The fog setters ([`super::FOG_SETTERS`], `(fog; value)`, and
    /// `0x006e36f0`'s `(fog; r, g, b)` to `+0x10..0x18`): unless the fog
    /// is moving (`isMoving`'s `eax` returned), the source's field(s) set
    /// and the parameters copied; `eax` the source.
    pub fn fog_set(h: &mut impl Host, fog: u32, field: u32, values: &[u32]) -> u32 {
        let moving = h.call_sym(FOG_IS_MOVING, Conv::Thiscall(fog), &[]);
        if moving & 0xff != 0 {
            return moving;
        }
        let src: u32 = h.read(fog + 0xa0);
        for (k, &v) in values.iter().enumerate() {
            h.write(src + field + 4 * k as u32, quiet(v));
        }
        fog_copy(h, fog)
    }
}
