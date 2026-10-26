//! List `p5-core`, continuation round:
//! `PRingBuffer<PPreprocessRingBufferBlock>::allocateBlock` (`0x005cd110`),
//! the preprocess ring buffer's allocator.
//!
//! The ring (`this`): its critical section at `+0`, the buffer's start
//! `+0x18`, end `+0x1c` and size `+0x20`, the write position `+0x24`, the
//! oldest block still to retire `+0x28` and the newest block `+0x2c`.
//! A block: the next block's address `+0`, busy `+4`, size `+8`.

use ffx_graphic::host::{Conv, Host};

/// `PCriticalSectionSimple::lock` / `unlock` (thiscall).
pub const LOCK: u32 = 0x0043_1920;
pub const UNLOCK: u32 = 0x0043_1a80;
/// `PRingBuffer_fits_005cd340` (thiscall `(ring; n)`, `al`).
pub const FITS: u32 = 0x005c_d340;
/// `PhyreOS::Sleep(ms)` (cdecl).
pub const SLEEP: u32 = 0x0043_0030;
/// `PhyrePrintf(level, format, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// The messages: waiting on a busy block, then the dump before the crash.
pub const MSG_WAITING: u32 = 0x00b3_8690;
pub const MSG_TOO_LONG: u32 = 0x00b3_86f8;
pub const MSG_START: u32 = 0x00b3_8760;
pub const MSG_BUFFER: u32 = 0x00b3_8788;
pub const MSG_HEAD: u32 = 0x00b3_87b4;
pub const MSG_END: u32 = 0x00b3_87e4;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        Conv, FITS, Host, LOCK, MSG_BUFFER, MSG_END, MSG_HEAD, MSG_START, MSG_TOO_LONG,
        MSG_WAITING, PRINTF, SLEEP, UNLOCK,
    };

    fn fits(h: &mut impl Host, ring: u32, n: u32) -> bool {
        h.call_sym(FITS, Conv::Thiscall(ring), &[n]) & 0xff != 0
    }

    /// Retires blocks from `+0x28` while they are not busy, up to the
    /// write position.
    fn retire(h: &mut impl Host, ring: u32) {
        let used: u32 = h.read(ring + 0x24);
        if h.read::<u32>(ring + 0x28) == used {
            return;
        }
        loop {
            let block: u32 = h.read(ring + 0x28);
            if h.read::<u32>(block.wrapping_add(4)) != 0 {
                return;
            }
            let next: u32 = h.read(block);
            h.write(ring + 0x28, next);
            if next == used {
                return;
            }
        }
    }

    fn printf(h: &mut impl Host, format: u32, args: &[u32]) {
        let f = h.rebase(format);
        let mut words = [2, f, 0, 0, 0];
        words[2..2 + args.len()].copy_from_slice(args);
        h.call_sym(PRINTF, Conv::Cdecl, &words[..2 + args.len()]);
    }

    /// `allocateBlock(this; size)` (thiscall, `ret 4`) -> the block or 0:
    /// `size` rounded up to 16 plus a 16-byte header; 0 when that is not
    /// less than the buffer, or with no write position. Under the lock:
    /// while the space is not free (`fits`, then retiring blocks), a wait
    /// (1 ms sleeps, retiring each time, a message every 1000; after
    /// 10000 the ring is printed and the process crashes on a write to
    /// address 0, as the original); then the block at the write position,
    /// or, when it would pass the end, the rest of the buffer as a filler
    /// block and a retry from the start. The new block is `+0x2c`.
    pub fn allocate_block(h: &mut impl Host, ring: u32, size: u32) -> u32 {
        let need = (size.wrapping_add(0xf) & !0xf).wrapping_add(0x10);
        if need >= h.read::<u32>(ring + 0x20) || h.read::<u32>(ring + 0x24) == 0 {
            return 0;
        }
        h.call_sym(LOCK, Conv::Thiscall(ring), &[]);
        let block = loop {
            if fits(h, ring, need) {
                retire(h, ring);
                if fits(h, ring, need) {
                    let mut spins: u32 = 0;
                    loop {
                        if spins.is_multiple_of(1000) {
                            let oldest: u32 = h.read(ring + 0x28);
                            printf(h, MSG_WAITING, &[oldest]);
                        }
                        h.call_sym(SLEEP, Conv::Cdecl, &[1]);
                        retire(h, ring);
                        let waited = spins;
                        spins += 1;
                        if waited > 10_000 {
                            printf(h, MSG_TOO_LONG, &[]);
                            printf(h, MSG_START, &[]);
                            let b: [u32; 3] = [
                                h.read(ring + 0x18),
                                h.read(ring + 0x1c),
                                h.read(ring + 0x20),
                            ];
                            printf(h, MSG_BUFFER, &b);
                            let p: [u32; 3] = [
                                h.read(ring + 0x24),
                                h.read(ring + 0x28),
                                h.read(ring + 0x2c),
                            ];
                            printf(h, MSG_HEAD, &p);
                            printf(h, MSG_END, &[]);
                            // the original's deliberate crash
                            h.write(0, 0u32);
                        }
                        if !fits(h, ring, need) {
                            break;
                        }
                    }
                }
            }
            let used: u32 = h.read(ring + 0x24);
            let end: u32 = h.read(ring + 0x1c);
            if used.wrapping_add(need) > end {
                // the rest of the buffer as a filler block; retry
                let start: u32 = h.read(ring + 0x18);
                h.write(ring + 0x24, start);
                h.write(used.wrapping_add(4), 0u32);
                h.write(used.wrapping_add(8), end.wrapping_sub(used));
                h.write(used, start);
                continue;
            }
            let mut next = used.wrapping_add(need);
            if next == end {
                next = h.read(ring + 0x18);
            }
            h.write(ring + 0x24, next);
            h.write(used.wrapping_add(4), 0u32);
            h.write(used.wrapping_add(8), size);
            h.write(used, next);
            break used;
        };
        h.write(ring + 0x2c, block);
        h.call_sym(UNLOCK, Conv::Thiscall(ring), &[]);
        block
    }
}
