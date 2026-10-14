//! Reading edited geometry back from binary glTF made by
//! [`Model::to_glb_with`] (or an editor that kept its `extras`).
//!
//! Each exported primitive carries `extras.ffx_segment`, its mesh
//! `extras.ffx_segments` (one per primitive, in order: editors such as
//! Blender keep mesh extras but not primitive ones). A segment takes the
//! first primitive naming it. Positions, normals, texture coordinates
//! (flipped back to v up), colors, joints and weights are read; joints are
//! mapped through the primitive's skin, by joint name, back into the
//! segment's bone palette. Values equal to what the export wrote keep
//! their stored form, so an unedited glTF changes nothing.

use std::collections::HashMap;

use serde_json::Value;

use crate::edit::ClusterEdit;
use crate::gltf::{export_normal, export_skin, export_uv};
use crate::model::{Model, Segment};
use crate::{Error, Result};

/// A JSON number as an index or size.
fn index(v: &Value) -> Option<usize> {
    v.as_u64().and_then(|x| usize::try_from(x).ok())
}

fn bad(msg: impl Into<String>) -> Error {
    Error::Unsupported(format!("glTF: {}", msg.into()))
}

/// The JSON and binary chunks of a `.glb`.
fn chunks(glb: &[u8]) -> Result<(Value, &[u8])> {
    let word = |o: usize| {
        glb.get(o..o + 4)
            .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")) as usize)
            .ok_or_else(|| bad("truncated"))
    };
    if glb.get(..4) != Some(b"glTF") || word(4)? != 2 {
        return Err(bad("not a glTF 2 binary (.glb)"));
    }
    let json_len = word(12)?;
    if glb.get(16..20) != Some(b"JSON") {
        return Err(bad("first chunk is not JSON"));
    }
    let json: Value =
        serde_json::from_slice(glb.get(20..20 + json_len).ok_or_else(|| bad("truncated"))?)
            .map_err(|e| bad(e.to_string()))?;
    let at = 20 + json_len;
    let bin = if glb.len() >= at + 8 && &glb[at + 4..at + 8] == b"BIN\0" {
        let len = word(at)?;
        glb.get(at + 8..at + 8 + len)
            .ok_or_else(|| bad("truncated BIN chunk"))?
    } else {
        &[]
    };
    Ok((json, bin))
}

struct Doc<'a> {
    json: Value,
    bin: &'a [u8],
}

impl Doc<'_> {
    /// An accessor's elements as `N` floats each: floats as they are,
    /// normalized integers scaled to 0–1, other integers converted.
    #[allow(clippy::cast_precision_loss)] // indices and joints are small
    fn floats<const N: usize>(&self, at: &Value) -> Result<Vec<[f32; N]>> {
        let i = index(at).ok_or_else(|| bad("accessor index"))?;
        let acc = &self.json["accessors"][i];
        let count = index(&acc["count"]).ok_or_else(|| bad("accessor count"))?;
        let components = match acc["type"].as_str() {
            Some("SCALAR") => 1,
            Some("VEC2") => 2,
            Some("VEC3") => 3,
            Some("VEC4") => 4,
            t => return Err(bad(format!("accessor type {t:?}"))),
        };
        if components < N {
            return Err(bad(format!(
                "accessor {i} has {components} components, {N} needed"
            )));
        }
        let ctype = acc["componentType"].as_u64().unwrap_or(0);
        let size = match ctype {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            5125 | 5126 => 4,
            _ => return Err(bad(format!("component type {ctype}"))),
        };
        let normalized = acc["normalized"].as_bool().unwrap_or(false);
        let Some(view) = index(&acc["bufferView"]) else {
            return Ok(vec![[0.0; N]; count]); // all zeros
        };
        let view = &self.json["bufferViews"][view];
        if view["buffer"].as_u64() != Some(0) {
            return Err(bad("data outside the binary chunk"));
        }
        let start =
            index(&view["byteOffset"]).unwrap_or(0) + index(&acc["byteOffset"]).unwrap_or(0);
        let stride = index(&view["byteStride"]).unwrap_or(components * size);
        let mut out = Vec::with_capacity(count);
        for e in 0..count {
            let mut v = [0f32; N];
            for (k, x) in v.iter_mut().enumerate() {
                let o = start + e * stride + k * size;
                let b = self
                    .bin
                    .get(o..o + size)
                    .ok_or_else(|| bad("accessor outside the buffer"))?;
                *x = match (ctype, normalized) {
                    (5126, _) => f32::from_le_bytes(b.try_into().expect("4 bytes")),
                    (5121, true) => f32::from(b[0]) / 255.0,
                    (5123, true) => f32::from(u16::from_le_bytes([b[0], b[1]])) / 65535.0,
                    (5120, true) => (f32::from(b[0].cast_signed()) / 127.0).max(-1.0),
                    (5122, true) => {
                        (f32::from(i16::from_le_bytes([b[0], b[1]])) / 32767.0).max(-1.0)
                    }
                    (5121, false) => f32::from(b[0]),
                    (5123, false) => f32::from(u16::from_le_bytes([b[0], b[1]])),
                    (5125, _) => u32::from_le_bytes(b.try_into().expect("4 bytes")) as f32,
                    _ => return Err(bad(format!("component type {ctype} unnormalized"))),
                };
            }
            out.push(v);
        }
        Ok(out)
    }
}

