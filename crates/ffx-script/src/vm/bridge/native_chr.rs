//! The natives of groups 1, 4, 5, 6, 11 and 12 in Rust (script round 12)
//! that the earlier modules do not have: math (group 1: sine, cosine,
//! abs, angle wrap, distances), the screen and scene effects (group 4:
//! `Sg_*` fades, flashes, filters, focus, lights, footstep tables), the
//! worker's character model (group 5: `Ch_*`, motion groups, attach
//! points, neck, mouth, cloth), the rest of the camera and screen natives
//! (group 6, on `native_cam::cam`), the movie player (group 11) and the
//! system natives of group 12 (event mode, start map, save config). Most
//! pop their arguments and hand them to one game function; the character
//! ones act only when the worker has a model (`AtelGetCharHandle`), and
//! some pop only then.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::x87;

use super::native::Native;
use super::run::NativeHost;
use super::{
    atel_get_basic_work, atel_get_char_handle, atel_pop_stack_float, atel_pop_stack_integer,
};

fn pop(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_integer(h, s)
}

/// `AtelPopStackFloat` as `fstp m32` leaves it, loaded back (`fld m32`):
/// the `f32` bits a callee gets.
fn popf(h: &mut impl NativeHost, s: u32) -> u32 {
    x87::store(x87::load(atel_pop_stack_float(h, s)))
}

fn call(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call_n(f, args)
}

fn callf(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> f64 {
    let f = h.rebase(sym);
    h.call_float(f, args)
}

/// The worker's model (`AtelGetCharHandle`, 0 for none).
fn chr(h: &mut impl NativeHost, w: u32) -> u32 {
    atel_get_char_handle(h, w)
}

/// Worker `i`'s model.
fn chr_of(h: &mut impl NativeHost, i: u32) -> u32 {
    let w = atel_get_basic_work(h, i.cast_signed());
    atel_get_char_handle(h, w)
}

/// `x` rounded to `f32` and loaded back.
fn r32(x: f64) -> f64 {
    x87::load(x87::store(x))
}

/// `f(v)` for one popped integer; 0.
fn pop1(h: &mut impl NativeHost, s: u32, f: u32) -> u32 {
    let v = pop(h, s);
    call(h, f, &[v]);
    0
}

/// `f(b, a)` for two popped integers (`a` popped first); 0.
fn pop2(h: &mut impl NativeHost, s: u32, f: u32) -> u32 {
    let a = pop(h, s);
    let b = pop(h, s);
    call(h, f, &[b, a]);
    0
}

/// `f(model, v)` when the worker has a model, `v` popped either way; 0.
fn chr_set(h: &mut impl NativeHost, w: u32, s: u32, f: u32) -> u32 {
    let c = chr(h, w);
    let v = pop(h, s);
    if c != 0 {
        call(h, f, &[c, v]);
    }
    0
}

/// As [`chr_set`], popping `v` only for a model.
fn chr_set_if(h: &mut impl NativeHost, w: u32, s: u32, f: u32) -> u32 {
    let c = chr(h, w);
    if c != 0 {
        let v = pop(h, s);
        call(h, f, &[c, v]);
    }
    0
}

/// `f(model, v f)` when the worker has a model, `v` popped either way; 0.
fn chr_setf(h: &mut impl NativeHost, w: u32, s: u32, f: u32) -> u32 {
    let c = chr(h, w);
    let v = popf(h, s);
    if c != 0 {
        call(h, f, &[c, v]);
    }
    0
}

/// `(worker, v)`: `f(worker's model, v)` for a model; 0.
fn other_set(h: &mut impl NativeHost, s: u32, f: u32) -> u32 {
    let v = pop(h, s);
    let i = pop(h, s);
    let c = chr_of(h, i);
    if c != 0 {
        call(h, f, &[c, v]);
    }
    0
}

/// `(v f)` -> `ST0` of `f(model, v)`, 0.0 without a model.
fn chr_getf(h: &mut impl NativeHost, w: u32, s: u32, f: u32) -> f64 {
    let c = chr(h, w);
    let v = popf(h, s);
    if c == 0 {
        return 0.0;
    }
    callf(h, f, &[c, v])
}

/// `(v f)` -> `ST0` of `f(v)`.
fn mathf(h: &mut impl NativeHost, s: u32, f: u32) -> f64 {
    let v = popf(h, s);
    callf(h, f, &[v])
}

macro_rules! natives {
    ($($(#[$m:meta])* $name:ident($h:ident, $w:ident, $a:ident, $s:ident) => $e:expr;)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>($h: &mut H, $w: u32, $a: u32, $s: u32) -> u32 {
            let _ = (&$h, $w, $a, $s);
            $e
        }
    )*};
}

natives! {
    /// `0x5061` (`0x00a78f80`): `(b, a)`: `Ch_MgrpRead(b, a)` (`0x00836a40`).
    native_5061(h, w, a, s) => pop2(h, s, 0x0083_6a40);
    /// `0x401a` (`0x00a781a0`): `(b, a)`: `Sg_AccSetAlpha(b, a)` (`0x0082bd10`).
    native_401a(h, w, a, s) => pop2(h, s, 0x0082_bd10);
    /// `0x4017` (`0x00a78130`): `(b, a)`: `Sg_IdSetMapTable(b, a)` (`0x0083e9e0`).
    native_4017(h, w, a, s) => pop2(h, s, 0x0083_e9e0);
    /// `Native_FootSetMaterial` (`0x00a77b30`, `0x403b`): `(index,
    /// material)`: `Sg_IdSetEffectTable(index, material)` (`0x0083e9b0`).
    native_foot_set_material(h, w, a, s) => pop2(h, s, 0x0083_e9b0);
    /// `0x4015` (`0x00a780c0`): `(b, a)`: `Sg_SetFocus(b, a)` (`0x0082ce20`).
    native_4015(h, w, a, s) => pop2(h, s, 0x0082_ce20);
    /// `0x503e` (`0x00a7a570`): `(b, a)`: `ChN_DataDispose(b, a)` (`0x00824f00`).
    native_503e(h, w, a, s) => pop2(h, s, 0x0082_4f00);
    /// `0x503d` (`0x00a7a520`): `(v)`: `ChN_DataDispose(v, 1)`.
    native_503d(h, w, a, s) => {
        let v = pop(h, s);
        call(h, 0x0082_4f00, &[v, 1]);
        0
    };
    /// `0x4019` (`0x00a78180`): `(v)`: `0x0082bdf0(v)`.
    native_4019(h, w, a, s) => pop1(h, s, 0x0082_bdf0);
    /// `0x5058` (`0x00a78b70`): `(v)`: `Ch_AllSetCalcNorm(v)` (`0x008325b0`).
    native_5058(h, w, a, s) => pop1(h, s, 0x0083_25b0);
    /// `0x503c` (`0x00a7a4d0`): `(v)`: `Ch_DisposeMGRP(v)` (`0x00836ad0`).
    native_503c(h, w, a, s) => pop1(h, s, 0x0083_6ad0);
    /// `0x5065` (`0x00a79420`): `(v)`: `0x00837650(v)` (motion group read
    /// start; the original's `dbgPrintf` is left out).
    native_5065(h, w, a, s) => pop1(h, s, 0x0083_7650);
    /// `0x4014` (`0x00a780a0`): `(v)` -> `Sg_EnFocus(v)` (`0x0082cb60`).
    native_4014(h, w, a, s) => {
        let v = pop(h, s);
        call(h, 0x0082_cb60, &[v])
    };
    /// `0x4007` (`0x00a77de0`): `(v)`: `Sg_Fade_Common(v, 0, 1)` (`0x0082cdc0`).
    native_4007(h, w, a, s) => {
        let v = pop(h, s);
        call(h, 0x0082_cdc0, &[v, 0, 1]);
        0
    };
    /// `0x4006` (`0x00a77dc0`): `(v)`: `Sg_Fade_Common(v, 1, 1)`.
    native_4006(h, w, a, s) => {
        let v = pop(h, s);
        call(h, 0x0082_cdc0, &[v, 1, 1]);
        0
    };
    /// `Atel_Sg_Flash` (`0x00a77d30`, `0x4003`, Fahrenheit's name): `(d, c,
    /// b, a)`: `Sg_Flash(d, c, b, a)` (`0x0082cca0`).
    atel_sg_flash(h, w, a, s) => {
        let a = pop(h, s);
        let b = pop(h, s);
        let c = pop(h, s);
        let d = pop(h, s);
        call(h, 0x0082_cca0, &[d, c, b, a]);
        0
    };
    /// `0x400c` (`0x00a77ed0`): `0x0082cc70()`.
    native_400c(h, w, a, s) => {
        call(h, 0x0082_cc70, &[]);
        0
    };
    /// `0x4020` (`0x00a782f0`): `0x00840e50()`.
    native_4020(h, w, a, s) => {
        call(h, 0x0084_0e50, &[]);
        0
    };
    /// `Native_SetControlAngle` (`0x00a77c30`, `0x4040`): `(degrees f)`:
    /// `0x0082d950(degrees)`.
    native_set_control_angle(h, w, a, s) => {
        let v = popf(h, s);
        call(h, 0x0082_d950, &[v]);
        0
    };
    /// `0x4001` (`0x00a77b90`): `(v f)`: `Sg_SetSuimen(v)` (`0x00836640`,
    /// the water level).
    native_4001(h, w, a, s) => {
        let v = popf(h, s);
        call(h, 0x0083_6640, &[v]);
        0
    };
    /// `0x4016` (`0x00a780f0`): `(i, v f)`: `0x0082d9e0(i, v)`.
    native_4016(h, w, a, s) => {
        let v = popf(h, s);
        let i = pop(h, s);
        call(h, 0x0082_d9e0, &[i, v]);
        0
    };
    /// `0x5076` (`0x00a79e30`): `(on)`: `Ch_MouthSetAutoTalk(model, on)`
    /// (`0x00834770`), popped only for a model.
    native_5076(h, w, a, s) => chr_set_if(h, w, s, 0x0083_4770);
    /// `0x506f` (`0x00a79b90`): `(v)`: `Ch_SetHideCalc(model, v)`
    /// (`0x0082b280`), popped only for a model.
    native_506f(h, w, a, s) => chr_set_if(h, w, s, 0x0082_b280);
    /// `0x5079` (`0x00a79f70`): `(on)`: `Ch_NeckSetEnable(model, on)` (`0x00834f60`).
    native_5079(h, w, a, s) => chr_set(h, w, s, 0x0083_4f60);
    /// `0x5018` (`0x00a791e0`): `(v)`: `Ch_SetMotionSpeed(model, v)` (`0x0082b380`).
    native_5018(h, w, a, s) => chr_set(h, w, s, 0x0082_b380);
    /// `0x505f` (`0x00a78e90`): `(v)`: `Ch_MotionSetLoop(model, v)` (`0x00839960`).
    native_505f(h, w, a, s) => chr_set(h, w, s, 0x0083_9960);
    /// `0x500d` (`0x00a78df0`): `(v)`: `Ch_SetHideEvent(model, v)` (`0x0082b2c0`).
    native_500d(h, w, a, s) => chr_set(h, w, s, 0x0082_b2c0);
    /// `0x5006` (`0x00a78a40`): `(scale f)`: `Ch_SetScaleAll(model, scale)` (`0x0082b590`).
    native_5006(h, w, a, s) => chr_setf(h, w, s, 0x0082_b590);
    /// `0x500b` (`0x00a78b90`): `(v f)`: `Ch_SetJimen(model, v)` (`0x0082b360`).
    native_500b(h, w, a, s) => chr_setf(h, w, s, 0x0082_b360);
    /// `0x5050` (`0x00a78710`): `(worker, v)`: `Ch_MotionSet737(model, v)` (`0x00837570`).
    native_5050(h, w, a, s) => other_set(h, s, 0x0083_7570);
    /// `0x5047` (`0x00a7a900`): `(worker, v)`: `Ch_SetSequence(model, v)` (`0x00837ac0`).
    native_5047(h, w, a, s) => other_set(h, s, 0x0083_7ac0);
    /// `0x504c` (`0x00a786a0`): `(worker, v)`: `Ch_SetMotionSpeed(model, v)`.
    native_504c(h, w, a, s) => other_set(h, s, 0x0082_b380);
    /// `0x5013` (`0x00a78ec0`): `(mode)`: `Ch_SetFieldMode(model, mode)`,
    /// `Ch_SetMotionType(model, mode)` (`0x0082b1e0`, `0x0082b3a0`).
    native_5013(h, w, a, s) => {
        let c = chr(h, w);
        let v = pop(h, s);
        if c != 0 {
            call(h, 0x0082_b1e0, &[c, v]);
            call(h, 0x0082_b3a0, &[c, v]);
        }
        0
    };
    /// `0x5056` (`0x00a78a80`): `(v)` -> `Ch_SetCalcNorm(model, v)`
    /// (`0x00835750`), 0 without a model.
    native_5056(h, w, a, s) => {
        let c = chr(h, w);
        let v = pop(h, s);
        if c == 0 {
            return 0;
        }
        call(h, 0x0083_5750, &[c, v])
    };
    /// `0x5054` (`0x00a78960`): `(part, on)` -> `Ch_ModelSetHide(model,
    /// part, on)` (`0x0082aea0`), 0 without a model.
    native_5054(h, w, a, s) => {
        let on = pop(h, s);
        let part = pop(h, s);
        let c = chr(h, w);
        if c == 0 {
            return 0;
        }
        call(h, 0x0082_aea0, &[c, part, on])
    };
    /// `0x507a` (`0x00a79fc0`): `(worker)`: `Ch_NeckSetTarget(model, that
    /// worker's model)` (`0x00835050`).
    native_507a(h, w, a, s) => {
        let c = chr(h, w);
        let i = pop(h, s);
        let t = chr_of(h, i);
        if c != 0 {
            call(h, 0x0083_5050, &[c, t]);
        }
        0
    };
    /// `0x5078` (`0x00a79ef0`): `(b, a)`: `Ch_TextureSetAnim(model, b, a)` (`0x0083d040`).
    native_5078(h, w, a, s) => {
        let c = chr(h, w);
        let a = pop(h, s);
        let b = pop(h, s);
        if c != 0 {
            call(h, 0x0083_d040, &[c, b, a]);
        }
        0
    };
    /// `0x5015` (`0x00a78fb0`): `(r f, g f, b f, n)`: `Ch_SetShadeCol(model,
    /// r, g, b, n)` (`0x0082b5d0`).
    native_5015(h, w, a, s) => {
        let c = chr(h, w);
        let n = pop(h, s);
        let b = popf(h, s);
        let g = popf(h, s);
        let r = popf(h, s);
        if c != 0 {
            call(h, 0x0082_b5d0, &[c, r, g, b, n]);
        }
        0
    };
    /// `0x508f` (`0x00a7a8a0`): `(v)`: the model's `+0x870` byte, popped
    /// only for a model.
    native_508f(h, w, a, s) => {
        let c = chr(h, w);
        if c != 0 {
            let v = pop(h, s);
            h.write(c + 0x870, v.to_le_bytes()[0]);
        }
        0
    };
    /// `Native_ChrAttach` (`0x00a79c50`, `0x502d`): `(worker, point)`:
    /// `Ch_BindToElem(model, that worker's model, point)` (`0x00832630`);
    /// worker -1 detaches (`Ch_BindToElem(model, 0, 0)`).
    native_chr_attach(h, w, a, s) => {
        let c = chr(h, w);
        let p = pop(h, s);
        let i = pop(h, s);
        if c == 0 {
            return 0;
        }
        if i == u32::MAX {
            call(h, 0x0083_2630, &[c, 0, 0]);
        } else {
            let t = chr_of(h, i);
            call(h, 0x0083_2630, &[c, t, p]);
        }
        0
    };
    /// `0x5044` (`0x00a7a810`): `Native_PlayMotionCommon(worker, stack, 1,
    /// 1)` (`0x00a7aae0`).
    native_5044(h, w, a, s) => {
        native_play_motion_common(h, w, s, 1, 1);
        0
    };
    /// `0x1000` (`0x008779c0`): `(slot, a, b, v)`: the current ctrl work's
    /// slot (`+0xa4`, 8 bytes, clamped to 0..31) = `a / 8 | 0x80`, `b / 4`,
    /// the worker's `+0x2e`, v for non-negative a and b, else its first two
    /// bytes cleared; 1.
    native_1000(h, w, a, s) => {
        let v = pop(h, s);
        let b = pop(h, s).cast_signed();
        let a = pop(h, s).cast_signed();
        let i = pop(h, s).cast_signed().clamp(0, 0x1f).cast_unsigned();
        let p = call(h, 0x0086_aff0, &[]).wrapping_add(0xa4).wrapping_add(i * 8);
        if a < 0 || b < 0 {
            h.write(p, 0_u16);
            return 1;
        }
        let [x, ..] = (a >> 3).to_le_bytes();
        let [y, ..] = (b >> 2).to_le_bytes();
        h.write(p, x & 0x3f | 0x80);
        h.write(p + 1, y & 0x7f);
        let id = h.read::<u16>(w + 0x2e);
        h.write(p + 2, id);
        h.write(p + 4, v);
        1
    };
    /// `Native_ChrLoadSystemMgrp` (`0x00a79820`, `0x5010`, start): `(chr
    /// id, n)` into the args words (`+0` id, `+4` n); `EAX` the id (what
    /// the left-out `dbgPrintf` keeps).
    native_chr_load_system_mgrp(h, w, a, s) => {
        let n = pop(h, s);
        let id = pop(h, s);
        h.write(a + 4, n);
        h.write(a, id);
        id
    };
}

/// `Native_ChrLoadSystemMgrp_Result` (`0x00a79980`, `0x5010`, int):
/// `ChN_ReadSystemMGRP(id, n)` (`0x00836870`) from the args words; 0.
pub fn native_chr_load_system_mgrp_result<H: NativeHost>(
    h: &mut H,
    _w: u32,
    a: u32,
    _s: u32,
) -> u32 {
    let id = h.read::<u32>(a);
    let n = h.read::<u32>(a + 4);
    call(h, 0x0083_6870, &[id, n]);
    0
}

/// The polls that are `mov eax, 5; ret` (`0x5010` `0x00a79970`, `0x5001`
/// `0x00a79a10`, `0xb000` `0x0076e980`).
pub fn native_done<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    5
}

