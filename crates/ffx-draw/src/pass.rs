//! A draw list for one pass of a world: what the renderer issues in place
//! of `PhyreEngine`'s `renderWorld` (`docs/draw.md`, "The map path").
//!
//! A list is a run of [`PassDraw`]s: each one's [`DrawState`] and
//! [`DrawCall`], the bytes of its constant buffer windows, and buffer
//! writes to make before it. Its objects are backend handles, made through
//! the backend (the scene builder's own resources) or brought in from the
//! host (the game's objects, `D3d11::import`). [`issue`] draws a list
//! between `begin` and `end`, so the host's state is put back afterwards.

use crate::backend::{Backend, Buffer};
use crate::cache::Cache;
use crate::desc::Stage;
use crate::state::{CbBinding, DrawCall, DrawState};

/// Which pass of a world (`Draw3DWorld` `0x0064fbb0` sets it before each
/// `PApplication_renderWorld`; docs/formats/scene.md, "Passes").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pass {
    /// `Opaque` / `OpaqueNoMSAA`, render direction 1 (near to far).
    Opaque,
    /// `Transparent` / `TransparentNoMSAA`, direction -1 (far to near).
    Transparent,
}

impl Pass {
    /// The pass of a `renderWorld` direction.
    #[must_use]
    pub fn from_dir(dir: i32) -> Self {
        if dir >= 0 {
            Self::Opaque
        } else {
            Self::Transparent
        }
    }
}

/// One constant buffer window a draw's shader sees: bound whole at `slot`
/// of `stage` from one of the cache's dynamic buffers written with `data`
/// just before the draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constants {
    pub stage: Stage,
    pub slot: u8,
    pub data: Vec<u8>,
}

/// New contents for part of a buffer, written before a draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferWrite {
    pub buffer: Buffer,
    pub offset: u32,
    pub data: Vec<u8>,
}

/// One draw of a pass.
#[derive(Debug, Clone, PartialEq)]
pub struct PassDraw {
    /// Everything bound but the constant buffers named in `constants`
    /// (their slots are filled in by [`issue`]).
    pub state: DrawState,
    pub call: DrawCall,
    pub constants: Vec<Constants>,
    pub writes: Vec<BufferWrite>,
}

/// What [`issue`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Issued {
    pub draws: usize,
    /// Constant buffers the backend refused (the draw went ahead without
    /// that slot).
    pub refused: usize,
}

/// Issues `draws` in order: `begin`, per draw its buffer writes, its
/// constants and the draw, then `end` (the host's state is put back).
pub fn issue<B: Backend>(b: &mut B, cache: &mut Cache, draws: &[PassDraw]) -> Issued {
    let mut out = Issued::default();
    if draws.is_empty() {
        return out;
    }
    b.begin();
    for d in draws {
        for w in &d.writes {
            b.write_buffer(w.buffer, w.offset, &w.data);
        }
        let mut state = d.state;
        for c in &d.constants {
            let bindings = match c.stage {
                Stage::Vertex => &mut state.vs,
                Stage::Pixel => &mut state.ps,
                Stage::Geometry => continue,
            };
            let Some(at) = bindings.cbs.get_mut(usize::from(c.slot)) else {
                continue;
            };
            match cache.constants(b, c.stage, c.slot, &c.data) {
                Ok(buffer) => {
                    *at = Some(CbBinding {
                        buffer,
                        first: 0,
                        count: 0,
                    });
                }
                Err(_) => out.refused += 1,
            }
        }
        b.draw(&state, d.call);
        out.draws += 1;
    }
    b.end();
    out
}
