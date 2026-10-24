//! The census of a captured frame's scene draws: for each draw, the
//! cluster objects it comes from (by content key), its pass, its shader
//! family, where each constant it uses comes from, and a verdict on every
//! static object it binds (`docs/formats/scene.md`).

// Matrix and box arithmetic names its terms as the formulas do (w, v, p,
// c), and the culling census keeps its draw lists side by side.
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ffx_gfx::{Draw, StateKind};

use crate::assets::{AssetIndex, Source};
use crate::capture::Capture;
use crate::key;

/// The scene pass a draw belongs to, from its blend and depth state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pass {
    /// No blending on the colour target, depth written.
    Opaque,
    /// Blending on, or depth not written.
    Transparent,
}

/// What a bound object is, against the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Its key is the key of these cluster objects.
    Matched(Vec<Source>),
    /// A `D3D11_USAGE_DYNAMIC` buffer: a ring the CPU refills each frame.
    Dynamic,
    /// A static buffer of a skinned draw that is not the file's block: the
    /// game rebuilds character geometry at load.
    Skinned,
    /// A render target or depth buffer the frame (or an earlier one) drew.
    RenderTarget,
    /// A texture the game makes at run time (not a render target).
    Generated,
    /// A texture whose top level has the texels of these file levels, but
    /// whose key differs: the description (levels, format) is not the one
    /// the asset side gives it.
    Texels(Vec<Source>),
    /// None of the above.
    Unexplained,
}

impl Verdict {
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Matched(_) => "matched",
            Self::Dynamic => "dynamic",
            Self::Skinned => "skinned",
            Self::RenderTarget => "render-target",
            Self::Generated => "generated",
            Self::Texels(_) => "texels-only",
            Self::Unexplained => "unexplained",
        }
    }
}

/// One bound object of a draw.
#[derive(Debug, Clone)]
pub struct Bound {
    /// `vb<slot>`, `ib`, `ps-t<slot>`, `vs`, `ps`, `layout`.
    pub role: String,
    pub id: u32,
    pub key: Option<u64>,
    pub verdict: Verdict,
}

/// Where a constant's value comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstSource {
    /// A tweakable parameter of the segment's material, as stored in the
    /// cluster (`equal`: the captured value is the stored one).
    Material { equal: bool },
    /// Computed by `PhyreEngine` from other values (the text says from what).
    Derived(&'static str),
    /// Written by the game's code (the text names the state).
    GameState(&'static str),
    /// Not classified yet.
    Unknown,
}

impl ConstSource {
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Material { equal: true } => "material".into(),
            Self::Material { equal: false } => "material (changed at run time)".into(),
            Self::Derived(how) => format!("derived: {how}"),
            Self::GameState(what) => format!("game state: {what}"),
            Self::Unknown => "unknown".into(),
        }
    }

    /// The class used in the summary tables.
    #[must_use]
    pub fn class(&self) -> &'static str {
        match self {
            Self::Material { equal: true } => "material",
            Self::Material { equal: false } => "material*",
            Self::Derived(_) => "derived",
            Self::GameState(_) => "game-state",
            Self::Unknown => "unknown",
        }
    }
}

/// One used constant of a draw.
#[derive(Debug, Clone)]
pub struct Constant {
    /// `v` or `p`.
    pub stage: char,
    /// `World`, or `scene.ViewProjection` for a struct member.
    pub name: String,
    pub offset: u32,
    pub size: u32,
    pub source: ConstSource,
}

/// One scene draw.
#[derive(Debug, Clone)]
pub struct DrawCensus {
    /// Index among the frame's commands, and among its scene draws.
    pub cmd: usize,
    pub number: usize,
    pub pass: Pass,
    pub vs_family: String,
    pub ps_family: String,
    pub skinned: bool,
    /// (model path, segment) the vertex buffers come from.
    pub segment: Option<(String, usize)>,
    pub instances: Vec<String>,
    pub bound: Vec<Bound>,
    pub constants: Vec<Constant>,
    /// View-space depth of the segment's bounds centre (camera looks down
    /// -z: smaller is farther), when the draw has `World` and `scene.View`.
    pub view_z: Option<f32>,
    /// The instance bounds' `m_render_order`.
    pub render_order: Option<i32>,
    pub index_count: u32,
    /// How the input layout's elements map to the file's streams.
    pub layout_pairs: Vec<LayoutPair>,
}

/// The names `PhyreEngine` computes per draw or per frame (its parameter
/// semantics), and what from. Filled from FFX.exe
/// (`docs/formats/scene.md`, "Constants").
fn derived(name: &str) -> Option<&'static str> {
    Some(match name {
        "WorldView" => "modelview 0x83: View . World, f32 sums (node buffer 0x005b7fe0)",
        "WorldViewProjection" => {
            "modelviewproj 0x84: ViewProjection . World, f64 sums rounded once (node buffer)"
        }
        "BoneTransforms" => "boneTransforms 0x8f: joint world . inverse bind, per m_skinBones",
        "scene.EyePosition" => "EyePosWS: the camera world's translation",
        "scene.View" => "view 3: FromMatrix3x4(PCamera +0), the inverse of scene +0xfa04",
        "scene.Projection" => {
            "projection 2: PCamera +0x30 in 0..1 depth (fov +0x154, aspect +0x150, near +0xb8, far +0xbc)"
        }
        "scene.ViewProjection" => "viewprojection 6: Projection . View",
        "scene.ViewInverse" => "viewi 4: the camera world 3x4 (scene +0xfa04)",
        "scene.cameraNearFar" | "scene.cameraNearTimesFar" | "scene.cameraFarMinusNear" => {
            "NearFarPlane, NearTimesFar, FarMinusNear: camera +0xb8/+0xbc"
        }
        "scene.ViewportWidthHeight" => "ViewportWidthHeight: the viewport",
        "scene.screenWidthHeightInv" => "ScreenWidthHeightInv: 0x00c95100/0x00c95104",
        _ => return None,
    })
}