/// The starts that are a bare `ret` (`0x400d` `0x00a77ee0`, `0xb009`
/// `0x0076eb30`): `EAX` is the caller's, which the dispatch ignores.
pub fn native_nothing<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `Native_ChrLoadMotionGroup` (`0x00a799b0`, `0x5001`, start): `(n)`
/// into the args word; `EAX` n.
pub fn native_chr_load_motion_group<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    h.write(a, n);
    n
}

/// `Native_ChrLoadMotionGroup_Result` (`0x00a79a20`, `0x5001`, int):
/// `Chr_LoadMotionGroup(n)` (`0x008375f0`); 0.
pub fn native_chr_load_motion_group_result<H: NativeHost>(
    h: &mut H,
    _w: u32,
    a: u32,
    _s: u32,
) -> u32 {
    let n = h.read::<u32>(a);
    call(h, 0x0083_75f0, &[n]);
    0
}

/// `0x504f` (`0x00a79340`, start): `(slot)`: args `+0` = slot, `+8` and
/// `+0xc` = 0; `EAX` the slot.
pub fn native_504f<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    h.write(a + 0xc, 0_u32);
    h.write(a + 8, 0_u32);
    let v = pop(h, s);
    h.write(a, v);
    v
}

/// `0x504f` (`0x00a79380`, poll): `Ch_DataReadSync(slot)` (`0x0082a040`):
/// with args `+8` set its result into `+0xc` and 5, else 1 when it is 0
/// (`neg; sbb; inc`), 0 otherwise.
pub fn native_504f_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let v = h.read::<u32>(a);
    let r = call(h, 0x0082_a040, &[v]);
    if h.read::<u32>(a + 8) != 0 {
        h.write(a + 0xc, r);
        return 5;
    }
    u32::from(r == 0)
}

/// `0x504f`, `0x5063` (`0x00a79410`, `0x00a79210`, int): the args word
/// `+0xc`.
pub fn native_args_c<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    h.read(a + 0xc)
}

/// `0x5063` (`0x00a79160`, start): `(id, n)`: args `+0` = id, `+4` = n,
/// `+8` and `+0xc` = 0; `EAX` the id.
pub fn native_5063<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    h.write(a + 0xc, 0_u32);
    h.write(a + 8, 0_u32);
    let n = pop(h, s);
    h.write(a + 4, n);
    let id = pop(h, s);
    h.write(a, id);
    id
}

/// `0x5063` (`0x00a791a0`, poll): as [`native_504f_poll`] with
/// `ChN_ReadSystemMGRPsync(id, n)` (`0x00836a50`).
pub fn native_5063_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let id = h.read::<u32>(a);
    let n = h.read::<u32>(a + 4);
    let r = call(h, 0x0083_6a50, &[id, n]);
    if h.read::<u32>(a + 8) != 0 {
        h.write(a + 0xc, r);
        return 5;
    }
    u32::from(r == 0)
}

/// `Native_FadeWait_Poll` (`0x00a77ef0`, `0x400d`): 1 when `0x0082cc10()`
/// is 0, else 0.
pub fn native_fade_wait_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    u32::from(call(h, 0x0082_cc10, &[]) == 0)
}

/// `Native_ChrSetHitRadiusScale` (`0x00a7a340`, `0x5039`, float): `(scale
/// f)` -> `Ch_SetThickness(model, scale)` (`0x0082b880`), 0.0 without a
/// model.
pub fn native_chr_set_hit_radius_scale<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    chr_getf(h, w, s, 0x0082_b880)
}

/// `0x503a` (`0x00a7a3d0`, float): `(z f)` -> `Ch_SetClipZ(model, z)`
/// (`0x0082b150`), 0.0 without a model.
pub fn native_503a<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    chr_getf(h, w, s, 0x0082_b150)
}

/// `AtelCall_Math_Sin_RF` (`0x00877a70`, `0x1001`, float, Fahrenheit's
/// name): `CIsin(v)` (`0x0088c3a0`).
pub fn atel_call_math_sin_rf<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    mathf(h, s, 0x0088_c3a0)
}

/// `AtelCall_Math_Cos_RF` (`0x00877aa0`, `0x1002`, float, Fahrenheit's
/// name): `CIcos(v)` (`0x0088b3e0`).
pub fn atel_call_math_cos_rf<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    mathf(h, s, 0x0088_b3e0)
}

/// `0x1013` (`0x00877d30`, float): `Math_WrapAngle(v)` (`0x0088bc20`).
pub fn native_1013<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    mathf(h, s, 0x0088_bc20)
}

/// `0x101a` (`0x00877ff0`, float): `(x1, z1, x2, z2)` floats ->
/// `sqrt((z2 - z1)² + (x2 - x1)²)`, each difference and the result
/// rounded to `f32`.
pub fn native_101a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let p1 = x87::load(atel_pop_stack_float(h, s));
    let p2 = x87::load(atel_pop_stack_float(h, s));
    let p3 = x87::load(atel_pop_stack_float(h, s));
    let p4 = r32(x87::load(atel_pop_stack_float(h, s)));
    let y = r32(x87::sub(p4, p2));
    let x = r32(x87::sub(p3, p1));
    r32(x87::sqrt(x87::add(x87::mul(y, y), x87::mul(x, x))))
}

/// `0x101b` (`0x00878060`, float): the 3D distance of two popped points,
/// as `0x101a`.
pub fn native_101b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let p1 = x87::load(atel_pop_stack_float(h, s));
    let p2 = x87::load(atel_pop_stack_float(h, s));
    let p3 = x87::load(atel_pop_stack_float(h, s));
    let p4 = x87::load(atel_pop_stack_float(h, s));
    let p5 = x87::load(atel_pop_stack_float(h, s));
    let p6 = r32(x87::load(atel_pop_stack_float(h, s)));
    let z = r32(x87::sub(p6, p3));
    let y = r32(x87::sub(p5, p2));
    let x = r32(x87::sub(p4, p1));
    let q = x87::add(x87::add(x87::mul(z, z), x87::mul(y, y)), x87::mul(x, x));
    r32(x87::sqrt(q))
}

