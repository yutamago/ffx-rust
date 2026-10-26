//! `PRenderThread::handleNonRenderInstanceCommand` (`0x00614c60`) of list
//! p6-render-c (continuation round 5): the render thread's dispatcher for
//! the queued commands that are not mesh instances.
//!
//! A command is 0x48 bytes: its kind (word `+0`) and payload words from
//! `+8`. Kinds 4 to 43 go through a jump table (`0x006150a0`) to a
//! renderer call (thiscall on `renderer`, the payload as its words; float
//! words through `fld`/`fstp`, byte words zero-extended). Its `eax` is the
//! dispatcher's, except for the state setters (kinds 24 to 39): after one,
//! bit `k` of the dirty mask (`[dirty + 8]`) says whether the state block
//! `dirty` is flushed (`0x0061c870`, thiscall on it), and `eax` is the
//! flush's or the masked word. Kind 15 and kinds outside 4..=43 do
//! nothing and leave `eax` = kind - 4.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// The state block's flush (thiscall on the block, no words).
pub const FLUSH: u32 = 0x0061_c870;
/// `Phyre_PSemaphoreWin32_signal(this; count, previous)` (thiscall).
pub const SEMAPHORE_SIGNAL: u32 = 0x0043_0210;
/// The render thread's semaphore (`+0x164`) signalled after a present.
pub const PRESENT_SEMAPHORE: u32 = 0x164;

/// How a payload word is passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    /// The word at this offset.
    W(u32),
    /// The float at this offset (`fld dword` / `fstp dword`: a signalling
    /// NaN comes out quiet).
    F(u32),
    /// The byte at this offset, zero-extended.
    B(u32),
}

