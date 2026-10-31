//! M5 stage 1, round "design-frames": the run-time frames, the varargs
//! forwarder and the wrapper-owned addresses (docs/porting.md, "Run-time
//! frames and varargs"; docs/hooking.md, "The hook table"), against the
//! original. Lists `phyre-render-b` (`ffx_prender::m5p_render_b_df`),
//! `phyre-scene` (`ffx_prender::m5p_scene_df`), `phyre-scripting`
//! (`ffx_pother_a::m5p_scripting_df`), `phyre-framework-input`
//! (`ffx_pother_b::m5p_framework_input_df`) and `system-a`
//! (`ffx_system::m5a_df`).
//!
//! None of this ran in a coverage run, so every case is synthetic memory
//! built for its branch, on the image alone and over a snapshot
//! (`FFX_SNAPSHOT`, run once with each of field.snap and battle.snap).
//! Needs `FFX_GAME_DIR`.
//!
//! The frames: each original carves its buffer with `_alloca_probe_16`
//! (16-byte aligned below its saved registers) or keeps locals at `ebp`
//! offsets, and hands their addresses to callees. The test computes where
//! they lie for the original's call (the oracle pushes the arguments from
//! `stack_range().end - 0x100`, then the return address; `ebp` is the
//! entry `esp` less 4), gives the port's bridge the same addresses in the
//! other oracle, and runs the port's own calls below them
//! (`Oracle::set_call_top`). Recording stubs log every call's words (the
//! frame pointers included), write what the callees write, and the
//! buffers are compared byte for byte after the call.
//!
//! The CRT (`strstr`, `strncpy`, `memcpy`, `sprintf`, ...) is the game's
//! own `MSVCR110.dll` in both oracles; the game's string helpers
//! (`Str_Utf8ToWide`, `Path_ToLowerAscii`) run as they are; the rest are
//! stubs.

#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::many_single_char_names,
    clippy::type_complexity,
    clippy::cast_precision_loss,
    dead_code
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};

/// Stands for `ecx` in a stub's writes (else a stack argument's index).
const ECX: u32 = 0xff;

/// Marks a [`Twins::check`] region as a UTF-16 string, compared up to its
/// NUL.
const WIDE: u32 = 0x8000_0000;

/// The bridge's view of an oracle.
struct G<'a>(&'a mut Oracle);

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
        self.0
            .call(addr, conv(c), args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }
    fn scratch(&mut self) -> u32 {
        panic!("no scratch in these ports")
    }
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

/// What the original returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// `eax`.
    U32(u32),
    /// `al` (the rest of `eax` is the caller's).
    Al(u8),
    /// A `double` left in `ST0`.
    F64(f64),
}

