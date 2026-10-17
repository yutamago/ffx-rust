//! The interpreter `Script_Run` (`0x008641e0`) over game memory: the
//! dispatch loop and every opcode handler, the signals in
//! [`super::signal`], the natives through [`Natives`] (Rust ones from
//! [`super::native`]); the natives not ported yet, the run lock,
//! `MsBattleCheck` and the ctrl work's callbacks are called in the
//! original through [`Host`]. The arithmetic is [`crate::vm::exec`].

// w, l, s: the worker, its level and its stack, as everywhere in the bridge
#![allow(clippy::many_single_char_names)]

use ffx_bridge::{Global, Host, Mem};

use super::signal::{self, Request};

use super::{
    CONTEXT, ERROR, atel_get_basic_work, atel_pop_stack_float, atel_pop_stack_integer,
    atel_push_stack_float, atel_push_stack_integer, script_push_var, script_push_var_elem,
    script_push_var_ref, script_store_var, script_worker_level,
};
use crate::vm::exec::{
    self, LAST, at_least_one, bit_test, float_binary, float_compare, float_negate, int_binary,
    int_compare,
};
use crate::vm::{block, ctrl, header, level, stack, through_x87, worker};

/// `g_ScriptRunMutex`: the lock `Script_Run` holds while it runs (created
/// on first use).
pub const RUN_MUTEX: Global<u32> = Global::new(0x00c5_2740);
/// `g_GameSpeed` (`f32`): the factor on a worker's speed (`+0x94`).
pub const GAME_SPEED: Global<u32> = Global::new(0x0132_6bd8);
/// The global integer constants of opcode `75`.
pub const CONSTANTS: Global<u32> = Global::new(0x00c5_2704);

/// `Mutex_Create()`.
pub const MUTEX_CREATE: u32 = 0x006f_2de0;
/// `Mutex_Lock(mutex)`.
pub const MUTEX_LOCK: u32 = 0x006f_2ed0;
/// `Mutex_Unlock(mutex)`.
pub const MUTEX_UNLOCK: u32 = 0x006f_2f50;
/// `MsBattleCheck()`.
pub const MS_BATTLE_CHECK: u32 = 0x0078_0c90;
/// `AtelExecReturn(worker id, value, kind)`.
pub const ATEL_EXEC_RETURN: u32 = 0x0086_91d0;
/// Wait state 1's check, `(worker, level) -> request or 0`.
pub const WAIT_CHECK: u32 = 0x0086_35c0;
/// Wait state 1's end, `(worker id, request)`.
pub const WAIT_END: u32 = 0x0086_54c0;
/// Party slot to worker, `(slot) -> worker or < 0`.
pub const PARTY_WORKER: u32 = 0x0086_a5e0;
/// Request to another worker, `(from, to, kind, level, entry) -> accepted`
/// (opcodes `39`-`3b`).
pub const REQUEST: u32 = 0x0086_eba0;
/// Wait on another worker, `(worker, entry) -> waits` (opcodes `77`, `78`).
pub const WAIT_FOR: u32 = 0x0086_3570;
/// The request families of opcodes `36`, `37` and `38` (and `45`-`53`).
pub const FAMILY1: u32 = 0x0086_7230;
/// See [`FAMILY1`].
pub const FAMILY2: u32 = 0x0086_7570;
/// See [`FAMILY1`].
pub const FAMILY3: u32 = 0x0086_73d0;
/// Opcode `79`, `(worker, a, b, c)`.
pub const OP_79: u32 = 0x0087_0d30;
/// Opcode `7a`, `(worker, a, b) -> value`.
pub const OP_7A: u32 = 0x0086_fab0;

/// The native function table as the interpreter reaches it (opcodes `35`
/// and `58`): start a call, poll it each run until done, then push its
/// result. [`OriginalNatives`] calls the game's own (`Script_CallNative`,
/// `Script_PollNative`, `Script_FinishNative`), which dispatch through the
/// per-group tables `AtelInitTotal` registers.
pub trait Natives<H: Host> {
    /// Starts native `id` for `worker`; `args` is the level's `+0x2c`.
    fn start(&mut self, host: &mut H, id: u32, worker: u32, args: u32, stack: u32);
    /// Polls it: bit 0 done, bit 1 do not yield, bit 2 run on.
    fn poll(&mut self, host: &mut H, id: u32, worker: u32, args: u32) -> u8;
    /// Pushes its result.
    fn finish(&mut self, host: &mut H, id: u32, worker: u32, args: u32, stack: u32);
}

/// The game's own native dispatch: [`script_call_native`],
/// [`script_poll_native`] and [`script_finish_native`] over the tables
/// `AtelInitTotal` registers, the natives themselves original.
#[derive(Debug, Default, Clone, Copy)]
pub struct OriginalNatives;

