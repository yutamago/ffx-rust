// The twin-oracle harness of tests/m5_render.rs (copied for the render-a
// list: recording stubs in a code cave, poisoned eax before callees,
// unwinding throw stubs); included by tests/m5p_render_a.rs.


/// The body of `FUN_006cbf20` (unreachable: `tools/reach.py --why`): the
/// recording stubs are written over it.
const CAVE: u32 = 0x006c_bf20;
const CAVE_LEN: u32 = 1800;
/// After the stubs: `mov eax, POISON; ret`, called before each original
/// callee so a callee leaving `eax` alone shows the poison.
const POISON_AT: u32 = CAVE + CAVE_LEN;
const POISON: u32 = 0xdead_beef;

/// Deterministic xorshift stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn chance(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }
    /// A word: small values, 0, 1, -1, the top bit and anything.
    fn word(&mut self) -> u32 {
        match self.below(6) {
            0 => 0,
            1 => 1,
            2 => self.below(8),
            3 => 0xffff_ffff,
            4 => 0x8000_0000 | self.below(4),
            _ => self.next(),
        }
    }
}

/// Where a stub's `eax` comes from.
#[derive(Clone, Copy)]
enum Ret {
    /// Left as it was.
    Keep,
    /// A constant.
    Val(u32),
    /// The next word of a table: the cell holds the table cursor.
    Seq(u32),
}

/// The cells the unwinding stubs read: the `esp` to unwind to and the
/// oracle's return sentinel, at the bottom of the stack (not compared).
fn cells(o: &Oracle) -> (u32, u32) {
    let s = o.stack_range().start;
    (s + 0x10, s + 0x14)
}

/// The oracle's default call top.
fn call_top(o: &Oracle) -> u32 {
    o.stack_range().end - 0x100
}

/// Sets the unwind target for a call of `conv` with `args` from the call
/// top `top`.
fn arm_unwind(o: &mut Oracle, top: u32, conv: Conv, args: &[u32]) {
    let n = match conv {
        Conv::Fastcall => args.len().saturating_sub(2),
        _ => args.len(),
    } as u32;
    let esp = match conv {
        Conv::Cdecl => top - 4 * n,
        _ => top,
    };
    let (cell, _) = cells(o);
    o.write(cell, &esp.to_le_bytes()).unwrap();
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
struct G<'a>(&'a mut Oracle, u32, u32);

impl Mem for G<'_> {
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

