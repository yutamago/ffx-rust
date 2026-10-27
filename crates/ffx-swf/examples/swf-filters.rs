//! Lists the display filters a movie's timeline puts on its objects
//! (`PlaceObject3` filter lists, sprites included), counted by kind and
//! parameters: `swf-filters <file.swf>...`.

use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use ruffle_core::swf::{self, Filter, Tag};

fn describe(f: &Filter) -> String {
    match f {
        Filter::GlowFilter(g) => format!(
            "glow colour {:02x}{:02x}{:02x}{:02x} blur {}x{} strength {} passes {} inner {} knockout {}",
            g.color.r,
            g.color.g,
            g.color.b,
            g.color.a,
            g.blur_x.to_f32(),
            g.blur_y.to_f32(),
            g.strength.to_f32(),
            g.num_passes(),
            g.is_inner(),
            g.is_knockout()
        ),
        Filter::DropShadowFilter(d) => format!(
            "dropshadow colour {:02x}{:02x}{:02x}{:02x} blur {}x{} angle {} distance {} strength {} passes {}",
            d.color.r,
            d.color.g,
            d.color.b,
            d.color.a,
            d.blur_x.to_f32(),
            d.blur_y.to_f32(),
            d.angle.to_f32(),
            d.distance.to_f32(),
            d.strength.to_f32(),
            d.num_passes()
        ),
        Filter::BlurFilter(b) => format!(
            "blur {}x{} passes {}",
            b.blur_x.to_f32(),
            b.blur_y.to_f32(),
            b.num_passes()
        ),
        other => format!("{other:?}").chars().take(80).collect(),
    }
}

fn walk(tags: &[Tag<'_>], count: &mut BTreeMap<String, u32>) {
    for t in tags {
        match t {
            Tag::PlaceObject(p) => {
                if let Some(filters) = &p.filters {
                    for f in filters {
                        *count.entry(describe(f)).or_default() += 1;
                    }
                }
            }
            Tag::DefineSprite(s) => walk(&s.tags, count),
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path)?;
        let buf = swf::decompress_swf(&data[..]).map_err(|e| anyhow!("{e}"))?;
        let parsed = swf::parse_swf(&buf).map_err(|e| anyhow!("{e}"))?;
        let mut count = BTreeMap::new();
        walk(&parsed.tags, &mut count);
        println!("{path}:");
        for (k, n) in count {
            println!("  {n:4} {k}");
        }
    }
    Ok(())
}
