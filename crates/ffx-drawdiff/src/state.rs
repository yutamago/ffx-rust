//! D3D11 state objects decoded field by field from the descriptions the
//! graphics layer recorded (`ffx-hook` `gfx::create_state`): only the
//! fields Direct3D reads, padding left out. A null state (id 0) is
//! Direct3D's default, so an explicit state equal to the default compares
//! equal to none.

use crate::Value;

fn u32_at(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4)
        .map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn u8_at(b: &[u8], at: usize) -> u32 {
    b.get(at).map_or(0, |&v| u32::from(v))
}

/// `D3D11_BLEND_DESC` (8 + 8 × 32 bytes). Render targets 1–7 are read
/// only with `IndependentBlendEnable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blend {
    pub alpha_to_coverage: u32,
    pub independent: u32,
    /// Enable, src, dest, op, src alpha, dest alpha, op alpha, write mask.
    pub rt: [[u32; 8]; 8],
}

const RT_FIELDS: [&str; 8] = [
    "enable",
    "src",
    "dest",
    "op",
    "src_alpha",
    "dest_alpha",
    "op_alpha",
    "write_mask",
];

impl Blend {
    pub const DEFAULT: Self = Self {
        alpha_to_coverage: 0,
        independent: 0,
        rt: [[0, 2, 1, 1, 2, 1, 1, 0xf]; 8],
    };

    #[must_use]
    pub fn decode(desc: Option<&[u8]>) -> Self {
        let Some(d) = desc else {
            return Self::DEFAULT;
        };
        let mut rt = [[0; 8]; 8];
        for (i, t) in rt.iter_mut().enumerate() {
            let at = 8 + 32 * i;
            for (k, v) in t.iter_mut().enumerate().take(7) {
                *v = u32_at(d, at + 4 * k);
            }
            // RenderTargetWriteMask is a UINT8; three bytes of padding follow.
            t[7] = u8_at(d, at + 28);
        }
        Self {
            alpha_to_coverage: u32_at(d, 0),
            independent: u32_at(d, 4),
            rt,
        }
    }

    /// Whether an enabled target blends with the blend factor
    /// (`D3D11_BLEND_BLEND_FACTOR`, `D3D11_BLEND_INV_BLEND_FACTOR`).
    #[must_use]
    pub fn uses_factor(&self) -> bool {
        let n = if self.independent != 0 { 8 } else { 1 };
        self.rt[..n]
            .iter()
            .any(|t| t[0] != 0 && t[1..=5].iter().any(|&f| f == 14 || f == 15))
    }

    pub fn fields(&self, out: &mut Vec<(String, Value)>) {
        out.push((
            "blend.alpha_to_coverage".into(),
            Value::U(self.alpha_to_coverage),
        ));
        out.push(("blend.independent".into(), Value::U(self.independent)));
        let n = if self.independent != 0 { 8 } else { 1 };
        for (i, t) in self.rt[..n].iter().enumerate() {
            for (name, &v) in RT_FIELDS.iter().zip(t) {
                out.push((format!("blend.rt{i}.{name}"), Value::U(v)));
            }
        }
    }
}

/// `D3D11_DEPTH_STENCIL_DESC` (52 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Depth {
    pub enable: u32,
    pub write_mask: u32,
    pub func: u32,
    pub stencil_enable: u32,
    pub stencil_read_mask: u32,
    pub stencil_write_mask: u32,
    /// Fail, depth fail, pass, func: front, then back.
    pub faces: [[u32; 4]; 2],
}

impl Depth {
    pub const DEFAULT: Self = Self {
        enable: 1,
        write_mask: 1,
        func: 2,
        stencil_enable: 0,
        stencil_read_mask: 0xff,
        stencil_write_mask: 0xff,
        faces: [[1, 1, 1, 8]; 2],
    };

    #[must_use]
    pub fn decode(desc: Option<&[u8]>) -> Self {
        let Some(d) = desc else {
            return Self::DEFAULT;
        };
        let mut faces = [[0; 4]; 2];
        for (f, face) in faces.iter_mut().enumerate() {
            for (k, v) in face.iter_mut().enumerate() {
                *v = u32_at(d, 20 + 16 * f + 4 * k);
            }
        }
        Self {
            enable: u32_at(d, 0),
            write_mask: u32_at(d, 4),
            func: u32_at(d, 8),
            stencil_enable: u32_at(d, 12),
            stencil_read_mask: u8_at(d, 16),
            stencil_write_mask: u8_at(d, 17),
            faces,
        }
    }

    pub fn fields(&self, out: &mut Vec<(String, Value)>) {
        out.push(("depth.enable".into(), Value::U(self.enable)));
        out.push(("depth.write_mask".into(), Value::U(self.write_mask)));
        out.push(("depth.func".into(), Value::U(self.func)));
        out.push(("depth.stencil".into(), Value::U(self.stencil_enable)));
        if self.stencil_enable != 0 {
            out.push((
                "depth.stencil_read_mask".into(),
                Value::U(self.stencil_read_mask),
            ));
            out.push((
                "depth.stencil_write_mask".into(),
                Value::U(self.stencil_write_mask),
            ));
            for (face, f) in ["front", "back"].iter().zip(&self.faces) {
                for (name, &v) in ["fail", "depth_fail", "pass", "func"].iter().zip(f) {
                    out.push((format!("depth.{face}.{name}"), Value::U(v)));
                }
            }
        }
    }
}

