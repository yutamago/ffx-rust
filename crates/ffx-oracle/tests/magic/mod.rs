//! Parity harness for the Rust overlays (`ffx-magic`) against the
//! player's own `magic_*.dll` (docs/formats/magic.md, "Porting the
//! overlays").
//!
//! [`Twin`] loads one DLL twice: the original in Unicorn through
//! `ffx-overlay` (a 4 GiB arena, the DLL at its preferred base, the CRT
//! started with `DllMain`), and the port's view of the same memory (a
//! sparse page map, the same bytes). `g_MagicImports` is a stand-in:
//! every function word of the game's table (classified from the player's
//! `FFX.exe`) points at a trap, every data word at a zeroed page of its
//! own ([`Twin::data`]). A call runs the original and the port from the
//! same registers and compares:
//! - the return (`eax`, unless the test says the original's is junk; ST0;
//!   the bytes popped; a tail jump's target and stack);
//! - every call-out: target (slot), and the argument words the port
//!   passes against the original's stack;
//! - memory: every page either side's set-up or the port wrote, the
//!   image, the stack, the data pages and the heap, byte for byte.
//!
//! Stubs ([`Twin::stub`]) answer a slot for both sides alike and may read
//! and write memory. Tests run one at a time (the arena is shared).

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    clippy::unused_self
)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use ffx_bridge::Mem;
use ffx_magic::{Frame, Game, Image, Libm, Out, Ret};
use ffx_overlay::{Config, Host, Overlay, Pe, Regs};

pub const SERVICE: u32 = 0x7fff_0000;
pub const SENTINEL: u32 = SERVICE + 0xfff0;
pub const TABLE: u32 = 0x0080_0000;
pub const SLOTS: u32 = 1024;
pub const TRAPS: u32 = 0x0090_0000;
pub const DATA: u32 = 0x0100_0000;
pub const HEAP: u32 = 0x2000_0000;
pub const HEAP_LEN: u32 = 0x0010_0000;
pub const STACK_TOP: u32 = 0x3000_0000;
pub const STACK_LEN: u32 = 0x0001_0000;
/// Where tests put the objects their stubs hand out.
pub const SCRATCH: u32 = 0x4000_0000;
pub const SCRATCH_LEN: u32 = 0x0004_0000;
const ENTRY_ESP: u32 = STACK_TOP - 0x100;

struct Arena {
    base: usize,
}

fn arena() -> &'static Arena {
    static A: OnceLock<Arena> = OnceLock::new();
    A.get_or_init(|| {
        let layout = std::alloc::Layout::from_size_align(1 << 32, 1 << 16).unwrap();
        // SAFETY: a non-zero layout; pages stay untouched until used.
        let base = unsafe { std::alloc::alloc_zeroed(layout) } as usize;
        assert!(base != 0);
        ffx_overlay::configure(Config {
            base,
            service: SERVICE,
            import_table: Some((TABLE, SLOTS)),
        });
        Arena { base }
    })
}

fn lock() -> MutexGuard<'static, ()> {
    static L: Mutex<()> = Mutex::new(());
    L.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The arena as memory.
#[derive(Clone, Copy)]
pub struct ArenaMem;

impl Mem for ArenaMem {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        // SAFETY: inside the 4 GiB arena.
        unsafe {
            std::ptr::copy_nonoverlapping(
                (arena().base + addr as usize) as *const u8,
                out.as_mut_ptr(),
                out.len(),
            );
        }
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        // SAFETY: inside the 4 GiB arena.
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                (arena().base + addr as usize) as *mut u8,
                data.len(),
            );
        }
    }
    fn image_base(&self) -> u32 {
        0
    }
}

/// The port's memory: 4 KiB pages, zero until written.
#[derive(Default, Clone)]
pub struct PageMem {
    pages: BTreeMap<u32, Box<[u8; 4096]>>,
}

