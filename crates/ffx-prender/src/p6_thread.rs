//! `PRenderThread` and render-state helpers of list p6-render-c
//! (continuation round): the singleton, the pool of 4 event flags (a
//! bit mask taken and given back with `InterlockedCompareExchange`), and
//! the state setters that forward to the D3D11 renderer's own.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `KERNEL32!InterlockedCompareExchange`'s import slot (stdcall
/// `(dest, exchange, comparand)` -> the old value).
pub const COMPARE_EXCHANGE: u32 = 0x00b0_c12c;
/// The render thread singleton and its first-use guard (bit 0).
pub const INSTANCE: u32 = 0x00cc_96f8;
pub const INSTANCE_GUARD: u32 = 0x00cc_9908;
/// `Phyre_PRendering_PRenderThread_PRenderThread(this)` (thiscall).
pub const THREAD_CTOR: u32 = 0x0061_4010;
/// The singleton's destructor, registered with `atexit`.
pub const INSTANCE_ATEXIT: u32 = 0x00b0_5f90;
/// The CRT's `atexit(f)` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// `Phyre_PEventFlagWin32` set-up `(flag; manual)` (thiscall, one word)
/// and its reset (thiscall).
pub const EVENT_FLAG_INIT: u32 = 0x0043_0050;
pub const EVENT_FLAG_RESET: u32 = 0x0043_01e0;
/// The thread's flag pool: 4 flags of 8 bytes from `+0x16c` (a flag's
/// "initialised" byte at `+4`), the taken mask at `+0x18c`.
pub const FLAGS: u32 = 0x16c;
pub const FLAG_MASK: u32 = 0x18c;
/// The forwarders' targets (thiscall on the renderer state).
pub const SET_BLEND_FACTOR: u32 = 0x0059_9d50;
pub const SET_SCISSOR_RECT: u32 = 0x0059_a200;
pub const SET_VIEWPORT_SLOT: u32 = 0x0059_9f30;
/// `0x0058c000(p)` (cdecl) and `0x0058c830(this; a)` (thiscall).
pub const STATE_BUSY: u32 = 0x0058_c000;
pub const STATE_APPLY: u32 = 0x0058_c830;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ATEXIT, COMPARE_EXCHANGE, Conv, EVENT_FLAG_INIT, EVENT_FLAG_RESET, FLAG_MASK, FLAGS, Host,
        INSTANCE, INSTANCE_ATEXIT, INSTANCE_GUARD, SET_BLEND_FACTOR, SET_SCISSOR_RECT,
        SET_VIEWPORT_SLOT, STATE_APPLY, STATE_BUSY, THREAD_CTOR, x87,
    };

    fn compare_exchange(h: &mut impl Host, dest: u32, exchange: u32, comparand: u32) -> u32 {
        let f: u32 = h.read(h.rebase(COMPARE_EXCHANGE));
        h.call(f, Conv::Stdcall, &[dest, exchange, comparand])
    }

    /// `PRenderThread::GetInstance()` (`0x00614410`, cdecl): the
    /// singleton, built on first use (guard bit 0, the constructor,
    /// `atexit`; the SEH frame dropped).
    pub fn get_instance(h: &mut impl Host) -> u32 {
        let guard = h.rebase(INSTANCE_GUARD);
        let g: u32 = h.read(guard);
        let instance = h.rebase(INSTANCE);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            h.call_sym(THREAD_CTOR, Conv::Thiscall(instance), &[]);
            let f = h.rebase(INSTANCE_ATEXIT);
            h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
        }
        instance
    }

    /// `PRenderThread::getEventFlag()` (`0x00614b70`, thiscall): takes
    /// the lowest free flag of the pool (its bit set with a
    /// compare-exchange; spins while all 4 are taken or the exchange
    /// loses), sets it up on first use (manual reset 0, its byte `+4`
    /// set), resets it and returns it.
    pub fn get_event_flag(h: &mut impl Host, thread: u32) -> u32 {
        let cell = thread.wrapping_add(FLAG_MASK);
        loop {
            let taken: u32 = h.read(cell);
            let mut bit = taken.wrapping_add(1) & !taken;
            if bit < 0x10 && compare_exchange(h, cell, bit | taken, taken) == taken {
                let mut flag = thread.wrapping_add(FLAGS);
                while bit > 1 {
                    bit >>= 1;
                    flag = flag.wrapping_add(8);
                }
                if h.read::<u8>(flag.wrapping_add(4)) == 0 {
                    h.call_sym(EVENT_FLAG_INIT, Conv::Thiscall(flag), &[0]);
                    h.write(flag.wrapping_add(4), 1u8);
                }
                h.call_sym(EVENT_FLAG_RESET, Conv::Thiscall(flag), &[]);
                return flag;
            }
        }
    }

    /// `0x00615ac0(thread; flag)` (thiscall): gives a pool flag back (its
    /// bit, from its offset, cleared with a compare-exchange until it
    /// sticks); nothing for no flag. `eax` (the old mask, or the
    /// caller's) is not read by its caller (`sync`).
    pub fn release_event_flag(h: &mut impl Host, thread: u32, flag: u32) {
        if flag == 0 {
            return;
        }
        let n = (flag.wrapping_sub(thread).wrapping_sub(FLAGS) as i32) >> 3;
        let keep = !(1u32 << (n as u32 & 31));
        let cell = thread.wrapping_add(FLAG_MASK);
        let mut old: u32 = h.read(cell);
        while compare_exchange(h, cell, keep & old, old) != old {
            old = h.read(cell);
        }
    }

    /// `0x00615bc0(state; r, g, b, a)` (thiscall, `ret 0x10`): the blend
    /// factor's four floats (through `fld`/`fstp`) to `0x00599d50`, which
    /// leaves `eax` as it found it.
    pub fn set_blend_factor(h: &mut impl Host, state: u32, rgba: [u32; 4]) {
        let args = rgba.map(|w| x87::round_trip(f32::from_bits(w)).to_bits());
        h.call_sym(SET_BLEND_FACTOR, Conv::Thiscall(state), &args);
    }

    /// `0x00615d00(state; rect)` (thiscall): the scissor rectangle (four
    /// words) kept at `+0x48`, the byte `+0x59` saying whether there is
    /// one, then `0x0059a200(state; rect)`'s result (a tail call).
    pub fn set_scissor_rect(h: &mut impl Host, state: u32, rect: u32) -> u32 {
        if rect != 0 {
            for k in 0..4 {
                let w: u32 = h.read(rect + 4 * k);
                h.write(state.wrapping_add(0x48 + 4 * k), w);
            }
        }
        h.write(state.wrapping_add(0x59), u8::from(rect != 0));
        h.call_sym(SET_SCISSOR_RECT, Conv::Thiscall(state), &[rect])
    }

    /// `0x00615c50(state; viewport, i)` (thiscall): viewport `i` (four
    /// words) kept at `+8 + 16 i` and bit `i` of the byte `+0x58` set, or
    /// for no viewport that bit cleared (the bit `1 << (i & 31)` in a
    /// byte: none for `i` from 8; viewport 5 overlaps the byte, which is
    /// updated after the copy); then `0x00599f30(state; viewport,
    /// i)`'s result.
    pub fn set_viewport_slot(h: &mut impl Host, state: u32, viewport: u32, i: u32) -> u32 {
        let bit = (1u32 << (i & 31)) as u8;
        if viewport != 0 {
            let at = state.wrapping_add(i.wrapping_shl(4)).wrapping_add(8);
            for k in 0..4 {
                let w: u32 = h.read(viewport + 4 * k);
                h.write(at.wrapping_add(4 * k), w);
            }
            // read after the copy: viewport 5's words cover this byte
            let mask: u8 = h.read(state.wrapping_add(0x58));
            h.write(state.wrapping_add(0x58), mask | bit);
        } else {
            let mask: u8 = h.read(state.wrapping_add(0x58));
            h.write(state.wrapping_add(0x58), mask & !bit);
        }
        h.call_sym(SET_VIEWPORT_SLOT, Conv::Thiscall(state), &[viewport, i])
    }

    /// `0x006147b0(this; a)` (thiscall): when `0x0058c000(this + 8)`
    /// (cdecl) says busy, `al = 0` over its result's upper bytes; else
    /// bit 0 of `+4` set and `0x0058c830(this; a)`'s result.
    pub fn apply_state(h: &mut impl Host, this: u32, a: u32) -> u32 {
        let r = h.call_sym(STATE_BUSY, Conv::Cdecl, &[this.wrapping_add(8)]);
        if r != 0 {
            return r & !0xff;
        }
        let f: u32 = h.read(this.wrapping_add(4));
        h.write(this.wrapping_add(4), f | 1);
        h.call_sym(STATE_APPLY, Conv::Thiscall(this), &[a])
    }
}
