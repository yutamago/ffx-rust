//! The scene stream between the game and the renderer: what the game's
//! Direct3D 11 device creates (buffers, textures, views, shaders, input
//! layouts, states) and, per frame, the draws the renderer takes over
//! with everything they had bound, constant buffer contents inline.
//!
//! The same messages go through shared memory to the live renderer and
//! into capture files (`*.ffxgfx`) the renderer can replay offline. Ids are
//! the game's object addresses; a new object at a reused address replaces
//! the old one.
//!
//! Encoding: each message is a `u32` tag, a `u32` payload length, then the
//! payload, little-endian.

pub mod ring;

use std::fmt;

pub type Id = u32;

/// `D3D11_BUFFER_DESC`: width, usage, bind flags, CPU access, misc,
/// structure stride.
pub type BufferDesc = [u32; 6];
/// `D3D11_TEXTURE2D_DESC`: width, height, mips, array size, format,
/// samples, quality, usage, bind flags, CPU access, misc.
pub type TextureDesc = [u32; 11];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subresource {
    pub row_pitch: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    ShaderResource = 0,
    RenderTarget = 1,
    DepthStencil = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Blend = 0,
    Depth = 1,
    Raster = 2,
    Sampler = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub semantic: String,
    pub index: u32,
    /// DXGI format.
    pub format: u32,
    pub slot: u32,
    pub offset: u32,
    pub per_instance: bool,
    pub step: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawKind {
    Indexed = 0,
    Plain = 1,
    IndexedInstanced = 2,
    Instanced = 3,
}

/// One draw and everything it had bound.
#[derive(Debug, Clone, PartialEq)]
pub struct Draw {
    pub kind: DrawKind,
    pub count: u32,
    pub instances: u32,
    pub start: u32,
    pub base: i32,
    pub first_instance: u32,
    /// `D3D11_PRIMITIVE_TOPOLOGY`.
    pub topology: u32,
    pub layout: Id,
    pub vs: Id,
    pub ps: Id,
    /// (slot, buffer, stride, offset).
    pub vertex: Vec<(u8, Id, u32, u32)>,
    /// (buffer, DXGI format, offset).
    pub index: (Id, u32, u32),
    /// (slot, contents): the window the shader sees.
    pub vs_cb: Vec<(u8, Vec<u8>)>,
    pub ps_cb: Vec<(u8, Vec<u8>)>,
    /// (slot, shader resource view).
    pub vs_srv: Vec<(u8, Id)>,
    pub ps_srv: Vec<(u8, Id)>,
    /// (slot, sampler state).
    pub vs_samplers: Vec<(u8, Id)>,
    pub ps_samplers: Vec<(u8, Id)>,
    pub blend: Id,
    pub blend_factor: [f32; 4],
    pub sample_mask: u32,
    pub depth: Id,
    pub stencil_ref: u32,
    pub raster: Id,
    pub viewport: [f32; 6],
    pub scissor: [i32; 4],
    /// (slot, render target view).
    pub rtv: Vec<(u8, Id)>,
    pub dsv: Id,
}

/// What a frame does, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    Draw(Box<Draw>),
    ClearTarget {
        view: Id,
        color: [f32; 4],
    },
    ClearDepth {
        view: Id,
        flags: u32,
        depth: f32,
        stencil: u8,
    },
    /// New contents for part of a buffer (dynamic vertex data).
    UpdateBuffer {
        id: Id,
        offset: u32,
        data: Vec<u8>,
    },
    /// The game copied one texture into another (a copy of the scene
    /// colour for water and distortion): the renderer copies its own.
    CopyTexture {
        dst: Id,
        src: Id,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub number: u32,
    pub width: u32,
    pub height: u32,
    pub cmds: Vec<Cmd>,
}

/// Input for the renderer's overlay, from the game window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    /// Show or hide the overlay (its hotkey).
    Toggle,
    /// The pointer, in window pixels.
    Move(f32, f32),
    /// 0 left, 1 right, 2 middle.
    Button { button: u8, pressed: bool },
    /// Wheel notches (positive: away from the user).
    Wheel(f32),
    /// A Windows virtual-key code.
    Key { vk: u32, pressed: bool },
    /// A typed character.
    Char(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Buffer {
        id: Id,
        desc: BufferDesc,
        data: Vec<u8>,
    },
    Texture {
        id: Id,
        desc: TextureDesc,
        subresources: Vec<Subresource>,
    },
    View {
        id: Id,
        kind: ViewKind,
        resource: Id,
        desc: Option<[u32; 6]>,
    },
    /// `kind`: `v`ertex or `p`ixel (others are not sent).
    Shader {
        id: Id,
        kind: u8,
        bytecode: Vec<u8>,
    },
    Layout {
        id: Id,
        elements: Vec<Element>,
    },
    State {
        id: Id,
        kind: StateKind,
        desc: Vec<u8>,
    },
    UpdateBuffer {
        id: Id,
        offset: u32,
        data: Vec<u8>,
    },
    UpdateTexture {
        id: Id,
        subresource: u32,
        row_pitch: u32,
        data: Vec<u8>,
    },
    Release {
        id: Id,
    },
    Frame(Frame),
    /// What happened in the game window since the last frame; `width` and
    /// `height` are its back buffer's.
    Input {
        width: u32,
        height: u32,
        events: Vec<Input>,
    },
    /// What the game is showing, sent before each frame: the scene (the
    /// event file of the map, e.g. `cdsp0000`; empty before the first)
    /// and the camera's eye and look-at point in world space, when the
    /// game has a camera.
    World {
        scene: String,
        camera: Option<([f32; 3], [f32; 3])>,
    },
    /// The glowing effects the game drew over the scene after the frame
    /// went to the renderer (additive draws into the scene colour against
    /// its depth), sent at Present: the renderer finds lights in them.
    Effects(Frame),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(pub &'static str);

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bad scene stream: {}", self.0)
    }
}

impl std::error::Error for DecodeError {}

/// Appends values to a byte buffer.
#[derive(Default)]
pub struct Writer {
    pub bytes: Vec<u8>,
}

impl Writer {
    pub fn u8(&mut self, v: u8) {
        self.bytes.push(v);
    }
    pub fn u32(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i32(&mut self, v: i32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    pub fn f32(&mut self, v: f32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
    pub fn blob(&mut self, v: &[u8]) {
        self.u32(u32::try_from(v.len()).expect("blob under 4 GiB"));
        self.bytes.extend_from_slice(v);
    }
    pub fn str(&mut self, v: &str) {
        self.blob(v.as_bytes());
    }
    pub fn u32s(&mut self, v: &[u32]) {
        for &x in v {
            self.u32(x);
        }
    }
    fn len_of<T>(&mut self, v: &[T]) {
        self.u32(u32::try_from(v.len()).expect("list under 4 G"));
    }
}

/// Reads values from a byte slice; every read fails on truncated input.
#[allow(clippy::missing_errors_doc)]
pub struct Reader<'a> {
    pub bytes: &'a [u8],
    pub at: usize,
}

#[allow(clippy::missing_errors_doc)]
impl<'a> Reader<'a> {
    #[must_use]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let s = self
            .bytes
            .get(self.at..self.at + n)
            .ok_or(DecodeError("truncated"))?;
        self.at += n;
        Ok(s)
    }
    pub fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    pub fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }
    pub fn i32(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }
    pub fn f32(&mut self) -> Result<f32, DecodeError> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }
    pub fn blob(&mut self) -> Result<Vec<u8>, DecodeError> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }
    pub fn str(&mut self) -> Result<String, DecodeError> {
        String::from_utf8(self.blob()?).map_err(|_| DecodeError("utf-8"))
    }
    pub fn u32s<const N: usize>(&mut self) -> Result<[u32; N], DecodeError> {
        let mut out = [0; N];
        for v in &mut out {
            *v = self.u32()?;
        }
        Ok(out)
    }
    #[must_use]
    pub fn done(&self) -> bool {
        self.at >= self.bytes.len()
    }
}

