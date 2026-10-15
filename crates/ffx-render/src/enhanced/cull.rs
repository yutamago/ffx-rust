//! Culling for the passes the game did not cull for: each shadow cascade
//! and probe face draws only what its view can hold. A draw's bounds come
//! from the vertices its indices use (vertex and index data stay on the CPU
//! for static buffers), once per buffer contents.

use std::collections::HashMap;

use super::{Item, Source, math};
use crate::Renderer;

/// Min x, y, z, then max x, y, z.
pub type Aabb = [f32; 6];

/// Bounds of draws, by what they read.
#[derive(Default)]
pub struct Bounds {
    cache: HashMap<u64, Option<Aabb>>,
}

impl Bounds {
    /// The draw's bounds in world space, if its vertices can be read
    /// (static buffers; not skinned: bones move them).
    pub fn world(&mut self, r: &Renderer, i: &Item) -> Option<Aabb> {
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        // FFX_RENDER_NO_CULL=1: draw everything everywhere (for comparing).
        if *OFF.get_or_init(|| std::env::var_os("FFX_RENDER_NO_CULL").is_some()) {
            return None;
        }
        if i.key.variant.skinned || i.key.variant.cloth || i.draw.instances > 1 {
            return None;
        }
        let (slot, stride, attr) =
            i.key
                .buffers
                .iter()
                .enumerate()
                .find_map(|(k, (stride, inst, attrs))| {
                    if *inst {
                        return None;
                    }
                    attrs
                        .iter()
                        .find(|a| a.0 == 0 && a.1 == wgpu::VertexFormat::Float32x3)
                        .map(|a| (k, *stride, a.2))
                })?;
        let Some(Source::Buffer(vid, voff)) = i.vertex.get(slot).copied() else {
            return None;
        };
        let vb = r.buffers.get(&vid)?;
        let index = match i.index {
            Some((Source::Buffer(iid, ioff), format)) => {
                let ib = r.buffers.get(&iid)?;
                Some((iid, ioff, format, ib.generation))
            }
            Some(_) => return None,
            None => None,
        };
        let d = &i.draw;
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (vid, voff, stride, attr, vb.generation).hash(&mut h);
            index
                .map(|(a, b, f, g)| (a, b, f == wgpu::IndexFormat::Uint32, g))
                .hash(&mut h);
            (d.start, d.count, d.base).hash(&mut h);
            h.finish()
        };
        let local = *self
            .cache
            .entry(key)
            .or_insert_with(|| local_bounds(r, vb, voff, stride, attr, index, d));
        let world = math_world(i)?;
        local.map(|b| transform(&world, &b))
    }
}

/// The item's World matrix (rows), from its uniform.
fn math_world(i: &Item) -> Option<math::Mat> {
    let u = i.uniform.get(..64)?;
    let mut m = [[0.0f32; 4]; 4];
    for (k, v) in u.as_chunks::<4>().0.iter().enumerate() {
        m[k / 4][k % 4] = f32::from_le_bytes(*v);
    }
    Some(m)
}

fn local_bounds(
    r: &Renderer,
    vb: &crate::Buffer,
    voff: u64,
    stride: u32,
    attr: u64,
    index: Option<(ffx_gfx::Id, u64, wgpu::IndexFormat, u64)>,
    d: &ffx_gfx::Draw,
) -> Option<Aabb> {
    if vb.data.is_empty() || stride == 0 {
        return None;
    }
    let mut b = [f32::MAX, f32::MAX, f32::MAX, f32::MIN, f32::MIN, f32::MIN];
    let mut any = false;
    let mut add = |vertex: i64| {
        if vertex < 0 {
            return;
        }
        let at = voff as usize + vertex as usize * stride as usize + attr as usize;
        let Some(p) = vb.data.get(at..at + 12) else {
            return;
        };
        let f = |k: usize| f32::from_le_bytes(p[k * 4..k * 4 + 4].try_into().expect("4"));
        let v = [f(0), f(1), f(2)];
        if v.iter().all(|x| x.is_finite()) {
            for k in 0..3 {
                b[k] = b[k].min(v[k]);
                b[k + 3] = b[k + 3].max(v[k]);
            }
            any = true;
        }
    };
    match index {
        Some((iid, ioff, format, _)) => {
            let data = &r.buffers.get(&iid)?.data;
            let size = if format == wgpu::IndexFormat::Uint32 {
                4
            } else {
                2
            };
            for n in 0..d.count as usize {
                let at = ioff as usize + (d.start as usize + n) * size;
                let x = data.get(at..at + size)?;
                let value = if size == 4 {
                    i64::from(u32::from_le_bytes(x.try_into().expect("4")))
                } else {
                    i64::from(u16::from_le_bytes(x.try_into().expect("2")))
                };
                add(value + i64::from(d.base));
            }
        }
        None => {
            for n in 0..i64::from(d.count) {
                add(i64::from(d.start) + n);
            }
        }
    }
    any.then_some(b)
}

fn transform(m: &math::Mat, b: &Aabb) -> Aabb {
    let mut out = [f32::MAX, f32::MAX, f32::MAX, f32::MIN, f32::MIN, f32::MIN];
    for c in 0..8 {
        let p = [
            if c & 1 == 0 { b[0] } else { b[3] },
            if c & 2 == 0 { b[1] } else { b[4] },
            if c & 4 == 0 { b[2] } else { b[5] },
        ];
        for (r, row) in m.iter().take(3).enumerate() {
            let v = row[0] * p[0] + row[1] * p[1] + row[2] * p[2] + row[3];
            out[r] = out[r].min(v);
            out[r + 3] = out[r + 3].max(v);
        }
    }
    out
}

/// Whether the box is wholly outside the view `vp` (rows, clip = vp · p,
/// depth 0–1). `depth`: test near and far too (shadow casters before the
/// near plane still cast, so cascades test only the sides).
#[must_use]
pub fn outside(vp: &math::Mat, b: &Aabb, depth: bool) -> bool {
    let mut out = [true; 6];
    for c in 0..8 {
        let p = [
            if c & 1 == 0 { b[0] } else { b[3] },
            if c & 2 == 0 { b[1] } else { b[4] },
            if c & 4 == 0 { b[2] } else { b[5] },
            1.0,
        ];
        let clip: Vec<f32> = vp
            .iter()
            .map(|row| row.iter().zip(&p).map(|(a, b)| a * b).sum())
            .collect();
        let (x, y, z, w) = (clip[0], clip[1], clip[2], clip[3]);
        out[0] &= x < -w;
        out[1] &= x > w;
        out[2] &= y < -w;
        out[3] &= y > w;
        out[4] &= z < 0.0;
        out[5] &= z > w;
    }
    out[..4].iter().any(|&o| o) || (depth && (out[4] || out[5]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_outside_a_view() {
        // An orthographic view of -1..1 in x and y, depth z 0..1.
        let vp = math::identity();
        assert!(!outside(&vp, &[-0.5, -0.5, 0.2, 0.5, 0.5, 0.4], true));
        assert!(outside(&vp, &[2.0, -0.5, 0.2, 3.0, 0.5, 0.4], true));
        // Before the near plane: kept when depth is not tested.
        assert!(outside(&vp, &[-0.5, -0.5, -3.0, 0.5, 0.5, -2.0], true));
        assert!(!outside(&vp, &[-0.5, -0.5, -3.0, 0.5, 0.5, -2.0], false));
        let moved = transform(
            &[
                [1.0, 0.0, 0.0, 10.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        );
        assert_eq!(moved, [10.0, 0.0, 0.0, 11.0, 1.0, 1.0]);
    }
}
