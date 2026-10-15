//! Materials per texture: what a surface is made of (roughness, metal,
//! highlights, surface detail, its own light, a tint), from the editor by
//! the texture's hash, else automatic: the generated material maps, and on
//! characters skin found by its colour, which takes the skin settings.

use serde::{Deserialize, Serialize};

use super::Settings;

/// What a surface is, for its starting values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// The renderer's guess: the generated maps, skin by colour on
    /// characters.
    #[default]
    Auto,
    Skin,
    Hair,
    Cloth,
    Leather,
    Metal,
    Stone,
    Wood,
    /// Smooth and shiny (lacquer, plastic, polished stone).
    Glossy,
    /// It glows.
    Glowing,
}

impl Kind {
    pub const ALL: [Self; 10] = [
        Self::Auto,
        Self::Skin,
        Self::Hair,
        Self::Cloth,
        Self::Leather,
        Self::Metal,
        Self::Stone,
        Self::Wood,
        Self::Glossy,
        Self::Glowing,
    ];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "automatic",
            Self::Skin => "skin",
            Self::Hair => "hair",
            Self::Cloth => "cloth",
            Self::Leather => "leather",
            Self::Metal => "metal",
            Self::Stone => "stone",
            Self::Wood => "wood",
            Self::Glossy => "glossy",
            Self::Glowing => "glowing",
        }
    }

    /// Roughness (none: the generated map's), metallic, specular, surface
    /// detail, emission.
    fn preset(self) -> (Option<f32>, f32, f32, f32, f32) {
        match self {
            Self::Auto => (None, 0.0, 1.0, 1.0, 0.0),
            Self::Skin => (Some(0.5), 0.0, 0.7, 0.3, 0.0),
            Self::Hair => (Some(0.35), 0.0, 1.0, 0.6, 0.0),
            Self::Cloth => (Some(0.85), 0.0, 0.4, 1.0, 0.0),
            Self::Leather => (Some(0.55), 0.0, 0.8, 1.0, 0.0),
            Self::Metal => (Some(0.3), 1.0, 1.0, 0.6, 0.0),
            Self::Stone => (Some(0.8), 0.0, 0.6, 1.2, 0.0),
            Self::Wood => (Some(0.7), 0.0, 0.6, 1.0, 0.0),
            Self::Glossy => (Some(0.15), 0.0, 1.2, 0.5, 0.0),
            Self::Glowing => (Some(0.6), 0.0, 0.5, 0.5, 1.0),
        }
    }
}

/// A texture's material as the editor sets it; what is not set comes from
/// the kind.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Material {
    pub kind: Kind,
    pub roughness: Option<f32>,
    pub metallic: Option<f32>,
    /// Highlights, 1 as the renderer's default.
    pub specular: Option<f32>,
    /// Surface detail (generated normals), 1 as the renderer's default.
    pub detail: Option<f32>,
    /// Its own light, times its colour.
    pub emission: Option<f32>,
    /// Multiplies the colour (linear).
    pub tint: Option<[f32; 3]>,
    /// Find skin by colour on this texture (default: on characters).
    pub skin: Option<bool>,
    /// An image (PNG) to use instead of the texture, from the `textures`
    /// folder next to the renderer.
    pub texture: Option<String>,
}

/// What the G-buffer pass gets for a draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolved {
    /// Below 0: from the material map.
    pub roughness: f32,
    pub metallic: f32,
    pub specular: f32,
    pub detail: f32,
    pub emission: f32,
    pub tint: [f32; 3],
    /// How much skin-coloured pixels take the skin material.
    pub skin: f32,
    pub skin_roughness: f32,
    pub skin_detail: f32,
}

/// The material of a draw: its texture's (if the editor set one), on a
/// character or not.
#[must_use]
pub fn resolve(m: Option<&Material>, character: bool, s: &Settings) -> Resolved {
    let kind = m.map_or(Kind::Auto, |m| m.kind);
    let (rough, metal, spec, detail, emission) = kind.preset();
    let pick = |v: Option<f32>, d: f32| v.unwrap_or(d);
    let skin_on = m
        .and_then(|m| m.skin)
        .unwrap_or(character && kind == Kind::Auto);
    Resolved {
        roughness: m.and_then(|m| m.roughness).or(rough).unwrap_or(-1.0),
        metallic: pick(m.and_then(|m| m.metallic), metal),
        specular: pick(m.and_then(|m| m.specular), spec),
        detail: pick(m.and_then(|m| m.detail), detail),
        emission: pick(m.and_then(|m| m.emission), emission),
        tint: m.and_then(|m| m.tint).unwrap_or([1.0; 3]),
        skin: if skin_on { s.skin_materials } else { 0.0 },
        skin_roughness: s.skin_roughness,
        skin_detail: s.skin_detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_win_over_the_kind() {
        let s = Settings::default();
        let auto = resolve(None, true, &s);
        assert!(auto.roughness < 0.0);
        assert!(auto.skin > 0.0);
        let metal = Material {
            kind: Kind::Metal,
            roughness: Some(0.6),
            ..Material::default()
        };
        let r = resolve(Some(&metal), true, &s);
        assert!((r.roughness - 0.6).abs() < 1e-6);
        assert!((r.metallic - 1.0).abs() < 1e-6);
        assert!(r.skin.abs() < 1e-6);
    }
}