/// `g_WaitFramesByTime` (`0x0112a008`).
const WAIT_FRAMES_BY_TIME: u32 = 0x0112_a008;
/// The movie being played (`0x00c40e28`, -1 for none).
const MOVIE: u32 = 0x00c4_0e28;
/// Set when a movie natives' work is done (`0x00c40e2c`).
const MOVIE_DONE: u32 = 0x00c4_0e2c;

fn set(h: &mut impl NativeHost, at: u32, v: u32) {
    let p = h.rebase(at);
    h.write(p, v);
}

/// The movie camera teardown of `0xb001`/`0xb004`: with a movie camera
/// (`movie_have_camera`, `0x0076efc0`), `MsSetCameraMatrix(0, 0, 0, 0, 0,
/// 0.0)` (`0x007c06d0`), `0x00821a20(0)`, `0x006f6f80()`.
fn movie_camera_off(h: &mut impl NativeHost) {
    if call(h, 0x0076_efc0, &[]) != 0 {
        call(h, 0x007c_06d0, &[0, 0, 0, 0, 0, x87::store(0.0)]);
        call(h, 0x0082_1a20, &[0]);
        call(h, 0x006f_6f80, &[]);
    }
}

/// `0xb000` (`0x0076e910`, start): `(movie, flags)`: args `+0` = flags,
/// `+4` = movie (also `0x00c40e28`); `graphicInitFMVPlayer(movie, 0)`
/// (`0x00641690`), `0x006427c0(1)`, with flags bit `0x2000`
/// `graphicSetPauseVideoAtEndFrame(1)`; the done flag = 1,
/// `g_WaitFramesByTime` = 0. `EAX` is the last call's.
pub fn native_b000<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let flags = pop(h, s);
    let m = pop(h, s);
    h.write(a, flags);
    h.write(a + 4, m);
    set(h, MOVIE, m);
    call(h, 0x0064_1690, &[m, 0]);
    let mut r = call(h, 0x0064_27c0, &[1]);
    if flags & 0x2000 != 0 {
        r = call(h, 0x0064_37f0, &[1]);
    }
    set(h, MOVIE_DONE, 1);
    set(h, WAIT_FRAMES_BY_TIME, 0);
    r
}

/// `0xb000` (`0x0076e990`, int): `movie_start_prog(movie, flags)`
/// (`0x0076f080`) from the args words; 0.
pub fn native_b000_result<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let m = h.read::<u32>(a + 4);
    let flags = h.read::<u32>(a);
    call(h, 0x0076_f080, &[m, flags]);
    0
}

/// `0xb001` (`0x0076e9b0`, start): the args word = 0; `EAX` its address.
pub fn native_b001<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    h.write(a, 0_u32);
    a
}

/// `0xb001` (`0x0076e9c0`, poll): 0 while `graphicIsVideoPlaying()`
/// (`0x00641cf0`) or `0x00641ce0()`; then `g_WaitFramesByTime` = 0, the
/// done flag = 1, `graphicDestroyFmv()` (`0x0063df30`) and 5.
pub fn native_b001_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if call(h, 0x0064_1cf0, &[]) != 0 || call(h, 0x0064_1ce0, &[]) != 0 {
        return 0;
    }
    set(h, WAIT_FRAMES_BY_TIME, 0);
    set(h, MOVIE_DONE, 1);
    call(h, 0x0063_df30, &[]);
    5
}

/// `0xb001` (`0x0076e9f0`, int): the movie camera teardown, the movie =
/// -1; 0.
pub fn native_b001_result<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    movie_camera_off(h);
    set(h, MOVIE, u32::MAX);
    0
}

/// `0xb003` (`0x0076ea40`): jumps to `graphicGetCurrentFrame`
/// (`0x006410d0`).
pub fn native_b003<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    call(h, 0x0064_10d0, &[w, a, s])
}

/// `0xb004` (`0x0076ea50`, start): `graphicSetPauseVideoAtEndFrame(0)`'s
/// `EAX`.
pub fn native_b004<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0064_37f0, &[0])
}

/// `0xb004` (`0x0076ea60`, poll): `0x00643e50()`, the movie camera
/// teardown, the movie = -1, `g_WaitFramesByTime` = 0, the done flag = 1;
/// 1.
pub fn native_b004_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0064_3e50, &[]);
    movie_camera_off(h);
    set(h, MOVIE, u32::MAX);
    set(h, WAIT_FRAMES_BY_TIME, 0);
    set(h, MOVIE_DONE, 1);
    1
}

/// `0xb004` (`0x0076eac0`, int): `graphicDestroyFmv()`; 0.
pub fn native_b004_result<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0063_df30, &[]);
    0
}

/// `0xb009` (`0x0076eb40`, poll): 5 once `0x00641ce0()` is 0 and
/// `0x00645dc0()` is not, else 0.
pub fn native_b009_poll<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if call(h, 0x0064_1ce0, &[]) != 0 || call(h, 0x0064_5dc0, &[]) == 0 {
        return 0;
    }
    5
}

/// `0xb009` (`0x0076eb60`, int): `g_WaitFramesByTime` = 1,
/// `0x006427c0(0)`, `0x0076edc0()`; 0.
pub fn native_b009_result<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    set(h, WAIT_FRAMES_BY_TIME, 1);
    call(h, 0x0064_27c0, &[0]);
    call(h, 0x0076_edc0, &[]);
    0
}

// ---- the rest, by call count ----

/// `f64::from(3.141_592_f32)`: the `fmul qword` constant at `0x00b582f0`
/// of the degree conversions (`* pi / 180`, `180.0` at `0x00b48378`).
const PI_B582F0: f64 = 3.141_592_025_756_836;
/// `f64::from(core::f32::consts::PI)` (`0x00b92288`).
const PI_F32: f64 = 3.141_592_741_012_573_2;

/// `x * PI_B582F0 / 180` rounded to `f32`, then wrapped to (-pi, pi] by
/// `0x00a7ae20` (`ST0`): the `f32` bits of the result.
fn wrap_degrees(h: &mut impl NativeHost, x: u32) -> u32 {
    let r = r32(x87::div(x87::mul(x87::load(x), PI_B582F0), 180.0));
    let w = callf(h, 0x00a7_ae20, &[x87::store(r)]);
    x87::store(w)
}

/// The worker's camera (`native_cam::cam`).
fn cam(h: &mut impl NativeHost, w: u32) -> u32 {
    super::native_cam::cam(h, w)
}

/// `f(worker, args, stack, k...)`'s `EAX`.
fn fwd(h: &mut impl NativeHost, f: u32, w: u32, a: u32, s: u32, k: &[u32]) -> u32 {
    let mut v = [w, a, s, 0, 0, 0, 0, 0];
    let n = 3 + k.len().min(5);
    v[3..n].copy_from_slice(&k[..n - 3]);
    call(h, f, &v[..n])
}

/// `n` popped integers, the first popped last (push order).
fn pops<const N: usize>(h: &mut impl NativeHost, s: u32) -> [u32; N] {
    let mut v = [0; N];
    for i in (0..N).rev() {
        v[i] = pop(h, s);
    }
    v
}

/// The `0x5049`/`0x501f`/`0x504a` test: 1 when the model's `+0x736` is
/// set (0 without a model).
fn busy(h: &impl NativeHost, c: u32) -> u32 {
    u32::from(c != 0 && h.read::<u8>(c + 0x736) != 0)
}