/// `Script_CallNative`.
pub const CALL_NATIVE: u32 = 0x0087_77a0;
/// `Script_PollNative`.
pub const POLL_NATIVE: u32 = 0x0087_7730;
/// `Script_FinishNative`.
pub const FINISH_NATIVE: u32 = 0x0087_77f0;
/// `g_NativeTables`: per group (the id's bits 12 and up) a table of
/// `0x10`-byte entries (the low 12 bits): start, poll, float result, int
/// result, each a cdecl function or null.
pub const NATIVE_TABLES: Global<u32> = Global::new(0x0132_8558);

/// What [`script_finish_native`] needs beyond [`Host`]: a call whose result
/// comes back in `ST0` (the float natives).
pub trait NativeHost: Host {
    /// Calls the cdecl function at `addr` (rebased) and returns `ST0` as
    /// an `fstp m64` stores it (exact at the game's 53-bit precision).
    fn call_float(&mut self, addr: u32, args: &[u32]) -> f64;
    /// 64 bytes of memory the game can read and write for the duration of
    /// a native: what the originals' stack locals are when they pass their
    /// address to a callee (`yiGetFogColor(&rgba)`).
    fn scratch(&mut self) -> u32;
    /// [`Host::call`] for up to 12 arguments (the game's own `Host::call`
    /// takes six); the default is `Host::call`.
    fn call_n(&mut self, addr: u32, args: &[u32]) -> u32 {
        self.call(addr, args)
    }
    /// Calls a two-operand CRT intrinsic (`_CIatan2` through the game's
    /// import thunk at `addr`, rebased) with `a` in `ST1` and `b` in `ST0`;
    /// returns its `ST0` as an `fstp m64` stores it. The default (hosts
    /// that cannot run the CRT) returns NaN.
    fn call_intrinsic2(&mut self, addr: u32, a: f64, b: f64) -> f64 {
        let _ = (addr, a, b);
        f64::NAN
    }
    /// Calls a one-operand CRT intrinsic (`_CIatan` through the game's
    /// import thunk at `addr`, rebased) with `a` in `ST0`; returns its
    /// `ST0` as an `fstp m64` stores it. The default returns NaN.
    fn call_intrinsic1(&mut self, addr: u32, a: f64) -> f64 {
        let _ = (addr, a);
        f64::NAN
    }
}

#[cfg(target_pointer_width = "32")]
thread_local! {
    static SCRATCH: core::cell::UnsafeCell<[u32; 16]> = const { core::cell::UnsafeCell::new([0; 16]) };
}

#[cfg(target_pointer_width = "32")]
impl NativeHost for ffx_bridge::RawMem {
    fn call_float(&mut self, addr: u32, args: &[u32]) -> f64 {
        type F6 = unsafe extern "C" fn(u32, u32, u32, u32, u32, u32) -> f64;
        // no panic: the natives run inside the game; a longer list fails
        if args.len() > 6 {
            ffx_bridge::too_many_arguments("NativeHost::call_float", addr, args.len());
            return f64::NAN;
        }
        let mut a = [0u32; 6];
        a[..args.len()].copy_from_slice(args);
        // SAFETY: the address is a cdecl native of the game returning in
        // ST0 and taking at most these arguments; the caller pops them.
        unsafe {
            let f: F6 = std::mem::transmute(addr as usize);
            f(a[0], a[1], a[2], a[3], a[4], a[5])
        }
    }
    #[allow(clippy::many_single_char_names)]
    fn call_n(&mut self, addr: u32, args: &[u32]) -> u32 {
        type F12 =
            unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32;
        // no panic: the natives run inside the game
        let mut a = [0u32; 12];
        let n = args.len().min(12);
        a[..n].copy_from_slice(&args[..n]);
        // SAFETY: a cdecl function of the game taking at most 12 word
        // arguments; the extra words sit on the caller's side, which pops.
        unsafe {
            let f: F12 = std::mem::transmute(addr as usize);
            f(
                a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9], a[10], a[11],
            )
        }
    }
    #[cfg(target_arch = "x86")]
    fn call_intrinsic2(&mut self, addr: u32, a: f64, b: f64) -> f64 {
        let mut out = 0f64;
        // SAFETY: the thunk jumps to the CRT's intrinsic, which takes its
        // operands in ST1 and ST0 and leaves the result alone in ST0 (eax,
        // ecx, edx, the x87 and SSE registers clobbered). The x87 stack is
        // empty here (Rust keeps no values in it). Only edi, which the
        // callee keeps, is read after the call.
        unsafe {
            std::arch::asm!(
                "fld qword ptr [{a}]",
                "fld qword ptr [{b}]",
                "call {f}",
                "fstp qword ptr [edi]",
                a = in(reg) &raw const a,
                b = in(reg) &raw const b,
                f = in(reg) addr,
                in("edi") &raw mut out,
                clobber_abi("C"),
            );
        }
        out
    }
    #[cfg(target_arch = "x86")]
    fn call_intrinsic1(&mut self, addr: u32, a: f64) -> f64 {
        let mut out = 0f64;
        // SAFETY: as call_intrinsic2, with the one operand in ST0.
        unsafe {
            std::arch::asm!(
                "fld qword ptr [{a}]",
                "call {f}",
                "fstp qword ptr [edi]",
                a = in(reg) &raw const a,
                f = in(reg) addr,
                in("edi") &raw mut out,
                clobber_abi("C"),
            );
        }
        out
    }
    fn scratch(&mut self) -> u32 {
        // the game thread's own buffer: a 32-bit process, so the address
        // fits; natives do not nest
        #[allow(clippy::cast_possible_truncation)]
        SCRATCH.with(|s| s.get() as usize as u32)
    }
}

