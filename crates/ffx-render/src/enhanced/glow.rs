//! Lights from the game's glowing effects (save spheres, lamps, magic
//! drawn over the scene after it went to the renderer): the hook sends
//! those draws at Present (`Msg::Effects`); they are drawn again alone,
//! with their nearest depth, and `glow.wgsl` sums light and position per
//! tile. The tiles are read back a frame later and gathered into point
//! lights that stay while the scene lasts (off screen too) and fade where
//! the glow is gone.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use ffx_gfx::{Cmd, Frame, Id, Msg, StateKind, ViewKind};

use super::{lights, math};
use crate::{Filter, Renderer};

pub(super) const GLOW_WGSL: &str = include_str!("glow.wgsl");
/// Ids of the targets in the renderer's mirror (below the effect light's).
const IDS: Id = 0xffe0_0000;
const TILE: u32 = 32;
/// Tiles nearer than this (world units) are one light.
const MERGE: f32 = 12.0;
/// A light found again within this distance is the same light.
const MATCH: f32 = 15.0;
/// At most this many lights from glows.
const MAX_LIGHTS: usize = 16;
/// Glow power (luminance × world area) to light intensity.
const POWER: f32 = 0.4;
/// The uniform: inverse view-projection, size.
const UNIFORM: usize = 64 + 16;

/// A light found in the glows.
#[derive(Debug, Clone)]
pub struct Found {
    pub light: lights::Light,
    /// What the glow says now (the light eases toward it).
    target: f32,
}

struct Targets {
    /// The scene colour's texture description (the copies take it).
    key: (u32, u32, u32),
    /// Colour texture and view, depth texture and view, a scratch colour
    /// texture and view, the depth state that writes.
    ids: [Id; 7],
}

/// Tiles being read back: the buffer is mapped once `done`.
struct Pending {
    done: Arc<AtomicBool>,
    across: u32,
    down: u32,
    /// The camera then (for what is on screen).
    view_proj: math::Mat,
}

pub struct Glow {
    targets: Option<Targets>,
    generation: u32,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    uniform: wgpu::Buffer,
    tiles: Option<(u64, wgpu::Buffer, wgpu::Buffer)>,
    pending: Option<Pending>,
    /// The lights found in this scene.
    pub found: Vec<Found>,
    scene: String,
    /// What the last readback held, for the log.
    pub report: String,
}

