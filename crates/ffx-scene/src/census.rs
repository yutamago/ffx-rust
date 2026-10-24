//! The census of a captured frame's scene draws: for each draw, the
//! cluster objects it comes from (by content key), its pass, its shader
//! family, where each constant it uses comes from, and a verdict on every
//! static object it binds (`docs/formats/scene.md`).

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
    pub index_count: u32,
    /// How the input layout's elements map to the file's streams.
    pub layout_pairs: Vec<LayoutPair>,
}

/// The names `PhyreEngine` computes per draw or per frame (its parameter
/// semantics), and what from. Filled from FFX.exe
/// (`docs/formats/scene.md`, "Constants").
fn derived(name: &str) -> Option<&'static str> {
    Some(match name {
        "World" => "the instance's world matrix (PWorldMatrix)",
        "WorldView" => "World x scene.View",
        "WorldViewProjection" => "World x scene.ViewProjection",
        "BoneTransforms" => "skin matrices: bone world x inverse bind",
        "ClothWorld" => "the cloth simulation's world matrix",
        "LightViewProjection" | "LVP2" => "shadow cascade view-projection",
        "MySMScaleBias" | "MySMScaleBias2" => "shadow map scale and bias",
        "scene.EyePosition" => "camera position (ViewInverse row 3)",
        "scene.View" => "inverse of the camera world matrix (scene +0xfa04)",
        "scene.Projection" => "camera projection (fov y +0x154, aspect +0x150, near +0xb8, far +0xbc)",
        "scene.ViewProjection" => "View x Projection",
        "scene.ViewProjection2" => "View x Projection (second camera slot)",
        "scene.ViewInverse" => "the camera world matrix",
        "scene.cameraNearFar"
        | "scene.cameraNearTimesFar"
        | "scene.cameraFarMinusNear" => "camera near and far planes",
        "scene.ViewportWidthHeight" | "scene.screenWidthHeightInv" => "viewport size",
        _ => return None,
    })
}

