//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the PC UI (ffx-pcui) tests. Every
//! state the drivers reached on the base snapshots (`tools/harvest.sh`); each
//! test skips without `FFX_GAME_DIR` or stored states. New lines go at the end
//! of this file.

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, harvest_test};

/// An oracle as the bridges' host: original callees run in the emulator.
struct Run<'a>(&'a mut Oracle);

impl Mem for Run<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

impl Host for Run<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        let c = match c {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
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

// pc round 2: a dynamic geometry's shader by its "GL/" material name
harvest_test!(
    class_dynamic_geometry_set_shader_gl,
    0x006a_48d0,
    args = 2,
    |m: &mut Oracle, s| {
        let work = m.stack_range().start + 0x400;
        ffx_pcui::dyngeo::bridge::set_shader_gl(&mut Run(m), s.ecx(), s.arg(1), work)
    }
);
