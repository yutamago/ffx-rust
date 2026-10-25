//! Scene inputs derived from a capture (round 2's stand-in for the live
//! snapshot, `docs/scene-builder.md`, "Inputs from a capture").
//!
//! The capture holds the constant buffers the original filled, so the
//! game state they were filled from can be read back through the same
//! location tables the builder fills them with, for every map draw (its
//! segment found by its index and vertex buffers, its shader pass by its
//! two programs):
//!
//! - the scene-wide buffer from the scene-wide entries;
//! - each drawn instance's `World` from the per-node entries (an instance
//!   no draw shows gets the `World` most drawn instances have, as the
//!   census does);
//! - the material values the game writes at run time
//!   ([`RUNTIME_MATERIAL_VALUES`]) from the material entries; every other
//!   material value stays as stored in the cluster;
//! - the camera from `scene.View` and `scene.ViewProjection`, the culling
//!   matrix by taking the latter's depth back to -w..w (`z' = 2z - w`);
//! - an instance that passes the box test but draws nothing is taken as
//!   hidden by the game (`m_meshInstance` 0; cdsp0000's
//!   `FBXASC056_0Shape`, round 2 Q3).
//!
//! - the render orders are the file's with the class the game adds at
//!   map init (`m_DObjKind` 1).
//!
//! What a capture can't tell (hidden bytes, depth biases) is taken as the
//! cluster has it.

use std::collections::HashMap;

use ffx_gfx::Draw;

use crate::build::Clusters;
use crate::capture::Capture;
use crate::effect::{ShaderPass, Source};
use crate::frame::{Camera, External, Mat, SceneInputs, Settings, SlotTexture};

/// Material values the game writes at run time, taken from the capture:
/// the shadow cascades `PhyreScene_RenderShadow` (`0x00653600`) writes into
/// the map materials (`docs/formats/scene.md`, "Game state").
pub const RUNTIME_MATERIAL_VALUES: [&str; 4] = [
    "LightViewProjection",
    "LVP2",
    "MySMScaleBias",
    "MySMScaleBias2",
];

/// A map draw of the capture: its scene draw number, segment, instance,
/// material and shader pass.
#[derive(Debug, Clone)]
pub struct Matched {
    pub number: usize,
    pub segment: usize,
    pub instance: usize,
    pub material: usize,
    pub effect: String,
    /// Whether the shader pass is of an opaque render pass type.
    pub opaque: bool,
}

/// Scene inputs read back from a capture.
#[derive(Debug, Clone)]
pub struct CaptureInputs {
    pub settings: Settings,
    pub msaa: bool,
    pub camera: Camera,
    pub worlds: HashMap<usize, Mat>,
    pub common_world: Option<Mat>,
    pub hidden: Vec<usize>,
    pub scene_wide: Vec<u8>,
    pub materials: HashMap<usize, Vec<u8>>,
    pub viewport: [f32; 6],
    pub scissor: [i32; 4],
    /// The captured view id of each external resource.
    pub externals: HashMap<External, u32>,
    /// The map draws found, in capture order.
    pub matched: Vec<Matched>,
    instance_count: usize,
    /// Per instance: the file's render order and `m_DObjKind`.
    render_orders: Vec<(i32, i32)>,
}

fn word_slice(d: &Draw, vertex: bool, slot: u32) -> Option<&[u8]> {
    let list = if vertex { &d.vs_cb } else { &d.ps_cb };
    list.iter()
        .find(|c| u32::from(c.0) == slot)
        .map(|c| c.1.as_slice())
}

