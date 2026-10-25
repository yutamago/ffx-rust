//! A capture's character-world draws against its scene inputs
//! (`docs/formats/scene-chars.md`, "Check"): every draw whose vertices
//! are a character model's (the census) or the blob shadow's is tied to
//! the record's (instance, segment) and its per-node and material
//! constants are rebuilt ([`crate::chars`]); the cloth and shadow
//! vertices are compared with the record's.

use std::collections::HashMap;

use crate::assets::{AssetIndex, Source as AssetSource};
use crate::capture::Capture;
use crate::census::{self, DrawCensus};
use crate::chars::{self, DrawCheck, Tie};
use crate::effect::{EffectVariant, Source};
use crate::inputs;

/// The vertex comparisons of a draw: what was compared and whether it is
/// equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexCheck {
    pub what: String,
    pub bytes: usize,
    pub equal: bool,
}

/// One character-world draw.
#[derive(Debug, Clone)]
pub struct Checked {
    pub opaque: bool,
    pub draw: DrawCheck,
    pub pass: String,
    pub family: String,
    pub vertices: Vec<VertexCheck>,
}

/// The culling and sort of one character-world pass, from the record,
/// against the captured draws.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrderCheck {
    pub pass: String,
    /// Instances in the order the record's bounds give after culling and
    /// sorting (`renderWorldCulledSorted` `0x009f3c30`, as for the map:
    /// `crate::cull`, `crate::sort`).
    pub predicted: Vec<usize>,
    /// Instances of the pass's tied draws, in draw order (each once).
    pub drawn: Vec<usize>,
    /// Instances the box test culls.
    pub culled: Vec<usize>,
    /// Whether `drawn` is `predicted` with the instances that have no
    /// segment in this pass left out.
    pub in_order: bool,
}

/// A capture's result.
#[derive(Debug, Clone, Default)]
pub struct FrameCheck {
    pub draws: Vec<Checked>,
    pub order: Vec<OrderCheck>,
    /// Scene draws of a character model or the blob shadow that could not
    /// be checked, with why.
    pub unchecked: Vec<(usize, String)>,
}

/// Archive clusters the check reads: the shaders, the characters and
/// the maps of `scenes` (so that a map draw is never taken for a
/// character's through shared vertex data).
#[must_use]
pub fn wanted(path: &str, scenes: &[String]) -> bool {
    crate::assets::is_shader_cluster(path)
        || path.contains("/ps3data/chr/")
        || scenes.iter().any(|s| path.contains(&map_folder(s)))
}

/// The scene a capture shows (its `World` message).
#[must_use]
pub fn scene_of(bytes: &[u8]) -> Option<String> {
    match Capture::read(bytes).ok()?.world? {
        ffx_gfx::Msg::World { scene, .. } => Some(scene),
        _ => None,
    }
}

/// A scene's map folder: `cdsp0000` -> `/map/cdsp/cdsp00/`.
#[must_use]
pub fn map_folder(scene: &str) -> String {
    let s = scene.to_lowercase();
    let area = s.get(..4).unwrap_or(&s);
    let map = s.get(..6).unwrap_or(&s);
    format!("/map/{area}/{map}/")
}

/// Whether a census draw belongs to the character world: its vertices
/// are a character model's (`chr/`), or it is the blob shadow
/// (`abmapshadercone`, CPU-built vertices).
#[must_use]
pub fn is_char_draw(c: &DrawCensus) -> bool {
    c.segment.as_ref().is_some_and(|(m, _)| m.contains("/chr/"))
        || c.vs_family.starts_with("abmapshadercone")
}

fn cb_of(list: &[(u8, Vec<u8>)], slot: u32) -> &[u8] {
    list.iter()
        .find(|x| u32::from(x.0) == slot)
        .map_or(&[][..], |x| x.1.as_slice())
}

