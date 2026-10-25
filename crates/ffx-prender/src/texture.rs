//! `PhyreEngine`'s texture and sampler base classes, as far as the render
//! code reaches them (M4 port round 1, h1-render-a).
//!
//! `PTextureCommonBase` (thiscall methods on the texture):
//! - the constructor `0x004c1d70`: `+0x00` its vtable (`0x00ca34cc`),
//!   `+0x04` a `u16` 0 (`+4` a flag byte the destructor reads), `+0x08`,
//!   `+0x0c` (the base mip level), `+0x10` (the max level) 0, `+0x14` 2
//!   (flags: bit 1 = the max level follows the size), `+0x18` 0 (the
//!   texture's data, an object with `+0` its owner and `+4` a kind byte);
//!   `PTexture2DBase` (`0x004c2f90`) adds `+0x1c`, `+0x20` (width,
//!   height), `PTexture3DBase` (`0x004c47a0`) `+0x24` (depth),
//!   `PTextureCubeMapBase` (`0x004c6070`) `+0x1c`;
//! - the destructor `0x004c1df0`: when the `+4` byte is set,
//!   `PhyrePrintf(3, <message at 0x00b1de58>)`; then the data object's
//!   release (`0x004c29b0`, thiscall on it) when there is one. The three
//!   derived destructors are `jmp`s to it;
//! - `GetMipCountForTextureSize(w, h, d)` `0x004c2430` (cdecl): the
//!   number of halvings of the largest size down to 1;
//! - `PTexture2DBase::updateMaxLevel` `0x004c4250`: the max level from the
//!   size (flag bit 1) or the base level, stored at `+0x10` when it
//!   changes; returns it;
//! - `0x004c28f0(data)`: sets the data object when there is none, or when
//!   the old one is owned by this texture (kind byte 0, owner `+0`);
//!   else 5.
//!
//! `PSamplerStateBase` (`+0x00` min filter, `+0x01` mag filter, `+0x02`
//! .. `+0x04` wrap S, T, R, `+0x08`, `+0x0c` floats, `+0x1c` flags: bit 7
//! = locked): the constructor `0x004be870` and the script setters
//! `setMinFilter` / `setWrapS` / `setWrapT` / `setWrapR`, which return
//! 0x16 on a locked state and else store their byte and return 0.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `PTextureCommonBase`'s vtable.
pub const VTABLE: u32 = 0x00ca_34cc;
/// `Phyre::PhyrePrintf` (cdecl `(level, format, ...)`).
pub const PRINTF: u32 = 0x0043_52f0;
/// The destructor's message.
pub const MESSAGE: u32 = 0x00b1_de58;
/// The data object's release (thiscall on the data object).
pub const RELEASE_DATA: u32 = 0x004c_29b0;

/// `PSamplerStateBase`'s byte setters: their field offsets.
pub const SET_MIN_FILTER: u32 = 0;
/// See [`SET_MIN_FILTER`].
pub const SET_WRAP_S: u32 = 2;
/// See [`SET_MIN_FILTER`].
pub const SET_WRAP_T: u32 = 3;
/// See [`SET_MIN_FILTER`].
pub const SET_WRAP_R: u32 = 4;

/// `GetMipCountForTextureSize(w, h, d)`: how often the largest of the
/// three (unsigned) halves before it is 1 or less.
#[must_use]
pub const fn mip_count(w: u32, h: u32, d: u32) -> u32 {
    let mut m = if w > h { w } else { h };
    if d > m {
        m = d;
    }
    let mut n = 0;
    while m > 1 {
        m >>= 1;
        n += 1;
    }
    n
}

pub mod bridge {
    //! The originals over game memory.

    use super::{Conv, Host, MESSAGE, Mem, PRINTF, RELEASE_DATA, VTABLE, mip_count};

    /// `PTextureCommonBase::PTextureCommonBase(this)` -> `this`.
    pub fn common_ctor(m: &mut impl Mem, this: u32) -> u32 {
        let vtable = m.rebase(VTABLE);
        m.write(this, vtable);
        m.write(this + 4, 0u16);
        m.write(this + 8, 0u32);
        m.write(this + 0xc, 0u32);
        m.write(this + 0x10, 0u32);
        m.write(this + 0x14, 2u32);
        m.write(this + 0x18, 0u32);
        this
    }