/// One stub write: `[arg + off] = [cell]` (`arg` a stack argument's index
/// or [`ECX`]; skipped when the argument is 0).
#[derive(Clone, Copy)]
struct Write {
    arg: u32,
    off: u32,
    cell: u32,
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `ecx`, up to
    /// 14 stack words).
    log: u32,
    /// The bytes each stub replaced (code `jmp`s and import slots), by
    /// address.
    saved: HashMap<u32, Vec<u8>>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        a.set_max_instructions(20_000_000);
        b.set_max_instructions(20_000_000);
        let mut t = Self {
            a,
            b,
            log: 0,
            saved: HashMap::new(),
        };
        t.log = t.zeros(0x10000);
        t
    }

    /// The image, then the snapshot when `FFX_SNAPSHOT` is set.
    fn all() -> Vec<(&'static str, Self)> {
        let Some(dir) = game_dir() else {
            return vec![];
        };
        let exe = dir.join("FFX.exe");
        let mut out = vec![(
            "image",
            Self::new(Oracle::load(&exe).unwrap(), Oracle::load(&exe).unwrap()),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Self::new(
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                ),
            ));
        } else {
            eprintln!("FFX_SNAPSHOT is not set; image only");
        }
        out
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

    /// A NUL-terminated string.
    fn cstr(&mut self, s: &[u8]) -> u32 {
        let mut v = s.to_vec();
        v.push(0);
        self.alloc(&v)
    }

    /// A word holding `v`.
    fn cell(&mut self, v: u32) -> u32 {
        self.alloc(&v.to_le_bytes())
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    /// A global of the image, kept to be put back by [`Twins::reset`].
    fn global(&mut self, sym: u32, bytes: &[u8]) {
        let at = self.rebase(sym);
        let old = Oracle::read(&mut self.a, at, bytes.len()).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, bytes);
    }

    /// Clears the log and puts back every stubbed original and global.
    fn reset(&mut self) {
        let log = self.log;
        self.w32(log, 0);
        let saved: Vec<(u32, Vec<u8>)> = self.saved.drain().collect();
        for (at, bytes) in saved {
            self.write(at, &bytes);
        }
        self.diff("reset");
    }

    /// The top of the stack the oracle's calls push from, with the same
    /// non-zero junk below it in both oracles (the frames start alike, and
    /// a byte the port leaves unwritten shows).
    fn top(&mut self) -> u32 {
        let junk: Vec<u8> = (0..0x4000u32)
            .map(|i| (i.wrapping_mul(29) ^ 0xa5) as u8 | 1)
            .collect();
        self.top_with(&junk)
    }

    /// [`Twins::top`] with zeros below it: for a never-written word the
    /// port reads as 0 (brief 2026-10-24, junk reads).
    fn top_zero(&mut self) -> u32 {
        self.top_with(&[0; 0x4000])
    }

    fn top_with(&mut self, fill: &[u8]) -> u32 {
        let top = self.a.stack_range().end - 0x100;
        assert_eq!(self.b.stack_range().end - 0x100, top);
        self.write(top - fill.len() as u32, fill);
        top
    }

    /// A recording stub in fresh code memory: logs `tag`, `ecx` when
    /// `this`, and `n` stack words; then the `writes`, and `copy` (`[arg]
    /// <- len` bytes from `src`, `len` the word at `len_cell`, skipped for
    /// a 0 argument); returns the word at `ret` (a cell), then, with
    /// `next`, puts `[next]` in it (a second call returns that); pops
    /// `pops` words.
    fn stub_code(
        &mut self,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        ret: u32,
        next: Option<u32>,
        writes: &[Write],
        copy: Option<(u32, u32, u32)>,
    ) -> u32 {
        assert!(n <= 14);
        let log = self.log;
        let mut c = vec![0xa1];
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
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        for w in writes {
            if w.arg == ECX {
                c.extend([0x89, 0xc8]); // mov eax, ecx
            } else {
                c.extend([0x8b, 0x44, 0x24, (4 + 4 * w.arg) as u8]); // mov eax, [esp+4+4k]
            }
            c.extend([0x85, 0xc0, 0x74, 12]); // test eax, eax; jz +12
            c.extend([0x8b, 0x15]);
            c.extend(w.cell.to_le_bytes()); // mov edx, [cell]
            c.extend([0x89, 0x90]);
            c.extend(w.off.to_le_bytes()); // mov [eax+off], edx
        }
        if let Some((arg, src, len_cell)) = copy {
            c.extend([0x56, 0x57, 0x51]); // push esi; push edi; push ecx
            c.extend([0x8b, 0x7c, 0x24, (16 + 4 * arg) as u8]); // mov edi, [esp+16+4k]
            c.extend([0x85, 0xff, 0x74, 14]); // test edi, edi; jz +14
            c.push(0xbe);
            c.extend(src.to_le_bytes()); // mov esi, src
            c.extend([0x8b, 0x0d]);
            c.extend(len_cell.to_le_bytes()); // mov ecx, [len]
            c.extend([0xfc, 0xf3, 0xa4]); // cld; rep movsb
            c.extend([0x59, 0x5f, 0x5e]); // pop ecx; pop edi; pop esi
        }
        c.push(0xa1);
        c.extend(ret.to_le_bytes()); // mov eax, [ret]
        if let Some(next) = next {
            c.extend([0x8b, 0x15]);
            c.extend(next.to_le_bytes()); // mov edx, [next]
            c.extend([0x89, 0x15]);
            c.extend(ret.to_le_bytes()); // mov [ret], edx
        }
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let at = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), at);
        at
    }

    /// A plain recording stub returning `ret`.
    fn simple(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
        let cell = self.cell(ret);
        self.stub_code(tag, this, n, pops, cell, None, &[], None)
    }

    /// `code` placed at the original `sym` (a `jmp` there).
    fn place(&mut self, sym: u32, code: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// [`Twins::simple`] at the original `sym`, logged with `sym` as its tag.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let code = self.simple(sym, this, n, pops, ret);
        self.place(sym, code);
    }

    /// `code` in the import slot `slot` (its `ffx.sym` address).
    fn slot(&mut self, slot: u32, code: u32) {
        self.global(slot, &code.to_le_bytes());
    }

    /// An object whose vtable holds `slots` (code addresses; 0 leaves a
    /// slot empty). Returns the object (0x40 bytes after its vtable word).
    fn object(&mut self, slots: &[u32]) -> u32 {
        let vt: Vec<u8> = slots.iter().flat_map(|s| s.to_le_bytes()).collect();
        let vt = self.alloc(&vt);
        let obj = self.zeros(0x44);
        self.w32(obj, vt);
        obj
    }

    fn diff(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{what}: memory differs at {} bytes, first {:x?}",
            d.len(),
            &d[..d.len().min(8)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// Runs the original at `sym` in `a` and `port` on `b` with its own
    /// calls below `low` (the lowest frame byte the port is handed; 0 for
    /// a port without a frame);
    /// compares the result, the `regions` (address, length: the frames
    /// callees saw) and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        cv: Conv,
        args: &[u32],
        low: u32,
        regions: &[(u32, u32)],
        port: impl FnOnce(&mut G) -> Out,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, cv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        if low != 0 {
            self.b.set_call_top(Some((low - 0x40) & !0xf));
        }
        let out = port(&mut G(&mut self.b));
        self.b.set_call_top(None);
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::Al(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::F64(v) => assert!(
                ra.st0_is(v),
                "{what}: ST0 {:?} against {v:?}",
                ra.st0.map(f64::to_bits)
            ),
        }
        if !matches!(out, Out::F64(_)) {
            assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        }
        for &(at, len) in regions {
            let len = if len & WIDE != 0 {
                // a UTF-16 string: up to its NUL (the rest of the buffer is
                // never written nor read)
                let raw = Oracle::read(&mut self.a, at, (len & !WIDE) as usize).unwrap();
                let units = raw
                    .chunks(2)
                    .position(|c| c == [0, 0])
                    .map_or(raw.len() / 2, |i| i + 1);
                2 * units as u32
            } else {
                len
            };
            let fa = Oracle::read(&mut self.a, at, len as usize).unwrap();
            let fb = Oracle::read(&mut self.b, at, len as usize).unwrap();
            assert_eq!(fa, fb, "{what}: the frame at {at:#x} the callees saw");
        }
        let calls = Mem::read::<u32>(&self.b, self.log);
        assert!(calls < 0x3f0, "{what}: log overflow");
        self.diff(what);
    }
}

