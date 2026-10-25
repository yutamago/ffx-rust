//! The character builder against a capture of the original
//! (`docs/formats/scene-chars.md`, "The builder"): the frame's records
//! and the `chr/` models build the character world's draws
//! ([`crate::chars_build`]), which replace the captured character draws
//! (the census's, [`crate::chars_check::is_char_draw`]) for
//! `ffx-drawdiff`.
//!
//! What the records don't hold comes from the capture, as for the map
//! builder's gate (`docs/scene-builder.md`, "Inputs from a capture"): the
//! scene-wide buffer, viewport and scissor from the map's draws
//! ([`CaptureInputs`]), and the host's objects (the scene targets, the
//! shadow map the blob shadow samples) from the replaced draws.

use std::collections::{HashMap, HashSet};

use ffx_gfx::Id;
use ffx_gfx::scene_inputs::SceneInputs as Record;

use crate::assets::AssetIndex;
use crate::build::{Built, Clusters, Pass, ViewSource};
use crate::capture::Capture;
use crate::capture_inputs::CaptureInputs;
use crate::chars_build::{self, CharClusters, DrawTrace, FrameInputs, Light, Role};
use crate::chars_model::ChrIndex;
use crate::frame::{External, SceneInputs as _, Settings};

/// The archive paths of every character model cluster.
#[must_use]
pub fn chr_models(archive: &ffx_vbf::Archive) -> Vec<String> {
    archive
        .entries()
        .iter()
        .map(|e| e.path.clone())
        .filter(|p| p.contains("/ps3data/chr/") && p.ends_with(".dae.phyre"))
        .collect()
}

/// The map model of a scene (`cdsp0000` ->
/// `.../map/cdsp/cdsp00/mdl/d3d11/cdsp00.dae.phyre`).
#[must_use]
pub fn map_model(scene: &str) -> String {
    let s = scene.to_lowercase();
    let map = s.get(..6).unwrap_or(&s);
    format!(
        "ffx_data/gamedata/ps3data{}mdl/d3d11/{map}.dae.phyre",
        crate::chars_check::map_folder(&s)
    )
}

/// One capture's run of the character builder.
#[derive(Debug)]
pub struct Outcome {
    pub built: Built,
    /// The captured character draws replaced, per pass (scene draw
    /// numbers).
    pub opaque: Vec<usize>,
    pub transparent: Vec<usize>,
    /// The models the records' instances resolved to.
    pub models: Vec<String>,
    /// The rebuilt capture's messages.
    pub msgs: Vec<ffx_gfx::Msg>,
    /// The inputs read back from the capture: lights per record instance,
    /// run-time material textures.
    pub lights: HashMap<usize, Vec<Light>>,
    pub material_textures: HashSet<(usize, u32)>,
    /// Constant words after a bone list taken from the capture
    /// ([`take_unwritten`]), and the draws where that is unsound
    /// ([`blend_index_problems`]: none expected).
    pub unwritten_words: usize,
    pub blend_problems: Vec<String>,
}

/// The host objects of the built draws: each external view takes the id
/// the captured draw at the same place of its pass bound in the same
/// register.
fn externals(
    cap: &Capture,
    built: &Built,
    numbers: &[(Pass, Vec<usize>)],
) -> HashMap<External, Id> {
    let scene: Vec<&ffx_gfx::Draw> = cap
        .draws()
        .filter(|(_, d)| Capture::scene_targets(d).is_some())
        .map(|(_, d)| d)
        .collect();
    let mut out = HashMap::new();
    let views = &built.resources.views;
    let external = |v: ffx_draw::backend::View| match views.get(v.0 as usize)?.source {
        ViewSource::External(e) => Some(e),
        ViewSource::Texture(_) => None,
    };
    for (pass, list) in numbers {
        for (d, &n) in built.pass(*pass).zip(list) {
            let Some(cd) = scene.get(n) else { continue };
            let at =
                |l: &[(u8, Id)], r: usize| l.iter().find(|x| usize::from(x.0) == r).map(|x| x.1);
            let s = &d.state;
            for (r, v) in s.targets.iter().enumerate() {
                if let (Some(e), Some(id)) = (v.and_then(external), at(&cd.rtv, r)) {
                    out.entry(e).or_insert(id);
                }
            }
            if let Some(e) = s.depth_target.and_then(external) {
                out.entry(e).or_insert(cd.dsv);
            }
            for (srvs, captured) in [(&s.vs.srvs, &cd.vs_srv), (&s.ps.srvs, &cd.ps_srv)] {
                for (r, v) in srvs.iter().enumerate() {
                    if let (Some(e), Some(id)) = (v.and_then(external), at(captured, r)) {
                        out.entry(e).or_insert(id);
                    }
                }
            }
        }
    }
    out
}

