//! The map natives in Rust: group 8 (`0x8000`-`0x806b`, the field
//! programs `Fp_*`, map parts, fog and back colours, the map's sprite
//! objects) and three field natives (`0x01a5`, `0x01e4`, `0x01e6`). Most
//! pop their arguments, call the map module and return 1; the colour ones
//! pass the address of a stack buffer to the map module, which here is
//! [`NativeHost::scratch`].

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::x87;

use super::native::Native;
use super::run::NativeHost;
use super::{atel_pop_stack_float, atel_pop_stack_integer, atel_push_stack_integer};

fn pop(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_integer(h, s)
}

fn popf(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_float(h, s)
}

/// `AtelPopStackFloat`'s `ST0` before any store: an integer slot exactly,
/// a float slot as loaded.
pub(super) fn pop_st0(h: &mut impl NativeHost, s: u32) -> f64 {
    let d = h.read::<i32>(s);
    if d > 0 && h.read::<u8>(s + 0x54 + (d - 1).cast_unsigned()) == crate::vm::stack::INT {
        f64::from(pop(h, s).cast_signed())
    } else {
        x87::load(popf(h, s))
    }
}

fn call(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call(f, args)
}

/// `pop` `n` arguments (the first popped first) and call `f` with them in
/// the reverse order (the order they were pushed); returns 1.
fn pop_call<H: NativeHost>(h: &mut H, s: u32, n: usize, f: u32) -> u32 {
    let mut v: Vec<u32> = (0..n).map(|_| pop(h, s)).collect();
    v.reverse();
    call(h, f, &v);
    1
}

macro_rules! pop_calls {
    ($($(#[$m:meta])* $name:ident => $n:expr, $f:expr;)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
            pop_call(h, s, $n, $f)
        }
    )*};
}

pop_calls! {
    /// `Native_01a5_TexSwapSetFrame` (`0x008574c0`): `(a, b)` -> `0x00909e30(a, b)`; 1.
    native_01a5 => 2, 0x0090_9e30;
    /// `Native_806a` (`0x0091af80`): `0x0091a260(x)`; 1.
    native_806a => 1, 0x0091_a260;
    /// `Native_MapPartHide` (`0x0091b1e0`, `0x8010`): `0x0091a7c0(part)`; 1.
    native_map_part_hide => 1, 0x0091_a7c0;
    /// `Native_8011_MapPartSetOffset` (`0x0091b200`): `0x0091ab30(a, b, c)`; 1.
    native_8011 => 3, 0x0091_ab30;
    /// `Native_801d_MapPartSetFrame` (`0x0091b400`): `0x0091a930(a, b)`; 1.
    native_801d => 2, 0x0091_a930;
    /// `Native_801e` (`0x0091b430`): `0x0091a080(x)`; 1.
    native_801e => 1, 0x0091_a080;
    /// `Native_801f` (`0x0091b450`): `0x0091a030(x)`; 1.
    native_801f => 1, 0x0091_a030;
    /// `Native_8020_MapPartAnimOn` (`0x0091b470`): `0x00919f90(x)`; 1.
    native_8020 => 1, 0x0091_9f90;
    /// `Native_8021_MapPartAnimOff` (`0x0091b490`): `0x00919f40(x)`; 1.
    native_8021 => 1, 0x0091_9f40;
    /// `Native_8022_MapPartAnimMode` (`0x0091b4b0`): `0x00919fe0(a, b)`; 1.
    native_8022 => 2, 0x0091_9fe0;
    /// `Native_802e_MapPartAnimRate` (`0x0091b6a0`): `0x0091a9c0(a, b)`; 1.
    native_802e => 2, 0x0091_a9c0;
    /// `Native_802f_MapPartLastFrame` (`0x0091b6d0`): `0x0091a970(x)`; 1.
    native_802f => 1, 0x0091_a970;
    /// `Native_8030_MapPartSetAlpha` (`0x0091b6f0`): `0x0091aa90(a, b)`; 1.
    native_8030 => 2, 0x0091_aa90;
    /// `Native_805e_GuideSetMapNo` (`0x0091c4d0`): `Yn_GuideMapSetMapNo`'s caller `0x00919da0(x)`; 1.
    native_805e => 1, 0x0091_9da0;
    /// `Native_805f_FpFree` (`0x0091c4f0`): `0x007295f0(x)`; 1.
    native_805f => 1, 0x0072_95f0;
    /// `Native_8060_FpGroupForceOff` (`0x0091c510`): `0x00729480(x)`; 1.
    native_8060 => 1, 0x0072_9480;
    /// `Native_8066` (`0x0091c7c0`): `0x0072a750(x)`; 1.
    native_8066 => 1, 0x0072_a750;
    /// `Native_BltzSetObjModels` (`0x0091c810`, `0x8067`): `0x00919a80(a, b, c, d)`; 1.
    native_bltz_set_obj_models => 4, 0x0091_9a80;
}

