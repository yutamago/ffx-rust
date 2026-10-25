//! What the builder needs from the running game for one frame: the scene
//! inputs. [`SceneInputs`] is the seam: [`crate::capture_inputs`] derives
//! them from a capture (round 2, until the live snapshot of
//! `inputs.rs` lands), the snapshot will implement the same trait.
//!
//! Matrices are 16 floats as the shaders get them in the constant buffer
//! ("cb layout": row by row for column vectors, translation in elements 3,
//! 7 and 11; `docs/formats/scene.md`, "Constants").

pub use crate::census::Mat;

/// The game's texture quality (`TextureQuality` in `GameSetting.ini`,
/// key `0x00b7c870`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureQuality {
    Low,
    Medium,
    High,
}

impl TextureQuality {
    /// From the `GameSetting.ini` value (`TQ_LOW`, `TQ_MEDIUM`,
    /// `TQ_HIGH`).
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "TQ_LOW" => Some(Self::Low),
            "TQ_MEDIUM" => Some(Self::Medium),
            "TQ_HIGH" => Some(Self::High),
            _ => None,
        }
    }

    /// Reads `TextureQuality=` from a `GameSetting.ini`'s text.
    #[must_use]
    pub fn from_ini(text: &str) -> Option<Self> {
        text.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "TextureQuality")
                .then(|| Self::parse(v))
                .flatten()
        })
    }

    /// The setting's index (`TQ_HIGH` 0, `TQ_MEDIUM` 1, `TQ_LOW` 2;
    /// `g_Settings +0x34`): the top mip levels the game drops from a
    /// texture cluster (`Phyre_PBinary_loadClusterTextures` `0x00465ab0`,
    /// with the exceptions of [`crate::assets::texture_start_level`];
    /// `docs/formats/scene-maps.md`, Q1). Only `TQ_LOW` is checked
    /// against captures (cdsp0000).
    #[must_use]
    pub fn dropped_levels(self) -> u32 {
        match self {
            Self::High => 0,
            Self::Medium => 1,
            Self::Low => crate::assets::TQ_LOW_DROPPED_LEVELS,
        }
    }
}

/// Game settings the draws depend on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub texture_quality: TextureQuality,
    /// The anisotropy setting (`0x00c25370`): above 1 every sampler the
    /// game makes is anisotropic with that maximum
    /// (`PSamplerStateD3D11` `0x00595bf0`, `0x0058aa10`).
    pub anisotropy: f32,
}

impl Settings {
    /// From a `GameSetting.ini`'s text: `TextureQuality` and `Anisotropic`
    /// (taken as the anisotropy value itself; 1 when absent).
    ///
    /// # Errors
    /// When `TextureQuality` is missing or unknown.
    pub fn from_ini(text: &str) -> Result<Self, String> {
        let texture_quality =
            TextureQuality::from_ini(text).ok_or("GameSetting.ini: no known TextureQuality")?;
        let anisotropy = text
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once('=')?;
                (k.trim() == "Anisotropic").then(|| v.trim().parse::<f32>().ok())?
            })
            .unwrap_or(1.0);
        Ok(Self {
            texture_quality,
            anisotropy,
        })
    }
}

/// The camera, as the renderer reads it (`PCamera`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// `scene.View`: `FromMatrix3x4(PCamera +0)`.
    pub view: Mat,
    /// `scene.ViewProjection` (depth 0..1).
    pub view_projection: Mat,
    /// The culling matrix `PCamera +0x70` (depth -w..w).
    pub cull: Mat,
}

/// A resource the host owns and the builder only binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum External {
    /// The scene's colour, view-space normal and depth targets.
    Color,
    Normal,
    Depth,
    /// A texture the game sets into a material's buffer at run time (the
    /// shadow maps `MySMSampler`, `MySMSampler2`): material and the
    /// sampler slot's offset.
    MaterialTexture {
        material: usize,
        offset: u32,
    },
    /// Vertices the game writes every frame into its dynamic vertex ring
    /// (the `Vertex` stream of a morphing mesh instance, `MorphManager`):
    /// instance, segment and the block whose layout they have.
    DynamicVertices {
        instance: usize,
        segment: usize,
        block: usize,
    },
}