impl Mem for PageMem {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        let mut done = 0;
        while done < out.len() {
            let a = addr.wrapping_add(done as u32);
            let o = (a & 0xfff) as usize;
            let n = (4096 - o).min(out.len() - done);
            match self.pages.get(&(a >> 12)) {
                Some(p) => out[done..done + n].copy_from_slice(&p[o..o + n]),
                None => out[done..done + n].fill(0),
            }
            done += n;
        }
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        let mut done = 0;
        while done < data.len() {
            let a = addr.wrapping_add(done as u32);
            let o = (a & 0xfff) as usize;
            let n = (4096 - o).min(data.len() - done);
            let chunk = &data[done..done + n];
            match self.pages.get_mut(&(a >> 12)) {
                Some(p) => p[o..o + n].copy_from_slice(chunk),
                // a zero write to a page never written leaves it zero
                None if chunk.iter().all(|&b| b == 0) => {}
                None => {
                    let mut p = Box::new([0; 4096]);
                    p[o..o + n].copy_from_slice(chunk);
                    self.pages.insert(a >> 12, p);
                }
            }
            done += n;
        }
    }
    fn image_base(&self) -> u32 {
        0
    }
}

/// Memory a stub may use (either side's).
pub trait StubMem {
    fn r32(&self, a: u32) -> u32;
    fn w32(&mut self, a: u32, v: u32);
}

impl<M: Mem> StubMem for M {
    fn r32(&self, a: u32) -> u32 {
        self.read::<u32>(a)
    }
    fn w32(&mut self, a: u32, v: u32) {
        self.write::<u32>(a, v);
    }
}

type Stub = Box<dyn Fn(&mut dyn StubMem, &[u32]) -> u32>;
/// A stub's float result (ST0).
type FloatStub = Box<dyn Fn(&mut dyn StubMem, &[u32]) -> f64>;

/// One call-out: the target, its slot, the argument words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallOut {
    pub target: u32,
    pub slot: Option<u32>,
    pub args: Vec<u32>,
}

/// How many stack words a call-out's log keeps from the original's stack
/// (its arity is not known there; the port's own count is compared).
// 16: m0052::scaled_camera passes 13 (its earlier calls' words stay on the stack)
const LOGGED: usize = 16;

#[derive(Default)]
pub struct Shared {
    stubs: HashMap<u32, Stub>,
    floats: HashMap<u32, FloatStub>,
    rand: u32,
}

fn slot_of(target: u32) -> Option<u32> {
    ((TRAPS..TRAPS + 16 * SLOTS).contains(&target) && (target - TRAPS).is_multiple_of(16))
        .then(|| (target - TRAPS) / 16)
}

/// The ST0 a slot's float stub answers (after its `eax` stub ran).
fn answer_st0(shared: &Shared, m: &mut dyn StubMem, target: u32, args: &[u32]) -> Option<f64> {
    slot_of(target)
        .and_then(|s| shared.floats.get(&s))
        .map(|f| f(m, args))
}

/// An x87 80-bit value from an `f64` (exact).
fn f64_to_f80(x: f64) -> [u8; 10] {
    let b = x.to_bits();
    let sign = ((b >> 63) as u16) << 15;
    let exp = ((b >> 52) & 0x7ff) as u16;
    let frac = b & ((1 << 52) - 1);
    let (e, m): (u16, u64) = match exp {
        0 if frac == 0 => (0, 0),
        0 => {
            let shift = frac.leading_zeros() - 11;
            (15361 - shift as u16, frac << (shift + 11))
        }
        0x7ff => (0x7fff, (1 << 63) | (frac << 11)),
        _ => (exp - 1023 + 16383, (1 << 63) | (frac << 11)),
    };
    let mut out = [0u8; 10];
    out[..8].copy_from_slice(&m.to_le_bytes());
    out[8..].copy_from_slice(&(sign | e).to_le_bytes());
    out
}

fn answer(shared: &Shared, m: &mut dyn StubMem, target: u32, args: &[u32]) -> u32 {
    match slot_of(target).and_then(|s| shared.stubs.get(&s)) {
        Some(f) => f(m, args),
        None => 0,
    }
}