/// Checks the character draws of a capture.
///
/// # Errors
/// When the capture or its inputs cannot be read, or it has no
/// character-world record.
#[allow(clippy::too_many_lines)] // one walk over the frame's character draws
pub fn check_capture(
    bytes: &[u8],
    index: &AssetIndex,
    archive: &ffx_vbf::Archive,
) -> Result<FrameCheck, String> {
    let cap = Capture::read(bytes).map_err(|e| format!("capture: {e:?}"))?;
    let all = inputs::read(bytes).map_err(|e| format!("inputs: {e:?}"))?;
    let records = chars::records(&all, cap.frame.number);
    let opaque = records
        .iter()
        .find(|s| s.dir > 0 && !s.pass.starts_with("post"))
        .ok_or("no character-world record")?;
    let transparent = records
        .iter()
        .find(|s| s.dir < 0 && !s.pass.starts_with("post"))
        .unwrap_or(opaque);
    let entries: HashMap<&str, &ffx_vbf::Entry> = archive
        .entries()
        .iter()
        .map(|e| (e.path.as_str(), e))
        .collect();
    let mut variants: HashMap<String, Option<EffectVariant>> = HashMap::new();
    let census = census::census(&cap, index);
    let scene_draws: Vec<(usize, &ffx_gfx::Draw)> = cap
        .draws()
        .filter(|(_, d)| Capture::scene_targets(d).is_some())
        .collect();
    let mut out = FrameCheck::default();
    for c in census.iter().filter(|c| is_char_draw(c)) {
        let Some(&(cmd, d)) = scene_draws.get(c.number) else {
            continue;
        };
        let (Some(vs), Some(ps)) = (cap.shader(d.vs), cap.shader(d.ps)) else {
            out.unchecked
                .push((c.number, "shader not in the capture".into()));
            continue;
        };
        // The vertex program's bytecode is in several variant clusters;
        // the one whose pass binds both programs is the material's.
        let clusters: Vec<String> = index
            .get(crate::key::shader(vs))
            .iter()
            .filter_map(|s| match s {
                AssetSource::Shader { cluster } => Some(cluster.clone()),
                _ => None,
            })
            .collect();
        if clusters.is_empty() {
            out.unchecked
                .push((c.number, "vertex shader not in the archive".into()));
            continue;
        }
        for cluster in &clusters {
            variants.entry(cluster.clone()).or_insert_with(|| {
                let e = entries.get(cluster.as_str())?;
                EffectVariant::parse(archive.read(e).ok()?).ok()
            });
        }
        // Several variants can bind both programs with other material
        // layouts: the material's own is the one whose layout ties the
        // draw (the best check of all of them).
        let found: Vec<(&EffectVariant, &str, &crate::effect::ShaderPass)> = clusters
            .iter()
            .filter_map(|cl| {
                let v = variants.get(cl)?.as_ref()?;
                let (t, p) = chars::pass_of(v, vs, ps)?;
                Some((v, t, p))
            })
            .collect();
        if found.is_empty() {
            out.unchecked.push((
                c.number,
                format!(
                    "no pass of {} variants binds these programs",
                    clusters.len()
                ),
            ));
            continue;
        }
        let record = if c.pass == census::Pass::Opaque {
            opaque
        } else {
            transparent
        };
        let score = |d: &DrawCheck| {
            (
                d.tie != Tie::None,
                d.checked.iter().filter(|x| x.exact).count(),
            )
        };
        // The segment's place in its file instance (the census ties the
        // draw to a file segment by its vertex data).
        let hint = c.segment.as_ref().and_then(|(model, seg)| {
            index
                .instances
                .get(model)?
                .iter()
                .find_map(|i| i.segments.iter().position(|s| s == seg))
        });
        let mut best: Option<(DrawCheck, &str)> = None;
        for (variant, pass_type, pass) in found {
            let check = chars::check_draw(
                record,
                c.number,
                variant,
                pass,
                cb_of(&d.vs_cb, pass.vs.global_cb),
                cb_of(&d.ps_cb, pass.ps.global_cb),
                hint,
            );
            if best.as_ref().is_none_or(|b| score(&check) > score(&b.0)) {
                best = Some((check, pass_type));
            }
        }
        let Some((check, pass_type)) = best else {
            continue;
        };
        let vertices = vertex_checks(&cap, cmd, d, record, &check.tie);
        out.draws.push(Checked {
            opaque: c.pass == census::Pass::Opaque,
            draw: check,
            pass: pass_type.to_owned(),
            family: c.vs_family.clone(),
            vertices,
        });
    }
    for (record, opaque) in [(*opaque, true), (*transparent, false)] {
        let drawn: Vec<usize> = out
            .draws
            .iter()
            .filter(|c| c.opaque == opaque)
            .filter_map(|c| match c.draw.tie {
                Tie::Segment { instance, .. } => Some(instance),
                Tie::None => None,
            })
            .fold(Vec::new(), |mut v, i| {
                if v.last() != Some(&i) {
                    v.push(i);
                }
                v
            });
        if let Some(o) = order_check(record, opaque, drawn) {
            out.order.push(o);
        }
    }
    Ok(out)
}