/// What a command does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `f(renderer; args)`, its `eax` the result.
    Call(u32, &'static [Arg]),
    /// `f(renderer; args)`, then the dirty-mask test of bit `.2` (`None`:
    /// the callee's `eax` with its low byte replaced by bit 0 of the
    /// mask's low byte).
    State(u32, &'static [Arg], Option<u32>),
    /// Kind 14: `[+0x14]` 0 picks the first callee, else the second;
    /// both take (`+0xc`, `+8`, `+0x10`).
    Pick(u32, u32),
    /// Kind 21: `renderer.+4` bit 0 cleared, then `0x00593010(renderer)`.
    Resolve,
    /// Kind 22: `0x00593760(renderer)` (present), then the render
    /// thread's semaphore signalled once.
    Present,
    /// Kind 15: nothing.
    Nothing,
}

use Arg::{B, F, W};

const WIDE: &[Arg] = &[
    B(8),
    W(0xc),
    W(0x10),
    W(0x14),
    W(0x18),
    W(0x1c),
    W(0x20),
    W(0x24),
    W(0x28),
    W(0x2c),
    W(0x30),
    W(0x34),
    W(0x38),
    W(0x3c),
    W(0x40),
];

/// Kinds 4 to 43, in order (read off the jump table's targets).
pub const KINDS: [Kind; 40] = [
    Kind::Call(0x0061_5de0, &[W(8), W(0xc), W(0x10), W(0x14)]),
    Kind::Call(0x0059_aaf0, &[W(8), W(0xc), W(0x10), W(0x14)]),
    Kind::Call(0x0061_5bc0, &[F(8), F(0xc), F(0x10), F(0x14)]),
    Kind::Call(0x0059_9cc0, &[W(8)]),
    Kind::Call(0x0059_9d80, &[W(8), W(0xc)]),
    Kind::Call(0x0059_9eb0, &[F(8)]),
    Kind::Call(0x0059_9ec0, &[W(8)]),
    Kind::Call(0x0061_5c50, &[W(8), W(0xc)]),
    Kind::Call(0x0061_5d00, &[W(8)]),
    Kind::Call(0x0058_ec50, &[W(8)]),
    Kind::Pick(0x0059_bc40, 0x0059_bc30),
    Kind::Nothing,
    Kind::Call(0x0058_e510, &[W(8), W(0xc), W(0x10)]),
    Kind::Call(0x0059_8d00, &[W(8), W(0xc), W(0x10)]),
    Kind::Call(0x0058_ebb0, &[W(0xc), W(8)]),
    Kind::Call(0x0061_47b0, &[W(8)]),
    Kind::Call(0x0058_e3d0, &[W(8)]),
    Kind::Resolve,
    Kind::Present,
    Kind::Call(0x0059_3790, &[]),
    Kind::State(0x0059_98f0, &[B(8), W(0xc), W(0x10)], None),
    Kind::State(
        0x0059_9950,
        &[B(8), W(0xc), W(0x10), W(0x14), W(0x18)],
        None,
    ),
    Kind::State(0x0061_5b50, &[F(8), F(0xc), F(0x10), F(0x14)], Some(0xd)),
    Kind::State(0x0059_97a0, &[B(8), W(0xc), F(0x10)], Some(1)),
    Kind::State(0x0059_97b0, &[B(8), W(0xc)], Some(0xe)),
    Kind::State(0x0059_9810, &[W(8), W(0xc)], Some(0xf)),
    Kind::State(0x0059_a120, &[B(8), W(0xc)], Some(2)),
    Kind::State(0x0059_a1c0, &[B(8)], Some(3)),
    Kind::State(0x0059_a070, &[W(8)], Some(4)),
    Kind::State(0x0059_a8d0, &[W(8)], Some(5)),
    Kind::State(0x0061_5da0, &[B(8), F(0xc), F(0x10)], Some(6)),
    Kind::State(0x0059_9ed0, &[B(8), B(9), B(0xa), B(0xb)], Some(7)),
    Kind::State(0x0059_a8b0, &[F(8)], Some(9)),
    Kind::State(0x0059_a2b0, &[F(8)], Some(0xc)),
    Kind::State(0x0059_a8c0, &[W(8)], Some(0xa)),
    Kind::State(0x0059_bac0, WIDE, Some(8)),
    Kind::Call(0x0061_5cd0, &[B(8), F(0xc), F(0x10)]),
    Kind::Call(0x0059_7ff0, &[W(8)]),
    Kind::Call(0x0059_7f20, &[W(8)]),
    Kind::Call(0x0059_7f10, &[]),
];

/// `0x00593010`, `0x00593760`: kinds 21 and 22's callees.
pub const RESOLVE_TARGETS: u32 = 0x0059_3010;
pub const PRESENT: u32 = 0x0059_3760;

pub mod bridge {
    //! The original over game memory.

    use super::{
        Arg, Conv, FLUSH, Host, KINDS, Kind, PRESENT, PRESENT_SEMAPHORE, RESOLVE_TARGETS,
        SEMAPHORE_SIGNAL, x87,
    };

    fn words(h: &mut impl Host, cmd: u32, args: &[Arg]) -> ([u32; 15], usize) {
        let mut out = [0u32; 15];
        for (w, &a) in out.iter_mut().zip(args) {
            *w = match a {
                Arg::W(k) => h.read(cmd.wrapping_add(k)),
                Arg::F(k) => x87::store(x87::load(h.read(cmd.wrapping_add(k)))),
                Arg::B(k) => u32::from(h.read::<u8>(cmd.wrapping_add(k))),
            };
        }
        (out, args.len().min(15))
    }

    /// `PRenderThread::handleNonRenderInstanceCommand(this; renderer,
    /// cmd, dirty)` (thiscall, `ret 0xc`): see the module. Returns `eax`
    /// (its caller, `processRenderCommands`, drops it).
    pub fn handle(h: &mut impl Host, thread: u32, renderer: u32, cmd: u32, dirty: u32) -> u32 {
        let op = h.read::<u32>(cmd).wrapping_sub(4);
        let Some(&kind) = KINDS.get(op as usize) else {
            return op;
        };
        match kind {
            Kind::Nothing => op,
            Kind::Call(f, args) => {
                let (w, n) = words(h, cmd, args);
                h.call_sym(f, Conv::Thiscall(renderer), &w[..n]) // call width: 15 at most (words)
            }
            Kind::Pick(zero, other) => {
                let pick: u32 = h.read(cmd.wrapping_add(0x14));
                let f = if pick == 0 { zero } else { other };
                let (w, n) = words(h, cmd, &[Arg::W(0xc), Arg::W(8), Arg::W(0x10)]);
                h.call_sym(f, Conv::Thiscall(renderer), &w[..n]) // call width: 15 at most (words)
            }
            Kind::Resolve => {
                let at = renderer.wrapping_add(4);
                let v: u32 = h.read(at);
                h.write(at, v & !1);
                h.call_sym(RESOLVE_TARGETS, Conv::Thiscall(renderer), &[])
            }
            Kind::Present => {
                h.call_sym(PRESENT, Conv::Thiscall(renderer), &[]);
                let semaphore = thread.wrapping_add(PRESENT_SEMAPHORE);
                h.call_sym(SEMAPHORE_SIGNAL, Conv::Thiscall(semaphore), &[1, 0])
            }
            Kind::State(f, args, bit) => {
                let (w, n) = words(h, cmd, args);
                let r = h.call_sym(f, Conv::Thiscall(renderer), &w[..n]); // call width: 15 at most
                let v = match bit {
                    None => {
                        let low: u8 = h.read(dirty.wrapping_add(8));
                        (r & 0xffff_ff00) | u32::from(low & 1)
                    }
                    Some(k) => (h.read::<u32>(dirty.wrapping_add(8)) >> k) & 0xffff_ff01,
                };
                if v as u8 == 0 {
                    v
                } else {
                    h.call_sym(FLUSH, Conv::Thiscall(dirty), &[])
                }
            }
        }
    }
}
