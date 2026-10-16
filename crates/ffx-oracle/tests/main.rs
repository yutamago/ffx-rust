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
