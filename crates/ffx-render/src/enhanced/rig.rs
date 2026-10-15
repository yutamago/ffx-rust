//! A scene's lights as an editor shows them, in physical units like
//! Unreal's: an exposure (EV100) and a list of lights, each with its own
//! properties. The game's own lighting is in the list too: its painted
//! light (`Original`), and the map's key, fill and ambient light, which
//! follow the game unless told otherwise.
//!
//! Units: directional lights in lux, point and spot lights in candela,
//! sky light in nits (cd/m²); world distances turn into metres at
//! [`UNITS_PER_METRE`]. The lighting pass works in units of
//! [`NITS_PER_UNIT`]; the exposure brings them to the screen.

use serde::{Deserialize, Serialize};

use super::lights::{Kind as LocalKind, Light as Local};
use super::math;

/// World units in a metre (a character, about 1.75 m, is about 20 units).
pub const UNITS_PER_METRE: f32 = 20.0 / 1.75;
/// The exposure at which the game's own light shows as it does in the
/// game (EV100 12 with the renderer's tone curve, a little brighter).
pub const EV_REFERENCE: f32 = 12.378_5;
/// Luminance (nits) of one unit of the lighting pass: what exposure
/// [`EV_REFERENCE`] brings to 1 (`1.2 · 2^EV`, the saturation-based
/// exposure).
pub const NITS_PER_UNIT: f32 = 6390.0;
/// Illuminance (lux) of a light of brightness 1 in the lighting pass on a
/// surface it faces (`π · NITS_PER_UNIT`).
pub const LUX_PER_UNIT: f32 = std::f32::consts::PI * NITS_PER_UNIT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LightType {
    /// The game's painted light (scenery's baked light, characters'
    /// ambient): intensity is a multiple of it.
    Original,
    /// Light from the whole sky (a hemisphere, upper and lower colour).
    Sky,
    /// Parallel light from far away (the sun, the moon).
    Directional,
    #[default]
    Point,
    Spot,
}

impl LightType {
    pub const ALL: [Self; 5] = [
        Self::Original,
        Self::Sky,
        Self::Directional,
        Self::Point,
        Self::Spot,
    ];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "Original light",
            Self::Sky => "Sky light",
            Self::Directional => "Directional light",
            Self::Point => "Point light",
            Self::Spot => "Spot light",
        }
    }

    /// The unit of its intensity.
    #[must_use]
    pub fn unit(self) -> &'static str {
        match self {
            Self::Original => "×",
            Self::Sky => "nits",
            Self::Directional => "lux",
            Self::Point | Self::Spot => "cd",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mobility {
    /// Never moves or changes in play: its shadows are made once and kept.
    Static,
    /// Worked out every frame.
    #[default]
    Dynamic,
}

/// Which of the game's lights a light follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameLight {
    #[default]
    None,
    /// The map's key light for characters (`Light0`).
    Key,
    /// Its fill light (`Light1`).
    Fill,
    /// Its ambient light (`CharacterAmbientLight`).
    Ambient,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)] // independent properties, as the editor shows them
pub struct SceneLight {
    pub name: String,
    pub kind: LightType,
    pub enabled: bool,
    pub mobility: Mobility,
    /// The game's light it follows: its direction and colour (with its
    /// brightness) come from the game where `follow_direction` and
    /// `follow_color` say so.
    pub game: GameLight,
    pub follow_direction: bool,
    pub follow_color: bool,
    pub position: [f32; 3],
    /// Where it shines (directional and spot lights; y up).
    pub direction: [f32; 3],
    /// Linear RGB, 0–1 (the sky's upper hemisphere).
    pub color: [f32; 3],
    /// The sky's lower hemisphere (light from the ground).
    pub lower_color: [f32; 3],
    pub use_temperature: bool,
    /// Kelvin.
    pub temperature: f32,
    /// Lux, candela, nits or a multiple, by kind ([`LightType::unit`]);
    /// for a light that follows the game's colour: at the game's
    /// brightness 1.
    pub intensity: f32,
    /// Directional: the source's angle across (degrees, soft shadow
    /// edges); point and spot: its radius (world units).
    pub source: f32,
    /// Point and spot: where the light ends, smoothly (world units).
    pub attenuation_radius: f32,
    /// Spot: the cone, half angles (degrees).
    pub inner_cone: f32,
    pub outer_cone: f32,
    pub cast_shadows: bool,
    /// How strongly it lights scenery and characters (Unreal's lighting
    /// channels, as strengths).
    pub scenery: f32,
    pub characters: f32,
    pub specular: f32,
    /// Its light bounced around (from the probes, which see the art):
    /// original light only so far.
    pub indirect: f32,
    /// Light shafts in the air (the shadowed directional light).
    pub volumetric: f32,
    /// Original light: how much the shadowed directional light's shadows
    /// darken the painted light, and ambient occlusion on it.
    pub key_shadows: f32,
    pub occlusion: f32,
    /// Original light: highlights of the shadowed directional light on
    /// scenery, where the painted light is bright.
    pub highlights: f32,
}

