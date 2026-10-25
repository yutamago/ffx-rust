//! Effect variant clusters (`shaders/d3d11/<effect>.fx#<md5>.phyre`): the
//! render passes a material can draw in, their shaders and pass states,
//! and the tables that say where each parameter goes in the constant
//! buffers and which texture and sampler registers take what
//! (`docs/scene-builder.md`, "Effects").
//!
//! Classes (from the clusters' namespace): `PEffectVariant` →
//! `m_sceneRenderPasses` (`PSceneRenderPass`: `m_passType`, `m_shaders`)
//! → `PShader` (`m_passes`, `m_streamDefinitionsForPasses`) →
//! `PShaderPassD3D11` (`m_state`: the raster, depth-stencil and blend
//! descriptions as D3D11 structures, created as they are by
//! `PShaderPassStateD3D11` `0x0058d4e0` and bound as they are by
//! `0x0058c6c0`; the vertex and fragment programs; the parameter location
//! tables).

// Location table entries and pass state words are read field by field
// (a, o, t, w, ...).
#![allow(clippy::many_single_char_names)]

use crate::phyre::{Graph, Obj};

/// A parameter definition (`PShaderParameterDefinition`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDef {
    pub name: String,
    /// `m_parameterType`: 0x40 a value, 0x42 a sampler with a texture,
    /// 0x47 a sampler state; 0x80 and up a renderer semantic (0x82
    /// `World`, 0x83 `WorldView`, 0x84 `WorldViewProjection`, ...).
    pub kind: u8,
    pub data_type: u8,
    /// Offset and size in the parameter buffer it lives in.
    pub offset: u32,
    pub size: u32,
}

/// One entry of a location table
/// (`PShaderParameterCaptureBufferLocationTypeConstantBuffer`): `size`
/// bytes from `offset` of the source buffer go to `target` of the
/// constant buffer (or, in a texture table, the register `target`;
/// `u32::MAX`: the program does not use it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loc {
    pub offset: u16,
    pub target: u32,
    pub size: u32,
    pub kind: u8,
}

/// Which buffer a location table entry reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// The scene-wide parameters (`PSceneWideParameters_getBuffer`
    /// `0x004fdec0`).
    SceneWide,
    /// The material's `PParameterBuffer`.
    Material,
    /// The per-node values (`0x005b7fe0`): `World`, `WorldView`, ...
    Node,
    /// A fourth group (streams); not seen on maps.
    Other,
}

/// A location table (`PShaderPassParameterLocationTypesConstantBuffer`):
/// entries in groups by source, `starts`/`counts` per group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocTable {
    pub starts: [u16; 4],
    pub counts: [u16; 4],
    pub entries: Vec<Loc>,
}

impl LocTable {
    /// The entries with their source group.
    pub fn sourced(&self) -> impl Iterator<Item = (Source, &Loc)> {
        self.entries.iter().enumerate().map(|(i, l)| {
            let group = (0..4).find(|&g| {
                let s = usize::from(self.starts[g]);
                i >= s && i < s + usize::from(self.counts[g])
            });
            let source = match group {
                Some(0) => Source::SceneWide,
                Some(1) => Source::Material,
                Some(2) => Source::Node,
                _ => Source::Other,
            };
            (source, l)
        })
    }
}

/// A vertex shader input (`PStreamInputDescD3D11`): the input layout
/// element the game makes for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDesc {
    pub semantic: String,
    /// The `PRenderDataType` it reads (`Vertex`, `ST`, ...).
    pub render_type: String,
    pub index: u32,
    pub format: u32,
    pub slot: u32,
}

/// A compiled program (`PShaderProgramD3D11`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Program {
    pub code: Vec<u8>,
    /// The `$Globals` buffer's size.
    pub cb_size: u32,
    /// Its slot (`u32::MAX`: none).
    pub global_cb: u32,
    /// Vertex programs: the input layout.
    pub inputs: Vec<InputDesc>,
}

/// A shader pass's fixed-function state (`PShaderPassStateD3D11`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassState {
    pub important: u32,
    /// `D3D11_RASTERIZER_DESC`, `D3D11_DEPTH_STENCIL_DESC`,
    /// `D3D11_BLEND_DESC` bytes.
    pub raster: Vec<u8>,
    pub depth: Vec<u8>,
    pub blend: Vec<u8>,
    pub stencil_ref: u8,
}

/// A shader's pass (`PShaderPassD3D11`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderPass {
    pub state: PassState,
    pub vs: Program,
    pub ps: Program,
    pub vs_params: LocTable,
    pub ps_params: LocTable,
    pub vs_textures: LocTable,
    pub ps_textures: LocTable,
}

