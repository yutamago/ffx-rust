//! The menu functions (`ffx_menu`) against the original, on a snapshot of
//! the running game: two oracles start alike, one runs `FFX.exe`'s code and
//! the other the Rust bridge; for every call the results and every written
//! byte must agree. Inputs are the game's own state (message tables, menu
//! globals) and random ones put in both (globals scrambled, windows,
//! selectors, command lists and scene data built on the emulated heap).
//! Needs `FFX_GAME_DIR` and `FFX_SNAPSHOT`; skipped otherwise.

#![allow(clippy::many_single_char_names, clippy::too_many_lines)] // long case lists

use std::path::PathBuf;

use ffx_bridge::{Mem, Value};
use ffx_menu::{
    gu::bridge as gu, msg::bridge as msg, scene::bridge as scene, scissor::bridge as scissor,
    tk::bridge as tk,
};
use ffx_oracle::{Conv, Oracle, Pages};

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }

    fn pick<T: Copy>(&mut self, v: &[T]) -> T {
        #[allow(clippy::cast_possible_truncation)]
        v[self.below(v.len() as u32) as usize]
    }

    /// One of `v`, or any value.
    fn pick_or_any(&mut self, v: &[u32]) -> u32 {
        let any = self.next();
        #[allow(clippy::cast_possible_truncation)]
        let k = self.below(v.len() as u32 + 1) as usize;
        v.get(k).copied().unwrap_or(any)
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        #[allow(clippy::cast_possible_truncation)]
        (0..n).map(|_| self.next() as u8).collect()
    }

    /// A signed value from a mix of edge cases and small and large ones.
    fn int(&mut self) -> u32 {
        match self.below(4) {
            0 => self.pick(&[
                0,
                1,
                2,
                0xffff_ffff,
                0x7fff_ffff,
                0x8000_0000,
                0xffff,
                0x1_0000,
            ]),
            1 => self.below(16),
            2 => self.below(0x400).wrapping_sub(0x200),
            _ => self.next(),
        }
    }
}

struct Twins {
    a: Oracle,
    b: Oracle,
    rng: Xorshift,
    calls: usize,
}

fn setup(seed: u32) -> Option<Twins> {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    Some(Twins {
        a: Oracle::from_snapshot(&exe, &pages).unwrap(),
        b: Oracle::from_snapshot(&exe, &pages).unwrap(),
        rng: Xorshift(seed),
        calls: 0,
    })
}

/// Like [`setup`], from the snapshot when `FFX_SNAPSHOT` is set and from
/// the bare image otherwise (synthetic state only: `.bss` is zero and the
/// game never ran).
fn setup_any(seed: u32) -> Option<Twins> {
    if std::env::var_os("FFX_SNAPSHOT").is_some() {
        return setup(seed);
    }
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    Some(Twins {
        a: Oracle::load(&exe).unwrap(),
        b: Oracle::load(&exe).unwrap(),
        rng: Xorshift(seed),
        calls: 0,
    })
}

/// `fstp st0; ret`, placed in code space no menu test reaches (the body of
/// `pppFpLoop`, past the x87 stand-ins).
const FPU_POP: u32 = 0x0072_9ab0 + 0x70;

impl Twins {
    /// Empties the original side's x87 stack (results earlier calls left),
    /// so the next call's `ST0` is its own.
    fn drain_x87(&mut self) {
        let at = self.a.rebase(FPU_POP);
        if Mem::read::<[u8; 3]>(&self.a, at) != [0xdd, 0xd8, 0xc3] {
            self.a.write(at, &[0xdd, 0xd8, 0xc3]).unwrap();
            self.b
                .write(self.b.rebase(FPU_POP), &[0xdd, 0xd8, 0xc3])
                .unwrap();
        }
        for _ in 0..8 {
            if self.a.call(at, Conv::Cdecl, &[]).unwrap().st0.is_none() {
                break;
            }
        }
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    fn sym(&self, sym: u32) -> u32 {
        self.b.rebase(sym)
    }

    /// The same bytes at the same new address in both.
    fn alloc(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), x);
        x
    }

    fn alloc_random(&mut self, n: usize) -> u32 {
        let b = self.rng.bytes(n);
        self.alloc(&b)
    }

    fn put<T: Value>(&mut self, at: u32, v: T) {
        Mem::write(&mut self.a, at, v);
        Mem::write(&mut self.b, at, v);
    }

    fn put_bytes(&mut self, at: u32, v: &[u8]) {
        self.a.write_bytes(at, v);
        self.b.write_bytes(at, v);
    }

    /// Random bytes over `len` bytes of a global, in both.
    fn scramble(&mut self, sym: u32, len: usize) {
        let at = self.sym(sym);
        let v = self.rng.bytes(len);
        self.put_bytes(at, &v);
    }

    /// The original on `a` and `rust` on `b`; results (under `mask`) and
    /// memory must agree. After a clean diff both memories are equal, so
    /// the dirty pages start over.
    fn twin(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        mask: u64,
        rust: impl FnOnce(&mut Oracle) -> u64,
    ) {
        self.drain_x87();
        let r = self
            .a
            .call(self.a.rebase(sym), Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{what} {args:x?}: {e}"));
        // an integer or void port of a function that leaves a float in ST0
        // drops the caller's result (a void Ch_CheckRadian hid every model)
        assert!(
            r.st0.is_none(),
            "{what} {args:x?}: the original returns {:?} in ST0; the port must return a float",
            r.st0
        );
        let ra = u64::from(r.eax) | (u64::from(r.edx) << 32);
        let rb = rust(&mut self.b);
        assert_eq!(ra & mask, rb & mask, "{what} {args:x?}: result");
        assert_eq!(
            self.a.diff(&mut self.b).unwrap(),
            [],
            "{what} {args:x?}: memory"
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
        self.calls += 1;
    }
}

impl Twins {
    /// Like [`Twins::twin`] for an original returning a float in `ST0`:
    /// the same bits (NaNs included) and the same memory.
    fn twin_f(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        rust: impl FnOnce(&mut Oracle) -> f32,
    ) {
        self.drain_x87();
        let r = self
            .a
            .call(self.a.rebase(sym), Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{what} {args:x?}: {e}"));
        let want = r
            .st0
            .unwrap_or_else(|| panic!("{what} {args:x?}: no float in ST0"));
        let got = f64::from(rust(&mut self.b));
        assert_eq!(
            want.to_bits(),
            got.to_bits(),
            "{what} {args:x?}: {want} vs {got}"
        );
        assert_eq!(
            self.a.diff(&mut self.b).unwrap(),
            [],
            "{what} {args:x?}: memory"
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
        self.calls += 1;
    }
}

const VOID: u64 = 0;
const U16: u64 = 0xffff;
const U32: u64 = 0xffff_ffff;
const U64: u64 = u64::MAX;

fn w(v: u32) -> u64 {
    u64::from(v)
}

#[allow(clippy::cast_sign_loss)]
fn s(v: i32) -> u64 {
    u64::from(v as u32)
}

#[allow(clippy::cast_possible_wrap)]
fn i(v: u32) -> i32 {
    v as i32
}

/// A window with `+0x90` controls (some ids shared, some above 16 bits'
/// reach) at `+0x94`.
fn window(t: &mut Twins) -> u32 {
    let n = t.rng.pick(&[-1i32, 0, 1, 2, 3, 6]);
    let mut controls = t.rng.bytes(6 * 0x44);
    for c in 0..6 {
        let id = t.rng.pick(&[1u16, 2, 3, 0xffff]);
        controls[c * 0x44 + 0x2e..c * 0x44 + 0x30].copy_from_slice(&id.to_le_bytes());
    }
    let controls = t.alloc(&controls);
    let win = t.alloc_random(0xa0);
    t.put(win + 0x90, n);
    t.put(win + 0x94, controls);
    win
}

/// A selector with 16 names (some null, from a small vocabulary) and values.
fn selector(t: &mut Twins) -> u32 {
    let words: [&[u8]; 5] = [b"ITEM\0", b"ITEMS\0", b"MAGIC\0", b"\0", b"ITEM\0"];
    let mut names = Vec::new();
    for _ in 0..16 {
        let n = if t.rng.below(5) == 0 {
            0
        } else {
            let wd = t.rng.pick(&words);
            t.alloc(wd)
        };
        names.extend(n.to_le_bytes());
    }
    let names = t.alloc(&names);
    let values = t.alloc_random(16 * 4);
    let sel = t.alloc_random(0x30);
    let v = t.rng.pick(&[0i16, 1, 5, 15, -1]);
    t.put(sel + 2, v);
    let v = t.rng.pick(&[-1i16, 0, 1, 6, 16]);
    t.put(sel + 4, v);
    t.put(sel + 8, names);
    t.put(sel + 0xc, values);
    sel
}

#[test]
fn gu_matches_the_original() {
    let Some(mut t) = setup_any(0x2545_f491) else {
        return;
    };
    for _ in 0..60 {
        let args = [t.rng.int(), t.rng.int(), t.rng.int()];
        t.twin("Gu_PrintSetColor", 0x0084_8740, &args, VOID, |o| {
            gu::gu_print_set_color(o, args[0], args[1], args[2]);
            0
        });
        t.scramble(gu::PRINT_BASE_X.sym, 16);
        let x = t.rng.int();
        t.twin("Gu_PrintSetX", 0x0084_8820, &[x], VOID, |o| {
            gu::gu_print_set_x(o, i(x));
            0
        });
        t.twin("Gu_PrintSetY", 0x0084_8860, &[x], VOID, |o| {
            gu::gu_print_set_y(o, i(x));
            0
        });
        t.twin("Gu_SetHScale", 0x0084_8b30, &[x], VOID, |o| {
            gu::gu_set_h_scale(o, i(x));
            0
        });
        t.scramble(gu::HIDDEN.sym, 4);
        t.twin("Gu_Hide", 0x0084_9780, &[x], U32, |o| w(gu::gu_hide(o, x)));
        t.twin("Gu_MouseSetCapture", 0x0084_9b60, &args[..2], VOID, |o| {
            gu::gu_mouse_set_capture(o, args[0], args[1]);
            0
        });
    }
    // packets
    for _ in 0..300 {
        let buf = t.alloc_random(0x200);
        t.put(t.sym(gu::PACKET.sym), buf);
        let v = t.rng.below(16);
        t.put(t.sym(gu::PACKET_COUNT.sym), v);
        let scale = t.rng.pick_or_any(&[0x100u32, 0x80, 0x1c0, 0xffff_ff00]);
        t.put(t.sym(gu::H_SCALE.sym), scale);
        let edge = [
            0u32,
            0x1fd,
            0x1fe,
            0x19e,
            0x19f,
            0xffff_ffff,
            0x8000_0000,
            0x7fff_ffff,
        ];
        let mut v = [0u32; 4];
        for x in &mut v {
            *x = if t.rng.below(3) == 0 {
                t.rng.pick(&edge)
            } else {
                t.rng.int()
            };
        }
        t.twin("Gu_SetScissor", 0x0084_8b40, &v, VOID, |o| {
            gu::gu_set_scissor(o, i(v[0]), i(v[1]), i(v[2]), i(v[3]));
            0
        });
        t.twin("Gu_SetTexFlush", 0x0084_8d10, &[], VOID, |o| {
            gu::gu_set_tex_flush(o);
            0
        });
    }
    // windows and their controls
    for _ in 0..300 {
        let win = window(&mut t);
        let id = t.rng.pick(&[0u32, 1, 2, 3, 0xffff, 0x1_0001, 0x1_0002]);
        let on = t.rng.pick_or_any(&[0u32, 1, 2, 3]);
        t.twin(
            "Gu_ButtonGetToggleValue",
            0x0084_9330,
            &[win, id],
            U32,
            |o| w(gu::gu_button_get_toggle_value(o, win, id)),
        );
        let f: [ControlSetter; 4] = [
            ("Gu_ButtonSetToggle", 0x0084_9380, gu::gu_button_set_toggle),
            (
                "Gu_ButtonSetToggleValue",
                0x0084_93e0,
                gu::gu_button_set_toggle_value,
            ),
            (
                "Gu_WindowControlSetDisable",
                0x0084_a620,
                gu::gu_window_control_set_disable,
            ),
            (
                "Gu_WindowControlSetHide",
                0x0084_a670,
                gu::gu_window_control_set_hide,
            ),
        ];
        let (name, sym, rust) = t.rng.pick(&f);
        t.twin(name, sym, &[win, id, on], VOID, |o| {
            rust(o, win, id, on);
            0
        });
        t.scramble(gu::MOUSE_X.sym, 8);
        t.twin("Gu_MouseGetWindowY", 0x0084_9aa0, &[win], U32, |o| {
            s(gu::gu_mouse_get_window_y(o, win))
        });
        t.twin("Gu_MouseGetWindowX", 0x0084_9ac0, &[win], U32, |o| {
            s(gu::gu_mouse_get_window_x(o, win))
        });
        t.twin("Gu_WindowGetArg", 0x0084_aec0, &[win], U32, |o| {
            w(gu::gu_window_get_arg(o, win))
        });
        t.twin("Gu_WindowGetWidth", 0x0084_afa0, &[win], U32, |o| {
            s(gu::gu_window_get_width(o, win))
        });
        t.twin("Gu_WindowGetX", 0x0084_afc0, &[win], U32, |o| {
            s(gu::gu_window_get_x(o, win))
        });
        t.twin("Gu_WindowSetArg", 0x0084_b560, &[win, on], VOID, |o| {
            gu::gu_window_set_arg(o, win, on);
            0
        });
        t.twin("Gu_WindowSetParent", 0x0084_b660, &[win, on], VOID, |o| {
            gu::gu_window_set_parent(o, win, on);
            0
        });
        let h = t.rng.int();
        t.twin("Gu_WindowSetClientH", 0x0084_b580, &[win, h], VOID, |o| {
            gu::gu_window_set_client_h(o, win, i(h));
            0
        });
        t.scramble(gu::DRAG_WINDOW.sym, 4);
        let drag = if t.rng.below(2) == 0 {
            gu::DRAG_WINDOW.get(&t.b)
        } else {
            win
        };
        t.twin("Gu_WindowIsDrag", 0x0084_b200, &[drag], U32, |o| {
            w(u32::from(gu::gu_window_is_drag(o, drag)))
        });
    }
    // opening windows: a list of up to four, the window in it or not
    for _ in 0..200 {
        let n = t.rng.below(5) as usize;
        let wins: Vec<u32> = (0..=n).map(|_| t.alloc_random(0xa0)).collect();
        for k in 0..n {
            let next = if k + 1 < n { wins[k + 1] } else { 0 };
            t.put(wins[k], next);
        }
        let head = if n == 0 { 0 } else { wins[0] };
        t.put(t.sym(gu::WINDOW_LIST.sym), head);
        t.scramble(gu::MOUSE_X.sym, 8);
        let win = wins[t.rng.below(u32::try_from(n).unwrap() + 1) as usize];
        t.twin("Gu_WindowOpen", 0x0084_b250, &[win], VOID, |o| {
            gu::gu_window_open(o, win);
            0
        });
    }
    // selectors
    for _ in 0..300 {
        let sel = selector(&mut t);
        let x = t.rng.int();
        t.twin("Gu_SelectorGetItemName", 0x0084_9d30, &[sel], U32, |o| {
            w(gu::gu_selector_get_item_name(o, sel))
        });
        t.twin(
            "Gu_SelectorSetCallback",
            0x0084_9fc0,
            &[sel, x],
            VOID,
            |o| {
                gu::gu_selector_set_callback(o, sel, x);
                0
            },
        );
        t.twin(
            "Gu_SelectorSetMouseFukki",
            0x0084_a0d0,
            &[sel, x],
            VOID,
            |o| {
                gu::gu_selector_set_mouse_fukki(o, sel, x);
                0
            },
        );
        let k = t.rng.below(16);
        // a real string: Gu_SelectorSetSelectSTR reads the names below
        let name = if t.rng.below(4) == 0 {
            0
        } else {
            t.alloc(b"MAGIC\0")
        };
        let value = t.rng.next();
        t.twin(
            "Gu_SelectorSetItem",
            0x0084_9fd0,
            &[sel, k, name, value],
            VOID,
            |o| {
                gu::gu_selector_set_item(o, sel, i(k), name, value);
                0
            },
        );
        let k = t
            .rng
            .pick(&[0u32, 1, 5, 6, 7, 16, 17, 0xffff_ffff, 0x8000_0000, x]);
        t.twin("Gu_SelectorSetSelect", 0x0084_a0e0, &[sel, k], VOID, |o| {
            gu::gu_selector_set_select(o, sel, i(k));
            0
        });
        let query: [&[u8]; 4] = [b"ITEM\0", b"MAGIC\0", b"\0", b"NONE\0"];
        let q = if t.rng.below(6) == 0 {
            0
        } else {
            let wd = t.rng.pick(&query);
            t.alloc(wd)
        };
        t.twin(
            "Gu_SelectorSetSelectSTR",
            0x0084_a110,
            &[sel, q],
            VOID,
            |o| {
                gu::gu_selector_set_select_str(o, sel, q);
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

/// The context's table at `+field`, its entry count (the first text offset
/// / 8), when there is one.
fn table(t: &Twins, field: u32) -> Option<(u32, u32)> {
    let ctx = msg::SCRIPT_CONTEXT.get(&t.b);
    let table = Mem::read::<u32>(&t.b, ctx + field);
    (table != 0).then(|| (table, u32::from(Mem::read::<u16>(&t.b, table)) / 8))
}

fn msg_cases(t: &mut Twins, n: u32, table: u32) {
    for k in 0..n {
        t.twin("Msg_Flags", 0x0086_bf10, &[k], U32, |o| {
            w(u32::from(msg::msg_flags(o, i(k))))
        });
        t.twin("Msg_TextAlt", 0x0086_ba90, &[k], U32, |o| {
            w(msg::msg_text_alt(o, i(k)))
        });
        t.twin("Msg_Text1", 0x0086_b170, &[k], U32, |o| {
            w(msg::msg_text1(o, i(k)))
        });
        for (alt, second) in [(0, 0), (1, 0), (0, 1), (5, 7)] {
            t.twin("Msg_Get", 0x0086_bf70, &[k, alt, second], U32, |o| {
                w(msg::msg_get(o, i(k), i(alt), i(second)))
            });
        }
        if table != 0 {
            t.twin("Msg_TableText", 0x0086_bed0, &[table, k], U32, |o| {
                w(msg::msg_table_text(o, table, i(k)))
            });
            t.twin("Msg_TableTextAlt", 0x0086_ba60, &[table, k], U32, |o| {
                w(msg::msg_table_text_alt(o, table, i(k)))
            });
        }
    }
}

/// A fake message table of `n` entries, a fifth of the offsets 0.
fn fake_table(t: &mut Twins, n: u32) -> u32 {
    let mut v = t.rng.bytes(n as usize * 8);
    for e in v.chunks_mut(8) {
        if t.rng.below(5) == 0 {
            e[0..2].fill(0);
        }
        if t.rng.below(5) == 0 {
            e[4..6].fill(0);
        }
    }
    t.alloc(&v)
}

#[test]
fn msg_matches_the_original() {
    let Some(mut t) = setup(0x1234_5678) else {
        return;
    };
    // the game's own tables
    let ctx = msg::SCRIPT_CONTEXT.get(&t.b);
    assert_ne!(ctx, 0, "the snapshot has a script context");
    match table(&t, 0x34) {
        Some((table, n)) => {
            eprintln!("event table at {table:#x}: {n} messages");
            msg_cases(&mut t, n.min(400), table);
        }
        None => eprintln!("no event message table"),
    }
    // synthetic tables, and none
    for round in 0..40 {
        let n = 1 + t.rng.below(40);
        let event = fake_table(&mut t, n);
        let second = if round % 3 == 0 {
            0
        } else {
            fake_table(&mut t, n)
        };
        let ebp = fake_table(&mut t, n);
        let event = if round % 7 == 0 { 0 } else { event };
        t.put(ctx + 0x34, event);
        t.put(ctx + 0x38, second);
        t.put(t.sym(msg::EBP_MESSAGES.sym), ebp);
        msg_cases(&mut t, n, if event == 0 { ebp } else { event });
    }
    // choices
    for _ in 0..2000 {
        let win = t.alloc_random(0x2c);
        let flags = match t.rng.below(4) {
            0 => 0u16,
            1 => u16::try_from(t.rng.below(8)).unwrap() << 8,
            2 => u16::try_from(t.rng.below(16)).unwrap(),
            _ => u16::try_from(t.rng.next() & 0xffff).unwrap(),
        };
        t.put(win + 0x16, flags);
        let mode = t.rng.pick_or_any(&[2u32, 0x42, 0x40, 0, 0xffff_ffff]);
        t.twin("Msg_ClampChoices", 0x0087_1210, &[win, mode], U32, |o| {
            w(msg::msg_clamp_choices(o, win, mode))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// A scene data block with these parameter and scene names, and its
/// records' handles.
fn scene_block(t: &mut Twins, params: &[&[u8]], scenes: &[&[u8]]) -> (u32, Vec<u32>, Vec<u32>) {
    let mut v = vec![0u8; 0x80];
    let mut strings = Vec::new();
    let name_at = |s: &[u8], strings: &mut Vec<u8>| {
        let o = strings.len();
        strings.extend_from_slice(s);
        strings.push(0);
        o
    };
    let mut fixups = Vec::new(); // (where in v, offset into strings, high byte)
    let (mut param_handles, mut scene_handles) = (Vec::new(), Vec::new());
    for h in 0..16u32 {
        let ps: Vec<&&[u8]> = params
            .iter()
            .filter(|p| ffx_menu::scene::bucket(p) == h)
            .collect();
        if !ps.is_empty() {
            let at = u32::try_from(v.len()).unwrap();
            v[h as usize * 4..h as usize * 4 + 4].copy_from_slice(&at.to_le_bytes());
            v.extend(u32::try_from(ps.len()).unwrap().to_le_bytes());
            for p in ps {
                let extra = t.rng.below(3);
                param_handles.push(u32::try_from(v.len()).unwrap());
                fixups.push((v.len(), name_at(p, &mut strings), extra));
                v.extend([0; 4]);
                v.extend(t.rng.bytes(extra as usize * 4));
            }
        }
        let ss: Vec<&&[u8]> = scenes
            .iter()
            .filter(|p| ffx_menu::scene::bucket(p) == h)
            .collect();
        if !ss.is_empty() {
            let at = u32::try_from(v.len()).unwrap();
            let slot = 0x40 + h as usize * 4;
            v[slot..slot + 4].copy_from_slice(&at.to_le_bytes());
            v.extend(u32::try_from(ss.len()).unwrap().to_le_bytes());
            for sname in ss {
                scene_handles.push(u32::try_from(v.len()).unwrap());
                fixups.push((v.len(), name_at(sname, &mut strings), 0));
                v.extend([0; 4]);
                let flags = t.rng.next() & 0x4000_ffff;
                v.extend(flags.to_le_bytes());
                v.extend(t.rng.bytes(4));
            }
        }
    }
    let base_strings = v.len();
    v.extend(&strings);
    for (at, o, extra) in fixups {
        let word = u32::try_from(base_strings + o).unwrap() | (extra << 24);
        v[at..at + 4].copy_from_slice(&word.to_le_bytes());
    }
    (t.alloc(&v), param_handles, scene_handles)
}

#[test]
fn scene_data_matches_the_original() {
    let Some(mut t) = setup_any(0x0bad_cafe) else {
        return;
    };
    let loaded: Vec<u32> = (0..8)
        .map(|k| scene::SCENE_DATA.ptr(&t.b).at(k).get(&t.b))
        .collect();
    eprintln!("scene data in the snapshot: {loaded:x?}");
    let params: [&[u8]; 9] = [
        b"cursor", b"window", b"item", b"magic", b"x", b"y", b"pa", b"ap", b"",
    ];
    let scenes: [&[u8]; 6] = [b"main", b"status", b"equip", b"ability", b"ma", b"am"];
    let absent: [&[u8]; 4] = [b"cursors", b"wind", b"q", b"mainx"];
    for round in 0..30 {
        let (block, ph, sh) = scene_block(&mut t, &params, &scenes);
        let slot = t.rng.below(8);
        let data = if round % 5 == 0 { 0 } else { block };
        t.twin(
            "TkMenuSetSceneDataAdrs",
            0x00a7_c7f0,
            &[slot, data],
            VOID,
            |o| {
                scene::tk_menu_set_scene_data_adrs(o, i(slot), data);
                0
            },
        );
        for name in params.iter().chain(&scenes).chain(&absent) {
            let q = t.alloc(&[name, &[0][..]].concat());
            for k in [i(slot), -1, 9, i(slot) + 1] {
                #[allow(clippy::cast_sign_loss)]
                let k = k as u32;
                t.twin("TkMenuGetParamHandle", 0x00a7_b7d0, &[k, q], U32, |o| {
                    w(scene::tk_menu_get_param_handle(o, i(k), q))
                });
                t.twin("TkMenuGetSceneHandle", 0x00a7_b920, &[k, q], U32, |o| {
                    w(scene::tk_menu_get_scene_handle(o, i(k), q))
                });
            }
        }
        for &h in ph.iter().chain(&sh).chain(&[0, 4, 0xffff_ffff]) {
            for n in [0u32, 1, 2] {
                t.twin(
                    "TkMenuGetParamOffset",
                    0x00a7_b8c0,
                    &[slot, h, n],
                    U32,
                    |o| w(scene::tk_menu_get_param_offset(o, i(slot), i(h), i(n))),
                );
            }
            if h != 0xffff_ffff {
                t.twin("TkMenuIsTermScene", 0x00a7_bb90, &[slot, h], U32, |o| {
                    w(scene::tk_menu_is_term_scene(o, i(slot), h))
                });
            }
        }
        if round % 10 == 9 {
            t.twin("TkMenuInitSeq", 0x00a7_bb00, &[], VOID, |o| {
                scene::tk_menu_init_seq(o);
                0
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

/// (name, original, global to scramble, its length, result mask, the bridge).
type Getter = (&'static str, u32, u32, usize, u64, fn(&mut Oracle) -> u64);
/// (name, original, the bridge with the argument).
type Setter = (&'static str, u32, fn(&mut Oracle, u32));
/// (name, original, the bridge with a window and a 16-bit value).
type Set = (&'static str, u32, fn(&mut Oracle, u32, u16));
/// (name, original, the bridge with window, id and value).
type ControlSetter = (&'static str, u32, fn(&mut Oracle, u32, u32, u32));

#[test]
fn tk_matches_the_original() {
    let Some(mut t) = setup_any(0x5eed_0001) else {
        return;
    };
    // one global read (scrambled first) or a constant
    let getters: [Getter; 24] = [
        (
            "TkSndReportStatus",
            0x0088_7b30,
            tk::SND_STATUS.sym,
            4,
            U32,
            |o| w(u32::from(tk::tk_snd_report_status(o))),
        ),
        ("TkGetLang", 0x0088_7d00, tk::LANG.sym, 1, U32, |o| {
            s(tk::tk_get_lang(o))
        }),
        (
            "TkGetLastSyncVCounter",
            0x0088_7dc0,
            tk::LANG.sym,
            0,
            U32,
            |_| w(tk::tk_get_last_sync_v_counter()),
        ),
        (
            "TkGetSoundSyncVFlag",
            0x0088_7dd0,
            tk::LANG.sym,
            0,
            U32,
            |_| w(tk::tk_get_last_sync_v_counter()),
        ),
        (
            "TkGetTotalSyncVCounter",
            0x0088_7e10,
            tk::TOTAL_SYNC_V_COUNTER.sym,
            4,
            U64,
            |o| tk::tk_get_total_sync_v_counter(o),
        ),
        (
            "TkGetBattleMusicSize",
            0x0088_d570,
            tk::LANG.sym,
            0,
            U32,
            |o| w(tk::tk_get_battle_music_size(o)),
        ),
        (
            "TkGetBattleMusicBuffer",
            0x0088_d580,
            tk::BATTLE_MUSIC_BUFFER.sym,
            4,
            U32,
            |o| w(tk::tk_get_battle_music_buffer(o)),
        ),
        (
            "TkGetEncountCounterEventStart",
            0x0088_d5c0,
            tk::ENCOUNT_COUNTER_EVENT_START.sym,
            4,
            U32,
            |o| w(tk::tk_get_encount_counter_event_start(o)),
        ),
        (
            "TkGetFFXCofingAdrs",
            0x0088_d5e0,
            tk::LANG.sym,
            0,
            U32,
            |o| w(tk::tk_get_ffx_cofing_adrs(o)),
        ),
        ("TkGetTGS", 0x0088_d6d0, tk::TGS.sym, 1, U32, |o| {
            w(u32::from(tk::tk_get_tgs(o)))
        }),
        (
            "TkMenuGetCurrentPlayerPos",
            0x008a_9870,
            tk::MENU_CURRENT_PLAYER_POS.sym,
            4,
            U32,
            |o| w(tk::tk_menu_get_current_player_pos(o)),
        ),
        (
            "TkMenuGetDrawShadeRatio",
            0x008a_98a0,
            tk::MENU_DRAW_SHADE_RATIO.sym,
            4,
            U32,
            |o| w(tk::tk_menu_get_draw_shade_ratio(o)),
        ),
        (
            "TkMenuGetPlayerListMax2",
            0x008a_9b50,
            tk::MENU_PLAYER_LIST_MAX2.sym,
            4,
            U32,
            |o| w(tk::tk_menu_get_player_list_max2(o)),
        ),
        (
            "TkMenuGetSummonListMax",
            0x008a_9bf0,
            tk::MENU_SUMMON_LIST_MAX.sym,
            4,
            U32,
            |o| w(tk::tk_menu_get_summon_list_max(o)),
        ),
        (
            "TkMenuMainGetMenuActiveMask",
            0x008a_a460,
            tk::MENU_MAIN_ACTIVE_MASK.sym,
            4,
            U32,
            |o| w(tk::tk_menu_main_get_menu_active_mask(o)),
        ),
        (
            "TkMenuSummonEnableMask",
            0x008a_b1e0,
            tk::MENU_SUMMON_ENABLE_MASK.sym,
            4,
            U32,
            |o| w(tk::tk_menu_summon_enable_mask(o)),
        ),
        (
            "TkMn2GetMainFrameRatio",
            0x008a_b230,
            tk::MN2_MAIN_FRAME_RATIO.sym,
            4,
            U32,
            |o| w(tk::tk_mn2_get_main_frame_ratio(o)),
        ),
        (
            "TkMn2GetMenuMode",
            0x008a_b240,
            tk::MN2_MENU_MODE.sym,
            4,
            U32,
            |o| w(tk::tk_mn2_get_menu_mode(o)),
        ),
        (
            "TkMenuGetImportantCounter",
            0x008c_8a10,
            tk::MENU_IMPORTANT_COUNTER.sym,
            4,
            U32,
            |o| w(tk::tk_menu_get_important_counter(o)),
        ),
        (
            "TkMenuImportantListMax",
            0x008c_8a20,
            tk::MENU_IMPORTANT_LIST_MAX.sym,
            4,
            U32,
            |o| w(tk::tk_menu_important_list_max(o)),
        ),
        (
            "TkMenuGetDummyControlerFlag",
            0x008c_a510,
            tk::MENU_DUMMY_CONTROLER_FLAG.sym,
            1,
            U32,
            |o| w(tk::tk_menu_get_dummy_controler_flag(o)),
        ),
        (
            "TkMenuMainGetMaxOptions",
            0x008e_2280,
            tk::MENU_MAIN_MAX_OPTIONS.sym,
            4,
            U32,
            |o| w(tk::tk_menu_main_get_max_options(o)),
        ),
        (
            "TkMenuGetCurrentPlayer",
            0x008a_9860,
            tk::MENU_PLAYERS.sym,
            16,
            U32,
            |o| w(u32::from(tk::tk_menu_get_current_player(o))),
        ),
        (
            "TkMenuGetCurrentSummon",
            0x008a_9880,
            tk::MENU_SUMMONS.sym,
            16,
            U32,
            |o| w(u32::from(tk::tk_menu_get_current_summon(o))),
        ),
    ];
    // the game's own values first
    for &(name, sym, _, _, mask, rust) in &getters {
        t.twin(name, sym, &[], mask, rust);
    }
    for _ in 0..40 {
        for &(name, sym, global, len, mask, rust) in &getters {
            t.scramble(global, len);
            if name.starts_with("TkMenuGetCurrent") {
                let pos = if name.ends_with("Player") {
                    tk::MENU_CURRENT_PLAYER_POS
                } else {
                    tk::MENU_CURRENT_SUMMON_POS
                };
                let k = t.rng.below(16);
                t.put(t.sym(pos.sym), k);
            }
            t.twin(name, sym, &[], mask, rust);
        }
    }
    // one argument stored, or a constant
    let setters: [Setter; 20] = [
        (
            "TkSndSetStandbyVoice",
            0x0088_7b50,
            tk::tk_snd_set_standby_voice,
        ),
        ("FFXSetLang", 0x0088_7d20, |o, v| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            tk::ffx_set_lang(o, v as i8);
        }),
        (
            "TkMenuDrawShadeRatioDiff",
            0x008a_9670,
            tk::tk_menu_draw_shade_ratio_diff,
        ),
        (
            "TkMenuMainSetIgnoreJoinFlag",
            0x008a_ad60,
            tk::tk_menu_main_set_ignore_join_flag,
        ),
        (
            "TkMenuSetCurrentPlayerPos",
            0x008a_ae80,
            tk::tk_menu_set_current_player_pos,
        ),
        (
            "TkMn2SetMainFrameRatioDiff",
            0x008a_b260,
            tk::tk_mn2_set_main_frame_ratio_diff,
        ),
        ("TkMenuSetDummyControlerFlag", 0x008c_a570, |o, v| {
            #[allow(clippy::cast_possible_truncation)]
            tk::tk_menu_set_dummy_controler_flag(o, v as u8);
        }),
        ("TkSetDebugMenuStatus", 0x0088_ea30, |o, v| {
            tk::tk_set_debug_menu_status(o, i(v));
        }),
        ("TkSetFadeOut", 0x0088_eb50, tk::tk_set_fade_out),
        ("TkMenuSetHelpMessage", 0x008a_aeb0, |o, v| {
            tk::tk_menu_set_help_message(o, i(v));
        }),
        ("TkSndClearKickFlag", 0x0088_7ab0, |o, _| {
            tk::tk_snd_clear_kick_flag(o);
        }),
        ("TkClearSoundSyncVFlag", 0x0088_7d30, |o, _| {
            tk::tk_clear_sound_sync_v_flag(o);
        }),
        ("TkInitControlerWork", 0x0088_8f00, |o, _| {
            tk::tk_init_controler_work(o);
        }),
        ("resetButtonData", 0x008b_1330, |o, _| {
            tk::reset_button_data(o);
        }),
        ("TkMenuSaveLoadActive", 0x008b_55c0, |o, _| {
            tk::tk_menu_save_load_active(o);
        }),
        ("TkMenuFlushDummyControler", 0x008c_a490, |o, _| {
            tk::tk_menu_flush_dummy_controler(o);
        }),
        ("CloseScissor", 0x008e_5cd0, |o, _| {
            scissor::close_scissor(o);
        }),
        ("TkMenuCloseSubCmdWindow", 0x008e_36f0, |o, _| {
            tk::tk_menu_close_sub_cmd_window(o);
        }),
        ("TkMenuRestartSubCmdWindow2", 0x008e_49d0, |o, _| {
            tk::tk_menu_restart_sub_cmd_window2(o);
        }),
        ("TkMenuDrawEnv", 0x008d_cca0, |o, _| tk::tk_menu_draw_env(o)),
    ];
    for _ in 0..60 {
        // what the setters read or keep
        t.scramble(tk::MENU_HELP_MESSAGE.sym, 12);
        t.scramble(tk::MENU_DUMMY_CONTROLER.sym, 16);
        t.scramble(scissor::SCISSOR_ON.sym, 8);
        let sub = if t.rng.below(3) == 0 {
            0
        } else {
            t.alloc_random(0x50)
        };
        t.put(t.sym(tk::MENU_SUB_CMD_WINDOW.sym), sub);
        let packet = t.alloc_random(0x80);
        t.put(t.sym(tk::MENU_PACKET.sym), packet);
        for &(name, sym, rust) in &setters {
            let v = if name == "TkMenuSetHelpMessage" && t.rng.below(2) == 0 {
                tk::MENU_HELP_MESSAGE.get(&t.b)[0].cast_unsigned()
            } else {
                t.rng.int()
            };
            t.twin(name, sym, &[v], VOID, |o| {
                rust(o, v);
                0
            });
        }
        t.scramble(tk::VIBRATE_VOLUME.sym, 6);
        if t.rng.below(2) == 0 {
            t.put(t.sym(tk::VIBRATE_OFF.sym), 0u8);
        }
        let pad = t
            .rng
            .pick_or_any(&[0, 1, 3, 7, 15, 16, 17, 31, 32, 0xffff_ffff, 0x8000_0000]);
        let on = t.rng.pick_or_any(&[0u32, 1]);
        t.twin("TkSetVibrateMask", 0x0087_5f60, &[pad, on], VOID, |o| {
            tk::tk_set_vibrate_mask(o, i(pad), i(on));
            0
        });
        let (pa, pb) = (t.alloc_random(4), t.alloc_random(4));
        t.twin("TkGetVibrateVolume", 0x0087_5e10, &[pa, pb], VOID, |o| {
            tk::tk_get_vibrate_volume(o, pa, pb);
            0
        });
        let out = t.alloc_random(8);
        let h = [out, t.rng.next(), t.rng.next(), t.rng.next(), t.rng.next()];
        t.twin("TkCreateDECI2Header", 0x0088_5120, &h, VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            tk::tk_create_deci2_header(o, out, h[1] as u16, h[2] as u16, h[3] as u8, h[4] as u8);
            0
        });
        let args = [t.rng.int(), t.rng.int(), t.rng.int(), t.rng.int()];
        t.twin("TkMn2SetFontShade", 0x008c_2c30, &args, VOID, |o| {
            tk::tk_mn2_set_font_shade(o, args[0], args[1], args[2], args[3]);
            0
        });
        t.twin("TkSetDebugFontLocate", 0x0088_faf0, &args[..2], VOID, |o| {
            tk::tk_set_debug_font_locate(o, i(args[0]), i(args[1]));
            0
        });
        t.twin("setButtonData", 0x008b_1350, &args[..2], VOID, |o| {
            tk::set_button_data(o, args[0], args[1]);
            0
        });
        let win = if t.rng.below(4) == 0 {
            0
        } else {
            t.alloc_random(0x50)
        };
        t.twin(
            "TkMenuMainClearSelectWindow",
            0x008a_a1e0,
            &[win],
            VOID,
            |o| {
                tk::tk_menu_main_clear_select_window(o, win);
                0
            },
        );
        t.twin(
            "TkMenuRestartSelFileWindow",
            0x008b_5530,
            &[win],
            VOID,
            |o| {
                tk::tk_menu_restart_sel_file_window(o, win);
                0
            },
        );
        let p = t.alloc_random(0x30);
        t.twin("TkSetEvSaveStatus", 0x0088_ea50, &[p], VOID, |o| {
            tk::tk_set_ev_save_status(o, p);
            0
        });
        // indexed tables
        t.scramble(tk::SYNC_V_CALLBACK_HANDLES.sym, 24);
        t.scramble(tk::SYNC_H.sym, 16);
        t.scramble(tk::MENU_MAIN_OPTIONS.sym, 16 * 8);
        t.scramble(tk::BG_THREAD_FUNC.sym, 4);
        let k = t.rng.pick(&[0u32, 1, 2, 3, 4, 0xffff_ffff, 0x8000_0000]);
        let f = t.rng.next();
        t.twin("TkGetSyncVCallbackHandle", 0x0088_8620, &[k], U32, |o| {
            w(tk::tk_get_sync_v_callback_handle(o, i(k)))
        });
        t.twin("TkSetSyncVCallbackPtr", 0x0088_8790, &[k, f], VOID, |o| {
            tk::tk_set_sync_v_callback_ptr(o, i(k), f);
            0
        });
        t.twin("TkRegistBGThreadFunc", 0x0088_8740, &[f], U32, |o| {
            w(tk::tk_regist_bg_thread_func(o, f))
        });
        let k = t.rng.below(4);
        t.twin("TkGetSyncH", 0x0088_c800, &[k], U32, |o| {
            w(tk::tk_get_sync_h(o, i(k)))
        });
        let k = t.rng.below(16);
        t.twin("TkMenuMainGetOptionMenu", 0x008e_2270, &[k], U32, |o| {
            w(tk::tk_menu_main_get_option_menu(o, i(k)))
        });
        t.twin("TkMenuMainGetOptionEnabled", 0x008e_2290, &[k], U32, |o| {
            s(tk::tk_menu_main_get_option_enabled(o, i(k)))
        });
        let p = t.rng.pick_or_any(&[0u32, 1]);
        t.twin("get_ptr32_func", 0x00a4_4650, &[p], U32, |_| {
            w(ffx_menu::get_ptr32_func(p))
        });
        t.twin("is_ptr32_null", 0x00a4_46a0, &[p], U32, |_| {
            w(u32::from(ffx_menu::is_ptr32_null(p)))
        });
    }
    // pad maps, command lists, the CRC
    for _ in 0..500 {
        let map: Vec<u8> = t.rng.bytes(64).into_iter().map(|b| b % 24).collect();
        let at = t.sym(tk::PAD_BUTTON_MAP.sym);
        t.put_bytes(at, &map);
        let (a, b) = (t.rng.below(3), t.rng.below(2));
        let buttons = t.rng.next();
        t.twin("Pad_MapButtons", 0x0088_8c40, &[a, b, buttons], U32, |o| {
            #[allow(clippy::cast_possible_truncation)]
            w(u32::from(tk::pad_map_buttons(
                o,
                i(a),
                i(b),
                buttons as u16,
            )))
        });
        let n = t.rng.below(12) as usize;
        let mut list = Vec::new();
        for _ in 0..n {
            list.extend([
                u8::try_from(t.rng.below(3)).unwrap(),
                u8::try_from(t.rng.below(3)).unwrap(),
            ]);
            list.extend(t.rng.bytes(2));
        }
        list.extend([0xff, 0, 0, 0]);
        let list = t.alloc(&list);
        let enabled = t.rng.pick(&[0u32, 1, 2]);
        let (x, y) = (
            t.rng.pick(&[0u32, 1, 2, 0x100, 0xffff_ffff]),
            t.rng.pick(&[0u32, 1, 2, 0x101]),
        );
        t.twin(
            "TkMenuGetCommandMax",
            0x008d_2d80,
            &[list, enabled, x, y],
            U32,
            |o| s(tk::tk_menu_get_command_max(o, list, i(enabled), x, y)),
        );
        t.twin(
            "TkMenuGetCommandOffset",
            0x008d_2dc0,
            &[list, enabled, x, y],
            U32,
            |o| s(tk::tk_menu_get_command_offset(o, list, i(enabled), x, y)),
        );
        let len = t.rng.below(300);
        let data = t.alloc_random(len as usize + 1);
        let len = t.rng.pick(&[len, len, 0, 0xffff_ffff]);
        t.twin("TkCRC", 0x008b_1450, &[len, data], U16, |o| {
            w(u32::from(tk::tk_crc(o, i(len), data)))
        });
    }
    // a CRC the save code computes: all 256 byte values, twice
    let all: Vec<u8> = (0..=255u8).chain(0..=255).collect();
    let data = t.alloc(&all);
    t.twin("TkCRC", 0x008b_1450, &[512, data], U16, |o| {
        w(u32::from(tk::tk_crc(o, 512, data)))
    });
    // summon growth: in battle only
    if ffx_kernel::bridge::SUMMON_GROW.get(&t.b) != 0 {
        t.twin("TkMn2GetSummonGrow", 0x008c_1c20, &[], U32, |o| {
            w(tk::tk_mn2_get_summon_grow(o))
        });
    } else {
        eprintln!("no summon growth table in this snapshot; TkMn2GetSummonGrow skipped");
    }
    eprintln!("{} calls", t.calls);
}

impl Xorshift {
    /// A float in `lo..hi`.
    #[allow(clippy::cast_precision_loss)]
    fn float(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next() as f32 / u32::MAX as f32)
    }
}

#[test]
fn floats_match_the_original() {
    let Some(mut t) = setup_any(0xf10a_7001) else {
        return;
    };
    let pi = std::f32::consts::PI;
    let mut angles = vec![
        0.0,
        -0.0,
        pi,
        -pi,
        f32::from_bits(pi.to_bits() + 1),
        -f32::from_bits(pi.to_bits() + 1),
        f32::from_bits(pi.to_bits() - 1),
        std::f32::consts::TAU,
        -std::f32::consts::TAU,
        1000.0,
        -99_999.0,
        f32::NAN,
    ];
    angles.extend((0..2000).map(|_| t.rng.float(-50.0, 50.0)));
    for a in angles {
        let r =
            t.a.call(t.a.rebase(0x0088_bc20), Conv::Cdecl, &[a.to_bits()])
                .unwrap();
        let want = r.st0.expect("a float in st0");
        let got = f64::from(ffx_menu::math::wrap_angle(a));
        assert!(
            ffx_oracle::same_float(want, got),
            "Math_WrapAngle({a}): {want} vs {got}"
        );
    }
    let specials = [
        0.0,
        -0.0,
        1e-30,
        1e-45,
        1e20,
        1e30,
        f32::INFINITY,
        f32::NAN,
        -3.0,
    ];
    for k in 0..3000 {
        let mut v = [0f32; 4];
        for x in &mut v {
            *x = if k % 4 == 0 {
                t.rng.pick(&specials)
            } else {
                t.rng.float(-100.0, 100.0)
            };
        }
        if k % 4 == 1 {
            // on the boundary: r within an ulp or two of the distance
            #[allow(clippy::cast_possible_truncation)]
            let d = (f64::from(v[1] * v[1]) + f64::from(v[2] * v[2]) + f64::from(v[3] * v[3]))
                .sqrt() as f32;
            let nudge = t.rng.below(5).cast_signed() - 2;
            v[0] = f32::from_bits(d.to_bits().wrapping_add_signed(nudge));
        }
        let args = v.map(f32::to_bits);
        t.twin("Math_InRadius", 0x0088_b730, &args, U32, |_| {
            w(u32::from(ffx_menu::math::in_radius(v[0], v[1], v[2], v[3])))
        });
    }
    // the scissor: random boxes, scales and rectangles, the pointers into
    // one block (sometimes the same float twice)
    for k in 0..4000 {
        let on = t.rng.pick(&[1u32, 1, 1, 0, 2]);
        t.put(t.sym(scissor::SCISSOR_ON.sym), on);
        let xyuv = t.rng.pick(&[0u8, 1, 1, 2]);
        t.put(t.sym(scissor::SCISSOR_XYUV.sym), xyuv);
        let any = t.rng.float(0.1, 4.0);
        let scale_a = t.rng.pick(&[1.0, 0.5, 2.0, any]);
        let any = t.rng.float(0.1, 4.0);
        let scale_b = t.rng.pick(&[1.0, 0.75, any]);
        t.put(t.sym(scissor::SCISSOR_SCALES.sym), [scale_a, scale_b]);
        let (l, top) = (t.rng.float(-50.0, 300.0), t.rng.float(-50.0, 200.0));
        let bx = [
            l,
            top,
            l + t.rng.float(0.0, 400.0),
            top + t.rng.float(0.0, 300.0),
        ];
        t.put(t.sym(scissor::SCISSOR_BOX.sym), bx);
        let x0 = t.rng.float(-200.0, 700.0);
        let y0 = t.rng.float(-200.0, 500.0);
        let q = [
            x0,
            y0,
            x0 + t.rng.float(-50.0, 500.0),
            y0 + t.rng.float(-50.0, 400.0),
            t.rng.float(0.0, 1.0),
            t.rng.float(0.0, 1.0),
            t.rng.float(0.0, 1.0),
            t.rng.float(0.0, 1.0),
        ];
        let q = if k % 50 == 0 {
            [bx[0], bx[1], bx[2], bx[3], 0.0, 0.0, 1.0, 1.0]
        } else {
            q
        };
        let mut bytes = Vec::new();
        for v in q {
            bytes.extend(v.to_le_bytes());
        }
        let block = t.alloc(&bytes);
        let mut at: [u32; 8] = std::array::from_fn(|n| block + 4 * u32::try_from(n).unwrap());
        if k % 10 == 7 {
            let (p, r) = (t.rng.below(8) as usize, t.rng.below(8) as usize);
            at[p] = at[r];
        }
        t.twin("BeginScissor_XYUV", 0x008e_5a20, &at, U32, |o| {
            w(u32::from(scissor::begin_scissor_xyuv_at(o, at)))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn players_vibration_and_degrees_match_the_original() {
    let Some(mut t) = setup_any(0x0dd_ba11) else {
        return;
    };
    for _ in 0..1500 {
        t.scramble(tk::MENU_PLAYERS.sym, 16);
        for g in [tk::MENU_PLAYER_LIST_MAX, tk::MENU_PLAYER_LIST_MAX2] {
            let v = t.rng.pick(&[0u32, 1, 3, 7, 0xffff_ffff, 0x7fff_ffff]);
            t.put(t.sym(g.sym), v);
        }
        let v = t.rng.pick(&[0u32, 1, 0x100]);
        t.put(t.sym(tk::MENU_MAIN_IGNORE_JOIN.sym), v);
        let v = t
            .rng
            .pick(&[0u32, 1, 2, 6, 7, 0xffff_ffff, 0x7fff_ffff, 0x8000_0000]);
        t.put(t.sym(tk::MENU_CURRENT_PLAYER_POS.sym), v);
        t.scramble(tk::MENU_PLAYER_STEP.sym, 1);
        t.twin("TkMenuMainGetIgnoreJoinFlag", 0x008a_a450, &[], U32, |o| {
            w(tk::tk_menu_main_get_ignore_join_flag(o))
        });
        t.twin("TkMenuGetPlayerListMax", 0x008a_9b30, &[], U32, |o| {
            w(tk::tk_menu_get_player_list_max(o))
        });
        let k = t
            .rng
            .pick(&[0u32, 1, 2, 5, 6, 7, 9, 0xffff_ffff, 0x8000_0000]);
        t.twin("TkMenuGetPlayerFromIndex2", 0x008a_9ad0, &[k], U32, |o| {
            w(u32::from(tk::tk_menu_get_player_from_index(o, i(k))))
        });
        t.twin(
            "TkMenuGetPlayerFromIndex2_008a9b00",
            0x008a_9b00,
            &[k],
            U32,
            |o| w(u32::from(tk::tk_menu_get_player_from_index2(o, i(k)))),
        );
        if t.rng.below(2) == 0 {
            t.twin("TkMenuSetNextPlayer2", 0x008a_af40, &[], VOID, |o| {
                tk::tk_menu_set_next_player2(o);
                0
            });
        } else {
            t.twin("TkMenuSetPrevPlayer2", 0x008a_afc0, &[], VOID, |o| {
                tk::tk_menu_set_prev_player2(o);
                0
            });
        }
        t.scramble(tk::VIBRATE_PADS.sym, 8 * tk::VIBRATE_PAD as usize);
        let pad = t
            .rng
            .pick(&[0u32, 1, 5, 7, 8, 100, 0xffff_ffff, 0x8000_0000]);
        t.twin("TkStopVibrate", 0x0087_6290, &[pad], VOID, |o| {
            tk::tk_stop_vibrate(o, i(pad));
            0
        });
        t.scramble(tk::VIBRATE_PADS.sym, 8 * tk::VIBRATE_PAD as usize);
        t.twin("TkStopVibrateAll", 0x0087_62f0, &[], VOID, |o| {
            tk::tk_stop_vibrate_all(o);
            0
        });
    }
    // degrees: random, whole turns, and the wrap's edges
    let mut degrees = vec![
        0.0,
        -0.0,
        180.0,
        -180.0,
        360.0,
        540.0,
        -720.0,
        90.0,
        f32::NAN,
    ];
    degrees.extend((0..3000).map(|_| t.rng.float(-2000.0, 2000.0)));
    for a in degrees {
        let r =
            t.a.call(t.a.rebase(0x0088_9b80), Conv::Cdecl, &[a.to_bits()])
                .unwrap();
        let want = r.st0.expect("a float in st0");
        let got = f64::from(ffx_menu::math::deg_to_rad(a));
        assert!(
            ffx_oracle::same_float(want, got),
            "TkDegtoRad({a}): {want} vs {got}"
        );
    }
    // the summon growth count, over the loaded table with its flag bytes
    // rewritten (in both)
    let kernel = ffx_kernel::bridge::SUMMON_GROW;
    if kernel.get(&t.b) == 0 {
        eprintln!("no summon growth table in this snapshot; SetSummonGrowMax skipped");
        return;
    }
    let record = ffx_kernel::bridge::first_record(&mut t.b, kernel, 0);
    let table = kernel.get(&t.b);
    let n = ffx_kernel::bridge::ms_get_excel_data_num(&t.b, table, 0);
    eprintln!("summon growth: {n} records at {record:#x}");
    for round in 0..60 {
        let start = if round % 4 == 0 { t.rng.next() } else { 0 };
        t.put(t.sym(tk::MENU_SUMMON_GROW_MAX.sym), start);
        if round > 0 {
            for r in 0..u32::try_from(n).unwrap_or(0) {
                let v = u8::from(t.rng.below(6) == 0);
                t.put(record + 8 * r + 7, v);
            }
        }
        t.twin("SetSummonGrowMax", 0x008c_1910, &[], VOID, |o| {
            tk::set_summon_grow_max(o);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

/// Compares a float the original leaves in `st0` with the bridge's.
fn same_float(what: &str, want: Option<f64>, got: f32) {
    let want = want.expect("a float in st0");
    let got = f64::from(got);
    assert!(ffx_oracle::same_float(want, got), "{what}: {want} vs {got}");
}

#[test]
fn gu_packets_and_windows_match_the_original() {
    let Some(mut t) = setup_any(0x9a11_0002) else {
        return;
    };
    for round in 0..400 {
        // the packet buffers, the scale and the screen
        let tags = t.alloc_random(0x200);
        let data = t.alloc_random(0x200);
        t.put(t.sym(gu::PACKET.sym), tags);
        t.put(t.sym(gu::PACKET_DATA.sym), data);
        let n = t.rng.below(8);
        t.put(t.sym(gu::PACKET_COUNT.sym), n);
        let scale = t.rng.pick(&[0x100u32, 0x80, 0x1c0, 0xffff_ff00]);
        t.put(t.sym(gu::H_SCALE.sym), scale);
        let h = t.rng.pick_or_any(&[448, 512, 0]);
        let wd = t.rng.pick_or_any(&[640, 512]);
        t.put(t.sym(gu::SCREEN_SIZE.sym), [h, wd]);
        let win = if round % 4 == 0 {
            0
        } else {
            t.alloc_random(0xa0)
        };
        t.twin("Gu_PacketBeginTag", 0x0084_8530, &[win], VOID, |o| {
            gu::gu_packet_begin_tag(o, win);
            0
        });
        for _ in 0..3 {
            let v = t.rng.int();
            t.twin("Gu_SetPrim", 0x0084_8480, &[v], VOID, |o| {
                gu::gu_set_prim(o, i(v));
                0
            });
            let c: [u32; 4] = std::array::from_fn(|_| {
                if t.rng.below(2) == 0 {
                    t.rng.below(256)
                } else {
                    t.rng.int()
                }
            });
            t.twin("Gu_SetRgbaq", 0x0084_84c0, &c, VOID, |o| {
                gu::gu_set_rgbaq(o, c.map(i));
                0
            });
            let f: [u32; 8] = std::array::from_fn(|_| {
                if t.rng.below(2) == 0 {
                    t.rng.below(4)
                } else {
                    t.rng.int()
                }
            });
            t.twin("Gu_SetTest", 0x0084_85d0, &f, VOID, |o| {
                gu::gu_set_test(o, f.map(i));
                0
            });
            let (x, y) = (t.rng.int(), t.rng.int());
            t.twin("Gu_SetXyzf2", 0x0084_8680, &[x, y], VOID, |o| {
                gu::gu_set_xyzf2(o, i(x), i(y));
                0
            });
        }
        t.twin("Gu_PacketEndTag", 0x0084_8460, &[], VOID, |o| {
            gu::gu_packet_end_tag(o);
            0
        });
        // text settings
        t.scramble(gu::PRINT_SIZE.sym, 0x50);
        let (a, b) = (t.rng.int(), t.rng.int());
        t.twin("Gu_PrintSetSize", 0x0084_8770, &[a], VOID, |o| {
            gu::gu_print_set_size(o, i(a));
            0
        });
        t.twin("Gu_PrintSetSizeOnly", 0x0084_87d0, &[b], VOID, |o| {
            gu::PRINT_SIZE.set(o, b);
            0
        });
        t.twin("Gu_PrintSetBold", 0x0084_8720, &[a], VOID, |o| {
            gu::PRINT_BOLD.set(o, a);
            0
        });
        t.twin("Gu_PrintSetItalic", 0x0084_8760, &[b], VOID, |o| {
            gu::PRINT_ITALIC.set(o, b);
            0
        });
        t.twin("Gu_WindowSetHokanEnable", 0x0084_8f00, &[a], VOID, |o| {
            gu::WINDOW_HOKAN.set(o, a);
            0
        });
        t.twin("Gu_PrintSetXY", 0x0084_8840, &[a, b], VOID, |o| {
            gu::gu_print_set_xy(o, i(a), i(b));
            0
        });
        t.scramble(gu::MOUSE_X.sym, 8);
        t.twin("Gu_MouseGetY", 0x0084_9ae0, &[], U32, |o| {
            s(gu::MOUSE_X.get(o))
        });
        t.twin("Gu_MouseGetX", 0x0084_9af0, &[], U32, |o| {
            s(gu::MOUSE_Y.get(o))
        });
        let sel = selector(&mut t);
        t.twin("Gu_SelectorGetItemValue", 0x0084_9d10, &[sel], U32, |o| {
            w(gu::gu_selector_get_item_value(o, sel))
        });
        // window fields
        let win = t.alloc_random(0xa0);
        for (name, sym, off) in [
            ("Gu_WindowGetClientH", 0x0084_aed0, 0x86),
            ("Gu_WindowGetClientW", 0x0084_aee0, 0x84),
            ("Gu_WindowGetClientTop", 0x0084_af00, 0x82),
            ("Gu_WindowGetSizeH", 0x0084_af10, 0x12),
            ("Gu_WindowGetHeight", 0x0084_af30, 0x1a),
            ("Gu_WindowGetSizeW", 0x0084_af90, 0x10),
            ("Gu_WindowGetY", 0x0084_afb0, 0xc),
        ] {
            t.twin(name, sym, &[win], U32, |o| {
                s(gu::gu_window_field(o, win, off))
            });
        }
        t.twin("Gu_WindowSetPos", 0x0084_b220, &[win, a, b], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            gu::gu_window_set_pos(o, win, a as u16, b as u16);
            0
        });
        t.twin("Gu_WindowSetSize", 0x0084_b720, &[win, b, a], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            gu::gu_window_set_size(o, win, b as u16, a as u16);
            0
        });
        let on = t.rng.pick_or_any(&[0, 1, 2, 3]);
        t.twin("Gu_WindowSetFlag2", 0x0084_b600, &[win, on], VOID, |o| {
            gu::gu_window_set_flag2(o, win, on);
            0
        });
        // the open list: to the front, and moving a window with its children
        let n = t.rng.below(6) as usize;
        let wins: Vec<u32> = (0..=n).map(|_| t.alloc_random(0xa0)).collect();
        for k in 0..n {
            let next = if k + 1 < n { wins[k + 1] } else { 0 };
            t.put(wins[k], next);
            // a parent earlier in the list, or none of them
            let parent = if k > 0 && t.rng.below(2) == 0 {
                wins[t.rng.below(u32::try_from(k).unwrap()) as usize]
            } else {
                t.rng.next() | 1
            };
            t.put(wins[k] + 8, parent);
        }
        t.put(t.sym(gu::WINDOW_LIST.sym), if n == 0 { 0 } else { wins[0] });
        let target = wins[t.rng.below(u32::try_from(n).unwrap() + 1) as usize];
        if t.rng.below(2) == 0 {
            t.twin("Gu_WindowToFront", 0x0084_b770, &[target], VOID, |o| {
                gu::gu_window_to_front(o, target);
                0
            });
        } else {
            t.twin("Gu_WindowStartMove", 0x0084_a530, &[target], VOID, |o| {
                gu::gu_window_start_move(o, target);
                0
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn tk_round2_matches_the_original() {
    let Some(mut t) = setup(0x9a11_0003) else {
        return;
    };
    let event = ffx_battle::save::bridge::ms_get_save_event_address(&t.b);
    let menu = ffx_battle::save::bridge::ms_get_save_menu(&t.b);
    // the game's own text tables first, both scripts
    for hiragana in [0u32, 8] {
        let m = Mem::read::<u32>(&t.b, menu);
        t.put(menu, (m & !8) | hiragana);
        let out = t.alloc(&[0; 4]);
        for id in (0..0x140u32).chain([0x2000, 0x2001, 0x6000, 0x7fff, 0xffff]) {
            t.twin("TkMsGetRomItem", 0x008a_b280, &[id, out], U32, |o| {
                w(tk::tk_ms_get_rom_item(o, id, out))
            });
            for (name, sym) in [
                ("TkMenuGetItemHelp", 0x008a_98f0),
                ("TkMn2GetItemHelp", 0x008c_1b30),
            ] {
                t.twin(name, sym, &[id], U32, |o| w(tk::tk_get_item_help(o, id)));
            }
            t.twin("TkMn2GetCommandName", 0x008c_1a20, &[id], U32, |o| {
                w(tk::tk_mn2_get_command_name(o, id))
            });
            t.twin("TkMn2GetAbilityMes", 0x008c_19a0, &[id], U32, |o| {
                w(tk::tk_mn2_get_ability_mes(o, id))
            });
            t.twin("TkMn2GetAbilityHelp", 0x008c_1960, &[id], U32, |o| {
                w(tk::tk_mn2_get_ability_help(o, id))
            });
            for (name, sym) in [
                ("TkMn2GetCommandHelp", 0x008c_19e0),
                ("TkMenuGetCommandHelp", 0x008d_de10),
            ] {
                t.twin(name, sym, &[id], U32, |o| w(tk::tk_get_command_help(o, id)));
            }
            t.twin("TkMsImportantGet", 0x008a_b2c0, &[id], U32, |o| {
                w(tk::tk_ms_important_get(o, id))
            });
        }
    }
    t.twin("TkCalcImportantListMax", 0x008c_6790, &[], U32, |o| {
        w(tk::tk_calc_important_list_max(o))
    });
    for chr in (0..0x12u32).chain([0x100, 0x111]) {
        for (name, sym, off) in [
            ("TkMenuGetHP", 0x008a_98c0, 0x1c),
            ("TkMenuGetMP", 0x008a_9970, 0x20),
            ("TkMenuGetMaxHP", 0x008a_9990, 0x24),
            ("TkMenuGetMaxMP", 0x008a_99b0, 0x28),
        ] {
            t.twin(name, sym, &[chr], U32, |o| {
                w(tk::tk_menu_get_ply(o, chr, off))
            });
        }
    }
    for round in 0..600 {
        // save fields
        let v = t.rng.bytes(0xd0);
        t.put_bytes(event, &v);
        if t.rng.below(2) == 0 {
            t.put(event + 0x2a, 0u8);
        }
        t.twin("TkGetNowEventJumpId", 0x0088_d660, &[], U32, |o| {
            w(u32::from(tk::tk_get_now_event_jump_id(o)))
        });
        t.twin("TkGetNowEventJumpIndex", 0x0088_d670, &[], U32, |o| {
            w(u32::from(tk::tk_get_now_event_jump_index(o)))
        });
        t.twin("TkGetNowEventJumpMapIdNo", 0x0088_d680, &[], U32, |o| {
            w(u32::from(tk::tk_get_now_event_jump_map_id_no(o)))
        });
        t.twin("TkGetNowEventJumpMapNo", 0x0088_d690, &[], U32, |o| {
            w(u32::from(tk::tk_get_now_event_jump_map_no(o)))
        });
        let (a, b) = (t.rng.int(), t.rng.int());
        t.twin("TkSetEventJumpMapNo", 0x0088_eaf0, &[a, b], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            tk::tk_set_event_jump_map_no(o, a as u16, b as u16);
            0
        });
        let m = t.rng.next();
        t.put(menu, m);
        t.twin("TkCheckPlayVoice", 0x0087_2340, &[], U32, |o| {
            w(tk::tk_check_play_voice(o))
        });
        // the summon growth count
        let start = if round % 3 == 0 { t.rng.next() } else { 0 };
        t.put(t.sym(tk::MENU_SUMMON_GROW_MAX.sym), start);
        t.twin("TkMn2GetSummonGrowMax", 0x008c_1c40, &[], U32, |o| {
            w(tk::tk_mn2_get_summon_grow_max(o))
        });
        // a worker's character number
        let work = t.alloc_random(0xa0);
        let chr = if t.rng.below(3) == 0 {
            0
        } else {
            t.alloc_random(0x200)
        };
        t.put(work + 0x9c, chr);
        if t.rng.below(2) == 0 {
            t.put(work + 0x40, u32::MAX);
        }
        t.twin("TkChSetBattleChrNum", 0x0087_2160, &[work], VOID, |o| {
            tk::tk_ch_set_battle_chr_num(o, work);
            0
        });
        // vibration
        t.scramble(tk::VIBRATE_PADS.sym, 8 * tk::VIBRATE_PAD as usize);
        let pad = t.rng.pick_or_any(&[0, 3, 7, 8, 0xffff_ffff]);
        t.twin("TkGetVibrateOn", 0x0087_5de0, &[pad], U32, |o| {
            w(u32::from(tk::tk_get_vibrate_on(o, i(pad))))
        });
        let curve = if t.rng.below(2) == 0 {
            0
        } else {
            t.alloc_random(32)
        };
        let kind = t.rng.pick_or_any(&[0, 1, 2, 3, 4, 5, 6, 7, 0xffff_fffe]);
        let k = t.rng.below(8);
        t.twin(
            "TkGetVibrateCurve",
            0x0087_63e0,
            &[kind, k, curve],
            U32,
            |o| w(u32::from(tk::tk_get_vibrate_curve(o, kind, i(k), curve))),
        );
        // sound and sync flags
        t.scramble(tk::SND_VOICE_FLAGS.sym, 16);
        for (name, sym, k) in [
            ("TkSndIsReadVoice", 0x0088_7b10, 0),
            ("TkSndIsReadVagVoice", 0x0088_7af0, 1),
            ("TkSndIsReadShoutVoice", 0x0088_7b00, 2),
            ("TkSndIsStandbyVoice", 0x0088_7b20, 3),
        ] {
            if t.rng.below(3) == 0 {
                t.put(t.sym(tk::SND_VOICE_FLAGS.sym) + 4 * k, 0u32);
            }
            t.twin(name, sym, &[], U32, |o| w(tk::tk_snd_flag(o, k)));
        }
        t.scramble(tk::SND_KICK_FLAG.sym, 1);
        t.twin("TkSndGetKickFlag", 0x0088_7ac0, &[], U32, |o| {
            w(u32::from(tk::SND_KICK_FLAG.get(o)))
        });
        t.scramble(tk::GS_CSR.sym, 4);
        t.scramble(tk::SYNC_V_COUNTER.sym, 4);
        t.twin("TkGetGsRegisterCSR", 0x0088_7db0, &[], U32, |o| {
            w(tk::GS_CSR.get(o))
        });
        t.twin("TkGetSyncVCounter", 0x0088_7e00, &[], U32, |o| {
            w(tk::SYNC_V_COUNTER.get(o))
        });
        t.twin("TkSetOutputEncount", 0x0088_ebe0, &[a], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            tk::OUTPUT_ENCOUNT.set(o, a as u8);
            0
        });
        t.twin("TkGetOutputEncount", 0x0088_d6a0, &[], U32, |o| {
            w(u32::from(tk::OUTPUT_ENCOUNT.get(o)))
        });
        t.twin("TkSetTGS", 0x0088_ec10, &[b], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            tk::TGS.set(o, b as u8);
            0
        });
        let (pa, pb) = (t.rng.below(4), t.rng.below(4));
        t.twin("TkGetControlerWork", 0x0088_8ec0, &[pa, pb], U32, |o| {
            w(tk::tk_get_controler_work(o, i(pa), i(pb)))
        });
        t.twin("TkGetControlerWork2", 0x0088_8e10, &[pa, pb], U32, |o| {
            w(tk::tk_get_controler_work2(o, i(pa), i(pb)))
        });
        // string widths, both modes
        let len = t.rng.below(24) as usize;
        let mut sbytes: Vec<u8> = t.rng.bytes(len).into_iter().map(|c| c.max(1)).collect();
        sbytes.extend([0, 0, 0]);
        let sp = t.alloc(&sbytes);
        for prop in [0u32, 1] {
            t.twin("TkGetStrWidth", 0x0088_ee70, &[prop, sp], U32, |o| {
                s(tk::tk_get_str_width(o, i(prop), sp))
            });
        }
        // the menu's lists and counters
        t.scramble(tk::MENU_SUMMONS.sym, 16);
        let max = t.rng.pick(&[0u32, 1, 3, 7, 16, 0x7fff_ffff, 0xffff_ffff]);
        t.put(t.sym(tk::MENU_SUMMON_LIST_MAX.sym), max);
        let pos = t.rng.pick_or_any(&[0, 1, 6, 0xffff_ffff]);
        t.put(t.sym(tk::MENU_CURRENT_SUMMON_POS.sym), pos);
        t.scramble(tk::MENU_PLAYER_STEP.sym, 2);
        let k = t.rng.pick(&[0u32, 1, 2, 6, 7, 9, 0xffff_ffff]);
        t.twin("TkMenuGetSummonFromIndex", 0x008a_9bc0, &[k], U32, |o| {
            w(u32::from(tk::tk_menu_get_summon_from_index(o, i(k))))
        });
        t.twin("TkMenuGetCurrentSummonPos", 0x008a_9890, &[], U32, |o| {
            w(tk::MENU_CURRENT_SUMMON_POS.get(o))
        });
        t.twin("TkMenuGetPlayerStep", 0x008a_9840, &[], U32, |o| {
            s(tk::tk_menu_get_player_step(o))
        });
        t.twin("TkMenuGetSummonStep", 0x008a_9bb0, &[], U32, |o| {
            s(tk::tk_menu_get_summon_step(o))
        });
        let (name, sym, rust): (&str, u32, fn(&mut Oracle)) = match t.rng.below(4) {
            0 => (
                "TkMenuSetNextSummon",
                0x008a_af70,
                tk::tk_menu_set_next_summon,
            ),
            1 => (
                "TkMenuSetPrevSummon",
                0x008a_afe0,
                tk::tk_menu_set_prev_summon,
            ),
            2 => (
                "TkMenuSetNextPlayer",
                0x008a_af10,
                tk::tk_menu_set_next_player,
            ),
            _ => (
                "TkMenuSetPrevPlayer",
                0x008a_afa0,
                tk::tk_menu_set_prev_player,
            ),
        };
        for g in [
            tk::MENU_PLAYER_LIST_MAX,
            tk::MENU_PLAYER_LIST_MAX2,
            tk::MENU_MAIN_IGNORE_JOIN,
            tk::MENU_CURRENT_PLAYER_POS,
        ] {
            let v = t.rng.pick_or_any(&[0, 1, 3, 7, 0xffff_ffff]);
            t.put(t.sym(g.sym), v);
        }
        t.twin(name, sym, &[], VOID, |o| {
            rust(o);
            0
        });
        for g in [
            tk::MENU_PLAYER_PARTY_MASK.sym,
            tk::MENU_HELP_RATIO2.sym,
            tk::MN2_LIST_MAX2.sym,
        ] {
            t.scramble(g, 4);
        }
        t.scramble(tk::MENU_TIMER.sym, 1);
        t.twin("TkMenuGetPlayerPartyMask", 0x008a_9b70, &[], U32, |o| {
            w(tk::MENU_PLAYER_PARTY_MASK.get(o))
        });
        t.twin("TkMenuGetTimer", 0x008a_9c60, &[], U32, |o| {
            w(u32::from(tk::MENU_TIMER.get(o)))
        });
        t.twin("TkMenuGetHelpRatio2", 0x008a_b220, &[], U32, |o| {
            w(tk::MENU_HELP_RATIO2.get(o))
        });
        t.twin("TkMn2GetListMax2", 0x008e_3390, &[], U32, |o| {
            w(tk::MN2_LIST_MAX2.get(o))
        });
        t.scramble(tk::MN2_FONT_SHADE.sym, 0x20);
        if round % 2 == 0 {
            t.twin("TkMn2SaveFontShade", 0x008c_2330, &[], VOID, |o| {
                tk::tk_mn2_save_font_shade(o);
                0
            });
        } else {
            t.twin("TkMn2RestoreFontShade", 0x008c_2300, &[], VOID, |o| {
                tk::tk_mn2_restore_font_shade(o);
                0
            });
        }
        t.twin(
            "TkMenuSetDummyControlerPending",
            0x008c_a560,
            &[a],
            VOID,
            |o| {
                tk::tk_menu_set_dummy_controler_pending(o, a);
                0
            },
        );
        // the status background fade
        t.scramble(tk::MN2_STATUS_BG_STEP.sym, 8);
        t.twin("TkMn2StepStatusBG", 0x008d_3280, &[], VOID, |o| {
            tk::tk_mn2_step_status_bg(o);
            0
        });
        t.twin("TkMn2GetStatusBG", 0x008d_4910, &[], U32, |o| {
            s(tk::tk_mn2_get_status_bg(o))
        });
        t.twin("TMn2SetStatusBGDiff", 0x008d_2660, &[b], VOID, |o| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            tk::tmn2_set_status_bg_diff(o, b as i16);
            0
        });
        if round % 7 == 0 {
            t.twin("TkMn2InitStatusBG", 0x008d_4ed0, &[], VOID, |o| {
                tk::tk_mn2_init_status_bg(o);
                0
            });
        }
        // offset tables
        let count = t.rng.below(5);
        let mut table = count.to_le_bytes().to_vec();
        table.extend(t.rng.bytes(4 * count as usize));
        let table = t.alloc(&table);
        let k = t.rng.pick(&[0u32, 1, 2, 4, 5, 0xffff_ffff]);
        t.twin("TkMenuGetTableEntry", 0x008d_ce10, &[k, table], U32, |o| {
            w(tk::tk_menu_get_table_entry(o, k, table))
        });
        // integer helpers
        let n = t.rng.int();
        let step = t.rng.pick_or_any(&[1, 2, 16, 0x800, 0xffff_ffff]).max(1);
        t.twin("Math_RoundUp", 0x0088_d120, &[n, step], U32, |_| {
            w(ffx_menu::math::round_up(n, step))
        });
        t.twin("Math_Clamp1To99", 0x008c_ed10, &[n], U32, |_| {
            s(ffx_menu::math::clamp_1_to_99(i(n)))
        });
        let angle = t.rng.next();
        t.twin("Math_SinTable", 0x0088_c690, &[angle], U32, |o| {
            s(ffx_menu::math::bridge::math_sin_table(o, angle))
        });
    }
    // floats
    let specials = [
        0.0,
        -0.0,
        1e-30,
        1e20,
        f32::INFINITY,
        -f32::INFINITY,
        f32::NAN,
        -3.0,
        3.0,
    ];
    for k in 0..3000 {
        let mut v = [0f32; 3];
        for x in &mut v {
            *x = if k % 4 == 0 {
                t.rng.pick(&specials)
            } else {
                t.rng.float(-1000.0, 1000.0)
            };
        }
        let r =
            t.a.call(
                t.a.rebase(0x0088_b400),
                Conv::Cdecl,
                &[v[0].to_bits(), v[1].to_bits()],
            )
            .unwrap();
        same_float("Math_MinAbs", r.st0, ffx_menu::math::min_abs(v[0], v[1]));
        let r =
            t.a.call(t.a.rebase(0x0088_c3c0), Conv::Cdecl, &v.map(f32::to_bits))
                .unwrap();
        same_float(
            "Math_LengthSq3",
            r.st0,
            ffx_menu::math::length_sq3(v[0], v[1], v[2]),
        );
    }
    // the XY scissor, pointers sometimes aliased
    for k in 0..4000 {
        let on = t.rng.pick(&[1u32, 1, 1, 0, 2]);
        t.put(t.sym(scissor::SCISSOR_ON.sym), on);
        let (l, top) = (t.rng.float(-50.0, 300.0), t.rng.float(-50.0, 200.0));
        let bx = [
            l,
            top,
            l + t.rng.float(0.0, 400.0),
            top + t.rng.float(0.0, 300.0),
        ];
        t.put(t.sym(scissor::SCISSOR_BOX.sym), bx);
        let x0 = t.rng.float(-200.0, 700.0);
        let y0 = t.rng.float(-200.0, 500.0);
        let q = [
            x0,
            y0,
            x0 + t.rng.float(-50.0, 500.0),
            y0 + t.rng.float(-50.0, 400.0),
        ];
        let q = if k % 50 == 0 { bx } else { q };
        let bytes: Vec<u8> = q.iter().flat_map(|v| v.to_le_bytes()).collect();
        let block = t.alloc(&bytes);
        let mut at: [u32; 4] = std::array::from_fn(|n| block + 4 * u32::try_from(n).unwrap());
        if k % 10 == 7 {
            let (p, r) = (t.rng.below(4) as usize, t.rng.below(4) as usize);
            at[p] = at[r];
        }
        t.twin("BeginScissor_XY", 0x008e_58e0, &at, U32, |o| {
            w(u32::from(scissor::begin_scissor_xy_at(o, at)))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_windows_and_items_match_the_original() {
    let Some(mut t) = setup(0x9a11_0004) else {
        return;
    };
    let windows: [(&str, u32, ffx_bridge::Global<u32>, u8); 23] = [
        // 0 close and forget, 1 close, zero +0x10, forget, 2 close only, 3 clear
        (
            "TkMenuSaveLoadCloseWindow2",
            0x008b_3d90,
            tk::SAVE_LOAD_WINDOW2,
            0,
        ),
        (
            "TkMenuSaveLoadCloseWindow",
            0x008b_3e40,
            tk::SAVE_LOAD_WINDOW,
            0,
        ),
        ("TkMenuItemCloseWindow3", 0x008c_6b40, tk::ITEM_WINDOW3, 1),
        ("TkMenuItemCloseWindow", 0x008c_6b60, tk::ITEM_WINDOW, 1),
        ("TkMenuItemCloseWindow2", 0x008c_6b80, tk::ITEM_WINDOW2, 0),
        ("TkMenuItemCloseWindow4", 0x008c_6ba0, tk::ITEM_WINDOW4, 1),
        ("TkMenuSummonCloseWindow", 0x008c_cb40, tk::SUMMON_WINDOW, 0),
        (
            "TkMn2CloseSummonGrowWindow",
            0x008c_d430,
            tk::SUMMON_GROW_WINDOW,
            2,
        ),
        (
            "TkMenuSummonCloseWindow2",
            0x008c_d440,
            tk::SUMMON_WINDOW2,
            0,
        ),
        ("TkMenuEquipCloseWindow", 0x008c_f970, tk::EQUIP_WINDOW, 0),
        ("TkMenuLimitCloseWindow", 0x008d_0c90, tk::LIMIT_WINDOW, 0),
        ("TkMenuLimitCloseWindow2", 0x008d_0cb0, tk::LIMIT_WINDOW2, 0),
        (
            "TkMenuKaizouCloseWindow2",
            0x008d_57d0,
            tk::KAIZOU_WINDOW2,
            0,
        ),
        ("TkMenuKaizouCloseWindow", 0x008d_57f0, tk::KAIZOU_WINDOW, 0),
        ("TkMenuWShopCloseWindow", 0x008d_9b10, tk::WSHOP_WINDOW, 1),
        ("TkMenuWShopCloseWindow3", 0x008d_9b90, tk::WSHOP_WINDOW3, 1),
        ("TkMenuWShopCloseWindow2", 0x008d_9bb0, tk::WSHOP_WINDOW2, 1),
        (
            "TkMenuCloseSubCmdWindow1",
            0x008e_3710,
            tk::MENU_SUB_CMD_WINDOW1,
            0,
        ),
        ("TkMenuItemClearWindow", 0x008c_93a0, tk::ITEM_WINDOW, 3),
        ("TkMenuItemClearWindow2", 0x008c_93c0, tk::ITEM_WINDOW2, 3),
        ("TkMenuEquipClearWindow", 0x008c_fd40, tk::EQUIP_WINDOW, 3),
        ("TkMenuLimitClearWindow", 0x008d_1b50, tk::LIMIT_WINDOW, 3),
        ("TkMenuLimitClearWindow2", 0x008d_1b70, tk::LIMIT_WINDOW2, 3),
    ];
    for _ in 0..60 {
        for &(name, sym, g, kind) in &windows {
            let win = if t.rng.below(4) == 0 {
                0
            } else {
                t.alloc_random(0x60)
            };
            t.put(t.sym(g.sym), win);
            t.twin(name, sym, &[], U32, |o| {
                w(match kind {
                    3 => tk::clear_window(o, g),
                    k => tk::close_window(o, g, k == 1, k != 2),
                })
            });
        }
        let win = if t.rng.below(4) == 0 {
            0
        } else {
            t.alloc_random(0x60)
        };
        t.twin("TkMenuRestartWindow", 0x008d_1b90, &[win], U32, |o| {
            w(tk::tk_menu_restart_window(o, win))
        });
        for g in [tk::MENU_SUB_CMD_WINDOW, tk::MENU_SUB_CMD_WINDOW1] {
            let v = if t.rng.below(4) == 0 {
                0
            } else {
                t.alloc_random(0x60)
            };
            t.put(t.sym(g.sym), v);
        }
        t.twin("TkMenuRestartSubCmdWindow1", 0x008e_49f0, &[], U32, |o| {
            w(tk::tk_menu_restart_sub_cmd_window1(o))
        });
        let k = t.rng.int();
        t.twin("TkMenuSetSubCmdWindowCursor", 0x008e_3b80, &[k], U32, |o| {
            #[allow(clippy::cast_possible_truncation)]
            w(tk::tk_menu_set_sub_cmd_window_cursor(o, k as u16))
        });
        // fades
        let win = t.alloc_random(0x60);
        if t.rng.below(4) == 0 {
            t.put(win + 0x52, 0u16);
        }
        if t.rng.below(4) == 0 {
            t.put(win + 0x54, 0u16);
        }
        t.twin("TkMenuWindowFadedOut", 0x008d_3200, &[win], U32, |o| {
            w(u32::from(tk::tk_menu_window_faded_out(o, win)))
        });
        t.twin("TkMenuWindowStepFade", 0x008a_b1f0, &[win], U32, |o| {
            w(tk::tk_menu_window_step_fade(o, win))
        });
        // a fresh window: the fade out of range, the step sometimes 0
        let win = t.alloc_random(0x60);
        if t.rng.below(3) == 0 {
            t.put(win + 0x54, 0u16);
        }
        t.twin("TkMn2WindowStepFade", 0x008d_26d0, &[win], U32, |o| {
            w(tk::tk_mn2_window_step_fade(o, win))
        });
    }
    // the item lists: the game's inventory, then random ones
    let ids = t.sym(ffx_battle::save::bridge::ITEM_IDS.sym);
    let nums = t.sym(ffx_battle::save::bridge::ITEM_COUNTS.sym);
    for round in 0..40 {
        if round > 0 {
            for k in 0..ffx_battle::save::ITEM_SLOTS {
                let id = t
                    .rng
                    .pick_or_any(&[0x2000, 0x2001, 0x2010, 0x2040, 0x2060, 0x20ff, 0xffff]);
                #[allow(clippy::cast_possible_truncation)]
                t.put(ids + 2 * k, (id & 0x20ff) as u16);
                let n = if t.rng.below(3) == 0 {
                    0u8
                } else {
                    t.rng.bytes(1)[0]
                };
                t.put(nums + k, n);
            }
            t.scramble(tk::MENU_SUMMON_ENABLE_MASK.sym, 4);
        }
        for mode in [0u32, 4, 6, 1, 5, 0xffff_ffff] {
            t.twin("TkMenuItemRefresh", 0x008c_8e60, &[mode], VOID, |o| {
                tk::tk_menu_item_refresh(o, i(mode));
                0
            });
        }
    }
    let _ = t.a.call(t.a.rebase(0x008c_6790), Conv::Cdecl, &[]).unwrap();
    tk::tk_calc_important_list_max(&mut t.b);
    for n in [None, Some(0u32), Some(1), Some(5)] {
        if let Some(n) = n {
            t.put(t.sym(tk::MENU_IMPORTANT_LIST_MAX.sym), n);
        }
        t.twin("TkMenuItemImportantRefresh", 0x008c_8ba0, &[], VOID, |o| {
            tk::tk_menu_item_important_refresh(o);
            0
        });
    }
    // healing and ability map ids from the kernel tables
    for id in (0..0x180u32).chain([0x2000, 0x3001, 0xffff]) {
        for chr in [0u32, 1, 7, 0x11, 0x100] {
            t.twin("Cmd_HealAmount", 0x008c_6920, &[id, chr], U32, |o| {
                s(tk::cmd_heal_amount(o, id, chr))
            });
        }
        t.twin(
            "TkGetAbilityMapIdFromItemId",
            0x008c_67e0,
            &[id],
            U32,
            |o| w(u32::from(tk::tk_get_ability_map_id_from_item_id(o, id))),
        );
    }
    // scene names the game asks for (from 0x008d0cd0), and one it lacks
    let out = t.alloc(&[0; 8]);
    for name in [
        0x00b6_5e60u32,
        0x00b6_5ea0,
        0x00b6_5eac,
        0x00b6_5e70,
        0x00b6_5eb8,
        0x00b6_5ec8,
        0x00b6_5e7c,
    ] {
        let p = t.sym(name);
        t.twin("TkMn2SeqGetPos", 0x008c_2b50, &[p, out], VOID, |o| {
            tk::tk_mn2_seq_get_pos(o, p, out);
            0
        });
        t.twin("TkMn2SeqGetRect", 0x008c_2b90, &[p, out], VOID, |o| {
            tk::tk_mn2_seq_get_rect(o, p, out);
            0
        });
        t.twin("TkMn2SeqGetColor", 0x008c_2ad0, &[p], U32, |o| {
            w(tk::tk_mn2_seq_get_color(o, 1, p))
        });
        t.twin("TkMn2SeqGetNum", 0x008c_2b10, &[p], U32, |o| {
            s(tk::tk_mn2_seq_get_num(o, 1, p))
        });
        for k in [0u32, 1, 2, 3] {
            t.twin("TkMn2SeqGetColorCustom", 0x008c_2af0, &[k, p], U32, |o| {
                w(tk::tk_mn2_seq_get_color(o, i(k), p))
            });
            t.twin("TkMn2SeqGetNumCustom", 0x008c_2b30, &[k, p], U32, |o| {
                s(tk::tk_mn2_seq_get_num(o, i(k), p))
            });
            t.twin(
                "TkMn2SeqGetPosCustom",
                0x008c_2b70,
                &[k, p, out],
                VOID,
                |o| {
                    tk::tk_mn2_seq_get_pos_custom(o, i(k), p, out);
                    0
                },
            );
            t.twin(
                "TkMn2SeqGetRectCustom",
                0x008c_2bc0,
                &[k, p, out],
                VOID,
                |o| {
                    tk::tk_mn2_seq_get_rect_custom(o, i(k), p, out);
                    0
                },
            );
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn tk_round3_matches_the_original() {
    let Some(mut t) = setup_any(0x9a11_0005) else {
        return;
    };
    let event = ffx_battle::save::bridge::ms_get_save_event_address(&t.b);
    for _ in 0..500 {
        // pads
        let (a, b) = (t.rng.below(3), t.rng.below(2));
        let mode = t.rng.int();
        t.twin(
            "TkSetControlerActuatorMode",
            0x0088_94d0,
            &[a, b, mode],
            U32,
            |o| {
                #[allow(clippy::cast_possible_truncation)]
                w(tk::tk_set_controler_actuator_mode(
                    o,
                    i(a),
                    i(b),
                    mode as u8,
                ))
            },
        );
        let work = t.alloc_random(0x100);
        if t.rng.below(2) == 0 {
            t.put(work + 0x9d, 0u8);
        }
        t.twin(
            "TkUpdateControlerActuator",
            0x0088_9410,
            &[work],
            VOID,
            |o| {
                tk::tk_update_controler_actuator(o, work);
                0
            },
        );
        let k = t.rng.int();
        t.twin("TkGetControlerFrame", 0x0088_8b60, &[work, k], U32, |o| {
            w(tk::tk_get_controler_frame(o, work, i(k)))
        });
        let map: Vec<u8> = t.rng.bytes(64).into_iter().map(|c| c % 24).collect();
        let at = t.sym(tk::PAD_BUTTON_MAP.sym);
        t.put_bytes(at, &map);
        let (k, v) = (t.rng.below(16), t.rng.next());
        t.twin("TkSetPadButtonMap", 0x0088_94f0, &[a, b, k, v], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            tk::tk_set_pad_button_map(o, i(a), i(b), i(k), v as u8);
            0
        });
        let k = t.rng.below(24);
        t.twin("TkFindPadButton", 0x0088_9ac0, &[a, b, k], U32, |o| {
            s(tk::tk_find_pad_button(o, i(a), i(b), i(k)))
        });
        // vibration and callbacks
        let pad = t.rng.pick_or_any(&[0, 3, 7, 8, 0xffff_ffff]);
        t.twin("TkGetVibratePad", 0x0087_5e50, &[pad], U32, |o| {
            w(tk::tk_get_vibrate_pad(o, i(pad)))
        });
        let kind = t.rng.pick_or_any(&[0, 1, 2, 3, 4, 5, 6, 7]);
        t.twin("TkGetVibrateDefaultCurve", 0x0087_6390, &[kind], U32, |o| {
            w(tk::tk_get_vibrate_default_curve(o, kind))
        });
        t.scramble(tk::SYNC_V_CALLBACK_PTRS.sym, 12);
        let k = t.rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff]);
        t.twin("TkGetSyncVCallbackPtr", 0x0088_8650, &[k], U32, |o| {
            w(tk::tk_get_sync_v_callback_ptr(o, i(k)))
        });
        // debug and flags
        let (x, y) = (t.rng.int(), t.rng.int());
        t.twin("TkSetDebugFontLocate2", 0x0089_04e0, &[x, y], VOID, |o| {
            tk::tk_set_debug_font_locate2(o, i(x), i(y));
            0
        });
        t.scramble(tk::MENU_DUMMY_CONTROLER.sym, 0x14);
        t.twin("TkMenuClearDummyControler", 0x008c_a540, &[], VOID, |o| {
            tk::tk_menu_clear_dummy_controler(o);
            0
        });
        t.twin("TkSetDebugDrawFlag", 0x0088_ea20, &[x], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            tk::DEBUG_DRAW_FLAG.set(o, x as u8);
            0
        });
        t.scramble(tk::DECI2_SEND.sym, 1);
        t.twin("TkIsDeci2Send", 0x0088_5420, &[], U32, |o| {
            s(i32::from(tk::DECI2_SEND.get(o)))
        });
        // strings, widths, save files
        let len = 2 + t.rng.below(6) as usize;
        let mut digits: Vec<u8> = t.rng.bytes(len).into_iter().map(|c| c | 1).collect();
        digits.push(0);
        let sp = t.alloc(&digits);
        t.twin("TkParseLast2Digits", 0x0088_8280, &[sp], U32, |o| {
            s(tk::tk_parse_last_2_digits(o, sp))
        });
        let c = t.rng.pick_or_any(&[0, 0x41, 0xff, 0x100, 0x8140, 0xffff]) & 0xffff;
        for prop in [0u32, 1] {
            t.twin("TkGetCharWidth", 0x0089_0a00, &[prop, c], U32, |o| {
                s(tk::tk_get_char_width(o, i(prop), c))
            });
        }
        let k = t.rng.pick_or_any(&[0, 1, 0x62, 0x63, 0xffff_ffff]);
        t.twin("TkMenuGetSaveFileInfo", 0x008b_4e40, &[k], U32, |o| {
            w(tk::tk_menu_get_save_file_info(o, i(k)))
        });
        // the save block's event fields
        let v = t.rng.bytes(0xd0);
        t.put_bytes(event, &v);
        let (name, sym, rust): (&str, u32, fn(&mut Oracle, u32)) = match t.rng.below(3) {
            0 => ("TkBackupEventJump", 0x0088_e800, tk::tk_backup_event_jump),
            1 => (
                "TkRestoreEventJumpPrev",
                0x0088_e7c0,
                tk::tk_restore_event_jump_prev,
            ),
            _ => (
                "TkRestoreEventJumpMapIndex",
                0x0088_e7e0,
                tk::tk_restore_event_jump_map_index,
            ),
        };
        t.twin(name, sym, &[event], VOID, |o| {
            rust(o, event);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn gu_round3_matches_the_original() {
    let Some(mut t) = setup_any(0x9a11_0006) else {
        return;
    };
    for _ in 0..600 {
        let win = t.alloc_random(0xa0);
        let v = t.rng.int();
        let sets: [Set; 4] = [
            ("Gu_WindowSetX", 0x0084_b6e0, gu::gu_window_set_x),
            ("Gu_WindowSetY", 0x0084_b700, gu::gu_window_set_y),
            ("Gu_WindowSetW", 0x0084_b6b0, gu::gu_window_set_w),
            ("Gu_WindowSetH", 0x0084_b5d0, gu::gu_window_set_h),
        ];
        let (name, sym, rust) = t.rng.pick(&sets);
        t.twin(name, sym, &[win, v], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            rust(o, win, v as u16);
            0
        });
        for (field, name, sym) in [
            (0, "Gu_RectSetX", 0x0084_92f0),
            (1, "Gu_RectSetY", 0x0084_9310),
            (2, "Gu_RectSetW", 0x0084_92d0),
            (3, "Gu_RectSetH", 0x0084_92b0),
        ] {
            let v = t.rng.next();
            t.twin(name, sym, &[win, v], VOID, |o| {
                #[allow(clippy::cast_possible_truncation)]
                gu::gu_rect_set(o, win, field, v as u16);
                0
            });
        }
        t.twin("Gu_WindowSetFlag1", 0x0084_b680, &[win, v], VOID, |o| {
            gu::gu_window_set_flag1(o, win, v);
            0
        });
        t.twin("Gu_WindowToggleFlag1", 0x0084_b7b0, &[win], U32, |o| {
            s(gu::gu_window_toggle_flag1(o, win))
        });
        t.twin("Gu_WindowGetFlag2", 0x0084_af40, &[win], U32, |o| {
            s(gu::gu_window_get_flag2(o, win))
        });
        t.twin("Gu_WindowGetClientLeft", 0x0084_aef0, &[win], U32, |o| {
            s(gu::gu_window_field(o, win, 0x80))
        });
        t.twin("Gu_SelectorGetSelect", 0x0084_9d50, &[win], U32, |o| {
            s(gu::gu_selector_get_select(o, win))
        });
        // the mouse near a rectangle
        let rect = t.alloc_random(8);
        let rx = i32::from(Mem::read::<i16>(&t.b, rect));
        let ry = i32::from(Mem::read::<i16>(&t.b, rect + 2));
        let (dx, dy) = (
            t.rng.below(64).cast_signed() - 32,
            t.rng.below(64).cast_signed() - 32,
        );
        let near = |t: &mut Twins, x: i32| x.wrapping_add(t.rng.below(200).cast_signed() - 100);
        let mx = near(&mut t, rx.wrapping_add(dx));
        let my = near(&mut t, ry.wrapping_add(dy));
        t.put(t.sym(gu::MOUSE_X.sym), mx);
        t.put(t.sym(gu::MOUSE_Y.sym), my);
        if t.rng.below(2) == 0 {
            let small: Vec<u8> = t.rng.bytes(4).into_iter().map(|c| c & 0x7f).collect();
            t.put_bytes(rect + 4, &small);
        }
        t.twin(
            "Gu_MouseInRect",
            0x0084_9440,
            &[rect, dx.cast_unsigned(), dy.cast_unsigned()],
            U32,
            |o| w(u32::from(gu::gu_mouse_in_rect(o, rect, dx, dy))),
        );
        let (wd, ht) = (t.rng.below(300), t.rng.below(300));
        let args = [win, dx.cast_unsigned(), dy.cast_unsigned(), wd, ht];
        t.twin("Gu_MouseInClientRect", 0x0084_9b00, &args, U32, |o| {
            w(u32::from(gu::gu_mouse_in_client_rect(
                o,
                win,
                dx,
                dy,
                i(wd),
                i(ht),
            )))
        });
        // drawing
        let buf = t.alloc_random(0x200);
        t.twin("Gu_DrawStart", 0x0084_7b00, &[buf], VOID, |o| {
            gu::gu_draw_start(o, buf);
            0
        });
        let h = t.rng.pick_or_any(&[448, 512]);
        let wd = t.rng.pick_or_any(&[640, 512]);
        t.put(t.sym(gu::SCREEN_SIZE.sym), [h, wd]);
        t.scramble(gu::PACKET_ORIGIN.sym, 8);
        t.twin("Gu_DrawRectStrip", 0x0085_4f60, &[rect], VOID, |o| {
            gu::gu_draw_rect_strip(o, rect);
            0
        });
    }
    // floats and matrices
    for k in 0..3000 {
        let v: [f32; 6] = std::array::from_fn(|_| {
            if k % 5 == 0 {
                t.rng.pick(&[0.0, -0.0, 1e20, -3.5, f32::INFINITY])
            } else {
                t.rng.float(-1000.0, 1000.0)
            }
        });
        let r =
            t.a.call(t.a.rebase(0x0088_bc70), Conv::Cdecl, &v.map(f32::to_bits))
                .unwrap();
        same_float(
            "Math_Det3Points",
            r.st0,
            ffx_menu::math::det3_points(v[0], v[1], v[2], v[3], v[4], v[5]),
        );
        let n = t.rng.int();
        let r =
            t.a.call(t.a.rebase(0x008e_2690), Conv::Cdecl, &[n])
                .unwrap();
        same_float(
            "TkScreenToMenuX",
            r.st0,
            ffx_menu::math::screen_to_menu(i(n), 512.0, 1920.0),
        );
        let r =
            t.a.call(t.a.rebase(0x008e_26b0), Conv::Cdecl, &[n])
                .unwrap();
        same_float(
            "TkScreenToMenuY",
            r.st0,
            ffx_menu::math::screen_to_menu(i(n), 416.0, 1080.0),
        );
    }
    for _ in 0..20 {
        let m = t.alloc_random(64);
        t.twin("Math_UnitMatrix", 0x0088_c510, &[m], VOID, |o| {
            ffx_menu::math::bridge::math_unit_matrix(o, m);
            0
        });
    }
    // message flags and scene records
    for _ in 0..200 {
        let table = fake_table(&mut t, 16);
        let k = t.rng.below(16);
        t.twin("Msg_TableFlags", 0x0086_bf00, &[table, k], U32, |o| {
            w(u32::from(msg::msg_table_flags(o, table, i(k))))
        });
    }
    let params: [&[u8]; 3] = [b"cursor", b"item", b""];
    let scenes: [&[u8]; 3] = [b"main", b"status", b"ma"];
    for round in 0..40 {
        let (block, ph, sh) = scene_block(&mut t, &params, &scenes);
        let slot = t.rng.below(8);
        t.put(
            t.sym(scene::SCENE_DATA.sym) + 4 * slot,
            if round % 5 == 0 { 0 } else { block },
        );
        t.twin("TkMenuGetSceneDataAdrs", 0x00a7_b910, &[slot], U32, |o| {
            w(scene::tk_menu_get_scene_data_adrs(o, i(slot)))
        });
        for &h in sh.iter().chain(&ph).chain(&[0, 4]) {
            t.twin("TkMenuGetSceneRecord", 0x00a7_b8f0, &[slot, h], U32, |o| {
                w(scene::tk_menu_get_scene_record(o, i(slot), h))
            });
            t.twin("TkMenuGetSceneOffsetX", 0x00a7_b9d0, &[slot, h], U32, |o| {
                s(scene::tk_menu_get_scene_field(o, i(slot), h, 8))
            });
            t.twin("TkMenuGetSceneOffsetY", 0x00a7_ba00, &[slot, h], U32, |o| {
                s(scene::tk_menu_get_scene_field(o, i(slot), h, 0xa))
            });
            let (x, y) = (t.rng.next(), t.rng.next());
            t.twin(
                "TkMenuSetSceneOffset",
                0x00a7_c810,
                &[slot, h, x, y],
                VOID,
                |o| {
                    #[allow(clippy::cast_possible_truncation)]
                    scene::tk_menu_set_scene_pos(o, i(slot), h, x as u16, y as u16);
                    0
                },
            );
            for n in [0u32, 1] {
                t.twin(
                    "TkMenuGetParamOffset2",
                    0x00a7_b890,
                    &[slot, h, n],
                    U32,
                    |o| w(scene::tk_menu_get_param_offset(o, i(slot), i(h), i(n))),
                );
            }
        }
        for name in params.iter().chain(&scenes) {
            let q = t.alloc(&[name, &[0][..]].concat());
            t.twin("TkMenuSceneHash", 0x008e_5cf0, &[q], U32, |o| {
                w(scene::tk_menu_scene_hash(o, q))
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

/// A menu window for the slot and callback functions: random bytes with
/// callbacks at `+0`, `+8`, `+0xc`, `+0x10` picked among null and two
/// small originals (`MenuEquip_WindowIdle`, `TkMenuRestartSelFileWindow`).
fn menu_window(t: &mut Twins) -> u32 {
    let w = t.alloc_random(0x100);
    let fns = [0, t.sym(0x008c_f950), t.sym(0x008b_5530)];
    for off in [0u32, 8, 0xc, 0x10] {
        let f = t.rng.pick(&fns);
        t.put(w + off, f);
    }
    let active = t.rng.pick(&[0u8, 1, 2]);
    t.put(w + 0x18, active);
    w
}

/// A getter case: name, original, bridge.
type Getter4 = (&'static str, u32, fn(&Oracle) -> u32);

fn callback(o: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    o.call(f, Conv::Cdecl, args).unwrap().eax
}

#[test]
fn tk_round4_matches_the_original() {
    let Some(mut t) = setup_any(0x7a4c_e019) else {
        return;
    };
    let event = ffx_battle::save::bridge::ms_get_save_event_address(&t.b);
    for _ in 0..200 {
        // Tk library state
        let v = t.rng.bytes(0xd0);
        t.put_bytes(event, &v);
        for sym in [
            tk::SND_LAST_KICK_ID.sym,
            tk::SND_FOOT_KICK_STAY.sym,
            tk::SOUND_SYNC_V_FLAG3.sym,
            tk::MC_DEBUG_LOCK.sym,
            tk::HDD_ERROR_FLAG.sym,
            tk::DEBUG_MENU_STATUS.sym,
            tk::MN2_LIST_MAX.sym,
            tk::MN2_NAME_MODE_MAX.sym,
            tk::MENU_DUMMY_CONTROLER.sym,
            tk::MENU_ITEM_ROWS.sym,
            tk::MENU_EQUIP_ALT_MODE.sym,
            tk::MN2_DRAW_Y.sym,
            tk::TUTO_CHOICE.sym,
            tk::TUTO_CHOSEN.sym,
        ] {
            t.scramble(sym, 4);
        }
        if t.rng.below(2) == 0 {
            t.put(t.sym(tk::MENU_EQUIP_ALT_MODE.sym), 0u32);
            t.put(t.sym(tk::TUTO_CHOSEN.sym), 0u32);
        }
        t.scramble(tk::START_MAP_EVENT.sym, 16);
        t.scramble(tk::MC_LOAD_FILE_POS.sym, 16);
        t.scramble(tk::BTL_END_RESULTS.sym, 0x50);
        let getters: [Getter4; 19] = [
            ("TkSndGetLastKickId", 0x0088_7ad0, |o| {
                tk::tk_snd_get_last_kick_id(o)
            }),
            ("TkGetSoundSyncVFlag3", 0x0088_7df0, |o| {
                tk::tk_get_sound_sync_v_flag3(o)
            }),
            ("TkGetCommonVibrate", 0x0088_d590, |o| {
                tk::tk_get_common_vibrate(o)
            }),
            ("TkGetDebugMenuStatus", 0x0088_d5b0, |o| {
                tk::tk_get_debug_menu_status(o)
            }),
            ("TkGetEventNewGame", 0x0088_d5d0, |o| {
                u32::from(tk::tk_get_event_new_game(o))
            }),
            ("TkGetHddErrorFlag", 0x0088_d5f0, |o| {
                tk::tk_get_hdd_error_flag(o)
            }),
            ("TkGetLastEventJumpId", 0x0088_d600, |o| {
                u32::from(tk::tk_get_last_event_jump_id(o))
            }),
            ("TkGetLastEventJumpIndex", 0x0088_d610, |o| {
                u32::from(tk::tk_get_last_event_jump_index(o))
            }),
            ("TkGetLastEventJumpMapIdNo", 0x0088_d620, |o| {
                u32::from(tk::tk_get_last_event_jump_map_id_no(o))
            }),
            ("TkGetLastEventJumpMapNo", 0x0088_d630, |o| {
                u32::from(tk::tk_get_last_event_jump_map_no(o))
            }),
            ("TkGetLocalStartMapEventId", 0x0088_d640, |o| {
                tk::LOCAL_START_MAP_EVENT.get(o)[0]
            }),
            ("TkGetLocalStartMapEventIndex", 0x0088_d650, |o| {
                tk::LOCAL_START_MAP_EVENT.get(o)[1]
            }),
            ("TkGetStartMapEventId", 0x0088_d6b0, |o| {
                tk::START_MAP_EVENT.get(o)[0]
            }),
            ("TkGetStartMapEventIndex", 0x0088_d6c0, |o| {
                tk::START_MAP_EVENT.get(o)[1]
            }),
            ("TkMenuGetTime", 0x008a_9c50, |o| tk::tk_menu_get_time(o)),
            ("TkMn2GetListMax", 0x008c_1bb0, |o| {
                tk::tk_mn2_get_list_max(o)
            }),
            ("TkMn2GetNameModeMax", 0x008c_1bc0, |o| {
                tk::tk_mn2_get_name_mode_max(o)
            }),
            ("MenuItem_GetRows", 0x008c_8a00, |o| {
                tk::MENU_ITEM_ROWS.get(o)
            }),
            ("MenuEquip_IsAltMode", 0x008c_fc30, |o| {
                u32::from(tk::MENU_EQUIP_ALT_MODE.get(o) != 0)
            }),
        ];
        for (name, sym, f) in getters {
            t.twin(name, sym, &[], U32, |o| w(f(o)));
        }
        for (name, sym) in [
            ("TkMenuGetDummyControler", 0x008c_a500),
            ("TkMenuGetDummyControlerR", 0x008c_a520),
            ("TkMenuGetDummyControlerT", 0x008c_a530),
        ] {
            t.twin(name, sym, &[], U32, |o| w(tk::MENU_DUMMY_CONTROLER.get(o)));
        }
        t.twin("MenuTuto_GetChoice", 0x008d_b840, &[], U32, |o| {
            s(tk::menu_tuto_get_choice(o))
        });
        t.twin("BtlEnd_GetGilWon", 0x008b_da20, &[], U32, |o| {
            w(tk::btl_end_get_result(o, 19))
        });
        for k in 0..4 {
            t.twin("TkMcGetLoadFilePos", 0x0088_8300, &[k], U32, |o| {
                w(tk::tk_mc_get_load_file_pos(o, k))
            });
        }
        for k in 0..19 {
            t.twin("BtlEnd_GetResult", 0x008b_da10, &[k], U32, |o| {
                w(tk::btl_end_get_result(o, k))
            });
        }
        // setters
        let (a, b) = (t.rng.int(), t.rng.int());
        t.twin("TkSndSetFootKickStay", 0x0088_7b40, &[a], U32, |o| {
            tk::tk_snd_set_foot_kick_stay(o, a);
            w(a)
        });
        t.twin("TkClearSoundSyncVFlag3", 0x0088_7d50, &[], VOID, |o| {
            tk::tk_clear_sound_sync_v_flag3(o);
            0
        });
        t.twin("TkMcSetDebugLock", 0x0088_8420, &[a], U32, |o| {
            tk::tk_mc_set_debug_lock(o, a);
            w(a)
        });
        let slot = t.rng.below(4);
        t.twin("TkMcSetLoadFilePos", 0x0088_8480, &[slot, a], U32, |o| {
            tk::tk_mc_set_load_file_pos(o, slot, a);
            w(slot)
        });
        t.twin("TkSetEventNewGame", 0x0088_eb40, &[a], U32, |o| {
            #[allow(clippy::cast_possible_truncation)]
            w(tk::tk_set_event_new_game(o, a as u8))
        });
        t.twin("TkSetHddErrorFlag", 0x0088_eb70, &[a], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            tk::HDD_ERROR_FLAG.set(o, a as u8);
            0
        });
        t.twin("TkSetLocalStartMapIndex", 0x0088_eb80, &[a, b], U32, |o| {
            tk::LOCAL_START_MAP_EVENT.set(o, [a, b]);
            w(b)
        });
        t.twin("TkSetStartMapIndex", 0x0088_ebf0, &[a, b], U32, |o| {
            tk::START_MAP_EVENT.set(o, [a, b]);
            w(b)
        });
        t.scramble(ffx_field::yi::DRAW_MASK.sym, 4);
        let on = t.rng.pick(&[0u32, 1, 0x100]);
        t.twin(
            "TkSetMapGroupDrawStatus",
            0x0088_eba0,
            &[a, on],
            VOID,
            |o| {
                tk::tk_set_map_group_draw_status(o, a, on);
                0
            },
        );
        t.twin("TkMenuSetHelpRatioDiff2", 0x008a_aee0, &[a], U32, |o| {
            tk::tk_menu_set_help_ratio_diff2(o, a);
            w(a)
        });
        t.twin("MenuHelp_SetRatio2", 0x008a_b270, &[], VOID, |o| {
            tk::tk_menu_set_help_ratio2(o);
            0
        });
        t.twin("MenuTuto_SetChoice", 0x008d_b7a0, &[b, a], U32, |o| {
            s(tk::menu_tuto_set_choice(o, i(a)))
        });
        t.twin("MenuItem_SetCell", 0x008c_5260, &[a, b], VOID, |o| {
            tk::menu_item_set_cell(o, a, b);
            0
        });
        t.twin("MenuItem_SetTarget", 0x008c_5440, &[a], U32, |o| {
            tk::MENU_ITEM_TARGET.set(o, a);
            w(a)
        });
        t.twin("MenuSaveLoad_ResetToggles", 0x008b_4eb0, &[], VOID, |o| {
            tk::MENU_SAVE_LOAD_TOGGLES.set(o, [0, 0]);
            0
        });
        t.twin("BtlEnd_SetFlag", 0x008b_db60, &[], VOID, |o| {
            tk::BTL_END_FLAG.set(o, 1);
            0
        });
        t.twin("Pause_Request", 0x008a_ae00, &[], VOID, |o| {
            tk::PAUSE_REQUEST.set(o, 1);
            0
        });
        t.twin("MenuMn2_StepDrawY", 0x008c_0a10, &[], VOID, |o| {
            let v = tk::MN2_DRAW_Y.get(o).wrapping_add(0x10);
            tk::MN2_DRAW_Y.set(o, v);
            0
        });
        // Gu
        t.twin("Gu_FontSetX", 0x0084_82f0, &[a], U32, |o| {
            gu::PRINT_X.set(o, i(a));
            w(a)
        });
        t.twin("Gu_FontSetY", 0x0084_8300, &[a], U32, |o| {
            gu::PRINT_Y.set(o, i(a));
            w(a)
        });
        t.twin("Gu_PrintSetCR", 0x0084_8730, &[a], U32, |o| {
            gu::PRINT_CR.set(o, a);
            w(a)
        });
        t.twin("Gu_MouseSetEnable", 0x0084_9b80, &[a], U32, |o| {
            gu::MOUSE_ENABLE.set(o, a);
            w(a)
        });
        let win = t.alloc_random(0xa0);
        t.twin("Gu_WindowGetHScale", 0x0084_af20, &[win], U32, |o| {
            s(gu::gu_window_get_h_scale(o, win))
        });
        t.twin("Gu_WindowSetHScale", 0x0084_b5b0, &[win, a], U32, |o| {
            #[allow(clippy::cast_possible_truncation)]
            w(gu::gu_window_set_h_scale(o, win, a as u16))
        });
        t.twin("Gu_WindowSetWord30", 0x0084_b7e0, &[win, a], U32, |o| {
            #[allow(clippy::cast_possible_truncation)]
            w(gu::gu_window_set_word30(o, win, a as u16))
        });
        // pure helpers
        t.twin("MenuMn2_Plus80", 0x008c_22f0, &[a], U32, |_| {
            w(ffx_menu::tk::plus_80(a))
        });
        t.twin("MenuItem_Less", 0x008c_6a60, &[a, b], U32, |_| {
            w(u32::from(ffx_menu::tk::less(i(a), i(b))))
        });
        t.twin("BtlEnd_RowX", 0x008b_a370, &[a, b], U32, |_| {
            s(ffx_menu::tk::btl_end_row_x(i(a), i(b)))
        });
        t.twin("MenuHelp_Ratio", 0x008b_f120, &[a, b], U32, |_| {
            s(ffx_menu::tk::menu_help_ratio(i(a), i(b)))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_slots_windows_and_party_match_the_original() {
    let Some(mut t) = setup(0x5e1f_77a3) else {
        return;
    };
    // party fields
    for chr in (0..0x12u32).chain([0x100, 0x111, 0xff05]) {
        for round in 0..6 {
            let ply = ffx_battle::save::bridge::ms_get_save_player_ptr(&t.b, chr);
            let v = t.rng.bytes(0x94);
            if round > 0 {
                t.put_bytes(ply, &v);
            }
            if round == 1 {
                t.put(ply + 0x1c, 0u32);
            }
            for (name, sym, off) in [
                ("TkMenuGetWeapon", 0x008a_9c70, 0x2d),
                ("TkMenuGetArmor", 0x008a_9820, 0x2e),
                ("TkMn2GetOverDriveId", 0x008c_1bd0, 0x38),
                ("TkMenuGetLimit", 0x008a_9930, 0x39),
                ("TkMenuGetLimitMax", 0x008a_9950, 0x3a),
            ] {
                t.twin(name, sym, &[chr], U32, |o| {
                    w(u32::from(tk::tk_menu_get_ply_byte(o, chr, off)))
                });
            }
            t.twin("TkMn2GetOverDriveType", 0x008c_1bf0, &[chr], U32, |o| {
                w(tk::tk_menu_get_ply(o, chr, 0x88))
            });
            for (name, sym) in [("TkMenuGetSP", 0x008a_9b80), ("BtlEnd_GetSP", 0x008b_da50)] {
                t.twin(name, sym, &[chr], U32, |o| {
                    w(ffx_battle::save::bridge::ms_get_save_sp(o, chr))
                });
            }
            t.scramble(tk::MENU_MAIN_IGNORE_JOIN.sym, 4);
            if t.rng.below(2) == 0 {
                t.put(t.sym(tk::MENU_MAIN_IGNORE_JOIN.sym), 0u32);
            }
            t.twin("MenuPlayer_DamageClass", 0x008a_99f0, &[chr], U32, |o| {
                w(tk::menu_player_damage_class(o, chr, true))
            });
            t.twin("MenuPlayer_DamageClass2", 0x008a_9a70, &[chr], U32, |o| {
                w(tk::menu_player_damage_class(o, chr, false))
            });
            let v = t.rng.int();
            let hp = if t.rng.below(3) == 0 { 0 } else { v };
            t.twin("TkMenuSetHP", 0x008a_ae90, &[chr, hp], U32, |o| {
                w(tk::tk_menu_set_hp(o, chr, hp))
            });
            t.twin("TkMenuSetMP", 0x008a_aef0, &[chr, v], U32, |o| {
                w(tk::tk_menu_set_ply(o, chr, 0x20, v))
            });
            t.twin("TkMenuSetSP", 0x008a_b000, &[chr, v], U32, |o| {
                #[allow(clippy::cast_possible_truncation)]
                w(tk::tk_menu_set_ply_byte(o, chr, 0x3b, v as u8))
            });
            t.twin("TkMn2SetOverDriveId", 0x008c_2c90, &[chr, v], U32, |o| {
                #[allow(clippy::cast_possible_truncation)]
                w(tk::tk_menu_set_ply_byte(o, chr, 0x38, v as u8))
            });
        }
    }
    for _ in 0..300 {
        // aeon totals, option table, item list
        t.scramble(tk::MENU_SUMMON_TOTALS.sym, 20);
        let n = t.rng.pick(&[0i32, 1, 5, 20, -3]);
        t.put(t.sym(tk::MENU_SUMMON_TOTAL_MAX.sym), n);
        let k = t
            .rng
            .pick_or_any(&[0, 1, 4, 19, 20, 0xffff_ffff, 0x8000_0000]);
        let k = if (0x100..0x8000_0000).contains(&k) {
            k & 0x1f
        } else {
            k
        };
        t.twin(
            "TkMenuGetSummonTotalFromIndex",
            0x008a_9c00,
            &[k],
            U32,
            |o| w(u32::from(tk::tk_menu_get_summon_total_from_index(o, i(k)))),
        );
        let opt = t.rng.below(32);
        t.twin("MenuMain_OptionY", 0x008e_2260, &[opt], U32, |o| {
            s(tk::menu_main_option_y(o, opt))
        });
        t.scramble(tk::MENU_SUMMON_GROW_MAX.sym, 1);
        let g = t.rng.below(8);
        t.twin("MenuSummon_GrowEntry", 0x008c_ecf0, &[g], U32, |o| {
            w(tk::menu_summon_grow_entry(o, g))
        });
        let item = t.rng.below(0x200);
        let list = t.rng.pick(&[0u32, 1, 3, 4]);
        if t.rng.below(4) == 0 {
            let e = t.sym(tk::MENU_ITEM_LIST.sym) + 4 * item;
            let id = t.rng.pick(&[0u16, 0xff]);
            t.put(e, id);
            t.put(e + 3, 0xffu8);
        }
        for (name, sym) in [
            ("MenuItem_UsableByte", 0x008c_6a30),
            ("MenuItem_UsableByte2", 0x008c_6a40),
            ("MenuWShop_UsableByte", 0x008d_9b00),
        ] {
            t.twin(name, sym, &[list, item], U32, |o| {
                w(u32::from(tk::menu_item_usable_byte(o, item)))
            });
        }
        t.twin(
            "MenuKaizou_ItemUsable",
            0x008d_5730,
            &[list, item],
            U32,
            |o| w(u32::from(tk::menu_kaizou_item_usable(o, item))),
        );
        t.twin(
            "MenuWShop_ItemPresent",
            0x008d_75d0,
            &[list, item],
            U32,
            |o| w(u32::from(tk::menu_wshop_item_present(o, item))),
        );
        let list = t.rng.pick(&[0u32, 1, 2, 3, 4, 0xffff_ffff]);
        t.scramble(tk::MENU_IMPORTANT_COUNTER.sym, 4);
        if t.rng.below(2) == 0 {
            t.put(t.sym(tk::MENU_IMPORTANT_COUNTER.sym), 0u32);
        }
        t.twin("MenuItem_IsUsable", 0x008c_9120, &[0, list], U32, |o| {
            w(tk::menu_item_is_usable(o, i(list)))
        });
        // list entries and windows
        let entry = t.alloc_random(0x40);
        let kk = t.rng.below(8);
        t.put(entry + 0xb, u8::try_from(kk).unwrap());
        let v = t.rng.pick(&[0u16, 0xff, 1, 0x100]);
        t.put(entry + 0xc + 2 * kk, v);
        t.twin("MenuList_IsEmptySlot", 0x008b_f720, &[entry], U32, |o| {
            w(tk::menu_list_is_empty_slot(o, entry))
        });
        let win = t.alloc_random(0x100);
        let v = t.rng.pick(&[0i16, 1, -1, 0x200, 0x7fff, -0x8000]);
        t.put(win + 0x46, v);
        t.put(win + 0x4e, v);
        t.twin("MenuSubCmd_CursorAtTop", 0x008e_4a90, &[win], U32, |o| {
            w(u32::from(tk::menu_sub_cmd_cursor_at_top(o, win)))
        });
        t.twin("MenuSubCmd_CenterX", 0x008e_4ad0, &[win], U32, |o| {
            s(tk::menu_sub_cmd_center_x(o, win))
        });
        for (name, sym) in [
            ("MenuEquip_WindowIdle", 0x008c_f950),
            ("MenuLimit_WindowIdle", 0x008d_0e80),
            ("MenuKaizou_WindowIdle", 0x008d_56d0),
        ] {
            t.twin(name, sym, &[win], U32, |o| w(tk::window_idle(o, win)));
        }
        // window globals: null or a random window
        let globals = [
            tk::WORLD_MAP_WINDOW,
            tk::WSHOP_WINDOWS[0],
            tk::WSHOP_WINDOWS[1],
            tk::WSHOP_WINDOWS[2],
            tk::WSHOP_WINDOWS[3],
            tk::WSHOP_WINDOWS[4],
            tk::SUMMON_STATUS_WINDOW,
            tk::SUMMON_WINDOW,
            tk::SUMMON_GROW_WINDOW,
            tk::SUMMON_WINDOW2,
            tk::KAIZOU_WINDOW2,
            tk::TUTO_WINDOWS[0],
            tk::TUTO_WINDOWS[1],
            tk::MENU_SUB_CMD_WINDOW,
            tk::MENU_SUB_CMD_WINDOW2,
        ];
        for g in globals {
            let w0 = if t.rng.below(4) == 0 {
                0
            } else {
                t.alloc_random(0x100)
            };
            t.put(t.sym(g.sym), w0);
        }
        for k in 0..8 {
            let w0 = if t.rng.below(3) == 0 {
                0
            } else {
                t.alloc_random(0x60)
            };
            t.put(t.sym(tk::STATUS_WINDOWS.sym) + 4 * k, w0);
        }
        let sub = tk::MENU_SUB_CMD_WINDOW.get(&t.b);
        if sub != 0 {
            let c = t.rng.below(4);
            t.put(sub + 0x48, u16::try_from(c).unwrap());
        }
        t.scramble(tk::SUMMON_RESTART_COUNT.sym, 4);
        t.scramble(tk::MN2_STATUS_BG_STEP.sym, 8);
        t.scramble(tk::MENU_HELP_MESSAGE.sym, 12);
        t.twin("MenuWorldMap_CloseWindow", 0x008e_2b10, &[], U32, |o| {
            w(tk::close_window(o, tk::WORLD_MAP_WINDOW, false, true))
        });
        for (name, sym, k) in [
            ("MenuWShop_CloseSubWindow0", 0x008d_7610, 0),
            ("MenuWShop_CloseSubWindow1", 0x008d_7650, 1),
            ("MenuWShop_CloseSubWindow2", 0x008d_75f0, 2),
            ("MenuWShop_CloseSubWindow3", 0x008d_7630, 3),
            ("MenuWShop_CloseSubWindow4", 0x008d_7670, 4),
        ] {
            t.twin(name, sym, &[], U32, |o| {
                w(tk::close_window(o, tk::WSHOP_WINDOWS[k], true, true))
            });
        }
        t.twin(
            "MenuSummon_CloseStatusWindow",
            0x008c_cb10,
            &[],
            VOID,
            |o| {
                tk::menu_summon_close_status_window(o);
                0
            },
        );
        t.twin("MenuSummon_RestartWindow", 0x008c_d1f0, &[], VOID, |o| {
            tk::menu_summon_restart_window(o);
            0
        });
        for (name, sym, g) in [
            (
                "MenuSummon_RestartGrowWindow",
                0x008c_ee30,
                tk::SUMMON_GROW_WINDOW,
            ),
            ("MenuSummon_RestartWindow2", 0x008c_ee50, tk::SUMMON_WINDOW2),
            ("MenuKaizou_RestartWindow2", 0x008d_6e00, tk::KAIZOU_WINDOW2),
            ("MenuTuto_RestartWindow", 0x008d_aaa0, tk::TUTO_WINDOWS[0]),
        ] {
            t.twin(name, sym, &[], U32, |o| w(tk::restart_global_window(o, g)));
        }
        t.twin("MenuTuto_RestartWindow2", 0x008d_aac0, &[], U32, |o| {
            w(tk::menu_tuto_restart_window2(o))
        });
        let shop = t.alloc_random(0x100);
        let shop = if t.rng.below(4) == 0 { 0 } else { shop };
        t.put(t.sym(tk::WSHOP_WINDOWS[0].sym), shop);
        t.twin("MenuWShop_RestartListWindow", 0x008d_8420, &[], U32, |o| {
            w(tk::menu_wshop_restart_list_window(o))
        });
        let k = t.rng.below(8);
        t.twin("MenuStatus_CloseWindow", 0x008d_2680, &[k], U32, |o| {
            w(tk::menu_status_close_window(o, k))
        });
        t.twin("TkMenuRefreshSubCmdHelp", 0x008e_49b0, &[], VOID, |o| {
            tk::menu_sub_cmd_refresh_help(o);
            0
        });
        let v = t.rng.next();
        t.twin("MenuSubCmd_SetValue", 0x008e_4a10, &[v], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            tk::menu_sub_cmd_set_value(o, tk::MENU_SUB_CMD_WINDOW, v as u16);
            0
        });
        t.twin("MenuSubCmd_SetValue2", 0x008e_4a30, &[v], VOID, |o| {
            #[allow(clippy::cast_possible_truncation)]
            tk::menu_sub_cmd_set_value(o, tk::MENU_SUB_CMD_WINDOW2, v as u16);
            0
        });
        // menu slots with callbacks
        t.scramble(tk::MN2_MENU_MODE.sym, 4);
        if t.rng.below(2) == 0 {
            let m = 0x80_0000 | t.rng.below(0x1_0000);
            t.put(t.sym(tk::MN2_MENU_MODE.sym), m);
        }
        t.scramble(tk::MENU_MAIN_ACTIVE_MASK.sym, 4);
        for k in 0..25 {
            let w0 = if t.rng.below(4) == 0 {
                0
            } else {
                menu_window(&mut t)
            };
            t.put(t.sym(tk::MENU_SLOTS.sym) + 4 * k, w0);
        }
        let slot = t.rng.pick_or_any(&[0, 1, 0x16, 0x18, 0x19, 0xffff_ffff]);
        let arg = t.rng.next();
        t.twin(
            "TkMenuMainActiveMenu",
            0x008a_a100,
            &[slot, arg],
            VOID,
            |o| {
                tk::tk_menu_main_active_menu(o, i(slot), arg, &mut callback);
                0
            },
        );
        let slot = t.rng.pick_or_any(&[0, 1, 0x18, 0x19, 0xffff_ffff]);
        t.twin("TkMenuMainSleepMenu", 0x008a_ad70, &[slot], VOID, |o| {
            tk::tk_menu_main_sleep_menu(o, i(slot), &mut callback);
            0
        });
        let mw = menu_window(&mut t);
        let slot = t.rng.pick_or_any(&[0, 3, 0x18, 0x19, 0xffff_ffff]);
        t.twin("TkMenuMainRegistMenu", 0x008a_aab0, &[slot, mw], U32, |o| {
            w(tk::tk_menu_main_regist_menu(o, i(slot), mw, &mut callback))
        });
        let mw = menu_window(&mut t);
        t.twin("TkMenuMainRegistWindow", 0x008a_ab00, &[mw], U32, |o| {
            w(tk::tk_menu_main_regist_window(o, mw, &mut callback))
        });
        // pad work
        let pad = t.rng.below(2);
        let port = t.rng.below(2);
        let work = ffx_menu::tk::bridge::tk_get_controler_work(&t.b, i(pad), i(port));
        let v = t.rng.bytes(0x100);
        t.put_bytes(work, &v);
        let idx = t.rng.below(12) | (t.rng.next() & 0xffff_ff00);
        let idx = idx & 0xff;
        t.twin(
            "TkGetControlerActuatorMode",
            0x0088_8ba0,
            &[pad, port],
            U32,
            |o| w(u32::from(tk::pad_get_work_byte_9d(o, i(pad), i(port)))),
        );
        t.twin(
            "TkGetControlerAnalog",
            0x0088_8c00,
            &[pad, port, idx],
            U32,
            |o| w(tk::tk_get_controler_analog(o, i(pad), i(port), idx)),
        );
        t.twin("TkGetControlerId", 0x0088_8d90, &[pad, port], U32, |o| {
            w(tk::tk_get_controler_id(o, i(pad), i(port)))
        });
        t.twin("TkGetControlerPress", 0x0088_8dd0, &[pad, port], U32, |o| {
            w(u32::from(tk::tk_get_controler_press(o, i(pad), i(port))))
        });
        let b = t.rng.next();
        t.twin(
            "Pad_SetActuatorFF",
            0x0088_9490,
            &[pad, port, b],
            U32,
            |o| {
                #[allow(clippy::cast_possible_truncation)]
                w(tk::pad_set_work_byte(o, i(pad), i(port), 0xff, b as u8))
            },
        );
        t.twin(
            "Pad_SetActuatorFE",
            0x0088_94b0,
            &[pad, port, b],
            U32,
            |o| {
                #[allow(clippy::cast_possible_truncation)]
                w(tk::pad_set_work_byte(o, i(pad), i(port), 0xfe, b as u8))
            },
        );
        t.twin("Pad_ResetAnalog", 0x0088_9000, &[work], U32, |o| {
            w(tk::pad_reset_analog(o, work, 0x84))
        });
        t.twin("Pad_ResetAnalogAt0C", 0x0088_9030, &[work], U32, |o| {
            w(tk::pad_reset_analog(o, work, 0xc))
        });
        let c = t.rng.next();
        t.twin(
            "Pad_SetActuatorPair",
            0x0088_9460,
            &[work, b, c],
            VOID,
            |o| {
                #[allow(clippy::cast_possible_truncation)]
                tk::pad_set_actuator_pair(o, work, b as u8, c as u8);
                0
            },
        );
        t.twin("Pad_ResetMode", 0x0088_8ee0, &[work], U32, |o| {
            w(tk::pad_reset_mode(o, work))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn tk_round5_matches_the_original() {
    let Some(mut t) = setup(0x1d3b_9e57) else {
        return;
    };
    let menu = ffx_battle::save::bridge::ms_get_save_menu(&t.b);
    let event = ffx_battle::save::bridge::ms_get_save_event_address(&t.b);
    let table = ffx_kernel::bridge::COMMAND.get(&t.b);
    for round in 0..200u32 {
        // memory card wrappers
        t.scramble(tk::MC_PORT.sym, 8);
        let tgs = t.rng.pick(&[0u8, 1]);
        t.put(t.sym(tk::TGS.sym), tgs);
        let a = t.rng.int();
        for (name, sym, n, sets) in [
            ("TkMcChdir", 0x0088_7fd0, 1, true),
            ("TkMcClose", 0x0088_8010, 0, false),
            ("TkMcDelete", 0x0088_8170, 1, true),
            ("TkMcFlush", 0x0088_81a0, 0, false),
            ("TkMcFormat", 0x0088_81c0, 0, true),
            ("TkMcGetDir", 0x0088_8200, 4, true),
            ("TkMcGetEntSpace", 0x0088_8240, 1, true),
            ("TkMcMkdir", 0x0088_8330, 1, true),
            ("TkMcOpen", 0x0088_8360, 2, true),
            ("TkMcRead", 0x0088_83a0, 0, false),
            ("TkMcRename", 0x0088_83c0, 2, true),
            ("TkMcSeek", 0x0088_8400, 0, false),
            ("TkMcSetFileInfo", 0x0088_8440, 3, true),
        ] {
            t.scramble(tk::MC_PORT.sym, 8);
            let args = [a, 1, 2, 3];
            t.twin(name, sym, &args[..n], U32, |o| w(tk::tk_mc_call(o, sets)));
        }
        t.twin("TkMcGetInfo", 0x0088_82c0, &[a, 0, 0, 0], U32, |o| {
            w(tk::tk_mc_get_info(o, a))
        });
        t.twin("TkVU1SyncPath", 0x0088_ec60, &[], U32, |_| 0);
        let buf = t.alloc_random(8);
        t.put(t.sym(tk::DEBUG_FONT_BUFFER.sym), buf);
        t.twin("TkGetDebugFontBuffer", 0x0088_edc0, &[], U32, |o| {
            w(tk::tk_get_debug_font_buffer(o))
        });
        // math
        let x = t.rng.int();
        t.twin("Math_CosTable", 0x0088_c120, &[x], U32, |o| {
            s(ffx_menu::math::bridge::math_sin_table(
                o,
                x.wrapping_add(0x2000),
            ))
        });
        t.twin("Math_SinTable2", 0x0088_c380, &[x], U32, |o| {
            s(ffx_menu::math::bridge::math_sin_table(o, x))
        });
        let x = t.rng.below(0x1100).wrapping_sub(0x40);
        t.twin("Math_SinLookup", 0x0088_e730, &[x], U32, |o| {
            w(tk::math_sin_lookup(o, i(x)))
        });
        t.twin("Math_CosLookup", 0x0088_e750, &[x], U32, |o| {
            w(tk::math_cos_lookup(o, i(x)))
        });
        let (c, tg, st) = (t.rng.int(), t.rng.int(), t.rng.int());
        if i(c).wrapping_sub(i(tg)) != i32::MIN {
            t.twin("Gu_Approach", 0x0084_9270, &[c, tg, st], U32, |_| {
                s(ffx_menu::gu::approach(i(c), i(tg), i(st)))
            });
        }
        // kernel lookups
        let id = t.rng.below(0x300);
        t.twin("TkMn2GetExcelData", 0x008c_1af0, &[id, table], U32, |o| {
            w(tk::tk_mn2_get_excel_data(o, i(id), table))
        });
        t.twin("TkMn2GetKaizou", 0x008c_1b70, &[], U32, |o| {
            w(tk::tk_mn2_get_kaizou(o))
        });
        t.twin("TkMn2GetKaizouMax", 0x008c_1b90, &[], U32, |o| {
            s(tk::tk_mn2_get_kaizou_max(o))
        });
        t.twin("TkMn2InitWork", 0x008c_1c50, &[], VOID, |o| {
            tk::tk_mn2_init_work(o);
            0
        });
        t.scramble(tk::MN2_DRAW_Y.sym, 4);
        t.twin(
            "MenuMn2_StepDrawY80",
            0x008c_0dd0,
            &[1, 2, 3, 4],
            U32,
            |o| w(tk::menu_mn2_step_draw_y80(o)),
        );
        // party
        let chr = t.rng.below(0x12);
        let ply = ffx_battle::save::bridge::ms_get_save_player_ptr(&t.b, chr);
        let v = t.rng.bytes(0x94);
        t.put_bytes(ply, &v);
        let n = t.rng.int();
        t.twin("MenuItem_AddMP", 0x008c_8e20, &[chr, n], U32, |o| {
            w(tk::menu_item_add_mp(o, chr, n))
        });
        t.scramble(tk::MENU_PLAYERS.sym, 8);
        let cnt = t.rng.pick(&[0u32, 1, 3, 7, 0xffff_ffff]);
        t.put(t.sym(tk::MENU_PLAYER_PARTY_MASK.sym), cnt);
        t.twin("MenuParty_RefreshMask", 0x008a_ae30, &[], VOID, |o| {
            tk::menu_party_refresh_mask(o);
            0
        });
        t.scramble(tk::MENU_MAIN_IGNORE_JOIN.sym, 4);
        t.twin(
            "TkMenuGetPlayerDamegedColor",
            0x008b_9eb0,
            &[chr],
            U32,
            |o| w(tk::menu_player_damage_class(o, chr, true)),
        );
        t.twin("BtlEnd_GetNextAP", 0x008b_da40, &[chr], U32, |o| {
            s(ffx_battle::menu::bridge::ms_get_next_ap(o, i(chr)))
        });
        // pad
        let (pad, port) = (t.rng.below(2), t.rng.below(2));
        let work = tk::tk_get_controler_work(&t.b, i(pad), i(port));
        let v = t.rng.bytes(0x100);
        t.put_bytes(work, &v);
        let k = t.rng.int();
        t.twin("Pad_GetFrame", 0x0088_8b80, &[pad, port, k], U32, |o| {
            w(tk::pad_get_frame(o, i(pad), i(port), i(k)))
        });
        // config callbacks
        let m = t.rng.next();
        t.put(menu, m);
        let lang = t.rng.pick(&[0i8, 1, 2, 0x12]);
        t.put(t.sym(tk::LANG.sym), lang);
        let opt = t.alloc_random(0x10);
        if round % 2 == 0 {
            t.put(opt + 8, 0u32);
        }
        t.twin("TkRefreshActuator", 0x0087_2480, &[], U32, |o| {
            w(tk::pad_refresh_actuator_mode(o))
        });
        t.twin("MenuConfig_SetBit9", 0x008c_adb0, &[opt], U32, |o| {
            w(tk::menu_config_set_bit9(o, opt))
        });
        t.twin("MenuConfig_GetBit5", 0x008c_ade0, &[opt], U32, |o| {
            w(tk::menu_config_get_bit5(o, opt))
        });
        for (name, sym, bit, inv) in [
            ("MenuConfig_GetBit1", 0x008c_ae40, 1, false),
            ("MenuConfig_GetBit10", 0x008c_ae80, 10, true),
            ("MenuConfig_GetBit11", 0x008c_aed0, 11, true),
            ("MenuConfig_GetBit6", 0x008c_af20, 6, true),
        ] {
            t.twin(name, sym, &[opt], U32, |o| {
                w(tk::menu_config_get_bit(o, opt, bit, inv))
            });
        }
        for (name, sym, mask, on0) in [
            ("MenuConfig_SetBit5", 0x008c_ae20, 0x20, true),
            ("MenuConfig_SetBit1", 0x008c_ae60, 2, false),
            ("MenuConfig_SetBit10", 0x008c_aea0, 0x400, true),
            ("MenuConfig_SetBit11", 0x008c_aef0, 0x800, true),
            ("MenuConfig_SetBit6", 0x008c_af40, 0x40, true),
        ] {
            let z = t.rng.pick(&[0u32, 1, 0x100]);
            t.put(opt + 8, z);
            t.twin(name, sym, &[opt], U32, |o| {
                w(tk::menu_config_set_bits(o, opt, mask, on0))
            });
        }
        t.twin("MenuConfig_GetEnglish", 0x008c_afd0, &[opt], U32, |o| {
            w(tk::menu_config_get_english(o, opt))
        });
        let z = t.rng.pick(&[0u32, 1]);
        t.put(opt + 8, z);
        t.twin("MenuConfig_SetEnglish", 0x008c_aff0, &[opt], VOID, |o| {
            tk::menu_config_set_english(o, opt);
            0
        });
        let v = t.rng.next();
        t.put(event + 0x5ec, v);
        t.twin("MenuConfig_GetSaveFlag", 0x008c_b090, &[opt], U32, |o| {
            w(tk::menu_config_get_save_flag(o, opt))
        });
        t.twin("MenuConfig_GetBit8", 0x008c_b490, &[opt], U32, |o| {
            w(tk::menu_config_get_bit8(o, opt))
        });
        t.twin("MenuConfig_GetBit9", 0x008c_b500, &[opt], U32, |o| {
            w(tk::menu_config_get_bit9(o, opt))
        });
        // vibration streams
        let vp = t.alloc_random(0x30);
        for (o6, o16) in [
            (0x200u16, 0x200u16),
            (0xfffe, 0xfffe),
            (0x200, 0),
            (0xfffe, 0x200),
        ] {
            let st = t.rng.pick(&[0u8, 1, 2, 3]);
            t.put(vp + 0x22, st);
            t.put(vp + 6, o6);
            t.put(vp + 0x16, o16);
            t.twin("Vibrate_CheckDone", 0x0087_5d30, &[vp], U32, |o| {
                w(tk::vibrate_check_done(o, vp))
            });
        }
        if round % 3 == 0 {
            t.put(vp, 0u32);
        }
        t.twin("Vibrate_HoldStream", 0x0087_6160, &[vp], U32, |o| {
            w(tk::vibrate_hold_stream(o, vp))
        });
        let curve = t.alloc_random(8);
        t.twin("Vibrate_StartStream", 0x0087_6350, &[vp, curve], U32, |o| {
            w(tk::vibrate_start_stream(o, vp, curve))
        });
        t.twin("Vibrate_ResetStream", 0x0087_5f30, &[vp], U32, |o| {
            w(tk::vibrate_reset_stream(o, vp))
        });
        let vp2 = t.alloc_random(0x30);
        t.twin("Vibrate_ResetPad", 0x0087_5ef0, &[vp2], U32, |o| {
            w(tk::vibrate_reset_pad(o, vp2))
        });
        // item and equipment menus
        t.scramble(tk::MENU_ITEM_TARGET.sym, 4);
        t.scramble(tk::MENU_ITEM_KEY_USE.sym, 1);
        let list = t.rng.pick(&[0u32, 3, 4, 5, 0xffff_ffff]);
        t.twin("MenuItem_IsUsable2", 0x008c_3550, &[0, list], U32, |o| {
            w(tk::menu_item_is_usable2(o, i(list)))
        });
        let ew = t.alloc_random(0x60);
        let cur = t.rng.below(0x200);
        t.put(ew + 0x48, u16::try_from(cur).unwrap());
        if round % 4 == 0 {
            t.put(t.sym(tk::MENU_ITEM_LIST.sym) + 4 * cur, 0u16);
        }
        t.put(t.sym(tk::EQUIP_WINDOW.sym), ew);
        t.twin("MenuEquip_PickItem", 0x008d_0430, &[], U32, |o| {
            w(tk::menu_equip_pick_item(o))
        });
        let slw = t.alloc_random(0x60);
        t.put(t.sym(tk::SAVE_LOAD_WINDOW2.sym), slw);
        t.twin("MenuSaveLoad_Reset", 0x008b_4e70, &[a], U32, |o| {
            w(tk::menu_save_load_reset(o, a))
        });
        // Gu and small helpers
        let size = t.rng.int();
        t.twin("Gu_PrintSetMetrics", 0x0084_87e0, &[size], U32, |o| {
            s(gu::gu_print_set_metrics(o, i(size)))
        });
        let win = t.alloc_random(0xa0);
        t.twin("Gu_WindowGetWord8C", 0x0084_af60, &[win], U32, |o| {
            s(gu::gu_window_get_word8c(o, win))
        });
        let tab = t.alloc_random(0x40);
        let k = t.rng.below(8);
        for second in [0u32, 1, 7] {
            t.twin("Table_PairWord", 0x0087_25c0, &[tab, k, second], U32, |o| {
                w(tk::table_pair_word(o, tab, k, second != 0))
            });
        }
        let obj = t.alloc_random(0x30);
        let base = t.alloc_random(0x40);
        t.put(obj + 0x20, base);
        let r = t.alloc(&[0; 8]);
        t.put(r, obj);
        let off = 4 * t.rng.below(15);
        t.put(r + 4, off);
        t.twin("Ref_Deref", 0x0087_25a0, &[r], U32, |o| {
            w(tk::ref_deref(o, r))
        });
        t.twin("MenuStatus_ResetEntry", 0x008d_ea20, &[win], U32, |o| {
            w(tk::menu_status_reset_entry(o, win))
        });
        t.twin("MenuHdd_WindowReset", 0x008e_28e0, &[win], U32, |o| {
            w(tk::window_clear_1c(o, win))
        });
        t.scramble(tk::MENU_MAIN_TASK.sym, 4);
        t.twin("MenuMain_GetTask", 0x008a_9b90, &[], U32, |o| {
            w(tk::MENU_MAIN_TASK.get(o))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn tk_round6_matches_the_original() {
    let Some(mut t) = setup(0x6b2f_0d41) else {
        return;
    };
    let event = ffx_battle::save::bridge::ms_get_save_event_address(&t.b);
    for round in 0..150u32 {
        // pad history
        let (pad, port) = (t.rng.below(2), t.rng.below(2));
        let work = tk::tk_get_controler_work(&t.b, i(pad), i(port));
        let v = t.rng.bytes(0x100);
        t.put_bytes(work, &v);
        let typ = t.rng.pick(&[5u8, 4, 7]);
        t.put(work + 0x9f, typ);
        let m = t.rng.pick(&[8u8, 0, 1]);
        t.put(work + 0xf6, m);
        let fr = t.rng.int();
        for (name, sym, off) in [
            ("TkGetControlerHist", 0x0088_8d70, 4),
            ("TkGetControlerTrigHist", 0x0088_8e80, 6),
            ("TkGetControlerTrigRHist", 0x0088_8ea0, 8),
            ("TkGetControlerReptHist", 0x0088_8e50, 0xa),
            ("TkGetControlerRept2Hist", 0x0088_8e30, 0x1c),
        ] {
            t.twin(name, sym, &[pad, port, fr], U32, |o| {
                w(tk::tk_get_controler_hist(o, i(pad), i(port), i(fr), off))
            });
        }
        let k = t.rng.below(16) | (t.rng.next() & 0xffff_ff00);
        let k = k & 0xff0f;
        t.twin(
            "TkGetControlerAnalogHist",
            0x0088_8c20,
            &[pad, port, k, fr],
            U32,
            |o| {
                w(tk::tk_get_controler_analog_hist(
                    o,
                    i(pad),
                    i(port),
                    k,
                    i(fr),
                ))
            },
        );
        t.twin(
            "TkGetControlerActuatorNumber",
            0x0088_8bc0,
            &[pad, port],
            U32,
            |o| w(tk::tk_get_controler_actuator_number(o, i(pad), i(port))),
        );
        // ability list
        let chr = t.rng.below(7);
        if round % 3 == 0 {
            let ply = ffx_battle::save::bridge::ms_get_save_player_ptr(&t.b, chr);
            let v = t.rng.bytes(0x94);
            t.put_bytes(ply, &v);
        }
        if round < 20 {
            t.twin("MenuAbility_RefreshList", 0x008c_6800, &[chr], U32, |o| {
                s(tk::menu_ability_refresh_list(o, chr))
            });
        }
        // windows and lists
        let win = t.alloc_random(0xa0);
        t.twin("MenuMain_ResetWindow", 0x008a_a4b0, &[win], VOID, |o| {
            tk::menu_main_reset_window(o, win);
            0
        });
        let list = t.alloc_random(0x100);
        let (a, b) = (t.rng.below(8), t.rng.below(8));
        if round % 2 == 0 {
            let v = t.rng.bytes(0x1c);
            t.put_bytes(list + 32 * b, &v);
            t.put_bytes(list + 32 * a, &v);
        }
        t.twin(
            "MenuItem_CompareEntries",
            0x008c_9410,
            &[list, a, b],
            U32,
            |o| s(tk::menu_item_compare_entries(o, list, a, b)),
        );
        t.twin(
            "MenuItem_SwapEntries",
            0x008c_9460,
            &[list, a, b],
            VOID,
            |o| {
                tk::menu_item_swap_entries(o, list, a, b);
                0
            },
        );
        let items = t.sym(tk::MENU_ITEM_LIST.sym);
        let mut bytes = t.rng.bytes(0x80);
        for x in bytes.iter_mut().skip(3).step_by(4) {
            *x = u8::from(t.rng.below(4) == 0);
        }
        t.put_bytes(items, &bytes);
        let sw = t.alloc_random(0x60);
        let n = t.rng.pick(&[0u16, 1, 5, 20, 32]);
        t.put(sw + 0x30, n);
        let vis = t.rng.pick(&[0u16, 1, 4, 8, 40]);
        t.put(sw + 0x3a, vis);
        t.twin("MenuWShop_InitCursor", 0x008d_9580, &[sw], VOID, |o| {
            tk::menu_wshop_init_cursor(o, sw);
            0
        });
        // save/load slots
        let mut slots = t.rng.bytes(99);
        for x in &mut slots {
            *x = t.rng.pick(&[0u8, 0, 1, 2, 3]);
        }
        t.put_bytes(t.sym(tk::SAVE_SLOTS.sym), &slots);
        let (free, need) = (t.rng.below(20).wrapping_sub(2), t.rng.below(5));
        t.twin(
            "MenuSaveLoad_MarkSlots",
            0x008b_54c0,
            &[free, need],
            VOID,
            |o| {
                tk::menu_save_load_mark_slots(o, i(free), i(need));
                0
            },
        );
        // weapon shop
        let table = ffx_kernel::bridge::COMMAND.get(&t.b);
        let id = t.rng.below(0x40);
        t.put(t.sym(tk::WSHOP_LIST.sym), id);
        t.put(t.sym(tk::WSHOP_LIST.sym) + 0xc, table);
        t.twin("MenuWShop_CountSlots", 0x008d_b440, &[], VOID, |o| {
            tk::menu_wshop_count_slots(o);
            0
        });
        t.put(t.sym(tk::WSHOP_TABLE.sym), table);
        let id = t.rng.int();
        t.twin("MenuWShop_GetRecord", 0x008d_9170, &[id], U32, |o| {
            w(tk::menu_wshop_get_record(o, id))
        });
        // battle end
        t.scramble(tk::BTL_END_RESULTS.sym, 0x50);
        t.twin("BtlEnd_InitGilStep", 0x008b_df10, &[], VOID, |o| {
            tk::btl_end_init_gil_step(o);
            0
        });
        t.twin("BtlEnd_ApStep", 0x008b_a3c0, &[chr], U32, |o| {
            w(tk::btl_end_ap_step_for(o, i(chr)))
        });
        t.scramble(tk::BTL_END_LIST.sym, 0x40);
        let n = t.rng.pick(&[0u8, 1, 2, 3, 4, 0xff]);
        t.put(t.sym(tk::BTL_END_LIST.sym), n);
        t.twin("BtlEnd_CompactList", 0x008b_a400, &[], VOID, |o| {
            tk::btl_end_compact_list(o);
            0
        });
        // status layout
        let tt = t.rng.below(0x1100);
        let dir = t.rng.int();
        t.twin("MenuMn2_Ease", 0x008d_30c0, &[tt, dir], U32, |o| {
            w(tk::menu_mn2_ease(o, i(tt), i(dir)))
        });
        t.scramble(tk::STATUS_RECT_SIZE.sym, 4);
        t.scramble(tk::STATUS_ORIGIN.sym, 4);
        t.scramble(0x0186_a900, 0x68);
        let rect = t.alloc_random(0x18);
        let out = t.alloc_random(0x18);
        let (dx, dy, sized) = (t.rng.int(), t.rng.int(), t.rng.below(2));
        t.twin(
            "MenuStatus_OffsetRect",
            0x008d_3220,
            &[dx, dy, out, rect, sized],
            VOID,
            |o| {
                #[allow(clippy::cast_possible_truncation)]
                tk::menu_status_offset_rect(o, dx as u16, dy as u16, out, rect, sized != 0);
                0
            },
        );
        let alt = t.rng.below(2);
        t.twin("MenuStatus_GetPos", 0x008d_4e30, &[out, alt], VOID, |o| {
            tk::menu_status_get_pos(o, out, alt != 0, 0x0186_a900, 0x0186_a948);
            0
        });
        t.twin("MenuStatus_GetPos2", 0x008d_4e80, &[out, alt], VOID, |o| {
            tk::menu_status_get_pos(o, out, alt != 0, 0x0186_a918, 0x0186_a960);
            0
        });
        // HDD close through the main task's callback
        let task = t.alloc_random(0x10);
        let f = t.rng.pick(&[0, t.sym(0x008c_6a60)]);
        t.put(task + 8, f);
        t.put(
            t.sym(tk::MENU_MAIN_TASK.sym),
            if round % 4 == 0 { 0 } else { task },
        );
        if f != 0 || round % 4 == 0 {
            t.twin("MenuHdd_Close", 0x008e_2930, &[], VOID, |o| {
                tk::menu_hdd_close(o, &mut callback);
                0
            });
        }
        // packets and scissors
        let tab = t.alloc_random(0x200);
        if round < 3 {
            t.twin(
                "MenuSaveLoad_BuildCrcTable",
                0x008b_1370,
                &[tab],
                VOID,
                |o| {
                    tk::menu_save_load_build_crc_table(o, tab);
                    0
                },
            );
        }
        let (x, y, z) = (t.rng.next(), t.rng.next(), t.rng.next());
        t.twin("MenuDraw_GifTag", 0x008d_e0a0, &[tab, x, y, z], U32, |o| {
            w(tk::menu_draw_gif_tag(o, tab, x, y, z))
        });
        t.twin("TkMenuDrawInitString", 0x008d_ce30, &[], VOID, |o| {
            tk::tk_menu_draw_init_string(o);
            0
        });
        let (sy, sh, ratio) = (
            t.rng.below(0x200),
            t.rng.below(0x200),
            t.rng.below(0x2000).wrapping_sub(0x1000),
        );
        let mode = t.rng.pick(&[0u32, 1, 2, 3]);
        t.twin(
            "TkMn2DrawMenuWindowScissor",
            0x008c_0f70,
            &[sy, sh, mode, ratio],
            U32,
            |o| {
                w(tk::tk_mn2_draw_menu_window_scissor(
                    o,
                    i(sy),
                    i(sh),
                    i(mode),
                    i(ratio),
                ))
            },
        );
        for (name, sym) in [
            ("TkMn2DrawScissorClear", 0x008c_1370),
            ("TkMenuDrawScissorClear", 0x008d_d440),
        ] {
            t.twin(name, sym, &[], U32, |o| {
                w(ffx_system::pkt::bridge::draw_scissor_xywh(
                    o, 0, 0, 0x200, 0x1a0,
                ))
            });
        }
        // message windows
        let bank = t.rng.pick(&[0i8, 1]);
        t.put(t.sym(0x0132_6b82), bank);
        let wi = t.rng.pick_or_any(&[0, 3, 7, 8, 0xffff_ffff]);
        let wi = if wi > 7 && wi < 0x8000_0000 { 9 } else { wi };
        let at = t.sym(0x0132_6d70) + (wi.min(7) + u32::from(bank.cast_unsigned()) * 8) * 0x2c;
        if wi < 8 {
            let st = t.rng.pick(&[0u16, 3, 1, 2]);
            t.put(at + 0x14, st);
        }
        t.twin("Msg_CloseWindow", 0x0086_40f0, &[wi], VOID, |o| {
            msg::msg_close_window(o, i(wi), |o, k| {
                o.call(o.rebase(0x008a_b8f0), Conv::Cdecl, &[k.cast_unsigned()])
                    .unwrap();
            });
            0
        });
        let _ = event;
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn tk_round6b_matches_the_original() {
    let Some(mut t) = setup_any(0x30a1_77c5) else {
        return;
    };
    for round in 0..150u32 {
        // menu window records
        let win = t.alloc_random(0xa0);
        t.twin("TkMenuMainFreeWindow", 0x008a_a3f0, &[win], VOID, |o| {
            tk::tk_menu_main_free_window(o, win);
            0
        });
        let win = t.alloc_random(0xa0);
        t.twin("MenuMain_ResetWindowRet", 0x008a_a180, &[win], U32, |o| {
            tk::menu_main_reset_window(o, win);
            w(win)
        });
        if round < 3 {
            t.scramble(tk::MENU_WINDOWS.sym, 0x98 * 32);
            t.twin("MenuMain_ResetAllWindows", 0x008a_a470, &[], VOID, |o| {
                tk::menu_main_reset_all_windows(o);
                0
            });
        }
        t.twin("TkMenuDrawKick", 0x008d_ce50, &[], U32, |o| {
            w(tk::tk_menu_draw_kick(o))
        });
        t.twin("TkMenuDrawInitString", 0x008d_ce30, &[], U32, |o| {
            w(tk::tk_menu_draw_init_string(o))
        });
        // script choice slots
        let ctx = Mem::read::<u32>(&t.b, t.sym(0x0132_6b28));
        if ctx != 0 {
            let slot = t.rng.pick_or_any(&[0, 1, 3, 4, 0xffff_ffff]);
            let s0 = ctx + (i(slot).clamp(0, 3).cast_unsigned() << 4);
            let target = t.alloc_random(4);
            let v = t.rng.pick(&[0u32, 5, 0xffff_ffff]);
            t.put(s0 + 0x64, v);
            t.put(s0 + 0x6c, target);
            let worker = t.rng.below(4);
            t.twin(
                "Msg_CancelChoice",
                0x0086_4150,
                &[worker, slot],
                VOID,
                |o| {
                    tk::msg_cancel_choice(o, i(worker), i(slot));
                    0
                },
            );
        }
        // debug font buffer
        let buf = t.alloc_random(0x10);
        t.put(t.sym(tk::DEBUG_FONT_BUFFER.sym), buf);
        let p = t.rng.next();
        t.twin("TkSetDebugFontBuffer", 0x0088_fab0, &[p], U32, |o| {
            w(tk::tk_set_debug_font_buffer(o, p))
        });
        // GS packers and helpers
        let a = [
            t.rng.next(),
            t.rng.next(),
            t.rng.next(),
            t.rng.next(),
            t.rng.next(),
        ];
        for (name, sym) in [("Gs_Pack4", 0x008d_e1b0), ("Gs_Pack4b", 0x008c_32e0)] {
            t.twin(name, sym, &a[..4], U64, |_| {
                ffx_menu::tk::gs_pack4(a[0], a[1], a[2], a[3])
            });
        }
        for (name, sym) in [
            ("Gs_Pack5", 0x008d_e1d0),
            ("Gs_Pack5b", 0x008c_3300),
            ("Gs_Pack5c", 0x0089_0a30),
            ("Gs_Pack5d", 0x008c_ad80),
        ] {
            t.twin(name, sym, &a, U64, |_| {
                ffx_menu::tk::gs_pack5(a[0], a[1], a[2], a[3], a[4])
            });
        }
        for (name, sym, add) in [
            ("MenuStatus_Plus10", 0x008d_e1a0, 0x10u32),
            ("MenuItem_Plus10", 0x008c_32d0, 0x10),
            ("MenuHelp_Plus40", 0x008b_f140, 0x40),
        ] {
            t.twin(name, sym, &a[..1], U32, |_| w(a[0].wrapping_add(add)));
        }
        for (name, sym) in [
            ("MenuStatus_Second", 0x008d_e200),
            ("MenuItem_Second", 0x008c_3330),
        ] {
            t.twin(name, sym, &a[..2], U32, |_| w(a[1]));
        }
        // floats
        let which = t.rng.pick(&[0u32, 1, 7]);
        let f = t.rng.float(-1000.0, 1000.0);
        t.twin(
            "Tk_SetScale",
            0x0087_2530,
            &[which, f.to_bits()],
            VOID,
            |o| {
                tk::tk_set_scale(o, which, f);
                0
            },
        );
        let r =
            t.a.call(t.a.rebase(0x0087_2360), Conv::Cdecl, &[which])
                .unwrap();
        same_float("Tk_GetScale", r.st0, tk::tk_get_scale(&t.b, which));
        let obj = t.alloc_random(0x30);
        let r =
            t.a.call(t.a.rebase(0x0087_2590), Conv::Cdecl, &[obj])
                .unwrap();
        same_float("Obj_GetFloat28", r.st0, tk::obj_get_float28(&t.b, obj));
        let ang = t.rng.next();
        let r =
            t.a.call(t.a.rebase(0x0088_c0a0), Conv::Cdecl, &[ang])
                .unwrap();
        same_float(
            "Math_AngleFrom12Bits",
            r.st0,
            tk::math_angle_from_bits(&t.b, ang, 0xfff, 0x00b5_15c8),
        );
        let r =
            t.a.call(t.a.rebase(0x0088_c0e0), Conv::Cdecl, &[ang])
                .unwrap();
        same_float(
            "Math_AngleFrom15Bits",
            r.st0,
            tk::math_angle_from_bits(&t.b, ang, 0x7fff, 0x00b5_1588),
        );
        // the getters write nothing but the stack
        t.a.clear_dirty();
        t.b.clear_dirty();
        // Gu windows
        let list = t.alloc(&[0; 4]);
        let mut wins = Vec::new();
        for _ in 0..4 {
            let wn = t.alloc_random(0xa0);
            let f = t.rng.pick(&[0, t.sym(0x008c_6a60)]);
            t.put(wn + 0x7c, f);
            let parent = if wins.is_empty() {
                0
            } else {
                t.rng.pick(&wins)
            };
            t.put(wn + 8, parent);
            wins.push(wn);
        }
        for k in 0..wins.len() {
            let next = wins.get(k + 1).copied().unwrap_or(0);
            t.put(wins[k], next);
        }
        let _ = list;
        t.put(t.sym(gu::WINDOW_LIST.sym), wins[0]);
        t.scramble(gu::MOUSE_X.sym, 8);
        t.twin("Gu_WindowStartDrag", 0x0084_a580, &[wins[0]], VOID, |o| {
            gu::gu_window_start_drag(o, wins[0]);
            0
        });
        t.twin("Gu_WindowCloseTree", 0x0084_ab80, &[wins[0]], VOID, |o| {
            gu::gu_window_close_tree(o, wins[0], &mut callback);
            0
        });
        t.twin("Gu_ListCopyNext", 0x0084_a600, &[wins[0]], VOID, |o| {
            gu::gu_list_copy_next(o, wins[0]);
            0
        });
        let wn = window(&mut t);
        let mx = t.rng.below(0x200);
        let my = t.rng.below(0x200);
        t.put(t.sym(gu::MOUSE_X.sym), mx);
        t.put(t.sym(gu::MOUSE_Y.sym), my);
        t.twin("Gu_WindowMouseIn", 0x0084_9490, &[wn], U32, |o| {
            w(gu::gu_window_mouse_in(o, wn))
        });
        let id = t.rng.pick(&[1u32, 2, 3, 0xffff, 0x1_0001]);
        let on = t.rng.next();
        t.twin(
            "Gu_WindowControlSetLeftText",
            0x0084_a6c0,
            &[wn, id, on],
            VOID,
            |o| {
                #[allow(clippy::cast_possible_truncation)]
                gu::gu_window_control_set_left_text(o, wn, id as u16, on);
                0
            },
        );
        let r = [
            t.rng.pick_or_any(&[0xffff_ffff]),
            t.rng.pick_or_any(&[0xffff_ffff]),
            t.rng.int(),
            t.rng.int(),
        ];
        t.twin(
            "Gu_WindowControlSetRect",
            0x0084_96d0,
            &[wn, id, r[0], r[1], r[2], r[3]],
            VOID,
            |o| {
                gu::gu_window_control_set_rect(o, wn, id, r.map(i));
                0
            },
        );
        t.twin("Gu_MouseSetWindow", 0x0084_9b90, &[wn], U32, |o| {
            s(gu::gu_mouse_set_window(o, wn))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_sprite_packets_match_the_original() {
    let Some(mut t) = setup_any(0x4d72_19b3) else {
        return;
    };
    let table = t.sym(0x00c5_a900);
    let entries = Mem::read::<u32>(&t.b, table);
    for round in 0..300u32 {
        let p = t.alloc_random(0x1000);
        let entry = t.rng.below(entries + 2);
        let x = t.rng.below(0x280).wrapping_sub(0x40);
        let y = t.rng.below(0x200).wrapping_sub(0x40);
        let (ox, oy) = (t.rng.below(0x100), t.rng.below(0x100));
        let alpha = t.rng.below(0x100);
        let args = [p, 0, x, y, entry, ox, oy, alpha];
        t.twin("MenuDraw_Sp3RBody", 0x008d_c1b0, &args, U32, |o| {
            w(tk::menu_draw_sprites(o, args, false))
        });
        t.twin("MenuDraw_Sp3UBody", 0x008d_c710, &args, U32, |o| {
            w(tk::menu_draw_sprites(o, args, true))
        });
        t.twin("MenuDraw_Sp3Body", 0x008d_bf20, &args, U32, |o| {
            w(tk::menu_draw_sprites_plain(o, args))
        });
        for (name, sym, z, ox, al) in [
            ("MenuDraw_SpriteP", 0x008d_bd40, 0, 0, None),
            ("MenuDraw_SpritePA", 0x008d_bd70, 0, 0, Some(alpha)),
            ("MenuDraw_SpriteP8A", 0x008d_bda0, 0, 8, Some(alpha)),
            ("MenuDraw_SpriteP1", 0x008d_be30, 1, 0, None),
            ("MenuDraw_SpriteP1A", 0x008d_be60, 1, 0, Some(alpha)),
            ("MenuDraw_SpriteP2", 0x008d_be90, 2, 0, None),
            ("MenuDraw_SpriteP2A", 0x008d_bec0, 2, 0, Some(alpha)),
            ("MenuDraw_SpriteP8", 0x008d_bef0, 0, 8, None),
        ] {
            let mut a = vec![p, x, y, entry];
            a.extend(al);
            let all = [p, z, x, y, entry, ox, 0, al.unwrap_or(0x80)];
            t.twin(name, sym, &a, U32, |o| {
                w(tk::menu_draw_sprites_plain(o, all))
            });
        }
        // the g_TkMenuPacket wrappers over them
        for (name, sym, z, ox, al, v) in [
            ("TkMenuDrawSp3", 0x008d_d520, 0, 0, None, false),
            ("TkMenuDrawSp3A", 0x008d_d550, 0, 0, Some(alpha), false),
            ("TkMenuDrawSp3AHalf", 0x008d_d580, 0, 8, Some(alpha), false),
            ("TkMenuDrawSp3AUR", 0x008d_d5b0, 0, 0, Some(alpha), true),
            ("TkMenuDrawSp3AURHalf", 0x008d_d5e0, 0, 8, Some(alpha), true),
            ("TkMenuDrawSp3FaceB", 0x008d_d740, 2, 0, None, false),
            ("TkMenuDrawSp3FaceBA", 0x008d_d770, 2, 0, Some(alpha), false),
            ("TkMenuDrawSp3Half", 0x008d_d7a0, 0, 8, None, false),
        ] {
            let pk = t.alloc_random(0x1000);
            t.put(t.sym(tk::MENU_PACKET.sym), pk);
            let mut a = vec![x, y, entry];
            a.extend(al);
            let all = [pk + 0x10, z, x, y, entry, ox, 0, al.unwrap_or(0x80)];
            t.twin(name, sym, &a, U32, |o| {
                let e = if v {
                    tk::menu_draw_sprites(o, all, true)
                } else {
                    tk::menu_draw_sprites_plain(o, all)
                };
                tk::MENU_PACKET.set(o, e);
                w(e)
            });
        }
        let pk = t.alloc_random(0x1000);
        t.put(t.sym(tk::MENU_PACKET.sym), pk);
        t.twin("TkMenuDrawSp30", 0x008d_ca20, &[x, y], U32, |o| {
            let e = tk::menu_draw_sprites_plain(o, [pk + 0x10, 0, x, y, 0, 0, 0, 0x80]);
            tk::MENU_PACKET.set(o, e);
            w(e)
        });
        for (name, sym, z, ox, al, mirror) in [
            ("MenuDraw_Sprite", 0x008d_c0f0, 0, 0, None, false),
            ("MenuDraw_SpriteA", 0x008d_c120, 0, 0, Some(alpha), false),
            ("MenuDraw_Sprite1", 0x008d_c150, 1, 0, None, false),
            ("MenuDraw_Sprite2", 0x008d_c180, 2, 0, None, false),
            ("MenuDraw_SpriteV", 0x008d_c6b0, 0, 0, None, true),
            ("MenuDraw_SpriteV8", 0x008d_c6e0, 0, 8, None, true),
            ("MenuDraw_SpriteVA", 0x008d_bdd0, 0, 0, Some(alpha), true),
            ("MenuDraw_SpriteV8A", 0x008d_be00, 0, 8, Some(alpha), true),
        ] {
            let mut a = vec![p, x, y, entry];
            a.extend(al);
            let all = [p, z, x, y, entry, ox, 0, al.unwrap_or(0x80)];
            t.twin(name, sym, &a, U32, |o| {
                w(tk::menu_draw_sprites(o, all, mirror))
            });
        }
        // bars from a random cell
        let cell = t.alloc_random(0x10);
        let n = t.rng.pick(&[0u32, 1, 5, 0x40, 0xffff_fff0]);
        let m = t.rng.below(0x80);
        t.twin("MenuDraw_Bar", 0x008d_c360, &[p, n, cell, x, y], U32, |o| {
            w(tk::menu_draw_bar(o, 0, p, i(n), 0, cell, i(x), i(y)))
        });
        t.twin(
            "MenuDraw_BarV",
            0x008d_c480,
            &[p, n, cell, x, y],
            U32,
            |o| w(tk::menu_draw_bar(o, 1, p, i(n), 0, cell, i(x), i(y))),
        );
        t.twin(
            "MenuDraw_Bar2",
            0x008d_c5a0,
            &[p, m, n, cell, x, y],
            U32,
            |o| w(tk::menu_draw_bar(o, 2, p, i(n), i(m), cell, i(x), i(y))),
        );
        let (a, b, w0) = (t.rng.next(), t.rng.next(), t.rng.int());
        t.twin(
            "MenuDraw_Line",
            0x008c_a580,
            &[p, x, y, w0, a, b],
            U32,
            |o| w(tk::menu_draw_line(o, p, [i(x), i(y), i(w0), i(a), i(b)])),
        );
        let xs = t.alloc_random(0x40);
        let k = t.rng.pick(&[0u32, 1, 2, 7, 16, 0xffff_ffff]);
        let dx = t.rng.int();
        for (name, sym, rev) in [
            ("MenuDraw_Strip", 0x008c_a610, false),
            ("MenuDraw_StripR", 0x008c_a6f0, true),
        ] {
            t.twin(name, sym, &[p, x, y, xs, k, dx, a], U32, |o| {
                w(tk::menu_draw_strip(o, p, [x, y, xs, k, dx, a], rev))
            });
        }
        // the g_TkMenuPacket wrappers
        let pk = t.alloc_random(0x1000);
        t.put(t.sym(tk::MENU_PACKET.sym), pk);
        let pp = pk + 0x10;
        let adv = |o: &mut Oracle, end: u32| {
            tk::MENU_PACKET.set(o, end);
            w(end)
        };
        t.twin("TkMenuDrawSp3R", 0x008d_d7d0, &[x, y, entry], U32, |o| {
            let e = tk::menu_draw_sprites(o, [pp, 0, x, y, entry, 0, 0, 0x80], false);
            adv(o, e)
        });
        let pp = tk::MENU_PACKET.get(&t.b) + 0x10;
        t.twin(
            "TkMenuDrawSp3RA",
            0x008d_d800,
            &[x, y, entry, alpha],
            U32,
            |o| {
                let e = tk::menu_draw_sprites(o, [pp, 0, x, y, entry, 0, 0, alpha], false);
                adv(o, e)
            },
        );
        let pk = t.alloc_random(0x1000);
        t.put(t.sym(tk::MENU_PACKET.sym), pk);
        let pp = pk + 0x10;
        t.twin(
            "TkMenuDrawSp3StretchD",
            0x008d_d830,
            &[cell, x, y, n],
            U32,
            |o| {
                let e = tk::menu_draw_bar(o, 0, pp, i(n), 0, cell, i(x), i(y));
                adv(o, e)
            },
        );
        let pp = tk::MENU_PACKET.get(&t.b) + 0x10;
        t.twin(
            "TkMenuDrawSp3StretchR",
            0x008d_d860,
            &[cell, x, y, n],
            U32,
            |o| {
                let e = tk::menu_draw_bar(o, 1, pp, i(n), 0, cell, i(x), i(y));
                adv(o, e)
            },
        );
        let pp = tk::MENU_PACKET.get(&t.b) + 0x10;
        t.twin(
            "TkMenuDrawSp3StretchRD",
            0x008d_d890,
            &[cell, x, y, m, n],
            U32,
            |o| {
                let e = tk::menu_draw_bar(o, 2, pp, i(n), i(m), cell, i(x), i(y));
                adv(o, e)
            },
        );
        let pp = tk::MENU_PACKET.get(&t.b) + 0x10;
        t.twin("TkMenuDrawSp3Tex0", 0x008d_d8c0, &[a, b, x], U32, |o| {
            let e = tk::menu_draw_gif_tag(o, pp, a, b, x);
            adv(o, e)
        });
        let pk = t.alloc_random(0x1000);
        t.put(t.sym(tk::MENU_PACKET.sym), pk);
        let pp = pk + 0x10;
        t.twin("TkMenuDrawSp3UR", 0x008d_d8f0, &[x, y, entry], U32, |o| {
            let e = tk::menu_draw_sprites(o, [pp, 0, x, y, entry, 0, 0, 0x80], true);
            adv(o, e)
        });
        let pp = tk::MENU_PACKET.get(&t.b) + 0x10;
        t.twin(
            "TkMenuDrawSp3URHalf",
            0x008d_d920,
            &[x, y, entry],
            U32,
            |o| {
                let e = tk::menu_draw_sprites(o, [pp, 0, x, y, entry, 8, 0, 0x80], true);
                adv(o, e)
            },
        );
        // the option dispatch
        if round < 100 {
            t.scramble(tk::MN2_MENU_MODE.sym, 4);
            t.scramble(tk::MENU_MAIN_ACTIVE_MASK.sym, 4);
            for s0 in 0..25 {
                let w0 = if t.rng.below(4) == 0 {
                    0
                } else {
                    menu_window(&mut t)
                };
                t.put(t.sym(tk::MENU_SLOTS.sym) + 4 * s0, w0);
            }
            let opts = t.sym(tk::MENU_MAIN_OPTIONS.sym);
            let mut v = t.rng.bytes(0x100);
            for k in 0..16 {
                let m: u32 = t.rng.pick(&[0, 1, 2, 3, 4, 5, 7, 8, 10, 15, 20, 21]);
                v[8 * k..8 * k + 4].copy_from_slice(&m.to_le_bytes());
            }
            t.put_bytes(opts, &v);
            let opt = t.rng.below(16);
            t.twin("TkMenuMainOpenOption", 0x008e_24c0, &[opt], U32, |o| {
                w(tk::tk_menu_main_open_option(o, i(opt), &mut callback))
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn tk_round6c_matches_the_original() {
    let Some(mut t) = setup_any(0x5e17_a2c9) else {
        return;
    };
    for round in 0..200u32 {
        let v = t.rng.int();
        // setters: the value (or al over it) stays in eax
        for (name, sym, g, byte) in [
            ("Gu_SetPopupWindow", 0x0084_b670, 0x0130_3740, false),
            ("Atel_SetScriptByteGate", 0x0087_20d0, 0x00c5_26d4, false),
            ("TkSetVibrateOff", 0x0087_5f50, 0x0132_8272, true),
            ("AtelSetDebugDraw", 0x0088_5110, 0x0132_8a78, true),
            ("TkConfig_SetByteC", 0x0088_ec20, 0x0133_888c, true),
            (
                "TkMenuSetDrawShadeRatioStep",
                0x008a_9680,
                0x0184_0880,
                false,
            ),
            ("TkMenuSetScreenX2Flag", 0x008a_b020, 0x0184_0885, true),
        ] {
            let mask = if byte { 0xff } else { U32 };
            t.twin(name, sym, &[v], mask, |o| {
                if byte {
                    #[allow(clippy::cast_possible_truncation)]
                    Mem::write(o, o.rebase(g), v as u8);
                } else {
                    Mem::write(o, o.rebase(g), v);
                }
                w(v)
            });
        }
        t.twin("Tk_ByteHighBit", 0x0087_2580, &[v], U32, |o| {
            let _ = o;
            w(v & 0x80)
        });
        t.twin("TkDebugPrint", 0x0088_ecf0, &[v], U32, |_| 1);
        // debug font width
        let len = t.rng.below(24) as usize;
        let mut sb: Vec<u8> = t.rng.bytes(len).into_iter().map(|c| c.max(1)).collect();
        sb.extend([0, 0, 0]);
        let sp = t.alloc(&sb);
        for prop in [0u32, 1] {
            t.twin(
                "TkGetDebugFontStringWidth",
                0x0088_edf0,
                &[prop, sp],
                U32,
                |o| s(tk::tk_get_debug_font_string_width(o, i(prop), sp)),
            );
        }
        // pad history wrappers
        for pad in [0u32, 1] {
            let work = tk::tk_get_controler_work(&t.b, i(pad), 0);
            let bytes = t.rng.bytes(0x100);
            t.put_bytes(work, &bytes);
            let typ = t.rng.pick(&[5u8, 4, 7]);
            t.put(work + 0x9f, typ);
            let m = t.rng.pick(&[8u8, 0, 1]);
            t.put(work + 0xf6, m);
        }
        let fr = t.rng.int();
        for (name, sym, pad, off) in [
            ("TkGetDebugControler1Hist", 0x0088_5150, 0, 4),
            ("TkGetDebugControler1TrigHist", 0x0088_5170, 0, 6),
            ("TkGetDebugControlerHist", 0x0088_51b0, 1, 4),
            ("TkGetDebugControlerReptHist", 0x0088_51d0, 1, 0xa),
            ("TkGetDebugControlerTrigHist", 0x0088_51f0, 1, 6),
        ] {
            t.twin(name, sym, &[fr], U32, |o| {
                w(tk::tk_get_debug_controler_hist(o, pad, i(fr), off))
            });
        }
        let pad = t.rng.below(2);
        let port = 0;
        t.twin(
            "TkGetControlerIdHist",
            0x0088_8db0,
            &[pad, port, fr],
            U32,
            |o| w(tk::tk_get_controler_id_hist(o, i(pad), 0, i(fr))),
        );
        t.twin(
            "TkGetControlerPressHist",
            0x0088_8df0,
            &[pad, port, fr],
            U32,
            |o| w(tk::tk_get_controler_press_hist(o, i(pad), 0, i(fr))),
        );
        // debug font buffer word
        let buf = t.alloc_random(0x10);
        t.put(t.sym(tk::DEBUG_FONT_BUFFER.sym), buf);
        t.twin("TkDebugFont_SetWord4", 0x0088_fb10, &[v], U32, |o| {
            w(tk::tk_debug_font_set_word4(o, v))
        });
        // window groups
        if round % 20 == 0 {
            t.scramble(tk::MENU_WINDOWS.sym, 0x98 * 32);
            let base = t.sym(tk::MENU_WINDOWS.sym);
            for k in 0..32 {
                let used = t.rng.pick(&[1u8, 1, 0, 2]);
                t.put(base + k * 0x98 + 0x40, used);
                let g = t.rng.pick(&[0u8, 1, 2, 3]);
                t.put(base + k * 0x98 + 0x36, g);
            }
        }
        let group = t.rng.pick(&[0u32, 1, 2, 3, 0x100]);
        t.twin(
            "TkMenuMainFreeWindowGroup",
            0x008a_a410,
            &[group],
            VOID,
            |o| {
                tk::tk_menu_main_free_window_group(o, group);
                0
            },
        );
        // DMA tags
        let p = t.alloc_random(0x40);
        let end = p.wrapping_add(t.rng.below(0x300)).wrapping_sub(0x40);
        let (a, b, c) = (t.rng.next(), t.rng.next(), t.rng.next());
        t.twin("Dma_CloseCnt", 0x0089_0a60, &[p, end, a], U32, |o| {
            w(tk::dma_close_cnt(o, p, end, a))
        });
        t.twin("Dma_CloseCnt2", 0x0089_0aa0, &[p, end, a, b], U32, |o| {
            w(tk::dma_close_cnt2(o, p, end, a, b))
        });
        t.twin("Dma_CloseNext", 0x0089_0ae0, &[p, end, a, b, c], U32, |o| {
            w(tk::dma_close_next(o, p, end, a, b, c))
        });
        for (name, sym, id, with) in [
            ("Dma_PutCall", 0x0089_08c0, 0x50u8, true),
            ("Dma_PutEnd", 0x0089_0910, 0x70, false),
            ("Dma_PutCnt", 0x0089_0960, 0x10, false),
            ("Dma_PutRet", 0x0089_09b0, 0x60, false),
            ("Dma_PutCall2", 0x0089_0d90, 0x50, true),
        ] {
            let args: Vec<u32> = if with { vec![p, a] } else { vec![p] };
            t.twin(name, sym, &args, U32, |o| {
                w(tk::dma_put_tag(o, p, id, if with { a } else { 0 }))
            });
        }
        // menu window states
        let win = t.alloc_random(0x130);
        let y = t
            .rng
            .pick(&[0.0f32, -12.0, 1.5, 300.25, -1e30, f32::INFINITY]);
        t.put(win + 0x4c, y);
        t.twin("MenuWin_StateSlide", 0x008e_0200, &[win], U32, |o| {
            w(tk::menu_win_state_slide(o, win))
        });
        t.twin("MenuWin_StateOpen", 0x008e_0240, &[win], U32, |o| {
            w(tk::menu_win_state_open(o, win))
        });
        let c0 = t.rng.pick(&[0u8, 0x41]);
        let text = t.alloc(&[c0]);
        t.put(win + 0x98, text);
        t.twin("MenuWin_StateWaitText", 0x008e_0280, &[win], U32, |o| {
            w(tk::menu_win_state_wait_text(o, win))
        });
        t.twin("Yi_PutRef", 0x0091_6ad0, &[p, a, b], U32, |o| {
            w(tk::yi_put_ref(o, p, a, b))
        });
        // the float clamp
        let f = t.rng.pick(&[
            0.0f32,
            3.0,
            3.2,
            -3.2,
            -std::f32::consts::PI,
            std::f32::consts::PI,
            1e9,
            -1e9,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
        ]);
        let r =
            t.a.call(t.a.rebase(0x0088_b800), Conv::Cdecl, &[f.to_bits()])
                .unwrap();
        same_float("Math_ClampPi", r.st0, ffx_menu::math::clamp_pi(f));
        // TkMenuDrawSp3R0
        let pk = t.alloc_random(0x1000);
        t.put(t.sym(tk::MENU_PACKET.sym), pk);
        let (x, yy) = (t.rng.below(0x280), t.rng.below(0x200));
        t.twin("TkMenuDrawSp3R0", 0x008d_ca30, &[x, yy], U32, |o| {
            let e = tk::menu_draw_sprites(o, [pk + 0x10, 0, x, yy, 0, 0, 0, 0x80], false);
            tk::MENU_PACKET.set(o, e);
            w(e)
        });
    }
    eprintln!("{} calls", t.calls);
}

/// `user_free` replaced in both oracles by a recorder appending its
/// argument at `*log` (`log` the cursor's address); returns 0.
fn record_frees(t: &mut Twins) -> u32 {
    let buf = t.alloc(&[0; 0x1000]);
    let log = t.alloc(&buf.to_le_bytes());
    let l = log.to_le_bytes();
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04][..],              // mov eax, [esp + 4]
        &[0x8b, 0x0d], &l,                          // mov ecx, [log]
        &[0x89, 0x01],                              // mov [ecx], eax
        &[0x83, 0x05], &l, &[0x04],                 // add dword [log], 4
        &[0x31, 0xc0, 0xc3],                        // xor eax, eax; ret
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x006f_b8b0);
        o.write(at, &code).unwrap();
    }
    log
}

/// The recorder's effect, for the bridge side.
fn free_into(o: &mut Oracle, log: u32, p: u32) -> u32 {
    let at: u32 = Mem::read(o, log);
    Mem::write(o, at, p);
    Mem::write(o, log, at + 4);
    0
}

#[test]
fn gu_frees_match_the_original() {
    let Some(mut t) = setup_any(0x1f2e_3d4c) else {
        return;
    };
    let log = record_frees(&mut t);
    for _ in 0..100 {
        let c = t.alloc_random(0x40);
        let on = t.rng.pick(&[0u16, 0, 1, 0x100]);
        t.put(c + 0x26, on);
        t.twin("Gu_ControlFreeText", 0x0084_94f0, &[c], U32, |o| {
            w(gu::gu_control_free_text(o, c, &mut |o, p| {
                free_into(o, log, p)
            }))
        });
        let sel = t.alloc_random(0x40);
        let items = t.rng.pick(&[0u32, 0x1234_5678, 0x10]);
        t.put(sel + 8, items);
        t.twin("Gu_SelectorDispose", 0x0084_9bd0, &[sel], U32, |o| {
            w(gu::gu_selector_dispose(o, sel, &mut |o, p| {
                free_into(o, log, p)
            }))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// `UserHeap_Malloc` replaced in both oracles: records the size at `*log`
/// and returns the arena at `log + 4`'s value (`0x1000` bytes).
fn record_mallocs(t: &mut Twins) -> (u32, u32) {
    let buf = t.alloc(&[0; 0x1000]);
    let arena = t.alloc(&[0; 0x1000]);
    let log = t.alloc(&buf.to_le_bytes());
    let l = log.to_le_bytes();
    let ar = arena.to_le_bytes();
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04][..],              // mov eax, [esp + 4]
        &[0x8b, 0x0d], &l,                          // mov ecx, [log]
        &[0x89, 0x01],                              // mov [ecx], eax
        &[0x83, 0x05], &l, &[0x04],                 // add dword [log], 4
        &[0xb8], &ar, &[0xc3],                      // mov eax, arena; ret
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x006f_b910);
        o.write(at, &code).unwrap();
    }
    (log, arena)
}

#[test]
fn tk_round7_matches_the_original() {
    let Some(mut t) = setup_any(0x7a11_c0de) else {
        return;
    };
    let snapshot = std::env::var_os("FFX_SNAPSHOT").is_some();
    let (log, arena) = record_mallocs(&mut t);
    let flog = record_frees(&mut t);
    for round in 0..150u32 {
        let size = t.rng.int();
        t.twin("TkMalloc", 0x0088_e650, &[size], U32, |o| {
            w(tk::tk_malloc(o, size, &mut |o, n| {
                let _ = free_into(o, log, n);
                arena
            }))
        });
        let sel = t.alloc_random(0x20);
        let items = t.rng.pick(&[0u32, 0x10, 0x2000]);
        t.put(sel + 8, items);
        let n = t.rng.pick(&[0u32, 1, 5, 0x40, 0xffff_ffff, 0xffff_0003]);
        t.twin("Gu_SelectorResetNbItem", 0x0084_9f40, &[sel, n], U32, |o| {
            w(gu::gu_selector_reset_nb_item(
                o,
                sel,
                n,
                &mut |o, p| free_into(o, flog, p),
                &mut |o, n| {
                    let _ = free_into(o, log, n);
                    arena
                },
            ))
        });
        let bx = t.alloc_random(0x28);
        let r4 = [t.rng.int(), t.rng.int(), t.rng.int(), t.rng.int()];
        t.twin(
            "Gu_InitBox",
            0x0084_97f0,
            &[bx, r4[0], r4[1], r4[2], r4[3]],
            U32,
            |o| w(gu::gu_init_box(o, bx, r4)),
        );
        let base = t.alloc_random(0x40);
        let ctx = t.alloc(&[[0u8; 4], base.to_le_bytes()].concat());
        let worker = t.alloc_random(0x20);
        t.twin("Script_PcOffset", 0x0087_25e0, &[ctx, worker], U32, |o| {
            w(tk::script_pc_offset(o, ctx, worker))
        });
        let k = t
            .rng
            .pick(&[0u32, 3, 7, 8, 0x100, 0xffff_ffff, 0x8000_0000]);
        t.twin("Vibrate_BumpStream", 0x0087_5af0, &[k], U32, |o| {
            w(tk::vibrate_bump_stream(o, i(k)))
        });
        let bufs = [t.alloc_random(0x10), t.alloc_random(0x10)];
        t.put(t.sym(0x0133_c8c4), bufs);
        let cur = t.rng.pick(&[bufs[0], bufs[1], 0]);
        t.put(t.sym(tk::DEBUG_FONT_BUFFER.sym), cur);
        t.twin("TkDebugFont_SwapBuffer", 0x0089_0780, &[], U32, |o| {
            w(tk::tk_debug_font_swap_buffer(o))
        });
        let tp = t.alloc_random(0x30);
        let qwc = t.rng.pick(&[0u32, 1, 0x1_0000, 0x1234]);
        let bits = [t.rng.next(), t.rng.next(), t.rng.next()];
        let addr = t.rng.next();
        for (name, sym) in [
            ("Dma_PutTagEx", 0x0089_0860),
            ("Dma_PutTagEx2", 0x0089_0d30),
        ] {
            t.twin(
                name,
                sym,
                &[tp, qwc, bits[0], bits[1], bits[2], addr],
                U32,
                |o| w(tk::dma_put_tag_ex(o, tp, qwc, bits, addr)),
            );
        }
        let ent = t.alloc_random(0x10);
        let src = if t.rng.below(4) == 0 {
            0
        } else {
            t.alloc_random(4)
        };
        t.twin("Vibrate_SetStream", 0x0087_6230, &[ent, src], U32, |o| {
            w(tk::vibrate_set_stream(o, ent, src))
        });
        if round % 25 == 0 {
            t.scramble(0x0132_8150, 0x128);
            t.twin("Vibrate_ResetAll", 0x0087_5e80, &[], U32, |o| {
                w(tk::vibrate_reset_all(o))
            });
        }
        let ch = t.alloc_random(2);
        let c0 = t
            .rng
            .pick(&[0u8, 0x41, 0x7f, 0x80, 0xa4, 0xb0, 0xdf, 0xe0, 0xff]);
        t.put(ch, c0);
        let ob = t.alloc_random(2);
        t.twin("TkEucToSjis", 0x0089_07e0, &[ch, ob], U32, |o| {
            w(tk::tk_euc_to_sjis(o, ch, ob))
        });
        let pkt = t.alloc_random(0x100);
        t.put(t.sym(gu::PACKET_DATA.sym), pkt);
        let n0 = t.rng.below(8);
        t.put(t.sym(gu::PACKET_COUNT.sym), n0);
        let al = [
            t.rng.int(),
            t.rng.int(),
            t.rng.int(),
            t.rng.int(),
            t.rng.int(),
        ];
        t.twin("Gu_SetAlpha", 0x0084_83e0, &al, VOID, |o| {
            gu::gu_set_alpha(o, al.map(i));
            0
        });
        let fw = t.alloc_random(0x98);
        t.twin("Gu_FontSetupWindow", 0x0084_8310, &[fw], VOID, |o| {
            gu::gu_font_setup_window(o, fw);
            0
        });
        let v: Vec<f32> = (0..4)
            .map(|_| t.rng.pick(&[0.0f32, 1.0, -2.5, 3.0e-3, 1e20, -7.0, 1e-30]))
            .collect();
        let vb: Vec<u8> = v.iter().flat_map(|f| f.to_le_bytes()).collect();
        let vp = t.alloc(&vb);
        t.twin("TkNormalFVector", 0x0088_c5a0, &[vp], VOID, |o| {
            tk::tk_normal_f_vector(o, vp);
            0
        });
        // the scene-name table against a string
        let table = t.sym(0x00c5_b240);
        let k0 = t.rng.below(7);
        let name: Vec<u8> = (0..12)
            .map(|j| Mem::read::<u8>(&t.b, table + 0x10 * k0 + j))
            .collect();
        let mut text = t
            .rng
            .bytes(8)
            .into_iter()
            .map(|c| (c & 0x3f) + 0x30)
            .collect::<Vec<u8>>();
        if t.rng.below(2) == 0 {
            text.extend(name.iter().take_while(|&&c| c != 0));
        }
        text.push(0);
        t.put_bytes(t.sym(0x023c_bc60), &text);
        let id: u32 = if t.rng.below(2) == 0 {
            Mem::read(&t.b, table + 0x10 * k0 + 0xc)
        } else {
            t.rng.next()
        };
        t.put(t.sym(0x0132_5ac0), id);
        let kind = t.rng.pick(&[0xeu32, 0xe, 3]);
        t.twin(
            "hardCodeToFixTheWrongData",
            0x008e_02f0,
            &[0, kind],
            U32,
            |o| w(tk::hard_code_to_fix_the_wrong_data(o, i(kind))),
        );
        for (name, sym, g) in [
            ("Ch_DrawGetShadowMode", 0x0082_ec60u32, 0x0130_08bdu32),
            ("Ch_DrawGetShadowSemi", 0x0082_ec70, 0x0130_08be),
            ("TkGetSavedLang", 0x0088_c850, 0x0133_0874),
        ] {
            t.scramble(g, 1);
            t.twin(name, sym, &[], U32, |o| w(tk::get_byte(o, g)));
        }
        t.twin("Ch_RequestCalcNormal", 0x0083_2540, &[], VOID, |o| {
            tk::ch_request_calc_normal(o);
            0
        });
        let sc = t.rng.pick(&[0u32, 1, 0x40, 0x7f, 0x80, 0x100, 0xffff_ffff]);
        t.scramble(0x0186_63d0, 8);
        t.twin("Tk_NoteEternalCalm", 0x0088_ec90, &[sc], U32, |o| {
            w(tk::tk_note_eternal_calm(o, i(sc)))
        });
        let plate = t.alloc_random(0x10);
        t.twin("TkMn2DumpPlate", 0x008c_18a0, &[plate], U32, |o| {
            w(tk::tk_mn2_dump_plate(o, plate))
        });
        // the debug pad's analog bytes
        let work = tk::tk_get_controler_work(&t.b, 1, 0);
        let bytes = t.rng.bytes(0x100);
        t.put_bytes(work, &bytes);
        let k = t.rng.below(8);
        t.twin("TkGetDebugControlerAnalog", 0x0088_5190, &[k], U32, |o| {
            w(tk::tk_get_debug_controler_analog(o, k))
        });
        // window records
        if round % 10 == 0 {
            t.scramble(tk::MENU_WINDOWS.sym, 0x98 * 32);
            let base = t.sym(tk::MENU_WINDOWS.sym);
            let free = t.rng.below(40);
            for j in 0..32 {
                t.put(base + j * 0x98 + 0x40, u8::from(j != free));
            }
        }
        t.twin("TkMenuMainAllocWindow", 0x008a_a1a0, &[], U32, |o| {
            w(tk::tk_menu_main_alloc_window(o))
        });
        if snapshot {
            // window 0's macros
            let n = t.rng.below(0x40);
            let (b, val) = (t.rng.next(), t.rng.next());
            t.twin("TkMn2SetMacroParam", 0x008c_2c60, &[n, b, val], U32, |o| {
                w(tk::tk_mn2_set_macro_param(o, i(n), b, val))
            });
            // scene layout and scenes
            let hw = t.alloc_random(0x40);
            let st = t.rng.pick(&[0u32, 1, 1, 2]);
            t.put(hw + 0x1c, st);
            let fl = t.rng.pick(&[0u32, 1]);
            t.put(t.sym(0x023c_c130), fl);
            t.twin("TkMenuCtrlHdd", 0x008e_2870, &[hw], VOID, |o| {
                tk::tk_menu_ctrl_hdd(o, hw, &mut callback);
                0
            });
            if round % 10 == 0 {
                t.twin("TkMn2GetStatusPos", 0x008d_4920, &[], VOID, |o| {
                    tk::tk_mn2_get_status_pos(o);
                    0
                });
            }
            let pending = t.rng.pick(&[0xffff_ffffu32, 0, 3]);
            t.put(t.sym(0x00c3_3480), pending);
            t.twin("TkInitSceneFromSave", 0x0088_dd60, &[], VOID, |o| {
                tk::tk_init_scene_from_save(o);
                0
            });
            let owner = t.rng.pick(&[0u32, 1, 2, 0x10, 0xffff_ffff]);
            t.twin("Atel_FindWorkerByOwner", 0x0087_6440, &[owner], U32, |o| {
                s(tk::atel_find_worker_by_owner(o, owner))
            });
            let code = t.rng.pick(&[0u32, 3, 4, 5, 8, 11, 12, 0xffff_ffff]);
            let pp = t.rng.below(2);
            t.twin(
                "TkGetControlerBindAnalogIndex",
                0x0088_8cf0,
                &[pp, 0, code],
                U32,
                |o| s(tk::tk_get_controler_bind_analog_index(o, i(pp), 0, i(code))),
            );
            let sw = t.alloc_random(0x30);
            let wh = t.rng.pick(&[0u32, 0x40, 0x100, 0xffff_fff0]);
            t.put(sw + 0x1c, wh);
            let smode = t.rng.pick(&[0u32, 1, 2, 3]);
            let st = t
                .rng
                .pick(&[0u32, 0x800, 0x1000, 0xffff_f800, 0x8000_0000, 7]);
            t.twin(
                "TkMenuDrawMenuWindowScissorWithLabel",
                0x008d_d2d0,
                &[sw, smode, st],
                U32,
                |o| {
                    w(tk::tk_menu_draw_menu_window_scissor_with_label(
                        o,
                        sw,
                        i(smode),
                        i(st),
                    ))
                },
            );
            t.twin(
                "TkMenuDrawMenuWindowScissor",
                0x008d_d220,
                &[sw, smode, st],
                U32,
                |o| w(tk::tk_menu_draw_menu_window_scissor(o, sw, i(smode), i(st))),
            );
            let kw = t.alloc_random(0x40);
            t.twin("MenuKaizou_InitList", 0x008d_55e0, &[kw], VOID, |o| {
                tk::menu_kaizou_init_list(o, kw);
                0
            });
            t.twin("TkMn2GetMainPos", 0x008e_2630, &[], U32, |o| {
                w(tk::tk_mn2_get_main_pos(o))
            });
            let plate = t.alloc_random(0x18);
            let names = [0x00b6_67ecu32, 0x00b6_682c, 0x00b6_683c].map(|n| t.sym(n));
            let (pa, pb) = (t.rng.next(), t.rng.next());
            t.twin(
                "TkMn2GetCommonPlate",
                0x008c_1a60,
                &[plate, names[0], names[1], names[2], pa, pb],
                U32,
                |o| w(tk::tk_mn2_get_common_plate(o, 1, plate, names, pa, pb)),
            );
            let k = t.rng.below(4);
            t.twin(
                "TkMn2GetCommonPlateCustom",
                0x008c_1aa0,
                &[k, plate, names[0], names[1], names[2], pa, pb],
                U32,
                |o| w(tk::tk_mn2_get_common_plate(o, i(k), plate, names, pa, pb)),
            );
            let nm = t.rng.pick(&[0x00b6_67ecu32, 0x00b6_682c, 0x00b6_5e60]);
            let name = t.sym(nm);
            t.twin("TkMn2SeqStartScene", 0x008c_2bf0, &[name], U32, |o| {
                w(tk::tk_mn2_seq_start_scene(o, name))
            });
            t.twin("TkMn2SeqStopScene", 0x008c_2c10, &[name], U32, |o| {
                w(tk::tk_mn2_seq_stop_scene(o, name))
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_openers_match_the_original() {
    let Some(mut t) = setup(0x0be2_0be2) else {
        return;
    };
    for round in 0..120u32 {
        // a free window record somewhere among used ones
        let base = t.sym(tk::MENU_WINDOWS.sym);
        if round % 10 == 0 {
            t.scramble(tk::MENU_WINDOWS.sym, 0x98 * 32);
        }
        let free = t.rng.below(32);
        for j in 0..32 {
            t.put(base + j * 0x98 + 0x40, u8::from(j != free));
        }
        let arg = t.rng.next();
        let pick = t.rng.below(14);
        let args4 = [
            t.rng.next(),
            t.rng.pick(&[0u32, 1, 5]),
            t.rng.next(),
            t.rng.next(),
        ];
        let a3 = [args4[0], args4[1], args4[2]];
        for k in 0..2usize {
            let w0 = t.rng.pick(&[0u32, 0, 1]);
            let w0 = if w0 == 0 { 0 } else { t.alloc_random(0x98) };
            t.put(t.sym(tk::TUTO_WINDOWS[k].sym), w0);
        }
        match pick {
            0 => t.twin("MenuTuto_CloseWindow", 0x008d_9b30, &[], U32, |o| {
                w(tk::menu_tuto_close_window(o, 0))
            }),
            1 => t.twin("MenuTuto_CloseWindow2", 0x008d_9b60, &[], U32, |o| {
                w(tk::menu_tuto_close_window(o, 1))
            }),
            2 => t.twin("MenuSave_OpenWindow2", 0x008b_4fb0, &[arg], U32, |o| {
                w(tk::menu_save_open_window2(o, arg, &mut callback))
            }),
            3 => t.twin("MenuWorldMap_OpenWindow", 0x008e_2d60, &[], U32, |o| {
                w(tk::menu_world_map_open_window(o, &mut callback))
            }),
            4 => t.twin("TkMn2OpenSummonGrowWindow", 0x008c_ed30, &[], U32, |o| {
                w(tk::tk_mn2_open_summon_grow_window(o, &mut callback))
            }),
            5 => t.twin("MenuLimit_OpenWindow", 0x008d_0d50, &[arg], U32, |o| {
                w(tk::menu_limit_open_window(o, arg, &mut callback))
            }),
            _ => {
                let (name, sym, b37, pair, n): (&str, u32, bool, [u32; 2], usize) = match pick {
                    6 => ("TkMenuOpenSubCmdWindowMain", 0x008e_33d0, true, [1, 1], 4),
                    7 => ("MenuSubCmd_OpenMainA", 0x008e_3470, false, [0, 1], 4),
                    8 => ("MenuSubCmd_OpenMainB", 0x008e_3510, true, [1, 2], 4),
                    9 => ("MenuSubCmd_OpenMainC", 0x008e_35b0, true, [0, 2], 4),
                    10 => ("TkMenuOpenSubCmdWindow", 0x008e_33a0, true, [1, 1], 3),
                    11 => ("TkMenuOpenSubCmdWindow2", 0x008e_33c0, true, [1, 1], 4),
                    12 => ("MenuSubCmd_OpenA", 0x008e_3650, false, [0, 1], 3),
                    _ => ("MenuSubCmd_OpenC", 0x008e_3690, true, [0, 2], 3),
                };
                let args: &[u32] = if n == 4 { &args4 } else { &a3 };
                let full = if n == 4 {
                    args4
                } else {
                    [a3[0], a3[1], a3[2], 0]
                };
                t.twin(name, sym, args, U32, |o| {
                    w(tk::menu_sub_cmd_open(o, full, b37, pair, &mut callback))
                });
            }
        }
    }
    // MenuSubCmd_OpenB once directly
    let base = t.sym(tk::MENU_WINDOWS.sym);
    t.put(base + 0x40, 0u8);
    t.twin("MenuSubCmd_OpenB", 0x008e_3670, &[1, 2, 3], U32, |o| {
        w(tk::menu_sub_cmd_open(
            o,
            [1, 2, 3, 0],
            true,
            [1, 2],
            &mut callback,
        ))
    });
    eprintln!("{} calls", t.calls);
}

/// The element words `TkMenuDrawRect` sets (the rest is stack garbage).
const UI_FIELDS: [u32; 16] = [
    0, 4, 0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x30, 0x34, 0x38, 0x3c, 0x80, 0x88, 0x90, 0x94,
];

/// `graphicGetAbmap` returns the word at `cell`; `graphicDrawUIElement`
/// appends the element's set words and its four other arguments at `*log`
/// and returns `0x77`. In both oracles. Returns (`cell`, `log`).
fn record_ui(t: &mut Twins) -> (u32, u32) {
    let cell = t.alloc(&[0; 4]);
    let buf = t.alloc(&vec![0; 0x10000]);
    let log = t.alloc(&buf.to_le_bytes());
    let l = log.to_le_bytes();
    let mut draw: Vec<u8> = vec![0x8b, 0x4c, 0x24, 0x04, 0x8b, 0x15]; // ecx = element, edx = [log]
    draw.extend(l);
    let mut k = 0u32;
    for off in UI_FIELDS {
        draw.extend([0x8b, 0x81]); // mov eax, [ecx + off]
        draw.extend(off.to_le_bytes());
        draw.extend([0x89, 0x82]); // mov [edx + k], eax
        draw.extend(k.to_le_bytes());
        k += 4;
    }
    for a in 0..4u8 {
        draw.extend([0x8b, 0x44, 0x24, 8 + 4 * a]); // mov eax, [esp + 8 + 4a]
        draw.extend([0x89, 0x82]);
        draw.extend(k.to_le_bytes());
        k += 4;
    }
    draw.extend([0x81, 0x05]); // add dword [log], k
    draw.extend(l);
    draw.extend(k.to_le_bytes());
    draw.extend([0xb8, 0x77, 0, 0, 0, 0xc3]);
    let mut abmap = vec![0xa1];
    abmap.extend(cell.to_le_bytes());
    abmap.push(0xc3);
    for o in [&mut t.a, &mut t.b] {
        let d = o.rebase(0x0063_eee0);
        o.write(d, &draw).unwrap();
        let a = o.rebase(0x0064_1060);
        o.write(a, &abmap).unwrap();
    }
    (cell, log)
}

/// The draw recorder's effect, for the bridge side.
fn draw_into(o: &mut Oracle, log: u32, fields: &[(u32, u32)]) -> u32 {
    let at: u32 = Mem::read(o, log);
    let mut k = 0;
    for off in UI_FIELDS {
        let v = fields.iter().find(|f| f.0 == off).expect("a set field").1;
        Mem::write(o, at + k, v);
        k += 4;
    }
    for v in [0u32, 1, 0, 0] {
        Mem::write(o, at + k, v);
        k += 4;
    }
    Mem::write(o, log, at + k);
    0x77
}

#[test]
fn menu_rects_match_the_original() {
    let Some(mut t) = setup_any(0x0dd3_4000) else {
        return;
    };
    let (cell, log) = record_ui(&mut t);
    let entries = Mem::read::<u32>(&t.b, t.sym(0x00c5_a900));
    for _ in 0..60 {
        let ab = t.rng.pick(&[0u32, 0, 1, 2]);
        t.put(cell, ab);
        let r = [t.rng.int(), t.rng.int(), t.rng.int(), t.rng.int()];
        let c = [t.rng.next(), t.rng.next()];
        t.twin(
            "TkMenuDrawRect",
            0x008d_d340,
            &[r[0], r[1], r[2], r[3], c[0], c[1]],
            U32,
            |o| {
                w(tk::tk_menu_draw_rect(
                    o,
                    r.map(i),
                    c,
                    &mut |o| Mem::read(o, cell),
                    &mut |o, f| draw_into(o, log, f),
                ))
            },
        );
        let pk = t.alloc_random(0x1000);
        t.put(t.sym(tk::MENU_PACKET.sym), pk);
        let (x, y) = (t.rng.below(0x280), t.rng.below(0x200));
        let entry = t.rng.below(entries + 2);
        let alpha = t.rng.below(0x100);
        for (name, sym, args, al) in [
            ("TkMenuDrawSp3FaceA", 0x008d_d610, vec![x, y, entry], 0x80),
            (
                "TkMenuDrawSp3FaceAA",
                0x008d_d6a0,
                vec![x, y, entry, alpha],
                alpha,
            ),
        ] {
            t.twin(name, sym, &args, U32, |o| {
                w(tk::tk_menu_draw_sp3_face(
                    o,
                    [x, y, entry, al],
                    &mut |o| Mem::read(o, cell),
                    &mut |o, f| draw_into(o, log, f),
                ))
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn tk_round8_matches_the_original() {
    let Some(mut t) = setup_any(0x8a8a_0808) else {
        return;
    };
    let snapshot = std::env::var_os("FFX_SNAPSHOT").is_some();
    let (log, arena) = record_mallocs(&mut t);
    let floats = [
        0.0f32,
        -0.0,
        1.0,
        2.0,
        0.25,
        3.0,
        3.2,
        -3.2,
        7.5,
        -100.25,
        1e9,
        -1e9,
        1e-30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        std::f32::consts::PI,
        -std::f32::consts::PI,
    ];
    for round in 0..150u32 {
        let f = t.rng.pick(&floats);
        let g = t.rng.pick(&floats);
        for (name, sym, func) in [
            (
                "CIsqrt",
                0x0088_c400,
                ffx_menu::math::ci_sqrt as fn(f32) -> f32,
            ),
            (
                "Math_WrapAngleFloor",
                0x0088_c630,
                ffx_menu::math::wrap_angle_floor,
            ),
        ] {
            let r =
                t.a.call(t.a.rebase(sym), Conv::Cdecl, &[f.to_bits()])
                    .unwrap();
            same_float(name, r.st0, func(f));
        }
        let fin = [
            0.0f32, -0.0, 1.0, 2.0, 0.25, 3.0, 3.2, -3.2, 7.5, -100.25, 1e5, 6.3, -6.3, 1e-30,
        ];
        let (fa, fb, fc) = (t.rng.pick(&fin), t.rng.pick(&fin), t.rng.pick(&fin));
        for (name, sym, scale, mask) in [
            (
                "Math_AngleTo12Bits",
                0x0088_bfe0u32,
                0x00b5_15c8u32,
                0xfffu32,
            ),
            ("Math_AngleTo15Bits", 0x0088_c040, 0x00b5_1588, 0x7fff),
        ] {
            t.twin(name, sym, &[fa.to_bits()], U32, |o| {
                w(tk::math_angle_to_bits(o, fa, scale, mask))
            });
        }
        let r =
            t.a.call(t.a.rebase(0x0088_c5c0), Conv::Cdecl, &[fa.to_bits()])
                .unwrap();
        same_float("Math_AngleToDegrees", r.st0, tk::math_angle_to_degrees(fa));
        let r =
            t.a.call(
                t.a.rebase(0x0088_b6b0),
                Conv::Cdecl,
                &[fa.to_bits(), fb.to_bits(), fc.to_bits()],
            )
            .unwrap();
        same_float(
            "Math_AngleStepTo",
            r.st0,
            tk::math_angle_step_to(fa, fb, fc),
        );
        let rec = t.alloc_random(0x24);
        let ha = if t.rng.below(3) == 0 {
            0
        } else {
            t.alloc_random(4)
        };
        let hb = if t.rng.below(3) == 0 {
            0
        } else {
            t.alloc_random(4)
        };
        t.twin(
            "Vibrate_SetStreams",
            0x0087_6180,
            &[rec, ha, hb],
            U32,
            |o| w(tk::vibrate_set_streams(o, rec, ha, hb)),
        );
        let vk = t.rng.pick(&[0u32, 3, 7, 9, 0xffff_ffff]);
        let ve = t.rng.pick(&[0u32, 1, 2, 0xffff_ffff]);
        let vf = t.rng.pick(&[0u32, 0x40, 0x7f, 0x80, 0xffff_ffff]);
        let vt = t.rng.pick(&[0u32, 0x40, 0x100, 0xffff_ffff]);
        let vl = t.rng.pick(&[0u32, 10, 0x8000, 0x1_0005, 0xffff_ffff]);
        t.twin(
            "Vibrate_StartLevel",
            0x0087_6050,
            &[vk, ve, vf, vt, vl],
            U32,
            |o| {
                s(tk::vibrate_start_level(
                    o,
                    i(vk),
                    i(ve),
                    i(vf),
                    i(vt),
                    i(vl),
                ))
            },
        );
        let mut tb = vec![0u8; 0x100];
        for k in 0..0x40usize {
            if k % 3 != 0 {
                tb[4 * k..4 * k + 4].copy_from_slice(&0x100u32.to_le_bytes());
            }
        }
        let curves = t.rng.bytes(0x40);
        tb.extend(curves);
        let table = if t.rng.below(8) == 0 { 0 } else { t.alloc(&tb) };
        let vrec = t.alloc_random(0x24);
        let vmode = t.rng.pick(&[0u32, 1, 2]);
        t.twin(
            "Vibrate_SetStreamsMode",
            0x0087_6130,
            &[vrec, vmode, ha, hb],
            U32,
            |o| w(tk::vibrate_set_streams_mode(o, vrec, i(vmode), ha, hb)),
        );
        let vrec2 = t.alloc_random(0x24);
        let smode = t.rng.pick(&[0u8, 1, 2]);
        t.put(vrec2 + 0x22, smode);
        let sent = t.alloc_random(0x10);
        let sdata = if t.rng.below(3) == 0 {
            0
        } else {
            t.alloc_random(8)
        };
        t.put(sent, sdata);
        let sstep = t.rng.pick(&[0u16, 3, 9, 0x7fff, 0xffff]);
        t.put(sent + 4, sstep);
        let slen6 = t.rng.pick(&[0u16, 10, 0x7fff, 0xfffe]);
        t.put(sent + 6, slen6);
        let slen = t.rng.pick(&[0u32, 4, 10, 0xffff_ffff]);
        t.twin(
            "Vibrate_StepLevel",
            0x0087_5c90,
            &[vrec2, sent, slen],
            U32,
            |o| w(tk::vibrate_step_level(o, vrec2, sent, i(slen))),
        );
        let lrec = t.alloc_random(0x24);
        let lmode = t.rng.pick(&[0u8, 1, 2, 3]);
        t.put(lrec + 0x22, lmode);
        let le = t.rng.below(2);
        let lentry = lrec + 0x10 * le;
        let lsize = t.rng.pick(&[0u16, 5, 0x1ff, 0x200, 0xfffe]);
        t.put(lentry + 6, lsize);
        t.put(lentry, 0u32);
        t.twin("Vibrate_EntryLevel", 0x0087_5c20, &[lrec, le], U32, |o| {
            w(tk::vibrate_entry_level(o, lrec, le))
        });
        let sk = t.rng.pick(&[0u32, 2, 7, 8, 0xffff_ffff]);
        let sc = t.rng.pick(&[0u32, 1, 2, 5, 0x3f, 0x40]);
        t.twin(
            "TkSetVibrateStream",
            0x0087_5fb0,
            &[sk, sc, table],
            U32,
            |o| w(tk::tk_set_vibrate_stream(o, i(sk), sc, table)),
        );
        let ck = t.rng.pick(&[0u32, 1, 2, 5, 0x3f, 0x40]);
        let cn = t.rng.pick(&[0u32, 1, 2, 3, 4, 7, 10]);
        t.twin(
            "Vibrate_CurveData",
            0x0087_5d70,
            &[ck, cn, table],
            U32,
            |o| w(tk::vibrate_curve_data(o, ck, i(cn), table)),
        );
        // the CLUT allocator
        if round % 30 == 0 {
            let mode = t.rng.pick(&[2u32, 3, 4, 0, 7]);
            t.scramble(0x0193_bb0c, 0x14);
            t.twin("yiClutAlloc_init", 0x0091_6e60, &[mode], U32, |o| {
                w(tk::yi_clut_alloc_init(o, i(mode)))
            });
        }
        t.twin("yiClutAlloc_get256idx", 0x0091_6e50, &[], U32, |o| {
            w(tk::yi_clut_alloc_get256idx(o))
        });
        let clut = t.alloc_random(0x1200);
        let csrc = t.alloc_random(0x400);
        let ck = t.rng.pick(&[0u32, 1, 2, 0xffff_ffff]);
        t.twin(
            "yiClutAlloc_copy256",
            0x0091_6d40,
            &[clut + 0x500, csrc, ck],
            U32,
            |o| w(tk::yi_clut_alloc_copy256(o, clut + 0x500, csrc, i(ck))),
        );
        let ci = t.rng.pick(&[0u32, 1, 2, 3, 7, 0xffff_ffff, 0xffff_fffe]);
        t.twin(
            "yiClutAlloc_copy16",
            0x0091_6c80,
            &[clut + 0x500, csrc, ck, ci],
            U32,
            |o| {
                w(tk::yi_clut_alloc_copy16(
                    o,
                    clut + 0x500,
                    csrc,
                    i(ck),
                    i(ci),
                ))
            },
        );
        let dst = t.alloc_random(4);
        let src = t.alloc(&g.to_le_bytes());
        t.twin(
            "Math_StoreMax",
            0x0088_9b00,
            &[dst, src, f.to_bits()],
            VOID,
            |o| {
                tk::math_store_max(o, dst, src, f);
                0
            },
        );
        t.twin(
            "Math_StoreMin",
            0x0088_9b40,
            &[dst, src, 0, 0, 0, f.to_bits()],
            VOID,
            |o| {
                tk::math_store_min(o, dst, src, f);
                0
            },
        );
        let busy = t.rng.pick(&[0u8, 0, 1]);
        t.put(t.sym(0x0132_f581), busy);
        let pend = t.rng.pick(&[0u32, 0, 5]);
        t.put(t.sym(0x0132_f578), pend);
        let (a, n) = (t.rng.next(), t.rng.int());
        t.twin("TkDeci2Send", 0x0088_5470, &[a, n], U32, |o| {
            s(tk::tk_deci2_send(o, a, i(n)))
        });
        t.twin("Gu_SelectorAllocate", 0x0084_9bb0, &[], U32, |o| {
            w(gu::gu_selector_allocate(o, &mut |o, n| {
                let _ = free_into(o, log, n);
                arena
            }))
        });
        let c = t.alloc_random(0x48);
        t.twin("Gu_ControlInit", 0x0084_95e0, &[c], U32, |o| {
            w(gu::gu_control_init(o, c))
        });
        let len = t.rng.below(20) as usize;
        let mut sb: Vec<u8> = t.rng.bytes(len).into_iter().map(|c| c.max(1)).collect();
        sb.push(0);
        let sp = t.alloc(&sb);
        t.twin("Yi_StrLower", 0x0091_6290, &[sp], VOID, |o| {
            gu::yi_str_case(o, sp, false);
            0
        });
        t.twin("Yi_StrUpper", 0x0091_62c0, &[sp], VOID, |o| {
            gu::yi_str_case(o, sp, true);
            0
        });
        if snapshot {
            let id = t
                .rng
                .pick(&[0u32, 1, 0x3000, 0x3010, 0x3050, 0x30ff, 0x1234]);
            t.twin("TkMenuGetCommandName", 0x008d_de50, &[id], U32, |o| {
                w(tk::tk_menu_get_command_name(o, id))
            });
            let count = ffx_chara::handle::bridge::CHR_COUNT.get(&t.b);
            let array = ffx_chara::handle::bridge::CHR_ARRAY.get(&t.b);
            let live: Vec<u32> = (0..u32::try_from(count).unwrap_or(0))
                .map(|k| array + k * ffx_chara::layout::SIZE)
                .filter(|&c| {
                    let blk: u32 = Mem::read(&t.b, c + ffx_chara::layout::BLK);
                    blk != 0 && blk != 0xcdcd_cdcd
                })
                .collect();
            if round % 10 == 0 && !live.is_empty() {
                let chr = live[t.rng.below(u32::try_from(live.len()).unwrap()) as usize];
                let p = [
                    t.rng.pick(&floats[..13]),
                    t.rng.pick(&floats[..13]),
                    t.rng.pick(&floats[..13]),
                ];
                t.twin(
                    "TkCh_SetPos",
                    0x0087_2310,
                    &[chr, p[0].to_bits(), p[1].to_bits(), p[2].to_bits()],
                    VOID,
                    |o| {
                        tk::tk_ch_set_pos(o, chr, p);
                        0
                    },
                );
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn gu_texts_match_the_original() {
    let Some(mut t) = setup_any(0x7e47_0001) else {
        return;
    };
    let flog = record_frees(&mut t);
    let (mlog, arena) = record_mallocs(&mut t);
    for _ in 0..200 {
        let c = t.alloc_random(0x44);
        let cap = t.rng.pick(&[0u16, 0, 0x14, 0x30, 0xffff]);
        t.put(c + 0x26, cap);
        let heap = t.alloc_random(0x40);
        t.put(c + 0x28, if cap == 0 { c + 0x30 } else { heap });
        let len = t.rng.pick(&[0usize, 3, 14, 15, 16, 0x20, 0x2f, 0x35]);
        let mut sb: Vec<u8> = t.rng.bytes(len).into_iter().map(|c| c.max(1)).collect();
        sb.push(0);
        let sp = if t.rng.below(5) == 0 { 0 } else { t.alloc(&sb) };
        t.twin("Gu_ControlSetText", 0x0084_9610, &[c, sp], VOID, |o| {
            gu::gu_control_set_text(o, c, sp, &mut |o, p| free_into(o, flog, p), &mut |o, n| {
                let _ = free_into(o, mlog, n);
                arena
            });
            0
        });
        let win = window(&mut t);
        let controls: u32 = Mem::read(&t.b, win + 0x94);
        for k in 0..6 {
            let ck = controls + k * 0x44;
            let hk = t.alloc_random(0x40);
            let capk = t.rng.pick(&[0u16, 0x20]);
            t.put(ck + 0x26, capk);
            t.put(ck + 0x28, if capk == 0 { ck + 0x30 } else { hk });
        }
        let tw = t.alloc_random(0x98);
        t.put(tw + 0x34 + 0x26, 0u16);
        t.put(tw + 0x34 + 0x28, tw + 0x34 + 0x30);
        t.twin("Gu_WindowSetTitle", 0x0084_b6a0, &[tw, sp], VOID, |o| {
            gu::gu_control_set_text(
                o,
                tw + 0x34,
                sp,
                &mut |o, p| free_into(o, flog, p),
                &mut |o, n| {
                    let _ = free_into(o, mlog, n);
                    arena
                },
            );
            0
        });
        let id = t.rng.pick(&[0u32, 1, 2, 3, 0x8001, 0x1_0001]);
        t.twin(
            "Gu_WindowControlSetText",
            0x0084_a710,
            &[win, id, sp],
            VOID,
            |o| {
                gu::gu_window_control_set_text(
                    o,
                    win,
                    id,
                    sp,
                    &mut |o, p| free_into(o, flog, p),
                    &mut |o, n| {
                        let _ = free_into(o, mlog, n);
                        arena
                    },
                );
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn gu_lists_match_the_original() {
    let Some(mut t) = setup_any(0x91e5_7001) else {
        return;
    };
    let flog = record_frees(&mut t);
    for _ in 0..100 {
        // a window list with some closed
        let n = t.rng.below(5);
        let mut next = 0u32;
        for _ in 0..n {
            let w0 = t.alloc_random(0xa0);
            t.put(w0, next);
            let fl = t.rng.pick(&[0u32, 4, 0, 0xff]);
            t.put(w0 + 0x88, fl);
            let nc = t.rng.pick(&[0i32, 1, 3, -1]);
            t.put(w0 + 0x90, nc);
            let ctrls = if t.rng.below(4) == 0 {
                0
            } else {
                t.alloc_random(0x44 * 3)
            };
            t.put(w0 + 0x94, ctrls);
            let big = t.rng.pick(&[0u16, 0, 5]);
            t.put(w0 + 0x5a, big);
            next = w0;
        }
        t.put(t.sym(gu::WINDOW_LIST.sym), next);
        t.twin("Gu_WindowFreeClosed", 0x0084_abd0, &[], VOID, |o| {
            gu::gu_window_free_closed(o, &mut |o, p| free_into(o, flog, p));
            0
        });
        // a selector fitted to a window
        let win = t.alloc_random(0xa0);
        let h = t.rng.pick(&[0u16, 9, 40, 90, 200, 0xfff0]);
        t.put(win + 0x12, h);
        let sel = t.alloc_random(0x20);
        let total = t.rng.pick(&[0u16, 1, 3, 10, 30, 0xffff]);
        let cur = t.rng.pick(&[0u16, 1, 5, 20, 0xfffe]);
        t.put(sel + 4, total);
        t.put(sel + 2, cur);
        t.twin("Gu_SelectorFitWindow", 0x0084_a000, &[win, sel], U32, |o| {
            s(gu::gu_selector_fit_window(o, win, sel))
        });
        // EUC text
        let len = t.rng.below(12) as usize;
        let mut text: Vec<u8> = t.rng.bytes(len).into_iter().map(|c| c.max(1)).collect();
        text.extend([0, 0]);
        let src = t.alloc(&text);
        let dst = t.alloc_random(0x20);
        let cnt = t.rng.pick(&[0u32, 1, 2, 5, 16, 0xffff_ffff]);
        let src = if t.rng.below(8) == 0 { 0 } else { src };
        t.twin(
            "TkEucToSjisString",
            0x0088_ed10,
            &[src, dst, cnt],
            U32,
            |o| w(tk::tk_euc_to_sjis_string(o, src, dst, i(cnt))),
        );
    }
    eprintln!("{} calls", t.calls);
}

/// Code space for x87 stand-ins: the body of `pppFpLoop` (`0x00729ab0`),
/// which the menu tests never reach.
const X87_CODE: u32 = 0x0072_9ab0;
/// `_CIcos`, `_CIsin`, `_CItan`, `_CIatan2` import slots (MSVCR110) and
/// their stand-ins, the CPU's own instructions on `ST0`/`ST1`: `fcos`,
/// `fsin`, `fptan; fstp st0`, `fpatan`, each then `ret`.
const X87_SLOTS: [(u32, &[u8]); 4] = [
    (0x00b0_c3b4, &[0xd9, 0xff, 0xc3]),
    (0x00b0_c3b8, &[0xd9, 0xfe, 0xc3]),
    (0x00b0_c3a0, &[0xd9, 0xf2, 0xdd, 0xd8, 0xc3]),
    (0x00b0_c3b0, &[0xd9, 0xf3, 0xc3]),
];
/// For the bridge's side: `fld qword [esp + 4]`, `fld qword [esp + 0xc]`
/// when the fifth word is set, then a jump to the sixth word.
const X87_CALL: [u8; 25] = [
    0xdd, 0x44, 0x24, 0x04, // fld qword [esp + 4]
    0x83, 0x7c, 0x24, 0x14, 0x00, // cmp dword [esp + 0x14], 0
    0x74, 0x04, // je +4
    0xdd, 0x44, 0x24, 0x0c, // fld qword [esp + 0xc]
    0x8b, 0x44, 0x24, 0x18, // mov eax, [esp + 0x18]
    0xff, 0xe0, // jmp eax
    0x90, 0x90, 0x90, 0x90,
];
/// `fstp st0; ret`, after reading a result.
const X87_POP: [u8; 3] = [0xdd, 0xd8, 0xc3];

/// The stand-ins in both oracles.
fn install_x87(t: &mut Twins) {
    for o in [&mut t.a, &mut t.b] {
        for (k, (slot, code)) in (0u32..).zip(X87_SLOTS) {
            let at = o.rebase(X87_CODE + 0x10 * k);
            o.write(at, code).unwrap();
            let s = o.rebase(slot);
            o.write(s, &at.to_le_bytes()).unwrap();
        }
        let at = o.rebase(X87_CODE + 0x40);
        o.write(at, &X87_CALL).unwrap();
        let at = o.rebase(X87_CODE + 0x60);
        o.write(at, &X87_POP).unwrap();
    }
}

/// An intrinsic stand-in run in `o` on `a` (and `b`): the result.
fn x87_call(o: &mut Oracle, slot: u32, a: f64, b: Option<f64>) -> f64 {
    let f: u32 = Mem::read(o, o.rebase(slot));
    let (x, y) = (a.to_bits(), b.unwrap_or(0.0).to_bits());
    #[allow(clippy::cast_possible_truncation)]
    let w = [
        x as u32,
        (x >> 32) as u32,
        y as u32,
        (y >> 32) as u32,
        u32::from(b.is_some()),
        f,
    ];
    let at = o.rebase(X87_CODE + 0x40);
    let r = o.call(at, Conv::Cdecl, &w).unwrap();
    let pop = o.rebase(X87_CODE + 0x60);
    o.call(pop, Conv::Cdecl, &[]).unwrap();
    // NaN sign and payload kept
    r.st0.expect("a float result")
}

#[test]
fn ci_wrappers_match_the_original() {
    let Some(mut t) = setup_any(0xc1c1_0009) else {
        return;
    };
    install_x87(&mut t);
    let floats = [
        0.0f32, -0.0, 0.5, 1.0, -1.0, 2.5, 3.0, -3.1, 10.0, 100.5, 1e-5, -7.25, 1e6,
    ];
    for x in floats {
        for (name, sym, slot) in [
            ("CIcos", 0x0088_b3e0u32, 0x00b0_c3b4u32),
            ("CIsin", 0x0088_c3a0, 0x00b0_c3b8),
            ("CItan", 0x0088_c420, 0x00b0_c3a0),
        ] {
            let r =
                t.a.call(t.a.rebase(sym), Conv::Cdecl, &[x.to_bits()])
                    .unwrap();
            let pop = t.a.rebase(X87_CODE + 0x60);
            t.a.call(pop, Conv::Cdecl, &[]).unwrap();
            let want = ffx_menu::math::ci_unary(x, |v| x87_call(&mut t.b, slot, v, None));
            same_float(name, r.st0, want);
        }
        for y in floats {
            let r =
                t.a.call(
                    t.a.rebase(0x0088_9e70),
                    Conv::Cdecl,
                    &[y.to_bits(), x.to_bits()],
                )
                .unwrap();
            let pop = t.a.rebase(X87_CODE + 0x60);
            t.a.call(pop, Conv::Cdecl, &[]).unwrap();
            let want =
                ffx_menu::math::ci_atan2(y, x, |a, b| x87_call(&mut t.b, 0x00b0_c3b0, a, Some(b)));
            same_float("CIatan2", r.st0, want);
        }
    }
}

/// MSVCR110 `sprintf`'s import slot.
const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

#[test]
fn printf_matches_the_runtime() {
    let Some(mut t) = setup_any(0x9f9f_0001) else {
        return;
    };
    let formats: &[&[u8]] = &[
        b"%d",
        b"%5d",
        b"%-5d|",
        b"%05d",
        b"%+d",
        b"% d",
        b"%u",
        b"%x",
        b"%#x",
        b"%X",
        b"%08X",
        b"%o",
        b"%#o",
        b"%.3d",
        b"%.0d",
        b"%c",
        b"%s",
        b"%.2s",
        b"%10s",
        b"%-10s|",
        b"%hd",
        b"%hhx",
        b"%hu",
        b"%ld",
        b"%%",
        b"%d.%02d",
        b"%d.%d",
        b"%p",
        b"%*d",
        b"%.*d",
        b"%-*d|",
        b"%3c",
        b"%I64d",
        b"%lld",
        b"%5.3d",
        b"%-08x|",
        b"%+05d",
        b"FF0906%02d",
        b"010906%02d.ico",
        b"",
    ];
    let text = t.alloc(b"hello\0");
    for round in 0..400u32 {
        let fmt = formats[(round as usize) % formats.len()];
        let mut fb = fmt.to_vec();
        fb.push(0);
        let f = t.alloc(&fb);
        let mut args: Vec<u32> = (0..4).map(|_| t.rng.int()).collect();
        if fmt
            .windows(2)
            .any(|w| w == b"%s" || w == b".2" || w == b"0s")
            || fmt.ends_with(b"s|")
        {
            args[0] = if round % 7 == 0 { 0 } else { text };
        }
        if fmt.contains(&b'*') {
            args[0] = t.rng.pick(&[0u32, 3, 12, 0xffff_fffb, 0xffff_ffff]);
        }
        let buf = t.alloc(&[0u8; 0x80]);
        let stub: u32 = Mem::read(&t.a, t.a.rebase(SPRINTF_SLOT));
        let mut call = vec![buf, f];
        call.extend(&args);
        let r =
            t.a.call(stub, Conv::Cdecl, &call)
                .unwrap_or_else(|e| panic!("{:?} {args:x?}: {e}", String::from_utf8_lossy(fmt)));
        let n = ffx_text::printf::sprintf(&mut t.b, buf, f, &args).expect("supported");
        assert_eq!(
            r.eax,
            n,
            "{:?} {args:x?}: length",
            String::from_utf8_lossy(fmt)
        );
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{:?} {args:x?}: text",
            String::from_utf8_lossy(fmt)
        );
        t.a.clear_dirty();
        t.b.clear_dirty();
    }
}

#[test]
fn save_paths_match_the_original() {
    let Some(mut t) = setup_any(0x5a5e_0002) else {
        return;
    };
    let codes: [&[u8]; 3] = [
        b"BISLPS-25088FF0906%02d\0",
        b"BESCES-50490FF0906%02d\0",
        b"BKSLPM-67513FF0906%02d\0",
    ];
    let flog = record_frees(&mut t);
    let (mlog, arena) = record_mallocs(&mut t);
    for round in 0..150u32 {
        let code = codes[(round % 3) as usize];
        t.put_bytes(t.sym(0x023c_c0a0), code);
        let n = t.rng.pick(&[0u32, 1, 9, 10, 99, 100, 0xffff_ffff]);
        let out = t.alloc_random(0x80);
        for (name, sym, kind) in [
            ("MenuSave_PathDir", 0x008b_3ff0u32, 0u8),
            ("MenuSave_PathData", 0x008b_4060, 1),
            ("MenuSave_PathIcon", 0x008b_4110, 2),
            ("MenuSave_PathIconSys", 0x008b_4230, 3),
        ] {
            t.twin(name, sym, &[out, n], VOID, |o| {
                assert!(tk::menu_save_path(o, out, n, kind));
                0
            });
        }
        t.twin("MenuSave_IconName", 0x008b_41e0, &[out, n], VOID, |o| {
            tk::menu_save_icon_name(o, out, n);
            0
        });
        // a formatted control text
        let win = window(&mut t);
        let controls: u32 = Mem::read(&t.b, win + 0x94);
        for k in 0..6 {
            let ck = controls + k * 0x44;
            let hk = t.alloc_random(0x40);
            let capk = t.rng.pick(&[0u16, 0x20]);
            t.put(ck + 0x26, capk);
            t.put(ck + 0x28, if capk == 0 { ck + 0x30 } else { hk });
        }
        let fmt = t.rng.pick(&[0u32, 1, 2, 3]);
        let fmts: [&[u8]; 4] = [
            b"%d.%02d\0",
            b"%d.%d\0",
            b"%s and %d %s\0",
            b"%08X-%-6d|%c\0",
        ];
        let f = t.alloc(fmts[fmt as usize]);
        let s1 = t.alloc(b"a fairly long label text\0");
        let args = [t.rng.int(), t.rng.int(), s1, 0x41];
        let args = if fmt == 2 { [s1, args[1], s1, 0] } else { args };
        let id = t.rng.pick(&[1u32, 2, 3, 0x1_0001]);
        let mut call = vec![win, id, f];
        call.extend(args);
        t.twin("Gu_WindowControlSetTextf", 0x0084_a810, &call, VOID, |o| {
            assert!(gu::gu_window_control_set_textf(
                o,
                win,
                id,
                f,
                &args,
                &mut |o, p| free_into(o, flog, p),
                &mut |o, n| {
                    let _ = free_into(o, mlog, n);
                    arena
                },
            ));
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

/// Menu leaves (round 15): summon stat fields, list searches, the fade
/// step, the item key parser, scene group resets, the party cursor.
#[test]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
fn menu_leaves_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_0015) else {
        return;
    };
    for round in 0..48u32 {
        // summon stats
        let base = t.alloc_random(0x40);
        let mut d = t.rng.bytes(0x14);
        d[0xc] = (t.rng.next() % 0x3c) as u8;
        d[0xd] = 0;
        d[0xe] = (t.rng.next() % 0x3c) as u8;
        d[0xf] = 0;
        d[0x10..0x14]
            .copy_from_slice(&(if t.rng.next() % 2 == 0 { 4u32 } else { 1 }).to_le_bytes());
        let desc = t.alloc(&d);
        let max = t.rng.next() % 2;
        t.twin(
            "MenuSummon_GetStat",
            0x008c_eca0,
            &[base, desc, max],
            U32,
            |m| u64::from(tk::menu_summon_get_stat(m, base, desc, max)),
        );
        let v = t.rng.next();
        t.twin(
            "MenuSummon_SetStat",
            0x008c_ee70,
            &[base, desc, max, v],
            VOID,
            |m| {
                tk::menu_summon_set_stat(m, base, desc, max, v);
                0
            },
        );
        // searches
        let mut b = t.rng.bytes(0x40);
        for x in &mut b {
            *x %= 6;
        }
        let bytes = t.alloc(&b);
        let (from, to) = ((t.rng.next() % 20) as i32 - 2, (t.rng.next() % 20) as i32);
        let val = t.rng.next() % 6;
        t.twin(
            "Menu_FindByte",
            0x008c_9bc0,
            &[bytes, val, from as u32, to as u32],
            U32,
            |m| u64::from(tk::menu_find_byte(m, bytes, val, from, to).cast_unsigned()),
        );
        let (from, to) = ((t.rng.next() % 12) as i32, (t.rng.next() % 16) as i32);
        t.twin(
            "Menu_FindFlagged",
            0x008c_9b90,
            &[bytes, from as u32, to as u32],
            U32,
            |m| u64::from(tk::menu_find_flagged(m, bytes, from, to).cast_unsigned()),
        );
        // fade
        let obj = t.alloc_random(0x70);
        let (p, q) = (
            (t.rng.next() % 0x200) as i32 - 0x100,
            (t.rng.next() % 0x80) as i32 - 0x40,
        );
        t.put(obj + 0x64, p);
        t.put(obj + 0x68, q);
        t.twin("Menu_FadeStep", 0x008e_2b30, &[obj], U32, |m| {
            u64::from(tk::menu_fade_step(m, obj))
        });
        // item keys
        let n = t.rng.next() % 40;
        let mut keys: Vec<u8> = (0..n)
            .map(|_| [4u8, 4, 7, 0x2f, 0x30, 1, 0x10, 0x5][(t.rng.next() % 8) as usize])
            .collect();
        keys.push(0);
        keys.extend([0x33, 0x44, 0]);
        let ks = t.alloc(&keys);
        t.scramble(0x0186_a280, 0x80);
        t.twin("Menu_ParseItemKeys", 0x008c_5280, &[ks], VOID, |m| {
            tk::menu_parse_item_keys(m, ks);
            0
        });
        // scene group
        let scene = t.rng.next() % 4;
        let data = t.alloc_random(0x200);
        let group = 0x10 + (t.rng.next() % 4) * 8;
        let list = 0x80;
        let top = t.rng.next() & 0xc000_0000;
        t.put(data + group + 4, list | top);
        let n = t.rng.next() % 5;
        t.put(data + list, n);
        let slot = t.sym(0x022f_b4dc) + 4 * scene;
        t.put(slot, if round % 8 == 7 { 0 } else { data });
        let off = if round % 6 == 5 { 0 } else { group };
        let on = t.rng.next() % 2;
        t.twin(
            "MenuScene_ResetGroup",
            0x00a7_ba70,
            &[scene, off, on],
            U32,
            |m| u64::from(tk::menu_scene_reset_group(m, scene, off, on)),
        );
        // party cursor
        let count = 1 + t.rng.next() % 6;
        let mut slots = vec![];
        let mut any = false;
        for k in 0..count {
            let mut e = t.rng.bytes(0x10);
            let sel = t.rng.next() % 2 == 0 || (k == count - 1 && !any);
            any |= sel;
            e[0xc..0x10].copy_from_slice(&u32::from(sel).to_le_bytes());
            slots.extend(t.alloc(&e).to_le_bytes());
        }
        let table = t.alloc(&slots);
        t.put(t.sym(0x0186_a44c), table);
        t.put(t.sym(0x0186_a448), count);
        let c0 = (t.rng.next() % count) as u8;
        t.put(t.sym(0x0186_a444), c0);
        t.twin("MenuParty_CursorPrev", 0x008c_ba60, &[], VOID, |m| {
            tk::menu_party_cursor_prev(m);
            0
        });
        t.twin("MenuParty_CursorNext", 0x008c_ba20, &[], VOID, |m| {
            tk::menu_party_cursor_next(m);
            0
        });
        // arithmetic and the counter
        let (a, b) = (t.rng.next() as i32 >> 8, (t.rng.next() % 64) as i32 - 32);
        t.twin(
            "Menu_Scale32",
            0x008b_f100,
            &[a as u32, b as u32],
            U32,
            |_| u64::from(tk::menu_scale32(a, b).cast_unsigned()),
        );
        let cell = t.alloc_random(12);
        let c =
            [(t.rng.next() % 0x800) as i32 - 0x400, 0, 0x199, -0x199][(t.rng.next() % 4) as usize];
        t.put(cell, c);
        t.twin(
            "Menu_EaseCounter",
            0x008b_f7e0,
            &[0, cell, cell + 4, cell + 8],
            VOID,
            |m| {
                tk::menu_ease_counter(m, cell, cell + 4, cell + 8);
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

/// Menu leaves, round 15 part 2: the colour tables, the name list and the
/// segment height gap.
#[test]
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
fn menu_leaves2_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_b15b) else {
        return;
    };
    for round in 0..64u32 {
        for (sym, table, last) in [
            (0x0088_fca0u32, 0x0133_c8f4u32, 3i32),
            (0x0089_0320, 0x0133_c8e4, 3),
            (0x0089_06d0, 0x0133_c904, 2),
        ] {
            let i = (t.rng.next() % 8) as i32 - 2;
            let c: [i32; 4] = std::array::from_fn(|_| (t.rng.next() % 0x180) as i32 - 0x40);
            let args = [i as u32, c[0] as u32, c[1] as u32, c[2] as u32, c[3] as u32];
            t.twin("TkSetRgba", sym, &args, U32, |m| {
                u64::from(tk::tk_set_rgba(m, table, last, i, c))
            });
        }
        // the name list
        let n = t.rng.next() % 12;
        t.put(t.sym(0x0187_16a0), n);
        let names = t.sym(0x0187_5710);
        for k in 0..n.min(10) {
            let mut s = vec![b'a' + (t.rng.next() % 3) as u8, b'b'];
            s.resize(0x100, 0);
            t.put_bytes(names + 0x100 * k, &s);
        }
        let mut s = vec![b'a' + (t.rng.next() % 4) as u8, b'b'];
        s.resize(0x100, round as u8);
        s[2] = 0;
        let at = t.alloc(&s);
        t.twin("Menu_AddName", 0x008e_5200, &[at], U32, |m| {
            u64::from(tk::menu_add_name(m, at))
        });
        // segment height gap
        let pts: [[f32; 3]; 4] = std::array::from_fn(|_| {
            std::array::from_fn(|_| {
                f32::from(i8::try_from(t.rng.next() % 64).unwrap_or(0) - 32) / 4.0
            })
        });
        let mut pts = pts;
        if round % 7 == 3 {
            pts[3] = [
                pts[2][0] + pts[1][0] - pts[0][0],
                1.0,
                pts[2][2] + pts[1][2] - pts[0][2],
            ];
        }
        if round % 5 == 2 {
            pts[1][0] = pts[0][0];
        }
        let addrs: Vec<u32> = pts
            .iter()
            .map(|p| t.alloc(&p.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>()))
            .collect();
        t.drain_x87();
        let r =
            t.a.call(t.a.rebase(0x0088_9e90), Conv::Cdecl, &addrs)
                .unwrap();
        same_float(
            &format!("Math_CrossHeightGap {pts:?}"),
            r.st0,
            ffx_menu::math::cross_height_gap(pts[0], pts[1], pts[2], pts[3]),
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_wrappers_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_3a3a) else {
        return;
    };
    record_frees(&mut t);
    install_x87(&mut t);
    // achievementUnlockAchievement (Steam), SndSqsIopFrame and
    // SndStatusUpdate (the sound driver, absent in the image): xor eax,
    // eax; ret in both
    for o in [&mut t.a, &mut t.b] {
        for f in [0x0082_2380, 0x0088_7310, 0x0088_7700] {
            let at = o.rebase(f);
            o.write(at, &[0x31, 0xc0, 0xc3]).unwrap();
        }
    }
    // dbgPrintf (a bare ret on PC) counting its calls: inc dword [n]; ret
    let n = t.alloc(&[0; 4]);
    let code = [&[0xff, 0x05][..], &n.to_le_bytes(), &[0xc3]].concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x0062_f500);
        o.write(at, &code).unwrap();
    }
    let w = u64::from;
    for round in 0..48u32 {
        t.twin("TkSyncV", 0x0088_7f00, &[], U32, |o| {
            w(tk::tk_sync_v(o, &mut callback))
        });
        let copy = if round % 2 == 0 { 0 } else { t.rng.next() };
        t.put(t.sym(0x0186_a244), copy);
        t.twin("TkMn2DrawInitPacket", 0x008c_0be0, &[], U32, |o| {
            w(tk::tk_mn2_draw_init_packet(o, &mut callback))
        });
        // the cursor, its start and the two marks around each other and the
        // 0x02000000 limit
        for a in [0x0186_a238u32, 0x0186_a240, 0x0186_a244, 0x0186_a248] {
            let v = match t.rng.below(4) {
                0 => 0x01ff_fff0 + t.rng.below(0x20),
                1 => t.rng.below(0x100),
                _ => t.rng.next(),
            };
            t.put(t.sym(a), v);
        }
        if round % 6 == 0 {
            // one step short of the limit
            t.put(t.sym(0x0186_a238), 0x01ff_fff0u32);
        }
        let clear = t.rng.pick(&[0, 1, 0x100]);
        t.twin("TkMn2DrawKickPacketBody", 0x008c_0c50, &[clear], U32, |o| {
            w(tk::tk_mn2_draw_kick_packet_body(o, clear, &mut callback))
        });
        let p = t.rng.next();
        t.twin("TkFree", 0x0088_d520, &[p], U32, |o| {
            w(tk::tk_free(o, p, &mut callback))
        });
        for a in [
            0x0132_ff10u32,
            0x0132_ff14,
            0x0132_ff18,
            0x0132_ff20,
            0x0132_ff24,
        ] {
            let v = t.rng.next();
            t.put(t.sym(a), v);
        }
        t.twin("TkSndUpdateMain", 0x0088_7bd0, &[], U32, |o| {
            w(tk::tk_snd_update_main(o, &mut callback))
        });
        // the music queue SndExecMusicClearQue counts down: entries whose
        // frames stay above 0 or already are at or below (the stop at 0
        // goes to the sound driver), up to 17
        let n = t.rng.below(18);
        t.put(t.sym(0x0133_0068), n);
        for k in 0..17 {
            let frames = t.rng.pick(&[0i16, -1, 4, 5, 100, 0x7fff]);
            let at = t.sym(0x0132_ffe0) + 8 * k;
            t.put(at + 2, frames);
        }
        // the sound sync flag (always 1 on PC; 0 for the other path)
        let flag = t.rng.below(2);
        return_const(&mut t, 0x0088_7dd0, flag);
        t.twin("TkSndUpdate", 0x0088_7b60, &[], U32, |o| {
            w(tk::tk_snd_update(o, &mut callback))
        });
        t.twin("TkSndUpdate2", 0x0088_7b80, &[], U32, |o| {
            w(tk::tk_snd_update2(o, &mut callback))
        });
        return_const(&mut t, 0x0088_7dd0, 1);
        let counter = t.rng.below(4);
        t.put(t.sym(0x0133_0200), counter);
        // what TkSndUpdateMain clears and the sync flag, to see whether it ran
        t.scramble(0x0132_ff10, 0x18);
        t.scramble(0x0133_01e4, 4);
        let last = t.rng.below(4);
        t.twin("TkSndUpdate3", 0x0088_7ba0, &[last], U32, |o| {
            w(tk::tk_snd_update3(o, last, &mut callback))
        });
        let n = match round % 3 {
            0 => 0xa000 + t.rng.below(0x24),
            1 => t.rng.below(0x100),
            _ => t.rng.next(),
        };
        t.twin("TkMsImportantSet", 0x0088_e790, &[n], U32, |o| {
            w(tk::tk_ms_important_set(o, n, &mut callback))
        });
        let b = t.rng.next();
        t.put(t.sym(0x0113_079c), b);
        t.twin("TkRefreshSoundMode", 0x0087_24f0, &[], U32, |o| {
            w(tk::tk_refresh_sound_mode(o, &mut callback))
        });
        let win = if round % 3 == 0 {
            0
        } else {
            t.alloc_random(0x60)
        };
        t.put(t.sym(tk::MENU_SUB_CMD_WINDOW.sym), win);
        t.twin("MenuSubCmd_CenterOpen", 0x008e_4ab0, &[], U32, |o| {
            w(tk::menu_sub_cmd_center_open(o, &mut callback))
        });
        // Math_Atan2XZ: axes, NaN and general points
        let pick = |t: &mut Twins| -> f32 {
            match t.rng.below(6) {
                0 => 0.0,
                1 => -0.0,
                2 => f32::NAN,
                _ => f32::from(i8::try_from(t.rng.below(64)).unwrap_or(0) - 32) / 4.0,
            }
        };
        let v: [f32; 4] = std::array::from_fn(|_| pick(&mut t));
        let mut v = v;
        if round % 4 == 1 {
            v[2] = v[0];
        }
        if round % 4 == 2 {
            v[3] = v[1];
        }
        t.drain_x87();
        let r =
            t.a.call(t.a.rebase(0x0088_b430), Conv::Cdecl, &v.map(f32::to_bits))
                .unwrap();
        let want = ffx_menu::math::atan2_xz(v[0], v[1], v[2], v[3], |a, b| {
            x87_call(&mut t.b, 0x00b0_c3b0, a, Some(b))
        });
        same_float(&format!("Math_Atan2XZ {v:?}"), r.st0, want);
        // Math_FacingWithin: finite headings (Math_WrapAngle never ends
        // for an infinity), tolerances around the difference, NaN
        let heading = if round % 7 == 0 {
            f32::NAN
        } else {
            f32::from(i16::try_from(t.rng.below(4000)).unwrap_or(0) - 2000) / 100.0
        };
        let tol = match t.rng.below(6) {
            0 => f32::NAN,
            1 => -1.0,
            2 => 0.0,
            _ => f32::from(u16::try_from(t.rng.below(400)).unwrap_or(0)) / 100.0,
        };
        let q: [f32; 4] = std::array::from_fn(|_| pick(&mut t));
        let args = [heading, tol, q[0], q[1], q[2], q[3]].map(f32::to_bits);
        t.twin("Math_FacingWithin", 0x0088_b780, &args, U32, |o| {
            w(tk::math_facing_within(
                o,
                heading,
                tol,
                [q[0], q[1]],
                [q[2], q[3]],
                &mut |o, a, b| x87_call(o, 0x00b0_c3b0, a, Some(b)),
            ))
        });
        // Math_PolarToVector: finite angles (Math_WrapAngle never ends for
        // an infinity), around ±π/2 and beyond a turn; NaN
        let angle = |t: &mut Twins| -> f32 {
            match t.rng.below(8) {
                0 => f32::NAN,
                1 => t.rng.pick(&[
                    std::f32::consts::FRAC_PI_2,
                    -std::f32::consts::FRAC_PI_2,
                    std::f32::consts::PI,
                ]),
                _ => f32::from(i16::try_from(t.rng.below(2000)).unwrap_or(0) - 1000) / 100.0,
            }
        };
        let (yaw, pitch) = (angle(&mut t), angle(&mut t));
        let r = pick(&mut t);
        let out = t.alloc_random(16);
        let args = [out, r.to_bits(), yaw.to_bits(), pitch.to_bits()];
        t.twin("Math_PolarToVector", 0x0088_a3a0, &args, VOID, |o| {
            tk::math_polar_to_vector(o, out, [r, yaw, pitch], &mut |o, cos, v| {
                x87_call(o, if cos { 0x00b0_c3b4 } else { 0x00b0_c3b8 }, v, None)
            });
            0
        });
        // Math_YawPitchTo from and to points sharing axes or not
        let mut p: [f32; 6] = std::array::from_fn(|_| pick(&mut t));
        match round % 4 {
            1 => p[3] = p[0],
            2 => p[5] = p[2],
            3 => {
                let k = t.rng.below(3) as usize;
                p[k] = f32::INFINITY;
                p[3 + k] = f32::INFINITY;
            }
            _ => {}
        }
        let (yaw, pitch) = (t.alloc_random(4), t.alloc_random(4));
        let pitch = if round % 8 == 0 { yaw } else { pitch };
        let mut args = vec![yaw, pitch];
        args.extend(p.map(f32::to_bits));
        t.twin("Math_YawPitchTo", 0x0088_b4d0, &args, U32, |o| {
            w(tk::math_yaw_pitch_to(
                o,
                yaw,
                pitch,
                [p[0], p[1], p[2]],
                [p[3], p[4], p[5]],
                &mut |o, a, b| x87_call(o, 0x00b0_c3b0, a, Some(b)),
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_cursor_and_event_jump_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_e7e7) else {
        return;
    };
    // graphicVFXDestroyAll (the renderer's effects): ret in both
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x0064_5310);
        o.write(at, &[0xc3]).unwrap();
    }
    for round in 0..64u32 {
        let summon = t.rng.below(3);
        for (pos, step) in [(0x0184_1c54u32, 0x0184_0887u32), (0x0184_1c28, 0x0184_0886)] {
            let v = t.rng.below(4);
            t.put(t.sym(pos), v);
            let s = u8::try_from(t.rng.below(0x100)).unwrap_or(0);
            t.put(t.sym(step), s);
        }
        let counter = t.alloc_random(4);
        let c = match round % 4 {
            0 => 0,
            1 => t.rng.below(0x400),
            2 => 0u32.wrapping_sub(t.rng.below(0x400)),
            _ => t.rng.next(),
        };
        t.put(counter, c);
        let (a, b, last) = (t.alloc_random(4), t.alloc_random(4), t.alloc_random(4));
        let l = t.rng.below(4);
        t.put(last, l);
        let args = [summon, counter, a, b, last];
        t.twin("Menu_CursorEase", 0x008b_f750, &args, VOID, |o| {
            tk::menu_cursor_ease(o, summon == 1, [counter, a, b, last], &mut callback);
            0
        });
        // TkSetEventJump on the save's event block
        let block = t.rng.bytes(0x100);
        t.put_bytes(t.sym(0x0112_ca90), &block);
        let pending = if round % 5 == 0 {
            0x17
        } else {
            t.rng.below(0x200)
        };
        t.put(t.sym(0x00c3_3480), pending);
        let map = match round % 4 {
            0 => 0x17,
            1 => 0u32.wrapping_sub(1 + t.rng.below(4)),
            2 => t.rng.below(0x200),
            _ => t.rng.next(),
        };
        let entrance = t.rng.next();
        t.twin("TkSetEventJump", 0x0088_ea60, &[map, entrance], VOID, |o| {
            tk::tk_set_event_jump(o, map.cast_signed(), entrance, &mut callback);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

/// `f` replaced in both oracles by a recorder appending its first three
/// arguments at `*log` and returning the first plus one.
fn record3(t: &mut Twins, f: u32) {
    let buf = t.alloc(&[0; 0x1000]);
    let log = t.alloc(&buf.to_le_bytes());
    let l = log.to_le_bytes();
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x0d][..], &l,                      // mov ecx, [log]
        &[0x8b, 0x44, 0x24, 0x04, 0x89, 0x01],      // [ecx] = arg 0
        &[0x8b, 0x44, 0x24, 0x08, 0x89, 0x41, 0x04], // [ecx + 4] = arg 1
        &[0x8b, 0x44, 0x24, 0x0c, 0x89, 0x41, 0x08], // [ecx + 8] = arg 2
        &[0x83, 0x05], &l, &[0x0c],                 // add dword [log], 12
        &[0x8b, 0x44, 0x24, 0x04, 0x40, 0xc3],      // eax = arg 0 + 1; ret
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(f);
        o.write(at, &code).unwrap();
    }
}

/// `f` replaced in both oracles by `mov eax, v; ret`.
fn return_const(t: &mut Twins, f: u32, v: u32) {
    let code = [&[0xb8][..], &v.to_le_bytes(), &[0xc3]].concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(f);
        o.write(at, &code).unwrap();
    }
}

#[test]
fn menu_selectors_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_5e1e) else {
        return;
    };
    let w = u64::from;
    // the characters' model calls, recorded
    record3(&mut t, 0x0082_aea0);
    record3(&mut t, 0x0083_2630);
    for round in 0..64u32 {
        // Math_AngleStep
        let pick = |t: &mut Twins| -> f32 {
            match t.rng.below(8) {
                0 => 0.0,
                1 => f32::NAN,
                _ => f32::from(i16::try_from(t.rng.below(2000)).unwrap_or(0) - 1000) / 100.0,
            }
        };
        let v: [f32; 3] = std::array::from_fn(|_| pick(&mut t));
        t.drain_x87();
        let r =
            t.a.call(t.a.rebase(0x0088_b570), Conv::Cdecl, &v.map(f32::to_bits))
                .unwrap();
        same_float(
            &format!("Math_AngleStep {v:?}"),
            r.st0,
            ffx_menu::math::angle_step(v[0], v[1], v[2]),
        );
        // the item tables, by language and target
        let lang = if round % 2 == 0 {
            0
        } else {
            1 + t.rng.below(10)
        };
        return_const(&mut t, 0x008a_c2f0, lang);
        let target = if round % 3 == 0 {
            0u32.wrapping_sub(1)
        } else {
            t.rng.below(0x12)
        };
        t.put(t.sym(tk::MENU_ITEM_TARGET.sym), target);
        t.twin("MenuItem_CursorTable", 0x008c_4ff0, &[], U32, |o| {
            w(tk::menu_item_cursor_table(o, &mut callback))
        });
        // the table as the image has it, or cut short at a random entry
        if round % 4 == 3 {
            let f = t.sym(0x008c_4ff0);
            let table = callback(&mut t.b, f, &[]);
            let k = t.rng.below(6);
            let v = u16::try_from(t.rng.below(3)).unwrap_or(0);
            t.put(table + 4 * k, v);
        }
        t.twin("MenuItem_CursorCount", 0x008c_5030, &[], U32, |o| {
            w(tk::menu_item_cursor_count(o, &mut callback))
        });
        t.twin("MenuItem_CellTable", 0x008c_4f80, &[], U32, |o| {
            w(tk::menu_item_cell_table(o, &mut callback))
        });
        // MenuShop_Price over a stand-in record
        let mut rec = t.rng.bytes(8);
        if round % 4 == 0 {
            rec[..4].copy_from_slice(&t.rng.below(5).to_le_bytes());
        }
        let rec = t.alloc(&rec);
        return_const(&mut t, 0x008c_1af0, rec);
        let ratio = if round % 2 == 0 {
            t.rng.below(300)
        } else {
            t.rng.next()
        };
        t.put(t.sym(0x0132_6b6c), ratio);
        let id = t.rng.next();
        t.twin("MenuShop_Price", 0x008d_b3b0, &[id], U32, |o| {
            w(tk::menu_shop_price(o, id, &mut callback).cast_unsigned())
        });
        // TkCh_BindToElem
        let kind = if round % 2 == 0 { 7 } else { t.rng.below(10) };
        let chr = if round % 5 == 0 { 0 } else { t.rng.next() };
        let keep = t.rng.below(2);
        let (a, b) = (t.rng.next(), t.rng.next());
        let args = [kind, a, chr, b, keep];
        t.twin("TkCh_BindToElem", 0x0087_22b0, &args, U32, |o| {
            w(tk::tk_ch_bind_to_elem(o, args, &mut callback))
        });
        // TkMenuMainRemoveMenu with a window in the slot (or none)
        let slot = t.rng.below(0x20).wrapping_sub(4);
        let win = if round % 4 == 0 {
            0
        } else {
            menu_window(&mut t)
        };
        if win != 0 {
            let f = t.rng.pick(&[0, t.sym(0x008c_f950)]);
            t.put(win + 0x14, f);
        }
        let at = t.sym(tk::MENU_SLOTS.sym) + 4 * slot.cast_signed().clamp(0, 0x18).cast_unsigned();
        t.put(at, win);
        let mask = t.rng.next();
        t.put(t.sym(tk::MENU_MAIN_ACTIVE_MASK.sym), mask);
        t.twin("TkMenuMainRemoveMenu", 0x008a_ab20, &[slot], U32, |o| {
            w(tk::tk_menu_main_remove_menu(
                o,
                slot.cast_signed(),
                &mut callback,
            ))
        });
        if round % 8 == 0 {
            t.twin("MenuNameEntry_RegistWindow", 0x008c_4450, &[], U32, |o| {
                w(tk::menu_name_entry_regist_window(o, &mut callback))
            });
        }
        if round % 8 == 4 {
            let index = t.rng.pick(&[0u32, 1, 2, 3, 4, 5, 6]);
            let args = [t.rng.next(), index, t.rng.next(), t.rng.next()];
            t.twin("MenuStatus_OpenWindow", 0x008d_3140, &args, U32, |o| {
                w(tk::menu_status_open_window(o, args, &mut callback))
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_weapon_prices_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_9370) else {
        return;
    };
    for round in 0..64u32 {
        let rec = t.alloc_random(8);
        return_const(&mut t, 0x008c_1af0, rec);
        if round % 3 == 0 {
            let small = t.rng.below(200);
            t.put(rec, small);
        }
        let mut e = t.rng.bytes(0x20);
        e[0xb] = u8::try_from(t.rng.below(6)).unwrap_or(0);
        for k in 0..4 {
            let v: u16 = match t.rng.below(4) {
                0 => 0,
                1 => 0xff,
                _ => u16::try_from(t.rng.below(0x10000)).unwrap_or(0),
            };
            e[0xe + 2 * k..0x10 + 2 * k].copy_from_slice(&v.to_le_bytes());
        }
        let e = t.alloc(&e);
        let ratio = if round % 2 == 0 {
            t.rng.below(300)
        } else {
            t.rng.next()
        };
        t.put(t.sym(0x0132_6b6c), ratio);
        t.twin("MenuShop_WeaponPrice", 0x008d_9370, &[e], U32, |o| {
            u64::from(tk::menu_shop_weapon_price(o, e, &mut callback).cast_unsigned())
        });
        t.twin("MenuShop_WeaponSellPrice", 0x008d_9410, &[e], U32, |o| {
            u64::from(tk::menu_shop_weapon_sell_price(o, e, &mut callback))
        });
        // what the shop compares the prices with: the gil (MsGetGIL) and
        // the count held (MsGetSaveItemNum), constants around the edges
        let gil = match t.rng.below(3) {
            0 => t.rng.next(),
            _ => t.rng.below(0x400),
        };
        return_const(&mut t, 0x0078_4e80, gil);
        let held = t.rng.pick(&[0u32, 98, 99, 100, 0xffff_ffff]);
        return_const(&mut t, 0x0079_04b0, held);
        let open = t.rng.pick(&[0u32, 1, 0x100]);
        t.put(t.sym(0x0186_ab14), open);
        if round % 4 == 1 {
            // exactly the price
            let f = t.sym(0x008d_9370);
            let price = callback(&mut t.b, f, &[e]);
            return_const(&mut t, 0x0078_4e80, price);
        }
        t.twin("MenuWShop_CanAfford", 0x008d_85d0, &[e], U32, |o| {
            u64::from(tk::menu_wshop_can_afford(o, e, &mut callback))
        });
        let id = t.rng.next();
        if round % 4 == 3 {
            let f = t.sym(0x008d_b3b0);
            let price = callback(&mut t.b, f, &[id]);
            return_const(&mut t, 0x0078_4e80, price);
        }
        t.twin("MenuShop_CanBuy", 0x008d_ab30, &[id], U32, |o| {
            u64::from(tk::menu_shop_can_buy(o, id, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_out_pointer_callers_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_5750) else {
        return;
    };
    // the originals' locals: the bottom of the bridge side's emulated
    // stack, which the diff leaves out as it does the originals' frames
    let local = t.b.stack_range().start + 0x100;
    // MsGetSaveWeaponName (the local's value, a text lookup the image
    // has no tables for): a constant in both
    return_const(&mut t, 0x007a_bdf0, 0x1234);
    let w = u64::from;
    for round in 0..96u32 {
        // a weapon in the inventory (< 200), the spoils (0xb000 + < 8) or
        // the 0x7000 slots, its record randomised
        let id = match round % 3 {
            0 => t.rng.below(0xd0),
            1 => 0xb000 + t.rng.below(10),
            _ => 0x7000 + t.rng.below(10),
        };
        let base = match id >> 12 {
            0xb => 0x0231_0dde,
            7 => 0x0231_0f9e,
            _ => 0x0113_0f2c,
        };
        let k = (id & 0xfff) as usize;
        let k = if (id >> 12 == 0 && k < 200) || (id >> 12 != 0 && k < 8) {
            k
        } else {
            0
        };
        let mut rec = t.rng.bytes(0x16);
        rec[2] = u8::try_from(t.rng.below(2)).unwrap_or(0);
        rec[3] = u8::try_from(t.rng.below(0x10)).unwrap_or(0);
        t.put_bytes(t.sym(base) + 0x16 * u32::try_from(k).unwrap_or(0), &rec);
        let any = t.rng.below(2);
        t.twin(
            "MenuEquip_IsSelectable",
            0x008d_5750,
            &[id, any],
            U32,
            |o| {
                w(tk::menu_equip_is_selectable(
                    o,
                    id,
                    any,
                    local,
                    &mut callback,
                ))
            },
        );
        // an item record of each kind, through a stand-in TkMsGetRomItem
        let mut item = t.rng.bytes(0x40);
        item[0x28] = u8::try_from(5 + t.rng.below(5)).unwrap_or(0);
        let item = t.alloc(&item);
        return_const(&mut t, 0x008a_b280, item);
        let player = t.rng.below(0x12);
        let mp = t.rng.below(2);
        let args = [t.rng.next(), player, mp];
        t.twin("TkMenuItemHealAmount", 0x008c_69a0, &args, U32, |o| {
            w(tk::tk_menu_item_heal_amount(o, args, local, &mut callback).cast_unsigned())
        });
    }
    eprintln!("{} calls", t.calls);
}

/// `f` replaced in both oracles by a recorder appending its first `n`
/// argument words at `*log` (128 KiB of them); returns `ret`. The code
/// takes `19 + 7n` bytes: `f` must be at least that long.
fn record_n(t: &mut Twins, f: u32, n: u8, ret: u32) -> u32 {
    let buf = t.alloc(&vec![0; 0x2_0000]);
    let log = t.alloc(&buf.to_le_bytes());
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat(); // mov ecx, [log]
    for k in 0..n {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
        code.extend([0x89, 0x41, 4 * k]); // mov [ecx + 4k], eax
    }
    code.extend([&[0x83, 0x05][..], &l, &[4 * n]].concat()); // add dword [log], 4n
    code.push(0xb8); // mov eax, ret
    code.extend(ret.to_le_bytes());
    code.push(0xc3);
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(f);
        o.write(at, &code).unwrap();
    }
    buf
}

/// The UI remaps through the originals, for the bridge side.
fn ui_remap(o: &mut Oracle, y: bool, v: f32) -> f64 {
    let at = o.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
    o.call(at, Conv::Cdecl, &[v.to_bits()])
        .unwrap()
        .st0
        .unwrap()
}

#[test]
fn draw_cross_menu_icons_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_6700) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    // the drawing (DrawCrossMenuIconXYWHRGBA, ToMakeBtlEasyFont) and the
    // name lookups, recorded or constant
    record_n(&mut t, 0x008e_6af0, 9, 0x51);
    record_n(&mut t, 0x0090_5ab0, 6, 0x52);
    return_const(&mut t, 0x007a_bdf0, 0x00c0_1234);
    record_n(&mut t, 0x007a_0c50, 3, 0x00c0_5678);
    return_const(&mut t, 0x0078_51f0, 1);
    let pick = |t: &mut Twins| -> u32 {
        match t.rng.below(6) {
            0 => f32::NAN.to_bits(),
            1 => t.rng.next(),
            _ => (f32::from(i16::try_from(t.rng.below(1000)).unwrap_or(0)) / 2.0).to_bits(),
        }
    };
    for round in 0..64u32 {
        let (x, y, colour) = (pick(&mut t), pick(&mut t), t.rng.next());
        let xy = [f32::from_bits(x), f32::from_bits(y)];
        // MsGetRomItem: a record, the text base written through the out
        let rec = t.alloc_random(0x20);
        let base = t.rng.next();
        #[rustfmt::skip]
        let code: Vec<u8> = [
            &[0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00][..], &base.to_le_bytes(), // *[esp + 8] = base
            &[0xb8], &rec.to_le_bytes(), &[0xc3],                           // eax = rec; ret
        ]
        .concat();
        for o in [&mut t.a, &mut t.b] {
            let at = o.rebase(0x0079_09f0);
            o.write(at, &code).unwrap();
        }
        let item = t.rng.next();
        t.twin(
            "DrawCrossMenuIconItemName",
            0x008e_6700,
            &[item, x, y, colour],
            U32,
            |o| {
                w64(tk::draw_cross_menu_icon_item_name(
                    o,
                    item,
                    xy,
                    colour,
                    local,
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
        // a weapon slot, its +4 sometimes equal to +6
        let id = t.rng.below(0xc8);
        let mut e = t.rng.bytes(0x16);
        if round % 2 == 0 {
            e[6] = e[4];
        }
        t.put_bytes(t.sym(0x0113_0f2c) + 0x16 * id, &e);
        t.twin(
            "DrawCrossMenuIconWeaponName",
            0x008e_67f0,
            &[id, x, y, colour],
            VOID,
            |o| {
                tk::draw_cross_menu_icon_weapon_name(
                    o,
                    id,
                    [x, y, colour],
                    local,
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
        let e = t.alloc(&e);
        t.twin(
            "DrawCrossMenuIconWeaponName2",
            0x008e_6970,
            &[e, x, y, colour],
            VOID,
            |o| {
                tk::draw_cross_menu_icon_weapon_name2(
                    o,
                    e,
                    [x, y, colour],
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

fn w64(v: u32) -> u64 {
    u64::from(v)
}

#[test]
fn tk_mn2_draw_gill_plate_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_0a20) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    // TOMkpShapeXYWHUV and ToMakeBtlEasyEdgeDigit2 recorded; the digit
    // width writes a float through its out pointer; the language a constant
    record_n(&mut t, 0x0090_3bb0, 9, 0x61);
    record_n(&mut t, 0x0090_5820, 6, 0x62);
    let pick = |t: &mut Twins| -> u32 {
        match t.rng.below(8) {
            // no signalling NaNs: the emulator keeps them on fld, the CPU
            // makes them quiet
            0 | 1 => f32::NAN.to_bits(),
            _ => (f32::from(i16::try_from(t.rng.below(1000)).unwrap_or(0)) / 4.0).to_bits(),
        }
    };
    for round in 0..64u32 {
        let width = pick(&mut t);
        #[rustfmt::skip]
        let code: Vec<u8> = [
            &[0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00][..], &width.to_le_bytes(), // *[esp + 8] = width
            &[0x31, 0xc0, 0xc3],                                            // eax = 0; ret
        ]
        .concat();
        for o in [&mut t.a, &mut t.b] {
            let at = o.rebase(0x0090_5230);
            o.write(at, &code).unwrap();
        }
        let lang = if round % 2 == 0 { 3 } else { t.rng.below(11) };
        return_const(&mut t, 0x008a_c2f0, lang);
        let args: [u32; 4] = std::array::from_fn(|_| pick(&mut t));
        let gil = t.rng.next();
        let call = [args[0], args[1], args[2], args[3], gil];
        t.twin("TkMn2DrawGillPlate", 0x008c_0a20, &call, U32, |o| {
            u64::from(tk::tk_mn2_draw_gill_plate(
                o,
                args.map(f32::from_bits),
                gil,
                local,
                &mut ui_remap,
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn tk_menu_list_window_ctrl_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_44b0) else {
        return;
    };
    // SndSepPlaySimple recorded; the pad getters answer per case
    record_n(&mut t, 0x0088_6e80, 1, 0);
    let checks = [0, t.sym(0x008c_f950), t.sym(0x008b_5530)];
    for round in 0..2000u32 {
        let state = match round % 4 {
            0 => 2,
            _ => t.rng.below(0x15),
        };
        let mut w = t.rng.bytes(0x60);
        w[0x28..0x2c].copy_from_slice(&state.to_le_bytes());
        let count = u16::try_from(t.rng.below(40)).unwrap_or(0);
        let rows = u16::try_from(1 + t.rng.below(8)).unwrap_or(1);
        let top = i16::try_from(t.rng.below(40)).unwrap_or(0) - 4;
        let cursor = i16::try_from(t.rng.below(48)).unwrap_or(0) - 4;
        w[0x30..0x32].copy_from_slice(&count.to_le_bytes());
        w[0x32..0x34].copy_from_slice(&top.to_le_bytes());
        w[0x3a..0x3c].copy_from_slice(&rows.to_le_bytes());
        w[0x48..0x4a].copy_from_slice(&cursor.to_le_bytes());
        w[0x42] = u8::try_from(t.rng.below(5)).unwrap_or(0);
        w[0x45] = u8::try_from(t.rng.below(6)).unwrap_or(0).wrapping_sub(1);
        w[0x37] = u8::try_from(t.rng.below(2)).unwrap_or(0);
        if round % 3 == 0 {
            let timer = u16::try_from(t.rng.below(0x400))
                .unwrap_or(0)
                .wrapping_sub(0x200);
            w[0x46..0x48].copy_from_slice(&timer.to_le_bytes());
        }
        let check = t.rng.pick(&checks);
        w[0x1c..0x20].copy_from_slice(&check.to_le_bytes());
        let w = t.alloc(&w);
        let bits = [0u32, 1, 2, 0x20, 0x40, 0x1000, 0x4000, 3, 0x1001, 0x4002];
        let pad = t.rng.pick(&[0u32, 1, 2, 3]);
        let rep = t.rng.pick(&bits) | (t.rng.next() & 0xffff_0000);
        let trg = if round % 5 == 0 {
            t.rng.next()
        } else {
            t.rng.pick(&bits)
        };
        return_const(&mut t, 0x008b_e410, pad);
        return_const(&mut t, 0x008b_e470, rep);
        return_const(&mut t, 0x008b_e4b0, trg);
        t.twin("TkMenuListWindowCtrl", 0x008b_44b0, &[w], VOID, |o| {
            tk::tk_menu_list_window_ctrl(o, w, &mut callback);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_item_keys_match_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_5390) else {
        return;
    };
    for round in 0..400u32 {
        // a key string of tokens of each shape (kind 4's second byte on
        // both sides of 0x2f; 6..=0x2f with an argument; lone bytes)
        let mut keys = Vec::new();
        for _ in 0..t.rng.below(6) {
            match t.rng.below(4) {
                0 => {
                    let c = u8::try_from(t.rng.below(0x40)).unwrap_or(0).max(1);
                    keys.extend([4, c]);
                    if c <= 0x2f || t.rng.below(4) == 0 {
                        keys.push(u8::try_from(1 + t.rng.below(0xff)).unwrap_or(1));
                    }
                }
                1 => {
                    let any = u8::try_from(6 + t.rng.below(0x2a)).unwrap_or(6);
                    let b = t.rng.pick(&[6, 0x2f, any]);
                    keys.extend([b, u8::try_from(1 + t.rng.below(0xff)).unwrap_or(1)]);
                }
                _ => keys.push(t.rng.pick(&[1u8, 2, 3, 5, 0x30, 0x31, 0x80, 0xff])),
            }
        }
        keys.push(0);
        let keys = t.alloc(&keys);
        // the entries Menu_ParseItemKeys makes of it, sometimes from
        // another string or with one byte changed
        let parsed = if round % 4 == 0 {
            let mut other: Vec<u8> = (0..t.rng.below(6))
                .map(|_| t.rng.pick(&[1u8, 4, 6, 0x10, 0x30, 0x40]))
                .collect();
            other.push(0);
            t.alloc(&other)
        } else {
            keys
        };
        let f = t.sym(0x008c_5280);
        for o in [&mut t.a, &mut t.b] {
            o.call(f, Conv::Cdecl, &[parsed]).unwrap();
        }
        if round % 2 == 1 {
            // mostly in the first entries, where the tokens are
            let k = t.rng.below(if round % 4 == 1 { 0xc } else { 0x20 });
            let v = u8::try_from(t.rng.below(4)).unwrap_or(0);
            t.put(t.sym(tk::ITEM_KEYS) + k, v);
        }
        t.a.clear_dirty();
        t.b.clear_dirty();
        // TOMenuGetNameMes: the string (or an empty one)
        let s = if round % 16 == 0 { t.alloc(&[0]) } else { keys };
        return_const(&mut t, 0x008b_edd0, s);
        let target = t.rng.below(0x14).wrapping_sub(2);
        t.twin("MenuItem_KeysMatch", 0x008c_5390, &[target], U32, |o| {
            u64::from(tk::menu_item_keys_match(
                o,
                target.cast_signed(),
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round18_windows_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_9140) else {
        return;
    };
    // the windows come from the 32 slots: a dozen calls at most
    for round in 0..6u32 {
        // the item and key-item counts (TOAmapGetItemListMax,
        // TkMenuGetImportantCounter), odd, even and negative
        let n = t
            .rng
            .pick(&[0u32, 1, 2, 7, 0xffff_ffff, 0xffff_fffe, 0x2_0001]);
        return_const(&mut t, 0x008c_8c30, n);
        return_const(&mut t, 0x008c_8a10, n.wrapping_add(1));
        t.scramble(0x0186_a3ac, 4);
        let kind = if round < 4 { round } else { t.rng.next() };
        t.twin("MenuItem_OpenListWindow", 0x008c_9140, &[kind], U32, |o| {
            u64::from(tk::menu_item_open_list_window(o, kind, &mut callback))
        });
        let chr = t.rng.below(0x14);
        t.scramble(0x023c_c3bc, 0x10);
        t.twin(
            "MenuNameEntry_OpenCharWindow",
            0x008c_43f0,
            &[chr],
            U32,
            |o| u64::from(tk::menu_name_entry_open_char_window(o, chr, &mut callback)),
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round18_checks_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_6a70) else {
        return;
    };
    // Msg_CloseWindow recorded; the message status and the control work
    // stand-ins
    record_n(&mut t, 0x0086_40f0, 1, 0);
    let ctrl = t.alloc_random(4);
    return_const(&mut t, 0x0086_aff0, ctrl);
    for round in 0..200u32 {
        // HP and MP around their maxima, some negative
        let chr = t.rng.below(0x12);
        let ply = ffx_battle::save::bridge::ms_get_save_player_ptr(&t.b, chr);
        let base = t.rng.below(1000);
        for o in [0x1cu32, 0x20, 0x24, 0x28] {
            let v = match t.rng.below(4) {
                0 => t.rng.next(),
                _ => base + t.rng.below(3),
            };
            t.put(ply + o, v);
        }
        let kind = if round % 8 == 7 {
            t.rng.next()
        } else {
            t.rng.below(5)
        };
        let all = t.rng.next();
        t.twin(
            "MenuItem_TargetNeedsRecovery",
            0x008c_6a70,
            &[kind, all, chr],
            U32,
            |o| u64::from(tk::menu_item_target_needs_recovery(o, kind, chr)),
        );
        // Msg_PollClose over the window bank's states
        let bank = u8::try_from(t.rng.below(2)).unwrap_or(0);
        t.put(t.sym(0x0132_6b82), bank);
        let i = t.rng.pick(&[0u32, 1, 3, 7, 8, 0xffff_ffff]);
        let f = t.sym(0x0086_be90);
        let window = callback(&mut t.b, f, &[i]);
        let state = t.rng.pick(&[0u16, 1, 2, 2, 3, 4, 5, 0x102]);
        t.put(window + 0x14, state);
        let status = t.rng.pick(&[3u32, 3, 2, 0x103]);
        return_const(&mut t, 0x008b_7ca0, status);
        let bits = u8::try_from(t.rng.below(0x100)).unwrap_or(0);
        t.put(ctrl, bits);
        t.twin("Msg_PollClose", 0x0086_0aa0, &[i], U32, |o| {
            u64::from(tk::msg_poll_close(o, i, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_summon_grow_info_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_d320) else {
        return;
    };
    // MenuSummon_GrowEntry: a random entry; the descriptors' gain
    // functions: a recorder (in the body of an unused debug function)
    // returning a value set per case
    let entry = t.alloc_random(8);
    return_const(&mut t, 0x008c_ecf0, entry);
    let out = t.alloc_random(8);
    record_n(&mut t, 0x0088_fd50, 2, 0);
    for _ in 0..300u32 {
        let stat = t.rng.below(14).wrapping_sub(2);
        let chr = t.rng.below(0x12);
        let ply = ffx_battle::save::bridge::ms_get_save_player_ptr(&t.b, chr);
        let rec = t.rng.bytes(0x94);
        t.put_bytes(ply, &rec);
        // the descriptor read (stat < 10, signed; 9 for the others)
        let k = if stat.cast_signed() < 10 { stat } else { 9 };
        let desc = t.sym(0x00c5_a608).wrapping_add(0x1c_u32.wrapping_mul(k));
        // the limit near the maximum the descriptor reads
        let max = tk::menu_summon_get_stat(&t.b, ply, desc, 1);
        let limit = match t.rng.below(4) {
            0 => t.rng.next(),
            _ => max.wrapping_add(t.rng.below(3)).wrapping_sub(1),
        };
        t.put(desc + 0x14, limit);
        let gain = t
            .rng
            .pick(&[0u32, 1, 50, 99, 100, 0xffff_ffff, 0x8000_0000]);
        // record_n's `mov eax, ret` immediate: 6 + 7 × 2 + 7 + 1 bytes in
        let f = t.sym(0x0088_fd50);
        t.put(f + 28, gain);
        t.put(desc + 0x18, f);
        let r = t.rng.bytes(4);
        t.put_bytes(entry, &r);
        let args = [stat, chr, out, out + 4];
        t.twin("MenuSummon_GrowInfo", 0x008c_d320, &args, U32, |o| {
            u64::from(tk::menu_summon_grow_info(
                o,
                stat.cast_signed(),
                chr,
                [out, out + 4],
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_name_entry_load_layout_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_5050) else {
        return;
    };
    // the plate and sequence lookups recorded, each into its own log
    record_n(&mut t, 0x008c_1a60, 6, 0x11);
    record_n(&mut t, 0x008c_2b10, 1, 0x22);
    record_n(&mut t, 0x008c_2ad0, 1, 0x33);
    record_n(&mut t, 0x008c_2b50, 2, 0x44);
    record_n(&mut t, 0x008c_2b90, 2, 0x55);
    for _ in 0..4 {
        t.scramble(0x0186_a300, 0x80);
        t.twin("MenuNameEntry_LoadLayout", 0x008c_5050, &[], U32, |o| {
            u64::from(tk::menu_name_entry_load_layout(o, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_summon_draw_face_slide_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_34a0) else {
        return;
    };
    // TkMn2DrawCrossCharFace and TkMn2SetFontShade recorded
    record_n(&mut t, 0x008c_0250, 5, 0x66);
    record_n(&mut t, 0x008c_2c30, 4, 0x77);
    let pick = |t: &mut Twins| -> u32 {
        match t.rng.below(8) {
            0 => f32::NAN.to_bits(),
            1 => t.rng.next() & 0xffbf_ffff | 0x0040_0000, // no signalling NaNs
            _ => (f32::from(i16::try_from(t.rng.below(2000)).unwrap_or(0) - 1000) / 4.0).to_bits(),
        }
    };
    for round in 0..300u32 {
        // the read-end flag
        let flag = t.rng.pick(&[0u32, 1, 0x100]);
        return_const(&mut t, 0x008a_8e50, flag);
        let step = match t.rng.below(3) {
            0 => t.rng.next(),
            _ => t.rng.below(0x200),
        };
        t.put(t.sym(0x0186_a9ec), step);
        t.scramble(0x023c_c458, 0x24);
        let xywh: [u32; 4] = std::array::from_fn(|_| pick(&mut t));
        let slide = match round % 4 {
            0 => 0,
            1 => t.rng.next(),
            _ => t.rng.below(0x2000).wrapping_sub(0x1000),
        };
        let cur = t.rng.below(12);
        let next = t.rng.below(12);
        let args = [xywh[0], xywh[1], xywh[2], xywh[3], slide, cur, next];
        t.twin("MenuSummon_DrawFaceSlide", 0x008d_34a0, &args, U32, |o| {
            u64::from(tk::menu_summon_draw_face_slide(
                o,
                xywh,
                slide.cast_signed(),
                [cur, next],
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn math_segments_cross_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_9bb0) else {
        return;
    };
    // points on a small grid (so collinear and touching cases come up),
    // sometimes anywhere, NaN or infinite
    let coord = |t: &mut Twins| -> f32 {
        match t.rng.below(16) {
            0 => f32::NAN,
            1 => f32::INFINITY,
            2 => f32::from(i16::try_from(t.rng.below(2000)).unwrap_or(0) - 1000) / 7.0,
            _ => f32::from(i8::try_from(t.rng.below(7)).unwrap_or(0) - 3),
        }
    };
    for round in 0..3000u32 {
        let mut pts = [0u32; 4];
        for p in &mut pts {
            let v: Vec<u8> = [
                coord(&mut t),
                f32::from_bits(t.rng.next() & 0x7fbf_ffff), // y, never read
                coord(&mut t),
            ]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
            *p = t.alloc(&v);
        }
        if round % 5 == 0 {
            // a shared end point
            let k = t.rng.below(2) as usize;
            pts[2 + k] = pts[t.rng.below(2) as usize];
        }
        t.twin("Math_SegmentsCross", 0x0088_9bb0, &pts, U32, |o| {
            u64::from(tk::math_segments_cross(o, pts).cast_unsigned())
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_summon_build_command_list_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_2b40) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    // TkMenuGetCommandMax and TkMenuGetCommandOffset: a word per group
    // (`mov eax, [esp + 0x10]; mov eax, [eax*4 + table]; ret`);
    // MsGetSaveCommand: a byte per id's low 6 bits
    let max = t.alloc(&[0; 32]);
    let off = t.alloc(&[0; 32]);
    let has = t.alloc(&[0; 64]);
    for (f, table) in [(0x008d_2d80u32, max), (0x008d_2dc0, off)] {
        let code = [
            &[0x8b, 0x44, 0x24, 0x10, 0x8b, 0x04, 0x85][..],
            &table.to_le_bytes(),
            &[0xc3],
        ]
        .concat();
        for o in [&mut t.a, &mut t.b] {
            let at = o.rebase(f);
            o.write(at, &code).unwrap();
        }
    }
    #[rustfmt::skip]
    let code = [
        &[0x8b, 0x44, 0x24, 0x08, 0x83, 0xe0, 0x3f, 0x0f, 0xb6, 0x80][..], &has.to_le_bytes(), &[0xc3],
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x0078_5020);
        o.write(at, &code).unwrap();
    }
    for round in 0..300u32 {
        let list = t.alloc_random(0x100);
        return_const(&mut t, 0x0079_0200, list);
        for g in 0..8u32 {
            let n = match t.rng.below(6) {
                0 => t.rng.pick(&[0u32, 0xffff_ffff]),
                1 => 8 + t.rng.below(8),
                _ => t.rng.below(5),
            };
            t.put(max + 4 * g, n);
            let o = t.rng.below(0x30);
            t.put(off + 4 * g, o);
        }
        let mut bits = t.rng.bytes(64);
        if round % 3 == 0 {
            for b in &mut bits {
                *b &= 1;
            }
        }
        t.put_bytes(has, &bits);
        t.scramble(0x0186_aa04, 0x28);
        let chr = t.rng.below(0x20);
        t.twin(
            "MenuSummon_BuildCommandList",
            0x008d_2b40,
            &[chr],
            U32,
            |o| {
                u64::from(tk::menu_summon_build_command_list(
                    o,
                    chr,
                    local,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round18_small_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_0cc0) else {
        return;
    };
    // SndSqsIopFrame and SndStatusUpdate (the sound driver): xor eax, eax;
    // ret; the packet builder 0x008f1b50, the sync-callback setters and
    // AbmapQuarterOrOne recorded
    for o in [&mut t.a, &mut t.b] {
        for f in [0x0088_7310, 0x0088_7700] {
            let at = o.rebase(f);
            o.write(at, &[0x31, 0xc0, 0xc3]).unwrap();
        }
    }
    // 0x008f1b50 logging the draw cursor it finds and its four arguments
    let log = t.alloc(&[0; 20]);
    let mut code = vec![0xa1];
    code.extend(t.sym(0x0186_a238).to_le_bytes()); // mov eax, [g_TkMn2DrawY]
    code.push(0xa3);
    code.extend(log.to_le_bytes()); // mov [log], eax
    for k in 1..5u8 {
        code.extend([0x8b, 0x44, 0x24, 4 * k]); // mov eax, [esp + 4k]
        code.push(0xa3);
        code.extend((log + 4 * u32::from(k)).to_le_bytes()); // mov [log + 4k], eax
    }
    code.push(0xb8);
    code.extend(0x0123_4560u32.to_le_bytes());
    code.push(0xc3);
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x008f_1b50);
        o.write(at, &code).unwrap();
    }
    record_n(&mut t, 0x0088_7e80, 1, 0x74);
    let rec = t.alloc_random(8);
    record_n(&mut t, 0x008c_1af0, 2, rec);
    record_n(&mut t, 0x0088_8620, 1, 0x71);
    record_n(&mut t, 0x0088_8790, 2, 0x72);
    record_n(&mut t, 0x00a5_cee0, 2, 0x73);
    let w = u64::from;
    for round in 0..200u32 {
        for a in [0x0186_a238u32, 0x0186_a240, 0x0186_a244, 0x0186_a248] {
            let v = match t.rng.below(4) {
                0 => 0x01ff_fff0 + t.rng.below(0x20),
                _ => t.rng.next(),
            };
            t.put(t.sym(a), v);
        }
        t.twin("TkMn2DrawKickSyncPacket", 0x008c_0cc0, &[], U32, |o| {
            w(tk::tk_mn2_draw_kick_sync_packet(o, 1, &mut callback))
        });
        // the scissor TkMn2DrawScissorClear would set
        t.scramble(0x0187_16d0, 0x40);
        t.twin("TkMn2DrawKickSyncPacket2", 0x008c_0cd0, &[], U32, |o| {
            w(tk::tk_mn2_draw_kick_sync_packet(o, 0, &mut callback))
        });
        let args = [t.rng.next(), t.rng.next(), t.rng.below(12)];
        t.twin("MenuMn2_EmitPacket", 0x008c_11b0, &args, U32, |o| {
            w(tk::menu_mn2_emit_packet(o, args, &mut callback))
        });
        for a in [
            0x0132_ff10u32,
            0x0132_ff14,
            0x0132_ff18,
            0x0132_ff1c,
            0x0132_ff20,
            0x0132_ff24,
        ] {
            let v = if t.rng.below(3) == 0 { t.rng.next() } else { 0 };
            t.put(t.sym(a), v);
        }
        t.twin("TkSndIsRead", 0x0088_6600, &[], U32, |o| {
            w(tk::tk_snd_is_read(o, &mut callback))
        });
        let (id, f) = (t.rng.next(), t.rng.next());
        t.twin("TkSetSyncVCallback", 0x0088_7ed0, &[id, f], U32, |o| {
            w(tk::tk_set_sync_v_callback(o, id, f, &mut callback))
        });
        let n = t
            .rng
            .pick(&[0u32, 1, 2, 0xffff_ffff, 0x8000_0000, 0x7fff_ffff]);
        t.put(t.sym(0x0133_01e0), n);
        t.twin("TkSyncVEI", 0x0088_7f40, &[], U32, |o| {
            w(tk::tk_sync_v_ei(o, &mut callback))
        });
        // MenuShop_SellPrice over a stand-in record
        let r = t.rng.bytes(8);
        t.put_bytes(rec, &r);
        t.scramble(0x0186_ac20, 0x10);
        let id = if round % 2 == 0 {
            t.rng.below(0x2000)
        } else {
            t.rng.next()
        };
        t.twin("MenuShop_SellPrice", 0x008d_b400, &[id], U32, |o| {
            w(tk::menu_shop_sell_price(o, id, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round18_batch7_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0015_56f0) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let w = u64::from;
    for round in 0..64u32 {
        // TkMenuStopScene over a stand-in MenuScene_ResetGroup entry (or none)
        let e = if round % 4 == 0 { 0 } else { t.alloc_random(8) };
        record_n(&mut t, 0x00a7_ba70, 3, e);
        let (scene, group) = (t.rng.next(), t.rng.next());
        t.twin("TkMenuStopScene", 0x00a7_c8a0, &[scene, group], U32, |o| {
            w(tk::tk_menu_stop_scene(o, scene, group, &mut callback))
        });
        // MenuKaizou_WeaponByte4: the list and a stand-in weapon record
        let list = t.alloc_random(0x100);
        t.put(t.sym(0x0186_aa38), list);
        let mut rec = t.rng.bytes(8);
        if round % 2 == 0 {
            rec[2] = 0;
        }
        let rec = t.alloc(&rec);
        record_n(&mut t, 0x007a_bbd0, 1, rec);
        let win = t.alloc_random(0x60);
        let k = i16::try_from(t.rng.below(0x60)).unwrap_or(0) - 0x10;
        t.put(win + 0x48, k);
        t.twin("MenuKaizou_WeaponByte4", 0x008d_56f0, &[win], U32, |o| {
            w(tk::menu_kaizou_weapon_byte4(o, win, local, &mut callback))
        });
        // MenuMain_BusyMask: the mask, the CD read and the callback queue
        let mask = if round % 2 == 0 { 0 } else { t.rng.next() };
        t.put(t.sym(tk::MENU_MAIN_ACTIVE_MASK.sym), mask);
        let cd = t.rng.pick(&[0u32, 1, 0x100]);
        return_const(&mut t, 0x0088_d1c0, cd);
        let que = t.rng.pick(&[0u32, 1, 2, 0x101]);
        return_const(&mut t, 0x0087_5180, que);
        t.twin("MenuMain_BusyMask", 0x008a_a610, &[], U32, |o| {
            w(tk::menu_main_busy_mask(o, &mut callback))
        });
        if round % 8 == 0 {
            t.twin("MenuWShop_OpenSubWindow4", 0x008d_83e0, &[], U32, |o| {
                w(tk::menu_wshop_open_sub_window4(o, &mut callback))
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round19_batch1_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0019_0001) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let w = u64::from;
    // the VRAM blocks recorded (buffer, GS address, size)
    let load = t.rng.next();
    record_n(&mut t, 0x008a_a710, 3, load);
    t.twin("MenuMain_RestoreVram", 0x008a_a690, &[], U32, |o| {
        w(tk::menu_main_restore_vram(o, &mut callback))
    });
    let store = t.rng.next();
    record_n(&mut t, 0x008a_ac10, 3, store);
    t.twin("MenuMain_SaveVram", 0x008a_ab90, &[], U32, |o| {
        w(tk::menu_main_save_vram(o, &mut callback))
    });
    // MenuAbility_ApplyHeal over recording stand-ins
    for round in 0..64u32 {
        let heal = t.rng.int();
        record_n(&mut t, 0x008c_6920, 2, heal);
        let added = t.rng.next();
        record_n(&mut t, 0x008c_8d10, 2, added);
        let mut rec = t.rng.bytes(0x40);
        rec[0x22] = t.rng.pick(&[0u8, 0x40, 0x80, 0xc0, 0x3f, 0xff]);
        let rec = t.alloc(&rec);
        record_n(&mut t, 0x0079_0a90, 1, rec);
        let aeons = t.rng.pick(&[0u32, 1, 3, 8, 0xffff_ffff]);
        return_const(&mut t, 0x008a_9bf0, aeons);
        let players = t.rng.pick(&[0u32, 1, 2, 7, 0x8000_0000]);
        return_const(&mut t, 0x008a_9b30, players);
        let chr = t.rng.below(0x20);
        record_n(&mut t, 0x008a_9bc0, 1, chr);
        record_n(&mut t, 0x008a_9ad0, 1, chr + 0x100);
        let amount = t.rng.int();
        record_n(&mut t, 0x008c_69a0, 3, amount);
        let single = t.rng.pick(&[0u8, 1, 2, 0xff]);
        t.put(t.sym(tk::ABILITY_SINGLE.sym), single);
        let cmd = u16::try_from(t.rng.below(0x1_0000)).unwrap_or(0);
        t.put(t.sym(tk::ABILITY_COMMAND.sym), cmd);
        let (target, user) = (t.rng.next(), t.rng.int());
        t.twin(
            "MenuAbility_ApplyHeal",
            0x008c_6d00,
            &[target, user],
            U32,
            |o| {
                w(tk::menu_ability_apply_heal(
                    o,
                    target,
                    user,
                    local,
                    &mut callback,
                ))
            },
        );
        // TkMn2DrawCommonPlateL: the plate writer recorded
        let end = t.rng.next();
        record_n(&mut t, 0x008f_0990, 8, end);
        let plate = t.alloc_random(0x18);
        let y = t.rng.next();
        t.put(t.sym(tk::MN2_DRAW_Y.sym), y);
        t.twin("TkMn2DrawCommonPlateL", 0x008b_f9b0, &[plate], U32, |o| {
            w(tk::tk_mn2_draw_common_plate_l(o, plate, &mut callback))
        });
        if round % 8 == 0 {
            t.twin("MenuWShop_OpenWindow3", 0x008d_a9c0, &[], U32, |o| {
                w(tk::menu_wshop_open_window3(o, &mut callback))
            });
            t.twin("MenuStatus_OpenMainWindow", 0x008d_3100, &[], U32, |o| {
                w(tk::menu_status_open_main_window(o, &mut callback))
            });
            t.twin("MenuSummon_OpenWindow", 0x008c_d1b0, &[], U32, |o| {
                w(tk::menu_summon_open_window(o, &mut callback))
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round19_batch2_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0019_0002) else {
        return;
    };
    let w = u64::from;
    for round in 0..64u32 {
        // TkRefreshHDDMode: the HDD names and the sound driver recorded
        let name = t.rng.next();
        record_n(&mut t, 0x0077_7eb0, 1, name);
        let wave = t.rng.next();
        record_n(&mut t, 0x0081_d470, 1, wave);
        let music = t.rng.next();
        record_n(&mut t, 0x0081_d3c0, 1, music);
        let hdd = t.rng.pick(&[0u8, 1, 0x80, 0xff]);
        t.put(t.sym(tk::HDD_MODE.sym), hdd);
        t.twin("TkRefreshHDDMode", 0x0088_e930, &[], U32, |o| {
            w(tk::tk_refresh_hdd_mode(o, &mut callback))
        });
        // SsSetProjectBase
        let base = t.rng.next();
        record_n(&mut t, 0x0081_eab0, 1, base);
        let path = t.rng.next();
        t.twin("SsSetProjectBase", 0x0088_7a90, &[path], U32, |o| {
            w(tk::ss_set_project_base(o, path, &mut callback))
        });
        // TkMn2StringWidth
        let size = t.rng.next();
        record_n(&mut t, 0x008b_7170, 1, size);
        let text = t.rng.next();
        t.twin("TkMn2StringWidth", 0x008c_2cb0, &[text], U32, |o| {
            w(tk::tk_mn2_string_width(o, text, &mut callback))
        });
        // MenuItem_LoadTargetKeys: a key string from a stand-in message
        let lang = t.rng.pick(&[0u32, 1, 2]);
        return_const(&mut t, 0x0064_10e0, lang);
        let n = t.rng.below(40) as usize;
        let mut keys: Vec<u8> = (0..n).map(|_| 1 + (t.rng.next() % 0x40) as u8).collect();
        keys.push(0);
        let keys = t.alloc(&keys);
        record_n(&mut t, 0x008b_edd0, 1, keys);
        t.scramble(tk::ITEM_KEYS, 0x80);
        let target = if round % 5 == 0 {
            t.rng.pick(&[-1i32, -2, i32::MIN])
        } else {
            i32::try_from(t.rng.below(18)).unwrap_or(0)
        };
        t.twin(
            "MenuItem_LoadTargetKeys",
            0x008c_5320,
            &[target.cast_unsigned()],
            VOID,
            |o| {
                tk::menu_item_load_target_keys(o, target, &mut callback);
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

/// `f` replaced in both oracles by a stand-in of `(stat, _, *a, *b)`:
/// `*a = A[stat]`, `*b = B[stat]`, returns `C[stat]` (tables of 10 words).
fn out2_by_index(t: &mut Twins, f: u32, a: &[u32], b: &[u32], c: &[u32]) {
    let words = |v: &[u32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let (ta, tb, tc) = (t.alloc(&words(a)), t.alloc(&words(b)), t.alloc(&words(c)));
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x4c, 0x24, 0x04][..],              // mov ecx, [esp + 4]
        &[0x8b, 0x04, 0x8d], &ta.to_le_bytes(),     // mov eax, [ecx*4 + A]
        &[0x8b, 0x54, 0x24, 0x0c, 0x89, 0x02],      // mov edx, [esp + 12]; mov [edx], eax
        &[0x8b, 0x04, 0x8d], &tb.to_le_bytes(),     // mov eax, [ecx*4 + B]
        &[0x8b, 0x54, 0x24, 0x10, 0x89, 0x02],      // mov edx, [esp + 16]; mov [edx], eax
        &[0x8b, 0x04, 0x8d], &tc.to_le_bytes(),     // mov eax, [ecx*4 + C]
        &[0xc3],
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(f);
        o.write(at, &code).unwrap();
    }
}

#[test]
fn menu_round19_batch3_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0019_0003) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let w = u64::from;
    for round in 0..64u32 {
        // MenuSummon_RefreshGrowMasks: stand-in grow infos, recorded item counts
        let items: Vec<u32> = (0..10).map(|_| t.rng.below(0x10000)).collect();
        let owned = t.rng.pick(&[0u32, 1, 5, 99, 0xffff_ffff]);
        let counts: Vec<u32> = (0..10)
            .map(|_| owned.wrapping_add(t.rng.below(5)).wrapping_sub(2))
            .collect();
        let grows: Vec<u32> = (0..10).map(|_| t.rng.pick(&[0u32, 1, 0x100])).collect();
        out2_by_index(&mut t, 0x008c_d320, &items, &counts, &grows);
        record_n(&mut t, 0x0079_04b0, 1, owned);
        t.scramble(tk::SUMMON_GROW_MASK.sym, 8);
        let chr = t.rng.next();
        t.twin(
            "MenuSummon_RefreshGrowMasks",
            0x008c_d3b0,
            &[chr],
            U32,
            |o| {
                w(tk::menu_summon_refresh_grow_masks(
                    o,
                    chr,
                    local,
                    &mut callback,
                ))
            },
        );
        // BtlEnd_GetSaveNextAP and BtlEnd_AddGil over recorders
        let ap = t.rng.next();
        record_n(&mut t, 0x0078_52b0, 1, ap);
        let arg = t.rng.int();
        t.twin("BtlEnd_GetSaveNextAP", 0x008b_da00, &[arg], U32, |o| {
            w(tk::btl_end_get_save_next_ap(o, arg, &mut callback))
        });
        let paid = t.rng.next();
        record_n(&mut t, 0x0078_59a0, 1, paid);
        t.twin("BtlEnd_AddGil", 0x008b_a3a0, &[arg], U32, |o| {
            w(tk::btl_end_add_gil(o, arg, &mut callback))
        });
        // TkMn2DrawKickPacket: the original body
        if round % 8 == 0 {
            t.twin("TkMn2DrawKickPacket", 0x008c_0c40, &[], U32, |o| {
                w(tk::tk_mn2_draw_kick_packet(o, &mut callback))
            });
        }
        // TkSyncVDI: the count from any value, SetGlobal013301f0 original
        t.scramble(0x0133_01e0, 4);
        t.twin("TkSyncVDI", 0x0088_7f20, &[], U32, |o| {
            w(tk::tk_sync_vdi(o, &mut callback))
        });
        // TkMenuRefreshPartyList over recorders
        let r1 = t.rng.next();
        record_n(&mut t, 0x008a_8f40, 1, r1);
        let r2 = t.rng.next();
        record_n(&mut t, 0x008a_9090, 1, r2);
        t.twin("TkMenuRefreshPartyList", 0x008a_ae10, &[], U32, |o| {
            w(tk::tk_menu_refresh_party_list(o, &mut callback))
        });
        // MenuMain_RemoveAllMenus over a recorder
        let r3 = t.rng.next();
        record_n(&mut t, 0x008a_ab20, 1, r3);
        t.twin("MenuMain_RemoveAllMenus", 0x008a_add0, &[], U32, |o| {
            w(tk::menu_main_remove_all_menus(o, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round19_batch4_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0019_0004) else {
        return;
    };
    let w = u64::from;
    let listed = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.999,
        -0.999,
        2.0,
        -2.0,
        1e9,
        -1e9,
        1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        1.0 / 127.0,
        -1.0 / 254.0,
    ];
    for round in 0..64u32 {
        // TkChPatchM339 over a recording Ch_ModelSetHide
        let hide = t.rng.next();
        record_n(&mut t, 0x0082_aea0, 3, hide);
        let chr = if round % 3 == 0 { 0 } else { t.rng.next() };
        t.twin("TkChPatchM339", 0x0082_ba00, &[chr], U32, |o| {
            w(tk::tk_ch_patch_m339(o, chr, &mut callback))
        });
        // Pad_StickToByte: listed axes, then random ones (no signalling NaN)
        let axis = if (round as usize) < listed.len() {
            listed[round as usize]
        } else {
            t.rng.float(-1.5, 1.5)
        };
        let bits = axis.to_bits();
        t.twin("Pad_StickToByte", 0x0088_9aa0, &[bits], U32, |o| {
            w(tk::pad_stick_to_byte(o, bits))
        });
        // TkMenuStartScene over a recording MenuScene_ResetGroup
        let e = if round % 4 == 0 { 0 } else { t.alloc_random(8) };
        record_n(&mut t, 0x00a7_ba70, 3, e);
        let (scene, group) = (t.rng.next(), t.rng.int());
        t.twin("TkMenuStartScene", 0x00a7_c880, &[scene, group], U32, |o| {
            w(tk::tk_menu_start_scene(o, scene, group, &mut callback))
        });
        // MenuSave_McWrite: TkGetTGS fixed, the original sceMcWrite stub
        let tgs = t.rng.pick(&[0u32, 1, 0x100]);
        return_const(&mut t, 0x0088_d6d0, tgs);
        let args = [t.rng.next(), t.rng.next(), t.rng.int()];
        t.twin("MenuSave_McWrite", 0x0088_8570, &args, U32, |o| {
            w(tk::menu_save_mc_write(o, args, &mut callback))
        });
        // HelpMem_Free over a recording yiAssert
        let assert = t.rng.next();
        record_n(&mut t, 0x0090_ae50, 3, assert);
        let used = t
            .rng
            .pick_or_any(&[0, 0x40, 0x3f, 0x1000, 0x8000_0000, 0x7fff_ffc0]);
        t.put(t.sym(tk::HELP_MEM_USED.sym), used);
        let size = t.rng.int();
        t.twin("HelpMem_Free", 0x0077_4aa0, &[size], U32, |o| {
            w(tk::help_mem_free(o, i(size), &mut callback))
        });
        // InitSysModuleEncoding: the language fixed, the loader recorded
        let lang = t.rng.pick(&[0u32, 1, 5]);
        return_const(&mut t, 0x0064_10e0, lang);
        let loaded = t.rng.next();
        record_n(&mut t, 0x0064_6920, 1, loaded);
        t.twin("InitSysModuleEncoding", 0x0064_6910, &[], U32, |o| {
            w(tk::init_sys_module_encoding(o, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round19_batch5_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0019_0005) else {
        return;
    };
    let w = u64::from;
    // TkGetControlerWork as `base + (port << 8) + slot`
    let works = t.alloc_random(0x800);
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04, 0xc1, 0xe0, 0x08][..], // mov eax, [esp + 4]; shl eax, 8
        &[0x03, 0x44, 0x24, 0x08, 0x05], &works.to_le_bytes(), // add eax, [esp + 8]; add eax, works
        &[0xc3],
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x0088_8ec0);
        o.write(at, &code).unwrap();
    }
    for round in 0..48u32 {
        // TkInitSyncV over the original setters
        for g in [
            0x0133_0220,
            0x0133_0200,
            0x0133_0210,
            0x0133_01f0,
            0x0133_01e0,
        ] {
            t.scramble(g, 4);
        }
        t.twin("TkInitSyncV", 0x0088_7e30, &[], U32, |o| {
            w(tk::tk_init_sync_v(o, &mut callback))
        });
        // MenuTuto_RestoreNames from a random copy (a zero every 16 bytes)
        let mut copy = t.rng.bytes(0x68c0);
        for k in (0..copy.len()).step_by(16) {
            copy[k + (round as usize % 16)] = 0;
        }
        let copy = t.alloc(&copy);
        t.put(t.sym(tk::TUTO_SAVE_COPY.sym), copy);
        t.twin("MenuTuto_RestoreNames", 0x008d_b7e0, &[], U32, |o| {
            w(tk::menu_tuto_restore_names(o, &mut callback))
        });
        // MenuWShop_OpenSelWindow over the original allocator
        if round % 8 == 0 {
            t.scramble(0x0186_ac24, 2);
            t.twin("MenuWShop_OpenSelWindow", 0x008d_a8f0, &[], U32, |o| {
                w(tk::menu_wshop_open_sel_window(o, &mut callback))
            });
        }
        // Pad_CheckPressMode over fixed pad stubs
        let info = t.rng.pick(&[0u32, 1, 2]);
        return_const(&mut t, 0x0062_fb70, info);
        let enter = t.rng.pick(&[0u32, 1, 0x100]);
        return_const(&mut t, 0x0062_fb10, enter);
        let status = t.alloc_random(4);
        let kind = t.rng.pick(&[0x09u8, 0x79, 0x07, 0x40]);
        t.put(status + 1, kind);
        let args = [t.rng.below(4), t.rng.below(4), t.rng.next(), status];
        t.twin("Pad_CheckPressMode", 0x0088_9510, &args, U32, |o| {
            w(tk::pad_check_press_mode(o, args, &mut callback))
        });
        // MenuMain_SaveFrameBuffer: a stand-in DC table, the blocks recorded
        let dc = t.alloc_random(0x60);
        record_n(&mut t, 0x0082_0530, 1, dc);
        let stored = t.rng.next();
        record_n(&mut t, 0x008a_acf0, 3, stored);
        t.scramble(0x0184_08f4, 4);
        t.twin("MenuMain_SaveFrameBuffer", 0x008a_ac80, &[], U32, |o| {
            w(tk::menu_main_save_frame_buffer(o, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round19_batch6_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0019_0006) else {
        return;
    };
    let w = u64::from;
    for round in 0..64u32 {
        // TkRefreshHdd: stand-in config reads, the HDD state fixed, the
        // sound driver recorded (TkRefreshHDDMode's callees)
        let menu = t.alloc_random(4);
        return_const(&mut t, 0x0078_5290, menu);
        let config = t.alloc_random(0x20);
        return_const(&mut t, 0x0088_d5e0, config);
        let word = t.rng.pick(&[0u32, 3, 1, 0x103]);
        return_const(&mut t, 0x0077_87b0, word);
        let status = t.rng.pick(&[0u32, 1, 2, 0x101]);
        return_const(&mut t, 0x0077_7e90, status);
        let name = t.rng.next();
        record_n(&mut t, 0x0077_7eb0, 1, name);
        let wave = t.rng.next();
        record_n(&mut t, 0x0081_d470, 1, wave);
        let music = t.rng.next();
        record_n(&mut t, 0x0081_d3c0, 1, music);
        t.scramble(tk::HDD_MODE.sym, 1);
        t.twin("TkRefreshHdd", 0x0087_24a0, &[], U32, |o| {
            w(tk::tk_refresh_hdd(o, &mut callback))
        });
        // BtlEnd_StepGil over a recording MsPayGIL
        let paid = t.rng.next();
        record_n(&mut t, 0x0078_59a0, 1, paid);
        let step = t.rng.pick_or_any(&[0, 1, 5, 0x8000_0000]);
        let rest = if round % 3 == 0 {
            step
        } else {
            t.rng.pick_or_any(&[0, 1, 7, 0xffff_ffff])
        };
        t.put(t.sym(tk::BTL_END_GIL.sym), [step, rest]);
        t.twin("BtlEnd_StepGil", 0x008b_a590, &[], U32, |o| {
            w(tk::btl_end_step_gil(o, &mut callback))
        });
        // TkInitWork: alarms, controllers and sync recorded, the save original
        if round % 4 == 0 {
            for f in [0x0088_c810, 0x0088_8f00, 0x0088_7e30] {
                let r = t.rng.next();
                record_n(&mut t, f, 0, r);
            }
            for g in [0x0133_0870, 0x0133_0878, 0x0133_087c, 0x023c_bec4] {
                t.scramble(g, 8);
            }
            let save = t.sym(0x0112_ca90);
            let junk = t.rng.bytes(0x40);
            t.put_bytes(save, &junk);
            t.twin("TkInitWork", 0x0088_e570, &[], U32, |o| {
                w(tk::tk_init_work(o, &mut callback))
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round19_batch7_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0019_0007) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let w = u64::from;
    for round in 0..64u32 {
        if round % 8 == 0 {
            // the weapon shop's list and second windows over the original
            // allocator; the cursor set-up recorded
            t.scramble(0x0186_ab10, 2);
            t.twin("MenuWShop_OpenListWindow", 0x008d_8260, &[], U32, |o| {
                w(tk::menu_wshop_open_list_window(o, &mut callback))
            });
            t.scramble(0x0186_a250, 4);
            let init = t.rng.next();
            record_n(&mut t, 0x008d_9580, 1, init);
            t.twin("MenuWShop_OpenSubWindow1", 0x008d_8370, &[], U32, |o| {
                w(tk::menu_wshop_open_sub_window1(o, &mut callback))
            });
            // the aeon status window: the current aeon fixed, the command
            // list and the status background recorded
            let summon = t.rng.below(8);
            return_const(&mut t, 0x008a_9880, summon);
            let built = t.rng.next();
            record_n(&mut t, 0x008d_2b40, 1, built);
            let bg = t.rng.next();
            record_n(&mut t, 0x008d_4ed0, 0, bg);
            t.scramble(0x023c_c3bc, 0x10);
            t.twin("MenuSummon_OpenStatusWindow", 0x008c_d140, &[], U32, |o| {
                w(tk::menu_summon_open_status_window(o, &mut callback))
            });
        }
        // BtlEnd_SelectedWeapon over a recording MsGetSaveWeapon
        let rec = t.rng.next();
        record_n(&mut t, 0x007a_bbd0, 1, rec);
        let list = t.alloc_random(0x200);
        t.put(t.sym(0x025c_c580), list);
        for (g, v) in [
            (0x0186_a19c, t.rng.pick(&[0u32, 1, 0x100])),
            (0x0186_a1a8, t.rng.pick(&[0u32, 1, 0x8000_0000])),
            (0x0186_a190, t.rng.below(0x80)),
            (0x0186_a1a0, t.rng.below(0x80)),
            (0x0186_a1a4, t.rng.below(0x10)),
        ] {
            t.put(t.sym(g), v);
        }
        t.scramble(0x0231_0f8e, 0x20);
        t.twin("BtlEnd_SelectedWeapon", 0x008b_da80, &[], U32, |o| {
            w(tk::btl_end_selected_weapon(o, local, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round19_batch8_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0019_0008) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let w = u64::from;
    // MsGetSavePartyMember as `*a, *b, *c = table[0..3]`
    let table = t.alloc(&[0; 12]);
    let mut code = vec![];
    for k in 0..3u8 {
        code.push(0xa1); // mov eax, [table + 4k]
        code.extend((table + 4 * u32::from(k)).to_le_bytes());
        code.extend([0x8b, 0x4c, 0x24, 4 + 4 * k]); // mov ecx, [esp + 4 + 4k]
        code.extend([0x89, 0x01]); // mov [ecx], eax
    }
    code.push(0xc3);
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x0078_52f0);
        o.write(at, &code).unwrap();
    }
    for round in 0..64u32 {
        if round % 8 == 0 {
            t.scramble(0x0186_aa3c, 2);
            t.twin("MenuKaizou_OpenWindow", 0x008d_6d60, &[], U32, |o| {
                w(tk::menu_kaizou_open_window(o, &mut callback))
            });
        }
        // BtlEnd_InParty
        let slots: Vec<u32> = (0..3).map(|_| t.rng.pick(&[0u32, 1, 2, 7, 0xff])).collect();
        for (at, v) in (table..).step_by(4).zip(&slots) {
            t.put(at, *v);
        }
        let chr = t.rng.pick(&[0u32, 1, 2, 7, 0xff, 0x101]);
        t.twin("BtlEnd_InParty", 0x008b_9ec0, &[chr], U32, |o| {
            w(tk::btl_end_in_party(o, chr, local, &mut callback))
        });
        // MenuWShop_ClampSubWindow1: random cursor, scroll, count and rows
        let restart = t.rng.next();
        record_n(&mut t, 0x008b_5530, 1, restart);
        let win = if round % 7 == 0 {
            0
        } else {
            t.alloc_random(0x60)
        };
        if win != 0 {
            for off in [0x30u32, 0x32, 0x3a, 0x48] {
                let v = t.rng.pick(&[0u16, 1, 5, 9, 10, 0x7fff, 0x8000, 0xffff]);
                let v = if t.rng.below(2) == 0 {
                    v
                } else {
                    (t.rng.next() & 0x1f) as u16
                };
                t.put(win + off, v);
            }
        }
        t.put(t.sym(tk::WSHOP_WINDOWS[1].sym), win);
        let mask = if win == 0 { VOID } else { U32 };
        t.twin("MenuWShop_ClampSubWindow1", 0x008d_8450, &[], mask, |o| {
            w(tk::menu_wshop_clamp_sub_window1(o, &mut callback).unwrap_or(0))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round20_batch1_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0020_0001) else {
        return;
    };
    let w = u64::from;
    // TOGetMessageSizePacked as `return text` (the width is its low half)
    let code = [0x8b, 0x44, 0x24, 0x04, 0xc3]; // mov eax, [esp + 4]; ret
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x008b_7170);
        o.write(at, &code).unwrap();
    }
    let local = t.b.stack_range().start + 0x100;
    record_n(&mut t, 0x008a_c850, 1, 0x00c0_1234);
    record_n(&mut t, 0x0090_5930, 6, 0x53);
    // MsGetSaveWeaponName: a constant (its kernel tables are not in the image)
    return_const(&mut t, 0x007a_bdf0, 0x00c0_5678);
    // dbgPrintf (a bare `ret` before int3 padding) keeps its format
    let fmt = t.alloc(&[0; 4]);
    let code = [
        &[0x8b, 0x44, 0x24, 0x04, 0xa3][..],
        &fmt.to_le_bytes(),
        &[0xc3],
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x0062_f500);
        o.write(at, &code).unwrap();
    }
    // floats: a quiet NaN, halves, small finite values
    let pick = |t: &mut Twins| -> u32 {
        match t.rng.below(6) {
            0 => f32::NAN.to_bits(),
            1 => (f32::from_bits(t.rng.next() & 0x7f7f_ffff) * 1e-30).to_bits(),
            _ => (f32::from(i16::try_from(t.rng.below(1000)).unwrap_or(0)) / 2.0).to_bits(),
        }
    };
    for round in 0..48u32 {
        // every window free again (32 records of 0x98 bytes)
        let windows = t.sym(tk::MENU_WINDOWS.sym);
        for k in 0..0x20u32 {
            t.put(windows + k * 0x98 + 0x40, 0u8);
        }
        // TkMenuOpenSubCmdWindowMain_008e4790: random texts (their widths)
        // and values, n from -1 to 6, widths around the row's length
        let n = t.rng.pick(&[0u32, 0xffff_ffff, 1, 2, 2, 3, 4, 6]);
        let small = round % 3 != 0;
        let mut texts: Vec<u32> = (0..6)
            .map(|_| {
                let v = t.rng.next();
                if small { v & 0x00ff_00ff } else { v }
            })
            .collect();
        // the edges: a row of exactly the width (0x1a0), of exactly 0x1f0, and a
        // negative width whose gap differs signed and unsigned (0x40, 0x80)
        let edge = match round {
            45 => Some((2, [0x100, 0x80, 0], 0x1a0)),
            46 => Some((2, [0x100, 0xd0, 0], 0x100)),
            47 => Some((3, [0xffff, 0x7ee1, 0], 0x8000)),
            _ => None,
        };
        let n = edge.map_or(n, |e| e.0);
        if let Some((_, e, _)) = edge {
            texts[..3].copy_from_slice(&e);
        }
        let values: Vec<u32> = (0..6).map(|_| t.rng.next()).collect();
        let texts = t.alloc(
            &texts
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let values = t.alloc(
            &values
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let width = t
            .rng
            .pick(&[0u32, 0x100, 0x1f0, 0x200, 0x2c0, 0x7fff, 0x8000, 0xffff]);
        let width = if t.rng.below(2) == 0 {
            width
        } else {
            t.rng.next()
        };
        let width = edge.map_or(width, |e| e.2);
        let (id, b44) = (t.rng.next(), t.rng.next());
        let args = [id, texts, values, n, width, b44];
        t.twin(
            "TkMenuOpenSubCmdWindowMain_008e4790",
            0x008e_4790,
            &args,
            U32,
            |o| w(tk::tk_menu_open_sub_cmd_window_row(o, args, &mut callback)),
        );
        // MenuNameEntry_DeleteKey: a language, keys parsed from a key string
        // (sometimes changed), the target string the same or another
        let lang = t.rng.pick(&[0u32, 1, 9, 10, 11]);
        return_const(&mut t, 0x008a_c2f0, lang);
        let n = t.rng.below(10);
        let mut keys: Vec<u8> = (0..n)
            .map(|_| t.rng.pick(&[1u8, 2, 3, 5, 0x30, 0x31, 0x80, 0xff]))
            .collect();
        keys.push(0);
        let keys = t.alloc(&keys);
        let f = t.sym(0x008c_5280);
        for o in [&mut t.a, &mut t.b] {
            o.call(f, Conv::Cdecl, &[keys]).unwrap();
        }
        if round % 3 == 1 {
            let k = t.rng.below(0x30);
            let v = u8::try_from(t.rng.below(3)).unwrap_or(0);
            t.put(t.sym(tk::ITEM_KEYS) + k, v);
        }
        let s = match round % 4 {
            0 => t.alloc(&[0]),
            1 => t.alloc(&[1, 2, 0]),
            _ => keys,
        };
        return_const(&mut t, 0x008b_edd0, s);
        let target = t.rng.below(0x14).wrapping_sub(2);
        t.put(t.sym(tk::MENU_ITEM_TARGET.sym), target);
        t.a.clear_dirty();
        t.b.clear_dirty();
        t.twin("MenuNameEntry_DeleteKey", 0x008c_44d0, &[], VOID, |o| {
            tk::menu_name_entry_delete_key(o, &mut callback);
            0
        });
        // TkFindLegendWeapon_008c30e0: on half the rounds random slots
        // (in use, flags with and without bit 2, owners 0..8)
        if round % 2 == 0 {
            let base = t.sym(ffx_battle::save::bridge::EQUIPMENT.sym);
            for k in 0..200u32 {
                let rec = [
                    u8::from(t.rng.below(4) != 0),
                    t.rng.pick(&[0u8, 2, 4, 6, 0xff]),
                    u8::try_from(t.rng.below(9)).unwrap_or(0),
                    u8::try_from(t.rng.below(2)).unwrap_or(0),
                    t.rng.pick(&[0u8, 3, 0xff]),
                ];
                t.put_bytes(base + k * 22 + 2, &rec);
            }
        }
        let chr = t.rng.pick(&[0u32, 1, 3, 6, 7, 8, 0xffff_ffff, 0x100]);
        t.twin(
            "TkFindLegendWeapon_008c30e0",
            0x008c_30e0,
            &[chr],
            U32,
            |o| w(tk::tk_find_legend_weapon(o, chr, local, &mut callback)),
        );
        // DrawCrossMenuSummonPlateNoHPMPClut: the name and the font recorded
        let (x, y) = (pick(&mut t), pick(&mut t));
        let (colour, chr) = (t.rng.next(), t.rng.next());
        t.twin(
            "DrawCrossMenuSummonPlateNoHPMPClut",
            0x008e_7110,
            &[chr, x, y, colour],
            U32,
            |o| {
                w(tk::draw_cross_menu_summon_plate_no_hpmp(
                    o,
                    chr,
                    [f32::from_bits(x), f32::from_bits(y)],
                    colour,
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

/// A stand-in `MsGetSaveWeapon(id, local)`: `base + (id & 0xff) * 8`, so
/// every id has its own slot (`local` untouched).
fn weapon_slots(t: &mut Twins, base: u32) {
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x0f, 0xb6, 0x44, 0x24, 0x04][..],        // movzx eax, byte [esp + 4]
        &[0xc1, 0xe0, 0x03],                        // shl eax, 3
        &[0x05], &base.to_le_bytes(),               // add eax, base
        &[0xc3],                                    // ret
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x007a_bbd0);
        o.write(at, &code).unwrap();
    }
}

#[test]
fn menu_round21_shop_windows_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0021_81d0) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    // five windows a round from the 32 slots
    for round in 0..5u32 {
        // the shop record, the prices, the counts and the gil recorded
        let record = t.alloc_random(0x80);
        record_n(&mut t, 0x008c_1af0, 2, record);
        for (f, n) in [
            (0x008d_9170u32, 1u8),
            (0x008d_9370, 1),
            (0x008d_9410, 1),
            (0x008d_b3b0, 1),
            (0x008d_b400, 1),
            (0x0079_04b0, 1),
            (0x0078_4e80, 0),
        ] {
            let r = t.rng.next();
            record_n(&mut t, f, n, r);
        }
        let weapon = t.rng.next();
        record_n(&mut t, 0x007a_bbd0, 1, weapon);
        for g in [0x0186_ab1c, 0x0186_aba8, 0x0186_ac20, 0x0186_ac2c] {
            t.scramble(g, 4);
        }
        t.scramble(0x0159_7740, 0x200);
        let parent = t.alloc_random(0x60);
        let k = i16::try_from(t.rng.below(0x40)).unwrap_or(0) - 0xc;
        t.put(parent + 0x48, k);
        t.twin(
            "MenuWShop_OpenSubWindow2",
            0x008d_81d0,
            &[parent],
            U32,
            |o| w(tk::menu_wshop_open_sub_window2(o, parent, &mut callback)),
        );
        t.twin(
            "MenuWShop_OpenSubWindow3",
            0x008d_82d0,
            &[parent],
            U32,
            |o| {
                w(tk::menu_wshop_open_sub_window3(
                    o,
                    parent,
                    local,
                    &mut callback,
                ))
            },
        );
        t.twin("MenuShop_OpenBuyWindow", 0x008d_a860, &[parent], U32, |o| {
            w(tk::menu_shop_open_buy_window(o, parent, &mut callback))
        });
        t.twin(
            "MenuShop_OpenSellWindow",
            0x008d_aa00,
            &[parent],
            U32,
            |o| w(tk::menu_shop_open_sell_window(o, parent, &mut callback)),
        );
        // the aeon menu's second window: the current aeon fixed, the
        // status background and the grow masks recorded
        let summon = if round == 0 {
            0xffff_ffff
        } else {
            t.rng.below(8)
        };
        return_const(&mut t, 0x008a_9880, summon);
        let bg = t.rng.next();
        record_n(&mut t, 0x008d_4ed0, 0, bg);
        let grow = t.rng.next();
        record_n(&mut t, 0x008c_d3b0, 1, grow);
        t.scramble(0x0186_a5ac, 4);
        t.twin("MenuSummon_OpenWindow2", 0x008c_eda0, &[], U32, |o| {
            w(tk::menu_summon_open_window2(o, &mut callback))
        });
    }
    // MenuItem_CountGearByOwner over stand-in gear slots
    let slots = t.alloc(&vec![0; 0x800]);
    weapon_slots(&mut t, slots);
    for round in 0..48u32 {
        let n = match round % 6 {
            0 => 0,
            1 => t.rng.pick(&[0xffff_ffff, 0x8000_0000, 0xffff_ff00]),
            2 => 1,
            _ => t.rng.below(0x60),
        };
        t.put(t.sym(0x0186_a250), n);
        let mut gear = t.rng.bytes(0x800);
        for s in gear.chunks_mut(8) {
            // owners mostly in range, kinds mostly 0 or 1
            s[4] = t.rng.pick(&[0u8, 1, 3, 6, 6, 7, 8, 0xff]);
            s[5] = t.rng.pick(&[0u8, 0, 1, 1, 2, 0x80]);
        }
        t.put_bytes(slots, &gear);
        t.scramble(0x0159_7770, 0x180);
        t.scramble(0x0186_a3e4, 0x20);
        t.twin("MenuItem_CountGearByOwner", 0x008c_94e0, &[], U32, |o| {
            w(tk::menu_item_count_gear_by_owner(o, local, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// A stand-in returning `start`, `start + 1`, ... on successive calls.
fn return_counter(t: &mut Twins, f: u32, start: u32) {
    let cell = t.alloc(&start.to_le_bytes()).to_le_bytes();
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0xa1][..], &cell,                         // mov eax, [cell]
        &[0xff, 0x05], &cell,                       // inc dword [cell]
        &[0xc3],                                    // ret
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(f);
        o.write(at, &code).unwrap();
    }
}

#[test]
fn menu_round21_sub_cmd_windows_and_params_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0021_4930) else {
        return;
    };
    // TOGetMessageSizePacked as `return text` (the width is its low half)
    let code = [0x8b, 0x44, 0x24, 0x04, 0xc3]; // mov eax, [esp + 4]; ret
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x008b_7170);
        o.write(at, &code).unwrap();
    }
    // floats: a quiet NaN, out of range, negative, fractions, small values
    let pick = |t: &mut Twins| -> f32 {
        match t.rng.below(8) {
            0 => f32::NAN,
            1 => f32::from_bits(t.rng.next() & 0x7f7f_ffff),
            2 => -f32::from_bits(t.rng.next() & 0x4fff_ffff),
            3 => 3e9,
            _ => f32::from(i16::try_from(t.rng.below(2000)).unwrap_or(0) - 500) / 4.0,
        }
    };
    let table = |t: &mut Twins| {
        let v: Vec<u8> = (0..6)
            .flat_map(|_| (t.rng.next() & 0x00ff_00ff).to_le_bytes())
            .collect();
        t.alloc(&v)
    };
    for round in 0..48u32 {
        // every window free again (32 records of 0x98 bytes)
        let windows = t.sym(tk::MENU_WINDOWS.sym);
        for k in 0..0x20u32 {
            t.put(windows + k * 0x98 + 0x40, 0u8);
        }
        let n = t.rng.pick(&[0u32, 0xffff_ffff, 1, 2, 3, 4, 6]);
        let (texts, values) = (table(&mut t), table(&mut t));
        let (id, b44) = (t.rng.next(), t.rng.next());
        let rect = [pick(&mut t), pick(&mut t), pick(&mut t), pick(&mut t)];
        let mut args = vec![id, texts, values, n, b44];
        args.extend(rect.map(f32::to_bits));
        t.twin("TkMenuOpenSubCmdWindowRect", 0x008e_4930, &args, U32, |o| {
            w(tk::tk_menu_open_sub_cmd_window_rect(
                o,
                [id, texts, values, n, b44],
                rect,
                &mut callback,
            ))
        });
        // the second window beside none or a random open one
        let open = if round % 4 == 0 {
            0
        } else {
            let open = t.alloc_random(0x200);
            let k = i16::try_from(t.rng.below(24)).unwrap_or(0) - 8;
            t.put(open + 0x48, k);
            open
        };
        t.put(t.sym(tk::MENU_SUB_CMD_WINDOW.sym), open);
        let args = [id, texts, values, n, b44];
        t.twin(
            "TkMenuOpenSubCmdWindowBeside",
            0x008e_46b0,
            &args,
            U32,
            |o| {
                w(tk::tk_menu_open_sub_cmd_window_beside(
                    o,
                    args,
                    &mut callback,
                ))
            },
        );
        // TkMsSetSaveParam: HP and MP below, at and above the maximum
        record_n(&mut t, 0x0078_60f0, 1, 0x11);
        for (get, max, set) in [
            (0x008a_98c0u32, 0x008a_9990u32, 0x008a_ae90u32),
            (0x008a_9970, 0x008a_99b0, 0x008a_aef0),
        ] {
            let m = t.rng.pick_or_any(&[0, 1, 9999, 0x8000_0000, 0x7fff_ffff]);
            let v = match t.rng.below(4) {
                0 => m,
                1 => m.wrapping_add(1),
                2 => m.wrapping_sub(1),
                _ => t.rng.next(),
            };
            return_const(&mut t, get, v);
            // the maximum counts up: the set takes the second fetch
            return_counter(&mut t, max, m);
            let r = t.rng.next();
            record_n(&mut t, set, 2, r);
        }
        let chr = t.rng.below(0x12);
        t.twin("TkMsSetSaveParam", 0x008c_3070, &[chr], U32, |o| {
            w(tk::tk_ms_set_save_param(o, chr, &mut callback))
        });
        // MenuKaizou_AddAbility: slots full, free (0 or 0xff) or none
        let mut weapon = t.rng.bytes(0x20);
        weapon[0xb] = t.rng.pick(&[0u8, 1, 2, 4, 4, 8]);
        for k in 0..8 {
            let v =
                u16::try_from(t.rng.pick(&[0, 0xff, 0x8001, 0x2034, 0x2034, 0x0100])).unwrap_or(0);
            weapon[0xe + 2 * k..0x10 + 2 * k].copy_from_slice(&v.to_le_bytes());
        }
        let weapon = t.alloc(&weapon);
        let all = t.rng.next();
        record_n(&mut t, 0x0078_6900, 0, all);
        let ability = t.rng.next();
        t.twin(
            "MenuKaizou_AddAbility",
            0x008d_5680,
            &[weapon, ability],
            U32,
            |o| {
                w(tk::menu_kaizou_add_ability(
                    o,
                    weapon,
                    ability,
                    &mut callback,
                ))
            },
        );
        // TkMsEventEffectRead, TkResetClip and MenuMain_Reset over recorders
        let r = t.rng.next();
        record_n(&mut t, 0x0078_8c60, 1, r);
        t.scramble(0x0132_6b80, 4);
        let id = t.rng.next();
        t.twin("TkMsEventEffectRead", 0x0087_2470, &[id], U32, |o| {
            w(tk::tk_ms_event_effect_read(o, id, &mut callback))
        });
        let debug = t.rng.pick(&[0u32, 1, 0x100]);
        return_const(&mut t, 0x0088_7d10, debug);
        let r = t.rng.next();
        record_n(&mut t, 0x0088_5120, 5, r);
        t.scramble(0x0133_0700, 0x10);
        t.twin("TkResetClip", 0x0088_c710, &[], U32, |o| {
            w(tk::tk_reset_clip(o, &mut callback))
        });
        if round % 8 == 7 {
            let r = t.rng.next();
            record_n(&mut t, 0x008a_a470, 0, r);
            let r = t.rng.next();
            record_n(&mut t, 0x008a_b270, 1, r);
            t.scramble(0x0184_0884, 0x78);
            t.twin("MenuMain_Reset", 0x008a_a570, &[], U32, |o| {
                w(tk::menu_main_reset(o, &mut callback))
            });
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round21_rows_names_and_hides_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0021_4e60) else {
        return;
    };
    // MenuItem_DrawRow, the name and English lookups and the unlocks recorded
    record_n(&mut t, 0x008c_9a70, 4, 0x61);
    // MsGetSaveConfigEnglish counts its calls
    return_counter(&mut t, 0x0078_51d0, 0x62);
    record_n(&mut t, 0x0082_b2c0, 2, 0x63);
    record_n(&mut t, 0x0082_b820, 2, 0x64);
    let float = |t: &mut Twins| -> f32 {
        match t.rng.below(6) {
            0 => f32::NAN,
            1 => f32::from_bits(t.rng.next() & 0x7f7f_ffff),
            _ => f32::from(i16::try_from(t.rng.below(2000)).unwrap_or(0) - 500) / 3.0,
        }
    };
    for round in 0..48u32 {
        let (window, first) = (t.rng.next(), t.rng.pick_or_any(&[0, 1, 0x7fff_fffc]));
        let (x, y) = (float(&mut t), float(&mut t));
        let args = [window, t.rng.next(), first, x.to_bits(), y.to_bits()];
        t.twin("MenuItem_DrawRows", 0x008c_99f0, &args, VOID, |o| {
            tk::menu_item_draw_rows(o, window, first, [x, y], &mut ui_remap, &mut callback);
            0
        });
        // MenuNameEntry_Commit: a character or the save's name, keys of
        // every kind, the languages that ask for the English setting
        let target = t
            .rng
            .pick(&[0u32, 3, 0xd, 0xe, 0x11, 0x12, 0xffff_ffff, 0x8000_0000]);
        t.put(t.sym(tk::MENU_ITEM_TARGET.sym), target);
        let name = t.alloc_random(0x40);
        let r = t.rng.next();
        record_n(&mut t, 0x0078_4fb0, 1, name);
        return_const(&mut t, 0x008c_5390, r);
        let lang = t.rng.pick(&[0u32, 1, 9, 10, 11]);
        return_const(&mut t, 0x008a_c2f0, lang);
        let save = t.alloc_random(0x140);
        return_const(&mut t, 0x0088_c830, save);
        let unlock = t.rng.next();
        record_n(&mut t, 0x0082_2380, 1, unlock);
        let n = if round % 5 == 0 { 0 } else { t.rng.below(9) };
        let mut keys = vec![0u8; 4 * 10];
        for k in 0..n as usize {
            keys[4 * k] = t.rng.pick(&[1u8, 4, 4, 5, 6, 0x2f, 0x30, 0x80, 0xff]);
            keys[4 * k + 1] = t.rng.pick(&[0u8, 0x2f, 0x30, 0x41, 0xff]);
            keys[4 * k + 2] = t.rng.pick(&[0u8, 0x41, 0x7f]);
            keys[4 * k + 3] = t.rng.pick(&[0u8, 9]);
        }
        t.put_bytes(t.sym(tk::ITEM_KEYS), &keys);
        t.twin("MenuNameEntry_Commit", 0x008c_4e60, &[], U32, |o| {
            w(tk::menu_name_entry_commit(o, &mut callback))
        });
        // TkHideAttachedChrs: masks, ids and pointers on and off
        let actor = t.alloc_random(0xb20);
        let mask = t.rng.pick(&[0u16, 1, 2, 3, 0xfffc, 0xffff]);
        t.put(actor + 0xb14, mask);
        for k in 0..2u32 {
            let id = u16::try_from(t.rng.pick(&[0, 0xff, 0x1ff, 3])).unwrap_or(0);
            t.put(actor + 0xb04 + 2 * k, id);
            let chr = t.rng.pick(&[0u32, 0x00c0_0000 + k, 0x1234_5678]);
            t.put(actor + 0xaf4 + 4 * k, chr);
        }
        let hide = t.rng.pick_or_any(&[0, 1]);
        t.twin(
            "TkHideAttachedChrs",
            0x0087_2380,
            &[actor, hide],
            U32,
            |o| w(tk::tk_hide_attached_chrs(o, actor, hide, &mut callback)),
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round22_save_pack_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0022_3e60) else {
        return;
    };
    for round in 0..150u32 {
        let mut save = t.rng.bytes(tk::SAVE_DATA_SIZE as usize);
        let story = t.rng.pick(&[0u16, 15, 16, 299, 300, 0xffff, 0x8000]);
        let story = if round % 4 == 3 {
            u16::try_from(t.rng.below(0x1_0000)).unwrap_or(0)
        } else {
            story
        };
        save[0xbec..0xbee].copy_from_slice(&story.to_le_bytes());
        for b in &mut save[0x3d58..0x3d5b] {
            *b = t.rng.pick(&[6u8, 6, 7, 0, 0xff]);
        }
        let save = t.alloc(&save);
        let out = t.alloc_random(0x40 + tk::SAVE_DATA_SIZE as usize);
        let party = t.rng.pick(&[0xffff_ffffu32, 0, 1, 2, 3, 6, 7, 8, 0x20]);
        t.put(t.sym(tk::MENU_PLAYER_LIST_MAX2.sym), party);
        t.scramble(tk::MENU_PLAYERS.sym, 8);
        let counter = t
            .rng
            .pick_or_any(&[0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0]);
        t.put(t.sym(tk::MENU_SAVE_COUNTER.sym), counter);
        // the getters' results, and a name of 0..15 letters
        for f in [0x0064_10e0, 0x0078_4e80, 0x0086_c410, 0x0086_a7e0] {
            let v = t.rng.next();
            return_const(&mut t, f, v);
        }
        let mut name = t.rng.bytes(16);
        for c in &mut name {
            *c |= 1;
        }
        name[t.rng.below(16) as usize] = 0;
        name.push(0);
        let name = t.alloc(&name);
        return_const(&mut t, 0x0078_4fb0, name);
        let slot = t.rng.next();
        t.twin(
            "MenuSave_PackSave",
            0x008b_3e60,
            &[out, save, slot],
            U32,
            |o| {
                #[allow(clippy::cast_possible_truncation)]
                w(tk::menu_save_pack_save(
                    o,
                    out,
                    save,
                    slot as u8,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round22_buy_sell_windows_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0022_2d00) else {
        return;
    };
    // floats: a quiet NaN, large, negative, fractions, small values
    let pick = |t: &mut Twins| -> f32 {
        match t.rng.below(8) {
            0 => f32::NAN,
            1 => f32::from_bits(t.rng.next() & 0x7f7f_ffff),
            2 => -f32::from_bits(t.rng.next() & 0x4fff_ffff),
            _ => f32::from(i16::try_from(t.rng.below(2000)).unwrap_or(0) - 500) / 4.0,
        }
    };
    let edge = [
        0u32,
        1,
        0xffff_ffff,
        0x8000_0000,
        0x7fff_ffff,
        999_999_999,
        1_000_000_000,
    ];
    // the drawing calls recorded, each with its own result
    for (f, n) in [
        (0x008f_5f70u32, 5u8),
        (0x008f_4b20, 6),
        (0x008e_6970, 4),
        (0x008f_9c20, 5),
        (0x008f_8bb0, 5),
        (0x008c_0a20, 5),
        (0x008e_6700, 4),
        (0x0090_3bb0, 9),
        (0x0090_5550, 6),
    ] {
        let r = t.rng.next();
        record_n(&mut t, f, n, r);
    }
    for _ in 0..60 {
        let (x, y) = (pick(&mut t), pick(&mut t));
        let weapon = t.rng.next();
        let amount = t.rng.pick_or_any(&edge);
        let owned = if t.rng.below(2) == 0 {
            amount.wrapping_add(t.rng.pick_or_any(&edge))
        } else {
            t.rng.pick_or_any(&edge)
        };
        t.twin(
            "TkMn2WeaponBuySellWindow",
            0x008c_2d00,
            &[x.to_bits(), y.to_bits(), weapon, amount, owned],
            U32,
            |o| {
                w(tk::tk_mn2_weapon_buy_sell_window(
                    o,
                    [x, y],
                    weapon,
                    i(amount),
                    i(owned),
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
    }
    // TkMn2BuySellWindow: the digit widths written through the out word
    let width = t.b.stack_range().start + 0x100;
    let cell = digit_width(&mut t);
    for _ in 0..60 {
        let digit = if t.rng.below(4) == 0 {
            f32::NAN
        } else {
            f32::from(u8::try_from(t.rng.below(200)).unwrap_or(0)) / 3.0
        };
        t.put(cell, digit);
        let (x, y) = (pick(&mut t), pick(&mut t));
        let v = [
            t.rng.next(),
            t.rng.pick_or_any(&edge),
            t.rng.pick_or_any(&edge),
            t.rng.pick_or_any(&[0, 1, 2, 99, 0xffff_ffff, 0x8000]),
            t.rng.pick_or_any(&edge),
        ];
        t.twin(
            "TkMn2BuySellWindow",
            0x008b_f170,
            &[&[x.to_bits(), y.to_bits()], &v[..]].concat(),
            U32,
            |o| {
                w(tk::tk_mn2_buy_sell_window(
                    o,
                    [x, y],
                    v,
                    width,
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

/// A stand-in `ToGetBtlEasyDigitWidth2(n, out, scale, b)`: `*out` the
/// float in the returned cell, the other arguments recorded (the out
/// pointer differs between the original's argument slot and the port's
/// scratch word).
fn digit_width(t: &mut Twins) -> u32 {
    width_out(t, 0x0090_5230).0
}

/// A stand-in width function `f(text, out, a, scale, b)` at `f`: `*out`
/// the float bits in the first returned cell, which then grow by the
/// second cell's word (so successive calls differ); arguments 0, 2 and 3
/// recorded (the out pointer differs between the original's local and the
/// port's scratch word).
fn width_out(t: &mut Twins, f: u32) -> (u32, u32) {
    let cell = t.alloc(&[0; 4]);
    let step = t.alloc(&[0; 4]);
    let buf = t.alloc(&vec![0; 0x1_0000]);
    let log = t.alloc(&buf.to_le_bytes());
    let l = log.to_le_bytes();
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x0d][..], &l,                      // mov ecx, [log]
        &[0x8b, 0x44, 0x24, 0x04, 0x89, 0x01],      // [ecx] = arg 0
        &[0x8b, 0x44, 0x24, 0x0c, 0x89, 0x41, 0x04], // [ecx + 4] = arg 2
        &[0x8b, 0x44, 0x24, 0x10, 0x89, 0x41, 0x08], // [ecx + 8] = arg 3
        &[0x83, 0x05], &l, &[0x0c],                 // add dword [log], 12
        &[0xa1], &cell.to_le_bytes(),               // mov eax, [cell]
        &[0x8b, 0x54, 0x24, 0x08, 0x89, 0x02],      // *arg 1 = eax
        &[0x8b, 0x0d], &step.to_le_bytes(),         // mov ecx, [step]
        &[0x01, 0x0d], &cell.to_le_bytes(),         // add [cell], ecx
        &[0xc3],
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(f);
        o.write(at, &code).unwrap();
    }
    (cell, step)
}

#[test]
fn menu_round22_legend_choice_and_segment_match_the_original() {
    use ffx_menu::{msg::bridge as msg, tk::bridge as tk};
    let Some(mut t) = setup_any(0x0022_3170) else {
        return;
    };
    // TkSetLegendAbility: the weapon finder and the save refresh recorded
    // (no weapon in the last rounds)
    let weapon = t.alloc_random(0x20);
    record_n(&mut t, 0x008c_30e0, 1, weapon);
    // MsSetSaveParamAll is a 5-byte jump before it: a call counter fits
    return_counter(&mut t, 0x008c_30d0, 0);
    for round in 0..80u32 {
        if round == 60 {
            return_const(&mut t, 0x008c_30e0, 0);
        }
        let chr = t
            .rng
            .pick_or_any(&[0, 1, 2, 3, 12, 13, 14, 17, 18, 0xffff_ffff]);
        let level = t.rng.pick(&[0, 1, 2, 3, 7, 0xffff_ffff, 0xffff_fffe]);
        t.twin("TkSetLegendAbility", 0x008c_3170, &[chr, level], U32, |o| {
            w(tk::tk_set_legend_ability(o, chr, level, &mut callback))
        });
    }
    // AtelOpenChoiceWindow: the script stack as a counter, the natives and
    // message lookups recorded
    let window = t.alloc_random(0x40);
    let start = t.rng.next();
    return_counter(&mut t, 0x0086_df00, start);
    for (f, n) in [
        (0x0086_e3f0u32, 3u8),
        (0x0085_80c0, 3),
        (0x0085_8b50, 3),
        (0x0085_9710, 3),
        (0x0086_bf30, 1),
        (0x0086_bbc0, 0),
        (0x0086_bf10, 2),
        (0x0086_be90, 1),
    ] {
        let r = if f == 0x0086_be90 {
            window
        } else {
            t.rng.next()
        };
        record_n(&mut t, f, n, r);
    }
    for round in 0..20u32 {
        t.scramble(0x0132_5ac0, 4);
        let (worker, args, stack) = (t.rng.next(), t.rng.next(), t.rng.next());
        let mode = if round % 2 == 0 {
            0
        } else {
            t.rng.pick_or_any(&[8])
        };
        t.twin(
            "AtelOpenChoiceWindow",
            0x0086_0b00,
            &[worker, args, stack, mode],
            U32,
            |o| {
                w(msg::atel_open_choice_window(
                    o,
                    [worker, args, stack],
                    mode,
                    &mut callback,
                ))
            },
        );
    }
    // Math_SegmentNearestT: points on and off the segment, degenerate
    // segments, NaNs and infinities
    let coord = |t: &mut Twins| -> f32 {
        match t.rng.below(10) {
            0 => f32::NAN,
            1 => f32::INFINITY,
            2 => 0.0,
            3 => f32::from_bits(t.rng.next() & 0x7f7f_ffff),
            _ => f32::from(i16::try_from(t.rng.below(400)).unwrap_or(0) - 200) / 8.0,
        }
    };
    for round in 0..400u32 {
        let mut pts = [[0f32; 4]; 3];
        for p in &mut pts {
            for c in p.iter_mut() {
                *c = coord(&mut t);
            }
        }
        if round % 4 == 0 {
            pts[2] = pts[1]; // a zero-length segment
        }
        if round % 7 == 0 {
            pts[0] = pts[1];
        }
        let addr: Vec<u32> = pts
            .iter()
            .map(|p| {
                let b: Vec<u8> = p.iter().flat_map(|c| c.to_le_bytes()).collect();
                t.alloc(&b)
            })
            .collect();
        if round % 9 == 0 {
            t.scramble(0x00c0_a004, 16);
        }
        t.scramble(0x00c8_f788, 4);
        t.twin_f("Math_SegmentNearestT", 0x0088_a040, &addr, |o| {
            ffx_menu::math::bridge::math_segment_nearest_t(o, addr[0], addr[1], addr[2])
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round22_angles_captions_and_pads_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0022_b610) else {
        return;
    };
    // Math_AngleStepMode: the three modes over angles near and far
    let angle = |t: &mut Twins| -> f32 {
        match t.rng.below(10) {
            0 => f32::NAN,
            1 => 0.0,
            2 => -0.0,
            3 => std::f32::consts::PI,
            _ => f32::from(i16::try_from(t.rng.below(2000)).unwrap_or(0) - 1000) / 150.0,
        }
    };
    for _ in 0..600 {
        let (a, b, step) = (angle(&mut t), angle(&mut t), angle(&mut t));
        let mode = t.rng.pick_or_any(&[0, 1, 2, 3, 0xffff_ffff]);
        t.twin_f(
            "Math_AngleStepMode",
            0x0088_b610,
            &[a.to_bits(), b.to_bits(), step.to_bits(), mode],
            |_| tk::math_angle_step_mode(a, b, step, i(mode)),
        );
    }
    // toGetStatusWeaponMesPos: the two captions' widths from a stand-in
    let r = t.rng.next();
    record_n(&mut t, 0x008b_ee70, 1, r);
    let (cell, step) = width_out(&mut t, 0x0090_5290);
    let widths = t.b.stack_range().start + 0x100;
    for round in 0..120u32 {
        let w = match round % 6 {
            0 => f32::NAN,
            1 => 3e9,
            _ => f32::from(u16::try_from(t.rng.below(400)).unwrap_or(0)) / 3.0,
        };
        t.put(cell, w);
        let grow = t
            .rng
            .pick(&[0u32, 1, 0x0010_0000, 0xfff0_0000, 0x0100_0000]);
        t.put(step, grow);
        if round % 6 == 5 {
            // the first width 2.0, the second a NaN (0x40000000 + 0x3f800001)
            t.put(cell, 0x4000_0000u32);
            t.put(step, 0x3f80_0001u32);
        }
        t.twin("toGetStatusWeaponMesPos", 0x008d_54a0, &[], U32, |o| {
            w64(tk::to_get_status_weapon_mes_pos(
                o,
                [widths, widths + 4],
                &mut ui_remap,
                &mut callback,
            ))
        });
    }
    // the pad helpers: the PC buttons, the work resets
    let buttons = t.alloc(&[0; 4]);
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0xa1][..], &buttons.to_le_bytes(),        // mov eax, [buttons]
        &[0xc3],
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x0063_0c80);
        o.write(at, &code).unwrap();
    }
    let r = t.rng.next();
    record_n(&mut t, 0x0088_9030, 1, r);
    let work = t.alloc_random(0x100);
    record_n(&mut t, 0x0088_8ec0, 2, work);
    for _ in 0..200 {
        let b = t
            .rng
            .pick_or_any(&[0xa000, 0x5000, 0xf000, 0xffff, 0x1_a000, 0x8000]);
        t.put(buttons, b);
        let pad = t.alloc_random(0x30);
        let mode = t
            .rng
            .pick(&[0x20u8, 0x22, 0x41, 0x60, 0x61, 0x62, 0x63, 0x6f, 0x73, 0x10]);
        t.put(pad + 1, mode);
        let w = t.alloc_random(0x100);
        let (a, b) = (t.rng.next(), t.rng.next());
        t.twin(
            "Pad_ReadCurrDevice",
            0x0088_98a0,
            &[a, b, w, pad],
            U32,
            |o| w64(tk::pad_read_curr_device(o, w, pad, &mut callback)),
        );
        t.twin("Pad_ResetWork", 0x0088_8970, &[w], U32, |o| {
            w64(tk::pad_reset_work(o, w, &mut callback))
        });
        t.twin("Pad_ClearAnalogWork", 0x0088_89f0, &[a, b], U32, |o| {
            w64(tk::pad_clear_analog_work(o, a, b, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round22_text_wrappers_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0022_df80) else {
        return;
    };
    // the 45 menu text wrappers over the game's tables, kana on and off
    let config = ffx_battle::save::bridge::MENU.sym;
    for round in 0..40u32 {
        for (f, text, help) in tk::MENU_TEXT_WRAPPERS {
            let id = t
                .rng
                .pick_or_any(&[0, 1, 2, 0x10, 0x2000_0001, 0xffff_ffff])
                & 0xffff_1fff;
            t.twin("menu text wrapper", f, &[id], U32, |o| {
                w(tk::menu_text_of(o, text, help, id))
            });
        }
        if round % 2 == 0 {
            t.scramble(config, 4);
        }
    }
    // the tail-jump wrappers, their targets recorded
    let r = t.rng.next();
    record_n(&mut t, 0x008b_8040, 1, r);
    let (smn, ply) = (t.rng.next(), t.rng.next());
    return_const(&mut t, 0x0090_1130, smn);
    return_const(&mut t, 0x0090_1100, ply);
    return_counter(&mut t, 0x008c_5440, 0);
    let r = t.rng.next();
    record_n(&mut t, 0x008c_5320, 1, r);
    let r = t.rng.next();
    record_n(&mut t, 0x008c_4e60, 0, r);
    for _ in 0..20 {
        let text = t.rng.next();
        t.twin("TkMenuDraw1612Width", 0x008d_c9e0, &[text], U32, |o| {
            w(tk::tk_menu_draw1612_width(o, text, &mut callback))
        });
        let summon = t.rng.pick_or_any(&[0, 1]);
        t.twin("TkMn2TransCharFace", 0x008c_2ce0, &[summon], U32, |o| {
            w(tk::tk_mn2_trans_char_face(o, summon, &mut callback))
        });
        t.twin(
            "MenuNameEntry_CommitDefault0b",
            0x008c_5370,
            &[],
            U32,
            |o| w(tk::menu_name_entry_commit_default0b(o, &mut callback)),
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round22_mes_rikku_and_item_help_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0022_1060) else {
        return;
    };
    // TkMn2DrawMesL: the font call recorded, the cursor and shades scrambled
    let r = t.rng.next();
    record_n(&mut t, 0x008e_d160, 9, r);
    for _ in 0..20 {
        t.scramble(tk::MN2_DRAW_Y.sym, 4);
        t.scramble(tk::MN2_FONT_SHADE.sym, 0x20);
        let v = [t.rng.next(), t.rng.next(), t.rng.next(), t.rng.next()];
        t.twin("TkMn2DrawMesL", 0x008c_1060, &v, U32, |o| {
            w(tk::tk_mn2_draw_mes_l(o, v, &mut callback))
        });
    }
    // debugSetRikkuName: the save name a scratch buffer; the text needs the
    // game's menu tables (snapshots only: the image has none, text 0)
    let name = t.alloc_random(0x40);
    return_const(&mut t, 0x0078_4fb0, name);
    let rounds = if std::env::var_os("FFX_SNAPSHOT").is_some() {
        10
    } else {
        0
    };
    for _ in 0..rounds {
        t.scramble(ffx_battle::save::bridge::MENU.sym, 4);
        t.twin("debugSetRikkuName", 0x0086_0a60, &[], U32, |o| {
            w(tk::debug_set_rikku_name(o, &mut callback))
        });
    }
    // MenuItem_RefreshHelp: every help source recorded with its own result
    for (f, n) in [
        (0x008a_98f0u32, 1u8),
        (0x008d_dea0, 1),
        (0x008d_de10, 1),
        (0x0078_f7c0, 2),
        (0x008a_aeb0, 1),
    ] {
        let r = t.rng.next();
        record_n(&mut t, f, n, r);
    }
    let rows = t.alloc(&[0; 4]);
    let max = t.alloc(&[0; 4]);
    let owned = t.alloc(&[0; 4]);
    for (f, cell) in [
        (0x008c_8a00u32, rows),
        (0x008c_8a20, max),
        (0x008a_b2c0, owned),
    ] {
        let code = [&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat(); // mov eax, [cell]; ret
        for o in [&mut t.a, &mut t.b] {
            let at = o.rebase(f);
            o.write(at, &code).unwrap();
        }
    }
    for round in 0..1200u32 {
        let window = t.alloc_random(0xa0);
        let sel = t.rng.pick(&[0u8, 1, 2, 3, 0xff, 0x80]);
        t.put(window + 0x45, sel);
        let lists = t.rng.pick(&[0u8, 1, 2, 4]);
        t.put(window + 0x42, lists);
        let kind = t.rng.pick(&[0u8, 1, 2, 2, 3]);
        t.put(window + 0x43, kind);
        for k in 0..6 {
            let base = t.rng.below(0x40);
            t.put(window + 0x54 + 4 * k, base);
        }
        // a quarter of the rounds on a negative row
        let cursor = if round % 4 == 1 {
            -24
        } else {
            i16::try_from(t.rng.below(0x40)).unwrap_or(0) - 8
        };
        t.put(window + 0x48, cursor);
        let on = if round % 8 == 0 {
            0
        } else {
            t.rng.pick(&[1u32, 0x100])
        };
        t.put(window + 0x94, on);
        // the row as the original computes it, the counts around it
        let k = i32::from(i8::from_le_bytes([sel])).max(0);
        let k = if k >= i32::from(lists) {
            i32::from(lists) - 1
        } else {
            k
        };
        let base: u32 = Mem::read(
            &t.b,
            (window + 0x58).wrapping_add(k.cast_unsigned().wrapping_mul(4)),
        );
        let row = base.wrapping_add(i32::from(cursor).cast_unsigned().wrapping_mul(2));
        for cell in [rows, max] {
            let v = row.wrapping_add(t.rng.pick(&[0xffff_ffffu32, 0, 1, 0x40]));
            t.put(cell, v);
        }
        let v = t.rng.pick(&[0u32, 1]);
        t.put(owned, v);
        let mut list = t.rng.bytes(0x400);
        for e in list.chunks_mut(4) {
            e[2] = t.rng.pick(&[0u8, 1, 2, 3]);
        }
        // from 16 entries before the list: negative rows read there
        t.put_bytes(t.sym(tk::MENU_ITEM_LIST.sym) - 0x40, &list);
        let mut keys = t.rng.bytes(0x200);
        for e in keys.chunks_mut(2) {
            if t.rng.below(3) == 0 {
                e.copy_from_slice(&[0, 0]);
            }
        }
        t.put_bytes(t.sym(tk::MENU_IMPORTANT_LIST.sym) - 0x80, &keys);
        let mask = if sel >= 0x80 { VOID } else { U32 };
        t.twin("MenuItem_RefreshHelp", 0x008c_7650, &[window], mask, |o| {
            w(tk::menu_item_refresh_help(o, window, &mut callback).unwrap_or(0))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round22_mc_sync_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0022_84a0) else {
        return;
    };
    // the sync a counter: 0 for the first `left` polls, then its count;
    // the modes summed
    let left = t.alloc(&[0; 4]);
    let log = t.alloc(&[0; 4]);
    let modes = t.alloc(&[0; 4]);
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04][..],              // mov eax, [esp + 4]
        &[0x01, 0x05], &modes.to_le_bytes(),        // add [modes], eax
        &[0xff, 0x05], &log.to_le_bytes(),          // inc dword [log]
        &[0x31, 0xc0],                              // xor eax, eax
        &[0x83, 0x2d], &left.to_le_bytes(), &[1],   // sub dword [left], 1
        &[0x79, 0x05],                              // jns +5
        &[0xa1], &log.to_le_bytes(),                // mov eax, [log]
        &[0xc3],                                    // ret
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x0088_84e0);
        o.write(at, &code).unwrap();
    }
    for round in 0..30u32 {
        t.put(left, round % 5);
        t.scramble(0x0133_0278, 4);
        let mode = if round % 3 == 0 {
            t.rng.pick_or_any(&[1, 2])
        } else {
            0
        };
        let (cmd, result) = (t.rng.next(), t.rng.next());
        t.twin(
            "MenuSave_McSync",
            0x0088_84a0,
            &[mode, cmd, result],
            U32,
            |o| w(tk::menu_save_mc_sync(o, [mode, cmd, result], &mut callback)),
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round22_sell_window_ctrl_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0022_9e20) else {
        return;
    };
    // the pads from cells, the sound and the sale recorded
    let (rep, trg) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    for (f, cell) in [(0x008b_e470u32, rep), (0x008b_e4b0, trg)] {
        let code = [&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat(); // mov eax, [cell]; ret
        for o in [&mut t.a, &mut t.b] {
            let at = o.rebase(f);
            o.write(at, &code).unwrap();
        }
    }
    return_counter(&mut t, 0x008b_e410, 0);
    for (f, n) in [(0x0088_6e80u32, 1u8), (0x0079_0550, 2), (0x0078_59a0, 1)] {
        let r = t.rng.next();
        record_n(&mut t, f, n, r);
    }
    // MsGetGIL: 0, 1, 2, ...
    return_counter(&mut t, 0x0078_4e80, 0);
    for round in 0..3000u32 {
        let window = t.alloc_random(0x80);
        let state = t.rng.pick_or_any(&[0, 1, 2, 3, 3, 3, 4, 5, 6, 7, 8]);
        t.put(window + 0x28, state);
        let n = t
            .rng
            .pick(&[0i16, 1, 2, 5, 10, 11, 98, 99, 100, -3, 0x7ffc]);
        t.put(window + 0x48, n);
        // half the rounds at or next to the amount
        let near = i32::from(n)
            .cast_unsigned()
            .wrapping_add(t.rng.pick(&[0u32, 1, 9, 10, 11]));
        let most = if round % 2 == 0 {
            near
        } else {
            t.rng
                .pick_or_any(&[1, 2, 10, 99, 100, 0x1_0063, 0xffff_ffff])
        };
        t.put(window + 0x64, most);
        let r = t
            .rng
            .pick_or_any(&[0, 0x4000, 0x1000, 0x8000, 0x2000, 0x5000, 0xa000]);
        t.put(rep, r);
        let g = t.rng.pick_or_any(&[0, 0, 0x20, 0x40, 0x60, 0x2000_0000]);
        t.put(trg, g);
        t.twin(
            "MenuShop_SellWindowCtrl",
            0x008d_9e20,
            &[window],
            VOID,
            |o| {
                tk::menu_shop_sell_window_ctrl(o, window, &mut callback);
                0
            },
        );
        // the buy window: the gil a counter, the price never 0
        let price = t.rng.pick_or_any(&[1, 2, 10, 50, 0x8000_0000]).max(1);
        t.put(window + 0x60, price);
        let owned = t.rng.pick_or_any(&[0, 1, 50, 89, 98, 99, 120]);
        t.put(window + 0x64, owned);
        t.put(window + 0x28, state);
        t.put(window + 0x48, n);
        t.twin(
            "MenuShop_BuyWindowCtrl",
            0x008d_9bd0,
            &[window],
            VOID,
            |o| {
                tk::menu_shop_buy_window_ctrl(o, window, &mut callback);
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round23_wshop_confirm_ctrls_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0023_7690) else {
        return;
    };
    // the yes/no window TkMn2GetListMax2 returns
    let list = t.alloc_random(0x80);
    return_const(&mut t, 0x008e_3390, list);
    // TkMenuMainMes: the ID + 1 (OpenSubCmdWindow logs them)
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x008d_df80);
        o.write(at, &[0x8b, 0x44, 0x24, 0x04, 0x40, 0xc3]).unwrap(); // mov eax, [esp + 4]; inc eax; ret
    }
    // the calls without arguments counted
    for f in [
        0x008b_e410u32,
        0x008b_e470,
        0x008b_e4b0,
        0x0078_4e80,
        0x008e_2de0,
    ] {
        return_counter(&mut t, f, 0);
    }
    for (f, n) in [
        (0x008e_33a0u32, 3u8),
        (0x0088_6e80, 1),
        (0x0078_59a0, 1),
        (0x008c_2c60, 3),
        (0x007a_b910, 1),
    ] {
        let r = t.rng.next();
        record_n(&mut t, f, n, r);
    }
    // MsGetSaveWeapon(id, &name): the name `names[id & 3]`, nothing for 0xff
    let names: Vec<u32> = [&b"Longsword\0"[..], b"\0", b"Kaiser Knuckles\0", b"x\0"]
        .iter()
        .map(|s| t.alloc(s))
        .collect();
    let table = t.alloc(
        &names
            .iter()
            .flat_map(|n| n.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04][..],              // mov eax, [esp + 4]
        &[0x3d, 0xff, 0, 0, 0, 0x74, 0x10],         // cmp eax, 0xff; je ret
        &[0x83, 0xe0, 0x03],                        // and eax, 3
        &[0x8b, 0x04, 0x85], &table.to_le_bytes(),  // mov eax, [table + eax * 4]
        &[0x8b, 0x4c, 0x24, 0x08, 0x89, 0x01],      // mov ecx, [esp + 8]; mov [ecx], eax
        &[0xc3],                                    // ret
    ]
    .concat();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(0x007a_bbd0);
        o.write(at, &code).unwrap();
    }
    let local = t.b.stack_range().start + 0x100;
    for _ in 0..1500u32 {
        let window = t.alloc_random(0x80);
        let state = t.rng.pick_or_any(&[0, 1, 2, 2, 3, 3, 3, 4, 5, 6, 7, 8]);
        t.put(window + 0x28, state);
        let id = t.rng.pick_or_any(&[0, 1, 2, 3, 0xff, 0x5003]);
        t.put(window + 0x6c, id);
        let answer = t.rng.pick(&[0u8, 0, 1, 1, 0x7f, 0x80, 0xff]);
        t.put(list + 0x45, answer);
        let row = t.rng.pick(&[0i16, 0, 1, -1, 0x100]);
        t.put(list + 0x48, row);
        t.twin(
            "MenuWShop_BuyConfirmCtrl",
            0x008d_7690,
            &[window],
            VOID,
            |o| {
                tk::menu_wshop_buy_confirm_ctrl(o, window, &mut ui_remap, &mut callback);
                0
            },
        );
        t.put(window + 0x28, state);
        t.twin(
            "MenuWShop_SellConfirmCtrl",
            0x008d_7860,
            &[window],
            VOID,
            |o| {
                tk::menu_wshop_sell_confirm_ctrl(o, window, local, &mut ui_remap, &mut callback);
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

/// `f` replaced in both oracles by `code`.
fn stub(t: &mut Twins, f: u32, code: &[u8]) {
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(f);
        o.write(at, code).unwrap();
    }
}

/// `f` replaced by `mov eax, [cell]; ret`; returns the cell.
fn return_cell(t: &mut Twins, f: u32) -> u32 {
    let cell = t.alloc(&[0; 4]);
    let code = [&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat();
    stub(t, f, &code);
    cell
}

/// `f(&n)` replaced by `n = [cell]` (left alone for -2); `return list`;
/// returns the cell.
fn out_count(t: &mut Twins, f: u32, list: u32) -> u32 {
    let cell = t.alloc(&[0; 4]);
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04][..],              // mov eax, [esp + 4]
        &[0x8b, 0x0d], &cell.to_le_bytes(),         // mov ecx, [cell]
        &[0x83, 0xf9, 0xfe, 0x74, 0x02],            // cmp ecx, -2; je +2 (no count)
        &[0x89, 0x08],                              // mov [eax], ecx
        &[0xb8], &list.to_le_bytes(), &[0xc3],      // mov eax, list; ret
    ]
    .concat();
    stub(t, f, &code);
    cell
}

#[test]
fn menu_round23_help_and_party_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0023_1a20) else {
        return;
    };
    // TkMn2DrawSelItemsHelp: every help getter logs and returns its own value
    for (f, n) in [
        (0x008c_1b30u32, 1u8),
        (0x008c_1960, 1),
        (0x008c_19e0, 1),
        (0x0078_f7c0, 2),
        (0x008d_dfa0, 1),
        (0x008a_aeb0, 1),
    ] {
        let r = t.rng.next();
        record_n(&mut t, f, n, r);
    }
    let list = t.sym(0x0159_7770);
    for _ in 0..3000u32 {
        let window = t.alloc_random(0x80);
        let rows = t.rng.pick(&[0u16, 1, 5, 0x200, 0xffff]);
        t.put(window + 0x30, rows);
        let row = t.rng.pick(&[0i16, 1, 4, 5, -1, 0x1ff]);
        t.put(window + 0x48, row);
        let entry = list.wrapping_add((i32::from(row) * 4).cast_unsigned());
        let id = t.rng.pick(&[0u16, 1, 0xf, 0x2001, 0xffff]);
        t.put(entry, id);
        let kind = t
            .rng
            .pick(&[0u8, 1, 2, 3, 4, 7, 8, 10, 11, 16, 17, 18, 19, 0xff]);
        t.put(entry + 2, kind);
        t.twin("TkMn2DrawSelItemsHelp", 0x008d_1a20, &[window], VOID, |o| {
            tk::tk_mn2_draw_sel_items_help(o, window, &mut callback);
            0
        });
    }
    // HelpMem_Get: the size and base from cells, the assertion logged
    let limit = return_cell(&mut t, 0x008b_ec80);
    let base = return_cell(&mut t, 0x008b_ec70);
    let r = t.rng.next();
    record_n(&mut t, 0x0090_ae50, 5, r);
    for _ in 0..3000u32 {
        let l = t.rng.pick_or_any(&[0x20_0000, 0x40, 0, 0x8000_0000]);
        t.put(limit, l);
        let b = t.rng.int();
        t.put(base, b);
        let used = t.rng.pick_or_any(&[0, 0x40, 0x1f_ffc0, 0x7fff_ffc0]);
        t.put(t.sym(0x0112_a09c), used);
        let reset = t.rng.pick(&[0u32, 1, 0x100]);
        let size = t
            .rng
            .pick_or_any(&[0, 1, 0x40, 0x41, 0xffff_ffc1, 0xffff_ffff]);
        t.twin("HelpMem_Get", 0x0077_4c00, &[reset, size], U32, |o| {
            w(tk::help_mem_get(o, reset, i(size), &mut callback))
        });
    }
    // TkMenuExchangePlayerPos: the party lists and counts from cells
    let party = t.alloc_random(0x10);
    let out_n = out_count(&mut t, 0x0078_52d0, party);
    let in_n = out_count(&mut t, 0x0078_5270, party + 8);
    let r = t.rng.next();
    record_n(&mut t, 0x0078_4d00, 2, r);
    return_counter(&mut t, 0x008a_ae30, 0);
    let local = t.b.stack_range().start + 0x100;
    let players = t.sym(0x0184_1c14);
    for _ in 0..3000u32 {
        t.scramble(0x0184_1c14, 0x10);
        let ids: Vec<u8> = (0..0x10).map(|_| t.rng.pick(&[0u8, 1, 2, 3])).collect();
        t.put_bytes(party, &ids);
        t.put_bytes(players, &ids);
        let mask = t.rng.pick_or_any(&[0, 3, 4, 0xffff_ffff]);
        t.put(t.sym(0x0184_1c24), mask);
        let n = t.rng.pick(&[0u32, 1, 3, 8, 0xffff_ffff, 0xffff_fffe]);
        t.put(out_n, n);
        t.put(in_n, n);
        let a = t.rng.pick(&[0u32, 1, 2, 7]);
        let b = t.rng.pick(&[0u32, 1, 3, 4, 7, 0xffff_ffff]);
        t.twin("TkMenuExchangePlayerPos", 0x008a_9720, &[a, b], VOID, |o| {
            tk::tk_menu_exchange_player_pos(o, i(a), i(b), local, &mut callback);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round23_target_window_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0023_9240) else {
        return;
    };
    let window = t.alloc_random(0x80);
    return_const(&mut t, 0x008a_a1a0, window);
    let players = return_cell(&mut t, 0x008a_9b30);
    let aeons = return_cell(&mut t, 0x008a_9bf0);
    // the characters: index + 0x10 (party) or + 0x18 (aeons); the name the
    // character; the width a table entry
    stub(
        &mut t,
        0x008a_9ad0,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xc0, 0x10, 0xc3],
    );
    stub(
        &mut t,
        0x008a_9bc0,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xc0, 0x18, 0xc3],
    );
    stub(&mut t, 0x008a_c850, &[0x8b, 0x44, 0x24, 0x04, 0xc3]);
    let widths = t.alloc(&[0; 0x40]);
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x0f][..], // mov eax, [esp + 4]; and eax, 15
        &[0x8b, 0x04, 0x85], &widths.to_le_bytes(),       // mov eax, [widths + eax * 4]
        &[0xc3],
    ]
    .concat();
    stub(&mut t, 0x008d_c9e0, &code);
    let r = t.rng.next();
    record_n(&mut t, 0x008a_ab00, 1, r);
    for _ in 0..2000u32 {
        let fresh = t.rng.bytes(0x80);
        t.put_bytes(window, &fresh);
        for k in 0..0x10 {
            let v = t
                .rng
                .pick_or_any(&[0, 0x76, 0x77, 0x78, 0x100, 0xffff_ffff, 0x8000_0000]);
            t.put(widths + 4 * k, v);
        }
        let n = t.rng.pick(&[0u32, 1, 3, 7, 0x10, 0xffff_ffff]);
        t.put(players, n);
        t.put(aeons, n);
        t.scramble(0x0186_a3ac, 4);
        let which = t.rng.pick(&[0u32, 1, 2]);
        let (a, b) = (t.rng.int(), t.rng.int());
        t.twin(
            "MenuItem_OpenTargetWindow",
            0x008c_9240,
            &[which, a, b],
            U32,
            |o| {
                w(tk::menu_item_open_target_window(
                    o,
                    which,
                    a,
                    b,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

/// `f(value, &out, a, b)` replaced by `[out] = [cell]; return value`,
/// `value`, `a` and `b` kept after the cell; returns the cell.
fn width_cell(t: &mut Twins, f: u32) -> u32 {
    let cell = t.alloc(&[0; 16]);
    #[rustfmt::skip]
    let mut code: Vec<u8> = Vec::new();
    for (arg, k) in [(0x04u8, 4u32), (0x0c, 8), (0x10, 12)] {
        code.extend([0x8b, 0x44, 0x24, arg]); // mov eax, [esp + arg]
        code.push(0xa3); // mov [cell + k], eax
        code.extend((cell + k).to_le_bytes());
    }
    #[rustfmt::skip]
    code.extend([
        &[0x8b, 0x44, 0x24, 0x08][..],              // mov eax, [esp + 8]
        &[0x8b, 0x0d], &cell.to_le_bytes(),         // mov ecx, [cell]
        &[0x89, 0x08],                              // mov [eax], ecx
        &[0x8b, 0x44, 0x24, 0x04, 0xc3],            // mov eax, [esp + 4]; ret
    ]
    .concat());
    stub(t, f, &code);
    cell
}

#[test]
fn menu_round23_item_require_window_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0023_1c90) else {
        return;
    };
    for (f, n) in [
        (0x008f_5f70u32, 5u8),
        (0x0090_3bb0, 9),
        (0x008e_6700, 4),
        (0x0090_5550, 6),
        (0x0090_5ab0, 5),
    ] {
        let r = t.rng.next();
        record_n(&mut t, f, n, r);
    }
    let count = return_cell(&mut t, 0x0079_04b0);
    let digit_width = width_cell(&mut t, 0x0090_5230);
    let font_width = width_cell(&mut t, 0x0090_5290);
    // TkMenuMainMes: the ID + 1
    stub(&mut t, 0x008d_df80, &[0x8b, 0x44, 0x24, 0x04, 0x40, 0xc3]);
    let local = t.b.stack_range().start + 0x100;
    for _ in 0..800u32 {
        let x = t.rng.pick_or_any(&[0, 100, 0xffff_ff9c, 0x0100_0001]);
        let y = t.rng.pick_or_any(&[0, 50, 0xffff_ffce, 0x7fff_ffff]);
        let item = t.rng.pick_or_any(&[0, 0x2001, 0xffff_ffff, 0x8000_0000]);
        let n = t.rng.pick_or_any(&[0, 1, 2, 99, 0x8000_0000]);
        t.put(count, n);
        let need = t.rng.pick_or_any(&[0, 1, 2, 99, 0x7fff_ffff]);
        for cell in [digit_width, font_width] {
            let v = t.rng.float(-100.0, 400.0);
            t.put(cell, v);
        }
        t.twin(
            "TkMn2ItemRequireWindow",
            0x008c_1c90,
            &[x, y, item, need],
            U32,
            |o| {
                w(tk::tk_mn2_item_require_window(
                    o,
                    [i(x), i(y)],
                    item,
                    i(need),
                    local,
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

/// A callback in test code space logging its one argument; its address.
fn logging_callback(t: &mut Twins) -> u32 {
    let buf = t.alloc(&vec![0; 0x2_0000]);
    let log = t.alloc(&buf.to_le_bytes());
    let l = log.to_le_bytes();
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x0d][..], &l,                      // mov ecx, [log]
        &[0x8b, 0x44, 0x24, 0x04, 0x89, 0x01],      // [ecx] = arg 0
        &[0x83, 0x05], &l, &[0x04],                 // add dword [log], 4
        &[0xc3],
    ]
    .concat();
    let at = t.a.alloc_code(&code).unwrap();
    assert_eq!(t.b.alloc_code(&code).unwrap(), at);
    at
}

#[test]
fn menu_round23_small_ones_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0023_5560) else {
        return;
    };
    // the texts: the ID + 1
    for f in [0x008d_e080u32, 0x008c_1a20] {
        stub(&mut t, f, &[0x8b, 0x44, 0x24, 0x04, 0x40, 0xc3]);
    }
    for kind in [0u32, 1, 2, 3, 4, 5, 0xffff_ffff] {
        let base = t.rng.int();
        t.twin("MenuSave_KindText", 0x008b_5560, &[kind, base], U32, |o| {
            w(tk::menu_save_kind_text(o, kind, base, &mut callback))
        });
    }

    // MenuWShop_EquipBought: the slot from a cell
    let slot = t.alloc_random(0x20);
    let local = t.b.stack_range().start + 0x100;
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x08][..],              // mov eax, [esp + 8]
        &[0xc7, 0x00], &0x1234u32.to_le_bytes(),    // mov dword [eax], 0x1234
        &[0xb8], &slot.to_le_bytes(), &[0xc3],      // mov eax, slot; ret
    ]
    .concat();
    stub(&mut t, 0x007a_bbd0, &code);
    for (f, n) in [
        (0x008a_9820u32, 1u8),
        (0x008a_9c70, 1),
        (0x007a_b970, 3),
        (0x008c_3070, 1),
        (0x008c_1060, 4),
        (0x008e_ba80, 5),
        (0x008f_9230, 4),
        (0x008a_e340, 2),
        (0x008a_9230, 1),
        (0x008a_e0f0, 1),
    ] {
        let r = t.rng.next();
        record_n(&mut t, f, n, r);
    }
    for _ in 0..200 {
        let bytes = t.rng.bytes(0x20);
        t.put_bytes(slot, &bytes);
        let armour = t.rng.pick(&[0u8, 0, 1, 2, 0xff]);
        t.put(slot + 5, armour);
        let id = t.rng.int();
        t.twin("MenuWShop_EquipBought", 0x008d_9120, &[id], U32, |o| {
            w(tk::menu_wshop_equip_bought(o, id, local, &mut callback))
        });
    }

    // MenuDrawChrNameOrCommand: TOAddPktChrNameIcon returns g_TkMn2DrawY
    let draw_y = t.sym(0x0186_a238);
    for o in [&mut t.a, &mut t.b] {
        let end = o.rebase(0x008e_ba80) + 19 + 7 * 5 - 6;
        let code = [&[0xa1][..], &draw_y.to_le_bytes(), &[0xc3]].concat(); // mov eax, [draw_y]; ret
        o.write(end, &code).unwrap();
    }
    for _ in 0..200 {
        t.scramble(0x0186_a238, 4);
        let chr = t.rng.pick(&[0u32, 1, 99, 100, 101, 0x1_0064]);
        let [x, y, colour] = [t.rng.int(), t.rng.int(), t.rng.int()];
        t.twin(
            "MenuDrawChrNameOrCommand",
            0x008c_0ec0,
            &[chr, x, y, colour],
            U32,
            |o| {
                w(tk::menu_draw_chr_name_or_command(
                    o,
                    [chr, x, y, colour],
                    &mut callback,
                ))
            },
        );
    }

    // TkMenuDrawLayer: the windows' draw callbacks log their window
    for f in [0x0088_ec60u32, 0x008b_ef30, 0x008b_ea20] {
        return_counter(&mut t, f, 0);
    }
    let draw = logging_callback(&mut t);
    let windows = t.sym(0x0184_0900);
    for _ in 0..300 {
        for k in 0..17u32 {
            let w = windows + k * 0x98;
            let layer = t.rng.pick(&[0u8, 1, 5, 6, 7, 0xff]);
            t.put(w + 0x3e, layer);
            let f = t.rng.pick(&[0, draw, draw]);
            t.put(w + 0x10, f);
        }
        let paused = t.rng.pick(&[0u32, 1, 1, 2]);
        t.put(t.sym(0x0134_085c), paused);
        let layer = t.rng.pick(&[0u32, 1, 5, 6, 7, 0xff, 0xffff_ffff]);
        t.twin("TkMenuDrawLayer", 0x008a_9690, &[layer], VOID, |o| {
            tk::tk_menu_draw_layer(o, i(layer), &mut callback);
            0
        });
    }

    // TkMenuCloseAll: the menus' callbacks log their menu
    let menus = t.alloc_random(26 * 0x20);
    for _ in 0..300 {
        for k in 0..26u32 {
            let menu = menus + k * 0x20;
            let f = t.rng.pick(&[0, draw, draw]);
            t.put(menu, f);
            let a = t.rng.pick(&[0u8, 0, 1]);
            t.put(menu + 0x18, a);
            let b = t.rng.pick(&[0u8, 0, 1]);
            t.put(menu + 0x1a, b);
            let p = t.rng.pick(&[0, menu, menu, menu]);
            t.put(t.sym(0x0184_0888) + 4 * k, p);
        }
        let mask = t.rng.pick_or_any(&[0, 1, 0x0100_0000, 0xffff_ffff]);
        t.put(t.sym(0x0184_08f0), mask);
        let other = t.rng.int();
        t.put(t.sym(0x0184_08ec), other);
        t.twin("TkMenuCloseAll", 0x008a_a200, &[], U32, |o| {
            w(tk::tk_menu_close_all(o, &mut callback))
        });
    }

    // MenuSaveLoad_SyncToggle: McSync's result and out word from cells
    let result = t.alloc(&[0; 4]);
    let out = t.alloc(&[0; 4]);
    let mode = t.alloc(&[0; 4]);
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04, 0xa3][..], &mode.to_le_bytes(), // [mode] = arg 0
        &[0x8b, 0x44, 0x24, 0x0c][..],              // mov eax, [esp + 12]
        &[0x8b, 0x0d], &out.to_le_bytes(),          // mov ecx, [out]
        &[0x89, 0x08],                              // mov [eax], ecx
        &[0xa1], &result.to_le_bytes(), &[0xc3],    // mov eax, [result]; ret
    ]
    .concat();
    stub(&mut t, 0x0088_84a0, &code);
    for _ in 0..500 {
        t.scramble(0x0186_6268, 8);
        let a = t.rng.pick(&[0u8, 1, 0xff]);
        t.put(t.sym(0x0186_626a), a);
        let b = t.rng.pick(&[0u8, 1, 2, 2, 2, 3]);
        t.put(t.sym(0x0186_626b), b);
        let c = t.rng.pick(&[0u32, 2, 2, 0x102]);
        t.put(t.sym(0x0186_626c), c);
        let r = t.rng.pick(&[0u32, 1, 1, 2, 0xffff_ffff, 0x8000_0000]);
        t.put(result, r);
        let o = t.rng.pick(&[0u32, 0, 1, 0x100]);
        t.put(out, o);
        t.twin("MenuSaveLoad_SyncToggle", 0x008b_4da0, &[], U32, |o| {
            w(tk::menu_save_load_sync_toggle(o, local, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// Code in test space for both oracles; its address.
fn test_code(t: &mut Twins, code: &[u8]) -> u32 {
    let at = t.a.alloc_code(code).unwrap();
    assert_eq!(t.b.alloc_code(code).unwrap(), at);
    at
}

/// `f` replaced by a jump to `code` in test space (for functions too
/// short for their stub).
fn jump_stub(t: &mut Twins, f: u32, code: &[u8]) {
    let to = test_code(t, code);
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(f);
        let rel = to.wrapping_sub(at + 5);
        o.write(at, &[&[0xe9][..], &rel.to_le_bytes()].concat())
            .unwrap();
    }
}

/// [`record_n`] in test space behind a jump.
fn record_far(t: &mut Twins, f: u32, n: u8, ret: u32) {
    let (_, mut code) = log_args(t, n);
    code.push(0xb8);
    code.extend(ret.to_le_bytes());
    code.push(0xc3);
    jump_stub(t, f, &code);
}

/// `mov ecx, [log]`, then `[ecx + 4k] = [esp + 4 + 4k]` for `n` words and
/// `add dword [log], 4n`; the log's buffer.
fn log_args(t: &mut Twins, n: u8) -> (u32, Vec<u8>) {
    let buf = t.alloc(&vec![0; 0x2_0000]);
    let log = t.alloc(&buf.to_le_bytes());
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat();
    for k in 0..n {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    code.extend([&[0x83, 0x05][..], &l, &[4 * n]].concat());
    (buf, code)
}

#[test]
fn menu_round23_shop_and_summon_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0023_d240) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;

    // MenuSummon_GrowStat
    let player = t.alloc_random(0x100);
    let entry = t.alloc_random(8);
    let r = t.rng.next();
    record_n(&mut t, 0x0078_5330, 1, player);
    record_n(&mut t, 0x008c_ecf0, 1, entry);
    for (f, n) in [(0x0079_0550u32, 2u8), (0x008c_ee70, 4), (0x008c_3070, 1)] {
        record_n(&mut t, f, n, r ^ f);
    }
    // MenuSummon_GetStat: [cell] + 0x1000 * the third argument, logged
    let stat = t.alloc(&[0; 4]);
    let (_, mut code) = log_args(&mut t, 3);
    code.extend([0x8b, 0x44, 0x24, 0x0c, 0xc1, 0xe0, 0x0c]); // mov eax, [esp + 12]; shl eax, 12
    code.extend([0x03, 0x05]); // add eax, [stat]
    code.extend(stat.to_le_bytes());
    // the original leaves ecx alone: the caller's descriptor
    code.extend([0x8b, 0x4c, 0x24, 0x08, 0xc3]); // mov ecx, [esp + 8]; ret
    stub(&mut t, 0x008c_eca0, &code);
    // the gain functions: arg 1 * 3 - arg 0, logged
    let (_, mut code) = log_args(&mut t, 2);
    code.extend([0x8b, 0x44, 0x24, 0x08, 0x8d, 0x04, 0x40]); // mov eax, [esp + 8]; lea eax, [eax + eax * 2]
    code.extend([0x2b, 0x44, 0x24, 0x04, 0xc3]); // sub eax, [esp + 4]; ret
    let gain = test_code(&mut t, &code);
    let descs = t.sym(0x00c5_a608);
    for _ in 0..500 {
        let slot = t.rng.pick(&[0u32, 1, 5, 9, 10, 11, 0xffff_ffff]);
        let desc = descs.wrapping_add(slot.wrapping_mul(0x1c));
        t.put(desc + 0x18, gain);
        let most = t.rng.pick_or_any(&[0, 255, 9999, 99999, 0xffff_ffff]);
        t.put(desc + 0x14, most);
        let e = t.rng.bytes(8);
        t.put_bytes(entry, &e);
        let v = t.rng.pick_or_any(&[0, 1, 250, 9990, 0xffff_ff00]);
        t.put(stat, v);
        let chr = t.rng.pick_or_any(&[8, 9, 0x20, 0, 0xffff]);
        t.twin("MenuSummon_GrowStat", 0x008c_d240, &[slot, chr], U32, |o| {
            w(tk::menu_summon_grow_stat(o, i(slot), chr, &mut callback))
        });
    }

    // TkInitStartMap
    for (f, n) in [(0x0076_cfc0u32, 1u8), (0x0076_f100, 1)] {
        record_far(&mut t, f, n, r ^ f);
    }
    // file_offset_get(&names): the three names kept (the pointer is a stack one)
    let names = t.alloc(&[0; 12]);
    let mut code = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp + 4]
    for k in 0..3u8 {
        code.extend([0x8b, 0x48, 4 * k, 0x89, 0x0d]); // mov ecx, [eax + 4k]; mov [names + 4k], ecx
        code.extend((names + u32::from(4 * k)).to_le_bytes());
    }
    code.push(0xc3);
    jump_stub(&mut t, 0x0076_db40, &code);
    for _ in 0..40 {
        let tgs = t.rng.pick(&[0u8, 1, 0x80]);
        t.put(t.sym(0x0133_0832), tgs);
        let env = t.rng.pick(&[0u8, 1, 2]);
        t.put(t.sym(0x00b5_ec4f), env);
        let stale = t.rng.bytes(12);
        t.put_bytes(names, &stale);
        for (at, n) in [(0x0133_085c, 16), (0x023c_bec0, 4)] {
            t.scramble(at, n);
        }
        t.twin("TkInitStartMap", 0x0088_e880, &[], VOID, |o| {
            tk::tk_init_start_map(o, local, &mut callback);
            0
        });
    }

    // MenuSaveLoad_OpenWindow
    let window = t.alloc_random(0x80);
    return_const(&mut t, 0x008a_a1a0, window);
    stub(&mut t, 0x008d_e080, &[0x8b, 0x44, 0x24, 0x04, 0x40, 0xc3]);
    for (f, n) in [(0x008a_aeb0u32, 1u8), (0x008a_ab00, 1), (0x008a_f0d0, 2)] {
        record_n(&mut t, f, n, r ^ f);
    }
    for _ in 0..40 {
        let fresh = t.rng.bytes(0x80);
        t.put_bytes(window, &fresh);
        let flag = t.rng.pick(&[0u8, 1, 0xff]);
        t.put(t.sym(0x0186_628a), flag);
        let mode = t.rng.int();
        t.twin("MenuSaveLoad_OpenWindow", 0x008b_5000, &[mode], U32, |o| {
            w(tk::menu_save_load_open_window(o, mode, &mut callback))
        });
    }

    // MenuWShop_EquipLoss: HP and MP flip by a key on every read
    let slot = t.alloc_random(0x20);
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x08][..],              // mov eax, [esp + 8]
        &[0xc7, 0x00], &0x1234u32.to_le_bytes(),    // mov dword [eax], 0x1234
        &[0xb8], &slot.to_le_bytes(), &[0xc3],      // mov eax, slot; ret
    ]
    .concat();
    stub(&mut t, 0x007a_bbd0, &code);
    let mut flips = Vec::new();
    for f in [0x008a_98c0u32, 0x008a_9970] {
        let (cell, key) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
        #[rustfmt::skip]
        let code: Vec<u8> = [
            &[0xa1][..], &cell.to_le_bytes(),       // mov eax, [cell]
            &[0x8b, 0x0d], &key.to_le_bytes(),      // mov ecx, [key]
            &[0x31, 0x0d], &cell.to_le_bytes(),     // xor [cell], ecx
            &[0xc3],
        ]
        .concat();
        stub(&mut t, f, &code);
        flips.push((cell, key));
    }
    for (f, n) in [
        (0x008a_9820u32, 1u8),
        (0x008a_9c70, 1),
        (0x007a_b970, 3),
        (0x008a_ae90, 2),
        (0x008a_aef0, 2),
    ] {
        record_far(&mut t, f, n, r ^ f);
    }
    for _ in 0..500 {
        let bytes = t.rng.bytes(0x20);
        t.put_bytes(slot, &bytes);
        let armour = t.rng.pick(&[0u8, 0, 1, 2]);
        t.put(slot + 5, armour);
        for &(cell, key) in &flips {
            let v = t.rng.int();
            t.put(cell, v);
            let k = t.rng.pick_or_any(&[0, 1, 0x8000_0000, 0xffff_ffff]);
            t.put(key, k);
        }
        let id = t.rng.int();
        t.twin("MenuWShop_EquipLoss", 0x008d_8520, &[id], U32, |o| {
            w(tk::menu_wshop_equip_loss(o, id, local, &mut callback))
        });
    }

    // the command windows: the texts the ID + 1, the helps + 0x100; the
    // window logs the six words behind its pointers and the rest
    stub(&mut t, 0x008d_df80, &[0x8b, 0x44, 0x24, 0x04, 0x40, 0xc3]);
    stub(
        &mut t,
        0x008d_df60,
        &[0x8b, 0x44, 0x24, 0x04, 0x05, 0, 1, 0, 0, 0xc3],
    );
    let (_, mut code) = log_args(&mut t, 9);
    // the log grew by 36: the six words go after it, over the two
    // pointers (stack addresses)
    code.extend([0x8b, 0x0d]);
    let log_cell = u32::from_le_bytes(code[2..6].try_into().unwrap());
    code.extend(log_cell.to_le_bytes()); // mov ecx, [log]
    code.extend([0x31, 0xc0, 0x89, 0x41, 0xe0, 0x89, 0x41, 0xe4]); // xor eax, eax; [ecx - 32] = [ecx - 28] = 0
    for (arg, k) in [(8u8, 0u8), (12, 12)] {
        code.extend([0x8b, 0x54, 0x24, arg]); // mov edx, [esp + arg]
        for j in 0..3u8 {
            code.extend([0x8b, 0x42, 4 * j, 0x89, 0x41, k + 4 * j]); // [ecx + k + 4j] = [edx + 4j]
        }
    }
    code.extend([&[0x83, 0x05][..], &log_cell.to_le_bytes(), &[24]].concat()); // add dword [log], 24
    code.extend([0xb8, 0x77, 0, 0, 0, 0xc3]); // mov eax, 0x77; ret
    jump_stub(&mut t, 0x008e_4930, &code);
    for f in [0x008d_83e0u32, 0x008d_a9c0] {
        return_counter(&mut t, f, 0);
    }
    for _ in 0..20 {
        for at in [0x0186_aaf0, 0x0186_ac34] {
            t.scramble(at, 8);
        }
        t.twin("MenuWShop_OpenCommandWindow", 0x008d_8110, &[], U32, |o| {
            w(tk::menu_wshop_open_command_window(
                o,
                local,
                &mut ui_remap,
                &mut callback,
            ))
        });
        t.twin("MenuWShop_Start", 0x008d_a7a0, &[], U32, |o| {
            w(tk::menu_wshop_start(o, local, &mut ui_remap, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round23_reads_cursors_and_pads_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0023_d2c0) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let r = t.rng.next();

    // TkCdReadWithMallocWithMessage: the CD object's methods log in test space
    let cd = t.alloc(&[0; 0x20]);
    return_const(&mut t, 0x0076_d0d0, cd);
    let size = t.alloc(&[0; 4]);
    let calm = t.alloc(&[0; 4]);
    for (o, n) in [(0u32, 4u8), (8, 1), (0x10, 1)] {
        let (_, mut code) = log_args(&mut t, n);
        code.extend([0xb8, 0x11, 0, 0, 0, 0xc3]); // mov eax, 0x11; ret
        let f = test_code(&mut t, &code);
        t.put(cd + o, f);
    }
    let (_, mut code) = log_args(&mut t, 1);
    code.extend([&[0xa1][..], &size.to_le_bytes(), &[0xc3]].concat()); // mov eax, [size]; ret
    let f = test_code(&mut t, &code);
    t.put(cd + 0x14, f);
    let (_, mut code) = log_args(&mut t, 1);
    code.extend([&[0xa1][..], &calm.to_le_bytes(), &[0xc3]].concat());
    jump_stub(&mut t, 0x0067_ab50, &code);
    let heap = t.alloc(&[0; 0x40]);
    for (f, n, ret) in [
        (0x0067_aae0u32, 1u8, 0x1234u32),
        (0x0087_3c70, 1, r),
        (0x0083_c4a0, 1, r),
        (0x0062_f4c0, 1, r),
    ] {
        record_far(&mut t, f, n, ret);
    }
    // user_malloc: a buffer, the size logged
    let (_, mut code) = log_args(&mut t, 1);
    code.extend([&[0xb8][..], &heap.to_le_bytes(), &[0xc3]].concat());
    jump_stub(&mut t, 0x0068_7020, &code);
    for _ in 0..300 {
        let s = t
            .rng
            .pick_or_any(&[0, 1, 0x800, 0x801, 0x7fff_f900, 0xffff_ffff]);
        t.put(size, s);
        let c = t.rng.pick(&[0u32, 0, 1, 2]);
        t.put(calm, c);
        t.scramble(heap, 8);
        let dir = t.rng.int();
        let file = t.rng.pick_or_any(&[0, 9, 17, 18, 0xffff_fff7, 0x8000_0000]);
        t.twin(
            "TkCdReadWithMallocWithMessage",
            0x0088_d2c0,
            &[dir, file],
            U32,
            |o| {
                w(tk::tk_cd_read_with_malloc_with_message(
                    o,
                    dir,
                    i(file),
                    &mut callback,
                ))
            },
        );
    }

    // MenuSave_DicText: the language from cells, the texts copied by
    // TOFFXToSJIS as they are
    let lang = return_cell(&mut t, 0x008a_c2f0);
    // MsGetSaveConfigEnglish flips by a key on every read
    let (english, flip) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0xa1][..], &english.to_le_bytes(),        // mov eax, [english]
        &[0x8b, 0x0d], &flip.to_le_bytes(),         // mov ecx, [flip]
        &[0x31, 0x0d], &english.to_le_bytes(),      // xor [english], ecx
        &[0xc3],
    ]
    .concat();
    jump_stub(&mut t, 0x0078_51d0, &code);
    // the dictionary, its texts behind it
    let dic = t.alloc(&[&[0; 0x40][..], b"Save\0Load completed\0\0"].concat());
    let texts = dic + 0x40;
    record_far(&mut t, 0x0086_c430, 2, texts + 5);
    record_far(&mut t, 0x008b_7f60, 1, dic);
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x4c, 0x24, 0x04][..],              // mov ecx, [esp + 4] (from)
        &[0x8b, 0x54, 0x24, 0x08],                  // mov edx, [esp + 8] (to)
        &[0x8a, 0x01, 0x88, 0x02, 0x41, 0x42],      // mov al, [ecx]; mov [edx], al; inc ecx; inc edx
        &[0x84, 0xc0, 0x75, 0xf6, 0xc3],            // test al, al; jne; ret
    ]
    .concat();
    jump_stub(&mut t, 0x008b_6200, &code);
    let out = t.alloc(&[0; 0x40]);
    for _ in 0..300 {
        let l = t.rng.pick(&[0u32, 0, 1]);
        t.put(lang, l);
        let e = t.rng.pick(&[0u32, 0, 1]);
        t.put(english, e);
        let k = t.rng.pick(&[0u32, 0, 1]);
        t.put(flip, k);
        let index = t.rng.pick(&[0u32, 1, 3, 7]);
        // the entry's offset points at the texts
        let entry = dic + 4 * index;
        let off = t.rng.pick(&[0u32, 5]) + texts - entry;
        t.put(entry + 2, u16::try_from(off).unwrap());
        t.scramble(out, 0x40);
        t.twin(
            "MenuSave_DicText",
            0x008b_43e0,
            &[out, 0, 0, index],
            VOID,
            |o| {
                tk::menu_save_dic_text(o, out, i(index), local, &mut callback);
                0
            },
        );
    }

    // MenuItem_DrawCellCursor
    let count = return_cell(&mut t, 0x008c_5030);
    record_far(&mut t, 0x008c_0580, 3, r);
    for _ in 0..1000 {
        let on = t.rng.pick(&[0u32, 1, 1, 1, 2]);
        t.put(t.sym(0x0186_a270), on);
        let n = t
            .rng
            .pick_or_any(&[0, 1, 3, 9, 0xffff_ffff, 0x0100_0001, 0x0100_0003]);
        t.put(count, n);
        let row = t.rng.pick_or_any(&[0, 1, 2, 8, 0xffff_ffff, 0x7fff_ffff]);
        t.put(t.sym(0x0186_a26c), row);
        let x = t.rng.pick_or_any(&[0, 10, 0xffff_fff6, 0x0100_0001]);
        let y = t.rng.pick_or_any(&[0, 10, 0xffff_fff6, 0x7fff_ffff]);
        t.twin(
            "MenuItem_DrawCellCursor",
            0x008c_4a90,
            &[0, x, y],
            VOID,
            |o| {
                tk::menu_item_draw_cell_cursor(o, [i(x), i(y)], &mut ui_remap, &mut callback);
                0
            },
        );
    }

    // MenuItem_UseOnTargets
    let item = t.alloc_random(0x40);
    for (f, n, ret) in [
        (0x008c_8e20u32, 2u8, r),
        (0x008a_b280, 1, item), // the second argument is a stack address
    ] {
        record_far(&mut t, f, n, ret);
    }
    // the heal amount: who * 3 + command, logged
    let (_, mut code) = log_args(&mut t, 3);
    code.extend([0x8b, 0x44, 0x24, 0x08, 0x8d, 0x04, 0x40]); // mov eax, [esp + 8]; lea eax, [eax + eax * 2]
    code.extend([0x03, 0x44, 0x24, 0x04, 0xc3]); // add eax, [esp + 4]; ret
    jump_stub(&mut t, 0x008c_69a0, &code);
    let aeons = return_cell(&mut t, 0x008a_9bf0);
    let party = return_cell(&mut t, 0x008a_9b30);
    stub(
        &mut t,
        0x008a_9bc0,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xc0, 0x20, 0xc3],
    );
    stub(
        &mut t,
        0x008a_9ad0,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xc0, 0x10, 0xc3],
    );
    for _ in 0..500 {
        let single = t.rng.pick(&[0u8, 1, 1, 2]);
        t.put(t.sym(0x0186_a391), single);
        t.scramble(0x0186_a3a8, 2);
        let flags = t.rng.pick(&[0u8, 0x40, 0x80, 0xc0, 0x3f]);
        t.put(item + 0x22, flags);
        for cell in [aeons, party] {
            let n = t.rng.pick(&[0u32, 1, 3, 0xffff_ffff]);
            t.put(cell, n);
        }
        let target = t.rng.int();
        t.twin("MenuItem_UseOnTargets", 0x008c_8d50, &[target], VOID, |o| {
            tk::menu_item_use_on_targets(o, target, local, &mut callback);
            0
        });
    }

    // MenuWeapon_DrawRow: MsGetSaveWeapon keeps the ID it was asked for
    let weapon_id = t.alloc(&[0; 4]);
    for (f, n) in [(0x008f_5f70u32, 5u8), (0x008e_67f0, 4)] {
        record_far(&mut t, f, n, r);
    }
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x08][..],              // mov eax, [esp + 8]
        &[0xc7, 0x00], &0x1234u32.to_le_bytes(),    // mov dword [eax], 0x1234
        &[0x8b, 0x44, 0x24, 0x04],                  // mov eax, [esp + 4]
        &[0xa3], &weapon_id.to_le_bytes(), &[0xc3], // mov [weapon_id], eax; ret
    ]
    .concat();
    jump_stub(&mut t, 0x007a_bbd0, &code);
    let list = t.sym(0x0159_7770);
    // (weapon_id is set by the MsGetSaveWeapon stand-in)
    for _ in 0..1000 {
        let window = t.alloc_random(0x80);
        let rows = t.rng.pick(&[0u16, 1, 5, 0x200]);
        t.put(window + 0x30, rows);
        let scroll = t.rng.pick(&[0i16, 1, -1, 0x7fff, -0x8000]);
        t.put(window + 0x46, scroll);
        let row = t.rng.pick(&[0u32, 1, 4, 5, 0x1ff, 0xffff_ffff]);
        if row < 0x200 {
            let id = t.rng.pick(&[0u16, 0x5001, 0xffff]);
            t.put(list + 4 * row, id);
        }
        let fixed = t.rng.pick(&[0u32, 0, 1, 0xffff_ffff]);
        let x = t.rng.float(-100.0, 600.0);
        let y = t.rng.float(-100.0, 600.0);
        t.twin(
            "MenuWeapon_DrawRow",
            0x008d_0170,
            &[window, x.to_bits(), y.to_bits(), row, fixed],
            VOID,
            |o| {
                tk::menu_weapon_draw_row(
                    o,
                    window,
                    [x, y],
                    [i(row), i(fixed)],
                    local,
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
    }

    // Pad_ActuatorIds: the pad answers from a table
    let answers = t.alloc(&[0; 0x400]);
    // scePadInfoAct(port, slot, a, k): k 0 the count cell, else answers[(a + k) & 0xff]
    #[rustfmt::skip]
    let act: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x10, 0x85, 0xc0, 0x75, 0x06][..], // mov eax, [esp + 16]; test eax, eax; jne +6
        &[0xa1], &answers.to_le_bytes(), &[0xc3],               // mov eax, [answers]; ret
        &[0x8b, 0x44, 0x24, 0x0c, 0x03, 0x44, 0x24, 0x10],     // mov eax, [esp + 12]; add eax, [esp + 16]
        &[0x25, 0xff, 0, 0, 0],                                 // and eax, 0xff
        &[0x8b, 0x04, 0x85], &(answers + 0x100).to_le_bytes(),  // mov eax, [answers + 0x100 + eax * 4]
        &[0xc3],
    ]
    .concat();
    jump_stub(&mut t, 0x0062_fb40, &act);
    // scePadInfoComb(port, slot, c, k): c -1 the combinations cell; k -1
    // a count; else (c * 4 + k) * 7 & 0xff
    #[rustfmt::skip]
    let comb: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x0c, 0x83, 0xf8, 0xff, 0x75, 0x06][..], // mov eax, [esp + 12]; cmp eax, -1; jne +6
        &[0xa1], &(answers + 4).to_le_bytes(), &[0xc3],               // mov eax, [answers + 4]; ret
        &[0x8b, 0x4c, 0x24, 0x10, 0x83, 0xf9, 0xff, 0x75, 0x06],      // mov ecx, [esp + 16]; cmp ecx, -1; jne +6
        &[0xa1], &(answers + 8).to_le_bytes(), &[0xc3],               // mov eax, [answers + 8]; ret
        &[0x8d, 0x04, 0x81, 0x6b, 0xc0, 0x07, 0x25, 0xff, 0, 0, 0],   // lea eax, [ecx + eax * 4]; imul eax, 7; and eax, 0xff
        &[0xc3],
    ]
    .concat();
    jump_stub(&mut t, 0x0062_fb50, &comb);
    for _ in 0..500 {
        let any = t.rng.pick(&[0u32, 1, 1, 1, 2]);
        t.put(answers, any);
        let combs = t.rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff]);
        t.put(answers + 4, combs);
        let n = t.rng.pick(&[0u32, 1, 2, 4]);
        t.put(answers + 8, n);
        for k in 0..0x100 {
            let kind = t.rng.pick(&[0u32, 0, 1, 2, 3]);
            t.put(answers + 0x100 + 4 * k, kind);
        }
        let (port, slot) = (t.rng.below(2), t.rng.below(2));
        t.twin("Pad_ActuatorIds", 0x0088_8a70, &[port, slot], U32, |o| {
            w(tk::pad_actuator_ids(o, port, slot, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// A float's bits: NaN, any non-signalling pattern, or a small value.
fn quiet_float(t: &mut Twins) -> u32 {
    match t.rng.below(8) {
        0 => f32::NAN.to_bits(),
        1 => t.rng.next() & 0xffbf_ffff | 0x0040_0000, // no signalling NaNs
        _ => (f32::from(i16::try_from(t.rng.below(2000)).unwrap_or(0) - 1000) / 4.0).to_bits(),
    }
}

#[test]
fn menu_round24_plates_rows_and_slides_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0024_1260) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let r = t.rng.next();

    // MenuItem_DrawChrSmallPlate
    record_far(&mut t, 0x008f_51e0, 3, r);
    for _ in 0..200 {
        let (x, y) = (quiet_float(&mut t), quiet_float(&mut t));
        let chr = t.rng.int();
        t.twin(
            "MenuItem_DrawChrSmallPlate",
            0x008c_1260,
            &[x, y, chr],
            U32,
            |o| {
                w(tk::menu_item_draw_chr_small_plate(
                    o,
                    [f32::from_bits(x), f32::from_bits(y)],
                    chr,
                    &mut callback,
                ))
            },
        );
    }

    // TkMenuConfigHelp: MsGetSaveConfigEnglish from a cell
    let english = return_cell(&mut t, 0x0078_51d0);
    let config = ffx_battle::save::bridge::MENU.sym;
    for round in 0..200u32 {
        let e = t.rng.pick(&[0u32, 0, 1, 0x100]);
        t.put(english, e);
        let id = t
            .rng
            .pick_or_any(&[0, 1, 2, 0x10, 0x2000_0001, 0xffff_ffff])
            & 0xffff_1fff;
        t.twin("TkMenuConfigHelp", 0x008d_c970, &[id], U32, |o| {
            w(tk::tk_menu_config_help(o, id, &mut callback))
        });
        if round % 2 == 0 {
            t.scramble(config, 4);
        }
    }

    // MenuWeapon_DrawRows: the rows recorded
    record_far(&mut t, 0x008d_0170, 5, r);
    for _ in 0..200 {
        let window = t.rng.next();
        let fixed = t.rng.int();
        let first = t.rng.int();
        let d = t.rng.next();
        let y = t.rng.int();
        t.twin(
            "MenuWeapon_DrawRows",
            0x008d_0250,
            &[window, fixed, first, d, y],
            VOID,
            |o| {
                tk::menu_weapon_draw_rows(
                    o,
                    window,
                    [i(fixed), i(first), i(y)],
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
    }

    // MenuSave_BuildIconSys: MenuSave_DicText copies a text of its own
    let text = t.alloc(&[0; 0x100]);
    // the three words after `out` (a stack address) logged
    let buf = t.alloc(&[0; 0x4000]);
    let log = t.alloc(&buf.to_le_bytes());
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &log.to_le_bytes()].concat(); // mov ecx, [log]
    for k in 0..3u8 {
        code.extend([0x8b, 0x44, 0x24, 8 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    code.extend([&[0x83, 0x05][..], &log.to_le_bytes(), &[12]].concat());
    #[rustfmt::skip]
    code.extend([
        &[0x8b, 0x54, 0x24, 0x04][..],              // mov edx, [esp + 4] (out)
        &[0xb9], &text.to_le_bytes(),               // mov ecx, text
        &[0x8a, 0x01, 0x88, 0x02, 0x41, 0x42],      // mov al, [ecx]; mov [edx], al; inc ecx; inc edx
        &[0x84, 0xc0, 0x75, 0xf6, 0xc3],            // test al, al; jne; ret
    ]
    .concat());
    jump_stub(&mut t, 0x008b_43e0, &code);
    for _ in 0..200 {
        let len = t.rng.pick(&[0usize, 1, 5, 0x40, 0xff]);
        let mut s: Vec<u8> = (0..len)
            .map(|_| 1 + u8::try_from(t.rng.below(0xff)).unwrap())
            .collect();
        s.push(0);
        t.put_bytes(text, &s);
        t.scramble(0x0159_e870, 0x3c4);
        let n = t.rng.pick_or_any(&[0, 1, 9, 10, 99, 100]);
        let (b, index) = (t.rng.next(), t.rng.int());
        t.twin(
            "MenuSave_BuildIconSys",
            0x008b_42e0,
            &[n, b, index],
            VOID,
            |o| {
                tk::menu_save_build_icon_sys(o, [n, b, index], local, &mut callback);
                0
            },
        );
    }

    // BtlEnd_StepEntry: BtlEnd_GetSP from a cell, both logged
    let sp = t.alloc(&[0; 4]);
    let (_, mut code) = log_args(&mut t, 1);
    code.extend([&[0xa1][..], &sp.to_le_bytes(), &[0xc3]].concat());
    jump_stub(&mut t, 0x008b_da50, &code);
    record_far(&mut t, 0x008b_dfc0, 2, r);
    for _ in 0..1000 {
        let k = t.rng.pick(&[0u32, 1, 2, 3, 5, 0xffff_ffff]);
        let e = t.sym(0x0186_9f22).wrapping_add(k.wrapping_mul(14));
        let state = t.rng.pick(&[0i16, 1, 2, 3, 4, 5, 6, -1, 0x7fff, 0x100]);
        let timer = t.rng.pick(&[
            0i16, 1, -1, 2, 0xf1c, 0xf1d, 0xfff, 0x1000, 0x7fff, -0x8000, 0x7f20,
        ]);
        t.put(e, timer);
        t.put(e + 2, state);
        let chr = t.rng.pick(&[0i16, 1, 7, -1, 0x7fff]);
        t.put(e + 4, chr);
        let s = t
            .rng
            .pick(&[0u32, 0x62, 0x63, 0x64, 0xffff_ffff, 0x8000_0000]);
        t.put(sp, s);
        t.twin("BtlEnd_StepEntry", 0x008b_b690, &[k], U32, |o| {
            w(tk::btl_end_step_entry(o, k, &mut callback))
        });
    }

    // the name entry's face slides
    record_n(&mut t, 0x008c_0250, 5, 0x66);
    record_n(&mut t, 0x008c_2c30, 4, 0x77);
    for round in 0..400u32 {
        let flag = t.rng.pick(&[0u32, 1, 0x100]);
        return_const(&mut t, 0x008a_8e50, flag);
        let step = match t.rng.below(3) {
            0 => t.rng.next(),
            _ => t.rng.below(0x200),
        };
        t.put(t.sym(0x0186_a9ec), step);
        t.scramble(0x023c_c458, 0x24);
        let xywh: [u32; 4] = std::array::from_fn(|_| quiet_float(&mut t));
        let slide = match round % 4 {
            0 => 0,
            1 => t.rng.next(),
            _ => t.rng.below(0x2000).wrapping_sub(0x1000),
        };
        let (cur, next) = (t.rng.next(), t.rng.next());
        let args = [xywh[0], xywh[1], xywh[2], xywh[3], slide, cur, next];
        let (what, f) = if round % 2 == 0 {
            ("MenuNameEntry_DrawCharFaceSlide", 0x008d_33b0)
        } else {
            ("MenuNameEntry_DrawSummonFaceSlide", 0x008d_35a0)
        };
        t.twin(what, f, &args, U32, |o| {
            w(tk::menu_name_entry_draw_face_slide(
                o,
                xywh,
                slide.cast_signed(),
                [cur, next],
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round24_eases_pads_and_kaizou_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0024_c6f0) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let r = t.rng.next();

    // MenuScene_KeyEase: _CIcos through the stand-ins
    install_x87(&mut t);
    for _ in 0..2000 {
        let key = t
            .rng
            .pick(&[0u32, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x7000])
            | (t.rng.next() & !0x7000);
        let frames = t
            .rng
            .pick_or_any(&[0, 1, 2, 3, 30, 0xffff_ffff, 0x8000_0000]);
        let frame = match t.rng.below(3) {
            0 => t.rng.int(),
            _ => t.rng.below(frames.clamp(1, 64) + 1),
        };
        t.twin_f(
            "MenuScene_KeyEase",
            0x00a7_c6f0,
            &[key, frame, frames],
            |o| {
                tk::menu_scene_key_ease(o, key, i(frame), i(frames), &mut |o, v| {
                    x87_call(o, 0x00b0_c3b4, v, None)
                })
            },
        );
    }

    // Pad_RepeatButton: inputGetCurTime from a cell
    let now = t.alloc(&[0; 4]);
    stub(
        &mut t,
        0x0063_0c60,
        &[&[0xd9, 0x05][..], &now.to_le_bytes(), &[0xc3]].concat(),
    );
    let work = t.alloc_random(0x200);
    for _ in 0..3000 {
        let time = match t.rng.below(4) {
            0 => quiet_float(&mut t),
            _ => t.rng.float(0.0, 100.0).to_bits(),
        };
        t.put(now, time);
        let i = t.rng.below(4);
        let slot = t.rng.pick(&[0u32, 1, 2, 3, 0x1_0002, 0xffff_0001]);
        let k = i + 4 * (slot & 3);
        let (counter, timer, last) = (
            work + 0xa0 + 2 * k,
            work + 0xb0 + 4 * k,
            work + 0xd0 + 4 * k,
        );
        let left = match t.rng.below(4) {
            0 => quiet_float(&mut t),
            1 => 0,
            _ => t.rng.float(-1.0, 1.0).to_bits(),
        };
        t.put(timer, left);
        let then = match t.rng.below(4) {
            0 => quiet_float(&mut t),
            1 => time,
            _ => (f32::from_bits(time) - t.rng.float(0.0, 1.0)).to_bits(),
        };
        t.put(last, then);
        let c = t.rng.bytes(2);
        t.put_bytes(counter, &c);
        t.scramble(0x00c5_4e34, 8);
        let bits = [0u32, 1, 2, 3, 0xffff, 0x1_0001, 0x8000];
        let pressed = t.rng.pick_or_any(&bits);
        let held = if t.rng.below(2) == 0 {
            pressed
        } else {
            t.rng.pick_or_any(&bits)
        };
        let b = t.rng.next();
        // a scrambled mask so every bit matters
        let args = [work, b, i, pressed, held, slot];
        t.twin("Pad_RepeatButton", 0x0088_9980, &args, U32, |o| {
            w(tk::pad_repeat_button(
                o,
                work,
                [i, pressed, held, slot],
                // the stand-in's `fld dword [now]`, run
                &mut |o| {
                    let f = o.rebase(0x0063_0c60);
                    fcallback(o, f, &[])
                },
            ))
        });
    }

    // the customise menu: MsGetSaveWeapon returns a record and a name
    let rec = t.alloc(&[0; 8]);
    let names = t.alloc(&[0; 4]);
    let (_, mut code) = log_args(&mut t, 1);
    #[rustfmt::skip]
    code.extend([
        &[0x8b, 0x44, 0x24, 0x08][..],              // mov eax, [esp + 8]
        &[0x8b, 0x0d], &names.to_le_bytes(),        // mov ecx, [names]
        &[0x89, 0x08],                              // mov [eax], ecx
        &[0xb8], &rec.to_le_bytes(), &[0xc3],       // mov eax, rec; ret
    ]
    .concat());
    jump_stub(&mut t, 0x007a_bbd0, &code);
    // ToGetBtlEasyFontWidth: the width from a cell, the other words logged
    let width = t.alloc(&[0; 4]);
    let buf = t.alloc(&[0; 0x4000]);
    let log = t.alloc(&buf.to_le_bytes());
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &log.to_le_bytes()].concat();
    for k in [0u8, 2, 3, 4] {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    #[rustfmt::skip]
    code.extend([
        &[0x83, 0x05][..], &log.to_le_bytes(), &[20],  // add dword [log], 20
        &[0x8b, 0x44, 0x24, 0x08],                     // mov eax, [esp + 8]
        &[0x8b, 0x0d], &width.to_le_bytes(),           // mov ecx, [width]
        &[0x89, 0x08, 0xc3],                           // mov [eax], ecx; ret
    ]
    .concat());
    jump_stub(&mut t, 0x0090_5290, &code);
    for (f, n) in [(0x0090_5ab0u32, 6u8), (0x008f_5f70, 5), (0x008e_67f0, 4)] {
        record_far(&mut t, f, n, r);
    }
    let selectable = t.alloc(&[0; 4]);
    let (_, mut code) = log_args(&mut t, 2);
    code.extend([&[0xa1][..], &selectable.to_le_bytes(), &[0xc3]].concat());
    jump_stub(&mut t, 0x008d_5750, &code);
    let list = t.alloc_random(0x400);
    t.put(t.sym(0x0186_aa38), list);
    for _ in 0..500 {
        let n = t.rng.next();
        t.put(names, n);
        let wd = quiet_float(&mut t);
        t.put(width, wd);
        let x = t.rng.int();
        let y = t.rng.int();
        let k = t.rng.below(0x200);
        t.twin(
            "MenuKaizou_DrawWeaponTitle",
            0x008d_6660,
            &[x, y, k],
            VOID,
            |o| {
                tk::menu_kaizou_draw_weapon_title(
                    o,
                    [i(x), i(y), i(k)],
                    local,
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
    }
    for _ in 0..1000 {
        let window = t.alloc_random(0x80);
        let rows = t.rng.pick(&[0u16, 1, 5, 0x200]);
        t.put(window + 0x30, rows);
        let scroll = t.rng.pick(&[0i16, 1, -1, 0x7fff, -0x8000]);
        t.put(window + 0x46, scroll);
        let row = t.rng.pick(&[0u32, 1, 4, 5, 0x1ff, 0xffff_ffff]);
        let have = t.rng.pick(&[0u8, 1, 1, 0x80]);
        t.put(rec + 2, have);
        let flags = t.rng.pick(&[0u8, 2, 1, 0xfd, 0xff]);
        t.put(rec + 3, flags);
        let sel = t.rng.pick(&[0u32, 0, 1, 0x100]);
        t.put(selectable, sel);
        let fixed = t.rng.pick(&[0u32, 0, 1, 0xffff_ffff]);
        let x = quiet_float(&mut t);
        let y = t.rng.float(-100.0, 600.0).to_bits();
        t.twin(
            "MenuKaizou_DrawRow",
            0x008d_6760,
            &[window, x, y, row, fixed],
            VOID,
            |o| {
                tk::menu_kaizou_draw_row(
                    o,
                    window,
                    [f32::from_bits(x), f32::from_bits(y)],
                    [i(row), i(fixed)],
                    local,
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
    }

    // MenuPlayer_DrawFaceSlide: eight party members in a scrambled order
    return_const(&mut t, 0x008a_9b30, 8);
    record_n(&mut t, 0x008c_0250, 5, 0x66);
    record_n(&mut t, 0x008c_2c30, 4, 0x77);
    for round in 0..400u32 {
        let flag = t.rng.pick(&[0u32, 1, 0x100]);
        return_const(&mut t, 0x008a_8e50, flag);
        let step = match t.rng.below(3) {
            0 => t.rng.next(),
            _ => t.rng.below(0x200),
        };
        t.put(t.sym(0x0186_a9ec), step);
        t.scramble(0x023c_c458, 0x24);
        t.scramble(0x0184_1c14, 8);
        let xywh: [u32; 4] = std::array::from_fn(|_| quiet_float(&mut t));
        let slide = match round % 4 {
            0 => 0,
            1 => t.rng.next(),
            _ => t.rng.below(0x2000).wrapping_sub(0x1000),
        };
        let (cur, next) = (t.rng.below(16), t.rng.below(16));
        let args = [xywh[0], xywh[1], xywh[2], xywh[3], slide, cur, next];
        t.twin("MenuPlayer_DrawFaceSlide", 0x008d_32b0, &args, U32, |o| {
            w(tk::menu_player_draw_face_slide(
                o,
                xywh,
                slide.cast_signed(),
                [cur, next],
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round24_tables_masks_and_rows_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0024_ac70) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let r = t.rng.next();

    // Pad_RepeatButtons: Pad_RepeatButton logged, its eax from a cell
    let eax = t.alloc(&[0; 4]);
    let (_, mut code) = log_args(&mut t, 6);
    code.extend(
        [
            &[0xa1][..],
            &eax.to_le_bytes(),
            &[0x03, 0x44, 0x24, 0x0c, 0xc3],
        ]
        .concat(),
    ); // mov eax, [eax]; add eax, [esp + 12]; ret
    jump_stub(&mut t, 0x0088_9980, &code);
    for _ in 0..300 {
        let e = t.rng.next();
        t.put(eax, e);
        let args: [u32; 5] = std::array::from_fn(|_| t.rng.next());
        t.twin("Pad_RepeatButtons", 0x0088_9940, &args, U32, |o| {
            w(tk::pad_repeat_buttons(
                o,
                args[0],
                [args[1], args[2], args[3], args[4]],
                &mut callback,
            ))
        });
    }

    // MenuKaizou_DrawRows: the rows recorded
    record_far(&mut t, 0x008d_6760, 5, r);
    for _ in 0..200 {
        let window = t.rng.next();
        let (fixed, first, d, y) = (t.rng.int(), t.rng.int(), t.rng.next(), t.rng.int());
        t.twin(
            "MenuKaizou_DrawRows",
            0x008d_6560,
            &[window, fixed, first, d, y],
            VOID,
            |o| {
                tk::menu_kaizou_draw_rows(
                    o,
                    window,
                    [i(fixed), i(first), i(y)],
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
    }

    // Math_SemicircleTable: _CIsin's stand-in, and _CIacos as
    // atan2(sqrt(1 - x²), x)
    install_x87(&mut t);
    #[rustfmt::skip]
    let acos = test_code(&mut t, &[
        0xd9, 0xc0, 0xd8, 0xc8,  // fld st0; fmul st0, st0
        0xd9, 0xe8, 0xde, 0xe1,  // fld1; fsubrp st1 (1 - x²)
        0xd9, 0xfa, 0xd9, 0xc9,  // fsqrt; fxch st1
        0xd9, 0xf3, 0xc3,        // fpatan; ret
    ]);
    t.put(t.sym(0x00b0_c3c4), acos);
    let out = t.alloc(&[0; 0x400]);
    for _ in 0..300 {
        let n = t.rng.pick(&[0u32, 1, 2, 3, 5, 16, 100, 0x1ff, 0xffff_ffff]);
        let amp = t
            .rng
            .pick_or_any(&[0, 1, 10, 0x7fff, 0xffff_fff0, 0x10_0000]);
        let fill = t.rng.bytes(0x400);
        t.put_bytes(out, &fill);
        t.twin(
            "Math_SemicircleTable",
            0x008c_ac70,
            &[out + 2, n, amp],
            U32,
            |o| {
                w(tk::math_semicircle_table(
                    o,
                    out + 2,
                    [i(n), i(amp)],
                    &mut |o, sin, v| {
                        x87_call(o, if sin { 0x00b0_c3b8 } else { 0x00b0_c3c4 }, v, None)
                    },
                ))
            },
        );
    }

    // MenuAbility_CommandTargetMask: the command record, the lists and
    // MenuItem_TargetNeedsRecovery stand-ins logged
    let rec = t.alloc(&[0; 0x40]);
    let (_, mut code) = log_args(&mut t, 1);
    code.extend([&[0xb8][..], &rec.to_le_bytes(), &[0xc3]].concat());
    jump_stub(&mut t, 0x0079_0a90, &code);
    let aeons = return_cell(&mut t, 0x008a_9bf0);
    let party = return_cell(&mut t, 0x008a_9b30);
    for (f, k) in [(0x008a_9bc0u32, 0x20u8), (0x008a_9ad0, 0x10)] {
        // i * 3 + k
        stub(
            &mut t,
            f,
            &[
                0x8b, 0x44, 0x24, 0x04, 0x8d, 0x04, 0x40, 0x83, 0xc0, k, 0xc3,
            ],
        );
    }
    let (_, mut code) = log_args(&mut t, 3);
    #[rustfmt::skip]
    code.extend([
        0x8b, 0x44, 0x24, 0x0c, 0xc1, 0xe8, 0x05,       // mov eax, [esp + 12]; shr eax, 5
        0x03, 0x44, 0x24, 0x0c, 0x03, 0x44, 0x24, 0x04, // add eax, [esp + 12]; add eax, [esp + 4]
        0x33, 0x44, 0x24, 0x08, 0xc1, 0xe8, 0x01,       // xor eax, [esp + 8]; shr eax, 1
        0x83, 0xe0, 0x01, 0xc3,                         // and eax, 1; ret
    ]);
    jump_stub(&mut t, 0x008c_6a70, &code);
    for _ in 0..1000 {
        let kind = t.rng.pick(&[0u8, 1, 2, 3, 4, 0xff]);
        t.put(rec + 0x23, kind);
        let targets = u16::from_le_bytes([t.rng.bytes(1)[0], t.rng.bytes(1)[0]]);
        t.put(rec + 0x1a, targets);
        let flags = t.rng.pick(&[0u8, 0x40, 0x80, 0xc0, 0x3f]);
        t.put(rec + 0x22, flags);
        for cell in [aeons, party] {
            let n = t.rng.pick(&[0u32, 1, 3, 8, 33, 0xffff_ffff]);
            t.put(cell, n);
        }
        let cmd = t.rng.next();
        t.twin(
            "MenuAbility_CommandTargetMask",
            0x008c_6bc0,
            &[cmd],
            U32,
            |o| {
                w(tk::menu_ability_command_target_mask(
                    o,
                    cmd,
                    local,
                    &mut callback,
                ))
            },
        );
    }

    // MenuItem_SortInventory: the menu list scrambled
    let count = return_cell(&mut t, 0x008c_8c30);
    record_far(&mut t, 0x008c_8e60, 1, r);
    let list = t.sym(0x0159_7770);
    for _ in 0..200 {
        let n = t.rng.pick(&[0u32, 1, 5, 0x70, 0x100, 0xffff_ffff]);
        t.put(count, n);
        for k in 0..0x100 {
            let id = t
                .rng
                .pick(&[
                    0x2000u16, 0x2001, 0x2010, 0x206f, 0x2070, 0x20ff, 0x2100, 0x0005, 0xffff,
                ])
                .wrapping_add(u16::try_from(t.rng.below(4)).unwrap());
            t.put(list + 4 * k, id);
            let c = t.rng.pick(&[0u8, 1, 99, 0xff]);
            t.put(list + 4 * k + 2, c);
        }
        t.twin("MenuItem_SortInventory", 0x008c_8fe0, &[], VOID, |o| {
            tk::menu_item_sort_inventory(o, local, &mut callback);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

/// `f` replaced by `inc dword [cell]; mov eax, ret; ret` (behind a jump);
/// returns the cell (the call count).
fn counter_stub(t: &mut Twins, f: u32, ret: u32) -> u32 {
    let cell = t.alloc(&[0; 4]);
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0xff, 0x05][..], &cell.to_le_bytes(),     // inc dword [cell]
        &[0xb8], &ret.to_le_bytes(), &[0xc3],       // mov eax, ret; ret
    ]
    .concat();
    jump_stub(t, f, &code);
    cell
}

#[test]
fn menu_round24_item_rows_and_name_entry_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0024_33f0) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let r = t.rng.next();

    // MenuItem_ItemTargetMask: the item record, the lists and
    // MenuItem_TargetNeedsRecovery stand-ins logged
    let rec = t.alloc(&[0; 0x40]);
    let (_, mut code) = log_args(&mut t, 1);
    code.extend([&[0xb8][..], &rec.to_le_bytes(), &[0xc3]].concat());
    jump_stub(&mut t, 0x008a_b280, &code);
    let lang = return_cell(&mut t, 0x008a_c2f0);
    let aeons = return_cell(&mut t, 0x008a_9bf0);
    let party = return_cell(&mut t, 0x008a_9b30);
    for (f, k) in [(0x008a_9bc0u32, 0x20u8), (0x008a_9ad0, 0x10)] {
        stub(
            &mut t,
            f,
            &[
                0x8b, 0x44, 0x24, 0x04, 0x8d, 0x04, 0x40, 0x83, 0xc0, k, 0xc3,
            ],
        );
    }
    let (_, mut code) = log_args(&mut t, 3);
    #[rustfmt::skip]
    code.extend([
        0x8b, 0x44, 0x24, 0x0c, 0xc1, 0xe8, 0x05,       // mov eax, [esp + 12]; shr eax, 5
        0x03, 0x44, 0x24, 0x0c, 0x03, 0x44, 0x24, 0x04, // add eax, [esp + 12]; add eax, [esp + 4]
        0x33, 0x44, 0x24, 0x08, 0xc1, 0xe8, 0x01,       // xor eax, [esp + 8]; shr eax, 1
        0x83, 0xe0, 0x01, 0xc3,                         // and eax, 1; ret
    ]);
    jump_stub(&mut t, 0x008c_6a70, &code);
    for _ in 0..1000 {
        let kind = t.rng.pick(&[0u8, 1, 2, 3, 4, 0xff]);
        t.put(rec + 0x23, kind);
        let targets = t.rng.bytes(2);
        t.put_bytes(rec + 0x1a, &targets);
        let flags = t
            .rng
            .pick(&[0u8, 0x40, 0x80, 0xc0, 0x10, 0x50, 0x90, 0xd0, 0x2f]);
        t.put(rec + 0x22, flags);
        let l = t.rng.pick(&[0u32, 1, 10, 11, 12]);
        t.put(lang, l);
        for cell in [aeons, party] {
            let n = t.rng.pick(&[0u32, 1, 3, 8, 17, 33, 0xffff_ffff]);
            t.put(cell, n);
        }
        let item = t.rng.next();
        t.twin("MenuItem_ItemTargetMask", 0x008c_8a30, &[item], U32, |o| {
            w(tk::menu_item_item_target_mask(
                o,
                item,
                local,
                &mut callback,
            ))
        });
    }

    // MenuItem_DrawRow: the count from id * 3, the width from a cell
    for (f, n) in [(0x008f_5f70u32, 5u8), (0x0090_5550, 6), (0x008e_6700, 4)] {
        record_far(&mut t, f, n, r);
    }
    let (_, mut code) = log_args(&mut t, 1);
    code.extend([0x8b, 0x44, 0x24, 0x04, 0x8d, 0x04, 0x40, 0xc3]); // mov eax, [esp + 4]; lea eax, [eax + eax * 2]; ret
    jump_stub(&mut t, 0x0079_04b0, &code);
    let width = t.alloc(&[0; 4]);
    let buf = t.alloc(&[0; 0x4000]);
    let log = t.alloc(&buf.to_le_bytes());
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &log.to_le_bytes()].concat();
    for k in [0u8, 2, 3] {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    #[rustfmt::skip]
    code.extend([
        &[0x83, 0x05][..], &log.to_le_bytes(), &[16],  // add dword [log], 16
        &[0x8b, 0x44, 0x24, 0x08],                     // mov eax, [esp + 8]
        &[0x8b, 0x0d], &width.to_le_bytes(),           // mov ecx, [width]
        &[0x89, 0x08, 0xc3],                           // mov [eax], ecx; ret
    ]
    .concat());
    jump_stub(&mut t, 0x0090_5230, &code);
    let list = t.sym(0x0159_7770);
    for _ in 0..1000 {
        let window = t.alloc_random(0x40);
        let rows = t.rng.pick(&[0u16, 1, 5, 0x100]);
        t.put(window + 0x30, rows);
        let row = t.rng.pick(&[0u32, 1, 4, 5, 0xff, 0xffff_ffff]);
        if row < 0x100 {
            let e = t.rng.bytes(4);
            t.put_bytes(list + 4 * row, &e);
            let on = t.rng.pick(&[0u8, 1, 0x80]);
            t.put(list + 4 * row + 3, on);
        }
        let wd = quiet_float(&mut t);
        t.put(width, wd);
        let (x, y) = (quiet_float(&mut t), t.rng.float(-100.0, 600.0).to_bits());
        t.twin(
            "MenuItem_DrawRow",
            0x008d_af50,
            &[window, x, y, row],
            VOID,
            |o| {
                tk::menu_item_draw_row(
                    o,
                    window,
                    [f32::from_bits(x), f32::from_bits(y)],
                    i(row),
                    local,
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
    }

    // NameInputChrChange: every callee a stand-in
    let mode = return_cell(&mut t, 0x008a_b240);
    let r2 = t.rng.next();
    for (f, n, ret) in [
        (0x008b_edd0u32, 1u8, r),
        (0x008c_43f0, 1, r2),
        (0x008a_c850, 1, r ^ 0x55),
        (0x008c_5280, 1, r2 ^ 0x77),
    ] {
        record_far(&mut t, f, n, ret);
    }
    let english = counter_stub(&mut t, 0x0078_51d0, 0);
    let counts = [
        counter_stub(&mut t, 0x008c_5050, 0),
        counter_stub(&mut t, 0x008d_4920, 0),
        counter_stub(&mut t, 0x008c_4450, r ^ 0x99),
    ];
    let _ = (english, counts);
    let window = t.alloc_random(0x80);
    t.put(t.sym(0x0186_a278), window);
    for _ in 0..1000 {
        let m = t.rng.pick(&[0u32, 0x8_0000, 0xfff7_ffff, 0xffff_ffff]);
        t.put(mode, m);
        let l = t.rng.pick(&[0u32, 0, 1, 11]);
        t.put(lang, l);
        let open = t.rng.pick(&[0u32, 0, 1, 0x1234_5678]);
        t.put(window + 0x58, open);
        let cur = t.rng.pick(&[0u32, 1, 7, 8, 9, 0x11, 0x12, 0xffff_ffff]);
        t.put(t.sym(0x0186_a258), cur);
        t.scramble(0x0186_a25c, 0x20);
        t.put(t.sym(0x0186_a278), window);
        t.scramble(0x023c_c3c4, 8);
        let chr = t.rng.pick(&[0u32, 1, 7, 8, 9, 0x11, 0x12, 0xffff_ffff]);
        let mask = if chr > 0x11 || chr == cur { VOID } else { U32 };
        t.twin("NameInputChrChange", 0x008c_33f0, &[chr], mask, |o| {
            w(tk::name_input_chr_change(o, chr, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// Code that appends `id` and its `n` word arguments to the shared log
/// at `[log]`, then `extra` (which may read the arguments), then
/// `mov eax, [cell]; ret`.
fn seq_code(log: u32, id: u32, n: u8, cell: u32, extra: &[u8]) -> Vec<u8> {
    let args: Vec<u8> = (0..n).collect();
    seq_code_of(log, id, &args, cell, extra)
}

/// [`seq_code`] logging only the arguments `args` (indices).
fn seq_code_of(log: u32, id: u32, args: &[u8], cell: u32, extra: &[u8]) -> Vec<u8> {
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat(); // mov ecx, [log]
    code.extend([&[0xc7, 0x01][..], &id.to_le_bytes()].concat()); // mov dword [ecx], id
    let mut at = 4u8;
    for &k in args {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, at]);
        at += 4;
    }
    code.extend([&[0x83, 0x05][..], &l, &[at]].concat()); // add dword [log], 4 + 4n
    code.extend(extra);
    code.extend([&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat()); // mov eax, [cell]; ret
    code
}

/// A shared call log: the cell holding the write pointer.
fn seq_log(t: &mut Twins) -> u32 {
    let buf = t.alloc(&vec![0; 0x4_0000]);
    t.alloc(&buf.to_le_bytes())
}

/// `f` replaced (behind a jump) by [`seq_code`]; returns the result cell.
fn seq_stub(t: &mut Twins, f: u32, log: u32, n: u8, extra: &[u8]) -> u32 {
    let cell = t.alloc(&[0; 4]);
    let code = seq_code(log, f, n, cell, extra);
    jump_stub(t, f, &code);
    cell
}

/// Fresh random results in `cells`.
fn reroll(t: &mut Twins, cells: &[u32]) {
    for &c in cells {
        let v = t.rng.next();
        t.put(c, v);
    }
}

#[test]
fn menu_round25_rows_pads_draw_cursor_and_count_plate_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0025_aea0) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let log = seq_log(&mut t);
    // every call logs from the start (each call's log is compared on its own)
    let start = Mem::read::<u32>(&t.a, log);

    // MenuItem_DrawRowPairs: the rows logged
    seq_stub(&mut t, 0x008d_af50, log, 4, &[]);
    for _ in 0..300 {
        t.put(log, start);
        let window = t.rng.next();
        let pair = t
            .rng
            .pick_or_any(&[0, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff]);
        let (x, y) = (quiet_float(&mut t), quiet_float(&mut t));
        let args = [window, t.rng.next(), pair, x, y];
        t.twin("MenuItem_DrawRowPairs", 0x008d_aea0, &args, VOID, |o| {
            tk::menu_item_draw_row_pairs(
                o,
                window,
                pair,
                [f32::from_bits(x), f32::from_bits(y)],
                &mut ui_remap,
                &mut callback,
            );
            0
        });
    }

    // Pad_UpdateFrame: the work from a cell, TkGetControlerFrame the
    // original, the repeats from their arguments, the reset logged
    let work_cell = seq_stub(&mut t, 0x0088_8ec0, log, 2, &[]);
    #[rustfmt::skip]
    let mix = [
        0x8b, 0x44, 0x24, 0x0c, 0x33, 0x44, 0x24, 0x10, // mov eax, [esp + 12]; xor eax, [esp + 16]
        0x03, 0x44, 0x24, 0x14, 0xc1, 0xc0, 0x05,       // add eax, [esp + 20]; rol eax, 5
        0x50, 0x58,                                     // push eax; pop eax
    ];
    let repeat_cell = t.alloc(&[0; 4]);
    let mut code = seq_code(log, 0x0088_9940, 5, repeat_cell, &mix);
    // the mix is the result: drop the cell load
    code.truncate(code.len() - 6);
    code.push(0xc3);
    jump_stub(&mut t, 0x0088_9940, &code);
    let reset_cell = seq_stub(&mut t, 0x0088_8970, log, 1, &[]);
    for _ in 0..1000 {
        t.put(log, start);
        let work = t.alloc_random(0x100);
        t.put(work_cell, work);
        let mode = t.rng.pick(&[0u8, 1, 1, 2, 2, 3, 0xff]);
        t.put(work + 0x81, mode);
        let keep = t.rng.pick(&[0u8, 0, 1, 0x80]);
        t.put(work + 0x9e, keep);
        reroll(&mut t, &[reset_cell]);
        let (a, b) = (t.rng.next(), t.rng.next());
        t.twin("Pad_UpdateFrame", 0x0088_9790, &[a, b], U32, |o| {
            w(tk::pad_update_frame(o, a, b, &mut callback))
        });
    }

    // TkMenuDrawAll: every callee, the slots' callbacks and the task logged
    let mut cells = Vec::new();
    for (f, n) in [
        (0x008d_ce30u32, 0u8),
        (0x008d_d440, 0),
        (0x008f_5c60, 2),
        (0x008d_ce50, 0),
        (0x0088_ec60, 0),
        (0x008a_9690, 1),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let abmap = seq_stub(&mut t, 0x0064_1060, log, 0, &[]);
    let mut callbacks = Vec::new();
    for k in 0..4u32 {
        let cell = t.alloc(&[0; 4]);
        // the task (3) takes no argument
        let code = seq_code(log, 0x100 + k, u8::from(k < 3), cell, &[]);
        callbacks.push(test_code(&mut t, &code));
    }
    let task_fn = callbacks[3];
    let task = t.alloc(&task_fn.to_le_bytes());
    let slots = t.sym(tk::MENU_SLOTS.sym);
    for round in 0..600u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let ab = t.rng.pick(&[0u32, 1, 1, 2, 0xffff_ffff]);
        t.put(abmap, ab);
        let first = t.rng.pick(&[0u8, 0, 1, 0xff]);
        t.put(t.sym(0x0134_0859), first);
        let pkt = t.rng.pick_or_any(&[0, 0, 1]);
        t.put(t.sym(0x0187_1538), pkt);
        let map = t.rng.pick(&[0x17eu32, 0x17e, 0x17f, 0, 0x1_017e]);
        t.put(t.sym(0x0133_0844), map);
        t.scramble(0x025d_1678, 8);
        let mode = t.rng.pick(&[
            0x0040_0000u32,
            0x0040_ffff,
            0x0041_0000,
            0x00c0_0000,
            0x0140_0000,
            0,
        ]);
        let mode = mode ^ (t.rng.next() & 0xffff);
        t.put(t.sym(tk::MN2_MENU_MODE.sym), mode);
        let mask = t.rng.pick_or_any(&[0, 0xffff_ffff, 0x0100_0001]);
        t.put(t.sym(0x0184_08f0), mask);
        for k in 0..25u32 {
            let menu = if t.rng.below(4) == 0 {
                0
            } else {
                let m = t.alloc_random(0x20);
                let cb = t.rng.pick(&[0, callbacks[0], callbacks[1], callbacks[2]]);
                t.put(m + 4, cb);
                for o in [0x19u32, 0x1b] {
                    let on = t.rng.pick(&[0u8, 0, 1, 0x80]);
                    t.put(m + o, on);
                }
                m
            };
            t.put(slots + 4 * k, menu);
        }
        let main = if round % 3 == 0 { 0 } else { task };
        t.put(t.sym(tk::MENU_MAIN_TASK.sym), main);
        let early = first == 0 && pkt != 0 && map != 0x17e;
        let mask = if early { VOID } else { U32 };
        t.twin("TkMenuDrawAll", 0x008a_a290, &[], mask, |o| {
            w(tk::tk_menu_draw_all(o, &mut callback))
        });
    }

    // MenuSelList_CursorX: names logged, the widths from a table in turn
    let summon = seq_stub(&mut t, 0x0087_3b50, log, 1, &[]);
    let mes = seq_stub(&mut t, 0x008d_dfc0, log, 1, &[]);
    let widths = t.alloc(&[0; 0x100]);
    let wptr = t.alloc(&widths.to_le_bytes());
    let p = wptr.to_le_bytes();
    #[rustfmt::skip]
    let next_width: Vec<u8> = [
        &[0x8b, 0x0d][..], &p,                      // mov ecx, [wptr]
        &[0x8b, 0x01],                              // mov eax, [ecx]
        &[0x83, 0x05], &p, &[0x04],                 // add dword [wptr], 4
        &[0x8b, 0x54, 0x24, 0x08, 0x89, 0x02],      // mov edx, [esp + 8]; mov [edx], eax
    ]
    .concat();
    // the out pointer (argument 1) is the original's argument slot: not logged
    let width_cell = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x0090_5290, &[0, 2, 3, 4], width_cell, &next_width);
    jump_stub(&mut t, 0x0090_5290, &code);
    let list = t.sym(tk::MENU_ITEM_LIST.sym);
    for _ in 0..1000 {
        t.put(log, start);
        reroll(&mut t, &[summon, mes, width_cell]);
        for k in 0..64u32 {
            let v = match t.rng.below(6) {
                0 => quiet_float(&mut t),
                1 => f32::NAN.to_bits(),
                _ => t.rng.float(-50.0, 900.0).to_bits(),
            };
            t.put(widths + 4 * k, v);
        }
        t.put(wptr, widths);
        let window = t.alloc_random(0x50);
        let n = t.rng.pick(&[0u16, 1, 2, 5, 20, 40, 0x101]);
        t.put(window + 0x30, n);
        for k in 0..u32::from(n) {
            let id = u16::try_from(t.rng.next() & 0xffff).unwrap_or(0);
            let kind = t.rng.pick(&[0u8, 1, 2, 2, 3, 3, 3, 4, 0xff]);
            let kind = if kind == 1 && t.rng.below(3) != 0 {
                2
            } else {
                kind
            };
            let on = t.rng.pick(&[0u8, 1, 1, 0x80]);
            t.put(list + 4 * k, id);
            t.put(list + 4 * k + 2, kind);
            t.put(list + 4 * k + 3, on);
        }
        t.twin_f("MenuSelList_CursorX", 0x008d_1bc0, &[window], |o| {
            tk::menu_sel_list_cursor_x(o, window, local, &mut ui_remap, &mut callback)
        });
    }

    // TkMn2DrawCrossSummonBattleCountPlate: every callee logged
    let player = seq_stub(&mut t, 0x0078_5330, log, 1, &[]);
    let mut cells = vec![player];
    for (f, n) in [
        (0x0090_3ee0u32, 11u8),
        (0x008d_e140, 1),
        (0x0090_16b0, 7),
        (0x008c_2c60, 3),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    for _ in 0..600 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let p = t.alloc_random(0x40);
        t.put(player, p);
        let (x, y) = (quiet_float(&mut t), quiet_float(&mut t));
        let chr = t.rng.next();
        t.twin(
            "TkMn2DrawCrossSummonBattleCountPlate",
            0x008c_d720,
            &[x, y, chr],
            U32,
            |o| {
                w(tk::tk_mn2_draw_cross_summon_battle_count_plate(
                    o,
                    [f32::from_bits(x), f32::from_bits(y)],
                    chr,
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round25_gear_sort_shop_rows_and_grow_rows_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0025_9fb0) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let list = t.sym(tk::MENU_ITEM_LIST.sym);

    // MsGetSaveWeapon(id, &out): the id logged (not the out pointer, a
    // frame word), the record at table + (id & 0xff) * 8, owner byte +4
    let table = t.alloc_random(0x800);
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = Vec::new();
    #[rustfmt::skip]
    code.extend([
        &[0x8b, 0x0d][..], &l,                          // mov ecx, [log]
        &[0xc7, 0x01], &0x007a_bbd0u32.to_le_bytes(),   // mov dword [ecx], id
        &[0x8b, 0x44, 0x24, 0x04, 0x89, 0x41, 0x04],    // [ecx + 4] = arg 0
        &[0x83, 0x05], &l, &[0x08],                     // add dword [log], 8
        &[0x8b, 0x54, 0x24, 0x08],                      // mov edx, [esp + 8]
        &[0xc7, 0x02, 0x0b, 0x0b, 0x0b, 0x0b],          // mov dword [edx], 0x0b0b0b0b (the out word)
        &[0x8b, 0x44, 0x24, 0x04, 0x25, 0xff, 0, 0, 0], // mov eax, [esp + 4]; and eax, 0xff
        &[0x8d, 0x04, 0xc5], &table.to_le_bytes(),      // lea eax, [table + eax * 8]
        &[0xc3],
    ]
    .concat());
    jump_stub(&mut t, 0x007a_bbd0, &code);

    // MenuItem_SortGearByOwner: groups that fit the list, owners that are
    // found or not, Menu_FindByte the original, the swaps logged
    let n_cell = seq_stub(&mut t, 0x008c_1bc0, log, 0, &[]);
    seq_stub(&mut t, 0x007a_b9f0, log, 2, &[]);
    let counts = t.sym(0x0186_a3e4);
    for round in 0..1500u32 {
        t.put(log, start);
        let n = t.rng.pick(&[0u32, 1, 3, 8, 20, 40, 64]);
        let n = if round % 50 == 0 { 0xffff_ffff } else { n };
        t.put(n_cell, n);
        let mut left = if n > 0x100 { 0 } else { n };
        let mut sizes = [0u32; 7];
        for k in 0..14u32 {
            let c = if left == 0 {
                0
            } else {
                t.rng.below(left.min(9) + 1)
            };
            left -= c;
            sizes[(k / 2) as usize] += c;
            t.put(counts + 2 * k, u16::try_from(c).unwrap_or(0));
        }
        if round % 2 == 0 {
            // ids 0.. with owners that fill the groups, shuffled: the sort
            // finds every one it needs
            let mut owners: Vec<u8> = (0..7u8)
                .flat_map(|o| std::iter::repeat_n(o, sizes[o as usize] as usize))
                .collect();
            while owners.len() < n.min(0x100) as usize {
                owners.push(t.rng.pick(&[0u8, 3, 6, 7]));
            }
            for k in (1..owners.len()).rev() {
                let j = t.rng.below(u32::try_from(k).unwrap_or(0) + 1) as usize;
                owners.swap(k, j);
            }
            for (k, &o) in (0u32..).zip(&owners) {
                t.put(table + 8 * k + 4, o);
                t.put(list + 4 * k, u16::try_from(k).unwrap_or(0));
            }
        } else {
            for k in 0..0x100u32 {
                let o = match t.rng.below(10) {
                    0 => t.rng.pick(&[7u8, 0xff, 0x80]),
                    _ => u8::try_from(t.rng.below(7)).unwrap_or(0),
                };
                t.put(table + 8 * k + 4, o);
            }
            let ids = t.rng.bytes(4 * 64);
            t.put_bytes(list, &ids);
        }
        t.twin("MenuItem_SortGearByOwner", 0x008c_9fb0, &[], U32, |o| {
            w(tk::menu_item_sort_gear_by_owner(
                o,
                local,
                local + 0x400,
                &mut callback,
            ))
        });
    }

    // the shop rows: every callee logged
    let mut cells = Vec::new();
    for (f, n) in [
        (0x008d_b3b0u32, 1u8),
        (0x008f_5f70, 5),
        (0x0090_3ee0, 11),
        (0x0090_55c0, 6),
        (0x008e_6700, 4),
        (0x008d_9170, 1),
        (0x008d_9370, 1),
        (0x008e_6970, 4),
        (0x008d_9410, 1),
        (0x008b_eeb0, 1),
        (0x0090_5ab0, 6),
        (0x008a_9880, 0),
        (0x0078_5330, 1),
        (0x008c_eca0, 3),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let can = [
        seq_stub(&mut t, 0x008d_ab30, log, 1, &[]),
        seq_stub(&mut t, 0x008d_85d0, log, 1, &[]),
    ];
    t.scramble(0x00c5_a608, 0x1c * 40);
    for round in 0..3000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        for c in can {
            let v = t.rng.pick_or_any(&[0, 0, 1]);
            t.put(c, v);
        }
        let window = t.alloc_random(0x50);
        let count = t.rng.pick(&[0u16, 1, 5, 12, 0x100, 0xffff]);
        t.put(window + 0x30, count);
        let row = t
            .rng
            .pick_or_any(&[0, 1, 4, 5, 11, 12, 31, 32, 0xff, 0xffff_ffff, 0x8000_0000]);
        let row = if row > 0x1000 && row < 0x8000_0000 {
            row & 0x3f
        } else {
            row
        };
        let ids = t.alloc_random(0x200);
        let (x, y) = (quiet_float(&mut t), quiet_float(&mut t));
        let mask = if i(row) < 0 { VOID } else { U32 };
        let (fx, fy) = (f32::from_bits(x), f32::from_bits(y));
        match round % 4 {
            0 => t.twin(
                "MenuShop_DrawRow",
                0x008d_ac70,
                &[window, x, y, row, ids],
                mask,
                |o| {
                    w(tk::menu_shop_draw_row(
                        o,
                        window,
                        [fx, fy],
                        i(row),
                        ids,
                        &mut ui_remap,
                        &mut callback,
                    ))
                },
            ),
            1 => t.twin(
                "MenuWShop_DrawBuyRow",
                0x008d_8cb0,
                &[window, x, y, row, ids],
                mask,
                |o| {
                    w(tk::menu_wshop_draw_buy_row(
                        o,
                        window,
                        [fx, fy],
                        i(row),
                        ids,
                        &mut ui_remap,
                        &mut callback,
                    ))
                },
            ),
            2 => {
                if row < 0x200 {
                    let e = t.rng.bytes(4);
                    t.put_bytes(list + 4 * row, &e);
                    let id = t.rng.pick(&[0u16, 0, 1, 0x1234]);
                    if id != 0x1234 {
                        t.put(list + 4 * row, id);
                    }
                    let on = t.rng.pick(&[0u8, 1, 0x80]);
                    t.put(list + 4 * row + 3, on);
                }
                t.twin(
                    "MenuWShop_DrawSellRow",
                    0x008d_8f60,
                    &[window, x, y, row],
                    mask,
                    |o| {
                        w(tk::menu_wshop_draw_sell_row(
                            o,
                            window,
                            [fx, fy],
                            i(row),
                            local,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
            _ => {
                t.scramble(0x0186_a600, 4);
                let fixed = t.rng.pick_or_any(&[0, 0, 1]);
                let mask = if i(row) < 0 || i(row) >= i32::from(count) {
                    VOID
                } else {
                    mask
                };
                t.twin(
                    "MenuSummon_DrawGrowRow",
                    0x008c_e5c0,
                    &[window, x, y, row, fixed],
                    mask,
                    |o| {
                        w(tk::menu_summon_draw_grow_row(
                            o,
                            window,
                            [fx, fy],
                            i(row),
                            fixed,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

/// A row loop's port, for the table of three copies.
type Rows = fn(
    &mut Oracle,
    u32,
    u32,
    [f32; 2],
    u32,
    &mut fn(&mut Oracle, bool, f32) -> f64,
    &mut fn(&mut Oracle, u32, &[u32]) -> u32,
);

#[test]
fn menu_round25_scan_row_loops_and_cursor_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0025_93e0) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);

    // TkScanControler: the busy thunk, the works and the updates logged
    let mut cells = vec![
        seq_stub(&mut t, 0x0082_2560, log, 0, &[]),
        seq_stub(&mut t, 0x0088_8e10, log, 2, &[]),
    ];
    let update = seq_stub(&mut t, 0x0088_9790, log, 3, &[]);
    cells.push(update);
    for _ in 0..200 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.twin("TkScanControler", 0x0088_93e0, &[], U32, |o| {
            w(tk::tk_scan_controler(o, &mut callback))
        });
    }

    // the row loops: each row logged
    for f in [0x008d_8cb0u32, 0x008d_8f60, 0x008d_ac70, 0x008c_e5c0] {
        seq_stub(&mut t, f, log, 5, &[]);
    }
    let loops: [(&str, u32, Rows); 3] = [
        (
            "MenuWShop_DrawBuyRows",
            0x008d_8c30,
            |o, w, f, xy, ids, r, c| {
                tk::menu_wshop_draw_buy_rows(o, w, f, xy, ids, r, c);
            },
        ),
        (
            "MenuWShop_DrawSellRows",
            0x008d_8ee0,
            |o, w, f, xy, ids, r, c| {
                tk::menu_wshop_draw_sell_rows(o, w, f, xy, ids, r, c);
            },
        ),
        (
            "MenuShop_DrawRows",
            0x008d_abf0,
            |o, w, f, xy, ids, r, c| {
                tk::menu_shop_draw_rows(o, w, f, xy, ids, r, c);
            },
        ),
    ];
    for round in 0..600u32 {
        t.put(log, start);
        let (what, f, port) = loops[(round % 3) as usize];
        let (window, ids) = (t.rng.next(), t.rng.next());
        let first = t.rng.pick_or_any(&[0, 1, 0x7fff_fffc, 0xffff_fffe]);
        let (x, y) = (quiet_float(&mut t), quiet_float(&mut t));
        let args = [window, t.rng.next(), first, x, y, ids];
        let xy = [f32::from_bits(x), f32::from_bits(y)];
        t.twin(what, f, &args, VOID, |o| {
            let mut r: fn(&mut Oracle, bool, f32) -> f64 = ui_remap;
            let mut c: fn(&mut Oracle, u32, &[u32]) -> u32 = callback;
            port(o, window, first, xy, ids, &mut r, &mut c);
            0
        });
    }

    // MenuSummon_DrawGrowRows: integer x, y
    for _ in 0..300 {
        t.put(log, start);
        let (window, fixed) = (t.rng.next(), t.rng.pick_or_any(&[0, 1]));
        let first = t.rng.pick_or_any(&[0, 1, 0x8000_0000]);
        let (x, y) = (t.rng.int(), t.rng.int());
        t.twin(
            "MenuSummon_DrawGrowRows",
            0x008c_e520,
            &[window, fixed, first, x, y],
            VOID,
            |o| {
                tk::menu_summon_draw_grow_rows(
                    o,
                    window,
                    fixed,
                    first,
                    [i(x), i(y)],
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
    }

    // MenuSelList_DrawCursor: CursorX logged and leaving a float in ST0
    let xcell = t.alloc(&[0; 4]);
    let fld = [&[0xd9, 0x05][..], &xcell.to_le_bytes()].concat(); // fld dword [xcell]
    let xret = seq_stub(&mut t, 0x008d_1bc0, log, 1, &fld);
    let cursor = seq_stub(&mut t, 0x008c_0670, log, 3, &[]);
    for _ in 0..600 {
        t.put(log, start);
        reroll(&mut t, &[xret, cursor]);
        let v = quiet_float(&mut t);
        t.put(xcell, v);
        let window = t.alloc_random(0x50);
        for o in [0x48u32, 0x32] {
            let v = t
                .rng
                .pick_or_any(&[0, 1, 2, 3, 0x7fff, 0x8000, 0xffff, 0xfffe]);
            t.put(window + o, u16::try_from(v & 0xffff).unwrap_or(0));
        }
        t.twin("MenuSelList_DrawCursor", 0x008d_15a0, &[window], U32, |o| {
            w(tk::menu_sel_list_draw_cursor(
                o,
                window,
                &mut ui_remap,
                &mut |o, w| {
                    let at = o.rebase(0x008d_1bc0);
                    o.call(at, Conv::Cdecl, &[w]).unwrap().st0.unwrap()
                },
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// [`seq_code`] for a callee taking a pointer: `id` and the `n` words at
/// `[esp + 4]` logged (the pointer itself is a frame address).
fn seq_code_deref(log: u32, id: u32, n: u8, cell: u32) -> Vec<u8> {
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat(); // mov ecx, [log]
    code.extend([&[0xc7, 0x01][..], &id.to_le_bytes()].concat()); // mov dword [ecx], id
    code.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp + 4]
    let mut at = 4u8;
    for k in 0..n {
        code.extend([0x8b, 0x42, 4 * k, 0x89, 0x41, at]); // mov eax, [edx + 4k]; mov [ecx + at], eax
        at += 4;
    }
    code.extend([&[0x83, 0x05][..], &l, &[at]].concat()); // add dword [log], 4 + 4n
    code.extend([&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat()); // mov eax, [cell]; ret
    code
}

/// `ToGetBtlEasyFontWidth(text, out, 0, s0, s1)` logged without `out` (a
/// frame address), writing the word in the returned cell to `[out]`.
fn width_stub(t: &mut Twins, log: u32) -> u32 {
    let width = t.alloc(&[0; 4]);
    #[rustfmt::skip]
    let extra: Vec<u8> = [
        &[0x8b, 0x54, 0x24, 0x08][..],              // mov edx, [esp + 8]
        &[0xa1], &width.to_le_bytes(),              // mov eax, [width]
        &[0x89, 0x02],                              // mov [edx], eax
    ]
    .concat();
    let cell = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x0090_5290, &[0, 2, 3, 4], cell, &extra);
    jump_stub(t, 0x0090_5290, &code);
    width
}

/// An `i32` argument: small, near the edges or anything.
fn status_int(t: &mut Twins) -> i32 {
    let v = match t.rng.below(6) {
        0 => t.rng.next(),
        1 => t.rng.pick(&[
            0u32,
            0x7fff_ffff,
            0x8000_0000,
            0xffff_ffff,
            0x7fff,
            0xffff_8000,
        ]),
        _ => t.rng.below(1200).wrapping_sub(200),
    };
    v.cast_signed()
}

#[test]
fn menu_round26_status_pages_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0026_4ee0) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);

    // the callees of the five drawers, logged
    let mut cells = Vec::new();
    for (f, n) in [
        (0x008f_5f70u32, 5u8),
        (0x0090_3ee0, 11),
        (0x008a_9950, 1),
        (0x008d_3690, 6),
        (0x008c_1bd0, 1),
        (0x008d_dfc0, 1),
        (0x0090_5ab0, 6),
        (0x008b_ee70, 1),
        (0x008a_9c70, 1),
        (0x008e_67f0, 4),
        (0x008c_1280, 6),
        (0x008c_1a20, 1),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let limit = seq_stub(&mut t, 0x008a_9930, log, 1, &[]);
    let armour = seq_stub(&mut t, 0x008a_9820, log, 1, &[]);
    let yes_no = seq_stub(&mut t, 0x008a_9b60, log, 0, &[]);
    // TOMenuDrawChrLayoutPlate: the result mixes in the draw cursor global
    let layout = t.alloc(&[0; 4]);
    let mut code = seq_code(log, 0x008f_0a70, 4, layout, &[]);
    code.truncate(code.len() - 6);
    let draw_y = t.sym(0x0186_a238).to_le_bytes();
    #[rustfmt::skip]
    code.extend([
        &[0xa1][..], &draw_y,                       // mov eax, [g_TkMn2DrawY]
        &[0x33, 0x05], &layout.to_le_bytes(),       // xor eax, [layout]
        &[0xc3],                                    // ret
    ].concat());
    jump_stub(&mut t, 0x008f_0a70, &code);
    let plate_cell = t.alloc(&[0; 4]);
    let code = seq_code_deref(log, 0x008b_f9b0, 6, plate_cell);
    jump_stub(&mut t, 0x008b_f9b0, &code);
    let width = width_stub(&mut t, log);
    cells.extend([limit, layout, plate_cell]);

    // TkMn2DrawOverDrive: both branches, MenuStatus_OffsetRect the original
    for _ in 0..600 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble(0x0186_a888, 0x28);
        let (x, y) = (status_int(&mut t), status_int(&mut t));
        let chr = t
            .rng
            .pick(&[
                0u32,
                1,
                7,
                8,
                9,
                0x12,
                0x7fff_ffff,
                0x8000_0000,
                0xffff_ffff,
            ])
            .cast_signed();
        let args = [x, y, chr].map(i32::cast_unsigned);
        t.twin("TkMn2DrawOverDrive", 0x008d_3d70, &args, U32, |o| {
            w(tk::tk_mn2_draw_over_drive(
                o,
                [x, y],
                chr,
                local,
                &mut ui_remap,
                &mut callback,
            ))
        });
    }

    // MenuStatus_DrawEquipPlate: armour 0xff or not (the whole word)
    for _ in 0..600 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let a = t.rng.pick(&[0xffu32, 0xff, 0, 0x1ff, 0xffff_ffff, 7]);
        t.put(armour, a);
        let wv = quiet_float(&mut t);
        t.put(width, wv);
        let chr = t.rng.next();
        let x = status_int(&mut t);
        t.twin(
            "MenuStatus_DrawEquipPlate",
            0x008d_4540,
            &[chr, x.cast_unsigned()],
            U32,
            |o| {
                w(tk::menu_status_draw_equip_plate(
                    o,
                    chr,
                    x,
                    local,
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
    }

    // MenuSummon_DrawCommandRow: the caption by the first count, the id
    for _ in 0..600 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let wv = quiet_float(&mut t);
        t.put(width, wv);
        t.scramble(0x0186_a9f4, 0x40);
        let first = t.rng.pick(&[0u16, 1, 3, 8, 0x7fff, 0x8000, 0xffff]);
        t.put(t.sym(0x0186_aa24), first);
        let (x, y) = (status_int(&mut t), status_int(&mut t));
        let i = t
            .rng
            .pick(&[0u32, 1, 2, 3, 7, 8, 9, 15, 16, 0xffff_ffff, 0xffff_fffc])
            .cast_signed();
        let args = [x, y, i].map(i32::cast_unsigned);
        t.twin("MenuSummon_DrawCommandRow", 0x008d_4350, &args, U32, |o| {
            w(tk::menu_summon_draw_command_row(
                o,
                [x, y],
                i,
                local,
                &mut ui_remap,
                &mut callback,
            ))
        });
    }

    // MenuSummon_DrawCommandRows: the row calls logged
    seq_stub(&mut t, 0x008d_4350, log, 3, &[]);
    for _ in 0..400 {
        t.put(log, start);
        let a = t.rng.pick(&[0u16, 0, 1, 2, 3, 5, 9, 200]);
        let b = t.rng.pick(&[0u16, 0, 1, 2, 4, 7, 100]);
        t.put(t.sym(0x0186_aa24), a);
        t.put(t.sym(0x0186_aa28), b);
        let x = status_int(&mut t);
        let args = [t.rng.next(), x.cast_unsigned()];
        t.twin(
            "MenuSummon_DrawCommandRows",
            0x008d_4280,
            &args,
            VOID,
            |o| {
                tk::menu_summon_draw_command_rows(o, x, &mut ui_remap, &mut callback);
                0
            },
        );
    }

    // MenuStatus_DrawLayoutRow: the rect by the yes/no mask bit
    for _ in 0..600 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let m = t.rng.pick(&[0u32, 0xffff_ffff, 1, 0x8000_0000]) ^ (t.rng.next() & t.rng.next());
        t.put(yes_no, m);
        t.scramble(0x023c_c13c, 0x10);
        t.scramble(0x0186_a238, 4);
        let (x, y) = (status_int(&mut t), status_int(&mut t));
        let chr = t.rng.pick(&[0u32, 1, 7, 8, 31, 32, 33, 0xffff_ffff]);
        let args = [x.cast_unsigned(), y.cast_unsigned(), chr];
        t.twin("MenuStatus_DrawLayoutRow", 0x008c_11e0, &args, U32, |o| {
            w(tk::menu_status_draw_layout_row(
                o,
                [x, y],
                chr,
                &mut callback,
            ))
        });
    }

    // MenuStatus_DrawPage: every page's calls logged
    let ease = seq_stub(&mut t, 0x008d_30c0, log, 2, &[]);
    let mut page_cells = vec![ease];
    for (f, n) in [
        (0x008a_99f0u32, 1u8),
        (0x008a_9860, 0),
        (0x008a_9880, 0),
        (0x008c_2ce0, 1),
        (0x008d_32b0, 7),
        (0x008d_34a0, 7),
        (0x008d_4540, 2),
        (0x008d_3d70, 3),
        (0x008f_f490, 3),
        (0x008d_4280, 2),
        (0x008c_11e0, 3),
        (0x008c_0c40, 0),
    ] {
        page_cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    for _ in 0..1000 {
        t.put(log, start);
        reroll(&mut t, &page_cells);
        let e = t
            .rng
            .pick(&[0u32, 1, 0x1000, 0xfff, 0x8000_0000, 0xffff_ffff])
            ^ (t.rng.next() & t.rng.next());
        t.put(ease, e);
        t.scramble(0x0186_a83c, 4);
        t.scramble(0x0186_a888, 4);
        t.scramble(0x0186_a8a0, 4);
        let faces = [t.alloc_random(4), t.alloc_random(4), t.alloc_random(4)];
        let (x, y) = (status_int(&mut t), status_int(&mut t));
        let chr = t.rng.next();
        let page = t
            .rng
            .pick(&[0u32, 0, 1, 1, 2, 2, 3, 3, 4, 0x8000_0000, 0xffff_ffff]);
        let tt = status_int(&mut t);
        #[rustfmt::skip]
        let args = [x.cast_unsigned(), y.cast_unsigned(), chr, page, faces[0], faces[1], faces[2], t.rng.next(), tt.cast_unsigned()];
        t.twin("MenuStatus_DrawPage", 0x008d_4ee0, &args, U32, |o| {
            w(tk::menu_status_draw_page(
                o,
                [x, y],
                chr,
                page,
                faces,
                tt,
                &mut ui_remap,
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// `f` logged with the argument indices `args`, writing the word in the
/// returned cell to `[esp + 4 + 4 * out]` (a frame address, not logged).
fn out_stub(t: &mut Twins, log: u32, f: u32, args: &[u8], out: u8) -> u32 {
    let value = t.alloc(&[0; 4]);
    #[rustfmt::skip]
    let extra: Vec<u8> = [
        &[0x8b, 0x54, 0x24, 4 + 4 * out][..],       // mov edx, [esp + 4 + 4 out]
        &[0xa1], &value.to_le_bytes(),              // mov eax, [value]
        &[0x89, 0x02],                              // mov [edx], eax
    ]
    .concat();
    let cell = t.alloc(&[0; 4]);
    let code = seq_code_of(log, f, args, cell, &extra);
    jump_stub(t, f, &code);
    value
}

#[test]
fn menu_round26_list_rows_status_plate_and_option_rows_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0026_1820) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = Vec::new();
    for (f, n) in [
        (0x008f_5f70u32, 5u8),
        (0x008f_5000, 4),
        (0x0087_3b50, 1),
        (0x008d_dfc0, 1),
        (0x0090_5ab0, 6),
        (0x0090_3bb0, 9),
        (0x008b_ee70, 1),
        (0x0090_5820, 6),
        (0x0090_3ee0, 11),
        (0x008e_2270, 1),
        (0x008d_df80, 1),
        (0x0090_5970, 7),
        (0x008c_0670, 3),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let player = seq_stub(&mut t, 0x0078_5330, log, 1, &[]);
    let enabled = seq_stub(&mut t, 0x008e_2290, log, 1, &[]);
    let width = out_stub(&mut t, log, 0x0090_5290, &[0, 2, 3, 4], 1);
    let digit_width = out_stub(&mut t, log, 0x0090_5230, &[0, 2, 3], 1);
    let list = t.sym(tk::MENU_ITEM_LIST.sym);

    // MenuSelList_DrawRow: rows in and out of range, every kind
    for _ in 0..1000 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let wv = quiet_float(&mut t);
        t.put(width, wv);
        let window = t.alloc_random(0x50);
        let rows = t.rng.pick(&[0u16, 1, 4, 9, 0x8000, 0xffff]);
        t.put(window + 0x30, rows);
        let scroll = t
            .rng
            .pick(&[0u16, 0x7f, 0x80, 0x1000, 0x7fff, 0x8000, 0xff81, 0xffff])
            ^ u16::try_from(t.rng.next() & t.rng.next() & 0xffff).unwrap_or(0);
        t.put(window + 0x46, scroll);
        let row = t
            .rng
            .pick(&[0u32, 1, 3, 8, 9, 10, 0x7fff, 0xffff_ffff, 0x8000_0000])
            .cast_signed();
        if (0..0x200).contains(&row) {
            let at = list + 4 * row.cast_unsigned();
            let id = u16::try_from(t.rng.next() & 0xffff).unwrap_or(0);
            let kind = t.rng.pick(&[0u8, 1, 2, 3, 3, 2, 1, 4, 0xff]);
            let on = t.rng.pick(&[0u8, 1, 1, 1, 0x80]);
            t.put(at, id);
            t.put(at + 2, kind);
            t.put(at + 3, on);
        }
        let (x, y) = (quiet_float(&mut t), quiet_float(&mut t));
        let fixed = t.rng.pick(&[0u32, 0, 1, 0x100]);
        let args = [window, x, y, row.cast_unsigned(), fixed];
        t.twin("MenuSelList_DrawRow", 0x008d_1820, &args, VOID, |o| {
            #[rustfmt::skip]
            tk::menu_sel_list_draw_row(o, window, [f32::from_bits(x), f32::from_bits(y)], row, fixed != 0, local, &mut ui_remap, &mut callback);
            0
        });
    }

    // TkMn2DrawStatusPlate: the stat byte from the player record
    for _ in 0..600 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let (wv, dv) = (quiet_float(&mut t), quiet_float(&mut t));
        t.put(width, wv);
        t.put(digit_width, dv);
        let p = t.alloc_random(0x100);
        t.put(player, p);
        let (x, y) = (status_int(&mut t), status_int(&mut t));
        let (chr, mes) = (t.rng.next(), t.rng.next());
        let field = t.rng.below(0x100);
        #[rustfmt::skip]
        let args = [x.cast_unsigned(), y.cast_unsigned(), t.rng.next(), t.rng.next(), chr, mes, field];
        t.twin("TkMn2DrawStatusPlate", 0x008c_1490, &args, U32, |o| {
            w(tk::tk_mn2_draw_status_plate(
                o,
                [x, y],
                chr,
                mes,
                field,
                local,
                &mut ui_remap,
                &mut callback,
            ))
        });
    }

    // MenuMain_DrawOptionRow: the fade, the chosen option's cursor
    for _ in 0..1000 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let wv = quiet_float(&mut t);
        t.put(width, wv);
        let e = t.rng.pick(&[0u32, 0, 1, 2, 0xffff_ffff]);
        t.put(enabled, e);
        let index = t
            .rng
            .pick(&[
                0u32,
                1,
                2,
                5,
                7,
                0xffff_ffff,
                0x8000_0000,
                0x7fff_ffff,
                0x0100_0001,
            ])
            .cast_signed();
        let chosen = if t.rng.below(2) == 0 {
            index.cast_unsigned()
        } else {
            t.rng.next()
        };
        t.put(t.sym(0x0187_1548), chosen);
        let fade = t
            .rng
            .pick(&[
                0x1000u32,
                0x1000,
                0,
                0x800,
                0xfff,
                0x1001,
                0xffff_ffe1,
                0x8000_0000,
                0x7fff_ffff,
                0x0100_0001,
            ])
            .cast_signed();
        let x = quiet_float(&mut t);
        #[rustfmt::skip]
        let args = [index.cast_unsigned(), x, t.rng.next(), t.rng.next(), fade.cast_unsigned()];
        t.twin("MenuMain_DrawOptionRow", 0x008e_22a0, &args, U32, |o| {
            w(tk::menu_main_draw_option_row(
                o,
                index,
                f32::from_bits(x),
                fade,
                local,
                &mut ui_remap,
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// Code logging `id`, the `n` words at `[esp + 4]` (a frame buffer; each
/// `& mask`) and then the word arguments `args` (indices), returning
/// `[cell]`.
fn seq_code_deref_args(log: u32, id: u32, n: u8, mask: u32, args: &[u8], cell: u32) -> Vec<u8> {
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat(); // mov ecx, [log]
    code.extend([&[0xc7, 0x01][..], &id.to_le_bytes()].concat()); // mov dword [ecx], id
    code.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp + 4]
    let mut at = 4u8;
    for k in 0..n {
        code.extend([0x8b, 0x42, 4 * k]); // mov eax, [edx + 4k]
        code.extend([&[0x25][..], &mask.to_le_bytes()].concat()); // and eax, mask
        code.extend([0x89, 0x41, at]); // mov [ecx + at], eax
        at += 4;
    }
    for &k in args {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, at]); // mov eax, [esp + 4 + 4k]; mov [ecx + at], eax
        at += 4;
    }
    code.extend([&[0x83, 0x05][..], &l, &[at]].concat()); // add dword [log], at
    code.extend([&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat()); // mov eax, [cell]; ret
    code
}

#[test]
fn menu_round26_pad_analog_and_save_slot_rows_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0026_9570) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);

    // Pad_UpdateAnalog: the stick floats from a stub, the rest original
    let stick = seq_stub(&mut t, 0x0063_0c90, log, 0, &[]);
    // Compute_008889e0 (the pressure of a button): kind and byte logged
    let pressure = seq_stub(&mut t, 0x0088_89e0, log, 2, &[]);
    for _ in 0..2000 {
        t.put(log, start);
        reroll(&mut t, &[pressure]);
        let axes = t.alloc(&[0; 16]);
        for k in 0..4u32 {
            let v = match t.rng.below(4) {
                0 => quiet_float(&mut t),
                _ => t.rng.float(-1.2, 1.2).to_bits(),
            };
            t.put(axes + 4 * k, v);
        }
        t.put(stick, axes);
        let raw = t.alloc_random(0x20);
        let nibble =
            t.rng.pick(&[0u8, 1, 2, 3, 4, 7, 0xf]) | (t.rng.next().to_le_bytes()[0] & 0xf0);
        t.put(raw + 1, nibble);
        let work = t.alloc_random(0xa0);
        let edge = t.rng.pick(&[0u32, 0xffff_ffff, 0xffff_0fff]);
        let buttons = (t.rng.next() & 0xf000) | (t.rng.next() & !0xf000 & edge);
        t.twin(
            "Pad_UpdateAnalog",
            0x0088_9570,
            &[work, raw, buttons],
            VOID,
            |o| {
                tk::pad_update_analog(o, work, raw, buttons, &mut callback);
                0
            },
        );
    }

    // MenuSaveLoad_DrawSlotRow: every callee logged, sprintf the CRT's
    let mut cells = Vec::new();
    for (f, n) in [
        (0x008f_9b60u32, 6u8),
        (0x0090_3a10, 3),
        (0x008f_3db0, 6),
        (0x008b_5560, 2),
        (0x0090_1d10, 5),
        (0x0090_1cb0, 4),
        (0x0086_c430, 2),
        (0x0078_51d0, 0),
        (0x008e_19f0, 4),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let info = seq_stub(&mut t, 0x008b_4e40, log, 1, &[]);
    let lang = seq_stub(&mut t, 0x008a_c2f0, log, 0, &[]);
    let number_cell = t.alloc(&[0; 4]);
    // the digits' first three bytes: "%02d" of row + 1 <= 999 (the bytes
    // past the terminator are each frame's own)
    let code = seq_code_deref_args(log, 0x0090_1f80, 1, 0x00ff_ffff, &[1, 2, 3], number_cell);
    jump_stub(&mut t, 0x0090_1f80, &code);
    cells.push(number_cell);
    for _ in 0..1500 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let block = t.alloc_random(0x40);
        t.put(info, block);
        let l = t.rng.pick(&[0u32, 0, 1, 5]);
        t.put(lang, l);
        let choice = t.rng.pick(&[0u8, 1, 1, 2]);
        t.put(t.sym(0x0186_628a), choice);
        for k in 0..100u32 {
            let s = t.rng.pick(&[0u8, 1, 1, 2, 3, 4, 5, 0xff]);
            t.put(t.sym(0x0186_6290) + k, s);
        }
        let window = t.alloc_random(0x50);
        let rows = t.rng.pick(&[0u16, 1, 10, 99, 99, 0x8000]);
        t.put(window + 0x30, rows);
        for o in [0x32u32, 0x34] {
            let v = t.rng.pick(&[0u16, 1, 5, 90, 0xffff, 0xfff0, 0x8000]);
            t.put(window + o, v);
        }
        let scroll = t
            .rng
            .pick(&[0u16, 0x55, 0x56, 0x1000, 0x7fff, 0x8000, 0xffab, 0xffff]);
        t.put(window + 0x46, scroll);
        let row = t
            .rng
            .pick(&[0u32, 1, 9, 10, 50, 98, 99, 0xffff_ffff, 0x8000_0000])
            .cast_signed();
        let mode = t.rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff]);
        let (x, y) = (status_int(&mut t), status_int(&mut t));
        let args = [
            window,
            x.cast_unsigned(),
            y.cast_unsigned(),
            row.cast_unsigned(),
            mode,
        ];
        t.twin("MenuSaveLoad_DrawSlotRow", 0x008b_4b70, &args, VOID, |o| {
            tk::menu_save_load_draw_slot_row(o, window, [x, y], row, mode, local, &mut callback);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

/// Code logging `id`, the string at `[esp + 4]` with its terminator (a
/// frame buffer) and the word arguments `args` (indices), returning
/// `[cell]`.
fn seq_code_str_args(log: u32, id: u32, args: &[u8], cell: u32) -> Vec<u8> {
    seq_code_str_at(log, id, 0, args, cell)
}

/// [`seq_code_str_args`] with the string in argument `k`.
fn seq_code_str_at(log: u32, id: u32, k: u8, args: &[u8], cell: u32) -> Vec<u8> {
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat(); // mov ecx, [log]
    code.extend([&[0xc7, 0x01][..], &id.to_le_bytes()].concat()); // mov dword [ecx], id
    #[rustfmt::skip]
    code.extend([
        0x83, 0xc1, 0x04,       // add ecx, 4
        0x8b, 0x54, 0x24, 4 + 4 * k, // mov edx, [esp + 4 + 4k]
        0x8a, 0x02,             // copy: mov al, [edx]
        0x88, 0x01,             // mov [ecx], al
        0x41, 0x42,             // inc ecx; inc edx
        0x84, 0xc0,             // test al, al
        0x75, 0xf6,             // jnz copy
    ]);
    for &k in args {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x01, 0x83, 0xc1, 0x04]); // mov eax, [esp + 4 + 4k]; mov [ecx], eax; add ecx, 4
    }
    code.extend([&[0x89, 0x0d][..], &l].concat()); // mov [log], ecx
    code.extend([&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat()); // mov eax, [cell]; ret
    code
}

#[test]
fn menu_round26_summon_plate_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0026_6e10) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = Vec::new();
    for (f, n) in [
        (0x008a_c850u32, 1u8),
        (0x0090_5930, 6),
        (0x0090_3bb0, 9),
        (0x008a_98c0, 1),
        (0x008a_9990, 1),
        (0x008a_9970, 1),
        (0x008a_99b0, 1),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let lang = seq_stub(&mut t, 0x008a_c2f0, log, 0, &[]);
    let text_cell = t.alloc(&[0; 4]);
    let code = seq_code_str_args(log, 0x0090_1660, &[1, 2, 3, 4, 5], text_cell);
    jump_stub(&mut t, 0x0090_1660, &code);
    cells.push(text_cell);
    for _ in 0..1500 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let l = t.rng.pick(&[0u32, 1, 11, 11, 12, 0xb00]);
        t.put(lang, l);
        // HP/MP values: small, wide or negative (the format is %5d/%4d)
        for &c in &cells[3..7] {
            let v = t
                .rng
                .pick(&[0u32, 1, 999, 9999, 99_999, 0xffff_ffff, 0x8000_0000]);
            let v = if t.rng.below(2) == 0 {
                v
            } else {
                t.rng.below(100_000)
            };
            t.put(c, v);
        }
        let (x, y) = (quiet_float(&mut t), quiet_float(&mut t));
        let (chr, alpha) = (t.rng.next(), t.rng.next());
        t.twin(
            "DrawCrossMenuSummonPlateClut",
            0x008e_6e10,
            &[chr, x, y, alpha],
            U32,
            |o| {
                w(tk::draw_cross_menu_summon_plate_clut(
                    o,
                    chr,
                    [f32::from_bits(x), f32::from_bits(y)],
                    alpha,
                    local,
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round27_summon_plate_check_pad_rows_and_btl_end_text_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0027_eb00) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = Vec::new();
    for (f, n) in [
        (0x008a_99f0u32, 1u8),
        (0x008e_6e10, 4),
        (0x008e_7110, 4),
        (0x008c_d720, 3),
        (0x0088_98a0, 4),
        (0x0088_9570, 3),
        (0x0063_0e20, 3),
        (0x008d_1820, 5),
        (0x0078_51f0, 0),
        (0x0078_fb60, 2),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let hp = seq_stub(&mut t, 0x008a_98c0, log, 1, &[]);
    let sub_cmd = t.sym(0x023c_c128);
    let marks = t.sym(0x0186_a5b0);
    let window = t.alloc(&[0; 0x98]);
    for _ in 0..1500 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let v = t
            .rng
            .pick(&[0u32, 1, 0xffff_ffff, 0x8000_0000, 0x7fff_ffff, 5]);
        t.put(hp, v);
        // the sub-command window: none, or a row marked 0, 1 or 2
        let open = t.rng.below(4) != 0;
        t.put(sub_cmd, if open { window } else { 0 });
        let row = t.rng.below(24).cast_signed() - 4;
        t.put(window + 0x48, i16::try_from(row).unwrap_or(0));
        for k in -4..20 {
            let m = t.rng.pick(&[0u8, 1, 1, 2]);
            t.put(marks.wrapping_add_signed(k), m);
        }
        let chr = t
            .rng
            .pick_or_any(&[0, 7, 8, 13, 14, 15, 0xffff_ffff, 0x8000_0000]);
        let (x, y) = (t.rng.int(), t.rng.int());
        t.twin(
            "TkMn2DrawSummonPlateCheckHP",
            0x008c_eb00,
            &[x, y, chr],
            U32,
            |o| {
                w(tk::tk_mn2_draw_summon_plate_check_hp(
                    o,
                    [i(x), i(y)],
                    i(chr),
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );

        // a pad's frame: work records 0..7, vibration on or off
        let (port, slot) = (t.rng.below(4), t.rng.below(4));
        let work = tk::tk_get_controler_work(&t.b, i(port), i(slot));
        for off in [0x98u32, 0x99, 0x9d, 0xfe, 0xff] {
            let b = t.rng.pick(&[0u8, 1, 0x80, 0xff]);
            t.put(work + off, b);
        }
        let pad = t.rng.next();
        t.twin(
            "Pad_ReadAndUpdate",
            0x0088_8f90,
            &[port, slot, pad],
            VOID,
            |o| {
                tk::pad_read_and_update(o, port, slot, pad, &mut callback);
                0
            },
        );

        let (list, fixed, first, y) = (
            t.rng.next(),
            t.rng.pick_or_any(&[0, 1]),
            t.rng.int(),
            t.rng.int(),
        );
        let unused = t.rng.next();
        t.twin(
            "MenuSelList_DrawRows",
            0x008d_1620,
            &[list, fixed, first, unused, y],
            VOID,
            |o| {
                tk::menu_sel_list_draw_rows(
                    o,
                    list,
                    fixed,
                    i(first),
                    i(y),
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );

        let id = t.rng.int();
        t.twin("TkBtlEndGetText", 0x008b_da60, &[id], U32, |o| {
            w(tk::tk_btl_end_get_text(o, id, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// [`callback`] for a callee returning in `ST0` ([`Oracle::call_f64`]: NaN
/// signs and payloads kept).
fn fcallback(o: &mut Oracle, f: u32, args: &[u32]) -> f64 {
    o.call_f64(f, args).unwrap()
}

#[test]
fn menu_round27_sps2_load_and_help_text_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0027_ce80) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);

    // TKMallocAndReadSPS2: sprintf_s runs as it is; fiosUnifyFilename
    // logs its size and copies the name with every byte's bit 0 flipped
    let lang = seq_stub(&mut t, 0x0064_10e0, log, 0, &[]);
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat(); // mov ecx, [log]
    code.extend([&[0xc7, 0x01][..], &0x0067_9820u32.to_le_bytes()].concat()); // mov dword [ecx], id
    #[rustfmt::skip]
    code.extend([
        0x8b, 0x44, 0x24, 0x0c, // mov eax, [esp + 12]
        0x89, 0x41, 0x04,       // mov [ecx + 4], eax
        0x83, 0x05, l[0], l[1], l[2], l[3], 0x08, // add dword [log], 8
        0x8b, 0x54, 0x24, 0x04, // mov edx, [esp + 4]
        0x8b, 0x4c, 0x24, 0x08, // mov ecx, [esp + 8]
        0x8a, 0x02,             // copy: mov al, [edx]
        0x84, 0xc0,             // test al, al
        0x74, 0x02,             // jz store
        0x34, 0x01,             // xor al, 1
        0x88, 0x01,             // store: mov [ecx], al
        0x41, 0x42,             // inc ecx; inc edx
        0x84, 0xc0,             // test al, al
        0x75, 0xf0,             // jnz copy
        0xc3,
    ]);
    jump_stub(&mut t, 0x0067_9820, &code);
    let exists = t.alloc(&[0; 4]);
    let code = seq_code_str_args(log, 0x0067_96f0, &[], exists);
    jump_stub(&mut t, 0x0067_96f0, &code);
    let file = t.alloc(&[0; 4]);
    let code = seq_code_str_args(log, 0x0067_9730, &[1], file);
    jump_stub(&mut t, 0x0067_9730, &code);
    let none = t.alloc(&[0; 4]);
    let code = seq_code_str_at(log, 0x0062_f500, 1, &[0], none);
    jump_stub(&mut t, 0x0062_f500, &code);
    let size = seq_stub(&mut t, 0x0067_9570, log, 1, &[]);
    let malloc = seq_stub(&mut t, 0x0068_7020, log, 1, &[]);
    let mut cells = vec![file];
    for (f, n) in [(0x0083_c4a0u32, 1u8), (0x0067_9800, 3), (0x0067_9510, 1)] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let heap = t.alloc(&[0; 16]);
    // names that make the path 254 and 255 characters long: sprintf_s's
    // 0xff limit takes the first and rejects the second
    let long_names: Vec<u32> = [226usize, 227]
        .map(|n| {
            let mut name = vec![b'a'; n];
            name.push(0);
            t.alloc(&name)
        })
        .to_vec();
    for _ in 0..400 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let l = t.rng.pick_or_any(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        t.put(lang, l);
        let e = t.rng.pick_or_any(&[0, 1]);
        t.put(exists, e);
        let n = t.rng.pick_or_any(&[
            0,
            1,
            0x10,
            0x7f1,
            0x7f0,
            0x7ff,
            0x800,
            0x8000_0000,
            0x7fff_f7f0,
            0xffff_fff0,
        ]);
        t.put(size, n);
        t.put(malloc, heap);
        let junk = t.rng.next();
        t.put(heap, junk);
        let (index, page) = (t.rng.below(5), t.rng.pick_or_any(&[0, 1]));
        let entry = t.sym(if page == 0 { 0x00c5_8e44 } else { 0x00c5_8e58 }) + 4 * index;
        let name = Mem::read::<u32>(&t.a, entry);
        if t.rng.below(4) == 0 {
            let to = t.rng.pick(&long_names);
            t.put(entry, to);
        }
        t.twin(
            "TKMallocAndReadSPS2",
            0x0088_ce80,
            &[index, page],
            U32,
            |o| {
                w(tk::tk_malloc_and_read_sps2(
                    o,
                    index,
                    page,
                    [local, local + 0x100],
                    &mut callback,
                ))
            },
        );
        t.put(entry, name);
    }

    // DrawMesFontLClutTypeRGBA_ForHelp: TOGetGroup and TOSkipCode run as
    // they are; the float callees return their cell in ST0
    let mut cells = Vec::new();
    let screen = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x0064_0db0, &[], screen, &[]);
    jump_stub(&mut t, 0x0064_0db0, &code);
    // TOAddFontWidthSpace moves the pen's x (to any value, -0.0 too)
    let pen_x = t.sym(0x025d_09a4);
    let moved = t.alloc(&[0; 4]);
    let mut extra = [&[0xa1][..], &moved.to_le_bytes()].concat(); // mov eax, [moved]
    extra.extend([&[0xa3][..], &pen_x.to_le_bytes()].concat()); // mov [pen_x], eax
    cells.push(seq_stub(&mut t, 0x008e_8ed0, log, 3, &extra));
    for (f, n) in [
        (0x008b_7240u32, 2u8),
        (0x0064_3af0, 1),
        (0x0064_0dd0, 1),
        (0x008f_bae0, 3),
        (0x008f_b940, 3),
        (0x008b_1350, 2),
        (0x008f_8ee0, 8),
        (0x008b_1330, 0),
        (0x008e_9ca0, 6),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let mut floats = Vec::new();
    for (f, n) in [(0x008e_5280u32, 5u8), (0x008f_a800, 2), (0x008f_a570, 1)] {
        let cell = t.alloc(&[0; 4]);
        let mut code = seq_code(log, f, n, cell, &[]);
        code.truncate(code.len() - 6);
        code.extend([&[0xd9, 0x05][..], &cell.to_le_bytes(), &[0xc3]].concat()); // fld dword [cell]; ret
        jump_stub(&mut t, f, &code);
        floats.push(cell);
    }
    let text = t.alloc(&[0; 64]);
    for _ in 0..1500 {
        t.put(log, start);
        reroll(&mut t, &cells);
        for &c in &floats {
            let v = quiet_float(&mut t);
            t.put(c, v);
        }
        let v = if t.rng.below(4) == 0 {
            (-0.0f32).to_bits()
        } else {
            quiet_float(&mut t)
        };
        t.put(moved, v);
        let mut bytes = Vec::new();
        for _ in 0..t.rng.below(9) {
            let r = t.rng.next().to_le_bytes();
            let token: Vec<u8> = match t.rng.below(10) {
                0 => vec![t.rng.pick(&[1u8, 2, 3])],
                1 => vec![7, t.rng.pick(&[0xe0, 0xe1, 0xe8, 0xef, 0xf0, r[0] | 1])],
                2 => vec![0xa, r[0]],
                3 => vec![0xb, t.rng.pick(&[0x33, 0x34, 0x37, 0x38, r[0] | 1])],
                4 => vec![t.rng.pick(&[8u8, 9, 0xd, 0xe, 0x10]), r[0] | 1],
                5 => vec![0x12 + t.rng.below(0x12).to_le_bytes()[0], r[0] | 1],
                6 => vec![t.rng.pick(&[4u8, 5, 6, 0xc, 0xf, 0x11]), r[0] | 1],
                7 => vec![0x47, 0x47],
                _ => vec![0x30 | r[1]],
            };
            bytes.extend(token);
        }
        bytes.truncate(56);
        bytes.resize(64, 0);
        t.put_bytes(text, &bytes);
        t.scramble(0x025d_09a0, 0x20);
        t.scramble(0x0184_1d50, 0x40);
        let key = t.rng.pick_or_any(&[0xffff_ffff, 0, 1]);
        let (x, y) = (t.rng.int(), t.rng.int());
        let flags = t.rng.next();
        let rgba = [t.rng.next(), t.rng.next(), t.rng.next(), t.rng.next()];
        t.twin(
            "DrawMesFontLClutTypeRGBA_ForHelp",
            0x008e_75c0,
            &[key, text, x, y, flags, rgba[0], rgba[1], rgba[2], rgba[3]],
            VOID,
            |o| {
                tk::draw_mes_font_lclut_type_rgba_for_help(
                    o,
                    key,
                    text,
                    [i(x), i(y)],
                    flags,
                    rgba,
                    local,
                    &mut callback,
                    &mut fcallback,
                );
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

/// `f` replaced by a logging stub returning `[cell]` in `ST0` (`fld`);
/// returns the cell.
fn float_stub(t: &mut Twins, f: u32, log: u32, n: u8) -> u32 {
    let cell = t.alloc(&[0; 4]);
    let mut code = seq_code(log, f, n, cell, &[]);
    code.truncate(code.len() - 6);
    code.extend([&[0xd9, 0x05][..], &cell.to_le_bytes(), &[0xc3]].concat()); // fld dword [cell]; ret
    jump_stub(t, f, &code);
    cell
}

#[test]
fn menu_round27_key_use_cells_and_status_stats_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0027_4770) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);

    // MenuItem_DrawKeyUseCells
    let table = seq_stub(&mut t, 0x008c_4f80, log, 0, &[]);
    let record = t.alloc(&[0; 8]);
    let mut cells = Vec::new();
    for (f, n) in [(0x008f_5f70u32, 5u8), (0x0090_5ab0, 6), (0x008c_0930, 6)] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let width = out_stub(&mut t, log, 0x0090_5290, &[0, 2, 3, 4], 1);
    let time = float_stub(&mut t, 0x0064_1410, log, 0);
    let names = t.sym(0x0186_a27c);
    let scale = t.sym(0x0186_a31c);
    for _ in 0..1500 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.put(table, record);
        let n = t.rng.pick_or_any(&[0, 1, 2, 4, 7, 8, 9]) & 0xf;
        t.put(record + 2, u16::try_from(n).unwrap_or(0));
        for k in 0..13 {
            let b = if t.rng.below(3) == 0 { 0 } else { t.rng.next() };
            t.put(names + 4 * k, b);
        }
        let s = t.rng.pick_or_any(&[0, 1, 2, 3, 0x7fff_ffff, 0x8000_0000]);
        t.put(scale, s);
        let text_w = quiet_float(&mut t);
        t.put(width, text_w);
        let v = t.rng.pick(&[
            3.25f32,
            3.5,
            3.75,
            0.5,
            0.75,
            -2.75,
            -0.25,
            3.0e9,
            -3.0e9,
            1.0e20,
            8_388_608.5,
        ]);
        let v = if t.rng.below(4) == 0 {
            quiet_float(&mut t)
        } else {
            v.to_bits()
        };
        t.put(time, v);
        t.twin("MenuItem_DrawKeyUseCells", 0x008c_4770, &[], VOID, |o| {
            tk::menu_item_draw_key_use_cells(
                o,
                local,
                &mut ui_remap,
                &mut callback,
                &mut fcallback,
            );
            0
        });
    }

    // TkMn2MainStatusPlayerStatus
    let ease = seq_stub(&mut t, 0x008d_30c0, log, 2, &[]);
    let mut cells = Vec::new();
    for (f, n) in [
        (0x0078_5330u32, 1u8),
        (0x008b_f120, 2),
        (0x008c_1490, 7),
        (0x008c_0cd0, 0),
        (0x008b_ee70, 1),
        (0x0090_3bb0, 9),
        (0x0090_16b0, 7),
        (0x008c_0cc0, 0),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let width = out_stub(&mut t, log, 0x0090_5320, &[0, 1, 3, 4], 2);
    let stats = t.sym(0x00c5_a790);
    let shift = t.sym(0x023c_c420);
    // text widths around the plate's limit: remapX2(80) + w against
    // remapX2(660)
    let limit = ui_remap(&mut t.b, false, 660.0);
    let pad = ui_remap(&mut t.b, false, 80.0);
    #[allow(clippy::cast_possible_truncation)]
    let edge = ((limit - pad) as f32).to_bits();
    t.b.clear_dirty();
    for _ in 0..1500 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let e = t
            .rng
            .pick_or_any(&[0, 1, 0x1000, 0xfff, 0xffff_f000, 0x7fff_ffff, 0x8000_0000]);
        t.put(ease, e);
        let text_w = if t.rng.below(3) == 0 {
            quiet_float(&mut t)
        } else if t.rng.below(2) == 0 {
            edge.wrapping_add(t.rng.below(9)).wrapping_sub(4)
        } else {
            (150.0 + f32::from(u16::try_from(t.rng.below(40)).unwrap_or(0))).to_bits()
        };
        t.put(width, text_w);
        let v = t.rng.next();
        t.put(shift, v);
        // the stat table, cut short now and then
        let cut = t.rng.below(12);
        let saved = Mem::read::<i32>(&t.a, stats + 8 * cut);
        if cut < 9 {
            let end = t.rng.pick(&[-1, -2, 0, i32::MIN]);
            t.put(stats + 8 * cut, end);
        }
        let (y, chr, time) = (
            t.rng.int(),
            t.rng.pick_or_any(&[0, 7, 13, 14, 15, 17, 18, 0xffff_ffff]),
            t.rng.next(),
        );
        let unused = t.rng.next();
        t.twin(
            "TkMn2MainStatusPlayerStatus",
            0x008d_5180,
            &[unused, y, chr, time],
            U32,
            |o| {
                w(tk::tk_mn2_main_status_player_status(
                    o,
                    i(y),
                    i(chr),
                    time,
                    local,
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
        t.put(stats + 8 * cut, saved);
    }
    eprintln!("{} calls", t.calls);
}

/// `f` replaced by a stub logging `args` and returning `base + (low byte
/// of argument 0) * stride` (a record per id).
fn record_stub(
    t: &mut Twins,
    f: u32,
    log: u32,
    args: &[u8],
    base: u32,
    stride: u8,
    out: Option<u8>,
) {
    let unused = t.alloc(&[0; 4]);
    // an out word: [argument out] = esp
    let extra: Vec<u8> = out.map_or(Vec::new(), |k| {
        vec![0x8b, 0x54, 0x24, 4 + 4 * k, 0x89, 0x22] // mov edx, [esp + 4 + 4k]; mov [edx], esp
    });
    let mut code = seq_code_of(log, f, args, unused, &extra);
    code.truncate(code.len() - 6);
    let scale = match stride {
        2 => 0x45,
        4 => 0x85,
        _ => 0xc5,
    };
    #[rustfmt::skip]
    code.extend([
        &[0x8b, 0x44, 0x24, 0x04][..], // mov eax, [esp + 4]
        &[0x0f, 0xb6, 0xc0],           // movzx eax, al
        &[0x8d, 0x04, scale], &base.to_le_bytes(), // lea eax, [eax * stride + base]
        &[0xc3],
    ].concat());
    jump_stub(t, f, &code);
}

#[test]
fn menu_round27_pad_init_event_movie_wshop_list_and_key_rows_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0027_9360) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);

    // Pad_InitWork and AtelEvent_FreeMovie
    let mut cells = Vec::new();
    for (f, n) in [
        (0x0088_8ee0u32, 1u8),
        (0x0088_88e0, 1),
        (0x0088_8910, 1),
        (0x0088_89f0, 2),
        (0x0088_94f0, 4),
        (0x0076_f090, 0),
        (0x0088_d520, 1),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let event = seq_stub(&mut t, 0x0088_c830, log, 0, &[]);
    let save = t.alloc(&[0; 0x100]);
    let buffers = t.sym(0x0132_7110);
    for _ in 0..600 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let (port, slot) = (t.rng.below(4), t.rng.below(4));
        let work = tk::tk_get_controler_work(&t.b, i(port), i(slot));
        t.scramble(work + 0x80, 0x24);
        t.twin("Pad_InitWork", 0x0088_9360, &[port, slot], VOID, |o| {
            tk::pad_init_work(o, port, slot, &mut callback);
            0
        });

        t.put(event, save);
        let kind = t.rng.pick_or_any(&[0, 1, 2, 3, 4, 5, 6, 0xffff_ffff]);
        t.put(save + 0xe0, kind);
        for k in 0..3 {
            let p = if t.rng.below(2) == 0 { 0 } else { t.rng.next() };
            t.put(buffers + 4 * k, p);
        }
        t.twin("AtelEvent_FreeMovie", 0x0087_4130, &[], U32, |o| {
            w(tk::atel_event_free_movie(o, &mut callback))
        });
    }

    // MenuWShop_InitList
    let excel = seq_stub(&mut t, 0x008c_1af0, log, 2, &[]);
    let list = t.alloc(&[0; 0x40]);
    let owned_cell = t.alloc(&[0; 4]);
    let count = t.alloc(&[0; 4]);
    #[rustfmt::skip]
    let extra: Vec<u8> = [
        &[0x8b, 0x54, 0x24, 0x10][..], // mov edx, [esp + 16]
        &[0xa1], &count.to_le_bytes(), // mov eax, [count]
        &[0x89, 0x02],                 // mov [edx], eax
    ].concat();
    let code = seq_code_of(log, 0x007a_bd10, &[0, 1, 2], owned_cell, &extra);
    jump_stub(&mut t, 0x007a_bd10, &code);
    let owned = t.alloc(&[0; 0x40]);
    let weapons = t.alloc(&[0; 0x400]);
    record_stub(&mut t, 0x007a_bbd0, log, &[0], weapons, 4, Some(1));
    let records = t.alloc(&[0; 0x800]);
    record_stub(&mut t, 0x008d_9170, log, &[0], records, 8, None);
    let named = seq_stub(&mut t, 0x0079_93b0, log, 1, &[]);
    for _ in 0..600 {
        t.put(log, start);
        let v = t.rng.next();
        t.put(named, v);
        t.put(excel, list);
        t.put(owned_cell, owned);
        let n = t.rng.pick_or_any(&[0, 1, 5, 16, 0xffff_ffff]) & 0x8000_001f;
        t.put(count, n);
        for k in 0..20 {
            let id = if t.rng.below(8) == 0 {
                0
            } else {
                t.rng.below(0x100)
            };
            t.put(list + 2 * k, u16::try_from(id).unwrap_or(0));
            let id = t.rng.below(0x100);
            t.put(owned + 2 * k, u16::try_from(id).unwrap_or(0));
        }
        for k in 0..0x100 {
            let b = u8::from(t.rng.below(16) != 0);
            t.put(weapons + 4 * k + 2, b);
        }
        t.scramble(0x0186_ab10, 8);
        t.scramble(0x0186_ab1c, 4);
        t.scramble(0x0186_aba8, 4);
        t.twin("MenuWShop_InitList", 0x008d_94a0, &[], U32, |o| {
            w(tk::menu_wshop_init_list(
                o,
                [local, local + 4],
                &mut callback,
            ))
        });
    }

    // MenuItem_KeyRowCount: PasswordGetTotalIndex counts up per call
    let lang = seq_stub(&mut t, 0x008a_c2f0, log, 0, &[]);
    let english = seq_stub(&mut t, 0x0078_51d0, log, 0, &[]);
    let total = t.alloc(&[0; 4]);
    let mut code = seq_code_of(log, 0x0079_02d0, &[], total, &[]);
    code.pop();
    code.extend([&[0xff, 0x05][..], &total.to_le_bytes(), &[0xc3]].concat()); // inc dword [total]; ret
    jump_stub(&mut t, 0x0079_02d0, &code);
    for _ in 0..600 {
        t.put(log, start);
        let l = t.rng.pick_or_any(&[0, 0, 1, 2, 9, 10, 11, 12]);
        t.put(lang, l);
        let e = t.rng.pick_or_any(&[0, 1]);
        t.put(english, e);
        let v = t
            .rng
            .pick_or_any(&[0, 1, 2, 3, 0xffff_fffe, 0x7fff_ffff, 0x8000_0000]);
        t.put(total, v);
        let mode = t.rng.below(3);
        t.put(t.sym(0x0186_a25c), mode);
        let target = t.rng.pick_or_any(&[0, 1, 0xffff_ffff, 0x8000_0000]);
        t.put(t.sym(0x0186_a258), target);
        t.twin("MenuItem_KeyRowCount", 0x008c_5180, &[], U32, |o| {
            s(tk::menu_item_key_row_count(o, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// `f` replaced by a stub logging `args` and returning the byte of the
/// table at `base` indexed by the low byte of argument `k`.
fn byte_table_stub(t: &mut Twins, f: u32, log: u32, args: &[u8], k: u8, base: u32) {
    let unused = t.alloc(&[0; 4]);
    let mut code = seq_code_of(log, f, args, unused, &[]);
    code.truncate(code.len() - 6);
    #[rustfmt::skip]
    code.extend([
        &[0x0f, 0xb6, 0x44, 0x24, 4 + 4 * k][..], // movzx eax, byte [esp + 4 + 4k]
        &[0x0f, 0xb6, 0x80], &base.to_le_bytes(), // movzx eax, byte [eax + base]
        &[0xc3],
    ].concat());
    jump_stub(t, f, &code);
}

/// Text for the key tests: characters, two- and three-byte codes, an
/// early end now and then.
fn key_text(t: &mut Twins) -> Vec<u8> {
    let mut bytes = Vec::new();
    for _ in 0..t.rng.below(10) {
        let r = t.rng.next().to_le_bytes();
        match t.rng.below(6) {
            0 => bytes.extend([6 + r[0] % 0x2a, r[1] | 1]),
            1 => bytes.extend([4, (r[0] % 0x30) | 1, r[1] | 1]),
            2 => bytes.extend([4, 0x30 | r[0]]),
            3 if t.rng.below(4) == 0 => bytes.push(0),
            _ => bytes.push(0x30 | r[0]),
        }
    }
    bytes.resize(40, 0);
    bytes
}

#[test]
fn menu_round27_type_key_and_disp_command_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0027_4c90) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);

    // MenuItem_TypeKey
    let table = seq_stub(&mut t, 0x008c_4f80, log, 0, &[]);
    let record = t.alloc(&[0; 0x40]);
    let lang = seq_stub(&mut t, 0x008a_c2f0, log, 0, &[]);
    let english = seq_stub(&mut t, 0x0078_51d0, log, 0, &[]);
    let total = seq_stub(&mut t, 0x0079_02d0, log, 0, &[]);
    let entry = seq_stub(&mut t, 0x0079_02a0, log, 1, &[]);
    let name = seq_stub(&mut t, 0x008b_edd0, log, 1, &[]);
    let (text_a, text_b) = (t.alloc(&[0; 40]), t.alloc(&[0; 40]));
    let keys = t.sym(0x0186_a27c);
    for _ in 0..1500 {
        t.put(log, start);
        t.put(table, record);
        t.put(entry, text_a);
        t.put(name, text_b);
        let n = t.rng.pick(&[0u16, 1, 2, 5, 8, 9]);
        t.put(record + 2, n);
        for k in 0..8 {
            let id = u16::try_from(t.rng.below(0x200)).unwrap_or(0);
            t.put(record + 2 * k, id);
        }
        t.put(record + 2, n);
        let l = t.rng.pick_or_any(&[0, 0, 1, 2, 9, 10, 11, 12]);
        t.put(lang, l);
        let e = t.rng.pick_or_any(&[0, 1]);
        t.put(english, e);
        let v = t.rng.pick(&[0u32, 1, 3, 8, 0xffff_ffff]);
        t.put(total, v);
        for text in [text_a, text_b] {
            let bytes = key_text(&mut t);
            t.put_bytes(text, &bytes);
        }
        let cursor = t.rng.below(60).cast_signed() - 15;
        t.put(t.sym(0x0186_a268), cursor);
        let mode = t.rng.below(3);
        t.put(t.sym(0x0186_a25c), mode);
        let target = t.rng.pick_or_any(&[0, 1, 0xffff_ffff, 0x8000_0000]);
        t.put(t.sym(0x0186_a258), target);
        for k in 0..12 {
            let v = if t.rng.below(3) == 0 { 0 } else { t.rng.next() };
            t.put(keys + 4 * k, v);
        }
        t.twin("MenuItem_TypeKey", 0x008c_4c90, &[], VOID, |o| {
            tk::menu_item_type_key(o, &mut callback);
            0
        });
    }

    // TkMenuGetDispCommand: membership and commands by id from byte tables
    let flags = t.alloc(&[0; 0x100]);
    let panels = t.alloc(&[0; 0x100]);
    let list = t.alloc(&[0; 0x60]);
    let list_cell = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x0079_0200, &[0], list_cell, &[]);
    jump_stub(&mut t, 0x0079_0200, &code);
    let max = seq_stub(&mut t, 0x008d_2d80, log, 4, &[]);
    let offset = seq_stub(&mut t, 0x008d_2dc0, log, 4, &[]);
    byte_table_stub(&mut t, 0x0078_5380, log, &[0], 0, flags);
    byte_table_stub(&mut t, 0x0078_5020, log, &[0, 1], 1, flags);
    byte_table_stub(&mut t, 0x0078_50b0, log, &[0], 0, panels);
    let weapon = seq_stub(&mut t, 0x008a_9c70, log, 1, &[]);
    let armour = seq_stub(&mut t, 0x008a_9820, log, 1, &[]);
    let gear = t.alloc(&[0; 0x900]);
    record_stub(&mut t, 0x007a_bbd0, log, &[0], gear, 8, Some(1));
    let mut cells = Vec::new();
    for (f, n) in [(0x0078_5330u32, 1u8), (0x008a_8e20, 0)] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let out = t.alloc(&[0; 0x80]);
    for _ in 0..1500 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.put(list_cell, list);
        for k in 0..0x18 {
            let id = if t.rng.below(6) == 0 {
                100
            } else {
                t.rng.below(0x100)
            };
            let low = t.rng.next() & 0xffff;
            t.put(list + 4 * k, low | id << 16);
        }
        // half the ids missing, half without a panel
        for table in [flags, panels] {
            let bytes: Vec<u8> = (0..0x100).map(|_| t.rng.pick(&[0u8, 0, 1, 0x80])).collect();
            t.put_bytes(table, &bytes);
        }
        for k in 0..0x120 {
            let v = t.rng.pick(&[0u16, 0xff, 1, 0x1234, 0x100]);
            t.put(gear + 8 * (k / 4) + 0xe + 2 * (k % 4), v);
        }
        let m = t.rng.pick_or_any(&[0, 1, 4, 8, 0xffff_ffff]) & 0x8000_000f;
        t.put(max, m);
        let o = t.rng.below(4);
        t.put(offset, o);
        let w0 = t.rng.pick_or_any(&[0xff, 0, 1, 0x80]) & 0xff;
        t.put(weapon, w0);
        let a0 = t.rng.pick_or_any(&[0xff, 0, 2, 0x81]) & 0xff;
        t.put(armour, a0);
        t.scramble_at(out, 0x80);
        let kind = t.rng.pick_or_any(&[0, 0, 1, 2]);
        let sub = t.rng.pick_or_any(&[1, 1, 0x40, 0x47, 0x48, 0x3f, 0]);
        let chr = t.rng.pick_or_any(&[0, 7, 8, 9, 0xffff_ffff]);
        let a = t.rng.next();
        t.twin(
            "TkMenuGetDispCommand",
            0x008d_2e10,
            &[kind, sub, a, chr, out],
            U32,
            |o| {
                s(tk::tk_menu_get_disp_command(
                    o,
                    kind,
                    sub,
                    a,
                    i(chr),
                    out,
                    [local, local + 4],
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round27_font1612_texts_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0027_4af0) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let lang = seq_stub(&mut t, 0x008a_c2f0, log, 0, &[]);
    // TOAddFontWidthSpace moves the pen's x; the face pass the packet cursor
    let pen_x = t.sym(0x025d_09a4);
    let moved = t.alloc(&[0; 4]);
    let mut extra = [&[0xa1][..], &moved.to_le_bytes()].concat(); // mov eax, [moved]
    extra.extend([&[0xa3][..], &pen_x.to_le_bytes()].concat()); // mov [pen_x], eax
    let mut cells = vec![seq_stub(&mut t, 0x008e_8ed0, log, 3, &extra)];
    let cursor = t.sym(0x0187_16cc);
    // the face pass also logs the pen's x
    let l = log.to_le_bytes();
    let mut bump = [&[0x83, 0x05][..], &cursor.to_le_bytes(), &[0x10]].concat(); // add dword [cursor], 0x10
    #[rustfmt::skip]
    bump.extend([
        &[0xa1][..], &pen_x.to_le_bytes(), // mov eax, [pen_x]
        &[0x8b, 0x0d], &l,                 // mov ecx, [log]
        &[0x89, 0x01],                     // mov [ecx], eax
        &[0x83, 0x05], &l, &[0x04],        // add dword [log], 4
    ].concat());
    cells.push(seq_stub(&mut t, 0x0090_6270, log, 2, &bump));
    for (f, n) in [
        (0x0090_6260u32, 2u8),
        (0x0090_6290, 2),
        (0x0064_3af0, 1),
        (0x0064_0dd0, 1),
        (0x008b_7240, 2),
        (0x008b_1350, 2),
        (0x008f_8ee0, 8),
        (0x008b_1330, 0),
        (0x008e_9b80, 3),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let floats = [
        float_stub(&mut t, 0x008e_5280, log, 5),
        float_stub(&mut t, 0x008f_a800, log, 2),
    ];
    let text = t.alloc(&[0; 64]);
    for round in 0..3000 {
        t.put(log, start);
        reroll(&mut t, &cells);
        for c in floats {
            let v = quiet_float(&mut t);
            t.put(c, v);
        }
        let v = if t.rng.below(4) == 0 {
            (-0.0f32).to_bits()
        } else {
            quiet_float(&mut t)
        };
        t.put(moved, v);
        // every eighth round the English x 200 case: (0xd6, 0xbb) and "h"
        let english_h = round % 8 == 0;
        let l = if english_h {
            1
        } else {
            t.rng.pick_or_any(&[0, 0, 1, 1, 2, 9])
        };
        t.put(lang, l);
        let mut bytes = Vec::new();
        match if english_h { 2 } else { t.rng.below(6) } {
            0 => bytes.extend([0x48, 0x73, 0x78, 0x82, 0x7f]),
            1 => bytes.extend([0x48, 0x73, 0x78, 0x82, 0x7e]),
            2 => bytes.push(b'h'),
            _ => {}
        }
        for _ in 0..t.rng.below(9) {
            let r = t.rng.next().to_le_bytes();
            let token: Vec<u8> = match t.rng.below(10) {
                0 => vec![t.rng.pick(&[1u8, 2, 3])],
                1 => vec![7, t.rng.pick(&[0xe0, 0xe1, 0xe8, 0xef, 0xf0, r[0] | 1])],
                2 => vec![0xa, r[0]],
                3 => vec![
                    0xb,
                    t.rng.pick(&[
                        0x20,
                        0x22,
                        0x23,
                        0x30,
                        0x33,
                        0x34,
                        0x37,
                        0x38,
                        0x42,
                        0x48,
                        r[0] | 1,
                    ]),
                ],
                4 => vec![t.rng.pick(&[8u8, 9, 0xd, 0xe, 0x10]), r[0] | 1],
                5 => vec![0x12 + t.rng.below(0x12).to_le_bytes()[0], r[0] | 1],
                6 => vec![t.rng.pick(&[4u8, 5, 6, 0xc, 0xf, 0x11]), r[0] | 1],
                7 => vec![0x47, 0x47],
                _ => vec![0x30 | r[1]],
            };
            bytes.extend(token);
        }
        bytes.truncate(56);
        bytes.resize(64, 0);
        t.put_bytes(text, &bytes);
        t.scramble(0x025d_09a0, 0x20);
        t.scramble(0x0184_1d50, 0x40);
        let key = t.rng.pick_or_any(&[0xffff_ffff, 0, 1]);
        let x = if english_h {
            0xd6
        } else {
            t.rng.pick_or_any(&[0xd6, 0, 100])
        };
        let y = if english_h {
            0xbb
        } else {
            t.rng.pick_or_any(&[0x163, 0xbb, 340, 350, 360])
        };
        let (packet, flags, edge) = (
            t.rng.next(),
            t.rng.next(),
            t.rng.pick_or_any(&[1, 0, 0x101]),
        );
        let rgba = [t.rng.next(), t.rng.next(), t.rng.next(), t.rng.next()];
        let args = [
            key, text, x, y, flags, edge, rgba[0], rgba[1], rgba[2], rgba[3],
        ];
        if round % 2 == 0 {
            t.twin(
                "MenuMes_AddPktFont1612",
                0x008e_4af0,
                &[&[packet][..], &args].concat(),
                U32,
                |o| {
                    w(tk::menu_mes_add_pkt_font1612(
                        o,
                        packet,
                        key,
                        text,
                        [i(x), i(y)],
                        [flags, edge],
                        rgba,
                        &mut callback,
                        &mut fcallback,
                    ))
                },
            );
        } else {
            t.twin(
                "MenuMes_AddPktFont1612Icons",
                0x008e_4e60,
                &[&[packet][..], &args].concat(),
                U32,
                |o| {
                    w(tk::menu_mes_add_pkt_font1612_icons(
                        o,
                        packet,
                        key,
                        text,
                        [i(x), i(y)],
                        [flags, edge],
                        rgba,
                        &mut callback,
                        &mut fcallback,
                    ))
                },
            );
        }
    }
    eprintln!("{} calls", t.calls);
}

impl Twins {
    /// Random bytes over `len` bytes at `at` (test memory), in both.
    fn scramble_at(&mut self, at: u32, len: usize) {
        let v = self.rng.bytes(len);
        self.put_bytes(at, &v);
    }
}

/// `dbgPrintf` (`0x0062f500`) replaced by a logger: the format and the
/// first value, and the second value for the formats in `three`.
fn printf_log(t: &mut Twins, log: u32, three: &[u32; 3]) {
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = Vec::new();
    #[rustfmt::skip]
    code.extend([
        &[0x8b, 0x0d][..], &l,                  // mov ecx, [log]
        &[0x8b, 0x44, 0x24, 0x04, 0x89, 0x01],  // [ecx] = format
        &[0x8b, 0x44, 0x24, 0x08, 0x89, 0x41, 0x04], // [ecx + 4] = value
        &[0x83, 0x05], &l, &[0x08],             // add dword [log], 8
        &[0x8b, 0x44, 0x24, 0x04],              // mov eax, format
        &[0x3d], &three[0].to_le_bytes(), &[0x74, 14],
        &[0x3d], &three[1].to_le_bytes(), &[0x74, 7],
        &[0x3d], &three[2].to_le_bytes(), &[0x75, 19],
        &[0x8b, 0x0d], &l,                      // mov ecx, [log]
        &[0x8b, 0x44, 0x24, 0x0c, 0x89, 0x01],  // [ecx] = second value
        &[0x83, 0x05], &l, &[0x04],             // add dword [log], 4
        &[0xc3],
    ]
    .concat());
    jump_stub(t, 0x0062_f500, &code);
}

#[test]
fn menu_round28_sort_gear_and_main_cmd_window_match_the_original() {
    let Some(mut t) = setup_any(0x0028_9c40) else {
        return;
    };
    let stack = t.b.stack_range().start;
    let (table, local) = (stack + 0x200, stack + 0x200 + 0x1900);
    // a 2 MB log: 200 entries sort with up to 20000 logged comparisons
    let buf = t.alloc(&vec![0; 0x20_0000]);
    let log = t.alloc(&buf.to_le_bytes());
    let start = Mem::read::<u32>(&t.a, log);
    let three = [0x00b6_5a10, 0x00b6_5a4c, 0x00b6_5a64].map(|a| t.sym(a));
    printf_log(&mut t, log, &three);

    // MenuItem_SortGear: MsGetSaveWeapon(id, &out) logs the id and gives
    // the 0x20-byte record of the id's low byte; abilities from a small set
    // (0xff skipped, high bytes that set the same bits) so that entries tie
    let records = t.alloc(&vec![0; 0x2000]);
    let mut code: Vec<u8> = seq_code_of(log, 0x007a_bbd0, &[0], 0, &[]);
    code.truncate(code.len() - 6);
    #[rustfmt::skip]
    code.extend([
        &[0x0f, 0xb6, 0x44, 0x24, 0x04][..],    // movzx eax, byte [esp + 4]
        &[0xc1, 0xe0, 0x05],                    // shl eax, 5
        &[0x05], &records.to_le_bytes(),        // add eax, records
        &[0xc3],
    ]
    .concat());
    jump_stub(&mut t, 0x007a_bbd0, &code);
    seq_stub(&mut t, 0x007a_b9f0, log, 2, &[]);
    let list = t.alloc(&[0; 4 * 0x120]);
    let abilities = [0u16, 5, 0x21, 0x105, 0xff, 0x1ff, 0x80, 0x9f, 0x3e, 0x1f];
    for round in 0..400u32 {
        t.put(log, start);
        for id in 0..0x100u32 {
            let rec = records + id * 0x20;
            let count = match t.rng.below(8) {
                0 => 0,
                7 => 8,
                k => u8::try_from(k.min(4)).unwrap_or(0),
            };
            t.put(rec + 0xb, count);
            for k in 0..8 {
                let a = t.rng.pick(&abilities);
                t.put(rec + 0xe + 2 * k, a);
            }
        }
        let pool: Vec<u16> = (0..=t.rng.below(12))
            .map(|_| u16::try_from(t.rng.next() & 0x1ff).unwrap_or(0))
            .collect();
        for k in 0..0x120 {
            let id = t.rng.pick(&pool);
            t.put(list + 4 * k, id);
        }
        let n = t.rng.pick(&[0i32, 1, 2, 3, 4, 6, 10, 25, 60, 200, -1, -7]);
        let n = if round == 7 { 200 } else { n };
        let first = t.rng.below(0x18);
        t.twin(
            "MenuItem_SortGear_008c9c40",
            0x008c_9c40,
            &[list, first, n.cast_unsigned()],
            VOID,
            |o| {
                tk::menu_item_sort_gear(o, list, first, n, table, local, &mut callback);
                0
            },
        );
    }

    // TkMenuAppearMainCmdWindow: the gates and every callee logged,
    // TkMenuMainGetOptionMenu/GetMaxOptions the originals
    let flag = seq_stub(&mut t, 0x0086_c470, log, 0, &[]);
    let menu_mask = seq_stub(&mut t, 0x0086_be50, log, 0, &[]);
    let players = seq_stub(&mut t, 0x008a_9b50, log, 0, &[]);
    let summons = seq_stub(&mut t, 0x008a_b1e0, log, 0, &[]);
    seq_stub(&mut t, 0x008a_aeb0, log, 1, &[]);
    let help = seq_stub(&mut t, 0x008d_df60, log, 1, &[]);
    let window = seq_stub(&mut t, 0x008a_a1a0, log, 0, &[]);
    let regist = seq_stub(&mut t, 0x008a_ab00, log, 1, &[]);
    let texts = t.alloc_random(0x400);
    record_stub(&mut t, 0x008d_df80, log, &[0], texts, 4, None);
    let mut code: Vec<u8> = seq_code_of(log, 0x008d_c9e0, &[0], 0, &[]);
    code.truncate(code.len() - 6);
    code.extend([0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0xc3]); // mov eax, [[esp + 4]]
    jump_stub(&mut t, 0x008d_c9e0, &code);
    let order = t.sym(0x00c5_b2d4);
    let game_order: Vec<u8> = (0..32).map(|k| Mem::read::<u8>(&t.a, order + k)).collect();
    let flags = [
        0u32,
        1,
        0x42,
        0x43,
        0xcf,
        0xd0,
        0xd1,
        0xd2,
        0xd8,
        0xd9,
        0x447,
        0x448,
        0x7fff_ffff,
        0xffff_ffff,
        0x8000_0000,
    ];
    for round in 0..600u32 {
        t.put(log, start);
        let f = if round < 60 {
            flags[(round % 15) as usize]
        } else {
            t.rng.pick(&flags)
        };
        t.put(flag, f);
        let m = t.rng.next();
        t.put(menu_mask, m);
        let p = t.rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff, 0x8000_0000]);
        t.put(players, p);
        let s = t.rng.pick(&[0u32, 1, 0x8000_0000, 0x100]);
        t.put(summons, s);
        reroll(&mut t, &[help, regist]);
        let win = t.alloc_random(0x100);
        t.put(window, win);
        t.scramble_at(texts, 0x400);
        for at in [0x0187_1540u32, 0x0187_1568, 0x0187_1668] {
            let at = t.sym(at);
            t.scramble_at(at, 0x40);
        }
        for at in [
            0x0187_1548u32,
            0x0187_154c,
            0x0187_1550,
            0x0187_1554,
            0x0187_1558,
            0x0187_1688,
        ] {
            let v = t.rng.next();
            let at = t.sym(at);
            t.put(at, v);
        }
        if round % 3 == 0 {
            let v: Vec<u8> = (0..32)
                .map(|_| u8::try_from(t.rng.below(32)).unwrap_or(0))
                .collect();
            t.put_bytes(order, &v);
        } else {
            t.put_bytes(order, &game_order);
        }
        let mode = match round % 4 {
            0 => t.rng.below(0x20),
            1 => t.rng.pick(&[0xc, 0xd, 0xe, 0xf, 0x10, 0x18]),
            2 => t
                .rng
                .pick(&[0xb, 0x11, 0x17, 0x19, 0xffff_ffff, 0x8000_000c]),
            _ => t.rng.next(),
        };
        let slide = t.rng.pick(&[0u32, 1, 0x8000_0000, 0x3000]);
        t.twin(
            "TkMenuAppearMainCmdWindow",
            0x008e_1c60,
            &[slide, mode],
            U32,
            |o| {
                w(tk::tk_menu_appear_main_cmd_window(
                    o,
                    slide,
                    mode,
                    &mut callback,
                ))
            },
        );
    }
    t.put_bytes(order, &game_order);
    eprintln!("{} calls", t.calls);
}

/// `f` logging the arguments `args` and storing the word in the returned
/// cell (a float width) through argument `out`, which it also returns.
fn out_float_stub(t: &mut Twins, f: u32, log: u32, args: &[u8], out: u8) -> u32 {
    let cell = t.alloc(&[0; 4]);
    let mut extra = vec![0x8b, 0x54, 0x24, 4 + 4 * out, 0xa1]; // mov edx, [esp + 4 + 4 out]; mov eax, [cell]
    extra.extend(cell.to_le_bytes());
    extra.extend([0x89, 0x02]); // mov [edx], eax
    let code = seq_code_of(log, f, args, cell, &extra);
    jump_stub(t, f, &code);
    cell
}

/// A float for the row tests: mostly on screen, sometimes odd.
fn row_float(t: &mut Twins) -> f32 {
    match t.rng.below(12) {
        0 => t.rng.pick(&[
            0.0f32,
            -0.0,
            1e30,
            -1e30,
            f32::INFINITY,
            f32::from_bits(0x7fc0_0000),
        ]),
        1 => f32::from_bits(t.rng.next() & 0xbfff_ffff),
        _ => t.rng.float(-200.0, 1500.0),
    }
}

/// `n` bytes, mostly 0 or 1.
fn small_bytes(t: &mut Twins, n: usize) -> Vec<u8> {
    (0..n)
        .map(|_| match t.rng.below(4) {
            0 => 0,
            1 => 1,
            2 => u8::try_from(t.rng.below(4)).unwrap_or(0),
            _ => u8::try_from(t.rng.next() & 0xff).unwrap_or(0),
        })
        .collect()
}

#[test]
fn menu_round28_list_row_and_gear_info_match_the_original() {
    let Some(mut t) = setup_any(0x0028_7c30) else {
        return;
    };
    let stack = t.b.stack_range().start;
    let (width, local) = (stack + 0x200, stack + 0x204);
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let list = t.sym(tk::MENU_ITEM_LIST.sym);
    let keys = t.sym(tk::MENU_IMPORTANT_LIST.sym);

    // every callee logged
    let plate = seq_stub(&mut t, 0x008f_5f70, log, 5, &[]);
    let sheet = seq_stub(&mut t, 0x008f_8bb0, log, 5, &[]);
    let digit_w = out_float_stub(&mut t, 0x0090_5230, log, &[0, 2, 3], 1);
    let digits = seq_stub(&mut t, 0x0090_5820, log, 6, &[]);
    let item_name = seq_stub(&mut t, 0x008e_6700, log, 4, &[]);
    let key_get = seq_stub(&mut t, 0x008a_b2c0, log, 1, &[]);
    let key_name = seq_stub(&mut t, 0x008f_50f0, log, 4, &[]);
    let roms = t.alloc(&[0; 0x900]);
    record_stub(&mut t, 0x0079_0a90, log, &[0], roms, 8, None);
    let shape = seq_stub(&mut t, 0x0090_3bb0, log, 9, &[]);
    let cmd_name = seq_stub(&mut t, 0x008f_5000, log, 4, &[]);
    let emit = seq_stub(&mut t, 0x008c_11b0, log, 3, &[]);
    let timer = seq_stub(&mut t, 0x008a_9c60, log, 0, &[]);
    let cursor = seq_stub(&mut t, 0x008c_13d0, log, 3, &[]);
    let maxes = [
        seq_stub(&mut t, 0x008c_8c30, log, 0, &[]),
        seq_stub(&mut t, 0x008c_8a20, log, 0, &[]),
        seq_stub(&mut t, 0x008c_8a00, log, 0, &[]),
    ];
    let small_plate = seq_stub(&mut t, 0x008c_1260, log, 3, &[]);
    let gear = t.alloc(&[0; 0x900]);
    record_stub(&mut t, 0x007a_bbd0, log, &[0], gear, 8, None);
    let worn = [
        seq_stub(&mut t, 0x008a_9820, log, 1, &[]),
        seq_stub(&mut t, 0x008a_9c70, log, 1, &[]),
    ];
    let stats = seq_stub(&mut t, 0x008d_8aa0, log, 3, &[]);
    let name2 = seq_stub(&mut t, 0x008e_6970, log, 4, &[]);
    let status_mes = seq_stub(&mut t, 0x008b_ee70, log, 1, &[]);
    let text_w = out_float_stub(&mut t, 0x0090_5290, log, &[0, 2, 3, 4], 1);
    let font = seq_stub(&mut t, 0x0090_5ab0, log, 6, &[]);
    let kick = seq_stub(&mut t, 0x008c_0cd0, log, 0, &[]);
    let cells = [
        plate,
        sheet,
        digits,
        item_name,
        key_get,
        key_name,
        shape,
        cmd_name,
        emit,
        timer,
        cursor,
        small_plate,
        stats,
        name2,
        status_mes,
        font,
        kick,
    ];

    // MenuItem_DrawListRow: every list kind, rows in and out of range,
    // the cursor row hit or not
    for round in 0..1500u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        for c in [digit_w, text_w] {
            let v = row_float(&mut t).to_bits();
            t.put(c, v);
        }
        let b = small_bytes(&mut t, 0x900);
        t.put_bytes(roms, &b);
        let b = small_bytes(&mut t, 0x800);
        t.put_bytes(list, &b);
        let k: Vec<u8> = (0..0x400)
            .map(|_| {
                if t.rng.below(4) == 0 {
                    0
                } else {
                    u8::try_from(t.rng.next() & 0xff).unwrap_or(0)
                }
            })
            .collect();
        t.put_bytes(keys, &k);
        let window = t.alloc_random(0x100);
        let kind = t.rng.pick(&[0u8, 1, 2, 2, 3]);
        t.put(window + 0x43, kind);
        let (s32, s34) = (
            i16::try_from(t.rng.below(8)).unwrap_or(0) - 1,
            i16::try_from(t.rng.below(8)).unwrap_or(0) - 1,
        );
        t.put(window + 0x32, s32);
        t.put(window + 0x34, s34);
        let mode = t.rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff]);
        let row = t
            .rng
            .pick(&[0u32, 1, 2, 5, 9, 30, 0xffff_ffff, 0xffff_fff0, 0x8000_0000]);
        let shift = match mode {
            0 | 2 => i32::from(s32) * 2,
            1 => i32::from(s34) * 2,
            _ => 0,
        };
        let real = row.wrapping_add(shift.cast_unsigned());
        for (k, &c) in maxes.iter().enumerate() {
            let m = match t.rng.below(4) {
                0 => real,
                1 => real.wrapping_add(1),
                2 => t.rng.below(0x40),
                _ => t.rng.next(),
            };
            let m = if k == 0 && round % 7 == 0 { 0x200 } else { m };
            t.put(c, m);
        }
        let active = t.rng.pick(&[2u8, 2, 1, 0]);
        t.put(window + 0x42, active);
        let shown = t.rng.pick(&[1i8, 3, 0, -1]);
        t.put(window + 0x45, shown);
        let c2c = i16::try_from(t.rng.below(6)).unwrap_or(0) - 2;
        t.put(window + 0x2c, c2c);
        let target = match t.rng.below(3) {
            0 => real,
            1 => u32::from(Mem::read::<u16>(&t.a, keys + real.wrapping_mul(2))),
            _ => t.rng.next(),
        };
        t.put(
            window + 0x58,
            target.wrapping_sub((i32::from(c2c) * 2).cast_unsigned()),
        );
        let (x, y) = (row_float(&mut t), row_float(&mut t));
        t.twin(
            "MenuItem_DrawListRow",
            0x008c_7c30,
            &[window, x.to_bits(), y.to_bits(), row, mode],
            VOID,
            |o| {
                tk::menu_item_draw_list_row(
                    o,
                    window,
                    [x, y],
                    row,
                    mode,
                    [width, local],
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
    }

    // MenuItem_DrawGearInfo: gear mode or not, listed rows or not, worn
    // pieces or none
    let mode = t.sym(tk::MENU_ITEM_GEAR_MODE.sym);
    for _ in 0..800u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let v = row_float(&mut t).to_bits();
        t.put(text_w, v);
        let b = small_bytes(&mut t, 0x900);
        t.put_bytes(gear, &b);
        let b = small_bytes(&mut t, 0x800);
        t.put_bytes(list, &b);
        for c in worn {
            let v = t.rng.pick(&[0xffu32, 0, 3, 0x1ff, 0x1_00ff]);
            let v = if t.rng.below(3) == 0 { t.rng.next() } else { v };
            t.put(c, v);
        }
        let on = t.rng.pick(&[1u8, 1, 1, 0, 0x80]);
        t.put(mode, on);
        let window = t.alloc_random(0x100);
        let row = t.rng.pick(&[0i16, 1, 7, 0x1ff, -1]);
        let row = if t.rng.below(2) == 0 {
            i16::try_from(t.rng.below(0x200)).unwrap_or(0)
        } else {
            row
        };
        t.put(window + 0x48, row);
        // row -1 reads the entry before the list
        let before = [t.rng.pick(&[0u8, 3]), 1, t.rng.pick(&[0u8, 1]), 0];
        t.put_bytes(list - 4, &before);
        t.twin("MenuItem_DrawGearInfo", 0x008c_9570, &[window], U32, |o| {
            w(tk::menu_item_draw_gear_info(
                o,
                window,
                [local, width],
                &mut ui_remap,
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round28_spline_segments_match_the_original() {
    let Some(mut t) = setup_any(0x0028_a500) else {
        return;
    };
    let stack = t.b.stack_range().start;
    let local = stack + 0x200;
    let top = t.a.stack_range().end;
    let points = t.alloc(&[0; 0x400]);
    let out = t.alloc(&[0; 0x10]);
    // the VU0 helpers run as the originals on both sides: the result and
    // every VU0 register they leave are compared
    for round in 0..1200u32 {
        for k in 0..0x100u32 {
            let v = match t.rng.below(16) {
                0 => 0.0f32,
                1 => f32::from_bits(t.rng.next() & 0xbfff_ffff),
                _ => t.rng.float(-500.0, 500.0),
            };
            t.put(points + 4 * k, v.to_bits());
        }
        let vregs = t.sym(0x00c8_f788);
        t.scramble_at(vregs, 0x90);
        let vf00 = t.sym(0x00c0_a004);
        let wv = t.rng.pick(&[1.0f32, 1.0, 2.0, 0.5, -1.0, 0.0]);
        t.put(vf00 + 0xc, wv.to_bits());
        let tv = match t.rng.below(11) {
            // denormals: t / 2 rounds when stored
            10 => f32::from_bits(t.rng.below(0x0080_0000) | (t.rng.next() & 0x8000_0000)),
            0 => t.rng.pick(&[
                0.0f32,
                1.0,
                -0.0,
                0.5,
                f32::from_bits(0x7fc0_0000),
                f32::INFINITY,
            ]),
            1 => t.rng.float(-3.0, 3.0),
            _ => t.rng.float(0.0, 1.0),
        };
        let index = t.rng.below(0x3c);
        let position = t.rng.pick(&[0u32, 1, 0x8000_0000]);
        // the quadratic copies a never-written frame word: zero it on both
        let zero = vec![0u8; 0x4000];
        t.a.write_bytes(top - 0x4000, &zero);
        t.put(local + 12, 0u32);
        let quadratic = round % 2 == 0;
        let (name, f) = if quadratic {
            ("TkMtCalcSplineSegment_0088abd0", 0x0088_abd0)
        } else {
            ("TkMtCalcSpline3Segment_0088a500", 0x0088_a500)
        };
        t.twin(
            name,
            f,
            &[out, index, tv.to_bits(), points, position],
            VOID,
            |o| {
                if quadratic {
                    tk::tk_mt_calc_spline_segment(
                        o,
                        out,
                        index,
                        tv,
                        points,
                        position,
                        local,
                        &mut callback,
                    );
                } else {
                    tk::tk_mt_calc_spline3_segment(
                        o,
                        out,
                        index,
                        tv,
                        points,
                        position,
                        local,
                        &mut callback,
                    );
                }
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

/// [`seq_code_deref_args`] that also stores the word in the returned cell
/// through argument `out` (a float width).
fn deref_out_stub(t: &mut Twins, f: u32, log: u32, args: &[u8], out: u8) -> u32 {
    let cell = t.alloc(&[0; 4]);
    let mut code = seq_code_deref_args(log, f, 1, u32::MAX, args, cell);
    code.truncate(code.len() - 6);
    code.extend([0x8b, 0x54, 0x24, 4 + 4 * out, 0xa1]); // mov edx, [esp + 4 + 4 out]; mov eax, [cell]
    code.extend(cell.to_le_bytes());
    code.extend([0x89, 0x02, 0xc3]); // mov [edx], eax; ret
    jump_stub(t, f, &code);
    cell
}

#[test]
fn menu_round28_config_option_row_matches_the_original() {
    let Some(mut t) = setup_any(0x0028_bae0) else {
        return;
    };
    let stack = t.b.stack_range().start;
    let (title, option, width) = (stack + 0x400, stack + 0x500, stack + 0x600);
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let l = log.to_le_bytes();

    // GetDerefU32_008cbaa0(row): the 0x80-byte record of row & 15
    let recs = t.alloc(&[0; 0x800]);
    let mut code: Vec<u8> = seq_code_of(log, 0x008c_baa0, &[0], 0, &[]);
    code.truncate(code.len() - 6);
    #[rustfmt::skip]
    code.extend([
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x0f, 0xc1, 0xe0, 0x07][..], // mov eax, [esp + 4]; and eax, 15; shl eax, 7
        &[0x05], &recs.to_le_bytes(), &[0xc3],  // add eax, recs; ret
    ]
    .concat());
    jump_stub(&mut t, 0x008c_baa0, &code);
    // SaveDataGetLoc(id, buf): logs id, buf gets id ^ 0x5a5a0000 and a 0
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x0d][..], &l, &[0xc7, 0x01], &0x0064_12d0u32.to_le_bytes(),
        &[0x8b, 0x44, 0x24, 0x04, 0x89, 0x41, 0x04],  // [ecx + 4] = id
        &[0x83, 0x05], &l, &[0x08],
        &[0x8b, 0x54, 0x24, 0x08],                     // mov edx, [esp + 8]
        &[0x35, 0x00, 0x00, 0x5a, 0x5a, 0x89, 0x02],   // xor eax, 0x5a5a0000; mov [edx], eax
        &[0xc7, 0x42, 0x04, 0, 0, 0, 0, 0xc3],         // mov dword [edx + 4], 0; ret
    ]
    .concat();
    jump_stub(&mut t, 0x0064_12d0, &code);
    let texts = t.alloc_random(0x400);
    record_stub(&mut t, 0x008d_c9b0, log, &[0], texts, 4, None);
    let font = t.alloc(&[0; 4]);
    let code = seq_code_deref_args(log, 0x0090_5ab0, 1, u32::MAX, &[1, 2, 3, 4, 5], font);
    jump_stub(&mut t, 0x0090_5ab0, &code);
    let wcell = deref_out_stub(&mut t, 0x0090_5290, log, &[2, 3, 4], 1);
    let cells = [
        seq_stub(&mut t, 0x008c_b530, log, 2, &[]),
        seq_stub(&mut t, 0x0090_3ee0, log, 11, &[]),
        seq_stub(&mut t, 0x008c_0670, log, 3, &[]),
        seq_stub(&mut t, 0x008b_f830, log, 4, &[]),
        seq_stub(&mut t, 0x008a_aeb0, log, 1, &[]),
        seq_stub(&mut t, 0x008d_c970, log, 1, &[]),
        font,
    ];
    let (loc_row, cursor, rows) = (
        t.sym(tk::MENU_CONFIG_LOC_ROW.sym),
        t.sym(tk::MENU_CONFIG_CURSOR.sym),
        t.sym(tk::MENU_CONFIG_ROWS.sym),
    );
    for _ in 0..1500u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let wv = row_float(&mut t).to_bits();
        t.put(wcell, wv);
        let row = t
            .rng
            .pick(&[0u32, 1, 2, 3, 5, 7, 15, 0xffff_ffff, 0x8000_0003]);
        let rec = recs + (row & 15) * 0x80;
        let b = t.rng.bytes(0x80);
        t.put_bytes(rec, &b);
        let n = t.rng.pick(&[0i32, 1, 2, 3, 4, 5, -1]);
        t.put(rec + 4, n);
        let choice = t.rng.pick(&[0u32, 1, 2, 4, 0xffff_ffff]);
        t.put(rec + 8, choice);
        let on = t.rng.pick(&[0u32, 1, 0x100]);
        t.put(rec + 0xc, on);
        let lr = if t.rng.below(3) == 0 {
            row
        } else {
            t.rng.below(8)
        };
        t.put(loc_row, lr);
        let c = if t.rng.below(2) == 0 {
            row.to_le_bytes()[0].cast_signed()
        } else {
            t.rng.pick(&[0i8, 1, -1, 3])
        };
        t.put(cursor, c);
        let r = t.rng.pick(&[1i32, 2, 5, 7, 0, -3, 0x7fff_ffff]);
        t.put(rows, r);
        let (a0, a1) = (t.rng.next(), t.rng.next());
        t.twin(
            "MenuConfig_DrawOptionRow",
            0x008c_bae0,
            &[a0, a1, row],
            VOID,
            |o| {
                tk::menu_config_draw_option_row(
                    o,
                    row,
                    [title, option, width],
                    &mut ui_remap,
                    &mut callback,
                );
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round28_sel_items_window_ctrl_matches_the_original() {
    let Some(mut t) = setup_any(0x0028_0eb0) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let pads = [
        seq_stub(&mut t, 0x008b_e410, log, 0, &[]),
        seq_stub(&mut t, 0x008b_e470, log, 0, &[]),
        seq_stub(&mut t, 0x008b_e4b0, log, 0, &[]),
    ];
    seq_stub(&mut t, 0x008a_b270, log, 1, &[]);
    seq_stub(&mut t, 0x0088_6e80, log, 1, &[]);
    // the help drawer logs the cursor it would draw
    let l = log.to_le_bytes();
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x0d][..], &l, &[0xc7, 0x01], &0x008d_1a20u32.to_le_bytes(),
        &[0x8b, 0x44, 0x24, 0x04, 0x89, 0x41, 0x04],   // [ecx + 4] = window
        &[0x0f, 0xb7, 0x40, 0x48, 0x89, 0x41, 0x08],   // [ecx + 8] = u16 [window + 0x48]
        &[0x83, 0x05], &l, &[0x0c, 0xc3],
    ]
    .concat();
    jump_stub(&mut t, 0x008d_1a20, &code);
    let pick = t.alloc(&[0; 4]);
    let code = seq_code(log, 0x0001_c0de, 3, pick, &[]);
    let pick_fn = test_code(&mut t, &code);
    let bits = [1u16, 2, 0x20, 0x40, 0x1000, 0x2000, 0x4000, 0x8000];
    for round in 0..8000u32 {
        t.put(log, start);
        for (k, &c) in pads.iter().enumerate() {
            let v: u32 = if k == 0 {
                t.rng.pick(&[0, 1, 2, 3, 0x1_0001])
            } else {
                let mut v = 0u32;
                for _ in 0..t.rng.below(3) {
                    v |= u32::from(t.rng.pick(&bits));
                }
                if t.rng.below(8) == 0 { t.rng.next() } else { v }
            };
            t.put(c, v);
        }
        let r = t.rng.pick(&[0u32, 1, 2, 0xffff_ffff, 0x8000_0000]);
        t.put(pick, r);
        let w = t.alloc_random(0x80);
        let any = t.rng.below(16);
        let state = if round < 160 {
            round % 16
        } else {
            t.rng
                .pick(&[2u32, 3, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0x10, any])
        };
        t.put(w + 0x28, state);
        let n = t.rng.pick(&[0u16, 1, 2, 3, 7, 8, 20, 21, 0xffff]);
        t.put(w + 0x30, n);
        let cursor = if t.rng.below(4) == 0 {
            u16::try_from(t.rng.next() & 0xffff).unwrap_or(0)
        } else {
            u16::try_from(t.rng.below(u32::from(n) + 2)).unwrap_or(0)
        };
        t.put(w + 0x48, cursor);
        let top = i16::try_from(t.rng.below(12)).unwrap_or(0) - 2;
        t.put(w + 0x32, top);
        let vis = t.rng.pick(&[0u16, 1, 3, 4, 9]);
        t.put(w + 0x3a, vis);
        let timer = t.rng.pick(&[
            0u16, 0x1000, 0xf000, 0x333, 0x111, 0x200, 0xfe00, 0xfccd, 0xfeef, 0xfdde, 1, 0xffff,
        ]);
        t.put(w + 0x46, timer);
        let picked = t.rng.pick(&[0i8, 1, 2, 3, -1]);
        t.put(w + 0x45, picked);
        let sound = t.rng.pick(&[0u8, 1]);
        t.put(w + 0x37, sound);
        let cb = t.rng.pick(&[0, pick_fn]);
        t.put(w + 0x1c, cb);
        match round % 7 {
            // a row down at the list's end (odd counts)
            0 => {
                let n = t.rng.pick(&[3u16, 5, 7, 9]);
                t.put(w + 0x30, n);
                let v = t.rng.pick(&[1u16, 2, 3]);
                t.put(w + 0x48, n - v);
                t.put(w + 0x28, 3u32);
                t.put(pads[1], 0x4000u32);
                t.put(pads[2], 0u32);
            }
            // a page down with an empty list and a negative cursor
            1 => {
                let v = t.rng.pick(&[0u16, 1]);
                t.put(w + 0x30, v);
                let v = t.rng.pick(&[0xfffdu16, 0xfffa, 0xfff0]);
                t.put(w + 0x48, v);
                let v = t.rng.pick(&[-1i16, -2, -3]);
                t.put(w + 0x32, v);
                let v = t.rng.pick(&[1u16, 2]);
                t.put(w + 0x3a, v);
                t.put(w + 0x28, 3u32);
                t.put(pads[0], 2u32);
                t.put(pads[1], 2u32);
                t.put(pads[2], 0u32);
            }
            // a pick with the count at -1
            2 => {
                t.put(w + 0x28, 0xcu32);
                t.put(w + 0x45, -1i8);
            }
            _ => {}
        }
        t.twin("TkMn2CtrlSelItemsWindow", 0x008d_0eb0, &[w], VOID, |o| {
            tk::tk_mn2_ctrl_sel_items_window(o, w, &mut callback);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

/// `f` logging the arguments `args`, storing the word in the second
/// returned cell through argument `out` and returning the first's.
fn ret_out_stub(t: &mut Twins, f: u32, log: u32, args: &[u8], out: u8) -> (u32, u32) {
    let (cell, value) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x54, 0x24, 4 + 4 * out, 0xa1]; // mov edx, [esp + 4 + 4 out]; mov eax, [value]
    extra.extend(value.to_le_bytes());
    extra.extend([0x89, 0x02]); // mov [edx], eax
    let code = seq_code_of(log, f, args, cell, &extra);
    jump_stub(t, f, &code);
    (cell, value)
}

/// `f` logging the arguments `args` and returning the word of `base`
/// selected by the low byte of argument `k`.
fn word_table_stub(t: &mut Twins, f: u32, log: u32, args: &[u8], k: u8, base: u32) {
    let unused = t.alloc(&[0; 4]);
    let mut code = seq_code_of(log, f, args, unused, &[]);
    code.truncate(code.len() - 6);
    #[rustfmt::skip]
    code.extend([
        &[0x0f, 0xb6, 0x44, 0x24, 4 + 4 * k][..], // movzx eax, byte [esp + 4 + 4k]
        &[0x8b, 0x04, 0x85], &base.to_le_bytes(), // mov eax, [eax * 4 + base]
        &[0xc3],
    ].concat());
    jump_stub(t, f, &code);
}

/// `n` bytes from `set`.
fn bytes_of(t: &mut Twins, n: usize, set: &[u8]) -> Vec<u8> {
    (0..n).map(|_| t.rng.pick(set)).collect()
}

#[test]
fn menu_round29_build_item_list_matches_the_original() {
    let Some(mut t) = setup_any(0x0029_2390) else {
        return;
    };
    let local = t.b.stack_range().start + 0x200;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    // every callee logged; results from cells or from tables by an id's
    // low byte; the record getters write their out word (a stack address)
    let (commands, _) = ret_out_stub(&mut t, 0x0079_0200, log, &[0], 1);
    let player = seq_stub(&mut t, 0x008a_9860, log, 0, &[]);
    let summon = seq_stub(&mut t, 0x008a_9880, log, 0, &[]);
    seq_stub(&mut t, 0x008a_8e20, log, 0, &[]);
    let max = seq_stub(&mut t, 0x008d_2d80, log, 4, &[]);
    let off = seq_stub(&mut t, 0x008d_2dc0, log, 4, &[]);
    let (has_cmd, panel) = (t.alloc(&[0; 0x100]), t.alloc(&[0; 0x100]));
    byte_table_stub(&mut t, 0x0078_5020, log, &[0, 1], 1, has_cmd);
    byte_table_stub(&mut t, 0x0078_50b0, log, &[0], 0, panel);
    let important = seq_stub(&mut t, 0x0079_0800, log, 1, &[]);
    let grow_max = seq_stub(&mut t, 0x008c_1c40, log, 0, &[]);
    let grow = seq_stub(&mut t, 0x008c_1c20, log, 0, &[]);
    let nums = t.alloc(&[0; 0x400]);
    word_table_stub(&mut t, 0x0079_04b0, log, &[0], 0, nums);
    let od = seq_stub(&mut t, 0x008c_1bf0, log, 1, &[]);
    let kaizou = seq_stub(&mut t, 0x008c_1b70, log, 0, &[]);
    let kaizou_max = seq_stub(&mut t, 0x008c_1b90, log, 0, &[]);
    let (abilities, weapons, items) = (
        t.alloc(&[0; 0x900]),
        t.alloc(&[0; 0x900]),
        t.alloc(&[0; 0x900]),
    );
    record_stub(&mut t, 0x0079_0970, log, &[0], abilities, 8, Some(1));
    let empty = seq_stub(&mut t, 0x008b_f720, log, 1, &[]);
    let (weapon_list, weapon_n) = ret_out_stub(&mut t, 0x007a_bd10, log, &[0, 1, 2], 3);
    record_stub(&mut t, 0x007a_bbd0, log, &[0], weapons, 8, Some(1));
    let (item_ids, item_n1) = ret_out_stub(&mut t, 0x0079_0530, log, &[], 0);
    let (item_nums, item_n2) = ret_out_stub(&mut t, 0x0079_0510, log, &[], 0);
    record_stub(&mut t, 0x008a_b280, log, &[0], items, 8, Some(1));
    let mask = seq_stub(&mut t, 0x008a_b1e0, log, 0, &[]);

    let list = t.sym(tk::MENU_ITEM_LIST.sym);
    let counts = t.sym(tk::MENU_ITEM_COUNTS.sym);
    let order = t.sym(tk::MN2_OVERDRIVE_ORDER.sym);
    let game_order: Vec<u8> = (0..20).map(|k| Mem::read::<u8>(&t.a, order + k)).collect();
    let (cmd_list, table, gear, ids) = (
        t.alloc(&[0; 0x400]),
        t.alloc(&[0; 0x200]),
        t.alloc(&[0; 0x20]),
        t.alloc(&[0; 0x400]),
    );
    let item_bytes = t.alloc(&[0; 0x200]);
    let who = [0u32, 1, 2, 3, 4, 5, 7, 8, 9, 0xc, 0xf, 0x10, 0xffff_ffff];
    for round in 0..1500u32 {
        t.put(log, start);
        t.scramble_at(list, 0x880);
        t.scramble_at(counts, 0x400);
        for g in [tk::MN2_LIST_MAX.sym, tk::MN2_NAME_MODE_MAX.sym] {
            let v = t.rng.next();
            let at = t.sym(g);
            t.put(at, v);
        }
        if round % 4 == 3 {
            let v: Vec<u8> = (0..20)
                .map(|_| t.rng.pick(&[0u8, 1, 5, 0x1f, 0x20, 0x3f, 0xff]))
                .collect();
            t.put_bytes(order, &v);
        } else {
            t.put_bytes(order, &game_order);
        }
        // commands
        t.put(commands, cmd_list);
        t.scramble_at(cmd_list, 0x400);
        let v = small_bytes(&mut t, 0x100);
        t.put_bytes(has_cmd, &v);
        let v = small_bytes(&mut t, 0x100);
        t.put_bytes(panel, &v);
        let p = t.rng.pick(&who);
        t.put(player, p);
        let s = t.rng.pick(&who);
        t.put(summon, s);
        let n = t.rng.pick(&[-3i32, 0, 1, 2, 5, 20, 60, 0x7fff_ffff]);
        t.put(max, n.min(60));
        let o = t.rng.below(0x40);
        t.put(off, o);
        // grow and kaizou tables (8-byte records), counts and abilities
        t.scramble_at(table, 0x200);
        let v = bytes_of(&mut t, 0x200, &[0, 1, 2, 3, 4, 5, 0xfe, 0xff, 0x41]);
        if round % 2 == 0 {
            t.put_bytes(table, &v);
        }
        for k in 0..0x100u32 {
            let n = t
                .rng
                .pick(&[0u32, 0, 1, 2, 3, 5, 99, 0xffff_ffff, 0x8000_0000, 0x100]);
            t.put(nums + 4 * k, n);
        }
        t.put(grow, table);
        t.put(kaizou, table);
        let n = t.rng.pick(&[-1i32, 0, 1, 2, 5, 12, 30, 63]);
        t.put(grow_max, n);
        let n = t.rng.pick(&[-1i32, 0, 1, 2, 5, 12, 30, 63]);
        t.put(kaizou_max, n);
        let imp = t.rng.pick(&[0u32, 0, 1, 0x8000_0000]);
        t.put(important, imp);
        let v = bytes_of(&mut t, 0x900, &[0, 1, 2, 3, 0xfe, 0xff]);
        t.put_bytes(abilities, &v);
        let v = small_bytes(&mut t, 0x20);
        t.put_bytes(gear, &v);
        for k in 0..4 {
            let a = t
                .rng
                .pick(&[0u16, 0xff, 1, 7, 0x20, 0x81, 0x1ff, 0x100, 0xffff]);
            t.put(gear + 0xe + 2 * k, a);
        }
        let e = t.rng.pick(&[0u32, 1, 2, 0x100]);
        t.put(empty, e);
        let m = t
            .rng
            .pick(&[0u32, 0xffff_ffff, 0x100, 0xff00, 0x8a00, 0x1_0000])
            ^ t.rng.next() & 0xff00;
        t.put(mask, m);
        let m = t.rng.next();
        t.put(od, m);
        // weapons and items
        t.put(weapon_list, ids);
        t.put(item_ids, ids);
        t.put(item_nums, item_bytes);
        let v: Vec<u8> = (0..0x400)
            .map(|k| {
                if k % 2 == 1 {
                    t.rng.pick(&[0u8, 0, 1, 2])
                } else {
                    u8::try_from(t.rng.next() & 0xff).unwrap_or(0)
                }
            })
            .collect();
        t.put_bytes(ids, &v);
        let v = bytes_of(&mut t, 0x200, &[0, 0, 1, 2, 0xff]);
        t.put_bytes(item_bytes, &v);
        let v = bytes_of(&mut t, 0x900, &[0, 1, 2, 3, 4, 8, 0xd, 0xff]);
        t.put_bytes(weapons, &v);
        let v = bytes_of(&mut t, 0x900, &[0, 1, 0x80, 0xc0, 0x81, 0x41, 0xc1, 0xff]);
        t.put_bytes(items, &v);
        let n = t.rng.pick(&[-2i32, 0, 1, 2, 3, 10, 60, 200]);
        t.put(weapon_n, n);
        let n = t.rng.pick(&[-2i32, 0, 1, 3, 10, 0x60, 0x1ff]);
        t.put(item_n1, n);
        let n = t.rng.pick(&[-2i32, 0, 1, 3, 10, 0x60, 0x1ff]);
        t.put(item_n2, n);
        let kind = match round % 3 {
            0 => round / 3 % 0x16,
            1 => t.rng.pick(&[0u32, 1, 2, 3, 5, 6, 7, 0x13, 0x14, 0x15]),
            _ => t
                .rng
                .pick(&[4u32, 8, 0x12, 0x16, 0x20, 0xffff_ffff, 0x8000_0000, 0x15]),
        };
        let arg = match kind {
            5 => gear,
            0x14 => t.rng.pick(&[0u32, 1, 2, 5, 0xffff_ffff]),
            _ => t.rng.next(),
        };
        t.twin(
            "TkMn2BuildItemList_008c2390",
            0x008c_2390,
            &[kind, arg],
            VOID,
            |o| {
                tk::tk_mn2_build_item_list(o, kind, arg, local, &mut callback);
                0
            },
        );
    }
    t.put_bytes(order, &game_order);
    eprintln!("{} calls", t.calls);
}

/// `copy: mov cl, [eax]; mov [edx], cl; inc eax; inc edx; test cl, cl;
/// jnz copy`, then `ret`: the string at `eax` copied to `edx`.
const COPY_RET: [u8; 11] = [
    0x8a, 0x08, 0x88, 0x0a, 0x40, 0x42, 0x84, 0xc9, 0x75, 0xf6, 0xc3,
];

/// With the log's write pointer in `ecx`: `n` raw bytes from argument `k`
/// appended (esi and edi kept).
fn raw_log(k: u8, n: u32) -> Vec<u8> {
    #[rustfmt::skip]
    let code = [
        &[0x56, 0x57][..],                          // push esi; push edi
        &[0x8b, 0x74, 0x24, 12 + 4 * k],            // mov esi, [esp + 12 + 4k]
        &[0x89, 0xcf, 0xb9], &n.to_le_bytes(),      // mov edi, ecx; mov ecx, n
        &[0xf3, 0xa4, 0x89, 0xf9, 0x5f, 0x5e],      // rep movsb; mov ecx, edi; pop edi; pop esi
    ]
    .concat();
    code
}

/// `mov ecx, [log]; [ecx] = id; ecx += 4`, then the words of `args`.
fn log_head(log: u32, id: u32, args: &[u8]) -> Vec<u8> {
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &log.to_le_bytes()].concat();
    code.extend([&[0xc7, 0x01][..], &id.to_le_bytes(), &[0x83, 0xc1, 0x04]].concat());
    for &k in args {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x01, 0x83, 0xc1, 0x04]);
    }
    code
}

/// `f(id, out)` logging `id` and `out`'s first `0x400` bytes, then copying
/// the text `base + (id & 3) * 0x100` to `out`.
fn loc_stub(t: &mut Twins, f: u32, log: u32, base: u32) {
    let mut code = log_head(log, f, &[0]);
    code.extend(raw_log(1, 0x400));
    #[rustfmt::skip]
    code.extend([
        &[0x89, 0x0d][..], &log.to_le_bytes(),      // mov [log], ecx
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x03, 0xc1, 0xe0, 0x08], // eax = (id & 3) << 8
        &[0x05], &base.to_le_bytes(),               // add eax, base
        &[0x8b, 0x54, 0x24, 0x08],                  // mov edx, [esp + 8]
        &COPY_RET,
    ].concat());
    jump_stub(t, f, &code);
}

/// `f(in, out)` logging the first `0x400` bytes of `in` and `out`,
/// then copying `in` without its first character (when it has one) to
/// `out`.
fn convert_stub(t: &mut Twins, f: u32, log: u32) {
    let mut code = log_head(log, f, &[]);
    code.extend(raw_log(0, 0x400));
    code.extend(raw_log(1, 0x400));
    #[rustfmt::skip]
    code.extend([
        &[0x89, 0x0d][..], &log.to_le_bytes(),      // mov [log], ecx
        &[0x8b, 0x44, 0x24, 0x04, 0x8b, 0x54, 0x24, 0x08], // eax = in, edx = out
        &[0x80, 0x38, 0x00, 0x74, 0x01, 0x40],      // cmp byte [eax], 0; je +1; inc eax
        &COPY_RET,
    ].concat());
    jump_stub(t, f, &code);
}

/// `f` logging the raw bytes of its arguments, `lens[k]` from argument
/// `k`; returns the result cell.
fn raw_stub(t: &mut Twins, f: u32, log: u32, lens: &[u32]) -> u32 {
    let cell = t.alloc(&[0; 4]);
    let mut code = log_head(log, f, &[]);
    for (k, &n) in (0u8..).zip(lens) {
        code.extend(raw_log(k, n));
    }
    code.extend([&[0x89, 0x0d][..], &log.to_le_bytes()].concat()); // mov [log], ecx
    code.extend([&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat()); // mov eax, [cell]; ret
    jump_stub(t, f, &code);
    cell
}

/// A text of `n` non-zero bytes and its zero.
fn text_of(t: &mut Twins, n: u32) -> Vec<u8> {
    let mut v: Vec<u8> = (0..n)
        .map(|_| u8::try_from(t.rng.below(255) + 1).unwrap_or(1))
        .collect();
    v.push(0);
    v
}

#[test]
fn menu_round29_pre_save_matches_the_original() {
    let Some(mut t) = setup_any(0x0029_50b0) else {
        return;
    };
    let local = t.b.stack_range().start + 0x200;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    // AtelGetSaveDic and MenuSave_PackSave bump the slot: the port must
    // read it again after each
    let slot = t.sym(tk::MENU_SAVE_SLOT.sym);
    let bump: Vec<u8> = [&[0xff, 0x05][..], &slot.to_le_bytes()].concat(); // inc dword [slot]
    let dic = seq_stub(&mut t, 0x0086_c410, log, 0, &bump);
    let event = seq_stub(&mut t, 0x0088_c830, log, 1, &[]);
    let time = seq_stub(&mut t, 0x008b_3e60, log, 3, &bump);
    seq_stub(&mut t, 0x008b_42e0, log, 3, &[]);
    let texts = t.alloc(&[0; 0x400]);
    loc_stub(&mut t, 0x0064_7f30, log, texts);
    convert_stub(&mut t, 0x0064_8150, log);
    convert_stub(&mut t, 0x0064_5f30, log);
    let name = seq_stub(&mut t, 0x0086_c430, log, 2, &[]);
    raw_stub(&mut t, 0x0064_86c0, log, &[0x40, 0x80, 0x200]);
    let icon = seq_stub(&mut t, 0x0064_8920, log, 1, &[]);
    let names = t.alloc(&[0; 0x100]);
    let buffer = t.sym(tk::SAVE_FILE_BUFFER.sym);
    let times = [
        0u32,
        59,
        60,
        61,
        3599,
        3600,
        3601,
        359_999,
        3_599_999,
        3_600_000,
        3_600_059,
        0xffff_ffff,
        0x8000_0000,
    ];
    for round in 0..600u32 {
        t.put(log, start);
        // texts 0x4f (index 3) under 64 bytes, the others under 128
        for (k, max) in [(0u32, 0x100), (1, 0x80), (2, 0x80), (3, 0x40)] {
            let n = match t.rng.below(4) {
                0 => 0,
                1 => max - 1,
                _ => t.rng.below(max),
            };
            let v = text_of(&mut t, n);
            t.put_bytes(texts + k * 0x100, &v);
        }
        let n = t.rng.pick(&[0u32, 1, 2, 0x20, 0x7f]);
        let v = text_of(&mut t, n);
        t.put_bytes(names, &v);
        t.put(name, names);
        let packed = t.rng.pick(&[0u32, 0, 1, 0x100]);
        t.put(t.sym(tk::MENU_PRE_SAVE_PACKED.sym), packed);
        for g in [tk::MENU_PRE_SAVE_DIC.sym, tk::MENU_SAVE_SLOT.sym] {
            let v = t.rng.next();
            t.put(t.sym(g), v);
        }
        t.put(t.sym(tk::SAVE_DATA_LOAD_RESULT.sym), 0u8);
        let s = if round < 26 {
            times[round as usize % times.len()]
        } else if round % 2 == 0 {
            t.rng.pick(&times)
        } else {
            t.rng.next()
        };
        let s2 = t.rng.pick(&times) ^ (t.rng.next() & 0xffff);
        let (packed_time, own_time) = if t.rng.below(2) == 0 {
            (s, s2)
        } else {
            (s2, s)
        };
        t.put(buffer + 0x10, packed_time);
        t.put(time, own_time);
        let id = t.rng.next();
        t.put(buffer + 0x18, id);
        reroll(&mut t, &[dic, event, icon]);
        // garbage in the port's frame: what it leaves unwritten shows
        let v = t.rng.bytes(0x16cc);
        Mem::write_bytes(&mut t.b, local, &v);
        t.twin("TkMenuPreSave", 0x008b_50b0, &[], U32, |o| {
            w(tk::tk_menu_pre_save(o, local, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round29_pre_save_time() {
    assert_eq!(tk::pre_save_time(0), "000:00:00");
    assert_eq!(tk::pre_save_time(3_599_999), "999:59:59");
    assert_eq!(tk::pre_save_time(3_600_000), "999:59:59");
    assert_eq!(tk::pre_save_time(3723), "001:02:03");
}

#[test]
fn menu_round29_list_wrappers_config_rows_and_btl_end_match_the_original() {
    let Some(mut t) = setup_any(0x0029_0003) else {
        return;
    };
    let local = t.b.stack_range().start + 0x200;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    seq_stub(&mut t, 0x008c_2390, log, 2, &[]);
    seq_stub(&mut t, 0x008c_2380, log, 2, &[]);
    let list_max = seq_stub(&mut t, 0x008c_1bb0, log, 0, &[]);
    let records = t.alloc(&[0; 0x900]);
    record_stub(&mut t, 0x007a_bbd0, log, &[0], records, 8, Some(1));
    seq_stub(&mut t, 0x0088_9360, log, 2, &[]);
    let bg = seq_stub(&mut t, 0x008f_5c10, log, 0, &[]);
    seq_stub(&mut t, 0x008c_bae0, log, 3, &[]);
    let kick = counter_stub(&mut t, 0x008c_0cd0, 0x1234_5678);
    let saved = seq_stub(&mut t, 0x008b_da00, log, 1, &[]);
    let next = seq_stub(&mut t, 0x008b_da40, log, 1, &[]);
    let sp = seq_stub(&mut t, 0x008b_da50, log, 1, &[]);
    seq_stub(&mut t, 0x008b_e020, log, 2, &[]);
    let add = seq_stub(&mut t, 0x008b_a390, log, 2, &[]);
    let names = t.alloc(&[0; 0x400]);
    word_table_stub(&mut t, 0x007a_bdf0, log, &[0], 0, names);
    seq_stub(&mut t, 0x008b_8960, log, 3, &[]);
    seq_stub(&mut t, 0x008b_8980, log, 3, &[]);
    seq_stub(&mut t, 0x008b_8680, log, 1, &[]);
    let text = seq_stub(&mut t, 0x008b_da60, log, 1, &[]);
    let ids = t.alloc(&[0; 0x200]);
    t.put(t.sym(0x0186_aa38), ids);
    let rows = t.sym(tk::MENU_CONFIG_ROWS.sym);
    let slots = t.sym(0x0231_0f8e);
    for round in 0..400u32 {
        t.put(log, start);
        t.scramble_at(ids, 0x200);
        let v = bytes_of(&mut t, 0x900, &[0, 0, 1, 2, 4, 8, 0xd, 0xff]);
        t.put_bytes(records, &v);
        t.scramble_at(names, 0x400);
        reroll(&mut t, &[list_max, bg, add, text]);
        let kind = t.rng.pick(&[0u32, 1, 2, 5, 0x13, 0x14, 0xffff_ffff]);
        let other = t.rng.next();
        t.twin("TkMn2RefreshList", 0x008c_2360, &[kind], VOID, |o| {
            tk::tk_mn2_refresh_list(o, kind, &mut callback);
            0
        });
        t.twin(
            "MenuWeapon_BuildKindList",
            0x008c_fd20,
            &[other, kind],
            U32,
            |o| w(tk::menu_weapon_build_kind_list(o, kind, &mut callback)),
        );
        let k = t.rng.below(0x100);
        t.twin("MenuKaizou_BuildList", 0x008d_6dd0, &[k], VOID, |o| {
            tk::menu_kaizou_build_list(o, k, local, &mut callback);
            0
        });
        t.twin("Pad_InitWorks", 0x0088_9340, &[], VOID, |o| {
            tk::pad_init_works(o, &mut callback);
            0
        });
        let n = t.rng.pick(&[-1i32, 0, 1, 2, 5, 12]);
        t.put(rows, n);
        t.twin("MenuConfig_DrawRows", 0x008c_bab0, &[], U32, |o| {
            w(tk::menu_config_draw_rows(o, &mut callback))
        });
        // BtlEnd_AddAP: around the next level, sphere level 98 or not
        let s = t.rng.pick(&[0u32, 10, 100, 0x7fff_fff0, 0xffff_fff0]);
        t.put(saved, s);
        let ap = t.rng.pick(&[0u32, 1, 5, 50, 0x7fff_ffff, 0xffff_ffff]);
        let delta = t.rng.pick(&[-2i32, -1, 0, 1, 2, 60, -60]);
        let nx = s.wrapping_add(ap).wrapping_add(delta.cast_unsigned());
        let nx = if round % 5 == 0 { t.rng.next() } else { nx };
        t.put(next, nx);
        let l = t.rng.pick(&[0x62u32, 0x62, 0x61, 0x63, 0, 0x162]);
        t.put(sp, l);
        let (chr, mark) = (t.rng.below(8), t.rng.next());
        t.twin(
            "BtlEnd_AddAP_008bdaf0",
            0x008b_daf0,
            &[chr, ap, mark],
            U32,
            |o| w(tk::btl_end_add_ap(o, chr, ap, mark, &mut callback)),
        );
        // TkBtlEndCheckLegendWeapon: six slots, often only one in use
        for k in 0..6 {
            let id = t.rng.pick(&[0u16, 1, 2, 0x10, 0x8f, 0x1ff, 0x103]);
            t.put(slots + 2 * k, id);
        }
        t.twin("TkBtlEndCheckLegendWeapon", 0x008b_a5c0, &[], U32, |o| {
            w(tk::tk_btl_end_check_legend_weapon(o, local, &mut callback))
        });
    }
    let k = Mem::read::<u32>(&t.a, kick);
    assert!(k > 100, "{k} kicks");
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round29_list_windows_and_ap_step_match_the_original() {
    let Some(mut t) = setup_any(0x0029_0004) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    seq_stub(&mut t, 0x008c_2360, log, 1, &[]);
    seq_stub(&mut t, 0x008d_6dd0, log, 1, &[]);
    seq_stub(&mut t, 0x008c_fd20, log, 2, &[]);
    let step = seq_stub(&mut t, 0x008b_a3c0, log, 1, &[]);
    let rest = seq_stub(&mut t, 0x008b_da10, log, 1, &[]);
    let level = seq_stub(&mut t, 0x008b_da50, log, 1, &[]);
    seq_stub(&mut t, 0x008b_dfc0, log, 2, &[]);
    seq_stub(&mut t, 0x008b_daf0, log, 3, &[]);
    let base = t.sym(tk::MENU_WINDOWS.sym);
    for round in 0..300u32 {
        t.put(log, start);
        if round % 10 == 0 {
            t.scramble(tk::MENU_WINDOWS.sym, 0x98 * 32);
            t.scramble(tk::LIMIT_SCROLLS.sym, 0x80);
        }
        let free = t.rng.below(32);
        for j in 0..32 {
            t.put(base + j * 0x98 + 0x40, u8::from(j != free));
        }
        for g in [
            tk::MN2_LIST_MAX.sym,
            tk::MN2_NAME_MODE_MAX.sym,
            tk::MENU_WEAPON_KIND_COUNT.sym,
        ] {
            let v = t.rng.next();
            t.put(t.sym(g), v);
        }
        let arg = t.rng.pick(&[0u32, 1, 3, 7, 15, 0x100, 0xffff_ffff]);
        let kind = t.rng.next();
        match round % 7 {
            0 => t.twin("TkMn2RefreshWeaponShopSell", 0x008d_9550, &[], VOID, |o| {
                tk::tk_mn2_refresh_weapon_shop_sell(o, 0x13, &mut callback);
                0
            }),
            1 => t.twin("TkMn2RefreshWeaponShopSell2", 0x008d_9560, &[], VOID, |o| {
                tk::tk_mn2_refresh_weapon_shop_sell(o, 0x15, &mut callback);
                0
            }),
            2 => t.twin("MenuWShop_OpenSellWindow", 0x008d_a950, &[], U32, |o| {
                w(tk::menu_wshop_open_sell_window(o, &mut callback))
            }),
            3 => t.twin("MenuKaizou_OpenListWindow", 0x008d_6ce0, &[arg], U32, |o| {
                w(tk::menu_kaizou_open_list_window(o, arg, &mut callback))
            }),
            4 => t.twin(
                "MenuEquip_OpenWeaponWindow",
                0x008c_fc90,
                &[kind, arg],
                U32,
                |o| {
                    w(tk::menu_equip_open_weapon_window(
                        o,
                        kind,
                        arg,
                        &mut callback,
                    ))
                },
            ),
            5 => t.twin("MenuLimit_OpenModeWindow", 0x008d_0dd0, &[arg], U32, |o| {
                w(tk::menu_limit_open_mode_window(o, arg, &mut callback))
            }),
            _ => {
                let s = t
                    .rng
                    .pick(&[0u32, 1, 5, 100, 0x7fff_ffff, 0xffff_ffff, 0x8000_0000]);
                let r = t
                    .rng
                    .pick(&[0u32, 1, 5, 100, 0x7fff_ffff, 0xffff_ffff, 0x8000_0000]);
                let r = if t.rng.below(3) == 0 { s } else { r };
                t.put(step, s);
                t.put(rest, r);
                let l = t
                    .rng
                    .pick(&[0x62u32, 0x63, 0x64, 0, 0xffff_ffff, 0x8000_0000]);
                t.put(level, l);
                let (chr, mark) = (t.rng.below(8), t.rng.next());
                t.twin(
                    "BtlEnd_CountApStep_008ba4e0",
                    0x008b_a4e0,
                    &[chr, mark],
                    U32,
                    |o| w(tk::btl_end_count_ap_step(o, chr, mark, &mut callback)),
                );
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round29_gear_window_pack_key_cursor_and_ap_steps_match_the_original() {
    let Some(mut t) = setup_any(0x0029_0005) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let refresh = seq_stub(&mut t, 0x008d_9560, log, 0, &[]);
    seq_stub(&mut t, 0x007a_b9f0, log, 2, &[]);
    // Menu_FindFlagged(list, from, to) logged, then searched as the original
    let mut code = log_head(log, 0x008c_9b90, &[1, 2]);
    #[rustfmt::skip]
    code.extend([
        &[0x89, 0x0d][..], &log.to_le_bytes(),          // mov [log], ecx
        &[0x8b, 0x54, 0x24, 0x04, 0x8b, 0x44, 0x24, 0x08], // edx = list, eax = from
        &[0x3b, 0x44, 0x24, 0x0c, 0x7d, 0x0a],          // next: cmp eax, to; jge none
        &[0x80, 0x7c, 0x82, 0x02, 0x00, 0x75, 0x06],    // cmp byte [edx + 4 eax + 2], 0; jne found
        &[0x40, 0xeb, 0xf0],                            // inc eax; jmp next
        &[0x83, 0xc8, 0xff, 0xc3],                      // none: or eax, -1; found: ret
    ]
    .concat());
    jump_stub(&mut t, 0x008c_9b90, &code);
    let key_rows = seq_stub(&mut t, 0x008c_5180, log, 0, &[]);
    let cursor = seq_stub(&mut t, 0x008c_0580, log, 3, &[]);
    let steps = t.alloc(&[0; 0x400]);
    word_table_stub(&mut t, 0x008b_a4e0, log, &[0, 1], 0, steps);
    let base = t.sym(tk::MENU_WINDOWS.sym);
    let list = t.sym(tk::MENU_ITEM_LIST.sym);
    let (cell, off) = (t.sym(tk::MENU_ITEM_CELL.sym), t.sym(0x0186_a270));
    let btl = t.sym(tk::BTL_END_LIST.sym);
    for round in 0..800u32 {
        t.put(log, start);
        reroll(&mut t, &[refresh, cursor]);
        let n = t.rng.pick(&[-1i32, 0, 1, 2, 5, 12, 30, 60]);
        t.put(t.sym(tk::MN2_NAME_MODE_MAX.sym), n);
        match round % 4 {
            0 => {
                if round % 40 == 0 {
                    t.scramble(tk::MENU_WINDOWS.sym, 0x98 * 32);
                }
                let free = t.rng.below(32);
                for j in 0..32 {
                    t.put(base + j * 0x98 + 0x40, u8::from(j != free));
                }
                t.put(t.sym(tk::MENU_ITEM_GEAR_MODE.sym), 1u8);
                t.twin("MenuItem_OpenGearWindow", 0x008c_9330, &[], U32, |o| {
                    w(tk::menu_item_open_gear_window(o, &mut callback))
                });
            }
            1 => {
                t.scramble_at(list, 0x100);
                for k in 0..0x40u32 {
                    let v = t.rng.pick(&[0u8, 0, 1, 2, 0xff]);
                    t.put(list + 4 * k + 2, v);
                }
                t.twin("MenuItem_PackGear", 0x008c_a120, &[], U32, |o| {
                    w(tk::menu_item_pack_gear(o, &mut callback))
                });
            }
            2 => {
                let c = if round < 200 {
                    (round / 4) % 50
                } else {
                    t.rng.pick(&[
                        0u32,
                        4,
                        5,
                        14,
                        15,
                        29,
                        44,
                        45,
                        0xffff_ffff,
                        0xffff_fff0,
                        0x8000_0000,
                    ])
                };
                t.put(cell, c);
                let o = t.rng.pick(&[0u32, 0, 0, 1, 0x100]);
                t.put(off, o);
                let r = t
                    .rng
                    .pick(&[0u32, 1, 6, 7, 9, 0xffff_ffff, 0x7fff_ffff, 0x0100_0001]);
                t.put(key_rows, r);
                let (x, y) = (
                    t.rng.pick(&[0i32, 100, -50, 1000]),
                    t.rng.pick(&[0i32, 300, -1, 700]),
                );
                let a0 = t.rng.next();
                t.twin(
                    "MenuItem_DrawKeyCursor",
                    0x008c_4b60,
                    &[a0, x.cast_unsigned(), y.cast_unsigned()],
                    U32,
                    |o| {
                        w(tk::menu_item_draw_key_cursor(
                            o,
                            [x, y],
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
            _ => {
                t.scramble_at(btl, 0x40);
                let n = t.rng.pick(&[0u8, 1, 2, 3, 4]);
                t.put(btl, n);
                for k in 0..0x100u32 {
                    let v = t.rng.pick(&[0u32, 0, 0, 1, 2]);
                    t.put(steps + 4 * k, v);
                }
                t.twin("BtlEnd_CountApSteps", 0x008b_a550, &[], U32, |o| {
                    w(tk::btl_end_count_ap_steps(o, &mut callback))
                });
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round29_window_callbacks_and_config_options_match_the_original() {
    let Some(mut t) = setup_any(0x0029_0006) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let choice = seq_stub(&mut t, 0x008d_b840, log, 0, &[]);
    let mut cells = vec![choice];
    for (f, n) in [
        (0x008b_44b0u32, 1u8),
        (0x008d_1a20, 1),
        (0x008d_0eb0, 1),
        (0x0090_0f60, 2),
        (0x008d_5750, 2),
        (0x008c_c170, 1),
        (0x0087_2480, 1),
        (0x0087_24f0, 1),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let ev = seq_stub(&mut t, 0x0078_5240, log, 0, &[]);
    let menu = seq_stub(&mut t, 0x0078_5290, log, 0, &[]);
    let block = t.alloc(&[0; 0x600]);
    let flags = t.alloc(&[0; 4]);
    t.put(ev, block);
    t.put(menu, flags);
    let sub = t.alloc(&[0; 0x100]);
    t.put(t.sym(0x023c_c128), sub);
    let ids = t.alloc(&[0; 0x200]);
    t.put(t.sym(0x0186_aa38), ids);
    let win = t.alloc(&[0; 0x100]);
    for round in 0..600u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let sel = t.rng.pick(&[0u32, 0, 1, 0x100]);
        t.put(cells[5], sel);
        let c = t.rng.pick(&[0u32, 4, 5, 6, 0xffff_ffff, 0x105]);
        t.put(choice, c);
        t.scramble_at(win, 0x100);
        t.scramble_at(ids, 0x200);
        let row = t.rng.pick(&[0u8, 0, 1, 2, 0x7f, 0x80, 0xff]);
        t.put(win + 0x45, row);
        let row = t.rng.pick(&[0u8, 0, 1, 2, 0x7f, 0x80, 0xff]);
        t.put(sub + 0x45, row);
        let v = t.rng.pick(&[0u32, 0, 1, 2, 0x100, 0x8000_0000]);
        t.put(win + 8, v);
        t.scramble_at(flags, 4);
        let k = t.rng.below(0x100);
        match round % 10 {
            0 => t.twin("MenuTuto_ChoiceIs5", 0x008d_b860, &[], U32, |o| {
                w(tk::menu_tuto_choice_is5(o, &mut callback))
            }),
            1 => t.twin("MenuKaizou_ListWindowCtrl", 0x008d_5810, &[win], U32, |o| {
                w(tk::menu_kaizou_list_window_ctrl(o, win, &mut callback))
            }),
            2..=4 => {
                let which = round % 10 - 2;
                let (name, f) = [
                    ("MenuWShop_SellWindowCtrl", 0x008d_9df0),
                    ("MenuItem_GearWindowCtrl", 0x008c_7630),
                    ("MenuLimit_ModeWindowCtrl", 0x008d_0e90),
                ][which as usize];
                t.twin(name, f, &[win], U32, |o| {
                    w(tk::menu_list_window_ctrl_by_row(
                        o,
                        which,
                        win,
                        &mut callback,
                    ))
                });
            }
            5 => t.twin(
                "MenuSubCmd_SetBrush_008e4a60",
                0x008e_4a60,
                &[win],
                U32,
                |o| w(tk::menu_sub_cmd_set_brush(o, win, &mut callback)),
            ),
            6 => t.twin(
                "MenuKaizou_IsSelectable",
                0x008d_57a0,
                &[win, k],
                U32,
                |o| w(tk::menu_kaizou_is_selectable(o, k, &mut callback)),
            ),
            _ => {
                let which = round % 10 - 7;
                let (name, f) = [
                    ("MenuConfig_SetBgmType", 0x008c_b0b0),
                    ("MenuConfig_SetVibration", 0x008c_afa0),
                    ("MenuConfig_GetSoundMode", 0x008c_b430),
                ][which as usize];
                t.twin(name, f, &[win], U32, |o| {
                    w(tk::menu_config_option(o, which, win, &mut callback))
                });
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round29_config_draw_options_messages_and_buy_window_match_the_original() {
    let Some(mut t) = setup_any(0x0029_0007) else {
        return;
    };
    let local = t.b.stack_range().start + 0x200;
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let flags = t.alloc(&[0; 4]);
    let menu = seq_stub(&mut t, 0x0078_5290, log, 0, &[]);
    t.put(menu, flags);
    // TkGetFFXCofingAdrs flips bits 0 and 13: the port must read the
    // settings again after it
    let flip: Vec<u8> = [
        &[0x81, 0x35][..],
        &flags.to_le_bytes(),
        &0x2001u32.to_le_bytes(),
    ]
    .concat();
    seq_stub(&mut t, 0x0088_d5e0, log, 0, &flip);
    let mut cells = Vec::new();
    for (f, n) in [
        (0x008c_2360u32, 1u8),
        (0x008d_ce50, 0),
        (0x008c_1790, 0),
        (0x008c_0be0, 0),
        (0x008c_bab0, 1),
        (0x008c_0cc0, 1),
        (0x008b_8ca0, 1),
        (0x008b_44b0, 1),
        (0x008c_1b30, 1),
        (0x008a_aeb0, 1),
        (0x0087_24a0, 1),
        (0x0087_24f0, 1),
        (0x0087_2480, 0),
        (0x0088_8bc0, 2),
        (0x0077_87b0, 0),
        (0x0077_7e90, 0),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let (word518, status, actuators) = (cells[14], cells[15], cells[13]);
    let excel = seq_stub(&mut t, 0x008c_1af0, log, 2, &[]);
    let rec = seq_stub(&mut t, 0x008c_baa0, log, 1, &[]);
    let width = out_stub(&mut t, log, 0x008b_9650, &[0, 1, 3, 4, 5], 2);
    let (data, rec_mem, m) = (
        t.alloc(&[0; 0x400]),
        t.alloc(&[0; 0x40]),
        t.alloc(&[0; 0x200]),
    );
    t.put(excel, data);
    t.put(rec, rec_mem);
    let sub = t.alloc(&[0; 0x100]);
    t.put(t.sym(0x023c_c128), sub);
    for round in 0..900u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let v = t.rng.pick(&[0u32, 1, 2, 3, 0x8000_0000]);
        t.put(word518, v);
        let v = t.rng.pick(&[0u32, 1, 2, 0x101]);
        t.put(status, v);
        let v = t.rng.pick(&[0u32, 0, 1, 2]);
        t.put(actuators, v);
        t.scramble_at(m, 0x200);
        t.scramble_at(data, 0x400);
        t.scramble_at(rec_mem, 0x40);
        t.scramble_at(flags, 4);
        t.scramble(0x0186_a610, 8);
        t.scramble(0x0186_ac20, 0x10);
        let v = t.rng.next();
        t.put(t.sym(tk::MN2_NAME_MODE_MAX.sym), v);
        let v = t.rng.pick(&[0u32, 0, 1, 2, 0x100]);
        t.put(m + 8, v);
        let v = t.rng.pick(&[0u16, 1, 1, 2, 0x101]);
        t.put(m + 0x12, v);
        let row = t.rng.pick(&[0u8, 1, 2, 0x80, 0xff]);
        t.put(sub + 0x45, row);
        let cursor = t.rng.pick(&[0u16, 1, 5, 0x1ff, 0xffff]);
        t.put(m + 0x48, cursor);
        let wv = t.rng.pick(&[
            0f32,
            1.5,
            -2.7,
            300.9,
            65535.9,
            1e10,
            -1e10,
            f32::from_bits(0x7fc0_0000),
        ]);
        t.put(width, wv.to_bits());
        match round % 9 {
            0 => t.twin("MenuSummon_GrowWindowInit", 0x008c_d2e0, &[m], U32, |o| {
                w(tk::menu_summon_grow_window_init(o, m, &mut callback))
            }),
            1 => t.twin("MenuConfig_Draw", 0x008c_b320, &[m], U32, |o| {
                w(tk::menu_config_draw(o, m, &mut callback))
            }),
            2 => t.twin("TkMenuMessageCb_008e02c0", 0x008e_02c0, &[m], U32, |o| {
                w(tk::tk_menu_message_cb(o, m, &mut callback))
            }),
            3 => t.twin("TkMenuMessageWidth_008de210", 0x008d_e210, &[m], U32, |o| {
                w(tk::tk_menu_message_width(o, m, local, &mut callback))
            }),
            4 => t.twin("MenuWShop_BuyWindowCtrl", 0x008d_9da0, &[m], U32, |o| {
                w(tk::menu_wshop_buy_window_ctrl(o, m, &mut callback))
            }),
            k => {
                let which = (k - 5 + round / 9) % 5;
                let (name, f) = [
                    ("MenuConfig_SetHdd", 0x008c_b060),
                    ("MenuConfig_SetSoundMode", 0x008c_b460),
                    ("MenuConfig_GetVibration", 0x008c_af60),
                    ("MenuConfig_SetOption1", 0x008c_b4c0),
                    ("MenuConfig_GetHdd", 0x008c_b020),
                ][which as usize];
                t.twin(name, f, &[m], U32, |o| {
                    w(tk::menu_config_option2(o, which, m, &mut callback))
                });
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round29_status_start_summon_ctrl_shop_panels_match_the_original() {
    let Some(mut t) = setup_any(0x0029_0008) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    // TkMenuGetCurrentPlayerPos counts up: three different results
    let pos = t.alloc(&[0; 4]);
    let bump: Vec<u8> = [&[0xff, 0x05][..], &pos.to_le_bytes()].concat();
    let code = seq_code(log, 0x008a_9870, 0, pos, &bump);
    jump_stub(&mut t, 0x008a_9870, &code);
    let mut cells = vec![pos];
    for (f, n) in [
        (0x008d_3100u32, 0u8),
        (0x008d_4ed0, 2),
        (0x008d_3280, 0),
        (0x008b_44b0, 1),
        (0x008b_ee90, 1),
        (0x008a_aeb0, 1),
        (0x008d_ce50, 0),
        (0x008c_1790, 0),
        (0x008c_0be0, 0),
        (0x008c_2d00, 5),
        (0x008b_f170, 7),
        (0x008c_0cc0, 1),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let excel = seq_stub(&mut t, 0x008c_1af0, log, 2, &[]);
    let record = seq_stub(&mut t, 0x008d_9170, log, 1, &[]);
    let (data, rec, w0) = (
        t.alloc(&[0; 0x400]),
        t.alloc(&[0; 0x10]),
        t.alloc(&[0; 0x100]),
    );
    t.put(excel, data);
    t.put(record, rec);
    let sub = t.alloc(&[0; 0x100]);
    t.put(t.sym(0x023c_c128), sub);
    for round in 0..600u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w0, 0x100);
        t.scramble_at(data, 0x400);
        t.scramble_at(rec, 0x10);
        t.scramble(0x0186_ab1c, 4);
        t.scramble(0x0186_aba0, 0xc);
        t.scramble(0x023c_c3bc, 0x10);
        let row = t.rng.pick(&[0u16, 1, 5, 9, 10, 0xffff, 0x8000, 0x1ff]);
        t.put(w0 + 0x48, row);
        let count = t.rng.pick(&[0u16, 1, 5, 10, 11, 0xffff]);
        t.put(w0 + 0x30, count);
        let b = t.rng.pick(&[0u8, 0, 1, 0xff]);
        t.put(w0 + 0x45, b);
        let b = t.rng.pick(&[0u8, 1, 2, 0x80, 0xff]);
        t.put(sub + 0x45, b);
        let r = t.rng.pick(&[0u16, 1, 2, 2, 3]);
        t.put(sub + 0x48, r);
        let a = t.rng.next();
        match round % 6 {
            0 => t.twin(
                "MenuStatus_Start_008d1d80",
                0x008d_1d80,
                &[w0, a],
                U32,
                |o| w(tk::menu_status_start(o, w0, a, &mut callback)),
            ),
            1 => t.twin(
                "MenuSummon_WindowCtrl_008cd6d0",
                0x008c_d6d0,
                &[w0],
                U32,
                |o| w(tk::menu_summon_window_ctrl(o, w0, &mut callback)),
            ),
            5 => t.twin("MenuWShop_StockWindowCtrl", 0x008d_77f0, &[w0], U32, |o| {
                w(tk::menu_wshop_stock_window_ctrl(o, w0, &mut callback))
            }),
            k => {
                let which = k - 2;
                let (name, f) = [
                    ("MenuWShop_DrawBuyPanel", 0x008d_7a40),
                    ("MenuWShop_DrawSellPanel", 0x008d_7cc0),
                    ("MenuShop_DrawBuySellPanel", 0x008d_9fe0),
                ][which as usize];
                t.twin(name, f, &[w0], U32, |o| {
                    w(tk::menu_shop_draw_panel(
                        o,
                        which,
                        w0,
                        &mut ui_remap,
                        &mut callback,
                    ))
                });
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round30_ctrls_draws_scroll_and_cd_reads_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0030_0001) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = vec![];
    for (f, n) in [
        (0x008d_ce50u32, 0u8),
        (0x008c_1790, 0),
        (0x008c_0be0, 0),
        (0x008b_f170, 7),
        (0x008c_0cc0, 1),
        (0x008b_44b0, 1),
        (0x008b_e410, 0),
        (0x008b_e470, 0),
        (0x008b_e4b0, 0),
        (0x008d_6d60, 0),
        (0x008a_ad70, 1),
        (0x008a_a100, 2),
        (0x008a_9860, 0),
        (0x008f_5c10, 0),
        (0x008e_71d0, 1),
        (0x008d_4ee0, 9),
        (0x008d_5180, 4),
        (0x008d_4910, 0),
        (0x008d_4170, 2),
        (0x008b_80b0, 2),
        (0x0083_c4a0, 1),
        (0x0062_f4c0, 1),
        (0x0088_d1d0, 2),
        (0x0081_d3f0, 3),
        (0x0081_d340, 3),
        (0x0068_7010, 1),
        (0x0083_b7b0, 3),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let lang = seq_stub(&mut t, 0x008a_c2f0, log, 0, &[]);
    let read_end = seq_stub(&mut t, 0x008a_8e50, log, 0, &[]);
    // the scroll clock moves +0xa0
    let tick_pos = t.alloc(&[0; 4]);
    let tick_extra: Vec<u8> = [
        &[0x8b, 0x4c, 0x24, 0x04, 0xa1][..], // mov ecx, [esp+4]; mov eax, [tick_pos]
        &tick_pos.to_le_bytes(),
        &[0x89, 0x81, 0xa0, 0, 0, 0], // mov [ecx+0xa0], eax
    ]
    .concat();
    cells.push(seq_stub(&mut t, 0x008d_f570, log, 1, &tick_extra));
    // MsGetSaveWeapon(id, &local): the local's address differs
    let weapon = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x007a_bbd0, &[0], weapon, &[]);
    jump_stub(&mut t, 0x007a_bbd0, &code);
    let get_cd = seq_stub(&mut t, 0x0076_d0d0, log, 0, &[]);
    let malloc = seq_stub(&mut t, 0x0068_7020, log, 1, &[]);
    // the CD object's methods +0, +4, +8, +0x10, +0x14
    let cd = t.alloc(&[0; 0x20]);
    let mut size_cell = 0;
    for (o, n) in [(0u32, 4u8), (4, 1), (8, 1), (0x10, 1), (0x14, 1)] {
        let cell = t.alloc(&[0; 4]);
        let code = seq_code(log, 0xcd00_0000 + o, n, cell, &[]);
        let at = test_code(&mut t, &code);
        t.put(cd + o, at);
        if o == 0x14 {
            size_cell = cell;
        } else {
            cells.push(cell);
        }
    }
    // the help menu's start (may change the mode) and step
    let mode = t.sym(0x0186_a234);
    let mode_cell = t.alloc(&[0; 4]);
    for (slot, id, extra) in [
        (
            0x00c4_16b8u32,
            0xe1e1_0000u32,
            [
                &[0xa1][..],
                &mode_cell.to_le_bytes(),
                &[0x66, 0xa3],
                &mode.to_le_bytes(),
            ]
            .concat(),
        ),
        (0x00c4_16ac, 0xe1e1_0001, vec![]),
    ] {
        let cell = t.alloc(&[0; 4]);
        cells.push(cell);
        let code = seq_code(log, id, 1, cell, &extra);
        let at = test_code(&mut t, &code);
        let s = t.sym(slot);
        t.put(s, at);
    }
    cells.push(mode_cell);
    let (rec, w0, sub, kaizou, text, buf) = (
        t.alloc(&[0; 0x10]),
        t.alloc(&[0; 0x200]),
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x10]),
        t.alloc(&[0; 0x10]),
    );
    t.put(get_cd, cd);
    t.put(malloc, buf);
    t.put(weapon, rec);
    t.put(t.sym(0x023c_c128), sub);
    t.put(t.sym(0x0186_aa30), kaizou);
    let local = t.b.stack_range().start + 0x100;
    for round in 0..900u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w0, 0x200);
        t.scramble_at(rec, 0x10);
        t.scramble_at(buf, 0x10);
        t.scramble_at(text, 0x10);
        t.scramble(0x0186_aba0, 4);
        t.scramble(0x0186_a22c, 0xc);
        t.scramble(0x0159_7770, 0x800);
        t.put(w0 + 0x98, text);
        let row = t.rng.pick(&[0u16, 1, 5, 0x1ff, 0xffff, 0x8000, 0x7fff]);
        t.put(w0 + 0x48, row);
        if t.rng.pick(&[false, true]) {
            let at = t.sym(0x0159_7770) + 4 * u32::from(row & 0x1ff);
            if row < 0x200 {
                t.put(at, 0u16);
            }
        }
        let b = t.rng.pick(&[0u8, 0, 1]);
        t.put(rec + 2, b);
        let b = t.rng.pick(&[0u8, 1, 2, 0x80, 0xff]);
        t.put(sub + 0x45, b);
        let r = t.rng.pick(&[0u16, 1, 2, 2, 3]);
        t.put(sub + 0x48, r);
        let b = t.rng.pick(&[0u8, 1, 0x7f, 0x80, 0xff]);
        t.put(kaizou + 0x45, b);
        let state = t.rng.pick(&[0u32, 1, 2, 3, 4, 5, 0x8000_0000]);
        t.put(w0 + 0x1c, state);
        let b = t.rng.pick(&[0u8, 0, 1]);
        t.put(text, b);
        let v = t.rng.pick(&[0u32, 1, 2, 6, 7, 15, 16, 18, 19, 0xffff_ffff]);
        t.put(lang, v);
        let v = t.rng.pick(&[0u32, 0, 1, 0x100]);
        t.put(read_end, v);
        let v = t.rng.pick(&[0u32, 0, 1, 0x1_0005, 0xffff]);
        t.put(mode, v);
        for at in [w0 + 0xa0, w0 + 0xa8, tick_pos] {
            let v = t
                .rng
                .pick(&[0i32, 1, 5, 6, 0x40, -1, -0x40, i32::MAX, i32::MIN]);
            t.put(at, v);
        }
        let v = t
            .rng
            .pick(&[0i32, 1, 0x7ff, 0x800, 0x801, -1, -0x801, i32::MAX, i32::MIN]);
        t.put(size_cell, v);
        let gs = t.sym(0x00b5_ec4f);
        let v = t.rng.pick(&[0u8, 0, 1, 0x80]);
        t.put(gs, v);
        let v = t.rng.pick(&[0u32, 0, 1, 0x40]);
        let load = v;
        match round % 9 {
            0 => t.twin("MenuShop_DrawSellPanel", 0x008d_a730, &[w0], U32, |o| {
                w(tk::menu_shop_draw_panel(
                    o,
                    3,
                    w0,
                    &mut ui_remap,
                    &mut callback,
                ))
            }),
            1 => t.twin("MenuWShop_SubWindow1Ctrl", 0x008d_79c0, &[w0], U32, |o| {
                w(tk::menu_wshop_sub_window1_ctrl(o, w0, local, &mut callback))
            }),
            2 => t.twin("TkMenuCtrlKaizou", 0x008d_5540, &[w0], VOID, |o| {
                tk::tk_menu_ctrl_kaizou(o, w0, &mut callback);
                0
            }),
            3 => t.twin("MenuStatus_DrawMainWindow", 0x008d_2710, &[w0], U32, |o| {
                w(tk::menu_status_draw_main_window(o, w0, &mut callback))
            }),
            4 => t.twin(
                "TOTextScrollInitLang_008de8c0",
                0x008d_e8c0,
                &[w0],
                U32,
                |o| w(tk::to_text_scroll_init_lang(o, w0, &mut callback)),
            ),
            5 => t.twin(
                "TOTextScrollStepLang_008de970",
                0x008d_e970,
                &[w0],
                U32,
                |o| w(tk::to_text_scroll_step_lang(o, w0, &mut callback)),
            ),
            6 => t.twin("TkMenuCtrlHelp", 0x008b_ef50, &[w0], U32, |o| {
                w(tk::tk_menu_ctrl_help(o, w0, &mut callback))
            }),
            7 => {
                let (file, size_of) = (t.rng.next(), t.rng.next());
                let min = t.rng.pick(&[0i32, 1, 0x800, -5, i32::MAX, i32::MIN]);
                t.twin(
                    "TkCdReadWithMalloc2",
                    0x0088_d240,
                    &[file, size_of, min.cast_unsigned()],
                    U32,
                    |o| {
                        w(tk::tk_cd_read_with_malloc2(
                            o,
                            file,
                            size_of,
                            min,
                            &mut callback,
                        ))
                    },
                );
            }
            _ => {
                let wave = round % 2 == 0;
                let (name, f) = if wave {
                    ("TkSqsLoadInfo_0088e440", 0x0088_e440)
                } else {
                    ("TkSqsLoadInfo_0088e3a0", 0x0088_e3a0)
                };
                t.twin(name, f, &[load], VOID, |o| {
                    tk::tk_sqs_load_info(o, wave, load, &mut callback);
                    0
                });
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

/// Code logging `id`, the `n` words argument `k` points to and then the
/// word arguments `args` (indices), returning `[cell]`.
fn seq_code_deref_at(log: u32, id: u32, k: u8, n: u8, args: &[u8], cell: u32) -> Vec<u8> {
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat(); // mov ecx, [log]
    code.extend([&[0xc7, 0x01][..], &id.to_le_bytes()].concat()); // mov dword [ecx], id
    code.extend([0x8b, 0x54, 0x24, 4 + 4 * k]); // mov edx, [esp + 4 + 4k]
    let mut at = 4u8;
    for i in 0..n {
        code.extend([0x8b, 0x42, 4 * i, 0x89, 0x41, at]); // mov eax, [edx + 4i]; mov [ecx + at], eax
        at += 4;
    }
    for &a in args {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * a, 0x89, 0x41, at]);
        at += 4;
    }
    code.extend([&[0x83, 0x05][..], &l, &[at]].concat()); // add dword [log], at
    code.extend([&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat()); // mov eax, [cell]; ret
    code
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round30_opens_ctrl_world_map_and_draws_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0030_0002) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    // TkMenuGetCurrentPlayerPos counts up: three different results
    let pos = t.alloc(&[0; 4]);
    let bump: Vec<u8> = [&[0xff, 0x05][..], &pos.to_le_bytes()].concat();
    let code = seq_code(log, 0x008a_9870, 0, pos, &bump);
    jump_stub(&mut t, 0x008a_9870, &code);
    cells.push(pos);
    for (f, n) in [
        (0x008a_9820u32, 1u8),
        (0x008a_9c70, 1),
        (0x008a_98c0, 1),
        (0x008a_9970, 1),
        (0x008c_fc30, 0),
        (0x008a_9860, 0),
        (0x008c_fd20, 2),
        (0x008c_2bf0, 1),
        (0x0088_ec60, 0),
        (0x008b_ef30, 0),
        (0x008f_dee0, 4),
        (0x008b_4b70, 5),
        (0x008f_9230, 4),
        (0x008b_37d0, 3),
        (0x008f_eb00, 3),
        (0x0090_2700, 7),
        (0x008b_ea20, 0),
        (0x008b_e410, 0),
        (0x008b_e470, 0),
        (0x008a_bab0, 3),
        (0x008b_0b10, 7),
        (0x008e_2d60, 0),
        (0x0088_6e80, 1),
        (0x008a_f4f0, 2),
        (0x008e_2dd0, 1),
        (0x008e_2b10, 0),
        (0x008a_ad70, 1),
        (0x008c_8e60, 1),
        (0x008c_6790, 0),
        (0x008c_8ba0, 0),
        (0x008c_9140, 1),
        (0x008c_2c10, 1),
        (0x008d_df40, 1),
        (0x008d_df20, 1),
        (0x008e_4ab0, 0),
        (0x008e_3b80, 1),
        (0x0090_3ee0, 11),
        (0x008d_ce50, 0),
        (0x008c_1790, 0),
        (0x008c_0be0, 0),
        (0x008d_4910, 0),
        (0x008d_4170, 2),
        (0x008c_2ce0, 1),
        (0x008d_33b0, 7),
        (0x008d_35a0, 7),
        (0x008c_0cc0, 0),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let trg = seq_stub(&mut t, 0x008b_e4b0, log, 0, &[]);
    let busy = seq_stub(&mut t, 0x008a_bb00, log, 0, &[]);
    let word64 = seq_stub(&mut t, 0x008e_2d50, log, 0, &[]);
    let mask = seq_stub(&mut t, 0x008a_a460, log, 0, &[]);
    // TkMenuMainGetMaxOptions stays the original: the caller keeps ecx
    // (the slide) across it; each drawn row takes one off the count
    let max = t.sym(0x0187_1688);
    // a drawn row moves the slide; the ease counter moves the face words
    let slide = t.sym(0x0187_1558);
    let bump: Vec<u8> = [
        &[0xff, 0x05][..],
        &slide.to_le_bytes(),
        &[0xff, 0x0d], // dec dword [max]
        &max.to_le_bytes(),
    ]
    .concat();
    cells.push(seq_stub(&mut t, 0x008e_22a0, log, 5, &bump));
    let face = t.sym(0x023c_c3c8);
    let bump: Vec<u8> = [&[0xff, 0x05][..], &face.to_le_bytes()].concat();
    cells.push(seq_stub(&mut t, 0x008b_f7e0, log, 5, &bump));
    // TkMenuOpenSubCmdWindowRect: the eight text and help words, the rest
    let cell = t.alloc(&[0; 4]);
    cells.push(cell);
    let code = seq_code_deref_at(log, 0x008e_4930, 1, 8, &[0, 3, 4, 5, 6, 7, 8], cell);
    jump_stub(&mut t, 0x008e_4930, &code);
    // TOGetMessageWH(id, text, &w, &h, line, flags): w and h from cells
    // (h only on a text byte with bit 0 set), the text one byte on
    let (wcell, hcell, next) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x15]; // mov eax, [esp+12]; mov edx, [wcell]
    extra.extend(wcell.to_le_bytes());
    // mov [eax], edx; mov eax, [esp+8]; test byte [eax], 1; jz over the h store
    extra.extend([
        0x89, 0x10, 0x8b, 0x44, 0x24, 0x08, 0xf6, 0x00, 0x01, 0x74, 0x0c,
    ]);
    extra.extend([0x8b, 0x44, 0x24, 0x10, 0x8b, 0x15]); // mov eax, [esp+16]; mov edx, [hcell]
    extra.extend(hcell.to_le_bytes());
    extra.extend([0x89, 0x10, 0x8b, 0x44, 0x24, 0x08, 0x40, 0xa3]); // mov [eax], edx; mov eax, [esp+8]; inc eax; mov [next], eax
    extra.extend(next.to_le_bytes());
    let code = seq_code_of(log, 0x008b_9650, &[0, 1, 4, 5], next, &extra);
    jump_stub(&mut t, 0x008b_9650, &code);
    // TOGetChangeMacroWidth: a float in ST0
    let (fcell, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let extra: Vec<u8> = [&[0xd9, 0x05][..], &fcell.to_le_bytes()].concat(); // fld dword [fcell]
    let code = seq_code_of(log, 0x008b_6a30, &[0, 1, 2], dummy, &extra);
    jump_stub(&mut t, 0x008b_6a30, &code);
    let (m0, text) = (t.alloc(&[0; 0x200]), t.alloc(&[0; 0x10]));
    let floats = [
        0f32,
        -0.0,
        1.5,
        -2.7,
        300.9,
        65535.9,
        1e10,
        -1e10,
        f32::from_bits(0x7fc0_0000),
    ];
    for round in 0..1200u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(m0, 0x200);
        t.scramble_at(text, 0x10);
        let n = t.rng.pick(&[0usize, 1, 2, 3, 5]);
        t.put(text + u32::try_from(n).unwrap(), 0u8);
        t.put(m0 + 0x90, text);
        let v = t.rng.pick(&[0u16, 1, 1, 2]);
        t.put(m0 + 0x22, v);
        let v = t.rng.pick(&floats);
        t.put(wcell, v.to_bits());
        let v = t.rng.pick(&floats);
        t.put(fcell, v.to_bits());
        let v = t.rng.pick(&[0u32, 1, 7, 0x7fff, 0x8000, 0xffff, 0x1_0005]);
        t.put(hcell, v);
        let state = t
            .rng
            .pick(&[0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0x8000_0000]);
        t.put(m0 + 0x1c, state);
        let v = t.rng.pick(&[0u32, 0x40, 0x1040, 0xbf]);
        t.put(trg, v);
        let v = t.rng.pick(&[0u32, 0, 1]);
        t.put(busy, v);
        let v = t.rng.pick(&[0u32, 0x80, 1, 0x180]);
        t.put(word64, v);
        let v = t.rng.pick(&[0u32, 0, 0x800, 0x8_0000, 0x7_f7ff]);
        t.put(mask, v);
        let v = t.rng.pick(&[0u32, 1, 3, 5, 0xffff_ffff]);
        t.put(max, v);
        let v = t.rng.pick(&[
            0i32,
            0,
            0x3000,
            1,
            -1,
            0x1000,
            i32::MIN,
            0x7fff_ffff,
            0x0100_0001,
            -0x0100_0003,
        ]);
        t.put(slide, v);
        let v = t.rng.pick(&[0u32, 1, 2, 4, 0xffff_ffff]);
        t.put(t.sym(0x0187_1548), v);
        let v = t
            .rng
            .pick(&[0u32, 1, 7, 8, 9, 0x11, 0xffff_ffff, 0x8000_0000]);
        t.put(t.sym(tk::MENU_ITEM_TARGET.sym), v);
        t.scramble(0x023c_c3bc, 0x10);
        t.scramble(0x0186_a624, 4);
        let a = t.rng.pick(&[0u16, 1, 5, 0xffff]);
        t.put(m0 + 0x32, a);
        let b = t.rng.pick(&[a, a, 2, 0x8000]);
        t.put(m0 + 0x34, b);
        match round % 7 {
            0 => t.twin("MenuEquip_Open_008cf830", 0x008c_f830, &[m0], U32, |o| {
                w(tk::menu_equip_open(o, m0, &mut callback))
            }),
            1 => t.twin("TkMenuMessageSize_008de250", 0x008d_e250, &[m0], U32, |o| {
                w(tk::tk_menu_message_size(
                    o,
                    m0,
                    local,
                    &mut callback,
                    &mut fcallback,
                ))
            }),
            2 => t.twin("MenuSaveLoad_DrawSlotList", 0x008b_4a50, &[m0], U32, |o| {
                w(tk::menu_save_load_draw_slot_list(o, m0, &mut callback))
            }),
            3 => t.twin("TkMenuCtrlWorldMap", 0x008e_2960, &[m0], U32, |o| {
                w(tk::tk_menu_ctrl_world_map(o, m0, &mut callback))
            }),
            4 => t.twin("MenuItem_Open_008c5f20", 0x008c_5f20, &[m0], U32, |o| {
                w(tk::menu_item_open(
                    o,
                    m0,
                    local,
                    &mut ui_remap,
                    &mut callback,
                ))
            }),
            5 => t.twin("MenuMain_DrawCmdWindow", 0x008e_20f0, &[m0], U32, |o| {
                w(tk::menu_main_draw_cmd_window(
                    o,
                    &mut ui_remap,
                    &mut callback,
                ))
            }),
            _ => t.twin("MenuItem_DrawTargetFace", 0x008c_3ec0, &[m0], U32, |o| {
                w(tk::menu_item_draw_target_face(
                    o,
                    &mut ui_remap,
                    &mut callback,
                ))
            }),
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round30_gear_sort_btl_end_pages_ctrls_and_wshop_layout_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0030_0003) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x007a_b9f0u32, 2u8),
        (0x008f_5f70, 5),
        (0x008f_8bb0, 5),
        (0x008c_0a20, 5),
        (0x008b_da30, 0),
        (0x008f_9230, 4),
        (0x008e_67f0, 4),
        (0x008a_b1f0, 1),
        (0x008b_f750, 5),
        (0x008e_49b0, 0),
        (0x008a_8e50, 0),
        (0x0088_6e80, 1),
        (0x008a_af70, 0),
        (0x008a_afe0, 0),
        (0x008c_2b90, 2),
        (0x008c_2b50, 2),
        (0x008c_2ad0, 1),
        (0x008c_1a60, 6),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let name_max = seq_stub(&mut t, 0x008c_1bc0, log, 0, &[]);
    let pads = [0x008b_e410u32, 0x008b_e470, 0x008b_e4b0].map(|f| seq_stub(&mut t, f, log, 0, &[]));
    let players = seq_stub(&mut t, 0x008a_9b30, log, 0, &[]);
    let summons = seq_stub(&mut t, 0x008a_9bf0, log, 0, &[]);
    // MsGetSaveWeapon(id, &out): the record of id & 0xff in a table (the
    // out word's address differs, so only the id is logged)
    let records = t.alloc(&[0; 0x800]);
    let weapon = t.alloc(&[0; 4]);
    let mut extra = vec![
        0x8b, 0x44, 0x24, 0x04, 0x25, 0xff, 0, 0, 0, 0x8d, 0x04, 0xc5,
    ]; // mov eax, [esp+4]; and eax, 0xff; lea eax, [eax*8 + records]
    extra.extend(records.to_le_bytes());
    extra.push(0xa3); // mov [weapon], eax
    extra.extend(weapon.to_le_bytes());
    let code = seq_code_of(log, 0x007a_bbd0, &[0], weapon, &extra);
    jump_stub(&mut t, 0x007a_bbd0, &code);
    // the window's +0x1c callback (w, row, selected)
    let accept = t.alloc(&[0; 4]);
    let code = seq_code(log, 0xcb00_001c, 3, accept, &[]);
    let callback_at = test_code(&mut t, &code);
    let w0 = t.alloc(&[0; 0x100]);
    for round in 0..4000u32 {
        let kind = round % 6;
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w0, 0x100);
        t.scramble_at(records, 0x800);
        // armour (byte +5) in half the records, or nearly all (searches
        // that fail)
        let dense = t.rng.pick(&[false, false, true]);
        for k in 0..0x100 {
            let v = if dense {
                t.rng.pick(&[0u8, 1, 1, 1, 1, 1, 1, 0x80])
            } else {
                t.rng.pick(&[0u8, 0, 1, 0x80])
            };
            t.put(records + 8 * k + 5, v);
        }
        t.scramble(0x0159_7770, 0x800);
        t.scramble(0x0186_9f14, 4);
        t.scramble(0x0231_0f8e, 0xc);
        t.scramble(0x0186_a3ac, 0x10);
        t.scramble(0x023c_c2fa, 2);
        for k in 0..14 {
            let v = t.rng.pick(&[0u16, 1, 2, 3, 5]);
            t.put(t.sym(0x0186_a3e4) + 2 * k, v);
        }
        // past the groups' 70 entries: the flags the groups read are all
        // written (the rest of the frame buffer differs between the twins)
        let v = t.rng.pick(&[0x46u32, 0x50, 0x80]);
        t.put(name_max, v);
        for &p in &pads {
            let v = t
                .rng
                .pick(&[0u32, 4, 8, 0x20, 0x40, 0x1000, 0x4000, 0x1_0004, 0xffff]);
            t.put(p, v);
        }
        if kind == 3 || kind == 4 {
            // the input states: no trigger most of the time, pads alone
            let v = t.rng.pick(&[0u32, 0, 0, 0x20, 0x20, 0x40, 0x1_0020]);
            t.put(pads[2], v);
            let v = t
                .rng
                .pick(&[0u32, 4, 8, 0x1000, 0x1000, 0x4000, 0xc, 0x1_4000]);
            t.put(pads[1], v);
            let v = t.rng.pick(&[0u32, 4, 8, 0xc]);
            t.put(pads[0], v);
        }
        // one count for both lists, the cursor often at its end
        let count = t.rng.pick(&[0u16, 1, 2, 3, 7]);
        t.put(players, u32::from(count));
        t.put(summons, u32::from(count));
        let v = t.rng.pick(&[0u32, 0, 1, 5]);
        t.put(accept, v);
        let state = if t.rng.pick(&[false, true]) {
            t.rng.pick(&[3u32, 5])
        } else {
            t.rng
                .pick(&[0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13])
        };
        t.put(w0 + 0x28, state);
        let v = t.rng.pick(&[0u16, 0xfff, 0x1000, 0x1001, 0x8000]);
        t.put(w0 + 0x52, v);
        let v = t.rng.pick(&[
            0u16,
            1,
            1,
            count.wrapping_sub(1),
            count.wrapping_sub(1),
            count,
            0x1f,
            0x20,
            0xffff,
            0x7fff,
        ]);
        t.put(w0 + 0x48, v);
        let v = t.rng.pick(&[0u32, 1, 2, 3]);
        t.put(w0 + 0x58, v);
        let v = t.rng.pick(&[0, callback_at]);
        t.put(w0 + 0x1c, v);
        let v = t.rng.pick(&[0u8, 1, 1, 2]);
        t.put(t.sym(0x0186_a391), v);
        let v = t.rng.pick(&[0u16, 1, 2, 5, 0x8000, 0xffff]);
        t.put(t.sym(0x0186_a3a4), v);
        if kind == 4 && t.rng.pick(&[false, true]) {
            // the target cursor's moves and wraps
            t.put(w0 + 0x28, 5u32);
            t.put(pads[2], 0u32);
            let v = t.rng.pick(&[0x1000u32, 0x4000]);
            t.put(pads[1], v);
            t.put(t.sym(0x0186_a391), 1u8);
            let v = t.rng.pick(&[0u16, 0, 1, count.wrapping_sub(1), count]);
            t.put(w0 + 0x48, v);
        }
        let dx = t
            .rng
            .pick(&[0i32, 1, -1, 100, -300, 0x0100_0001, 0x0300_0001, i32::MIN]);
        match kind {
            0 => t.twin(
                "MenuItem_SortGearByKind_008ca1b0",
                0x008c_a1b0,
                &[],
                U32,
                |o| {
                    w(tk::menu_item_sort_gear_by_kind(
                        o,
                        local,
                        local + 0x400,
                        &mut callback,
                    ))
                },
            ),
            1 => t.twin(
                "BtlEnd_DrawGilPage_008bbb10",
                0x008b_bb10,
                &[dx.cast_unsigned()],
                U32,
                |o| {
                    w(tk::btl_end_draw_gil_page(
                        o,
                        dx,
                        &mut ui_remap,
                        &mut callback,
                    ))
                },
            ),
            2 => t.twin(
                "BtlEnd_DrawWeaponPage_008bbed0",
                0x008b_bed0,
                &[dx.cast_unsigned()],
                VOID,
                |o| {
                    tk::btl_end_draw_weapon_page(o, dx, local, &mut ui_remap, &mut callback);
                    0
                },
            ),
            3 => t.twin(
                "MenuSummon_CmdWindowCtrl_008cced0",
                0x008c_ced0,
                &[w0],
                U32,
                |o| w(tk::menu_summon_cmd_window_ctrl(o, w0, &mut callback)),
            ),
            4 => t.twin(
                "MenuItem_TargetWindowCtrl_008c7430",
                0x008c_7430,
                &[w0],
                U32,
                |o| w(tk::menu_item_target_window_ctrl(o, w0, &mut callback)),
            ),
            _ => t.twin("TkMn2GetWShopPos", 0x008d_9190, &[], U32, |o| {
                w(tk::tk_mn2_get_wshop_pos(o, &mut callback))
            }),
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round30_scrolling_lists_and_btl_end_items_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0030_0004) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008c_1af0u32, 2u8),
        (0x008e_71d0, 1),
        (0x008c_0f70, 4),
        (0x008d_8c30, 6),
        (0x008d_8ee0, 6),
        (0x008c_1370, 0),
        (0x008d_8bb0, 3),
        (0x008d_8e60, 3),
        (0x008e_6cc0, 7),
        (0x008d_ce50, 0),
        (0x008c_1790, 0),
        (0x008c_0be0, 0),
        (0x008c_9570, 1),
        (0x008c_99f0, 5),
        (0x008c_9940, 3),
        (0x008c_0cc0, 0),
        (0x008f_9230, 4),
        (0x008f_5f70, 5),
        (0x008f_8bb0, 5),
        (0x008e_6700, 4),
        (0x0090_5820, 6),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    // ToGetBtlEasyDigitWidth2(n, &width, scale, x): the width from a cell
    let (wcell, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15]; // mov eax, [esp+8]; mov edx, [wcell]
    extra.extend(wcell.to_le_bytes());
    extra.extend([0x89, 0x10]); // mov [eax], edx
    let code = seq_code_of(log, 0x0090_5230, &[0, 2, 3], dummy, &extra);
    jump_stub(&mut t, 0x0090_5230, &code);
    let (w0, sub) = (t.alloc(&[0; 0x100]), t.alloc(&[0; 0x100]));
    t.put(t.sym(0x023c_c128), sub);
    let floats = [
        0f32,
        -0.0,
        1.5,
        -2.7,
        300.9,
        65535.9,
        1e10,
        f32::from_bits(0x7fc0_0000),
    ];
    for round in 0..1600u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w0, 0x100);
        t.scramble(0x0186_ab1c, 4);
        t.scramble(0x0186_aba8, 4);
        t.scramble(0x0186_a37c, 4);
        t.scramble(0x0231_0f70, 0x20);
        let v = t.rng.pick(&[0i8, 1, 3, 6, 16, -1]);
        t.put(t.sym(0x0231_0f70), v);
        let a = t.rng.pick(&[0u16, 1, 5, 0x8000]);
        t.put(w0 + 0x32, a);
        let b = t.rng.pick(&[a, a, 2, 0xffff]);
        t.put(w0 + 0x34, b);
        let v = t.rng.pick(&[0u16, 1, 0x1000, 0x7fff, 0x8000, 0xfff0]);
        t.put(w0 + 0x46, v);
        let v = t.rng.pick(&[0u8, 1, 0x7f, 0x80, 0xff]);
        t.put(w0 + 0x45, v);
        let v = t.rng.pick(&[0u8, 1, 0x7f, 0x80, 0xff]);
        t.put(sub + 0x45, v);
        let v = t.rng.pick(&floats);
        t.put(wcell, v.to_bits());
        let dx = t.rng.pick(&[0i32, 1, -1, 100, -300, 0x0300_0001, i32::MIN]);
        match round % 4 {
            0 | 1 => {
                let sell = round % 4 == 1;
                let (name, f) = if sell {
                    ("MenuWShop_DrawSellList", 0x008d_7d20)
                } else {
                    ("MenuWShop_DrawBuyList", 0x008d_7aa0)
                };
                t.twin(name, f, &[w0], U32, |o| {
                    w(tk::menu_wshop_draw_list(
                        o,
                        sell,
                        w0,
                        &mut ui_remap,
                        &mut callback,
                    ))
                });
            }
            2 => t.twin("MenuItem_DrawGearList", 0x008c_87e0, &[w0], U32, |o| {
                w(tk::menu_item_draw_gear_list(
                    o,
                    w0,
                    &mut ui_remap,
                    &mut callback,
                ))
            }),
            _ => t.twin(
                "BtlEnd_DrawItemPage_008bbca0",
                0x008b_bca0,
                &[dx.cast_unsigned()],
                VOID,
                |o| {
                    tk::btl_end_draw_item_page(o, dx, local, &mut ui_remap, &mut callback);
                    0
                },
            ),
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round30_weapon_list_and_item_target_names_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0030_0005) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008c_0f70u32, 4u8),
        (0x008d_0250, 5),
        (0x008c_1370, 0),
        (0x008d_0100, 3),
        (0x008e_6cc0, 7),
        (0x008d_ce50, 0),
        (0x008c_1790, 0),
        (0x008c_0be0, 0),
        (0x008f_5f70, 5),
        (0x008b_edd0, 1),
        (0x0090_5ab0, 6),
        (0x0090_3bb0, 9),
        (0x008c_4a90, 3),
        (0x008c_0cc0, 0),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let table_cell = seq_stub(&mut t, 0x008c_4ff0, log, 0, &[]);
    let count = seq_stub(&mut t, 0x008c_5030, log, 0, &[]);
    // ToGetBtlEasyFontWidth(text, &width, ...): the width from a cell
    let (wcell, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15]; // mov eax, [esp+8]; mov edx, [wcell]
    extra.extend(wcell.to_le_bytes());
    extra.extend([0x89, 0x10]); // mov [eax], edx
    let code = seq_code_of(log, 0x0090_5290, &[0, 2, 3, 4], dummy, &extra);
    jump_stub(&mut t, 0x0090_5290, &code);
    let (w0, table) = (t.alloc(&[0; 0x100]), t.alloc(&[0; 0x40]));
    t.put(table_cell, table);
    let floats = [
        0f32,
        -0.0,
        1.5,
        -2.7,
        300.9,
        65535.9,
        1e10,
        f32::from_bits(0x7fc0_0000),
    ];
    // scroll offsets whose row offset rounds across an integer as an f32
    // (the original stores it as one before ftol; with the game's remap
    // scale no i16 offset does, so the list keeps only the defaults)
    let span = ui_remap(&mut t.b, true, 79.0) * 5.0;
    let mut scrolls = vec![0u16, 1, 0x1000, 0x7fff, 0x8000, 0xfff0];
    for k in i16::MIN..=i16::MAX {
        let exact = span * f64::from(k) * -0.000_244_140_625;
        #[allow(clippy::cast_possible_truncation)] // the f32 store
        let rounded = f64::from(exact as f32);
        if rounded.trunc().to_bits() != exact.trunc().to_bits() && scrolls.len() < 10 {
            scrolls.push(k.cast_unsigned());
        }
    }
    for round in 0..1200u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w0, 0x100);
        t.scramble_at(table, 0x40);
        let rows = t.rng.pick(&[0u32, 1, 3, 7]);
        t.put(table + 4 * rows, 0u16);
        let slot = t.rng.pick(&[0u16, 1, 2]);
        t.put(table + 2, slot);
        t.put(table + 6, 2u16);
        let v = t.rng.pick(&[0u32, 1, 2, 0x1_0002]);
        t.put(t.sym(0x0186_a260), v);
        let v = t.rng.pick(&[0i32, 1, 3, 7, -2, 0x0100_0001]);
        t.put(count, v);
        let a = t.rng.pick(&[0u16, 1, 5, 0x8000]);
        t.put(w0 + 0x32, a);
        let b = t.rng.pick(&[a, a, 2, 0xffff]);
        t.put(w0 + 0x34, b);
        let v = t.rng.pick(&scrolls);
        t.put(w0 + 0x46, v);
        let v = t.rng.pick(&floats);
        t.put(wcell, v.to_bits());
        if round % 2 == 0 {
            t.twin(
                "MenuWeapon_DrawList_008cf9f0",
                0x008c_f9f0,
                &[w0],
                U32,
                |o| {
                    w(tk::menu_weapon_draw_list(
                        o,
                        w0,
                        &mut ui_remap,
                        &mut callback,
                    ))
                },
            );
        } else {
            t.twin(
                "MenuItem_DrawTargetNames_008c4030",
                0x008c_4030,
                &[w0],
                U32,
                |o| {
                    w(tk::menu_item_draw_target_names(
                        o,
                        w0,
                        local,
                        &mut ui_remap,
                        &mut callback,
                    ))
                },
            );
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round30_sell_window_grow_ctrl_limit_open_and_layout_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0030_0006) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008d_ce50u32, 0u8),
        (0x008c_1790, 0),
        (0x008c_0be0, 0),
        (0x008e_71d0, 1),
        (0x008c_1af0, 2),
        (0x008c_0f70, 4),
        (0x008d_aea0, 5),
        (0x008c_1370, 0),
        (0x008e_6cc0, 7),
        (0x008d_ae00, 3),
        (0x008c_0cc0, 0),
        (0x008d_3280, 0),
        (0x008d_2660, 1),
        (0x008b_44b0, 1),
        (0x0088_6e80, 1),
        (0x008a_aeb0, 1),
        (0x008d_1a20, 1),
        (0x008d_3140, 4),
        (0x008d_2680, 1),
        (0x008a_9870, 0),
        (0x008a_ae80, 1),
        (0x008c_2360, 1),
        (0x008b_ee70, 1),
        (0x008b_ee50, 1),
        (0x008e_4ab0, 0),
        (0x008a_9860, 0),
        (0x008d_0d50, 1),
        (0x008e_3b80, 1),
        (0x008b_7f40, 1),
        (0x008b_80b0, 2),
        (0x008b_8cd0, 5),
        (0x008b_8d10, 6),
        (0x008b_9a60, 2),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let pads = [0x008b_e410u32, 0x008b_e470, 0x008b_e4b0].map(|f| seq_stub(&mut t, f, log, 0, &[]));
    let members = seq_stub(&mut t, 0x008a_9b50, log, 0, &[]);
    let mode = seq_stub(&mut t, 0x008c_1bd0, log, 1, &[]);
    let rows = seq_stub(&mut t, 0x008c_1bb0, log, 0, &[]);
    let window_mode = seq_stub(&mut t, 0x008a_c7b0, log, 0, &[]);
    let lang = seq_stub(&mut t, 0x008a_c2f0, log, 0, &[]);
    // TkMenuGetPlayerFromIndex2 counts up from a picked character
    let chr = t.alloc(&[0; 4]);
    let bump: Vec<u8> = [&[0xff, 0x05][..], &chr.to_le_bytes()].concat();
    let code = seq_code(log, 0x008a_9b00, 1, chr, &bump);
    jump_stub(&mut t, 0x008a_9b00, &code);
    // TkMenuOpenSubCmdWindowRect: the helps and texts (four words from
    // argument 2), then the rest
    let cell = t.alloc(&[0; 4]);
    cells.push(cell);
    let code = seq_code_deref_at(log, 0x008e_4930, 2, 4, &[0, 3, 4, 5, 6, 7, 8], cell);
    jump_stub(&mut t, 0x008e_4930, &code);
    // TOGetMessageWH(id, text, &w, &h, line, flags): w from a cell
    let (wcell, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x15]; // mov eax, [esp+12]; mov edx, [wcell]
    extra.extend(wcell.to_le_bytes());
    extra.extend([0x89, 0x10]); // mov [eax], edx
    let code = seq_code_of(log, 0x008b_9650, &[0, 1, 3, 4, 5], dummy, &extra);
    jump_stub(&mut t, 0x008b_9650, &code);
    let (w0, sub, text) = (
        t.alloc(&[0; 0x140]),
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x10]),
    );
    t.put(t.sym(0x023c_c128), sub);
    let floats = [
        0f32,
        -0.0,
        1.5,
        299.5,
        300.0,
        650.2,
        -2.7,
        1e10,
        f32::from_bits(0x7fc0_0000),
    ];
    for round in 0..2000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w0, 0x140);
        t.scramble_at(text, 0x10);
        t.scramble(0x0186_a720, 0x100);
        t.scramble(0x0159_7770, 0x40);
        t.scramble(0x025d_0b20, 0x40);
        let v = t.rng.pick(&[0u8, 0, 1]);
        t.put(text, v);
        let a = t.rng.pick(&[0u16, 1, 5, 0x8000]);
        t.put(w0 + 0x32, a);
        let b = t.rng.pick(&[a, a, 2, 0xffff]);
        t.put(w0 + 0x34, b);
        let v = t.rng.pick(&[0u8, 1, 0x7f, 0x80, 0xff]);
        t.put(sub + 0x45, v);
        let v = t.rng.pick(&[0u16, 1, 2, 3]);
        t.put(sub + 0x48, v);
        for &p in &pads {
            let v = t.rng.pick(&[0u32, 0x10, 0x10, 0x40, 0x50, 0x1_0010]);
            t.put(p, v);
        }
        let v = t.rng.pick(&[0u32, 1, 2, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        t.put(t.sym(0x0186_a610), v);
        let v = t.rng.pick(&[0u8, 0, 1]);
        t.put(w0 + 0x45, v);
        let v = t.rng.pick(&[0u16, 1, 5, 0xffff]);
        t.put(w0 + 0x48, v);
        let v = t.rng.pick(&[0u16, 1, 6]);
        t.put(w0 + 0x30, v);
        let v = t.rng.pick(&[0u32, 1, 3, 0xffff_ffff]);
        t.put(members, v);
        let v = t.rng.pick(&[0u32, 1, 2, 5]);
        t.put(chr, v);
        let v = t.rng.pick(&[0u32, 1, 6, 9, 20, 21, 21, 0xffff_ffff]);
        t.put(rows, v);
        let m = t.rng.pick(&[0u16, 3, 7]);
        t.put(mode, u32::from(m));
        let k = t.rng.pick(&[0u32, 3, 7, 8, 13, 15, 15, 19]);
        t.put(t.sym(0x0159_7770) + 4 * k, m);
        // the message
        for (o, e) in [(0x94u32, 0x98u32), (0x60, 0x64), (0x78, 0x7c)] {
            t.put(w0 + o, text);
            let v = t.rng.pick(&[0, text]);
            t.put(w0 + e, v);
        }
        let v = t.rng.pick(&[0u32, 0, 1]);
        t.put(w0 + 0xac, v);
        let v = t.rng.pick(&[0i16, 1, 3, 7]);
        t.put(w0 + 4, v);
        let v = t.rng.pick(&[0u32, 1, 1, 2]);
        t.put(window_mode, v);
        let v = t.rng.pick(&[0u32, 3, 1]);
        t.put(lang, v);
        let v = t.rng.pick(&floats);
        t.put(wcell, v.to_bits());
        let v = t.rng.pick(&[0x17u32, 0x17, 0x18]);
        t.put(t.sym(0x012f_bc40), v);
        let v = t.rng.pick(&[0x1c7u16, 0x1c7, 0]);
        t.put(w0 + 0x14, v);
        let v = t.rng.pick(&[0xc7u16, 0xc7, 0]);
        t.put(w0 + 0x16, v);
        let v = t.rng.pick(&[0u16, 1, 1, 2]);
        t.put(w0 + 0x1e, v);
        let a = t.rng.next();
        match round % 4 {
            0 => t.twin(
                "MenuWShop_DrawSellWindow_008da470",
                0x008d_a470,
                &[w0],
                U32,
                |o| {
                    w(tk::menu_wshop_draw_sell_window(
                        o,
                        w0,
                        &mut ui_remap,
                        &mut callback,
                    ))
                },
            ),
            1 => t.twin(
                "MenuSummon_GrowWindowCtrl_008cd460",
                0x008c_d460,
                &[w0],
                U32,
                |o| w(tk::menu_summon_grow_window_ctrl(o, w0, &mut callback)),
            ),
            2 => t.twin("MenuLimit_Open_008d0a20", 0x008d_0a20, &[w0, a], U32, |o| {
                w(tk::menu_limit_open(
                    o,
                    w0,
                    a,
                    local,
                    &mut ui_remap,
                    &mut callback,
                ))
            }),
            _ => t.twin(
                "TkMenuMessageLayout_008de370",
                0x008d_e370,
                &[w0],
                U32,
                |o| w(tk::tk_menu_message_layout(o, w0, local, &mut callback)),
            ),
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round30_btl_end_init_and_flip_window_ctrl_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0030_0007) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    // TkBtlEndGetText counts up: different texts
    let texts = t.alloc(&[0; 4]);
    let bump: Vec<u8> = [&[0xff, 0x05][..], &texts.to_le_bytes()].concat();
    let code = seq_code(log, 0x008b_da60, 1, texts, &bump);
    jump_stub(&mut t, 0x008b_da60, &code);
    cells.push(texts);
    for (f, n) in [
        (0x008b_deb0u32, 1u8),
        (0x008b_df10, 0),
        (0x007a_b910, 1),
        (0x008b_dfe0, 2),
        (0x008c_ac70, 3),
        (0x008a_aeb0, 1),
        (0x0088_6e80, 1),
        (0x008a_afa0, 0),
        (0x008a_af10, 0),
        (0x008a_afe0, 0),
        (0x008a_af70, 0),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let pads = [0x008b_e410u32, 0x008b_e470, 0x008b_e4b0].map(|f| seq_stub(&mut t, f, log, 0, &[]));
    let players = seq_stub(&mut t, 0x008a_9b30, log, 0, &[]);
    let summons = seq_stub(&mut t, 0x008a_9bf0, log, 0, &[]);
    let ratio = seq_stub(&mut t, 0x008a_b220, log, 0, &[]);
    // MsGetSaveWeaponList(a, b, c, &count): the count from a cell
    let (list_cell, ccell) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x10, 0x8b, 0x15]; // mov eax, [esp+16]; mov edx, [ccell]
    extra.extend(ccell.to_le_bytes());
    extra.extend([0x89, 0x10]); // mov [eax], edx
    let code = seq_code(log, 0x007a_bd10, 4, list_cell, &extra);
    jump_stub(&mut t, 0x007a_bd10, &code);
    // MsGetSaveWeapon(id, &out): the record of id & 0xff
    let records = t.alloc(&[0; 0x800]);
    let weapon = t.alloc(&[0; 4]);
    let mut extra = vec![
        0x8b, 0x44, 0x24, 0x04, 0x25, 0xff, 0, 0, 0, 0x8d, 0x04, 0xc5,
    ];
    extra.extend(records.to_le_bytes());
    extra.push(0xa3);
    extra.extend(weapon.to_le_bytes());
    // and the gear count shrinks (the list's count is read again)
    extra.extend([0xff, 0x0d]);
    extra.extend(t.sym(0x0186_a188).to_le_bytes());
    let code = seq_code_of(log, 0x007a_bbd0, &[0], weapon, &extra);
    jump_stub(&mut t, 0x007a_bbd0, &code);
    // MsGetSaveWeaponAmount(&out): the amount from a cell
    let (acell, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x15]; // mov eax, [esp+4]; mov edx, [acell]
    extra.extend(acell.to_le_bytes());
    extra.extend([0x89, 0x10]);
    let code = seq_code_of(log, 0x007a_bc60, &[], dummy, &extra);
    jump_stub(&mut t, 0x007a_bc60, &code);
    // the window's +0x1c callback
    let accept = t.alloc(&[0; 4]);
    let code = seq_code(log, 0xcb00_001c, 3, accept, &[]);
    let callback_at = test_code(&mut t, &code);
    let (list, w0) = (t.alloc(&[0; 0x80]), t.alloc(&[0; 0x100]));
    t.put(list_cell, list);
    for round in 0..3000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(list, 0x80);
        t.scramble_at(records, 0x800);
        for k in 0..0x100 {
            let v = t.rng.pick(&[0u8, 1, 1, 1]);
            t.put(records + 8 * k + 2, v);
        }
        t.scramble(0x0231_0f8e, 0xc);
        t.scramble(0x0186_a0b0, 0x120);
        let v = t.rng.pick(&[0i32, 1, 5, 9, 20, -1]);
        t.put(ccell, v);
        let v = t.rng.pick(&[0i8, 1, 3, 6, 7, -1, -2]);
        t.put(t.sym(0x0231_0f72), v);
        let v = t.rng.pick(&[0i32, 1, 3, 6, -1]);
        t.put(acell, v);
        // the flip window
        t.scramble_at(w0, 0x100);
        let state = if t.rng.pick(&[false, true]) {
            5
        } else {
            t.rng
                .pick(&[0u32, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 14, 15, 16, 17])
        };
        t.put(w0 + 0x28, state);
        let v = t.rng.pick(&[0x1000u16, 0x1000, 0xff0, 0xfff, 0, 0x8000]);
        t.put(w0 + 0x46, v);
        let v = t.rng.pick(&[0u16, 0, 0x10, 0x8000, 0xfff0]);
        t.put(w0 + 0x38, v);
        let rows = t.rng.pick(&[0u16, 1, 2, 5]);
        t.put(w0 + 0x30, rows);
        let v = t.rng.pick(&[0u16, 1, rows.wrapping_sub(1), 0xffff]);
        t.put(w0 + 0x48, v);
        let v = t.rng.pick(&[0u8, 0, 1, 1, 2]);
        t.put(w0 + 0x44, v);
        let v = t.rng.pick(&[0, callback_at]);
        t.put(w0 + 0x1c, v);
        let v = t.rng.pick(&[0u32, 0, 1]);
        t.put(accept, v);
        let v = t.rng.pick(&[0u32, 0x20, 0x40, 0, 0, 0x1_0020]);
        t.put(pads[2], v);
        let v = t.rng.pick(&[0u32, 4, 8, 0x8000, 0x2000, 0xc, 0x1_2000]);
        t.put(pads[1], v);
        let v = t.rng.pick(&[0u32, 4, 8, 0xc]);
        t.put(pads[0], v);
        let v = t.rng.pick(&[0u32, 1, 2, 3]);
        t.put(players, v);
        let v = t.rng.pick(&[0u32, 1, 2, 3]);
        t.put(summons, v);
        let v = t.rng.pick(&[0u32, 1, 1]);
        t.put(ratio, v);
        let kind = t.rng.next();
        if round % 2 == 0 {
            t.twin("TkBtlEndInit", 0x008b_db70, &[0, kind], U32, |o| {
                w(tk::tk_btl_end_init(o, kind, local, &mut callback))
            });
        } else {
            t.twin(
                "MenuSubCmd_FlipWindowCtrl_008e3880",
                0x008e_3880,
                &[w0],
                U32,
                |o| w(tk::menu_sub_cmd_flip_window_ctrl(o, w0, &mut callback)),
            );
        }
    }
    eprintln!("{} calls", t.calls);
}

/// Two calls of `f` give `[second]` then `[first]`: the stub swaps the
/// cells before it returns `[first]`.
fn swap_stub(t: &mut Twins, f: u32, log: u32, n: u8) -> (u32, u32) {
    let (first, second) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x15]; // mov edx, [first]
    extra.extend(first.to_le_bytes());
    extra.push(0xa1); // mov eax, [second]
    extra.extend(second.to_le_bytes());
    extra.push(0xa3); // mov [first], eax
    extra.extend(first.to_le_bytes());
    extra.extend([0x89, 0x15]); // mov [second], edx
    extra.extend(second.to_le_bytes());
    let code = seq_code(log, f, n, first, &extra);
    jump_stub(t, f, &code);
    (first, second)
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round31_save_work_sel_list_pad_scan_exchange_page_and_layout2_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0031_0001) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x0078_7470u32, 0u8),
        (0x0086_d4e0, 0),
        (0x0087_24f0, 0),
        (0x0078_5ed0, 1),
        (0x008c_0f70, 4),
        (0x008d_1620, 5),
        (0x008c_1370, 0),
        (0x008e_6cc0, 7),
        (0x008d_15a0, 3),
        (0x0088_88e0, 1),
        (0x0088_8910, 1),
        (0x0088_8f90, 3),
        (0x0088_9410, 1),
        (0x0062_fbd0, 3),
        (0x0088_8a70, 2),
        (0x0088_9460, 3),
        (0x0062_fbc0, 3),
        (0x008f_9230, 4),
        (0x008f_5f70, 5),
        (0x008f_8bb0, 5),
        (0x008e_67f0, 4),
        (0x008f_eb00, 3),
        (0x008b_7f40, 1),
        (0x008b_6660, 2),
        (0x008b_8d10, 6),
        (0x008b_9a60, 2),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let event = seq_stub(&mut t, 0x0078_5240, log, 0, &[]);
    let asia = seq_stub(&mut t, 0x0086_a7f0, log, 0, &[]);
    // TkGetControlerWork stays the original: the caller keeps ecx (the
    // port) across it for scePadGetState
    let state = seq_stub(&mut t, 0x0062_fb30, log, 2, &[]);
    let poll = seq_stub(&mut t, 0x0062_fb20, log, 2, &[]);
    let press = seq_stub(&mut t, 0x0088_9510, log, 4, &[]);
    let modes = swap_stub(&mut t, 0x0062_fb60, log, 4);
    let aligns = swap_stub(&mut t, 0x0088_8a40, log, 3);
    let timer = seq_stub(&mut t, 0x008a_9c60, log, 0, &[]);
    let above = seq_stub(&mut t, 0x008b_8140, log, 3, &[]);
    let words = seq_stub(&mut t, 0x008b_80b0, log, 2, &[]);
    let calm = seq_stub(&mut t, 0x0067_ab50, log, 1, &[]);
    let main_mode = seq_stub(&mut t, 0x0062_fbe0, log, 4, &[]);
    // MsGetSaveWeapon(id, &out): the record of id & 0xff
    let records = t.alloc(&[0; 0x800]);
    let weapon = t.alloc(&[0; 4]);
    let mut extra = vec![
        0x8b, 0x44, 0x24, 0x04, 0x25, 0xff, 0, 0, 0, 0x8d, 0x04, 0xc5,
    ];
    extra.extend(records.to_le_bytes());
    extra.push(0xa3);
    extra.extend(weapon.to_le_bytes());
    let code = seq_code_of(log, 0x007a_bbd0, &[0], weapon, &extra);
    jump_stub(&mut t, 0x007a_bbd0, &code);
    // TOGetMessageWH(id, text, &w, &h, line, flags): w and h from cells
    let (wcell, hcell, next) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x15]; // mov eax, [esp+12]; mov edx, [wcell]
    extra.extend(wcell.to_le_bytes());
    extra.extend([0x89, 0x10, 0x8b, 0x44, 0x24, 0x10, 0x8b, 0x15]); // mov [eax], edx; mov eax, [esp+16]; mov edx, [hcell]
    extra.extend(hcell.to_le_bytes());
    extra.extend([0x89, 0x10]); // mov [eax], edx
    let code = seq_code_of(log, 0x008b_9650, &[0, 1, 4, 5], next, &extra);
    jump_stub(&mut t, 0x008b_9650, &code);
    cells.push(next);
    // TOGetChangeMacroWidth: a float in ST0
    let (fcell, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let extra: Vec<u8> = [&[0xd9, 0x05][..], &fcell.to_le_bytes()].concat(); // fld dword [fcell]
    let code = seq_code_of(log, 0x008b_6a30, &[0, 1, 2], dummy, &extra);
    jump_stub(&mut t, 0x008b_6a30, &code);
    let (rec, w0, pad) = (t.alloc(&[0; 0x200]), t.alloc(&[0; 0x140]), t.alloc(&[0; 4]));
    let (dst, buffer) = (t.alloc(&vec![0; 0x6a00]), t.alloc(&vec![0; 0x6a00]));
    let floats = [
        0f32,
        -0.0,
        1.5,
        -2.7,
        299.5,
        65535.9,
        1e10,
        -1e10,
        f32::from_bits(0x7fc0_0000),
    ];
    for round in 0..3000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let kind = round % 6;
        match kind {
            0 => {
                if round % 60 == 0 {
                    t.scramble_at(buffer, 0x6a00);
                    let (a, b) = (dst, buffer);
                    t.twin("TkMenuCopyLoadedSave", 0x008b_54a0, &[a, b], U32, |o| {
                        w(tk::tk_menu_copy_loaded_save(o, a, b, &mut callback))
                    });
                }
                t.scramble_at(rec, 0x200);
                t.put(event, rec);
                let v = t.rng.pick(&[0u32, 0, 1, 2]);
                t.put(asia, v);
                t.twin("TkInitSaveWork", 0x0088_df40, &[], U32, |o| {
                    w(tk::tk_init_save_work(o, &mut callback))
                });
            }
            1 => {
                t.scramble_at(w0, 0x140);
                let a = t.rng.pick(&[0u16, 1, 5, 0x8000]);
                t.put(w0 + 0x32, a);
                let b = t.rng.pick(&[a, a, 2, 0xffff, a ^ 0x100]);
                t.put(w0 + 0x34, b);
                let v = t.rng.pick(&[0u16, 1, 0x1000, 0x7fff, 0x8000, 0xfff0]);
                t.put(w0 + 0x46, v);
                let v = t.rng.pick(&[0u8, 1, 0x7f, 0x80, 0xff]);
                t.put(w0 + 0x45, v);
                t.twin(
                    "MenuSelList_DrawList_008d1400",
                    0x008d_1400,
                    &[w0],
                    U32,
                    |o| {
                        w(tk::menu_sel_list_draw_list(
                            o,
                            w0,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
            2 => {
                let (p, s) = (t.rng.below(2), t.rng.below(4));
                let work = t.sym(0x0133_0288) + (p + s) * 0x100;
                t.scramble_at(work, 0x100);
                let v = t.rng.pick(&[0u8, 1, 1, 2, 3, 4, 5, 6, 7, 8, 0x20]);
                t.put(work + 0x9f, v);
                let v = t.rng.pick(&[0u8, 0, 1, 2]);
                t.put(work + 0x83, v);
                let v = t.rng.pick(&[0u32, 1, 2, 2, 3, 6, 6, 7, 8, 0xffff_ffff]);
                t.put(state, v);
                let v = t.rng.pick(&[0u32, 1, 1, 2]);
                t.put(poll, v);
                let v = t.rng.pick(&[0u32, 1, 3, 5, 5, 5, 7, 0x105]);
                t.put(press, v);
                for c in [modes.0, modes.1] {
                    let v = t.rng.pick(&[0u32, 1, 4, 7, 2, 0x1_0000, 0x1_0004]);
                    t.put(c, v);
                }
                for c in [aligns.0, aligns.1] {
                    let v = t.rng.pick(&[0u32, 0, 1, 8, 0x1ff]);
                    t.put(c, v);
                }
                let v = t.rng.pick(&[0u32, 1]);
                t.put(main_mode, v);
                let v = t.rng.next();
                t.put(pad, v);
                let v = t.rng.pick(&[0u8, 0, 1, 0x80]);
                t.put(pad, v);
                let (p, s) = (t.rng.below(2), t.rng.below(4));
                t.twin(
                    "Pad_ScanPort_00889050",
                    0x0088_9050,
                    &[p, s, pad],
                    U32,
                    |o| w(tk::pad_scan_port(o, p, s, pad, &mut callback)),
                );
            }
            3 => {
                t.scramble_at(records, 0x800);
                for k in 0..0x100 {
                    let v = t.rng.pick(&[0u8, 1, 1, 1]);
                    t.put(records + 8 * k + 2, v);
                }
                t.scramble(0x0231_0f8e, 0xc);
                t.scramble(0x0186_a19c, 0x10);
                let v = t.rng.pick(&[0u32, 1, 1, 2]);
                t.put(t.sym(0x0186_a19c), v);
                let v = t.rng.pick(&[0u32, 0, 1, 0x100]);
                t.put(t.sym(0x0186_a1a8), v);
                let v = t.rng.pick(&[0i32, 1, 5, -1, 0x0100_0001]);
                t.put(t.sym(0x0186_a1a4), v);
                let v = t.rng.pick(&[0i32, 1, 2, 3, -1, -2, -3]);
                t.put(timer, v);
                let dx = t.rng.pick(&[0i32, 1, -1, 100, -300, 0x0300_0001, i32::MIN]);
                t.twin(
                    "BtlEnd_DrawExchangePage_008bd670",
                    0x008b_d670,
                    &[dx.cast_unsigned()],
                    VOID,
                    |o| {
                        tk::btl_end_draw_exchange_page(o, dx, local, &mut ui_remap, &mut callback);
                        0
                    },
                );
            }
            _ => {
                t.scramble_at(w0, 0x140);
                let text = w0 + 0x120;
                for (o, e) in [(0x94u32, 0x98u32), (0x60, 0x64), (0x78, 0x7c)] {
                    t.put(w0 + o, text);
                    let v = t.rng.pick(&[0, text]);
                    t.put(w0 + e, v);
                }
                let v = t.rng.pick(&[0u16, 1, 1, 2]);
                t.put(w0 + 0x22, v);
                let v = t.rng.pick(&[0u16, 1, 1, 2]);
                t.put(w0 + 0x1e, v);
                let v = t.rng.pick(&[0i16, 1, -1]);
                t.put(w0 + 0x1c, v);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(w0 + 0xac, v);
                let v = t.rng.pick(&[0i16, 1, 3, 7]);
                t.put(w0 + 4, v);
                let v = t.rng.pick(&floats);
                t.put(wcell, v.to_bits());
                let v = t.rng.pick(&floats);
                t.put(fcell, v.to_bits());
                let v = t.rng.pick(&[0u32, 1, 7, 0x7fff, 0x8000, 0xffff, 0x1_0005]);
                t.put(hcell, v);
                let v = t.rng.pick(&[0u32, 1, 2, 0xffff, 0x1_0001]);
                t.put(above, v);
                let v = t.rng.pick(&[0u32, 1, 2, 4, 0xffff_ffff]);
                t.put(words, v);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(calm, v);
                t.twin(
                    "TkMenuMessageLayout2_008de5d0",
                    0x008d_e5d0,
                    &[w0],
                    U32,
                    |o| {
                        w(tk::tk_menu_message_layout2(
                            o,
                            w0,
                            local,
                            &mut callback,
                            &mut fcallback,
                        ))
                    },
                );
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round31_ishop_layout_summon_status_caption_kaizou_and_item_list_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0031_0002) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008c_2b90u32, 2u8),
        (0x008c_2b50, 2),
        (0x008c_2ad0, 1),
        (0x008c_1a60, 6),
        (0x008b_f750, 5),
        (0x008d_3280, 0),
        (0x008d_2660, 1),
        (0x0088_6e80, 1),
        (0x008a_afe0, 0),
        (0x008a_af70, 0),
        (0x008d_2b40, 1),
        (0x008d_3140, 4),
        (0x008d_2680, 1),
        (0x008f_dee0, 4),
        (0x008f_5f70, 5),
        (0x008f_5c10, 0),
        (0x008e_71d0, 1),
        (0x008a_9860, 0),
        (0x008c_1390, 4),
        (0x008d_6560, 5),
        (0x008c_0f70, 4),
        (0x008c_1370, 0),
        (0x008d_65f0, 3),
        (0x008e_6cc0, 7),
        (0x008f_8bb0, 5),
        (0x008d_4910, 0),
        (0x008d_6340, 1),
        (0x008d_63f0, 1),
        (0x008d_56f0, 1),
        (0x008f_f490, 3),
        (0x008c_7c30, 5),
        (0x008c_7af0, 3),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let pads = [0x008b_e410u32, 0x008b_e470, 0x008b_e4b0].map(|f| seq_stub(&mut t, f, log, 0, &[]));
    let summon = seq_stub(&mut t, 0x008a_9880, log, 0, &[]);
    let max = seq_stub(&mut t, 0x008a_9bf0, log, 0, &[]);
    let ratio = seq_stub(&mut t, 0x008a_b220, log, 0, &[]);
    // ToGetCrossExtMesFontWidth(_, text, &w, scale, extra): the first call
    // writes [c1], the second [c2]
    let (c1, c2, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x15]; // mov eax, [esp+12]; mov edx, [c1]
    extra.extend(c1.to_le_bytes());
    extra.extend([0x89, 0x10, 0xa1]); // mov [eax], edx; mov eax, [c2]
    extra.extend(c2.to_le_bytes());
    extra.push(0xa3); // mov [c1], eax
    extra.extend(c1.to_le_bytes());
    extra.extend([0x89, 0x15]); // mov [c2], edx
    extra.extend(c2.to_le_bytes());
    let code = seq_code_of(log, 0x0090_5320, &[0, 1, 3, 4], dummy, &extra);
    jump_stub(&mut t, 0x0090_5320, &code);
    // TOMkpCrossExtMesFontLClut(_, text, ..): logs 16 bytes of the text
    let cell = t.alloc(&[0; 4]);
    cells.push(cell);
    let code = seq_code_deref_at(log, 0x0090_16b0, 1, 4, &[0, 2, 3, 4, 5, 6], cell);
    jump_stub(&mut t, 0x0090_16b0, &code);
    // TOGetGroup(code): (code >> 4) & 3
    let group = t.alloc(&[0; 4]);
    let mut extra = vec![
        0x8b, 0x44, 0x24, 0x04, 0xc1, 0xe8, 0x04, 0x83, 0xe0, 0x03, 0xa3,
    ];
    extra.extend(group.to_le_bytes());
    let code = seq_code(log, 0x008d_f610, 1, group, &extra);
    jump_stub(&mut t, 0x008d_f610, &code);
    // TOSkipCode(p): p + 1
    let next = t.alloc(&[0; 4]);
    let mut extra = vec![0x8b, 0x44, 0x24, 0x04, 0x40, 0xa3]; // mov eax, [esp+4]; inc eax; mov [next], eax
    extra.extend(next.to_le_bytes());
    let code = seq_code(log, 0x008d_fd80, 1, next, &extra);
    jump_stub(&mut t, 0x008d_fd80, &code);
    let (w0, text) = (t.alloc(&[0; 0x100]), t.alloc(&[0; 0x10]));
    // remapX2(660) itself (when it is an f32), for the caption's width test
    #[allow(clippy::cast_possible_truncation)]
    let narrow = ui_remap(&mut t.a, false, 660.0) as f32;
    ui_remap(&mut t.b, false, 660.0);
    // scrolls where the kaizou rows' f32 rounding moves ftol
    let step = ui_remap(&mut t.a, true, 675.0);
    ui_remap(&mut t.b, true, 675.0);
    t.a.clear_dirty();
    t.b.clear_dirty();
    let mut rounding = vec![0x1000u16];
    for v in i16::MIN..=i16::MAX {
        use ffx_bridge::x87::{load, mul, store};
        let y = mul(mul(step, f64::from(v)), -0.000_244_140_625);
        #[allow(clippy::cast_possible_truncation)]
        if y.trunc() as i32 != load(store(y)).trunc() as i32 {
            rounding.push(v.cast_unsigned());
        }
    }
    eprintln!("{} rounding scrolls", rounding.len() - 1);
    let floats = [
        0f32,
        -0.0,
        1.5,
        100.0,
        299.5,
        650.2,
        900.0,
        1e10,
        f32::from_bits(0x7fc0_0000),
        narrow,
    ];
    for round in 0..5000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w0, 0x100);
        match round % 5 {
            0 => t.twin("TkMn2GetIShopPos", 0x008d_b080, &[], U32, |o| {
                w(tk::tk_mn2_get_ishop_pos(o, &mut callback))
            }),
            1 => {
                let state = t.rng.pick(&[
                    0u32, 1, 2, 3, 4, 4, 4, 4, 4, 4, 4, 4, 5, 6, 7, 8, 8, 9, 10, 10, 11, 12, 12,
                    13, 14, 14, 15, 16, 17,
                ]);
                t.put(w0 + 0x28, state);
                let v = t.rng.pick(&[0u32, 4, 8, 0xc, 0x1_0004]);
                t.put(pads[0], v);
                let v = t.rng.pick(&[0u32, 4, 8, 0xc]);
                t.put(pads[1], v);
                let v = t.rng.pick(&[0u32, 0x20, 0x40, 0x60, 0, 0, 0, 0x1_0020]);
                t.put(pads[2], v);
                let v = t.rng.pick(&[0u32, 1, 0xd, 0xe, 0x11, 0x12, 0xffff_ffff]);
                t.put(summon, v);
                let v = t.rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff]);
                t.put(max, v);
                let v = t.rng.pick(&[0u32, 1, 1]);
                t.put(ratio, v);
                t.twin(
                    "MenuSummon_StatusWindowCtrl_008ccb60",
                    0x008c_cb60,
                    &[w0],
                    U32,
                    |o| w(tk::menu_summon_status_window_ctrl(o, w0, &mut callback)),
                );
            }
            2 => {
                let v = t.rng.pick(&[0u16, 0, 1, 0x100]);
                t.put(t.sym(0x0186_a168), v);
                for g in [0x025c_c594, 0x025c_c58c] {
                    let v = t.rng.pick(&[0, text, text]);
                    t.put(t.sym(g), v);
                }
                t.scramble_at(text, 0x10);
                for k in 0..0x10 {
                    let v = t.rng.pick(&[0x10u8, 0x20, 0x30, 0x41, 0x52]);
                    t.put(text + k, v);
                }
                let n = t.rng.below(8);
                t.put(text + n, 0u8);
                t.put(text + 0xf, 0u8);
                for c in [c1, c2] {
                    let v = t.rng.pick(&floats);
                    t.put(c, v.to_bits());
                }
                t.twin("BtlEnd_DrawCaption_008b9f30", 0x008b_9f30, &[], U32, |o| {
                    w(tk::btl_end_draw_caption(
                        o,
                        local,
                        &mut ui_remap,
                        &mut callback,
                    ))
                });
            }
            3 => {
                let a = t.rng.pick(&[0u16, 1, 5, 0x8000]);
                t.put(w0 + 0x32, a);
                let b = t.rng.pick(&[a, a, 2, 0xffff, a ^ 0x100]);
                t.put(w0 + 0x34, b);
                let v = t.rng.pick(&[0u16, 1, 0x1000, 0x7fff, 0x8000, 0xfff0]);
                let v = if t.rng.below(3) == 0 {
                    t.rng.pick(&rounding)
                } else {
                    v
                };
                t.put(w0 + 0x46, v);
                t.twin("MenuKaizou_Draw_008d6860", 0x008d_6860, &[w0], U32, |o| {
                    w(tk::menu_kaizou_draw(o, w0, &mut ui_remap, &mut callback))
                });
            }
            _ => {
                let a = t.rng.pick(&[0u16, 1, 5, 0x8000]);
                t.put(w0 + 0x32, a);
                let b = t.rng.pick(&[a, a, 2, 0xffff, a ^ 0x100]);
                t.put(w0 + 0x34, b);
                let v = t.rng.pick(&[0u16, 1, 0x1000, 0x7fff, 0x8000, 0xfff0]);
                t.put(w0 + 0x46, v);
                // the rows loop runs u16 +0x3a times (kept small: every row
                // logs into the call log)
                let v = t.rng.pick(&[0u16, 1, 3, 9, 0x41]);
                t.put(w0 + 0x3a, v);
                let v = t.rng.pick(&[0u8, 1, 2, 2, 3]);
                t.put(w0 + 0x43, v);
                let v = t.rng.pick(&[0u32, 0, 1, 0x100]);
                t.put(w0 + 0x94, v);
                let v = t.rng.pick(&[0u32, 1, 1, 2]);
                t.put(t.sym(0x0186_a408), v);
                t.twin(
                    "MenuItem_ListWindowDraw_008c7750",
                    0x008c_7750,
                    &[w0],
                    U32,
                    |o| {
                        w(tk::menu_item_list_window_draw(
                            o,
                            w0,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round31_ctrl_ability_summon_window2_and_status_window_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0031_0003) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008e_2830u32, 5u8),
        (0x008a_aef0, 2),
        (0x008a_9ad0, 1),
        (0x008a_9bc0, 1),
        (0x0088_6e80, 1),
        (0x008c_6d00, 2),
        (0x008c_6800, 1),
        (0x008c_6bc0, 1),
        (0x008c_93a0, 0),
        (0x008c_6b60, 0),
        (0x008c_7650, 1),
        (0x008a_a410, 1),
        (0x008c_9240, 3),
        (0x008c_93c0, 0),
        (0x008c_6b80, 0),
        (0x008a_ad70, 1),
        (0x008a_a100, 2),
        (0x008a_f4f0, 2),
        (0x008c_1790, 0),
        (0x008c_1c90, 5),
        (0x008f_f490, 3),
        (0x008f_5f70, 5),
        (0x008f_8bb0, 5),
        (0x008c_0f70, 4),
        (0x008c_e520, 5),
        (0x008c_1370, 0),
        (0x008c_da20, 3),
        (0x008e_6cc0, 7),
        (0x008d_4910, 0),
        (0x008d_4170, 2),
        (0x008c_0cc0, 0),
        (0x008d_ce50, 0),
        (0x008c_0be0, 0),
        (0x008c_2330, 0),
        (0x008c_2c30, 4),
        (0x008f_9280, 4),
        (0x008c_2300, 0),
        (0x008c_0ec0, 4),
        (0x008f_4f10, 4),
        (0x008f_5000, 4),
        (0x008c_0cd0, 0),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let pads = [0x008b_e410u32, 0x008b_e470, 0x008b_e4b0].map(|f| seq_stub(&mut t, f, log, 0, &[]));
    let player = seq_stub(&mut t, 0x008a_9860, log, 0, &[]);
    // TkMenuSetPrevPlayer / TkMenuSetNextPlayer move the current member
    for f in [0x008a_afa0, 0x008a_af10] {
        let bump: Vec<u8> = [&[0xff, 0x05][..], &player.to_le_bytes()].concat();
        cells.push(seq_stub(&mut t, f, log, 0, &bump));
    }
    cells.push(seq_stub(&mut t, 0x008a_9880, log, 0, &[]));
    let mp = seq_stub(&mut t, 0x008a_9970, log, 1, &[]);
    // MsGetRomPlyCommand(id, &out): the out word is the caller's frame
    let rom = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x0079_0a90, &[0], rom, &[]);
    jump_stub(&mut t, 0x0079_0a90, &code);
    let max = seq_stub(&mut t, 0x008a_9b30, log, 0, &[]);
    let ratio = seq_stub(&mut t, 0x008a_b220, log, 0, &[]);
    let ease = seq_stub(&mut t, 0x008d_30c0, log, 2, &[]);
    let help_ratio = seq_stub(&mut t, 0x008b_f120, log, 2, &[]);
    let scale = seq_stub(&mut t, 0x008b_f100, log, 2, &[]);
    // MenuSummon_GrowInfo(stat, chr, &cost, &gain): both from cells
    let (cost, gain, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x15]; // mov eax, [esp+12]; mov edx, [cost]
    extra.extend(cost.to_le_bytes());
    extra.extend([0x89, 0x10, 0x8b, 0x44, 0x24, 0x10, 0x8b, 0x15]); // mov [eax], edx; mov eax, [esp+16]; mov edx, [gain]
    extra.extend(gain.to_le_bytes());
    extra.extend([0x89, 0x10]);
    let code = seq_code(log, 0x008c_d320, 2, dummy, &extra);
    jump_stub(&mut t, 0x008c_d320, &code);
    // TkMenuGetDispCommand(.., list): 22 words copied into the list
    let (commands, count) = (t.alloc(&[0; 0x58]), t.alloc(&[0; 4]));
    let mut extra = vec![0x56, 0x57, 0xbe]; // push esi; push edi; mov esi, commands
    extra.extend(commands.to_le_bytes());
    // mov edi, [esp+0x1c]; mov ecx, 22; rep movsd; pop edi; pop esi
    extra.extend([
        0x8b, 0x7c, 0x24, 0x1c, 0xb9, 0x16, 0, 0, 0, 0xf3, 0xa5, 0x5f, 0x5e,
    ]);
    let code = seq_code(log, 0x008d_2e10, 4, count, &extra);
    jump_stub(&mut t, 0x008d_2e10, &code);
    let (m0, w1, w2, rec) = (
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x40]),
    );
    t.put(t.sym(0x0186_a394), w1);
    t.put(t.sym(0x0186_a39c), w2);
    for round in 0..4000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(m0, 0x100);
        match round % 3 {
            0 => {
                t.scramble_at(w1, 0x100);
                t.scramble_at(w2, 0x100);
                t.scramble_at(rec, 0x40);
                t.put(rom, rec);
                t.scramble(0x0186_a3b4, 0x30);
                t.scramble(0x0159_7770, 0x80);
                t.scramble(0x0186_a38c, 4);
                t.scramble(0x0186_a3a8, 2);
                // state 4 half the time (the member flip)
                let state = if t.rng.below(2) == 0 {
                    4
                } else {
                    t.rng.below(0x15)
                };
                t.put(m0 + 0x1c, state);
                let v = t.rng.pick(&[0u32, 4, 8, 0xc, 0x1_0004]);
                t.put(pads[0], v);
                let v = t.rng.pick(&[0u32, 4, 8, 0xc]);
                t.put(pads[1], v);
                for w in [w1, w2] {
                    let v = t.rng.pick(&[0u8, 0, 0, 0, 1, 0x7f, 0x80, 0xff]);
                    t.put(w + 0x45, v);
                    let v = t.rng.pick(&[0i16, 1, 3, -1]);
                    t.put(w + 0x48, v);
                    let v = t.rng.pick(&[0u32, 0, 1, 0x1_0000]);
                    t.put(w + 0x58, v);
                }
                let v = t.rng.pick(&[0u32, 1, 5, 7]);
                t.put(player, v);
                for k in 0..8 {
                    let v = t.rng.pick(&[0i16, 1, 4, -1, -0x8000]);
                    t.put(t.sym(0x0186_a3b4) + 2 * k, v);
                }
                let v = t.rng.pick(&[0u32, 3, 10, 100, 0xffff_fff0]);
                t.put(mp, v);
                let v = t.rng.pick(&[0u8, 3, 10, 50, 0xff]);
                t.put(rec + 0x25, v);
                let v = t.rng.pick(&[0u8, 1, 2, 3]);
                t.put(t.sym(0x0186_a392), v);
                let v = t.rng.pick(&[0u32, 1, 2, 3, 3, 0xffff_ffff]);
                t.put(max, v);
                let v = t.rng.pick(&[0u32, 1, 1]);
                t.put(ratio, v);
                let v = t.rng.pick(&[0i32, 1, 2, 5, -1, -2, -5]);
                t.put(t.sym(0x0186_a38c), v);
                t.twin("TkMenuCtrlAbility", 0x008c_60b0, &[m0], U32, |o| {
                    w(tk::tk_menu_ctrl_ability(o, m0, local, &mut callback))
                });
            }
            1 => {
                let v = t.rng.pick(&[0xffff_ffffu32, 0, 5, 0x8000_0000]);
                t.put(cost, v);
                let a = t.rng.pick(&[0u16, 1, 5, 0x8000]);
                t.put(m0 + 0x32, a);
                let b = t.rng.pick(&[a, a, 2, 0xffff, a ^ 0x100]);
                t.put(m0 + 0x34, b);
                let v = t.rng.pick(&[0u16, 1, 0x1000, 0x7fff, 0x8000, 0xfff0]);
                t.put(m0 + 0x46, v);
                t.scramble(0x0186_a5e4, 4);
                t.twin(
                    "MenuSummon_Window2Draw_008ce750",
                    0x008c_e750,
                    &[m0],
                    U32,
                    |o| {
                        w(tk::menu_summon_window2_draw(
                            o,
                            m0,
                            local,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
            _ => {
                t.scramble_at(commands, 0x58);
                for k in 0..22 {
                    let v = t.rng.pick(&[0u32, 0x3000, 0x3001, 0x3002]);
                    t.put(commands + 4 * k, v);
                }
                let v = t.rng.pick(&[0u32, 1, 3, 8, 22, 0xffff_ffff]);
                t.put(count, v);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(m0 + 0x64, v);
                let v = t.rng.pick(&[0u32, 1, 1, 2]);
                t.put(m0 + 0x58, v);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(m0 + 0x60, v);
                let v = t.rng.pick(&[0i16, 0, 1, 0x40, -1, 0x1000]);
                t.put(m0 + 0x54, v);
                t.scramble(0x0186_a9da, 2);
                let v = t.rng.pick(&[0u32, 0x1000, 0x800, 0xffff_f000, 0x7fff_ffff]);
                t.put(ease, v);
                for c in [help_ratio, scale] {
                    // -456 and 568 put x on the +-512 edges (remapX2(210) = 56)
                    let v = t
                        .rng
                        .pick(&[0i32, 10, -100, 300, 1000, -1000, 0x0100_0001, -456, 568]);
                    t.put(c, v);
                }
                t.twin(
                    "MenuStatus_WindowDraw_008d2790",
                    0x008d_2790,
                    &[m0],
                    U32,
                    |o| {
                        w(tk::menu_status_window_draw(
                            o,
                            m0,
                            local,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round31_gear_customize_limit_page_and_spoil_list_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0031_0004) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008c_1c90u32, 5u8),
        (0x008f_5f70, 5),
        (0x008f_8bb0, 5),
        (0x008a_9860, 0),
        (0x008c_1390, 4),
        (0x008d_5d50, 5),
        (0x008c_0f70, 4),
        (0x008c_1370, 0),
        (0x008d_5df0, 3),
        (0x008e_6cc0, 7),
        (0x008d_6660, 3),
        (0x0090_3bb0, 9),
        (0x0090_3ee0, 11),
        (0x008a_9950, 1),
        (0x008a_9930, 1),
        (0x008d_3690, 6),
        (0x008c_1bd0, 1),
        (0x008d_dfc0, 1),
        (0x0090_5ab0, 6),
        (0x008f_9230, 4),
        (0x008b_e0d0, 4),
        (0x008f_eb00, 3),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let kaizou = seq_stub(&mut t, 0x008c_1b70, log, 0, &[]);
    let timer = seq_stub(&mut t, 0x008a_9c60, log, 0, &[]);
    // ToGetBtlEasyFontWidth(text, &w, ..): w from a cell, unless [skip]
    let (wcell, skip, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x83, 0x3d]; // cmp dword [skip], 0
    extra.extend(skip.to_le_bytes());
    extra.extend([0x00, 0x75, 0x0c]); // jnz over the store
    extra.extend([0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15]); // mov eax, [esp+8]; mov edx, [wcell]
    extra.extend(wcell.to_le_bytes());
    extra.extend([0x89, 0x10]);
    let code = seq_code_of(log, 0x0090_5290, &[0, 2, 3, 4], dummy, &extra);
    jump_stub(&mut t, 0x0090_5290, &code);
    let (w0, table, other) = (
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x800]),
        t.alloc(&[0; 0x100]),
    );
    t.put(kaizou, table);
    t.put(t.sym(0x0186_aa30), other);
    let floats = [
        0f32,
        -0.0,
        1.5,
        100.0,
        299.5,
        1e10,
        f32::from_bits(0x7fc0_0000),
    ];
    for round in 0..3000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        match round % 3 {
            0 => {
                t.scramble_at(w0, 0x100);
                t.scramble_at(table, 0x800);
                t.scramble_at(other, 0x100);
                t.scramble(0x0159_7770, 0x100);
                let row = t.rng.pick(&[0i16, 1, 5, 0x3f, -1]);
                t.put(w0 + 0x48, row);
                let v = t.rng.pick(&[0xffu8, 0, 3, 0x80]);
                #[allow(clippy::cast_sign_loss)]
                let at = t.sym(0x0159_7773).wrapping_add((i32::from(row) * 4) as u32);
                t.put(at, v);
                let a = t.rng.pick(&[0u16, 1, 5, 0x8000]);
                t.put(w0 + 0x32, a);
                let b = t.rng.pick(&[a, a, 2, 0xffff, a ^ 0x100]);
                t.put(w0 + 0x34, b);
                let v = t.rng.pick(&[0u16, 1, 0x1000, 0x7fff, 0x8000, 0xfff0]);
                t.put(w0 + 0x46, v);
                t.twin("DrawGearCustomizationMenu", 0x008d_5f60, &[w0], U32, |o| {
                    w(tk::draw_gear_customization_menu(
                        o,
                        w0,
                        &mut ui_remap,
                        &mut callback,
                    ))
                });
            }
            1 => {
                let v = t.rng.pick(&floats);
                t.put(wcell, v.to_bits());
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(skip, v);
                let chr = t.rng.pick(&[0u32, 1, 7, 8, 9, 0xffff_ffff, 0x8000_0000]);
                t.twin("DrawLimitPageOverDrive", 0x008d_2270, &[chr], U32, |o| {
                    w(tk::draw_limit_page_over_drive(
                        o,
                        chr,
                        local,
                        &mut ui_remap,
                        &mut callback,
                    ))
                });
            }
            _ => {
                t.scramble(0x0186_a188, 0x24);
                let a = t.rng.pick(&[0u32, 1, 5, 0x7fff_ffff]);
                t.put(t.sym(0x0186_a190), a);
                let b = t.rng.pick(&[a, a, 2, 0xffff_ffff, a ^ 0x100]);
                t.put(t.sym(0x0186_a194), b);
                let v = t.rng.pick(&[0u32, 0, 1, 2, 0x100]);
                t.put(t.sym(0x0186_a19c), v);
                let v = t.rng.pick(&[0u32, 0, 1, 0x100]);
                t.put(t.sym(0x0186_a1a8), v);
                let v = t.rng.pick(&[0i32, 1, 5, -1, 0x0100_0001]);
                t.put(t.sym(0x0186_a1a0), v);
                let v = t.rng.pick(&[
                    0i32,
                    1,
                    0x1000,
                    -0x800,
                    0x7fff_ffff,
                    0x0100_0001,
                    0x0123_4567,
                ]);
                t.put(t.sym(0x0186_a18c), v);
                let v = t.rng.pick(&[0i32, 1, 2, 3, -1, -2, -3]);
                t.put(timer, v);
                let dx = t
                    .rng
                    .pick(&[0i32, 0, 1, -1, 100, -300, 0x0300_0001, i32::MIN]);
                t.twin(
                    "BtlEnd_DrawSpoilListPage_008bd280",
                    0x008b_d280,
                    &[dx.cast_unsigned()],
                    VOID,
                    |o| {
                        tk::btl_end_draw_spoil_list_page(o, dx, &mut ui_remap, &mut callback);
                        0
                    },
                );
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round32_scan_spline_limit_draw_and_open_mode_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0032_0001) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x0088_8e10u32, 2u8),
        (0x0088_9050, 3),
        (0x008d_ce50, 0),
        (0x008c_1790, 0),
        (0x008c_0be0, 0),
        (0x008c_1a20, 1),
        (0x0090_5ab0, 6),
        (0x008f_9280, 4),
        (0x008d_1400, 2),
        (0x008c_0cc0, 0),
        (0x008a_aab0, 2),
        (0x008a_a100, 2),
        (0x008e_1c60, 2),
        (0x008a_8e70, 0),
        (0x0082_2380, 1),
        (0x0078_73a0, 0),
        (0x008a_b260, 1),
        // the segment (its coefficient copy carries a frame word)
        (0x0088_abd0, 5),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let player = seq_stub(&mut t, 0x008a_9860, log, 0, &[]);
    let offset = seq_stub(&mut t, 0x008d_2dc0, log, 4, &[]);
    // MsMenuStatusNum(0, &n): the list; the out word is a frame address
    let (list, list_cell, dummy) = (t.alloc(&[0; 0x100]), t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    t.put(list_cell, list);
    let code = seq_code_of(log, 0x0079_0200, &[0], list_cell, &[]);
    jump_stub(&mut t, 0x0079_0200, &code);
    // ToGetBtlEasyFontWidth(text, &w, ..): w from a cell, unless [skip]
    let (wcell, skip) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x83, 0x3d]; // cmp dword [skip], 0
    extra.extend(skip.to_le_bytes());
    extra.extend([0x00, 0x75, 0x0c]); // jnz over the store
    extra.extend([0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15]); // mov eax, [esp+8]; mov edx, [wcell]
    extra.extend(wcell.to_le_bytes());
    extra.extend([0x89, 0x10]);
    let code = seq_code_of(log, 0x0090_5290, &[0, 2, 3, 4], dummy, &extra);
    jump_stub(&mut t, 0x0090_5290, &code);
    // TOGetFFXLangJmp: [lang] ^= [toggle] on each call, then returns it
    let (lang, toggle) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0xa1]; // mov eax, [lang]
    extra.extend(lang.to_le_bytes());
    extra.extend([0x33, 0x05]); // xor eax, [toggle]
    extra.extend(toggle.to_le_bytes());
    extra.push(0xa3); // mov [lang], eax
    extra.extend(lang.to_le_bytes());
    let code = seq_code(log, 0x008a_c2f0, 0, lang, &extra);
    jump_stub(&mut t, 0x008a_c2f0, &code);
    // spline points with room on both sides (n below 2 reads before them)
    let (area, out) = (t.alloc(&[0; 0x300]), t.alloc(&[0; 0x10]));
    let points = area + 0x100;
    let floats = [
        0f32,
        -0.0,
        0.25,
        0.5,
        0.999,
        1.0,
        1.5,
        -2.0,
        -0.25,
        1e-8,
        f32::from_bits(0x7fc0_0000),
    ];
    for round in 0..4000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        match round % 4 {
            0 => {
                t.twin("TkUpdateScanControler", 0x0088_9a80, &[], U32, |o| {
                    w(tk::tk_update_scan_controler(o, &mut callback))
                });
            }
            1 => {
                for k in 0..0xc0 {
                    let v = t.rng.below(2001).cast_signed() - 1000;
                    #[allow(clippy::cast_precision_loss)]
                    let v = v as f32 / 10.0;
                    t.put(area + 4 * k, v.to_bits());
                }
                if t.rng.below(4) == 0 {
                    // p1 = p0: a zero tangent
                    for k in 0..4 {
                        let v: u32 = Mem::read(&t.a, points + 4 * k);
                        t.put(points + 0x10 + 4 * k, v);
                    }
                }
                t.scramble_at(out, 0x10);
                // m and k beyond f32's 24 bits (the segment is a stub): f32(k)
                // rounds, and frac leaves 0..=1 for the loops to repair
                let n = t.rng.pick(&[
                    -5i32,
                    -1,
                    0,
                    1,
                    2,
                    2,
                    3,
                    3,
                    4,
                    5,
                    8,
                    12,
                    0x0200_0003,
                    0x0180_0003,
                    0x0180_0003,
                    0x0123_4567,
                ]);
                #[allow(clippy::cast_precision_loss)]
                let tv = match t.rng.below(4) {
                    0 => t.rng.below(0x1_0001) as f32 / 65536.0,
                    1 if n > 2 => {
                        // j steps of f32(1 / f32(n - 2)), give or take an ulp:
                        // the rounding the frac loops repair
                        let m = (n - 2) as f32;
                        let step = 1.0 / m;
                        let j = t.rng.below(n.cast_unsigned() - 1) as f32;
                        let bits = (j * step).to_bits();
                        let nudge = t.rng.pick(&[0u32, 1, 0xffff_ffff]);
                        f32::from_bits(bits.wrapping_add(nudge)).clamp(0.0, 1.0)
                    }
                    _ => t.rng.pick(&floats),
                };
                let position = t.rng.pick(&[0u32, 0, 1, 7]);
                let args = [out, points, n.cast_unsigned(), tv.to_bits(), position];
                t.twin("TkMtCalcSplineAt_0088afd0", 0x0088_afd0, &args, VOID, |o| {
                    tk::tk_mt_calc_spline_at(o, [out, points], n, tv, position, &mut callback);
                    0
                });
            }
            2 => {
                t.scramble_at(list, 0x100);
                let w0 = t.alloc_random(0x80);
                let v = t.rng.pick(&[0u32, 0, 1, 0x100]);
                t.put(w0 + 0x5c, v);
                let v = t.rng.pick(&[0u32, 1, 5, 0x3f]);
                t.put(offset, v);
                let v = t.rng.pick(&floats);
                t.put(wcell, v.to_bits());
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(skip, v);
                t.scramble(0x0186_a70c, 4);
                let v = t.rng.pick(&[0i16, 1, -1, 0x7fff, -0x8000]);
                t.put(t.sym(0x0186_a70e), v);
                let v = t.rng.below(8);
                t.put(player, v);
                t.twin(
                    "MenuLimit_WindowDraw_008d16f0",
                    0x008d_16f0,
                    &[w0],
                    U32,
                    |o| {
                        w(tk::menu_limit_window_draw(
                            o,
                            w0,
                            local,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
            _ => {
                let mode = t.rng.pick(&[
                    0x1_0000u32,
                    0x2_0000,
                    0x4_0000,
                    0x8_0000,
                    0x8_0000,
                    0x8_0000,
                    0x10_0000,
                    0x20_0000,
                    0x40_0000,
                    0x80_0000,
                    0,
                    1,
                    0x3_0000,
                    0x8_0001,
                    0x100_0000,
                    0xffff_ffff,
                ]);
                let arg = t
                    .rng
                    .pick(&[0u32, 5, 0xd, 0xe, 0x11, 0x1f, 0x20, 0x21, 0xffff_ffff]);
                let v = t.rng.pick(&[0xbu32, 0xb, 0, 1, 0xa]);
                t.put(lang, v);
                let v = t.rng.pick(&[0u32, 0, 0xb, 0xb ^ 1]);
                t.put(toggle, v);
                let v = t.rng.pick(&[0x17eu32, 0x17e, 0, 0x17f, 0x1_017e]);
                t.put(t.sym(0x0133_0844), v);
                t.twin(
                    "TkMenuMainOpenMode_008aa780",
                    0x008a_a780,
                    &[mode, arg],
                    U32,
                    |o| w(tk::tk_menu_main_open_mode(o, mode, arg, &mut callback)),
                );
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round32_spline_wrappers_and_summon_list_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    type Wrapper = fn(&mut Oracle, [u32; 2], i32, f32);
    let Some(mut t) = setup_any(0x0032_0002) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = vec![];
    for (f, n) in [
        // both segments (their coefficient copies carry a frame word)
        (0x0088_abd0u32, 5u8),
        (0x0088_a500, 5),
        (0x008a_9bc0, 1),
        (0x0090_3bb0, 9),
        (0x008e_7d30, 12),
        (0x008c_eb00, 3),
        (0x008c_0580, 3),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let count = seq_stub(&mut t, 0x008a_9bf0, log, 0, &[]);
    let pos = seq_stub(&mut t, 0x008a_9890, log, 0, &[]);
    let (area, out) = (t.alloc(&[0; 0x300]), t.alloc(&[0; 0x10]));
    let points = area + 0x100;
    let floats = [
        0f32,
        -0.0,
        0.25,
        0.5,
        0.999,
        1.0,
        1.5,
        -2.0,
        -0.25,
        1e-8,
        f32::from_bits(0x7fc0_0000),
    ];
    let wrappers: [(&str, u32, Wrapper); 4] = [
        ("TkMtCalcSpline", 0x0088_a4a0, |o, p, n, v| {
            tk::tk_mt_calc_spline(o, p, n, v, &mut callback);
        }),
        ("TkMtCalcSplineTangent", 0x0088_b3b0, |o, p, n, v| {
            tk::tk_mt_calc_spline_tangent(o, p, n, v, &mut callback);
        }),
        ("TkMtCalcSpline3", 0x0088_a4d0, |o, p, n, v| {
            tk::tk_mt_calc_spline3(o, p, n, v, &mut callback);
        }),
        (
            "TkMtCalcSpline3Tangent_0088aba0",
            0x0088_aba0,
            |o, p, n, v| {
                tk::tk_mt_calc_spline3_tangent(o, p, n, v, &mut callback);
            },
        ),
    ];
    for round in 0..4000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        if round % 3 == 2 {
            let v = t.rng.pick(&[0u32, 1, 2, 3, 5, 0xffff_ffff]);
            t.put(count, v);
            let v = t.rng.pick(&[0u32, 1, 2, 4, 0xffff_ffff]);
            t.put(pos, v);
            let dx = t.rng.pick(&[0i32, 0, 1, -100, 300, 0x0123_4567, i32::MIN]);
            let cursor = t.rng.pick(&[0i32, 0, 1, -1, -1, -2, 5, i32::MIN]);
            let args = [dx.cast_unsigned(), t.rng.next(), cursor.cast_unsigned()];
            t.twin(
                "MenuSummon_DrawList_008ce130",
                0x008c_e130,
                &args,
                U32,
                |o| {
                    w(tk::menu_summon_draw_list(
                        o,
                        dx,
                        cursor,
                        &mut ui_remap,
                        &mut callback,
                    ))
                },
            );
            continue;
        }
        for k in 0..0xc0 {
            let v = t.rng.below(2001).cast_signed() - 1000;
            #[allow(clippy::cast_precision_loss)]
            let v = v as f32 / 10.0;
            t.put(area + 4 * k, v.to_bits());
        }
        t.scramble_at(out, 0x10);
        let n = t.rng.pick(&[
            -5i32,
            -1,
            0,
            1,
            2,
            3,
            3,
            4,
            4,
            5,
            8,
            12,
            0x0180_0004,
            0x0180_0004,
            0x0123_4567,
        ]);
        #[allow(clippy::cast_precision_loss)]
        let tv = match t.rng.below(4) {
            0 => t.rng.below(0x1_0001) as f32 / 65536.0,
            1 if n > 3 => {
                let m = (n - 3) as f32;
                let step = 1.0 / m;
                let j = t.rng.below(n.cast_unsigned() - 2) as f32;
                let bits = (j * step).to_bits();
                let nudge = t.rng.pick(&[0u32, 1, 0xffff_ffff]);
                f32::from_bits(bits.wrapping_add(nudge)).clamp(0.0, 1.0)
            }
            _ => t.rng.pick(&floats),
        };
        if round % 3 == 0 {
            let (what, f, port) = t.rng.pick(&wrappers);
            let args = [out, points, n.cast_unsigned(), tv.to_bits()];
            t.twin(what, f, &args, VOID, |o| {
                port(o, [out, points], n, tv);
                0
            });
        } else {
            let position = t.rng.pick(&[0u32, 0, 1, 7]);
            let args = [out, points, n.cast_unsigned(), tv.to_bits(), position];
            t.twin(
                "TkMtCalcSpline3At_0088aa40",
                0x0088_aa40,
                &args,
                VOID,
                |o| {
                    tk::tk_mt_calc_spline3_at(o, [out, points], n, tv, position, &mut callback);
                    0
                },
            );
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round32_ctrl_status_w_shop_and_sub_cmd_items_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0032_0003) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008b_f750u32, 5u8),
        (0x008d_3280, 0),
        (0x008d_2660, 1),
        (0x0088_6e80, 1),
        (0x008a_a410, 1),
        (0x008a_afa0, 0),
        (0x008a_af10, 0),
        (0x008d_3140, 4),
        (0x008d_2680, 1),
        (0x008a_ad70, 1),
        (0x008a_a100, 2),
        (0x0088_d190, 3),
        (0x008d_b440, 0),
        (0x008a_b260, 1),
        (0x008d_a7a0, 0),
        (0x008e_49d0, 0),
        (0x008d_a950, 0),
        (0x008d_9b30, 0),
        (0x008d_a8f0, 0),
        (0x008d_aaa0, 0),
        (0x008d_df80, 1),
        (0x008e_33a0, 3),
        (0x008e_2de0, 0),
        (0x008d_a860, 1),
        (0x008d_9b10, 0),
        (0x008d_aac0, 0),
        (0x008d_aa00, 1),
        (0x008d_9bb0, 0),
        (0x008c_2360, 1),
        (0x008e_36f0, 0),
        (0x0090_0f80, 4),
        (0x0090_0f60, 2),
        (0x008f_f260, 1),
        (0x0090_3bb0, 9),
        (0x008f_9230, 4),
        (0x0090_5ab0, 6),
        (0x008c_0670, 3),
        (0x008f_f140, 1),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let pads = [0x008b_e410u32, 0x008b_e470, 0x008b_e4b0].map(|f| seq_stub(&mut t, f, log, 0, &[]));
    // MenuTuto_CloseWindow2 moves the sub-command window to [moved]
    let moved = t.alloc(&[0; 4]);
    let mut extra = vec![0xa1]; // mov eax, [moved]
    extra.extend(moved.to_le_bytes());
    extra.push(0xa3); // mov [g_TkMenuSubCmdWindow], eax
    extra.extend(t.sym(0x023c_c128).to_le_bytes());
    cells.push(seq_stub(&mut t, 0x008d_9b60, log, 0, &extra));
    let first_read = seq_stub(&mut t, 0x008a_8e50, log, 0, &[]);
    let members = seq_stub(&mut t, 0x008a_9b30, log, 0, &[]);
    let ratio = seq_stub(&mut t, 0x008a_b220, log, 0, &[]);
    let sync = seq_stub(&mut t, 0x0088_d1c0, log, 1, &[]);
    let size = seq_stub(&mut t, 0x0088_d390, log, 2, &[]);
    let buffer = seq_stub(&mut t, 0x008c_1b10, log, 0, &[]);
    let excel = seq_stub(&mut t, 0x008c_1af0, log, 2, &[]);
    let can_buy = seq_stub(&mut t, 0x008d_ab30, log, 1, &[]);
    let held = seq_stub(&mut t, 0x0079_04b0, log, 1, &[]);
    let yes_no = seq_stub(&mut t, 0x008e_3390, log, 0, &[]);
    // ToGetBtlEasyFontWidth(text, &w, ..): w from a cell
    let (wcell, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15]; // mov eax, [esp+8]; mov edx, [wcell]
    extra.extend(wcell.to_le_bytes());
    extra.extend([0x89, 0x10]);
    let code = seq_code_of(log, 0x0090_5290, &[0, 2, 3, 4], dummy, &extra);
    jump_stub(&mut t, 0x0090_5290, &code);
    // the item window's +0x1c callback
    let usable = t.alloc(&[0; 4]);
    let code = seq_code(log, 0x0000_001c, 3, usable, &[]);
    let usable_fn = test_code(&mut t, &code);
    let (m0, status_win, rec) = (
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x100]),
    );
    let wins = [(); 6].map(|()| t.alloc(&[0; 0x80]));
    let floats = [0f32, 1.5, 30.0, 100.25, -8.0, 1e6];
    for round in 0..6000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(m0, 0x100);
        for w in wins {
            t.scramble_at(w, 0x80);
        }
        match round % 3 {
            0 => {
                // state 4 (the member flip) half the time
                let state = if t.rng.below(2) == 0 {
                    4
                } else {
                    t.rng.below(0x16)
                };
                t.put(m0 + 0x1c, state);
                let v = t.rng.pick(&[0u32, 0, 1, 0x100]);
                t.put(m0 + 0x20, v);
                for p in pads {
                    let v = t
                        .rng
                        .pick(&[0u32, 4, 8, 0x20, 0x40, 0x60, 0xc, 0x1_0004, 0x1_0020]);
                    t.put(p, v);
                }
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(first_read, v);
                let v = t.rng.pick(&[0u32, 1, 2, 7, 0xffff_ffff]);
                t.put(members, v);
                let v = t.rng.pick(&[0u32, 1, 1]);
                t.put(ratio, v);
                t.put(t.sym(0x0186_a838), status_win);
                t.twin("TkMenuCtrlStatus", 0x008d_1e00, &[m0], U32, |o| {
                    w(tk::tk_menu_ctrl_status(o, m0, &mut callback))
                });
            }
            1 => {
                let state = t.rng.below(0x23);
                t.put(m0 + 0x1c, state);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(sync, v);
                let v = t
                    .rng
                    .pick(&[0u32, 1, 0x40, 0x41, 0x1234, 0xffff_ffc0, 0x8000_0000]);
                t.put(size, v);
                let v = t.rng.next();
                t.put(buffer, v);
                t.put(excel, rec);
                t.scramble_at(rec, 0x100);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(can_buy, v);
                let v = t.rng.pick(&[0u32, 98, 99, 100, 0xffff_ffff]);
                t.put(held, v);
                // no yes/no window only where the original tests for it
                let v = if state == 0xe {
                    t.rng.pick(&[0, wins[5], wins[5]])
                } else {
                    wins[5]
                };
                t.put(yes_no, v);
                t.put(t.sym(0x023c_c128), wins[0]);
                let v = t.rng.pick(&[wins[0], wins[4]]);
                t.put(moved, v);
                for (k, cell) in [0x0186_ac34u32, 0x0186_ac38, 0x0186_ac3c, 0x0186_ac40]
                    .into_iter()
                    .enumerate()
                {
                    // state 7 opens a missing tutorial window; the others read them
                    let v = if k < 2 && state == 7 && t.rng.below(2) == 0 {
                        0
                    } else {
                        wins[k + 1]
                    };
                    t.put(t.sym(cell), v);
                }
                for w in wins {
                    let v = t.rng.pick(&[0u8, 0, 1, 0x7f, 0x80, 0xff]);
                    t.put(w + 0x45, v);
                    let v = t.rng.pick(&[0i16, 0, 1, 1, 2, 3, -1, 0x40]);
                    t.put(w + 0x48, v);
                }
                t.scramble(0x0186_ac18, 0x14);
                t.twin("TkMenuCtrlWShop", 0x008d_9600, &[m0], U32, |o| {
                    w(tk::tk_menu_ctrl_w_shop(o, m0, &mut callback))
                });
            }
            _ => {
                let s = wins[0];
                t.put(t.sym(0x023c_c128), s);
                for o in [0x4a, 0x4c, 0x4e] {
                    let v = t.rng.pick(&[0i16, 100, 210, -50, 600, 0x7fff]);
                    t.put(s + o, v);
                }
                let fade = t.rng.pick(&[0i16, 0x1000, 0x800, 0x1001, -1]);
                t.put(s + 0x46, fade);
                let k = t.rng.pick(&[0u16, 1, 2, 3, 5]);
                t.put(s + 0x30, k);
                let v = t.rng.pick(&[0i16, 0, 1, 2, 4, -1]);
                t.put(s + 0x48, v);
                let n = t.rng.pick(&[0u16, 1, 2, 3, 4, 6]);
                t.put(m0 + 0x30, n);
                let cb = t.rng.pick(&[0, usable_fn]);
                t.put(m0 + 0x1c, cb);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(usable, v);
                let v = t.rng.pick(&[0i16, 0, 1, 2, 5, -1]);
                t.put(m0 + 0x48, v);
                let v = t.rng.pick(&[0u8, 0, 1, 0x80, 0xff]);
                t.put(m0 + 0x45, v);
                let v = t.rng.pick(&[0x1000u16, 0x1000, 0, 0x800, 0x9000]);
                t.put(m0 + 0x46, v);
                let v = t.rng.pick(&floats);
                t.put(wcell, v.to_bits());
                t.twin(
                    "MenuSubCmd_DrawItems_008e4140",
                    0x008e_4140,
                    &[m0],
                    U32,
                    |o| {
                        w(tk::menu_sub_cmd_draw_items(
                            o,
                            m0,
                            local,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round32_sub_cmd_row_buy_list_and_compare_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0032_0004) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008f_5f70u32, 5u8),
        (0x0090_0f80, 4),
        (0x0090_0f60, 2),
        (0x008f_f260, 1),
        (0x0090_3bb0, 9),
        (0x008f_9230, 4),
        (0x0090_5ab0, 6),
        (0x008c_0670, 3),
        (0x008f_f140, 1),
        (0x008e_71d0, 1),
        (0x008f_8bb0, 5),
        (0x0090_5550, 6),
        (0x008c_0f70, 4),
        (0x008d_abf0, 6),
        (0x008c_1370, 0),
        (0x008d_ab70, 3),
        (0x008e_6cc0, 7),
        (0x008d_8aa0, 3),
        (0x008e_6970, 4),
        (0x008b_ee70, 1),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let held = seq_stub(&mut t, 0x0079_04b0, log, 1, &[]);
    let excel = seq_stub(&mut t, 0x008c_1af0, log, 2, &[]);
    let record = seq_stub(&mut t, 0x008d_9170, log, 1, &[]);
    let gear = [0x008a_9820u32, 0x008a_9c70].map(|f| seq_stub(&mut t, f, log, 1, &[]));
    // MsGetSaveWeapon(id, &out): the record from a cell (out is a frame address)
    let weapon = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x007a_bbd0, &[0], weapon, &[]);
    jump_stub(&mut t, 0x007a_bbd0, &code);
    // the width words: ToGetBtlEasyFontWidth(text, &w, ..) and
    // ToGetBtlEasyDigitWidth2(n, &w, ..) store [wcell] unless [skip]
    let (wcell, skip, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x83, 0x3d]; // cmp dword [skip], 0
    extra.extend(skip.to_le_bytes());
    extra.extend([0x00, 0x75, 0x0c]); // jnz over the store
    extra.extend([0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15]); // mov eax, [esp+8]; mov edx, [wcell]
    extra.extend(wcell.to_le_bytes());
    extra.extend([0x89, 0x10]);
    for (f, args) in [
        (0x0090_5290u32, &[0u8, 2, 3, 4][..]),
        (0x0090_5230, &[0, 2, 3]),
    ] {
        let code = seq_code_of(log, f, args, dummy, &extra);
        jump_stub(&mut t, f, &code);
    }
    let usable = t.alloc(&[0; 4]);
    let code = seq_code(log, 0x0000_001c, 3, usable, &[]);
    let usable_fn = test_code(&mut t, &code);
    let (m0, rec, sub_cmd) = (
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x80]),
    );
    let (r1, r2) = (t.alloc(&[0; 0x10]), t.alloc(&[0; 0x10]));
    let floats = [0f32, 1.5, 30.0, 100.25, -8.0, 1e6];
    for round in 0..6000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(m0, 0x100);
        let v = t.rng.pick(&floats);
        t.put(wcell, v.to_bits());
        match round % 3 {
            0 => {
                for o in [0x4a, 0x4c, 0x4e, 0x50] {
                    let v = t.rng.pick(&[0i16, 100, 210, -50, 600, 0x7fff]);
                    t.put(m0 + o, v);
                }
                let v = t.rng.pick(&[0i16, 0x1000, 0x800, 0x1001, -1]);
                t.put(m0 + 0x46, v);
                let n = t.rng.pick(&[0u16, 1, 2, 3, 4, 6]);
                t.put(m0 + 0x30, n);
                let cb = t.rng.pick(&[0, usable_fn]);
                t.put(m0 + 0x1c, cb);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(usable, v);
                let v = t.rng.pick(&[0i16, 0, 1, 2, 5, -1]);
                t.put(m0 + 0x48, v);
                let v = t.rng.pick(&[0u8, 0, 1, 0x80, 0xff]);
                t.put(m0 + 0x45, v);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(m0 + 0x94, v);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(skip, v);
                // without items eax is what the last remap left
                let mask = if n == 0 { VOID } else { U32 };
                t.twin(
                    "MenuSubCmd_DrawRow_008e3cf0",
                    0x008e_3cf0,
                    &[m0],
                    mask,
                    |o| {
                        w(tk::menu_sub_cmd_draw_row(
                            o,
                            m0,
                            local,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
            1 => {
                t.put(excel, rec);
                t.scramble_at(rec, 0x100);
                t.scramble(0x0186_ac20, 4);
                t.scramble(0x0186_ac2c, 4);
                t.scramble_at(sub_cmd, 0x80);
                t.put(t.sym(0x023c_c128), sub_cmd);
                let v = t.rng.pick(&[0u8, 1, 0x80]);
                t.put(sub_cmd + 0x45, v);
                let v = t.rng.pick(&[0u16, 2, 2, 1]);
                t.put(sub_cmd + 0x48, v);
                let v = t.rng.pick(&[0u8, 0, 1, 0xff]);
                t.put(m0 + 0x45, v);
                let v = t.rng.pick(&[0i16, 1, 9, -1]);
                t.put(m0 + 0x48, v);
                let a = t.rng.pick(&[0u16, 1, 5, 0x8000]);
                t.put(m0 + 0x32, a);
                let b = t.rng.pick(&[a, a, 2, 0xffff, a ^ 0x100]);
                t.put(m0 + 0x34, b);
                let v = t.rng.pick(&[0u32, 0, 1]);
                t.put(skip, v);
                let v = t.rng.pick(&[0u32, 1, 99, 0xffff_ffff]);
                t.put(held, v);
                t.twin(
                    "MenuWShop_BuyListDraw_008da050",
                    0x008d_a050,
                    &[m0],
                    U32,
                    |o| {
                        w(tk::menu_wshop_buy_list_draw(
                            o,
                            m0,
                            local,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
            _ => {
                t.put(excel, rec);
                t.scramble_at(rec, 0x100);
                for c in [r1, r2] {
                    t.scramble_at(c, 0x10);
                    let v = t.rng.pick(&[0u8, 0, 1, 0x80]);
                    t.put(c + 5, v);
                }
                t.put(record, r1);
                t.scramble(0x0186_ab1c, 4);
                t.scramble(0x0186_aba8, 4);
                t.put(weapon, r2);
                for c in gear {
                    let v = t.rng.pick(&[0xffu32, 3, 0x1ff]);
                    t.put(c, v);
                }
                let v = t.rng.pick(&[0u32, 3, 6, 7, 0xffff_ffff]);
                t.put(t.sym(0x0186_aba0), v);
                let v = t.rng.pick(&[0i16, 1, 9, -1]);
                t.put(m0 + 0x48, v);
                t.scramble(0x0159_7770, 0x40);
                t.put(skip, 0u32);
                let shop = t.rng.pick(&[0u32, 0, 1, 7]);
                t.twin(
                    "MenuWShop_DrawCompare_008d8620",
                    0x008d_8620,
                    &[m0, shop],
                    VOID,
                    |o| {
                        tk::menu_wshop_draw_compare(
                            o,
                            m0,
                            shop,
                            local,
                            &mut ui_remap,
                            &mut callback,
                        );
                        0
                    },
                );
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

/// `dbgPrintf` (`0x0062f500`) replaced by a logger of the format and, for
/// each `(format, n)` of `formats`, its `n` values (other formats log
/// none).
fn printf_log_n(t: &mut Twins, log: u32, formats: &[(u32, u8)]) {
    let l = log.to_le_bytes();
    #[rustfmt::skip]
    let mut code: Vec<u8> = [
        &[0x8b, 0x0d][..], &l,                  // mov ecx, [log]
        &[0x8b, 0x44, 0x24, 0x04, 0x89, 0x01],  // [ecx] = format
        &[0x83, 0x05], &l, &[0x04],             // add dword [log], 4
    ]
    .concat();
    for &(f, n) in formats {
        let mut block: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat();
        for k in 0..n {
            block.extend([0x8b, 0x54, 0x24, 8 + 4 * k, 0x89, 0x51, 4 * k]);
        }
        block.extend([&[0x83, 0x05][..], &l, &[4 * n, 0xc3]].concat());
        code.extend(
            [
                &[0x3d][..],
                &f.to_le_bytes(),
                &[0x75, u8::try_from(block.len()).unwrap()],
            ]
            .concat(),
        );
        code.extend(block);
    }
    code.push(0xc3);
    jump_stub(t, 0x0062_f500, &code);
}

#[test]
fn menu_round33_summon_window_draw_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0033_0001) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = vec![];
    for (f, n) in [
        (0x008a_9880u32, 0u8),
        (0x008e_71d0, 1),
        (0x008d_34a0, 7),
        (0x008c_e130, 3),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let ease = seq_stub(&mut t, 0x008d_30c0, log, 2, &[]);
    let w = t.alloc(&[0; 0x80]);
    let words = [0x023c_c3c0u32, 0x023c_c3c4, 0x023c_c3c8].map(|a| t.sym(a));
    for _ in 0..2000 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w, 0x80);
        for a in words {
            let v = t.rng.next();
            t.put(a, v);
        }
        let v = t.rng.pick_or_any(&[
            0,
            1,
            0x1000,
            0xffff_f000,
            0x7fff_ffff,
            0x8000_0000,
            0x0055_5556,
            11,
        ]);
        t.put(ease, v);
        let dir = t.rng.pick(&[0u16, 1, 0x7fff, 0x8000, 0xffff, 0x1234]);
        t.put(w + 0x54, dir);
        t.twin(
            "MenuSummon_WindowDraw_008cebe0",
            0x008c_ebe0,
            &[w],
            U32,
            |o| {
                w64(tk::menu_summon_window_draw(
                    o,
                    w,
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn menu_round33_gear_customization_state_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0033_0002) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008d_3280u32, 0u8),
        (0x008d_df60, 1),
        (0x008a_aeb0, 1),
        (0x008b_5530, 1),
        (0x008d_2660, 1),
        (0x008b_44b0, 1),
        (0x0088_6e80, 1),
        (0x008b_eeb0, 1),
        (0x008e_33a0, 3),
        (0x008c_2c60, 3),
        (0x008e_2de0, 0),
        (0x008d_6ce0, 1),
        (0x008d_6e00, 0),
        (0x008d_6dd0, 1),
        (0x008c_2c10, 1),
        (0x008d_57d0, 0),
        (0x008d_4ed0, 0),
        (0x008d_5680, 2),
        (0x0079_93b0, 1),
        (0x0079_0550, 2),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let formats = [(0x00b6_6338u32, 1u8)].map(|(f, n)| (t.sym(f), n));
    printf_log_n(&mut t, log, &formats);
    let selectable = seq_stub(&mut t, 0x008d_5750, log, 2, &[]);
    let yes_no = seq_stub(&mut t, 0x008e_3390, log, 0, &[]);
    let kaizou = seq_stub(&mut t, 0x008c_1b70, log, 0, &[]);
    // MsGetSaveWeapon(id, &name): name = [names], then [names] += [step]
    let (names_cell, step) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15]; // mov eax, [esp+8]; mov edx, [names]
    extra.extend(names_cell.to_le_bytes());
    extra.extend([0x89, 0x10, 0x8b, 0x15]); // mov [eax], edx; mov edx, [step]
    extra.extend(step.to_le_bytes());
    extra.extend([0x01, 0x15]); // add [names], edx
    extra.extend(names_cell.to_le_bytes());
    // (the name word is a frame local: its address is not logged)
    let weapon = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x007a_bbd0, &[0], weapon, &extra);
    jump_stub(&mut t, 0x007a_bbd0, &code);
    let (m, win30, win34, win_yn) = (
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
    );
    let (ids, rec, recs, names) = (
        t.alloc(&[0; 0x200]),
        t.alloc(&[0; 0x10]),
        t.alloc(&[0; 0x810]),
        t.alloc(&[0; 0x80]),
    );
    let state_at = t.sym(0x0186_aa68);
    let items = t.sym(0x0159_7770);
    t.put(t.sym(0x0186_aa30), win30);
    t.put(t.sym(0x0186_aa34), win34);
    t.put(t.sym(0x0186_aa38), ids + 0x100);
    t.put(weapon, rec);
    t.put(kaizou, recs);
    let answers = [0u8, 0, 1, 2, 0x7f, 0x80, 0xff];
    let rows = [0i16, 0, 1, 2, 5, -1, -0x80, 0x7f];
    for round in 0..8000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        for w in [m, win30, win34, win_yn] {
            t.scramble_at(w, 0x80);
        }
        t.scramble_at(ids, 0x200);
        t.scramble_at(rec, 0x10);
        t.scramble_at(recs, 0x810);
        let state = if round % 50 == 49 {
            t.rng.pick(&[0xeu32, 0xffff_ffff, 0x8000_0000])
        } else {
            t.rng.below(14)
        };
        t.put(state_at, state);
        for (w, at) in [(m, 0x45), (win34, 0x45), (win_yn, 0x45)] {
            let v = t.rng.pick(&answers);
            t.put(w + at, v);
        }
        for w in [m, win30, win34] {
            let v = t.rng.pick(&rows);
            t.put(w + 0x48, v);
        }
        let v = t.rng.pick(&[0i16, 0, 1, 2, -1]);
        t.put(win_yn + 0x48, v);
        let v = t.rng.pick(&[0u32, 0, 1, 0x100]);
        t.put(selectable, v);
        let v = if (state == 4 || state == 10) && t.rng.below(3) == 0 {
            0
        } else {
            win_yn
        };
        t.put(yes_no, v);
        // the ability list's item: id, kind, record index
        let k = i32::from(Mem::read::<i16>(&t.a, win34 + 0x48)).cast_unsigned();
        let entry = items.wrapping_add(k.wrapping_mul(4));
        let mut entry_bytes = t.rng.bytes(4);
        if t.rng.below(8) != 0 {
            entry_bytes[2] = t
                .rng
                .pick(&[0xbu8, 0xb, 0xc, 0xd, 0xe, 0xf, 0x10, 0x11, 0xa, 0]);
        }
        t.put_bytes(entry, &entry_bytes);
        // the two names (equal or not), up to 0x3f bytes
        let len = t.rng.pick(&[0usize, 1, 5, 0x3f]);
        let mut a: Vec<u8> = (0..len).map(|_| (t.rng.below(3) + 0x41) as u8).collect();
        a.push(0);
        let mut b = a.clone();
        match t.rng.below(4) {
            0 if len > 0 => b[t.rng.below(len as u32) as usize] ^= 1,
            1 => {
                b.truncate(len.saturating_sub(1));
                b.push(0);
            }
            _ => {}
        }
        t.put_bytes(names, &a);
        t.put_bytes(names + 0x40, &b);
        t.put(names_cell, names);
        let v = t.rng.pick(&[0u32, 0x40]);
        t.put(step, v);
        t.twin(
            "UpdateGearCustomizationMenuState",
            0x008d_5830,
            &[m],
            U32,
            |o| {
                w64(tk::update_gear_customization_menu_state(
                    o,
                    m,
                    local,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn menu_round33_ctrl_limit_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0033_0003) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = vec![];
    for (f, n) in [
        (0x008b_e4b0u32, 0u8),
        (0x008c_1bb0, 0),
        (0x0088_6e80, 1),
        (0x008c_2c90, 2),
        (0x008a_afa0, 0),
        (0x008a_af10, 0),
        (0x008d_1a20, 1),
        (0x008a_ad70, 1),
        (0x008a_a100, 2),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let formats = [(0x00b6_5eecu32, 2u8)].map(|(f, n)| (t.sym(f), n));
    printf_log_n(&mut t, log, &formats);
    let pad = seq_stub(&mut t, 0x008b_e410, log, 0, &[]);
    let rep = seq_stub(&mut t, 0x008b_e470, log, 0, &[]);
    let player = seq_stub(&mut t, 0x008a_9860, log, 0, &[]);
    let kind = seq_stub(&mut t, 0x008c_1bf0, log, 1, &[]);
    let members = seq_stub(&mut t, 0x008a_9b30, log, 0, &[]);
    let ratio = seq_stub(&mut t, 0x008a_b220, log, 0, &[]);
    let (g1, g2) = (t.sym(0x0186_a714), t.sym(0x0186_a718));
    let (win1, win2, sub, m) = (
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x40]),
    );
    // MenuLimit_OpenWindow / OpenModeWindow put their window in the global
    for (f, g, w) in [(0x008d_0d50u32, g1, win1), (0x008d_0dd0, g2, win2)] {
        let cell = t.alloc(&w.to_le_bytes());
        let mut extra = vec![0xa1]; // mov eax, [cell]
        extra.extend(cell.to_le_bytes());
        extra.push(0xa3); // mov [g], eax
        extra.extend(g.to_le_bytes());
        seq_stub(&mut t, f, log, 1, &extra);
    }
    // TkMenuRestartSubCmdWindow2 sets both globals from cells, TkMn2RefreshList
    // masks them (a list gone after the refresh)
    let (c1, c2, mask1, mask2) = (
        t.alloc(&[0; 4]),
        t.alloc(&[0; 4]),
        t.alloc(&[0; 4]),
        t.alloc(&[0; 4]),
    );
    let mut extra = vec![];
    for (c, g) in [(c1, g1), (c2, g2)] {
        extra.push(0xa1); // mov eax, [c]
        extra.extend(c.to_le_bytes());
        extra.push(0xa3); // mov [g], eax
        extra.extend(g.to_le_bytes());
    }
    cells.push(seq_stub(&mut t, 0x008e_49d0, log, 0, &extra));
    let mut extra = vec![];
    for (c, g) in [(mask1, g1), (mask2, g2)] {
        extra.push(0xa1); // mov eax, [mask]
        extra.extend(c.to_le_bytes());
        extra.extend([0x21, 0x05]); // and [g], eax
        extra.extend(g.to_le_bytes());
    }
    cells.push(seq_stub(&mut t, 0x008c_2360, log, 1, &extra));
    t.put(t.sym(0x023c_c128), sub);
    let items = t.sym(0x0159_7770);
    let saved = t.sym(0x0186_a720);
    let answers = [0u8, 0, 1, 2, 0x7f, 0x80, 0xff];
    for round in 0..8000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        for w in [win1, win2, sub] {
            t.scramble_at(w, 0x80);
        }
        t.scramble_at(saved, 0x100);
        let state = if round % 50 == 49 {
            t.rng.pick(&[0xdu32, 0xffff_ffff, 0x8000_0000])
        } else {
            t.rng.below(13)
        };
        t.put(m + 0x1c, state);
        let v = t
            .rng
            .pick_or_any(&[4, 8, 4, 8, 0, 0xc, 0x1_0004, 0xffff_0008]);
        t.put(pad, v);
        let v = t.rng.pick_or_any(&[4, 8, 0xc, 0, 0xffff_fff3]);
        t.put(rep, v);
        let v = t.rng.below(16);
        t.put(player, v);
        let v = t.rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff]);
        t.put(members, v);
        let v = t.rng.pick(&[0u32, 1, 1, 0x100]);
        t.put(ratio, v);
        let (on1, on2) = (t.rng.below(4) != 0, t.rng.below(4) != 0);
        t.put(g1, if on1 { win1 } else { 0 });
        t.put(g2, if on2 { win2 } else { 0 });
        if !(on1 && on2) {
            // the member flip reads a list that is gone
            t.put(members, 1u32);
        }
        for (w, at) in [(win1, 0x45), (win2, 0x45), (sub, 0x45)] {
            let v = t.rng.pick(&answers);
            t.put(w + at, v);
        }
        let v = t.rng.pick(&[0i16, 0, 1, 1, 2, -1, 0x100, 0x101]);
        t.put(sub + 0x48, v);
        let k = if t.rng.below(8) == 0 {
            t.rng.pick(&[0xffff_ffffu32, 0xffff_fffe])
        } else {
            t.rng.below(0x200)
        };
        t.put(win2 + 0x48, k as u16);
        let id = t.rng.next() as u16;
        t.put(items + 4 * k, id);
        let v = if t.rng.below(2) == 0 {
            u32::from(id)
        } else {
            t.rng.next()
        };
        t.put(kind, v);
        let v = if t.rng.below(4) == 0 { 0 } else { win1 };
        t.put(c1, v);
        let v = if t.rng.below(4) == 0 { 0 } else { win2 };
        t.put(c2, v);
        // only where a flip's refresh is the last use (state 6's pick rereads the list)
        let flip2 =
            state == 4 || state == 5 || (state == 6 && Mem::read::<u8>(&t.a, win2 + 0x45) == 0);
        for (c, can) in [(mask1, (7..=9).contains(&state)), (mask2, flip2)] {
            let gone = can && t.rng.below(3) == 0;
            t.put(c, if gone { 0 } else { 0xffff_ffffu32 });
        }
        t.twin("TkMenuCtrlLimit", 0x008d_0460, &[m], U32, |o| {
            w64(tk::tk_menu_ctrl_limit(o, m, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn menu_round33_summon_grow_window_draw_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0033_0004) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008c_1790u32, 0u8),
        (0x008e_71d0, 1),
        (0x008a_9880, 0),
        (0x008c_1c90, 5),
        (0x008f_f490, 3),
        (0x008f_5f70, 5),
        (0x008f_8bb0, 5),
        (0x008c_0f70, 4),
        (0x008c_d990, 5),
        (0x008c_1370, 0),
        (0x008c_da20, 3),
        (0x008b_eeb0, 1),
        (0x008e_6cc0, 7),
        (0x0090_3bb0, 9),
        (0x0090_16b0, 7),
        (0x008d_4910, 1),
        (0x008d_4170, 2),
        (0x008c_0cc0, 0),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let grow = seq_stub(&mut t, 0x008c_1c20, log, 0, &[]);
    // ToGetCrossExtMesFontWidth(table, text, &w, scale, extra): w from a cell
    let (wcell, dummy) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let mut extra = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x15]; // mov eax, [esp+12]; mov edx, [wcell]
    extra.extend(wcell.to_le_bytes());
    extra.extend([0x89, 0x10]);
    let code = seq_code_of(log, 0x0090_5320, &[0, 1, 3, 4], dummy, &extra);
    jump_stub(&mut t, 0x0090_5320, &code);
    let (w, recs) = (t.alloc(&[0; 0x80]), t.alloc(&[0; 0x810]));
    t.put(grow, recs);
    let items = t.sym(0x0159_7770);
    let globals = [0x0186_a5e4u32, 0x0186_a5e6].map(|a| t.sym(a));
    // the label exactly at remapX2(660)
    let edge = {
        let x80 = ui_remap(&mut t.a, false, 80.0);
        let x660 = ui_remap(&mut t.a, false, 660.0);
        let near = ((x660 - x80) as f32).to_bits();
        (near - 4..near + 4)
            .map(f32::from_bits)
            .find(|&w| f64::from((x80 + f64::from(w)) as f32).to_bits() == x660.to_bits())
            .unwrap()
    };
    for _ in 0..3000 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w, 0x80);
        t.scramble_at(recs, 0x810);
        for g in globals {
            let v = t.rng.next() as u16;
            t.put(g, v);
        }
        let k = t.rng.pick(&[0u16, 1, 0x40, 0x1ff, 0xffff]);
        t.put(w + 0x48, k);
        // i16 -1 reads the entry before the list
        let entry = if k == 0xffff {
            items - 4
        } else {
            items + 4 * u32::from(k)
        };
        let mut bytes = t.rng.bytes(4);
        if t.rng.below(2) == 0 {
            bytes[2] = 0;
        }
        t.put_bytes(entry, &bytes);
        if t.rng.below(2) == 0 {
            let v = Mem::read::<u16>(&t.a, w + 0x32);
            t.put(w + 0x34, v);
        }
        let fade = t
            .rng
            .pick_or_any(&[0, 1, 0x1000, 0xf000, 0x7fff, 0x8000, 0x0800]) as u16;
        t.put(w + 0x46, fade);
        #[allow(clippy::cast_precision_loss)]
        let width = match t.rng.below(4) {
            0 => t.rng.below(2000) as f32,
            1 => t.rng.below(0x10000) as f32 / 64.0,
            2 => f32::from_bits(t.rng.next() & 0x7fff_ffff).min(1e30),
            _ => t.rng.pick(&[
                0f32,
                -0.0,
                400.0,
                560.0,
                580.0,
                1e9,
                f32::from_bits(0x7fc0_0000),
                edge,
                edge,
            ]),
        };
        t.put(wcell, width.to_bits());
        t.twin(
            "MenuSummon_GrowWindowDraw_008cdba0",
            0x008c_dba0,
            &[w],
            U32,
            |o| {
                w64(tk::menu_summon_grow_window_draw(
                    o,
                    w,
                    local,
                    &mut ui_remap,
                    &mut callback,
                ))
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn menu_round33_ctrl_equip_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0033_0005) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = vec![];
    for (f, n) in [
        (0x008b_f750u32, 5u8),
        (0x008a_9820, 1),
        (0x008a_9c70, 1),
        (0x008c_fc30, 0),
        (0x007a_b970, 3),
        (0x008c_3070, 1),
        (0x008b_f150, 1),
        (0x008a_ae90, 2),
        (0x008a_aef0, 2),
        (0x008e_33a0, 3),
        (0x008e_2de0, 0),
        (0x008e_49d0, 0),
        (0x008d_df60, 1),
        (0x008a_aeb0, 1),
        (0x0088_6e80, 1),
        (0x008a_afa0, 0),
        (0x008a_af10, 0),
        (0x008c_fc90, 2),
        (0x008c_fd40, 0),
        (0x008d_0430, 0),
        (0x008c_f970, 0),
        (0x008c_30d0, 0),
        (0x0087_51b0, 3),
        (0x008a_ad70, 1),
        (0x008a_a100, 2),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let formats =
        [(0x00b6_5df8u32, 1u8), (0x00b6_5e04, 1), (0x00b6_5e10, 1)].map(|(f, n)| (t.sym(f), n));
    printf_log_n(&mut t, log, &formats);
    let pads = [0x008b_e410u32, 0x008b_e470, 0x008b_e4b0].map(|f| seq_stub(&mut t, f, log, 0, &[]));
    let player = seq_stub(&mut t, 0x008a_9860, log, 0, &[]);
    let queue = seq_stub(&mut t, 0x0087_5000, log, 1, &[]);
    let (hp, mp) = (
        seq_stub(&mut t, 0x008a_98c0, log, 1, &[]),
        seq_stub(&mut t, 0x008a_9970, log, 1, &[]),
    );
    let yes_no = seq_stub(&mut t, 0x008e_3390, log, 0, &[]);
    let members = seq_stub(&mut t, 0x008a_9b30, log, 0, &[]);
    let ratio = seq_stub(&mut t, 0x008a_b220, log, 0, &[]);
    let (m, win, win_yn) = (
        t.alloc(&[0; 0x40]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
    );
    // an odd window pointer: state 19 returns it with the answer in its low byte
    let win = win + 0x13;
    t.put(t.sym(0x0186_a61c), win);
    let [fade, step, alt] = [0x0186_a614u32, 0x0186_a618, 0x0186_a624].map(|a| t.sym(a));
    let (hp_table, mp_table) = (t.sym(0x0186_a694), t.sym(0x0186_a6b4));
    let items = t.sym(0x0159_7770);
    let answers = [0u8, 0, 1, 2, 0x7f, 0x80, 0xff];
    for round in 0..8000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(win, 0x6d);
        t.scramble_at(win_yn, 0x80);
        let state = if round % 50 == 49 {
            t.rng.pick(&[0x1fu32, 0xffff_ffff, 0x8000_0000])
        } else if round % 4 == 1 {
            10
        } else {
            t.rng.below(31)
        };
        t.put(m + 0x1c, state);
        for c in pads {
            let v = t.rng.pick_or_any(&[
                1,
                2,
                5,
                9,
                8,
                4,
                0,
                4,
                8,
                0xc,
                0x20,
                0x40,
                0x60,
                0x1000,
                0x4000,
                0x1_0004,
                0xffff_0008,
            ]);
            t.put(c, v);
        }
        let v = t
            .rng
            .pick_or_any(&[0, 0x800, 0xfff, 0x1000, 0x1001, 0x7fff_ffff, 0xffff_ff00]);
        t.put(fade, v);
        let v = t
            .rng
            .pick_or_any(&[0, 1, 0x155, 0xffff_feab, 0x8000_0000, 0x7fff_ffff]);
        t.put(step, v);
        let v = t.rng.pick(&[0u32, 0, 1, 1, 2, 0xffff_ffff]);
        t.put(alt, v);
        let chr = t.rng.below(8);
        t.put(player, chr);
        let v = t.rng.pick(&[0u32, 0, 1]);
        t.put(queue, v);
        for (cell, table) in [(hp, hp_table), (mp, mp_table)] {
            let now = t.rng.pick_or_any(&[0, 1, 100, 9999, 0x8000_0000]);
            t.put(cell, now);
            let kept = match t.rng.below(3) {
                0 => now,
                1 => now.wrapping_add(1),
                _ => now.wrapping_sub(1),
            };
            t.put(table + 4 * chr, kept);
        }
        let v = if t.rng.below(5) == 0 { 0 } else { win_yn };
        t.put(yes_no, v);
        let v = t.rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff]);
        t.put(members, v);
        let v = t.rng.pick(&[0u32, 1, 1, 0x100]);
        t.put(ratio, v);
        for w in [win, win_yn] {
            let v = t.rng.pick(&answers);
            t.put(w + 0x45, v);
        }
        let v = t.rng.pick(&[0i16, 0, 1, 1, 2, -1]);
        t.put(win_yn + 0x2c, v);
        let k = t.rng.pick(&[0u16, 1, 0x40, 0x1ff, 0xffff]);
        t.put(win + 0x2c, k);
        // i16 -1 reads the entry before the list
        let entry = if k == 0xffff {
            items - 4
        } else {
            items + 4 * u32::from(k)
        };
        let id = if t.rng.below(3) == 0 {
            0
        } else {
            t.rng.next() as u16
        };
        t.put(entry, id);
        t.twin("TkMenuCtrlEquip", 0x008c_f020, &[m], U32, |o| {
            w64(tk::tk_menu_ctrl_equip(o, m, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn menu_round33_item_list_window_ctrl_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0033_0006) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = vec![];
    for (f, n) in [(0x0088_6e80u32, 1u8), (0x008c_7650, 1), (0x008a_b270, 1)] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let pads = [0x008b_e410u32, 0x008b_e470, 0x008b_e4b0].map(|f| seq_stub(&mut t, f, log, 0, &[]));
    // the window's +0x1c check
    let passed = t.alloc(&[0; 4]);
    let code = seq_code(log, 0x0000_001c, 3, passed, &[]);
    let check = test_code(&mut t, &code);
    let w = t.alloc(&[0; 0x800]);
    let bits = [0x20u32, 0x40, 0x8000, 0x2000, 0x1000, 0x4000, 1, 2];
    for round in 0..12000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w, 0xa0);
        let state = if round % 50 == 49 {
            t.rng.pick(&[0x15u32, 0xffff_ffff])
        } else if round % 3 == 1 {
            2
        } else {
            t.rng.below(21)
        };
        t.put(w + 0x28, state);
        for (k, c) in pads.into_iter().enumerate() {
            let mut v = 0u32;
            for _ in 0..t.rng.below(3) {
                v |= t.rng.pick(&bits);
            }
            if k == 0 && t.rng.below(2) == 0 {
                v = t.rng.pick(&[1u32, 2]);
            }
            if t.rng.below(8) == 0 {
                v |= t.rng.next() & 0xffff_0000;
            }
            t.put(c, v);
        }
        let picked = t.rng.pick(&[0u8, 0, 1, 2, 0xff, 0x7f]);
        t.put(w + 0x45, picked);
        let picks = t.rng.pick(&[1u8, 2, 2, 3, 0]);
        t.put(w + 0x42, picks);
        let v = if t.rng.below(4) == 0 { 0 } else { check };
        t.put(w + 0x1c, v);
        let v = t.rng.pick(&[0u32, 1, 0x100]);
        t.put(passed, v);
        let items = t.rng.pick_or_any(&[0, 1, 2, 5, 6, 30, 31]) % 0x100;
        t.put(w + 0x94, items);
        let rows = t.rng.pick(&[0u16, 1, 3, 15, 16, 40]);
        t.put(w + 0x30, rows);
        let page = t.rng.pick(&[0u16, 1, 8, 10]);
        t.put(w + 0x3a, page);
        let top = if t.rng.below(16) == 0 {
            t.rng.next() as i16
        } else {
            t.rng.pick(&[0i16, 0, 1, 5, 10, 30, -1, -17])
        };
        // a page down that ends exactly at a negative last page
        let page_down = state == 2 && t.rng.below(4) == 0;
        if page_down {
            for (c, v) in pads.into_iter().zip([2u32, 2, 0]) {
                t.put(c, v);
            }
        }
        let top = if page_down || t.rng.below(16) == 0 {
            i16::try_from(i32::from(rows) - 2 * i32::from(page)).unwrap()
        } else {
            top
        };
        t.put(w + 0x32, top);
        let v = if t.rng.below(2) == 0 {
            top
        } else {
            t.rng.pick(&[0i16, 2, 6, 31])
        };
        t.put(w + 0x34, v);
        let row = top.wrapping_add(t.rng.pick(&[0i16, 0, 1, 7, 9, 10, -1, 30]));
        t.put(w + 0x48, row);
        let timer = t.rng.pick_or_any(&[
            0, 0x333, 0x334, 0x111, 0x112, 0x1000, 0xf000, 0xfeef, 0xfccd, 0xffff, 0x8000,
        ]) as u16;
        t.put(w + 0x46, timer);
        for k in 0..4 {
            let v = t.rng.pick(&[0u32, 0, 1, 2]);
            t.put(w + 0x58 + 4 * k, v);
        }
        t.twin(
            "MenuItem_ListWindowCtrl_008c6dd0",
            0x008c_6dd0,
            &[w],
            U32,
            |o| w64(tk::menu_item_list_window_ctrl(o, w, &mut callback)),
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn menu_round33_wshop_ctrl_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0033_0007) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008b_e410u32, 0u8),
        (0x008b_e470, 0),
        (0x008b_e4b0, 0),
        (0x008d_94a0, 0),
        (0x008d_9550, 0),
        (0x008a_b260, 1),
        (0x008d_8110, 0),
        (0x008e_49d0, 0),
        (0x008d_8370, 0),
        (0x008d_7610, 0),
        (0x008d_8260, 0),
        (0x008d_7650, 0),
        (0x008d_9170, 1),
        (0x0088_6e80, 1),
        (0x0088_d390, 2),
        (0x008c_1b10, 0),
        (0x0088_d190, 3),
        (0x008d_8420, 0),
        (0x008d_df80, 1),
        (0x008e_33a0, 3),
        (0x008e_2de0, 0),
        (0x008d_81d0, 1),
        (0x008d_75f0, 0),
        (0x008d_9120, 1),
        (0x008d_8450, 0),
        (0x008d_82d0, 1),
        (0x008d_9410, 1),
        (0x007a_bca0, 1),
        (0x0078_59a0, 1),
        (0x008d_7630, 0),
        (0x0087_51b0, 3),
        (0x008e_36f0, 0),
        (0x008a_ad70, 1),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let sync = seq_stub(&mut t, 0x0088_d1c0, log, 1, &[]);
    let excel = seq_stub(&mut t, 0x008c_1af0, log, 2, &[]);
    let afford = seq_stub(&mut t, 0x008d_85d0, log, 1, &[]);
    let yes_no = seq_stub(&mut t, 0x008e_3390, log, 0, &[]);
    let loss = seq_stub(&mut t, 0x008d_8520, log, 1, &[]);
    // MsGetSaveWeapon(id, &name): the name word is the caller's slot, not logged
    let weapon = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x007a_bbd0, &[0], weapon, &[]);
    jump_stub(&mut t, 0x007a_bbd0, &code);
    let (m, sub, win_yn, rec, recs) = (
        t.alloc(&[0; 0x40]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x10]),
        t.alloc(&[0; 0x40]),
    );
    // odd window pointers: the answer replaces their low byte in eax
    let wins = [(); 4].map(|()| t.alloc(&[0; 0x90]) + 0x0b);
    let subs = [0x0186_aaf0u32, 0x0186_aaf4, 0x0186_aaf8, 0x0186_aafc].map(|a| t.sym(a));
    t.put(t.sym(0x023c_c128), sub);
    t.put(weapon, rec);
    t.put(excel, recs);
    let items = t.sym(0x0159_7770);
    let answers = [0u8, 0, 1, 2, 0x7f, 0x80, 0xff];
    for round in 0..8000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        for w in wins.into_iter().chain([sub, win_yn]) {
            t.scramble_at(w, 0x80);
        }
        t.scramble_at(rec, 0x10);
        t.scramble_at(recs, 0x40);
        for (g, w) in subs.into_iter().zip(wins) {
            let v = if t.rng.below(5) == 0 { 0 } else { w };
            t.put(g, v);
        }
        let state = if round % 50 == 49 {
            t.rng.pick(&[0x25u32, 0xffff_ffff])
        } else {
            t.rng.below(37)
        };
        t.put(m + 0x1c, state);
        // the windows a state reads must be there
        for (g, w) in subs.into_iter().zip(wins) {
            if Mem::read::<u32>(&t.a, g) == 0
                && matches!(
                    state,
                    0xc | 0x11 | 0x12 | 0x19 | 0x1a | 0x1c | 0x1e | 0x1f | 0x20
                )
            {
                t.put(g, w);
            }
        }
        for c in [sync, afford] {
            let v = t.rng.pick(&[0u32, 0, 1, 0x100]);
            t.put(c, v);
        }
        let v = if matches!(state, 0x10 | 0x1d) && t.rng.below(4) == 0 {
            0
        } else {
            win_yn
        };
        t.put(yes_no, v);
        let v = t.rng.pick_or_any(&[0, 1, 2, 3, 4]);
        t.put(loss, v);
        for w in wins.into_iter().chain([sub, win_yn]) {
            let v = t.rng.pick(&answers);
            t.put(w + 0x45, v);
            let v = t.rng.pick(&[0i16, 0, 1, 1, 2, 3, -1, 5]);
            t.put(w + 0x48, v);
        }
        let k = t.rng.pick(&[0u16, 1, 2, 3, 0x40, 0x1ff, 0xffff]);
        t.put(wins[1] + 0x48, k);
        let entry = if k == 0xffff {
            items - 4
        } else {
            items + 4 * u32::from(k)
        };
        let mut bytes = t.rng.bytes(4);
        if t.rng.below(3) == 0 {
            bytes[0] = 0;
            bytes[1] = 0;
        }
        if t.rng.below(3) == 0 {
            bytes[3] = 0;
        }
        t.put_bytes(entry, &bytes);
        for at in [3u32, 5] {
            let v = t.rng.pick(&[0u8, 0, 1, 4, 8, 0xd, 0x2]);
            t.put(rec + at, v);
        }
        for a in [
            0x0186_ab14u32,
            0x0186_ab04,
            0x0186_ab0c,
            0x0186_ab18,
            0x0186_ab1c,
        ] {
            let v = t.rng.pick_or_any(&[0, 1, 0x3f, 0x40, 0x41, 0xffff_ffc0]);
            t.put(t.sym(a), v);
        }
        t.twin("MenuWShop_Ctrl_008d6e40", 0x008d_6e40, &[m], U32, |o| {
            w64(tk::menu_wshop_ctrl(o, m, local, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn menu_round33_ctrl_summon_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0033_0008) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = vec![];
    for (f, n) in [
        (0x008b_e410u32, 0u8),
        (0x008b_e470, 0),
        (0x008b_e4b0, 0),
        (0x008e_49d0, 0),
        (0x008c_d1b0, 0),
        (0x008c_d1f0, 0),
        (0x008c_d140, 0),
        (0x008c_ed30, 0),
        (0x008c_ee30, 0),
        (0x008c_eda0, 0),
        (0x008c_ee50, 0),
        (0x0088_6e80, 1),
        (0x008c_cb40, 0),
        (0x008c_cb10, 0),
        (0x008c_d430, 0),
        (0x008d_4ed0, 0),
        (0x008e_2de0, 0),
        (0x0078_5c50, 3),
        (0x0079_0550, 2),
        (0x008c_2360, 1),
        (0x008c_d440, 0),
        (0x008c_2c60, 3),
        (0x008d_e180, 1),
        (0x008e_33a0, 3),
        (0x008b_ee70, 1),
        (0x008c_d240, 2),
        (0x008c_d3b0, 1),
        (0x008a_ad70, 1),
        (0x008a_a100, 2),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let formats = [(0x00b6_5d00u32, 4u8)].map(|(f, n)| (t.sym(f), n));
    printf_log_n(&mut t, log, &formats);
    let aeon = seq_stub(&mut t, 0x008a_9880, log, 0, &[]);
    let yes_no = seq_stub(&mut t, 0x008e_3390, log, 0, &[]);
    let grow = seq_stub(&mut t, 0x008c_1c20, log, 0, &[]);
    let entry_cell = seq_stub(&mut t, 0x008c_ecf0, log, 1, &[]);
    let (m, sub, win_yn, recs, entry) = (
        t.alloc(&[0; 0x40]),
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x810]),
        t.alloc(&[0; 0x10]),
    );
    let wins = [(); 4].map(|()| t.alloc(&[0; 0x90]) + 0x0b);
    // bytes before the sub-command window are scrambled too
    let sub = sub + 0x60;
    let globals = [0x0186_a5a0u32, 0x0186_a5a4, 0x0186_a5a8, 0x0186_a5ac].map(|a| t.sym(a));
    for (g, w) in globals.into_iter().zip(wins) {
        t.put(g, w);
    }
    t.put(grow, recs);
    t.put(entry_cell, entry);
    let sub_at = t.sym(0x023c_c128);
    let [restarts, grow_mask, afford_mask] =
        [0x0186_a5fcu32, 0x0186_a600, 0x0186_a604].map(|a| t.sym(a));
    let kinds = t.sym(0x0186_a5b0);
    let items = t.sym(0x0159_7770);
    let answers = [0u8, 0, 1, 2, 0x7f, 0x80, 0xff];
    for round in 0..10000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        for w in wins.into_iter().chain([win_yn]) {
            t.scramble_at(w, 0x80);
        }
        t.scramble_at(sub - 0x60, 0xb0);
        t.scramble_at(recs, 0x810);
        t.scramble_at(entry, 0x10);
        let state = if round % 50 == 49 {
            t.rng.pick(&[0x25u32, 0xffff_ffff])
        } else {
            t.rng.below(37)
        };
        t.put(m + 0x1c, state);
        let v = if state == 0xd && t.rng.below(4) == 0 {
            0
        } else {
            sub
        };
        t.put(sub_at, v);
        let v = if state == 0x12 && t.rng.below(4) == 0 {
            0
        } else {
            wins[2]
        };
        t.put(globals[2], v);
        let v = if matches!(state, 0x14 | 0x17 | 0x1c | 0x1f) && t.rng.below(4) == 0 {
            0
        } else {
            win_yn
        };
        t.put(yes_no, v);
        let v = t.rng.pick(&[0u32, 1, 0xd, 0xe, 0xffff_ffff]);
        t.put(aeon, v);
        let v = t.rng.pick(&[0u32, 1, 2, 0xffff_ffff]);
        t.put(restarts, v);
        for g in [grow_mask, afford_mask] {
            let v = t.rng.next() & t.rng.next();
            t.put(g, v);
        }
        for w in wins.into_iter().chain([sub, win_yn]) {
            let v = t.rng.pick(&answers);
            t.put(w + 0x45, v);
            let v = t.rng.pick(&[0i16, 0, 1, 1, 2, 3, 5, 31, 32, -1]);
            t.put(w + 0x48, v);
        }
        let row = i32::from(Mem::read::<i16>(&t.a, sub + 0x48)).cast_unsigned();
        // (row -1 reads the top byte of the stat list's pointer before the table)
        if row < 0x40 {
            let v = t.rng.pick(&[0u8, 0, 1, 1, 2, 3, 0xff]);
            t.put(kinds + row, v);
        }
        let k = i32::from(Mem::read::<i16>(&t.a, wins[2] + 0x48)).cast_unsigned();
        let mut bytes = t.rng.bytes(4);
        bytes[2] = t.rng.pick(&[4u8, 4, 5, 6, 7, 0]);
        t.put_bytes(items.wrapping_add(k.wrapping_mul(4)), &bytes);
        t.twin("TkMenuCtrlSummon", 0x008c_c330, &[m], U32, |o| {
            w64(tk::tk_menu_ctrl_summon(o, m, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn menu_round33_name_entry_window_ctrl_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0033_0009) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = vec![];
    for (f, n) in [
        (0x008b_e410u32, 0u8),
        (0x008d_3280, 0),
        (0x008a_aeb0, 1),
        (0x008a_b260, 1),
        (0x0088_6e80, 1),
        (0x008c_5260, 2),
        (0x008c_5320, 1),
        (0x008c_44d0, 0),
        (0x008c_4c90, 0),
        (0x008b_edd0, 1),
        (0x008e_33a0, 3),
        (0x008e_2de0, 0),
        (0x008c_4e60, 0),
        (0x0086_0f20, 1),
        (0x008c_33f0, 1),
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let [
        rep,
        trg,
        rows,
        side_len,
        first_read,
        lang,
        table_cell,
        matches,
        yes_no,
        mask,
        ratio,
        megas,
    ] = [
        0x008b_e470u32,
        0x008b_e4b0,
        0x008c_5180,
        0x008c_5030,
        0x008a_8e50,
        0x008a_c2f0,
        0x008c_4ff0,
        0x008c_5390,
        0x008e_3390,
        0x008a_a460,
        0x008a_b230,
        0x0086_0a90,
    ]
    .map(|f| {
        let n = u8::from(f == 0x008c_5390);
        seq_stub(&mut t, f, log, n, &[])
    });
    let (w, win_yn, first, second, table) = (
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
        t.alloc(&[0; 0x80]),
    );
    t.put(table_cell, table + 0x10);
    t.put(w + 0x64, second);
    let names = [
        0x0186_a258u32,
        0x0186_a25c,
        0x0186_a260,
        0x0186_a264,
        0x0186_a268,
        0x0186_a26c,
        0x0186_a270,
        0x0186_a274,
    ]
    .map(|a| t.sym(a));
    let [target, page, _last, flag, cell, side, in_side, help] = names;
    let typed = t.sym(0x0186_a280);
    let bits = [
        0x20u32, 0x40, 0x100, 0x800, 1, 2, 4, 8, 0x1000, 0x2000, 0x4000, 0x8000,
    ];
    for round in 0..12000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w, 0x60);
        t.scramble_at(win_yn, 0x80);
        t.scramble_at(table, 0x80);
        for k in 0..0x18u32 {
            let v = t.rng.pick(&[0u16, 1, 2, 3, 4, 5, 6]);
            t.put(table + 0x12 + 4 * k, v);
        }
        let state = if round % 50 == 49 {
            t.rng.pick(&[0xcu32, 0xffff_ffff])
        } else if round % 3 == 1 {
            4
        } else {
            t.rng.below(12)
        };
        t.put(w + 0x28, state);
        let v = if t.rng.below(2) == 0 { first } else { 0 };
        t.put(w + 0x60, v);
        for c in [rep, trg] {
            let mut v = 0u32;
            for _ in 0..t.rng.below(3) {
                v |= t.rng.pick(&bits);
            }
            if t.rng.below(8) == 0 {
                v |= t.rng.next() & 0xffff_0000;
            }
            t.put(c, v);
        }
        let v = t.rng.pick(&[1u32, 2, 3, 5, 6]);
        t.put(rows, v);
        let v = t.rng.pick(&[1u32, 3, 6, 8]);
        t.put(side_len, v);
        for (c, of) in [
            (first_read, &[0u32, 1][..]),
            (lang, &[0, 0, 1, 9, 10, 11, 2][..]),
            (matches, &[0, 1][..]),
            (mask, &[0, 8, 0xf7, 0xffff_ffff][..]),
            (ratio, &[0, 0x7f, 0x80, 0x100, 0xffff_ffff][..]),
            (megas, &[0, 1, 2][..]),
        ] {
            let v = t.rng.pick(of);
            t.put(c, v);
        }
        let v = if state == 8 && t.rng.below(4) == 0 {
            0
        } else {
            win_yn
        };
        t.put(yes_no, v);
        let v = t.rng.pick(&[0u8, 0, 1, 2, 0x80, 0xff]);
        t.put(win_yn + 0x45, v);
        let v = t.rng.pick(&[0i16, 0, 1, 1, 2, -1]);
        t.put(win_yn + 0x48, v);
        let v = t
            .rng
            .pick(&[0xffff_ffffu32, 0, 1, 7, 0xf, 0x10, 0x11, 0x12]);
        t.put(target, v);
        let v = t.rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff]);
        t.put(page, v);
        let v = t.rng.pick(&[0u32, 1]);
        t.put(flag, v);
        let v = t.rng.pick_or_any(&[
            0,
            1,
            14,
            15,
            16,
            29,
            44,
            45,
            74,
            75,
            89,
            90,
            200,
            0xffff_ffff,
            0xffff_fff1,
        ]);
        t.put(cell, v);
        let v = t.rng.pick(&[0u32, 1, 2, 5, 7, 0xffff_ffff]);
        t.put(side, v);
        let v = t.rng.pick(&[0u32, 0, 1, 1, 2]);
        t.put(in_side, v);
        let v = t.rng.pick(&[0u8, 1, 0x41]);
        t.put(typed, v);
        let v = t.rng.next();
        t.put(help, v);
        t.twin(
            "MenuNameEntry_WindowCtrl_008c3580",
            0x008c_3580,
            &[w],
            VOID,
            |o| {
                tk::menu_name_entry_window_ctrl(o, w, &mut callback);
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round34_target_window_draw_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0034_0001) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    // results drawn at random for every case
    let mut cells = vec![];
    for (f, n) in [
        (0x008a_9ad0u32, 1u8), // TkMenuGetPlayerFromIndex2
        (0x008a_9bc0, 1),      // TkMenuGetSummonFromIndex
        (0x008a_c850, 1),      // TOGetSaveChrName
        (0x008a_9970, 1),      // TkMenuGetMP
        (0x008a_99b0, 1),      // TkMenuGetMaxMP
        (0x008a_98c0, 1),      // TkMenuGetHP
        (0x008a_9990, 1),      // TkMenuGetMaxHP
        (0x008a_99f0, 1),      // MenuPlayer_DamageClass
        (0x008f_5f70, 5),      // TODrawMenuPlateXYWHType
        (0x0090_3bb0, 9),      // TOMkpShapeXYWHUV
        (0x0090_5550, 6),      // ToMakeBtlEasyDigit2
        (0x0090_1660, 6),      // TOMkpCrossEasyStrFontSClut
        (0x0090_5930, 6),      // ToMakeBtlEasyEdgeFont
        (0x008c_0670, 3),      // TkMn2DrawCrossCursor
        (0x008c_13d0, 3),      // TkMn2DrawStatiCursor
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let players = seq_stub(&mut t, 0x008a_9b30, log, 0, &[]);
    let aeons = seq_stub(&mut t, 0x008a_9bf0, log, 0, &[]);
    let ease = seq_stub(&mut t, 0x008d_30c0, log, 2, &[]);
    let lang = seq_stub(&mut t, 0x008a_c2f0, log, 0, &[]);
    let timer = seq_stub(&mut t, 0x008a_9c60, log, 0, &[]);
    let (item_menu, single, mask) = (t.sym(0x0186_a408), t.sym(0x0186_a391), t.sym(0x0186_a3a4));
    let w = t.alloc(&[0; 0x80]);
    for case in 0..1500 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(w, 0x80);
        let n = if case % 50 == 0 {
            t.rng.pick(&[17u32, 33, 40, 0xffff_ffff, 0x8000_0000])
        } else {
            t.rng.below(13)
        };
        // the other list's count is never read
        let other = t.rng.next();
        let mode = t.rng.pick(&[0u32, 0, 1, 2, 0x100]);
        t.put(w + 0x58, mode);
        let (p, a) = if mode == 0 { (n, other) } else { (other, n) };
        t.put(players, p);
        t.put(aeons, a);
        for o in [0x5c, 0x60] {
            let v = t.rng.pick(&[0u32, 1, 1, 2, 0x101]);
            t.put(w + o, v);
        }
        let row = t.rng.pick(&[0u16, 1, 2, 5, 8, 0xffff]);
        t.put(w + 0x48, row);
        let dir = t.rng.pick(&[0x8000u16, 0xffff, 0, 1, 0x7fff]);
        t.put(w + 0x54, dir);
        let v = t.rng.pick_or_any(&[
            0,
            1,
            0x400,
            0x800,
            0x1000,
            0xffff_f000,
            0x7fff_ffff,
            0x8000_0000,
        ]);
        t.put(ease, v);
        let v = t.rng.pick_or_any(&[0xb, 0xb, 0, 1, 0x10b]);
        t.put(lang, v);
        let v = t
            .rng
            .pick_or_any(&[0, 1, 2, 3, 0xffff_ffff, 0xffff_fffd, 0x8000_0000]);
        t.put(timer, v);
        let v = t.rng.pick(&[0u32, 1, 2, 0x8000_0001]);
        t.put(item_menu, v);
        let v = t.rng.pick(&[0u8, 1, 1, 2, 0xff]);
        t.put(single, v);
        let v = t.rng.pick(&[0u16, 0xffff, 0x8001, 0x5555, 4]);
        t.put(mask, v);
        t.twin(
            "MenuItem_TargetWindowDraw_008c80b0",
            0x008c_80b0,
            &[w],
            VOID,
            |o| {
                tk::menu_item_target_window_draw(o, w, &mut ui_remap, &mut callback);
                0
            },
        );
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round34_ctrl_main_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0034_0002) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = vec![];
    for (f, n) in [
        (0x008b_e410u32, 0u8), // TOMenuGetControlPad
        (0x008a_9860, 0),      // TkMenuGetCurrentPlayer
        (0x008a_ad60, 1),      // TkMenuMainSetIgnoreJoinFlag
        (0x008a_ae80, 1),      // TkMenuSetCurrentPlayerPos
        (0x008d_df60, 1),      // TkMenuMainHelp
        (0x008a_aeb0, 1),      // TkMenuSetHelpMessage
        (0x008a_afc0, 0),      // TkMenuSetPrevPlayer2
        (0x008a_af40, 0),      // TkMenuSetNextPlayer2
        (0x008a_b260, 1),      // TkMn2SetMainFrameRatioDiff
        (0x008a_9b00, 1),      // TkMenuGetPlayerFromIndex2_008a9b00
        (0x008c_30d0, 0),      // TkMsSetSaveParamAll
        (0x008a_9720, 2),      // TkMenuExchangePlayerPos
        (0x008a_a410, 1),      // TkMenuMainFreeWindowGroup
        (0x008a_9680, 1),      // TkMenuSetDrawShadeRatioStep
        (0x008a_a100, 2),      // TkMenuMainActiveMenu
        (0x008a_ad70, 1),      // TkMenuMainSleepMenu
        (0x008a_ab20, 1),      // TkMenuMainRemoveMenu
        (0x0088_6e80, 1),      // SndSepPlaySimple
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    // TkMenuMainGetMaxOptions and MenuMain_OptionY keep ecx (the caller
    // reads it after them): push ecx / pop ecx, and ecx = the argument
    let options = t.alloc(&[0; 4]);
    let code = [
        &[0x51u8][..],
        &seq_code(log, 0x008e_2280, 0, options, &[0x59]),
    ]
    .concat();
    jump_stub(&mut t, 0x008e_2280, &code);
    cells.push(options);
    let mut picked = vec![];
    for (f, n) in [
        (0x008b_e470u32, 0u8), // TOMenuGetControlPadRep
        (0x008b_e4b0, 0),      // TOMenuGetControlPadTrg
        (0x008e_2290, 1),      // TkMenuMainGetOptionEnabled
        (0x008e_2270, 1),      // TkMenuMainGetOptionMenu
        (0x008e_24c0, 1),      // TkMenuMainOpenOption
        (0x008a_9870, 0),      // TkMenuGetCurrentPlayerPos
        (0x008a_9b30, 0),      // TkMenuGetPlayerListMax
        (0x008a_9b50, 0),      // TkMenuGetPlayerListMax2
        (0x0078_5380, 1),      // MsGetSavePlyJoin
        (0x008a_98a0, 0),      // TkMenuGetDrawShadeRatio
        (0x008a_8e50, 0),      // TOMenuGetFirstReadEndFlag
        (0x008a_b230, 0),      // TkMn2GetMainFrameRatio
    ] {
        picked.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let option_y = seq_stub(&mut t, 0x008e_2260, log, 1, &[0x8b, 0x4c, 0x24, 0x04]);
    picked.insert(4, option_y);
    let m = t.alloc(&[0; 0x40]);
    let (fade, busy, cursor, from, to, done) = (
        t.sym(0x0187_1544),
        t.sym(0x0187_1564),
        t.sym(0x0187_1548),
        t.sym(0x0187_155c),
        t.sym(0x0187_1560),
        t.sym(0x0187_1554),
    );
    let small = [0u32, 1, 2, 3, 4, 0xffff_ffff, 0x80, 0x7f];
    for round in 0..12000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        t.scramble_at(m, 0x40);
        // the input states twice as often
        let state = if round % 40 == 39 {
            t.rng.pick_or_any(&[0x27, 0x28, 0xffff_ffff])
        } else if round % 2 == 0 {
            t.rng.pick(&[4, 5, 7, 0xe, 0x13, 0x14])
        } else {
            round % 0x27
        };
        t.put(m + 0x1c, state);
        let high = t.rng.next() & 0xffff_0000;
        let rep = t.rng.pick_or_any(&[0, 0x1000, 0x4000, 0x4001, 0x5000, 0]) | high;
        t.put(picked[0], rep);
        let high = t.rng.next() & 0xffff_0000;
        let trg = t.rng.pick_or_any(&[0, 0x20, 0x40, 0x60, 0]) | high;
        t.put(picked[1], trg);
        let v = t.rng.pick_or_any(&[0, 1]);
        t.put(picked[2], v);
        let v = t.rng.pick_or_any(&[6, 6, 0, 1]);
        t.put(picked[3], v);
        let v = t.rng.pick_or_any(&[0, 0, 1, 2, 2]);
        t.put(picked[4], v);
        let v = t.rng.pick_or_any(&[3, 9, 0]);
        t.put(picked[5], v);
        for &c in &picked[6..10] {
            let v = t.rng.pick_or_any(&small);
            t.put(c, v);
        }
        for &c in &picked[10..] {
            let v = t
                .rng
                .pick_or_any(&[0, 1, 0x7f, 0x80, 0xffff_ffff, 0x8000_0000]);
            t.put(c, v);
        }
        let v = t
            .rng
            .pick(&[0u16, 0xf55, 0xf56, 0xf57, 0x1000, 0x7f60, 0x7fff, 0xffff]);
        t.put(fade, v);
        let v = t.rng.pick(&[0u16, 0, 1, 0x100]);
        t.put(busy, v);
        let v = t.rng.pick(&[0u32, 0, 1, 0x100]);
        t.put(done, v);
        for c in [cursor, from, to] {
            let v = t.rng.pick_or_any(&small);
            t.put(c, v);
        }
        // the option count around the cursor
        let v = t.rng.pick_or_any(&small);
        t.put(cells[18], v);
        t.twin("TkMenuCtrlMain", 0x008e_0350, &[m], U32, |o| {
            w(tk::tk_menu_ctrl_main(o, m, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round34_btl_end_draw_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0034_0003) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x0088_e750u32, 1u8), // Math_CosLookup
        (0x0088_e730, 1),      // Math_SinLookup
        (0x008b_ef30, 0),      // TOMenuOpenPktBuffTmp
        (0x0090_3bb0, 9),      // TOMkpShapeXYWHUV
        (0x008e_7d30, 12),     // DrawWaterWaveShapeC2
        (0x008a_c850, 1),      // TOGetSaveChrName
        (0x0090_5930, 6),      // ToMakeBtlEasyEdgeFont
        (0x008b_da10, 1),      // BtlEnd_GetResult
        (0x0090_55c0, 6),      // ToMakeBtlEasyDigitRight
        (0x008a_9b80, 1),      // TkMenuGetSP
        (0x0090_5550, 6),      // ToMakeBtlEasyDigit2
        (0x0090_1660, 6),      // TOMkpCrossEasyStrFontSClut
        (0x0078_4e90, 1),      // MsGetNextAP
        (0x0078_52b0, 1),      // MsGetSaveNextAP
        (0x008f_5f70, 5),      // TODrawMenuPlateXYWHType
        (0x008b_ea20, 0),      // TOMenuDrawKickTmp
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    // ToGetBtlEasyDigitWidth2(value, &width, scale, extra): the width from
    // a cell; the out pointer (a frame local) is not logged
    let width = t.alloc(&[0; 4]);
    let mut extra = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15]; // mov eax, [esp+8]; mov edx, [width]
    extra.extend(width.to_le_bytes());
    extra.extend([0x89, 0x10]); // mov [eax], edx
    let wcell = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x0090_5230, &[0, 2, 3], wcell, &extra);
    jump_stub(&mut t, 0x0090_5230, &code);
    let lang = seq_stub(&mut t, 0x008a_c2f0, log, 0, &[]);
    let level = seq_stub(&mut t, 0x008b_da50, log, 1, &[]);
    let list = t.sym(0x0186_9f20);
    for round in 0..3000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let i = t.rng.pick(&[0u32, 1, 2, 3, 4, 5, 6, 7]);
        let entry = list + 14 * i;
        let b = t.rng.bytes(14);
        t.put_bytes(entry, &b);
        let kind = t.rng.pick(&[1u16, 1, 2, 3, 4, 4, 0, 5, 6, 0xffff, 0x8001]);
        t.put(entry + 4, kind);
        let flash = t.rng.pick(&[0u8, 1, 12, 13, 14, 15, 16, 0x7f, 0x80, 0xff]);
        t.put(entry + 0xa, flash);
        let cos = t.rng.pick_or_any(&[0, 0x1000, 0xffff, 0x8000, 0x1_0800]);
        t.put(cells[0], cos);
        let sin = t.rng.pick_or_any(&[0, 0x1000, 0xffff, 0x8000, 0x1_0800]);
        t.put(cells[1], sin);
        let v = t.rng.pick_or_any(&[0xb, 0xb, 0, 1]);
        t.put(lang, v);
        let v = t.rng.pick_or_any(&[98, 99, 100, 0, 0xffff_ffff]);
        t.put(level, v);
        let v = if round % 3 == 0 {
            quiet_float(&mut t)
        } else {
            t.rng.pick(&[0f32, 24.0, 37.5, -8.0, 1e30]).to_bits()
        };
        t.put(width, v);
        t.twin("TkBtlEndDraw", 0x008b_c340, &[i], U32, |o| {
            w(tk::tk_btl_end_draw(
                o,
                i.cast_signed(),
                local,
                &mut ui_remap,
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round34_btl_end_ctrl_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0034_0004) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x008a_bd00u32, 0u8), // TODebugAddPadPrintFlag
        (0x0088_6e80, 1),      // SndSepPlaySimple
        (0x008b_e0a0, 0),      // MsExchangeSaveWeapon
        (0x0088_6960, 1),      // SndMusicRead
        (0x0088_6a20, 1),      // SndMusicStart
        (0x0088_6eb0, 1),      // SndSepStop
        (0x008b_e050, 1),      // SetEntries_01869f20
        (0x008b_da60, 1),      // TkBtlEndGetText
        (0x008b_dfe0, 2),      // TOSetPair_008bdfe0
        (0x008a_f0d0, 2),      // TOSeqStartScene
        (0x0064_48d0, 0),      // graphicUnlockScreenFadeColor
        (0x0064_3900, 4),      // graphicSetScreenFadeColor
        (0x0063_9280, 5),      // graphicAddScreenFadeTaskAfterRender
        (0x0064_38e0, 1),      // graphicSetScreenFadeAlphaMode
        (0x0064_1e20, 0),      // graphicLockScreenFadeColor
        (0x008b_df60, 0),      // TOMenuFieldMusic_008bdf60
        (0x0082_cb80, 1),      // Sg_FadeIn
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let mut picked = vec![];
    for (f, n) in [
        (0x008b_e410u32, 0u8), // TOMenuGetControlPad
        (0x008b_e470, 0),      // TOMenuGetControlPadRep
        (0x008b_e4b0, 0),      // TOMenuGetControlPadTrg
        (0x008b_a5c0, 0),      // TkBtlEndCheckLegendWeapon
        (0x008b_b690, 1),      // BtlEnd_StepEntry
        (0x0086_bc20, 0),      // AtelIsKeepPlayStatusClear8
        (0x0088_7b30, 0),      // TkSndReportStatus
        (0x0088_63f0, 0),      // SndGetNowPlayMusic
        (0x0088_7900, 1),      // SndSyncMusicStandby
        (0x008b_da50, 1),      // BtlEnd_GetSP
        (0x0064_1cb0, 0),      // graphicIsScreenFadeColorLocked
        (0x0086_bc60, 0),      // AtelGetKeepPlayStatusNoStopFanfare
        (0x0087_2b50, 0),      // AtelCheckMenuOtakaraFlag
        (0x0086_bc10, 0),      // AtelIsKeepPlayStatusClear12
    ] {
        picked.push(seq_stub(&mut t, f, log, n, &[]));
    }
    // BtlEnd_StepGil and BtlEnd_CountApSteps count down to 0 (the
    // original loops until they do)
    let mut countdown = vec![];
    for f in [0x008b_a590u32, 0x008b_a550] {
        let cell = t.alloc(&[0; 4]);
        // cmp dword [cell], 0; je +6; dec dword [cell]: down to 0, then 0
        let mut extra = vec![0x83, 0x3d];
        extra.extend(cell.to_le_bytes());
        extra.extend([0x00, 0x74, 0x06, 0xff, 0x0d]);
        extra.extend(cell.to_le_bytes());
        let code = seq_code(log, f, 0, cell, &extra);
        jump_stub(&mut t, f, &code);
        countdown.push(cell);
    }
    // the weapon records: MsGetSaveWeapon(id, &name) (the name word is
    // a frame local: not logged) and BtlEnd_SelectedWeapon()
    let record = t.alloc(&[0; 0x20]);
    let wcell = t.alloc(&record.to_le_bytes());
    let code = seq_code_of(log, 0x007a_bbd0, &[0], wcell, &[]);
    jump_stub(&mut t, 0x007a_bbd0, &code);
    let scell = t.alloc(&record.to_le_bytes());
    let code = seq_code(log, 0x008b_da80, 0, scell, &[]);
    jump_stub(&mut t, 0x008b_da80, &code);
    // GetCD() -> a record whose +8 callback is logged
    let cd_cell = t.alloc(&[0; 4]);
    let cd_fn = test_code(&mut t, &seq_code(log, 0x00cd_0008, 1, cd_cell, &[]));
    let mut cd = vec![0u8; 0x10];
    cd[8..12].copy_from_slice(&cd_fn.to_le_bytes());
    let cd = t.alloc(&cd);
    let cd_ret = t.alloc(&cd.to_le_bytes());
    let code = seq_code(log, 0x0076_d0d0, 0, cd_ret, &[]);
    jump_stub(&mut t, 0x0076_d0d0, &code);
    let ids = t.alloc(&[0; 0x100]);
    t.put(t.sym(0x025c_c580), ids + 0x40);
    let state = t.sym(0x0186_9f08);
    let small = [0u32, 1, 2, 5, 6, 0xffff_ffff];
    for round in 0..20000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let s = if round % 50 == 49 {
            t.rng.pick_or_any(&[0x2c, 0xffff_ffff])
        } else if round % 2 == 0 {
            t.rng
                .pick(&[0x20u32, 0x20, 0x25, 0x25, 0x21, 0x26, 0x18, 0xc, 0xf, 0x15])
        } else {
            (round / 2) % 0x2c
        };
        t.put(state, s);
        let v = t.rng.pick_or_any(&[0, 1, 2, 1, 2]);
        t.put(picked[0], v);
        let rep = t.rng.pick_or_any(&[0, 0x20, 0x1000, 0x4000, 1, 2, 3, 0x21]);
        t.put(picked[1], rep);
        let trg = t
            .rng
            .pick_or_any(&[0, 0x100, 0x20, 0x40, 0x8000, 0x2000, 0x10100, 0x60]);
        t.put(picked[2], trg);
        for &c in &picked[3..] {
            let v = t.rng.pick_or_any(&[0, 1, 0x8a, 98, 99, 0xffff_ffff]);
            t.put(c, v);
        }
        for &c in &countdown {
            let v = 1 + t.rng.below(4);
            t.put(c, v);
        }
        t.scramble_at(record, 0x20);
        let v = t.rng.pick(&[0u8, 1, 0xff]);
        t.put(record + 2, v);
        let v = t.rng.pick(&[0u8, 0xff, 0xff, 1]);
        t.put(record + 6, v);
        t.scramble_at(ids, 0x100);
        // the fades, their speeds and the counter
        for (a, picks) in [
            (
                0x0186_a0b4u32,
                &[0u16, 0x1000, 0xfff, 0x7f80, 0x8000, 0x10][..],
            ),
            (0x0186_a160, &[0, 0x1000, 0xf90, 0x7fff][..]),
            (0x0186_a168, &[0, 0x1000, 0xf80, 0xffff][..]),
            (0x0186_a170, &[0, 0x1000, 0xe00, 0x7e00][..]),
            (0x0186_a178, &[0, 0xf00, 0xf01, 0x7f00][..]),
            (0x0186_a114, &[0, 0x60, 0xff80][..]),
            (0x0186_a164, &[0, 0x80, 0xff80][..]),
            (0x0186_a16c, &[0, 0x80, 0xff80][..]),
            (0x0186_a174, &[0, 0x200, 0xfe00][..]),
        ] {
            let v = t.rng.pick(picks);
            t.put(t.sym(a), v);
        }
        // the word between two fades (a misread neighbour must show)
        t.scramble(0x0186_a172, 2);
        let v = t.rng.pick(&[0u8, 1, 2, 3, 4]);
        t.put(t.sym(0x0186_9f18), v);
        let v = t.rng.pick(&[0u8, 1]);
        t.put(t.sym(0x0186_9f19), v);
        t.scramble(0x0186_9f20, 14 * 4);
        for a in [0x0186_9f14u32, 0x0186_a184] {
            let v = t.rng.pick(&[0u32, 1]);
            t.put(t.sym(a), v);
        }
        for a in [0x0231_0f70u32, 0x0231_0f72] {
            let v = t.rng.pick(&[0u8, 0, 1]);
            t.put(t.sym(a), v);
        }
        let v = t.rng.pick(&[5u32, 6, 7, 0]);
        t.put(t.sym(0x0186_a17c), v);
        let v = t.rng.pick(&[0u32, 3, 7, 8, 11, 12, 20]);
        t.put(t.sym(0x0186_a188), v);
        for a in [0x0186_a190u32, 0x0186_a194] {
            let v = t
                .rng
                .pick(&[0u32, 1, 2, 6, 7, 13, 0xffff_ffff, 0xffff_fff7]);
            t.put(t.sym(a), v);
        }
        let v = t
            .rng
            .pick(&[0u32, 0x1000, 0xffff_f000, 0x111, 0x333, 0x222]);
        t.put(t.sym(0x0186_a18c), v);
        let v = t.rng.pick(&[0x111u32, 0x333, 0x1000]);
        t.put(t.sym(0x0186_a198), v);
        let v = t.rng.pick(&[0u32, 0, 1, 1, 2]);
        t.put(t.sym(0x0186_a19c), v);
        // (cursor + top indexes the gear ids: small values only)
        for k in 0..4 {
            let v = t.rng.pick(&small);
            t.put(t.sym(0x0186_a1a0) + 4 * k, v);
        }
        let v = t.rng.pick_or_any(&small);
        t.put(t.sym(0x0186_a1a8), v);
        t.twin("TkBtlEndCtrl", 0x008b_a660, &[], VOID, |o| {
            tk::tk_btl_end_ctrl(o, local, &mut callback);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

/// A text of `n` bytes (any non-zero) and eight zeros: a two- or
/// three-byte character can swallow the first zero and still stop.
fn padded_text(t: &mut Twins, n: u32) -> u32 {
    let mut v = text_of(t, n);
    v.extend([0; 7]);
    t.alloc(&v)
}

/// A key text of mixed characters: one byte, two (6..=0x2f), and after
/// 4 two (a second byte from 0x30) or three, then zeros.
fn mixed_key_text(t: &mut Twins) -> u32 {
    let mut v = vec![];
    for _ in 0..=t.rng.below(5) {
        match t.rng.below(4) {
            0 => v.push(0x41 + u8::try_from(t.rng.below(0x30)).unwrap_or(0)),
            1 => v.extend([6 + u8::try_from(t.rng.below(0x2a)).unwrap_or(0), 0x55]),
            2 => v.extend([4, t.rng.pick(&[0x30u8, 0x31, 0x80])]),
            _ => v.extend([4, t.rng.pick(&[0x2fu8, 0x10]), 0x66]),
        }
    }
    v.extend([0; 8]);
    t.alloc(&v)
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round34_small_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0034_0005) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x0082_66f0u32, 1u8), // Ch_Dispose
        (0x008b_3d20, 0),      // TkMenuSaveLoad_PollCard_008b3d20
        (0x008b_ef10, 2),      // TOMenuHelpSetExtMode
        (0x008a_b260, 1),      // TkMn2SetMainFrameRatioDiff
        (0x008b_0920, 1),      // TOSwapInternationalEnvInit
        (0x008b_3d90, 0),      // TkMenuSaveLoadCloseWindow2
        (0x008a_ad70, 1),      // TkMenuMainSleepMenu
        (0x008a_a100, 2),      // TkMenuMainActiveMenu
        (0x0064_3e70, 0),      // graphicSwitchGPUMode
        (0x0088_6e80, 1),      // SndSepPlaySimple
        (0x0088_c830, 0),      // TkGetSaveEventAddress
        (0x008b_54a0, 2),      // TkMenuCopyLoadedSave
        (0x008c_c170, 0),      // iMusicRefreshBgmType
        (0x008d_e080, 1),      // TkMenuSaveMes
        (0x008a_aeb0, 1),      // TkMenuSetHelpMessage
        (0x0087_24f0, 0),      // TkRefreshSoundMode
        (0x0087_24a0, 0),      // TkRefreshHdd
        (0x008a_f4f0, 2),      // TOSeqStopScene
        (0x008a_97e0, 0),      // eiAbmParaGet's thunk
        (0x0079_0340, 0),      // PasswordTextRead
        (0x008d_4ed0, 0),      // TkMn2InitStatusBG
        (0x008d_2660, 1),      // TMn2SetStatusBGDiff
        (0x008a_ab00, 1),      // TkMenuMainRegistWindow
        (0x008c_43f0, 1),      // MenuNameEntry_OpenCharWindow
        (0x008c_5050, 0),      // MenuNameEntry_LoadLayout
        (0x008d_4920, 0),      // TkMn2GetStatusPos
        (0x008c_4450, 0),      // MenuNameEntry_RegistWindow
        (0x008a_c850, 1),      // TOGetSaveChrName
        (0x008c_5280, 1),      // Menu_ParseItemKeys
        (0x0088_ec60, 0),      // TkVU1SyncPath
        (0x008b_ef30, 0),      // TOMenuOpenPktBuffTmp
        (0x008f_f370, 4),      // TODrawMesBox_008ff370
        (0x008f_f3e0, 4),      // TODrawMesBoxFit_008ff3e0
        (0x008b_ea20, 0),      // TOMenuDrawKickTmp
        (0x008b_e410, 0),      // TOMenuGetControlPad
        (0x008c_ba60, 0),      // MenuParty_CursorPrev
        (0x008c_ba20, 0),      // MenuParty_CursorNext
        (0x008d_c9b0, 1),      // TkMenuConfigMes
        (0x008e_3690, 3),      // MenuSubCmd_OpenC
        (0x008b_0970, 1),      // TOSwapInternationalEnvInit_008b0970
        (0x008e_2de0, 0),      // TOMenuCloseYesNoWindow
        (0x0090_0f40, 0),      // TOMenuInitBrush
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let mut p = std::collections::HashMap::new();
    for (f, n) in [
        (0x0082_61b0u32, 1u8), // Ch_CheckHandle
        (0x0078_51d0, 0),      // MsGetSaveConfigEnglish
        (0x008a_b240, 0),      // TkMn2GetMenuMode
        (0x008a_c390, 0),      // TOGetFFXLangSwapFlag
        (0x008b_0500, 0),      // TOSwapInternationalEnvExecNonBlock
        (0x008a_c2f0, 0),      // TOGetFFXLangJmp
        (0x008a_a1a0, 0),      // TkMenuMainAllocWindow
        (0x008c_5180, 0),      // MenuItem_KeyRowCount
        (0x0079_02d0, 0),      // PasswordGetTotalIndex
        (0x0079_02a0, 1),      // PasswordText_GetEntry_007902a0
        (0x008b_edd0, 1),      // TOMenuGetNameMes
        (0x008a_98e0, 0),      // GetGlobal01841c0c_008a98e0
        (0x008b_e470, 0),      // TOMenuGetControlPadRep
        (0x008b_e4b0, 0),      // TOMenuGetControlPadTrg
        (0x008b_0440, 0),      // TOSwapInternationalEnvExecMenu
    ] {
        p.insert(f, seq_stub(&mut t, f, log, n, &[]));
    }
    // ToMakeBtlEasyFont(buf, x, y, alpha, scale, extra): the values and
    // then the buffer's 4 bytes (the buffer is a frame local)
    let l = log.to_le_bytes();
    #[rustfmt::skip]
    let extra: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00][..], // mov eax, [esp+4]; mov eax, [eax]
        &[0x8b, 0x0d], &l, &[0x89, 0x01],          // mov ecx, [log]; mov [ecx], eax
        &[0x83, 0x05], &l, &[0x04],                // add dword [log], 4
    ]
    .concat();
    let font_cell = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x0090_5ab0, &[1, 2, 3, 4, 5], font_cell, &extra);
    jump_stub(&mut t, 0x0090_5ab0, &code);
    // TOGetMessageWH(_, text, &w, &h, _, _): the width from a cell
    let width = t.alloc(&[0; 4]);
    #[rustfmt::skip]
    let mut extra: Vec<u8> = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x15]; // mov eax, [esp+12]; mov edx, [width]
    extra.extend(width.to_le_bytes());
    extra.extend([0x89, 0x10]); // mov [eax], edx
    let wh_cell = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x008b_9650, &[0, 1, 4, 5], wh_cell, &extra);
    jump_stub(&mut t, 0x008b_9650, &code);
    // row callbacks (+0x10, +0x14) of the config rows
    let row_cell = t.alloc(&[0; 4]);
    let row_fn = test_code(&mut t, &seq_code(log, 0x00c0_0010, 1, row_cell, &[]));
    let rows = t.alloc(&[0; 0x100]);
    let rows_list = t.alloc(&[0; 0x20]);
    for k in 0..8u32 {
        t.put(rows_list + 4 * k, rows + 0x20 * k);
        t.put(rows + 0x20 * k + 0x10, row_fn);
        t.put(rows + 0x20 * k + 0x14, row_fn);
    }
    t.put(t.sym(0x0186_a44c), rows_list);
    let window = t.alloc(&[0; 0x80]);
    t.put(p[&0x008a_a1a0], window);
    // the character buffer starts as the same non-zero junk on both
    // sides (the original's in its frame): a missing terminator shows
    let zero = |t: &mut Twins| {
        let end = t.a.stack_range().end;
        t.a.write(end - 0x1000, &[0xaau8; 0xf00]).unwrap();
        t.b.write(local, &[0xaau8; 8]).unwrap();
    };
    for round in 0..4000u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        for (&f, &c) in &p {
            if f != 0x008a_a1a0 {
                let v = t.rng.pick_or_any(&[0, 1, 2, 9, 0xa, 0xb]);
                t.put(c, v);
            }
        }
        let v = t.rng.pick(&[0u32, 1, 6, 7, 9]);
        t.put(p[&0x008c_5180], v);
        let v = t.rng.pick(&[0u32, 1, 3, 4]);
        t.put(p[&0x0079_02d0], v);
        // (each function runs every 8th round: alternate on round / 8)
        let text = if (round / 8) % 2 == 0 {
            mixed_key_text(&mut t)
        } else {
            let n = t.rng.below(6);
            padded_text(&mut t, n)
        };
        t.put(p[&0x0079_02a0], text);
        t.put(p[&0x008b_edd0], text);
        let v = t.rng.pick(&[0u32, 1, 2]);
        t.put(t.sym(0x0186_a25c), v);
        let v = t.rng.pick(&[0u32, 3, 0xffff_ffff]);
        t.put(t.sym(0x0186_a258), v);
        for a in [0x0186_6240u32, 0x0186_a43c] {
            let v = t.rng.pick(&[0u32, 1]);
            t.put(t.sym(a), v);
        }
        let v = t.rng.pick(&[0u8, 1]);
        t.put(t.sym(0x0186_628a), v);
        // 22528: 0x200000 - 98 s is -4096 * 27 (truncation shows a +1)
        let v = t
            .rng
            .pick_or_any(&[0, 1, 0x800, 0x1000, 0xffff_fffb, 22528]);
        t.put(p[&0x008a_98e0], v);
        t.scramble(0x0133_c910, 8);
        let v = t.rng.pick(&[0f32, 100.0, 481.5, 600.0]).to_bits();
        t.put(width, v);
        t.scramble_at(rows, 0x100);
        for k in 0..8u32 {
            t.put(rows + 0x20 * k + 0x10, row_fn);
            t.put(rows + 0x20 * k + 0x14, row_fn);
            let v = t.rng.pick(&[1u32, 2, 3]);
            t.put(rows + 0x20 * k + 4, v);
            let v = t.rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff]);
            t.put(rows + 0x20 * k + 8, v);
        }
        let v = t.rng.pick(&[0u32, 1, 3, 8]);
        t.put(t.sym(0x0186_a448), v);
        let v = t.rng.pick(&[0u8, 1, 7]);
        t.put(t.sym(0x0186_a444), v);
        match round % 8 {
            0 => {
                let chr = t.rng.next();
                t.twin("TkCh_Dispose", 0x0087_22f0, &[chr], U32, |o| {
                    w(tk::tk_ch_dispose(o, chr, &mut callback))
                });
            }
            1 => {
                let m = window;
                t.twin("MenuConfig_Open_008cb3c0", 0x008c_b3c0, &[m], U32, |o| {
                    w(tk::menu_config_open(o, m, &mut callback))
                });
            }
            2 => t.twin("TkMenuCloseSaveLoadScreen", 0x008b_3db0, &[], U32, |o| {
                w(tk::tk_menu_close_save_load_screen(o, &mut callback))
            }),
            3 => t.twin("TkMenuJumpToLoadedScene", 0x008b_4ec0, &[], U32, |o| {
                w(tk::tk_menu_jump_to_loaded_scene(o, &mut callback))
            }),
            4 => {
                let aeon = t.rng.pick_or_any(&[0, 1]);
                let target = t
                    .rng
                    .pick(&[0u32, 3, 7, 8, 0x11, 0x12, 0xffff_ffff, 0x8000_0000]);
                t.scramble_at(window, 0x80);
                t.twin(
                    "TkMenuOpenNameEntryWindow",
                    0x008c_4250,
                    &[aeon, target],
                    U32,
                    |o| {
                        w(tk::tk_menu_open_name_entry_window(
                            o,
                            aeon,
                            target.cast_signed(),
                            &mut callback,
                        ))
                    },
                );
            }
            5 => {
                zero(&mut t);
                t.twin(
                    "MenuNameEntry_DrawKeys_008c4550",
                    0x008c_4550,
                    &[],
                    U32,
                    |o| {
                        w(tk::menu_name_entry_draw_keys(
                            o,
                            local,
                            &mut ui_remap,
                            &mut callback,
                        ))
                    },
                );
            }
            6 => {
                let m = window;
                let v = t.rng.pick(&[0u32, 1, 7, 0xffff_ffff]);
                t.put(m + 0x58, v);
                t.twin(
                    "MenuSave_DrawMesBox_008b4920",
                    0x008b_4920,
                    &[m],
                    U32,
                    |o| w(tk::menu_save_draw_mes_box(o, m, local, &mut callback)),
                );
            }
            _ => {
                let m = window;
                let s = t
                    .rng
                    .pick_or_any(&[0, 1, 2, 3, 3, 3, 4, 5, 6, 7, 8, 9, 0xa, 0xb, 0xc]);
                t.put(m + 0x1c, s);
                let v = t.rng.pick_or_any(&[0, 0x1000, 0x4000, 0x5000, 1]);
                t.put(p[&0x008b_e470], v);
                let v = t
                    .rng
                    .pick_or_any(&[0, 0x8000, 0x2000, 0xa000, 0x40, 0xa040, 1]);
                t.put(p[&0x008b_e4b0], v);
                t.twin("TkMenuCtrlConfig", 0x008c_b0e0, &[m], VOID, |o| {
                    tk::tk_menu_ctrl_config(o, m, &mut callback);
                    0
                });
            }
        }
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn menu_round34_ctrl_tuto_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0034_0006) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let mut cells = vec![];
    for (f, n) in [
        (0x0088_8e80u32, 3u8), // TkGetControlerTrigHist
        (0x008c_a570, 1),      // TkMenuSetDummyControlerFlag
        (0x0088_d190, 3),      // TkCdReadNB
        (0x0078_5ed0, 1),      // MsSetSaveConfigEnglish
        (0x008d_b7e0, 0),      // MenuTuto_RestoreNames
        (0x008a_ae10, 0),      // TkMenuRefreshPartyList
        (0x008a_97e0, 0),      // eiAbmParaGet's thunk
        (0x008c_30d0, 0),      // TkMsSetSaveParamAll
        (0x008b_e370, 3),      // TOMenuAtelReq
        (0x008a_b260, 1),      // TkMn2SetMainFrameRatioDiff
        (0x008a_a100, 2),      // TkMenuMainActiveMenu
        (0x008a_9b30, 0),      // TkMenuGetPlayerListMax
        (0x008a_9bf0, 0),      // TkMenuGetSummonListMax
        (0x0088_6dd0, 1),      // SndSepLock
        (0x008a_ab20, 1),      // TkMenuMainRemoveMenu
        (0x0078_51d0, 0),      // MsGetSaveConfigEnglish
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    let vcount = seq_stub(&mut t, 0x0088_7dc0, log, 0, &[]);
    let first = seq_stub(&mut t, 0x008a_8e50, log, 0, &[]);
    let sync = seq_stub(&mut t, 0x0088_d1c0, log, 1, &[]);
    let mask = seq_stub(&mut t, 0x008a_a460, log, 0, &[]);
    let buffer = t.alloc(&vec![0; 0x6a00]);
    let buf_cell = seq_stub(&mut t, 0x008c_1b10, log, 0, &[]);
    t.put(buf_cell, buffer);
    let formats = [
        (0x00b6_672cu32, 0u8),
        (0x00b6_6744, 2),
        (0x00b6_676c, 1),
        (0x00b6_6780, 1),
        (0x00b6_6798, 1),
        (0x00b6_67b0, 0),
        (0x00b6_67c4, 1),
    ]
    .map(|(f, n)| (t.sym(f), n));
    printf_log_n(&mut t, log, &formats);
    let m = t.alloc(&[0; 0x40]);
    for round in 0..1200u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let s = if round % 30 == 29 {
            t.rng.pick_or_any(&[0xb, 0xffff_ffff])
        } else {
            (round / 2) % 0xb
        };
        t.put(m + 0x1c, s);
        let v = t.rng.pick(&[0u32, 1, 2, 4]);
        t.put(vcount, v);
        for c in [first, sync] {
            let v = t.rng.pick(&[0u32, 1]);
            t.put(c, v);
        }
        let v = t.rng.pick_or_any(&[0, 1, 2, 0x40_0000]);
        t.put(mask, v);
        let v = t.rng.pick(&[0u32, 1, 2]);
        t.put(t.sym(0x0187_1528), v);
        t.put(t.sym(0x0187_1520), buffer);
        t.put(t.sym(0x0187_1524), buffer + 0x80);
        if round % 4 == 0 {
            t.scramble_at(buffer, 0x6a00);
        }
        t.twin("TkMenuCtrlTuto", 0x008d_b4e0, &[m], VOID, |o| {
            tk::tk_menu_ctrl_tuto(o, m, &mut callback);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
#[allow(clippy::too_many_lines)]
fn menu_round35_main_draw_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x0035_0ba0) else {
        return;
    };
    let log = seq_log(&mut t);
    let start = Mem::read::<u32>(&t.a, log);
    let local = t.b.stack_range().start + 0x100;
    let mut cells = vec![];
    for (f, n) in [
        (0x0088_ec60u32, 0u8), // TkVU1SyncPath
        (0x008b_ef30, 0),      // TOMenuOpenPktBuffTmp
        (0x008f_5c10, 0),      // TODrawMenuBG
        (0x0088_e750, 1),      // Math_CosLookup
        (0x0090_3bb0, 9),      // TOMkpShapeXYWHUV
        (0x008e_7d30, 12),     // DrawWaterWaveShapeC2
        (0x008a_9a70, 1),      // MenuPlayer_DamageClass2
        (0x008a_c850, 1),      // TOGetSaveChrName
        (0x0090_5930, 6),      // ToMakeBtlEasyEdgeFont
        (0x008a_9b80, 1),      // TkMenuGetSP
        (0x0090_5820, 6),      // ToMakeBtlEasyEdgeDigit2
        (0x008c_0670, 3),      // TkMn2DrawCrossCursor
        (0x008c_13d0, 3),      // TkMn2DrawStatiCursor
        (0x008f_4b20, 6),      // TODrawCrossBoxXYWHC2
        (0x008a_98b0, 0),      // jmp MsGetGIL
        (0x008c_0a20, 5),      // TkMn2DrawGillPlate
        (0x008a_9c50, 0),      // TkMenuGetTime
        (0x008e_19f0, 5),      // TOMenuDrawTimer
        (0x0090_3ee0, 11),     // TOMkpShapeXYWHUVC2
        (0x0086_c410, 0),      // AtelGetSaveDic
        (0x0078_51d0, 0),      // MsGetSaveConfigEnglish
        (0x0086_c430, 2),      // AtelGetSaveDicName
        (0x008d_c9e0, 1),      // TkMenuDraw1612Width
        (0x0090_16b0, 7),      // TOMkpCrossExtMesFontLClut
        (0x008b_ea20, 0),      // TOMenuDrawKickTmp
    ] {
        cells.push(seq_stub(&mut t, f, log, n, &[]));
    }
    // the sprintf texts are logged as strings
    let text_cell = t.alloc(&[0; 4]);
    let code = seq_code_str_args(log, 0x0090_1660, &[1, 2, 3, 4, 5], text_cell);
    jump_stub(&mut t, 0x0090_1660, &code);
    // ToGetBtlEasyDigitWidth2(value, &width, scale, extra): the width from
    // a cell; the out pointer (a frame local) is not logged
    let width = t.alloc(&[0; 4]);
    let mut extra = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15]; // mov eax, [esp+8]; mov edx, [width]
    extra.extend(width.to_le_bytes());
    extra.extend([0x89, 0x10]); // mov [eax], edx
    let wcell = t.alloc(&[0; 4]);
    let code = seq_code_of(log, 0x0090_5230, &[0, 2, 3], wcell, &extra);
    jump_stub(&mut t, 0x0090_5230, &code);
    // TkMenuGetPlayerFromIndex2(i) = i ^ [key]: rows get their own characters
    let key = t.alloc(&[0; 4]);
    let mut extra = vec![0x8b, 0x44, 0x24, 0x04, 0x33, 0x05]; // mov eax, [esp+4]; xor eax, [key]
    extra.extend(key.to_le_bytes());
    extra.push(0xc3); // ret
    let code = seq_code(log, 0x008a_9b00, 1, key, &extra);
    jump_stub(&mut t, 0x008a_9b00, &code);
    let count = seq_stub(&mut t, 0x008a_9b50, log, 0, &[]);
    let members = seq_stub(&mut t, 0x008a_9b70, log, 0, &[]);
    // TOGetFFXLangJmp: [lang], then [lang] += [flip] (1: 0xb comes once,
    // so the offsets a row keeps from an earlier one show)
    let lang = t.alloc(&[0; 4]);
    let flip = t.alloc(&[0; 4]);
    let mut extra = vec![0xa1]; // mov eax, [lang]
    extra.extend(lang.to_le_bytes());
    extra.extend([0x8b, 0x15]); // mov edx, [flip]
    extra.extend(flip.to_le_bytes());
    extra.extend([0x01, 0x15]); // add [lang], edx
    extra.extend(lang.to_le_bytes());
    extra.push(0xc3); // ret
    let code = seq_code(log, 0x008a_c2f0, 0, lang, &extra);
    jump_stub(&mut t, 0x008a_c2f0, &code);
    let timer = seq_stub(&mut t, 0x008a_9c60, log, 0, &[]);
    let values: Vec<u32> = [0x008a_98c0u32, 0x008a_9990, 0x008a_9970, 0x008a_99b0]
        .iter()
        .map(|&f| seq_stub(&mut t, f, log, 1, &[]))
        .collect();
    let slides = t.sym(0x0187_1678);
    let (choice, from, to) = (t.sym(0x0187_154c), t.sym(0x0187_155c), t.sym(0x0187_1560));
    let rows = [0xffff_ffffu32, 0, 1, 2, 3, 5, 7, 8];
    for round in 0..1200u32 {
        t.put(log, start);
        reroll(&mut t, &cells);
        let n = t.rng.pick(&[0u32, 1, 3, 3, 5, 7, 8, 8, 0xffff_ffff]);
        t.put(count, n);
        let m = t.rng.pick(&[0u32, 1, 2, 3, 3, 8, 0xffff_ffff]);
        t.put(members, m);
        for k in 0..8 {
            let v = t
                .rng
                .pick(&[0u16, 0, 1, 0x10, 0x400, 0x7fff, 0x8000, 0xfff0]);
            t.put(slides + 2 * k, v);
        }
        let k = t.rng.pick(&[0u32, 0, 1, 2, 5]);
        t.put(key, k);
        for c in [choice, from, to] {
            let v = t.rng.pick(&rows);
            t.put(c, v);
        }
        let v = t.rng.pick(&[0xbu32, 0xb, 0, 1, 7, 9, 0xa]);
        t.put(lang, v);
        let v = t.rng.pick(&[0u32, 0, 1]);
        t.put(flip, v);
        let v = t
            .rng
            .pick(&[0u32, 1, 2, 3, 0xffff_fffe, 0xffff_fffd, 0x8000_0000]);
        t.put(timer, v);
        // HP/MP: values whose "%5d/%5d" texts fit the original's 16 bytes
        for &c in &values {
            let v = t.rng.pick(&[0u32, 1, 999, 9999, 99_999, 0xffff_fffb]);
            let v = if t.rng.below(2) == 0 {
                v
            } else {
                t.rng.below(100_000)
            };
            t.put(c, v);
        }
        let v = if round % 3 == 0 {
            quiet_float(&mut t)
        } else {
            t.rng.pick(&[0f32, 24.0, 37.5, -8.0, 1e30]).to_bits()
        };
        t.put(width, v);
        t.twin("TkMenuDrawMain", 0x008e_0ba0, &[], U32, |o| {
            w(tk::tk_menu_draw_main(
                o,
                [local, local + 32],
                &mut ui_remap,
                &mut callback,
            ))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// Small round 1 e: `TkDrawNoTrans` (`AtelDrawNoTrans` counted,
/// `TkSndUpdate2` from a cell), `TkMenuOpenNameEntry_008c3390` (the save
/// name and the name text stand-ins return buffers, the window call
/// recorded) and `TkChPatchM160` (`Ch_ModelSetAlpha` recorded, counting
/// down a word the test points at the part count now and then).
#[test]
fn small1_menu_wrappers_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x5a11_0e01) else {
        return;
    };
    let count = t.alloc(&[0; 4]);
    let code = [&[0xff, 0x05][..], &count.to_le_bytes(), &[0xc3]].concat();
    stub(&mut t, 0x0087_2c90, &code);
    let snd = return_cell(&mut t, 0x0088_7b80);
    let dst = t.alloc(&[0; 0x40]);
    record_n(&mut t, 0x0078_4fb0, 1, dst);
    let src = t.alloc(&[0; 0x40]);
    record_n(&mut t, 0x008b_edd0, 1, src);
    record_n(&mut t, 0x008c_4250, 2, 0x0123_4567);
    // Ch_ModelSetAlpha: its three words logged, then `dec word [[cell]]`
    let buf = t.alloc(&vec![0; 0x2_0000]);
    let log = t.alloc(&buf.to_le_bytes());
    let cell = t.alloc(&[0; 4]);
    let dummy = t.alloc(&[0; 4]);
    let l = log.to_le_bytes();
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat();
    for k in 0..3u8 {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    code.extend([&[0x83, 0x05][..], &l, &[12]].concat());
    code.extend([&[0xa1][..], &cell.to_le_bytes(), &[0x66, 0xff, 0x08, 0xc3]].concat());
    stub(&mut t, 0x0082_ae30, &code);
    let chr = t.alloc(&[0; 0x200]);
    let block = t.alloc(&[0; 0x10]);
    t.put(chr + 0x1c0, block);
    let mut callback = callback;
    for round in 0..400u32 {
        let v = t.rng.next();
        t.put(snd, v);
        t.twin("TkDrawNoTrans", 0x0088_d4f0, &[], U32, |o| {
            w(tk::tk_draw_no_trans(o, &mut callback))
        });
        // a name of 0..=0x30 bytes (no 0 inside), the old one random
        let len = t.rng.below(0x31);
        let mut text = t.rng.bytes(0x40);
        for b in &mut text {
            *b |= 1;
        }
        text[len as usize] = 0;
        t.put_bytes(src, &text);
        let old = t.rng.bytes(0x40);
        t.put_bytes(dst, &old);
        let id = t.rng.next();
        t.put(t.sym(0x00c5_a14c), id);
        let target = t.rng.pick(&[0xbu32, 0xb, 0, 0xa, 0xc, 0x10b, 0xffff_ffff]);
        let aeon = t.rng.next();
        t.twin(
            "TkMenuOpenNameEntry_008c3390",
            0x008c_3390,
            &[aeon, target],
            U32,
            |o| w(tk::tk_menu_open_name_entry_8c3390(o, target, &mut callback)),
        );
        // 0x101: a count past a byte
        let parts = t.rng.pick(&[0u16, 1, 2, 5, 0x11, 0x40, 0x101]);
        t.put(block + 6, parts);
        t.put(cell, if round % 3 == 0 { block + 6 } else { dummy });
        t.put(log, buf);
        let c = if round % 9 == 0 { 0 } else { chr };
        t.twin("TkChPatchM160", 0x0082_b9b0, &[c], VOID, |o| {
            tk::tk_ch_patch_m160(o, c, &mut callback);
            0
        });
    }
    eprintln!("{} calls", t.calls);
}

#[test]
fn small3_c_menu_allocators_match_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x5a11_3c03) else {
        return;
    };
    // the allocators log their words and return a cell's word; every
    // callee logs its words
    let logged_cell = |t: &mut Twins, f: u32, n: u8| {
        let cell = t.alloc(&[0; 4]);
        let buf = t.alloc(&vec![0; 0x1_0000]);
        let log = t.alloc(&buf.to_le_bytes());
        let l = log.to_le_bytes();
        let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat(); // mov ecx, [log]
        for k in 0..n {
            code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
        }
        code.extend([&[0x83, 0x05][..], &l, &[4 * n]].concat()); // add dword [log], 4n
        code.extend([&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat()); // mov eax, [cell]
        stub(t, f, &code);
        (cell, log, buf)
    };
    let (chr, log1, buf1) = logged_cell(&mut t, 0x0082_4f90, 1);
    let (chr2, log2, buf2) = logged_cell(&mut t, 0x0082_5650, 2);
    let _ = record_n(&mut t, 0x0083_6870, 2, 0x55);
    let _ = record_n(&mut t, 0x0082_aea0, 3, 0x66);
    let _ = record_n(&mut t, 0x0082_b9b0, 1, 0x77);
    let _ = record_n(&mut t, 0x0082_ba00, 1, 0x88);
    let mut callback = callback;
    for _ in 0..300u32 {
        t.put(log1, buf1);
        t.put(log2, buf2);
        let key = t
            .rng
            .pick(&[3u32, 7, 0x10a0, 0x1075, 0x1153, 0, 4, 0x1_0003, 0x10a1]);
        let r = t.rng.next();
        let c = t.rng.pick(&[0u32, 0x1234_5678, r]);
        t.put(chr, c);
        t.twin("TkCh_Allocate", 0x0087_2190, &[key], U32, |o| {
            w(tk::tk_ch_allocate(o, key, &mut callback))
        });
        let v = t.rng.next();
        let r = t.rng.next();
        let c = t.rng.pick(&[0u32, 0x1234_5678, r]);
        t.put(chr2, c);
        t.twin("TkCh_Allocate2", 0x0087_2210, &[key, v], U32, |o| {
            w(tk::tk_ch_allocate2(o, key, v, &mut callback))
        });
    }
    eprintln!("{} calls", t.calls);
}

/// Small round 4 c: `MenuNameEntry_Draw_008c3d00(window)`, every callee a
/// logging stand-in (the remaps real): the frames', plate's and shape's
/// words as the original builds them in place, the window passed on, the
/// tail call's `eax`.
#[test]
fn small4_c_name_entry_draw_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x5a11_4c03) else {
        return;
    };
    // the calls without words: a tag each into one log
    let tags = t.alloc(&[0; 0x400]);
    let tag_log = t.alloc(&tags.to_le_bytes());
    for (tag, f) in [(1u32, 0x008d_ce50u32), (2, 0x008c_1790), (3, 0x008c_0be0)] {
        let l = tag_log.to_le_bytes();
        let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &l].concat(); // mov ecx, [log]
        code.extend([&[0xc7, 0x01][..], &tag.to_le_bytes()].concat()); // mov [ecx], tag
        code.extend([&[0x83, 0x05][..], &l, &[4, 0xc3]].concat()); // add dword [log], 4; ret
        stub(&mut t, f, &code);
    }
    let logs: Vec<u32> = [
        (0x008f_4df0u32, 6u8),
        (0x008f_5f70, 5),
        (0x0090_3bb0, 9),
        (0x008c_4770, 1),
        (0x008c_4550, 1),
        (0x008c_4b60, 3),
    ]
    .into_iter()
    .map(|(f, n)| record_n(&mut t, f, n, 0x1234_5678))
    .collect();
    let mut callback = callback;
    for k in 0..20u32 {
        t.put(tag_log, tags);
        let ret = t.rng.next();
        let _ = record_n(&mut t, 0x008c_0cc0, 0, ret);
        let r = t.rng.next();
        let window = t.rng.pick(&[0u32, 0x0186_a278, r]);
        t.twin("MenuNameEntry_Draw", 0x008c_3d00, &[window], U32, |o| {
            w(tk::menu_name_entry_draw(
                o,
                window,
                &mut ui_remap,
                &mut callback,
            ))
        });
        if k == 0 {
            // the first frame's w, remapX2(1100), as logged
            let w = f32::from_bits(Mem::read::<u32>(&t.b, logs[0] + 8));
            assert!((293.0..294.0).contains(&w), "{w}");
        }
    }
    eprintln!("{} calls", t.calls);
}

/// Small round 6 d: `MenuSummon_Open_008cc1b0` for every key-item
/// combination (`MsImportantGet` from a table by the item's low 3 bits);
/// the texts are the kind + 1, the helps + 0x100; the window logs its nine
/// words and the `n` texts and helps behind its pointers (the rest of the
/// original's frame stays unwritten); the cursor and centre calls logged,
/// `TkMenuGetCurrentSummonPos` a counter.
#[test]
fn small6_d_summon_open_matches_the_original() {
    use ffx_menu::tk::bridge as tk;
    let Some(mut t) = setup_any(0x5a11_6d01) else {
        return;
    };
    let local = t.b.stack_range().start + 0x100;
    stub(&mut t, 0x008d_e180, &[0x8b, 0x44, 0x24, 0x04, 0x40, 0xc3]);
    stub(
        &mut t,
        0x008d_e160,
        &[0x8b, 0x44, 0x24, 0x04, 0x05, 0, 1, 0, 0, 0xc3],
    );
    let held = t.alloc(&[0; 32]);
    #[rustfmt::skip]
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x07][..], // mov eax, [esp + 4]; and eax, 7
        &[0x8b, 0x04, 0x85], &held.to_le_bytes(),        // mov eax, [eax * 4 + held]
        &[0xc3],
    ]
    .concat();
    stub(&mut t, 0x0079_0800, &code);
    let (_, mut code) = log_args(&mut t, 9);
    let log_cell = u32::from_le_bytes(code[2..6].try_into().unwrap());
    let l = log_cell.to_le_bytes();
    // the two pointers are stack addresses: logged as 0
    code.extend([0x31, 0xc0, 0x89, 0x41, 0x04, 0x89, 0x41, 0x08]); // xor eax, eax; [ecx + 4] = [ecx + 8] = 0
    #[rustfmt::skip]
    code.extend([
        &[0x56, 0x57][..],                         // push esi; push edi
        &[0x8b, 0x4c, 0x24, 0x18],                 // mov ecx, [esp + 0x18] (n)
        &[0x8b, 0x3d], &l,                         // mov edi, [log]
        &[0x8b, 0x74, 0x24, 0x10, 0xf3, 0xa5],     // mov esi, texts; rep movsd
        &[0x8b, 0x4c, 0x24, 0x18],                 // mov ecx, n
        &[0x8b, 0x74, 0x24, 0x14, 0xf3, 0xa5],     // mov esi, helps; rep movsd
        &[0x89, 0x3d], &l,                         // mov [log], edi
        &[0x5f, 0x5e, 0xb8, 0x77, 0, 0, 0, 0xc3],  // pop edi; pop esi; mov eax, 0x77; ret
    ]
    .concat());
    jump_stub(&mut t, 0x008e_4930, &code);
    return_counter(&mut t, 0x008e_4ab0, 0x11);
    record_far(&mut t, 0x008e_3b80, 1, 0x22);
    return_counter(&mut t, 0x008a_9890, 0x100);
    // the remaps as v + 1 (X) and v + 2 (Y), so their order and axes show
    for (f, add) in [(0x0064_47e0u32, 1.0f32), (0x0064_4820, 2.0)] {
        let c = t.alloc(&add.to_bits().to_le_bytes());
        #[rustfmt::skip]
        let code: Vec<u8> = [
            &[0xd9, 0x44, 0x24, 0x04][..],  // fld dword [esp + 4]
            &[0xd8, 0x05], &c.to_le_bytes(), // fadd dword [c]
            &[0xc3],
        ]
        .concat();
        jump_stub(&mut t, f, &code);
    }
    let m = t.alloc_random(0x40);
    for k in 0..64u32 {
        for (bit, slot) in [(1u32, 6u32), (2, 2), (4, 7)] {
            let v = if k & bit != 0 {
                t.rng.pick(&[1u32, 0x100, 0xffff_ffff])
            } else {
                0
            };
            t.put(held + 4 * slot, v);
        }
        let bytes = t.rng.bytes(0x40);
        t.put_bytes(m, &bytes);
        t.scramble(0x0186_a5b0, 4);
        t.scramble(0x0186_a5fc, 4);
        t.scramble(0x023c_c3bc, 16);
        t.twin("MenuSummon_Open", 0x008c_c1b0, &[m], U32, |o| {
            w(tk::menu_summon_open(
                o,
                m,
                local,
                &mut ui_remap,
                &mut callback,
            ))
        });
    }
}

/// A stand-in for `f` behind a jump, in both: `pre`, then it appends `id`
/// and its first `n` arguments at `*cursor`, then `post` (which ends with
/// its `ret`).
fn small7_log(t: &mut Twins, f: u32, cursor: u32, id: u32, n: u8, pre: &[u8], post: &[u8]) {
    let c = cursor.to_le_bytes();
    let mut code = pre.to_vec();
    code.extend([&[0x8b, 0x0d][..], &c].concat()); // mov ecx, [cursor]
    code.extend([0xc7, 0x01]); // mov dword [ecx], id
    code.extend(id.to_le_bytes());
    for j in 0..n {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * j, 0x89, 0x41, 4 + 4 * j]);
    }
    code.extend([&[0x83, 0x05][..], &c, &[4 + 4 * n]].concat()); // add dword [cursor], 4 + 4n
    code.extend(post);
    jump_stub(t, f, &code);
}

/// Small round 7 c: `TkMenuMainLoop_008a9ca0` over every mode family
/// (0x800001..3, -1, 0x10000.., others), opening or open, busy or idle on
/// each `MenuMain_BusyMask` call (a bit sequence), the Iggy byte, skipped
/// frames, the black background, the active mask, both ratios near their
/// clamps; every callee logged with its arguments and a result of its own,
/// some changing what the loop reads after them (the timer in `TkSyncV`,
/// the Iggy byte flipped in `AtelExecMenuCtrl`, the shade in
/// `TkMenuGetDrawShadeRatio`, the frame ratio in `sceGsSyncPath`, the open
/// flag set or kept in `graphicSetFlipVsnc`).
#[test]
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn small7_c_menu_main_loop_matches_the_original() {
    use ffx_menu::main_loop::{callee as c, global as g};
    let Some(mut t) = setup_any(0x5a11_7c01) else {
        return;
    };
    let log = t.alloc(&vec![0; 0x4000]);
    let cursor = t.alloc(&log.to_le_bytes());
    let (busy, reopen) = (t.alloc(&[0; 4]), t.alloc(&[0; 4]));
    let (timer, iggy, shade, ratio, open) = (
        t.sym(g::TIMER),
        t.sym(g::IGGY),
        t.sym(g::SHADE_RATIO),
        t.sym(g::FRAME_RATIO),
        t.sym(g::OPEN),
    );
    let rets: Vec<u32> = (0..8).map(|_| t.alloc(&[0; 4])).collect();
    let ret = |cell: u32| [&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat();
    let inc_byte = |at: u32| [&[0xfe, 0x05][..], &at.to_le_bytes()].concat();
    let inc_word = |at: u32| [&[0xff, 0x05][..], &at.to_le_bytes()].concat();
    let busy_post = [
        &[0xa1][..],
        &busy.to_le_bytes(),
        &[0x83, 0xe0, 0x01, 0xd1, 0x2d],
        &busy.to_le_bytes(),
        &[0xc3],
    ]
    .concat(); // mov eax, [busy]; and eax, 1; shr dword [busy], 1; ret
    // mov eax, [reopen]; cmp eax, -1; je +5; mov [open], eax
    let reopen_pre = [
        &[0xa1][..],
        &reopen.to_le_bytes(),
        &[0x83, 0xf8, 0xff, 0x74, 0x05, 0xa3],
        &open.to_le_bytes(),
    ]
    .concat();
    let flip_iggy = [&[0x80, 0x35][..], &iggy.to_le_bytes(), &[0x01]].concat(); // xor byte [iggy], 1
    let slot_pre = [0x8b, 0x44, 0x24, 0x04, 0xc7, 0, 0x34, 0x12, 0, 0]; // mov eax, [esp + 4]; mov dword [eax], 0x1234
    let callees: Vec<(u32, u8, Vec<u8>, Vec<u8>)> = vec![
        (c::SAVE_VRAM, 0, vec![], vec![]),
        (c::SAVE_FRAME_BUFFER, 0, vec![], vec![]),
        (c::SET_FRAME_RATIO_DIFF, 1, vec![], vec![]),
        (c::INIT_MENU_GRAPH, 0, vec![], vec![]),
        (c::SEQ_INIT_SCENE, 1, vec![], vec![]),
        (c::ENTER_MENU_CALLBACK, 1, vec![], vec![]),
        (c::MAIN_RESET, 2, vec![], vec![]),
        (c::OPEN_MODE, 2, vec![], vec![]),
        (c::INIT_BRUSH, 0, vec![], vec![]),
        (c::BUSY_MASK, 0, vec![], busy_post),
        (c::MOVIE_HAVE_CAMERA, 0, vec![], ret(rets[0])),
        (c::VIDEO_PAUSE, 0, vec![], vec![]),
        (c::SET_FLIP_VSYNC, 1, reopen_pre, ret(rets[1])),
        (c::MENU_RESET, 0, vec![], vec![]),
        (c::SYNC_V, 1, inc_byte(timer), ret(rets[2])),
        (c::EXEC_VIBRATE, 1, vec![], vec![]),
        (c::IGGY_MENU_UPDATE, 0, vec![], vec![]),
        (c::GET_DBUFF_DC, 0, slot_pre.to_vec(), vec![]),
        (c::GET_PAD_STATUS, 0, vec![], vec![]),
        (c::FILE_READ_MAIN, 0, vec![], vec![]),
        (c::EXEC_MENU_CTRL, 0, flip_iggy, vec![]),
        (c::CLOSE_ALL, 0, inc_byte(timer), vec![]),
        (c::CONTROL_MESSAGE_WINDOW, 0, vec![], vec![]),
        (c::SND_UPDATE, 0, vec![], vec![]),
        (c::TRANS_TEXTURE, 0, vec![], vec![]),
        (c::ABMAP_RESET_SEGMENTS, 0, vec![], vec![]),
        (c::GET_ABMAP, 0, vec![], ret(rets[3])),
        (c::DRAW_BLACK_BACKGROUND, 0, vec![], vec![]),
        (c::DRAW_ALL, 0, vec![], vec![]),
        (c::VU1_SYNC_PATH, 0, vec![], vec![]),
        (c::OPEN_PKT_BUFF_TMP, 0, vec![], vec![]),
        (c::DRAW_SCISSOR_XYWH, 4, vec![], vec![]),
        (c::GET_DRAW_SHADE_RATIO, 0, inc_word(shade), ret(rets[4])),
        (c::DRAW_BOX_XYWH_C2, 6, vec![], vec![]),
        (c::DRAW_KICK_TMP, 0, vec![], vec![]),
        (c::DRAW_MESSAGE_WINDOW, 0, vec![], vec![]),
        (c::DAPS_DRAW, 0, vec![], vec![]),
        (c::ABMAP_UPDATE, 0, vec![], vec![]),
        (c::DRAW_KICK, 0, vec![], vec![]),
        (c::GS_SYNC_PATH, 2, inc_word(ratio), vec![]),
        (c::FRAME_FADE, 4, vec![], vec![]),
        (c::MENU_UPDATE, 0, vec![], vec![]),
        (c::REMOVE_ALL_MENUS, 0, vec![], vec![]),
        (c::ATEL_RELEASE, 0, vec![], vec![]),
        (c::RECOVER_FROM_MENU, 0, vec![], vec![]),
        (c::RESTORE_VRAM, 0, vec![], vec![]),
        (c::DRAW_INIT_STRING, 0, vec![], vec![]),
        (c::DRAW_ENV, 0, vec![], vec![]),
    ];
    for (k, (f, n, pre, post)) in callees.into_iter().enumerate() {
        let id = 0x7c00 + u32::try_from(k).unwrap();
        // a result of its own (`mov eax, id; ret`) unless given
        let post = if post.is_empty() {
            [&[0xb8][..], &id.to_le_bytes(), &[0xc3]].concat()
        } else {
            post
        };
        small7_log(&mut t, f, cursor, id, n, &pre, &post);
    }
    let slot = t.b.stack_range().start + 0x80;
    for round in 0..400u32 {
        t.put(cursor, log);
        let mode = match t.rng.below(7) {
            0 => 0x0080_0001,
            1 => 0x0080_0002,
            2 => 0x0080_0003,
            3 => u32::MAX,
            4 => 0x0001_0000 | t.rng.below(0x20),
            5 => t.rng.pick(&[0x0002_0000, 0x0040_0000, 0]) | t.rng.below(0x20),
            _ => t.rng.next(),
        };
        let b = t.rng.below(8);
        t.put(busy, b);
        let r = t.rng.pick(&[u32::MAX, u32::MAX, 0, 1, 2]);
        t.put(reopen, r);
        for &cell in &rets {
            let v = t.rng.pick(&[0u32, 1, 2, 0x55]);
            t.put(cell, v);
        }
        let o = t.rng.below(2);
        t.put(open, o);
        for gl in [
            g::BLACK_BACK,
            g::MODE_800003,
            g::MOVIE_PICTURE,
            g::SKIP_FRAME,
        ] {
            let v = t.rng.pick(&[0u32, 0, 1, 2]);
            t.put(t.sym(gl), v);
        }
        let v = t.rng.pick(&[0u8, 0, 1]);
        t.put(iggy, v);
        let v = u8::try_from(t.rng.below(0x100)).unwrap();
        t.put(timer, v);
        t.scramble(g::FRAMES, 4);
        t.scramble(g::DRAW_SKIP_MASK, 4);
        t.scramble(g::CLEARED, 12);
        let v = u8::try_from(t.rng.below(4)).unwrap();
        t.put(t.sym(g::ACTIVE_MASK), v);
        for (at, step) in [
            (g::SHADE_RATIO, g::SHADE_RATIO + 4),
            (g::FRAME_RATIO, g::FRAME_RATIO_DIFF),
        ] {
            let v = t
                .rng
                .pick(&[0u32, 1, 0x7f, 0x80, 0x40, 0xffff_ffff, 0x8000_0000]);
            t.put(t.sym(at), v);
            let d = t
                .rng
                .pick(&[0u32, 1, 2, 0xffff_ffff, 0xffff_fff0, 0x7fff_ffff]);
            t.put(t.sym(step), d);
        }
        t.twin(
            &format!("TkMenuMainLoop {round}"),
            0x008a_9ca0,
            &[mode],
            U32,
            |o| {
                w(ffx_menu::main_loop::bridge::tk_menu_main_loop(
                    o,
                    mode,
                    slot,
                    &mut callback,
                ))
            },
        );
    }
}

/// Small round 7 d: `TkMn2ArmsRefreshArms` from every state (0..12) on
/// up to five actors (work records from an `AtelGetBasicWork` stand-in,
/// party members from `AtelGetBindPC`'s), the pick flags, both gear bits,
/// shown and changed gear, models with and without handles and ids, the
/// reads pending or done; every callee logged with its arguments.
#[test]
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn small7_d_arms_refresh_matches_the_original() {
    use ffx_menu::arms::{self, callee as c};
    let Some(mut t) = setup_any(0x5a11_7d01) else {
        return;
    };
    let log = t.alloc(&vec![0; 0x4000]);
    let cursor = t.alloc(&log.to_le_bytes());
    let works = t.alloc(&vec![0; 0xc00 * 8]);
    let state = t.alloc(&[0; 4]);
    let (total, gear, sync, wmodel, amodel) = (
        t.alloc(&[0; 4]),
        t.alloc(&[0; 4]),
        t.alloc(&[0; 4]),
        t.alloc(&[0; 4]),
        t.alloc(&[0; 4]),
    );
    let ret = |cell: u32| [&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat();
    #[rustfmt::skip]
    let work_post: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x07][..],             // mov eax, [esp + 4]; and eax, 7
        &[0x69, 0xc0, 0x00, 0x0c, 0, 0],                             // imul eax, eax, 0xc00
        &[0x05], &works.to_le_bytes(), &[0xc3],                     // add eax, works; ret
    ]
    .concat();
    let pc_post = [0x8b, 0x44, 0x24, 0x04, 0x8b, 0x80, 0xf0, 0x0b, 0, 0, 0xc3]; // mov eax, [esp + 4]; mov eax, [eax + 0xbf0]; ret
    let alloc_post = [0x8b, 0x44, 0x24, 0x04, 0x05, 0, 0x10, 0, 0, 0xc3]; // mov eax, [esp + 4]; add eax, 0x1000; ret
    let callees: Vec<(u32, u8, Vec<u8>)> = vec![
        (c::CHANGE_CURRENT_CTRL, 1, vec![]),
        (c::TOTAL_ACTOR_NUMBER2, 0, ret(total)),
        (c::GET_BASIC_WORK, 1, work_post),
        (c::GET_BIND_PC, 1, pc_post.to_vec()),
        (c::GET_WEAPON, 1, ret(gear)),
        (c::GET_ARMOR, 1, ret(gear)),
        (c::CH_DISPOSE, 1, vec![]),
        (c::DATA_DISPOSE, 2, vec![]),
        (c::WEAPON_MODEL, 1, ret(wmodel)),
        (c::ARMOR_MODEL, 1, ret(amodel)),
        (c::DATA_READ, 1, vec![]),
        (c::DATA_READ_SYNC, 1, ret(sync)),
        (c::CH_ALLOCATE, 1, alloc_post.to_vec()),
        (c::SOUND_SET_ID, 2, vec![]),
        (c::BIND_TO_ELEM, 5, vec![]),
        (c::EN_CLIP, 2, vec![]),
        (c::VU1_SET_CLIPMODE, 2, vec![]),
        (c::SET_CURRENT_CTRL, 1, vec![]),
    ];
    for (k, (f, n, post)) in callees.into_iter().enumerate() {
        let id = 0x7d00 + u32::try_from(k).unwrap();
        let post = if post.is_empty() {
            [&[0xb8][..], &id.to_le_bytes(), &[0xc3]].concat()
        } else {
            post
        };
        small7_log(&mut t, f, cursor, id, n, &[], &post);
    }
    let actor = t.sym(arms::ACTOR);
    let (shown_w, shown_a) = (t.sym(arms::SHOWN_WEAPON), t.sym(arms::SHOWN_ARMOR));
    for round in 0..800u32 {
        t.put(cursor, log);
        let s = t.rng.below(13);
        t.put(state, s);
        let n = t.rng.below(6);
        t.put(total, n);
        let i = t.rng.below(n + 2).wrapping_sub(1);
        t.put(actor, i);
        // a signed bound: a negative count ends at once
        if round % 16 == 5 {
            t.put(total, u32::MAX);
        }
        let g = t.rng.pick(&[3u32, 0x41, 0x1_0003]);
        t.put(gear, g);
        let v = t.rng.pick(&[0u32, 1, 1, 2]);
        t.put(sync, v);
        for cell in [wmodel, amodel] {
            let v = t.rng.pick(&[0u32, 0x123, 0x1_0000, 0xabcd_0042]);
            t.put(cell, v);
        }
        for k in 0..9 {
            let v = t.rng.pick(&[g, g, 7]);
            t.put(shown_w + 4 * k, v);
            let v = t.rng.pick(&[g, g, 9]);
            t.put(shown_a + 4 * k, v);
        }
        for a in 0..8 {
            let w = works + 0xc00 * a;
            let v = t.rng.pick(&[1u8, 1, 0, 2]);
            t.put(w + 0xaa, v);
            let v = t.rng.pick(&[0u32, 0x5555, 0x5555]);
            t.put(w + 0x9c, v);
            let v = t.rng.below(9);
            t.put(w + 0xbf0, v);
            let v = u8::try_from(t.rng.below(4)).unwrap();
            t.put(w + 0xb14, v);
            for o in [0xaf4, 0xaf8] {
                let v = t.rng.pick(&[0u32, 0x9999]);
                t.put(w + o, v);
            }
            for o in [0xb04, 0xb06] {
                let v = t.rng.pick(&[0u16, 0x31, 0xfff0]);
                t.put(w + o, v);
            }
            for o in [0xafc, 0xb00, 0xb08, 0xb0c] {
                let v = t.rng.next();
                t.put(w + o, v);
            }
            let v = u16::try_from(t.rng.below(0x1_0000)).unwrap();
            t.put(w + 0xa8, v);
        }
        t.twin(
            &format!("TkMn2ArmsRefreshArms {round}"),
            0x008c_fd70,
            &[state],
            U32,
            |o| {
                w(arms::bridge::tk_mn2_arms_refresh_arms(
                    o,
                    state,
                    &mut callback,
                ))
            },
        );
    }
}

/// Small round 8 a: `TkMenuCtrlAbilityMap2` from states 0, 1 and others
/// with the first read done or not, `TkMenuDrawAbilityMap2_008e2720` with
/// and without the grid frame, and `TkMenuExecScene` on blocks present or
/// missing, 0 and wrapping handles, records started or not, -1..4 tracks
/// whose steps return 0 or bits (a step that flips the record's flags
/// word, to show the final read); every callee logged with its arguments.
#[test]
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn small8_a_ability_map_and_exec_scene_match_the_original() {
    use ffx_menu::{ability_map as am, scene};
    let Some(mut t) = setup_any(0x5a11_8a01) else {
        return;
    };
    let log = t.alloc(&vec![0; 0x4000]);
    let cursor = t.alloc(&log.to_le_bytes());
    let (read_end, steps, flags_at, calc) = (
        t.alloc(&[0; 4]),
        t.alloc(&[0; 4]),
        t.alloc(&[0; 4]),
        t.alloc(&[0; 4]),
    );
    let ret = |cell: u32| [&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat();
    // mov eax, [steps]; and eax, 3; shr dword [steps], 2; ret
    let step_post = [
        &[0xa1][..],
        &steps.to_le_bytes(),
        &[0x83, 0xe0, 0x03, 0xc1, 0x2d],
        &steps.to_le_bytes(),
        &[0x02, 0xc3],
    ]
    .concat();
    // mov eax, [flags_at]; xor dword [eax], 0x40000001
    let step_pre = [
        &[0xa1][..],
        &flags_at.to_le_bytes(),
        &[0x81, 0x30, 0x01, 0, 0, 0x40],
    ]
    .concat();
    let callees: Vec<(u32, u8, Vec<u8>, Vec<u8>)> = vec![
        (am::callee::GET_CONTROL_PAD, 0, vec![], vec![]),
        (am::callee::GET_CONTROL_PAD_REP, 0, vec![], vec![]),
        (am::callee::GET_CONTROL_PAD_TRG, 0, vec![], vec![]),
        (
            am::callee::GET_FIRST_READ_END_FLAG,
            0,
            vec![],
            ret(read_end),
        ),
        (am::callee::ABM_START, 0, vec![], vec![]),
        (am::callee::ABM_CALC, 0, vec![], ret(calc)),
        (am::callee::OPEN_PKT_BUFF_TMP, 0, vec![], vec![]),
        (am::callee::DRAW_SCISSOR_XYWH, 4, vec![], vec![]),
        (am::callee::DRAW_BOX_XYWH_C2, 6, vec![], vec![]),
        (am::callee::DRAW_KICK_TMP, 0, vec![], vec![]),
        (am::callee::VU1_SYNC_PATH, 0, vec![], vec![]),
        (am::callee::DRAW_KICK, 0, vec![], vec![]),
        (am::callee::ABM_DRAW_FRAME, 0, vec![], vec![]),
        (am::callee::DRAW_ENV, 0, vec![], vec![]),
        (scene::bridge::TRACK_STEP, 5, step_pre, step_post),
    ];
    for (k, (f, n, pre, post)) in callees.into_iter().enumerate() {
        let id = 0x8a00 + u32::try_from(k).unwrap();
        let post = if post.is_empty() {
            [&[0xb8][..], &id.to_le_bytes(), &[0xc3]].concat()
        } else {
            post
        };
        small7_log(&mut t, f, cursor, id, n, &pre, &post);
    }
    let window = t.alloc(&[0; 0x40]);
    for round in 0..300u32 {
        t.put(cursor, log);
        let st = t.rng.pick(&[0u32, 0, 1, 1, 2, 7, 0x100, u32::MAX]);
        t.put(window + 0x1c, st);
        let v = t.rng.pick(&[0u32, 1, 0x100]);
        t.put(read_end, v);
        let v = t.rng.pick(&[0u32, 1, 2, 0x8a05]);
        t.put(calc, v);
        t.twin(
            &format!("TkMenuCtrlAbilityMap2 {round}"),
            0x008e_26d0,
            &[window],
            U32,
            |o| {
                w(am::bridge::tk_menu_ctrl_ability_map2(
                    o,
                    window,
                    &mut callback,
                ))
            },
        );
        t.put(cursor, log);
        t.twin(
            &format!("TkMenuDrawAbilityMap2 {round}"),
            0x008e_2720,
            &[window],
            U32,
            |o| {
                w(am::bridge::tk_menu_draw_ability_map2(
                    o,
                    window,
                    &mut callback,
                ))
            },
        );
    }
    let blocks: Vec<u32> = (0..3).map(|_| t.alloc(&[0; 0x400])).collect();
    let data = t.sym(0x022f_b4dc);
    // TkMenuExecScene keeps the block in its handle slot: the original's
    // slots are below Oracle::call's top of stack; the port's go lower
    // (the callbacks' own arguments reuse that top)
    let a_slot = t.a.stack_range().end - 0x100 - 12;
    let slot = t.b.stack_range().start + 0x80;
    for round in 0..600u32 {
        t.put(cursor, log);
        for k in 0..8 {
            let v = t.rng.pick(&[0, blocks[0], blocks[1], blocks[2]]);
            t.put(data + 4 * k, v);
        }
        let i = t.rng.below(8);
        let base = Mem::read::<u32>(&t.b, data + 4 * i);
        let handle = t.rng.pick(&[0u32, 0x100, 0x100, 0x100, 0x140, 1]);
        let handle = if round % 37 == 3 {
            base.wrapping_neg()
        } else {
            handle
        };
        for b in &blocks {
            // the record at handle 0 (the block's own +4)
            let v = t.rng.pick(&[0u32, 0x8000_0200]);
            t.put(b + 4, v);
            let tab = t.rng.pick(&[0x300u32, 0x340, 0]);
            t.put(b + 0x80, tab);
            for rec in [0x100u32, 0x140] {
                let started = t.rng.pick(&[0x8000_0000u32, 0x8000_0000, 0]);
                let ended = t.rng.pick(&[0x4000_0000u32, 0]);
                let list = t.rng.pick(&[0x200u32, 0x280]);
                let flags = if t.rng.below(8) == 0 {
                    0
                } else {
                    started | ended | list
                };
                t.put(b + rec + 4, flags);
                let v = t.rng.next();
                t.put(b + rec + 8, v);
            }
            for list in [0x200u32, 0x280] {
                let n = t.rng.pick(&[u32::MAX, 0, 1, 2, 4]);
                t.put(b + list, n);
                for k in 0..0x14 {
                    let v = t.rng.next();
                    t.put(b + list + 4 + 4 * k, v);
                }
            }
        }
        let v = t.rng.pick(&[0u32, 0, 0x1, 0x40, 0xffff]);
        t.put(steps, v);
        let f = base.wrapping_add(handle).wrapping_add(4);
        let f = if round % 3 == 0 { f } else { window + 0x30 };
        t.put(flags_at, f);
        t.scramble(scene::bridge::SCENE_OFFSET_X, 8);
        let out = t.rng.next();
        Mem::write(&mut t.b, slot + 4, handle);
        t.twin(
            &format!("TkMenuExecScene {round}"),
            0x00a7_b460,
            &[i, handle, out],
            U32,
            |o| {
                w(scene::bridge::tk_menu_exec_scene(
                    o,
                    i,
                    handle,
                    out,
                    slot,
                    &mut callback,
                ))
            },
        );
        assert_eq!(
            Mem::read::<u32>(&t.a, a_slot + 4),
            Mem::read::<u32>(&t.b, slot + 4),
            "TkMenuExecScene {round}: the handle slot"
        );
    }
}

/// [`small7_log`]'s stand-in as code in test space (for pointers the game
/// holds); its address.
fn small8_log_code(t: &mut Twins, cursor: u32, id: u32, n: u8, pre: &[u8], post: &[u8]) -> u32 {
    let c = cursor.to_le_bytes();
    let mut code = pre.to_vec();
    code.extend([&[0x8b, 0x0d][..], &c].concat());
    code.extend([0xc7, 0x01]);
    code.extend(id.to_le_bytes());
    for j in 0..n {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * j, 0x89, 0x41, 4 + 4 * j]);
    }
    code.extend([&[0x83, 0x05][..], &c, &[4 + 4 * n]].concat());
    code.extend(post);
    test_code(t, &code)
}

/// `mov eax, [cell]; ret`.
fn small8_ret(cell: u32) -> Vec<u8> {
    [&[0xa1][..], &cell.to_le_bytes(), &[0xc3]].concat()
}

/// Small round 8 b: `TkInit` with the debug link on or off, every
/// language and video mode family, HDD mode on or off (flipped by
/// `get_hdd_name`), an old stream table or none, sizes around the sector
/// rounding and the stream table's 0x8000 floor (negative ones too), and
/// `g_GsDefDrawEnv2` 0 (the disc info blocks); the CD interface a table of
/// logging stand-ins. `TkMenuStart` open or closed, staying open or
/// closing, under a pause, with mode bit 24 and the high bits, the task
/// buffer over its limit, with a task or none, both draw buffer
/// parities.
#[test]
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn small8_b_tk_init_and_menu_start_match_the_original() {
    use ffx_menu::start::{callee as c, global as g};
    let Some(mut t) = setup_any(0x5a11_8b01) else {
        return;
    };
    let log = t.alloc(&vec![0; 0x8000]);
    let cursor = t.alloc(&log.to_le_bytes());
    let cells: Vec<u32> = (0..12).map(|_| t.alloc(&[0; 4])).collect();
    let [
        debug,
        lang,
        video,
        size,
        heap,
        task,
        dc,
        open,
        size2,
        reopen,
        lang2,
        seen,
    ] = cells[..].try_into().unwrap();
    let hdd = t.sym(g::HDD_MODE);
    let menu_open = t.sym(g::MENU_OPEN);
    // sizes: the cell, then shifted, so each look-up differs
    // mov eax, [size]; rol dword [size], 5; ret
    let size_post = [
        &[0xa1][..],
        &size.to_le_bytes(),
        &[0xc1, 0x05],
        &size.to_le_bytes(),
        &[0x05, 0xc3],
    ]
    .concat();
    // mov eax, [heap]; add dword [heap], 0x40; ret
    let malloc_post = [
        &[0xa1][..],
        &heap.to_le_bytes(),
        &[0x83, 0x05],
        &heap.to_le_bytes(),
        &[0x40, 0xc3],
    ]
    .concat();
    let flip_hdd = [&[0x80, 0x35][..], &hdd.to_le_bytes(), &[0x01]].concat();
    // the language: first call [lang], later [lang2]
    // mov eax, [lang]; push dword [lang2]; pop dword [lang]; ret
    let lang_post = [
        &[0xa1][..],
        &lang.to_le_bytes(),
        &[0xff, 0x35],
        &lang2.to_le_bytes(),
        &[0x8f, 0x05],
        &lang.to_le_bytes(),
        &[0xc3],
    ]
    .concat();
    // mov eax, [reopen]; cmp eax, -1; je +5; mov [open], eax
    let reopen_pre = [
        &[0xa1][..],
        &reopen.to_le_bytes(),
        &[0x83, 0xf8, 0xff, 0x74, 0x05, 0xa3],
        &menu_open.to_le_bytes(),
    ]
    .concat();
    // mov eax, [esp + 4]; mov ecx, [dc]; mov [eax], ecx
    let dc_pre = [
        &[0x8b, 0x44, 0x24, 0x04, 0x8b, 0x0d][..],
        &dc.to_le_bytes(),
        &[0x89, 0x08],
    ]
    .concat();
    let config_hdd = t.sym(g::CONFIG + 0xe);
    let flip_config_hdd = [&[0x80, 0x35][..], &config_hdd.to_le_bytes(), &[0x04]].concat();
    // mov eax, [g_TkMenuMainTask]; mov [seen], eax
    let keep_task = [
        &[0xa1][..],
        &t.sym(g::MAIN_TASK).to_le_bytes(),
        &[0xa3],
        &seen.to_le_bytes(),
    ]
    .concat();
    // the CD interface
    let mut table = [0u32; 6];
    for (k, (o, n)) in [(0usize, 4u8), (1, 1), (2, 1), (4, 1), (5, 1)]
        .into_iter()
        .enumerate()
    {
        let id = 0x8b80 + u32::try_from(k).unwrap();
        let post = if o == 5 {
            size_post.clone()
        } else {
            [&[0xb8][..], &id.to_le_bytes(), &[0xc3]].concat()
        };
        table[o] = small8_log_code(&mut t, cursor, id, n, &[], &post);
    }
    let table_bytes: Vec<u8> = table.iter().flat_map(|v| v.to_le_bytes()).collect();
    let cd = t.alloc(&table_bytes);
    let cd_post = [&[0xb8][..], &cd.to_le_bytes(), &[0xc3]].concat();
    let task_end = small8_log_code(
        &mut t,
        cursor,
        0x8bf0,
        0,
        &[],
        &[0xb8, 0x77, 0x66, 0x55, 0x44, 0xc3],
    );
    let task_rec = t.alloc(&[&[0u8; 4][..], &task_end.to_le_bytes()].concat());
    let callees: Vec<(u32, u8, Vec<u8>, Vec<u8>)> = vec![
        (c::INIT_WORK, 0, vec![], vec![]),
        (c::IS_DEBUG, 0, vec![], small8_ret(debug)),
        (c::INIT_DECI2, 0, vec![], vec![]),
        (c::RESET_CLIP, 0, vec![], vec![]),
        (c::INIT_ALL_SYNC_V_CALLBACK, 0, vec![], vec![]),
        (c::CLEAR_ALARM, 0, vec![], vec![]),
        (c::PAD_INIT_WORKS, 0, vec![], vec![]),
        (c::INIT_SCENE_FROM_SAVE, 0, vec![], vec![]),
        // changes the HDD byte of the config TkInit copies back
        (c::SND_SET_MUSIC_MUTE, 1, flip_config_hdd, vec![]),
        (c::INIT_START_MAP, 0, vec![], vec![]),
        (c::READ_THREAD_INIT, 0, vec![], vec![]),
        (c::PRINTF_CUT, 0, vec![], vec![]),
        (c::GET_CD, 0, vec![], cd_post),
        (c::MEM_SET_ALIGN, 1, vec![], vec![]),
        (c::USER_MALLOC, 1, vec![], malloc_post),
        (c::USER_FREE, 1, vec![], vec![]),
        (c::FLUSH_CACHE, 1, vec![], vec![]),
        (c::TO_INIT, 0, vec![], vec![]),
        (c::ATEL_INIT_TOTAL, 0, vec![], vec![]),
        (c::GET_LANG, 0, vec![], lang_post),
        (c::SET_PROJECT_BASE, 1, vec![], vec![]),
        (c::GET_VIDEO_MODE, 0, vec![], small8_ret(video)),
        (c::SET_CYCLE_MODE, 1, vec![], vec![]),
        (c::SND_INIT_WORK, 0, vec![], vec![]),
        (c::INIT_WAVE_INFO, 3, vec![], vec![]),
        (c::INIT_MUSIC_INFO, 3, vec![], vec![]),
        (c::MEM_CHECK, 3, vec![], vec![]),
        (c::SORT_COMMON_WAVE, 1, vec![], vec![]),
        (c::STANDBY_SHOUT_STREAM, 1, vec![], vec![]),
        (c::DATA_SEND_SYNC, 1, vec![], vec![]),
        (c::REPORT_STANDBY_STATUS, 1, vec![], vec![]),
        (c::SHOUT_STREAMING_DATA_SEND, 2, vec![], vec![]),
        (c::GET_HDD_NAME, 1, flip_hdd, vec![]),
        (c::INIT_WAVE_INFO_HDD, 1, vec![], vec![]),
        (c::INIT_MUSIC_INFO_HDD, 1, vec![], vec![]),
        (c::ABM_LOAD, 0, vec![], vec![]),
        (c::MAIN_LOOP, 1, reopen_pre, vec![]),
        (c::SET_TK_ANTI_XYB, 3, vec![], vec![]),
        (c::ENABLE_TK_ANTI, 1, vec![], vec![]),
        (c::PAUSE_REQUEST, 1, vec![], vec![]),
        (c::MN2_INIT_WORK, 0, vec![], vec![]),
        (c::MN2_GET_MAIN_POS, 0, vec![], vec![]),
        (c::SHADE_RATIO_DIFF, 1, vec![], vec![]),
        (c::SET_SHADE_RATIO_STEP, 1, vec![], vec![]),
        // keeps the task pointer it sees
        (c::TASK_BUFF_SIZE_GET, 1, keep_task, small8_ret(size2)),
        (c::ASSERT, 3, vec![], vec![]),
        (c::TASK_INIT2, 3, vec![], small8_ret(task)),
        (c::PLAYER_LIST_INIT, 1, vec![], vec![]),
        (c::SUMMON_LIST_INIT, 1, vec![], vec![]),
        // not logged: its argument is a local of each side's own stack
        (c::GET_DBUFF_DC, 0, dc_pre, vec![]),
        (c::CALC_IMPORTANT_LIST_MAX, 0, vec![], vec![]),
        (c::SET_IGNORE_JOIN_FLAG, 1, vec![], vec![]),
    ];
    for (k, (f, n, pre, post)) in callees.into_iter().enumerate() {
        let id = 0x8b00 + u32::try_from(k).unwrap();
        let post = if post.is_empty() {
            [&[0xb8][..], &id.to_le_bytes(), &[0xc3]].concat()
        } else {
            post
        };
        small7_log(&mut t, f, cursor, id, n, &pre, &post);
    }
    let buffers = t.alloc(&vec![0; 0x2000]);
    let def_env = t.sym(g::GS_DEF_DRAW_ENV2);
    for round in 0..200u32 {
        t.put(cursor, log);
        let v = t.rng.pick(&[0u32, 0, 1]);
        t.put(debug, v);
        for cell in [lang, lang2] {
            let v = t.rng.pick(&[0u32, 1, 3, 6, 7, 9, u32::MAX]);
            t.put(cell, v);
        }
        let v = t.rng.pick(&[0u32, 1, 2, u32::MAX]);
        t.put(video, v);
        let v = t.rng.pick(&[
            0u32,
            1,
            0x7ff,
            0x800,
            0x801,
            0x7fff,
            0x8001,
            0xffff_f000,
            0x8000_0000,
            0x7fff_f900,
        ]);
        t.put(size, v);
        t.put(heap, buffers);
        let v = t.rng.bytes(0x2000);
        t.put_bytes(buffers, &v);
        let v = t.rng.pick(&[0u8, 0, 1, 2]);
        t.put(hdd, v);
        let v = t.rng.pick(&[1u8, 1, 0]);
        t.put(def_env, v);
        let v = t.rng.pick(&[0u32, 0x1234_5678]);
        t.put(t.sym(g::SND_STREAM_TABLE), v);
        t.scramble(g::CONFIG, 0x40);
        t.scramble(g::DECAY_TOP - 0x90, 0x94);
        t.twin(&format!("TkInit {round}"), 0x0088_d710, &[], U32, |o| {
            w(ffx_menu::start::bridge::tk_init(o, &mut callback))
        });
    }
    t.put(def_env, 1u8);
    let local = t.b.stack_range().start + 0x80;
    for round in 0..120u32 {
        t.put(cursor, log);
        let v = t.rng.pick(&[0u32, 0, 1]);
        t.put(open, v);
        t.put(menu_open, v);
        let v = t.rng.pick(&[u32::MAX, 0, 1, 5]);
        t.put(reopen, v);
        let v = t.rng.pick(&[0u32, 0, 0, 1]);
        t.put(t.sym(g::PAUSE_ON), v);
        let mode = match t.rng.below(4) {
            0 => t.rng.below(0x20) | 0x0080_0000,
            1 => t.rng.next(),
            2 => 0x0100_0000 | t.rng.below(0x20),
            _ => 0xc000_0000 | t.rng.below(0x1_0000),
        };
        let v = t.rng.pick(&[0u32, 0x7fff, 0x8000, 0xffff_ffff]);
        t.put(size2, v);
        let v = t.rng.pick(&[0u32, task_rec]);
        t.put(task, v);
        let v = t.rng.pick(&[0u32, task_rec]);
        t.put(t.sym(g::MAIN_TASK), v);
        let v = t.rng.next();
        t.put(dc, v);
        // the work area's ends
        let v = t.rng.next();
        t.put(t.sym(g::MENU_WORK), v);
        t.put(t.sym(g::MENU_WORK + g::MENU_WORK_SIZE - 4), v);
        t.scramble(g::TIMER, 2);
        t.scramble(g::FRAMES, 4);
        t.twin(
            &format!("TkMenuStart {round}"),
            0x008a_b030,
            &[mode],
            U32,
            |o| {
                w(ffx_menu::start::bridge::tk_menu_start(
                    o,
                    mode,
                    local,
                    &mut callback,
                ))
            },
        );
    }
}

/// Small round 8 c: `TkMenuCtrlItem` from every state (0..0x3a), over
/// windows with random cursors, outcomes (`+0x45` -1..3) and rows, the
/// item list, the use kind (0..5), items with and without a target
/// (`+0x22` bit 3), the sub-command rows (-1..4), missing windows for
/// state 53, an active-mask bit 23 or not, the gear sort's result; every
/// callee logged with its arguments (the frame's message pairs and the
/// save buffers read through their pointers), `TkMenuItemClearWindow`
/// moving `g_TkMenuItemWindow` (the original keeps the old one in `edx`).
#[test]
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn small8_c_ctrl_item_matches_the_original() {
    use ffx_menu::item::{callee as c, global as g};
    let Some(mut t) = setup_any(0x5a11_8c01) else {
        return;
    };
    let log = t.alloc(&vec![0; 0x4000]);
    let cursor = t.alloc(&log.to_le_bytes());
    let cells: Vec<u32> = (0..12).map(|_| t.alloc(&[0; 4])).collect();
    let [
        rom,
        types,
        nums,
        mask,
        sorted,
        list_win,
        name_win,
        moved,
        moved2,
        _,
        _,
        _,
    ] = cells[..].try_into().unwrap();
    let pairs = t.alloc(&[0; 0x20]);
    let slot = t.alloc(&[0; 0x40]);
    // each stand-in first logs the state word (to see the stores it
    // reads): mov eax, [slot + 0x1c]; mov ecx, [cursor]; mov [ecx], eax;
    // add dword [cursor], 4
    let state_log = [
        &[0xa1][..],
        &(slot + 0x1c).to_le_bytes(),
        &[0x8b, 0x0d],
        &cursor.to_le_bytes(),
        &[0x89, 0x01, 0x83, 0x05],
        &cursor.to_le_bytes(),
        &[0x04],
    ]
    .concat();
    let window_at = t.sym(g::WINDOW);
    // what TkMenuItemClearWindow does (or states 26/27 loop for ever),
    // then g_TkMenuItemWindow moved:
    // mov eax, [window]; test eax, eax; je +11; mov byte [eax + 0x45], 0;
    // mov dword [eax + 0x28], 0; mov eax, [moved]; mov [window], eax
    let move_window = [
        &[0xa1][..],
        &window_at.to_le_bytes(),
        &[0x85, 0xc0, 0x74, 0x0b, 0xc6, 0x40, 0x45, 0x00],
        &[0xc7, 0x40, 0x28, 0, 0, 0, 0],
        &[0xa1],
        &moved.to_le_bytes(),
        &[0xa3],
        &window_at.to_le_bytes(),
    ]
    .concat();
    // mov eax, [moved2]; mov [g_TkMenuItemWindow], eax
    let move_window2 = [
        &[0xa1][..],
        &moved2.to_le_bytes(),
        &[0xa3],
        &window_at.to_le_bytes(),
    ]
    .concat();
    // the message pairs and the counts behind the pointers, into `pairs`
    #[rustfmt::skip]
    let copy_pairs: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x08][..],                         // mov eax, [esp + 8]
        &[0x8b, 0x08, 0x89, 0x0d], &pairs.to_le_bytes(),       // mov ecx, [eax]; mov [pairs], ecx
        &[0x8b, 0x48, 0x04, 0x89, 0x0d], &(pairs + 4).to_le_bytes(),
        &[0x8b, 0x44, 0x24, 0x0c],
        &[0x8b, 0x08, 0x89, 0x0d], &(pairs + 8).to_le_bytes(),
        &[0x8b, 0x48, 0x04, 0x89, 0x0d], &(pairs + 12).to_le_bytes(),
        &[0x8b, 0x44, 0x24, 0x10, 0xa3], &(pairs + 16).to_le_bytes(),
        &[0x8b, 0x44, 0x24, 0x14, 0xa3], &(pairs + 20).to_le_bytes(),
    ]
    .concat();
    let plus = |k: u32| {
        [
            &[0x8b, 0x44, 0x24, 0x04, 0x05][..],
            &k.to_le_bytes(),
            &[0xc3],
        ]
        .concat()
    };
    let callees: Vec<(u32, u8, Vec<u8>, Vec<u8>)> = vec![
        (c::GET_CONTROL_PAD, 0, vec![], vec![]),
        (c::GET_CONTROL_PAD_REP, 0, vec![], vec![]),
        (c::GET_CONTROL_PAD_TRG, 0, vec![], vec![]),
        // its out word is a local of each side's stack: not logged
        (c::GET_ROM_ITEM, 1, vec![], small8_ret(rom)),
        (c::ITEM_TARGET_MASK, 1, move_window2.clone(), vec![]),
        (c::FREE_WINDOW_GROUP, 1, vec![], vec![]),
        (c::OPEN_TARGET_WINDOW, 3, vec![], vec![]),
        (c::GET_PLAYER_FROM_INDEX2, 1, vec![], plus(0x100)),
        (c::GET_SUMMON_FROM_INDEX, 1, vec![], plus(0x200)),
        (c::SND_SEP_PLAY_SIMPLE, 1, vec![], vec![]),
        (c::APPLY_HEAL, 1, vec![], vec![]),
        (c::USE_ON_TARGETS, 1, vec![], vec![]),
        // moves g_TkMenuItemWindow too
        (c::ITEM_REFRESH, 1, move_window2, vec![]),
        (c::CLEAR_WINDOW, 0, move_window, vec![]),
        (c::SAVE_ITEM_TYPE_BUFFER, 0, vec![], small8_ret(types)),
        (c::SAVE_ITEM_NUM_BUFFER, 0, vec![], small8_ret(nums)),
        (c::SORT_INVENTORY, 0, vec![], vec![]),
        (c::PACK_GEAR, 0, vec![], vec![]),
        (c::COUNT_GEAR_BY_OWNER, 0, vec![], vec![]),
        (c::SORT_GEAR_BY_OWNER, 0, vec![], small8_ret(sorted)),
        (c::SORT_GEAR_BY_KIND, 0, vec![], vec![]),
        (c::SORT_ALL_GEAR, 0, vec![], vec![]),
        (c::SWAP_SAVE_GEAR, 2, vec![], vec![]),
        (c::REFRESH_WEAPON_SHOP_SELL2, 0, vec![], vec![]),
        (c::MAIN_ACTIVE_MENU, 2, vec![], vec![]),
        (c::MAIN_GET_MENU_ACTIVE_MASK, 0, vec![], small8_ret(mask)),
        (c::CLOSE_WINDOW2, 0, vec![], vec![]),
        (c::SAVE_ITEM_USE, 2, vec![], vec![]),
        (c::CLEAR_WINDOW2, 0, vec![], vec![]),
        (c::OPEN_NAME_ENTRY_WINDOW, 2, vec![], small8_ret(name_win)),
        (c::ITEM_MES, 1, vec![], plus(0x1000)),
        (c::ITEM_HELP, 1, vec![], plus(0x2000)),
        (c::OPEN_SUB_CMD_WINDOW_BESIDE, 1, copy_pairs, vec![]),
        (c::SUB_CMD_SET_VALUE, 1, vec![], vec![]),
        (c::SUB_CMD_SET_VALUE2, 1, vec![], vec![]),
        (c::RESTART_SUB_CMD_WINDOW1, 0, vec![], vec![]),
        (c::RESTART_SUB_CMD_WINDOW2, 0, vec![], vec![]),
        (c::OPEN_LIST_WINDOW, 1, vec![], small8_ret(list_win)),
        (c::CLOSE_WINDOW3, 0, vec![], vec![]),
        (c::CLOSE_WINDOW4, 0, vec![], vec![]),
        (c::OPEN_GEAR_WINDOW, 0, vec![], vec![]),
        (c::CLOSE_WINDOW, 0, vec![], vec![]),
        (c::CLOSE_SUB_CMD_WINDOW1, 0, vec![], vec![]),
        (c::MAIN_SLEEP_MENU, 1, vec![], vec![]),
    ];
    for (k, (f, n, pre, post)) in callees.into_iter().enumerate() {
        let id = 0x8c00 + u32::try_from(k).unwrap();
        let post = if post.is_empty() {
            [&[0xb8][..], &id.to_le_bytes(), &[0xc3]].concat()
        } else {
            post
        };
        let pre = [&state_log[..], &pre].concat();
        small7_log(&mut t, f, cursor, id, n, &pre, &post);
    }
    let windows: Vec<u32> = (0..6).map(|_| t.alloc(&[0; 0x80])).collect();
    let (rom_rec, type_buf, num_buf) = (
        t.alloc(&[0; 0x40]),
        t.alloc(&[0; 0x100]),
        t.alloc(&[0; 0x100]),
    );
    t.put(rom, rom_rec);
    t.put(types, type_buf);
    t.put(nums, num_buf);
    let local = t.b.stack_range().start + 0x80;
    let globals = [
        g::WINDOW,
        g::WINDOW2,
        g::TARGET_WINDOW,
        g::NAME_WINDOW,
        g::GEAR_WINDOW,
        g::SUB_CMD_WINDOW,
        g::SUB_CMD_WINDOW2,
    ];
    for round in 0..3000u32 {
        t.put(cursor, log);
        let state = t.rng.below(0x3b);
        t.put(slot + 0x1c, state);
        for &w in &windows {
            let v = t.rng.pick(&[-1i8, -1, 0, 1, 2, 3]);
            t.put(w + 0x45, v);
            let v = u8::try_from(t.rng.below(4)).unwrap();
            t.put(w + 0x42, v);
            for o in [0x48, 0x2c, 0x2e] {
                let v = t.rng.pick(&[0i16, 1, 2, 3, 4, -1]);
                t.put(w + o, v);
            }
            for o in [0x58, 0x5c] {
                let v = t.rng.pick(&[0u32, 0, 1, 4, 8]);
                t.put(w + o, v);
            }
            let v = t.rng.pick(&[0u16, 1, 0x100]);
            t.put(w + 0x30, v);
            for o in [0x1c, 0x28] {
                let v = t.rng.next();
                t.put(w + o, v);
            }
            for o in [0x37, 0x41] {
                let v = t.rng.pick(&[0u8, 5]);
                t.put(w + o, v);
            }
        }
        for gl in globals {
            let v = t.rng.pick(&windows);
            t.put(t.sym(gl), v);
        }
        if state == 53 {
            for gl in [g::WINDOW, g::WINDOW2, g::GEAR_WINDOW] {
                if t.rng.below(2) == 0 {
                    t.put(t.sym(gl), 0u32);
                }
            }
        }
        for cell in [moved, moved2, list_win, name_win] {
            let v = t.rng.pick(&windows);
            t.put(cell, v);
        }
        let v = t.rng.pick(&[0u32, 0x0080_0000, 0x0180_0001, 1]);
        t.put(mask, v);
        let v = t.rng.pick(&[0u32, 1]);
        t.put(sorted, v);
        let v = t.rng.pick(&[0u8, 8, 0xf7, 0xff]);
        t.put(rom_rec + 0x22, v);
        let v = u8::try_from(t.rng.below(6)).unwrap();
        t.put(t.sym(g::USE_KIND), v);
        t.scramble(g::LIST, 0x80);
        for k in 0..0x20 {
            let v = t.rng.pick(&[0u8, 0, 1, 7]);
            t.put(t.sym(g::LIST + 2 + 4 * k), v);
        }
        // equal ids for the gear swap now and then
        if round % 5 == 0 {
            t.put(t.sym(g::LIST + 4), 0x55u16);
            t.put(t.sym(g::LIST), 0x55u16);
        }
        t.scramble(g::COMMAND, 2);
        t.scramble(g::GEAR_MODE, 1);
        t.scramble(g::TARGET_MODE, 4);
        let v = t.rng.bytes(0x100);
        t.put_bytes(type_buf, &v);
        let v = t.rng.bytes(0x100);
        t.put_bytes(num_buf, &v);
        // now and then the count buffer over the type buffer's first rows
        // (the order of the swap's reads and writes shows)
        let v = if round % 4 == 0 {
            type_buf + 6
        } else {
            num_buf
        };
        t.put(nums, v);
        t.twin(
            &format!("TkMenuCtrlItem {round} state {state}"),
            0x008c_5450,
            &[slot],
            U32,
            |o| {
                w(ffx_menu::item::bridge::tk_menu_ctrl_item(
                    o,
                    slot,
                    local,
                    &mut callback,
                ))
            },
        );
    }
    // the swap (state 27) with the count buffer over the type buffer
    for round in 0..400u32 {
        t.put(cursor, log);
        t.put(slot + 0x1c, 27u32);
        let win = t.rng.pick(&windows);
        t.put(t.sym(g::WINDOW), win);
        t.put(moved2, win);
        t.put(win + 0x45, 3u8);
        t.put(win + 0x42, 0u8);
        for o in [0x2c, 0x2e] {
            let v = i16::try_from(t.rng.below(4)).unwrap();
            t.put(win + o, v);
        }
        for o in [0x58, 0x5c] {
            let v = t.rng.below(6);
            t.put(win + o, v);
        }
        let v = t.rng.bytes(0x100);
        t.put_bytes(type_buf, &v);
        let v = type_buf + t.rng.below(12);
        t.put(nums, v);
        t.twin(
            &format!("TkMenuCtrlItem swap {round}"),
            0x008c_5450,
            &[slot],
            U32,
            |o| {
                w(ffx_menu::item::bridge::tk_menu_ctrl_item(
                    o,
                    slot,
                    local,
                    &mut callback,
                ))
            },
        );
    }
}
