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
                // bit for bit, NaN payloads too
                assert_eq!(
                    st0.to_bits(),
                    f64::from(v).to_bits(),
                    "{what}: {st0} vs {v}"
                );
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
        let cloth = t.alloc(0x50);
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
    let c2 = fk.fake(t, 0x0086_5890, 10, 2);
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
            &format!("Ch_IsShadowHidden {round}"),
            0x0083_03a0,
            &[chr],
            |m| {
                Res::Int(mo::ch_is_shadow_hidden(m, chr, &mut |m, f, a| {
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

impl Fakes {
    /// A stand-in at new code running `prefix`, then logging `id` and the
    /// arguments at `idxs` (0-based) and returning the word at its result.
    fn fake_code_idxs(&self, t: &mut Twin, prefix: &[u8], id: u32, idxs: &[u8]) -> (u32, u32) {
        let ret = t.alloc(4);
        let mut c = prefix.to_vec();
        c.extend([0x8b, 0x0d]);
        c.extend(self.log.to_le_bytes());
        c.extend([0xc7, 0x01]);
        c.extend(id.to_le_bytes());
        for (j, &k) in (0u8..).zip(idxs) {
            c.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 + 4 * j]);
        }
        c.extend([0x83, 0x05]);
        c.extend(self.log.to_le_bytes());
        c.push(4 * (u8::try_from(idxs.len()).unwrap() + 1));
        c.push(0xa1);
        c.extend(ret.to_le_bytes());
        c.push(0xc3);
        (twin_code(t, &c), ret)
    }
}

#[test]
fn round11i_matches_the_original() {
    for mut t in twins() {
        round11i_on(&mut t);
    }
}

#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
fn round11i_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0011_1111);
    let fk = Fakes::new(t);
    let pop = t.b.rebase(FPU_POP);
    // graphicUpdateShadowMap: writes w and h through its 7th and 8th words
    let (wa, ha) = (t.alloc(4), t.alloc(4));
    let mut pre = vec![0x8b, 0x44, 0x24, 0x1c, 0x8b, 0x0d];
    pre.extend(wa.to_le_bytes());
    pre.extend([0x89, 0x08, 0x8b, 0x44, 0x24, 0x20, 0x8b, 0x0d]);
    pre.extend(ha.to_le_bytes());
    pre.extend([0x89, 0x08]);
    let (map_code, uv) = fk.fake_code_idxs(t, &pre, 1, &[1, 2, 4, 8, 9, 10, 11]);
    // Ch_DrawQuad: its matrix (the caller's stack) copied out, then logged
    let mats = t.alloc(0x40);
    let mut pre = vec![0x8b, 0x44, 0x24, 0x04];
    for k in 0..16u8 {
        pre.extend([0x8b, 0x48, 4 * k, 0x89, 0x0d]);
        pre.extend((mats + 4 * u32::from(k)).to_le_bytes());
    }
    let (quad_code, _) = fk.fake_code_idxs(t, &pre, 2, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    for (sym, code) in [(0x0064_4a00u32, map_code), (0x0083_1d10, quad_code)] {
        let at = t.a.rebase(sym);
        let mut jmp = vec![0xe9];
        jmp.extend(code.wrapping_sub(at + 5).to_le_bytes());
        t.put(at, &jmp);
    }
    let ci1 = twin_code(t, &CI1_STUB);
    let f = |rng: &mut Xorshift, lo: i32, hi: i32| {
        let span = u32::try_from(hi - lo).unwrap() * 16;
        ((rng.next() % span) as f32 / 16.0 + lo as f32).to_bits()
    };
    for round in 0..24u32 {
        fk.reset(t);
        let uvs = t.alloc(16);
        for k in 0..4 {
            let v = f(&mut rng, 0, 1);
            t.put_u32(uvs + 4 * k, v);
        }
        t.put_u32(uv, uvs);
        t.put_u32(wa, f(&mut rng, 0, 4));
        t.put_u32(ha, f(&mut rng, 0, 4));
        let chr = t.alloc(0x900);
        for k in 0..0x240 {
            let v = f(&mut rng, -4, 4);
            t.put_u32(chr + 4 * k, v);
        }
        let id: u16 = [7, 0x6b, 0x10, 0x1001][usize::try_from(rng.next() % 4).unwrap()];
        t.put(chr, &id.to_le_bytes());
        let p3 = rng.next();
        let scratch = t.alloc(p::SHADOW_QUAD_SCRATCH as usize);
        let at = t.a.rebase(0x0082_e3e0);
        let ra = t.a.call(at, Conv::Cdecl, &[chr, rng.next(), p3]).unwrap();
        let rb = {
            let mut call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
            let mut fcall = |m: &mut Oracle, f: u32, a: &[u32]| {
                let r = m.call(f, Conv::Cdecl, a).unwrap();
                m.call(pop, Conv::Cdecl, &[]).unwrap();
                r.st0.expect("a float result")
            };
            let mut ci = |m: &mut Oracle, f: u32, a: &[f64]| {
                let b = a[0].to_bits();
                #[allow(clippy::cast_possible_truncation)]
                let w = [b as u32, (b >> 32) as u32, f];
                f32::from_bits(m.call(ci1, Conv::Cdecl, &w).unwrap().eax)
            };
            let mut c = p::NeckCalls {
                call: &mut call,
                fcall: &mut fcall,
                ci: &mut ci,
            };
            p::ch_draw_shadow_quad(&mut t.b, chr, p3, scratch, &mut c)
        };
        assert_eq!(ra.eax, rb, "{}: Ch_DrawShadowQuad {round}", t.name);
        let s = Oracle::read(&mut t.b, scratch, p::SHADOW_QUAD_SCRATCH as usize).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_DrawShadowQuad {round} memory",
            t.name
        );
    }
}

/// Which skeleton matrices `graphicCharacterInstanceSetSkeletonMatrices`
/// (`0x0063be60`) reads from `Ch_CalcElement`'s stack buffer, against the
/// ones it computes (each element's `+4`): every index the instance's
/// meshes remap through must be a computed one, and a poisoned stack
/// must not reach the instance's matrices.
#[test]
fn ch_calc_element_reads_only_computed_matrices() {
    const POISON: u32 = 0xffba_dbad;
    for mut t in twins() {
        if t.name == "synthetic" {
            continue;
        }
        let mut rng = Xorshift(0x0012_0012);
        let mut checked = 0;
        for chr in live_characters(&mut t, &mut rng) {
            let hdr: u32 = rd(&t.a, chr + 0x1c0);
            if hdr == 0 || rd::<u16>(&t.a, hdr + 8) == 0 {
                continue;
            }
            let n = u32::from(rd::<u16>(&t.a, hdr + 10));
            let elems: u32 = rd(&t.a, chr + 0x328);
            let computed: std::collections::BTreeSet<u32> = (0..n)
                .map(|k| rd::<u32>(&t.a, elems + 0x160 * k + 4))
                .collect();
            for off in [0x830u32, 0x838] {
                let inst: u32 = rd(&t.a, chr + off);
                if inst == 0 {
                    continue;
                }
                let meshes = rd::<u32>(&t.a, inst) & 0x7fff_ffff;
                let remaps: u32 = rd(&t.a, rd::<u32>(&t.a, inst + 0x10) + 0x14);
                for i in 0..meshes {
                    let mesh: u32 = rd(&t.a, rd::<u32>(&t.a, inst + 4) + 4 * i);
                    let bones = rd::<u32>(&t.a, rd::<u32>(&t.a, mesh) + 0x18) & 0x7fff_ffff;
                    let remap: u32 = rd(&t.a, remaps + 4 * i);
                    for j in 0..bones {
                        let b: u32 = rd(&t.a, remap + 4 * j);
                        let skipped = rd::<u16>(&t.a, chr) == 0x206a && b >= 0x42;
                        assert!(
                            skipped || computed.contains(&b),
                            "chr {chr:#x}: instance {inst:#x} mesh {i} reads matrix {b}, not computed ({computed:?})"
                        );
                    }
                }
            }
            // and with a poisoned stack
            let stack = t.a.stack_range();
            let words = vec![POISON; ((stack.end - stack.start) / 4) as usize];
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            t.a.write(stack.start, &bytes).unwrap();
            let at = t.a.rebase(0x0083_2760);
            t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
            for off in [0x830u32, 0x838] {
                let inst: u32 = rd(&t.a, chr + off);
                if inst == 0 {
                    continue;
                }
                let meshes = rd::<u32>(&t.a, inst) & 0x7fff_ffff;
                for i in 0..meshes {
                    let mesh: u32 = rd(&t.a, rd::<u32>(&t.a, inst + 4) + 4 * i);
                    let bones = rd::<u32>(&t.a, rd::<u32>(&t.a, mesh) + 0x18) & 0x7fff_ffff;
                    let dest: u32 = rd(&t.a, mesh + 0xc);
                    for w in 0..16 * bones {
                        assert_ne!(
                            rd::<u32>(&t.a, dest + 4 * w),
                            POISON,
                            "chr {chr:#x}: poison in mesh {i}"
                        );
                    }
                }
            }
            checked += 1;
        }
        eprintln!("{}: {checked} characters checked", t.name);
    }
}

#[test]
fn round12_matches_the_original() {
    for mut t in twins() {
        round12_on(&mut t);
    }
}

/// `Ch_CalcElement` on made-up characters. Both sides' skeleton buffers
/// start poisoned (the original's in its stack), so the upload stand-in's
/// copy compares exactly the entries the original computes.
#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]
fn round12_on(t: &mut Twin) {
    const POISON: u32 = 0xffba_dbad;
    let mut rng = Xorshift(0x0012_1212);
    let fk = Fakes::new(t);
    let pop = t.b.rebase(FPU_POP);
    let dest = t.alloc(0x8000);
    // the upload: copies the 512 matrices out, then logs id and instance
    let mut pre = vec![0x56, 0x57, 0x8b, 0x74, 0x24, 0x14, 0xbf];
    pre.extend(dest.to_le_bytes());
    pre.extend([0xb9, 0x00, 0x20, 0x00, 0x00, 0xf3, 0xa5, 0x5f, 0x5e]);
    let (upload, _) = fk.fake_code_idxs(t, &pre, 1, &[0, 1]);
    let (restore, _) = fk.fake_code_idxs(t, &[], 2, &[0, 1]);
    for (sym, code) in [(0x0063_be60u32, upload), (0x0063_bfc0, restore)] {
        let at = t.a.rebase(sym);
        let mut jmp = vec![0xe9];
        jmp.extend(code.wrapping_sub(at + 5).to_le_bytes());
        t.put(at, &jmp);
    }
    let (skip, restore66) = (t.a.rebase(0x012f_b7d8), t.a.rebase(0x0130_08e0));
    let f = |rng: &mut Xorshift, lo: i32, hi: i32| {
        let span = u32::try_from(hi - lo).unwrap() * 16;
        ((rng.next() % span) as f32 / 16.0 + lo as f32).to_bits()
    };
    let fill = |t: &mut Twin, rng: &mut Xorshift, at: u32, words: u32, lo: i32, hi: i32| {
        for k in 0..words {
            let v = f(rng, lo, hi);
            t.put_u32(at + 4 * k, v);
        }
    };
    let poison: Vec<u8> = std::iter::repeat_n(POISON.to_le_bytes(), 0xc11c / 4)
        .flatten()
        .collect();
    for round in 0..32u32 {
        fk.reset(t);
        t.put_u32(skip, u32::from(round % 8 == 5));
        t.put_u32(restore66, rng.next() % 2);
        let model = t.alloc(0x80);
        fill(t, &mut rng, model, 0x20, 1, 3);
        let chr = t.alloc(0x900);
        fill(t, &mut rng, chr, 0x240, -2, 2);
        t.put(
            chr,
            &(if rng.next().is_multiple_of(3) {
                0x66u16
            } else {
                0x12
            })
            .to_le_bytes(),
        );
        t.put_u32(chr + 0x1c4, model);
        t.put_u32(chr + 0x110, f(&mut rng, 1, 3));
        for off in [0x5cu32, 0x60, 0x64] {
            t.put_u32(chr + off, f(&mut rng, 1, 3));
        }
        if rng.next().is_multiple_of(2) {
            t.put_u32(chr + 0x160, 0);
        }
        // the world matrix's source
        let kind = round % 3;
        t.put_u32(chr + 0x19c, 0);
        t.put_u32(chr + 0x10c, 0);
        if kind == 1 {
            let m = t.alloc(0x40);
            fill(t, &mut rng, m, 16, -2, 2);
            t.put_u32(chr + 0x10c, m);
        } else if kind == 2 {
            let parent = t.alloc(0x400);
            fill(t, &mut rng, parent, 0x100, -2, 2);
            let pm = t.alloc(0x40);
            fill(t, &mut rng, pm, 0x10, 1, 3);
            t.put_u32(parent + 0x1c4, pm);
            let pe = t.alloc(0x160 * 2);
            fill(t, &mut rng, pe, 0xb0, -2, 2);
            t.put_u32(parent + 0x328, pe);
            t.put_u32(chr + 0x19c, parent);
            t.put_u32(chr + 0x1a0, rng.next() % 2);
        }
        // elements
        let n = 1 + rng.next() % 6;
        let hdr = t.alloc(0x10);
        t.put(
            hdr + 8,
            &(u16::from(!rng.next().is_multiple_of(4))).to_le_bytes(),
        );
        #[allow(clippy::cast_possible_truncation)]
        t.put(hdr + 0xa, &(n as u16).to_le_bytes());
        t.put_u32(chr + 0x1c0, hdr);
        let elems = t.alloc(0x160 * 6);
        t.put_u32(chr + 0x328, elems);
        for k in 0..n {
            let e = elems + 0x160 * k;
            fill(t, &mut rng, e, 0x58, -2, 2);
            let bone = t.alloc(0xc8);
            fill(t, &mut rng, bone, 0x32, -2, 2);
            // a bone matrix worth inverting: the unit plus small terms
            for d in 0..4 {
                t.put_u32(bone + 0x88 + 0x14 * d, f(&mut rng, 2, 4));
            }
            t.put_u32(e, bone);
            t.put_u32(e + 4, (k * 37 + rng.next() % 5) % 512);
            let given = if rng.next().is_multiple_of(3) {
                let m = t.alloc(0x40);
                fill(t, &mut rng, m, 16, -2, 2);
                m
            } else {
                0
            };
            t.put_u32(e + 0xc8, given);
            let target = if rng.next().is_multiple_of(2) {
                let m = t.alloc(0x40);
                fill(t, &mut rng, m, 16, -2, 2);
                m
            } else {
                0
            };
            t.put_u32(e + 0xcc, target);
            t.put_u32(
                e + 0xe0,
                [0f32, 1.0, 0.5][usize::try_from(rng.next() % 3).unwrap()].to_bits(),
            );
        }
        // poison both skeleton buffers
        let stack = t.a.stack_range();
        let words = vec![POISON; ((stack.end - stack.start) / 4) as usize];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        t.a.write(stack.start, &bytes).unwrap();
        let scratch = t.alloc(p::CALC_ELEMENT_SCRATCH as usize);
        t.b.write(scratch, &poison).unwrap();
        t.a.write(scratch, &poison).unwrap();
        let at = t.a.rebase(0x0083_2760);
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
            p::ch_calc_element(&mut t.b, chr, scratch, &mut c);
        }
        // the frame's other temporaries are the original's stack
        let s = Oracle::read(&mut t.b, scratch, p::CALC_ELEMENT_SCRATCH as usize).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_CalcElement {round}",
            t.name
        );
    }
}

/// `Ch_CalcElement` on the snapshots' own characters, with the real
/// renderer upload (both skeleton buffers poisoned, as in `round12`).
#[test]
fn round12b_matches_the_original() {
    const POISON: u32 = 0xffba_dbad;
    for mut t in twins() {
        if t.name == "synthetic" {
            continue;
        }
        let pop = t.b.rebase(FPU_POP);
        let poison: Vec<u8> = std::iter::repeat_n(POISON.to_le_bytes(), 0xc11c / 4)
            .flatten()
            .collect();
        let mut rng = Xorshift(0x0012_b12b);
        let mut chrs = live_characters(&mut t, &mut rng);
        // the last is the made-up one, whose pointers lead nowhere
        chrs.pop();
        eprintln!("{}: {} characters", t.name, chrs.len());
        for chr in chrs {
            if rd::<u32>(&t.a, chr + 0x1c0) == 0 {
                continue;
            }
            let stack = t.a.stack_range();
            let words = vec![POISON; ((stack.end - stack.start) / 4) as usize];
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            t.a.write(stack.start, &bytes).unwrap();
            let scratch = t.alloc(p::CALC_ELEMENT_SCRATCH as usize);
            t.b.write(scratch, &poison).unwrap();
            let at = t.a.rebase(0x0083_2760);
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
                p::ch_calc_element(&mut t.b, chr, scratch, &mut c);
            }
            let s = Oracle::read(&mut t.b, scratch, p::CALC_ELEMENT_SCRATCH as usize).unwrap();
            t.a.write(scratch, &s).unwrap();
            assert_eq!(
                t.a.diff(&mut t.b).unwrap(),
                [],
                "{}: Ch_CalcElement {chr:#x}",
                t.name
            );
        }
    }
}

#[test]
fn round12c_matches_the_original() {
    for mut t in twins() {
        round12c_on(&mut t);
    }
}

#[allow(clippy::too_many_lines)]
fn round12c_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0012_c12c);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    // Sg_MemMoveWork: a non-zero result also sets *moved; its pointers are
    // partly the caller's stack, so only the call is logged
    let ret = t.alloc(4);
    let mut pre = vec![0xa1];
    pre.extend(ret.to_le_bytes());
    pre.extend([
        0x85, 0xc0, 0x74, 0x0a, 0x8b, 0x4c, 0x24, 0x08, 0xc7, 0x01, 1, 0, 0, 0,
    ]);
    let (mw_code, mw_ret) = fk.fake_code_idxs(t, &pre, 1, &[]);
    let at = t.a.rebase(0x0083_c350);
    let mut jmp = vec![0xe9];
    jmp.extend(mw_code.wrapping_sub(at + 5).to_le_bytes());
    t.put(at, &jmp);
    let mut rets = vec![];
    for (sym, id, n) in [
        (0x0083_3770u32, 2u32, 2u8),
        (0x0082_6980, 3, 2),
        (0x0083_9890, 4, 2),
        (0x0083_cdd0, 5, 1),
        (0x0077_e9f0, 6, 1),
        (0x0077_ea10, 7, 2),
        (0x0083_ca20, 8, 2),
        (0x0083_c940, 9, 2),
        (0x0080_3610, 10, 0),
        (0x0083_7050, 11, 0),
        (0x0083_1b70, 12, 0),
    ] {
        rets.push(fk.fake(t, sym, id, n));
    }
    let trigger = t.a.rebase(0x012f_bda0);
    let (count, array) = (t.a.rebase(0x023c_44e0), t.a.rebase(0x023c_44e4));
    for round in 0..32u32 {
        fk.reset(t);
        for &r in &rets {
            t.put_u32(r, rng.next());
        }
        t.put_u32(rets[9], u32::from(rng.next().is_multiple_of(3)));
        let v = if rng.next().is_multiple_of(2) {
            0
        } else {
            rng.next() | 1
        };
        t.put_u32(mw_ret, v);
        t.put_u32(ret, v);
        t.put(trigger, &[u8::from(round % 8 != 7)]);
        let n = 1 + rng.next() % 3;
        let chrs = t.alloc(0x880 * 3);
        t.put_u32(count, n);
        t.put_u32(array, chrs);
        for k in 0..n {
            let chr = chrs + 0x880 * k;
            for w in 0..0x220 {
                t.put_u32(chr + 4 * w, rng.next());
            }
            t.put(chr + 2, &[u8::from(!rng.next().is_multiple_of(4))]);
            t.put_u32(
                chr + 0x198,
                rng.next()
                    & 0x0c00
                    & if rng.next().is_multiple_of(3) {
                        u32::MAX
                    } else {
                        0
                    },
            );
            let hdr = t.alloc(8);
            #[allow(clippy::cast_possible_truncation)]
            t.put(hdr + 6, &((rng.next() % 3) as u16).to_le_bytes());
            t.put_u32(chr + 0x1c0, hdr);
            let models = t.alloc(0x38 * 3);
            t.put_u32(
                chr + 0x1bc,
                if rng.next().is_multiple_of(4) {
                    0
                } else {
                    models
                },
            );
            for off in [0x6fcu32, 0x704] {
                t.put_u32(chr + off, rng.next() % 2);
            }
        }
        let scratch = t.alloc(0xc);
        let at = t.a.rebase(0x0082_6fa0);
        let ra = t.a.call(at, Conv::Cdecl, &[]).unwrap();
        let rb = c::ch_mem_move(&mut t.b, scratch, &mut |m, f, a| call(m, f, a));
        assert_eq!(ra.eax, rb, "{}: Ch_MemMove {round}", t.name);
        let s = Oracle::read(&mut t.b, scratch, 0xc).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_MemMove {round} memory",
            t.name
        );
    }
}

#[test]
fn round12d_matches_the_original() {
    for mut t in twins() {
        round12d_on(&mut t);
    }
}

fn round12d_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0012_d12d);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let free = fk.fake(t, 0x0068_7010, 1, 1);
    let dispose = fk.fake(t, 0x0083_6830, 2, 2);
    let unuse = fk.fake(t, 0x0084_0d70, 3, 2);
    let tex = fk.fake(t, 0x0083_0330, 4, 1);
    let norm = fk.fake(t, 0x0083_4090, 5, 1);
    let circle = fk.fake(t, 0x0082_e5a0, 6, 0);
    // Ch_MemMove: moves on the first few calls
    let left = t.alloc(4);
    let mut c = vec![0x31, 0xc0, 0x83, 0x2d];
    c.extend(left.to_le_bytes());
    c.extend([0x01, 0x78, 0x01, 0x40, 0xc3]);
    // xor eax,eax; sub dword [left],1; js +1; inc eax; ret
    let mm = twin_code(t, &c);
    let at = t.a.rebase(0x0082_6fa0);
    let mut jmp = vec![0xe9];
    jmp.extend(mm.wrapping_sub(at + 5).to_le_bytes());
    t.put(at, &jmp);
    for round in 0..24u32 {
        fk.reset(t);
        for r in [free, dispose, unuse, tex, norm, circle] {
            t.put_u32(r, rng.next());
        }
        let blk = t.alloc(0x130);
        for k in 0..0x4c {
            t.put_u32(blk + 4 * k, rng.next());
        }
        if round % 6 == 5 {
            t.put_u32(blk, u32::MAX);
        }
        t.check(
            &format!("Ch_BlkDispose {round}"),
            0x0082_58d0,
            &[blk],
            |m| Res::Int(c::ch_blk_dispose(m, blk, &mut |m, f, a| call(m, f, a))),
        );
        let chr = t.alloc(4);
        t.put(
            chr,
            &[
                [3u8, 0x67, 4, 0][usize::try_from(rng.next() % 4).unwrap()],
                0,
            ],
        );
        t.check(
            &format!("Ch_IsShadowExceptionId {round}"),
            0x0082_e200,
            &[chr],
            |m| Res::Int(c::ch_is_shadow_exception_id(m, chr)),
        );
        t.put_u32(left, rng.next() % 4);
        t.check(&format!("Ch_MemMove_Over {round}"), 0x0082_7260, &[], |m| {
            Res::Int(c::ch_mem_move_over(m, &mut |m, f, a| call(m, f, a)))
        });
        t.check(&format!("Ch_DrawInitAll {round}"), 0x0082_ed30, &[], |m| {
            c::ch_draw_init_all(m, &mut |m, f, a| call(m, f, a));
            Res::Void
        });
    }
}

