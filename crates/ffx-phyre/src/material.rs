//! Materials and the textures they sample, the node hierarchy and the
//! mesh instances, resolved through the fixups.
//!
//! The path to a texture is:
//!
//! 1. `PMesh.m_defaultMaterials` is a pointer array of `PMaterial`. For
//!    pointer arrays, fixup word 5 is the element index, and a segment's
//!    `m_materialIndex` indexes this array.
//! 2. `PMaterial.m_parameterBuffer` points to a `PParameterBuffer`, which
//!    sits in its own instance list.
//! 3. `PParameterBuffer.m_tweakableShaderParameterDefinitions` points to
//!    its `PShaderParameterDefinition`s. A definition has a name (a
//!    `PString` array), `m_parameterType` (`0x42` = texture sampler) and
//!    `m_bufferLoc` (size << 16 | offset).
//! 4. A sampler's texture is a user fixup at buffer object + 8 + offset + 4.
//!    The sampler state sits at + 8 + offset.
//! 5. That user fixup is an asset import reference: two bytes, a
//!    big-endian index into the `PAssetReferenceImport` list. The import's
//!    `m_id` is the texture path, e.g. `PS3Data/chr/mon/m009/tex/m009.dds`.

use crate::cluster::Cluster;
use crate::fixup::{ArrayFixup, PointerFixup};
use crate::{Error, Result};

/// A texture a material samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialTexture {
    /// The shader parameter, e.g. `TextureSampler` or `SpecularMapSampler`.
    pub sampler: String,
    /// The import path as stored, e.g. `PS3Data/chr/mon/m009/tex/m009.dds`.
    pub path: String,
}

impl MaterialTexture {
    /// The texture's path in `FFX_Data.vbf`, e.g.
    /// `ffx_data/gamedata/ps3data/chr/mon/m009/tex/d3d11/m009.dds.phyre`.
    #[must_use]
    pub fn archive_path(&self) -> String {
        let lower = self.path.to_ascii_lowercase();
        let (dir, file) = lower.rsplit_once('/').unwrap_or(("", &lower));
        format!("ffx_data/gamedata/{dir}/d3d11/{file}.phyre")
    }
}

/// A shader parameter value of a material (`m_parameterType` 0x40).
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// E.g. `MaterialColour`, `AlphaThreshold`.
    pub name: String,
    /// `m_dataType`: 0 float, 1 float2, 2 float3, 3 float4, 0x31 float4x4.
    pub data_type: u8,
    pub values: Vec<f32>,
}

/// A material: the textures it samples and its parameter values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Material {
    pub textures: Vec<MaterialTexture>,
    pub parameters: Vec<Parameter>,
}

impl Material {
    /// The base color texture: the `TextureSampler` parameter.
    #[must_use]
    pub fn base_color(&self) -> Option<&MaterialTexture> {
        self.textures.iter().find(|t| t.sampler == "TextureSampler")
    }

    /// A texture by sampler name.
    #[must_use]
    pub fn texture(&self, sampler: &str) -> Option<&MaterialTexture> {
        self.textures.iter().find(|t| t.sampler == sampler)
    }

    /// A parameter's values by name.
    #[must_use]
    pub fn parameter(&self, name: &str) -> Option<&[f32]> {
        self.parameters
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.values.as_slice())
    }
}

const VALUE: u8 = 0x40;

const SAMPLER: u8 = 0x42;

/// Fixup and array lookups over one parsed cluster.
pub(crate) struct Resolver<'a> {
    pub c: &'a Cluster,
    pub bytes: &'a [u8],
    pub pointers: &'a [Vec<PointerFixup>],
    pub arrays: &'a [Vec<ArrayFixup>],
}

