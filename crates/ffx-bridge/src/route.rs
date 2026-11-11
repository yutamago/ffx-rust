//! The call router (docs/host.md, "One source of truth"): every call from
//! Rust into the game's own code goes through here.
//!
//! - In the hooked game (`ffx-hook`, i686) no router is set, and a call is
//!   a native call of the address: exactly the words given, `ecx`/`edx` by
//!   the convention, the callee popping or not as its convention says, the
//!   result read from `eax`, `edx:eax` or `ST0` ([`native`]).
//! - In `ffx-host` the executor sets itself as the thread's [`Router`]
//!   while a port runs ([`enter`]), and a call goes to its function table:
//!   the port when there is one, the import's Rust side, else the original
//!   in Unicorn. The router also serves the game's memory to [`crate::RawMem`]
//!   on 64-bit hosts, and the scratch and stack frames that must live in
//!   the game's address space.
//!
//! The typed calls ([`cdecl`], [`stdcall`], [`thiscall`], [`fastcall`],
//! [`caller_pops`], [`x87`]) pass exactly the words given: a helper that
//! pads (the hook's `call_cdecl` passes twelve) pads before it calls.

use std::cell::Cell;
use std::ptr::NonNull;

/// How a function takes its arguments and who pops them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conv {
    /// Stack words; the caller pops.
    Cdecl,
    /// Stack words; the callee pops (`ret 4n`).
    Stdcall,
    /// `this` in `ecx`, stack words; the callee pops.
    Thiscall,
    /// `ecx`, `edx`, stack words; the callee pops.
    Fastcall,
    /// `ecx`, `edx`, stack words; the caller pops: the custom convention
    /// MSVC's link-time code generation made for some internal functions
    /// (docs/hooking.md, `fastcall-caller-pops`).
    FastcallCallerPops,
}

impl Conv {
    /// Whether the callee pops its stack words.
    #[must_use]
    pub const fn callee_pops(self) -> bool {
        matches!(self, Self::Stdcall | Self::Thiscall | Self::Fastcall)
    }

    /// How many leading arguments travel in registers (`ecx`, `edx`).
    #[must_use]
    pub const fn register_args(self) -> usize {
        match self {
            Self::Cdecl | Self::Stdcall => 0,
            Self::Thiscall => 1,
            Self::Fastcall | Self::FastcallCallerPops => 2,
        }
    }

    /// The `ported!` ABI string's convention.
    #[must_use]
    pub fn from_abi(abi: &str) -> Option<Self> {
        Some(match abi {
            "C" | "cdecl" => Self::Cdecl,
            "stdcall" => Self::Stdcall,
            "thiscall" => Self::Thiscall,
            "fastcall" => Self::Fastcall,
            "fastcall-caller-pops" => Self::FastcallCallerPops,
            _ => return None,
        })
    }
}

/// A call's arguments as the original's caller passed them: the register
/// arguments, then the stack words.
#[derive(Debug, Clone, Default)]
pub struct Args {
    /// `ecx`.
    pub ecx: u32,
    /// `edx`.
    pub edx: u32,
    /// The stack words, lowest address first.
    pub stack: Vec<u32>,
    /// The game address of the first stack word (a port whose original
    /// writes its own argument slots writes them there); 0 when the
    /// caller is Rust.
    pub slots: u32,
}

impl Args {
    /// The `i`-th argument in the original's order: the register ones
    /// first (by `conv`), then the stack words; 0 past the end.
    #[must_use]
    pub fn arg(&self, conv: Conv, i: usize) -> u32 {
        let regs = conv.register_args();
        match (regs, i) {
            (1.., 0) => self.ecx,
            (2, 1) => self.edx,
            _ => self.stack.get(i - regs).copied().unwrap_or(0),
        }
    }
}

/// What a function returns, where the original's caller reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Ret {
    /// `eax`.
    pub eax: u32,
    /// `edx` (64-bit results), or `None` to leave the caller's.
    pub edx: Option<u32>,
    /// `ST0` (float results), or `None` for an empty x87 stack.
    pub st0: Option<f64>,
    /// Whether `eax` is a result (`false`: the caller's `eax` is kept, as
    /// the hook's thunk keeps it for a void shim).
    pub sets_eax: bool,
}