/// The record's bounds culled and sorted for one pass, against the
/// instances drawn.
#[must_use]
pub fn order_check(
    s: &ffx_gfx::scene_inputs::SceneInputs,
    opaque: bool,
    drawn: Vec<usize>,
) -> Option<OrderCheck> {
    use crate::{cull, sort};
    let cull_vp = inputs::transpose(&inputs::cull_view_projection(s)?);
    let view = inputs::view(s)?;
    let mut entries = Vec::new();
    let mut culled = Vec::new();
    for (k, b) in s.bounds.iter().enumerate() {
        let Some(inst) = b.instance else { continue };
        let world = b.world.as_ref().map(|m| inputs::world(Some(m)));
        let m = cull::cull_matrix(&cull_vp, world.as_ref());
        if !cull::box_visible(b.min, b.size, &m) {
            culled.push(inst as usize);
            continue;
        }
        let centre = cull::centre(b.min, b.size, world.as_ref());
        let bias = inputs::depth_bias(s, u32::try_from(k).unwrap_or(u32::MAX));
        entries.push(sort::Entry {
            instance: inst as usize,
            key: cull::depth_key(&view, opaque, centre, bias),
            render_order: b.render_order,
        });
    }
    sort::sort(&mut entries);
    let predicted: Vec<usize> = entries.iter().map(|e| e.instance).collect();
    // `drawn` must be a subsequence of `predicted`.
    let mut it = predicted.iter();
    let in_order = drawn.iter().all(|d| it.any(|p| p == d));
    Some(OrderCheck {
        pass: s.pass.clone(),
        predicted,
        drawn,
        culled,
        in_order,
    })
}

/// The cloth positions of a tied segment against the draw's 16-byte
/// stream, and for the blob shadow (a draw whose tie's instance has no
/// skin and whose record has dynamic geometry) the dynamic geometry
/// arrays and indices against its streams.
fn vertex_checks(
    cap: &Capture,
    cmd: usize,
    d: &ffx_gfx::Draw,
    s: &ffx_gfx::scene_inputs::SceneInputs,
    tie: &Tie,
) -> Vec<VertexCheck> {
    let mut out = Vec::new();
    let Some(cw) = s.chars.as_ref() else {
        return out;
    };
    let Tie::Segment { instance, segment } = *tie else {
        return out;
    };
    let stream = |slot: u8| {
        let &(_, id, stride, offset) = d.vertex.iter().find(|v| v.0 == slot)?;
        let data = chars::buffer_at(cap, cmd, id)?;
        let base = offset as usize + d.base.max(0).unsigned_abs() as usize * stride as usize;
        Some((stride, data.get(base..)?.to_vec()))
    };
    for c in cw
        .cloth
        .iter()
        .filter(|c| c.instance as usize == instance && c.segment as usize == segment)
    {
        // The segment's own 16-byte streams (blend weights) are static;
        // the cloth's is the one in the dynamic ring (slot 5 on c001).
        let slot = d.vertex.iter().find_map(|v| {
            let (stride, data) = stream(v.0)?;
            (stride == 16 && data.get(..c.vertices.len()) == Some(&c.vertices[..])).then_some(v.0)
        });
        out.push(VertexCheck {
            what: format!(
                "cloth vertices ({} of 16 bytes){}",
                c.count,
                slot.map_or_else(String::new, |s| format!(", stream slot {s}"))
            ),
            bytes: c.vertices.len(),
            equal: slot.is_some(),
        });
    }
    let shadow = s
        .instances
        .get(instance)
        .is_some_and(|i| i.cluster != s.instances[0].cluster);
    if let (true, Some(g)) = (shadow, &cw.dyn_geo) {
        let sizes = [12, 12, 16, 8];
        for (k, a) in g.arrays.iter().enumerate() {
            let equal = d.vertex.iter().any(|v| {
                stream(v.0).is_some_and(|(stride, data)| {
                    stride == sizes[k] && data.get(..a.len()) == Some(&a[..])
                })
            });
            out.push(VertexCheck {
                what: format!("dynamic geometry array {k} ({} bytes a vertex)", sizes[k]),
                bytes: a.len(),
                equal,
            });
        }
        // The lists hold u16 indices; the draw binds them as
        // `R32_UINT` (0x2a) in the index ring, widened.
        let (id, format, offset) = d.index;
        let width = if format == 0x2a { 4 } else { 2 };
        let want: Vec<u8> = g
            .index_bytes
            .chunks(2)
            .flat_map(|w| {
                let v = u32::from(u16::from_le_bytes([w[0], w.get(1).copied().unwrap_or(0)]));
                v.to_le_bytes()[..width].to_vec()
            })
            .collect();
        let ib = chars::buffer_at(cap, cmd, id).and_then(|b| {
            let at = offset as usize + width * d.start as usize;
            b.get(at..at + want.len()).map(<[u8]>::to_vec)
        });
        out.push(VertexCheck {
            what: format!(
                "dynamic geometry indices ({}, {}-byte in the draw)",
                g.indices, width
            ),
            bytes: want.len(),
            equal: ib.as_deref() == Some(&want[..]),
        });
    }
    out
}