fn game_state(name: &str) -> Option<&'static str> {
    Some(match name {
        "World" => {
            "model 0x82: the instance's world matrix as the game sets it (map: 10 x unit, map objects PhyreScene_setObjectRotation/Translation 0x0066db00/0x0066e3e0, characters setMeshInstanceWorldMatrix 0x0066d810)"
        }
        "scene.GlobalAmbientColor" => "g_PhyreApplication +0x50 (graphicSetGlobalAmbientColor)",
        "scene.FogColor" => "g_PhyreApplication +0x60 (FogEffectManager)",
        "scene.FogNear" => "g_PhyreApplication +0x70",
        "scene.FogFar" => "g_PhyreApplication +0x74",
        "scene.FogCurve" => "g_PhyreApplication +0x78",
        "scene.FogLimit" => "g_PhyreApplication +0x7c",
        "scene.BGColor" => "g_PhyreApplication +0x80 (graphicSetBackgroundColour)",
        "scene.GlobalTime" => "g_PhyreApplication +0x120",
        "scene.ViewProjection2" => "g_PhyreApplication +0x90",
        "ClothWorld" => "the cloth simulation's matrix (Ch_Cloth*), in the material buffer",
        "LightViewProjection" | "LVP2" | "MySMScaleBias" | "MySMScaleBias2" => {
            "shadow cascades, written into material buffers by PhyreScene_RenderShadow 0x00653600"
        }
        "CharacterAmbientLight" | "CharacterShadeColor" | "CharacterShadeCoeff" => {
            "character lighting (graphicCharacterLighting*), in the material buffer"
        }
        "Light0.m_direction"
        | "Light0.m_colorIntensity"
        | "Light1.m_direction"
        | "Light1.m_colorIntensity"
        | "Light2.m_direction"
        | "Light2.m_colorIntensity" => {
            "character lights (graphicCharacterLightingSetLightMatrix), in the material buffer"
        }
        "LgtMatVec" | "LgtMatClr" => {
            "PS2 light matrices (graphicPS2LightingSetupEffect), in the material buffer"
        }
        _ => return None,
    })
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect()
}

/// The bytes of a constant in a draw's buffer window.
fn constant_bytes(d: &Draw, stage: char, slot: u8, offset: u32, size: u32) -> Option<&[u8]> {
    let list = if stage == 'v' { &d.vs_cb } else { &d.ps_cb };
    let data = &list.iter().find(|c| c.0 == slot)?.1;
    data.get(offset as usize..(offset + size) as usize)
}

/// A 4x4 matrix as stored (16 floats).
pub type Mat = [f32; 16];

fn mat(bytes: &[u8]) -> Option<Mat> {
    let f = floats(bytes);
    f.get(..16).and_then(|s| s.try_into().ok())
}

/// `a * b` with rows of `a` against columns of `b`, element order as
/// stored, each sum in f32 from k = 0 up.
#[must_use]
pub fn mul(a: &Mat, b: &Mat) -> Mat {
    let mut out = [0f32; 16];
    for i in 0..4 {
        for j in 0..4 {
            let mut s = 0f32;
            for k in 0..4 {
                s += a[i * 4 + k] * b[k * 4 + j];
            }
            out[i * 4 + j] = s;
        }
    }
    out
}

/// As [`mul`], each sum in f64 from k = 0 up and rounded to f32 once (the
/// x87 at 53 bits): how `WorldViewProjection` is made.
#[must_use]
pub fn mul_f64(a: &Mat, b: &Mat) -> Mat {
    let mut out = [0f32; 16];
    for i in 0..4 {
        for j in 0..4 {
            let mut s = 0f64;
            for k in 0..4 {
                s += f64::from(a[i * 4 + k]) * f64::from(b[k * 4 + j]);
            }
            #[allow(clippy::cast_possible_truncation)] // the store to f32
            {
                out[i * 4 + j] = s as f32;
            }
        }
    }
    out
}

/// The largest distance in units in the last place between two matrices.
#[must_use]
pub fn ulps(a: &Mat, b: &Mat) -> u32 {
    let ord = |x: f32| {
        let i = x.to_bits().cast_signed();
        if i < 0 { i32::MIN - i } else { i }
    };
    a.iter()
        .zip(b)
        .map(|(x, y)| ord(*x).abs_diff(ord(*y)))
        .max()
        .unwrap_or(0)
}

