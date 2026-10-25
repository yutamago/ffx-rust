//! The scene builder's inputs from the live snapshot: the `SceneInputs`
//! records the hook reads at the map's `renderWorld` calls
//! ([`crate::inputs`], `docs/formats/scene.md`, "Scene inputs"), with
//! their [`Wire`] block (`docs/scene-builder.md`, "Inputs from the
//! snapshot"). Nothing here reads a capture's draws.
//!
//! - Bounds, in the visibility check's walk order and per instance list
//!   (the jobs of 400), their `m_meshInstance` (shown), `m_worldMatrix`
//!   and `m_render_order`; `g_DepthBias`; the segments' hidden bytes.
//! - The materials' live `PParameterBuffer`s (the shadow cascades,
//!   `UVMorphWeight`, fades) and the textures in their slots (texture
//!   animation, run-time textures).
//! - The camera (`PCamera` `+0`, `+0x30`, `+0x70`, near, far) and its
//!   world matrix (`*(PCamera +0xb0)`), the renderer's scene-wide values
//!   (`g_PhyreApplication +0x40..`) and the screen size: the scene-wide
//!   buffer ([`scene_wide`]).
//!
//! Version 1 records (before the [`Wire`] block: `r2a-a`, `r2a-b`) lack
//! the camera's world matrix, the texture ids and the settings:
//! [`LiveInputs::new`] then takes `EyePosition` from `fallback` (the
//! gate reads it from the capture's constants), the slots' textures as
//! the cluster has them (a texture-animated slot differs: `r2a`'s
//! `besaid-101`) and the settings it is given.

// Matrix and record arithmetic names its terms as the formulas do.
#![allow(clippy::many_single_char_names)]

use std::collections::HashMap;

use crate::build::Clusters;
use crate::frame::{Camera, Mat, SceneInputs as Inputs, Settings, SlotTexture};
use crate::inputs::{self, SceneInputs, Wire};

/// The map world's offset in `PhyreScene`.
pub const MAP_WORLD: u32 = 0xac;

/// One scene-wide value: where it goes in the scene-wide buffer and
/// where it comes from. The offsets are the effects' (their location
/// tables' scene-wide entries, the same in every map shader; the
/// `scenewide` example lists them with the `$Globals` member each one
/// fills); the sources are `PSceneWideParameters_getBuffer`'s
/// (`0x004fdec0`) cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneValue {
    /// `size` bytes of `g_PhyreApplication` at this offset (the
    /// renderer's context, `app +0x40`, at `offset - 0x40`).
    App(usize),
    /// A matrix there (memory order, as the buffer holds every matrix).
    AppMatrix(usize),
    ScreenInverse,
    ViewProjection,
    ViewInverse,
    View,
    Projection,
    EyePosition,
    ViewportSize,
    NearFar,
    FarMinusNear,
    NearTimesFar,
}

/// The scene-wide buffer's layout: (offset, size, name, value).
pub const SCENE_WIDE: [(usize, usize, &str, SceneValue); 19] = [
    (0x08, 8, "screenWidthHeightInv", SceneValue::ScreenInverse),
    (0x10, 64, "ViewProjection2", SceneValue::AppMatrix(0x90)),
    (0x50, 64, "ViewProjection", SceneValue::ViewProjection),
    (0xd0, 64, "ViewInverse", SceneValue::ViewInverse),
    (0x110, 64, "View", SceneValue::View),
    (0x150, 64, "Projection", SceneValue::Projection),
    (0x1e0, 16, "EyePosition", SceneValue::EyePosition),
    (0x200, 12, "BGColor", SceneValue::App(0x80)),
    (0x20c, 12, "FogColor", SceneValue::App(0x60)),
    (0x218, 12, "GlobalAmbientColor", SceneValue::App(0x50)),
    (0x224, 4, "GlobalTime", SceneValue::App(0x120)),
    (0x230, 8, "ViewportWidthHeight", SceneValue::ViewportSize),
    (0x238, 8, "cameraNearFar", SceneValue::NearFar),
    (0x240, 4, "FogLimit", SceneValue::App(0x7c)),
    (0x244, 4, "FogCurve", SceneValue::App(0x78)),
    (0x248, 4, "FogFar", SceneValue::App(0x74)),
    (0x24c, 4, "FogNear", SceneValue::App(0x70)),
    (0x25c, 4, "cameraFarMinusNear", SceneValue::FarMinusNear),
    (0x260, 4, "cameraNearTimesFar", SceneValue::NearTimesFar),
];

