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
