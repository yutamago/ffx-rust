//! The Iggy menus' manager (`g_IggyMenuManager`) and the Esc menu
//! (`FFEscMenu`): the pieces of their C++ classes that are game logic.

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    use crate::iggy::bridge as iggy;

    /// `0x00a28f40(menu, x, y, w, h)` (`thiscall`, unnamed): a menu's
    /// viewport, the four words through `ftol` into `+0x84..+0x90`.
    pub const MENU_SET_VIEWPORT: u32 = 0x00a2_8f40;

    /// `IggyMenuManager_ApplyViewport_00a265a0(this)` (name ours;
    /// `thiscall`, from the viewport setters `0x00a26a20` and
    /// `0x00a26bd0`): every menu of the list (`+0xc`, `+8 & 0x7fffffff`
    /// entries, both re-read for each) that is not null gets the viewport
    /// `(x +0x18, screen h +0x2c - y +0x1c - h +0x24, w +0x20, h +0x24)`,
    /// bottom-up, its fields re-read for each.
    #[allow(clippy::many_single_char_names)] // the rectangle
    pub fn apply_viewport(mem: &mut impl Host, this: u32) {
        let mut i = 0u32;
        while i < mem.read::<u32>(this.wrapping_add(8)) & 0x7fff_ffff {
            let list: u32 = mem.read(this.wrapping_add(0xc));
            let menu: u32 = mem.read(list.wrapping_add(4 * i));
            if menu != 0 {
                let [x, y, w, h, screen_h] = [0x18, 0x1c, 0x20, 0x24, 0x2c]
                    .map(|off| mem.read::<u32>(this.wrapping_add(off)));
                let bottom = screen_h.wrapping_sub(y).wrapping_sub(h);
                mem.call_sym(MENU_SET_VIEWPORT, Conv::Thiscall(menu), &[x, bottom, w, h]);
            }
            i += 1;
        }
    }

    /// `IggyMenuManager_ApplyViewport_00a265a0` ([`apply_viewport`]).
    pub const APPLY_VIEWPORT: u32 = 0x00a2_65a0;
    /// `g_IggyTargetAspect` (`f32`, name ours): the aspect the menus are
    /// laid out for; [`set_viewport`] letterboxes or pillarboxes to it.
    pub const G_IGGY_TARGET_ASPECT: u32 = 0x00c6_b270;

    /// `IggyMenuManager_SetViewport_00a26a20(this, w, h)` (name ours;
    /// `thiscall`, `ret 8`; from `0x00a262e0` and through the jump at
    /// `0x00a26960`): the viewport for a `w` x `h` screen. With the
    /// screen's aspect `r = f32(f32(w) / f32(h))` and the target `a`
    /// ([`G_IGGY_TARGET_ASPECT`]):
    /// - `a == r`: `(0, 0, w, h)`;
    /// - `a > r` (letterbox): `k = f32(r * 0.5625)`, `(0, ftol((1 - k) h
    ///   0.5), w, ftol(k h))`;
    /// - else, unordered included (pillarbox): `q = f32(a * (1 / r))`,
    ///   `(ftol((1 - q) w 0.5), 0, ftol(q w), h)`;
    ///
    /// as `(+0x18, +0x1c, +0x20, +0x24)` with the screen `(w, h)` at
    /// `(+0x28, +0x2c)`. When any of the six words differs they are all
    /// stored and the menus get it ([`APPLY_VIEWPORT`], whose `eax` is
    /// returned); else `eax` is `h` (equal aspects) or the last `ftol`.
    #[allow(clippy::many_single_char_names)] // the rectangle
    #[allow(clippy::float_cmp)] // the original's fucom: exact
    pub fn set_viewport(mem: &mut impl Host, this: u32, w: u32, h: u32) -> u32 {
        use ffx_bridge::x87;
        let wf = x87::load(x87::store(f64::from(w.cast_signed())));
        let hf = x87::load(x87::store(f64::from(h.cast_signed())));
        let r = x87::load(x87::store(x87::div(wf, hf)));
        let a = x87::load(mem.read::<u32>(mem.rebase(G_IGGY_TARGET_ASPECT)));
        let (view, eax) = if a == r {
            ([0, 0, w, h], h)
        } else if a > r {
            let k = x87::load(x87::store(x87::mul(r, 0.5625)));
            let y = x87::ftol(mem, x87::mul(x87::mul(x87::sub(1.0, k), hf), 0.5));
            let vh = x87::ftol(mem, x87::mul(k, hf));
            ([0, y, w, vh], vh)
        } else {
            let q = x87::load(x87::store(x87::mul(a, x87::div(1.0, r))));
            let x = x87::ftol(mem, x87::mul(x87::mul(x87::sub(1.0, q), wf), 0.5));
            let vw = x87::ftol(mem, x87::mul(q, wf));
            ([x, 0, vw, h], vw)
        };
        let words = [view[0], view[1], view[2], view[3], w, h];
        if mem.read::<[u32; 6]>(this.wrapping_add(0x18)) == words {
            return eax;
        }
        mem.write(this.wrapping_add(0x18), words);
        mem.call_sym(APPLY_VIEWPORT, Conv::Thiscall(this), &[])
    }

    /// `0x007018a0(this, a, b)` and `0x00a28c10(this, a, b)` (`thiscall`,
    /// unnamed): the Esc menu's check and the base menu's handler.
    pub const ESC_MENU_CHECK: u32 = 0x0070_18a0;
    pub const MENU_HANDLE: u32 = 0x00a2_8c10;

    /// `FFEscMenu_vfunc_0068e530(this, a, b)` (name ours; a slot of the
    /// vtable at `0x00b46c24`, `thiscall`, `ret 8`): the base menu's
    /// handler `0x00a28c10(this, a, b)` only when `0x007018a0(this, a, b)`
    /// returns `al` 1; `eax` the last call's.
    pub fn esc_menu_handle(mem: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let r = mem.call_sym(ESC_MENU_CHECK, Conv::Thiscall(this), &[a, b]);
        if r.to_le_bytes()[0] != 1 {
            return r;
        }
        mem.call_sym(MENU_HANDLE, Conv::Thiscall(this), &[a, b])
    }

    /// `graphicGetLocTextWithLength(id, out, len)`: localised text `id`.
    pub const LOC_TEXT: u32 = 0x0064_12f0;
    /// `0x006e96b0(src, dst, len)` (a jump to `LocText_ConvertLineBreaks_006e9700`,
    /// name ours): `src` into `dst` with the text's line-break marker
    /// turned into `'\n'`.
    pub const TEXT_TO_IGGY: u32 = 0x006e_96b0;
    /// `LocString_Load_0068e180(this, id)` (name ours; `thiscall`, `ret
    /// 4`): localised text `id` through [`TEXT_TO_IGGY`] into the
    /// 0x100-byte text at `this`; returns `this`.
    pub const LOC_STRING_LOAD: u32 = 0x0068_e180;
    /// `LocString_ToValue_0068e270(this, out)` (name ours; `thiscall`,
    /// `ret 4`): the `IggyDataValue` at `this + 0x100` set to the text at
    /// `this`, its four words copied to `out`.
    pub const LOC_STRING_TO_VALUE: u32 = 0x0068_e270;
    /// `IggyMenu_callFlashFunction_3_0068ddd0(menu, name, v0, v1, v2)` and
    /// `IggyMenu_callFlashFunction_5_0068def0(menu, name, v0..v4)` (names
    /// ours, after Fahrenheit's `IggyMenu::callFlashFunction<1>` at
    /// `0x0068dd40`; `cdecl`): the values passed by value, four words
    /// each, 14 and 22 stack words.
    pub const CALL_FLASH_3: u32 = 0x0068_ddd0;
    pub const CALL_FLASH_5: u32 = 0x0068_def0;
    /// `"showPopUpWithBtn"`, `""` (`.rdata`).
    pub const SHOW_POP_UP_WITH_BTN: u32 = 0x00b4_6cac;
    pub const EMPTY_TEXT: u32 = 0x00b8_fbeb;
    /// `g_FFEscMenu`: the Esc menu.
    pub const G_FF_ESC_MENU: u32 = 0x00cc_c870;
    /// `FFEscMenu_QuitConfirmed_0068fcf0(yes)` (name ours): with `yes`,
    /// the menu off, the cursor shown, and back to the launcher.
    pub const QUIT_CONFIRMED: u32 = 0x0068_fcf0;

    /// The four words of the `IggyDataValue` at `v`, as a by-value
    /// argument (word 1, and word 3 of a boolean, are whatever the frame
    /// held: the setters don't write them).
    fn value_words(mem: &impl Host, v: u32) -> [u32; 4] {
        mem.read(v)
    }

    /// Bytes of work [`show_quit_pop_up`] needs: the original's frame
    /// (one 0x100-byte localised text, three converted ones, three
    /// values).
    pub const QUIT_POP_UP_WORK: u32 = 0x430;

    /// `FFEscMenu_ShowQuitPopUp_0068fb20()` (name ours; `cdecl`, the
    /// handler `FUN_0068fb00` queues for `EscMenuQuit`, called as `f(0)`):
    /// localised texts 0x309, 0x308, 0x307, each through
    /// [`TEXT_TO_IGGY`] into its own buffer and set as a string value;
    /// `showPopUpWithBtn` on `g_FFEscMenu` with the values of 0x307,
    /// 0x308, 0x309 ([`CALL_FLASH_3`] through `wide`, 14 words); then
    /// `g_FFEscMenu` (read again) gets the answer callback
    /// [`QUIT_CONFIRMED`] at `+0x10c` and 0 at `+0x110`. Returns that
    /// `g_FFEscMenu` (`eax`).
    pub fn show_quit_pop_up<H: Host>(
        mem: &mut H,
        work: u32,
        wide: &mut impl FnMut(&mut H, u32, &[u32]) -> u32,
    ) -> u32 {
        let loc = work;
        let mut args = vec![0u32; 2];
        let mut values = [[0u32; 4]; 3];
        for (k, id) in [0x309u32, 0x308, 0x307].into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // k < 3
            let k32 = k as u32;
            let text = work.wrapping_add(0x100 + 0x100 * k32);
            let value = work.wrapping_add(0x400 + 0x10 * k32);
            mem.call_sym(LOC_TEXT, Conv::Cdecl, &[id, loc, 0x100]);
            mem.call_sym(TEXT_TO_IGGY, Conv::Cdecl, &[loc, text, 0x100]);
            iggy::set_utf8(mem, value, text);
            values[k] = value_words(mem, value);
        }
        let g = mem.rebase(G_FF_ESC_MENU);
        args[0] = mem.read(g);
        args[1] = mem.rebase(SHOW_POP_UP_WITH_BTN);
        for v in values.iter().rev() {
            args.extend(v);
        }
        let f = mem.rebase(CALL_FLASH_3);
        wide(mem, f, &args);
        let menu: u32 = mem.read(g);
        let done = mem.rebase(QUIT_CONFIRMED);
        mem.write(menu.wrapping_add(0x10c), done);
        mem.write(menu.wrapping_add(0x110), 0u32);
        menu
    }

    /// Bytes of work [`show_incomplete_pop_up`] needs: the original's
    /// frame (two 0x110-byte localised strings, five values).
    pub const INCOMPLETE_POP_UP_WORK: u32 = 0x270;

    /// The texts of `FFEscMenu_ShowKeyIncompletePopUp_00701470` and
    /// `FFEscMenu_ShowPadIncompletePopUp_00701c50`, in load order.
    pub const KEY_INCOMPLETE_TEXTS: [u32; 2] = [0x302, 0x301];
    pub const PAD_INCOMPLETE_TEXTS: [u32; 2] = [0x37b, 0x37a];

    /// The settings pages' "incomplete" pop-up (`cdecl(menu)`, queued as
    /// `f(menu)` by `EscMenuKeyboardSettingIncomplete` and
    /// `EscMenuControllerSettingIncomplete`): `FFEscMenu_ShowKeyIncompletePopUp_00701470`
    /// with `ids` [`KEY_INCOMPLETE_TEXTS`], `FFEscMenu_ShowPadIncompletePopUp_00701c50`
    /// with [`PAD_INCOMPLETE_TEXTS`] (names ours). Two number values 1, the
    /// string value `""`, then each id's localised string
    /// ([`LOC_STRING_LOAD`], [`LOC_STRING_TO_VALUE`]) in that order;
    /// `showPopUpWithBtn` on `menu` with the values of `ids[1]`, `ids[0]`,
    /// `""`, 1, 1 ([`CALL_FLASH_5`] through `wide`, 22 words). Returns its
    /// `eax`.
    pub fn show_incomplete_pop_up<H: Host>(
        mem: &mut H,
        menu: u32,
        ids: [u32; 2],
        work: u32,
        wide: &mut impl FnMut(&mut H, u32, &[u32]) -> u32,
    ) -> u32 {
        let value = |k: u32| work.wrapping_add(0x220 + 0x10 * k);
        let (one_a, one_b, empty) = (value(0), value(1), value(2));
        iggy::set_int(mem, one_a, 1);
        iggy::set_int(mem, one_b, 1);
        let text = mem.rebase(EMPTY_TEXT);
        iggy::set_utf8(mem, empty, text);
        for (k, id) in (0u32..).zip(ids) {
            let s = work.wrapping_add(0x110 * k);
            let s = mem.call_sym(LOC_STRING_LOAD, Conv::Thiscall(s), &[id]);
            mem.call_sym(LOC_STRING_TO_VALUE, Conv::Thiscall(s), &[value(3 + k)]);
        }
        let mut args = vec![menu, mem.rebase(SHOW_POP_UP_WITH_BTN)];
        for v in [value(4), value(3), empty, one_b, one_a] {
            args.extend(value_words(mem, v));
        }
        let f = mem.rebase(CALL_FLASH_5);
        wide(mem, f, &args)
    }
}
