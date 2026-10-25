//! The UI and 2D draws: what the menus, the fonts and the Iggy screens
//! draw, and from what (M4 census, `docs/formats/ui-draw.md`).
//!
//! A capture (`capture.ffxgfx`, `ffx-hook`'s `gfx/capture.rs`) is walked
//! draw by draw ([`frame`]); every draw outside the scene targets is
//! given its family by its shaders' origin ([`index`]) and, for the UI
//! families, its producer, texture atlas, vertex source, states and
//! constants are checked against what the game's code builds
//! ([`census`]).

pub mod census;
pub mod frame;
pub mod index;

/// Archive folders whose textures the UI draws sample: the menu textures
/// (`ps3data/menu/d3d11/`), the English menu and font textures
/// (`menu_us/`, `menu_us/base_ftc/`) and the sphere grid.
pub const UI_TEXTURE_FOLDERS: [&str; 3] = ["/menu/", "/menu_us/", "/abmap/"];

/// The index the census uses: every shader cluster, the textures under
/// [`UI_TEXTURE_FOLDERS`], and the DXBC blobs in `FFX.exe` (`GDraw`'s).
///
/// # Errors
/// When the archive or `FFX.exe` cannot be read.
pub fn ui_index(game: &std::path::Path) -> Result<index::Index, String> {
    let archive = ffx_vbf::Archive::open(game.join("data/FFX_Data.vbf"))
        .map_err(|e| format!("archive: {e}"))?;
    let mut index = index::Index::default();
    index
        .add_archive(&archive, |p| {
            UI_TEXTURE_FOLDERS.iter().any(|f| p.contains(f))
        })
        .map_err(|e| format!("archive: {e}"))?;
    let exe = std::fs::read(game.join("FFX.exe")).map_err(|e| format!("FFX.exe: {e}"))?;
    index.add_image("FFX.exe", &exe);
    Ok(index)
}