/// `Native_8059_FpSetStickMap` (`0x0091c3c0`): `(a, b)` -> `0x0072a590(b, a)` (the top
/// first); 1.
pub fn native_8059<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let a = pop(h, s);
    call(h, 0x0072_a590, &[b, a]);
    1
}

/// `Native_01e4_SetFieldVec3Int` (`0x00858c90`): `(x, y, z, n)`: returns
/// `0x0090a470(x, y, z, n)`'s `EAX`.
pub fn native_01e4<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    let z = popf(h, s);
    let y = popf(h, s);
    let x = popf(h, s);
    call(h, 0x0090_a470, &[x, y, z, n])
}

/// `Native_01e6_SetFieldPair` (`0x008591d0`): `(n, x, y)`: `0x0090a430(n, x, y)`;
/// returns `n`.
pub fn native_01e6<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let y = popf(h, s);
    let x = popf(h, s);
    let n = pop(h, s);
    call(h, 0x0090_a430, &[n, x, y]);
    n
}

/// `Native_8005` (`0x0091c7e0`, start): `(a, b)` into the args words
/// (`+0` a, `+4` b); returns `a`.
pub fn native_8005<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let x = pop(h, s);
    h.write(a + 4, b);
    h.write(a, x);
    x
}

/// `Native_8004` (`0x0091af60`, start): pushes 1 and goes on as
/// [`native_8005`] (so `+4` is 1).
pub fn native_8004<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    atel_push_stack_integer(h, s, 1);
    native_8005(h, w, a, s)
}

/// `Native_8005_Poll` (`0x0091c860`): the state of sprite object `+0`
/// (`0x0072a760`): 1 not done, 2 done unless `+4` is set, others done (5).
pub fn native_8005_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let id = u32::from(h.read::<u16>(a));
    match call(h, 0x0072_a760, &[id]) {
        1 => 0,
        2 if h.read::<u32>(a + 4) != 0 => 0,
        _ => 5,
    }
}

/// `Native_8004_Poll` (`0x0091afa0`): [`native_8005_poll`].
pub fn native_8004_poll<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
    native_8005_poll(h, w, a, s)
}

/// `g_MapSpriteRefresh` (`0x025d5eb8`): 2 (or 1/0) before `0x00645e40`.
pub const MAP_SPRITE_REFRESH: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x025d_5eb8);

/// `Native_806b` (`0x0091afc0`): `(on)`: 2 into `0x025d5eb8` when on, then
/// `0x00645e40()`; 1.
pub fn native_806b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    if pop(h, s) != 0 {
        MAP_SPRITE_REFRESH.set(h, 2);
    }
    call(h, 0x0064_5e40, &[]);
    1
}

/// `Native_805c` (`0x0091c470`): `(on)`: `on != 0` into `0x025d5eb8`, then
/// `0x00645e40()`; 1.
pub fn native_805c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = u32::from(pop(h, s) != 0);
    MAP_SPRITE_REFRESH.set(h, on);
    call(h, 0x0064_5e40, &[]);
    1
}

/// `Native_8006_MapJumpToEntrance` (`0x0091aff0`, start): `(n)`, `0x00874000(n)`'s `EAX`; on
/// map `0x159c` at scenario `0x7e4` with `n` 2 it first sets `0x0193bf68`.
pub fn native_8006<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    if call(h, 0x0064_1da0, &[]) == 0x159c && call(h, 0x0078_7360, &[]) & 0xffff == 0x7e4 && n == 2
    {
        let at = h.rebase(0x0193_bf68);
        h.write(at, 1u32);
    }
    super::helpers::atel_jump_to_entrance(h, n)
}