/// The original's host: Unicorn's call-outs logged and answered by the
/// stubs.
struct OrigHost<'a> {
    shared: &'a mut Shared,
    calls: Vec<CallOut>,
    heap: u32,
}

impl Host for OrigHost<'_> {
    fn call_out(&mut self, target: u32, slot: Option<u32>, regs: &mut Regs) -> Result<(), String> {
        let m = ArenaMem;
        let args: Vec<u32> = (0..LOGGED as u32)
            .map(|i| m.read::<u32>(regs.esp + 4 + 4 * i))
            .collect();
        let mut mm = ArenaMem;
        regs.eax = answer(self.shared, &mut mm, target, &args);
        if let Some(v) = answer_st0(self.shared, &mut mm, target, &args) {
            regs.st[0] = f64_to_f80(v);
            regs.st_len = 1;
        }
        self.calls.push(CallOut { target, slot, args });
        regs.esp += 4;
        Ok(())
    }

    fn import(&mut self, name: &str, regs: &mut Regs) -> Option<Result<(), String>> {
        if name == "rand" {
            self.shared.rand = self
                .shared
                .rand
                .wrapping_mul(214_013)
                .wrapping_add(2_531_011);
            regs.eax = (self.shared.rand >> 16) & 0x7fff;
            regs.esp += 4;
            return Some(Ok(()));
        }
        None
    }

    fn alloc(&mut self, size: u32) -> Option<u32> {
        let at = self.heap;
        self.heap = (self.heap + size + 15) & !15;
        (self.heap <= HEAP + HEAP_LEN).then_some(at)
    }

    fn free(&mut self, _addr: u32) {}
}

/// The port's game.
pub struct PortGame {
    pub mem: PageMem,
    shared: Shared,
    pub calls: Vec<CallOut>,
}

impl Mem for PortGame {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.mem.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        self.mem.write_bytes(addr, data);
    }
    fn image_base(&self) -> u32 {
        0
    }
}

impl Game for PortGame {
    fn call(&mut self, target: u32, _ecx: u32, args: &[u32]) -> Out {
        let eax = answer(&self.shared, &mut self.mem, target, args);
        let st0 = answer_st0(&self.shared, &mut self.mem, target, args);
        self.calls.push(CallOut {
            target,
            slot: slot_of(target),
            args: args.to_vec(),
        });
        Out {
            eax,
            st0,
            ..Out::default()
        }
    }
    fn rand(&mut self) -> u32 {
        self.shared.rand = self
            .shared
            .rand
            .wrapping_mul(214_013)
            .wrapping_add(2_531_011);
        (self.shared.rand >> 16) & 0x7fff
    }
    fn libm(&mut self, f: Libm, x: f64) -> f64 {
        match f {
            Libm::Sin => x.sin(),
            Libm::Cos => x.cos(),
            Libm::Atan => x.atan(),
            Libm::Acos => x.acos(),
        }
    }
    fn atan2(&mut self, y: f64, x: f64) -> f64 {
        y.atan2(x)
    }
}

/// Whether the original's `eax` is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eax {
    Checked,
    /// The original leaves junk there (a void function's last value),
    /// and no caller reads it.
    Junk,
}

/// The registers of a call.
#[derive(Debug, Clone, Copy, Default)]
pub struct Call {
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
}

/// One DLL in Unicorn and in Rust, on the same memory.
pub struct Twin {
    _guard: MutexGuard<'static, ()>,
    pub name: String,
    pub img: Image,
    pub ov: Overlay,
    port: PageMem,
    shared: Shared,
    /// Function words of `g_MagicImports` (by the player's `FFX.exe`).
    functions: Vec<bool>,
    heap: u32,
    /// Words the next comparison leaves out (the CRT's own state).
    ignore: Vec<u32>,
}

fn game_dir() -> Option<PathBuf> {
    std::env::var_os("FFX_GAME_DIR").map(PathBuf::from)
}

