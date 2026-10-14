//! Shader Model 4 programs to WGSL, so the renderer can run the game's own
//! shaders on wgpu with the same results.
//!
//! Registers keep their raw 32-bit values (`vec4<u32>`); float, signed and
//! unsigned instructions reinterpret them (`bitcast`), as the bytecode
//! does: comparisons produce all-ones masks that later `and`s use as
//! floats. Each stage gets its own bind group: constant buffer `b<n>` is
//! binding `n`, texture `t<n>` binding `16 + n`, sampler `s<n>` binding
//! `32 + n`.
//!
//! Inputs and outputs that the bytecode packs into one register (a
//! texture coordinate in `xy` and the fog factor in `z`) are separate WGSL
//! locations; the stages are linked by semantic (`link`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::sm4::{Decl, Dim, Index, Instruction, Interp, Op, Operand, Reg, Select};
use crate::{Container, Error, Result, Signature};

/// Bind group layout of one stage.
pub const TEXTURE_BINDING: u32 = 16;
pub const SAMPLER_BINDING: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    F32,
    I32,
    U32,
}

impl Ty {
    fn from_component(c: u32) -> Self {
        match c {
            1 => Self::U32,
            2 => Self::I32,
            _ => Self::F32,
        }
    }

    fn scalar(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::I32 => "i32",
            Self::U32 => "u32",
        }
    }

    fn vec(self, n: u32) -> String {
        if n == 1 {
            self.scalar().to_owned()
        } else {
            format!("vec{n}<{}>", self.scalar())
        }
    }
}

/// One value passed from the vertex to the pixel shader.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Varying {
    pub semantic: String,
    pub index: u32,
    pub location: u32,
    /// Components (1–4).
    pub width: u32,
    /// The first component in the vertex shader's output register.
    pub start: u32,
    pub ty: Ty,
    /// The WGSL interpolation attribute (empty for the default).
    pub interp: String,
}

fn components(mask: u8) -> (u32, u32) {
    if mask == 0 {
        return (0, 0);
    }
    let start = mask.trailing_zeros();
    let end = 8 - mask.leading_zeros();
    (start, end - start)
}

fn same(a: &Signature, semantic: &str, index: u32) -> bool {
    a.semantic.eq_ignore_ascii_case(semantic) && a.index == index
}

/// The values a vertex and a pixel shader pass between them.
///
/// # Errors
/// When a signature does not parse.
pub fn link(vs: &Container, ps: &Container) -> Result<Vec<Varying>> {
    let outputs = vs.outputs()?;
    let inputs = ps.inputs()?;
    let program = ps.program_code()?;
    let interp: BTreeMap<u32, Interp> = program
        .decls
        .iter()
        .filter_map(|d| match d {
            Decl::Input {
                reg,
                interp: Some(i),
                ..
            } => Some((*reg, *i)),
            _ => None,
        })
        .collect();
    let mut out = Vec::new();
    for e in inputs.iter().filter(|e| e.system == 0) {
        let vs_e = outputs.iter().find(|o| same(o, &e.semantic, e.index));
        let (start, width) = components(vs_e.map_or(e.mask, |o| o.mask));
        let ty = Ty::from_component(vs_e.map_or(e.component, |o| o.component));
        let attr = if ty == Ty::F32 {
            match interp.get(&e.register) {
                Some(Interp::Constant) => "@interpolate(flat) ",
                Some(Interp::LinearCentroid) => "@interpolate(perspective, centroid) ",
                Some(Interp::NoPerspective) => "@interpolate(linear) ",
                _ => "",
            }
        } else {
            "@interpolate(flat) "
        };
        out.push(Varying {
            semantic: e.semantic.clone(),
            index: e.index,
            location: out.len() as u32,
            width: width.max(1),
            start,
            ty,
            interp: attr.to_owned(),
        });
    }
    Ok(out)
}