/// `Native_8006_MapJumpToEntrance_Poll` (`0x0091b040`): done at once.
pub fn native_8006_poll<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    1
}

macro_rules! call_ones {
    ($($(#[$m:meta])* $name:ident => $f:expr;)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
            call(h, $f, &[]);
            1
        }
    )*};
}

call_ones! {
    /// `Native_8007` (`0x0091b050`): `0x00729740()`; 1.
    native_8007 => 0x0072_9740;
    /// `Native_8008` (`0x0091b060`): `0x007296a0()`; 1.
    native_8008 => 0x0072_96a0;
    /// `Native_802c` (`0x0091b680`): `0x0091a220()`; 1.
    native_802c => 0x0091_a220;
    /// `Native_802d` (`0x0091b690`): `0x0091a200()`; 1.
    native_802d => 0x0091_a200;
}

/// `Native_FpBind` (`0x0091b070`, `0x8009`): `(fp, bone)` ->
/// `AtelBindFp(worker, fp, bone)`; 1.
pub fn native_fp_bind<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let bone = pop(h, s);
    let fp = pop(h, s);
    super::helpers::atel_bind_fp(h, w, fp, bone);
    1
}

/// `Native_FpBindPos` (`0x0091b0a0`, `0x800a`): `(fp, x, y, z)`: the fp id
/// truncated from the bottom value, `AtelBindFpPos(worker, fp, x, y, z)` (the
/// `dbgPrintf` before it is an empty function); 1.
pub fn native_fp_bind_pos<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let z = popf(h, s);
    let y = popf(h, s);
    let x = popf(h, s);
    let v = pop_st0(h, s);
    let fp = x87::ftol(h, v);
    let f = h.scratch();
    super::helpers::atel_bind_fp_pos(h, w, fp, [x, y, z], f);
    1
}

/// `Native_FpUnbind` (`0x0091b130`, `0x800b`): `(fp)` (negative as 0) ->
/// `AtelBindFpOff(worker, fp)`; 1.
pub fn native_fp_unbind<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let fp = pop(h, s);
    let fp = if fp.cast_signed() < 0 { 0 } else { fp };
    super::helpers::atel_bind_fp_off(h, w, fp);
    1
}

/// `Native_800c_FpSetDisp` (`0x0091b160`): `(on)` -> `0x00729050(on != 0)`; returns
/// `on != 0`.
pub fn native_800c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = u32::from(pop(h, s) != 0);
    call(h, 0x0072_9050, &[on]);
    on
}

/// `Native_MapPartShow` (`0x0091b1a0`, `0x800f`): `(part)`, replaced by the
/// map's remap of it (`0x006412b0(0x006410e0(hash), hash)`) unless that is
/// -1, then `0x0091a830(part)`; 1.
pub fn native_map_part_show<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let part = pop(h, s);
    let hash = call(h, 0x0064_1da0, &[]);
    let x = call(h, 0x0064_10e0, &[hash]);
    let r = call(h, 0x0064_12b0, &[x, hash]);
    let part = if r == u32::MAX { part } else { r };
    call(h, 0x0091_a830, &[part]);
    1
}

/// `Native_8014` (`0x0091b280`): `(n, x)` -> `0x0091acc0(n, x)`; 1.
pub fn native_8014<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let x = popf(h, s);
    let n = pop(h, s);
    call(h, 0x0091_acc0, &[n, x]);
    1
}

/// `Native_8026` (`0x0091b550`): `(a, x, b, c, d)` (x a float) ->
/// `0x0091ad80(a, x, b, c, d)`; 1.
pub fn native_8026<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let d = pop(h, s);
    let c = pop(h, s);
    let b = pop(h, s);
    let x = popf(h, s);
    let a = pop(h, s);
    call(h, 0x0091_ad80, &[a, x, b, c, d]);
    1
}

/// `Native_8032_GuideMapSetLimit` (`0x0091b740`): `(x)` -> `0x00919df0(x)`; 1.
pub fn native_8032<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let x = popf(h, s);
    call(h, 0x0091_9df0, &[x]);
    1
}