/// Reflection of a shader: (cbuffer slot, used variable name, offset, size),
/// struct members expanded as `struct.member`.
fn used_constants(code: &[u8]) -> Vec<(u8, String, u32, u32)> {
    let Ok(c) = ffx_dxbc::Container::parse(code) else {
        return Vec::new();
    };
    let Ok(Some(r)) = c.reflection() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for b in &r.buffers {
        let slot = r
            .bindings
            .iter()
            .find(|x| x.kind == ffx_dxbc::BindKind::ConstantBuffer && x.name == b.name)
            .map_or(0, |x| u8::try_from(x.slot).unwrap_or(u8::MAX));
        for v in b.variables.iter().filter(|v| v.used) {
            if v.members.is_empty() {
                out.push((slot, v.name.clone(), v.offset, v.size));
            } else {
                for m in &v.members {
                    let size = u32::from(m.rows.max(1)) * u32::from(m.columns) * 4;
                    out.push((
                        slot,
                        format!("{}.{}", v.name, m.name),
                        v.offset + m.offset,
                        size,
                    ));
                }
            }
        }
    }
    out
}

/// An input layout element and the file stream it reads.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LayoutPair {
    /// `PVertexStream` semantic and `m_type`.
    pub stream: String,
    pub stream_type: u8,
    /// D3D semantic, index and `DXGI_FORMAT`.
    pub semantic: String,
    pub index: u32,
    pub format: u32,
}

/// The `DXGI_FORMAT`s a `PVertexStream.m_type` is read as.
fn stream_formats(t: u8) -> &'static [u32] {
    match t {
        1 => &[16],        // float2: R32G32_FLOAT
        2 => &[6],         // float3: R32G32B32_FLOAT
        3 => &[2],         // float4: R32G32B32A32_FLOAT
        0x13 => &[28, 30], // four bytes: R8G8B8A8_UNORM or _UINT
        _ => &[],
    }
}

/// Whether the layout's elements are the matched blocks' streams: each
/// element reads, from the vertex buffer in its slot, a stream at its
/// offset whose type gives its format. Returns the pairs found.
fn layout_verdict(
    elements: &[ffx_gfx::Element],
    bound: &[Bound],
    segment: Option<&(String, usize)>,
) -> (Verdict, Vec<LayoutPair>) {
    let mut pairs = Vec::new();
    let mut all = true;
    for e in elements {
        let role = format!("vb{}", e.slot);
        let streams = bound
            .iter()
            .filter(|b| b.role == role)
            .find_map(|b| match &b.verdict {
                Verdict::Matched(src) => src.iter().find_map(|s| match s {
                    Source::Vertices {
                        model,
                        segment: seg,
                        streams,
                        ..
                    } if segment.is_none_or(|x| x.0 == *model && x.1 == *seg) => Some(streams),
                    _ => None,
                }),
                _ => None,
            });
        let hit = streams.and_then(|list| {
            list.iter().find(|(_, t, offset)| {
                *offset == e.offset as usize && stream_formats(*t).contains(&e.format)
            })
        });
        // An input the mesh has no stream for reads a stand-in: the slot
        // holds another block of the segment, read with the shader's
        // format (TEXCOORD1 float3 from the ST block, 4 draws of fw1).
        let stand_in = || {
            streams
                .and_then(|list| list.first())
                .map(|(name, t, _)| (format!("{name} (stand-in)"), *t))
        };
        match hit.map(|(n, t, _)| (n.clone(), *t)).or_else(stand_in) {
            Some((name, t)) => pairs.push(LayoutPair {
                stream: name,
                stream_type: t,
                semantic: e.semantic.clone(),
                index: e.index,
                format: e.format,
            }),
            None => all = false,
        }
    }
    let verdict = if all && !elements.is_empty() {
        Verdict::Matched(Vec::new())
    } else if bound
        .iter()
        .any(|b| b.role.starts_with("vb") && b.verdict == Verdict::Dynamic)
    {
        Verdict::Dynamic
    } else {
        Verdict::Unexplained
    };
    (verdict, pairs)
}

/// A shader's texture bindings: slot → name (`TextureSampler`, …).
fn texture_names(code: &[u8]) -> std::collections::HashMap<u32, String> {
    let Ok(c) = ffx_dxbc::Container::parse(code) else {
        return std::collections::HashMap::new();
    };
    let Ok(Some(r)) = c.reflection() else {
        return std::collections::HashMap::new();
    };
    r.bindings
        .iter()
        .filter(|b| b.kind == ffx_dxbc::BindKind::Texture)
        .map(|b| (b.slot, b.name.clone()))
        .collect()
}

/// A shader's family: the effect cluster it comes from.
fn family(cap: &Capture, index: &AssetIndex, id: u32) -> String {
    cap.shader(id)
        .map(key::shader)
        .and_then(|k| index.shader_cluster.get(&k).cloned())
        .unwrap_or_else(|| "?".into())
}

fn blend_on(desc: &[u8]) -> bool {
    // D3D11_BLEND_DESC: two BOOLs, then RenderTarget[0].BlendEnable.
    desc.get(8..12).is_some_and(|b| b != [0, 0, 0, 0])
}

fn depth_writes(desc: &[u8]) -> bool {
    // D3D11_DEPTH_STENCIL_DESC: DepthEnable, DepthWriteMask.
    desc.get(0..4).is_some_and(|b| b != [0, 0, 0, 0])
        && desc.get(4..8).is_some_and(|b| b != [0, 0, 0, 0])
}

