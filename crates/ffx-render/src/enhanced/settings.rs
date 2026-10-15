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
    /// Light probes: characters take their indirect light from the
    /// scenery around them (its baked light, seen from where they stand).
    pub probes: bool,
    pub probe_strength: f32,
    /// How much the probe light replaces the characters' ambient.
    pub probe_mix: f32,
    /// Skin scattering: light bleeding red past the terminator and into
    /// shadow edges on what looks like skin.
    pub skin: f32,
    /// Light from the game's effects (fire, magic) on what is around them.
    pub effect_light: bool,
    pub effect_light_strength: f32,
    /// The enhanced water (reflections, refraction, light); off: the
    /// game's.
    pub water: bool,
    pub water_reflection: f32,
    /// How much the surface bends what lies under it.
    pub water_refraction: f32,
    /// Absorption per world unit of depth.
    pub water_absorption: f32,
    /// Brightness of the game's environment map in reflections.
    pub water_environment: f32,
    /// The colour of deep water (multiplied by the game's water colour).
    pub water_deep: [f32; 3],
    /// Volumetric light: shafts of the key light through the air.
    pub volumetrics: bool,
    /// Extinction per world unit (a character is about 20 tall).
    pub fog_density: f32,
    /// Forward scattering, 0–0.95 (higher: shafts glow toward the light).
    pub fog_anisotropy: f32,
    /// Brightness of the light shafts relative to the key light on
    /// surfaces.
    pub volumetric_strength: f32,
    /// Sky light the air scatters evenly (haze).
    pub haze: f32,
    pub volumetric_steps: u32,
    /// How far the march goes, in world units.
    pub volumetric_distance: f32,
    /// The enhanced passes' resolution relative to the game's (above 1:
    /// supersampling).
    pub render_scale: f32,
    /// 0 off, 1 FXAA, 2 temporal (TAA).
    pub antialiasing: u8,
    /// TAA: the weight of each new frame (lower: smoother, more ghosting).
    pub taa_blend: f32,
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
    /// 6 shadow cascades, 7 skin, 8 probe light.
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
            probes: true,
            probe_strength: 1.0,
            probe_mix: 0.7,
            skin: 0.5,
            effect_light: true,
            effect_light_strength: 4.0,
            water: true,
            water_reflection: 1.0,
            water_refraction: 0.04,
            water_absorption: 0.03,
            water_environment: 1.0,
            water_deep: [0.02, 0.05, 0.07],
            volumetrics: true,
            fog_density: 0.0015,
            fog_anisotropy: 0.6,
            volumetric_strength: 0.05,
            haze: 0.3,
            volumetric_steps: 24,
            volumetric_distance: 300.0,
            render_scale: 1.0,
            antialiasing: 2,
            taa_blend: 0.1,
            ao: true,
            ao_radius: 4.0,
            ao_strength: 1.0,
            ao_on_baked: 0.6,
            fake_shadows: false,
            debug_view: 0,
        }
    }
}