fn word(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

impl Glow {
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        use super::{UNFILTERED, layout_entries, tex, uniform};
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glow"),
            entries: &layout_entries(
                &[
                    (0, uniform(UNIFORM as u64)),
                    (1, tex(UNFILTERED)),
                    (2, tex(wgpu::TextureSampleType::Depth)),
                    (
                        3,
                        wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(32),
                        },
                    ),
                ],
                wgpu::ShaderStages::COMPUTE,
            ),
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glow"),
            source: wgpu::ShaderSource::Wgsl(GLOW_WGSL.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("glow"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                }),
            ),
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            targets: None,
            generation: 0,
            layout,
            pipeline,
            uniform: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("glow"),
                size: UNIFORM as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            tiles: None,
            pending: None,
            found: Vec::new(),
            scene: String::new(),
            report: String::new(),
        }
    }

    /// The mirror textures: the glows alone, their depth, the scratch.
    #[must_use]
    pub fn target_ids(&self) -> Vec<Id> {
        self.targets
            .as_ref()
            .map(|t| vec![t.ids[0], t.ids[2], t.ids[4]])
            .unwrap_or_default()
    }

    /// The lights for this frame.
    pub fn lights(&self) -> impl Iterator<Item = &lights::Light> {
        self.found.iter().map(|f| &f.light)
    }

    /// Forgets the lights when the scene changes.
    pub fn scene(&mut self, scene: &str) {
        if self.scene != scene {
            scene.clone_into(&mut self.scene);
            self.found.clear();
        }
    }

    /// Mirror targets like the scene colour `color` and depth `depth`.
    fn targets(&mut self, r: &mut Renderer, color: Id, depth: Id) -> Result<Option<[Id; 7]>> {
        let desc = |r: &Renderer, view: Id| {
            r.views
                .get(&view)
                .and_then(|v| r.textures.get(&v.resource))
                .map(|t| t.desc)
        };
        let (Some(cd), Some(dd)) = (desc(r, color), desc(r, depth)) else {
            return Ok(None);
        };
        let key = (cd[0], cd[1], dd[4]);
        if let Some(t) = self.targets.as_ref().filter(|t| t.key == key) {
            return Ok(Some(t.ids));
        }
        if let Some(old) = self.targets.take() {
            for id in old.ids {
                r.apply(Msg::Release { id }, &Filter::default())?;
            }
        }
        self.generation += 1;
        let base = IDS + self.generation * 8;
        let ids: [Id; 7] = std::array::from_fn(|k| base + k as Id);
        let plain = |mut d: ffx_gfx::TextureDesc, bind: u32| {
            d[2] = 1;
            d[3] = 1;
            d[5] = 1;
            d[6] = 0;
            d[7] = 0;
            d[8] = bind;
            d[9] = 0;
            d[10] = 0;
            d
        };
        for (tex, view, desc, kind) in [
            (ids[0], ids[1], plain(cd, 0x28), ViewKind::RenderTarget),
            (ids[2], ids[3], plain(dd, 0x48), ViewKind::DepthStencil),
            (ids[4], ids[5], plain(cd, 0x28), ViewKind::RenderTarget),
        ] {
            r.apply(
                Msg::Texture {
                    id: tex,
                    desc,
                    subresources: Vec::new(),
                },
                &Filter::default(),
            )?;
            r.apply(
                Msg::View {
                    id: view,
                    kind,
                    resource: tex,
                    desc: None,
                },
                &Filter::default(),
            )?;
        }
        // Depth on, written, nearer wins; no stencil.
        let mut state = Vec::new();
        for v in [1u32, 1, 2, 0, 0, 1, 1, 1, 8, 1, 1, 1, 8] {
            state.extend_from_slice(&word(v));
        }
        state[16] = 0xff;
        state[17] = 0xff;
        r.apply(
            Msg::State {
                id: ids[6],
                kind: StateKind::Depth,
                desc: state,
            },
            &Filter::default(),
        )?;
        self.targets = Some(Targets { key, ids });
        Ok(Some(ids))
    }

    /// The effects of the last frame (`view_proj`, unjittered, is its
    /// camera): reads back what the last call found, then measures these.
    ///
    /// # Errors
    /// When they cannot be drawn.
    #[allow(clippy::too_many_lines)] // two replays, a reduction, the readback
    pub fn effects(
        &mut self,
        r: &mut Renderer,
        frame: &Frame,
        view_proj: &math::Mat,
        strength: f32,
    ) -> Result<()> {
        self.collect(strength);
        if self.pending.is_some() {
            return Ok(());
        }
        let Some((color, depth)) = frame.cmds.iter().find_map(|c| match c {
            Cmd::Draw(d) => d.rtv.first().map(|x| (x.1, d.dsv)),
            _ => None,
        }) else {
            return Ok(());
        };
        let Some(ids) = self.targets(r, color, depth)? else {
            return Ok(());
        };
        // The glows alone onto black against the scene's depth, then again
        // for their own nearest depth.
        let mut cmds = vec![
            Cmd::ClearTarget {
                view: ids[1],
                color: [0.0; 4],
            },
            Cmd::ClearTarget {
                view: ids[5],
                color: [0.0; 4],
            },
            Cmd::ClearDepth {
                view: ids[3],
                flags: 1,
                depth: 1.0,
                stencil: 0,
            },
        ];
        let mut again = Vec::new();
        for c in &frame.cmds {
            match c {
                Cmd::UpdateBuffer { .. } => cmds.push(c.clone()),
                Cmd::Draw(d) => {
                    let mut lit = d.clone();
                    lit.rtv = vec![(0, ids[1])];
                    cmds.push(Cmd::Draw(lit));
                    let mut near = d.clone();
                    near.rtv = vec![(0, ids[5])];
                    near.dsv = ids[3];
                    near.depth = ids[6];
                    again.push(Cmd::Draw(near));
                }
                _ => {}
            }
        }
        cmds.extend(again);
        r.draw_frame(
            &Frame {
                number: frame.number,
                width: frame.width,
                height: frame.height,
                cmds,
            },
            &Filter::default(),
        )?;
        let (Some(color_view), Some(depth_view)) =
            (r.texture_view(ids[1], false), r.texture_view(ids[3], true))
        else {
            return Ok(());
        };
        let Some((w, h)) = r
            .views
            .get(&ids[1])
            .and_then(|v| r.textures.get(&v.resource))
            .map(|t| (t.desc[0], t.desc[1]))
        else {
            return Ok(());
        };
        let across = w.div_ceil(TILE);
        let down = h.div_ceil(TILE);
        let bytes = u64::from(across * down) * 32;
        let device = r.device.clone();
        let queue = r.queue.clone();
        if self.tiles.as_ref().is_none_or(|t| t.0 < bytes) {
            let make = |usage| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("glow tiles"),
                    size: bytes,
                    usage,
                    mapped_at_creation: false,
                })
            };
            self.tiles = Some((
                bytes,
                make(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC),
                make(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST),
            ));
        }
        let (_, tiles, readback) = self.tiles.as_ref().expect("made");
        let mut u = Vec::with_capacity(UNIFORM);
        super::put_rows(&mut u, &math::transpose(&math::invert(view_proj)));
        super::put4(&mut u, [w as f32, h as f32, across as f32, 0.04]);
        queue.write_buffer(&self.uniform, 0, &u);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: tiles,
                        offset: 0,
                        size: wgpu::BufferSize::new(bytes),
                    }),
                },
            ],
        });
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("glow"),
        });
        {
            let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("glow"),
                timestamp_writes: None,
            });
            cp.set_pipeline(&self.pipeline);
            cp.set_bind_group(0, &group, &[]);
            cp.dispatch_workgroups(across.div_ceil(8), down.div_ceil(8), 1);
        }
        enc.copy_buffer_to_buffer(tiles, 0, readback, 0, bytes);
        queue.submit([enc.finish()]);
        let done = Arc::new(AtomicBool::new(false));
        let flag = done.clone();
        readback
            .slice(..bytes)
            .map_async(wgpu::MapMode::Read, move |res| {
                flag.store(res.is_ok(), Ordering::Release);
            });
        self.pending = Some(Pending {
            done,
            across,
            down,
            view_proj: *view_proj,
        });
        Ok(())
    }

    /// Takes in the tiles read back, when they are there.
    fn collect(&mut self, strength: f32) {
        let Some(p) = self.pending.as_ref() else {
            return;
        };
        if !p.done.load(Ordering::Acquire) {
            return;
        }
        let Some((_, _, readback)) = self.tiles.as_ref() else {
            return;
        };
        let bytes = u64::from(p.across * p.down) * 32;
        let floats: Vec<f32> = match readback.slice(..bytes).get_mapped_range() {
            Ok(data) => data
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| f32::from_le_bytes(*c))
                .collect(),
            Err(_) => Vec::new(),
        };
        readback.unmap();
        let view_proj = p.view_proj;
        self.pending = None;
        let clusters = gather(&floats);
        let lit = floats
            .as_chunks::<8>()
            .0
            .iter()
            .filter(|t| t[0] > 1e-3)
            .count();
        self.report = format!("{lit} glowing tiles, {} groups", clusters.len());
        self.update(&clusters, &view_proj, strength);
    }

    /// Matches what the glows show now with the lights found before.
    fn update(&mut self, clusters: &[Cluster], view_proj: &math::Mat, strength: f32) {
        let mut seen = vec![false; self.found.len()];
        for c in clusters.iter().take(MAX_LIGHTS) {
            let intensity = c.power * POWER * strength;
            let near = self
                .found
                .iter()
                .enumerate()
                .filter(|(k, f)| !seen[*k] && dist(f.light.position, c.position) < MATCH)
                .min_by(|a, b| {
                    dist(a.1.light.position, c.position)
                        .total_cmp(&dist(b.1.light.position, c.position))
                })
                .map(|(k, _)| k);
            match near {
                Some(k) => {
                    seen[k] = true;
                    let f = &mut self.found[k];
                    for i in 0..3 {
                        f.light.position[i] += (c.position[i] - f.light.position[i]) * 0.3;
                        f.light.color[i] += (c.color[i] - f.light.color[i]) * 0.3;
                    }
                    f.target = intensity;
                }
                None if self.found.len() < MAX_LIGHTS => {
                    seen.push(true);
                    self.found.push(Found {
                        light: lights::Light {
                            name: format!("Glow {}", self.found.len() + 1),
                            position: c.position,
                            color: c.color,
                            intensity: 0.0,
                            specular: 0.5,
                            ..lights::Light::default()
                        },
                        target: intensity,
                    });
                }
                None => {}
            }
        }
        // Gone where it should show: it fades. Off screen it stays.
        for (k, f) in self.found.iter_mut().enumerate() {
            if !seen[k] && on_screen(view_proj, f.light.position) {
                f.target = 0.0;
            }
            f.light.intensity += (f.target - f.light.intensity) * 0.3;
            f.light.range = (50.0 * f.light.intensity.max(0.0).sqrt()).clamp(15.0, 250.0);
        }
        self.found
            .retain(|f| f.target > 0.0 || f.light.intensity > 0.01);
    }
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn on_screen(m: &math::Mat, p: [f32; 3]) -> bool {
    let c: [f32; 4] =
        std::array::from_fn(|r| m[r][0] * p[0] + m[r][1] * p[1] + m[r][2] * p[2] + m[r][3]);
    c[3] > 0.1 && (c[0] / c[3]).abs() < 0.9 && (c[1] / c[3]).abs() < 0.9
}

