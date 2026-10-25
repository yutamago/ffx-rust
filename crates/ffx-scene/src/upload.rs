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
    /// External buffers (`Resources::external_buffers`) are [`HOST`].
    pub buffers: Vec<Buffer>,
    pub textures: Vec<ffx_draw::backend::Texture>,
    pub views: Vec<View>,
}

/// The handle of an object the host binds per draw (an external buffer;
/// an external view when the host gives this one).
pub const HOST: u32 = u32::MAX;

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
        up.extend(b, r, external)?;
        Ok(up)
    }

    /// Makes the objects `r` holds past those made so far (`r` grows by
    /// appending: `build::build_into`), so a host keeps one `Uploaded`
    /// across frames.
    ///
    /// # Errors
    /// As [`Self::new`]; what was made before the failure stays.
    pub fn extend<B: Backend>(
        &mut self,
        b: &mut B,
        r: &Resources,
        external: impl Fn(External) -> Option<View>,
    ) -> Result<()> {
        for (stage, code) in r.shaders.iter().skip(self.shaders.len()) {
            self.shaders.push(b.create_shader(*stage, code)?);
        }
        for (elements, vs) in r.layouts.iter().skip(self.layouts.len()) {
            let code = r
                .shaders
                .get(vs.0 as usize)
                .ok_or_else(|| Error("layout of an unknown shader".into()))?;
            self.layouts.push(b.create_layout(elements, &code.1)?);
        }
        for d in r.blends.iter().skip(self.blends.len()) {
            self.blends.push(b.create_blend(d)?);
        }
        for d in r.depths.iter().skip(self.depths.len()) {
            self.depths.push(b.create_depth_state(d)?);
        }
        for d in r.rasters.iter().skip(self.rasters.len()) {
            self.rasters.push(b.create_raster(d)?);
        }
        for d in r.samplers.iter().skip(self.samplers.len()) {
            self.samplers.push(b.create_sampler(d)?);
        }
        for k in self.buffers.len()..r.buffers.len() {
            let (desc, data) = &r.buffers[k];
            let host = u32::try_from(k).is_ok_and(|k| r.external_buffers.contains_key(&k));
            self.buffers.push(if host {
                Buffer(HOST)
            } else {
                b.create_buffer(desc, Some(data))?
            });
        }
        for t in r.textures.iter().skip(self.textures.len()) {
            let init: Vec<InitData> = t
                .subresources
                .iter()
                .map(|(row_pitch, data)| InitData {
                    row_pitch: *row_pitch,
                    data,
                })
                .collect();
            self.textures.push(b.create_texture(&t.desc, &init)?);
        }
        for v in r.views.iter().skip(self.views.len()) {
            let view = match v.source {
                ViewSource::Texture(t) => {
                    let tex = get(&self.textures, t.0)
                        .ok_or_else(|| Error("view of an unknown texture".into()))?;
                    b.create_view(tex, v.kind, v.desc.as_ref())?
                }
                ViewSource::External(e) => {
                    external(e).ok_or_else(|| Error(format!("no host view for {e:?}")))?
                }
            };
            self.views.push(view);
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use ffx_draw::DrawCall;
    use ffx_draw::backend::{Readback, Texture};
    use ffx_draw::desc::{
        BlendDesc, BufferDesc, DepthStencilDesc, InputElement, RasterDesc, SamplerDesc, Stage,
        TextureDesc, ViewDesc, ViewKind,
    };

    use super::*;
    use crate::build::{TextureData, ViewData};

    /// Hands out handles from 101 on and lists the objects made.
    #[derive(Default)]
    struct Counter {
        next: u32,
        made: Vec<&'static str>,
    }

    impl Counter {
        fn id(&mut self, what: &'static str) -> u32 {
            self.made.push(what);
            self.next += 1;
            100 + self.next
        }
    }

    impl Backend for Counter {
        fn begin(&mut self) {}
        fn end(&mut self) {}
        fn create_shader(&mut self, _: Stage, _: &[u8]) -> Result<Shader> {
            Ok(Shader(self.id("shader")))
        }
        fn create_layout(&mut self, _: &[InputElement], _: &[u8]) -> Result<Layout> {
            Ok(Layout(self.id("layout")))
        }
        fn create_blend(&mut self, _: &BlendDesc) -> Result<Blend> {
            Ok(Blend(self.id("blend")))
        }
        fn create_depth_state(&mut self, _: &DepthStencilDesc) -> Result<DepthState> {
            Ok(DepthState(self.id("depth")))
        }
        fn create_raster(&mut self, _: &RasterDesc) -> Result<Raster> {
            Ok(Raster(self.id("raster")))
        }
        fn create_sampler(&mut self, _: &SamplerDesc) -> Result<Sampler> {
            Ok(Sampler(self.id("sampler")))
        }
        fn create_buffer(&mut self, _: &BufferDesc, _: Option<&[u8]>) -> Result<Buffer> {
            Ok(Buffer(self.id("buffer")))
        }
        fn create_texture(&mut self, _: &TextureDesc, init: &[InitData]) -> Result<Texture> {
            assert_eq!(init.len(), 1);
            Ok(Texture(self.id("texture")))
        }
        fn create_view(&mut self, _: Texture, _: ViewKind, _: Option<&ViewDesc>) -> Result<View> {
            Ok(View(self.id("view")))
        }
        fn write_buffer(&mut self, _: Buffer, _: u32, _: &[u8]) {}
        fn clear_target(&mut self, _: View, _: [f32; 4]) {}
        fn clear_depth(&mut self, _: View, _: u32, _: f32, _: u8) {}
        fn copy_texture(&mut self, _: Texture, _: Texture) {}
        fn draw(&mut self, _: &DrawState, _: DrawCall) {}
        fn view_texture(&self, _: View) -> Option<Texture> {
            None
        }
        fn read_texture(&mut self, _: Texture) -> Result<Readback> {
            Err(Error("no readback".into()))
        }
    }

    #[test]
    fn objects_are_made_and_states_moved_onto_the_backends_handles() {
        let mut r = Resources::default();
        r.shaders.push((Stage::Vertex, vec![1]));
        r.layouts.push((Vec::new(), Shader(0)));
        r.buffers.push((
            BufferDesc::from_words([16, 2, 4, 0x1_0000, 0, 0]),
            vec![0; 16],
        ));
        r.textures.push(TextureData {
            desc: TextureDesc::from_words([4, 4, 1, 1, 71, 1, 0, 0, 8, 0, 0]),
            subresources: vec![(8, vec![0; 8])],
        });
        r.views.push(ViewData {
            source: ViewSource::Texture(Texture(0)),
            kind: ViewKind::ShaderResource,
            desc: None,
        });
        r.views.push(ViewData {
            source: ViewSource::External(External::Color),
            kind: ViewKind::RenderTarget,
            desc: None,
        });
        let mut b = Counter::default();
        let up = Uploaded::new(&mut b, &r, |e| (e == External::Color).then_some(View(7)))
            .expect("upload");
        assert_eq!(b.made, ["shader", "layout", "buffer", "texture", "view"]);
        let mut s = DrawState {
            layout: Some(Layout(0)),
            ..DrawState::default()
        };
        s.vs.shader = Some(Shader(0));
        s.vs.cbs[0] = Some(CbBinding {
            buffer: Buffer(0),
            first: 0,
            count: 0,
        });
        s.vs.srvs[0] = Some(View(0));
        s.targets[0] = Some(View(1));
        let t = up.state(&s);
        assert_eq!(t.layout, Some(Layout(102)));
        assert_eq!(t.vs.shader, Some(Shader(101)));
        assert_eq!(t.vs.cbs[0].map(|c| c.buffer), Some(Buffer(103)));
        assert_eq!(t.vs.srvs[0], Some(View(105)));
        assert_eq!(t.targets[0], Some(View(7)));
        // A host without the external view: refused.
        let mut b = Counter::default();
        assert!(Uploaded::new(&mut b, &r, |_| None).is_err());
    }
}