natives! {
    /// `0x506d` (`0x00a79a80`): `(a f, b f)`: `Ch_SetRotX(model,
    /// wrap(b degrees), a)` (`0x008357f0`).
    native_506d(h, w, a, s) => {
        let c = chr(h, w);
        let x = popf(h, s);
        let y = popf(h, s);
        if c != 0 {
            let r = wrap_degrees(h, y);
            call(h, 0x0083_57f0, &[c, r, x]);
        }
        0
    };
    /// `0x507d` (`0x00a7a090`): `(b f, a f)` degrees: `Ch_NeckSetRot(model,
    /// wrap(b), wrap(a))` (`0x00834ff0`).
    native_507d(h, w, a, s) => {
        let c = chr(h, w);
        let x = popf(h, s);
        let y = popf(h, s);
        if c != 0 {
            let rx = wrap_degrees(h, x);
            let ry = wrap_degrees(h, y);
            call(h, 0x0083_4ff0, &[c, ry, rx]);
        }
        0
    };
    /// `0x5066` (`0x00a79480`, start): `(n)`: args `+0` = n, `+0xc` = 0;
    /// `EAX` n.
    native_5066(h, w, a, s) => {
        h.write(a + 0xc, 0_u32);
        let n = pop(h, s);
        h.write(a, n);
        n
    };
    /// `0x5066` (`0x00a794d0`, poll): 1 once `0x00837670(n)` is 0 (the
    /// `dbgPrintf` left out), else 0.
    native_5066_poll(h, w, a, s) => {
        let n = h.read::<u32>(a);
        u32::from(call(h, 0x0083_7670, &[n]) == 0)
    };
    /// `0x5084` (`0x00a7a5d0`): `(v f)`: `Ch_NeckSetSpeed(model, v)` (`0x00835020`).
    native_5084(h, w, a, s) => {
        let v = popf(h, s);
        let c = chr(h, w);
        if c != 0 {
            call(h, 0x0083_5020, &[c, v]);
        }
        0
    };
    /// `0x5085` (`0x00a7a640`): `(v f)`: `Ch_ShadowSetHeight(model, v)` (`0x00830cb0`).
    native_5085(h, w, a, s) => {
        let v = popf(h, s);
        let c = chr(h, w);
        if c != 0 {
            call(h, 0x0083_0cb0, &[c, v]);
        }
        0
    };
    /// `0x5080` (`0x00a7a410`): `(v f)`: `Ch_SetHiju(model, v)` (`0x0082b340`).
    native_5080(h, w, a, s) => {
        let v = popf(h, s);
        let c = chr(h, w);
        if c != 0 {
            call(h, 0x0082_b340, &[c, v]);
        }
        0
    };
    /// `0x5014` (`0x00a78f30`): `(v f, n)`: `Ch_SetShade(model, v, n)` (`0x0082b5b0`).
    native_5014(h, w, a, s) => {
        let c = chr(h, w);
        let n = pop(h, s);
        let v = popf(h, s);
        if c != 0 {
            call(h, 0x0082_b5b0, &[c, v, n]);
        }
        0
    };
    /// `0x5088` (`0x00a7a6b0`): `(v f, n)`: `Ch_SetShadeID(model, v, n)` (`0x0082b720`).
    native_5088(h, w, a, s) => {
        let c = chr(h, w);
        let n = pop(h, s);
        let v = popf(h, s);
        if c != 0 {
            call(h, 0x0082_b720, &[c, v, n]);
        }
        0
    };
    /// `0x503b` (`0x00a7a450`): `(b, a)`: `ChN_DisposeSystemMGRP(b, a)`
    /// (`0x00836830`) when the worker has a model.
    native_503b(h, w, a, s) => {
        let c = chr(h, w);
        let x = pop(h, s);
        let y = pop(h, s);
        if c != 0 {
            call(h, 0x0083_6830, &[y, x]);
        }
        0
    };
    /// `0x1015` (`0x00877dc0`): six floats: `0x0088b780(f6, ..., f1)`'s `EAX`.
    native_1015(h, w, a, s) => {
        let mut f = [0; 6];
        for i in (0..6).rev() {
            f[i] = popf(h, s);
        }
        call(h, 0x0088_b780, &f)
    };
    /// `0x4008` (`0x00a77e30`): `(d, c, b, a)`: `Sg_FilterAdd(d, c, b, a)` (`0x0082cc20`).
    native_4008(h, w, a, s) => {
        let v = pops::<4>(h, s);
        call(h, 0x0082_cc20, &v);
        0
    };
    /// `0x4009` (`0x00a77e80`): `(d, c, b, a)`: `Sg_FilterSub(d, c, b, a)` (`0x0082cc80`).
    native_4009(h, w, a, s) => {
        let v = pops::<4>(h, s);
        call(h, 0x0082_cc80, &v);
        0
    };
    /// `0x5007` (`0x00a789a0`): `(n, x f, y f, z f)`: `Ch_SetScale(model,
    /// n, x, y, z)` (`0x0082b560`).
    native_5007(h, w, a, s) => {
        let c = chr(h, w);
        let z = popf(h, s);
        let y = popf(h, s);
        let x = popf(h, s);
        let n = pop(h, s);
        if c != 0 {
            call(h, 0x0082_b560, &[c, n, x, y, z]);
        }
        0
    };
    /// `0x5049` (`0x00a785f0`, start): `(worker)`: the args word = that
    /// worker's model, also `EAX`.
    native_5049(h, w, a, s) => {
        let i = pop(h, s);
        let c = chr_of(h, i);
        h.write(a, c);
        c
    };
    /// `0x5049` (`0x00a78620`, poll): 0 while the model in the args word
    /// has `+0x736` set (or there is none), else 5.
    native_5049_poll(h, w, a, s) => {
        let c = h.read::<u32>(a);
        if c == 0 {
            return 0;
        }
        if h.read::<u8>(c + 0x736) == 0 { 5 } else { 0 }
    };
    /// `0x501f` (`0x00a79450`): 1 while the worker's model has `+0x736` set.
    native_501f(h, w, a, s) => {
        let c = chr(h, w);
        busy(h, c)
    };
    /// `0x504a` (`0x00a78640`): `(worker)`: as `0x501f` for that worker.
    native_504a(h, w, a, s) => {
        let i = pop(h, s);
        let c = chr_of(h, i);
        busy(h, c)
    };
    /// `0x504b` (`0x00a78670`): `(worker)`: `Ch_ResetMotion` (`0x0082b080`)
    /// on that worker's model.
    native_504b(h, w, a, s) => {
        let i = pop(h, s);
        let c = chr_of(h, i);
        if c != 0 {
            call(h, 0x0082_b080, &[c]);
        }
        0
    };
    /// `0x401f` (`0x00a782e0`): `0x00840e40()`.
    native_401f(h, w, a, s) => {
        call(h, 0x0084_0e40, &[]);
        0
    };
    /// `0x400b` (`0x00a77e20`): `0x0082cc00()`.
    native_400b(h, w, a, s) => {
        call(h, 0x0082_cc00, &[]);
        0
    };
    /// `0x4024` (`0x00a78360`): `0x00831a30()`.
    native_4024(h, w, a, s) => {
        call(h, 0x0083_1a30, &[]);
        0
    };
    /// `0x403f` (`0x00a77c20`): `0x0082d930()`.
    native_403f(h, w, a, s) => {
        call(h, 0x0082_d930, &[]);
        0
    };
    /// `0x5081` (`0x00a7a4a0`): `(v)`: `Ch_SetChocobo(model, v)`
    /// (`0x0082b130`), popped only for a model.
    native_5081(h, w, a, s) => chr_set_if(h, w, s, 0x0082_b130);
    /// `0x5082` (`0x00a7a4f0`): `(v)`: `Ch_ClothSetEnable(model, v)`
    /// (`0x0082a930`), popped only for a model.
    native_5082(h, w, a, s) => chr_set_if(h, w, s, 0x0082_a930);
    /// `0x5073` (`0x00a79d30`): `(v)`: `Ch_SetOverlapHit(model, v)`
    /// (`0x0082b420`), popped only for a model.
    native_5073(h, w, a, s) => chr_set_if(h, w, s, 0x0082_b420);
    /// `0x5083` (`0x00a7a540`): `(v)`: `Ch_Vu1SetClipmode(model, v)`
    /// (`0x0082b970`), popped only for a model.
    native_5083(h, w, a, s) => chr_set_if(h, w, s, 0x0082_b970);
    /// `0x5060` (`0x00a78f00`): `(v)`: `Ch_IdEnableShade(model, v)` (`0x008340f0`).
    native_5060(h, w, a, s) => chr_set(h, w, s, 0x0083_40f0);
    /// `0x508e` (`0x00a7a850`): `(v)`: `Ch_ClothEnableGrav(model, v)` (`0x008245f0`).
    native_508e(h, w, a, s) => chr_set(h, w, s, 0x0082_45f0);
    /// `0x5067` (`0x00a79580`): `(v)`: `Ch_DrawSetCull(model, v)` (`0x008300b0`).
    native_5067(h, w, a, s) => chr_set(h, w, s, 0x0083_00b0);
    /// `0x5071` (`0x00a79cc0`): `(v)`: `Ch_TextureLoadImage(model, v)` (`0x00831300`).
    native_5071(h, w, a, s) => chr_set(h, w, s, 0x0083_1300);
    /// `0x506a` (`0x00a79640`): `(v)`: `Ch_PLightSetEnable(model, v)` (`0x00830ae0`).
    native_506a(h, w, a, s) => chr_set(h, w, s, 0x0083_0ae0);
    /// `0x508d` (`0x00a7a7e0`): `(v)`: `Ch_TextureSetAnimTimer(model, v)` (`0x0083d070`).
    native_508d(h, w, a, s) => chr_set(h, w, s, 0x0083_d070);
    /// `0x5069` (`0x00a795e0`): `(v)` -> `Ch_SetKeepFps(model, v)`
    /// (`0x00839b50`), 0 without a model.
    native_5069(h, w, a, s) => {
        let c = chr(h, w);
        let v = pop(h, s);
        if c == 0 {
            return 0;
        }
        call(h, 0x0083_9b50, &[c, v])
    };
    /// `0x505a` (`0x00a78c60`): `(b, a)` -> `Ch_TextureSetClutBase(model,
    /// b, a)` (`0x008313c0`), 0 without a model.
    native_505a(h, w, a, s) => {
        let c = chr(h, w);
        let x = pop(h, s);
        let y = pop(h, s);
        if c == 0 {
            return 0;
        }
        call(h, 0x0083_13c0, &[c, y, x])
    };
    /// `0x506e` (`0x00a79b30`): `(v f)`: `Ch_BindSetScale(model, v)` (`0x00832610`).
    native_506e(h, w, a, s) => chr_setf(h, w, s, 0x0083_2610);
    /// `0x5086` (`0x00a7a680`): `Ch_ReserveCalcNorm(model)` (`0x00835710`).
    native_5086(h, w, a, s) => {
        let c = chr(h, w);
        if c != 0 {
            call(h, 0x0083_5710, &[c]);
        }
        0
    };
    /// `0x507c` (`0x00a7a070`): `Ch_NeckSetTargetCamera(model)` (`0x00835080`).
    native_507c(h, w, a, s) => {
        let c = chr(h, w);
        if c != 0 {
            call(h, 0x0083_5080, &[c]);
        }
        0
    };
    /// `0x5072` (`0x00a79cf0`): `Ch_IdGetPNum(model)` (`0x0082adc0`), 0
    /// without a model.
    native_5072(h, w, a, s) => {
        let c = chr(h, w);
        if c == 0 {
            return 0;
        }
        call(h, 0x0082_adc0, &[c])
    };
    /// `0x5035` (`0x00a79fa0`): `Ch_GetSeqFrame(model)` (`0x00836af0`), 0
    /// without a model.
    native_5035(h, w, a, s) => {
        let c = chr(h, w);
        if c == 0 {
            return 0;
        }
        call(h, 0x0083_6af0, &[c])
    };
    /// `0x5090` (`0x00a7a8d0`): `(v)`: the model's `+0x871` byte, popped
    /// only for a model.
    native_5090(h, w, a, s) => {
        let c = chr(h, w);
        if c != 0 {
            let v = pop(h, s);
            h.write(c + 0x871, v.to_le_bytes()[0]);
        }
        0
    };
    /// `0x503f` (`0x00a7a5a0`): `(v)`: `Ch_SetSequence(model, v)` (`0x00837ac0`).
    native_503f(h, w, a, s) => {
        let v = pop(h, s);
        let c = chr(h, w);
        if c != 0 {
            call(h, 0x0083_7ac0, &[c, v]);
        }
        0
    };
    /// `0x5048` (`0x00a7a940`): as `0x5047`.
    native_5048(h, w, a, s) => other_set(h, s, 0x0083_7ac0);
    /// `0x507b` (`0x00a7a000`): `(x f, y f, z f)`:
    /// `Ch_NeckSetTargetPos(model, &[x, y, z])` (`0x008350a0`).
    native_507b(h, w, a, s) => {
        let c = chr(h, w);
        let z = popf(h, s);
        let y = popf(h, s);
        let x = popf(h, s);
        if c != 0 {
            let v = h.scratch();
            h.write(v, x);
            h.write(v + 4, y);
            h.write(v + 8, z);
            // the original's fourth word (its local beyond z) is stack
            // leftovers; zeroed here
            h.write(v + 12, 0_u32);
            call(h, 0x0083_50a0, &[c, v]);
        }
        0
    };
    /// `0x502c` (`0x00a79bc0`): `(worker, point)`: as `Native_ChrAttach`
    /// with `Ch_BindToElemDirect` (`0x00832680`).
    native_502c(h, w, a, s) => {
        let c = chr(h, w);
        let p = pop(h, s);
        let i = pop(h, s);
        if c == 0 {
            return 0;
        }
        if i == u32::MAX {
            call(h, 0x0083_2680, &[c, 0, 0]);
        } else {
            let t = chr_of(h, i);
            call(h, 0x0083_2680, &[c, t, p]);
        }
        0
    };
    /// `0x5052` (`0x00a787e0`): `(worker, point, x f, y f, z f)`:
    /// `Ch_BindToElemDirectOfs(model, that worker's model, point, x, y,
    /// z)` (`0x008326e0`); worker -1 detaches with the offset.
    native_5052(h, w, a, s) => {
        let c = chr(h, w);
        let z = popf(h, s);
        let y = popf(h, s);
        let x = popf(h, s);
        let p = pop(h, s);
        let i = pop(h, s);
        if c == 0 {
            return 0;
        }
        if i == u32::MAX {
            call(h, 0x0083_26e0, &[c, 0, 0, x, y, z]);
        } else {
            let t = chr_of(h, i);
            call(h, 0x0083_26e0, &[c, t, p, x, y, z]);
        }
        0
    };
    /// `0x5026` (`0x00a796e0`): `(worker, point, &x, &y, &z)`:
    /// `Ch_ElemGetPos(model, point, 0, &v)` (`0x00833d10`) into the three
    /// variables (null ones skipped); 0.
    native_5026(h, w, a, s) => {
        let [i, pt, vx, vy, vz] = pops::<5>(h, s);
        let px = super::atel_get_real_var_adrs(h, w, vx);
        let py = super::atel_get_real_var_adrs(h, w, vy);
        let pz = super::atel_get_real_var_adrs(h, w, vz);
        let c = chr_of(h, i);
        if c != 0 {
            let v = h.scratch();
            for k in 0..4 {
                h.write(v + k * 4, 0_u32);
            }
            call(h, 0x0083_3d10, &[c, pt, 0, v]);
            for (p, k) in [(px, 0), (py, 4), (pz, 8)] {
                if p != 0 {
                    let x = x87::store(x87::load(h.read(v + k)));
                    h.write(p, x);
                }
            }
        }
        0
    };
    /// `0x5036` (`0x00a7a120`): `0x00a7a980(worker, args, stack, 0)`.
    native_5036(h, w, a, s) => {
        fwd(h, 0x00a7_a980, w, a, s, &[0]);
        0
    };
    /// `0x5046` (`0x00a7a880`): `Native_PlayMotionCommon(worker, stack, 1, 1)`.
    native_5046(h, w, a, s) => {
        native_play_motion_common(h, w, s, 1, 1);
        0
    };
    /// `0x5089` (`0x00a7a700`): `Native_PlayMotionCommon(worker, stack, 2, 0)`.
    native_5089(h, w, a, s) => {
        native_play_motion_common(h, w, s, 2, 0);
        0
    };
    /// `0x508b` (`0x00a7a760`): the same as `0x5089`.
    native_508b(h, w, a, s) => {
        native_play_motion_common(h, w, s, 2, 0);
        0
    };
    /// `0x5019` (`0x00a79220`): `Native_PlayMotionCommon(worker, stack, 0, 0)`.
    native_5019(h, w, a, s) => {
        native_play_motion_common(h, w, s, 0, 0);
        0
    };
    /// `0x500c` (`0x00a78d30`): `(v)` popped and dropped (the original only
    /// prints a warning for a model); 0.
    native_500c(h, w, a, s) => {
        pop(h, s);
        0
    };
    /// `0x400e` (`0x00a77f00`): `(v)`: `Sg_EnableCalcMap(v)` (`0x00820440`).
    native_400e(h, w, a, s) => pop1(h, s, 0x0082_0440);
    /// `0x400f` (`0x00a77f20`): `(v)`: `Sg_EnableDrawMap(v)` (`0x00820400`).
    native_400f(h, w, a, s) => pop1(h, s, 0x0082_0400);
    /// `0x4045` (`0x00a77ce0`): `(v)`: `0x0082d9d0(v)`.
    native_4045(h, w, a, s) => pop1(h, s, 0x0082_d9d0);
    /// `0x401b` (`0x00a781d0`): `(v)`: `Sg_AccSetDrawMap(v)` (`0x0082bcb0`).
    native_401b(h, w, a, s) => pop1(h, s, 0x0082_bcb0);
    /// `0x401c` (`0x00a781f0`): `(v)`: `Sg_AccEnableBG(v)` (`0x0082be10`).
    native_401c(h, w, a, s) => pop1(h, s, 0x0082_be10);
    /// `0x4023` (`0x00a78340`): `(v)`: `Sg_PLightAllocate(v)` (`0x00831950`).
    native_4023(h, w, a, s) => pop1(h, s, 0x0083_1950);
    /// `0x403a` (`0x00a77b10`): `(v)`: `Sg_AccSetDrawChr(v)` (`0x0082be00`).
    native_403a(h, w, a, s) => pop1(h, s, 0x0082_be00);
    /// `0x4046` (`0x00a77d00`): `(v)`: `Ch_DrawSetShadowSemi(v)` (`0x00821b20`).
    native_4046(h, w, a, s) => pop1(h, s, 0x0082_1b20);
    /// `0xc003` (`0x00878510`): `(map)`: `AtelMapLoad(map)` (`0x0086dca0`).
    native_c003(h, w, a, s) => pop1(h, s, 0x0086_dca0);
    /// `0xc031` (`0x00878530`): `(v)`: `0x0086dcd0(v)`.
    native_c031(h, w, a, s) => pop1(h, s, 0x0086_dcd0);
    /// `0x4022` (`0x00a78320`): `(v)` -> `Sg_SetKeepFps(v)` (`0x00821ae0`).
    native_4022(h, w, a, s) => {
        let v = pop(h, s);
        call(h, 0x0082_1ae0, &[v])
    };
    /// `0x4043` (`0x00a77c80`): `(b, a)`: `Ch_DrawSetShadowDir(b, a)` (`0x00830120`).
    native_4043(h, w, a, s) => pop2(h, s, 0x0083_0120);
    /// `0x402a` (`0x00a785b0`): `(b, a)`: `Sg_PLightSetEnable(b, a)` (`0x00831c60`).
    native_402a(h, w, a, s) => pop2(h, s, 0x0083_1c60);
    /// `0x4036` (`0x00a77a10`): `(x f, y f, z f)`: `Sg_GlobalSetWind(x, y,
    /// z)` (`0x00824e30`).
    native_4036(h, w, a, s) => {
        let z = popf(h, s);
        let y = popf(h, s);
        let x = popf(h, s);
        call(h, 0x0082_4e30, &[x, y, z]);
        0
    };
    /// `0x403e` (`0x00a77be0`): `(i, v f)`: `Sg_PlayerSetDirFix(i, v)` (`0x0082d990`).
    native_403e(h, w, a, s) => {
        let v = popf(h, s);
        let i = pop(h, s);
        call(h, 0x0082_d990, &[i, v]);
        0
    };
    /// `0x403c` (`0x00a77b60`): `(v f)`: `Sg_SetWaterLimit(v)` (`0x00836660`).
    native_403c(h, w, a, s) => {
        let v = popf(h, s);
        call(h, 0x0083_6660, &[v]);
        0
    };
    /// `0x4025` (`0x00a78370`): `(n, a f, b f, c f, d f, e, f, g, i)`:
    /// `Sg_PLightDefine(n, a, b, c, d, e, f, g, i)` (`0x008319b0`; the
    /// original's `dbgPrintf` is left out).
    native_4025(h, w, a, s) => {
        let i1 = pop(h, s);
        let i2 = pop(h, s);
        let i3 = pop(h, s);
        let i4 = pop(h, s);
        let f5 = popf(h, s);
        let f6 = popf(h, s);
        let f7 = popf(h, s);
        let f8 = popf(h, s);
        let n = pop(h, s);
        call(h, 0x0083_19b0, &[n, f8, f7, f6, f5, i4, i3, i2, i1]);
        0
    };
    /// `0x4044` (`0x00a77cb0`): three popped and dropped; 0.
    native_4044(h, w, a, s) => {
        pops::<3>(h, s);
        0
    };
    /// `0xc053` (`0x008793f0`): four popped and dropped; 0.
    native_c053(h, w, a, s) => {
        pops::<4>(h, s);
        0
    };
    /// `0x100a`, `0xc02c` (`0x008778b0`, `0x00878e60`): one popped; `EAX`
    /// the value.
    native_pop_one(h, w, a, s) => pop(h, s);
    /// `0xc058` (`0x00878280`): `(v)` -> `AtelGetRealVarAdrs(worker, v)`.
    native_c058(h, w, a, s) => {
        let v = pop(h, s);
        super::atel_get_real_var_adrs(h, w, v)
    };
    /// `0xc02f` (`0x00878e80`): `(on, v f)`: the current ctrl work's
    /// `+0x208` = v and `+2` bit 0 = on != 0; `EAX` on != 0.
    native_c02f(h, w, a, s) => {
        let cw = call(h, 0x0086_aff0, &[]);
        let v = popf(h, s);
        let on = u8::from(pop(h, s) != 0);
        h.write(cw + 0x208, v);
        let b = h.read::<u8>(cw + 2);
        h.write(cw + 2, b & !1 | on);
        u32::from(on)
    };
    /// `0xc030` (`0x00878ed0`): `TkIsDebug()`, then the current ctrl
    /// work's `+0x1f4`.
    native_c030(h, w, a, s) => {
        let cw = call(h, 0x0086_aff0, &[]);
        call(h, 0x0088_7d10, &[]);
        h.read(cw + 0x1f4)
    };
    /// `0xc009` (`0x00878630`): `(mode)`: `AtelSetEventMode(mode)`
    /// (`0x00873e40`); mode.
    native_c009(h, w, a, s) => {
        let v = pop(h, s);
        call(h, 0x0087_3e40, &[v]);
        v
    };
    /// `0xc007` (`0x00878600`): `(map, index)`: `TkSetStartMapIndex(map,
    /// index)` (`0x0088ebf0`); map.
    native_c007(h, w, a, s) => {
        let i = pop(h, s);
        let m = pop(h, s);
        call(h, 0x0088_ebf0, &[m, i]);
        m
    };
    /// `0xc051` (`0x008793b0`): `(on)`: `TkSetTGS(on != 0)` (`0x0088ec10`); on != 0.
    native_c051(h, w, a, s) => {
        let on = u32::from(pop(h, s) != 0);
        call(h, 0x0088_ec10, &[on]);
        on
    };
    /// `0xc036` (`0x00878f70`): `(on)`: `pppFpSetDisp(on != 0)` (`0x00729050`); on != 0.
    native_c036(h, w, a, s) => {
        let on = u32::from(pop(h, s) != 0);
        call(h, 0x0072_9050, &[on]);
        on
    };
    /// `0xc00d` (`0x008786d0`): `(on)`: `MsSetSaveConfigCreditName(on != 0)`
    /// (`0x00785e90`); on != 0.
    native_c00d(h, w, a, s) => {
        let on = u32::from(pop(h, s) != 0);
        call(h, 0x0078_5e90, &[on]);
        on
    };
    /// `0xc002` (`0x008784e0`): `(on)`: `MsSetSaveConfigHiragana(on != 0)`
    /// (`0x00785f40`); 0.
    native_c002(h, w, a, s) => {
        let on = u32::from(pop(h, s) != 0);
        call(h, 0x0078_5f40, &[on]);
        0
    };
    /// `0xc013` (`0x008787b0`): 1 when `MsGetSaveConfigHiragana()`
    /// (`0x007851f0`) is 1.
    native_c013(h, w, a, s) => u32::from(call(h, 0x0078_51f0, &[]) == 1);
    /// `0xc014` (`0x008787c0`): 1 when `0x007851b0()` is not 0.
    native_c014(h, w, a, s) => u32::from(call(h, 0x0078_51b0, &[]) != 0);
    /// `0xc052` (`0x008793e0`): jumps to `0x0088d6d0`.
    native_c052(h, w, a, s) => call(h, 0x0088_d6d0, &[w, a, s]);
    /// `0xc054` (`0x008781b0`): `(n)` -> `TkMalloc(n)` (`0x0088e650`).
    native_c054(h, w, a, s) => {
        let v = pop(h, s);
        call(h, 0x0088_e650, &[v])
    };
    /// `0xc055` (`0x008781d0`): `(p)`: `TkFree(p)` (`0x0088d520`); 1.
    native_c055(h, w, a, s) => {
        let v = pop(h, s);
        call(h, 0x0088_d520, &[v]);
        1
    };
    /// `0x101c` (`0x008780f0`): `0x00878140(worker, args, stack)`'s record
    /// `+4` = 0; 0.
    native_101c(h, w, a, s) => {
        let r = fwd(h, 0x0087_8140, w, a, s, &[]);
        h.write(r.wrapping_add(4), 0_u32);
        0
    };
    /// `0xc024` (`0x00878c10`, start): `(_, v)`: the battle end
    /// (`AtelEventSetUpForBattleMusic`, `AtelEventRemoveForBattle` for the
    /// event jump map, `AtelBattleEndCallback(1, v, map)`); its `EAX`.
    native_c024(h, w, a, s) => {
        pop(h, s);
        let v = pop(h, s);
        let m = call(h, 0x0086_c1f0, &[]);
        call(h, 0x0086_6240, &[m]);
        let m = call(h, 0x0086_c1f0, &[]);
        call(h, 0x0086_5dc0, &[m]);
        let m = call(h, 0x0086_c1f0, &[0]);
        call(h, 0x0086_1160, &[1, v, m])
    };
    /// `0xc024` (`0x00878c60`, poll), `0xb00a` (`0x0076ec10`, int) and the
    /// like: 1.
    native_one(h, w, a, s) => 1;
    /// `0xb00a` (`0x0076ec10`, int): 0.
    native_zero(h, w, a, s) => 0;
    /// `0xb00d` (`0x0076ecb0`): the movie done flag = 0; 0.
    native_b00d(h, w, a, s) => {
        set(h, MOVIE_DONE, 0);
        0
    };
    /// `0xb00c` (`0x0076ec90`): jumps to `0x00641140` (after a
    /// `dbgPrintf`, left out).
    native_b00c(h, w, a, s) => call(h, 0x0064_1140, &[w, a, s]);
    /// `0xb00a` (`0x0076eb80`, start): `(movie, flags)`: the movie =
    /// movie, `graphicInitFMVPlayer(movie, flags & 0x80)`; when it starts,
    /// `0x006427c0(0)`, `g_WaitFramesByTime`, the done flag and
    /// `0x0112a00c` = 1. `EAX` the last call's.
    native_b00a(h, w, a, s) => {
        let flags = pop(h, s);
        let m = pop(h, s);
        set(h, MOVIE, m);
        let r = call(h, 0x0064_1690, &[m, flags & 0x80]);
        if r == 0 {
            return 0;
        }
        let r = call(h, 0x0064_27c0, &[0]);
        set(h, WAIT_FRAMES_BY_TIME, 1);
        set(h, MOVIE_DONE, 1);
        set(h, 0x0112_a00c, 1);
        r
    };
    /// `0xb00a` (`0x0076ebe0`, poll): 5 unless `0x0112a00c` is set while
    /// `0x00641ce0()` or not `0x00645dc0()`.
    native_b00a_poll(h, w, a, s) => {
        let p = h.rebase(0x0112_a00c);
        if h.read::<u32>(p) == 0 {
            return 5;
        }
        if call(h, 0x0064_1ce0, &[]) != 0 || call(h, 0x0064_5dc0, &[]) == 0 {
            return 0;
        }
        5
    };
    /// `0xb00b` (`0x0076ec30`, poll): 5 when `0x00645e50()` is not 0.
    native_b00b_poll(h, w, a, s) => {
        if call(h, 0x0064_5e50, &[]) == 0 { 0 } else { 5 }
    };
    /// `0xb00b` (`0x0076ec40`, int): `graphicDestroyFmv()` for movie
    /// `0x34` (the `dbgPrintf` left out), then `0xb001`'s int.
    native_b00b_result(h, w, a, s) => {
        let m = h.rebase(MOVIE);
        if h.read::<u32>(m) == 0x34 {
            call(h, 0x0063_df30, &[]);
        }
        native_b001_result(h, w, a, s)
    };
    // ---- group 6 ----
    /// `0x601e` (`0x007b83d0`): `(var, deg f)`: `0x007bf520(cam, 1, 0,
    /// &var, deg * pi / 180)`; the camera.
    native_601e(h, w, a, s) => {
        let c = cam(h, w);
        let f = popf(h, s);
        let r = x87::store(x87::div(x87::mul(x87::load(f), PI_F32), 180.0));
        let v = pop(h, s);
        let p = super::atel_get_real_var_adrs(h, w, v);
        call(h, 0x007b_f520, &[c, 1, 0, p, r]);
        c
    };
    /// `0x6042` (`0x007b85f0`): `(v)`: `0x007bdf90(cam, 0, v ^ 0xffff)`;
    /// the camera.
    native_6042(h, w, a, s) => {
        let c = cam(h, w);
        let v = pop(h, s);
        call(h, 0x007b_df90, &[c, 0, v ^ 0xffff]);
        c
    };
    /// `Native_CamSetDepthMaskMode` (`0x007b88b0`, `0x607e`): `(flags)`:
    /// `0x007bcf50(cam, flags)`; the camera.
    native_cam_set_depth_mask_mode(h, w, a, s) => {
        let c = cam(h, w);
        let v = pop(h, s);
        call(h, 0x007b_cf50, &[c, v]);
        c
    };
    /// `Native_CamSetDepthMask` (`0x007b8920`, `0x607f`): `(plane, z)`:
    /// `0x007bf280(cam, plane, z)`; the camera.
    native_cam_set_depth_mask(h, w, a, s) => {
        let c = cam(h, w);
        let z = pop(h, s);
        let p = pop(h, s);
        call(h, 0x007b_f280, &[c, p, z]);
        c
    };
    /// `0x6013` (`0x007b9870`): `(frames)`: `MsCameraMoveFrame(cam, 1, 1,
    /// frames, 3)` (`0x007bde50`); 0.
    native_6013(h, w, a, s) => {
        let c = cam(h, w);
        let v = pop(h, s);
        call(h, 0x007b_de50, &[c, 1, 1, v, 3]);
        0
    };
    /// `0x601b` (`0x007b9ca0`): `Cam_IsMoving(cam, 1)` (`0x007bdd20`).
    native_601b(h, w, a, s) => {
        let c = cam(h, w);
        call(h, 0x007b_dd20, &[c, 1])
    };
    /// `0x6061` (`0x007b9960`): `(v)`: `MsScreenResetShake(v)` (`0x007c56c0`).
    native_6061(h, w, a, s) => pop1(h, s, 0x007c_56c0);
    /// `0x604e` (`0x007b8ad0`): six: `0x007bc930(f, 1)`, `0x007bc890(f, e,
    /// d, c, b, a, 0)`; 0.
    native_604e(h, w, a, s) => {
        let [p6, p5, p4, p3, p2, p1] = pops::<6>(h, s);
        call(h, 0x007b_c930, &[p6, 1]);
        call(h, 0x007b_c890, &[p6, p5, p4, p3, p2, p1, 0]);
        0
    };
    /// `0x6050` (`0x007b8ba0`): five: `0x007bc750(e, d, c, b, a)`; 0.
    native_6050(h, w, a, s) => {
        let v = pops::<5>(h, s);
        call(h, 0x007b_c750, &v);
        0
    };
    /// `0x6051` (`0x007b8c30`): `(b, a)`: `MsScreenLinkSet(b, 1, a)` (`0x007bbdd0`).
    native_6051(h, w, a, s) => {
        let [b, x] = pops::<2>(h, s);
        call(h, 0x007b_bdd0, &[b, 1, x]);
        0
    };
    /// `0x6052` (`0x007b8c00`): `(b, a)`: `MsScreenLinkSet(b, 0, a)`.
    native_6052(h, w, a, s) => {
        let [b, x] = pops::<2>(h, s);
        call(h, 0x007b_bdd0, &[b, 0, x]);
        0
    };
    /// `0x6055` (`0x007b8cc0`): `(d, c, b, a)`: `MsScreenMoveAcc(d, 0, c,
    /// b, a)` (`0x007bbeb0`).
    native_6055(h, w, a, s) => {
        let [d, c, b, x] = pops::<4>(h, s);
        call(h, 0x007b_beb0, &[d, 0, c, b, x]);
        0
    };
    /// `0x6058` (`0x007b8d70`): as `0x6055` with mode 1.
    native_6058(h, w, a, s) => {
        let [d, c, b, x] = pops::<4>(h, s);
        call(h, 0x007b_beb0, &[d, 1, c, b, x]);
        0
    };
    /// `0x604f` (`0x007b8b80`): `(v)`: `0x007bc950(v, 0, v)`.
    native_604f(h, w, a, s) => {
        let v = pop(h, s);
        call(h, 0x007b_c950, &[v, 0, v]);
        0
    };
    /// `0x607b` (`0x007b8e40`, start): `(screen)` into the args word; `EAX` it.
    native_607b(h, w, a, s) => {
        let v = pop(h, s);
        h.write(a, v);
        v
    };
    /// `0x607b` (`0x007b8e60`, poll): 5 once `MsScreenMoveCheck(screen, 0)`
    /// (`0x007bbfc0`) is 0.
    native_607b_poll(h, w, a, s) => {
        let v = h.read::<u32>(a);
        if call(h, 0x007b_bfc0, &[v, 0]) == 0 { 5 } else { 0 }
    };
}

