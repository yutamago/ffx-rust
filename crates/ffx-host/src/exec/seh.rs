//! Structured exceptions in emulated code (docs/host.md, "Exceptions"), as
//! 32-bit Windows dispatches them: `RaiseException` (a C++ `throw` is
//! `RaiseException(0xe06d7363, ...)` from MSVCR110's `_CxxThrowException`)
//! and faults (an access violation, a division by zero, an illegal
//! instruction, a stray `int3`) walk the thread's registration chain at
//! `fs:[0]`, calling each frame's handler (MSVCR110's `__CxxFrameHandler3`,
//! `_except_handler4`) with the exception record, the frame, a `CONTEXT`
//! and a dispatcher context, as `RtlDispatchException` does; `RtlUnwind`
//! calls the handlers again with `EXCEPTION_UNWINDING` and unlinks the
//! frames down to the target.
//!
//! The dispatch runs in the executor's own loop, not as Rust recursion: a
//! handler is entered with a return address in the thread's stub page
//! (`int3`), so a handler that never returns (a C++ catch continues in the
//! catching function, `_JumpToContinuation`) simply goes on. Each handler
//! call is wrapped in a registration of the dispatcher's own, as Windows'
//! `RtlpExecuteHandler2` does: MSVCR110's catch code unlinks it
//! (`_UnwindNestedFrames`, `_JumpToContinuation`), and its handler reports
//! a nested exception. `RtlUnwind`'s handler calls return normally (they
//! run destructors and `__finally` blocks), so it calls them from Rust.
//!
//! A catch that continues in a frame older than a Rust port on the way
//! (the port called the code that threw) is found when that frame returns
//! to an outer level's sentinel: every Rust level in between is abandoned,
//! as an unwind abandons a frame without handlers ([`Exec::run`]).

use ffx_bridge::Mem as _;

#[cfg(feature = "dev-unicorn")]
use super::SAVED_REGS;
use super::{Exec, RegisterX86, Stop};
use crate::table::{Args, Ret};
use crate::{Error, Result};

/// `EXCEPTION_NONCONTINUABLE`.
const NONCONTINUABLE: u32 = 0x01;
/// `EXCEPTION_UNWINDING`.
const UNWINDING: u32 = 0x02;
/// `EXCEPTION_EXIT_UNWIND`.
const EXIT_UNWIND: u32 = 0x04;
/// `EXCEPTION_NESTED_CALL`.
#[cfg(feature = "dev-unicorn")]
const NESTED_CALL: u32 = 0x10;
/// The end of the registration chain.
const CHAIN_END: u32 = u32::MAX;
/// `STATUS_UNWIND`: the record `RtlUnwind` makes when given none.
const STATUS_UNWIND: u32 = 0xc000_0027;
/// The C++ exception code (`'msc' | 0xe0000000`).
pub const CXX_EXCEPTION: u32 = 0xe06d_7363;
/// `EXCEPTION_ACCESS_VIOLATION`.
pub const ACCESS_VIOLATION: u32 = 0xc000_0005;
/// `EXCEPTION_INT_DIVIDE_BY_ZERO`.
pub const DIVIDE_BY_ZERO: u32 = 0xc000_0094;
/// `EXCEPTION_ILLEGAL_INSTRUCTION`.
pub const ILLEGAL_INSTRUCTION: u32 = 0xc000_001d;
/// `EXCEPTION_BREAKPOINT`.
pub const BREAKPOINT: u32 = 0x8000_0003;

/// `sizeof(CONTEXT)` on x86.
const CONTEXT_SIZE: u32 = 0x2cc;
/// `sizeof(EXCEPTION_RECORD)`.
const RECORD_SIZE: u32 = 0x50;
/// The records' block below the stack: the `CONTEXT`, the record, the
/// dispatcher context.
const AREA: u32 = 0x330;
const AREA_RECORD: u32 = 0x2d0;
const AREA_DC: u32 = 0x320;
/// `CONTEXT_i386 | CONTROL | INTEGER | SEGMENTS | FLOATING_POINT`.
const CONTEXT_FLAGS: u32 = 0x0001_000f;
/// How deep exceptions may nest (one raised in a handler, ...).
#[cfg(feature = "dev-unicorn")]
const MAX_NESTED: usize = 32;