/// Takes the census of every scene draw of `cap`.
#[must_use]
#[allow(clippy::too_many_lines)] // one pass over a draw's bindings
pub fn census(cap: &Capture, index: &AssetIndex) -> Vec<DrawCensus> {
    let mut out = Vec::new();
    // Which textures are render targets: any view of them is an RTV/DSV.
    let mut targets = std::collections::HashSet::new();
    for m in cap.objects.values() {
        if let ffx_gfx::Msg::View { kind, resource, .. } = m
            && *kind != ffx_gfx::ViewKind::ShaderResource
        {
            targets.insert(*resource);
        }
    }
    for (cmd, d) in cap.draws() {
        if Capture::scene_targets(d).is_none() {
            continue;
        }
        let number = out.len();
        let pass = {
            let blend = cap.state(d.blend, StateKind::Blend).is_some_and(blend_on);
            let writes = cap
                .state(d.depth, StateKind::Depth)
                .is_none_or(depth_writes);
            if blend || !writes {
                Pass::Transparent
            } else {
                Pass::Opaque
            }
        };
        let (vs_code, ps_code) = (cap.shader(d.vs), cap.shader(d.ps));
        let mut consts = Vec::new();
        for (stage, code) in [('v', vs_code), ('p', ps_code)] {
            for (slot, name, offset, size) in code.map(used_constants).unwrap_or_default() {
                consts.push((stage, slot, name, offset, size));
            }
        }
        let skinned = consts.iter().any(|c| c.2 == "BoneTransforms")
            || cap
                .layout(d.layout)
                .is_some_and(|l| l.iter().any(|e| e.semantic == "BLENDINDICES"));
        let mut bound = Vec::new();
        for (role, id) in [("vs", d.vs), ("ps", d.ps)] {
            let k = cap.shader(id).map(key::shader);
            let found: Vec<Source> = k
                .map(|k| index.get(k))
                .unwrap_or_default()
                .iter()
                .filter(|s| matches!(s, Source::Shader { .. }))
                .cloned()
                .collect();
            bound.push(Bound {
                role: role.into(),
                id,
                key: k,
                verdict: if found.is_empty() {
                    Verdict::Unexplained
                } else {
                    Verdict::Matched(found)
                },
            });
        }
        let mut candidates: Option<std::collections::BTreeSet<(String, usize)>> = None;
        let buffers = d
            .vertex
            .iter()
            .map(|v| (format!("vb{}", v.0), v.1))
            .chain((d.index.0 != 0).then(|| ("ib".to_string(), d.index.0)));
        for (role, id) in buffers {
            let Some((desc, data)) = cap.buffer(id) else {
                bound.push(Bound {
                    role,
                    id,
                    key: None,
                    verdict: Verdict::Unexplained,
                });
                continue;
            };
            let k = key::buffer(desc, data);
            let index_buffer = role == "ib";
            let found: Vec<Source> = index
                .get(k)
                .iter()
                .filter(|s| match s {
                    Source::Indices { .. } => index_buffer,
                    Source::Vertices { .. } => !index_buffer,
                    _ => false,
                })
                .cloned()
                .collect();
            let verdict = if desc[1] == 2 {
                Verdict::Dynamic
            } else if !found.is_empty() {
                // The segment every matched buffer of the draw shares.
                let here: std::collections::BTreeSet<(String, usize)> = found
                    .iter()
                    .filter_map(|s| match s {
                        Source::Vertices { model, segment, .. }
                        | Source::Indices { model, segment } => Some((model.clone(), *segment)),
                        _ => None,
                    })
                    .collect();
                candidates = Some(match candidates.take() {
                    None => here,
                    Some(c) => {
                        let both: std::collections::BTreeSet<_> =
                            c.intersection(&here).cloned().collect();
                        if both.is_empty() { c } else { both }
                    }
                });
                Verdict::Matched(found)
            } else if skinned {
                Verdict::Skinned
            } else {
                Verdict::Unexplained
            };
            bound.push(Bound {
                role,
                id,
                key: (desc[1] != 2).then_some(k),
                verdict,
            });
        }
        let segment = candidates.and_then(|c| c.into_iter().next());
        let mut layout_pairs = Vec::new();
        if let Some(elements) = cap.layout(d.layout) {
            let (verdict, pairs) = layout_verdict(elements, &bound, segment.as_ref());
            layout_pairs = pairs;
            bound.push(Bound {
                role: "layout".into(),
                id: d.layout,
                key: None,
                verdict,
            });
        }
        let names = ps_code.map(texture_names).unwrap_or_default();
        for &(slot, view) in &d.ps_srv {
            let tex = cap.view_texture(view);
            let shape = tex.map_or_else(String::new, |t| {
                format!("[{}x{}/{} f{}]", t.desc[0], t.desc[1], t.desc[2], t.desc[4])
            });
            let role = match names.get(&u32::from(slot)) {
                Some(n) => format!("ps-t{slot}:{n}{shape}"),
                None => format!("ps-t{slot}{shape}"),
            };
            let resource = cap.view_resource(view).unwrap_or(0);
            let k = tex.map(|t| t.key());
            let found: Vec<Source> = k
                .map(|k| index.get(k))
                .unwrap_or_default()
                .iter()
                .filter(|s| matches!(s, Source::Texture { .. }))
                .cloned()
                .collect();
            let texels = tex
                .and_then(|t| t.content())
                .and_then(|c| index.by_content.get(&c))
                .cloned()
                .unwrap_or_default();
            let bound_as_target = tex.is_some_and(|t| ffx_drawdiff::key::is_target(t.desc));
            let verdict = if !found.is_empty() {
                Verdict::Matched(found)
            } else if !texels.is_empty() {
                Verdict::Texels(texels)
            } else if targets.contains(&resource) || bound_as_target {
                Verdict::RenderTarget
            } else if tex.is_some_and(|t| t.desc[7] == 2) {
                Verdict::Generated
            } else {
                Verdict::Unexplained
            };
            bound.push(Bound {
                role,
                id: resource,
                key: k,
                verdict,
            });
        }
        let info = segment.as_ref().and_then(|s| index.segments.get(s));
        let material = info.and_then(|i| i.material.as_ref());
        let constants = consts
            .iter()
            .map(|(stage, slot, name, offset, size)| {
                let source = if let Some(how) = derived(name) {
                    ConstSource::Derived(how)
                } else if let Some(what) = game_state(name) {
                    ConstSource::GameState(what)
                } else if let Some(stored) = material.and_then(|m| m.parameter(name)) {
                    let now = constant_bytes(d, *stage, *slot, *offset, *size).map(floats);
                    ConstSource::Material {
                        equal: now.is_some_and(|n| {
                            n.iter()
                                .zip(stored)
                                .all(|(a, b)| a.to_bits() == b.to_bits())
                        }),
                    }
                } else {
                    ConstSource::Unknown
                };
                Constant {
                    stage: *stage,
                    name: name.clone(),
                    offset: *offset,
                    size: *size,
                    source,
                }
            })
            .collect();
        let first_bounds = info.and_then(|i| i.bounds.iter().flatten().next().copied());
        let view_z = first_bounds.and_then(|b| view_depth(d, &consts, &b));
        let render_order = first_bounds.map(|b| b.render_order);
        out.push(DrawCensus {
            cmd,
            number,
            pass,
            vs_family: family(cap, index, d.vs),
            ps_family: family(cap, index, d.ps),
            skinned,
            segment: segment.clone(),
            instances: info.map(|i| i.instances.clone()).unwrap_or_default(),
            bound,
            constants,
            view_z,
            render_order,
            index_count: d.count,
            layout_pairs,
        });
    }
    out
}