macro_rules! shakes {
    ($($(#[$m:meta])* $name:ident => [$($k:expr),*];)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
            fwd(h, 0x007b_b8c0, w, a, s, &[$($k),*])
        }
    )*};
}

shakes! {
    /// `0x6069` (`0x007b8ff0`): `MsAtelCameraShake(worker, args, stack, 0, 3)` (`0x007bb8c0`).
    native_6069 => [0, 3];
    /// `0x6067` (`0x007b8fc0`): the shake with `(1, 2)`.
    native_6067 => [1, 2];
    /// `0x6081` (`0x007b92c0`): the shake with `(0, 7)`.
    native_6081 => [0, 7];
    /// `0x6082` (`0x007b9300`): the shake with `(1, 7)`.
    native_6082 => [1, 7];
    /// `0x6066` (`0x007b8fa0`): the shake with `(0, 2)`.
    native_6066 => [0, 2];
    /// `0x606c` (`0x007b9150`): the shake with `(0, 4)`.
    native_606c => [0, 4];
}

macro_rules! cam_fwd {
    ($($(#[$m:meta])* $name:ident => $f:expr, [$($k:expr),*];)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
            fwd(h, $f, w, a, s, &[$($k),*]);
            0
        }
    )*};
}

cam_fwd! {
    /// `0x6033` (`0x007b8150`): `0x007ba5b0(worker, args, stack, 0, 1)`; 0.
    native_6033 => 0x007b_a5b0, [0, 1];
    /// `0x6074` (`0x007b9610`): `0x007bb840(worker, args, stack, 6)`; 0.
    native_6074 => 0x007b_b840, [6];
    /// `0x606e` (`0x007b94e0`): `0x007bb840(..., 4)`; 0.
    native_606e => 0x007b_b840, [4];
    /// `0x605e` (`0x007b9420`): `0x007bb840(..., 1)`; 0.
    native_605e => 0x007b_b840, [1];
    /// `0x600a` (`0x007b9440`): `0x007bb2b0(..., 1, 0)`; 0.
    native_600a => 0x007b_b2b0, [1, 0];
    /// `0x602a` (`0x007ba1f0`): `0x007bb370(..., 0, 0)`; 0.
    native_602a => 0x007b_b370, [0, 0];
    /// `0x604c` (`0x007b8790`): `0x007badb0(..., 1, 3)`; 0.
    native_604c => 0x007b_adb0, [1, 3];
}