impl Default for SceneLight {
    fn default() -> Self {
        Self {
            name: "Point light".into(),
            kind: LightType::Point,
            enabled: true,
            mobility: Mobility::Dynamic,
            game: GameLight::None,
            follow_direction: true,
            follow_color: true,
            position: [0.0; 3],
            direction: [0.0, -1.0, 0.0],
            color: [1.0; 3],
            lower_color: [0.3, 0.27, 0.25],
            use_temperature: false,
            temperature: 6500.0,
            intensity: 0.0,
            source: 2.0,
            attenuation_radius: 80.0,
            inner_cone: 20.0,
            outer_cone: 30.0,
            cast_shadows: true,
            scenery: 1.0,
            characters: 1.0,
            specular: 1.0,
            indirect: 1.0,
            volumetric: 1.0,
            key_shadows: 0.0,
            occlusion: 0.0,
            highlights: 0.0,
        }
    }
}

impl SceneLight {
    /// A new light of `kind` with its usual values.
    #[must_use]
    pub fn new(kind: LightType) -> Self {
        let base = Self {
            name: kind.name().into(),
            kind,
            ..Self::default()
        };
        match kind {
            LightType::Original => Self {
                intensity: 1.0,
                characters: 2.0,
                key_shadows: 0.4,
                occlusion: 0.6,
                highlights: 0.5,
                cast_shadows: false,
                ..base
            },
            LightType::Sky => Self {
                intensity: 0.3 * NITS_PER_UNIT,
                color: [0.6, 0.7, 0.9],
                scenery: 0.2,
                cast_shadows: false,
                ..base
            },
            LightType::Directional => Self {
                intensity: 0.6 * LUX_PER_UNIT,
                direction: math::normalize([-0.35, -0.85, 0.4]),
                color: [1.0, 0.95, 0.85],
                source: 1.4,
                ..base
            },
            LightType::Point | LightType::Spot => Self {
                // 1 unit of light 10 units away, as the editor's lights had.
                intensity: candela_for(1.0, 10.0),
                color: [1.0, 0.85, 0.6],
                ..base
            },
        }
    }

    /// Its colour: the temperature's or the colour's.
    #[must_use]
    pub fn tint(&self) -> [f32; 3] {
        if self.use_temperature {
            kelvin(self.temperature)
        } else {
            self.color
        }
    }
}

/// The candela of a light that gives `units` of light `at` world units
/// away.
#[must_use]
pub fn candela_for(units: f32, at: f32) -> f32 {
    let m = at / UNITS_PER_METRE;
    units * LUX_PER_UNIT * m * m
}

/// A scene's lights.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rig {
    /// EV100: higher is darker (bright day 14–15, overcast 12, interior
    /// 7–9, night 2–4).
    pub exposure: f32,
    pub lights: Vec<SceneLight>,
}

impl Default for Rig {
    /// The renderer's look: the game's painted light, and its key, fill
    /// and ambient light on characters.
    fn default() -> Self {
        let original = SceneLight::new(LightType::Original);
        let key = SceneLight {
            name: "Key light (game)".into(),
            game: GameLight::Key,
            scenery: 0.0,
            ..SceneLight::new(LightType::Directional)
        };
        let fill = SceneLight {
            name: "Fill light (game)".into(),
            game: GameLight::Fill,
            intensity: 0.2 * LUX_PER_UNIT,
            scenery: 0.0,
            cast_shadows: false,
            ..SceneLight::new(LightType::Directional)
        };
        let sky = SceneLight {
            name: "Sky light (game)".into(),
            game: GameLight::Ambient,
            ..SceneLight::new(LightType::Sky)
        };
        Self {
            exposure: 12.0,
            lights: vec![original, key, fill, sky],
        }
    }
}

