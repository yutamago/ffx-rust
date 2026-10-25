//! The scene builder's inputs at one `renderWorld` call, as the hook reads
//! them from the game's memory (`ffx-hook`'s `world.rs`) for reference
//! captures: the `SceneInputs` message of `capture.ffxgfx`
//! (`docs/formats/scene.md`, "Scene inputs"). `ffx_scene::inputs` reads
//! them and derives what the draws use.
//!
//! Nothing here is a pointer: the hook resolves every object to a stable
//! index (the world's cluster order, slots in a cluster's instance lists,
//! this record's own tables), so two runs of the same frame give equal
//! records where the game state is equal.
//!
//! Encoding (inside the message payload): a `u32` version
//! ([`VERSION`]), then the fields in declaration order, lists as a `u32`
//! count and their items, byte strings as a `u32` length and the bytes,
//! options as a `u8` 0/1 and the value, floats as their bits.

use crate::{DecodeError, Reader, Writer};

/// The payload layout version.
pub const VERSION: u32 = 1;

/// Bytes of `PCamera` the hook copies: the view (3x4 at `+0`), the
/// projection (`+0x30`), the culling view-projection (`+0x70`), near and
/// far (`+0xb8`, `+0xbc`), aspect and vertical FOV (`+0x150`, `+0x154`).
pub const CAMERA_LEN: usize = 0x180;
/// `g_PhyreApplication` bytes the hook copies, from `+0x40` (the context
/// `renderWorld` passes on, `+0x40`, holds the scene-wide values: ambient
/// `+0x50`, fog `+0x60..+0x7c`, background `+0x80`, `ViewProjection2`
/// `+0x90`, `GlobalTime` `+0x120`).
pub const APP_FROM: usize = 0x40;
pub const APP_LEN: usize = 0x140;

/// A 3x4 matrix as `PhyreEngine` stores it (`PMatrix4x3`, 12 floats):
/// `PMatrix4_FromMatrix3x4` (`0x00451080`) makes the 4x4 from it.
pub type Matrix3x4 = [f32; 12];

/// One `renderWorld` call.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneInputs {
    /// The capture frame that draws this call's commands (from 0;
    /// `u32::MAX` when the capture holds no such frame).
    pub frame: u32,
    /// The update whose render made the call (the harness's frame label:
    /// updates run so far). `renderWorld` runs on the main thread after
    /// the update; the render thread draws its commands a frame later.
    pub update: u32,
    /// The thread that made the call.
    pub thread: u32,
    /// The world's offset in `PhyreScene` (`0xac` map, `0x7c` characters,
    /// `0xbc` ground shadows; 0 another world).
    pub world: u32,
    /// The renderer's pass (`*(renderer +0x48)`: `Opaque`,
    /// `OpaqueNoMSAA`, `Transparent`, `TransparentNoMSAA`) and the sort
    /// direction (1 opaque, -1 transparent).
    pub pass: String,
    pub dir: i32,
    /// `PCamera` bytes `0..CAMERA_LEN`.
    pub camera: Vec<u8>,
    /// `g_PhyreApplication` bytes `APP_FROM..APP_FROM + APP_LEN`.
    pub app: Vec<u8>,
    /// The screen size (`0x00c95100`, two `u32`s).
    pub screen: [u32; 2],
    /// The scene's switches that hide or move map parts: the group mask
    /// (`PhyreScene +0xfb14`: an instance whose `m_groupID` bit is clear
    /// gets `m_meshInstance` 0, `0x0066d610`), the shadow solution
    /// (`+0xf9cc`, `ShadowManager_selectShadowSolution`), the map's alpha
    /// fast flag (`+0xd4`, the transparent pass order).
    pub group_mask: u32,
    pub shadow_solution: u32,
    pub alpha_fast: u8,
    /// The 2D layers (`PhyreScene +0x1140`, `+0x1138` entries; `+0x11d8`
    /// the current group): `PhyreScene_show2DLayer` (`0x0066f2f0`).
    pub layer_group: u32,
    pub layers: Vec<Layer>,
    /// `g_DepthBias_Count` (`0x01942998`) and the entries in use.
    pub depth_bias_count: u32,
    pub depth_bias: Vec<DepthBias>,
    /// The world's clusters that hold the bounds and instances below.
    pub clusters: Vec<Cluster>,
    /// Every `PMeshInstanceBounds` of the world, in the order the
    /// renderer's visibility check walks them.
    pub bounds: Vec<Bounds>,
    /// The mesh instances the bounds point at.
    pub instances: Vec<Instance>,
    /// The materials those instances' segments use.
    pub materials: Vec<Material>,
    /// The characters' world (`+0x7c`) and the ground-shadow world
    /// (`+0xbc`) only: what their draws take besides the above
    /// (`docs/formats/scene-chars.md`). Written after the fields above,
    /// absent in older records and in the map's.
    pub chars: Option<Box<CharWorld>>,
}