// CONTEXT offsets
const CTX_FLOAT: u32 = 0x1c;
const CTX_GS: u32 = 0x8c;
#[cfg(feature = "dev-unicorn")]
const CTX_EDI: u32 = 0x9c;
const CTX_EIP: u32 = 0xb8;
const CTX_CS: u32 = 0xbc;
const CTX_EFLAGS: u32 = 0xc0;
const CTX_ESP: u32 = 0xc4;
const CTX_SS: u32 = 0xc8;

/// The thread stub page's exception stubs (offsets from the sentinel).
pub(super) const STUB_RETURN: u32 = 0xf10;
const STUB_NESTED: u32 = 0xf20;
const STUB_SEARCH: u32 = 0xf50;

/// One dispatch in flight: the records in the block at `area`, the frame
/// whose handler runs.
#[cfg(feature = "dev-unicorn")]
#[derive(Debug, Clone, Copy)]
struct Dispatch {
    area: u32,
    /// The frame whose handler was called last.
    frame: u32,
    /// The innermost frame a nested exception searched (`0`: none).
    nested: u32,
    /// Where the handler's return address is.
    ret_sp: u32,
}

/// A run being unwound to an outer Rust level: a C++ catch continued in a
/// frame older than a port, and that frame returned to the outer level's
/// sentinel with these registers. (Without the emulator nothing
/// unwinds: the dispatch never starts.)
#[cfg_attr(not(feature = "dev-unicorn"), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub(super) struct Unwinding {
    /// The `esp` the outer level's sentinel return leaves.
    pub esp: u32,
    pub regs: [u64; 8],
}

/// A `RaiseException` waiting for the executor to start its dispatch.
/// (Without the emulator only its code is read, for the stop.)
#[cfg_attr(not(feature = "dev-unicorn"), allow(dead_code))]
#[derive(Debug, Clone)]
pub(super) struct Raise {
    pub(super) code: u32,
    flags: u32,
    params: Vec<u32>,
}

/// One thread's exception state.
#[derive(Debug, Default)]
pub(super) struct State {
    pub pending: Option<Raise>,
    #[cfg(feature = "dev-unicorn")]
    dispatches: Vec<Dispatch>,
    /// The `esp` each Rust → Unicorn call's sentinel return leaves,
    /// outermost first.
    #[cfg(feature = "dev-unicorn")]
    pub levels: Vec<u32>,
    pub unwinding: Option<Unwinding>,
    /// Exceptions dispatched (for the report and the tests).
    pub raised: u64,
}

/// What the exception at `rec` is, for a report: the code by name, the
/// C++ type (from the throw information), the access violation's address.
#[must_use]
pub fn describe(mem: &impl ffx_bridge::Mem, rec: u32) -> String {
    let code: u32 = mem.read(rec);
    let n: u32 = mem.read(rec + 0x10);
    let info = |i: u32| -> u32 {
        if i < n {
            mem.read(rec + 0x14 + 4 * i)
        } else {
            0
        }
    };
    match code {
        CXX_EXCEPTION => {
            let ty = cxx_type(mem, info(2));
            format!(
                "C++ exception {} (object {:#010x})",
                ty.as_deref().unwrap_or("of unknown type"),
                info(1)
            )
        }
        ACCESS_VIOLATION => {
            let what = match info(0) {
                0 => "read",
                1 => "write",
                8 => "execute",
                _ => "access",
            };
            format!("access violation ({what} at {:#010x})", info(1))
        }
        DIVIDE_BY_ZERO => "integer division by zero".into(),
        ILLEGAL_INSTRUCTION => "illegal instruction".into(),
        BREAKPOINT => "breakpoint (int3)".into(),
        c => format!("exception {c:#010x}"),
    }
}