/// `0x6078` (`0x007b91f0`, float): jumps to `0x007bd510(worker, args,
/// stack)` (`ST0`).
pub fn native_6078<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> f64 {
    callf(h, 0x007b_d510, &[w, a, s])
}

/// `AtelCall_Math_Abs_RF` (`0x00877fd0`, `0x1019`, float, Fahrenheit's
/// name): `|v|` rounded to `f32`.
pub fn atel_call_math_abs_rf<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    r32(x87::load(atel_pop_stack_float(h, s)).abs())
}

/// `0x5028` (`0x00a799d0`, float): `(v f)` -> `Ch_MoveSetChgRun(model,
/// v)` (`0x0082b040`), 0.0 without a model.
pub fn native_5028<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> f64 {
    chr_getf(h, w, s, 0x0082_b040)
}

/// `0x505e` (`0x00a78e20`, float): `Ch_GetSp(model)` (`0x0082ad60`), 0.0
/// without a model.
pub fn native_505e<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> f64 {
    let c = chr(h, w);
    if c == 0 {
        return 0.0;
    }
    callf(h, 0x0082_ad60, &[c])
}

/// `0x4034` (`0x00a779e0`, float): `(v)`: `Sg_SetMapAlphaFast(v)`
/// (`0x00821b00`); 0.0.
pub fn native_4034<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = pop(h, s);
    call(h, 0x0082_1b00, &[v]);
    0.0
}

/// `0x4039` (`0x00a77ac0`, float): `(d, c, b, a)`: `Sg_AccSetClear(d, c,
/// b, a)` (`0x0082bdc0`); 0.0.
pub fn native_4039<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = pops::<4>(h, s);
    call(h, 0x0082_bdc0, &v);
    0.0
}

/// `0x504d` (`0x00a786e0`, float): `(v f)`: `Ch_AllSetThicknessEvent(v)`
/// (`0x0082a8d0`); 0.0.
pub fn native_504d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let v = popf(h, s);
    call(h, 0x0082_a8d0, &[v]);
    0.0
}

// ---- round 13: the four left over ----

/// `AtelCall_Math_ATan2_RF` (`0x00877b30`, `0x1005`, float, Fahrenheit's
/// name): `(y f, x f)` -> `atan2(y, x)` through the game's own
/// `_CIatan2` (the import thunk `0x009498bc`), rounded to `f32`.
pub fn atel_call_math_atan2_rf<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> f64 {
    let x = r32(x87::load(atel_pop_stack_float(h, s)));
    let y = x87::load(atel_pop_stack_float(h, s));
    let f = h.rebase(0x0094_98bc);
    r32(h.call_intrinsic2(f, y, x))
}

/// `0x6077` (`0x007b9190`, float): the camera's `+0xc90` angle
/// (`0x007bd500`, `ST0`, rounded to `f32`) in degrees: `MsNearRag(0, v) *
/// 180 / pi`, rounded to `f32`.
pub fn native_6077<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> f64 {
    let v = x87::store(callf(h, 0x007b_d500, &[]));
    let r = callf(h, 0x007c_04c0, &[x87::store(0.0), v]);
    r32(x87::div(x87::mul(r, 180.0), PI_F32))
}

