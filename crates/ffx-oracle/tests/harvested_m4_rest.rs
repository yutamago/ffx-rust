//! `m4-rest`'s ports against the original on harvested call states
//! (`ffx_oracle::harvest`, docs/hooking.md "Harvested call states"):
//! `renderMeshInstanceList` as `graphicRender` (`0x00642bb0`) calls it on
//! field.snap and battle.snap (`target/release/examples/harvest --driver
//! 0x00642bb0 --frames 2 --targets 0x005b9a90,0x005b3df0`, p6-render-c's
//! harvest). Each test skips without `FFX_GAME_DIR` or stored states.
//!
//! Frames: the port gets the original's frame at its address (`ebp` is the
//! state's `ESP - 4`); the port's own calls run below it.

mod common;

use ffx_oracle::{Oracle, harvest_test};
use ffx_prender::m4r_meshlist as ml;

/// An oracle as the bridges' host: original callees run in the emulator.
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
        ffx_oracle::check_width(
            "Host::call",
            addr,
            args.len(),
            ffx_graphic::host::max_words(c, false),
        );
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
        self.0.stack_range().start + 0x400
    }
}

harvest_test!(
    harvested_render_mesh_instance_list,
    0x005b_3df0,
    args = 3,
    |m: &mut Oracle, s| {
        let ebp = s.regs.esp - 4;
        // the frame, the three saved registers, then the port's calls
        m.set_call_top(Some(ebp - ml::FRAME - 0x200));
        let r = ml::bridge::render_list(&mut Run(m), s.ecx(), s.arg(0), s.arg(1), s.arg(2), ebp);
        m.set_call_top(None);
        r
    }
);