// ---- Renderer_SetDisplayMode ----

#[test]
fn set_display_mode_matches_the_original() {
    use ffx_prender::m5p_render_b_df::bridge as db;
    for (label, mut t) in Twins::all() {
        let modes: Vec<(u32, u32)> = vec![
            (640, 480),
            (800, 600),
            (1280, 720),
            (1920, 1080),
            (1280, 720),
            (2560, 1440),
        ];
        for (count, want) in [
            (6u32, (1280u32, 720u32)),
            (6, (640, 480)),
            (6, (2560, 1440)),
            (6, (1024, 768)),
            (6, (1920, 600)),
            (1, (640, 480)),
            (1, (800, 600)),
            (0, (640, 480)),
        ] {
            let what = format!("{label}: {count} modes, want {want:?}");
            let mut table = Vec::new();
            for (i, &(w, h)) in modes.iter().take(count as usize).enumerate() {
                for v in [w, h, 60 + i as u32, 1, 0x1c, 0, 0] {
                    table.extend(v.to_le_bytes());
                }
            }
            table.resize(0x200, 0xcc);
            let table = t.alloc(&table);
            let n = t.cell(count);
            let len = t.cell(count * 0x1c);
            let zero = t.cell(0);
            let resize = t.simple(0x7038, false, 2, 2, 0);
            let chain = t.object(&[0; 14].iter().copied().chain([resize]).collect::<Vec<_>>());
            let list = t.stub_code(
                0x7020,
                false,
                5,
                5,
                zero,
                None,
                &[Write {
                    arg: 3,
                    off: 0,
                    cell: n,
                }],
                Some((4, table, len)),
            );
            let mut slots = vec![0u32; 9];
            slots[8] = list;
            let output = t.object(&slots);
            let out_cell = t.cell(output);
            let outputs = t.stub_code(
                0x701c,
                false,
                3,
                3,
                zero,
                None,
                &[Write {
                    arg: 2,
                    off: 0,
                    cell: out_cell,
                }],
                None,
            );
            let mut slots = vec![0u32; 8];
            slots[7] = outputs;
            let adapter = t.object(&slots);
            let ad_cell = t.cell(adapter);
            let adapters = t.stub_code(
                0x601c,
                false,
                3,
                3,
                zero,
                None,
                &[Write {
                    arg: 2,
                    off: 0,
                    cell: ad_cell,
                }],
                None,
            );
            slots[7] = adapters;
            let factory = t.object(&slots);
            let r = t.zeros(0x1400);
            t.w32(r + 0x133c, factory);
            t.w32(r + 0x74, chain);
            t.stub(db_printf(), false, 3, 0, 0);
            let top = t.top();
            let ebp = top - 12 - 4;
            let locals = ebp - 0x10;
            let buf = (ebp - 0x20 - count * 0x1c) & !0xf;
            t.check(
                &what,
                0x0058_df00,
                Conv::Thiscall(r),
                &[want.0, want.1],
                buf,
                &[(locals, 0xc), (buf, count * 0x1c)],
                |g| {
                    let c = db::display_mode_count(g, r, locals);
                    assert_eq!(db::display_modes_bytes(c), count * 0x1c);
                    Out::U32(db::set_display_mode(g, r, want.0, want.1, locals, c, buf))
                },
            );
            t.reset();
        }
    }
}

const fn db_printf() -> u32 {
    ffx_prender::m5p_render_b_df::PRINTF
}

// ---- PRenderer::flip ----

