//! The function table (docs/host.md, "The function table"): every port
//! under its original address. Unicorn code that reaches a registered
//! address enters the port; a port that calls an address goes through
//! [`crate::exec::Exec::call`], which takes the port when there is one and
//! Unicorn otherwise. Later this is the modding seam: a mod replaces an
//! entry.

use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};

use crate::exec::Exec;

/// The convention, the arguments and the result: `ffx_bridge::route`'s,
/// which the ports' calls into the game use too.
pub use ffx_bridge::route::{Args, Conv, Ret};

/// A port: the original's arguments in, its result out.
#[derive(Debug, Clone, Copy)]
pub enum Port {
    /// With the executor, for calls back into the game (the hand-written
    /// ports of [`crate::ports`]).
    Exec(fn(&mut Exec, &Args) -> Ret),
    /// A `ported!` shim (`ffx-host-ports`): its calls go through
    /// `ffx_bridge::route`, which the executor serves while it runs.
    Routed(fn(&Args) -> Ret),
}

/// One entry.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// The `ffx.sym` name.
    pub name: &'static str,
    /// The convention.
    pub conv: Conv,
    /// Stack words the caller pushes (register arguments not counted).
    pub words: usize,
    /// The port.
    pub port: Port,
}

/// A hasher for game addresses (`u32` keys): one multiply, for the maps
/// every call looks up (m5-perf1; `std`'s `SipHash` and the 15,000-entry
/// `BTreeMap` were a measurable part of each call's dispatch).
#[derive(Debug, Default, Clone, Copy)]
pub struct AddrHasher(u64);

impl Hasher for AddrHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(8) ^ u64::from(b)).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        }
    }
    fn write_u32(&mut self, n: u32) {
        // the high bits carry the mix: hashbrown takes its tag from them
        self.0 = (self.0 ^ u64::from(n)).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    fn write_usize(&mut self, n: usize) {
        self.0 = (self.0 ^ n as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
}

/// A map keyed by game address with [`AddrHasher`].
pub type AddrMap<V> = HashMap<u32, V, BuildHasherDefault<AddrHasher>>;

/// Every port by original address.
#[derive(Debug, Default, Clone)]
pub struct Table {
    entries: BTreeMap<u32, Entry>,
    /// The same entries for [`Table::get`] (every call looks one up).
    index: AddrMap<Entry>,
    /// Ports that wrap their original: address → the original's first
    /// instructions (whole ones, position-independent), which the executor
    /// copies into a trampoline ([`crate::exec::Exec::call_original`]).
    wraps: BTreeMap<u32, &'static [u8]>,
    /// The ports wrappers replaced: address → the port.
    wrapped_ports: BTreeMap<u32, Entry>,
}

impl Table {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `entry` at `addr`, replacing (and returning) an earlier one.
    pub fn register(&mut self, addr: u32, entry: Entry) -> Option<Entry> {
        self.index.insert(addr, entry);
        self.entries.insert(addr, entry)
    }

    /// Registers `entry` at `addr` as a wrapper of the original there,
    /// whose code starts with `prologue` (checked against the image when
    /// the executor is built; the wrapper calls the original through
    /// [`crate::exec::Exec::call_original`]). When a port was registered
    /// there (a `ported!` entry), that port is what the wrapper calls as
    /// its original.
    pub fn wrap(&mut self, addr: u32, entry: Entry, prologue: &'static [u8]) -> Option<Entry> {
        self.wraps.insert(addr, prologue);
        self.index.insert(addr, entry);
        let replaced = self.entries.insert(addr, entry);
        if let Some(e) = replaced {
            self.wrapped_ports.insert(addr, e);
        }
        replaced
    }

    /// The port a wrapper at `addr` replaced ([`Table::wrap`]).
    #[must_use]
    pub fn wrapped_port(&self, addr: u32) -> Option<&Entry> {
        self.wrapped_ports.get(&addr)
    }

    /// The wrapped originals: address and prologue.
    pub fn wrapped(&self) -> impl Iterator<Item = (u32, &'static [u8])> + '_ {
        self.wraps.iter().map(|(&a, &p)| (a, p))
    }

    /// The port at `addr`.
    #[must_use]
    pub fn get(&self, addr: u32) -> Option<&Entry> {
        self.index.get(&addr)
    }

    /// Every address with a port.
    pub fn addresses(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries.keys().copied()
    }

    /// Every entry by address.
    pub fn entries(&self) -> impl Iterator<Item = (u32, Entry)> + '_ {
        self.entries.iter().map(|(&a, &e)| (a, e))
    }

    /// How many ports.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
