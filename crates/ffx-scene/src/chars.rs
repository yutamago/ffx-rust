//! The character world (`PhyreScene +0x7c`): what its draws derive from
//! the scene inputs' character part ([`ffx_gfx::scene_inputs::CharWorld`])
//! (`docs/formats/scene-chars.md`).
//!
//! - The joint matrices `computeJointMatrices` (`0x004f6ef0`) builds for
//!   an instance: slot -1 its world, slot k its parent's slot (the mesh's
//!   `+0x2c` table) times the instance's joint matrix k (`0x004f6fc0`).
//! - A segment's `World` is the joint slot its `+4` names
//!   (`renderMeshInstance` `0x005b9a90`).
//! - `BoneTransforms` (`generateNodeParameterCaptureBuffer` `0x005b7fe0`,
//!   semantic `0x8f`): per `m_skinBones` entry, joint `hierarchy` times
//!   skeleton matrix `skeleton` (`PMatrix4_MulThis_004c9da0`).
//! - The per-node buffer and the constant buffers through the location
//!   tables, as for the map ([`crate::build`]).
//!
//! Matrices are in memory order here (column-major, translation in
//! elements 12-14) unless a name says `cb` (the shaders' order,
//! [`crate::inputs::transpose`]).

// Matrix and location-table code names its values by position (a, b, c,
// r, o, t, n, ...).
#![allow(clippy::many_single_char_names)]

use ffx_gfx::scene_inputs::{CharInstance, Instance, SceneInputs};

use crate::effect::{EffectVariant, LocTable, Source};
use crate::inputs::{self, Mat};

/// The characters' world in `PhyreScene`.
pub const CHAR_WORLD: u32 = 0x7c;

/// The records of the character world of capture frame `frame`.
#[must_use]
pub fn records(all: &[SceneInputs], frame: u32) -> Vec<&SceneInputs> {
    all.iter()
        .filter(|s| s.frame == frame && s.world == CHAR_WORLD && s.chars.is_some())
        .collect()
}

/// The character part's entry for instance `index` of `s`.
#[must_use]
pub fn char_instance(s: &SceneInputs, index: usize) -> Option<&CharInstance> {
    s.chars.as_ref()?.instances.get(index)
}

/// The instance's world as `computeJointMatrices` takes it: the identity
/// when `+0x6c` bit 0 is set or the world pointer is 0, else
/// `PMatrix4_FromMatrix3x4(*(inst +4))`.
#[must_use]
pub fn instance_world(i: &Instance, c: &CharInstance) -> Mat {
    match (&i.world, c.flags & 1) {
        (Some(m), 0) => inputs::from_3x4(m),
        _ => inputs::IDENTITY,
    }
}

/// `m` times the 3-vector `v` (`0x004ca140`: w taken as 0) or the point
/// `v` (`0x004c9eb0`: w 1), in memory order: per row `r`, `((m[r] x +
/// m[4+r] y) + m[8+r] z)`, plus `m[12+r]` for a point, each product and
/// sum stored as an `f32` (so plain `f32` arithmetic). All four rows.
#[must_use]
pub fn transform(m: &Mat, v: [f32; 3], point: bool) -> [f32; 4] {
    std::array::from_fn(|r| {
        let s = m[r] * v[0] + m[4 + r] * v[1];
        let s = s + m[8 + r] * v[2];
        if point { s + m[12 + r] } else { s }
    })
}

/// `0x004f6fc0`'s product for one joint: the columns of `joint` (its
/// first three rows; the fourth is dropped) through `parent`, the
/// fourth as a point, then made a matrix of those four columns
/// (`0x00450de0`).
#[must_use]
pub fn joint_product(parent: &Mat, joint: &Mat) -> Mat {
    let col = |c: usize| [joint[4 * c], joint[4 * c + 1], joint[4 * c + 2]];
    let mut out = [0.0; 16];
    for c in 0..4 {
        out[4 * c..4 * c + 4].copy_from_slice(&transform(parent, col(c), c == 3));
    }
    out
}