const BUFFER: u32 = 1;
const TEXTURE: u32 = 2;
const VIEW: u32 = 3;
const SHADER: u32 = 4;
const LAYOUT: u32 = 5;
const STATE: u32 = 6;
const UPDATE_BUFFER: u32 = 7;
const UPDATE_TEXTURE: u32 = 8;
const RELEASE: u32 = 9;
const FRAME: u32 = 10;
const INPUT: u32 = 11;
const WORLD: u32 = 12;
const EFFECTS: u32 = 13;

fn slots_u8(w: &mut Writer, v: &[(u8, Id)]) {
    w.len_of(v);
    for &(s, id) in v {
        w.u8(s);
        w.u32(id);
    }
}

fn read_slots(r: &mut Reader) -> Result<Vec<(u8, Id)>, DecodeError> {
    let n = r.u32()?;
    (0..n).map(|_| Ok((r.u8()?, r.u32()?))).collect()
}

fn cbs(w: &mut Writer, v: &[(u8, Vec<u8>)]) {
    w.len_of(v);
    for (s, data) in v {
        w.u8(*s);
        w.blob(data);
    }
}

fn read_cbs(r: &mut Reader) -> Result<Vec<(u8, Vec<u8>)>, DecodeError> {
    let n = r.u32()?;
    (0..n).map(|_| Ok((r.u8()?, r.blob()?))).collect()
}

