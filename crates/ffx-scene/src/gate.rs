//! The builder gate's view of a capture: which captured scene draws are
//! the map's (found by their geometry, not by anything the builder
//! computes), and in which pass. Works on full and hash-only captures
//! (`gfxcapture ... hash`: static vertex buffers hold their bytes' hash,
//! index buffers their bytes), and with morphing meshes, whose `Vertex`
//! stream comes from the dynamic ring.

use std::collections::HashMap;

use ffx_gfx::Draw;

use crate::build::Clusters;
use crate::capture::Capture;
use crate::effect::ShaderPass;

/// A captured map draw.
#[derive(Debug, Clone)]
pub struct Found {
    /// Its number among the frame's scene draws, and its command index.
    pub number: usize,
    pub cmd: usize,
    pub segment: usize,
    /// The instances whose mesh holds the segment (several share a mesh).
    pub instances: Vec<usize>,
    /// The first instance's material for it, and that material's effect.
    pub material: usize,
    pub effect: String,
    /// Whether the shader pass belongs to an opaque render pass.
    pub opaque: bool,
}

/// The render pass type and shader pass of an effect variant (the
/// material's first, then any loaded one) whose programs are these.
#[must_use]
pub fn find_pass<'a>(
    c: &'a Clusters,
    effect: &str,
    vs: &[u8],
    ps: &[u8],
) -> Option<(&'a str, &'a ShaderPass)> {
    let own = c.effects.get(effect).into_iter();
    let rest = c.effects.values();
    own.chain(rest).find_map(|v| {
        v.passes.iter().find_map(|p| {
            p.shaders
                .iter()
                .flat_map(|s| &s.passes)
                .find(|sp| sp.vs.code == vs && sp.ps.code == ps)
                .map(|sp| (p.pass_type.as_str(), sp))
        })
    })
}

/// The map draws of a capture's frame, in order: scene draws whose index
/// buffer is a segment's and whose static vertex buffers are that
/// segment's blocks (by content), with a dynamic buffer allowed only for
/// a morphing instance's segment.
#[must_use]
pub fn map_draws(cap: &Capture, c: &Clusters) -> Vec<Found> {
    let map = &c.map;
    let mut by_indices: HashMap<u64, Vec<usize>> = HashMap::new();
    for (s, seg) in map.segments.iter().enumerate() {
        let ix = map.bytes.get(seg.indices.clone()).unwrap_or_default();
        by_indices
            .entry(crate::key::static_buffer(ix, true))
            .or_default()
            .push(s);
    }
    let block_hash = |s: usize, b: usize| {
        crate::key::content(
            map.bytes
                .get(map.segments[s].blocks[b].bytes.clone())
                .unwrap_or_default(),
        )
    };
    let dynamic_segment = |s: usize| {
        map.instances
            .iter()
            .any(|i| i.dynamic && i.segments.contains(&s))
    };
    let mut out = Vec::new();
    let scene = cap
        .draws()
        .filter(|(_, d)| Capture::scene_targets(d).is_some());
    for (number, (cmd, d)) in scene.enumerate() {
        let Some(found) = segment_of(cap, c, d, &by_indices, &block_hash, &dynamic_segment) else {
            continue;
        };
        let instances: Vec<usize> = (0..map.instances.len())
            .filter(|&i| map.instances[i].segments.contains(&found))
            .collect();
        let Some(&first) = instances.first() else {
            continue;
        };
        let Some(material) = map.material(first, found) else {
            continue;
        };
        let effect = map.materials[material].effect.clone();
        let (Some(vs), Some(ps)) = (cap.shader(d.vs), cap.shader(d.ps)) else {
            continue;
        };
        let Some((pass_type, _)) = find_pass(c, &effect, vs, ps) else {
            continue;
        };
        out.push(Found {
            number,
            cmd,
            segment: found,
            instances,
            material,
            effect,
            opaque: pass_type.starts_with("Opaque"),
        });
    }
    out
}

fn segment_of(
    cap: &Capture,
    c: &Clusters,
    d: &Draw,
    by_indices: &HashMap<u64, Vec<usize>>,
    block_hash: &dyn Fn(usize, usize) -> u64,
    dynamic_segment: &dyn Fn(usize) -> bool,
) -> Option<usize> {
    let (desc, data) = cap.buffer(d.index.0)?;
    let candidates = by_indices.get(&crate::key::buffer(desc, data))?;
    candidates.iter().copied().find(|&s| {
        let blocks = c.map.segments[s].blocks.len();
        d.vertex.iter().all(|v| match cap.vertex_content(v.1) {
            Some(h) => (0..blocks).any(|b| block_hash(s, b) == h),
            // A dynamic buffer: the morph output.
            None => cap.buffer(v.1).is_some() && dynamic_segment(s),
        })
    })
}

/// The command indices of the map draws of each pass (opaque, then
/// transparent).
#[must_use]
pub fn pass_cmds(found: &[Found]) -> (Vec<usize>, Vec<usize>) {
    let pick = |o: bool| {
        found
            .iter()
            .filter(|f| f.opaque == o)
            .map(|f| f.cmd)
            .collect()
    };
    (pick(true), pick(false))
}

