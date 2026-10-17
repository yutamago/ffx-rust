//! The natives of groups 1, 4, 5, 11 and 12 in Rust (script round 12):
//! math (group 1: sine, cosine, angle wrap, distances), the screen and
//! scene effects (group 4: `Sg_*` fades, flashes, focus, footstep
//! tables), the worker's character model (group 5: `Ch_*`, motion groups,
//! attach points, neck and mouth) and the movie player (group 11). Most pop
//! their arguments and hand them to one game function; the character ones
//! act only when the worker has a model (`AtelGetCharHandle`), and some
//! pop only then.

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
            let _ = ($w, $a, $s);
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
        call(h, 0x00a7_aae0, &[w, s, 1, 1]);
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
/// into the args word; `EAX` the args word's address.
pub fn native_chr_load_motion_group<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    h.write(a, n);
    a
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

/// The Rust version of a group 1, 4, 5, 11 or 12 handler, if there is one.
#[must_use]
#[allow(clippy::too_many_lines)] // one arm per handler
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    use Native::{Float, Int};
    Some(match sym {
        0x00a7_9820 => Int(native_chr_load_system_mgrp),
        0x0076_e980 | 0x00a7_9970 | 0x00a7_9a10 => Int(native_done),
        0x00a7_9980 => Int(native_chr_load_system_mgrp_result),
        0x00a7_9c50 => Int(native_chr_attach),
        0x00a7_9340 => Int(native_504f),
        0x00a7_9380 => Int(native_504f_poll),
        0x00a7_9210 | 0x00a7_9410 => Int(native_args_c),
        0x00a7_8f80 => Int(native_5061),
        0x00a7_81a0 => Int(native_401a),
        0x00a7_8130 => Int(native_4017),
        0x00a7_9e30 => Int(native_5076),
        0x0076_eb30 | 0x00a7_7ee0 => Int(native_nothing),
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
        _ => return None,
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 75] = [
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
];
