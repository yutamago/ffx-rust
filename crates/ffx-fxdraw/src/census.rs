//! The census: every draw of a captured frame with its producer
//! (`docs/formats/effects-draw.md`).
//!
//! Effect draws are traced through the `PhyreScene` VFX lists of the
//! snapshot taken for the frame:
//! - each list's mesh instances, in order, give the draws `PhyreEngine`
//!   makes for them: one per segment of the instance's `VFXDynamicGeometry`
//!   render data with indices (`renderMeshInstance`), in segment order;
//! - the captured effect draws (the VFX vertex layout or a `vfx*.fx`
//!   shader) are aligned with that sequence (longest common subsequence),
//!   a pair counting only when the draw's ring bytes are the segment's
//!   arrays: positions, normals, colours, uvs (one of the segment's buffer
//!   slots) and the indices relative to the smallest;
//! - the geometry's key names the ppp object and program.
//!
//! The rest is told apart by targets and shader family: scene draws (two
//! render targets and a depth target), post-processing passes and the UI.

use std::fmt::Write as _;

use crate::capture::{Capture, DrawRec};
use crate::game::{Geometry, List, Maker, Overlay, State};
use crate::mem::Mem;
use crate::shaders::Families;

/// Bytes per vertex of the four VFX arrays and the slot each one is bound
/// to: positions (slot 0), normals (1), colours (2), uvs (3).
pub const VFX_STRIDES: [u32; 4] = [12, 12, 16, 8];

/// Who made a draw.
#[derive(Debug, Clone, PartialEq)]
pub enum Producer {
    /// An effect instance's dynamic geometry.
    Vfx(VfxDraw),
    /// The 3D scene (map, characters, monsters): `docs/formats/scene.md`.
    Scene,
    /// A post-processing pass (`PostProcessManager`, glow, copies).
    Post,
    /// The UI (menus, fonts, the PC UI).
    Ui,
    /// A 3D world drawn into the colour target alone after the glow
    /// (`PhyreScene_render`: the world at scene `+0x9c` in Opaque,
    /// Transparent and postTransparent, and the postOpaque and
    /// postTransparent passes): the scene builder's.
    LateWorld,
    /// An effect draw that no list entry explains.
    UnmatchedEffect(String),
    /// Nothing above.
    Unknown(String),
}

impl Producer {
    #[must_use]
    pub fn class(&self) -> &'static str {
        match self {
            Self::Vfx(_) => "vfx",
            Self::Scene => "scene",
            Self::Post => "post",
            Self::Ui => "ui",
            Self::LateWorld => "late-world",
            Self::UnmatchedEffect(_) => "UNMATCHED-EFFECT",
            Self::Unknown(_) => "unknown",
        }
    }
}

/// An effect draw traced to its geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct VfxDraw {
    /// The list and the mesh instance's place in it.
    pub list: List,
    pub place: usize,
    pub geometry: u32,
    pub key: u64,
    pub segment: usize,
    /// 0: the segment's own arrays, 1 and 2: its buffer slots.
    pub buffer: usize,
    /// Who made the instance, from its key.
    pub maker: Maker,
    /// The shader `vfxShader_fx.txt` gives the flags, when the table has
    /// them, and whether the draw's pixel shader is that file's.
    pub table_shader: Option<(String, bool)>,
    /// The geometry's VSP flags (`+0x3c`, `vfxShader_fx.txt`).
    pub flags: u32,
    /// The list entry is 0 in the snapshot and the geometry sits in the
    /// table's delete pool: destroyed (`graphicVFXDestroy`;
    /// `PhyreScene_removeVFXFromRenderArray` zeroes the entry) after the
    /// frame was drawn; `place` is the zeroed entry between its
    /// neighbours' places.
    pub removed: bool,
}

/// One draw and its producer.
#[derive(Debug, Clone)]
pub struct Line {
    pub n: usize,
    pub vs_family: String,
    pub ps_family: String,
    pub producer: Producer,
}

