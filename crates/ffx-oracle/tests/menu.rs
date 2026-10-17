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
            want.to_bits() == got.to_bits() || (want.is_nan() && got.is_nan()),
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
            want.to_bits() == got.to_bits() || (want.is_nan() && got.is_nan()),
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
    assert!(
        want.to_bits() == got.to_bits() || (want.is_nan() && got.is_nan()),
        "{what}: {want} vs {got}"
    );
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
        t.twin("BtlEnd_GetResult19", 0x008b_da20, &[], U32, |o| {
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
        t.twin("BtlEnd_InitApStep", 0x008b_df10, &[], VOID, |o| {
            tk::btl_end_init_ap_step(o);
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
/// takes `13 + 7n` bytes: `f` must be at least that long.
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