    /// `PTexture2DBase::PTexture2DBase(this)` -> `this`.
    pub fn ctor_2d(m: &mut impl Mem, this: u32) -> u32 {
        common_ctor(m, this);
        m.write(this + 0x1c, 0u32);
        m.write(this + 0x20, 0u32);
        this
    }

    /// `PTexture3DBase::PTexture3DBase(this)` -> `this`.
    pub fn ctor_3d(m: &mut impl Mem, this: u32) -> u32 {
        common_ctor(m, this);
        m.write(this + 0x1c, 0u32);
        m.write(this + 0x20, 0u32);
        m.write(this + 0x24, 0u32);
        this
    }

    /// `PTextureCubeMapBase::PTextureCubeMapBase(this)` -> `this`.
    pub fn ctor_cube(m: &mut impl Mem, this: u32) -> u32 {
        common_ctor(m, this);
        m.write(this + 0x1c, 0u32);
        this
    }

    /// `PTextureCommonBase`'s destructor (`0x004c1df0`): the message when
    /// the `+4` byte is set, then the data object's release.
    pub fn common_dtor(h: &mut impl Host, this: u32) {
        if h.read::<u8>(this + 4) != 0 {
            let message = h.rebase(MESSAGE);
            h.call_sym(PRINTF, Conv::Cdecl, &[3, message]);
        }
        let data: u32 = h.read(this + 0x18);
        if data != 0 {
            h.call_sym(RELEASE_DATA, Conv::Thiscall(data), &[]);
        }
    }

    /// The texture data object's destructor (`0x004c79f0`, a `jmp`): its
    /// release on `data`; that `eax`.
    pub fn release_data(h: &mut impl Host, data: u32) -> u32 {
        h.call_sym(RELEASE_DATA, Conv::Thiscall(data), &[])
    }

    /// `PTexture2DBase::updateMaxLevel(this)`: the level count from the
    /// size when flag bit 1 is set, else the base level; stored at
    /// `+0x10` when it differs; returned.
    pub fn update_max_level(m: &mut impl Mem, this: u32) -> u32 {
        let level = if m.read::<u32>(this + 0x14) >> 1 & 1 != 0 {
            mip_count(m.read(this + 0x1c), m.read(this + 0x20), 0)
        } else {
            m.read(this + 0xc)
        };
        if level != m.read::<u32>(this + 0x10) {
            m.write(this + 0x10, level);
        }
        level
    }

    /// `0x004c28f0(this, data)`: `data` becomes the texture's data object
    /// when it has none or the old one is this texture's own (kind byte
    /// `+4` 0 and owner `+0` this): 0. Else 5.
    pub fn set_data(m: &mut impl Mem, this: u32, data: u32) -> u32 {
        let old: u32 = m.read(this + 0x18);
        if old != 0 {
            let owner = if m.read::<u8>(old + 4) == 0 {
                m.read(old)
            } else {
                0
            };
            if owner != this {
                return 5;
            }
        }
        m.write(this + 0x18, data);
        0
    }

    /// `PSamplerStateBase::PSamplerStateBase(this)` -> `this`: filters
    /// 5, 1, wraps 1, 1, 1 (`0x01010105` and a byte), `+8` 0.0, `+0xc`
    /// 1.0, `+0x18` 0x400, the rest 0.
    pub fn sampler_ctor(m: &mut impl Mem, this: u32) -> u32 {
        m.write(this, 0x0101_0105u32);
        m.write(this + 4, 1u8);
        m.write(this + 8, 0.0f32);
        m.write(this + 0x10, 0u32);
        m.write(this + 0xc, 1.0f32);
        m.write(this + 0x14, 0u32);
        m.write(this + 0x18, 0x400u32);
        m.write(this + 0x1c, 0u32);
        this
    }

    /// A `PSamplerStateBase` byte setter: 0x16 when the state is locked
    /// (flags `+0x1c` bit 7), else the byte at `off` and 0.
    pub fn sampler_set(m: &mut impl Mem, this: u32, off: u32, value: u8) -> u32 {
        if m.read::<u32>(this + 0x1c) >> 7 & 1 != 0 {
            return 0x16;
        }
        m.write(this + off, value);
        0
    }
}