impl Resolver<'_> {
    /// Whether a fixup's member word names `class.member`, in either form
    /// (byte offset with bit 31, or namespace member index). `inner` is
    /// added for fields inside embedded members (`m_els` of a `PArray`).
    fn is_field(&self, word: u32, class: &str, member: &str, inner: u32) -> bool {
        if word & 0x8000_0000 != 0 {
            return self
                .c
                .member_offset(class, member)
                .is_some_and(|o| o + inner == word & 0x7fff_ffff);
        }
        inner == 0
            && self
                .c
                .member_by_index(word)
                .is_some_and(|(_, m)| m.name == member)
    }

    /// The pointer fixups of `list` for object `obj` and the given field.
    fn pointers_of<'s>(
        &'s self,
        list: usize,
        obj: u32,
        class: &'s str,
        member: &'s str,
        inner: u32,
    ) -> impl Iterator<Item = &'s PointerFixup> + 's {
        self.pointers[list]
            .iter()
            .filter(move |f| f.object() == obj && self.is_field(f.member(), class, member, inner))
    }

    /// A NUL-terminated string held in a list's arrays, for the array
    /// fixup of `obj`'s `member`.
    /// Falls back to the object's only array fixup: `PNode.m_name` (+0x50)
    /// is recorded at +0x4c, for reasons not known yet.
    fn array_string(&self, list: usize, obj: u32, class: &str, member: &str) -> Option<String> {
        let mut of_object = self.arrays[list].iter().filter(|f| f.object() == obj);
        let f = of_object
            .clone()
            .find(|f| self.is_field(f.member(), class, member, 0))
            .or_else(|| of_object.next().filter(|_| of_object.next().is_none()))?;
        let range = self.c.lists[list].arrays();
        let tail = self.bytes.get(range.start + f.0[3] as usize..range.end)?;
        let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
        Some(String::from_utf8_lossy(&tail[..end]).into_owned())
    }

    fn list_of(&self, class: &str) -> Option<usize> {
        self.c
            .lists
            .iter()
            .position(|l| self.c.classes[l.class].name == class)
    }

    /// The import paths, by import index.
    fn imports(&self) -> Vec<Option<String>> {
        let Some(list) = self.list_of("PAssetReferenceImport") else {
            return Vec::new();
        };
        (0..self.c.lists[list].count)
            .map(|i| self.array_string(list, i, "PAssetReferenceImport", "m_id"))
            .collect()
    }

    /// One `PMaterial` object's textures.
    fn material(&self, list: usize, obj: u32, imports: &[Option<String>]) -> Result<Material> {
        let c = self.c;
        let Some(buffer) = self
            .pointers_of(list, obj, "PMaterial", "m_parameterBuffer", 0)
            .next()
        else {
            return Ok(Material::default());
        };
        let (bl, bo) = (buffer.target_list() as usize, buffer.target());
        if bl >= c.lists.len() {
            return Err(Error::Malformed("parameter buffer list"));
        }
        let Some(defs) = self
            .pointers_of(
                bl,
                bo,
                "PParameterBuffer",
                "m_tweakableShaderParameterDefinitions",
                4,
            )
            .next()
        else {
            return Ok(Material::default());
        };
        let (dl, d0, dn) = (defs.target_list() as usize, defs.target(), defs.count());
        let def_list = c
            .lists
            .get(dl)
            .ok_or(Error::Malformed("parameter definition list"))?;
        let need = |member: &str| {
            c.member_offset("PShaderParameterDefinition", member)
                .map(|o| o as usize)
                .ok_or_else(|| {
                    Error::Unsupported(format!("no PShaderParameterDefinition.{member}"))
                })
        };
        let (o_type, o_loc, o_data_type) = (
            need("m_parameterType")?,
            need("m_bufferLoc")?,
            need("m_dataType")?,
        );
        // Values sit at their buffer location from the object's start (samplers
        // 8 bytes further, see below).
        let buffer_at = c.lists[bl].object(bo);
        let mut textures = Vec::new();
        let mut parameters = Vec::new();
        for d in d0..d0 + dn {
            let at = def_list.object(d);
            if self.bytes.get(at + o_type) == Some(&VALUE) {
                let loc = crate::model::u32_at(self.bytes, at + o_loc)? as usize;
                let (offset, size) = (loc & 0xffff, loc >> 16);
                let values = (0..size / 4)
                    .map(|k| {
                        crate::model::u32_at(self.bytes, buffer_at + offset + 4 * k)
                            .map(f32::from_bits)
                    })
                    .collect::<Result<Vec<_>>>()?;
                parameters.push(Parameter {
                    name: self
                        .array_string(dl, d, "PShaderParameterDefinition", "m_name")
                        .unwrap_or_default(),
                    data_type: self.bytes.get(at + o_data_type).copied().unwrap_or(0),
                    values,
                });
                continue;
            }
            if self.bytes.get(at + o_type) != Some(&SAMPLER) {
                continue;
            }
            let loc = crate::model::u32_at(self.bytes, at + o_loc)? & 0xffff;
            let slot = 0x8000_0000 | (8 + loc + 4);
            let Some(u) = self.pointers[bl]
                .iter()
                .find(|f| f.object() == bo && f.member() == slot)
                .and_then(PointerFixup::user_fixup)
            else {
                continue;
            };
            let data = &c
                .user_fixups
                .get(u as usize)
                .ok_or(Error::Malformed("user fixup index"))?
                .data;
            let [hi, lo] = data[..] else {
                return Err(Error::Malformed("texture reference is not an import index"));
            };
            let path = imports
                .get(usize::from(u16::from_be_bytes([hi, lo])))
                .cloned()
                .flatten()
                .ok_or(Error::Malformed("texture import index"))?;
            let sampler = self
                .array_string(dl, d, "PShaderParameterDefinition", "m_name")
                .unwrap_or_default();
            textures.push(MaterialTexture { sampler, path });
        }
        Ok(Material {
            textures,
            parameters,
        })
    }

    /// The pointer fixups of `list`/`obj` for the field at a byte offset.
    fn pointers_at(
        &self,
        list: usize,
        obj: u32,
        offset: u32,
    ) -> impl Iterator<Item = &PointerFixup> + '_ {
        self.pointers[list]
            .iter()
            .filter(move |f| f.object() == obj && f.member() == 0x8000_0000 | offset)
    }

    fn need(&self, class: &str, member: &str) -> Result<u32> {
        self.c
            .member_offset(class, member)
            .ok_or_else(|| Error::Unsupported(format!("no {class}.{member}")))
    }

    /// A segment's bone palette (`m_skinBones` → `PSkinBoneRemap`s).
    fn bones(&self, segments: usize, s: u32) -> Result<Vec<Bone>> {
        let Some(f) = self
            .pointers_of(segments, s, "PMeshSegment", "m_skinBones", 4)
            .next()
        else {
            return Ok(Vec::new());
        };
        let list = self
            .c
            .lists
            .get(f.target_list() as usize)
            .ok_or(Error::Malformed("bone list"))?;
        let (o_h, o_s) = (
            self.need("PSkinBoneRemap", "m_hierarchyMatrixIndex")? as usize,
            self.need("PSkinBoneRemap", "m_skeletonMatrixIndex")? as usize,
        );
        let u16_at = |o: usize| {
            self.bytes
                .get(o..o + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .ok_or(Error::Malformed("truncated bone"))
        };
        (f.target()..f.target() + f.count())
            .map(|b| {
                let at = list.object(b);
                Ok(Bone {
                    hierarchy: u16_at(at + o_h)?,
                    skeleton: u16_at(at + o_s)?,
                })
            })
            .collect()
    }

    /// A mesh's matrix names and skeleton matrices.
    fn skeleton(&self, meshes: usize, m: u32) -> Result<Skeleton> {
        let mut skeleton = Skeleton::default();
        if let Some(f) = self
            .pointers_of(meshes, m, "PMesh", "m_skeletonMatrices", 4)
            .next()
        {
            let list = self
                .c
                .lists
                .get(f.target_list() as usize)
                .ok_or(Error::Malformed("matrix list"))?;
            for k in f.target()..f.target() + f.count() {
                skeleton.inverse_bind.push(self.matrix(list.object(k))?);
            }
        }
        if let Some(f) = self
            .pointers_of(meshes, m, "PMesh", "m_matrixNames", 4)
            .next()
        {
            let list = f.target_list() as usize;
            for k in f.target()..f.target() + f.count() {
                skeleton.names.push(
                    self.array_string(list, k, "PString", "m_buffer")
                        .unwrap_or_default(),
                );
            }
        }
        Ok(skeleton)
    }

    fn matrix(&self, at: usize) -> Result<[f32; 16]> {
        let mut m = [0f32; 16];
        for (k, v) in m.iter_mut().enumerate() {
            *v = f32::from_bits(crate::model::u32_at(self.bytes, at + 4 * k)?);
        }
        Ok(m)
    }

    /// The `PMeshInstanceBounds` whose `m_meshInstance` is object `obj` of
    /// the `PMeshInstance` list `instances`.
    /// Also gives the file offsets of `m_min` and `m_size`.
    fn instance_bounds(&self, instances: usize, obj: u32) -> Result<Option<(Bounds, [usize; 2])>> {
        let c = self.c;
        let Some(list) = self.list_of("PMeshInstanceBounds") else {
            return Ok(None);
        };
        let field = |m: &str| {
            c.member_offset("PMeshInstanceBounds", m)
                .map(|o| o as usize)
        };
        let (Some(o_min), Some(o_size), Some(o_order)) =
            (field("m_min"), field("m_size"), field("m_render_order"))
        else {
            return Ok(None);
        };
        let Some(f) = self.pointers[list].iter().find(|f| {
            f.target_list() as usize == instances
                && f.target() == obj
                && self.is_field(f.member(), "PMeshInstanceBounds", "m_meshInstance", 0)
        }) else {
            return Ok(None);
        };
        let at = c.lists[list].object(f.object());
        let v3 = |o: usize| -> Result<[f32; 3]> {
            let mut v = [0f32; 3];
            for (k, x) in v.iter_mut().enumerate() {
                *x = f32::from_bits(crate::model::u32_at(self.bytes, at + o + 4 * k)?);
            }
            Ok(v)
        };
        Ok(Some((
            Bounds {
                min: v3(o_min)?,
                size: v3(o_size)?,
                render_order: crate::model::u32_at(self.bytes, at + o_order)?.cast_signed(),
            },
            [at + o_min, at + o_size],
        )))
    }

    /// Resolves the scene: materials, default materials per segment, the
    /// node hierarchy and the mesh instances.
    #[allow(clippy::too_many_lines)] // materials, nodes and instances share the lookups
    pub fn scene(&self) -> Result<Scene> {
        let c = self.c;
        let mut scene = Scene::default();
        let (Some(meshes), Some(segments)) = (self.list_of("PMesh"), self.list_of("PMeshSegment"))
        else {
            return Ok(scene);
        };
        let imports = self.imports();
        let o_index = self.need("PMeshSegment", "m_materialIndex")? as usize;
        let o_default = self.need("PMesh", "m_defaultMaterials")?;
        let mut known: Vec<((usize, u32), usize)> = Vec::new();

        // A material set (`PMaterialSet` at a byte offset of an object):
        // element index → material.
        let mut material_set = |scene: &mut Scene,
                                list: usize,
                                obj: u32,
                                offset: u32|
         -> Result<Vec<(usize, usize)>> {
            let mut set = Vec::new();
            for f in self.pointers_at(list, obj, offset + 4) {
                let key = (f.target_list() as usize, f.target());
                let index = if let Some(&(_, i)) = known.iter().find(|(k, _)| *k == key) {
                    i
                } else {
                    scene.materials.push(self.material(key.0, key.1, &imports)?);
                    known.push((key, scene.materials.len() - 1));
                    scene.materials.len() - 1
                };
                set.push((f.count() as usize, index));
            }
            Ok(set)
        };
        // A mesh's segments, each with the material its index selects.
        let mesh_segments = |mesh: u32,
                             set: &[(usize, usize)]|
         -> Result<Vec<(usize, Option<usize>)>> {
            let Some(range) = self
                .pointers_of(meshes, mesh, "PMesh", "m_meshSegments", 4)
                .next()
            else {
                return Ok(Vec::new());
            };
            if range.target_list() as usize != segments {
                return Ok(Vec::new());
            }
            (range.target()..range.target() + range.count())
                .map(|s| {
                    let k = crate::model::u32_at(self.bytes, c.lists[segments].object(s) + o_index)?
                        as usize;
                    Ok((
                        s as usize,
                        set.iter().find(|(e, _)| *e == k).map(|&(_, i)| i),
                    ))
                })
                .collect()
        };

        scene.segment_materials = vec![None; c.lists[segments].count as usize];
        scene.segment_bones = (0..c.lists[segments].count)
            .map(|s| self.bones(segments, s))
            .collect::<Result<_>>()?;
        for m in 0..c.lists[meshes].count {
            scene.skeletons.push(self.skeleton(meshes, m)?);
            let set = material_set(&mut scene, meshes, m, o_default)?;
            for (s, material) in mesh_segments(m, &set)? {
                if let Some(slot) = scene.segment_materials.get_mut(s) {
                    *slot = material;
                }
            }
        }

        // Nodes, linked to instances through their shared world matrix.
        let mut node_worlds = Vec::new();
        if let Some(nodes) = self.list_of("PNode") {
            let o_local = self.need("PNode", "m_localMatrix")? as usize;
            for n in 0..c.lists[nodes].count {
                let parent = self
                    .pointers_of(nodes, n, "PNode", "m_parent", 0)
                    .next()
                    .map(|f| f.target() as usize);
                let world = self
                    .pointers_of(nodes, n, "PNode", "m_worldMatrix", 0)
                    .next()
                    .map(|f| (f.target_list(), f.target()));
                node_worlds.push(world);
                scene.nodes.push(Node {
                    name: self
                        .array_string(nodes, n, "PNode", "m_name")
                        .unwrap_or_default(),
                    parent,
                    local: self.matrix(c.lists[nodes].object(n) + o_local)?,
                });
            }
        }
        if let Some(instances) = self.list_of("PMeshInstance") {
            for i in 0..c.lists[instances].count {
                let Some(mesh) = self
                    .pointers_of(instances, i, "PMeshInstance", "m_mesh", 0)
                    .next()
                else {
                    continue;
                };
                let set = match self
                    .pointers_of(instances, i, "PMeshInstance", "m_materialSet", 0)
                    .next()
                {
                    Some(f) => material_set(
                        &mut scene,
                        f.target_list() as usize,
                        f.target(),
                        f.target_offset(),
                    )?,
                    None => Vec::new(),
                };
                let world = self
                    .pointers_of(instances, i, "PMeshInstance", "m_localToWorldMatrix", 0)
                    .next()
                    .map(|f| (f.target_list(), f.target()));
                let bounds = self.instance_bounds(instances, i)?;
                scene.bounds_at.push(bounds.map(|b| b.1));
                scene.instances.push(Instance {
                    bounds: bounds.map(|b| b.0),
                    name: self
                        .array_string(instances, i, "PMeshInstance", "m_name")
                        .unwrap_or_default(),
                    node: world.and_then(|w| node_worlds.iter().position(|&n| n == Some(w))),
                    segments: mesh_segments(mesh.target(), &set)?,
                    skeleton: Some(mesh.target() as usize).filter(|&m| {
                        scene
                            .skeletons
                            .get(m)
                            .is_some_and(|s| !s.inverse_bind.is_empty())
                    }),
                });
            }
        }
        Ok(scene)
    }
}

