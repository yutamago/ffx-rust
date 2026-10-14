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

use serde_json::Value;

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
    #[allow(clippy::too_many_lines)] // one pass over meshes and attributes
    pub fn import_glb(&mut self, glb: &[u8]) -> Result<Vec<usize>> {
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
                let Some(s) = segment.and_then(|s| usize::try_from(s).ok()) else {
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
