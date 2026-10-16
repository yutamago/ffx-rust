//! The character functions (`ffx_chara`) against the original: two
//! oracles started alike, one runs `FFX.exe`'s code, the other the Rust
//! bridge; results and every written byte must agree. Every test runs on
//! the image alone with a made-up character array (`FFX_GAME_DIR`), then
//! on a snapshot of the running game when `FFX_SNAPSHOT` is set: the
//! snapshot's own characters (`g_ChrArray`) and one more of random bytes.
//! Float arguments include signed zeros, denormals, infinities and quiet
//! NaNs.
//!
//! No signalling NaNs: the emulator keeps them through `fld m32`, the CPU
//! quiets them, and the bridges do what the CPU does (`ffx_chara::x87`,
//! whose tests check that against the host's x87 unit).

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_chara::chr::bridge as c;
use ffx_chara::handle::bridge as h;
use ffx_chara::layout as l;
use ffx_chara::parts::bridge as p;
use ffx_chara::sys::bridge as sy;
use ffx_chara::texture::bridge as tx;
use ffx_chara::{fixed, id::bridge as id};
use ffx_oracle::{Conv, Oracle, Pages};

/// `fstp st(0); ret` inside `FFX.exe`'s code: pops what a float function
/// left on the x87 stack, so eight calls do not overflow it.
const FPU_POP: u32 = 0x0068_ab21;

/// The oracles to run on: the image alone with a made-up character array
/// (needs `FFX_GAME_DIR`), then the snapshot when `FFX_SNAPSHOT` is set.
fn twins() -> Vec<Twin> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return Vec::new();
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut a = Oracle::load(&exe).unwrap();
    let mut b = Oracle::load(&exe).unwrap();
    for o in [&mut a, &mut b] {
        // the game thread's control word, as the snapshot oracle sets it
        o.set_fpu_control(0x027f).unwrap();
    }
    let mut t = Twin {
        name: "synthetic",
        a,
        b,
    };
    made_up_array(&mut t, &mut Xorshift(0x00dd_ba11));
    let mut out = vec![t];
    if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        out.push(Twin {
            name: "snapshot",
            a: Oracle::from_snapshot(&exe, &pages).unwrap(),
            b: Oracle::from_snapshot(&exe, &pages).unwrap(),
        });
    } else {
        eprintln!("FFX_SNAPSHOT is not set; synthetic cases only");
    }
    out
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
    name: &'static str,
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
        let what = &format!("{}: {what}", self.name);
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

/// Random words, none of them a signalling NaN.
fn random(rng: &mut Xorshift, len: u32) -> Vec<u8> {
    (0..len / 4)
        .flat_map(|_| quiet(rng.next()).to_le_bytes())
        .collect()
}

/// Random bytes in both oracles at a new allocation.
fn random_block(t: &mut Twin, rng: &mut Xorshift, len: u32) -> u32 {
    let at = t.alloc(len as usize);
    t.put(at, &random(rng, len));
    at
}

/// A character of random bytes at `at` (or a new allocation), with every
/// pointer the ported functions follow aimed at random blocks: model
/// block (with texture data), models and header, elements, textures,
/// cluts.
fn make_character(t: &mut Twin, rng: &mut Xorshift, at: Option<u32>) -> u32 {
    let chr = at.unwrap_or_else(|| t.alloc(l::SIZE as usize));
    let mut bytes = random(rng, l::SIZE);
    let blk = random_block(t, rng, l::BLK_SIZE);
    let tex = random_block(t, rng, 0x20);
    let tex_cluts = random_block(t, rng, 8 * 4);
    let tex_images = random_block(t, rng, 16 * 4);
    t.put(tex + 8, &3u16.to_le_bytes());
    t.put(tex + 0xc, &3u16.to_le_bytes());
    t.put_u32(tex + 0x18, tex_cluts);
    t.put_u32(tex + 0x1c, tex_images);
    t.put_u32(blk + l::BLK_TEX, tex);
    let models = random_block(t, rng, 4 * l::MODEL_SIZE);
    let header = t.alloc(16);
    let version: u16 = if rng.next() & 1 == 0 { 0x0830 } else { 0x1029 };
    let mut hdr = [0u8; 16];
    hdr[4..6].copy_from_slice(&version.to_le_bytes());
    hdr[6] = 4;
    hdr[0xa] = 4;
    t.put(header, &hdr);
    let elems = random_block(t, rng, 4 * l::ELEM_SIZE);
    for i in 0..4 {
        let data = random_block(t, rng, 8);
        t.put_u32(elems + i * l::ELEM_SIZE + l::ELEM_DATA, data);
    }
    let clut_tex = random_block(t, rng, 4 * l::TEXTURE_SIZE);
    let image_tex = random_block(t, rng, 4 * l::TEXTURE_SIZE);
    let cluts = random_block(t, rng, 4 * 16);
    let mut put = |off: u32, v: &[u8]| {
        let o = off as usize;
        bytes[o..o + v.len()].copy_from_slice(v);
    };
    put(l::BLK, &blk.to_le_bytes());
    put(l::MODELS, &models.to_le_bytes());
    put(l::MODEL_HEADER, &header.to_le_bytes());
    put(l::ELEMS, &elems.to_le_bytes());
    put(l::CLUT_TEXTURES, &clut_tex.to_le_bytes());
    put(l::CLUT_TEXTURE_COUNT, &3u16.to_le_bytes());
    put(l::IMAGE_TEXTURES, &image_tex.to_le_bytes());
    put(l::IMAGE_TEXTURE_COUNT, &3u16.to_le_bytes());
    put(l::CLUTS, &cluts.to_le_bytes());
    // a sane angle: the wrap loops for ever on infinities
    put(l::ROT, &1.0f32.to_bits().to_le_bytes());
    t.put(chr, &bytes);
    chr
}