/// A transform node (`PNode`).
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: String,
    /// Index into [`crate::Model::nodes`].
    pub parent: Option<usize>,
    /// Local transform, column-major (translation in elements 12–14).
    pub local: [f32; 16],
}

/// A placed mesh (`PMeshInstance`).
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub name: String,
    /// The node it is attached to (the one sharing its world matrix).
    pub node: Option<usize>,
    /// Its mesh's segments (indices into [`crate::Model::segments`]), each
    /// with the material the instance's material set gives it.
    pub segments: Vec<(usize, Option<usize>)>,
    /// Its mesh's skeleton, an index into [`crate::Model::skeletons`].
    pub skeleton: Option<usize>,
    /// Its `PMeshInstanceBounds` (what the renderer culls and sorts by,
    /// `docs/formats/scene.md`).
    pub bounds: Option<Bounds>,
}

/// A `PMeshInstanceBounds`: the box in the instance's model space and its
/// sort class.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// `m_min`, `m_size`.
    pub min: [f32; 3],
    pub size: [f32; 3],
    /// `m_render_order`.
    pub render_order: i32,
}

/// A mesh's skeleton: the matrices its segments are skinned to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Skeleton {
    /// Hierarchy matrix names (`PMesh.m_matrixNames`); they name `PNode`s.
    pub names: Vec<String>,
    /// Skeleton matrices (`PMesh.m_skeletonMatrices`), column-major: the
    /// inverse bind matrices of the skinned joints.
    pub inverse_bind: Vec<[f32; 16]>,
}

/// One entry of a segment's bone palette (`PSkinBoneRemap`): a
/// `SkinIndices` value `i` selects `bones[i]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bone {
    /// Index into [`Skeleton::names`].
    pub hierarchy: u16,
    /// Index into [`Skeleton::inverse_bind`].
    pub skeleton: u16,
}

#[derive(Debug, Default)]
pub(crate) struct Scene {
    pub materials: Vec<Material>,
    pub segment_materials: Vec<Option<usize>>,
    pub segment_bones: Vec<Vec<Bone>>,
    pub skeletons: Vec<Skeleton>,
    pub nodes: Vec<Node>,
    pub instances: Vec<Instance>,
    /// Per instance, the file offsets of its bounds' `m_min` and `m_size`.
    pub bounds_at: Vec<Option<[usize; 2]>>,
}
