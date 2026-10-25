//! The builder's draws in a capture's terms, for `ffx-drawdiff`: a copy
//! of a captured frame whose map draws are replaced by the builder's,
//! each object the builder made declared under a fresh id.

use std::collections::HashMap;

use ffx_draw::DrawCall;
use ffx_draw::backend::{Blend, Buffer, DepthState, Layout, Raster, Sampler, Shader, View};
use ffx_draw::desc::{Stage, ViewKind};
use ffx_draw::state::StageBindings;
use ffx_gfx::{Cmd, Draw, DrawKind, Frame, Id, Msg, StateKind};

use crate::build::{Built, Pass, ViewSource};
use crate::capture::Capture;
use crate::frame::External;

/// Fresh ids start here (the game's object addresses are below).
const FIRST_ID: Id = 0xf000_0000;

fn view_kind(k: ViewKind) -> ffx_gfx::ViewKind {
    match k {
        ViewKind::ShaderResource => ffx_gfx::ViewKind::ShaderResource,
        ViewKind::RenderTarget => ffx_gfx::ViewKind::RenderTarget,
        ViewKind::DepthStencil => ffx_gfx::ViewKind::DepthStencil,
    }
}

struct Ids {
    shaders: Vec<Id>,
    layouts: Vec<Id>,
    blends: Vec<Id>,
    depths: Vec<Id>,
    rasters: Vec<Id>,
    samplers: Vec<Id>,
    buffers: Vec<Id>,
    views: Vec<Id>,
}

fn at(list: &[Id], i: u32) -> Id {
    list.get(i as usize).copied().unwrap_or(0)
}

#[allow(clippy::too_many_lines)] // one list per object kind
/// The builder's objects as capture messages, under fresh ids; external
/// views take the id `external` gives (a captured view's).
fn objects(b: &Built, external: &dyn Fn(External) -> Option<Id>) -> (Vec<Msg>, Ids, Vec<String>) {
    let r = &b.resources;
    let mut next = FIRST_ID;
    let mut fresh = || {
        next += 1;
        next
    };
    let mut msgs = Vec::new();
    let mut missing = Vec::new();
    let mut ids = Ids {
        shaders: Vec::new(),
        layouts: Vec::new(),
        blends: Vec::new(),
        depths: Vec::new(),
        rasters: Vec::new(),
        samplers: Vec::new(),
        buffers: Vec::new(),
        views: Vec::new(),
    };
    for (stage, code) in &r.shaders {
        let id = fresh();
        let kind = if *stage == Stage::Vertex { b'v' } else { b'p' };
        msgs.push(Msg::Shader {
            id,
            kind,
            bytecode: code.clone(),
        });
        ids.shaders.push(id);
    }
    for (elements, _) in &r.layouts {
        let id = fresh();
        msgs.push(Msg::Layout {
            id,
            elements: elements
                .iter()
                .map(|e| ffx_gfx::Element {
                    semantic: e.semantic.clone(),
                    index: e.index,
                    format: e.format,
                    slot: e.slot,
                    offset: e.offset,
                    per_instance: e.per_instance,
                    step: e.step,
                })
                .collect(),
        });
        ids.layouts.push(id);
    }
    let mut state = |kind: StateKind, desc: Vec<u8>, list: &mut Vec<Id>, msgs: &mut Vec<Msg>| {
        let id = fresh();
        msgs.push(Msg::State { id, kind, desc });
        list.push(id);
    };
    for d in &r.blends {
        state(StateKind::Blend, d.to_d3d11(), &mut ids.blends, &mut msgs);
    }
    for d in &r.depths {
        state(StateKind::Depth, d.to_d3d11(), &mut ids.depths, &mut msgs);
    }
    for d in &r.rasters {
        state(StateKind::Raster, d.to_d3d11(), &mut ids.rasters, &mut msgs);
    }
    for d in &r.samplers {
        state(
            StateKind::Sampler,
            d.to_d3d11(),
            &mut ids.samplers,
            &mut msgs,
        );
    }
    let mut next = FIRST_ID + 0x0100_0000;
    let mut fresh = || {
        next += 1;
        next
    };
    for (desc, data) in &r.buffers {
        let id = fresh();
        msgs.push(Msg::Buffer {
            id,
            desc: desc.words(),
            data: data.clone(),
        });
        ids.buffers.push(id);
    }
    let mut textures = Vec::new();
    for t in &r.textures {
        let id = fresh();
        msgs.push(Msg::Texture {
            id,
            desc: t.desc.words(),
            subresources: t
                .subresources
                .iter()
                .map(|(row_pitch, data)| ffx_gfx::Subresource {
                    row_pitch: *row_pitch,
                    data: data.clone(),
                })
                .collect(),
        });
        textures.push(id);
    }
    for v in &r.views {
        match v.source {
            ViewSource::Texture(t) => {
                let id = fresh();
                msgs.push(Msg::View {
                    id,
                    kind: view_kind(v.kind),
                    resource: at(&textures, t.0),
                    desc: v.desc.map(|d| d.words()),
                });
                ids.views.push(id);
            }
            ViewSource::External(e) => {
                let id = external(e).unwrap_or_else(|| {
                    missing.push(format!("no captured view for {e:?}"));
                    0
                });
                ids.views.push(id);
            }
        }
    }
    (msgs, ids, missing)
}

