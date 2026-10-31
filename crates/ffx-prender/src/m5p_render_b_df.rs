//! List `phyre-render-b`'s run-time frame and harness-owned flip (M5 stage
//! 1, round "design-frames"):
//! - `Renderer_SetDisplayMode` (`0x0058df00`): the output's display
//!   modes listed into an `_alloca_probe_16` buffer sized from their
//!   count (`DXGI_MODE_DESC`, 28 bytes each), the first of the asked
//!   width and height handed to `IDXGISwapChain::ResizeTarget`;
//! - `PRenderer::flip` (`0x005b6af0`): the flip command (`0x16`) put in
//!   the render ring, after kicking the render thread when the ring would
//!   wrap onto what it still reads; the harness's frame-boundary wrapper
//!   (`harness.rs` `flip`) owns the address and calls this port.
//!
//! `Renderer_SetDisplayMode`'s bridge is split at the alloca
//! (docs/porting.md, "Run-time frames and varargs"):
//! [`bridge::display_mode_count`] runs up to it and returns the count the
//! size comes from, [`bridge::set_display_mode`] takes the buffer.

/// One `DXGI_MODE_DESC` (width, height, refresh rate, format, scanline
/// ordering, scaling).
pub const MODE_DESC: u32 = 0x1c;
/// `DXGI_FORMAT_R8G8B8A8_UNORM`, the format the modes are listed in.
pub const MODE_FORMAT: u32 = 0x1c;
/// `"number of modes: %d\n"`.
pub const MODES_FORMAT: u32 = 0x00b3_3be0;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// What `Renderer_SetDisplayMode` returns when no mode matches.
pub const NO_MODE: u32 = 0x13;
/// `SetDisplayMode`'s locals the COM calls write by address: the adapter
/// (`ebp - 0x10`), the output (`ebp - 0xc`), the mode count (`ebp - 8`).
pub const MODE_LOCALS: u32 = 0xc;

/// `PRenderThread::GetInstance()` (cdecl).
pub const RENDER_THREAD: u32 = 0x0061_4410;
/// `PRenderThread::getFlipSlot(thread)` (thiscall).
pub const FLIP_SLOT: u32 = 0x0061_4c00;
/// `PRenderRingBuffer::stepForAllocation(ring; at, size, &end)` (thiscall,
/// ret 0xc).
pub const STEP: u32 = 0x0061_3f00;
/// `PRenderRingBufferSegmented::allocate(ring; size)` (thiscall, ret 4).
pub const ALLOCATE: u32 = 0x0061_3e10;
/// The flip command's code and size in the ring.
pub const FLIP_COMMAND: u32 = 0x16;
/// See [`FLIP_COMMAND`].
pub const FLIP_SIZE: u32 = 0x48;
/// What `flip` returns while a flip is pending (bit 0 of `+4`) and
/// without room in the ring.
pub const FLIP_BUSY: u32 = 0x16;
/// See [`FLIP_BUSY`].
pub const NO_MEMORY: u32 = 0xd;

/// The originals over game memory.
pub mod bridge {
    use super::{
        ALLOCATE, FLIP_BUSY, FLIP_COMMAND, FLIP_SIZE, FLIP_SLOT, MODE_DESC, MODE_FORMAT,
        MODES_FORMAT, NO_MEMORY, NO_MODE, PRINTF, RENDER_THREAD, STEP,
    };
    use ffx_graphic::host::{Conv, Host};

