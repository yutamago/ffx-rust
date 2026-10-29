//! M5 stage 1, list `phyre-render-b`: the small template shapes of
//! `PRendering`'s and the post-processing classes' code, one generic port
//! per shape with the constants of each copy in the generated table
//! (`m5p_render_b_table.rs.in`, by the copy's address). The shims and the
//! oracle tests read the table from here, so every copy runs against its
//! own original (`crates/ffx-oracle/tests/m5p_render_b.rs`).
//!
//! - [`VT_TAILS`]: a destructor body that stores its class's vtable and
//!   tail-jumps to the parent's destructor (`thiscall`, no stack words),
//!   `Phyre_PClassDescriptor_Destruct` (`0x0043b3e0`) for the
//!   `PClassDescriptorForType<T>` copies; only the exception-unwind
//!   funclets of the static initialisers jump to them. The parent's `eax`.
//! - [`RELEASES`]: a `PArray<T>`'s storage release (`thiscall`): word 0
//!   is the count with bit 31 set for a buffer the array does not own,
//!   word 1 the buffer; the buffer goes to `Phyre_PhyreFree` (or
//!   [`RecordsFree`](Release::RecordsFree)'s record release) when owned,
//!   then both words are zeroed.
//! - [`NOT_SUPPORTED`]: the render interface's state setters D3D11 has no
//!   state for: `eax = 0x17` (`thiscall`, `ret n`).
//! - [`COPIES`]: `PClassDescriptorConcrete<T>` and `PClassDataMemberPhyreArray<T>`
//!   element copies `(dst, src, count)` (cdecl): `count` elements of the
//!   copy's size, each only when its destination is not null; [`ZERO16`]:
//!   `(dst, count)` zeroing 16-byte elements the same way.

include!("m5p_render_b_table.rs.in");

/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre_PDynamicGeometry_freeRecords_005c1ef0(records, n)` (cdecl).
pub const FREE_RECORDS: u32 = 0x005c_1ef0;
/// What the render interface's unsupported state setters return.
pub const E_NOT_SUPPORTED: u32 = 0x17;

/// How a [`RELEASES`] copy releases its buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    /// `PhyreFree(buffer)` when owned, even a null buffer.
    Free,
    /// `PhyreFree(buffer)` when owned and not null.
    FreeNonNull,
    /// Nothing at all for an empty array (word 0 is 0); else as
    /// [`Release::FreeNonNull`].
    SkipEmpty,
    /// `freeRecords(buffer, count)` when owned (the count without bit 31).
    RecordsFree,
}

/// The copy of `f` in a table keyed by address (a compile error when `f` is
/// missing).
#[must_use]
pub const fn vt_tail_of(f: u32) -> (u32, u32) {
    let mut i = 0;
    while i < VT_TAILS.len() {
        if VT_TAILS[i].0 == f {
            return (VT_TAILS[i].1, VT_TAILS[i].2);
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The [`RELEASES`] kind of `f`.
#[must_use]
pub const fn release_of(f: u32) -> Release {
    let mut i = 0;
    while i < RELEASES.len() {
        if RELEASES[i].0 == f {
            return RELEASES[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The [`COPIES`] element size of `f`.
#[must_use]
pub const fn copy_size_of(f: u32) -> u32 {
    let mut i = 0;
    while i < COPIES.len() {
        if COPIES[i].0 == f {
            return COPIES[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The [`NOT_SUPPORTED`] stack words of `f`.
#[must_use]
pub const fn not_supported_words(f: u32) -> u32 {
    let mut i = 0;
    while i < NOT_SUPPORTED.len() {
        if NOT_SUPPORTED[i].0 == f {
            return NOT_SUPPORTED[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The `eax` the release leaves when it frees nothing because the buffer
/// is not owned: `shr eax, 31` then `not al` of word 0.
const NOT_OWNED_EAX: u32 = 0xfe;

pub mod bridge {
    //! The originals over game memory.

    use super::{FREE_RECORDS, NOT_OWNED_EAX, PHYRE_FREE, Release};
    use ffx_graphic::host::{Conv, Host};

    /// [`super::VT_TAILS`]: `[this] = vtable`, then the parent's
    /// destructor `parent(this)`; its `eax`.
    pub fn vt_tail(h: &mut impl Host, this: u32, vtable: u32, parent: u32) -> u32 {
        let vt = h.rebase(vtable);
        h.write(this, vt);
        h.call_sym(parent, Conv::Thiscall(this), &[])
    }

    /// [`super::RELEASES`]: the array at `this` releases its buffer as
    /// `kind` says; the `eax` the original leaves (the callee's, `0xfe`
    /// when the buffer is not owned, 0 or word 0 when there is nothing to
    /// free).
    pub fn release(h: &mut impl Host, this: u32, kind: Release) -> u32 {
        let count: u32 = h.read(this);
        let buffer: u32 = h.read(this + 4);
        let owned = count & 0x8000_0000 == 0;
        let eax = match kind {
            Release::Free if owned => h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer]),
            Release::RecordsFree if owned => {
                h.call_sym(FREE_RECORDS, Conv::Cdecl, &[buffer, count & 0x7fff_ffff])
            }
            Release::Free | Release::RecordsFree => NOT_OWNED_EAX,
            Release::FreeNonNull => {
                if !owned {
                    NOT_OWNED_EAX
                } else if buffer == 0 {
                    0
                } else {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer])
                }
            }
            Release::SkipEmpty => {
                if count == 0 {
                    return 0;
                }
                if buffer == 0 {
                    count
                } else if !owned {
                    NOT_OWNED_EAX
                } else {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer])
                }
            }
        };
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// [`super::COPIES`]: `count` elements of `size` bytes from `src` to
    /// `dst`; a null destination copies nothing and leaves the source
    /// where it is (the original's `if (p) new (p) T(*src++)`). Word by
    /// word, as the original's moves (and `rep movsd`) go, so overlapping
    /// ranges come out alike.
    pub fn copy(h: &mut impl Host, dst: u32, src: u32, count: u32, size: u32) {
        let mut src = src;
        for i in 0..count {
            let d = dst.wrapping_add(i.wrapping_mul(size));
            if d == 0 {
                continue;
            }
            for k in (0..size).step_by(4) {
                let w: u32 = h.read(src.wrapping_add(k));
                h.write(d.wrapping_add(k), w);
            }
            src = src.wrapping_add(size);
        }
    }

    /// [`super::ZERO16`]: `count` 16-byte elements at `dst` zeroed, each
    /// only when its address is not null.
    pub fn zero16(h: &mut impl Host, dst: u32, count: u32) {
        for i in 0..count {
            let d = dst.wrapping_add(i.wrapping_mul(16));
            if d != 0 {
                h.write(d, [0u32; 4]);
            }
        }
    }
}