/// Keeps `old` where the export of `old` equals the imported value.
fn keep<T: Copy, E: PartialEq>(new: &mut [T], old: &[T], imported: &[E], export: impl Fn(T) -> E) {
    if new.len() == old.len() {
        for ((n, &o), i) in new.iter_mut().zip(old).zip(imported) {
            if export(o) == *i {
                *n = o;
            }
        }
    }
}

impl Model {
    /// Replaces the geometry of the segments a glTF names with the glTF's;
    /// then [`Model::write`] puts them into the cluster. Returns the
    /// segments read.
    ///
    /// # Errors
    /// Fails on glTF this exporter did not make, missing attributes the
    /// cluster stores, or joints outside a segment's bone palette.
    pub fn import_glb(&mut self, glb: &[u8]) -> Result<Vec<usize>> {
        self.import_glb_with(glb, &|s| s, &HashMap::new())
    }

    /// [`Model::import_glb`] for a model whose segments were renumbered:
    /// `renumber` maps the glTF's segment numbers to this model's, and
    /// `added` gives untagged primitives (glTF mesh, primitive) a segment.
    #[allow(clippy::too_many_lines)] // one pass over meshes and attributes
    fn import_glb_with(
        &mut self,
        glb: &[u8],
        renumber: &dyn Fn(usize) -> usize,
        added: &HashMap<(usize, usize), usize>,
    ) -> Result<Vec<usize>> {
        let (json, bin) = chunks(glb)?;
        let doc = Doc { json, bin };
        let empty = Vec::new();
        let nodes = doc.json["nodes"].as_array().unwrap_or(&empty);
        let mut done = Vec::new();
        for (mesh_index, mesh) in doc.json["meshes"]
            .as_array()
            .unwrap_or(&empty)
            .iter()
            .enumerate()
        {
            let prims = mesh["primitives"].as_array().unwrap_or(&empty);
            let listed = mesh["extras"]["ffx_segments"].as_array();
            // The skin of a node that uses this mesh, as joint names.
            let skin_names: Option<Vec<String>> = nodes
                .iter()
                .find(|n| n["mesh"].as_u64() == Some(mesh_index as u64) && n.get("skin").is_some())
                .and_then(|n| doc.json["skins"][index(&n["skin"])?]["joints"].as_array())
                .map(|joints| {
                    joints
                        .iter()
                        .map(|j| {
                            j.as_u64()
                                .and_then(|j| nodes.get(usize::try_from(j).ok()?))
                                .and_then(|n| n["name"].as_str())
                                .unwrap_or_default()
                                .to_owned()
                        })
                        .collect()
                });
            for (p_index, prim) in prims.iter().enumerate() {
                let segment = prim["extras"]["ffx_segment"].as_u64().or_else(|| {
                    listed
                        .filter(|l| l.len() == prims.len())?
                        .get(p_index)?
                        .as_u64()
                });
                let Some(s) = segment
                    .and_then(|s| usize::try_from(s).ok())
                    .map(renumber)
                    .or_else(|| added.get(&(mesh_index, p_index)).copied())
                else {
                    continue;
                };
                if done.contains(&s) {
                    continue;
                }
                let old = self
                    .segments
                    .get(s)
                    .ok_or_else(|| bad(format!("segment {s} is not in the model")))?
                    .clone();
                let attrs = &prim["attributes"];
                let positions: Vec<[f32; 3]> = doc.floats(&attrs["POSITION"])?;
                let n = positions.len();
                let mut seg = Segment {
                    positions,
                    material: old.material,
                    bones: old.bones.clone(),
                    ..Segment::default()
                };
                let present = |a: &str| !attrs[a].is_null();
                let need = |what: &str, stored: bool| {
                    if stored {
                        Err(bad(format!("segment {s} needs {what}")))
                    } else {
                        Ok(())
                    }
                };
                if present("NORMAL") {
                    let imported: Vec<[f32; 3]> = doc.floats(&attrs["NORMAL"])?;
                    seg.normals.clone_from(&imported);
                    keep(&mut seg.normals, &old.normals, &imported, export_normal);
                } else {
                    need("NORMAL", !old.normals.is_empty())?;
                }
                if present("TEXCOORD_0") {
                    let imported: Vec<[f32; 2]> = doc.floats(&attrs["TEXCOORD_0"])?;
                    seg.uvs = imported.iter().map(|&[u, v]| [u, 1.0 - v]).collect();
                    keep(&mut seg.uvs, &old.uvs, &imported, export_uv);
                } else {
                    need("TEXCOORD_0", !old.uvs.is_empty())?;
                }
                if present("COLOR_0") {
                    let imported: Vec<[f32; 4]> = match doc.floats::<4>(&attrs["COLOR_0"]) {
                        Ok(c) => c,
                        Err(_) => doc
                            .floats::<3>(&attrs["COLOR_0"])?
                            .into_iter()
                            .map(|[r, g, b]| [r, g, b, 1.0])
                            .collect(),
                    };
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped
                    let bytes = |c: [f32; 4]| c.map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8);
                    seg.colors = imported.iter().map(|&c| bytes(c)).collect();
                } else {
                    need("COLOR_0", !old.colors.is_empty())?;
                }
                if !old.joints.is_empty() {
                    let (Some(names), true, true) =
                        (&skin_names, present("JOINTS_0"), present("WEIGHTS_0"))
                    else {
                        return Err(bad(format!(
                            "segment {s} is skinned; JOINTS_0, WEIGHTS_0 and a skin are needed"
                        )));
                    };
                    let skeleton = self
                        .instances
                        .iter()
                        .find(|i| i.segments.iter().any(|&(x, _)| x == s))
                        .and_then(|i| i.skeleton)
                        .and_then(|k| self.skeletons.get(k))
                        .ok_or_else(|| bad(format!("segment {s} has no skeleton")))?;
                    let joints: Vec<[f32; 4]> = doc.floats(&attrs["JOINTS_0"])?;
                    let weights: Vec<[f32; 4]> = doc.floats(&attrs["WEIGHTS_0"])?;
                    // Joints as skeleton hierarchy indices, through the skin's names.
                    let mut exported = Vec::with_capacity(n);
                    for (j, w) in joints.iter().zip(&weights) {
                        let mut hierarchy = [0u16; 4];
                        for k in 0..4 {
                            if w[k] <= 0.0 {
                                continue;
                            }
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            let name = names
                                .get(j[k] as usize)
                                .ok_or_else(|| bad("joint outside the skin"))?;
                            let h =
                                skeleton
                                    .names
                                    .iter()
                                    .position(|x| x == name)
                                    .ok_or_else(|| {
                                        bad(format!("joint {name} is not in the skeleton"))
                                    })?;
                            hierarchy[k] = u16::try_from(h).unwrap_or(u16::MAX);
                        }
                        exported.push((hierarchy, *w));
                    }
                    for (v, &(hierarchy, w)) in exported.iter().enumerate() {
                        // Unedited vertices keep their stored joints and weights.
                        if n == old.joints.len()
                            && export_skin(old.joints[v], old.weights[v], &old.bones)
                                == (hierarchy, w)
                        {
                            seg.joints.push(old.joints[v]);
                            seg.weights.push(old.weights[v]);
                            continue;
                        }
                        let mut palette = [0u8; 4];
                        for k in 0..4 {
                            if w[k] <= 0.0 {
                                continue;
                            }
                            let p = old
                                .bones
                                .iter()
                                .position(|b| b.hierarchy == hierarchy[k])
                                .ok_or_else(|| {
                                    bad(format!(
                                        "segment {s}: joint {} is not in its bone palette",
                                        skeleton.names[usize::from(hierarchy[k])]
                                    ))
                                })?;
                            palette[k] =
                                u8::try_from(p).map_err(|_| bad("bone palette over 256"))?;
                        }
                        seg.joints.push(palette);
                        seg.weights.push(w);
                    }
                }
                #[allow(clippy::cast_precision_loss)] // under 65,536 vertices
                let indices = match prim.get("indices") {
                    Some(i) => doc.floats::<1>(i)?,
                    None => (0..n).map(|i| [i as f32]).collect(),
                };
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // checked below
                {
                    seg.indices = indices.iter().map(|[i]| *i as u16).collect();
                    if indices
                        .iter()
                        .any(|[i]| *i as usize >= n || *i as usize > 0xffff)
                    {
                        return Err(bad(format!("segment {s}: indices past its vertices")));
                    }
                }
                if prim["mode"].as_u64().unwrap_or(4) != 4 {
                    return Err(bad(format!("segment {s}: only triangle lists")));
                }
                self.segments[s] = seg;
                done.push(s);
            }
        }
        done.sort_unstable();
        Ok(done)
    }
}

