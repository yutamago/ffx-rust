//! The pipeline-state layer: one backend object per distinct shader
//! (by its DXBC), input layout, state description and constant buffer
//! size, made on first use.

use std::collections::HashMap;

use crate::backend::{Backend, Blend, Buffer, DepthState, Layout, Raster, Result, Sampler, Shader};
use crate::desc::{
    BIND_CONSTANT, BlendDesc, BufferDesc, CPU_WRITE, DepthStencilDesc, InputElement, RasterDesc,
    SamplerDesc, Stage, USAGE_DYNAMIC,
};

/// FNV-1a 64, the hash the graphics layer keys shaders by.
#[must_use]
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Interned backend objects.
#[derive(Default)]
pub struct Cache {
    shaders: HashMap<(Stage, Vec<u8>), Shader>,
    layouts: HashMap<(Vec<InputElement>, u64), Layout>,
    blends: HashMap<BlendDesc, Blend>,
    depths: HashMap<DepthStencilDesc, DepthState>,
    rasters: HashMap<RasterDesc, Raster>,
    samplers: HashMap<SamplerDesc, Sampler>,
    /// Dynamic constant buffers by (stage, slot, size): one each, written
    /// before every draw that binds it (a discard per draw, as the game's
    /// own constant buffers are).
    constants: HashMap<(Stage, u8, u32), Buffer>,
}

impl Cache {
    /// The shader for this DXBC.
    ///
    /// # Errors
    /// When the backend refuses it.
    pub fn shader<B: Backend>(&mut self, b: &mut B, stage: Stage, dxbc: &[u8]) -> Result<Shader> {
        if let Some(&s) = self.shaders.get(&(stage, dxbc.to_vec())) {
            return Ok(s);
        }
        let s = b.create_shader(stage, dxbc)?;
        self.shaders.insert((stage, dxbc.to_vec()), s);
        Ok(s)
    }

    /// The input layout for `elements` with the vertex shader `vs_dxbc`.
    ///
    /// # Errors
    /// When the backend refuses it.
    pub fn layout<B: Backend>(
        &mut self,
        b: &mut B,
        elements: &[InputElement],
        vs_dxbc: &[u8],
    ) -> Result<Layout> {
        let key = (elements.to_vec(), fnv1a(vs_dxbc));
        if let Some(&l) = self.layouts.get(&key) {
            return Ok(l);
        }
        let l = b.create_layout(elements, vs_dxbc)?;
        self.layouts.insert(key, l);
        Ok(l)
    }

    /// # Errors
    /// When the backend refuses it.
    pub fn blend<B: Backend>(&mut self, b: &mut B, d: &BlendDesc) -> Result<Blend> {
        if let Some(&s) = self.blends.get(d) {
            return Ok(s);
        }
        let s = b.create_blend(d)?;
        self.blends.insert(*d, s);
        Ok(s)
    }

    /// # Errors
    /// When the backend refuses it.
    pub fn depth<B: Backend>(&mut self, b: &mut B, d: &DepthStencilDesc) -> Result<DepthState> {
        if let Some(&s) = self.depths.get(d) {
            return Ok(s);
        }
        let s = b.create_depth_state(d)?;
        self.depths.insert(*d, s);
        Ok(s)
    }

    /// # Errors
    /// When the backend refuses it.
    pub fn raster<B: Backend>(&mut self, b: &mut B, d: &RasterDesc) -> Result<Raster> {
        if let Some(&s) = self.rasters.get(d) {
            return Ok(s);
        }
        let s = b.create_raster(d)?;
        self.rasters.insert(*d, s);
        Ok(s)
    }

    /// # Errors
    /// When the backend refuses it.
    pub fn sampler<B: Backend>(&mut self, b: &mut B, d: &SamplerDesc) -> Result<Sampler> {
        if let Some(&s) = self.samplers.get(d) {
            return Ok(s);
        }
        let s = b.create_sampler(d)?;
        self.samplers.insert(*d, s);
        Ok(s)
    }

    /// A dynamic constant buffer of `size` bytes (a multiple of 16) for
    /// `slot` of `stage`, filled with `data` (shorter data is padded with
    /// zeros, which is what the API reads past a buffer's end).
    ///
    /// # Errors
    /// When the backend refuses it.
    pub fn constants<B: Backend>(
        &mut self,
        b: &mut B,
        stage: Stage,
        slot: u8,
        data: &[u8],
    ) -> Result<Buffer> {
        let size = u32::try_from(data.len().next_multiple_of(16).max(16))
            .map_err(|_| crate::backend::Error("constant buffer too large".into()))?;
        let key = (stage, slot, size);
        let buf = match self.constants.get(&key) {
            Some(&buf) => buf,
            None => {
                let buf = b.create_buffer(
                    &BufferDesc {
                        size,
                        usage: USAGE_DYNAMIC,
                        bind: BIND_CONSTANT,
                        cpu_access: CPU_WRITE,
                        misc: 0,
                        stride: 0,
                    },
                    None,
                )?;
                self.constants.insert(key, buf);
                buf
            }
        };
        if data.len() == size as usize {
            b.write_buffer(buf, 0, data);
        } else {
            let mut padded = data.to_vec();
            padded.resize(size as usize, 0);
            b.write_buffer(buf, 0, &padded);
        }
        Ok(buf)
    }

    /// Objects made so far: shaders, layouts, states, constant buffers.
    #[must_use]
    pub fn counts(&self) -> [usize; 4] {
        [
            self.shaders.len(),
            self.layouts.len(),
            self.blends.len() + self.depths.len() + self.rasters.len() + self.samplers.len(),
            self.constants.len(),
        ]
    }
}