/// The character-world part of a record ([`SceneInputs::chars`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharWorld {
    /// Per entry of [`SceneInputs::instances`], in the same order: its
    /// skinning inputs.
    pub instances: Vec<CharInstance>,
    /// The cloth vertex buffers `batchSetClothVertexBuffers`
    /// (`0x006365f0`) uploads, per (instance, segment).
    pub cloth: Vec<Cloth>,
    /// The character shader's dynamic geometry lists (the blob shadow
    /// quads `Ch_DrawQuad` `0x00831d10` appends), the current buffer's.
    pub dyn_geo: Option<DynGeo>,
    /// The renderer's two camera matrices `generateNodeParameterCaptureBuffer`
    /// reads (`renderer +0x60` and `+0xa0`, 4x4 each, memory order):
    /// `WorldViewInverse` is `WorldInverse . (+0x60)`, `WorldView`
    /// `(+0xa0) . World`.
    pub renderer: Vec<[f32; 16]>,
}

/// A `PMeshInstance`'s skinning inputs, as `computeJointMatrices`
/// (`0x004f6ef0`) and `generateNodeParameterCaptureBuffer`'s
/// `BoneTransforms` (`0x005b7fe0`, semantic `0x8f`) read them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharInstance {
    /// `+0x6c` (bit 0: the instance world is the identity).
    pub flags: u32,
    /// Per joint (mesh `+0x18` of them): the instance's matrix (`+0xc`,
    /// 16 floats, column-major: the joint relative to its parent) and its
    /// parent from the mesh's `+0x2c` table (`i32`, -1 the instance
    /// world).
    pub joints: Vec<[f32; 16]>,
    pub joint_parent: Vec<i32>,
    /// The mesh's skeleton matrices (`+8` count, `+0xc` array): each
    /// joint's inverse bind matrix.
    pub skeleton: Vec<[f32; 16]>,
    /// Per segment (as [`Instance::segments`]): `m_skinBones` (`+8`
    /// count, `+0xc` array of `PSkinBoneRemap`: hierarchy and skeleton
    /// matrix index).
    pub skin_bones: Vec<Vec<[u16; 2]>>,
}

/// One cloth vertex buffer (`batchSetClothVertexBuffers`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cloth {
    /// Index into [`SceneInputs::instances`] (`u32::MAX`: an instance
    /// outside this world) and the segment.
    pub instance: u32,
    pub segment: u32,
    /// The graphic character slot (`0x00ccb254` table) and the instance's
    /// place in its list.
    pub slot: u32,
    pub list_index: u32,
    /// The cloth object's first 16 floats (what `ClothWorld` is set
    /// from), its `+0x44` (vertex count) and the bytes of its `+0x4c` (16
    /// per vertex).
    pub matrix: [f32; 16],
    pub count: u32,
    pub vertices: Vec<u8>,
}

/// The dynamic geometry lists of `ClassChrShaderRender`
/// (`graphicShaderGetVertexInfo` `0x00643db0`, record `+0x94`): `+4`
/// vertex count, `+8` index count, then per vertex the arrays at `+0xc`
/// (12 bytes), `+0x10` (12), `+0x14` (16), `+0x18` (8), and the `u16`
/// indices at `+0x1c`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DynGeo {
    /// The buffer id (`+0x10` of the manager).
    pub id: u32,
    pub vertices: u32,
    pub indices: u32,
    pub arrays: [Vec<u8>; 4],
    pub index_bytes: Vec<u8>,
}

