//! The character functions (`ffx_chara`) against the original, on a
//! snapshot of the running game: two oracles started alike, one runs
//! `FFX.exe`'s code, the other the Rust bridge; results and every written
//! byte must agree. The characters are the snapshot's own (`g_ChrArray`)
//! and one made up of random bytes; float arguments include signed zeros,
//! denormals, infinities and quiet NaNs. Needs `FFX_GAME_DIR` and
//! `FFX_SNAPSHOT`; skipped otherwise.
//!
//! No signalling NaNs: the emulator keeps them through `fld m32`, the CPU
//! quiets them, and the bridges do what the CPU does (`ffx_chara::x87`,
//! whose tests check that against the host's x87 unit).

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_chara::chr::bridge as c;
use ffx_chara::handle::bridge as h;
use ffx_chara::layout as l;
use ffx_oracle::{Conv, Oracle, Pages};

/// `fstp st(0); ret` inside `FFX.exe`'s code: pops what a float function
/// left on the x87 stack, so eight calls do not overflow it.
const FPU_POP: u32 = 0x0068_ab21;

fn setup() -> Option<(Oracle, Oracle)> {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    Some((
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
}

/// What a function returns.
#[derive(Debug, Clone, Copy)]
enum Res {
    Void,
    Int(u32),
    Float(f32),
}

fn int(v: i32) -> Res {
    #[allow(clippy::cast_sign_loss)]
    Res::Int(v as u32)
}

/// The two oracles.
struct Twin {
    a: Oracle,
    b: Oracle,
}

impl Twin {
    /// The same bytes in both.
    fn put(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn put_u32(&mut self, at: u32, v: u32) {
        self.put(at, &v.to_le_bytes());
    }

    /// The same allocation in both.
    fn alloc(&mut self, len: usize) -> u32 {
        let x = self.a.alloc(len).unwrap();
        assert_eq!(self.b.alloc(len).unwrap(), x);
        x
    }

    /// Runs the original at `f` (cdecl) in `a` and `rust` in `b`.
    fn check(&mut self, what: &str, f: u32, args: &[u32], rust: impl FnOnce(&mut Oracle) -> Res) {
        let at = self.a.rebase(f);
        let ra = self
            .a
            .call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{what}: {e}"));
        let rb = rust(&mut self.b);
        if ra.st0.is_some() {
            let pop = self.a.rebase(FPU_POP);
            while self.a.call(pop, Conv::Cdecl, &[]).unwrap().st0.is_some() {}
        }
        match rb {
            Res::Void => {}
            Res::Int(v) => assert_eq!(ra.eax, v, "{what}: result"),
            Res::Float(v) => {
                let st0 = ra.st0.unwrap_or_else(|| panic!("{what}: no float result"));
                if v.is_nan() {
                    assert!(st0.is_nan(), "{what}: {st0} for NaN");
                } else {
                    assert_eq!(
                        st0.to_bits(),
                        f64::from(v).to_bits(),
                        "{what}: {st0} vs {v}"
                    );
                }
            }
        }
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
    }
}

fn twin() -> Option<Twin> {
    let (a, b) = setup()?;
    Some(Twin { a, b })
}

/// Floats worth trying, as bits: zeros, ±1, around ±π, large and tiny,
/// denormals, infinities, quiet NaNs.
const FLOATS: [u32; 25] = [
    0x0000_0000,
    0x8000_0000,
    0x3f80_0000,
    0xbf80_0000,
    0x3fc0_0000,
    0x4049_0fd8,
    0x4049_0fd9,
    0xc049_0fd8,
    0xc049_0fd9,
    0x40c9_0fd8,
    0x4120_0000,
    0xc2c8_0000,
    0x447a_0000,
    0x0000_0001,
    0x8000_0001,
    0x0080_0000,
    0x7f7f_ffff,
    0xff7f_ffff,
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_0000,
    0xffc0_0000,
    0xffc0_4321,
    0x7fff_ffff,
    0x7fc1_2345,
];

/// Integers worth trying.
const INTS: [u32; 17] = [
    0,
    1,
    2,
    3,
    0x7f,
    0x80,
    0xff,
    0x100,
    0x101,
    0x7fff,
    0x8000,
    0xffff,
    0x1_0000,
    0x7fff_ffff,
    0x8000_0000,
    0xffff_fffe,
    0xffff_ffff,
];

fn fl(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Random float bits, signalling NaNs made quiet.
fn quiet(bits: u32) -> u32 {
    if ffx_chara::x87::is_signalling(fl(bits)) {
        bits | 0x0040_0000
    } else {
        bits
    }
}

/// A value in emulated memory (`Mem::read`; `Oracle::read` is the
/// emulator's own).
fn rd<T: ffx_bridge::Value>(o: &Oracle, at: u32) -> T {
    Mem::read(o, at)
}

/// The snapshot's characters with a model block (active or not), and one
/// of random bytes with random blocks and models behind it.
fn characters(t: &mut Twin, rng: &mut Xorshift) -> Vec<u32> {
    let count = h::CHR_COUNT.get(&t.b);
    let array = h::CHR_ARRAY.get(&t.b);
    let mut out = Vec::new();
    for i in 0..u32::try_from(count).unwrap() {
        let chr = array + i * l::SIZE;
        let blk: u32 = rd(&t.b, chr + l::BLK);
        if blk != 0 && blk != 0xcdcd_cdcd {
            out.push(chr);
        }
    }
    assert!(!out.is_empty(), "no characters in the snapshot");
    // random words, none of them a signalling NaN
    let mut random = |len: u32| -> Vec<u8> {
        (0..len / 4)
            .flat_map(|_| quiet(rng.next()).to_le_bytes())
            .collect()
    };
    let chr = t.alloc(l::SIZE as usize);
    let mut bytes = random(l::SIZE);
    let blk = t.alloc(l::BLK_SIZE as usize);
    t.put(blk, &random(l::BLK_SIZE));
    let models = t.alloc(4 * l::MODEL_SIZE as usize);
    t.put(models, &random(4 * l::MODEL_SIZE));
    let header = t.alloc(8);
    t.put(header, &[0, 0, 0, 0, 0, 0, 4, 0]);
    // a sane angle: the wrap loops for ever on infinities
    let rot = 1.0f32.to_bits();
    for (off, v) in [
        (l::BLK, blk),
        (l::MODELS, models),
        (l::MODEL_HEADER, header),
        (l::ROT, rot),
    ] {
        let o = off as usize;
        bytes[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    t.put(chr, &bytes);
    out.push(chr);
    out
}

/// A float field read by a getter: (name, address, field, rust).
type FloatGetter = (
    &'static str,
    u32,
    fn(&Oracle, u32) -> u32,
    fn(&Oracle, u32) -> f32,
);

fn in_blk(o: &Oracle, chr: u32, off: u32) -> u32 {
    rd::<u32>(o, chr + l::BLK) + off
}

/// A flag setter: (name, address, word, rust).
type Flag = (&'static str, u32, u32, fn(&mut Oracle, u32, u32) -> u32);

/// A float setter: (name, address, rust).
type FloatSet = (&'static str, u32, fn(&mut Oracle, u32, f32) -> Res);

/// An integer setter: (name, address, rust).
type IntSet = (&'static str, u32, fn(&mut Oracle, u32, u32) -> Res);

fn float_getters() -> [FloatGetter; 14] {
    [
        (
            "Ch_GetCamHeight",
            0x0082_ab30,
            |_, x| x + l::CAM_HEIGHT,
            |m, x| c::ch_get_cam_height(m, x),
        ),
        (
            "Ch_GetDataScale",
            0x0082_ab60,
            |o, x| in_blk(o, x, l::BLK_DATA_SCALE),
            |m, x| c::ch_get_data_scale(m, x),
        ),
        (
            "Ch_GetDir",
            0x0082_ab80,
            |_, x| x + l::DIR,
            |m, x| c::ch_get_dir(m, x),
        ),
        (
            "Ch_GetFixHeight",
            0x0082_aba0,
            |_, x| x + l::FIX_HEIGHT,
            |m, x| c::ch_get_fix_height(m, x),
        ),
        (
            "Ch_GetGrav",
            0x0082_abb0,
            |_, x| x + l::GRAV,
            |m, x| c::ch_get_grav(m, x),
        ),
        (
            "Ch_GetJimen",
            0x0082_ac40,
            |_, x| x + l::JIMEN,
            |m, x| c::ch_get_jimen(m, x),
        ),
        (
            "Ch_GetScaleX",
            0x0082_ad30,
            |_, x| x + l::SCALE,
            |m, x| c::ch_get_scale_axis(m, x, 0),
        ),
        (
            "Ch_GetScaleY",
            0x0082_ad40,
            |_, x| x + l::SCALE + 4,
            |m, x| c::ch_get_scale_axis(m, x, 1),
        ),
        (
            "Ch_GetScaleZ",
            0x0082_ad50,
            |_, x| x + l::SCALE + 8,
            |m, x| c::ch_get_scale_axis(m, x, 2),
        ),
        (
            "Ch_GetSp",
            0x0082_ad60,
            |_, x| x + l::SP,
            |m, x| c::ch_get_sp(m, x),
        ),
        (
            "Ch_GetThickness",
            0x0082_ad70,
            |o, x| in_blk(o, x, l::BLK_THICKNESS),
            |m, x| c::ch_get_thickness(m, x),
        ),
        (
            "Ch_GetThicknessBtl",
            0x0082_ad90,
            |o, x| in_blk(o, x, l::BLK_THICKNESS_BTL),
            |m, x| c::ch_get_thickness_btl(m, x),
        ),
        (
            "Ch_GetThicknessBtlCam",
            0x0082_adb0,
            |_, x| x + l::THICKNESS_BTL_CAM,
            |m, x| c::ch_get_thickness_btl_cam(m, x),
        ),
        (
            "Ch_GetRot",
            0x0082_ace0,
            |_, x| x + l::ROT,
            |m, x| c::ch_get_rot(m, x),
        ),
    ]
}

fn int_getters() -> [IntGetter; 19] {
    [
        (
            "Ch_EffectGetEffectLevel",
            0x0082_a9a0,
            l::EFFECT_LEVEL,
            |m, x| Res::Int(c::ch_effect_get_effect_level(m, x)),
        ),
        ("Ch_GetBodyHit", 0x0082_ab10, l::FLAGS, |m, x| {
            Res::Int(c::ch_get_body_hit(m, x))
        }),
        ("Ch_GetClip", 0x0082_ab40, l::FLAGS, |m, x| {
            Res::Int(c::ch_get_clip(m, x))
        }),
        ("Ch_GetFieldMode", 0x0082_ab90, l::FIELD_MODE, |m, x| {
            int(c::ch_get_field_mode(m, x))
        }),
        ("Ch_GetGravMode", 0x0082_abc0, l::GRAV_MODE, |m, x| {
            int(c::ch_get_grav_mode(m, x))
        }),
        ("Ch_GetHide", 0x0082_abd0, l::HIDE, |m, x| {
            int(c::ch_get_hide(m, x))
        }),
        ("Ch_GetHideCalc", 0x0082_abe0, l::HIDE_CALC, |m, x| {
            Res::Int(c::ch_get_hide_calc(m, x))
        }),
        ("Ch_GetHideCalc", 0x0082_abe0, l::FLAGS, |m, x| {
            Res::Int(c::ch_get_hide_calc(m, x))
        }),
        ("Ch_GetHideEvent", 0x0082_ac20, l::HIDE, |m, x| {
            Res::Int(c::ch_get_hide_event(m, x))
        }),
        ("Ch_GetMotionSpeed", 0x0082_ac50, l::MOTION_SPEED, |m, x| {
            int(c::ch_get_motion_speed(m, x))
        }),
        ("Ch_GetOrgMatrix", 0x0082_ac60, l::ORG_MATRIX, |m, x| {
            Res::Int(c::ch_get_org_matrix(m, x))
        }),
        ("Ch_GetOverlapHit", 0x0082_ac70, l::FLAGS, |m, x| {
            Res::Int(c::ch_get_overlap_hit(m, x))
        }),
        ("Ch_GetResHide", 0x0082_acd0, l::HIDE, |m, x| {
            int(c::ch_get_hide(m, x))
        }),
        ("Ch_IdGetPNum", 0x0082_adc0, l::PNUM, |m, x| {
            int(c::ch_id_get_p_num(m, x))
        }),
        (
            "Ch_TextureGetAnimId",
            0x0082_b960,
            l::TEXTURE_ANIM_ID,
            |m, x| Res::Int(c::ch_texture_get_anim_id(m, x)),
        ),
        (
            "Ch_GetChrRenderInstance",
            0x0082_6b70,
            l::RENDER_INSTANCE,
            |m, x| Res::Int(h::ch_get_chr_render_instance(m, x)),
        ),
        ("Ch_GetId", 0x0082_6b80, l::ID, |m, x| {
            Res::Int(h::ch_get_id(m, x))
        }),
        ("Ch_GetName", 0x0082_6b90, l::NAME, |m, x| {
            Res::Int(h::ch_get_name(m, x))
        }),
        ("Ch_GetBlkDirName", 0x0082_6a60, l::BLK, |m, x| {
            Res::Int(h::ch_get_blk_dir_name(m, x))
        }),
    ]
}

fn flag_setters() -> [Flag; 11] {
    [
        ("Ch_EnBodyHit", 0x0082_a9f0, l::FLAGS, c::ch_en_body_hit),
        ("Ch_EnClip", 0x0082_aa30, l::FLAGS, c::ch_en_clip),
        ("Ch_EnMapID", 0x0082_aa70, l::FLAGS, c::ch_en_map_id),
        ("Ch_EnRotHokan", 0x0082_aae0, l::FLAGS, c::ch_en_rot_hokan),
        (
            "Ch_IdEnableShade",
            0x0083_40f0,
            l::FLAGS,
            c::ch_id_enable_shade,
        ),
        ("Ch_SetChocobo", 0x0082_b130, l::FLAGS2, c::ch_set_chocobo),
        (
            "Ch_SetOverlapHit",
            0x0082_b420,
            l::FLAGS,
            c::ch_set_overlap_hit,
        ),
        (
            "Ch_Vu1SetClipmode",
            0x0082_b970,
            l::FLAGS2,
            c::ch_vu1_set_clipmode,
        ),
        (
            "Ch_SetHideBattle",
            0x0082_b260,
            l::HIDE,
            c::ch_set_hide_battle,
        ),
        (
            "Ch_SetHideEffect",
            0x0082_b2a0,
            l::HIDE,
            c::ch_set_hide_effect,
        ),
        (
            "Ch_SetHideBattle",
            0x0082_b260,
            l::DIRTY,
            c::ch_set_hide_battle,
        ),
    ]
}

fn float_setters() -> [FloatSet; 15] {
    [
        ("Ch_AddGrav", 0x0082_a890, |m, x, v| {
            c::ch_add_grav(m, x, v);
            Res::Void
        }),
        ("Ch_MoveSetChgRun", 0x0082_b040, |m, x, v| {
            Res::Float(c::ch_move_set_chg_run(m, x, v))
        }),
        ("Ch_SetCamHeight", 0x0082_b0f0, |m, x, v| {
            c::ch_set_cam_height(m, x, v);
            Res::Void
        }),
        ("Ch_SetDir", 0x0082_b190, |m, x, v| {
            c::ch_set_f32(m, x, l::DIR, v);
            Res::Void
        }),
        ("Ch_SetFixHeight", 0x0082_b200, |m, x, v| {
            c::ch_set_fix_height(m, x, v);
            Res::Void
        }),
        ("Ch_SetHitHeight", 0x0082_b320, |m, x, v| {
            c::ch_set_f32(m, x, l::HIT_HEIGHT, v);
            Res::Void
        }),
        ("Ch_SetHiju", 0x0082_b340, |m, x, v| {
            c::ch_set_f32(m, x, l::HIJU, v);
            Res::Void
        }),
        ("Ch_SetJimen", 0x0082_b360, |m, x, v| {
            c::ch_set_f32(m, x, l::JIMEN, v);
            Res::Void
        }),
        ("Ch_SetOrgMatrixScale", 0x0082_b400, |m, x, v| {
            c::ch_set_f32(m, x, l::ORG_MATRIX_SCALE, v);
            Res::Void
        }),
        ("Ch_SetScaleAll", 0x0082_b590, |m, x, v| {
            c::ch_set_scale_all(m, x, v);
            Res::Void
        }),
        ("Ch_SetSp", 0x0082_b840, |m, x, v| {
            c::ch_set_f32(m, x, l::SP, v);
            Res::Void
        }),
        ("Ch_SetThickness", 0x0082_b880, |m, x, v| {
            Res::Float(c::ch_set_thickness(m, x, v))
        }),
        ("Ch_SetThicknessBtlCam", 0x0082_b8c0, |m, x, v| {
            c::ch_set_thickness_btl_cam(m, x, v);
            Res::Void
        }),
        ("Ch_SetVelY", 0x0082_b920, |m, x, v| {
            c::ch_set_vel_y(m, x, v);
            Res::Void
        }),
        ("Ch_SetWeight", 0x0082_b940, |m, x, v| {
            Res::Float(c::ch_set_weight(m, x, v))
        }),
    ]
}

fn int_setters() -> [IntSet; 13] {
    [
        ("Ch_EffectSetEffectLevel", 0x0082_a9d0, |m, x, v| {
            c::ch_effect_set_effect_level(m, x, v);
            Res::Void
        }),
        ("Ch_MotionSetHokan", 0x0082_afb0, |m, x, v| {
            c::ch_motion_set_hokan(m, x, v);
            Res::Void
        }),
        ("Ch_MotionSetNextHokan", 0x0082_b010, |m, x, v| {
            #[allow(clippy::cast_possible_wrap)]
            c::ch_motion_set_next_hokan(m, x, v as i32);
            Res::Void
        }),
        ("Ch_SetBattleChrNum", 0x0082_b0d0, |m, x, v| {
            c::ch_set_battle_chr_num(m, x, v);
            Res::Void
        }),
        ("Ch_SetFieldMode", 0x0082_b1e0, |m, x, v| {
            c::ch_set_u8(m, x, l::FIELD_MODE, v);
            Res::Void
        }),
        ("Ch_SetGravMode", 0x0082_b240, |m, x, v| {
            c::ch_set_u8(m, x, l::GRAV_MODE, v);
            Res::Void
        }),
        ("Ch_SetHideCalc", 0x0082_b280, |m, x, v| {
            c::ch_set_u8(m, x, l::HIDE_CALC, v);
            Res::Void
        }),
        ("Ch_SetMotionType", 0x0082_b3a0, |m, x, v| {
            c::ch_set_u8(m, x, l::MOTION_TYPE, v);
            Res::Void
        }),
        ("Ch_SetMotionSpeed", 0x0082_b380, |m, x, v| {
            c::ch_set_motion_speed(m, x, v);
            Res::Void
        }),
        ("Ch_SetOrgMatrix", 0x0082_b3e0, |m, x, v| {
            c::ch_set_org_matrix(m, x, v);
            Res::Void
        }),
        ("Ch_SetShadowMode", 0x0082_b820, |m, x, v| {
            int(c::ch_set_shadow_mode(m, x, v))
        }),
        ("Ch_ResetMotion", 0x0082_b080, |m, x, _| {
            c::ch_reset_motion(m, x);
            Res::Void
        }),
        ("Ch_ResetPos", 0x0082_b0b0, |m, x, _| {
            c::ch_reset_pos(m, x);
            Res::Void
        }),
    ]
}

#[test]
fn float_getters_match_the_original() {
    let Some(mut t) = twin() else { return };
    let mut rng = Xorshift(0x2545_f491);
    let getters = float_getters();
    for chr in characters(&mut t, &mut rng) {
        for (name, f, field, rust) in getters {
            // the game's own value first
            t.check(&format!("{name} {chr:#x}"), f, &[chr], |m| {
                Res::Float(rust(m, chr))
            });
            for bits in FLOATS {
                let v = fl(bits);
                // the wrap loops for ever on infinities, like the original
                if name == "Ch_GetRot" && !(v.abs() <= 1.0e4 || v.is_nan()) {
                    continue;
                }
                let a = field(&t.b, chr);
                t.put_u32(a, bits);
                t.check(&format!("{name} {chr:#x} {bits:#x}"), f, &[chr], |m| {
                    Res::Float(rust(m, chr))
                });
            }
        }
        // angles over many turns, both ways
        for _ in 0..200 {
            #[allow(clippy::cast_precision_loss)]
            let v = (rng.next() % 200_000) as f32 / 10.0 - 10_000.0;
            t.put_u32(chr + l::ROT, v.to_bits());
            t.check(
                &format!("Ch_GetRot {chr:#x} {v}"),
                0x0082_ace0,
                &[chr],
                |m| Res::Float(c::ch_get_rot(m, chr)),
            );
        }
    }
}

/// An integer field read by a getter: (name, address, field, rust).
type IntGetter = (&'static str, u32, u32, fn(&Oracle, u32) -> Res);

#[test]
fn int_getters_match_the_original() {
    let Some(mut t) = twin() else { return };
    let mut rng = Xorshift(0x1234_5678);
    let getters = int_getters();
    for chr in characters(&mut t, &mut rng) {
        // the game's own values first
        for (name, f, _, rust) in getters {
            t.check(&format!("{name} {chr:#x}"), f, &[chr], |m| rust(m, chr));
        }
        // with the hide byte clear, Ch_GetHideCalc reads the alpha
        for bits in FLOATS {
            t.put(chr + l::HIDE_CALC, &[0]);
            let w: u32 = rd(&t.b, chr + l::FLAGS);
            t.put_u32(chr + l::FLAGS, w & !l::F_HIDE_CALC);
            t.put_u32(chr + l::FADE_ALPHA, bits);
            t.check(
                &format!("Ch_GetHideCalc {chr:#x} alpha {bits:#x}"),
                0x0082_abe0,
                &[chr],
                |m| Res::Int(c::ch_get_hide_calc(m, chr)),
            );
        }
        let saved_blk: u32 = rd(&t.b, chr + l::BLK);
        for (name, f, field, rust) in getters {
            let values: Vec<u32> = INTS
                .iter()
                .copied()
                .chain((0..8).map(|_| rng.next()))
                .collect();
            for v in values {
                t.put_u32(chr + field, v);
                t.check(&format!("{name} {chr:#x} {v:#x}"), f, &[chr], |m| {
                    rust(m, chr)
                });
            }
            if field == l::BLK {
                t.put_u32(chr + l::BLK, saved_blk);
            }
        }
        t.put_u32(chr + l::BLK, saved_blk);
        // models: hidden, normals, vertices
        let header: u32 = rd(&t.b, chr + l::MODEL_HEADER);
        let n = u32::from(rd::<u16>(&t.b, header + 6));
        for i in 0..n.min(16) {
            t.check(
                &format!("Ch_ModelGetHide {chr:#x} {i}"),
                0x0082_add0,
                &[chr, i],
                |m| int(c::ch_model_get_hide(m, chr, i)),
            );
            t.check(
                &format!("Ch_ModelGetNromPtr {chr:#x} {i}"),
                0x0082_adf0,
                &[chr, i],
                |m| Res::Int(c::ch_model_get_nrom_ptr(m, chr, i)),
            );
            t.check(
                &format!("Ch_ModelGetVtexPtr {chr:#x} {i}"),
                0x0082_ae10,
                &[chr, i],
                |m| Res::Int(c::ch_model_get_vtex_ptr(m, chr, i)),
            );
        }
        // the model block's resident pointer
        t.check(
            &format!("Ch_EffectGetResidentPtr {chr:#x}"),
            0x0082_a9b0,
            &[chr],
            |m| Res::Int(c::ch_effect_get_resident_ptr(m, chr)),
        );
    }
}

#[test]
fn outputs_match_the_original() {
    let Some(mut t) = twin() else { return };
    let mut rng = Xorshift(0x0bad_cafe);
    let out = t.alloc(16);
    for chr in characters(&mut t, &mut rng) {
        for (name, f, field) in [
            ("Ch_GetPos", 0x0082_ac90, l::POS),
            ("Ch_GetScale", 0x0082_ad00, l::SCALE),
        ] {
            for mask in 0..8u32 {
                for bits in [0x3f80_0000, 0x7fc0_0001, 0x8000_0001, quiet(rng.next())] {
                    for i in 0..3 {
                        t.put_u32(chr + field + 4 * i, bits.wrapping_add(i));
                    }
                    let outs: Vec<u32> = (0..3)
                        .map(|i| if mask & (1 << i) != 0 { out + 4 * i } else { 0 })
                        .collect();
                    let args = [chr, outs[0], outs[1], outs[2]];
                    t.check(
                        &format!("{name} {chr:#x} {mask} {bits:#x}"),
                        f,
                        &args,
                        |m| {
                            if f == 0x0082_ac90 {
                                c::ch_get_pos(m, chr, outs[0], outs[1], outs[2]);
                            } else {
                                c::ch_get_scale(m, chr, outs[0], outs[1], outs[2]);
                            }
                            Res::Void
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn flags_match_the_original() {
    let Some(mut t) = twin() else { return };
    let mut rng = Xorshift(0x5eed_0001);
    let flags = flag_setters();
    for chr in characters(&mut t, &mut rng) {
        for (name, f, field, rust) in flags {
            for word in [0, u32::MAX, rng.next(), rng.next()] {
                for on in INTS {
                    t.put_u32(chr + field, word);
                    t.check(
                        &format!("{name} {chr:#x} {word:#x} {on:#x}"),
                        f,
                        &[chr, on],
                        |m| Res::Int(rust(m, chr, on)),
                    );
                }
            }
        }
        for hide in [0u8, 0x80, 0xff, 0x13, 0x7f] {
            for on in [0, 1, 0x100] {
                for mask in [0, 1, 2, 0x10, 0x80, 0x100, 0xff, 0xffff_ff00, u32::MAX] {
                    t.put(chr + l::HIDE, &[hide]);
                    t.check(
                        &format!("Chr_SetHideFlags {chr:#x} {hide:#x} {on} {mask:#x}"),
                        0x0082_ba30,
                        &[chr, on, mask],
                        |m| Res::Int(c::chr_set_hide_flags(m, chr, on, mask)),
                    );
                }
            }
        }
    }
}

#[test]
fn float_setters_match_the_original() {
    let Some(mut t) = twin() else { return };
    let mut rng = Xorshift(0x5eed_0002);
    let floats = float_setters();
    let chrs = characters(&mut t, &mut rng);
    for &chr in &chrs {
        for (name, f, rust) in floats {
            for bits in FLOATS
                .iter()
                .copied()
                .chain((0..8).map(|_| quiet(rng.next())))
            {
                t.check(
                    &format!("{name} {chr:#x} {bits:#x}"),
                    f,
                    &[chr, bits],
                    |m| rust(m, chr, fl(bits)),
                );
            }
        }
        // the gravity sum with every pair, NaNs included
        for g in FLOATS {
            for x in FLOATS {
                t.put_u32(chr + l::GRAV, g);
                t.check(
                    &format!("Ch_AddGrav {chr:#x} {g:#x} + {x:#x}"),
                    0x0082_a890,
                    &[chr, x],
                    |m| {
                        c::ch_add_grav(m, chr, fl(x));
                        Res::Void
                    },
                );
            }
        }
    }
}

#[test]
fn vector_setters_match_the_original() {
    let Some(mut t) = twin() else { return };
    let mut rng = Xorshift(0x5eed_0005);
    for chr in characters(&mut t, &mut rng) {
        // three floats
        for _ in 0..12 {
            let bits: Vec<u32> = (0..3)
                .map(|_| FLOATS[rng.next() as usize % FLOATS.len()])
                .collect();
            let [px, py, pz] = [fl(bits[0]), fl(bits[1]), fl(bits[2])];
            t.check(
                &format!("Ch_SetOffset {chr:#x} {bits:x?}"),
                0x0082_b3c0,
                &[chr, bits[0], bits[1], bits[2]],
                |m| {
                    c::ch_set_offset(m, chr, px, py, pz);
                    Res::Void
                },
            );
            t.check(
                &format!("Ch_SetShadeColEffect {chr:#x} {bits:x?}"),
                0x0082_b6e0,
                &[chr, bits[0], bits[1], bits[2]],
                |m| {
                    c::ch_set_shade_col_effect(m, chr, px, py, pz);
                    Res::Void
                },
            );
            t.check(
                &format!("Ch_SetPos {chr:#x} {bits:x?}"),
                0x0082_b440,
                &[chr, bits[0], bits[2]],
                |m| {
                    c::ch_set_pos(m, chr, px, pz);
                    Res::Void
                },
            );
            for skip in [0u8, 1, 0x80] {
                t.put(t.a.rebase(c::SKIP_SET_POS3.sym), &[skip]);
                t.check(
                    &format!("Ch_SetPos3 {chr:#x} {skip} {bits:x?}"),
                    0x0082_b480,
                    &[chr, bits[0], bits[1], bits[2]],
                    |m| {
                        c::ch_set_pos3(m, chr, px, py, pz);
                        Res::Void
                    },
                );
            }
            for axes in [0, 1, 2, 4, 5, 7, 0xf8, 0x107] {
                t.check(
                    &format!("Ch_SetScale {chr:#x} {axes} {bits:x?}"),
                    0x0082_b560,
                    &[chr, axes, bits[0], bits[1], bits[2]],
                    |m| {
                        c::ch_set_scale(m, chr, axes, px, py, pz);
                        Res::Void
                    },
                );
            }
        }
    }
}

#[test]
fn other_setters_match_the_original() {
    let Some(mut t) = twin() else { return };
    let mut rng = Xorshift(0x5eed_0006);
    let ints = int_setters();
    for chr in characters(&mut t, &mut rng) {
        // fades: target and frames
        for (name, f, field) in [
            ("Ch_SetTransparent", 0x0082_b900, l::FADE_ALPHA),
            ("Ch_SetShade", 0x0082_b5b0, l::FADE_SHADE),
            ("Ch_SetSpecularAlpha", 0x0082_b860, l::FADE_SPECULAR),
        ] {
            for bits in [0x3f80_0000, 0x7fc0_0001, 0, 0xbf00_0000] {
                for frames in [0, 1, 60, u32::MAX, 0x8000_0000] {
                    #[allow(clippy::cast_possible_wrap)]
                    let n = frames as i32;
                    t.check(
                        &format!("{name} {chr:#x} {bits:#x} {frames}"),
                        f,
                        &[chr, bits, frames],
                        |m| {
                            c::ch_set_fade(m, chr, field, fl(bits), n);
                            Res::Void
                        },
                    );
                }
            }
        }
        for (name, f, rust) in ints {
            for v in INTS.iter().copied().chain((0..8).map(|_| rng.next())) {
                t.check(&format!("{name} {chr:#x} {v:#x}"), f, &[chr, v], |m| {
                    rust(m, chr, v)
                });
            }
        }
        // motions by slot; -1 takes the default
        for slot in 0..8 {
            for motion in [u32::MAX, 0, 5, 0x1234, 0xffff_fffe] {
                t.check(
                    &format!("Ch_MotionSetMoves {chr:#x} {slot} {motion:#x}"),
                    0x0082_afe0,
                    &[chr, slot, motion],
                    |m| {
                        c::ch_motion_set_moves(m, chr, slot, motion);
                        Res::Void
                    },
                );
            }
        }
    }
}

#[test]
fn handles_match_the_original() {
    let Some(mut t) = twin() else { return };
    let mut rng = Xorshift(0x5eed_0003);
    let chrs = characters(&mut t, &mut rng);
    let array = h::CHR_ARRAY.get(&t.b);
    let count = u32::try_from(h::CHR_COUNT.get(&t.b)).unwrap();
    let mut candidates = vec![0, 1, array, array + 1, array + count * l::SIZE, rng.next()];
    for i in 0..count {
        candidates.push(array + i * l::SIZE);
    }
    candidates.extend(&chrs);
    for &p in &candidates {
        t.check(&format!("Ch_CheckHandle {p:#x}"), 0x0082_61b0, &[p], |m| {
            int(h::ch_check_handle(m, p))
        });
    }
    // the walk, from every start
    for start in 0..=count + 1 {
        t.put_u32(t.a.rebase(h::HANDLE_CURSOR.sym), start);
        loop {
            let at = h::HANDLE_CURSOR.get(&t.b);
            let mut got = 0;
            t.check(
                &format!("Ch_HandleGetNext from {at}"),
                0x0082_6d70,
                &[],
                |m| {
                    got = h::ch_handle_get_next(m);
                    Res::Int(got)
                },
            );
            if got == 0 {
                break;
            }
        }
        t.check("Ch_HandleRewind", 0x0082_6db0, &[], |m| {
            h::ch_handle_rewind(m);
            Res::Void
        });
    }
    // model blocks by their first word
    let blk0 = t.b.rebase(h::CHR_BLK.sym);
    let mut keys = vec![0, 0xcdcd_cdcd, rng.next(), u32::MAX];
    for i in 0..l::BLKS {
        keys.push(rd(&t.b, blk0 + i * l::BLK_SIZE));
    }
    for &k in &keys {
        t.check(
            &format!("Ch_BlkGetHandleFast {k:#x}"),
            0x0082_5ea0,
            &[k],
            |m| Res::Int(h::ch_blk_get_handle_fast(m, k)),
        );
    }
    // render instances by an address in a character's resident data
    let mut ps = vec![0, 1, u32::MAX, rng.next()];
    for &chr in &chrs {
        let blk: u32 = rd(&t.b, chr + l::BLK);
        let start: u32 = rd(&t.b, blk + l::BLK_RESIDENT);
        let size: u32 = rd(&t.b, blk + l::BLK_RESIDENT_SIZE);
        ps.extend([
            start,
            start.wrapping_sub(1),
            start.wrapping_add(size / 2),
            start.wrapping_add(size).wrapping_sub(1),
            start.wrapping_add(size),
        ]);
    }
    let f = 0x0082_6af0;
    for &p in &ps {
        t.check(
            &format!("Ch_GetChrRenderInsatanceFromOEF2Content {p:#x}"),
            f,
            &[p],
            |m| Res::Int(h::ch_get_chr_render_instance_from_oef2_content(m, p)),
        );
    }
    // again with a character excluded, one inactive and one without an instance
    for i in 0..count.min(4) {
        let chr = array + i * l::SIZE;
        let w: u32 = rd(&t.b, chr + l::FLAGS2);
        match i {
            0 => t.put_u32(chr + l::FLAGS2, w | l::F2_NO_OEF2),
            1 => t.put(chr + l::ACTIVE, &[0]),
            _ => t.put_u32(chr + l::RENDER_INSTANCE, 0),
        }
    }
    for &p in &ps {
        t.check(
            &format!("Ch_GetChrRenderInsatanceFromOEF2Content' {p:#x}"),
            f,
            &[p],
            |m| Res::Int(h::ch_get_chr_render_instance_from_oef2_content(m, p)),
        );
    }
    t.check("Ch_FunctionHistoryInit", 0x0082_6a40, &[], |m| {
        h::ch_function_history_init(m);
        Res::Void
    });
}

#[test]
fn helpers_match_the_original() {
    let Some(mut t) = twin() else { return };
    let mut rng = Xorshift(0x5eed_0004);
    // the angle wrap on its own, both sides of every bound
    let mut angles: Vec<u32> = FLOATS
        .iter()
        .copied()
        .filter(|&b| fl(b).is_nan() || fl(b).abs() <= 1.0e4)
        .collect();
    for b in [0x4049_0fd8u32, 0xc049_0fd8, 0x40c9_0fd8, 0xc0c9_0fd8] {
        angles.extend([b - 1, b + 1, b - 0x100, b + 0x100]);
    }
    for _ in 0..300 {
        #[allow(clippy::cast_precision_loss)]
        let v = (rng.next() % 2_000_000) as f32 / 100.0 - 10_000.0;
        angles.push(v.to_bits());
    }
    for a in angles {
        t.check(&format!("Chr_WrapAngle {a:#x}"), 0x0082_ba70, &[a], |_| {
            Res::Float(ffx_chara::chr::wrap_angle(fl(a)))
        });
    }
    // a fade anywhere
    let fade = t.alloc(12);
    for bits in [0x3f80_0000, 0x7fc0_0001, 0x8000_0000, quiet(rng.next())] {
        for frames in [0, 1, 30, u32::MAX, 0x8000_0000] {
            #[allow(clippy::cast_possible_wrap)]
            let n = frames as i32;
            let start = [quiet(rng.next()), quiet(rng.next()), rng.next()];
            for (i, w) in (0u32..).zip(start) {
                t.put_u32(fade + 4 * i, w);
            }
            let args = [fade, bits, frames];
            t.check(
                &format!("Chr_FadeSet {bits:#x} {frames}"),
                0x0082_b990,
                &args,
                |m| {
                    c::chr_fade_set(m, fade, fl(bits), n);
                    Res::Void
                },
            );
        }
    }
}
