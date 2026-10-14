//! Shader Model 4 bytecode (the `SHDR` chunk): declarations and
//! instructions, decoded from the token stream.
//!
//! Token layout (the D3D10/11 tokenized program format):
//!
//! - Opcode token: bits 0–10 opcode, 11–23 opcode controls (saturate is
//!   bit 13, the test is bit 18: non-zero when set), 24–30 length in
//!   tokens, bit 31 an extended opcode token follows.
//! - Operand token: bits 0–1 components (0, 1, 4, n), 2–3 selection mode
//!   (mask, swizzle, select-1), 4–11 the mask, swizzle or component,
//!   12–19 operand type, 20–21 index dimension, 22–30 how each index is
//!   given (immediate, relative, immediate plus relative), bit 31 an
//!   extended operand token (modifiers: neg, abs) follows.

use std::fmt::Write as _;

use crate::{Error, Result};

/// The opcodes the game's shaders use, plus the common ones around them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Op {
    Add,
    And,
    Break,
    Breakc,
    Continue,
    Continuec,
    DerivRtx,
    DerivRty,
    Discard,
    Div,
    Dp2,
    Dp3,
    Dp4,
    Else,
    EndIf,
    EndLoop,
    Eq,
    Exp,
    Frc,
    Ftoi,
    Ftou,
    Ge,
    Iadd,
    If,
    Ieq,
    Ige,
    Ilt,
    Imad,
    Imax,
    Imin,
    Imul,
    Ine,
    Ineg,
    Ishl,
    Ishr,
    Itof,
    Ld,
    LdMs,
    Log,
    Loop,
    Lt,
    Mad,
    Min,
    Max,
    Mov,
    Movc,
    Mul,
    Ne,
    Nop,
    Not,
    Or,
    Resinfo,
    Ret,
    Retc,
    RoundNe,
    RoundNi,
    RoundPi,
    RoundZ,
    Rsq,
    Sample,
    SampleC,
    SampleCLz,
    SampleL,
    SampleD,
    SampleB,
    Sqrt,
    Sincos,
    Udiv,
    Ult,
    Uge,
    Umul,
    Umax,
    Umin,
    Ushr,
    Utof,
    Xor,
}

impl Op {
    fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Add,
            1 => Self::And,
            2 => Self::Break,
            3 => Self::Breakc,
            7 => Self::Continue,
            8 => Self::Continuec,
            11 => Self::DerivRtx,
            12 => Self::DerivRty,
            13 => Self::Discard,
            14 => Self::Div,
            15 => Self::Dp2,
            16 => Self::Dp3,
            17 => Self::Dp4,
            18 => Self::Else,
            21 => Self::EndIf,
            22 => Self::EndLoop,
            24 => Self::Eq,
            25 => Self::Exp,
            26 => Self::Frc,
            27 => Self::Ftoi,
            28 => Self::Ftou,
            29 => Self::Ge,
            30 => Self::Iadd,
            31 => Self::If,
            32 => Self::Ieq,
            33 => Self::Ige,
            34 => Self::Ilt,
            35 => Self::Imad,
            36 => Self::Imax,
            37 => Self::Imin,
            38 => Self::Imul,
            39 => Self::Ine,
            40 => Self::Ineg,
            41 => Self::Ishl,
            42 => Self::Ishr,
            43 => Self::Itof,
            45 => Self::Ld,
            46 => Self::LdMs,
            47 => Self::Log,
            48 => Self::Loop,
            49 => Self::Lt,
            50 => Self::Mad,
            51 => Self::Min,
            52 => Self::Max,
            54 => Self::Mov,
            55 => Self::Movc,
            56 => Self::Mul,
            57 => Self::Ne,
            58 => Self::Nop,
            59 => Self::Not,
            60 => Self::Or,
            61 => Self::Resinfo,
            62 => Self::Ret,
            63 => Self::Retc,
            64 => Self::RoundNe,
            65 => Self::RoundNi,
            66 => Self::RoundPi,
            67 => Self::RoundZ,
            68 => Self::Rsq,
            69 => Self::Sample,
            70 => Self::SampleC,
            71 => Self::SampleCLz,
            72 => Self::SampleL,
            73 => Self::SampleD,
            74 => Self::SampleB,
            75 => Self::Sqrt,
            77 => Self::Sincos,
            78 => Self::Udiv,
            79 => Self::Ult,
            80 => Self::Uge,
            81 => Self::Umul,
            83 => Self::Umax,
            84 => Self::Umin,
            85 => Self::Ushr,
            86 => Self::Utof,
            87 => Self::Xor,
            _ => return None,
        })
    }

    /// The disassembler's name, for comparing with `D3DDisassemble`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::And => "and",
            Self::Break => "break",
            Self::Breakc => "breakc",
            Self::Continue => "continue",
            Self::Continuec => "continuec",
            Self::DerivRtx => "deriv_rtx",
            Self::DerivRty => "deriv_rty",
            Self::Discard => "discard",
            Self::Div => "div",
            Self::Dp2 => "dp2",
            Self::Dp3 => "dp3",
            Self::Dp4 => "dp4",
            Self::Else => "else",
            Self::EndIf => "endif",
            Self::EndLoop => "endloop",
            Self::Eq => "eq",
            Self::Exp => "exp",
            Self::Frc => "frc",
            Self::Ftoi => "ftoi",
            Self::Ftou => "ftou",
            Self::Ge => "ge",
            Self::Iadd => "iadd",
            Self::If => "if",
            Self::Ieq => "ieq",
            Self::Ige => "ige",
            Self::Ilt => "ilt",
            Self::Imad => "imad",
            Self::Imax => "imax",
            Self::Imin => "imin",
            Self::Imul => "imul",
            Self::Ine => "ine",
            Self::Ineg => "ineg",
            Self::Ishl => "ishl",
            Self::Ishr => "ishr",
            Self::Itof => "itof",
            Self::Ld => "ld",
            Self::LdMs => "ldms",
            Self::Log => "log",
            Self::Loop => "loop",
            Self::Lt => "lt",
            Self::Mad => "mad",
            Self::Min => "min",
            Self::Max => "max",
            Self::Mov => "mov",
            Self::Movc => "movc",
            Self::Mul => "mul",
            Self::Ne => "ne",
            Self::Nop => "nop",
            Self::Not => "not",
            Self::Or => "or",
            Self::Resinfo => "resinfo",
            Self::Ret => "ret",
            Self::Retc => "retc",
            Self::RoundNe => "round_ne",
            Self::RoundNi => "round_ni",
            Self::RoundPi => "round_pi",
            Self::RoundZ => "round_z",
            Self::Rsq => "rsq",
            Self::Sample => "sample",
            Self::SampleC => "sample_c",
            Self::SampleCLz => "sample_c_lz",
            Self::SampleL => "sample_l",
            Self::SampleD => "sample_d",
            Self::SampleB => "sample_b",
            Self::Sqrt => "sqrt",
            Self::Sincos => "sincos",
            Self::Udiv => "udiv",
            Self::Ult => "ult",
            Self::Uge => "uge",
            Self::Umul => "umul",
            Self::Umax => "umax",
            Self::Umin => "umin",
            Self::Ushr => "ushr",
            Self::Utof => "utof",
            Self::Xor => "xor",
        }
    }
}