fn write_mat16(w: &mut Writer, m: &[f32; 16]) {
    write_floats(w, m);
}

fn write_mats(w: &mut Writer, v: &[[f32; 16]]) {
    w.u32(len(v.len()));
    for m in v {
        write_mat16(w, m);
    }
}

fn read_mats(r: &mut Reader) -> Result<Vec<[f32; 16]>, DecodeError> {
    let mut out = Vec::new();
    for _ in 0..count(r, 64)? {
        out.push(read_floats(r)?);
    }
    Ok(out)
}

impl CharWorld {
    fn write(&self, w: &mut Writer) {
        w.u32(len(self.instances.len()));
        for i in &self.instances {
            w.u32(i.flags);
            write_mats(w, &i.joints);
            w.u32(len(i.joint_parent.len()));
            for &p in &i.joint_parent {
                w.i32(p);
            }
            write_mats(w, &i.skeleton);
            w.u32(len(i.skin_bones.len()));
            for s in &i.skin_bones {
                w.u32(len(s.len()));
                for b in s {
                    w.u32(u32::from(b[0]) | u32::from(b[1]) << 16);
                }
            }
        }
        w.u32(len(self.cloth.len()));
        for c in &self.cloth {
            w.u32s(&[c.instance, c.segment, c.slot, c.list_index, c.count]);
            write_mat16(w, &c.matrix);
            w.blob(&c.vertices);
        }
        match &self.dyn_geo {
            Some(d) => {
                w.u8(1);
                w.u32s(&[d.id, d.vertices, d.indices]);
                for a in &d.arrays {
                    w.blob(a);
                }
                w.blob(&d.index_bytes);
            }
            None => w.u8(0),
        }
        write_mats(w, &self.renderer);
    }

    fn read(r: &mut Reader) -> Result<Self, DecodeError> {
        let mut out = Self::default();
        for _ in 0..count(r, 20)? {
            let flags = r.u32()?;
            let joints = read_mats(r)?;
            let mut joint_parent = Vec::new();
            for _ in 0..count(r, 4)? {
                joint_parent.push(r.i32()?);
            }
            let skeleton = read_mats(r)?;
            let mut skin_bones = Vec::new();
            for _ in 0..count(r, 4)? {
                let mut s = Vec::new();
                for _ in 0..count(r, 4)? {
                    let v = r.u32()?;
                    #[allow(clippy::cast_possible_truncation)] // two halves
                    s.push([v as u16, (v >> 16) as u16]);
                }
                skin_bones.push(s);
            }
            out.instances.push(CharInstance {
                flags,
                joints,
                joint_parent,
                skeleton,
                skin_bones,
            });
        }
        for _ in 0..count(r, 88)? {
            let [instance, segment, slot, list_index, count] = r.u32s()?;
            out.cloth.push(Cloth {
                instance,
                segment,
                slot,
                list_index,
                matrix: read_floats(r)?,
                count,
                vertices: r.blob()?,
            });
        }
        if r.u8()? != 0 {
            let [id, vertices, indices] = r.u32s()?;
            out.dyn_geo = Some(DynGeo {
                id,
                vertices,
                indices,
                arrays: [r.blob()?, r.blob()?, r.blob()?, r.blob()?],
                index_bytes: r.blob()?,
            });
        }
        out.renderer = read_mats(r)?;
        Ok(out)
    }
}

/// A `g_DepthBias` entry: `key * scale + bias` for the bounds it names.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DepthBias {
    /// Index into [`SceneInputs::bounds`], or `None` when the entry's
    /// key is no bounds of this world.
    pub bounds: Option<u32>,
    pub scale: f32,
    pub bias: f32,
}

/// A 2D layer entry (8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Layer {
    /// `+0` shown, `+1` group (signed), `+2` id (signed).
    pub shown: u8,
    pub group: i8,
    pub id: i16,
    /// `+4` its mesh instance: whether set, and (cluster index, slot)
    /// when it is in this world's clusters.
    pub has_mesh: bool,
    pub mesh: Option<(u32, u32)>,
}