#[test]
fn round12e_matches_the_original() {
    for mut t in twins() {
        round12e_on(&mut t);
    }
}

fn round12e_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0012_e12e);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let spad = fk.fake(t, 0x0084_3220, 1, 0);
    let hidden = fk.fake(t, 0x0083_03a0, 2, 1);
    let not_exc = fk.fake(t, 0x0083_1780, 3, 1);
    let quad = fk.fake(t, 0x0082_e3e0, 4, 3);
    let dir = fk.fake(t, 0x0064_2fb0, 5, 2);
    let (mode, dirs) = (t.a.rebase(0x0130_08bd), t.a.rebase(0x0130_08c4));
    let (count, array) = (t.a.rebase(0x023c_44e0), t.a.rebase(0x023c_44e4));
    for round in 0..32u32 {
        fk.reset(t);
        for r in [spad, quad, dir] {
            t.put_u32(r, rng.next());
        }
        t.put_u32(hidden, u32::from(rng.next().is_multiple_of(5)));
        t.put_u32(not_exc, u32::from(!rng.next().is_multiple_of(5)));
        t.put(mode, &[(rng.next() % 5).to_le_bytes()[0]]);
        t.put_u32(dirs, rng.next() % 0x2000_0000);
        t.put_u32(dirs + 4, rng.next());
        let n = rng.next() % 4;
        let chrs = t.alloc(0x880 * 3 + 4);
        t.put_u32(count, n);
        t.put_u32(array, chrs);
        for k in 0..n.max(1) {
            let chr = chrs + 0x880 * k;
            t.put(chr, &rng.next().to_le_bytes());
            t.put(chr + 0x183, &[(rng.next() % 5).to_le_bytes()[0]]);
            t.put(chr + 0x186, &[(rng.next() % 5).to_le_bytes()[0]]);
            t.put(
                chr + 0x198,
                &[if rng.next().is_multiple_of(4) {
                    0
                } else {
                    0x10
                }],
            );
            t.put(chr + 2, &[u8::from(!rng.next().is_multiple_of(4))]);
        }
        let on = rng.next() % 3;
        t.check(&format!("Ch_DrawShadow {round}"), 0x0083_0180, &[on], |m| {
            c::ch_draw_shadow(m, on, &mut |m, f, a| call(m, f, a));
            Res::Void
        });
        t.check(
            &format!("Ch_DrawSingleShadow {round}"),
            0x0083_0240,
            &[chrs],
            |m| {
                c::ch_draw_single_shadow(m, chrs, &mut |m, f, a| call(m, f, a));
                Res::Void
            },
        );
    }
}

#[test]
fn round12f_matches_the_original() {
    for mut t in twins() {
        round12f_on(&mut t);
    }
}

fn round12f_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0012_f12f);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let malloc = fk.fake_alloc(t, 0x0068_7020, 1, 0x100);
    let size = fk.fake(t, 0x0083_c050, 2, 1);
    let memset = fk.fake(t, 0x0094_964c, 3, 3);
    let name = fk.fake(t, 0x0083_c580, 4, 3);
    let remap = fk.fake(t, 0x0083_cdd0, 5, 1);
    let set = fk.fake(t, 0x0077_c950, 6, 2);
    for round in 0..24u32 {
        fk.reset(t);
        t.put_u32(malloc, 0x1000_0000 + 0x10000 * round);
        for r in [size, memset, name, remap, set] {
            t.put_u32(r, rng.next());
        }
        let chr = t.alloc(0x800);
        let blk = t.alloc(0x20);
        let data = t.alloc(0x20);
        for k in 0..8 {
            t.put_u32(data + 4 * k, rng.next());
        }
        let v: u16 = [0x1028, 0x1029, 0x1126, 0x0800][usize::try_from(rng.next() % 4).unwrap()];
        t.put(data + 0x10, &v.to_le_bytes());
        t.put_u32(blk + 0x14, if round % 6 == 5 { 0 } else { data });
        t.put_u32(chr + 0x1c4, blk);
        t.put_u32(chr + 0x6fc, rng.next() % 2);
        t.check(
            &format!("Ch_TextureSetup {round}"),
            0x0083_d0a0,
            &[chr],
            |m| Res::Int(tx::ch_texture_setup(m, chr, &mut |m, f, a| call(m, f, a))),
        );
    }
}

#[test]
fn round12g_matches_the_original() {
    for mut t in twins() {
        round12g_on(&mut t);
    }
}

#[allow(clippy::cast_precision_loss)]
fn round12g_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0012_a12a);
    let fk = Fakes::new(t);
    let hash = fk.fake(t, 0x0064_1da0, 1, 0);
    let cam = t.alloc(16);
    let stub = copy_stub(cam, None, 2);
    fake_with(t, 0x007b_d470, &stub);
    let visible = fk.fake(t, 0x0063_b230, 3, 1);
    let split = fk.fake(t, 0x0064_1cc0, 4, 0);
    let valefor = fk.fake(t, 0x0078_7340, 5, 0);
    let ci1 = twin_code(t, &CI1_STUB);
    let f = |rng: &mut Xorshift, lo: i32, hi: i32| {
        let span = u32::try_from(hi - lo).unwrap() * 4;
        ((rng.next() % span) as f32 / 4.0 + lo as f32).to_bits()
    };
    for round in 0..48u32 {
        fk.reset(t);
        t.put_u32(
            hash,
            if rng.next().is_multiple_of(2) {
                0x24f9
            } else {
                7
            },
        );
        t.put_u32(visible, u32::from(!rng.next().is_multiple_of(4)));
        t.put_u32(split, u32::from(rng.next().is_multiple_of(4)));
        t.put_u32(valefor, rng.next() & 0x84);
        for k in 0..4 {
            let v = f(&mut rng, -20, 20);
            t.put_u32(cam + 4 * k, v);
        }
        let chr = t.alloc(0x900);
        for k in 0..4 {
            let v = f(&mut rng, -20, 20);
            t.put_u32(chr + 0x694 + 4 * k, v);
        }
        t.put(
            chr + 0x180,
            &[[0u8, 0x28, 0x01][usize::try_from(rng.next() % 3).unwrap()]],
        );
        t.put_u32(chr + 0x194, rng.next());
        t.put_u32(chr + 0x174, f(&mut rng, 0, 40));
        t.put_u32(chr + 0x830, rng.next());
        let scratch = t.alloc(p::CULL_SCRATCH as usize);
        let at = t.a.rebase(0x0082_e220);
        let ra = t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
        let rb = {
            let mut call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
            let mut fcall = |_: &mut Oracle, _: u32, _: &[u32]| f64::NAN;
            let mut ci = |m: &mut Oracle, f: u32, a: &[f64]| {
                let b = a[0].to_bits();
                #[allow(clippy::cast_possible_truncation)]
                let w = [b as u32, (b >> 32) as u32, f];
                f32::from_bits(m.call(ci1, Conv::Cdecl, &w).unwrap().eax)
            };
            let mut c = p::NeckCalls {
                call: &mut call,
                fcall: &mut fcall,
                ci: &mut ci,
            };
            p::ch_cull_by_distance(&mut t.b, chr, scratch, &mut c)
        };
        assert_eq!(ra.eax, rb, "{}: Ch_CullByDistance {round}", t.name);
        let s = Oracle::read(&mut t.b, scratch, p::CULL_SCRATCH as usize).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_CullByDistance {round} memory",
            t.name
        );
    }
}

#[test]
fn round12h_matches_the_original() {
    for mut t in twins() {
        round12h_on(&mut t);
    }
}

fn round12h_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0012_b0b0);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let clear = fk.fake(t, 0x0062_f4e0, 1, 2);
    let arc = fk.fake(t, 0x0082_5ee0, 2, 2);
    let size = fk.fake(t, 0x0083_c050, 3, 1);
    let name = fk.fake(t, 0x0083_8060, 4, 1);
    let heap = fk.fake_alloc(t, 0x0068_7020, 5, 0x40);
    let set_str = fk.fake(t, 0x0083_c580, 6, 3);
    let set_type = fk.fake(t, 0x0083_c590, 7, 2);
    let copy = fk.fake(t, 0x0094_9256, 8, 3);
    let blks = t.a.rebase(0x023c_8d00);
    for round in 0..24u32 {
        fk.reset(t);
        for r in [clear, arc, set_str, set_type, copy] {
            t.put_u32(r, rng.next());
        }
        t.put_u32(size, rng.next() % 0x1000);
        let s = t.alloc(16);
        t.put(s, format!("c{:03}\0", rng.next() % 1000).as_bytes());
        t.put_u32(name, s);
        t.put_u32(heap, 0x1100_0000 + 0x1000 * round);
        let free = rng.next() % 40;
        for k in 0..40 {
            let b = blks + 0x12c * k;
            t.put_u32(b, if k == free { u32::MAX } else { k });
        }
        let b = blks + 0x12c * free;
        t.put_u32(b + 0x70, rng.next());
        t.put_u32(
            b + 0x120,
            if rng.next().is_multiple_of(3) {
                0
            } else {
                0x2000_0000 + rng.next() % 0x100
            },
        );
        let (id, data) = (rng.next() % 0x10000, 0x2000_0100 + rng.next() % 0x100);
        t.check(
            &format!("Ch_BlkAllocate {round}"),
            0x0082_5760,
            &[id, data],
            |m| {
                Res::Int(h::ch_blk_allocate(m, id, data, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
    }
}

#[test]
fn round13_matches_the_original() {
    for mut t in twins() {
        round13_on(&mut t);
    }
}

/// `Ch_GetCharID`: its digit slot is the caller's ECX (bytes the name does
/// not reach, and the 4th, undefined); the original runs with a non-digit
/// ECX, the only case with a defined result.
fn round13_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0013_0013);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let index = fk.fake(t, 0x0082_a640, 1, 1);
    let (lookup, name) = fk.fake_code(t, 2, 1);
    let slot = t.a.rebase(0x0231_0c98);
    t.put_u32(slot, lookup);
    // the original entered with a non-digit ECX: mov ecx, 'AAAA'; jmp
    let target = t.a.rebase(0x0082_a080);
    let at = t.a.alloc_code(&[0; 16]).unwrap();
    assert_eq!(t.b.alloc_code(&[0; 16]).unwrap(), at);
    let mut c = vec![0xb9, 0x41, 0x41, 0x41, 0x41, 0xe9];
    c.extend(target.wrapping_sub(at + 10).to_le_bytes());
    t.put(at, &c);
    for round in 0..32u32 {
        fk.reset(t);
        t.put_u32(index, rng.next() % 64);
        let s = t.alloc(32);
        let len = usize::try_from(rng.next() % 12).unwrap();
        let text: Vec<u8> = (0..len)
            .map(|_| {
                let r = usize::try_from(rng.next() % 14).unwrap();
                b"0123456789c_.-"[r]
            })
            .chain([0])
            .collect();
        t.put(s, &text);
        t.put_u32(name, if round % 8 == 7 { 0 } else { s });
        let scratch = t.alloc(4);
        let id = rng.next();
        let ra = t.a.call(at, Conv::Cdecl, &[id]).unwrap();
        let rb = sy::ch_get_char_id(&mut t.b, id, scratch, &mut |m, f, a| call(m, f, a));
        assert_eq!(ra.eax, rb, "{}: Ch_GetCharID {round} {text:?}", t.name);
        let sc = Oracle::read(&mut t.b, scratch, 4).unwrap();
        t.a.write(scratch, &sc).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_GetCharID {round} memory",
            t.name
        );
    }
}

#[test]
fn round13b_matches_the_original() {
    for mut t in twins() {
        round13b_on(&mut t);
    }
}