/// A 4x4 variable of a vertex shader's `$Globals` by name (used or not).
fn named_matrix(cap: &Capture, d: &Draw, name: &str) -> Option<Mat> {
    let code = cap.shader(d.vs)?;
    let c = ffx_dxbc::Container::parse(code).ok()?;
    let r = c.reflection().ok()??;
    for b in &r.buffers {
        let slot = r
            .bindings
            .iter()
            .find(|x| x.kind == ffx_dxbc::BindKind::ConstantBuffer && x.name == b.name)
            .map_or(0, |x| x.slot);
        for v in &b.variables {
            let mut hits = vec![(v.name.clone(), v.offset)];
            hits.extend(
                v.members
                    .iter()
                    .map(|m| (format!("{}.{}", v.name, m.name), v.offset + m.offset)),
            );
            if let Some((_, off)) = hits.into_iter().find(|h| h.0 == name) {
                let bytes = word_slice(d, true, slot)?.get(off as usize..off as usize + 64)?;
                return Some(std::array::from_fn(|k| {
                    f32::from_le_bytes([
                        bytes[4 * k],
                        bytes[4 * k + 1],
                        bytes[4 * k + 2],
                        bytes[4 * k + 3],
                    ])
                }));
            }
        }
    }
    None
}

fn bits(m: &Mat) -> [u32; 16] {
    m.map(f32::to_bits)
}

