//! The main module (`ffx_sg`) against the original, on two oracles
//! started alike: one runs `FFX.exe`'s code, the other the Rust bridge on
//! the same memory; results (`EAX`, `AL`, or `ST0` as the original returns
//! them) and every written byte must agree. Floats compare bit for bit at
//! the game thread's x87 precision (control word `0x027f`); signalling
//! NaNs are left out (the emulator keeps them signalling through `fld`).
//!
//! Synthetic cases run on the image alone (`FFX_GAME_DIR`); the same cases
//! run on `FFX_SNAPSHOT` (field or battle) from the game's own state, with
//! the packet and table pointers moved to fresh blocks where a case writes
//! through them.
//!
//! Two code patches go into both oracles alike: the import thunk of
//! `_CIsqrt` (`0x009497b8`, no oracle stub) becomes `fsqrt; ret`, and
//! `fstp st(0); ret` goes in the padding at `0x0092b856` to pop a float
//! result. `crossAssertIsValidFloat`'s CRT calls (`_isnan`, `_finite`)
//! answer through their import stubs, fixed per case to what the CRT
//! returns.

#![allow(clippy::too_many_lines)] // case lists
#![allow(clippy::many_single_char_names)] // oracles, pointers, components

use std::path::PathBuf;

use ffx_bridge::{Global, Mem};
use ffx_oracle::{Conv, Oracle, Pages, Ret};
use ffx_sg::{assert, gs, light, state, vec};

const SQRT_THUNK: u32 = 0x0094_97b8;
const POP_STUB: u32 = 0x0092_b856;
/// `crossAssertIsValidFloat`'s import slots: `_isnan`, `_finite`.
const IAT_ISNAN: u32 = 0x00b0_c478;
const IAT_FINITE: u32 = 0x00b0_c474;
/// Padding (`int3` after the import thunks and after the function) where
/// the two slots point instead: `mov eax, answer; ret` each (the oracle's
/// own import stubs cannot be patched).
const ISNAN_STUB: u32 = 0x0094_97d6;
const FINITE_STUB: u32 = 0x006e_f30a;

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

fn prepare(mut a: Oracle, mut b: Oracle) -> (Oracle, Oracle) {
    for o in [&mut a, &mut b] {
        o.set_fpu_control(0x027f).unwrap();
        let at = o.rebase(SQRT_THUNK);
        o.write(at, &[0xd9, 0xfa, 0xc3]).unwrap(); // fsqrt; ret
        let at = o.rebase(POP_STUB);
        o.write(at, &[0xdd, 0xd8, 0xc3]).unwrap(); // fstp st(0); ret
    }
    (a, b)
}

fn synthetic() -> Option<(Oracle, Oracle)> {
    let exe = exe()?;
    Some(prepare(
        Oracle::load(&exe).unwrap(),
        Oracle::load(&exe).unwrap(),
    ))
}

fn snapshot() -> Option<(Oracle, Oracle)> {
    let exe = exe()?;
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    Some(prepare(
        Oracle::from_snapshot(&exe, &pages).unwrap(),
        Oracle::from_snapshot(&exe, &pages).unwrap(),
    ))
}

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

    /// A small signed value, sometimes any word.
    fn int(&mut self) -> i32 {
        match self.below(4) {
            0 => self.next().cast_signed(),
            _ => self.below(0x400).cast_signed() - 0x200,
        }
    }

    /// Float bits: ordinary values of mixed magnitudes, specials, quiet
    /// NaNs; no signalling NaN.
    fn float(&mut self) -> u32 {
        const SPECIAL: [u32; 10] = [
            0,
            0x8000_0000,
            0x3f80_0000,
            0xbf80_0000,
            0x7f80_0000,
            0xff80_0000,
            0x7f7f_ffff,
            0x0000_0001,
            0x7fc0_0000,
            0xffc1_2345,
        ];
        match self.below(10) {
            0 => SPECIAL[self.below(10) as usize],
            1 => self.next() & 0x807f_ffff, // denormal
            _ => {
                let e = self.below(40) + 107;
                self.next() & 0x807f_ffff | e << 23
            }
        }
    }
}

/// What a bridge returns, checked against what the original left.
trait Res {
    fn check(self, r: &Ret, what: &str);
}

impl Res for () {
    fn check(self, _: &Ret, _: &str) {}
}

impl Res for u32 {
    fn check(self, r: &Ret, what: &str) {
        assert_eq!(r.eax, self, "{what}: eax");
    }
}

impl Res for i32 {
    fn check(self, r: &Ret, what: &str) {
        assert_eq!(r.eax, self.cast_unsigned(), "{what}: eax");
    }
}

impl Res for u16 {
    fn check(self, r: &Ret, what: &str) {
        let [lo, hi, _, _] = r.eax.to_le_bytes();
        assert_eq!(u16::from_le_bytes([lo, hi]), self, "{what}: ax");
    }
}

impl Res for u8 {
    fn check(self, r: &Ret, what: &str) {
        assert_eq!(r.eax.to_le_bytes()[0], self, "{what}: al");
    }
}

impl Res for f32 {
    fn check(self, r: &Ret, what: &str) {
        let st0 = r.st0.unwrap_or_else(|| panic!("{what}: no float result"));
        #[allow(clippy::cast_possible_truncation)] // an f32 loaded or stored
        let st0 = st0 as f32;
        if self.is_nan() {
            assert!(st0.is_nan(), "{what}: {st0} for NaN");
        } else {
            assert_eq!(st0.to_bits(), self.to_bits(), "{what}: st0");
        }
    }
}

/// Runs `sym` (cdecl) in `a` and `rust` in `b` from the same state;
/// checks the result and every byte written. Pops a float result.
fn twin<R: Res>(
    what: &str,
    a: &mut Oracle,
    b: &mut Oracle,
    sym: u32,
    args: &[u32],
    rust: impl FnOnce(&mut Oracle) -> R,
) {
    let at = a.rebase(sym);
    let r = a
        .call(at, Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{what} ({sym:#010x}): {e}"));
    if r.st0.is_some() {
        let pop = a.rebase(POP_STUB);
        a.call(pop, Conv::Cdecl, &[]).unwrap();
    }
    rust(b).check(&r, what);
    assert_eq!(a.diff(b).unwrap(), [], "{what}: memory");
    settle(a, b);
}

/// Forgets the pages both sides wrote so far, after a diff found them
/// equal: a page neither side writes again stays equal, so later diffs need
/// only the pages written after this. Without it every diff re-read every
/// page any earlier case wrote, and the long tests spent most of their time
/// there.
fn settle(a: &mut Oracle, b: &mut Oracle) {
    a.clear_dirty();
    b.clear_dirty();
}

fn put<T: ffx_bridge::Value>(a: &mut Oracle, b: &mut Oracle, addr: u32, v: T) {
    Mem::write(a, addr, v);
    Mem::write(b, addr, v);
}

fn set<T: ffx_bridge::Value>(a: &mut Oracle, b: &mut Oracle, g: Global<T>, v: T) {
    g.set(a, v);
    g.set(b, v);
}

fn alloc(a: &mut Oracle, b: &mut Oracle, len: usize) -> u32 {
    let x = a.alloc(len).unwrap();
    assert_eq!(b.alloc(len).unwrap(), x);
    x
}

/// Random bytes in a fresh block of both.
fn block(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, len: usize) -> u32 {
    let p = alloc(a, b, len);
    let bytes: Vec<u8> = (0..len).map(|_| rng.next().to_le_bytes()[0]).collect();
    a.write(p, &bytes).unwrap();
    b.write(p, &bytes).unwrap();
    p
}

/// A block of random floats.
fn floats(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, n: u32) -> u32 {
    let p = alloc(a, b, 4 * n as usize);
    for k in 0..n {
        let f = rng.float();
        put(a, b, p + 4 * k, f);
    }
    p
}

type Get<R> = (&'static str, u32, fn(&Oracle) -> R);
type Set<R> = (&'static str, u32, fn(&mut Oracle, u32) -> R);
type Call0<R> = (&'static str, u32, fn(&mut Oracle) -> R);

/// Byte switches and the getters reading them.
const BYTE_GLOBALS: [u32; 30] = [
    0x012f_b8f8,
    0x012f_b8f9,
    0x012f_b8fb,
    0x012f_b8fe,
    0x012f_b8ff,
    0x012f_bbb0,
    0x012f_bbb1,
    0x012f_bbb2,
    0x012f_bbb3,
    0x012f_bbb4,
    0x012f_bbb6,
    0x012f_bbb7,
    0x012f_bbb9,
    0x012f_bbba,
    0x012f_bbbb,
    0x012f_bc34,
    0x00c4_96d4,
    0x0130_07b0,
    0x0130_07b3,
    0x0130_07eb,
    0x0130_181c,
    0x0130_181e,
    0x0130_34b8,
    0x0130_34b9,
    0x0133_c910,
    0x0130_070e,
    0x0130_070f,
    0x0130_071e,
    0x0130_071f,
    0x00c4_9718,
];

const GET_I32: [Get<i32>; 19] = [
    ("Sg_EnableGetCalcCam", 0x0082_0490, |m| {
        state::bridge::get_calc_cam(m)
    }),
    ("Sg_GetDispMode", 0x0082_0550, |m| {
        state::bridge::get_disp_mode(m)
    }),
    ("Sg_EnableGetTkAnti", 0x0082_0570, |m| {
        state::bridge::get_tk_anti(m)
    }),
    ("Sg_GetKeepFps", 0x0082_05a0, |m| {
        state::bridge::get_keep_fps(m)
    }),
    ("Sg_GetMapAlphaFast", 0x0082_05d0, |m| {
        state::bridge::get_map_alpha_fast(m)
    }),
    ("Sg_GetPause", 0x0082_05e0, |m| state::bridge::get_pause(m)),
    ("Sg_GetViewerMode", 0x0082_0640, |m| {
        state::bridge::get_viewer_mode(m)
    }),
    ("Sg_GetthDispEnable", 0x0082_0650, |m| {
        state::bridge::get_th_disp_enable(m)
    }),
    ("Sg_GuiGetDrawChr", 0x0082_0660, |m| {
        state::bridge::gui_get_draw_chr(m)
    }),
    ("Sg_GetExecFrames", 0x0082_d810, |m| {
        state::bridge::get_exec_frames(m)
    }),
    ("Sg_GetExecStep", 0x0082_d820, |m| {
        state::bridge::get_exec_step(m)
    }),
    ("Sg_GetLRButtonMode", 0x0082_d840, |m| {
        state::bridge::get_lr_button_mode(m)
    }),
    ("Sg_GuiGetTextureAnim", 0x0083_1930, |m| {
        state::bridge::gui_get_texture_anim(m)
    }),
    ("Sg_CaptureGetSelect", 0x0083_a640, |m| {
        state::bridge::capture_get_select(m)
    }),
    ("Sg_IdExistShade", 0x0084_4bb0, |m| {
        state::bridge::id_exist_shade(m)
    }),
    ("Sg_MapGetEnableShade", 0x0084_4ce0, |m| {
        state::bridge::map_get_enable_shade(m)
    }),
    ("Sg_AccGetAlpha", 0x0082_bcc0, |m| {
        state::bridge::acc_get_alpha(m)
    }),
    ("Sg_GetBlur", 0x0082_cd20, |m| state::bridge::get_blur(m)),
    ("Ch_GuiGetMouthTh", 0x0083_4070, |m| {
        state::bridge::chr_gui_get_mouth_th(m)
    }),
];

/// Word globals the word getters read.
const WORD_GLOBALS: [u32; 18] = [
    0x0112_8ac8,
    0x00c4_94a8,
    0x023c_44b4,
    0x0130_0788,
    0x0130_1818,
    0x00c4_98dc,
    0x0130_1a94,
    0x00c4_df50,
    0x0130_3690,
    0x0130_3694,
    0x0132_412c,
    0x0132_525c,
    0x0130_3344,
    0x0130_076c,
    0x0130_1804,
    0x0130_1940,
    0x0130_1974,
    0x0130_34b0,
];

const GET_U32: [Get<u32>; 19] = [
    ("get_read_buffer", 0x0076_dc50, |m| {
        state::bridge::get_read_buffer(m)
    }),
    ("Sg_GetUserName", 0x0082_0610, |m| {
        state::bridge::get_user_name(m)
    }),
    ("Sg_GetIgnoreID", 0x0082_d830, |m| {
        state::bridge::get_ignore_id(m)
    }),
    ("Sg_GetPlayer", 0x0082_d860, |m| {
        state::bridge::get_player(m)
    }),
    ("Sg_PcReadGetLen", 0x0083_ae70, |m| {
        state::bridge::pc_read_get_len(m)
    }),
    ("Sg_PacketGetPtr", 0x0083_b650, |m| {
        state::bridge::packet_get_ptr(m)
    }),
    ("Sg_IdGetPtr", 0x0083_e970, |m| state::bridge::id_get_ptr(m)),
    ("Sg_MapGetDirSelect", 0x0084_4cd0, |m| {
        state::bridge::map_get_dir_select(m)
    }),
    ("sg_getnorm", 0x0084_5c50, |m| state::bridge::get_norm(m)),
    ("sg_getvtex", 0x0084_5c60, |m| state::bridge::get_vtex(m)),
    ("sg_GetY", 0x0085_0830, |m| state::bridge::get_y(m)),
    ("Sg_MouseGetMove", 0x0085_1910, |m| {
        state::bridge::mouse_get_move(m)
    }),
    ("SendSpadToMFIFO", 0x0084_3200, |m| {
        state::bridge::send_spad_to_mfifo(m)
    }),
    ("Sg_GetFuncName", 0x0084_56c0, |m| {
        state::bridge::get_func_name(m)
    }),
    ("Sg_GetSpadPtr", 0x0084_3220, |m| {
        state::bridge::get_spad_ptr(m)
    }),
    ("Sg_GetFocusEnable", 0x0082_cd40, |m| {
        state::bridge::get_focus_enable(m)
    }),
    ("Sg_CaptureGetGuiHide", 0x0083_a620, |m| {
        state::bridge::capture_get_gui_hide(m)
    }),
    ("Sg_VramGetAddrZ", 0x0082_ce80, |_| {
        state::bridge::vram_get_addr_z()
    }),
    ("Sg_TextureGetSpecular", 0x0083_d3a0, |m| {
        state::bridge::texture_get_specular(m)
    }),
];

const ENABLE: [Set<i32>; 10] = [
    ("Sg_EnableCalcCam", 0x0082_03a0, |m, v| {
        state::bridge::enable_calc_cam(m, v)
    }),
    ("Sg_EnableDrawChr", 0x0082_03c0, |m, v| {
        state::bridge::enable_draw_chr(m, v)
    }),
    ("Sg_EnableCalcChr", 0x0082_03e0, |m, v| {
        state::bridge::enable_calc_chr(m, v)
    }),
    ("Sg_EnableDrawMap", 0x0082_0400, |m, v| {
        state::bridge::enable_draw_map(m, v)
    }),
    ("Sg_EnableDrawGui", 0x0082_0420, |m, v| {
        state::bridge::enable_draw_gui(m, v)
    }),
    ("Sg_EnableCalcMap", 0x0082_0440, |m, v| {
        state::bridge::enable_calc_map(m, v)
    }),
    ("Sg_EnableDrawMenu", 0x0082_0460, |m, v| {
        state::bridge::enable_draw_menu(m, v)
    }),
    ("Sg_EnableTkAnti", 0x0082_0500, |m, v| {
        state::bridge::enable_tk_anti(m, v)
    }),
    ("Sg_EnableTkAnti_0083add0", 0x0083_add0, |m, v| {
        state::bridge::set_enable_tk_anti(m, v)
    }),
    ("Sg_SetKeepFps", 0x0082_1ae0, |m, v| {
        state::bridge::set_keep_fps(m, v)
    }),
];

const SET_BYTE: [Set<()>; 20] = [
    ("Sg_EffectRoutineEnable", 0x0082_0380, |m, v| {
        state::bridge::effect_routine_enable(m, v);
    }),
    ("Sg_GuiSetDrawChr", 0x0082_0670, |m, v| {
        state::bridge::gui_set_draw_chr(m, v);
    }),
    ("Sg_SetPause", 0x0082_1cb0, |m, v| {
        state::bridge::set_pause(m, v);
    }),
    ("Sg_SetthDispEnable", 0x0082_1d50, |m, v| {
        state::bridge::set_th_disp_enable(m, v);
    }),
    ("Sg_AccSetDrawMap", 0x0082_bcb0, |m, v| {
        state::bridge::acc_set_draw_map(m, v);
    }),
    ("Sg_AccSetDrawChr", 0x0082_be00, |m, v| {
        state::bridge::acc_set_draw_chr(m, v);
    }),
    ("Sg_AccEnableBG", 0x0082_be10, |m, v| {
        state::bridge::acc_enable_bg(m, v);
    }),
    ("Sg_DrawSetClearIgnore", 0x0082_c7a0, |m, v| {
        state::bridge::draw_set_clear_ignore(m, v);
    }),
    ("Sg_SetExecSpecialSpeed", 0x0082_da90, |m, v| {
        state::bridge::set_exec_special_speed(m, v);
    }),
    ("Sg_SetLRButtonMode", 0x0082_daa0, |m, v| {
        state::bridge::set_lr_button_mode(m, v);
    }),
    ("Sg_SetMovieGomiAvoid", 0x0083_0150, |m, v| {
        state::bridge::set_movie_gomi_avoid(m, v);
    }),
    ("Sg_GuiSetTextureAnim", 0x0083_1940, |m, v| {
        state::bridge::gui_set_texture_anim(m, v);
    }),
    ("Ch_SetShadeByLight", 0x0083_1d00, |m, v| {
        state::bridge::chr_set_shade_by_light(m, v);
    }),
    ("Sg_CaptureSetSelect", 0x0083_a680, |m, v| {
        state::bridge::capture_set_select(m, v);
    }),
    ("Sg_MemSetCheck", 0x0083_c4d0, |m, v| {
        state::bridge::mem_set_check(m, v);
    }),
    ("Sg_MemSetFast", 0x0083_c520, |m, v| {
        state::bridge::mem_set_fast(m, v);
    }),
    ("Sg_IdSetEffectTableIndex", 0x0083_e9d0, |m, v| {
        state::bridge::id_set_effect_table_index(m, v);
    }),
    ("Sg_MapSetEnableShade", 0x0084_4fa0, |m, v| {
        state::bridge::map_set_enable_shade(m, v);
    }),
    ("read_err_disp_set", 0x0076_e5e0, |m, v| {
        state::bridge::read_err_disp_set(m, v);
    }),
    ("set_hdd_err", 0x0077_9f70, |m, v| {
        state::bridge::set_hdd_err(m, v);
    }),
];

const SET_WORD: [Set<u32>; 6] = [
    ("enableGameControlTextureAnimation", 0x0083_6710, |m, v| {
        state::bridge::enable_game_control_texture_animation(m, v)
    }),
    ("Sg_MemSetErrorIgnore", 0x0083_c510, |m, v| {
        state::bridge::mem_set_error_ignore(m, v)
    }),
    ("Sg_MapSetDirSelect", 0x0084_4f90, |m, v| {
        state::bridge::map_set_dir_select(m, v)
    }),
    ("Sg_MemSetMoveHeapStartAddr", 0x0083_c530, |m, v| {
        state::bridge::mem_set_move_heap_start_addr(m, v)
    }),
    ("Sg_EnFocus", 0x0082_cb60, |m, v| {
        state::bridge::en_focus(m, v)
    }),
    ("SetPrintfFunc", 0x0085_10c0, |m, v| {
        state::bridge::set_printf_func(m, v)
    }),
];

const CALL_U32: [Call0<u32>; 3] = [
    ("Sg_FilterReset", 0x0082_cc70, |m| {
        state::bridge::filter_reset(m)
    }),
    ("Sg_FlashReset", 0x0082_cce0, |m| {
        state::bridge::flash_reset(m)
    }),
    ("Sg_Id_coltest", 0x0084_4cb0, |m| {
        state::bridge::id_coltest(m)
    }),
];

const CALL_VOID: [Call0<()>; 4] = [
    ("Sg_EnableEffectMatirx", 0x0082_0480, |m| {
        state::bridge::enable_effect_matrix(m);
    }),
    ("Sg_ToggleTkAnti", 0x0082_1e30, |m| {
        state::bridge::toggle_tk_anti(m);
    }),
    ("Sg_GlobalInitWind", 0x0082_4e10, |m| {
        vec::bridge::global_init_wind(m);
    }),
    ("Sg_InitCamera", 0x0083_f780, |m| {
        vec::bridge::init_camera(m);
    }),
];

/// An argument: small values, bytes with high bits, any word.
fn arg(rng: &mut Xorshift) -> u32 {
    match rng.below(4) {
        0 => rng.below(3),
        1 => rng.below(0x100),
        _ => rng.next(),
    }
}

/// Randomizes the switches and plain words (not pointers the code follows).
fn scramble(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    for g in BYTE_GLOBALS {
        let v = rng.next().to_le_bytes()[0];
        put(a, b, a.rebase(g), v);
    }
    for g in WORD_GLOBALS {
        let v = arg(rng);
        put(a, b, a.rebase(g), v);
    }
}

fn accessors(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32, randomize: bool) {
    for round in 0..rounds {
        if randomize || round > 0 {
            scramble(a, b, rng);
        }
        for (name, sym, f) in GET_I32 {
            twin(name, a, b, sym, &[], |m| f(m));
        }
        for (name, sym, f) in GET_U32 {
            twin(name, a, b, sym, &[], |m| f(m));
        }
        twin("isDebugMode", a, b, 0x0082_2550, &[], |m| {
            state::bridge::is_debug_mode(m)
        });
        for (name, sym, f) in ENABLE {
            let v = arg(rng);
            twin(&format!("{name}({v:#x})"), a, b, sym, &[v], |m| f(m, v));
        }
        for (name, sym, f) in SET_BYTE {
            let v = arg(rng);
            twin(&format!("{name}({v:#x})"), a, b, sym, &[v], |m| f(m, v));
        }
        for (name, sym, f) in SET_WORD {
            let v = arg(rng);
            twin(&format!("{name}({v:#x})"), a, b, sym, &[v], |m| f(m, v));
        }
        for (name, sym, f) in CALL_U32 {
            twin(name, a, b, sym, &[], f);
        }
        for (name, sym, f) in CALL_VOID {
            twin(name, a, b, sym, &[], f);
        }
        let mode = state::bridge::ID_DRAW;
        let v = rng.int();
        set(a, b, mode, v);
        twin(
            &format!("Sg_SwitchDrawId from {v}"),
            a,
            b,
            0x0084_5010,
            &[],
            state::bridge::switch_draw_id,
        );
        let n = rng.int();
        twin(
            &format!("Sg_SetExecFrames({n})"),
            a,
            b,
            0x0082_da60,
            &[n.cast_unsigned()],
            |m| {
                state::bridge::set_exec_frames(m, n);
            },
        );
        let (x, y, z) = (rng.next(), rng.next(), rng.next());
        twin("Sg_SetTkAntiXYB", a, b, 0x0083_b110, &[x, y, z], |m| {
            state::bridge::set_tk_anti_xyb(m, x, y, z)
        });
        twin("sg_setvtex_and_normal", a, b, 0x0084_5ca0, &[x, y], |m| {
            state::bridge::set_vtex_and_normal(m, x, y)
        });
        let i = rng.below(8);
        twin(&format!("Sg_GetFocus({i})"), a, b, 0x0082_cd30, &[i], |m| {
            state::bridge::get_focus(m, i)
        });
        let i = rng.below(64);
        twin(
            &format!("Sg_GetResourceSTR({i})"),
            a,
            b,
            0x0083_fc30,
            &[i],
            |m| state::bridge::get_resource_str(m, i),
        );
        let which = rng.below(2);
        twin(
            &format!("Sg_MapGetName({which})"),
            a,
            b,
            0x0084_4cf0,
            &[which],
            |m| state::bridge::map_get_name(m, which),
        );
        let n = arg(rng);
        twin(
            &format!("sa_task_buff_size_get({n:#x})"),
            a,
            b,
            0x0077_a180,
            &[n],
            |_| state::bridge::sa_task_buff_size_get(n),
        );
        let dc = if rng.below(4) == 0 { 0 } else { alloc(a, b, 4) };
        twin("Sg_GetDBuffDC", a, b, 0x0082_0530, &[dc], |m| {
            state::bridge::get_dbuff_dc(m, dc)
        });
        let p = block(a, b, rng, 2);
        twin("ReadU16Unaligned", a, b, 0x0083_8220, &[p], |m| {
            state::bridge::read_u16_unaligned(m, p)
        });
        // Sg_IdSetEffectTable: a set index of -2..=5, an entry of 0..4.
        let set_index = (rng.below(8).cast_signed() - 2).to_le_bytes()[0];
        set(a, b, state::bridge::ID_EFFECT_TABLE_INDEX, set_index);
        let (i, v) = (rng.below(4), arg(rng));
        twin("Sg_IdSetEffectTable", a, b, 0x0083_e9b0, &[i, v], |m| {
            state::bridge::id_set_effect_table(m, i, v)
        });
        let status = rng.next();
        set(a, b, ffx_field::yi::RENDER_STATUS, status);
        twin("Sg_GetIsPrerend", a, b, 0x0084_3210, &[], |m| {
            state::bridge::get_is_prerend(ffx_field::yi::bridge::yi_get_render_status(m))
        });
        let z = rng.float();
        put(a, b, a.rebase(0x0130_1a90), z);
        twin("Sg_IdGetZoom", a, b, 0x0083_e9a0, &[], |m| {
            state::bridge::id_get_zoom(m)
        });
    }
}

fn points(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32, keep_table: bool) {
    let table = light::bridge::PLIGHTS.get(a);
    let last = light::bridge::PLIGHT_LAST.get(a);
    let real = keep_table && table != 0 && (0..64).contains(&last);
    for _ in 0..rounds {
        // A table of 16 lights with room for indexes -2 and -1 before it.
        let last = if real {
            last
        } else {
            // Floats throughout (no signalling NaN), bytes where the flags are.
            let base = floats(a, b, rng, 18 * 11);
            set(a, b, light::bridge::PLIGHTS, base + 2 * 0x2c);
            let last = rng.below(16).cast_signed() - 1;
            set(a, b, light::bridge::PLIGHT_LAST, last);
            last
        };
        let lo = if real { 0 } else { -2 };
        let i = rng.below((last + 2 - lo).cast_unsigned()).cast_signed() + lo;
        let iu = i.cast_unsigned();
        let what = |n: &str| format!("{n}({i}) last {last}");
        twin(
            &what("Sg_PLightGetAttenuation"),
            a,
            b,
            0x0083_1a40,
            &[iu],
            |m| light::bridge::get_attenuation(m, i),
        );
        twin(&what("Sg_PLightGetRadius"), a, b, 0x0083_1b80, &[iu], |m| {
            light::bridge::get_radius(m, i)
        });
        twin(&what("Sg_PLightGetEnable"), a, b, 0x0083_1ad0, &[iu], |m| {
            light::bridge::get_enable(m, i)
        });
        twin(&what("Sg_PLightGetLight"), a, b, 0x0083_1b00, &[iu], |m| {
            light::bridge::get_light(m, i)
        });
        let out: [u32; 3] =
            std::array::from_fn(|_| if rng.below(4) == 0 { 0 } else { alloc(a, b, 4) });
        twin(
            &what("Sg_PLightGetPos"),
            a,
            b,
            0x0083_1b30,
            &[iu, out[0], out[1], out[2]],
            |m| {
                light::bridge::get_pos(m, i, out);
            },
        );
        twin(
            &what("Sg_PLightGetColor"),
            a,
            b,
            0x0083_1a60,
            &[iu, out[0], out[1], out[2]],
            |m| {
                light::bridge::get_color(m, i, out);
            },
        );
        if real {
            // Leave the game's lights as they are on the snapshot.
            continue;
        }
        let f = rng.float();
        twin(
            &what("Sg_PLightSetAttenuation"),
            a,
            b,
            0x0083_1bf0,
            &[iu, f],
            |m| {
                light::bridge::set_attenuation(m, i, f32::from_bits(f));
            },
        );
        let f = rng.float();
        twin(
            &what("Sg_PLightSetRadius"),
            a,
            b,
            0x0083_1ce0,
            &[iu, f],
            |m| {
                light::bridge::set_radius(m, i, f32::from_bits(f));
            },
        );
        let v = arg(rng);
        twin(
            &what("Sg_PLightSetEnable"),
            a,
            b,
            0x0083_1c60,
            &[iu, v],
            |m| light::bridge::set_enable(m, i, v),
        );
        let v = arg(rng);
        twin(
            &what("Sg_PLightSetLight"),
            a,
            b,
            0x0083_1c80,
            &[iu, v],
            |m| light::bridge::set_light(m, i, v),
        );
        let mask = rng.below(8);
        let xyz = [rng.float(), rng.float(), rng.float()];
        twin(
            &what("Sg_PLightSetPos"),
            a,
            b,
            0x0083_1ca0,
            &[iu, mask, xyz[0], xyz[1], xyz[2]],
            |m| {
                light::bridge::set_pos(m, i, mask, xyz.map(f32::from_bits));
            },
        );
        let rgb = [rng.int(), rng.int(), rng.int()];
        let args = [
            iu,
            rgb[0].cast_unsigned(),
            rgb[1].cast_unsigned(),
            rgb[2].cast_unsigned(),
        ];
        twin(&what("Sg_PLightSetColor"), a, b, 0x0083_1c10, &args, |m| {
            light::bridge::set_color(m, i, rgb);
        });
        twin(
            &what("Sg_PLightGetColor after set"),
            a,
            b,
            0x0083_1a60,
            &[iu, out[0], out[1], out[2]],
            |m| {
                light::bridge::get_color(m, i, out);
            },
        );
    }
    twin("Sg_PLightGetPtr", a, b, 0x0083_1b70, &[], |m| {
        light::bridge::get_ptr(m)
    });
    twin("Sg_LightGetColorMatrixPtr", a, b, 0x0083_61b0, &[], |m| {
        light::bridge::get_color_matrix_ptr(m)
    });
    twin("Sg_LightGetNormalMatrixPtr", a, b, 0x0083_6240, &[], |m| {
        light::bridge::get_normal_matrix_ptr(m)
    });
    twin("Sg_GetSuimen", a, b, 0x0083_60a0, &[], |m| {
        light::bridge::get_suimen(m)
    });
    for _ in 0..rounds {
        let h = rng.float();
        twin("Sg_SetSuimen", a, b, 0x0083_6640, &[h], |m| {
            light::bridge::set_suimen(m, f32::from_bits(h));
        });
        twin("Sg_GetSuimen", a, b, 0x0083_60a0, &[], |m| {
            light::bridge::get_suimen(m)
        });
        let h = rng.float();
        twin("Sg_SetWaterLimit", a, b, 0x0083_6660, &[h], |m| {
            light::bridge::set_water_limit(m, f32::from_bits(h));
        });
    }
}

fn vectors(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    for _ in 0..rounds {
        let va = floats(a, b, rng, 4);
        let vb = floats(a, b, rng, 4);
        // The output a fresh block, or one of the inputs.
        let out = match rng.below(3) {
            0 => va,
            1 => vb,
            _ => alloc(a, b, 16),
        };
        twin("Sg_VecAdd", a, b, 0x0082_4ce0, &[out, va, vb], |m| {
            vec::bridge::add(m, out, va, vb);
        });
        twin("Sg_VecSub", a, b, 0x0082_4da0, &[out, va, vb], |m| {
            vec::bridge::sub(m, out, va, vb);
        });
        twin("Sg_VecDot", a, b, 0x0082_4d40, &[va, vb, out], |m| {
            vec::bridge::dot(m, va, vb, out);
        });
        let s = rng.float();
        twin("Sg_VecScale", a, b, 0x0082_4d70, &[out, va, s], |m| {
            vec::bridge::scale(m, out, va, f32::from_bits(s));
        });
        twin("Sg_VecCopy4", a, b, 0x0082_4d10, &[out, vb], |m| {
            vec::bridge::copy4(m, out, vb);
        });
        let (x, y, z) = (rng.float(), rng.float(), rng.float());
        twin("Sg_GlobalSetWind", a, b, 0x0082_4e30, &[x, y, z], |m| {
            vec::bridge::global_set_wind(
                m,
                f32::from_bits(x),
                f32::from_bits(y),
                f32::from_bits(z),
            );
        });
        let r = rng.float();
        twin("Sg_RadToDeg", a, b, 0x0083_66f0, &[r], |m| {
            vec::bridge::rad_to_deg(m, f32::from_bits(r))
        });
        twin("Sg_VectorGetLength", a, b, 0x0083_6670, &[va], |m| {
            vec::bridge::vector_get_length(m, va)
        });
        let x = rng.float();
        for (slot, stub, value) in [
            (IAT_ISNAN, ISNAN_STUB, f32::from_bits(x).is_nan()),
            (IAT_FINITE, FINITE_STUB, f32::from_bits(x).is_finite()),
        ] {
            for o in [&mut *a, &mut *b] {
                let at = o.rebase(stub);
                Mem::write(o, o.rebase(slot), at);
                o.stub_return(at, u32::from(value)).unwrap();
            }
        }
        twin(
            &format!("crossAssertIsValidFloat({x:#x})"),
            a,
            b,
            0x006e_f2d0,
            &[x],
            |_| assert::is_valid_float(f32::from_bits(x)),
        );
    }
}

fn packet(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use gs::bridge as g;
    let i = |v: i32| v.cast_unsigned();
    for _ in 0..rounds {
        // A fresh packet of 64 quadwords, a count inside it.
        let p = block(a, b, rng, 64 * 16);
        set(a, b, g::PACKET, p);
        set(a, b, g::COUNT, 3 + rng.below(40));
        let (w, h) = (rng.int(), rng.int());
        twin("Sg_SetScreenWH", a, b, 0x0082_1d10, &[i(w), i(h)], |m| {
            g::set_screen_wh(m, w, h)
        });
        if rng.below(2) == 0 {
            let (w, h) = (
                640 + rng.below(4).cast_signed() * 64,
                448 + rng.below(3).cast_signed(),
            );
            set(a, b, g::SCREEN_W, w);
            set(a, b, g::SCREEN_H, h);
        }
        twin("Sg_GsGet", a, b, 0x0084_1a30, &[], |m| g::get(m));
        twin("Sg_GsStart", a, b, 0x0084_1d10, &[], g::start);
        twin("Sg_GsInc", a, b, 0x0084_1a50, &[], g::inc);
        twin("Sg_GsTEXFLUSH", a, b, 0x0084_1ee0, &[], g::texflush);
        twin("Sg_GsDIMX", a, b, 0x0084_16f0, &[], g::dimx);
        let v = rng.int();
        twin("Sg_GsPRIM", a, b, 0x0084_1a60, &[i(v)], |m| g::prim(m, v));
        twin("Sg_GsDTHE", a, b, 0x0084_1730, &[i(v)], |m| g::dthe(m, v));
        let c: [i32; 5] = std::array::from_fn(|_| rng.int());
        twin(
            "Sg_GsRGBAQ",
            a,
            b,
            0x0084_1aa0,
            &[i(c[0]), i(c[1]), i(c[2]), i(c[3])],
            |m| g::rgbaq(m, c[0], c[1], c[2], c[3]),
        );
        twin("Sg_GsALPHA", a, b, 0x0084_1670, &c.map(i), |m| {
            g::alpha(m, [c[0], c[1], c[2], c[3]], c[4])
        });
        let t: [i32; 8] = std::array::from_fn(|_| {
            if rng.below(3) == 0 {
                rng.int()
            } else {
                rng.below(4).cast_signed()
            }
        });
        twin("Sg_GsTEST", a, b, 0x0084_1d80, &t.map(i), |m| {
            g::test_1(m, t)
        });
        twin("Sg_GsTEST2", a, b, 0x0084_1e30, &t.map(i), |m| {
            g::test_2(m, t)
        });
        let s: [i32; 4] = std::array::from_fn(|_| rng.int());
        twin("Sg_GsSCISSOR", a, b, 0x0084_1b90, &s.map(i), |m| {
            g::scissor_1(m, s[0], s[1], s[2], s[3])
        });
        twin("Sg_GsSCISSOR2", a, b, 0x0084_1c40, &s.map(i), |m| {
            g::scissor_2(m, s[0], s[1], s[2], s[3])
        });
        let (fbp, msk) = (rng.next(), rng.next());
        twin("Sg_GsFRAME", a, b, 0x0084_1870, &[fbp, msk], |m| {
            g::frame_1(m, fbp, msk)
        });
        twin("Sg_GsFRAME2", a, b, 0x0084_18f0, &[fbp, msk], |m| {
            g::frame_2(m, fbp, msk)
        });
        twin("Sg_GsFRAME_16", a, b, 0x0084_19d0, &[fbp, msk], |m| {
            g::frame_16_1(m, fbp, msk)
        });
        twin("Sg_GsFRAME2_16", a, b, 0x0084_1970, &[fbp, msk], |m| {
            g::frame_16_2(m, fbp, msk)
        });
        let (x, y) = (rng.int(), rng.int());
        twin("Sg_GsXYOFFSET", a, b, 0x0084_2000, &[i(x), i(y)], |m| {
            g::xyoffset_1(m, x, y)
        });
        twin("Sg_GsXYOFFSET2", a, b, 0x0084_2070, &[i(x), i(y)], |m| {
            g::xyoffset_2(m, x, y)
        });
        twin("Sg_GsXYZF2", a, b, 0x0084_2140, &[i(x), i(y)], |m| {
            g::xyzf2(m, x, y)
        });
        let zf = rng.next();
        twin("Sg_GsXYZF2V", a, b, 0x0084_2210, &[i(x), i(y), zf], |m| {
            g::xyzf2v(m, x, y, zf)
        });
        let (u, v) = (arg(rng), arg(rng));
        twin("Sg_GsUV", a, b, 0x0084_1f60, &[u, v], |m| g::uv_1(m, u, v));
        twin("Sg_GsUV2", a, b, 0x0084_1fb0, &[u, v], |m| g::uv_2(m, u, v));
        let on = rng.below(3);
        twin("Sg_GsTexHokan", a, b, 0x0084_1f10, &[on], |m| {
            g::tex_hokan(m, on)
        });
        twin("Sg_GsEnd2", a, b, 0x0084_1800, &[], g::end2);
        twin("Sg_GsStartVIF", a, b, 0x0084_1d50, &[], g::start_vif);
        let load = block(a, b, rng, 0x40);
        twin("Sg_GsloadSetBase", a, b, 0x0083_d370, &[load], |m| {
            g::gsload_set_base(m, load);
        });
        // The debug lines, with debug on or off.
        let db = alloc(a, b, 0x40);
        set(a, b, g::DB_PTR, db);
        set(a, b, g::IS_DEBUG, rng.below(2).to_le_bytes()[0]);
        twin("Sg_DbLine", a, b, 0x0084_1470, &[], g::db_line);
        twin("Sg_DbLineStrip", a, b, 0x0084_1490, &[], |m| {
            g::db_line_strip(m);
        });
        let v = floats(a, b, rng, 3);
        twin("Sg_DbVertex", a, b, 0x0084_1610, &[v], |m| {
            g::db_vertex(m, v);
        });
    }
}

fn all(a: &mut Oracle, b: &mut Oracle, seed: u32, from_game: bool) {
    let mut rng = Xorshift(seed);
    accessors(a, b, &mut rng, 200, !from_game);
    points(a, b, &mut rng, 300, false);
    vectors(a, b, &mut rng, 500);
    packet(a, b, &mut rng, 200);
    round2(a, b, &mut rng, 150);
    round3(a, b, &mut rng, 150);
}

#[test]
fn synthetic_cases() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    all(&mut a, &mut b, 0x5eed_0001, false);
}

#[test]
fn snapshot_cases() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0002);
    // The game's own lights first, then the rest.
    points(&mut a, &mut b, &mut rng, 20, true);
    all(&mut a, &mut b, 0x5eed_0003, true);
}

// Round 2: camera and screen, light set-up, angle wraps, fades, capture,
// resources, maps, the debug lines and sequences.

/// A float of moderate size (no infinity: the angle wraps never end on
/// one) or a quiet NaN.
fn angle(rng: &mut Xorshift) -> u32 {
    match rng.below(12) {
        0 => 0x7fc0_0000,
        1 => 0,
        2 => 0x8000_0000,
        _ => {
            let e = rng.below(18) + 115; // 2^-12 .. 2^5 times 1..2
            rng.next() & 0x807f_ffff | e << 23
        }
    }
}

/// `n` floats at `at` in both.
fn put_floats(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, at: u32, n: u32) {
    for k in 0..n {
        let f = rng.float();
        put(a, b, at + 4 * k, f);
    }
}

fn camera_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::camera::bridge as c;
    let i = |v: i32| v.cast_unsigned();
    // The battle camera in use: a block of floats.
    let cam = floats(a, b, rng, 0xda0 / 4);
    set(a, b, ffx_battle::camera::bridge::CAMERA, cam);
    for _ in 0..rounds {
        put_floats(a, b, rng, cam + 0x50, 3);
        put_floats(a, b, rng, cam + 0x61c + 0x50, 3);
        put_floats(a, b, rng, cam + 0xc9c, 1);
        let center = a.rebase(c::SCREEN_CENTER.sym);
        put_floats(a, b, rng, center, 2);
        twin("Sg_CalcScreenMatrix", a, b, 0x0083_f160, &[], |m| {
            c::calc_screen_matrix(m);
        });
        let mtx = block(a, b, rng, 64);
        let wide = rng.below(3);
        twin(
            "Sg_CalcViewClipMatrix",
            a,
            b,
            0x0083_f1c0,
            &[mtx, wide],
            |m| c::calc_view_clip_matrix(m, mtx, wide),
        );
        let v = if rng.below(4) == 0 {
            0
        } else {
            alloc(a, b, 16)
        };
        twin("Sg_CameraGetPos", a, b, 0x0083_f260, &[v], |m| {
            c::camera_get_pos(m, v)
        });
        let at = a.rebase(c::CAMERA_REF.sym);
        put_floats(a, b, rng, at, 3);
        twin("Sg_CameraGetRef", a, b, 0x0083_f2a0, &[v], |m| {
            c::camera_get_ref(m, v)
        });
        twin("Sg_GetLookVector", a, b, 0x0083_f710, &[v], |m| {
            c::get_look_vector(m, v)
        });
        let (x, y) = (rng.int(), rng.int());
        twin(
            "Sg_SetScreenCenter",
            a,
            b,
            0x0083_f7a0,
            &[i(x), i(y)],
            |m| c::set_screen_center(m, x, y),
        );
        // Battle camera positions and channels (`g_Cameras`).
        let (n, side) = (rng.below(3), rng.next());
        let pos = a.rebase(c::CAMERA_POS_NOW.sym) + n * 0xda0 + (side & 1) * 0x61c;
        put_floats(a, b, rng, pos, 3);
        let out: [u32; 3] =
            std::array::from_fn(|_| if rng.below(4) == 0 { 0 } else { alloc(a, b, 4) });
        let cam_arg = n | rng.next() << 8;
        twin(
            "Cam_GetPos",
            a,
            b,
            0x007b_d420,
            &[cam_arg, side, out[0], out[1], out[2]],
            |m| c::cam_get_pos(m, cam_arg, side, out),
        );
        let id = n | (rng.below(4) << 8);
        let channels = a.rebase(c::CHANNELS.sym) + n * 0xda0;
        for k in 0..4 {
            let w = if rng.below(2) == 0 { id } else { rng.next() };
            put(a, b, channels + 8 * k, w);
        }
        let on = arg(rng);
        twin(
            "Cam_SetChannelSuspended",
            a,
            b,
            0x007c_0010,
            &[id, on],
            |m| {
                c::cam_set_channel_suspended(m, id, on);
            },
        );
    }
}

fn light_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use light::bridge as l;
    for _ in 0..rounds {
        for g in l::LIGHT_DIRS {
            put_floats(a, b, rng, a.rebase(g.sym), 4);
        }
        put_floats(a, b, rng, a.rebase(l::LIGHT_COLORS.sym), 16);
        let out = alloc(a, b, 16);
        let i = rng.below(6);
        twin("Sg_LightGetColor", a, b, 0x0083_60f0, &[i, out], |m| {
            l::get_color_of(m, i, out);
        });
        twin("Sg_LightGetDir", a, b, 0x0083_61c0, &[i, out], |m| {
            l::get_dir(m, i, out);
        });
        let v = if rng.below(4) == 0 { 0 } else { out };
        twin("Sg_GetLightPos", a, b, 0x0083_6070, &[rng.next(), v], |m| {
            l::get_light_pos(m, v)
        });
        twin("Sg_InitLight", a, b, 0x0083_60b0, &[], l::init_light);
        let chr = floats(a, b, rng, 0x100);
        let flags = rng.next().to_le_bytes()[0];
        put(a, b, chr + 0x188, flags);
        twin(
            &format!("Sg_LightSetChr flags {flags:#x}"),
            a,
            b,
            0x0083_6250,
            &[chr],
            |m| l::set_chr(m, chr),
        );
        // Sg_AnimValF: equal, NaN, zero and other frame counts.
        let v = floats(a, b, rng, 3);
        match rng.below(4) {
            0 => {
                let x: u32 = Mem::read(&*a, v);
                put(a, b, v + 4, x);
            }
            1 => put(a, b, v + 8, 0u32),
            _ => put(a, b, v + 8, rng.int()),
        }
        twin("Sg_AnimValF", a, b, 0x0083_6020, &[v], |m| {
            l::anim_val_f(m, v)
        });
        // Sg_PLightDefine on a fresh table.
        let base = floats(a, b, rng, 16 * 11);
        set(a, b, l::PLIGHTS, base);
        let last = rng.below(16).cast_signed() - 1;
        set(a, b, l::PLIGHT_LAST, last);
        let n = rng.below(16).cast_signed();
        let xyz = [rng.float(), rng.float(), rng.float()];
        let radius = rng.float();
        let rgb = [rng.int(), rng.int(), rng.int()];
        let light = arg(rng);
        let args = [
            n.cast_unsigned(),
            xyz[0],
            xyz[1],
            xyz[2],
            radius,
            rgb[0].cast_unsigned(),
            rgb[1].cast_unsigned(),
            rgb[2].cast_unsigned(),
            light,
        ];
        twin(
            &format!("Sg_PLightDefine({n}) last {last}"),
            a,
            b,
            0x0083_19b0,
            &args,
            |m| {
                l::define(
                    m,
                    n,
                    xyz.map(f32::from_bits),
                    f32::from_bits(radius),
                    rgb,
                    light,
                );
            },
        );
    }
}

const ANGLE_WRAPS: [u32; 6] = [
    0x0082_db50,
    0x0082_9800,
    0x0083_6720,
    0x0083_a010,
    0x0084_5640,
    0x0084_eb60,
];

fn vector_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    for _ in 0..rounds {
        let va = floats(a, b, rng, 4);
        let vb = floats(a, b, rng, 4);
        let out = match rng.below(3) {
            0 => va,
            1 => vb,
            _ => alloc(a, b, 16),
        };
        twin("Sg_VecCross", a, b, 0x0082_95a0, &[out, va, vb], |m| {
            vec::bridge::cross(m, out, va, vb);
        });
        let mtx = floats(a, b, rng, 16);
        twin(
            "Sg_MatrixApplyVec3",
            a,
            b,
            0x0082_4e60,
            &[out, mtx, va],
            |m| {
                vec::bridge::matrix_apply_vec3(m, out, mtx, va);
            },
        );
        put_floats(a, b, rng, a.rebase(vec::bridge::WIND.sym), 3);
        let o: [u32; 3] =
            std::array::from_fn(|_| if rng.below(4) == 0 { 0 } else { alloc(a, b, 4) });
        twin("Sg_GlobalGetWind", a, b, 0x0082_4dd0, &o, |m| {
            vec::bridge::global_get_wind(m, o[0], o[1], o[2]);
        });
        let x = angle(rng);
        for sym in ANGLE_WRAPS {
            twin(
                &format!("Angle_Wrap {sym:#x}({x:#x})"),
                a,
                b,
                sym,
                &[x],
                |_| vec::bridge::angle_wrap(f32::from_bits(x)),
            );
        }
    }
}

fn state_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use state::bridge as s;
    let i = |v: i32| v.cast_unsigned();
    // `Phyre_PTimerWin32_GetTime` becomes `fld qword [now]; ret`.
    let now = alloc(a, b, 8);
    for o in [&mut *a, &mut *b] {
        let mut code = vec![0xdd, 0x05];
        code.extend_from_slice(&now.to_le_bytes());
        code.push(0xc3);
        let at = o.rebase(0x0042_fd80);
        o.write(at, &code).unwrap();
    }
    for _ in 0..rounds {
        let (t, x, y) = (arg(rng), arg(rng), arg(rng));
        twin("Sg_Fade_Common", a, b, 0x0082_cdc0, &[t, x, y], |m| {
            s::fade_common(m, t, x, y)
        });
        let c: [u32; 4] = std::array::from_fn(|_| arg(rng));
        twin("Sg_Flash", a, b, 0x0082_cca0, &c, |m| {
            s::flash(m, c[0], c[1], c[2], c[3]);
        });
        let c: [u32; 4] = std::array::from_fn(|_| arg(rng));
        twin("Sg_FilterAdd", a, b, 0x0082_cc20, &c, |m| {
            s::filter_add(m, c[0], c[1], c[2], c[3])
        });
        let (n, v) = (rng.below(4), arg(rng));
        twin("Sg_SetFocus", a, b, 0x0082_ce20, &[n, v], |m| {
            s::set_focus(m, n, v)
        });
        twin("Sg_AccSetClear", a, b, 0x0082_bdc0, &c, |m| {
            s::acc_set_clear(m, c[0], c[1], c[2], c[3]);
        });
        let chr = if rng.below(4) == 0 {
            0
        } else {
            floats(a, b, rng, 8)
        };
        twin("Sg_SetPlayer", a, b, 0x0082_dad0, &[chr], |m| {
            s::set_player(m, chr)
        });
        for g in [0x0130_07b1u32, 0x0130_07b2, 0x0130_07b3, 0x00c4_96d4] {
            let v = rng.next().to_le_bytes()[0] & 0x83;
            put(a, b, a.rebase(g), v);
        }
        twin("Sg_GetCurExecFrames", a, b, 0x0082_d7e0, &[], |m| {
            s::get_cur_exec_frames(m)
        });
        let out = alloc(a, b, 12);
        twin("Sg_GetPlayerPos", a, b, 0x0082_d880, &[out], |m| {
            s::get_player_pos(m, out)
        });
        for g in s::DISPLAY_ENV {
            let v = rng.next();
            put(a, b, a.rebase(g.sym), v);
        }
        put(
            a,
            b,
            a.rebase(s::CAPTURE_GUI.sym),
            rng.next().to_le_bytes()[0],
        );
        twin("Sg_CaptureEnable", a, b, 0x0083_a470, &[], |m| {
            s::capture_enable(m)
        });
        let (mode, on) = (arg(rng), rng.below(3));
        twin(
            "Sg_CaptureEnable_Kari",
            a,
            b,
            0x0083_a4f0,
            &[mode, on],
            |m| s::capture_enable_kari(m, mode, on),
        );
        twin("Sg_CaptureEnable_KariOn", a, b, 0x0083_a580, &[mode], |m| {
            s::capture_enable_kari_on(m, mode)
        });
        twin("Sg_CaptureSetGui", a, b, 0x0083_a660, &[mode], |m| {
            s::capture_set_gui(m, mode);
        });
        let o: [u32; 3] =
            std::array::from_fn(|_| if rng.below(4) == 0 { 0 } else { alloc(a, b, 4) });
        twin("Sg_GetTkAntiXYB", a, b, 0x0083_adf0, &o, |m| {
            s::get_tk_anti_xyb(m, o);
        });
        // Resources, through the game's own table.
        let (n, v) = (rng.below(40), rng.next());
        twin(
            &format!("Sg_GetResource({n})"),
            a,
            b,
            0x0083_fbe0,
            &[n, v],
            |m| s::get_resource(m, n, v),
        );
        twin(
            &format!("Sg_SetResource({n})"),
            a,
            b,
            0x0083_fe10,
            &[n, v],
            |m| s::set_resource(m, n, v),
        );
        let len = usize::try_from(rng.below(24)).unwrap();
        let mut text: Vec<u8> = (0..len)
            .map(|_| b'a' + rng.below(26).to_le_bytes()[0])
            .collect();
        text.push(0);
        let str_at = a.alloc_bytes(&text).unwrap();
        assert_eq!(b.alloc_bytes(&text).unwrap(), str_at);
        twin(
            &format!("Sg_SetResourceSTR({n}, {len})"),
            a,
            b,
            0x0083_fe60,
            &[n, str_at],
            |m| {
                s::set_resource_str(m, n, str_at);
            },
        );
        // Maps.
        set(a, b, s::MAP_DIR_SELECT, rng.below(3));
        let which = rng.below(2);
        twin("Sg_MapSetName", a, b, 0x0084_4fb0, &[str_at, which], |m| {
            s::map_set_name(m, str_at, which);
        });
        set(a, b, s::IS_DEBUG, rng.below(2).to_le_bytes()[0]);
        twin("Sg_MapInitAll", a, b, 0x0084_4eb0, &[], |m| {
            s::map_init_all(m);
        });
        let (x, y) = (rng.int(), rng.int());
        twin("Sg_SetMouse", a, b, 0x0085_1ae0, &[i(x), i(y)], |m| {
            s::set_mouse(m, x, y)
        });
        twin("sg_AddY", a, b, 0x0085_0820, &[i(x)], |m| s::add_y(m, i(x)));
        twin("sa_task_start", a, b, 0x0077_a2c0, &[], |m| {
            s::sa_task_start(m);
        });
        // The kind tables, over every small value and the letters.
        let k = if rng.below(2) == 0 {
            rng.below(0x80)
        } else {
            arg(rng)
        };
        twin(
            &format!("sg_getdbname({k:#x})"),
            a,
            b,
            0x0082_9c90,
            &[k],
            |m| s::get_db_name(m, k),
        );
        twin(
            &format!("sg_getdbnum({k:#x})"),
            a,
            b,
            0x0082_9d80,
            &[k],
            |_| s::get_db_num(k.cast_signed()),
        );
        twin(
            &format!("sg_getdbtype({k:#x})"),
            a,
            b,
            0x0082_9e10,
            &[k],
            |_| s::get_db_type(k),
        );
        // FFXApplication::getAppTime at a chosen clock.
        let t = f64::from(rng.below(100_000)) / 64.0;
        put(a, b, now, t);
        put(
            a,
            b,
            a.rebase(s::APP_START.sym),
            f64::from(rng.below(1000)) / 8.0,
        );
        let app = floats(a, b, rng, 16);
        let at = a.rebase(0x0042_f690);
        let r = a.call(at, Conv::Thiscall(app), &[]).unwrap();
        if r.st0.is_some() {
            let pop = a.rebase(POP_STUB);
            a.call(pop, Conv::Cdecl, &[]).unwrap();
        }
        s::get_app_time(b, app, t).check(&r, "FFXApplication_getAppTime");
        assert_eq!(a.diff(b).unwrap(), [], "FFXApplication_getAppTime: memory");
    }
}

fn line_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use gs::bridge as g;
    put_floats(a, b, rng, a.rebase(g::DB_COS.sym), 13);
    put_floats(a, b, rng, a.rebase(g::DB_SIN.sym), 13);
    for _ in 0..rounds {
        let p = block(a, b, rng, 64 * 16);
        let given = if rng.below(3) == 0 { 0 } else { p };
        set(a, b, state::bridge::PACKET, p + 16);
        twin("Sg_GsSetPacket", a, b, 0x0084_1cf0, &[given], |m| {
            g::set_packet(m, given)
        });
        let z = rng.next();
        twin("Sg_GsZBUF", a, b, 0x0084_22f0, &[z], |m| g::zbuf_1(m, z));
        twin("Sg_GsZBUF2", a, b, 0x0084_2350, &[z], |m| g::zbuf_2(m, z));
        put(a, b, a.rebase(g::DEBUG_TEX.sym), rng.next());
        put(a, b, a.rebase(g::DEBUG_TEX_WIDTH.sym), rng.next());
        let q = block(a, b, rng, 16);
        twin("Sg_SetDebugTEX_0", a, b, 0x0085_1160, &[q], |m| {
            g::set_debug_tex0(m, q)
        });
        let db = alloc(a, b, 0x200);
        set(a, b, g::DB_PTR, db);
        set(a, b, g::IS_DEBUG, rng.below(2).to_le_bytes()[0]);
        let (x, y) = (rng.next(), rng.next());
        twin("Sg_Db2DVertex", a, b, 0x0084_0fc0, &[x, y], |m| {
            g::db_2d_vertex(m, x, y);
        });
        twin("Sg_DbColor", a, b, 0x0084_1230, &[x, y, z], |m| {
            g::db_color(m, x, y, z);
        });
        let c = floats(a, b, rng, 3);
        let r = rng.float();
        twin("Sg_DbCircle", a, b, 0x0084_10a0, &[c, r], |m| {
            g::db_circle_xz(m, c, f32::from_bits(r));
        });
        twin("Sg_DbCircleXY", a, b, 0x0084_1170, &[c, r], |m| {
            g::db_circle_xy(m, c, f32::from_bits(r));
        });
        twin("Sg_DbCircleYZ", a, b, 0x0084_11d0, &[c, r], |m| {
            g::db_circle_yz(m, c, f32::from_bits(r));
        });
        set(a, b, g::DB_PTR, db);
        twin("Sg_DbNull", a, b, 0x0084_14b0, &[c, r], |m| {
            g::db_null(m, c, f32::from_bits(r));
        });
    }
}

/// A channel byte stream: deltas, waits and two-byte deltas.
fn stream(rng: &mut Xorshift, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| match rng.below(4) {
            0 => 0x80 | rng.below(4).to_le_bytes()[0],
            _ => rng.next().to_le_bytes()[0],
        })
        .collect()
}

fn sequence_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::seq::bridge as q;
    for _ in 0..rounds {
        // A sequence with two tables of 16-byte entries.
        let seq = block(a, b, rng, 0x20);
        let counts = [
            rng.below(6).cast_signed() - 1,
            rng.below(6).cast_signed() - 1,
        ];
        let delta = rng.next();
        for (k, n) in (0u32..).zip(counts) {
            let n16 = i16::try_from(n).unwrap();
            put(a, b, seq + 8 + 2 * k, n16);
            let t = block(a, b, rng, 0x60);
            put(a, b, seq + 0xc + 4 * k, t.wrapping_sub(delta));
        }
        let given = if rng.below(8) == 0 { 0 } else { seq };
        twin(
            "SG_RelocateSequence",
            a,
            b,
            0x0083_6fc0,
            &[given, delta],
            |m| q::relocate_sequence(m, given, delta),
        );
        // An object with a list of channels.
        let obj = alloc(a, b, 0x750);
        let mut next = 0u32;
        for _ in 0..rng.below(4) {
            let ch = block(a, b, rng, 0x20);
            let bytes = stream(rng, 128);
            let s = a.alloc_bytes(&bytes).unwrap();
            assert_eq!(b.alloc_bytes(&bytes).unwrap(), s);
            put(a, b, ch, next);
            put(a, b, ch + 0xc, s);
            put(a, b, ch + 6, u16::from(rng.below(3).to_le_bytes()[0]));
            next = ch;
        }
        put(a, b, obj + 0x74c, next);
        let frames = rng.below(20).cast_signed() - 2;
        twin(
            &format!("SG_StepChannels({frames})"),
            a,
            b,
            0x0083_94d0,
            &[obj, frames.cast_unsigned()],
            |m| q::step_channels(m, obj, frames),
        );
    }
}

fn round2(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    camera_cases(a, b, rng, rounds);
    light_cases(a, b, rng, rounds);
    vector_cases(a, b, rng, rounds);
    state_cases(a, b, rng, rounds);
    line_cases(a, b, rng, rounds);
    sequence_cases(a, b, rng, rounds);
}

// Round 3: fades and focus, the frame rate, the mouse, map ids, the
// memory manager's blocks, quaternions and directions (with the C
// runtime's trigonometry replaced alike on both sides), sequences.

/// `_CIcos`, `_CIsin` and `_CIacos` (imports the oracle lacks) replaced at
/// their thunks by exact stand-ins: `x²` (`fmul st, st`), `2x` (`fadd st,
/// st`) and `x + 1` (`fld1; faddp`); the bridges get the same through their
/// callback.
fn patch_trig(o: &mut Oracle) {
    for (thunk, code) in [
        (0x0094_98c2, &[0xd8, 0xc8, 0xc3][..]),
        (0x0094_98c8, &[0xd8, 0xc0, 0xc3][..]),
        (0x0094_98da, &[0xd9, 0xe8, 0xde, 0xc1, 0xc3][..]),
    ] {
        let at = o.rebase(thunk);
        o.write(at, code).unwrap();
    }
}

fn stand_in(c: ffx_sg::quat::Crt, x: f64) -> f32 {
    use ffx_bridge::x87::Reg;
    let x = Reg::constant(x);
    match c {
        ffx_sg::quat::Crt::Cos => (x * x).store(),
        ffx_sg::quat::Crt::Sin => (x + x).store(),
        ffx_sg::quat::Crt::Acos => (x + Reg::ONE).store(),
        ffx_sg::quat::Crt::Atan => (-x).store(),
    }
}

fn frame_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use state::bridge as s;
    for _ in 0..rounds {
        let v = arg(rng);
        twin("Sg_MemSetAlign", a, b, 0x0083_c4a0, &[v], |m| {
            s::mem_set_align(m, v)
        });
        twin("Sg_MemCache", a, b, 0x0083_b700, &[v], |_| s::mem_cache(v));
        let fades: [Set<u32>; 4] = [
            ("Sg_FadeIn", 0x0082_cb80, |m, t| s::fade_in(m, t)),
            ("Sg_FadeInW", 0x0082_cba0, |m, t| s::fade_in_w(m, t)),
            ("Sg_FadeOut", 0x0082_cbc0, |m, t| s::fade_out(m, t)),
            ("Sg_FadeOutW", 0x0082_cbe0, |m, t| s::fade_out_w(m, t)),
        ];
        for (name, sym, f) in fades {
            let t = arg(rng);
            twin(name, a, b, sym, &[t], |m| f(m, t));
        }
        let c: [u32; 4] = std::array::from_fn(|_| arg(rng));
        twin("Sg_FilterSub", a, b, 0x0082_cc80, &c, |m| {
            s::filter_sub(m, c[0], c[1], c[2], c[3])
        });
        twin("Sg_FocusReset", a, b, 0x0082_ccf0, &[], |m| {
            s::focus_reset(m)
        });
        put(a, b, a.rebase(s::FOCUS.sym), [arg(rng), arg(rng)]);
        twin("Sg_ToggleFocus", a, b, 0x0082_cd80, &[], |m| {
            s::toggle_focus(m);
        });
        // The frame rate.
        set(a, b, s::VCOUNT, rng.next());
        set(a, b, s::VCOUNT_LAST, rng.next());
        set(a, b, s::RATE_MODE, rng.below(3));
        let base = rng.below(8);
        set(a, b, s::RATE_HIST_BASE, base);
        set(a, b, s::RATE_HIST_HEAD, base + rng.below(8));
        for k in 0..8 {
            put(a, b, a.rebase(s::RATE_HIST.sym) + 4 * k, rng.next());
        }
        for g in [0x0130_07b1u32, 0x0130_07b2, 0x0130_07b3, 0x00c4_96d4] {
            put(a, b, a.rebase(g), rng.below(3).to_le_bytes()[0]);
        }
        twin("Sg_MainCalcRate", a, b, 0x0082_06c0, &[], |m| {
            s::main_calc_rate(m)
        });
        put(
            a,
            b,
            a.rebase(s::VIEWER_MODE.sym),
            rng.below(2).to_le_bytes()[0],
        );
        twin("Sg_GetUserName2", a, b, 0x0082_0620, &[], |m| {
            s::get_user_name2(m)
        });
        put(
            a,
            b,
            a.rebase(s::ID_EXIST_SHADE.sym),
            rng.below(2).to_le_bytes()[0],
        );
        let c = rng.next();
        twin("Sg_IdGetShade", a, b, 0x0083_e980, &[c], |m| {
            s::id_get_shade(m, c)
        });
        let what = if rng.below(2) == 0 { 0x1113 } else { arg(rng) };
        twin("Sg_Debug", a, b, 0x0083_e5c0, &[what, v], |m| {
            s::debug(m, what, v);
        });
        twin("getUCStatus", a, b, 0x0082_2460, &[], |m| {
            s::get_uc_status(m)
        });
        let id = if rng.below(3) == 0 { 0 } else { alloc(a, b, 4) };
        if id != 0 {
            put(
                a,
                b,
                id,
                if rng.below(2) == 0 {
                    0x1154u16
                } else {
                    rng.next().to_le_bytes()[0].into()
                },
            );
        }
        twin("set0434isDarkYojimbo", a, b, 0x0082_9c60, &[id], |m| {
            s::set_dark_yojimbo(m, id);
        });
        let deg = angle(rng);
        twin("Sg_PlayerSetDirFix", a, b, 0x0082_d990, &[v, deg], |m| {
            s::player_set_dir_fix(m, v, f32::from_bits(deg));
        });
        // The mouse buttons.
        set(a, b, s::MOUSE_RAW, rng.below(8));
        set(a, b, s::MOUSE_BUTTONS, rng.next() & 0x7_0707);
        let pos = [rng.below(3), rng.below(3)];
        put(a, b, a.rebase(s::MOUSE.sym), pos);
        set(
            a,
            b,
            s::MOUSE_CLICK_POS,
            if rng.below(2) == 0 {
                pos
            } else {
                [rng.below(3), 7]
            },
        );
        set(a, b, s::MOUSE_CLICK_TIMER, rng.below(3));
        twin("Sg_MouseButtonMain", a, b, 0x0085_17c0, &[], |m| {
            s::mouse_button_main(m)
        });
        // Map ids: an id with its header, the map objects.
        let id = if rng.below(4) == 0 {
            0
        } else {
            let id = floats(a, b, rng, 8);
            let hdr = block(a, b, rng, 16);
            put(a, b, id + 0x1c, hdr);
            put(a, b, id + 0x18, rng.next());
            id
        };
        let n = rng.below(4);
        let objs = block(a, b, rng, 0x880 * 4);
        set(a, b, s::MAP_OBJECTS, objs);
        set(a, b, s::MAP_OBJECT_COUNT, n);
        twin("Sg_IdSetPtr", a, b, 0x0083_ea00, &[id], |m| {
            s::id_set_ptr(m, id);
        });
        let req = [0x4010_0000, 0x4020_0000, 0x4200_0000, rng.next()][rng.below(4) as usize];
        put(
            a,
            b,
            a.rebase(ffx_system::save::bridge::SCREEN_STATE.sym),
            rng.below(2),
        );
        set(a, b, s::MENU_ACTIVE, rng.below(2));
        twin(
            &format!("Sg_MenuSetReserve({req:#x})"),
            a,
            b,
            0x0082_1750,
            &[req],
            |m| {
                s::menu_set_reserve(m, req, |m| {
                    ffx_menu::tk::bridge::tk_menu_save_load_active(m);
                });
            },
        );
        twin("Sg_ToggleEnableTkAnti", a, b, 0x0083_b100, &[], |m| {
            s::toggle_enable_tk_anti(m);
        });
        let i = rng.below(16);
        twin("Sg_IdSetMapTable", a, b, 0x0083_e9e0, &[i, v], |m| {
            s::id_set_map_table(m, i, v)
        });
        twin("Sg_IdGetMapTable", a, b, 0x0083_e960, &[i], |m| {
            s::id_get_map_table(m, i)
        });
        twin("Sg_IdEventInitAll", a, b, 0x0083_e8d0, &[], |m| {
            s::id_event_init_all(m);
        });
        set(a, b, s::EID_PRINT_HEAD, rng.below(0x100));
        set(a, b, s::EID_PRINT_TAIL, rng.below(0x100));
        twin("eidprintfPrint", a, b, 0x0076_b8d0, &[], |m| {
            s::eid_printf_print(m);
        });
        for sym in [0x0077_a480u32, 0x0077_a490, 0x0077_a4a0, 0x0084_e870] {
            twin(&format!("stub {sym:#x}"), a, b, sym, &[], |_| ());
        }
        for sym in [0x00a4_3ca0u32, 0x00a4_3cb0] {
            twin(&format!("stub {sym:#x}"), a, b, sym, &[], |_| {
                s::not_implemented()
            });
        }
    }
}

/// A chain of memory blocks (16-byte headers, `+0` next, `+4` previous,
/// owner nibble at `+0xc`), the first in `MEM_FIRST`; their data
/// addresses.
fn blocks(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> Vec<u32> {
    let n = 2 + rng.below(5);
    let sizes: Vec<u32> = (0..n).map(|_| 16 * (1 + rng.below(6))).collect();
    let total: u32 = sizes.iter().map(|s| s + 16).sum::<u32>() + 16;
    let base = block(a, b, rng, total as usize);
    let mut at = base;
    let mut heads = Vec::new();
    for s in &sizes {
        heads.push(at);
        at += 16 + s;
    }
    heads.push(at);
    for (k, &h) in heads.iter().enumerate() {
        let next = heads.get(k + 1).copied().unwrap_or(h + 16);
        // The manager walks `+4` from the first block (`Sg_MemCache_SerchFree`).
        let prev = next;
        put(a, b, h, next);
        put(a, b, h + 4, prev);
        let owner = if rng.below(2) == 0 {
            0u32
        } else {
            1 + rng.below(15)
        };
        let w: u32 = Mem::read(&*a, h + 0xc);
        put(a, b, h + 0xc, w & !0xf | owner);
        put(a, b, h + 10, u16::from(rng.below(4).to_le_bytes()[0]) * 3);
    }
    set(a, b, state::bridge::MEM_FIRST, heads[0]);
    heads.iter().map(|h| h + 16).collect()
}

fn memory_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use state::bridge as s;
    // The debug names live at 0x05fb0000; map that page in both.
    set(a, b, s::IS_DEBUG, 0);
    for _ in 0..rounds {
        let data = blocks(a, b, rng);
        let p = data[rng.below(u32::try_from(data.len()).unwrap()) as usize];
        let maybe = if rng.below(6) == 0 { 0 } else { p };
        twin("Sg_MemIsFree", a, b, 0x0083_c320, &[maybe], |m| {
            s::mem_is_free(m, maybe)
        });
        twin("Sg_MemGetSum", a, b, 0x0083_c0b0, &[p - 16], |m| {
            s::mem_get_sum(m, p - 16)
        });
        twin("Sg_MemGetStr", a, b, 0x0083_c070, &[maybe], |m| {
            s::mem_get_str(m, maybe)
        });
        twin("Sg_MemClearStr", a, b, 0x0083_bab0, &[maybe], |m| {
            s::mem_clear_str(m, maybe);
        });
        let r = rng.next();
        twin("Sg_MemSetRtAddr", a, b, 0x0083_c560, &[r], |m| {
            s::mem_set_rt_addr(m, r)
        });
        twin("Sg_MemCache_SerchFree", a, b, 0x0083_b770, &[p], |m| {
            s::mem_cache_serch_free(m, p)
        });
    }
}

/// Free image bytes after `.text` for the classifier stand-ins.
const CLASSIFY_CODE: u32 = 0x00b0_bb00;

/// Real `_isnan` (of a double) and `_finite` at their import slots, for
/// `cdc_Vector_SafeNormalize3` under `sg_normalize`.
fn patch_classify(o: &mut Oracle) {
    #[rustfmt::skip]
    let isnan: [u8; 34] = [
        0x8b, 0x44, 0x24, 0x08,             // mov eax, [esp+8]
        0x25, 0xff, 0xff, 0xff, 0x7f,       // and eax, 0x7fffffff
        0x3d, 0x00, 0x00, 0xf0, 0x7f,       // cmp eax, 0x7ff00000
        0x77, 0x0c,                         // ja nan
        0x72, 0x07,                         // jb num
        0x83, 0x7c, 0x24, 0x04, 0x00,       // cmp dword [esp+4], 0
        0x75, 0x03,                         // jne nan
        0x31, 0xc0, 0xc3,                   // num: xor eax, eax; ret
        0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3, // nan: mov eax, 1; ret
    ];
    #[rustfmt::skip]
    let finite: [u8; 21] = [
        0x8b, 0x44, 0x24, 0x08,             // mov eax, [esp+8]
        0x25, 0x00, 0x00, 0xf0, 0x7f,       // and eax, 0x7ff00000
        0x3d, 0x00, 0x00, 0xf0, 0x7f,       // cmp eax, 0x7ff00000
        0x0f, 0x95, 0xc0,                   // setne al
        0x0f, 0xb6, 0xc0,                   // movzx eax, al
        0xc3,                               // ret
    ];
    let code = o.rebase(CLASSIFY_CODE);
    o.write(code, &isnan).unwrap();
    o.write(code + 0x40, &finite).unwrap();
    Mem::write(o, o.rebase(IAT_ISNAN), code);
    Mem::write(o, o.rebase(IAT_FINITE), code + 0x40);
}

fn quaternion_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::quat::bridge as q;
    patch_trig(a);
    patch_trig(b);
    patch_classify(a);
    patch_classify(b);
    for n in 0..rounds {
        let qa = floats(a, b, rng, 4);
        let m = if n % 5 == 0 {
            qa
        } else {
            floats(a, b, rng, 16)
        };
        twin("Sg_QuaternionToMatrix", a, b, 0x0084_5450, &[qa, m], |o| {
            q::quaternion_to_matrix(o, qa, m);
        });
        let mtx = floats(a, b, rng, 16);
        if n % 2 == 0 {
            // A positive trace now and then.
            for k in [0u32, 5, 10] {
                put(
                    a,
                    b,
                    mtx + 4 * k,
                    f32::from(u8::try_from(rng.below(8)).unwrap()) / 4.0,
                );
            }
        }
        let out = if n % 7 == 0 { mtx } else { alloc(a, b, 16) };
        twin(
            "Sg_MatrixToQuaternion",
            a,
            b,
            0x0084_5170,
            &[mtx, out],
            |o| q::matrix_to_quaternion(o, mtx, out),
        );
        let qb = floats(a, b, rng, 4);
        if n % 3 == 0 {
            // Nearly the same quaternion: the linear branch.
            let x: [u32; 4] = Mem::read(&*a, qa);
            put(a, b, qb, x);
        }
        let t = rng.float();
        let out = if n % 4 == 0 { qa } else { alloc(a, b, 16) };
        twin(
            "Sg_QuaternionSlerp",
            a,
            b,
            0x0084_5310,
            &[qa, qb, t, out],
            |o| q::quaternion_slerp(o, qa, qb, f32::from_bits(t), out, &mut stand_in),
        );
        let (h, v) = (angle(rng), angle(rng));
        let out = alloc(a, b, 16);
        twin(
            "Sg_CreateVectorFromHV",
            a,
            b,
            0x0084_50d0,
            &[out, h, v],
            |o| {
                q::create_vector_from_hv(
                    o,
                    out,
                    f32::from_bits(h),
                    f32::from_bits(v),
                    &mut stand_in,
                );
            },
        );
        let v = floats(a, b, rng, 3);
        let m = block(a, b, rng, 64);
        twin("Sg_VectorToMatrix2", a, b, 0x0084_5560, &[m, v], |o| {
            q::vector_to_matrix2(o, m, v)
        });
        let count = rng.below(5);
        let vs = floats(a, b, rng, 16);
        let c = count | rng.next() << 16;
        twin("sg_normalize", a, b, 0x0084_6710, &[vs, c], |o| {
            q::normalize(o, vs, c);
        });
    }
}

fn misc3_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::camera::bridge as c;
    use ffx_sg::seq::bridge as q;
    use gs::bridge as g;
    for _ in 0..rounds {
        // Cam_IsMoving over random camera bytes.
        let cam = rng.below(3);
        let base = a.rebase(ffx_battle::camera::bridge::CAMERAS.sym) + cam * 0xda0;
        for _ in 0..24 {
            let off = rng.below(0xda0 / 2) * 2;
            let v = if rng.below(2) == 0 {
                0u16
            } else {
                rng.below(16).to_le_bytes()[0].into()
            };
            put(a, b, base + off, v);
        }
        let side = rng.next();
        twin(
            "Cam_IsMoving",
            a,
            b,
            0x007b_dd20,
            &[cam | rng.next() << 8, side],
            |m| c::cam_is_moving(m, cam, side),
        );
        // Debug colour and the draw frame.
        let db = alloc(a, b, 0x40);
        set(a, b, g::DB_PTR, db);
        set(a, b, g::IS_DEBUG, rng.below(2).to_le_bytes()[0]);
        let col = rng.next();
        twin("Sg_DbColor8", a, b, 0x0084_1270, &[col], |m| {
            g::db_color8(m, col);
        });
        let p = block(a, b, rng, 32 * 16);
        set(a, b, g::PACKET, p);
        set(a, b, g::COUNT, rng.below(8));
        set(a, b, g::DRAW_MODE, rng.below(4).to_le_bytes()[0]);
        set(a, b, g::DRAW_FBP16, rng.int());
        set(a, b, g::DRAW_FBP, rng.next());
        twin("Sg_DrawSetFrame", a, b, 0x0082_cf80, &[], |m| {
            g::draw_set_frame(m);
        });
        // A motion group of sequences that moved.
        let n = rng.below(4);
        let group = alloc(a, b, 0x10);
        let table = alloc(a, b, 20 * 4);
        let delta = rng.below(0x100) * 4;
        put(
            a,
            b,
            group,
            group.wrapping_sub(if rng.below(5) == 0 { 0 } else { delta }),
        );
        put(a, b, group + 4, n);
        put(a, b, group + 0xc, table.wrapping_sub(delta));
        for k in 0..n {
            let e = table + 20 * k;
            let cnt = rng.below(3);
            put(a, b, e + 8, [i16::try_from(cnt).unwrap(), 0]);
            let t = block(a, b, rng, 0x40);
            put(a, b, e + 0xc, t.wrapping_sub(delta));
            put(a, b, e + 0x10, 0u32);
        }
        let given = if rng.below(8) == 0 { 0 } else { group };
        twin("SG_RelocateGroup", a, b, 0x0083_6f70, &[given], |m| {
            q::relocate_group(m, given);
        });
        // Channels to rewind.
        let obj = alloc(a, b, 0x7e0);
        let mut next = 0u32;
        for _ in 0..rng.below(4) {
            let ch = block(a, b, rng, 0x20);
            let bytes = stream(rng, 64);
            let s = a.alloc_bytes(&bytes).unwrap();
            assert_eq!(b.alloc_bytes(&bytes).unwrap(), s);
            put(a, b, ch, next);
            put(a, b, ch + 8, s);
            next = ch;
        }
        put(a, b, obj + 0x74c, next);
        twin("SG_RewindChannels", a, b, 0x0083_98f0, &[obj], |m| {
            q::rewind_channels(m, obj);
        });
        // Motion groups and sequences tables.
        let groups = a.rebase(q::MOTION_GROUPS.sym);
        for k in 0..50u32 {
            let g = if rng.below(3) == 0 {
                0
            } else {
                rng.below(4) + 1
            };
            put(a, b, groups + 12 * k, g);
            put(
                a,
                b,
                groups + 12 * k + 4,
                [i16::from(rng.below(2) == 1), i16::from(rng.below(2) == 1)],
            );
            put(a, b, groups + 12 * k + 8, rng.below(2));
        }
        let (x, y, key) = (rng.below(2), rng.below(2), rng.below(2));
        twin("SG_FindMotionGroup", a, b, 0x0083_6f10, &[x, y, key], |m| {
            q::find_motion_group(m, x.cast_signed(), y.cast_signed(), key)
        });
        let gid = rng.below(6);
        twin("SG_RemoveMotionGroup", a, b, 0x0083_6da0, &[gid], |m| {
            q::remove_motion_group(m, gid);
        });
        let seqs = a.rebase(q::SEQUENCES.sym);
        for k in 0..100u32 {
            let v = if rng.below(4) == 0 {
                0
            } else {
                rng.below(8) + 1
            };
            put(a, b, seqs + 20 * k, v);
        }
        let sq = rng.below(10);
        twin("SG_RemoveSequence", a, b, 0x0083_6de0, &[sq], |m| {
            q::remove_sequence(m, sq);
        });
        // Angles and distances.
        let (cur, target, step) = (angle(rng), angle(rng), angle(rng));
        twin(
            "Sg_AngleApproach",
            a,
            b,
            0x0083_5f80,
            &[cur, target, step],
            |_| {
                vec::bridge::angle_approach(
                    f32::from_bits(cur),
                    f32::from_bits(target),
                    f32::from_bits(step),
                )
            },
        );
        let raw = angle(rng);
        twin("SG_RawToRad", a, b, 0x0083_8c60, &[raw], |m| {
            vec::bridge::raw_to_rad(m, f32::from_bits(raw))
        });
        let (va, vb) = (floats(a, b, rng, 3), floats(a, b, rng, 3));
        twin("Sg_VectorGetLength2", a, b, 0x0083_66a0, &[va, vb], |m| {
            vec::bridge::vector_get_length2(m, va, vb)
        });
        twin("Sg_VectorGetLengthXZ", a, b, 0x0083_e3e0, &[va, vb], |m| {
            vec::bridge::vector_get_length_xz(m, va, vb)
        });
    }
}

fn round3(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    frame_cases(a, b, rng, rounds);
    memory_cases(a, b, rng, rounds);
    quaternion_cases(a, b, rng, rounds);
    misc3_cases(a, b, rng, rounds);
    packet_setter_cases(a, b, rng, rounds);
}

/// The `Ch_PacketSet*` setters over random arguments.
fn packet_setter_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::chpacket::bridge as p;
    let f = f32::from_bits;
    for _ in 0..rounds {
        let v = rng.next();
        twin("Ch_PacketSetName", a, b, 0x0084_3000, &[v], |m| {
            p::set_name(m, v)
        });
        let ones: [Set<()>; 7] = [
            ("Ch_PacketSetCull", 0x0084_3010, |m, v| p::set_cull(m, v)),
            ("Ch_PacketSetFSpecular", 0x0084_3020, |m, v| {
                p::set_fspecular(m, v);
            }),
            ("Ch_PacketSetShadow", 0x0084_3040, |m, v| {
                p::set_shadow(m, v);
            }),
            ("Ch_PacketSetShadowSemi", 0x0084_3070, |m, v| {
                p::set_shadow_semi(m, v);
            }),
            ("Ch_PacketSetShadowTexture", 0x0084_3080, |m, v| {
                p::set_shadow_texture(m, v);
            }),
            ("Ch_PacketSetSpecular", 0x0084_30e0, |m, v| {
                p::set_specular(m, v);
            }),
            ("Ch_PacketSetTexture", 0x0084_30f0, |m, v| {
                p::set_texture(m, v);
            }),
        ];
        for (name, sym, set) in ones {
            let v = rng.next();
            twin(name, a, b, sym, &[v], |m| set(m, v));
        }
        let [r, g, bl, al] = [rng.next(), rng.next(), rng.next(), rng.next()];
        twin(
            "Ch_PacketSetShadowColor",
            a,
            b,
            0x0084_3050,
            &[r, g, bl],
            |m| {
                p::set_shadow_color(m, r, g, bl);
            },
        );
        twin(
            "Ch_PacketSetSpecColor",
            a,
            b,
            0x0084_30a0,
            &[r, g, bl, al],
            |m| {
                p::set_spec_color(m, r, g, bl, al);
            },
        );
        let (x, y) = (rng.float(), rng.float());
        twin(
            "Ch_PacketSetSpecCoefficient",
            a,
            b,
            0x0084_3090,
            &[x],
            |m| {
                p::set_spec_coefficient(m, f(x));
            },
        );
        twin("Ch_PacketSetSpecSize", a, b, 0x0084_30d0, &[y], |m| {
            p::set_spec_size(m, f(y));
        });
    }
}

// Main round 1 (control.rs): the requests the main loop passes on. Their
// callees (renderer, sound, DMA) are recording stubs, so these cases run
// on oracles of their own.

/// The bridges' host: the oracle's memory and, for the callees, its
/// emulator.
struct H<'a>(&'a mut Oracle);

impl Mem for H<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

impl ffx_bridge::Host for H<'_> {
    fn call(&mut self, addr: u32, args: &[u32]) -> u32 {
        self.0
            .call(addr, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }

    fn callee_scratch(&mut self) -> Option<u32> {
        // the bottom of the emulated stack: no call reaches that deep, and
        // the diff leaves the stacks out, as it does the originals' frames
        Some(self.0.stack_range().start + 0x100)
    }
}

/// Free image bytes for the recorders: the `.reloc` section, which the
/// code never reads.
const REC_CODE: u32 = 0x0270_0000;
/// The log: a count, then 32-byte entries (the callee, up to six
/// arguments).
const REC_LOG: u32 = 0x0270_f000;
/// One answer per recorder.
const REC_RETS: u32 = 0x0270_fc00;
/// `Yn_PreBgLayerDispGet`'s answers by layer id (16 bytes).
const LAYER_DISP: u32 = 0x0270_fe00;

/// A cdecl recording stub over the original at `sym` (slot `k` of the
/// code area): logs `sym` and `n` arguments, returns `ret`.
fn recorder(a: &mut Oracle, b: &mut Oracle, k: u32, sym: u32, n: u32, ret: u32) {
    assert!(n <= 6);
    for o in [a, b] {
        let log = o.rebase(REC_LOG);
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], sym
        for i in 0..n {
            let d = u8::try_from(4 + 4 * i).unwrap();
            c.extend([0x8b, 0x54, 0x24, d]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, d]); // mov [eax+4+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        // the answer from data, not code: rewritten code would not be seen
        // (the emulator keeps what it translated)
        let slot = o.rebase(REC_RETS) + 4 * k;
        o.write(slot, &ret.to_le_bytes()).unwrap();
        c.push(0xa1);
        c.extend(slot.to_le_bytes()); // mov eax, [slot]
        c.push(0xc3);
        assert!(c.len() <= 0x60);
        let code = o.rebase(REC_CODE) + 0x60 * k;
        o.write(code, &c).unwrap();
        let at = o.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        o.write(at, &j).unwrap();
    }
}

fn control_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32, from_game: bool) {
    use ffx_sg::control::bridge as c;
    let callees = [
        (c::GRAPHIC_SET_MOTION_BLUR_ENABLE, 1),
        (c::GRAPHIC_SET_MOTION_BLUR_WEIGHT, 1),
        (c::GRAPHIC_UPDATE_CROSS_FADE, 2),
        (c::GRAPHIC_SET_MAP_ALPHA_FAST, 1),
        (c::VOICE_SET_PAUSED, 1),
        (c::SFX_PAUSE, 1),
        (c::SFX_STOP_NON_LOOP, 0),
        (c::SOUND_SET_MUTE, 2),
        (c::USER_FREE, 1),
        (c::MEM_MOVE_SET_TRIGGER, 0),
        (0x0063_9150, 0),
        (0x0064_2390, 1),
        (c::VU0_NORMAL_LIGHT_MATRIX, 4),
        (c::LIGHTING_SET_LIGHT_DIR, 4),
        (c::FLUSH_CACHE, 1),
        (c::DMA_SYNC, 3),
        (c::DMA_SEND, 2),
    ];
    for (k, &(sym, n)) in (0u32..).zip(callees.iter()) {
        recorder(a, b, k, sym, n, 0x1000 + k);
    }
    let pause_slot = u32::try_from(callees.len()).unwrap();
    let log = a.rebase(REC_LOG);
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        set(a, b, c::ACC_FLAGS, rng.next());
        let w = match rng.below(4) {
            0 => 0,
            1 => rng.next(),
            _ => rng.below(0x200),
        };
        twin("Sg_Blur", a, b, 0x0082_bee0, &[w], |m| {
            c::blur(&mut H(m), w)
        });

        put(a, b, log, 0u32);
        set(a, b, c::ACC_FLAGS, rng.next());
        let now = if rng.below(3) == 0 {
            0
        } else {
            u16::try_from(rng.next() & 0xffff).unwrap()
        };
        set(a, b, c::ACC_ALPHA, now);
        let map = if rng.below(3) == 0 {
            0x102
        } else {
            rng.below(0x200)
        };
        set(a, b, c::MAP_NO, map);
        let alpha = if rng.below(4) == 0 {
            rng.next()
        } else {
            rng.below(0x100)
        };
        let frames = match rng.below(5) {
            0 => 0,
            1 => rng.next() | 1,
            2 => 0x10000 * (rng.below(4) + 1),
            _ => rng.below(60) + 1,
        };
        twin("Sg_AccSetAlpha", a, b, 0x0082_bd10, &[alpha, frames], |m| {
            c::acc_set_alpha(&mut H(m), alpha, frames)
        });

        // the menu's pause answer, fixed per round
        let held = rng.below(3) == 0;
        recorder(a, b, pause_slot, c::IS_IN_TK_MENU_PAUSE, 0, u32::from(held));
        let on = if rng.below(4) == 0 {
            rng.next()
        } else {
            rng.below(2)
        };
        put(a, b, log, 0u32);
        set(a, b, c::SOUND_STATE, rng.below(4));
        twin("Sg_SetPauseAllSounds", a, b, 0x0082_1cc0, &[on], |m| {
            c::set_pause_all_sounds(&mut H(m), on);
        });
        put(a, b, log, 0u32);
        set(a, b, c::SOUND_STATE, rng.below(4));
        twin("Sg_SetMuteSound", a, b, 0x0082_1c60, &[on], |m| {
            c::set_mute_sound(&mut H(m), on);
        });
        put(a, b, log, 0u32);
        twin("Sg_SetMapAlphaFast", a, b, 0x0082_1b00, &[on], |m| {
            c::set_map_alpha_fast(&mut H(m), on)
        });

        put(a, b, log, 0u32);
        set(a, b, c::PLIGHT_TABLE, rng.next());
        set(a, b, c::PLIGHT_COUNT, rng.next());
        twin("Sg_PLightDispose", a, b, 0x0083_1ba0, &[], |m| {
            c::plight_dispose(&mut H(m))
        });
        put(a, b, log, 0u32);
        twin("Sg_AccRequestCurrent", a, b, 0x0082_bdf0, &[], |m| {
            c::acc_request_current(&mut H(m))
        });
        put(a, b, log, 0u32);
        twin("dreamParamReset", a, b, 0x0082_2410, &[], |m| {
            c::dream_param_reset(&mut H(m))
        });

        put(a, b, log, 0u32);
        let dir = floats(a, b, rng, 4);
        let i = rng.below(4);
        twin("Sg_LightSetDir", a, b, 0x0083_6430, &[i, dir], |m| {
            c::light_set_dir(&mut H(m), i, dir)
        });

        // the GS packet in a fresh block (the game's own, from a snapshot,
        // is left alone), the tag inside it
        put(a, b, log, 0u32);
        let n = rng.below(16) + 1;
        let packet = block(a, b, rng, 16 * (n as usize + 2));
        set(a, b, gs::bridge::PACKET, packet);
        set(a, b, gs::bridge::COUNT, n);
        set(a, b, gs::bridge::START, rng.below(n + 1));
        set(a, b, gs::bridge::TAG, packet + 16 * rng.below(n));
        if !from_game || rng.below(2) == 0 {
            set(a, b, c::GS_DMA_CHANNEL, rng.next());
        }
        twin("Sg_GsEnd", a, b, 0x0084_1770, &[], |m| c::gs_end(&mut H(m)));
    }
}

#[test]
fn control_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0401);
    control_cases(&mut a, &mut b, &mut rng, 300, false);
    scene_cases(&mut a, &mut b, &mut rng, 300);
    init_cases(&mut a, &mut b, &mut rng, 200);
    round3_main_cases(&mut a, &mut b, &mut rng, 300);
    round4_main_cases(&mut a, &mut b, &mut rng, 300, false);
    sync_cases(&mut a, &mut b, &mut rng, 300);
    round5_main_cases(&mut a, &mut b, &mut rng, 300);
    round6_main_cases(&mut a, &mut b, &mut rng, 300);
    round7_main_cases(&mut a, &mut b, &mut rng, 300);
    round8_main_cases(&mut a, &mut b, &mut rng, 300, false);
    round9_main_cases(&mut a, &mut b, &mut rng, 300);
    init_bones_cases(&mut a, &mut b, &mut rng, 300);
    envelope_cases(&mut a, &mut b, &mut rng, 300);
    packed_envelope_cases(&mut a, &mut b, &mut rng, 300);
    source_envelope_cases(&mut a, &mut b, &mut rng, 300);
    source_packed_envelope_cases(&mut a, &mut b, &mut rng, 300);
    push_apart_cases(&mut a, &mut b, &mut rng, 600);
    cloth_anchor_cases(&mut a, &mut b, &mut rng, 300);
    cloth_step_cases(&mut a, &mut b, &mut rng, 300);
    round11_main_cases(&mut a, &mut b, &mut rng, 300);
    draw_filter_cases(&mut a, &mut b, &mut rng, 600);
    vu_helper_cases(&mut a, &mut b, &mut rng, 200);
}

#[test]
fn control_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0402);
    control_cases(&mut a, &mut b, &mut rng, 300, true);
    scene_cases(&mut a, &mut b, &mut rng, 300);
    init_cases(&mut a, &mut b, &mut rng, 200);
    round3_main_cases(&mut a, &mut b, &mut rng, 300);
    round4_main_cases(&mut a, &mut b, &mut rng, 300, true);
    sync_cases(&mut a, &mut b, &mut rng, 300);
    round5_main_cases(&mut a, &mut b, &mut rng, 300);
    round6_main_cases(&mut a, &mut b, &mut rng, 300);
    round7_main_cases(&mut a, &mut b, &mut rng, 300);
    round8_main_cases(&mut a, &mut b, &mut rng, 300, true);
    round9_main_cases(&mut a, &mut b, &mut rng, 300);
    init_bones_cases(&mut a, &mut b, &mut rng, 300);
    envelope_cases(&mut a, &mut b, &mut rng, 300);
    packed_envelope_cases(&mut a, &mut b, &mut rng, 300);
    source_envelope_cases(&mut a, &mut b, &mut rng, 300);
    source_packed_envelope_cases(&mut a, &mut b, &mut rng, 300);
    push_apart_cases(&mut a, &mut b, &mut rng, 600);
    cloth_anchor_cases(&mut a, &mut b, &mut rng, 300);
    cloth_step_cases(&mut a, &mut b, &mut rng, 300);
    round11_main_cases(&mut a, &mut b, &mut rng, 300);
    draw_filter_cases(&mut a, &mut b, &mut rng, 600);
    vu_helper_cases(&mut a, &mut b, &mut rng, 200);
}

// Main round 2: map id data, motion groups, the player's settings.

fn scene_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::seq::bridge as sq;
    use ffx_sg::state::bridge as st;
    recorder(a, b, 18, st::LIGHTING_SET_MAP_ID_FLAG_CHANGED, 2, 0x2000);
    recorder(a, b, 19, sq::CACHE_UNUSE, 2, 0x2001);
    let log = a.rebase(REC_LOG);
    let f = f32::from_bits;
    for _ in 0..rounds {
        // Sg_IdFixupAndSet: offsets from a base (0: from the map, the
        // map itself: already pointers, or any)
        put(a, b, log, 0u32);
        let n = rng.below(5);
        let table = block(a, b, rng, 16 * n as usize + 16);
        let map = block(a, b, rng, 0x40);
        let base = match rng.below(3) {
            0 => 0,
            1 => map,
            _ => rng.next(),
        };
        let delta = map.wrapping_sub(base);
        put(a, b, map, base);
        let version = match rng.below(3) {
            0 => 0x1202,
            1 => 0x1203,
            _ => rng.next() & 0xffff,
        };
        put(a, b, map + 4, u16::try_from(version).unwrap());
        let count = if rng.below(8) == 0 { 0xfffe } else { n };
        put(a, b, map + 6, u16::try_from(count).unwrap());
        put(a, b, map + 0x1c, table.wrapping_sub(delta));
        let shade = match rng.below(3) {
            0 => 0,
            1 => 1,
            _ => u8::try_from(rng.next() & 0xff).unwrap(),
        };
        set(a, b, st::CHR_ID_SHADE_ON, shade);
        let arg = if rng.below(8) == 0 { 0 } else { map };
        twin("Sg_IdFixupAndSet", a, b, 0x0084_4bc0, &[arg], |m| {
            st::id_fixup_and_set(&mut H(m), arg);
        });

        // SG_DisposeMotionGroup over one loaded group and a few
        // characters, some on it
        put(a, b, log, 0u32);
        let group = block(a, b, rng, 0x20);
        let nseq = rng.below(4);
        let seqs = block(a, b, rng, 20 * nseq as usize + 4);
        put(a, b, group + 0xc, seqs);
        put(
            a,
            b,
            group + 4,
            if rng.below(8) == 0 { u32::MAX } else { nseq },
        );
        for k in 0..nseq {
            if rng.below(2) == 0 {
                let slot = a.rebase(sq::SEQUENCES.sym) + 20 * rng.below(100);
                put(a, b, slot, seqs + 20 * k);
            }
        }
        let kind = [0i32, 1, 2, 5][rng.below(4) as usize];
        let num = rng.int();
        let key = rng.next();
        let slot = a.rebase(sq::MOTION_GROUPS.sym) + 12 * rng.below(50);
        put(a, b, slot, group);
        put(
            a,
            b,
            slot + 4,
            u16::try_from(kind.cast_unsigned() & 0xffff).unwrap(),
        );
        put(
            a,
            b,
            slot + 6,
            u16::try_from(num.cast_unsigned() & 0xffff).unwrap(),
        );
        put(a, b, slot + 8, key);
        let nchr = rng.below(4);
        let chrs = alloc(a, b, 0x880 * nchr as usize + 4);
        for c in 0..nchr {
            let p = chrs + 0x880 * c;
            put(a, b, p + 2, u8::from(rng.below(3) != 0));
            put(
                a,
                b,
                p + 0x7e8,
                if rng.below(3) == 0 { rng.next() } else { group },
            );
            put(a, b, p + 0x714, rng.next());
        }
        set(a, b, sq::CHR_ARRAY, chrs);
        set(a, b, sq::CHR_COUNT, nchr);
        let (ka, kb, kk) = if rng.below(6) == 0 {
            (kind, num, key ^ 1)
        } else {
            (kind, num, key)
        };
        let (ua, ub) = (ka.cast_unsigned(), kb.cast_unsigned());
        twin(
            "SG_DisposeMotionGroup",
            a,
            b,
            0x0083_6e20,
            &[ua, ub, kk],
            |m| sq::dispose_motion_group(&mut H(m), ka, kb, kk),
        );

        // the player's settings
        let v = if rng.below(4) == 0 {
            rng.next()
        } else {
            rng.below(2)
        };
        twin("Sg_PlayerSetDiving", a, b, 0x0082_d9d0, &[v], |m| {
            st::player_set_diving(m, v)
        });
        let kind = match rng.below(4) {
            0 => rng.next(),
            1 => rng.below(0x16),
            _ => [1, 2, 0x12, 0x13][rng.below(4) as usize],
        };
        let x = rng.float();
        twin("Sg_PlayerSetSpeed", a, b, 0x0082_d9e0, &[kind, x], |m| {
            st::player_set_speed(m, kind, f(x))
        });
        set(
            a,
            b,
            st::CAMERA_FOLLOW,
            u8::try_from(rng.next() & 0xff).unwrap(),
        );
        twin("Sg_SetCameraFollow", a, b, 0x0082_dab0, &[v], |m| {
            st::set_camera_follow(m, v)
        });
    }
}

/// The main loop's initialisers: filters, ids, the player, the user
/// name, the point lights.
fn init_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::control::bridge as c;
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    for _ in 0..rounds {
        let filter = a.rebase(st::FILTER_BLOCK);
        for k in 0..0x1b {
            put(a, b, filter - 4 + 4 * k, rng.next());
        }
        twin("Sg_InitFilter", a, b, 0x0082_cd50, &[], |m| {
            st::init_filter(m)
        });
        twin("Sg_IdInitAll", a, b, 0x0084_4ca0, &[], |m| {
            st::id_init_all(m);
        });
        twin("Sg_PlayerInitAll", a, b, 0x0082_d910, &[], |m| {
            st::player_init_all(m)
        });

        put(a, b, log, 0u32);
        let copy = rng.next();
        recorder(a, b, 20, st::SG_STRDUP_D, 1, copy);
        let name = rng.next();
        twin("Sg_SetUserName", a, b, 0x0082_1d30, &[name], |m| {
            st::set_user_name(&mut H(m), name)
        });

        put(a, b, log, 0u32);
        let n = match rng.below(6) {
            0 => 0,
            1 => 0xffff_fff0 + rng.below(16),
            _ => rng.below(5) + 1,
        };
        let size = if n.cast_signed() > 0 { 0x2c * n } else { 4 };
        let table = block(a, b, rng, size as usize);
        recorder(a, b, 21, c::USER_MALLOC, 1, table);
        set(a, b, c::PLIGHT_TABLE, rng.next());
        twin("Sg_PLightAllocate", a, b, 0x0083_1950, &[n], |m| {
            c::plight_allocate(&mut H(m), n)
        });
    }
}

// Main round 3: start-up and event-jump initialisers, setFramSync, light
// directions from floats, the vector helpers.

fn round3_main_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::control::bridge as c;
    use ffx_sg::state::bridge as st;
    use ffx_sg::vec::bridge as v;
    let log = a.rebase(REC_LOG);
    recorder(a, b, 22, c::GET_SCENERIO_FLAG, 0, 0x3000);
    let f = f32::from_bits;
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        twin("Sg_AccInitAll", a, b, 0x0082_bcd0, &[], |m| {
            c::acc_init_all(&mut H(m));
        });
        twin("Sg_PlayerInitDefault", a, b, 0x0082_d8d0, &[], |m| {
            st::player_init_default(m)
        });

        put(a, b, log, 0u32);
        let (x, y, z) = (rng.float(), rng.float(), rng.float());
        let i = rng.below(4);
        twin("Sg_LightSetDirXYZ", a, b, 0x0083_64f0, &[i, x, y, z], |m| {
            c::light_set_dir_xyz(&mut H(m), i, f(x), f(y), f(z))
        });

        put(a, b, log, 0u32);
        recorder(a, b, 20, st::SG_STRDUP_D, 1, rng.next());
        set(a, b, c::IS_DEBUG, u8::try_from(rng.below(3)).unwrap());
        let count = [0, 1, 2, 5, u32::MAX][rng.below(5) as usize];
        let args = block(a, b, rng, 12);
        twin("Sg_InitArgs", a, b, 0x0082_0310, &[count, args], |m| {
            c::init_args(&mut H(m), count, args);
        });

        put(a, b, log, 0u32);
        let battle = if rng.below(3) == 0 { 0 } else { rng.next() };
        let id = if rng.below(3) == 0 {
            u32::MAX
        } else {
            rng.below(0x100)
        };
        recorder(a, b, 23, c::MS_BATTLE_CHECK, 0, battle);
        recorder(a, b, 24, c::MAGIC_FILE_GET_MAGIC_ID, 0, id);
        set(a, b, c::FRAM_SYNC, rng.next());
        twin("setFramSync", a, b, 0x0082_2720, &[], |m| {
            c::set_fram_sync(&mut H(m))
        });

        // vectors
        let src = block(a, b, rng, 16);
        let out = block(a, b, rng, 16);
        twin("Sg_IntVecToFloat", a, b, 0x0084_5d90, &[out, src], |m| {
            v::int_vec_to_float(m, out, src)
        });
        let small = rng.below(2) == 0;
        let verts: Vec<u32> = (0..3)
            .map(|_| {
                let p = alloc(a, b, 8);
                for k in 0..3 {
                    let c16 = if small {
                        rng.below(4)
                    } else {
                        rng.next() & 0xffff
                    };
                    put(a, b, p + 2 * k, u16::try_from(c16).unwrap());
                }
                p
            })
            .collect();
        let n = block(a, b, rng, 16);
        twin(
            "Sg_CalcTriangleNormal",
            a,
            b,
            0x0083_e420,
            &[verts[0], verts[1], verts[2], n],
            |m| v::calc_triangle_normal(m, verts[0], verts[1], verts[2], n),
        );
        let dir = floats(a, b, rng, 4);
        if rng.below(3) == 0 {
            for k in 0..3 {
                put(
                    a,
                    b,
                    dir + 4 * k,
                    f32::from(i8::try_from(rng.below(3)).unwrap() - 1),
                );
            }
        }
        let mtx = block(a, b, rng, 256);
        let rows = match rng.below(3) {
            0 => 0x12,
            1 => 0x012 + (rng.below(2) << 8),
            _ => rng.next(),
        };
        twin(
            "Sg_MatrixFromDir",
            a,
            b,
            0x0084_35a0,
            &[mtx, dir, rows],
            |m| v::matrix_from_dir(m, mtx, dir, rows),
        );
    }
}

// Main round 4: bones, sequences, id kinds, the dream effect, cloth rods.

/// A moderate float (-8..8, sometimes any float).
fn modest(rng: &mut Xorshift) -> u32 {
    if rng.below(8) == 0 {
        return rng.float();
    }
    let v = f64::from(rng.below(0x10000)) / 4096.0 - 8.0;
    ffx_bridge::x87::store(v)
}

fn round4_main_cases(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rounds: u32,
    from_game: bool,
) {
    use ffx_sg::control::bridge as c;
    use ffx_sg::seq::bridge as sq;
    use ffx_sg::state::bridge as st;
    use ffx_sg::vec::bridge as v;
    let log = a.rebase(REC_LOG);
    let f = f32::from_bits;
    recorder(a, b, 25, sq::YI_ASSERT, 3, 0x4000);
    // Yn_PreBgLayerDispGet: layer `id`'s byte in the display table
    for o in [&mut *a, &mut *b] {
        let code = o.rebase(REC_CODE) + 0x60 * 26;
        let table = o.rebase(LAYER_DISP);
        // mov eax, [esp+4]; movzx eax, byte [eax + table]; ret
        let mut s = vec![0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x80];
        s.extend(table.to_le_bytes());
        s.push(0xc3);
        o.write(code, &s).unwrap();
        let at = o.rebase(c::YN_PRE_BG_LAYER_DISP_GET);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        o.write(at, &j).unwrap();
    }
    for (k, (sym, n)) in [
        (c::GRAPHIC_MENU_LOAD_SPECIAL_TEXTURE, 3),
        (c::GRAPHIC_2D_HIDE_LAYER, 1),
        (c::GRAPHIC_TEXANIM_SET_MATERIAL_VISIBILITY, 2),
        (c::GRAPHIC_DRAW_DREAM, 0),
        (c::GRAPHIC_2D_SHOW_LAYER, 1),
    ]
    .into_iter()
    .enumerate()
    {
        recorder(
            a,
            b,
            27 + u32::try_from(k).unwrap(),
            sym,
            n,
            0x4100 + u32::try_from(k).unwrap(),
        );
    }
    for _ in 0..rounds {
        // bones
        let chan = alloc(a, b, 9 * 0x2c);
        for k in 0..9 {
            let ch = chan + 0x2c * k;
            // a rotation: finite (Angle_Wrap steps by 2 pi, an infinity
            // never ends)
            let x = modest(rng);
            let x = if k < 3 && (x >> 23 & 0xff) > 0x90 {
                0x3f80_0000
            } else {
                x
            };
            put(a, b, ch + 0x20, x);
            let flag = match rng.below(3) {
                0 => 0u8,
                1 => 1,
                _ => u8::try_from(rng.next() & 0xff).unwrap(),
            };
            put(a, b, ch + 0x28, flag);
        }
        let bone = alloc(a, b, 0x140);
        for k in 0..0x50 {
            put(a, b, bone + 4 * k, modest(rng));
        }
        let scale = modest(rng);
        let first = rng.next();
        let damped = rng.below(2) == 0;
        let (name, sym) = if damped {
            ("SG_ApplyBoneDamped", 0x0083_8470)
        } else {
            ("SG_ApplyBone", 0x0083_8280)
        };
        twin(name, a, b, sym, &[first, chan, bone, scale], |m| {
            sq::apply_bone(m, chan, bone, f(scale), damped);
        });

        // sequences: some of the table used, sometimes all of it
        put(a, b, log, 0u32);
        if !from_game {
            let base = a.rebase(sq::SEQUENCES.sym);
            let full = rng.below(6) == 0;
            for k in 0..100 {
                let used = full || rng.below(3) == 0;
                put(a, b, base + 20 * k, if used { rng.next() | 1 } else { 0 });
                put(a, b, base + 20 * k + 4, rng.next());
            }
        }
        let seq = block(a, b, rng, 8);
        let val = rng.next();
        twin("SG_AddSequence", a, b, 0x0083_6d30, &[seq, val], |m| {
            sq::add_sequence(&mut H(m), seq, val);
        });

        // id kinds: the map table only leads to lower kinds (no cycle)
        let table = alloc(a, b, 16 * 8);
        let kinds = [0u32, 1, 2, 13, 14, 3, 0x30, 0x35, 0x3f, 0x40, 0x7f];
        for k in 0..8 {
            let kind = kinds[rng.below(11) as usize] | (rng.next() & !0x7f);
            put(a, b, table + 16 * k + 0xc, kind);
        }
        set(a, b, st::ID_TABLE, table);
        let map = a.rebase(st::CHR_ID_MAP_TABLE);
        for k in 0x30..0x40 {
            let to = match rng.below(4) {
                0 => [1, 2, 13, 14][rng.below(4) as usize],
                1 if k > 0x30 => 0x30 + rng.below(k - 0x30),
                2 => rng.next() | 0x40,
                _ => rng.below(0x30),
            };
            put(a, b, map + 4 * k, to);
        }
        let chr = block(a, b, rng, 0x200);
        set(
            a,
            b,
            st::PLAYER,
            if rng.below(2) == 0 { chr } else { rng.next() },
        );
        let i = rng.below(8);
        twin("Sg_IdMatchChr", a, b, 0x0083_e4e0, &[chr, i], |m| {
            st::id_match_chr(m, chr, i)
        });

        // the dream effect over layer ids 0..15
        put(a, b, log, 0u32);
        let disp = [0u8, 1, 2, 3, 4, 8, 0x80, 0x81];
        let disp_at = a.rebase(LAYER_DISP);
        for k in 0..16 {
            put(a, b, disp_at + k, disp[rng.below(8) as usize]);
        }
        let la = rng.below(16);
        let lb = rng.below(16);
        set(a, b, c::DREAM_ON, rng.below(2));
        set(a, b, c::DREAM_PHASE, [0, 1, 5, 7][rng.below(4) as usize]);
        if rng.below(2) == 0 {
            twin("recallSceneEffect", a, b, 0x0082_2580, &[la], |m| {
                c::recall_scene_effect(&mut H(m), la)
            });
        } else {
            twin("recallSceneEffect2", a, b, 0x0082_2620, &[la, lb], |m| {
                c::recall_scene_effect2(&mut H(m), la, lb)
            });
        }

        // cloth rods
        let npts = rng.below(6) + 2;
        let points = alloc(a, b, 16 * npts as usize);
        for k in 0..4 * npts {
            put(a, b, points + 4 * k, modest(rng));
        }
        let n = rng.below(6);
        let rods = alloc(a, b, 12 * n as usize + 4);
        for k in 0..n {
            let e = rods + 12 * k;
            let pa = rng.below(npts);
            // one point twice: the distance 0, against a limit of 0 too
            let same = rng.below(5) == 0;
            let pb = if same { pa } else { rng.below(npts) };
            put(a, b, e, u16::try_from(pa).unwrap());
            put(a, b, e + 2, u16::try_from(pb).unwrap());
            put(a, b, e + 4, modest(rng));
            let lim = if same {
                0
            } else if rng.below(4) == 0 {
                modest(rng)
            } else {
                rng.below(64) * 0x0080_0000 / 8 + 0x3c00_0000
            };
            put(a, b, e + 8, lim);
        }
        let kind = rng.below(6) + 1;
        let sym = [
            0x0082_2890,
            0x0082_2960,
            0x0082_2a50,
            0x0082_2b20,
            0x0082_2c10,
            0x0082_2cf0,
        ][kind as usize - 1];
        twin("Sg_ClothRod", a, b, sym, &[rods, n, points], |m| {
            v::cloth_rods(m, kind, rods, n, points)
        });
    }
}

// Main round 5: the cut scene's sync (a callee writing through a pointer
// to the original's local: the host's callee scratch).

fn sync_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::control::bridge as c;
    // iSyncGetData(&time, &rate, &frame): writes the three from data
    // slots 33..35, returns slot 32
    for o in [&mut *a, &mut *b] {
        let rets = o.rebase(REC_RETS);
        let code = o.rebase(REC_CODE) + 0x60 * 32;
        let mut s = vec![];
        for (arg, slot) in [(4u8, 34u32), (8, 35), (0xc, 33)] {
            s.extend([0x8b, 0x4c, 0x24, arg]); // mov ecx, [esp+arg]
            s.push(0x8b);
            s.push(0x15);
            s.extend((rets + 4 * slot).to_le_bytes()); // mov edx, [slot]
            s.extend([0x89, 0x11]); // mov [ecx], edx
        }
        s.push(0xa1);
        s.extend((rets + 4 * 32).to_le_bytes()); // mov eax, [slot 32]
        s.push(0xc3);
        o.write(code, &s).unwrap();
        let at = o.rebase(c::ISYNC_GET_DATA);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        o.write(at, &j).unwrap();
    }
    let rets = a.rebase(REC_RETS);
    for _ in 0..rounds {
        let answer = if rng.below(3) == 0 { 999 } else { rng.below(4) };
        put(a, b, rets + 4 * 32, answer);
        put(a, b, rets + 4 * 33, rng.next());
        put(a, b, rets + 4 * 34, modest(rng));
        put(a, b, rets + 4 * 35, modest(rng));
        let base = a.rebase(0x012f_b814);
        for k in 0..0x28 {
            put(a, b, base + 4 * k, rng.next());
        }
        // the index within the table (the game's), sometimes past it
        let i = if rng.below(6) == 0 {
            rng.below(0x26)
        } else {
            rng.below(30)
        };
        set(a, b, c::SYNC_INDEX, i);
        let at = a.rebase(0x00c4_94d4);
        put(a, b, at, rng.below(2));
        twin("Sg_SyncCutScene", a, b, 0x0082_1d60, &[], |m| {
            c::sync_cut_scene(&mut H(m))
        });
    }
}

/// Id normals, the bone dispatcher, the dream effect by map.
fn round5_main_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::control::bridge as c;
    use ffx_sg::seq::bridge as sq;
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    let f = f32::from_bits;
    for _ in 0..rounds {
        // id triangles over 8 vertices
        let verts = block(a, b, rng, 8 * 8);
        let table = alloc(a, b, 16 * 4);
        for k in 0..12 {
            put(
                a,
                b,
                table + 16 * (k / 3) + 2 * (k % 3),
                u16::try_from(rng.below(8)).unwrap(),
            );
        }
        set(
            a,
            b,
            st::ID_TABLE,
            if rng.below(8) == 0 { 0 } else { table },
        );
        set(a, b, st::ID_VERTICES, verts);
        let i = if rng.below(8) == 0 {
            u32::MAX
        } else {
            rng.below(4)
        };
        let out = block(a, b, rng, 16);
        twin("Sg_IdGetNormal", a, b, 0x0083_e130, &[out, i], |m| {
            st::id_get_normal(m, out, i)
        });

        // the bone dispatcher
        let chan = alloc(a, b, 9 * 0x2c);
        for k in 0..9 {
            let x = modest(rng);
            let x = if k < 3 && (x >> 23 & 0xff) > 0x90 {
                0x3f80_0000
            } else {
                x
            };
            put(a, b, chan + 0x2c * k + 0x20, x);
            put(
                a,
                b,
                chan + 0x2c * k + 0x28,
                u8::try_from(rng.below(2)).unwrap(),
            );
        }
        let bone = alloc(a, b, 0x148);
        for k in 0..0x50 {
            put(a, b, bone + 4 * k, modest(rng));
        }
        put(a, b, bone + 0x144, u8::try_from(rng.below(2)).unwrap());
        let scale = modest(rng);
        let first = rng.next();
        twin(
            "SG_ApplyBoneSel",
            a,
            b,
            0x0083_8240,
            &[first, chan, bone, scale],
            |m| {
                sq::apply_bone_sel(m, chan, bone, f(scale));
            },
        );

        // the dream effect by map
        put(a, b, log, 0u32);
        let disp = [0u8, 1, 2, 3, 4, 8, 0x80, 0x81];
        let disp_at = a.rebase(LAYER_DISP);
        for k in 0..16 {
            put(a, b, disp_at + k, disp[rng.below(8) as usize]);
        }
        let maps = [
            0x86,
            0xa5,
            0xbf,
            0xf9,
            0xfc,
            0x129,
            0x187,
            0x85,
            0xfa,
            0x200,
            0x10,
            u32::MAX,
        ];
        set(a, b, c::GRAPHIC_MAP_ID, maps[rng.below(12) as usize]);
        set(a, b, c::DREAM_ON, rng.below(2));
        set(a, b, c::DREAM_PHASE, [0, 1, 5][rng.below(3) as usize]);
        twin("drawDreamEffect", a, b, 0x0082_23a0, &[], |m| {
            c::draw_dream_effect(&mut H(m))
        });
    }
}

// Main round 6: the effect strip, channel sampling, pose blending.

/// A channel list of 1-4 channels with random streams; returns the first.
fn channel_list(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> u32 {
    let n = rng.below(4) + 1;
    let mut next = 0;
    for _ in 0..n {
        let ch = block(a, b, rng, 0x30);
        let stream = block(a, b, rng, 0x100);
        put(a, b, ch, next);
        put(a, b, ch + 5, u8::try_from(rng.below(2)).unwrap());
        put(a, b, ch + 8, stream);
        put(a, b, ch + 0xc, stream + rng.below(8));
        put(a, b, ch + 6, u16::try_from(rng.below(4)).unwrap());
        put(a, b, ch + 0x1c, modest(rng));
        put(a, b, ch + 0x20, modest(rng));
        next = ch;
    }
    next
}

fn round6_main_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::control::bridge as c;
    use ffx_sg::seq::bridge as sq;
    let log = a.rebase(REC_LOG);
    recorder(a, b, 36, c::CH_EFFECT_PACKET_STORE_POLY, 3, 0x5000);
    for _ in 0..rounds {
        // the effect strip: vertices in a buffer with room behind them for
        // the back references
        put(a, b, log, 0u32);
        let buf = alloc(a, b, 2 * 96);
        for k in 0..96 {
            put(
                a,
                b,
                buf + 2 * k,
                u16::try_from(rng.next() & 0x7fff).unwrap(),
            );
        }
        let v = buf + 2 * (40 + 4 * rng.below(10));
        for k in 0..2 {
            if rng.below(3) == 0 {
                let back = 0x8000 | rng.below(24);
                put(a, b, v + 2 * k, u16::try_from(back).unwrap());
            }
        }
        set(a, b, c::STRIP_COUNT, u16::try_from(rng.below(7)).unwrap());
        let arg = if rng.below(8) == 0 { 0 } else { v };
        twin(
            "Ch_EffectPacketStoreStrip",
            a,
            b,
            0x0084_4370,
            &[arg],
            |m| c::effect_store_strip(&mut H(m), arg),
        );

        // channel sampling, within the streams' bytes (at most ~60 frames)
        let obj = block(a, b, rng, 0x800);
        let list = channel_list(a, b, rng);
        put(a, b, obj + 0x74c, list);
        put(a, b, obj + 0x740, rng.below(0x3c00));
        put(a, b, obj + 0x71c, rng.below(0x3c00));
        put(a, b, obj + 0x720, rng.below(0x3c00));
        put(a, b, obj + 0x724, u16::try_from(rng.below(3)).unwrap());
        put(a, b, obj + 0x7dc, rng.below(60));
        if rng.below(4) == 0 {
            let t: u32 = Mem::read(a, obj + 0x740);
            put(a, b, obj + 0x740, t & !0xff);
        }
        twin("SG_SampleChannels", a, b, 0x0083_93c0, &[obj], |m| {
            sq::sample_channels(m, obj);
        });

        // pose blending: a channel motion or a model's bones
        let obj = block(a, b, rng, 0x800);
        let list = channel_list(a, b, rng);
        put(a, b, obj + 0x74c, list);
        let nb = rng.below(4);
        let model = block(a, b, rng, 0x10);
        put(a, b, model + 0xa, u16::try_from(nb).unwrap());
        put(a, b, obj + 0x1c0, model);
        let bones = alloc(a, b, 0x18c * nb as usize + 0x20);
        for k in 0..(0x18c * nb + 0x20) / 4 {
            put(a, b, bones + 4 * k, modest(rng));
        }
        put(a, b, obj + 0x748, bones);
        // some values already at their targets (no step, even over 0 frames)
        for k in 0..nb {
            for o in [0x80u32, 0xac, 0xd8, 0x104, 0x130, 0x15c] {
                if rng.below(3) == 0 {
                    let at = bones + 0x20 + 0x18c * k + o;
                    let t: u32 = Mem::read(a, at);
                    put(a, b, at + 4, t);
                }
            }
        }
        put(a, b, obj + 0x196, u8::try_from(rng.below(2)).unwrap());
        let n = rng.below(10).cast_signed() - 2;
        twin(
            "SG_BlendToTarget",
            a,
            b,
            0x0083_95b0,
            &[obj, n.cast_unsigned()],
            |m| {
                sq::blend_to_target(m, obj, n);
            },
        );
    }
}

// Main round 7: the edge slide, the free camera, raw clips.

/// A float in -8..8 (no specials: the edge slide steps by one unit until it
/// is back, which never ends far out).
fn bounded(rng: &mut Xorshift) -> u32 {
    let v = f64::from(rng.below(0x10000)) / 4096.0 - 8.0;
    ffx_bridge::x87::store(v)
}

fn round7_main_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::seq::bridge as sq;
    use ffx_sg::state::bridge as st;
    for _ in 0..rounds {
        // the edge slide
        let va = alloc(a, b, 8);
        let vb = alloc(a, b, 8);
        for (v, k) in [(va, 0u32), (va, 4), (vb, 0), (vb, 4)] {
            let c = if rng.below(6) == 0 { 0 } else { rng.below(64) };
            put(a, b, v + k, u16::try_from(c).unwrap());
        }
        set(a, b, st::ID_EDGE_A, va);
        set(a, b, st::ID_EDGE_B, vb);
        let from = alloc(a, b, 16);
        let to = alloc(a, b, 16);
        for k in 0..4 {
            put(a, b, from + 4 * k, bounded(rng));
            put(a, b, to + 4 * k, bounded(rng));
        }
        twin("Sg_IdSlideEdge", a, b, 0x0083_e180, &[from, to], |m| {
            st::id_slide_edge(m, from, to);
        });

        // the free camera
        let stick = |rng: &mut Xorshift| u8::try_from(rng.next() & 0xff).unwrap();
        let (sx, sy) = (stick(rng), stick(rng));
        set(a, b, st::PAD_RSTICK_X, sx);
        set(a, b, st::PAD_RSTICK_Y, sy);
        set(a, b, st::PAD_HELD, rng.next() & 0x10f);
        set(a, b, st::PAD_PRESSED, rng.next() & 0x400);
        for g in [st::FREE_CAM_YAW, st::FREE_CAM_PITCH] {
            let x = if rng.below(3) == 0 {
                modest(rng)
            } else {
                bounded(rng) & 0xbfff_ffff
            };
            set(a, b, g, f32::from_bits(x));
        }
        set(a, b, st::FREE_CAM_HEIGHT, f32::from_bits(modest(rng)));
        set(a, b, st::FREE_CAM_DIST, f32::from_bits(modest(rng)));
        set(a, b, st::FREE_CAM_PRESET, rng.next());
        set(a, b, st::IS_DEBUG, u8::try_from(rng.below(2)).unwrap());
        set(
            a,
            b,
            st::LR_BUTTON_MODE,
            u8::try_from(rng.below(2)).unwrap(),
        );
        set(
            a,
            b,
            st::CAPTURE_SELECT,
            u8::try_from(rng.below(2)).unwrap(),
        );
        if rng.below(6) == 0 {
            // exactly pi (the f32 pi in the f64 constant): no wrap
            let pi = f32::from_bits(ffx_bridge::x87::store(st::PI_F64.get(a)));
            set(a, b, st::FREE_CAM_YAW, pi);
            set(a, b, st::PAD_RSTICK_X, 0x80);
        }
        twin("Sg_ControlCamera", a, b, 0x0083_f4b0, &[], |m| {
            st::control_camera(m)
        });

        // raw clips
        let obj = block(a, b, rng, 0x800);
        let nb = rng.below(5);
        let model = block(a, b, rng, 0x10);
        put(a, b, model + 0xa, u16::try_from(nb).unwrap());
        put(a, b, obj + 0x1c0, model);
        let bones = block(a, b, rng, 0x4000);
        put(a, b, obj + 0x328, bones);
        let scale = block(a, b, rng, 0x40);
        put(a, b, scale + 0x38, modest(rng));
        put(a, b, obj + 0x1c4, scale);
        put(a, b, obj + 0x7ec, u8::try_from(rng.below(5)).unwrap());
        let map = if rng.below(2) == 0 {
            0
        } else {
            let m = alloc(a, b, 0x20);
            for k in 0..8 {
                let v = if rng.below(4) == 0 {
                    0xffff
                } else {
                    rng.below(0x2d)
                };
                put(a, b, m + 8 + 2 * k, u16::try_from(v).unwrap());
            }
            m
        };
        put(a, b, obj + 0x758, map);
        let stride = rng.below(5) + 2;
        let in_clip = rng.below(6);
        let len = 0x10 + 2 * stride * 9 * in_clip;
        let clip = block(a, b, rng, len as usize + 4);
        put(a, b, clip, u16::try_from(stride).unwrap());
        put(a, b, clip + 2, u16::try_from(in_clip).unwrap());
        let frame = rng.below(stride);
        let t = (frame << 8)
            | if rng.below(3) == 0 {
                0
            } else {
                rng.below(0x100)
            };
        let t2 = rng.below(stride) << 8 | rng.below(0x100);
        let arg = if rng.below(10) == 0 { 0 } else { clip };
        twin(
            "SG_ApplyRawClip",
            a,
            b,
            0x0083_8750,
            &[obj, arg, t, t2],
            |m| {
                sq::apply_raw_clip(m, obj, arg, t, t2);
            },
        );
    }
}

// Main round 8: motion groups, id heights, bones per character, the motion
// script.

fn round8_main_cases(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rounds: u32,
    from_game: bool,
) {
    use ffx_sg::seq::bridge as sq;
    use ffx_sg::state::bridge as st;
    let f = f32::from_bits;
    let log = a.rebase(REC_LOG);
    recorder(a, b, 37, sq::CH_ANIM_COPY_NEAR, 1, 0);
    recorder(a, b, 38, sq::CH_ANIM_COPY2, 1, 0);
    recorder(a, b, 39, sq::CH_SET_MOTION_SEQ, 5, 0x6000);
    recorder(a, b, 40, sq::DBG_PRINTF, 2, 0x6001);
    let rets = a.rebase(REC_RETS);
    for _ in 0..rounds {
        // motion groups: sequences without tables (relocation moves only
        // the table pointers), the group table sometimes full
        put(a, b, log, 0u32);
        if !from_game {
            let groups = a.rebase(sq::MOTION_GROUPS.sym);
            let full = rng.below(6) == 0;
            for k in 0..50 {
                let used = full || rng.below(4) == 0;
                put(a, b, groups + 12 * k, if used { rng.next() | 1 } else { 0 });
            }
            let seqs = a.rebase(sq::SEQUENCES.sym);
            for k in 0..100 {
                put(
                    a,
                    b,
                    seqs + 20 * k,
                    if rng.below(3) == 0 { rng.next() | 1 } else { 0 },
                );
            }
        }
        let group = block(a, b, rng, 0x20);
        let n = rng.below(4);
        let records = block(a, b, rng, 20 * n as usize + 4);
        for k in 0..n {
            put(a, b, records + 20 * k + 8, 0u32);
        }
        // built at `group - d`: the table offset relative to that
        let d = if rng.below(2) == 0 {
            0
        } else {
            4 * rng.below(0x100)
        };
        put(a, b, group + 4, n);
        put(a, b, group + 0xc, records.wrapping_sub(d));
        put(a, b, group, group.wrapping_sub(d));
        let arg = if rng.below(10) == 0 { 0 } else { group };
        let (ga, gb, key) = (rng.next(), rng.next(), rng.next());
        twin(
            "SG_AddMotionGroup",
            a,
            b,
            0x0083_6c90,
            &[arg, ga, gb, key],
            |m| {
                sq::add_motion_group(&mut H(m), arg, ga, gb, key);
            },
        );

        // id heights
        let verts = block(a, b, rng, 8 * 8);
        for k in 0..24 {
            put(a, b, verts + 2 * k, u16::try_from(rng.below(64)).unwrap());
        }
        let table = alloc(a, b, 16 * 4);
        for k in 0..12 {
            put(
                a,
                b,
                table + 16 * (k / 3) + 2 * (k % 3),
                u16::try_from(rng.below(8)).unwrap(),
            );
        }
        set(
            a,
            b,
            st::ID_TABLE,
            if rng.below(8) == 0 { 0 } else { table },
        );
        set(a, b, st::ID_VERTICES, verts);
        let p = alloc(a, b, 16);
        for k in 0..3 {
            put(a, b, p + 4 * k, modest(rng));
        }
        let i = if rng.below(8) == 0 {
            u32::MAX
        } else {
            rng.below(4)
        };
        twin("Sg_IdGetHeight", a, b, 0x0083_e040, &[p, i], |m| {
            st::id_get_height(m, p, i)
        });

        // bones per character
        let chr = block(a, b, rng, 0x900);
        let nb = rng.below(4);
        let model = block(a, b, rng, 0x10);
        put(a, b, model + 0xa, u16::try_from(nb).unwrap());
        put(a, b, chr + 0x1c0, model);
        let def = block(a, b, rng, 0x40);
        put(a, b, def + 0x38, modest(rng));
        put(a, b, chr + 0x1c4, def);
        let chan = alloc(a, b, 0x18c * nb as usize + 4);
        for k in 0..(0x18c * nb) / 4 {
            put(a, b, chan + 4 * k, bounded(rng));
        }
        for k in 0..9 * nb {
            put(
                a,
                b,
                chan + 0x2c * k + 0x28,
                u8::try_from(rng.below(2)).unwrap(),
            );
        }
        put(a, b, chr + 0x748, chan);
        let bones = block(a, b, rng, 0x400);
        for k in 0..0x100 {
            put(a, b, bones + 4 * k, modest(rng));
        }
        put(a, b, chr + 0x328, bones);
        let map = if rng.below(2) == 0 {
            0
        } else {
            let m = alloc(a, b, 0x20);
            put(a, b, m + 2, u16::try_from(rng.below(5)).unwrap());
            for k in 0..8 {
                let v = if rng.below(4) == 0 {
                    0xffff
                } else {
                    rng.below(2)
                };
                put(a, b, m + 8 + 2 * k, u16::try_from(v).unwrap());
            }
            m
        };
        put(a, b, chr + 0x758, map);
        // mostly the copy path (where the map and kind decide)
        put(a, b, chr + 0x871, u8::from(rng.below(6) == 0));
        put(a, b, chr + 0x870, u8::from(rng.below(6) != 0));
        put(a, b, chr + 0x820, if rng.below(4) == 0 { 4u8 } else { 0 });
        let kinds = [0u16, 4, 0x5045, 1, 7];
        put(a, b, chr, kinds[rng.below(5) as usize]);
        let maps = [0x170, 0x11d, 0x50, 0x137, 0x64, 0x3c, 0x10, 0x200];
        set(a, b, sq::GRAPHIC_MAP_ID, maps[rng.below(8) as usize]);
        // the scenario around 0x9a, where map 0x3c changes
        put(a, b, rets + 4 * 22, 0x99 + rng.below(2));
        put(
            a,
            b,
            rets + 4 * 37,
            u32::from(rng.below(4) != 0) * (rng.next() | 1),
        );
        put(a, b, rets + 4 * 38, rng.below(2) * (rng.next() | 1));
        put(a, b, log, 0u32);
        twin("SG_ApplyBones", a, b, 0x0083_8f70, &[chr], |m| {
            sq::apply_bones(&mut H(m), chr);
        });

        // the motion script
        put(a, b, log, 0u32);
        let chr = block(a, b, rng, 0x800);
        let script = alloc(a, b, 16);
        let ops = [0u8, 1, 2, 3, 4, 5, 6, 7, 0xff];
        put(a, b, script, ops[rng.below(9) as usize]);
        // u16 operands at 1, 3, 5, 7: small (indices into 4 entries)
        for k in 1..10 {
            let v = if k % 2 == 1 { rng.below(4) } else { 0 };
            put(a, b, script + k, u8::try_from(v).unwrap());
        }
        put(a, b, chr + 0x714, script);
        let clips = block(a, b, rng, 0x20);
        let entries = block(a, b, rng, 16 * 4);
        let marks = block(a, b, rng, 2 * 4);
        for k in 0..4 {
            put(a, b, entries + 16 * k + 8, marks);
        }
        put(a, b, clips + 0x10, entries);
        put(a, b, chr + 0x710, clips);
        let jumps = block(a, b, rng, 0x10);
        let offs = block(a, b, rng, 2 * 4);
        put(a, b, jumps + 8, offs);
        put(a, b, chr + 0x70c, jumps);
        put(a, b, chr + 0x728, u16::try_from(rng.below(2)).unwrap());
        put(a, b, chr + 0x72a, u16::try_from(rng.below(3)).unwrap());
        put(a, b, chr + 0x737, u8::try_from(rng.below(5)).unwrap());
        twin("SG_SeqStep", a, b, 0x0083_7900, &[chr], |m| {
            sq::seq_step(&mut H(m), chr)
        });
    }
    let _ = f;
}

// Main round 9: the frame's pad.

/// A recording stub over `sym` whose answer is `table[arg]`, the argument
/// `k` (0-based) indexing words at `table` (an image address).
fn table_stub(a: &mut Oracle, b: &mut Oracle, slot: u32, sym: u32, arg: u8, table: u32) {
    for o in [a, b] {
        let code = o.rebase(REC_CODE) + 0x60 * slot;
        let t = o.rebase(table);
        // mov eax, [esp+4+4k]; mov eax, [eax*4 + table]; ret
        let mut s = vec![0x8b, 0x44, 0x24, 4 + 4 * arg, 0x8b, 0x04, 0x85];
        s.extend(t.to_le_bytes());
        s.push(0xc3);
        o.write(code, &s).unwrap();
        let at = o.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        o.write(at, &j).unwrap();
    }
}

/// Answers for the pad stubs (words by axis; the buttons at slot 0).
const PAD_ANSWERS: u32 = 0x0270_fe40;

fn round9_main_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::state::bridge as st;
    recorder(a, b, 41, st::TK_GET_CONTROLER_HIST, 3, 0);
    table_stub(a, b, 42, st::TK_GET_CONTROLER_ANALOG_HIST, 2, PAD_ANSWERS);
    let rets = a.rebase(REC_RETS);
    let answers = a.rebase(PAD_ANSWERS);
    for _ in 0..rounds {
        let held = if rng.below(2) == 0 {
            rng.next()
        } else {
            rng.below(16) | rng.next() & 0x400
        };
        put(a, b, rets + 4 * 41, held);
        for k in 0..4 {
            put(a, b, answers + 4 * k, rng.next());
        }
        set(
            a,
            b,
            st::PAD_LAST,
            if rng.below(2) == 0 {
                rng.next()
            } else {
                rng.below(16)
            },
        );
        set(a, b, st::IS_DEBUG, u8::from(rng.below(4) != 0));
        set(a, b, st::LR_BUTTON_MODE, u8::from(rng.below(3) == 0));
        let lv = [0u8, 1, 0x7f, 0x80, 0xff, 5][rng.below(6) as usize];
        set(a, b, st::EXEC_FRAMES, lv);
        set(
            a,
            b,
            st::FREE_CAM_YAW,
            f32::from_bits(bounded(rng) & 0xbfff_ffff),
        );
        set(
            a,
            b,
            st::FREE_CAM_PITCH,
            f32::from_bits(bounded(rng) & 0xbfff_ffff),
        );
        twin("Sg_ControlMain", a, b, 0x0082_d070, &[], |m| {
            st::control_main(&mut H(m))
        });
    }
}

/// The allocator stand-in: each call answers the next word of a list
/// (`mov eax, [cursor]; mov eax, [eax]; add dword [cursor], 4; ret`).
const ALLOC_CURSOR: u32 = 0x0270_fe60;
const ALLOC_LIST: u32 = 0x0270_fe80;
/// `Ch_OptposGetElement`'s answers by option point, `Chr_ModelEntry`'s by
/// index.
const OPT_ANSWERS: u32 = 0x0270_fec0;
const MODEL_ANSWERS: u32 = 0x0270_ff00;

fn alloc_stub(a: &mut Oracle, b: &mut Oracle, slot: u32, sym: u32) {
    for o in [a, b] {
        let code = o.rebase(REC_CODE) + 0x60 * slot;
        let cur = o.rebase(ALLOC_CURSOR);
        let mut s = vec![0xa1];
        s.extend(cur.to_le_bytes()); // mov eax, [cursor]
        s.extend([0x8b, 0x00]); // mov eax, [eax]
        s.extend([0x83, 0x05]);
        s.extend(cur.to_le_bytes());
        s.push(4); // add dword [cursor], 4
        s.push(0xc3);
        o.write(code, &s).unwrap();
        let at = o.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        o.write(at, &j).unwrap();
    }
}

fn init_bones_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::seq::bridge as sq;
    let log = a.rebase(REC_LOG);
    alloc_stub(a, b, 43, sq::USER_MALLOC);
    // sprintf through its import slot: a stub that only returns
    for o in [&mut *a, &mut *b] {
        let code = o.rebase(REC_CODE) + 0x60 * 44;
        o.write(code, &[0x31, 0xc0, 0xc3]).unwrap(); // xor eax, eax; ret
        let slot = o.rebase(sq::SPRINTF_SLOT);
        o.write(slot, &code.to_le_bytes()).unwrap();
    }
    recorder(a, b, 45, sq::SG_MEM_SET_STR, 1, 0x7000);
    recorder(a, b, 46, sq::SG_MEM_SET_TYPE, 2, 0x7001);
    recorder(a, b, 47, sq::MEMCLEAR, 2, 0x7002);
    recorder(a, b, 48, sq::CH_ELEM_INIT, 1, 0x7003);
    table_stub(a, b, 49, sq::CH_OPTPOS_GET_ELEMENT, 1, OPT_ANSWERS);
    table_stub(a, b, 50, sq::CHR_MODEL_ENTRY, 0, MODEL_ANSWERS);
    recorder(a, b, 51, sq::CH_MODEL_CREATE_PACKET, 1, 0x7004);
    let (cursor, list) = (a.rebase(ALLOC_CURSOR), a.rebase(ALLOC_LIST));
    let (opt, models) = (a.rebase(OPT_ANSWERS), a.rebase(MODEL_ANSWERS));
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        let n = rng.below(5);
        let m = rng.below(4);
        let c = if rng.below(2) == 0 {
            0
        } else {
            rng.below(3) + 1
        };
        let model = block(a, b, rng, 0x40);
        put(a, b, model + 0xa, u16::try_from(n).unwrap());
        put(a, b, model + 6, u16::try_from(m).unwrap());
        put(a, b, model + 0x2c, u16::try_from(c).unwrap());
        // bones: each parent itself (the root) or an earlier bone
        let table = block(a, b, rng, 20 * n as usize + 4);
        let mut depth = vec![];
        for k in 0..n {
            let parent = if k == 0 || rng.below(3) == 0 {
                k
            } else {
                rng.below(k)
            };
            put(a, b, table + 20 * k, u16::try_from(parent).unwrap());
            let d = if parent == k {
                0
            } else {
                depth[parent as usize] + 1
            };
            depth.push(d);
        }
        put(a, b, model + 0x1c, table);
        let chr = block(a, b, rng, 0x900);
        put(a, b, chr + 0x1c0, model);
        put(a, b, chr + 0x1c8, 0u32);
        // the allocations, in order
        let sizes = [0x160 * n, 0x18c * n, 0x38 * m, 0x60 * c];
        for (k, size) in (0u32..).zip(sizes) {
            let p = block(a, b, rng, size as usize + 4);
            put(a, b, list + 4 * k, p);
        }
        put(a, b, cursor, list);
        // option points 7 and 8: none, or two bones at the same depth
        let (pa, pb) = if n == 0 || rng.below(3) == 0 {
            (0, rng.below(3))
        } else {
            let x = rng.below(n);
            let same: Vec<u32> = (0..n)
                .filter(|&k| depth[k as usize] == depth[x as usize])
                .collect();
            (
                x,
                same[rng.below(u32::try_from(same.len()).unwrap()) as usize],
            )
        };
        put(a, b, opt + 4 * 7, pa);
        put(a, b, opt + 4 * 8, pb);
        for k in 0..m {
            let e = block(a, b, rng, 0x20);
            put(a, b, e + 0x18, u16::try_from(rng.below(2)).unwrap());
            put(a, b, models + 4 * k, e);
        }
        twin("SG_InitBones", a, b, 0x0082_77f0, &[chr], |m| {
            sq::init_bones(&mut H(m), chr);
        });
    }
}

/// Vertex lists for the envelope: rigid entries (16 bytes), then two
/// weighted lists (20 bytes); indices below 16.
fn envelope_lists(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> u32 {
    let counts = [rng.below(4), rng.below(4), rng.below(4)];
    let len = 12 + 16 * counts[0] + 20 * (counts[1] + counts[2]);
    let lists = alloc(a, b, len as usize + 4);
    let mut p = lists;
    for (l, &n) in counts.iter().enumerate() {
        put(a, b, p, n);
        p += 4;
        for _ in 0..n {
            for k in 0..3 {
                put(a, b, p + 4 * k, modest(rng));
            }
            if l == 0 {
                put(a, b, p + 0xc, rng.below(16));
                p += 0x10;
            } else {
                put(a, b, p + 0xc, modest(rng));
                put(a, b, p + 0x10, rng.below(16));
                p += 0x14;
            }
        }
    }
    lists
}

fn envelope_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::envelope::bridge as e;
    for _ in 0..rounds {
        let m = alloc(a, b, 64);
        for k in 0..16 {
            put(a, b, m + 4 * k, modest(rng));
        }
        let out = alloc(a, b, 16 * 16);
        for k in 0..64 {
            put(a, b, out + 4 * k, modest(rng));
        }
        let lists = envelope_lists(a, b, rng);
        twin(
            "sg_calcenvelopeW_asm_opt",
            a,
            b,
            0x006e_e270,
            &[lists, out, m],
            |o| {
                e::calc_envelope_w_opt(o, lists, out, m);
            },
        );
    }
}

/// Packed vertex lists (`sg_calcenvelope_asm*`): rigid entries, then
/// groups of first weighted entries ending in `0xffff`, 2 bytes, groups of
/// further weighted entries ending in `0xffff`. Indices below 0x110 (the
/// group bases 0 or 1).
fn packed_lists(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> u32 {
    let mut bytes: Vec<u8> = vec![];
    let w16 = |v: &mut Vec<u8>, x: u32| v.extend(u16::try_from(x & 0xffff).unwrap().to_le_bytes());
    let vertex = |v: &mut Vec<u8>, rng: &mut Xorshift| {
        for _ in 0..3 {
            w16(v, rng.next());
        }
    };
    let n = rng.below(4);
    w16(&mut bytes, n);
    w16(&mut bytes, rng.next());
    for _ in 0..n {
        vertex(&mut bytes, rng);
        w16(&mut bytes, rng.below(0x110));
    }
    for section in 0..2 {
        if section == 1 {
            w16(&mut bytes, rng.next());
        }
        for _ in 0..rng.below(3) {
            let c = rng.below(3);
            w16(&mut bytes, c);
            w16(&mut bytes, rng.below(2));
            for _ in 0..c {
                vertex(&mut bytes, rng);
                bytes.push(u8::try_from(rng.next() & 0xff).unwrap());
                bytes.push(u8::try_from(rng.below(16)).unwrap());
            }
        }
        w16(&mut bytes, 0xffff);
    }
    let lists = alloc(a, b, bytes.len() + 4);
    a.write(lists, &bytes).unwrap();
    b.write(lists, &bytes).unwrap();
    lists
}

fn packed_envelope_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::envelope::bridge as e;
    for _ in 0..rounds {
        let m = alloc(a, b, 64);
        for k in 0..16 {
            put(a, b, m + 4 * k, modest(rng));
        }
        let out = alloc(a, b, 16 * 0x110);
        for k in 0..4 * 0x110 {
            put(a, b, out + 4 * k, modest(rng));
        }
        let lists = packed_lists(a, b, rng);
        twin(
            "sg_calcenvelope_asm_opt",
            a,
            b,
            0x006e_ee40,
            &[lists, out, m],
            |o| {
                e::calc_envelope_opt(o, lists, out, m);
            },
        );
    }
}

/// Source-form vertex lists (`sg_calcenvelopeW_asm`): rigid 8-byte
/// entries after a `u16` count and 2 bytes, then two weighted lists of
/// 10-byte entries after a `u16` count; indices below 16.
fn source_lists(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> u32 {
    let mut bytes: Vec<u8> = vec![];
    let w16 = |v: &mut Vec<u8>, x: u32| v.extend(u16::try_from(x & 0xffff).unwrap().to_le_bytes());
    let n = rng.below(5);
    w16(&mut bytes, n);
    w16(&mut bytes, rng.next());
    for _ in 0..n {
        for _ in 0..3 {
            w16(&mut bytes, rng.next());
        }
        w16(&mut bytes, rng.below(16));
    }
    for _ in 0..2 {
        let n = rng.below(5);
        w16(&mut bytes, n);
        for _ in 0..n {
            for _ in 0..3 {
                w16(&mut bytes, rng.next());
            }
            w16(&mut bytes, rng.below(16));
            w16(&mut bytes, rng.next());
        }
    }
    let lists = alloc(a, b, bytes.len() + 4);
    a.write(lists, &bytes).unwrap();
    b.write(lists, &bytes).unwrap();
    lists
}

fn source_envelope_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::envelope::bridge as e;
    let (cursor, list) = (a.rebase(ALLOC_CURSOR), a.rebase(ALLOC_LIST));
    for _ in 0..rounds {
        let m = alloc(a, b, 64);
        for k in 0..16 {
            put(a, b, m + 4 * k, modest(rng));
        }
        let out = alloc(a, b, 16 * 16);
        for k in 0..64 {
            put(a, b, out + 4 * k, modest(rng));
        }
        let lists = source_lists(a, b, rng);
        let mask = if rng.below(3) == 0 {
            0
        } else {
            let mk = alloc(a, b, 16);
            for k in 0..16 {
                put(a, b, mk + k, u8::from(rng.below(3) != 0));
            }
            mk
        };
        let dst = block(a, b, rng, 0x200);
        put(a, b, list, dst);
        put(a, b, cursor, list);
        twin(
            "sg_calcenvelopeW_asm",
            a,
            b,
            0x006e_d9e0,
            &[lists, out, mask, m],
            |o| e::calc_envelope_w(&mut H(o), lists, out, mask, m),
        );
    }
}

fn source_packed_envelope_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::envelope::bridge as e;
    let (cursor, list) = (a.rebase(ALLOC_CURSOR), a.rebase(ALLOC_LIST));
    for _ in 0..rounds {
        let m = alloc(a, b, 64);
        for k in 0..16 {
            put(a, b, m + 4 * k, modest(rng));
        }
        let out = alloc(a, b, 16 * 0x110);
        for k in 0..4 * 0x110 {
            put(a, b, out + 4 * k, modest(rng));
        }
        let lists = packed_lists(a, b, rng);
        let mask = if rng.below(3) == 0 {
            0
        } else {
            let mk = alloc(a, b, 0x110);
            for k in 0..0x110 {
                put(a, b, mk + k, u8::from(rng.below(3) != 0));
            }
            mk
        };
        let dst = block(a, b, rng, 0x200);
        put(a, b, list, dst);
        put(a, b, cursor, list);
        twin(
            "sg_calcenvelope_asm",
            a,
            b,
            0x006e_e570,
            &[lists, out, mask, m],
            |o| e::calc_envelope(&mut H(o), lists, out, mask, m),
        );
    }
}

// Main round 10: two characters pushed apart.
fn push_apart_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::state::bridge as st;
    let weight = |rng: &mut Xorshift| match rng.below(5) {
        0 => (-1.0f32).to_bits(),
        1 => 0,
        2 => 0.5f32.to_bits(),
        3 => modest(rng),
        _ => bounded(rng),
    };
    for _ in 0..rounds {
        let ca = alloc(a, b, 0x600);
        let cb = alloc(a, b, 0x600);
        for c in [ca, cb] {
            put(a, b, c + 0x500, weight(rng));
            for k in [0xc, 0x10, 0x14, 0x1c, 0x20, 0x24] {
                put(a, b, c + k, bounded(rng));
            }
            for k in [0x4c, 0x54] {
                let v = if rng.below(3) == 0 { 0 } else { bounded(rng) };
                put(a, b, c + k, v);
            }
            put(
                a,
                b,
                c + 0x194,
                rng.next() & !0x20 | (rng.below(4) / 3) << 5,
            );
        }
        if rng.below(6) == 0 {
            // equal anchor distances
            for k in [0xc, 0x10, 0x14, 0x1c, 0x20, 0x24] {
                let v: u32 = Mem::read(a, ca + k);
                put(a, b, cb + k, v);
            }
        }
        if rng.below(10) == 0 {
            for k in [0xc, 0x14] {
                let v: u32 = Mem::read(a, ca + k);
                put(a, b, cb + k, v);
            }
        }
        let reach = bounded(rng);
        let dist = match rng.below(4) {
            0 => reach,
            _ => bounded(rng),
        };
        twin(
            "Sg_ChrPushApart",
            a,
            b,
            0x0083_ecd0,
            &[ca, cb, reach, dist],
            |m| st::chr_push_apart(m, ca, cb, f32::from_bits(reach), f32::from_bits(dist)),
        );
    }
}

// Main round 10: the cloth anchors.
fn cloth_anchor_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::vec::bridge as v;
    let log = a.rebase(REC_LOG);
    let rets = a.rebase(REC_RETS);
    recorder(a, b, 52, 0x0084_0850, 2, 0);
    recorder(a, b, 53, 0x0064_1da0, 0, 0);
    // the anchor answers by group, in unused answer slots 60..64
    let anchors = rets + 4 * 60;
    table_stub(a, b, 54, 0x0083_16d0, 1, REC_RETS + 4 * 60);
    let words = |a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, n: usize| {
        let p = alloc(a, b, 4 * n.max(1));
        for k in 0..n {
            put(a, b, p + 4 * u32::try_from(k).unwrap(), bounded(rng));
        }
        p
    };
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for g in 0..4 {
            put(a, b, anchors + 4 * g, u32::from(rng.below(3) == 0));
        }
        let hash = if rng.below(3) == 0 {
            0xc58
        } else {
            rng.next() & 0xfff
        };
        put(a, b, rets + 4 * 53, hash);
        // the model: three groups
        let model = alloc(a, b, 3 * 0x38);
        for g in 0..3 {
            let grp = model + 0x38 * g;
            let head = alloc(a, b, 0x20);
            let subs = [0u16, 3, 100, 200][rng.below(4) as usize];
            put(a, b, head + 0x18, subs);
            put(a, b, grp, head);
            let vs = words(a, b, rng, 8 * 4);
            put(a, b, grp + 8, vs);
            let tbl = if rng.below(6) == 0 {
                0
            } else {
                let t = alloc(a, b, 400);
                for r in 0..100 {
                    if rng.below(4) != 0 {
                        let sub = alloc(a, b, 0x48);
                        let faces = alloc(a, b, 6 * 8);
                        for q in 0..24 {
                            put(a, b, faces + 2 * q, u16::try_from(rng.below(8)).unwrap());
                        }
                        put(a, b, sub + 0x44, faces);
                        put(a, b, t + 4 * r, sub);
                    }
                }
                t
            };
            put(a, b, grp + 0x2c, tbl);
        }
        let chr = alloc(a, b, 0x900);
        put(a, b, chr, u16::try_from(rng.below(3)).unwrap());
        put(a, b, chr + 0x180, u8::try_from(rng.next() & 0xff).unwrap());
        let flags = if rng.below(6) == 0 {
            rng.next() & !0x20_0000
        } else {
            rng.next() | 0x20_0000
        };
        put(a, b, chr + 0x194, flags);
        put(a, b, chr + 0x1bc, model);
        for r in [0x1d0u32, 0x1e0, 0x1f0] {
            for k in 0..3 {
                let x = match rng.below(5) {
                    0 => 0,
                    1 => 1e-4f32.to_bits(),
                    2 => 4e-4f32.to_bits(),
                    _ => bounded(rng),
                };
                put(a, b, chr + r + 4 * k, x);
            }
        }
        let lists = rng.below(4);
        let cloth = if rng.below(8) == 0 {
            0
        } else {
            alloc(a, b, 0x40)
        };
        let vb = alloc(a, b, 4 + 4 * 4);
        put(a, b, rets + 4 * 52, if rng.below(8) == 0 { 0 } else { vb });
        if cloth != 0 {
            put(a, b, cloth + 0x14, lists);
            for i in 0..lists {
                let m = rng.below(4);
                let l = if rng.below(8) == 0 {
                    0
                } else {
                    alloc(a, b, 4 + 4 * 4)
                };
                let vl = if rng.below(8) == 0 {
                    0
                } else {
                    alloc(a, b, 4 + 4 * 4)
                };
                put(a, b, cloth + 0x30 + 4 * i, l);
                put(a, b, vb + 4 + 4 * i, vl);
                if l == 0 || vl == 0 {
                    continue;
                }
                put(a, b, l, m);
                for j in 0..m {
                    let n = rng.below(5);
                    let e = if rng.below(10) == 0 {
                        0
                    } else {
                        alloc(a, b, 0x50)
                    };
                    let ve = if rng.below(10) == 0 {
                        0
                    } else {
                        alloc(a, b, 0x10)
                    };
                    put(a, b, l + 4 + 4 * j, e);
                    put(a, b, vl + 4 + 4 * j, ve);
                    if e != 0 {
                        put(a, b, e + 0x40, u16::try_from(rng.below(3)).unwrap());
                        let k = [2u16, 102, 5, 0][rng.below(4) as usize]
                            + if rng.below(3) == 0 {
                                u16::try_from(rng.below(300)).unwrap()
                            } else {
                                0
                            };
                        put(a, b, e + 0x42, k);
                        put(a, b, e + 0x44, n);
                        put(a, b, e + 0x48, rng.next());
                        let out = if rng.below(8) == 0 {
                            0
                        } else {
                            words(a, b, rng, 4 * 4)
                        };
                        put(a, b, e + 0x4c, out);
                    }
                    if ve != 0 {
                        let tri = alloc(a, b, 8 * 4);
                        for q in 0..4 {
                            put(a, b, tri + 8 * q, u8::try_from(rng.next() & 0xff).unwrap());
                            let idx = if rng.below(6) == 0 {
                                0xffff
                            } else {
                                u16::try_from(rng.below(8)).unwrap()
                            };
                            put(a, b, tri + 8 * q + 2, idx);
                            put(a, b, tri + 8 * q + 4, bounded(rng));
                        }
                        put(a, b, ve + 8, tri);
                        let ws = words(a, b, rng, 3 * 4);
                        put(a, b, ve + 0xc, ws);
                    }
                }
            }
        }
        put(a, b, chr + 0x834, cloth);
        twin("Sg_ClothPlaceAnchors", a, b, 0x0082_dbc0, &[chr], |m| {
            v::cloth_place_anchors(&mut H(m), chr);
        });
    }
}

/// `rand()`'s stand-in at its import slot: 16 answers in turn from
/// `REC_RETS + 4 * 65`, the turn at `REC_RETS + 4 * 64`.
fn rand_stub(a: &mut Oracle, b: &mut Oracle) {
    for o in [a, b] {
        let code = o.rebase(REC_CODE) + 0x60 * 57;
        let cur = o.rebase(REC_RETS) + 4 * 64;
        let table = o.rebase(REC_RETS) + 4 * 65;
        let mut c = vec![0xa1];
        c.extend(cur.to_le_bytes()); // mov eax, [cur]
        c.extend([0xff, 0x05]);
        c.extend(cur.to_le_bytes()); // inc dword [cur]
        c.extend([0x83, 0xe0, 0x0f]); // and eax, 15
        c.extend([0x8b, 0x04, 0x85]);
        c.extend(table.to_le_bytes()); // mov eax, [eax*4 + table]
        c.push(0xc3);
        o.write(code, &c).unwrap();
        let slot = o.rebase(0x00b0_c420);
        o.write(slot, &code.to_le_bytes()).unwrap();
    }
}

/// A matrix near a rotation plus translation: the identity with small
/// changes, a translation within ±2 (the cloth stays at a scale where its
/// collisions happen).
fn near_rigid(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, m: u32) {
    for k in 0..16u32 {
        let x = match k {
            0 | 5 | 10 | 15 => 1.0,
            3 | 7 | 11 => 0.0,
            12..=14 => f32::from_bits(bounded(rng)) / 4.0,
            _ => f32::from_bits(bounded(rng)) / 32.0,
        };
        put(a, b, m + 4 * k, x);
    }
}

// Main round 10: the cloth step.
#[allow(clippy::too_many_lines)]
fn cloth_step_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::cloth::bridge as cl;
    let rets = a.rebase(REC_RETS);
    patch_trig(a);
    patch_trig(b);
    rand_stub(a, b);
    recorder(a, b, 55, cl::MS_BATTLE_CHECK, 0, 0);
    recorder(a, b, 56, cl::MAGIC_FILE_GET_MAGIC_ID, 0, 0);
    // getScenerioFlag is slot 22's recorder
    let f = |x: f32| x.to_bits();
    let floats = |a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, n: u32| {
        let p = alloc(a, b, 4 * n as usize);
        for k in 0..n {
            put(a, b, p + 4 * k, bounded(rng));
        }
        p
    };
    for _ in 0..rounds {
        put(a, b, rets + 4 * 64, rng.below(16));
        for q in 0..16 {
            put(a, b, rets + 4 * (65 + q), rng.next() & 0x7fff);
        }
        let battle = rng.below(2) == 0;
        put(a, b, rets + 4 * 55, u32::from(battle));
        // map 0x107's lift: character 2, group 2, strand 2
        let lift107 = rng.below(4) == 0;
        let magic = if rng.below(3) == 0 { 0x1b4 } else { 7 };
        put(a, b, rets + 4 * 56, magic);
        let sf = if lift107 {
            [0x532, 0x53c][rng.below(2) as usize]
        } else {
            [0u32, 1, 0x531, 0x532, 0x53c, 0x53d][rng.below(6) as usize]
        };
        put(a, b, rets + 4 * 22, sf);
        let map = if lift107 {
            0x107
        } else {
            [0x15d, 0x84, 0xc0, 0x107, 0x3b, 0x17d][rng.below(6) as usize]
        };
        set(a, b, cl::MAP, map);
        let strength = if rng.below(8) == 0 {
            f32::from(u16::try_from(rng.below(600)).unwrap())
        } else {
            f32::from_bits(bounded(rng)) / 4096.0
        };
        let wind = [f(strength), bounded(rng), bounded(rng)];
        set(a, b, cl::WIND, wind.map(f32::from_bits));

        // the model: records, bones, collision points
        let nrec = rng.below(4);
        let nbones = 4u32;
        let bones = alloc(a, b, 0x160 * nbones as usize);
        for i in 0..nbones * 0x160 / 4 {
            put(a, b, bones + 4 * i, bounded(rng));
        }
        for i in 0..nbones {
            let m = bones + 0x160 * i + 0x88;
            near_rigid(a, b, rng, m);
            // bone translations are in thousandths (scaled by 1000 around
            // the products)
            for k in 12..15 {
                let x: f32 = Mem::read(&*a, m + 4 * k);
                put(a, b, m + 4 * k, x / 1000.0);
            }
        }
        let records = alloc(a, b, 0x40 * 4);
        for r in 0..nrec {
            let rec = records + 0x40 * r;
            put(a, b, rec, bounded(rng));
            let reach = if rng.below(2) == 0 {
                [f(40.0), f(1.0e6), f(1.0e12)][rng.below(3) as usize]
            } else {
                bounded(rng)
            };
            put(a, b, rec + 4, reach);
            put(a, b, rec + 8, u16::try_from(rng.below(4)).unwrap());
            put(a, b, rec + 0xa, i16::try_from(rng.below(nbones)).unwrap());
            put(a, b, rec + 0xc, bounded(rng));
            for k in 4..12 {
                put(a, b, rec + 4 * k, bounded(rng));
            }
        }
        let pts = floats(a, b, rng, 0x18 * 4);
        let ngroups = if lift107 { 3 } else { rng.below(4) };
        let model = alloc(a, b, 0x40);
        put(a, b, model + 6, u16::try_from(ngroups).unwrap());
        put(a, b, model + 0x2c, u16::try_from(nrec).unwrap());
        put(a, b, model + 0x30, records);

        // the groups, their strands
        let groups = alloc(a, b, 0x38 * 4);
        for g in 0..ngroups {
            let grp = groups + 0x38 * g;
            let hdr = alloc(a, b, 0x20);
            put(a, b, hdr, u16::try_from(rng.below(nbones)).unwrap());
            let ns = if lift107 {
                3
            } else if rng.below(6) == 0 {
                0
            } else {
                1 + rng.below(3)
            };
            put(a, b, hdr + 0x18, u16::try_from(ns).unwrap());
            let strands = alloc(a, b, 0x24 * 3);
            put(a, b, hdr + 0x1c, strands);
            put(a, b, grp, hdr);
            let verts = floats(a, b, rng, 16 * 4);
            put(a, b, grp + 8, verts);
            for k in [0x1c, 0x20, 0x24] {
                let x = if rng.below(3) == 0 { 0 } else { bounded(rng) };
                put(a, b, grp + k, x);
            }
            let marks = alloc(a, b, 16);
            put(a, b, grp + 0x30, marks);
            put(a, b, grp + 0x34, u32::from(rng.below(2) == 0));
            let mut vsize = 0;
            for k in 0..ns {
                let s = strands + 0x24 * k;
                let total = 1 + rng.below(5);
                let fixed = rng.below(total + 1).min(2);
                let ends = rng.below(total - fixed + 1);
                let nanch = rng.below(3);
                let ncol = if nrec == 0 { 0 } else { 1 + rng.below(2) };
                put(a, b, s, i16::try_from(fixed).unwrap());
                put(a, b, s + 4, bounded(rng));
                let idx = alloc(a, b, 2 * 8);
                for q in 0..8 {
                    put(a, b, idx + 2 * q, u16::try_from(rng.below(16)).unwrap());
                }
                put(a, b, s + 8, idx);
                // rods: up to three runs, then 0
                let tc = total + nanch;
                let rods = alloc(a, b, 3 * (12 + 3 * 12) + 2);
                let mut r = rods;
                for _ in 0..rng.below(4) {
                    let kind = 1 + rng.below(7);
                    let n = rng.below(4);
                    put(a, b, r, i16::try_from(kind).unwrap());
                    put(a, b, r + 2, i16::try_from(n).unwrap());
                    for j in 0..n {
                        let rod = r + 12 + 12 * j;
                        put(a, b, rod, i16::try_from(rng.below(tc)).unwrap());
                        put(a, b, rod + 2, i16::try_from(rng.below(tc)).unwrap());
                        put(a, b, rod + 4, bounded(rng));
                        let lim = if rng.below(2) == 0 {
                            f(4.0)
                        } else {
                            bounded(rng)
                        };
                        put(a, b, rod + 8, lim);
                    }
                    r += 12 + 12 * n;
                }
                put(a, b, r, 0u16);
                put(a, b, s + 0xc, rods);
                put(a, b, s + 0x12, i8::try_from(ncol).unwrap());
                put(a, b, s + 0x13, i8::try_from(nanch).unwrap());
                let cols = alloc(a, b, 8);
                for c in 0..3 {
                    let ci = if nrec == 0 { 0 } else { rng.below(nrec) };
                    put(a, b, cols + 2 * c, i16::try_from(ci).unwrap());
                }
                put(a, b, s + 0x14, cols);
                put(a, b, s + 0x18, bounded(rng));
                put(a, b, s + 0x1c, i16::try_from(total).unwrap());
                put(a, b, s + 0x1e, i16::try_from(ends).unwrap());
                let anch = alloc(a, b, 0x20 * 3);
                for j in 0..3 {
                    for q in 0..4 {
                        put(a, b, anch + 0x20 * j + 4 * q, bounded(rng));
                    }
                    put(
                        a,
                        b,
                        anch + 0x20 * j + 0x10,
                        i16::try_from(rng.below(nbones)).unwrap(),
                    );
                }
                put(a, b, s + 0x20, anch);
                vsize += 3 * tc;
            }
            let vp = floats(a, b, rng, 4 * vsize.max(1));
            put(a, b, grp + 0xc, vp);
        }

        // the character
        let chr = alloc(a, b, 0x900);
        let id = if lift107 {
            2
        } else if battle && rng.below(2) == 0 {
            3
        } else {
            [1u16, 2, 3, 6, 7, 0x1120, 0x4001, 0x66][rng.below(8) as usize]
        };
        put(a, b, chr, id);
        for k in [0x10, 0x20, 0x7c, 0x80, 0x84] {
            put(a, b, chr + k, bounded(rng));
        }
        let fl = if rng.below(8) == 0 {
            rng.next()
        } else {
            rng.next() | 0x60_0000
        };
        put(a, b, chr + 0x194, fl);
        put(a, b, chr + 0x198, u8::try_from(rng.next() & 0xff).unwrap());
        put(a, b, chr + 0x1bc, groups);
        put(a, b, chr + 0x1c0, model);
        let sc = alloc(a, b, 0x40);
        put(
            a,
            b,
            sc + 0x3c,
            if rng.below(2) == 0 {
                f(1.0)
            } else {
                bounded(rng)
            },
        );
        put(a, b, chr + 0x1c4, sc);
        near_rigid(a, b, rng, chr + 0x1d0);
        near_rigid(a, b, rng, chr + 0x210);
        put(a, b, chr + 0x328, bones);
        put(a, b, chr + 0x32c, pts);
        for k in 0..8 {
            let x = if rng.below(3) == 0 {
                0
            } else {
                modest(rng) & 0xbfff_ffff
            };
            put(a, b, chr + 0x684 + 4 * k, x);
        }
        let motion = [0x3008_101a_u32, 0x310b0, 7][rng.below(3) as usize];
        put(a, b, chr + 0x72c, motion);
        for k in [0x804, 0x808, 0x80c, 0x814, 0x818] {
            put(a, b, chr + k, bounded(rng));
        }
        put(
            a,
            b,
            chr + 0x81c,
            if rng.below(2) == 0 { 0 } else { bounded(rng) },
        );
        put(a, b, chr + 0x810, u16::try_from(rng.below(6) / 5).unwrap());
        put(a, b, chr + 0x812, i16::try_from(rng.below(3)).unwrap());
        put(
            a,
            b,
            chr + 0x820,
            rng.next() & if rng.below(4) == 0 { 0xf } else { 0xb },
        );
        twin("Sg_ClothStep", a, b, 0x0082_2de0, &[chr], |m| {
            cl::cloth_step(&mut H(m), chr, &mut stand_in);
        });
    }
}

// Main round 11.
fn round11_main_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::control::bridge as c;
    for n in 0..rounds {
        let ids = [
            0x4040_0003,
            0x4080_0001,
            0x4080_0002,
            0x4080_0003,
            0x4080_0004,
            0x4040_0002,
        ];
        let id = if n % 3 == 0 {
            rng.next()
        } else {
            ids[rng.below(6) as usize]
        };
        twin("ExportValidateInterface", a, b, 0x0082_02e0, &[id], |_| {
            c::export_validate_interface(id)
        });
    }
}

// Main round 11: the fades and the focus.
fn draw_filter_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::control::bridge as c;
    let log = a.rebase(REC_LOG);
    for (k, sym, n) in [
        (82, c::GRAPHIC_HAS_RUN_DRAW_FILTER, 0),
        (83, c::GRAPHIC_RESET_HAS_RUN_DRAW_FOCUS, 0),
        (84, c::GRAPHIC_HAS_RUN_DRAW_FOCUS, 0),
        (85, c::GRAPHIC_SET_FOCUS_Z, 2),
        (86, c::GRAPHIC_ADD_SCREEN_FADE_TASK, 5),
        (87, c::SG_DRAW_FOCUS, 1),
        (88, c::SG_DRAW_LEVEL, 1),
    ] {
        recorder(a, b, k, sym, n, 0);
    }
    let slots = a.rebase(c::FADE_SLOTS);
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        set(
            a,
            b,
            c::FADE_ACTIVE,
            if rng.below(6) == 0 {
                0
            } else {
                rng.next() & 0x1f
            },
        );
        set(a, b, c::FOCUS_ON, rng.below(2));
        set(a, b, c::FOCUS_DRAW, rng.below(2));
        for z in c::FOCUS_Z {
            set(a, b, z, if rng.below(3) == 0 { 0 } else { rng.next() });
        }
        set(a, b, c::ACC_CURRENT_REQUEST, u32::from(rng.below(3) == 0));
        set(
            a,
            b,
            c::SCREEN_W,
            if rng.below(2) == 0 { 0x200 } else { 0x280 },
        );
        set(a, b, c::DREAM_ON, rng.below(2));
        set(
            a,
            b,
            c::DREAM_FADE_MODE,
            if rng.below(2) == 0 { u32::MAX } else { 2 },
        );
        set(
            a,
            b,
            c::DREAM_FADE_TOTAL,
            i32::try_from(rng.below(40)).unwrap() - 4,
        );
        set(a, b, c::DREAM_FADE_OUT, rng.below(2));
        let small = |rng: &mut Xorshift, lo: i32, hi: u32| {
            i16::try_from(i32::try_from(rng.below(hi)).unwrap() + lo).unwrap()
        };
        for i in 0..5 {
            let s = slots + 0x10 * i;
            put(a, b, s, u8::try_from(rng.below(10)).unwrap());
            for o in 1..4 {
                put(a, b, s + o, u8::try_from(rng.next() & 0xff).unwrap());
            }
            let n = if rng.below(5) == 0 {
                0
            } else {
                small(rng, -3, 24)
            };
            put(a, b, s + 4, n);
            let total = small(rng, 1, 40) * if rng.below(6) == 0 { -1 } else { 1 };
            put(a, b, s + 6, total);
            put(a, b, s + 8, small(rng, -40, 120));
            put(a, b, s + 0xa, small(rng, 0, 30));
        }
        twin("Sg_DrawFilter", a, b, 0x0082_c0c0, &[], |m| {
            c::draw_filter(&mut H(m));
        });
    }
}

// Main round 11: sg_calcenvelope_normW's translated VU0 and MIPS helpers.
fn vu_helper_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::envelope::bridge as e;
    let acc = a.rebase(e::VU0_ACC);
    let val = |rng: &mut Xorshift| match rng.below(4) {
        0 => rng.float(),
        1 => modest(rng),
        _ => bounded(rng),
    };
    for n in 0..rounds {
        let v = rng.next();
        #[allow(clippy::cast_possible_truncation)]
        let h = v as u16;
        twin("Sg_SetChrPacketState", a, b, 0x0084_5c30, &[v], |m| {
            e::set_chr_packet_state(m, h);
        });
        twin("Sg_SetChrPacketWord", a, b, 0x0084_5c40, &[v], |m| {
            e::set_chr_packet_word(m, h);
        });
        let src = alloc(a, b, 16);
        put(a, b, src, rng.next());
        put(a, b, src + 4, rng.next());
        let dst = if n % 4 == 0 { src } else { alloc(a, b, 16) };
        let imm = rng.next();
        twin("sg_Daddiu", a, b, 0x0084_5cc0, &[dst, src, imm], |m| {
            e::daddiu(m, dst, src, imm)
        });
        let sh = rng.next();
        twin("sg_Sll", a, b, 0x0084_5d70, &[dst, src, sh], |m| {
            e::sll(m, dst, src, sh)
        });
        let f = val(rng);
        twin("sg_SetFloat", a, b, 0x0084_5cf0, &[dst, f], |m| {
            e::set_float(m, dst, f32::from_bits(f))
        });
        let v3 = alloc(a, b, 16);
        for k in 0..4 {
            put(a, b, v3 + 4 * k, val(rng));
            put(a, b, acc + 4 * k, val(rng));
        }
        let out = match n % 3 {
            0 => v3,
            1 => acc,
            _ => alloc(a, b, 16),
        };
        let s = val(rng);
        twin("sg_VmulScalar", a, b, 0x0084_5e80, &[out, v3, s], |m| {
            e::vmul_scalar(m, out, v3, f32::from_bits(s))
        });
        let s = val(rng);
        twin(
            "sg_VmaddScalar",
            a,
            b,
            0x0084_5e40,
            &[out, v3, 0, 0, s],
            |m| e::vmadd_scalar(m, out, v3, f32::from_bits(s)),
        );
        let v = if n % 5 == 0 { acc } else { v3 };
        let s = val(rng);
        twin("sg_VmaddaScalar", a, b, 0x0084_5df0, &[0, v, 0, s], |m| {
            e::vmadda_scalar(m, v, f32::from_bits(s))
        });
    }
}

// Main round 13: the player's control from the pad.

/// `_CIatan` and `_CIatan2` (imports the oracle lacks) replaced at their
/// thunks by exact stand-ins: `-x` (`fchs`) and `y - x` (`fsubp st(1),
/// st`); the bridge gets the same through its callbacks.
fn patch_atan(o: &mut Oracle) {
    for (thunk, code) in [
        (0x0094_9d5c, &[0xd9, 0xe0, 0xc3][..]),
        (0x0094_98bc, &[0xde, 0xe9, 0xc3][..]),
    ] {
        let at = o.rebase(thunk);
        o.write(at, code).unwrap();
    }
}

fn atan_stand_in(t: ffx_battle::pos::bridge::Trig, x: u32) -> u32 {
    use ffx_bridge::x87::{load, store};
    match t {
        ffx_battle::pos::bridge::Trig::Atan => store(-load(x)),
        _ => panic!("no cosine or sine here"),
    }
}

fn atan2_stand_in(y: u32, x: u32) -> u32 {
    use ffx_bridge::x87::{load, store, sub};
    store(sub(load(y), load(x)))
}

/// A stick byte: at rest, at a dead zone's edges, or any.
fn stick(rng: &mut Xorshift) -> u8 {
    const EDGES: [u8; 10] = [0x80, 0x58, 0x59, 0xa7, 0xa8, 88, 89, 167, 168, 0];
    match rng.below(4) {
        0 => EDGES[rng.below(10) as usize],
        1 => 0x80,
        _ => rng.next().to_le_bytes()[0],
    }
}

fn control_player_cases(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rounds: u32,
    from_game: bool,
) {
    use ffx_sg::player::bridge as p;
    patch_atan(a);
    patch_atan(b);
    let log = a.rebase(REC_LOG);
    let rets = a.rebase(REC_RETS);
    recorder(a, b, 89, p::SG_GET_PLAYER_CONTROL_ENABLE, 0, 0);
    recorder(a, b, 90, p::GRAPHIC_LEVEL_GET_NAME_HASH, 0, 0);
    let game_camera = ffx_battle::camera::bridge::CAMERA.get(a);
    let level = |rng: &mut Xorshift| -> i32 {
        match rng.below(8) {
            0 => rng.next().cast_signed(),
            1 => 0,
            2 => 0x100,
            // the speed's floor and top exactly (48, 256 from one axis)
            3 => [0x30, 0x50, 0x130][rng.below(3) as usize],
            _ => rng.below(0x180).cast_signed() - 0x40,
        }
    };
    let angle = |rng: &mut Xorshift| {
        if rng.below(6) == 0 {
            rng.float()
        } else {
            bounded(rng)
        }
    };
    for n in 0..rounds {
        put(a, b, log, 0u32);
        put(a, b, rets + 4 * 89, u32::from(rng.below(6) != 0));
        let hash = [0x19c, 0x220, 0x19c, 7][rng.below(4) as usize];
        put(
            a,
            b,
            rets + 4 * 90,
            if rng.below(3) == 0 { hash } else { 0 },
        );
        set(a, b, p::PAD_LSTICK_Y, stick(rng));
        set(a, b, p::PAD_LSTICK_X, stick(rng));
        let mut held = rng.next();
        if rng.below(3) != 0 {
            held &= !0xf000;
        }
        set(a, b, p::PAD_HELD, held);
        let chr = if rng.below(10) == 0 {
            0
        } else {
            let chr = block(a, b, rng, 0x880);
            put(a, b, chr + ffx_chara::layout::GRAV, modest(rng));
            if rng.below(2) == 0 {
                put(a, b, chr + ffx_chara::layout::MOTION_TYPE, 2u8);
            }
            chr
        };
        set(a, b, p::PLAYER, chr);
        for l in p::LEVELS {
            set(a, b, l, level(rng));
        }
        for g in [
            p::CAMERA_YAW,
            p::STICK_DIR,
            p::STICK_DIR_TAKEN,
            p::DIR_OFFSET,
            p::DIR_FIX,
        ] {
            let v = angle(rng);
            put(a, b, g.ptr(a).addr, v);
        }
        for k in 0..4 {
            let at = a.rebase(p::SPEED) + 4 * k;
            put(a, b, at, modest(rng));
        }
        if rng.below(6) == 0 {
            // a turn of exactly the bound since the yaw was taken
            let turn = p::TURN.get(a);
            set(a, b, p::STICK_DIR, turn);
            set(a, b, p::STICK_DIR_TAKEN, 0.0);
        }
        set(a, b, p::DIVING, u8::from(rng.below(2) == 0));
        set(a, b, p::DIR_FIX_ON, u8::from(rng.below(3) == 0));
        let follow = if rng.below(3) == 0 { rng.below(8) } else { 3 };
        set(a, b, p::CAMERA_FOLLOW, u8::try_from(follow).unwrap());
        let cam = if from_game && game_camera != 0 && n % 2 == 0 {
            game_camera
        } else {
            let cam = alloc(a, b, 0x700);
            for k in 0..4 {
                put(a, b, cam + 0x50 + 4 * k, bounded(rng));
                put(a, b, cam + 0x66c + 4 * k, bounded(rng));
            }
            if rng.below(8) == 0 {
                let w: [u32; 4] = Mem::read(a, cam + 0x50);
                put(a, b, cam + 0x66c, w);
            }
            cam
        };
        set(a, b, ffx_battle::camera::bridge::CAMERA, cam);
        twin("Sg_ControlPlayer", a, b, 0x0082_d180, &[], |m| {
            p::control_player(&mut H(m), &atan_stand_in, &atan2_stand_in);
        });
    }
}

#[test]
fn control_player_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0d18);
    control_player_cases(&mut a, &mut b, &mut rng, 3000, false);
}

#[test]
fn control_player_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0d19);
    control_player_cases(&mut a, &mut b, &mut rng, 3000, true);
}

// Main round 13: sg_calcenvelope_normW.

/// Normal lists (`sg_calcenvelope_normW`): a `u16` count, 2 bytes, rigid
/// entries (8 bytes: `i16` x, y, z, `u16` index); then twice a `u16` count
/// and weighted entries (10 bytes: x, y, z, `u16` weight, index). Indices
/// below 16.
fn normal_lists(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> u32 {
    let counts = [rng.below(5), rng.below(5), rng.below(5)];
    let len = 8 + 8 * counts[0] + 10 * (counts[1] + counts[2]);
    let lists = alloc(a, b, len as usize + 4);
    let mut p = lists;
    let half = |rng: &mut Xorshift| -> u16 {
        match rng.below(3) {
            0 => rng.next().to_le_bytes()[0].into(),
            _ => u16::from_le_bytes([rng.next().to_le_bytes()[0], rng.next().to_le_bytes()[1]]),
        }
    };
    for (l, &n) in counts.iter().enumerate() {
        put(a, b, p, u16::try_from(n).unwrap());
        p += if l == 0 { 4 } else { 2 };
        for _ in 0..n {
            for k in 0..3 {
                put(a, b, p + 2 * k, half(rng));
            }
            if l == 0 {
                put(a, b, p + 6, u16::try_from(rng.below(16)).unwrap());
                p += 8;
            } else {
                let w = if rng.below(4) == 0 {
                    half(rng)
                } else {
                    u16::try_from(rng.below(10001)).unwrap()
                };
                put(a, b, p + 6, w);
                put(a, b, p + 8, u16::try_from(rng.below(16)).unwrap());
                p += 10;
            }
        }
    }
    lists
}

fn normal_envelope_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::envelope::bridge as e;
    let regs = a.rebase(e::EE_A0);
    let vu = a.rebase(e::VU_I);
    for n in 0..rounds {
        // stale registers (r0 stays the zero register)
        if n % 2 == 0 {
            for k in 0..(0x6a0 - 0x548) / 4 {
                put(a, b, regs + 4 * k, rng.next());
            }
            for k in 0..(0x850 - 0x784) / 4 {
                put(a, b, vu + 4 * k, modest(rng));
            }
        }
        let m = alloc(a, b, 64);
        for k in 0..16 {
            put(a, b, m + 4 * k, modest(rng));
        }
        let out = alloc(a, b, 16 * 16);
        for k in 0..64 {
            put(a, b, out + 4 * k, modest(rng));
        }
        let lists = normal_lists(a, b, rng);
        twin(
            "sg_calcenvelope_normW",
            a,
            b,
            0x0084_5ec0,
            &[lists, out, m],
            |o| e::calc_envelope_norm_w(o, lists, out, m),
        );
    }
}

#[test]
fn normal_envelope_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e01);
    normal_envelope_cases(&mut a, &mut b, &mut rng, 2000);
}

#[test]
fn normal_envelope_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e02);
    normal_envelope_cases(&mut a, &mut b, &mut rng, 2000);
}

// Main round 13: the screens' camera bytes.
fn screen_camera_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::camera::bridge as c;
    for _ in 0..rounds {
        let screen = if rng.below(4) == 0 {
            rng.next()
        } else {
            rng.below(5)
        };
        let on = rng.next();
        twin(
            "Cam_SetScreenCameraOn",
            a,
            b,
            0x007b_c930,
            &[screen, on],
            |m| {
                c::set_screen_camera_on(m, screen, ffx_sg::lo8(on));
            },
        );
        let cam = rng.next();
        let screen = if rng.below(4) == 0 {
            rng.next()
        } else {
            rng.below(5)
        };
        twin(
            "Cam_SetScreenCamera",
            a,
            b,
            0x007b_c950,
            &[screen, on, cam],
            |m| {
                c::set_screen_camera(m, screen, ffx_sg::lo8(on), ffx_sg::lo8(cam));
            },
        );
    }
}

#[test]
fn screen_camera_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e11);
    screen_camera_cases(&mut a, &mut b, &mut rng, 500);
}

#[test]
fn screen_camera_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e12);
    screen_camera_cases(&mut a, &mut b, &mut rng, 500);
}

// Main round 13: passThisTriangle's table.
fn triangle_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    let pick = |rng: &mut Xorshift, set: &[u32]| {
        if rng.below(5) == 0 {
            rng.next()
        } else {
            set[rng.below(u32::try_from(set.len()).unwrap()) as usize]
        }
    };
    for _ in 0..rounds {
        // mostly the table's own (a, b, c)
        const ROWS: [[u32; 3]; 11] = [
            [2, 2, 0xc],
            [2, 2, 0xd],
            [2, 0, 5],
            [4, 0, 10],
            [7, 0, 6],
            [7, 0, 2],
            [0x29, 0, 6],
            [0x68, 2, 0xc],
            [3, 2, 2],
            [0x67, 1, 1],
            [0x10002, 2, 0xc],
        ];
        let [ka, kb, kc] = if rng.below(8) == 0 {
            [
                pick(rng, &[2, 3, 4, 7, 0x29, 0x67, 0x68]),
                pick(rng, &[0, 1, 2, 3]),
                pick(rng, &[0, 1, 2, 6, 10, 0xc, 0xd]),
            ]
        } else {
            ROWS[rng.below(11) as usize]
        };
        let group = if rng.below(4) == 0 {
            rng.next()
        } else {
            rng.below(22)
        };
        let tag = match rng.below(4) {
            0 => rng.next(),
            1 => (group & 0xffff) * 100 + (kc & 0xffff),
            _ => rng.below(23) * 100 + rng.below(100),
        };
        let lo = |v: u32| ffx_sg::lo16(v);
        let args = [ka, kb, kc, group, tag];
        twin("passThisTriangle", a, b, 0x0082_9870, &args, |_| {
            ffx_sg::triangle::pass_this_triangle(lo(ka), lo(kb), lo(kc), lo(group), lo(tag))
        });
    }
}

#[test]
fn triangle_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e21);
    triangle_cases(&mut a, &mut b, &mut rng, 40000);
}

#[test]
fn triangle_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e22);
    triangle_cases(&mut a, &mut b, &mut rng, 5000);
}

// Main round 13: the push-apart list sorted by z.
fn sort_by_z_cases(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rounds: u32,
    from_game: bool,
) {
    use ffx_sg::state::bridge as s;
    let list = a.rebase(s::PUSH_CHRS);
    let game_n = s::PUSH_CHR_COUNT.get(a);
    for r in 0..rounds {
        let n = if from_game && game_n > 0 && r % 4 == 0 {
            game_n
        } else {
            let n = if rng.below(8) == 0 {
                rng.int()
            } else {
                rng.below(41).cast_signed()
            };
            let chrs = alloc(a, b, 0x20 * 40);
            let zs = [rng.float(), rng.float(), modest(rng)];
            for k in 0..n.clamp(0, 40).cast_unsigned() {
                let chr = chrs + 0x20 * k;
                // repeated values often, to see which equal one wins
                let z = if rng.below(3) == 0 {
                    rng.float()
                } else {
                    zs[rng.below(3) as usize]
                };
                put(a, b, chr + 0x14, z);
                put(a, b, list + 4 * k, chr);
            }
            n.clamp(-5, 40)
        };
        twin(
            "Sg_ChrSortByZ",
            a,
            b,
            0x0083_eb30,
            &[n.cast_unsigned()],
            |m| {
                s::chr_sort_by_z(m, n);
            },
        );
    }
}

#[test]
fn sort_by_z_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e31);
    sort_by_z_cases(&mut a, &mut b, &mut rng, 1500, false);
}

#[test]
fn sort_by_z_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e32);
    sort_by_z_cases(&mut a, &mut b, &mut rng, 1500, true);
}

// Main round 13: SG_SeqFrame over short motion scripts.
fn seq_frame_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::seq::bridge as sq;
    let log = a.rebase(REC_LOG);
    recorder(a, b, 91, sq::YI_ASSERT, 3, 0x5a);
    recorder(a, b, 92, sq::CH_SET_MOTION_SEQ, 5, 0);
    recorder(a, b, 93, sq::DBG_PRINTF, 2, 0x6001);
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        let chr = block(a, b, rng, 0x800);
        // ops until one that stops: 1 (9 bytes), 2 (1), 3 (3), 6 (3) step;
        // 0, 5 and unknown codes stop
        let script = alloc(a, b, 0x80);
        let mut p = 0;
        while p < 0x70 {
            let op = [1u8, 2, 3, 6, 1, 2, 3, 6, 0, 5, 7, 0xff][rng.below(12) as usize];
            put(a, b, script + p, op);
            for k in 1..9 {
                let v = if k % 2 == 1 { rng.below(3) } else { 0 };
                put(a, b, script + p + k, u8::try_from(v).unwrap());
            }
            p += match op {
                1 => 9,
                2 => 1,
                3 | 6 => 3,
                _ => break,
            };
        }
        if p >= 0x70 {
            put(a, b, script + p, 0u8);
        }
        put(a, b, chr + 0x714, script);
        let clips = block(a, b, rng, 0x20);
        let entries = block(a, b, rng, 16 * 4);
        let marks = block(a, b, rng, 2 * 4);
        for k in 0..4 {
            put(a, b, entries + 16 * k + 8, marks);
        }
        put(a, b, clips + 0x10, entries);
        put(a, b, chr + 0x710, clips);
        put(a, b, chr + 0x728, u16::try_from(rng.below(2)).unwrap());
        put(a, b, chr + 0x72a, u16::try_from(rng.below(3)).unwrap());
        put(a, b, chr + 0x737, u8::try_from(rng.below(5)).unwrap());
        // the loop: frame, end, length, loops
        put(a, b, chr + 0x740, rng.int());
        put(a, b, chr + 0x71c, rng.int());
        put(a, b, chr + 0x720, rng.below(0x400));
        put(a, b, chr + 0x724, rng.int());
        twin("SG_SeqFrame", a, b, 0x0083_77c0, &[chr], |m| {
            sq::seq_frame(&mut H(m), chr)
        });
    }
}

#[test]
fn seq_frame_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e41);
    seq_frame_cases(&mut a, &mut b, &mut rng, 1500);
}

#[test]
fn seq_frame_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e42);
    seq_frame_cases(&mut a, &mut b, &mut rng, 1500);
}

// Main round 13: the id triangle under a point.
fn find_triangle_cases(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rounds: u32,
    from_game: bool,
) {
    use ffx_sg::state::bridge as st;
    let game = (
        st::ID_TABLE.get(a),
        st::ID_VERTICES.get(a),
        st::ID_COUNT.get(a),
    );
    for r in 0..rounds {
        let use_game = from_game && game.0 != 0 && game.2 > 0 && r % 2 == 0;
        let (table, verts, n) = if use_game {
            game
        } else {
            let nv = 10u32;
            let verts = alloc(a, b, 8 * nv as usize);
            for k in 0..nv {
                let c = |rng: &mut Xorshift| -> i16 { i16::try_from(rng.below(129)).unwrap() - 64 };
                put(a, b, verts + 8 * k, c(rng));
                put(a, b, verts + 8 * k + 2, c(rng));
                put(a, b, verts + 8 * k + 4, c(rng));
            }
            let n = rng.below(13);
            let table = alloc(a, b, 16 * 13);
            for k in 0..n * 3 {
                put(
                    a,
                    b,
                    table + 16 * (k / 3) + 2 * (k % 3),
                    u16::try_from(rng.below(nv)).unwrap(),
                );
            }
            let table = if rng.below(10) == 0 { 0 } else { table };
            let n = if rng.below(10) == 0 {
                -rng.int().abs()
            } else {
                n.cast_signed()
            };
            (table, verts, n)
        };
        set(a, b, st::ID_TABLE, table);
        set(a, b, st::ID_VERTICES, verts);
        set(a, b, st::ID_COUNT, n);
        // a point near a triangle's middle, or anywhere
        let p = alloc(a, b, 16);
        let (mut cx, mut cy, mut cz) = (0f64, 0f64, 0f64);
        if table != 0 && n > 0 {
            let i = rng.below(n.cast_unsigned());
            for k in 0..3 {
                let v: i16 = Mem::read(a, table + 16 * i + 2 * k);
                let at = verts.wrapping_add((i32::from(v) * 8).cast_unsigned());
                cx += f64::from(Mem::read::<i16>(a, at)) / 3.0;
                cy += f64::from(Mem::read::<i16>(a, at + 2)) / 3.0;
                cz += f64::from(Mem::read::<i16>(a, at + 4)) / 3.0;
            }
        }
        let jitter = |rng: &mut Xorshift| f64::from(rng.below(64)) / 8.0 - 4.0;
        #[allow(clippy::cast_possible_truncation)]
        let mut q = [
            (cx + jitter(rng)) as f32,
            (cy + 4.0 * jitter(rng)) as f32,
            (cz + jitter(rng)) as f32,
        ];
        if rng.below(6) == 0 {
            q = [rng.float(), rng.float(), rng.float()].map(f32::from_bits);
        }
        for (k, v) in (0u32..).zip(q) {
            put(a, b, p + 4 * k, v);
        }
        twin("Sg_IdFindTriangle", a, b, 0x0083_de10, &[p], |m| {
            st::id_find_triangle(m, p)
        });
    }
}

#[test]
fn find_triangle_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e51);
    find_triangle_cases(&mut a, &mut b, &mut rng, 3000, false);
}

#[test]
fn find_triangle_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e52);
    find_triangle_cases(&mut a, &mut b, &mut rng, 3000, true);
}

// Main round 13: Sg_PcRead and Sg_PcReadD over recorded callees.
fn pc_read_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::control::bridge as c;
    let log = a.rebase(REC_LOG);
    let rets = a.rebase(REC_RETS);
    recorder(a, b, 94, c::SG_PC_READ_FILE, 2, 0);
    recorder(a, b, 95, c::SG_MEM_SET_CURRENT_ADDR, 1, 0x77);
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        put(a, b, rets + 4 * 94, rng.next());
        let path = rng.next();
        twin("Sg_PcRead", a, b, 0x0083_ae30, &[path], |m| {
            c::pc_read(&mut H(m), path)
        });
        put(a, b, log, 0u32);
        put(a, b, rets + 4 * 94, rng.next());
        twin("Sg_PcReadD", a, b, 0x0083_ae50, &[path], |m| {
            c::pc_read_d(&mut H(m), path)
        });
    }
}

#[test]
fn pc_read_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e61);
    pc_read_cases(&mut a, &mut b, &mut rng, 200);
}

#[test]
fn pc_read_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e62);
    pc_read_cases(&mut a, &mut b, &mut rng, 200);
}

// Main round 14: the CD read queue (`cd_read_q_clear`,
// `CDfileReadSector_CD`).

/// Busy bytes and ids of the 64 requests, from a few ids so some match.
fn fill_read_queue(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, all_busy: bool) {
    use ffx_sg::cdread::bridge as c;
    let base = c::CHANNELS.ptr(a).addr;
    for i in 0..64 {
        let r = base + 0x38 * i;
        for k in 0..14 {
            put(a, b, r + 4 * k, rng.next());
        }
        let busy = if all_busy || rng.below(3) != 0 {
            rng.next() | 1
        } else {
            0
        };
        put(a, b, r + 0x31, ffx_sg::lo8(busy));
        let id = match rng.below(4) {
            0 => rng.next(),
            _ => rng.below(6),
        };
        put(a, b, r + 0x24, id);
    }
}

fn read_q_clear_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::cdread::bridge as c;
    for _ in 0..rounds {
        fill_read_queue(a, b, rng, false);
        let id = match rng.below(4) {
            0 => rng.next(),
            1 => 0u32.wrapping_sub(rng.below(3)),
            _ => rng.below(6),
        };
        twin("cd_read_q_clear", a, b, 0x0076_e130, &[id], |m| {
            c::cd_read_q_clear(m, id.cast_signed());
        });
    }
}

#[test]
fn read_q_clear_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e71);
    read_q_clear_cases(&mut a, &mut b, &mut rng, 300);
}

#[test]
fn read_q_clear_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e72);
    let id = 0;
    twin(
        "cd_read_q_clear (game)",
        &mut a,
        &mut b,
        0x0076_e130,
        &[id],
        |m| {
            ffx_sg::cdread::bridge::cd_read_q_clear(m, 0);
        },
    );
    read_q_clear_cases(&mut a, &mut b, &mut rng, 100);
}

fn read_sector_cases(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rounds: u32,
    from_game: bool,
) {
    use ffx_sg::cdread::bridge as c;
    let log = a.rebase(REC_LOG);
    recorder(a, b, 96, c::MOVIE_ERR, 3, 0);
    let opts = block(a, b, rng, 0x20);
    for n in 0..rounds {
        put(a, b, log, 0u32);
        if !(from_game && n == 0) {
            let all = rng.below(5) == 0;
            fill_read_queue(a, b, rng, all);
            let id = match rng.below(4) {
                0 => 0xffff - rng.below(2),
                1 => rng.next(),
                _ => rng.below(0x1000),
            };
            set(a, b, c::READ_ID, id.cast_signed());
            set(a, b, c::CHANNEL_KEEP_NAMES, rng.below(2) * rng.next());
        }
        let args = [
            rng.next(),
            rng.next(),
            rng.next(),
            rng.next(),
            if rng.below(2) == 0 { 0 } else { opts },
        ];
        twin("CDfileReadSector_CD", a, b, 0x0076_bdd0, &args, |m| {
            c::cd_file_read_sector(&mut H(m), args[0], args[1], args[2], args[3], args[4])
        });
    }
}

#[test]
fn read_sector_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e73);
    read_sector_cases(&mut a, &mut b, &mut rng, 300, false);
}

#[test]
fn read_sector_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0e74);
    read_sector_cases(&mut a, &mut b, &mut rng, 100, true);
}

// Main round 15: the `sa_decode` LZ decoders (`sa_meth01dec`,
// `sa_meth02dec`).

/// A random code stream of 1..=`codes` codes and the end code; no `0x7f`
/// run of 0 (2^32 bytes). References are mostly long ones early on, so
/// they reach before the output.
#[allow(clippy::cast_possible_truncation)] // bytes of random words
fn lz_stream(rng: &mut Xorshift, codes: u32) -> Vec<u8> {
    let mut s = vec![rng.below(3) as u8, 0, 0, 0, 0];
    for _ in 0..=rng.below(codes) {
        match rng.below(10) {
            0..=2 => {
                let n = rng.below(0x7d) + 1;
                s.push(n as u8);
                s.extend((0..n).map(|_| rng.next() as u8));
            }
            3 => s.extend([0x7e, rng.next() as u8, rng.next() as u8]),
            4 => {
                let [lo, hi] = (rng.below(0x300) as u16 + 1).to_le_bytes();
                s.extend([0x7f, lo, hi, rng.next() as u8]);
            }
            _ => s.extend([(rng.next() | 0x80) as u8, rng.next() as u8]),
        }
    }
    s.push(0);
    s
}

/// Method 1's edge: a reference of the longest distance right at
/// `dst + 0x7fe` (still near) and `dst + 0x7ff` (far: reads `dst - 1`);
/// method 2's at `dst + 0xfff` (near) and `dst + 0x1000`.
#[allow(clippy::cast_possible_truncation)] // bytes of random words
fn lz_edge(method: u8, at: u32) -> Vec<u8> {
    let mut s = vec![method, 0, 0, 0, 0];
    let mut left = at;
    while left > 0 {
        let n = left.min(0xffff);
        let [lo, hi] = (n as u16).to_le_bytes();
        s.extend([0x7f, lo, hi, 0x5a]);
        left -= n;
    }
    // 0xff 0xff: the longest distance (and length) of either method
    s.extend([0xff, 0xff, 0x03, 1, 2, 3, 0]);
    s
}

#[allow(clippy::cast_possible_truncation)] // bytes of random words
fn decode_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::decode::bridge as d;
    let src = alloc(a, b, 0x20000);
    let out = alloc(a, b, 0x80000);
    let size = alloc(a, b, 4);
    let run = |a: &mut Oracle, b: &mut Oracle, s: &[u8], two: bool, junk: &[u8]| {
        a.write(src, s).unwrap();
        b.write(src, s).unwrap();
        // what the game leaves before and in the output
        a.write(out, junk).unwrap();
        b.write(out, junk).unwrap();
        let dst = out + 8;
        let args = [src, dst, size];
        if two {
            twin("sa_meth02dec", a, b, 0x00a4_3cc0, &args, |m| {
                d::sa_meth02dec(m, src, dst, size)
            });
        } else {
            twin("sa_meth01dec", a, b, 0x00a4_3bc0, &args, |m| {
                d::sa_meth01dec(m, src, dst, size)
            });
        }
    };
    for (method, at) in [(1, 0x7fe), (1, 0x7ff), (2, 0xfff), (2, 0x1000)] {
        let junk: Vec<u8> = (0..0x10).map(|_| rng.next() as u8 | 1).collect();
        run(a, b, &lz_edge(method, at), method == 2, &junk);
    }
    for n in 0..rounds {
        let codes = if n % 4 == 0 { 400 } else { 40 };
        let s = lz_stream(rng, codes);
        let junk: Vec<u8> = (0..0x10).map(|_| rng.next() as u8).collect();
        run(a, b, &s, rng.below(2) == 0, &junk);
    }
}

#[test]
fn decode_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f01);
    decode_cases(&mut a, &mut b, &mut rng, 200);
}

#[test]
fn decode_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f02);
    decode_cases(&mut a, &mut b, &mut rng, 60);
}

// Main round 15: the `MsGetSaveCommand(chr, cmd)` accessors of the
// function table at `0x00c410c8`.

const MS_GET_SAVE_COMMAND: u32 = 0x0078_5020;

/// Each accessor against its bridge; `stub` puts a recorder over
/// `MsGetSaveCommand` (it logs both arguments and returns a random word),
/// else both run the original.
fn save_command_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32, stub: bool) {
    let log = a.rebase(REC_LOG);
    let ret = a.rebase(REC_RETS) + 4 * 97;
    if stub {
        recorder(a, b, 97, MS_GET_SAVE_COMMAND, 2, 0);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{sym:#x}: {e}"))
            .eax
    };
    for _ in 0..rounds {
        for &(at, _) in &ffx_sg::savecmd::bridge::SAVE_COMMANDS {
            if stub {
                put(a, b, log, 0u32);
                put(a, b, ret, rng.next());
            }
            // the original indexes the save data by chr: any word only
            // under the recorder
            let chr = if stub && rng.below(4) == 0 {
                rng.next()
            } else {
                rng.below(20)
            };
            twin("KernelSaveCommand", a, b, at, &[chr], |m| {
                ffx_sg::savecmd::bridge::kernel_save_command(m, at, chr, call)
            });
        }
    }
}

#[test]
fn save_command_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f03);
    save_command_cases(&mut a, &mut b, &mut rng, 4, true);
}

#[test]
fn save_command_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f04);
    save_command_cases(&mut a, &mut b, &mut rng, 2, true);
}

#[test]
fn save_command_original_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f05);
    save_command_cases(&mut a, &mut b, &mut rng, 2, false);
}

// Main round 15: the sphere grid draws (`ffx_sg::abmap`).

/// A cdecl logging stand-in: appends the words of the arguments `args`,
/// `len` bytes at argument `ptr` and 0x40 bytes behind each of its pointer
/// fields `fields` to `log + 4 + count * entry` (`count` at `log`); the
/// logged pointer fields are made relative to the struct (frames differ,
/// the layout must not). Returns 0; keeps `esi`/`edi`.
fn deref_logger(log: u32, args: &[u8], ptr: u8, len: u32, fields: &[u8]) -> (Vec<u8>, u32) {
    let fields_len = 0x40 * u32::try_from(fields.len()).unwrap();
    let args_len = 4 * u32::try_from(args.len()).unwrap();
    let entry = args_len + len + fields_len;
    let arg_at = |i: u8| 0x0c + 4 * i; // after the two pushes and the return
    let mut c = vec![0x56, 0x57, 0xa1]; // push esi; push edi; mov eax, [log]
    c.extend(log.to_le_bytes());
    c.extend([0x69, 0xf8]); // imul edi, eax, entry
    c.extend(entry.to_le_bytes());
    c.extend([0x81, 0xc7]); // add edi, log + 4
    c.extend((log + 4).to_le_bytes());
    c.extend([0xff, 0x05]); // inc dword [log]
    c.extend(log.to_le_bytes());
    for &i in args {
        c.extend([0x8b, 0x44, 0x24, arg_at(i), 0xab]); // mov eax, [esp+..]; stosd
    }
    c.extend([0x8b, 0x74, 0x24, arg_at(ptr), 0xb9]); // mov esi, [esp+..]; mov ecx, len/4
    c.extend((len / 4).to_le_bytes());
    c.extend([0xf3, 0xa5]); // rep movsd
    for &f in fields {
        c.extend([0x8b, 0x74, 0x24, arg_at(ptr), 0x8b, 0x76, f]); // mov esi, [[esp+..]+f]
        c.extend([0xb9, 0x10, 0, 0, 0, 0xf3, 0xa5]);
    }
    c.extend([0x81, 0xef]); // sub edi, len + fields: back to the struct's copy
    c.extend((len + fields_len).to_le_bytes());
    for &f in fields {
        // mov eax, [edi+f]; test eax, eax; jz +4; sub eax, [esp+ptr]; mov [edi+f], eax
        c.extend([
            0x8b,
            0x47,
            f,
            0x85,
            0xc0,
            0x74,
            0x04,
            0x2b,
            0x44,
            0x24,
            arg_at(ptr),
        ]);
        c.extend([0x89, 0x47, f]);
    }
    c.extend([0x31, 0xc0, 0x5f, 0x5e, 0xc3]); // xor eax, eax; pop edi; pop esi; ret
    (c, entry)
}

/// Puts `code` in both and jumps `sym` there.
fn redirect_to(a: &mut Oracle, b: &mut Oracle, sym: u32, code: &[u8]) {
    let at = a.alloc_code(code).unwrap();
    assert_eq!(b.alloc_code(code).unwrap(), at);
    for o in [a, b] {
        let from = o.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(at.wrapping_sub(from + 5).to_le_bytes());
        o.write(from, &j).unwrap();
    }
}

/// Random float bits in `[-m, m)`.
fn float_within(rng: &mut Xorshift, m: f32) -> u32 {
    #[allow(clippy::cast_precision_loss)] // a fraction
    let t = rng.below(1 << 20) as f32 / (1 << 20) as f32;
    (m * (2.0 * t - 1.0)).to_bits()
}

/// A sphere grid in `ab`: `nodes` nodes (x, y `i16`, kind `u16 +6`, some
/// 0xffff), 16 kinds with sizes, the cursor near the nodes, its range, a
/// view matrix; the EE zero register and models random.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)] // bytes and halves of random words, small floats
fn abmap_grid(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, ab: u32, nodes: u32) {
    put(a, b, ab + 2, nodes as u16);
    let span = [64, 256, 1024][rng.below(3) as usize];
    for k in 0..nodes {
        let n = ab + 0x808 + 0x28 * k;
        let bytes: Vec<u8> = (0..0x28).map(|_| rng.next() as u8).collect();
        a.write(n, &bytes).unwrap();
        b.write(n, &bytes).unwrap();
        put(a, b, n, (rng.below(2 * span) as i32 - span as i32) as i16);
        put(
            a,
            b,
            n + 2,
            (rng.below(2 * span) as i32 - span as i32) as i16,
        );
        let z = if rng.below(4) == 0 {
            rng.next() as i16 / 64
        } else {
            0
        };
        put(a, b, n + 4, z);
        let kind = if rng.below(6) == 0 {
            0xffff
        } else {
            rng.below(16) as u16
        };
        put(a, b, n + 6, kind);
    }
    for k in 0..16 {
        let size = match rng.below(32) {
            0 => 0x7fc0_0000,
            1 => 0,
            _ => float_within(rng, 2.0),
        };
        put(a, b, ab + 0xf838 + 0x30 * k, size);
        put(a, b, ab + 0xf83c + 0x30 * k, rng.next() as u16);
    }
    for i in 0..3 {
        let m = if i == 2 {
            span as f32 / 8.0
        } else {
            span as f32
        };
        put(a, b, ab + 0x11520 + 4 * i, float_within(rng, m));
    }
    let range = match rng.below(8) {
        0 => 0,
        1 => 0x7fc0_0000,
        _ => (rng.below(span * 2) as f32 + 1.0).to_bits(),
    };
    put(a, b, ab + 0x115a0, range);
    for i in 0..16 {
        put(a, b, ab + 0x113e0 + 4 * i, float_within(rng, 2.0));
    }
    put(a, b, ab + 0x116a4, rng.next());
    let r0 = a.rebase(0x00c8_f508);
    for i in 0..4 {
        put(
            a,
            b,
            r0 + 4 * i,
            if rng.below(2) == 0 { 0 } else { rng.next() },
        );
    }
    for m in [0x0230_57ec, 0x0230_57f4] {
        let at = a.rebase(m);
        put(a, b, at, rng.next());
    }
}

/// Fills the frames in both with the same 16-byte pattern, so the
/// requests' unset fields read alike (both frames are 16-byte aligned).
fn fill_frames(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, work: u32, len: usize) {
    let unit: Vec<u8> = (0..16).map(|_| rng.next().to_le_bytes()[0]).collect();
    let top = a.stack_range().end;
    let pattern: Vec<u8> = unit.iter().copied().cycle().take(0x2f00).collect();
    a.write(top - 0x3000, &pattern).unwrap();
    b.write(work, &pattern[..len]).unwrap();
}

/// One node at the origin (kind 0, size 1) and the cursor at `(x, y, 0)`
/// with range `r`: the halo's rounding and boundary cases.
fn halo_edge(a: &mut Oracle, b: &mut Oracle, ab: u32, r: f32, x: f32, y: f32) {
    put(a, b, ab + 2, 1u16);
    for o in [0u32, 2, 4] {
        put(a, b, ab + 0x808 + o, 0u16);
    }
    put(a, b, ab + 0x808 + 6, 0u16);
    put(a, b, ab + 0xf838, 1f32.to_bits());
    put(a, b, ab + 0x115a0, r.to_bits());
    put(a, b, ab + 0x11520, x.to_bits());
    put(a, b, ab + 0x11524, y.to_bits());
    put(a, b, ab + 0x11528, 0u32);
}

fn node_halo_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as g;
    let log = alloc(a, b, 4 + 0x170 * 30);
    let (stub, _) = deref_logger(log, &[0, 2, 3], 1, 0x50, &[0x20, 0x2c, 0x44]);
    redirect_to(a, b, g::MD_DRAW, &stub);
    let ab = alloc(a, b, 0x1_4000);
    set(a, b, g::ABMAP, ab);
    let work = b.stack_range().start + 0x1000;
    let call = &mut |m: &mut Oracle, f: u32, args: &[u32]| {
        m.call(f, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"))
            .eax
    };
    let mut draws = 0;
    // f32 roundings and bounds: d2, v, dist and 65536 / r rounded; v at
    // 0x1000 exactly; d2 at r * r / 2 exactly
    let edges: [(f32, f32, f32); 6] = [
        (256.0, 59.458_03, 23.68),
        (29.0, 7.25, 0.0),
        (7.0, 1.749_999_9, 0.0),
        (256.0, 16.0, 0.0),
        (16.0, 8.0, 8.0),
        (256.0, 64.0, 0.0),
    ];
    for n in 0..rounds + 6 {
        if let Some(&(r, x, y)) = edges.get(n as usize) {
            abmap_grid(a, b, rng, ab, 1);
            halo_edge(a, b, ab, r, x, y);
        } else {
            let nodes = if n % 8 == 0 { 60 } else { rng.below(20) };
            abmap_grid(a, b, rng, ab, nodes);
        }
        put(a, b, log, 0u32);
        fill_frames(a, b, rng, work, g::NODE_WORK as usize);
        twin("AbmapDrawNodeHalos", a, b, 0x00a5_0290, &[], |m| {
            g::abmap_draw_node_halos(m, work, call);
        });
        draws += Mem::read::<u32>(a, log);
    }
    assert!(draws > rounds, "{draws} draws");
}

#[test]
fn node_halos_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f06);
    node_halo_cases(&mut a, &mut b, &mut rng, 300);
}

#[test]
fn node_halos_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f07);
    node_halo_cases(&mut a, &mut b, &mut rng, 100);
}

#[allow(clippy::cast_possible_truncation)] // bytes and halves of random words
fn node_sphere_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as g;
    let log = alloc(a, b, 4 + 0x170 * 7 * 64);
    let (stub, _) = deref_logger(log, &[0, 2, 3], 1, 0x50, &[0x20, 0x2c, 0x44]);
    redirect_to(a, b, g::MD_DRAW_SPHE, &stub);
    let ab = alloc(a, b, 0x1_4000);
    set(a, b, g::ABMAP, ab);
    let work = b.stack_range().start + 0x1000;
    let call = &mut |m: &mut Oracle, f: u32, args: &[u32]| {
        m.call(f, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"))
            .eax
    };
    let sin1 = a.rebase(0x00c4_4bf4);
    for n in 0..rounds {
        let nodes = if n % 8 == 0 { 60 } else { rng.below(12) };
        abmap_grid(a, b, rng, ab, nodes);
        // part offsets: 0x1000 for some kinds, else small
        for k in 0..16 {
            let rec = ab + 0xf83c + 0x30 * k;
            put(a, b, rec, if rng.below(4) == 0 { 0x1000u16 } else { 0 });
            for i in 0..14 {
                put(a, b, rec + 2 * i, (rng.below(0x200) as i16) - 0x100);
            }
            if rng.below(4) == 0 {
                put(a, b, rec, 0x1000u16);
            }
        }
        put(a, b, ab + 0x115c9, rng.next() as u8);
        put(a, b, ab + 0x11698, rng.next());
        if n == 0 {
            // a sine where 0.15 as f32 and as f64 truncate apart (-3, -2):
            // entry 1 for node 0, every node lit
            put(a, b, sin1, 0x3e55_5555u32);
            put(a, b, ab + 0x11698, 0u32);
            put(a, b, ab + 0x115c9, 0xffu8);
            for k in 0..nodes {
                let node = ab + 0x808 + 0x28 * k;
                put(a, b, node + 0x26, 0x10u16);
                put(a, b, node + 0x21, 0xffu8);
                put(a, b, node + 6, 1u16);
            }
            put(a, b, ab + 0xf83c + 0x30, 0u16);
        }
        put(a, b, log, 0u32);
        fill_frames(a, b, rng, work, g::NODE_WORK as usize);
        twin("AbmapDrawNodeSpheres", a, b, 0x00a5_05e0, &[], |m| {
            g::abmap_draw_node_spheres(m, work, call);
        });
    }
}

#[test]
fn node_spheres_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f08);
    node_sphere_cases(&mut a, &mut b, &mut rng, 200);
}

#[test]
fn node_spheres_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f09);
    node_sphere_cases(&mut a, &mut b, &mut rng, 60);
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // bytes of random words, small floats
fn link_strip_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as g;
    let log = alloc(a, b, 4 + 0x88 * 64);
    let (stub, _) = deref_logger(log, &[1, 2], 0, 0x80, &[]);
    redirect_to(a, b, g::DRAW_UI_ELEMENT, &stub);
    let ab = alloc(a, b, 0x1_4000);
    set(a, b, g::ABMAP, ab);
    let packet = alloc(a, b, 0x160);
    put(a, b, a.rebase(g::STRIP_PACKET), packet);
    let p = alloc(a, b, 0x200);
    let points = alloc(a, b, 0x18 * 10);
    let work = b.stack_range().start + 0x1000;
    let call = &mut |m: &mut Oracle, f: u32, args: &[u32]| {
        m.call(f, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"))
            .eax
    };
    let mut draws = 0;
    for n in 0..rounds {
        let bytes: Vec<u8> = (0..0x200).map(|_| rng.next() as u8).collect();
        a.write(p, &bytes).unwrap();
        b.write(p, &bytes).unwrap();
        if rng.below(2) == 0 {
            // packet byte +0x13: the top byte of p + 0x140
            put(a, b, p + 0x143, 1u8);
        }
        for i in 0..16 {
            put(a, b, ab + 0x113e0 + 4 * i, float_within(rng, 2.0));
            put(a, b, ab + 0x114e0 + 4 * i, float_within(rng, 2.0));
        }
        put(a, b, ab + 0x113e0 + 0xc, float_within(rng, 0.01));
        put(a, b, ab + 0x113e0 + 0x1c, float_within(rng, 0.01));
        put(
            a,
            b,
            ab + 0x113e0 + 0x3c,
            (4.0 + rng.below(4) as f32).to_bits(),
        );
        put(a, b, ab + 0x11350, float_within(rng, 4.0));
        put(a, b, ab + 0x11620, float_within(rng, 2.0));
        put(a, b, a.rebase(g::GS_SINK), rng.next() & !0x100);
        for i in 0..60 {
            put(a, b, points + 4 * i, float_within(rng, 100.0));
        }
        let mut count = 2 + rng.below(5);
        match n % 8 {
            // the first w at or below 0: 0, -1, NaN
            1 => {
                put(a, b, ab + 0x113e0 + 0xc, 0u32);
                put(a, b, ab + 0x113e0 + 0x1c, 0u32);
                let w = [0u32, (-1f32).to_bits(), 0x7fc0_0000][(n / 8 % 3) as usize];
                put(a, b, ab + 0x113e0 + 0x3c, w);
            }
            // a tiny w: offsets past 2^24 (the f32 roundings of the
            // integers) and a divisor where 0.1 as f32 and f64 round apart
            4 => {
                put(a, b, ab + 0x113e0 + 0xc, 0u32);
                put(a, b, ab + 0x113e0 + 0x1c, 0u32);
                put(a, b, ab + 0x113e0 + 0x3c, 1e-3f32.to_bits());
                put(a, b, ab + 0x11350, 0x3204_e52du32);
            }
            // a corner of 2^24 + 1: 2^24 from b (x 524288 through a unit
            // rotation, doubled, at W 1) plus the centre's 1 (x 1/16)
            6 => {
                for i in 0..16 {
                    put(a, b, ab + 0x113e0 + 4 * i, 0u32);
                    put(a, b, ab + 0x114e0 + 4 * i, 0u32);
                }
                put(a, b, ab + 0x113e0 + 0x30, 0.0625f32.to_bits());
                put(a, b, ab + 0x113e0 + 0x3c, 1f32.to_bits());
                put(a, b, ab + 0x114e0, 1f32.to_bits());
                put(a, b, ab + 0x114e0 + 0x14, 1f32.to_bits());
                put(a, b, ab + 0x11350, 1.5f32.to_bits());
                for t in 0..10 {
                    put(a, b, points + 0x18 * t + 8, 524_288f32.to_bits());
                }
                count = 2;
            }
            5 => {
                put(a, b, ab + 0x113e0 + 0xc, 0u32);
                put(a, b, ab + 0x113e0 + 0x1c, 0u32);
                put(a, b, ab + 0x113e0 + 0x3c, 1e-4f32.to_bits());
                put(a, b, ab + 0x11350, (1.0 + rng.below(4) as f32).to_bits());
            }
            // a later one: x of triple 3 far out along a steep w
            2 => {
                put(a, b, ab + 0x113e0 + 0xc, 0.1f32.to_bits());
                put(a, b, ab + 0x113e0 + 0x1c, 0u32);
                put(a, b, ab + 0x113e0 + 0x3c, 1f32.to_bits());
                for t in 0..10 {
                    put(a, b, points + 0x18 * t, (rng.below(5) as f32).to_bits());
                }
                put(a, b, points + 0x18 * 3, (-20f32).to_bits());
                count = 6;
            }
            // n = 1 runs on until a w fails (triple 2 here)
            3 => {
                put(a, b, ab + 0x113e0 + 0xc, 0.1f32.to_bits());
                put(a, b, ab + 0x113e0 + 0x1c, 0u32);
                put(a, b, ab + 0x113e0 + 0x3c, 1f32.to_bits());
                for t in 0..10 {
                    put(a, b, points + 0x18 * t, (rng.below(5) as f32).to_bits());
                }
                put(a, b, points + 0x18 * 2, (-20f32).to_bits());
                count = 1;
            }
            _ => {}
        }
        put(a, b, log, 0u32);
        fill_frames(a, b, rng, work, g::STRIP_WORK as usize);
        twin(
            "AbmapDrawLinkStrip",
            a,
            b,
            0x00a5_21a0,
            &[p, points, count],
            |m| {
                g::abmap_draw_link_strip(m, p, points, count, work, call);
            },
        );
        draws += Mem::read::<u32>(a, log);
    }
    assert!(draws > rounds, "{draws} draws");
}

#[test]
fn link_strip_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f0a);
    link_strip_cases(&mut a, &mut b, &mut rng, 200);
}

#[test]
fn link_strip_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f0b);
    link_strip_cases(&mut a, &mut b, &mut rng, 60);
}

/// The slot name accessors over recorders for both name functions (each
/// logs its id and returns a random word) and a random record table.
fn slot_name_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::savecmd::bridge as s;
    let log = a.rebase(REC_LOG);
    let rets = [a.rebase(REC_RETS) + 4 * 98, a.rebase(REC_RETS) + 4 * 99];
    recorder(a, b, 98, s::ABILITY_NAME, 1, 0);
    recorder(a, b, 99, s::ITEM_NAME, 1, 0);
    let table = alloc(a, b, 0x1d8 * 8);
    set(a, b, Global::<u32>::new(0x0112_a034), table);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{sym:#x}: {e}"))
            .eax
    };
    for _ in 0..rounds {
        let bytes: Vec<u8> = (0..0x1d8 * 8)
            .map(|_| rng.next().to_le_bytes()[0])
            .collect();
        a.write(table, &bytes).unwrap();
        b.write(table, &bytes).unwrap();
        for &(at, _, _) in &s::SLOT_NAMES {
            put(a, b, log, 0u32);
            for r in rets {
                put(a, b, r, rng.next());
            }
            let i = rng.below(8);
            twin("KernelSlotName", a, b, at, &[i], |m| {
                s::kernel_slot_name(m, at, i, call)
            });
        }
    }
}

#[test]
fn save_command_slot_names_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f0c);
    slot_name_cases(&mut a, &mut b, &mut rng, 4);
}

#[test]
fn save_command_slot_names_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f0d);
    slot_name_cases(&mut a, &mut b, &mut rng, 2);
}

// Main round 15: small glue (stored streams, size tables, camera rects,
// cache entries, a task helper).

fn stored_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::decode::bridge as d;
    let buf = alloc(a, b, 0x8000);
    let out = alloc(a, b, 0x4000);
    let size = alloc(a, b, 4);
    for n in 0..rounds {
        let fill: Vec<u8> = (0..0x8000).map(|_| rng.next().to_le_bytes()[0]).collect();
        a.write(buf, &fill).unwrap();
        b.write(buf, &fill).unwrap();
        let len = match n % 4 {
            0 => rng.below(0x3000),
            1 => 0,
            _ => rng.below(0x200),
        };
        // apart, or overlapping either way inside one block
        let (src, dst) = match n % 3 {
            0 => (buf, out),
            1 => (buf + 0x10, buf + 0x10 + 5 + rng.below(0x40)),
            _ => (buf + 0x1000, buf + 0x1000 + 5 - rng.below(0x40)),
        };
        let at = src + 1;
        put(a, b, at, len.to_le_bytes()[0]);
        for (i, byte) in (0u32..).zip(len.to_le_bytes()) {
            put(a, b, at + i, byte);
        }
        twin("sa_meth00dec", a, b, 0x00a4_3da0, &[src, dst, size], |m| {
            d::sa_meth00dec(m, src, dst, size)
        });
    }
}

#[test]
fn glue15_stored_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f0e);
    stored_cases(&mut a, &mut b, &mut rng, 60);
}

#[test]
fn glue15_stored_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f0f);
    stored_cases(&mut a, &mut b, &mut rng, 30);
}

#[allow(clippy::cast_possible_truncation)] // bytes of random words
fn glue15_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::{cache::bridge as c, camera::bridge as cam, cdread::bridge as cd};
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 100, cd::READ_WHOLE_FILE, 2, 0);
    recorder(a, b, 101, cam::MS_CAMERA_GET, 3, 0);
    recorder(a, b, 102, cam::MS_CAMERA_SET_RECT, 3, 0);
    recorder(a, b, 103, c::SG_FREE_A, 2, 0);
    recorder(a, b, 104, c::USER_FREE, 1, 0);
    recorder(a, b, 105, c::TASK_CREATE_IN_GROUP, 4, 0);
    let work = b.stack_range().start + 0x1000;
    let entry = alloc(a, b, 0x20);
    let count = ffx_system::cd::bridge::EX_SIZE_COUNT;
    for n in 0..rounds {
        put(a, b, log, 0u32);
        for k in 100..106 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        set(a, b, count, rng.next().cast_signed() >> (rng.below(32)));
        twin("pc_sizetbl_set_Ex", a, b, 0x00a4_3100, &[], |m| {
            cd::pc_sizetbl_set_ex(&mut H(m))
        });
        for (sym, path) in [
            (0x00a4_3080, cd::MODULE_SIZE_PATH),
            (0x00a4_30c0, cd::SIZETBL_PATH),
        ] {
            twin("pc_sizetbl_set", a, b, sym, &[], |m| {
                cd::pc_sizetbl_set(&mut H(m), path, work)
            });
        }
        twin("Sg_CameraPutRects", a, b, 0x0083_f130, &[], |m| {
            cam::sg_camera_put_rects(&mut H(m))
        });
        let bytes: Vec<u8> = (0..0x20).map(|_| rng.next() as u8).collect();
        a.write(entry, &bytes).unwrap();
        b.write(entry, &bytes).unwrap();
        if n % 2 == 0 {
            put(a, b, entry, c::SG_BLOCK);
        }
        let block: u32 = Mem::read(a, entry + 0xc);
        set(
            a,
            b,
            c::CHR_TIDUS,
            if n % 3 == 0 { block } else { rng.next() },
        );
        twin("Cache_FreeEntry", a, b, 0x0084_0ec0, &[entry], |m| {
            c::cache_free_entry(&mut H(m), entry)
        });
        let args = [rng.next(), rng.next(), rng.next()];
        twin("task_create_group7", a, b, 0x0077_a060, &args, |m| {
            c::task_create_group7(&mut H(m), args[0], args[1], args[2])
        });
        assert_eq!(Mem::read::<u32>(a, log), 8, "calls logged");
    }
}

#[test]
fn glue15_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f10);
    glue15_cases(&mut a, &mut b, &mut rng, 40);
}

#[test]
fn glue15_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f11);
    glue15_cases(&mut a, &mut b, &mut rng, 20);
}

/// The polar camera requests over recorders for the battle camera
/// callees (random answers; the slot table set so the request runs on
/// some calls).
#[allow(clippy::cast_possible_truncation)] // bytes of random words
fn cam_polar_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::camera::bridge as c;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 110, c::ID_CHECK, 1, 0);
    recorder(a, b, 111, c::GET_NUM, 1, 0);
    recorder(a, b, 112, c::SET_CHECK_CHANGE, 4, 0);
    recorder(a, b, 113, c::RECT_TO_POLAR, 5, 0);
    recorder(a, b, 114, c::FOCUS_WEIGHTS, 2, 0);
    let target = block(a, b, rng, 0x10);
    for n in 0..rounds {
        put(a, b, log, 0u32);
        // camera and request bytes past 0x7f now and then (the masks)
        let (cam, sub) = if n % 16 == 15 {
            (0x80 + rng.below(0x80), 0x80 + rng.below(0x80))
        } else {
            (rng.below(4), rng.below(5))
        };
        let id = cam
            | sub << 8
            | if rng.below(4) == 0 {
                rng.next() & !0xffff
            } else {
                0
            };
        let side = rng.next() % 4;
        put(
            a,
            b,
            ret(a, 110),
            if n % 5 == 0 { 0 } else { 1 + rng.below(3) },
        );
        let num = rng.below(4);
        put(a, b, ret(a, 111), num);
        for k in [112, 113, 114] {
            put(a, b, ret(a, k), rng.next());
        }
        // the running slot: this id (mostly), another, or flagged off/done
        let slot = a.rebase(c::CAMERA_SLOTS) + 8 * (cam * 0x1b4 + num);
        let running = if rng.below(3) == 0 { rng.next() } else { id };
        put(a, b, slot, running);
        put(a, b, slot + 4, u8::from(rng.below(4) == 0));
        put(a, b, slot + 5, u8::from(rng.below(4) == 0));
        // the request's set byte for Cam_AddPolar
        let rec =
            a.rebase(c::CAMERA_SIDES) + (side & 1) * 0x61c + cam * 0xda0 + (id >> 8 & 0xff) * 0x130;
        put(a, b, rec + 0xc0, (rng.below(2) as u8) * 3);
        put(a, b, rec + 0xc7, rng.next() as u8 | 1);
        let polar = [rng.float(), rng.float(), rng.float()];
        match n % 3 {
            0 => twin(
                "Cam_SetPolar",
                a,
                b,
                0x007b_f700,
                &[id, side, polar[0], polar[1], polar[2]],
                |m| c::cam_set_polar(&mut H(m), id, side, polar),
            ),
            1 => twin(
                "Cam_AddPolar",
                a,
                b,
                0x007b_f7f0,
                &[id, side, polar[0], polar[1], polar[2]],
                |m| c::cam_add_polar(&mut H(m), id, side, polar),
            ),
            _ => {
                let flags = rng.next();
                let args = [id, side, flags, target, polar[0], polar[1], polar[2]];
                twin("MsCameraSetTargetPolar", a, b, 0x007b_f600, &args, |m| {
                    c::cam_set_target_polar(&mut H(m), id, side, flags, target, polar)
                });
            }
        }
    }
}

#[test]
fn cam_polar_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f12);
    cam_polar_cases(&mut a, &mut b, &mut rng, 300);
}

#[test]
fn cam_polar_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f13);
    cam_polar_cases(&mut a, &mut b, &mut rng, 100);
}

fn light_color_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::light::bridge as l;
    let log = a.rebase(REC_LOG);
    recorder(a, b, 115, l::LIGHT_COLOR_MATRIX, 5, 0);
    recorder(a, b, 116, l::SET_LIGHT_COLOUR, 4, 0);
    let rets = [a.rebase(REC_RETS) + 4 * 115, a.rebase(REC_RETS) + 4 * 116];
    let rgba = alloc(a, b, 0x10);
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for r in rets {
            put(a, b, r, rng.next());
        }
        for i in 0..4 {
            put(a, b, rgba + 4 * i, rng.float());
        }
        let kind = match rng.below(4) {
            0 => rng.next(),
            _ => rng.below(6),
        };
        twin("Sg_LightSetColor", a, b, 0x0083_6310, &[kind, rgba], |m| {
            l::sg_light_set_color(&mut H(m), kind, rgba)
        });
    }
}

#[test]
fn light_color_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f14);
    light_color_cases(&mut a, &mut b, &mut rng, 200);
}

#[test]
fn light_color_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f15);
    light_color_cases(&mut a, &mut b, &mut rng, 60);
}

/// The pad entry draw callbacks over recorders (the 12-word font call
/// through a logging stand-in), the entry's locked byte and kind random.
#[allow(clippy::cast_possible_truncation)] // bytes of random words
fn pad_entry_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as g;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 117, g::ADD_COMMAND_ICON_NAME, 5, 0);
    recorder(a, b, 118, g::HELP_LINE_DRAW, 3, 0);
    recorder(a, b, 119, g::SAVE_PLAYER_PTR, 1, 0);
    recorder(a, b, 120, g::DIGIT_RIGHT, 6, 0);
    let font_log = alloc(a, b, 4 + 0x30 * 8);
    let (stub, _) = deref_logger(font_log, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], 0, 0, &[]);
    redirect_to(a, b, g::ADD_MES_FONT, &stub);
    let ab = alloc(a, b, 0x1_2000);
    set(a, b, g::ABMAP, ab);
    let player = block(a, b, rng, 0x40);
    let entry = alloc(a, b, 0x10);
    let call = &mut |m: &mut Oracle, f: u32, args: &[u32]| {
        m.call(f, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"))
            .eax
    };
    for n in 0..rounds {
        put(a, b, log, 0u32);
        put(a, b, font_log, 0u32);
        for k in [117, 118, 120] {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        let at = ret(a, 119);
        put(a, b, at, player);
        put(a, b, ab + 0x115b9, rng.next() as u8);
        put(a, b, ab + 0x115bc, rng.next() as u8);
        set(a, b, g::PACKET_POS, rng.next());
        set(a, b, g::STAT_LAYOUT, rng.below(2) * rng.next());
        put(a, b, entry, rng.next());
        put(
            a,
            b,
            entry + 4,
            if rng.below(2) == 0 {
                0u8
            } else {
                rng.next() as u8 | 1
            },
        );
        put(
            a,
            b,
            entry + 8,
            if rng.below(4) == 0 {
                rng.next()
            } else {
                rng.below(11)
            },
        );
        let (x, y) = (rng.below(0x400), rng.below(0x400));
        let args = [rng.next(), entry, rng.next(), x, y];
        let locked = Mem::read::<u8>(a, entry + 4) != 0;
        match n % 6 {
            0 | 1 => {
                let sym = if n % 6 == 0 { 0x00a4_f250 } else { 0x00a4_f9a0 };
                if locked {
                    // the caller's eax stays: nothing to compare
                    twin("AbmapPadEntryDrawIcon", a, b, sym, &args, |m| {
                        assert!(g::abmap_pad_entry_draw_icon(m, entry, x, y, call).is_none());
                    });
                } else {
                    twin("AbmapPadEntryDrawIcon", a, b, sym, &args, |m| {
                        g::abmap_pad_entry_draw_icon(m, entry, x, y, call).unwrap()
                    });
                }
            }
            2 => twin("AbmapPadEntryDrawText", a, b, 0x00a4_f9e0, &args, |m| {
                g::abmap_pad_entry_draw_text(m, entry, x, y, call)
            }),
            3 => twin("AbmapPadEntryDrawHelp_e", a, b, 0x00a4_f930, &args, |m| {
                g::abmap_pad_entry_draw_help(m, entry, x, y, 0xe, call)
            }),
            4 => twin("AbmapPadEntryDrawHelp_5", a, b, 0x00a4_fb70, &args, |m| {
                g::abmap_pad_entry_draw_help(m, entry, x, y, 5, call)
            }),
            _ => twin("AbmapPadEntryDrawStat", a, b, 0x00a4_fa30, &args, |m| {
                g::abmap_pad_entry_draw_stat(m, entry, x, y, call)
            }),
        }
    }
}

#[test]
fn pad_entry_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f16);
    pad_entry_cases(&mut a, &mut b, &mut rng, 300);
}

#[test]
fn pad_entry_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f17);
    pad_entry_cases(&mut a, &mut b, &mut rng, 120);
}

/// `Cam_Apply` over recorders (the two pointer callees through a logging
/// stand-in that keeps the value) and `_CIatan` as `-x`.
fn cam_apply_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::camera::bridge as c;
    patch_atan(a);
    patch_atan(b);
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    let ptr_log = alloc(a, b, 4 + 8 * 4);
    for (sym, k) in [(c::SET_FOVY, 0u32), (c::SET_ASPECT, 1)] {
        // each its own log: count, then the value
        let (stub, _) = deref_logger(ptr_log + 0x10 * k, &[], 0, 4, &[]);
        redirect_to(a, b, sym, &stub);
    }
    recorder(a, b, 121, c::MAGIC_ID, 0, 0);
    recorder(a, b, 122, c::SET_WORLD_SUB, 2, 0);
    recorder(a, b, 123, c::SPLIT_START, 1, 0);
    recorder(a, b, 124, c::SPLIT_SET_CAMERA, 1, 0);
    recorder(a, b, 125, c::SPLIT_END, 0, 0);
    recorder(a, b, 126, c::SET_WORLD, 1, 0);
    let work = b.stack_range().start + 0x1000;
    let call = &mut |m: &mut Oracle, f: u32, args: &[u32]| {
        m.call(f, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"))
            .eax
    };
    let atan = |_: &mut Oracle, x: f64| ffx_bridge::x87::store(-x);
    for n in 0..rounds {
        put(a, b, log, 0u32);
        put(a, b, ptr_log, 0u32);
        put(a, b, ptr_log + 0x10, 0u32);
        let cam = match rng.below(8) {
            0 => 0,
            1 => 2,
            2 => 3,
            _ => 1,
        };
        let screen = a.rebase(c::APPLY_SCREENS) + cam * 0x1c0;
        for o in [0u32, 4, 0xc] {
            let v = if rng.below(6) == 0 {
                rng.float()
            } else {
                float_within(rng, 400.0)
            };
            put(a, b, screen + o, v);
        }
        let split = if rng.below(2) == 0 {
            256f32.to_bits()
        } else {
            rng.float()
        };
        put(a, b, screen + 0x78, split);
        set(
            a,
            b,
            c::APPLY_NARROW,
            if rng.below(2) == 0 { 2 } else { rng.below(4) },
        );
        set(
            a,
            b,
            c::APPLY_SUB,
            if rng.below(2) == 0 {
                1
            } else {
                rng.next().to_le_bytes()[0]
            },
        );
        let magic = [0x98, 0x259, rng.below(0x300)][rng.below(3) as usize];
        let at = ret(a, 121);
        put(a, b, at, magic);
        let _ = n;
        twin("Cam_Apply", a, b, 0x007b_c980, &[cam], |m| {
            c::cam_apply(m, cam, work, call, atan);
        });
    }
}

#[test]
fn cam_apply_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f18);
    cam_apply_cases(&mut a, &mut b, &mut rng, 300);
}

#[test]
fn cam_apply_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0f19);
    cam_apply_cases(&mut a, &mut b, &mut rng, 100);
}

/// A stand-in for a function returning a float in `ST0`: `fld dword
/// [slot]; ret`.
fn st0_stub(a: &mut Oracle, b: &mut Oracle, sym: u32, slot: u32) {
    let mut c = vec![0xd9, 0x05];
    c.extend(slot.to_le_bytes());
    c.push(0xc3);
    redirect_to(a, b, sym, &c);
}

/// `Sg_PlayerSetControlAngle` with `Sg_CameraGetYaw` through `fcall` (its
/// `ST0`): a stand-in loading a random yaw, or (`real`) the original on
/// the snapshot's camera.
fn control_angle_cases(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rounds: u32,
    real: bool,
) {
    use ffx_sg::player::bridge as p;
    let slot = alloc(a, b, 4);
    if !real {
        st0_stub(a, b, p::SG_CAMERA_GET_YAW, slot);
    }
    let fcall = &mut |m: &mut Oracle, f: u32, args: &[u32]| {
        let r = m
            .call(f, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"));
        let st0 = r.st0.unwrap_or_else(|| panic!("{f:#x}: no ST0"));
        let pop = m.rebase(POP_STUB);
        m.call(pop, Conv::Cdecl, &[]).unwrap();
        st0
    };
    for _ in 0..rounds {
        let deg = loop {
            let d = if rng.below(3) == 0 {
                rng.float()
            } else {
                float_within(rng, 2000.0)
            };
            // Angle_Wrap steps by 2 pi: no infinity, nothing huge
            if f32::from_bits(d).is_nan() || f32::from_bits(d).abs() < 1.0e6 {
                break d;
            }
        };
        put(a, b, slot, rng.float());
        set(a, b, p::STICK_DIR, f32::from_bits(rng.float()));
        set(a, b, p::DIR_OFFSET, f32::from_bits(rng.float()));
        set(a, b, p::CAMERA_YAW, f32::from_bits(rng.float()));
        twin("Sg_PlayerSetControlAngle", a, b, 0x0082_d950, &[deg], |m| {
            p::set_control_angle(m, f32::from_bits(deg), fcall);
        });
    }
}

#[test]
fn control_angle_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1001);
    control_angle_cases(&mut a, &mut b, &mut rng, 400, false);
}

#[test]
fn control_angle_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1002);
    control_angle_cases(&mut a, &mut b, &mut rng, 40, true);
    control_angle_cases(&mut a, &mut b, &mut rng, 200, false);
}

/// A recording stand-in for a thiscall original without stack
/// arguments (slot `k`): logs `sym` and `ecx` like [`recorder`], returns
/// slot `k`'s answer.
fn this_recorder(a: &mut Oracle, b: &mut Oracle, k: u32, sym: u32, ret: u32) {
    for o in [a, b] {
        let log = o.rebase(REC_LOG);
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], sym
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        let slot = o.rebase(REC_RETS) + 4 * k;
        o.write(slot, &ret.to_le_bytes()).unwrap();
        c.push(0xa1);
        c.extend(slot.to_le_bytes()); // mov eax, [slot]
        c.push(0xc3);
        let code = o.rebase(REC_CODE) + 0x60 * k;
        o.write(code, &c).unwrap();
        let at = o.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        o.write(at, &j).unwrap();
    }
}

/// `FFXApplication::resize` (thiscall) over recorders: the scene's
/// resize (cdecl), then the base class's with `this` in `ecx`, whose
/// `eax` comes back.
fn app_resize_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::app::bridge as app;
    recorder(a, b, 1, app::SCENE_RESIZE, 0, 0);
    this_recorder(a, b, 2, app::APPLICATION_RESIZE, 0);
    let log = a.rebase(REC_LOG);
    let call = &mut |m: &mut Oracle, f: u32, args: &[u32]| {
        m.call(f, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"))
            .eax
    };
    let thiscall = &mut |m: &mut Oracle, f: u32, this: u32, args: &[u32]| {
        m.call(f, Conv::Thiscall(this), args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"))
            .eax
    };
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        let ret = [0, 1, rng.next()][rng.below(3) as usize];
        let slot = a.rebase(REC_RETS) + 4 * 2;
        put(a, b, slot, ret);
        let this = rng.next();
        let at = a.rebase(0x0042_f9a0);
        let r = a
            .call(at, Conv::Thiscall(this), &[])
            .unwrap_or_else(|e| panic!("FFXApplication_resize: {e}"));
        let eax = app::resize(b, this, call, thiscall);
        assert_eq!(r.eax, eax, "FFXApplication_resize: eax");
        assert_eq!(Mem::read::<u32>(&*a, log), 2, "two calls");
        assert_eq!(a.diff(b).unwrap(), [], "FFXApplication_resize: memory");
    }
}

#[test]
fn app_resize_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1003);
    app_resize_cases(&mut a, &mut b, &mut rng, 50);
}

#[test]
fn app_resize_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1004);
    app_resize_cases(&mut a, &mut b, &mut rng, 50);
}

/// `Cam_SetScreenVector` on screens 0..=2 and others, ints of every size.
fn screen_vector_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::camera::bridge as c;
    for _ in 0..rounds {
        let screen = match rng.below(6) {
            0 => 3,
            1 => rng.next(),
            k => k - 2,
        };
        let xyzw: [i32; 4] = std::array::from_fn(|_| {
            if rng.below(3) == 0 {
                rng.next().cast_signed()
            } else {
                rng.int()
            }
        });
        let v = rng.next();
        let args = [
            screen,
            xyzw[0].cast_unsigned(),
            xyzw[1].cast_unsigned(),
            xyzw[2].cast_unsigned(),
            xyzw[3].cast_unsigned(),
            v,
        ];
        let at = a.rebase(0x007b_c890);
        let r = a
            .call(at, Conv::Cdecl, &args)
            .unwrap_or_else(|e| panic!("Cam_SetScreenVector: {e}"));
        let got = c::set_screen_vector(b, screen, xyzw, v);
        if screen <= 2 {
            assert_eq!(r.eax, got, "Cam_SetScreenVector: eax");
        }
        assert_eq!(a.diff(b).unwrap(), [], "Cam_SetScreenVector: memory");
    }
}

#[test]
fn screen_vector_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1005);
    screen_vector_cases(&mut a, &mut b, &mut rng, 400);
}

#[test]
fn screen_vector_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1006);
    screen_vector_cases(&mut a, &mut b, &mut rng, 200);
}

// Main round 16, group 2: the lights from script arguments, the sphere
// grid's language files, the file tables, the VRAM resources.

/// Zeroes the top of both emulated stacks, where the originals' frames go:
/// for locals an original reads without writing (the bridges' are 0).
fn zero_stacks(a: &mut Oracle, b: &mut Oracle) {
    for o in [a, b] {
        let end = o.stack_range().end;
        o.write(end - 0x1000, &[0u8; 0xf00]).unwrap();
    }
}

/// The battle vector code's trigonometry as [`patch_trig`]'s stand-ins
/// (`cos x = x²`, `sin x = 2x`): `f32` bits in and out.
fn vector_trig_stand_in(t: ffx_battle::pos::bridge::Trig, x: u32) -> u32 {
    use ffx_battle::pos::bridge::Trig;
    use ffx_bridge::x87::Reg;
    let v = Reg::load(f32::from_bits(x));
    match t {
        Trig::Cos => (v * v).store().to_bits(),
        Trig::Sin => (v + v).store().to_bits(),
        Trig::Atan => panic!("no arc tangent here"),
    }
}

/// An angle in degrees `Angle_Wrap` ends on soon: mostly within a few
/// turns, sometimes up to 2^17.
fn degrees(rng: &mut Xorshift) -> i32 {
    match rng.below(4) {
        0 => (rng.next() >> 15).cast_signed() - (1 << 16),
        1 => rng.int() >> 14,
        _ => rng.below(2000).cast_signed() - 1000,
    }
}

fn light_set_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::control::bridge as c;
    use ffx_sg::light::bridge as l;
    patch_trig(a);
    patch_trig(b);
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, l::LIGHT_COLOR_MATRIX, 5, 0);
    recorder(a, b, 2, l::SET_LIGHT_COLOUR, 4, 0);
    recorder(a, b, 3, c::VU0_NORMAL_LIGHT_MATRIX, 4, 0);
    recorder(a, b, 4, c::LIGHTING_SET_LIGHT_DIR, 4, 0);
    recorder(a, b, 5, l::LIGHT_SET_LOG, 6, 0);
    let int = |rng: &mut Xorshift| {
        if rng.below(3) == 0 {
            rng.int()
        } else {
            rng.below(256).cast_signed()
        }
    };
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..6 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        zero_stacks(a, b);
        let light = match rng.below(5) {
            0 => rng.next(),
            _ => rng.below(5),
        };
        set(
            a,
            b,
            c::IS_DEBUG,
            [0, 0, 1, rng.next().to_le_bytes()[0]][rng.below(4) as usize],
        );
        let rgb = [int(rng), int(rng), int(rng)];
        let (pitch, yaw) = (degrees(rng), degrees(rng));
        let args = [
            light,
            rgb[0].cast_unsigned(),
            rgb[1].cast_unsigned(),
            rgb[2].cast_unsigned(),
            pitch.cast_unsigned(),
            yaw.cast_unsigned(),
        ];
        twin("Light_Set", a, b, 0x0083_6520, &args, |m| {
            l::light_set(&mut H(m), &vector_trig_stand_in, light, rgb, pitch, yaw)
        });

        put(a, b, log, 0u32);
        let (r, g, bl) = (rng.float(), rng.float(), rng.float());
        let f = f32::from_bits;
        twin(
            "Sg_LightSetColorRGB",
            a,
            b,
            0x0083_6400,
            &[light, r, g, bl],
            |m| l::sg_light_set_color_rgb(&mut H(m), light, f(r), f(g), f(bl)),
        );
    }
}

#[test]
fn light_set_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1007);
    light_set_cases(&mut a, &mut b, &mut rng, 300);
}

#[test]
fn light_set_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1008);
    light_set_cases(&mut a, &mut b, &mut rng, 100);
}

/// `eiAbmSetLang` over recorders (the CD library's two pointers aimed at
/// recorders too); the files are random bytes of random length in the two
/// blocks `user_malloc` hands out.
fn abm_set_lang_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as ab;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, ab::GET_SAVE_CONFIG_ENGLISH, 0, 0);
    // user_malloc: the size into `size_log`, the next block of the list
    let (cursor, list) = (a.rebase(ALLOC_CURSOR), a.rebase(ALLOC_LIST));
    let size_log = alloc(a, b, 4);
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xa3]; // mov eax, [esp+4]; mov [size_log], eax
    c.extend(size_log.to_le_bytes());
    c.push(0xa1); // mov eax, [cursor]
    c.extend(cursor.to_le_bytes());
    c.extend([0x8b, 0x00, 0x83, 0x05]); // mov eax, [eax]; add dword [cursor], 4
    c.extend(cursor.to_le_bytes());
    c.extend([4, 0xc3]);
    redirect_to(a, b, ab::USER_MALLOC, &c);
    recorder(a, b, 3, ab::GET_EXCEL_BIN_SIZE, 2, 0);
    recorder(a, b, 4, ab::USER_FREE, 1, 0);
    recorder(a, b, 5, ab::FLUSH_CACHE, 1, 0);
    // two dead functions as the CD library's stand-ins' homes
    recorder(a, b, 6, 0x0077_a3f0, 1, 0);
    recorder(a, b, 7, 0x0077_a420, 1, 0);
    let code = |a: &Oracle, k: u32| a.rebase(REC_CODE) + 0x60 * k;
    let (read_table, read_sync) = (code(a, 6), code(a, 7));
    set(a, b, ab::CDLIB_READ_TABLE, read_table);
    set(a, b, ab::CDLIB_READ_SYNC, read_sync);
    let bufs = [alloc(a, b, 0x200), alloc(a, b, 0x200)];
    let dst = [alloc(a, b, 0x200), alloc(a, b, 0x200)];
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..8 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        let at = ret(a, 1);
        put(a, b, at, [0, 1, rng.next()][rng.below(3) as usize]);
        put(a, b, cursor, list);
        put(a, b, list, bufs);
        put(a, b, size_log, 0u32);
        let at = ret(a, 3);
        put(a, b, at, rng.below(0x201));
        for buf in bufs {
            for k in 0..0x80 {
                put(a, b, buf + 4 * k, rng.next());
            }
        }
        for (k, (_, ptr)) in ab::LANG_FILES.into_iter().enumerate() {
            let to = if rng.below(2) == 0 {
                dst[k]
            } else {
                dst[1 - k]
            };
            put(a, b, a.rebase(ptr), to);
        }
        twin("eiAbmSetLang", a, b, 0x00a5_4ab0, &[], |m| {
            ab::ei_abm_set_lang(&mut H(m))
        });
    }
}

#[test]
fn abm_set_lang_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1009);
    abm_set_lang_cases(&mut a, &mut b, &mut rng, 100);
}

#[test]
fn abm_set_lang_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_100a);
    abm_set_lang_cases(&mut a, &mut b, &mut rng, 50);
}

/// `pc_fat_index_filename_set` with the file reads recorded and the
/// original `MsSetCountryName`; both HDD modes.
fn fat_index_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::cdread::bridge as cd;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, cd::READ_WHOLE_FILE, 2, 0);
    recorder(a, b, 2, cd::READ_WHOLE_FILE_2, 2, 0);
    let work = b.stack_range().start + 0x1000;
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..3 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        for t in cd::FAT_TABLES {
            put(a, b, a.rebase(t), rng.next());
        }
        let mode = [0, 1, rng.next().to_le_bytes()[0]][rng.below(3) as usize];
        set(a, b, cd::HDD_MODE, mode);
        twin("pc_fat_index_filename_set", a, b, 0x00a4_2fc0, &[], |m| {
            cd::pc_fat_index_filename_set(&mut H(m), work)
        });
    }
}

#[test]
fn fat_index_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_100b);
    fat_index_cases(&mut a, &mut b, &mut rng, 60);
}

#[test]
fn fat_index_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_100c);
    fat_index_cases(&mut a, &mut b, &mut rng, 60);
}

/// `Sg_LoadVramResource` over recorders; the store-image calls log their
/// arguments but the frame's request (its address differs).
fn vram_resource_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::gs::bridge as g;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, g::FLUSH_CACHE, 1, 0);
    recorder(a, b, 2, g::SYNC_PATH, 2, 0);
    recorder(a, b, 3, g::SAVE_VRAM_RESOURCE, 0, 0);
    recorder(a, b, 4, g::DBG_PRINTF, 1, 0);
    let store_log = alloc(a, b, 4 + 0x20 * 4);
    let (set_def, _) = deref_logger(store_log, &[1, 2, 3, 4, 5, 6, 7], 0, 0, &[]);
    redirect_to(a, b, g::SET_DEF_STORE_IMAGE, &set_def);
    let (exec, _) = deref_logger(store_log, &[1], 0, 0, &[]);
    redirect_to(a, b, g::EXEC_STORE_IMAGE, &exec);
    let work = b.stack_range().start + 0x1000;
    let call = &mut |m: &mut Oracle, f: u32, args: &[u32]| {
        m.call(f, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"))
            .eax
    };
    let table = a.rebase(g::VRAM_RESOURCES);
    let back = a.rebase(g::VRAM_READ_BACK);
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        put(a, b, store_log, 0u32);
        for k in 1..5 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        for k in 0..0x14 {
            put(a, b, table + 4 * k, rng.next());
        }
        let first: [u8; 2] = match rng.below(4) {
            0 => [0x29, rng.next().to_le_bytes()[0]],
            1 => [rng.next().to_le_bytes()[0], 0xff],
            2 => rng.next().to_le_bytes()[..2].try_into().unwrap(),
            _ => [0x29, 0xff],
        };
        put(a, b, back, first);
        twin("Sg_LoadVramResource", a, b, 0x0083_fc50, &[], |m| {
            g::load_vram_resource(m, work, call)
        });
    }
}

#[test]
fn vram_resource_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_100d);
    vram_resource_cases(&mut a, &mut b, &mut rng, 60);
}

#[test]
fn vram_resource_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_100e);
    vram_resource_cases(&mut a, &mut b, &mut rng, 60);
}

/// `Sg_CameraSetPlayer` over recorders (`Ch_GetFieldMode`, the battle
/// camera calls of `Sg_CameraPutRects`) with the trigonometry stand-ins;
/// the player none or a block with a random position.
fn camera_set_player_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::camera::bridge as cam;
    use ffx_sg::state::bridge as st;
    patch_trig(a);
    patch_trig(b);
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, st::CH_GET_FIELD_MODE, 1, 0);
    recorder(a, b, 2, cam::MS_CAMERA_GET, 3, 0);
    recorder(a, b, 3, cam::MS_CAMERA_SET_RECT, 3, 0);
    let chr = alloc(a, b, 0x200);
    let f = |rng: &mut Xorshift| {
        if rng.below(5) == 0 {
            rng.float()
        } else {
            float_within(rng, 500.0)
        }
    };
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..4 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        for k in 0..3 {
            let v = f(rng);
            put(a, b, chr + ffx_chara::layout::POS + 4 * k, v);
            let v = f(rng);
            put(a, b, a.rebase(st::FREE_CAM_OFFSET) + 4 * k, v);
            let v = f(rng);
            put(a, b, a.rebase(cam::CAMERA_REF.sym) + 4 * k, v);
        }
        set(a, b, st::PLAYER, if rng.below(4) == 0 { 0 } else { chr });
        let angle = |rng: &mut Xorshift| match rng.below(4) {
            0 => rng.float(),
            1 => [0x3fc9_0fd8, 0xbfc9_0fd8, 0x3fc9_0fd9, 0xbfc9_0fd9][rng.below(4) as usize],
            _ => float_within(rng, 3.0),
        };
        set(a, b, st::FREE_CAM_YAW, f32::from_bits(angle(rng)));
        set(a, b, st::FREE_CAM_PITCH, f32::from_bits(angle(rng)));
        let dist = |rng: &mut Xorshift| match rng.below(4) {
            0 => rng.float(),
            1 => float_within(rng, 2.0),
            _ => float_within(rng, 300.0),
        };
        set(a, b, st::FREE_CAM_DIST, f32::from_bits(dist(rng)));
        set(a, b, st::FREE_CAM_DIST_NOW, f32::from_bits(dist(rng)));
        twin("Sg_CameraSetPlayer", a, b, 0x0083_f2e0, &[], |m| {
            st::camera_set_player(&mut H(m), &vector_trig_stand_in)
        });
    }
}

#[test]
fn camera_set_player_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_100f);
    camera_set_player_cases(&mut a, &mut b, &mut rng, 400);
}

#[test]
fn camera_set_player_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1010);
    camera_set_player_cases(&mut a, &mut b, &mut rng, 150);
}

/// `Sg_EffectReadPoll` over recorders whose answers pick the branches:
/// the async load 0, 1 or other, each check 0 or not, the PRX error 0, 1,
/// 2 or negative; both reads idle, running or fresh.
fn effect_read_poll_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    let callees = [
        st::ASYNC_LOAD_AVAILABLE,
        st::GETU32_01940AB4,
        st::PRX_READY,
        st::PRX_ERROR,
        st::EFFECT_READ_ERROR,
        st::DBG_PRINTF,
        st::IS_LOAD_TEXTURE,
        st::EFFECT_READ_COMPLETE,
        st::MAGIC_FILE_LOADED,
        st::EVENT_EFFECT_READ_COMPLETE,
        st::EFFECT_READ_STAT_INIT,
    ];
    for (k, sym) in (1..).zip(callees) {
        recorder(a, b, k, sym, u32::from(sym == st::DBG_PRINTF), 0);
    }
    let flag = |rng: &mut Xorshift| match rng.below(3) {
        0 => 0,
        1 => 1,
        _ => rng.next(),
    };
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..=11 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        for k in [1, 2, 3, 7, 9] {
            let at = ret(a, k);
            put(a, b, at, flag(rng));
        }
        let at = ret(a, 4);
        put(a, b, at, [0, 1, 2, 3, u32::MAX][rng.below(5) as usize]);
        let frames = |rng: &mut Xorshift| match rng.below(4) {
            0 => -1,
            1 => 0,
            2 => i32::MAX,
            _ => rng.int(),
        };
        set(a, b, st::EFFECT_READ_FRAMES, frames(rng));
        set(a, b, st::EVENT_READ_FRAMES, frames(rng));
        set(a, b, st::EFFECT_PRELOADED, flag(rng));
        set(a, b, st::EVENT_PRELOADED, flag(rng));
        twin("Sg_EffectReadPoll", a, b, 0x0082_2250, &[], |m| {
            st::effect_read_poll(&mut H(m))
        });
    }
}

#[test]
fn effect_read_poll_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1011);
    effect_read_poll_cases(&mut a, &mut b, &mut rng, 800);
}

#[test]
fn effect_read_poll_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1012);
    effect_read_poll_cases(&mut a, &mut b, &mut rng, 300);
}

/// A `sprintf` stand-in that knows `%s` only (every `%` takes the next
/// argument as a string): enough for the path builders.
const SPRINTF_S: [u8; 54] = [
    0x56, 0x57, 0x53, // push esi; push edi; push ebx
    0x8b, 0x7c, 0x24, 0x10, // mov edi, [esp+0x10] (dst)
    0x8b, 0x74, 0x24, 0x14, // mov esi, [esp+0x14] (format)
    0x8d, 0x5c, 0x24, 0x18, // lea ebx, [esp+0x18] (arguments)
    0xac, // loop: lodsb
    0x3c, 0x25, // cmp al, '%'
    0x75, 0x12, // jne store
    0xac, // lodsb (the conversion)
    0x56, // push esi
    0x8b, 0x33, // mov esi, [ebx]
    0x83, 0xc3, 0x04, // add ebx, 4
    0xac, // copy: lodsb
    0x84, 0xc0, // test al, al
    0x74, 0x03, // jz end
    0xaa, // stosb
    0xeb, 0xf8, // jmp copy
    0x5e, // end: pop esi
    0xeb, 0xe9, // jmp loop
    0xaa, // store: stosb
    0x84, 0xc0, // test al, al
    0x75, 0xe4, // jnz loop
    0x89, 0xf8, // mov eax, edi
    0x2b, 0x44, 0x24, 0x10, // sub eax, [esp+0x10]
    0x48, // dec eax (the length)
    0x5b, 0x5f, 0x5e, 0xc3, // pop ebx; pop edi; pop esi; ret
];

/// A cdecl stand-in that appends 0x40 bytes behind each of its first `n`
/// arguments (strings) to `log + 4 + count * 0x40 n` (`count` at `log`)
/// and answers the word at `ret`.
fn string_logger(log: u32, n: u8, ret: u32) -> Vec<u8> {
    let mut c = vec![0x56, 0x57, 0xa1]; // push esi; push edi; mov eax, [log]
    c.extend(log.to_le_bytes());
    c.extend([0x69, 0xf8]); // imul edi, eax, 0x40 n
    c.extend((0x40 * u32::from(n)).to_le_bytes());
    c.extend([0x81, 0xc7]); // add edi, log + 4
    c.extend((log + 4).to_le_bytes());
    c.extend([0xff, 0x05]); // inc dword [log]
    c.extend(log.to_le_bytes());
    for i in 0..n {
        c.extend([0x8b, 0x74, 0x24, 0x0c + 4 * i]); // mov esi, [esp+..]
        c.extend([0xb9, 0x10, 0, 0, 0, 0xf3, 0xa5]); // mov ecx, 16; rep movsd
    }
    c.push(0xa1); // mov eax, [ret]
    c.extend(ret.to_le_bytes());
    c.extend([0x5f, 0x5e, 0xc3]); // pop edi; pop esi; ret
    c
}

/// `Sg_MapInit` with `sprintf` doing `%s`, the path users logging their
/// strings, the rest recorded; every selection, the shade flag, debug
/// lines and id reads that fail or not.
fn map_init_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, st::YI_RESET, 0, 0);
    recorder(a, b, 2, st::ID_FIXUP_AND_SET, 1, 0);
    let strings = [
        alloc(a, b, 4 + 0x40 * 4),
        alloc(a, b, 4 + 0x80 * 2),
        alloc(a, b, 4 + 0xc0 * 2),
    ];
    let pc_read = ret(a, 3);
    let id_reads = alloc(a, b, 4);
    let ids = [alloc(a, b, 4), alloc(a, b, 4)];
    // Sg_PcRead: the path logged, the answers of a list in turn
    let mut c = string_logger(strings[0], 1, pc_read);
    c.truncate(c.len() - 8);
    c.push(0xa1); // mov eax, [id_reads]
    c.extend(id_reads.to_le_bytes());
    c.extend([0x8b, 0x00, 0x83, 0x05]); // mov eax, [eax]; add dword [id_reads], 4
    c.extend(id_reads.to_le_bytes());
    c.extend([4, 0x5f, 0x5e, 0xc3]);
    redirect_to(a, b, st::PC_READ, &c);
    let setup = string_logger(strings[1], 2, ret(a, 4));
    redirect_to(a, b, st::YI_MAP_SETUP, &setup);
    let printf = string_logger(strings[2], 3, ret(a, 5));
    redirect_to(a, b, st::DBG_PRINTF, &printf);
    let sprintf = a.alloc_code(&SPRINTF_S).unwrap();
    assert_eq!(b.alloc_code(&SPRINTF_S).unwrap(), sprintf);
    set(a, b, st::SPRINTF_SLOT, sprintf);
    let work = b.stack_range().start + 0x1000;
    let name = |rng: &mut Xorshift| -> Vec<u8> {
        let n = rng.below(9) as usize;
        let mut v: Vec<u8> = (0..n)
            .map(|_| b'a' + rng.below(26).to_le_bytes()[0])
            .collect();
        v.resize(0x40, 0);
        v
    };
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for log in strings {
            put(a, b, log, 0u32);
        }
        for k in 1..6 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        zero_stacks(a, b);
        let select = [0, 1, 1, 2, 3][rng.below(5) as usize];
        set(a, b, st::MAP_DIR_SELECT, select);
        set(a, b, st::MAP_SHADE, u8::from(rng.below(2) == 0));
        for base in [a.rebase(st::MAP_DIRS), a.rebase(st::MAP_NAMES.sym)] {
            let v = name(rng);
            for (k, byte) in (0u32..).zip(v) {
                put(a, b, base + 0x40 * select + k, byte);
            }
        }
        put(a, b, a.rebase(st::DEBUG_LINE), 0xffu8);
        let answers = [[0, 1, rng.next()][rng.below(3) as usize], rng.next()];
        put(a, b, id_reads, ids[0]);
        put(a, b, ids[0], answers[0]);
        put(a, b, ids[0] + 4, answers[1]);
        let _ = ids[1];
        // the frames start zeroed on both sides: the loggers copy whole
        // buffers, the bytes after each string too
        b.write(work, &[0u8; 0xc0]).unwrap();
        twin("Sg_MapInit", a, b, 0x0084_4d10, &[], |m| {
            st::map_init(&mut H(m), work);
        });
    }
}

#[test]
fn map_init_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1013);
    map_init_cases(&mut a, &mut b, &mut rng, 200);
}

#[test]
fn map_init_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1014);
    map_init_cases(&mut a, &mut b, &mut rng, 100);
}

/// `Sg_CanSpeedUp` over recorders answering values from its lists (magic
/// ids, level hashes, flags, maps), so every test is reached.
fn can_speed_up_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    let callees = [
        st::MAGIC_ID,
        st::LEVEL_NAME_HASH,
        st::GET_SCENERIO_FLAG,
        st::BATTLE_CHECK,
        st::VIDEO_PLAYING,
        st::MOVIE_HAVE_CAMERA,
        st::GET_UC_STATUS,
        st::GET_INTER_VALEFOR,
        st::WINDOWS_BUSY,
        st::IS_SPEEDING_UP,
    ];
    for (k, sym) in (1..).zip(callees) {
        recorder(a, b, k, sym, 0, 0);
    }
    let pick = |rng: &mut Xorshift, list: &[u32]| {
        if rng.below(4) == 0 {
            rng.next()
        } else {
            list[rng.below(u32::try_from(list.len()).unwrap()) as usize]
        }
    };
    let rare =
        |rng: &mut Xorshift| u32::from(rng.below(6) == 0) * [1, rng.next()][rng.below(2) as usize];
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        let magic = pick(
            rng,
            &[
                0x14a,
                0x14c,
                0x17d,
                0x132,
                0xa9,
                0x1b5,
                0x1b6,
                0xcd,
                u32::MAX,
                0,
                0x14b,
            ],
        );
        let hash = pick(
            rng,
            &[
                0x1cd6, 0x56d, 0x24f9, 0x1b4f, 0x7b4, 0x1c99, 0x1ca, 0x6e1, 0x1b84, 0x1f7, 0x1899,
                0x1d21, 0xc04, 0x1e48, 0xbae, 0x1c45, 0,
            ],
        );
        let flag = pick(rng, &[0x5be, 0x2de, 0x309, 0xc3f, 0xb68, 0x1_05be, 0]);
        let battle = [0, 1, rng.next()][rng.below(3) as usize];
        let valefor = [0, 4, 0x84, 0x80, rng.next()][rng.below(5) as usize];
        let busy = [2, 0, rng.next()][rng.below(3) as usize];
        let answers = [
            magic,
            hash,
            flag,
            battle,
            rare(rng),
            rare(rng),
            u32::from(rng.below(2) == 0),
            valefor,
            busy,
            u32::from(rng.below(8) == 0),
        ];
        for (k, v) in (1..).zip(answers) {
            let at = ret(a, k);
            put(a, b, at, v);
        }
        set(a, b, st::WAIT_BY_TIME, rare(rng));
        set(
            a,
            b,
            st::CURRENT_MAP,
            pick(rng, &[0xd0, 0xa4, 0x5f, 0x78, 0x7f, 0x3b, 0]),
        );
        set(a, b, st::MOVIE_FADING, rare(rng));
        for g in st::SPEED_UP_BLOCKS {
            put(a, b, a.rebase(g), u32::from(rng.below(10) == 0));
        }
        twin("Sg_CanSpeedUp", a, b, 0x0081_fd30, &[], |m| {
            st::can_speed_up(&mut H(m))
        });
    }
}

#[test]
fn can_speed_up_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1015);
    can_speed_up_cases(&mut a, &mut b, &mut rng, 20000);
}

#[test]
fn can_speed_up_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1016);
    can_speed_up_cases(&mut a, &mut b, &mut rng, 5000);
}

/// A cdecl stand-in that appends its arguments to `log + 4 + count *
/// entry` (`count` at `log`): for each `(i, len)` argument `i` itself
/// (`len` 0) or `len` bytes behind it; then runs `tail` (which sees the
/// arguments at `[esp + 4 + 4 i]` and must return).
fn arg_logger(log: u32, args: &[(u8, u32)], tail: &[u8]) -> Vec<u8> {
    let entry: u32 = args
        .iter()
        .map(|&(_, len)| if len == 0 { 4 } else { len })
        .sum();
    let mut c = vec![0x56, 0x57, 0xa1]; // push esi; push edi; mov eax, [log]
    c.extend(log.to_le_bytes());
    c.extend([0x69, 0xf8]); // imul edi, eax, entry
    c.extend(entry.to_le_bytes());
    c.extend([0x81, 0xc7]); // add edi, log + 4
    c.extend((log + 4).to_le_bytes());
    c.extend([0xff, 0x05]); // inc dword [log]
    c.extend(log.to_le_bytes());
    for &(i, len) in args {
        if len == 0 {
            c.extend([0x8b, 0x44, 0x24, 0x0c + 4 * i, 0xab]); // mov eax, [esp+..]; stosd
        } else {
            c.extend([0x8b, 0x74, 0x24, 0x0c + 4 * i, 0xb9]); // mov esi, [esp+..]; mov ecx, len/4
            c.extend((len / 4).to_le_bytes());
            c.extend([0xf3, 0xa5]); // rep movsd
        }
    }
    c.extend([0x5f, 0x5e]); // pop edi; pop esi
    c.extend(tail);
    c
}

/// `mov eax, [slot]; ret`.
fn answer(slot: u32) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend(slot.to_le_bytes());
    c.push(0xc3);
    c
}

/// `Sg_IdChrMove` over logging stand-ins: the walk flips a bit of the
/// point's x and sets the triangle from a slot, the slide flips one of
/// its z, the height comes from a slot through `ST0`.
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn id_chr_move_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, st::GET_IGNORE_ID, 0, 0);
    recorder(a, b, 2, st::ID_GET_NORMAL, 2, 0);
    let logs: Vec<u32> = (0..4).map(|_| alloc(a, b, 4 + 0x20 * 8)).collect();
    let (find, walk, walk_tri, height) = (ret(a, 3), ret(a, 4), ret(a, 5), ret(a, 6));
    let c = arg_logger(logs[0], &[(0, 12)], &answer(find));
    redirect_to(a, b, st::ID_FIND_TRIANGLE, &c);
    let mut tail = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
    tail.extend([0x81, 0x30, 0, 0, 0x40, 0]); // xor dword [eax], 0x400000
    tail.extend([0x8b, 0x4c, 0x24, 0x04, 0x66, 0x8b, 0x15]); // mov ecx, [esp+4]; mov dx, [walk_tri]
    tail.extend(walk_tri.to_le_bytes());
    tail.extend([0x66, 0x89, 0x91, 0x24, 0x08, 0, 0]); // mov [ecx+0x824], dx
    tail.extend(answer(walk));
    let c = arg_logger(logs[1], &[(0, 0), (1, 12), (2, 0)], &tail);
    redirect_to(a, b, st::ID_WALK, &c);
    let mut tail = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
    tail.extend([0x81, 0x70, 0x08, 0, 0, 0x40, 0]); // xor dword [eax+8], 0x400000
    tail.extend([0x31, 0xc0, 0xc3]); // xor eax, eax; ret
    let c = arg_logger(logs[2], &[(0, 0), (1, 12)], &tail);
    redirect_to(a, b, st::ID_SLIDE_EDGE, &c);
    let mut tail = vec![0xd9, 0x05]; // fld dword [height]
    tail.extend(height.to_le_bytes());
    tail.push(0xc3);
    let c = arg_logger(logs[3], &[(0, 12), (1, 0)], &tail);
    redirect_to(a, b, st::ID_GET_HEIGHT, &c);
    let fcall = &mut |m: &mut H, f: u32, args: &[u32]| {
        let r =
            m.0.call(f, Conv::Cdecl, args)
                .unwrap_or_else(|e| panic!("{f:#x}: {e}"));
        let st0 = r.st0.unwrap_or_else(|| panic!("{f:#x}: no ST0"));
        let pop = m.0.rebase(POP_STUB);
        m.0.call(pop, Conv::Cdecl, &[]).unwrap();
        st0
    };
    let chr = alloc(a, b, 0x900);
    let table = alloc(a, b, 0x10 * 0x20);
    let point = b.stack_range().start + 0x1000;
    let f = |rng: &mut Xorshift| {
        if rng.below(6) == 0 {
            rng.float()
        } else {
            float_within(rng, 300.0)
        }
    };
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for l in &logs {
            put(a, b, *l, 0u32);
        }
        zero_stacks(a, b);
        b.write(point, &[0u8; 16]).unwrap();
        for k in 1..7 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        let tri = |rng: &mut Xorshift| {
            [0xffff, rng.below(0x20), rng.next() & 0xffff][rng.below(3) as usize]
        };
        put(a, b, ret(a, 1), u32::from(rng.below(3) == 0));
        put(a, b, find, tri(rng));
        put(a, b, walk, [0, 1, rng.next()][rng.below(3) as usize]);
        put(a, b, walk_tri, tri(rng));
        put(a, b, height, f(rng));
        for o in [0xc, 0x10, 0x14, 0x4c, 0x54] {
            let v = f(rng);
            put(a, b, chr + o, v);
        }
        put(
            a,
            b,
            chr,
            [1u16, 0, 0x101, rng.next().to_le_bytes()[0].into()][rng.below(4) as usize],
        );
        set(a, b, st::ID_WALK_STEP, rng.next());
        put(
            a,
            b,
            chr + 0x194,
            if rng.below(2) == 0 {
                0x200
            } else {
                rng.next() & !0x200
            },
        );
        put(a, b, chr + st::CHR_TRIANGLE, tri(rng));
        for k in 0..0x80 {
            put(a, b, table + 4 * k, rng.next());
        }
        set(
            a,
            b,
            st::ID_TABLE,
            if rng.below(6) == 0 { 0 } else { table },
        );
        set(a, b, st::ID_ZOOM, f32::from_bits(f(rng)));
        set(a, b, st::ID_REACH, f32::from_bits(f(rng)));
        set(
            a,
            b,
            st::GRAPHIC_MAP_ID,
            if rng.below(2) == 0 {
                0x12a
            } else {
                rng.below(0x200)
            },
        );
        twin("Sg_IdChrMove", a, b, 0x0083_e5f0, &[chr], |m| {
            st::id_chr_move(&mut H(m), chr, point, fcall);
        });
    }
}

#[test]
fn id_chr_move_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1017);
    id_chr_move_cases(&mut a, &mut b, &mut rng, 2000);
}

#[test]
fn id_chr_move_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1018);
    id_chr_move_cases(&mut a, &mut b, &mut rng, 500);
}

/// `SG_UpdateMotion` over recorders: raw clips (bit 31 or not), sequences
/// with and without the kept frame rate, both `apply`s, the half speed,
/// blend counters around 1, weapons of the five kinds and others.
fn update_motion_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::seq::bridge as sq;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    for (k, (sym, n)) in (1..).zip([
        (sq::MOTION_LOOP_FRAME, 1),
        (sq::APPLY_RAW_CLIP, 4),
        (sq::GET_KEEP_FPS, 0),
        (sq::SEQ_FRAME, 1),
        (sq::SAMPLE_CHANNELS, 1),
        (sq::BLEND_TO_TARGET, 2),
        (sq::APPLY_BONES, 1),
    ]) {
        recorder(a, b, k, sym, n, 0);
    }
    let chr = alloc(a, b, 0x900);
    let owner = alloc(a, b, 0x400);
    let skins = [alloc(a, b, 0x600), alloc(a, b, 0x1100)];
    let clip = alloc(a, b, 8);
    let small = |rng: &mut Xorshift| -> i16 {
        match rng.below(4) {
            0 => rng.next().to_le_bytes()[0].into(),
            1 => -i16::from(rng.next().to_le_bytes()[0]),
            _ => i16::from_le_bytes(rng.next().to_le_bytes()[..2].try_into().unwrap()),
        }
    };
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..8 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        put(a, b, ret(a, 3), u32::from(rng.below(2) == 0));
        let mut flags = rng.next() & !(0x40 | 0x8000_0000 | 0x10_0000);
        if rng.below(2) == 0 {
            flags |= 0x40;
        }
        if rng.below(2) == 0 {
            flags |= 0x8000_0000;
        }
        if rng.below(2) == 0 {
            flags |= 0x10_0000;
        }
        put(a, b, chr + 0x194, flags);
        put(a, b, chr + 0x800, if rng.below(8) == 0 { 0 } else { clip });
        let len = 1 + i32::from(rng.next().to_le_bytes()[0]) + 0x100 * i32::from(rng.below(2) == 0);
        put(a, b, clip, i16::try_from(len).unwrap());
        put(a, b, clip + 4, rng.next());
        let speed = small(rng);
        let half = rng.below(2) == 0;
        set(a, b, sq::MOTION_HALF, u32::from(half));
        let moved = if half {
            i32::from(speed) / 2
        } else {
            i32::from(speed)
        };
        // the time after the move: anywhere, a multiple of the length, or -1
        let t = match rng.below(4) {
            0 => (len << 8) * rng.below(4).cast_signed(),
            1 => -1,
            _ => rng.int() << rng.below(10),
        };
        put(a, b, chr + 0x740, t.wrapping_sub(moved));
        put(a, b, chr + 0x714, u32::from(rng.below(5) != 0));
        put(a, b, chr + 0x750, small(rng));
        put(a, b, chr + 0x754, speed);
        put(
            a,
            b,
            chr + 0x752,
            [0i16, 1, 2, 3, small(rng)][rng.below(5) as usize],
        );
        put(a, b, chr + 0x7e0, rng.int());
        set(a, b, sq::KEEP_FPS_SCALE, rng.below(0x200));
        put(
            a,
            b,
            chr,
            [
                0x403du16,
                0x4041,
                0x403c,
                0x4042,
                0x403f,
                rng.next().to_le_bytes()[0].into(),
            ][rng.below(6) as usize],
        );
        put(a, b, chr + 0x19c, if rng.below(4) == 0 { 0 } else { owner });
        put(a, b, owner + 0x328, skins[1]);
        put(a, b, chr + 0x328, skins[0]);
        for k in 0..0x440 {
            put(a, b, skins[1] + 4 * k, rng.next());
        }
        let apply = [0, 1, rng.next()][rng.below(3) as usize];
        twin("SG_UpdateMotion", a, b, 0x0083_8c90, &[chr, apply], |m| {
            sq::update_motion(&mut H(m), chr, apply);
        });
    }
}

#[test]
fn update_motion_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1019);
    update_motion_cases(&mut a, &mut b, &mut rng, 1500);
}

#[test]
fn update_motion_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_101a);
    update_motion_cases(&mut a, &mut b, &mut rng, 400);
}

// Main round 17: the sphere grid's stat bonuses.

/// `eiAbmParaGet` over stand-ins: the init, save map, command and
/// save-param calls recorded; `MsGetAbilityMap(chr)` the 16-byte record
/// `outs + 16 * chr`; `MsGetExcelData(id, table, _)` the 32-byte record
/// `recs + 32 * id`, the table it was handed into `seen`. A save map of
/// 0x401 entries with a few activated nodes (the log holds 95 calls), the
/// one past the end activated for everyone.
fn abm_para_get_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as ab;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, ab::INIT_CHR_ABILITY_MAP, 0, 0);
    recorder(a, b, 2, ab::GET_SAVE_ABILITY_MAP, 0, 0);
    recorder(a, b, 3, ab::ABILITY_MAP_SET_COMMAND, 2, 0);
    recorder(a, b, 4, ab::SET_SAVE_PARAM_ALL, 0, 0);
    let outs = alloc(a, b, 16 * 8);
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc1, 0xe0, 0x04, 0x05]; // mov eax, [esp+4]; shl eax, 4; add eax, outs
    c.extend(outs.to_le_bytes());
    c.push(0xc3);
    redirect_to(a, b, ab::GET_ABILITY_MAP, &c);
    let recs = alloc(a, b, 32 * 256);
    let seen = alloc(a, b, 4);
    let mut c = vec![0x8b, 0x4c, 0x24, 0x08, 0x89, 0x0d]; // mov ecx, [esp+8]; mov [seen], ecx
    c.extend(seen.to_le_bytes());
    c.extend([0x8b, 0x44, 0x24, 0x04, 0xc1, 0xe0, 0x05, 0x05]); // mov eax, [esp+4]; shl eax, 5; add eax, recs
    c.extend(recs.to_le_bytes());
    c.push(0xc3);
    redirect_to(a, b, ab::GET_EXCEL_DATA, &c);
    let save = alloc(a, b, 2 * 0x401);
    let strings = b.stack_range().start + 0x1000;
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..5 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        let at = ret(a, 2);
        put(a, b, at, save);
        put(a, b, seen, 0u32);
        set(a, b, ab::NODE_KINDS, rng.next());
        for k in 0..32 * 2 {
            put(a, b, outs + 4 * k, rng.next());
        }
        let amount = [0x55u8, 0x80, 0x33, 1, 0xff, 0][rng.below(8) as usize % 6];
        let fixed = rng.below(2) == 0;
        for k in 0..32 * 256 / 4 {
            put(a, b, recs + 4 * k, rng.next());
        }
        for id in 0..256 {
            let r = recs + 32 * id;
            if fixed {
                put(a, b, r + 0x14, amount);
            }
            if rng.below(3) == 0 {
                put(a, b, r + 0x10, 0xffffu16);
            }
        }
        for k in 0..0x401 {
            let kind = rng.next().to_le_bytes()[0];
            put(a, b, save + 2 * k, [kind, 0]);
        }
        put(a, b, save + 2 * 0x400 + 1, 0xffu8);
        for _ in 0..rng.below(11) {
            let node = match rng.below(4) {
                0 => 0,
                1 => 0x3ff,
                _ => rng.below(0x400),
            };
            let bits = match rng.below(3) {
                0 => 0xff,
                1 => 1 << rng.below(8),
                _ => rng.next().to_le_bytes()[0],
            };
            put(a, b, save + 2 * node + 1, bits);
        }
        twin("eiAbmParaGet", a, b, 0x00a5_4860, &[], |m| {
            ab::ei_abm_para_get(&mut H(m), strings)
        });
    }
}

#[test]
fn abm_para_get_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1101);
    abm_para_get_cases(&mut a, &mut b, &mut rng, 150);
}

#[test]
fn abm_para_get_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1102);
    abm_para_get_cases(&mut a, &mut b, &mut rng, 60);
}

/// `eiAbmParaGet` with the game's own callees on the snapshot's save data
/// (every character's real grid).
#[test]
fn abm_para_get_snapshot_real() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let strings = b.stack_range().start + 0x1000;
    twin("eiAbmParaGet", &mut a, &mut b, 0x00a5_4860, &[], |m| {
        ffx_sg::abmap::bridge::ei_abm_para_get(&mut H(m), strings)
    });
}

/// A stand-in that logs through recorder `k` (its `n` arguments passed
/// on), then runs `tail` (which must return).
fn logged(a: &mut Oracle, b: &mut Oracle, k: u32, sym: u32, n: u8, tail: &[u8]) {
    recorder(a, b, k, sym, n.into(), 0);
    let rec = a.rebase(REC_CODE) + 0x60 * k;
    let mut c = vec![];
    for _ in 0..n {
        c.extend([0xff, 0x74, 0x24, 4 * n]); // push dword [esp + 4n]
    }
    c.push(0xb8);
    c.extend(rec.to_le_bytes()); // mov eax, rec
    c.extend([0xff, 0xd0, 0x83, 0xc4, 4 * n]); // call eax; add esp, 4n
    c.extend(tail);
    redirect_to(a, b, sym, &c);
}

/// [`logged`] answering `[fail]` while `[count]` is not 0 (counting it
/// down) and `[ok]` after; each `poke` first stores the byte at `.0`
/// into `.1`.
fn countdown(
    a: &mut Oracle,
    b: &mut Oracle,
    k: u32,
    sym: u32,
    n: u8,
    [count, fail, ok]: [u32; 3],
    poke: &[(u32, u32)],
) {
    let mut c = vec![];
    for &(from, to) in poke {
        c.push(0xa0);
        c.extend(from.to_le_bytes()); // mov al, [from]
        c.push(0xa2);
        c.extend(to.to_le_bytes()); // mov [to], al
    }
    c.push(0xa1);
    c.extend(count.to_le_bytes()); // mov eax, [count]
    c.extend([0x85, 0xc0, 0x74, 0x0c, 0xff, 0x0d]); // test eax, eax; jz ok; dec dword [count]
    c.extend(count.to_le_bytes());
    c.extend(answer(fail));
    c.extend(answer(ok));
    logged(a, b, k, sym, n, &c);
}

/// `cd_read_thread_sector` over stand-ins: seek, HDD step and read count
/// down to their answer; the step pokes request 63's busy byte from a
/// slot; zero, one or more busy requests.
fn cd_read_thread_sector_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::cdread::bridge as cd;
    let log = a.rebase(REC_LOG);
    let slots = alloc(a, b, 4 * 12);
    let chans = cd::CHANNELS.ptr(a).addr;
    let last = chans + 63 * 0x38 + 0x31;
    let ctl = alloc(a, b, 0x80);
    // the step also notes the first byte of `cd + 0x74`
    countdown(a, b, 1, cd::CD_SEEK, 1, [slots, slots + 4, slots + 8], &[]);
    countdown(
        a,
        b,
        2,
        cd::HDD_READ_STEP,
        0,
        [slots + 12, slots + 16, slots + 20],
        &[(slots + 36, last), (ctl + 0x74, slots + 40)],
    );
    countdown(
        a,
        b,
        3,
        cd::CD_READ,
        0,
        [slots + 24, slots + 28, slots + 32],
        &[],
    );
    recorder(a, b, 4, cd::TASK_WAIT, 1, 0);
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        put(a, b, slots, rng.below(19));
        put(a, b, slots + 4, [0, 2, rng.next()][rng.below(3) as usize]);
        put(a, b, slots + 8, 1u32);
        put(a, b, slots + 12, rng.below(6));
        put(
            a,
            b,
            slots + 16,
            [1, u32::MAX, rng.next() | 1][rng.below(3) as usize],
        );
        put(a, b, slots + 20, 0u32);
        put(a, b, slots + 24, rng.below(6));
        put(a, b, slots + 28, 0u32);
        put(a, b, slots + 32, [1, rng.next() | 1][rng.below(2) as usize]);
        put(
            a,
            b,
            slots + 36,
            [0u8, 1, 0, rng.next().to_le_bytes()[0]][rng.below(4) as usize],
        );
        for k in 0..0x20 {
            put(a, b, ctl + 4 * k, rng.next());
        }
        let n = rng.below(64);
        let req = chans + 0x38 * n;
        for k in 0..64 * 0x38 / 4 {
            put(a, b, chans + 4 * k, rng.next());
        }
        let busy = rng.below(5);
        for k in 0..64 {
            let on = match busy {
                0 => 0,
                1 | 2 => u8::from(k == n),
                3 => u8::from(k == (n + 1) % 64),
                _ => u8::from(rng.below(16) == 0),
            };
            put(
                a,
                b,
                chans + 0x38 * k + 0x31,
                on * (1 + rng.next().to_le_bytes()[0] % 255),
            );
        }
        set(
            a,
            b,
            cd::HDD_MODE,
            [0, 0, 0, 1, rng.next().to_le_bytes()[0]][rng.below(5) as usize],
        );
        twin(
            "cd_read_thread_sector",
            a,
            b,
            0x0076_e4b0,
            &[ctl, req, n],
            |m| {
                cd::cd_read_thread_sector(&mut H(m), ctl, req, n);
            },
        );
    }
}

#[test]
fn cd_read_thread_sector_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1103);
    cd_read_thread_sector_cases(&mut a, &mut b, &mut rng, 800);
}

#[test]
fn cd_read_thread_sector_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1104);
    cd_read_thread_sector_cases(&mut a, &mut b, &mut rng, 300);
}

/// `cd_read_thread_file` over stand-ins: `sprintf` doing `%s`, the C
/// runtime's `memcpy`, the rest logged; the flush ticks the clock;
/// `task_wait` clears the background busy byte after a count. Sizes
/// around sector multiples, packed or not, stored (first byte 0) or not,
/// an HDD copy or not, either decoder.
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn cd_read_thread_file_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::cdread::bridge as cd;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    let clock = alloc(a, b, 0x20);
    let tick = clock + 0x1c;
    // the find notes `cd + 0x5c`, `+0x70`, `+0x74` as it sees them
    let ctl = alloc(a, b, 0x80);
    let seen = alloc(a, b, 12);
    let mut c = vec![];
    for (k, off) in (0u32..).zip([0x5c, 0x70, 0x74]) {
        c.push(0xa1);
        c.extend((ctl + off).to_le_bytes()); // mov eax, [ctl + off]
        c.push(0xa3);
        c.extend((seen + 4 * k).to_le_bytes()); // mov [seen + 4k], eax
    }
    c.extend(answer(ret(a, 1)));
    logged(a, b, 1, cd::HDD_INSTALL_FIND, 2, &c);
    recorder(a, b, 2, cd::HD_FILE_READ, 4, 0);
    recorder(a, b, 3, cd::GET_HDD_ERR, 0, 0);
    recorder(a, b, 4, cd::TK_REFRESH_HDD_MODE, 0, 0);
    recorder(a, b, 5, cd::SA_CD_READ_TASK, 5, 0);
    // the flush ticks the clock and moves the request's archive, sector
    // and buffer (read again after each call)
    let req_at = alloc(a, b, 4);
    let mut c = vec![0xff, 0x05]; // inc dword [tick]
    c.extend(tick.to_le_bytes());
    c.push(0xa1);
    c.extend(req_at.to_le_bytes()); // mov eax, [req_at]
    // inc dword [eax+0x28]; inc dword [eax+0x1c]; add dword [eax+4], 4
    c.extend([
        0xff, 0x40, 0x28, 0xff, 0x40, 0x1c, 0x83, 0x40, 0x04, 0x04, 0xc3,
    ]);
    logged(a, b, 6, cd::FLUSH_CACHE, 1, &c);
    recorder(a, b, 7, cd::TK_CONFIG, 0, 0);
    recorder(a, b, 8, cd::TK_SIGNAL_BG, 0, 0);
    recorder(a, b, 9, cd::DECODE_DATA, 1, 0);
    recorder(a, b, 10, cd::READ_CALLBACK, 1, 0);
    let waits = alloc(a, b, 4);
    let busy = a.rebase(cd::BG_BUSY.sym);
    let mut c = vec![0xa1];
    c.extend(waits.to_le_bytes()); // mov eax, [waits]
    c.extend([0x85, 0xc0, 0x75, 0x08, 0xc6, 0x05]); // test eax, eax; jnz more; mov byte [busy], 0
    c.extend(busy.to_le_bytes());
    c.extend([0, 0xc3, 0xff, 0x0d]); // ret; more: dec dword [waits]
    c.extend(waits.to_le_bytes());
    c.push(0xc3);
    logged(a, b, 11, cd::TASK_WAIT, 1, &c);
    let sprintf = a.alloc_code(&SPRINTF_S).unwrap();
    assert_eq!(b.alloc_code(&SPRINTF_S).unwrap(), sprintf);
    set(a, b, cd::SPRINTF_SLOT, sprintf);
    set(a, b, cd::CLOCK, clock);
    let entry = alloc(a, b, 0x40);
    let config = alloc(a, b, 0x20);
    let sector_buf = alloc(a, b, 0x800);
    let buffer = alloc(a, b, 0x5000);
    let chans = cd::CHANNELS.ptr(a).addr;
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..12 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        let at = ret(a, 1);
        put(a, b, at, if rng.below(3) == 0 { entry } else { 0 });
        let at = ret(a, 3);
        put(a, b, at, [0, 0, 1, rng.next()][rng.below(4) as usize]);
        let at = ret(a, 7);
        put(a, b, at, config);
        put(a, b, config + 0x10, u8::from(rng.below(2) == 0));
        for k in 0..0x10 {
            put(a, b, entry + 4 * k, rng.next());
        }
        let name: Vec<u8> = (0..rng.below(12))
            .map(|_| b'a' + rng.below(26).to_le_bytes()[0])
            .chain([0])
            .collect();
        a.write(entry + 0x24, &name).unwrap();
        b.write(entry + 0x24, &name).unwrap();
        for k in 0..0x200 {
            put(a, b, sector_buf + 4 * k, rng.next());
        }
        for k in 0..0x1400 {
            put(a, b, buffer + 4 * k, rng.next());
        }
        // the first byte read: stored (0) or packed
        let first =
            |rng: &mut Xorshift| [0, 0, 1, rng.next().to_le_bytes()[0]][rng.below(4) as usize];
        put(a, b, sector_buf, first(rng));
        // the flush moves the buffer on by 4 each time
        let byte = first(rng);
        for k in 0..4 {
            put(a, b, buffer + 4 * k, byte);
        }
        set(a, b, cd::READ_LAST_SRC, rng.next());
        set(a, b, cd::READ_BUFFER, sector_buf);
        for k in 0..0x20 {
            put(a, b, ctl + 4 * k, rng.next());
        }
        put(
            a,
            b,
            ctl + 0x70,
            [0, 0, 0, 1, rng.next()][rng.below(5) as usize],
        );
        put(a, b, tick, rng.next());
        put(a, b, waits, rng.below(4));
        set(
            a,
            b,
            cd::BG_BUSY,
            [0, 1, rng.next().to_le_bytes()[0]][rng.below(3) as usize],
        );
        set(a, b, cd::HDD_MODE, rng.next().to_le_bytes()[0]);
        let n = rng.below(64);
        let req = chans + 0x38 * n;
        for k in 0..0x38 / 4 {
            put(a, b, req + 4 * k, rng.next());
        }
        let size = match rng.below(5) {
            0 => rng.below(0x800),
            1 => 0x800 * rng.below(5),
            2 => 0x800 * rng.below(5) + [1, 0x7fb, 0x7ff][rng.below(3) as usize],
            _ => rng.below(0x4000),
        };
        let mut stored = match rng.below(5) {
            0 => size,
            1 => size.wrapping_sub(1),
            2 => [
                0x800 * rng.below(5) + rng.below(3),
                0x800 * (rng.below(4) + 1) - 6 - rng.below(3),
                0x800 * rng.below(5) + 0x3fb,
            ][rng.below(3) as usize],
            _ => rng.below(size.max(1)),
        };
        // a stored file ending within 5 bytes past a sector copies a
        // negative length: the original faults (the data never does it)
        if stored + 5 >= 0x800 && (stored + 5) & 0x7ff < 5 {
            stored -= 5;
        }
        // a packed size past `i32::MAX`: one sector read (its first byte
        // not 0, else the copy runs away)
        let wide = rng.below(16) == 0;
        let size = if wide {
            stored = 0x7fff_fffb;
            put(a, b, sector_buf, 1u8);
            u32::MAX - rng.below(4)
        } else {
            size
        };
        put(a, b, req, stored);
        put(a, b, req + 4, buffer);
        put(a, b, req + 0x2c, size);
        put(a, b, req + 0x1c, rng.below(0x10_0000));
        put(
            a,
            b,
            req + 0x20,
            if rng.below(2) == 0 { 0 } else { rng.next() },
        );
        put(
            a,
            b,
            req + 0x34,
            [0u8, 1, rng.next().to_le_bytes()[0]][rng.below(3) as usize],
        );
        if wide {
            put(a, b, req + 0x34, 1u8);
        }
        put(a, b, req_at, req);
        twin(
            "cd_read_thread_file",
            a,
            b,
            0x0076_e1c0,
            &[ctl, req, n],
            |m| {
                cd::cd_read_thread_file(&mut H(m), ctl, req, n);
            },
        );
    }
}

#[test]
fn cd_read_thread_file_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1105);
    cd_read_thread_file_cases(&mut a, &mut b, &mut rng, 800);
}

#[test]
fn cd_read_thread_file_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1106);
    cd_read_thread_file_cases(&mut a, &mut b, &mut rng, 300);
}

/// The CD-mode read thread `0x0076e780` (an endless loop) over stand-ins,
/// N passes. `task_wait` counts every call down and, once the count is
/// spent, leaves the original's frame (`add esp, 8; pop edi; pop esi;
/// pop ebx; ret`) when called from the loop's own `task_wait(1)` (its
/// return address): the original stops after N passes with its registers
/// restored, the port side runs the pass and the wait until the same
/// count is spent. The two handlers log and release the request (the
/// file one only when a slot says so), `sceCdRead` counts down to 1.
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn cd_read_thread_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::cdread::bridge as cd;
    let log = a.rebase(REC_LOG);
    let slots = alloc(a, b, 0x20);
    let (waits, read_count, read_fail, read_ok, release) =
        (slots, slots + 4, slots + 8, slots + 12, slots + 16);
    // mov eax, [esp + 8]; and byte [eax + 0x31], 0
    logged(
        a,
        b,
        1,
        cd::CD_READ_THREAD_SECTOR,
        3,
        &[0x8b, 0x44, 0x24, 0x08, 0x80, 0x60, 0x31, 0x00, 0xc3],
    );
    // mov cl, [release]; mov eax, [esp + 8]; and [eax + 0x31], cl
    let mut c = vec![0x8a, 0x0d];
    c.extend(release.to_le_bytes());
    c.extend([0x8b, 0x44, 0x24, 0x08, 0x20, 0x48, 0x31, 0xc3]);
    logged(a, b, 2, cd::CD_READ_THREAD_FILE, 3, &c);
    countdown(
        a,
        b,
        3,
        cd::CD_READ,
        0,
        [read_count, read_fail, read_ok],
        &[],
    );
    // dec dword [waits]; jg back; cmp dword [esp], ret; jne back;
    // add esp, 8; pop edi; pop esi; pop ebx; back: ret
    let wait_ret = a.rebase(cd::CD_READ_THREAD_WAIT_RET);
    let mut c = vec![0xff, 0x0d];
    c.extend(waits.to_le_bytes());
    c.extend([0x7f, 0x10, 0x81, 0x3c, 0x24]);
    c.extend(wait_ret.to_le_bytes());
    c.extend([0x75, 0x07, 0x83, 0xc4, 0x08, 0x5f, 0x5e, 0x5b, 0xc3, 0xc3]);
    logged(a, b, 4, cd::TASK_WAIT, 1, &c);
    put(a, b, read_fail, 0u32);
    put(a, b, read_ok, 1u32);
    let lib = ffx_system::cd::bridge::CD_LIB.ptr(a).addr;
    let chans = cd::CHANNELS.ptr(a).addr;
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        let passes = 1 + rng.below(4);
        put(a, b, waits, passes);
        put(a, b, read_count, rng.below(4));
        put(a, b, release, [0u8, 0xff][rng.below(2) as usize]);
        for k in 0..0x24 {
            put(a, b, lib + 0x50 + 4 * k, rng.next());
        }
        let hdd_state = ffx_system::cd::bridge::HDD_STATE;
        set(a, b, hdd_state, [rng.next(), rng.next()]);
        let reading = match rng.below(4) {
            0 => u32::MAX,
            1 => rng.below(64),
            2 => 0,
            _ => 63,
        };
        put(a, b, lib + 0x74, reading);
        set(a, b, cd::CHANNEL_KEEP_NAMES, u32::from(rng.below(2) == 0));
        for k in 0..64 * 0x38 / 4 {
            put(a, b, chans + 4 * k, rng.next());
        }
        let density = [0, 1, 4, 16, 64][rng.below(5) as usize];
        for k in 0..64 {
            let r = chans + 0x38 * k;
            let busy = density != 0 && rng.below(64) < density;
            put(
                a,
                b,
                r + 0x31,
                if busy {
                    1 + rng.next().to_le_bytes()[0] % 255
                } else {
                    0
                },
            );
            let kind = [0u8, 0, 1, 1, 2, 0xff, rng.next().to_le_bytes()[0]][rng.below(7) as usize];
            put(a, b, r + 0x30, kind);
            let cancel = if rng.below(3) == 0 {
                1 + rng.next().to_le_bytes()[0] % 255
            } else {
                0
            };
            put(a, b, r + 0x33, cancel);
        }
        // one cancelled request alone, the one being read: the cancel
        // finds the queue empty
        if rng.below(4) == 0 {
            for k in 0..64 {
                put(a, b, chans + 0x38 * k + 0x31, 0u8);
            }
            let n = rng.below(64);
            let r = chans + 0x38 * n;
            put(a, b, r + 0x31, 1 + rng.next().to_le_bytes()[0] % 255);
            put(a, b, r + 0x30, 0u8);
            put(a, b, r + 0x33, 1u8);
            put(a, b, lib + 0x74, n);
        }
        twin("cd_read_thread", a, b, 0x0076_e780, &[], |m| {
            let mut h = H(m);
            let at = h.rebase(cd::TASK_WAIT);
            let lib = ffx_system::cd::bridge::get_cd(&h);
            loop {
                cd::cd_read_thread_pass(&mut h, lib);
                ffx_bridge::Host::call(&mut h, at, &[1]);
                if h.read::<i32>(waits) <= 0 {
                    break;
                }
            }
        });
    }
}

#[test]
fn cd_read_thread_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1801);
    cd_read_thread_cases(&mut a, &mut b, &mut rng, 400);
}

#[test]
fn cd_read_thread_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1802);
    cd_read_thread_cases(&mut a, &mut b, &mut rng, 150);
}

/// Puts `code` in both; its address.
fn code_in_both(a: &mut Oracle, b: &mut Oracle, code: &[u8]) -> u32 {
    let at = a.alloc_code(code).unwrap();
    assert_eq!(b.alloc_code(code).unwrap(), at);
    at
}

/// `eiAbmDataInit` over stand-ins: `sprintf` and `sprintf_s` doing `%s`,
/// `fiosUnifyFilename` dropping the first byte, the game's `memset`, the
/// file calls logging
/// the path's bytes (not the frame address) and answering from lists
/// (some files missing), `user_aligned_malloc` handing out a cursor, the
/// CD library's three calls logging and the read filling the buffer
/// from a template (kind header 0x31 or not, counts up to the frame's
/// end), `eiAbmParaGet` recorded with an answer; every grid, loaded or
/// not.
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn abm_data_init_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as ab;
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    let log = a.rebase(REC_LOG);
    let n = ab::DATA_FILES;
    let logs = [
        alloc(a, b, (4 + 0x84 * n) as usize),
        alloc(a, b, (4 + 12 * 2 * n) as usize),
        alloc(a, b, (4 + 8 * n) as usize),
        alloc(a, b, (4 + 12 * n) as usize),
        alloc(a, b, (4 + 0x84 * n) as usize),
        alloc(a, b, 4 + 12),
        alloc(a, b, (4 + 0x84 * n) as usize),
        alloc(a, b, (4 + 4 * n) as usize),
    ];
    let answers = alloc(a, b, (4 * n) as usize);
    let (opened, cursor) = (alloc(a, b, 4), alloc(a, b, 4));
    let template = alloc(a, b, ab::DATA_INIT_UNIFIED as usize);
    let sprintf = code_in_both(a, b, &SPRINTF_S);
    let mut sprintf_s = SPRINTF_S;
    sprintf_s[10] = 0x18;
    sprintf_s[14] = 0x1c;
    let sprintf_s = code_in_both(a, b, &sprintf_s);
    // its size logged first (mov eax, sprintf_s; jmp eax)
    let mut tail = vec![0xb8];
    tail.extend(sprintf_s.to_le_bytes());
    tail.extend([0xff, 0xe0]);
    let sprintf_s = code_in_both(a, b, &arg_logger(logs[7], &[(1, 0)], &tail));
    set(a, b, ab::SPRINTF_SLOT, sprintf);
    set(a, b, ab::SPRINTF_S_SLOT, sprintf_s);
    // fiosUnifyFilename: the source and size logged, the source but its
    // first byte copied out (push esi; push edi; mov esi, [esp + 0xc];
    // mov edi, [esp + 0x10]; inc esi; copy: lodsb; stosb; test al, al;
    // jnz copy; pop edi; pop esi; ret)
    let tail = [
        0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0x7c, 0x24, 0x10, 0x46, 0xac, 0xaa, 0x84, 0xc0,
        0x75, 0xfa, 0x5f, 0x5e, 0xc3,
    ];
    redirect_to(
        a,
        b,
        ab::FIOS_UNIFY,
        &arg_logger(logs[6], &[(0, 0x80), (2, 0)], &tail),
    );
    // fiosOpen: the path, the mode; the next answer of the list
    let mut tail = vec![0xa1];
    tail.extend(opened.to_le_bytes()); // mov eax, [opened]
    tail.extend([0x8b, 0x00, 0x83, 0x05]); // mov eax, [eax]; add dword [opened], 4
    tail.extend(opened.to_le_bytes());
    tail.extend([4, 0xc3]);
    redirect_to(
        a,
        b,
        ab::FIOS_OPEN,
        &arg_logger(logs[0], &[(0, 0x80), (1, 0)], &tail),
    );
    // fiosLseek: the file's low 14 bits as its size
    let tail = [0x8b, 0x44, 0x24, 0x04, 0x25, 0xff, 0x3f, 0, 0, 0xc3];
    redirect_to(
        a,
        b,
        ab::FIOS_LSEEK,
        &arg_logger(logs[1], &[(0, 0), (1, 0), (2, 0)], &tail),
    );
    // user_aligned_malloc: the cursor, moved on by 0x40
    let mut tail = vec![0xa1];
    tail.extend(cursor.to_le_bytes()); // mov eax, [cursor]
    tail.extend([0x83, 0x05]); // add dword [cursor], 0x40
    tail.extend(cursor.to_le_bytes());
    tail.extend([0x40, 0xc3]);
    redirect_to(
        a,
        b,
        ab::ALIGNED_MALLOC,
        &arg_logger(logs[2], &[(0, 0), (1, 0)], &tail),
    );
    redirect_to(
        a,
        b,
        ab::FIOS_READ,
        &arg_logger(logs[3], &[(0, 0), (1, 0), (2, 0)], &answer(ret(a, 1))),
    );
    recorder(a, b, 2, ab::FIOS_CLOSE, 1, 0);
    redirect_to(
        a,
        b,
        ab::DBG_PRINTF,
        &arg_logger(logs[4], &[(0, 0), (1, 0x80)], &answer(ret(a, 3))),
    );
    recorder(a, b, 4, ab::EI_ABM_PARA_GET, 0, 0);
    // the CD library's read: table and the two zeros logged, the template
    // copied to the buffer (push esi; push edi; mov edi, [esp + 0x10];
    // mov esi, template; mov ecx, n; rep movsd; pop edi; pop esi; ret)
    let mut tail = vec![0x56, 0x57, 0x8b, 0x7c, 0x24, 0x10, 0xbe];
    tail.extend(template.to_le_bytes());
    tail.push(0xb9);
    tail.extend((ab::DATA_INIT_UNIFIED / 4).to_le_bytes());
    tail.extend([0xf3, 0xa5, 0x5f, 0x5e, 0xc3]);
    let read = code_in_both(a, b, &arg_logger(logs[5], &[(0, 0), (2, 0), (3, 0)], &tail));
    set(a, b, ab::CDLIB_READ, read);
    // the table and the wait: logged through recorders 5 and 6, called
    // by pointer (the recorder's jump sits on a free code byte range)
    for (k, slot, sym) in [
        (5, ab::CDLIB_READ_TABLE, 0x0270_8000u32),
        (6, ab::CDLIB_READ_SYNC, 0x0270_8100),
    ] {
        recorder(a, b, k, sym, 1, 0);
        let at = a.rebase(sym);
        set(a, b, slot, at);
    }
    let menu = ffx_battle::save::bridge::MENU;
    let save = alloc(a, b, 0x1400);
    let lists = alloc(a, b, 0x40 * 21);
    let active: Vec<Vec<u32>> = ab::ACTIVE_LISTS
        .iter()
        .map(|&t| (0..7).map(|k| Mem::read(a, a.rebase(t) + 4 * k)).collect())
        .collect();
    let starts: Vec<Vec<u16>> = ab::START_NODES
        .iter()
        .map(|&t| (0..7).map(|k| Mem::read(a, a.rebase(t) + 2 * k)).collect())
        .collect();
    let work = b.stack_range().start + 0x1000;
    let block_words: Vec<u32> = (0..n)
        .map(|i| ab::DATA_BLOCKS - 4 * i)
        .chain(ab::DATA_COPIES.iter().flat_map(|(at, r)| {
            let len = r.end() - r.start() + 1;
            (0..len).map(move |k| at + 4 * k)
        }))
        .collect();
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for l in logs {
            put(a, b, l, 0u32);
        }
        for k in 1..7 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        zero_stacks(a, b);
        b.write(work, &[0u8; ab::DATA_INIT_WORK as usize]).unwrap();
        put(a, b, opened, answers);
        let missing = [0, 2, 6, 70][rng.below(4) as usize];
        for i in 0..n {
            let file = if rng.below(70) < missing {
                0
            } else {
                rng.next() | 1
            };
            put(a, b, answers + 4 * i, file);
        }
        put(a, b, cursor, rng.next());
        for &w in &block_words {
            put(a, b, a.rebase(w), rng.next());
        }
        set(
            a,
            b,
            ab::DATA_LOADED,
            [0, 0, 1, rng.next()][rng.below(4) as usize],
        );
        set(a, b, ab::DATA_INIT_CLEAR, rng.next());
        set(a, b, menu, rng.next());
        for k in 0..ab::DATA_INIT_UNIFIED / 4 {
            put(a, b, template + 4 * k, rng.next());
        }
        if rng.below(3) != 0 {
            put(a, b, template, 0x31u16);
        }
        let limit = ab::DATA_INIT_WORK - 8;
        let count = [0, 1, rng.below(0x475), limit, rng.below(limit + 1)][rng.below(5) as usize];
        put(a, b, template + 2, u16::try_from(count).unwrap());
        // the activation lists: the game's, or lists of our own whose
        // nodes overlap (the bits add up), and start nodes of our own
        if rng.below(2) == 0 {
            for (g, &table) in (0u32..).zip(ab::ACTIVE_LISTS.iter()) {
                for k in 0..7u32 {
                    let list = lists + 0x40 * (7 * g + k);
                    let len = rng.below(0x1f);
                    for j in 0..len {
                        let node = if rng.below(2) == 0 {
                            rng.below(8)
                        } else {
                            rng.below(0x980)
                        };
                        put(a, b, list + 2 * j, u16::try_from(node).unwrap());
                    }
                    put(a, b, list + 2 * len, 0xffffu16);
                    put(a, b, a.rebase(table) + 4 * k, list);
                }
            }
            for &table in &ab::START_NODES {
                for k in 0..7u32 {
                    put(a, b, a.rebase(table) + 2 * k, rng.next().to_le_bytes()[0]);
                }
            }
        } else {
            for (at, game) in [
                (ab::ACTIVE_LISTS[0], &active[0]),
                (ab::ACTIVE_LISTS[1], &active[1]),
                (ab::ACTIVE_LISTS[2], &active[2]),
            ] {
                for (k, &w) in (0u32..).zip(game.iter()) {
                    put(a, b, a.rebase(at) + 4 * k, w);
                }
            }
            for (&at, game) in ab::START_NODES.iter().zip(&starts) {
                for (k, &w) in (0u32..).zip(game.iter()) {
                    put(a, b, a.rebase(at) + 2 * k, w);
                }
            }
        }
        for k in 0..0x1400 / 4 {
            put(a, b, save + 4 * k, rng.next());
        }
        twin("eiAbmDataInit", a, b, 0x00a5_3de0, &[save], |m| {
            ab::ei_abm_data_init(&mut H(m), save, work)
        });
    }
}

#[test]
fn abm_data_init_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1803);
    abm_data_init_cases(&mut a, &mut b, &mut rng, 60);
}

#[test]
fn abm_data_init_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1804);
    abm_data_init_cases(&mut a, &mut b, &mut rng, 30);
}

/// `sa_menu_start` (`0x00770430`) over recorders: `dbgPrintf` logs its
/// value only for the formats that have one, `HelpMem_Get` hands out
/// blocks of an arena, the CD library's sizes come from a list, the task
/// points `+8` at a recorder; every kind, both tails, an event FTC or
/// not, English or not.
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn sa_menu_start_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    let (prints, gets, sizes) = (
        alloc(a, b, 4 + 8 * 16),
        alloc(a, b, 4 + 8 * 16),
        alloc(a, b, 4 * 8),
    );
    let (cursor, size_at) = (alloc(a, b, 4), alloc(a, b, 4));
    let size_log = alloc(a, b, 4 + 4 * 16);
    let arena = alloc(a, b, 12 * 0x4800);
    // dbgPrintf: the format, and the value unless the format has none
    let mut c = vec![0xa1];
    c.extend(prints.to_le_bytes()); // mov eax, [prints]
    c.extend([0xc1, 0xe0, 0x03, 0x05]); // shl eax, 3; add eax, prints + 4
    c.extend((prints + 4).to_le_bytes());
    c.extend([0x8b, 0x4c, 0x24, 0x04, 0x89, 0x08, 0x31, 0xd2]); // mov ecx, [esp+4]; mov [eax], ecx; xor edx, edx
    for (k, s) in [st::STR_MENU_START, st::STR_DASHES, st::STR_CNF]
        .into_iter()
        .enumerate()
    {
        c.extend([0x81, 0xf9]); // cmp ecx, s
        c.extend(a.rebase(s).to_le_bytes());
        c.extend([0x74, [20u8, 12, 4][k]]); // je skip
    }
    c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
    c.extend([0x89, 0x50, 0x04, 0xff, 0x05]); // skip: mov [eax+4], edx; inc dword [prints]
    c.extend(prints.to_le_bytes());
    c.push(0xc3);
    redirect_to(a, b, st::SA_DBG_PRINTF, &c);
    // HelpMem_Get: the cursor, moved on by 0x4800
    let mut tail = vec![0xa1];
    tail.extend(cursor.to_le_bytes()); // mov eax, [cursor]
    tail.extend([0x81, 0x05]); // add dword [cursor], 0x4800
    tail.extend(cursor.to_le_bytes());
    tail.extend(0x4800u32.to_le_bytes());
    tail.push(0xc3);
    redirect_to(
        a,
        b,
        st::HELP_MEM_GET,
        &arg_logger(gets, &[(0, 0), (1, 0)], &tail),
    );
    for (k, sym, n) in [
        (1, st::CLEAR_PAD_BUFF, 0),
        (2, st::GET_SAVE_CONFIG_ENGLISH, 0),
        (3, st::SBIN_PARSE, 1),
        (4, st::HELP_MEM_FREE, 1),
        (5, st::USER_FREE_THUNK, 1),
        (6, st::READ_EVENT_FTC, 1),
        (7, st::FTC_TRANSFER, 1),
        (8, st::SA_TASK_BUFF_SIZE_GET, 1),
        (9, st::SA_TASK_INIT2, 3),
        (10, st::READ_SPS2, 2),
        (11, st::SET_FILE_TABLES, 0),
        (12, st::DISK_OPEN_CHECK_SET, 1),
        (13, st::HDD_OPEN_PARTITION, 0),
        (14, st::PATCH_MENU_RECORDS, 0),
        (15, st::FIND_RECORD_AT, 0),
        (16, st::FIND_RECORD_FLAGGED, 0),
        // the task's +8 and the CD library's calls, by pointer
        (17, 0x0270_8000, 3),
        (18, 0x0270_8100, 4),
        (19, 0x0270_8200, 1),
        (20, 0x0270_8300, 1),
    ] {
        recorder(a, b, k, sym, n, 0);
    }
    // g_CdLib + 0x14: the next size of the list
    let mut tail = vec![0xa1];
    tail.extend(size_at.to_le_bytes()); // mov eax, [size_at]
    tail.extend([0x8b, 0x00, 0x83, 0x05]); // mov eax, [eax]; add dword [size_at], 4
    tail.extend(size_at.to_le_bytes());
    tail.extend([4, 0xc3]);
    let size_fn = code_in_both(a, b, &arg_logger(size_log, &[(0, 0)], &tail));
    let task = alloc(a, b, 0x10);
    put(a, b, task + 8, a.rebase(0x0270_8000));
    let ret9 = ret(a, 9);
    put(a, b, ret9, task);
    for (g, sym) in [
        (st::CDLIB_READ, 0x0270_8100u32),
        (st::CDLIB_SYNC, 0x0270_8200),
        (st::CDLIB_TABLE, 0x0270_8300),
    ] {
        let at = a.rebase(sym);
        set(a, b, g, at);
    }
    set(a, b, st::CDLIB_SIZE, size_fn);
    let words: Vec<u32> = [
        0x00c4_16bc,
        0x00c4_16c0,
        0x00c4_16c4,
        0x00c4_16cc,
        0x00c4_16d0,
        0x00c4_16d4,
        0x00c4_16d8,
        0x00c4_16dc,
        0x0112_a010,
        0x0112_a014,
        0x0112_a018,
        0x0112_a02c,
        0x0112_a030,
        0x0112_a034,
        0x0112_a038,
        0x0112_a03c,
        0x0112_a040,
        0x0112_a044,
        0x0112_a048,
        0x0112_a04c,
        0x0112_a050,
        0x0112_a054,
        0x0112_a064,
        0x0112_a068,
        0x0112_a07c,
        0x0112_a08c,
        0x0112_a090,
        0x0112_a094,
        0x0112_a0a4,
        0x0112_a4a8,
        0x0231_0c14,
        0x0231_0c18,
        0x0231_0c1c,
        0x0231_0c20,
    ]
    .into_iter()
    .collect();
    let kinds = [0u32, 1, 2, 3, 4, 5, 0xff, 0x8000, 0x8001, 0x8004, 0x104];
    let n_kinds = u32::try_from(kinds.len()).unwrap();
    for round in 0..rounds {
        put(a, b, log, 0u32);
        put(a, b, prints, 0u32);
        put(a, b, gets, 0u32);
        put(a, b, size_log, 0u32);
        for k in 1..17 {
            if k != 9 {
                let at = ret(a, k);
                put(a, b, at, rng.next());
            }
        }
        let at = ret(a, 2);
        put(a, b, at, [0, 1, rng.next()][rng.below(3) as usize]);
        let at = ret(a, 6);
        put(a, b, at, if rng.below(2) == 0 { 0 } else { rng.next() });
        for &w in &words {
            put(a, b, a.rebase(w), rng.next());
        }
        if rng.below(2) == 0 {
            put(a, b, a.rebase(0x0112_a010), 0u32);
        }
        set(a, b, st::CD_READ_MODE, rng.next());
        set(
            a,
            b,
            st::FFX_HDD_MODE,
            rng.next().to_le_bytes()[0].cast_signed(),
        );
        put(a, b, cursor, arena);
        let fill: Vec<u8> = (0..12 * 0x4800)
            .map(|_| rng.next().to_le_bytes()[0])
            .collect();
        a.write(arena, &fill).unwrap();
        b.write(arena, &fill).unwrap();
        put(a, b, size_at, sizes);
        for k in 0..8 {
            put(a, b, sizes + 4 * k, rng.below(0x1000));
        }
        let kind = if round < n_kinds {
            kinds[round as usize]
        } else if rng.below(4) == 0 {
            rng.next()
        } else {
            kinds[rng.below(n_kinds) as usize] | (rng.next() & 0x7f00)
        };
        twin("sa_menu_start", a, b, 0x0077_0430, &[kind], |m| {
            st::sa_menu_start(&mut H(m), kind)
        });
    }
}

#[test]
fn sa_menu_start_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1805);
    sa_menu_start_cases(&mut a, &mut b, &mut rng, 300);
}

#[test]
fn sa_menu_start_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1806);
    sa_menu_start_cases(&mut a, &mut b, &mut rng, 100);
}

/// `AbmapDrawChrPanel` over stand-ins: the message size writes its
/// width and height from slots (only the text is logged: the two
/// pointers are frame addresses); every slide frame, both directions.
fn abm_chr_panel_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as ab;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, ab::PANEL_FRAME, 3, 0);
    recorder(a, b, 2, ab::DRAW_CHR_FACE, 2, 0);
    recorder(a, b, 3, ab::SAVE_CHR_NAME, 1, 0);
    let size = alloc(a, b, 8);
    let mut c = vec![];
    for (arg, from) in [(8u8, size), (12, size + 4)] {
        c.extend([0x8b, 0x44, 0x24, arg, 0x8b, 0x0d]); // mov eax, [esp+arg]; mov ecx, [from]
        c.extend(from.to_le_bytes());
        c.extend([0x89, 0x08]); // mov [eax], ecx
    }
    c.push(0xc3);
    logged(a, b, 4, ab::MESSAGE_SIZE, 1, &c);
    recorder(a, b, 5, ab::HELP_LINE_DRAW, 3, 0);
    recorder(a, b, 6, ab::GET_SAVE_SP, 1, 0);
    recorder(a, b, 7, ab::DIGIT_RIGHT, 6, 0);
    let g = alloc(a, b, 0x1_1700);
    set(a, b, ab::ABMAP, g);
    let work = b.stack_range().start + 0x1000;
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..8 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        let w = [0, 1, 0x7f, rng.below(0x200), rng.next()][rng.below(5) as usize];
        put(a, b, size, w);
        put(a, b, size + 4, rng.next());
        put(a, b, g + 0x115bc, rng.next().to_le_bytes()[0]);
        put(
            a,
            b,
            g + 0x115c0,
            [0u8, 0, 1, rng.next().to_le_bytes()[0]][rng.below(4) as usize],
        );
        let frame = [0u8, 1, 2, 3, 4, 0xff, rng.next().to_le_bytes()[0]][rng.below(7) as usize];
        put(a, b, g + 0x115c1, frame);
        put(
            a,
            b,
            g + 0x115c2,
            [0u8, 1, rng.next().to_le_bytes()[0]][rng.below(3) as usize],
        );
        put(a, b, g + 0x1161c, rng.next());
        twin("AbmapDrawChrPanel", a, b, 0x00a4_fbe0, &[], |m| {
            ab::abmap_draw_chr_panel(&mut H(m), work)
        });
    }
}

#[test]
fn abm_chr_panel_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1107);
    abm_chr_panel_cases(&mut a, &mut b, &mut rng, 1500);
}

#[test]
fn abm_chr_panel_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1108);
    abm_chr_panel_cases(&mut a, &mut b, &mut rng, 500);
}

/// `AbmapPadEntryDrawItem` over stand-ins: the element draw logs its
/// quad (0x80 bytes) and kind; rows around the clip's edges, both
/// parities (negative indices too), dimmed or not.
fn abm_draw_item_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as ab;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, ab::SAVE_ITEM_NUM, 1, 0);
    recorder(a, b, 2, ab::ICON_ITEM, 5, 0);
    recorder(a, b, 3, ab::ABMAP_DIGIT_RIGHT, 4, 0);
    let quads = alloc(a, b, 4 + 0x84 * 2);
    let mut c = arg_logger(quads, &[(0, 0x80), (2, 0)], &[]);
    c.extend(answer(ret(a, 4)));
    redirect_to(a, b, ab::DRAW_UI_ELEMENT, &c);
    let panel = alloc(a, b, 0x40);
    let entry = alloc(a, b, 0x10);
    let work = b.stack_range().start + 0x1000;
    let pos = a.rebase(ab::PACKET_POS.sym);
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        put(a, b, quads, 0u32);
        for k in 1..5 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        for k in 0..0x10 {
            put(a, b, panel + 4 * k, rng.next());
        }
        for k in 0..4 {
            put(a, b, entry + 4 * k, rng.next());
        }
        put(
            a,
            b,
            entry + 4,
            [0u8, 1, rng.next().to_le_bytes()[0]][rng.below(3) as usize],
        );
        put(a, b, pos, rng.next());
        let small = |rng: &mut Xorshift, m: u32| (rng.below(2 * m)).wrapping_sub(m);
        let top = small(rng, 0x100);
        let height = rng.below(0x100);
        put(a, b, panel + 0xa, u16::try_from(top & 0xffff).unwrap());
        put(a, b, panel + 0xe, u16::try_from(height).unwrap());
        put(
            a,
            b,
            panel + 0xc,
            u16::try_from(small(rng, 0x200) & 0xffff).unwrap(),
        );
        put(
            a,
            b,
            panel + 0x20,
            u16::try_from(small(rng, 0x200) & 0xffff).unwrap(),
        );
        // rows at the clip's edges and anywhere
        let y = match rng.below(4) {
            0 => top.wrapping_sub(0x13).wrapping_add(rng.below(4)),
            1 => top
                .wrapping_add(height)
                .wrapping_sub(2)
                .wrapping_add(rng.below(5))
                .wrapping_sub(2),
            _ => top.wrapping_add(small(rng, 0x80)),
        };
        let x = small(rng, 0x400);
        let index = [0, 1, 2, u32::MAX, 0xffff_fffe, rng.next()][rng.below(6) as usize];
        let args = [panel, entry, index, x, y];
        twin("AbmapPadEntryDrawItem", a, b, 0x00a4_f630, &args, |m| {
            ab::abmap_pad_entry_draw_item(&mut H(m), work, args)
        });
    }
}

#[test]
fn abm_draw_item_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1109);
    abm_draw_item_cases(&mut a, &mut b, &mut rng, 3000);
}

#[test]
fn abm_draw_item_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_110a);
    abm_draw_item_cases(&mut a, &mut b, &mut rng, 1000);
}

/// A stand-in tail that stores 1 through argument `arg`'s pointer while
/// `[count]` is not 0 (counting it down), then `[last]`, and answers
/// `[ret]`.
fn store_countdown(arg: u8, count: u32, last: u32, ret: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 4 + 4 * arg, 0x8b, 0x0d]; // mov eax, [esp+..]; mov ecx, [count]
    c.extend(count.to_le_bytes());
    c.extend([0x85, 0xc9, 0x74, 0x0d, 0xff, 0x0d]); // test ecx, ecx; jz last; dec dword [count]
    c.extend(count.to_le_bytes());
    c.extend([0xc7, 0x00, 1, 0, 0, 0]); // mov dword [eax], 1
    c.push(0xa1);
    c.extend(ret.to_le_bytes()); // mov eax, [ret]
    c.push(0xc3);
    // jz lands here: 6 + 6 + 5 + 1 = 18 bytes on
    let at = c.len() - 18;
    c[at - 1] = 18;
    c.extend([0x8b, 0x0d]);
    c.extend(last.to_le_bytes()); // mov ecx, [last]
    c.extend([0x89, 0x08]); // mov [eax], ecx
    c.push(0xa1);
    c.extend(ret.to_le_bytes()); // mov eax, [ret]
    c.push(0xc3);
    c
}

/// `menu_list_step` over stand-ins: up to three lists of up to four
/// records (text, kind 6, value records), the text expansion jumping a
/// counted number of times, the cursor step repeating a counted number.
#[allow(clippy::too_many_lines)] // the stand-ins, then the lists
fn menu_list_step_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, st::PAD_PRINT_FLAG, 0, 0);
    recorder(a, b, 2, st::CONTROL_PAD, 0, 0);
    recorder(a, b, 3, st::CONTROL_PAD_TRG, 0, 0);
    recorder(a, b, 4, st::CONTROL_PAD_REP, 0, 0);
    // the cursor check notes the mirror word as it sees it
    let mirrors = alloc(a, b, 4 + 64 * 4);
    let mut c = vec![0x8b, 0x0d];
    c.extend(mirrors.to_le_bytes()); // mov ecx, [mirrors]
    c.push(0xa1);
    c.extend(a.rebase(st::ML_MIRROR.sym).to_le_bytes()); // mov eax, [mirror]
    c.extend([0x89, 0x04, 0x8d]);
    c.extend((mirrors + 4).to_le_bytes()); // mov [mirrors + 4 + 4 ecx], eax
    c.extend([0xff, 0x05]);
    c.extend(mirrors.to_le_bytes()); // inc dword [mirrors]
    c.push(0xc3);
    logged(a, b, 5, st::CURSOR_BLOCKED, 0, &c);
    recorder(a, b, 6, st::RESET_RECORDS, 0, 0);
    // the open hook's function: a recorder in a dead function's place
    recorder(a, b, 7, 0x0077_a3f0, 0, 0);
    let counts = alloc(a, b, 16);
    // the expansion may move the target (from a slot)
    let moved = alloc(a, b, 4);
    let mut c = vec![0x8b, 0x0d];
    c.extend(moved.to_le_bytes()); // mov ecx, [moved]
    c.extend([0x89, 0x0d]);
    c.extend(a.rebase(st::ML_TARGET.sym).to_le_bytes()); // mov [target], ecx
    c.extend(store_countdown(3, counts, counts + 4, ret(a, 8)));
    logged(a, b, 8, st::EXPAND_CODES, 3, &c);
    let c = store_countdown(0, counts + 8, counts + 12, ret(a, 9));
    logged(a, b, 9, st::CURSOR_STEP, 0, &c);
    let hook = alloc(a, b, 4);
    put(a, b, hook, a.rebase(0x0077_a3f0));
    let lists = alloc(a, b, 3 * 8);
    let recs = [
        alloc(a, b, 4 * 0x138),
        alloc(a, b, 4 * 0x138),
        alloc(a, b, 4 * 0x138),
    ];
    let slots = alloc(a, b, 8 * 4);
    let texts = alloc(a, b, 64 * 4);
    // one pair before the table for index -1
    let pairs = alloc(a, b, 9 * 8) + 8;
    let work = b.stack_range().start + 0x1000;
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..10 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        put(a, b, counts, [0, 0, 1, 2][rng.below(4) as usize]);
        put(a, b, counts + 4, 0u32);
        put(a, b, counts + 8, rng.below(3));
        put(a, b, counts + 12, [0, 2, rng.next()][rng.below(3) as usize]);
        set(a, b, st::ML_HOOK, if rng.below(2) == 0 { 0 } else { hook });
        let base = [0, 0x40, rng.below(0x100)][rng.below(3) as usize];
        set(a, b, st::ML_BASE, base);
        for (k, &r) in (0u32..).zip(&recs) {
            put(a, b, lists + 8 * k, r.wrapping_sub(base));
            let n = [0u16, 1, 2, 3, 4, 0xffff][rng.below(6) as usize];
            put(a, b, lists + 8 * k + 4, n);
            for i in 0..4 * 0x138 / 4 {
                put(a, b, r + 4 * i, rng.next());
            }
            for i in 0..4 {
                let rec = r + 0x138 * i;
                let small = |rng: &mut Xorshift, m: u32| u16::try_from(rng.below(m)).unwrap();
                put(
                    a,
                    b,
                    rec,
                    [0u8, 0, 6, 6, rng.next().to_le_bytes()[0]][rng.below(5) as usize],
                );
                put(a, b, rec + 0x16, small(rng, 9));
                for o in [0x18, 0x24] {
                    let v = if rng.below(3) == 0 {
                        0xffff
                    } else {
                        small(rng, 8)
                    };
                    put(a, b, rec + o, v);
                }
                put(a, b, rec + 0x1c, [0u16, 0, 1, 3][rng.below(4) as usize]);
                let pair = if rng.below(6) == 0 {
                    0xffff
                } else {
                    small(rng, 5)
                };
                put(a, b, rec + 0x1e, pair);
                put(
                    a,
                    b,
                    rec + 0x105,
                    [0u8, 1, 2, 3, 4, 0xff][rng.below(6) as usize],
                );
                put(a, b, rec + 0x106, [0u8, 1, 2, 3][rng.below(4) as usize]);
                for o in [0x108, 0x10a] {
                    let high = if rng.below(8) == 0 { 0x8000 } else { 0 };
                    put(a, b, rec + o, small(rng, 11) | high);
                }
                put(a, b, rec + 0x10c, small(rng, 8));
                put(
                    a,
                    b,
                    rec + 0x110,
                    [2u8, 2, 0, rng.next().to_le_bytes()[0]][rng.below(4) as usize],
                );
                put(
                    a,
                    b,
                    rec + 0x116,
                    [0u16, 0, 1, 0x8000][rng.below(4) as usize],
                );
            }
        }
        set(
            a,
            b,
            st::ML_LISTS,
            if rng.below(8) == 0 { 0 } else { lists },
        );
        set(a, b, st::ML_COUNT, [0, 1, 2, 3, -1][rng.below(5) as usize]);
        let current = rng.below(3);
        set(a, b, st::ML_CURRENT, current);
        let first = recs[current as usize];
        set(
            a,
            b,
            st::ML_LAST,
            if rng.below(2) == 0 { first } else { rng.next() },
        );
        let target = match rng.below(4) {
            0 => 0,
            1 => rng.next(),
            _ => recs[rng.below(3) as usize] + 0x138 * rng.below(4),
        };
        set(a, b, st::ML_TARGET, target);
        let next = recs[rng.below(3) as usize] + 0x138 * rng.below(4);
        put(a, b, moved, if rng.below(2) == 0 { target } else { next });
        put(a, b, mirrors, 0u32);
        set(a, b, st::ML_BUSY, u8::from(rng.below(6) == 0));
        set(a, b, st::ML_SLOTS, slots);
        set(a, b, st::ML_TEXTS, texts);
        set(a, b, st::ML_PAIRS, pairs);
        set(a, b, st::ML_OUT, rng.next());
        set(a, b, st::ML_TEXT_BASE, rng.next());
        set(a, b, st::ML_TRG, rng.next().to_le_bytes()[0].into());
        for k in 0..9 {
            put(a, b, pairs - 8 + 8 * k, rng.below(8));
            put(a, b, pairs - 4 + 8 * k, rng.below(11));
        }
        for k in 0..8 {
            put(a, b, slots + 4 * k, rng.below(11));
        }
        for k in 0..64 {
            put(a, b, texts + 4 * k, rng.next());
        }
        // pad words with the bits the value records test
        for k in 2..5 {
            let at = ret(a, k);
            let bits = [0, 0x1000, 0x4000, 0x8000, 0x2000, 1, 4, 8, rng.next()];
            put(
                a,
                b,
                at,
                bits[rng.below(9) as usize] | bits[rng.below(9) as usize],
            );
        }
        twin("menu_list_step", a, b, 0x0076_f470, &[], |m| {
            st::menu_list_step(&mut H(m), work)
        });
    }
}

#[test]
fn menu_list_step_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_110b);
    menu_list_step_cases(&mut a, &mut b, &mut rng, 3000);
}

#[test]
fn menu_list_step_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_110c);
    menu_list_step_cases(&mut a, &mut b, &mut rng, 1000);
}

/// `eiAbmCalc` with the vector and matrix originals and the rest
/// recorded (`AbmapState_HomeOnNode` too, so the state compares as the
/// game's); every fade frame, each pad branch, the targets near and past
/// the edges.
#[allow(clippy::too_many_lines)] // the stand-ins, then the grid
fn abm_calc_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as ab;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    let stands = [
        (ab::READ_PAD, 2),
        (ab::DBG_PRINTF, 1),
        (ab::CTRL_ZOOM, 0),
        (ab::CTRL_TILT, 0),
        (ab::TRY_CHANGE_PLAYER, 0),
        (ab::STATE_HOME_ON_NODE, 0),
        (ab::PAD_CTRL, 0),
        (ab::PULSE_CURSOR, 0),
        (ab::SET_ABMAP, 5),
        (ab::SPREAD_PLY_TAGS, 0),
        // a state and a pad slot callback in dead functions' places
        (0x0077_a3f0, 0),
        (0x0077_a420, 1),
    ];
    for (k, &(sym, n)) in (1u32..).zip(&stands) {
        recorder(a, b, k, sym, n, 0);
    }
    let code = |a: &Oracle, k: u32| a.rebase(REC_CODE) + 0x60 * k;
    let (state, callback) = (code(a, 11), code(a, 12));
    let home = a.rebase(ab::STATE_HOME_ON_NODE);
    let g = alloc(a, b, 0x1_1700);
    set(a, b, ab::ABMAP, g);
    let pad = alloc(a, b, 0x2710);
    set(a, b, ab::ABMAP_PAD, pad);
    let work = b.stack_range().start + 0x1000;
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..13 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        for k in (0x1_1300..0x1_1700).step_by(4) {
            put(a, b, g + k, rng.float());
        }
        // a moderate scale, edges around the target
        let scale = [0x3f80_0000, 0x4000_0000, 0x3f00_0000, rng.float()][rng.below(4) as usize];
        put(a, b, g + 0x1_1350, scale);
        let near = |rng: &mut Xorshift| float_within(rng, 2000.0);
        for o in [0x1_1308, 0x1_130c, 0x1_15fc, 0x1_1600] {
            put(a, b, g + o, near(rng));
        }
        for o in [0x1_1604, 0x1_1608] {
            let v = f32::from_bits(near(rng)) + 1500.0;
            put(a, b, g + o, v.to_bits());
        }
        if rng.below(8) == 0 {
            put(a, b, g + 0x1_1308, rng.float());
        }
        // exactly on an edge (scale 1: the half screen 341 x 208, less
        // 96 for the clamp)
        if rng.below(3) == 0 {
            put(a, b, g + 0x1_1350, 1.0f32);
            #[allow(clippy::cast_precision_loss)] // small integers
            let int = |rng: &mut Xorshift| (rng.below(1000) as f32) - 500.0;
            let (x0, x1, y0, y1) = (int(rng), int(rng) + 1500.0, int(rng), int(rng) + 1500.0);
            let (mut x, mut y) = (int(rng), int(rng));
            let (mut x0, mut x1, mut y1) = (x0, x1, y1);
            match rng.below(5) {
                0 => x = x0 + 469.0,
                1 => x = x1 - 469.0,
                2 => (x1, y1) = (x0 + 490.0, y0 + 224.0),
                3 => (x, y) = (x0 + 245.0, y0 + 112.0),
                _ => (x, y) = (x1 - 245.0, y1 - 112.0),
            }
            // -0 on an edge at +0: the store shows
            match rng.below(6) {
                0 => (x0, x) = (-245.0, -0.0),
                1 => (x1, x) = (245.0, -0.0),
                _ => {}
            }
            for (o, v) in [
                (0x1_15fc, x0),
                (0x1_1604, x1),
                (0x1_1600, y0),
                (0x1_1608, y1),
                (0x1_1308, x),
                (0x1_130c, y),
            ] {
                put(a, b, g + o, v);
            }
        }
        let frame = [0u8, 1, 9, 10, 11, 14, 15, 19, 20, 0x14, 0x15, 0xff][rng.below(12) as usize];
        put(a, b, g + 0x1_15b8, frame);
        put(a, b, g + 0x1_166e, [0u16, 0, 1][rng.below(3) as usize]);
        for o in [0x1_15ce, 0x1_15d0] {
            put(a, b, g + o, [0u16, 1, 0xffff][rng.below(3) as usize]);
        }
        put(a, b, g + 0x1_15a8, [0, state, home][rng.below(3) as usize]);
        for o in [0x1_15ba, 0x1_15c0, 0x1_15c7] {
            put(
                a,
                b,
                g + o,
                [0u8, 1, rng.next().to_le_bytes()[0]][rng.below(3) as usize],
            );
        }
        put(a, b, g + 0x1_1340, rng.next());
        put(
            a,
            b,
            g + 0x1_16a0,
            [99u32, 100, 101, rng.next()][rng.below(4) as usize],
        );
        for k in 0..12 {
            put(
                a,
                b,
                pad + 0x340 * k + 0x34,
                if rng.below(3) == 0 { callback } else { 0 },
            );
        }
        put(a, b, pad + 0x13a3, 0u8);
        put(
            a,
            b,
            pad + 0x2700,
            [0xffffu16, 7, 7, 0, 3][rng.below(5) as usize],
        );
        twin("eiAbmCalc", a, b, 0x00a5_3570, &[], |m| {
            ab::ei_abm_calc(&mut H(m), work)
        });
    }
}

#[test]
fn abm_calc_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_110d);
    abm_calc_cases(&mut a, &mut b, &mut rng, 1500);
}

#[test]
fn abm_calc_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_110e);
    abm_calc_cases(&mut a, &mut b, &mut rng, 500);
}

/// `Sg_InitAll` and `Sg_Test` over recorders (`user_malloc` answering
/// from a slot); every `Sg_Test` case and some outside the table.
fn init_all_test_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    let stands = [
        (st::LOAD_VRAM_RESOURCE, 0),
        (st::SG_USER_MALLOC, 1),
        (st::SG_USER_FREE, 1),
        (st::INIT_FILTER, 0),
        (st::DB_INIT_ALL, 0),
        (st::SG_DMY, 0),
        (st::INIT_0083_9CA0, 1),
        (st::FUNCTION_HISTORY_INIT, 0),
        (st::TEXTURE_SPEC_INIT_ALL, 0),
        (st::DRAW_SET_SHADOW_MODE, 1),
        (st::MEM_MOVE_SET_TRIGGER, 0),
        (st::SG_DBG_PRINTF, 2),
        (st::MAP_SET_NAME, 2),
        (st::SG_MAP_INIT, 0),
    ];
    for (k, &(sym, n)) in (1u32..).zip(&stands) {
        recorder(a, b, k, sym, n, 0);
    }
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..15 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        set(a, b, st::TEST_BLOCK, rng.next());
        set(a, b, st::SG_FRAME, rng.next());
        set(a, b, st::GS_REGISTER_SINK, rng.next());
        twin("Sg_InitAll", a, b, 0x0083_b5f0, &[], |m| {
            st::init_all(&mut H(m))
        });
        put(a, b, log, 0u32);
        let n = [0, 1, 2, 3, 4, u32::MAX, rng.next()][rng.below(7) as usize];
        twin("Sg_Test", a, b, 0x0084_67b0, &[n], |m| {
            st::sg_test(&mut H(m), n)
        });
    }
}

#[test]
fn init_all_test_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_110f);
    init_all_test_cases(&mut a, &mut b, &mut rng, 200);
}

#[test]
fn init_all_test_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1110);
    init_all_test_cases(&mut a, &mut b, &mut rng, 100);
}

/// `eiAbmLoad` over `eiAbmSetLang`'s stand-ins: `user_malloc` hands out
/// a list of blocks, `getExcelBinSize` answers a list of sizes.
fn abm_load_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::abmap::bridge as ab;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    recorder(a, b, 1, ab::GET_SAVE_CONFIG_ENGLISH, 0, 0);
    // a list stand-in: logs its first argument, answers the next word
    let list = |a: &mut Oracle, b: &mut Oracle, sym: u32, n: u8| -> u32 {
        let cursor = alloc(a, b, 4);
        let mut c = vec![0xa1]; // mov eax, [cursor]
        c.extend(cursor.to_le_bytes());
        c.extend([0x8b, 0x00, 0x83, 0x05]); // mov eax, [eax]; add dword [cursor], 4
        c.extend(cursor.to_le_bytes());
        c.extend([4, 0xc3]);
        let k = if n == 1 { 2 } else { 3 };
        logged(a, b, k, sym, n, &c);
        cursor
    };
    let mallocs = list(a, b, ab::USER_MALLOC, 1);
    let sizes = list(a, b, ab::GET_EXCEL_BIN_SIZE, 2);
    recorder(a, b, 4, ab::USER_FREE, 1, 0);
    recorder(a, b, 5, ab::FLUSH_CACHE, 1, 0);
    recorder(a, b, 6, 0x0077_a3f0, 1, 0);
    recorder(a, b, 7, 0x0077_a420, 1, 0);
    let code = |a: &Oracle, k: u32| a.rebase(REC_CODE) + 0x60 * k;
    let (read_table, read_sync) = (code(a, 6), code(a, 7));
    set(a, b, ab::CDLIB_READ_TABLE, read_table);
    set(a, b, ab::CDLIB_READ_SYNC, read_sync);
    // the load's buffer and two blocks, then a buffer per file
    let blocks = [0; 5].map(|_| alloc(a, b, 0x200));
    let (block_list, size_list) = (alloc(a, b, 20), alloc(a, b, 20));
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..8 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        put(a, b, mallocs, block_list);
        put(a, b, sizes, size_list);
        for (k, &blk) in (0u32..).zip(&blocks) {
            put(a, b, block_list + 4 * k, blk);
            put(a, b, size_list + 4 * k, rng.below(0x201));
            for i in 0..0x80 {
                put(a, b, blk + 4 * i, rng.next());
            }
        }
        twin("eiAbmLoad", a, b, 0x00a5_4810, &[], |m| {
            ab::ei_abm_load(&mut H(m))
        });
    }
}

#[test]
fn abm_load_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1111);
    abm_load_cases(&mut a, &mut b, &mut rng, 100);
}

#[test]
fn abm_load_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1112);
    abm_load_cases(&mut a, &mut b, &mut rng, 50);
}

/// `Sg_FrameSync` over stand-ins: the microsecond clock answers a list
/// (around the sync's last time, then later), the cut-scene sync turns the
/// sync off after a count; every branch of the frame and voice hold.
#[allow(clippy::too_many_lines)] // the stand-ins, then the state
fn frame_sync_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    let stands = [
        (st::SET_FRAM_SYNC, 0),
        (st::CAN_SPEED_UP, 0),
        (st::SCREEN_CLOSED, 0),
        (st::SCAN_CONTROLER, 0),
        (st::VSYNC_MAIN, 0),
        (st::EFFECT_READ_POLL, 1),
    ];
    for (k, &(sym, n)) in (1u32..).zip(&stands) {
        recorder(a, b, k, sym, n, 0);
    }
    // the sync: logged; after `[count]` calls it stores `[off]` in the
    // on flag
    let (count, off) = (alloc(a, b, 4), alloc(a, b, 4));
    let on = a.rebase(st::FS_SYNC_ON.sym);
    let mut c = vec![0xa1];
    c.extend(count.to_le_bytes()); // mov eax, [count]
    c.extend([0x85, 0xc0, 0x75, 0x0d, 0x8b, 0x0d]); // test eax, eax; jnz more; mov ecx, [off]
    c.extend(off.to_le_bytes());
    c.extend([0x89, 0x0d]);
    c.extend(on.to_le_bytes()); // mov [on], ecx
    c.extend([0xc3, 0xff, 0x0d]); // ret; more: dec dword [count]
    c.extend(count.to_le_bytes());
    c.push(0xc3);
    logged(a, b, 7, st::SYNC_CUT_SCENE, 0, &c);
    // the clock: the next of a list of 64-bit times
    let (cursor, times) = (alloc(a, b, 4), alloc(a, b, 8 * 40));
    let mut c = vec![0x8b, 0x0d];
    c.extend(cursor.to_le_bytes()); // mov ecx, [cursor]
    c.extend([0x8b, 0x01, 0x8b, 0x51, 0x04, 0x83, 0x05]); // mov eax, [ecx]; mov edx, [ecx+4]; add dword [cursor], 8
    c.extend(cursor.to_le_bytes());
    c.extend([8, 0xc3]);
    logged(a, b, 8, st::TIME_US, 0, &c);
    let time_us = a.rebase(st::TIME_US);
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..9 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        let at = ret(a, 2);
        put(a, b, at, rng.below(2));
        let frac = |rng: &mut Xorshift| float_within(rng, 1.0) & 0x7fff_ffff;
        let rem = if rng.below(4) == 0 { 0 } else { frac(rng) };
        set(a, b, st::FS_REMAINDER, rem);
        set(a, b, st::FS_VOICE_REMAINDER, frac(rng));
        set(
            a,
            b,
            st::FS_VOICE_HOLD,
            [0, 0, 0, 1, 2][rng.below(5) as usize],
        );
        set(a, b, st::FS_SYNC_ON, [1, 1, 0, 2][rng.below(4) as usize]);
        set(a, b, st::FS_RESTART, [0, 0, 1, 2][rng.below(4) as usize]);
        put(a, b, count, rng.below(4));
        put(a, b, off, [0u32, 2][rng.below(2) as usize]);
        let start = u64::from(rng.next());
        let paused = u64::from(rng.below(1_000_000));
        let add = u64::from(rng.below(1000));
        set(a, b, st::FS_START, start);
        set(a, b, st::FS_PAUSED, paused);
        set(a, b, st::FS_PAUSE_ADD, add);
        #[allow(clippy::cast_precision_loss)] // test values
        let last = (rng.below(1_000_000) as f32) / 10.0;
        set(a, b, st::FS_SYNC_LAST, last.to_bits());
        set(
            a,
            b,
            st::FS_SYNC_STEP,
            (last + [0.0f32, 16.6, 33.3][rng.below(3) as usize]).to_bits(),
        );
        set(
            a,
            b,
            st::FS_SYNC_RATE,
            [16.6f32, 0.0, 100.0][rng.below(3) as usize].to_bits(),
        );
        set(a, b, st::FS_SYNC_NOW, rng.float());
        // the clock: about the last time, then 30 ms steps, then far on
        let step = [30.0f64, 0.04, 0.0][rng.below(3) as usize];
        let delta = [
            -200.0f64, -50.0, -0.2, -0.15, -0.1, -0.05, 0.0, 0.05, 50.0, 200.0,
        ][rng.below(10) as usize];
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // test values
        let at_ms = |ms: f64| start + paused + add + (ms * 1000.0).max(0.0) as u64;
        for k in 0..40u32 {
            let t = if k < 36 {
                at_ms(f64::from(last) + delta + step * f64::from(k))
            } else {
                u64::MAX / 2
            };
            put(a, b, times + 8 * k, t);
        }
        if rng.below(10) == 0 {
            put(a, b, times, start.wrapping_sub(1));
        }
        // exactly 0.1 ms ahead: the last time f32 0.1, the clock at 0
        if rng.below(10) == 0 {
            set(a, b, st::FS_SYNC_LAST, 0.1f32.to_bits());
            put(a, b, times, start + paused + add);
            put(a, b, times + 8, start + paused + add);
        }
        put(a, b, cursor, times);
        for g in [st::FS_MOVIE_FADING, st::FS_PAUSE_ON, st::FS_PAUSE_2] {
            set(a, b, g, u32::from(rng.below(4) == 0));
        }
        set(a, b, st::FS_IS_DEBUG, [0u8, 1, 0x80][rng.below(3) as usize]);
        set(a, b, st::FS_VCOUNT, rng.next());
        set(a, b, st::FS_VCOUNT2, rng.next());
        let dt = match rng.below(6) {
            0 => 0x3e99_999a, // 0.3
            1 => 0x3c88_8889, // 1/60
            2 => 0x3d08_8889, // 1/30
            3 => 0x3f00_0000,
            4 => rng.float(),
            _ => float_within(rng, 0.4),
        };
        twin("Sg_FrameSync", a, b, 0x0082_1e80, &[dt], |m| {
            st::frame_sync(&mut H(m), dt, &mut |h| {
                let r = h.0.call(time_us, Conv::Cdecl, &[]).unwrap();
                u64::from(r.edx) << 32 | u64::from(r.eax)
            })
        });
    }
}

#[test]
fn frame_sync_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1113);
    frame_sync_cases(&mut a, &mut b, &mut rng, 3000);
}

#[test]
fn frame_sync_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_1114);
    frame_sync_cases(&mut a, &mut b, &mut rng, 1000);
}

/// Battle round 35: `spline_get_near_point` (`0x00a44120`) over splines
/// of 0 to 0x10 points, start parameters in and out of 0..1, sample
/// counts (0 and NaN too), both evaluators. `TkMtCalcSpline`/`3` run as
/// the originals on both sides; the three frame points start zeroed on
/// both, as the outputs do in the block.
fn near_point_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::spline::bridge as sp;
    let spline = alloc(a, b, 0x200);
    let outs = alloc(a, b, 0x10);
    let locals = b.stack_range().start + 0x400;
    let top = a.stack_range().end;
    let btop = b.stack_range().end;
    let mut reach = [0u32; 6];
    for round in 0..rounds {
        let count = [0u32, 1, 2, 2, 3, 3, 4, 4, 5, 6, 8, 0x10][rng.below(12) as usize];
        if round % 3 == 0 {
            for k in 0..0x7cu32 {
                let v = if rng.below(12) == 0 {
                    rng.float()
                } else if rng.below(3) == 0 {
                    // full mantissas: the squared distances' sums round
                    #[allow(clippy::cast_precision_loss)]
                    let v = (rng.next() as f32 / u32::MAX as f32 - 0.5) * 300.0;
                    v.to_bits()
                } else {
                    #[allow(clippy::cast_precision_loss)]
                    let v = (rng.below(2000) as f32 - 1000.0) / 8.0;
                    v.to_bits()
                };
                put(a, b, spline + 0x10 + 4 * k, v);
            }
        }
        put(a, b, spline, count);
        let t = match rng.below(8) {
            0 => [0u32, 0x3f80_0000, 0xbe00_0000, 0x3fa0_0000, 0x7fc0_0000][rng.below(5) as usize],
            #[allow(clippy::cast_precision_loss)]
            _ => (rng.below(1001) as f32 / 1000.0).to_bits(),
        };
        let samples = match rng.below(8) {
            0 => rng.float(),
            1 => 0,
            #[allow(clippy::cast_precision_loss)]
            _ => ((count * 10).max(1) as f32 * [1.0f32, 0.5, 2.0, 4.0][rng.below(4) as usize])
                .to_bits(),
        };
        set(a, b, sp::NEAR_SAMPLES, samples);
        // at times t is the first step: a step back lands on 0 exactly
        #[allow(clippy::cast_possible_truncation)]
        let first = (10.0 / f64::from(f32::from_bits(samples))) as f32;
        let t = if rng.below(6) == 0 {
            first.to_bits()
        } else {
            t
        };
        set(a, b, sp::NEAR_T, t);
        let pos: [u32; 3] = std::array::from_fn(|_| {
            if rng.below(16) == 0 {
                rng.float()
            } else if rng.below(3) == 0 {
                #[allow(clippy::cast_precision_loss)]
                let v = (rng.next() as f32 / u32::MAX as f32 - 0.5) * 300.0;
                v.to_bits()
            } else {
                #[allow(clippy::cast_precision_loss)]
                let v = (rng.below(2400) as f32 - 1200.0) / 8.0;
                v.to_bits()
            }
        });
        let mut cubic = [0u32, 1, 0x100][rng.below(3) as usize];
        let (mut t, mut pos, mut samples) = (t, pos, samples);
        if round == 0 {
            // The squared distance's sum order: for these x, y, z,
            // f32((x² + y²) + z²) != f32(x² + (y² + z²)) (a 53-bit sum
            // next to an f32 midpoint). The quadratic spline of 5 points
            // p0 = p1 = (x, y, z), p2 far, p3 = p4 = (z, y, x) gives those
            // two points at t 0 and 1 (the first step 0.5 from t 0.5,
            // samples 20) and a farther one at t, so the other order swaps
            // `a` and `b` and the step goes the other way.
            let [x, y, z] = [0x42d6_cf83u32, 0x3f0a_2a36, 0x3adc_b08d];
            let far = 0x447a_0000;
            for (k, p) in [[x, y, z], [x, y, z], [far; 3], [z, y, x], [z, y, x]]
                .into_iter()
                .enumerate()
            {
                let at = spline + 0x10 + 0x10 * u32::try_from(k).unwrap();
                for (w, v) in (0u32..).zip(p.into_iter().chain([0])) {
                    put(a, b, at + 4 * w, v);
                }
            }
            put(a, b, spline, 5);
            (t, pos, samples, cubic) = (0x3f00_0000, [0; 3], 0x41a0_0000, 0);
            set(a, b, sp::NEAR_SAMPLES, samples);
            set(a, b, sp::NEAR_T, t);
        }
        for k in 0..4 {
            put(a, b, outs + 4 * k, rng.next() & 0xbfff_ffff);
        }
        // the evaluators read a never-written frame word: zero both stacks
        a.write(top - 0x2000, &[0; 0x2000]).unwrap();
        b.write(btop - 0x2000, &[0; 0x2000]).unwrap();
        b.write(locals, &[0; 0x30]).unwrap();
        let args = [
            spline,
            pos[0],
            pos[1],
            pos[2],
            outs,
            outs + 4,
            outs + 8,
            cubic,
        ];
        let r = a.call(a.rebase(0x00a4_4120), Conv::Cdecl, &args).unwrap();
        a.call(a.rebase(POP_STUB), Conv::Cdecl, &[]).unwrap();
        let got = sp::spline_get_near_point(b, args, locals, &mut |o, cubic, w| {
            let f = if cubic { 0x0088_a4d0 } else { 0x0088_a4a0 };
            let at = o.rebase(f);
            o.call(at, Conv::Cdecl, &w).unwrap();
        });
        let want = r.st0_f32().expect("ST0");
        assert_eq!(
            want.to_bits(),
            got.to_bits(),
            "near point {args:x?} t {t:#x} s {samples:#x}: ST0"
        );
        assert_eq!(
            a.diff(b).unwrap(),
            [],
            "near point {args:x?} t {t:#x}: memory"
        );
        let moved = sp::NEAR_T.get(b);
        let kind = if moved == t {
            0
        } else if f32::from_bits(moved) < 0.0 {
            1
        } else if f32::from_bits(moved) > 1.0 {
            2
        } else {
            3
        };
        reach[kind] += 1;
        reach[4 + usize::from(cubic != 0)] += 1;
    }
    eprintln!("near point: {reach:?}");
}

#[test]
fn near_point_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x35a4_4120);
    near_point_cases(&mut a, &mut b, &mut rng, 1500);
}

#[test]
fn near_point_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x35a4_4121);
    near_point_cases(&mut a, &mut b, &mut rng, 800);
}

// ---- chara-main-map round 1

/// `code` in both oracles, with a jump to it at `sym`.
fn cmm_stub(a: &mut Oracle, b: &mut Oracle, sym: u32, code: &[u8]) {
    let at = a.alloc_code(code).unwrap();
    assert_eq!(b.alloc_code(code).unwrap(), at);
    let f = a.rebase(sym);
    let mut j = vec![0xe9];
    j.extend(at.wrapping_sub(f + 5).to_le_bytes());
    put(a, b, f, j[0]);
    for (k, byte) in j.iter().enumerate().skip(1) {
        put(a, b, f + u32::try_from(k).unwrap(), *byte);
    }
}

/// `op_shape_draw_eamap(shape, key, slot, layer)` as a stand-in logging
/// at `[log]`: shape, slot, layer, the key's `u16 +2`, words `+4`, `+8`,
/// `+0xc`, byte `+0x18`, then the 16 words at `[key +0x10]` and at
/// `[key +0x1c]` (the key is on the original's stack); returns 0.
fn eamap_stand_in(log: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x0d];
    c.extend(log.to_le_bytes()); // mov ecx, [log]
    for i in [0u8, 2, 3] {
        // mov eax, [esp + 4 + 4i]; mov [ecx], eax; add ecx, 4
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * i, 0x89, 0x01, 0x83, 0xc1, 0x04]);
    }
    c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp + 8]
    c.extend([0x0f, 0xb7, 0x42, 0x02, 0x89, 0x01, 0x83, 0xc1, 0x04]); // movzx eax, word [edx+2]
    for o in [4u8, 8, 0xc] {
        c.extend([0x8b, 0x42, o, 0x89, 0x01, 0x83, 0xc1, 0x04]); // mov eax, [edx+o]
    }
    c.extend([0x0f, 0xb6, 0x42, 0x18, 0x89, 0x01, 0x83, 0xc1, 0x04]); // movzx eax, byte [edx+0x18]
    for o in [0x10u8, 0x1c] {
        c.extend([0x8b, 0x44, 0x24, 0x08, 0x8b, 0x50, o]); // mov eax, [esp+8]; mov edx, [eax+o]
        for k in 0..16u8 {
            // mov eax, [edx + 4k]; mov [ecx], eax; add ecx, 4
            c.extend([0x8b, 0x42, 4 * k, 0x89, 0x01, 0x83, 0xc1, 0x04]);
        }
    }
    c.extend([0x89, 0x0d]);
    c.extend(log.to_le_bytes()); // mov [log], ecx
    c.extend([0x33, 0xc0, 0xc3]); // xor eax, eax; ret
    c
}

/// A stand-in returning the word at `slot`, which then grows by `step`.
fn counting_stand_in(slot: u32, step: u32) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend(slot.to_le_bytes()); // mov eax, [slot]
    c.extend([0x81, 0x05]);
    c.extend(slot.to_le_bytes());
    c.extend(step.to_le_bytes()); // add dword [slot], step
    c.push(0xc3);
    c
}

/// A stand-in that clears the byte at the address the word at `list`
/// points to, then moves `list` on by 4 (a list of the busy bytes to end,
/// one per call); returns 0.
fn clearing_stand_in(list: u32, log: u32) -> Vec<u8> {
    // [log] = its argument
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xa3];
    c.extend(log.to_le_bytes()); // mov eax, [esp+4]; mov [log], eax
    c.extend([0x8b, 0x0d]);
    c.extend(list.to_le_bytes()); // mov ecx, [list]
    c.extend([0x8b, 0x01, 0xc6, 0x00, 0x00]); // mov eax, [ecx]; mov byte [eax], 0
    c.extend([0x83, 0x05]);
    c.extend(list.to_le_bytes());
    c.push(4); // add dword [list], 4
    c.extend([0x33, 0xc0, 0xc3]);
    c
}

/// Chara-main-map round 1, main group: `Sg_CacheDisposeChr`,
/// `Sg_EventjumpDispose`, `0x0083b670`, `0x00a51340` (a made-up sphere
/// grid; the node matrices and the view product as the originals, the
/// area-map draw a stand-in logging its key) and the movie read task loop
/// `0x0076e660` (its callees recorders; `TkSyncV` ends one busy request
/// per pass, so the loop ends).
#[allow(clippy::too_many_lines)]
fn cmm1_main_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_sg::cdread::bridge as cd;
    use ffx_sg::state::bridge as st;
    let log = a.rebase(REC_LOG);
    let ret = |a: &Oracle, k: u32| a.rebase(REC_RETS) + 4 * k;
    let stands = [
        (st::CACHE_DISPOSE, 3),
        (st::SG_DBG_PRINTF, 1),
        (st::MEM_SET_FAST, 1),
        (st::CH_DISPOSE_ALL, 0),
        (st::CH_BLK_DISPOSE_ALL, 0),
        (st::CH_DISPOSE_ALL_MGRP, 0),
        (st::PLIGHT_DISPOSE, 0),
        (st::GET_BINDP_LIST, 1),
        (st::CACHE_CLEAR_CHECK, 0),
        (st::CACHE_TO_TOP, 2),
        (st::REG_MOTION_TO_TOP, 0),
        (st::CACHE_DISPOSE_CHECK, 0),
        (st::CACHE_EMPTY, 0),
        (st::REG_MOTION_USE, 0),
        (st::CH_DATA_READ, 1),
        (st::CH_MGRP_READ, 2),
        (st::CH_READ_SYNC, 1),
    ];
    for (k, &(sym, n)) in (1u32..).zip(&stands) {
        recorder(a, b, k, sym, n, 0);
    }
    let movie = [
        (cd::MOVIE_ERR, 3),
        (cd::SOUND_LOOP, 0),
        (cd::SIGNAL_SEMA, 1),
        (cd::STANDBY_VOICE, 0),
        (cd::LOADING_ACTIVE, 0),
        (cd::RESET_LOADING_SCREEN, 0),
        (cd::IS_BATTLE_MAP, 0),
        (cd::GET_IS_BATTLE_MAP, 0),
        (cd::LOADING_SCREEN_STEP, 0),
        (cd::LOADING_CALLBACK, 0),
        (cd::WAIT_SEMA, 1),
    ];
    for (k, &(sym, n)) in (20u32..).zip(&movie) {
        recorder(a, b, k, sym, n, 0);
    }
    // the task's step: a recorder of its own at slot 40's code
    recorder(a, b, 40, 0x0062_f4c0, 0, 0);
    let step = a.rebase(REC_CODE) + 0x60 * 40;
    let poll = alloc(a, b, 4);
    cmm_stub(a, b, cd::POLL_SEMA, &counting_stand_in(poll, 1));
    let busy_list = alloc(a, b, 4);
    let sync_arg = alloc(a, b, 4);
    cmm_stub(a, b, cd::TK_SYNC_V, &clearing_stand_in(busy_list, sync_arg));
    let elog = alloc(a, b, 8);
    let ebuf = alloc(a, b, 0x8000);
    cmm_stub(a, b, 0x007f_49e0, &eamap_stand_in(elog));
    let grid = alloc(a, b, 0x1_1700);
    let abmap = a.rebase(0x0230_5834);
    for _ in 0..rounds {
        put(a, b, log, 0u32);
        for k in 1..60 {
            let at = ret(a, k);
            put(a, b, at, rng.next());
        }
        let id = rng.next();
        twin("Sg_CacheDisposeChr", a, b, 0x0084_0e60, &[id], |m| {
            st::cache_dispose_chr(&mut H(m), id)
        });
        put(a, b, log, 0u32);
        twin("Sg_EventjumpDispose", a, b, 0x0083_b420, &[], |m| {
            st::eventjump_dispose(&mut H(m))
        });
        // the bind list: 0 to 5 ids
        put(a, b, log, 0u32);
        let n = rng.below(6);
        let list = block(a, b, rng, 2 * 8);
        put(a, b, list, u16::try_from(n).unwrap());
        let at = ret(a, 8);
        put(a, b, at, list);
        twin("Sg_EventjumpCacheChrs", a, b, 0x0083_b670, &[], |m| {
            st::eventjump_cache_chrs(&mut H(m))
        });
        // ---- the grid's nodes into the area map
        put(a, b, log, 0u32);
        put(a, b, elog, ebuf);
        let nodes = rng.below(6);
        for k in 0..0x1_1700 / 4 {
            let v = rng.next();
            let v = if v & 0x7f80_0000 == 0x7f80_0000 {
                v & 0xbfff_ffff
            } else {
                v
            };
            put(a, b, grid + 4 * k, v);
        }
        put(a, b, grid + 2, i16::try_from(nodes).unwrap());
        for i in 0..nodes {
            let node = grid + 0x808 + 0x28 * i;
            let kind: u16 = if rng.below(4) == 0 {
                0xffff
            } else {
                u16::try_from(rng.below(3)).unwrap()
            };
            put(a, b, node + 6, kind);
            for k in 0..3 {
                #[allow(clippy::cast_precision_loss)]
                let v = (rng.below(2000) as f32 - 1000.0).to_bits();
                put(a, b, node + 4 * k + 8, v);
            }
        }
        for k in 0..16 {
            #[allow(clippy::cast_precision_loss)]
            let v = (rng.below(200) as f32 / 50.0 - 2.0).to_bits();
            put(a, b, grid + 0x113e0 + 4 * k, v);
        }
        put(a, b, grid + 0x115bc, u8::try_from(rng.below(10)).unwrap());
        put(a, b, abmap, grid);
        // the EE zero register the key's vertex block copies
        let r0 = a.rebase(0x00c8_f508);
        for k in 0..4 {
            put(a, b, r0 + 4 * k, rng.next() & 0x3fff_ffff);
        }
        twin("AbmapDrawNodesEamap", a, b, 0x00a5_1340, &[], |m| {
            let w = m.stack_range().start + 0x400;
            ffx_sg::abmap::bridge::abmap_draw_nodes_eamap(m, w, &mut |m, f, args| {
                m.call(f, Conv::Cdecl, args).unwrap().eax
            });
        });
        // ---- the movie read task: some busy requests, each ended by a
        // TkSyncV; without a task record none
        put(a, b, log, 0u32);
        let chans = a.rebase(0x0112_7c80);
        let task = if rng.below(6) == 0 {
            0
        } else {
            block(a, b, rng, 8)
        };
        if task != 0 {
            put(a, b, task, step);
        }
        let mut busy = Vec::new();
        for i in 0..64u32 {
            let on = task != 0 && rng.below(24) == 0;
            put(a, b, chans + 0x38 * i + 0x31, u8::from(on));
            if on {
                busy.push(chans + 0x38 * i + 0x31);
            }
        }
        let spare = alloc(a, b, 4);
        let ends = alloc(a, b, 4 * (busy.len() + 4));
        for (k, &p) in (0u32..).zip(busy.iter().chain([spare; 4].iter())) {
            put(a, b, ends + 4 * k, p);
        }
        put(a, b, busy_list, ends);
        put(
            a,
            b,
            poll,
            [0u32, 0u32.wrapping_sub(2), 1][rng.below(3) as usize],
        );
        put(a, b, a.rebase(0x0112_8adc), task);
        for (k, v) in [(23, 0u32), (24, 1), (26, 0), (27, 0)] {
            let at = ret(a, k);
            let r = rng.below(3);
            put(a, b, at, if r == 0 { v ^ 1 } else { v });
        }
        twin("MovieCdReadTaskLoop", a, b, 0x0076_e660, &[], |m| {
            cd::movie_cd_read_task_loop(&mut H(m));
        });
    }
}

#[test]
fn cmm1_main_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x00c3_a1a1);
    cmm1_main_cases(&mut a, &mut b, &mut rng, 60);
}

#[test]
fn cmm1_main_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x00c3_a1a2);
    cmm1_main_cases(&mut a, &mut b, &mut rng, 30);
}

// ---- chara-main-map round 2

/// Chara-main-map round 2: `Sg_ChrMovePushApart_0083d3b0` on made-up
/// field characters (up to 8, bunched so they overlap; steps, climbing
/// and pusher flags, floors, the originals of `Sg_ChrSortByZ`,
/// `Sg_VectorGetLengthXZ` and `Sg_ChrPushApart`), `Sg_EventjumpInit`
/// (counts below and above 10) and `HandleSaveDataScreen` (every screen
/// state and answer). Callees outside these are recorders.
#[allow(clippy::too_many_lines)]
fn cmm2_main_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32, poke: bool) {
    use ffx_sg::state::bridge as st;
    use st::save_screen as ss;
    let log = a.rebase(REC_LOG);
    // one recorder per callee (most arguments any caller passes)
    let mut syms: Vec<(u32, u32)> = vec![(st::ID_CHR_MOVE, 1), (st::REQUEST_OWNER_TRIGGER, 2)];
    syms.extend(
        st::EVENTJUMP_INIT_FIRST
            .iter()
            .map(|&(f, v)| (f, u32::from(v.is_some()))),
    );
    for (f, n) in [
        // one word: the second dbgPrintf's format reads none, and the stack
        // word after it is junk (the original's stale argument)
        (st::SG_DBG_PRINTF, 1),
        (st::SET_SCREEN_FADE_COLOR, 4),
        (st::SET_SCREEN_FADE_LAST_REASON, 1),
        (st::LOCK_SCREEN_FADE_COLOR, 0),
        (st::PLAYER_SET_DIR_FIX, 2),
        (st::CH_SYS_DRAW_SET_SITUATION, 1),
        (st::SET_SUIMEN, 1),
        (st::SET_WATER_LIMIT, 1),
        (st::LIGHT_SET, 6),
        (st::CH_ALLOCATE_INIT_ALL, 1),
    ] {
        syms.push((f, n));
    }
    syms.extend(
        st::EVENTJUMP_INIT_LAST
            .iter()
            .map(|&(f, args)| (f, u32::try_from(args.len()).unwrap())),
    );
    for (f, n) in [
        (ss::IS_SCREEN_CLOSED, 0),
        (ss::GET_FADE_LAST_REASON, 0),
        (ss::IS_LOADING_ALBHED, 0),
        (ss::IS_FADE_COLOR_LOCKED, 0),
        (ss::UNLOCK_FADE_COLOR, 0),
        (ss::SET_FADE_ALPHA_MODE, 1),
        (ss::IS_BUSY, 0),
        (ss::INPUT_CLEAR, 0),
        (ss::GET_SCREEN_STATE, 0),
        (ss::TK_GET_CONTROLER_HIST, 3),
        (ss::IS_SAVING_FINISHED, 0),
        (ss::DEBUG_LOAD_ALL, 1),
        (ss::UPDATE_ICON, 1),
        (ss::TO_SAVE, 0),
        (ss::SET_STATE, 1),
        (ss::CLOSE_SAVE_LOAD_SCREEN, 0),
        (ss::SET_SCREEN_STATE, 1),
        (ss::IS_SAVE, 0),
        (ss::PRE_SAVE, 0),
        (ss::IS_SAVE_ENABLED, 0),
        (ss::TO_LOAD, 0),
        (ss::IS_LOADING_FINISHED, 0),
        (ss::GET_DIALOG_STATE, 0),
        (ss::IS_LOADING_SUCCESSFUL, 0),
        (ss::IS_OPERATION_CANCELED, 0),
        (ss::IS_ALBHED_LOADED, 0),
        (ss::MARK_ALBHED_LOADED, 0),
        (ss::IS_DELETING_FINISHED, 0),
        (ss::RESET_TO_IDLE, 0),
        (ss::CLEAR_FORCE_PLACE_ID, 0),
        (ss::LOADED_CHOICE, 0),
        (ss::JUMP_TO_LOADED_SCENE, 0),
    ] {
        syms.push((f, n));
    }
    if poke {
        syms.push((st::CHR_PUSH_APART, 4));
    }
    let mut slot_of: std::collections::BTreeMap<u32, (u32, u32)> =
        std::collections::BTreeMap::new();
    for &(f, n) in &syms {
        let e = slot_of.entry(f).or_insert((0u32, n));
        e.1 = e.1.max(n);
    }
    for (k, (&f, e)) in (100u32..).zip(slot_of.iter_mut()) {
        e.0 = k;
        recorder(a, b, k, f, e.1, 0);
    }
    let cell = |a: &Oracle, f: u32| a.rebase(REC_RETS) + 4 * slot_of[&f].0;
    // poking stand-ins (`poke`): after a countdown (`[ctl]`) they write the
    // word `[ctl + 8]` to `[ctl + 4]`, once; `Sg_ChrPushApart` becomes one
    // too, answering from its cell
    let pokes = alloc(a, b, 16 * 4);
    if poke {
        for (k, f) in [
            st::ID_CHR_MOVE,
            st::REQUEST_OWNER_TRIGGER,
            st::CHR_PUSH_APART,
        ]
        .into_iter()
        .enumerate()
        {
            let ctl = pokes + 16 * u32::try_from(k).unwrap();
            let (slot, n) = slot_of[&f];
            let rec = a.rebase(REC_CODE) + 0x60 * slot;
            let mut c = vec![0x83, 0x3d];
            c.extend(ctl.to_le_bytes());
            c.extend([0x00, 0x74, 0x15]); // cmp dword [ctl], 0; je +0x15
            c.extend([0xff, 0x0d]);
            c.extend(ctl.to_le_bytes()); // dec dword [ctl]
            c.extend([0x75, 0x0d]); // jnz +0xd
            c.push(0xa1);
            c.extend((ctl + 4).to_le_bytes()); // mov eax, [ctl + 4]
            c.extend([0x8b, 0x0d]);
            c.extend((ctl + 8).to_le_bytes()); // mov ecx, [ctl + 8]
            c.extend([0x89, 0x08]); // mov [eax], ecx
            c.push(0xe9); // jmp the recorder
            let here = a.alloc_code(&[0; 64]).unwrap();
            assert_eq!(b.alloc_code(&[0; 64]).unwrap(), here);
            c.extend(
                rec.wrapping_sub(here + u32::try_from(c.len()).unwrap() + 4)
                    .to_le_bytes(),
            );
            for o in [&mut *a, &mut *b] {
                o.write(here, &c).unwrap();
                let at = o.rebase(f);
                let mut j = vec![0xe9];
                j.extend(here.wrapping_sub(at + 5).to_le_bytes());
                o.write(at, &j).unwrap();
            }
            let _ = n;
        }
    }
    // the fade colour: written from three cells
    let rgb = alloc(a, b, 12);
    let mut c = Vec::new();
    for (k, d) in [0x04u8, 0x08, 0x0c].into_iter().enumerate() {
        c.extend([0x8b, 0x44, 0x24, d]); // mov eax, [esp + d]
        c.extend([0x8b, 0x0d]);
        c.extend((rgb + 4 * u32::try_from(k).unwrap()).to_le_bytes()); // mov ecx, [cell]
        c.extend([0x89, 0x08]); // mov [eax], ecx
    }
    c.push(0xc3);
    cmm_stub(a, b, st::GET_SCREEN_FADE_COLOR, &c);
    // dbgPrintf: the format, and the count after Sg_EventjumpInit's first
    // (the other formats read no word)
    let dlog = alloc(a, b, 4 + 8 * 64);
    put(a, b, dlog, dlog + 4);
    let mut c = vec![0x8b, 0x0d];
    c.extend(dlog.to_le_bytes()); // mov ecx, [dlog]
    c.extend([0x8b, 0x44, 0x24, 0x04, 0x89, 0x01, 0x83, 0xc1, 0x04]); // log [esp+4]
    c.push(0x3d);
    c.extend(a.rebase(st::EVENTJUMP_INIT_LOGS[0]).to_le_bytes()); // cmp eax, fmt
    c.extend([0x75, 0x09]); // jne +9
    c.extend([0x8b, 0x44, 0x24, 0x08, 0x89, 0x01, 0x83, 0xc1, 0x04]); // log [esp+8]
    c.extend([0x89, 0x0d]);
    c.extend(dlog.to_le_bytes()); // mov [dlog], ecx
    c.push(0xa1);
    c.extend(cell(a, st::SG_DBG_PRINTF).to_le_bytes()); // mov eax, [answer]
    c.push(0xc3);
    cmm_stub(a, b, st::SG_DBG_PRINTF, &c);
    // the debug save's info: title and name, the description's bytes
    let info_log = alloc(a, b, 4 + 72 * 8);
    cmm_stub(
        a,
        b,
        ss::SET_SAVE_DATA_INFO,
        &arg_logger(info_log, &[(0, 0), (1, 0), (2, 64)], &[0x33, 0xc0, 0xc3]),
    );
    let fcall = &mut |m: &mut H, f: u32, args: &[u32]| {
        let r =
            m.0.call(f, Conv::Cdecl, args)
                .unwrap_or_else(|e| panic!("{f:#x}: {e}"));
        let st0 = r.st0.unwrap_or_else(|| panic!("{f:#x}: no ST0"));
        let pop = m.0.rebase(POP_STUB);
        m.0.call(pop, Conv::Cdecl, &[]).unwrap();
        st0
    };
    let (count_g, array_g) = (a.rebase(0x023c_44e0), a.rebase(0x023c_44e4));
    let (keep_n, keep_a): (u32, u32) = (Mem::read(&*a, count_g), Mem::read(&*a, array_g));
    let within = |rng: &mut Xorshift, lo: f32, hi: f32| {
        #[allow(clippy::cast_precision_loss)]
        let t = rng.below(1 << 16) as f32 / (1 << 16) as f32;
        (lo + (hi - lo) * t).to_bits()
    };
    let mut reach = [0u32; 8];
    for round in 0..rounds {
        // ---- the push-apart pass: a quarter grid at times (exact ties),
        // NaN positions and heights now and then, a few huge reaches, and
        // an active character right past the array's end
        put(a, b, log, 0u32);
        for k in 0..4 {
            put(a, b, pokes + 16 * k, 0u32);
        }
        let n = 2 + rng.below(7);
        let chrs = alloc(a, b, 0x880 * (n as usize + 1));
        let span = [0.5f32, 2.0, 6.0][rng.below(3) as usize];
        let grid = rng.below(3) == 0;
        let val = |rng: &mut Xorshift, lo: f32, hi: f32| {
            if rng.below(24) == 0 {
                [0x7fc0_0000u32, 0xffc0_0000, 0x7fc1_2345][rng.below(3) as usize]
            } else if grid {
                #[allow(
                    clippy::cast_precision_loss,
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss
                )]
                let q = rng.below(((hi - lo) * 4.0) as u32 + 1) as f32 / 4.0;
                (lo + q).to_bits()
            } else {
                within(rng, lo, hi)
            }
        };
        for i in 0..=n {
            let c = chrs + 0x880 * i;
            for k in 0..0x880 / 4 {
                let v = rng.next();
                let v = if v & 0x7f80_0000 == 0x7f80_0000 {
                    v & 0xbfff_ffff
                } else {
                    v
                };
                put(a, b, c + 4 * k, v);
            }
            put(a, b, c + 2, u8::from(i == n || rng.below(8) != 0));
            put(a, b, c + 0x180, u8::from(rng.below(6) == 0));
            put(a, b, c + 0x181, u8::from(rng.below(2) == 0));
            let mut f = 0u32;
            for (bit, p) in [
                (7u32, 3u32),
                (8, 6),
                (9, 3),
                (14, 3),
                (15, 3),
                (30, 2),
                (5, 4),
            ] {
                if rng.below(p) == 0 {
                    f |= 1 << bit;
                }
            }
            put(a, b, c + 0x194, f);
            for k in 0..3 {
                let s = if k == 1 { 0.5 } else { span };
                put(a, b, c + 0xc + 4 * k, val(rng, -s, s));
                put(a, b, c + 0x1c + 4 * k, within(rng, -s, s));
                put(a, b, c + 0x4c + 4 * k, within(rng, -0.25, 0.25));
            }
            let (r0, r1) = if rng.below(8) == 0 {
                (5.0, 12.0)
            } else {
                (0.125, 1.5)
            };
            put(a, b, c + 0x5c, val(rng, r0, r1));
            put(a, b, c + 0x64, val(rng, r0, r1));
            put(
                a,
                b,
                c + 0x4e4,
                if grid {
                    1f32.to_bits()
                } else {
                    within(rng, 0.5, 1.5)
                },
            );
            put(
                a,
                b,
                c + 0x51c,
                [
                    0u32,
                    0x8000_0000,
                    within(rng, -1.0, 1.0),
                    val(rng, -1.0, 1.0),
                ][rng.below(4) as usize],
            );
            put(a, b, c + 0x6a8, val(rng, 0.0, 3.0));
            put(a, b, c + 0x698, val(rng, 0.0, 3.0));
            put(
                a,
                b,
                c + 0x154,
                [0u32, 0x8000_0000, 1f32.to_bits(), 0x7fc0_0000][rng.below(4) as usize],
            );
            put(
                a,
                b,
                c + 0x500,
                [(-1f32).to_bits(), 0, 1f32.to_bits(), 2f32.to_bits()][rng.below(4) as usize],
            );
        }
        put(a, b, count_g, n);
        put(a, b, array_g, chrs);
        if poke {
            // one of the callees changes the counts or the array once
            let push_count = a.rebase(st::PUSH_CHR_COUNT.sym);
            // (stand-in: Sg_IdChrMove 0, the trigger 1, Sg_ChrPushApart 2;
            // a walk in progress only ever grows by the guard character,
            // or it would step past its end)
            let (k, to, v) = [
                (0u32, push_count, 0),
                (0, push_count, 1),
                (0, count_g, n + 1),
                (0, array_g, chrs + 0x880),
                (1, count_g, n + 1),
                (1, array_g, chrs + 0x880),
                (2, push_count, 0),
                (2, push_count, 1),
                (2, count_g, n.saturating_sub(1)),
                (2, count_g, 0),
                (2, array_g, chrs + 0x880),
            ][[9, 5, rng.below(11) as usize, rng.below(11) as usize][rng.below(4) as usize]];
            put(
                a,
                b,
                pokes + 16 * k,
                if k == 1 { 1 } else { 1 + rng.below(3) },
            );
            put(a, b, pokes + 16 * k + 4, to);
            put(a, b, pokes + 16 * k + 8, v);
            let at = cell(a, st::CHR_PUSH_APART);
            put(a, b, at, [0u32, 0x20, 0x20][rng.below(3) as usize]);
        }
        twin(
            &format!("Sg_ChrMovePushApart {round}"),
            a,
            b,
            0x0083_d3b0,
            &[],
            |m| {
                st::chr_move_push_apart(&mut H(m), fcall);
            },
        );
        let logged: u32 = Mem::read(&*b, log);
        reach[0] += u32::from(logged > n);
        // ---- Sg_EventjumpInit
        put(a, b, log, 0u32);
        put(a, b, dlog, dlog + 4);
        for k in 0..3 {
            put(a, b, rgb + 4 * k, within(rng, 0.0, 1.0));
        }
        let at = cell(a, st::SG_DBG_PRINTF);
        put(a, b, at, rng.next());
        let n = [0u32, 5, 9, 10, 11, 0x24, u32::MAX, 0x8000_0000][rng.below(8) as usize];
        twin(
            &format!("Sg_EventjumpInit {n:#x}"),
            a,
            b,
            0x0083_b460,
            &[n],
            |m| {
                let w = m.stack_range().start + 0x400;
                st::eventjump_init(&mut H(m), n, w)
            },
        );
        // ---- HandleSaveDataScreen: random answers, every screen state,
        // eight cases a round
        for case in 0..8 {
            put(a, b, log, 0u32);
            put(a, b, info_log, 0u32);
            for &f in slot_of.keys() {
                let at = cell(a, f);
                let v = [0u32, 0, 1, 1, 2, 3, 4, u32::MAX][rng.below(8) as usize];
                put(a, b, at, v);
            }
            let at = cell(a, ss::GET_SCREEN_STATE);
            put(
                a,
                b,
                at,
                [0u32, 1, 1, 2, 2, 3, 4, 4, 5][rng.below(9) as usize],
            );
            let at = cell(a, ss::TK_GET_CONTROLER_HIST);
            put(
                a,
                b,
                at,
                [0u32, 0x80, 0x7f, 0xff00, 0xffff_ff80][rng.below(5) as usize],
            );
            let at = cell(a, ss::IS_ALBHED_LOADED);
            put(a, b, at, [u32::MAX, 0, 1][rng.below(3) as usize]);
            if rng.below(4) == 0 {
                // a load that finished well
                for (f, v) in [
                    (ss::GET_SCREEN_STATE, 2),
                    (ss::IS_LOADING_FINISHED, 1),
                    (ss::GET_DIALOG_STATE, 0),
                    (ss::IS_LOADING_SUCCESSFUL, 1),
                    (ss::IS_OPERATION_CANCELED, 0),
                ] {
                    let at = cell(a, f);
                    put(a, b, at, v);
                }
            }
            let at = cell(a, ss::LOADED_CHOICE);
            put(a, b, at, [0u32, 1, 2, 3][rng.below(4) as usize]);
            set(
                a,
                b,
                st::SAVE_DEBUG_SLOT,
                [-1i32, -1, 0, 5, 198, 199][rng.below(6) as usize],
            );
            set(a, b, st::SAVE_CLOSE_AFTER_DELETE, rng.below(3));
            set(a, b, st::SAVE_LOADED_PENDING, rng.below(3));
            reach[1 + rng.below(1) as usize] += 1;
            twin(
                &format!("HandleSaveDataScreen {round}.{case}"),
                a,
                b,
                0x0081_ff80,
                &[],
                |m| {
                    // junk where the description goes: a short clear shows
                    let w = m.stack_range().start + 0x400;
                    m.write(w, &[0x55; 0x40]).unwrap();
                    st::handle_save_data_screen(&mut H(m), w);
                },
            );
        }
    }
    if !poke {
        push_apart_crafted(a, b, fcall);
    }
    put(a, b, count_g, keep_n);
    put(a, b, array_g, keep_a);
    eprintln!("cmm2 main reach {reach:?}");
}

#[test]
fn cmm2_main_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x00c3_a2a1);
    cmm2_main_cases(&mut a, &mut b, &mut rng, 120, false);
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    cmm2_main_cases(&mut a, &mut b, &mut rng, 120, true);
}

#[test]
fn cmm2_main_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x00c3_a2a2);
    cmm2_main_cases(&mut a, &mut b, &mut rng, 60, false);
}

/// `Sg_ChrMovePushApart_0083d3b0`'s exact ties, three characters each (at
/// rest, listed, on a quarter grid, x and y 0 unless given):
/// - the z box's edge exactly (a 0.25 reach at 0, another at 1) does not
///   end the walk: a third (z 1.5, reach 2) is still pushed;
/// - a reach sum equal to the distance (0.5 + 0.75, 1.25 apart) is not;
/// - a climber whose height equals the other's top (1.0) still is.
fn push_apart_crafted(
    a: &mut Oracle,
    b: &mut Oracle,
    fcall: &mut impl FnMut(&mut H, u32, &[u32]) -> f64,
) {
    use ffx_sg::state::bridge as st;
    // (x, z, reach, flags, height source +0x6a8, top +0x698)
    type Chr = (f32, f32, f32, u32, f32, f32);
    let cases: [[Chr; 3]; 3] = [
        [
            (0.0, 0.0, 0.25, 0, 0.0, 0.0),
            (0.0, 1.0, 0.25, 0, 0.0, 0.0),
            (0.0, 1.5, 2.0, 0, 0.0, 0.0),
        ],
        [
            (0.0, 0.0, 0.5, 0, 0.0, 0.0),
            (0.75, 1.0, 0.75, 0, 0.0, 0.0),
            (9.0, 9.0, 0.125, 0, 0.0, 0.0),
        ],
        [
            (0.0, 0.0, 0.5, 1 << 30, 0.5, 1.0),
            (0.25, 0.5, 0.5, 1 << 30, 1.0, 2.0),
            (9.0, 9.0, 0.125, 0, 0.0, 0.0),
        ],
    ];
    let (count_g, array_g) = (a.rebase(0x023c_44e0), a.rebase(0x023c_44e4));
    for (k, case) in cases.iter().enumerate() {
        let chrs = alloc(a, b, 0x880 * 3);
        for (i, &(x, z, r, f, h, top)) in (0u32..).zip(case) {
            let c = chrs + 0x880 * i;
            put(a, b, c + 2, 1u8);
            put(a, b, c + 0x194, f | 1 << 7);
            for (o, v) in [
                (0xcu32, x),
                (0x10, 0.0),
                (0x14, z),
                (0x5c, r),
                (0x64, r),
                (0x4e4, 1.0),
                (0x51c, 0.0),
                (0x6a8, h),
                (0x698, top),
                (0x500, 1.0),
            ] {
                put(a, b, c + o, v.to_bits());
            }
        }
        put(a, b, count_g, 3u32);
        put(a, b, array_g, chrs);
        twin(
            &format!("Sg_ChrMovePushApart crafted {k}"),
            a,
            b,
            0x0083_d3b0,
            &[],
            |m| {
                st::chr_move_push_apart(&mut H(m), fcall);
            },
        );
    }
}
