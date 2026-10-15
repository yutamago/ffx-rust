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
    /// Characters' light falloff: 0 the game's soft wrap, 1 physical.
    pub character_falloff: f32,
    /// Scale of the characters' ambient light (they have no baked light).
    pub character_ambient: f32,
    /// Normal and roughness maps made from the colour textures.
    pub material_maps: bool,
    pub normal_strength: f32,
    /// The same for characters (their textures have painted shading).
    pub character_detail: f32,
    pub roughness: f32,
    pub specular: f32,
    pub shadows: bool,
    pub shadow_size: u32,
    /// How far the first (sharpest) shadow cascade reaches, in world
    /// units; the next two reach 3.5 and 12 times as far.
    pub shadow_extent: f32,
    /// Filter radius, in shadow texels.
    pub shadow_softness: f32,
    pub fog: bool,
    /// The enhanced passes' resolution relative to the game's (above 1:
    /// supersampling).
    pub render_scale: f32,
    /// 0 off, 1 FXAA.
    pub antialiasing: u8,
    /// Screen-space ambient occlusion.
    pub ao: bool,
    /// Its radius in world units (a character is about 20 tall).
    pub ao_radius: f32,
    pub ao_strength: f32,
    /// How much of it darkens scenery's baked light (which has coarse
    /// occlusion of its own).
    pub ao_on_baked: f32,
    /// Show one input of the lighting instead of the image: 0 none,
    /// 1 shadow, 2 occlusion, 3 normal, 4 albedo, 5 baked light,
    /// 6 shadow cascades.
    pub debug_view: u8,
    /// Keep the game's blob shadows (redundant with real shadows).
    pub fake_shadows: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            exposure: 1.3,
            tone_curve: 1,
            saturation: 1.0,
            contrast: 1.0,
            map_lights: true,
            key_scale: 0.6,
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
            character_falloff: 0.3,
            character_ambient: 2.0,
            material_maps: true,
            normal_strength: 1.0,
            character_detail: 0.3,
            roughness: 0.7,
            specular: 0.5,
            shadows: true,
            shadow_size: 2048,
            shadow_extent: 50.0,
            shadow_softness: 2.0,
            fog: true,
            render_scale: 1.0,
            antialiasing: 1,
            ao: true,
            ao_radius: 4.0,
            ao_strength: 1.0,
            ao_on_baked: 0.6,
            fake_shadows: false,
            debug_view: 0,
        }
    }
}
