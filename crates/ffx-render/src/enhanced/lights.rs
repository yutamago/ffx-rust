//! Local lights: point and spot lights in world space, from the editor
//! (per scene and for all scenes) and found automatically where the
//! game's effects glow. The lighting pass loops over them (`lights` in
//! `lighting.wgsl`).

use serde::{Deserialize, Serialize};

/// At most this many lights reach the lighting pass in a frame.
pub const MAX: usize = 64;
/// The GPU list: a count, then per light four vectors.
pub const SIZE: usize = 16 + MAX * 64;
/// How far a light's brightness is given for, in world units (a character
/// is about 20 tall): `intensity` is its brightness there.
const REFERENCE: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Point,
    Spot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Light {
    pub name: String,
    pub enabled: bool,
    pub kind: Kind,
    pub position: [f32; 3],
    /// Linear RGB, 0–1.
    pub color: [f32; 3],
    /// Brightness 10 units away (it falls off with the square of the
    /// distance).
    pub intensity: f32,
    /// Where the light ends (smoothly), in world units.
    pub range: f32,
    /// Spot lights: where they shine, and the cone (full angles, degrees):
    /// full brightness inside `inner`, none outside `outer`.
    pub direction: [f32; 3],
    pub inner: f32,
    pub outer: f32,
    /// Ray-traced shadows (from the scenery, where the GPU traces rays).
    pub shadows: bool,
    /// The share of highlights it makes (0: diffuse only).
    pub specular: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            name: "Light".into(),
            enabled: true,
            kind: Kind::Point,
            position: [0.0; 3],
            color: [1.0, 0.85, 0.6],
            intensity: 1.0,
            range: 80.0,
            direction: [0.0, 1.0, 0.0],
            inner: 30.0,
            outer: 45.0,
            shadows: true,
            specular: 1.0,
        }
    }
}

/// The lights as the lighting pass reads them (`Lights` in
/// `lighting.wgsl`); at most [`MAX`]. Shadows only with `rays`.
#[must_use]
pub fn pack<'a>(lights: impl Iterator<Item = &'a Light>, rays: bool) -> Vec<u8> {
    let mut items = Vec::with_capacity(MAX * 64);
    let mut n = 0u32;
    for l in lights.filter(|l| l.enabled && l.intensity > 0.0 && l.range > 0.0) {
        if n as usize == MAX {
            break;
        }
        n += 1;
        let scale = l.intensity * REFERENCE * REFERENCE;
        let spot = l.kind == Kind::Spot;
        let d = super::math::normalize(l.direction);
        let outer = (l.outer.clamp(1.0, 179.0) * 0.5).to_radians().cos();
        let inner = (l.inner.clamp(0.0, l.outer.clamp(1.0, 179.0)) * 0.5)
            .to_radians()
            .cos()
            .max(outer + 1e-4);
        for v in [
            [l.position[0], l.position[1], l.position[2], l.range],
            [
                l.color[0] * scale,
                l.color[1] * scale,
                l.color[2] * scale,
                if spot { 1.0 } else { 0.0 },
            ],
            [d[0], d[1], d[2], outer],
            [
                inner,
                if l.shadows && rays { 1.0 } else { 0.0 },
                l.specular,
                0.0,
            ],
        ] {
            for x in v {
                items.extend_from_slice(&x.to_le_bytes());
            }
        }
    }
    let mut out = Vec::with_capacity(SIZE);
    #[allow(clippy::cast_precision_loss)] // at most 64
    for x in [n as f32, 0.0, 0.0, 0.0] {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out.extend_from_slice(&items);
    out.resize(SIZE, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_enabled_lights() {
        let on = Light::default();
        let off = Light {
            enabled: false,
            ..Light::default()
        };
        let bytes = pack([&on, &off, &on].into_iter(), true);
        assert_eq!(bytes.len(), SIZE);
        assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), 2.0);
        // Range in w of the first vector.
        let range = f32::from_le_bytes(bytes[16 + 12..16 + 16].try_into().unwrap());
        assert!((range - 80.0).abs() < 1e-6);
    }
}
