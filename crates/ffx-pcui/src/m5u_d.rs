//! M5 stage 1, list `pc-ui`, batch 4: the text utility's resize (its
//! world matrix and line height from the screen size), the Iggy string
//! tokenizer, a pointer vector's `push_back`, the skip prompt, the remap
//! button icon's state and a `std::map` iterator step.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' argument words
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    use crate::m5u_a::bridge::PHYRE_FREE;
    use crate::m5u_c::bridge::PHYRE_MALLOC;

    pub const MATRIX_IDENTITY: u32 = 0x0045_4cf0;
    pub const MATRIX_COPY: u32 = 0x004c_9430;
    pub const MATRIX_SET_UPPER3: u32 = 0x0053_1a40;
    pub const MATRIX_TO_WORLD3X4: u32 = 0x0045_1190;
    pub const TEXT_SCROLL_TO_VISIBLE: u32 = 0x006f_6860;
    /// The resize's `f64` constants: 0.001, -0.32, 0.1, 0.8, 1.3.
    pub const K_EPS: u32 = 0x00b4_7c68;
    pub const K_X: u32 = 0x00b4_bf10;
    pub const K_Y_SCALE: u32 = 0x00b9_21f0;
    pub const K_Y_OFFSET: u32 = 0x00b3_eb68;
    pub const K_LINE: u32 = 0x00b4_bf00;
    pub const VEC_GROW: u32 = 0x006a_0a30;
    /// `MSVCR110!strchr` (cdecl).
    pub const STRCHR_IAT: u32 = 0x00b0_c3c0;
    pub const SKIP_DELIMS: u32 = 0x00a2_9030;
    pub const COUNT_TOKENS: u32 = 0x00a2_90d0;
    pub const RELEASE_TAGGED_WORDS: u32 = 0x0043_4360;
    pub const MEMCPY_THUNK: u32 = 0x0094_9256;
    pub const GRAPHIC_GET_TIME: u32 = 0x0064_1410;
    pub const ICON_SLOT_SET: u32 = 0x006e_8390;
    /// `g_ChrClothFlip` (-1.0) and 0.5 (`0x00b92194`).
    pub const MINUS_ONE: u32 = 0x00b9_2340;
    pub const HALF: u32 = 0x00b9_2194;

    fn rd(h: &impl Host, at: u32) -> u32 {
        h.read(at)
    }

    fn konst(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read::<u64>(h.rebase(sym)))
    }

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `fild` of a word read as unsigned (the `+ 2^32` fix-up for a set
    /// sign bit), then `fstp dword`.
    fn u32_f32(v: u32) -> u32 {
        x87::store(f64::from(v))
    }

    /// `TextUtil_resize(this; w, h)` (`ret 8`): the aspect `r = f32((w +
    /// 0.001) / (h + 0.001))` (both through `f32`); with the text on
    /// (`+0x80`), its object `+0x10` and asset `+0xc`: from the asset's
    /// glyph count `n` (`*(+0xc)+0x30 -> +4`) a scale `k = f32(f32(1 /
    /// (f32(6n) r)) * +0x8c)`, the matrix (identity, upper 3x3 diag(k, k,
    /// 1), translation `(f32((+0x84 + r) * -0.32), f32(+0x88 * 0.1 -
    /// 0.8), 1)`) handed to the object (`+0x128`, `toWorldMatrix3x4`) and
    /// kept at `+0x20..+0x5c`, the line height `+0x64 = f32(f32(n) * 1.3
    /// / (h + 0.001) * +0x8c)`, and `scrollToVisible(+0x74 & 0x7fffffff)`.
    /// `frame` is the original's `ebp - 0xc8`; the argument slots it reuses
    /// for its temporaries are not stored (no caller reads them).
    pub fn text_resize(h: &mut impl Host, this: u32, w: u32, hh: u32, frame: u32) {
        let eps = konst(h, K_EPS);
        let a = x87::add(x87::load(u32_f32(hh)), eps);
        h.write(frame + 0x70, a.to_bits());
        let b = x87::add(eps, x87::load(u32_f32(w)));
        let r = x87::load(x87::store(x87::div(b, a)));
        if h.read::<u8>(this + 0x80) == 0 || rd(h, this + 0x10) == 0 {
            return;
        }
        let asset = rd(h, this + 0xc);
        if asset == 0 {
            return;
        }
        let n = rd(h, rd(h, asset + 0x30) + 4);
        let s6 = x87::load(u32_f32(n.wrapping_mul(6)));
        let k = x87::store(x87::div(1.0, x87::mul(s6, r)));
        let ident = frame;
        let m4 = frame + 0x78;
        let id = h.call_sym(MATRIX_IDENTITY, Conv::Cdecl, &[ident]);
        h.call_sym(MATRIX_COPY, Conv::Thiscall(m4), &[id]);
        let k2 = x87::store(x87::mul(x87::load(k), x87::load(rd(h, this + 0x8c))));
        h.write(frame + 0xb8, k2);
        h.write(frame + 0xbc, k2);
        // the upper 3x3 (rows of 16 bytes, their fourth words unset)
        let m3 = frame + 0x40;
        let zero = 0u32;
        let one = 1.0f32.to_bits();
        for (off, v) in [
            (0x0, k2),
            (0x4, zero),
            (0x8, zero),
            (0x10, zero),
            (0x14, k2),
            (0x18, zero),
            (0x20, zero),
            (0x24, zero),
            (0x28, one),
        ] {
            h.write(m3 + off, v);
        }
        h.call_sym(MATRIX_SET_UPPER3, Conv::Thiscall(m4), &[m3]);
        let x = x87::store(x87::mul(
            x87::add(x87::load(rd(h, this + 0x84)), r),
            konst(h, K_X),
        ));
        let y = x87::store(x87::sub(
            x87::mul(x87::load(rd(h, this + 0x88)), konst(h, K_Y_SCALE)),
            konst(h, K_Y_OFFSET),
        ));
        h.write(frame + 0xb8, x);
        h.write(frame + 0xbc, y);
        h.write(m4 + 0x30, x);
        h.write(m4 + 0x34, y);
        h.write(m4 + 0x38, one);
        let obj = rd(h, this + 0x10) + 0x128;
        h.call_sym(MATRIX_TO_WORLD3X4, Conv::Thiscall(obj), &[m4]);
        for i in 0..12 {
            let v = rd(h, m4 + 4 * i);
            h.write(this + 0x20 + 4 * i, x87::store(x87::load(v)));
        }
        h.write(this + 0x50, x);
        h.write(this + 0x54, y);
        h.write(this + 0x58, one);
        let w3 = rd(h, m4 + 0x3c);
        h.write(this + 0x5c, x87::store(x87::load(w3)));
        let line = x87::mul(
            x87::div(x87::mul(x87::load(u32_f32(n)), konst(h, K_LINE)), a),
            x87::load(rd(h, this + 0x8c)),
        );
        h.write(this + 0x64, x87::store(line));
        let top = rd(h, this + 0x74) & 0x7fff_ffff;
        h.call_sym(TEXT_SCROLL_TO_VISIBLE, Conv::Thiscall(this), &[top]);
    }

    /// `0x006a2b20(this; &v)` (`std::vector<u32>::push_back`, `ret 4`): an
    /// element of the vector itself is read by index after a possible
    /// growth (`0x006a0a30(this; 1)`); the end moves by 4 even when the
    /// grown end is null.
    pub fn vec_push_back(h: &mut impl Host, this: u32, v: u32) {
        let end = rd(h, this + 4);
        let begin = rd(h, this);
        let inside = v < end && begin <= v;
        let idx = (v.wrapping_sub(begin).cast_signed() >> 2).cast_unsigned();
        if end == rd(h, this + 8) {
            h.call_sym(VEC_GROW, Conv::Thiscall(this), &[1]);
        }
        let p = rd(h, this + 4);
        if p != 0 {
            let x = if inside {
                rd(h, rd(h, this).wrapping_add(idx.wrapping_mul(4)))
            } else {
                rd(h, v)
            };
            h.write(p, x);
        }
        let e = rd(h, this + 4);
        h.write(this + 4, e.wrapping_add(4));
    }

    /// `strchr(set, (i8) c)` through the import.
    fn in_set(h: &mut impl Host, set: u32, c: u8) -> bool {
        let c = i32::from(c.cast_signed()).cast_unsigned();
        import(h, STRCHR_IAT, Conv::Cdecl, &[set, c]) != 0
    }

    /// `IggyString_skipDelims_00a29030(s, delims)` -> the first character
    /// of `s` not in `delims`, or 0 (none, or either string null/empty).
    pub fn skip_delims(h: &mut impl Host, s: u32, delims: u32) -> u32 {
        if delims == 0 || s == 0 {
            return 0;
        }
        let mut p = s;
        let mut c: u8 = h.read(p);
        while c != 0 {
            if !in_set(h, delims, c) {
                return p;
            }
            p = p.wrapping_add(1);
            c = h.read(p);
        }
        0
    }

    /// `IggyString_countTokens_00a290d0(s, delims)`: the tokens of `s`.
    pub fn count_tokens(h: &mut impl Host, s: u32, delims: u32) -> u32 {
        let mut n = 0;
        if delims == 0 || s == 0 {
            return 0;
        }
        let mut p = h.call_sym(SKIP_DELIMS, Conv::Cdecl, &[s, delims]);
        while p != 0 {
            n += 1;
            loop {
                let c: u8 = h.read(p);
                if c == 0 {
                    return n;
                }
                if in_set(h, delims, c) {
                    break;
                }
                p = p.wrapping_add(1);
            }
            p = h.call_sym(SKIP_DELIMS, Conv::Cdecl, &[p, delims]);
        }
        n
    }

    /// The end of the token at `p`: the first delimiter, else the
    /// terminator (as the original: `strlen` when `delims` or `p` is null).
    fn token_end(h: &mut impl Host, p: u32, delims: u32) -> u32 {
        if delims != 0 && p != 0 && h.read::<u8>(p) != 0 {
            let mut e = p;
            loop {
                let c: u8 = h.read(e);
                if in_set(h, delims, c) {
                    if h.read::<u8>(e) != 0 {
                        return e;
                    }
                    break;
                }
                e = e.wrapping_add(1);
                if h.read::<u8>(e) == 0 {
                    break;
                }
            }
        }
        let mut len = 0u32;
        while h.read::<u8>(p.wrapping_add(len)) != 0 {
            len += 1;
        }
        p.wrapping_add(len)
    }

    /// `IggyString_tokenize_00a29140(s, delims, arr)` -> 0, or 1 (a null
    /// token), 5 (a bad token), 0xd (no memory): the tokens of `s` as
    /// `PhyreMalloc`'d strings in the `PArray` `arr` (`+0` count, bit 31
    /// "not owned"; `+4` the words): the array is remade when its count
    /// differs (zeroed, the old words released when owned), each slot's
    /// old string freed unless its pointer's bit 0 is set.
    pub fn tokenize(h: &mut impl Host, s: u32, delims: u32, arr: u32) -> u32 {
        let n = h.call_sym(COUNT_TOKENS, Conv::Cdecl, &[s, delims]);
        if n != rd(h, arr) {
            let mut p = 0;
            if n != 0 {
                p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n << 2, 4]);
                if p == 0 {
                    return 0xd;
                }
                for i in 0..n {
                    h.write(p.wrapping_add(4 * i), 0u32);
                }
            }
            let old = rd(h, arr + 4);
            if old != p {
                let cap = rd(h, arr);
                if cap >> 31 == 0 {
                    h.call_sym(RELEASE_TAGGED_WORDS, Conv::Cdecl, &[old, cap & 0x7fff_ffff]);
                }
            }
            h.write(arr + 4, p);
            h.write(arr, n);
        }
        let mut tok = h.call_sym(SKIP_DELIMS, Conv::Cdecl, &[s, delims]);
        let mut i = 0;
        while i < n {
            let end = token_end(h, tok, delims);
            let slot = rd(h, arr + 4).wrapping_add(i.wrapping_mul(4));
            if tok == 0 || end == 0 {
                return 1;
            }
            if end < tok {
                return 5;
            }
            let len = end - tok;
            let buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[len + 1, 2]);
            if buf == 0 {
                return 0xd;
            }
            h.call_sym(MEMCPY_THUNK, Conv::Cdecl, &[buf, tok, len + 1]);
            h.write(buf + len, 0u8);
            let old = rd(h, slot);
            if old & 1 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
            }
            h.write(slot, buf);
            tok = h.call_sym(SKIP_DELIMS, Conv::Cdecl, &[end, delims]);
            i += 1;
        }
        0
    }

    /// `RemapButtonIcon_ctor_006e77d0(this)` -> `this`: `+0` -1.0, `+4`,
    /// `+8`, `+0x10` 0.0, `+0xc` 0.5, `+0x14..+0x68` 0.
    pub fn icon_state_init(h: &mut impl Host, this: u32) -> u32 {
        let m1 = x87::store(x87::load(rd(h, h.rebase(MINUS_ONE))));
        let half = x87::store(x87::load(rd(h, h.rebase(HALF))));
        h.write(this, m1);
        h.write(this + 4, 0u32);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, half);
        h.write(this + 0x10, 0u32);
        for off in (0x14..=0x68).step_by(4) {
            h.write(this + off, 0u32);
        }
        this
    }

    /// `SkipPrompt_show_006e78a0(this)`: `+0x14` = 1, `+4` = the time,
    /// `+0x15` = 1.
    pub fn skip_prompt_show(h: &mut impl Host, this: u32) {
        h.write(this + 0x14, 1u8);
        let t = h.call_sym_st0(GRAPHIC_GET_TIME, Conv::Cdecl, &[]);
        h.write(this + 4, x87::store(f64::from(t)));
        h.write(this + 0x15, 1u8);
    }

    /// `SkipPrompt_hide_006e78c0(this)`: `+4` = 0, `+0x14` = 0, `+8` =
    /// the time, `+0x16` = 1.
    pub fn skip_prompt_hide(h: &mut impl Host, this: u32) {
        h.write(this + 4, 0u32);
        h.write(this + 0x14, 0u8);
        let t = h.call_sym_st0(GRAPHIC_GET_TIME, Conv::Cdecl, &[]);
        h.write(this + 8, x87::store(f64::from(t)));
        h.write(this + 0x16, 1u8);
    }

    /// `RemapButtonIcon_setButtons_006e78e0(this; a, b, c)` (`ret 0xc`):
    /// `0x006e8390(this; button, &x, &y, &u, &v)` for the three slots at
    /// `+0x18`, `+0x34`, `+0x50`.
    pub fn icon_set_buttons(h: &mut impl Host, this: u32, buttons: [u32; 3]) {
        for (b, base) in buttons.into_iter().zip([0x18u32, 0x34, 0x50]) {
            let args = [
                b,
                this + base + 0xc,
                this + base + 0x10,
                this + base,
                this + base + 8,
            ];
            h.call_sym(ICON_SLOT_SET, Conv::Thiscall(this), &args);
        }
    }

    /// `StdMapIterator_dec_006e91f0(it)` -> `it` (`_Tree` iterator
    /// `operator--`; nodes `+0` left, `+4` parent, `+8` right, `+0xd` the
    /// nil byte): from the head (nil) to the last node; else to the
    /// rightmost node of the left subtree, or up while it is a left
    /// child, then to the parent where that stops (kept when it is the
    /// head).
    pub fn tree_dec<H: Host>(h: &mut H, it: u32) -> u32 {
        let nil = |h: &H, n: u32| h.read::<u8>(n + 0xd) != 0;
        let node = rd(h, it);
        if nil(h, node) {
            let r = rd(h, node + 8);
            h.write(it, r);
            return it;
        }
        let left = rd(h, node);
        if !nil(h, left) {
            let mut p = left;
            while !nil(h, rd(h, p + 8)) {
                p = rd(h, p + 8);
            }
            h.write(it, p);
            return it;
        }
        let mut p = rd(h, node + 4);
        while !nil(h, p) && rd(h, it) == rd(h, p) {
            h.write(it, p);
            p = rd(h, p + 4);
        }
        if !nil(h, rd(h, it)) {
            h.write(it, p);
        }
        it
    }

    // ---- the Iggy helpers and debug callbacks ----

    /// `g_TipSystem + 0xc` (the helper's list), the attention texture name
    /// (`"Loc_Texture_Saveload_Attention_B"`) and its id 0x1f9.
    pub const TIP_HELPERS: u32 = 0x00ce_6eb0;
    pub const ATTENTION_TEXTURE: u32 = 0x00b4_b4e0;
    pub const HELPER_FIND: u32 = 0x006e_8970;
    pub const HELPER_INSERT: u32 = 0x006e_8c80;
    /// `g_CameraData`.
    pub const CAMERA_DATA: u32 = 0x00ce_82ac;
    pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
    pub const UNLOCK_ACHIEVEMENT: u32 = 0x0082_2380;
    pub const DEBUG_FULL_ITEM: u32 = 0x0078_49d0;
    pub const DEBUG_FULL_COMMAND: u32 = 0x0078_48b0;
    pub const DEBUG_FULL_GILL: u32 = 0x0078_49c0;
    /// `iggy_w32` imports (stdcall): `IggyValueRefFree@8`,
    /// `IggyValueRefFromTempRef@12`, `IggyValuePathFromRef@12`,
    /// `IggyValuePathMakeArrayRef@12`, `IggyValuePathSetArrayIndex@8`,
    /// `IggyValueGetS32RS@16`.
    pub const VALUE_REF_FREE_IAT: u32 = 0x00b0_c820;
    pub const VALUE_REF_FROM_TEMP_IAT: u32 = 0x00b0_c828;
    pub const VALUE_PATH_FROM_REF_IAT: u32 = 0x00b0_c82c;
    pub const VALUE_PATH_ARRAY_IAT: u32 = 0x00b0_c830;
    pub const VALUE_PATH_SET_INDEX_IAT: u32 = 0x00b0_c834;
    pub const VALUE_GET_S32_IAT: u32 = 0x00b0_c838;
    pub const GET_DEREF: u32 = 0x00a2_8d10;

    /// `InitIggyHelper()`: the attention texture's record `{name, 0x1f9}`
    /// at `frame` (the original's `ebp - 8`) looked up in the tip helpers
    /// (`0x006e8970`) and inserted (`0x006e8c80(list; &rec, 0, found +
    /// 0x10, found)`); its `eax`.
    pub fn init_iggy_helper(h: &mut impl Host, frame: u32) -> u32 {
        let name = h.rebase(ATTENTION_TEXTURE);
        h.write(frame, name);
        h.write(frame + 4, 0x1f9u32);
        let list = h.rebase(TIP_HELPERS);
        let found = h.call_sym(HELPER_FIND, Conv::Thiscall(list), &[frame]);
        h.call_sym(
            HELPER_INSERT,
            Conv::Thiscall(list),
            &[frame, 0, found.wrapping_add(0x10), found],
        )
    }

    /// `UnloadCameraData()`: [`CAMERA_DATA`] deleted (`delete[]`) when set,
    /// then 0.
    pub fn unload_camera_data(h: &mut impl Host) {
        let p = rd(h, h.rebase(CAMERA_DATA));
        if p != 0 {
            h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[p]);
        }
        let at = h.rebase(CAMERA_DATA);
        h.write(at, 0u32);
    }

    /// `FFEscMenu_onDebugFill_006fc970(yes, &choice)`: on yes, choice 0
    /// fills the items, 1 unlocks achievements 0x19 and 0x18 and fills the
    /// commands, 2 fills the gil (tail jumps to `MsDebugFull*`); its
    /// `eax`.
    pub fn on_debug_fill(h: &mut impl Host, yes: u32, choice: u32) -> u32 {
        if yes == 0 {
            return yes;
        }
        match rd(h, choice) {
            0 => h.call_sym(DEBUG_FULL_ITEM, Conv::Cdecl, &[]),
            1 => {
                h.call_sym(UNLOCK_ACHIEVEMENT, Conv::Cdecl, &[0x19]);
                h.call_sym(UNLOCK_ACHIEVEMENT, Conv::Cdecl, &[0x18]);
                h.call_sym(DEBUG_FULL_COMMAND, Conv::Cdecl, &[])
            }
            2 => h.call_sym(DEBUG_FULL_GILL, Conv::Cdecl, &[]),
            c => c.wrapping_sub(2),
        }
    }

    /// `IggyValueRef_free_006fcc60(this)`: `IggyValueRefFree(+0, +4)`.
    pub fn value_ref_free(h: &mut impl Host, this: u32) -> u32 {
        let a = rd(h, this);
        let b = rd(h, this + 4);
        import(h, VALUE_REF_FREE_IAT, Conv::Stdcall, &[a, b])
    }

    /// `FFEscMenu_readS32Array12_006fed60(this; out, args)` (`ret 8`): the
    /// call's array argument (`args[1]`, a temp ref made a ref) read as 12
    /// `S32`s into `out` through a value path (`frame`, the original's
    /// `ebp - 0x34`: the path at `+0`, the array ref at `+0x18`); the ref
    /// freed.
    pub fn read_s32_array12(h: &mut impl Host, this: u32, out: u32, args: u32, frame: u32) -> u32 {
        let temp = rd(h, args + 4);
        let player = h.call_sym(GET_DEREF, Conv::Thiscall(this), &[]);
        let r = import(
            h,
            VALUE_REF_FROM_TEMP_IAT,
            Conv::Stdcall,
            &[player, temp, 1],
        );
        let player = h.call_sym(GET_DEREF, Conv::Thiscall(this), &[]);
        let path = frame;
        let arr = frame + 0x18;
        import(
            h,
            VALUE_PATH_FROM_REF_IAT,
            Conv::Stdcall,
            &[path, player, r],
        );
        import(h, VALUE_PATH_ARRAY_IAT, Conv::Stdcall, &[arr, path, 0]);
        for i in 0..12 {
            import(h, VALUE_PATH_SET_INDEX_IAT, Conv::Stdcall, &[arr, i]);
            import(
                h,
                VALUE_GET_S32_IAT,
                Conv::Stdcall,
                &[arr, 0, 0, out + 4 * i],
            );
        }
        let player = h.call_sym(GET_DEREF, Conv::Thiscall(this), &[]);
        import(h, VALUE_REF_FREE_IAT, Conv::Stdcall, &[player, r])
    }
}
