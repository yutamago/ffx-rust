//! The builder's [`Resources`] made on an `ffx-draw` [`Backend`], and its
//! draw states moved onto the backend's handles: what the in-game map
//! path (R2c) issues.
//!
//! ```ignore
//! let up = Uploaded::new(&mut backend, &built.resources, |e| host_view(e))?;
//! for d in &built.draws {
//!     backend.draw(&up.state(&d.state), d.call);
//! }
//! ```

use ffx_draw::backend::{
    Backend, Blend, Buffer, DepthState, InitData, Layout, Raster, Result, Sampler, Shader, View,
};
use ffx_draw::state::{CbBinding, IndexBinding, StageBindings, VertexBinding};
use ffx_draw::{DrawState, Error};

use crate::build::{Resources, ViewSource};
use crate::frame::External;

/// The backend's handle for each of the builder's.
#[derive(Debug, Clone, Default)]
pub struct Uploaded {
    pub shaders: Vec<Shader>,
    pub layouts: Vec<Layout>,
    pub blends: Vec<Blend>,
    pub depths: Vec<DepthState>,
    pub rasters: Vec<Raster>,
    pub samplers: Vec<Sampler>,
    pub buffers: Vec<Buffer>,
    pub views: Vec<View>,
}

fn get<T: Copy>(list: &[T], i: u32) -> Option<T> {
    list.get(i as usize).copied()
}

impl Uploaded {
    /// Makes every object of `r` on `b`; external views come from the
    /// host (`external`).
    ///
    /// # Errors
    /// When the backend refuses an object, or the host has no view for an
    /// external resource.
    pub fn new<B: Backend>(
        b: &mut B,
        r: &Resources,
        external: impl Fn(External) -> Option<View>,
    ) -> Result<Self> {
        let mut up = Self::default();
        for (stage, code) in &r.shaders {
            up.shaders.push(b.create_shader(*stage, code)?);
        }
        for (elements, vs) in &r.layouts {
            let code = r
                .shaders
                .get(vs.0 as usize)
                .ok_or_else(|| Error("layout of an unknown shader".into()))?;
            up.layouts.push(b.create_layout(elements, &code.1)?);
        }
        for d in &r.blends {
            up.blends.push(b.create_blend(d)?);
        }
        for d in &r.depths {
            up.depths.push(b.create_depth_state(d)?);
        }
        for d in &r.rasters {
            up.rasters.push(b.create_raster(d)?);
        }
        for d in &r.samplers {
            up.samplers.push(b.create_sampler(d)?);
        }
        for (desc, data) in &r.buffers {
            up.buffers.push(b.create_buffer(desc, Some(data))?);
        }
        let mut textures = Vec::new();
        for t in &r.textures {
            let init: Vec<InitData> = t
                .subresources
                .iter()
                .map(|(row_pitch, data)| InitData {
                    row_pitch: *row_pitch,
                    data,
                })
                .collect();
            textures.push(b.create_texture(&t.desc, &init)?);
        }
        for v in &r.views {
            up.views.push(match v.source {
                ViewSource::Texture(t) => {
                    let tex = get(&textures, t.0)
                        .ok_or_else(|| Error("view of an unknown texture".into()))?;
                    b.create_view(tex, v.kind, v.desc.as_ref())?
                }
                ViewSource::External(e) => {
                    external(e).ok_or_else(|| Error(format!("no host view for {e:?}")))?
                }
            });
        }
        Ok(up)
    }

    fn stage(&self, s: &StageBindings) -> StageBindings {
        let mut out = StageBindings {
            shader: s.shader.and_then(|x| get(&self.shaders, x.0)),
            ..StageBindings::default()
        };
        for (o, c) in out.cbs.iter_mut().zip(&s.cbs) {
            *o = c.and_then(|c| {
                get(&self.buffers, c.buffer.0).map(|buffer| CbBinding { buffer, ..c })
            });
        }
        for (o, v) in out.srvs.iter_mut().zip(&s.srvs) {
            *o = v.and_then(|v| get(&self.views, v.0));
        }
        for (o, v) in out.samplers.iter_mut().zip(&s.samplers) {
            *o = v.and_then(|v| get(&self.samplers, v.0));
        }
        out
    }

    /// A built draw state on the backend's handles.
    #[must_use]
    pub fn state(&self, s: &DrawState) -> DrawState {
        let mut out = *s;
        out.layout = s.layout.and_then(|x| get(&self.layouts, x.0));
        for (o, v) in out.vertex.iter_mut().zip(&s.vertex) {
            *o = v.and_then(|v| {
                get(&self.buffers, v.buffer.0).map(|buffer| VertexBinding { buffer, ..v })
            });
        }
        out.index = s.index.and_then(|x| {
            get(&self.buffers, x.buffer.0).map(|buffer| IndexBinding { buffer, ..x })
        });
        out.vs = self.stage(&s.vs);
        out.ps = self.stage(&s.ps);
        out.gs = None;
        out.blend = s.blend.and_then(|x| get(&self.blends, x.0));
        out.depth = s.depth.and_then(|x| get(&self.depths, x.0));
        out.raster = s.raster.and_then(|x| get(&self.rasters, x.0));
        for (o, v) in out.targets.iter_mut().zip(&s.targets) {
            *o = v.and_then(|v| get(&self.views, v.0));
        }
        out.depth_target = s.depth_target.and_then(|v| get(&self.views, v.0));
        out
    }
}