#[test]
fn flip_matches_the_original() {
    use ffx_prender::m5p_render_b_df::{self as df, bridge as db};
    for (label, mut t) in Twins::all() {
        let mut seed = 0x1234_5678u32;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for case in 0..96 {
            let what = format!("{label}: flip case {case}");
            let pick = |v: u32| [0x100u32, 0x200, 0x300][(v % 3) as usize];
            let (s1, l1, s2, l2) = (pick(next()), pick(next()), pick(next()), pick(next()));
            let end = t.cell(pick(next()));
            let step_ret = t.cell(pick(next()));
            let cmd = if case % 11 == 5 { 0 } else { t.zeros(0x48) };
            let cmd_cell = t.cell(cmd);
            let step = t.stub_code(
                df::STEP,
                true,
                3,
                3,
                step_ret,
                None,
                &[Write {
                    arg: 2,
                    off: 0,
                    cell: end,
                }],
                None,
            );
            t.place(df::STEP, step);
            let alloc = t.stub_code(df::ALLOCATE, true, 1, 1, cmd_cell, None, &[], None);
            t.place(df::ALLOCATE, alloc);
            t.stub(df::RENDER_THREAD, false, 0, 0, 0x00ab_c000);
            t.stub(df::FLIP_SLOT, true, 0, 0, 7);
            let kick = t.simple(0x7008, true, 0, 0, 3);
            let r = t.object(&[0, 0, kick]);
            let r2 = t.zeros(0x200);
            // the renderer: its vtable word, then the rest
            let vt = Mem::read::<u32>(&t.a, r);
            t.w32(r2, vt);
            t.w32(r2 + 4, u32::from(case % 13 == 3));
            t.w32(r2 + 0x24, s1);
            t.w32(r2 + 0x28, l1);
            t.w32(r2 + 0x38, s2);
            t.w32(r2 + 0x3c, l2);
            t.w32(r2 + 0x4c, if case % 2 == 0 { 0 } else { 0x00de_ad00 });
            t.w32(r2 + 0x13c, case);
            let top = t.top();
            let frame = top - 4 - 4 - 8;
            t.check(
                &what,
                0x005b_6af0,
                Conv::Thiscall(r2),
                &[],
                frame,
                &[(frame, 8)],
                |g| Out::U32(db::flip(g, r2, frame)),
            );
            t.reset();
        }
    }
}

// ---- 0x0044e9e0, the varargs forwarder ----

#[test]
fn member_value_matches_the_original() {
    use ffx_prender::m5p_scene_df::bridge as sb;
    for (label, mut t) in Twins::all() {
        for (case, (has_accessor, extra)) in [
            (true, vec![0x11u32, 0x22]),
            (true, vec![]),
            (true, vec![5, 6, 7, 8]),
            (false, vec![0x33]),
            (false, vec![]),
        ]
        .into_iter()
        .enumerate()
        {
            let what = format!("{label}: member value case {case}");
            let v0 = t.cell(0xaaaa_0000 + case as u32);
            let v1 = t.cell(0xbbbb_0000 + case as u32);
            let ret = t.cell(0x1357);
            let get = t.stub_code(
                0x7008,
                true,
                4,
                4,
                ret,
                None,
                &[
                    Write {
                        arg: 2,
                        off: 0,
                        cell: v0,
                    },
                    Write {
                        arg: 2,
                        off: 4,
                        cell: v1,
                    },
                ],
                None,
            );
            let accessor = t.object(&[0, 0, get]);
            let member = t.zeros(0x40);
            if has_accessor {
                t.w32(member + 0x14, accessor);
            }
            let object = t.zeros(0x20);
            let mut args = vec![member, object];
            args.extend(&extra);
            let top = t.top_zero();
            let entry = top - 4 * args.len() as u32 - 4;
            let ebp = entry - 4;
            let cell = ebp - 0x20;
            let value = (ebp - 0x1c + 15) & !15;
            // the first variable word: the caller's third
            let va = top - 4 * args.len() as u32 + 8;
            t.check(
                &what,
                0x0044_e9e0,
                Conv::Cdecl,
                &args,
                cell,
                &[(cell, 4), (value, 8)],
                |g| Out::U32(sb::member_value(g, member, object, va, cell, value)),
            );
            t.reset();
        }
    }
}

// ---- the scripting name builders ----

