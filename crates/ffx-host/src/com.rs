//! COM objects in the arena (docs/host.md, "D3D11 and DXGI").
//!
//! The game holds a COM interface as a pointer to a word that points at a
//! vtable. Here that pointer is a 16-byte block on the arena heap: its
//! first word is the address of the class's vtable, also in the arena,
//! and every vtable slot points at its own `int3` in the COM stub region.
//! The executor runs the slot's trap ([`crate::exec::Trap`]) there, with
//! the method's stack words (`this` first) and its stdcall `ret 4n`. So
//! an object is a `u32` handle (its block's address) whatever the host's
//! pointer width, and nothing of the host's objects lives in the arena.
//!
//! This module knows the vtables ([`crate::com_methods`], generated from
//! `d3d11.h`/`dxgi.h`'s declaration order), the objects and their
//! reference counts, and `QueryInterface`'s interface chains; what the
//! methods do is [`crate::gfx`]'s.

use std::collections::HashMap;

use ffx_bridge::Mem as _;

use crate::Result;
use crate::com_methods::{CLASSES, IIDS, Method};
use crate::exec::{Exec, Trap, TrapFn};
use crate::table::Conv;

/// `S_OK`.
pub const S_OK: u32 = 0;
/// `S_FALSE`.
pub const S_FALSE: u32 = 1;
/// `E_NOTIMPL`.
pub const E_NOTIMPL: u32 = 0x8000_4001;
/// `E_NOINTERFACE`.
pub const E_NOINTERFACE: u32 = 0x8000_4002;
/// `E_POINTER`.
pub const E_POINTER: u32 = 0x8000_4003;
/// `E_FAIL`.
pub const E_FAIL: u32 = 0x8000_4005;
/// `E_INVALIDARG`.
pub const E_INVALIDARG: u32 = 0x8007_0057;
/// `E_OUTOFMEMORY`.
pub const E_OUTOFMEMORY: u32 = 0x8007_000e;
/// `DXGI_ERROR_NOT_FOUND`.
pub const DXGI_ERROR_NOT_FOUND: u32 = 0x887a_0002;

/// Where the stub region goes looking for room.
const REGION_HINT: u32 = 0x0300_0000;
/// The stub region: the vtables, then one `int3` per slot.
const REGION_SIZE: u32 = 0x0001_0000;
/// An object's block: the vtable word, the class, a tag word, spare.
const OBJECT_SIZE: u32 = 16;

/// One live (or released) object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Object {
    /// Its class (an index into [`CLASSES`], [`crate::com_methods::class`]).
    pub class: usize,
    /// The reference count.
    pub refs: u32,
}

/// The vtables and the objects.
#[derive(Debug)]
pub struct Com {
    /// Each class's vtable address.
    vtables: Vec<u32>,
    /// Every object by its handle (the game's interface pointer).
    objects: HashMap<u32, Object>,
}

impl Com {
    /// Maps the stub region, writes every class's vtable and stubs, and
    /// places one trap per slot that runs `run` with [`tag`]`(class,
    /// slot)`.
    ///
    /// # Errors
    /// When the arena has no room.
    pub fn new(e: &Exec, run: TrapFn) -> Result<Self> {
        let arena = e.arena();
        let at = arena.map_anywhere(REGION_HINT, REGION_SIZE, "COM vtables")?;
        let slots: usize = CLASSES.iter().map(|c| c.2.len()).sum();
        let mut stub = at + u32::try_from(slots * 4).unwrap_or(0);
        stub = stub.next_multiple_of(16);
        let mut table = at;
        let mut vtables = Vec::with_capacity(CLASSES.len());
        for (class, &(_, _, methods)) in CLASSES.iter().enumerate() {
            vtables.push(table);
            for (slot, &(_, _, words, _)) in methods.iter().enumerate() {
                arena.fill(stub, &[0xcc]);
                arena.fill(table, &stub.to_le_bytes());
                e.shared().add_trap(
                    stub,
                    Trap {
                        run,
                        tag: tag(class, slot),
                        conv: Conv::Stdcall,
                        words,
                    },
                );
                stub += 1;
                table += 4;
            }
        }
        Ok(Self {
            vtables,
            objects: HashMap::new(),
        })
    }

    /// A new object of `class` with one reference: its handle, or 0 when
    /// the heap is full.
    pub fn create(&mut self, e: &Exec, class: usize) -> u32 {
        let p = e.alloc(OBJECT_SIZE);
        if p == 0 {
            return 0;
        }
        let mut mem = e.mem();
        mem.write(p, self.vtables[class]);
        mem.write(p + 4, u32::try_from(class).unwrap_or(0));
        mem.write(p + 8, 0u32);
        mem.write(p + 12, 0u32);
        self.objects.insert(p, Object { class, refs: 1 });
        p
    }

    /// The object at `handle`.
    #[must_use]
    pub fn get(&self, handle: u32) -> Option<Object> {
        self.objects.get(&handle).copied()
    }

    /// The class of the object at `handle`.
    #[must_use]
    pub fn class_of(&self, handle: u32) -> Option<usize> {
        self.objects.get(&handle).map(|o| o.class)
    }

    /// `AddRef`: the new count (0 for an unknown handle).
    pub fn add_ref(&mut self, handle: u32) -> u32 {
        self.objects.get_mut(&handle).map_or(0, |o| {
            o.refs += 1;
            o.refs
        })
    }

    /// `Release`: the new count. An object at 0 stays known (its block is
    /// not reused, so a stale pointer still names it in the log).
    pub fn release(&mut self, handle: u32) -> u32 {
        self.objects.get_mut(&handle).map_or(0, |o| {
            o.refs = o.refs.saturating_sub(1);
            o.refs
        })
    }

