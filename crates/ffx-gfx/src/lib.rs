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
pub mod scene_inputs;

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
    /// The pointer moved by (x, y) window pixels while the renderer holds
    /// it (reply flag 1024: it stays at the window's centre, hidden).
    Motion(f32, f32),
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
    /// and the camera's eye and look-at point in the renderer's world space
    /// (the view matrix's), when the game has a camera.
    World {
        scene: String,
        camera: Option<([f32; 3], [f32; 3])>,
    },
    /// The glowing effects the game drew over the scene after the frame
    /// went to the renderer (additive draws into the scene colour against
    /// its depth), sent at Present: the renderer finds lights in them.
    Effects(Frame),
    /// The game's field minimap as its update left it this frame (the
    /// hook draws none of it when the renderer draws its own).
    Minimap(Minimap),
    /// What the scene builder needs at one `renderWorld` call (reference
    /// captures; [`scene_inputs`]).
    SceneInputs(Box<scene_inputs::SceneInputs>),
}

/// The field minimap's state, in the game's space (ground x/z, y down).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Minimap {
    /// Its fade, 0 (hidden) to 1.
    pub shown: f32,
    pub player: [f32; 3],
    /// Radians, from +x toward +z.
    pub yaw: f32,
    pub markers: Vec<MapMarker>,
    /// The map's mesh, when it changed since the last message.
    pub mesh: Option<MapMesh>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapMarker {
    /// The game's marker flags (`AtelGetGuideMapActorInfo`): colour, shape,
    /// player, edge pointer, line.
    pub flags: u32,
    /// Its colour (the game's palette for the flags' colour index), 0–1.
    pub color: [f32; 4],
    pub pos: [f32; 3],
    /// A line marker's ends (x, z, x, z): exits and triggers.
    pub line: Option<[f32; 4]>,
}