fn draw(d: &crate::build::MapDraw, ids: &Ids, cbs: &dyn Fn(Buffer) -> Vec<u8>) -> Draw {
    let s = &d.state;
    let stage_cbs = |st: &StageBindings| -> Vec<(u8, Vec<u8>)> {
        (0u8..)
            .zip(st.cbs.iter())
            .filter_map(|(i, c)| c.map(|c| (i, cbs(c.buffer))))
            .collect()
    };
    let views = |list: &[Option<View>]| -> Vec<(u8, Id)> {
        (0u8..)
            .zip(list)
            .filter_map(|(i, v)| v.map(|v| (i, at(&ids.views, v.0))))
            .collect()
    };
    let samplers = |list: &[Option<Sampler>]| -> Vec<(u8, Id)> {
        (0u8..)
            .zip(list)
            .filter_map(|(i, v)| v.map(|v| (i, at(&ids.samplers, v.0))))
            .collect()
    };
    let (kind, count, start, base) = match d.call {
        DrawCall::Indexed { count, start, base } => (DrawKind::Indexed, count, start, base),
        DrawCall::Plain { count, start } => (DrawKind::Plain, count, start, 0),
        DrawCall::IndexedInstanced {
            count, start, base, ..
        } => (DrawKind::IndexedInstanced, count, start, base),
        DrawCall::Instanced { count, start, .. } => (DrawKind::Instanced, count, start, 0),
    };
    Draw {
        kind,
        count,
        instances: 1,
        start,
        base,
        first_instance: 0,
        topology: s.topology,
        layout: s.layout.map_or(0, |Layout(l)| at(&ids.layouts, l)),
        vs: s.vs.shader.map_or(0, |Shader(x)| at(&ids.shaders, x)),
        ps: s.ps.shader.map_or(0, |Shader(x)| at(&ids.shaders, x)),
        vertex: (0u8..)
            .zip(s.vertex.iter())
            .filter_map(|(i, v)| v.map(|v| (i, at(&ids.buffers, v.buffer.0), v.stride, v.offset)))
            .collect(),
        index: s.index.map_or((0, 0, 0), |x| {
            (at(&ids.buffers, x.buffer.0), x.format, x.offset)
        }),
        vs_cb: stage_cbs(&s.vs),
        ps_cb: stage_cbs(&s.ps),
        vs_srv: views(&s.vs.srvs),
        ps_srv: views(&s.ps.srvs),
        vs_samplers: samplers(&s.vs.samplers),
        ps_samplers: samplers(&s.ps.samplers),
        blend: s.blend.map_or(0, |Blend(x)| at(&ids.blends, x)),
        blend_factor: s.blend_factor,
        sample_mask: s.sample_mask,
        depth: s.depth.map_or(0, |DepthState(x)| at(&ids.depths, x)),
        stencil_ref: s.stencil_ref,
        raster: s.raster.map_or(0, |Raster(x)| at(&ids.rasters, x)),
        viewport: s.viewport,
        scissor: s.scissor,
        rtv: views(&s.targets),
        dsv: s.depth_target.map_or(0, |View(v)| at(&ids.views, v)),
    }
}

/// The messages of a capture of `cap`'s frame with the map draws
/// `replaced` (scene draw numbers: the opaque run, then the transparent
/// run) swapped for the builder's draws of each pass. Errors name the
/// external views that have no captured counterpart.
///
/// # Errors
/// When `replaced` is not two runs of consecutive scene draws, or an
/// external view has no captured id.
pub fn replace_map_draws(
    cap: &Capture,
    built: &Built,
    replaced: &[usize],
    external: &dyn Fn(External) -> Option<Id>,
) -> Result<Vec<Msg>, String> {
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for &n in replaced {
        match runs.last_mut() {
            Some(r) if r.last() == Some(&(n - 1)) => r.push(n),
            _ => runs.push(vec![n]),
        }
    }
    if runs.len() != 2 {
        return Err(format!(
            "the map draws are {} runs of scene draws, not two (opaque, transparent)",
            runs.len()
        ));
    }
    let (mut msgs, ids, missing) = objects(built, external);
    if !missing.is_empty() {
        return Err(missing.join("; "));
    }
    let cbs = |b: Buffer| {
        built
            .resources
            .buffers
            .get(b.0 as usize)
            .map(|x| x.1.clone())
            .unwrap_or_default()
    };
    let draws = |pass: Pass| -> Vec<Cmd> {
        built
            .pass(pass)
            .map(|d| Cmd::Draw(Box::new(draw(d, &ids, &cbs))))
            .collect()
    };
    let (opaque, transparent) = (draws(Pass::Opaque), draws(Pass::Transparent));
    let mut scene_number: HashMap<usize, usize> = HashMap::new();
    for (n, (cmd, _)) in cap
        .draws()
        .filter(|(_, d)| Capture::scene_targets(d).is_some())
        .enumerate()
    {
        scene_number.insert(cmd, n);
    }
    let mut cmds = Vec::new();
    for (i, c) in cap.frame.cmds.iter().enumerate() {
        match scene_number.get(&i) {
            Some(n) if *n == runs[0][0] => cmds.extend(opaque.iter().cloned()),
            Some(n) if *n == runs[1][0] => cmds.extend(transparent.iter().cloned()),
            Some(n) if replaced.contains(n) => {}
            _ => cmds.push(c.clone()),
        }
    }
    let mut out: Vec<Msg> = cap.objects.values().cloned().collect();
    out.append(&mut msgs);
    if let Some(w) = &cap.world {
        out.push(w.clone());
    }
    out.push(Msg::Frame(Frame {
        number: cap.frame.number,
        width: cap.frame.width,
        height: cap.frame.height,
        cmds,
    }));
    Ok(out)
}

/// A capture file of messages (`capture.ffxgfx` bytes).
#[must_use]
pub fn capture_bytes(msgs: &[Msg]) -> Vec<u8> {
    let mut out = ffx_gfx::CAPTURE_MAGIC.to_vec();
    for m in msgs {
        m.write(&mut out);
    }
    out
}