/// The view-space z of an instance's bounds centre, as the renderer's sort
/// key takes it (`docs/formats/scene.md`, "Sort"): centre
/// `m_min + m_size * 0.5` in f32, through the draw's `World`, dotted with
/// the view's z row (`scene.View`'s third stored row), each step rounded
/// to f32. The
/// transparent key is this value, the opaque key its negation; the camera
/// looks down -z, so ascending keys draw transparent far to near and
/// opaque near to far.
fn view_depth(
    d: &Draw,
    consts: &[(char, u8, String, u32, u32)],
    bounds: &ffx_phyre::Bounds,
) -> Option<f32> {
    let find = |name: &str| {
        consts
            .iter()
            .find(|c| c.2 == name)
            .and_then(|c| constant_bytes(d, c.0, c.1, c.3, 64))
            .and_then(mat)
    };
    let (world, view) = (find("World")?, find("scene.View")?);
    let c: [f32; 3] = std::array::from_fn(|k| bounds.size[k] * 0.5 + bounds.min[k]);
    // World is stored transposed: row i of the stored matrix gives
    // coordinate i.
    let p: [f32; 3] = std::array::from_fn(|i| {
        world[i * 4] * c[0] + world[i * 4 + 1] * c[1] + world[i * 4 + 2] * c[2] + world[i * 4 + 3]
    });
    let z = &view[8..12];
    Some(z[3] + (z[0] * p[0] + z[1] * p[1] + z[2] * p[2]))
}

/// The renderer's order of two visible instances (`docs/formats/scene.md`,
/// "Sort"; `PCheckVisibleBoundsJob_compareEntries` `0x009f2d70`): render
/// orders with `ro / 100000000 == 1` first, then `ro % 1000` ascending,
/// then `(ro % 100000) / 1000` ascending, then the depth key ascending.
#[must_use]
pub fn sort_before(a: (i32, f32), b: (i32, f32)) -> bool {
    let class = |ro: i32| ro / 100_000_000 == 1;
    if class(a.0) != class(b.0) {
        return class(a.0);
    }
    if a.0 % 1000 != b.0 % 1000 {
        return a.0 % 1000 < b.0 % 1000;
    }
    if (a.0 % 100_000) / 1000 != (b.0 % 100_000) / 1000 {
        return (a.0 % 100_000) / 1000 < (b.0 % 100_000) / 1000;
    }
    a.1 < b.1
}

/// Checks the frame's draw order against the sort rule: per run of draws
/// of one pass and one cluster, the instances in draw order (a mesh
/// instance's segments are drawn together), each pair of neighbours in
/// rule order. Returns (pairs checked, pairs out of order, the first
/// ones out of order as text).
#[must_use]
pub fn sort_check(draws: &[DrawCensus]) -> (usize, usize, Vec<String>) {
    let mut instances: Vec<(Pass, String, String, i32, f32, usize)> = Vec::new();
    for d in draws {
        let (Some((model, _)), Some(z), Some(ro)) = (&d.segment, d.view_z, d.render_order) else {
            continue;
        };
        let name = d.instances.first().cloned().unwrap_or_default();
        let key = if d.pass == Pass::Opaque { -z } else { z };
        if instances
            .last()
            .is_some_and(|l| l.0 == d.pass && l.1 == *model && l.2 == name)
        {
            continue;
        }
        instances.push((d.pass, model.clone(), name, ro, key, d.number));
    }
    let (mut checked, mut bad, mut notes) = (0, 0, Vec::new());
    for w in instances.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if a.0 != b.0 || a.1 != b.1 {
            continue;
        }
        checked += 1;
        if sort_before((b.3, b.4), (a.3, a.4)) {
            bad += 1;
            if notes.len() < 10 {
                notes.push(format!(
                    "draw {} {} (ro {}, key {}) after draw {} {} (ro {}, key {})",
                    b.5, b.2, b.3, b.4, a.5, a.2, a.3, a.4
                ));
            }
        }
    }
    (checked, bad, notes)
}

