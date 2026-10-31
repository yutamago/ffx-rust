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
}
