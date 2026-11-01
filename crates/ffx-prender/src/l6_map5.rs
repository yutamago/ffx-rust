//! M5 stage 1, round 6 (list `left6`, from `map2.left`), batch 3: the
//! scene text loader's camera record (`rcbgSceneLoadTextSub`'s camera
//! line: a position and target, or a position and three angles).
//!
//! Callees stay calls to the originals (cdecl); `rcdtParseGetFloat` returns
//! in `ST0` (stored as the original stores it), the angles go through the
//! CRT's `_CIfmod`.

/// `rcdtParseIsEnd(p)`, `rcdtParseGetKeyword(p, table)`,
/// `rcdtParseNextToken(p)`, `rcdtParseGetFloat(p)` (`ST0`),
/// `rcbgSceneSetMatrix(scene)`, `rcbgSceneLoadTextCameraCalcRef(scene)`.
pub const PARSE_IS_END: u32 = 0x0092_bfb0;
pub const PARSE_GET_KEYWORD: u32 = 0x0092_be40;
pub const PARSE_NEXT_TOKEN: u32 = 0x0092_ba30;
pub const PARSE_GET_FLOAT: u32 = 0x0092_bbb0;
pub const SCENE_SET_MATRIX: u32 = 0x0092_0c00;
pub const CAMERA_CALC_REF: u32 = 0x0093_b110;
/// The keyword table (the target form), `_CIfmod`'s thunk, f64 360,
/// f32 -180, 180 and -1.
pub const TARGET_KEYWORDS: u32 = 0x00c6_0ea4;
pub const CI_FMOD: u32 = 0x0094_98e0;
pub const DEG_360: u32 = 0x00b4_8380;
pub const DEG_MINUS_180: u32 = 0x00b5_5f80;
pub const DEG_180: u32 = 0x00b5_a480;
pub const MINUS_ONE: u32 = 0x00b9_2340;

pub mod bridge {
    use super::{
        CAMERA_CALC_REF, CI_FMOD, DEG_180, DEG_360, DEG_MINUS_180, MINUS_ONE, PARSE_GET_FLOAT,
        PARSE_GET_KEYWORD, PARSE_IS_END, PARSE_NEXT_TOKEN, SCENE_SET_MATRIX, TARGET_KEYWORDS,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// The next float (`fstp dword`), negated first with `neg` (`fchs`).
    fn float(h: &mut impl Host, parse: u32, neg: bool) -> u32 {
        let v = h
            .call_sym_st0(PARSE_GET_FLOAT, Conv::Cdecl, &[parse])
            .to_bits();
        if neg { v ^ 0x8000_0000 } else { v }
    }

    /// `scene + off` = `v`.
    fn w(h: &mut impl Host, scene: u32, off: u32, v: u32) {
        h.write(scene.wrapping_add(off), v);
    }

    /// An `f32` constant as `fld dword` loads it.
    fn k32(h: &impl Host, sym: u32) -> f64 {
        x87::load(h.read(h.rebase(sym)))
    }

    /// `FUN_0093af60(scene, parse)` -> `eax` as the original: at the end
    /// of the text `rcdtParseIsEnd`'s; else the camera line. With the
    /// target keyword: the position (`+0x68`, y and z negated) and the
    /// target (`+0x78`, likewise), the up vector (0, -1, 0 at `+0x88`),
    /// three floats skipped and the field of view (`+0xa8`); otherwise the
    /// position, three angles (`+0x98`, y and z negated) each wrapped into
    /// -180..180 (`fmod` 360, then +360 below -180, -360 above 180, each
    /// step rounded to f32) and the field of view, then the reference
    /// point (`rcbgSceneLoadTextCameraCalcRef`). Then the scene's matrix
    /// (`rcbgSceneSetMatrix`), whose `eax` it returns.
    pub fn camera_line(h: &mut impl Host, scene: u32, parse: u32) -> u32 {
        let end = cdecl(h, PARSE_IS_END, &[parse]);
        if end != 0 {
            return end;
        }
        let table = h.rebase(TARGET_KEYWORDS);
        if cdecl(h, PARSE_GET_KEYWORD, &[parse, table]) != 0 {
            cdecl(h, PARSE_NEXT_TOKEN, &[parse]);
            cdecl(h, PARSE_NEXT_TOKEN, &[parse]);
            for (off, neg) in [
                (0x68, false),
                (0x6c, true),
                (0x70, true),
                (0x78, false),
                (0x7c, true),
                (0x80, true),
            ] {
                let v = float(h, parse, neg);
                w(h, scene, off, v);
            }
            w(h, scene, 0x88, 0);
            let m1 = x87::store(k32(h, MINUS_ONE));
            w(h, scene, 0x8c, m1);
            w(h, scene, 0x90, 0);
            for _ in 0..3 {
                float(h, parse, false);
            }
            let fov = float(h, parse, false);
            w(h, scene, 0xa8, fov);
            return cdecl(h, SCENE_SET_MATRIX, &[scene]);
        }
        cdecl(h, PARSE_NEXT_TOKEN, &[parse]);
        for (off, neg) in [
            (0x68, false),
            (0x6c, true),
            (0x70, true),
            (0x98, false),
            (0x9c, true),
            (0xa0, true),
            (0xa8, false),
        ] {
            let v = float(h, parse, neg);
            w(h, scene, off, v);
        }
        let full = f64::from_bits(h.read(h.rebase(DEG_360)));
        let fmod = h.rebase(CI_FMOD);
        for off in [0x98_u32, 0x9c, 0xa0] {
            let a = x87::load(h.read(scene.wrapping_add(off)));
            let mut v = x87::load(h.call_intrinsic2(fmod, a, full).to_bits());
            if k32(h, DEG_MINUS_180) > v {
                v = x87::load(x87::store(x87::add(v, full)));
            }
            if k32(h, DEG_180) < v {
                v = x87::load(x87::store(x87::sub(v, full)));
            }
            w(h, scene, off, x87::store(v));
        }
        cdecl(h, CAMERA_CALC_REF, &[scene]);
        cdecl(h, SCENE_SET_MATRIX, &[scene])
    }
}