impl Layer {
    fn read(r: &mut Reader) -> Result<Self, DecodeError> {
        let shown = r.u8()?;
        let group = r.u8()?.cast_signed();
        let id = u16::try_from(r.u32()?)
            .map_err(|_| DecodeError("scene inputs: layer id"))?
            .cast_signed();
        let has_mesh = r.u8()? != 0;
        let mesh = if r.u8()? == 0 {
            None
        } else {
            Some((r.u32()?, r.u32()?))
        };
        Ok(Self {
            shown,
            group,
            id,
            has_mesh,
            mesh,
        })
    }
}

/// A cluster: its place in the world's cluster list and what names it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cluster {
    /// Position in the world's cluster list (`PWorld +0`, newest first).
    pub index: u32,
    /// The map model's symbol (`PhyreScene +0xd8`) when this is the
    /// scene's model cluster (`PhyreScene +0x10d4`), else empty.
    pub name: String,
    /// Its instance lists: (class name, objects including free slots).
    pub lists: Vec<(String, u32)>,
}

/// A `PMeshInstanceBounds`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bounds {
    /// Index into [`SceneInputs::clusters`], and the object's slot in
    /// the cluster's `PMeshInstanceBounds` list (the file's object order).
    pub cluster: u32,
    pub slot: u32,
    /// Which instance list of the walk it came from (0, 1, ...): the
    /// visibility jobs run per list.
    pub list: u32,
    /// `m_min` (+0), `m_size` (+0x10).
    pub min: [f32; 3],
    pub size: [f32; 3],
    /// `*m_worldMatrix` (+0xc); `None` when the pointer is 0 (identity).
    pub world: Option<Matrix3x4>,
    /// `m_meshInstance` (+0x1c) as an index into
    /// [`SceneInputs::instances`]; `None` when 0 (the renderer skips it).
    pub instance: Option<u32>,
    /// `m_render_order` (+0x20).
    pub render_order: i32,
}

/// A `PMeshInstance`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Instance {
    /// Index into [`SceneInputs::clusters`], and the slot in the
    /// cluster's `PMeshInstance` list (the file's instance order).
    pub cluster: u32,
    pub slot: u32,
    /// `*(+4)`, the instance's world matrix; `None` when 0.
    pub world: Option<Matrix3x4>,
    /// Whether its bounds' `m_worldMatrix` points at the same matrix.
    pub world_shared: bool,
    /// Its mesh's segments (`*(+0)`: count, `+4` the array).
    pub segments: Vec<Segment>,
    /// Its material set (`*(+0x10)`), as indices into
    /// [`SceneInputs::materials`]; `None` for a null entry.
    pub materials: Vec<Option<u32>>,
    /// FFX's fields: `m_groupID` (`+0x48`), `m_DObjKind` (`+0x4c`, 1:
    /// `+100000000` on the bounds' render order at map init),
    /// `m_dObjFlag` (`+0x54`), the object id (`+0x58`).
    pub group_id: u32,
    pub dobj_kind: u32,
    pub dobj_flag: u32,
    pub obj_id: u32,
}

/// A `PMeshSegment` (`0x6c` bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Segment {
    /// `+0`: index into the instance's material set.
    pub material: u32,
    /// `+4`: the joint matrix it is drawn with.
    pub matrix: u32,
    /// `+0x14`: FFX's hidden byte (non-zero: not drawn).
    pub hidden: u8,
    /// `+0x18`: the segment id.
    pub id: u32,
    /// `+0x1c`: `m_segmentRenderOrder` (the bucket, 0-3 drawn).
    pub render_order: u32,
}

/// A `PMaterial`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Material {
    /// The pass remapping (`+8` from, `+0xc` to; empty when 0).
    pub remap_from: String,
    pub remap_to: String,
    /// The live `PParameterBuffer` (`*(+4)`): `size` bytes from its size
    /// word, so a parameter's offset indexes it directly.
    pub parameters: Vec<u8>,
}