impl CaptureInputs {
    /// Reads the inputs of the map world from a capture.
    ///
    /// # Errors
    /// When no scene draw of the capture is a draw of the map.
    #[allow(clippy::too_many_lines)] // one pass over the map draws
    pub fn new(
        cap: &Capture,
        c: &Clusters,
        settings: Settings,
        msaa: bool,
    ) -> Result<Self, String> {
        let map = &c.map;
        let mut matched = Vec::new();
        let mut scene_wide = Vec::new();
        let mut node_worlds: HashMap<usize, Mat> = HashMap::new();
        let mut materials: HashMap<usize, Vec<u8>> = (0..map.materials.len())
            .map(|m| (m, map.materials[m].buffer.clone()))
            .collect();
        let mut externals = HashMap::new();
        let mut viewport = None;
        let mut camera_draw = None;
        for found in crate::gate::map_draws(cap, c) {
            let crate::gate::Found {
                number,
                cmd,
                segment,
                instances: candidates,
                material,
                effect,
                ..
            } = found;
            let Some(ffx_gfx::Cmd::Draw(d)) = cap.frame.cmds.get(cmd) else {
                continue;
            };
            let d: &Draw = d;
            let first = candidates[0];
            let (Some(vs), Some(ps)) = (cap.shader(d.vs), cap.shader(d.ps)) else {
                continue;
            };
            let Some((pass_type, sp)) = find_pass(c, &effect, vs, ps) else {
                continue;
            };
            let variant = &c.effects[&effect];
            Self::invert(d, sp, true, &mut scene_wide, None);
            Self::invert(d, sp, false, &mut scene_wide, None);
            // World from the per-node entries.
            let mut world = None;
            if let Some(w) = variant.untweakable.iter().find(|u| u.kind == 0x82) {
                let mut node =
                    vec![0u8; usize::from(variant.untweakable_size).max(w.offset as usize + 64)];
                for vertex in [true, false] {
                    Self::invert_group(d, sp, vertex, Source::Node, &mut node, None);
                }
                let at = w.offset as usize;
                let m: Mat = std::array::from_fn(|k| {
                    f32::from_le_bytes([
                        node[at + 4 * k],
                        node[at + 4 * k + 1],
                        node[at + 4 * k + 2],
                        node[at + 4 * k + 3],
                    ])
                });
                world = m.iter().any(|x| *x != 0.0).then_some(m);
            }
            // The instance: one already seen with this World (another of
            // its segments), else the first not seen yet.
            let instance = world
                .and_then(|w| {
                    candidates
                        .iter()
                        .copied()
                        .find(|i| node_worlds.get(i).is_some_and(|x| bits(x) == bits(&w)))
                })
                .or_else(|| {
                    candidates
                        .iter()
                        .copied()
                        .find(|i| !node_worlds.contains_key(i))
                })
                .unwrap_or(first);
            if let Some(w) = world {
                node_worlds.insert(instance, w);
            }
            // Material values written at run time.
            let buf = materials
                .entry(material)
                .or_insert_with(|| map.materials[material].buffer.clone());
            let runtime: Vec<(u32, u32)> = variant
                .tweakable
                .iter()
                .filter(|t| RUNTIME_MATERIAL_VALUES.contains(&t.name.as_str()))
                .map(|t| (t.offset, t.size))
                .collect();
            for vertex in [true, false] {
                Self::invert_group(d, sp, vertex, Source::Material, buf, Some(&runtime));
            }
            // Textures the game set into the material.
            for (table, srvs) in [(&sp.ps_textures, &d.ps_srv), (&sp.vs_textures, &d.vs_srv)] {
                for (src, loc) in table.sourced() {
                    if src != Source::Material || loc.target == u32::MAX {
                        continue;
                    }
                    let off = u32::from(loc.offset);
                    let is_texture = variant
                        .tweakable_at(loc.offset)
                        .is_some_and(|t| matches!(t.kind, 0x42 | 0x44));
                    let in_file = map.materials[material]
                        .textures
                        .iter()
                        .any(|t| t.offset == off);
                    if is_texture
                        && !in_file
                        && let Some(&(_, view)) = srvs.iter().find(|s| u32::from(s.0) == loc.target)
                    {
                        externals.insert(
                            External::MaterialTexture {
                                material,
                                offset: off,
                            },
                            view,
                        );
                    }
                }
            }
            if let Some(t) = Capture::scene_targets(d) {
                externals.insert(External::Color, t.color);
                externals.insert(External::Normal, t.normal);
                externals.insert(External::Depth, t.depth);
            }
            viewport.get_or_insert((d.viewport, d.scissor));
            camera_draw.get_or_insert(d);
            matched.push(Matched {
                number,
                segment,
                instance,
                material,
                effect,
                opaque: pass_type.starts_with("Opaque"),
            });
        }
        let d = camera_draw.ok_or("no scene draw of the capture is a draw of the map")?;
        let view = named_matrix(cap, d, "scene.View").ok_or("no scene.View")?;
        let view_projection =
            named_matrix(cap, d, "scene.ViewProjection").ok_or("no scene.ViewProjection")?;
        let mut cull = view_projection;
        for k in 0..4 {
            cull[8 + k] = 2.0 * view_projection[8 + k] - view_projection[12 + k];
        }
        let camera = Camera {
            view,
            view_projection,
            cull,
        };
        // The World most drawn instances share.
        let mut counts: Vec<(Mat, usize)> = Vec::new();
        for w in node_worlds.values() {
            match counts.iter_mut().find(|c| bits(&c.0) == bits(w)) {
                Some(c) => c.1 += 1,
                None => counts.push((*w, 1)),
            }
        }
        counts.sort_by_key(|c| std::cmp::Reverse(c.1));
        let common_world = counts.first().map(|c| c.0);
        let mut me = Self {
            settings,
            msaa,
            camera,
            worlds: node_worlds,
            common_world,
            hidden: Vec::new(),
            scene_wide,
            materials,
            viewport: viewport.map_or([0.0; 6], |v| v.0),
            scissor: viewport.map_or([0; 4], |v| v.1),
            externals,
            matched,
            instance_count: map.instances.len(),
            render_orders: map
                .instances
                .iter()
                .map(|i| (i.bounds.map_or(0, |b| b.render_order), i.dobj_kind))
                .collect(),
        };
        // Instances the box test keeps that draw nothing: hidden.
        let drawn: std::collections::HashSet<usize> =
            me.matched.iter().map(|m| m.instance).collect();
        for (i, inst) in map.instances.iter().enumerate() {
            let Some(b) = inst.bounds else { continue };
            if drawn.contains(&i) {
                continue;
            }
            let world = me.world(i);
            let m = crate::cull::cull_matrix(&me.camera.cull, world.as_ref());
            if crate::cull::box_visible(b.min, b.size, &m) {
                me.hidden.push(i);
            }
        }
        Ok(me)
    }

    fn invert(
        d: &Draw,
        sp: &ShaderPass,
        vertex: bool,
        scene_wide: &mut Vec<u8>,
        only: Option<&[(u32, u32)]>,
    ) {
        Self::invert_group(d, sp, vertex, Source::SceneWide, scene_wide, only);
    }

