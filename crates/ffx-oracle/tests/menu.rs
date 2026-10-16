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

impl Twins {
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
        let r = self.a.call(self.a.rebase(sym), Conv::Cdecl, args).unwrap();
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
    let Some(mut t) = setup(0x2545_f491) else {
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
        t.scramble(gu::MOUSE_Y.sym, 8);
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
        t.scramble(gu::MOUSE_Y.sym, 8);
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
    let Some(mut t) = setup(0x0bad_cafe) else {
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
/// (name, original, the bridge with window, id and value).
type ControlSetter = (&'static str, u32, fn(&mut Oracle, u32, u32, u32));

#[test]
fn tk_matches_the_original() {
    let Some(mut t) = setup(0x5eed_0001) else {
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
    let Some(mut t) = setup(0xf10a_7001) else {
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
    let Some(mut t) = setup(0x0dd_ba11) else {
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