impl<H: NativeHost> Natives<H> for OriginalNatives {
    fn start(&mut self, host: &mut H, id: u32, worker: u32, args: u32, stack: u32) {
        script_call_native(host, id, worker, args, stack);
    }
    fn poll(&mut self, host: &mut H, id: u32, worker: u32, args: u32) -> u8 {
        script_poll_native(host, id, worker, args).to_le_bytes()[0]
    }
    fn finish(&mut self, host: &mut H, id: u32, worker: u32, args: u32, stack: u32) {
        script_finish_native(host, id, worker, args, stack);
    }
}

/// `Script_NativeEntry(id)` (`0x00877770`): the table entry of native
/// `id` (no bound on the group or the index).
pub fn script_native_entry(mem: &impl Mem, id: u32) -> u32 {
    let t: u32 = mem.read(NATIVE_TABLES.ptr(mem).addr.wrapping_add((id >> 12) * 4));
    t.wrapping_add((id & 0xfff) << 4)
}

/// `Script_CallNative(id, worker, args, stack)` (`0x008777a0`): a group 6
/// (camera) native sets the worker's flag `0x20` unless it has `0x10`;
/// then the entry's start function `(worker, args, stack)`, whose `EAX` it
/// returns (0 without one).
pub fn script_call_native<H: NativeHost>(host: &mut H, id: u32, w: u32, args: u32, s: u32) -> u32 {
    if id >> 12 == 6 {
        let f = host.read::<u16>(w + worker::FLAGS2);
        if f & 0x10 == 0 {
            host.write(w + worker::FLAGS2, f | 0x20);
        }
    }
    let f: u32 = host.read(script_native_entry(host, id));
    if f == 0 {
        return 0;
    }
    call_int(host, f, w, args, s, 3)
}

/// The `ffx.sym` address of a handler at `f`.
fn sym_of(host: &impl Mem, f: u32) -> u32 {
    f.wrapping_sub(host.image_base()).wrapping_add(0x0040_0000)
}

/// Calls an integer handler: its Rust version ([`super::native::find`])
/// or the original, with `n` of `(worker, args, stack)`.
fn call_int<H: NativeHost>(host: &mut H, f: u32, w: u32, args: u32, s: u32, n: usize) -> u32 {
    match super::native::find::<H>(sym_of(host, f)) {
        Some(super::native::Native::Int(r)) => r(host, w, args, s),
        _ => host.call(f, &[w, args, s][..n]),
    }
}

/// `Script_PollNative(id, worker, args)` (`0x00877730`): the entry's poll
/// function `(worker, args)`, or 5 (done, run on) without one.
pub fn script_poll_native<H: NativeHost>(host: &mut H, id: u32, w: u32, args: u32) -> u32 {
    let f: u32 = host.read(script_native_entry(host, id) + 4);
    if f == 0 {
        return 5;
    }
    call_int(host, f, w, args, 0, 2)
}

/// `Script_FinishNative(id, worker, args, stack)` (`0x008777f0`): pushes
/// the entry's float result (`ST0` stored as `f32`) or else its integer
/// result, or 0 without either.
pub fn script_finish_native<H: NativeHost>(host: &mut H, id: u32, w: u32, args: u32, s: u32) {
    let e = script_native_entry(host, id);
    let f: u32 = host.read(e + 8);
    if f != 0 {
        let x = match super::native::find::<H>(sym_of(host, f)) {
            Some(super::native::Native::Float(r)) => r(host, w, args, s),
            _ => host.call_float(f, &[w, args, s]),
        };
        // fstp m32, then fld/fstp of that f32 (a quiet NaN stays as it is)
        atel_push_stack_float(host, s, ffx_bridge::x87::store(x));
        return;
    }
    let f: u32 = host.read(e + 0xc);
    let v = if f == 0 {
        0
    } else {
        call_int(host, f, w, args, s, 3)
    };
    atel_push_stack_integer(host, s, v);
}

fn call(host: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
    let f = host.rebase(sym);
    host.call(f, args)
}

/// `AtelFetchCommand(worker, level)` (`0x00869d60`): the instruction at the
/// level's PC ([`exec::fetch`]).
pub fn atel_fetch_command(mem: &impl Mem, l: u32) -> u32 {
    let pc = mem.read::<u32>(l + level::PC);
    let b0 = mem.read::<u8>(pc);
    if b0 & 0x80 == 0 {
        return exec::fetch(b0, 0, 0);
    }
    let [b1, b2] = mem.read::<[u8; 2]>(pc.wrapping_add(1));
    exec::fetch(b0, b1, b2)
}

