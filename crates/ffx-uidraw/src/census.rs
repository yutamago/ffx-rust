//! The census: every draw of a frame gets a family from its shaders'
//! origin, and every UI draw (Tk menus and windows, fonts, Iggy) its
//! producer, texture atlas, vertex source, states and constants, checked
//! against what the game's code builds (`docs/formats/ui-draw.md`).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ffx_drawdiff::state::{Blend, Depth, Raster};
use ffx_gfx::{DrawKind, StateKind};

use crate::frame::{Capture, DrawRec};
use crate::index::Index;

/// What a draw belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Family {
    /// Into the scene targets (`docs/formats/scene.md`).
    Scene,
    /// A Tk element without texture: `graphicDrawUIElement` with no
    /// texture name (`MenuDefaultShaderNT`).
    MenuPlain,
    /// A Tk element with a texture (`MenuDefaultShader`).
    MenuTextured,
    /// A Tk element sampling the frame buffer (`MenuDefaultShaderNA`,
    /// texture name `RealFrameBuffer`).
    MenuFrameBuffer,
    /// Other menu shader variants (`NB`, `NTNB`, `NTZTest`).
    MenuOther,
    /// A glyph run: `graphicFontDraw` (`OptimizedFontShader`).
    Font,
    /// The sphere grid's own shaders (`AbmapShader*`).
    Abmap,
    /// Iggy (Flash) through `GDraw`'s Direct3D 11 back end, whose shaders
    /// are compiled into `FFX.exe`.
    Iggy,
    /// The VFX shaders (`vfx*`, `phyredistortionshader`): the effects
    /// census (`docs/formats/effects-draw.md`).
    Effect,
    /// Any other archive shader: post-processing, copies, the movie
    /// player (`docs/formats/postfx.md`).
    Other,
    /// A shader of unknown origin.
    Unknown,
}

impl Family {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Scene => "scene",
            Self::MenuPlain => "menu-nt",
            Self::MenuTextured => "menu-tex",
            Self::MenuFrameBuffer => "menu-fb",
            Self::MenuOther => "menu-other",
            Self::Font => "font",
            Self::Abmap => "abmap",
            Self::Iggy => "iggy",
            Self::Effect => "effect",
            Self::Other => "other",
            Self::Unknown => "unknown",
        }
    }

    /// The families this census traces (the UI and 2D draws).
    #[must_use]
    pub fn is_ui(self) -> bool {
        matches!(
            self,
            Self::MenuPlain
                | Self::MenuTextured
                | Self::MenuFrameBuffer
                | Self::MenuOther
                | Self::Font
                | Self::Abmap
                | Self::Iggy
        )
    }

    /// The code that makes this family's draws.
    #[must_use]
    pub fn producer(self) -> &'static str {
        match self {
            Self::Scene => "PhyreScene_Draw3DWorld 0x0064fbb0",
            Self::MenuPlain | Self::MenuTextured | Self::MenuOther => {
                "graphicDrawUIElement 0x0063eee0 -> MenuManager_GetMenuVertexInfo 0x0067ba50; drawn by PhyreScene_DrawNormalUI 0x00650cb0"
            }
            Self::MenuFrameBuffer => {
                "graphicDrawUIElement 0x0063eee0 (texture \"RealFrameBuffer\") -> MenuManager_GetMenuVertexInfo; PhyreScene_DrawNormalUI"
            }
            Self::Font => {
                "graphicFontDraw 0x00640660 -> MenuManager_GetMenuVertexInfo (+0xbc = 1); PhyreScene_DrawNormalUI"
            }
            Self::Abmap => "graphicDrawUIAbmapElement 0x0063e930 / AbmapManager_GetVertexInfo",
            Self::Iggy => {
                "IggyMenuManager_Render_00a267f0 -> IggyMenu_Draw_00a28eb0 -> IggyPlayer_QueueDraw_00a3d9b0 (render thread) -> GDraw D3D11 (FFX.exe 0x00a34cc0..)"
            }
            Self::Effect => "effects (docs/formats/effects-draw.md)",
            Self::Other => "post-processing and the rest (docs/formats/postfx.md)",
            Self::Unknown => "?",
        }
    }
}