    /// How many objects are alive, per class name.
    #[must_use]
    pub fn census(&self) -> Vec<(&'static str, usize)> {
        let mut n = vec![0usize; CLASSES.len()];
        for o in self.objects.values().filter(|o| o.refs > 0) {
            n[o.class] += 1;
        }
        CLASSES
            .iter()
            .zip(n)
            .filter(|(_, n)| *n > 0)
            .map(|(c, n)| (c.1, n))
            .collect()
    }
}

/// A trap tag: the class and the vtable slot.
#[must_use]
pub fn tag(class: usize, slot: usize) -> u32 {
    u32::try_from((class << 16) | slot).unwrap_or(u32::MAX)
}

/// The class and the method behind a trap tag.
#[must_use]
pub fn method(tag: u32) -> Option<(usize, usize, &'static Method)> {
    let class = (tag >> 16) as usize;
    let slot = (tag & 0xffff) as usize;
    CLASSES
        .get(class)
        .and_then(|c| c.2.get(slot))
        .map(|m| (class, slot, m))
}

/// Whether objects of `class` answer `QueryInterface` for `iface`: the
/// interfaces their vtable declares (its chain, `IUnknown` up).
#[must_use]
pub fn implements(class: usize, iface: &str) -> bool {
    CLASSES
        .get(class)
        .is_some_and(|c| c.2.iter().any(|m| m.0 == iface))
}

/// A `GUID` from the arena (`data1`, `data2`, `data3` little-endian, then
/// the 8 bytes of `data4`), as the `u128` [`IIDS`] holds.
#[must_use]
pub fn read_guid(mem: &impl ffx_bridge::Mem, at: u32) -> u128 {
    let mut b = [0u8; 16];
    mem.read_bytes(at, &mut b);
    let d1 = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    let d2 = u16::from_le_bytes([b[4], b[5]]);
    let d3 = u16::from_le_bytes([b[6], b[7]]);
    let d4 = u64::from_be_bytes([b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]]);
    (u128::from(d1) << 96) | (u128::from(d2) << 80) | (u128::from(d3) << 64) | u128::from(d4)
}

/// The interface an IID names, when the host knows it.
#[must_use]
pub fn iid_name(iid: u128) -> Option<&'static str> {
    IIDS.iter().find(|(_, v)| *v == iid).map(|(n, _)| *n)
}

/// An IID as Windows writes it (`{xxxxxxxx-xxxx-...}`), for the log.
#[must_use]
pub fn iid_text(iid: u128) -> String {
    format!(
        "{{{:08x}-{:04x}-{:04x}-{:04x}-{:012x}}}",
        iid >> 96,
        (iid >> 80) & 0xffff,
        (iid >> 64) & 0xffff,
        (iid >> 48) & 0xffff,
        iid & 0xffff_ffff_ffff
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::com_methods::{CONTEXT, DEVICE, SWAP_CHAIN, class};

    #[test]
    fn the_slots_the_hook_knows_are_where_d3d11_h_puts_them() {
        // the graphics layer's slot numbers (ffx-hook gfx/vt.rs)
        assert_eq!(DEVICE[3].1, "CreateBuffer");
        assert_eq!(DEVICE[5].1, "CreateTexture2D");
        assert_eq!(DEVICE[11].1, "CreateInputLayout");
        assert_eq!(DEVICE[23].1, "CreateSamplerState");
        assert_eq!(DEVICE[40].1, "GetImmediateContext");
        assert_eq!(CONTEXT[12].1, "DrawIndexed");
        assert_eq!(CONTEXT[14].1, "Map");
        assert_eq!(CONTEXT[33].1, "OMSetRenderTargets");
        assert_eq!(CONTEXT[50].1, "ClearRenderTargetView");
        assert_eq!(CONTEXT[69].1, "CSSetShader");
        assert_eq!(CONTEXT[115].1, "CopySubresourceRegion1");
        assert_eq!(CONTEXT[123].1, "PSSetConstantBuffers1");
        assert_eq!(SWAP_CHAIN[8].1, "Present");
        assert_eq!(SWAP_CHAIN[9].1, "GetBuffer");
        // stack words with `this`: ClearDepthStencilView(view, flags,
        // depth, stencil)
        assert_eq!(
            CONTEXT[53],
            (
                "ID3D11DeviceContext",
                "ClearDepthStencilView",
                5,
                crate::com_methods::Returns::Void
            )
        );
    }

    #[test]
    fn query_interface_follows_the_declared_chain() {
        assert!(implements(class::DEVICE, "ID3D11Device"));
        assert!(implements(class::DEVICE, "ID3D11Device1"));
        assert!(!implements(class::DEVICE, "IDXGIDevice"));
        assert!(implements(class::TEXTURE2D, "ID3D11Resource"));
        assert!(implements(class::SWAP_CHAIN, "IDXGIDeviceSubObject"));
    }

    #[test]
    fn guids_read_as_windows_lays_them_out() {
        // IID_ID3D11Texture2D as FFX.exe stores it at 0x00b31470
        let b: [u8; 16] = [
            0xf2, 0xaa, 0x15, 0x6f, 0x08, 0xd2, 0x89, 0x4e, 0x9a, 0xb4, 0x48, 0x95, 0x35, 0xd3,
            0x4f, 0x9c,
        ];
        let mut m = ffx_bridge::VecMem::new(0, 0x1000);
        ffx_bridge::Mem::write_bytes(&mut m, 0x100, &b);
        let g = read_guid(&m, 0x100);
        assert_eq!(iid_name(g), Some("ID3D11Texture2D"));
        assert_eq!(iid_text(g), "{6f15aaf2-d208-4e89-9ab4-489535d34f9c}");
    }
}