/// Characters made up for the image-only oracle: five slots (the second
/// and fifth not in use) and room for one more after them
/// (`Ch_WorkSwap`); `g_ChrArray` and `g_ChrCount` point at them.
fn made_up_array(t: &mut Twin, rng: &mut Xorshift) {
    let array = t.alloc(6 * l::SIZE as usize);
    for i in 0..5 {
        let chr = make_character(t, rng, Some(array + i * l::SIZE));
        t.put(chr + l::ACTIVE, &[u8::from(i != 1 && i != 4)]);
    }
    let (count, arr) = (t.a.rebase(h::CHR_COUNT.sym), t.a.rebase(h::CHR_ARRAY.sym));
    t.put_u32(count, 5);
    t.put_u32(arr, array);
}

/// The array's characters with a model block (active or not), and one
/// made up.
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
    assert!(!out.is_empty(), "no characters");
    out.push(make_character(t, rng, None));
    out
}

/// [`characters`] in use (and the made-up one): those out of use keep
/// stale pointers to elements and textures (`0xcdcdcdcd`).
fn live_characters(t: &mut Twin, rng: &mut Xorshift) -> Vec<u32> {
    let chrs = characters(t, rng);
    let made_up = *chrs.last().unwrap();
    chrs.into_iter()
        .filter(|&c| c == made_up || rd::<u8>(&t.b, c + l::ACTIVE) != 0)
        .collect()
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
    for mut t in twins() {
        float_getters_match_the_original_on(&mut t);
    }
}

fn float_getters_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x2545_f491);
    let getters = float_getters();
    for chr in characters(t, &mut rng) {
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
    for mut t in twins() {
        int_getters_match_the_original_on(&mut t);
    }
}

fn int_getters_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x1234_5678);
    let getters = int_getters();
    for chr in characters(t, &mut rng) {
        // the game's own values first
        for (name, f, _, rust) in getters {
            t.check(&format!("{name} {chr:#x}"), f, &[chr], |m| rust(m, chr));
        }
        // with the hide byte clear, Ch_GetHideCalc reads the alpha
        for bits in FLOATS {
            t.put(chr + l::HIDE_CALC, &[0]);
            let w: u32 = rd(&t.b, chr + l::FLAGS);
            t.put_u32(chr + l::FLAGS, w & !l::F_DRAW_EFFECT);
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
    for mut t in twins() {
        outputs_match_the_original_on(&mut t);
    }
}

fn outputs_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0bad_cafe);
    let out = t.alloc(16);
    for chr in characters(t, &mut rng) {
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
    for mut t in twins() {
        flags_match_the_original_on(&mut t);
    }
}

fn flags_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0001);
    let flags = flag_setters();
    for chr in characters(t, &mut rng) {
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
    for mut t in twins() {
        float_setters_match_the_original_on(&mut t);
    }
}

fn float_setters_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0002);
    let floats = float_setters();
    let chrs = characters(t, &mut rng);
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
    for mut t in twins() {
        vector_setters_match_the_original_on(&mut t);
    }
}

fn vector_setters_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0005);
    for chr in characters(t, &mut rng) {
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
    for mut t in twins() {
        other_setters_match_the_original_on(&mut t);
    }
}

fn other_setters_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0006);
    let ints = int_setters();
    for chr in characters(t, &mut rng) {
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
    for mut t in twins() {
        handles_match_the_original_on(&mut t);
    }
}

fn handles_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0003);
    let chrs = characters(t, &mut rng);
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
    for mut t in twins() {
        helpers_match_the_original_on(&mut t);
    }
}