/// The first catchable type's name in a C++ exception's `ThrowInfo`
/// (`attributes, pmfnUnwind, pForwardCompat, pCatchableTypeArray`; the
/// array `{count, types...}`; a type `{properties, TypeDescriptor*, ...}`;
/// the descriptor `{vftable, spare, name}`), undecorated a little
/// (`.?AVbad_alloc@std@@` → `std::bad_alloc`).
fn cxx_type(mem: &impl ffx_bridge::Mem, throw_info: u32) -> Option<String> {
    if throw_info == 0 {
        return None;
    }
    let array: u32 = mem.read(throw_info + 12);
    let count: u32 = mem.read(array);
    if array == 0 || count == 0 {
        return None;
    }
    let first: u32 = mem.read(array + 4);
    let desc: u32 = mem.read(first + 4);
    let mut name = Vec::new();
    for i in 0..256 {
        let b: u8 = mem.read(desc + 8 + i);
        if b == 0 {
            break;
        }
        name.push(b);
    }
    let raw = String::from_utf8_lossy(&name).into_owned();
    Some(undecorate(&raw))
}

/// `.?AVname@ns@@` (a class), `.?AUname@@` (a struct) → `ns::name`;
/// `.H` → `int` and the other builtins MSVC codes in one letter; anything
/// else as it is.
fn undecorate(raw: &str) -> String {
    let builtin = match raw {
        ".H" => Some("int"),
        ".I" => Some("unsigned int"),
        ".J" => Some("long"),
        ".K" => Some("unsigned long"),
        ".D" => Some("char"),
        ".E" => Some("unsigned char"),
        ".M" => Some("float"),
        ".N" => Some("double"),
        ".PAD" => Some("char *"),
        ".PBD" => Some("const char *"),
        _ => None,
    };
    if let Some(b) = builtin {
        return b.into();
    }
    let Some(body) = raw
        .strip_prefix(".?AV")
        .or_else(|| raw.strip_prefix(".?AU"))
        .and_then(|b| b.strip_suffix("@@"))
    else {
        return raw.into();
    };
    let mut parts: Vec<&str> = body.split('@').collect();
    parts.reverse();
    parts.join("::")
}

impl Exec {
    fn fs0(&self) -> u32 {
        self.mem().read(self.teb)
    }

    fn set_fs0(&self, v: u32) {
        let mut mem = self.mem();
        mem.write(self.teb, v);
    }

    /// Exceptions dispatched on this thread so far.
    #[must_use]
    pub fn exceptions_raised(&self) -> u64 {
        self.seh.raised
    }

    /// Writes the stubs the dispatcher uses into this thread's stub page
    /// (`sentinel` is its start).
    pub(super) fn write_seh_stubs(&self) {
        let arena = &self.shared.arena;
        arena.fill(self.sentinel + STUB_RETURN, &[super::INT3]);
        // The handler of the dispatcher's own registration (the frame at
        // [esp+8] holds the dispatched frame at +8): during a dispatch a
        // nested exception, the frame in the dispatcher context; during
        // an unwind, continue the search.
        let nested: [u8; 36] = [
            0x8b, 0x44, 0x24, 0x04, // mov eax, [esp+4]  (the record)
            0xf6, 0x40, 0x04, 0x06, // test byte [eax+4], 6  (unwinding)
            0x75, 0x13, // jnz search
            0x8b, 0x44, 0x24, 0x08, // mov eax, [esp+8]  (this registration)
            0x8b, 0x40, 0x08, // mov eax, [eax+8]  (the frame being dispatched)
            0x8b, 0x4c, 0x24, 0x10, // mov ecx, [esp+16]  (the dispatcher context)
            0x89, 0x01, // mov [ecx], eax
            0xb8, 0x02, 0x00, 0x00, 0x00, // mov eax, ExceptionNestedException
            0xc3, // ret
            0xb8, 0x01, 0x00, 0x00, 0x00, // search: mov eax, ExceptionContinueSearch
            0xc3, // ret
            0x90,
        ];
        arena.fill(self.sentinel + STUB_NESTED, &nested);
        // the unwind wrapper's handler: continue the search
        arena.fill(
            self.sentinel + STUB_SEARCH,
            &[0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3],
        );
    }

    /// `RaiseException(code, flags, n, args)` from emulated code: noted
    /// here, dispatched by the executor once the import returns
    /// ([`Exec::raise_pending`]).
    pub(crate) fn raise_exception(&mut self, args: &Args) -> Ret {
        let arg = |i: usize| args.stack.get(i).copied().unwrap_or(0);
        let n = arg(2).min(15);
        let mem = self.mem();
        let params = (0..n).map(|i| mem.read::<u32>(arg(3) + 4 * i)).collect();
        self.seh.pending = Some(Raise {
            code: arg(0),
            flags: arg(1) & NONCONTINUABLE,
            params,
        });
        Ret::VOID
    }