/// A stream the shader reads (`PShaderStreamDefinition`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamDef {
    pub name: String,
    pub render_type: String,
    pub data_type: u8,
    pub index: u8,
}

/// A shader (`PShader`): one context variant of a render pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shader {
    pub passes: Vec<ShaderPass>,
    pub streams: Vec<StreamDef>,
}

/// A render pass of a variant (`PSceneRenderPass`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPass {
    /// `Opaque`, `OpaqueNoMSAA`, `Transparent`, `TransparentNoMSAA`,
    /// `ZPrePass`, ...
    pub pass_type: String,
    pub shaders: Vec<Shader>,
}

/// An effect variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectVariant {
    pub switches: Vec<(String, String)>,
    pub passes: Vec<RenderPass>,
    pub tweakable: Vec<ParamDef>,
    pub untweakable: Vec<ParamDef>,
    pub tweakable_size: u16,
    pub untweakable_size: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn err(what: impl Into<String>) -> Error {
    Error(what.into())
}

impl EffectVariant {
    /// Reads the variant of an effect variant cluster.
    ///
    /// # Errors
    /// When the cluster has no `PEffectVariant` or a field is missing.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, Error> {
        let g = Graph::parse(bytes).map_err(|e| err(format!("effect cluster: {e}")))?;
        let v = *g
            .objects("PEffectVariant")
            .first()
            .ok_or_else(|| err("no PEffectVariant"))?;
        let c = "PEffectVariant";
        let switches = g
            .targets(v, c, "m_switches", 4)
            .into_iter()
            .map(|s| {
                (
                    g.string(s, "PMaterialSwitch", "m_name").unwrap_or_default(),
                    g.string(s, "PMaterialSwitch", "m_value")
                        .unwrap_or_default(),
                )
            })
            .collect();
        let defs = |member| -> Vec<ParamDef> {
            g.targets(v, c, member, 4)
                .into_iter()
                .map(|d| param_def(&g, d))
                .collect()
        };
        let mut passes = Vec::new();
        for rp in g.targets(v, c, "m_sceneRenderPasses", 4) {
            let r = "PSceneRenderPass";
            let pass_type = g
                .user_text(rp, r, "m_passType")
                .ok_or_else(|| err("render pass without a type"))?
                .to_owned();
            let mut shaders = Vec::new();
            for sh in g.targets(rp, r, "m_shaders", 4) {
                let s = "PShader";
                let streams = g
                    .targets(sh, s, "m_streamDefinitionsForPasses", 4)
                    .into_iter()
                    .map(|d| {
                        let t = "PShaderStreamDefinition";
                        StreamDef {
                            name: g.string(d, t, "m_name").unwrap_or_default(),
                            render_type: g
                                .user_text(d, t, "m_renderType")
                                .unwrap_or_default()
                                .to_owned(),
                            data_type: g.u8(d, t, "m_dataType").unwrap_or(0),
                            index: g.u8(d, t, "m_index").unwrap_or(0),
                        }
                    })
                    .collect();
                let passes = g
                    .targets(sh, s, "m_passes", 4)
                    .into_iter()
                    .map(|p| shader_pass(&g, p))
                    .collect::<Result<Vec<_>, _>>()?;
                shaders.push(Shader { passes, streams });
            }
            passes.push(RenderPass { pass_type, shaders });
        }
        Ok(Self {
            switches,
            passes,
            tweakable: defs("m_tweakableShaderParameterDefinitions"),
            untweakable: defs("m_untweakableShaderParameterDefinitions"),
            tweakable_size: g.u16(v, c, "m_tweakableParameterBufferSize").unwrap_or(0),
            untweakable_size: g.u16(v, c, "m_untweakableParameterBufferSize").unwrap_or(0),
        })
    }

    /// The render pass of a type.
    #[must_use]
    pub fn pass(&self, pass_type: &str) -> Option<&RenderPass> {
        self.passes.iter().find(|p| p.pass_type == pass_type)
    }

    /// A tweakable parameter by its offset in the material buffer.
    #[must_use]
    pub fn tweakable_at(&self, offset: u16) -> Option<&ParamDef> {
        self.tweakable
            .iter()
            .find(|d| d.offset == u32::from(offset))
    }
}

