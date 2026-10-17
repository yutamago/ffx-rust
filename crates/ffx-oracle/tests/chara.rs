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
        // A float the original leaves in ST0 is a result its callers use.
        assert!(
            ra.st0.is_none() || matches!(rb, Res::Float(_)),
            "{what}: the original returns {:?} in ST0",
            ra.st0
        );
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
#[allow(clippy::too_many_lines)]
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
    // the clut list's cluts: a 16-byte header (size in 16 bytes, - 2, at
    // -0x10) and 64 bytes of colours
    for i in 0..4 {
        let c = random_block(t, rng, 0x60) + 0x20;
        t.put_u32(c - 0x10, 2 | (rng.next() & 0xffff_8000));
        t.put_u32(tex_cluts + 8 * i + 4, c);
    }
    // the archive behind the block: attachment points, the image list
    let arc = random_block(t, rng, 0x80);
    let attrs = random_block(t, rng, 8 * 16);
    for i in 0..8u32 {
        let kind = [0u32, 0x4000, 0x8000, 0xc000][rng.next() as usize % 4];
        t.put(
            attrs + 16 * i,
            &u16::try_from(i | kind).unwrap().to_le_bytes(),
        );
        t.put(
            attrs + 16 * i + 2,
            &u16::try_from(i % 4).unwrap().to_le_bytes(),
        );
    }
    t.put_u32(arc + 0x20, attrs);
    t.put_u32(arc + 0x24, 8);
    let images = random_block(t, rng, 0x20);
    let list = random_block(t, rng, 3 * 16);
    let image_table = random_block(t, rng, 4 * 16);
    for i in 0..3 {
        t.put(list + 16 * i, &[u8::try_from(rng.next() % 4).unwrap()]);
    }
    t.put(images + 4, &3u16.to_le_bytes());
    t.put_u32(images + 0x14, list);
    t.put_u32(images + 0x1c, image_table);
    t.put_u32(arc + 0x18, images);
    t.put_u32(blk + 0x6c, arc);
    let models = random_block(t, rng, 4 * l::MODEL_SIZE);
    // model 0 has a cloth of two vertices (one extra), the others none
    for i in 0..4 {
        let m = models + i * l::MODEL_SIZE;
        let data = random_block(t, rng, 0x40);
        let verts = random_block(t, rng, 4 * 16);
        let buf = random_block(t, rng, 48 * 4);
        t.put_u32(m, data);
        t.put_u32(m + 8, verts);
        t.put_u32(m + 0xc, buf);
        t.put(data + 0x18, &u16::from(i == 0).to_le_bytes());
        // base element 3 - i, i vertices
        t.put(data, &u16::try_from(3 - i).unwrap().to_le_bytes());
        t.put(data + 4, &u16::try_from(i).unwrap().to_le_bytes());
        let cloth = random_block(t, rng, 0x24);
        let idx = t.alloc(8);
        t.put(idx, &[1, 0, 3, 0, 0, 0, 2, 0]);
        t.put_u32(cloth + 8, idx);
        t.put(cloth + 0x13, &[1]);
        t.put(cloth + 0x1c, &2u16.to_le_bytes());
        t.put_u32(data + 0x1c, cloth);
    }
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
    let cluts = random_block(t, rng, 0x400);
    // motion group nodes (+0x74c, a list) and curves (+0x748)
    let node2 = random_block(t, rng, 16);
    t.put_u32(node2, 0);
    let node1 = random_block(t, rng, 16);
    t.put_u32(node1, node2);
    let curves = random_block(t, rng, 36 * 0x2c);
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
    put(0x74c, &node1.to_le_bytes());
    put(0x748, &curves.to_le_bytes());
    // no motion element list, no texture list
    put(0x758, &0u32.to_le_bytes());
    put(0x6e0, &0u32.to_le_bytes());
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

/// A float argument: one of [`FLOATS`] or random bits (quiet).
fn any_float(rng: &mut Xorshift) -> u32 {
    if rng.next() & 1 == 0 {
        FLOATS[rng.next() as usize % FLOATS.len()]
    } else {
        quiet(rng.next())
    }
}

/// Small floats: the angles and weights the lerps get in the game.
fn small_float(rng: &mut Xorshift) -> u32 {
    #[allow(clippy::cast_precision_loss)]
    let v = (rng.next() % 20_000) as f32 / 1000.0 - 10.0;
    v.to_bits()
}

fn set_global(t: &mut Twin, sym: u32, v: u32) {
    let at = t.a.rebase(sym);
    t.put_u32(at, v);
}

#[test]
fn motion_lerps_match_the_original() {
    for mut t in twins() {
        motion_lerps_match_the_original_on(&mut t);
    }
}

#[allow(clippy::many_single_char_names)]
fn motion_lerps_match_the_original_on(t: &mut Twin) {
    use ffx_chara::motion as mo;
    let mut rng = Xorshift(0x5eed_0201);
    for &a in &FLOATS {
        for &b in &FLOATS {
            for &w in &FLOATS[..13] {
                t.check(
                    &format!("Chr_Lerp {a:#x} {b:#x} {w:#x}"),
                    0x0083_9cb0,
                    &[a, b, w],
                    |_| Res::Float(mo::lerp(fl(a), fl(b), fl(w))),
                );
            }
        }
    }
    for i in 0..6000 {
        let pick = |rng: &mut Xorshift| {
            if i % 2 == 0 {
                small_float(rng)
            } else {
                any_float(rng)
            }
        };
        let args = [
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
        ];
        let [a, b, x, lo, k] = args.map(fl);
        t.check(
            &format!("Chr_LerpDamped {args:x?}"),
            0x0083_9d70,
            &args,
            |_| Res::Float(mo::lerp_damped(a, b, x, lo, k)),
        );
        t.check(
            &format!("Chr_LerpAngleDamped {args:x?}"),
            0x0083_9cd0,
            &args,
            |_| Res::Float(mo::lerp_angle_damped(a, b, x, lo, k)),
        );
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
            for tt in [0u32, 0x80, 0x100, 0x200, 0x8000, 0xffff_ff00, u32::MAX] {
                cases.push([a, b, tt]);
            }
        }
    }
    cases.extend((0..3000).map(|_| [rng.next(), rng.next(), rng.next()]));
    for [a, b, tt] in cases {
        t.check(
            &format!("AngleTurn_Lerp {a:#x} {b:#x} {tt:#x}"),
            0x0083_9e80,
            &[a, b, tt],
            |_| Res::Float(mo::angle_turn_lerp(a, b, tt)),
        );
    }
    for chr in characters(t, &mut rng) {
        t.check(
            &format!("Ch_MotionGetLoop {chr:#x}"),
            0x0083_9880,
            &[chr],
            |m| int(ffx_chara::motion::bridge::ch_motion_get_loop(m, chr)),
        );
    }
}

#[test]
fn attachments_match_the_original() {
    for mut t in twins() {
        attachments_match_the_original_on(&mut t);
    }
}

fn attachments_match_the_original_on(t: &mut Twin) {
    use ffx_chara::attach::bridge as at;
    let mut rng = Xorshift(0x5eed_0202);
    let out = t.alloc(16);
    let elem = t.alloc(4);
    let chrs = live_characters(t, &mut rng);
    for &chr in &chrs {
        let list: u32 = rd(&t.b, in_blk(&t.b, chr, at::BLK_ATTRS));
        let mut ids: Vec<u32> = (0..10).collect();
        if list != 0 {
            let n: u32 = rd(&t.b, list + 0x24);
            let e: u32 = rd(&t.b, list + 0x20);
            for i in 0..n.min(64) {
                ids.push(u32::from(rd::<u16>(&t.b, e + 16 * i)) & 0x3fff);
            }
        }
        ids.extend([0x3fff, 0x4001, 0xffff_ffff, rng.next()]);
        for &id in &ids {
            for o in [out, 0] {
                t.check(
                    &format!("Ch_ElemGetAttr {chr:#x} {id:#x} {o:#x}"),
                    0x0083_3a70,
                    &[chr, id, elem, o],
                    |m| Res::Int(at::ch_elem_get_attr(m, chr, id, elem, o)),
                );
                t.check(
                    &format!("Ch_OptposGetElement {chr:#x} {id:#x} {o:#x}"),
                    0x0083_5660,
                    &[chr, id, o],
                    |m| Res::Int(at::ch_optpos_get_element(m, chr, id, o)),
                );
            }
            if list != 0 {
                t.check(
                    &format!("Ch_ElemGetDirect {chr:#x} {id:#x}"),
                    0x0083_3bf0,
                    &[chr, id],
                    |m| Res::Int(at::ch_elem_get_direct(m, chr, id)),
                );
            }
        }
        for &parent in chrs.iter().chain(&[0]) {
            let id = ids[rng.next() as usize % ids.len()];
            t.check(
                &format!("Ch_BindToElem {chr:#x} {parent:#x} {id:#x}"),
                0x0083_2630,
                &[chr, parent, id],
                |m| {
                    let r = at::ch_bind_to_elem(m, chr, parent, id);
                    // a null parent leaves the caller's eax
                    if parent == 0 { Res::Void } else { Res::Int(r) }
                },
            );
        }
    }
    // optional positions: some all zero, some only w zero
    for &chr in &chrs {
        for round in 0..3 {
            for i in 0..23u32 {
                let p = chr + at::OPTPOS + 16 * i;
                match rng.next() % 4 {
                    0 => t.put(p, &[0; 16]),
                    1 => t.put_u32(p + 12, 0),
                    2 => t.put(p, &[0; 12]),
                    _ => {}
                }
            }
            if round == 2 {
                t.put(chr + at::OPTPOS, &[0; 16]);
            }
            for i in 0..23u32 {
                t.check(
                    &format!("Ch_OptposGet {chr:#x} {i}"),
                    0x0083_54f0,
                    &[chr, i, out],
                    |m| {
                        at::ch_optpos_get(m, chr, i, out);
                        Res::Void
                    },
                );
            }
            t.check(
                &format!("Ch_OptposGetBase {chr:#x}"),
                0x0083_55e0,
                &[chr, out],
                |m| {
                    at::ch_optpos_get_base(m, chr, out);
                    Res::Void
                },
            );
        }
    }
}