fn round13b_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mo;
    let mut rng = Xorshift(0x0013_b13b);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let find = fk.fake(t, 0x0083_6f10, 1, 3);
    let yet = fk.fake(t, 0x0083_ffe0, 2, 2);
    let read = fk.fake(t, 0x0082_a4c0, 3, 2);
    let sync = fk.fake(t, 0x0082_a5e0, 4, 1);
    let exist = fk.fake(t, 0x0084_06f0, 5, 2);
    let used = fk.fake(t, 0x0084_0dd0, 6, 2);
    let add = fk.fake(t, 0x0083_6c90, 7, 4);
    let loaded = fk.fake(t, 0x0083_7710, 8, 0);
    let not_yet = fk.fake(t, 0x0084_05f0, 9, 3);
    let dispose = fk.fake(t, 0x0082_58d0, 10, 1);
    let blks = t.a.rebase(0x023c_8d00);
    for round in 0..32u32 {
        fk.reset(t);
        for r in [sync, exist, used, add, not_yet, dispose] {
            t.put_u32(r, rng.next());
        }
        t.put_u32(
            find,
            if rng.next().is_multiple_of(3) {
                rng.next()
            } else {
                0
            },
        );
        t.put_u32(yet, rng.next() % 4);
        t.put_u32(read, u32::from(rng.next().is_multiple_of(3)));
        t.put_u32(loaded, u32::from(rng.next().is_multiple_of(2)));
        let (a, b) = (rng.next() % 0x10000, rng.next() % 0x100);
        t.check(
            &format!("ChN_ReadSystemMGRP {round}"),
            0x0083_6870,
            &[a, b],
            |m| {
                Res::Int(mo::chn_read_system_mgrp(m, a, b, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        let data = if rng.next().is_multiple_of(4) {
            0
        } else {
            rng.next()
        };
        t.check(
            &format!("ChN_AddEventMgrp {round}"),
            0x0083_7630,
            &[data, b],
            |m| {
                Res::Int(mo::chn_add_event_mgrp(m, data, b, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        t.check(
            &format!("ChN_AddSystemMgrp {round}"),
            0x0083_7750,
            &[data],
            |m| {
                Res::Int(mo::chn_add_system_mgrp(m, data, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        for k in 0..40 {
            t.put_u32(
                blks + 0x12c * k,
                if rng.next().is_multiple_of(2) {
                    u32::MAX
                } else {
                    k
                },
            );
        }
        t.check(
            &format!("Ch_BlkDisposeAll {round}"),
            0x0082_59c0,
            &[],
            |m| {
                c::ch_blk_dispose_all(m, &mut |m, f, a| call(m, f, a));
                Res::Void
            },
        );
    }
}

#[test]
fn round14_matches_the_original() {
    for mut t in twins() {
        round14_on(&mut t);
    }
}

/// `Sg_IdWalk` over a made-up strip of triangles (a 4 x 4 grid of
/// vertices, two triangles per cell, neighbours across shared edges).
#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
#[allow(clippy::many_single_char_names)]
fn round14_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0014_0014);
    let pop = t.b.rebase(FPU_POP);
    let fk = Fakes::new(t);
    let matches = fk.fake(t, 0x0083_e4e0, 1, 2);
    let normal = t.alloc(16);
    let stub = copy_stub(normal, None, 1);
    fake_with(t, 0x0083_e130, &stub);
    // Sg_IdGetHeight: fld dword [h]; ret
    let h = t.alloc(4);
    let mut c = vec![0xd9, 0x05];
    c.extend(h.to_le_bytes());
    c.push(0xc3);
    fake_with(t, 0x0083_e040, &c);
    let verts = t.alloc(8 * 16);
    for j in 0..4i16 {
        for i in 0..4i16 {
            let v = verts + 8 * u32::try_from(4 * j + i).unwrap();
            t.put(v, &(i * 100 - 150).to_le_bytes());
            t.put(v + 2, &((rng.next() % 50) as i16).to_le_bytes());
            t.put(v + 4, &(j * 100 - 150).to_le_bytes());
        }
    }
    // cell (i, j): triangle 2c = (a, b, d), 2c + 1 = (a, d, e) with
    // a = (i, j), b = (i + 1, j), d = (i + 1, j + 1), e = (i, j + 1)
    let tris = t.alloc(16 * 18);
    let tri = |i: i16, j: i16, k: i16| {
        if (0..3).contains(&i) && (0..3).contains(&j) {
            2 * (3 * j + i) + k
        } else {
            -1
        }
    };
    for j in 0..3i16 {
        for i in 0..3i16 {
            let (a, b, d, e) = (
                4 * j + i,
                4 * j + i + 1,
                4 * (j + 1) + i + 1,
                4 * (j + 1) + i,
            );
            for (k, (vs, ns)) in [
                (
                    [a, b, d],
                    [tri(i, j - 1, 1), tri(i + 1, j, 1), tri(i, j, 1)],
                ),
                (
                    [a, d, e],
                    [tri(i, j, 0), tri(i, j + 1, 0), tri(i - 1, j, 0)],
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let at = tris + 16 * u32::try_from(tri(i, j, i16::try_from(k).unwrap())).unwrap();
                for (n, v) in (0u32..).zip(vs.iter().chain(ns.iter())) {
                    t.put(at + 2 * n, &v.to_le_bytes());
                }
            }
        }
    }
    t.put_u32(t.a.rebase(0x0130_1a84), tris);
    t.put_u32(t.a.rebase(0x0130_1a8c), verts);
    let depth = t.a.rebase(0x0130_1ae8);
    let slack = t.a.rebase(0x0130_1af4);
    let f = |rng: &mut Xorshift, lo: i32, hi: i32| {
        (rng.next() % u32::try_from(hi - lo).unwrap()) as f32 + lo as f32
    };
    for round in 0..96u32 {
        fk.reset(t);
        t.put_u32(matches, u32::from(rng.next().is_multiple_of(5)));
        t.put_u32(
            normal + 4,
            if rng.next().is_multiple_of(3) {
                0
            } else {
                0.5f32.to_bits()
            },
        );
        t.put_u32(h, f(&mut rng, -10, 60).to_bits());
        t.put_u32(slack, f(&mut rng, 0, 20).to_bits());
        t.put_u32(depth, rng.next() % 12);
        let chr = t.alloc(0x900);
        for off in [0x3cu32, 0x44] {
            t.put_u32(chr + off, f(&mut rng, -160, 160).to_bits());
        }
        t.put_u32(chr + 0x40, f(&mut rng, -10, 60).to_bits());
        let p = t.alloc(16);
        t.put_u32(p, f(&mut rng, -170, 170).to_bits());
        t.put_u32(p + 8, f(&mut rng, -170, 170).to_bits());
        let start = if round % 16 == 15 {
            u32::MAX
        } else {
            rng.next() % 18
        };
        let scratch = t.alloc(p::ID_WALK_SCRATCH as usize);
        let at = t.a.rebase(0x0083_d960);
        let ra = t.a.call(at, Conv::Cdecl, &[chr, p, start]).unwrap();
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
            p::sg_id_walk(&mut t.b, chr, p, start, scratch, &mut c)
        };
        assert_eq!(ra.eax, rb, "{}: Sg_IdWalk {round}", t.name);
        let s = Oracle::read(&mut t.b, scratch, p::ID_WALK_SCRATCH as usize).unwrap();
        t.a.write(scratch, &s).unwrap();
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Sg_IdWalk {round} memory",
            t.name
        );
    }
}

#[test]
fn round14b_matches_the_original() {
    for mut t in twins() {
        round14b_on(&mut t);
    }
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round14b_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0014_b14b);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let yet = fk.fake(t, 0x0083_ffe0, 1, 2);
    let index = fk.fake(t, 0x0082_a640, 2, 1);
    let heap = fk.fake_alloc(t, 0x0068_7020, 3, 0x100);
    let name = fk.fake(t, 0x0083_8100, 4, 1);
    let section = fk.fake(t, 0x0083_8150, 5, 2);
    // Sg_MemSetStr: the name (the caller's stack) copied out, then logged
    let names = t.alloc(32);
    let mut pre = vec![0x8b, 0x44, 0x24, 0x08];
    for k in 0..8u8 {
        pre.extend([0x8b, 0x48, 4 * k, 0x89, 0x0d]);
        pre.extend((names + 4 * u32::from(k)).to_le_bytes());
    }
    let (set_code, set_str) = fk.fake_code_idxs(t, &pre, 6, &[0]);
    let at = t.a.rebase(0x0083_c580);
    let mut jmp = vec![0xe9];
    jmp.extend(set_code.wrapping_sub(at + 5).to_le_bytes());
    t.put(at, &jmp);
    let entry = fk.fake(t, 0x0084_0560, 7, 3);
    let not_yet = fk.fake(t, 0x0084_05f0, 8, 3);
    let load = fk.fake(t, 0x0082_a1c0, 9, 4);
    let add = fk.fake(t, 0x0083_7630, 10, 2);
    let (size_code, size) = fk.fake_code(t, 11, 1);
    let (file_code, file) = fk.fake_code(t, 12, 1);
    let (read_code, read) = fk.fake_code(t, 13, 3);
    t.put_u32(t.a.rebase(0x0231_0c54), size_code);
    t.put_u32(t.a.rebase(0x0231_0c98), file_code);
    t.put_u32(t.a.rebase(0x0231_0c40), read_code);
    // Cache_Entry_RenderData: writes its two slot pointers and two words
    let (slots, words) = (t.alloc(8), t.alloc(8));
    let mut pre = vec![];
    for (k, src) in [(3u8, slots), (4, slots + 4), (5, words), (6, words + 4)] {
        // mov eax, [esp + 4k]; mov ecx, src (as a value); mov [eax], ecx
        pre.extend([0x8b, 0x44, 0x24, 4 * k, 0x8b, 0x0d]);
        pre.extend(src.to_le_bytes());
        pre.extend([0x89, 0x08]);
    }
    let (render, _) = fk.fake_code_idxs(t, &pre, 14, &[0, 1]);
    let at = t.a.rebase(0x0084_0680);
    let mut jmp = vec![0xe9];
    jmp.extend(render.wrapping_sub(at + 5).to_le_bytes());
    t.put(at, &jmp);
    let strs = t.alloc(32);
    t.put(strs, b"c010\0\0\0\0chr\0");
    for round in 0..24u32 {
        fk.reset(t);
        t.put_u32(yet, rng.next() % 4);
        t.put_u32(
            index,
            if rng.next().is_multiple_of(5) {
                u32::MAX
            } else {
                rng.next() % 64
            },
        );
        t.put_u32(heap, 0x1200_0000 + 0x1000 * round);
        t.put_u32(name, strs);
        t.put_u32(section, strs + 8);
        for r in [set_str, entry, not_yet, load, add, file, read] {
            t.put_u32(r, rng.next());
        }
        t.put_u32(
            size,
            if rng.next().is_multiple_of(5) {
                0
            } else {
                rng.next() % 0x10000
            },
        );
        let (a, b) = (t.alloc(4), t.alloc(4));
        t.put_u32(a, rng.next());
        t.put_u32(b, rng.next());
        t.put_u32(slots, a);
        t.put_u32(slots + 4, b);
        t.put_u32(words, rng.next());
        t.put_u32(words + 4, rng.next());
        let id = rng.next() % 0x10000;
        let scratch = t.alloc(sy::DATA_READ_SCRATCH as usize);
        let at = t.a.rebase(0x0082_9ef0);
        let ra = t.a.call(at, Conv::Cdecl, &[id]).unwrap();
        let rb = sy::ch_data_read(&mut t.b, id, scratch, &mut |m, f, a| call(m, f, a));
        assert_eq!(ra.eax, rb, "{}: Ch_DataRead {round}", t.name);
        let s = Oracle::read(&mut t.b, scratch, sy::DATA_READ_SCRATCH as usize).unwrap();
        t.a.write(scratch, &s).unwrap();
        // the copied name: only up to its NUL (the rest is either stack)
        for o in [&mut t.a, &mut t.b] {
            let mut n = Oracle::read(o, names, 32).unwrap();
            if let Some(end) = n.iter().position(|&c| c == 0) {
                n[end..].fill(0);
            }
            o.write(names, &n).unwrap();
        }
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_DataRead {round} memory",
            t.name
        );
        let req = t.alloc(0x20);
        t.put_u32(req + 0xc, rng.next());
        t.put_u32(req + 0x10, rng.next());
        t.check(
            &format!("Ch_RomReadEventMGRPcb {round}"),
            0x0082_a7d0,
            &[req],
            |m| {
                Res::Int(sy::ch_rom_read_event_mgrp_cb(m, req, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
    }
}

impl Fakes {
    /// [`Fakes::fake`] whose result word takes the word at `next` after
    /// each call (a first call's result, then another's): `(result, next)`.
    fn fake_seq(&self, t: &mut Twin, sym: u32, id: u32, n: u8) -> (u32, u32) {
        let (code, ret) = self.fake_code(t, id, n);
        let next = t.alloc(4);
        // the stand-in again, ending `mov ecx, [next]; mov [ret], ecx; ret`
        let len = 25 + 7 * usize::from(n);
        let old = Oracle::read(&mut t.a, code, len).unwrap();
        let mut c = old[..len - 1].to_vec();
        c.extend([0x8b, 0x0d]);
        c.extend(next.to_le_bytes());
        c.extend([0x89, 0x0d]);
        c.extend(ret.to_le_bytes());
        c.push(0xc3);
        let at = t.a.alloc_code(&c).unwrap();
        assert_eq!(t.b.alloc_code(&c).unwrap(), at);
        let f = t.a.rebase(sym);
        let mut jmp = vec![0xe9];
        jmp.extend(at.wrapping_sub(f + 5).to_le_bytes());
        t.put(f, &jmp);
        (ret, next)
    }
}

#[test]
fn round35_allocate_matches_the_original() {
    for mut t in twins() {
        round35_allocate_on(&mut t);
    }
}

/// `Ch_Allocate` (battle round 35). `Ch_BlkGetHandle` (its PC-file path),
/// `dbgPrintf`, the render registration and `MagicFile_GetMagicId` are
/// stand-ins; the rest runs as the originals (the chara ones the port calls
/// as its Rust). On the image alone only the missing-block paths run; on
/// a snapshot a new character is made from the live characters' blocks in
/// the first free slot (one is freed when there is none), with the block's
/// id at times one of the ids the function looks for.
///
/// Round 36 added: every slot in use (the null character, written through
/// on a zeroed null page); id tables with an entry of the id
/// (`FLAG2_IDS` with ids from 0x8000 up, which the original sign-extends;
/// `ID_FLOATS`' last entry); and stand-ins for `Ch_SetShadeID` and
/// `op_et_monster_init` that leave marks in the new character (moves 0 and
/// 0x10, the cloth offset, FLAGS2 bits 3 and 0, FLAGS bit 0, the shadow
/// mode, the shadow slot; `op_et_monster_init` also logs FLAGS2), so the
/// defaults that overwrite them, and FLAGS2 0x400 held around the set-up,
/// show.
#[allow(clippy::too_many_lines)]
fn round35_allocate_on(t: &mut Twin) {
    const IDS: [u16; 32] = [
        0x1088, 0x1089, 0x108a, 0x108c, 0x10a6, 0x10b3, 0x111e, 0x1122, 0x1124, 0x3004, 0x3009,
        0x300b, 0x300c, 0x1003, 0x1010, 0x1133, 0x0002, 0x0006, 0x006b, 0x4001, 0x7001, 0xf001,
        0xf002, 0x5010, 0x6123, 0x10b4, 0x1123, 0x301c, 0x3004, 0x10b2, 0x0065, 0x8004,
    ];

    let mut rng = Xorshift(0x0035_4f90);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let (blk_ret, blk_next) = fk.fake_seq(t, 0x0082_5a40, 1, 1);
    allocate_print_stub(t, &fk);
    let _ = fk.fake(t, 0x0063_d370, 3, 4);
    let magic = fk.fake(t, 0x009d_a3b0, 4, 0);
    let err = t.a.rebase(0x012f_bd98);
    let debug = t.a.rebase(0x0133_c910);
    if t.name == "synthetic" {
        for round in 0..4u32 {
            fk.reset(t);
            t.put_u32(blk_ret, 0);
            t.put_u32(blk_next, 0);
            t.put_u32(err, 1 + round);
            let key = rng.next() & 0xffff;
            t.check(
                &format!("Ch_Allocate missing {round}"),
                0x0082_4f90,
                &[key],
                |m| Res::Int(h::ch_allocate(m, key, &mut |m, f, a| call(m, f, a))),
            );
        }
        return;
    }
    let _ = (blk_ret, blk_next, magic, err, debug, fk);
    let (mut ran, mut skipped) = (0u32, 0u32);
    let mut reach = [0u32; 6];
    let mut next_id = 0usize;
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    for round in 0..54u32 {
        // a fresh copy each round: the game's heap has room for few new
        // characters' bones (more come from the oracle's VirtualAlloc)
        let mut s = Twin {
            name: t.name,
            a: Oracle::from_snapshot(&exe, &pages).unwrap(),
            b: Oracle::from_snapshot(&exe, &pages).unwrap(),
        };
        let fk = Fakes::new(&mut s);
        let (blk_ret, blk_next) = fk.fake_seq(&mut s, 0x0082_5a40, 1, 1);
        allocate_print_stub(&mut s, &fk);
        let _ = fk.fake(&mut s, 0x0063_d370, 3, 4);
        let magic = fk.fake(&mut s, 0x009d_a3b0, 4, 0);
        let _ = fk.fake(&mut s, 0x0083_c4d0, 5, 1);
        let _ = fk.fake(&mut s, 0x0084_0ac0, 6, 3);
        let render = fk.fake(&mut s, 0x0084_0890, 7, 2);
        let shadow = fk.fake(&mut s, 0x0084_08d0, 8, 2);
        s.put_u32(render, rng.next());
        s.put_u32(shadow, if round % 2 == 0 { 0 } else { rng.next() | 1 });
        // the snapshot's own live characters (not the made-up last one)
        let chrs = live_characters(&mut s, &mut rng);
        let blks: Vec<u32> = chrs[..chrs.len() - 1]
            .iter()
            .map(|&c| rd::<u32>(&s.b, c + l::BLK))
            .filter(|&b| b != 0 && b != 0xcdcd_cdcd)
            .collect();
        assert!(!blks.is_empty(), "no live character blocks");
        let arr: u32 = rd(&s.b, s.a.rebase(0x023c_44e4));
        let count: i32 = rd(&s.b, s.a.rebase(0x023c_44e0));
        // a free slot: the first one, or one freed; at times none (the
        // null character, on a zeroed null page)
        let full = round % 9 == 8;
        let free = (0..count).find(|&i| rd::<u8>(&s.b, arr + 0x880 * i.cast_unsigned() + 2) == 0);
        if full {
            for i in 0..count.cast_unsigned() {
                s.put(arr + 0x880 * i + 2, &[1]);
            }
            s.a.map_zeroed(0, 0x1_0000).unwrap();
            s.b.map_zeroed(0, 0x1_0000).unwrap();
        } else if free.is_none() || round % 5 == 4 {
            let i = rng.next() % count.cast_unsigned();
            s.put(arr + 0x880 * i + 2, &[0]);
        }
        if round % 7 == 0 && !full {
            s.put(arr + 2, &[0]);
        }
        let blk = blks[rng.next() as usize % blks.len()];
        // every listed id in turn; at times the block's own
        let id = if round % 5 == 3 {
            rd::<u16>(&s.b, blk)
        } else {
            next_id += 1;
            IDS[(next_id - 1) % IDS.len()]
        };
        s.put(blk, &id.to_le_bytes());
        // the id in the tables: a FLAG2_IDS entry, ID_FLOATS' last one
        if round % 3 == 1 || id >= 0x8000 {
            let k = if round % 6 == 1 {
                0xc4 / 2 - 1
            } else {
                rng.next() % (0xc4 / 2)
            };
            s.put(s.a.rebase(0x00c4_94d8) + 2 * k, &id.to_le_bytes());
        }
        if round % 4 == 2 {
            let e = s.a.rebase(0x00c4_95b0) + 12 * 12;
            s.put_u32(e, u32::from(id));
            s.put_u32(e + 4, rng.next() & 0x3fff_ffff);
            s.put_u32(e + 8, rng.next() & 0x3fff_ffff);
        }
        // stand-ins that leave marks for the defaults to overwrite
        let marks = round % 3 == 2 || (4..=7).contains(&(id >> 12));
        let flags2_log = s.alloc(4);
        let shadow_block = s.alloc(0x400);
        if marks {
            let shade = [
                Poke::Or(l::FLAGS2, 8),
                Poke::Set(l::MOVES, 5),
                Poke::Set(l::MOVES + 4 * 0x10, 5),
                Poke::Set(0x864, 0x3f80_0000),
            ];
            poke_stub(&mut s, 0x0082_b7a0, &shade);
            let monster = [
                Poke::Log(l::FLAGS2, flags2_log),
                Poke::Or(l::FLAGS2, 1),
                Poke::Or(l::FLAGS, 1),
                Poke::Byte(0x183, 2),
                // a shadow instance (read before the original clears it)
                Poke::Set(0x838, shadow_block),
            ];
            poke_stub(&mut s, 0x007f_c780, &monster);
        }
        // the missing block taking block 1, or the block itself
        let fallback = round % 4 == 1;
        s.put_u32(blk_ret, if fallback { 0 } else { blk });
        s.put_u32(blk_next, blk);
        s.put_u32(s.a.rebase(0x012f_bd98), 0);
        s.put(s.a.rebase(0x0133_c910), &[u8::from(round % 3 == 0)]);
        // Rikku in and out of magic file 0x113
        let rikku_file = rng.next().is_multiple_of(2);
        s.put_u32(
            magic,
            if rikku_file {
                0x113
            } else {
                [0, 0x112][rng.next() as usize % 2]
            },
        );
        let key = rng.next() & 0xffff;
        let what = format!("{}: Ch_Allocate {round} id {id:#x}", s.name);
        let ra = match s.a.call(s.a.rebase(0x0082_4f90), Conv::Cdecl, &[key]) {
            Ok(r) => r,
            Err(ffx_oracle::Error::UnstubbedImport(_) | ffx_oracle::Error::RuntimeAbort(_)) => {
                skipped += 1;
                continue;
            }
            Err(e) => panic!("{what}: {e}"),
        };
        let rb = h::ch_allocate(&mut s.b, key, &mut |m, f, a| call(m, f, a));
        assert_eq!(ra.eax, rb, "{what}: result");
        assert_eq!(s.a.diff(&mut s.b).unwrap(), [], "{what}: memory");
        ran += 1;
        reach[usize::from(fallback)] += 1;
        reach[2] += u32::from(id >> 12 == 0xf);
        reach[3] += u32::from(full);
        reach[4] += u32::from(marks);
        reach[5] += u32::from(rb != 0 && rd::<u32>(&s.b, rb + 0x83c) == 1);
    }
    eprintln!(
        "{}: Ch_Allocate {ran} ran, {skipped} skipped, {reach:?} (fallback, block, id 0xf..., \
         no free slot, marks, rest pose)",
        t.name
    );
    assert_eq!(skipped, 0, "{}: Ch_Allocate rounds skipped", t.name);
}

/// What a [`poke_stub`] does to the character in its first argument.
#[derive(Clone, Copy)]
enum Poke {
    /// `[chr + off] |= v`.
    Or(u32, u32),
    /// `[chr + off] = v`.
    Set(u32, u32),
    /// `byte [chr + off] = v`.
    Byte(u32, u8),
    /// `[at] = [chr + off]`.
    Log(u32, u32),
}

/// Replaces the cdecl function at `sym` with a stand-in that applies
/// `pokes` to its first argument's fields and returns 0.
fn poke_stub(t: &mut Twin, sym: u32, pokes: &[Poke]) {
    let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp + 4]
    for p in pokes {
        match *p {
            Poke::Or(off, v) => {
                c.extend([0x81, 0x88]); // or dword [eax + off], v
                c.extend(off.to_le_bytes());
                c.extend(v.to_le_bytes());
            }
            Poke::Set(off, v) => {
                c.extend([0xc7, 0x80]); // mov dword [eax + off], v
                c.extend(off.to_le_bytes());
                c.extend(v.to_le_bytes());
            }
            Poke::Byte(off, v) => {
                c.extend([0xc6, 0x80]); // mov byte [eax + off], v
                c.extend(off.to_le_bytes());
                c.push(v);
            }
            Poke::Log(off, at) => {
                c.extend([0x8b, 0x88]); // mov ecx, [eax + off]
                c.extend(off.to_le_bytes());
                c.extend([0x89, 0x0d]); // mov [at], ecx
                c.extend(at.to_le_bytes());
            }
        }
    }
    c.extend([0x33, 0xc0, 0xc3]); // xor eax, eax; ret
    let at = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), at);
    let f = t.a.rebase(sym);
    let mut jmp = vec![0xe9];
    jmp.extend(at.wrapping_sub(f + 5).to_le_bytes());
    t.put(f, &jmp);
}

/// `dbgPrintf` as a stand-in logging id 2 and its format, and both names
/// after `Ch_Allocate`'s own format (other callers pass fewer words).
fn allocate_print_stub(t: &mut Twin, fk: &Fakes) {
    print_stub(t, fk, 0x00b5_94f4, 2);
}

/// `dbgPrintf` as a stand-in logging id 2 and its format, and two more
/// words after the format `fmt` only (other callers pass fewer words).
fn print_stub(t: &mut Twin, fk: &Fakes, fmt: u32, extra: u8) {
    let fmt = t.a.rebase(fmt);
    let log = fk.log;
    let mut c = vec![0x8b, 0x0d];
    c.extend(log.to_le_bytes()); // mov ecx, [log]
    c.extend([0xc7, 0x01, 2, 0, 0, 0]); // mov dword [ecx], 2
    c.extend([0x8b, 0x44, 0x24, 0x04, 0x89, 0x41, 0x04]); // [ecx + 4] = fmt
    c.push(0x3d);
    c.extend(fmt.to_le_bytes()); // cmp eax, fmt
    let mut matched = Vec::new();
    for k in 0..extra {
        // [ecx + 8 + 4k] = [esp + 8 + 4k]
        matched.extend([0x8b, 0x44, 0x24, 8 + 4 * k, 0x89, 0x41, 8 + 4 * k]);
    }
    matched.extend([0x83, 0x05]);
    matched.extend(log.to_le_bytes());
    matched.extend([8 + 4 * extra, 0xc3]); // add dword [log], n; ret
    c.extend([0x75, u8::try_from(matched.len()).unwrap()]); // jne past it
    c.extend(matched);
    c.extend([0x83, 0x05]);
    c.extend(log.to_le_bytes());
    c.extend([0x08, 0xc3]); // add dword [log], 8; ret
    let at = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), at);
    let f = t.a.rebase(0x0062_f500);
    let mut jmp = vec![0xe9];
    jmp.extend(at.wrapping_sub(f + 5).to_le_bytes());
    t.put(f, &jmp);
}

#[test]
fn round35_dispose_matches_the_original() {
    for mut t in twins() {
        round35_dispose_on(&mut t);
    }
}

/// `ChN_DataDispose` and `Ch_Dispose` (battle round 35). `dbgPrintf`,
/// `yiAssert`, `Sg_CacheDisposeChr`, the magic file, the cloth job wait
/// and the render instances are stand-ins; the rest runs as the originals
/// (the chara ones the ports call as their Rust). On the image alone:
/// keys of no block and of the made-up characters, an inactive character.
/// On a snapshot each round starts from a fresh copy: a live character's
/// key (in use), another block's, none; then a live character disposed.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // t, s, c, m, f, a
fn round35_dispose_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0035_66f0);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let stubs = |s: &mut Twin, fk: &Fakes, (fmt, extra): (u32, u8), rng: &mut Xorshift| {
        print_stub(s, fk, fmt, extra);
        let _ = fk.fake(s, 0x0090_ae50, 3, 4);
        let _ = fk.fake(s, 0x0084_0e60, 4, 1);
        let handle = fk.fake(s, 0x009d_a390, 5, 0);
        let magic = fk.fake(s, 0x009d_a3b0, 6, 0);
        let _ = fk.fake(s, 0x00a4_4b30, 7, 0);
        let _ = fk.fake(s, 0x0063_a030, 8, 2);
        let cloth = fk.fake(s, 0x0063_a160, 9, 1);
        let _ = fk.fake(s, 0x0063_c390, 10, 2);
        s.put_u32(handle, [u32::MAX, 0, 3][rng.next() as usize % 3]);
        s.put_u32(magic, [0x113, 0x112][rng.next() as usize % 2]);
        s.put_u32(cloth, rng.next());
    };
    if t.name == "synthetic" {
        let fk = Fakes::new(t);
        stubs(t, &fk, (0x00b5_93fc, 1), &mut rng);
        let chrs = characters(t, &mut rng);
        for round in 0..12u32 {
            fk.reset(t);
            let key = if round % 2 == 0 {
                u32::from(rd::<u16>(&t.b, chrs[round as usize % chrs.len()]))
            } else {
                rng.next() | 0x8000_0000
            };
            let check = round % 3;
            t.check(
                &format!("ChN_DataDispose {round}"),
                0x0082_4f00,
                &[key, check],
                |m| {
                    Res::Int(h::ch_n_data_dispose(m, key, check, &mut |m, f, a| {
                        call(m, f, a)
                    }))
                },
            );
        }
        // an inactive character: Ch_CheckHandle's -1
        let c = *chrs.last().unwrap();
        t.put(c + l::ACTIVE, &[0]);
        t.check("Ch_Dispose inactive", 0x0082_66f0, &[c], |m| {
            Res::Int(h::ch_dispose(m, c, &mut |m, f, a| call(m, f, a)))
        });
        return;
    }
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let blks = t.a.rebase(0x023c_8d00);
    let mut reach = [0u32; 6];
    let mut skipped = 0u32;
    for round in 0..36u32 {
        let mut s = Twin {
            name: t.name,
            a: Oracle::from_snapshot(&exe, &pages).unwrap(),
            b: Oracle::from_snapshot(&exe, &pages).unwrap(),
        };
        let fk = Fakes::new(&mut s);
        let dispose = round % 2 == 1;
        // the CLUT rounds log Ch_ClutDispose's name and answer instead
        let fmt = match (dispose, round % 6) {
            (true, 1) => (0x00b5_a3fc, 2),
            (true, _) => (0x00b5_950c, 1),
            _ => (0x00b5_93fc, 1),
        };
        stubs(&mut s, &fk, fmt, &mut rng);
        let chrs = live_characters(&mut s, &mut rng);
        let live = &chrs[..chrs.len() - 1];
        if dispose {
            let c = live[(round as usize / 2) % live.len()];
            if round % 6 == 5 {
                // a block of its own with +0x70 bit 1: no monster effects
                let blk: u32 = rd(&s.b, c + l::BLK);
                let f: u8 = rd(&s.b, blk + 0x70);
                s.put(blk + 0x70, &[f ^ 2]);
            }
            // the trigger clear, the player at times this one, a shadow
            // instance, and at times a +0x874 table of heap blocks
            s.put(s.a.rebase(0x012f_bda0), &[0]);
            let player = s.a.rebase(0x0130_0788);
            s.put_u32(player, if round % 4 == 1 { c } else { rng.next() | 1 });
            s.put_u32(c + 0x838, if round % 3 == 0 { 0 } else { rng.next() | 1 });
            let header: u32 = rd(&s.b, c + 0x1c0);
            let mut n = u32::from(rd::<u16>(&s.b, header + 8));
            let malloc = |s: &mut Twin, len: u32| {
                let at = s.a.rebase(0x0068_7020);
                let x = s.a.call_cdecl(at, &[len]).unwrap();
                assert_eq!(s.b.call_cdecl(at, &[len]).unwrap(), x);
                x
            };
            // round 36: CLUTs to give back (+0x704), a texture animation
            // flag in +0x6fc's low byte only (id 0), and no +0x874 table
            // with the next word's low byte set
            if round % 6 == 1 {
                let cluts = malloc(&mut s, 0x40);
                s.put_u32(c + 0x704, cluts);
                reach[4] += 1;
                if round % 12 == 7 {
                    // petrified: the clut stays (a stand-in's answer)
                    let stone = fk.fake(&mut s, 0x007f_cc80, 12, 1);
                    s.put_u32(stone, 1);
                }
            }
            if round % 6 == 3 {
                s.put_u32(c + 0x6fc, 1);
                s.put_u32(c + 0x700, 0);
            }
            if round % 4 == 1 {
                s.put_u32(c + 0x874, 0);
                s.put(c + 0x878, &[1 + u8::try_from(rng.next() % 0xfe).unwrap()]);
            }
            if round % 4 == 3 && n <= 64 {
                // a logging user_free: the null entries are skipped
                let _ = fk.fake(&mut s, 0x0068_7010, 11, 1);
                if n == 0 {
                    // a header without entries: some to free
                    n = 5;
                    s.put(header + 8, &5u16.to_le_bytes());
                }
                let table = malloc(&mut s, 4 * n.max(1));
                for k in 0..n {
                    let p = if k % 3 == 1 { 0 } else { malloc(&mut s, 0x20) };
                    s.put_u32(table + 4 * k, p);
                }
                s.put_u32(c + 0x874, table);
                reach[5] += 1;
            }
            let what = format!("{}: Ch_Dispose {round} chr {c:#x}", s.name);
            // the original may fault on a character the snapshot caught
            // half set up (stale pointers): such rounds are counted, not run
            let ra = match s.a.call(s.a.rebase(0x0082_66f0), Conv::Cdecl, &[c]) {
                Ok(r) => r,
                Err(ffx_oracle::Error::BadAccess(at)) => {
                    eprintln!("{what}: the original faults at {at:#x}: skipped");
                    skipped += 1;
                    continue;
                }
                Err(e) => panic!("{what}: {e}"),
            };
            let rb = h::ch_dispose(&mut s.b, c, &mut |m, f, a| call(m, f, a));
            assert_eq!(ra.eax, rb, "{what}: result");
            assert_eq!(s.a.diff(&mut s.b).unwrap(), [], "{what}: memory");
            reach[3] += 1;
            continue;
        }
        let in_use: Vec<u32> = live
            .iter()
            .map(|&c| u32::from(rd::<u16>(&s.b, c)))
            .collect();
        let free_blk = (0..40u32)
            .map(|k| rd::<u32>(&s.b, blks + 0x12c * k))
            .find(|id| *id != u32::MAX && !in_use.contains(id));
        let (key, kind) = match (round / 2) % 3 {
            0 => (in_use[rng.next() as usize % in_use.len()], 0),
            1 => {
                // a block no character uses: one of its own, or a live
                // character's with every character of that id made inactive
                let key = free_blk.unwrap_or_else(|| {
                    let key = in_use[rng.next() as usize % in_use.len()];
                    for &c in live {
                        if u32::from(rd::<u16>(&s.b, c)) == key {
                            s.put(c + l::ACTIVE, &[0]);
                        }
                    }
                    key
                });
                (key, 1)
            }
            _ => (0x7ff0 | (rng.next() & 0xf), 2),
        };
        reach[kind] += 1;
        let check = rng.next() % 2;
        let what = format!("{}: ChN_DataDispose {round} key {key:#x}", s.name);
        s.check(&what, 0x0082_4f00, &[key, check], |m| {
            Res::Int(h::ch_n_data_dispose(m, key, check, &mut |m, f, a| {
                call(m, f, a)
            }))
        });
    }
    eprintln!("{}: dispose reach {reach:?}, {skipped} skipped", t.name);
    assert!(reach[3] >= 6, "{}: too few Ch_Dispose rounds ran", t.name);
}

#[test]
fn round36_shade_id_matches_the_original() {
    for mut t in twins() {
        round36_shade_id_on(&mut t);
    }
}

/// `Ch_SetShadeID` (battle round 36): the renderer's lighting setters are
/// logging stand-ins with random results; `Ch_SetShadeIDRelation` runs as
/// the original on a snapshot's live characters (it walks the bound
/// ones) and is a stand-in on the made-up ones. Targets and colour fades
/// are any floats (quiet NaNs, infinities, denormals), frames 0 to 60 and
/// negative.
fn round36_shade_id_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0036_b720);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let coeff = fk.fake(t, 0x0063_cdd0, 1, 2);
    let color = fk.fake(t, 0x0063_d050, 2, 4);
    let synthetic = t.name == "synthetic";
    if synthetic {
        let _ = fk.fake(t, 0x0082_b7a0, 3, 3);
    }
    let chrs = if synthetic {
        characters(t, &mut rng)
    } else {
        let live = live_characters(t, &mut rng);
        live[..live.len() - 1].to_vec()
    };
    for round in 0..120u32 {
        fk.reset(t);
        let chr = chrs[rng.next() as usize % chrs.len()];
        t.put_u32(coeff, rng.next());
        t.put_u32(color, rng.next());
        for o in [0x330, 0x33c, 0x348, 0x36c, 0x370] {
            t.put_u32(chr + o, any_float(&mut rng));
        }
        t.put_u32(chr + 0x374, rng.next() % 40);
        let v = any_float(&mut rng);
        let frames = [0, 1, rng.next() % 60, u32::MAX, 0x8000_0000][rng.next() as usize % 5];
        t.check(
            &format!("Ch_SetShadeID {round}"),
            0x0082_b720,
            &[chr, v, frames],
            |m| {
                Res::Int(c::ch_set_shade_id(m, chr, fl(v), frames, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
    }
}

/// A stand-in for the cdecl function at `sym` that appends the words of
/// arguments `words` and then the C string its argument `string` points
/// to at the cursor `log` (a word holding the next address), and returns
/// the words at the cursor `table` in turn.
fn string_stub(t: &mut Twin, sym: u32, string: u8, words: &[u8], log: u32, table: u32) {
    let mut c = vec![0x56, 0x57, 0x8b, 0x3d]; // push esi; push edi; mov edi, [log]
    c.extend(log.to_le_bytes());
    for &k in words {
        c.extend([0x8b, 0x44, 0x24, 12 + 4 * k, 0xab]); // mov eax, [arg k]; stosd
    }
    c.extend([0x8b, 0x74, 0x24, 12 + 4 * string]); // mov esi, [arg]
    c.extend([0xac, 0xaa, 0x84, 0xc0, 0x75, 0xfa]); // copy through the NUL
    c.extend([0x89, 0x3d]);
    c.extend(log.to_le_bytes()); // mov [log], edi
    c.push(0xa1);
    c.extend(table.to_le_bytes()); // mov eax, [table]
    c.extend([0x8b, 0x00, 0x83, 0x05]); // mov eax, [eax]; add dword [table], 4
    c.extend(table.to_le_bytes());
    c.extend([0x04, 0x5f, 0x5e, 0xc3]);
    let at = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), at);
    let f = t.a.rebase(sym);
    let mut jmp = vec![0xe9];
    jmp.extend(at.wrapping_sub(f + 5).to_le_bytes());
    t.put(f, &jmp);
}

#[test]
fn round36_blk_get_handle_matches_the_original() {
    for mut t in twins() {
        round36_blk_get_handle_on(&mut t);
    }
}

/// `Ch_BlkGetHandle` (battle round 36). `Sg_PcRead` and `Sg_MemSetStr`
/// log their strings and words (`Sg_PcRead` returning made-up files in
/// turn), `Sg_MemSetAlign`, `Chb_AttachMotionTBL_OLD`, `Ch_DataRead`, the
/// CD library's sync and `dbgPrintf` are logging stand-ins, and the render
/// cache runs as the original or as stand-ins (the missing paths). The
/// rest runs as the originals (the chara ones the port calls as its
/// Rust). Rounds by kind: 0 a block in the table; 1 the cache's data
/// (the block freed first); 2 the same with `g_FFXIsDebug` and a debug
/// list entry; 3 no debug entry; 4 not in the cache (read, then there or
/// not); 5 a viewer `.chr`, with or without its texture; 6 a viewer
/// `.mdl`; 7 no viewer file. On the image alone kinds 3, 4 (still
/// missing) and 7 run; on a snapshot each round starts from a fresh copy.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // t, s, c, f, e
fn round36_blk_get_handle_on(t: &mut Twin) {
    let mut rng = Xorshift(0x0036_5a40);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let snapshot = match (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) {
        (Some(dir), Some(snap)) if t.name != "synthetic" => Some((
            PathBuf::from(dir).join("FFX.exe"),
            Pages::read(PathBuf::from(snap)).unwrap(),
        )),
        _ => None,
    };
    if t.name != "synthetic" && snapshot.is_none() {
        return;
    }
    let blks = t.a.rebase(0x023c_8d00);
    let mut reach = [0u32; 9];
    for round in 0..48u32 {
        let kind = round % 8;
        if snapshot.is_none() && ![3, 4, 7].contains(&kind) {
            continue;
        }
        let mut fresh = snapshot.as_ref().map(|(exe, pages)| Twin {
            name: t.name,
            a: Oracle::from_snapshot(exe, pages).unwrap(),
            b: Oracle::from_snapshot(exe, pages).unwrap(),
        });
        let s = fresh.as_mut().unwrap_or(t);
        let fk = Fakes::new(s);
        let strings = s.alloc(0x2000);
        let log = s.alloc(4);
        s.put_u32(log, strings);
        let files = s.alloc(0x40);
        let table = s.alloc(4);
        s.put_u32(table, files);
        let zeros = s.alloc(0x40);
        let zeros_at = s.alloc(4);
        s.put_u32(zeros_at, zeros);
        string_stub(s, 0x0083_ae30, 0, &[], log, table);
        string_stub(s, 0x0083_c580, 1, &[0, 2], log, zeros_at);
        let _ = fk.fake(s, 0x0083_c4a0, 1, 1);
        let _ = fk.fake(s, 0x0083_7fa0, 2, 1);
        let _ = fk.fake(s, 0x0082_9ef0, 3, 1);
        // dbgPrintf (empty in the game): only "chrdata %x is not exist"
        // counts, with its key (Ch_BlkAllocate's port leaves its own out)
        let printed = s.alloc(8);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x3d];
        c.extend(s.a.rebase(0x00b5_93c4).to_le_bytes());
        c.extend([0x75, 0x0f, 0x8b, 0x44, 0x24, 0x08, 0xa3]);
        c.extend(printed.to_le_bytes());
        c.extend([0xff, 0x05]);
        c.extend((printed + 4).to_le_bytes());
        c.push(0xc3);
        let at = s.a.alloc_code(&c).unwrap();
        assert_eq!(s.b.alloc_code(&c).unwrap(), at);
        let f = s.a.rebase(0x0062_f500);
        let mut jmp = vec![0xe9];
        jmp.extend(at.wrapping_sub(f + 5).to_le_bytes());
        s.put(f, &jmp);
        let (sync, _) = fk.fake_code(s, 4, 0);
        s.put_u32(s.a.rebase(0x0231_0c48), sync);
        let debug = s.a.rebase(0x0133_c910);
        s.put(debug, &[u8::from(kind == 2 || kind == 3)]);
        // the live blocks (id, address)
        let live: Vec<(u32, u32)> = (0..40u32)
            .map(|k| blks + 0x12c * k)
            .map(|b| (rd::<u32>(&s.b, b), b))
            .filter(|&(id, _)| id != u32::MAX && id != 0)
            .collect();
        let pick = |rng: &mut Xorshift| live[rng.next() as usize % live.len().max(1)];
        // a live block's cache data (asked of the original on both)
        let cached = |s: &mut Twin, id: u32| {
            let at = s.a.rebase(0x0084_0790);
            let x = s.a.call_cdecl(at, &[1, id]).unwrap();
            assert_eq!(s.b.call_cdecl(at, &[1, id]).unwrap(), x);
            x
        };
        let key = match kind {
            0 => pick(&mut rng).0,
            // the debug list missing a key above 0xff (one entry of its
            // low byte)
            3 if round % 16 == 11 => 0x3000 | 0x100 | (rng.next() & 0xff),
            1..=4 => {
                // the image: a key of no block, nothing cached
                let (id, data) = if live.is_empty() {
                    (0x2000 | (rng.next() & 0xfff), 0)
                } else {
                    let (id, b) = pick(&mut rng);
                    let data = cached(s, id);
                    s.put_u32(b, u32::MAX);
                    (id, data)
                };
                if kind == 4 {
                    let (first, next) = fk.fake_seq(s, 0x0084_0790, 5, 2);
                    s.put_u32(first, 0);
                    s.put_u32(next, if round % 16 == 4 { 0 } else { data });
                    let yet = fk.fake(s, 0x0083_ffe0, 6, 2);
                    s.put_u32(yet, [0, 1, 2][rng.next() as usize % 3]);
                    let _ = fk.fake(s, 0x0084_0dd0, 7, 2);
                }
                id
            }
            _ => {
                // entries past 0xff too
                let e = rng.next() % 0x20 + if round % 16 >= 8 { 0x100 } else { 0 };
                let name = s.alloc(0x20);
                let dir = s.alloc(0x20);
                let n = format!("v{:03}", rng.next() % 1000);
                s.put(name, &[n.as_bytes(), &[0]].concat());
                s.put(dir, b"prot_mon\0");
                let entry = s.a.rebase(0x012f_bca8) + 12 * e;
                s.put_u32(entry, name);
                s.put_u32(entry + 8, dir);
                // texture data the mapping takes (version 0x1029, its
                // base 0, one clut starting with the colour key)
                let tex = s.alloc(0x40);
                s.put(tex + 0x10, &0x1029u16.to_le_bytes());
                s.put(tex + 8, &1u16.to_le_bytes());
                s.put_u32(tex + 0x18, 0x20);
                s.put_u32(tex + 0x24, 0x30);
                s.put(tex + 0x30, &[0, 0xff, 0, 0x7f]);
                let (data, slot) = if live.is_empty() {
                    (0, 0)
                } else {
                    let (id, b) = pick(&mut rng);
                    (cached(s, id), b)
                };
                // a free block for the new one
                if slot != 0 {
                    s.put_u32(slot, u32::MAX);
                }
                let mdl = s.alloc(0x200);
                let tex = if round % 16 < 8 { tex } else { 0 };
                let seq: Vec<u32> = match kind {
                    5 => vec![data, tex],
                    6 => vec![0, mdl, tex],
                    _ => vec![0, 0],
                };
                if kind == 6 {
                    let _ = fk.fake(s, 0x0082_7590, 8, 1);
                }
                for (k, v) in (0u32..).zip(seq) {
                    s.put_u32(files + 4 * k, v);
                }
                0xf000 | e
            }
        };
        if kind == 2 || kind == 3 {
            // the kind's debug list: entries of other numbers, and of the
            // key's (twice: the last counts) for kind 2
            let k = key >> 12;
            let n = 1 + rng.next() % 4;
            let list = s.alloc(0x80 * n as usize);
            for j in 0..n {
                let num = if kind == 2 && j >= n / 2 {
                    key & 0xfff
                } else if j == 0 {
                    key & 0xff
                } else {
                    (key & 0xfff) ^ (1 + j)
                };
                s.put_u32(list + 0x80 * j, num);
                s.put_u32(list + 0x80 * j + 0x30, rng.next());
            }
            s.put_u32(s.a.rebase(0x012f_bc64) + 4 * k, list);
            let count = if kind == 3 && round % 16 == 3 { 0 } else { n };
            s.put_u32(s.a.rebase(0x012f_bc84) + 4 * k, count);
        }
        let frame = s.b.stack_range().start + 0x400;
        let what = format!("Ch_BlkGetHandle {round} key {key:#x}");
        s.check(&what, 0x0082_5a40, &[key], |m| {
            Res::Int(h::ch_blk_get_handle(m, key, frame, &mut |m, f, a| {
                call(m, f, a)
            }))
        });
        let got =
            ffx_oracle::Oracle::call_cdecl(&mut s.b, s.a.rebase(0x0082_5ea0), &[key]).unwrap();
        reach[kind as usize] += u32::from(got != 0);
        reach[8] += 1;
    }
    eprintln!(
        "{}: Ch_BlkGetHandle reach {reach:?} (a block after each kind, rounds)",
        t.name
    );
}

// ---- chara-main-map round 1

/// Code at `sym` (both oracles): a jump to `code`.
fn jump_to(t: &mut Twin, sym: u32, code: &[u8]) {
    let at = t.a.alloc_code(code).unwrap();
    assert_eq!(t.b.alloc_code(code).unwrap(), at);
    let f = t.a.rebase(sym);
    let mut jmp = vec![0xe9];
    jmp.extend(at.wrapping_sub(f + 5).to_le_bytes());
    t.put(f, &jmp);
}

/// Code at `sym` (both oracles): a jump to `at`, code already there.
fn point_to(t: &mut Twin, sym: u32, at: u32) {
    let f = t.a.rebase(sym);
    let mut jmp = vec![0xe9];
    jmp.extend(at.wrapping_sub(f + 5).to_le_bytes());
    t.put(f, &jmp);
}

/// A stand-in at `sym` that logs its first `n` arguments and, the first
/// time after `flag` was set, decrements the word `count` (`dec` = (count,
/// flag)), so the caller's walk sees the array change; returns 0.
fn log_dec_stub(t: &mut Twin, fk: &Fakes, sym: u32, n: u8, dec: Option<(u32, u32)>) {
    log_dec_move_stub(t, fk, sym, n, dec, None);
}

/// [`log_dec_stub`] that also moves the array word `array` one character
/// on with the decrement (the walk's end stays where it was).
fn log_dec_move_stub(
    t: &mut Twin,
    fk: &Fakes,
    sym: u32,
    n: u8,
    dec: Option<(u32, u32)>,
    array: Option<u32>,
) {
    let c = log_dec_move_code(fk, n, dec, array);
    jump_to(t, sym, &c);
}

/// The code of [`log_dec_move_stub`].
fn log_dec_move_code(fk: &Fakes, n: u8, dec: Option<(u32, u32)>, array: Option<u32>) -> Vec<u8> {
    let mut c = vec![0x8b, 0x0d];
    c.extend(fk.log.to_le_bytes()); // mov ecx, [log]
    for k in 0..n {
        // mov eax, [esp + 4 + 4k]; mov [ecx + 4k], eax
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    c.extend([0x83, 0x05]);
    c.extend(fk.log.to_le_bytes());
    c.push(4 * n); // add dword [log], 4n
    if let Some((count, flag)) = dec {
        c.extend([0x83, 0x3d]);
        c.extend(flag.to_le_bytes());
        let skip: u8 = if array.is_some() { 0x1a } else { 0x10 };
        c.extend([0x00, 0x74, skip]); // cmp dword [flag], 0; je past
        if let Some(a) = array {
            c.extend([0x81, 0x05]);
            c.extend(a.to_le_bytes());
            c.extend(l::SIZE.to_le_bytes()); // add dword [array], 0x880
        }
        c.extend([0xff, 0x0d]);
        c.extend(count.to_le_bytes()); // dec dword [count]
        c.extend([0xc7, 0x05]);
        c.extend(flag.to_le_bytes());
        c.extend(0u32.to_le_bytes()); // mov dword [flag], 0
    }
    c.extend([0x33, 0xc0, 0xc3]); // xor eax, eax; ret
    c
}

/// A stand-in at `sym` that logs the C string its argument `arg` (0-based)
/// points at, then returns the word at `ret`, which grows by `step`.
fn cstr_stub(t: &mut Twin, fk: &Fakes, sym: u32, arg: u8, ret: u32, step: u32) {
    cstr_args_stub(t, fk, sym, &[], arg, (ret, step));
}

/// [`cstr_stub`] that logs its arguments `args` (0-based) first.
fn cstr_args_stub(
    t: &mut Twin,
    fk: &Fakes,
    sym: u32,
    args: &[u8],
    arg: u8,
    (ret, step): (u32, u32),
) {
    let mut c = vec![0x8b, 0x0d];
    c.extend(fk.log.to_le_bytes()); // mov ecx, [log]
    for &i in args {
        // mov eax, [esp + 4 + 4i]; mov [ecx], eax; add ecx, 4
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * i, 0x89, 0x01, 0x83, 0xc1, 0x04]);
    }
    c.extend([0x8b, 0x54, 0x24, 4 + 4 * arg]); // mov edx, [esp + 4 + 4arg]
    // copy up to the NUL: mov al, [edx]; mov [ecx], al; inc edx; inc ecx;
    // test al, al; jnz back
    c.extend([0x8a, 0x02, 0x88, 0x01, 0x42, 0x41, 0x84, 0xc0, 0x75, 0xf6]);
    c.extend([0x89, 0x0d]);
    c.extend(fk.log.to_le_bytes()); // mov [log], ecx
    c.push(0xa1);
    c.extend(ret.to_le_bytes()); // mov eax, [ret]
    c.extend([0x81, 0x05]);
    c.extend(ret.to_le_bytes());
    c.extend(step.to_le_bytes()); // add dword [ret], step
    c.push(0xc3);
    jump_to(t, sym, &c);
}

/// A stand-in at `sym` that logs its arguments `args` (0-based), then
/// `words` words from `from` bytes into its pointer argument `p`, and
/// returns `0x1234`.
fn deref_stub(t: &mut Twin, fk: &Fakes, sym: u32, args: &[u8], (p, from): (u8, u8), words: u8) {
    let code = deref_code(fk, args, (p, from), words);
    jump_to(t, sym, &code);
}

fn deref_code(fk: &Fakes, args: &[u8], (p, from): (u8, u8), words: u8) -> Vec<u8> {
    multi_deref_code(fk, args, &[(p, from, words)], (0, 0))
}

/// Code that logs its arguments `args` (0-based), then for each `(arg,
/// from, words)` that many words from `from` bytes into that pointer
/// argument, and returns the word at `ret` (0: `0x1234`), which grows by
/// `step`.
fn multi_deref_code(
    fk: &Fakes,
    args: &[u8],
    derefs: &[(u8, u8, u8)],
    (ret, step): (u32, u32),
) -> Vec<u8> {
    let mut c = vec![0x8b, 0x0d];
    c.extend(fk.log.to_le_bytes()); // mov ecx, [log]
    for &i in args {
        // mov eax, [esp + 4 + 4i]; mov [ecx], eax; add ecx, 4
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * i, 0x89, 0x01, 0x83, 0xc1, 0x04]);
    }
    for &(p, from, words) in derefs {
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * p]); // mov eax, [esp + 4 + 4p]
        for k in 0..words {
            // mov edx, [eax + from + 4k]; mov [ecx], edx; add ecx, 4
            c.extend([0x8b, 0x50, from + 4 * k, 0x89, 0x11, 0x83, 0xc1, 0x04]);
        }
    }
    c.extend([0x89, 0x0d]);
    c.extend(fk.log.to_le_bytes()); // mov [log], ecx
    if ret == 0 {
        c.push(0xb8);
        c.extend(0x1234u32.to_le_bytes()); // mov eax, 0x1234
    } else {
        c.push(0xa1);
        c.extend(ret.to_le_bytes()); // mov eax, [ret]
        c.extend([0x81, 0x05]);
        c.extend(ret.to_le_bytes());
        c.extend(step.to_le_bytes()); // add dword [ret], step
    }
    c.push(0xc3);
    c
}

