//! The fixed-function state and the constants of each draw, as
//! `ffx-drawdiff` decodes them (`docs/drawdiff.md`): what the effects
//! census tabulates per VSP flag combination.

use ffx_drawdiff::Value;
use ffx_drawdiff::capture::{Capture, FrameItems};

/// One draw's state summary and the constant variables its shaders use.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Summary {
    /// `blend src/dest/op alpha src/dest/op`, `depth enable/write/func`,
    /// `cull`, `topology` (Direct3D 11 enum values).
    pub state: String,
    /// `<stage>.cb<slot>.<variable>` names, sorted, without element
    /// suffixes.
    pub constants: Vec<String>,
}

fn field<'a>(item: &'a ffx_drawdiff::Item, name: &str) -> Option<&'a Value> {
    item.fields
        .iter()
        .find(|f| f.name == name)
        .map(|f| &f.value)
}

fn v(item: &ffx_drawdiff::Item, name: &str) -> String {
    match field(item, name) {
        Some(Value::U(x)) => x.to_string(),
        Some(Value::I(x)) => x.to_string(),
        Some(Value::F(x)) => format!("{:?}", f32::from_bits(*x)),
        Some(other) => other.to_string(),
        None => "-".into(),
    }
}

/// The summaries of a frame's draws, by draw index (clears and copies are
/// left out, as in the census).
#[must_use]
pub fn summaries(frame: &FrameItems) -> Vec<Summary> {
    frame
        .items
        .iter()
        .filter(|i| i.kind == ffx_drawdiff::ItemKind::Draw)
        .map(|i| {
            let blend = if v(i, "blend.rt0.enable") == "1" {
                format!(
                    "blend {}/{}/{} alpha {}/{}/{}",
                    v(i, "blend.rt0.src"),
                    v(i, "blend.rt0.dest"),
                    v(i, "blend.rt0.op"),
                    v(i, "blend.rt0.src_alpha"),
                    v(i, "blend.rt0.dest_alpha"),
                    v(i, "blend.rt0.op_alpha"),
                )
            } else {
                "blend off".into()
            };
            let depth = if v(i, "depth.enable") == "1" {
                format!(
                    "depth on write {} func {}",
                    v(i, "depth.write_mask"),
                    v(i, "depth.func")
                )
            } else {
                "depth off".into()
            };
            let state = format!(
                "{blend}, {depth}, cull {}, mask {}",
                v(i, "raster.cull"),
                v(i, "blend.rt0.write_mask")
            );
            let mut constants: Vec<String> = i
                .constants
                .iter()
                .flat_map(|c| {
                    c.vars
                        .iter()
                        .map(move |var| format!("{}.cb{}.{}", c.stage, c.slot, var.name))
                })
                .collect();
            constants.sort();
            constants.dedup();
            Summary { state, constants }
        })
        .collect()
}

/// Reads a capture through `ffx-drawdiff` and summarises frame `n`.
///
/// # Errors
/// When the capture cannot be read.
pub fn read(
    path: &std::path::Path,
    n: usize,
) -> Result<Vec<Summary>, ffx_drawdiff::capture::Error> {
    let c = Capture::read(path)?;
    Ok(c.frames.get(n).map(summaries).unwrap_or_default())
}