/// `AtelFetchCommandMain(long, pc)` (`0x00869da0`).
pub fn atel_fetch_command_main(mem: &impl Mem, long: u32, pc: u32) -> u32 {
    let b0 = mem.read::<u8>(pc);
    if b0 & 0x80 == 0 && long == 0 {
        return exec::fetch_main(false, b0, 0, 0);
    }
    let [b1, b2] = mem.read::<[u8; 2]>(pc.wrapping_add(1));
    exec::fetch_main(long != 0, b0, b1, b2)
}

/// `AtelFetchStackInteger(worker, stack)` (`0x0086a000`): the top slot as
/// an integer, not popped (opcode `2b`).
pub fn atel_fetch_stack_integer(mem: &mut impl Mem, s: u32) -> u32 {
    super::script_top_int(mem, 0, s)
}

/// `Script_TopTwoInts(worker, stack)` (`0x0086d9e0`): whether the top two
/// slots are both integers (the slots below an empty stack are read as
/// they lie).
pub fn script_top_two_ints(mem: &impl Mem, s: u32) -> u32 {
    let d = mem.read::<i32>(s + stack::DEPTH);
    let kinds = s.wrapping_add(stack::KINDS).wrapping_add_signed(d);
    let top = mem.read::<u8>(kinds.wrapping_sub(1));
    let below = mem.read::<u8>(kinds.wrapping_sub(2));
    u32::from(top == below && top == stack::INT)
}

/// The code start of a worker's block: offsets in the block's tables are
/// from here.
fn code(mem: &impl Mem, w: u32) -> u32 {
    let b = mem.read::<u32>(w + worker::BLOCK);
    mem.read::<u32>(b + block::CODE).wrapping_add(b)
}

/// `Script_Jump(worker, level, label)` (`0x008726f0`): the PC to label
/// `label` of the worker running at this call depth (the level's last
/// callee's labels inside a call, the worker's own outside).
pub fn script_jump(mem: &mut impl Mem, w: u32, l: u32, label: u32) {
    let depth = i32::from(mem.read::<i8>(l + level::CALL_DEPTH));
    let owner = if depth > 0 {
        let at = l.wrapping_add(depth.cast_unsigned().wrapping_mul(2)) + 0x0e;
        atel_get_basic_work(mem, i32::from(mem.read::<u16>(at)))
    } else {
        w
    };
    let h = mem.read::<u32>(owner + worker::HEADER);
    let labels = mem
        .read::<u32>(h + 0x24)
        .wrapping_add(mem.read::<u32>(owner + worker::BLOCK));
    let off = mem.read::<u32>(labels.wrapping_add(label.wrapping_mul(4)));
    mem.write(l + 0x1e, 0u8);
    let pc = code(mem, w).wrapping_add(off);
    mem.write(l + level::PC, pc);
}

/// Where a handler leaves the loop's PC.
enum Next {
    /// Step over the instruction (`0x00864512`).
    Advance,
    /// Leave the PC (jumps, calls, waits; `0x00864bf2`).
    Stay,
}

/// The interpreter's registers, as the original keeps them in its frame.
struct Run {
    w: u32,
    l: u32,
    s: u32,
    /// `param_2`: 1 = one instruction per run (the debugger's step).
    mode: u32,
    /// Run on after the stack empties (`[ebp+8]`).
    cont: u32,
    /// The result (`[ebp-4]`): bit 0 the speed accumulator reached 1,
    /// bits 14-15 from the debugger.
    result: u32,
    /// Whether the worker may yield (`[ebp-0x14]`), to flag `0x8` of
    /// worker `+0x34`.
    yield_ok: u32,
    /// The byte the PC steps over, when a breakpoint replaced it
    /// (`[ebp-0x18]`, -1 for none).
    op_byte: i32,
}

fn set_flags(mem: &mut impl Mem, w: u32, keep: u16, set: u16) {
    let f = mem.read::<u16>(w + worker::FLAGS);
    mem.write(w + worker::FLAGS, f & keep | set);
}

/// `Script_Run(worker, mode, callbacks)` (`0x008641e0`): runs worker `w`'s
/// current level until it waits, ends or empties its stack, under the run
/// lock; `callbacks` calls the ctrl work's `+0x58`/`+0x5c` around it.
/// Returns 1 when the worker's speed accumulator reached 1 this run (and,
/// with `mode` 1, the debugger's bits 14-15).
pub fn script_run<H: Host>(
    host: &mut H,
    natives: &mut impl Natives<H>,
    w: u32,
    mode: u32,
    callbacks: u32,
) -> u32 {
    script_run_traced(host, natives, w, mode, callbacks, |_, _| {})
}