/// The tag of the `SceneInputs` message (`Msg::SceneInputs`), for
/// [`write_message`] and readers that walk the raw stream.
pub const MSG_TAG: u32 = 15;

/// Extension blocks after the version 1 fields: `(tag u32, length u32,
/// bytes)` each, to the end of the payload. Readers of version 1 stop
/// before them (`Msg::read` ignores the rest), [`read_extensions`] skips
/// tags it doesn't know, so blocks can be added without a new version.
/// [`Wire`]'s tag.
pub const EXT_WIRE: u32 = 0x5752_0001;

/// What the scene builder needs beyond version 1 (block [`EXT_WIRE`],
/// M4 round 2 m4-wire, `docs/scene-builder.md`, "Inputs from the
/// snapshot").
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Wire {
    /// The camera's world matrix, `*(PCamera +0xb0)` (`None`: the pointer
    /// is 0, identity): `PSceneWideParametersKey_initialize`
    /// (`0x004fe940`) copies it into the key's `+0xb0`, the source of
    /// `ViewInverse` and `EyePosition`.
    pub camera_world: Option<Matrix3x4>,
    /// `Phyre_PRenderInterface` `+0x1340` (`0x00c96240`): the texture
    /// quality the textures were loaded with (0 high, 1 medium, 2 low).
    pub texture_quality: u32,
    /// The anisotropy setting (`0x00c25370`), as its bits.
    pub anisotropy: u32,
    /// Per [`SceneInputs::clusters`] entry: the id of the cluster's first
    /// `PAssetReference` (`+0x18`), up to its `#`: the file it was
    /// loaded from (`PS3Data/map/bsil/bsil00/mdl/bsil00.dae`).
    pub cluster_assets: Vec<String>,
    /// Per [`SceneInputs::materials`] entry: its texture slots.
    pub material_textures: Vec<Vec<MaterialTexture>>,
}

/// A material's texture slot: a tweakable definition of the material's
/// effect variant (`*(PMaterial +0)`, `m_tweakableShaderParameterDefinitions`
/// `+0x20`) of type `0x42` or `0x44` and the texture its 16-byte slot in
/// the parameter buffer holds (`+0xc`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MaterialTexture {
    /// The definition's offset in the parameter buffer, and its type.
    pub offset: u32,
    pub kind: u8,
    /// Whether the slot holds a texture.
    pub set: bool,
    /// The texture's asset id (its `PAssetReference` in the asset tree,
    /// `Phyre_PAssetReference_GetAssetReferenceByAsset` `0x0045cb50`),
    /// e.g. `PS3Data/map/bsil/bsil00/tex/x.dds`; empty for a texture made
    /// at run time (the shadow maps) or not found.
    pub asset: String,
}

impl Wire {
    fn write(&self, w: &mut Writer) {
        write_matrix(w, self.camera_world.as_ref());
        w.u32(self.texture_quality);
        w.u32(self.anisotropy);
        w.u32(len(self.cluster_assets.len()));
        for a in &self.cluster_assets {
            w.str(a);
        }
        w.u32(len(self.material_textures.len()));
        for m in &self.material_textures {
            w.u32(len(m.len()));
            for t in m {
                w.u32(t.offset);
                w.u8(t.kind);
                w.u8(u8::from(t.set));
                w.str(&t.asset);
            }
        }
    }

    fn read(r: &mut Reader) -> Result<Self, DecodeError> {
        let mut out = Self {
            camera_world: read_matrix(r)?,
            ..Self::default()
        };
        out.texture_quality = r.u32()?;
        out.anisotropy = r.u32()?;
        for _ in 0..count(r, 4)? {
            out.cluster_assets.push(r.str()?);
        }
        for _ in 0..count(r, 4)? {
            let mut m = Vec::new();
            for _ in 0..count(r, 10)? {
                m.push(MaterialTexture {
                    offset: r.u32()?,
                    kind: r.u8()?,
                    set: r.u8()? != 0,
                    asset: r.str()?,
                });
            }
            out.material_textures.push(m);
        }
        Ok(out)
    }
}

