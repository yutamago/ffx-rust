//! Shadows of static lights, made once and kept: a cube of six depth
//! faces of the scenery around each static point or spot light, made
//! again only when the light or the scenery within its reach changes;
//! characters (what moves) cast none from them, as in Unreal.

use super::math;

/// Static lights with shadows of their own at most.
pub const MAX: usize = 8;
/// A face's size, texels.
pub const FACE: u32 = 384;
/// Bytes of the faces' view-projections (rows) for the lighting pass.
pub const ROWS_SIZE: usize = MAX * 6 * 64;
/// Lights made in a frame at most (the rest the frames after).
pub const PER_FRAME: usize = 2;

/// The faces: depth layers, one view each, and their rows for the
/// lighting pass.
pub struct Cubes {
    pub array: wgpu::TextureView,
    pub layers: Vec<wgpu::TextureView>,
    pub rows: wgpu::Buffer,
    /// What each cube was made of (light and scenery), none: not made.
    pub made: [Option<u64>; MAX],
}

impl Cubes {
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("static light shadows"),
            size: wgpu::Extent3d {
                width: FACE,
                height: FACE,
                depth_or_array_layers: (MAX * 6) as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let array = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let layers = (0..(MAX * 6) as u32)
            .map(|l| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: l,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let rows = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("static light faces"),
            size: ROWS_SIZE as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            array,
            layers,
            rows,
            made: [None; MAX],
        }
    }
}

/// The six faces' view-projections (rows) of a light at `p` reaching
/// `range`: +x, -x, +y, -y, +z, -z (as `cube_shadow` in `lighting.wgsl`
/// picks them).
#[must_use]
pub fn faces(p: [f32; 3], range: f32) -> [math::Mat; 6] {
    let (near, far) = (0.5f32, range.max(1.0));
    let proj = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, far / (far - near), -far * near / (far - near)],
        [0.0, 0.0, 1.0, 0.0],
    ];
    let dirs = [
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ];
    dirs.map(|d| math::mul(&proj, &math::look_along(p, d)))
}

/// Whether the box `b` (min, max) reaches the sphere at `c`, `r` across.
#[must_use]
pub fn touches(b: &[f32; 6], c: [f32; 3], r: f32) -> bool {
    let d2: f32 = (0..3)
        .map(|k| {
            let v = c[k].clamp(b[k], b[k + 3]);
            (v - c[k]).powi(2)
        })
        .sum();
    d2 <= r * r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_see_their_side() {
        let f = faces([10.0, 0.0, 0.0], 50.0);
        // A point 5 units along +x is in the middle of face 0, in front.
        let p = [15.0, 0.0, 0.0, 1.0];
        let c: Vec<f32> = f[0]
            .iter()
            .map(|row| row.iter().zip(&p).map(|(a, b)| a * b).sum())
            .collect();
        assert!(c[3] > 0.0);
        assert!((c[0] / c[3]).abs() < 1e-4 && (c[1] / c[3]).abs() < 1e-4);
        let z = c[2] / c[3];
        assert!(z > 0.0 && z < 1.0);
        assert!(touches(
            &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            [3.0, 0.5, 0.5],
            2.5
        ));
        assert!(!touches(
            &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            [3.0, 0.5, 0.5],
            1.5
        ));
    }
}
