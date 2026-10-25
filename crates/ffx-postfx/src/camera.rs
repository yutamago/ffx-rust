//! The map's cameras: `PhyreScene_findGlobalCamera` (`0x006519f0`) keeps
//! the map cluster's `PCamera` named `GlobalCamera` at scene `+0x4f8`, and
//! `PhyreScene_setupPostProcessParams` (`0x0066f140`) copies its glow and
//! depth-of-field words into the scene context (`*(scene +0xcc)`), its
//! background colour becomes the scene's clear colour (`0x0065b8a0`).
//!
//! The offsets are the ones FFX.exe reads (object bytes as in the file).
//! The cluster's member table puts `PCamera`'s own members 4 bytes lower
//! (it leaves out the vtable pointer that starts the object), so
//! [`MEMBERS`] names each word by the member at `offset - 4`.

/// What FFX.exe reads from the `GlobalCamera`, by object offset.
pub const BG_COLOR: usize = 0xf4;
pub const GLOW_UPPER: usize = 0x104;
pub const GLOW_LOWER: usize = 0x108;
pub const GLOW_GAIN: usize = 0x10c;
pub const GLOW_SPREAD: usize = 0x110;
pub const DOF: usize = 0x114;

/// The member names of those words (`PCamera` in the map clusters).
pub const MEMBERS: [(usize, &str); 6] = [
    (BG_COLOR, "m_bgColor"),
    (GLOW_UPPER, "m_glowUpperThreshold"),
    (GLOW_LOWER, "m_glowLowerThreshold"),
    (GLOW_GAIN, "m_glowGain"),
    (GLOW_SPREAD, "m_glowSpread"),
    (DOF, "m_dofFocusPlaneDistance"),
];

/// One camera object of a cluster.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub path: String,
    pub class: String,
    pub index: u32,
    /// `m_bgColor` r, g, b.
    pub bg: [f32; 3],
    pub glow_upper: f32,
    pub glow_lower: f32,
    pub glow_gain: f32,
    pub glow_spread: f32,
    /// The three depth-of-field words (scene context `+0x90..+0x98`).
    pub dof: [f32; 3],
}

impl Camera {
    /// `NoGlowThreshold` as `setupPostProcessParams` makes it: the lower
    /// threshold, at least 0.35 (`0x3eb33333`).
    #[must_use]
    pub fn no_glow_threshold(&self) -> f32 {
        if self.glow_lower < 0.35 {
            f32::from_bits(0x3eb3_3333)
        } else {
            self.glow_lower
        }
    }
}

fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// Whether a class (or a base) has member `name` at byte `offset`.
fn has_member(c: &ffx_phyre::cluster::Cluster, class: usize, name: &str, offset: u32) -> bool {
    let mut k = Some(class);
    while let Some(i) = k {
        let cl = &c.classes[i];
        if cl
            .members
            .iter()
            .any(|m| m.name == name && m.offset == offset)
        {
            return true;
        }
        k = cl.base;
    }
    false
}

/// Every camera object of a cluster with the glow members.
#[must_use]
pub fn cameras(path: &str, bytes: &[u8]) -> Vec<Camera> {
    let Ok(cluster) = ffx_phyre::cluster::Cluster::parse(bytes) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for list in &cluster.lists {
        let ok = MEMBERS.iter().all(|&(at, name)| {
            u32::try_from(at - 4).is_ok_and(|o| has_member(&cluster, list.class, name, o))
        });
        if !ok {
            continue;
        }
        for index in 0..list.count {
            let object = list.object(index);
            let three = |at: usize| -> Option<[f32; 3]> {
                Some([
                    f32_at(bytes, object + at)?,
                    f32_at(bytes, object + at + 4)?,
                    f32_at(bytes, object + at + 8)?,
                ])
            };
            let word = |at: usize| f32_at(bytes, object + at);
            let (Some(bg), Some(dof)) = (three(BG_COLOR), three(DOF)) else {
                continue;
            };
            let (Some(glow_upper), Some(glow_lower), Some(glow_gain), Some(glow_spread)) = (
                word(GLOW_UPPER),
                word(GLOW_LOWER),
                word(GLOW_GAIN),
                word(GLOW_SPREAD),
            ) else {
                continue;
            };
            out.push(Camera {
                path: path.to_owned(),
                class: cluster.classes[list.class].name.clone(),
                index,
                bg,
                glow_upper,
                glow_lower,
                glow_gain,
                glow_spread,
                dof,
            });
        }
    }
    out
}

/// The cameras of every `.phyre` cluster of `archive` whose path contains
/// `filter` (a map folder such as `/map/cdsp/cdsp00/`).
///
/// # Errors
/// When the archive cannot be read.
pub fn map_cameras(
    archive: &ffx_vbf::Archive,
    filter: &str,
) -> Result<Vec<Camera>, ffx_vbf::Error> {
    let mut out = Vec::new();
    for e in archive.entries() {
        if e.path.contains(filter) && e.path.ends_with(".dae.phyre") {
            out.extend(cameras(&e.path, &archive.read(e)?));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_no_glow_threshold_is_at_least_0_35() {
        let mut c = Camera {
            path: String::new(),
            class: String::new(),
            index: 0,
            bg: [0.0; 3],
            glow_upper: 1.0,
            glow_lower: 0.1,
            glow_gain: 0.0,
            glow_spread: 0.0,
            dof: [0.0; 3],
        };
        assert_eq!(c.no_glow_threshold().to_bits(), 0x3eb3_3333);
        c.glow_lower = 0.5;
        assert!((c.no_glow_threshold() - 0.5).abs() < f32::EPSILON);
    }
}
