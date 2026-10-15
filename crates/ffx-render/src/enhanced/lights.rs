//! The lights the lighting pass loops over (`lights` in `lighting.wgsl`),
//! in its units: point and spot lights in world space and unshadowed
//! directional lights, from the scene's rig (`rig.rs`) and found
//! automatically where the game's effects glow.

use serde::{Deserialize, Serialize};

/// At most this many lights reach the lighting pass in a frame.
pub const MAX: usize = 64;
/// The GPU list: a count, then per light five vectors.
pub const SIZE: usize = 16 + MAX * 80;
/// How far a light's brightness is given for, in world units (a character
/// is about 20 tall): `intensity` is its brightness there.
pub const REFERENCE: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Point,
    Spot,
    /// Parallel light (`direction`; position and range unused).
    Directional,
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
    /// How strongly it lights scenery and characters.
    pub scenery: f32,
    pub characters: f32,
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
            scenery: 1.0,
            characters: 1.0,
        }
    }
}

/// The key light (the sun) as the editor changes it: what is set replaces
/// the map's (or the settings').
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sun {
    /// The direction the light travels (world space; y up).
    pub direction: Option<[f32; 3]>,
    pub color: Option<[f32; 3]>,
    /// Times the brightness it has.
    pub scale: Option<f32>,
}

impl Sun {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// A light found in the glows, as the editor changes it: matched by where
/// it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GlowEdit {
    pub position: [f32; 3],
    pub enabled: bool,
    /// Times the brightness it is found with.
    pub brightness: f32,
    pub color: Option<[f32; 3]>,
    pub shadows: Option<bool>,
}

impl Default for GlowEdit {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            enabled: true,
            brightness: 1.0,
            color: None,
            shadows: None,
        }
    }
}

/// How near a glow light must be to an edit's position to take it.
pub const GLOW_MATCH: f32 = 15.0;

impl GlowEdit {
    /// `l` (a glow light) as this edit changes it.
    #[must_use]
    pub fn apply(&self, l: &Light) -> Light {
        let mut out = l.clone();
        out.enabled = self.enabled;
        out.intensity *= self.brightness;
        if let Some(c) = self.color {
            out.color = c;
        }
        if let Some(s) = self.shadows {
            out.shadows = s;
        }
        out
    }
}

/// The edit for a glow light at `p`, if one is near.
#[must_use]
pub fn glow_edit(edits: &[GlowEdit], p: [f32; 3]) -> Option<&GlowEdit> {
    edits
        .iter()
        .map(|e| {
            let d = (0..3).map(|i| (e.position[i] - p[i]).powi(2)).sum::<f32>();
            (e, d)
        })
        .filter(|(_, d)| *d < GLOW_MATCH * GLOW_MATCH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(e, _)| e)
}

/// The lights as the lighting pass reads them (`Lights` in
/// `lighting.wgsl`); at most [`MAX`]. Shadows only with `rays`.
#[must_use]
pub fn pack<'a>(lights: impl Iterator<Item = &'a Light>, rays: bool) -> Vec<u8> {
    let mut items = Vec::with_capacity(MAX * 80);
    let mut n = 0u32;
    for l in lights.filter(|l| l.enabled && l.intensity > 0.0 && l.range > 0.0) {
        if n as usize == MAX {
            break;
        }
        n += 1;
        let directional = l.kind == Kind::Directional;
        // Directional light is its colour; the others are given at
        // REFERENCE units away.
        let scale = if directional {
            l.intensity
        } else {
            l.intensity * REFERENCE * REFERENCE
        };
        let kind = match l.kind {
            Kind::Point => 0.0,
            Kind::Spot => 1.0,
            Kind::Directional => 2.0,
        };
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
                kind,
            ],
            [d[0], d[1], d[2], outer],
            [
                inner,
                if l.shadows && rays && !directional {
                    1.0
                } else {
                    0.0
                },
                l.specular,
                0.0,
            ],
            [l.scenery, l.characters, 0.0, 0.0],
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