impl Draw {
    fn write(&self, w: &mut Writer) {
        w.u8(self.kind as u8);
        w.u32s(&[self.count, self.instances, self.start]);
        w.i32(self.base);
        w.u32s(&[
            self.first_instance,
            self.topology,
            self.layout,
            self.vs,
            self.ps,
        ]);
        w.len_of(&self.vertex);
        for &(s, id, stride, offset) in &self.vertex {
            w.u8(s);
            w.u32s(&[id, stride, offset]);
        }
        w.u32s(&[self.index.0, self.index.1, self.index.2]);
        cbs(w, &self.vs_cb);
        cbs(w, &self.ps_cb);
        slots_u8(w, &self.vs_srv);
        slots_u8(w, &self.ps_srv);
        slots_u8(w, &self.vs_samplers);
        slots_u8(w, &self.ps_samplers);
        w.u32(self.blend);
        for f in self.blend_factor {
            w.f32(f);
        }
        w.u32s(&[self.sample_mask, self.depth, self.stencil_ref, self.raster]);
        for f in self.viewport {
            w.f32(f);
        }
        for s in self.scissor {
            w.i32(s);
        }
        slots_u8(w, &self.rtv);
        w.u32(self.dsv);
    }

    fn read(r: &mut Reader) -> Result<Self, DecodeError> {
        let kind = match r.u8()? {
            0 => DrawKind::Indexed,
            1 => DrawKind::Plain,
            2 => DrawKind::IndexedInstanced,
            3 => DrawKind::Instanced,
            _ => return Err(DecodeError("draw kind")),
        };
        let [count, instances, start] = r.u32s()?;
        let base = r.i32()?;
        let [first_instance, topology, layout, vs, ps] = r.u32s()?;
        let n = r.u32()?;
        let vertex = (0..n)
            .map(|_| {
                let s = r.u8()?;
                let [id, stride, offset] = r.u32s()?;
                Ok((s, id, stride, offset))
            })
            .collect::<Result<_, DecodeError>>()?;
        let [ib, ifmt, ioff] = r.u32s()?;
        let vs_cb = read_cbs(r)?;
        let ps_cb = read_cbs(r)?;
        let vs_srv = read_slots(r)?;
        let ps_srv = read_slots(r)?;
        let vs_samplers = read_slots(r)?;
        let ps_samplers = read_slots(r)?;
        let blend = r.u32()?;
        let blend_factor = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
        let [sample_mask, depth, stencil_ref, raster] = r.u32s()?;
        let mut viewport = [0.0; 6];
        for v in &mut viewport {
            *v = r.f32()?;
        }
        let scissor = [r.i32()?, r.i32()?, r.i32()?, r.i32()?];
        let rtv = read_slots(r)?;
        let dsv = r.u32()?;
        Ok(Self {
            kind,
            count,
            instances,
            start,
            base,
            first_instance,
            topology,
            layout,
            vs,
            ps,
            vertex,
            index: (ib, ifmt, ioff),
            vs_cb,
            ps_cb,
            vs_srv,
            ps_srv,
            vs_samplers,
            ps_samplers,
            blend,
            blend_factor,
            sample_mask,
            depth,
            stencil_ref,
            raster,
            viewport,
            scissor,
            rtv,
            dsv,
        })
    }
}