/// A group of glowing tiles: where, what colour (brightest channel 1), how
/// strong.
#[derive(Debug, Clone, PartialEq)]
struct Cluster {
    position: [f32; 3],
    color: [f32; 3],
    power: f32,
}

/// Tiles (8 floats each, `Tile` in `glow.wgsl`) into clusters, strongest
/// first.
fn gather(floats: &[f32]) -> Vec<Cluster> {
    // (Σ lum, Σ lum·p, Σ colour, power)
    let mut sums: Vec<(f32, [f32; 3], [f32; 3], f32)> = Vec::new();
    let mut tiles: Vec<&[f32; 8]> = floats
        .as_chunks::<8>()
        .0
        .iter()
        .filter(|t| t[0] > 1e-3 && t.iter().all(|x| x.is_finite()))
        .collect();
    tiles.sort_by(|a, b| b[7].total_cmp(&a[7]));
    for t in tiles {
        let p = [t[1] / t[0], t[2] / t[0], t[3] / t[0]];
        let near = sums
            .iter_mut()
            .find(|s| dist([s.1[0] / s.0, s.1[1] / s.0, s.1[2] / s.0], p) < MERGE);
        match near {
            Some(s) => {
                s.0 += t[0];
                for i in 0..3 {
                    s.1[i] += t[1 + i];
                    s.2[i] += t[4 + i];
                }
                s.3 += t[7];
            }
            None => sums.push((t[0], [t[1], t[2], t[3]], [t[4], t[5], t[6]], t[7])),
        }
    }
    let mut out: Vec<Cluster> = sums
        .into_iter()
        .map(|(lum, p, c, power)| {
            let top = c[0].max(c[1]).max(c[2]).max(1e-6);
            Cluster {
                position: [p[0] / lum, p[1] / lum, p[2] / lum],
                color: [c[0] / top, c[1] / top, c[2] / top],
                power,
            }
        })
        .collect();
    out.sort_by(|a, b| b.power.total_cmp(&a.power));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_tiles_are_one_light() {
        let tile = |lum: f32, p: [f32; 3], power: f32| {
            vec![
                lum,
                lum * p[0],
                lum * p[1],
                lum * p[2],
                lum * 0.5,
                lum,
                lum * 0.25,
                power,
            ]
        };
        let mut floats = tile(1.0, [0.0, 0.0, 0.0], 2.0);
        floats.extend(tile(1.0, [4.0, 0.0, 0.0], 1.0));
        floats.extend(tile(0.5, [100.0, 0.0, 0.0], 5.0));
        floats.extend(vec![0.0; 8]);
        let c = gather(&floats);
        assert_eq!(c.len(), 2);
        assert!((c[0].power - 5.0).abs() < 1e-6);
        assert!((c[1].position[0] - 2.0).abs() < 1e-4);
        assert!((c[1].color[1] - 1.0).abs() < 1e-6);
    }
}