/// A text report: one line per draw, the constants that differ, and
/// totals per source and name.
#[must_use]
pub fn report(f: &FrameCheck) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let mut totals: std::collections::BTreeMap<(String, String), (usize, usize)> =
        std::collections::BTreeMap::new();
    for c in &f.draws {
        let d = &c.draw;
        let exact = d.checked.iter().filter(|x| x.exact).count();
        let _ = writeln!(
            out,
            "draw {} {} {} tie {:?} candidates {}: {exact}/{} constants exact{}",
            d.number,
            c.pass,
            c.family,
            d.tie,
            d.candidates,
            d.checked.len(),
            if d.notes.is_empty() {
                String::new()
            } else {
                format!(" ({})", d.notes.join("; "))
            }
        );
        for x in &d.checked {
            let e = totals
                .entry((format!("{:?}", x.source), x.name.clone()))
                .or_default();
            e.0 += 1;
            e.1 += usize::from(x.exact);
            if let Some((w, got, want)) = x.first_diff.filter(|_| !x.exact) {
                let _ = writeln!(
                    out,
                    "  {} {} ({:?}, cb {:#x}): word {w}: captured {got:#010x} ({}), inputs {want:#010x} ({})",
                    x.stage,
                    x.name,
                    x.source,
                    x.target,
                    f32::from_bits(got),
                    f32::from_bits(want)
                );
            }
        }
        for v in &c.vertices {
            let _ = writeln!(
                out,
                "  {}: {} bytes {}",
                v.what,
                v.bytes,
                if v.equal { "equal" } else { "DIFFER" }
            );
        }
    }
    for o in &f.order {
        let _ = writeln!(
            out,
            "pass {}: culled {:?}, predicted order {:?}, drawn {:?}: {}",
            o.pass,
            o.culled,
            o.predicted,
            o.drawn,
            if o.in_order {
                "in order"
            } else {
                "OUT OF ORDER"
            }
        );
    }
    for (n, why) in &f.unchecked {
        let _ = writeln!(out, "draw {n}: not checked: {why}");
    }
    let _ = writeln!(out, "== constants (draws x stages: exact/checked) ==");
    for ((src, name), (n, e)) in &totals {
        let _ = writeln!(out, "{src}\t{name}\t{e}/{n}");
    }
    out
}

/// The totals of a check: (constants checked, exact; vertex comparisons,
/// equal).
#[must_use]
pub fn totals(f: &FrameCheck) -> (usize, usize, usize, usize) {
    let mut t = (0, 0, 0, 0);
    for c in &f.draws {
        t.0 += c.draw.checked.len();
        t.1 += c.draw.checked.iter().filter(|x| x.exact).count();
        t.2 += c.vertices.len();
        t.3 += c.vertices.iter().filter(|v| v.equal).count();
    }
    t
}

/// The names checked per source, for the scene-chars table.
#[must_use]
pub fn sources(f: &FrameCheck) -> Vec<(Source, String)> {
    let mut out: Vec<(Source, String)> = Vec::new();
    for c in &f.draws {
        for x in &c.draw.checked {
            if !out.iter().any(|(s, n)| *s == x.source && *n == x.name) {
                out.push((x.source, x.name.clone()));
            }
        }
    }
    out
}