/// The joint matrices of an instance (`computeJointMatrices`): the
/// instance world (slot -1, [`instance_world`]) and, in joint order,
/// [`joint_product`] of the joint's parent slot (the mesh's `+0x2c`
/// table: -1 the world, else an earlier joint) and its `+0xc` matrix. A
/// parent the list doesn't hold yet (none on the captured characters)
/// takes the identity.
#[must_use]
pub fn joints(i: &Instance, c: &CharInstance) -> (Mat, Vec<Mat>) {
    let world = instance_world(i, c);
    let mut list: Vec<Mat> = Vec::with_capacity(c.joints.len());
    for (k, j) in c.joints.iter().enumerate() {
        let parent = match c.joint_parent.get(k).copied().unwrap_or(-1) {
            p if p < 0 => world,
            p => list
                .get(p.unsigned_abs() as usize)
                .copied()
                .unwrap_or(inputs::IDENTITY),
        };
        list.push(joint_product(&parent, j));
    }
    (world, list)
}

/// A joint slot: -1 (or any negative value) the instance world, else
/// joint `slot`.
#[must_use]
pub fn joint_slot(world: &Mat, joints: &[Mat], slot: u32) -> Option<Mat> {
    match usize::try_from(slot.cast_signed()) {
        Ok(k) => joints.get(k).copied(),
        Err(_) => Some(*world),
    }
}

/// `PMatrix4_MulThis_004c9da0` (`a . b`, memory order): element (r, c)
/// = `((a(r,0) b(0,c) + a(r,1) b(1,c)) + a(r,2) b(2,c)) + a(r,3)
/// b(3,c)`, each step an `f32`.
#[must_use]
pub fn mul_this(a: &Mat, b: &Mat) -> Mat {
    std::array::from_fn(|i| {
        let (c, r) = (i / 4, i % 4);
        let mut s = a[r] * b[4 * c];
        for k in 1..4 {
            s += a[4 * k + r] * b[4 * c + k];
        }
        s
    })
}