/// `(id, on)`: `on_f(id)` or `off_f(id)`; returns `on`.
fn on_off<H: NativeHost>(h: &mut H, s: u32, on_f: u32, off_f: u32) -> u32 {
    let on = pop(h, s);
    let id = pop(h, s);
    call(h, if on != 0 { on_f } else { off_f }, &[id]);
    on
}

/// `Native_8035_FpRunGroup` (`0x0091b790`): `0x00729530`/`0x007294f0`.
pub fn native_8035<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    on_off(h, s, 0x0072_9530, 0x0072_94f0)
}

/// `Native_8036_FpShowGroup` (`0x0091b7d0`): `0x00729440`/`0x00729400`.
pub fn native_8036<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    on_off(h, s, 0x0072_9440, 0x0072_9400)
}

/// `Native_8001` (`0x0091c5b0`): `(on)`: `0x00909e20(0x10)` and
/// `0x00643df0(1)`, or `0x00909e10(0x10)` and `0x00643df0(0)`; returns
/// `on`.
pub fn native_8001<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = pop(h, s);
    if on != 0 {
        call(h, 0x0090_9e20, &[0x10]);
        call(h, 0x0064_3df0, &[1, 0x10]);
    } else {
        call(h, 0x0090_9e10, &[0x10]);
        call(h, 0x0064_3df0, &[0, 0x10]);
    }
    on
}

/// `Native_804f_MapPartScaleTo` (`0x0091c150`): `(a, x, b)` -> `0x0091a150(a, x, x, b)`; 1.
pub fn native_804f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let b = pop(h, s);
    let x = popf(h, s);
    let a = pop(h, s);
    call(h, 0x0091_a150, &[a, x, x, b]);
    1
}

/// `Native_805d` (`0x0091c4a0`): `0x007297e0()`, then `0x0091a930(i, 0)`
/// for each `i` below `0x0091a3d0()` (asked again each time); 1.
pub fn native_805d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0072_97e0, &[]);
    let mut i = 0u32;
    while i.cast_signed() < call(h, 0x0091_a3d0, &[]).cast_signed() {
        call(h, 0x0091_a930, &[i, 0]);
        i += 1;
    }
    1
}

/// `Native_805b` (`0x0091c410`): `(n, x, y, z)` ->
/// `pppFpSetStickPos(n, &[z, y, x])` (the floats in pop order); 1.
pub fn native_805b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let buf = h.scratch();
    for i in 0..3 {
        let v = popf(h, s);
        h.write(buf + 4 * i, v);
    }
    let n = pop(h, s);
    call(h, 0x0072_a630, &[n, buf]);
    1
}

/// `yiGetFogColor(&rgba)` (`0x00908980`).
pub const YI_GET_FOG_COLOR: u32 = 0x0090_8980;
/// `yiSetFogColor(&rgba)` (`0x0090a2c0`).
pub const YI_SET_FOG_COLOR: u32 = 0x0090_a2c0;
/// `yiGetBackColor(&rgba)` (`0x009086d0`).
pub const YI_GET_BACK_COLOR: u32 = 0x0090_86d0;
/// `yiSetBackColor(&rgba)` (`0x0090a040`).
pub const YI_SET_BACK_COLOR: u32 = 0x0090_a040;
/// `yiGetFogParam(&a, &b, &c)` (`0x00908bb0`).
pub const YI_GET_FOG_PARAM: u32 = 0x0090_8bb0;
/// `yiSetFogParam(a, b, c)` (`0x0090a350`).
pub const YI_SET_FOG_PARAM: u32 = 0x0090_a350;
/// `0x0091c8a0`: 1 on the stage whose id (`0x012fbc40`) is `0x140`.
pub const IS_STAGE_140: u32 = 0x0091_c8a0;

/// `255.0` (`0x00b170c0`, `f64`).
fn c255(h: &impl NativeHost) -> f64 {
    h.read(h.rebase(0x00b1_70c0))
}