/// Writes a whole `SceneInputs` message (tag, length, payload) with the
/// [`Wire`] block after the version 1 fields.
pub fn write_message(s: &SceneInputs, wire: Option<&Wire>, out: &mut Vec<u8>) {
    let mut w = Writer::default();
    s.write(&mut w);
    if let Some(x) = wire {
        // The character part's flag comes first (a record without the
        // part ends at its materials, so a block would read as one).
        if s.chars.is_none() {
            w.u8(0);
        }
        let mut block = Writer::default();
        x.write(&mut block);
        w.u32(EXT_WIRE);
        w.blob(&block.bytes);
    }
    out.extend_from_slice(&MSG_TAG.to_le_bytes());
    out.extend_from_slice(&len(w.bytes.len()).to_le_bytes());
    out.extend_from_slice(&w.bytes);
}

/// Reads a `SceneInputs` payload with its extension blocks: the record
/// and its [`Wire`] block, when it has one.
///
/// # Errors
/// On a malformed record or block.
pub fn read_extensions(payload: &[u8]) -> Result<(SceneInputs, Option<Wire>), DecodeError> {
    let mut r = Reader::new(payload);
    let s = SceneInputs::read(&mut r)?;
    let mut wire = None;
    while r.at + 8 <= r.bytes.len() {
        let tag = r.u32()?;
        let block = r.blob()?;
        if tag == EXT_WIRE {
            wire = Some(Wire::read(&mut Reader::new(&block))?);
        }
    }
    Ok((s, wire))
}

fn write_opt_u32(w: &mut Writer, v: Option<u32>) {
    match v {
        Some(v) => {
            w.u8(1);
            w.u32(v);
        }
        None => w.u8(0),
    }
}

fn read_opt_u32(r: &mut Reader) -> Result<Option<u32>, DecodeError> {
    Ok(if r.u8()? == 0 { None } else { Some(r.u32()?) })
}

fn write_floats(w: &mut Writer, v: &[f32]) {
    for &x in v {
        w.u32(x.to_bits());
    }
}

fn read_floats<const N: usize>(r: &mut Reader) -> Result<[f32; N], DecodeError> {
    let mut out = [0.0; N];
    for x in &mut out {
        *x = f32::from_bits(r.u32()?);
    }
    Ok(out)
}

fn write_matrix(w: &mut Writer, m: Option<&Matrix3x4>) {
    match m {
        Some(m) => {
            w.u8(1);
            write_floats(w, m);
        }
        None => w.u8(0),
    }
}

fn read_matrix(r: &mut Reader) -> Result<Option<Matrix3x4>, DecodeError> {
    Ok(if r.u8()? == 0 {
        None
    } else {
        Some(read_floats(r)?)
    })
}