/// What an operand names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    Temp,
    Input,
    Output,
    IndexableTemp,
    Immediate,
    Sampler,
    Resource,
    ConstantBuffer,
    ImmediateConstantBuffer,
    PrimitiveId,
    OutputDepth,
    Null,
    Other(u32),
}

/// How an operand's components are chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Select {
    /// Destination write mask (bit 0 = x).
    Mask(u8),
    /// Source swizzle: the component read for x, y, z, w.
    Swizzle([u8; 4]),
    /// One component, replicated.
    One(u8),
    /// A scalar operand (or none).
    Scalar,
}

/// One index of an operand: an immediate, a register plus an immediate.
#[derive(Debug, Clone, PartialEq)]
pub enum Index {
    Imm(u32),
    Rel(Box<Operand>, u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Operand {
    pub reg: Reg,
    pub select: Select,
    pub index: Vec<Index>,
    /// Immediate values (1 or 4).
    pub imm: Vec<u32>,
    pub neg: bool,
    pub abs: bool,
}

impl Operand {
    /// The first index as an immediate (register number).
    #[must_use]
    pub fn number(&self) -> u32 {
        match self.index.first() {
            Some(Index::Imm(n)) => *n,
            _ => 0,
        }
    }

    /// The write mask, for destinations (all four when none is given).
    #[must_use]
    pub fn mask(&self) -> u8 {
        match self.select {
            Select::Mask(m) => m,
            Select::One(c) => 1 << c,
            _ => 1,
        }
    }
}

impl Instruction {
    /// The mnemonic as `D3DDisassemble` writes it (`mad_sat`, `if_nz`,
    /// `sample_l_aoffimmi(1,0,0)`).
    #[must_use]
    pub fn mnemonic(&self) -> String {
        let mut m = self.op.name().to_owned();
        if matches!(
            self.op,
            Op::If | Op::Breakc | Op::Discard | Op::Retc | Op::Continuec
        ) {
            m.push_str(if self.nonzero { "_nz" } else { "_z" });
        }
        if self.offset != [0, 0, 0] {
            let _ = write!(
                m,
                "_aoffimmi({},{},{})",
                self.offset[0], self.offset[1], self.offset[2]
            );
        }
        if self.saturate {
            m.push_str("_sat");
        }
        m
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub op: Op,
    /// Opcode-specific control bits (11–23), e.g. `resinfo`'s return type.
    pub controls: u32,
    pub saturate: bool,
    /// For `if`, `breakc`, `discard`, `retc`, `continuec`: test non-zero.
    pub nonzero: bool,
    /// `aoffimmi` texel offsets of sample instructions.
    pub offset: [i8; 3],
    pub operands: Vec<Operand>,
}

/// Resource dimensions of `dcl_resource`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dim {
    Buffer,
    Texture1D,
    Texture2D,
    Texture2DMs(u32),
    Texture3D,
    TextureCube,
    Other(u32),
}

/// Interpolation of a pixel shader input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interp {
    Constant,
    Linear,
    LinearCentroid,
    NoPerspective,
    Other(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    ConstantBuffer {
        slot: u32,
        size: u32,
        dynamic: bool,
    },
    Sampler {
        slot: u32,
        comparison: bool,
    },
    /// `ret`: the return type of x (1 unorm, 2 snorm, 3 sint, 4 uint,
    /// 5 float).
    Resource {
        slot: u32,
        dim: Dim,
        ret: u32,
    },
    Input {
        reg: u32,
        mask: u8,
        interp: Option<Interp>,
        system: Option<u32>,
    },
    Output {
        reg: u32,
        mask: u8,
        system: Option<u32>,
    },
    /// `oDepth` and friends: outputs without a register.
    OutputSpecial(Reg),
    Temps(u32),
    IndexableTemp {
        reg: u32,
        size: u32,
        components: u32,
    },
    ImmediateConstants(Vec<[u32; 4]>),
    GlobalFlags(u32),
    Other(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// 0 pixel, 1 vertex, 2 geometry, …
    pub kind: u32,
    pub major: u32,
    pub minor: u32,
    pub decls: Vec<Decl>,
    pub code: Vec<Instruction>,
}

struct Reader<'a> {
    t: &'a [u32],
    at: usize,
}

impl Reader<'_> {
    fn next(&mut self) -> Result<u32> {
        let v = *self.t.get(self.at).ok_or(Error::Truncated(self.at * 4))?;
        self.at += 1;
        Ok(v)
    }

    fn operand(&mut self) -> Result<Operand> {
        let tok = self.next()?;
        let components = tok & 3;
        let mode = (tok >> 2) & 3;
        let sel = (tok >> 4) & 0xff;
        let select = match (components, mode) {
            (2, 0) => Select::Mask((sel & 0xf) as u8),
            (2, 1) => Select::Swizzle([
                (sel & 3) as u8,
                ((sel >> 2) & 3) as u8,
                ((sel >> 4) & 3) as u8,
                ((sel >> 6) & 3) as u8,
            ]),
            (2, 2) => Select::One((sel & 3) as u8),
            _ => Select::Scalar,
        };
        let reg = match (tok >> 12) & 0xff {
            0 => Reg::Temp,
            1 => Reg::Input,
            2 => Reg::Output,
            3 => Reg::IndexableTemp,
            4 => Reg::Immediate,
            6 => Reg::Sampler,
            7 => Reg::Resource,
            8 => Reg::ConstantBuffer,
            9 => Reg::ImmediateConstantBuffer,
            11 => Reg::PrimitiveId,
            12 => Reg::OutputDepth,
            13 => Reg::Null,
            n => Reg::Other(n),
        };
        let dims = (tok >> 20) & 3;
        let mut neg = false;
        let mut abs = false;
        if tok & 0x8000_0000 != 0 {
            let ext = self.next()?;
            if ext & 0x3f == 1 {
                match (ext >> 6) & 0xff {
                    1 => neg = true,
                    2 => abs = true,
                    3 => {
                        neg = true;
                        abs = true;
                    }
                    _ => {}
                }
            }
        }
        let mut imm = Vec::new();
        let mut index = Vec::new();
        if reg == Reg::Immediate {
            let n = if components == 1 { 1 } else { 4 };
            for _ in 0..n {
                imm.push(self.next()?);
            }
        } else {
            for d in 0..dims {
                let rep = (tok >> (22 + 3 * d)) & 7;
                index.push(match rep {
                    0 => Index::Imm(self.next()?),
                    2 => Index::Rel(Box::new(self.operand()?), 0),
                    3 => {
                        let base = self.next()?;
                        Index::Rel(Box::new(self.operand()?), base)
                    }
                    _ => return Err(Error::Truncated(self.at * 4)),
                });
            }
        }
        Ok(Operand {
            reg,
            select,
            index,
            imm,
            neg,
            abs,
        })
    }
}

/// Decodes a `SHDR` (or `SHEX`) chunk.
///
/// # Errors
/// On a truncated or unknown token stream.
pub fn decode(chunk: &[u8]) -> Result<Program> {
    let tokens: Vec<u32> = chunk
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_le_bytes(*c))
        .collect();
    let mut r = Reader { t: &tokens, at: 0 };
    let version = r.next()?;
    let length = r.next()? as usize;
    let end = length.min(tokens.len());
    let mut decls = Vec::new();
    let mut code = Vec::new();
    while r.at < end {
        let start = r.at;
        let tok = r.next()?;
        let opcode = tok & 0x7ff;
        let mut len = ((tok >> 24) & 0x7f) as usize;
        if opcode == 53 {
            // dcl_customdata: the length is the next token.
            len = r.next()? as usize;
            let class = tok >> 11;
            if class == 3 {
                let words = &tokens[r.at..start + len];
                decls.push(Decl::ImmediateConstants(words.as_chunks::<4>().0.to_vec()));
            }
            r.at = start + len;
            continue;
        }
        if len == 0 {
            return Err(Error::Truncated(start * 4));
        }
        let next = start + len;
        let mut offset = [0i8; 3];
        let mut ext = tok & 0x8000_0000 != 0;
        while ext {
            let e = r.next()?;
            ext = e & 0x8000_0000 != 0;
            if e & 0x3f == 1 {
                let s4 = |v: u32| ((((v & 0xf) << 4) as u8) as i8) >> 4;
                offset = [s4(e >> 9), s4(e >> 13), s4(e >> 17)];
            }
        }
        if (88..=106).contains(&opcode) {
            let d = match opcode {
                89 => {
                    let o = r.operand()?;
                    let (slot, size) = match (o.index.first(), o.index.get(1)) {
                        (Some(Index::Imm(s)), Some(Index::Imm(n))) => (*s, *n),
                        _ => (o.number(), 0),
                    };
                    Decl::ConstantBuffer {
                        slot,
                        size,
                        dynamic: tok & 0x800 != 0,
                    }
                }
                90 => Decl::Sampler {
                    slot: r.operand()?.number(),
                    comparison: (tok >> 11) & 0xf == 1,
                },
                88 => {
                    let dim = match (tok >> 11) & 0x1f {
                        1 => Dim::Buffer,
                        2 => Dim::Texture1D,
                        3 => Dim::Texture2D,
                        4 => Dim::Texture2DMs((tok >> 16) & 0x7f),
                        5 => Dim::Texture3D,
                        6 => Dim::TextureCube,
                        n => Dim::Other(n),
                    };
                    let slot = r.operand()?.number();
                    let ret = r.next()? & 0xf;
                    Decl::Resource { slot, dim, ret }
                }
                95..=100 => {
                    let o = r.operand()?;
                    let interp = (98..=100)
                        .contains(&opcode)
                        .then_some(match (tok >> 11) & 0xf {
                            1 => Interp::Constant,
                            2 => Interp::Linear,
                            3 => Interp::LinearCentroid,
                            4 => Interp::NoPerspective,
                            n => Interp::Other(n),
                        });
                    let system = matches!(opcode, 96 | 97 | 99 | 100)
                        .then(|| r.next())
                        .transpose()?;
                    if o.reg == Reg::Input {
                        Decl::Input {
                            reg: o.number(),
                            mask: o.mask(),
                            interp,
                            system,
                        }
                    } else {
                        Decl::Other(opcode)
                    }
                }
                101..=103 => {
                    let o = r.operand()?;
                    let system = (opcode != 101).then(|| r.next()).transpose()?;
                    if o.reg == Reg::Output {
                        Decl::Output {
                            reg: o.number(),
                            mask: o.mask(),
                            system,
                        }
                    } else {
                        Decl::OutputSpecial(o.reg)
                    }
                }
                104 => Decl::Temps(r.next()?),
                105 => Decl::IndexableTemp {
                    reg: r.next()?,
                    size: r.next()?,
                    components: r.next()?,
                },
                106 => Decl::GlobalFlags(tok >> 11),
                n => Decl::Other(n),
            };
            decls.push(d);
            r.at = next;
            continue;
        }
        let Some(op) = Op::from_code(opcode) else {
            // Unknown: skip it, but keep its place visible.
            decls.push(Decl::Other(opcode));
            r.at = next;
            continue;
        };
        let mut operands = Vec::new();
        while r.at < next {
            operands.push(r.operand()?);
        }
        code.push(Instruction {
            op,
            controls: (tok >> 11) & 0x1fff,
            saturate: tok & 0x2000 != 0,
            nonzero: tok & 0x4_0000 != 0,
            offset,
            operands,
        });
        r.at = next;
    }
    Ok(Program {
        kind: version >> 16,
        major: (version >> 4) & 0xf,
        minor: version & 0xf,
        decls,
        code,
    })
}
