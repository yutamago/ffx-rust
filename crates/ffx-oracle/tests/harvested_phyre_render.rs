//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the map render chain
//! (`ffx_prender`). The states come from `tools/harvest-render.sh`
//! (`graphicRender` on field.snap and battle.snap: every world of the
//! frame, the map's and the characters' segments included); each test
//! skips without `FFX_GAME_DIR` or stored states. Shared helpers:
//! `tests/common/mod.rs`.
//!
//! Frames: the ports' own scratch (matrices, the sort's entry) sits at the
//! bottom of the oracle's stack, out of the caller's stack the comparison
//! reads. `renderWorldCulledSorted` hands its frame to callees that keep
//! pointers to it (the jobs), so its port gets the frame at the original's
//! address (`ebp - 0x9c` from the state's `ESP`; the harvest runs the
//! port's calls 4 KiB below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use ffx_oracle::harvest::Masked;
use ffx_oracle::{Oracle, harvest_test};
use ffx_prender::{mesh, sort, visibility, world};

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
        scratch(self.0)
    }
}

/// The ports' own scratch: the bottom of the oracle's stack.
fn scratch(m: &Oracle) -> u32 {
    m.stack_range().start + 0x400
}

harvest_test!(
    harvested_find_technique_by_pass,
    0x004e_71d0,
    args = 1,
    |m: &mut Oracle, s| mesh::bridge::find_technique(&mut Run(m), s.ecx(), s.arg(0))
);

harvest_test!(
    harvested_select_technique_pass,
    0x005b_a690,
    args = 8,
    |m: &mut Oracle, s| {
        mesh::bridge::select_technique_pass(
            &mut Run(m),
            s.ecx(),
            s.arg(0),
            s.arg(1),
            s.arg(2),
            s.arg(3),
            s.arg(4),
            s.arg(5),
            s.arg(6),
            s.arg(7),
        )
    }
);

harvest_test!(
    harvested_allocate_and_fill_parameter_buffers,
    0x005b_5a50,
    args = 8,
    |m: &mut Oracle, s| {
        mesh::bridge::allocate_and_fill(
            &mut Run(m),
            s.ecx(),
            s.arg(0),
            s.arg(1),
            s.arg(2),
            s.arg(3),
            s.arg(4),
            s.arg(5),
            s.arg(6),
            s.arg(7),
        )
    }
);

harvest_test!(
    harvested_compare_entries,
    0x009f_2d70,
    args = 2,
    |m: &mut Oracle, s| Masked(
        u32::from(sort::bridge::compare_entries(m, s.arg(0), s.arg(1))),
        0xff
    )
);

harvest_test!(
    harvested_sort_visible_entries,
    0x009f_17c0,
    args = 4,
    |m: &mut Oracle, s| {
        let tmp = scratch(m);
        sort::bridge::sort(
            &mut Run(m),
            tmp,
            s.arg(0),
            s.arg(1),
            s.arg(2) as i32,
            s.arg(3),
        );
    }
);

harvest_test!(
    harvested_insertion_sort_visible_entries,
    0x009f_0d20,
    args = 4,
    |m: &mut Oracle, s| {
        let tmp = scratch(m);
        sort::bridge::insertion_sort(&mut Run(m), tmp, s.arg(0), s.arg(1), s.arg(2));
    }
);

harvest_test!(
    harvested_visibility_check_init,
    0x009f_2580,
    args = 2,
    |m: &mut Oracle, s| {
        let frame = scratch(m);
        visibility::bridge::init(&mut Run(m), frame, s.ecx(), s.arg(0), s.arg(1) as i32)
    }
);

harvest_test!(
    harvested_culling_job,
    0x009f_3160,
    args = 3,
    |m: &mut Oracle, s| {
        let frame = scratch(m);
        visibility::bridge::execute(&mut Run(m), frame, s.ecx());
    }
);

harvest_test!(
    harvested_merge_results,
    0x009f_3660,
    args = 2,
    |m: &mut Oracle, s| world::bridge::merge(&mut Run(m), s.ecx(), s.arg(0), s.arg(1))
);

harvest_test!(
    harvested_render_world_culled_sorted,
    0x009f_3c30,
    args = 4,
    |m: &mut Oracle, s| {
        // the original's frame: ESP at entry holds the return address, then
        // push ebp; the locals from ebp - 0x9c
        let frame = s.regs.esp - 4 - 0x9c - visibility::INIT_FRAME;
        world::bridge::culled_sorted(
            &mut Run(m),
            frame,
            s.ecx(),
            s.arg(0),
            s.arg(1),
            s.arg(2),
            s.arg(3),
        )
    }
);

harvest_test!(
    harvested_render_world,
    0x0062_be70,
    args = 3,
    |m: &mut Oracle, s| {
        world::bridge::render_world(&mut Run(m), s.ecx(), s.arg(0), s.arg(1), s.arg(2))
    }
);