/// [`multi_deref_code`] installed at `sym`.
fn multi_deref_stub(
    t: &mut Twin,
    fk: &Fakes,
    sym: u32,
    args: &[u8],
    derefs: &[(u8, u8, u8)],
    ret: (u32, u32),
) {
    let code = multi_deref_code(fk, args, derefs, ret);
    jump_to(t, sym, &code);
}

#[test]
fn cmm1_a_matches_the_original() {
    for mut t in twins() {
        cmm1_a_on(&mut t);
    }
}

/// Chara-main-map round 1, group a: `Ch_DisposeAll`, `Ch_Allocate2`,
/// `Ch_SetShadeIDRelation`, `Ch_ModelSetAlpha`, the footstep effect
/// `0x008338d0`, `Chb_AttachMotionTBL_OLD`, `Ch_RomReadEventMGRP`,
/// `Ch_IdShadeCalc`. Their callees outside the chara module are logging
/// stand-ins (pointers to the original's stack logged by what they point
/// at); the math ones (`Sg_VectorGetLength*`, `FFXVu0Normalize`) run as
/// the originals. The ports' scratch is on the Rust oracle's stack, which
/// the memory compare leaves out, as it does the original's.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // t, c, v, i, a, x
fn cmm1_a_on(t: &mut Twin) {
    let mut rng = Xorshift(0x00c3_a001);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fcall = |m: &mut Oracle, f: u32, a: &[u32]| m.call_f64(f, a).unwrap();
    let fk = Fakes::new(t);
    let frame = t.b.stack_range().start + 0x400;
    let (count, array) = (t.a.rebase(h::CHR_COUNT.sym), t.a.rebase(h::CHR_ARRAY.sym));
    let chrs = live_characters(t, &mut rng);
    // the renderer and effect callees
    let _ = fk.fake(t, 0x0063_bc50, 1, 4); // SetMeshSegmentAlpha
    let _ = fk.fake(t, 0x0063_cec0, 2, 1); // SetShadeCoeff_debug
    deref_stub(t, &fk, 0x007f_b740, &[0, 2, 3], (1, 0), 4); // op_et_bindeff2_walk(chr, foot, 0, 0)
    let reads = t.alloc(4);
    cstr_stub(t, &fk, 0x0083_ae30, 0, reads, 0); // Sg_PcRead
    // Ch_RomReadEventMGRP's callees: the CD library, the heap, names
    let mut cd = Vec::new();
    for (g, n) in [(0x0231_0c50u32, 1u8), (0x0231_0c54, 1), (0x0231_0c98, 2)] {
        let (code, ret) = fk.fake_code(t, 0x20 + (g & 0xff), n);
        let at = t.a.rebase(g);
        t.put_u32(at, code);
        cd.push(ret);
    }
    let lib = deref_code(&fk, &[0, 1, 2], (3, 0xc), 2); // g_CdLib(file, buf, cb, &req)
    let lib = {
        let at = t.a.alloc_code(&lib).unwrap();
        assert_eq!(t.b.alloc_code(&lib).unwrap(), at);
        at
    };
    let at = t.a.rebase(0x0231_0c40);
    t.put_u32(at, lib);
    let eternal = fk.fake(t, 0x0067_ab30, 3, 1);
    let eternal_size = fk.fake(t, 0x0067_aa70, 4, 1);
    let _ = fk.fake(t, 0x0083_c4a0, 5, 1); // Sg_MemSetAlign
    let heap = t.alloc(0x40);
    let malloc = fk.fake_alloc(t, 0x0068_7020, 6, 0x10);
    t.put_u32(malloc, heap);
    let none = t.alloc(4);
    cstr_args_stub(t, &fk, 0x0083_c580, &[0, 2], 1, (none, 0)); // Sg_MemSetStr(p, name, -1)
    let _ = fk.fake(t, 0x0068_7010, 10, 1); // user_free
    let flag = t.alloc(4);
    let _ = fk.fake(t, 0x0083_c590, 7, 2); // Sg_MemSetType
    let _ = fk.fake(t, 0x0062_f500, 8, 3); // dbgPrintf(fmt, r, buf)
    let mut reach = [0u32; 12];
    for round in 0..40u32 {
        fk.reset(t);
        let chr = chrs[rng.next() as usize % chrs.len()];
        // Ch_SetShadeIDRelation: some characters bound to chr; at times the
        // stand-in shortens the array
        let dec = round % 5 == 4;
        log_dec_stub(t, &fk, 0x0082_b720, 3, dec.then_some((count, flag)));
        t.put_u32(flag, 1);
        let (n0, a0) = (h::CHR_COUNT.get(&t.b), h::CHR_ARRAY.get(&t.b));
        let n0u = u32::try_from(n0).unwrap();
        for i in 0..n0u {
            let c = a0 + i * l::SIZE;
            // the walk must not step past a shortened end: the last slot
            // stays unbound then
            let bind = rng.next().is_multiple_of(3) && !(dec && i + 1 == n0u);
            let other: u32 = rd(&t.b, c + 0x19c);
            t.put_u32(
                c + 0x19c,
                if bind {
                    chr
                } else {
                    other ^ u32::from(other == chr)
                },
            );
            reach[6] += u32::from(bind && rd::<u8>(&t.b, c + 2) != 0 && c != chr);
        }
        let v = FLOATS[rng.next() as usize % FLOATS.len()];
        let frames = INTS[rng.next() as usize % INTS.len()];
        t.check(
            &format!("Ch_SetShadeIDRelation {round}"),
            0x0082_b7a0,
            &[chr, v, frames],
            |m| {
                Res::Int(c::ch_set_shade_id_relation(
                    m,
                    chr,
                    fl(v),
                    frames,
                    &mut |m, f, a| call(m, f, a),
                ))
            },
        );
        t.put_u32(count, n0.cast_unsigned());
        // Ch_IdShadeCalc: ids off (detached or not), on with and without
        // flag bit 18; Ch_SetShadeID a plain logger
        log_dec_stub(t, &fk, 0x0082_b720, 3, None);
        t.put(
            t.a.rebase(id::ID_SHADE_ON.sym),
            &[[0u8, 1, 0xff][round as usize % 3]],
        );
        t.put_u32(t.a.rebase(0x012f_b7d8), [0, 0, 1, 2][round as usize % 4]);
        // the vector's w, which the original leaves as its stack had it
        t.b.write(frame + 0xc, &1f32.to_bits().to_le_bytes())
            .unwrap();
        let flags: u32 = rd(&t.b, chr + l::FLAGS);
        t.put_u32(chr + l::FLAGS, flags ^ (rng.next() & 0x4_0000));
        for off in [0x3cu32, 0x8c, 0x9c, 0xac] {
            for k in 0..3 {
                let x = match rng.next() % 8 {
                    0 => FLOATS[rng.next() as usize % FLOATS.len()],
                    1 => 0,
                    #[allow(clippy::cast_precision_loss)]
                    _ => ((rng.next() % 2000) as f32 - 1000.0).to_bits(),
                };
                t.put_u32(chr + off + 4 * k, x);
            }
        }
        t.put_u32(chr + l::PID, rng.next());
        let on = rd::<i8>(&t.b, t.b.rebase(id::ID_SHADE_ON.sym)) != 0;
        reach[usize::from(on)] += 1;
        reach[2] += u32::from(on && rd::<u32>(&t.b, chr + l::FLAGS) & 0x4_0000 != 0);
        t.check(
            &format!("Ch_IdShadeCalc {round}"),
            0x0083_4110,
            &[chr],
            |m| {
                id::ch_id_shade_calc(
                    m,
                    chr,
                    frame,
                    &mut |m, f, a| call(m, f, a),
                    &mut |m, f, a| fcall(m, f, a),
                );
                Res::Void
            },
        );
        // Ch_ModelSetAlpha: a made-up model list
        let nm = 1 + rng.next() % 4;
        let hdr = t.alloc(8);
        t.put(hdr + 6, &u16::try_from(nm).unwrap().to_le_bytes());
        let models = random_block(t, &mut rng, 0x38 * 4);
        let (old_hdr, old_models): (u32, u32) = (rd(&t.b, chr + 0x1c0), rd(&t.b, chr + 0x1bc));
        t.put_u32(chr + 0x1c0, hdr);
        t.put_u32(chr + 0x1bc, models);
        // the error message's name
        let old_name: u32 = rd(&t.b, chr + 4);
        let name = t.alloc(16);
        t.put(name, b"c001\0");
        t.put_u32(chr + 4, name);
        let i = [0, 1, nm - 1, nm, nm + 3, u32::MAX][rng.next() as usize % 6];
        let a = FLOATS[rng.next() as usize % FLOATS.len()];
        reach[3] += u32::from(i < nm);
        t.check(
            &format!("Ch_ModelSetAlpha {round}"),
            0x0082_ae30,
            &[chr, i, a],
            |m| {
                Res::Int(c::ch_model_set_alpha(m, chr, i, fl(a), &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        t.put_u32(chr + 0x1c0, old_hdr);
        t.put_u32(chr + 0x1bc, old_models);
        t.put_u32(chr + 4, old_name);
        // the footstep effect: feet 7 and 8 set, empty, NaN
        for k in [7u32, 8] {
            for c in 0..4 {
                let x = match rng.next() % 6 {
                    0 => 0,
                    1 => FLOATS[rng.next() as usize % FLOATS.len()],
                    #[allow(clippy::cast_precision_loss)]
                    _ => ((rng.next() % 200) as f32 - 100.0).to_bits(),
                };
                t.put_u32(chr + 0x524 + 16 * k + 4 * c, x);
            }
        }
        t.check(&format!("foot effect {round}"), 0x0083_38d0, &[chr], |m| {
            Res::Int(ffx_chara::attach::bridge::ch_sound_foot_effect(
                m,
                chr,
                frame,
                &mut |m, f, a| call(m, f, a),
            ))
        });
        // Chb_AttachMotionTBL_OLD: no tables, either, both; built for
        // their own address or another
        let blk = random_block(t, &mut rng, l::BLK_SIZE);
        t.put(blk + 0xb8, b"/ffx/proj2/chr/prot/mon/m042\0");
        let tables = t.alloc(0x200);
        let (t1, t2) = (tables, tables + 0x100);
        t.put(tables, &random(&mut rng, 0x200));
        for (tb, moved) in [(t1, rng.next() % 4), (t2, rng.next() % 4)] {
            let first = match moved {
                0 => tb,
                1 => tb - 1,
                _ => rng.next(),
            };
            t.put_u32(tb, first);
        }
        let n = [0u32, 1, 5, 0x30, u32::MAX][rng.next() as usize % 5];
        t.put_u32(t2 + 4, n);
        let (first, second) = match round % 4 {
            0 => (0, 0),
            1 => (t1, 0),
            2 => (0, t2),
            _ => (t1, t2),
        };
        t.put_u32(reads, first);
        // the stand-in adds `step` after the first call
        cstr_stub(t, &fk, 0x0083_ae30, 0, reads, second.wrapping_sub(first));
        t.check(
            &format!("Chb_AttachMotionTBL_OLD {round}"),
            0x0083_7fa0,
            &[blk],
            |m| {
                h::chb_attach_motion_tbl_old(m, blk, frame, &mut |m, f, a| call(m, f, a));
                Res::Void
            },
        );
        // Ch_RomReadEventMGRP: sizes 0 (Eternal Calm or not), 0x10, others
        t.put_u32(cd[1], [0, 0, 0x10, 0x11, 0x2345][rng.next() as usize % 5]);
        t.put_u32(eternal, [0, 1, 0x100][rng.next() as usize % 3]);
        t.put_u32(eternal_size, [0, 0x10, 0x800][rng.next() as usize % 3]);
        t.put_u32(cd[2], rng.next());
        t.put_u32(malloc, heap);
        let file = [rng.next(), rng.next() % 0x300, 18 * 7, u32::MAX - 20][rng.next() as usize % 4];
        let x = [rng.next(), 0, 7, u32::MAX][rng.next() as usize % 4];
        reach[4] += u32::from(rd::<u32>(&t.b, cd[1]) == 0);
        t.check(
            &format!("Ch_RomReadEventMGRP {round}"),
            0x0082_a6e0,
            &[file, x],
            |m| {
                Res::Int(sy::ch_rom_read_event_mgrp(
                    m,
                    file,
                    x,
                    frame,
                    &mut |m, f, a| call(m, f, a),
                ))
            },
        );
        // Ch_Allocate2: Ch_Allocate a stand-in returning a block
        let block = t.alloc(0x200);
        let ret = fk.fake(t, 0x0082_4f90, 9, 1);
        t.put_u32(ret, block);
        let (key, v) = (rng.next(), rng.next());
        t.check(
            &format!("Ch_Allocate2 {round}"),
            0x0082_5650,
            &[key, v],
            |m| Res::Int(h::ch_allocate2(m, key, v, &mut |m, f, a| call(m, f, a))),
        );
        // Ch_DisposeAll: a new array (or none); Ch_Dispose a logger that
        // at times shortens it; user_free a logger
        let n = rng.next() % 5;
        let arr = if round % 6 == 5 {
            0
        } else {
            t.alloc(l::SIZE as usize * 5)
        };
        if arr != 0 {
            for k in 0..n {
                t.put(
                    arr + k * l::SIZE + 2,
                    &[u8::from(!rng.next().is_multiple_of(3))],
                );
            }
        }
        let (keep_n, keep_a) = (h::CHR_COUNT.get(&t.b), h::CHR_ARRAY.get(&t.b));
        t.put_u32(count, n);
        t.put_u32(array, arr);
        let dec = round % 3 == 1;
        if dec && n > 0 && arr != 0 {
            t.put(arr + (n - 1) * l::SIZE + 2, &[0]);
        }
        let moves = (round % 6 == 4).then_some(array);
        log_dec_move_stub(t, &fk, 0x0082_66f0, 1, dec.then_some((count, flag)), moves);
        t.put_u32(flag, 1);
        t.check(&format!("Ch_DisposeAll {round}"), 0x0082_68e0, &[], |m| {
            h::ch_dispose_all(m, &mut |m, f, a| call(m, f, a));
            Res::Void
        });
        reach[5] += u32::from(arr != 0);
        t.put_u32(count, keep_n.cast_unsigned());
        t.put_u32(array, keep_a);
    }
    eprintln!("{}: cmm1 a reach {reach:?}", t.name);
}

#[test]
fn cmm1_b_matches_the_original() {
    for mut t in twins() {
        cmm1_b_on(&mut t);
    }
}

/// A made-up model header for `chr`: `models` models (of the character's
/// four), `n` skin entries and as many normal entries.
fn skin_header(t: &mut Twin, rng: &mut Xorshift, models: u16, n: u32) -> u32 {
    let hdr = random_block(t, rng, 0x30);
    t.put(hdr + 6, &models.to_le_bytes());
    t.put(hdr + 8, &u16::try_from(n).unwrap().to_le_bytes());
    let list = random_block(t, rng, 12 * n.max(1));
    let normals = random_block(t, rng, 12 * n.max(1));
    let m = u32::from(models.max(1));
    for e in 0..n {
        let r = |rng: &mut Xorshift, k: u32| u16::try_from(rng.next() % k).unwrap();
        let at = list + 12 * e;
        let base = if e > 0 && rng.next().is_multiple_of(2) {
            r(rng, 1)
        } else {
            r(rng, 4)
        };
        for (o, v) in [(0u32, r(rng, 4)), (2, r(rng, m)), (4, base), (6, r(rng, 2))] {
            t.put(at + o, &v.to_le_bytes());
        }
        let at = normals + 12 * e;
        t.put(at + 2, &r(rng, m).to_le_bytes());
        t.put(at + 6, &r(rng, 2).to_le_bytes());
        if rng.next().is_multiple_of(3) {
            t.put_u32(at + 8, 0);
        }
    }
    t.put_u32(hdr + 0x14, list);
    t.put_u32(hdr + 0x24, normals);
    hdr
}

/// Chara-main-map round 1, group b: `Apply_Envelope`,
/// `Ch_DrawMainForSituation`, `Ch_EffectPacketCreateMain`. The matrix
/// math runs as the originals; the skinning kernels, the renderer and the
/// packet builders are logging stand-ins (matrices and vertices logged by
/// their contents: the originals keep them on their stacks).
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // t, c, n, m, e, q
fn cmm1_b_on(t: &mut Twin) {
    let mut rng = Xorshift(0x00c3_b002);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let frame = t.b.stack_range().start + 0x400;
    let (count, array) = (t.a.rebase(h::CHR_COUNT.sym), t.a.rebase(h::CHR_ARRAY.sym));
    let flag = t.alloc(4);
    // Apply_Envelope's callees
    let magic = t.alloc(4);
    let (code, _) = (multi_deref_code(&fk, &[], &[], (magic, 0)), 0);
    jump_to(t, 0x009d_a3b0, &code);
    let malloc = fk.fake_alloc(t, 0x0068_7020, 1, 0x40);
    // the skin cache (4 bytes per entry; a real header has 0xffff at most)
    let heap = t.alloc(0x4_0000);
    for (sym, args) in [
        (0x006e_e570u32, &[0u8, 1, 2][..]),
        (0x006e_d9e0, &[0, 1, 2]),
        (0x006e_ee40, &[0, 1]),
        (0x006e_e270, &[0, 1]),
        (0x0084_5eb0, &[0, 1]),
        (0x0084_5ec0, &[0, 1]),
    ] {
        let m = u8::try_from(args.len()).unwrap();
        let ret = t.alloc(4);
        t.put_u32(ret, 0x5000 + sym % 0x100);
        multi_deref_stub(t, &fk, sym, args, &[(m, 0, 16)], (ret, 3));
    }
    let _ = fk.fake(t, 0x0084_6710, 2, 2); // sg_normalize
    // Ch_DrawMainForSituation's
    multi_deref_stub(t, &fk, 0x0063_c3e0, &[0], &[(1, 0, 16)], (0, 0));
    multi_deref_stub(t, &fk, 0x0063_c430, &[0, 2], &[(1, 0, 16)], (0, 0));
    let _ = fk.fake(t, 0x0063_c500, 3, 1);
    // Ch_EffectPacketCreateMain's
    let _ = fk.fake(t, 0x0084_5ca0, 4, 2);
    for (k, sym) in [
        0x0084_5800u32,
        0x0084_5810,
        0x0084_5c30,
        0x0084_5c40,
        0x0084_4370,
    ]
    .into_iter()
    .enumerate()
    {
        let _ = fk.fake(t, sym, 5 + u32::try_from(k).unwrap(), 1);
    }
    multi_deref_stub(
        t,
        &fk,
        0x0084_5820,
        &[],
        &[(0, 0, 2), (1, 0, 2), (2, 0, 2)],
        (0, 0),
    );
    let _ = fk.fake(t, 0x0084_57d0, 10, 0);
    let detached = t.a.rebase(0x012f_b7d8);
    let chrs = live_characters(t, &mut rng);
    let mut reach = [0u32; 12];
    for round in 0..48u32 {
        fk.reset(t);
        t.put_u32(malloc, heap);
        // ---- Apply_Envelope
        let chr = chrs[rng.next() as usize % chrs.len()];
        let synthetic = t.name == "synthetic" || rng.next().is_multiple_of(2);
        let (old_hdr, old_cache): (u32, u32) = (rd(&t.b, chr + 0x1c0), rd(&t.b, chr + 0x874));
        if synthetic {
            // no more models than the character has
            let real = u32::from(rd::<u16>(&t.b, old_hdr + 6));
            let most = if t.name == "synthetic" {
                4
            } else {
                real.min(4)
            };
            let models = u16::try_from(rng.next() % (most + 1)).unwrap();
            let n = rng.next() % 6;
            let hdr = skin_header(t, &mut rng, models, n);
            t.put_u32(chr + 0x1c0, hdr);
            let cache = if rng.next().is_multiple_of(3) {
                0
            } else {
                let c = random_block(t, &mut rng, 4 * 8);
                for k in 0..8 {
                    if rng.next().is_multiple_of(2) {
                        t.put_u32(c + 4 * k, 0);
                    }
                }
                c
            };
            t.put_u32(chr + 0x874, cache);
        }
        let models: u32 = rd(&t.b, chr + 0x1bc);
        let hdr: u32 = rd(&t.b, chr + 0x1c0);
        let nm = u32::from(rd::<u16>(&t.b, hdr + 6));
        if synthetic {
            for k in 0..nm.min(4) {
                let m = models + k * l::MODEL_SIZE;
                t.put_u32(m + 0x2c, u32::from(!rng.next().is_multiple_of(3)));
                t.put_u32(m + 0x34, rng.next() % 2);
            }
        }
        let ids = [0x1114u16, 0x1063, 0x1064, 0x0001];
        let old_id: u16 = rd(&t.b, chr);
        t.put(chr, &ids[rng.next() as usize % 4].to_le_bytes());
        let ms = [0x1b4u32, 0x1b5, 0x1b6][rng.next() as usize % 3];
        let step = [0u32, 1, u32::MAX][rng.next() as usize % 3];
        t.put_u32(magic, ms);
        // the magic stand-in grows by `step` per call: rebuild it
        let code = multi_deref_code(&fk, &[], &[], (magic, step));
        jump_to(t, 0x009d_a3b0, &code);
        t.put_u32(detached, [0, 0, 1][rng.next() as usize % 3]);
        t.put_u32(
            t.a.rebase(0x0130_09e0),
            u32::from(rng.next().is_multiple_of(2)),
        );
        t.put(
            t.a.rebase(0x0130_09e4),
            &[[0u8, 0, 1, 2, 0xff][rng.next() as usize % 5]],
        );
        let fl: u32 = rd(&t.b, chr + l::FLAGS);
        t.put_u32(chr + l::FLAGS, (fl & !0x19) | (rng.next() & 0x19));
        let all = [0u32, 0, 1, 0x100][rng.next() as usize % 4];
        reach[0] += u32::from(rd::<u32>(&t.b, chr + 0x874) == 0 && all == 0);
        t.check(
            &format!("Apply_Envelope {round}"),
            0x0083_2110,
            &[chr, all],
            |m| {
                ffx_chara::draw::bridge::apply_envelope(m, chr, all, frame, &mut |m, f, a| {
                    call(m, f, a)
                });
                Res::Void
            },
        );
        t.put(chr, &old_id.to_le_bytes());
        t.put_u32(chr + 0x1c0, old_hdr);
        t.put_u32(chr + 0x874, old_cache);
        // ---- Ch_DrawMainForSituation over the whole array
        let n0 = u32::try_from(h::CHR_COUNT.get(&t.b)).unwrap();
        let a0 = h::CHR_ARRAY.get(&t.b);
        let dec = round % 4 == 3;
        let mut saved = Vec::new();
        for i in 0..n0 {
            let c = a0 + i * l::SIZE;
            let blk: u32 = rd(&t.b, c + l::BLK);
            let made = t.name == "synthetic" || blk == 0 || blk == 0xcdcd_cdcd;
            let keep = Oracle::read(&mut t.b, c, l::SIZE as usize).unwrap();
            saved.push((c, keep));
            if !made && rng.next().is_multiple_of(2) {
                continue; // a real character as it is
            }
            if made {
                let mc = make_character(t, &mut rng, None);
                let bytes = Oracle::read(&mut t.b, mc, l::SIZE as usize).unwrap();
                t.put(c, &bytes);
                let models = u16::try_from(rng.next() % 5).unwrap();
                let n = rng.next() % 6;
                let hdr = skin_header(t, &mut rng, models, n);
                t.put_u32(c + 0x1c0, hdr);
            }
            t.put(
                c + 2,
                &[u8::from(
                    !(rng.next().is_multiple_of(4) || (dec && i + 1 == n0)),
                )],
            );
            t.put(c + 0x180, &[u8::from(rng.next().is_multiple_of(5))]);
            let f2: u32 = rd(&t.b, c + 0x198);
            t.put_u32(
                c + 0x198,
                (f2 & !0x20)
                    | (rng.next() & 0x20 & u32::from(rng.next().is_multiple_of(4)).wrapping_neg()),
            );
            let alpha =
                [0x3f80_0000u32, 0, 0x8000_0000, 0x7fc0_0000, 0xbf80_0000][rng.next() as usize % 5];
            t.put_u32(c + 0x354, alpha);
            if rng.next().is_multiple_of(4) {
                t.put_u32(c + 0x830, 0);
            }
        }
        log_dec_stub(t, &fk, 0x0083_14c0, 1, dec.then_some((count, flag)));
        t.put_u32(flag, 1);
        t.check(
            &format!("Ch_DrawMainForSituation {round}"),
            0x0082_fe00,
            &[],
            |m| {
                ffx_chara::draw::bridge::ch_draw_main_for_situation(m, frame, &mut |m, f, a| {
                    call(m, f, a)
                });
                Res::Void
            },
        );
        for (c, keep) in saved {
            t.put(c, &keep);
        }
        t.put_u32(count, n0);
        t.put_u32(array, a0);
        // ---- Ch_EffectPacketCreateMain on made-up model data
        let p = t.alloc(12);
        let hdr = random_block(t, &mut rng, 0x20);
        let ng = rng.next() % 7;
        t.put(hdr + 6, &u16::try_from(ng).unwrap().to_le_bytes());
        let groups = random_block(t, &mut rng, 12 * ng.max(1));
        for g in 0..ng {
            let e = groups + 12 * g;
            let word = [0u8, 1, 4, 5, 0xff, 0x13][rng.next() as usize % 6];
            let pick = |rng: &mut Xorshift| [0u8, 1, 0xff][rng.next() as usize % 3];
            let (mat, st) = (pick(&mut rng), pick(&mut rng));
            t.put(e, &[word, mat, st]);
            // a triangle count past a byte at times
            let n = if word & 1 == 0 && rng.next().is_multiple_of(6) {
                0x102
            } else {
                rng.next() % 10
            };
            t.put(e + 4, &u16::try_from(n).unwrap().to_le_bytes());
            let data = if word & 1 != 0 {
                // strip blocks of 1-2 x 16 bytes: point offset, strip count, lengths
                let d = t.alloc(0x200);
                let mut q = d;
                for _ in 0..n {
                    let size = 1 + rng.next() % 2;
                    let strips = rng.next() % 4;
                    t.put(
                        q,
                        &[
                            u8::try_from(size).unwrap(),
                            0x10,
                            u8::try_from(strips).unwrap(),
                            0,
                        ],
                    );
                    for j in 0..4 {
                        t.put(q + 4 + j, &[u8::try_from(rng.next() % 3).unwrap()]);
                    }
                    q += size * 16;
                    reach[1] += strips;
                }
                d
            } else {
                // triangles behind 0x200 bytes the back references reach
                let len = 0x18 * (n / 3).max(1);
                let d = random_block(t, &mut rng, 0x200 + len) + 0x200;
                for k in 0..len / 2 {
                    let w = rng.next();
                    let v = if w.is_multiple_of(3) {
                        0x8000 | ((w >> 8) % 0x60)
                    } else {
                        w >> 16 & 0x7fff
                    };
                    t.put(d + 2 * k, &u16::try_from(v).unwrap().to_le_bytes());
                }
                reach[2] += n / 3;
                d
            };
            t.put_u32(e + 8, data);
        }
        t.put_u32(hdr + 0x10, groups);
        t.put_u32(p, hdr);
        t.put_u32(p + 4, rng.next());
        t.put_u32(p + 8, rng.next());
        t.check(
            &format!("Ch_EffectPacketCreateMain {round}"),
            0x0084_2c80,
            &[p],
            |m| {
                sy::ch_effect_packet_create_main(m, p, frame, &mut |m, f, a| call(m, f, a));
                Res::Void
            },
        );
    }
    eprintln!("{}: cmm1 b reach {reach:?}", t.name);
}

#[test]
fn cmm1_c_matches_the_original() {
    for mut t in twins() {
        cmm1_c_on(&mut t);
    }
}

/// Chara-main-map round 1, group c: `Ch_WorkRestore` (random saved
/// copies, bound or not; the event handle lookup a logging stand-in) and
/// `Ch_PLightCalc` (made-up point lights around the character: enabled or
/// not, inside their radius or not, NaN positions, exponents 0, ±, large,
/// NaN; `_CIpow` and the VU0 math as the originals, the renderer a
/// stand-in logging both matrices).
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // t, c, p, d, e
fn cmm1_c_on(t: &mut Twin) {
    let mut rng = Xorshift(0x00c3_c003);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let frame = t.b.stack_range().start + 0x400;
    let handle = fk.fake(t, 0x0086_b3a0, 1, 1);
    multi_deref_stub(t, &fk, 0x0063_cb80, &[0], &[(1, 0, 16), (2, 0, 16)], (0, 0));
    let ci2 = twin_code(t, &CI2_STUB);
    let chrs = live_characters(t, &mut rng);
    let (lights_g, count_g) = (t.a.rebase(0x0130_08cc), t.a.rebase(0x0130_08d0));
    let detached = t.a.rebase(0x012f_b7d8);
    let mut reach = [0u32; 8];
    for round in 0..80u32 {
        fk.reset(t);
        // ---- Ch_WorkRestore
        let chr = chrs[rng.next() as usize % chrs.len()];
        let keep = Oracle::read(&mut t.b, chr, l::SIZE as usize).unwrap();
        let saved = random_block(t, &mut rng, l::SIZE);
        let bound = if rng.next().is_multiple_of(3) {
            u32::MAX
        } else {
            rng.next() % 0x40
        };
        t.put_u32(saved + 0x1b8, bound);
        t.put_u32(handle, rng.next());
        reach[usize::from(bound == u32::MAX)] += 1;
        t.check(
            &format!("Ch_WorkRestore {round}"),
            0x0082_8620,
            &[chr, saved],
            |m| {
                Res::Int(c::ch_work_restore(m, chr, saved, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        t.put(chr, &keep);
        // ---- Ch_PLightCalc
        let n = rng.next() % 6;
        let lights = random_block(t, &mut rng, 0x2c * n.max(1));
        let at = |k: u32| chr + 0x6c4 + 4 * k;
        let centre: Vec<f32> = (0..3).map(|k| fl(rd::<u32>(&t.b, at(k)))).collect();
        let centre: Vec<f32> = centre
            .iter()
            .map(|c| {
                if c.is_finite() && c.abs() < 1e6 {
                    *c
                } else {
                    0.0
                }
            })
            .collect();
        for k in 0..3 {
            t.put_u32(at(k), centre[k as usize].to_bits());
        }
        for i in 0..n {
            let p = lights + 0x2c * i;
            #[allow(clippy::cast_precision_loss)]
            let radius = (rng.next() % 500) as f32;
            // anywhere around, on the character, exactly on the radius
            let mode = rng.next() % 6;
            for k in 0..3 {
                #[allow(clippy::cast_precision_loss)]
                let off = (rng.next() % 400) as f32 - 200.0;
                let mut v = match mode {
                    1 if k == 0 => centre[0] + radius,
                    0 | 1 => centre[k as usize],
                    _ => centre[k as usize] + off,
                };
                if rng.next().is_multiple_of(10) {
                    v = f32::NAN;
                }
                t.put_u32(p + 4 * k, v.to_bits());
            }
            t.put_u32(p + 0x20, radius.to_bits());
            let exp = [0f32, 1.0, 2.0, 0.5, -1.5, f32::NAN, 2e6, -0.0, -3.0, -0.75]
                [rng.next() as usize % 10];
            t.put_u32(p + 0x24, exp.to_bits());
            t.put(p + 0x28, &[u8::from(!rng.next().is_multiple_of(4))]);
            t.put(p + 0x29, &[[0u8, 1, 2, 3][rng.next() as usize % 4]]);
            for k in 0..3 {
                #[allow(clippy::cast_precision_loss)]
                let c = (rng.next() % 300) as f32 / 100.0;
                t.put_u32(p + 0x10 + 4 * k, c.to_bits());
            }
        }
        // the tunables: the limit and the largest colour component
        let mut limit = [999_999f32, 2.0, 0.5][rng.next() as usize % 3];
        if n > 0 && round % 2 == 1 {
            // one light whose position, exponent or attenuation alone
            // decides: z NaN (exponent 0); on its radius with exponent
            // -0.75 (attenuation 0) against 0.5; at 0.4 of it with -1.5
            // (attenuation about -2.95) against 1.6
            let p = lights;
            let r: f32 = fl(rd(&t.b, p + 0x20)).max(10.0);
            t.put_u32(p + 0x20, r.to_bits());
            let (x, z, e, lim) = match round % 8 {
                1 => (r * 0.5, f32::NAN, 0f32, 999_999f32),
                3 => (r, 0.0, -0.75, 0.5),
                _ => (r * 0.4, 0.0, -1.5, 1.6),
            };
            t.put_u32(p, (centre[0] + x).to_bits());
            t.put_u32(p + 4, centre[1].to_bits());
            t.put_u32(p + 8, (centre[2] + z).to_bits());
            t.put_u32(p + 0x24, e.to_bits());
            t.put(p + 0x28, &[1]);
            for i in 1..n {
                t.put(lights + 0x2c * i + 0x28, &[0]);
            }
            limit = lim;
        }
        t.put_u32(t.a.rebase(0x00c4_9714), limit.to_bits());
        let most = [2f32, 0.5][rng.next() as usize % 2];
        t.put_u32(t.a.rebase(0x00c4_9700), most.to_bits());
        t.put_u32(lights_g, lights);
        let shown = if round % 2 == 1 {
            n
        } else {
            [n, n, 0, u32::MAX][rng.next() as usize % 4]
        };
        t.put_u32(count_g, shown);
        t.put_u32(detached, u32::from(round % 5 == 4));
        t.check(
            &format!("Ch_PLightCalc {round}"),
            0x0083_05a0,
            &[chr],
            |m| {
                let mut call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
                let mut fcall = |m: &mut Oracle, f: u32, a: &[u32]| m.call_f64(f, a).unwrap();
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
                    f32::from_bits(m.call(ci2, Conv::Cdecl, &words).unwrap().eax)
                };
                let mut c = ffx_chara::draw::bridge::PLightCalls {
                    call: &mut call,
                    fcall: &mut fcall,
                    ci: &mut ci,
                };
                ffx_chara::draw::bridge::ch_plight_calc(m, chr, frame, &mut c);
                Res::Void
            },
        );
        t.put(chr, &keep);
        reach[2] += n;
    }
    eprintln!("{}: cmm1 c reach {reach:?}", t.name);
}

// ---- chara-main-map round 2

#[test]
fn cmm2_a_matches_the_original() {
    for mut t in twins() {
        cmm2_a_on(&mut t);
    }
}

/// A stand-in at `sym` that logs its argument words `args` (0-based) and
/// copies the C string of argument `src` into the buffer of argument
/// `dst` (`sprintf` without the formatting); returns 0.
fn copy_str_stub(t: &mut Twin, fk: &Fakes, sym: Option<u32>, args: &[u8], dst: u8, src: u8) -> u32 {
    let mut c = vec![0x8b, 0x0d];
    c.extend(fk.log.to_le_bytes()); // mov ecx, [log]
    for &i in args {
        // mov eax, [esp + 4 + 4i]; mov [ecx], eax; add ecx, 4
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * i, 0x89, 0x01, 0x83, 0xc1, 0x04]);
    }
    c.extend([0x89, 0x0d]);
    c.extend(fk.log.to_le_bytes()); // mov [log], ecx
    c.extend([0x8b, 0x54, 0x24, 4 + 4 * src]); // mov edx, [esp + 4 + 4src]
    c.extend([0x8b, 0x4c, 0x24, 4 + 4 * dst]); // mov ecx, [esp + 4 + 4dst]
    // mov al, [edx]; mov [ecx], al; inc edx; inc ecx; test al, al; jnz back
    c.extend([0x8a, 0x02, 0x88, 0x01, 0x42, 0x41, 0x84, 0xc0, 0x75, 0xf6]);
    c.extend([0x33, 0xc0, 0xc3]); // xor eax, eax; ret
    match sym {
        Some(s) => {
            jump_to(t, s, &c);
            0
        }
        None => twin_code(t, &c),
    }
}

/// Chara-main-map round 2, group a: `Ch_ModelCalcSizeEffectPacket`,
/// `Ch_ModelCreateEffectPacket`, `Ch_ReadEventMgrpAsync_00837650`,
/// `Ch_Sound` (all of its jump table and both high bytes),
/// `Ch_DrawMainEffect`, `Ch_Cloth_PerCharacter` (a callee shortening the
/// count) and `Ch_InitAll` (debug on, off, and switched off mid-walk).
/// Callees are logging stand-ins, except the event-jump and motion-index
/// lookups, which run as the originals.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn cmm2_a_on(t: &mut Twin) {
    use ffx_chara::attach::bridge as at;
    use ffx_chara::draw::bridge as dr;
    use ffx_chara::motion::bridge as mo;
    let mut rng = Xorshift(0x00c3_a201);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let frame = t.b.stack_range().start + 0x400;
    let chrs = live_characters(t, &mut rng);
    // the effect packet's callees
    let _ = fk.fake(t, 0x0084_5ca0, 1, 2); // sg_setvtex_and_normal
    let _ = fk.fake(t, 0x0083_13f0, 2, 1); // Ch_TextureSetTex0
    let set_ptr = fk.fake(t, 0x0084_5710, 3, 1);
    let create = fk.fake(t, 0x0084_2c80, 4, 1);
    let size = fk.fake(t, 0x0084_5700, 5, 0);
    // Ch_RomReadEventMGRP
    let mgrp = fk.fake(t, 0x0082_a6e0, 6, 2);
    // Ch_Sound's callees
    let sound = fk.fake(t, 0x0083_d900, 7, 2); // Ch_IdGetSound (and the left word)
    let _ = fk.fake(t, 0x0088_66b0, 8, 4); // SndKickFoot
    let _ = fk.fake(t, 0x0083_38d0, 9, 1); // the footstep effect
    let dump = fk.fake(t, 0x0083_3f50, 10, 0); // Ch_GetDumpMotEv
    // dbgPrintf: as many words as each call pushes
    let (dbg7, _) = fk.fake_code(t, 11, 7);
    let (dbg5, _) = fk.fake_code(t, 24, 5);
    let (dbg2, _) = fk.fake_code(t, 25, 2);
    let _ = fk.fake(t, 0x0079_fe30, 12, 2); // MsChrSePlayAlive
    // Ch_DrawMainEffect's
    let situation = fk.fake(t, 0x0083_0cd0, 13, 1);
    let magic = fk.fake(t, 0x009d_a3b0, 14, 0);
    let _ = fk.fake(t, 0x0082_fe00, 15, 0);
    // Ch_Cloth_PerCharacter's: Sg_ClothStep shortens the count at times
    let _ = fk.fake(t, 0x0083_2110, 16, 2);
    let _ = fk.fake(t, 0x0082_dbc0, 17, 1);
    let flag = t.alloc(4);
    let count = t.alloc(4);
    let step = twin_code(t, &log_dec_move_code(&fk, 1, None, None));
    let step_dec = twin_code(t, &log_dec_move_code(&fk, 1, Some((count, flag)), None));
    // Ch_InitAll's
    let _ = fk.fake(t, 0x0082_a5f0, 18, 0); // Ch_RomDataReadInit
    let _ = fk.fake(t, 0x0082_ed30, 19, 0); // Ch_DrawInitAll
    let sprintf = copy_str_stub(t, &fk, None, &[2], 0, 1);
    let slot = t.a.rebase(0x00b0_c3dc);
    let reads = t.alloc(4);
    cstr_stub(t, &fk, 0x0083_ae50, 0, reads, 0x40); // Sg_PcReadD
    let len = fk.fake(t, 0x0083_ae70, 20, 0);
    let _ = fk.fake(t, 0x0082_68e0, 21, 0); // Ch_DisposeAll
    let _ = fk.fake(t, 0x0083_60b0, 22, 0); // Sg_InitLight
    let _ = fk.fake(t, 0x0083_f780, 23, 0); // Sg_InitCamera
    let debug = t.a.rebase(0x0133_c910);
    let detached = t.a.rebase(0x012f_b7d8);
    let enable = t.a.rebase(0x0130_07e8);
    let name = twin_code(t, &log_dec_move_code(&fk, 1, None, None));
    let name_dec = twin_code(t, &log_dec_move_code(&fk, 1, Some((debug, flag)), None));
    let mut reach = [0u32; 24];
    for round in 0..96u32 {
        fk.reset(t);
        let chr = chrs[rng.next() as usize % chrs.len()];
        // ---- the effect packets: a made-up model table
        // the table in the middle of its block: negative indices read it too
        let models = random_block(t, &mut rng, 0x38 * 6) + 0x38 * 2;
        let keep_models: u32 = rd(&t.b, chr + 0x1bc);
        t.put_u32(chr + 0x1bc, models);
        let i = [0, 1, 3, u32::MAX, u32::MAX - 1][rng.next() as usize % 5];
        for r in [set_ptr, create, size] {
            t.put_u32(r, rng.next());
        }
        t.check(
            &format!("Ch_ModelCalcSizeEffectPacket {round}"),
            0x0083_0450,
            &[chr, i],
            |m| {
                Res::Int(sy::ch_model_calc_size_effect_packet(
                    m,
                    chr,
                    i,
                    &mut |m, f, a| call(m, f, a),
                ))
            },
        );
        t.put(enable, &[[0u8, 1, 0xff][round as usize % 3]]);
        let p = rng.next();
        t.check(
            &format!("Ch_ModelCreateEffectPacket {round}"),
            0x0083_0490,
            &[chr, i, p],
            |m| {
                Res::Int(sy::ch_model_create_effect_packet(
                    m,
                    chr,
                    i,
                    p,
                    &mut |m, f, a| call(m, f, a),
                ))
            },
        );
        t.put_u32(chr + 0x1bc, keep_models);
        // ---- the asynchronous event motion group
        t.put_u32(mgrp, rng.next());
        let x = [rng.next(), 0, 1, u32::MAX][rng.next() as usize % 4];
        t.check(
            &format!("Ch_ReadEventMgrpAsync {round}"),
            0x0083_7650,
            &[x],
            |m| {
                Res::Int(mo::ch_read_event_mgrp_async(m, x, &mut |m, f, a| {
                    call(m, f, a)
                }))
            },
        );
        // ---- Ch_Sound: every low byte 0..=0x12, high bytes 0, 1, 2, 0x81,
        // 0xff; four events a round
        for k in 0..4u32 {
            let low = (round * 4 + k) % 0x13;
            let high = [0u32, 0, 0, 0, 1, 2, 0x81, 0xff][rng.next() as usize % 8];
            let ev = low | high << 8 | (rng.next() & 0xffff_0000);
            t.put_u32(chr + 0x82c, rng.next());
            let fl194: u32 = rd(&t.b, chr + 0x194);
            t.put_u32(chr + 0x194, fl194 ^ (rng.next() & 0x2000_0000));
            t.put_u32(sound, rng.next());
            t.put_u32(dump, [0, 1, 0x100][rng.next() as usize % 3]);
            reach[(high & 3) as usize] += 1;
            reach[4] +=
                u32::from(rd::<u32>(&t.b, dump) != 0 && high == 0 && (low == 4 || low >= 0xc));
            reach[11] +=
                u32::from(high == 0 && low == 4 && rd::<u32>(&t.b, chr + 0x82c) & 0x18 == 0x18);
            point_to(t, 0x0062_f500, if low == 4 { dbg7 } else { dbg5 });
            t.check(&format!("Ch_Sound {ev:#x}"), 0x0083_5860, &[chr, ev], |m| {
                at::ch_sound(m, chr, ev, &mut |m, f, a| call(m, f, a));
                Res::Void
            });
        }
        // ---- Ch_DrawMainEffect
        t.put_u32(situation, rng.next());
        t.put_u32(
            magic,
            [0x23d, 0x23c, 0x23e, 0x1_023d][rng.next() as usize % 4],
        );
        let v = rng.next();
        reach[5] += u32::from(rd::<u32>(&t.b, magic) == 0x23d);
        t.check(
            &format!("Ch_DrawMainEffect {round}"),
            0x0082_6950,
            &[v],
            |m| Res::Int(dr::ch_draw_main_effect(m, v, &mut |m, f, a| call(m, f, a))),
        );
        // ---- Ch_Cloth_PerCharacter: up to 5 pointers, counts 0, -1, n
        let n = rng.next() % 6;
        let ptrs = random_block(t, &mut rng, 4 * 6);
        let c = [n, 0, u32::MAX, n][round as usize % 4];
        t.put_u32(count, c);
        let dec = round % 3 == 2 && c != 0 && c != u32::MAX;
        point_to(t, 0x0082_2de0, if dec { step_dec } else { step });
        t.put_u32(flag, 1);
        t.put_u32(detached, [0, 0, 1, 2][rng.next() as usize % 4]);
        reach[6] += u32::from(dec);
        t.check(
            &format!("Ch_Cloth_PerCharacter {round}"),
            0x0082_e390,
            &[ptrs, count],
            |m| {
                dr::ch_cloth_per_character(m, ptrs, count, &mut |m, f, a| call(m, f, a));
                Res::Void
            },
        );
        // ---- Ch_InitAll: debug off, on, or switched off by sg_getdbname
        let blk = t.a.rebase(h::CHR_BLK.sym);
        let keep_blk = Oracle::read(&mut t.b, blk, 12000).unwrap();
        let (keep_n, keep_a) = (h::CHR_COUNT.get(&t.b), h::CHR_ARRAY.get(&t.b));
        let keep_slot: u32 = rd(&t.b, slot);
        t.put_u32(slot, sprintf);
        // stale blocks and counts, so every clear shows
        t.put(blk, &random(&mut rng, 12000));
        t.put_u32(t.a.rebase(0x012f_bca4), rng.next());
        let mode = round % 4;
        t.put_u32(debug, [0, 1, 2, 1][mode as usize]);
        point_to(t, 0x0082_9c90, if mode == 3 { name_dec } else { name });
        point_to(t, 0x0062_f500, dbg2);
        t.put_u32(flag, 1);
        t.put_u32(reads, rng.next());
        t.put_u32(len, rng.next());
        reach[7 + mode as usize] += 1;
        t.check(&format!("Ch_InitAll {round}"), 0x0082_6ea0, &[], |m| {
            h::ch_init_all(m, frame, &mut |m, f, a| call(m, f, a));
            Res::Void
        });
        t.put(blk, &keep_blk);
        t.put_u32(t.a.rebase(h::CHR_COUNT.sym), keep_n.cast_unsigned());
        t.put_u32(t.a.rebase(h::CHR_ARRAY.sym), keep_a);
        t.put_u32(slot, keep_slot);
        t.put_u32(debug, 0);
    }
    eprintln!("{}: cmm2 a reach {reach:?}", t.name);
}

/// A stand-in at `sym` that logs its first `n` arguments and returns bit 0
/// of the word at `bits`, rotating it left by one each call.
fn bits_stub(t: &mut Twin, fk: &Fakes, sym: u32, n: u8, bits: u32) {
    let mut c = vec![0x8b, 0x0d];
    c.extend(fk.log.to_le_bytes()); // mov ecx, [log]
    for k in 0..n {
        // mov eax, [esp + 4 + 4k]; mov [ecx + 4k], eax
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    c.extend([0x83, 0x05]);
    c.extend(fk.log.to_le_bytes());
    c.push(4 * n); // add dword [log], 4n
    c.push(0xa1);
    c.extend(bits.to_le_bytes()); // mov eax, [bits]
    c.extend([0xd1, 0x05]);
    c.extend(bits.to_le_bytes()); // rol dword [bits], 1
    c.extend([0x83, 0xe0, 0x01, 0xc3]); // and eax, 1; ret
    jump_to(t, sym, &c);
}

#[test]
fn cmm2_b_matches_the_original() {
    for mut t in twins() {
        cmm2_b_on(&mut t);
    }
}

/// Chara-main-map round 2, group b: `InsideTriangle` on made-up models:
/// cloth groups of up to 0x301 triangles over small integer vertices (at
/// times all in one plane through the origin, so the determinant is 0),
/// through a group matrix or not, `passThisTriangle` a stand-in taking
/// triangles by the bits of a random word, positions with NaNs and
/// infinities now and then; no model, no group, no group table, and the
/// signed distance asked for or not. The vector callees run as the
/// originals.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn cmm2_b_on(t: &mut Twin) {
    let mut rng = Xorshift(0x00c3_b201);
    let fk = Fakes::new(t);
    let bits = t.alloc(4);
    bits_stub(t, &fk, 0x0082_9870, 5, bits);
    let _ = fk.fake(t, 0x0062_f500, 1, 4); // dbgPrintf
    let small = |rng: &mut Xorshift, span: u32| {
        #[allow(clippy::cast_possible_wrap)]
        let v = (rng.next() % (2 * span + 1)) as i32 - span as i32;
        i16::try_from(v).unwrap()
    };
    // a float in -1..1 with a random mantissa (or one of FLOATS, 1 in 16)
    let wild = |rng: &mut Xorshift, scale: f32| {
        if rng.next().is_multiple_of(16) {
            FLOATS[rng.next() as usize % FLOATS.len()]
        } else {
            #[allow(clippy::cast_possible_truncation)]
            let x = (f64::from(rng.next()) / f64::from(u32::MAX) * 2.0 - 1.0) as f32 * scale;
            x.to_bits()
        }
    };
    let mut reach = [0u32; 8];
    for round in 0..200u32 {
        fk.reset(t);
        let chr = random_block(t, &mut rng, l::SIZE);
        let name = t.alloc(8);
        t.put(name, b"c042\0");
        t.put_u32(chr + 4, name);
        let models = 3u32;
        let hdr = t.alloc(0x10);
        t.put(hdr + 6, &u16::try_from(models).unwrap().to_le_bytes());
        t.put(hdr + 8, &[u8::from(round % 3 != 0), 0]);
        t.put_u32(chr + 0x1c0, hdr);
        let table = t.alloc(0x38 * models as usize);
        t.put_u32(chr + 0x1bc, table);
        let flat = round % 4 == 3;
        // far rounds: the matrix moves everything about 1e6 away, so
        // Cramer's rule cancels (the sums' order shows in the f32 results)
        let far = round % 6 == 5;
        let shift = [
            1.0e6f32 + f32::from_bits(wild(&mut rng, 1.0e5)),
            2.0e6,
            -1.5e6,
        ];
        let span = [3u32, 20, 100][rng.next() as usize % 3];
        let groups = 16u16;
        for m in 0..models {
            let entry = table + 0x38 * m;
            let first = t.alloc(0x20);
            t.put(first + 0x18, &groups.to_le_bytes());
            t.put_u32(entry, first);
            // 12 vertices; the buffer pointer at the seventh, so indices
            // run -6..=5
            let nv = 12u32;
            let vtx = t.alloc(6 * nv as usize);
            for k in 0..nv {
                let x = small(&mut rng, span);
                let y = small(&mut rng, span);
                let z = if flat { 0 } else { small(&mut rng, span) };
                for (o, c) in [x, y, z].into_iter().enumerate() {
                    t.put(
                        vtx + 6 * k + 2 * u32::try_from(o).unwrap(),
                        &c.to_le_bytes(),
                    );
                }
            }
            // the vertices: the model's first word's +8
            t.put_u32(first + 8, vtx + 6 * 6);
            let tbl = if m == 2 {
                0
            } else {
                t.alloc(4 * usize::from(groups))
            };
            t.put_u32(entry + 0x2c, tbl);
            if tbl == 0 {
                continue;
            }
            for gi in 0..u32::from(groups) {
                if gi == 4 {
                    t.put_u32(tbl + 4 * gi, 0);
                    continue;
                }
                let n: i16 = match rng.next() % 8 {
                    0 => 0,
                    1 => -1,
                    2 => 0x301,
                    3 => 1,
                    _ => i16::try_from(2 + rng.next() % 40).unwrap(),
                };
                let tris = u32::try_from(n.clamp(0, 0x300)).unwrap();
                let grp = t.alloc(0x50);
                // the matrix: about the unit matrix, random mantissas (two
                // different NaNs and the like now and then); flat rounds
                // keep the plane through the origin (no translation)
                for r in 0..4u32 {
                    for c in 0..4u32 {
                        let v = if r == 3 && c == 3 {
                            1f32.to_bits()
                        } else if r == 3 && flat {
                            0
                        } else if r == 3 && far {
                            shift[c as usize].to_bits()
                        } else if far && r != c {
                            0
                        } else if r == c {
                            (1.5 + f32::from_bits(wild(&mut rng, 0.5))).to_bits()
                        } else {
                            wild(&mut rng, 1.0)
                        };
                        t.put_u32(grp + 16 * r + 4 * c, v);
                    }
                }
                t.put(grp + 0x40, &n.to_le_bytes());
                let idx = t.alloc(6 * tris as usize + 2);
                let ids = t.alloc(2 * tris as usize + 2);
                for k in 0..tris {
                    for j in 0..3 {
                        let ix = i16::try_from(rng.next() % nv).unwrap() - 6;
                        t.put(idx + 6 * k + 2 * j, &ix.to_le_bytes());
                    }
                    t.put(
                        ids + 2 * k,
                        &u16::try_from(rng.next() & 0xffff).unwrap().to_le_bytes(),
                    );
                }
                t.put_u32(grp + 0x44, idx);
                t.put_u32(grp + 0x48, ids);
                t.put_u32(tbl + 4 * gi, grp);
            }
        }
        // the point: near the vertices, random mantissas
        let pos = t.alloc(16);
        #[allow(clippy::cast_precision_loss)]
        for k in 0..4u32 {
            let x = wild(&mut rng, span as f32 * 1.5);
            let x = if far && k < 3 {
                (shift[k as usize] + f32::from_bits(x)).to_bits()
            } else {
                x
            };
            t.put_u32(pos + 4 * k, x);
        }
        let model = [0u32, 1, 0x1_0000, 2, 3, 0xffff][rng.next() as usize % 6];
        let group = [0u32, 1, 2, 3, 4, 5, 11, 12, 15, 16, 17][rng.next() as usize % 11];
        let code = group + 100 * (rng.next() % 3) + (rng.next() & 0xffff_0000);
        t.put_u32(
            bits,
            [rng.next(), u32::MAX, rng.next() | rng.next()][rng.next() as usize % 3],
        );
        let dist = round % 5 != 4;
        reach[usize::from(flat)] += 1;
        let got = inside_triangle_check(
            t,
            &format!("InsideTriangle {round}"),
            chr,
            pos,
            model,
            code,
            dist,
        );
        reach[2] += u32::from(got != 0xffff);
    }
    // crafted cases: one model, one group, every triangle accepted
    let craft = |t: &mut Twin, what: &str, verts: &[[i16; 3]], matrix: [f32; 4], pos: [f32; 3]| {
        let chr = random_block(t, &mut Xorshift(0x0bad_cafe), l::SIZE);
        let name = t.alloc(8);
        t.put(name, b"c043\0");
        t.put_u32(chr + 4, name);
        let hdr = t.alloc(0x10);
        t.put(hdr + 6, &1u16.to_le_bytes());
        let apply = matrix.iter().any(|&x| x.to_bits() != 1f32.to_bits());
        t.put(hdr + 8, &u16::from(apply).to_le_bytes());
        t.put_u32(chr + 0x1c0, hdr);
        let entry = t.alloc(0x38);
        t.put_u32(chr + 0x1bc, entry);
        let first = t.alloc(0x20);
        t.put(first + 0x18, &1u16.to_le_bytes());
        t.put_u32(entry, first);
        let vtx = t.alloc(6 * verts.len());
        for (k, v) in verts.iter().enumerate() {
            for (o, c) in v.iter().enumerate() {
                t.put(
                    vtx + u32::try_from(6 * k + 2 * o).unwrap(),
                    &c.to_le_bytes(),
                );
            }
        }
        t.put_u32(first + 8, vtx);
        let tbl = t.alloc(4);
        t.put_u32(entry + 0x2c, tbl);
        let grp = t.alloc(0x50);
        for r in 0..4u32 {
            for c in 0..4u32 {
                let v = if r == c { matrix[r as usize] } else { 0.0 };
                t.put_u32(grp + 16 * r + 4 * c, v.to_bits());
            }
        }
        let n = u32::try_from(verts.len() / 3).unwrap();
        t.put(grp + 0x40, &u16::try_from(n).unwrap().to_le_bytes());
        let idx = t.alloc(6 * n as usize);
        let ids = t.alloc(2 * n as usize);
        for k in 0..3 * n {
            t.put(idx + 2 * k, &u16::try_from(k).unwrap().to_le_bytes());
        }
        t.put_u32(grp + 0x44, idx);
        t.put_u32(grp + 0x48, ids);
        t.put_u32(tbl, grp);
        let p = t.alloc(16);
        for (k, x) in pos.iter().enumerate() {
            t.put_u32(p + 4 * u32::try_from(k).unwrap(), x.to_bits());
        }
        t.put_u32(bits, u32::MAX);
        inside_triangle_check(t, what, chr, p, 0, 0, true)
    };
    let unit = [1.0f32; 4];
    // A: the near triangle exactly on both bounds of the first pass
    // (u = f32(1.1), v = f32(-0.1)), a farther one inside (distance 2,
    // not 15 times farther)
    let a = craft(
        t,
        "InsideTriangle bounds",
        &[
            [0, 0, 10],
            [10, 0, 10],
            [0, 10, 10],
            [-5, -5, 13],
            [5, -5, 13],
            [-5, 5, 13],
        ],
        unit,
        [-1.0, 0.0, 11.0],
    );
    // B: one found in the first pass (distance 4), a second-pass winner
    // (distance 1, 15 times nearer), then one nearer than the first only
    let b = craft(
        t,
        "InsideTriangle second pass",
        &[
            [-5, -5, 4],
            [5, -5, 4],
            [-5, 5, 4],
            [2, -5, 1],
            [12, -5, 1],
            [2, 5, 1],
            [2, -5, 2],
            [12, -5, 2],
            [2, 5, 2],
        ],
        unit,
        [0.0, 0.0, 0.0],
    );
    // C: a nearer triangle at u = 1.4, inside the range only from the
    // third pass on
    let c = craft(
        t,
        "InsideTriangle third pass",
        &[
            [-5, -5, 4],
            [5, -5, 4],
            [-5, 5, 4],
            [2, 2, 1],
            [12, 2, 1],
            [2, 12, 1],
        ],
        unit,
        [0.0, 0.0, 0.0],
    );
    // D: a flat triangle scaled by 1e9: n.n overflows, the distance is 0
    // and the coordinates are NaN (which pass)
    let d = craft(
        t,
        "InsideTriangle NaN coordinates",
        &[[1, 1, 0], [9, 2, 0], [3, 8, 0]],
        [1e9, 1e9, 1e9, 1.0],
        [4.0e9, 4.0e9, 0.0],
    );
    // E: the determinant exactly the limit, f32(1e-5)
    let e = craft(
        t,
        "InsideTriangle determinant at the limit",
        &[[1, 0, 0], [0, 1, 0], [0, 0, 1]],
        [1e-5, 1.0, 1.0, 1.0],
        [3.3e-6, 0.3, 0.4],
    );
    eprintln!(
        "{}: cmm2 b reach {reach:?}, crafted {a:#x} {b:#x} {c:#x} {d:#x} {e:#x}",
        t.name
    );
}

/// `InsideTriangle` on both oracles with fresh outputs (`dist` or not);
/// the port's result.
fn inside_triangle_check(
    t: &mut Twin,
    what: &str,
    chr: u32,
    pos: u32,
    model: u32,
    code: u32,
    dist: bool,
) -> u32 {
    use ffx_chara::cloth::bridge as cl;
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let frame = t.b.stack_range().start + 0x400;
    let mut rng = Xorshift(0x0005_eed5 ^ chr);
    let outs = random_block(t, &mut rng, 16);
    // stale candidate tables, so every clear shows
    for (g, n) in [
        (cl::TRI_BARY, 0x3000),
        (cl::TRI_DIST, 0xc00),
        (cl::TRI_SIGNED, 0xc00),
    ] {
        let at = t.a.rebase(g);
        t.put(at, &random(&mut rng, n));
    }
    let out = cl::TriangleOut {
        u: outs,
        v: outs + 4,
        w: outs + 8,
        dist: if dist { outs + 12 } else { 0 },
    };
    let mut got = 0;
    t.check(
        what,
        0x0082_8c10,
        &[chr, pos, model, code, out.u, out.v, out.w, out.dist],
        |m| {
            got = cl::inside_triangle(m, chr, pos, model, code, out, frame, &mut |m, f, a| {
                call(m, f, a)
            });
            Res::Int(got)
        },
    );
    got & 0xffff
}

#[test]
fn cmm2_c_matches_the_original() {
    for mut t in twins() {
        cmm2_c_on(&mut t);
    }
}

/// Chara-main-map round 2, group c: `chr_texanim_main_sub_new` on made-up
/// new-format entries: tracks 1, 2, 0x15 and the entry's own (or none),
/// flag bits 0 (later) and 1 (skipped), work records of every mode (blink
/// states, stepping, scrolling with wraps both ways), empty frame lists,
/// unused records, with and without drawing, Tidus (`c001`, leader alive
/// or not), `s024`'s extra layers and `c043` on map 0xe2. `rand` is a
/// stand-in in its import slot; `TexAnim_SetMaterialsChar` and
/// `SetMaterialUV` are loggers; `strstr` is the C runtime's.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn cmm2_c_on(t: &mut Twin) {
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x00c3_c201);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let fk = Fakes::new(t);
    let _ = fk.fake(t, 0x0077_fbf0, 1, 12); // TexAnim_SetMaterialsChar
    let _ = fk.fake(t, 0x0064_4150, 2, 5); // SetMaterialUV
    // rand: a Weyl sequence's top 15 bits
    let rand_state = t.alloc(4);
    let mut c = vec![0xa1];
    c.extend(rand_state.to_le_bytes()); // mov eax, [rand_state]
    c.extend([0x81, 0x05]);
    c.extend(rand_state.to_le_bytes());
    c.extend(0x9e37_79b9u32.to_le_bytes()); // add dword [rand_state], k
    c.extend([0xc1, 0xe8, 0x11, 0xc3]); // shr eax, 17; ret
    let rand = twin_code(t, &c);
    let slot = t.a.rebase(0x00b0_c420);
    t.put_u32(slot, rand);
    let names = t.alloc(0x40);
    t.put(
        names,
        b"c001\0\0\0\0s024\0\0\0\0pc043x\0\0m099\0\0\0\0c0010\0\0\0s0241\0\0\0",
    );
    let (map, leader) = (t.a.rebase(0x012f_bc40), t.a.rebase(0x00c4_21fc));
    let table = t.a.rebase(ta::TEX_ANIMS.sym);
    let small_f = |rng: &mut Xorshift, span: u32| {
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
        let v = (rng.next() % (2 * span * 8 + 1)) as i32 - (span * 8) as i32;
        #[allow(clippy::cast_precision_loss)]
        (v as f32 / 8.0).to_bits()
    };
    let mut reach = [0u32; 12];
    for round in 0..300u32 {
        fk.reset(t);
        t.put_u32(rand_state, rng.next());
        let i = rng.next() % 4;
        let e = table + 0x30 * i;
        // the source: kind byte, rectangles at +0x20, records at +0x200
        let n_rec = [0u32, 1, 2, 4, 6][rng.next() as usize % 5];
        let src = t.alloc(0x200 + 0x110 * 8 + 0x400);
        t.put(src, &random(&mut rng, 0x200));
        t.put(src + 0x200, &vec![0; 0x110 * 8 + 0x400]);
        let kind = [2u8, 2, 2, 2, 2, 1, 3][rng.next() as usize % 7];
        t.put(src, &[kind]);
        t.put_u32(src + 4, 0x20);
        t.put_u32(src + 8, 0x200);
        for r in 0..16u32 {
            let at = src + 0x20 + 12 * r;
            let w = [0u16, 1, 8, 16, 0xfff0, 32, 3, 24][rng.next() as usize % 8];
            let h = [0u16, 1, 8, 16, 0xfff0, 32, 7, 24][rng.next() as usize % 8];
            t.put_u32(at, rng.next());
            t.put_u32(at + 4, u32::from(w) | u32::from(h) << 16);
            t.put_u32(at + 8, rng.next() % 0x1000);
        }
        // frame lists: 4-byte frames (i16 time, rectangle, wait scale)
        let frames = 0x100u32;
        for f in 0..0x40u32 {
            let time = [0i16, 1, 2, 5, -1, 3][rng.next() as usize % 6];
            let at = src + frames + 4 * f;
            if at + 4 > src + 0x200 {
                break;
            }
            t.put(at, &time.to_le_bytes());
            t.put(
                at + 2,
                &[
                    u8::try_from(rng.next() % 16).unwrap(),
                    u8::try_from(rng.next() & 0xff).unwrap(),
                ],
            );
        }
        let own = [3u32, 5, 0x1d, 1, 2, 0x15, 0xff, 0x103, 0x101, 0x115][rng.next() as usize % 10];
        for k in 0..n_rec {
            let s = src + 0x200 + 0x110 * k;
            t.put(s + 8, &random(&mut rng, 8));
            for track in [1u32, 2, 3, 5, 0x15, 0x1d] {
                let cnt = [0i16, 1, 2, 3, 5, -2][rng.next() as usize % 6];
                let start = frames + 4 * (rng.next() % 8);
                t.put_u32(s + 0x10 + 8 * track, start);
                t.put(s + 0x14 + 8 * track, &cnt.to_le_bytes());
            }
        }
        // the work block: records from +8, unused bytes at +0xd08
        let work = t.alloc(0xd08 + 0x40);
        t.put(work, &random(&mut rng, 0xd08 + 0x40));
        let count = if rng.next().is_multiple_of(6) {
            0
        } else {
            n_rec
        };
        t.put_u32(work, count);
        for k in 0..n_rec {
            let r = work + 8 + 0x34 * k;
            t.put_u32(
                r + 4,
                [0u32, 1, 2, 4, rng.next() % 3, 0, 1, 2][rng.next() as usize % 8],
            );
            let mode = [0i16, 1, 2, 3, 3, 4, 5, -1][rng.next() as usize % 8];
            t.put(r + 0x1c, &mode.to_le_bytes());
            t.put(
                r + 0x1a,
                &[0i16, 1, 2, 3][rng.next() as usize % 4].to_le_bytes(),
            );
            t.put(
                r + 0x18,
                &[0i16, 1, 2, 3, 100, -1][rng.next() as usize % 6].to_le_bytes(),
            );
            t.put(
                r + 0x14,
                &[0i16, 1, 4, 8, -3][rng.next() as usize % 5].to_le_bytes(),
            );
            t.put(
                r + 0x16,
                &[0i16, 1, 2, 7][rng.next() as usize % 4].to_le_bytes(),
            );
            for o in [0x24u32, 0x28, 0x2c, 0x30] {
                t.put_u32(r + o, small_f(&mut rng, 24));
            }
            // scroll sums of exactly 0 (and -0) now and then
            for (sp, p) in [(0x24u32, 0x2c), (0x28, 0x30)] {
                match rng.next() % 6 {
                    0 => {
                        let v: u32 = rd(&t.b, r + sp);
                        t.put_u32(r + p, v ^ 0x8000_0000);
                    }
                    1 => {
                        t.put_u32(r + sp, 0x8000_0000);
                        t.put_u32(r + p, 0x8000_0000);
                    }
                    _ => {}
                }
            }
            t.put(
                work + 0xd08 + k,
                &[[0xffu8, 1, 0xfe, 0, 1][rng.next() as usize % 5]],
            );
            reach[usize::try_from(mode.clamp(0, 5)).unwrap()] += 1;
        }
        // the entry: source, work, own track, speed, flags by track
        t.put_u32(e, if round % 17 == 16 { 0 } else { src });
        t.put_u32(e + 4, work);
        t.put_u32(e + 8, own);
        let speed = [1i8, 2, 3, -1, 0, 5][rng.next() as usize % 6];
        t.put(e + 0xe, &speed.to_le_bytes());
        for track in [1u32, 2, 3, 5, 0x15, 0x1d] {
            let fl = [0u8, 0, 0, 1, 2, 3][rng.next() as usize % 6];
            t.put(e + 0xf + track, &[fl]);
        }
        let name = names + 8 * (rng.next() % 6);
        let on_map = rng.next().is_multiple_of(3);
        t.put_u32(map, if on_map { 0xe2 } else { 0x40 });
        t.put_u32(leader, rng.next() % 2);
        let draw = [0u32, 1, rng.next(), 0x100][rng.next() as usize % 4];
        reach[6] += u32::from(draw != 0 && name == names);
        reach[7] += u32::from(draw != 0 && name == names + 8);
        reach[8] += u32::from(on_map && name == names + 16);
        t.check(
            &format!("chr_texanim_main_sub_new {round}"),
            0x0077_f360,
            &[i, name, draw],
            |m| {
                ta::chr_texanim_main_sub_new(m, i, name, draw, &mut |m, f, a| call(m, f, a));
                Res::Void
            },
        );
    }
    eprintln!("{}: cmm2 c reach {reach:?}", t.name);
}

#[test]
fn small2_a_matches_the_original() {
    for mut t in twins() {
        small2_a_on(&mut t);
    }
}

/// `chr_texanim_main`, `Chr_LoadMotionGroup` and `Ch_MoveMotion_00835bb0`
/// (small round 2, group a).
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
fn small2_a_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mo;
    use ffx_chara::texanim::bridge as ta;
    let mut rng = Xorshift(0x5a11_2a2a);
    let fk = Fakes::new(t);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let mut reach = [0u32; 12];

    // chr_texanim_main: the two steps faked
    let old = fk.fake(t, ta::MAIN_SUB_OLD, 1, 1);
    let new = fk.fake(t, ta::MAIN_SUB_NEW, 2, 3);
    let table = t.a.rebase(0x0231_0000);
    for round in 0..60u32 {
        fk.reset(t);
        t.put_u32(old, rng.next());
        t.put_u32(new, rng.next());
        let i = if round < 50 {
            rng.next() % 64
        } else {
            rng.next() % 0x100
        };
        let kind = [0u8, 1, 2, 0xff, 0x81][(round % 5) as usize];
        reach[usize::from(kind.min(2))] += 1;
        t.put(table + i * 0x30 + 0xd, &[kind]);
        let (name, draw) = (rng.next(), rng.next() % 2);
        t.check(
            &format!("chr_texanim_main {round}"),
            0x0077_c8d0,
            &[i, name, draw],
            |m| {
                ta::chr_texanim_main(m, i, name, draw, &mut |m: &mut Oracle, f, a: &[u32]| {
                    call(m, f, a)
                });
                Res::Void
            },
        );
    }

    // Chr_LoadMotionGroup: the finder and readers faked
    let find = fk.fake(t, 0x0083_6f10, 3, 3);
    let read = fk.fake(t, mo::READ_EVENT_MGRP, 4, 1);
    let read_async = fk.fake(t, mo::READ_EVENT_MGRP_ASYNC, 5, 1);
    let sync = fk.fake(t, mo::READ_SYNC, 6, 1);
    for round in 0..32u32 {
        fk.reset(t);
        let f = if round % 3 == 0 { rng.next() | 1 } else { 0 };
        let r = if rng.next().is_multiple_of(2) {
            0
        } else {
            rng.next()
        };
        t.put_u32(find, f);
        t.put_u32(read, r);
        t.put_u32(read_async, rng.next());
        t.put_u32(sync, rng.next());
        reach[3 + usize::from(f != 0) + usize::from(f == 0 && r != 0) * 2] += 1;
        let n = rng.next();
        t.check(
            &format!("Chr_LoadMotionGroup {round}"),
            0x0083_75f0,
            &[n],
            |m| {
                Res::Int(mo::chr_load_motion_group(
                    m,
                    n,
                    &mut |m: &mut Oracle, f, a: &[u32]| call(m, f, a),
                ))
            },
        );
    }

    // Ch_MoveMotion_00835bb0: the angle functions are the originals
    // the motion setters also set the pending byte, so its reset shows
    let pending_stub = |t: &mut Twin, sym: u32, id: u32, n: u8| {
        let (code, ret) = fk.fake_code(t, id, n);
        let mut c = vec![
            0x8b, 0x44, 0x24, 0x04, 0xc6, 0x80, 0x36, 0x07, 0, 0, 0x5a, 0xe9,
        ];
        let at = t.a.alloc_code(&[0; 16]).unwrap();
        assert_eq!(t.b.alloc_code(&[0; 16]).unwrap(), at);
        c.extend(code.wrapping_sub(at + 16).to_le_bytes());
        t.put(at, &c);
        let f = t.a.rebase(sym);
        let mut jmp = vec![0xe9];
        jmp.extend(at.wrapping_sub(f + 5).to_le_bytes());
        t.put(f, &jmp);
        ret
    };
    let seq = pending_stub(t, 0x0083_7ac0, 7, 2);
    let sys = pending_stub(t, 0x0083_7d00, 8, 3);
    let sound = fk.fake(t, mo::SOUND, 9, 2);
    let set_loop = fk.fake(t, mo::MOTION_SET_LOOP, 10, 2);
    let pop = t.b.rebase(FPU_POP);
    let f = |rng: &mut Xorshift, lo: i32, hi: i32| {
        let span = u32::try_from(hi - lo).unwrap() * 16;
        ((rng.next() % span) as f32 / 16.0 + lo as f32).to_bits()
    };
    for round in 0..96u32 {
        fk.reset(t);
        for w in [seq, sys, sound, set_loop] {
            t.put_u32(w, rng.next());
        }
        let chr = t.alloc(0x800);
        for k in 0..0x200 {
            t.put_u32(chr + 4 * k, rng.next());
        }
        let speed = [0f32, 0.5, 1.0, 2.0, -0.0, f32::NAN][(rng.next() % 6) as usize];
        t.put_u32(chr + 0x154, speed.to_bits());
        t.put_u32(chr + 0x170, f(&mut rng, 0, 2));
        t.put_u32(chr + 0x158, f(&mut rng, -7, 7));
        t.put_u32(chr + 0x168, f(&mut rng, -7, 7));
        t.put(chr + 0x736, &[u8::from(rng.next().is_multiple_of(4))]);
        for off in [0x75cu32, 0x760, 0x764] {
            let low = if rng.next().is_multiple_of(4) {
                0x1000
            } else {
                rng.next() % 0x2000
            };
            let m = (rng.next() & 0xffff_0000) | low;
            t.put_u32(chr + off, m);
        }
        let at = t.a.rebase(0x0083_5bb0);
        t.a.call(at, Conv::Cdecl, &[chr]).unwrap();
        mo::ch_move_motion(
            &mut t.b,
            chr,
            &mut |m: &mut Oracle, f, a: &[u32]| m.call_cdecl(f, a).unwrap(),
            &mut |m: &mut Oracle, f, a: &[u32]| {
                let r = m.call(f, Conv::Cdecl, a).unwrap();
                m.call(pop, Conv::Cdecl, &[]).unwrap();
                r.st0.expect("a float result")
            },
        );
        let flags: u32 = rd(&t.a, chr + 0x194);
        reach[6 + usize::from(flags & 0x0c00_0000 == 0x0400_0000)] += 1;
        assert_eq!(
            t.a.diff(&mut t.b).unwrap(),
            [],
            "{}: Ch_MoveMotion {round}",
            t.name
        );
    }
    eprintln!("{}: small2 a reach {reach:?}", t.name);
}

#[test]
fn small2_b_matches_the_original() {
    for mut t in twins() {
        small2_b_on(&mut t);
    }
}

/// `Ch_MotEvent` (small round 2, group b): random event lists, every
/// callee faked (the callback too), the debug switch on and off.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::similar_names
)]
fn small2_b_on(t: &mut Twin) {
    use ffx_chara::motion::bridge as mo;
    let mut rng = Xorshift(0x5a11_2b2b);
    let fk = Fakes::new(t);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let sound = fk.fake(t, mo::SOUND, 1, 2);
    let tex = fk.fake(t, mo::TEX_ANIM_NO_SET, 2, 5);
    let signal = fk.fake(t, mo::EFFECT_SIGNAL, 3, 3);
    let print = fk.fake(t, 0x0062_f500, 4, 11);
    // the callback (given the character) moves the last frame and drops
    // an event, so both are read again
    let (cb_log, cb_ret) = fk.fake_code(t, 5, 2);
    let mut c = vec![0x8b, 0x44, 0x24, 0x04];
    c.extend([
        0x66, 0xff, 0x80, 0xf2, 0x07, 0, 0, 0x66, 0xff, 0x88, 0xf0, 0x07, 0, 0, 0xe9,
    ]);
    let cb = t.a.alloc_code(&[0; 24]).unwrap();
    assert_eq!(t.b.alloc_code(&[0; 24]).unwrap(), cb);
    c.extend(cb_log.wrapping_sub(cb + 23).to_le_bytes());
    t.put(cb, &c);
    let dump = t.a.rebase(mo::DUMP_MOT_EV);
    let sg_frame = t.a.rebase(0x023c_bbf0);
    let mut reach = [0u32; 10];
    let small = |rng: &mut Xorshift, lo: i32, span: u32| (lo + (rng.next() % span) as i32) as u32;
    for round in 0..160u32 {
        fk.reset(t);
        for w in [sound, tex, signal, print, cb_ret] {
            t.put_u32(w, rng.next());
        }
        t.put(dump, &[[0u8, 1, 0x80][(rng.next() % 3) as usize]]);
        t.put_u32(sg_frame, rng.next());
        let chr = t.alloc(0x880);
        for k in 0..0x220 {
            t.put_u32(chr + 4 * k, rng.next());
        }
        let n = if rng.next().is_multiple_of(8) {
            small(&mut rng, -2, 3)
        } else {
            rng.next() % 9
        };
        let list = t.alloc(8 * 0x20);
        let mut at = list;
        for _ in 0..n.min(9) {
            let kind = [0u8, 1, 2, 3, 4, 5, 6, 0xff, 1, 2, 3, 5][(rng.next() % 12) as usize];
            let extra = [0i8, 1, 2, 0, 1, 2, -1, -2][(rng.next() % 8) as usize];
            let start = small(&mut rng, -3, 30) as u16;
            let end = small(&mut rng, -3, 34) as u16;
            let id = if rng.next().is_multiple_of(3) {
                rng.next() as u16 | 0x8000
            } else {
                rng.next() as u16
            };
            let mut e = vec![kind, extra.cast_unsigned()];
            for v in [start, end, id] {
                e.extend(v.to_le_bytes());
            }
            for _ in 0..extra.max(0) {
                e.extend(rng.next().to_le_bytes());
            }
            t.put(at, &e);
            at = at.wrapping_add((8 + 4 * i32::from(extra)).cast_unsigned());
        }
        t.put(chr + 0x7f0, &(n as u16).to_le_bytes());
        t.put_u32(chr + 0x7f4, if round % 16 == 5 { 0 } else { list });
        let last = small(&mut rng, -3, 30);
        t.put(chr + 0x7f2, &(last as u16).to_le_bytes());
        let frame = small(&mut rng, -3, 34);
        t.put_u32(chr + 0x744, (frame << 8) | (rng.next() & 0xff));
        t.put_u32(
            chr + 0x7f8,
            if rng.next().is_multiple_of(4) { 0 } else { cb },
        );
        t.put_u32(chr + 0x6fc, rng.next() % 2);
        t.put_u32(chr + 0x7fc, chr);
        let before: u32 = rd(&t.a, fk.log);
        t.check(&format!("Ch_MotEvent {round}"), 0x0083_4350, &[chr], |m| {
            mo::ch_mot_event(m, chr, &mut |m: &mut Oracle, f, a: &[u32]| call(m, f, a));
            Res::Void
        });
        let after: u32 = rd(&t.a, fk.log);
        let mut p = before;
        while p < after {
            let id: u32 = rd(&t.a, p);
            reach[id as usize] += 1;
            p += 4 * (1 + [0, 2, 5, 3, 11, 2][id as usize]);
        }
        reach[6 + usize::from(frame.cast_signed() <= i32::from(last as i16))] += 1;
    }
    eprintln!("{}: small2 b reach {reach:?}", t.name);
}