/// The game's lights this frame (linear; directions the light travels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Game {
    pub key: ([f32; 3], [f32; 3]),
    pub fill: ([f32; 3], [f32; 3]),
    pub ambient: [f32; 3],
}

/// The rig as the lighting pass takes it, in its units.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    /// Times the lighting pass's values on the way to the screen.
    pub exposure: f32,
    /// Original light: on scenery, on characters, key shadows on it,
    /// occlusion on it, highlights, bounce (probes).
    pub original: [f32; 6],
    /// The shadowed directional light: where it travels, its light, on
    /// scenery and characters, specular, volumetric, source angle
    /// (radians, half), shadows on, static.
    pub key: Key,
    /// Sky: upper and lower light, on scenery and characters.
    pub sky: ([f32; 3], [f32; 3], f32, f32),
    /// Every other light (directional ones unshadowed).
    pub locals: Vec<Local>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub direction: [f32; 3],
    pub light: [f32; 3],
    pub scenery: f32,
    pub characters: f32,
    pub specular: f32,
    pub volumetric: f32,
    pub angle: f32,
    pub shadows: bool,
    pub static_shadows: bool,
}

impl Default for Key {
    fn default() -> Self {
        Self {
            direction: [0.0, -1.0, 0.0],
            light: [0.0; 3],
            scenery: 0.0,
            characters: 0.0,
            specular: 0.0,
            volumetric: 0.0,
            angle: 0.0,
            shadows: false,
            static_shadows: false,
        }
    }
}

fn scale3(c: [f32; 3], s: f32) -> [f32; 3] {
    [c[0] * s, c[1] * s, c[2] * s]
}

impl Rig {
    /// What the lighting pass uses this frame, with the game's lights.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one arm per kind of light
    pub fn resolve(&self, game: Option<&Game>) -> Resolved {
        let mut out = Resolved {
            exposure: 2f32.powf(EV_REFERENCE - self.exposure),
            original: [0.0; 6],
            key: Key::default(),
            sky: ([0.0; 3], [0.0; 3], 0.0, 0.0),
            locals: Vec::new(),
        };
        let (mut have_original, mut have_key, mut have_sky) = (false, false, false);
        for l in self.lights.iter().filter(|l| l.enabled) {
            let followed = match (l.game, game) {
                (GameLight::Key, Some(g)) => Some(g.key),
                (GameLight::Fill, Some(g)) => Some(g.fill),
                (GameLight::Ambient, Some(g)) => Some(([0.0, -1.0, 0.0], g.ambient)),
                _ => None,
            };
            let direction = match followed {
                Some((d, _)) if l.follow_direction => math::normalize(d),
                _ => math::normalize(l.direction),
            };
            let color = match followed {
                Some((_, c)) if l.follow_color => c,
                _ => l.tint(),
            };
            match l.kind {
                LightType::Original => {
                    if have_original {
                        continue;
                    }
                    have_original = true;
                    let i = l.intensity.max(0.0);
                    out.original = [
                        i * l.scenery,
                        i * l.characters,
                        l.key_shadows,
                        l.occlusion,
                        l.highlights,
                        l.indirect,
                    ];
                }
                LightType::Sky => {
                    if have_sky {
                        continue;
                    }
                    have_sky = true;
                    let i = l.intensity.max(0.0) / NITS_PER_UNIT;
                    out.sky = (
                        scale3(color, i),
                        scale3(l.lower_color, i),
                        l.scenery,
                        l.characters,
                    );
                }
                LightType::Directional => {
                    let light = scale3(color, l.intensity.max(0.0) / LUX_PER_UNIT);
                    if l.cast_shadows && !have_key {
                        have_key = true;
                        out.key = Key {
                            direction,
                            light,
                            scenery: l.scenery,
                            characters: l.characters,
                            specular: l.specular,
                            volumetric: l.volumetric,
                            angle: (l.source.max(0.0) * 0.5).to_radians(),
                            shadows: true,
                            static_shadows: l.mobility == Mobility::Static,
                        };
                    } else {
                        out.locals.push(Local {
                            name: l.name.clone(),
                            kind: LocalKind::Directional,
                            direction,
                            color: light,
                            intensity: 1.0,
                            range: 1.0,
                            shadows: false,
                            specular: l.specular,
                            scenery: l.scenery,
                            characters: l.characters,
                            ..Local::default()
                        });
                    }
                }
                LightType::Point | LightType::Spot => {
                    out.locals.push(Local {
                        name: l.name.clone(),
                        enabled: true,
                        kind: if l.kind == LightType::Spot {
                            LocalKind::Spot
                        } else {
                            LocalKind::Point
                        },
                        position: l.position,
                        color,
                        intensity: l.intensity.max(0.0)
                            / candela_for(1.0, super::lights::REFERENCE),
                        range: l.attenuation_radius.max(0.1),
                        direction,
                        inner: (l.inner_cone * 2.0).clamp(0.0, 178.0),
                        outer: (l.outer_cone * 2.0).clamp(1.0, 179.0),
                        shadows: l.cast_shadows,
                        specular: l.specular,
                        scenery: l.scenery,
                        characters: l.characters,
                    });
                }
            }
        }
        out
    }
}

