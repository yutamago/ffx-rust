//! How a captured effect draw's uvs sample its texture
//! (`ffx_host::vfx::upright_uv`; docs/formats/effects-draw.md, "Texture
//! coordinates"), on the texture of Besaid's save sphere sparkle quads.
//! Needs `FFX_GAME_DIR`; skips without it.

use ffx_host::vfx::upright_uv;

/// The sparkle quads' texture (flags `0xa`, keys `(6, 0xd29c68)` and
/// `(8, 0xd29da8)` at Besaid, developer save 7), as the archive names it.
const SPARKLE: &str = "ffx_data/gamedata/ps3data/yonishi_data/dat_et/bat_eff/et_tex/tex/d3d11/15456_19_0_0_256_1024.dds.phyre";

/// Their uv block, the same on every frame (`vfx-probe`, updates 60-210).
const U: (f32, f32) = (0.0039, 0.2461);
const V: (f32, f32) = (0.4385, 0.4990);

/// The alpha of the upright image's texel at `uv` (`(0, 0)` its top left).
fn alpha(rgba: &[u8], w: u32, h: u32, uv: [f32; 2]) -> u8 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let at = |t: f32, n: u32| ((t * n as f32) as u32).min(n - 1) as usize;
    rgba[(at(uv[1], h) * w as usize + at(uv[0], w)) * 4 + 3]
}

/// Read with `upright_uv` (v up from the bottom), the block is one soft
/// ball, bright in the middle and clear at the corners, as the game shows
/// it; read top-down it is the corner of four atlas cells (frames of
/// other effects), clear in the middle.
#[test]
fn the_sparkle_block_is_one_glow_with_v_from_the_bottom() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive =
        ffx_vbf::Archive::open(std::path::Path::new(&dir).join("data/FFX_Data.vbf")).unwrap();
    let entry = archive.find(SPARKLE).expect("the sparkle texture");
    let t = ffx_phyre::Texture::parse(archive.read(entry).unwrap()).unwrap();
    let (w, h) = (t.width, t.height);
    let rgba = t.to_rgba();
    let centre = [U.0.midpoint(U.1), V.0.midpoint(V.1)];
    let corners = [[U.0, V.0], [U.1, V.0], [U.0, V.1], [U.1, V.1]];

    let up = |uv| alpha(&rgba, w, h, upright_uv(uv));
    assert!(up(centre) > 200, "centre alpha {}", up(centre));
    for c in corners {
        assert!(up(c) < 8, "corner {c:?} alpha {}", up(c));
    }

    let as_is = |uv| alpha(&rgba, w, h, uv);
    assert!(as_is(centre) < 16, "as is: centre alpha {}", as_is(centre));
    // the four cells' middles: at least one holds a bright frame
    let cells = [U.0 + (U.1 - U.0) / 4.0, U.1 - (U.1 - U.0) / 4.0]
        .into_iter()
        .flat_map(|u| [[u, V.0 + (V.1 - V.0) / 4.0], [u, V.1 - (V.1 - V.0) / 4.0]]);
    assert!(cells.map(as_is).max().unwrap() > 200);
}