#[test]
fn small2_c_matches_the_original() {
    for mut t in twins() {
        small2_c_on(&mut t);
    }
}

/// `Ch_PreCalcActiveClothMesh` (small round 2, group c): random cloths
/// with and without (matching) baked data. The caches, `dbgPrintf` and
/// `InsideTriangle` are faked; the stand-in for `InsideTriangle` writes a
/// distance on every other call it gets one, so the stale slot shows (the
/// original's slot, `ebp-0x64`, and the port's scratch start equal).
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::many_single_char_names
)]
fn small2_c_on(t: &mut Twin) {
    use ffx_chara::cloth::bridge as cl;
    let mut rng = Xorshift(0x5a11_2c2c);
    let fk = Fakes::new(t);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let vbuf = fk.fake(t, cl::CLOTH_VERTEX_BUFF, 1, 2);
    let mdata = fk.fake(t, cl::CLOTH_MAPPING_DATA, 2, 2);
    let print = fk.fake(t, cl::DBG_PRINTF, 3, 4);
    // InsideTriangle: every other call with a distance pointer gets a
    // distance (a positive float below 2); then the two pointers, which
    // differ (stack against scratch), are logged by what they hold: the
    // position's 16 bytes go to their own log (`pos` logged as 0), and
    // `&dist` as whether one was given
    let (tri_log, tri_ret) = fk.fake_code(t, 4, 8);
    let counter = t.alloc(4);
    let pos_log = t.alloc(4);
    let pos_area = t.alloc(0x1000);
    let stub = t.a.alloc_code(&[0; 128]).unwrap();
    assert_eq!(t.b.alloc_code(&[0; 128]).unwrap(), stub);
    let mut c = vec![0x8b, 0x44, 0x24, 0x20, 0x85, 0xc0, 0x74, 0x1d, 0x8b, 0x0d];
    c.extend(counter.to_le_bytes());
    c.extend([0x81, 0x05]);
    c.extend(counter.to_le_bytes());
    c.extend(0x9e37_79b9u32.to_le_bytes());
    c.extend([
        0xf6, 0xc1, 0x01, 0x74, 0x08, 0x81, 0xe1, 0xff, 0xff, 0xff, 0x3f, 0x89, 0x08,
    ]);
    // mov eax, [esp+8]; mov edx, [pos_log]; copy 4 words; [pos_log] += 16
    c.extend([0x8b, 0x44, 0x24, 0x08, 0x8b, 0x15]);
    c.extend(pos_log.to_le_bytes());
    c.extend([
        0x8b, 0x08, 0x89, 0x0a, 0x8b, 0x48, 0x04, 0x89, 0x4a, 0x04, 0x8b, 0x48, 0x08, 0x89, 0x4a,
        0x08, 0x8b, 0x48, 0x0c, 0x89, 0x4a, 0x0c, 0x83, 0x05,
    ]);
    c.extend(pos_log.to_le_bytes());
    c.push(0x10);
    // [esp+8] = 0; [esp+0x20] = [esp+0x20] != 0
    c.extend([
        0xc7, 0x44, 0x24, 0x08, 0, 0, 0, 0, 0x83, 0x7c, 0x24, 0x20, 0x00, 0x0f, 0x95, 0xc0, 0x0f,
        0xb6, 0xc0, 0x89, 0x44, 0x24, 0x20, 0xe9,
    ]);
    c.extend(tri_log.wrapping_sub(stub + 104).to_le_bytes());
    assert_eq!(c.len(), 104);
    t.put(stub, &c);
    let tri_at = t.a.rebase(cl::INSIDE_TRIANGLE);
    let mut jmp = vec![0xe9];
    jmp.extend(stub.wrapping_sub(tri_at + 5).to_le_bytes());
    t.put(tri_at, &jmp);

    let frame = t.b.stack_range().start + 0x400;
    let a_slot = t.a.stack_range().end - 0x100 - 0x74;
    let normal = [
        0.0f32,
        0.1,
        f32::from_bits(0x3e99_9999),
        0.3,
        f32::from_bits(0x3e99_999b),
        0.31,
        0.5,
        -0.5,
        -0.3,
        -0.1,
        1.0,
        f32::NAN,
    ];
    let mut reach = [0u32; 8];
    for round in 0..240u32 {
        fk.reset(t);
        t.put_u32(print, 0);
        t.put_u32(tri_ret, rng.next());
        t.put_u32(counter, rng.next());
        t.put_u32(pos_log, pos_area);
        let stale = (rng.next() & 0x3fff_ffff) | 1;
        t.a.write(a_slot, &stale.to_le_bytes()).unwrap();
        t.b.write(frame + 0x10, &stale.to_le_bytes()).unwrap();

        let chr = t.alloc(0x880);
        for k in 0..0x220 {
            t.put_u32(chr + 4 * k, rng.next());
        }
        let id: u16 = if rng.next().is_multiple_of(3) {
            0x66
        } else {
            (rng.next() % 0x120) as u16
        };
        t.put(chr, &id.to_le_bytes());
        t.put_u32(chr + 0x844, ((rng.next() % 4000) as f32 / 8.0).to_bits());
        let dead = t.alloc(16);
        t.put_u32(chr + 0x1c0, dead);
        let cloth = t.alloc(0x50);
        let meshes = rng.next() % 4;
        t.put_u32(cloth + 0xc, u32::from(round % 16 == 3));
        t.put_u32(cloth + 0x14, meshes);
        t.put_u32(
            cloth + 0x1c,
            (rng.next() % 0x0080_0000 + 0x3c00_0000) | (rng.next() & 0x8000_0000),
        );
        t.put_u32(chr + 0x834, if round % 16 == 7 { 0 } else { cloth });
        let vb = t.alloc(0x20);
        t.put_u32(vbuf, if round % 16 == 9 { 0 } else { vb });
        let with_md = !rng.next().is_multiple_of(4);
        let md = t.alloc(0x2000);
        let small: Vec<u8> = (0..0x800)
            .flat_map(|_| (rng.next() % 6).to_le_bytes())
            .collect();
        t.put(md, &small);
        t.put_u32(mdata, if with_md { md } else { 0 });
        let mut p = md + 8;
        let hdr_ok = !rng.next().is_multiple_of(6);
        t.put_u32(p, if hdr_ok { meshes } else { meshes + 1 });
        p += 4;
        for m in 0..meshes {
            let segs = rng.next() % 4;
            let list = t.alloc(0x20);
            t.put_u32(list, segs);
            let vbm = t.alloc(0x20);
            let null = rng.next().is_multiple_of(10);
            t.put_u32(cloth + 0x30 + 4 * m, if null { 0 } else { list });
            t.put_u32(vb + 4 + 4 * m, vbm);
            let mh_ok = !rng.next().is_multiple_of(6);
            t.put_u32(p, if mh_ok { segs } else { segs + 1 });
            p += 4;
            if null {
                continue;
            }
            for s in 0..segs {
                let n = rng.next() % 6;
                let seg = t.alloc(0x50);
                let vseg = t.alloc(0x10);
                let normals = t.alloc(0x60 + 16);
                let model: u16 = if rng.next().is_multiple_of(2) {
                    2
                } else {
                    (rng.next() % 4) as u16
                };
                let code: u16 = if rng.next().is_multiple_of(2) {
                    10
                } else {
                    (rng.next() % 400) as u16
                };
                t.put(seg + 0x40, &model.to_le_bytes());
                t.put(seg + 0x42, &code.to_le_bytes());
                t.put_u32(seg + 0x44, n);
                let seg_null = rng.next().is_multiple_of(10);
                t.put_u32(seg + 0x4c, if seg_null { 0 } else { normals });
                for k in 0..n * 4 {
                    let v = normal[(rng.next() % 12) as usize];
                    t.put_u32(normals + 4 * k, v.to_bits());
                }
                let src = t.alloc(0x60);
                for k in 0..n * 3 {
                    t.put_u32(
                        src + 4 * k,
                        ((rng.next() % 512) as f32 / 64.0 - 4.0).to_bits(),
                    );
                }
                let records = random_block(t, &mut rng, 0x40);
                let tris = random_block(t, &mut rng, 0x60);
                t.put_u32(vseg + 4, src);
                t.put_u32(vseg + 8, records);
                t.put_u32(vseg + 0xc, tris);
                t.put_u32(list + 4 + 4 * s, seg);
                let vseg_null = rng.next().is_multiple_of(12);
                t.put_u32(vbm + 4 + 4 * s, if vseg_null { 0 } else { vseg });
                let sh_ok = !rng.next().is_multiple_of(6);
                let sh = if sh_ok { n } else { (n + 1) % 6 };
                t.put_u32(p, sh);
                p += 4;
                let baked = with_md && hdr_ok && mh_ok && sh_ok;
                p += sh
                    * if seg_null || vseg_null || baked {
                        20
                    } else {
                        16
                    };
                if !seg_null && !vseg_null {
                    reach[usize::from(baked)] += 1;
                }
            }
        }
        // a whole mesh past the count, which only a loop too long reaches
        {
            let list = t.alloc(8);
            let vbm = t.alloc(8);
            let seg = t.alloc(0x50);
            let vseg = t.alloc(0x10);
            let normals = t.alloc(0x20);
            for k in 0..8 {
                t.put_u32(normals + 4 * k, 1f32.to_bits());
            }
            t.put_u32(list, 1);
            t.put_u32(list + 4, seg);
            t.put_u32(vbm + 4, vseg);
            t.put_u32(seg + 0x44, 2);
            t.put_u32(seg + 0x4c, normals);
            for off in [4u32, 8, 0xc] {
                let b = random_block(t, &mut rng, 0x20);
                t.put_u32(vseg + off, b);
            }
            t.put_u32(cloth + 0x30 + 4 * meshes, list);
            t.put_u32(vb + 4 + 4 * meshes, vbm);
        }
        let skip = u32::from(rng.next().is_multiple_of(5));
        let before: u32 = rd(&t.a, fk.log);
        t.check(
            &format!("Ch_PreCalcActiveClothMesh {round}"),
            0x0082_7e90,
            &[chr, skip],
            |m| {
                cl::ch_pre_calc_active_cloth_mesh(
                    m,
                    chr,
                    skip,
                    frame,
                    &mut |m: &mut Oracle, f, a: &[u32]| call(m, f, a),
                );
                Res::Void
            },
        );
        let after: u32 = rd(&t.a, fk.log);
        let mut q = before;
        while q < after {
            let id: u32 = rd(&t.a, q);
            if id == 4 {
                let code: u32 = rd(&t.a, q + 16);
                let d: u32 = rd(&t.a, q + 32);
                reach[2 + d as usize] += 1;
                reach[4] += u32::from(code < 100);
            }
            q += 4 * (1 + [0, 2, 2, 4, 8][id as usize]);
        }
        reach[5] += skip;
    }
    eprintln!("{}: small2 c reach {reach:?}", t.name);
}