/// `BoneTransforms` of segment `segment`: per `m_skinBones` entry
/// (hierarchy `h`, skeleton `s`), joint `h` times skeleton matrix `s`
/// ([`mul_this`]), in memory order. `None` for an entry whose joint or
/// skeleton matrix is missing.
#[must_use]
pub fn bone_matrices(c: &CharInstance, joints: &[Mat], segment: usize) -> Vec<Option<Mat>> {
    c.skin_bones
        .get(segment)
        .map(|bones| {
            bones
                .iter()
                .map(|&[h, s]| {
                    let j = joints.get(usize::from(h))?;
                    let k = c.skeleton.get(usize::from(s))?;
                    Some(mul_this(j, k))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `a b - c d` with the products exact and one rounding to `f32` (the
/// x87 at 53 bits, then the store).
#[allow(clippy::cast_possible_truncation)] // the f32 store
fn diff_of_products(a: f32, b: f32, c: f32, d: f32) -> f32 {
    (f64::from(a) * f64::from(b) - f64::from(c) * f64::from(d)) as f32
}

/// `PMatrix4_Inverse_004cc0c0`: the inverse of the affine part of `m`
/// (memory order) as Vectormath's `inverse(Transform3)` (`0x004cc890`)
/// computes it: the cross products of the columns (each component one
/// rounding), the determinant `(c2.x t2.x + c2.y t2.y) + c2.z t2.z` and
/// `1 / det` as `f32` steps, each element times it, the translation
/// `-((i0 tx + i1 ty) + i2 tz)` per row as `f32` steps (checked on the
/// captured draws); bottom row 0 0 0
/// 1 (`0x004c9360`).
#[must_use]
pub fn affine_inverse(m: &Mat) -> Mat {
    let p = |i: usize| m[i];
    let dp = diff_of_products;
    // cross(c1, c2), cross(c2, c0), cross(c0, c1).
    let t0 = [
        dp(p(5), p(10), p(9), p(6)),
        dp(p(8), p(6), p(4), p(10)),
        dp(p(4), p(9), p(5), p(8)),
    ];
    let t1 = [
        dp(p(9), p(2), p(1), p(10)),
        dp(p(0), p(10), p(8), p(2)),
        dp(p(8), p(1), p(9), p(0)),
    ];
    let t2 = [
        dp(p(1), p(6), p(5), p(2)),
        dp(p(4), p(2), p(0), p(6)),
        dp(p(0), p(5), p(1), p(4)),
    ];
    let det = p(8) * t2[0];
    let det = det + p(9) * t2[1];
    let det = det + p(10) * t2[2];
    let inv = 1.0 / det;
    let mut out = [0.0f32; 16];
    for r in 0..3 {
        out[r] = [t0[0], t1[0], t2[0]][r] * inv;
        out[4 + r] = [t0[1], t1[1], t2[1]][r] * inv;
        out[8 + r] = [t0[2], t1[2], t2[2]][r] * inv;
    }
    for r in 0..3 {
        let s = out[r] * p(12);
        let s = s + out[4 + r] * p(13);
        let s = s + out[8 + r] * p(14);
        out[12 + r] = -s;
    }
    out[15] = 1.0;
    out
}

/// What the per-node buffer of one draw is made from.
#[derive(Debug, Clone)]
pub struct NodeInputs {
    /// The camera's `scene.View` and `scene.ViewProjection` (cb order).
    pub view: Mat,
    pub view_projection: Mat,
    /// The segment's `World` (memory order: its joint slot).
    pub world: Mat,
    /// The segment's bone matrices (memory order; empty for an unskinned
    /// segment).
    pub bones: Vec<Option<Mat>>,
    /// The renderer's `+0x60` matrix (memory order), when the record has
    /// it: `WorldViewInverse` = `WorldInverse . it`.
    pub view_inverse: Option<Mat>,
}

fn put_mat(buf: &mut [u8], at: usize, m: &Mat) {
    for (k, x) in m.iter().enumerate() {
        if let Some(dst) = buf.get_mut(at + 4 * k..at + 4 * k + 4) {
            dst.copy_from_slice(&x.to_le_bytes());
        }
    }
}

/// The per-node buffer of a draw as `generateNodeParameterCaptureBuffer`
/// (`0x005b7fe0`) fills it, in memory order: `World` (0x82) the joint
/// slot, `WorldView` (0x83, `View . World`, f32 sums), `WorldViewProjection`
/// (0x84, `ViewProjection . World`, f64 sums rounded once),
/// `BoneTransforms` (0x8f, data type `'1'`: one 4x4 per bone, `'0'`: the
/// 4x4's first 12 floats; after the last bone, while the array has room
/// (`size >> 6 & 0x7f` or `(size & 0x1fff) / 0x30`), the next slot's
/// first word is `0xffffffff`). Other semantics are left 0 and named in
/// the second result. Words no definition writes are 0 here; the game's
/// are ring memory, which no location entry reads.
#[must_use]
pub fn node_buffer(v: &EffectVariant, n: &NodeInputs) -> (Vec<u8>, Vec<String>) {
    let mut buf = vec![0u8; usize::from(v.untweakable_size).max(16)];
    let mut notes = Vec::new();
    for d in &v.untweakable {
        let at = d.offset as usize;
        match d.kind {
            0x82 => put_mat(&mut buf, at, &n.world),
            0x83 => {
                let wv = crate::census::mul(&n.view, &inputs::transpose(&n.world));
                put_mat(&mut buf, at, &inputs::transpose(&wv));
            }
            0x84 => {
                let wvp = crate::census::mul_f64(&n.view_projection, &inputs::transpose(&n.world));
                put_mat(&mut buf, at, &inputs::transpose(&wvp));
            }
            0x8f => {
                let (stride, room) = if d.data_type == b'0' {
                    (0x30, (d.size as usize & 0x1fff) / 0x30)
                } else {
                    (0x40, (d.size as usize >> 6) & 0x7f)
                };
                let count = n.bones.len().min(room);
                for (k, m) in n.bones.iter().take(count).enumerate() {
                    let m = m.unwrap_or(inputs::IDENTITY);
                    let bytes: Vec<u8> = m.iter().flat_map(|x| x.to_le_bytes()).collect();
                    if let Some(dst) = buf.get_mut(at + k * stride..at + (k + 1) * stride) {
                        dst.copy_from_slice(&bytes[..stride]);
                    }
                }
                if count < room
                    && let Some(dst) = buf.get_mut(at + count * stride..at + count * stride + 4)
                {
                    dst.copy_from_slice(&u32::MAX.to_le_bytes());
                }
            }
            0x85 => put_mat(&mut buf, at, &affine_inverse(&n.world)),
            0x86 => match &n.view_inverse {
                Some(v) => put_mat(&mut buf, at, &mul_this(&affine_inverse(&n.world), v)),
                None => notes.push("WorldViewInverse: no renderer matrix in the record".into()),
            },
            k => notes.push(format!("per-node semantic {k:#x} ({}) left 0", d.name)),
        }
    }
    (buf, notes)
}

/// What one location entry writes into the constant buffer (the render
/// thread's copy, `0x0059a650`): kind `'1'` (0x31) a 4x4 per 64 bytes,
/// transposed (element (r, c) from source word `4c + r`), `'0'` (0x30)
/// per 48 bytes the source's 4 groups of 3 words as 3 rows of 4, both
/// stopping at an element whose first word is `0xffffffff` (the words
/// after it keep what the buffer held); any other kind a plain copy.
#[must_use]
pub fn cb_copy(kind: u8, src: &[u8]) -> Vec<u8> {
    let word = |i: usize| -> [u8; 4] {
        src.get(4 * i..4 * i + 4)
            .and_then(|b| b.try_into().ok())
            .unwrap_or([0; 4])
    };
    let (stride, order): (usize, fn(usize) -> usize) = match kind {
        0x31 => (16, |i| (i % 4) * 4 + i / 4),
        0x30 => (12, |i| (i % 4) * 3 + i / 4),
        _ => return src.to_vec(),
    };
    let mut out = Vec::new();
    for e in 0..src.len() / (4 * stride) {
        if word(e * stride) == [0xff; 4] {
            break;
        }
        for i in 0..stride {
            out.extend_from_slice(&word(e * stride + order(i)));
        }
    }
    out
}

/// One compared constant: where it goes, its name and source, and whether
/// the inputs give the captured bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    pub stage: char,
    pub name: String,
    pub source: Source,
    pub target: u32,
    pub size: u32,
    /// Bytes the entry writes (a `BoneTransforms` copy stops after the
    /// last bone).
    pub written: u32,
    pub exact: bool,
    /// The first differing word (captured, from the inputs).
    pub first_diff: Option<(usize, u32, u32)>,
    /// The largest difference of a word as floats, in units in the last
    /// place (`u32::MAX`: a value missing).
    pub max_ulp: u32,
}

/// The distance of two floats in units in the last place.
#[must_use]
pub fn ulp_distance(a: u32, b: u32) -> u32 {
    let ord = |x: u32| {
        let i = x.cast_signed();
        if i < 0 { i32::MIN - i } else { i }
    };
    ord(a).abs_diff(ord(b))
}

/// The material and per-node entries of a location table checked against
/// a captured constant buffer: material values from the snapshot's live
/// `PParameterBuffer`, per-node values from [`node_buffer`], each through
/// [`cb_copy`]. Scene-wide entries are the map's (checked there) and
/// skipped.
#[must_use]
pub fn check_table(
    stage: char,
    table: &LocTable,
    variant: &EffectVariant,
    material: &[u8],
    node: &[u8],
    captured: &[u8],
) -> Vec<Checked> {
    let mut out = Vec::new();
    for (src, loc) in table.sourced() {
        // A stage that binds no constant buffer (the blob shadow's pixel
        // program) reads none of its entries.
        if loc.target == u32::MAX || captured.is_empty() {
            continue;
        }
        let (o, t, n) = (
            usize::from(loc.offset),
            loc.target as usize,
            loc.size as usize,
        );
        let (name, from) = match src {
            Source::Material => (
                variant
                    .tweakable_at(loc.offset)
                    .map_or_else(|| format!("material {o:#x}"), |d| d.name.clone()),
                material,
            ),
            Source::Node => (
                variant
                    .untweakable
                    .iter()
                    .find(|d| d.offset == u32::from(loc.offset))
                    .map_or_else(|| format!("node {o:#x}"), |d| d.name.clone()),
                node,
            ),
            _ => continue,
        };
        let want = from.get(o..o + n).map(|b| cb_copy(loc.kind, b));
        let got = captured.get(t..t + n);
        let first_diff = match (&want, got) {
            (Some(w), Some(g)) => (0..w.len() / 4).find_map(|k| {
                let a = u32::from_le_bytes(g[4 * k..4 * k + 4].try_into().ok()?);
                let b = u32::from_le_bytes(w[4 * k..4 * k + 4].try_into().ok()?);
                (a != b).then_some((k, a, b))
            }),
            _ => Some((0, 0, 0)),
        };
        let max_ulp = match (&want, got) {
            (Some(w), Some(g)) => (0..w.len() / 4)
                .map(|k| {
                    let word = |b: &[u8]| {
                        u32::from_le_bytes([b[4 * k], b[4 * k + 1], b[4 * k + 2], b[4 * k + 3]])
                    };
                    ulp_distance(word(g), word(w))
                })
                .max()
                .unwrap_or(0),
            _ => u32::MAX,
        };
        out.push(Checked {
            stage,
            name,
            source: src,
            target: loc.target,
            size: loc.size,
            written: want
                .as_ref()
                .map_or(0, |w| u32::try_from(w.len()).unwrap_or(0)),
            exact: first_diff.is_none(),
            first_diff,
            max_ulp,
        });
    }
    out
}

/// What a character-world draw was tied to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tie {
    /// A segment of an instance of the record.
    Segment { instance: usize, segment: usize },
    /// No candidate reproduces the draw's material values.
    None,
}

/// One checked character-world draw.
#[derive(Debug, Clone)]
pub struct DrawCheck {
    /// The scene draw number.
    pub number: usize,
    pub tie: Tie,
    /// How many (instance, segment) pairs reproduce the draw's material
    /// values (more than one: equal materials; the node values decide).
    pub candidates: usize,
    pub checked: Vec<Checked>,
    pub notes: Vec<String>,
}

impl DrawCheck {
    /// Whether every compared constant is exact.
    #[must_use]
    pub fn exact(&self) -> bool {
        self.tie != Tie::None && self.checked.iter().all(|c| c.exact)
    }
}

/// The shader pass of a variant that binds exactly these programs.
#[must_use]
pub fn pass_of<'a>(
    v: &'a EffectVariant,
    vs: &[u8],
    ps: &[u8],
) -> Option<(&'a str, &'a crate::effect::ShaderPass)> {
    v.passes.iter().find_map(|rp| {
        rp.shaders
            .iter()
            .flat_map(|s| &s.passes)
            .find(|p| p.vs.code == vs && p.ps.code == ps)
            .map(|p| (rp.pass_type.as_str(), p))
    })
}

/// The node inputs of segment `segment` of instance `index` of `s`.
#[must_use]
pub fn node_inputs(s: &SceneInputs, index: usize, segment: usize) -> Option<NodeInputs> {
    let i = s.instances.get(index)?;
    let c = char_instance(s, index)?;
    let (world, list) = joints(i, c);
    let seg = i.segments.get(segment)?;
    Some(NodeInputs {
        view: inputs::view(s)?,
        view_projection: inputs::view_projection(s)?,
        world: joint_slot(&world, &list, seg.matrix)?,
        bones: bone_matrices(c, &list, segment),
        view_inverse: s.chars.as_ref()?.renderer.first().copied(),
    })
}

/// Checks one captured draw against the record: the shader pass `pass`
/// of `variant` (the draw's programs), the captured `$Globals` of each
/// stage. Every drawable (instance, segment) of the record whose live
/// material buffer gives the draw's material values is a candidate; the
/// tie is the one with the most exact constants, and among equals the
/// one whose segment is `hint` (the segment's place in its file
/// instance, from the census's vertex data), else the first.
#[must_use]
pub fn check_draw(
    s: &SceneInputs,
    number: usize,
    variant: &EffectVariant,
    pass: &crate::effect::ShaderPass,
    vs_cb: &[u8],
    ps_cb: &[u8],
    hint: Option<usize>,
) -> DrawCheck {
    let skinned = variant.untweakable.iter().any(|d| d.kind == 0x8f);
    let run = |inst: usize, seg: usize, mat: &[u8]| -> Option<Vec<Checked>> {
        let (node, _) = node_buffer(variant, &node_inputs(s, inst, seg)?);
        let mut out = check_table('v', &pass.vs_params, variant, mat, &node, vs_cb);
        out.extend(check_table(
            'p',
            &pass.ps_params,
            variant,
            mat,
            &node,
            ps_cb,
        ));
        Some(out)
    };
    let mut candidates: Vec<(usize, usize, Vec<Checked>)> = Vec::new();
    for (k, i) in s.instances.iter().enumerate() {
        let Some(c) = char_instance(s, k) else {
            continue;
        };
        for (g, seg) in i.segments.iter().enumerate() {
            if seg.hidden != 0 || skinned == c.skin_bones.get(g).is_none_or(Vec::is_empty) {
                continue;
            }
            let Some(m) = inputs::segment_material(s, i, seg) else {
                continue;
            };
            let Some(checked) = run(k, g, &m.parameters) else {
                continue;
            };
            if checked
                .iter()
                .filter(|c| c.source == Source::Material)
                .all(|c| c.exact)
            {
                candidates.push((k, g, checked));
            }
        }
    }
    let n = candidates.len();
    let mut best = 0;
    let mut most = (0, false);
    for (k, (_, g, c)) in candidates.iter().enumerate() {
        let n = (c.iter().filter(|x| x.exact).count(), hint == Some(*g));
        if n > most {
            (best, most) = (k, n);
        }
    }
    let mut notes = Vec::new();
    if n > 1 {
        notes.push(format!("{n} segments have this draw's material values"));
    }
    match candidates.into_iter().nth(best) {
        Some((instance, segment, checked)) => DrawCheck {
            number,
            tie: Tie::Segment { instance, segment },
            candidates: n,
            checked,
            notes,
        },
        None => {
            // For the report: the segment that misses the fewest material
            // values, and what it misses.
            let mut best: Option<(usize, usize, usize, Vec<Checked>)> = None;
            for (k, i) in s.instances.iter().enumerate() {
                for (g, seg) in i.segments.iter().enumerate() {
                    let Some(m) = inputs::segment_material(s, i, seg) else {
                        continue;
                    };
                    let Some(checked) = run(k, g, &m.parameters) else {
                        continue;
                    };
                    let miss = checked
                        .iter()
                        .filter(|c| c.source == Source::Material && !c.exact)
                        .count();
                    if best.as_ref().is_none_or(|b| miss < b.0) {
                        best = Some((miss, k, g, checked));
                    }
                }
            }
            let mut notes = vec!["no segment of the record has this draw's material values".into()];
            if let Some((miss, k, g, _)) = &best {
                notes.push(format!(
                    "closest: instance {k} segment {g}, {miss} material values differ"
                ));
            }
            DrawCheck {
                number,
                tie: Tie::None,
                candidates: 0,
                checked: best.map(|b| b.3).unwrap_or_default(),
                notes,
            }
        }
    }
}

/// The contents of buffer `id` as draw command `cmd` of the frame sees
/// it: the declared contents with the frame's earlier `UpdateBuffer`
/// commands applied in order.
#[must_use]
pub fn buffer_at(cap: &crate::capture::Capture, cmd: usize, id: u32) -> Option<Vec<u8>> {
    let mut data = cap.buffer(id)?.1.to_vec();
    for c in cap.frame.cmds.iter().take(cmd) {
        if let ffx_gfx::Cmd::UpdateBuffer {
            id: u,
            offset,
            data: d,
        } = c
            && *u == id
        {
            let o = *offset as usize;
            if data.len() < o + d.len() {
                data.resize(o + d.len(), 0);
            }
            data[o..o + d.len()].copy_from_slice(d);
        }
    }
    Some(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joint_products_follow_the_columns() {
        // A parent that moves by (1, 2, 3) and doubles x.
        let mut parent = inputs::IDENTITY;
        parent[0] = 2.0;
        parent[12] = 1.0;
        parent[13] = 2.0;
        parent[14] = 3.0;
        let mut joint = inputs::IDENTITY;
        joint[12] = 5.0;
        // The joint's fourth row is dropped.
        joint[3] = 9.0;
        let j = joint_product(&parent, &joint);
        assert_eq!(&j[0..4], &[2.0, 0.0, 0.0, 0.0]);
        assert_eq!(&j[12..16], &[11.0, 2.0, 3.0, 1.0]);
    }

    #[test]
    fn joint_slots_take_the_world_for_minus_one() {
        let w = [7.0; 16];
        let j = vec![[1.0; 16]];
        assert_eq!(joint_slot(&w, &j, u32::MAX), Some(w));
        assert_eq!(joint_slot(&w, &j, 0), Some(j[0]));
        assert_eq!(joint_slot(&w, &j, 1), None);
    }

    #[test]
    fn matrix_copies_transpose_and_stop_at_the_marker() {
        let mut src = Vec::new();
        for i in 0..16u32 {
            src.extend_from_slice(&f32::from(u16::try_from(i).unwrap_or(0)).to_le_bytes());
        }
        src.extend_from_slice(&[0xff; 4]);
        src.extend_from_slice(&[0; 60]);
        let out = cb_copy(0x31, &src);
        assert_eq!(out.len(), 64);
        let f = |i: usize| f32::from_le_bytes(out[4 * i..4 * i + 4].try_into().unwrap());
        // Row 0 of the constant is the source's first column... transposed.
        assert_eq!([f(0), f(1), f(2), f(3)], [0.0, 4.0, 8.0, 12.0]);
        assert_eq!(cb_copy(0, &src[..8]), src[..8].to_vec());
        let three = cb_copy(0x30, &src[..48]);
        let g = |i: usize| f32::from_le_bytes(three[4 * i..4 * i + 4].try_into().unwrap());
        assert_eq!([g(0), g(1), g(2), g(3)], [0.0, 3.0, 6.0, 9.0]);
    }

    #[test]
    fn affine_inverses_undo_the_matrix() {
        let mut m = inputs::IDENTITY;
        m[0] = 2.0;
        m[12] = 4.0;
        m[13] = -1.0;
        let i = affine_inverse(&m);
        assert_eq!(i[0], 0.5);
        assert_eq!([i[12], i[13], i[14], i[15]], [-2.0, 1.0, -0.0, 1.0]);
        assert_eq!(mul_this(&m, &i), inputs::IDENTITY);
    }

    #[test]
    fn mul_this_is_the_column_major_product() {
        let mut a = inputs::IDENTITY;
        a[12] = 1.0; // translate x by 1
        let mut b = inputs::IDENTITY;
        b[0] = 3.0; // scale x by 3
        b[12] = 2.0;
        // a . b: scale, then move by 2, then by 1.
        let p = mul_this(&a, &b);
        assert_eq!(p[0], 3.0);
        assert_eq!(p[12], 3.0);
    }
}