fn game_state(name: &str) -> Option<&'static str> {
    Some(match name {
        "scene.GlobalAmbientColor" => "g_PhyreApplication +0x50 (graphicSetGlobalAmbientColor)",
        "scene.FogColor" => "g_PhyreApplication +0x60",
        "scene.FogNear" => "g_PhyreApplication +0x70",
        "scene.FogFar" => "g_PhyreApplication +0x74",
        "scene.FogCurve" => "g_PhyreApplication +0x78",
        "scene.FogLimit" => "g_PhyreApplication +0x7c",
        "scene.BGColor" => "g_PhyreApplication +0x80 (graphicSetBackgroundColour)",
        "scene.GlobalTime" => "the application's clock",
        "CharacterAmbientLight" | "CharacterShadeColor" | "CharacterShadeCoeff" => {
            "character lighting (graphicCharacterLighting*)"
        }
        "Light0" | "Light1" | "Light2" | "Light0.m_direction" | "Light0.m_colorIntensity"
        | "Light1.m_direction" | "Light1.m_colorIntensity" | "Light2.m_direction"
        | "Light2.m_colorIntensity" => "character lights (graphicCharacterLightingSetLightMatrix)",
        "LgtMatVec" | "LgtMatClr" => "PS2 light matrices (graphicPS2LightingSetupEffect)",
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
fn constant_bytes<'a>(d: &'a Draw, stage: char, slot: u8, offset: u32, size: u32) -> Option<&'a [u8]> {
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
            .map_or(0, |x| x.slot);
        for v in b.variables.iter().filter(|v| v.used) {
            if v.members.is_empty() {
                out.push((slot as u8, v.name.clone(), v.offset, v.size));
            } else {
                for m in &v.members {
                    let size = u32::from(m.rows.max(1)) * u32::from(m.columns) * 4;
                    out.push((
                        slot as u8,
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
        1 => &[16],     // float2: R32G32_FLOAT
        2 => &[6],      // float3: R32G32B32_FLOAT
        3 => &[2],      // float4: R32G32B32A32_FLOAT
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
        match hit {
            Some((name, t, _)) => pairs.push(LayoutPair {
                stream: name.clone(),
                stream_type: *t,
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
            let writes = cap.state(d.depth, StateKind::Depth).is_none_or(depth_writes);
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
            let k = key::buffer(data);
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
            let role = match names.get(&u32::from(slot)) {
                Some(n) => format!("ps-t{slot}:{n}"),
                None => format!("ps-t{slot}"),
            };
            let tex = cap.view_texture(view);
            let resource = cap.view_resource(view).unwrap_or(0);
            let k = tex.and_then(|t| t.key());
            let found: Vec<Source> = k
                .map(|k| index.get(k))
                .unwrap_or_default()
                .iter()
                .filter(|s| matches!(s, Source::Texture { .. }))
                .cloned()
                .collect();
            // D3D11_BIND_RENDER_TARGET 0x20, D3D11_BIND_DEPTH_STENCIL 0x40
            let bound_as_target = tex.is_some_and(|t| t.desc[8] & 0x60 != 0);
            let verdict = if !found.is_empty() {
                Verdict::Matched(found)
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
        let view_z = view_depth(cap, d, &consts);
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
            index_count: d.count,
            layout_pairs,
        });
    }
    out
}

/// The view-space z of the draw's vertex bounds centre: positions from the
/// vertex buffer in slot 0 (`POSITION` at offset 0), through `World` and
/// `scene.View` (row vectors: `p . World . View`).
fn view_depth(cap: &Capture, d: &Draw, consts: &[(char, u8, String, u32, u32)]) -> Option<f32> {
    let find = |name: &str| {
        consts
            .iter()
            .find(|c| c.2 == name)
            .and_then(|c| constant_bytes(d, c.0, c.1, c.3, 64))
            .and_then(mat)
    };
    let (world, view) = (find("World")?, find("scene.View")?);
    let v = d.vertex.iter().find(|v| v.0 == 0)?;
    let (_, data) = cap.buffer(v.1)?;
    let stride = v.2 as usize;
    if stride < 12 {
        return None;
    }
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for c in data.chunks_exact(stride) {
        for k in 0..3 {
            let x = f32::from_le_bytes(c[4 * k..4 * k + 4].try_into().ok()?);
            lo[k] = lo[k].min(x);
            hi[k] = hi[k].max(x);
        }
    }
    let p = [
        f32::midpoint(lo[0], hi[0]),
        f32::midpoint(lo[1], hi[1]),
        f32::midpoint(lo[2], hi[2]),
        1.0,
    ];
    // Matrices are stored transposed (column-major packing): WorldView's
    // stored form is `View . World`, and z is its third stored row.
    let wv = mul(&view, &world);
    Some((0..4).map(|k| p[k] * wv[8 + k]).sum())
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
        let Some(code) = cap.shader(d.vs) else { continue };
        let consts: Vec<(char, u8, String, u32, u32)> = used_constants(code)
            .into_iter()
            .map(|(s, n, o, z)| ('v', s, n, o, z))
            .collect();
        // Every variable of the vertex shader's buffer, used or not.
        let all = all_constants(code);
        let find = |name: &str| {
            all.iter()
                .chain(consts.iter().map(|c| (c.1, c.2.clone(), c.3)).collect::<Vec<_>>().iter())
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
            .map_or(0, |x| x.slot) as u8;
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
            "{:3} cmd {:4} {:?} {}/{} n={} seg={} inst={:?} z={}",
            d.number,
            d.cmd,
            d.pass,
            d.vs_family.trim_end_matches(".fx"),
            d.ps_family.trim_end_matches(".fx"),
            d.index_count,
            seg,
            d.instances,
            d.view_z.map_or_else(|| "-".into(), |z| format!("{z:.1}")),
        );
        for b in &d.bound {
            let what = match &b.verdict {
                Verdict::Matched(src) => match src.first() {
                    Some(Source::Texture { path, level, .. }) => format!(
                        "{}@L{level}",
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
    let _ = writeln!(s, "\n== bindings per verdict (bindings / distinct objects) ==");
    for ((k, v), n) in &verdicts {
        let distinct = unique.get(&(*v, k.clone())).map_or(0, std::collections::BTreeSet::len);
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