/// What a material's texture slot holds this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotTexture {
    /// A texture cluster, by its import id (`PS3Data/.../x.dds`): the
    /// cluster's own, or one the game put there (texture animation).
    Import(String),
    /// A texture the game made at run time (the shadow maps):
    /// [`External::MaterialTexture`].
    Runtime,
    /// Nothing: no texture is bound.
    Empty,
}

/// The game state one frame's map draws come from.
pub trait SceneInputs {
    fn settings(&self) -> Settings;
    fn camera(&self) -> Camera;
    /// The passes `Draw3DWorld` sets for the map world (scene `+0xd0`
    /// `+0x48`): `Opaque` or `OpaqueNoMSAA`, `Transparent` or
    /// `TransparentNoMSAA`.
    fn pass_names(&self) -> (String, String);
    /// The world's `PMeshInstanceBounds` in its order: the map
    /// instance each one points at.
    fn bounds_order(&self) -> Vec<usize>;
    /// The visibility jobs: [`Self::bounds_order`] in runs of at most 400
    /// (`startJobsForAllBounds` `0x009f2250`), per instance list of the
    /// walk.
    fn jobs(&self) -> Vec<Vec<usize>> {
        self.bounds_order()
            .chunks(400)
            .map(<[usize]>::to_vec)
            .collect()
    }
    /// Whether the bounds' `m_meshInstance` is set (the game clears it to
    /// hide an instance).
    fn instance_shown(&self, instance: usize) -> bool;
    /// The bounds' `m_worldMatrix` (cb layout); `None`: null.
    fn world(&self, instance: usize) -> Option<Mat>;
    /// The bounds' `m_render_order`.
    fn render_order(&self, instance: usize) -> Option<i32>;
    /// A segment's FFX hidden byte (`+0x14`).
    fn segment_hidden(&self, segment: usize) -> bool;
    /// `g_DepthBias` (`0x0194299c`): scale and bias for an instance.
    fn depth_bias(&self, instance: usize) -> Option<(f32, f32)>;
    /// A material's live `PParameterBuffer` bytes (values at their
    /// definitions' offsets).
    fn material_buffer(&self, material: usize) -> Vec<u8>;
    /// The texture in a material's sampler slot (`offset`: the sampler
    /// definition's), given the cluster's import for it (`file`). The
    /// game sets run-time textures into slots the cluster leaves empty
    /// (the shadow maps `MySMSampler`) and swaps others (texture
    /// animation); a slot left empty binds nothing (`EnvironmentMapSampler`
    /// on cdsp0000).
    fn material_texture(&self, material: usize, offset: u32, file: Option<&str>) -> SlotTexture;
    /// The scene-wide parameter buffer (`PSceneWideParameters_getBuffer`).
    fn scene_wide(&self) -> Vec<u8>;
    /// `D3D11_VIEWPORT` of the scene targets.
    fn viewport(&self) -> [f32; 6];
    /// Left, top, right, bottom.
    fn scissor(&self) -> [i32; 4];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_quality_from_the_ini() {
        let ini = "[Graphics]\r\nQuality=VQ_LOW\r\nTextureQuality=TQ_LOW\r\nMSAA=0\r\n";
        assert_eq!(TextureQuality::from_ini(ini), Some(TextureQuality::Low));
        assert_eq!(TextureQuality::Low.dropped_levels(), 2);
        assert_eq!(TextureQuality::Medium.dropped_levels(), 1);
        assert_eq!(TextureQuality::High.dropped_levels(), 0);
        assert_eq!(TextureQuality::from_ini("x=1"), None);
    }
}