/// [`script_run`], calling `trace(opcode, operand)` before each
/// instruction it executes (for tests and debugging).
#[allow(clippy::too_many_lines)] // the original's prologue and loop
pub fn script_run_traced<H: Host>(
    host: &mut H,
    natives: &mut impl Natives<H>,
    w: u32,
    mode: u32,
    callbacks: u32,
    mut trace: impl FnMut(u8, u32),
) -> u32 {
    if RUN_MUTEX.get(host) == u32::MAX {
        let m = call(host, MUTEX_CREATE, &[]);
        RUN_MUTEX.set(host, m);
    }
    let m = RUN_MUTEX.get(host);
    call(host, MUTEX_LOCK, &[m]);
    let flags = host.read::<u16>(w + worker::FLAGS);
    let skip = flags & 2 == 0
        || (flags & 4 == 0
            && host.read::<u8>(w + worker::FLAGS2) & 0x40 == 0
            && call(host, MS_BATTLE_CHECK, &[]) != 0)
        || {
            let c = CONTEXT.get(host);
            match host.read::<u8>(c + ctrl::MODE) & 3 {
                1 => true,
                2 => host.read::<u16>(w + worker::FLAGS2) & 0x8000 == 0,
                _ => false,
            }
        };
    if skip {
        let m = RUN_MUTEX.get(host);
        call(host, MUTEX_UNLOCK, &[m]);
        return 0;
    }
    let l = script_worker_level(host, w, i32::from(host.read::<u8>(w + worker::LEVEL)));
    let mut r = Run {
        w,
        l,
        s: w + worker::STACK,
        mode,
        cont: 0,
        result: 1,
        yield_ok: 1,
        op_byte: -1,
    };
    // the speed accumulator
    let x = exec::x87_binary(host.read(w + 0x94), GAME_SPEED.get(host), |a, b| a * b);
    let mut acc = exec::x87_binary(x, host.read(w + 0x98), |a, b| a + b);
    if at_least_one(x) {
        acc = exec::x87_binary(acc, 1f32.to_bits(), |a, b| a - b);
        r.cont = 1;
    }
    host.write(w + 0x98, acc);
    if !at_least_one(acc) {
        r.result = 0;
    }
    if callbacks != 0 {
        let f = host.read::<u32>(CONTEXT.get(host) + ctrl::RUN_BEGIN);
        host.call(f, &[w, l]);
    }
    if !wait(host, &r) {
        return finish(host, &mut r, callbacks, false);
    }
    set_flags(host, w, 0xe7ff, 0x400);
    let mut count = 0u32;
    loop {
        count += 1;
        if count > 0x1_0000 {
            set_flags(host, w, 0xf7ff, 0x1400);
            r.cont = 0;
            ERROR.set(host, 0);
        }
        if r.cont == 0 {
            let empty = host.read::<i32>(r.s + stack::DEPTH) == 0;
            if r.mode == 1 {
                if empty {
                    return finish(host, &mut r, callbacks, true);
                }
                r.yield_ok = 0;
                return finish(host, &mut r, callbacks, false);
            }
            if empty {
                return finish(host, &mut r, callbacks, true);
            }
        }
        let insn = atel_fetch_command(host, l);
        r.yield_ok = 1;
        let mut op = (insn >> 24).to_le_bytes()[0];
        let mut operand = insn & 0x00ff_ffff;
        let next = loop {
            if op > LAST {
                ERROR.set(host, 0);
                break Next::Stay;
            }
            if op == 0x40 {
                match breakpoint(host, &mut r) {
                    Some((o, x)) => {
                        op = o;
                        operand = x;
                        continue;
                    }
                    None => break Next::Stay,
                }
            }
            trace(op, operand);
            break execute(host, natives, &mut r, op, operand);
        };
        if let Next::Advance = next {
            advance(host, &r);
        }
        r.op_byte = -1;
        if r.mode == 1 {
            if r.cont != 0 {
                r.result |= 0x4000;
            }
            r.result &= 0xc000;
            r.cont = 0;
        }
    }
}

/// The wait states before a level runs (`0x008642f7`): false when it still
/// waits.
fn wait(host: &mut impl Host, r: &Run) -> bool {
    let l = r.l;
    loop {
        match host.read::<u8>(l + level::STATE) {
            0 => return true,
            1 => {
                let q = signal::atel_search_reply_signal(host, r.w, l);
                if q == 0 {
                    return false;
                }
                let id = u32::from(host.read::<u16>(r.w + worker::ID));
                signal::atel_free_signal(host, id, q);
                host.write(l + level::STATE, 0u8);
                return true;
            }
            2 => {
                let other = i32::from(host.read::<u16>(l + 0x2e));
                let wk = atel_get_basic_work(host, other);
                let entry = host.read::<u16>(l + 0x2c);
                let mut p = host.read::<u32>(wk + 0x80);
                while p != 0 {
                    let next = host.read::<u32>(p);
                    if host.read::<u8>(p + 0xf) != 3 && host.read::<u16>(p + 8) == entry {
                        return false;
                    }
                    p = next;
                }
                host.write(l + level::STATE, 0u8);
            }
            _ => return false,
        }
    }
}