/// Column-major 4×4 product `a · b`.
fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut m = [0f32; 16];
    for c in 0..4 {
        for r in 0..4 {
            m[c * 4 + r] = (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum();
        }
    }
    m
}

/// Inverse of a column-major 4×4 matrix (cofactor expansion).
#[allow(clippy::many_single_char_names)]
fn invert(m: &[f32; 16]) -> Option<[f32; 16]> {
    let m: Vec<f64> = m.iter().map(|&x| f64::from(x)).collect();
    let mut inv = [0f64; 16];
    let idx = |c: usize, r: usize| c * 4 + r;
    for c in 0..4 {
        for r in 0..4 {
            // Minor of element (row c, column r) of the transpose = cofactor.
            let mut sub = [0f64; 9];
            let mut k = 0;
            for cc in (0..4).filter(|&x| x != r) {
                for rr in (0..4).filter(|&x| x != c) {
                    sub[k] = m[idx(cc, rr)];
                    k += 1;
                }
            }
            let det3 = sub[0] * (sub[4] * sub[8] - sub[5] * sub[7])
                - sub[3] * (sub[1] * sub[8] - sub[2] * sub[7])
                + sub[6] * (sub[1] * sub[5] - sub[2] * sub[4]);
            let sign = if (r + c) % 2 == 0 { 1.0 } else { -1.0 };
            inv[idx(c, r)] = sign * det3;
        }
    }
    let det: f64 = (0..4).map(|k| m[idx(k, 0)] * inv[idx(0, k)]).sum();
    if det.abs() < 1e-12 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    Some(inv.map(|x| (x / det) as f32))
}

