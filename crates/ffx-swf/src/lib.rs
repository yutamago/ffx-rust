//! The PC screens' Flash movies played by Ruffle instead of Iggy (M5 pilot,
//! `docs/formats/pc-ui.md` "Iggy calls" and "Ruffle"): the hook's Iggy call
//! log ([`log`]) replayed into Ruffle players ([`stage`]), headless.

pub mod compose;
pub mod log;
pub mod stage;

/// The `.swf` files under `ffx_data/gamedata/ps3data/flash/` of the
/// player's `FFX_Data.vbf`: (file name, bytes).
///
/// # Errors
/// The archive.
pub fn flash_movies(vbf: &std::path::Path) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
    let archive = ffx_vbf::Archive::open(vbf)?;
    let mut out = Vec::new();
    for e in archive.entries() {
        let path = e.path.to_ascii_lowercase();
        if let Some(name) = path.strip_prefix("ffx_data/gamedata/ps3data/flash/")
            && name.ends_with(".swf")
        {
            out.push((name.to_owned(), archive.read(e)?));
        }
    }
    Ok(out)
}