/// `g_MagicImports`' first `SLOTS` words in the player's `FFX.exe`:
/// whether each points into code.
fn table_kinds(dir: &std::path::Path) -> Vec<bool> {
    let exe = std::fs::read(dir.join("FFX.exe")).expect("FFX.exe");
    let pe = goblin::pe::PE::parse(&exe).expect("FFX.exe is a PE");
    let base = pe.image_base as u32;
    let text = pe
        .sections
        .iter()
        .find(|s| s.name().unwrap_or("") == ".text")
        .map(|s| {
            (
                base + s.virtual_address,
                base + s.virtual_address + s.virtual_size,
            )
        })
        .expect(".text");
    let data_rva = 0x00c6_4ce8 - base;
    let sec = pe
        .sections
        .iter()
        .find(|s| data_rva >= s.virtual_address && data_rva < s.virtual_address + s.virtual_size)
        .expect("g_MagicImports' section");
    let off = (data_rva - sec.virtual_address + sec.pointer_to_raw_data) as usize;
    (0..SLOTS as usize)
        .map(|i| {
            let w = u32::from_le_bytes(exe[off + 4 * i..off + 4 * i + 4].try_into().unwrap());
            w >= text.0 && w < text.1
        })
        .collect()
}

fn zero(m: &mut impl Mem, at: u32, len: u32) {
    let z = vec![0u8; 0x1_0000];
    let mut a = at;
    while a < at + len {
        let n = (at + len - a).min(0x1_0000);
        m.write_bytes(a, &z[..n as usize]);
        a += n;
    }
}

impl Twin {
    /// Loads `magic_NNNN.dll` (`number`), starts its CRT in Unicorn and
    /// copies the result to the port's side. `None` without
    /// `FFX_GAME_DIR`.
    pub fn load(number: u16) -> Option<Twin> {
        Twin::load_with(number, true)
    }

    /// [`Twin::load`] without running `DllMain`.
    pub fn load_unstarted(number: u16) -> Option<Twin> {
        Twin::load_with(number, false)
    }

    fn load_with(number: u16, started: bool) -> Option<Twin> {
        let dir = game_dir().or_else(|| {
            eprintln!("FFX_GAME_DIR is not set; skipping");
            None
        })?;
        let guard = lock();
        let _ = arena();
        let name = format!("magic_{number:04}.dll");
        let file = std::fs::read(dir.join("magicFiles/FFX").join(&name)).expect("the DLL");
        let pe = Pe::parse(&file).expect("a PE image");
        let base = pe.preferred_base;
        let mut a = ArenaMem;
        // a clean arena for the regions the tests use
        zero(&mut a, base, pe.span());
        zero(&mut a, TABLE, 4 * SLOTS);
        zero(&mut a, DATA, 0x1000 * SLOTS);
        zero(&mut a, HEAP, HEAP_LEN);
        zero(&mut a, STACK_TOP - STACK_LEN, STACK_LEN);
        zero(&mut a, SCRATCH, SCRATCH_LEN);
        for o in ffx_overlay_loaded() {
            o.unload();
        }
        let ov = Overlay::load(&name, &file, &pe, base).expect("load");
        let img = Image::new(&name, &file, base).expect("image");
        let functions = table_kinds(&dir);
        for (i, f) in (0u32..).zip(&functions) {
            let v = if *f {
                TRAPS + 16 * i
            } else {
                DATA + 0x1000 * i
            };
            a.write::<u32>(TABLE + 4 * i, v);
        }
        let mut t = Twin {
            _guard: guard,
            name,
            img,
            ov,
            port: PageMem::default(),
            shared: Shared {
                stubs: HashMap::new(),
                floats: HashMap::new(),
                rand: 1,
            },
            functions,
            heap: HEAP,
            ignore: Vec::new(),
        };
        // this thread's Unicorn engine (its thread block comes from the
        // heap) before any snapshot: GetEffectOverlayTable(1)
        let get = t
            .img
            .export("GetEffectOverlayTable")
            .expect("GetEffectOverlayTable");
        let _ = t.orig(get, Call::default(), &[1]);
        if started {
            // DllMain(base, DLL_PROCESS_ATTACH, 0) in Unicorn: the CRT's state
            let r = t.orig(t.ov.entry_point(), Call::default(), &[base, 1, 0]).0;
            assert_eq!(r.eax, 1, "{}: DllMain", t.name);
        }
        // the port's side starts from the same bytes
        t.sync_regions();
        Some(t)
    }