/// A vertex attribute the vertex shader reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexInput {
    pub semantic: String,
    pub index: u32,
    pub location: u32,
    pub ty: Ty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureKind {
    Float,
    Uint,
    Sint,
    Depth,
    Multisampled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureBinding {
    pub slot: u32,
    pub dim: Dim,
    pub kind: TextureKind,
}

/// A translated stage.
#[derive(Debug, Clone)]
pub struct Shader {
    pub wgsl: String,
    pub vertex: bool,
    /// Constant buffers: (slot, size in bytes the shader declares).
    pub buffers: Vec<(u32, u32)>,
    pub textures: Vec<TextureBinding>,
    /// Samplers: (slot, comparison).
    pub samplers: Vec<(u32, bool)>,
    pub inputs: Vec<VertexInput>,
    /// Render targets written: (slot, type).
    pub targets: Vec<(u32, Ty)>,
    pub writes_depth: bool,
}

fn is_target(s: &Signature) -> bool {
    s.system == 64 || s.semantic.eq_ignore_ascii_case("SV_Target")
}

/// The vertex shader's position output: `SV_Position`, or for old-style
/// shaders `POSITION0`.
fn position_register(outputs: &[Signature]) -> Option<u32> {
    outputs
        .iter()
        .find(|s| s.system == 1)
        .or_else(|| outputs.iter().find(|s| same(s, "POSITION", 0)))
        .map(|s| s.register)
}

fn is_depth(s: &Signature) -> bool {
    s.system == 65 || s.semantic.eq_ignore_ascii_case("SV_Depth")
}

struct Gen {
    vertex: bool,
    out: String,
    depth: usize,
    textures: BTreeMap<u32, TextureBinding>,
}

const SWZ: [char; 4] = ['x', 'y', 'z', 'w'];

fn swizzle(sel: Select) -> String {
    match sel {
        Select::Swizzle(s) => {
            let mut out = String::from(".");
            for c in s {
                out.push(SWZ[c as usize]);
            }
            out
        }
        Select::One(c) => {
            let c = SWZ[c as usize];
            format!(".{c}{c}{c}{c}")
        }
        _ => String::new(),
    }
}

impl Gen {
    fn line(&mut self, text: &str) {
        for _ in 0..self.depth {
            self.out.push_str("    ");
        }
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn index(&self, i: &Index) -> Result<String> {
        Ok(match i {
            Index::Imm(n) => format!("{n}u"),
            Index::Rel(op, base) => format!("({} + {base}u)", self.scalar_u(op)?),
        })
    }

    /// The operand's register as a raw `vec4<u32>`, before its swizzle.
    fn base(&self, op: &Operand) -> Result<String> {
        let n = op.number();
        Ok(match op.reg {
            Reg::Temp => format!("r{n}"),
            Reg::Input => format!("v{n}"),
            Reg::Output => format!("o{n}"),
            Reg::IndexableTemp => {
                let i = op.index.get(1).ok_or(Error::Truncated(0))?;
                format!("x{n}[{}]", self.index(i)?)
            }
            Reg::ConstantBuffer => {
                let i = op.index.get(1).ok_or(Error::Truncated(0))?;
                format!("cb{n}.v[{}]", self.index(i)?)
            }
            Reg::ImmediateConstantBuffer => {
                let i = op.index.first().ok_or(Error::Truncated(0))?;
                format!("icb[{}]", self.index(i)?)
            }
            Reg::Immediate => {
                let v: Vec<u32> = if op.imm.len() == 1 {
                    vec![op.imm[0]; 4]
                } else {
                    op.imm.clone()
                };
                format!(
                    "vec4<u32>({:#x}u, {:#x}u, {:#x}u, {:#x}u)",
                    v[0], v[1], v[2], v[3]
                )
            }
            Reg::PrimitiveId => "vec4<u32>(prim_id)".into(),
            Reg::OutputDepth => "vec4<u32>(odepth)".into(),
            Reg::Null | Reg::Sampler | Reg::Resource | Reg::Other(_) => {
                return Err(Error::Truncated(0));
            }
        })
    }

    /// A source operand as `vec4<ty>`, swizzled, with its modifiers.
    fn src(&self, op: &Operand, ty: Ty) -> Result<String> {
        let raw = format!("{}{}", self.base(op)?, swizzle(op.select));
        let mut e = match ty {
            Ty::U32 => raw,
            Ty::F32 => format!("bitcast<vec4<f32>>({raw})"),
            Ty::I32 => format!("bitcast<vec4<i32>>({raw})"),
        };
        if op.abs {
            e = format!("abs({e})");
        }
        if op.neg {
            e = format!("(-{e})");
        }
        Ok(e)
    }

    /// One component of an operand as `u32` (indices, conditions).
    fn scalar_u(&self, op: &Operand) -> Result<String> {
        let c = match op.select {
            Select::One(c) => c,
            Select::Swizzle(s) => s[0],
            _ => 0,
        };
        Ok(format!("{}.{}", self.base(op)?, SWZ[c as usize]))
    }

    /// Writes `value` (a `vec4<ty>`) to the destination's masked
    /// components, saturated when asked.
    fn store(&mut self, dst: &Operand, value: &str, ty: Ty, saturate: bool) -> Result<()> {
        if dst.reg == Reg::Null {
            return Ok(());
        }
        let mut v = value.to_owned();
        if saturate && ty == Ty::F32 {
            v = format!("clamp({v}, vec4<f32>(0.0), vec4<f32>(1.0))");
        }
        let raw = match ty {
            Ty::U32 => v,
            _ => format!("bitcast<vec4<u32>>({v})"),
        };
        if dst.reg == Reg::OutputDepth {
            self.line(&format!("odepth = ({raw}).x;"));
            return Ok(());
        }
        let target = self.base(dst)?;
        let mask = dst.mask();
        if mask == 0xf {
            self.line(&format!("{target} = {raw};"));
            return Ok(());
        }
        self.line(&format!("{{ let t = {raw};"));
        for (i, c) in SWZ.iter().enumerate() {
            if mask & (1 << i) != 0 {
                self.line(&format!("    {target}.{c} = t.{c};"));
            }
        }
        self.line("}");
        Ok(())
    }

    fn binary(&mut self, i: &Instruction, ty: Ty, f: &str) -> Result<()> {
        let a = self.src(&i.operands[1], ty)?;
        let b = self.src(&i.operands[2], ty)?;
        let e = if f.len() <= 2 {
            format!("({a} {f} {b})")
        } else {
            format!("{f}({a}, {b})")
        };
        self.store(&i.operands[0], &e, ty, i.saturate)
    }

    fn unary(&mut self, i: &Instruction, ty: Ty, f: &str) -> Result<()> {
        let a = self.src(&i.operands[1], ty)?;
        self.store(&i.operands[0], &format!("{f}({a})"), ty, i.saturate)
    }

    fn compare(&mut self, i: &Instruction, ty: Ty, f: &str) -> Result<()> {
        let a = self.src(&i.operands[1], ty)?;
        let b = self.src(&i.operands[2], ty)?;
        let e = format!("select(vec4<u32>(0u), vec4<u32>(0xffffffffu), {a} {f} {b})");
        self.store(&i.operands[0], &e, Ty::U32, false)
    }

    fn dot(&mut self, i: &Instruction, n: usize) -> Result<()> {
        let comps = &"xyzw"[..n];
        let a = self.src(&i.operands[1], Ty::F32)?;
        let b = self.src(&i.operands[2], Ty::F32)?;
        let e = format!("vec4<f32>(dot(({a}).{comps}, ({b}).{comps}))");
        self.store(&i.operands[0], &e, Ty::F32, i.saturate)
    }

    fn texture(&self, op: &Operand) -> Result<&TextureBinding> {
        self.textures.get(&op.number()).ok_or(Error::Truncated(0))
    }

    fn coords(dim: Dim) -> &'static str {
        match dim {
            Dim::Texture1D => "x",
            Dim::Texture3D | Dim::TextureCube => "xyz",
            _ => "xy",
        }
    }

    /// The texture result (a `vec4` of the texture's type) swizzled by the
    /// resource operand, as raw bits.
    fn texel(value: &str, resource: &Operand, kind: TextureKind) -> String {
        let swz = swizzle(resource.select);
        match kind {
            TextureKind::Uint => format!("({value}){swz}"),
            _ => format!("bitcast<vec4<u32>>(({value}){swz})"),
        }
    }

    #[allow(clippy::too_many_lines)] // one arm per opcode
    fn instruction(&mut self, i: &Instruction) -> Result<()> {
        use Op as O;
        let o = &i.operands;
        match i.op {
            O::Add => self.binary(i, Ty::F32, "+")?,
            O::Mul => self.binary(i, Ty::F32, "*")?,
            O::Div => self.binary(i, Ty::F32, "/")?,
            O::Min => self.binary(i, Ty::F32, "min")?,
            O::Max => self.binary(i, Ty::F32, "max")?,
            O::Mad => {
                let a = self.src(&o[1], Ty::F32)?;
                let b = self.src(&o[2], Ty::F32)?;
                let c = self.src(&o[3], Ty::F32)?;
                self.store(&o[0], &format!("({a} * {b} + {c})"), Ty::F32, i.saturate)?;
            }
            O::Dp2 => self.dot(i, 2)?,
            O::Dp3 => self.dot(i, 3)?,
            O::Dp4 => self.dot(i, 4)?,
            O::Exp => self.unary(i, Ty::F32, "exp2")?,
            O::Log => self.unary(i, Ty::F32, "log2")?,
            O::Rsq => self.unary(i, Ty::F32, "inverseSqrt")?,
            O::Sqrt => self.unary(i, Ty::F32, "sqrt")?,
            O::Frc => self.unary(i, Ty::F32, "fract")?,
            O::RoundNe => self.unary(i, Ty::F32, "round")?,
            O::RoundNi => self.unary(i, Ty::F32, "floor")?,
            O::RoundPi => self.unary(i, Ty::F32, "ceil")?,
            O::RoundZ => self.unary(i, Ty::F32, "trunc")?,
            O::DerivRtx => self.unary(i, Ty::F32, "dpdx")?,
            O::DerivRty => self.unary(i, Ty::F32, "dpdy")?,
            O::Sincos => {
                let a = self.src(&o[2], Ty::F32)?;
                self.store(&o[0], &format!("sin({a})"), Ty::F32, i.saturate)?;
                self.store(&o[1], &format!("cos({a})"), Ty::F32, i.saturate)?;
            }
            O::Mov => {
                if o[1].neg || o[1].abs || i.saturate {
                    let a = self.src(&o[1], Ty::F32)?;
                    self.store(&o[0], &a, Ty::F32, i.saturate)?;
                } else {
                    let a = self.src(&o[1], Ty::U32)?;
                    self.store(&o[0], &a, Ty::U32, false)?;
                }
            }
            O::Movc => {
                let c = self.src(&o[1], Ty::U32)?;
                let float = o[2].neg || o[2].abs || o[3].neg || o[3].abs || i.saturate;
                let ty = if float { Ty::F32 } else { Ty::U32 };
                let a = self.src(&o[2], ty)?;
                let b = self.src(&o[3], ty)?;
                let e = format!("select({b}, {a}, {c} != vec4<u32>(0u))");
                self.store(&o[0], &e, ty, i.saturate)?;
            }
            O::Eq => self.compare(i, Ty::F32, "==")?,
            O::Ne => self.compare(i, Ty::F32, "!=")?,
            O::Lt => self.compare(i, Ty::F32, "<")?,
            O::Ge => self.compare(i, Ty::F32, ">=")?,
            O::Ieq => self.compare(i, Ty::I32, "==")?,
            O::Ine => self.compare(i, Ty::I32, "!=")?,
            O::Ilt => self.compare(i, Ty::I32, "<")?,
            O::Ige => self.compare(i, Ty::I32, ">=")?,
            O::Ult => self.compare(i, Ty::U32, "<")?,
            O::Uge => self.compare(i, Ty::U32, ">=")?,
            O::And => self.binary(i, Ty::U32, "&")?,
            O::Or => self.binary(i, Ty::U32, "|")?,
            O::Xor => self.binary(i, Ty::U32, "^")?,
            O::Not => self.unary(i, Ty::U32, "~")?,
            O::Iadd => self.binary(i, Ty::I32, "+")?,
            O::Imax => self.binary(i, Ty::I32, "max")?,
            O::Imin => self.binary(i, Ty::I32, "min")?,
            O::Umax => self.binary(i, Ty::U32, "max")?,
            O::Umin => self.binary(i, Ty::U32, "min")?,
            O::Ineg => self.unary(i, Ty::I32, "-")?,
            O::Ishl => {
                let a = self.src(&o[1], Ty::I32)?;
                let b = self.src(&o[2], Ty::U32)?;
                self.store(
                    &o[0],
                    &format!("({a} << ({b} & vec4<u32>(31u)))"),
                    Ty::I32,
                    false,
                )?;
            }
            O::Ishr => {
                let a = self.src(&o[1], Ty::I32)?;
                let b = self.src(&o[2], Ty::U32)?;
                self.store(
                    &o[0],
                    &format!("({a} >> ({b} & vec4<u32>(31u)))"),
                    Ty::I32,
                    false,
                )?;
            }
            O::Ushr => {
                let a = self.src(&o[1], Ty::U32)?;
                let b = self.src(&o[2], Ty::U32)?;
                self.store(
                    &o[0],
                    &format!("({a} >> ({b} & vec4<u32>(31u)))"),
                    Ty::U32,
                    false,
                )?;
            }
            O::Imul | O::Umul => {
                let ty = if i.op == O::Imul { Ty::I32 } else { Ty::U32 };
                let a = self.src(&o[2], ty)?;
                let b = self.src(&o[3], ty)?;
                // The high half is rarely used; only the low one is kept.
                self.store(&o[1], &format!("({a} * {b})"), ty, false)?;
            }
            O::Imad => {
                let a = self.src(&o[1], Ty::I32)?;
                let b = self.src(&o[2], Ty::I32)?;
                let c = self.src(&o[3], Ty::I32)?;
                self.store(&o[0], &format!("({a} * {b} + {c})"), Ty::I32, false)?;
            }
            O::Udiv => {
                let a = self.src(&o[2], Ty::U32)?;
                let b = self.src(&o[3], Ty::U32)?;
                self.store(&o[0], &format!("({a} / {b})"), Ty::U32, false)?;
                self.store(&o[1], &format!("({a} % {b})"), Ty::U32, false)?;
            }
            O::Itof => {
                let a = self.src(&o[1], Ty::I32)?;
                self.store(&o[0], &format!("vec4<f32>({a})"), Ty::F32, i.saturate)?;
            }
            O::Utof => {
                let a = self.src(&o[1], Ty::U32)?;
                self.store(&o[0], &format!("vec4<f32>({a})"), Ty::F32, i.saturate)?;
            }
            O::Ftoi => {
                let a = self.src(&o[1], Ty::F32)?;
                self.store(&o[0], &format!("vec4<i32>({a})"), Ty::I32, false)?;
            }
            O::Ftou => {
                let a = self.src(&o[1], Ty::F32)?;
                self.store(&o[0], &format!("vec4<u32>({a})"), Ty::U32, false)?;
            }
            O::If => {
                let c = self.scalar_u(&o[0])?;
                let test = if i.nonzero { "!=" } else { "==" };
                self.line(&format!("if ({c} {test} 0u) {{"));
                self.depth += 1;
            }
            O::Else => {
                self.depth -= 1;
                self.line("} else {");
                self.depth += 1;
            }
            O::EndIf | O::EndLoop => {
                self.depth -= 1;
                self.line("}");
            }
            O::Loop => {
                self.line("loop {");
                self.depth += 1;
            }
            O::Break => self.line("break;"),
            O::Continue => self.line("continue;"),
            O::Breakc | O::Continuec | O::Retc | O::Discard => {
                let c = self.scalar_u(&o[0])?;
                let test = if i.nonzero { "!=" } else { "==" };
                let what = match i.op {
                    O::Breakc => "break",
                    O::Continuec => "continue",
                    O::Retc => "return",
                    _ => "discard",
                };
                self.line(&format!("if ({c} {test} 0u) {{ {what}; }}"));
            }
            O::Ret => self.line("return;"),
            O::Nop => {}
            O::Sample | O::SampleL | O::SampleB | O::SampleD | O::SampleC | O::SampleCLz => {
                let t = self.texture(&o[2])?.clone();
                let coord = self.src(&o[1], Ty::F32)?;
                let c = format!("({coord}).{}", Self::coords(t.dim));
                let (tn, sn) = (o[2].number(), o[3].number());
                let off = if i.offset == [0, 0, 0] || t.dim == Dim::TextureCube {
                    String::new()
                } else if t.dim == Dim::Texture3D {
                    format!(
                        ", vec3<i32>({}, {}, {})",
                        i.offset[0], i.offset[1], i.offset[2]
                    )
                } else {
                    format!(", vec2<i32>({}, {})", i.offset[0], i.offset[1])
                };
                let value = match i.op {
                    O::Sample if !self.vertex => format!("textureSample(t{tn}, s{sn}, {c}{off})"),
                    O::Sample => format!("textureSampleLevel(t{tn}, s{sn}, {c}, 0.0{off})"),
                    O::SampleL => {
                        let l = self.src(&o[4], Ty::F32)?;
                        format!("textureSampleLevel(t{tn}, s{sn}, {c}, ({l}).x{off})")
                    }
                    O::SampleB => {
                        let b = self.src(&o[4], Ty::F32)?;
                        if self.vertex {
                            format!("textureSampleLevel(t{tn}, s{sn}, {c}, ({b}).x{off})")
                        } else {
                            format!("textureSampleBias(t{tn}, s{sn}, {c}, ({b}).x{off})")
                        }
                    }
                    O::SampleD => {
                        let dx = self.src(&o[4], Ty::F32)?;
                        let dy = self.src(&o[5], Ty::F32)?;
                        let cc = Self::coords(t.dim);
                        format!(
                            "textureSampleGrad(t{tn}, s{sn}, {c}, ({dx}).{cc}, ({dy}).{cc}{off})"
                        )
                    }
                    O::SampleC | O::SampleCLz => {
                        let r = self.src(&o[4], Ty::F32)?;
                        let f = if i.op == O::SampleC && !self.vertex {
                            "textureSampleCompare"
                        } else {
                            "textureSampleCompareLevel"
                        };
                        format!("vec4<f32>({f}(t{tn}, s{sn}, {c}, ({r}).x{off}))")
                    }
                    _ => unreachable!(),
                };
                let raw = Self::texel(&value, &o[2], t.kind);
                self.store(&o[0], &raw, Ty::U32, false)?;
                if i.saturate {
                    let a = self.src(&o[0], Ty::F32)?;
                    self.store(&o[0], &a, Ty::F32, true)?;
                }
            }
            O::Ld => {
                let t = self.texture(&o[2])?.clone();
                let c = self.src(&o[1], Ty::I32)?;
                let tn = o[2].number();
                let value = match t.dim {
                    Dim::Buffer | Dim::Texture1D => format!("textureLoad(t{tn}, ({c}).x, ({c}).y)"),
                    Dim::Texture3D => format!("textureLoad(t{tn}, ({c}).xyz, ({c}).w)"),
                    _ => format!("textureLoad(t{tn}, ({c}).xy, ({c}).w)"),
                };
                let raw = Self::texel(&value, &o[2], t.kind);
                self.store(&o[0], &raw, Ty::U32, false)?;
            }
            O::LdMs => {
                let t = self.texture(&o[2])?.clone();
                let c = self.src(&o[1], Ty::I32)?;
                let s = self.src(&o[3], Ty::I32)?;
                let tn = o[2].number();
                let value = format!("textureLoad(t{tn}, ({c}).xy, ({s}).x)");
                let raw = Self::texel(&value, &o[2], t.kind);
                self.store(&o[0], &raw, Ty::U32, false)?;
            }
            O::Resinfo => {
                let mip = self.scalar_u(&o[1])?;
                let tn = o[2].number();
                let t = self.texture(&o[2])?.clone();
                let swz = swizzle(o[2].select);
                let dims = if t.kind == TextureKind::Multisampled {
                    format!("textureDimensions(t{tn})")
                } else {
                    format!("textureDimensions(t{tn}, {mip})")
                };
                let levels = if t.kind == TextureKind::Multisampled {
                    "1u".to_owned()
                } else {
                    format!("textureNumLevels(t{tn})")
                };
                // Return type: 0 float, 1 reciprocal float, 2 uint.
                let e = match i.controls & 3 {
                    2 => format!("vec4<u32>({dims}.x, {dims}.y, 0u, {levels}){swz}"),
                    1 => format!(
                        "bitcast<vec4<u32>>(vec4<f32>(1.0 / f32({dims}.x), 1.0 / f32({dims}.y), 0.0, f32({levels})){swz})"
                    ),
                    _ => format!(
                        "bitcast<vec4<u32>>(vec4<f32>(f32({dims}.x), f32({dims}.y), 0.0, f32({levels})){swz})"
                    ),
                };
                self.store(&o[0], &e, Ty::U32, false)?;
            }
        }
        Ok(())
    }
}

fn texture_type(t: &TextureBinding) -> String {
    let dim = match t.dim {
        Dim::Texture1D => "1d",
        Dim::Texture3D => "3d",
        Dim::TextureCube => "cube",
        _ => "2d",
    };
    match t.kind {
        TextureKind::Depth => match t.dim {
            Dim::TextureCube => "texture_depth_cube".into(),
            _ => "texture_depth_2d".into(),
        },
        TextureKind::Multisampled => "texture_multisampled_2d<f32>".into(),
        TextureKind::Uint => format!("texture_{dim}<u32>"),
        TextureKind::Sint => format!("texture_{dim}<i32>"),
        TextureKind::Float => format!("texture_{dim}<f32>"),
    }
}

/// Translates one stage. `varyings` links it to the other stage (from
/// [`link`]); `group` is its bind group.
///
/// # Errors
/// When the bytecode does not decode or uses something unsupported.
#[allow(clippy::too_many_lines)] // one pass over declarations and code
pub fn translate(c: &Container, varyings: &[Varying], group: u32) -> Result<Shader> {
    let program = c.program_code()?;
    let vertex = program.kind == 1;
    if !matches!(program.kind, 0 | 1) {
        return Err(Error::Magic);
    }
    let inputs = c.inputs()?;
    let outputs = c.outputs()?;
    let comparison: BTreeSet<u32> = program
        .code
        .iter()
        .filter(|i| matches!(i.op, Op::SampleC | Op::SampleCLz))
        .map(|i| i.operands[2].number())
        .collect();
    let mut textures = BTreeMap::new();
    let mut samplers = Vec::new();
    let mut buffers = Vec::new();
    let mut temps = 0;
    let mut indexable = Vec::new();
    let mut icb = None;
    let mut input_regs = BTreeSet::new();
    let mut output_regs = BTreeSet::new();
    for d in &program.decls {
        match d {
            Decl::ConstantBuffer { slot, size, .. } => buffers.push((*slot, size.max(&1) * 16)),
            Decl::Sampler { slot, comparison } => samplers.push((*slot, *comparison)),
            Decl::Resource { slot, dim, ret } => {
                let kind = if comparison.contains(slot) {
                    TextureKind::Depth
                } else if matches!(dim, Dim::Texture2DMs(_)) {
                    TextureKind::Multisampled
                } else {
                    match ret {
                        3 => TextureKind::Sint,
                        4 => TextureKind::Uint,
                        _ => TextureKind::Float,
                    }
                };
                textures.insert(
                    *slot,
                    TextureBinding {
                        slot: *slot,
                        dim: *dim,
                        kind,
                    },
                );
            }
            Decl::Temps(n) => temps = *n,
            Decl::IndexableTemp { reg, size, .. } => indexable.push((*reg, *size)),
            Decl::ImmediateConstants(v) => icb = Some(v.clone()),
            Decl::Input { reg, .. } => {
                input_regs.insert(*reg);
            }
            Decl::Output { reg, .. } => {
                output_regs.insert(*reg);
            }
            _ => {}
        }
    }
    for s in &inputs {
        input_regs.insert(s.register);
    }
    for s in &outputs {
        if !is_depth(s) {
            output_regs.insert(s.register);
        }
    }
    let mut g = Gen {
        vertex,
        out: String::new(),
        depth: 0,
        textures,
    };
    let mut head = String::new();
    head.push_str("diagnostic(off, derivative_uniformity);\n\n");
    for (slot, size) in &buffers {
        let _ = writeln!(
            head,
            "struct Cb{slot} {{ v: array<vec4<u32>, {}> }}\n@group({group}) @binding({slot}) var<uniform> cb{slot}: Cb{slot};",
            size / 16
        );
    }
    for t in g.textures.values() {
        let _ = writeln!(
            head,
            "@group({group}) @binding({}) var t{}: {};",
            TEXTURE_BINDING + t.slot,
            t.slot,
            texture_type(t)
        );
    }
    for (slot, cmp) in &samplers {
        let _ = writeln!(
            head,
            "@group({group}) @binding({}) var s{slot}: {};",
            SAMPLER_BINDING + slot,
            if *cmp {
                "sampler_comparison"
            } else {
                "sampler"
            }
        );
    }
    for r in 0..temps {
        let _ = writeln!(head, "var<private> r{r}: vec4<u32>;");
    }
    for r in &input_regs {
        let _ = writeln!(head, "var<private> v{r}: vec4<u32>;");
    }
    for r in &output_regs {
        let _ = writeln!(head, "var<private> o{r}: vec4<u32>;");
    }
    for (reg, size) in &indexable {
        let _ = writeln!(head, "var<private> x{reg}: array<vec4<u32>, {size}>;");
    }
    let writes_depth = outputs.iter().any(is_depth)
        || program
            .decls
            .iter()
            .any(|d| matches!(d, Decl::OutputSpecial(Reg::OutputDepth)));
    if writes_depth {
        head.push_str("var<private> odepth: u32;\n");
    }
    head.push_str("var<private> prim_id: u32;\n");
    if let Some(v) = &icb {
        let _ = write!(
            head,
            "var<private> icb: array<vec4<u32>, {}> = array<vec4<u32>, {}>(",
            v.len(),
            v.len()
        );
        for (k, e) in v.iter().enumerate() {
            if k > 0 {
                head.push_str(", ");
            }
            let _ = write!(
                head,
                "vec4<u32>({:#x}u, {:#x}u, {:#x}u, {:#x}u)",
                e[0], e[1], e[2], e[3]
            );
        }
        head.push_str(");\n");
    }

    // The program.
    g.line("fn run() {");
    g.depth = 1;
    for i in &program.code {
        g.instruction(i)?;
    }
    g.depth = 0;
    g.line("}");
    let body = std::mem::take(&mut g.out);

    // Entry point: unpack inputs, run, pack outputs.
    let mut io = String::new();
    let mut entry = String::new();
    let mut vertex_inputs = Vec::new();
    let mut targets = Vec::new();
    let comp =
        |start: u32, n: u32| -> String { (start..start + n).map(|k| SWZ[k as usize]).collect() };
    if vertex {
        io.push_str("struct In {\n");
        for (k, s) in inputs.iter().enumerate() {
            match s.system {
                6 => {
                    let _ = writeln!(io, "    @builtin(vertex_index) vid{k}: u32,");
                }
                8 => {
                    let _ = writeln!(io, "    @builtin(instance_index) iid{k}: u32,");
                }
                _ => {
                    let ty = Ty::from_component(s.component);
                    let _ = writeln!(io, "    @location({k}) a{k}: {},", ty.vec(4));
                    vertex_inputs.push(VertexInput {
                        semantic: s.semantic.clone(),
                        index: s.index,
                        location: k as u32,
                        ty,
                    });
                }
            }
        }
        if inputs.is_empty() {
            io.push_str("    @builtin(vertex_index) unused: u32,\n");
        }
        io.push_str("}\nstruct Out {\n");
        let position = position_register(&outputs);
        if let Some(r) = position {
            let _ = writeln!(io, "    @builtin(position) pos{r}: vec4<f32>,");
        }
        for v in varyings {
            let _ = writeln!(
                io,
                "    @location({}) {}e{}: {},",
                v.location,
                v.interp,
                v.location,
                v.ty.vec(v.width)
            );
        }
        io.push_str("}\n");
        entry.push_str("@vertex\nfn main(input: In) -> Out {\n");
        for (k, s) in inputs.iter().enumerate() {
            let (start, n) = components(s.mask);
            let r = s.register;
            match s.system {
                6 => {
                    let _ = writeln!(entry, "    v{r}.{} = input.vid{k};", SWZ[start as usize]);
                }
                8 => {
                    let _ = writeln!(entry, "    v{r}.{} = input.iid{k};", SWZ[start as usize]);
                }
                _ => {
                    let ty = Ty::from_component(s.component);
                    let raw = match ty {
                        Ty::U32 => format!("input.a{k}"),
                        _ => format!("bitcast<vec4<u32>>(input.a{k})"),
                    };
                    for j in start..start + n {
                        let c = SWZ[j as usize];
                        let _ = writeln!(
                            entry,
                            "    v{r}.{c} = ({raw}).{};",
                            SWZ[(j - start) as usize]
                        );
                    }
                }
            }
        }
        entry.push_str("    run();\n    var out: Out;\n");
        if let Some(r) = position {
            let _ = writeln!(entry, "    out.pos{r} = bitcast<vec4<f32>>(o{r});");
        }
        for v in varyings {
            let src = outputs.iter().find(|o| same(o, &v.semantic, v.index));
            let value = match src {
                Some(o) => {
                    let (start, _) = components(o.mask);
                    let raw = format!("o{}.{}", o.register, comp(start, v.width.min(4 - start)));
                    match v.ty {
                        Ty::U32 => raw,
                        t => format!("bitcast<{}>({raw})", t.vec(v.width)),
                    }
                }
                None => format!("{}()", v.ty.vec(v.width)),
            };
            let _ = writeln!(entry, "    out.e{} = {value};", v.location);
        }
        entry.push_str("    return out;\n}\n");
    } else {
        io.push_str("struct In {\n");
        let mut any = false;
        for s in &inputs {
            match s.system {
                1 => {
                    let _ = writeln!(io, "    @builtin(position) pos: vec4<f32>,");
                    any = true;
                }
                9 => {
                    let _ = writeln!(io, "    @builtin(front_facing) front: bool,");
                    any = true;
                }
                0 => {
                    if let Some(v) = varyings.iter().find(|v| same(s, &v.semantic, v.index)) {
                        let _ = writeln!(
                            io,
                            "    @location({}) {}e{}: {},",
                            v.location,
                            v.interp,
                            v.location,
                            v.ty.vec(v.width)
                        );
                        any = true;
                    }
                }
                _ => {}
            }
        }
        if !any {
            io.push_str("    @builtin(position) unused_pos: vec4<f32>,\n");
        }
        io.push_str("}\nstruct Out {\n");
        let mut any_out = false;
        for s in &outputs {
            if is_target(s) {
                let ty = Ty::from_component(s.component);
                let _ = writeln!(io, "    @location({0}) c{0}: {1},", s.register, ty.vec(4));
                targets.push((s.register, ty));
                any_out = true;
            } else if is_depth(s) {
                io.push_str("    @builtin(frag_depth) depth: f32,\n");
                any_out = true;
            }
        }
        io.push_str("}\n");
        let returns = if any_out { " -> Out" } else { "" };
        let _ = writeln!(entry, "@fragment\nfn main(input: In){returns} {{");
        for s in &inputs {
            let (start, n) = components(s.mask);
            let r = s.register;
            match s.system {
                1 => {
                    let _ = writeln!(entry, "    v{r} = bitcast<vec4<u32>>(input.pos);");
                }
                9 => {
                    let _ = writeln!(
                        entry,
                        "    v{r}.{} = select(0u, 0xffffffffu, input.front);",
                        SWZ[start as usize]
                    );
                }
                0 => {
                    if let Some(v) = varyings.iter().find(|v| same(s, &v.semantic, v.index)) {
                        for j in 0..n.min(v.width) {
                            let value = if v.width == 1 {
                                format!("input.e{}", v.location)
                            } else {
                                format!("input.e{}.{}", v.location, SWZ[j as usize])
                            };
                            let value = match v.ty {
                                Ty::U32 => value,
                                _ => format!("bitcast<u32>({value})"),
                            };
                            let _ = writeln!(
                                entry,
                                "    v{r}.{} = {value};",
                                SWZ[(start + j) as usize]
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        entry.push_str("    run();\n");
        if any_out {
            entry.push_str("    var out: Out;\n");
            for s in &outputs {
                if is_target(s) {
                    let ty = Ty::from_component(s.component);
                    let v = match ty {
                        Ty::U32 => format!("o{}", s.register),
                        t => format!("bitcast<{}>(o{})", t.vec(4), s.register),
                    };
                    let _ = writeln!(entry, "    out.c{} = {v};", s.register);
                } else if is_depth(s) {
                    entry.push_str("    out.depth = bitcast<f32>(odepth);\n");
                }
            }
            entry.push_str("    return out;\n");
        }
        entry.push_str("}\n");
    }
    let textures: Vec<TextureBinding> = g.textures.values().cloned().collect();
    Ok(Shader {
        wgsl: format!("{head}\n{io}\n{body}\n{entry}"),
        vertex,
        buffers,
        textures,
        samplers,
        inputs: vertex_inputs,
        targets,
        writes_depth,
    })
}

/// The varyings of a vertex shader on its own (all its outputs), for
/// translating it without a pixel shader.
///
/// # Errors
/// When the signature does not parse.
pub fn own_varyings(vs: &Container) -> Result<Vec<Varying>> {
    Ok(vs
        .outputs()?
        .iter()
        .filter(|o| o.system == 0 && !same(o, "POSITION", 0))
        .enumerate()
        .map(|(k, o)| {
            let (start, width) = components(o.mask);
            let ty = Ty::from_component(o.component);
            Varying {
                semantic: o.semantic.clone(),
                index: o.index,
                location: k as u32,
                width: width.max(1),
                start,
                ty,
                interp: if ty == Ty::F32 {
                    String::new()
                } else {
                    "@interpolate(flat) ".into()
                },
            }
        })
        .collect())
}

/// The varyings of a pixel shader on its own (all its inputs).
///
/// # Errors
/// When the signature does not parse.
pub fn ps_varyings(ps: &Container) -> Result<Vec<Varying>> {
    let inputs = ps.inputs()?;
    let program = ps.program_code()?;
    Ok(inputs
        .iter()
        .filter(|e| e.system == 0)
        .enumerate()
        .map(|(k, e)| {
            let (start, width) = components(e.mask);
            let ty = Ty::from_component(e.component);
            let constant = program.decls.iter().any(|d| {
                matches!(d, Decl::Input { reg, interp: Some(Interp::Constant), .. } if *reg == e.register)
            });
            Varying {
                semantic: e.semantic.clone(),
                index: e.index,
                location: k as u32,
                width: width.max(1),
                start,
                ty,
                interp: if ty != Ty::F32 || constant {
                    "@interpolate(flat) ".into()
                } else {
                    String::new()
                },
            }
        })
        .collect())
}
