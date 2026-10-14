//! FFX's PS2 skeletons and motions, which the remaster still plays:
//!
//! - [`Skeleton`]: the bind pose in a character's `mdl/*.chr`;
//! - [`MotionGroup`]: the clips and motion scripts in `mot/*.mgrp` and
//!   `event/obj/**.mgrp`.
//!
//! PS2 bone *i* drives the remastered model's node `SkeletonNode_i`. A
//! bone's local transform is `T · Rz · Ry · Rx · S`. The format is
//! described in `docs/formats/motion.md`.

pub mod clip;
pub mod group;
mod json;

pub use clip::{Channel, Clip};
pub use group::{ClipEntry, Motion, MotionGroup, Sequence};

/// Padding byte between sections.
pub(crate) const PAD: u8 = 0x77;

/// Translation units per model unit in clips (bind poses use 1000).
pub const MOTION_UNITS: f32 = 100.0;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("truncated at {0:#x}")]
    Truncated(usize),
    #[error("malformed: {0}")]
    Malformed(&'static str),
    #[error("delta {0} does not fit 14 bits")]
    Unencodable(i16),
    #[error("JSON: {0}")]
    Json(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A bone's bind pose (`SG_InitBones`, `0x008277f0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bone {
    /// The parent bone; a root is its own parent.
    pub parent: u16,
    /// Rotation x, y, z in 1/100 degree.
    pub rotation: [i16; 3],
    /// Translation in 1/1000 unit.
    pub translation: [i16; 3],
    /// Scale in 1/4096 (the game keeps only the integer part).
    pub scale: [i16; 3],
}

/// The bones of a `.chr` model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

impl Skeleton {
    /// Reads the bones of a `.chr` file: its model block (offset at +0x10)
    /// has the bone count at +0x0a and the bone table offset at +0x1c,
    /// 20 bytes per bone. Files without a model block have no bones.
    ///
    /// # Errors
    /// Fails on truncated data.
    pub fn from_chr(b: &[u8]) -> Result<Self> {
        let u32_at = |o: usize| {
            b.get(o..o + 4)
                .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")) as usize)
                .ok_or(Error::Truncated(o))
        };
        let i16_at = |o: usize| {
            b.get(o..o + 2)
                .map(|s| i16::from_le_bytes([s[0], s[1]]))
                .ok_or(Error::Truncated(o))
        };
        let model = u32_at(0x10)?;
        if model == 0 {
            // Some files (chr/skl) have no model block.
            return Ok(Self { bones: Vec::new() });
        }
        let count = i16_at(model + 0xa)?.cast_unsigned() as usize;
        let table = model + u32_at(model + 0x1c)?;
        let bones = (0..count)
            .map(|i| {
                let o = table + 20 * i;
                let w = |k: usize| i16_at(o + 2 * k);
                Ok(Bone {
                    parent: w(0)?.cast_unsigned(),
                    rotation: [w(1)?, w(2)?, w(3)?],
                    translation: [w(4)?, w(5)?, w(6)?],
                    scale: [w(7)?, w(8)?, w(9)?],
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if bones.iter().any(|b| usize::from(b.parent) >= count.max(1)) {
            return Err(Error::Malformed("bone parent out of range"));
        }
        Ok(Self { bones })
    }
}

/// Model kinds by the top nibble of a character id (`0x00829e10`), with
/// their `chr/` folders.
const KINDS: [(char, &str); 7] = [
    ('c', "pc"),
    ('m', "mon"),
    ('n', "npc"),
    ('s', "sum"),
    ('w', "wep"),
    ('f', "obj"),
    ('k', "skl"),
];

/// The `chr/` folder of a character id, e.g. `0x1002` → `mon/m002`.
#[must_use]
pub fn character_dir(id: u16) -> Option<String> {
    let (letter, dir) = KINDS.get(usize::from(id >> 12))?;
    Some(format!("{dir}/{letter}{:03}", id & 0xfff))
}

/// The character id of a model name such as `c001` or `m002`.
#[must_use]
pub fn character_id(name: &str) -> Option<u16> {
    let mut chars = name.chars();
    let letter = chars.next()?;
    let kind = KINDS.iter().position(|(l, _)| *l == letter)?;
    let number: u16 = chars.as_str().parse().ok().filter(|&n| n < 0x1000)?;
    Some(u16::try_from(kind).expect("7 kinds") << 12 | number)
}

/// Characters whose motions a `.chr` file names: motion ids (`character
/// << 16 | number`, numbers `0x1000..0x2000`) of another character in the
/// header before the model block. Monsters that share a skeleton name the
/// one that owns the motions this way (`0x00837ac0` looks a motion up by
/// the id's character). Most often named first.
#[must_use]
pub fn referenced_characters(chr: &[u8], own: u16) -> Vec<u16> {
    let end = chr
        .get(0x10..0x14)
        .map_or(0, |s| {
            u32::from_le_bytes(s.try_into().expect("4 bytes")) as usize
        })
        .min(chr.len());
    let mut counts: Vec<(u16, usize)> = Vec::new();
    for word in chr.get(0x70..end).unwrap_or(&[]).as_chunks::<4>().0 {
        let character = u16::from_le_bytes([word[2], word[3]]);
        let number = u16::from_le_bytes([word[0], word[1]]);
        if character != own
            && character != 0
            && usize::from(character >> 12) < KINDS.len()
            && number >> 12 == 1
        {
            match counts.iter_mut().find(|(c, _)| *c == character) {
                Some((_, n)) => *n += 1,
                None => counts.push((character, 1)),
            }
        }
    }
    counts.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    counts.into_iter().map(|(c, _)| c).collect()
}

/// A bone transform from the nine clip channel values, in glTF terms:
/// translation, rotation quaternion `[x, y, z, w]` and scale.
#[must_use]
#[allow(clippy::many_single_char_names)] // angles x, y, z; t, r, s
pub fn trs(values: [i16; 9]) -> ([f32; 3], [f32; 4], [f32; 3]) {
    let turn = |v: i16| f32::from(v) / 4096.0 * std::f32::consts::TAU;
    let [x, y, z] = [turn(values[0]), turn(values[1]), turn(values[2])];
    let t = [3, 4, 5].map(|k| f32::from(values[k]) / MOTION_UNITS);
    let s = [6, 7, 8].map(|k| f32::from(values[k]) / 4096.0);
    (t, euler_xyz(x, y, z), s)
}

/// The quaternion of `Rz(z) · Ry(y) · Rx(x)` (x applied first).
#[must_use]
pub fn euler_xyz(x: f32, y: f32, z: f32) -> [f32; 4] {
    let (sx, cx) = (x / 2.0).sin_cos();
    let (sy, cy) = (y / 2.0).sin_cos();
    let (sz, cz) = (z / 2.0).sin_cos();
    [
        sx * cy * cz - cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
        cx * cy * cz + sx * sy * sz,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first column of a quaternion's rotation matrix.
    fn x_axis([x, y, z, w]: [f32; 4]) -> [f32; 3] {
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + w * z),
            2.0 * (x * z - w * y),
        ]
    }

    #[test]
    fn character_ids() {
        assert_eq!(character_dir(0x1002).as_deref(), Some("mon/m002"));
        assert_eq!(character_id("c001"), Some(1));
        assert_eq!(character_id("w014"), Some(0x400e));
        assert_eq!(character_id("x001"), None);
    }

    #[test]
    fn rotation_order_matches_the_remastered_bind_pose() {
        // c001 bone 11: (24.57°, 8.92°, 15.55°); its SkeletonNode_11 local
        // matrix has the x axis (0.952, 0.265, -0.155).
        let deg = |d: f32| d.to_radians();
        let q = euler_xyz(deg(24.57), deg(8.92), deg(15.55));
        let x = x_axis(q);
        for (a, b) in x.iter().zip([0.952, 0.265, -0.155]) {
            assert!((a - b).abs() < 2e-3, "{x:?}");
        }
    }
}