/// Linear RGB of a black body at `k` kelvin, its brightest channel 1
/// (fitted to the CIE 1931 colours of Planck's law, 1000–40000 K).
#[must_use]
pub fn kelvin(k: f32) -> [f32; 3] {
    let t = k.clamp(1000.0, 40000.0) / 100.0;
    let r = if t <= 66.0 {
        255.0
    } else {
        329.698_73 * (t - 60.0).powf(-0.133_204_76)
    };
    let g = if t <= 66.0 {
        99.470_8 * t.ln() - 161.119_57
    } else {
        288.122_16 * (t - 60.0).powf(-0.075_514_85)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_73 * (t - 10.0).ln() - 305.044_8
    };
    let lin = |c: f32| {
        let c = (c / 255.0).clamp(0.0, 1.0);
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let v = [lin(r), lin(g), lin(b)];
    let m = v[0].max(v[1]).max(v[2]).max(1e-6);
    [v[0] / m, v[1] / m, v[2] / m]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_rig_is_the_old_look() {
        let game = Game {
            key: ([0.0, -1.0, 0.0], [1.0, 0.9, 0.8]),
            fill: ([1.0, 0.0, 0.0], [0.2, 0.2, 0.3]),
            ambient: [0.4, 0.4, 0.5],
        };
        let r = Rig::default().resolve(Some(&game));
        assert!((r.exposure - 1.3).abs() < 1e-3);
        assert!((r.original[0] - 1.0).abs() < 1e-6);
        assert!((r.original[1] - 2.0).abs() < 1e-6);
        // The game's key at 0.6 of its brightness, on characters only.
        assert!((r.key.light[0] - 0.6).abs() < 1e-4);
        assert!(r.key.scenery.abs() < 1e-6);
        assert!(r.key.shadows);
        // Sky light: the game's ambient at 0.3.
        assert!((r.sky.0[2] - 0.15).abs() < 1e-4);
        // The fill goes with the other lights, unshadowed.
        assert_eq!(r.locals.len(), 1);
        assert_eq!(r.locals[0].kind, LocalKind::Directional);
    }

    #[test]
    fn units_and_colours() {
        // A point light of `candela_for(1, 10)` is 1 unit of light at 10.
        let l = SceneLight::new(LightType::Point);
        let r = Rig {
            exposure: 12.0,
            lights: vec![l],
        }
        .resolve(None);
        assert!((r.locals[0].intensity - 1.0).abs() < 1e-4);
        let warm = kelvin(2700.0);
        let cold = kelvin(9000.0);
        assert!(warm[0] > warm[2] && cold[2] > cold[0]);
        let white = kelvin(6600.0);
        assert!(white.iter().all(|c| *c > 0.9));
    }
}