/// Joints a primitive is weighted to, as hierarchy indices of `names`.
fn used_joints(
    doc: &Doc,
    prim: &Value,
    skin: Option<&Vec<String>>,
    names: &[String],
) -> Result<Vec<u16>> {
    let attrs = &prim["attributes"];
    let (Some(skin), false, false) = (
        skin,
        attrs["JOINTS_0"].is_null(),
        attrs["WEIGHTS_0"].is_null(),
    ) else {
        return Ok(Vec::new());
    };
    let joints: Vec<[f32; 4]> = doc.floats(&attrs["JOINTS_0"])?;
    let weights: Vec<[f32; 4]> = doc.floats(&attrs["WEIGHTS_0"])?;
    let mut used = Vec::new();
    for (j, w) in joints.iter().zip(&weights) {
        for k in 0..4 {
            if w[k] <= 0.0 {
                continue;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let name = skin
                .get(j[k] as usize)
                .ok_or_else(|| bad("joint outside the skin"))?;
            let h = names
                .iter()
                .position(|x| x == name)
                .ok_or_else(|| bad(format!("joint {name} is not in the skeleton")))?;
            let h = u16::try_from(h).map_err(|_| bad("skeleton over 65,536 joints"))?;
            if !used.contains(&h) {
                used.push(h);
            }
        }
    }
    used.sort_unstable();
    Ok(used)
}

/// What [`import_into_cluster`] made.
#[derive(Debug, Clone)]
pub struct Imported {
    /// The new cluster.
    pub bytes: Vec<u8>,
    /// The segments written (new ones included).
    pub segments: Vec<usize>,
    /// Texture import ids the cluster did not have before. The game finds
    /// a character's textures through its asset list
    /// (`chr/…/<name>.ahwin32`, see [`crate::assets::add_files`]), which
    /// must name them.
    pub new_textures: Vec<String>,
}

/// Puts a glTF's geometry into a model cluster: [`Model::import_glb`] for
/// the segments it names, plus what that cannot do in place. Primitives
/// without a segment become new segments, copies of a segment of the same
/// instance with the same material (else its first) that take the
/// primitive's geometry. Segments weighted to joints outside their bone
/// palette get a larger palette, and joints their mesh has no skeleton
/// matrix for get one: the inverse of the joint node's world matrix.
/// A new primitive whose glTF material is not one of the model's gets a
/// new material: a copy of its template's, with the textures the glTF
/// material lists in `extras.ffx_textures` (sampler → import id).
///
/// # Errors
/// Fails like [`Model::import_glb`], and on primitives of a glTF mesh
/// none of whose primitives names a segment.
#[allow(clippy::too_many_lines)] // the plan, then the structural edits
pub fn import_into_cluster(original: &[u8], glb: &[u8]) -> Result<Imported> {
    let model = Model::parse(original)?;
    let (json, bin) = chunks(glb)?;
    let doc = Doc { json, bin };
    let empty = Vec::new();
    let nodes = doc.json["nodes"].as_array().unwrap_or(&empty);
    // Plan: new primitives with their template, palettes to grow.
    let mut new_prims: Vec<((usize, usize), usize, Vec<u16>)> = Vec::new();
    let mut palettes: Vec<(usize, Vec<u16>)> = Vec::new();
    for (mi, mesh) in doc.json["meshes"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .enumerate()
    {
        let prims = mesh["primitives"].as_array().unwrap_or(&empty);
        let listed = mesh["extras"]["ffx_segments"].as_array();
        let tag = |pi: usize, prim: &Value| {
            prim["extras"]["ffx_segment"]
                .as_u64()
                .or_else(|| listed.filter(|l| l.len() == prims.len())?.get(pi)?.as_u64())
                .and_then(|s| usize::try_from(s).ok())
                .filter(|&s| s < model.segments.len())
        };
        let skin: Option<Vec<String>> = nodes
            .iter()
            .find(|n| n["mesh"].as_u64() == Some(mi as u64) && n.get("skin").is_some())
            .and_then(|n| doc.json["skins"][index(&n["skin"])?]["joints"].as_array())
            .map(|joints| {
                joints
                    .iter()
                    .map(|j| {
                        j.as_u64()
                            .and_then(|j| nodes.get(usize::try_from(j).ok()?))
                            .and_then(|n| n["name"].as_str())
                            .unwrap_or_default()
                            .to_owned()
                    })
                    .collect()
            });
        let tagged: Vec<usize> = prims
            .iter()
            .enumerate()
            .filter_map(|(pi, p)| tag(pi, p))
            .collect();
        for (pi, prim) in prims.iter().enumerate() {
            let target = tag(pi, prim);
            let instance = model.instances.iter().find(|i| {
                i.segments
                    .iter()
                    .any(|&(s, _)| target.map_or_else(|| tagged.contains(&s), |t| t == s))
            });
            let names = instance
                .and_then(|i| i.skeleton)
                .and_then(|k| model.skeletons.get(k))
                .map_or(&[][..], |k| &k.names[..]);
            let used = used_joints(&doc, prim, skin.as_ref(), names)?;
            match target {
                Some(s) => {
                    let palette: Vec<u16> = model.segments[s]
                        .bones
                        .iter()
                        .map(|b| b.hierarchy)
                        .collect();
                    if used.iter().any(|h| !palette.contains(h)) {
                        let mut grown = palette;
                        grown.extend(
                            used.iter()
                                .filter(|h| !grown.contains(h))
                                .copied()
                                .collect::<Vec<_>>(),
                        );
                        palettes.push((s, grown));
                    }
                }
                None => {
                    let instance = instance.ok_or_else(|| {
                        bad(format!(
                            "mesh {mi} primitive {pi}: no primitive of its mesh names a segment to place it with"
                        ))
                    })?;
                    let material = index(&prim["material"]);
                    let template = instance
                        .segments
                        .iter()
                        .find(|&&(_, m)| m.is_some() && m == material)
                        .or_else(|| instance.segments.first())
                        .ok_or_else(|| bad("an instance without segments"))?
                        .0;
                    new_prims.push(((mi, pi), template, used));
                }
            }
        }
    }
    if new_prims.is_empty() && palettes.is_empty() {
        let mut model = model;
        let segments = model.import_glb(glb)?;
        return Ok(Imported {
            bytes: model.write(original)?,
            segments,
            new_textures: Vec::new(),
        });
    }

    // Skeleton matrices for joints without one, per mesh (by a segment of it).
    let mut edit = ClusterEdit::parse(original)?;
    let skeleton_of = |s: usize| {
        model
            .instances
            .iter()
            .find(|i| i.segments.iter().any(|&(x, _)| x == s))
            .and_then(|i| i.skeleton)
    };
    let mut known: HashMap<(usize, u16), u16> = HashMap::new();
    for (s, seg) in model.segments.iter().enumerate() {
        if let Some(k) = skeleton_of(s) {
            for b in &seg.bones {
                known.insert((k, b.hierarchy), b.skeleton);
            }
        }
    }
    let world = |name: &str| -> Option<[f32; 16]> {
        let mut i = model.nodes.iter().position(|n| n.name == name)?;
        let mut m = model.nodes[i].local;
        while let Some(p) = model.nodes[i].parent {
            m = mul(&model.nodes[p].local, &m);
            i = p;
        }
        Some(m)
    };
    let wanted: Vec<(usize, Vec<u16>)> = palettes
        .iter()
        .cloned()
        .chain(new_prims.iter().map(|(_, t, used)| (*t, used.clone())))
        .collect();
    for (s, joints) in &wanted {
        let k = skeleton_of(*s).ok_or_else(|| bad(format!("segment {s} has no skeleton")))?;
        for &h in joints {
            if known.contains_key(&(k, h)) {
                continue;
            }
            let name = &model.skeletons[k].names[usize::from(h)];
            let inverse = world(name)
                .and_then(|w| invert(&w))
                .ok_or_else(|| bad(format!("joint {name}: no node to bind to")))?;
            let index =
                edit.add_skeleton_matrix(u32::try_from(*s).expect("segment"), h, &inverse)?;
            known.insert((k, h), index);
        }
    }
    let palette = |s: usize, joints: &[u16]| -> Vec<(u16, u16)> {
        let k = skeleton_of(s).expect("checked above");
        joints.iter().map(|&h| (h, known[&(k, h)])).collect()
    };
    for (s, joints) in &palettes {
        edit.set_palette(u32::try_from(*s).expect("segment"), &palette(*s, joints))?;
    }
    // New segments; each shifts the segment numbers after it.
    let mut shifts: Vec<usize> = Vec::new();
    let mut added = HashMap::new();
    let renumber = |shifts: &[usize], s: usize| {
        shifts
            .iter()
            .fold(s, |s, &n| if s >= n { s + 1 } else { s })
    };
    let glb_materials = doc.json["materials"].as_array().unwrap_or(&empty);
    let mut made: HashMap<(u32, usize), u32> = HashMap::new();
    let mut new_textures = Vec::new();
    for ((mi, pi), template, used) in &new_prims {
        let t = renumber(&shifts, *template);
        let new = edit.add_segment(u32::try_from(t).expect("segment"))? as usize;
        for v in added.values_mut() {
            if *v >= new {
                *v += 1;
            }
        }
        shifts.push(new);
        if !used.is_empty() {
            edit.set_palette(
                u32::try_from(new).expect("segment"),
                &palette(*template, used),
            )?;
        }
        added.insert((*mi, *pi), new);
        // A glTF material that is not one of the model's: a new material.
        let prim = &doc.json["meshes"][*mi]["primitives"][*pi];
        if let Some(gm) = index(&prim["material"]).filter(|&m| m >= model.materials.len()) {
            let segment = u32::try_from(new).expect("segment");
            let mesh = edit.mesh_of(segment)?.0;
            let set_index = if let Some(&i) = made.get(&(mesh, gm)) {
                i
            } else {
                let base = edit.segment_material(segment)?;
                let (material, i) = edit.add_material(base, segment)?;
                let old = model.segments[*template]
                    .material
                    .map(|m| &model.materials[m].textures[..])
                    .unwrap_or_default();
                if let Some(textures) = glb_materials
                    .get(gm)
                    .and_then(|m| m["extras"]["ffx_textures"].as_object())
                {
                    for (sampler, id) in textures {
                        let Some(id) = id.as_str() else { continue };
                        if old.iter().any(|t| &t.sampler == sampler && t.path == id) {
                            continue;
                        }
                        if edit.import_index(id)?.is_none() && !new_textures.iter().any(|t| t == id)
                        {
                            new_textures.push(id.to_owned());
                        }
                        edit.set_texture(material, sampler, id)?;
                    }
                }
                made.insert((mesh, gm), i);
                i
            };
            edit.set_segment_material(segment, set_index)?;
        }
    }
    let grown = edit.write()?;
    let mut model = Model::parse(&grown)?;
    let segments = model.import_glb_with(glb, &|s| renumber(&shifts, s), &added)?;
    Ok(Imported {
        bytes: model.write(&grown)?,
        segments,
        new_textures,
    })
}