fn helpers_match_the_original_on(t: &mut Twin) {
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

// ---- round 2: cloth, drawing, models, binding, elements, mouth and neck,
// optional positions, motion state, textures, map ids, settings

/// Flag setters of the second round.
fn more_flag_setters() -> [Flag; 9] {
    [
        (
            "Ch_ClothEnableGrav",
            0x0082_45f0,
            l::DIRTY,
            p::ch_cloth_enable_grav,
        ),
        (
            "Ch_ClothSetEnable",
            0x0082_a930,
            l::FLAGS,
            p::ch_cloth_set_enable,
        ),
        (
            "Ch_DrawEnEffect",
            0x0082_eb60,
            l::FLAGS,
            p::ch_draw_en_effect,
        ),
        ("Ch_DrawSetCull", 0x0083_00b0, l::FLAGS, p::ch_draw_set_cull),
        (
            "Ch_TextureAnimSetEnable",
            0x0083_0d10,
            l::FLAGS2,
            tx::ch_texture_anim_set_enable,
        ),
        ("Ch_SetCalcNorm", 0x0083_5750, l::FLAGS, p::ch_set_calc_norm),
        ("Ch_SetKeepFps", 0x0083_9b50, l::FLAGS, p::ch_set_keep_fps),
        (
            "Ch_DisableBodyHit",
            0x0082_a950,
            l::FLAGS,
            p::ch_disable_body_hit,
        ),
        ("Ch_DisableID", 0x0082_a980, l::FLAGS, p::ch_disable_id),
    ]
}

/// Integer getters of the second round: (name, address, field, rust).
fn more_int_getters() -> [IntGetter; 20] {
    [
        ("Ch_GetCalcNorm", 0x0083_3f30, l::FLAGS, |m, x| {
            Res::Int(p::ch_get_calc_norm(m, x))
        }),
        ("Ch_GetKeepFps", 0x0083_92a0, l::FLAGS, |m, x| {
            Res::Int(p::ch_get_keep_fps(m, x))
        }),
        ("Ch_GetMotionType", 0x0083_3f70, l::MOTION_TYPE, |m, x| {
            int(p::ch_get_motion_type(m, x))
        }),
        (
            "Ch_GetWaitMotion",
            0x0083_92c0,
            l::MOTION_SCRIPTED,
            |m, x| int(p::ch_get_wait_motion(m, x)),
        ),
        (
            "Ch_MouthGetAutoTalk",
            0x0083_4740,
            l::MOUTH_AUTO_TALK,
            |m, x| int(p::ch_mouth_get_auto_talk(m, x)),
        ),
        ("Ch_PLightGetEnable", 0x0083_0ad0, l::PLIGHT, |m, x| {
            int(p::ch_plight_get_enable(m, x))
        }),
        ("Ch_GetShadowDirH", 0x0083_0440, l::SHADOW_DIR_H, |m, x| {
            Res::Int(p::ch_get_shadow_dir_h(m, x))
        }),
        ("Ch_MotionGetId", 0x0083_71a0, l::MOTION_ID, |m, x| {
            Res::Int(p::ch_motion_get_id(m, x))
        }),
        ("Ch_MotionGetFrame", 0x0083_9860, l::MOTION_FRAME, |m, x| {
            Res::Int(p::ch_motion_get_frame(m, x))
        }),
        (
            "Ch_ClutGetNb",
            0x0083_c9a0,
            l::CLUT_TEXTURE_COUNT,
            |m, x| int(tx::ch_clut_get_nb(m, x)),
        ),
        ("Ch_IdGetDictionary", 0x0083_d7a0, l::PID, |m, x| {
            Res::Int(id::ch_id_get_field(m, x, 11))
        }),
        ("Ch_IdGetEffect", 0x0083_d7c0, l::PID, |m, x| {
            Res::Int(id::ch_id_get_field(m, x, 9))
        }),
        ("Ch_IdGetEncount", 0x0083_d820, l::PID, |m, x| {
            Res::Int(id::ch_id_get_field(m, x, 7))
        }),
        ("Ch_IdGetWater", 0x0083_d920, l::PID, |m, x| {
            Res::Int(id::ch_id_get_field(m, x, 13))
        }),
        ("Ch_IdGetSound", 0x0083_d900, l::PID, |m, x| {
            Res::Int(id::ch_id_get_field(m, x, 15))
        }),
        ("Ch_IdGetMap", 0x0083_d840, l::PID, |m, x| {
            Res::Int(id::ch_id_get_map(m, x))
        }),
        ("Ch_IdGetMapB", 0x0083_d860, l::PID, |m, x| {
            Res::Int(id::ch_id_get_map_b(m, x))
        }),
        ("Ch_IdGetEffectTable", 0x0083_d7e0, l::PID, |m, x| {
            Res::Int(id::ch_id_get_effect_table(m, x))
        }),
        ("Ch_IdGetEffectTable", 0x0083_d7e0, l::FLAGS, |m, x| {
            Res::Int(id::ch_id_get_effect_table(m, x))
        }),
        (
            "Ch_IdGetEffectTable",
            0x0083_d7e0,
            l::MOTION_TYPE,
            |m, x| Res::Int(id::ch_id_get_effect_table(m, x)),
        ),
    ]
}

/// Integer setters of the second round.
fn more_int_setters() -> [IntSet; 9] {
    [
        ("Ch_MouthSetAutoTalk", 0x0083_4770, |m, x, v| {
            p::ch_mouth_set_auto_talk(m, x, v);
            Res::Void
        }),
        ("Ch_PLightSetEnable", 0x0083_0ae0, |m, x, v| {
            p::ch_plight_set_enable(m, x, v);
            Res::Void
        }),
        ("Ch_SetShadowDirH", 0x0083_0c90, |m, x, v| {
            p::ch_set_shadow_dir_h(m, x, v);
            Res::Void
        }),
        ("Ch_MotionSetLoop", 0x0083_9960, |m, x, v| {
            p::ch_motion_set_loop(m, x, v);
            Res::Void
        }),
        ("Ch_SoundSetId", 0x0083_5b50, |m, x, v| {
            p::ch_sound_set_id(m, x, v);
            Res::Void
        }),
        ("Ch_ClutEnable", 0x0083_c940, |m, x, v| {
            tx::ch_clut_enable(m, x, v);
            Res::Void
        }),
        ("Ch_IdSetDefPid", 0x0083_d940, |m, x, _| {
            id::ch_id_set_def_pid(m, x);
            Res::Void
        }),
        ("Ch_MouthInit", 0x0083_4750, |m, x, _| {
            p::ch_mouth_init(m, x);
            Res::Void
        }),
        ("Ch_ClothSetFast", 0x0082_4920, |m, x, _| {
            p::ch_cloth_set_fast(m, x);
            Res::Void
        }),
    ]
}

/// Float setters of the second round.
fn more_float_setters() -> [FloatSet; 4] {
    [
        ("Ch_AddY", 0x0082_a8b0, |m, x, v| {
            p::ch_add_y(m, x, v);
            Res::Void
        }),
        ("Ch_ShadowSetHeight", 0x0083_0cb0, |m, x, v| {
            p::ch_shadow_set_height(m, x, v);
            Res::Void
        }),
        ("Ch_BindSetScale", 0x0083_2610, |m, x, v| {
            p::ch_bind_set_scale(m, x, v);
            Res::Void
        }),
        ("Ch_NeckSetSpeed", 0x0083_5020, |m, x, v| {
            p::ch_neck_set_speed(m, x, v);
            Res::Void
        }),
    ]
}

#[test]
fn parts_flags_and_fields_match_the_original() {
    for mut t in twins() {
        parts_flags_and_fields_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)] // one table after another
fn parts_flags_and_fields_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0101);
    for chr in characters(t, &mut rng) {
        for (name, f, field, rust) in more_flag_setters() {
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
        let getters = more_int_getters();
        // the game's own values first
        for (name, f, _, rust) in getters {
            t.check(&format!("{name} {chr:#x}"), f, &[chr], |m| rust(m, chr));
        }
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
        }
        // the effect table: motion type 2, and the other sets
        for (mt, set) in [(2u8, 0u8), (2, 1), (0, 0), (0, 1), (0, 2), (1, 0xff)] {
            t.put(chr + l::MOTION_TYPE, &[mt]);
            t.put(t.a.rebase(id::EFFECT_TABLE_INDEX.sym), &[set]);
            for w in [0, u32::MAX, 0x2000_0000, rng.next()] {
                t.put_u32(chr + l::FLAGS, w);
                t.put_u32(chr + l::PID, rng.next());
                t.check(
                    &format!("Ch_IdGetEffectTable {chr:#x} {mt} {set} {w:#x}"),
                    0x0083_d7e0,
                    &[chr],
                    |m| Res::Int(id::ch_id_get_effect_table(m, chr)),
                );
            }
        }
        for (name, f, rust) in more_int_setters() {
            for v in INTS.iter().copied().chain((0..8).map(|_| rng.next())) {
                t.check(&format!("{name} {chr:#x} {v:#x}"), f, &[chr, v], |m| {
                    rust(m, chr, v)
                });
            }
        }
        // the normal update requests, with and without the every-frame bit
        for w in [0, 1, 8, 0x18, u32::MAX, rng.next(), rng.next() | 1] {
            for (name, f, bit) in [
                ("Ch_ReserveCalcNorm", 0x0083_5710, l::F_RESERVE_CALC_NORM),
                ("Ch_ReserveCalcNorm2", 0x0083_5730, l::F_RESERVE_CALC_NORM2),
            ] {
                t.put_u32(chr + l::FLAGS, w);
                t.check(&format!("{name} {chr:#x} {w:#x}"), f, &[chr], |m| {
                    p::ch_reserve_calc_norm(m, chr, bit);
                    Res::Void
                });
            }
        }
        for (bits, mask) in [
            (0, 0),
            (1, 1),
            (0xff, 0x0f),
            (0x10, 0xff),
            (rng.next(), rng.next()),
        ] {
            t.check(
                &format!("Ch_PLightSetEnableMask {chr:#x} {bits:#x} {mask:#x}"),
                0x0083_0b00,
                &[chr, bits, mask],
                |m| {
                    p::ch_plight_set_enable_mask(m, chr, bits, mask);
                    Res::Void
                },
            );
        }
        // cloth by character id
        let saved: u16 = rd(&t.b, chr + l::ID);
        for chr_id in (0u16..0x80).chain([0x65, 0x66, 0xffff, 0x1001, saved]) {
            t.put(chr + l::ID, &chr_id.to_le_bytes());
            t.check(
                &format!("Ch_ClothSetFast {chr:#x} id {chr_id:#x}"),
                0x0082_4920,
                &[chr],
                |m| {
                    p::ch_cloth_set_fast(m, chr);
                    Res::Void
                },
            );
            t.check(
                &format!("Ch_ClothMeshMappingOffset {chr:#x} id {chr_id:#x}"),
                0x0082_4620,
                &[chr],
                |m| {
                    p::ch_cloth_mesh_mapping_offset(m, chr);
                    Res::Void
                },
            );
        }
        t.put(chr + l::ID, &saved.to_le_bytes());
    }
}