/// After a fog colour is set: on stage `0x140`, the colour divided by 255
/// (each component truncated) into `0x00643150(r, g, b)` as floats.
fn fog_color_tail<H: NativeHost>(h: &mut H, buf: u32) {
    if call(h, IS_STAGE_140, &[]) != 1 {
        return;
    }
    let k = c255(h);
    let mut rgb = [0u32; 3];
    for (i, c) in (0..).zip(&mut rgb) {
        // fild, fstp m32, fld: the component as f32
        let v = h.read::<i32>(buf + 4 * i);
        let f = x87::load(x87::store(f64::from(v)));
        let t = x87::ftol(h, x87::div(f, k));
        *c = x87::store(f64::from(t.cast_signed()));
    }
    call(h, 0x0064_3150, &rgb);
}

/// `Native_803e` (`0x0091bad0`): `(r, g, b)` -> `yiSetFogColor(&[r, g, b,
/// 0])`, then [`fog_color_tail`]; 1.
pub fn native_803e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let buf = h.scratch();
    h.write(buf + 0xc, 0u32);
    for i in [2u32, 1, 0] {
        let v = pop(h, s);
        h.write(buf + 4 * i, v);
    }
    call(h, YI_SET_FOG_COLOR, &[buf]);
    fog_color_tail(h, buf);
    1
}

/// One fog colour component `i` set from the stack, the rest as they are.
fn set_fog_component<H: NativeHost>(h: &mut H, s: u32, i: u32) -> u32 {
    let buf = h.scratch();
    call(h, YI_GET_FOG_COLOR, &[buf]);
    let v = pop(h, s);
    h.write(buf + 4 * i, v);
    call(h, YI_SET_FOG_COLOR, &[buf]);
    fog_color_tail(h, buf);
    1
}

/// `Native_803f` (`0x0091bbb0`): the fog colour's red.
pub fn native_803f<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    set_fog_component(h, s, 0)
}

/// `Native_8040` (`0x0091bc80`): green.
pub fn native_8040<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    set_fog_component(h, s, 1)
}

/// `Native_8041` (`0x0091bd50`): blue.
pub fn native_8041<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    set_fog_component(h, s, 2)
}

fn get_rgba<H: NativeHost>(h: &mut H, f: u32, i: u32) -> u32 {
    let buf = h.scratch();
    call(h, f, &[buf]);
    h.read(buf + 4 * i)
}

/// `Native_8042` (`0x0091be20`): the fog colour's red.
pub fn native_8042<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    get_rgba(h, YI_GET_FOG_COLOR, 0)
}

/// `Native_8043` (`0x0091be50`): green.
pub fn native_8043<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    get_rgba(h, YI_GET_FOG_COLOR, 1)
}

/// `Native_8044` (`0x0091be80`): blue.
pub fn native_8044<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    get_rgba(h, YI_GET_FOG_COLOR, 2)
}

/// `Native_8049` (`0x0091c000`): the back colour's red.
pub fn native_8049<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    get_rgba(h, YI_GET_BACK_COLOR, 0)
}

/// `Native_804a` (`0x0091c030`): green.
pub fn native_804a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    get_rgba(h, YI_GET_BACK_COLOR, 1)
}

/// `Native_804b` (`0x0091c060`): blue.
pub fn native_804b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    get_rgba(h, YI_GET_BACK_COLOR, 2)
}

/// `Native_8045` (`0x0091beb0`): `(r, g, b)` -> `yiSetBackColor(&[r, g,
/// b, 0])`; 1.
pub fn native_8045<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let buf = h.scratch();
    h.write(buf + 0xc, 0u32);
    for i in [2u32, 1, 0] {
        let v = pop(h, s);
        h.write(buf + 4 * i, v);
    }
    call(h, YI_SET_BACK_COLOR, &[buf]);
    1
}

/// `yiGetFogParam` into the scratch: `[p0, p1, p2]` at `+0`, `+4`, `+8`.
fn fog_params<H: NativeHost>(h: &mut H) -> (u32, [u32; 3]) {
    let buf = h.scratch();
    call(h, YI_GET_FOG_PARAM, &[buf, buf + 4, buf + 8]);
    (buf, [h.read(buf), h.read(buf + 4), h.read(buf + 8)])
}

/// `Native_8039` (`0x0091b930`, float): the fog's first parameter.
pub fn native_8039<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> f64 {
    x87::load(fog_params(h).1[0])
}

/// `Native_803b` (`0x0091b9e0`, float): the second.
pub fn native_803b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> f64 {
    x87::load(fog_params(h).1[1])
}