#[test]
fn small2_f_matches_the_original() {
    for mut t in twins() {
        small2_f_on(&mut t);
    }
}

/// `Ch_CalcMiddle` (small round 2, group f): random character arrays
/// (active, running, modelled, bone counts around the 200 cut), the job
/// scheduler, the pause, `Ch_MotEvent` and `Ch_RequestCalcNormal` faked.
fn small2_f_on(t: &mut Twin) {
    use ffx_chara::handle::bridge as hb;
    let mut rng = Xorshift(0x5a11_2f2f);
    let fk = Fakes::new(t);
    let call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
    let wait = fk.fake(t, hb::CALC_MIDDLE_WAIT, 1, 0);
    let sched = fk.fake(t, hb::CALC_MIDDLE_SCHEDULE, 2, 3);
    let pause = fk.fake(t, hb::GET_PAUSE, 3, 0);
    let (array, count) = (t.a.rebase(0x023c_44e4), t.a.rebase(0x023c_44e0));
    let sink = t.a.rebase(0x0230_56e8);
    // stand-ins with a side effect before the logging one: Ch_MotEvent
    // flips bit 28 of the character's +0x194 (so it is read again), the
    // normal request moves the GS register sink
    let chained = |t: &mut Twin, sym: u32, id: u32, n: u8, pre: &[u8]| {
        let (code, ret) = fk.fake_code(t, id, n);
        let at = t.a.alloc_code(&[0; 32]).unwrap();
        assert_eq!(t.b.alloc_code(&[0; 32]).unwrap(), at);
        let mut c = pre.to_vec();
        c.push(0xe9);
        let end = at + u32::try_from(c.len()).unwrap() + 4;
        c.extend(code.wrapping_sub(end).to_le_bytes());
        t.put(at, &c);
        let f = t.a.rebase(sym);
        let mut jmp = vec![0xe9];
        jmp.extend(at.wrapping_sub(f + 5).to_le_bytes());
        t.put(f, &jmp);
        ret
    };
    let mot = chained(
        t,
        hb::MOT_EVENT,
        4,
        1,
        &[
            0x8b, 0x44, 0x24, 0x04, 0x81, 0xb0, 0x94, 0x01, 0, 0, 0, 0, 0, 0x10,
        ],
    );
    let mut add = vec![0x81, 0x05];
    add.extend(sink.to_le_bytes());
    add.extend(0x0123_4567u32.to_le_bytes());
    let normal = chained(t, hb::REQUEST_CALC_NORMAL, 5, 0, &add);
    let mut reach = [0u32; 6];
    for round in 0..120u32 {
        fk.reset(t);
        for w in [wait, sched, mot, normal] {
            t.put_u32(w, rng.next());
        }
        t.put_u32(pause, u32::from(rng.next().is_multiple_of(6)));
        t.put_u32(sink, rng.next());
        let n = if round % 20 == 0 { 0 } else { rng.next() % 24 };
        let chrs = t.alloc(0x880 * n.max(1) as usize);
        for k in 0..n {
            let c = chrs + 0x880 * k;
            t.put(c + 2, &[u8::from(!rng.next().is_multiple_of(5))]);
            t.put(c + 0x180, &[u8::from(rng.next().is_multiple_of(3))]);
            t.put(c + 0x181, &[u8::from(rng.next().is_multiple_of(2))]);
            t.put_u32(
                c + 0x19c,
                if rng.next().is_multiple_of(3) {
                    0
                } else {
                    rng.next() | 1
                },
            );
            let skin = t.alloc(0x10);
            let bones = (rng.next() % 140) as u16;
            t.put(skin + 0xa, &bones.to_le_bytes());
            t.put_u32(c + 0x1c0, skin);
            t.put_u32(c + 0x194, rng.next());
        }
        // a null array with no characters once in a while
        t.put_u32(array, if round % 40 == 20 { 0 } else { chrs });
        t.put_u32(count, n);
        let before: u32 = rd(&t.a, fk.log);
        t.check(&format!("Ch_CalcMiddle {round}"), 0x0083_3390, &[], |m| {
            Res::Int(hb::ch_calc_middle(
                m,
                &mut |m: &mut Oracle, f, a: &[u32]| call(m, f, a),
            ))
        });
        let after: u32 = rd(&t.a, fk.log);
        let mut q = before;
        while q < after {
            let id: u32 = rd(&t.a, q);
            reach[id as usize] += 1;
            q += 4 * (1 + [0, 0, 3, 0, 1, 0][id as usize]);
        }
    }
    eprintln!("{}: small2 f reach {reach:?}", t.name);
}