/// The family of a draw from its shaders' origins.
#[must_use]
pub fn family(scene: bool, vs: &str, ps: &str) -> Family {
    if scene {
        return Family::Scene;
    }
    let ps = ps.to_ascii_lowercase();
    let vs = vs.to_ascii_lowercase();
    let stem = |s: &str| s.split(".fx").next().unwrap_or(s).to_owned();
    let p = stem(&ps);
    // GDraw's shaders, or its stencil-only mask draws (no pixel shader).
    if vs.starts_with("ffx.exe") && (ps.starts_with("ffx.exe") || ps == "none") {
        return Family::Iggy;
    }
    match p.as_str() {
        "menudefaultshadernt" => Family::MenuPlain,
        "menudefaultshader" => Family::MenuTextured,
        "menudefaultshaderna" => Family::MenuFrameBuffer,
        "menudefaultshadernb" | "menudefaultshaderntnb" | "menudefaultshaderntztest" => {
            Family::MenuOther
        }
        "optimizedfontshader" => Family::Font,
        _ if p.starts_with("abmapshader") => Family::Abmap,
        _ if p.starts_with("vfx") || p == "phyredistortionshader" => Family::Effect,
        _ if p == "?" || p == "none" || vs == "?" => Family::Unknown,
        _ => Family::Other,
    }
}

/// One vertex of a UI quad as the menu code writes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

/// What the vertex check found for a menu or font draw.
#[derive(Debug, Clone, Default)]
pub struct Quads {
    /// Quads (six indices each) in the draw.
    pub quads: usize,
    /// Every quad indexes `b, b+1, b+3, b+3, b+1, b+2`, `b` a multiple of 4.
    pub pattern: bool,
    /// Every normal is (0, 0, 1) and every z 1 (as `graphicDrawUIElement`
    /// and `graphicFontDraw` store them).
    pub flat: bool,
    /// The bounding box in 1920 x 1080 pixels (x + 960, y + 540).
    pub rect: [f32; 4],
    /// Colour range (min, max) over all components.
    pub color: [f32; 2],
    /// UV range.
    pub uv: [f32; 4],
    /// The stream layout as (semantic, DXGI format, slot, offset).
    pub layout: Vec<(String, u32, u32, u32)>,
    /// The vertex buffers are dynamic rings.
    pub ring: bool,
}

fn f32_at(b: &[u8], at: usize) -> Option<f32> {
    b.get(at..at + 4)
        .map(|s| f32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Decodes the vertices a draw reads by its input layout: position,
/// normal, colour and UV floats.
#[must_use]
pub fn vertices(cap: &Capture, d: &DrawRec) -> Option<(Vec<Vertex>, Quads)> {
    let (_, elements) = cap.objects.layouts.get(&d.draw.layout)?;
    let mut q = Quads {
        layout: elements
            .iter()
            .map(|e| (e.semantic.clone(), e.format, e.slot, e.offset))
            .collect(),
        ..Quads::default()
    };
    let lo = *d.indices.iter().min()?;
    let hi = *d.indices.iter().max()?;
    let base = i64::from(d.draw.base);
    let mut out = Vec::new();
    q.ring = d.vertex.iter().all(|v| v.3);
    for i in lo..=hi {
        let mut v = Vertex {
            pos: [0.0; 3],
            normal: [0.0; 3],
            color: [0.0; 4],
            uv: [0.0; 2],
        };
        for e in elements {
            let (_, stride, bytes, _) = d.vertex.iter().find(|v| u32::from(v.0) == e.slot)?;
            let at = usize::try_from(
                (i64::from(i) + base - i64::try_from(d.first_vertex).ok()?) * i64::from(*stride),
            )
            .ok()?
                + e.offset as usize;
            let n = match e.format {
                2 => 4,
                6 => 3,
                16 => 2,
                _ => 0,
            };
            let mut vals = [0f32; 4];
            for (k, x) in vals.iter_mut().enumerate().take(n) {
                *x = f32_at(bytes, at + 4 * k)?;
            }
            match e.semantic.as_str() {
                "POSITION" => v.pos.copy_from_slice(&vals[..3]),
                "NORMAL" => v.normal.copy_from_slice(&vals[..3]),
                "COLOR" => v.color = vals,
                "TEXCOORD" if e.index == 0 => v.uv.copy_from_slice(&vals[..2]),
                _ => {}
            }
        }
        out.push(v);
    }
    q.quads = d.indices.len() / 6;
    q.pattern = d.indices.len().is_multiple_of(6)
        && d.indices.chunks(6).all(|c| {
            let b = c[0];
            b % 4 == 0 && c == [b, b + 1, b + 3, b + 3, b + 1, b + 2]
        });
    q.flat = out.iter().all(|v| {
        v.normal.map(f32::to_bits) == [0.0f32, 0.0, 1.0].map(f32::to_bits)
            && v.pos[2].to_bits() == 1.0f32.to_bits()
    });
    let mut rect = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    let mut color = [f32::MAX, f32::MIN];
    let mut uv = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for v in &out {
        rect = [
            rect[0].min(v.pos[0] + 960.0),
            rect[1].min(v.pos[1] + 540.0),
            rect[2].max(v.pos[0] + 960.0),
            rect[3].max(v.pos[1] + 540.0),
        ];
        for c in v.color {
            color = [color[0].min(c), color[1].max(c)];
        }
        uv = [
            uv[0].min(v.uv[0]),
            uv[1].min(v.uv[1]),
            uv[2].max(v.uv[0]),
            uv[3].max(v.uv[1]),
        ];
    }
    q.rect = rect;
    q.color = color;
    q.uv = uv;
    Some((out, q))
}

/// Where a texture a UI draw samples comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Atlas {
    /// None bound on a slot the shader reads.
    None,
    /// An archive texture cluster, from that level.
    Archive { path: String, level: u32 },
    /// A render target or a texture a copy wrote (its role).
    Target { role: u32, desc: String },
    /// A texture made at run time (not from a cluster): Iggy's glyph
    /// cache and bitmaps, the font atlas, and the like.
    Runtime { desc: String },
}

impl std::fmt::Display for Atlas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Archive { path, level } => {
                let short = path
                    .strip_prefix("ffx_data/gamedata/ps3data/")
                    .unwrap_or(path);
                write!(f, "{short}@{level}")
            }
            Self::Target { role, desc } => write!(f, "target#{role} {desc}"),
            Self::Runtime { desc } => write!(f, "runtime {desc}"),
        }
    }
}