/// `Native_803d` (`0x0091baa0`, float): the third.
pub fn native_803d<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> f64 {
    x87::load(fog_params(h).1[2])
}

/// The maps whose fog the script may not change (`0xca2`, `0x1e38`,
/// `0x699`, by `0x00641da0`).
fn fog_locked(hash: u32) -> bool {
    matches!(hash, 0xca2 | 0x1e38 | 0x699)
}

/// One fog parameter `i` from the stack (`yiSetFogParam`), then `f(x)`.
fn set_fog_param<H: NativeHost>(h: &mut H, s: u32, i: usize, f: u32, scale: bool) -> u32 {
    let (_, mut p) = fog_params(h);
    let hash = call(h, 0x0064_1da0, &[]);
    let x = popf(h, s);
    p[i] = x;
    if !fog_locked(hash) {
        call(h, YI_SET_FOG_PARAM, &p);
        let arg = if scale {
            x87::store(x87::div(x87::load(x), c255(h)))
        } else {
            x
        };
        call(h, f, &[arg]);
    }
    1
}

/// `Native_8038` (`0x0091b8b0`): the fog's first parameter, then
/// `0x006431e0(x)`; 1.
pub fn native_8038<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    set_fog_param(h, s, 0, 0x0064_31e0, false)
}

/// `Native_803a` (`0x0091b960`): the second, then `0x006431a0(x)`; 1.
pub fn native_803a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    set_fog_param(h, s, 1, 0x0064_31a0, false)
}

/// `Native_803c` (`0x0091ba10`): the third, then `0x006431c0(x / 255)`; 1.
pub fn native_803c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    set_fog_param(h, s, 2, 0x0064_31c0, true)
}

/// `Native_8037` (`0x0091b810`): `(a, b, c)` -> `yiSetFogParam(a, b, c)`;
/// on stage `0x140` also `0x006431e0(a)`, `0x006431a0(b)`,
/// `0x006431c0(c / 255)` and `0x00643180(1.0)`; 1.
pub fn native_8037<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let c = popf(h, s);
    let b = popf(h, s);
    let a = popf(h, s);
    call(h, YI_SET_FOG_PARAM, &[a, b, c]);
    if call(h, IS_STAGE_140, &[]) == 1 {
        call(h, 0x0064_31e0, &[a]);
        call(h, 0x0064_31a0, &[b]);
        let t = x87::store(x87::div(x87::load(c), c255(h)));
        call(h, 0x0064_31c0, &[t]);
        call(h, 0x0064_3180, &[1f32.to_bits()]);
    }
    1
}