#[allow(clippy::too_many_lines)] // one arm per message
impl Msg {
    /// Appends the encoded message.
    pub fn write(&self, out: &mut Vec<u8>) {
        let mut w = Writer::default();
        let tag = match self {
            Self::Buffer { id, desc, data } => {
                w.u32(*id);
                w.u32s(desc);
                w.blob(data);
                BUFFER
            }
            Self::Texture {
                id,
                desc,
                subresources,
            } => {
                w.u32(*id);
                w.u32s(desc);
                w.len_of(subresources);
                for s in subresources {
                    w.u32(s.row_pitch);
                    w.blob(&s.data);
                }
                TEXTURE
            }
            Self::View {
                id,
                kind,
                resource,
                desc,
            } => {
                w.u32(*id);
                w.u8(*kind as u8);
                w.u32(*resource);
                match desc {
                    Some(d) => {
                        w.u8(1);
                        w.u32s(d);
                    }
                    None => w.u8(0),
                }
                VIEW
            }
            Self::Shader { id, kind, bytecode } => {
                w.u32(*id);
                w.u8(*kind);
                w.blob(bytecode);
                SHADER
            }
            Self::Layout { id, elements } => {
                w.u32(*id);
                w.len_of(elements);
                for e in elements {
                    w.str(&e.semantic);
                    w.u32s(&[
                        e.index,
                        e.format,
                        e.slot,
                        e.offset,
                        u32::from(e.per_instance),
                        e.step,
                    ]);
                }
                LAYOUT
            }
            Self::State { id, kind, desc } => {
                w.u32(*id);
                w.u8(*kind as u8);
                w.blob(desc);
                STATE
            }
            Self::UpdateBuffer { id, offset, data } => {
                w.u32s(&[*id, *offset]);
                w.blob(data);
                UPDATE_BUFFER
            }
            Self::UpdateTexture {
                id,
                subresource,
                row_pitch,
                data,
            } => {
                w.u32s(&[*id, *subresource, *row_pitch]);
                w.blob(data);
                UPDATE_TEXTURE
            }
            Self::Release { id } => {
                w.u32(*id);
                RELEASE
            }
            Self::Input {
                width,
                height,
                events,
            } => {
                w.u32s(&[*width, *height]);
                w.len_of(events);
                for e in events {
                    match *e {
                        Input::Toggle => w.u8(0),
                        Input::Move(x, y) => {
                            w.u8(1);
                            w.f32(x);
                            w.f32(y);
                        }
                        Input::Button { button, pressed } => {
                            w.u8(2);
                            w.u8(button);
                            w.u8(u8::from(pressed));
                        }
                        Input::Wheel(d) => {
                            w.u8(3);
                            w.f32(d);
                        }
                        Input::Key { vk, pressed } => {
                            w.u8(4);
                            w.u32(vk);
                            w.u8(u8::from(pressed));
                        }
                        Input::Char(c) => {
                            w.u8(5);
                            w.u32(c);
                        }
                    }
                }
                INPUT
            }
            Self::World { scene, camera } => {
                w.str(scene);
                match camera {
                    Some((eye, target)) => {
                        w.u8(1);
                        for v in eye.iter().chain(target) {
                            w.f32(*v);
                        }
                    }
                    None => w.u8(0),
                }
                WORLD
            }
            Self::Frame(f) => {
                write_frame(&mut w, f);
                FRAME
            }
            Self::Effects(f) => {
                write_frame(&mut w, f);
                EFFECTS
            }
        };
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(
            &u32::try_from(w.bytes.len())
                .expect("message under 4 GiB")
                .to_le_bytes(),
        );
        out.extend_from_slice(&w.bytes);
    }