/// A 4x4 constant of a draw's vertex shader, by name (`World`,
/// `scene.ViewProjection`, …), used or not.
fn draw_matrix(cap: &Capture, d: &Draw, name: &str) -> Option<Mat> {
    let code = cap.shader(d.vs)?;
    let (slot, _, offset) = all_constants(code).into_iter().find(|c| c.1 == name)?;
    constant_bytes(d, 'v', slot, offset, 64).and_then(mat)
}

/// The renderer's frustum test of a box (`BoundingBox_IsVisible_00663a30`
/// with `ClipOutcode` `0x00652470`): visible unless all eight corners are
/// strictly outside one plane of the clip volume `-w..w` in x, y and z.
/// `world` and `view_projection` are as the shaders get them (stored
/// transposed, depth 0..1); z is taken back to the renderer's `-w..w`
/// range (`2 z - w`), the range of the camera's `+0x70` matrix it culls
/// with.
#[must_use]
pub fn box_visible(b: &ffx_phyre::Bounds, world: &Mat, view_projection: &Mat) -> bool {
    let mut all = 0x3fu8;
    for corner in 0..8 {
        let c: [f32; 3] =
            std::array::from_fn(|k| b.min[k] + if corner >> k & 1 == 1 { b.size[k] } else { 0.0 });
        let p: [f32; 4] = std::array::from_fn(|i| {
            if i == 3 {
                1.0
            } else {
                world[i * 4] * c[0]
                    + world[i * 4 + 1] * c[1]
                    + world[i * 4 + 2] * c[2]
                    + world[i * 4 + 3]
            }
        });
        let clip: [f32; 4] =
            std::array::from_fn(|i| (0..4).map(|k| view_projection[i * 4 + k] * p[k]).sum());
        let (x, y, w) = (clip[0], clip[1], clip[3]);
        let z = 2.0 * clip[2] - w;
        let mut code = 0u8;
        code |= u8::from(x > w) | u8::from(y > w) << 1 | u8::from(z > w) << 2;
        code |= u8::from(x < -w) << 3 | u8::from(y < -w) << 4 | u8::from(z < -w) << 5;
        all &= code;
    }
    all == 0
}

/// The culling census: for every model the scene draws from, how the
/// file's node world matrices compare with the drawn instances' `World`
/// constants, then the instances that drew nothing this frame and whether
/// the frustum test rejects them (with the World most drawn instances of
/// the model share: the game sets map worlds at run time), and any
/// drawn instance the test would reject. Lines of text.
#[must_use]
#[allow(clippy::too_many_lines)] // the world check, then the frustum test, per model
pub fn cull_check(cap: &Capture, draws: &[DrawCensus], index: &AssetIndex) -> Vec<String> {
    let mut out = Vec::new();
    let scene: Vec<&Draw> = cap
        .draws()
        .filter(|(_, d)| Capture::scene_targets(d).is_some())
        .map(|(_, d)| d)
        .collect();
    let mut by_model: BTreeMap<String, Vec<&DrawCensus>> = BTreeMap::new();
    for d in draws {
        if let Some((m, _)) = &d.segment {
            by_model.entry(m.clone()).or_default().push(d);
        }
    }
    for (model, list) in by_model {
        let Some(instances) = index.instances.get(&model) else {
            continue;
        };
        let name = model.rsplit('/').next().unwrap_or(&model);
        // The captured World of each drawn instance against the file's.
        let mut drawn: BTreeMap<&str, Option<Mat>> = BTreeMap::new();
        for d in &list {
            if let (Some(i), Some(x)) = (d.instances.first(), scene.get(d.number)) {
                drawn
                    .entry(i.as_str())
                    .or_insert_with(|| draw_matrix(cap, x, "World"));
            }
        }
        let (mut same, mut differ) = (0, Vec::new());
        for i in instances {
            if let (Some(Some(got)), Some(file)) = (drawn.get(i.name.as_str()), i.world) {
                if got
                    .iter()
                    .zip(&file)
                    .all(|(a, b)| a.to_bits() == b.to_bits())
                {
                    same += 1;
                } else {
                    differ.push(format!("{} ({} ulp)", i.name, ulps(got, &file)));
                }
            }
        }
        out.push(format!(
            "{name}: World = file node world for {same} drawn instances, differs for {}",
            differ.len()
        ));
        // The World most drawn instances share: what the others are
        // assumed to have (the game writes map worlds at run time).
        let mut counts: Vec<(Mat, usize)> = Vec::new();
        for w in drawn.values().flatten() {
            match counts
                .iter_mut()
                .find(|c| c.0.map(f32::to_bits) == w.map(f32::to_bits))
            {
                Some(c) => c.1 += 1,
                None => counts.push((*w, 1)),
            }
        }
        let common = counts.iter().max_by_key(|c| c.1).map(|c| c.0);
        if let Some(c) = common {
            out.push(format!(
                "  most common World ({} of {}): {c:?}",
                counts.iter().map(|c| c.1).max().unwrap_or(0),
                drawn.len()
            ));
        }
        let vp = list.iter().find_map(|d| {
            scene
                .get(d.number)
                .and_then(|x| draw_matrix(cap, x, "scene.ViewProjection"))
        });
        let Some(vp) = vp else { continue };
        let (mut outside, mut inside, mut unknown) = (Vec::new(), Vec::new(), Vec::new());
        for i in instances
            .iter()
            .filter(|i| !drawn.contains_key(i.name.as_str()))
        {
            match (i.bounds, common) {
                (Some(b), Some(w)) => {
                    if box_visible(&b, &w, &vp) {
                        inside.push(i.name.clone());
                    } else {
                        outside.push(i.name.clone());
                    }
                }
                _ => unknown.push(i.name.clone()),
            }
        }
        out.push(format!(
            "{name}: {} instances, {} drawn; not drawn (with the common World): {} outside the frustum {outside:?}, {} inside {inside:?}, {} not testable {unknown:?}",
            instances.len(),
            drawn.len(),
            outside.len(),
            inside.len(),
            unknown.len(),
        ));
        let rejected: Vec<&str> = instances
            .iter()
            .filter(|i| drawn.contains_key(i.name.as_str()))
            .filter(|i| {
                let world = drawn.get(i.name.as_str()).copied().flatten();
                matches!((i.bounds, world), (Some(b), Some(w)) if !box_visible(&b, &w, &vp))
            })
            .map(|i| i.name.as_str())
            .collect();
        if !rejected.is_empty() {
            out.push(format!("  drawn but outside by the test: {rejected:?}"));
        }
    }
    out
}