/// The census of one UI draw.
#[derive(Debug, Clone)]
pub struct UiDraw {
    pub item: usize,
    pub family: Family,
    pub vs: String,
    pub ps: String,
    pub kind: DrawKind,
    pub count: u32,
    pub textures: Vec<(u8, Atlas)>,
    pub quads: Option<Quads>,
    /// Blend (enable, src, dest, op, src alpha, dest alpha, op alpha,
    /// write mask) of target 0.
    pub blend: [u32; 8],
    /// Depth enable, write mask, func.
    pub depth: [u32; 3],
    /// Cull mode and scissor enable.
    pub cull: u32,
    pub scissor: Option<[i32; 4]>,
    pub viewport: [f32; 6],
    /// Render target 0's role and its description.
    pub target: String,
    /// Used constants: `stage.cb<slot>.name` and the values (floats).
    pub constants: Vec<(String, Vec<f32>)>,
}

fn texture_desc(t: &crate::frame::Texture) -> String {
    format!("{}x{} f{}", t.width(), t.height(), t.format())
}

/// The atlas of one texture resource.
#[must_use]
pub fn atlas(cap: &Capture, index: &Index, roles: &BTreeMap<u32, u32>, res: u32) -> Atlas {
    if res == 0 {
        return Atlas::None;
    }
    let Some(t) = cap.textures.get(&res) else {
        return Atlas::Runtime {
            desc: "unknown object".into(),
        };
    };
    if t.target {
        return Atlas::Target {
            role: roles.get(&res).copied().unwrap_or(u32::MAX),
            desc: texture_desc(t),
        };
    }
    if let Some(s) = t
        .content()
        .and_then(|k| index.textures.get(&k))
        .and_then(|l| l.first())
    {
        return Atlas::Archive {
            path: s.path.clone(),
            level: s.level,
        };
    }
    Atlas::Runtime {
        desc: texture_desc(t),
    }
}

/// The used constants of a draw, by the shaders' reflection.
#[must_use]
pub fn constants(cap: &Capture, d: &DrawRec) -> Vec<(String, Vec<f32>)> {
    let mut out = Vec::new();
    for (stage, shader, windows) in [
        ("vs", d.draw.vs, &d.draw.vs_cb),
        ("ps", d.draw.ps, &d.draw.ps_cb),
    ] {
        let Some(s) = cap.objects.shaders.get(&shader) else {
            continue;
        };
        for (slot, window) in windows {
            let Some(c) = s.layout.decode(stage, *slot, window) else {
                continue;
            };
            for v in &c.vars {
                let vals = v
                    .words
                    .iter()
                    .map(|(w, _)| w.map_or(f32::NAN, f32::from_bits))
                    .collect();
                out.push((format!("{stage}.cb{slot}.{}", v.name), vals));
            }
        }
    }
    out
}