#[test]
fn cloth_matches_the_original() {
    for mut t in twins() {
        cloth_matches_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn cloth_matches_the_original_on(t: &mut Twin) {
    use ffx_chara::cloth::bridge as cl;
    let mut rng = Xorshift(0x5eed_0203);
    let chrs = live_characters(t, &mut rng);
    let probe = make_character(t, &mut rng, None);
    let maps = [
        0u32, 0x15d, 0x84, 0x107, 0x17d, 0x3b, 0x39, 0xa1, 0xb7, 0x135, 0x1234,
    ];
    for (n, id) in [
        0u16, 2, 3, 4, 6, 7, 8, 0x66, 0x67, 0x68, 0x6b, 0x20f1, 0x3003, 0x1001,
    ]
    .into_iter()
    .enumerate()
    {
        t.put(probe + l::ID, &id.to_le_bytes());
        for model in 0..4u32 {
            for v in [0u32, 1, 2, 6, 8, 9, 10, 11, 101, 110, 0x1_000a, rng.next()] {
                t.check(
                    &format!("Ch_UseAnchorVertex {id:#x} {model} {v}"),
                    0x0083_16d0,
                    &[probe, model, v],
                    |m| Res::Int(cl::ch_use_anchor_vertex(m, probe, model, v)),
                );
            }
        }
        for &map in &maps {
            set_global(t, cl::MAP.sym, map);
            for motion in [
                0u32,
                0x0067_100b,
                0x0067_100c,
                0x0066_1032,
                0x0066_1036,
                0x0066_1037,
            ] {
                t.put_u32(probe + l::MOTION_ID, motion);
                t.put(probe + 0x87c, &[u8::from(n % 3 == 0)]);
                set_global(
                    t,
                    ffx_system::instance::bridge::MAGIC_ID.sym,
                    if n % 2 == 0 { 0xa0 } else { 0x2a0 },
                );
                for a in 0..5u32 {
                    for b in [0u32, 1, 2, 3, 7, 10, 11, 13, 14, 0x1_0000] {
                        t.check(
                            &format!(
                                "Ch_UseLargeCollisionRad {id:#x} {a} {b} map {map:#x} {motion:#x}"
                            ),
                            0x0082_4b40,
                            &[probe, a, b],
                            |m| Res::Float(cl::ch_use_large_collision_rad(m, probe, a, b)),
                        );
                        t.check(
                            &format!("Ch_ShouldDisableClothForMotion {id:#x} {a} {b} map {map:#x} {motion:#x}"),
                            0x0082_4a60,
                            &[probe, a, b],
                            |m| Res::Int(cl::ch_should_disable_cloth_for_motion(m, probe, a, b)),
                        );
                    }
                }
            }
        }
    }
    for id in (0..0x70u16).chain([0x1001, 0xffff]) {
        t.put(probe + l::ID, &id.to_le_bytes());
        for &map in &maps {
            set_global(t, cl::MAP.sym, map);
            for motion in [
                0u32,
                0x2113c,
                0x2113d,
                0x2113f,
                0x21140,
                0x0066_10cc,
                0x8000_0000,
            ] {
                t.put_u32(probe + l::MOTION_ID, motion);
                t.put_u32(probe + l::DIRTY, rng.next());
                t.check(
                    &format!("Ch_ClothSetAtariJimenCheck {id:#x} {map:#x} {motion:#x}"),
                    0x0082_4800,
                    &[probe],
                    |m| {
                        cl::ch_cloth_set_atari_jimen_check(m, probe);
                        Res::Void
                    },
                );
            }
        }
    }
    // resets: bind some characters to others first
    for (i, &chr) in chrs.iter().enumerate() {
        let parent = chrs[(i + 1) % chrs.len()];
        if rng.next() & 1 == 0 {
            t.put_u32(chr + l::BIND_PARENT, parent);
        }
        let f: u32 = rd(&t.b, chr + l::FLAGS);
        t.put_u32(
            chr + l::FLAGS,
            f ^ (rng.next() & (cl::F_CLOTH_READY | cl::F_CLOTH_RESET)),
        );
    }
    for &chr in &chrs {
        for (name, at, f) in [
            (
                "Ch_ClothReset2",
                0x0082_4670u32,
                cl::ch_cloth_reset2 as fn(&mut Oracle, u32),
            ),
            ("Ch_ClothReset", 0x0082_4650, cl::ch_cloth_reset),
            (
                "Ch_ClothResetRelation",
                0x0083_3860,
                cl::ch_cloth_reset_relation,
            ),
        ] {
            let flags: u32 = rd(&t.b, chr + l::FLAGS);
            t.put_u32(
                chr + l::FLAGS,
                (flags & !cl::F_CLOTH_RESET) | cl::F_CLOTH_READY,
            );
            t.check(&format!("{name} {chr:#x}"), at, &[chr], |m| {
                f(m, chr);
                Res::Void
            });
        }
    }
    t.check("Ch_ClothResetAll", 0x0083_3800, &[], |m| {
        cl::ch_cloth_reset_all(m);
        Res::Void
    });
    // a model without a cloth buffer stops the reset
    let models: u32 = rd(&t.b, probe + l::MODELS);
    t.put_u32(models + 0xc, 0);
    let flags: u32 = rd(&t.b, probe + l::FLAGS);
    t.put_u32(probe + l::FLAGS, flags | cl::F_CLOTH_READY);
    t.check("Ch_ClothReset2 no buffer", 0x0082_4670, &[probe], |m| {
        cl::ch_cloth_reset2(m, probe);
        Res::Void
    });
}

#[test]
fn all_and_misc_match_the_original() {
    for mut t in twins() {
        all_and_misc_match_the_original_on(&mut t);
    }
}

fn all_and_misc_match_the_original_on(t: &mut Twin) {
    use ffx_system::instance::bridge as si;
    let mut rng = Xorshift(0x5eed_0204);
    let chrs = characters(t, &mut rng);
    t.check("Ch_SysGetNbUse", 0x0082_85e0, &[], |m| {
        Res::Int(h::ch_sys_get_nb_use(m))
    });
    t.check("Ch_AllReserveCalcNorm", 0x0083_2550, &[], |m| {
        h::ch_all_reserve_calc_norm(m);
        Res::Void
    });
    for on in [0u32, 1, 2, u32::MAX] {
        t.check(
            &format!("Ch_AllSetCalcNorm {on}"),
            0x0083_25b0,
            &[on],
            |m| {
                h::ch_all_set_calc_norm(m, on);
                Res::Void
            },
        );
        for &chr in &chrs {
            t.check(
                &format!("Ch_SetCalcNorm2 {chr:#x} {on}"),
                0x0083_5780,
                &[chr, on],
                |m| Res::Int(p::ch_set_calc_norm2(m, chr, on)),
            );
        }
    }
    for hold in [0u32, 1] {
        for handle in [u32::MAX, 0, 5] {
            for id in [0u32, 0x2a0, 0x2a1] {
                set_global(t, tx::CLUT_DISPOSE_HOLD.sym, hold);
                set_global(t, si::MAGIC_HANDLE.sym, handle);
                set_global(t, si::MAGIC_ID.sym, id);
                t.check(
                    &format!("Ch_ClutDisposeCheck {hold} {handle:#x} {id:#x}"),
                    0x0083_c910,
                    &[],
                    |m| Res::Int(tx::ch_clut_dispose_check(m)),
                );
            }
        }
    }
    let out = t.alloc(0x40);
    for (pk, data) in tx::SPECULAR.into_iter().zip(tx::SPECULAR_DATA) {
        let _ = pk;
        if data.get(&t.b) < 0x1000 {
            let d = random_block(t, &mut rng, 0x40) + 0x20;
            set_global(t, data.sym, d);
        }
    }
    t.check("Ch_TextureSendSpecular", 0x0083_cf80, &[out], |m| {
        Res::Int(tx::ch_texture_send_specular(m, out))
    });
    let wh = t.alloc(8);
    let rows = t.alloc(4 * 0x100);
    for chr in live_characters(t, &mut rng) {
        let arc: u32 = rd(&t.b, in_blk(&t.b, chr, 0x6c));
        if arc == 0 || rd::<u32>(&t.b, arc + 0x18) == 0 {
            continue;
        }
        t.check(
            &format!("Ch_GetBigImageSize {chr:#x}"),
            0x0083_3e60,
            &[chr, wh, wh + 4, rows],
            |m| {
                tx::ch_get_big_image_size(m, chr, wh, wh + 4, rows);
                Res::Void
            },
        );
    }
}

/// Loaded data that moved: built at offsets in `bytes`, its pointers as
/// if it sat at `old`.
struct Blob {
    bytes: Vec<u8>,
    old: u32,
}

impl Blob {
    fn new(rng: &mut Xorshift, len: u32) -> Self {
        Self {
            bytes: random(rng, len),
            old: rng.next() & 0x7fff_fff0,
        }
    }
    fn w32(&mut self, at: u32, v: u32) {
        let a = at as usize;
        self.bytes[a..a + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn w16(&mut self, at: u32, v: u16) {
        let a = at as usize;
        self.bytes[a..a + 2].copy_from_slice(&v.to_le_bytes());
    }
    /// A pointer to `at` inside.
    fn ptr(&mut self, field: u32, at: u32) {
        self.w32(field, self.old.wrapping_add(at));
    }
    /// A pointer elsewhere, sometimes null.
    fn leaf(&mut self, rng: &mut Xorshift, field: u32, nullable: bool) {
        let v = if nullable && rng.next().is_multiple_of(3) {
            0
        } else {
            rng.next() | 1
        };
        self.w32(field, v);
    }
    /// Into both oracles; returns where it went.
    fn place(&self, t: &mut Twin) -> u32 {
        let at = t.alloc(self.bytes.len());
        t.put(at, &self.bytes);
        at
    }

    /// A model header at `at` (0x500 bytes).
    fn model_header(&mut self, rng: &mut Xorshift, at: u32) {
        let version: u16 =
            [0x0830, 0x0831, 0x1029, 0x1313, 0x1314, 0x2000][rng.next() as usize % 6];
        let small = (0x831..0x1314).contains(&version);
        let (stride, nmodels) = if small { (24, 1) } else { (40, 2) };
        self.ptr(at, at);
        self.w16(at + 4, version);
        self.w16(at + 6, nmodels);
        self.ptr(at + 0x10, at + 0x100);
        let t14 = !rng.next().is_multiple_of(3);
        if t14 {
            self.ptr(at + 0x14, at + 0x200);
        } else {
            self.w32(at + 0x14, 0);
        }
        self.w16(at + 8, if t14 { 2 } else { 0 });
        self.ptr(at + 0x18, at + 0x240);
        self.w16(at + 0xc, 3);
        self.leaf(rng, at + 0x1c, false);
        let t24 = !rng.next().is_multiple_of(3);
        if t24 {
            self.ptr(at + 0x24, at + 0x280);
        } else {
            self.w32(at + 0x24, 0);
        }
        self.w16(at + 0x20, if t24 { 2 } else { 0 });
        self.leaf(rng, at + 0x30, true);
        for i in 0..u32::from(nmodels) {
            let e = at + 0x100 + i * stride;
            self.w16(e + 6, 2);
            self.leaf(rng, e + 8, false);
            self.leaf(rng, e + 0xc, false);
            self.ptr(e + 0x10, at + 0x300 + i * 0x40);
            self.w16(e + 0x18, 2);
            self.ptr(e + 0x1c, at + 0x400 + i * 0x80);
            for j in 0..2 {
                self.leaf(rng, at + 0x300 + i * 0x40 + 12 * j + 8, false);
                let c = at + 0x400 + i * 0x80 + 0x24 * j;
                self.leaf(rng, c + 8, false);
                self.leaf(rng, c + 0xc, false);
                self.leaf(rng, c + 0x14, true);
                self.leaf(rng, c + 0x20, true);
            }
        }
        for j in 0..2 {
            self.leaf(rng, at + 0x200 + 12 * j + 8, false);
            self.leaf(rng, at + 0x280 + 12 * j + 8, true);
        }
        for j in 0..3 {
            self.leaf(rng, at + 0x240 + 4 * j, false);
        }
    }

    /// Texture data at `at` (0x300 bytes).
    fn tex_data(&mut self, rng: &mut Xorshift, at: u32) {
        self.ptr(at, at);
        self.w16(at + 8, 3);
        self.w16(at + 0xc, 3);
        let version: u16 = [0x1028, 0x1029, 0x1125, 0x1126, 0x2000][rng.next() as usize % 5];
        self.w16(at + 0x10, version);
        self.leaf(rng, at + 0x14, false);
        self.ptr(at + 0x18, at + 0x40);
        self.ptr(at + 0x1c, at + 0x80);
        self.leaf(rng, at + 0x20, true);
        for i in 0..3 {
            self.ptr(at + 0x40 + 8 * i + 4, at + 0x100 + 0x40 * i);
            let key: [u8; 3] = match rng.next() % 4 {
                0 => [0, 0xff, 0],
                1 => [0xff, 0, 0],
                2 => [0xff, 0, 1],
                _ => [0, 0xff, 1],
            };
            let c = (at + 0x100 + 0x40 * i) as usize;
            self.bytes[c..c + 3].copy_from_slice(&key);
            self.leaf(rng, at + 0x80 + 16 * i + 0xc, true);
        }
    }

    /// A character archive at 0 (0x1000 bytes).
    fn archive(&mut self, rng: &mut Xorshift) {
        self.ptr(0, 0);
        let n = [6u32, 10, 11, 9][rng.next() as usize % 4];
        self.w32(4, n);
        for i in 0..n {
            self.leaf(rng, 0x10 + 8 * i, true);
            self.w32(0x14 + 8 * i, rng.next() & 0xffff);
        }
        self.ptr(0x10, 0x200);
        self.model_header(rng, 0x200);
        if rng.next().is_multiple_of(4) {
            self.w32(0x18, 0);
        } else {
            self.ptr(0x18, 0x700);
            self.tex_data(rng, 0x700);
        }
        self.ptr(0x30, 0xb00);
        self.w32(0x34, 3);
        for j in 0..3 {
            self.leaf(rng, 0xb00 + 4 * j, false);
        }
        if n > 9 {
            self.ptr(0x58, 0xc00);
            self.w32(0xc00, [0x1623u32, 0x1624, 0x2000][rng.next() as usize % 3]);
            self.w32(0xc0c, small_float(rng));
        }
    }
}

#[test]
fn mapping_matches_the_original() {
    for mut t in twins() {
        mapping_matches_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn mapping_matches_the_original_on(t: &mut Twin) {
    use ffx_chara::mapping::{self as mp, bridge as mb};
    let mut rng = Xorshift(0x5eed_0205);
    for _ in 0..300 {
        let (i, base, v) = (
            rng.next() % 300,
            rng.next(),
            [0x830, 0x831, 0x1313, 0x1314, rng.next()][rng.next() as usize % 5],
        );
        t.check(
            &format!("Chr_ModelEntry {i} {base:#x} {v:#x}"),
            0x0082_8be0,
            &[i, base, v],
            |_| Res::Int(mp::model_entry(i, base, v)),
        );
    }
    for round in 0..40 {
        let mut b = Blob::new(&mut rng, 0x500);
        b.model_header(&mut rng, 0);
        let at = b.place(t);
        for k in 0..2 {
            t.check(
                &format!("Ch_ModelMapping {round} {k}"),
                0x0082_7590,
                &[at],
                |m| {
                    mb::ch_model_mapping(m, at);
                    Res::Void
                },
            );
        }
        let mut b = Blob::new(&mut rng, 0x300);
        b.tex_data(&mut rng, 0);
        let at = b.place(t);
        for (k, a) in [at, at, 0].into_iter().enumerate() {
            t.check(
                &format!("Ch_TexDataMapping {round} {k}"),
                0x0083_cba0,
                &[a],
                |m| {
                    mb::ch_tex_data_mapping(m, a);
                    Res::Void
                },
            );
        }
        let mut b = Blob::new(&mut rng, 0x1000);
        b.archive(&mut rng);
        let arc = b.place(t);
        if round % 2 == 0 {
            for k in 0..2 {
                t.check(
                    &format!("Ch_ArcMapping {round} {k}"),
                    0x0082_56f0,
                    &[arc],
                    |m| {
                        mb::ch_arc_mapping(m, arc);
                        Res::Void
                    },
                );
            }
        }
        let blk = random_block(t, &mut rng, l::BLK_SIZE);
        let kind =
            [0x2000u32, 0x1000, 0x3000][rng.next() as usize % 3] | (rng.next() & 0xffff_0fff);
        t.put_u32(blk, kind);
        t.check(
            &format!("Ch_CbArcMapping {round}"),
            0x0082_5ee0,
            &[blk, arc],
            |m| {
                mb::ch_cb_arc_mapping(m, blk, arc);
                Res::Void
            },
        );
        // a character takes the block
        let chr = make_character(t, &mut rng, None);
        let name: &[u8] = [&b"c001\0"[..], b"c101\0", b"c0011", b"c10\0", b"m001\0"][round % 5];
        t.put(blk + l::BLK_NAME, name);
        if round % 4 == 3 {
            t.put_u32(chr + l::MODELS, 0);
        }
        t.check(
            &format!("Ch_CbMapping {round}"),
            0x0082_6070,
            &[chr, blk],
            |m| {
                mb::ch_cb_mapping(m, chr, blk);
                Res::Void
            },
        );
    }
    let chrs = live_characters(t, &mut rng);
    for &chr in &chrs {
        let elems: u32 = rd(&t.b, chr + l::ELEMS);
        let hdr: u32 = rd(&t.b, chr + l::MODEL_HEADER);
        let n = u32::from(rd::<u16>(&t.b, hdr + l::HEADER_ELEMS));
        if n > 0 && rng.next() & 1 == 0 {
            t.put_u32(elems, chr + 0x1c8);
        }
        let d = rng.next();
        for (k, d) in [d, d.wrapping_neg(), 0].into_iter().enumerate() {
            t.check(
                &format!("Ch_MoveMgrp {chr:#x} {k}"),
                0x0083_7590,
                &[chr, d],
                |m| {
                    mb::ch_move_mgrp(m, chr, d);
                    Res::Void
                },
            );
            t.check(
                &format!("Ch_ElemRemapping {chr:#x} {k}"),
                0x0082_6980,
                &[chr, d],
                |m| {
                    mb::ch_elem_remapping(m, chr, d);
                    Res::Void
                },
            );
        }
        for (k, d) in [d, d.wrapping_neg(), 0].into_iter().enumerate() {
            t.check(
                &format!("Ch_MotionRemappingFcurve {chr:#x} {k}"),
                0x0083_9890,
                &[chr, d],
                |m| {
                    mb::ch_motion_remapping_fcurve(m, chr, d);
                    Res::Void
                },
            );
        }
        for copy in [0u32, 1] {
            t.check(
                &format!("Ch_ClutMapping {chr:#x} {copy}"),
                0x0083_ca20,
                &[chr, copy],
                |m| {
                    mb::ch_clut_mapping(m, chr, copy);
                    Res::Void
                },
            );
        }
    }
}

/// A void result, after the call in its argument.
macro_rules! void {
    ($e:expr) => {{
        $e;
        Res::Void
    }};
}

#[test]
fn settings_and_accessors_match_the_original() {
    for mut t in twins() {
        settings_and_accessors_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn settings_and_accessors_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0301);
    for &v in INTS.iter().chain(&[rng.next(), rng.next()]) {
        t.check(&format!("Ch_SetDrawIcon {v:#x}"), 0x0083_0c70, &[v], |m| {
            void!(sy::set_bit0(m, sy::DRAW_ICON, v))
        });
        t.check(&format!("Ch_SetDrawIcon2 {v:#x}"), 0x0083_0c80, &[v], |m| {
            void!(sy::set_bit0(m, sy::DRAW_ICON2, v))
        });
        t.check(&format!("Ch_SysSet7e4 {v:#x}"), 0x0083_0d00, &[v], |m| {
            void!(sy::SYS_7E4.set(m, v))
        });
        t.check(&format!("Ch_SysSet6e0 {v:#x}"), 0x0083_20e0, &[v], |m| {
            void!(sy::SYS_6E0.set(m, v))
        });
        set_global(t, sy::SWITCH_7D8.sym, v.rotate_left(8));
        t.check(
            &format!("Ch_SysSetSwitch7d8 {v:#x}"),
            0x0083_17b0,
            &[v],
            |m| int(sy::ch_sys_set_switch_7d8(m, v)),
        );
        let at = t.a.rebase(p::SHADE_BY_LIGHT.sym);
        t.put(at, &[v.to_le_bytes()[0]]);
        t.check(
            &format!("Ch_DrawGetShadeByLight {v:#x}"),
            0x0083_17d0,
            &[],
            |m| int(sy::get_i8(m, p::SHADE_BY_LIGHT)),
        );
    }
    t.check("Ch_SetAllocateErr", 0x0082_85d0, &[], |m| {
        void!(sy::ch_set_allocate_err(m))
    });
    t.check("Ch_FunctionHistoryClear", 0x0083_3760, &[], |m| {
        void!(sy::CALC_NORM_CURSOR.set(m, 0))
    });
    let outs = t.alloc(12);
    let chrs = live_characters(t, &mut rng);
    for &chr in &chrs {
        for &bits in &FLOATS {
            t.put(
                chr + 0x804,
                &[
                    bits.to_le_bytes(),
                    quiet(rng.next()).to_le_bytes(),
                    bits.to_le_bytes(),
                ]
                .concat(),
            );
            for mask in 0..8u32 {
                let o = |k: u32| if mask >> k & 1 == 1 { outs + 4 * k } else { 0 };
                let (a, b, c) = (o(0), o(1), o(2));
                t.check(
                    &format!("Ch_WindGet {chr:#x} {bits:#x} {mask}"),
                    0x0082_4c30,
                    &[chr, a, b, c],
                    |m| void!(c::ch_wind_get(m, chr, a, b, c)),
                );
            }
        }
        t.check(
            &format!("Ch_WindGetEnable {chr:#x}"),
            0x0082_4c70,
            &[chr],
            |m| Res::Int(c::ch_wind_get_enable(m, chr)),
        );
        let (a, b) = (rng.next(), rng.next());
        t.check(
            &format!("Ch_Set7f8 {chr:#x}"),
            0x0083_3940,
            &[chr, a, b],
            |m| void!(c::ch_set_7f8(m, chr, a, b)),
        );
        t.check(
            &format!("Chr_GetU16 {chr:#x}"),
            0x0082_ba20,
            &[chr + 1],
            |m| Res::Int(c::chr_get_u16(m, chr + 1)),
        );
        t.check(
            &format!("Ch_CalcHeight {chr:#x}"),
            0x0083_0b20,
            &[chr],
            |m| void!(c::ch_calc_height(m, chr)),
        );
        // the vertices' heights: some NaN, infinite, negative
        let n = u32::from(rd::<u16>(
            &t.b,
            rd::<u32>(&t.b, chr + l::MODEL_HEADER) + l::HEADER_MODELS,
        ));
        let models: u32 = rd(&t.b, chr + l::MODELS);
        for i in 0..n.min(8) {
            let data: u32 = rd(&t.b, models + i * l::MODEL_SIZE);
            let verts: u32 = rd(&t.b, models + i * l::MODEL_SIZE + 8);
            let count = u32::from(rd::<u16>(&t.b, data + 4)).min(64);
            for k in 0..count {
                let v = FLOATS[rng.next() as usize % FLOATS.len()];
                t.put_u32(verts + 16 * k + 4, v);
            }
        }
        t.check(
            &format!("Ch_CalcHeight {chr:#x} odd"),
            0x0083_0b20,
            &[chr],
            |m| void!(c::ch_calc_height(m, chr)),
        );
        t.check(
            &format!("Ch_MotionSetNull {chr:#x}"),
            0x0083_7540,
            &[chr],
            |m| void!(ffx_chara::motion::bridge::ch_motion_set_null(m, chr)),
        );
        for v in [0u32, 1, 0x1ff, u32::MAX] {
            t.check(
                &format!("Ch_MotionSet737 {chr:#x} {v:#x}"),
                0x0083_7570,
                &[chr, v],
                |m| Res::Int(ffx_chara::motion::bridge::ch_motion_set_737(m, chr, v)),
            );
        }
    }
}

#[test]
fn handles_and_elements_match_the_original() {
    for mut t in twins() {
        handles_and_elements_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn handles_and_elements_match_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0302);
    let chrs = characters(t, &mut rng);
    let array = h::CHR_ARRAY.get(&t.b);
    let mut ids: Vec<u32> = chrs
        .iter()
        .map(|&c| u32::from(rd::<u16>(&t.b, c)))
        .collect();
    ids.extend([0, 0xffff, 0x1_0000 | ids[0], rng.next()]);
    for &id in &ids {
        t.check(
            &format!("Ch_HandleFromId {id:#x}"),
            0x0082_61f0,
            &[id],
            |m| Res::Int(h::ch_handle_from_id(m, id)),
        );
    }
    let pp = t.alloc(8);
    for (i, &a) in chrs.iter().enumerate() {
        let b = chrs[(i + 1) % chrs.len()];
        t.put_u32(pp, a);
        t.put_u32(pp + 4, b);
        t.check(
            &format!("Ch_CompareDrawOrder {a:#x} {b:#x}"),
            0x0083_20f0,
            &[pp, pp + 4],
            |m| Res::Int(h::ch_compare_draw_order(m, pp, pp + 4)),
        );
    }
    // bind some to the first slot, then unbind its children
    for &c in &chrs {
        if rng.next() & 1 == 0 {
            t.put_u32(c + l::BIND_PARENT, array);
        }
    }
    for chr in [0, array] {
        t.check(
            &format!("Ch_UnbindChildren {chr:#x}"),
            0x0082_6150,
            &[chr],
            |m| void!(h::ch_unbind_children(m, chr)),
        );
    }
    t.check("Ch_AllClearFlags24", 0x0083_2db0, &[], |m| {
        void!(h::ch_all_clear_flags24(m))
    });
    let table = t.a.rebase(p::EXCEPTION_IDS.sym);
    let mut ids: Vec<u32> = (0..9).map(|i| rd::<u32>(&t.b, table + 4 * i)).collect();
    ids.extend([0, 1, 0x3003, rng.next()]);
    for &id in &ids {
        t.check(
            &format!("Ch_IsNotExceptionId {id:#x}"),
            0x0083_1780,
            &[id],
            |m| Res::Int(p::ch_is_not_exception_id(m, id)),
        );
    }
    let holder = t.alloc(0x40);
    for (k, &chr) in chrs.iter().chain(&[0]).enumerate() {
        t.put_u32(holder, chr);
        t.put(holder + 0x3c, &[u8::from(k % 2 == 0), u8::from(k % 3 == 0)]);
        t.check(
            &format!("Ch_SetCalcNorm2 holder {chr:#x}"),
            0x0079_69c0,
            &[holder],
            |m| Res::Int(p::ch_set_calc_norm2_holder(m, holder)),
        );
    }
    let made_up = *chrs.last().unwrap();
    let motion = t.alloc(8 + 2 * 8);
    t.put(motion + 2, &5u16.to_le_bytes());
    for i in 0..8u32 {
        t.put(
            motion + 8 + 2 * i,
            &u16::try_from(rng.next() % 6).unwrap().to_le_bytes(),
        );
    }
    for &chr in &chrs {
        if chr == made_up {
            t.put_u32(chr + 0x758, 0);
        }
        for round in 0..2 {
            if round == 1 && chr == made_up {
                t.put_u32(chr + 0x758, motion);
            }
            for e in (0..8u32).chain([0xffff, rng.next()]) {
                t.check(
                    &format!("Ch_ElemInMotion {chr:#x} {e:#x} {round}"),
                    0x0083_42c0,
                    &[chr, e],
                    |m| Res::Int(p::ch_elem_in_motion(m, chr, e)),
                );
            }
        }
    }
}

#[test]
fn motion_frames_match_the_original() {
    for mut t in twins() {
        motion_frames_match_the_original_on(&mut t);
    }
}

fn motion_frames_match_the_original_on(t: &mut Twin) {
    use ffx_chara::motion::{self as mo, bridge as mb};
    let mut rng = Xorshift(0x5eed_0303);
    let edges = [i32::MIN, -2, -1, 0, 1, 2, 3, 5, i32::MAX];
    for &a in &edges {
        for &b in &edges {
            for &x in &edges {
                for &y in &edges[2..6] {
                    let args = [a, b, x, y].map(i32::cast_unsigned);
                    t.check(
                        &format!("Chr_RangeClass {a} {b} {x} {y}"),
                        0x0083_1330,
                        &args,
                        |_| Res::Int(mo::range_class(a, b, x, y)),
                    );
                }
            }
        }
    }
    let chr = make_character(t, &mut rng, None);
    for i in 0..3000 {
        let frame = [rng.next() % 0x4000, rng.next(), 0x8000_0000 | rng.next()][i % 3];
        let end = rng.next() % 0x4000;
        let len = [0, 0x100, rng.next() % 0x2000, rng.next()][rng.next() as usize % 4];
        let loops = [0u16, 1, 2, 3, 0xffff][rng.next() as usize % 5];
        t.put_u32(chr + 0x740, frame);
        t.put_u32(chr + 0x71c, end);
        t.put_u32(chr + 0x720, len);
        t.put(chr + 0x724, &loops.to_le_bytes());
        t.put(
            chr + 0x756,
            &u16::try_from(rng.next() % 0x400).unwrap().to_le_bytes(),
        );
        t.check(
            &format!("Ch_MotionLoopFrame {i} {frame:#x} {end:#x} {len:#x} {loops}"),
            0x0083_92d0,
            &[chr],
            |m| Res::Int(mb::ch_motion_loop_frame(m, chr)),
        );
    }
    // the sequence table: as found, then made up
    let base = t.a.rebase(mb::SEQUENCES.sym);
    for round in 0..20 {
        if round > 0 {
            for i in 0..100u32 {
                let e = base + 0x14 * i;
                let data = if rng.next().is_multiple_of(3) {
                    0
                } else {
                    rng.next() | 1
                };
                t.put_u32(e, data);
                t.put(e + 4, &u16::try_from(rng.next() % 6).unwrap().to_le_bytes());
                t.put(e + 6, &u16::try_from(rng.next() % 2).unwrap().to_le_bytes());
            }
        }
        t.check(&format!("Ch_SeqLink {round}"), 0x0083_8170, &[], |m| {
            void!(mb::ch_seq_link(m))
        });
    }
}

#[test]
fn matrices_and_texanim_match_the_original() {
    for mut t in twins() {
        matrices_and_texanim_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn matrices_and_texanim_match_the_original_on(t: &mut Twin) {
    use ffx_chara::matrix::bridge as mx;
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x5eed_0304);
    let chrs = live_characters(t, &mut rng);
    let made_up = *chrs.last().unwrap();
    let out = t.alloc(64);
    for &chr in &chrs {
        for o in [out, 0] {
            t.check(
                &format!("Ch_GetRootGlobalMatrix {chr:#x} {o:#x}"),
                0x0083_3f80,
                &[chr, o],
                |m| Res::Int(mx::ch_get_root_global_matrix(m, chr, o)),
            );
            t.check(
                &format!("Ch_GetRootLightMatrix {chr:#x} {o:#x}"),
                0x0083_3fc0,
                &[chr, o],
                |m| Res::Int(mx::ch_get_root_light_matrix(m, chr, o)),
            );
            let n = u32::from(rd::<u16>(
                &t.b,
                rd::<u32>(&t.b, chr + l::MODEL_HEADER) + l::HEADER_MODELS,
            ));
            for i in 0..n.min(4) {
                t.check(
                    &format!("Ch_ModelGetBaseMatrix {chr:#x} {i} {o:#x}"),
                    0x0083_0500,
                    &[chr, i, o],
                    |m| Res::Int(mx::ch_model_get_base_matrix(m, chr, i, o)),
                );
                t.check(
                    &format!("Ch_ModelGetMatrix {chr:#x} {i} {o:#x}"),
                    0x0083_0550,
                    &[chr, i, o],
                    |m| void!(mx::ch_model_get_matrix(m, chr, i, o)),
                );
            }
        }
        t.check(
            &format!("Ch_GetRootScaledMatrix {chr:#x}"),
            0x0082_6a80,
            &[out, chr],
            |m| void!(mx::ch_get_root_scaled_matrix(m, out, chr)),
        );
    }
    let e = random_block(t, &mut rng, 0x160);
    t.check("Ch_ElemInit", 0x0082_8b60, &[e], |m| {
        Res::Int(mx::ch_elem_init(m, e))
    });

    // texture animation
    let table = t.a.rebase(ta::TEXANIM.sym);
    let nm = t.alloc(8);
    t.check("chr_texanim_count", 0x0077_cdc0, &[nm, nm + 4], |m| {
        void!(ta::chr_texanim_count(m, nm, nm + 4))
    });
    t.check("chr_texanim_init", 0x0077_c870, &[], |m| {
        void!(ta::chr_texanim_init(m))
    });
    for i in 0..64u32 {
        let e = table + 0x30 * i;
        if rng.next() & 1 == 0 {
            t.put_u32(e, rng.next() | 1);
        }
        let work = random_block(t, &mut rng, 0xd28 + 4 * 8);
        t.put_u32(e + 4, work);
        t.put(e + 0xd, &[u8::try_from(rng.next() % 3).unwrap()]);
    }
    t.check("chr_texanim_count set", 0x0077_cdc0, &[nm, nm + 4], |m| {
        void!(ta::chr_texanim_count(m, nm, nm + 4))
    });
    let src = random_block(t, &mut rng, 0x30);
    let recs = random_block(t, &mut rng, 8 * 16);
    t.put_u32(src + 0x1c, recs);
    for i in [0u32, 1, 2, 17, 63] {
        t.put(
            src + 0xc,
            &u16::try_from(rng.next() % 9).unwrap().to_le_bytes(),
        );
        t.check(
            &format!("chr_texanim_memory_move {i}"),
            0x0077_c900,
            &[src, i],
            |m| void!(ta::chr_texanim_memory_move(m, src, i)),
        );
        t.check(
            &format!("chr_texanim_memory_move_new {i}"),
            0x0077_f970,
            &[src, i],
            |m| void!(ta::chr_texanim_memory_move_new(m, src, i)),
        );
        t.check(
            &format!("chr_texanim_memory_move_old {i}"),
            0x0078_03a0,
            &[src, i],
            |m| void!(ta::chr_texanim_memory_move_old(m, src, i)),
        );
    }
    let sw_src = random_block(t, &mut rng, 0x100);
    let sw_dst = t.alloc(0x40);
    for which in [0u32, 1, 0x100] {
        t.check(
            &format!("chr_texanim_clut_swizzle {which:#x}"),
            0x0077_ca10,
            &[sw_dst, sw_src, which],
            |m| void!(ta::chr_texanim_clut_swizzle(m, sw_dst, sw_src, which)),
        );
    }
    let img = t.alloc(8);
    let tex = random_block(t, &mut rng, 0x40);
    for bpp in [0u32, 4, 8, 0x10, 0x11, 0x20, u32::MAX] {
        t.put_u32(img, bpp);
        t.put_u32(img + 4, rng.next());
        t.check(
            &format!("Ch_TextureSetClut {bpp:#x}"),
            0x0083_d1e0,
            &[img, tex],
            |m| void!(ta::ch_texture_set_clut(m, img, tex)),
        );
    }
    // the texture list
    let list = random_block(t, &mut rng, 4 * 0x20);
    for i in 0..4u32 {
        t.put(
            list + 0x20 * i,
            &u16::try_from(rng.next() % 3).unwrap().to_le_bytes(),
        );
        t.put(
            list + 0x20 * i + 2,
            &u16::try_from(rng.next() % 3).unwrap().to_le_bytes(),
        );
    }
    for &chr in &chrs {
        for (k, (l6e0, n)) in [(0u32, 2i32), (list, 4), (list, 0), (list, -1)]
            .into_iter()
            .enumerate()
        {
            if chr == made_up {
                t.put_u32(chr + 0x6e0, l6e0);
                t.put_u32(chr + 0x6f4, n.cast_unsigned());
            } else if k > 0 {
                continue;
            }
            t.check(
                &format!("Ch_TextureSetTex0 {chr:#x} {k}"),
                0x0083_13f0,
                &[chr],
                |m| void!(ta::ch_texture_set_tex0(m, chr)),
            );
        }
    }
}

#[test]
fn heap_relink_matches_the_original() {
    for mut t in twins() {
        heap_relink_matches_the_original_on(&mut t);
    }
}

fn heap_relink_matches_the_original_on(t: &mut Twin) {
    let mut rng = Xorshift(0x5eed_0305);
    for round in 0..50 {
        let n = 1 + rng.next() % 10;
        let blocks: Vec<u32> = (0..=n).map(|_| random_block(t, &mut rng, 0x10)).collect();
        for (i, &b) in blocks.iter().enumerate() {
            let next = blocks.get(i + 1).copied().unwrap_or(0);
            t.put_u32(b, next);
            let kind = if i == blocks.len() - 1 {
                0xf
            } else {
                [0u32, 0, 1, 2][rng.next() as usize % 4]
            };
            t.put_u32(b + 0xc, (rng.next() & !0xf) | kind);
        }
        let heap = random_block(t, &mut rng, 0x10);
        t.put_u32(heap + 0xc, blocks[0]);
        t.check(
            &format!("Ch_MemRelinkBack {round}"),
            0x0083_c6a0,
            &[heap],
            |m| void!(ffx_chara::mapping::bridge::ch_mem_relink_back(m, heap)),
        );
    }
}

#[test]
fn optpos_calc_matches_the_original() {
    for mut t in twins() {
        optpos_calc_matches_the_original_on(&mut t);
    }
}

fn optpos_calc_matches_the_original_on(t: &mut Twin) {
    use ffx_chara::attach::bridge as at;
    let mut rng = Xorshift(0x5eed_0401);
    let chrs = live_characters(t, &mut rng);
    for round in 0..4 {
        for &chr in &chrs {
            if round > 0 {
                // odd heights and positions
                for off in [l::POS + 4, l::CAM_HEIGHT, 0x6a8, 0x698] {
                    t.put_u32(chr + off, any_float(&mut rng));
                }
                for off in [0x1d0u32, 0x1e0, 0x1f0, 0x200] {
                    t.put_u32(chr + off + 4 * (rng.next() % 4), small_float(&mut rng));
                }
            }
            if round == 3 {
                let b: u32 = rd(&t.b, chr + l::BLK);
                t.put_u32(b + l::BLK_DATA_SCALE, any_float(&mut rng));
            }
            t.check(
                &format!("Ch_OptposCalc {chr:#x} {round}"),
                0x0083_50f0,
                &[chr],
                |m| void!(at::ch_optpos_calc(m, chr)),
            );
        }
    }
}

#[test]
fn set_sequence_matches_the_original() {
    for mut t in twins() {
        set_sequence_matches_the_original_on(&mut t);
    }
}

fn set_sequence_matches_the_original_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mb;
    let mut rng = Xorshift(0x5eed_0402);
    let chrs = live_characters(t, &mut rng);
    // what the snapshot plays now, before anything is made up
    let real = if t.name == "snapshot" {
        chrs.len() - 1
    } else {
        0
    };
    for &c in &chrs[..real] {
        let id: u32 = rd(&t.b, c + l::MOTION_ID);
        t.check(
            &format!("Ch_SetSequence {c:#x} same"),
            0x0083_7ac0,
            &[c, id],
            |m| Res::Int(mb::ch_set_sequence(m, c, id)),
        );
    }
    // a made-up sequence table: keys 0..4, motions numbered 0..8
    let base = t.a.rebase(mb::SEQUENCES.sym);
    for i in 0..100u32 {
        let e = base + 0x14 * i;
        if rng.next().is_multiple_of(3) {
            t.put_u32(e, 0);
            continue;
        }
        let n = rng.next() % 4;
        let d = random_block(t, &mut rng, 0x10);
        let list = random_block(t, &mut rng, 16 * 4);
        t.put(d + 8, &u16::try_from(n).unwrap().to_le_bytes());
        t.put_u32(d + 0xc, list);
        for k in 0..4 {
            t.put(
                list + 16 * k,
                &u16::try_from(rng.next() % 9).unwrap().to_le_bytes(),
            );
        }
        t.put_u32(e, d);
        t.put(e + 4, &u16::try_from(rng.next() % 5).unwrap().to_le_bytes());
        t.put_u32(e + 0x10, rng.next());
    }
    t.check("Ch_SeqLink for Ch_SetSequence", 0x0083_8170, &[], |m| {
        void!(mb::ch_seq_link(m))
    });
    // the model blocks' motion lists
    for &c in &chrs {
        let b: u32 = rd(&t.b, c + l::BLK);
        let motions: Vec<u32> = (0..4)
            .map(|_| {
                let p = t.alloc(4);
                t.put(p, &u16::try_from(rng.next() % 5).unwrap().to_le_bytes());
                p
            })
            .collect();
        let list = t.alloc(16);
        let table = t.alloc(0x20);
        t.put_u32(table + 4, 3);
        for (k, &p) in (0u32..).zip(&motions) {
            t.put_u32(list + 4 * k, p);
            t.put_u32(table + 0x10 + 4 * k, p);
        }
        let flags: u8 = rd(&t.b, b + 0x70);
        t.put(
            b + 0x70,
            &[(flags & !1) | u8::try_from(rng.next() & 1).unwrap()],
        );
        let tbl = if rng.next().is_multiple_of(4) {
            0
        } else {
            table
        };
        t.put_u32(b + 0x10c, tbl);
        t.put(b + 0x110, &3u16.to_le_bytes());
        let motion_list = if rng.next().is_multiple_of(4) {
            0
        } else {
            list
        };
        t.put_u32(b + 0x114, motion_list);
    }
    for i in 0..400 {
        let c = chrs[i % chrs.len()];
        let own = u32::from(rd::<u16>(&t.b, c + l::ID));
        let key = [own, rng.next() % 5, 0x8001][rng.next() as usize % 3];
        let mut id = (key << 16) | (rng.next() % 10);
        if i % 7 == 0 {
            t.put_u32(c + l::MOTION_730, rng.next() & 1);
        }
        if i % 5 == 0 {
            id = rd(&t.b, c + l::MOTION_ID);
        }
        t.check(
            &format!("Ch_SetSequence {c:#x} {id:#x}"),
            0x0083_7ac0,
            &[c, id],
            |m| Res::Int(mb::ch_set_sequence(m, c, id)),
        );
    }
}

#[test]
fn cloth_matrix_matches_the_original() {
    for mut t in twins() {
        cloth_matrix_matches_the_original_on(&mut t);
    }
}

fn cloth_matrix_matches_the_original_on(t: &mut Twin) {
    use ffx_chara::cloth::bridge as cl;
    let mut rng = Xorshift(0x5eed_0403);
    let chrs = live_characters(t, &mut rng);
    let made_up = *chrs.last().unwrap();
    for round in 0..6 {
        for &chr in &chrs {
            let flags: u32 = rd(&t.b, chr + l::FLAGS);
            let ready = if round == 1 { 0 } else { cl::F_CLOTH_READY };
            t.put_u32(chr + l::FLAGS, (flags & !cl::F_CLOTH_READY) | ready);
            let rc: u32 = rd(&t.b, chr + 0x834);
            if round >= 2 || chr == made_up || rc == 0 || t.name == "synthetic" {
                // a made-up render cloth: two lists of meshes of models 0..n
                let n = u32::from(rd::<u16>(
                    &t.b,
                    rd::<u32>(&t.b, chr + l::MODEL_HEADER) + l::HEADER_MODELS,
                ))
                .min(4);
                if n == 0 {
                    continue;
                }
                let rc = random_block(t, &mut rng, 0x40);
                t.put_u32(rc + 0xc, [0u32, 1, 2, rng.next()][round % 4]);
                t.put_u32(rc + 0x14, 2);
                for i in 0..2u32 {
                    let list = random_block(t, &mut rng, 4 + 4 * 4);
                    t.put_u32(list, 1 + rng.next() % 4);
                    for k in 0..4u32 {
                        let mesh = if rng.next().is_multiple_of(5) {
                            0
                        } else {
                            let mesh = random_block(t, &mut rng, 0x50);
                            let v = random_block(t, &mut rng, 16 * 4);
                            t.put(
                                mesh + 0x40,
                                &u16::try_from(rng.next() % n).unwrap().to_le_bytes(),
                            );
                            t.put_u32(mesh + 0x44, rng.next() % 5);
                            t.put_u32(mesh + 0x4c, v);
                            mesh
                        };
                        t.put_u32(list + 4 + 4 * k, mesh);
                    }
                    t.put_u32(rc + 0x30 + 4 * i, list);
                }
                t.put_u32(chr + 0x834, rc);
            }
            t.check(
                &format!("Ch_UpdateClothMatrix {chr:#x} {round}"),
                0x0083_14c0,
                &[chr],
                |m| void!(cl::ch_update_cloth_matrix(m, chr)),
            );
        }
    }
}

#[test]
fn packet_poly_matches_the_original() {
    for mut t in twins() {
        packet_poly_matches_the_original_on(&mut t);
    }
}

fn packet_poly_matches_the_original_on(t: &mut Twin) {
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x5eed_0404);
    let list = random_block(t, &mut rng, 4 * 0x20);
    for i in 0..4u32 {
        let (w, h) =
            [(0u16, 0u16), (1, 1), (64, 128), (256, 0), (0xffff, 3)][rng.next() as usize % 5];
        t.put(
            list + 0x20 * i + 8,
            &[w.to_le_bytes(), h.to_le_bytes()].concat(),
        );
    }
    let verts = t.alloc(3 * 8);
    let buf = t.alloc(0x2000);
    let st = t.a.rebase(sy::PACKET_STATE.sym);
    for round in 0..600 {
        if round % 50 == 0 {
            let p = if round % 100 == 0 { 0 } else { buf };
            set_global(t, sy::PACKET_PTR.sym, p);
            set_global(
                t,
                ta::TEX0_LIST.sym,
                if round % 150 == 0 { 0 } else { list },
            );
            set_global(t, sy::PACKET_SIZE.sym, rng.next() % 0x1000);
            set_global(t, sy::PACKET_MAX.sym, rng.next() % 0x200);
            set_global(
                t,
                sy::PACKET_MAX.sym + 4,
                (rng.next() % 0x200).wrapping_sub(0x10),
            );
            // a new block first: the state differs from the block's
            t.put(st + 4, &0xffffu16.to_le_bytes());
        }
        if rng.next().is_multiple_of(4) {
            let k = rng.next() % 4;
            let v = if k == 0 {
                rng.next() % 4
            } else {
                rng.next() % 3
            };
            t.put(st + 8 * k, &u16::try_from(v).unwrap().to_le_bytes());
        }
        for i in 0..12u32 {
            let v = [rng.next() % 0x400, rng.next() & 0xffff][rng.next() as usize % 2];
            t.put(verts + 2 * i, &u16::try_from(v).unwrap().to_le_bytes());
        }
        let (a, b, c) = (verts, verts + 8, verts + 16);
        t.check(
            &format!("Ch_EffectPacketStorePoly {round}"),
            0x0084_5820,
            &[a, b, c],
            |m| void!(sy::ch_effect_packet_store_poly(m, a, b, c)),
        );
    }
}

#[test]
fn frac_step_matches_the_original() {
    for mut t in twins() {
        frac_step_matches_the_original_on(&mut t);
    }
}

fn frac_step_matches_the_original_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mb;
    let mut rng = Xorshift(0x5eed_0405);
    let mut cases: Vec<[u32; 3]> = Vec::new();
    for &a in &FLOATS {
        for &b in &FLOATS {
            cases.push([a, b, [1u32, 3, 0, u32::MAX][rng.next() as usize % 4]]);
        }
    }
    for _ in 0..4000 {
        cases.push([
            small_float(&mut rng),
            small_float(&mut rng),
            rng.next() % 64,
        ]);
        cases.push([any_float(&mut rng), small_float(&mut rng), rng.next()]);
    }
    for [a, b, n] in cases {
        t.check(
            &format!("Chr_FracStep {a:#x} {b:#x} {n}"),
            0x0083_9f50,
            &[a, b, n],
            |m| Res::Float(mb::chr_frac_step(m, fl(a), fl(b), n.cast_signed())),
        );
    }
}

#[test]
fn elements_and_vectors_match_the_original() {
    for mut t in twins() {
        elements_and_vectors_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn elements_and_vectors_match_the_original_on(t: &mut Twin) {
    use ffx_chara::matrix::bridge as mx;
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x5eed_0406);
    let chrs = live_characters(t, &mut rng);
    let out = t.alloc(64);
    let v = t.alloc(16);
    for &chr in &chrs {
        for on in [0u32, 1, 2, 3, u32::MAX] {
            t.check(
                &format!("Ch_WindSetEnable {chr:#x} {on}"),
                0x0082_4cc0,
                &[chr, on],
                |m| void!(c::ch_wind_set_enable(m, chr, on)),
            );
        }
        let n = u32::from(rd::<u16>(
            &t.b,
            rd::<u32>(&t.b, chr + l::MODEL_HEADER) + l::HEADER_ELEMS,
        ))
        .min(6);
        for e in 0..n {
            for o in [out, 0] {
                t.check(
                    &format!("Ch_ElemGetBaseMatrix {chr:#x} {e} {o:#x}"),
                    0x0083_3b80,
                    &[chr, e, o],
                    |m| Res::Int(mx::ch_elem_get_base_matrix(m, chr, e, o)),
                );
                t.check(
                    &format!("Ch_ElemGetMatrix {chr:#x} {e} {o:#x}"),
                    0x0083_3c30,
                    &[chr, e, o],
                    |m| void!(mx::ch_elem_get_matrix(m, chr, e, o)),
                );
            }
            t.put(
                v,
                &[
                    small_float(&mut rng),
                    small_float(&mut rng),
                    small_float(&mut rng),
                    1f32.to_bits(),
                ]
                .map(u32::to_le_bytes)
                .concat(),
            );
            for vv in [v, 0] {
                t.check(
                    &format!("Ch_ElemGetPos {chr:#x} {e} {vv:#x}"),
                    0x0083_3d10,
                    &[chr, e, vv, out],
                    |m| void!(mx::ch_elem_get_pos(m, chr, e, vv, out)),
                );
            }
        }
    }
    let clut = random_block(t, &mut rng, 0x10);
    let img = random_block(t, &mut rng, 0x10);
    let tex = random_block(t, &mut rng, 0x40);
    for i in 0..300 {
        t.put_u32(clut + 8, rng.next());
        t.check(
            &format!("Ch_TextureSetImage {i}"),
            0x0083_d220,
            &[clut, tex],
            |m| void!(ta::ch_texture_set_image(m, clut, tex)),
        );
        let sizes = [0u16, 1, 2, 3, 64, 100, 256, 0x4000, 0x8000];
        let (w, h) = (
            sizes[rng.next() as usize % 9],
            sizes[rng.next() as usize % 9],
        );
        t.put(img + 4, &[w.to_le_bytes(), h.to_le_bytes()].concat());
        t.put_u32(img, rng.next());
        t.put_u32(tex + 0x14, rng.next());
        t.put_u32(tex + 0x18, rng.next());
        t.put_u32(tex + 0x1c, rng.next());
        t.check(
            &format!("Ch_TextureSetRegs {i} {w} {h}"),
            0x0083_d250,
            &[img, tex],
            |m| void!(ta::ch_texture_set_regs(m, img, tex)),
        );
    }
}

#[test]
fn wind_vectors_and_remapping_match_the_original() {
    for mut t in twins() {
        wind_vectors_and_remapping_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn wind_vectors_and_remapping_match_the_original_on(t: &mut Twin) {
    use ffx_chara::attach::bridge as at;
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x5eed_0407);
    let chrs = live_characters(t, &mut rng);
    let out = t.alloc(64);
    let off = t.alloc(16);
    for &chr in &chrs {
        let w = [
            any_float(&mut rng),
            any_float(&mut rng),
            small_float(&mut rng),
        ];
        t.check(
            &format!("Ch_WindSet {chr:#x}"),
            0x0082_4c90,
            &[chr, w[0], w[1], w[2]],
            |m| void!(c::ch_wind_set(m, chr, fl(w[0]), fl(w[1]), fl(w[2]))),
        );
        let b: u32 = rd(&t.b, chr + l::BLK);
        let list: u32 = rd(&t.b, b + at::BLK_ATTRS);
        let mut ids: Vec<u32> = (0..4).collect();
        if list != 0 {
            let n: u32 = rd(&t.b, list + 0x24);
            let e: u32 = rd(&t.b, list + 0x20);
            ids.extend((0..n.min(16)).map(|i| u32::from(rd::<u16>(&t.b, e + 16 * i)) & 0x3fff));
        }
        for id in ids {
            for o in [off, 0] {
                t.check(
                    &format!("Ch_OptposGetMatrix {chr:#x} {id} {o:#x}"),
                    0x0083_56b0,
                    &[chr, id, o, out],
                    |m| void!(at::ch_optpos_get_matrix(m, chr, id, o, out)),
                );
            }
        }
    }
    // texture re-mapping: made-up texture data into a made-up buffer
    for (k, &chr) in chrs.iter().enumerate() {
        let b: u32 = rd(&t.b, chr + l::BLK);
        let tex = random_block(t, &mut rng, 0x40);
        let (nl, nc, ni) = (rng.next() % 4, rng.next() % 4, rng.next() % 4);
        t.put(tex + 4, &u16::try_from(nl).unwrap().to_le_bytes());
        t.put(tex + 8, &u16::try_from(nc).unwrap().to_le_bytes());
        t.put(tex + 0xc, &u16::try_from(ni).unwrap().to_le_bytes());
        let version: u16 = [0x1028, 0x1029, 0x1125, 0x1126, 0x2000][k % 5];
        t.put(tex + 0x10, &version.to_le_bytes());
        let lst = random_block(t, &mut rng, 16 * 4);
        for i in 0..4u32 {
            let size = [1u16, 2, 64, 100, 256][rng.next() as usize % 5];
            t.put(
                lst + 16 * i + 4,
                &[size.to_le_bytes(), size.to_le_bytes()].concat(),
            );
        }
        let cl = random_block(t, &mut rng, 8 * 4);
        let im = random_block(t, &mut rng, 16 * 4);
        t.put_u32(tex + 0x14, lst);
        t.put_u32(tex + 0x18, cl);
        t.put_u32(tex + 0x1c, im);
        t.put_u32(b + l::BLK_TEX, tex);
        let buf = random_block(t, &mut rng, 0x20 * 4 + 0x40 * 8);
        t.put_u32(chr + 0x6dc, if k % 6 == 5 { 0 } else { buf });
        let cluts = random_block(t, &mut rng, 0x400);
        t.put_u32(chr + l::CLUTS, cluts);
        t.put(chr + l::CLUT_ENABLED, &[u8::from(k % 2 == 0)]);
        t.check(
            &format!("Ch_TextureReMapping {chr:#x} {version:#x}"),
            0x0083_cdd0,
            &[chr],
            |m| void!(ta::ch_texture_re_mapping(m, chr)),
        );
    }
}

#[test]
fn history_and_block_moves_match_the_original() {
    for mut t in twins() {
        history_and_block_moves_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn history_and_block_moves_match_the_original_on(t: &mut Twin) {
    use ffx_chara::mapping::bridge as mb;
    let mut rng = Xorshift(0x5eed_0408);
    // a model block's archive moves: every active character on the block
    // is mapped again
    let blk_base = t.a.rebase(h::CHR_BLK.sym);
    for round in 0..6u32 {
        let chr = make_character(t, &mut rng, None);
        let slot = blk_base + (round * 7 % 40) * l::BLK_SIZE;
        let mut b = Blob::new(&mut rng, 0x1000);
        b.archive(&mut rng);
        let old = b.place(t);
        // map it once where it is, then copy it elsewhere
        let blk_bytes = random(&mut rng, l::BLK_SIZE);
        t.put(slot, &blk_bytes);
        t.put_u32(slot, if round == 5 { u32::MAX } else { 0x1000 | round });
        t.check(
            &format!("Ch_CbArcMapping first {round}"),
            0x0082_5ee0,
            &[slot, old],
            |m| void!(mb::ch_cb_arc_mapping(m, slot, old)),
        );
        let bytes = Oracle::read(&mut t.b, old, 0x1000).unwrap();
        let new = t.alloc(0x1000);
        t.put(new, &bytes);
        // the character on it, with no texture animation
        t.put_u32(chr + l::BLK, slot);
        t.put(chr + l::ACTIVE, &[1]);
        t.put_u32(chr + 0x6fc, 0);
        t.put_u32(chr + 0x6dc, 0);
        t.put_u32(chr + 0x758, if round % 2 == 0 { 0 } else { rng.next() });
        // in the array, as the first slot
        let array = h::CHR_ARRAY.get(&t.b);
        let saved = Oracle::read(&mut t.b, array, l::SIZE as usize).unwrap();
        let mine = Oracle::read(&mut t.b, chr, l::SIZE as usize).unwrap();
        t.put(array, &mine);
        t.check(
            &format!("Ch_MoveChrBlk {round}"),
            0x0083_47a0,
            &[new, old],
            |m| void!(mb::ch_move_chr_blk(m, new, old)),
        );
        t.put(array, &saved);
    }
}

#[test]
fn sequences_and_texture_bases_match_the_original() {
    for mut t in twins() {
        sequences_and_texture_bases_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn sequences_and_texture_bases_match_the_original_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mb;
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x5eed_0409);
    let chrs = live_characters(t, &mut rng);
    // the snapshot's own sequences first
    for &chr in &chrs {
        if t.name == "snapshot" && chr != *chrs.last().unwrap() {
            t.check(
                &format!("Ch_GetSeqFrame {chr:#x} own"),
                0x0083_6af0,
                &[chr],
                |m| int(mb::ch_get_seq_frame(m, chr)),
            );
            t.check(
                &format!("Ch_MotionIsEnd {chr:#x} own"),
                0x0083_71b0,
                &[chr],
                |m| Res::Int(mb::ch_motion_is_end(m, chr)),
            );
        }
    }
    // made-up sequences: a motion group, segments with key tables, a
    // jump list
    let chr = make_character(t, &mut rng, None);
    let keys = t.alloc(2 * 16);
    for i in 0..16u32 {
        t.put(
            keys + 2 * i,
            &u16::try_from(rng.next() % 0x200).unwrap().to_le_bytes(),
        );
    }
    let segs = t.alloc(16 * 8);
    for i in 0..8u32 {
        t.put_u32(segs + 16 * i + 8, keys);
    }
    let group = t.alloc(0x20);
    t.put_u32(group + 0x10, segs);
    t.put_u32(chr + 0x710, group);
    let seq = t.alloc(0x100);
    let jumps = t.alloc(8);
    // forward only: to the end, or to one more step before it
    t.put(jumps, &[0x30u8, 0, 0x34, 0, 0x30, 0, 0x34, 0]);
    let list = t.alloc(0x10);
    t.put_u32(list + 8, jumps);
    t.put_u32(list + 0xc, seq);
    t.put_u32(chr + 0x70c, list);
    for round in 0..400 {
        let mut bytes = vec![0u8; 0x40];
        let mut at = 0usize;
        while at < 0x30 {
            let code = [1u8, 1, 1, 2, 3, 6, 4, 0, 5, 7, 0xff][rng.next() as usize % 11];
            bytes[at] = code;
            let small = |rng: &mut Xorshift, n: u32| u8::try_from(rng.next() % n).unwrap();
            match code {
                1 => {
                    bytes[at + 1] = small(&mut rng, 8);
                    bytes[at + 3] = [0u8, 1, 2, 0x80, 0xff][rng.next() as usize % 5];
                    bytes[at + 4] = small(&mut rng, 2) * 0xff;
                    bytes[at + 5] = small(&mut rng, 16);
                    bytes[at + 7] = small(&mut rng, 16);
                }
                3 => {
                    bytes[at + 1] = small(&mut rng, 256);
                    bytes[at + 2] = small(&mut rng, 256);
                }
                4 => bytes[at + 1] = small(&mut rng, 4),
                _ => {}
            }
            at += [1usize, 9, 1, 3, 3, 1, 3, 1][usize::from(code.min(7))];
        }
        bytes[0x30..].fill(0);
        bytes[0x34] = 3;
        bytes[0x35] = 0x7f;
        t.put(seq, &bytes);
        t.put(seq + 0x40, &[0u8; 0xc0]);
        t.put_u32(chr + 0x714, if round % 50 == 0 { 0 } else { seq });
        t.put(chr + 0x73e, &[u8::try_from(round % 2).unwrap()]);
        t.put(
            chr + l::MOTION_LOOPS_LEFT,
            &u16::try_from(round % 3).unwrap().to_le_bytes(),
        );
        t.check(
            &format!("Ch_GetSeqFrame {round} {:x?}", &bytes[..16]),
            0x0083_6af0,
            &[chr],
            |m| int(mb::ch_get_seq_frame(m, chr)),
        );
        t.check(
            &format!("Ch_MotionIsEnd {round}"),
            0x0083_71b0,
            &[chr],
            |m| Res::Int(mb::ch_motion_is_end(m, chr)),
        );
    }
    // texture bases
    let tex = random_block(t, &mut rng, 0x40 * 6);
    // data pointers (the DMA reads the size before them), some null
    let data = random_block(t, &mut rng, 0x20) + 0x10;
    for k in 0..6u32 {
        t.put_u32(tex + 0x40 * k + 0x30, if k % 3 == 0 { 0 } else { data });
    }
    for round in 0..200 {
        for k in 0..6u32 {
            let b = [0xffffu16, 0, 0x100, 0x2ea0][rng.next() as usize % 4];
            t.put(tex + 0x40 * k + 0x38, &b.to_le_bytes());
        }
        let n = [0i32, 1, 3, 6, -1][rng.next() as usize % 5];
        let base = [u32::MAX, 0, 0x1234, rng.next()][rng.next() as usize % 4];
        let i = [u32::MAX, 0, 2, 5][rng.next() as usize % 4];
        let nu = n.cast_unsigned();
        t.check(
            &format!("Ch_TextureSetBase {round} {n} {base:#x} {i:#x}"),
            0x0083_2050,
            &[tex, nu, base, i],
            |m| Res::Int(ta::ch_texture_set_base(m, tex, n, base, i)),
        );
        t.check(
            &format!("ch_TextureLoad {round} {n}"),
            0x0083_1f00,
            &[tex, nu],
            |m| void!(ta::ch_texture_load(m, tex, n)),
        );
    }
    for &chr in &chrs {
        for i in [u32::MAX, 0, 1] {
            let base = rng.next() & 0x3fff;
            t.check(
                &format!("Ch_TextureSetImageBase {chr:#x} {i:#x}"),
                0x0083_1390,
                &[chr, i, base],
                |m| Res::Int(ta::ch_texture_set_image_base(m, chr, i, base)),
            );
            t.check(
                &format!("Ch_TextureSetClutBase {chr:#x} {i:#x}"),
                0x0083_13c0,
                &[chr, i, base],
                |m| Res::Int(ta::ch_texture_set_clut_base(m, chr, i, base)),
            );
        }
    }
    // the specular textures: the game's data on the snapshot, made up on
    // the image
    if ta::SPECULAR_TEX.get(&t.b) == 0 {
        let mut b = Blob::new(&mut rng, 0x300);
        b.tex_data(&mut rng, 0);
        let at = b.place(t);
        let sizes = t.alloc(16 * 4);
        for k in 0..4u32 {
            t.put(sizes + 16 * k + 4, &[64u8, 0, 32, 0]);
        }
        // version, lists: only what the setup reads
        t.put(at + 0x10, &0x1029u16.to_le_bytes());
        set_global(t, ta::SPECULAR_TEX.sym, at);
        let lists = random_block(t, &mut rng, 0x40);
        t.put_u32(at, at);
        t.put_u32(at + 0x14, sizes);
        t.put_u32(at + 0x18, lists);
        t.put_u32(at + 0x1c, lists);
    }
    t.check("Ch_TextureSpecInitAll", 0x0083_d140, &[], |m| {
        void!(ta::ch_texture_spec_init_all(m))
    });
}

#[test]
fn debug_stubs_match_the_original() {
    for mut t in twins() {
        for (name, at) in [
            ("asmAddBlock1", 0x0077_f1d0u32),
            ("asmAddBlock2", 0x0077_f1e0),
            ("asmClearBlock", 0x0077_f1f0),
            ("asmClearBlock0", 0x0077_f200),
        ] {
            // nothing written; the result is the caller's eax
            t.check(name, at, &[], |_| Res::Void);
        }
    }
}

#[test]
fn shades_situations_and_names_match_the_original() {
    for mut t in twins() {
        shades_situations_and_names_match_the_original_on(&mut t);
    }
}

fn shades_situations_and_names_match_the_original_on(t: &mut Twin) {
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x5eed_040a);
    let chrs = characters(t, &mut rng);
    for on in [0u8, 1, 0x80] {
        let at = t.a.rebase(id::ID_SHADE_ON.sym);
        t.put(at, &[on]);
        for &chr in &chrs {
            for pid in [
                rd::<u32>(&t.b, chr + l::PID),
                rng.next(),
                0x8000_0000,
                0xfffe_0000,
            ] {
                t.put_u32(chr + l::PID, pid);
                t.check(
                    &format!("Ch_IdGetShade {chr:#x} {pid:#x} {on}"),
                    0x0083_d880,
                    &[chr],
                    |m| Res::Int(id::ch_id_get_shade(m, chr)),
                );
                for (k, f) in [0x0083_d8a0u32, 0x0083_d8c0, 0x0083_d8e0]
                    .into_iter()
                    .enumerate()
                {
                    let k = u32::try_from(k).unwrap();
                    t.check(
                        &format!("Ch_IdGetShade{k} {chr:#x} {pid:#x}"),
                        f,
                        &[chr],
                        |m| Res::Int(id::ch_id_get_shade_field(m, chr, k)),
                    );
                }
            }
        }
    }
    let out = t.alloc(0x10);
    t.check("Ch_PacketRefEffect", 0x0083_0df0, &[rng.next(), out], |m| {
        Res::Int(c::ch_packet_ref_effect(m, out))
    });
    let live = live_characters(t, &mut rng);
    let made_up = *live.last().unwrap();
    for chr in live {
        if t.name == "synthetic" || chr == made_up {
            let inst = if rng.next() & 1 == 0 {
                0
            } else {
                random_block(t, &mut rng, 0x20)
            };
            t.put_u32(chr + l::RENDER_INSTANCE, inst);
            let entry = rng.next() % 64;
            t.put_u32(chr + 0x700, entry);
        }
        for v in [0u32, 1, 0x1ff, rng.next()] {
            t.check(
                &format!("Ch_DrawSetSituation {chr:#x} {v:#x}"),
                0x0083_0160,
                &[chr, v],
                |m| void!(c::ch_draw_set_situation(m, chr, v)),
            );
            if rd::<u32>(&t.b, chr + 0x700) < 64 {
                for anim in [0u32, 1] {
                    t.put_u32(chr + 0x6fc, anim);
                    t.check(
                        &format!("Ch_TextureSetAnimTimer {chr:#x} {v:#x} {anim}"),
                        0x0083_d070,
                        &[chr, v],
                        |m| void!(ta::ch_texture_set_anim_timer(m, chr, v)),
                    );
                }
            }
        }
    }
    for id in (0..16u32)
        .map(|k| k << 12)
        .chain([0x8000_f123, rng.next(), u32::MAX])
    {
        t.check(
            &format!("ch_create_section {id:#x}"),
            0x0083_8150,
            &[id],
            |m| Res::Int(ta::ch_create_section(m, id)),
        );
    }
}

#[test]
fn effect_color_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x5eed_040b);
        let mut vals: Vec<u32> = FLOATS.to_vec();
        vals.extend((0..300).map(|_| small_float(&mut rng)));
        vals.extend((0..300).map(|_| any_float(&mut rng)));
        for &v in &vals {
            let rgb = [v, any_float(&mut rng), small_float(&mut rng)];
            t.check(
                &format!("Ch_EffectSetColor {rgb:x?}"),
                0x0084_2c40,
                &rgb,
                |m| void!(sy::ch_effect_set_color(m, rgb.map(fl))),
            );
            t.check(
                &format!("Ch_EffectSetAlpha {v:#x}"),
                0x0084_2c60,
                &[v],
                |m| void!(sy::ch_effect_set_alpha(m, fl(v))),
            );
        }
    }
}

/// The functions the oracle's CRT stubs unblocked: `sprintf`, `strstr`,
/// `strncpy`, `_isnan`/`_finite`.
#[test]
fn crt_users_match_the_original() {
    for mut t in twins() {
        crt_users_match_the_original_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn crt_users_match_the_original_on(t: &mut Twin) {
    use ffx_chara::mapping::bridge as mb;
    use ffx_chara::motion::bridge as mo;
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x5eed_0501);
    let chrs = live_characters(t, &mut rng);
    let made_up = *chrs.last().unwrap();
    // the function history: some characters without models, named
    let file = t.alloc(32);
    t.put(file, b"..\\proj\\src\\chr\\chr.c\0");
    for (k, &chr) in chrs.iter().enumerate() {
        if t.name == "synthetic" || chr == made_up {
            let name = t.alloc(16);
            t.put(
                name,
                if k % 3 == 0 {
                    &b"c001\0"[..]
                } else {
                    b"m123_long_name\0"
                },
            );
            t.put_u32(chr + l::NAME, name);
            let hdr: u32 = rd(&t.b, chr + l::MODEL_HEADER);
            t.put(hdr + 6, &u16::from(k % 2 == 1).to_le_bytes());
        }
    }
    for line in [0u32, 0xb52, 0xb85, u32::MAX, 0x8000_0000] {
        t.check(
            &format!("Ch_FunctionHistory {line:#x}"),
            0x0083_3770,
            &[file, line],
            |m| void!(h::ch_function_history(m, file, line)),
        );
    }
    // rotations: invalid floats become 0
    for &chr in &chrs {
        for &bits in &FLOATS {
            t.check(
                &format!("Ch_SetRot {chr:#x} {bits:#x}"),
                0x0082_b520,
                &[chr, bits],
                |m| void!(c::ch_set_rot(m, chr, fl(bits))),
            );
            t.check(
                &format!("Ch_SetDirF {chr:#x} {bits:#x}"),
                0x0082_b1b0,
                &[chr, bits],
                |m| void!(c::ch_set_dir_f(m, chr, fl(bits))),
            );
        }
    }
    // the radian check: a debug line for ten turns and more (finite only:
    // the wrap never ends on infinities)
    let mut angles: Vec<u32> = FLOATS
        .iter()
        .copied()
        .filter(|&b| fl(b).is_nan() || fl(b).abs() < 1.0e6)
        .collect();
    angles.extend([62.831_84f32, -62.831_84, 62.831_83, 100.5, -1234.567, 0.001].map(f32::to_bits));
    angles.extend((0..200).map(|_| small_float(&mut rng)));
    for a in angles {
        t.check(&format!("Ch_CheckRadian {a:#x}"), 0x0083_6790, &[a], |m| {
            Res::Float(c::ch_check_radian(m, fl(a)))
        });
    }
    // names
    for id in [
        0u32,
        0x1000,
        0x2abc,
        0x6fff,
        0x7123,
        0xf001,
        0x1_0005,
        rng.next(),
        u32::MAX,
    ] {
        t.check(
            &format!("ch_create_name {id:#x}"),
            0x0083_8100,
            &[id],
            |m| Res::Int(ta::ch_create_name(m, id)),
        );
    }
    // .chr paths
    let src = t.alloc(64);
    let dst = t.alloc(64);
    for path in [
        &b"ffx_ps2/ffx/master/jppc/chr/mdl/c001/c001.chr\0"[..],
        b"chr/c001/c001.chr\0",
        b"no extension\0",
        b"a.chr.chr\0",
        b"mdl/.chr\0",
    ] {
        t.put(src, path);
        t.put(dst, &random(&mut rng, 64));
        t.put(dst + 50, &[0]);
        t.check(
            &format!("Ch_AnalysisChrNum {}", String::from_utf8_lossy(path)),
            0x0082_9e60,
            &[src, dst],
            |m| Res::Int(mb::ch_analysis_chr_num(m, src, dst)),
        );
    }
    // near the camera: compare al only (the rest of eax is what the
    // original's last instructions left)
    let cam = if mo::CAMERA.get(&t.b) == 0 || t.name == "synthetic" {
        let cam = random_block(t, &mut rng, 0x700);
        set_global(t, mo::CAMERA.sym, cam);
        cam
    } else {
        mo::CAMERA.get(&t.b)
    };
    for (k, &chr) in chrs.iter().enumerate() {
        for round in 0..6u32 {
            if round > 0 {
                let p: [f32; 3] = rd(&t.b, cam + 0x65c);
                let d = [10.0f32, 25.0, 31.0, 49.0, 60.0][round as usize % 5];
                let at = [p[0] + d * 0.6, p[1], p[2] + d * 0.8];
                t.put(
                    chr + 0x694,
                    &at.map(f32::to_bits).map(u32::to_le_bytes).concat(),
                );
                t.put(cam + 0x64d, &[u8::from(round % 2 == 0)]);
                // -0.785398, the bound Ch_AnimCopy2 compares against (not pi/4)
                let ang =
                    [-1.0f32, f32::from_bits(0xbf49_0fd8), 0.0, 0.5][(round as usize + k) % 4];
                t.put_u32(cam + 0x6b0, ang.to_bits());
                t.put_u32(cam + 0x6d0, (-ang).to_bits());
            }
            let at = t.a.rebase(0x0083_9140);
            let ra = t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
            let rb = mo::ch_anim_copy2(&t.b, chr);
            assert_eq!(
                ra.eax & 0xff,
                u32::from(rb),
                "{}: Ch_AnimCopy2 {chr:#x} {round}",
                t.name
            );
            assert_eq!(t.a.diff(&mut t.b).unwrap(), [], "Ch_AnimCopy2 memory");
        }
    }
}

/// `Ch_MgrpMove(new, old)`: a motion group moved; its table entry, its
/// sequences and the characters playing it follow (with the main module's
/// `SG_RelocateGroup` port for the group itself). On the snapshot `old` is
/// sometimes one of the game's own groups, so its real sequences and
/// characters move too.
#[test]
fn mgrp_move_matches_the_original() {
    for mut t in twins() {
        mgrp_move_on(&mut t);
    }
}

fn mgrp_move_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mo;
    let mut rng = Xorshift(0x0006_6a70);
    let chrs = live_characters(t, &mut rng);
    let table = 0x0130_11e0u32;
    let seqs = 0x0130_0a08u32;
    for round in 0..12u32 {
        // the former address: made up, or a group the table holds
        let held: Vec<u32> = (0..50)
            .map(|i| rd::<u32>(&t.b, table + 12 * i))
            .filter(|&g| g != 0)
            .collect();
        let old = if round % 3 == 2 && !held.is_empty() {
            held[rng.next() as usize % held.len()]
        } else {
            0x0200_0000 + (rng.next() & 0xfff_fff0)
        };
        // the group at its new place, built for `old`
        let n = rng.next() % 3;
        let group = t.alloc(0x10);
        let entries = t.alloc(20 * 3);
        let delta = group.wrapping_sub(old);
        t.put_u32(group, if round % 5 == 4 { group } else { old });
        t.put_u32(group + 4, n);
        t.put_u32(group + 0xc, entries.wrapping_sub(delta));
        for k in 0..n {
            let e = entries + 20 * k;
            t.put(e + 8, &[0, 0, 0, 0]);
            let data = random_block(t, &mut rng, 0x10);
            t.put_u32(e + 0xc, data.wrapping_sub(delta));
        }
        if round % 3 != 2 && !rng.next().is_multiple_of(4) {
            let i = rng.next() % 50;
            t.put_u32(table + 12 * i, old);
        }
        // sequences of the group (some unused), others left alone
        for _ in 0..rng.next() % 4 {
            let e = seqs + 0x14 * (rng.next() % 100);
            let data = if rng.next().is_multiple_of(4) {
                0
            } else {
                rng.next() | 1
            };
            t.put_u32(e, data);
            t.put_u32(e + 0x10, old);
        }
        // characters playing it
        for &chr in &chrs {
            if rng.next().is_multiple_of(3) {
                t.put_u32(chr + 0x7e8, old);
            }
        }
        t.check(
            &format!("Ch_MgrpMove {round}"),
            0x0083_70c0,
            &[group, old],
            |m| {
                Res::Int(mo::ch_mgrp_move(
                    m,
                    group,
                    old,
                    ffx_sg::seq::bridge::relocate_group,
                ))
            },
        );
    }
}

/// Round 7: the reader entry points (through a recording stand-in), the
/// kind tables, the shadow getters, the clut load, the neck switch, the
/// system motion group check, the first motion of a sequence and the
/// character names and paths.
#[test]
fn round7_matches_the_original() {
    for mut t in twins() {
        round7_on(&mut t);
    }
}

/// Code that records its first `n` (0..=2) arguments at `slot` and
/// returns the word at `slot + 8`, at the same address in both oracles.
fn recorder(t: &mut Twin, slot: u32, n: u32) -> u32 {
    let mut c = Vec::new();
    for k in 0..n {
        #[allow(clippy::cast_possible_truncation)] // 4 or 8
        c.extend([0x8b, 0x44, 0x24, (4 + 4 * k) as u8, 0xa3]);
        c.extend((slot + 4 * k).to_le_bytes());
    }
    c.push(0xa1);
    c.extend((slot + 8).to_le_bytes());
    c.push(0xc3);
    let at = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), at);
    at
}

/// A random motion sequence at a new allocation: skips (2, 3, 6), jumps
/// (4) to one of `targets`, then an end, a motion (1) or an unknown code.
fn sequence(t: &mut Twin, rng: &mut Xorshift, jumps: usize) -> Vec<u8> {
    let mut s = Vec::new();
    for _ in 0..rng.next() % 4 {
        match rng.next() % 3 {
            0 => s.push(2),
            1 => s.extend([3, 0xaa, 0xbb]),
            _ => s.extend([6, 0x11, 0x22]),
        }
    }
    let _ = t;
    if jumps > 0 && rng.next().is_multiple_of(3) {
        #[allow(clippy::cast_possible_truncation)] // a small index
        let k = (rng.next() as usize % jumps) as u8;
        s.extend([4, k, 0]);
    } else {
        #[allow(clippy::cast_possible_truncation)] // random bytes
        let r = rng.next() as u8;
        match rng.next() % 6 {
            0 => s.push(0),
            1 => s.push(5),
            2 => s.push(7),
            3 => s.push(0x90),
            _ => s.extend([1, r, (rng.next() % 3) as u8]),
        }
    }
    s
}

#[allow(clippy::too_many_lines)]
fn round7_on(t: &mut Twin) {
    use ffx_chara::chr::bridge as c;
    use ffx_chara::motion::bridge as mo;
    use ffx_chara::sys::bridge as sy;
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x7007_0007);
    let slot = t.alloc(12);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    for (g, n) in [(0x0231_0c48u32, 0), (0x0231_0c50, 1), (0x0231_0cc0, 2)] {
        let rec = recorder(t, slot, n);
        let at = t.a.rebase(g);
        t.put_u32(at, rec);
    }
    for round in 0..6u32 {
        t.put_u32(slot + 8, rng.next());
        t.check(&format!("Ch_ReadSync {round}"), 0x0082_a5e0, &[], |m| {
            Res::Int(sy::ch_read_sync(m, &mut |m, f, a| call(m, f, a)))
        });
        let p = rng.next();
        t.check(
            &format!("Ch_RomReadEventMGRPsync {round}"),
            0x0082_a7f0,
            &[p],
            |m| {
                Res::Int(sy::ch_rom_read_event_mgrp_sync(m, p, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
    }
    // the kind tables: made up on the synthetic image, the game's on the
    // snapshot (with a new one sometimes)
    let tables = t.a.rebase(0x012f_fab8);
    for k in 0..8u32 {
        let have: u32 = rd(&t.b, tables + 4 * k);
        if t.name == "synthetic" || have == 0 || rng.next().is_multiple_of(4) {
            let n = rng.next() % 6;
            let tab = t.alloc(2 + 2 * n as usize);
            #[allow(clippy::cast_possible_truncation)]
            t.put(tab, &(n as i16).to_le_bytes());
            for i in 0..n {
                #[allow(clippy::cast_possible_truncation)]
                t.put(tab + 2 + 2 * i, &((rng.next() % 8) as i16).to_le_bytes());
            }
            t.put_u32(tables + 4 * k, tab);
        }
    }
    if t.name == "synthetic" {
        let args = t.a.rebase(0x00c4_9660);
        t.put(args, &random(&mut rng, 16));
        let stride = t.a.rebase(0x00c4_967f);
        #[allow(clippy::cast_possible_truncation)]
        t.put(stride, &[rng.next() as u8 | 1]);
    }
    for round in 0..40u32 {
        let id = (rng.next() % 12) << 12 | (rng.next() % 9);
        t.check(
            &format!("Chr_TableIndex {id:#x} {round}"),
            0x0082_a640,
            &[id],
            |m| Res::Int(sy::chr_table_index(m, id, &mut |m, f, a| call(m, f, a)).cast_unsigned()),
        );
    }
    // the shadow getters
    for round in 0..4u32 {
        #[allow(clippy::cast_possible_truncation)]
        let v = [rng.next() as u8, rng.next() as u8];
        let at = t.a.rebase(0x0130_08bd);
        t.put(at, &v);
        t.check(
            &format!("Ch_DrawGetShadowMode {round}"),
            0x0082_ec60,
            &[],
            |m| Res::Int(sy::get_u8(m, sy::SHADOW_MODE)),
        );
        t.check(
            &format!("Ch_DrawGetShadowSemi {round}"),
            0x0082_ec70,
            &[],
            |m| Res::Int(sy::get_u8(m, sy::MOVIE_GOMI_AVOID)),
        );
    }
    // the clut textures and the neck
    let chrs = live_characters(t, &mut rng);
    let made_up = *chrs.last().unwrap();
    for &chr in &chrs {
        if t.name == "synthetic" || chr == made_up {
            let n = rng.next() % 4;
            let tex = random_block(t, &mut rng, 0x40 * n.max(1));
            let data = random_block(t, &mut rng, 0x20) + 0x10;
            for k in 0..n {
                t.put_u32(tex + 0x40 * k + 0x30, if k % 2 == 0 { 0 } else { data });
                if rng.next().is_multiple_of(2) {
                    t.put(tex + 0x40 * k + 0x38, &[0xff, 0xff]);
                }
            }
            t.put_u32(chr + 0x6e4, tex);
            #[allow(clippy::cast_possible_truncation)]
            t.put(chr + 0x6e8, &(n as i16).to_le_bytes());
        }
        t.check(
            &format!("Ch_TextureLoadClut {chr:#x}"),
            0x0083_11d0,
            &[chr, 0],
            |m| {
                ta::ch_texture_load_clut(m, chr);
                Res::Void
            },
        );
        for on in [0u32, 1, 0x100, 0xffff_ff00, 7] {
            #[allow(clippy::cast_possible_truncation)]
            let (x, y) = (rng.next() as u8 % 3, rng.next() as u8 % 3);
            t.put(chr + 0x40d, &[x, y]);
            for k in 0..12 {
                t.put_u32(chr + 0x46c + 4 * k, small_float(&mut rng));
            }
            t.check(
                &format!("Ch_NeckSetEnable {chr:#x} {on:#x}"),
                0x0083_4f60,
                &[chr, on],
                |m| {
                    c::ch_neck_set_enable(m, chr, on, ffx_sg::quat::bridge::matrix_to_quaternion);
                    Res::Void
                },
            );
        }
    }
    // system motion groups: keys the table holds, and others
    let groups = t.a.rebase(0x0130_11e0);
    let keys: Vec<u32> = (0..50)
        .map(|i| rd::<u32>(&t.b, groups + 12 * i + 8))
        .collect();
    for round in 0..20u32 {
        let key = if round % 2 == 0 {
            keys[rng.next() as usize % keys.len()]
        } else {
            rng.next() % 4
        };
        let b = rng.next() % 3;
        t.check(
            &format!("ChN_ExistSystemMGRP {round}"),
            0x0083_6850,
            &[key, b],
            |m| {
                Res::Int(mo::chn_exist_system_mgrp(m, key, b, |m, x, y, k| {
                    ffx_sg::seq::bridge::find_motion_group(m, x, y, k)
                }))
            },
        );
    }
    // the first motion of a sequence
    for round in 0..30u32 {
        let chr = t.alloc(0x720);
        let n = 1 + rng.next() as usize % 3;
        // the jump targets, then the sequence itself
        let base = t.alloc(0x100);
        let mut offsets = Vec::new();
        let mut at = 0u32;
        for _ in 0..n {
            let s = sequence(t, &mut rng, 0);
            t.put(base + at, &s);
            offsets.push(at);
            at += u32::try_from(s.len()).unwrap();
        }
        let start = sequence(t, &mut rng, n);
        let seq = t.alloc(start.len());
        t.put(seq, &start);
        let sg = t.alloc(0x10);
        let tab = t.alloc(2 * n);
        for (k, o) in offsets.iter().enumerate() {
            t.put(
                tab + 2 * u32::try_from(k).unwrap(),
                &u16::try_from(*o).unwrap().to_le_bytes(),
            );
        }
        t.put_u32(sg + 8, tab);
        t.put_u32(sg + 0xc, base);
        let mg = t.alloc(0x14);
        t.put_u32(mg + 0x10, rng.next());
        t.put_u32(chr + 0x70c, sg);
        t.put_u32(chr + 0x710, mg);
        t.put_u32(chr + 0x714, if round % 10 == 9 { 0 } else { seq });
        t.check(
            &format!("Ch_SeqGetFastMotion {round}"),
            0x0083_7860,
            &[chr],
            |m| Res::Int(mo::ch_seq_get_fast_motion(m, chr)),
        );
    }
    // names and paths
    for round in 0..30u32 {
        let id = ((rng.next() % 16) << 12) | (rng.next() % 0x1000);
        t.check(
            &format!("ch_create_chrname {id:#x} {round}"),
            0x0083_8060,
            &[id],
            |m| Res::Int(ta::ch_create_chrname(m, id)),
        );
        t.check(
            &format!("ch_create_chrpath {id:#x} {round}"),
            0x0083_80b0,
            &[id],
            |m| Res::Int(ta::ch_create_chrpath(m, id)),
        );
    }
}

/// Round 7, second part: the x turn and the effect packet start.
#[test]
fn round7b_matches_the_original() {
    for mut t in twins() {
        round7b_on(&mut t);
    }
}

fn round7b_on(t: &mut Twin) {
    use ffx_chara::chr::bridge as c;
    use ffx_chara::sys::bridge as sy;
    let mut rng = Xorshift(0x7b7b_0007);
    let chrs = live_characters(t, &mut rng);
    for (k, &chr) in chrs.iter().enumerate() {
        for round in 0..8u32 {
            let a = small_float(&mut rng);
            let speed = match round % 4 {
                0 => 0,
                1 => 0x8000_0000,
                2 => any_float(&mut rng),
                _ => small_float(&mut rng),
            };
            // no infinities: the wrap would never end
            let speed = if fl(speed).is_infinite() {
                0x3f80_0000
            } else {
                speed
            };
            t.check(
                &format!("Ch_SetRotX {k} {round}"),
                0x0083_57f0,
                &[chr, a, speed],
                |m| {
                    Res::Int(c::ch_set_rot_x(
                        m,
                        chr,
                        fl(a),
                        fl(speed),
                        ffx_sg::vec::bridge::angle_wrap,
                    ))
                },
            );
        }
    }
    let at = |t: &Twin, g: u32| t.a.rebase(g);
    for round in 0..8u32 {
        // measured values to copy, and the buffers whose offsets it takes
        let polys = at(t, 0x0130_3660);
        t.put_u32(polys, rng.next() % 0x2_0000);
        t.put_u32(polys + 4, rng.next());
        t.put_u32(polys + 8, rng.next());
        let p = if round % 3 == 2 { 0 } else { t.alloc(0x40) };
        t.check(
            &format!("Ch_EffectPacketSetPtr {round}"),
            0x0084_5710,
            &[p],
            |m| {
                Res::Int(sy::ch_effect_packet_set_ptr(
                    m,
                    p,
                    |m| ffx_sg::state::bridge::get_vtex(m),
                    |m| ffx_sg::state::bridge::get_norm(m),
                ))
            },
        );
    }
}

/// Stand-ins for the callees of round 8's functions: the code at `f`
/// (in both oracles) logs `id` and its first `n` arguments at the log
/// pointer `log` (advanced past them) and returns the word at `ret`.
struct Fakes {
    log: u32,
    start: u32,
}

impl Fakes {
    fn new(t: &mut Twin) -> Self {
        let log = t.alloc(8);
        let start = t.alloc(0x1000);
        t.put_u32(log, start);
        Self { log, start }
    }

    /// Replaces the function at `sym` (an ffx.sym address) with a jump
    /// to a stand-in (5 bytes, so short neighbours stay intact); returns
    /// the address of its result word.
    fn fake(&self, t: &mut Twin, sym: u32, id: u32, n: u8) -> u32 {
        let (code, ret) = self.fake_code(t, id, n);
        let at = t.a.rebase(sym);
        let mut jmp = vec![0xe9];
        jmp.extend(code.wrapping_sub(at + 5).to_le_bytes());
        t.put(at, &jmp);
        ret
    }

    /// A stand-in at new code (for function pointers): its address and
    /// result word.
    fn fake_code(&self, t: &mut Twin, id: u32, n: u8) -> (u32, u32) {
        self.fake_code_step(t, id, n, 0)
    }

    /// [`Fakes::fake`] for an allocator: the result word grows by `step`
    /// after each call, so every call gets its own block.
    fn fake_alloc(&self, t: &mut Twin, sym: u32, id: u32, step: u32) -> u32 {
        let (code, ret) = self.fake_code_step(t, id, 1, step);
        let at = t.a.rebase(sym);
        let mut jmp = vec![0xe9];
        jmp.extend(code.wrapping_sub(at + 5).to_le_bytes());
        t.put(at, &jmp);
        ret
    }

    fn fake_code_step(&self, t: &mut Twin, id: u32, n: u8, step: u32) -> (u32, u32) {
        let ret = t.alloc(4);
        let mut c = vec![0x8b, 0x0d];
        c.extend(self.log.to_le_bytes());
        c.extend([0xc7, 0x01]);
        c.extend(id.to_le_bytes());
        for k in 0..n {
            c.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 + 4 * k]);
        }
        c.extend([0x83, 0x05]);
        c.extend(self.log.to_le_bytes());
        c.push(4 * (n + 1));
        c.push(0xa1);
        c.extend(ret.to_le_bytes());
        if step != 0 {
            c.extend([0x81, 0x05]);
            c.extend(ret.to_le_bytes());
            c.extend(step.to_le_bytes());
        }
        c.push(0xc3);
        let at = t.a.alloc_code(&c).unwrap();
        assert_eq!(t.b.alloc_code(&c).unwrap(), at);
        (at, ret)
    }

    /// Empties the log.
    fn reset(&self, t: &mut Twin) {
        t.put_u32(self.log, self.start);
    }
}

#[test]
fn round8_matches_the_original() {
    for mut t in twins() {
        round8_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::many_single_char_names)] // t, c, m, f, a, x, y, v, p
fn round8_on(t: &mut Twin) {
    use ffx_chara::chr::bridge as c;
    use ffx_chara::motion::bridge as mo;
    use ffx_chara::sys::bridge as sy;
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x0008_8888);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let scene = fk.fake(t, 0x0078_2970, 1, 0);
    let hash = fk.fake(t, 0x0064_1da0, 2, 0);
    let hide = fk.fake(t, 0x0082_ba30, 3, 3);
    let visible = fk.fake(t, 0x0063_c390, 4, 2);
    let coeff = fk.fake(t, 0x0063_cdd0, 5, 2);
    let color = fk.fake(t, 0x0063_d050, 6, 4);
    let fade = fk.fake(t, 0x0082_b990, 7, 3);
    let check_yet = fk.fake(t, 0x0083_ffe0, 8, 2);
    let to_top = fk.fake(t, 0x0084_0cf0, 9, 2);
    let used = fk.fake(t, 0x0084_0dd0, 10, 2);
    let exist3 = fk.fake(t, 0x0084_0790, 11, 2);
    let exist_cloth = fk.fake(t, 0x0084_0850, 12, 2);
    let assert = fk.fake(t, 0x0090_ae50, 13, 4);
    let tex_anim = fk.fake(t, 0x0077_e940, 14, 5);
    let gui = fk.fake(t, 0x0084_c9d0, 15, 0);
    let rets = [
        scene,
        hash,
        hide,
        visible,
        coeff,
        color,
        fade,
        check_yet,
        to_top,
        used,
        exist3,
        exist_cloth,
        assert,
        tex_anim,
        gui,
    ];
    // compared unrelocated (the image has no relocation for it)
    let the_scene = 0x0237_0000;
    let chrs = live_characters(t, &mut rng);
    for round in 0..24u32 {
        fk.reset(t);
        for r in rets {
            t.put_u32(r, rng.next() % 4);
        }
        let chr = chrs[rng.next() as usize % chrs.len()];
        // Ch_SetHideEvent: the special scene, level and character
        t.put_u32(
            scene,
            if round % 2 == 0 {
                the_scene
            } else {
                rng.next()
            },
        );
        t.put_u32(
            hash,
            if round % 3 == 0 {
                0x21e8
            } else {
                rng.next() % 3
            },
        );
        if round % 4 < 2 {
            t.put(chr, &0x108au16.to_le_bytes());
        }
        let flags = rng.next();
        t.check(
            &format!("Ch_SetHideEvent {round}"),
            0x0082_b2c0,
            &[chr, flags],
            |m| {
                Res::Int(c::ch_set_hide_event(m, chr, flags, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        let v = [0, 1, 0x100, rng.next()][rng.next() as usize % 4];
        t.check(
            &format!("Ch_SetResHide {round}"),
            0x0082_b4e0,
            &[chr, v],
            |m| Res::Int(c::ch_set_res_hide(m, chr, v, &mut |m, f, a| call(m, f, a))),
        );
        t.put_u32(chr + 0x174, any_float(&mut rng));
        let z = any_float(&mut rng);
        t.check(
            &format!("Ch_SetClipZ {round}"),
            0x0082_b150,
            &[chr, z],
            |m| {
                Res::Float(c::ch_set_clip_z(m, chr, fl(z), &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        let p = t.alloc(4);
        t.put_u32(p, any_float(&mut rng));
        t.check(
            &format!("Ch_CalcLight {round}"),
            0x0083_c6f0,
            &[chr, p],
            |m| Res::Int(c::ch_calc_light(m, chr, p, &mut |m, f, a| call(m, f, a))),
        );
        for k in 0..3 {
            t.put_u32(chr + 0xbc + 4 * k, small_float(&mut rng));
        }
        // around the 1.3 limit, sometimes anything
        #[allow(clippy::cast_precision_loss)]
        let col = [0; 3].map(|_| {
            if rng.next().is_multiple_of(3) {
                any_float(&mut rng)
            } else {
                ((rng.next() % 3000) as f32 / 1000.0).to_bits()
            }
        });
        let frames = rng.next() % 60;
        t.check(
            &format!("Ch_SetShadeCol {round}"),
            0x0082_b5d0,
            &[chr, col[0], col[1], col[2], frames],
            |m| {
                Res::Int(c::ch_set_shade_col(
                    m,
                    chr,
                    col.map(fl),
                    frames,
                    &mut |m, f, a| call(m, f, a),
                ))
            },
        );
        // the cache
        let id = rng.next();
        t.check(
            &format!("Ch_DataReadSync {round}"),
            0x0082_a040,
            &[id],
            |m| Res::Int(mo::ch_data_read_sync(m, id, &mut |m, f, a| call(m, f, a))),
        );
        let (x, y) = (rng.next() % 0x2_0000, rng.next() % 0x2_0000);
        t.check(
            &format!("ChN_ReadSystemMGRPsync {round}"),
            0x0083_6a50,
            &[x, y],
            |m| {
                Res::Int(mo::chn_read_system_mgrp_sync(m, x, y, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        t.check(
            &format!("Ch_RegMotionToTop {round}"),
            0x0083_7780,
            &[],
            |m| {
                Res::Int(mo::ch_reg_motion(m, mo::CACHE_TO_TOP, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        t.check(&format!("Ch_RegMotionUse {round}"), 0x0083_7790, &[], |m| {
            Res::Int(mo::ch_reg_motion(m, mo::CACHE_USE, &mut |m, f, a| {
                call(m, f, a)
            }))
        });
        t.check(
            &format!("Ch_RegMotionGetExistMem {round}"),
            0x0083_7730,
            &[],
            |m| {
                Res::Int(mo::ch_reg_motion_get_exist_mem(m, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        let cloth = if round % 3 == 0 { 0 } else { t.alloc(0x10) };
        t.put_u32(chr + 0x834, cloth);
        t.put_u32(chr + 0x194, rng.next());
        #[allow(clippy::cast_possible_truncation)]
        t.put(chr + 0x180, &[rng.next() as u8]);
        t.check(
            &format!("Ch_UpdateClothHideFlag {round}"),
            0x0083_1480,
            &[chr],
            |m| {
                Res::Int(mo::ch_update_cloth_hide_flag(m, chr, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        t.put_u32(chr + 0x6fc, rng.next() % 2);
        let (a, b) = (rng.next(), rng.next());
        t.check(
            &format!("Ch_TextureSetAnim {round}"),
            0x0083_d040,
            &[chr, a, b],
            |m| {
                Res::Int(ta::ch_texture_set_anim(m, chr, a, b, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        let dbg = t.a.rebase(0x0133_c910);
        #[allow(clippy::cast_possible_truncation)]
        t.put(dbg, &[(rng.next() % 2) as u8]);
        let v = [0, 1, 2, 3, 0xffff_ffff, rng.next()][rng.next() as usize % 6];
        t.check(
            &format!("Ch_EnableSetTexture {round}"),
            0x0083_0330,
            &[v],
            |m| {
                Res::Int(
                    sy::ch_enable_set_texture(m, v, &mut |m, f, a| call(m, f, a)).cast_unsigned(),
                )
            },
        );
        // the neck
        let q = ffx_sg::quat::bridge::matrix_to_quaternion;
        #[allow(clippy::cast_possible_truncation)]
        t.put(
            chr + 0x40d,
            &[(rng.next() % 5) as u8, (rng.next() % 5) as u8],
        );
        for k in 0..12 {
            t.put_u32(chr + 0x46c + 4 * k, small_float(&mut rng));
        }
        match round % 4 {
            0 => t.check(
                &format!("Ch_NeckSetTargetCamera {round}"),
                0x0083_5080,
                &[chr],
                |m| {
                    c::ch_neck_set_target_camera(m, chr, q);
                    Res::Void
                },
            ),
            1 => {
                let target = rng.next();
                t.check(
                    &format!("Ch_NeckSetTarget {round}"),
                    0x0083_5050,
                    &[chr, target],
                    |m| {
                        c::ch_neck_set_target(m, chr, target, q);
                        Res::Void
                    },
                );
            }
            2 => {
                let pos = random_block(t, &mut rng, 16);
                t.check(
                    &format!("Ch_NeckSetTargetPos {round}"),
                    0x0083_50a0,
                    &[chr, pos],
                    |m| {
                        c::ch_neck_set_target_pos(m, chr, pos, q);
                        Res::Void
                    },
                );
            }
            _ => {
                let (x, y) = (any_float(&mut rng), any_float(&mut rng));
                t.check(
                    &format!("Ch_NeckSetRot {round}"),
                    0x0083_4ff0,
                    &[chr, x, y],
                    |m| {
                        c::ch_neck_set_rot(m, chr, fl(x), fl(y), q);
                        Res::Void
                    },
                );
            }
        }
    }
}

#[test]
fn round8b_matches_the_original() {
    for mut t in twins() {
        round8b_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round8b_on(t: &mut Twin) {
    use ffx_chara::handle::bridge as h;
    use ffx_chara::motion::bridge as mo;
    use ffx_chara::sys::bridge as sy;
    let mut rng = Xorshift(0x0008_b8b8);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let malloc = fk.fake(t, 0x0068_7020, 1, 1);
    let set_str = fk.fake(t, 0x0083_c580, 2, 3);
    let clear = fk.fake(t, 0x0083_3760, 3, 0);
    let memclear = fk.fake(t, 0x0062_f4e0, 4, 2);
    let mapping = fk.fake(t, 0x0082_7590, 5, 1);
    let work = fk.fake(t, 0x0082_0580, 6, 1);
    let reserve = fk.fake(t, 0x0083_5730, 7, 1);
    let exist = fk.fake(t, 0x0084_0850, 8, 2);
    let ctrl = fk.fake(t, 0x0086_aff0, 9, 0);
    let insert = fk.fake(t, 0x0063_9ff0, 10, 1);
    let mut cd = Vec::new();
    for (k, (g, n)) in [(0x0231_0c40u32, 4u8), (0x0231_0c50, 1), (0x0231_0c54, 1)]
        .into_iter()
        .enumerate()
    {
        let (code, ret) = fk.fake_code(t, 11 + u32::try_from(k).unwrap(), n);
        let at = t.a.rebase(g);
        t.put_u32(at, code);
        cd.push(ret);
    }
    let rets = [
        malloc, set_str, clear, memclear, mapping, work, reserve, exist, ctrl, insert,
    ];
    let chrs = live_characters(t, &mut rng);
    for round in 0..12u32 {
        fk.reset(t);
        for r in rets.iter().chain(&cd) {
            t.put_u32(*r, rng.next());
        }
        let p = t.alloc(0x40);
        t.put_u32(malloc, p);
        // Ch_SysDrawSetSituation
        let det = t.a.rebase(0x012f_b7d8);
        t.put_u32(det, rng.next() % 2);
        let v = rng.next();
        t.check(
            &format!("Ch_SysDrawSetSituation {round}"),
            0x0083_0cd0,
            &[v],
            |m| {
                Res::Int(sy::ch_sys_draw_set_situation(m, v, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        // Ch_AnimCopyNear
        let chr = chrs[rng.next() as usize % chrs.len()];
        let w = t.alloc(4);
        #[allow(clippy::cast_possible_truncation)]
        t.put(w, &[rng.next() as u8 & 3]);
        t.put_u32(ctrl, w);
        let bs = t.a.rebase(0x0112_a8e0);
        t.put(bs, &[u8::from(round % 5 == 4)]);
        for k in 0..3 {
            let a = small_float(&mut rng);
            let d = [0.0f32, 1.9, 2.0, -1.99, -2.5, f32::NAN][rng.next() as usize % 6];
            t.put_u32(chr + 0x684 + 4 * k, a);
            t.put_u32(chr + 0x694 + 4 * k, (fl(a) + d).to_bits());
        }
        let at = t.a.rebase(0x0083_9200);
        let ra = t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
        let rb = mo::ch_anim_copy_near(&mut t.b, chr, &mut |m, f, a| call(m, f, a));
        assert_eq!(
            ra.eax & 0xff,
            u32::from(rb),
            "{}: Ch_AnimCopyNear {round}",
            t.name
        );
        assert!(ra.st0.is_none());
        assert_eq!(t.a.diff(&mut t.b).unwrap(), [], "Ch_AnimCopyNear memory");
        // Ch_DrawClothClear: a made-up cloth and buffer
        let n = rng.next() % 3;
        let cloth = t.alloc(0x40);
        t.put_u32(cloth + 0x14, n);
        let buf = t.alloc(0x20);
        for i in 0..n {
            let m = rng.next() % 4;
            let list = if rng.next().is_multiple_of(4) {
                0
            } else {
                t.alloc(4 + 4 * m as usize)
            };
            let other = if rng.next().is_multiple_of(4) {
                0
            } else {
                t.alloc(4 + 4 * m as usize)
            };
            if list != 0 {
                t.put_u32(list, m);
                for j in 0..m {
                    let e = if rng.next().is_multiple_of(4) {
                        0
                    } else {
                        t.alloc(0x50)
                    };
                    if e != 0 {
                        t.put_u32(e + 0x48, rng.next());
                        t.put_u32(e + 0x4c, rng.next() % 2);
                    }
                    t.put_u32(list + 4 + 4 * j, e);
                }
            }
            if other != 0 {
                for j in 0..m {
                    t.put_u32(other + 4 + 4 * j, rng.next() % 2);
                }
            }
            t.put_u32(cloth + 0x30 + 4 * i, list);
            t.put_u32(buf + 4 + 4 * i, other);
        }
        t.put_u32(chr + 0x834, if round % 4 == 3 { 0 } else { cloth });
        t.put_u32(chr + 0x194, if round % 3 == 2 { 0 } else { 0x20_0000 });
        t.put_u32(exist, if round % 5 == 1 { 0 } else { buf });
        t.put_u32(det, u32::from(round % 6 == 5));
        t.check(
            &format!("Ch_DrawClothClear {round}"),
            0x0083_0b90,
            &[chr],
            |m| {
                Res::Int(mo::ch_draw_cloth_clear(m, chr, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        // Ch_DrawReserveCalcNorm: flags and the cursor
        for &c in &chrs {
            #[allow(clippy::cast_possible_truncation)]
            {
                t.put(c + 0x194, &[rng.next() as u8 & 6]);
                t.put(c + 0x198, &[rng.next() as u8 & 0x40]);
            }
        }
        t.put_u32(
            work,
            [0, 100, 461, 462, 300, 0xffff_ff00][rng.next() as usize % 6],
        );
        let cur = t.a.rebase(0x0130_09e8);
        // the cursor is an element of the array (else the original never ends)
        let arr: u32 = rd(&t.b, t.a.rebase(0x023c_44e4));
        let count: i32 = rd(&t.b, t.a.rebase(0x023c_44e0));
        let pick = if rng.next().is_multiple_of(3) || count <= 0 {
            0
        } else {
            arr + 0x880 * (rng.next() % count.cast_unsigned())
        };
        t.put_u32(cur, pick);
        t.check(
            &format!("Ch_DrawReserveCalcNorm {round}"),
            0x0083_36a0,
            &[],
            |m| {
                Res::Int(h::ch_draw_reserve_calc_norm(m, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        // Ch_BlkNew: free blocks or none
        let blk = t.a.rebase(0x023c_8d00);
        if t.name == "synthetic" || round % 4 == 0 {
            for i in 0..40 {
                let v = if rng.next().is_multiple_of(5) {
                    u32::MAX
                } else {
                    rng.next() | 1
                };
                t.put_u32(blk + 0x12c * i, v);
            }
        }
        let has_free = (0..40).any(|i| rd::<u32>(&t.b, blk + 0x12c * i) == u32::MAX);
        if has_free {
            let (hd, key, x) = (rng.next(), rng.next(), rng.next());
            t.check(
                &format!("Ch_BlkNew {round}"),
                0x0082_5860,
                &[hd, key, x],
                |m| Res::Int(h::ch_blk_new(m, hd, key, x, &mut |m, f, a| call(m, f, a))),
            );
        }
        // Ch_RomDataReadInit: the tables come from the fake malloc
        t.check(
            &format!("Ch_RomDataReadInit {round}"),
            0x0082_a5f0,
            &[],
            |m| Res::Int(sy::ch_rom_data_read_init(m, &mut |m, f, a| call(m, f, a))),
        );
    }
    // last: it replaces the character array
    for n in [0i32, 1, 3, -1] {
        fk.reset(t);
        let arr = t.alloc(0x880 * 4);
        t.put_u32(malloc, arr);
        t.check(
            &format!("Ch_Allocate_InitAll {n}"),
            0x0082_5670,
            &[n.cast_unsigned()],
            |m| Res::Int(h::ch_allocate_init_all(m, n, &mut |m, f, a| call(m, f, a))),
        );
    }
}

#[test]
fn round9_matches_the_original() {
    for mut t in twins() {
        round9_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round9_on(t: &mut Twin) {
    use ffx_chara::chr::bridge as c;
    use ffx_chara::sys::bridge as sy;
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x0009_9999);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let seg_hide = fk.fake(t, 0x0063_bdf0, 1, 3);
    let visible = fk.fake(t, 0x0063_c390, 2, 2);
    let size = fk.fake(t, 0x0083_c050, 3, 1);
    let free = fk.fake(t, 0x0068_7010, 4, 1);
    let trigger = fk.fake(t, 0x0082_7250, 5, 0);
    let check_yet = fk.fake(t, 0x0083_ffe0, 6, 2);
    let index = fk.fake(t, 0x0082_a640, 7, 1);
    let malloc = fk.fake(t, 0x0068_7020, 8, 1);
    let chrname = fk.fake(t, 0x0083_8060, 9, 1);
    let set_str = fk.fake(t, 0x0083_c580, 10, 1);
    let set_type = fk.fake(t, 0x0083_c590, 11, 2);
    let entry = fk.fake(t, 0x0084_0560, 12, 3);
    let not_yet = fk.fake(t, 0x0084_05f0, 13, 3);
    let image = fk.fake(t, 0x0083_0d90, 14, 2);
    let flush = fk.fake(t, 0x0062_f4c0, 15, 1);
    let dma = fk.fake(t, 0x0062_f710, 16, 1);
    let load = fk.fake(t, 0x0083_1f00, 17, 3);
    // the CD library entries: g_CdLib (the request pointer differs: the
    // original's is its stack), +0x14, +0x98
    let mut cd = Vec::new();
    for (g, n) in [(0x0231_0c40u32, 3u8), (0x0231_0c54, 1), (0x0231_0c98, 2)] {
        let (code, ret) = fk.fake_code(t, 20 + g, n);
        let at = t.a.rebase(g);
        t.put_u32(at, code);
        cd.push(ret);
    }
    // chr_texanim_draw: writes a word into the packet (its second
    // argument) and returns the packet plus the length at `draw_ret`
    let draw_ret = t.alloc(4);
    let mut code = vec![
        0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00, 0x11, 0x22, 0x33, 0x44, 0x03, 0x05,
    ];
    code.extend(draw_ret.to_le_bytes());
    code.push(0xc3);
    let at = t.a.rebase(0x0077_c830);
    t.put(at, &code);
    let rets = [
        seg_hide, visible, size, free, trigger, check_yet, index, malloc, chrname, set_str,
        set_type, entry, not_yet, image, flush, dma, load,
    ];
    let chrs = live_characters(t, &mut rng);
    let made_up = *chrs.last().unwrap();
    for round in 0..16u32 {
        fk.reset(t);
        for r in rets.iter().chain(&cd) {
            t.put_u32(*r, rng.next());
        }
        let chr = chrs[rng.next() as usize % chrs.len()];
        // Ch_ModelSetHide: a made-up model list on the made-up character
        if chr == made_up || t.name == "synthetic" {
            let n = 1 + rng.next() % 4;
            let hdr = t.alloc(8);
            #[allow(clippy::cast_possible_truncation)]
            t.put(hdr + 6, &(n as u16).to_le_bytes());
            let models = random_block(t, &mut rng, 0x38 * 4);
            t.put_u32(chr + 0x1c0, hdr);
            t.put_u32(chr + 0x1bc, models);
            let name = t.alloc(16);
            t.put(
                name,
                if round % 2 == 0 {
                    b"f117_00\0"
                } else {
                    b"c001\0"
                },
            );
            t.put_u32(chr + 4, name);
        }
        let q = t.a.rebase(0x012f_fad4);
        t.put_u32(q, [0, 1, 3, 4, 5, 0xffff_ffff][rng.next() as usize % 6]);
        let hdr: u32 = rd(&t.b, chr + 0x1c0);
        let n = u32::from(rd::<u16>(&t.b, hdr + 6));
        let i = [0, 1, n.saturating_sub(1), n, n + 3][rng.next() as usize % 5];
        let hide = [0, 1, 2, 3][rng.next() as usize % 4];
        t.check(
            &format!("Ch_ModelSetHide {round}"),
            0x0082_aea0,
            &[chr, i, hide],
            |m| {
                Res::Int(c::ch_model_set_hide(m, chr, i, hide, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        // Ch_ModelPurgePacket
        t.put_u32(chr + 0x198, rng.next() & 0x60);
        let all = [0i32, 1, -1][rng.next() as usize % 3];
        t.check(
            &format!("Ch_ModelPurgePacket {round}"),
            0x0082_7700,
            &[chr, all.cast_unsigned()],
            |m| {
                Res::Int(c::ch_model_purge_packet(m, chr, all, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        // Ch_MgrpRead: cache states, table indexes, sizes
        t.put_u32(
            check_yet,
            [0, 1, 2, 3, 0xffff_ffff][rng.next() as usize % 5],
        );
        t.put_u32(index, [0, 5, 0xffff_ffff][rng.next() as usize % 3]);
        t.put_u32(cd[1], [0, 0x10, 0x11, 0x2345][rng.next() as usize % 4]);
        let nm = t.alloc(16);
        t.put(nm, b"mon:m042\0");
        t.put_u32(chrname, nm);
        let (id, k) = (rng.next() % 0x6000, rng.next() % 3);
        let scratch = t.alloc(0x60);
        let at = t.a.rebase(0x0082_a4c0);
        let ra = t.a.call(at, Conv::Cdecl, &[id, k]).unwrap();
        let rb = sy::ch_mgrp_read(&mut t.b, id, k, scratch, &mut |m, f, a| call(m, f, a));
        assert_eq!(ra.eax, rb, "{}: Ch_MgrpRead {round}", t.name);
        // the request and the name are on the original's stack
        let mut words = vec![0u8; 0x60];
        words.copy_from_slice(&Oracle::read(&mut t.b, scratch, 0x60).unwrap());
        t.a.write(scratch, &words).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_MgrpRead {round} memory",
            t.name
        );
        // Ch_TextureLoadImage (and so Ch_TextureLoadAnim)
        t.put_u32(chr + 0x6fc, rng.next() % 2);
        let on = t.a.rebase(0x0130_07eb);
        t.put(on, &[u8::from(round % 3 != 0)]);
        let packet = t.alloc(0xa00);
        // the packet's length (0: nothing written)
        t.put_u32(draw_ret, if round % 4 == 0 { 0 } else { 0x40 });
        let sink = t.a.rebase(0x0230_56e8);
        t.put_u32(sink, 0);
        let x = rng.next();
        let at = t.a.rebase(0x0083_1300);
        t.a.call(at, Conv::Cdecl, &[chr, x]).unwrap_or_else(|e| {
            panic!(
                "{}: Ch_TextureLoadImage {round}: {e} 6fc {:#x}",
                t.name,
                rd::<u32>(&t.b, chr + 0x6fc)
            )
        });
        ta::ch_texture_load_image(&mut t.b, chr, x, packet, &mut |m, f, a| call(m, f, a));
        let p = Oracle::read(&mut t.b, packet, 0xa00).unwrap();
        t.a.write(packet, &p).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_TextureLoadImage {round}",
            t.name
        );
    }
}

#[test]
fn round9b_matches_the_original() {
    for mut t in twins() {
        round9b_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round9b_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mo;
    use ffx_chara::texture::bridge as tx;
    let mut rng = Xorshift(0x0009_b9b9);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let mut rets = Vec::new();
    for (k, (sym, n)) in [
        (0x0083_7ac0u32, 2u8),
        (0x0083_d040, 3),
        (0x0083_c580, 1),
        (0x0083_c590, 2),
        (0x0083_c550, 2),
        (0x0084_0560, 3),
        (0x0084_0dd0, 2),
        (0x0084_0960, 2),
        (0x0084_05f0, 3),
        (0x007f_cc80, 1),
        (0x0083_cb90, 1),
        (0x0083_c6f0, 2),
        (0x0083_c940, 2),
        (0x0068_7010, 1),
        (0x0083_c910, 0),
        (0x0064_2a10, 1),
        (0x0079_5b40, 2),
        // called with 5 and with 2 arguments: log 2
        (0x0082_6b70, 2),
        (0x0064_3340, 5),
        (0x0064_0100, 2),
    ]
    .into_iter()
    .enumerate()
    {
        rets.push((sym, fk.fake(t, sym, u32::try_from(k).unwrap() + 1, n)));
    }
    let ret = |sym: u32| rets.iter().find(|r| r.0 == sym).unwrap().1;
    let sets = t.a.rebase(0x00c4_9784);
    let chrs = live_characters(t, &mut rng);
    for round in 0..24u32 {
        fk.reset(t);
        for &(_, r) in &rets {
            t.put_u32(r, rng.next() % 3);
        }
        let chr = chrs[rng.next() as usize % chrs.len()];
        // Ch_SetSysMotion on a made-up definition of either form
        let def = t.alloc(0x110);
        t.put_u32(def, rng.next() % 0x100);
        t.put(def + 0x70, &[u8::from(round % 2 == 0)]);
        let tab = t.alloc(0x40);
        for k in 0..4 {
            let set = if rng.next().is_multiple_of(4) {
                0
            } else {
                t.alloc(0x50)
            };
            if set != 0 {
                for m in 0..0x14 {
                    t.put_u32(
                        set + 4 * m,
                        if rng.next().is_multiple_of(3) {
                            rng.next()
                        } else {
                            0
                        },
                    );
                }
            }
            t.put_u32(tab + 0x10 + 8 * k, set);
        }
        t.put_u32(def + 0x6c, tab);
        // the slots' sets within the made-up table (the game's are other)
        #[allow(clippy::cast_possible_truncation)]
        t.put(sets, &[0, 1, 2, 3].map(|_| (rng.next() % 4) as u8));
        let list = if round % 7 == 6 { 0 } else { t.alloc(0x20) };
        if list != 0 {
            for s in 0..3 {
                let n = rng.next() % 4;
                let pairs = t.alloc(4 * n as usize + 4);
                for i in 0..n {
                    #[allow(clippy::cast_possible_truncation)]
                    t.put(
                        pairs + 4 * i,
                        &[
                            (rng.next() % 6) as u8,
                            0,
                            rng.next() as u8,
                            rng.next() as u8,
                        ],
                    );
                }
                t.put_u32(list + 8 + 8 * s, n);
                t.put_u32(list + 0xc + 8 * s, pairs);
            }
        }
        t.put_u32(def + 0x108, list);
        t.put_u32(chr + 0x1c4, def);
        t.put_u32(chr + 0x198, rng.next() & 0x200);
        let slot = rng.next() % 3;
        let motion = if rng.next().is_multiple_of(3) {
            0x10
        } else {
            rng.next() % 6
        };
        t.check(
            &format!("Ch_SetSysMotion {round}"),
            0x0083_7d00,
            &[chr, slot, motion],
            |m| {
                Res::Int(mo::ch_set_sys_motion(
                    m,
                    chr,
                    slot,
                    motion,
                    &mut |m, f, a| call(m, f, a),
                ))
            },
        );
        // Ch_RegMotionEntryCache, Ch_RomReadChrCB
        let p = rng.next();
        t.check(
            &format!("Ch_RegMotionEntryCache {round}"),
            0x0083_76b0,
            &[p],
            |m| {
                Res::Int(mo::ch_reg_motion_entry_cache(m, p, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        let req = random_block(t, &mut rng, 0x20);
        t.check(
            &format!("Ch_RomReadChrCB {round}"),
            0x0082_a6c0,
            &[req],
            |m| Res::Int(mo::ch_rom_read_chr_cb(m, req, &mut |m, f, a| call(m, f, a))),
        );
        // Ch_ClutDispose (not with no clut: EAX is the caller's then)
        t.put_u32(chr + 0x704, rng.next() | 1);
        t.put_u32(ret(0x007f_cc80), u32::from(round % 5 == 0));
        let inst: u32 = rd(&t.b, chr + 0x830);
        let pend = t.a.rebase(0x012f_4110);
        t.put_u32(pend, rng.next() % 2);
        t.put_u32(pend + 4, [0, inst, inst ^ 4][rng.next() as usize % 3]);
        t.check(
            &format!("Ch_ClutDispose {round}"),
            0x0083_c810,
            &[chr],
            |m| Res::Int(tx::ch_clut_dispose(m, chr, &mut |m, f, a| call(m, f, a))),
        );
    }
}

#[test]
fn round10_matches_the_original() {
    for mut t in twins() {
        round10_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round10_on(t: &mut Twin) {
    use ffx_chara::cloth::bridge as cl;
    use ffx_chara::motion::bridge as mo;
    use ffx_chara::texanim::bridge as ta;
    use ffx_chara::texture::bridge as tx;
    let mut rng = Xorshift(0x0010_1010);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let mut rets = Vec::new();
    let heap = fk.fake_alloc(t, 0x0068_7020, 50, 0x100);
    for (k, (sym, n)) in [
        (0x0062_f4e0u32, 2u8),
        (0x0068_7010, 1),
        (0x0076_efa0, 3),
        (0x0083_7d00, 3),
        (0x007f_cc80, 1),
        (0x0083_c810, 1),
        (0x0083_c4a0, 1),
        (0x0083_c580, 1),
        (0x0083_ca20, 2),
        (0x0083_c940, 2),
    ]
    .into_iter()
    .enumerate()
    {
        rets.push((sym, fk.fake(t, sym, u32::try_from(k).unwrap() + 1, n)));
    }
    let ret = |sym: u32| rets.iter().find(|r| r.0 == sym).unwrap().1;
    let (shut, shut_ret) = fk.fake_code(t, 100, 0);
    let chrs = live_characters(t, &mut rng);
    for round in 0..16u32 {
        fk.reset(t);
        for &(_, r) in &rets {
            t.put_u32(r, rng.next() % 3);
        }
        t.put_u32(shut_ret, rng.next());
        let blocks = t.alloc(0x100 * 0x20);
        t.put_u32(heap, blocks);
        let chr = chrs[rng.next() as usize % chrs.len()];
        // models with cloth meshes and bones
        let nmod = rng.next() % 4;
        let hdr = t.alloc(0x10);
        #[allow(clippy::cast_possible_truncation)]
        {
            t.put(hdr + 6, &(nmod as u16).to_le_bytes());
            t.put(hdr + 8, &((rng.next() % 2) as u16).to_le_bytes());
        }
        let models = t.alloc(0x38 * 4);
        let bones = random_block(t, &mut rng, 0x160 * 4);
        for i in 0..nmod {
            let d = t.alloc(0x20);
            #[allow(clippy::cast_possible_truncation)]
            {
                t.put(d, &((rng.next() % 4) as u16).to_le_bytes());
                t.put(d + 0x18, &((rng.next() % 3) as u16).to_le_bytes());
            }
            let m = models + 0x38 * i;
            t.put_u32(m, d);
            // some lists already there, with some blocks
            let list = if rng.next().is_multiple_of(2) {
                0
            } else {
                t.alloc(0x10)
            };
            if list != 0 {
                for k in 0..3 {
                    let b = if rng.next().is_multiple_of(2) {
                        0
                    } else {
                        random_block(t, &mut rng, 0x50)
                    };
                    t.put_u32(list + 4 * k, b);
                }
            }
            t.put_u32(m + 0x2c, list);
            t.put_u32(m + 0x30, rng.next());
        }
        t.put_u32(chr + 0x1c0, hdr);
        t.put_u32(chr + 0x1bc, models);
        t.put_u32(chr + 0x328, bones);
        t.put_u32(chr + 0x194, if round % 3 == 0 { 0 } else { 0x20_0000 });
        t.check(
            &format!("Ch_ClothMeshDestroy {round}"),
            0x0082_6260,
            &[chr],
            |m| {
                Res::Int(cl::ch_cloth_mesh_destroy(m, chr, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        t.check(
            &format!("Ch_GetRestPoseMatrices {round}"),
            0x0082_6ba0,
            &[chr],
            |m| {
                Res::Int(cl::ch_get_rest_pose_matrices(m, chr, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        // Ch_MotionCancel
        t.check(
            &format!("Ch_MotionCancel {round}"),
            0x0083_7180,
            &[chr],
            |m| Res::Int(mo::ch_motion_cancel(m, chr, &mut |m, f, a| call(m, f, a))),
        );
        // chr_texanim_del: ids in and out of range, each kind
        let id = [0i32, 5, 63, 64, -1, 17][rng.next() as usize % 6];
        let tab = t.a.rebase(0x0231_0000);
        let e = tab.wrapping_add(0x30u32.wrapping_mul(id.cast_unsigned()));
        if (0..64).contains(&id) {
            let work = if rng.next().is_multiple_of(4) {
                0
            } else {
                t.alloc(0x10)
            };
            if work != 0 {
                // the shutdown table at +0x460f8
                let ft = t.alloc(8);
                t.put_u32(ft + 4, shut);
                let base = t.alloc(0x46100);
                t.put_u32(base + 0x460f8, ft);
                t.put_u32(e + 4, base);
            } else {
                t.put_u32(e + 4, 0);
            }
            #[allow(clippy::cast_possible_truncation)]
            t.put(e + 0xc, &[[0u8, 1, 2, 3, 0xff][rng.next() as usize % 5]]);
            t.put_u32(e, rng.next());
            t.put_u32(e + 8, rng.next());
        }
        if (0..64).contains(&id) || t.name == "synthetic" {
            t.check(
                &format!("chr_texanim_del {id} {round}"),
                0x0077_c770,
                &[id.cast_unsigned()],
                |m| Res::Int(ta::chr_texanim_del(m, id, &mut |m, f, a| call(m, f, a))),
            );
        }
        // Ch_ClutCreate: cluts of the definition's texture data
        t.put_u32(ret(0x007f_cc80), u32::from(round % 4 == 3));
        let n = rng.next() % 4;
        let def = t.alloc(0x20);
        let texd = t.alloc(0x20);
        let list = t.alloc(8 * 4);
        for k in 0..n {
            let data = t.alloc(0x20) + 0x10;
            t.put_u32(data - 0x10, rng.next());
            t.put_u32(list + 8 * k + 4, data);
        }
        t.put_u32(texd + 0x18, list);
        t.put_u32(def + 0x14, texd);
        t.put_u32(chr + 0x1c4, def);
        #[allow(clippy::cast_possible_truncation)]
        t.put(chr + 0x6e8, &(n as i16).to_le_bytes());
        let name = t.alloc(16);
        t.put(name, b"c010\0");
        t.put_u32(chr + 4, name);
        let scratch = t.alloc(0x40);
        let at = t.a.rebase(0x0083_c710);
        let ra = t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
        let rb = tx::ch_clut_create(&mut t.b, chr, scratch, &mut |m, f, a| call(m, f, a));
        assert_eq!(ra.eax, rb, "{}: Ch_ClutCreate {round}", t.name);
        let s = Oracle::read(&mut t.b, scratch, 0x40).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_ClutCreate {round} memory",
            t.name
        );
    }
}

#[test]
fn round10b_matches_the_original() {
    for mut t in twins() {
        round10b_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round10b_on(t: &mut Twin) {
    use ffx_chara::cloth::bridge as cl;
    use ffx_chara::motion::bridge as mo;
    let mut rng = Xorshift(0x0010_b10b);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let heap = fk.fake_alloc(t, 0x0068_7020, 1, 0x200);
    let clear = fk.fake(t, 0x0062_f4e0, 2, 2);
    let free = fk.fake(t, 0x0068_7010, 3, 1);
    let mesh_data = fk.fake(t, 0x0084_07d0, 4, 2);
    let step = fk.fake(t, 0x0083_94d0, 5, 2);
    let chrs = live_characters(t, &mut rng);
    for round in 0..16u32 {
        fk.reset(t);
        for r in [clear, free, step] {
            t.put_u32(r, rng.next());
        }
        let blocks = t.alloc(0x200 * 0x40);
        t.put_u32(heap, blocks);
        let chr = chrs[rng.next() as usize % chrs.len()];
        // models: data entries with mesh and vertex counts
        let nmod = 1 + rng.next() % 3;
        let hdr = t.alloc(0x10);
        #[allow(clippy::cast_possible_truncation)]
        {
            t.put(hdr + 6, &(nmod as u16).to_le_bytes());
            t.put(hdr + 0xa, &((rng.next() % 4) as u16).to_le_bytes());
        }
        let models = t.alloc(0x38 * 3);
        for i in 0..nmod {
            let d = t.alloc(0x20);
            #[allow(clippy::cast_possible_truncation)]
            {
                t.put(d + 4, &((8 + rng.next() % 8) as u16).to_le_bytes());
                t.put(d + 0x18, &((1 + rng.next() % 3) as u16).to_le_bytes());
            }
            let m = models + 0x38 * i;
            t.put_u32(m, d);
            let list = if rng.next().is_multiple_of(2) {
                0
            } else {
                t.alloc(0x10)
            };
            if list != 0 {
                for k in 0..3 {
                    let b = if rng.next().is_multiple_of(2) {
                        0
                    } else {
                        t.alloc(0x50)
                    };
                    t.put_u32(list + 4 * k, b);
                }
            }
            t.put_u32(m + 0x2c, list);
            let flags = if rng.next().is_multiple_of(2) {
                0
            } else {
                t.alloc(0x10)
            };
            t.put_u32(m + 0x30, flags);
        }
        t.put_u32(chr + 0x1c0, hdr);
        t.put_u32(chr + 0x1bc, models);
        // cloth mesh data: the box, then records
        let data = if round % 4 == 3 {
            0
        } else {
            let d = t.alloc(0x400);
            for k in 0..6 {
                t.put_u32(d + 4 * k, small_float(&mut rng));
            }
            let mut p = d + 0x18;
            for _ in 0..rng.next() % 3 {
                let tris = rng.next() % 3;
                #[allow(clippy::cast_possible_truncation)]
                let rec = [
                    (rng.next() % nmod) as u16,
                    (rng.next() % 2) as u16,
                    tris as u16,
                    0,
                ];
                t.put(p, &rec.map(u16::to_le_bytes).concat());
                p += 8;
                for _ in 0..tris {
                    #[allow(clippy::cast_possible_truncation)]
                    let tri = [0; 16].map(|_| (rng.next() % 8) as u16);
                    t.put(p, &tri.map(u16::to_le_bytes).concat());
                    p += 0x20;
                }
            }
            t.put(p, &[0xff, 0xff]);
            d
        };
        t.put_u32(mesh_data, data);
        t.check(
            &format!("Ch_ClothMeshMapping {round}"),
            0x0082_6340,
            &[chr],
            |m| {
                Res::Int(cl::ch_cloth_mesh_mapping(m, chr, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        // Ch_MotionSetupFcurve: a clip with all four channel kinds
        let chans = t.alloc(0x2c * 40);
        t.put_u32(chr + 0x748, chans);
        let clip = t.alloc(0x200);
        #[allow(clippy::cast_possible_truncation)]
        {
            t.put(clip + 2, &((rng.next() % 5) as i16).to_le_bytes());
            t.put(clip + 4, &(rng.next() as u16).to_le_bytes());
            t.put(clip + 6, &((rng.next() % 2) as u16).to_le_bytes());
        }
        t.put_u32(clip + 8, 0x20);
        t.put_u32(clip + 0xc, 0x60);
        t.put_u32(clip + 0x10, rng.next());
        for k in 0..4 {
            t.put_u32(clip + 0x20 + 4 * k, rng.next());
        }
        // data: constants and streams whose first halfword is a small length
        for k in 0..0x50u32 {
            #[allow(clippy::cast_possible_truncation)]
            let v = (rng.next() % 9) as u16;
            t.put(clip + 0x60 + 2 * k, &v.to_le_bytes());
        }
        let given = if round % 5 == 4 { 0 } else { clip };
        t.check(
            &format!("Ch_MotionSetupFcurve {round}"),
            0x0083_9980,
            &[chr, given],
            |m| {
                Res::Int(mo::ch_motion_setup_fcurve(m, chr, given, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
    }
}

#[test]
fn round10c_matches_the_original() {
    for mut t in twins() {
        round10c_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round10c_on(t: &mut Twin) {
    use ffx_chara::chr::bridge as c;
    let mut rng = Xorshift(0x0010_c10c);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let heap = fk.fake_alloc(t, 0x0068_7020, 1, 0x400);
    let normalize = fk.fake(t, 0x006e_d6f0, 2, 2);
    // the name buffer is the original's stack: log only the block
    let set_str = fk.fake(t, 0x0083_c580, 3, 1);
    let set_type = fk.fake(t, 0x0083_c590, 4, 2);
    let reset = fk.fake(t, 0x0082_4650, 5, 1);
    let magic = fk.fake(t, 0x009d_a3b0, 6, 0);
    let visible = fk.fake(t, 0x0063_c390, 7, 2);
    for round in 0..24u32 {
        fk.reset(t);
        for r in [normalize, set_str, set_type, reset, visible] {
            t.put_u32(r, rng.next());
        }
        t.put_u32(
            magic,
            if rng.next().is_multiple_of(2) {
                0x295
            } else {
                rng.next() % 0x400
            },
        );
        let blocks = t.alloc(0x400 * 0x10);
        t.put_u32(heap, blocks);
        let chr = t.alloc(0x900);
        let id: u16 = if rng.next().is_multiple_of(2) {
            0x402e
        } else {
            0x1000
        };
        t.put(chr, &id.to_le_bytes());
        t.put(chr + 0x180, &[0]);
        t.put_u32(chr + 0x198, rng.next());
        t.put_u32(chr + 0x830, rng.next());
        let name = t.alloc(16);
        t.put(name, b"c051\0");
        t.put_u32(chr + 4, name);
        let nmod = rng.next() % 4;
        let hdr = t.alloc(0x10);
        #[allow(clippy::cast_possible_truncation)]
        t.put(hdr + 6, &(nmod as u16).to_le_bytes());
        t.put_u32(chr + 0x1c0, hdr);
        let models = t.alloc(0x38 * 4);
        t.put_u32(chr + 0x1bc, models);
        for i in 0..nmod {
            let m = models + 0x38 * i;
            let d = t.alloc(0x20);
            t.put_u32(m, d);
            for off in [4u32, 8, 0xc] {
                let v = if rng.next().is_multiple_of(3) {
                    rng.next()
                } else {
                    0
                };
                t.put_u32(m + off, v);
            }
            t.put_u32(m + 0x10, rng.next());
            let nv = rng.next() % 6;
            let nn = rng.next() % 6;
            let np = rng.next() % 4;
            for (off, n) in [(4u32, nv), (0x16, nn), (0x18, np)] {
                #[allow(clippy::cast_possible_truncation)]
                t.put(d + off, &(n as u16).to_le_bytes());
            }
            for (off, n) in [(8u32, nv), (0xc, nn)] {
                let src = t.alloc(6 * 6);
                for k in 0..n * 3 {
                    #[allow(clippy::cast_possible_truncation)]
                    t.put(src + 2 * k, &(rng.next() as u16).to_le_bytes());
                }
                t.put_u32(d + off, src);
            }
            let ents = t.alloc(0x24 * 4);
            for k in 0..np {
                let e = ents + 0x24 * k;
                #[allow(clippy::cast_possible_truncation)]
                {
                    t.put(e + 0x13, &[(rng.next() % 16) as u8]);
                    t.put(e + 0x1c, &((rng.next() % 32) as u16).to_le_bytes());
                }
            }
            t.put_u32(d + 0x1c, ents);
        }
        let scratch = t.alloc(c::CREATE_PACKET_SCRATCH as usize);
        let at = t.a.rebase(0x0082_7280);
        let ra = t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
        let rb = c::ch_model_create_packet(&mut t.b, chr, scratch, &mut |m, f, a| call(m, f, a));
        assert_eq!(ra.eax, rb, "{}: Ch_ModelCreatePacket {round}", t.name);
        let s = Oracle::read(&mut t.b, scratch, c::CREATE_PACKET_SCRATCH as usize).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_ModelCreatePacket {round} memory",
            t.name
        );
    }
}

#[test]
fn round10d_matches_the_original() {
    for mut t in twins() {
        round10d_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round10d_on(t: &mut Twin) {
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x0010_d10d);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let heap = fk.fake_alloc(t, 0x0068_7020, 1, 0x1000);
    let mut rets = vec![];
    for (sym, id) in [
        (0x0077_fc90, 2),
        (0x0077_fda0, 3),
        (0x0078_0750, 4),
        (0x0078_08f0, 5),
    ] {
        rets.push(fk.fake(t, sym, id, 3));
    }
    for round in 0..24u32 {
        fk.reset(t);
        for &r in &rets {
            t.put_u32(r, rng.next());
        }
        let blocks = t.alloc(0x1000 * 2);
        t.put_u32(heap, blocks);
        let id = rng.next() % 64;
        let e = t.a.rebase(0x0231_0000) + 0x30 * id;
        let src = t.alloc(0x2300);
        t.put_u32(e, src);
        t.put_u32(src, rng.next());
        t.put_u32(src + 8, 0x40);
        for k in 0..32 {
            #[allow(clippy::cast_possible_truncation)]
            let m = [0xffu8, 0, 1, 2, 3, 4, 5, 0xfe][rng.next() as usize % 8];
            t.put(src + 0xc + k, &[m]);
        }
        for k in 0..(0x2200 / 4) {
            // floats without signalling NaNs
            let mut v = rng.next();
            if v & 0x7f80_0000 == 0x7f80_0000 {
                v |= 0x0040_0000;
            }
            t.put_u32(src + 0x40 + 4 * k, v);
        }
        let def = t.alloc(0x20);
        let n = rng.next() % 5;
        #[allow(clippy::cast_possible_truncation)]
        t.put(def + 0xc, &(n as u16).to_le_bytes());
        let ents = t.alloc(0x10 * 5);
        for k in 0..n * 4 {
            t.put_u32(ents + 4 * k, rng.next());
        }
        t.put_u32(def + 0x1c, ents);
        let handle = rng.next();
        let old = round % 2 == 1;
        let (what, at) = if old {
            ("chr_texanim_set_old", 0x0078_03f0)
        } else {
            ("chr_texanim_set_new", 0x0077_f9c0)
        };
        t.check(&format!("{what} {round}"), at, &[id, handle, def], |m| {
            Res::Int(if old {
                ta::chr_texanim_set_old(m, id, handle, def, &mut |m, f, a| call(m, f, a))
            } else {
                ta::chr_texanim_set_new(m, id, handle, def, &mut |m, f, a| call(m, f, a))
            })
        });
    }
}

#[test]
fn round10e_matches_the_original() {
    for mut t in twins() {
        round10e_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn round10e_on(t: &mut Twin) {
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x0010_e10e);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let small = |rng: &mut Xorshift, n: u32| rng.next() % n;
    for round in 0..160u32 {
        let id = rng.next() % 64;
        let e = t.a.rebase(0x0231_0000) + 0x30 * id;
        let src = t.alloc(0x4000);
        t.put_u32(e, if round % 16 == 15 { 0 } else { src });
        let kind = if round % 8 == 7 { 1 } else { 2u8 };
        t.put(src, &[kind]);
        t.put_u32(src + 4, 0x100);
        t.put_u32(src + 8, 0x400);
        for r in 0..8 {
            let rect = src + 0x100 + 12 * r;
            let w = 1 + small(&mut rng, 8) as u16;
            let h = 1 + small(&mut rng, 6) as u16;
            t.put(rect, &(rng.next() as u16).to_le_bytes());
            t.put(rect + 2, &(rng.next() as u16).to_le_bytes());
            t.put(rect + 4, &w.to_le_bytes());
            t.put(rect + 6, &h.to_le_bytes());
            t.put_u32(rect + 8, 0x800 + 0x100 * r);
        }
        for k in 0..0x800 / 4 {
            t.put_u32(src + 0x800 + 4 * k, rng.next());
        }
        let track = small(&mut rng, 4);
        t.put_u32(e + 8, track);
        for k in 0..4 {
            let ss = src + 0x400 + 0x90 * k;
            t.put(ss + 8, &(small(&mut rng, 16) as u16).to_le_bytes());
            t.put(ss + 0xa, &(small(&mut rng, 8) as u16).to_le_bytes());
            t.put(ss + 0xc, &(small(&mut rng, 4) as u16).to_le_bytes());
            let list = ss + 8 * track;
            t.put_u32(list + 0x10, 0x1800 + 0x40 * k);
            let cnt = if rng.next().is_multiple_of(6) {
                0
            } else {
                1 + small(&mut rng, 4)
            };
            t.put(list + 0x14, &(cnt as u16).to_le_bytes());
            for f in 0..4 {
                let fr = src + 0x1800 + 0x40 * k + 4 * f;
                let time = small(&mut rng, 6) as i16 - 1;
                t.put(fr, &time.to_le_bytes());
                t.put(fr + 2, &[small(&mut rng, 8) as u8, rng.next() as u8]);
            }
        }
        let buf = t.alloc(0x600);
        t.put_u32(e + 4, buf);
        t.put_u32(buf + 0x400, small(&mut rng, 6).wrapping_sub(1));
        for k in 0..16 {
            let m = [0xffu8, 0, 1, 2][small(&mut rng, 4) as usize];
            t.put(buf + 0x5a8 + k, &[m]);
        }
        let dst = t.alloc(0x1000);
        for k in 0..4 {
            t.put_u32(buf + 0x5b8 + 4 * k, dst + 0x10 * k);
            t.put(buf + 0x5c8 + 2 * k, &32u16.to_le_bytes());
        }
        for k in 0..4 {
            let s = buf + 0x420 + 0x34 * k;
            for off in [0x18u32, 0x10, 0xe, 0xc, 0xa, 8, 6] {
                t.put_u32(s - off, rng.next());
            }
            t.put_u32(s - 0x14, small(&mut rng, 4));
            t.put(s - 4, &(small(&mut rng, 8) as u16).to_le_bytes());
            t.put(s - 2, &(small(&mut rng, 4) as u16).to_le_bytes());
            t.put(s, &(small(&mut rng, 4) as i16 - 1).to_le_bytes());
            t.put(s + 2, &(small(&mut rng, 4) as u16).to_le_bytes());
            t.put(s + 4, &(small(&mut rng, 6) as u16).to_le_bytes());
            t.put(s + 6, &[0x55, 0x55]);
            t.put_u32(s + 8, rng.next());
            for off in [0xcu32, 0x10] {
                let v = (rng.next() % 16) as f32 / 2.0 - 4.0;
                t.put_u32(s + off, v.to_bits());
            }
            for off in [0x14u32, 0x18] {
                let v = (rng.next() % 32) as f32 / 2.0 - 8.0;
                t.put_u32(s + off, v.to_bits());
            }
        }
        t.check(
            &format!("chr_texanim_main_sub_old {round}"),
            0x0077_ff00,
            &[id],
            |m| {
                Res::Int(ta::chr_texanim_main_sub_old(m, id, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
    }
}

#[test]
fn round10f_matches_the_original() {
    for mut t in twins() {
        round10f_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round10f_on(t: &mut Twin) {
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x0010_f10f);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let heap = fk.fake_alloc(t, 0x0068_7020, 1, 0x1000);
    let mut rets = vec![];
    for (sym, id) in [
        (0x0077_fc90, 2),
        (0x0077_fda0, 3),
        (0x0078_0750, 4),
        (0x0078_08f0, 5),
    ] {
        rets.push(fk.fake(t, sym, id, 3));
    }
    rets.push(fk.fake(t, 0x0076_efa0, 6, 3));
    let table = t.a.rebase(0x0231_0000);
    for round in 0..32u32 {
        fk.reset(t);
        for &r in &rets {
            t.put_u32(r, rng.next());
        }
        let blocks = t.alloc(0x1000 * 2);
        t.put_u32(heap, blocks);
        let free = if round % 8 == 7 { 64 } else { rng.next() % 64 };
        for i in 0..64 {
            t.put_u32(table + 0x30 * i, if i < free { 0x0100_0000 + i } else { 0 });
        }
        let src = t.alloc(0x2300);
        #[allow(clippy::cast_possible_truncation)]
        t.put(
            src,
            &[
                (rng.next() % 4) as u8,
                rng.next() as u8,
                (rng.next() % 3) as u8,
            ],
        );
        t.put_u32(src + 8, 0x40);
        for k in 0..32 {
            #[allow(clippy::cast_possible_truncation)]
            let m = [0xffu8, 0, 1, 2, 3, 4, 5, 0xfe][rng.next() as usize % 8];
            t.put(src + 0xc + k, &[m]);
        }
        let def = t.alloc(0x40);
        let n = rng.next() % 5;
        #[allow(clippy::cast_possible_truncation)]
        t.put(def + 0xc, &(n as u16).to_le_bytes());
        let ents = t.alloc(0x10 * 5);
        for k in 0..n * 4 {
            t.put_u32(ents + 4 * k, rng.next());
        }
        t.put_u32(def + 0x1c, ents);
        t.put_u32(def + 0x20, src);
        let handle = rng.next();
        t.check(
            &format!("chr_texanim_set {round}"),
            0x0077_c950,
            &[def, handle],
            |m| {
                Res::Int(ta::chr_texanim_set(m, def, handle, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
    }
}

#[test]
fn round10g_matches_the_original() {
    for mut t in twins() {
        round10g_on(&mut t);
    }
}

fn round10g_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mo;
    let mut rng = Xorshift(0x0010_a10a);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let fcurve = fk.fake(t, 0x0083_9980, 1, 2);
    let map = t.a.rebase(0x0133_0844);
    for round in 0..32u32 {
        fk.reset(t);
        t.put_u32(fcurve, rng.next());
        t.put_u32(
            map,
            if rng.next().is_multiple_of(2) {
                0x7a
            } else {
                0x10
            },
        );
        let chr = t.alloc(0x800);
        for k in 0..0x200 {
            t.put_u32(chr + 4 * k, rng.next());
        }
        let name = t.alloc(8);
        t.put(
            name,
            if rng.next().is_multiple_of(2) {
                b"c001\0"
            } else {
                b"c00\0\0"
            },
        );
        t.put_u32(chr + 4, name);
        if rng.next().is_multiple_of(2) {
            t.put(chr + 0x726, &[0xff, 0xff]);
        }
        if rng.next().is_multiple_of(3) {
            t.put(chr + 0x73a, &[0, 0]);
        }
        let clip = t.alloc(4);
        t.put_u32(clip, rng.next());
        let loops = rng.next();
        let start = if rng.next().is_multiple_of(2) {
            0x3000
        } else {
            (rng.next() % 0x4000).cast_signed()
        };
        let end = match rng.next() % 3 {
            0 => -0x100,
            1 => -(rng.next() % 9).cast_signed(),
            _ => (rng.next() % 0x8000).cast_signed(),
        };
        t.check(
            &format!("Ch_SetMotionSeq {round}"),
            0x0083_9b80,
            &[chr, clip, loops, start.cast_unsigned(), end.cast_unsigned()],
            |m| {
                Res::Int(mo::ch_set_motion_seq(
                    m,
                    chr,
                    clip,
                    loops,
                    start,
                    end,
                    &mut |m, f, a| call(m, f, a),
                ))
            },
        );
    }
}

/// Calls a one-operand intrinsic (third word) on `fld qword [esp + 4]`,
/// stores the result as `f32` and returns its bits in EAX.
const CI1_STUB: [u8; 17] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xd9, 0x5c, 0x24, 0x04, 0x8b, 0x44, 0x24, 0x04,
    0xc3,
];
/// The two-operand form: `[esp + 4]` to ST1, `[esp + 0xc]` to ST0, the
/// intrinsic in the fifth word.
const CI2_STUB: [u8; 21] = [
    0xdd, 0x44, 0x24, 0x04, 0xdd, 0x44, 0x24, 0x0c, 0xff, 0x54, 0x24, 0x14, 0xd9, 0x5c, 0x24, 0x04,
    0x8b, 0x44, 0x24, 0x04, 0xc3,
];

/// Code at the same new address in both oracles.
fn twin_code(t: &mut Twin, code: &[u8]) -> u32 {
    let at = t.a.alloc_code(code).unwrap();
    assert_eq!(t.b.alloc_code(code).unwrap(), at);
    at
}

/// Replaces the function at `sym` with a jump to `code` in both.
fn fake_with(t: &mut Twin, sym: u32, code: &[u8]) {
    let to = twin_code(t, code);
    let at = t.a.rebase(sym);
    let mut jmp = vec![0xe9];
    jmp.extend(to.wrapping_sub(at + 5).to_le_bytes());
    t.put(at, &jmp);
}

/// A stand-in copying the 16 bytes at `src` (plus `id & 0x1f` × 16 when
/// `id_arg` names the word holding an id) to the pointer in word `out`.
fn copy_stub(src: u32, id_arg: Option<u8>, out: u8) -> Vec<u8> {
    let mut c = vec![];
    if let Some(k) = id_arg {
        c.extend([
            0x8b,
            0x44,
            0x24,
            4 * k,
            0x83,
            0xe0,
            0x1f,
            0xc1,
            0xe0,
            0x04,
            0x05,
        ]);
    } else {
        c.push(0xb8);
    }
    c.extend(src.to_le_bytes());
    c.extend([0x8b, 0x54, 0x24, 4 * out]);
    for k in [0u8, 4, 8, 0xc] {
        c.extend([0x8b, 0x48, k, 0x89, 0x4a, k]);
    }
    c.push(0xc3);
    c
}

#[test]
fn round11_matches_the_original() {
    for mut t in twins() {
        round11_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
#[allow(clippy::cast_precision_loss)]
fn round11_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0011_0011);
    let fk = Fakes::new(t);
    let elem = fk.fake(t, 0x0083_5660, 1, 3);
    let set = fk.fake(t, 0x0083_3df0, 2, 4);
    let check = fk.fake(t, 0x0082_61b0, 3, 1);
    let exist = fk.fake(t, 0x0083_5680, 4, 2);
    let pts = t.alloc(0x20 * 16);
    let cam = t.alloc(16);
    let opt = copy_stub(pts, Some(2), 3);
    fake_with(t, 0x0083_54f0, &opt);
    let camera = copy_stub(cam, None, 1);
    fake_with(t, 0x0083_f260, &camera);
    let ci1 = twin_code(t, &CI1_STUB);
    let ci2 = twin_code(t, &CI2_STUB);
    let pop = t.b.rebase(FPU_POP);
    let f = |rng: &mut Xorshift, lo: i32, hi: i32| {
        let span = u32::try_from(hi - lo).unwrap() * 16;
        ((rng.next() % span) as f32 / 16.0 + lo as f32).to_bits()
    };
    for round in 0..64u32 {
        fk.reset(t);
        t.put_u32(
            elem,
            if round % 16 == 15 {
                0
            } else {
                0x100 + rng.next() % 8
            },
        );
        t.put_u32(set, rng.next());
        t.put_u32(check, u32::from(rng.next().is_multiple_of(4)));
        t.put_u32(exist, u32::from(!rng.next().is_multiple_of(4)));
        for k in 0..0x20 * 4 {
            let v = f(&mut rng, -10, 10);
            t.put_u32(pts + 4 * k, v);
        }
        for k in 0..4 {
            let v = f(&mut rng, -20, 20);
            t.put_u32(cam + 4 * k, v);
        }
        let chr = t.alloc(0x900);
        for k in 0..0x240 {
            let v = f(&mut rng, -2, 2);
            t.put_u32(chr + 4 * k, v);
        }
        let model = t.alloc(0x80);
        t.put(model + 0x40, &[u8::from(rng.next().is_multiple_of(2))]);
        t.put_u32(chr + 0x1c4, model);
        #[allow(clippy::cast_possible_truncation)]
        t.put(chr + 0x40d, &[1 + (rng.next() % 4) as u8]);
        t.put_u32(
            chr + 0x414,
            (if rng.next().is_multiple_of(4) {
                0f32
            } else {
                1f32
            })
            .to_bits(),
        );
        let w = (rng.next() % 17) as f32 / 16.0;
        t.put_u32(chr + 0x418, w.to_bits());
        t.put_u32(
            chr + 0x420,
            [1f32, 2.0, 4.0, 8.0][(rng.next() % 4) as usize].to_bits(),
        );
        t.put_u32(chr + 0x158, f(&mut rng, -3, 3));
        t.put_u32(chr + 0x424, f(&mut rng, 0, 120));
        t.put_u32(chr + 0x428, f(&mut rng, -120, 0));
        t.put_u32(chr + 0x41c, f(&mut rng, -180, 180));
        t.put_u32(chr + 0x4d0, rng.next() % 2);
        if rng.next().is_multiple_of(2) {
            t.put_u32(chr + 0x4c0, f(&mut rng, -3, 3));
            t.put_u32(chr + 0x4c4, f(&mut rng, -3, 3));
        }
        t.put_u32(
            chr + 0x4bc,
            [0f32, 0.25, 0.5, 1.0][(rng.next() % 4) as usize].to_bits(),
        );
        // a unit quaternion at +0x4ac
        for k in 0..4 {
            t.put_u32(chr + 0x4ac + 4 * k, 0.5f32.to_bits());
        }
        let scratch = t.alloc(p::NECK_SCRATCH as usize);
        let at = t.a.rebase(0x0083_48d0);
        let ra = t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
        let rb = {
            let mut call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
            let mut fcall = |m: &mut Oracle, f: u32, a: &[u32]| {
                let r = m.call(f, Conv::Cdecl, a).unwrap();
                m.call(pop, Conv::Cdecl, &[]).unwrap();
                r.st0.expect("a float result")
            };
            let mut ci = |m: &mut Oracle, f: u32, a: &[f64]| {
                let words: Vec<u32> = a
                    .iter()
                    .flat_map(|x| {
                        let b = x.to_bits();
                        #[allow(clippy::cast_possible_truncation)]
                        [b as u32, (b >> 32) as u32]
                    })
                    .chain([f])
                    .collect();
                let stub = if a.len() == 1 { ci1 } else { ci2 };
                f32::from_bits(m.call(stub, Conv::Cdecl, &words).unwrap().eax)
            };
            let mut c = p::NeckCalls {
                call: &mut call,
                fcall: &mut fcall,
                ci: &mut ci,
            };
            p::ch_neck(&mut t.b, chr, scratch, &mut c)
        };
        assert_eq!(ra.eax, rb, "{}: Ch_Neck {round}", t.name);
        let s = Oracle::read(&mut t.b, scratch, p::NECK_SCRATCH as usize).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_Neck {round} memory",
            t.name
        );
    }
}

#[test]
fn round11b_matches_the_original() {
    for mut t in twins() {
        round11b_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
#[allow(clippy::many_single_char_names)]
fn round11b_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0011_b11b);
    let fk = Fakes::new(t);
    let elem = fk.fake(t, 0x0083_5660, 1, 3);
    let limit = fk.fake(t, 0x0083_4fd0, 2, 3);
    let pop = t.b.rebase(FPU_POP);
    let f = |rng: &mut Xorshift, lo: i32, hi: i32| {
        let span = u32::try_from(hi - lo).unwrap() * 16;
        ((rng.next() % span) as f32 / 16.0 + lo as f32).to_bits()
    };
    for round in 0..32u32 {
        fk.reset(t);
        t.put_u32(elem, if round % 8 == 7 { 0 } else { rng.next() % 4 });
        t.put_u32(limit, rng.next());
        let chr = t.alloc(0x500);
        for k in 0..0x140 {
            let v = f(&mut rng, -2, 2);
            t.put_u32(chr + 4 * k, v);
        }
        let elems = t.alloc(0x160 * 4);
        for k in 0..0x160 {
            let v = f(&mut rng, -3, 3);
            t.put_u32(elems + 4 * k, v);
        }
        let bones = t.alloc(0xc8);
        for k in 0..0x32 {
            let v = f(&mut rng, -2, 2);
            t.put_u32(bones + 4 * k, v);
        }
        for k in 0..4 {
            t.put_u32(elems + 0x160 * k, bones);
        }
        t.put_u32(chr + 0x328, elems);
        let scratch = t.alloc(p::NECK_INIT_SCRATCH as usize);
        let at = t.a.rebase(0x0083_4e60);
        t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
        {
            let mut call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
            let mut fcall = |m: &mut Oracle, f: u32, a: &[u32]| {
                let r = m.call(f, Conv::Cdecl, a).unwrap();
                m.call(pop, Conv::Cdecl, &[]).unwrap();
                r.st0.expect("a float result")
            };
            let mut ci = |_: &mut Oracle, _: u32, _: &[f64]| f32::NAN;
            let mut c = p::NeckCalls {
                call: &mut call,
                fcall: &mut fcall,
                ci: &mut ci,
            };
            p::ch_neck_init(&mut t.b, chr, scratch, &mut c);
        }
        let s = Oracle::read(&mut t.b, scratch, p::NECK_INIT_SCRATCH as usize).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_NeckInit {round}",
            t.name
        );
        // Ch_ElemCalcQuaternion
        let e = t.alloc(0x100);
        for k in 0..0x40 {
            let v = f(&mut rng, -2, 2);
            t.put_u32(e + 4 * k, v);
        }
        let other = t.alloc(0x40);
        for k in 0..0x10 {
            let v = f(&mut rng, -2, 2);
            t.put_u32(other + 4 * k, v);
        }
        t.put_u32(e + 0xcc, other);
        let w = [0f32, 1.0, 0.5, 0.25][usize::try_from(rng.next() % 4).unwrap()];
        t.put_u32(e + 0xe0, w.to_bits());
        let scratch = t.alloc(p::ELEM_QUAT_SCRATCH as usize);
        let at = t.a.rebase(0x0083_3960);
        t.a.call(at, Conv::Cdecl, &[e]).unwrap();
        let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
        p::ch_elem_calc_quaternion(&mut t.b, e, scratch, &mut |m, f, a| call(m, f, a));
        let s = Oracle::read(&mut t.b, scratch, p::ELEM_QUAT_SCRATCH as usize).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_ElemCalcQuaternion {round}",
            t.name
        );
    }
}

#[test]
fn round11c_matches_the_original() {
    for mut t in twins() {
        round11c_on(&mut t);
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
fn round11c_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0011_c11c);
    let fk = Fakes::new(t);
    let elem = fk.fake(t, 0x0083_5660, 1, 3);
    let pos_set = fk.fake(t, 0x0077_e9a0, 2, 3);
    let pop = t.b.rebase(FPU_POP);
    let (mode, shape, th) = (
        t.a.rebase(0x0130_08d9),
        t.a.rebase(0x0130_08da),
        t.a.rebase(0x00c4_9718),
    );
    for round in 0..96u32 {
        fk.reset(t);
        t.put_u32(elem, if round % 8 == 7 { 0 } else { rng.next() % 4 });
        t.put_u32(pos_set, rng.next());
        t.put(mode, &[(rng.next() % 5) as u8]);
        t.put(shape, &[(rng.next() % 8) as u8]);
        t.put(th, &((rng.next() % 200) as i16).to_le_bytes());
        let chr = t.alloc(0x800);
        for k in 0..0x200 {
            t.put_u32(chr + 4 * k, rng.next());
        }
        t.put(chr + 0x4d4, &[u8::from(rng.next().is_multiple_of(2))]);
        let open = (rng.next() % 90) as i16 - 10;
        t.put(chr + 0x4dc, &open.to_le_bytes());
        let goal = if rng.next().is_multiple_of(4) {
            0
        } else {
            (rng.next() % 90) as i16 - 10
        };
        t.put(chr + 0x4de, &goal.to_le_bytes());
        if rng.next().is_multiple_of(4) {
            t.put(chr + 0x4dc, &goal.to_le_bytes());
        }
        t.put(chr + 0x4e1, &[(rng.next() % 3) as u8]);
        t.put(chr + 0x4e3, &[1 + (rng.next() % 20) as u8]);
        t.put_u32(chr + 0x6fc, rng.next() % 2);
        let model = t.alloc(0x80);
        for k in 0..3 {
            t.put_u32(
                model + 0x44 + 4 * k,
                (((rng.next() % 64) as f32) / 16.0 - 2.0).to_bits(),
            );
        }
        t.put(
            model + 0x42,
            &[
                (rng.next() % 4) as u8,
                u8::from(rng.next().is_multiple_of(2)),
            ],
        );
        t.put_u32(chr + 0x1c4, model);
        let elems = t.alloc(0x160 * 4);
        t.put_u32(chr + 0x328, elems);
        let at = t.a.rebase(0x0083_44f0);
        let ra = t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
        let rb = {
            let mut call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
            let mut fcall = |m: &mut Oracle, f: u32, a: &[u32]| {
                let r = m.call(f, Conv::Cdecl, a).unwrap();
                m.call(pop, Conv::Cdecl, &[]).unwrap();
                r.st0.expect("a float result")
            };
            let mut ci = |_: &mut Oracle, _: u32, _: &[f64]| f32::NAN;
            let mut c = p::NeckCalls {
                call: &mut call,
                fcall: &mut fcall,
                ci: &mut ci,
            };
            p::ch_mouth(&mut t.b, chr, &mut c)
        };
        assert_eq!(ra.eax, rb, "{}: Ch_Mouth {round}", t.name);
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_Mouth {round} memory",
            t.name
        );
    }
}

#[test]
fn round11d_matches_the_original() {
    for mut t in twins() {
        round11d_on(&mut t);
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn round11d_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mo;
    let mut rng = Xorshift(0x0011_d11d);
    let fk = Fakes::new(t);
    let seq = fk.fake(t, 0x0083_7ac0, 1, 2);
    let sys = fk.fake(t, 0x0083_7d00, 2, 3);
    let pop = t.b.rebase(FPU_POP);
    let water = t.a.rebase(0x0130_09d8);
    let f = |rng: &mut Xorshift, lo: i32, hi: i32| {
        let span = u32::try_from(hi - lo).unwrap() * 16;
        ((rng.next() % span) as f32 / 16.0 + lo as f32).to_bits()
    };
    for round in 0..64u32 {
        fk.reset(t);
        t.put_u32(seq, rng.next());
        t.put_u32(sys, rng.next());
        t.put_u32(water, f(&mut rng, -4, 4));
        let chr = t.alloc(0x800);
        for k in 0..0x200 {
            t.put_u32(chr + 4 * k, rng.next());
        }
        t.put_u32(chr + 0x194, rng.next());
        let speed = [0f32, 0.5, 1.0, 2.0][(rng.next() % 4) as usize];
        t.put_u32(chr + 0x154, speed.to_bits());
        t.put_u32(chr + 0x170, f(&mut rng, 0, 2));
        t.put_u32(chr + 0x158, f(&mut rng, -7, 7));
        t.put_u32(chr + 0x168, f(&mut rng, -7, 7));
        t.put_u32(chr + 0x10, f(&mut rng, -6, 6));
        t.put_u32(chr + 0x504, f(&mut rng, 0, 2));
        t.put(chr + 0x736, &[u8::from(rng.next().is_multiple_of(4))]);
        for off in [0x79cu32, 0x7a4, 0x7a8] {
            let m = (rng.next() & 0xffff_0000) | (rng.next() % 0x2000);
            t.put_u32(chr + off, m);
        }
        let at = t.a.rebase(0x0083_5d30);
        t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
        mo::ch_auto_motion(
            &mut t.b,
            chr,
            &mut |m: &mut Oracle, f, a: &[u32]| m.call_cdecl(f, a).unwrap(),
            &mut |m: &mut Oracle, f, a: &[u32]| {
                let r = m.call(f, Conv::Cdecl, a).unwrap();
                m.call(pop, Conv::Cdecl, &[]).unwrap();
                r.st0.expect("a float result")
            },
        );
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_AutoMotion {round}",
            t.name
        );
    }
}

#[test]
fn round11e_matches_the_original() {
    for mut t in twins() {
        round11e_on(&mut t);
    }
}

#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
fn round11e_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0011_e11e);
    let fk = Fakes::new(t);
    let info = fk.fake(t, 0x0064_3db0, 1, 0);
    let f = |rng: &mut Xorshift, lo: i32, hi: i32| {
        let span = u32::try_from(hi - lo).unwrap() * 16;
        ((rng.next() % span) as f32 / 16.0 + lo as f32).to_bits()
    };
    for round in 0..32u32 {
        fk.reset(t);
        let shader = t.alloc(0x100);
        let vi = t.alloc(0x20);
        t.put_u32(shader + 0x94, vi);
        t.put_u32(info, shader);
        let n = rng.next() % 8;
        t.put_u32(vi + 4, n);
        t.put_u32(vi + 8, rng.next() % 12);
        for (off, size) in [(0xcu32, 12u32), (0x14, 16), (0x18, 8), (0x1c, 2)] {
            let buf = t.alloc(usize::try_from(size * 32).unwrap());
            t.put_u32(vi + off, buf);
        }
        let m = t.alloc(0x40);
        for k in 0..16 {
            let v = f(&mut rng, -2, 2);
            t.put_u32(m + 4 * k, v);
        }
        let args: Vec<u32> = [m, rng.next(), rng.next(), rng.next()]
            .into_iter()
            .chain((0..7).map(|_| f(&mut rng, -4, 4)))
            .collect();
        let scratch = t.alloc(p::QUAD_SCRATCH as usize);
        let at = t.a.rebase(0x0083_1d10);
        let ra = t.a.call(at, Conv::Cdecl, &args).unwrap();
        let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
        let rb = p::ch_draw_quad(
            &mut t.b,
            args[0],
            args[1],
            [args[4], args[5]],
            args[6],
            [args[7], args[8], args[9], args[10]],
            scratch,
            &mut |m, f, a| call(m, f, a),
        );
        assert_eq!(ra.eax, rb, "{}: Ch_DrawQuad {round}", t.name);
        let s = Oracle::read(&mut t.b, scratch, p::QUAD_SCRATCH as usize).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_DrawQuad {round} memory",
            t.name
        );
    }
}

#[test]
fn round11f_matches_the_original() {
    for mut t in twins() {
        round11f_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn round11f_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mo;
    let mut rng = Xorshift(0x0011_f11f);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let save = fk.fake(t, 0x0088_c830, 1, 0);
    let reset = fk.fake(t, 0x0082_b080, 2, 1);
    let seq = fk.fake(t, 0x0083_7ac0, 3, 2);
    let fast = fk.fake(t, 0x0083_7860, 4, 1);
    let play = fk.fake(t, 0x0083_9b80, 5, 5);
    let dispose = fk.fake(t, 0x0083_6e20, 6, 3);
    let find = fk.fake(t, 0x0083_6f10, 7, 3);
    let battle = fk.fake(t, 0x0078_0c90, 8, 0);
    let c1 = fk.fake(t, 0x0086_c1c0, 9, 1);
    let c2 = fk.fake(t, 0x0086_5890, 10, 1);
    let c3 = fk.fake(t, 0x0082_a7f0, 11, 1);
    for round in 0..24u32 {
        fk.reset(t);
        for r in [reset, play, dispose, c1, c2, c3] {
            t.put_u32(r, rng.next());
        }
        let data = t.alloc(0x100);
        t.put(data + 0xd2, &[rng.next().to_le_bytes()[0]]);
        t.put_u32(save, data);
        t.put_u32(seq, u32::from(rng.next().is_multiple_of(4)));
        let clip = t.alloc(0x10);
        t.put_u32(clip + 0xc, rng.next());
        t.put_u32(
            fast,
            if rng.next().is_multiple_of(4) {
                0
            } else {
                clip
            },
        );
        t.put_u32(
            find,
            if rng.next().is_multiple_of(2) {
                0
            } else {
                rng.next()
            },
        );
        t.put_u32(battle, u32::from(!rng.next().is_multiple_of(3)));
        let chr = t.alloc(0x800);
        t.put(chr + 0x186, &[(rng.next() % 5).to_le_bytes()[0]]);
        let args = [
            chr,
            rng.next(),
            rng.next(),
            rng.next(),
            rng.next() % 0x100,
            rng.next() % 0x100,
        ];
        t.check(
            &format!("Ch_MotionSetLow2 {round}"),
            0x0083_73e0,
            &args,
            |m| {
                Res::Int(mo::ch_motion_set_low2(m, args, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        let (a, b) = (rng.next(), rng.next());
        t.check(
            &format!("ChN_DisposeSystemMGRP {round}"),
            0x0083_6830,
            &[a, b],
            |m| {
                Res::Int(mo::chn_dispose_system_mgrp(m, a, b, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        t.check(
            &format!("ChN_SystemMgrpLoaded {round}"),
            0x0083_7710,
            &[],
            |m| Res::Int(mo::chn_system_mgrp_loaded(m, &mut |m, f, a| call(m, f, a))),
        );
        t.check(
            &format!("Ch_ReadEventMgrp {round}"),
            0x0083_7670,
            &[a],
            |m| Res::Int(mo::ch_read_event_mgrp(m, a, &mut |m, f, a| call(m, f, a))),
        );
        t.check(
            &format!("Ch_SaveMotionByte {round}"),
            0x0083_03a0,
            &[chr],
            |m| {
                Res::Int(mo::ch_save_motion_byte(m, chr, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        let v = a.to_le_bytes()[0];
        t.check(
            &format!("SetByte_012fb8fd {round}"),
            0x0082_1b20,
            &[u32::from(v)],
            |m| {
                mo::set_byte_012fb8fd(m, v);
                Res::Void
            },
        );
    }
}

#[test]
fn round11g_matches_the_original() {
    for mut t in twins() {
        round11g_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn round11g_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mo;
    let mut rng = Xorshift(0x0011_a11a);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let reset = fk.fake(t, 0x0082_b080, 2, 1);
    let seq = fk.fake(t, 0x0083_7ac0, 3, 2);
    let fast = fk.fake(t, 0x0083_7860, 4, 1);
    let play = fk.fake(t, 0x0083_9b80, 5, 5);
    // SG_DisposeMotionGroup: clears the entry Ch_DisposeAllMGRP picks, then logs
    let table = t.a.rebase(0x0130_11e0);
    let (logger, _) = fk.fake_code(t, 6, 3);
    let mut stub = vec![0xb8];
    stub.extend((table + 4).to_le_bytes());
    stub.extend([
        0x83, 0x78, 0xfc, 0x00, 0x74, 0x06, 0x66, 0x83, 0x38, 0x02, 0x75, 0x0b,
    ]);
    stub.extend([0x83, 0xc0, 0x0c, 0x3d]);
    stub.extend((table + 50 * 12 + 4).to_le_bytes());
    stub.extend([0x7c, 0xea, 0xc3, 0xc7, 0x40, 0xfc, 0, 0, 0, 0, 0xe9]);
    let at = t.a.alloc_code(&[0; 64]).unwrap();
    assert_eq!(t.b.alloc_code(&[0; 64]).unwrap(), at);
    let jmp_from = at + u32::try_from(stub.len()).unwrap() + 4;
    stub.extend(logger.wrapping_sub(jmp_from).to_le_bytes());
    t.put(at, &stub);
    let dispose = t.a.rebase(0x0083_6e20);
    let mut jmp = vec![0xe9];
    jmp.extend(at.wrapping_sub(dispose + 5).to_le_bytes());
    t.put(dispose, &jmp);
    for round in 0..24u32 {
        fk.reset(t);
        t.put_u32(reset, rng.next());
        t.put_u32(play, rng.next());
        t.put_u32(seq, u32::from(rng.next().is_multiple_of(4)));
        let clip = t.alloc(0x10);
        let marks = t.alloc(0x20);
        for k in 0..16 {
            #[allow(clippy::cast_possible_truncation)]
            t.put(
                marks + 2 * k,
                &((rng.next() % 200) as i16 - 20).to_le_bytes(),
            );
        }
        t.put_u32(clip + 8, marks);
        t.put_u32(clip + 0xc, rng.next());
        t.put_u32(
            fast,
            if rng.next().is_multiple_of(4) {
                0
            } else {
                clip
            },
        );
        let chr = t.alloc(0x800);
        let loops = rng.next() % 3;
        let ends = [rng.next() % 16, rng.next() % 16];
        let raw = [
            rng.next() % 0x100,
            (rng.next() % 0x100).wrapping_sub(0x20)
                & if rng.next().is_multiple_of(4) {
                    0
                } else {
                    u32::MAX
                },
        ];
        let args = [chr, rng.next(), rng.next(), loops, raw[0], raw[1]];
        t.check(
            &format!("Chr_PlayMotionInclEnd {round}"),
            0x0083_74c0,
            &args,
            |m| {
                Res::Int(mo::chr_play_motion(
                    m,
                    mo::PlayFrames::InclEnd,
                    args,
                    &mut |m, f, a| call(m, f, a),
                ))
            },
        );
        let args = [chr, rng.next(), rng.next(), loops, ends[0], ends[1]];
        t.check(
            &format!("Chr_PlayMotionMarks {round}"),
            0x0083_7350,
            &args,
            |m| {
                Res::Int(mo::chr_play_motion(
                    m,
                    mo::PlayFrames::Marks,
                    args,
                    &mut |m, f, a| call(m, f, a),
                ))
            },
        );
        let x = rng.next();
        t.check(&format!("Ch_DisposeMGRP {round}"), 0x0083_6ad0, &[x], |m| {
            Res::Int(mo::ch_dispose_mgrp(m, x, &mut |m, f, a| call(m, f, a)))
        });
        for k in 0..50 {
            let e = table + 12 * k;
            t.put_u32(
                e,
                if rng.next().is_multiple_of(3) {
                    0
                } else {
                    rng.next() | 1
                },
            );
            #[allow(clippy::cast_possible_truncation)]
            t.put(e + 4, &((rng.next() % 4) as i16).to_le_bytes());
            t.put(e + 6, &(rng.next().to_le_bytes()[..2]));
            t.put_u32(e + 8, rng.next());
        }
        t.check(
            &format!("Ch_DisposeAllMGRP {round}"),
            0x0083_6a80,
            &[],
            |m| Res::Int(mo::ch_dispose_all_mgrp(m, &mut |m, f, a| call(m, f, a))),
        );
    }
}

#[test]
fn round11h_matches_the_original() {
    for mut t in twins() {
        round11h_on(&mut t);
    }
}

fn round11h_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mo;
    let mut rng = Xorshift(0x0011_b0b0);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    // its first argument is the caller's stack: logged without arguments
    let moved = fk.fake(t, 0x0083_c350, 1, 0);
    let mgrp_move = fk.fake(t, 0x0083_70c0, 2, 2);
    let table = t.a.rebase(0x0130_11e0);
    let ci1 = twin_code(t, &CI1_STUB);
    // Ch_InitCircleTable, through the CRT's own intrinsics
    t.check("Ch_InitCircleTable", 0x0082_e5a0, &[], |m| {
        p::ch_init_circle_table(m, &mut |m: &mut Oracle, f, a: &[f64]| {
            let b = a[0].to_bits();
            #[allow(clippy::cast_possible_truncation)]
            let w = [b as u32, (b >> 32) as u32, f];
            f32::from_bits(m.call(ci1, Conv::Cdecl, &w).unwrap().eax)
        });
        Res::Void
    });
    for round in 0..24u32 {
        fk.reset(t);
        t.put_u32(moved, u32::from(rng.next().is_multiple_of(3)));
        t.put_u32(mgrp_move, rng.next());
        for k in 0..50 {
            let e = table + 12 * k;
            t.put_u32(
                e,
                if rng.next().is_multiple_of(2) {
                    0
                } else {
                    rng.next() | 1
                },
            );
            #[allow(clippy::cast_possible_truncation)]
            t.put(e + 4, &((rng.next() % 3) as i16).to_le_bytes());
        }
        let scratch = t.alloc(4);
        let at = t.a.rebase(0x0083_7050);
        let ra = t.a.call(at, Conv::Cdecl, &[]).unwrap();
        let rb = mo::ch_mem_move_event_mgrp(&mut t.b, scratch, &mut |m, f, a| call(m, f, a));
        assert_eq!(ra.eax, rb, "{}: Ch_MemMoveEventMgrp {round}", t.name);
        let s = Oracle::read(&mut t.b, scratch, 4).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_MemMoveEventMgrp {round} memory",
            t.name
        );
    }
}