    /// Copies the captured constant buffer's values of one source group
    /// back into that source's buffer (growing it as needed), optionally
    /// only the given (offset, size) values.
    fn invert_group(
        d: &Draw,
        sp: &ShaderPass,
        vertex: bool,
        group: Source,
        out: &mut Vec<u8>,
        only: Option<&[(u32, u32)]>,
    ) {
        let (prog, table) = if vertex {
            (&sp.vs, &sp.vs_params)
        } else {
            (&sp.ps, &sp.ps_params)
        };
        if prog.global_cb == u32::MAX {
            return;
        }
        let Some(cb) = word_slice(d, vertex, prog.global_cb) else {
            return;
        };
        for (src, loc) in table.sourced() {
            if src != group || loc.target == u32::MAX {
                continue;
            }
            if let Some(list) = only
                && !list.contains(&(u32::from(loc.offset), loc.size))
            {
                continue;
            }
            let (o, t, n) = (
                usize::from(loc.offset),
                loc.target as usize,
                loc.size as usize,
            );
            let Some(from) = cb.get(t..t + n) else {
                continue;
            };
            if out.len() < o + n {
                out.resize(o + n, 0);
            }
            if group == Source::Material && crate::build::is_matrix(loc) {
                out[o..o + n].copy_from_slice(&crate::build::transpose_bytes(from));
            } else {
                out[o..o + n].copy_from_slice(from);
            }
        }
    }

    /// The matched scene draw numbers of the opaque and the transparent
    /// pass.
    #[must_use]
    pub fn pass_numbers(&self) -> (Vec<usize>, Vec<usize>) {
        let pick = |o: bool| {
            self.matched
                .iter()
                .filter(|m| m.opaque == o)
                .map(|m| m.number)
                .collect()
        };
        (pick(true), pick(false))
    }

    /// The captured view id of an external resource.
    #[must_use]
    pub fn external_view(&self, e: External) -> Option<u32> {
        self.externals.get(&e).copied()
    }
}

use crate::gate::find_pass;

impl SceneInputs for CaptureInputs {
    fn settings(&self) -> Settings {
        self.settings
    }

    fn camera(&self) -> Camera {
        self.camera
    }

    fn pass_names(&self) -> (String, String) {
        if self.msaa {
            ("Opaque".into(), "Transparent".into())
        } else {
            ("OpaqueNoMSAA".into(), "TransparentNoMSAA".into())
        }
    }

    fn bounds_order(&self) -> Vec<usize> {
        (0..self.instance_count).collect()
    }

    fn instance_shown(&self, instance: usize) -> bool {
        !self.hidden.contains(&instance)
    }

    fn world(&self, instance: usize) -> Option<Mat> {
        self.worlds.get(&instance).copied().or(self.common_world)
    }

    /// The file's order, plus 100,000,000 for `m_DObjKind` 1 (what
    /// `MorphManager_registerClusterMorphs` `0x006aca10` adds at map init,
    /// `docs/formats/scene-maps.md`, Q6).
    fn render_order(&self, instance: usize) -> Option<i32> {
        let (b, kind) = self.render_orders.get(instance).copied()?;
        Some(if kind == 1 {
            b.wrapping_add(100_000_000)
        } else {
            b
        })
    }

    fn segment_hidden(&self, _segment: usize) -> bool {
        false
    }

    fn depth_bias(&self, _instance: usize) -> Option<(f32, f32)> {
        None
    }

    fn material_buffer(&self, material: usize) -> Vec<u8> {
        self.materials.get(&material).cloned().unwrap_or_default()
    }

    fn material_texture(&self, material: usize, offset: u32, file: Option<&str>) -> SlotTexture {
        match file {
            Some(f) => SlotTexture::Import(f.to_owned()),
            None if self
                .externals
                .contains_key(&External::MaterialTexture { material, offset }) =>
            {
                SlotTexture::Runtime
            }
            None => SlotTexture::Empty,
        }
    }

    fn scene_wide(&self) -> Vec<u8> {
        self.scene_wide.clone()
    }

    fn viewport(&self) -> [f32; 6] {
        self.viewport
    }

    fn scissor(&self) -> [i32; 4] {
        self.scissor
    }
}