    /// Starts the dispatch of a pending `RaiseException` (its import's
    /// registers restored: `esp` at the return address); returns where to
    /// go on (the first handler).
    #[cfg(feature = "dev-unicorn")]
    pub(super) fn raise_pending(&mut self, stub: u32) -> Result<u32> {
        let Some(r) = self.seh.pending.take() else {
            return Err(Error::Interrupt(3, stub));
        };
        let esp = self.reg(RegisterX86::ESP)?;
        let ret: u32 = self.mem().read(esp);
        // the context RaiseException's caller continues with: after the
        // stdcall return
        self.dispatch(r.code, r.flags, stub, &r.params, ret, esp + 4 + 16)
    }

    /// Starts the dispatch of a fault at `eip` (the faulting instruction).
    #[cfg(feature = "dev-unicorn")]
    pub(super) fn raise_fault(&mut self, code: u32, eip: u32, params: &[u32]) -> Result<u32> {
        let esp = self.reg(RegisterX86::ESP)?;
        self.dispatch(code, 0, eip, params, eip, esp)
    }

    /// Writes the records below `resume_sp` and calls the first handler.
    #[cfg(feature = "dev-unicorn")]
    fn dispatch(
        &mut self,
        code: u32,
        flags: u32,
        address: u32,
        params: &[u32],
        resume_pc: u32,
        resume_sp: u32,
    ) -> Result<u32> {
        self.seh.raised += 1;
        let esp = self.reg(RegisterX86::ESP)?.min(resume_sp);
        // finished dispatches (a catch continued above them) are gone
        self.seh.dispatches.retain(|d| d.area >= esp);
        if self.seh.dispatches.len() >= MAX_NESTED {
            return self.unhandled_at(
                format!("{MAX_NESTED} nested exceptions (the last {code:#010x})"),
                resume_pc,
            );
        }
        let area = (esp - 0x100 - AREA) & !0xf;
        if area < self.stack.0 + 0x1000 {
            return Err(Error::StackOverflow(address));
        }
        let ctx = area;
        let rec = area + AREA_RECORD;
        self.write_context(ctx, resume_pc, resume_sp)?;
        let mut mem = self.mem();
        mem.write(rec, code);
        mem.write(rec + 4, flags);
        mem.write(rec + 8, 0u32);
        mem.write(rec + 0xc, address);
        mem.write(rec + 0x10, u32::try_from(params.len()).unwrap_or(0).min(15));
        for (i, &p) in (0u32..15).zip(params) {
            mem.write(rec + 0x14 + 4 * i, p);
        }
        mem.write(area + AREA_DC, 0u32);
        let d = Dispatch {
            area,
            frame: self.fs0(),
            nested: 0,
            ret_sp: 0,
        };
        self.seh.dispatches.push(d);
        self.call_handler()
    }

    /// Calls the handler of the top dispatch's frame (or ends the thread
    /// at the chain's end); returns where to go on.
    #[cfg(feature = "dev-unicorn")]
    fn call_handler(&mut self) -> Result<u32> {
        let Some(mut d) = self.seh.dispatches.pop() else {
            return Err(Error::Interrupt(3, self.sentinel + STUB_RETURN));
        };
        let rec = d.area + AREA_RECORD;
        let mut mem = self.mem();
        let frame = d.frame;
        if frame == CHAIN_END || frame == 0 {
            return self.unhandled(rec, d.area);
        }
        if frame < self.stack.0 || frame + 8 > self.stack.1 || frame & 3 != 0 {
            let why = format!(
                "{}, and the registration chain is broken at {frame:#010x}",
                describe(&mem, rec)
            );
            return self.unhandled_at(why, mem.read(d.area + CTX_EIP));
        }
        let handler: u32 = mem.read(frame + 4);
        // the dispatcher's registration {next, handler, frame}, then the
        // call: handler(record, frame, context, dispatcher context)
        let node = d.area - 0x10;
        mem.write(node, self.fs0());
        mem.write(node + 4, self.sentinel + STUB_NESTED);
        mem.write(node + 8, frame);
        self.set_fs0(node);
        let sp = node - 0x14;
        for (i, w) in (0u32..).zip([
            self.sentinel + STUB_RETURN,
            rec,
            frame,
            d.area,
            d.area + AREA_DC,
        ]) {
            mem.write(sp + 4 * i, w);
        }
        d.ret_sp = sp;
        self.seh.dispatches.push(d);
        self.uc.reg_write(RegisterX86::ESP, u64::from(sp))?;
        // handlers start with the direction flag clear
        let fl = self.uc.reg_read(RegisterX86::EFLAGS)?;
        self.uc.reg_write(RegisterX86::EFLAGS, fl & !0x400)?;
        Ok(handler)
    }