/// The end (`0x008651c2`): the run-end callback, flag `0x8` of worker
/// `+0x34` from the yield flag, the lock released. `idle` is the path
/// where the yield flag stands as it is (`0x008651bf`); otherwise it
/// counts only with an empty stack (`0x008643c9`).
fn finish(host: &mut impl Host, r: &mut Run, callbacks: u32, idle: bool) -> u32 {
    let w = r.w;
    let y = if idle || host.read::<i32>(r.s + stack::DEPTH) == 0 {
        r.yield_ok
    } else {
        0
    };
    let lvl = script_worker_level(host, w, i32::from(host.read::<u8>(w + worker::LEVEL)));
    if callbacks != 0 {
        let f = host.read::<u32>(CONTEXT.get(host) + ctrl::RUN_END);
        host.call(f, &[w, lvl]);
    }
    let f = host.read::<u16>(w + worker::FLAGS);
    #[allow(clippy::cast_possible_truncation)] // the low bits
    let bit = (y << 3) as u16 & 8;
    host.write(w + worker::FLAGS, f & !8 | bit);
    let m = RUN_MUTEX.get(host);
    call(host, MUTEX_UNLOCK, &[m]);
    r.result
}

/// Steps the PC over the instruction (`0x00864512`).
fn advance(mem: &mut impl Mem, r: &Run) {
    let l = r.l;
    let pc = mem.read::<u32>(l + level::PC);
    let byte = if r.op_byte < 0 {
        mem.read::<u8>(pc)
    } else {
        r.op_byte.to_le_bytes()[0]
    };
    mem.write(l + 0x1e, 0u8);
    mem.write(l + level::PC, pc.wrapping_add(exec::length(byte)));
}

/// Opcode `40` (`0x0086442e`): a debugger breakpoint from the ctrl work's
/// table at `+0x64` (four entries of `0x10`: `+0` >= 0 when set, `+8` its
/// PC, `+0xe` the replaced byte, `+0xf` armed). Returns the replaced
/// instruction to run, or `None` to stop here.
fn breakpoint(host: &mut impl Host, r: &mut Run) -> Option<(u8, u32)> {
    r.cont = 0;
    set_flags(host, r.w, 0xe3ff, 0);
    let pc = host.read::<u32>(r.l + level::PC);
    let mut e = CONTEXT.get(host) + 0x64;
    let mut found = None;
    for _ in 0..4 {
        let saved = host.read::<u8>(e + 0xe);
        r.op_byte = i32::from(saved);
        if host.read::<i32>(e) >= 0 && host.read::<u32>(e + 8) == pc {
            found = Some((e, saved));
            break;
        }
        e += 0x10;
    }
    let (e, saved) = found?;
    if host.read::<u8>(e + 0xf) == 0 && r.mode != 1 {
        r.result |= 0x8000;
        host.write(e + 0xf, 1u8);
        r.mode = 1;
        return None;
    }
    host.write(e + 0xf, 0u8);
    let insn = atel_fetch_command_main(host, u32::from(saved & 0x80), pc);
    r.cont = 1;
    let op = exec::opcode(saved);
    if op > LAST {
        ERROR.set(host, 0);
        return None;
    }
    Some((op, insn & 0x00ff_ffff))
}

