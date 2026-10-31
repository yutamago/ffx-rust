//! M5 stage 1, list `render-pcui` (round 6), batch 3: the pc-ui list's
//! leftovers: the crash dump's memory reader, the video cluster release
//! and the PC menus.

/// The originals over game memory.
#[allow(
    clippy::cast_possible_wrap, // the originals' signed arithmetic
    clippy::cast_sign_loss,
    clippy::many_single_char_names, // the originals' locals
    clippy::cast_possible_truncation // bytes and halves of words
)]
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The crash dump's `ReadProcessMemory` override (`0x00ccc848`) and
    /// its context word (`0x00ccc84c`).
    pub const READ_MEMORY_HOOK: u32 = 0x00cc_c848;
    pub const READ_MEMORY_CONTEXT: u32 = 0x00cc_c84c;
    /// `ReadProcessMemory` (import slot, `stdcall`, five words).
    pub const READ_PROCESS_MEMORY: u32 = 0x00b0_c0b0;

    /// `CrashDump_readMemory_00689370(process, base, unused, buffer, size,
    /// read)` (`stdcall`, `ret 0x18`; the count cell at `cell`, the
    /// original's `ebp - 4`) -> the call's `eax`: the stack walk's memory
    /// reader. With an override set it gets all six words and the context
    /// word; otherwise `ReadProcessMemory(process, base, buffer, size,
    /// &n)` (its result returned) and `*read = n`.
    pub fn crash_read_memory(h: &mut impl Host, cell: u32, args: [u32; 6]) -> u32 {
        let [process, base, unused, buffer, size, read] = args;
        let hook = word(h, h.rebase(READ_MEMORY_HOOK));
        if hook != 0 {
            let ctx = word(h, h.rebase(READ_MEMORY_CONTEXT));
            return h.call(
                hook,
                Conv::Cdecl,
                &[process, base, unused, buffer, size, read, ctx],
            );
        }
        let f = word(h, h.rebase(READ_PROCESS_MEMORY));
        let r = h.call(f, Conv::Stdcall, &[process, base, buffer, size, cell]);
        let n = word(h, cell);
        h.write(read, n);
        r
    }

    pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
    pub const RELEASE_CLUSTER: u32 = 0x0069_bdb0;
    pub const DELETE_MATERIAL: u32 = 0x0065_a2c0;
    pub const RELEASE_POOL_ARRAY: u32 = 0x0065_a3a0;
    pub const DELETE_POOL_OBJECT56: u32 = 0x0065_a430;
    pub const DELETE_MESH_INSTANCE: u32 = 0x0065_a4c0;
    /// `g_PhyreScene`.
    pub const SCENE: u32 = 0x00cc_b9d8;

    /// `VideoTexture_release_006e7d30(this)`: the three video planes'
    /// clusters (`+0x24`, `+0x40`, `+0x5c`, each with the word after it)
    /// released, then their materials (`+0x18`, `+0x34`, `+0x50`), pool
    /// arrays (`+0x1c`, ...), pool objects (`+0x2c`, ...) and mesh
    /// instances (`+0x30`, ...) deleted from the scene's cluster
    /// (`g_PhyreScene + 0x1284`) and cleared; `+0x14` cleared first. Returns
    /// the last slot's `eax`: its delete's result, or 0 for an empty slot.
    pub fn video_texture_release(h: &mut impl Host, this: u32) -> u32 {
        let mut last = 0;
        h.write(this + 0x14, 0_u8);
        let cluster = word(h, h.rebase(SCENE)).wrapping_add(0x1284);
        for off in [0x24, 0x40, 0x5c] {
            let c = word(h, this + off);
            if c != 0 {
                let cm = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[c]);
                method(h, RELEASE_CLUSTER, cm, &[c]);
                h.write(this + off, [0_u32; 2]);
            }
        }
        let parts: [(u32, &[u32]); 4] = [
            (DELETE_MATERIAL, &[0x18, 0x34, 0x50]),
            (RELEASE_POOL_ARRAY, &[0x1c, 0x38, 0x54]),
            (DELETE_POOL_OBJECT56, &[0x2c, 0x48, 0x64]),
            (DELETE_MESH_INSTANCE, &[0x30, 0x4c, 0x68]),
        ];
        for (f, offs) in parts {
            for &off in offs {
                let p = word(h, this + off);
                if p == 0 {
                    last = 0;
                    continue;
                }
                let scene = word(h, h.rebase(SCENE));
                last = if f == DELETE_MATERIAL {
                    method(h, f, scene, &[p, cluster, 0, 0])
                } else {
                    method(h, f, scene, &[p, cluster])
                };
                h.write(this + off, 0_u32);
            }
        }
        last
    }

    pub const SCOPE_LOCK_ENTER: u32 = 0x0046_3d50;
    pub const SCOPE_LOCK_LEAVE: u32 = 0x0046_3e90;
    /// The render interface singleton, its guard and its constructor and
    /// destructor (`atexit`).
    pub const RENDER_INTERFACE: u32 = 0x00c9_4f00;
    pub const RENDER_INTERFACE_GUARD: u32 = 0x00c9_628c;
    pub const RENDER_INTERFACE_CTOR: u32 = 0x004f_ee70;
    pub const RENDER_INTERFACE_DTOR: u32 = 0x00af_b710;
    pub const ATEXIT: u32 = 0x0094_9207;
    pub const DISPLAY_MODES: u32 = 0x0059_40d0;
    pub const NEW_ARRAY: u32 = 0x0063_04c0;
    pub const DELETE_ARRAY: u32 = 0x0063_0540;
    /// `g_Settings`.
    pub const SETTINGS: u32 = 0x022f_b504;

    /// `n * 0x1c` with the original's overflow to `0xffffffff`.
    fn modes_size(n: u32) -> u32 {
        n.saturating_mul(0x1c)
    }

    /// Copies a 0x1c-byte display mode from `src` to `dst`.
    fn copy_mode(h: &mut impl Mem, dst: u32, src: u32) {
        for k in 0..7 {
            let v = word(h, src + 4 * k);
            h.write(dst + 4 * k, v);
        }
    }

    /// `FFEscMenu_buildResolutionList(this)` (the render scope lock at
    /// `lock`, the original's `ebp - 0x24`): under the render lock, the
    /// display modes (`getDisplayModeList` on the render interface, made
    /// on first use) are sorted by width then height (unsigned) with
    /// duplicates of the same size dropped (the last entry moved into
    /// their place), copied into a new array (`+0xdc`, count `+0xd4`),
    /// and the entry matching the settings' resolution (`[g_Settings +
    /// 4]`: width in the high half, height in the low, signed) selected
    /// (`+0xd8`).
    pub fn build_resolution_list(h: &mut impl Host, lock: u32, this: u32) {
        method(h, SCOPE_LOCK_ENTER, lock, &[]);
        let guard = h.rebase(RENDER_INTERFACE_GUARD);
        let ri = h.rebase(RENDER_INTERFACE);
        let g = word(h, guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            method(h, RENDER_INTERFACE_CTOR, ri, &[]);
            let d = h.rebase(RENDER_INTERFACE_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[d]);
        }
        let count = this + 0xd4;
        method(h, DISPLAY_MODES, ri, &[count, 0]);
        let size = modes_size(word(h, count));
        let buf = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[size]);
        method(h, DISPLAY_MODES, ri, &[count, buf]);
        let mut n = word(h, count);
        let mut i = 1_u32;
        while (i.wrapping_sub(1).cast_signed()) < n.wrapping_sub(1).cast_signed() {
            let a = buf.wrapping_add(0x1c * (i - 1));
            let mut last = buf.wrapping_add(0x1c_u32.wrapping_mul(n.wrapping_sub(1)));
            let mut j = i;
            while j.cast_signed() < n.cast_signed() {
                let b = buf.wrapping_add(0x1c * j);
                let (aw, bw) = (word(h, a), word(h, b));
                let (ah, bh) = (word(h, a + 4), word(h, b + 4));
                if aw == bw && ah == bh {
                    copy_mode(h, b, last);
                    last = last.wrapping_sub(0x1c);
                    n = n.wrapping_sub(1);
                    continue;
                }
                if aw > bw || (aw == bw && ah > bh) {
                    let tmp: [u32; 7] = core::array::from_fn(|k| word(h, a + 4 * k as u32));
                    copy_mode(h, a, b);
                    h.write(b, tmp);
                }
                j += 1;
            }
            i += 1;
        }
        let arr = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[modes_size(n)]);
        h.write(this + 0xdc, arr);
        if n.cast_signed() > 0 {
            for k in 0..n {
                let dst = word(h, this + 0xdc).wrapping_add(0x1c * k);
                copy_mode(h, dst, buf.wrapping_add(0x1c * k));
            }
        }
        h.call_sym(DELETE_ARRAY, Conv::Cdecl, &[buf]);
        h.write(count, n);
        let cur = word(h, word(h, h.rebase(SETTINGS)) + 4);
        let (cw, ch) = (
            (cur.cast_signed() >> 16).cast_unsigned(),
            i32::from(cur as i16).cast_unsigned(),
        );
        for k in 0..n {
            let e = word(h, this + 0xdc).wrapping_add(0x1c * k);
            if word(h, e) == cw && word(h, e + 4) == ch {
                h.write(this + 0xd8, k);
            }
        }
        method(h, SCOPE_LOCK_LEAVE, lock, &[]);
    }

    pub const SET_SETTING: u32 = 0x0040_2a00;
    pub const GET_APPLICATION: u32 = 0x0062_3b90;
    pub const APPLY_SCREEN_MODE: u32 = 0x0062_bef0;
    pub const APPLY_SETTINGS: u32 = 0x0042_f9b0;
    pub const DETECT_QUALITY: u32 = 0x0040_2a20;

    /// The video menu's lookup tables: the shadow choice to its setting
    /// (`[2, 1, 0]`), the anti-aliasing choice (`[0, 3, 2, 1]`) and the
    /// MSAA samples (`[0, 2, 4, 8]`).
    pub const SHADOW_TABLE: [u32; 3] = [2, 1, 0];
    pub const AA_TABLE: [u32; 4] = [0, 3, 2, 1];
    pub const MSAA_TABLE: [u32; 4] = [0, 2, 4, 8];

    /// An entry of a choice table; past its end the original reads its
    /// own frame (the stack cookie, saved registers): 0 in the port
    /// (junk read, brief 2026-10-24; the menu only offers the listed
    /// choices).
    fn choice(table: &[u32], i: u32) -> u32 {
        table.get(i as usize).copied().unwrap_or(0)
    }

    /// `FFEscMenu_applyVideoSettings(this; s)` (`ret 4`) -> `delete[]`'s
    /// `eax`: the video page's twelve choices `s` go to `g_Settings`
    /// (`SetWordAt`): the screen mode (2), a new resolution (1: the mode
    /// list's `+0xdc` entry, width and height as two halves; the
    /// application's resize flag `+0x17`, `+0xd8` updated), the screen
    /// mode applied when it changed or the resolution changed in mode 1,
    /// then 9, 10 (flag `+0x18`), the shadows (13, flag `+0x15`), 11 (flag
    /// `+0x16`), the anti-aliasing (12), MSAA (14) and 15..18; the
    /// application applies them, the quality is detected again, the
    /// choices are kept (`+0xa4`) and `s` freed.
    pub fn apply_video_settings<H1: Host>(h: &mut H1, this: u32, s: u32) -> u32 {
        let settings = |h: &H1| word(h, h.rebase(SETTINGS));
        let set = |h: &mut H1, k: u32, v: u32| {
            let st = settings(h);
            method(h, SET_SETTING, st, &[k, v]);
        };
        let flag = |h: &mut H1, off: u32| {
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            h.write(app.wrapping_add(off), 1_u8);
        };
        let mode = word(h, s + 4);
        let mode_changed = mode != word(h, this + 0xa8);
        set(h, 2, mode);
        let res = word(h, s);
        let changed = res != word(h, this + 0xd8);
        if changed {
            let e = word(h, this + 0xdc).wrapping_add(res.wrapping_mul(0x1c));
            let v = u32::from(h.read::<u16>(e)) << 16 | u32::from(h.read::<u16>(e.wrapping_add(4)));
            set(h, 1, v);
            flag(h, 0x17);
            h.write(this + 0xd8, res);
        }
        if mode_changed || (changed && word(h, settings(h) + 8) == 1) {
            h.call_sym(APPLY_SCREEN_MODE, Conv::Cdecl, &[]);
        }
        set(h, 9, word(h, s + 8));
        set(h, 0xa, word(h, s + 0xc));
        flag(h, 0x18);
        set(h, 0xd, choice(&SHADOW_TABLE, word(h, s + 0x10)));
        flag(h, 0x15);
        set(h, 0xb, word(h, s + 0x14));
        flag(h, 0x16);
        set(h, 0xc, choice(&AA_TABLE, word(h, s + 0x18)));
        set(h, 0xe, choice(&MSAA_TABLE, word(h, s + 0x1c)));
        for (k, off) in [(0xf, 0x20), (0x10, 0x24), (0x11, 0x28), (0x12, 0x2c)] {
            set(h, k, word(h, s + off));
        }
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        method(h, APPLY_SETTINGS, app, &[]);
        let st = settings(h);
        method(h, DETECT_QUALITY, st, &[]);
        for k in 0..12 {
            let v = word(h, s + 4 * k);
            h.write(this + 0xa4 + 4 * k, v);
        }
        h.call_sym(DELETE_ARRAY, Conv::Cdecl, &[s])
    }

    pub const KEY_HELD: u32 = 0x0062_9d50;
    pub const LOC_STRING_LOAD: u32 = 0x0068_e180;
    pub const LOC_STRING_TO_VALUE: u32 = 0x0068_e270;
    pub const CALL_FLASH: u32 = 0x0068_dd40;
    pub const INPUT_SWAP_STATE: u32 = 0x0063_0df0;
    pub const CALL_AS3: u32 = 0x00a2_8ba0;
    pub const KEY_NAME: u32 = 0x0040_5af0;
    pub const IGGY_SET_UTF8: u32 = 0x00a2_8d70;
    pub const RESET_INPUT_STATE: u32 = 0x0070_1930;
    pub const KEY_SLOTS: u32 = 0x0040_5b90;
    pub const HAS_KEY_NAME: u32 = 0x0040_5a90;
    /// `"setHelpText"`, `"onCancelKey"`, `"setKeyBinding"`.
    pub const SET_HELP_TEXT: u32 = 0x00b4_6ca0;
    pub const ON_CANCEL_KEY: u32 = 0x00b4_6c7c;
    pub const SET_KEY_BINDING: u32 = 0x00b4_c810;
    /// The escape key and the help texts of the binding prompt.
    pub const KEY_ESCAPE: u32 = 0x40;
    pub const HELP_WAITING: u32 = 0x277;
    pub const HELP_DONE: u32 = 0x276;

    /// The help text `id` (a localised string through the loader at
    /// `frame + 0x10`) shown by the menu's `setHelpText`.
    fn help_text(h: &mut impl Host, frame: u32, this: u32, id: u32) {
        let obj = method(h, LOC_STRING_LOAD, frame + 0x10, &[id]);
        method(h, LOC_STRING_TO_VALUE, obj, &[frame]);
        flash_value(h, frame, this, SET_HELP_TEXT);
    }

    /// `IggyMenu_callFlashFunction_1(this, name, value)` with the 16-byte
    /// value at `frame`.
    fn flash_value(h: &mut impl Host, frame: u32, this: u32, name: u32) {
        let v: [u32; 4] = core::array::from_fn(|k| word(h, frame + 4 * k as u32));
        let name = h.rebase(name);
        h.call_sym(
            CALL_FLASH,
            Conv::Cdecl,
            &[this, name, v[0], v[1], v[2], v[3]],
        );
    }

    /// `FFEscMenu_keyBindingInput_00701190(this; done)` (`ret 4`; the frame
    /// from the original's `ebp - 0x124` at `frame`: the Iggy value `+0`,
    /// the string loader `+0x10`): the key binding prompt's update.
    /// `*done = 1`; the waiting help text is shown once (`+0xfc`). Escape
    /// held starts a cancel (`+0xf8` = 0x40), which fires `onCancelKey`
    /// on release; a held key (`+0xf8`) on release becomes the binding
    /// (`setKeyBinding` with its name, the done help text, the input state
    /// reset). Otherwise the first held key with a keyboard name is
    /// remembered.
    pub fn key_binding_input<H2: Host>(h: &mut H2, frame: u32, this: u32, done: u32) {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.write(done, 1_u32);
        let held = |h: &mut H2, k: u32| method(h, KEY_HELD, app, &[k, 0]) as u8 != 0;
        if h.read::<u8>(this + 0xfc) == 0 {
            help_text(h, frame, this, HELP_WAITING);
            h.write(this + 0xfc, 1_u8);
        }
        if held(h, KEY_ESCAPE) {
            h.write(this + 0xf8, KEY_ESCAPE);
        }
        let k = word(h, this + 0xf8);
        if k == KEY_ESCAPE {
            if held(h, KEY_ESCAPE) {
                return;
            }
            h.write(this + 0xfc, 0_u8);
            h.call_sym(INPUT_SWAP_STATE, Conv::Cdecl, &[frame + 4, 0, 0, 0]);
            let s = h.rebase(ON_CANCEL_KEY);
            method(h, CALL_AS3, this, &[0, s, 0, 0, 1]);
            h.write(this + 0xf8, u32::MAX);
            return;
        }
        if k != u32::MAX && !held(h, k) {
            let key = word(h, this + 0xf8);
            let name = h.call_sym(KEY_NAME, Conv::Cdecl, &[key]);
            h.call_sym(IGGY_SET_UTF8, Conv::Cdecl, &[frame, name]);
            flash_value(h, frame, this, SET_KEY_BINDING);
            help_text(h, frame, this, HELP_DONE);
            method(h, RESET_INPUT_STATE, this, &[]);
        }
        let n = h.call_sym(KEY_SLOTS, Conv::Cdecl, &[]).cast_signed();
        let mut i = 0_i32;
        while i < n {
            if h.call_sym(HAS_KEY_NAME, Conv::Cdecl, &[i.cast_unsigned()]) as u8 != 0
                && held(h, i.cast_unsigned())
            {
                h.write(this + 0xf8, i.cast_unsigned());
                return;
            }
            i += 1;
        }
    }

    pub const MENU_CONTEXT: u32 = 0x00a2_8d10;
    pub const VALUE_REF_CREATE: u32 = 0x00a2_88b0;
    pub const VALUE_GET_UTF8: u32 = 0x00a2_8b60;
    pub const VALUE_REF_FREE: u32 = 0x00a2_8a20;
    pub const RANGE_CHECK_FAILURE: u32 = 0x0094_9b02;
    /// `strchr` (import slot).
    pub const STRCHR: u32 = 0x00b0_c3c0;
    pub const DYNAMIC_CAST: u32 = 0x0094_9d86;
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    pub const ADD_CALL: u32 = 0x00a2_8df0;
    pub const INPUT_MANAGER: u32 = 0x0063_1300;
    pub const BUILD_PAD_MAP: u32 = 0x0063_15c0;
    /// The key binding records (`0x00c399ec`, 0x10 bytes each up to
    /// `0x00c39b5c`: the first and second key's setting slots at `+0`,
    /// `+4`), the RTTI descriptors of `IggyMenu` and `FFEscMenu`, the bound
    /// call and the special feature page update.
    pub const KEY_BINDINGS: u32 = 0x00c3_99f0;
    pub const KEY_BINDINGS_END: u32 = 0x00c3_9b60;
    pub const RTTI_IGGY_MENU: u32 = 0x00c3_3f60;
    pub const RTTI_ESC_MENU: u32 = 0x00c3_3f78;
    pub const CALL_BOUND_MEMBER: u32 = 0x006f_fc30;
    pub const UPDATE_SPECIAL_FEATURE_PAGE: u32 = 0x0070_2660;

    /// The first keyboard key slot (`0..n`) named `text`, or `n`.
    fn key_slot(h: &mut impl Host, n: i32, text: u32) -> u32 {
        let mut i = 0_i32;
        while i < n {
            let k = i.cast_unsigned();
            if h.call_sym(HAS_KEY_NAME, Conv::Cdecl, &[k]) as u8 != 0 {
                let name = h.call_sym(KEY_NAME, Conv::Cdecl, &[k]);
                if same_text(h, text, name) {
                    return k;
                }
            }
            i += 1;
        }
        if n > 0 { n.cast_unsigned() } else { 0 }
    }

    /// `strcmp(a, b) == 0` as the original inlines it.
    fn same_text(h: &impl Mem, a: u32, b: u32) -> bool {
        let mut k = 0_u32;
        loop {
            let (x, y) = (
                h.read::<u8>(a.wrapping_add(k)),
                h.read::<u8>(b.wrapping_add(k)),
            );
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// `FFEscMenu_loadKeyBindings_00701680(menu, args)` (`cdecl`, an Iggy
    /// callback; the frame from the original's `ebp - 0xa0` at `frame`:
    /// the value reference `+0`, the text buffer `+0x50`) -> the value
    /// reference's free `eax`: each record's text (`"<key>\n<key>"`, an
    /// array element of the call's value `[args + 4]`) names the keyboard
    /// slots stored in `g_Settings + 0x60` at the record's two setting
    /// indices (the second 0 when empty; an unknown name gives the slot
    /// count). Then the special feature page update is queued as a bound
    /// call on the menu (`FFEscMenu` by `__RTDynamicCast`) and the pad map
    /// rebuilt. A text of 64 bytes or more is a range check failure.
    pub fn load_key_bindings<H3: Host>(h: &mut H3, frame: u32, menu: u32, args: u32) -> u32 {
        let settings = word(h, h.rebase(SETTINGS));
        let (vref, buf) = (frame, frame + 0x50);
        let v = word(h, args + 4);
        let ctx = method(h, MENU_CONTEXT, menu, &[]);
        method(h, VALUE_REF_CREATE, vref, &[ctx, v]);
        let end = h.rebase(KEY_BINDINGS_END);
        let mut p = h.rebase(KEY_BINDINGS);
        let mut i = 0_u32;
        while p.cast_signed() < end.cast_signed() {
            let len = method(h, VALUE_GET_UTF8, vref, &[i, buf, 0x40]);
            if len >= 0x40 {
                h.call_sym(RANGE_CHECK_FAILURE, Conv::Cdecl, &[]);
                return 0;
            }
            h.write(buf + len, 0_u8);
            let f = word(h, h.rebase(STRCHR));
            let nl = h.call(f, Conv::Cdecl, &[buf, 0xa]);
            h.write(nl, 0_u8);
            let second = nl.wrapping_add(1);
            let n = h.call_sym(KEY_SLOTS, Conv::Cdecl, &[]).cast_signed();
            let first = key_slot(h, n, buf);
            let slot = |h: &H3, at: u32| {
                settings
                    .wrapping_add(0x60)
                    .wrapping_add(word(h, at).wrapping_mul(4))
            };
            let s1 = slot(h, p.wrapping_sub(4));
            h.write(s1, first);
            let k2 = if h.read::<u8>(second) == 0 {
                0
            } else {
                key_slot(h, n, second)
            };
            let s2 = slot(h, p);
            h.write(s2, k2);
            p = p.wrapping_add(0x10);
            i += 1;
        }
        let (r1, r2) = (h.rebase(RTTI_IGGY_MENU), h.rebase(RTTI_ESC_MENU));
        let obj = h.call_sym(DYNAMIC_CAST, Conv::Cdecl, &[menu, 0, r1, r2, 0]);
        let c = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x10]);
        h.write(c + 8, obj);
        let f = h.rebase(UPDATE_SPECIAL_FEATURE_PAGE);
        h.write(c, f);
        h.write(c + 4, 0_u32);
        let call = h.rebase(CALL_BOUND_MEMBER);
        method(h, ADD_CALL, obj, &[call, c]);
        let im = h.call_sym(INPUT_MANAGER, Conv::Cdecl, &[]);
        method(h, BUILD_PAD_MAP, im, &[]);
        method(h, VALUE_REF_FREE, vref, &[])
    }

    pub const KEY_DOWN: u32 = 0x0062_9ca0;
    pub const PAD_NAME: u32 = 0x0040_5b10;
    pub const PAD_SLOTS: u32 = 0x0040_5ba0;
    pub const HAS_PAD_NAME: u32 = 0x0040_5ac0;
    /// The controller binding records (`0x00c39ebc`, 0xc bytes each up to
    /// `0x00c39f64`: the setting slot first).
    pub const PAD_BINDINGS: u32 = 0x00c3_9ebc;
    pub const PAD_BINDINGS_END: u32 = 0x00c3_9f64;

    /// `FFEscMenu_padBindingInput_007019b0(this; done)` (`ret 4`; the frame
    /// as [`key_binding_input`]'s): the controller binding prompt. As the
    /// keyboard one, with the buttons pressed this frame (`Input_KeyDown`),
    /// the controller names, and the cancel resetting the input state
    /// before `onCancelKey`.
    pub fn pad_binding_input<H4: Host>(h: &mut H4, frame: u32, this: u32, done: u32) {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.write(done, 1_u32);
        if h.read::<u8>(this + 0xfc) == 0 {
            help_text(h, frame, this, HELP_WAITING);
            h.write(this + 0xfc, 1_u8);
        }
        let held = |h: &mut H4, k: u32| method(h, KEY_HELD, app, &[k, 0]) as u8 != 0;
        let down = |h: &mut H4, k: u32| method(h, KEY_DOWN, app, &[k, 0]) as u8 != 0;
        if held(h, KEY_ESCAPE) {
            h.write(this + 0xf8, KEY_ESCAPE);
        }
        let k = word(h, this + 0xf8);
        if k == KEY_ESCAPE {
            if held(h, KEY_ESCAPE) {
                return;
            }
            method(h, RESET_INPUT_STATE, this, &[]);
            let s = h.rebase(ON_CANCEL_KEY);
            method(h, CALL_AS3, this, &[0, s, 0, 0, 1]);
            h.write(this + 0xf8, u32::MAX);
            return;
        }
        if k != u32::MAX && !down(h, k) {
            let key = word(h, this + 0xf8);
            let name = h.call_sym(PAD_NAME, Conv::Cdecl, &[key]);
            h.call_sym(IGGY_SET_UTF8, Conv::Cdecl, &[frame, name]);
            flash_value(h, frame, this, SET_KEY_BINDING);
            help_text(h, frame, this, HELP_DONE);
            method(h, RESET_INPUT_STATE, this, &[]);
        }
        let n = h.call_sym(PAD_SLOTS, Conv::Cdecl, &[]).cast_signed();
        let mut i = 0_i32;
        while i < n {
            let b = i.cast_unsigned();
            if down(h, b) && h.call_sym(HAS_PAD_NAME, Conv::Cdecl, &[b]) as u8 != 0 {
                h.write(this + 0xf8, b);
                return;
            }
            i += 1;
        }
    }

    /// `FFEscMenu_loadPadBindings_00701e60(menu, args)` (`cdecl`, an Iggy
    /// callback; the frame from the original's `ebp - 0x94` at `frame`:
    /// the value reference `+0`, the text buffer `+0x44`) -> the value
    /// reference's free `eax`: each controller record's text names the
    /// controller slot stored in `g_Settings + 0x60` at the record's
    /// setting index (the slot count when unknown); then the pad map is
    /// rebuilt. A text of 64 bytes or more is a range check failure.
    pub fn load_pad_bindings(h: &mut impl Host, frame: u32, menu: u32, args: u32) -> u32 {
        let settings = word(h, h.rebase(SETTINGS));
        let (vref, buf) = (frame, frame + 0x44);
        let v = word(h, args + 4);
        let ctx = method(h, MENU_CONTEXT, menu, &[]);
        method(h, VALUE_REF_CREATE, vref, &[ctx, v]);
        let end = h.rebase(PAD_BINDINGS_END);
        let mut p = h.rebase(PAD_BINDINGS);
        let mut i = 0_u32;
        while p.cast_signed() < end.cast_signed() {
            let len = method(h, VALUE_GET_UTF8, vref, &[i, buf, 0x40]);
            if len >= 0x40 {
                h.call_sym(RANGE_CHECK_FAILURE, Conv::Cdecl, &[]);
                return 0;
            }
            h.write(buf + len, 0_u8);
            let n = h.call_sym(PAD_SLOTS, Conv::Cdecl, &[]).cast_signed();
            let mut k = 0_i32;
            while k < n {
                let s = k.cast_unsigned();
                if h.call_sym(HAS_PAD_NAME, Conv::Cdecl, &[s]) as u8 != 0 {
                    let name = h.call_sym(PAD_NAME, Conv::Cdecl, &[s]);
                    if same_text(h, buf, name) {
                        break;
                    }
                }
                k += 1;
            }
            let k = if n > 0 { k } else { 0 };
            let at = settings
                .wrapping_add(0x60)
                .wrapping_add(word(h, p).wrapping_mul(4));
            h.write(at, k.cast_unsigned());
            p = p.wrapping_add(0xc);
            i += 1;
        }
        let im = h.call_sym(INPUT_MANAGER, Conv::Cdecl, &[]);
        method(h, BUILD_PAD_MAP, im, &[]);
        method(h, VALUE_REF_FREE, vref, &[])
    }

    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const SCE_IOCTL: u32 = 0x0062_f8a0;
    pub const MOVIE_WAIT: u32 = 0x0077_daa0;
    pub const AUDIO_STOP: u32 = 0x0077_b170;
    pub const MOVIE_CSC: u32 = 0x0077_d5e0;
    pub const PAD_PRINT_FLAG: u32 = 0x0077_da90;
    /// `"Sync Error\n"`, `"Sync\n"`, `"Sync Wait !!!\n"`, `"Read End :  %d
    /// frame wait\n"`.
    pub const MSG_SYNC_ERROR: u32 = 0x00b6_8efc;
    pub const MSG_SYNC: u32 = 0x00b6_8f08;
    pub const MSG_SYNC_WAIT: u32 = 0x00b5_35b4;
    pub const MSG_READ_END: u32 = 0x00b5_3570;

    /// `syncStrPc(play, state)` (`cdecl`; the status cell at `cell`, the
    /// original's `ebp - 4`) -> the last status (0), or 0 without a
    /// stream: waits for the movie stream (`[state + 0x50544]`) to finish
    /// reading (`sceIoctl(fd, 1, &status)` until 0), a frame at a time;
    /// while playing (`play`) the audio is stopped once and each frame
    /// converted (`movie_csc`) and the pad print flag polled.
    pub fn sync_str_pc<H5: Host>(h: &mut H5, cell: u32, play: u32, state: u32) -> u32 {
        let fd = word(h, state.wrapping_add(0x5_0544));
        let msg = |h: &mut H5, m: u32| {
            let s = h.rebase(m);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[s]);
        };
        h.write(cell, u32::MAX);
        if fd.cast_signed() < 0 {
            msg(h, MSG_SYNC_ERROR);
            return 0;
        }
        msg(h, MSG_SYNC);
        h.write(cell, u32::MAX);
        h.call_sym(SCE_IOCTL, Conv::Cdecl, &[fd, 1, cell]);
        let (mut waited, mut stopped) = (false, false);
        let mut frames = 0_u32;
        while word(h, cell) != 0 {
            if !waited {
                waited = true;
                msg(h, MSG_SYNC_WAIT);
            }
            h.call_sym(MOVIE_WAIT, Conv::Cdecl, &[]);
            frames += 1;
            if play != 0 {
                if !stopped {
                    stopped = true;
                    h.call_sym(AUDIO_STOP, Conv::Cdecl, &[state]);
                }
                h.call_sym(MOVIE_CSC, Conv::Cdecl, &[]);
                h.call_sym(PAD_PRINT_FLAG, Conv::Cdecl, &[]);
            }
            h.write(cell, u32::MAX);
            h.call_sym(SCE_IOCTL, Conv::Cdecl, &[fd, 1, cell]);
        }
        let s = h.rebase(MSG_READ_END);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[s, frames]);
        word(h, cell)
    }

    pub const GU_DRAW_CHAR: u32 = 0x0084_6840;
    /// The debug print state (`g_GuPrintX` `0x013036e4` and its
    /// neighbours).
    pub const GU_BASE_X: u32 = 0x0130_36e0;
    pub const GU_X: u32 = 0x0130_36e4;
    pub const GU_Y: u32 = 0x0130_36ec;
    pub const GU_ADVANCE: u32 = 0x0130_36f0;
    pub const GU_NEXT_LINE: u32 = 0x0130_36f4;
    pub const GU_LINE: u32 = 0x0130_36f8;
    pub const GU_TEX_A: u32 = 0x0130_36fc;
    pub const GU_TEX_B: u32 = 0x0130_3700;
    pub const GU_ITALIC: u32 = 0x0130_3718;
    pub const GU_SLANT: u32 = 0x0130_371c;
    pub const GU_BOLD: u32 = 0x0130_3720;
    pub const GU_BOLD_OFFSET: u32 = 0x0130_3724;
    pub const GU_COUNT: u32 = 0x0130_36bc;

    /// `GuPrintChar_00849070(c)` (`cdecl`) -> the new character count, or
    /// the next line height for a newline: a newline returns to the base
    /// x and moves down a line; another character is drawn from the 8x8
    /// font sheet (column `(c - 32) & 15`, row `(c - 32) >> 4`; slanted
    /// when italic, twice offset when bold) and advances x. (The original
    /// keeps the slant in its argument slot; the shim's is not written.)
    pub fn gu_print_char<H5: Host>(h: &mut H5, c: u32) -> u32 {
        let g = |h: &H5, sym: u32| word(h, h.rebase(sym));
        let set = |h: &mut H5, sym: u32, v: u32| {
            let at = h.rebase(sym);
            h.write(at, v);
        };
        if c == 0xa {
            let base = g(h, GU_BASE_X);
            set(h, GU_X, base);
            let y = g(h, GU_Y).wrapping_add(g(h, GU_LINE));
            set(h, GU_Y, y);
            let next = g(h, GU_NEXT_LINE);
            set(h, GU_LINE, next);
            return next;
        }
        let ch = c.wrapping_sub(0x20);
        let col = ((ch & 0xf) << 7).cast_signed() >> 4;
        let row = ((ch & 0xf0) << 3).cast_signed() >> 4;
        let (col, row) = (col.cast_unsigned(), row.cast_unsigned());
        let slant = if g(h, GU_ITALIC) != 0 {
            g(h, GU_SLANT)
        } else {
            0
        };
        let x = g(h, GU_X);
        let args = |h: &H5, x: u32| {
            [
                x,
                g(h, GU_Y),
                g(h, GU_TEX_A),
                g(h, GU_TEX_B),
                col,
                row,
                8,
                8,
                slant,
            ]
        };
        let a = args(h, x);
        h.call_sym(GU_DRAW_CHAR, Conv::Cdecl, &a); // call width: 9
        let count = if g(h, GU_BOLD) != 0 {
            let off = g(h, GU_BOLD_OFFSET);
            let a = args(h, off.wrapping_add(x));
            h.call_sym(GU_DRAW_CHAR, Conv::Cdecl, &a); // call width: 9
            let nx = x.wrapping_add(g(h, GU_ADVANCE).wrapping_add(off));
            set(h, GU_X, nx);
            g(h, GU_COUNT).wrapping_add(2)
        } else {
            let nx = x.wrapping_add(g(h, GU_ADVANCE));
            set(h, GU_X, nx);
            g(h, GU_COUNT).wrapping_add(1)
        };
        set(h, GU_COUNT, count);
        count
    }

    /// The six words `code_0084ca00` clears (`0x023cbc40..0x023cbc48`,
    /// `0x023cbc50..0x023cbc58`).
    pub const CLOCK_STATE_A: u32 = 0x023c_bc40;
    pub const CLOCK_STATE_B: u32 = 0x023c_bc50;

    /// `SgClock_resetState_0084ca00()` -> 0: clears the two three-word
    /// clock states (`0x023cbc50..`, then `0x023cbc40..`).
    pub fn sg_clock_reset_state(h: &mut impl Host) -> u32 {
        let b = h.rebase(CLOCK_STATE_B);
        h.write(b, [0_u32; 3]);
        let a = h.rebase(CLOCK_STATE_A);
        h.write(a, [0_u32; 3]);
        0
    }

    /// The debug script reader: its cursor and end (`0x01325908`,
    /// `0x0132590c`), the token count (`0x01325910`) and the line buffer
    /// (`g_DebugLine`, `0x023c4000`).
    pub const DEBUG_CURSOR: u32 = 0x0132_5908;
    pub const DEBUG_END: u32 = 0x0132_590c;
    pub const DEBUG_TOKENS: u32 = 0x0132_5910;
    pub const DEBUG_LINE: u32 = 0x023c_4000;
    pub const PACKET_PTR: u32 = 0x0083_b650;
    pub const DEBUG_READ_LINE: u32 = 0x0085_30d0;
    /// `strtok` (import slot) and its separators `" \t\r"`.
    pub const STRTOK: u32 = 0x00b0_c488;
    pub const TOKEN_SEPARATORS: u32 = 0x00b5_b608;

    /// `DebugScript_readLine_008530d0()` -> the line's length, or 0 at the
    /// end: the next non-empty line of the debug script (up to `;` or a
    /// newline; carriage returns dropped) into `g_DebugLine`, the cursor
    /// moved past it; the token count cleared.
    pub fn debug_read_line(h: &mut impl Host) -> u32 {
        let cur_at = h.rebase(DEBUG_CURSOR);
        let end = word(h, h.rebase(DEBUG_END)).cast_signed();
        let tokens = h.rebase(DEBUG_TOKENS);
        h.write(tokens, 0_u32);
        let line = h.rebase(DEBUG_LINE);
        let mut pos = word(h, cur_at);
        loop {
            let mut d = line;
            if end > pos.cast_signed() {
                loop {
                    let c = h.read::<i8>(pos);
                    pos = pos.wrapping_add(1);
                    if c == 0x3b || c == 0xa {
                        break;
                    }
                    if c != 0xd {
                        h.write(d, c);
                        d += 1;
                    }
                    if end <= pos.cast_signed() {
                        break;
                    }
                }
                h.write(cur_at, pos);
            }
            h.write(d, 0_u8);
            let mut n = 0_u32;
            while h.read::<u8>(line + n) != 0 {
                n += 1;
            }
            if n != 0 {
                return n;
            }
            if end <= pos.cast_signed() {
                return 0;
            }
        }
    }

    /// `DebugScript_tokenizeLine_00853070()` -> the token count (0 without
    /// a line): the next debug script line split at blanks (`strtok`),
    /// each token's pointer into the packet (`Sg_PacketGetPtr`) and
    /// counted (`0x01325910`).
    pub fn debug_tokenize_line(h: &mut impl Host) -> u32 {
        let mut out = h.call_sym(PACKET_PTR, Conv::Cdecl, &[]);
        if h.call_sym(DEBUG_READ_LINE, Conv::Cdecl, &[]) == 0 {
            return 0;
        }
        let f = word(h, h.rebase(STRTOK));
        let (line, sep) = (h.rebase(DEBUG_LINE), h.rebase(TOKEN_SEPARATORS));
        let tokens = h.rebase(DEBUG_TOKENS);
        let mut tok = h.call(f, Conv::Cdecl, &[line, sep]);
        h.write(tokens, 0_u32);
        let mut n = 0_u32;
        while tok != 0 {
            n = word(h, tokens).wrapping_add(1);
            h.write(out, tok);
            out = out.wrapping_add(4);
            h.write(tokens, n);
            tok = h.call(f, Conv::Cdecl, &[0, sep]);
            n = word(h, tokens);
        }
        n
    }

    pub const GU_WINDOW_CREATE: u32 = 0x0084_a920;
    /// The debug GUI windows `Gui_MasakiInit` opens, by index: the open
    /// flag, `(x, y, w, h)` and the window procedure.
    pub const MASAKI_WINDOWS: [(u32, [u32; 4], u32); 10] = [
        (0x0113_9ee0, [0x25, 0x62, 0x168, 0x35], 0x007c_fcd0),
        (0x0113_9ef0, [0x50, 0, 0x12c, 0xea], 0x007d_aed0),
        (0x0113_9ee4, [0x25, 4, 0xba, 0x7b], 0x007d_9c20),
        (0x0113_9ef8, [0x40, 0x40, 0x100, 0x3f], 0x007c_6110),
        (0x0113_9ee8, [0xe0, 4, 0xbe, 0x85], 0x007d_44a0),
        (0x0113_9ef4, [0x40, 0x20, 0xf0, 0x49], 0x007d_80a0),
        (0x0113_9efc, [0xa, 0, 0x154, 0xd5], 0x007c_6e20),
        (0x0113_9f00, [0x50, 0, 0x190, 0xcb], 0x007d_d890),
        (0x0113_9f04, [0x50, 0, 0x190, 0xd5], 0x007d_e790),
        (0x0113_9f08, [0x80, 0x20, 0x100, 0x8f], 0x007d_fde0),
    ];

    /// `Gui_MasakiInit(which)` (`cdecl`) -> `Gu_WindowCreate`'s `eax`, or
    /// `which` when nothing opens: the debug window `which` (0..9: info,
    /// battle info, reference position, battle console, camera, battle,
    /// battle config, two character pages, shake) is opened once (style
    /// 7).
    pub fn gui_masaki_init(h: &mut impl Host, which: u32) -> u32 {
        let Some(&(flag, [x, y, w, ht], proc_)) = MASAKI_WINDOWS.get(which as usize) else {
            return which;
        };
        let flag = h.rebase(flag);
        if word(h, flag) != 0 {
            return which;
        }
        h.write(flag, 1_u32);
        let p = h.rebase(proc_);
        h.call_sym(GU_WINDOW_CREATE, Conv::Cdecl, &[x, y, w, ht, 7, p])
    }

    pub const PACKET_ARG: u32 = 0x0085_3140;
    pub const PACKET_ARG_INT: u32 = 0x0085_3170;
    pub const DEBUG_TOKENIZE: u32 = 0x0085_3070;
    pub const DEBUG_LIGHTS: u32 = 0x0084_d790;
    /// `sprintf` (import slot).
    pub const SPRINTF: u32 = 0x00b0_c3dc;
    pub const DEBUG_INTERPRET: u32 = 0x0085_3210;
    /// `"LGT"`, `"VERSION"`, `"#LIGHT"`, `""`.
    pub const KW_LGT: u32 = 0x00b5_ae6c;
    pub const KW_VERSION: u32 = 0x00b5_b60c;
    pub const KW_LIGHT: u32 = 0x00b5_b614;
    pub const KW_EMPTY: u32 = 0x00b8_fbeb;
    /// `atoi` (import slot).
    pub const ATOI: u32 = 0x00b0_c40c;
    /// The debug lights (`0x01303780`: 7 x 6 records of 0x1c bytes).
    pub const DEBUG_LIGHT_TABLE: u32 = 0x0130_3780;
    pub const USER_NAME: u32 = 0x0082_0610;
    pub const PC_READ: u32 = 0x0083_ae50;
    pub const PC_READ_LEN: u32 = 0x0083_ae70;
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `"/ffx/proj/proj/setting/%s.txt"`, `"p=%x limit=%x\n"`.
    pub const LIGHT_SCRIPT_PATH: u32 = 0x00b5_b5e8;
    pub const LIGHT_SCRIPT_MSG: u32 = 0x00b5_b63c;

    /// `DebugScript_readLights_0084d790(n)` (`cdecl`): the next `n` script
    /// lines; each `LGT group light kind ...` line sets the debug light
    /// `group * 6 + light` (`group` below 7, `light` below 6, signed): a
    /// directional or point light (kinds 0..2) its range (arguments 7, 8)
    /// then, with spot lights (3, 4), its kind byte (3) and colour (4..6).
    pub fn debug_read_lights<H6: Host>(h: &mut H6, n: u32) {
        let arg = |h: &mut H6, i: u32| h.call_sym(PACKET_ARG_INT, Conv::Cdecl, &[i]);
        let table = h.rebase(DEBUG_LIGHT_TABLE);
        let mut left = n;
        while left != 0 {
            left -= 1;
            h.call_sym(DEBUG_TOKENIZE, Conv::Cdecl, &[]);
            let first = h.call_sym(PACKET_ARG, Conv::Cdecl, &[0]);
            let kw = h.rebase(KW_LGT);
            if !same_text(h, kw, first) {
                continue;
            }
            let group = arg(h, 1).cast_signed();
            let light = arg(h, 2);
            if group >= 7 || light.cast_signed() >= 6 || light > 4 {
                continue;
            }
            let e = table.wrapping_add(
                group
                    .wrapping_mul(6)
                    .wrapping_add(light.cast_signed())
                    .wrapping_mul(0x1c)
                    .cast_unsigned(),
            );
            if light <= 2 {
                let v = arg(h, 7);
                h.write(e.wrapping_add(0x10), v);
                let v = arg(h, 8);
                h.write(e.wrapping_add(0x14), v);
            }
            let v = arg(h, 3);
            h.write(e.wrapping_add(0x1a), v as u8);
            for (k, off) in [(4, 0), (5, 4), (6, 8)] {
                let v = arg(h, k);
                h.write(e.wrapping_add(off), v);
            }
        }
    }

    /// `DebugScript_interpret_00853210()` -> 0: every line of the loaded
    /// debug script: a `VERSION` line is skipped, a `#LIGHT n` line reads
    /// `n` light lines, another `#...` line with a number skips that many
    /// lines.
    pub fn debug_interpret(h: &mut impl Host) -> u32 {
        if h.call_sym(DEBUG_TOKENIZE, Conv::Cdecl, &[]) == 0 {
            return 0;
        }
        let atoi = word(h, h.rebase(ATOI));
        let tokens = h.rebase(DEBUG_TOKENS);
        loop {
            let p = h.call_sym(PACKET_PTR, Conv::Cdecl, &[]);
            let a = if word(h, tokens).cast_signed() > 0 {
                word(h, p)
            } else {
                h.rebase(KW_EMPTY)
            };
            let p = h.call_sym(PACKET_PTR, Conv::Cdecl, &[]);
            let b = if word(h, tokens).cast_signed() > 1 {
                word(h, p + 4)
            } else {
                h.rebase(KW_EMPTY)
            };
            let n = h.call(atoi, Conv::Cdecl, &[b]);
            let version = h.rebase(KW_VERSION);
            if !same_text(h, a, version) {
                let light = h.rebase(KW_LIGHT);
                if same_text(h, a, light) {
                    h.call_sym(DEBUG_LIGHTS, Conv::Cdecl, &[n]);
                } else if h.read::<u8>(a) == b'#' && n != 0 {
                    let mut k = n;
                    while k != 0 {
                        h.call_sym(DEBUG_TOKENIZE, Conv::Cdecl, &[]);
                        k -= 1;
                    }
                }
            }
            if h.call_sym(DEBUG_TOKENIZE, Conv::Cdecl, &[]) == 0 {
                return 0;
            }
        }
    }

    /// `DebugScript_loadLights_008531a0()` -> the free's `eax`, or 0
    /// without the file: `/ffx/proj/proj/setting/<user>.txt` read
    /// (`Sg_PcReadD`) as the debug script (cursor, end), interpreted and
    /// freed.
    pub fn debug_load_lights(h: &mut impl Host) -> u32 {
        let tokens = h.rebase(DEBUG_TOKENS);
        h.write(tokens, 0_u32);
        let user = h.call_sym(USER_NAME, Conv::Cdecl, &[]);
        let line = h.rebase(DEBUG_LINE);
        let f = h.rebase(LIGHT_SCRIPT_PATH);
        let sprintf = word(h, h.rebase(SPRINTF));
        h.call(sprintf, Conv::Cdecl, &[line, f, user]);
        let buf = h.call_sym(PC_READ, Conv::Cdecl, &[line]);
        let cur = h.rebase(DEBUG_CURSOR);
        h.write(cur, buf);
        if buf == 0 {
            return 0;
        }
        let len = h.call_sym(PC_READ_LEN, Conv::Cdecl, &[]);
        let start = word(h, cur);
        let end = start.wrapping_sub(1).wrapping_add(len);
        let e = h.rebase(DEBUG_END);
        h.write(e, end);
        let m = h.rebase(LIGHT_SCRIPT_MSG);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[m, start, end]);
        h.call_sym(DEBUG_INTERPRET, Conv::Cdecl, &[]);
        h.call_sym(USER_FREE, Conv::Cdecl, &[buf])
    }

    pub const GU_WINDOW_SET_LOCAL_TRANS: u32 = 0x0084_b620;
    pub const GU_WINDOW_SET_PARENT: u32 = 0x0084_b660;
    pub const SG_SET_RESOURCE: u32 = 0x0083_fe10;
    /// The progress average window: its open flag, the inner window and
    /// a byte cleared on open; the two window procedures.
    pub const PROG_OPEN: u32 = 0x0132_59fc;
    pub const PROG_INNER: u32 = 0x0132_5948;
    pub const PROG_BYTE: u32 = 0x0132_5958;
    pub const PROG_PROC: u32 = 0x0085_5570;
    pub const PROG_INNER_PROC: u32 = 0x0085_54b0;

    /// `Gui_ProgAvarage()`: opens the progress average debug window once:
    /// an outer window (8, 0xc1, 0x1f0 x 0xf) moved to (1, 0x10) and an
    /// inner one parented to it; resource 2 cleared. Returns
    /// `Sg_SetResource`'s `eax` when it opens, `None` otherwise (the
    /// caller's `eax` stays).
    pub fn gui_prog_average(h: &mut impl Host) -> Option<u32> {
        let open = h.rebase(PROG_OPEN);
        if word(h, open) != 0 {
            return None;
        }
        let p = h.rebase(PROG_PROC);
        let w = h.call_sym(GU_WINDOW_CREATE, Conv::Cdecl, &[8, 0xc1, 0x1f0, 0xf, 0, p]);
        h.call_sym(GU_WINDOW_SET_LOCAL_TRANS, Conv::Cdecl, &[w, 1, 0x10]);
        h.write(open, 1_u32);
        let b = h.rebase(PROG_BYTE);
        h.write(b, 0_u8);
        let p = h.rebase(PROG_INNER_PROC);
        let inner = h.call_sym(GU_WINDOW_CREATE, Conv::Cdecl, &[0, 0x4d2, 0, 10, 0, p]);
        let at = h.rebase(PROG_INNER);
        h.write(at, inner);
        h.call_sym(GU_WINDOW_SET_PARENT, Conv::Cdecl, &[inner, w]);
        Some(h.call_sym(SG_SET_RESOURCE, Conv::Cdecl, &[2, 0]))
    }

    pub const GU_CONTROL_SET_DISABLE: u32 = 0x0084_a620;
    pub const GU_CONTROL_SET_TEXT: u32 = 0x0084_a710;
    /// The character view's two button labels.
    pub const CHRVIEW_TEXT_ON: u32 = 0x00b5_abd4;
    pub const CHRVIEW_TEXT_OFF: u32 = 0x00b5_abdc;

    /// `ChrView_updateControls_0084cc30(window, chr)` (`cdecl`) -> the text
    /// call's `eax`: the character view's controls 7, 5, 4, 10 and
    /// 0x12..0x15 enabled unless the character's flag bit 6 (`+0x194`) is
    /// set, and control 8 labelled to match.
    pub fn chrview_update_controls(h: &mut impl Host, window: u32, chr: u32) -> u32 {
        let en = 1 - ((word(h, chr + 0x194) >> 6) & 1);
        for id in [7, 5, 4, 10, 0x12, 0x13, 0x14, 0x15] {
            h.call_sym(GU_CONTROL_SET_DISABLE, Conv::Cdecl, &[window, id, en]);
        }
        let t = h.rebase(if en != 0 {
            CHRVIEW_TEXT_ON
        } else {
            CHRVIEW_TEXT_OFF
        });
        h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 8, t])
    }

    /// The light page's mode byte and its two names, and the labels it
    /// switches between.
    pub const LIGHT_MODE: u32 = 0x0130_3774;
    pub const LIGHT_NAME_ON: u32 = 0x0130_3770;
    pub const LIGHT_NAME_OFF: u32 = 0x0130_376c;
    pub const LIGHT_LABELS: [(u32, u32); 2] =
        [(0x00b5_add8, 0x00b5_ade0), (0x00b5_ade8, 0x00b5_adf0)];

    /// `ChrMenu_updateLightTexts_0084d540(window)` (`cdecl`) -> the last
    /// text call's `eax`: controls 0x54, 4 and 3 show the light mode's
    /// label, name and second label (`0x01303774` set or not).
    pub fn chrmenu_update_light_texts<H7: Host>(h: &mut H7, window: u32) -> u32 {
        let on = |h: &H7| h.read::<u8>(h.rebase(LIGHT_MODE)) != 0;
        let pick = |h: &H7, (a, b): (u32, u32)| h.rebase(if on(h) { a } else { b });
        let t = pick(h, LIGHT_LABELS[0]);
        h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 0x54, t]);
        let name = if on(h) { LIGHT_NAME_ON } else { LIGHT_NAME_OFF };
        let name = word(h, h.rebase(name));
        h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 4, name]);
        let t = pick(h, LIGHT_LABELS[1]);
        h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 3, t])
    }

    pub const CH_SYS_NB_USE: u32 = 0x0082_85e0;
    pub const GU_SELECTOR_RESET: u32 = 0x0084_9f40;
    pub const GU_SELECTOR_SET_ITEM: u32 = 0x0084_9fd0;
    /// `g_ChrCount`, `g_ChrArray` (0x880-byte characters).
    pub const CHR_COUNT: u32 = 0x023c_44e0;
    pub const CHR_ARRAY: u32 = 0x023c_44e4;

    /// `ChrMenu_fillSelector_0084ce10(selector)` (`cdecl`) -> 0, or -1
    /// without characters in use: the selector `[selector]` is reset to
    /// the count in use and gets each used character (`+2` set; its name
    /// `+4`) in order.
    pub fn chrmenu_fill_selector<H8: Host>(h: &mut H8, selector: u32) -> u32 {
        let n = h.call_sym(CH_SYS_NB_USE, Conv::Cdecl, &[]);
        let sel = word(h, selector);
        h.call_sym(GU_SELECTOR_RESET, Conv::Cdecl, &[sel, n]);
        if n == 0 {
            return u32::MAX;
        }
        let end = |h: &H8| {
            let (n, a) = (word(h, h.rebase(CHR_COUNT)), word(h, h.rebase(CHR_ARRAY)));
            n.wrapping_mul(0x880).wrapping_add(a)
        };
        let mut chr = word(h, h.rebase(CHR_ARRAY));
        let mut i = 0_u32;
        while chr != end(h) {
            if h.read::<u8>(chr.wrapping_add(2)) != 0 {
                let (sel, name) = (word(h, selector), word(h, chr.wrapping_add(4)));
                h.call_sym(GU_SELECTOR_SET_ITEM, Conv::Cdecl, &[sel, i, name, chr]);
                i += 1;
            }
            chr = chr.wrapping_add(0x880);
        }
        0
    }

    pub const SG_SET_PLAYER: u32 = 0x0082_dad0;
    pub const GU_WINDOW_GET_ARG: u32 = 0x0084_aec0;
    pub const SET_PTR32: u32 = 0x00a4_4700;
    pub const GET_PTR32: u32 = 0x00a4_4650;
    pub const SG_DB_NAME: u32 = 0x0082_9c90;
    pub const GU_WINDOW_SET_TITLE: u32 = 0x0084_b6a0;
    pub const CHRVIEW_UPDATE_CONTROLS: u32 = 0x0084_cc30;
    pub const GU_BUTTON_SET_TOGGLE: u32 = 0x0084_9380;
    pub const CH_GET_CLIP: u32 = 0x0082_ab40;
    pub const GU_BUTTON_SET_TOGGLE_VALUE: u32 = 0x0084_93e0;
    pub const CHRVIEW_PROC: u32 = 0x0084_c1e0;

    /// Copies the C string at `src` to `dst` with its NUL.
    fn copy_cstr(h: &mut impl Mem, dst: u32, src: u32) {
        let mut k = 0_u32;
        loop {
            let c = h.read::<u8>(src.wrapping_add(k));
            h.write(dst.wrapping_add(k), c);
            if c == 0 {
                return;
            }
            k += 1;
        }
    }

    /// `create_chrview(chr)` (`cdecl`) -> the clip toggle's `eax`: the
    /// character view debug window for `chr` (the player set to it): its
    /// argument block gets the character's handle (`+4`), its database
    /// name (`+0x50`, by `id >> 12`) and its name (`+0x70`, from `chr +
    /// 4`); the title, the controls, and the clip toggle (control 0xc)
    /// set from `Ch_GetClip`.
    pub fn create_chrview(h: &mut impl Host, chr: u32) -> u32 {
        h.call_sym(SG_SET_PLAYER, Conv::Cdecl, &[chr]);
        let p = h.rebase(CHRVIEW_PROC);
        let w = h.call_sym(GU_WINDOW_CREATE, Conv::Cdecl, &[0, 0, 0x5a, 0x78, 6, p]);
        let arg = h.call_sym(GU_WINDOW_GET_ARG, Conv::Cdecl, &[w]);
        let handle = h.call_sym(SET_PTR32, Conv::Cdecl, &[chr]);
        h.write(arg + 4, handle);
        let id = u32::from(h.read::<u16>(chr)) >> 12;
        let db = h.call_sym(SG_DB_NAME, Conv::Cdecl, &[id]);
        copy_cstr(h, arg + 0x50, db);
        let name = word(h, chr + 4);
        copy_cstr(h, arg + 0x70, name);
        let handle = word(h, arg + 4);
        let c = h.call_sym(GET_PTR32, Conv::Cdecl, &[handle]);
        let title = word(h, c + 4);
        h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[w, title]);
        h.call_sym(CHRVIEW_UPDATE_CONTROLS, Conv::Cdecl, &[w, chr]);
        h.call_sym(GU_BUTTON_SET_TOGGLE, Conv::Cdecl, &[w, 0xc, 1]);
        let clip = h.call_sym(CH_GET_CLIP, Conv::Cdecl, &[chr]);
        h.call_sym(
            GU_BUTTON_SET_TOGGLE_VALUE,
            Conv::Cdecl,
            &[w, 0xc, 1u32.wrapping_sub(clip)],
        )
    }

    pub const MAKE_FILELIST: u32 = 0x0085_09e0;
    pub const LINE_PRINTF: u32 = 0x0085_33b0;
    /// The character menu's file lists: the motion list (count, data at
    /// `+8`) and the character list; the `"data"`/`"prot"` roots, the list
    /// file and the two listing commands.
    pub const MOTION_LIST: u32 = 0x023c_bc50;
    pub const CHR_LIST: u32 = 0x023c_bc40;
    pub const ROOT_DATA: u32 = 0x00b5_ad14;
    pub const ROOT_PROT: u32 = 0x00b5_ad1c;
    pub const LIST_FILE: u32 = 0x00b5_aadc;
    pub const LIST_MOTIONS_CMD: u32 = 0x00b5_ad58;
    pub const LIST_CHRS_CMD: u32 = 0x00b5_ad24;

    /// A file list (`{count, _, data}` at `list`) into the selector `sel`
    /// -> the last selector call's `eax`.
    fn fill_file_selector(h: &mut impl Host, sel: u32, list: u32) -> u32 {
        let n = word(h, list);
        let mut eax = h.call_sym(GU_SELECTOR_RESET, Conv::Cdecl, &[sel, n]);
        let mut i = 0_i32;
        while i < word(h, list).cast_signed() {
            let k = i.cast_unsigned();
            let item = word(h, word(h, list + 8).wrapping_add(4 * k));
            eax = h.call_sym(GU_SELECTOR_SET_ITEM, Conv::Cdecl, &[sel, k, item, k]);
            i += 1;
        }
        eax
    }

    /// `ChrMenu_listMotions_0084d3f0(sel)` (`cdecl`; the command buffer at
    /// `frame`, the original's `ebp - 0x84`) -> the last selector call's
    /// `eax`: the current character's (light mode name) motion folder is
    /// listed on the host (`sg_make_filelist`) into the motion list and
    /// shown in `sel`.
    pub fn chrmenu_list_motions(h: &mut impl Host, frame: u32, sel: u32) -> u32 {
        let on = h.read::<u8>(h.rebase(LIGHT_MODE)) != 0;
        let (root, name) = if on {
            (ROOT_DATA, word(h, h.rebase(LIGHT_NAME_ON)))
        } else {
            (ROOT_PROT, word(h, h.rebase(LIGHT_NAME_OFF)))
        };
        let (f, r, l) = (
            h.rebase(LIST_MOTIONS_CMD),
            h.rebase(root),
            h.rebase(LIST_FILE),
        );
        let sprintf = word(h, h.rebase(SPRINTF));
        h.call(sprintf, Conv::Cdecl, &[frame, f, r, name, l]);
        let list = h.rebase(MOTION_LIST);
        h.call_sym(MAKE_FILELIST, Conv::Cdecl, &[list, frame, 0]);
        fill_file_selector(h, sel, list)
    }

    /// `ChrMenu_listCharacters_0084d4a0(sel)` (`cdecl`; the command buffer
    /// at `frame`, the original's `ebp - 0x84`) -> the last selector
    /// call's `eax`: the character folders of the light mode's root listed
    /// into the character list and shown in `sel`.
    pub fn chrmenu_list_characters(h: &mut impl Host, frame: u32, sel: u32) -> u32 {
        let on = h.read::<u8>(h.rebase(LIGHT_MODE)) != 0;
        let root = h.rebase(if on { ROOT_DATA } else { ROOT_PROT });
        let (f, l) = (h.rebase(LIST_CHRS_CMD), h.rebase(LIST_FILE));
        let sprintf = word(h, h.rebase(SPRINTF));
        h.call(sprintf, Conv::Cdecl, &[frame, f, root, l]);
        let list = h.rebase(CHR_LIST);
        h.call_sym(MAKE_FILELIST, Conv::Cdecl, &[list, frame, 0]);
        fill_file_selector(h, sel, list)
    }

    /// `"#LIGHT\t%d"` and the two light line formats.
    pub const SAVE_LIGHT_HEADER: u32 = 0x00b5_ae70;
    pub const SAVE_LIGHT_FULL: u32 = 0x00b5_ae7c;
    pub const SAVE_LIGHT_SPOT: u32 = 0x00b5_ae98;

    /// `Gui_SettingSaveLight()` -> the last line's `eax`: the debug lights
    /// written as a script (`Gu_LinePrintf`): `#LIGHT 35`, then for each
    /// of the 7 groups lights 0..2 with their colour, kind and range and
    /// lights 3..4 with their colour and kind (5 unused).
    pub fn gui_setting_save_light(h: &mut impl Host) -> u32 {
        let f = h.rebase(SAVE_LIGHT_HEADER);
        let mut eax = h.call_sym(LINE_PRINTF, Conv::Cdecl, &[f, 0x23]);
        let table = h.rebase(DEBUG_LIGHT_TABLE);
        for g in 0..7_u32 {
            for k in 0..6_u32 {
                let e = table + 0x1c * (6 * g + k);
                let kind = i32::from(h.read::<i8>(e + 0x1a)).cast_unsigned();
                let (c0, c1, c2) = (word(h, e), word(h, e + 4), word(h, e + 8));
                if k <= 2 {
                    let f = h.rebase(SAVE_LIGHT_FULL);
                    let (r0, r1) = (word(h, e + 0x10), word(h, e + 0x14));
                    eax = h.call_sym(
                        LINE_PRINTF,
                        Conv::Cdecl,
                        &[f, g, k, kind, c0, c1, c2, r0, r1],
                    );
                } else if k <= 4 {
                    let f = h.rebase(SAVE_LIGHT_SPOT);
                    eax = h.call_sym(LINE_PRINTF, Conv::Cdecl, &[f, g, k, kind, c0, c1, c2]);
                }
            }
        }
        eax
    }

    pub const GU_WINDOW_SET_ARG: u32 = 0x0084_b560;
    pub const GU_WINDOW_ADD_STATIC: u32 = 0x0084_a4d0;
    pub const GU_WINDOW_ADD_BUTTON: u32 = 0x0084_a470;
    /// The specular power hook (a bare `ret`).
    pub const SET_SPECULAR_POWER: u32 = 0x0083_fec0;
    /// The specular window's value (`0x023cbc34`) and its texts: `"%d"`,
    /// `"Spec"`, `"Specular"`, the arrows and `"256"`.
    pub const SPEC_VALUE: u32 = 0x023c_bc34;
    pub const FMT_D: u32 = 0x00b0_f604;
    pub const SPEC_TITLE: u32 = 0x00b5_adb8;
    pub const SPEC_LABEL: u32 = 0x00b5_adc0;
    pub const SPEC_LEFT: u32 = 0x00b5_adcc;
    pub const SPEC_256: u32 = 0x00b5_add0;
    pub const SPEC_RIGHT: u32 = 0x00b5_add4;

    /// `SpecWindowProc_0084ccc0(window, msg, arg)` (`cdecl`; the number
    /// buffer at `frame`, the original's `ebp - 0x18`) -> 0: the specular
    /// debug window. 0x8001 builds it (title, label, the arrows around the
    /// value, the value 256); 0x8005 steps the value by 1 (button 0 down,
    /// 2 up), 0x8009 by 10, clamped to 0..256, shown and applied.
    pub fn spec_window_proc(h: &mut impl Host, frame: u32, window: u32, msg: u32, arg: u32) -> u32 {
        let v = h.call_sym(GU_WINDOW_GET_ARG, Conv::Cdecl, &[window]);
        let step = match msg {
            0x8001 => {
                let val = h.rebase(SPEC_VALUE);
                h.write(val, 0_u32);
                h.call_sym(GU_WINDOW_SET_ARG, Conv::Cdecl, &[window, val]);
                let s = h.rebase(SPEC_TITLE);
                h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[window, s]);
                let items = [
                    (GU_WINDOW_ADD_STATIC, u32::MAX, SPEC_LABEL, [4, 2, 0x4b, 9]),
                    (GU_WINDOW_ADD_BUTTON, 0, SPEC_LEFT, [4, 0xb, 0xe, 9]),
                    (GU_WINDOW_ADD_STATIC, 1, SPEC_256, [0x11, 0xb, 0x28, 9]),
                    (GU_WINDOW_ADD_BUTTON, 2, SPEC_RIGHT, [0x3e, 0xb, 0xe, 9]),
                ];
                for (f, id, text, [x, y, w, ht]) in items {
                    let t = h.rebase(text);
                    h.call_sym(f, Conv::Cdecl, &[window, id, t, x, y, w, ht]);
                }
                h.write(val, 0x100_u32);
                return 0;
            }
            0x8005 => 1_u32,
            0x8009 => 10,
            _ => return 0,
        };
        let cur = word(h, v);
        match arg {
            0 => h.write(v, cur.wrapping_sub(step)),
            2 => h.write(v, cur.wrapping_add(step)),
            _ => {}
        }
        let cur = word(h, v).cast_signed();
        if cur < 0 {
            h.write(v, 0_u32);
        } else if cur > 0x100 {
            h.write(v, 0x100_u32);
        }
        let f = h.rebase(FMT_D);
        let sprintf = word(h, h.rebase(SPRINTF));
        let n = word(h, v);
        h.call(sprintf, Conv::Cdecl, &[frame, f, n]);
        h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 1, frame]);
        let n = word(h, v);
        h.call_sym(SET_SPECULAR_POWER, Conv::Cdecl, &[n]);
        0
    }

    pub const GU_SELECTOR_ITEM_VALUE: u32 = 0x0084_9d10;
    pub const GU_SELECTOR_ITEM_NAME: u32 = 0x0084_9d30;
    pub const GU_SELECTOR_MOUSE_FUKKI: u32 = 0x0084_a0d0;
    pub const CREATE_CHRVIEW: u32 = 0x0084_cea0;
    pub const CHR_LOAD_SELECTED: u32 = 0x0084_d5e0;
    pub const SG_SET_RESOURCE_STR: u32 = 0x0083_fe60;
    /// `_strdup` (import slot).
    pub const STRDUP: u32 = 0x00b0_c45c;

    /// `gui_chr_GWM_SELECTOR(window, msg, sel)` (`cdecl`) -> the last
    /// call's `eax` (`msg` for other messages): the character menu's
    /// selectors. 3 takes the picked name (`_strdup`) as the light mode's
    /// name, shows it and stores it as resource 15 (on) or 10 (off); 0xf
    /// loads the picked character (`0x0084d5e0`); 0x3e opens the view of
    /// the picked character.
    pub fn gui_chr_selector(h: &mut impl Host, window: u32, msg: u32, sel: u32) -> u32 {
        match msg {
            3 => {
                let on = h.read::<u8>(h.rebase(LIGHT_MODE)) != 0;
                let item = word(h, sel + 4);
                let name = h.call_sym(GU_SELECTOR_ITEM_NAME, Conv::Cdecl, &[item]);
                let f = word(h, h.rebase(STRDUP));
                let s = h.call(f, Conv::Cdecl, &[name]);
                let (slot, res) = if on {
                    (LIGHT_NAME_ON, 0xf)
                } else {
                    (LIGHT_NAME_OFF, 0xa)
                };
                let slot = h.rebase(slot);
                h.write(slot, s);
                h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 4, s]);
                let s = word(h, slot);
                h.call_sym(SG_SET_RESOURCE_STR, Conv::Cdecl, &[res, s])
            }
            0xf => h.call_sym(CHR_LOAD_SELECTED, Conv::Cdecl, &[sel]),
            0x3e => {
                let s = word(h, sel);
                h.call_sym(GU_SELECTOR_ITEM_VALUE, Conv::Cdecl, &[s]);
                let s = word(h, sel);
                let chr = h.call_sym(GU_SELECTOR_ITEM_VALUE, Conv::Cdecl, &[s]);
                h.call_sym(CREATE_CHRVIEW, Conv::Cdecl, &[chr]);
                let s = word(h, sel);
                h.call_sym(GU_SELECTOR_MOUSE_FUKKI, Conv::Cdecl, &[s, 0])
            }
            _ => msg,
        }
    }

    pub const SSCANF: u32 = 0x00b0_c448;
    pub const SG_DB_NUM: u32 = 0x0082_9d80;
    pub const CH_ID_FROM_NAME: u32 = 0x0082_6dc0;
    pub const CH_SET_ALLOCATE_ERR: u32 = 0x0082_85d0;
    pub const CH_ALLOCATE: u32 = 0x0082_4f90;
    pub const OP_ET_BINDEFF: u32 = 0x007f_b290;
    pub const CHN_READ_SYSTEM_MGRP: u32 = 0x0083_6870;
    pub const CH_SET_MOTION_TYPE: u32 = 0x0082_b3a0;
    pub const CH_SET_SYS_MOTION: u32 = 0x0083_7d00;
    pub const CH_MOTION_SET_LOOP: u32 = 0x0083_9960;
    pub const GU_MESSAGE_BOX: u32 = 0x0084_9970;
    /// `"%03d"`, `"ChrModel [%s:%s] Open Error!"`.
    pub const FMT_03D: u32 = 0x00b5_ad90;
    pub const CHR_OPEN_ERROR: u32 = 0x00b5_ad98;

    /// `ChrMenu_loadSelected_0084d5e0(sel)` (`cdecl`; the frame from the
    /// original's `ebp - 0x48` at `frame`: the character id `+0`, the
    /// name buffer `+4`, 64 bytes) -> the selector call's `eax`: the
    /// character picked in `sel[1]` is allocated: in light mode its id is
    /// the name's letter (`sg_getdbnum`) and number (`"%03d"`), otherwise
    /// `Ch_IdGetFromName` in the folder. A loaded character gets its
    /// effects bound and a view; in light mode its motion group read
    /// (and for kinds 1 and 3 the system motion 0x10, looping off). A
    /// failure shows a message. (A name without a number leaves the id
    /// cell as it was: the original's frame, 0 in the port: junk read,
    /// brief 2026-10-24.)
    pub fn chrmenu_load_selected<H9: Host>(h: &mut H9, frame: u32, sel: u32) -> u32 {
        let (cell, buf) = (frame, frame + 4);
        let item = |h: &H9| word(h, sel + 4);
        let folder;
        if h.read::<u8>(h.rebase(LIGHT_MODE)) != 0 {
            let s = item(h);
            let name = h.call_sym(GU_SELECTOR_ITEM_NAME, Conv::Cdecl, &[s]);
            copy_cstr(h, buf, name);
            let (f, sscanf) = (h.rebase(FMT_03D), word(h, h.rebase(SSCANF)));
            h.call(sscanf, Conv::Cdecl, &[buf + 1, f, cell]);
            let c = i32::from(h.read::<i8>(buf)).cast_unsigned();
            let db = h.call_sym(SG_DB_NUM, Conv::Cdecl, &[c]);
            folder = word(h, h.rebase(LIGHT_NAME_ON));
            let id = word(h, cell) | db.wrapping_shl(12);
            h.write(cell, id);
        } else {
            folder = word(h, h.rebase(LIGHT_NAME_OFF));
            let s = item(h);
            let name = h.call_sym(GU_SELECTOR_ITEM_NAME, Conv::Cdecl, &[s]);
            let id = h.call_sym(CH_ID_FROM_NAME, Conv::Cdecl, &[folder, name]);
            h.write(cell, id);
        }
        let s = item(h);
        let name = h.call_sym(GU_SELECTOR_ITEM_NAME, Conv::Cdecl, &[s]);
        h.call_sym(SG_SET_RESOURCE_STR, Conv::Cdecl, &[0xb, name]);
        h.call_sym(CH_SET_ALLOCATE_ERR, Conv::Cdecl, &[]);
        let id = word(h, cell);
        let chr = h.call_sym(CH_ALLOCATE, Conv::Cdecl, &[id]);
        if chr == 0 {
            let s = item(h);
            let name = h.call_sym(GU_SELECTOR_ITEM_NAME, Conv::Cdecl, &[s]);
            let (f, sprintf) = (h.rebase(CHR_OPEN_ERROR), word(h, h.rebase(SPRINTF)));
            h.call(sprintf, Conv::Cdecl, &[buf, f, folder, name]);
            h.call_sym(GU_MESSAGE_BOX, Conv::Cdecl, &[buf]);
        } else {
            h.call_sym(OP_ET_BINDEFF, Conv::Cdecl, &[chr]);
            h.call_sym(CREATE_CHRVIEW, Conv::Cdecl, &[chr]);
            if h.read::<u8>(h.rebase(LIGHT_MODE)) != 0 {
                let id = word(h, cell);
                let kind = id.cast_signed() >> 12;
                if kind == 1 || kind == 3 {
                    h.call_sym(CHN_READ_SYSTEM_MGRP, Conv::Cdecl, &[id, 1]);
                    h.call_sym(CH_SET_MOTION_TYPE, Conv::Cdecl, &[chr, 1]);
                    h.call_sym(CH_SET_SYS_MOTION, Conv::Cdecl, &[chr, 1, 0x10]);
                    h.call_sym(CH_MOTION_SET_LOOP, Conv::Cdecl, &[chr, 0]);
                } else {
                    h.call_sym(CHN_READ_SYSTEM_MGRP, Conv::Cdecl, &[id, 0]);
                }
            }
        }
        let s = item(h);
        h.call_sym(GU_SELECTOR_MOUSE_FUKKI, Conv::Cdecl, &[s, 0])
    }

    pub const SG_GET_RESOURCE_STR: u32 = 0x0083_fc30;
    pub const SG_GET_RESOURCE: u32 = 0x0083_fbe0;
    pub const SG_ADD_BUTTON: u32 = 0x0085_08a0;
    pub const SG_ADD_STATIC: u32 = 0x0085_08f0;
    pub const SG_ADD_Y: u32 = 0x0085_0820;
    pub const SG_GET_Y: u32 = 0x0085_0830;
    pub const SG_DISP_ENABLE: u32 = 0x0082_0650;
    pub const CH_CALC_NORMAL: u32 = 0x0083_4060;
    pub const CH_MOTION_HALF: u32 = 0x0083_9870;
    pub const CH_ENABLE_TEXTURE: u32 = 0x0083_0320;
    pub const CHRMENU_LIGHT_TEXTS: u32 = 0x0084_d540;
    /// The character menu's window (`0x01303768`) and its help text.
    pub const CHRMENU_WINDOW: u32 = 0x0130_3768;
    pub const CHRMENU_HELP: u32 = 0x00b5_ae60;
    /// The character menu's layout: buttons `(id, text)`, the folder name
    /// static (text 0) and line steps (`id` `u32::MAX`, the step).
    pub const CHRMENU_LAYOUT: [(u32, u32); 21] = [
        (0x54, 0x00b5_ade0),
        (u32::MAX, 0xffff_ffff),
        (4, 0),
        (3, 0x00b5_adf0),
        (2, 0x00b5_ae00),
        (1, 0x00b5_6a08),
        (0x40, 0x00b5_ae08),
        (0x47, 0x00b5_ae10),
        (u32::MAX, 2),
        (8, 0x00b5_ae18),
        (0x4e, 0x00b5_ae20),
        (0x45, 0x00b5_ae2c),
        (u32::MAX, 1),
        (0x11, 0x00b5_ae38),
        (0x4c, 0x00b5_ae40),
        (u32::MAX, 2),
        (0xb, 0x00b5_ae48),
        (0xe, 0x00b5_ae50),
        (0x5a, 0x00b5_ae58),
        (0x69, 0x00b3_c3ec),
        (0x68, 0x00b5_adc0),
    ];

    /// `Gui_ChrMenuInit(window, info)` (`cdecl`) -> the layout's height:
    /// the character menu debug window. The prototype folder name
    /// (resource 10, the user name when empty) and the data name
    /// (resource 15) are taken, the buttons laid out (`sg_addButtonS`,
    /// `sg_AddY` steps, the folder name as a static), the toggles 0xb,
    /// 0x45, 8, 0x5a, 0x64 enabled and set from the display, normal and
    /// half motion flags, the light mode (resource 15's flag) read and
    /// its texts shown, the texture toggle set; `info + 0x7c` gets the
    /// height, `+0x80` the help text.
    pub fn gui_chr_menu_init(h: &mut impl Host, window: u32, info: u32) -> u32 {
        let mut name = h.call_sym(SG_GET_RESOURCE_STR, Conv::Cdecl, &[0xa]);
        if h.read::<u8>(name) == 0 {
            name = h.call_sym(USER_NAME, Conv::Cdecl, &[]);
            h.call_sym(SG_SET_RESOURCE_STR, Conv::Cdecl, &[0xa, name]);
        }
        let w_at = h.rebase(CHRMENU_WINDOW);
        h.write(w_at, window);
        let off = h.rebase(LIGHT_NAME_OFF);
        h.write(off, name);
        let data = h.call_sym(SG_GET_RESOURCE_STR, Conv::Cdecl, &[0xf]);
        let on = h.rebase(LIGHT_NAME_ON);
        h.write(on, data);
        for (id, text) in CHRMENU_LAYOUT {
            if id == u32::MAX {
                h.call_sym(SG_ADD_Y, Conv::Cdecl, &[text]);
            } else if text == 0 {
                let s = word(h, off);
                h.call_sym(SG_ADD_STATIC, Conv::Cdecl, &[window, id, s]);
            } else {
                let t = h.rebase(text);
                h.call_sym(SG_ADD_BUTTON, Conv::Cdecl, &[window, id, t]);
            }
        }
        for id in [0xb, 0x45, 8, 0x5a, 0x64] {
            h.call_sym(GU_BUTTON_SET_TOGGLE, Conv::Cdecl, &[window, id, 1]);
        }
        let d = h.call_sym(SG_DISP_ENABLE, Conv::Cdecl, &[]);
        h.call_sym(
            GU_BUTTON_SET_TOGGLE_VALUE,
            Conv::Cdecl,
            &[window, 0xb, 1u32.wrapping_sub(d)],
        );
        let v = h.call_sym(CH_CALC_NORMAL, Conv::Cdecl, &[]);
        h.call_sym(GU_BUTTON_SET_TOGGLE_VALUE, Conv::Cdecl, &[window, 0x45, v]);
        let v = h.call_sym(CH_MOTION_HALF, Conv::Cdecl, &[]);
        h.call_sym(GU_BUTTON_SET_TOGGLE_VALUE, Conv::Cdecl, &[window, 0x64, v]);
        let mode = h.call_sym(SG_GET_RESOURCE, Conv::Cdecl, &[0xf, 0]);
        let m = h.rebase(LIGHT_MODE);
        h.write(m, mode as u8);
        h.call_sym(CHRMENU_LIGHT_TEXTS, Conv::Cdecl, &[window]);
        let w = word(h, w_at);
        if w != 0 {
            let t = h.call_sym(CH_ENABLE_TEXTURE, Conv::Cdecl, &[]);
            h.call_sym(
                GU_BUTTON_SET_TOGGLE_VALUE,
                Conv::Cdecl,
                &[w, 8, 1u32.wrapping_sub(t)],
            );
        }
        let y = h.call_sym(SG_GET_Y, Conv::Cdecl, &[]);
        h.write(info + 0x7c, y);
        let help = h.rebase(CHRMENU_HELP);
        h.write(info + 0x80, help);
        y
    }

    /// One value of the light editor: the message's step applied when the
    /// row's down (`row`) or up (`row + 2`) button is the one pressed
    /// (0x8005: 1, 0x8009: 10, 0x800d: to 0 / 255 unless `as_ten`).
    fn light_edit_step<H10: Host>(
        h: &mut H10,
        at: u32,
        msg: u32,
        row: u32,
        button: u32,
        as_ten: bool,
    ) {
        let msg = if msg == 0x800d && as_ten { 0x8009 } else { msg };
        // the value stepped down or up
        let step = |x: u32, up: bool| match (msg, up) {
            (0x8005, false) => Some(x.wrapping_sub(1)),
            (0x8005, true) => Some(x.wrapping_add(1)),
            (0x8009, false) => Some(x.wrapping_sub(10)),
            (0x8009, true) => Some(x.wrapping_add(10)),
            (0x800d, false) => Some(0),
            (0x800d, true) => Some(0xff),
            _ => None,
        };
        for (id, up) in [(row, false), (row.wrapping_add(2), true)] {
            if id == button
                && let Some(x) = step(word(h, at), up)
            {
                h.write(at, x);
            }
        }
    }

    fn clamp_byte(h: &mut impl Mem, at: u32) {
        if word(h, at).cast_signed() < 0 {
            h.write(at, 0_u32);
        }
        if word(h, at).cast_signed() > 0xff {
            h.write(at, 0xff_u32);
        }
    }

    /// `ChrMenu_editLight_0084d940(values, _, msg, button)` (`cdecl`) ->
    /// the last row's `eax` (the angle's quotient by 360, or the wrapped
    /// angle): the light editor's six rows of buttons (three apart): the
    /// red, green and blue values (0..255), a row stepping all three, and
    /// two angles (wrapped to 0..359, the 0x800d reset acting as a
    /// step of 10 there).
    pub fn chrmenu_edit_light<H10: Host>(h: &mut H10, values: u32, msg: u32, button: u32) -> u32 {
        let mut eax = 0_u32;
        for i in 0..6_u32 {
            let row = 3 * i;
            let at = values + 4 * i;
            light_edit_step(h, at, msg, row, button, i >= 4);
            match i {
                0..=2 => clamp_byte(h, at),
                3 => {
                    for j in 0..3 {
                        let at = values + 4 * j;
                        light_edit_step(h, at, msg, row, button, false);
                        clamp_byte(h, at);
                    }
                }
                _ => {
                    let v = word(h, at).cast_signed();
                    let (q, r) = (v.wrapping_div(360), v.wrapping_rem(360));
                    h.write(at, r);
                    eax = q.cast_unsigned();
                    if r < 0 {
                        let w = r + 360;
                        h.write(at, w);
                        eax = w.cast_unsigned();
                    }
                }
            }
        }
        eax
    }

    pub const LIGHT_APPLY: u32 = 0x0084_ebd0;
    pub const YI_SET_BACK_COLOR: u32 = 0x0090_a040;
    pub const CHRMENU_EDIT_LIGHT: u32 = 0x0084_d940;
    pub const REPORT_RANGE_FAILURE: u32 = 0x0094_9b02;
    /// The light window's light (`0x01303cc8`), the row labels (pointer
    /// table `0x00c4df80`), the light flags (`0x01303c18`, 6 bytes) and
    /// the texts `"on"`, `"off"`, `"0"`, `"rgb"`.
    pub const LIGHT_WINDOW_LIGHT: u32 = 0x0130_3cc8;
    pub const LIGHT_ROW_LABELS: u32 = 0x00c4_df80;
    pub const LIGHT_FLAGS: u32 = 0x0130_3c18;
    pub const TEXT_ON: u32 = 0x00b4_88d8;
    pub const TEXT_OFF: u32 = 0x00b4_88dc;
    pub const TEXT_ZERO: u32 = 0x00b7_c998;
    pub const TEXT_RGB: u32 = 0x00b5_aed0;
    /// The light window's on/off button.
    pub const LIGHT_ON_BUTTON: u32 = 0x4d2;

    /// `LightWindowProc_0084e100(window, msg, button)` (`cdecl`; the frame
    /// from the original's `ebp - 0x28` at `frame`: a black colour `+0`,
    /// the number buffer `+0x10`) -> 0: the debug light window. 0x8001
    /// builds it (an on/off button unless it is the back colour, light
    /// 4; four rows of arrows around a value, the fourth labelled
    /// `"rgb"`); 0x8003 clears the light's flag; 0x8005/0x8009/0x800d
    /// edit the values (`ChrMenu_editLight`) unless on the on/off button;
    /// 0x8007 on it toggles the light; 0x8011 shows the values and
    /// applies the light (the back colour, black when off).
    pub fn light_window_proc<H11: Host>(
        h: &mut H11,
        frame: u32,
        window: u32,
        msg: u32,
        button: u32,
    ) -> u32 {
        let v = h.call_sym(GU_WINDOW_GET_ARG, Conv::Cdecl, &[window]);
        let kind = |h: &H11| i32::from(h.read::<i16>(v.wrapping_add(0x18)));
        match msg {
            0x8001 => light_window_build(h, window),
            0x8011 => {
                let on = h.read::<i8>(v + 0x1a);
                h.call_sym(
                    GU_BUTTON_SET_TOGGLE_VALUE,
                    Conv::Cdecl,
                    &[window, LIGHT_ON_BUTTON, i32::from(on).cast_unsigned()],
                );
                let t = h.rebase(if h.read::<u8>(v + 0x1a) != 0 {
                    TEXT_ON
                } else {
                    TEXT_OFF
                });
                h.call_sym(
                    GU_CONTROL_SET_TEXT,
                    Conv::Cdecl,
                    &[window, LIGHT_ON_BUTTON, t],
                );
                let buf = frame + 0x10;
                for k in [0_u32, 1, 2] {
                    let (f, sprintf) = (h.rebase(FMT_D), word(h, h.rebase(SPRINTF)));
                    let n = word(h, v + 4 * k);
                    h.call(sprintf, Conv::Cdecl, &[buf, f, n]);
                    h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 3 * k + 1, buf]);
                }
                if kind(h) != 4 {
                    h.call_sym(LIGHT_APPLY, Conv::Cdecl, &[v]);
                } else if h.read::<u8>(v + 0x1a) != 0 {
                    h.call_sym(YI_SET_BACK_COLOR, Conv::Cdecl, &[v]);
                } else {
                    h.write(frame, [0_u32; 4]);
                    h.call_sym(YI_SET_BACK_COLOR, Conv::Cdecl, &[frame]);
                }
            }
            _ => light_window_edit(h, v, window, msg, button),
        }
        0
    }

    /// The light windows' common messages: 0x8003 clears the light's flag
    /// (a range failure past kind 5), 0x8005/0x8009/0x800d edit the values
    /// (`ChrMenu_editLight`) unless on the on/off button, 0x8007 on it
    /// toggles the light.
    fn light_window_edit(h: &mut impl Host, v: u32, window: u32, msg: u32, button: u32) {
        match msg {
            0x8003 => {
                let k = i32::from(h.read::<i16>(v.wrapping_add(0x18)));
                if k.cast_unsigned() >= 6 {
                    h.call_sym(REPORT_RANGE_FAILURE, Conv::Cdecl, &[]);
                    return;
                }
                let f = h.rebase(LIGHT_FLAGS);
                h.write(f.wrapping_add(k.cast_unsigned()), 0_u8);
            }
            0x8005 | 0x8009 | 0x800d if button != LIGHT_ON_BUTTON => {
                h.call_sym(CHRMENU_EDIT_LIGHT, Conv::Cdecl, &[v, window, msg, button]);
            }
            0x8007 if button == LIGHT_ON_BUTTON => {
                let b = h.read::<u8>(v + 0x1a).wrapping_sub(1) & 1;
                h.write(v + 0x1a, b);
            }
            _ => {}
        }
    }

    /// One row of a light window: its label, the down button `id`, the
    /// value `id + 1` ("0") and the up button `id + 2`, at `y`.
    fn light_row(h: &mut impl Host, window: u32, id: u32, label: u32, y: u32) {
        let (left, zero, right) = (
            h.rebase(SPEC_LEFT),
            h.rebase(TEXT_ZERO),
            h.rebase(SPEC_RIGHT),
        );
        h.call_sym(
            GU_WINDOW_ADD_STATIC,
            Conv::Cdecl,
            &[window, u32::MAX, label, 2, y, 10, 9],
        );
        h.call_sym(
            GU_WINDOW_ADD_BUTTON,
            Conv::Cdecl,
            &[window, id, left, 0xc, y, 0xe, 9],
        );
        h.call_sym(
            GU_WINDOW_ADD_STATIC,
            Conv::Cdecl,
            &[window, id + 1, zero, 0x1b, y, 0x28, 9],
        );
        h.call_sym(
            GU_WINDOW_ADD_BUTTON,
            Conv::Cdecl,
            &[window, id + 2, right, 0x48, y, 0xe, 9],
        );
    }

    fn light_window_build(h: &mut impl Host, window: u32) {
        let light = word(h, h.rebase(LIGHT_WINDOW_LIGHT));
        h.call_sym(GU_WINDOW_SET_ARG, Conv::Cdecl, &[window, light]);
        let mut y = 0_u32;
        if h.read::<i16>(light.wrapping_add(0x18)) != 4 {
            let t = h.rebase(TEXT_ON);
            h.call_sym(
                GU_WINDOW_ADD_BUTTON,
                Conv::Cdecl,
                &[window, LIGHT_ON_BUTTON, t, 0xc, 0, 0x46, 8],
            );
            h.call_sym(
                GU_BUTTON_SET_TOGGLE,
                Conv::Cdecl,
                &[window, LIGHT_ON_BUTTON, 1],
            );
            h.call_sym(
                GU_BUTTON_SET_TOGGLE_VALUE,
                Conv::Cdecl,
                &[window, LIGHT_ON_BUTTON, 1],
            );
            y = 9;
        }
        for k in 0..4_u32 {
            let id = 3 * k;
            let label = word(h, h.rebase(LIGHT_ROW_LABELS) + 4 * k);
            light_row(h, window, id, label, y);
            if k == 3 {
                let t = h.rebase(TEXT_RGB);
                h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, id + 1, t]);
            }
            y += 0xb;
        }
    }

    use ffx_bridge::x87;
    pub const GU_PACKET_BEGIN_TAG: u32 = 0x0084_8530;
    pub const GU_PACKET_END_TAG: u32 = 0x0084_8460;
    pub const GU_SET_TEST: u32 = 0x0084_85d0;
    pub const GU_SET_PRIM: u32 = 0x0084_8480;
    pub const GU_SET_RGBAQ: u32 = 0x0084_84c0;
    pub const GU_SET_XYZF2: u32 = 0x0084_8680;
    pub const SG_CAMERA_GET_YAW: u32 = 0x0083_f2d0;
    pub const ANGLE_WRAP: u32 = 0x0084_eb60;
    /// `_CIsin`, `_CIcos` (thunks).
    pub const CI_SIN: u32 = 0x0094_98c8;
    pub const CI_COS: u32 = 0x0094_98c2;
    /// The direction light window's row labels (pointer table
    /// `0x00c4df64`, six) and its rows' bottom (`0x01303ccc`, where the
    /// gizmo is drawn).
    pub const DIR_ROW_LABELS: u32 = 0x00c4_df64;
    pub const DIR_WINDOW_BOTTOM: u32 = 0x0130_3ccc;
    /// `f64` constants: 20, 40, 45, pi/2 (1.570796), pi (3.141592), 180.
    pub const C_20: u32 = 0x00b4_9968;
    pub const C_40: u32 = 0x00b5_af28;
    pub const C_45: u32 = 0x00b5_af30;
    pub const C_HALF_PI: u32 = 0x00b5_82e8;
    pub const C_PI: u32 = 0x00b5_82f0;
    pub const C_180: u32 = 0x00b4_8378;

    fn qword(h: &impl Mem, sym: u32) -> f64 {
        f64::from_bits(h.read::<u64>(h.rebase(sym)))
    }

    /// `_CIsin`/`_CIcos` of an `f32`, stored as `f32` and loaded again.
    fn trig(h: &mut impl Host, sym: u32, x: u32) -> f64 {
        let at = h.rebase(sym);
        x87::load(h.call_intrinsic(at, x87::load(x)).to_bits())
    }

    /// An `i32` through `fild` and an `f32` store.
    fn int_f32(n: u32) -> u32 {
        x87::store(f64::from(n.cast_signed()))
    }

    /// Degrees (`i32`) to radians as the original rounds it (`f32`).
    fn radians(h: &impl Mem, deg: u32) -> u32 {
        x87::store(x87::div(
            x87::mul(x87::load(int_f32(deg)), qword(h, C_PI)),
            qword(h, C_180),
        ))
    }

    fn xyz(h: &mut impl Host, x: u32, y: u32) {
        h.call_sym(GU_SET_XYZF2, Conv::Cdecl, &[x, y]);
    }

    fn rgbaq(h: &mut impl Host, c: [u32; 4]) {
        h.call_sym(GU_SET_RGBAQ, Conv::Cdecl, &c);
    }

    /// The gizmo's 80 x 40 frame from `y` down, as two lines per side.
    fn gizmo_box(h: &mut impl Host, y: u32) {
        let b = y + 0x28;
        for (x, y) in [
            (5, y),
            (0x55, y),
            (0x55, y),
            (0x55, b),
            (0x55, b),
            (5, b),
            (5, b),
            (5, y),
        ] {
            xyz(h, x, y);
        }
    }

    /// `FUN_0084dac0(window, msg, button)` (`cdecl`; the number buffer at
    /// `buf`, the original's `ebp - 0x18`) -> 0: the debug direction
    /// light window. 0x8001 builds it (the on/off button, six rows of
    /// arrows around a value, labels from `0x00c4df64`; the rows' bottom
    /// kept at `0x01303ccc`); 0x8003/0x8005/0x8009/0x800d/0x8007 as the
    /// light window; 0x8011 applies the light (`FUN_0084ebd0`), shows its
    /// values (the fourth row none) and draws the gizmo below the rows:
    /// the camera's yaw (two lines of the yaw and the yaw + pi/2 as 40 x
    /// 20 ellipses around (45, bottom + 20)), the light's angle `+0x10`
    /// past the yaw, its elevation `-[+0x14]` in a second box.
    pub fn dir_light_window_proc(
        h: &mut impl Host,
        buf: u32,
        window: u32,
        msg: u32,
        button: u32,
    ) -> u32 {
        let v = h.call_sym(GU_WINDOW_GET_ARG, Conv::Cdecl, &[window]);
        match msg {
            0x8001 => {
                let light = word(h, h.rebase(LIGHT_WINDOW_LIGHT));
                h.call_sym(GU_WINDOW_SET_ARG, Conv::Cdecl, &[window, light]);
                let t = h.rebase(TEXT_ON);
                h.call_sym(
                    GU_WINDOW_ADD_BUTTON,
                    Conv::Cdecl,
                    &[window, LIGHT_ON_BUTTON, t, 0xc, 0, 0x46, 8],
                );
                h.call_sym(
                    GU_BUTTON_SET_TOGGLE,
                    Conv::Cdecl,
                    &[window, LIGHT_ON_BUTTON, 1],
                );
                h.call_sym(
                    GU_BUTTON_SET_TOGGLE_VALUE,
                    Conv::Cdecl,
                    &[window, LIGHT_ON_BUTTON, 1],
                );
                let mut y = 9;
                for k in 0..6_u32 {
                    let label = word(h, h.rebase(DIR_ROW_LABELS) + 4 * k);
                    light_row(h, window, 3 * k, label, y);
                    y += 0xb;
                }
                let at = h.rebase(DIR_WINDOW_BOTTOM);
                h.write(at, y);
            }
            0x8011 => dir_light_draw(h, buf, window, v),
            _ => light_window_edit(h, v, window, msg, button),
        }
        0
    }

    fn dir_light_draw(h: &mut impl Host, buf: u32, window: u32, v: u32) {
        h.call_sym(LIGHT_APPLY, Conv::Cdecl, &[v]);
        let on = h.read::<i8>(v + 0x1a);
        h.call_sym(
            GU_BUTTON_SET_TOGGLE_VALUE,
            Conv::Cdecl,
            &[window, LIGHT_ON_BUTTON, i32::from(on).cast_unsigned()],
        );
        let t = h.rebase(if h.read::<u8>(v + 0x1a) != 0 {
            TEXT_ON
        } else {
            TEXT_OFF
        });
        h.call_sym(
            GU_CONTROL_SET_TEXT,
            Conv::Cdecl,
            &[window, LIGHT_ON_BUTTON, t],
        );
        for k in 0..6_u32 {
            if k == 3 {
                let t = h.rebase(TEXT_RGB);
                h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 3 * k + 1, t]);
            } else {
                let (f, sprintf) = (h.rebase(FMT_D), word(h, h.rebase(SPRINTF)));
                let n = word(h, v + 4 * k);
                h.call(sprintf, Conv::Cdecl, &[buf, f, n]);
                h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 3 * k + 1, buf]);
            }
        }
        h.call_sym(GU_PACKET_BEGIN_TAG, Conv::Cdecl, &[window]);
        h.call_sym(GU_SET_TEST, Conv::Cdecl, &[1, 5, 0x7f, 0, 0, 0, 0, 1]);
        h.call_sym(GU_SET_PRIM, Conv::Cdecl, &[1]);
        let centre = word(h, h.rebase(DIR_WINDOW_BOTTOM)).wrapping_add(0x14);
        rgbaq(h, [0, 0, 0xff, 0xff]);
        let yaw = h
            .call_sym_st0(SG_CAMERA_GET_YAW, Conv::Cdecl, &[])
            .to_bits();
        let (c45, c40, c20) = (qword(h, C_45), qword(h, C_40), qword(h, C_20));
        let cy = int_f32(centre);
        // the yaw's line, through the centre
        let c = trig(h, CI_COS, yaw);
        let s = trig(h, CI_SIN, yaw);
        rgbaq(h, [0, 0xff, 0, 0xff]);
        let (s20, c40y) = (x87::mul(s, c20), x87::mul(c, c40));
        let y = x87::ftol(h, x87::add(s20, x87::load(cy)));
        let x = x87::ftol(h, x87::add(c40y, c45));
        xyz(h, x, y);
        let y = x87::ftol(h, x87::sub(x87::load(cy), s20));
        let x = x87::ftol(h, x87::sub(c45, c40y));
        xyz(h, x, y);
        // the one a quarter turn on
        let a = x87::store(x87::add(x87::load(yaw), qword(h, C_HALF_PI)));
        let w = h.call_sym_st0(ANGLE_WRAP, Conv::Cdecl, &[a]).to_bits();
        let c = trig(h, CI_COS, w);
        let s = trig(h, CI_SIN, w);
        rgbaq(h, [0, 0xff, 0xff, 0xff]);
        let (s20, c40y) = (x87::mul(s, c20), x87::mul(c, c40));
        let y = x87::ftol(h, x87::sub(x87::load(cy), s20));
        let x = x87::ftol(h, x87::sub(c45, c40y));
        xyz(h, x, y);
        let y = x87::ftol(h, x87::add(x87::load(cy), s20));
        let x = x87::ftol(h, x87::add(c40y, c45));
        xyz(h, x, y);
        // the light's angle past the yaw, from the centre
        let r = radians(h, word(h, v + 0x10));
        let wr = h.call_sym_st0_f64(ANGLE_WRAP, Conv::Cdecl, &[r]);
        let pi = qword(h, C_PI);
        let b = x87::store(x87::add(x87::add(wr, x87::load(yaw)), pi));
        let b = h.call_sym_st0(ANGLE_WRAP, Conv::Cdecl, &[b]).to_bits();
        rgbaq(h, [0xff, 0, 0, 0xff]);
        xyz(h, 0x2d, centre);
        let s = trig(h, CI_SIN, b);
        let y = x87::ftol(h, x87::sub(x87::load(cy), x87::mul(s, c20)));
        let c = trig(h, CI_COS, b);
        let x = x87::ftol(h, x87::sub(c45, x87::mul(c, c40)));
        xyz(h, x, y);
        rgbaq(h, [0, 0, 0, 0xff]);
        let top = word(h, h.rebase(DIR_WINDOW_BOTTOM));
        gizmo_box(h, top);
        // the elevation, in a second box
        let top = word(h, h.rebase(DIR_WINDOW_BOTTOM));
        let (second, mid) = (top.wrapping_add(0x2c), top.wrapping_add(0x40));
        rgbaq(h, [0, 0xff, 0, 0xff]);
        xyz(h, 5, mid);
        xyz(h, 0x55, mid);
        let r = radians(h, word(h, v + 0x14).wrapping_neg());
        let e = h.call_sym_st0(ANGLE_WRAP, Conv::Cdecl, &[r]).to_bits();
        let c = trig(h, CI_COS, e);
        let x = x87::ftol(h, x87::add(x87::mul(c, c40), c45));
        let s = trig(h, CI_SIN, e);
        let y = x87::ftol(h, x87::add(x87::mul(s, c20), x87::load(int_f32(mid))));
        rgbaq(h, [0xff, 0, 0, 0xff]);
        xyz(h, 0x2d, mid);
        xyz(h, x, y);
        rgbaq(h, [0, 0, 0, 0xff]);
        gizmo_box(h, second);
        h.call_sym(GU_PACKET_END_TAG, Conv::Cdecl, &[]);
    }

    pub const SG_LIGHT_SET_COLOR: u32 = 0x0083_6310;
    pub const SG_LIGHT_SET_DIR: u32 = 0x0083_6430;
    /// The six lights being edited (`0x01303c20`, 0x1c bytes each).
    pub const EDIT_LIGHTS: u32 = 0x0130_3c20;
    /// 255 (`f64`).
    pub const C_255: u32 = 0x00b1_70c0;

    /// `FUN_0084e3c0(group, load)` (`cdecl`; the colour/direction vector
    /// at `vec`, the original's aligned `ebp - 0x20`) -> the group's end
    /// in the debug light table: the six edited lights (`0x01303c20`)
    /// saved into the table's group (0xa8 bytes each) when `load` is 0;
    /// otherwise loaded from it, each then set as a light of its kind
    /// (`+0x18`): its colour (r, g, b / 255, or black when `+0x1a` is
    /// clear; w 0) and direction (`(sin b cos a, sin a, cos b cos a, 0)`
    /// from the degrees `a` = `+0x14`, `b` = `+0x10`), and the three
    /// lights after it applied (`FUN_0084ebd0`) and the fourth's back
    /// colour set (`yiSetBackColor`) - every time, past the six for the
    /// last ones.
    pub fn light_group_load_save(h: &mut impl Host, vec: u32, group: u32, load: u32) -> u32 {
        let mut entry = h
            .rebase(DEBUG_LIGHT_TABLE)
            .wrapping_add(group.wrapping_mul(0xa8));
        let first = h.rebase(EDIT_LIGHTS);
        for i in 0..6 {
            let cur = first + 0x1c * i;
            let (from, to) = if load == 0 {
                (cur, entry)
            } else {
                (entry, cur)
            };
            // `rep movsd`, a word at a time (group 7 overlaps the lights)
            for k in 0..7 {
                let w = word(h, from + 4 * k);
                h.write(to + 4 * k, w);
            }
            if load != 0 {
                light_set(h, vec, cur);
            }
            entry = entry.wrapping_add(0x1c);
        }
        entry
    }

    fn light_set(h: &mut impl Host, vec: u32, cur: u32) {
        h.write(vec + 0xc, 0_u32);
        let col = if h.read::<u8>(cur + 0x1a) != 0 {
            let c255 = qword(h, C_255);
            [0, 4, 8].map(|o| x87::store(x87::div(x87::load(int_f32(word(h, cur + o))), c255)))
        } else {
            [0; 3]
        };
        h.write(vec, col);
        let kind = i32::from(h.read::<i16>(cur + 0x18)).cast_unsigned();
        h.call_sym(SG_LIGHT_SET_COLOR, Conv::Cdecl, &[kind, vec]);
        let r = radians(h, word(h, cur + 0x14));
        let a = h.call_sym_st0(ANGLE_WRAP, Conv::Cdecl, &[r]).to_bits();
        let sa = trig(h, CI_SIN, a);
        h.write(vec + 4, x87::store(sa));
        let ca = trig(h, CI_COS, a);
        let r = radians(h, word(h, cur + 0x10));
        let b = h.call_sym_st0(ANGLE_WRAP, Conv::Cdecl, &[r]).to_bits();
        let sb = trig(h, CI_SIN, b);
        h.write(vec, x87::store(x87::mul(sb, ca)));
        let cb = trig(h, CI_COS, b);
        h.write(vec + 8, x87::store(x87::mul(cb, ca)));
        h.call_sym(SG_LIGHT_SET_DIR, Conv::Cdecl, &[kind, vec]);
        for o in [0x1c, 0x38, 0x54] {
            h.call_sym(LIGHT_APPLY, Conv::Cdecl, &[cur + o]);
        }
        h.call_sym(YI_SET_BACK_COLOR, Conv::Cdecl, &[cur + 0x70]);
    }

    pub const GU_SPIN_MSG: u32 = 0x0085_1b20;
    pub const GU_SPIN_ADD: u32 = 0x0085_1c70;
    pub const GU_CONTROL_SET_TEXTF: u32 = 0x0084_a810;
    pub const CH_GET_DISABLE_SPEC: u32 = 0x0082_eb80;
    pub const CH_GET_FOURCE_SPEC: u32 = 0x0082_eb90;
    pub const CH_GET_PARA_EDIT: u32 = 0x0082_ebe0;
    pub const CH_SET_DISABLE_SPEC: u32 = 0x0083_00d0;
    pub const CH_SET_FOURCE_SPEC: u32 = 0x0083_00e0;
    pub const CH_SET_PARA_EDIT: u32 = 0x0083_00f0;
    pub const CH_SET_SPEC_SIZE: u32 = 0x0084_30d0;
    pub const CH_SET_SPEC_COEFFICIENT: u32 = 0x0084_3090;
    pub const CH_SET_SPEC_COLOR: u32 = 0x0084_30a0;
    /// The specular window's spin controls (0x30 bytes each): size,
    /// coefficient, red, green, blue.
    pub const SPIN_SIZE: u32 = 0x0132_5398;
    pub const SPIN_COEF: u32 = 0x0132_53c8;
    pub const SPIN_RED: u32 = 0x0132_52d8;
    pub const SPIN_GREEN: u32 = 0x0132_5308;
    pub const SPIN_BLUE: u32 = 0x0132_5338;
    /// The byte 0x8003 clears (`0x013252cb`).
    pub const SPEC_WINDOW_FLAG: u32 = 0x0132_52cb;
    /// `"Specular"`, `"ALLON"`, `"ALLOFF"`, `"EDIT"`, `"COEF"`, `"SIZE"`,
    /// `"R"`, `"G"`, `"B"`, `"%d.%d"`; 10 (`f64`).
    pub const TEXT_SPECULAR: u32 = 0x00b5_adc0;
    pub const TEXT_ALLON: u32 = 0x00b5_b534;
    pub const TEXT_ALLOFF: u32 = 0x00b5_b53c;
    pub const TEXT_EDIT: u32 = 0x00b5_54dc;
    pub const TEXT_COEF: u32 = 0x00b5_b544;
    pub const TEXT_SIZE: u32 = 0x00b5_b54c;
    pub const TEXT_R: u32 = 0x00b8_f7e8;
    pub const TEXT_G: u32 = 0x00b8_f7bc;
    pub const TEXT_B: u32 = 0x00b8_f7ac;
    pub const FMT_TENTHS: u32 = 0x00b5_b554;
    pub const C_10: u32 = 0x00b9_22b0;

    /// A spin control's set-up: its three ids, format (0 or an image
    /// address, rebased), value, minimum and maximum; the fixed words
    /// (`+0` 1 (`u16`), `+2` 1 (byte), `+4` 1, `+8` -1, `+0xc` 10, `+0x2c` 0).
    fn spin_init(
        h: &mut impl Mem,
        spin: u32,
        ids: [u32; 3],
        fmt: u32,
        value: u32,
        min: u32,
        max: u32,
    ) {
        h.write(spin + 0x10, ids);
        h.write(spin + 0x1c, fmt);
        h.write(spin + 0x20, value);
        h.write(spin + 0x28, min);
        h.write(spin + 0x24, max);
        h.write(spin, 1_u16);
        h.write(spin + 2, 1_u8);
        h.write(spin + 4, [1_u32, u32::MAX, 10]);
        h.write(spin + 0x2c, 0_u32);
    }

    /// `FUN_00852a30(window, msg, button)` (`cdecl`) -> 0: the specular
    /// window. Every message goes to its five spin controls first
    /// (`Gu_SpinMsg`). 0x8001 builds it (title, the toggles ALLON 6,
    /// ALLOFF 7, EDIT 8, the spins size 0.1..10 (1.0), coefficient
    /// 0..10 (2.0) and red, green, blue 0..255 (128)); 0x8003 clears
    /// `0x013252cb`; 0x8007 toggles "force" (6, clearing "disable"),
    /// "disable" (7, clearing edit and force) or edit (8, unless
    /// disabled); 0x8011 shows them, enables the spins only in edit, and
    /// sets the size, coefficient (both tenths) and colour.
    pub fn spec_edit_window_proc(h: &mut impl Host, window: u32, msg: u32, button: u32) -> u32 {
        for spin in [SPIN_SIZE, SPIN_COEF, SPIN_RED, SPIN_GREEN, SPIN_BLUE] {
            let s = h.rebase(spin);
            h.call_sym(GU_SPIN_MSG, Conv::Cdecl, &[window, msg, button, s]);
        }
        match msg {
            0x8001 => spec_edit_build(h, window),
            0x8003 => {
                let at = h.rebase(SPEC_WINDOW_FLAG);
                h.write(at, 0_u8);
            }
            0x8007 => spec_edit_toggle(h, button),
            0x8011 => spec_edit_show(h, window),
            _ => {}
        }
        0
    }

    fn get(h: &mut impl Host, f: u32) -> u32 {
        h.call_sym(f, Conv::Cdecl, &[])
    }

    fn spec_edit_toggle(h: &mut impl Host, button: u32) {
        match button {
            6 => {
                let v = 1_u32.wrapping_sub(get(h, CH_GET_FOURCE_SPEC));
                h.call_sym(CH_SET_FOURCE_SPEC, Conv::Cdecl, &[v]);
                if get(h, CH_GET_FOURCE_SPEC) != 0 {
                    h.call_sym(CH_SET_DISABLE_SPEC, Conv::Cdecl, &[0]);
                }
            }
            7 => {
                let v = 1_u32.wrapping_sub(get(h, CH_GET_DISABLE_SPEC));
                h.call_sym(CH_SET_DISABLE_SPEC, Conv::Cdecl, &[v]);
                if get(h, CH_GET_DISABLE_SPEC) != 0 {
                    h.call_sym(CH_SET_PARA_EDIT, Conv::Cdecl, &[0]);
                    h.call_sym(CH_SET_FOURCE_SPEC, Conv::Cdecl, &[0]);
                }
            }
            8 if get(h, CH_GET_DISABLE_SPEC) == 0 => {
                let v = 1_u32.wrapping_sub(get(h, CH_GET_PARA_EDIT));
                h.call_sym(CH_SET_PARA_EDIT, Conv::Cdecl, &[v]);
            }
            _ => {}
        }
    }

    fn spec_edit_build(h: &mut impl Host, window: u32) {
        let t = h.rebase(TEXT_SPECULAR);
        h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[window, t]);
        for (id, text, x, y) in [
            (6, TEXT_ALLON, 0, 0),
            (7, TEXT_ALLOFF, 0x38, 0),
            (8, TEXT_EDIT, 0x38, 0xa),
        ] {
            let t = h.rebase(text);
            h.call_sym(
                GU_WINDOW_ADD_BUTTON,
                Conv::Cdecl,
                &[window, id, t, x, y, 0x36, 9],
            );
        }
        for id in [6, 7, 8] {
            h.call_sym(GU_BUTTON_SET_TOGGLE, Conv::Cdecl, &[window, id, 1]);
        }
        let fmt = h.rebase(FMT_D);
        // (spin, ids, format, value, minimum, maximum, y, label)
        let spins = [
            (
                SPIN_COEF,
                [0xd, 0xe, 0xf],
                0,
                0x14,
                0,
                0x64,
                0x14,
                TEXT_COEF,
            ),
            (SPIN_SIZE, [0xa, 0xb, 0xc], 0, 0xa, 1, 0x64, 0x1f, TEXT_SIZE),
            (
                SPIN_RED,
                [0x10, 0x11, 0x12],
                fmt,
                0x80,
                0,
                0xff,
                0x2a,
                TEXT_R,
            ),
            (
                SPIN_GREEN,
                [0x13, 0x14, 0x15],
                fmt,
                0x80,
                0,
                0xff,
                0x35,
                TEXT_G,
            ),
            (
                SPIN_BLUE,
                [0x16, 0x17, 0x18],
                fmt,
                0x80,
                0,
                0xff,
                0x40,
                TEXT_B,
            ),
        ];
        for (spin, ids, fmt, value, min, max, y, label) in spins {
            let s = h.rebase(spin);
            spin_init(h, s, ids, fmt, value, min, max);
            let l = h.rebase(label);
            h.call_sym(
                GU_SPIN_ADD,
                Conv::Cdecl,
                &[window, s, 0, y, 0xa, l, 0x2c, 0x28],
            );
        }
    }

    /// A spin's value in tenths: shown as `"%d.%d"` in its value control
    /// and handed to `set` as an `f32`.
    fn spec_tenths(h: &mut impl Host, window: u32, spin: u32, set: u32) {
        let s = h.rebase(spin);
        let v = word(h, s + 0x20).cast_signed();
        let id = word(h, s + 0x18);
        let f = h.rebase(FMT_TENTHS);
        let (q, r) = (
            v.wrapping_div(10).cast_unsigned(),
            v.wrapping_rem(10).cast_unsigned(),
        );
        h.call_sym(GU_CONTROL_SET_TEXTF, Conv::Cdecl, &[window, id, f, q, r]); // call width: 5
        let x = x87::store(x87::div(
            x87::load(int_f32(v.cast_unsigned())),
            qword(h, C_10),
        ));
        h.call_sym(set, Conv::Cdecl, &[x]);
    }

    fn spec_edit_show(h: &mut impl Host, window: u32) {
        for (id, f) in [
            (6, CH_GET_FOURCE_SPEC),
            (7, CH_GET_DISABLE_SPEC),
            (8, CH_GET_PARA_EDIT),
        ] {
            let v = get(h, f);
            h.call_sym(GU_BUTTON_SET_TOGGLE_VALUE, Conv::Cdecl, &[window, id, v]);
        }
        for id in (0xa..0x19).chain([u32::MAX]) {
            let off = u32::from(get(h, CH_GET_PARA_EDIT) == 0);
            h.call_sym(GU_CONTROL_SET_DISABLE, Conv::Cdecl, &[window, id, off]);
        }
        spec_tenths(h, window, SPIN_SIZE, CH_SET_SPEC_SIZE);
        spec_tenths(h, window, SPIN_COEF, CH_SET_SPEC_COEFFICIENT);
        let [r, g, b] = [SPIN_RED, SPIN_GREEN, SPIN_BLUE].map(|s| word(h, h.rebase(s) + 0x20));
        h.call_sym(CH_SET_SPEC_COLOR, Conv::Cdecl, &[r, g, b, 0x80]);
    }

    pub const CAPTURE_SELECT: u32 = 0x0132_5914;
    pub const CAPTURE_GUI: u32 = 0x0132_5915;
    pub const CAPTURE_MODE: u32 = 0x00c4_fff8;
    pub const CAPTURE_FILTER: u32 = 0x00c4_fff9;
    pub const SG_CAPTURE_SET_FILTER: u32 = 0x0083_a650;
    pub const SG_CAPTURE_SET_GUI: u32 = 0x0083_a660;
    pub const SG_CAPTURE_SET_MODE: u32 = 0x0083_a670;
    pub const SG_CAPTURE_SET_SELECT: u32 = 0x0083_a680;

    fn sbyte(h: &impl Mem, sym: u32) -> u32 {
        i32::from(h.read::<i8>(h.rebase(sym))).cast_unsigned()
    }

    /// `FUN_00853420(window)` (`cdecl`, tail-jumps on) ->
    /// `Sg_CaptureSetGui`'s `eax`: the capture window's controls from its
    /// settings (select 0x4d2, GUI 4, filter 5; controls -2 and 1..5
    /// enabled only with select on; the mode's radio buttons 1..3), then
    /// the settings handed to the capture (filter, select, mode, GUI).
    pub fn capture_window_update(h: &mut impl Host, window: u32) -> u32 {
        for (id, at) in [
            (0x4d2, CAPTURE_SELECT),
            (4, CAPTURE_GUI),
            (5, CAPTURE_FILTER),
        ] {
            let v = sbyte(h, at);
            h.call_sym(GU_BUTTON_SET_TOGGLE_VALUE, Conv::Cdecl, &[window, id, v]);
        }
        let off = u32::from(h.read::<u8>(h.rebase(CAPTURE_SELECT)) == 0);
        for id in [0xffff_fffe, 1, 2, 3, 4, 5] {
            h.call_sym(GU_CONTROL_SET_DISABLE, Conv::Cdecl, &[window, id, off]);
        }
        for id in 1..4 {
            let on = u32::from(id == sbyte(h, CAPTURE_MODE).wrapping_add(1));
            h.call_sym(GU_BUTTON_SET_TOGGLE_VALUE, Conv::Cdecl, &[window, id, on]);
        }
        for (f, at) in [
            (SG_CAPTURE_SET_FILTER, CAPTURE_FILTER),
            (SG_CAPTURE_SET_SELECT, CAPTURE_SELECT),
            (SG_CAPTURE_SET_MODE, CAPTURE_MODE),
        ] {
            let v = sbyte(h, at);
            h.call_sym(f, Conv::Cdecl, &[v]);
        }
        let v = sbyte(h, CAPTURE_GUI);
        h.call_sym(SG_CAPTURE_SET_GUI, Conv::Cdecl, &[v])
    }

    pub const SG_CAPTURE_ENABLE: u32 = 0x0083_a470;
    pub const GU_BUTTON_GET_TOGGLE_VALUE: u32 = 0x0084_9330;
    pub const GU_CONTROL_SET_LEFT_TEXT: u32 = 0x0084_a6c0;
    pub const CAPTURE_WINDOW_UPDATE: u32 = 0x0085_3420;
    /// The byte 0x8003 clears (`0x01325916`).
    pub const CAPTURE_WINDOW_FLAG: u32 = 0x0132_5916;
    /// `"Enable"`, `"DoIt"`, `" "`, `"512x416"`, `"LowReso(^^;"`,
    /// `"Filter"`, `"with GUI"`, `"Capture"`.
    pub const TEXT_ENABLE: u32 = 0x00b5_b64c;
    pub const TEXT_DOIT: u32 = 0x00b5_b654;
    pub const TEXT_SPACE: u32 = 0x00b2_0b30;
    pub const TEXT_512: u32 = 0x00b5_b65c;
    pub const TEXT_LOW_RESO: u32 = 0x00b5_b664;
    pub const TEXT_FILTER: u32 = 0x00b5_b670;
    pub const TEXT_WITH_GUI: u32 = 0x00b5_b678;
    pub const TEXT_CAPTURE: u32 = 0x00b5_b29c;
    /// The capture window's "do it" button.
    pub const CAPTURE_DO_IT: u32 = 1000;

    /// `SpriteFrameRegion_Load(window, msg, button)` (`cdecl`; Fahrenheit's
    /// Switch name, the PC build's capture window procedure) -> 0: 0x8001
    /// builds it (Enable 0x4d2, `DoIt` 1000, the radio buttons 3, 1 and
    /// toggles 5, 4 with their labels; title "Capture") and shows the
    /// settings (`FUN_00853420`); 0x8003 clears `0x01325916`; 0x8006 on
    /// `DoIt` captures (`Sg_CaptureEnable`); 0x8007 sets select (0x4d2,
    /// from its toggle), the mode (1: 0, 3: 2), GUI (4, from its toggle)
    /// or flips the filter (5), and shows them.
    pub fn capture_window_proc(h: &mut impl Host, window: u32, msg: u32, button: u32) -> u32 {
        match msg {
            0x8001 => capture_window_build(h, window),
            0x8003 => {
                let at = h.rebase(CAPTURE_WINDOW_FLAG);
                h.write(at, 0_u8);
            }
            0x8006 if button == CAPTURE_DO_IT => {
                h.call_sym(SG_CAPTURE_ENABLE, Conv::Cdecl, &[]);
            }
            0x8007 => {
                match button {
                    LIGHT_ON_BUTTON => {
                        let v = toggled(h, window, LIGHT_ON_BUTTON);
                        set_byte(h, CAPTURE_SELECT, v);
                    }
                    1 => set_byte(h, CAPTURE_MODE, 0),
                    3 => set_byte(h, CAPTURE_MODE, 2),
                    4 => {
                        let v = toggled(h, window, 4);
                        set_byte(h, CAPTURE_GUI, v);
                    }
                    5 => {
                        let v = h.read::<u8>(h.rebase(CAPTURE_FILTER)).wrapping_sub(1) & 1;
                        set_byte(h, CAPTURE_FILTER, v);
                    }
                    _ => return 0,
                }
                h.call_sym(CAPTURE_WINDOW_UPDATE, Conv::Cdecl, &[window]);
            }
            _ => {}
        }
        0
    }

    fn set_byte(h: &mut impl Mem, sym: u32, v: u8) {
        let at = h.rebase(sym);
        h.write(at, v);
    }

    /// The toggle's value flipped (`1 - al`).
    fn toggled(h: &mut impl Host, window: u32, id: u32) -> u8 {
        let v = h.call_sym(GU_BUTTON_GET_TOGGLE_VALUE, Conv::Cdecl, &[window, id]);
        1_u8.wrapping_sub(v as u8)
    }

    fn capture_window_build(h: &mut impl Host, window: u32) {
        let buttons = [
            (LIGHT_ON_BUTTON, TEXT_ENABLE, 0, 0, 0x41),
            (CAPTURE_DO_IT, TEXT_DOIT, 0x46, 0, 0x28),
            (3, TEXT_SPACE, 0, 0xb, 0xc),
            (1, TEXT_SPACE, 0, 0x16, 0xc),
            (5, TEXT_SPACE, 0, 0x21, 0xc),
            (4, TEXT_SPACE, 0, 0x2c, 0xc),
        ];
        for (id, text, x, y, w) in buttons {
            let t = h.rebase(text);
            h.call_sym(
                GU_WINDOW_ADD_BUTTON,
                Conv::Cdecl,
                &[window, id, t, x, y, w, 0xa],
            );
        }
        for (text, y, w) in [
            (TEXT_512, 0xb, 0x64),
            (TEXT_LOW_RESO, 0x16, 0x64),
            (TEXT_FILTER, 0x21, 0x5a),
            (TEXT_WITH_GUI, 0x2c, 0x5a),
        ] {
            let t = h.rebase(text);
            h.call_sym(
                GU_WINDOW_ADD_STATIC,
                Conv::Cdecl,
                &[window, 0xffff_fffe, t, 0xc, y, w, 0xa],
            );
        }
        for id in [LIGHT_ON_BUTTON, 1, 2, 3, 4, 5] {
            h.call_sym(GU_BUTTON_SET_TOGGLE, Conv::Cdecl, &[window, id, 1]);
        }
        h.call_sym(
            GU_CONTROL_SET_LEFT_TEXT,
            Conv::Cdecl,
            &[window, 0xffff_fffe, 1],
        );
        let t = h.rebase(TEXT_CAPTURE);
        h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[window, t]);
        h.call_sym(CAPTURE_WINDOW_UPDATE, Conv::Cdecl, &[window]);
        let at = h.rebase(CAPTURE_WINDOW_FLAG);
        h.write(at, 0_u8);
    }

    pub const CH_CHECK_HANDLE: u32 = 0x0082_61b0;
    pub const GU_PRINTF: u32 = 0x0084_8880;
    pub const GU_LOCATE: u32 = 0x0084_8840;
    pub const GU_PRINT_SET_SIZE_ONLY: u32 = 0x0084_87d0;
    pub const GU_PRINT_SET_ITALIC: u32 = 0x0084_8760;
    pub const GU_PRINT_SET_BOLD: u32 = 0x0084_8720;
    pub const GU_SET_HSCALE: u32 = 0x0084_8b30;
    pub const GU_LOG_PRINTF: u32 = 0x0085_3e40;
    pub const GU_LOG_Y: u32 = 0x0132_5920;
    pub const MOTION_NAME: u32 = 0x0083_71f0;
    pub const CH_ID_GET_MAP: u32 = 0x0083_d840;
    pub const CH_ID_GET_MAP_B: u32 = 0x0083_d860;
    pub const CH_ID_GET_WATER: u32 = 0x0083_d920;
    pub const CH_ID_GET_ENCOUNT: u32 = 0x0083_d820;
    pub const CH_ID_GET_DICTIONARY: u32 = 0x0083_d7a0;
    pub const CH_ID_GET_EFFECT: u32 = 0x0083_d7c0;
    pub const CH_ID_GET_SOUND: u32 = 0x0083_d900;
    pub const CH_ID_GET_SHADE: u32 = 0x0083_d880;
    pub const CH_ID_GET_SHADE0: u32 = 0x0083_d8a0;
    pub const CH_ID_GET_SHADE1: u32 = 0x0083_d8c0;
    pub const CH_ID_GET_SHADE2: u32 = 0x0083_d8e0;
    pub const GU_WINDOW_GET_LEFT: u32 = 0x0084_afb0;
    pub const GU_WINDOW_GET_X: u32 = 0x0084_afc0;
    pub const GU_WINDOW_GET_WIDTH: u32 = 0x0084_afa0;
    pub const GU_SELECTOR_ALLOCATE: u32 = 0x0084_9bb0;
    pub const GU_SELECTOR_SET_CALLBACK: u32 = 0x0084_9fc0;
    pub const GU_SELECTOR_OPEN_XYWH: u32 = 0x0084_9dc0;
    pub const GU_MOUSE_SET_WINDOW: u32 = 0x0084_9b90;
    pub const GU_CHR_SELECTOR_FILL: u32 = 0x0085_3dc0;
    /// The character selector's callback (`FUN_00853d40`).
    pub const CHR_SELECTOR_PICK: u32 = 0x0085_3d40;
    pub const GU_CHR_SELECTOR: u32 = 0x0132_5918;
    pub const GU_CHR_SELECTOR_WINDOW: u32 = 0x0132_591c;
    /// `"CHRINFO"`, `"click here"`, `"CHRINFO:%s\n"`.
    pub const TEXT_CHRINFO: u32 = 0x00b5_b684;
    pub const TEXT_CLICK_HERE: u32 = 0x00b5_b68c;
    pub const FMT_CHRINFO: u32 = 0x00b5_b698;
    /// The info lines' formats, in order (`0x00b5b6a4` .. `0x00b5b860`).
    pub const FMT_POS: u32 = 0x00b5_b6a4;
    pub const FMT_OFFSET: u32 = 0x00b5_b6c0;
    pub const FMT_ROT: u32 = 0x00b5_b6dc;
    pub const FMT_HIDE: u32 = 0x00b5_b6f8;
    /// The hide flags (`+0x180`): bit 0 `" Ev"`, 1 `" Eff"`, 4 `" Btl"`,
    /// 3 `" Disp"`, 5 `" Z"`, appended in this order.
    pub const HIDE_FLAGS: [(u8, u32); 5] = [
        (1, 0x00b5_b704),
        (2, 0x00b5_b708),
        (0x10, 0x00b5_b710),
        (8, 0x00b5_b718),
        (0x20, 0x00b5_b720),
    ];
    pub const FMT_CAMERA: u32 = 0x00b5_b724;
    pub const FMT_MOTION_NAME: u32 = 0x00b5_b740;
    pub const FMT_MOTION_ID: u32 = 0x00b5_b74c;
    pub const FMT_FRAME: u32 = 0x00b5_b758;
    pub const FMT_SPEED: u32 = 0x00b5_b770;
    pub const FMT_NEXT_HOKAN: u32 = 0x00b5_b78c;
    pub const FMT_MAP: u32 = 0x00b5_b79c;
    pub const FMT_IDS: u32 = 0x00b5_b7c0;
    pub const FMT_SHADE: u32 = 0x00b5_b7e4;
    pub const FMT_HIT: u32 = 0x00b5_b808;
    pub const FMT_SHADE_RGBA: u32 = 0x00b5_b824;
    pub const FMT_EFFECT_RGB: u32 = 0x00b5_b844;
    pub const FMT_ID_SHADE: u32 = 0x00b5_b860;

    /// The two words of an `f64` vararg.
    fn dwords(x: f64) -> [u32; 2] {
        let b = x.to_bits();
        [b as u32, (b >> 32) as u32]
    }

    /// `Gu_LogPrintf(fmt, ...)` with `words`, then the `f32`s at `fields`
    /// of `chr` as doubles.
    fn log(h: &mut impl Host, fmt: u32, words: &[u32], chr: u32, fields: &[u32]) {
        let mut a = vec![h.rebase(fmt)];
        a.extend_from_slice(words);
        for &o in fields {
            a.extend(dwords(x87::load(word(h, chr + o))));
        }
        h.call_sym(GU_LOG_PRINTF, Conv::Cdecl, &a); // call width: 9
    }

    /// An angle in radians (`f32`) to degrees as the original rounds it.
    fn degrees(h: &impl Mem, rad: u32) -> f64 {
        let d = x87::div(x87::mul(x87::load(rad), qword(h, C_180)), qword(h, C_PI));
        x87::load(x87::store(d))
    }

    fn i16_at(h: &impl Mem, at: u32) -> u32 {
        i32::from(h.read::<i16>(at)).cast_unsigned()
    }

    /// `FUN_008537d0(window, msg)` (`cdecl`; the text buffer at `buf`, the
    /// original's `ebp - 0x104`, 0x100 bytes) -> 0: the character info
    /// window. Its character (the window's argument) is dropped when
    /// `Ch_CheckHandle` refuses it. 0x8001 titles it and takes the
    /// mouse; 0x8006 opens the character selector (`g_GuChrSelector`,
    /// filled by `0x00853dc0`, picked by `0x00853d40`) beside it; 0x8011
    /// prints the character's state with `Gu_LogPrintf` (title
    /// `"CHRINFO:<name>"`; "click here" without one, nothing when its
    /// byte `+2` is clear).
    pub fn chr_info_window_proc(h: &mut impl Host, buf: u32, window: u32, msg: u32) -> u32 {
        let mut chr = h.call_sym(GU_WINDOW_GET_ARG, Conv::Cdecl, &[window]);
        if h.call_sym(CH_CHECK_HANDLE, Conv::Cdecl, &[chr]) != 0 {
            chr = 0;
            h.call_sym(GU_WINDOW_SET_ARG, Conv::Cdecl, &[window, 0]);
        }
        match msg {
            0x8001 => {
                let t = h.rebase(TEXT_CHRINFO);
                h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[window, t]);
                h.call_sym(GU_MOUSE_SET_WINDOW, Conv::Cdecl, &[window]);
            }
            0x8006 => chr_info_select(h, window),
            0x8011 => {
                let t = h.rebase(TEXT_CHRINFO);
                h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[window, t]);
                if chr == 0 {
                    let t = h.rebase(TEXT_CLICK_HERE);
                    h.call_sym(GU_PRINTF, Conv::Cdecl, &[t]);
                } else if h.read::<u8>(chr + 2) != 0 {
                    chr_info_print(h, buf, window, chr);
                }
            }
            _ => {}
        }
        0
    }

    fn chr_info_select(h: &mut impl Host, window: u32) {
        let mut left = h.call_sym(GU_WINDOW_GET_LEFT, Conv::Cdecl, &[window]);
        let x = h.call_sym(GU_WINDOW_GET_X, Conv::Cdecl, &[window]);
        if left.cast_signed() > 0x46 {
            left = left.wrapping_sub(0x46);
        } else {
            let w = h.call_sym(GU_WINDOW_GET_WIDTH, Conv::Cdecl, &[window]);
            left = left.wrapping_add(w);
        }
        let sel = h.call_sym(GU_SELECTOR_ALLOCATE, Conv::Cdecl, &[]);
        let cell = h.rebase(GU_CHR_SELECTOR);
        h.write(cell, sel);
        if h.call_sym(GU_CHR_SELECTOR_FILL, Conv::Cdecl, &[]) != 0 {
            return;
        }
        let pick = h.rebase(CHR_SELECTOR_PICK);
        let sel = word(h, cell);
        h.call_sym(GU_SELECTOR_SET_CALLBACK, Conv::Cdecl, &[sel, pick]);
        let sel = word(h, cell);
        let at = h.rebase(GU_CHR_SELECTOR_WINDOW);
        h.write(at, window);
        h.call_sym(
            GU_SELECTOR_OPEN_XYWH,
            Conv::Cdecl,
            &[sel, window, LIGHT_ON_BUTTON, left, x, 0x46, 0x64],
        );
    }

    fn log_y_add(h: &mut impl Mem, n: u32) {
        let at = h.rebase(GU_LOG_Y);
        let y = word(h, at).wrapping_add(n);
        h.write(at, y);
    }

    /// Appends the C string at `text` to the one at `buf`.
    fn str_append(h: &mut impl Mem, buf: u32, text: u32) {
        let mut end = buf;
        while h.read::<u8>(end) != 0 {
            end += 1;
        }
        let mut i = 0;
        loop {
            let c: u8 = h.read(text + i);
            h.write(end + i, c);
            if c == 0 {
                break;
            }
            i += 1;
        }
    }

    #[allow(clippy::too_many_lines)] // one line after another
    fn chr_info_print(h: &mut impl Host, buf: u32, window: u32, chr: u32) {
        h.call_sym(GU_PRINT_SET_SIZE_ONLY, Conv::Cdecl, &[0xc0]);
        h.call_sym(GU_PRINT_SET_ITALIC, Conv::Cdecl, &[1]);
        h.call_sym(GU_PRINT_SET_BOLD, Conv::Cdecl, &[1]);
        let sprintf = word(h, h.rebase(SPRINTF));
        let (f, name) = (h.rebase(FMT_CHRINFO), word(h, chr + 4));
        let at = h.rebase(GU_LOG_Y);
        h.write(at, 0_u32);
        h.call(sprintf, Conv::Cdecl, &[buf, f, name]);
        h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[window, buf]);
        log(h, FMT_POS, &[], chr, &[0xc, 0x10, 0x14]);
        log(h, FMT_OFFSET, &[], chr, &[0x6c, 0x70, 0x74]);
        let dir = degrees(h, word(h, chr + 0x168));
        let rot = degrees(h, word(h, chr + 0x158));
        let [r0, r1] = dwords(rot);
        let [d0, d1] = dwords(dir);
        log(h, FMT_ROT, &[r0, r1, d0, d1], chr, &[0x154]);
        log_y_add(h, 2);
        let hide = h.read::<i8>(chr + 0x180);
        let f = h.rebase(FMT_HIDE);
        h.call(
            sprintf,
            Conv::Cdecl,
            &[buf, f, i32::from(hide).cast_unsigned()],
        );
        for (bit, text) in HIDE_FLAGS {
            if h.read::<u8>(chr + 0x180) & bit != 0 {
                let t = h.rebase(text);
                str_append(h, buf, t);
            }
        }
        h.call_sym(GU_LOG_PRINTF, Conv::Cdecl, &[buf]);
        let ten = qword(h, C_10);
        let len = x87::div(x87::load(word(h, chr + 0x174)), ten);
        let clip = x87::div(x87::load(word(h, chr + 0x178)), ten);
        let [c0, c1] = dwords(clip);
        let [l0, l1] = dwords(len);
        log(h, FMT_CAMERA, &[c0, c1, l0, l1], chr, &[]);
        let motion = word(h, chr + 0x72c);
        log_y_add(h, 2);
        let name = h.call_sym(MOTION_NAME, Conv::Cdecl, &[motion]);
        if name == 0 {
            let motion = word(h, chr + 0x72c);
            log(h, FMT_MOTION_ID, &[motion], chr, &[]);
        } else {
            log(h, FMT_MOTION_NAME, &[name], chr, &[]);
        }
        let frame = word(h, chr + 0x744);
        let (from, to) = (word(h, chr + 0x718), word(h, chr + 0x71c));
        let whole = (frame.cast_signed() >> 8).cast_unsigned();
        log(h, FMT_FRAME, &[whole, frame & 0xff, from, to], chr, &[]);
        let speed = [0x754, 0x724, 0x752].map(|o| i16_at(h, chr + o));
        log(h, FMT_SPEED, &speed, chr, &[]);
        let next = i16_at(h, chr + 0x73c);
        log(h, FMT_NEXT_HOKAN, &[next], chr, &[]);
        log_y_add(h, 2);
        let map = h.call_sym(CH_ID_GET_MAP, Conv::Cdecl, &[chr]);
        let map_b = h.call_sym(CH_ID_GET_MAP_B, Conv::Cdecl, &[chr]);
        let id = word(h, chr + 0x828) & 0x1_ffff;
        let num = i16_at(h, chr + 0x824);
        log(h, FMT_MAP, &[num, id, map_b, map], chr, &[]);
        let mut ids = [
            CH_ID_GET_WATER,
            CH_ID_GET_ENCOUNT,
            CH_ID_GET_DICTIONARY,
            CH_ID_GET_EFFECT,
            CH_ID_GET_SOUND,
        ]
        .map(|f| h.call_sym(f, Conv::Cdecl, &[chr]));
        ids.reverse();
        log(h, FMT_IDS, &ids, chr, &[]);
        let mut shades = [
            CH_ID_GET_SHADE2,
            CH_ID_GET_SHADE1,
            CH_ID_GET_SHADE0,
            CH_ID_GET_SHADE,
        ]
        .map(|f| h.call_sym(f, Conv::Cdecl, &[chr]));
        shades.reverse();
        log(h, FMT_SHADE, &shades, chr, &[0x16c]);
        log_y_add(h, 3);
        let bits = word(h, chr + 0x194);
        log(h, FMT_HIT, &[!(bits >> 8) & 1, (bits >> 30) & 1], chr, &[]);
        log(h, FMT_SHADE_RGBA, &[], chr, &[0x330, 0x33c, 0x348, 0x354]);
        log(h, FMT_EFFECT_RGB, &[], chr, &[0xbc, 0xc0, 0xc4]);
        let bits = word(h, chr + 0x194);
        log(h, FMT_ID_SHADE, &[(bits >> 0x12) & 1], chr, &[0x36c, 0x360]);
        h.call_sym(GU_SET_HSCALE, Conv::Cdecl, &[0x200]);
        h.call_sym(GU_PRINT_SET_SIZE_ONLY, Conv::Cdecl, &[0x100]);
    }

    pub const GU_PRINT_SET_COLOR: u32 = 0x0084_8740;
    pub const GU_PRINT_SET_X: u32 = 0x0084_8820;
    pub const GU_PRINT_SET_XY: u32 = 0x0084_8840;
    pub const GU_PRINT_SET_Y: u32 = 0x0084_8860;
    pub const GU_PRINT_GET_WIDTH: u32 = 0x0084_8d60;
    pub const GU_WRAP_ANGLE: u32 = 0x0085_4200;
    /// The clock's date and time words (`0x023cbc1c` weekday, `+4` day,
    /// `+8` month, `+0xc` seconds, `+0x10` minutes, `+0x14` hours) and the
    /// weekday names (pointer table `0x00c4fffc`).
    pub const CLOCK_WEEKDAY: u32 = 0x023c_bc1c;
    pub const CLOCK_DAY: u32 = 0x023c_bc20;
    pub const CLOCK_MONTH: u32 = 0x023c_bc24;
    pub const CLOCK_SECOND: u32 = 0x023c_bc28;
    pub const CLOCK_MINUTE: u32 = 0x023c_bc2c;
    pub const CLOCK_HOUR: u32 = 0x023c_bc30;
    pub const WEEKDAY_NAMES: u32 = 0x00c4_fffc;
    /// `"12"`, `"6"`, `"9"`, `"3"`, `"%d/%d %s"`, `"%02d:%02d:%02d"`.
    pub const TEXT_12: u32 = 0x00b4_1b34;
    pub const TEXT_6: u32 = 0x00b8_f810;
    pub const TEXT_9: u32 = 0x00b8_f818;
    pub const TEXT_3: u32 = 0x00b7_c99c;
    pub const FMT_DATE: u32 = 0x00b5_b87c;
    pub const FMT_TIME: u32 = 0x00b5_b888;
    /// `f64` constants: 90; the hands' lengths -20/-40 (seconds and
    /// minutes), -15/-30 (hours).
    pub const C_90: u32 = 0x00b5_8cd0;
    pub const C_M20: u32 = 0x00b5_b898;
    pub const C_M40: u32 = 0x00b5_b8a8;
    pub const C_M15: u32 = 0x00b5_01c8;
    pub const C_M30: u32 = 0x00b5_b8a0;
    /// The clock face's centre.
    pub const CLOCK_X: u32 = 0x2a;
    pub const CLOCK_Y: u32 = 0x18;

    fn global(h: &impl Mem, sym: u32) -> u32 {
        word(h, h.rebase(sym))
    }

    /// A hand at `deg` degrees from 12 (`(deg - 90)` in radians, rounded
    /// to `f32` and wrapped), `(dy, dx)` long: its end.
    fn clock_hand(h: &mut impl Host, deg: u32, dy: u32, dx: u32) -> (u32, u32) {
        let d = x87::sub(x87::load(int_f32(deg)), qword(h, C_90));
        let r = x87::store(x87::div(x87::mul(d, qword(h, C_PI)), qword(h, C_180)));
        let w = h.call_sym_st0(GU_WRAP_ANGLE, Conv::Cdecl, &[r]).to_bits();
        let (sin, cos) = (h.rebase(CI_SIN), h.rebase(CI_COS));
        let s = h.call_intrinsic_f64(sin, x87::load(w));
        let y = CLOCK_Y.wrapping_sub(x87::ftol(h, x87::mul(s, qword(h, dy))));
        let c = h.call_intrinsic_f64(cos, x87::load(w));
        let x = CLOCK_X.wrapping_sub(x87::ftol(h, x87::mul(c, qword(h, dx))));
        (x, y)
    }

    /// `FUN_00853ec0(window, msg)` (`cdecl`; the text buffer at `buf`, the
    /// original's `ebp - 0x24`, 0x20 bytes) -> 0: the debug clock window.
    /// On 0x8011: the face's 12/6/9/3, the date (`"%d/%d %s"`, centred
    /// on x 42, y 30), the time as the title, and the hands from (42,
    /// 24): seconds (black, 6 degrees a second), minutes (blue) and hours
    /// (red), from the seconds of the day `s + 60 (m + 60 h)`.
    pub fn clock_window_proc(h: &mut impl Host, buf: u32, window: u32, msg: u32) -> u32 {
        let hour = global(h, CLOCK_HOUR);
        let minute = global(h, CLOCK_MINUTE).wrapping_add(hour.wrapping_mul(60));
        let secs = global(h, CLOCK_SECOND).wrapping_add(minute.wrapping_mul(60));
        if msg != 0x8011 {
            return 0;
        }
        h.call_sym(GU_PRINT_SET_ITALIC, Conv::Cdecl, &[1]);
        h.call_sym(GU_PRINT_SET_BOLD, Conv::Cdecl, &[1]);
        h.call_sym(GU_PRINT_SET_COLOR, Conv::Cdecl, &[0, 0, 0x80, 0]);
        h.call_sym(GU_PRINT_SET_X, Conv::Cdecl, &[0x22]);
        let face = [
            (TEXT_12, None),
            (TEXT_6, Some((0x26, 0x28))),
            (TEXT_9, Some((0, 0x14))),
        ];
        for (text, at) in face {
            if let Some((x, y)) = at {
                h.call_sym(GU_PRINT_SET_XY, Conv::Cdecl, &[x, y]);
            }
            let t = h.rebase(text);
            h.call_sym(GU_PRINTF, Conv::Cdecl, &[t]);
        }
        h.call_sym(GU_PRINT_SET_X, Conv::Cdecl, &[0x50]);
        let t = h.rebase(TEXT_3);
        h.call_sym(GU_PRINTF, Conv::Cdecl, &[t]);
        h.call_sym(GU_PRINT_SET_COLOR, Conv::Cdecl, &[0, 0x40, 0, 0]);
        let sprintf = word(h, h.rebase(SPRINTF));
        let day = word(
            h,
            h.rebase(WEEKDAY_NAMES)
                .wrapping_add(global(h, CLOCK_WEEKDAY).wrapping_mul(4)),
        );
        let (month, date) = (global(h, CLOCK_MONTH), global(h, CLOCK_DAY));
        let f = h.rebase(FMT_DATE);
        h.call(sprintf, Conv::Cdecl, &[buf, f, month, date, day]);
        let w = h
            .call_sym(GU_PRINT_GET_WIDTH, Conv::Cdecl, &[buf])
            .cast_signed();
        let x = (CLOCK_X.cast_signed().wrapping_sub(w / 2)).cast_unsigned();
        h.call_sym(GU_PRINT_SET_X, Conv::Cdecl, &[x]);
        h.call_sym(GU_PRINT_SET_Y, Conv::Cdecl, &[0x1e]);
        h.call_sym(GU_PRINTF, Conv::Cdecl, &[buf]);
        let hms = [CLOCK_HOUR, CLOCK_MINUTE, CLOCK_SECOND].map(|s| global(h, s));
        let f = h.rebase(FMT_TIME);
        h.call(sprintf, Conv::Cdecl, &[buf, f, hms[0], hms[1], hms[2]]);
        h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[window, buf]);
        h.call_sym(GU_PACKET_BEGIN_TAG, Conv::Cdecl, &[window]);
        h.call_sym(GU_SET_TEST, Conv::Cdecl, &[1, 5, 0x7f, 0, 0, 0, 0, 1]);
        h.call_sym(GU_SET_PRIM, Conv::Cdecl, &[1]);
        let s = secs.cast_signed();
        let hands = [
            (secs.wrapping_mul(6), [0, 0, 0, 0xff], C_M20, C_M40),
            ((s / 10).cast_unsigned(), [0, 0, 0x80, 0xff], C_M20, C_M40),
            ((s / 120).cast_unsigned(), [0x80, 0, 0, 0xff], C_M15, C_M30),
        ];
        for (deg, colour, dy, dx) in hands {
            rgbaq(h, colour);
            xyz(h, CLOCK_X, CLOCK_Y);
            let (x, y) = clock_hand(h, deg, dy, dx);
            xyz(h, x, y);
        }
        h.call_sym(GU_PACKET_END_TAG, Conv::Cdecl, &[]);
        0
    }

    pub const SG_ACC_REQUEST_CURRENT: u32 = 0x0082_bdf0;
    pub const SG_ACC_GET_ALPHA: u32 = 0x0082_bcc0;
    pub const SG_ACC_SET_ALPHA: u32 = 0x0082_bd10;
    pub const SG_GET_BLUR: u32 = 0x0082_cd20;
    pub const SG_BLUR: u32 = 0x0082_bee0;
    /// `"AccBuf"`, `"Current"`, `""`, `"0x%x"`, `"Blur"`.
    pub const TEXT_ACC_BUF: u32 = 0x00b5_b8c0;
    pub const TEXT_CURRENT: u32 = 0x00b5_b8c8;
    pub const TEXT_EMPTY: u32 = 0x00b8_fbeb;
    pub const FMT_HEX: u32 = 0x00b5_b8d0;
    pub const TEXT_BLUR: u32 = 0x00b5_b3d0;

    /// A value window's step: `get()` down (button 0) or up (2) by 1
    /// (0x8005) or 10 (0x8009), clamped to `0..=max`.
    fn value_step(h: &mut impl Host, get: u32, msg: u32, button: u32, max: i32) -> u32 {
        let step = if msg == 0x8005 { 1 } else { 10 };
        let mut v = h.call_sym(get, Conv::Cdecl, &[]).cast_signed();
        match button {
            0 => v = v.wrapping_sub(step),
            2 => v = v.wrapping_add(step),
            _ => {}
        }
        v.clamp(0, max).cast_unsigned()
    }

    /// A value window's row: down 0, the value 1, up 2 at `y`.
    fn value_row(h: &mut impl Host, window: u32, y: u32) {
        let (left, empty, right) = (
            h.rebase(SPEC_LEFT),
            h.rebase(TEXT_EMPTY),
            h.rebase(SPEC_RIGHT),
        );
        h.call_sym(
            GU_WINDOW_ADD_BUTTON,
            Conv::Cdecl,
            &[window, 0, left, 4, y, 0xe, 9],
        );
        h.call_sym(
            GU_WINDOW_ADD_STATIC,
            Conv::Cdecl,
            &[window, 1, empty, 0x11, y, 0x28, 9],
        );
        h.call_sym(
            GU_WINDOW_ADD_BUTTON,
            Conv::Cdecl,
            &[window, 2, right, 0x3e, y, 0xe, 9],
        );
    }

    /// A value window's 0x8011: `get()` as `"0x%x"` in control 1.
    fn value_show(h: &mut impl Host, buf: u32, window: u32, get: u32) {
        let v = h.call_sym(get, Conv::Cdecl, &[]);
        let (f, sprintf) = (h.rebase(FMT_HEX), word(h, h.rebase(SPRINTF)));
        h.call(sprintf, Conv::Cdecl, &[buf, f, v]);
        h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 1, buf]);
    }

    /// `FUN_00854270(window, msg, button)` (`cdecl`; the text buffer at
    /// `buf`, the original's `ebp - 0x18`) -> 0: the `"AccBuf"` window:
    /// the accumulation buffer's alpha (`Sg_AccGetAlpha`) stepped by
    /// 1/10 (0x8005/0x8009, buttons 0 and 2) within 0..0x80
    /// (`Sg_AccSetAlpha(a, 0)`), shown on 0x8011; "Current" (0x4d2) on
    /// 0x8006 asks for the current frame (`Sg_AccRequestCurrent(0)`).
    pub fn acc_buf_window_proc(
        h: &mut impl Host,
        buf: u32,
        window: u32,
        msg: u32,
        button: u32,
    ) -> u32 {
        match msg {
            0x8001 => {
                let t = h.rebase(TEXT_ACC_BUF);
                h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[window, t]);
                let t = h.rebase(TEXT_CURRENT);
                h.call_sym(
                    GU_WINDOW_ADD_BUTTON,
                    Conv::Cdecl,
                    &[window, LIGHT_ON_BUTTON, t, 4, 2, 0x46, 9],
                );
                value_row(h, window, 0xd);
            }
            0x8005 | 0x8009 => {
                let a = value_step(h, SG_ACC_GET_ALPHA, msg, button, 0x80);
                h.call_sym(SG_ACC_SET_ALPHA, Conv::Cdecl, &[a, 0]);
            }
            0x8006 if button == LIGHT_ON_BUTTON => {
                h.call_sym(SG_ACC_REQUEST_CURRENT, Conv::Cdecl, &[0]);
            }
            0x8011 => value_show(h, buf, window, SG_ACC_GET_ALPHA),
            _ => {}
        }
        0
    }

    /// `FUN_00854420(window, msg, button)` (`cdecl`; the text buffer at
    /// `buf`, the original's `ebp - 0x18`) -> 0: the `"Blur"` window: the
    /// blur (`Sg_GetBlur`) stepped by 1/10 (0x8005/0x8009, buttons 0 and
    /// 2) within 0..0x7f (`Sg_Blur`), shown on 0x8011.
    pub fn blur_window_proc(
        h: &mut impl Host,
        buf: u32,
        window: u32,
        msg: u32,
        button: u32,
    ) -> u32 {
        match msg {
            0x8001 => {
                let t = h.rebase(TEXT_BLUR);
                h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[window, t]);
                value_row(h, window, 2);
            }
            0x8005 | 0x8009 => {
                let b = value_step(h, SG_GET_BLUR, msg, button, 0x7f);
                h.call_sym(SG_BLUR, Conv::Cdecl, &[b]);
            }
            0x8011 => value_show(h, buf, window, SG_GET_BLUR),
            _ => {}
        }
        0
    }

    pub const SG_FLASH: u32 = 0x0082_cca0;
    /// The flash window's settings: time (`0x0132593c`, 1..600), red,
    /// green, blue (`0x01325930`.., 0..255).
    pub const FLASH_TIME: u32 = 0x0132_593c;
    pub const FLASH_RGB: [u32; 3] = [0x0132_5930, 0x0132_5934, 0x0132_5938];
    /// `"Flash"`, `"Flash!"`, the row letters `"TRGB"`.
    pub const TEXT_FLASH: u32 = 0x00b5_b3d8;
    pub const TEXT_FLASH_NOW: u32 = 0x00b5_b8b0;
    pub const TEXT_TRGB: u32 = 0x00b5_b8b8;

    /// The flash window's four settings in row order.
    fn flash_cells(h: &impl Mem) -> [u32; 4] {
        [FLASH_TIME, FLASH_RGB[0], FLASH_RGB[1], FLASH_RGB[2]].map(|s| h.rebase(s))
    }

    /// `FUN_00854590(window, msg, button)` (`cdecl`; the text buffer at
    /// `buf`, the original's `ebp - 0x18`) -> 0: the `"Flash"` window:
    /// rows T, R, G, B (down 3k, value 3k + 1, up 3k + 2) stepped by 1
    /// (0x8005, after an unused `Sg_AccGetAlpha`) or 10 (0x8009), time
    /// kept in 1..600 and the colour in 0..255; "Flash!" (0x4d2) on
    /// 0x8007 flashes (`Sg_Flash(t, r, g, b)`); 0x8011 shows them.
    pub fn flash_window_proc(
        h: &mut impl Host,
        buf: u32,
        window: u32,
        msg: u32,
        button: u32,
    ) -> u32 {
        let cells = flash_cells(h);
        match msg {
            0x8001 => flash_window_build(h, buf, window),
            0x8005 | 0x8009 => {
                let step = if msg == 0x8005 {
                    h.call_sym(SG_ACC_GET_ALPHA, Conv::Cdecl, &[]);
                    1
                } else {
                    10
                };
                if button < 0xc && button % 3 != 1 {
                    let at = cells[(button / 3) as usize];
                    let v = word(h, at);
                    let v = if button.is_multiple_of(3) {
                        v.wrapping_sub(step)
                    } else {
                        v.wrapping_add(step)
                    };
                    h.write(at, v);
                }
                for &at in &cells[1..] {
                    let v = word(h, at).cast_signed().clamp(0, 0xff);
                    h.write(at, v);
                }
                let t = word(h, cells[0]).cast_signed().clamp(1, 0x258);
                h.write(cells[0], t);
            }
            0x8007 if button == LIGHT_ON_BUTTON => {
                let [t, r, g, b] = cells.map(|at| word(h, at));
                h.call_sym(SG_FLASH, Conv::Cdecl, &[t, r, g, b]);
            }
            0x8011 => {
                let (f, sprintf) = (h.rebase(FMT_D), word(h, h.rebase(SPRINTF)));
                for (k, at) in (0_u32..).zip(cells) {
                    let v = word(h, at);
                    h.call(sprintf, Conv::Cdecl, &[buf, f, v]);
                    h.call_sym(GU_CONTROL_SET_TEXT, Conv::Cdecl, &[window, 3 * k + 1, buf]);
                }
            }
            _ => {}
        }
        0
    }

    fn flash_window_build(h: &mut impl Host, buf: u32, window: u32) {
        let t = h.rebase(TEXT_FLASH);
        h.call_sym(GU_WINDOW_SET_TITLE, Conv::Cdecl, &[window, t]);
        let t = h.rebase(TEXT_FLASH_NOW);
        h.call_sym(
            GU_WINDOW_ADD_BUTTON,
            Conv::Cdecl,
            &[window, LIGHT_ON_BUTTON, t, 4, 2, 0x52, 9],
        );
        let (letters, left, empty, right) = (
            h.rebase(TEXT_TRGB),
            h.rebase(SPEC_LEFT),
            h.rebase(TEXT_EMPTY),
            h.rebase(SPEC_RIGHT),
        );
        for k in 0..4_u32 {
            let (id, y) = (3 * k, 0xd + 0xb * k);
            let c: u8 = h.read(letters + k);
            h.write(buf, [c, 0]);
            h.call_sym(
                GU_WINDOW_ADD_STATIC,
                Conv::Cdecl,
                &[window, u32::MAX, buf, 4, y, 0xa, 9],
            );
            h.call_sym(
                GU_WINDOW_ADD_BUTTON,
                Conv::Cdecl,
                &[window, id, left, 0xe, y, 0xe, 9],
            );
            h.call_sym(
                GU_WINDOW_ADD_STATIC,
                Conv::Cdecl,
                &[window, id + 1, empty, 0x1b, y, 0x28, 9],
            );
            h.call_sym(
                GU_WINDOW_ADD_BUTTON,
                Conv::Cdecl,
                &[window, id + 2, right, 0x48, y, 0xe, 9],
            );
        }
        let [t, r, g, b] = flash_cells(h);
        for at in [b, g, r] {
            h.write(at, 0xff_u32);
        }
        h.write(t, 0x1e_u32);
    }

    pub const GU_WINDOW_SET_SIZE: u32 = 0x0084_b720;
    pub const GU_WINDOW_SET_FLAG2: u32 = 0x0084_b600;
    /// The help window's text (`g_GuHelpText`, `0x013259a0`) and its
    /// colour bytes (`0x0132599c`..).
    pub const GU_HELP_TEXT: u32 = 0x0132_59a0;
    pub const GU_HELP_COLOUR: u32 = 0x0132_599c;
    /// The tip window's text (`0x01325958`), its colour record (pointer
    /// `0x01325998`, r, g, b at `+5`) and the window it hides
    /// (`0x01325948`).
    pub const GU_TIP_TEXT: u32 = 0x0132_5958;
    pub const GU_TIP_RECORD: u32 = 0x0132_5998;
    pub const GU_TIP_OWNER: u32 = 0x0132_5948;

    /// A text box's 0x8011: sized to `text` (bold, from x 14) and a
    /// square of `colour` (r, g, b bytes) at (2, 1)-(8, 6).
    fn text_box(h: &mut impl Host, window: u32, text: u32, colour: u32) {
        h.call_sym(GU_PRINT_SET_BOLD, Conv::Cdecl, &[1]);
        let w = h.call_sym(GU_PRINT_GET_WIDTH, Conv::Cdecl, &[text]);
        h.call_sym(
            GU_WINDOW_SET_SIZE,
            Conv::Cdecl,
            &[window, w.wrapping_add(0x22), 0xa],
        );
        h.call_sym(GU_PRINT_SET_X, Conv::Cdecl, &[0xe]);
        h.call_sym(GU_PRINTF, Conv::Cdecl, &[text]);
        h.call_sym(GU_PACKET_BEGIN_TAG, Conv::Cdecl, &[window]);
        h.call_sym(GU_SET_TEST, Conv::Cdecl, &[1, 5, 0x7f, 0, 0, 0, 0, 1]);
        let [r, g, b]: [u8; 3] = h.read(colour);
        rgbaq(h, [r.into(), g.into(), b.into(), 0xff]);
        h.call_sym(GU_SET_PRIM, Conv::Cdecl, &[6]);
        xyz(h, 2, 1);
        xyz(h, 8, 6);
        h.call_sym(GU_PACKET_END_TAG, Conv::Cdecl, &[]);
    }

    /// `FUN_00855290(window, msg)` (`cdecl`) -> 0: the help window
    /// (`g_GuHelpText` and its colour, `0x0132599c`) drawn on 0x8011.
    pub fn help_window_proc(h: &mut impl Host, window: u32, msg: u32) -> u32 {
        if msg == 0x8011 {
            let (text, colour) = (h.rebase(GU_HELP_TEXT), h.rebase(GU_HELP_COLOUR));
            text_box(h, window, text, colour);
        }
        0
    }

    /// `FUN_008554b0(window, msg)` (`cdecl`) -> 0: the tip window: on
    /// 0x8011 with a text (`0x01325958`), the window `0x01325948` loses
    /// flag 2 and the text is drawn with the record's colour (`+5`).
    pub fn tip_window_proc(h: &mut impl Host, window: u32, msg: u32) -> u32 {
        let record = word(h, h.rebase(GU_TIP_RECORD));
        let text = h.rebase(GU_TIP_TEXT);
        if msg == 0x8011 && h.read::<u8>(text) != 0 {
            let owner = word(h, h.rebase(GU_TIP_OWNER));
            h.call_sym(GU_WINDOW_SET_FLAG2, Conv::Cdecl, &[owner, 0]);
            text_box(h, window, text, record.wrapping_add(5));
        }
        0
    }

    pub const SG_MEM_USAGE: u32 = 0x0083_c0f0;
    pub const GU_WINDOW_GET_CLIENT_W: u32 = 0x0084_aee0;
    pub const GU_WINDOW_GET_CLIENT_H: u32 = 0x0084_aed0;
    pub const GU_WINDOW_GET_CLIENT_TOP: u32 = 0x0084_af00;
    pub const GU_WINDOW_MOUSE_IN: u32 = 0x0084_9490;
    pub const GU_MOUSE_GET_WINDOW_X: u32 = 0x0084_9ac0;
    pub const GU_MOUSE_GET_X: u32 = 0x0084_9ae0;
    pub const GU_MOUSE_GET_Y: u32 = 0x0084_9af0;
    pub const GU_WINDOW_SET_POS: u32 = 0x0084_b220;
    pub const GU_WINDOW_CLOSE_TREE: u32 = 0x0084_ab80;
    pub const GU_DRAW_RECT_STRIP: u32 = 0x0085_4f60;
    pub const GU_HELP_WINDOW_SHOW: u32 = 0x0085_5360;
    /// `g_GuHelpWindow`, `g_GuMemGraphOpen`, and where the mouse was
    /// (`0x013259e0`, `0x013259e4`).
    pub const GU_HELP_WINDOW: u32 = 0x0132_5950;
    pub const GU_MEM_GRAPH_OPEN: u32 = 0x0132_5954;
    pub const GU_HELP_MOUSE: u32 = 0x0132_59e0;
    /// The memory graph's nine rows (8 bytes: name, r, g, b) and the
    /// help line's format `"%s %dK[%xh] "`.
    pub const MEM_GRAPH_ROWS: u32 = 0x00b5_b930;
    pub const FMT_MEM_ROW: u32 = 0x00b5_b9cc;

    /// `FUN_00855020(window, msg)` (`cdecl`; its frame from the
    /// original's `ebp - 0x80` at `frame`: the client rectangle `+0`
    /// (`i16` x, y, w, h), the mouse x `+8`, the hit flag `+0xc`, the
    /// previous row's end `+0x10`, the height `+0x14`, the nine usage
    /// words `+0x18`, the help text `+0x3c`) -> 0: the memory graph.
    /// 0x8003 closes it (`g_GuMemGraphOpen` 0, the help window closed,
    /// `Sg_SetResource(0x11, 1)`); 0x8011 stacks the nine usages
    /// (`Sg_MemUsage`) as coloured bands over the client height, the
    /// band under the mouse described in the help window (opened by
    /// `0x00855360`, moved to the mouse, closed when no band is hit).
    pub fn mem_graph_window_proc(h: &mut impl Host, frame: u32, window: u32, msg: u32) -> u32 {
        match msg {
            0x8003 => {
                let open = h.rebase(GU_MEM_GRAPH_OPEN);
                h.write(open, 0_u32);
                mem_graph_close_help(h);
                h.call_sym(SG_SET_RESOURCE, Conv::Cdecl, &[0x11, 1]);
            }
            0x8011 => mem_graph_draw(h, frame, window),
            _ => {}
        }
        0
    }

    fn mem_graph_close_help(h: &mut impl Host) {
        let cell = h.rebase(GU_HELP_WINDOW);
        let help = word(h, cell);
        if help != 0 {
            h.call_sym(GU_WINDOW_CLOSE_TREE, Conv::Cdecl, &[help]);
            h.write(cell, 0_u32);
        }
    }

    fn mem_graph_draw(h: &mut impl Host, frame: u32, window: u32) {
        let usage = frame + 0x18;
        let total = h
            .call_sym(SG_MEM_USAGE, Conv::Cdecl, &[usage])
            .cast_signed();
        let half = total / 2;
        h.write(frame, 0_u32);
        let w = h
            .call_sym(GU_WINDOW_GET_CLIENT_W, Conv::Cdecl, &[window])
            .wrapping_sub(2);
        h.write(frame + 4, w as u16);
        let ch = h
            .call_sym(GU_WINDOW_GET_CLIENT_H, Conv::Cdecl, &[window])
            .wrapping_sub(2);
        h.write(frame + 6, ch as u16);
        let mouse_x = if h.call_sym(GU_WINDOW_MOUSE_IN, Conv::Cdecl, &[window]) == 0 {
            u32::MAX
        } else {
            let x = h.call_sym(GU_MOUSE_GET_WINDOW_X, Conv::Cdecl, &[window]);
            let top = h.call_sym(GU_WINDOW_GET_CLIENT_TOP, Conv::Cdecl, &[window]);
            let mx = x.wrapping_sub(top);
            let y = h
                .call_sym(GU_MOUSE_GET_Y, Conv::Cdecl, &[])
                .wrapping_sub(10);
            let x = h.call_sym(GU_MOUSE_GET_X, Conv::Cdecl, &[]);
            let at = h.rebase(GU_HELP_MOUSE);
            h.write(at, [x, y]);
            let help = word(h, h.rebase(GU_HELP_WINDOW));
            if help != 0 {
                h.call_sym(GU_WINDOW_SET_POS, Conv::Cdecl, &[help, x, y]);
            }
            mx
        };
        h.write(frame + 8, mouse_x);
        h.call_sym(GU_PACKET_BEGIN_TAG, Conv::Cdecl, &[window]);
        h.call_sym(GU_SET_PRIM, Conv::Cdecl, &[6]);
        let height = i32::from(ch as i16);
        h.write(frame + 0x14, height);
        let right = i32::from(w as i16).cast_unsigned();
        h.write(frame + 0xc, 0_u32);
        let mut hit = false;
        let (mut cum, mut prev) = (0_i32, 1_i32);
        let rows = h.rebase(MEM_GRAPH_ROWS);
        for i in 0..9 {
            let row = rows + 8 * i;
            let [r, g, b]: [u8; 3] = h.read(row + 4);
            rgbaq(h, [r.into(), g.into(), b.into(), 0xff]);
            let n = word(h, usage + 4 * i).cast_signed();
            cum = cum.wrapping_add(n);
            let y = (cum / 2)
                .wrapping_mul(height)
                .wrapping_div(half)
                .wrapping_add(1);
            let mx = mouse_x.cast_signed();
            if mx >= prev && mx < y {
                let at = h.rebase(GU_HELP_COLOUR);
                h.write(at, [r, g, b]);
                let (f, sprintf) = (h.rebase(FMT_MEM_ROW), word(h, h.rebase(SPRINTF)));
                let name = word(h, row);
                let text = frame + 0x3c;
                h.call(
                    sprintf,
                    Conv::Cdecl,
                    &[text, f, name, (n / 1024).cast_unsigned(), n.cast_unsigned()],
                );
                h.call_sym(GU_HELP_WINDOW_SHOW, Conv::Cdecl, &[text]);
                hit = true;
                h.write(frame + 0xc, 1_u32);
            }
            xyz(h, 0, prev.cast_unsigned());
            xyz(h, right, y.cast_unsigned());
            prev = y;
            h.write(frame + 0x10, prev);
        }
        if !hit {
            mem_graph_close_help(h);
        }
        h.call_sym(GU_SET_PRIM, Conv::Cdecl, &[2]);
        rgbaq(h, [0; 4]);
        h.call_sym(GU_DRAW_RECT_STRIP, Conv::Cdecl, &[frame]);
        h.call_sym(GU_PACKET_END_TAG, Conv::Cdecl, &[]);
        h.call_sym(GU_PRINT_SET_SIZE_ONLY, Conv::Cdecl, &[0x100]);
    }

    /// The debug name lists: the current one (`0x025d5f90`), and per
    /// list the 12-byte entries (`0x025d5f44[]`, flag bit 1 = picked),
    /// their counts (`0x025d5f4c[]`) and the 16-byte points
    /// (`0x025d5f54[]`, x, y, z).
    pub const NAME_LIST: u32 = 0x025d_5f90;
    pub const NAME_LIST_ENTRIES: u32 = 0x025d_5f44;
    pub const NAME_LIST_COUNTS: u32 = 0x025d_5f4c;
    pub const NAME_LIST_POINTS: u32 = 0x025d_5f54;
    pub const NAME_LIST_CLEAR: u32 = 0x0094_2320;
    /// The debug camera handle (`0x025d5ee0`).
    pub const DEBUG_CAMERA: u32 = 0x025d_5ee0;
    pub const MS_CAMERA_SET_RECT: u32 = 0x007b_f940;
    pub const RCUI_PROJECT_TO_INT: u32 = 0x0093_d710;
    /// The debug view matrix (`0x0193c338`, 16 `f32`, row-major).
    pub const DEBUG_VIEW_MATRIX: u32 = 0x0193_c338;
    /// 0.5 and 400 (`f64`).
    pub const C_HALF: u32 = 0x00b9_2220;
    pub const C_400: u32 = 0x00b6_91c8;

    /// The current list's cell of the per-list array `sym`.
    fn list_cell(h: &impl Mem, sym: u32) -> u32 {
        let l = word(h, h.rebase(NAME_LIST));
        h.rebase(sym).wrapping_add(l.wrapping_mul(4))
    }

    /// `FUN_009401c0()` (`cdecl`; its locals from the original's
    /// `ebp - 0x10` at `frame`: the two picked indices, the points, the
    /// entries) -> the picked count (fewer than two), else
    /// `Sg_NameListClear`'s `eax`: a point between the first two picked
    /// ones: the entries and points from the second on moved up one,
    /// the second's point the midpoint, the count one more.
    pub fn name_list_split(h: &mut impl Host, frame: u32) -> u32 {
        let entries = word(h, list_cell(h, NAME_LIST_ENTRIES));
        let points = word(h, list_cell(h, NAME_LIST_POINTS));
        let n = word(h, list_cell(h, NAME_LIST_COUNTS)).cast_signed();
        h.write(frame + 0xc, entries);
        h.write(frame + 8, points);
        let mut found = 0_u32;
        let mut i = 0_i32;
        while i < n {
            if h.read::<u8>(entries.wrapping_add(i.cast_unsigned().wrapping_mul(0xc))) & 2 != 0 {
                h.write(frame + 4 * found, i);
                found += 1;
                if found == 2 {
                    break;
                }
            }
            i += 1;
        }
        if found != 2 {
            return found;
        }
        let [a, b]: [u32; 2] = h.read(frame);
        let mut k = n.cast_unsigned();
        while k.cast_signed() > b.cast_signed() {
            let e = entries.wrapping_add(k.wrapping_mul(0xc));
            let src: [u32; 3] = h.read(e.wrapping_sub(0xc));
            h.write(e, src);
            let p = points.wrapping_add(k.wrapping_mul(0x10));
            let src: [u32; 4] = h.read(p.wrapping_sub(0x10));
            h.write(p, src);
            k = k.wrapping_sub(1);
        }
        let half = qword(h, C_HALF);
        let pa = points.wrapping_add(a.wrapping_mul(0x10));
        let pb = points.wrapping_add(b.wrapping_mul(0x10));
        for o in [0, 4, 8] {
            let s = x87::add(x87::load(word(h, pb + o)), x87::load(word(h, pa + o)));
            h.write(pb + o, x87::store(x87::mul(s, half)));
        }
        let count = list_cell(h, NAME_LIST_COUNTS);
        let c = word(h, count).wrapping_add(1);
        h.write(count, c);
        h.call_sym(NAME_LIST_CLEAR, Conv::Cdecl, &[])
    }

    /// `FUN_00940580()` (`cdecl`; its locals from the original's
    /// `ebp - 0x34` at `frame`: the sums, the count, the two positions
    /// `+0x10` and `+0x20`) -> the last `MsCameraSetRect`'s `eax`: the
    /// debug camera aimed at the current list's centroid, from 400
    /// above it (`f32` sums, each divided by the count).
    pub fn name_list_view_centre(h: &mut impl Host, frame: u32) -> u32 {
        let n = word(h, list_cell(h, NAME_LIST_COUNTS));
        let mut p = word(h, list_cell(h, NAME_LIST_POINTS));
        let mut s = [0_u32; 3]; // x, y, z
        let mut i = n.cast_signed();
        while i > 0 {
            for (k, sum) in s.iter_mut().enumerate() {
                let v = word(h, p + 4 * k as u32);
                *sum = x87::store(x87::add(x87::load(v), x87::load(*sum)));
            }
            p = p.wrapping_add(0x10);
            i -= 1;
        }
        let nf = x87::store(x87::load(int_f32(n)));
        let [x, y, z] = s.map(|v| x87::store(x87::div(x87::load(v), x87::load(nf))));
        let above = x87::store(x87::add(x87::load(y), qword(h, C_400)));
        // (sx, n, sz, sy) as the original leaves them, then the two
        // positions with an unused word between
        h.write(frame, [x, nf, z, y, x, y, z]);
        h.write(frame + 0x20, [x, above, z]);
        let cam = word(h, h.rebase(DEBUG_CAMERA));
        h.call_sym(MS_CAMERA_SET_RECT, Conv::Cdecl, &[cam, 0, frame + 0x10]);
        let cam = word(h, h.rebase(DEBUG_CAMERA));
        h.call_sym(MS_CAMERA_SET_RECT, Conv::Cdecl, &[cam, 1, frame + 0x20])
    }

    /// `FUN_0093fd60(v, out)` (`cdecl`; the matrix at `m`, the original's
    /// `ebp - 0x44`) -> the projection's `1/w` truncated: `v` projected
    /// to integers (`rcuiProjectToInt(out, m, v, 1)`) through the debug
    /// view matrix (`0x0193c338`, transposed into `m`), then moved by
    /// (-0x700, -0x730).
    pub fn debug_project(h: &mut impl Host, m: u32, v: u32, out: u32) -> u32 {
        let g = h.rebase(DEBUG_VIEW_MATRIX);
        for k in 0..16_u32 {
            let x = word(h, g + 4 * k);
            h.write(m + 4 * ((k % 4) * 4 + k / 4), x87::store(x87::load(x)));
        }
        let w = h.call_sym_st0_f64(RCUI_PROJECT_TO_INT, Conv::Cdecl, &[out, m, v, 1]);
        let r = x87::ftol(h, w);
        let x = word(h, out).wrapping_add(0xffff_f900);
        h.write(out, x);
        let y = word(h, out + 4).wrapping_add(0xffff_f8d0);
        h.write(out + 4, y);
        r
    }

    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_LSEEK: u32 = 0x0062_f8e0;
    pub const SCE_READ: u32 = 0x0062_fc00;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    /// The name list file's path (`0x025d60a0`, empty: none) and the
    /// messages `"file open err !!! [%s]\n"`, `"file err !!! [%s]\n"`,
    /// `"%s load\n"`.
    pub const NAME_LIST_PATH: u32 = 0x025d_60a0;
    pub const MSG_OPEN_ERR: u32 = 0x00b6_918c;
    pub const MSG_FILE_ERR: u32 = 0x00b6_91a4;
    pub const MSG_LOADED: u32 = 0x00b6_91b8;

    /// `FUN_009410e0()` (`cdecl`) -> the entries' pointer (nothing done),
    /// else `Sg_NameListClear`'s `eax`: with more than five entries and
    /// at least five not picked, the picked ones (flag bit 1) dropped
    /// from the current list, entries and points moved down.
    pub fn name_list_drop_picked(h: &mut impl Host) -> u32 {
        let entries = word(h, list_cell(h, NAME_LIST_ENTRIES));
        let points = word(h, list_cell(h, NAME_LIST_POINTS));
        let n = word(h, list_cell(h, NAME_LIST_COUNTS)).cast_signed();
        if n <= 5 {
            return entries;
        }
        let mut picked = 0_i32;
        for i in 0..n {
            if h.read::<u8>(entries.wrapping_add(i.cast_unsigned().wrapping_mul(0xc))) & 2 != 0 {
                picked += 1;
            }
        }
        if n.wrapping_sub(picked) < 5 {
            return entries;
        }
        let (mut de, mut dp) = (entries, points);
        for i in 0..n.cast_unsigned() {
            let e = entries.wrapping_add(i.wrapping_mul(0xc));
            if h.read::<u8>(e) & 2 != 0 {
                continue;
            }
            let p = points.wrapping_add(i.wrapping_mul(0x10));
            let ev: [u32; 3] = h.read(e);
            let pv: [u32; 4] = h.read(p);
            h.write(de, ev);
            h.write(dp, pv);
            de = de.wrapping_add(0xc);
            dp = dp.wrapping_add(0x10);
        }
        let count = list_cell(h, NAME_LIST_COUNTS);
        let c = word(h, count).wrapping_sub(picked.cast_unsigned());
        h.write(count, c);
        h.call_sym(NAME_LIST_CLEAR, Conv::Cdecl, &[])
    }

    /// `FUN_00941460()` (`cdecl`; its locals from the original's
    /// `ebp - 0x10` at `frame`: the index, the entry cursor, a scratch
    /// word, the count): with a path (`0x025d60a0`) and the current
    /// list's arrays, the list read from the file: the count, three
    /// unused words, then per point x, y, z and an angle in radians,
    /// each entry `(1, 0, degrees)`; errors and success logged.
    pub fn name_list_load(h: &mut impl Host, frame: u32) {
        let path = h.rebase(NAME_LIST_PATH);
        if h.read::<u8>(path) == 0 {
            return;
        }
        let entries = word(h, list_cell(h, NAME_LIST_ENTRIES));
        if entries == 0 {
            return;
        }
        let points = word(h, list_cell(h, NAME_LIST_POINTS));
        if points == 0 {
            return;
        }
        let fd = h.call_sym(SCE_OPEN, Conv::Cdecl, &[path, 1]);
        if fd.cast_signed() < 0 {
            let m = h.rebase(MSG_OPEN_ERR);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[m, path]);
            return;
        }
        h.call_sym(SCE_LSEEK, Conv::Cdecl, &[fd, 0, 0]);
        h.call_sym(SCE_READ, Conv::Cdecl, &[fd, frame + 0xc, 4]);
        let n = word(h, frame + 0xc);
        if n.cast_signed() <= 0 {
            let m = h.rebase(MSG_FILE_ERR);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[m, path]);
            return;
        }
        let count = list_cell(h, NAME_LIST_COUNTS);
        h.write(count, n);
        for _ in 0..3 {
            h.call_sym(SCE_READ, Conv::Cdecl, &[fd, frame + 8, 4]);
        }
        let (c180, pi) = (qword(h, C_180), qword(h, C_PI));
        h.write(frame, 0_u32);
        let mut i = 0_u32;
        let mut cursor = entries.wrapping_add(8);
        h.write(frame + 4, cursor);
        loop {
            let e = entries.wrapping_add(i.wrapping_mul(0xc));
            h.write(e, [1_u32, 0]);
            let p = points.wrapping_add(i.wrapping_mul(0x10));
            for k in 0..4 {
                h.call_sym(SCE_READ, Conv::Cdecl, &[fd, p + 4 * k, 4]);
            }
            let deg = x87::div(x87::mul(x87::load(word(h, p + 0xc)), c180), pi);
            let d = x87::ftol(h, deg);
            h.write(cursor, d);
            cursor = cursor.wrapping_add(0xc);
            i += 1;
            h.write(frame + 4, cursor);
            h.write(frame, i);
            if i.cast_signed() >= word(h, frame + 0xc).cast_signed() {
                break;
            }
        }
        h.call_sym(SCE_CLOSE, Conv::Cdecl, &[fd]);
        let m = h.rebase(MSG_LOADED);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[m, path]);
    }

    /// `KERNEL32!InitializeCriticalSection`, `EnterCriticalSection`,
    /// `LeaveCriticalSection`, `CreateEventA` (import slots).
    pub const INIT_CRITICAL_SECTION_IAT: u32 = 0x00b0_c150;
    pub const ENTER_CRITICAL_SECTION_IAT: u32 = 0x00b0_c154;
    pub const LEAVE_CRITICAL_SECTION_IAT: u32 = 0x00b0_c158;
    pub const CREATE_EVENT_IAT: u32 = 0x00b0_c0fc;
    /// The page manager's "built" byte (`0x00c613ac`).
    pub const PAGE_MANAGER_BUILT: u32 = 0x00c6_13ac;
    /// `FUN_00943a50(this = arena; size)` (`ret 4`): an allocation.
    pub const ARENA_ALLOC: u32 = 0x0094_3a50;
    pub const PAGE_BUCKET_INIT: u32 = 0x0094_8900;
    pub const CRT_MEMSET: u32 = 0x0094_964c;

    fn import(h: &mut impl Host, iat: u32, args: &[u32]) -> u32 {
        let f = word(h, h.rebase(iat));
        h.call(f, Conv::Stdcall, args)
    }

    /// An allocation of `size` from the arena at `cs + 0x18`, under the
    /// lock `cs`.
    fn locked_alloc(h: &mut impl Host, cs: u32, size: u32) -> u32 {
        import(h, ENTER_CRITICAL_SECTION_IAT, &[cs]);
        let p = method(h, ARENA_ALLOC, cs.wrapping_add(0x18), &[size]);
        import(h, LEAVE_CRITICAL_SECTION_IAT, &[cs]);
        p
    }

    /// `FUN_00946f50(this)` (`thiscall`) -> `this`: the page manager's
    /// constructor: its lock (`+0x1c0c`), counters (`+0x1c30` 0x1000),
    /// an event (`+0x1c74`, `CreateEventA(0, 0, 0, 0)`), the list head
    /// `+0x1c58` (empty, pointing at itself) and 256 0x1c-byte buckets
    /// from `+0` (each `+0xc`/`+0x10` pointing at itself); marks
    /// `0x00c613ac`.
    pub fn page_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 0x1c00, [0_u32; 3]);
        import(h, INIT_CRITICAL_SECTION_IAT, &[this + 0x1c0c]);
        h.write(
            this + 0x1c24,
            [0_u32, 0, 0, 0x1000, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        h.write(this + 0x1c6c, [0_u32; 2]);
        let ev = import(h, CREATE_EVENT_IAT, &[0, 0, 0, 0]);
        h.write(this + 0x1c74, ev);
        let at = h.rebase(PAGE_MANAGER_BUILT);
        h.write(at, 1_u8);
        let head = this + 0x1c58;
        h.write(head, [0_u32, 0, head, head, 0]);
        for i in 0..0x100 {
            let b = this + 0x1c * i;
            h.write(b, [0_u32, 0, 0, b, b, 0, 0]);
        }
        this
    }

    /// `FUN_00946b20(this; index, arg)` (`thiscall`, `ret 8`): a page
    /// bucket (0x30 bytes from the arena of the lock `+0x404`,
    /// `PageBucket_init(index, arg, [this])`, its lock `+0x18`) in slot
    /// `index / 4` (`+4`), marked in the bitmap `+0x40c`, counted
    /// `+0x414`, and copied down into the free slots below it up to the
    /// first marked one.
    pub fn page_bucket_add(h: &mut impl Host, this: u32, index: u32, arg: u32) {
        let q = index.cast_signed() / 4;
        let cs = word(h, this + 0x404);
        let obj = locked_alloc(h, cs, 0x30);
        if obj != 0 {
            let heap = word(h, this);
            method(h, PAGE_BUCKET_INIT, obj, &[index, arg, heap]);
            import(h, INIT_CRITICAL_SECTION_IAT, &[obj + 0x18]);
        }
        let qu = q.cast_unsigned();
        h.write(this.wrapping_add(4).wrapping_add(qu.wrapping_mul(4)), obj);
        let bits = word(h, this + 0x40c);
        let cell = bits.wrapping_add(((q / 32).cast_unsigned()).wrapping_mul(4));
        let v = word(h, cell) | 1_u32.rotate_left(qu & 0x1f);
        h.write(cell, v);
        let n = word(h, this + 0x414).wrapping_add(1);
        h.write(this + 0x414, n);
        let mut i = q - 1;
        let mut mask = 1_u32.rotate_left(i.cast_unsigned() & 0x1f);
        while i >= 0 {
            let bits = word(h, this + 0x40c);
            let iu = i.cast_unsigned();
            if word(h, bits.wrapping_add((iu >> 5).wrapping_mul(4))) & mask != 0 {
                break;
            }
            mask = mask.rotate_right(1);
            h.write(this.wrapping_add(4).wrapping_add(iu.wrapping_mul(4)), obj);
            i -= 1;
        }
    }

    /// `FUN_00948750(this; bits, cs)` (`thiscall`, `ret 8`) -> `this`: a
    /// bitmap of `bits` bits (`+0`), its words (`+4`) from the arena of
    /// the lock `cs` (`+0x10`), cleared, the bits past the end of the
    /// last word set.
    pub fn bitmap_init(h: &mut impl Host, this: u32, bits: u32, cs: u32) -> u32 {
        h.write(this, bits);
        let bytes = ((bits.wrapping_add(0x1f) & !0x1f).cast_signed() / 8).cast_unsigned();
        h.write(this + 8, [0_u32, 0, cs]);
        let p = locked_alloc(h, cs, bytes);
        h.write(this + 4, p);
        h.call_sym(CRT_MEMSET, Conv::Cdecl, &[p, 0, bytes]);
        let r = bits & 0x1f;
        if r != 0 {
            let last = word(h, this + 4)
                .wrapping_add((bytes >> 2).wrapping_mul(4))
                .wrapping_sub(4);
            let mut mask = 1_u32.rotate_left(r);
            for _ in 0..(32 - r) {
                let v = word(h, last) | mask;
                h.write(last, v);
                mask = mask.rotate_left(1);
            }
        }
        this
    }

    pub const DELETE_CRITICAL_SECTION_IAT: u32 = 0x00b0_c15c;
    pub const LIST_WALK_TO_END: u32 = 0x0094_8a80;
    /// `FUN_00942ce0(this = arena)`, `FUN_00948960(this = bucket)` (its
    /// destructor), `FUN_00943e30(this = arena; p)` (a free) and
    /// `PageBucket_lockedCall_00948820(this = bitmap)` (frees its words).
    pub const ARENA_RESET: u32 = 0x0094_2ce0;
    pub const PAGE_BUCKET_DTOR: u32 = 0x0094_8960;
    pub const ARENA_FREE: u32 = 0x0094_3e30;
    pub const BITMAP_FREE: u32 = 0x0094_8820;

    /// Bucket `i` of a bucket table (slots `+4`, bitmap words
    /// `[+0x40c]`) when marked (`mask` = bit `i % 32`).
    fn marked_bucket(h: &impl Mem, this: u32, i: u32, mask: u32) -> Option<u32> {
        let bits = word(h, this + 0x40c);
        (word(h, bits.wrapping_add((i >> 5) * 4)) & mask != 0).then(|| word(h, this + 4 + 4 * i))
    }

    /// `FUN_00946840(this)` (`thiscall`) -> 0x100: each marked bucket's
    /// list walked to its end (`List_walkToEnd`) under its lock `+0x18`.
    pub fn bucket_table_walk(h: &mut impl Host, this: u32) -> u32 {
        let mut mask = 1_u32;
        for i in 0..0x100 {
            if let Some(obj) = marked_bucket(h, this, i, mask) {
                import(h, ENTER_CRITICAL_SECTION_IAT, &[obj.wrapping_add(0x18)]);
                method(h, LIST_WALK_TO_END, obj, &[]);
                import(h, LEAVE_CRITICAL_SECTION_IAT, &[obj.wrapping_add(0x18)]);
            }
            mask = mask.rotate_left(1);
        }
        0x100
    }

    /// `FUN_00945b80(this)` (`thiscall`) -> the last
    /// `LeaveCriticalSection`'s `eax`: both bucket tables (`+0xd0`,
    /// `+0x4ec`) walked, then the arenas of the locks `+0x908` and
    /// `+0x938` reset (`FUN_00942ce0`) under them.
    pub fn page_manager_walk(h: &mut impl Host, this: u32) -> u32 {
        method(h, PAGE_TABLE_WALK, this + 0xd0, &[]);
        method(h, PAGE_TABLE_WALK, this + 0x4ec, &[]);
        let mut r = 0;
        for cs in [this + 0x908, this + 0x938] {
            import(h, ENTER_CRITICAL_SECTION_IAT, &[cs]);
            method(h, ARENA_RESET, cs + 0x18, &[]);
            r = import(h, LEAVE_CRITICAL_SECTION_IAT, &[cs]);
        }
        r
    }
    pub const PAGE_TABLE_WALK: u32 = 0x0094_6840;

    /// `FUN_00946660(this)` (`thiscall`) -> `PageBucket_lockedCall`'s
    /// `eax`: the bucket table's destructor: each marked bucket's lock
    /// deleted, the bucket destroyed (`0x00948960`) and freed into the
    /// arena of the lock `+0x404`; then the bitmap `+0x408` freed.
    pub fn bucket_table_dtor(h: &mut impl Host, this: u32) -> u32 {
        let mut mask = 1_u32;
        for i in 0..0x100 {
            if let Some(obj) = marked_bucket(h, this, i, mask) {
                let cs = word(h, this + 0x404);
                import(h, DELETE_CRITICAL_SECTION_IAT, &[obj.wrapping_add(0x18)]);
                method(h, PAGE_BUCKET_DTOR, obj, &[]);
                import(h, ENTER_CRITICAL_SECTION_IAT, &[cs]);
                method(h, ARENA_FREE, cs.wrapping_add(0x18), &[obj]);
                import(h, LEAVE_CRITICAL_SECTION_IAT, &[cs]);
            }
            mask = mask.rotate_left(1);
        }
        method(h, BITMAP_FREE, this + 0x408, &[])
    }

    /// `FUN_009453d0(this; key)` (`thiscall`, `ret 4`) -> the removed
    /// node's value (`+8`): removal from an open-addressed hash table
    /// (`+0` size, a power of two; `+4` slots; `+8` count; `+0x14` free
    /// nodes) of the node whose id (`+0`) is the key's (`+0`), probing
    /// from the key's hash (`+4`); the cluster's later nodes whose home
    /// lies outside the hole's range moved back into it, the node put
    /// on the free list. The node must be there (the original reads
    /// through a null slot otherwise).
    pub fn hash_remove<H14: Host>(h: &mut H14, this: u32, key: u32) -> u32 {
        let n = word(h, this);
        let mask = n.wrapping_sub(1);
        let slots = word(h, this + 4);
        let slot = |h: &H14, i: u32| word(h, slots.wrapping_add(i.wrapping_mul(4)));
        let id = word(h, key);
        let mut i = word(h, key + 4) & mask;
        while slot(h, i) != 0 && word(h, slot(h, i)) != id {
            i = i.wrapping_add(1) & mask;
        }
        let node = slot(h, i);
        let value = word(h, node + 8);
        // the cluster's first slot
        let mut start = i;
        let prev = |j: u32| if j == 0 { n } else { j }.wrapping_sub(1);
        let mut j = prev(i);
        while slot(h, j) != 0 {
            start = j;
            j = prev(j);
        }
        let mut hole = i;
        loop {
            let n2 = word(h, this);
            let m2 = n2.wrapping_sub(1);
            let s2 = word(h, this + 4);
            let mut c = hole.wrapping_add(1) & m2;
            let mut found = None;
            while word(h, s2.wrapping_add(c.wrapping_mul(4))) != 0 {
                let origin =
                    (word(h, word(h, s2.wrapping_add(c.wrapping_mul(4))) + 4) & m2).cast_signed();
                let (s, e) = (start.cast_signed(), hole.cast_signed());
                let take = if s <= e {
                    origin >= s && origin <= e
                } else {
                    origin >= s || origin <= e
                };
                if take {
                    found = Some(c);
                }
                c = c.wrapping_add(1) & m2;
            }
            let Some(f) = found else { break };
            let v = word(h, s2.wrapping_add(f.wrapping_mul(4)));
            h.write(s2.wrapping_add(hole.wrapping_mul(4)), v);
            hole = f;
        }
        let s = word(h, this + 4);
        h.write(s.wrapping_add(hole.wrapping_mul(4)), 0_u32);
        let free = word(h, this + 0x14);
        h.write(node, free);
        let count = word(h, this + 8).wrapping_sub(1);
        h.write(this + 8, count);
        h.write(this + 0x14, node);
        value
    }

    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// The `GDraw` state (`0x01a858b4`; its first word the D3D11 device)
    /// and `IggyGDrawSendWarning` (import slot).
    pub const GDRAW: u32 = 0x01a8_58b4;
    pub const SEND_WARNING_IAT: u32 = 0x00b0_c874;
    /// The warnings (out of memory; an error with its code), the
    /// `"CreateBuffer"`, `" creating dynamic vertex buffer"` and
    /// `"CreatePixelShader"` texts; `E_OUTOFMEMORY`.
    pub const MSG_D3D_OOM: u32 = 0x00b7_a4ec;
    pub const MSG_D3D_ERROR: u32 = 0x00b7_a50c;
    pub const TEXT_CREATE_BUFFER: u32 = 0x00b7_a6e4;
    pub const TEXT_DYNAMIC_VB: u32 = 0x00b7_a848;
    pub const TEXT_CREATE_PS: u32 = 0x00b7_a938;
    pub const E_OUTOFMEMORY: u32 = 0x8007_000e;
    pub const PFILE_SEEK: u32 = 0x0060_80e0;
    pub const PSTREAM_FILE_READ: u32 = 0x0060_8090;

    /// `FUN_00a26480(this; buffer, value)` (`thiscall`, `ret 8`) -> 0, or
    /// 13 without a buffer: an array (`+0` count, top bit "not owned";
    /// `+4` words) grown by one into `buffer`: the words copied, `*value`
    /// appended, the old words freed (`Phyre_PhyreFree`) unless the same
    /// or not owned; the count's top bit cleared.
    pub fn array_push_into(h: &mut impl Host, this: u32, buffer: u32, value: u32) -> u32 {
        if buffer == 0 {
            return 13;
        }
        let n = word(h, this) & 0x7fff_ffff;
        let mut src = word(h, this + 4);
        for k in 0..n {
            let dst = buffer.wrapping_add(4 * k);
            if dst != 0 {
                let v = word(h, src);
                h.write(dst, v);
                src = src.wrapping_add(4);
            }
        }
        let end = buffer.wrapping_add(n.wrapping_mul(4));
        if end != 0 {
            let v = word(h, value);
            h.write(end, v);
        }
        let old = word(h, this + 4);
        if old != buffer && word(h, this) >> 31 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 4, buffer);
        h.write(this, n.wrapping_add(1));
        0
    }

    /// `IggyGDrawSendWarning(0, fmt, what, detail[, hr])` for a failed
    /// D3D call: out of memory, or the error code.
    fn d3d_warning(h: &mut impl Host, hr: u32, what: u32, detail: u32) -> u32 {
        let f = word(h, h.rebase(SEND_WARNING_IAT));
        let (what, detail) = (h.rebase(what), h.rebase(detail));
        if hr == E_OUTOFMEMORY {
            let m = h.rebase(MSG_D3D_OOM);
            h.call(f, Conv::Cdecl, &[0, m, what, detail])
        } else {
            let m = h.rebase(MSG_D3D_ERROR);
            h.call(f, Conv::Cdecl, &[0, m, what, detail, hr])
        }
    }

    /// `FUN_00a35a70(size, bind)` (`cdecl`; the buffer cell and
    /// description at `frame`, the original's `ebp - 0x20`) -> the
    /// buffer or 0: a dynamic D3D11 buffer (usage 2, CPU write 0x10000)
    /// from the `GDraw` device's `CreateBuffer` (slot 3), a warning on
    /// failure.
    pub fn gdraw_create_dynamic_buffer(h: &mut impl Host, frame: u32, size: u32, bind: u32) -> u32 {
        let dev = word(h, word(h, h.rebase(GDRAW)));
        h.write(frame, [0_u32, size, 2, bind, 0x10000, 0, 0]);
        let f = word(h, word(h, dev) + 0xc);
        let hr = h.call(f, Conv::Stdcall, &[dev, frame + 4, 0, frame]);
        if hr.cast_signed() < 0 {
            d3d_warning(h, hr, TEXT_CREATE_BUFFER, TEXT_DYNAMIC_VB);
            return 0;
        }
        word(h, frame)
    }

    /// `FUN_00a35b40(shader, code)` (`cdecl`) -> the call's `eax`:
    /// `shader` = `code` (bytecode, size), then with bytecode the `GDraw`
    /// device's `CreatePixelShader(bytecode, size, 0, &shader[1])` (slot
    /// 15) into the second word, 0 there and a warning on failure.
    pub fn gdraw_create_pixel_shader(h: &mut impl Host, shader: u32, code: u32) -> u32 {
        let a = word(h, code);
        h.write(shader, a);
        let set = word(h, shader) != 0;
        let b = word(h, code + 4);
        h.write(shader + 4, b);
        if !set {
            return b;
        }
        let dev = word(h, word(h, h.rebase(GDRAW)));
        let f = word(h, word(h, dev) + 0x3c);
        let (bytecode, size) = (word(h, shader), word(h, shader + 4));
        let hr = h.call(f, Conv::Stdcall, &[dev, bytecode, size, 0, shader + 4]);
        if hr.cast_signed() >= 0 {
            return hr;
        }
        let r = d3d_warning(h, hr, TEXT_CREATE_PS, TEXT_EMPTY);
        h.write(shader + 4, 0_u32);
        r
    }

    /// `VbfMkvReader_Read(this; offset (i64), size, buffer)` (`thiscall`,
    /// `ret 0x10`) -> 0, or -1 (no file `+0x10`, a negative offset or
    /// size, an offset at or past the length `+8` (i64), a failed seek
    /// or a short read): `size` bytes at `offset` read into `buffer`.
    pub fn mkv_reader_read(
        h: &mut impl Host,
        this: u32,
        lo: u32,
        hi: u32,
        size: u32,
        buffer: u32,
    ) -> u32 {
        let file = word(h, this + 0x10);
        let off = (u64::from(hi) << 32 | u64::from(lo)).cast_signed();
        if file == 0 || off < 0 || size.cast_signed() < 0 {
            return u32::MAX;
        }
        if size == 0 {
            return 0;
        }
        let len =
            (u64::from(word(h, this + 0xc)) << 32 | u64::from(word(h, this + 8))).cast_signed();
        if off >= len {
            return u32::MAX;
        }
        let r = method(h, PFILE_SEEK, file, &[lo, 0]);
        if r != lo || (r.cast_signed() >> 31).cast_unsigned() != hi {
            return u32::MAX;
        }
        let file = word(h, this + 0x10);
        let n = method(h, PSTREAM_FILE_READ, file, &[buffer, size]);
        if n < size { u32::MAX } else { 0 }
    }

    /// `g_GDrawContext` (`0x01a858c4`) and
    /// `Phyre_PRendering_PRenderer_syncRender` (`thiscall`).
    pub const GDRAW_CONTEXT: u32 = 0x01a8_58c4;
    pub const RENDERER_SYNC_RENDER: u32 = 0x005b_c3f0;

    /// `FUN_00a3ce00(x, y, w, h)` (`cdecl`; the viewport and rectangle
    /// at `frame`, the original's `ebp - 0x2c`) -> the `GDraw` state
    /// pointer: the device context's viewport (`RSSetViewports`, slot
    /// 44: x, y, w, h as floats, depth 0..1) and scissor rectangle
    /// (`RSSetScissorRects`, slot 45), the rectangle also kept at
    /// `GDraw` `+0x6fc` (x, y, w, h); `GDraw` `+0x9c4` cleared.
    pub fn gdraw_set_viewport(
        h: &mut impl Host,
        frame: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
    ) -> u32 {
        let g = word(h, h.rebase(GDRAW));
        let ctx = word(h, g + 4);
        let one = 1.0_f32.to_bits();
        h.write(
            frame,
            [int_f32(x), int_f32(y), int_f32(w), int_f32(ht), 0, one],
        );
        let f = word(h, word(h, ctx) + 0xb0);
        h.call(f, Conv::Stdcall, &[ctx, 1, frame]);
        let g = word(h, h.rebase(GDRAW));
        h.write(g + 0x704, w);
        h.write(g + 0x700, y);
        h.write(g + 0x6fc, x);
        h.write(g + 0x708, ht);
        h.write(frame + 0x18, [x, y, w.wrapping_add(x), y.wrapping_add(ht)]);
        let ctx = word(h, g + 4);
        let f = word(h, word(h, ctx) + 0xb4);
        h.call(f, Conv::Stdcall, &[ctx, 1, frame + 0x18]);
        let g = word(h, h.rebase(GDRAW));
        h.write(g + 0x9c4, 0_u32);
        g
    }

    /// An owned word array's words (`+8`, count `+4` with top bit "not
    /// owned") freed with `Phyre_PhyreFree` and both cleared.
    fn array_release(h: &mut impl Host, this: u32) {
        let n = word(h, this + 4);
        if n != 0 {
            let p = word(h, this + 8);
            if p != 0 && n >> 31 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            }
            h.write(this + 4, [0_u32, 0]);
        }
    }

    /// `FUN_00a3d310(this)` (`thiscall`) -> `this`: a `GDraw` texture
    /// record's constructor: words `+0..+0x18` 0, floats `+0x1c..+0x24`
    /// 0, byte `+0x28` 0 (then the just-cleared array `+4` released,
    /// which does nothing).
    pub fn gdraw_record_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, [0_u32; 7]);
        h.write(this + 0x28, 0_u8);
        h.write(this + 0x1c, [0_u32; 3]);
        array_release(h, this);
        this
    }

    /// `FUN_00a3d410(this)` (`thiscall`): its teardown: the `GDraw`
    /// context's renderer (`+4`) flushed (virtual slot 2) and synced,
    /// then the array `+4` released (twice, as the inlined destructor
    /// does: the second time the count is 0).
    pub fn gdraw_record_release(h: &mut impl Host, this: u32) {
        let ctx = word(h, h.rebase(GDRAW_CONTEXT));
        if ctx != 0 {
            let r = word(h, ctx + 4);
            if r != 0 {
                let f = word(h, word(h, r) + 8);
                h.call(f, Conv::Thiscall(r), &[]);
                let r = word(h, ctx + 4);
                method(h, RENDERER_SYNC_RENDER, r, &[]);
            }
        }
        array_release(h, this);
        let n = word(h, this + 4);
        if n >> 31 == 0 {
            let p = word(h, this + 8);
            if p != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            }
        }
        h.write(this + 4, [0_u32, 0]);
    }

    pub const PHYRE_MALLOC: u32 = 0x0042_fb40;
    pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
    pub const PAIR_CLEAR: u32 = 0x00a3_4a30;
    pub const IGGY_RENDER_SETUP: u32 = 0x00a3_ba50;
    /// The `GDraw` context's destructor (`0x00a34ab0`, a bare `ret`).
    pub const GDRAW_CONTEXT_DTOR: u32 = 0x00a3_4ab0;
    /// `g_GDrawContextRefs` (`0x01a858c8`).
    pub const GDRAW_CONTEXT_REFS: u32 = 0x01a8_58c8;
    /// Iggy imports: `IggyPlayerDispatchEventRS`,
    /// `IggyPlayerCreateFromMemory`, `IggyPlayerInitializeAndTickRS`,
    /// `IggyPlayerPlay`, `IggyPlayerProperties` (stdcall slots).
    pub const IGGY_DISPATCH_EVENT_IAT: u32 = 0x00b0_c85c;
    pub const IGGY_CREATE_FROM_MEMORY_IAT: u32 = 0x00b0_c7dc;
    pub const IGGY_INIT_AND_TICK_IAT: u32 = 0x00b0_c7e4;
    pub const IGGY_PLAY_IAT: u32 = 0x00b0_c7f0;
    pub const IGGY_PROPERTIES_IAT: u32 = 0x00b0_c7e0;
    /// The movie's width, height and frame rate lines.
    pub const MSG_MOVIE_WIDTH: u32 = 0x00b7_a980;
    pub const MSG_MOVIE_HEIGHT: u32 = 0x00b7_a9a0;
    pub const MSG_MOVIE_FPS: u32 = 0x00b7_a9c0;

    fn refs_inc(h: &mut impl Host) {
        let at = h.rebase(GDRAW_CONTEXT_REFS);
        let n = word(h, at).wrapping_add(1);
        h.write(at, n);
    }

    /// `FUN_00a3d5f0(renderer, audio, arg)` (`cdecl`) -> 0, 13 (no
    /// memory) or the set-up's error: the shared `GDraw` context, made on
    /// first use (8 bytes, `Pair_Clear`, `IggyGDraw_renderSetup`; freed
    /// again when that fails), its reference count raised.
    pub fn gdraw_context_acquire(h: &mut impl Host, renderer: u32, audio: u32, arg: u32) -> u32 {
        let cell = h.rebase(GDRAW_CONTEXT);
        if word(h, cell) == 0 {
            let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[8]);
            let obj = if p == 0 {
                0
            } else {
                method(h, PAIR_CLEAR, p, &[])
            };
            if obj == 0 {
                return 13;
            }
            let r = method(h, IGGY_RENDER_SETUP, obj, &[renderer, audio, arg]);
            if r != 0 {
                method(h, GDRAW_CONTEXT_DTOR, obj, &[]);
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[obj]);
                return r;
            }
            h.write(cell, obj);
        }
        refs_inc(h);
        0
    }

    /// `FUN_00a3d7f0(this; list)` (`thiscall`, `ret 4`; the event result
    /// at `out`, the original's `ebp - 0x10`) -> the last dispatch's
    /// `eax`, else `list`: each of the list's events (`[list + 4]`:
    /// count `+8`, 0x18-byte events from `+0xc`) dispatched to the
    /// player `[this]` (`IggyPlayerDispatchEventRS`).
    pub fn iggy_dispatch_events(h: &mut impl Host, out: u32, this: u32, list: u32) -> u32 {
        let events = word(h, list + 4);
        let mut r = list;
        if events != 0 && word(h, this) != 0 {
            let mut i = 0_u32;
            while i < word(h, events + 8) {
                let player = word(h, this);
                let e = events.wrapping_add(0xc).wrapping_add(i.wrapping_mul(0x18));
                r = import(h, IGGY_DISPATCH_EVENT_IAT, &[player, e, out]);
                i += 1;
            }
        }
        r
    }

    /// `FUN_00a3d900(this; a, b, c, d, data, size, _)` (`thiscall`, `ret
    /// 0x1c`) -> 0, 5 (no data) or 9 (no player): an Iggy player made
    /// from the movie in memory (`[this]`), initialised, playing, its
    /// width, height and frame rate printed; `+0xc` = c, `+0x10` = d,
    /// `+0x14` = a, `+0x18` = b.
    pub fn iggy_player_create(h: &mut impl Host, this: u32, args: [u32; 6]) -> u32 {
        let [a, b, c, d, data, size] = args;
        if data == 0 {
            return 5;
        }
        let p = import(h, IGGY_CREATE_FROM_MEMORY_IAT, &[data, size, 0]);
        h.write(this, p);
        if p == 0 {
            return 9;
        }
        import(h, IGGY_INIT_AND_TICK_IAT, &[p]);
        let p = word(h, this);
        import(h, IGGY_PLAY_IAT, &[p]);
        let p = word(h, this);
        let props = import(h, IGGY_PROPERTIES_IAT, &[p]);
        if props != 0 {
            for (m, o) in [(MSG_MOVIE_WIDTH, 0), (MSG_MOVIE_HEIGHT, 4)] {
                let (m, v) = (h.rebase(m), word(h, props + o));
                h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, m, v]);
            }
            let m = h.rebase(MSG_MOVIE_FPS);
            let [lo, hi] = dwords(x87::load(word(h, props + 0xc)));
            h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, m, lo, hi]); // call width: 4
        }
        h.write(this + 0xc, [c, d, a, b]);
        0
    }

    pub const MS_CAMERA_SET_SPLINE: u32 = 0x007b_fca0;
    pub const MS_CAMERA_SET_ROLL: u32 = 0x007b_fa80;
    pub const TK_CALC_SPLINE: u32 = 0x0088_a4a0;
    pub const TK_CALC_SPLINE_TANGENT: u32 = 0x0088_b3b0;
    pub const TK_CALC_SPLINE3: u32 = 0x0088_a4d0;
    pub const TK_CALC_SPLINE3_TANGENT: u32 = 0x0088_aba0;
    /// The debug camera path's position (`0x0193c380`, `f32` 0..1), its
    /// step (`0x0193c384`) and its spline kind byte (`0x025d5f31`).
    pub const PATH_T: u32 = 0x0193_c380;
    pub const PATH_STEP: u32 = 0x0193_c384;
    pub const PATH_SPLINE3: u32 = 0x025d_5f31;

    /// The path position stepped on and stopped at 1 (a NaN kept).
    fn path_advance(h: &mut impl Mem) -> u32 {
        let (at, step) = (h.rebase(PATH_T), h.rebase(PATH_STEP));
        let t = x87::store(x87::add(x87::load(word(h, at)), x87::load(word(h, step))));
        h.write(at, t);
        let x = x87::load(t);
        if x < 1.0 || x.is_nan() {
            t
        } else {
            let one = 1.0_f32.to_bits();
            h.write(at, one);
            one
        }
    }

    /// `FUN_009402c0()` (`cdecl`) -> `MsCameraSetSpline`'s `eax`: the
    /// path stepped, the current list's points headed by their count
    /// (`points - 0x10`: count, 0, 0, 0) handed to the debug camera's
    /// spline (`MsCameraSetSpline(camera, 1, head, t, 0.0, 0)`).
    pub fn camera_path_spline(h: &mut impl Host) -> u32 {
        let t = path_advance(h);
        let n = word(h, list_cell(h, NAME_LIST_COUNTS));
        let head = word(h, list_cell(h, NAME_LIST_POINTS)).wrapping_sub(0x10);
        h.write(head, [n, 0, 0, 0]);
        let head = word(h, list_cell(h, NAME_LIST_POINTS)).wrapping_sub(0x10);
        let cam = word(h, h.rebase(DEBUG_CAMERA));
        h.call_sym(MS_CAMERA_SET_SPLINE, Conv::Cdecl, &[cam, 1, head, t, 0, 0])
    }

    /// `FUN_00940380()` / `FUN_00940480()` (`cdecl`; the position and
    /// the aim at `frame`, the original's `ebp - 0x24`) -> the last
    /// `MsCameraSetRect`'s `eax`: the path stepped and evaluated along
    /// the current list (`TkMtCalcSpline`/`Tangent`, or the `3` pair
    /// when `cubic`; the kind byte `0x025d5f31` set to match), the
    /// debug camera placed there (roll = the position's fourth float)
    /// looking along the tangent.
    pub fn camera_path_follow(h: &mut impl Host, frame: u32, cubic: bool) -> u32 {
        let t = path_advance(h);
        let kind = h.rebase(PATH_SPLINE3);
        h.write(kind, u8::from(cubic));
        let (spline, tangent) = if cubic {
            (TK_CALC_SPLINE3, TK_CALC_SPLINE3_TANGENT)
        } else {
            (TK_CALC_SPLINE, TK_CALC_SPLINE_TANGENT)
        };
        let (pts, n) = (
            word(h, list_cell(h, NAME_LIST_POINTS)),
            word(h, list_cell(h, NAME_LIST_COUNTS)),
        );
        h.call_sym(spline, Conv::Cdecl, &[frame, pts, n, t]);
        let t = word(h, h.rebase(PATH_T));
        let (pts, n) = (
            word(h, list_cell(h, NAME_LIST_POINTS)),
            word(h, list_cell(h, NAME_LIST_COUNTS)),
        );
        let aim = frame + 0x10;
        h.call_sym(tangent, Conv::Cdecl, &[aim, pts, n, t]);
        for k in 0..3 {
            let s = x87::add(
                x87::load(word(h, aim + 4 * k)),
                x87::load(word(h, frame + 4 * k)),
            );
            h.write(aim + 4 * k, x87::store(s));
        }
        h.write(aim + 0xc, 1.0_f32.to_bits());
        let roll = x87::store(x87::load(word(h, frame + 0xc)));
        let cam = word(h, h.rebase(DEBUG_CAMERA));
        h.call_sym(MS_CAMERA_SET_ROLL, Conv::Cdecl, &[cam, roll]);
        let cam = word(h, h.rebase(DEBUG_CAMERA));
        h.call_sym(MS_CAMERA_SET_RECT, Conv::Cdecl, &[cam, 0, aim]);
        let cam = word(h, h.rebase(DEBUG_CAMERA));
        h.call_sym(MS_CAMERA_SET_RECT, Conv::Cdecl, &[cam, 1, frame])
    }
}