    /// `DllMain` attach then detach against the port, which keeps no CRT
    /// state: the words the original writes are left out of the
    /// comparison, after checking that none of them is an operand of the
    /// DLL's ported functions (`ported`, RVAs; their lengths from the
    /// signatures that bind there, or `internal` for functions the ports
    /// call directly).
    pub fn dll_main_check(&mut self, ported: &[u32], internal: &[(u32, u32)]) {
        let base = self.img.base;
        let mut fns: Vec<(u32, u32)> = internal.iter().map(|&(r, n)| (base + r, n)).collect();
        for &r in ported {
            let a = base + r;
            let i = ffx_magic::bind(&self.img, a)
                .unwrap_or_else(|| panic!("{}: nothing binds at {a:#010x}", self.name));
            fns.push((a, ffx_magic::PORTS[i].1.len));
        }
        let operands: Vec<u32> = fns
            .iter()
            .flat_map(|&(a, n)| self.img.operands(a, n))
            .collect();
        for reason in [1, 0] {
            let entry = self.ov.entry_point();
            let before = self.snapshot();
            let heap = self.heap;
            let _ = self.orig(entry, Call::default(), &[base, reason, 0]);
            let after = self.snapshot();
            let written: Vec<u32> = before
                .iter()
                .zip(&after)
                .flat_map(|((a, x), (_, y))| {
                    (0..x.len() / 4).filter_map(move |i| {
                        (x[4 * i..4 * i + 4] != y[4 * i..4 * i + 4]).then_some(a + 4 * i as u32)
                    })
                })
                .collect();
            for w in &written {
                assert!(
                    !operands.iter().any(|o| o & !3 == *w),
                    "{}: DllMain({reason}) writes {w:#010x}, which a ported function uses",
                    self.name
                );
            }
            eprintln!(
                "{}: DllMain({reason}) writes {} CRT words",
                self.name,
                written.len()
            );
            // back to the state before, then the real comparison
            for (a, b) in &before {
                ArenaMem.write_bytes(*a, b);
            }
            self.heap = heap;
            self.ignore = written;
            // (the original's CRT state stays behind for the next call)
            self.check_entry(entry, Call::default(), &[base, reason, 0], Eax::Checked);
        }
    }

    fn snapshot(&self) -> Vec<(u32, Vec<u8>)> {
        [(self.ov.base(), self.ov.span()), (HEAP, HEAP_LEN)]
            .iter()
            .map(|&(a, n)| {
                let mut b = vec![0u8; n as usize];
                ArenaMem.read_bytes(a, &mut b);
                (a, b)
            })
            .collect()
    }

    /// Copies every region the tests use from the arena to the port.
    fn sync_regions(&mut self) {
        for (at, len) in self.regions() {
            let mut b = vec![0u8; len as usize];
            ArenaMem.read_bytes(at, &mut b);
            self.port.write_bytes(at, &b);
        }
    }

    fn regions(&self) -> Vec<(u32, u32)> {
        vec![
            (self.ov.base(), self.ov.span()),
            (TABLE, 4 * SLOTS),
            (DATA, 0x1000 * SLOTS),
            (HEAP, HEAP_LEN),
            (STACK_TOP - STACK_LEN, STACK_LEN),
            (SCRATCH, SCRATCH_LEN),
        ]
    }

    /// The address of `rva` in the image.
    #[must_use]
    pub fn at(&self, rva: u32) -> u32 {
        self.img.base + rva
    }

    /// The zeroed page data word `slot` of `g_MagicImports` points at.
    #[must_use]
    pub fn data(&self, slot: u32) -> u32 {
        assert!(!self.functions[slot as usize], "slot {slot} is a function");
        DATA + 0x1000 * slot
    }

    /// Writes on both sides.
    pub fn write(&mut self, at: u32, b: &[u8]) {
        ArenaMem.write_bytes(at, b);
        self.port.write_bytes(at, b);
    }