/// `0x5037` (`0x00a7a140`): `(worker, &x, &y, &z)`: the model's point 0
/// (`Ch_OptposGet`, `0x008354f0`) into the three variables (null ones
/// skipped); 0.
pub fn native_5037<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let [i, vx, vy, vz] = pops::<4>(h, s);
    let px = super::atel_get_real_var_adrs(h, w, vx);
    let py = super::atel_get_real_var_adrs(h, w, vy);
    let pz = super::atel_get_real_var_adrs(h, w, vz);
    let c = chr_of(h, i);
    if c != 0 {
        let v = h.scratch();
        for k in 0..4 {
            h.write(v + k * 4, 0_u32);
        }
        call(h, 0x0083_54f0, &[c, 0, v]);
        for (p, k) in [(px, 0), (py, 4), (pz, 8)] {
            if p != 0 {
                let x = x87::store(x87::load(h.read(v + k)));
                h.write(p, x);
            }
        }
    }
    0
}

/// `0xc025` (`0x00878c70`): `(msg)`: copies the message text
/// (`Msg_Text`, `0x0086bf30`) over the player's name
/// (`MsGetSaveChrName(0)`, `0x00784fb0`) when it is shorter than
/// `MsGetSaveChrNameSize(0)` (`0x00784fe0`); 1 when it did, else 0.
pub fn native_c025<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let p = call(h, 0x0086_bf30, &[v]);
    let n = call(h, 0x0078_4fe0, &[0]);
    let mut len = 0_u32;
    while h.read::<u8>(p.wrapping_add(len)) != 0 && len < 0x1_0000 {
        len += 1;
    }
    if len >= n {
        return 0;
    }
    let q = call(h, 0x0078_4fb0, &[0]);
    if q == 0 {
        return 0;
    }
    for k in 0..=len {
        let b = h.read::<u8>(p.wrapping_add(k));
        h.write(q.wrapping_add(k), b);
    }
    1
}

// ---- round 16 ----

/// `Native_PlayMotionCommon(worker, stack, mode, other)` (`0x00a7aae0`):
/// pops `(motion, _, loops, start, end)` (and a worker when `other`, whose
/// model then plays); per-map fixes set the model's `+0x870` (map `0xcd`
/// at scenario `0x825`, `0x16c` at `0x498`, `0x11` at 200); model `0x66`
/// sets `0x013008e0` = (motion == `0x21092`); then mode 0
/// `Chr_PlayMotionMarks` (`0x00837350`), 1 `Ch_MotionSetLow2`
/// (`0x008373e0`), 2 `Chr_PlayMotionInclEnd` (`0x008374c0`) with `(model,
/// motion, _, loops, start, end)`. Returns 0.
pub fn native_play_motion_common<H: NativeHost>(
    h: &mut H,
    w: u32,
    s: u32,
    mode: u32,
    other: u32,
) -> u32 {
    let end = pop(h, s);
    let start = pop(h, s);
    let loops = pop(h, s);
    let x = pop(h, s);
    let motion = pop(h, s);
    let c = if other == 0 {
        chr(h, w)
    } else {
        let i = pop(h, s);
        chr_of(h, i)
    };
    if c == 0 {
        return 0;
    }
    let map = h.read::<u32>(h.rebase(0x0133_0844));
    let id = h.read::<u16>(c);
    let flag = |h: &mut H, on: u8| h.write(c + 0x870, on);
    let scenario = |h: &mut H| call(h, 0x0078_7360, &[]) & 0xffff;
    let (m, st, en) = (motion, start, end);
    if map == 0xcd && scenario(h) == 0x825 {
        if id == 0x79 && m == 0x0079_1002 {
            if st == 0x76c && en == 0x8f2 {
                flag(h, 0);
            }
            if st == 0x8f2 && en == u32::MAX {
                flag(h, 1);
            }
        }
        if id == 0x400b && m == 0x400b_1007 {
            if st == 0x76c && en == 0x8cd {
                flag(h, 0);
            }
            if st == 0x8cf && en == 0x8d7 {
                flag(h, 1);
            }
        }
    } else if map == 0x16c && scenario(h) == 0x498 {
        if id == 7 {
            if m == 0x0007_1068 && st == 0x2c && en == 0x42 {
                flag(h, 0);
            }
            if m == 0x0007_1069 && st == 0x44 && en == 0x62 {
                flag(h, 1);
            }
        }
        if id == 0x506a && m == 0x506a_1000 && st == 0x2c && en == u32::MAX {
            flag(h, 0);
        }
    } else if map == 0x11 && scenario(h) == 200 {
        if id == 5 {
            if m == 0x0005_1076 && st == 0 && en == u32::MAX {
                flag(h, 0);
            }
            if m == 0x0005_1077 && st == 0 && en == u32::MAX {
                flag(h, 1);
            }
        }
        if id == 0x4001 {
            if m == 0x4001_1001 && st == 0 && en == u32::MAX {
                flag(h, 0);
            }
            if m == 0x4001_1002 && st == 0 && en == u32::MAX {
                flag(h, 1);
            }
        }
    }
    if id == 0x66 {
        h.write(h.rebase(0x0130_08e0), u32::from(m == 0x0002_1092));
    }
    let f = match mode {
        0 => 0x0083_7350,
        1 => 0x0083_73e0,
        2 => 0x0083_74c0,
        _ => return 0,
    };
    call(h, f, &[c, m, x, loops, start, end]);
    0
}