#[test]
fn load_module_file_matches_the_original() {
    use ffx_pother_a::m5p_scripting_df::{self as sd, bridge as sdb};
    for (label, mut t) in Twins::all() {
        for (case, (name, loader)) in [
            (&b"system"[..], true),
            (b"a", true),
            (b"", true),
            (b"scripts/field/main", true),
            (b"already.lua", true),
            (b"x.luac", true),
            (b"name", false),
            (b"other.lua", false),
        ]
        .into_iter()
        .enumerate()
        {
            let what = format!("{label}: load file {case}");
            let s = t.cstr(name);
            let code = t.simple(0x7100, true, 2, 0, 0x5000 + case as u32);
            t.global(
                sd::FILE_LOADER,
                &(if loader { code } else { 0 }).to_le_bytes(),
            );
            t.global(sd::FILE_LOADER_USER, &0x7777u32.to_le_bytes());
            let top = t.top();
            let ebp = top - 8 - 4;
            let len = name.len() as u32;
            let buf = (ebp - 0x10 - (len + 5)) & !0xf;
            let found = name.windows(4).any(|w| w == b".lua");
            let regions: Vec<(u32, u32)> = if found { vec![] } else { vec![(buf, len + 5)] };
            t.check(&what, 0x005d_57b0, Conv::Cdecl, &[s], buf, &regions, |g| {
                let size = sdb::load_file_size(g, s);
                assert_eq!(size.is_none(), found, "{what}: the extension");
                if let Some(size) = size {
                    assert_eq!(size, (len, len + 5), "{what}: the size");
                }
                Out::U32(sdb::load_file(g, s, size.map(|(l, b)| (buf, l, b))))
            });
            t.reset();
        }
    }
}

#[test]
fn module_name_from_path_matches_the_original() {
    use ffx_pother_a::m5p_scripting_df::{self as sd, bridge as sdb};
    for (label, mut t) in Twins::all() {
        for (case, path) in [
            &b"scripts/field/main.lua"[..],
            b"main.lua",
            b"/main.lua",
            b"a/b/c.lua",
            b"noext",
            b".lua",
            b"x/.lua",
            b"dir/sub.lua/more.lua",
            b"",
        ]
        .into_iter()
        .enumerate()
        {
            let what = format!("{label}: module name {case}");
            let p = t.cstr(path);
            let string = t.zeros(8);
            t.stub(sd::PSTRING_ASSIGN, true, 1, 1, 0x40 + case as u32);
            let top = t.top();
            let ebp = top - 12 - 4;
            let found = path.windows(4).position(|w| w == b".lua");
            let len = found.map_or(0, |end| {
                let mut start = end;
                while start > 0 && path[start] != b'/' {
                    start -= 1;
                }
                if start != 0 {
                    start += 1;
                }
                (end - start) as u32
            });
            let buf = (ebp - 0x10 - (len + 1)) & !0xf;
            let regions: Vec<(u32, u32)> = if found.is_some() {
                vec![(buf, len + 1)]
            } else {
                vec![]
            };
            t.check(
                &what,
                0x005d_6650,
                Conv::Cdecl,
                &[string, p],
                buf,
                &regions,
                |g| {
                    let size = sdb::module_name_size(g, p);
                    if let Some((_, l, b)) = size {
                        assert_eq!((l, b), (len, len + 1), "{what}: length and size");
                    }
                    Out::U32(sdb::module_name(
                        g,
                        string,
                        p,
                        size.map(|(st, l, _)| (buf, st, l)),
                    ))
                },
            );
            t.reset();
        }
    }
}

// ---- Framerate and the input-log command (the same buffer shape) ----

const COMMA_TEXTS: [&[u8]; 8] = [
    b"abc",
    b"a,b",
    b",x",
    b"",
    b"ab,",
    b"first,second,third",
    b",",
    b"a long name without a comma, 0123456789 0123456789",
];

#[test]
fn framerate_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_df::{self as fd, bridge as fdb};
    for (label, mut t) in Twins::all() {
        for (case, text) in COMMA_TEXTS.into_iter().enumerate() {
            for bits in [0x4270_0000u32, 0x3f80_0001, 0x7fc0_0001, 0xff80_0000, 0] {
                let what = format!("{label}: framerate {case} {bits:#x}");
                let s = t.cstr(text);
                let app = t.zeros(0x40);
                t.w32(app + 0x30, bits);
                t.w32(app + 0x34, 60 + case as u32);
                t.stub(fd::PRINTF, false, 6, 0, 0x99);
                let top = t.top();
                let ebp = top - 12 - 4;
                let bytes = text.len() as u32 + 1;
                let buf = (ebp - 0xc - bytes) & !0xf;
                t.check(
                    &what,
                    0x0062_35d0,
                    Conv::Cdecl,
                    &[app, s],
                    buf,
                    &[(buf, bytes)],
                    |g| {
                        assert_eq!(fdb::framerate_size(g, s), bytes);
                        Out::U32(fdb::framerate(g, app, s, buf, bytes))
                    },
                );
                t.reset();
            }
        }
    }
}