    pub fn set_u32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    pub fn set_u16(&mut self, at: u32, v: u16) {
        self.write(at, &v.to_le_bytes());
    }

    pub fn set_u8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    /// Reads the original's side.
    #[must_use]
    pub fn u32(&self, at: u32) -> u32 {
        ArenaMem.read::<u32>(at)
    }

    /// Answers `slot` for both sides.
    pub fn stub(&mut self, slot: u32, f: impl Fn(&mut dyn StubMem, &[u32]) -> u32 + 'static) {
        self.shared.stubs.insert(slot, Box::new(f));
    }

    /// Makes `slot` return a float in ST0 on both sides (as the game's
    /// float functions do), besides the `eax` its [`Twin::stub`] gives
    /// (0 without one; that stub runs first).
    pub fn stub_st0(&mut self, slot: u32, f: impl Fn(&mut dyn StubMem, &[u32]) -> f64 + 'static) {
        self.shared.floats.insert(slot, Box::new(f));
    }

    fn orig(&mut self, at: u32, c: Call, args: &[u32]) -> (Regs, Vec<CallOut>) {
        let mut a = ArenaMem;
        let sp = ENTRY_ESP;
        a.write::<u32>(sp, SENTINEL);
        for (i, w) in (0u32..).zip(args) {
            a.write::<u32>(sp + 4 + 4 * i, *w);
        }
        let mut regs = Regs::at(sp);
        regs.eax = c.eax;
        regs.ecx = c.ecx;
        regs.edx = c.edx;
        let mut host = OrigHost {
            shared: &mut self.shared,
            calls: Vec::new(),
            heap: self.heap,
        };
        let r = ffx_overlay::call(&mut host, at, &regs)
            .unwrap_or_else(|e| panic!("{}: original at {at:#010x}: {e}", self.name));
        self.heap = host.heap;
        (r, host.calls)
    }

    /// Runs the original at `at` and `port` from the same registers and
    /// stack arguments; panics on any difference. Returns the port's
    /// result.
    pub fn check(
        &mut self,
        at: u32,
        c: Call,
        args: &[u32],
        eax: Eax,
        port: impl FnOnce(&mut PortGame, &Image, &Frame) -> Ret,
    ) -> Ret {
        // the port's stack gets the same words
        self.port.write::<u32>(ENTRY_ESP, SENTINEL);
        for (i, w) in (0u32..).zip(args) {
            self.port.write::<u32>(ENTRY_ESP + 4 + 4 * i, *w);
        }
        let (o, ocalls) = self.orig(at, c, args);
        let frame = Frame {
            at,
            esp: ENTRY_ESP,
            eax: c.eax,
            ecx: c.ecx,
            edx: c.edx,
        };
        let img = self.img.clone();
        let mut g = PortGame {
            mem: std::mem::take(&mut self.port),
            shared: std::mem::take(&mut self.shared),
            calls: Vec::new(),
        };
        let mut r = port(&mut g, &img, &frame);
        if let Some(t) = r.jump {
            // the original's tail jump reached the target with the
            // caller's stack as it was: the same words
            let args: Vec<u32> = (0..LOGGED as u32)
                .map(|i| g.mem.read::<u32>(ENTRY_ESP + 4 + 4 * i))
                .collect();
            let out = g.call(t, frame.ecx, &args);
            r.eax = out.eax;
            r.pops = 0;
        }
        let pcalls = std::mem::take(&mut g.calls);
        self.port = std::mem::take(&mut g.mem);
        self.shared = std::mem::take(&mut g.shared);
        self.compare_calls(at, &ocalls, &pcalls);
        let opops = o.esp - (ENTRY_ESP + 4);
        assert_eq!(r.pops, opops, "{}: {at:#010x} pops", self.name);
        if eax == Eax::Checked {
            assert_eq!(r.eax, o.eax, "{}: {at:#010x} eax", self.name);
        }
        let ost0 = (o.st_len > 0).then(|| ffx_oracle::f80_to_f64(&o.st[0]));
        match (ost0, r.st0) {
            (None, None) => {}
            (Some(a), Some(b)) => assert!(
                ffx_oracle::same_float(a, b),
                "{}: {at:#010x} st0 {a} vs {b}",
                self.name
            ),
            (a, b) => panic!("{}: {at:#010x} st0 {a:?} vs {b:?}", self.name),
        }
        self.compare_memory(at);
        r
    }

