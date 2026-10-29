//! M5 stage 1, list `stubs`: the 179 `PhyreEngine` functions under five
//! bytes (`docs/porting.md`, "Functions under five bytes"). They are too
//! short for the 32-bit hook's `jmp`, so the hooked game keeps running
//! their bytes; `ffx-host`'s function table gets one entry per copy
//! (`host_only!` in `crates/ffx-hook/src/ported/m5p_stubs.rs`).
//!
//! The linker folded most of them (one body can be slot 10 of many
//! `PClassDataMemberPhyreArray<T>` vtables, or `asNamedSemanticDescriptor`
//! of every descriptor), and the rest are one-instruction members. Nine
//! shapes cover all of them, each a generic port in [`bridge`]; the
//! generated [`STUBS`] table (`m5p_stubs_table.rs.in`) gives each copy its
//! shape, its field offset, whether it is a method (`ecx` = `this`) and
//! the stack words its `ret n` pops. The shims check their row at compile
//! time ([`listed`], [`offset`]); the oracle test runs every row against
//! its own bytes.
//!
//! Results in `al` only ([`Kind::False`], [`Kind::True`], [`Kind::Byte`]):
//! the original leaves the upper 24 bits of `eax` as its caller had them
//! (the function pointer it called through, as a rule). The bridge returns
//! the byte, as every `u8` port does (`crate`'s note), and the callers
//! test `al` only.

use ffx_bridge::Mem;

/// What a copy does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    /// `xor eax, eax; ret`: `eax = 0`.
    Zero,
    /// `xor al, al; ret`: `al = 0`.
    False,
    /// `mov al, 1; ret`: `al = 1`.
    True,
    /// `ret` / `ret n`: nothing (every register the caller's).
    Nothing,
    /// `mov eax, ecx; ret`: `eax = this`.
    This,
    /// `mov eax, [ecx + offset]; ret`: a word field.
    Word,
    /// `lea eax, [ecx + offset]; ret`: a field's address.
    Addr,
    /// `mov al, [ecx + offset]; ret`: a byte field.
    Byte,
    /// `fld dword [ecx + offset]; ret`: a float field in `ST0`.
    Float,
}

/// One copy.
#[derive(Debug, Clone, Copy)]
pub struct Stub {
    /// Its address in `FFX.exe` (rebased before use).
    pub addr: u32,
    /// What it does.
    pub kind: Kind,
    /// The field's offset ([`Kind::Word`], [`Kind::Addr`], [`Kind::Byte`],
    /// [`Kind::Float`]; 0 otherwise).
    pub offset: u32,
    /// A method: thiscall on `ecx` (a vtable slot, a `ret n` member, or a
    /// body that reads `ecx`). The others are cdecl with no arguments:
    /// they read neither `ecx` nor the stack, so the two conventions are
    /// the same to their callers.
    pub this: bool,
    /// Stack words the `ret n` pops (0 for a bare `ret`).
    pub words: u32,
}

include!("m5p_stubs_table.rs.in");

/// Copy `addr`'s row (compile time: a missing copy is a build error).
#[must_use]
pub const fn stub_of(addr: u32) -> Stub {
    let mut i = 0;
    while i < STUBS.len() {
        if STUBS[i].addr == addr {
            return STUBS[i];
        }
        i += 1;
    }
    panic!("not in the stubs table");
}

/// Checks that copy `addr` has shape `kind` (compile time, for the shims).
pub const fn listed(addr: u32, kind: Kind) {
    assert!(
        stub_of(addr).kind as u8 == kind as u8,
        "the copy has another shape"
    );
}

/// Copy `addr`'s field offset, checking its shape (compile time).
#[must_use]
pub const fn offset(addr: u32, kind: Kind) -> u32 {
    listed(addr, kind);
    stub_of(addr).offset
}

/// The generic ports, one per shape.
pub mod bridge {
    use super::{Kind, Mem, Stub};

    /// [`Kind::Zero`]: `eax = 0`.
    #[must_use]
    pub const fn zero() -> u32 {
        0
    }

    /// [`Kind::False`]: `al = 0`.
    #[must_use]
    pub const fn no() -> u8 {
        0
    }

    /// [`Kind::True`]: `al = 1`.
    #[must_use]
    pub const fn yes() -> u8 {
        1
    }

    /// [`Kind::Nothing`]: nothing at all.
    pub const fn nothing() {}

    /// [`Kind::This`]: `eax = this`.
    #[must_use]
    pub const fn this(this: u32) -> u32 {
        this
    }

    /// [`Kind::Word`]: `eax = [this + offset]`.
    #[must_use]
    pub fn word(m: &impl Mem, this: u32, offset: u32) -> u32 {
        m.read(this.wrapping_add(offset))
    }

    /// [`Kind::Addr`]: `eax = this + offset` (wrapping, as `lea`).
    #[must_use]
    pub const fn addr(this: u32, offset: u32) -> u32 {
        this.wrapping_add(offset)
    }

    /// [`Kind::Byte`]: `al = [this + offset]`.
    #[must_use]
    pub fn byte(m: &impl Mem, this: u32, offset: u32) -> u8 {
        m.read(this.wrapping_add(offset))
    }

    /// [`Kind::Float`]: `ST0 = [this + offset]` (the `fld` widens the f32
    /// exactly; signalling NaNs quieted, as the FPU does).
    #[must_use]
    pub fn float(m: &impl Mem, this: u32, offset: u32) -> f32 {
        m.read(this.wrapping_add(offset))
    }

    /// What a copy returns, for callers that hold a [`Stub`] row (the
    /// tests run every row through this).
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Out {
        /// Nothing returned.
        Void,
        /// All of `eax`.
        Eax(u32),
        /// `al` only.
        Al(u8),
        /// `ST0`.
        St0(f32),
    }

    /// Copy `s` on `this` (its stack words are never read).
    #[must_use]
    pub fn run(m: &impl Mem, s: &Stub, this: u32) -> Out {
        match s.kind {
            Kind::Zero => Out::Eax(zero()),
            Kind::False => Out::Al(no()),
            Kind::True => Out::Al(yes()),
            Kind::Nothing => {
                nothing();
                Out::Void
            }
            Kind::This => Out::Eax(self::this(this)),
            Kind::Word => Out::Eax(word(m, this, s.offset)),
            Kind::Addr => Out::Eax(addr(this, s.offset)),
            Kind::Byte => Out::Al(byte(m, this, s.offset)),
            Kind::Float => Out::St0(float(m, this, s.offset)),
        }
    }
}