    /// A handler returned to the dispatcher's stub: its disposition in
    /// `eax` decides; returns where to go on.
    #[cfg(feature = "dev-unicorn")]
    pub(super) fn handler_returned(&mut self) -> Result<u32> {
        let esp = self.reg(RegisterX86::ESP)?;
        let Some(i) = self
            .seh
            .dispatches
            .iter()
            .rposition(|d| d.ret_sp + 4 == esp)
        else {
            return Err(Error::Interrupt(3, self.sentinel + STUB_RETURN));
        };
        self.seh.dispatches.truncate(i + 1);
        let mut d = self.seh.dispatches[i];
        let mut mem = self.mem();
        let node = d.area - 0x10;
        self.set_fs0(mem.read(node));
        let rec = d.area + AREA_RECORD;
        // past the innermost frame a nested exception searched: no longer
        // nested (after its handler, as RtlDispatchException does)
        if d.nested == d.frame {
            let flags: u32 = mem.read(rec + 4);
            mem.write(rec + 4, flags & !NESTED_CALL);
            d.nested = 0;
        }
        let disposition = self.reg(RegisterX86::EAX)?;
        match disposition {
            // ExceptionContinueExecution
            0 => {
                let flags: u32 = mem.read(rec + 4);
                if flags & NONCONTINUABLE != 0 {
                    let why = format!(
                        "a handler continued the noncontinuable {}",
                        describe(&mem, rec)
                    );
                    return self.unhandled_at(why, mem.read(d.area + CTX_EIP));
                }
                self.seh.dispatches.pop();
                self.read_context(d.area)
            }
            // ExceptionContinueSearch, ExceptionNestedException
            1 | 2 => {
                if disposition == 2 {
                    let flags: u32 = mem.read(rec + 4);
                    mem.write(rec + 4, flags | NESTED_CALL);
                    let inner: u32 = mem.read(d.area + AREA_DC);
                    d.nested = d.nested.max(inner);
                }
                d.frame = mem.read(d.frame);
                self.seh.dispatches[i] = d;
                self.call_handler()
            }
            other => {
                let why = format!(
                    "{}: handler of frame {:#010x} returned disposition {other}",
                    describe(&mem, rec),
                    d.frame
                );
                self.unhandled_at(why, mem.read(d.area + CTX_EIP))
            }
        }
    }

    /// The chain ended without a handler taking the exception: the
    /// thread ends (the main thread: the run) with a report.
    #[cfg(feature = "dev-unicorn")]
    fn unhandled(&mut self, rec: u32, area: u32) -> Result<u32> {
        let mem = self.mem();
        let eip: u32 = mem.read(area + CTX_EIP);
        self.unhandled_at(format!("unhandled {}", describe(&mem, rec)), eip)
    }

    #[cfg(feature = "dev-unicorn")]
    fn unhandled_at(&mut self, why: String, eip: u32) -> Result<u32> {
        self.seh.dispatches.clear();
        self.end(Stop::Failed(why, eip));
        Err(Error::Stopped)
    }

    /// `RtlUnwind(target, ip, record, value)`: calls every handler from
    /// the chain's head down to `target` (0: all) with
    /// `EXCEPTION_UNWINDING`, unlinking each frame, then returns `value`
    /// (on x86 `ip` is unused: the caller goes on after its call).
    pub(crate) fn rtl_unwind(&mut self, args: &Args) -> Ret {
        let arg = |i: usize| args.stack.get(i).copied().unwrap_or(0);
        let (target, value) = (arg(0), arg(3));
        let esp = self.uc.reg_read(RegisterX86::ESP).unwrap_or(0);
        let r = self.unwind(target, arg(2));
        let _ = self.uc.reg_write(RegisterX86::ESP, esp);
        match r {
            Ok(()) => Ret::eax(value),
            Err(Error::Stopped) => Ret::VOID,
            Err(e) => {
                let eip = self.reg(RegisterX86::EIP).unwrap_or(0);
                self.end(Stop::Failed(format!("RtlUnwind: {e}"), eip));
                Ret::VOID
            }
        }
    }