/// A draw the lists say `PhyreEngine` makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expected {
    pub list: List,
    pub place: usize,
    /// Index into [`State::geometries`].
    pub geometry: usize,
    pub segment: usize,
    pub indices: u32,
}

/// The census of a frame: the lines and the list entries no draw took.
#[derive(Debug, Clone, Default)]
pub struct Census {
    pub lines: Vec<Line>,
    /// Expected draws that no captured draw matched.
    pub missing: Vec<Expected>,
    /// List entries (mesh instances) without a geometry in the table.
    pub orphans: Vec<(List, usize, u32)>,
}

/// Whether a shader family is an effect shader.
#[must_use]
pub fn is_vfx_family(f: &str) -> bool {
    f.starts_with("vfx")
}

/// Post-processing families (`PostProcessManager`, the glow chain).
#[must_use]
pub fn is_post_family(f: &str) -> bool {
    const POST: [&str; 11] = [
        "phyreglow",
        "phyreblur",
        "copybuffer",
        "phyredistortionshader",
        "brightnessshader",
        "screenfade",
        "phyremotionblur",
        "phyrehdao",
        "phyredof",
        "crossfade",
        "postprocess",
    ];
    POST.iter().any(|p| f.starts_with(p))
}

/// UI families (menus, fonts).
#[must_use]
pub fn is_ui_family(f: &str) -> bool {
    f.starts_with("menu") || f.starts_with("font") || f.starts_with("optimizedfont")
}

/// The VFX vertex layout: four ring slots with the VFX strides.
#[must_use]
pub fn has_vfx_layout(d: &DrawRec) -> bool {
    d.slots.len() == 4
        && d.slots
            .iter()
            .zip(VFX_STRIDES)
            .all(|(s, stride)| s.ring && s.stride == stride && !s.per_instance)
}

fn slot_bytes(d: &DrawRec, slot: u8) -> Option<&[u8]> {
    d.slots
        .iter()
        .find(|s| s.slot == slot)
        .map(|s| s.bytes.as_slice())
}

/// The draws the lists ask for, in `PhyreScene_render`'s order of the
/// passes: `DrawMostVFX`, then the others.
#[must_use]
pub fn expected(state: &State) -> (Vec<Expected>, Vec<(List, usize, u32)>) {
    let mut out = Vec::new();
    let mut orphans = Vec::new();
    for (list, entries) in &state.lists {
        for (place, &mi) in entries.iter().enumerate() {
            if mi == 0 {
                continue;
            }
            let Some(gi) = state
                .geometries
                .iter()
                .position(|g| !g.pooled && g.mesh_instance == mi)
            else {
                orphans.push((*list, place, mi));
                continue;
            };
            for (si, seg) in state.geometries[gi].segments.iter().enumerate() {
                if seg.indices > 0 {
                    out.push(Expected {
                        list: *list,
                        place,
                        geometry: gi,
                        segment: si,
                        indices: seg.indices,
                    });
                }
            }
        }
    }
    (out, orphans)
}

/// Whether the draw reads segment `si`'s arrays of one buffer slot: every
/// vertex slot's bytes from the draw's first record, and the indices
/// relative to their smallest. Returns the slot.
fn bytes_match(m: &Mem, d: &DrawRec, g: &Geometry, si: usize) -> Option<usize> {
    let seg = g.segments.get(si)?;
    let first = d.slots.first().map_or(0, |s| s.first);
    let ix = d.indices.as_ref()?;
    if u32::try_from(ix.len()).ok()? != seg.indices {
        return None;
    }
    seg.buffers.iter().position(|a| {
        if a.positions == 0 || a.index == 0 {
            return false;
        }
        let same = |slot: u8, base: u32, stride: u32| -> bool {
            slot_bytes(d, slot).is_some_and(|b| {
                !b.is_empty() && m.bytes(base + stride * first, b.len()) == Some(b)
            })
        };
        if !(same(0, a.positions, 12)
            && same(1, a.normals, 12)
            && same(2, a.colours, 16)
            && same(3, a.uvs, 8))
        {
            return false;
        }
        let Some(raw) = m.bytes(a.index, 2 * ix.len()) else {
            return false;
        };
        let theirs: Vec<u32> = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u32::from(u16::from_le_bytes(*c)))
            .collect();
        let lo_a = ix.iter().copied().min().unwrap_or(0);
        let lo_b = theirs.iter().copied().min().unwrap_or(0);
        ix.iter()
            .zip(&theirs)
            .all(|(x, y)| x.wrapping_sub(lo_a) == y.wrapping_sub(lo_b))
    })
}