#[test]
fn parts_floats_and_outputs_match_the_original() {
    for mut t in twins() {
        parts_floats_and_outputs_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)] // one table after another
fn parts_floats_and_outputs_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0102);
    let out = t.alloc(64);
    let chrs = live_characters(t, &mut rng);
    for &chr in &chrs {
        for (name, f, rust) in more_float_setters() {
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
        for bits in FLOATS
            .iter()
            .copied()
            .chain((0..4).map(|_| quiet(rng.next())))
        {
            t.check(
                &format!("Ch_AllSetThicknessEvent {bits:#x}"),
                0x0082_a8d0,
                &[bits],
                |m| {
                    p::ch_all_set_thickness_event(m, fl(bits));
                    Res::Void
                },
            );
        }
        for _ in 0..12 {
            let v: Vec<u32> = (0..6)
                .map(|_| FLOATS[rng.next() as usize % FLOATS.len()])
                .collect();
            t.check(
                &format!("Ch_NeckSetLimit {chr:#x} {v:x?}"),
                0x0083_4fd0,
                &[chr, v[0], v[1]],
                |m| {
                    p::ch_neck_set_limit(m, chr, fl(v[0]), fl(v[1]));
                    Res::Void
                },
            );
            // velocities from the positions
            for (i, &bits) in v.iter().enumerate() {
                let i = u32::try_from(i).unwrap();
                t.put_u32(chr + l::POS + 4 * (i % 3), bits);
                t.put_u32(chr + l::POS_NEXT + 4 * (i % 3), v[(i as usize + 1) % 6]);
                t.put_u32(chr + l::POS_2 + 4 * (i % 3), v[(i as usize + 2) % 6]);
            }
            t.check(
                &format!("Ch_GetVel {chr:#x} {v:x?}"),
                0x0083_4000,
                &[chr, out],
                |m| {
                    p::ch_get_vel(m, chr, out);
                    Res::Void
                },
            );
            t.check(
                &format!("Ch_GetVel2 {chr:#x} {v:x?}"),
                0x0083_4030,
                &[chr, out],
                |m| {
                    p::ch_get_vel2(m, chr, out);
                    Res::Void
                },
            );
            // the shade from the fades
            for (i, &bits) in v.iter().enumerate() {
                let i = u32::try_from(i).unwrap();
                t.put_u32(chr + l::FADE_SHADE_COL + 12 * (i % 3), bits);
                t.put_u32(
                    chr + l::SHADE_COL_EFFECT + 4 * (i % 3),
                    v[(i as usize + 3) % 6],
                );
            }
            t.put_u32(chr + l::FADE_SHADE, v[rng.next() as usize % 6]);
            t.put_u32(chr + l::FADE_LIGHT, v[rng.next() as usize % 6]);
            for by_light in [0u8, 1, 0x80] {
                t.put(t.a.rebase(p::SHADE_BY_LIGHT.sym), &[by_light]);
                let args = [chr, out, out + 4, out + 8];
                t.check(
                    &format!("Ch_DrawCalcShade {chr:#x} {by_light} {v:x?}"),
                    0x0082_eaf0,
                    &args,
                    |m| {
                        p::ch_draw_calc_shade(m, chr, out, out + 4, out + 8);
                        Res::Void
                    },
                );
            }
            // the model alpha
            t.put_u32(chr + l::FADE_ALPHA, v[0]);
            let models: u32 = rd(&t.b, chr + l::MODELS);
            for model in [u32::MAX, 0, 1, 2] {
                if model != u32::MAX {
                    t.put_u32(
                        models + model * l::MODEL_SIZE + l::MODEL_ALPHA,
                        v[(model as usize + 1) % 6],
                    );
                }
                t.check(
                    &format!("Ch_DrawGetModelAlpha {chr:#x} {model}"),
                    0x0082_eba0,
                    &[chr, model],
                    |m| Res::Float(p::ch_draw_get_model_alpha(m, chr, model)),
                );
            }
            // optional positions
            for i in 0..8u32 {
                t.put_u32(chr + l::OPTPOS + 16 * i, v[i as usize % 6]);
                t.check(
                    &format!("Ch_OptposGetExist {chr:#x} {i}"),
                    0x0083_5680,
                    &[chr, i],
                    |m| Res::Int(p::ch_optpos_get_exist(m, chr, i)),
                );
            }
        }
        for (name, f, field) in [
            ("Ch_OptposGetBottom", 0x0083_5600, l::OPTPOS_BOTTOM),
            ("Ch_OptposGetCameraRef", 0x0083_5630, l::OPTPOS_CAMERA_REF),
            ("Ch_OptposGetTop", 0x0083_56e0, l::OPTPOS_TOP),
        ] {
            t.check(&format!("{name} {chr:#x}"), f, &[chr, out], |m| {
                p::ch_optpos_copy(m, chr, field, out);
                Res::Void
            });
        }
        // models and elements
        let header: u32 = rd(&t.b, chr + l::MODEL_HEADER);
        for version in [0x0830u16, 0x0831, 0x1029, rd(&t.b, header + 4)] {
            t.put(header + 4, &version.to_le_bytes());
            t.check(
                &format!("Ch_GetNbModel {chr:#x} {version:#x}"),
                0x0083_0410,
                &[chr],
                |m| Res::Int(p::ch_get_nb_model(m, chr)),
            );
        }
        t.check(
            &format!("Ch_GetNbElem {chr:#x}"),
            0x0083_03f0,
            &[chr],
            |m| Res::Int(p::ch_get_nb_elem(m, chr)),
        );
        let elems = p::ch_get_nb_elem(&t.b, chr).min(8);
        for e in 0..elems {
            t.check(
                &format!("Ch_ElemGetParent {chr:#x} {e}"),
                0x0083_3cf0,
                &[chr, e],
                |m| Res::Int(p::ch_elem_get_parent(m, chr, e)),
            );
            let (mtx, w) = (rng.next(), FLOATS[rng.next() as usize % FLOATS.len()]);
            t.check(
                &format!("Ch_ElemSetQuatMatrix {chr:#x} {e}"),
                0x0083_3df0,
                &[chr, e, mtx, w],
                |m| {
                    p::ch_elem_set_quat_matrix(m, chr, e, mtx, fl(w));
                    Res::Void
                },
            );
            t.check(
                &format!("Ch_ElemSetReplaceMatrix {chr:#x} {e}"),
                0x0083_3e30,
                &[chr, e, mtx],
                |m| {
                    p::ch_elem_set_replace_matrix(m, chr, e, mtx);
                    Res::Void
                },
            );
        }
        // binding to another character (or none)
        for &parent in chrs.iter().take(3).chain(&[0]) {
            let elem = rng.next() % 8;
            t.check(
                &format!("Ch_BindToElemDirect {chr:#x} {parent:#x}"),
                0x0083_2680,
                &[chr, parent, elem],
                |m| {
                    p::ch_bind_to_elem_direct(m, chr, parent, elem);
                    Res::Void
                },
            );
            for _ in 0..4 {
                let v: Vec<u32> = (0..3)
                    .map(|_| FLOATS[rng.next() as usize % FLOATS.len()])
                    .collect();
                let args = [chr, parent, elem, v[0], v[1], v[2]];
                t.check(
                    &format!("Ch_BindToElemDirectOfs {chr:#x} {parent:#x} {v:x?}"),
                    0x0083_26e0,
                    &args,
                    |m| {
                        p::ch_bind_to_elem_direct_ofs(
                            m,
                            chr,
                            parent,
                            elem,
                            [fl(v[0]), fl(v[1]), fl(v[2])],
                        );
                        Res::Void
                    },
                );
            }
        }
    }
}