/// The scene-wide buffer's size (past its last value).
pub const SCENE_WIDE_LEN: usize = 0x264;

/// What a version 1 record can't give, from elsewhere (the gate takes it
/// from the capture's constants): `EyePosition`, the translation of the
/// camera's world matrix (the view's inverse is 1 ulp off it).
#[derive(Debug, Clone, Default)]
pub struct Fallback {
    pub eye_position: Option<[f32; 4]>,
}

/// A matrix in the shaders' order as the buffer holds it (memory order:
/// the constant buffer copy transposes every `float4x4`,
/// `build::copy_matrices`).
fn memory(m: Mat) -> Vec<f32> {
    inputs::transpose(&m).to_vec()
}

fn put(buf: &mut [u8], at: usize, words: &[f32]) {
    for (k, w) in words.iter().enumerate() {
        if let Some(d) = buf.get_mut(at + 4 * k..at + 4 * k + 4) {
            d.copy_from_slice(&w.to_le_bytes());
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // the fstp dword; u32 to float
fn inverse(x: u32) -> f32 {
    // `fild` of the size (as a float first), then `fdivr 1.0`, stored.
    (1.0 / f64::from(x as f32)) as f32
}

fn round(x: f64) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // the fstp dword
    {
        x as f32
    }
}

/// `0x004cc890` through `PMatrix4_Inverse_004cc0c0` (with `0x004c9360`):
/// the inverse of an affine `PMatrix4` (memory order: the 3x3 in words
/// 0-2, 4-6, 8-10, the translation in 12-14), as the x87 at 53 bits
/// computes it. Each cofactor is a difference of two exact products
/// rounded to a float; the determinant `row 2 . cofactors of column 0`
/// is summed with a store after each term; its inverse `1 / det` is
/// stored; each element of the inverse 3x3 is `cofactor * inv`, stored;
/// the translation `-(p14 * r8 + (p13 * r4 + p12 * r0))` (each product
/// stored, then each sum). Words 3, 7 and 11 are 0, 15 is 1.
#[must_use]
pub fn affine_inverse(p: &Mat) -> Mat {
    let x = |i: usize| f64::from(p[i]);
    let cof = |a: usize, b: usize, c: usize, d: usize| round(x(a) * x(b) - x(c) * x(d));
    // Cofactors by output element: c00 c01 c02 (output 0, 4, 8),
    // c10 c11 c12 (1, 5, 9), c20 c21 c22 (2, 6, 10).
    let c00 = cof(5, 10, 6, 9);
    let c01 = cof(6, 8, 4, 10);
    let c02 = cof(4, 9, 5, 8);
    let c10 = cof(2, 9, 1, 10);
    let c11 = cof(0, 10, 2, 8);
    let c12 = cof(1, 8, 0, 9);
    let c20 = cof(1, 6, 2, 5);
    let c21 = cof(4, 2, 0, 6);
    let c22 = cof(0, 5, 1, 4);
    let d1 = round(x(8) * f64::from(c20));
    let d2 = round(x(9) * f64::from(c21) + f64::from(d1));
    let d3 = round(x(10) * f64::from(c22) + f64::from(d2));
    let inv = f64::from(round(1.0 / f64::from(d3)));
    let r = |c: f32| round(f64::from(c) * inv);
    let rows = [
        [r(c00), r(c10), r(c20)],
        [r(c01), r(c11), r(c21)],
        [r(c02), r(c12), r(c22)],
    ];
    let mut out = [0.0f32; 16];
    for (k, row) in rows.iter().enumerate() {
        out[4 * k..4 * k + 3].copy_from_slice(row);
    }
    for col in 0..3 {
        let t = round(x(12) * f64::from(rows[0][col]));
        let u = round(x(13) * f64::from(rows[1][col]));
        let v = round(x(14) * f64::from(rows[2][col]));
        let s1 = round(f64::from(u) + f64::from(t));
        let w = round(f64::from(v) + f64::from(s1));
        out[12 + col] = -w;
    }
    out[15] = 1.0;
    out
}

/// The scene-wide buffer of a record (`PSceneWideParameters_getBuffer`
/// `0x004fdec0`): the camera's matrices as `ffx_scene::inputs` derives
/// them, `ViewInverse` the view's inverse (`PMatrix4_Inverse_004cc0c0`,
/// [`affine_inverse`]), `EyePosition` the camera world matrix's
/// translation (`*(PCamera +0xb0)`, the key's `+0xd0`) with w 1, the
/// renderer's values copied, the screen size and its inverse, near and
/// far with their difference and product. `None` where a value is
/// missing (`missing` names it).
#[must_use]
pub fn scene_wide(
    s: &SceneInputs,
    camera_world: Option<&inputs::Matrix3x4>,
    fallback: &Fallback,
    missing: &mut Vec<String>,
) -> Vec<u8> {
    let mut buf = vec![0u8; SCENE_WIDE_LEN];
    let (near, far) = inputs::near_far(s).unwrap_or((0.0, 0.0));
    let [w, h] = s.screen;
    #[allow(clippy::cast_precision_loss)] // u32 to float, as the game's fild
    let size = [w as f32, h as f32];
    for (at, len, name, v) in SCENE_WIDE {
        let words: Option<Vec<f32>> = match v {
            SceneValue::App(o) => {
                let n = len / 4;
                (0..n)
                    .map(|k| inputs::app_f32(s, o + 4 * k))
                    .collect::<Option<Vec<f32>>>()
            }
            SceneValue::AppMatrix(o) => (0..16)
                .map(|k| inputs::app_f32(s, o + 4 * k))
                .collect::<Option<Vec<f32>>>(),
            SceneValue::ScreenInverse => Some(vec![inverse(w), inverse(h)]),
            SceneValue::ViewProjection => inputs::view_projection(s).map(memory),
            SceneValue::View => inputs::view(s).map(memory),
            SceneValue::Projection => inputs::projection(s).map(memory),
            SceneValue::ViewInverse => {
                inputs::view(s).map(|v| affine_inverse(&inputs::transpose(&v)).to_vec())
            }
            SceneValue::EyePosition => camera_world
                .map(|m| [m[8], m[9], m[10], 1.0])
                .or(fallback.eye_position)
                .map(|m| m.to_vec()),
            SceneValue::ViewportSize => Some(size.to_vec()),
            SceneValue::NearFar => Some(vec![near, far]),
            #[allow(clippy::cast_possible_truncation)] // the fstp dword
            SceneValue::FarMinusNear => Some(vec![(f64::from(far) - f64::from(near)) as f32]),
            #[allow(clippy::cast_possible_truncation)] // the fstp dword
            SceneValue::NearTimesFar => Some(vec![(f64::from(near) * f64::from(far)) as f32]),
        };
        match words {
            Some(w) => put(&mut buf, at, &w),
            None => missing.push(format!("scene-wide {name}")),
        }
    }
    buf
}

/// The live inputs of one frame's map passes.
#[derive(Debug, Clone)]
pub struct LiveInputs {
    pub settings: Settings,
    pub pass_names: (String, String),
    pub camera: Camera,
    /// Per instance list of the walk, the map instances of its bounds,
    /// in order (the visibility jobs run per list).
    pub lists: Vec<Vec<usize>>,
    /// Per map instance: shown, world (cb layout), render order.
    pub shown: HashMap<usize, bool>,
    pub worlds: HashMap<usize, Option<Mat>>,
    pub render_orders: HashMap<usize, i32>,
    pub depth_bias: HashMap<usize, (f32, f32)>,
    /// Global segment → hidden byte set.
    pub hidden: HashMap<usize, bool>,
    /// Map material → the snapshot's material (index into the record's).
    pub materials: HashMap<usize, usize>,
    pub scene_wide: Vec<u8>,
    pub screen: [u32; 2],
    record: SceneInputs,
    wire: Option<Wire>,
    /// What the snapshot didn't give or didn't match the clusters.
    pub notes: Vec<String>,
}

fn model_of(c: &Clusters, s: &SceneInputs, wire: Option<&Wire>, cluster: usize) -> Option<usize> {
    if let Some(asset) = wire.and_then(|w| w.cluster_assets.get(cluster))
        && !asset.is_empty()
    {
        return c.model_of(&crate::map::archive_path(asset));
    }
    let name = &s.clusters.get(cluster)?.name;
    if name.is_empty() {
        return None;
    }
    let tail = format!("/{name}.dae.phyre");
    c.models.iter().position(|m| m.0.ends_with(&tail))
}

impl LiveInputs {
    /// The map passes' inputs from one frame's records (the map world's
    /// `Opaque*` and `Transparent*` calls; `post*` calls draw nothing of
    /// the map). `fallback_settings` stand in for a version 1 record's.
    ///
    /// # Errors
    /// When the records lack the map's opaque or transparent call, or the
    /// world holds no loaded model.
    #[allow(clippy::too_many_lines)] // one pass over the record's tables
    pub fn new(
        c: &Clusters,
        records: &[(SceneInputs, Option<Wire>)],
        fallback_settings: Settings,
        fallback: &Fallback,
    ) -> Result<Self, String> {
        let pick = |p: &str| {
            records
                .iter()
                .find(|(s, _)| s.world == MAP_WORLD && s.pass.starts_with(p))
        };
        let (s, wire) = pick("Opaque").ok_or("no map Opaque renderWorld record")?;
        let (t, _) = pick("Transparent").ok_or("no map Transparent renderWorld record")?;
        let mut notes = Vec::new();
        let settings = wire.as_ref().map_or(fallback_settings, |w| Settings {
            texture_quality: match w.texture_quality {
                0 => crate::frame::TextureQuality::High,
                1 => crate::frame::TextureQuality::Medium,
                _ => crate::frame::TextureQuality::Low,
            },
            anisotropy: f32::from_bits(w.anisotropy),
        });
        let view = inputs::view(s).ok_or("record: no camera view")?;
        let view_projection = inputs::view_projection(s).ok_or("record: no camera projection")?;
        let cull =
            inputs::transpose(&inputs::cull_view_projection(s).ok_or("record: no culling matrix")?);
        // Clusters to models, snapshot instances to map instances.
        let models: Vec<Option<usize>> = (0..s.clusters.len())
            .map(|k| model_of(c, s, wire.as_ref(), k))
            .collect();
        if models.iter().all(Option::is_none) {
            return Err("no cluster of the world is a loaded model".into());
        }
        let global = |cluster: u32, slot: u32| -> Option<usize> {
            let m = (*models.get(cluster as usize)?)?;
            let r = &c.models[m].1;
            let i = r.start + slot as usize;
            (i < r.end).then_some(i)
        };
        let instance_of: Vec<Option<usize>> = s
            .instances
            .iter()
            .map(|i| global(i.cluster, i.slot))
            .collect();
        let mut lists: Vec<Vec<usize>> = Vec::new();
        let mut shown = HashMap::new();
        let mut worlds = HashMap::new();
        let mut render_orders = HashMap::new();
        let mut by_bounds = Vec::new();
        for b in &s.bounds {
            // The map instance: the one the bounds point at, else (hidden,
            // `m_meshInstance` 0) the one whose bounds object this is.
            let inst = match b.instance {
                Some(i) => instance_of.get(i as usize).copied().flatten(),
                None => models
                    .get(b.cluster as usize)
                    .copied()
                    .flatten()
                    .and_then(|m| {
                        c.models[m]
                            .1
                            .clone()
                            .find(|&i| c.map.instances[i].bounds_slot == Some(b.slot as usize))
                    }),
            };
            by_bounds.push(inst);
            let Some(i) = inst else {
                if b.instance.is_some() {
                    notes.push(format!(
                        "bounds of cluster {} slot {}: no loaded model",
                        b.cluster, b.slot
                    ));
                }
                continue;
            };
            let list = b.list as usize;
            if lists.len() <= list {
                lists.resize(list + 1, Vec::new());
            }
            lists[list].push(i);
            shown.insert(i, b.instance.is_some());
            worlds.insert(i, b.world.as_ref().map(|m| inputs::world(Some(m))));
            render_orders.insert(i, b.render_order);
            if let Some(fb) = c.map.instances[i].bounds
                && (fb.min.map(f32::to_bits) != b.min.map(f32::to_bits)
                    || fb.size.map(f32::to_bits) != b.size.map(f32::to_bits))
            {
                notes.push(format!(
                    "instance {} bounds differ from the file's",
                    c.map.instances[i].name
                ));
            }
        }
        let mut depth_bias = HashMap::new();
        for d in &s.depth_bias {
            if let Some(i) = d
                .bounds
                .and_then(|b| by_bounds.get(b as usize).copied().flatten())
            {
                depth_bias.entry(i).or_insert((d.scale, d.bias));
            }
        }
        // Hidden bytes and materials, through each instance's segments
        // and material set.
        let mut hidden = HashMap::new();
        let mut materials = HashMap::new();
        for (k, si) in s.instances.iter().enumerate() {
            let Some(i) = instance_of[k] else { continue };
            let inst = &c.map.instances[i];
            if si.segments.len() != inst.segments.len() {
                notes.push(format!("instance {}: segment count differs", inst.name));
            }
            for (seg, &g) in si.segments.iter().zip(&inst.segments) {
                *hidden.entry(g).or_insert(false) |= seg.hidden != 0;
            }
            for (live, file) in si.materials.iter().zip(&inst.materials) {
                if let (Some(l), Some(f)) = (live, file) {
                    let prev = materials.insert(*f, *l as usize);
                    if prev.is_some_and(|p| p != *l as usize) {
                        notes.push(format!(
                            "material {f}: two live materials (a set changed at run time)"
                        ));
                    }
                }
            }
        }
        let mut missing = Vec::new();
        let scene_wide = scene_wide(
            s,
            wire.as_ref().and_then(|w| w.camera_world.as_ref()),
            fallback,
            &mut missing,
        );
        notes.extend(
            missing
                .into_iter()
                .map(|m| format!("{m}: not in the record")),
        );
        Ok(Self {
            settings,
            pass_names: (s.pass.clone(), t.pass.clone()),
            camera: Camera {
                view,
                view_projection,
                cull,
            },
            lists,
            shown,
            worlds,
            render_orders,
            depth_bias,
            hidden,
            materials,
            scene_wide,
            screen: s.screen,
            record: s.clone(),
            wire: wire.clone(),
            notes,
        })
    }

    /// The import ids of every texture the record's material slots hold
    /// (for [`Clusters::add_textures`]).
    #[must_use]
    pub fn texture_imports(&self) -> Vec<String> {
        self.wire
            .iter()
            .flat_map(|w| &w.material_textures)
            .flatten()
            .filter(|t| !t.asset.is_empty())
            .map(|t| t.asset.clone())
            .collect()
    }

    fn live_material(&self, material: usize) -> Option<&inputs::Material> {
        self.record.materials.get(*self.materials.get(&material)?)
    }
}

impl Inputs for LiveInputs {
    fn settings(&self) -> Settings {
        self.settings
    }

    fn camera(&self) -> Camera {
        self.camera
    }

    fn pass_names(&self) -> (String, String) {
        self.pass_names.clone()
    }

    fn bounds_order(&self) -> Vec<usize> {
        self.lists.concat()
    }

    fn jobs(&self) -> Vec<Vec<usize>> {
        self.lists
            .iter()
            .flat_map(|l| l.chunks(400).map(<[usize]>::to_vec))
            .collect()
    }

    fn instance_shown(&self, instance: usize) -> bool {
        self.shown.get(&instance).copied().unwrap_or(false)
    }

    fn world(&self, instance: usize) -> Option<Mat> {
        self.worlds.get(&instance).copied().flatten()
    }

    fn render_order(&self, instance: usize) -> Option<i32> {
        self.render_orders.get(&instance).copied()
    }

    fn segment_hidden(&self, segment: usize) -> bool {
        self.hidden.get(&segment).copied().unwrap_or(false)
    }

    fn depth_bias(&self, instance: usize) -> Option<(f32, f32)> {
        self.depth_bias.get(&instance).copied()
    }

    fn material_buffer(&self, material: usize) -> Vec<u8> {
        self.live_material(material)
            .map(|m| m.parameters.clone())
            .unwrap_or_default()
    }

    /// With the [`Wire`] block: the slot's texture by its asset id, a
    /// run-time one when it has none, nothing when the slot is empty.
    /// Without it: the cluster's texture, else a run-time one when the
    /// slot's texture pointer (`+0xc` of the 16-byte slot, in the live
    /// buffer) is set.
    fn material_texture(&self, material: usize, offset: u32, file: Option<&str>) -> SlotTexture {
        let live = self.materials.get(&material).copied();
        if let (Some(w), Some(m)) = (&self.wire, live)
            && let Some(slot) = w
                .material_textures
                .get(m)
                .and_then(|l| l.iter().find(|t| t.offset == offset))
        {
            return match (slot.set, slot.asset.is_empty()) {
                (false, _) => SlotTexture::Empty,
                (true, false) => SlotTexture::Import(slot.asset.clone()),
                (true, true) => SlotTexture::Runtime,
            };
        }
        if let Some(f) = file {
            return SlotTexture::Import(f.to_owned());
        }
        let set = self.live_material(material).is_some_and(|m| {
            let at = offset as usize + 0xc;
            m.parameters
                .get(at..at + 4)
                .is_some_and(|b| b.iter().any(|&x| x != 0))
        });
        if set {
            SlotTexture::Runtime
        } else {
            SlotTexture::Empty
        }
    }

    fn scene_wide(&self) -> Vec<u8> {
        self.scene_wide.clone()
    }

    /// The scene targets' viewport: the screen (`0x00c95100`).
    #[allow(clippy::cast_precision_loss)] // the screen size as floats
    fn viewport(&self) -> [f32; 6] {
        [
            0.0,
            0.0,
            self.screen[0] as f32,
            self.screen[1] as f32,
            0.0,
            1.0,
        ]
    }

    fn scissor(&self) -> [i32; 4] {
        [
            0,
            0,
            self.screen[0].cast_signed(),
            self.screen[1].cast_signed(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: Mat = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];

    #[test]
    fn the_affine_inverse_of_simple_matrices() {
        assert_eq!(affine_inverse(&IDENTITY), IDENTITY);
        // Scale 2, translation (1, 2, 3) (memory order: 12-14).
        let mut m = IDENTITY;
        for k in [0, 5, 10] {
            m[k] = 2.0;
        }
        m[12..15].copy_from_slice(&[1.0, 2.0, 3.0]);
        let i = affine_inverse(&m);
        assert_eq!([i[0], i[5], i[10], i[15]], [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(i[12..15], [-0.5, -1.0, -1.5]);
        assert_eq!([i[3], i[7], i[11]], [0.0; 3]);
        // A turn about y: the inverse is the transpose.
        let mut r = IDENTITY;
        r[0] = 0.0;
        r[2] = -1.0;
        r[8] = 1.0;
        r[10] = 0.0;
        let i = affine_inverse(&r);
        assert_eq!([i[0], i[2], i[8], i[10]], [0.0, 1.0, -1.0, 0.0]);
    }

    #[test]
    fn the_scene_wide_layout_is_ordered() {
        let mut end = 0;
        for (at, len, name, _) in SCENE_WIDE {
            assert!(at >= end, "{name} overlaps");
            end = at + len;
        }
        assert_eq!(end, SCENE_WIDE_LEN);
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)] // the f32 stores
    fn scene_wide_values_from_a_record() {
        let mut camera = vec![0u8; ffx_gfx::scene_inputs::CAMERA_LEN];
        let mut put_f = |at: usize, v: f32| camera[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put_f(inputs::camera::NEAR, 0.5);
        put_f(inputs::camera::FAR, 1000.0);
        let mut app = vec![0u8; ffx_gfx::scene_inputs::APP_LEN];
        let at = inputs::app::FOG_NEAR - ffx_gfx::scene_inputs::APP_FROM;
        app[at..at + 4].copy_from_slice(&7.0f32.to_le_bytes());
        let s = SceneInputs {
            camera,
            app,
            screen: [1280, 720],
            ..SceneInputs::default()
        };
        let mut missing = Vec::new();
        let buf = scene_wide(&s, None, &Fallback::default(), &mut missing);
        let f = |at: usize| f32::from_le_bytes(buf[at..at + 4].try_into().expect("word"));
        assert_eq!(f(0x08), (1.0f64 / 1280.0) as f32);
        assert_eq!(f(0x0c), (1.0f64 / 720.0) as f32);
        assert_eq!([f(0x230), f(0x234)], [1280.0, 720.0]);
        assert_eq!([f(0x238), f(0x23c)], [0.5, 1000.0]);
        assert_eq!(f(0x24c), 7.0);
        assert_eq!(f(0x25c), 999.5);
        assert_eq!(f(0x260), 500.0);
        // No camera world matrix and no fallback: EyePosition is missing.
        assert_eq!(missing, vec!["scene-wide EyePosition".to_owned()]);
    }
}