/// Aligns the effect draws with the expected sequence (LCS on byte
/// matches): pairs of (draw index in `draws`, expected index, slot).
#[allow(clippy::many_single_char_names)]
fn align(
    m: &Mem,
    state: &State,
    draws: &[&DrawRec],
    exp: &[Expected],
) -> Vec<(usize, usize, usize)> {
    let (n, k) = (draws.len(), exp.len());
    let mut hit = vec![None; n * k];
    for (i, d) in draws.iter().enumerate() {
        for (j, e) in exp.iter().enumerate() {
            hit[i * k + j] = bytes_match(m, d, &state.geometries[e.geometry], e.segment);
        }
    }
    // lcs[i][j]: the longest common subsequence of draws[i..] and exp[j..]
    let mut lcs = vec![0u32; (n + 1) * (k + 1)];
    for i in (0..n).rev() {
        for j in (0..k).rev() {
            let v = if hit[i * k + j].is_some() {
                lcs[(i + 1) * (k + 1) + j + 1] + 1
            } else {
                lcs[(i + 1) * (k + 1) + j].max(lcs[i * (k + 1) + j + 1])
            };
            lcs[i * (k + 1) + j] = v;
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < k {
        if let Some(slot) = hit[i * k + j]
            && lcs[i * (k + 1) + j] == lcs[(i + 1) * (k + 1) + j + 1] + 1
        {
            out.push((i, j, slot));
            i += 1;
            j += 1;
        } else if lcs[(i + 1) * (k + 1) + j] >= lcs[i * (k + 1) + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// The census of one frame. `overlay`: the playing spell's file, to name
/// programs whose definitions sit in it.
#[must_use]
pub fn census(
    cap: &Capture,
    frame: usize,
    m: &Mem,
    fam: &Families,
    overlay: Option<&Overlay>,
) -> Census {
    let state = State::read(m).unwrap_or_default();
    let Some(f) = cap.frames.get(frame) else {
        return Census::default();
    };
    let families: Vec<(String, String)> = f
        .draws
        .iter()
        .map(|d| (fam.name(d.vs).to_owned(), fam.name(d.ps).to_owned()))
        .collect();
    let is_effect = |i: usize| {
        let (vs, ps) = &families[i];
        has_vfx_layout(&f.draws[i]) || is_vfx_family(vs) || is_vfx_family(ps)
    };
    let effect: Vec<usize> = (0..f.draws.len()).filter(|&i| is_effect(i)).collect();
    let effect_draws: Vec<&DrawRec> = effect.iter().map(|&i| &f.draws[i]).collect();
    let (exp, orphans) = expected(&state);
    let pairs = align(m, &state, &effect_draws, &exp);
    let mut producers: Vec<Option<Producer>> = vec![None; f.draws.len()];
    let mut taken = vec![false; exp.len()];
    for &(di, ej, slot) in &pairs {
        let e = exp[ej];
        taken[ej] = true;
        let g = &state.geometries[e.geometry];
        producers[effect[di]] = Some(Producer::Vfx(VfxDraw {
            list: e.list,
            place: e.place,
            geometry: g.addr,
            key: g.key,
            segment: e.segment,
            buffer: slot,
            maker: Maker::of(m, overlay, g),
            flags: m.u32(g.addr + 0x3c).unwrap_or(0),
            table_shader: None,
            removed: false,
        }));
    }
    removed_draws(
        m,
        &state,
        overlay,
        &effect,
        &effect_draws,
        &pairs,
        &exp,
        &mut producers,
    );
    let lines = f
        .draws
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let (vs_family, ps_family) = families[i].clone();
            if let Some(Producer::Vfx(v)) = &mut producers[i] {
                v.table_shader = fam
                    .vsp
                    .get(&v.flags)
                    .map(|want| (want.clone(), *want == ps_family));
            }
            let producer = producers[i].take().unwrap_or_else(|| {
                let vfx_shader = is_vfx_family(&vs_family) || is_vfx_family(&ps_family);
                if vfx_shader
                    || (has_vfx_layout(d)
                        && classify_other(d, &vs_family, &ps_family).class() == "unknown")
                {
                    Producer::UnmatchedEffect(format!(
                        "{} indices, {} vertices from {}",
                        d.draw.count,
                        slot_bytes(d, 0).map_or(0, |b| b.len() / 12),
                        d.slots.first().map_or(0, |s| s.first)
                    ))
                } else {
                    classify_other(d, &vs_family, &ps_family)
                }
            });
            Line {
                n: d.n,
                vs_family,
                ps_family,
                producer,
            }
        })
        .collect();
    Census {
        lines,
        missing: exp
            .iter()
            .zip(&taken)
            .filter(|(_, t)| !**t)
            .map(|(e, _)| *e)
            .collect(),
        orphans,
    }
}

/// Effect draws the alignment left: a geometry of the delete pool whose
/// segment bytes the draw reads, at a zeroed `DrawMostVFX` entry between
/// the places of the aligned draws before and after it.
#[allow(clippy::too_many_arguments)]
fn removed_draws(
    m: &Mem,
    state: &State,
    overlay: Option<&Overlay>,
    effect: &[usize],
    draws: &[&DrawRec],
    pairs: &[(usize, usize, usize)],
    exp: &[Expected],
    producers: &mut [Option<Producer>],
) {
    let most: &[u32] = state
        .lists
        .iter()
        .find(|(l, _)| *l == List::Most)
        .map_or(&[], |(_, v)| v.as_slice());
    let mut used: Vec<usize> = Vec::new();
    for (di, d) in draws.iter().enumerate() {
        if producers[effect[di]].is_some() {
            continue;
        }
        // the places of the aligned Most draws around it
        let before = pairs
            .iter()
            .rev()
            .find(|p| p.0 < di && exp[p.1].list == List::Most)
            .map_or(0, |p| exp[p.1].place);
        let after = pairs
            .iter()
            .find(|p| p.0 > di && exp[p.1].list == List::Most)
            .map_or(most.len(), |p| exp[p.1].place + 1);
        let zero = (before..after.min(most.len())).find(|&i| most[i] == 0 && !used.contains(&i));
        let Some(zero) = zero else { continue };
        let hit = state.geometries.iter().filter(|g| g.pooled).find_map(|g| {
            (0..g.segments.len()).find_map(|si| bytes_match(m, d, g, si).map(|slot| (g, si, slot)))
        });
        if let Some((g, si, slot)) = hit {
            used.push(zero);
            producers[effect[di]] = Some(Producer::Vfx(VfxDraw {
                list: List::Most,
                place: zero,
                geometry: g.addr,
                key: g.key,
                segment: si,
                buffer: slot,
                maker: Maker::of(m, overlay, g),
                flags: m.u32(g.addr + 0x3c).unwrap_or(0),
                table_shader: None,
                removed: true,
            }));
        }
    }
}

fn classify_other(d: &DrawRec, vs: &str, ps: &str) -> Producer {
    if is_ui_family(vs) || is_ui_family(ps) {
        Producer::Ui
    } else if is_post_family(vs) || is_post_family(ps) {
        Producer::Post
    } else if d.is_scene() {
        Producer::Scene
    } else if d.depth_target != 0 && (vs.ends_with("litshader.fx") || ps.ends_with("litshader.fx"))
    {
        Producer::LateWorld
    } else {
        Producer::Unknown(format!("{vs}/{ps}"))
    }
}

impl Census {
    /// Draws per producer class.
    #[must_use]
    pub fn count(&self, class: &str) -> usize {
        self.lines
            .iter()
            .filter(|l| l.producer.class() == class)
            .count()
    }

    /// Effect draws whose pixel shader is not the one `vfxShader_fx.txt`
    /// gives their geometry's flags.
    #[must_use]
    pub fn shader_mismatches(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| {
                matches!(&l.producer, Producer::Vfx(v) if matches!(v.table_shader, Some((_, false))))
            })
            .count()
    }

    /// Effect draws whose flags the table does not list (a special shader:
    /// `VfxShaderManager`'s per-texture table of the level or spell).
    #[must_use]
    pub fn shader_unlisted(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| matches!(&l.producer, Producer::Vfx(v) if v.table_shader.is_none()))
            .count()
    }

    /// The gate: every effect draw traced (no unmatched effect draw, no
    /// unknown draw), and every traced one drawn with its flags' shader.
    #[must_use]
    pub fn traced(&self) -> bool {
        self.count("UNMATCHED-EFFECT") == 0
            && self.count("unknown") == 0
            && self.shader_mismatches() == 0
    }

    /// The report: one line per draw, the counts, the list entries no draw
    /// took.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn report(&self, state: &State, m: &Mem) -> String {
        let mut out = String::new();
        for l in &self.lines {
            let _ = write!(
                out,
                "{:4} {:8} {}/{}",
                l.n,
                l.producer.class(),
                l.vs_family,
                l.ps_family
            );
            match &l.producer {
                Producer::Vfx(v) => {
                    let prog = v.maker.label(m);
                    let _ = write!(
                        out,
                        " | {}[{}]{} geom {:#x} key {:#018x} seg {} buf {} flags {:#x}{} | {prog}",
                        v.list.name(),
                        v.place,
                        if v.removed {
                            " (removed, delete pool)"
                        } else {
                            ""
                        },
                        v.geometry,
                        v.key,
                        v.segment,
                        v.buffer,
                        v.flags,
                        match &v.table_shader {
                            Some((_, true)) => String::new(),
                            Some((want, false)) => format!(" SHADER (table: {want})"),
                            None => " (flags not in the table)".into(),
                        },
                    );
                }
                Producer::UnmatchedEffect(s) | Producer::Unknown(s) => {
                    let _ = write!(out, " | {s}");
                }
                _ => {}
            }
            out.push('\n');
        }
        let _ = writeln!(
            out,
            "draws {}: vfx {}, scene {}, late-world {}, post {}, ui {}, unknown {}, UNMATCHED-EFFECT {}",
            self.lines.len(),
            self.count("vfx"),
            self.count("scene"),
            self.count("late-world"),
            self.count("post"),
            self.count("ui"),
            self.count("unknown"),
            self.count("UNMATCHED-EFFECT")
        );
        let makers = |c: &str| {
            self.lines
                .iter()
                .filter(|l| matches!(&l.producer, Producer::Vfx(v) if v.maker.class() == c))
                .count()
        };
        let _ = writeln!(
            out,
            "effect shaders: {} not the table's, {} with flags the table lacks",
            self.shader_mismatches(),
            self.shader_unlisted()
        );
        let _ = writeln!(
            out,
            "effect makers: ppp {}, ppp (index names a program without a draw) {}, ppp (program list not readable) {}, op {}, other {}",
            makers("ppp"),
            makers("ppp-nodraw"),
            makers("ppp?"),
            makers("op"),
            makers("other")
        );
        let _ = writeln!(
            out,
            "expected draws not drawn: {}; list entries without a geometry: {}",
            self.missing.len(),
            self.orphans.len()
        );
        for e in &self.missing {
            let g = &state.geometries[e.geometry];
            let _ = writeln!(
                out,
                "  not drawn: {}[{}] geom {:#x} key {:#018x} seg {} ({} indices)",
                e.list.name(),
                e.place,
                g.addr,
                g.key,
                e.segment,
                e.indices
            );
        }
        for (l, p, mi) in &self.orphans {
            let _ = writeln!(
                out,
                "  no geometry: {}[{p}] mesh instance {mi:#x}",
                l.name()
            );
        }
        out
    }
}