#[test]
fn textures_match_the_original() {
    for mut t in twins() {
        textures_match_the_original_on(&mut t);
    }
}

fn textures_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0103);
    let out = t.alloc(16);
    for chr in live_characters(t, &mut rng) {
        let n = i32::from(rd::<i16>(&t.b, chr + l::CLUT_TEXTURE_COUNT)).clamp(0, 8);
        let ni = i32::from(rd::<i16>(&t.b, chr + l::IMAGE_TEXTURE_COUNT)).clamp(0, 8);
        for i in 0..=n.max(ni) + 1 {
            #[allow(clippy::cast_sign_loss)]
            let a = i as u32;
            {
                t.check(
                    &format!("Ch_TextureGetClutAddr {chr:#x} {i}"),
                    0x0083_0d30,
                    &[chr, a],
                    |m| Res::Int(tx::ch_texture_get_clut_addr(m, chr, i)),
                );
                t.check(
                    &format!("Ch_TextureGetClutBase {chr:#x} {i}"),
                    0x0083_0d60,
                    &[chr, a],
                    |m| Res::Int(tx::ch_texture_get_clut_base(m, chr, i)),
                );
            }
            t.check(
                &format!("Ch_TextureGetImageAddr {chr:#x} {i}"),
                0x0083_0d90,
                &[chr, a],
                |m| Res::Int(tx::ch_texture_get_image_addr(m, chr, i)),
            );
            t.check(
                &format!("Ch_TextureGetImageBase {chr:#x} {i}"),
                0x0083_0dc0,
                &[chr, a],
                |m| Res::Int(tx::ch_texture_get_image_base(m, chr, i)),
            );
        }
        let cluts: u32 = rd(&t.b, chr + l::CLUTS);
        for i in 0..u32::try_from(n).unwrap().max(1) {
            if cluts != 0 {
                t.put_u32(
                    cluts + 16 * i + 8,
                    [0, 0x1f, 0x20, 0x60, 0x1b, rng.next()][i as usize % 6],
                );
            }
            t.check(
                &format!("Ch_ClutGetPtr {chr:#x} {i}"),
                0x0083_c9d0,
                &[chr, i],
                |m| Res::Int(tx::ch_clut_get_ptr(m, chr, i)),
            );
            t.check(
                &format!("Ch_ClutGetOrgPtr {chr:#x} {i}"),
                0x0083_c9b0,
                &[chr, i],
                |m| Res::Int(tx::ch_clut_get_org_ptr(m, chr, i)),
            );
            t.check(
                &format!("Ch_ClutGetSize {chr:#x} {i}"),
                0x0083_c9f0,
                &[chr, i],
                |m| int(tx::ch_clut_get_size(m, chr, i)),
            );
        }
        t.check(
            &format!("Ch_GetLightValue {chr:#x}"),
            0x0083_cb90,
            &[chr],
            |_| Res::Int(tx::ch_get_light_value(chr)),
        );
        t.check(
            &format!("Ch_TexGetNbClut {chr:#x}"),
            0x0083_cd90,
            &[chr],
            |m| Res::Int(tx::ch_tex_get_nb_clut(m, chr)),
        );
        t.check(
            &format!("Ch_TexGetNbImage {chr:#x}"),
            0x0083_cdb0,
            &[chr],
            |m| Res::Int(tx::ch_tex_get_nb_image(m, chr)),
        );
        let nc = tx::ch_tex_get_nb_clut(&t.b, chr).min(8);
        let nimg = tx::ch_tex_get_nb_image(&t.b, chr).min(8);
        for i in 0..nc.max(1) {
            t.check(
                &format!("Ch_TexGetClut {chr:#x} {i}"),
                0x0083_cd20,
                &[chr, i, out],
                |m| Res::Int(tx::ch_tex_get_clut(m, chr, i, out)),
            );
        }
        for i in 0..nimg.max(1) {
            t.check(
                &format!("Ch_TexGetImage {chr:#x} {i}"),
                0x0083_cd50,
                &[chr, i, out, out + 4],
                |m| Res::Int(tx::ch_tex_get_image(m, chr, i, out, out + 4)),
            );
        }
    }
}