#[allow(clippy::too_many_lines)] // one arm per opcode family, as the original's switch
fn execute<H: Host>(
    host: &mut H,
    natives: &mut impl Natives<H>,
    r: &mut Run,
    op: u8,
    operand: u32,
) -> Next {
    let (w, l, s) = (r.w, r.l, r.s);
    let pop = |host: &mut H| atel_pop_stack_integer(host, s);
    let push = |host: &mut H, v: u32| atel_push_stack_integer(host, s, v);
    match op {
        0x00 | 0x1d | 0x1e | 0x76 => {}
        0x01 | 0x02 => {
            let b = pop(host);
            let a = pop(host);
            let v = if op == 0x01 {
                a != 0 || b != 0
            } else {
                a != 0 && b != 0
            };
            push(host, u32::from(v));
        }
        0x03..=0x05 | 0x12 | 0x13 | 0x18 => {
            let b = pop(host);
            let a = pop(host);
            push(host, int_binary(op, a, b).expect("an integer opcode"));
        }
        0x06..=0x0f => {
            let v = if script_top_two_ints(host, s) != 0 {
                let b = pop(host);
                let a = pop(host);
                int_compare(op, a, b)
            } else {
                let b = atel_pop_stack_float(host, s);
                let a = atel_pop_stack_float(host, s);
                float_compare(op, f32::from_bits(a), f32::from_bits(b))
            };
            push(host, u32::from(v.expect("a comparison")));
        }
        0x10 | 0x11 => {
            let b = pop(host);
            let a = pop(host);
            push(host, u32::from(bit_test(op, a, b).expect("a bit test")));
        }
        0x14..=0x17 => {
            if script_top_two_ints(host, s) != 0 {
                let b = pop(host);
                let a = pop(host);
                push(host, int_binary(op, a, b).expect("arithmetic"));
            } else {
                let b = atel_pop_stack_float(host, s);
                let a = atel_pop_stack_float(host, s);
                let v = float_binary(op, a, b).expect("arithmetic");
                atel_push_stack_float(host, s, v);
            }
        }
        0x19 => {
            let a = pop(host);
            push(host, u32::from(a == 0));
        }
        0x1a => {
            let a = atel_pop_stack_float(host, s);
            atel_push_stack_float(host, s, float_negate(a));
        }
        0x1c => {
            let a = pop(host);
            push(host, !a);
        }
        0x1b | 0x41..=0x44 => {
            ERROR.set(host, 0);
            return Next::Stay;
        }
        0x1f => script_push_var(host, w, s, operand, 0),
        0x20 => {
            script_store_var(host, w, s, operand, 0);
        }
        0x21 => {
            script_store_var(host, w, s, operand, 1);
        }
        0x22 => script_push_var_elem(host, w, s, operand),
        0x23 => {
            script_store_var(host, w, s, operand, 0x80);
        }
        0x24 => {
            script_store_var(host, w, s, operand, 0x81);
        }
        0x25 => {
            let v = atel_pop_stack_float(host, s);
            host.write(l + level::FACC, v);
        }
        0x26 => {
            let v = through_x87(host.read(l + level::FACC));
            atel_push_stack_float(host, s, v);
        }
        0x27 => script_push_var_ref(host, w, s, operand),
        0x28 => push(host, host.read(l + level::A)),
        0x29 => push(host, host.read(l + level::B)),
        0x2a => {
            let v = pop(host);
            host.write(l + level::A, v);
        }
        0x2b => {
            let v = atel_fetch_stack_integer(host, s);
            push(host, v);
        }
        0x2c => {
            let v = pop(host);
            host.write(l + level::B, v);
        }
        0x2d | 0x2f => {
            let field = if op == 0x2d {
                header::INTS
            } else {
                header::FLOATS
            };
            let h = host.read::<u32>(w + worker::HEADER);
            let at = host
                .read::<u32>(h + field)
                .wrapping_add(operand.wrapping_mul(4))
                .wrapping_add(host.read::<u32>(w + worker::BLOCK));
            let v = host.read::<u32>(at);
            if op == 0x2d {
                push(host, v);
            } else {
                atel_push_stack_float(host, s, through_x87(v));
            }
        }
        0x2e => {
            // the 16-bit immediate, sign-extended
            let [lo, hi, _, _] = operand.to_le_bytes();
            let v = i32::from(i16::from_le_bytes([lo, hi]));
            push(host, v.cast_unsigned());
        }
        0x30..=0x32 | 0x55..=0x57 => {
            if op >= 0x55 {
                let v = pop(host);
                host.write(l + level::A, v);
            }
            let a = host.read::<u32>(l + level::A);
            let take = match op {
                0x30 | 0x55 => true,
                0x31 | 0x56 => a != 0,
                _ => a == 0,
            };
            if !take {
                return Next::Advance;
            }
            script_jump(host, w, l, operand);
            return Next::Stay;
        }
        0x33 => {
            let depth = host.read::<i8>(l + level::CALL_DEPTH);
            if depth >= 3 {
                ERROR.set(host, 0);
                return Next::Advance;
            }
            let d = u32::from(depth.cast_unsigned());
            let ret = host
                .read::<u32>(l + level::PC)
                .wrapping_sub(code(host, w))
                .wrapping_add(3);
            host.write(l + d * 4, ret);
            #[allow(clippy::cast_possible_truncation)] // the 16-bit worker ID
            host.write(l + 0x10 + d * 2, operand as u16);
            host.write(l + level::CALL_DEPTH, depth.wrapping_add(1));
            let callee = atel_get_basic_work(host, operand.cast_signed());
            let entry = {
                let h = host.read::<u32>(callee + worker::HEADER);
                let b = host.read::<u32>(callee + worker::BLOCK);
                host.read::<u32>(b.wrapping_add(host.read::<u32>(h + 0x20)))
            };
            host.write(l + 0x1e, 0u8);
            let pc = code(host, w).wrapping_add(entry);
            host.write(l + level::PC, pc);
            return Next::Stay;
        }
        0x34 => {
            let depth = host.read::<i8>(l + level::CALL_DEPTH);
            if depth <= 0 {
                ERROR.set(host, 0);
                return Next::Advance;
            }
            let d = u32::from((depth - 1).cast_unsigned());
            let ret = host.read::<u32>(l + d * 4);
            host.write(l + 0x1e, 0u8);
            let pc = code(host, w).wrapping_add(ret);
            host.write(l + level::PC, pc);
            host.write(l + level::CALL_DEPTH, depth - 1);
            return Next::Stay;
        }
        0x35 | 0x58 => {
            set_flags(host, w, 0xe7ff, 0x400);
            r.cont = 0;
            let args = l + 0x2c;
            match host.read::<u8>(l + 0x1e) {
                0 => {
                    host.write(l + 0x1e, 1u8);
                    natives.start(host, operand, w, args, s);
                }
                1 => {}
                _ => return Next::Stay,
            }
            let p = natives.poll(host, operand, w, args);
            if p & 2 != 0 {
                r.yield_ok = 0;
            }
            if p & 4 != 0 {
                r.cont = 1;
            }
            if p & 1 == 0 {
                return Next::Stay;
            }
            natives.finish(host, operand, w, args, s);
            host.write(l + 0x1e, 0u8);
            if op == 0x58 {
                let v = atel_pop_stack_float(host, s);
                host.write(l + level::FACC, v);
            }
        }
        0x36 | 0x45..=0x49 => {
            signal::script_op_req(host, op, w, s);
        }
        0x37 | 0x4a..=0x4e | 0x38 | 0x4f..=0x53 => {
            r.cont = signal::script_op_req_wait(host, op, w, l, s);
            set_flags(host, w, 0xebff, 0x800);
        }
        0x39 => {
            let entry = pop(host);
            let slot = pop(host);
            let lvl = pop(host);
            let to = signal::atel_get_party_worker(host, slot);
            let v = if entry == 0 || to.cast_signed() < 0 {
                0
            } else {
                let id = u32::from(host.read::<u16>(w + worker::ID));
                signal::atel_request(host, Request::new(id, to, 0, lvl, entry))
            };
            push(host, v);
        }
        0x3a | 0x3b => {
            let entry = pop(host);
            let slot = pop(host);
            let lvl = pop(host);
            let to = signal::atel_get_party_worker(host, slot);
            let v = if entry != 0 && to.cast_signed() >= 0 {
                let id = u32::from(host.read::<u16>(w + worker::ID));
                let kind = if op == 0x3a { 1 } else { 2 };
                signal::atel_request(host, Request::new(id, to, kind, lvl, entry))
            } else {
                0
            };
            push(host, v);
            if v != 0 {
                #[allow(clippy::cast_possible_truncation)] // 16-bit fields
                {
                    host.write(l + 0x2c, entry as u16);
                    host.write(l + 0x2e, to as u16);
                    host.write(l + 0x30, lvl as u16);
                }
                host.write(l + level::STATE, 1u8);
            }
            r.cont = 0;
            set_flags(host, w, 0xebff, 0x800);
        }
        0x3c | 0x3e => {
            set_flags(host, w, 0xefff, 0xc00);
            r.cont = 0;
            let f = host.read::<u32>(CONTEXT.get(host) + 0x28);
            let id = u32::from(host.read::<u16>(w + worker::ID));
            if f != 0 {
                host.call(f, &[id, u32::from(op)]);
            } else {
                signal::atel_exec_return(host, id, u32::MAX, u32::from(op == 0x3e));
            }
            return Next::Stay;
        }
        0x3d | 0x3f => {
            set_flags(host, w, 0xefff, 0xc00);
            r.cont = 0;
            let v = pop(host);
            let id = u32::from(host.read::<u16>(w + worker::ID));
            signal::atel_exec_return(host, id, v, u32::from(op == 0x3f));
            return Next::Stay;
        }
        0x54 => {
            set_flags(host, w, 0xf3ff, 0x1000);
            r.cont = 0;
            let id = u32::from(host.read::<u16>(w + worker::ID));
            signal::atel_exec_return(host, id, 0, 0);
            return Next::Stay;
        }
        0x59..=0x5c => {
            let v = pop(host);
            host.write(w + 0x48 + u32::from(op - 0x59) * 4, v);
        }
        0x5d..=0x66 => {
            let v = atel_pop_stack_float(host, s);
            host.write(w + 0x58 + u32::from(op - 0x5d) * 4, v);
        }
        0x67..=0x6a => push(host, host.read(w + 0x48 + u32::from(op - 0x67) * 4)),
        0x6b..=0x74 => {
            let v = through_x87(host.read(w + 0x58 + u32::from(op - 0x6b) * 4));
            atel_push_stack_float(host, s, v);
        }
        0x75 => {
            let at = CONSTANTS.ptr(host).at(operand).addr;
            push(host, host.read(at));
        }
        0x77 | 0x78 => {
            let entry = pop(host);
            let a = pop(host);
            let (to, ok) = if op == 0x77 {
                (a, signal::atel_is_signal_queued(host, a, entry) != 0)
            } else {
                let to = signal::atel_get_party_worker(host, a);
                let ok = signal::atel_is_signal_queued(host, to, entry) != 0;
                (to, ok && to.cast_signed() >= 0)
            };
            if ok {
                #[allow(clippy::cast_possible_truncation)] // 16-bit fields
                {
                    host.write(l + 0x2e, to as u16);
                    host.write(l + 0x2c, entry as u16);
                }
                set_flags(host, w, 0xe7ff, 0x400);
                r.cont = 0;
                host.write(l + level::STATE, 2u8);
            }
        }
        0x79 => {
            let c = pop(host);
            let b = pop(host);
            let a = pop(host);
            signal::script_op79(host, w, a, b, c);
        }
        0x7a => {
            let b = pop(host);
            let a = pop(host);
            let v = signal::script_op7a(host, w, a, b);
            push(host, v);
        }
        _ => unreachable!("opcode {op:#x} is past the table or handled by the loop"),
    }
    Next::Advance
}