/// Checks of how the derived matrices are computed: for each relation, the
/// number of draws where it holds bit for bit, and the largest ULP
/// distance seen.
#[must_use]
pub fn matrix_checks(cap: &Capture) -> BTreeMap<&'static str, (usize, usize, u32)> {
    let mut out: BTreeMap<&'static str, (usize, usize, u32)> = BTreeMap::new();
    for (_, d) in cap.draws() {
        if Capture::scene_targets(d).is_none() {
            continue;
        }
        let Some(code) = cap.shader(d.vs) else {
            continue;
        };
        let consts: Vec<(char, u8, String, u32, u32)> = used_constants(code)
            .into_iter()
            .map(|(s, n, o, z)| ('v', s, n, o, z))
            .collect();
        // Every variable of the vertex shader's buffer, used or not.
        let all = all_constants(code);
        let find = |name: &str| {
            all.iter()
                .chain(
                    consts
                        .iter()
                        .map(|c| (c.1, c.2.clone(), c.3))
                        .collect::<Vec<_>>()
                        .iter(),
                )
                .find(|c| c.1 == name)
                .and_then(|c| constant_bytes(d, 'v', c.0, c.2, 64))
                .and_then(mat)
        };
        let (w, v, p, vp, wv, wvp) = (
            find("World"),
            find("scene.View"),
            find("scene.Projection"),
            find("scene.ViewProjection"),
            find("WorldView"),
            find("WorldViewProjection"),
        );
        let mut check = |name: &'static str, got: Option<Mat>, want: Option<Mat>| {
            if let (Some(g), Some(w)) = (got, want) {
                let e = out.entry(name).or_default();
                let u = ulps(&g, &w);
                e.0 += 1;
                if u == 0 {
                    e.1 += 1;
                }
                e.2 = e.2.max(u);
            }
        };
        // `mul(a, b)`: a's rows against b's columns as the bytes lie; the
        // reversed product is the same relation for transposed storage.
        for (name, reversed, got, a, b) in [
            (
                "WorldView = World . View",
                "WorldView = View . World (reversed)",
                wv,
                w,
                v,
            ),
            (
                "WorldViewProjection = World . ViewProjection",
                "WorldViewProjection = ViewProjection . World (reversed)",
                wvp,
                w,
                vp,
            ),
            (
                "WorldViewProjection = WorldView . Projection",
                "WorldViewProjection = Projection . WorldView (reversed)",
                wvp,
                wv,
                p,
            ),
            (
                "ViewProjection = View . Projection",
                "ViewProjection = Projection . View (reversed)",
                vp,
                v,
                p,
            ),
        ] {
            check(name, got, a.zip(b).map(|(a, b)| mul(&a, &b)));
            check(reversed, got, a.zip(b).map(|(a, b)| mul(&b, &a)));
        }
        check(
            "WorldViewProjection = ViewProjection . World (reversed, f64 sums)",
            wvp,
            w.zip(vp).map(|(w, vp)| mul_f64(&vp, &w)),
        );
        check(
            "WorldView = View . World (reversed, f64 sums)",
            wv,
            w.zip(v).map(|(w, v)| mul_f64(&v, &w)),
        );
    }
    out
}