fn len(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A list count, refused when it cannot fit in what is left (a corrupt
/// count must not allocate gigabytes).
fn count(r: &mut Reader, min_item: usize) -> Result<usize, DecodeError> {
    let n = r.u32()? as usize;
    if n.saturating_mul(min_item.max(1)) > r.bytes.len().saturating_sub(r.at) {
        return Err(DecodeError("scene inputs: count past the end"));
    }
    Ok(n)
}

impl SceneInputs {
    /// Appends the payload.
    pub fn write(&self, w: &mut Writer) {
        w.u32(VERSION);
        w.u32(self.frame);
        w.u32(self.update);
        w.u32(self.thread);
        w.u32(self.world);
        w.str(&self.pass);
        w.i32(self.dir);
        w.blob(&self.camera);
        w.blob(&self.app);
        w.u32s(&self.screen);
        w.u32(self.group_mask);
        w.u32(self.shadow_solution);
        w.u8(self.alpha_fast);
        w.u32(self.layer_group);
        w.u32(len(self.layers.len()));
        for l in &self.layers {
            w.u8(l.shown);
            w.u8(l.group.cast_unsigned());
            w.u32(u32::from(l.id.cast_unsigned()));
            w.u8(u8::from(l.has_mesh));
            match l.mesh {
                Some((c, slot)) => {
                    w.u8(1);
                    w.u32(c);
                    w.u32(slot);
                }
                None => w.u8(0),
            }
        }
        w.u32(self.depth_bias_count);
        w.u32(len(self.depth_bias.len()));
        for d in &self.depth_bias {
            write_opt_u32(w, d.bounds);
            w.u32(d.scale.to_bits());
            w.u32(d.bias.to_bits());
        }
        w.u32(len(self.clusters.len()));
        for c in &self.clusters {
            w.u32(c.index);
            w.str(&c.name);
            w.u32(len(c.lists.len()));
            for (name, n) in &c.lists {
                w.str(name);
                w.u32(*n);
            }
        }
        w.u32(len(self.bounds.len()));
        for b in &self.bounds {
            w.u32(b.cluster);
            w.u32(b.slot);
            w.u32(b.list);
            write_floats(w, &b.min);
            write_floats(w, &b.size);
            write_matrix(w, b.world.as_ref());
            write_opt_u32(w, b.instance);
            w.i32(b.render_order);
        }
        w.u32(len(self.instances.len()));
        for i in &self.instances {
            w.u32(i.cluster);
            w.u32(i.slot);
            write_matrix(w, i.world.as_ref());
            w.u8(u8::from(i.world_shared));
            w.u32(len(i.segments.len()));
            for s in &i.segments {
                w.u32(s.material);
                w.u32(s.matrix);
                w.u8(s.hidden);
                w.u32(s.id);
                w.u32(s.render_order);
            }
            w.u32(len(i.materials.len()));
            for m in &i.materials {
                write_opt_u32(w, *m);
            }
            w.u32s(&[i.group_id, i.dobj_kind, i.dobj_flag, i.obj_id]);
        }
        w.u32(len(self.materials.len()));
        for m in &self.materials {
            w.str(&m.remap_from);
            w.str(&m.remap_to);
            w.blob(&m.parameters);
        }
        if let Some(c) = &self.chars {
            w.u8(1);
            c.write(w);
        }
    }

    /// Reads a payload.
    ///
    /// # Errors
    /// On another version or truncated bytes.
    pub fn read(r: &mut Reader) -> Result<Self, DecodeError> {
        if r.u32()? != VERSION {
            return Err(DecodeError("scene inputs: unknown version"));
        }
        let mut s = Self {
            frame: r.u32()?,
            update: r.u32()?,
            thread: r.u32()?,
            world: r.u32()?,
            pass: r.str()?,
            dir: r.i32()?,
            camera: r.blob()?,
            app: r.blob()?,
            screen: r.u32s()?,
            group_mask: r.u32()?,
            shadow_solution: r.u32()?,
            alpha_fast: r.u8()?,
            layer_group: r.u32()?,
            ..Self::default()
        };
        for _ in 0..count(r, 8)? {
            s.layers.push(Layer::read(r)?);
        }
        s.depth_bias_count = r.u32()?;
        for _ in 0..count(r, 9)? {
            s.depth_bias.push(DepthBias {
                bounds: read_opt_u32(r)?,
                scale: f32::from_bits(r.u32()?),
                bias: f32::from_bits(r.u32()?),
            });
        }
        for _ in 0..count(r, 12)? {
            let index = r.u32()?;
            let name = r.str()?;
            let mut lists = Vec::new();
            for _ in 0..count(r, 8)? {
                lists.push((r.str()?, r.u32()?));
            }
            s.clusters.push(Cluster { index, name, lists });
        }
        for _ in 0..count(r, 42)? {
            s.bounds.push(Bounds {
                cluster: r.u32()?,
                slot: r.u32()?,
                list: r.u32()?,
                min: read_floats(r)?,
                size: read_floats(r)?,
                world: read_matrix(r)?,
                instance: read_opt_u32(r)?,
                render_order: r.i32()?,
            });
        }
        for _ in 0..count(r, 34)? {
            let cluster = r.u32()?;
            let slot = r.u32()?;
            let world = read_matrix(r)?;
            let world_shared = r.u8()? != 0;
            let mut segments = Vec::new();
            for _ in 0..count(r, 17)? {
                segments.push(Segment {
                    material: r.u32()?,
                    matrix: r.u32()?,
                    hidden: r.u8()?,
                    id: r.u32()?,
                    render_order: r.u32()?,
                });
            }
            let mut materials = Vec::new();
            for _ in 0..count(r, 1)? {
                materials.push(read_opt_u32(r)?);
            }
            let [group_id, dobj_kind, dobj_flag, obj_id] = r.u32s()?;
            s.instances.push(Instance {
                cluster,
                slot,
                world,
                world_shared,
                segments,
                materials,
                group_id,
                dobj_kind,
                dobj_flag,
                obj_id,
            });
        }
        for _ in 0..count(r, 12)? {
            s.materials.push(Material {
                remap_from: r.str()?,
                remap_to: r.str()?,
                parameters: r.blob()?,
            });
        }
        // The character part, after everything else (older records end
        // here).
        if !r.done() && r.u8()? != 0 {
            s.chars = Some(Box::new(CharWorld::read(r)?));
        }
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(s: &SceneInputs) -> SceneInputs {
        let mut w = Writer::default();
        s.write(&mut w);
        let mut r = Reader::new(&w.bytes);
        let back = SceneInputs::read(&mut r).expect("read");
        assert!(r.done(), "trailing bytes");
        back
    }

    #[test]
    fn the_character_part_round_trips() {
        let mut s = SceneInputs {
            world: 0x7c,
            pass: "Opaque".into(),
            ..SceneInputs::default()
        };
        // Without the part, the bytes are those of a record written
        // before it existed: nothing follows the materials.
        assert_eq!(round_trip(&s), s);
        s.chars = Some(Box::new(CharWorld {
            instances: vec![CharInstance {
                flags: 1,
                joints: vec![[1.5; 16], [2.0; 16]],
                joint_parent: vec![-1, 0],
                skeleton: vec![[-1.0; 16]],
                skin_bones: vec![vec![[1, 2], [0xffff, 7]], vec![]],
            }],
            cloth: vec![Cloth {
                instance: 0,
                segment: 6,
                slot: 2,
                list_index: 0,
                matrix: [0.5; 16],
                count: 1,
                vertices: vec![9; 16],
            }],
            dyn_geo: Some(DynGeo {
                id: 1,
                vertices: 4,
                indices: 6,
                arrays: [vec![1; 48], vec![2; 48], vec![3; 64], vec![4; 32]],
                index_bytes: vec![0, 0, 2, 0, 1, 0],
            }),
            renderer: vec![[1.0; 16], [2.0; 16]],
        }));
        assert_eq!(round_trip(&s), s);
    }

    #[test]
    fn the_wire_block_follows_version_1() {
        let s = SceneInputs {
            pass: "OpaqueNoMSAA".into(),
            dir: 1,
            materials: vec![Material::default()],
            ..SceneInputs::default()
        };
        let wire = Wire {
            camera_world: Some([2.0; 12]),
            texture_quality: 2,
            anisotropy: 1f32.to_bits(),
            cluster_assets: vec!["PS3Data/map/cdsp/cdsp00/mdl/cdsp00.dae".into()],
            material_textures: vec![vec![MaterialTexture {
                offset: 0x40,
                kind: 0x42,
                set: true,
                asset: "PS3Data/x.dds".into(),
            }]],
        };
        let mut out = Vec::new();
        write_message(&s, Some(&wire), &mut out);
        // The tag is the one `Msg` writes, and `Msg` still reads it.
        let mut plain = Vec::new();
        crate::Msg::SceneInputs(Box::new(s.clone())).write(&mut plain);
        assert_eq!(out[..4], plain[..4]);
        let msgs = crate::read_all(&out).expect("messages");
        assert!(matches!(&msgs[..], [crate::Msg::SceneInputs(x)] if **x == s));
        let (back, w) = read_extensions(&out[8..]).expect("extensions");
        assert_eq!((back, w), (s.clone(), Some(wire)));
        // A version 1 payload has no block.
        assert_eq!(read_extensions(&plain[8..]).expect("v1").1, None);
    }
}