#[test]
fn read_input_log_command_matches_the_original() {
    use ffx_system::m5a_df::bridge as ad;
    for (label, mut t) in Twins::all() {
        for (case, text) in COMMA_TEXTS.into_iter().enumerate() {
            let what = format!("{label}: input log {case}");
            let s = t.cstr(text);
            let app = t.zeros(0x40);
            t.stub(ad::READ_INPUT_LOG, false, 2, 0, 0x300 + case as u32);
            let top = t.top();
            let ebp = top - 12 - 4;
            let bytes = text.len() as u32 + 1;
            let buf = (ebp - 0xc - bytes) & !0xf;
            t.check(
                &what,
                0x0062_4410,
                Conv::Cdecl,
                &[app, s],
                buf,
                &[(buf, bytes)],
                |g| {
                    assert_eq!(ad::name_size(g, s), bytes);
                    Out::U32(ad::read_input_log_cmd(g, app, s, buf, bytes))
                },
            );
            t.reset();
        }
    }
}

// ---- File_Probe and openFile (two buffers) ----

const PATHS: [&[u8]; 6] = [
    b"ffx_ps2\\ffx\\Master\\New_Uspc\\Battle\\Kernel\\item.bin",
    b"data/FFX_Data.vbf",
    b"x",
    b"Caf\xc3\xa9\\M\xc3\xbcsic.TXT",
    b"\\\\",
    b"a/b\\c/D.e",
];

#[test]
fn file_probe_matches_the_original() {
    use ffx_system::m5a_df::bridge as ad;
    for (label, mut t) in Twins::all() {
        let mut cases: Vec<(Option<&[u8]>, u32, u32)> = vec![(None, 0, 0), (Some(b""), 0, 0)];
        for p in PATHS {
            cases.push((Some(p), 1, 0));
            cases.push((Some(p), 0, 0xffff_ffff));
            cases.push((Some(p), 0, 0x88));
            cases.push((Some(p), 0x100, 0x88));
        }
        for (case, (path, exists, handle)) in cases.into_iter().enumerate() {
            let what = format!("{label}: probe {case}");
            let p = path.map_or(0, |p| t.cstr(p));
            t.stub(ad::FILE_SYS, false, 0, 0, 0x00f1_1e00);
            t.stub(ad::EXISTS, true, 1, 1, exists);
            let create = t.simple(0x6001, false, 7, 7, handle);
            t.slot(ad::CREATE_FILE_SLOT, create);
            let set_end = t.simple(0x6002, false, 1, 1, 1);
            t.slot(ad::SET_END_OF_FILE_SLOT, set_end);
            let close = t.simple(0x6003, false, 1, 1, 1);
            t.slot(ad::CLOSE_HANDLE_SLOT, close);
            let top = t.top();
            let ebp = top - 8 - 4;
            let len = path.map_or(0, |p| p.len() as u32);
            let wide = (ebp - 0x14 - (2 * len + 2)) & !0xf;
            let narrow = (wide - (len + 1)) & !0xf;
            let has = path.is_some_and(|p| !p.is_empty());
            let regions: Vec<(u32, u32)> = if has {
                vec![(wide, WIDE | (2 * len + 2)), (narrow, len + 1)]
            } else {
                vec![]
            };
            t.check(
                &what,
                0x0060_7e00,
                Conv::Cdecl,
                &[p],
                narrow,
                &regions,
                |g| {
                    let sizes = ad::probe_sizes(g, p);
                    assert_eq!(sizes.is_some(), has, "{what}: sizes");
                    Out::Al(ad::file_probe(
                        g,
                        p,
                        sizes.map(|(l, _, _)| (l, wide, narrow)),
                    ))
                },
            );
            t.reset();
        }
    }
}

#[test]
fn open_file_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_df::{self as fd, bridge as fdb};
    for (label, mut t) in Twins::all() {
        // (path, mode, archive entry, first CreateFileW, second)
        let mut cases: Vec<(Option<&[u8]>, u32, u32, u32, u32)> =
            vec![(None, 1, 0, 0, 0), (Some(b""), 0, 0, 0, 0)];
        let bad = 0xffff_ffff;
        for p in PATHS {
            cases.push((Some(p), 1, 0x00e0_0000, 0, 0));
            cases.push((Some(p), 1, 0, 0x44, 0));
            cases.push((Some(p), 0x101, 0, bad, 0));
            cases.push((Some(p), 0x100, 0, 0x44, 0));
            cases.push((Some(p), 0, 0, bad, 0x45));
            cases.push((Some(p), 0, 0, bad, bad));
        }
        for (case, (path, mode, entry, h1, h2)) in cases.into_iter().enumerate() {
            let what = format!("{label}: open {case}");
            let p = path.map_or(0, |p| t.cstr(p));
            let file = t.zeros(0x10);
            t.w32(file, 0x0bad_0bad);
            t.stub(fd::PRINTF, false, 3, 0, 0x99);
            t.stub(fd::FILE_SYS, false, 0, 0, 0x00f1_1e00);
            t.stub(fd::ARCHIVE_OPEN, true, 1, 1, entry);
            let first = t.cell(h1);
            let second = t.cell(h2);
            let create = t.stub_code(0x6001, false, 7, 7, first, Some(second), &[], None);
            t.slot(fd::CREATE_FILE_SLOT, create);
            let close = t.simple(0x6003, false, 1, 1, 1);
            t.slot(fd::CLOSE_HANDLE_SLOT, close);
            let top = t.top();
            let ebp = top - 24 - 4;
            let len = path.map_or(0, |p| p.len() as u32);
            let wide = (ebp - 0x14 - (2 * len + 2)) & !0xf;
            let narrow = (wide - (len + 1)) & !0xf;
            let has = path.is_some_and(|p| !p.is_empty());
            let regions: Vec<(u32, u32)> = if has {
                vec![(wide, WIDE | (2 * len + 2)), (narrow, len + 1)]
            } else {
                vec![]
            };
            t.check(
                &what,
                0x0060_7f40,
                Conv::Thiscall(file),
                &[p, mode, 0x31, 0x32, 0x33],
                narrow,
                &regions,
                |g| {
                    let sizes = fdb::open_file_sizes(g, p);
                    assert_eq!(sizes.is_some(), has, "{what}: sizes");
                    Out::U32(fdb::open_file(
                        g,
                        file,
                        p,
                        mode,
                        sizes.map(|(l, _, _)| (l, wide, narrow)),
                    ))
                },
            );
            t.reset();
        }
    }
}

