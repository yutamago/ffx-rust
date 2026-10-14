//! Direct3D shader bytecode (DXBC) containers: the chunk list, and the
//! reflection the HLSL compiler leaves in them: constant buffers with their
//! variables (`RDEF`), resource bindings, and the input and output
//! signatures (`ISGN`, `OSGN`).
//!
//! The game's shaders (`shaders/d3d11/*.fx.phyre`) keep their reflection,
//! so the renderer can find a draw's matrices and material values in its
//! constant buffers by name.

// Bytecode decoding narrows token fields on purpose; the translator names
// operands a, b, c as the instruction set does.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

pub mod sm4;
pub mod wgsl;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("not a DXBC container")]
    Magic,
    #[error("truncated at {0:#x}")]
    Truncated(usize),
}

pub type Result<T> = std::result::Result<T, Error>;

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
        .ok_or(Error::Truncated(at))
}

fn u16_at(b: &[u8], at: usize) -> Result<u16> {
    b.get(at..at + 2)
        .map(|s| u16::from_le_bytes(s.try_into().expect("2 bytes")))
        .ok_or(Error::Truncated(at))
}

fn cstr(b: &[u8], at: usize) -> Result<String> {
    let rest = b.get(at..).ok_or(Error::Truncated(at))?;
    let end = rest
        .iter()
        .position(|&c| c == 0)
        .ok_or(Error::Truncated(at))?;
    Ok(String::from_utf8_lossy(&rest[..end]).into_owned())
}

/// A DXBC container: its chunks by tag.
#[derive(Debug, Clone)]
pub struct Container<'a> {
    pub bytes: &'a [u8],
    pub chunks: Vec<([u8; 4], &'a [u8])>,
}

impl<'a> Container<'a> {
    /// # Errors
    /// When `bytes` is not a whole DXBC container.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        if bytes.get(..4) != Some(b"DXBC") {
            return Err(Error::Magic);
        }
        let count = u32_at(bytes, 28)? as usize;
        let mut chunks = Vec::with_capacity(count);
        for i in 0..count {
            let off = u32_at(bytes, 32 + 4 * i)? as usize;
            let tag: [u8; 4] = bytes
                .get(off..off + 4)
                .ok_or(Error::Truncated(off))?
                .try_into()
                .expect("4 bytes");
            let len = u32_at(bytes, off + 4)? as usize;
            let data = bytes
                .get(off + 8..off + 8 + len)
                .ok_or(Error::Truncated(off))?;
            chunks.push((tag, data));
        }
        Ok(Self { bytes, chunks })
    }

    #[must_use]
    pub fn chunk(&self, tag: &[u8; 4]) -> Option<&'a [u8]> {
        self.chunks.iter().find(|(t, _)| t == tag).map(|(_, d)| *d)
    }

    /// The program type and shader model from the code chunk: (`vs`, `ps`,
    /// …, major, minor).
    #[must_use]
    pub fn program(&self) -> Option<(&'static str, u8, u8)> {
        let code = self.chunk(b"SHDR").or_else(|| self.chunk(b"SHEX"))?;
        let ver = u32_at(code, 0).ok()?;
        let kind = match ver >> 16 {
            0 => "ps",
            1 => "vs",
            2 => "gs",
            3 => "hs",
            4 => "ds",
            5 => "cs",
            _ => "?",
        };
        Some((kind, ((ver >> 4) & 0xf) as u8, (ver & 0xf) as u8))
    }

    /// The decoded program.
    ///
    /// # Errors
    /// When there is no code chunk or it does not decode.
    pub fn program_code(&self) -> Result<sm4::Program> {
        let code = self
            .chunk(b"SHDR")
            .or_else(|| self.chunk(b"SHEX"))
            .ok_or(Error::Magic)?;
        sm4::decode(code)
    }

    /// The reflection, when the compiler left it.
    ///
    /// # Errors
    /// When the `RDEF` chunk is malformed.
    pub fn reflection(&self) -> Result<Option<Reflection>> {
        self.chunk(b"RDEF").map(Reflection::parse).transpose()
    }

    /// # Errors
    /// When the chunk is malformed.
    pub fn inputs(&self) -> Result<Vec<Signature>> {
        self.chunk(b"ISGN").map_or(Ok(Vec::new()), Signature::parse)
    }

    /// # Errors
    /// When the chunk is malformed.
    pub fn outputs(&self) -> Result<Vec<Signature>> {
        self.chunk(b"OSGN").map_or(Ok(Vec::new()), Signature::parse)
    }
}

