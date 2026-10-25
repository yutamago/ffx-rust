//! `PCluster`, `PNamespace` and the `PBinary` cluster headers.
//!
//! Layouts:
//! - `PNamespace` (`0x0043dd80`): three intrusive list heads (`+0`, `+8`,
//!   `+0x14`, each `next, prev` pointing at itself when empty) and a count
//!   at `+0x10`;
//! - `PCluster` (`0x0043f800`): a `PClusterBase`, then a list head at
//!   `+0x48`;
//! - `PClusterHeaderBase` (`0x00468670`): `"RYHP"`, 0x54, zeroes;
//!   `PClusterHeader` (`0x00469980`) adds three zero words at
//!   `+0x48..+0x54`.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `PClusterBase::PClusterBase` (thiscall).
pub const CLUSTER_BASE_CTOR: u32 = 0x0043_f820;
/// `PClusterHeaderBase::PClusterHeaderBase` (thiscall).
pub const HEADER_BASE_CTOR: u32 = 0x0046_8670;

/// Pure parts: the empty intrusive list.
pub mod core {
    /// The two words of an empty list head at `at` (`next`, `prev`).
    #[must_use]
    pub const fn empty_list(at: u32) -> [u32; 2] {
        [at, at]
    }
}

/// The ports over game memory.
pub mod bridge {
    use super::{CLUSTER_BASE_CTOR, Conv, HEADER_BASE_CTOR, Host, Mem, core};

    fn init_list(m: &mut impl Mem, at: u32) {
        m.write(at, core::empty_list(at));
    }

    /// `PNamespace::PNamespace(ns)` (`0x0043dd80`, thiscall): three empty
    /// lists, count 0; returns `ns`.
    pub fn namespace_ctor(m: &mut impl Mem, ns: u32) -> u32 {
        init_list(m, ns);
        init_list(m, ns + 8);
        m.write(ns + 0x10, 0u32);
        init_list(m, ns + 0x14);
        ns
    }

    /// `0x00576630` (thiscall): `PClusterBase::PClusterBase(this)`;
    /// returns `this`.
    pub fn cluster_base_init(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(CLUSTER_BASE_CTOR, Conv::Thiscall(this), &[]);
        this
    }

    /// `PCluster::PCluster(cluster)` (`0x0043f800`, thiscall): the base,
    /// then an empty list at `+0x48`; returns `cluster`.
    pub fn cluster_ctor(h: &mut impl Host, cluster: u32) -> u32 {
        cluster_base_init(h, cluster);
        init_list(h, cluster + 0x48);
        cluster
    }

    /// `PClusterHeader::PClusterHeader(header)` (`0x00469980`,
    /// thiscall): the base header, then three zero words; returns it.
    pub fn header_ctor(h: &mut impl Host, header: u32) -> u32 {
        h.call_sym(HEADER_BASE_CTOR, Conv::Thiscall(header), &[]);
        h.write(header + 0x48, [0u32; 3]);
        header
    }

    /// `0x004787e0` (thiscall `(a, b, c)`): `0x004786a0(this, a, b, 1,
    /// c)`; returns `this`.
    pub fn mapping_init(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        h.call_sym(0x0047_86a0, Conv::Thiscall(this), &[a, b, 1, c]);
        this
    }

    /// `0x00465aa0` (thiscall `(a, b, c)`): a tail call of
    /// `PBinary::loadClusterTextures` (`0x00465ab0`) with the same
    /// arguments; its `eax`.
    pub fn load_cluster_textures(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        h.call_sym(0x0046_5ab0, Conv::Thiscall(this), &[a, b, c])
    }

    /// `0x00593bd0` (thiscall): the low 31 bits of `+0x20`, or 1 for 0.
    pub fn count_at_least_one(m: &impl Mem, this: u32) -> u32 {
        let v: u32 = m.read(this + 0x20);
        let v = v & 0x7fff_ffff;
        if v == 0 { 1 } else { v }
    }

    /// `0x00595fa0` / `0x00595fb0` (thiscall): whether `+0x64` is set.
    pub fn has_word_64(m: &impl Mem, this: u32) -> u32 {
        let v: u32 = m.read(this + 0x64);
        u32::from(v != 0)
    }
}