    /// Decodes one message's payload.
    ///
    /// # Errors
    /// When the payload does not match its tag.
    pub fn read(tag: u32, payload: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(payload);
        Ok(match tag {
            BUFFER => Self::Buffer {
                id: r.u32()?,
                desc: r.u32s()?,
                data: r.blob()?,
            },
            TEXTURE => {
                let id = r.u32()?;
                let desc = r.u32s()?;
                let n = r.u32()?;
                let subresources = (0..n)
                    .map(|_| {
                        Ok(Subresource {
                            row_pitch: r.u32()?,
                            data: r.blob()?,
                        })
                    })
                    .collect::<Result<_, DecodeError>>()?;
                Self::Texture {
                    id,
                    desc,
                    subresources,
                }
            }
            VIEW => {
                let id = r.u32()?;
                let kind = match r.u8()? {
                    0 => ViewKind::ShaderResource,
                    1 => ViewKind::RenderTarget,
                    2 => ViewKind::DepthStencil,
                    _ => return Err(DecodeError("view kind")),
                };
                let resource = r.u32()?;
                let desc = if r.u8()? == 1 { Some(r.u32s()?) } else { None };
                Self::View {
                    id,
                    kind,
                    resource,
                    desc,
                }
            }
            SHADER => Self::Shader {
                id: r.u32()?,
                kind: r.u8()?,
                bytecode: r.blob()?,
            },
            LAYOUT => {
                let id = r.u32()?;
                let n = r.u32()?;
                let elements = (0..n)
                    .map(|_| {
                        let semantic = r.str()?;
                        let [index, format, slot, offset, inst, step] = r.u32s()?;
                        Ok(Element {
                            semantic,
                            index,
                            format,
                            slot,
                            offset,
                            per_instance: inst != 0,
                            step,
                        })
                    })
                    .collect::<Result<_, DecodeError>>()?;
                Self::Layout { id, elements }
            }
            STATE => {
                let id = r.u32()?;
                let kind = match r.u8()? {
                    0 => StateKind::Blend,
                    1 => StateKind::Depth,
                    2 => StateKind::Raster,
                    3 => StateKind::Sampler,
                    _ => return Err(DecodeError("state kind")),
                };
                Self::State {
                    id,
                    kind,
                    desc: r.blob()?,
                }
            }
            UPDATE_BUFFER => Self::UpdateBuffer {
                id: r.u32()?,
                offset: r.u32()?,
                data: r.blob()?,
            },
            UPDATE_TEXTURE => Self::UpdateTexture {
                id: r.u32()?,
                subresource: r.u32()?,
                row_pitch: r.u32()?,
                data: r.blob()?,
            },
            RELEASE => Self::Release { id: r.u32()? },
            INPUT => {
                let [width, height] = r.u32s()?;
                let n = r.u32()?;
                let mut events = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    events.push(match r.u8()? {
                        0 => Input::Toggle,
                        1 => Input::Move(r.f32()?, r.f32()?),
                        2 => Input::Button {
                            button: r.u8()?,
                            pressed: r.u8()? != 0,
                        },
                        3 => Input::Wheel(r.f32()?),
                        4 => Input::Key {
                            vk: r.u32()?,
                            pressed: r.u8()? != 0,
                        },
                        5 => Input::Char(r.u32()?),
                        _ => return Err(DecodeError("input")),
                    });
                }
                Self::Input {
                    width,
                    height,
                    events,
                }
            }
            WORLD => {
                let scene = r.str()?;
                let camera = if r.u8()? == 0 {
                    None
                } else {
                    let mut v = [0.0f32; 6];
                    for x in &mut v {
                        *x = r.f32()?;
                    }
                    Some(([v[0], v[1], v[2]], [v[3], v[4], v[5]]))
                };
                Self::World { scene, camera }
            }
            FRAME => Self::Frame(read_frame(&mut r)?),
            EFFECTS => Self::Effects(read_frame(&mut r)?),
            _ => return Err(DecodeError("tag")),
        })
    }
}

fn write_frame(w: &mut Writer, f: &Frame) {
    w.u32s(&[f.number, f.width, f.height]);
    w.len_of(&f.cmds);
    for c in &f.cmds {
        match c {
            Cmd::Draw(d) => {
                w.u8(0);
                d.write(w);
            }
            Cmd::ClearTarget { view, color } => {
                w.u8(1);
                w.u32(*view);
                for v in color {
                    w.f32(*v);
                }
            }
            Cmd::ClearDepth {
                view,
                flags,
                depth,
                stencil,
            } => {
                w.u8(2);
                w.u32s(&[*view, *flags]);
                w.f32(*depth);
                w.u8(*stencil);
            }
            Cmd::UpdateBuffer { id, offset, data } => {
                w.u8(3);
                w.u32s(&[*id, *offset]);
                w.blob(data);
            }
            Cmd::CopyTexture { dst, src } => {
                w.u8(4);
                w.u32s(&[*dst, *src]);
            }
        }
    }
}

