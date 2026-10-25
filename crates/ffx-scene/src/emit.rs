//! The builder's draws in a capture's terms, for `ffx-drawdiff`: a copy
//! of a captured frame whose map draws are replaced by the builder's,
//! each object the builder made declared under a fresh id.

use std::collections::HashMap;

use ffx_draw::DrawCall;
use ffx_draw::backend::{Blend, DepthState, Layout, Raster, Sampler, Shader, View};
use ffx_draw::desc::{Stage, ViewKind};
use ffx_gfx::{Cmd, Draw, DrawKind, Frame, Id, Msg, StateKind};

use crate::build::{Built, Pass, ViewSource};
use crate::capture::Capture;

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
/// The builder's objects as capture messages, under fresh ids (in a
/// hash-only capture's form when `hashed`); external views and buffers
/// get id 0 here and the captured draw's objects in [`draw`].
fn objects(b: &Built, hashed: bool) -> (Vec<Msg>, Ids) {
    let r = &b.resources;
    let mut next = FIRST_ID;
    let mut fresh = || {
        next += 1;
        next
    };
    let mut msgs = Vec::new();
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
    for (k, (desc, data)) in r.buffers.iter().enumerate() {
        if r.external_buffers.contains_key(&handle(k)) {
            ids.buffers.push(0);
            continue;
        }
        let id = fresh();
        let vertices = desc.bind & 1 != 0 && desc.usage != ffx_draw::desc::USAGE_DYNAMIC;
        let data = if hashed && vertices {
            crate::key::content(data).to_le_bytes().to_vec()
        } else {
            data.clone()
        };
        msgs.push(Msg::Buffer {
            id,
            desc: desc.words(),
            data,
        });
        ids.buffers.push(id);
    }
    let mut textures = Vec::new();
    for t in &r.textures {
        let id = fresh();
        let subresources = if hashed {
            vec![ffx_gfx::Subresource {
                row_pitch: 0,
                data: texture_hash(&t.subresources).to_le_bytes().to_vec(),
            }]
        } else {
            t.subresources
                .iter()
                .map(|(row_pitch, data)| ffx_gfx::Subresource {
                    row_pitch: *row_pitch,
                    data: data.clone(),
                })
                .collect()
        };
        msgs.push(Msg::Texture {
            id,
            desc: t.desc.words(),
            subresources,
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
            ViewSource::External(_) => ids.views.push(0),
        }
    }
    (msgs, ids)
}

fn handle(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A texture's content hash as a hash-only capture holds it (`ffx-hook`
/// `gfx/capture.rs` `texture_hash`): FNV-1a 64 of each subresource's
/// rows, chained `(h ^ part) * prime`. The builder's rows are packed.
fn texture_hash(subresources: &[(u32, Vec<u8>)]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (_, data) in subresources {
        h = (h ^ crate::key::content(data)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// One built draw as a captured one. External objects (scene targets,
/// run-time textures, the dynamic ring's vertices) are the host's: they
/// take `host`'s (the captured draw in that place) in the same slot.
fn draw(d: &crate::build::MapDraw, r: &crate::build::Resources, ids: &Ids, host: &Draw) -> Draw {
    let s = &d.state;
    let external_view = |v: View| {
        r.views
            .get(v.0 as usize)
            .is_some_and(|x| matches!(x.source, ViewSource::External(_)))
    };
    let host_slot = |list: &[(u8, Id)], i: u8| list.iter().find(|x| x.0 == i).map_or(0, |x| x.1);
    let stage_cbs = |stage: Stage| -> Vec<(u8, Vec<u8>)> {
        d.constants
            .iter()
            .filter(|c| c.0 == stage)
            .map(|c| (c.1, c.2.clone()))
            .collect()
    };
    let views = |list: &[Option<View>], host: &[(u8, Id)]| -> Vec<(u8, Id)> {
        (0u8..)
            .zip(list)
            .filter_map(|(i, v)| {
                v.map(|v| {
                    if external_view(v) {
                        (i, host_slot(host, i))
                    } else {
                        (i, at(&ids.views, v.0))
                    }
                })
            })
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
            .filter_map(|(i, v)| {
                v.map(|v| {
                    if r.external_buffers.contains_key(&v.buffer.0) {
                        host.vertex
                            .iter()
                            .find(|x| x.0 == i)
                            .copied()
                            .unwrap_or((i, 0, v.stride, 0))
                    } else {
                        (i, at(&ids.buffers, v.buffer.0), v.stride, v.offset)
                    }
                })
            })
            .collect(),
        index: s.index.map_or((0, 0, 0), |x| {
            (at(&ids.buffers, x.buffer.0), x.format, x.offset)
        }),
        vs_cb: stage_cbs(Stage::Vertex),
        ps_cb: stage_cbs(Stage::Pixel),
        vs_srv: views(&s.vs.srvs, &host.vs_srv),
        ps_srv: views(&s.ps.srvs, &host.ps_srv),
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
        rtv: views(&s.targets, &host.rtv),
        dsv: s.depth_target.map_or(0, |v| {
            if external_view(v) {
                host.dsv
            } else {
                at(&ids.views, v.0)
            }
        }),
    }
}

/// The messages of a capture of `cap`'s frame with its map draws swapped
/// for the builder's, in place: the k-th built draw of a pass takes the
/// place of the k-th captured map draw of that pass (`opaque` and
/// `transparent`: command indices, [`crate::gate::pass_cmds`]), so the
/// commands between them (ring updates) stay where they were. Extra built
/// draws follow the pass's last captured one; captured draws the builder
/// has no draw for are dropped. External objects take the captured draw's
/// in that place (the pass's first for extras). In a hash-only capture
/// the builder's textures and vertex buffers take that form.
///
/// # Errors
/// When a pass has no captured map draw.
pub fn replace_map_draws(
    cap: &Capture,
    built: &Built,
    opaque: &[usize],
    transparent: &[usize],
) -> Result<Vec<Msg>, String> {
    let (Some(&last_opaque), Some(&last_transparent)) = (opaque.last(), transparent.last()) else {
        return Err("no captured map draw of a pass".into());
    };
    let (mut msgs, ids) = objects(built, cap.is_hashed());
    let captured = |cmd: usize| match cap.frame.cmds.get(cmd) {
        Some(Cmd::Draw(d)) => Some(&**d),
        _ => None,
    };
    let mut place: HashMap<usize, Vec<Cmd>> = HashMap::new();
    for (pass, list, last) in [
        (Pass::Opaque, opaque, last_opaque),
        (Pass::Transparent, transparent, last_transparent),
    ] {
        let first = captured(list[0]).ok_or("a listed command is no draw")?;
        for (k, d) in built.pass(pass).enumerate() {
            let at = list.get(k).copied();
            let host = at.and_then(captured).unwrap_or(first);
            let cmd = Cmd::Draw(Box::new(draw(d, &built.resources, &ids, host)));
            place.entry(at.unwrap_or(last)).or_default().push(cmd);
        }
    }
    let mut cmds = Vec::new();
    for (i, c) in cap.frame.cmds.iter().enumerate() {
        let listed = opaque.contains(&i) || transparent.contains(&i);
        if !listed {
            cmds.push(c.clone());
        }
        // The built draw for this place; after a pass's last, its extras.
        if let Some(mut built_here) = place.remove(&i) {
            cmds.append(&mut built_here);
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

/// The messages of `cap`'s frame as it is (objects, `World`, the frame):
/// a single-frame capture of one frame of a multi-frame one.
#[must_use]
pub fn frame_msgs(cap: &Capture) -> Vec<Msg> {
    let mut out: Vec<Msg> = cap.objects.values().cloned().collect();
    if let Some(w) = &cap.world {
        out.push(w.clone());
    }
    out.push(Msg::Frame(cap.frame.clone()));
    out
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