#[test]
fn lookups_match_the_original() {
    for mut t in twins() {
        lookups_match_the_original_on(&mut t);
    }
}

fn lookups_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0104);
    let chrs = characters(t, &mut rng);
    let mut ids = vec![0, u32::MAX, rng.next()];
    for &chr in &chrs {
        ids.push(rd(&t.b, chr + l::TEXTURE_ANIM_ID));
    }
    for &key in &ids {
        t.check(
            &format!("Ch_TextureAnimGetHandle {key:#x}"),
            0x0083_5ed0,
            &[key],
            |m| Res::Int(h::ch_texture_anim_get_handle(m, key)),
        );
    }
    // cloth loads: the table's own keys, then one entry made loading
    let first = t.a.rebase(h::CLOTH_LOADS.sym);
    let mut keys = vec![0, 1, u32::MAX, rng.next()];
    for i in 0..h::CLOTH_LOADS_LEN {
        keys.push(rd(&t.b, first + i * 0x110 + 4));
    }
    for state in [None, Some(1u8), Some(2)] {
        if let Some(s) = state {
            t.put(first + 7 * 0x110, &[s]);
            t.put_u32(first + 7 * 0x110 + 4, 0x1234_5678);
            keys.push(0x1234_5678);
        }
        for &key in &keys {
            t.check(
                &format!("Ch_IsLoadingClothFile {key:#x} {state:?}"),
                0x0082_a100,
                &[key],
                |m| Res::Int(h::ch_is_loading_cloth_file(m, key)),
            );
        }
    }
    // pointer lists moved by a delta
    for n in [0u32, 1, 5] {
        let ptrs = random_block(t, &mut rng, 4 * n.max(1));
        let list = t.alloc(8);
        t.put_u32(list, ptrs);
        t.put_u32(list + 4, n);
        let delta = rng.next();
        t.check(
            &format!("Ch_RelElemMapping {n} {delta:#x}"),
            0x0083_77a0,
            &[list, delta],
            |m| {
                p::ch_rel_elem_mapping(m, list, delta);
                Res::Void
            },
        );
    }
    // the work slot after the array
    let array = h::CHR_ARRAY.get(&t.b);
    #[allow(clippy::cast_sign_loss)]
    let end = array + h::CHR_COUNT.get(&t.b) as u32 * l::SIZE;
    if Oracle::read(&mut t.a, end, l::SIZE as usize).is_ok() {
        t.check("Ch_WorkSwap", 0x0082_8b30, &[array], |m| {
            Res::Int(p::ch_work_swap(m, array))
        });
    }
}