fn read_frame(r: &mut Reader) -> Result<Frame, DecodeError> {
    let [number, width, height] = r.u32s()?;
    let n = r.u32()?;
    let mut cmds = Vec::with_capacity(n as usize);
    for _ in 0..n {
        cmds.push(match r.u8()? {
            0 => Cmd::Draw(Box::new(Draw::read(r)?)),
            1 => Cmd::ClearTarget {
                view: r.u32()?,
                color: [r.f32()?, r.f32()?, r.f32()?, r.f32()?],
            },
            2 => Cmd::ClearDepth {
                view: r.u32()?,
                flags: r.u32()?,
                depth: r.f32()?,
                stencil: r.u8()?,
            },
            3 => Cmd::UpdateBuffer {
                id: r.u32()?,
                offset: r.u32()?,
                data: r.blob()?,
            },
            4 => Cmd::CopyTexture {
                dst: r.u32()?,
                src: r.u32()?,
            },
            _ => return Err(DecodeError("command")),
        });
    }
    Ok(Frame {
        number,
        width,
        height,
        cmds,
    })
}

/// Splits a stream into messages.
///
/// # Errors
/// On a truncated or malformed stream.
pub fn read_all(bytes: &[u8]) -> Result<Vec<Msg>, DecodeError> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 8 <= bytes.len() {
        let tag = u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4"));
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().expect("4")) as usize;
        let payload = bytes
            .get(at + 8..at + 8 + len)
            .ok_or(DecodeError("truncated message"))?;
        out.push(Msg::read(tag, payload)?);
        at += 8 + len;
    }
    Ok(out)
}

/// Magic at the start of a capture file.
pub const CAPTURE_MAGIC: &[u8; 8] = b"FFXGFX01";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::too_many_lines)] // one of each message
    fn messages_round_trip() {
        let draw = Draw {
            kind: DrawKind::Indexed,
            count: 36,
            instances: 1,
            start: 0,
            base: -2,
            first_instance: 0,
            topology: 4,
            layout: 7,
            vs: 8,
            ps: 9,
            vertex: vec![(0, 10, 12, 0), (1, 11, 8, 4)],
            index: (12, 57, 0),
            vs_cb: vec![(0, vec![1, 2, 3, 4])],
            ps_cb: vec![],
            vs_srv: vec![],
            ps_srv: vec![(0, 13)],
            vs_samplers: vec![],
            ps_samplers: vec![(0, 14)],
            blend: 15,
            blend_factor: [0.0, 0.5, 1.0, 1.0],
            sample_mask: !0,
            depth: 16,
            stencil_ref: 0,
            raster: 17,
            viewport: [0.0, 0.0, 1280.0, 720.0, 0.0, 1.0],
            scissor: [0, 0, 1280, 720],
            rtv: vec![(0, 18), (1, 19)],
            dsv: 20,
        };
        let msgs = vec![
            Msg::Buffer {
                id: 1,
                desc: [16, 1, 1, 0, 0, 0],
                data: vec![9; 16],
            },
            Msg::Layout {
                id: 7,
                elements: vec![Element {
                    semantic: "POSITION".into(),
                    index: 0,
                    format: 6,
                    slot: 0,
                    offset: 0,
                    per_instance: false,
                    step: 0,
                }],
            },
            Msg::Input {
                width: 1280,
                height: 720,
                events: vec![
                    Input::Toggle,
                    Input::Move(3.0, 4.5),
                    Input::Button {
                        button: 0,
                        pressed: true,
                    },
                    Input::Wheel(-1.0),
                    Input::Key {
                        vk: 0x79,
                        pressed: false,
                    },
                    Input::Char(0x41),
                ],
            },
            Msg::World {
                scene: "cdsp0000".into(),
                camera: Some(([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])),
            },
            Msg::World {
                scene: String::new(),
                camera: None,
            },
            Msg::Effects(Frame {
                number: 4,
                width: 1280,
                height: 720,
                cmds: vec![Cmd::UpdateBuffer {
                    id: 3,
                    offset: 8,
                    data: vec![1, 2, 3],
                }],
            }),
            Msg::Frame(Frame {
                number: 3,
                width: 1280,
                height: 720,
                cmds: vec![
                    Cmd::ClearTarget {
                        view: 18,
                        color: [0.0, 0.0, 0.0, 1.0],
                    },
                    Cmd::Draw(Box::new(draw)),
                ],
            }),
        ];
        let mut bytes = Vec::new();
        for m in &msgs {
            m.write(&mut bytes);
        }
        assert_eq!(read_all(&bytes).unwrap(), msgs);
    }
}