/// The Rust version of a group 1, 4, 5, 6, 11 or 12 handler, if there is
/// one.
#[must_use]
#[allow(clippy::too_many_lines)] // one arm per handler
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    use Native::{Float, Int};
    Some(match sym {
        0x0087_7b30 => Float(atel_call_math_atan2_rf),
        0x007b_9190 => Float(native_6077),
        0x00a7_a140 => Int(native_5037),
        0x0087_8c70 => Int(native_c025),
        0x00a7_9820 => Int(native_chr_load_system_mgrp),
        0x0076_e980 | 0x00a7_9970 | 0x00a7_9a10 => Int(native_done),
        0x00a7_9980 => Int(native_chr_load_system_mgrp_result),
        0x00a7_9c50 => Int(native_chr_attach),
        0x00a7_9340 => Int(native_504f),
        0x00a7_9380 => Int(native_504f_poll),
        0x00a7_9210 | 0x00a7_9410 | 0x00a7_9540 => Int(native_args_c),
        0x00a7_8f80 => Int(native_5061),
        0x00a7_81a0 => Int(native_401a),
        0x00a7_8130 => Int(native_4017),
        0x00a7_9e30 => Int(native_5076),
        0x0076_eb30 | 0x0076_ec20 | 0x00a7_7ee0 => Int(native_nothing),
        0x00a7_7ef0 => Int(native_fade_wait_poll),
        0x0087_79c0 => Int(native_1000),
        0x00a7_99b0 => Int(native_chr_load_motion_group),
        0x00a7_9a20 => Int(native_chr_load_motion_group_result),
        0x00a7_a340 => Float(native_chr_set_hit_radius_scale),
        0x00a7_8710 => Int(native_5050),
        0x00a7_8fb0 => Int(native_5015),
        0x00a7_a520 => Int(native_503d),
        0x0087_7ff0 => Float(native_101a),
        0x00a7_8a40 => Int(native_5006),
        0x00a7_8180 => Int(native_4019),
        0x00a7_8ec0 => Int(native_5013),
        0x00a7_7c30 => Int(native_set_control_angle),
        0x00a7_8a80 => Int(native_5056),
        0x00a7_80c0 => Int(native_4015),
        0x00a7_a570 => Int(native_503e),
        0x0087_8060 => Float(native_101b),
        0x00a7_8960 => Int(native_5054),
        0x00a7_9160 => Int(native_5063),
        0x00a7_91a0 => Int(native_5063_poll),
        0x00a7_80a0 => Int(native_4014),
        0x00a7_9f70 => Int(native_5079),
        0x00a7_a900 => Int(native_5047),
        0x00a7_9fc0 => Int(native_507a),
        0x00a7_9ef0 => Int(native_5078),
        0x00a7_80f0 => Int(native_4016),
        0x00a7_8b90 => Int(native_500b),
        0x00a7_9b90 => Int(native_506f),
        0x00a7_8b70 => Int(native_5058),
        0x00a7_a3d0 => Float(native_503a),
        0x0087_7d30 => Float(native_1013),
        0x00a7_7d30 => Int(atel_sg_flash),
        0x0076_ea40 => Int(native_b003),
        0x00a7_a810 => Int(native_5044),
        0x00a7_86a0 => Int(native_504c),
        0x00a7_91e0 => Int(native_5018),
        0x0087_7aa0 => Float(atel_call_math_cos_rf),
        0x00a7_7de0 => Int(native_4007),
        0x0087_7a70 => Float(atel_call_math_sin_rf),
        0x0076_ea50 => Int(native_b004),
        0x0076_ea60 => Int(native_b004_poll),
        0x0076_eac0 => Int(native_b004_result),
        0x0076_e910 => Int(native_b000),
        0x0076_e990 => Int(native_b000_result),
        0x0076_e9b0 => Int(native_b001),
        0x0076_e9c0 => Int(native_b001_poll),
        0x0076_e9f0 => Int(native_b001_result),
        0x00a7_7dc0 => Int(native_4006),
        0x00a7_7b90 => Int(native_4001),
        0x00a7_8e90 => Int(native_505f),
        0x00a7_8df0 => Int(native_500d),
        0x00a7_a4d0 => Int(native_503c),
        0x00a7_a8a0 => Int(native_508f),
        0x00a7_7b30 => Int(native_foot_set_material),
        0x0076_eb40 => Int(native_b009_poll),
        0x0076_eb60 => Int(native_b009_result),
        0x00a7_7ed0 => Int(native_400c),
        0x00a7_82f0 => Int(native_4020),
        0x00a7_9420 => Int(native_5065),
        0x00a7_9a80 => Int(native_506d),
        0x00a7_a090 => Int(native_507d),
        0x00a7_9480 => Int(native_5066),
        0x00a7_94d0 => Int(native_5066_poll),
        0x00a7_a5d0 => Int(native_5084),
        0x00a7_a640 => Int(native_5085),
        0x00a7_a410 => Int(native_5080),
        0x00a7_8f30 => Int(native_5014),
        0x00a7_a6b0 => Int(native_5088),
        0x00a7_a450 => Int(native_503b),
        0x0087_7dc0 => Int(native_1015),
        0x00a7_7e30 => Int(native_4008),
        0x00a7_7e80 => Int(native_4009),
        0x00a7_89a0 => Int(native_5007),
        0x00a7_85f0 => Int(native_5049),
        0x00a7_8620 => Int(native_5049_poll),
        0x00a7_9450 => Int(native_501f),
        0x00a7_8640 => Int(native_504a),
        0x00a7_8670 => Int(native_504b),
        0x00a7_82e0 => Int(native_401f),
        0x00a7_7e20 => Int(native_400b),
        0x00a7_8360 => Int(native_4024),
        0x00a7_7c20 => Int(native_403f),
        0x00a7_a4a0 => Int(native_5081),
        0x00a7_a4f0 => Int(native_5082),
        0x00a7_9d30 => Int(native_5073),
        0x00a7_a540 => Int(native_5083),
        0x00a7_8f00 => Int(native_5060),
        0x00a7_a850 => Int(native_508e),
        0x00a7_9580 => Int(native_5067),
        0x00a7_9cc0 => Int(native_5071),
        0x00a7_9640 => Int(native_506a),
        0x00a7_a7e0 => Int(native_508d),
        0x00a7_95e0 => Int(native_5069),
        0x00a7_8c60 => Int(native_505a),
        0x00a7_9b30 => Int(native_506e),
        0x00a7_a680 => Int(native_5086),
        0x00a7_a070 => Int(native_507c),
        0x00a7_9cf0 => Int(native_5072),
        0x00a7_9fa0 => Int(native_5035),
        0x00a7_a8d0 => Int(native_5090),
        0x00a7_a5a0 => Int(native_503f),
        0x00a7_a940 => Int(native_5048),
        0x00a7_a000 => Int(native_507b),
        0x00a7_9bc0 => Int(native_502c),
        0x00a7_87e0 => Int(native_5052),
        0x00a7_96e0 => Int(native_5026),
        0x00a7_a120 => Int(native_5036),
        0x00a7_a880 => Int(native_5046),
        0x00a7_a700 => Int(native_5089),
        0x00a7_a760 => Int(native_508b),
        0x00a7_9220 => Int(native_5019),
        0x00a7_8d30 => Int(native_500c),
        0x00a7_7f00 => Int(native_400e),
        0x00a7_7f20 => Int(native_400f),
        0x00a7_7ce0 => Int(native_4045),
        0x00a7_81d0 => Int(native_401b),
        0x00a7_81f0 => Int(native_401c),
        0x00a7_8340 => Int(native_4023),
        0x00a7_7b10 => Int(native_403a),
        0x00a7_7d00 => Int(native_4046),
        0x0087_8510 => Int(native_c003),
        0x0087_8530 => Int(native_c031),
        0x00a7_8320 => Int(native_4022),
        0x00a7_7c80 => Int(native_4043),
        0x00a7_85b0 => Int(native_402a),
        0x00a7_7a10 => Int(native_4036),
        0x00a7_7be0 => Int(native_403e),
        0x00a7_7b60 => Int(native_403c),
        0x00a7_8370 => Int(native_4025),
        0x00a7_7cb0 => Int(native_4044),
        0x0087_93f0 => Int(native_c053),
        0x0087_8280 => Int(native_c058),
        0x0087_8e80 => Int(native_c02f),
        0x0087_8ed0 => Int(native_c030),
        0x0087_8630 => Int(native_c009),
        0x0087_8600 => Int(native_c007),
        0x0087_93b0 => Int(native_c051),
        0x0087_8f70 => Int(native_c036),
        0x0087_86d0 => Int(native_c00d),
        0x0087_84e0 => Int(native_c002),
        0x0087_87b0 => Int(native_c013),
        0x0087_87c0 => Int(native_c014),
        0x0087_93e0 => Int(native_c052),
        0x0087_81b0 => Int(native_c054),
        0x0087_81d0 => Int(native_c055),
        0x0087_80f0 => Int(native_101c),
        0x0087_8c10 => Int(native_c024),
        0x0087_8c60 => Int(native_one),
        0x0076_eb80 => Int(native_b00a),
        0x0076_ebe0 => Int(native_b00a_poll),
        0x0076_ec10 => Int(native_zero),
        0x0076_ecb0 => Int(native_b00d),
        0x0076_ec90 => Int(native_b00c),
        0x0076_ec30 => Int(native_b00b_poll),
        0x0076_ec40 => Int(native_b00b_result),
        0x007b_83d0 => Int(native_601e),
        0x007b_85f0 => Int(native_6042),
        0x007b_88b0 => Int(native_cam_set_depth_mask_mode),
        0x007b_8920 => Int(native_cam_set_depth_mask),
        0x007b_9870 => Int(native_6013),
        0x007b_9ca0 => Int(native_601b),
        0x007b_9960 => Int(native_6061),
        0x007b_8ad0 => Int(native_604e),
        0x007b_8ba0 => Int(native_6050),
        0x007b_8c30 => Int(native_6051),
        0x007b_8c00 => Int(native_6052),
        0x007b_8cc0 => Int(native_6055),
        0x007b_8d70 => Int(native_6058),
        0x007b_8b80 => Int(native_604f),
        0x007b_8e40 => Int(native_607b),
        0x007b_8e60 => Int(native_607b_poll),
        0x007b_8ff0 => Int(native_6069),
        0x007b_8fc0 => Int(native_6067),
        0x007b_92c0 => Int(native_6081),
        0x007b_9300 => Int(native_6082),
        0x007b_8fa0 => Int(native_6066),
        0x007b_9150 => Int(native_606c),
        0x007b_8150 => Int(native_6033),
        0x007b_9610 => Int(native_6074),
        0x007b_94e0 => Int(native_606e),
        0x007b_9420 => Int(native_605e),
        0x007b_9440 => Int(native_600a),
        0x007b_a1f0 => Int(native_602a),
        0x007b_8790 => Int(native_604c),
        0x007b_91f0 => Float(native_6078),
        0x0087_7fd0 => Float(atel_call_math_abs_rf),
        0x00a7_99d0 => Float(native_5028),
        0x00a7_8e20 => Float(native_505e),
        0x00a7_79e0 => Float(native_4034),
        0x00a7_7ac0 => Float(native_4039),
        0x00a7_86e0 => Float(native_504d),
        0x0087_78b0 | 0x0087_8e60 => Int(native_pop_one),
        _ => return None,
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 215] = [
    (0x0087_7b30, true),
    (0x007b_9190, true),
    (0x00a7_a140, false),
    (0x0087_8c70, false),
    (0x00a7_9820, false),
    (0x00a7_9970, false),
    (0x00a7_9980, false),
    (0x00a7_9c50, false),
    (0x00a7_9340, false),
    (0x00a7_9380, false),
    (0x00a7_9410, false),
    (0x00a7_8f80, false),
    (0x00a7_81a0, false),
    (0x00a7_8130, false),
    (0x00a7_9e30, false),
    (0x00a7_7ee0, false),
    (0x00a7_7ef0, false),
    (0x0087_79c0, false),
    (0x00a7_99b0, false),
    (0x00a7_9a10, false),
    (0x00a7_9a20, false),
    (0x00a7_a340, true),
    (0x00a7_8710, false),
    (0x00a7_8fb0, false),
    (0x00a7_a520, false),
    (0x0087_7ff0, true),
    (0x00a7_8a40, false),
    (0x00a7_8180, false),
    (0x00a7_8ec0, false),
    (0x00a7_7c30, false),
    (0x00a7_8a80, false),
    (0x00a7_80c0, false),
    (0x00a7_a570, false),
    (0x0087_8060, true),
    (0x00a7_8960, false),
    (0x00a7_9160, false),
    (0x00a7_91a0, false),
    (0x00a7_9210, false),
    (0x00a7_80a0, false),
    (0x00a7_9f70, false),
    (0x00a7_a900, false),
    (0x00a7_9fc0, false),
    (0x00a7_9ef0, false),
    (0x00a7_80f0, false),
    (0x00a7_8b90, false),
    (0x00a7_9b90, false),
    (0x00a7_8b70, false),
    (0x00a7_a3d0, true),
    (0x0087_7d30, true),
    (0x00a7_7d30, false),
    (0x0076_ea40, false),
    (0x00a7_a810, false),
    (0x00a7_86a0, false),
    (0x00a7_91e0, false),
    (0x0087_7aa0, true),
    (0x00a7_7de0, false),
    (0x0087_7a70, true),
    (0x0076_ea50, false),
    (0x0076_ea60, false),
    (0x0076_eac0, false),
    (0x0076_e910, false),
    (0x0076_e980, false),
    (0x0076_e990, false),
    (0x0076_e9b0, false),
    (0x0076_e9c0, false),
    (0x0076_e9f0, false),
    (0x00a7_7dc0, false),
    (0x00a7_7b90, false),
    (0x00a7_8e90, false),
    (0x00a7_8df0, false),
    (0x00a7_a4d0, false),
    (0x00a7_a8a0, false),
    (0x00a7_7b30, false),
    (0x0076_eb30, false),
    (0x0076_eb40, false),
    (0x0076_eb60, false),
    (0x00a7_7ed0, false),
    (0x00a7_82f0, false),
    (0x00a7_9420, false),
    (0x00a7_9a80, false),
    (0x00a7_a090, false),
    (0x00a7_9480, false),
    (0x00a7_94d0, false),
    (0x00a7_9540, false),
    (0x00a7_a5d0, false),
    (0x00a7_a640, false),
    (0x00a7_a410, false),
    (0x00a7_8f30, false),
    (0x00a7_a6b0, false),
    (0x00a7_a450, false),
    (0x0087_7dc0, false),
    (0x00a7_7e30, false),
    (0x00a7_7e80, false),
    (0x00a7_89a0, false),
    (0x00a7_85f0, false),
    (0x00a7_8620, false),
    (0x00a7_9450, false),
    (0x00a7_8640, false),
    (0x00a7_8670, false),
    (0x00a7_82e0, false),
    (0x00a7_7e20, false),
    (0x00a7_8360, false),
    (0x00a7_7c20, false),
    (0x00a7_a4a0, false),
    (0x00a7_a4f0, false),
    (0x00a7_9d30, false),
    (0x00a7_a540, false),
    (0x00a7_8f00, false),
    (0x00a7_a850, false),
    (0x00a7_9580, false),
    (0x00a7_9cc0, false),
    (0x00a7_9640, false),
    (0x00a7_a7e0, false),
    (0x00a7_95e0, false),
    (0x00a7_8c60, false),
    (0x00a7_9b30, false),
    (0x00a7_a680, false),
    (0x00a7_a070, false),
    (0x00a7_9cf0, false),
    (0x00a7_9fa0, false),
    (0x00a7_a8d0, false),
    (0x00a7_a5a0, false),
    (0x00a7_a940, false),
    (0x00a7_a000, false),
    (0x00a7_9bc0, false),
    (0x00a7_87e0, false),
    (0x00a7_96e0, false),
    (0x00a7_a120, false),
    (0x00a7_a880, false),
    (0x00a7_a700, false),
    (0x00a7_a760, false),
    (0x00a7_9220, false),
    (0x00a7_8d30, false),
    (0x00a7_7f00, false),
    (0x00a7_7f20, false),
    (0x00a7_7ce0, false),
    (0x00a7_81d0, false),
    (0x00a7_81f0, false),
    (0x00a7_8340, false),
    (0x00a7_7b10, false),
    (0x00a7_7d00, false),
    (0x0087_8510, false),
    (0x0087_8530, false),
    (0x00a7_8320, false),
    (0x00a7_7c80, false),
    (0x00a7_85b0, false),
    (0x00a7_7a10, false),
    (0x00a7_7be0, false),
    (0x00a7_7b60, false),
    (0x00a7_8370, false),
    (0x00a7_7cb0, false),
    (0x0087_93f0, false),
    (0x0087_8280, false),
    (0x0087_8e80, false),
    (0x0087_8ed0, false),
    (0x0087_8630, false),
    (0x0087_8600, false),
    (0x0087_93b0, false),
    (0x0087_8f70, false),
    (0x0087_86d0, false),
    (0x0087_84e0, false),
    (0x0087_87b0, false),
    (0x0087_87c0, false),
    (0x0087_93e0, false),
    (0x0087_81b0, false),
    (0x0087_81d0, false),
    (0x0087_80f0, false),
    (0x0087_8c10, false),
    (0x0087_8c60, false),
    (0x0076_eb80, false),
    (0x0076_ebe0, false),
    (0x0076_ec10, false),
    (0x0076_ecb0, false),
    (0x0076_ec90, false),
    (0x0076_ec20, false),
    (0x0076_ec30, false),
    (0x0076_ec40, false),
    (0x007b_83d0, false),
    (0x007b_85f0, false),
    (0x007b_88b0, false),
    (0x007b_8920, false),
    (0x007b_9870, false),
    (0x007b_9ca0, false),
    (0x007b_9960, false),
    (0x007b_8ad0, false),
    (0x007b_8ba0, false),
    (0x007b_8c30, false),
    (0x007b_8c00, false),
    (0x007b_8cc0, false),
    (0x007b_8d70, false),
    (0x007b_8b80, false),
    (0x007b_8e40, false),
    (0x007b_8e60, false),
    (0x007b_8ff0, false),
    (0x007b_8fc0, false),
    (0x007b_92c0, false),
    (0x007b_9300, false),
    (0x007b_8fa0, false),
    (0x007b_9150, false),
    (0x007b_8150, false),
    (0x007b_9610, false),
    (0x007b_94e0, false),
    (0x007b_9420, false),
    (0x007b_9440, false),
    (0x007b_a1f0, false),
    (0x007b_8790, false),
    (0x007b_91f0, true),
    (0x0087_7fd0, true),
    (0x00a7_99d0, true),
    (0x00a7_8e20, true),
    (0x00a7_79e0, true),
    (0x00a7_7ac0, true),
    (0x00a7_86e0, true),
    (0x0087_78b0, false),
    (0x0087_8e60, false),
];