impl Ret {
    /// Nothing returned: every register the caller's.
    pub const VOID: Self = Self {
        eax: 0,
        edx: None,
        st0: None,
        sets_eax: false,
    };

    /// A result in `eax`.
    #[must_use]
    pub const fn eax(v: u32) -> Self {
        Self {
            eax: v,
            edx: None,
            st0: None,
            sets_eax: true,
        }
    }

    /// A result in `edx:eax`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // the two halves
    pub const fn u64(v: u64) -> Self {
        Self {
            eax: v as u32,
            edx: Some((v >> 32) as u32),
            st0: None,
            sets_eax: true,
        }
    }

    /// A result in `ST0`.
    #[must_use]
    pub const fn st0(v: f64) -> Self {
        Self {
            eax: 0,
            edx: None,
            st0: Some(v),
            sets_eax: false,
        }
    }
}

/// Whether and how a call's `ST0` result is popped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Float {
    /// No float result: the x87 stack is left alone.
    None,
    /// `fstp dword`: one rounding from the 80-bit register.
    F32,
    /// `fstp qword`.
    F64,
}

/// A call's registers after it returned.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Raw {
    /// `eax`.
    pub eax: u32,
    /// `edx`.
    pub edx: u32,
    /// `ST0` as stored by the [`Float`] asked for (`F32`: the `f32` widened).
    pub st0: f64,
}

/// What a router does for the ports of its thread.
pub trait Router {
    /// Calls `addr` with `args` by `conv`; `float` says whether it
    /// returns in `ST0`.
    fn call(&mut self, addr: u32, conv: Conv, args: &Args, float: Float) -> Raw;
    /// Calls `addr` (a CRT intrinsic's import thunk) with `st` loaded on
    /// the x87 stack in order (the last in `ST0`); returns its `ST0`.
    fn call_x87(&mut self, addr: u32, st: &[f64], float: Float) -> f64;
    /// Fills `out` from game memory at `addr`.
    fn read_bytes(&mut self, addr: u32, out: &mut [u8]);
    /// Stores `data` in game memory at `addr`.
    fn write_bytes(&mut self, addr: u32, data: &[u8]);
    /// The thread's [`crate::SCRATCH_LEN`]-byte scratch number `slot` in
    /// game memory ([`Scratch`]: each host trait has its own, as each had
    /// its own thread-local buffer in the hooked game).
    fn scratch(&mut self, slot: Scratch) -> u32;
    /// `bytes` zeroed bytes on the thread's game stack (a port's frame),
    /// 16-byte aligned; released by [`Router::frame_free`] in reverse order.
    fn frame_alloc(&mut self, bytes: u32) -> u32;
    /// Releases the frame at `at` and every one allocated after it.
    fn frame_free(&mut self, at: u32);
}

thread_local! {
    static ROUTER: Cell<Option<NonNull<dyn Router>>> = const { Cell::new(None) };
}

/// The previous router, put back when dropped ([`enter`]).
#[derive(Debug)]
#[must_use = "the router is set until this is dropped"]
pub struct Entered {
    prev: Option<NonNull<dyn Router>>,
}

impl Drop for Entered {
    fn drop(&mut self) {
        ROUTER.with(|r| r.set(self.prev));
    }
}

/// Makes `router` this thread's router until the result is dropped.
///
/// # Safety
/// `router` stays valid, and is used by nothing else, until then (the
/// executor sets itself while a port runs, and the port's calls reach it
/// only through here).
pub unsafe fn enter(router: NonNull<dyn Router>) -> Entered {
    Entered {
        prev: ROUTER.with(|r| r.replace(Some(router))),
    }
}

/// Whether this thread has a router (runs in `ffx-host`).
#[must_use]
pub fn active() -> bool {
    ROUTER.with(Cell::get).is_some()
}