    fn compare_calls(&self, at: u32, o: &[CallOut], p: &[CallOut]) {
        let n = o.len().min(p.len());
        for (i, (a, b)) in o.iter().zip(p).enumerate() {
            assert_eq!(
                (a.target, a.slot),
                (b.target, b.slot),
                "{}: {at:#010x} call-out {i}: original {a:x?}, port {b:x?}",
                self.name
            );
            assert!(
                b.args.len() <= a.args.len() && a.args[..b.args.len()] == b.args[..],
                "{}: {at:#010x} call-out {i} (slot {:?}) arguments: original {:x?}, port {:x?}",
                self.name,
                a.slot,
                a.args,
                b.args
            );
        }
        assert_eq!(
            o.len(),
            p.len(),
            "{}: {at:#010x} call-outs: original {:x?}, port {:x?} (first {n} agree)",
            self.name,
            o,
            p
        );
    }

    fn compare_memory(&mut self, at: u32) {
        let mut pages: Vec<u32> = self.port.pages.keys().copied().collect();
        for (a, len) in self.regions() {
            pages.extend((a >> 12)..((a + len) >> 12));
        }
        pages.sort_unstable();
        pages.dedup();
        // the stack below the entry is the originals' own frames
        let below = (STACK_TOP - STACK_LEN) >> 12..=(ENTRY_ESP >> 12);
        let mut a = [0u8; 4096];
        let mut b = [0u8; 4096];
        for p in pages {
            ArenaMem.read_bytes(p << 12, &mut a);
            self.port.read_bytes(p << 12, &mut b);
            if below.contains(&p) {
                let keep = if p == ENTRY_ESP >> 12 {
                    (ENTRY_ESP & 0xfff) as usize
                } else {
                    4096
                };
                a[..keep].fill(0);
                b[..keep].fill(0);
            }
            for &w in &self.ignore {
                if w >> 12 == p {
                    let o = (w & 0xfff) as usize;
                    a[o..o + 4].fill(0);
                    b[o..o + 4].fill(0);
                }
            }
            if a != b {
                let i = (0..4096).find(|&i| a[i] != b[i]).unwrap_or(0);
                let w = |m: &[u8; 4096]| {
                    let j = i & !3;
                    u32::from_le_bytes(m[j..j + 4].try_into().unwrap())
                };
                panic!(
                    "{}: {at:#010x} memory differs at {:#010x}: original {:#010x}, port {:#010x}",
                    self.name,
                    (p << 12) + (i as u32 & !3),
                    w(&a),
                    w(&b)
                );
            }
        }
        // the next call starts equal again (the stack below too)
        self.ignore.clear();
        self.sync_regions();
    }

    /// Runs the bound port of `addr` against the original
    /// ([`ffx_magic::bind`] must find one).
    pub fn check_entry(&mut self, addr: u32, c: Call, args: &[u32], eax: Eax) -> Ret {
        let i = ffx_magic::bind(&self.img, addr)
            .unwrap_or_else(|| panic!("{}: nothing binds at {addr:#010x}", self.name));
        let f = ffx_magic::entry::<PortGame>(i).expect("an entry");
        self.check(addr, c, args, eax, f)
    }

    /// `InitMagicPRX(TABLE)` on both sides, checked.
    pub fn init(&mut self) {
        let f = self.img.export("InitMagicPRX").expect("InitMagicPRX");
        self.check_entry(f, Call::default(), &[TABLE], Eax::Checked);
    }
}

fn ffx_overlay_loaded() -> Vec<Overlay> {
    (0..0x100u32)
        .filter_map(|i| ffx_overlay::by_base(0x1000_0000 + i * 0x1_0000))
        .collect()
}