/// The Rust version of a map native at `sym`.
#[allow(clippy::too_many_lines)] // a table
#[must_use]
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    use Native::{Float, Int};
    Some(match sym {
        0x0085_74c0 => Int(native_01a5),
        0x0085_8c90 => Int(native_01e4),
        0x0085_91d0 => Int(native_01e6),
        0x0091_af60 => Int(native_8004),
        0x0091_afa0 => Int(native_8004_poll),
        0x0091_c7e0 => Int(native_8005),
        0x0091_c860 => Int(native_8005_poll),
        0x0091_af80 => Int(native_806a),
        0x0091_afc0 => Int(native_806b),
        0x0091_aff0 => Int(native_8006),
        0x0091_b040 => Int(native_8006_poll),
        0x0091_b050 => Int(native_8007),
        0x0091_b060 => Int(native_8008),
        0x0091_b070 => Int(native_fp_bind),
        0x0091_b0a0 => Int(native_fp_bind_pos),
        0x0091_b130 => Int(native_fp_unbind),
        0x0091_b160 => Int(native_800c),
        0x0091_b1a0 => Int(native_map_part_show),
        0x0091_b1e0 => Int(native_map_part_hide),
        0x0091_b200 => Int(native_8011),
        0x0091_b280 => Int(native_8014),
        0x0091_b400 => Int(native_801d),
        0x0091_b430 => Int(native_801e),
        0x0091_b450 => Int(native_801f),
        0x0091_b470 => Int(native_8020),
        0x0091_b490 => Int(native_8021),
        0x0091_b4b0 => Int(native_8022),
        0x0091_b550 => Int(native_8026),
        0x0091_b680 => Int(native_802c),
        0x0091_b690 => Int(native_802d),
        0x0091_b6a0 => Int(native_802e),
        0x0091_b6d0 => Int(native_802f),
        0x0091_b6f0 => Int(native_8030),
        0x0091_b740 => Int(native_8032),
        0x0091_b790 => Int(native_8035),
        0x0091_b7d0 => Int(native_8036),
        0x0091_b810 => Int(native_8037),
        0x0091_b8b0 => Int(native_8038),
        0x0091_b930 => Float(native_8039),
        0x0091_b960 => Int(native_803a),
        0x0091_b9e0 => Float(native_803b),
        0x0091_ba10 => Int(native_803c),
        0x0091_baa0 => Float(native_803d),
        0x0091_bad0 => Int(native_803e),
        0x0091_bbb0 => Int(native_803f),
        0x0091_bc80 => Int(native_8040),
        0x0091_bd50 => Int(native_8041),
        0x0091_be20 => Int(native_8042),
        0x0091_be50 => Int(native_8043),
        0x0091_be80 => Int(native_8044),
        0x0091_beb0 => Int(native_8045),
        0x0091_c000 => Int(native_8049),
        0x0091_c030 => Int(native_804a),
        0x0091_c060 => Int(native_804b),
        0x0091_c150 => Int(native_804f),
        0x0091_c3c0 => Int(native_8059),
        0x0091_c410 => Int(native_805b),
        0x0091_c470 => Int(native_805c),
        0x0091_c4a0 => Int(native_805d),
        0x0091_c4d0 => Int(native_805e),
        0x0091_c4f0 => Int(native_805f),
        0x0091_c510 => Int(native_8060),
        0x0091_c5b0 => Int(native_8001),
        0x0091_c7c0 => Int(native_8066),
        0x0091_c810 => Int(native_bltz_set_obj_models),
        _ => return super::native_cam::find(sym),
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 65] = [
    (0x0085_74c0, false),
    (0x0085_8c90, false),
    (0x0085_91d0, false),
    (0x0091_af60, false),
    (0x0091_afa0, false),
    (0x0091_c7e0, false),
    (0x0091_c860, false),
    (0x0091_af80, false),
    (0x0091_afc0, false),
    (0x0091_aff0, false),
    (0x0091_b040, false),
    (0x0091_b050, false),
    (0x0091_b060, false),
    (0x0091_b070, false),
    (0x0091_b0a0, false),
    (0x0091_b130, false),
    (0x0091_b160, false),
    (0x0091_b1a0, false),
    (0x0091_b1e0, false),
    (0x0091_b200, false),
    (0x0091_b280, false),
    (0x0091_b400, false),
    (0x0091_b430, false),
    (0x0091_b450, false),
    (0x0091_b470, false),
    (0x0091_b490, false),
    (0x0091_b4b0, false),
    (0x0091_b550, false),
    (0x0091_b680, false),
    (0x0091_b690, false),
    (0x0091_b6a0, false),
    (0x0091_b6d0, false),
    (0x0091_b6f0, false),
    (0x0091_b740, false),
    (0x0091_b790, false),
    (0x0091_b7d0, false),
    (0x0091_b810, false),
    (0x0091_b8b0, false),
    (0x0091_b930, true),
    (0x0091_b960, false),
    (0x0091_b9e0, true),
    (0x0091_ba10, false),
    (0x0091_baa0, true),
    (0x0091_bad0, false),
    (0x0091_bbb0, false),
    (0x0091_bc80, false),
    (0x0091_bd50, false),
    (0x0091_be20, false),
    (0x0091_be50, false),
    (0x0091_be80, false),
    (0x0091_beb0, false),
    (0x0091_c000, false),
    (0x0091_c030, false),
    (0x0091_c060, false),
    (0x0091_c150, false),
    (0x0091_c3c0, false),
    (0x0091_c410, false),
    (0x0091_c470, false),
    (0x0091_c4a0, false),
    (0x0091_c4d0, false),
    (0x0091_c4f0, false),
    (0x0091_c510, false),
    (0x0091_c5b0, false),
    (0x0091_c7c0, false),
    (0x0091_c810, false),
];