/// What a binding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindKind {
    ConstantBuffer,
    TextureBuffer,
    Texture,
    Sampler,
    Other(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub name: String,
    pub kind: BindKind,
    /// `D3D_SRV_DIMENSION` for textures.
    pub dimension: u32,
    pub slot: u32,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    /// Byte offset in the buffer.
    pub offset: u32,
    pub size: u32,
    /// `D3D_SVC_*`: 0 scalar, 1 vector, 2 row-major matrix, 3 column-major
    /// matrix, 5 struct.
    pub class: u16,
    /// `D3D_SVT_*`: 1 bool, 2 int, 3 float, 19 uint.
    pub base: u16,
    pub rows: u16,
    pub columns: u16,
    /// Array elements (0: not an array).
    pub elements: u16,
    /// Whether the shader reads it (`D3D_SVF_USED`).
    pub used: bool,
    /// Struct members, relative to the variable.
    pub members: Vec<Member>,
}

/// A member of a struct variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    pub offset: u32,
    pub class: u16,
    pub base: u16,
    pub rows: u16,
    pub columns: u16,
    pub elements: u16,
    pub members: Vec<Member>,
}

/// The members of the type at `ty` (depth-limited).
fn members(rdef: &[u8], ty: usize, depth: u32) -> Result<Vec<Member>> {
    let count = u16_at(rdef, ty + 10)? as usize;
    let at = u32_at(rdef, ty + 12)? as usize;
    if depth > 4 || count == 0 || at == 0 {
        return Ok(Vec::new());
    }
    (0..count)
        .map(|i| {
            let m = at + 12 * i;
            let t = u32_at(rdef, m + 4)? as usize;
            Ok(Member {
                name: cstr(rdef, u32_at(rdef, m)? as usize)?,
                offset: u32_at(rdef, m + 8)?,
                class: u16_at(rdef, t)?,
                base: u16_at(rdef, t + 2)?,
                rows: u16_at(rdef, t + 4)?,
                columns: u16_at(rdef, t + 6)?,
                elements: u16_at(rdef, t + 8)?,
                members: members(rdef, t, depth + 1)?,
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantBuffer {
    pub name: String,
    pub size: u32,
    pub variables: Vec<Variable>,
}

impl ConstantBuffer {
    #[must_use]
    pub fn variable(&self, name: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reflection {
    pub buffers: Vec<ConstantBuffer>,
    pub bindings: Vec<Binding>,
    pub creator: String,
}

impl Reflection {
    /// # Errors
    /// When the chunk is malformed.
    pub fn parse(rdef: &[u8]) -> Result<Self> {
        let cb_count = u32_at(rdef, 0)? as usize;
        let cb_off = u32_at(rdef, 4)? as usize;
        let bind_count = u32_at(rdef, 8)? as usize;
        let bind_off = u32_at(rdef, 12)? as usize;
        let major = rdef.get(17).copied().ok_or(Error::Truncated(17))?;
        let creator = cstr(rdef, u32_at(rdef, 24)? as usize)?;
        // Shader model 5 variables carry texture and sampler ranges too.
        let var_size = if major >= 5 { 40 } else { 24 };
        let mut bindings = Vec::with_capacity(bind_count);
        for i in 0..bind_count {
            let at = bind_off + 32 * i;
            let kind = match u32_at(rdef, at + 4)? {
                0 => BindKind::ConstantBuffer,
                1 => BindKind::TextureBuffer,
                2 => BindKind::Texture,
                3 => BindKind::Sampler,
                k => BindKind::Other(k),
            };
            bindings.push(Binding {
                name: cstr(rdef, u32_at(rdef, at)? as usize)?,
                kind,
                dimension: u32_at(rdef, at + 12)?,
                slot: u32_at(rdef, at + 20)?,
                count: u32_at(rdef, at + 24)?,
            });
        }
        let mut buffers = Vec::with_capacity(cb_count);
        for i in 0..cb_count {
            let at = cb_off + 24 * i;
            let vars = u32_at(rdef, at + 4)? as usize;
            let var_off = u32_at(rdef, at + 8)? as usize;
            let mut variables = Vec::with_capacity(vars);
            for k in 0..vars {
                let v = var_off + var_size * k;
                let ty = u32_at(rdef, v + 16)? as usize;
                variables.push(Variable {
                    name: cstr(rdef, u32_at(rdef, v)? as usize)?,
                    offset: u32_at(rdef, v + 4)?,
                    size: u32_at(rdef, v + 8)?,
                    used: u32_at(rdef, v + 12)? & 2 != 0,
                    class: u16_at(rdef, ty)?,
                    base: u16_at(rdef, ty + 2)?,
                    rows: u16_at(rdef, ty + 4)?,
                    columns: u16_at(rdef, ty + 6)?,
                    elements: u16_at(rdef, ty + 8)?,
                    members: members(rdef, ty, 0)?,
                });
            }
            buffers.push(ConstantBuffer {
                name: cstr(rdef, u32_at(rdef, at)? as usize)?,
                size: u32_at(rdef, at + 12)?,
                variables,
            });
        }
        Ok(Self {
            buffers,
            bindings,
            creator,
        })
    }

    /// The constant buffer bound at register `slot` (`b<slot>`).
    #[must_use]
    pub fn buffer_at(&self, slot: u32) -> Option<&ConstantBuffer> {
        let name = &self
            .bindings
            .iter()
            .find(|b| b.kind == BindKind::ConstantBuffer && b.slot == slot)?
            .name;
        self.buffers.iter().find(|b| &b.name == name)
    }

    /// The register of the binding called `name`.
    #[must_use]
    pub fn slot_of(&self, name: &str, kind: BindKind) -> Option<u32> {
        self.bindings
            .iter()
            .find(|b| b.kind == kind && b.name == name)
            .map(|b| b.slot)
    }
}

/// One element of an input or output signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub semantic: String,
    pub index: u32,
    /// `D3D_NAME`: 0 for user values, 1 position, 64 target, …
    pub system: u32,
    /// 1 uint, 2 int, 3 float.
    pub component: u32,
    pub register: u32,
    pub mask: u8,
    pub used: u8,
}

impl Signature {
    /// # Errors
    /// When the chunk is malformed.
    pub fn parse(chunk: &[u8]) -> Result<Vec<Self>> {
        let count = u32_at(chunk, 0)? as usize;
        (0..count)
            .map(|i| {
                let at = 8 + 24 * i;
                Ok(Self {
                    semantic: cstr(chunk, u32_at(chunk, at)? as usize)?,
                    index: u32_at(chunk, at + 4)?,
                    system: u32_at(chunk, at + 8)?,
                    component: u32_at(chunk, at + 12)?,
                    register: u32_at(chunk, at + 16)?,
                    mask: *chunk.get(at + 20).ok_or(Error::Truncated(at))?,
                    used: *chunk.get(at + 21).ok_or(Error::Truncated(at))?,
                })
            })
            .collect()
    }
}

/// The DXBC containers inside a file (a shader cluster holds many), with
/// their offsets.
#[must_use]
pub fn find_all(bytes: &[u8]) -> Vec<(usize, &[u8])> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(at) = bytes[i..].windows(4).position(|w| w == b"DXBC") {
        let start = i + at;
        if let Ok(size) = u32_at(bytes, start + 24)
            && let Some(blob) = bytes.get(start..start + size as usize)
            && size > 32
            && Container::parse(blob).is_ok()
        {
            out.push((start, blob));
            i = start + size as usize;
        } else {
            i = start + 4;
        }
    }
    out
}

/// FNV-1a 64 of a blob: how the graphics layer names shaders.
#[must_use]
pub fn hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}
