//! Compares two captured D3D11 call streams of a frame, draw by draw (the M4
//! gate, `docs/drawdiff.md`).
//!
//! A capture (`capture.ffxgfx`, `ffx-hook` `gfx::capture`) is read into
//! [`capture::Capture`]: per frame, one [`Item`] per draw, clear and copy,
//! every object reference replaced by a stable content key ([`key`]),
//! states decoded field by field ([`state`]) and constant buffers decoded
//! by the shader's own reflection, used variables only ([`cb`]).
//! [`diff::compare`] aligns the two item lists (Myers, [`align`]) and
//! compares the pairs field by field, giving [`Difference`]s of one of the
//! [`Class`]es. Floats compare by their bits; a ULP tolerance and a noise
//! allow list come from files with a written cause ([`rules`]).

pub mod align;
pub mod capture;
pub mod cb;
pub mod diff;
pub mod key;
pub mod report;
pub mod rules;
pub mod state;

use std::fmt;

pub use key::Key;

/// What kind of difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// In A, not in B.
    Missing,
    /// In B, not in A.
    Extra,
    /// In both, at another place in the order.
    Moved,
    /// Draw parameters, topology, blend, depth, raster and sampler state,
    /// viewport, scissor, clear values.
    State,
    /// A used shader constant.
    Constant,
    /// A bound object: buffer, texture, target, layout.
    Resource,
}

impl Class {
    pub const ALL: [Self; 6] = [
        Self::Missing,
        Self::Extra,
        Self::Moved,
        Self::State,
        Self::Constant,
        Self::Resource,
    ];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Extra => "extra",
            Self::Moved => "moved",
            Self::State => "state",
            Self::Constant => "constant",
            Self::Resource => "resource",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.name() == s)
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A compared value. Floats are kept as their bits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    U(u32),
    I(i32),
    /// An `f32`, by its bits.
    F(u32),
    /// A content key, with a description for the report.
    Key(Key, String),
    /// Not there (a constant outside the bound window, a slot left empty).
    Absent,
}

impl Value {
    /// Whether two values are the same: keys by key alone, floats by bits.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Key(a, _), Self::Key(b, _)) => a == b,
            _ => self == other,
        }
    }

    pub fn hash_into(&self, h: &mut key::Fnv) {
        match self {
            Self::U(v) => h.u32(1).u32(*v),
            Self::I(v) => h.u32(2).u32(v.cast_unsigned()),
            Self::F(v) => h.u32(3).u32(*v),
            Self::Key(k, _) => h.u32(4).u64(*k),
            Self::Absent => h.u32(5),
        };
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::U(v) if *v < 16 => write!(f, "{v}"),
            Self::U(v) => write!(f, "{v:#x}"),
            Self::I(v) => write!(f, "{v}"),
            Self::F(v) => write!(f, "{v:#010x} ({:?})", f32::from_bits(*v)),
            Self::Key(k, label) if label.is_empty() => write!(f, "{k:016x}"),
            Self::Key(_, label) => f.write_str(label),
            Self::Absent => f.write_str("absent"),
        }
    }
}

/// The distance between two `f32`s in units in the last place: `None`
/// when either is a NaN. `-0` and `+0` are 1 apart (they differ by bits).
#[must_use]
pub fn ulps(a: u32, b: u32) -> Option<u64> {
    let nan = |v: u32| v & 0x7f80_0000 == 0x7f80_0000 && v & 0x007f_ffff != 0;
    if nan(a) || nan(b) {
        return None;
    }
    let ord = |v: u32| -> i64 {
        if v & 0x8000_0000 == 0 {
            i64::from(v)
        } else {
            -i64::from(v & 0x7fff_ffff) - 1
        }
    };
    Some(ord(a).abs_diff(ord(b)))
}

/// What an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Draw,
    ClearTarget,
    ClearDepth,
    Copy,
}

impl ItemKind {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Draw => "draw",
            Self::ClearTarget => "clear",
            Self::ClearDepth => "clear-depth",
            Self::Copy => "copy",
        }
    }
}

/// A named field of an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub class: Class,
    pub value: Value,
}

/// One draw, clear or copy of a frame, with every reference keyed.
#[derive(Debug, Clone)]
pub struct Item {
    /// Its place in the frame's command list.
    pub cmd: usize,
    pub kind: ItemKind,
    /// Shader keys (0 for clears and copies).
    pub vs: Key,
    pub ps: Key,
    /// Drawn into the scene targets (two render targets and a depth
    /// target), or a clear or copy of one of them.
    pub scene: bool,
    /// What alignment pairs on: shaders, vertex and index keys, start,
    /// count (a draw); the target (a clear or copy).
    pub match_key: Key,
    /// Everything compared: two items with the same fingerprint show no
    /// difference.
    pub fingerprint: Key,
    /// State and resource fields, sorted by name.
    pub fields: Vec<Field>,
    /// Used constants, per stage and slot.
    pub constants: Vec<cb::Constants>,
    /// A one-line description for the report.
    pub label: String,
}

/// One difference between the two captures.
#[derive(Debug, Clone)]
pub struct Difference {
    pub class: Class,
    /// Item indices in A and B (frame-local).
    pub a: Option<usize>,
    pub b: Option<usize>,
    /// The field (`blend.rt0.src`, `ps.cb2.MaterialColour[1].y`, ...);
    /// empty for missing, extra and moved.
    pub field: String,
    /// The constant's variable name, for constants.
    pub var: Option<String>,
    pub va: Value,
    pub vb: Value,
    /// The item's kind and shaders (A's, or B's for an extra item).
    pub kind: ItemKind,
    pub vs: Key,
    pub ps: Key,
    /// The allow-list entry that covers it (index into the rules).
    pub allowed: Option<usize>,
}