#[test]
fn settings_match_the_original() {
    for mut t in twins() {
        settings_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)] // one setting after another
fn settings_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0105);
    let out = t.alloc(16);
    let i8s: [(&str, u32, &str, u32, ffx_bridge::Global<i8>); 4] = [
        (
            "Ch_ClothGetParaEdit",
            0x0082_4610,
            "Ch_ClothSetParaEdit",
            0x0082_4a30,
            sy::CLOTH_PARA_EDIT,
        ),
        (
            "Ch_DrawGetDisableSpec",
            0x0082_eb80,
            "Ch_DrawSetDisableSpec",
            0x0083_00d0,
            sy::DRAW_DISABLE_SPEC,
        ),
        (
            "Ch_DrawGetFourceSpec",
            0x0082_eb90,
            "Ch_DrawSetFourceSpec",
            0x0083_00e0,
            sy::DRAW_FOURCE_SPEC,
        ),
        (
            "Ch_DrawGetParaEdit",
            0x0082_ebe0,
            "Ch_DrawSetParaEdit",
            0x0083_00f0,
            sy::DRAW_PARA_EDIT,
        ),
    ];
    for (get, fg, set, fs, g) in i8s {
        t.check(get, fg, &[], |m| int(sy::get_i8(m, g)));
        for v in INTS {
            t.check(&format!("{set} {v:#x}"), fs, &[v], |m| {
                sy::set_i8(m, g, v);
                Res::Void
            });
            t.check(&format!("{get} after {v:#x}"), fg, &[], |m| {
                int(sy::get_i8(m, g))
            });
        }
    }
    for v in INTS {
        t.check(&format!("Ch_SetDumpMotEv {v:#x}"), 0x0083_57c0, &[v], |m| {
            sy::set_i8(m, sy::DUMP_MOT_EV, v);
            Res::Void
        });
        t.check("Ch_GetDumpMotEv", 0x0083_3f50, &[], |m| {
            int(sy::get_i8(m, sy::DUMP_MOT_EV))
        });
        t.put(t.a.rebase(sy::ENABLE_TEXTURE.sym), &v.to_le_bytes()[..1]);
        t.check("Ch_EnableGetTexture", 0x0083_0320, &[], |m| {
            int(sy::get_i8(m, sy::ENABLE_TEXTURE))
        });
        t.check(
            &format!("Ch_GuiSetCalcNormAll {v:#x}"),
            0x0083_4090,
            &[v],
            |m| {
                sy::ch_gui_set_calc_norm_all(m, v);
                Res::Void
            },
        );
        t.check("Ch_GuiGetCalcNormal", 0x0083_4060, &[], |m| {
            Res::Int(sy::ch_gui_get_calc_normal(m))
        });
        #[allow(clippy::cast_possible_wrap)]
        let s = v as i32;
        t.check(
            &format!("Ch_GuiSetMouthTh {v:#x}"),
            0x0083_40a0,
            &[v],
            |m| int(sy::ch_gui_set_mouth_th(m, s)),
        );
        t.check(
            &format!("Ch_DrawSetShadowMode {v:#x}"),
            0x0083_0140,
            &[v],
            |m| {
                sy::ch_draw_set_shadow_mode(m, v);
                Res::Void
            },
        );
        t.check(
            &format!("Ch_MotionSetHalf {v:#x}"),
            0x0083_9950,
            &[v],
            |m| {
                sy::ch_motion_set_half(m, v);
                Res::Void
            },
        );
        t.check("Ch_MotionGetHalf", 0x0083_9870, &[], |m| {
            Res::Int(sy::ch_motion_get_half(m))
        });
        for (name, f, g) in [
            ("Ch_EffectPacketStoreMAT", 0x0084_5800, sy::PACKET_MAT),
            ("Ch_EffectPacketStorePRG", 0x0084_5810, sy::PACKET_PRG),
        ] {
            t.check(&format!("{name} {v:#x}"), f, &[v], |m| {
                sy::set_u16(m, g, v);
                Res::Void
            });
        }
    }
    for bits in FLOATS {
        for (name, f, g) in [
            ("Ch_ClothSetXZ", 0x0082_4a40, sy::CLOTH_XZ),
            ("Ch_ClothSetY", 0x0082_4a50, sy::CLOTH_Y),
        ] {
            t.check(&format!("{name} {bits:#x}"), f, &[bits], |m| {
                sy::set_f32(m, g, fl(bits));
                Res::Void
            });
        }
    }
    for _ in 0..8 {
        let (w0, w1, w2) = (rng.next(), rng.next(), rng.next());
        t.check(
            &format!("Ch_DrawSetShadowColor {w0:#x} {w1:#x} {w2:#x}"),
            0x0083_0100,
            &[w0, w1, w2],
            |m| {
                sy::ch_draw_set_shadow_color(m, [w0, w1, w2]);
                Res::Void
            },
        );
        t.check(
            &format!("Ch_DrawSetShadowDir {w0:#x} {w1:#x}"),
            0x0083_0120,
            &[w0, w1],
            |m| {
                sy::ch_draw_set_shadow_dir(m, [w0, w1]);
                Res::Void
            },
        );
        t.check(
            &format!("Ch_GuiSetTalk {w0:#x} {w1:#x}"),
            0x0083_40d0,
            &[w0, w1],
            |m| {
                sy::ch_gui_set_talk(m, w0, w1);
                Res::Void
            },
        );
        for mask in 0..8u32 {
            let o: Vec<u32> = (0..3)
                .map(|i| if mask & (1 << i) != 0 { out + 4 * i } else { 0 })
                .collect();
            t.check(
                &format!("Ch_DrawGetShadowColor {mask}"),
                0x0082_ebf0,
                &o,
                |m| {
                    sy::ch_draw_get_shadow_color(m, [o[0], o[1], o[2]]);
                    Res::Void
                },
            );
            t.check(
                &format!("Ch_DrawGetShadowDir {mask}"),
                0x0082_ec30,
                &o[..2],
                |m| {
                    sy::ch_draw_get_shadow_dir(m, [o[0], o[1]]);
                    Res::Void
                },
            );
        }
    }
    for i in 0..8 {
        let v = rng.next();
        t.check(
            &format!("Ch_SetFuka {i} {v:#x}"),
            0x0083_57d0,
            &[i, v],
            |m| {
                sy::ch_set_fuka(m, i, v);
                Res::Void
            },
        );
        t.check(&format!("Ch_GetFuka {i}"), 0x0083_3f60, &[i], |m| {
            Res::Int(sy::ch_get_fuka(m, i))
        });
    }
    t.check("Ch_SysGetScreenMatrixPtr", 0x0083_5b70, &[], |m| {
        Res::Int(sy::ch_sys_get_screen_matrix_ptr(m))
    });
    t.check("Ch_SysGetCameraMatrixPtr", 0x0083_5ba0, &[], |m| {
        Res::Int(sy::ch_sys_get_camera_matrix_ptr(m))
    });
    let src = random_block(t, &mut rng, 64);
    t.check("Ch_SysSetScreenMatrix", 0x0083_5e90, &[src], |m| {
        sy::set_matrix(m, sy::SCREEN_MATRIX, src);
        Res::Void
    });
    let src = random_block(t, &mut rng, 64);
    t.check("Ch_SysSetCameraMatrix", 0x0083_5eb0, &[src], |m| {
        sy::set_matrix(m, sy::CAMERA_MATRIX, src);
        Res::Void
    });
    t.check("Ch_MemMoveSetTrigger", 0x0082_7250, &[], |m| {
        sy::ch_mem_move_set_trigger(m);
        Res::Void
    });
    // the effect packet end tag: measuring, then writing
    let buf = random_block(t, &mut rng, 64);
    for ptr in [0, buf, buf + 16] {
        t.put_u32(t.a.rebase(sy::PACKET_PTR.sym), ptr);
        t.check(
            &format!("Ch_EffectPacketStoreEND {ptr:#x}"),
            0x0084_57d0,
            &[],
            |m| {
                sy::ch_effect_packet_store_end(m);
                Res::Void
            },
        );
        t.check("Ch_EffectPacketGetSize", 0x0084_5700, &[], |m| {
            Res::Int(sy::ch_effect_packet_get_size(m))
        });
    }
}