    fn unwind(&mut self, target: u32, record: u32) -> Result<()> {
        let esp = self.reg(RegisterX86::ESP)?;
        let ret: u32 = self.mem().read(esp);
        let area = (esp - 0x100 - AREA) & !0xf;
        if area < self.stack.0 + 0x1000 {
            return Err(Error::StackOverflow(ret));
        }
        let ctx = area;
        self.write_context(ctx, ret, esp + 4 + 16)?;
        let mut mem = self.mem();
        let rec = if record == 0 {
            let r = area + AREA_RECORD;
            mem.write(r, STATUS_UNWIND);
            for k in 1..(RECORD_SIZE / 4) {
                mem.write(r + 4 * k, 0u32);
            }
            mem.write(r + 0xc, ret);
            r
        } else {
            record
        };
        let flags: u32 = mem.read(rec + 4);
        let exit = if target == 0 { EXIT_UNWIND } else { 0 };
        mem.write(rec + 4, flags | UNWINDING | exit);
        mem.write(area + AREA_DC, 0u32);
        let node = area - 0x10;
        // the handlers run below the block
        self.uc
            .reg_write(RegisterX86::ESP, u64::from(node - 0x10))?;
        let mut frame = self.fs0();
        while frame != CHAIN_END && frame != target {
            if target != 0 && frame > target {
                return Err(Error::Image(format!(
                    "{}: unwind target {target:#010x} is not on the chain (frame {frame:#010x})",
                    describe(&mem, rec)
                )));
            }
            if frame < self.stack.0 || frame + 8 > self.stack.1 || frame & 3 != 0 {
                return Err(Error::Image(format!(
                    "{}: the registration chain is broken at {frame:#010x}",
                    describe(&mem, rec)
                )));
            }
            let handler: u32 = mem.read(frame + 4);
            mem.write(node, self.fs0());
            mem.write(node + 4, self.sentinel + STUB_SEARCH);
            self.set_fs0(node);
            let r = self.call_cdecl(handler, &[rec, frame, ctx, area + AREA_DC])?;
            self.set_fs0(mem.read(node));
            // ExceptionContinueSearch; a collided unwind (3) goes on too
            if r != 1 && r != 3 {
                return Err(Error::Image(format!(
                    "{}: unwind handler of frame {frame:#010x} returned disposition {r}",
                    describe(&mem, rec)
                )));
            }
            frame = mem.read(frame);
            self.set_fs0(frame);
        }
        if frame == CHAIN_END && target != 0 {
            return Err(Error::Image(format!(
                "{}: unwind target {target:#010x} is not on the chain",
                describe(&mem, rec)
            )));
        }
        Ok(())
    }

    /// The registers as a `CONTEXT` at `ctx`, `eip` and `esp` given.
    fn write_context(&mut self, ctx: u32, eip: u32, esp: u32) -> Result<()> {
        let mut mem = self.mem();
        for k in 0..CONTEXT_SIZE / 4 {
            mem.write(ctx + 4 * k, 0u32);
        }
        mem.write(ctx, CONTEXT_FLAGS);
        // FLOATING_SAVE_AREA: control, status, tag, ..., the registers ST0..ST7
        for (k, r) in (0u32..).zip([RegisterX86::FPCW, RegisterX86::FPSW, RegisterX86::FPTAG]) {
            mem.write(ctx + CTX_FLOAT + 4 * k, self.reg(r)? & 0xffff);
        }
        for (k, r) in (0u32..).zip(ST) {
            let v = self.uc.reg_read_long(r)?;
            mem.write_bytes(ctx + CTX_FLOAT + 0x1c + 10 * k, &v[..10]);
        }
        for (k, r) in (0u32..).zip([
            RegisterX86::GS,
            RegisterX86::FS,
            RegisterX86::ES,
            RegisterX86::DS,
            RegisterX86::EDI,
            RegisterX86::ESI,
            RegisterX86::EBX,
            RegisterX86::EDX,
            RegisterX86::ECX,
            RegisterX86::EAX,
            RegisterX86::EBP,
        ]) {
            mem.write(ctx + CTX_GS + 4 * k, self.reg(r)?);
        }
        mem.write(ctx + CTX_EIP, eip);
        mem.write(ctx + CTX_CS, self.reg(RegisterX86::CS)?);
        mem.write(ctx + CTX_EFLAGS, self.reg(RegisterX86::EFLAGS)?);
        mem.write(ctx + CTX_ESP, esp);
        mem.write(ctx + CTX_SS, self.reg(RegisterX86::SS)?);
        Ok(())
    }