/// `D3D11_RASTERIZER_DESC` (40 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raster {
    /// Fill, cull, front counter-clockwise, depth bias, depth bias clamp
    /// (f32 bits), slope-scaled depth bias (f32 bits), depth clip,
    /// scissor, multisample, antialiased lines.
    pub v: [u32; 10],
}

const RASTER_FIELDS: [&str; 10] = [
    "fill",
    "cull",
    "front_ccw",
    "depth_bias",
    "depth_bias_clamp",
    "slope_scaled_depth_bias",
    "depth_clip",
    "scissor",
    "multisample",
    "aa_lines",
];

impl Raster {
    pub const DEFAULT: Self = Self {
        v: [3, 3, 0, 0, 0, 0, 1, 0, 0, 0],
    };

    #[must_use]
    pub fn decode(desc: Option<&[u8]>) -> Self {
        let Some(d) = desc else {
            return Self::DEFAULT;
        };
        let mut v = [0; 10];
        for (k, x) in v.iter_mut().enumerate() {
            *x = u32_at(d, 4 * k);
        }
        Self { v }
    }

    #[must_use]
    pub fn scissor(&self) -> bool {
        self.v[7] != 0
    }

    pub fn fields(&self, out: &mut Vec<(String, Value)>) {
        for (k, (name, &v)) in RASTER_FIELDS.iter().zip(&self.v).enumerate() {
            let value = match k {
                3 => Value::I(v.cast_signed()),
                4 | 5 => Value::F(v),
                _ => Value::U(v),
            };
            out.push((format!("raster.{name}"), value));
        }
    }
}

/// `D3D11_SAMPLER_DESC` (52 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sampler {
    /// Filter, address u, v, w, mip LOD bias (f32 bits), max anisotropy,
    /// comparison, border colour (4 × f32 bits), min LOD, max LOD (f32
    /// bits).
    pub v: [u32; 13],
}

const SAMPLER_FIELDS: [&str; 13] = [
    "filter",
    "address_u",
    "address_v",
    "address_w",
    "mip_lod_bias",
    "max_anisotropy",
    "comparison",
    "border.r",
    "border.g",
    "border.b",
    "border.a",
    "min_lod",
    "max_lod",
];

impl Sampler {
    pub const DEFAULT: Self = Self {
        v: [
            0x15,
            3,
            3,
            3,
            0,
            1,
            1,
            0x3f80_0000,
            0x3f80_0000,
            0x3f80_0000,
            0x3f80_0000,
            0xff7f_ffff,
            0x7f7f_ffff,
        ],
    };

    #[must_use]
    pub fn decode(desc: Option<&[u8]>) -> Self {
        let Some(d) = desc else {
            return Self::DEFAULT;
        };
        let mut v = [0; 13];
        for (k, x) in v.iter_mut().enumerate() {
            *x = u32_at(d, 4 * k);
        }
        Self { v }
    }

    pub fn fields(&self, prefix: &str, out: &mut Vec<(String, Value)>) {
        for (k, (name, &v)) in SAMPLER_FIELDS.iter().zip(&self.v).enumerate() {
            let value = if matches!(k, 4 | 7..=12) {
                Value::F(v)
            } else {
                Value::U(v)
            };
            out.push((format!("{prefix}.{name}"), value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(v: &[u32]) -> Vec<u8> {
        v.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    #[test]
    fn default_desc_equals_null_state() {
        let mut blend = words(&[0, 0]);
        for _ in 0..8 {
            blend.extend(words(&[0, 2, 1, 1, 2, 1, 1]));
            // the write mask, then junk in the padding
            blend.extend([0xf, 0xcc, 0xcc, 0xcc]);
        }
        assert_eq!(Blend::decode(Some(&blend)), Blend::DEFAULT);
        let mut depth = words(&[1, 1, 2, 0]);
        depth.extend([0xff, 0xff, 0x12, 0x34]);
        depth.extend(words(&[1, 1, 1, 8, 1, 1, 1, 8]));
        assert_eq!(Depth::decode(Some(&depth)), Depth::DEFAULT);
        let raster = words(&[3, 3, 0, 0, 0, 0, 1, 0, 0, 0]);
        assert_eq!(Raster::decode(Some(&raster)), Raster::DEFAULT);
        let sampler = words(&Sampler::DEFAULT.v);
        assert_eq!(Sampler::decode(Some(&sampler)), Sampler::DEFAULT);
    }

    #[test]
    fn other_targets_only_with_independent_blend() {
        let mut b = Blend::DEFAULT;
        b.rt[3][1] = 5;
        let mut f = Vec::new();
        b.fields(&mut f);
        assert!(!f.iter().any(|(n, _)| n.starts_with("blend.rt3")));
        b.independent = 1;
        f.clear();
        b.fields(&mut f);
        assert!(f.contains(&("blend.rt3.src".into(), Value::U(5))));
    }
}