/// Every variable of a shader's buffers, used or not: (slot, name, offset).
fn all_constants(code: &[u8]) -> Vec<(u8, String, u32)> {
    let Ok(c) = ffx_dxbc::Container::parse(code) else {
        return Vec::new();
    };
    let Ok(Some(r)) = c.reflection() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for b in &r.buffers {
        let slot = r
            .bindings
            .iter()
            .find(|x| x.kind == ffx_dxbc::BindKind::ConstantBuffer && x.name == b.name)
            .map_or(0, |x| u8::try_from(x.slot).unwrap_or(u8::MAX));
        for v in &b.variables {
            out.push((slot, v.name.clone(), v.offset));
            for m in &v.members {
                out.push((slot, format!("{}.{}", v.name, m.name), v.offset + m.offset));
            }
        }
    }
    out
}

/// The census as text: one line per draw, then the totals.
#[must_use]
pub fn report(draws: &[DrawCensus]) -> String {
    let mut s = String::new();
    for d in draws {
        let seg = d.segment.as_ref().map_or_else(
            || "-".to_string(),
            |(m, i)| format!("{}#{i}", m.rsplit('/').next().unwrap_or(m)),
        );
        let _ = write!(
            s,
            "{:3} cmd {:4} {:?} {}/{} n={} seg={} inst={:?} z={} ro={}",
            d.number,
            d.cmd,
            d.pass,
            d.vs_family.trim_end_matches(".fx"),
            d.ps_family.trim_end_matches(".fx"),
            d.index_count,
            seg,
            d.instances,
            d.view_z.map_or_else(|| "-".into(), |z| format!("{z:.3}")),
            d.render_order.map_or_else(|| "-".into(), |r| r.to_string()),
        );
        for b in &d.bound {
            let tag = if matches!(b.verdict, Verdict::Texels(_)) {
                "texels-only:"
            } else {
                ""
            };
            let what = match &b.verdict {
                Verdict::Matched(src) | Verdict::Texels(src) => match src.first() {
                    Some(Source::Texture {
                        path,
                        level,
                        width,
                        height,
                        levels,
                    }) => format!(
                        "{tag}{}@L{level}({width}x{height}/{levels})",
                        path.rsplit('/').next().unwrap_or(path)
                    ),
                    Some(Source::Vertices { block, .. }) => format!("block{block}"),
                    Some(Source::Indices { .. }) => "indices".into(),
                    Some(Source::Shader { cluster }) => {
                        cluster.rsplit('/').next().unwrap_or(cluster).to_owned()
                    }
                    // a layout made from the matched blocks' streams
                    None => "streams".into(),
                },
                v => v.label().into(),
            };
            let _ = write!(s, " {}={what}", b.role);
        }
        s.push('\n');
        for c in &d.constants {
            let _ = writeln!(s, "      {}:{} {}", c.stage, c.name, c.source.label());
        }
    }
    s.push_str(&summary(draws));
    s
}

/// The totals: draws per pass and family, objects per verdict, constants
/// per source class.
#[must_use]
pub fn summary(draws: &[DrawCensus]) -> String {
    let mut s = String::new();
    let mut per: BTreeMap<(Pass, String), usize> = BTreeMap::new();
    let mut verdicts: BTreeMap<(String, &'static str), usize> = BTreeMap::new();
    let mut consts: BTreeMap<(&'static str, String), usize> = BTreeMap::new();
    let mut unique: BTreeMap<(&'static str, String), std::collections::BTreeSet<u64>> =
        BTreeMap::new();
    for d in draws {
        *per.entry((d.pass, d.ps_family.clone())).or_default() += 1;
        for b in &d.bound {
            let kind = match b.role.as_str() {
                r if r.starts_with("vb") => "vertex buffer",
                "ib" => "index buffer",
                r if r.starts_with("ps-t") => "texture",
                "layout" => "input layout",
                _ => "shader",
            };
            *verdicts
                .entry((kind.to_string(), b.verdict.label()))
                .or_default() += 1;
            unique
                .entry((b.verdict.label(), kind.to_string()))
                .or_default()
                .insert(b.key.unwrap_or(u64::from(b.id)));
        }
        for c in &d.constants {
            *consts
                .entry((c.source.class(), c.name.clone()))
                .or_default() += 1;
        }
    }
    let _ = writeln!(s, "\n== draws per pass and family (pixel shader) ==");
    for ((p, f), n) in &per {
        let _ = writeln!(s, "{p:?}\t{f}\t{n}");
    }
    let _ = writeln!(
        s,
        "\n== bindings per verdict (bindings / distinct objects) =="
    );
    for ((k, v), n) in &verdicts {
        let distinct = unique
            .get(&(*v, k.clone()))
            .map_or(0, std::collections::BTreeSet::len);
        let _ = writeln!(s, "{k}\t{v}\t{n}\t{distinct}");
    }
    let _ = writeln!(s, "\n== constants by source (draw uses) ==");
    for ((class, name), n) in &consts {
        let _ = writeln!(s, "{class}\t{name}\t{n}");
    }
    let mut pairs: BTreeMap<&LayoutPair, usize> = BTreeMap::new();
    for d in draws {
        for p in &d.layout_pairs {
            *pairs.entry(p).or_default() += 1;
        }
    }
    let _ = writeln!(
        s,
        "\n== input layout elements from file streams (stream, type -> semantic, format: draws) =="
    );
    for (p, n) in pairs {
        let _ = writeln!(
            s,
            "{} {:#x} -> {}{} DXGI {}\t{n}",
            p.stream, p.stream_type, p.semantic, p.index, p.format
        );
    }
    s
}