/// Every draw of a frame with its family; the UI draws in detail.
#[must_use]
pub fn census(cap: &Capture, index: &Index, frame: &[DrawRec]) -> (Vec<Family>, Vec<UiDraw>) {
    // Target roles in order of first use (as the frame walk numbered them).
    let mut roles = BTreeMap::new();
    for d in frame {
        if let Some((res, role)) = d.target {
            roles.entry(res).or_insert(role);
        }
    }
    let mut families = Vec::new();
    let mut ui = Vec::new();
    for d in frame {
        let vs = index.vertex_shader(d.vs, d.ps).to_owned();
        let ps = index.shader(d.ps).to_owned();
        let fam = family(d.scene, &vs, &ps);
        families.push(fam);
        if !fam.is_ui() {
            continue;
        }
        let used: Vec<u8> = cap
            .objects
            .shaders
            .get(&d.draw.ps)
            .map(|s| {
                d.ps_textures
                    .iter()
                    .map(|t| t.0)
                    .filter(|&slot| s.layout.uses_texture(slot))
                    .collect()
            })
            .unwrap_or_default();
        let textures = d
            .ps_textures
            .iter()
            .filter(|t| used.contains(&t.0))
            .map(|&(slot, res)| (slot, atlas(cap, index, &roles, res)))
            .collect();
        let quads = if matches!(fam, Family::Iggy) {
            None
        } else {
            vertices(cap, d).map(|v| v.1)
        };
        let blend = Blend::decode(cap.state(d.draw.blend, StateKind::Blend));
        let depth = Depth::decode(cap.state(d.draw.depth, StateKind::Depth));
        let raster = Raster::decode(cap.state(d.draw.raster, StateKind::Raster));
        let target = d.target.map_or_else(
            || "none".into(),
            |(res, role)| {
                cap.textures.get(&res).map_or_else(
                    || format!("#{role}"),
                    |t| format!("#{role} {}", texture_desc(t)),
                )
            },
        );
        ui.push(UiDraw {
            item: d.item,
            family: fam,
            vs,
            ps,
            kind: d.draw.kind,
            count: d.draw.count,
            textures,
            quads,
            blend: blend.rt[0],
            depth: [depth.enable, depth.write_mask, depth.func],
            cull: raster.v[1],
            scissor: raster.scissor().then_some(d.draw.scissor),
            viewport: d.draw.viewport,
            target,
            constants: constants(cap, d),
        });
    }
    (families, ui)
}

/// Whether a UI draw is fully traced: a known producer, every sampled
/// texture placed, and (menu and font draws) the quads as the menu code
/// writes them.
///
/// # Errors
/// What is missing, as text.
pub fn traced(u: &UiDraw) -> Result<(), String> {
    if !u.family.is_ui() {
        return Err("not a UI draw".into());
    }
    for (slot, a) in &u.textures {
        if *a == Atlas::None {
            return Err(format!("texture slot {slot} empty"));
        }
    }
    match u.family {
        Family::Iggy | Family::Abmap => Ok(()),
        _ => {
            let q = u.quads.as_ref().ok_or("no vertices")?;
            if !q.pattern {
                return Err("indices are not the menu code's quads".into());
            }
            if !q.flat {
                return Err("normals or z are not the menu code's".into());
            }
            Ok(())
        }
    }
}