/// The guide map's mesh: vertices (i16 x, y, z), triangles, and the
/// matrix that takes vertices to the world (as the game stores it, 16
/// floats).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapMesh {
    pub id: u32,
    pub matrix: [f32; 16],
    pub vertices: Vec<[i16; 3]>,
    pub triangles: Vec<[u16; 3]>,
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
const MINIMAP: u32 = 14;
const SCENE_INPUTS: u32 = 15;

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
                        Input::Motion(x, y) => {
                            w.u8(6);
                            w.f32(x);
                            w.f32(y);
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
            Self::Minimap(m) => {
                w.f32(m.shown);
                for v in m.player {
                    w.f32(v);
                }
                w.f32(m.yaw);
                w.len_of(&m.markers);
                for k in &m.markers {
                    w.u32(k.flags);
                    for v in k.color.iter().chain(&k.pos) {
                        w.f32(*v);
                    }
                    match k.line {
                        Some(l) => {
                            w.u8(1);
                            for v in l {
                                w.f32(v);
                            }
                        }
                        None => w.u8(0),
                    }
                }
                match &m.mesh {
                    Some(mesh) => {
                        w.u8(1);
                        w.u32(mesh.id);
                        for v in mesh.matrix {
                            w.f32(v);
                        }
                        let vb: Vec<u8> = mesh
                            .vertices
                            .iter()
                            .flatten()
                            .flat_map(|v| v.to_le_bytes())
                            .collect();
                        w.blob(&vb);
                        let tb: Vec<u8> = mesh
                            .triangles
                            .iter()
                            .flatten()
                            .flat_map(|v| v.to_le_bytes())
                            .collect();
                        w.blob(&tb);
                    }
                    None => w.u8(0),
                }
                MINIMAP
            }
            Self::SceneInputs(s) => {
                s.write(&mut w);
                SCENE_INPUTS
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
                        6 => Input::Motion(r.f32()?, r.f32()?),
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
            MINIMAP => {
                let shown = r.f32()?;
                let player = [r.f32()?, r.f32()?, r.f32()?];
                let yaw = r.f32()?;
                let n = r.u32()? as usize;
                let mut markers = Vec::with_capacity(n.min(4096));
                for _ in 0..n {
                    let flags = r.u32()?;
                    let color = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
                    let pos = [r.f32()?, r.f32()?, r.f32()?];
                    let line = if r.u8()? == 0 {
                        None
                    } else {
                        Some([r.f32()?, r.f32()?, r.f32()?, r.f32()?])
                    };
                    markers.push(MapMarker {
                        flags,
                        color,
                        pos,
                        line,
                    });
                }
                let mesh = if r.u8()? == 0 {
                    None
                } else {
                    let id = r.u32()?;
                    let mut matrix = [0.0f32; 16];
                    for v in &mut matrix {
                        *v = r.f32()?;
                    }
                    let vb = r.blob()?;
                    let tb = r.blob()?;
                    let vertices = vb
                        .as_chunks::<6>()
                        .0
                        .iter()
                        .map(|c| [0, 2, 4].map(|o| i16::from_le_bytes([c[o], c[o + 1]])))
                        .collect();
                    let triangles = tb
                        .as_chunks::<6>()
                        .0
                        .iter()
                        .map(|c| [0, 2, 4].map(|o| u16::from_le_bytes([c[o], c[o + 1]])))
                        .collect();
                    Some(MapMesh {
                        id,
                        matrix,
                        vertices,
                        triangles,
                    })
                };
                Self::Minimap(Minimap {
                    shown,
                    player,
                    yaw,
                    markers,
                    mesh,
                })
            }
            SCENE_INPUTS => Self::SceneInputs(Box::new(scene_inputs::SceneInputs::read(&mut r)?)),
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

/// A frame on its own (as a `Frame` message's payload; for recordings).
#[must_use]
pub fn encode_frame(f: &Frame) -> Vec<u8> {
    let mut w = Writer::default();
    write_frame(&mut w, f);
    w.bytes
}

/// A frame from [`encode_frame`]'s bytes.
///
/// # Errors
/// On truncated or malformed bytes.
pub fn decode_frame(bytes: &[u8]) -> Result<Frame, DecodeError> {
    read_frame(&mut Reader::new(bytes))
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

/// Reference captures (`docs/renderer.md`, "Reference captures"): what a
/// capture adds next to its scene stream so that two runs can be compared
/// without their heap addresses. They go into `capture.ffxref` (magic
/// [`reference::MAGIC`]), apart from `capture.ffxgfx`, so readers of the
/// scene stream see the same messages as before.
pub mod reference {
    use super::{DecodeError, Id, Reader, Writer};

    /// Magic at the start of `capture.ffxref`.
    pub const MAGIC: &[u8; 8] = b"FFXREF01";

    /// Where a [`SceneSample`] was read.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum When {
        /// On the harness thread when the capture was armed, with the
        /// render thread held in `Present` (lockstep) and the main thread
        /// waiting for it: the state the next frame's draws are built from.
        Held,
        /// On the render thread, inside the `Present` that ended a frame;
        /// the main thread is in its next update meanwhile, so values can
        /// be half written.
        Present,
    }

    /// Tags of one captured frame.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FrameTag {
        /// Index of the frame within the capture, from 0.
        pub frame: u32,
        /// `Present` calls before the one that ended this frame (the
        /// harness's frame count at that `Present`).
        pub present: u32,
        /// Ctrl work 0's frame count (`0x01325d94`) at that `Present`
        /// (render thread: it may already have counted the next update).
        pub event_count: u32,
        pub draws: u32,
        /// (thread id, Direct3D calls the capture saw on it): draws,
        /// clears, copies, render target changes and the noted calls.
        pub calls: Vec<(u32, u32)>,
        /// The thread that called `Present`.
        pub present_thread: u32,
    }

    /// Scene inputs read from the game's memory, by name
    /// (`docs/renderer.md` lists them), bytes as they were.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SceneSample {
        pub frame: u32,
        pub when: When,
        pub thread: u32,
        pub present: u32,
        pub fields: Vec<(String, Vec<u8>)>,
        /// Files the game opened since the previous sample, with the
        /// `Present` count at the open.
        pub opened: Vec<(u32, String)>,
    }

    /// A thread of the game's process at the end of the capture.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ThreadInfo {
        pub id: u32,
        /// Its start address (`FFX.exe` address when inside the image).
        pub start: u32,
        /// `main` (created first), `render` (made the draws), or empty.
        pub role: String,
    }

    /// The content keys of one draw: what it reads, without addresses.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DrawKeys {
        pub frame: u32,
        /// Index of the draw within the frame, from 1.
        pub draw: u32,
        /// (slot, hash of the bytes the draw reads for dynamic buffers, of
        /// the whole buffer for static ones).
        pub vertex: Vec<(u8, u64)>,
        /// Hash of the indices the draw reads (dynamic) or of the buffer.
        pub index: u64,
        /// (stage `v`/`p`, slot, constant buffer window hash).
        pub constants: Vec<(u8, u8, u64)>,
        /// (stage, slot, key): content hash for textures, `role` hash for
        /// render targets read as textures.
        pub textures: Vec<(u8, u8, u64)>,
        /// (slot, role) of each render target, and the depth target's role.
        pub targets: Vec<(u8, String)>,
        pub depth: String,
    }

    /// One reference record.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Ref {
        Tag(FrameTag),
        Scene(SceneSample),
        Threads(Vec<ThreadInfo>),
        Keys(DrawKeys),
        /// A render target's contents when the game bound others after
        /// drawing into it (`after_draw` draws into the frame), or the
        /// back buffer at `Present` (`role` `backbuffer`).
        Target {
            frame: u32,
            after_draw: u32,
            view: Id,
            role: String,
            hash: u64,
        },
        /// The game made a new object at the address of one the layer
        /// knew (the old one was released unseen: the layer does not hook
        /// `Release`); records made before this one about `id` refer to
        /// the old object.
        Reborn {
            frame: u32,
            id: Id,
        },
    }

    const TAG: u32 = 0x101;
    const SCENE: u32 = 0x102;
    const THREADS: u32 = 0x103;
    const KEYS: u32 = 0x104;
    const TARGET: u32 = 0x105;
    const REBORN: u32 = 0x106;

    fn u64w(w: &mut Writer, v: u64) {
        w.bytes.extend_from_slice(&v.to_le_bytes());
    }

    fn u64r(r: &mut Reader) -> Result<u64, DecodeError> {
        let lo = u64::from(r.u32()?);
        Ok(lo | (u64::from(r.u32()?) << 32))
    }

    fn len(w: &mut Writer, n: usize) {
        w.u32(u32::try_from(n).expect("list under 4 G"));
    }

    fn keys3(w: &mut Writer, v: &[(u8, u8, u64)]) {
        len(w, v.len());
        for &(a, b, h) in v {
            w.u8(a);
            w.u8(b);
            u64w(w, h);
        }
    }

    fn read_keys3(r: &mut Reader) -> Result<Vec<(u8, u8, u64)>, DecodeError> {
        let n = r.u32()?;
        (0..n).map(|_| Ok((r.u8()?, r.u8()?, u64r(r)?))).collect()
    }

    impl Ref {
        /// Appends the encoded record (tag, payload length, payload, as
        /// the scene stream's messages).
        pub fn write(&self, out: &mut Vec<u8>) {
            let mut w = Writer::default();
            let tag = match self {
                Self::Tag(t) => {
                    w.u32s(&[t.frame, t.present, t.event_count, t.draws]);
                    len(&mut w, t.calls.len());
                    for &(id, n) in &t.calls {
                        w.u32s(&[id, n]);
                    }
                    w.u32(t.present_thread);
                    TAG
                }
                Self::Scene(s) => {
                    w.u32(s.frame);
                    w.u8(match s.when {
                        When::Held => 0,
                        When::Present => 1,
                    });
                    w.u32s(&[s.thread, s.present]);
                    len(&mut w, s.fields.len());
                    for (name, bytes) in &s.fields {
                        w.str(name);
                        w.blob(bytes);
                    }
                    len(&mut w, s.opened.len());
                    for (at, path) in &s.opened {
                        w.u32(*at);
                        w.str(path);
                    }
                    SCENE
                }
                Self::Threads(list) => {
                    len(&mut w, list.len());
                    for t in list {
                        w.u32s(&[t.id, t.start]);
                        w.str(&t.role);
                    }
                    THREADS
                }
                Self::Keys(k) => {
                    w.u32s(&[k.frame, k.draw]);
                    len(&mut w, k.vertex.len());
                    for &(slot, h) in &k.vertex {
                        w.u8(slot);
                        u64w(&mut w, h);
                    }
                    u64w(&mut w, k.index);
                    keys3(&mut w, &k.constants);
                    keys3(&mut w, &k.textures);
                    len(&mut w, k.targets.len());
                    for (slot, role) in &k.targets {
                        w.u8(*slot);
                        w.str(role);
                    }
                    w.str(&k.depth);
                    KEYS
                }
                Self::Target {
                    frame,
                    after_draw,
                    view,
                    role,
                    hash,
                } => {
                    w.u32s(&[*frame, *after_draw, *view]);
                    w.str(role);
                    u64w(&mut w, *hash);
                    TARGET
                }
                Self::Reborn { frame, id } => {
                    w.u32s(&[*frame, *id]);
                    REBORN
                }
            };
            out.extend_from_slice(&tag.to_le_bytes());
            out.extend_from_slice(
                &u32::try_from(w.bytes.len())
                    .expect("record under 4 GiB")
                    .to_le_bytes(),
            );
            out.extend_from_slice(&w.bytes);
        }

        /// Decodes one record's payload.
        ///
        /// # Errors
        /// When the payload does not match its tag.
        pub fn read(tag: u32, payload: &[u8]) -> Result<Self, DecodeError> {
            let mut r = Reader::new(payload);
            let r = &mut r;
            Ok(match tag {
                TAG => {
                    let [frame, present, event_count, draws] = r.u32s()?;
                    let n = r.u32()?;
                    let calls = (0..n)
                        .map(|_| Ok((r.u32()?, r.u32()?)))
                        .collect::<Result<_, DecodeError>>()?;
                    Self::Tag(FrameTag {
                        frame,
                        present,
                        event_count,
                        draws,
                        calls,
                        present_thread: r.u32()?,
                    })
                }
                SCENE => {
                    let frame = r.u32()?;
                    let when = match r.u8()? {
                        0 => When::Held,
                        1 => When::Present,
                        _ => return Err(DecodeError("sample point")),
                    };
                    let [thread, present] = r.u32s()?;
                    let n = r.u32()?;
                    let fields = (0..n)
                        .map(|_| Ok((r.str()?, r.blob()?)))
                        .collect::<Result<_, DecodeError>>()?;
                    let n = r.u32()?;
                    let opened = (0..n)
                        .map(|_| Ok((r.u32()?, r.str()?)))
                        .collect::<Result<_, DecodeError>>()?;
                    Self::Scene(SceneSample {
                        frame,
                        when,
                        thread,
                        present,
                        fields,
                        opened,
                    })
                }
                THREADS => {
                    let n = r.u32()?;
                    Self::Threads(
                        (0..n)
                            .map(|_| {
                                let [id, start] = r.u32s()?;
                                Ok(ThreadInfo {
                                    id,
                                    start,
                                    role: r.str()?,
                                })
                            })
                            .collect::<Result<_, DecodeError>>()?,
                    )
                }
                KEYS => {
                    let [frame, draw] = r.u32s()?;
                    let n = r.u32()?;
                    let vertex = (0..n)
                        .map(|_| Ok((r.u8()?, u64r(r)?)))
                        .collect::<Result<_, DecodeError>>()?;
                    let index = u64r(r)?;
                    let constants = read_keys3(r)?;
                    let textures = read_keys3(r)?;
                    let n = r.u32()?;
                    let targets = (0..n)
                        .map(|_| Ok((r.u8()?, r.str()?)))
                        .collect::<Result<_, DecodeError>>()?;
                    Self::Keys(DrawKeys {
                        frame,
                        draw,
                        vertex,
                        index,
                        constants,
                        textures,
                        targets,
                        depth: r.str()?,
                    })
                }
                TARGET => {
                    let [frame, after_draw, view] = r.u32s()?;
                    Self::Target {
                        frame,
                        after_draw,
                        view,
                        role: r.str()?,
                        hash: u64r(r)?,
                    }
                }
                REBORN => {
                    let [frame, id] = r.u32s()?;
                    Self::Reborn { frame, id }
                }
                _ => return Err(DecodeError("tag")),
            })
        }
    }

    /// Splits a `capture.ffxref` (after its magic) into records.
    ///
    /// # Errors
    /// On a truncated or malformed file.
    pub fn read_all(bytes: &[u8]) -> Result<Vec<Ref>, DecodeError> {
        let mut out = Vec::new();
        let mut at = 0;
        while at + 8 <= bytes.len() {
            let word =
                |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
            let (tag, n) = (word(at), word(at + 4) as usize);
            let payload = bytes
                .get(at + 8..at + 8 + n)
                .ok_or(DecodeError("truncated record"))?;
            out.push(Ref::read(tag, payload)?);
            at += 8 + n;
        }
        Ok(out)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn records_round_trip() {
            let refs = vec![
                Ref::Tag(FrameTag {
                    frame: 1,
                    present: 900,
                    event_count: 102,
                    draws: 1234,
                    calls: vec![(0x1a2c, 5000)],
                    present_thread: 0x1a2c,
                }),
                Ref::Scene(SceneSample {
                    frame: 0,
                    when: When::Held,
                    thread: 7,
                    present: 899,
                    fields: vec![("g_CameraMatrix".into(), vec![1, 2, 3])],
                    opened: vec![(
                        12,
                        "ffx_ps2/ffx/master/jppc/event/obj/cd/cdsp0000.ebp".into(),
                    )],
                }),
                Ref::Threads(vec![ThreadInfo {
                    id: 7,
                    start: 0x0061_4710,
                    role: "render".into(),
                }]),
                Ref::Keys(DrawKeys {
                    frame: 0,
                    draw: 3,
                    vertex: vec![(0, u64::MAX - 1)],
                    index: 5,
                    constants: vec![(b'v', 0, 9)],
                    textures: vec![(b'p', 1, 10)],
                    targets: vec![(0, "1920x1080f28b28#0".into())],
                    depth: String::new(),
                }),
                Ref::Target {
                    frame: 0,
                    after_draw: 3,
                    view: 0x1234,
                    role: "backbuffer".into(),
                    hash: 0xdead_beef_0000_0001,
                },
                Ref::Reborn { frame: 2, id: 0x55 },
            ];
            let mut bytes = Vec::new();
            for r in &refs {
                r.write(&mut bytes);
            }
            assert_eq!(read_all(&bytes).unwrap(), refs);
        }
    }
}

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
                    Input::Motion(-2.0, 7.0),
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
            Msg::Minimap(Minimap {
                shown: 0.5,
                player: [1.0, 2.0, 3.0],
                yaw: 0.25,
                markers: vec![
                    MapMarker {
                        flags: 0x800,
                        color: [1.0, 1.0, 0.5, 1.0],
                        pos: [1.0, 0.0, 2.0],
                        line: None,
                    },
                    MapMarker {
                        flags: 0x10003,
                        color: [0.0, 0.5, 1.0, 1.0],
                        pos: [4.0, 0.0, 5.0],
                        line: Some([1.0, 2.0, 3.0, 4.0]),
                    },
                ],
                mesh: Some(MapMesh {
                    id: 7,
                    matrix: [1.0; 16],
                    vertices: vec![[1, -2, 3], [4, 5, -6]],
                    triangles: vec![[0, 1, 0]],
                }),
            }),
            Msg::SceneInputs(Box::new(scene_inputs::SceneInputs {
                frame: 1,
                update: 2,
                thread: 3,
                world: 0xac,
                pass: "OpaqueNoMSAA".into(),
                dir: -1,
                camera: vec![1, 2, 3],
                app: vec![4; 8],
                screen: [1920, 1080],
                group_mask: 0xffff_fff7,
                shadow_solution: 2,
                alpha_fast: 1,
                layer_group: 1,
                layers: vec![scene_inputs::Layer {
                    shown: 1,
                    group: -1,
                    id: -2,
                    has_mesh: true,
                    mesh: Some((0, 3)),
                }],
                depth_bias_count: 1,
                depth_bias: vec![scene_inputs::DepthBias {
                    bounds: Some(0),
                    scale: 2.0,
                    bias: -1.0,
                }],
                clusters: vec![scene_inputs::Cluster {
                    index: 3,
                    name: "cdsp00".into(),
                    lists: vec![("PMeshInstance".into(), 29)],
                }],
                bounds: vec![scene_inputs::Bounds {
                    cluster: 0,
                    slot: 5,
                    list: 0,
                    min: [1.0, 2.0, 3.0],
                    size: [4.0, 5.0, 6.0],
                    world: Some([10.0; 12]),
                    instance: None,
                    render_order: 4,
                }],
                instances: vec![scene_inputs::Instance {
                    cluster: 0,
                    slot: 7,
                    world: None,
                    world_shared: true,
                    segments: vec![scene_inputs::Segment {
                        material: 1,
                        matrix: 0,
                        hidden: 1,
                        id: 9,
                        render_order: 2,
                    }],
                    materials: vec![None, Some(0)],
                    group_id: 0xc,
                    dobj_kind: 1,
                    dobj_flag: 0x101,
                    obj_id: 8,
                }],
                materials: vec![scene_inputs::Material {
                    remap_from: "Opaque".into(),
                    remap_to: String::new(),
                    parameters: vec![16, 0, 0, 0, 1, 2],
                }],
                chars: None,
            })),
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