impl Host for G<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(c, false));
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        arm_unwind(self.0, self.2, conv(c), args);
        self.0
            .call(addr, conv(c), args)
            .unwrap_or_else(|e| panic!("{addr:#x}: {e}"))
            .eax
    }
    fn call_st0_f64(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f64 {
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        let r = self.0.call(addr, conv(c), args).unwrap();
        r.st0.unwrap_or_else(|| panic!("{addr:#x}: no ST0"))
    }
    fn call_wide(&mut self, addr: u32, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call_wide", addr, args.len(), host::WIDE_WORDS);
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        self.0.call(addr, Conv::Cdecl, args).unwrap().eax
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

/// What a function returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
    /// `ST0`, a float the original loads with `fld dword`.
    F32(f32),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `this`, up
    /// to 14 arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    snapshot: bool,
    /// Bytes the stubs replaced, put back by [`Twins::reset`].
    patched: Vec<(u32, Vec<u8>)>,
    /// A call top for the port's callees below the default one (to keep
    /// the words above it, such as an argument slot the port writes).
    b_top: Option<u32>,
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        a.set_max_instructions(2_000_000);
        b.set_max_instructions(2_000_000);
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
            snapshot,
            patched: Vec::new(),
            b_top: None,
        };
        t.log = t.zeros(0x1_0000);
        let at = t.rebase(POISON_AT);
        let mut code = vec![0xb8];
        code.extend(POISON.to_le_bytes());
        code.push(0xc3);
        t.write(at, &code);
        // the return sentinel: what a call leaves at its top - 4
        for o in [&mut t.a, &mut t.b] {
            o.call(at, Conv::Cdecl, &[]).unwrap();
            let top = o.stack_range().end - 0x100;
            let sentinel: u32 = Mem::read(o, top - 4);
            let (_, cell) = cells(o);
            o.write(cell, &sentinel.to_le_bytes()).unwrap();
        }
        t
    }

    fn image() -> Option<Self> {
        let exe = game_dir()?.join("FFX.exe");
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
            false,
        ))
    }

    fn snapshot() -> Option<Self> {
        let dir = game_dir()?;
        let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
            eprintln!("FFX_SNAPSHOT is not set; skipping");
            return None;
        };
        let exe = dir.join("FFX.exe");
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        Some(Self::new(
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            true,
        ))
    }

    /// Both kinds: the image, then the snapshot when one is set.
    fn each(mut f: impl FnMut(&mut Self)) {
        if let Some(mut t) = Self::image() {
            f(&mut t);
        }
        if let Some(mut t) = Self::snapshot() {
            f(&mut t);
        }
    }

    fn rebase(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    fn alloc(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), x);
        x
    }

    fn zeros(&mut self, n: usize) -> u32 {
        self.alloc(&vec![0; n])
    }

    fn words(&mut self, w: &[u32]) -> u32 {
        let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.alloc(&bytes)
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.b, at)
    }

    /// Writes `bytes` at `at`, to be put back by [`Twins::reset`].
    fn patch(&mut self, at: u32, bytes: &[u8]) {
        let mut old = vec![0; bytes.len()];
        Mem::read_bytes(&self.b, at, &mut old);
        self.patched.push((at, old));
        self.write(at, bytes);
    }

    /// Sets the global at `sym` until [`Twins::reset`].
    fn global(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.patch(at, &v.to_le_bytes());
    }

    fn reset(&mut self) {
        for (at, old) in std::mem::take(&mut self.patched).into_iter().rev() {
            self.write(at, &old);
        }
        self.cave_next = self.cave;
        let log = self.log;
        self.w32(log, 0);
    }

    /// A table of return values for [`Ret::Seq`]: the cursor cell.
    fn seq(&mut self, values: &[u32]) -> Ret {
        let table = self.words(values);
        Ret::Seq(self.words(&[table]))
    }

    /// Writes a recording stub into the cave: logs `tag`, `ecx` when
    /// `this`, and `n` stack arguments; returns `ret` and pops `pops`
    /// words, or with `unwind` returns to the test's call (a throw).
    fn stub_code(
        &mut self,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        ret: Ret,
        unwind: bool,
    ) -> u32 {
        assert!(n <= 14);
        let log = self.log;
        let keep = u8::from(matches!(ret, Ret::Keep)) * 4;
        let mut c = vec![];
        if keep != 0 {
            c.push(0x50); // push eax
        }
        c.push(0xa1);
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8 + keep]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        match ret {
            Ret::Val(v) => {
                c.push(0xb8);
                c.extend(v.to_le_bytes()); // mov eax, v
            }
            Ret::Keep => c.push(0x58), // pop eax
            Ret::Seq(cell) => {
                c.push(0xa1);
                c.extend(cell.to_le_bytes()); // mov eax, [cell]
                c.extend([0x8b, 0x00]); // mov eax, [eax]
                c.extend([0x83, 0x05]);
                c.extend(cell.to_le_bytes());
                c.push(0x04); // add dword [cell], 4
            }
        }
        if unwind {
            let (esp, sentinel) = cells(&self.a);
            for (op, v) in [
                (0xbb, 0x1111_1111_u32),
                (0xbe, 0x2222_2222),
                (0xbf, 0x3333_3333),
                (0xbd, 0x4444_4444),
            ] {
                c.push(op);
                c.extend(v.to_le_bytes()); // the saved registers' check values
            }
            c.extend([0x8b, 0x25]);
            c.extend(esp.to_le_bytes()); // mov esp, [cell]
            c.extend([0xff, 0x25]);
            c.extend(sentinel.to_le_bytes()); // jmp [sentinel]
        } else if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let addr = self.cave_next;
        if addr + c.len() as u32 > self.cave + CAVE_LEN {
            let x = self.a.alloc_code(&c).unwrap();
            assert_eq!(self.b.alloc_code(&c).unwrap(), x);
            return x;
        }
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// A stub over the original at `sym` (a `jmp` to it).
    fn stub_sym(&mut self, sym: u32, tag: u32, this: bool, n: u32, pops: u32, ret: Ret) -> u32 {
        let s = self.stub_code(tag, this, n, pops, ret, false);
        self.jump(sym, s);
        s
    }

    /// An unwinding stub over the thrower at `sym`.
    fn throw_sym(&mut self, sym: u32, tag: u32, n: u32) -> u32 {
        let s = self.stub_code(tag, false, n, 0, Ret::Val(0), true);
        self.jump(sym, s);
        s
    }

    /// An unwinding stub for the throwing import whose slot is `sym`.
    fn throw_import(&mut self, sym: u32, tag: u32, n: u32) {
        let s = self.stub_code(tag, false, n, 0, Ret::Val(0), true);
        let at = self.rebase(sym);
        self.patch(at, &s.to_le_bytes());
    }

    fn jump(&mut self, sym: u32, to: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    /// A stub for the import whose slot is `sym`, popping `pops` words.
    fn import_sym(&mut self, sym: u32, tag: u32, n: u32, pops: u32, ret: Ret) {
        let s = self.stub_code(tag, false, n, pops, ret, false);
        let at = self.rebase(sym);
        self.patch(at, &s.to_le_bytes());
    }

    /// An object whose vtable holds `slots` (stub addresses).
    fn object(&mut self, slots: &[u32], size: usize) -> u32 {
        let vt = self.words(slots);
        let obj = self.zeros(size.max(4));
        self.w32(obj, vt);
        obj
    }

    fn diff(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        if !d.is_empty() && std::env::var_os("M5_LOG").is_some() {
            for (side, o) in [("a", &self.a), ("b", &self.b)] {
                let n: u32 = Mem::read(o, self.log);
                for k in 0..n.min(64) {
                    let e: Vec<u32> = (0..16)
                        .map(|w| Mem::read::<u32>(o, self.log + 4 + 64 * k + 4 * w))
                        .collect();
                    eprintln!("{side} {k}: {e:x?}");
                }
            }
        }
        assert_eq!(d, [], "{what}: memory");
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// Runs the original at `sym` in `a` and `bridge` on `b`; compares the
    /// result as `bridge` says and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        cv: Conv,
        args: &[u32],
        bridge: impl FnOnce(&mut G) -> Out,
    ) {
        let addr = self.rebase(sym);
        let top = call_top(&self.a);
        arm_unwind(&mut self.a, top, cv, args);
        let ra = self
            .a
            .call(addr, cv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let poison = self.rebase(POISON_AT);
        let top = self.b_top.unwrap_or_else(|| call_top(&self.b));
        self.b.set_call_top(self.b_top);
        let out = bridge(&mut G(&mut self.b, poison, top));
        self.b.set_call_top(None);
        if let Out::F32(v) = out {
            let st0 = ra.st0.unwrap_or_else(|| panic!("{what}: no float in ST0"));
            assert!(
                ffx_oracle::same_float(st0, f64::from(v)),
                "{what}: st0 {st0} vs {v}"
            );
        } else {
            assert!(
                ra.st0_raw.is_none(),
                "{what}: the original leaves ST0 set ({:?})",
                ra.st0
            );
        }
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::None | Out::F32(_) => {}
        }
        self.diff(what);
    }

    /// The call log's entry count in `b`.
    fn calls(&self) -> u32 {
        self.r32(self.log)
    }
}

/// The argument slot of a cdecl call's first word from the oracle's call
/// top (what the original sees at `[ebp + 8]`).
fn first_slot(o: &Oracle, words: u32) -> u32 {
    o.stack_range().end - 0x100 - 4 * words
}

// ---- forwarders ----
