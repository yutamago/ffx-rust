//! What the enhanced renderer does, as the overlay and `ffx-render.toml`
//! set it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Draw scenery and characters the enhanced way (off: the game's own
    /// shaders, as in the original).
    pub enabled: bool,
    /// Multiplier before the tone curve.
    pub exposure: f32,
    /// 0: the game's curve, 1: ACES, 2: filmic (AgX-like).
    pub tone_curve: u8,
    pub saturation: f32,
    pub contrast: f32,
    /// Take the key, fill and sky light from the map's character lights
    /// (the game's own per-map lighting); off: the values below.
    pub map_lights: bool,
    /// Scale of the map's key light, fill light and ambient (sky).
    pub key_scale: f32,
    pub fill_scale: f32,
    pub sky_scale: f32,
    /// The direction the key light travels (world space).
    pub sun_direction: [f32; 3],
    pub sun_intensity: f32,
    pub sun_color: [f32; 3],
    pub sky_color: [f32; 3],
    pub sky_intensity: f32,
    pub ground_color: [f32; 3],
    /// Weight of the game's baked vertex light (the indirect term).
    pub baked_light: f32,
    /// How much the key light reshapes scenery's baked light (shadows,
    /// facing), 0–1.
    pub scenery_key: f32,
    /// Specular highlights on scenery from the key light.
    pub scenery_specular: f32,
    /// Scale of the characters' ambient light (they have no baked light).
    pub character_ambient: f32,
    pub roughness: f32,
    pub specular: f32,
    pub shadows: bool,
    pub shadow_size: u32,
    /// Half-width of the shadow map's box, in world units.
    pub shadow_extent: f32,
    /// Filter radius, in shadow texels.
    pub shadow_softness: f32,
    pub fog: bool,
    /// Keep the game's blob shadows (redundant with real shadows).
    pub fake_shadows: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            exposure: 1.15,
            tone_curve: 1,
            saturation: 1.0,
            contrast: 1.0,
            map_lights: true,
            key_scale: 0.35,
            fill_scale: 0.2,
            sky_scale: 0.3,
            sun_direction: [-0.35, -0.85, 0.4],
            sun_intensity: 0.0,
            sun_color: [1.0, 0.95, 0.85],
            sky_color: [0.6, 0.7, 0.9],
            sky_intensity: 0.0,
            ground_color: [0.3, 0.27, 0.25],
            baked_light: 1.0,
            scenery_key: 0.6,
            scenery_specular: 0.5,
            character_ambient: 2.0,
            roughness: 0.7,
            specular: 0.5,
            shadows: true,
            shadow_size: 2048,
            shadow_extent: 60.0,
            shadow_softness: 2.0,
            fog: true,
            fake_shadows: false,
        }
    }
}