    /// A COM method: slot `offset` of `obj`'s vtable, `obj` first (stdcall).
    fn com(h: &mut impl Host, obj: u32, offset: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt.wrapping_add(offset));
        let mut words = [0u32; 6];
        words[0] = obj;
        let n = args.len().min(5);
        words[1..=n].copy_from_slice(&args[..n]);
        h.call(f, Conv::Stdcall, &words[..=n])
    }

    /// `Renderer_SetDisplayMode` up to its alloca, with `locals` its three
    /// words ([`super::MODE_LOCALS`] bytes at the original's `ebp - 0x10`):
    /// all 0; with the factory at `+0x133c`, `EnumAdapters(0, &adapter)`,
    /// `EnumOutputs(0, &output)` and `GetDisplayModeList(R8G8B8A8, 0,
    /// &count, NULL)` as far as each gives an object. The count, which
    /// sizes the buffer ([`display_modes_bytes`]).
    pub fn display_mode_count(h: &mut impl Host, this: u32, locals: u32) -> u32 {
        let (adapter, output, count) = (locals, locals + 4, locals + 8);
        let factory: u32 = h.read(this.wrapping_add(0x133c));
        h.write(adapter, 0u32);
        h.write(output, 0u32);
        h.write(count, 0u32);
        if factory == 0 {
            return 0;
        }
        com(h, factory, 0x1c, &[0, adapter]);
        let a: u32 = h.read(adapter);
        if a != 0 {
            com(h, a, 0x1c, &[0, output]);
        }
        let o: u32 = h.read(output);
        if o != 0 {
            com(h, o, 0x20, &[MODE_FORMAT, 0, count, 0]);
        }
        h.read(count)
    }

    /// The bytes `Renderer_SetDisplayMode` asks `_alloca_probe_16` for:
    /// `count` mode descriptions.
    #[must_use]
    pub const fn display_modes_bytes(count: u32) -> u32 {
        count.wrapping_mul(MODE_DESC)
    }

    /// The rest of `Renderer_SetDisplayMode(renderer; width, height)`
    /// (thiscall, ret 8) -> 0 or [`NO_MODE`], with `modes` the
    /// [`display_modes_bytes`] buffer: `PhyrePrintf(2, "number of modes:
    /// %d\n", count)`, `GetDisplayModeList(R8G8B8A8, 0, &count, modes)`
    /// on the output, and for the first mode of that width and height
    /// `ResizeTarget(mode)` on the swap chain at `+0x74`. The output must
    /// exist here, as in the original (which reads through a null one).
    pub fn set_display_mode(
        h: &mut impl Host,
        this: u32,
        width: u32,
        height: u32,
        locals: u32,
        count: u32,
        modes: u32,
    ) -> u32 {
        let fmt = h.rebase(MODES_FORMAT);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, fmt, count]);
        let output: u32 = h.read(locals + 4);
        com(h, output, 0x20, &[MODE_FORMAT, 0, locals + 8, modes]);
        let n: u32 = h.read(locals + 8);
        let mut i = 0u32;
        while i < n {
            let mode = modes.wrapping_add(i.wrapping_mul(MODE_DESC));
            if h.read::<u32>(mode) == width && h.read::<u32>(mode + 4) == height {
                let chain: u32 = h.read(this.wrapping_add(0x74));
                com(h, chain, 0x38, &[mode]);
                return 0;
            }
            i += 1;
        }
        NO_MODE
    }

    /// Whether `stepForAllocation`'s answer means the allocation would
    /// overtake the reader, as the original compares it: `start` and
    /// `limit` the ring's write and read positions, `end` the word it
    /// filled, `next` its `eax`.
    fn overtakes(start: u32, limit: u32, end: u32, next: u32) -> bool {
        if end < start {
            start <= limit
        } else {
            start <= limit && next > limit
        }
    }

    /// `PRenderer::flip(renderer)` (thiscall) -> 0, [`FLIP_BUSY`] or
    /// [`NO_MEMORY`], with `frame` its two words (`ebp - 8`, `ebp - 4`):
    /// nothing while a flip is pending (bit 0 of `+4`); else the render
    /// thread's flip slot taken (`getFlipSlot`), and when stepping either
    /// ring (`+0x1c` by 0 from `+0x24` against `+0x28`, then `+0x30` by
    /// the command's size from `+0x38` against `+0x3c`) would overtake
    /// its reader, the renderer's method `+8` (the kick); then the command
    /// allocated in ring `+0x30`, written (`0x16`), noted as the first
    /// (`+0x4c`, when none) and last (`+0x50` = its end), and `+0x13c`
    /// counted.
    pub fn flip(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        if h.read::<u8>(this.wrapping_add(4)) & 1 != 0 {
            return FLIP_BUSY;
        }
        let thread = h.call_sym(RENDER_THREAD, Conv::Cdecl, &[]);
        h.call_sym(FLIP_SLOT, Conv::Thiscall(thread), &[]);
        let (end_a, end_b) = (frame + 4, frame);
        let start: u32 = h.read(this.wrapping_add(0x24));
        let limit: u32 = h.read(this.wrapping_add(0x28));
        let next = h.call_sym(
            STEP,
            Conv::Thiscall(this.wrapping_add(0x1c)),
            &[start, 0, end_a],
        );
        let mut kick = overtakes(start, limit, h.read(end_a), next);
        if !kick {
            let start: u32 = h.read(this.wrapping_add(0x38));
            let limit: u32 = h.read(this.wrapping_add(0x3c));
            let next = h.call_sym(
                STEP,
                Conv::Thiscall(this.wrapping_add(0x30)),
                &[start, FLIP_SIZE, end_b],
            );
            kick = overtakes(start, limit, h.read(end_b), next);
        }
        if kick {
            let vt: u32 = h.read(this);
            let f: u32 = h.read(vt.wrapping_add(8));
            h.call(f, Conv::Thiscall(this), &[]);
        }
        let cmd = h.call_sym(
            ALLOCATE,
            Conv::Thiscall(this.wrapping_add(0x30)),
            &[FLIP_SIZE],
        );
        if cmd == 0 {
            return NO_MEMORY;
        }
        if h.read::<u32>(this.wrapping_add(0x4c)) == 0 {
            h.write(this.wrapping_add(0x4c), cmd);
        }
        h.write(this.wrapping_add(0x50), cmd.wrapping_add(FLIP_SIZE));
        h.write(cmd, FLIP_COMMAND);
        let n: u32 = h.read(this.wrapping_add(0x13c));
        h.write(this.wrapping_add(0x13c), n.wrapping_add(1));
        0
    }
}