/// The model paths of a frame's records: the map world's clusters, by
/// their asset ids (the [`crate::inputs::Wire`] block), else the scene's
/// model by its symbol (`cdsp00`: `map/cdsp/cdsp00/`).
#[must_use]
pub fn model_paths(
    records: &[(crate::inputs::SceneInputs, Option<crate::inputs::Wire>)],
) -> Vec<String> {
    let Some((s, wire)) = records
        .iter()
        .find(|(s, _)| s.world == crate::live::MAP_WORLD)
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (k, cl) in s.clusters.iter().enumerate() {
        let asset = wire
            .as_ref()
            .and_then(|w| w.cluster_assets.get(k))
            .filter(|a| !a.is_empty());
        let path = match asset {
            Some(a) => crate::map::archive_path(a),
            None if cl.name.len() >= 6 => format!(
                "ffx_data/gamedata/ps3data/map/{}/{}/mdl/d3d11/{}.dae.phyre",
                &cl.name[..4],
                cl.name,
                cl.name
            ),
            None => continue,
        };
        if !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// `EyePosition` from the first map draw's constants, for version 1
/// records (they lack the camera's world matrix).
#[must_use]
pub fn capture_fallback(cap: &Capture, found: &[Found]) -> crate::live::Fallback {
    let Some(Some(ffx_gfx::Cmd::Draw(d))) = found.first().map(|f| cap.frame.cmds.get(f.cmd)) else {
        return crate::live::Fallback::default();
    };
    let eye =
        crate::inputs::draw_matrix(cap, d, "scene.EyePosition").map(|m| [m[0], m[1], m[2], m[3]]);
    crate::live::Fallback { eye_position: eye }
}

/// One gate run: the builder's map draws from a frame's snapshot against
/// the frame's captured map draws.
#[derive(Debug)]
pub struct Run {
    /// Captured map draws per pass, built draws per pass.
    pub captured: (usize, usize),
    pub built: (usize, usize),
    pub a_items: usize,
    pub b_items: usize,
    /// Differences no allow rule covers.
    pub unallowed: usize,
    pub report: String,
    /// The builder's and the inputs' notes.
    pub notes: Vec<String>,
    /// Whether the record had the `Wire` block.
    pub wired: bool,
    /// The rebuilt frame (for `drawdiff`).
    pub msgs: Vec<ffx_gfx::Msg>,
}

/// Builds frame `cap`'s map world from `records` (that frame's) alone,
/// and diffs it against the captured map draws (`ffx-drawdiff`, scene
/// part). `read` maps archive paths to bytes; `settings` stand in for a
/// version 1 record's.
///
/// # Errors
/// When the clusters don't load, the records lack the map passes, or the
/// build fails.
pub fn run(
    cap: &Capture,
    records: &[(crate::inputs::SceneInputs, Option<crate::inputs::Wire>)],
    read: &dyn Fn(&str) -> Option<Vec<u8>>,
    settings: crate::frame::Settings,
    allow: Vec<ffx_drawdiff::rules::Rule>,
    limit: usize,
    label: &str,
) -> Result<Run, String> {
    let paths = model_paths(records);
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    let mut c = Clusters::load_models(read, &refs).map_err(|e| e.to_string())?;
    let found = map_draws(cap, &c);
    let fallback = capture_fallback(cap, &found);
    let wired = records.iter().any(|r| r.1.is_some());
    let fallback = if wired {
        crate::live::Fallback::default()
    } else {
        fallback
    };
    let live = crate::live::LiveInputs::new(&c, records, settings, &fallback)?;
    c.add_textures(read, &live.texture_imports())
        .map_err(|e| e.to_string())?;
    let built = crate::build::build(&c, &live).map_err(|e| e.to_string())?;
    let (opaque, transparent) = pass_cmds(&found);
    let msgs = crate::emit::replace_map_draws(cap, &built, &opaque, &transparent)?;
    let a = ffx_drawdiff::capture::Capture::from_msgs(&crate::emit::frame_msgs(cap));
    let b = ffx_drawdiff::capture::Capture::from_msgs(&msgs);
    let opts = ffx_drawdiff::diff::Options {
        part: ffx_drawdiff::diff::Part::Scene,
        tolerances: Vec::new(),
        allow,
    };
    let cmp = ffx_drawdiff::diff::compare(&a, &b, &opts);
    let style = ffx_drawdiff::report::Style {
        limit,
        ..Default::default()
    };
    let report = ffx_drawdiff::report::render((label, "built"), &a, &b, &cmp, &opts, style);
    let (a_items, b_items) = cmp
        .frames
        .first()
        .map_or((0, 0), |f| (f.a_items, f.b_items));
    let mut notes = live.notes.clone();
    notes.extend(built.notes.iter().cloned());
    Ok(Run {
        captured: (opaque.len(), transparent.len()),
        built: (
            built.pass(crate::build::Pass::Opaque).count(),
            built.pass(crate::build::Pass::Transparent).count(),
        ),
        a_items,
        b_items,
        unallowed: cmp.unallowed(),
        report,
        notes,
        wired,
        msgs,
    })
}