// ---- PApplication::screenshot ----

#[test]
fn screenshot_matches_the_original() {
    use ffx_system::m5a_df::bridge as ad;
    for (label, mut t) in Twins::all() {
        // (app, guard, viewport error, w, h, malloc ok, read error,
        // open, write error)
        let mut cases = vec![(false, 0u32, 0u32, 4u32, 3u32, true, 0u32, 1u32, 0u32)];
        cases.push((true, 0, 0x11, 4, 3, true, 0, 1, 0));
        cases.push((true, 1, 0, 4, 3, false, 0, 1, 0));
        cases.push((true, 0, 0, 4, 3, true, 0x12, 1, 0));
        cases.push((true, 1, 0, 4, 3, true, 0, 0, 0));
        cases.push((true, 0, 0, 4, 3, true, 0, 1, 5));
        cases.push((true, 0, 0, 4, 3, true, 0, 1, 0));
        cases.push((true, 1, 0, 0, 3, true, 0, 1, 0));
        cases.push((true, 0, 0, 5, 0, true, 0, 1, 0));
        cases.push((true, 0x100, 0, 7, 5, true, 0, 0x100, 0));
        for (case, (has_app, guard, vp_err, w, h, malloc_ok, read_err, open, write_err)) in
            cases.into_iter().enumerate()
        {
            let text: &[u8] = if case % 2 == 0 {
                b"shot.bmp,1"
            } else {
                b"x.bmp"
            };
            let what = format!("{label}: screenshot {case}");
            let s = t.cstr(text);
            let pitch = w * 3 + 1;
            let size = pitch * h + 0x38;
            let block = if malloc_ok {
                t.zeros(size as usize + 0x40)
            } else {
                0
            };
            let pixels: Vec<u8> = (0..pitch * h).map(|i| (i * 7 + 3) as u8).collect();
            let pixels = t.alloc(&pixels);
            let flush = t.simple(0x7008, true, 0, 0, 1);
            let renderer_vt = t.object(&[0, 0, flush]);
            let app = if has_app { t.zeros(0x200) } else { 0 };
            if has_app {
                let vt = Mem::read::<u32>(&t.a, renderer_vt);
                t.w32(app + 0x124, vt);
            }
            t.global(ad::RENDER_INTERFACE_GUARD, &guard.to_le_bytes());
            t.stub(ad::PRINTF, false, 3, 0, 0x99);
            t.stub(ad::SYNC_RENDER, true, 0, 0, 2);
            t.stub(ad::LOCK_ENTER, true, 0, 0, 3);
            t.stub(ad::LOCK_LEAVE, true, 0, 0, 4);
            t.stub(ad::RENDER_INTERFACE_CTOR, true, 0, 0, 5);
            t.stub(ad::ATEXIT, false, 1, 0, 0);
            let (cw, ch, cp) = (t.cell(w), t.cell(h), t.cell(pitch));
            let vp_ret = t.cell(vp_err);
            let vp = t.stub_code(
                ad::VIEWPORT_SIZE,
                true,
                3,
                3,
                vp_ret,
                None,
                &[
                    Write {
                        arg: 0,
                        off: 0,
                        cell: cw,
                    },
                    Write {
                        arg: 1,
                        off: 0,
                        cell: ch,
                    },
                    Write {
                        arg: 2,
                        off: 0,
                        cell: cp,
                    },
                ],
                None,
            );
            t.place(ad::VIEWPORT_SIZE, vp);
            t.stub(ad::MALLOC, false, 2, 0, block);
            let read_ret = t.cell(read_err);
            let plen = t.cell(pitch * h);
            let read = t.stub_code(
                ad::READ_PIXELS,
                true,
                1,
                1,
                read_ret,
                None,
                &[],
                Some((0, pixels, plen)),
            );
            t.place(ad::READ_PIXELS, read);
            let err = t.cell(0x2b);
            let ctor_ret = t.cell(0);
            let ctor = t.stub_code(
                ad::WRITER_CTOR,
                true,
                1,
                1,
                ctor_ret,
                None,
                &[Write {
                    arg: ECX,
                    off: 4,
                    cell: err,
                }],
                None,
            );
            t.place(ad::WRITER_CTOR, ctor);
            t.stub(ad::WRITER_IS_OPEN, true, 0, 0, open);
            t.stub(ad::WRITER_WRITE, true, 2, 2, write_err);
            t.stub(ad::WRITER_CLOSE, true, 0, 0, 6);
            t.stub(ad::WRITER_DTOR, true, 0, 0, 7);
            t.stub(ad::FREE, false, 1, 0, 0);
            let top = t.top();
            let ebp = top - 12 - 4;
            let frame = ebp - 0x44;
            let bytes = text.len() as u32 + 1;
            let buf = (ebp - 0x54 - bytes) & !0xf;
            t.check(
                &what,
                0x0062_4da0,
                Conv::Cdecl,
                &[app, s],
                buf,
                &[(buf, bytes), (frame, 0x18), (frame + 0x24, 0x10)],
                |g| {
                    assert_eq!(ad::name_size(g, s), bytes);
                    Out::U32(ad::screenshot(g, app, s, buf, bytes, frame))
                },
            );
            t.reset();
        }
    }
}