fn param_def(g: &Graph, d: Obj) -> ParamDef {
    let t = "PShaderParameterDefinition";
    let loc = g.u32(d, t, "m_bufferLoc").unwrap_or(0);
    ParamDef {
        name: g.string(d, t, "m_name").unwrap_or_default(),
        kind: g.u8(d, t, "m_parameterType").unwrap_or(0),
        data_type: g.u8(d, t, "m_dataType").unwrap_or(0),
        offset: loc & 0xffff,
        size: loc >> 16,
    }
}

fn program(g: &Graph, prog: Obj) -> Program {
    let pc = "PShaderProgramD3D11";
    let len = g.u32(prog, pc, "m_compiledCode").unwrap_or(0) as usize;
    let code = g
        .array_bytes(prog, pc, "m_compiledCode", 4)
        .map(|b| b[..len.min(b.len())].to_vec())
        .unwrap_or_default();
    let mut inputs = Vec::new();
    if g.class_of(prog.list) == "PShaderVertexProgram"
        && let Some(il) = g.offset("PShaderVertexProgramD3D11", "m_inputLayout")
        && let Some(t) = g.ptr_at(prog, il + 4)
    {
        for i in t.first..t.first + t.count {
            let d = g.obj(t.list, i);
            let s = "PStreamInputDescD3D11";
            inputs.push(InputDesc {
                semantic: g.string(d, s, "m_semantic").unwrap_or_default(),
                render_type: g
                    .user_text(d, s, "m_renderType")
                    .unwrap_or_default()
                    .to_owned(),
                index: g.u32(d, s, "m_semanticIndex").unwrap_or(0),
                format: g.u32(d, s, "m_d3dFormat").unwrap_or(0),
                slot: g.u32(d, s, "m_inputSlot").unwrap_or(0),
            });
        }
    }
    Program {
        code,
        cb_size: g.u32(prog, pc, "m_constantBufferSize").unwrap_or(0),
        global_cb: g
            .u32(prog, pc, "m_globalConstantBufferIndex")
            .unwrap_or(u32::MAX),
        inputs,
    }
}

fn loc_table(g: &Graph, p: Obj, member: &str) -> LocTable {
    let c = "PShaderPassD3D11";
    let Some(off) = g.offset(c, member) else {
        return LocTable::default();
    };
    let at = p.at + off as usize;
    let half = |k: usize| {
        g.bytes
            .get(at + k..at + k + 2)
            .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
    };
    let mut t = LocTable {
        starts: [half(0), half(2), half(4), half(6)],
        counts: [half(8), half(10), half(12), half(14)],
        entries: Vec::new(),
    };
    if let Some(target) = g.ptr_at(p, off + 0x14) {
        for i in target.first..target.first + target.count {
            let o = g.obj(target.list, i);
            let w = |k: usize| g.word_at(o.at + k).unwrap_or(0);
            t.entries.push(Loc {
                offset: (w(0) & 0xffff) as u16,
                target: w(4),
                size: w(8),
                kind: (w(12) & 0xff) as u8,
            });
        }
    }
    t
}

fn shader_pass(g: &Graph, p: Obj) -> Result<ShaderPass, Error> {
    let c = "PShaderPassD3D11";
    let st = g
        .offset(c, "m_state")
        .ok_or_else(|| err("no PShaderPassD3D11.m_state"))? as usize;
    let field = |class: &str, member: &str, len: usize| -> Result<Vec<u8>, Error> {
        let o = g
            .offset(class, member)
            .ok_or_else(|| err(format!("no {class}.{member}")))? as usize;
        g.bytes
            .get(p.at + st + o..p.at + st + o + len)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| err("pass state outside the file"))
    };
    let s = "PShaderPassStateD3D11";
    let state = PassState {
        important: g.word_at(p.at + st).unwrap_or(0),
        raster: field(s, "m_rasterDesc", 40)?,
        depth: field(s, "m_depthDesc", 52)?,
        blend: field(s, "m_blendDesc", 264)?,
        stencil_ref: field(s, "m_stencilRef", 1)?[0],
    };
    let prog = |member| {
        g.target_obj(p, "PShaderPassBase", member)
            .map(|o| program(g, o))
            .unwrap_or_default()
    };
    Ok(ShaderPass {
        state,
        vs: prog("m_vertexProgram"),
        ps: prog("m_fragmentProgram"),
        vs_params: loc_table(g, p, "m_vertexParameterLocation"),
        ps_params: loc_table(g, p, "m_fragmentParameterLocation"),
        vs_textures: loc_table(g, p, "m_vertexTexParameterLocation"),
        ps_textures: loc_table(g, p, "m_fragmentTexParameterLocation"),
    })
}
