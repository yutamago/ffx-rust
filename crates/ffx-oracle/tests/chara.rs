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
        void!(sy::HISTORY_9E8.set(m, 0))
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