/// The captured scene draws, and per pass the built draws (with their
/// traces) paired with the captured draw at the same place.
fn pairs<'a>(
    cap: &'a Capture,
    built: &'a Built,
    traces: &'a [DrawTrace],
    numbers: &'a [(Pass, Vec<usize>)],
) -> Vec<(usize, &'a ffx_gfx::Draw, &'a DrawTrace)> {
    let scene: Vec<&ffx_gfx::Draw> = cap
        .draws()
        .filter(|(_, d)| Capture::scene_targets(d).is_some())
        .map(|(_, d)| d)
        .collect();
    let mut out = Vec::new();
    for (pass, list) in numbers {
        let draws = (0..built.draws.len()).filter(|&k| built.draws[k].pass == *pass);
        for (k, &n) in draws.zip(list) {
            if let (Some(cd), Some(t)) = (scene.get(n), traces.get(k)) {
                out.push((k, *cd, t));
            }
        }
    }
    out
}

/// The `$Globals` bytes a captured draw bound for a stage, at the slot
/// the built draw binds its buffer.
fn captured_cb<'a>(d: &crate::build::MapDraw, cd: &'a ffx_gfx::Draw, vertex: bool) -> &'a [u8] {
    let stage = if vertex { &d.state.vs } else { &d.state.ps };
    let Some(slot) = stage.cbs.iter().position(Option::is_some) else {
        return &[];
    };
    let captured = if vertex { &cd.vs_cb } else { &cd.ps_cb };
    captured
        .iter()
        .find(|x| usize::from(x.0) == slot)
        .map_or(&[][..], |x| x.1.as_slice())
}

/// The node context lights of each record instance, read back from the
/// captured draws through the built draws' light slots. A stand-in: the
/// scene inputs record doesn't hold the character lighting's `PLight`s.
#[must_use]
pub fn lights(
    cap: &Capture,
    built: &Built,
    traces: &[DrawTrace],
    numbers: &[(Pass, Vec<usize>)],
) -> HashMap<usize, Vec<Light>> {
    let mut out: HashMap<usize, Vec<Light>> = HashMap::new();
    for (k, cd, t) in pairs(cap, built, traces, numbers) {
        let d = &built.draws[k];
        for s in &t.lights {
            let cb = captured_cb(d, cd, s.vertex);
            let (at, size) = (s.target as usize, s.size.min(16) as usize);
            let Some(bytes) = cb.get(at..at + size) else {
                continue;
            };
            let list = out.entry(d.instance).or_default();
            if list.len() <= s.light as usize {
                list.resize(s.light as usize + 1, Light::default());
            }
            let l = &mut list[s.light as usize];
            let words = if s.direction {
                &mut l.direction
            } else {
                &mut l.color
            };
            for (i, w) in bytes.as_chunks::<4>().0.iter().enumerate() {
                words[i] = u32::from_le_bytes(*w);
            }
        }
    }
    out
}

/// The texture slots the game set at run time though the cluster leaves
/// them empty: the built draws' empty slots where the captured draw binds
/// a view. A stand-in, as the map gate's run-time material textures
/// (`docs/scene-builder.md`): the record doesn't hold the materials'
/// texture slots.
#[must_use]
pub fn material_textures(
    cap: &Capture,
    built: &Built,
    traces: &[DrawTrace],
    numbers: &[(Pass, Vec<usize>)],
) -> HashSet<(usize, u32)> {
    let mut out = HashSet::new();
    for (_, cd, t) in pairs(cap, built, traces, numbers) {
        for s in &t.empty_textures {
            let srvs = if s.vertex { &cd.vs_srv } else { &cd.ps_srv };
            if srvs.iter().any(|x| u32::from(x.0) == s.register) {
                out.insert((s.material, s.offset));
            }
        }
    }
    out
}

/// Gives each built draw's unwritten constant bytes (after the last bone
/// of `BoneTransforms`) the captured draw's: the render thread's copy
/// stops at the bone list's end marker (`0x0059a650`), so the game's
/// buffer holds there whatever an earlier draw left, and the shader reads
/// none of it (every blend index of the segment is below its bone count,
/// [`blend_index_problems`]). Returns the words taken.
pub fn take_unwritten(
    cap: &Capture,
    built: &mut Built,
    traces: &[DrawTrace],
    numbers: &[(Pass, Vec<usize>)],
) -> usize {
    let mut writes = Vec::new();
    for (k, cd, t) in pairs(cap, built, traces, numbers) {
        let d = &built.draws[k];
        for (vertex, r) in &t.unwritten {
            let stage = if *vertex { &d.state.vs } else { &d.state.ps };
            let Some(buffer) = stage.cbs.iter().flatten().next().map(|c| c.buffer) else {
                continue;
            };
            let cb = captured_cb(d, cd, *vertex);
            let (a, b) = (r.start as usize, r.end as usize);
            if let Some(bytes) = cb.get(a..b) {
                writes.push((buffer, a, bytes.to_vec()));
            }
        }
    }
    let mut words = 0;
    for (buffer, at, bytes) in writes {
        if let Some((_, data)) = built.resources.buffers.get_mut(buffer.0 as usize)
            && let Some(dst) = data.get_mut(at..at + bytes.len())
        {
            dst.copy_from_slice(&bytes);
            words += bytes.len() / 4;
        }
    }
    words
}