// ---- PTimerWin32::GetTime ----

#[test]
fn get_time_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_df::{self as fd, bridge as fdb};
    for (label, mut t) in Twins::all() {
        let values: [(u32, u32); 8] = [
            (0, 0),
            (1, 0),
            (0xd53e_8000, 0x01db_3a7c),
            (0xffff_ffff, 0x7fff_ffff),
            (0, 0x8000_0000),
            (0x0000_0801, 0x8000_0000),
            (0xffff_ffff, 0xffff_ffff),
            (0x1234_5679, 0x0020_0000),
        ];
        for precise in [true, false] {
            for (case, &(lo, hi)) in values.iter().enumerate() {
                let what = format!("{label}: get time {precise} {case}");
                let (clo, chi) = (t.cell(lo), t.cell(hi));
                let ret = t.cell(1);
                let writes = [
                    Write {
                        arg: 0,
                        off: 0,
                        cell: clo,
                    },
                    Write {
                        arg: 0,
                        off: 4,
                        cell: chi,
                    },
                ];
                let code = t.stub_code(0x6100, false, 1, 1, ret, None, &writes, None);
                if precise {
                    t.global(fd::PRECISE_TIME, &code.to_le_bytes());
                } else {
                    t.global(fd::PRECISE_TIME, &0u32.to_le_bytes());
                    t.slot(fd::QPC_SLOT, code);
                    let k = 1.0f64 / (3_579_545.0 + case as f64);
                    t.global(fd::SECONDS_PER_TICK, &k.to_le_bytes());
                    t.global(fd::COUNTER, &[0; 8]);
                }
                let top = t.top();
                let ft = top - 4 - 4 - 8;
                t.check(&what, 0x0042_fd80, Conv::Cdecl, &[], ft, &[(ft, 8)], |g| {
                    Out::F64(fdb::get_time(g, ft))
                });
                t.reset();
            }
        }
    }
}

// ---- Input_KeyHeld ----

#[test]
fn key_held_matches_the_original() {
    use ffx_system::m5a_df::bridge as ad;
    for (label, mut t) in Twins::all() {
        let state: Vec<u8> = (0..0x120u32).map(|i| u8::from(i % 3 == 1)).collect();
        let kb = t.alloc(&[vec![0u8; 0x18], state].concat());
        let table = t.zeros(0x100);
        for d in [0u32, 2, 6] {
            t.w32(table + 0x80 + 4 * d, kb);
        }
        // a device past the 7 the original takes
        t.w32(table + 0x80 + 4 * 7, kb);
        let devices = t.zeros(0x300);
        t.w32(devices + 0x2b8, table);
        for device in [0u32, 1, 2, 6, 7, 8, 0xffff_ffff] {
            for binding in [
                0u32,
                1,
                27,
                28,
                29,
                30,
                31,
                100,
                283,
                284,
                0x1000,
                0xffff_ffff,
            ] {
                let what = format!("{label}: key {binding} device {device}");
                t.check(
                    &what,
                    0x0062_9d50,
                    Conv::Thiscall(devices),
                    &[binding, device],
                    t_low(),
                    &[],
                    |g| Out::U32(ad::key_held(g, devices, binding, device)),
                );
            }
        }
        t.reset();
    }
}

/// A call top for ports without a frame: well below the original's.
const fn t_low() -> u32 {
    0
}