/// Runs `f` on this thread's router, if it has one.
pub fn with<R>(f: impl FnOnce(&mut (dyn Router + 'static)) -> R) -> Option<R> {
    let p = ROUTER.with(Cell::get)?;
    // SAFETY: `enter`'s caller keeps the router valid and unaliased while
    // it is set; calls nest, so this borrow ends before the router's own
    // frame resumes.
    Some(f(unsafe { &mut *p.as_ptr() }))
}

/// The primitive call: `addr` with `stack` words, `ecx`/`edx` as `conv`
/// takes them (ignored otherwise), `float` saying what to pop from `ST0`.
#[allow(clippy::must_use_candidate)] // a void original's result goes unread
pub fn call(addr: u32, conv: Conv, ecx: u32, edx: u32, stack: &[u32], float: Float) -> Raw {
    if let Some(r) = with(|r| {
        let args = Args {
            ecx,
            edx,
            stack: stack.to_vec(),
            slots: 0,
        };
        r.call(addr, conv, &args, float)
    }) {
        return r;
    }
    native(addr, ecx, edx, stack, &[], float)
}

/// Calls the CRT intrinsic behind the import thunk at `addr` (`_CIsin`,
/// `_CIpow`, ...) with `st` loaded on the x87 stack in order (`fld`, the
/// last one in `ST0`); returns its `ST0` stored as `float` asks.
#[allow(clippy::must_use_candidate)] // a void original's result goes unread
pub fn x87_raw(addr: u32, st: &[f64], float: Float) -> f64 {
    if let Some(v) = with(|r| r.call_x87(addr, st, float)) {
        return v;
    }
    native(addr, 0, 0, &[], st, float).st0
}

/// [`x87_raw`] read as `R` (`f32`: `fstp dword`; `f64`: `fstp qword`).
#[allow(clippy::must_use_candidate)] // a void original's result goes unread
pub fn x87<R: Out>(addr: u32, st: &[f64]) -> R {
    let st0 = x87_raw(addr, st, R::FLOAT);
    R::from_raw(&Raw {
        eax: 0,
        edx: 0,
        st0,
    })
}

/// A result type of the typed calls: where it comes from.
pub trait Out {
    /// What to pop from `ST0`.
    const FLOAT: Float;
    /// The value from the registers.
    fn from_raw(r: &Raw) -> Self;
}

impl Out for () {
    const FLOAT: Float = Float::None;
    fn from_raw(_: &Raw) -> Self {}
}

macro_rules! out_int {
    ($($t:ty),*) => {$(
        impl Out for $t {
            const FLOAT: Float = Float::None;
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // the low bits
            fn from_raw(r: &Raw) -> Self {
                r.eax as $t
            }
        }
    )*};
}
out_int!(u32, i32, u16, i16, u8, i8);

impl Out for bool {
    const FLOAT: Float = Float::None;
    fn from_raw(r: &Raw) -> Self {
        r.eax & 0xff != 0
    }
}

impl Out for u64 {
    const FLOAT: Float = Float::None;
    fn from_raw(r: &Raw) -> Self {
        u64::from(r.edx) << 32 | u64::from(r.eax)
    }
}

impl Out for i64 {
    const FLOAT: Float = Float::None;
    fn from_raw(r: &Raw) -> Self {
        u64::from_raw(r).cast_signed()
    }
}

impl Out for f32 {
    const FLOAT: Float = Float::F32;
    #[allow(clippy::cast_possible_truncation)] // exact: it was an f32
    fn from_raw(r: &Raw) -> Self {
        r.st0 as f32
    }
}

impl Out for f64 {
    const FLOAT: Float = Float::F64;
    fn from_raw(r: &Raw) -> Self {
        r.st0
    }
}

fn typed<R: Out>(addr: u32, conv: Conv, ecx: u32, edx: u32, stack: &[u32]) -> R {
    R::from_raw(&call(addr, conv, ecx, edx, stack, R::FLOAT))
}

/// A cdecl call of `addr` with exactly `args`.
#[allow(clippy::must_use_candidate)] // a void original's result goes unread
pub fn cdecl<R: Out>(addr: u32, args: &[u32]) -> R {
    typed(addr, Conv::Cdecl, 0, 0, args)
}

/// A stdcall call of `addr` with exactly `args` (the callee pops them).
#[allow(clippy::must_use_candidate)] // a void original's result goes unread
pub fn stdcall<R: Out>(addr: u32, args: &[u32]) -> R {
    typed(addr, Conv::Stdcall, 0, 0, args)
}

/// A thiscall call of `addr`: `this` in `ecx`, exactly `args` on the
/// stack (the callee pops them).
#[allow(clippy::must_use_candidate)] // a void original's result goes unread
pub fn thiscall<R: Out>(addr: u32, this: u32, args: &[u32]) -> R {
    typed(addr, Conv::Thiscall, this, 0, args)
}

/// A fastcall call of `addr`: the first two of `args` in `ecx` and `edx`
/// (zero when missing), the rest on the stack (the callee pops them).
#[allow(clippy::must_use_candidate)] // a void original's result goes unread
pub fn fastcall<R: Out>(addr: u32, args: &[u32]) -> R {
    let (ecx, edx) = (
        args.first().copied().unwrap_or(0),
        args.get(1).copied().unwrap_or(0),
    );
    typed(addr, Conv::Fastcall, ecx, edx, args.get(2..).unwrap_or(&[]))
}

/// A [`Conv::FastcallCallerPops`] call of `addr`: as [`fastcall`], the
/// caller popping the stack words.
#[allow(clippy::must_use_candidate)] // a void original's result goes unread
pub fn caller_pops<R: Out>(addr: u32, args: &[u32]) -> R {
    let (ecx, edx) = (
        args.first().copied().unwrap_or(0),
        args.get(1).copied().unwrap_or(0),
    );
    typed(
        addr,
        Conv::FastcallCallerPops,
        ecx,
        edx,
        args.get(2..).unwrap_or(&[]),
    )
}

/// Which per-thread scratch ([`Router::scratch`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scratch {
    /// `ffx_bridge::Host::callee_scratch`.
    Bridge,
    /// `ffx_graphic::host::Host::scratch`.
    Graphic,
    /// `ffx_script`'s `NativeHost::scratch`.
    Script,
    /// A shim's per-thread block that the game reads after the shim
    /// returns (`TOGetChangeMacroAddress`'s folded macro text, a
    /// `thread_local!` in the hooked game).
    Shim,
}

/// The thread's scratch `slot` in game memory, when a router serves one.
#[must_use]
pub fn scratch(slot: Scratch) -> Option<u32> {
    with(|r| r.scratch(slot))
}

/// What [`native`] hands its assembly (offsets fixed: the asm reads them).
#[cfg(target_arch = "x86")]
#[repr(C)]
struct NativeCall {
    addr: u32,         // 0
    ecx: u32,          // 4
    edx: u32,          // 8
    words: u32,        // 12
    stack: *const u32, // 16
    eax_out: u32,      // 20
    edx_out: u32,      // 24
    float: u32,        // 28: 0 none, 1 fstp dword, 2 fstp qword
    st0: [u32; 2],     // 32
    st: *const f64,    // 40
    sts: u32,          // 44
}

/// The call itself, in this process: loads `st` on the x87 stack, pushes
/// `stack` (last word first), sets `ecx` and `edx`, calls `addr`, puts
/// `esp` back (whoever popped), and reads `eax`, `edx` and, as `float`
/// says, `ST0`.
#[cfg(target_arch = "x86")]
#[must_use]
pub fn native(addr: u32, ecx: u32, edx: u32, stack: &[u32], st: &[f64], float: Float) -> Raw {
    #[allow(clippy::cast_possible_truncation)] // a 32-bit process
    let mut c = NativeCall {
        addr,
        ecx,
        edx,
        words: stack.len() as u32,
        stack: stack.as_ptr(),
        eax_out: 0,
        edx_out: 0,
        float: match float {
            Float::None => 0,
            Float::F32 => 1,
            Float::F64 => 2,
        },
        st0: [0; 2],
        st: st.as_ptr(),
        sts: st.len().min(7) as u32,
    };
    // SAFETY: `addr` is a function of this process taking these words and
    // registers by its convention (the caller's promise, as for the
    // transmuted calls this replaces). The callee keeps ebx, esi, edi and
    // ebp (every convention does), so `esp` comes back from edi whether
    // the callee popped its words or not; the x87 stack is empty here
    // (Rust keeps nothing in it) and left as the callee leaves it but for
    // the popped result.
    unsafe {
        std::arch::asm!(
            "push ebp",
            "push ebx",
            "push esi",
            "push edi",
            "mov ebx, eax",
            "mov edi, esp",
            "mov ecx, [ebx + 44]",
            "mov edx, [ebx + 40]",
            "2:",
            "test ecx, ecx",
            "jz 3f",
            "fld qword ptr [edx]",
            "add edx, 8",
            "dec ecx",
            "jmp 2b",
            "3:",
            "mov ecx, [ebx + 12]",
            "mov edx, [ebx + 16]",
            "4:",
            "test ecx, ecx",
            "jz 5f",
            "push dword ptr [edx + 4 * ecx - 4]",
            "dec ecx",
            "jmp 4b",
            "5:",
            "mov ecx, [ebx + 4]",
            "mov edx, [ebx + 8]",
            "call dword ptr [ebx]",
            "mov esp, edi",
            "mov [ebx + 20], eax",
            "mov [ebx + 24], edx",
            "mov ecx, [ebx + 28]",
            "cmp ecx, 1",
            "jne 6f",
            "fstp dword ptr [ebx + 32]",
            "jmp 7f",
            "6:",
            "cmp ecx, 2",
            "jne 7f",
            "fstp qword ptr [ebx + 32]",
            "7:",
            "pop edi",
            "pop esi",
            "pop ebx",
            "pop ebp",
            in("eax") &raw mut c,
            clobber_abi("C"),
        );
    }
    let st0 = match float {
        Float::None => 0.0,
        Float::F32 => f64::from(f32::from_bits(c.st0[0])),
        Float::F64 => f64::from_bits(u64::from(c.st0[1]) << 32 | u64::from(c.st0[0])),
    };
    Raw {
        eax: c.eax_out,
        edx: c.edx_out,
        st0,
    }
}

/// Off the game's architecture there is no native code to call: without a
/// router a call does nothing and returns zeros (said once on stderr).
#[cfg(not(target_arch = "x86"))]
#[must_use]
pub fn native(addr: u32, _: u32, _: u32, _: &[u32], _: &[f64], _: Float) -> Raw {
    use std::sync::atomic::{AtomicBool, Ordering};
    static SAID: AtomicBool = AtomicBool::new(false);
    if !SAID.swap(true, Ordering::Relaxed) {
        use std::io::Write;
        let _ = writeln!(
            std::io::stderr(),
            "ffx_bridge::route: no router on this thread; {addr:#010x} not called"
        );
    }
    Raw::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo(Vec<(u32, Conv, Args)>);

    impl Router for Echo {
        fn call(&mut self, addr: u32, conv: Conv, args: &Args, _: Float) -> Raw {
            self.0.push((addr, conv, args.clone()));
            Raw {
                eax: args.stack.iter().sum::<u32>() + args.ecx,
                edx: 7,
                st0: 1.5,
            }
        }
        fn call_x87(&mut self, _: u32, st: &[f64], _: Float) -> f64 {
            st.iter().sum()
        }
        fn read_bytes(&mut self, _: u32, out: &mut [u8]) {
            out.fill(0xab);
        }
        fn write_bytes(&mut self, _: u32, _: &[u8]) {}
        fn scratch(&mut self, _: Scratch) -> u32 {
            0x1234
        }
        fn frame_alloc(&mut self, _: u32) -> u32 {
            0
        }
        fn frame_free(&mut self, _: u32) {}
    }

    #[test]
    fn calls_reach_the_router() {
        let mut echo = Echo(Vec::new());
        {
            let p = NonNull::from(&mut echo as &mut dyn Router);
            // SAFETY: `echo` outlives the guard and is used only through it.
            let _g = unsafe { enter(p) };
            assert!(active());
            assert_eq!(cdecl::<u32>(0x10, &[1, 2]), 3);
            assert_eq!(thiscall::<u32>(0x20, 5, &[1]), 6);
            assert_eq!(fastcall::<u32>(0x30, &[4, 9, 1, 1]), 6);
            assert_eq!(cdecl::<u64>(0x10, &[1]), 7 << 32 | 1);
            assert!((cdecl::<f64>(0x10, &[]) - 1.5).abs() < f64::EPSILON);
            assert!((x87::<f64>(0x40, &[1.0, 2.0]) - 3.0).abs() < f64::EPSILON);
            assert_eq!(scratch(Scratch::Bridge), Some(0x1234));
        }
        assert!(!active());
        let (addr, conv, args) = &echo.0[2];
        assert_eq!(
            (*addr, *conv, args.ecx, args.edx),
            (0x30, Conv::Fastcall, 4, 9)
        );
        assert_eq!(args.stack, [1, 1]);
    }
}
