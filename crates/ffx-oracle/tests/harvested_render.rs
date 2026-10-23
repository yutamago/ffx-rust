//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the render (ffx-graphic) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use ffx_oracle::{Oracle, harvest_test};

/// An oracle as the render bridges' host (`ffx_graphic::host::Host`):
/// original callees run in the emulator.
struct Run<'a>(&'a mut Oracle);

impl ffx_bridge::Mem for Run<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        ffx_bridge::Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

impl ffx_graphic::host::Host for Run<'_> {
    fn call(&mut self, addr: u32, c: ffx_graphic::host::Conv, args: &[u32]) -> u32 {
        use ffx_graphic::host::Conv as H;
        use ffx_oracle::Conv as O;
        let c = match c {
            H::Cdecl => O::Cdecl,
            H::Stdcall => O::Stdcall,
            H::Thiscall(t) => O::Thiscall(t),
            H::Fastcall => O::Fastcall,
        };
        self.0
            .call(addr, c, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }

    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

// render (ffx-graphic): a UI element's texture remap
harvest_test!(
    graphic_ui_element_remap,
    0x0064_4320,
    args = 2,
    |m: &mut Oracle, s| {
        ffx_graphic::ui::bridge::element_remap(m, s.arg(0), s.arg(1));
    }
);

// pc round 2: an object's meshes turned (the Phyre matrix helpers run as
// they are), and its cdecl entry
harvest_test!(
    phyre_scene_set_object_rotation,
    0x0066_db00,
    args = 3,
    |m: &mut Oracle, s| {
        let work = m.stack_range().start + 0x400;
        ffx_graphic::state::bridge::scene_set_object_rotation(
            &mut Run(m),
            s.ecx(),
            s.arg(0),
            s.arg(1),
            s.arg(2),
            work,
        )
    }
);
harvest_test!(
    graphic_object_set_rotation,
    0x0064_26a0,
    args = 3,
    |m: &mut Oracle, s| {
        ffx_graphic::state::bridge::graphic_object_set_rotation(
            &mut Run(m),
            s.arg(0),
            s.arg(1),
            s.arg(2),
        )
    }
);
