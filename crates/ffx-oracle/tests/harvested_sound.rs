//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the sound (ffx-sound) tests.
//! Every state the drivers reached on the base snapshots (`tools/harvest.sh`);
//! each test skips without `FFX_GAME_DIR` or stored states. New lines go at
//! the end of this file.

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::harvest::{PortResult, StateRet};
use ffx_oracle::{Conv, Oracle, harvest_test};

/// An oracle as the bridges' host: original callees run in the emulator,
/// `ST0` results included.
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

fn conv(c: host::Conv) -> Conv {
    match c {
        host::Conv::Cdecl => Conv::Cdecl,
        host::Conv::Stdcall => Conv::Stdcall,
        host::Conv::Thiscall(t) => Conv::Thiscall(t),
        host::Conv::Fastcall => Conv::Fastcall,
    }
}

impl Host for Run<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(c, false));
        self.0
            .call(addr, conv(c), args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }

    fn call_st0(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f32 {
        ffx_oracle::check_width("Host::call_st0", addr, args.len(), host::max_words(c, true));
        let r = self
            .0
            .call(addr, conv(c), args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"));
        r.st0_f32().expect("a float in ST0")
    }

    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

/// The pan when the request ran; without an id or parameters the original
/// returns at once with its stack cookie in `eax` (garbage), so that is
/// left unchecked.
struct PanIfRan(Option<u32>);

impl PortResult for PanIfRan {
    fn compare(self, o: &StateRet) -> Result<(), String> {
        self.0.map_or(Ok(()), |pan| pan.compare(o))
    }
}

// junk round 1: the positioned footstep request (its frame in stack
// scratch, the never-written direction 0 as the hook has it)
harvest_test!(
    snd3d_foot_request,
    0x0087_6950,
    args = 3,
    |m: &mut Oracle, s| {
        let req = s.arg(0);
        let ran = Mem::read::<u32>(m, req) != 0 && Mem::read::<u32>(m, req + 4) != 0;
        let work = m.stack_range().start + 0x400;
        let pan = ffx_sound::foot3d::bridge::snd3d_foot_request(
            &mut Run(m),
            req,
            s.arg(1),
            s.arg(2).cast_signed(),
            work,
        );
        PanIfRan(ran.then_some(pan))
    }
);