#[test]
fn fixed_point_matches_the_original() {
    for mut t in twins() {
        fixed_point_matches_the_original_on(&mut t);
    }
}

fn fixed_point_matches_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0106);
    for low in (0..=0xffffu32)
        .step_by(7)
        .chain([0x8000, 0x7fff, 0xffff, 1])
    {
        let v = low | (rng.next() & 0xffff_0000);
        t.check(
            &format!("Fixed12_ToFloat {v:#x}"),
            0x0083_9dd0,
            &[v],
            |_| Res::Float(fixed::fixed12_to_float(v)),
        );
        t.check(&format!("Fixed8_ToFloat {v:#x}"), 0x0083_9e00, &[v], |_| {
            Res::Float(fixed::fixed8_to_float(v))
        });
    }
    let edges = [
        0u32,
        1,
        0x7ff,
        0x800,
        0x801,
        0xfff,
        0x1000,
        0xffff_f800,
        u32::MAX,
    ];
    let mut cases: Vec<[u32; 3]> = Vec::new();
    for &a in &edges {
        for &b in &edges {
            for tt in [0u32, 0x80, 0x100, 0x200, 0xffff_ff00] {
                cases.push([a, b, tt]);
            }
        }
    }
    cases.extend((0..2000).map(|_| [rng.next(), rng.next(), rng.next() % 0x300]));
    for [a, b, tt] in cases {
        #[allow(clippy::cast_possible_wrap)]
        let (sa, sb, st) = (a as i32, b as i32, tt as i32);
        t.check(
            &format!("Angle12_Lerp {a:#x} {b:#x} {tt:#x}"),
            0x0083_9e30,
            &[a, b, tt],
            |_| int(fixed::angle12_lerp(sa, sb, st)),
        );
    }
}
