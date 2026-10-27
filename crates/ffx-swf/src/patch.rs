//! Load-time adjustments that make Ruffle draw like Iggy without changing
//! Ruffle: the movie is rewritten before Ruffle reads it.
//!
//! Iggy smooths every bitmap fill (bilinear), including the fills the SWF
//! marks as not smoothed (fill types 0x42 and 0x43), which Ruffle and Flash
//! draw with nearest-neighbour sampling: the save list's portraits and
//! thumbnails came out blocky (`docs/formats/pc-ui.md`, "Ruffle").
//! [`smooth_bitmap_fills`] marks them smoothed.

use anyhow::{Result, anyhow};
use ruffle_core::swf::{self, FillStyle, LineStyle, ShapeRecord, Tag};

fn smooth(fills: &mut [FillStyle]) -> usize {
    let mut n = 0;
    for f in fills {
        if let FillStyle::Bitmap { is_smoothed, .. } = f
            && !*is_smoothed
        {
            *is_smoothed = true;
            n += 1;
        }
    }
    n
}

fn smooth_lines(lines: &mut [LineStyle]) -> usize {
    let mut n = 0;
    for l in lines {
        if let FillStyle::Bitmap {
            id,
            matrix,
            is_smoothed: false,
            is_repeating,
        } = l.fill_style()
        {
            let f = FillStyle::Bitmap {
                id: *id,
                matrix: *matrix,
                is_smoothed: true,
                is_repeating: *is_repeating,
            };
            *l = l.clone().with_fill_style(f);
            n += 1;
        }
    }
    n
}

fn smooth_records(records: &mut [ShapeRecord]) -> usize {
    let mut n = 0;
    for r in records {
        if let ShapeRecord::StyleChange(c) = r
            && let Some(styles) = c.new_styles.as_mut()
        {
            n += smooth(&mut styles.fill_styles) + smooth_lines(&mut styles.line_styles);
        }
    }
    n
}

/// The movie with every bitmap fill smoothed, uncompressed (`FWS`), and how
/// many fills changed.
///
/// # Errors
/// The SWF does not parse or write.
pub fn smooth_bitmap_fills(data: &[u8]) -> Result<(Vec<u8>, usize)> {
    let buf = swf::decompress_swf(data).map_err(|e| anyhow!("{e}"))?;
    let mut parsed = swf::parse_swf(&buf).map_err(|e| anyhow!("{e}"))?;
    let mut n = 0;
    for tag in &mut parsed.tags {
        match tag {
            Tag::DefineShape(s) => {
                n += smooth(&mut s.styles.fill_styles)
                    + smooth_lines(&mut s.styles.line_styles)
                    + smooth_records(&mut s.shape);
            }
            Tag::DefineMorphShape(m) => {
                n += smooth(&mut m.start.fill_styles)
                    + smooth(&mut m.end.fill_styles)
                    + smooth_records(&mut m.start.shape);
            }
            _ => {}
        }
    }
    let mut header = parsed.header.swf_header().clone();
    header.compression = swf::Compression::None;
    let mut out = Vec::with_capacity(buf.data.len() + 64);
    swf::write_swf(&header, &parsed.tags, &mut out).map_err(|e| anyhow!("{e}"))?;
    Ok((out, n))
}