/// A text report: one line per UI draw, then per-family totals.
#[must_use]
pub fn report(families: &[Family], ui: &[UiDraw]) -> String {
    let mut s = String::new();
    let mut totals: BTreeMap<Family, usize> = BTreeMap::new();
    for f in families {
        *totals.entry(*f).or_default() += 1;
    }
    for u in ui {
        let tex: Vec<String> = u
            .textures
            .iter()
            .map(|(s, a)| format!("t{s}={a}"))
            .collect();
        let q = u.quads.as_ref().map_or_else(String::new, |q| {
            format!(
                " quads {}{}{} rect {:.0},{:.0}-{:.0},{:.0} colour {:.3}..{:.3}{}",
                q.quads,
                if q.pattern { "" } else { " (pattern!)" },
                if q.flat { "" } else { " (not flat!)" },
                q.rect[0],
                q.rect[1],
                q.rect[2],
                q.rect[3],
                q.color[0],
                q.color[1],
                if q.ring { " ring" } else { " static" },
            )
        });
        let b = u.blend;
        let blend = if b[0] == 0 {
            "off".to_owned()
        } else {
            format!("{}/{}/{} a {}/{}/{}", b[1], b[2], b[3], b[4], b[5], b[6])
        };
        let _ = writeln!(
            s,
            "{:04} {:<10} {:?} {} rt {} {}{} blend {} depth {}/{}/{} cull {} sc {}",
            u.item,
            u.family.name(),
            u.kind,
            u.count,
            u.target,
            tex.join(" "),
            q,
            blend,
            u.depth[0],
            u.depth[1],
            u.depth[2],
            u.cull,
            u.scissor.map_or_else(|| "off".into(), |r| format!("{r:?}")),
        );
        if let Err(e) = traced(u) {
            let _ = writeln!(s, "     NOT TRACED: {e}");
        }
    }
    let _ = writeln!(s, "\nfamilies:");
    for (f, n) in &totals {
        let _ = writeln!(s, "  {:<10} {n:>4}  {}", f.name(), f.producer());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffx_gfx::{Cmd, Draw, Element, Frame, Msg};

    fn floats(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    /// One quad as `graphicDrawUIElement` stores it: positions in the
    /// centred 1920 x 1080 space at z 1, normals (0, 0, 1), colours / 255
    /// (alpha x 2), UVs with v flipped, indices b, b+1, b+3, b+3, b+1, b+2.
    fn quad_capture(indices: &[u16], normal_z: f32) -> Capture {
        let pos = floats(&[
            -960.0, -540.0, 1.0, -860.0, -540.0, 1.0, -860.0, -440.0, 1.0, -960.0, -440.0, 1.0,
        ]);
        let nrm = floats(&[
            0.0, 0.0, normal_z, 0.0, 0.0, normal_z, 0.0, 0.0, normal_z, 0.0, 0.0, normal_z,
        ]);
        let col = floats(&[0.5; 16]);
        let uv = floats(&[0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
        let ib: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        let ring = |id, data: Vec<u8>, bind| Msg::Buffer {
            id,
            desc: [u32::try_from(data.len()).unwrap(), 2, bind, 0x10000, 0, 0],
            data,
        };
        let el = |semantic: &str, format, slot| Element {
            semantic: semantic.into(),
            index: 0,
            format,
            slot,
            offset: 0,
            per_instance: false,
            step: 0,
        };
        let draw = Draw {
            kind: DrawKind::Indexed,
            count: u32::try_from(indices.len()).unwrap(),
            instances: 1,
            start: 0,
            base: 0,
            first_instance: 0,
            topology: 4,
            layout: 9,
            vs: 0,
            ps: 0,
            vertex: vec![(0, 1, 12, 0), (1, 2, 12, 0), (2, 3, 16, 0), (3, 4, 8, 0)],
            index: (5, 57, 0),
            vs_cb: Vec::new(),
            ps_cb: Vec::new(),
            vs_srv: Vec::new(),
            ps_srv: Vec::new(),
            vs_samplers: Vec::new(),
            ps_samplers: Vec::new(),
            blend: 0,
            blend_factor: [0.0; 4],
            sample_mask: u32::MAX,
            depth: 0,
            stencil_ref: 0,
            raster: 0,
            viewport: [0.0, 0.0, 1280.0, 720.0, 0.0, 1.0],
            scissor: [0, 0, 1280, 720],
            rtv: vec![(0, 0)],
            dsv: 0,
        };
        Capture::from_msgs(&[
            ring(1, pos, 1),
            ring(2, nrm, 1),
            ring(3, col, 1),
            ring(4, uv, 1),
            ring(5, ib, 2),
            Msg::Layout {
                id: 9,
                elements: vec![
                    el("POSITION", 6, 0),
                    el("NORMAL", 6, 1),
                    el("COLOR", 2, 2),
                    el("TEXCOORD", 16, 3),
                ],
            },
            Msg::Frame(Frame {
                number: 0,
                width: 1280,
                height: 720,
                cmds: vec![Cmd::Draw(Box::new(draw))],
            }),
        ])
    }

    #[test]
    fn a_menu_quad_decodes() {
        let cap = quad_capture(&[0, 1, 3, 3, 1, 2], 1.0);
        let d = &cap.frames[0][0];
        let (v, q) = vertices(&cap, d).expect("vertices");
        assert_eq!(v.len(), 4);
        assert_eq!(q.quads, 1);
        assert!(q.pattern && q.flat && q.ring);
        assert_eq!(q.rect, [0.0, 0.0, 100.0, 100.0]);
        assert_eq!(q.color, [0.5, 0.5]);
        assert_eq!(q.uv, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn other_indices_or_normals_are_not_the_menu_code() {
        let cap = quad_capture(&[0, 1, 2, 2, 3, 0], 1.0);
        let q = vertices(&cap, &cap.frames[0][0]).expect("vertices").1;
        assert!(!q.pattern && q.flat);
        let cap = quad_capture(&[0, 1, 3, 3, 1, 2], -1.0);
        let q = vertices(&cap, &cap.frames[0][0]).expect("vertices").1;
        assert!(q.pattern && !q.flat);
    }
}
