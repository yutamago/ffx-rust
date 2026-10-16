//! Character asset lists, `ffx_data/gamedata/ps3data/chr/…/<name>/<name>.ahwin32`.
//!
//! `Ch_LoadHdRenderData` (`0x0082a1c0`) reads the character's list, a generated
//! C header (CRLF lines), and loads the files of its D3D11 section: the
//! textures, the model and the shaders the model uses. A model's imports
//! resolve against what the list loaded, so a texture a model newly
//! imports must be listed. The section:
//!
//! ```text
//! #ifdef PHYRE_RENDERING_PLATFORM_D3D11 // 32 Bit
//! const Phyre::PUInt32 c001 = 24;            index of a named file
//! const Phyre::PUInt32 g_fileNamesCount = 38;
//! const Phyre::PChar *g_fileNamesPrefix = "/FFX_Data/";
//! const Phyre::PChar *g_fileNames[] = {
//! "/FFX_Data/GameData/PS3Data/chr/pc/c001/tex/D3D11/c001_anim_n0_p0.dds.phyre"
//! ,"…"
//! };
//! ```

use std::fmt::Write as _;

use crate::{Error, Result};

const SECTION: &[u8] = b"#ifdef PHYRE_RENDERING_PLATFORM_D3D11";

/// The list entry of a texture import id (`PS3Data/chr/pc/c002/tex/c002.dds`
/// → `/FFX_Data/GameData/PS3Data/chr/pc/c002/tex/D3D11/c002.dds.phyre`).
#[must_use]
pub fn texture_entry(import: &str) -> String {
    let (dir, file) = import.rsplit_once('/').unwrap_or(("", import));
    format!("/FFX_Data/GameData/{dir}/D3D11/{file}.phyre")
}

/// Puts `files` (list entries) first in the D3D11 section's file list,
/// raising the count and every named index by their number. Files already
/// listed are left out.
///
/// # Errors
/// Fails on lists without a D3D11 section or a file list.
pub fn add_files(list: &[u8], files: &[String]) -> Result<Vec<u8>> {
    let bad = || Error::Unsupported("asset list without a D3D11 file list".into());
    let start = list
        .windows(SECTION.len())
        .position(|w| w == SECTION)
        .ok_or_else(bad)?;
    let end = list[start..]
        .windows(6)
        .position(|w| w == b"#endif")
        .map_or(list.len(), |e| start + e);
    let section = String::from_utf8_lossy(&list[start..end]).into_owned();
    let new: Vec<&String> = files
        .iter()
        .filter(|f| !section.contains(&format!("\"{f}\"")))
        .collect();
    if new.is_empty() {
        return Ok(list.to_vec());
    }
    let n = new.len();
    let mut out = String::with_capacity(section.len() + 80 * n);
    let mut lines = section.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        let text = line.trim_end();
        if let Some(rest) = text.strip_prefix("const Phyre::PUInt32 ")
            && let Some((name, value)) = rest.split_once(" = ")
            && let Ok(v) = value.trim_end_matches(';').parse::<usize>()
        {
            let eol = &line[text.len()..];
            let _ = write!(out, "const Phyre::PUInt32 {name} = {};{eol}", v + n);
            continue;
        }
        out.push_str(line);
        if text.ends_with("g_fileNames[] = {") {
            let eol = &line[text.len()..];
            let first = lines.next().ok_or_else(bad)?;
            for f in &new {
                let _ = write!(out, "\"{f}\"{eol},");
            }
            out.push_str(first);
        }
    }
    let mut bytes = list[..start].to_vec();
    bytes.extend(out.into_bytes());
    bytes.extend_from_slice(&list[end..]);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_files_first_and_shifts_indices() {
        let list =
            b"#ifdef PHYRE_RENDERING_PLATFORM_GL\r\nconst Phyre::PUInt32 c001 = 1;\r\n#endif\r\n\
#ifdef PHYRE_RENDERING_PLATFORM_D3D11 // 32 Bit\r\nconst Phyre::PUInt32 c001 = 1;\r\n\
const Phyre::PUInt32 g_fileNamesCount = 2;\r\nconst Phyre::PChar *g_fileNames[] = {\r\n\
\"/a.dds.phyre\"\r\n,\"/c001.dae.phyre\"\r\n};\r\n#endif\r\n";
        let entry = texture_entry("PS3Data/chr/pc/c002/tex/c002.dds");
        assert_eq!(
            entry,
            "/FFX_Data/GameData/PS3Data/chr/pc/c002/tex/D3D11/c002.dds.phyre"
        );
        let out =
            String::from_utf8(add_files(list, std::slice::from_ref(&entry)).unwrap()).unwrap();
        let d3d = &out[out.find("D3D11").unwrap()..];
        assert!(
            d3d.contains("c001 = 2;\r\n") && d3d.contains("g_fileNamesCount = 3;"),
            "{out}"
        );
        assert!(
            d3d.contains(&format!("{{\r\n\"{entry}\"\r\n,\"/a.dds.phyre\"")),
            "{out}"
        );
        assert!(
            out.starts_with("#ifdef PHYRE_RENDERING_PLATFORM_GL\r\nconst Phyre::PUInt32 c001 = 1;")
        );
        // Listed already: unchanged.
        assert_eq!(add_files(out.as_bytes(), &[entry]).unwrap(), out.as_bytes());
    }
}
