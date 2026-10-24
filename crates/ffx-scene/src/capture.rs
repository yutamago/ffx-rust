//! A `capture.ffxgfx` (`gfxcapture`, `ffx-hook`'s `gfx/capture.rs`): the
//! objects one frame used and its commands, with the lookups the census
//! needs.

use std::collections::HashMap;

use ffx_gfx::{Cmd, Draw, Frame, Msg, StateKind, ViewKind};

use crate::key;

#[derive(Debug)]
pub enum Error {
    Magic,
    Decode(&'static str),
    NoFrame,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Magic => write!(f, "not a capture.ffxgfx (magic)"),
            Self::Decode(why) => write!(f, "malformed capture: {why}"),
            Self::NoFrame => write!(f, "the capture has no frame"),
        }
    }
}

impl std::error::Error for Error {}

/// The scene targets: colour and view-space normal with a depth buffer
/// (as `ffx-hook`'s `gfx/live.rs` tells scene draws from the rest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneTargets {
    pub color: u32,
    pub normal: u32,
    pub depth: u32,
}

/// A loaded capture.
#[derive(Debug, Clone)]
pub struct Capture {
    pub objects: HashMap<u32, Msg>,
    pub frame: Frame,
    /// The `World` message: scene name and camera.
    pub world: Option<Msg>,
}

/// A texture as captured: its description and every subresource.
#[derive(Debug, Clone, Copy)]
pub struct TextureRef<'a> {
    pub id: u32,
    pub desc: &'a ffx_gfx::TextureDesc,
    pub subresources: &'a [ffx_gfx::Subresource],
}

impl TextureRef<'_> {
    /// The top mip level's texels, rows packed; `None` for formats
    /// without a known block size.
    #[must_use]
    pub fn top_mip(&self) -> Option<Vec<u8>> {
        let row = key::row_bytes(self.desc[4], self.desc[0])?;
        let sub = self.subresources.first()?;
        Some(key::pack_rows(&sub.data, sub.row_pitch as usize, row))
    }

    /// The `ffx-drawdiff` key ([`key::texture`]).
    #[must_use]
    pub fn key(&self) -> u64 {
        key::texture(self.desc, self.subresources)
    }

    /// [`key::content`] of the top mip's texels.
    #[must_use]
    pub fn content(&self) -> Option<u64> {
        self.top_mip().map(|m| key::content(&m))
    }
}

impl Capture {
    /// Reads a capture file's bytes.
    ///
    /// # Errors
    /// On a wrong magic, a malformed stream or a stream without a frame.
    pub fn read(bytes: &[u8]) -> Result<Self, Error> {
        let rest = bytes
            .strip_prefix(&ffx_gfx::CAPTURE_MAGIC[..])
            .ok_or(Error::Magic)?;
        let msgs = ffx_gfx::read_all(rest).map_err(|e| Error::Decode(e.0))?;
        let mut objects = HashMap::new();
        let mut frame = None;
        let mut world = None;
        for m in msgs {
            let id = match &m {
                Msg::Buffer { id, .. }
                | Msg::Texture { id, .. }
                | Msg::View { id, .. }
                | Msg::Shader { id, .. }
                | Msg::Layout { id, .. }
                | Msg::State { id, .. } => Some(*id),
                _ => None,
            };
            match (id, m) {
                (Some(id), m) => {
                    objects.insert(id, m);
                }
                (None, Msg::Frame(f)) => frame = Some(f),
                (None, m @ Msg::World { .. }) => world = Some(m),
                _ => {}
            }
        }
        Ok(Self {
            objects,
            frame: frame.ok_or(Error::NoFrame)?,
            world,
        })
    }

    /// The draws, in order, with their index among the frame's commands.
    pub fn draws(&self) -> impl Iterator<Item = (usize, &Draw)> {
        self.frame
            .cmds
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c {
                Cmd::Draw(d) => Some((i, &**d)),
                _ => None,
            })
    }

    /// The scene targets a draw writes, when it is a scene draw.
    #[must_use]
    pub fn scene_targets(d: &Draw) -> Option<SceneTargets> {
        let slot = |s: u8| d.rtv.iter().find(|r| r.0 == s).map(|r| r.1);
        (d.rtv.len() == 2 && d.dsv != 0).then_some(())?;
        Some(SceneTargets {
            color: slot(0)?,
            normal: slot(1)?,
            depth: d.dsv,
        })
    }

    /// A shader's bytecode.
    #[must_use]
    pub fn shader(&self, id: u32) -> Option<&[u8]> {
        match self.objects.get(&id)? {
            Msg::Shader { bytecode, .. } => Some(bytecode),
            _ => None,
        }
    }

    /// A buffer's description and contents (for dynamic buffers, the
    /// contents at the end of the frame).
    #[must_use]
    pub fn buffer(&self, id: u32) -> Option<(&ffx_gfx::BufferDesc, &[u8])> {
        match self.objects.get(&id)? {
            Msg::Buffer { desc, data, .. } => Some((desc, data)),
            _ => None,
        }
    }

    /// The texture behind a shader resource view.
    #[must_use]
    pub fn view_texture(&self, view: u32) -> Option<TextureRef<'_>> {
        let Msg::View {
            kind: ViewKind::ShaderResource,
            resource,
            ..
        } = self.objects.get(&view)?
        else {
            return None;
        };
        match self.objects.get(resource)? {
            Msg::Texture {
                id,
                desc,
                subresources,
            } => Some(TextureRef {
                id: *id,
                desc,
                subresources,
            }),
            _ => None,
        }
    }

    /// The resource behind a view.
    #[must_use]
    pub fn view_resource(&self, view: u32) -> Option<u32> {
        match self.objects.get(&view)? {
            Msg::View { resource, .. } => Some(*resource),
            _ => None,
        }
    }

    /// An input layout's elements.
    #[must_use]
    pub fn layout(&self, id: u32) -> Option<&[ffx_gfx::Element]> {
        match self.objects.get(&id)? {
            Msg::Layout { elements, .. } => Some(elements),
            _ => None,
        }
    }

    /// A state object's description bytes (`D3D11_*_DESC`).
    #[must_use]
    pub fn state(&self, id: u32, kind: StateKind) -> Option<&[u8]> {
        match self.objects.get(&id)? {
            Msg::State { kind: k, desc, .. } if *k == kind => Some(desc),
            _ => None,
        }
    }
}