    /// Loads the registers from the `CONTEXT` at `ctx` (a handler said
    /// continue); returns its `eip`.
    #[cfg(feature = "dev-unicorn")]
    fn read_context(&mut self, ctx: u32) -> Result<u32> {
        let mem = self.mem();
        for (k, r) in (0u32..).zip([
            RegisterX86::EDI,
            RegisterX86::ESI,
            RegisterX86::EBX,
            RegisterX86::EDX,
            RegisterX86::ECX,
            RegisterX86::EAX,
            RegisterX86::EBP,
        ]) {
            let v: u32 = mem.read(ctx + CTX_EDI + 4 * k);
            self.uc.reg_write(r, u64::from(v))?;
        }
        let esp: u32 = mem.read(ctx + CTX_ESP);
        let fl: u32 = mem.read(ctx + CTX_EFLAGS);
        self.uc.reg_write(RegisterX86::ESP, u64::from(esp))?;
        self.uc.reg_write(RegisterX86::EFLAGS, u64::from(fl))?;
        for (k, r) in (0u32..).zip([RegisterX86::FPCW, RegisterX86::FPSW, RegisterX86::FPTAG]) {
            let v: u32 = mem.read(ctx + CTX_FLOAT + 4 * k);
            self.uc.reg_write(r, u64::from(v & 0xffff))?;
        }
        for (k, r) in (0u32..).zip(ST) {
            let mut v = [0u8; 10];
            mem.read_bytes(ctx + CTX_FLOAT + 0x1c + 10 * k, &mut v);
            self.uc.reg_write_long(r, &v)?;
        }
        Ok(mem.read(ctx + CTX_EIP))
    }

    /// The outer Rust level a sentinel return at `esp` belongs to, if not
    /// the innermost (`expect`).
    #[cfg(feature = "dev-unicorn")]
    pub(super) fn outer_level(&self, esp: u32, expect: u32) -> bool {
        esp != expect && self.seh.levels.iter().rev().skip(1).any(|&l| l == esp)
    }

    /// Starts unwinding the Rust levels to the one whose sentinel return
    /// leaves `esp`.
    #[cfg(feature = "dev-unicorn")]
    pub(super) fn start_unwinding(&mut self, esp: u32) -> Result<()> {
        let regs = self.save_regs()?;
        self.seh.unwinding = Some(Unwinding { esp, regs });
        Ok(())
    }

    /// If this level (`expect`) is the one being unwound to: its
    /// registers back as the sentinel return left them; `true`.
    #[cfg(feature = "dev-unicorn")]
    pub(super) fn unwound_to(&mut self, expect: u32) -> Result<bool> {
        match self.seh.unwinding {
            Some(u) if u.esp == expect => {
                self.seh.unwinding = None;
                for (&v, r) in u.regs.iter().zip(SAVED_REGS) {
                    self.uc.reg_write(r, v)?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

const ST: [RegisterX86; 8] = [
    RegisterX86::ST0,
    RegisterX86::ST1,
    RegisterX86::ST2,
    RegisterX86::ST3,
    RegisterX86::ST4,
    RegisterX86::ST5,
    RegisterX86::ST6,
    RegisterX86::ST7,
];

#[cfg(test)]
mod tests {
    use super::undecorate;

    #[test]
    fn type_names() {
        assert_eq!(undecorate(".?AVbad_alloc@std@@"), "std::bad_alloc");
        assert_eq!(undecorate(".?AUfoo@@"), "foo");
        assert_eq!(undecorate(".H"), "int");
        assert_eq!(undecorate("odd"), "odd");
    }
}