/// The built draws whose segment has a blend index at or past its bone
/// count (each a description): the words after the last bone are only
/// unread when there are none.
#[must_use]
pub fn blend_index_problems(c: &CharClusters, s: &Record, built: &Built) -> Vec<String> {
    let roles = chars_build::resolve(c, s);
    let mut out = Vec::new();
    let cw = s.chars.as_deref();
    for d in &built.draws {
        let Some(Role::Model(fi)) = roles.get(d.instance).copied() else {
            continue;
        };
        let bones = cw
            .and_then(|w| w.instances.get(d.instance))
            .and_then(|i| i.skin_bones.get(d.segment))
            .map_or(0, Vec::len);
        if bones == 0 {
            continue;
        }
        let segment = c.map.instances[fi].segments[d.segment];
        let seg = &c.map.segments[segment];
        let Some(block) = seg
            .blocks
            .iter()
            .find(|b| b.streams.iter().any(|s| s.render_type == "SkinIndices"))
        else {
            out.push(format!(
                "instance {} segment {}: no SkinIndices",
                d.instance, d.segment
            ));
            continue;
        };
        let max = c
            .map
            .bytes
            .get(block.bytes.clone())
            .and_then(|b| b.iter().max().copied())
            .unwrap_or(0);
        if usize::from(max) >= bones {
            out.push(format!(
                "instance {} segment {}: blend index {max}, {bones} bones",
                d.instance, d.segment
            ));
        }
    }
    out
}

/// Builds a capture's character world and swaps it in.
///
/// `index` must hold the shaders and the `chr/` models (for the census),
/// `chr` the candidate models.
///
/// # Errors
/// When the capture has no character records, the map inputs can't be
/// read, or the build fails.
pub fn run(
    bytes: &[u8],
    archive: &ffx_vbf::Archive,
    index: &AssetIndex,
    chr: &ChrIndex,
    settings: Settings,
) -> Result<Outcome, String> {
    let read = |p: &str| archive.find(p).and_then(|e| archive.read(e).ok());
    let cap = Capture::read(bytes).map_err(|e| format!("capture: {e:?}"))?;
    let all = crate::inputs::read(bytes).map_err(|e| format!("inputs: {e:?}"))?;
    let records = chars_build::pass_records(&all, cap.frame.number)
        .ok_or("no Opaque and Transparent character-world records")?;
    let mut models = chr.models_for(records[0]);
    for m in chr.models_for(records[1]) {
        if !models.contains(&m) {
            models.push(m);
        }
    }
    let c = CharClusters::load(read, &models).map_err(|e| e.0)?;

    // The map's inputs (scene-wide buffer, viewport, scissor).
    let scene = crate::chars_check::scene_of(bytes).ok_or("capture without a scene")?;
    let map = Clusters::load(read, &map_model(&scene)).map_err(|e| e.0)?;
    let map_inputs = CaptureInputs::new(&cap, &map, settings, false)?;
    let tables = c
        .effects
        .values()
        .chain(c.shadow.iter().map(|s| &s.variant))
        .flat_map(|v| v.passes.iter())
        .flat_map(|rp| rp.shaders.iter())
        .flat_map(|sh| sh.passes.iter())
        .flat_map(|sp| [&sp.vs_params, &sp.ps_params]);
    let mut frame = FrameInputs {
        settings,
        viewport: map_inputs.viewport(),
        scissor: map_inputs.scissor(),
        scene_wide: chars_build::scene_wide_to_memory(&map_inputs.scene_wide(), tables),
        lights: HashMap::new(),
        material_textures: HashSet::new(),
    };
    let (first, traces) = chars_build::build_traced(&c, records, &frame).map_err(|e| e.0)?;

    // The captured character draws, split as the built passes are.
    let census = crate::census::census(&cap, index);
    let mut numbers: Vec<usize> = census
        .iter()
        .filter(|d| crate::chars_check::is_char_draw(d))
        .map(|d| d.number)
        .collect();
    numbers.sort_unstable();
    let n_opaque = first.pass(Pass::Opaque).count().min(numbers.len());
    let transparent = numbers.split_off(n_opaque);
    let opaque = numbers;
    let passes = [
        (Pass::Opaque, opaque.clone()),
        (Pass::Transparent, transparent.clone()),
    ];
    // What the records don't hold, from the captured draws.
    frame.lights = lights(&cap, &first, &traces, &passes);
    frame.material_textures = material_textures(&cap, &first, &traces, &passes);
    let (mut built, traces) = chars_build::build_traced(&c, records, &frame).map_err(|e| e.0)?;
    let blend_problems = blend_index_problems(&c, records[0], &built);
    let unwritten_words = take_unwritten(&cap, &mut built, &traces, &passes);
    let ext = externals(&cap, &built, &passes);
    let msgs = crate::emit::replace_map_draws(&cap, &built, &opaque, &transparent, &|e| {
        ext.get(&e).copied().or_else(|| map_inputs.external_view(e))
    })?;
    Ok(Outcome {
        built,
        opaque,
        transparent,
        models,
        msgs,
        lights: frame.lights,
        material_textures: frame.material_textures,
        unwritten_words,
        blend_problems,
    })
}
