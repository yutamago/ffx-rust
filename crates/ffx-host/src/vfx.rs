//! The effects' draws as the game makes them (docs/state-api.md, "Effect
//! draws"): an opt-in capture of what `PhyreScene_DrawMostVFX`
//! (`0x00650670`) draws each frame, for a frontend that draws the game's
//! own effects (gd-ffx).
//!
//! The capture is off unless a frontend switches it on
//! ([`Capture::set_enabled`]); off, the wrapper only checks the switch and
//! calls the port. On, it first reads the scene's list `+0x38cc` (count
//! `+0xf8cc`), its draw events (`+0xfb5c`, count `+0xfb54`) and, for each
//! entry, the `VFXDynamicGeometry` of `g_VFXRenderDataTable` whose mesh
//! instance (`+0xc`) it is: the instance key, the VSP flags (`+0x3c`), the
//! mesh instance's world matrix and each segment with indices, with its
//! texture name, vertex arrays and the blend of each shader pass its
//! material's technique draws with (`docs/formats/effects-draw.md`). It only
//! reads game memory: on or off, the game runs the same.
//!
//! `DrawMostVFX` runs on the game's main thread between two updates, after
//! update `n` and before the boundary of update `n + 1`: the frame it draws
//! is the state after `n` updates, the [`ffx_state::Snapshot`] whose
//! `update` is `n` ([`Frame::update`]; checked against reads at the
//! boundary on Besaid, docs/state-api.md).
//!
//! The same read takes the screen distortion's state ([`Distortion`],
//! `Distortion_GetInstance`): `Distortion_renderDistortion` (`0x006c2dc0`)
//! draws it later in the same `PhyreScene_render` (`0x00668930`), after the
//! effects, with nothing between that writes it.
//!
//! The same read takes the scene's other VFX lists ([`Pass`]): the DTT
//! list (`PhyreScene_RenderDTTVFX`), the list drawn after post-processing
//! (`PhyreScene_RenderVFXAfterPostProcess`), the UI 2D list
//! (`PhyreScene_DrawUI2DVFX`, while the scene fades) and `+0x3880` (no pass
//! draws it). They are filled by the same updates as the main list and
//! drawn later in the same `PhyreScene_render`.
//!
//! The newest frames are kept ([`Capture::latest`], [`Capture::frame`]) and
//! encode to a blob of their own ([`Frame::encode`]), apart from the
//! snapshot.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use crate::arena::Arena;
use crate::exec::Exec;
use crate::table::{Args, Conv, Entry, Port, Ret, Table};

/// `PhyreScene_DrawMostVFX(this)` (thiscall, `eax` 0 or an event's error).
pub const DRAW_MOST_VFX: u32 = 0x0065_0670;
/// `g_VFXRenderDataTable`: the pointer to the table.
pub const TABLE: u32 = 0x00cc_ca6c;
/// `g_MagicFile_MagicId` (what `MagicFile_GetMagicId` `0x009da3b0` reads).
pub const MAGIC_ID: u32 = 0x00c6_4ca0;
/// The magic whose geometry [`LAST_DRAWN_KEY`] `DrawMostVFX` draws last.
pub const LAST_DRAWN_MAGIC: u32 = 0x296;
/// The key of that geometry, `(lo, hi)` = `(0x78, 0x5bc)`.
pub const LAST_DRAWN_KEY: u64 = (0x5bc << 32) | 0x78;
/// The encoding's first four bytes ([`Frame::encode`]).
pub const MAGIC: [u8; 4] = *b"FXDR";
/// The encoding's version (5: draws carry their texture's [`Sampler`]; 4
/// added the draw's [`Technique`] and a [`Blend`] per shader pass, 3 the
/// draw's [`Pass`] and the event's actor slot).
pub const VERSION: u16 = 5;
/// The render pass every VFX list is drawn with (`renderer +0x48`):
/// `"Transparent"` (`DrawMostVFX`, `RenderDTTVFX`,
/// `RenderVFXAfterPostProcess` and `DrawUI2DVFX` all set it).
pub const PASS_TRANSPARENT: u32 = 0x00c1_69d4;
/// `"OptimizedTrans"`, the pass `PhyreScene_UseVFXOptimizedTrans`
/// (`0x00654b20`) remaps a VFX material's [`PASS_TRANSPARENT`] to.
pub const PASS_OPTIMIZED_TRANS: u32 = 0x00c1_6b84;
/// A shader pass's `D3D11_BLEND_DESC` (`PShaderPassStateD3D11`: the state
/// block at `+0x18`, its blend description at block `+0x60`; the blend
/// state made from it at block `+0x174`).
const PASS_BLEND_DESC: u32 = 0x78;
/// A shader's passes are `PShaderPassStateD3D11`s of this size, one after
/// the other (`queueMeshCommands` `0x005ba380` steps by it).
const SHADER_PASS_SIZE: u32 = 0x254;
/// At most this many shader passes are read for a draw.
const SHADER_PASSES_MAX: u32 = 8;
/// A shader parameter's type for a sampler state (`PShaderParameterDefinition`
/// `+2`; docs/formats/phyre.md, "Materials").
const PARAM_SAMPLER_STATE: u8 = 0x47;
/// The parameter the VFX pixel shaders sample the texture with (`s0`).
const TEXTURE_SAMPLER_STATE: &str = "TextureSamplerState";
/// Bounds on the effect's tweakable parameter definitions read.
const PARAMETERS_MAX: u32 = 256;
/// `PRenderer`'s global maximum anisotropy (`f32`), which
/// `PSamplerStateD3D11::create` (`0x00595bf0`) applies over 1.
const MAX_ANISOTROPY: u32 = 0x00c2_5370;
/// `Distortion_GetInstance`'s instance pointer (`0x006c2290`).
pub const DISTORTION: u32 = 0x00cd_eb88;
/// The distortion grid: 9 x 9 vertices (`0x006c2700`), 8 x 8 quads of two
/// triangles (`0x006c26d0`: 0x180 indices).
pub const GRID_VERTICES: usize = 81;
pub const GRID_INDICES: usize = 0x180;
/// The copy's alpha cap in `Distortion_renderDistortion`.
pub const COPY_ALPHA_MAX: f32 = 0.8;
/// The list's capacity (`+0x38cc`, 0xc00 entries).
const LIST_MAX: u32 = 0xc00;
/// At most this many entries of one of the other lists are read (`PArray`s
/// with no fixed capacity).
const OTHER_MAX: u32 = 0x1000;
/// `g_MovieFading` (`DrawUI2DVFX` draws while it or the scene's `+0x10dcc`
/// is set).
const MOVIE_FADING: u32 = 0x0134_0824;
/// Non-zero on maps 0x163 and 0x164, where `PhyreScene_render` draws the
/// 2D VFX in its late step (after the post-processing, before the screen
/// fade and the after-post VFX) instead of the UI step.
const MAP_163_OR_164: u32 = 0x00cc_b9c4;
/// `PhyreScene_render`'s menu capture state: at 2 the scene step skips its
/// post chain, the DTT VFX with it (`h3e_scene`).
const MENU_CAPTURE_STATE: u32 = 0x0134_0844;
/// The name `DrawUI2DVFX` skips (`[0x00ccb9e4] & !1` against the mesh
/// instance's `+0x2c`, `Phyre_PRendering_NameIs`).
const UI2D_SKIP_NAME: u32 = 0x00cc_b9e4;
/// `g_ChrCount` and `g_ChrArray` (records of 0x880 bytes): a character
/// instance's `+0x18` is one of them.
pub(crate) const CHR_COUNT: u32 = 0x023c_44e0;
pub(crate) const CHR_ARRAY: u32 = 0x023c_44e4;
pub(crate) const CHR_SIZE: u32 = 0x880;
/// Bounds on what one frame reads (guards against a torn list).
const SEGMENTS_MAX: i32 = 256;
const VERTICES_MAX: u32 = 0x1_0000;
const INDICES_MAX: u32 = 0x3_0000;
const EVENTS_MAX: u32 = 0x1000;
/// The identity in [`Draw::world`]'s layout.
pub const IDENTITY: [f32; 12] = [0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
/// Frames kept ([`Capture::frame`]).
const KEPT: usize = 4;

/// The pass that draws a [`Draw`], in frame order
/// (`PhyreScene_render`, docs/formats/postfx.md "Order"; the numbers are
/// the encoding's).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Pass {
    /// `PhyreScene_DrawMostVFX` (`+0x38cc`): into the scene colour against
    /// the scene depth (test on, write off), after the glow (or before it
    /// with `isGlowAfterVFX`), before the distortion; the draw events run
    /// between its draws.
    #[default]
    Main = 0,
    /// `PhyreScene_RenderDTTVFX` (`0x006531b0`, `+0x385c`, count `+0x3854`):
    /// right after the main pass and the DTT copies to pp `+0xa8`, before the
    /// specular pass and the screen fade; the scene pass, no camera change.
    Dtt = 1,
    /// `PhyreScene_RenderVFXAfterPostProcess` (`0x006538d0`, `+0x3888`,
    /// count `+0x388c`): after the post-processing (distortion, blur, AA,
    /// brightness), before the UI, on the colour and depth with the scene
    /// camera and the default pass.
    AfterPost = 2,
    /// `PhyreScene_DrawUI2DVFX` (`0x00650ea0`, `+0x3868`, count `+0x3860`):
    /// in the UI step, before the normal UI, with the scene's second camera
    /// (`+0x500`) and the transparent pass; only while the scene fades
    /// (`+0x10dcc` or `g_MovieFading`); the capture applies the same test
    /// and the pass's name skip. On maps 0x163/0x164 the late step draws it
    /// instead, after the post-processing and before [`Pass::AfterPost`]:
    /// there the frame holds these draws before the after-post ones (the
    /// numbers order the passes everywhere else).
    Ui2d = 3,
    /// `+0x3880` (count `+0x3878`, `PhyreScene_AddVFXToList3880`, geometry
    /// byte `+0x50`): no pass in `PhyreScene_render` draws it (as far as
    /// is known); captured so that a frontend can tell, not to be drawn.
    Undrawn = 4,
}

impl Pass {
    /// Every pass, in frame order.
    pub const ALL: [Self; 5] = [
        Self::Main,
        Self::Dtt,
        Self::AfterPost,
        Self::Ui2d,
        Self::Undrawn,
    ];

    /// The pass of the encoding's number.
    #[must_use]
    pub fn from_u8(v: u8) -> Option<Self> {
        Self::ALL.get(usize::from(v)).copied()
    }
}

/// A Direct3D 11 blend factor (`D3D11_BLEND`; the numbers are D3D11's and
/// the encoding's).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlendFactor {
    Zero = 1,
    #[default]
    One = 2,
    SrcColour = 3,
    InvSrcColour = 4,
    SrcAlpha = 5,
    InvSrcAlpha = 6,
    DestAlpha = 7,
    InvDestAlpha = 8,
    DestColour = 9,
    InvDestColour = 10,
    SrcAlphaSat = 11,
    /// The blend factor constant: 0, 0, 0, 0 for every VFX draw
    /// (`PShaderPassStateD3D11_bind` passes zeros).
    BlendFactor = 14,
    InvBlendFactor = 15,
    Src1Colour = 16,
    InvSrc1Colour = 17,
    Src1Alpha = 18,
    InvSrc1Alpha = 19,
}

impl BlendFactor {
    /// The factor of D3D11's number.
    #[must_use]
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::Zero,
            2 => Self::One,
            3 => Self::SrcColour,
            4 => Self::InvSrcColour,
            5 => Self::SrcAlpha,
            6 => Self::InvSrcAlpha,
            7 => Self::DestAlpha,
            8 => Self::InvDestAlpha,
            9 => Self::DestColour,
            10 => Self::InvDestColour,
            11 => Self::SrcAlphaSat,
            14 => Self::BlendFactor,
            15 => Self::InvBlendFactor,
            16 => Self::Src1Colour,
            17 => Self::InvSrc1Colour,
            18 => Self::Src1Alpha,
            19 => Self::InvSrc1Alpha,
            _ => return None,
        })
    }
}

/// A Direct3D 11 blend operation (`D3D11_BLEND_OP`, the same numbers):
/// `Add` is `src * src factor + dest * dest factor`, `RevSubtract`
/// `dest * dest factor - src * src factor`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlendOp {
    #[default]
    Add = 1,
    Subtract = 2,
    RevSubtract = 3,
    Min = 4,
    Max = 5,
}

impl BlendOp {
    /// The operation of D3D11's number.
    #[must_use]
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::Add,
            2 => Self::Subtract,
            3 => Self::RevSubtract,
            4 => Self::Min,
            5 => Self::Max,
            _ => return None,
        })
    }
}

/// The effect technique a draw uses: the material's remapping of the
/// [`PASS_TRANSPARENT`] pass (material `+8` → `+0xc`). The two techniques
/// of a VFX shader run different pixel shaders, which write the colour
/// differently (premultiplied by the alpha or not; per shader family in
/// docs/formats/effects-draw.md, "Blend"): a frontend needs the technique
/// with the blend.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Technique {
    /// No remapping: the effect's `Transparent` technique.
    #[default]
    Transparent = 1,
    /// Remapped to `OptimizedTrans` (`graphicUseOptimizedVFXShader` with
    /// `on`: the shp and mdl fills do it when no vertex colour component
    /// is above 0.5, and switch it off again otherwise).
    OptimizedTrans = 2,
    /// Remapped to another pass (`ForceTransparent`, …; not seen on VFX).
    Other = 3,
}

impl Technique {
    /// The technique of the encoding's number.
    #[must_use]
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::Transparent,
            2 => Self::OptimizedTrans,
            3 => Self::Other,
            _ => return None,
        })
    }
}

/// The blend state one shader pass draws with: the `D3D11_BLEND_DESC`
/// (render target 0) of the pass, read from its pass state as the game set
/// it up (the state object `OMSetBlendState` binds is made from it). Blend
/// factor constant 0, 0, 0, 0; sample mask all ones; the scene colour
/// target is `R8G8B8A8_UNORM`, so the pixel shader's output is clamped to
/// 0..1 before blending.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Blend {
    /// `BlendEnable`: false writes the shader's output as it is.
    pub enabled: bool,
    /// Colour: `SrcBlend`, `DestBlend`, `BlendOp`.
    pub src: BlendFactor,
    pub dest: BlendFactor,
    pub op: BlendOp,
    /// Alpha: `SrcBlendAlpha`, `DestBlendAlpha`, `BlendOpAlpha`.
    pub src_alpha: BlendFactor,
    pub dest_alpha: BlendFactor,
    pub op_alpha: BlendOp,
    /// `RenderTargetWriteMask` (bit 0 red … bit 3 alpha; 15 all).
    pub write_mask: u8,
}

/// A texture address mode: D3D11's `D3D11_TEXTURE_ADDRESS_MODE` numbers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Address {
    /// Repeats the texture (`u` and `u + 1` sample the same texel).
    #[default]
    Wrap = 1,
    /// Repeats it mirrored every other time.
    Mirror = 2,
    /// Clamps to the edge texels (0..1).
    Clamp = 3,
    /// Outside 0..1 the border colour ([`Sampler::border`]).
    Border = 4,
    /// Mirrored once about 0, then clamped.
    MirrorOnce = 5,
}

impl Address {
    /// The mode of D3D11's number.
    #[must_use]
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::Wrap,
            2 => Self::Mirror,
            3 => Self::Clamp,
            4 => Self::Border,
            5 => Self::MirrorOnce,
            _ => return None,
        })
    }

    /// The mode of a `PSamplerState` wrap byte, as `0x0058ab10` maps it: 1
    /// wrap, 3 border, 4 mirror, anything else clamp.
    #[must_use]
    pub fn from_phyre(v: u8) -> Self {
        match v {
            1 => Self::Wrap,
            3 => Self::Border,
            4 => Self::Mirror,
            _ => Self::Clamp,
        }
    }
}

/// The sampler a draw's texture is read with (`TextureSamplerState`, the
/// pixel shader's `s0`): the material's `PSamplerState` described as
/// `PSamplerStateD3D11::create` (`0x00595bf0`) makes its
/// `D3D11_SAMPLER_DESC`. Not carried: the comparison function
/// (`LESS_EQUAL` with the comparison bit `0x80` in [`Sampler::filter`],
/// else `NEVER`) and the LOD range (`-FLT_MAX .. FLT_MAX`), which the game
/// always sets so.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Sampler {
    /// `Filter` (`D3D11_FILTER`): `0x15` trilinear (min, mag and mip
    /// linear), `0x55` anisotropic (the renderer's maximum anisotropy over
    /// 1), …; from the min filter byte `+0`, the mag filter byte `+1` and
    /// the comparison bit 6 of `+0x1c` (`0x0058aa10`).
    pub filter: u32,
    /// `AddressU`, `AddressV`, `AddressW` (the wrap bytes `+2..+4`).
    pub address: [Address; 3],
    /// `MipLODBias` (`+8`).
    pub mip_lod_bias: f32,
    /// `MaxAnisotropy`: `+0xc` truncated, or the renderer's maximum's
    /// integer part when that is over 1.
    pub max_anisotropy: u32,
    /// `BorderColor` as bytes (R, G, B, A; the description divides each by
    /// 255), used by [`Address::Border`].
    pub border: [u8; 4],
}

/// One indexed draw: one segment of one list entry's geometry.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Draw {
    /// The pass and list it is drawn from.
    pub pass: Pass,
    /// The entry's place in its pass's list (for [`Pass::Main`], `+0x38cc`,
    /// the position draw events are given in).
    pub place: u32,
    /// The instance key `(hi << 32) | lo`, as the effect code passes it to
    /// `graphicVFXGetRenderDataByInstance(lo, hi)`: for a ppp program
    /// `(g_pppCurrentProgram, object)`, for the op system `(0, record)`, a
    /// part `i` of a several-part effect at `lo + 10000 i`
    /// (`docs/formats/effects-draw.md`, "Instance keys and makers").
    pub key: u64,
    /// The `VFXDynamicGeometry` (an address, for logs).
    pub geometry: u32,
    /// Its VSP flags (`+0x3c`): `0x8` modulate, `0xa` additive, `0x18`
    /// subtract, `0x9` z-matrix, `0x2` additive untextured, …
    /// (`vfxShader_fx.txt`; docs/formats/effects-draw.md, "Shaders").
    pub flags: u32,
    /// The geometry is in the table's delete pool (destroyed this frame,
    /// still drawn).
    pub pooled: bool,
    /// The segment's place in the geometry's render data.
    pub segment: u16,
    /// The effect technique the segment's material draws with (which
    /// pixel shaders of the family); `None` when the blends could not be
    /// read (no material or technique for the segment, a context variant
    /// the renderer has not picked, a value outside D3D11's).
    pub technique: Option<Technique>,
    /// The technique's shader passes, in draw order: the game draws the
    /// segment once per pass, each with its blend (one pass for every VFX
    /// shader but the DTT one, which has two). Empty with no `technique`.
    pub blends: Vec<Blend>,
    /// The sampler the pixel shader reads the texture with (the material's
    /// `TextureSamplerState`); `None` when unreadable (no material, no
    /// sampler state parameter, as on the untextured families).
    pub sampler: Option<Sampler>,
    /// The mesh instance's world matrix (`*(mesh instance +4)`), 12 floats
    /// laid out as `ffx_state::Camera::world`:
    /// `[Y.x Y.y Y.z X.x | Z.x Z.y Z.z X.y | P.x P.y P.z X.z]` (axes X, Y,
    /// Z and the place P, render space). [`Draw::positions`] are already
    /// placed by it ([`to_render`]). [`IDENTITY`] when there is none.
    pub world: [f32; 12],
    /// The segment's texture name (`+0x20`) as the game holds it, e.g.
    /// `../../../FFX_Data/GameData/PS3Data/map/bsil/bsil00/fp/tex/D3D11/13824_19_0_0_128_128.dds.phyre`;
    /// `NoTexture` for untextured draws, empty when unreadable.
    pub texture: String,
    /// Positions in render space.
    pub positions: Vec<[f32; 3]>,
    /// Colours (R, G, B, A) as the arrays hold them (`0.5` is the PS2's
    /// `0x80`, which the VFX shaders double). As many as positions.
    pub colours: Vec<[f32; 4]>,
    /// Texture coordinates; as many as positions (not meaningful without a
    /// texture: the arrays are left uninitialised). The shaders sample them
    /// as they are with D3D's convention, `v = 0` the texture's first row
    /// as the file stores it, which is its bottom row: on an upright image
    /// of a file texture they sample [`upright_uv`] (not for the DTT's
    /// `FrameBuffer`, a render target, nor the undecoded `0x28` family:
    /// unverified there). Read with [`Draw::sampler`]
    /// (wrapped or clamped outside 0..1).
    pub uvs: Vec<[f32; 2]>,
    /// Triangle-list indices into the vertices.
    pub indices: Vec<u16>,
}

/// A draw event (`PhyreScene_addVFXDrawEvent` `0x00656940` and its two
/// siblings, 16 bytes at scene `+0xfb5c`), run by
/// `PhyreScene_processVFXDrawEvent` (`0x00666e40`) once `at` entries of the
/// main list are drawn. [`Event::info`] decodes the words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// 0 characters with the event camera, 1 copy a buffer, 2 screen fade,
    /// 3 PS2 lighting, 4 specular, 5 draw a character, 6 z pre-pass.
    pub kind: u32,
    /// The main list's count when it was added (plus the current sync
    /// path's offset): it runs before the draws of place `at` and later.
    pub at: u32,
    /// The record's `+8` and `+0xc`.
    pub words: [u32; 2],
    /// For kinds 3 to 6, the `g_ChrArray` slot ([`ffx_state::Actor`]
    /// `slot`) of the character whose instance `words[0]` is (instance
    /// `+0x18`); -1 for the other kinds or when it is no record.
    pub actor: i16,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            kind: 0,
            at: 0,
            words: [0; 2],
            actor: -1,
        }
    }
}

/// A post-process target a copy event reads or writes (the
/// `PostProcessManager`'s cells, docs/formats/postfx.md "Targets").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buffer {
    /// pp `+0x8c`: the frame's colour (scene, glow, effects so far).
    Colour,
    /// pp `+0xa8`: the DTT copy, the frame copy effects sample.
    Copy,
    /// pp `+0x1232c`: the cross-fade capture.
    CrossFade,
}

/// The blend of a screen fade, `calculateVFXSP_AlphaFunction`'s VSP bits
/// as `PostProcessManager_renderScreenFade` picks its pass from them
/// (`+0x1239c`: 2 `0x00ccc96c`, 0x10 `0x00ccc97c`, else `0x00ccc95c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeBlend {
    /// 2 (`VSP_ALPHA_ADDITIVE`).
    Additive,
    /// 0x10 (`VSP_ALPHA_SUBSTRACT`).
    Subtract,
    /// Anything else: the plain alpha blend.
    Alpha,
}

/// An [`Event`]'s words, decoded (from its makers and
/// `PhyreScene_processVFXDrawEvent`; docs/formats/effects-draw.md, "Draw
/// events").
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventInfo {
    /// 0 (`PhyreScene_addChrDrawSituation` `0x006564a0`, from
    /// `graphicCharacterInsertDrawSituation`): with the scene camera set,
    /// every shown (`+0x14`) character instance of the scene's list
    /// (`+0xfb50`) whose layer byte (`+0x15`) is `layer` is drawn (its
    /// matrix put back after each), then the scene ends and begins again.
    /// It also opens a new sync path of the main list.
    Characters { layer: i32 },
    /// 1 (`0x006568a0`, from `graphicVFXAddCopyBufferEvent*`): the scene
    /// ends and `from` is copied into `to`; `rgb` (the `…WithRGB` makers)
    /// copies through `copyDepthBuffer` with the pass `0x00ccc8ec` and a
    /// colour scale of 1, otherwise `copyBuffer`; `clear` (the `…WithClear`
    /// makers) clears the colour to 0 when the scene begins again. Skipped
    /// while the motion blur is on and the destination byte is 1 (decoded
    /// as [`Buffer::Copy`], as are values of 3 and more, which are not
    /// skipped).
    CopyBuffer {
        from: Buffer,
        to: Buffer,
        rgb: bool,
        clear: bool,
    },
    /// 2 (`graphicVFXAddScreenFadeEvent` `0x00644d40`, `op_flash_path3`):
    /// the scene ends, the screen fade is drawn over the colour with
    /// `colour` (the bytes / 128: 1.0 is the PS2's `0x80`) and `blend`
    /// (kept unless the fade colour is locked), and the scene begins again.
    ScreenFade { colour: [f32; 4], blend: FadeBlend },
    /// 3 (`Phyre_PS2LightingSetup_registerWithPointLightEffect`
    /// `0x006e5c90`): the PS2 point-light set-up of the character instance
    /// `instance` is applied (`0x006e5a40`); nothing is drawn.
    Ps2Lighting { instance: u32 },
    /// 4 (`SpecularMapParamsManager_AddSpecularEvent` `0x006c56f0`, from
    /// the specular set-ups `0x006c5b80`.. `0x006c7090`): the specular pass
    /// of the slot kept for the character instance `instance`
    /// (`renderCorrespondSpecular`); live, it names the same instance as
    /// the kind 3 events of the frame.
    Specular { instance: u32 },
    /// 5 (`graphicVFXAddDrawCharacterEvent` `0x00644ca0`): the character
    /// instance drawn now, shown for the draw (opaque, transparent and
    /// post-transparent passes; `clip`: the `TransparentClipY` pass twice
    /// instead), between the effects; the scene ends after it.
    DrawCharacter { instance: u32, clip: bool },
    /// 6 (`graphicVFXAddDrawZPrePassEvent` `0x00644cf0`): the character
    /// instance's meshes drawn into the depth only (`ZPrePass`).
    ZPrePass { instance: u32 },
    /// Another kind: the game does nothing for it.
    Other,
}

impl Event {
    /// The words decoded by kind.
    #[must_use]
    pub fn info(&self) -> EventInfo {
        let [w0, w1] = self.words;
        let byte = |k: u32| (w0 >> (8 * k)) & 0xff;
        match self.kind {
            0 => EventInfo::Characters {
                layer: w0.cast_signed(),
            },
            1 => EventInfo::CopyBuffer {
                from: match byte(0) {
                    1 => Buffer::Copy,
                    2 => Buffer::CrossFade,
                    _ => Buffer::Colour,
                },
                to: match byte(1) {
                    0 => Buffer::Colour,
                    2 => Buffer::CrossFade,
                    _ => Buffer::Copy,
                },
                rgb: w0 >> 16 != 0,
                clear: w1 != 0,
            },
            2 => EventInfo::ScreenFade {
                colour: w0.to_le_bytes().map(|b| f32::from(b) / 128.0),
                blend: match w1 {
                    2 => FadeBlend::Additive,
                    0x10 => FadeBlend::Subtract,
                    _ => FadeBlend::Alpha,
                },
            },
            3 => EventInfo::Ps2Lighting { instance: w0 },
            4 => EventInfo::Specular { instance: w0 },
            5 => EventInfo::DrawCharacter {
                instance: w0,
                clip: w1 & 0xff != 0,
            },
            6 => EventInfo::ZPrePass { instance: w0 },
            _ => EventInfo::Other,
        }
    }
}

/// The screen distortion as `Distortion_renderDistortion` (`0x006c2dc0`)
/// will draw it this frame (docs/formats/postfx.md, "Distortion"): each
/// frame the colour is blended into a buffer kept across frames
/// (`buffer = colour * a + buffer * (1 - a)`, `a` = [`Distortion::copy_alpha`];
/// with [`Distortion::reset`] a plain copy, `buffer = colour`), then the
/// grid, sampling that buffer, replaces the colour: per pixel
/// `buffer(uv) * colour` (no blend; the grid covers the screen).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Distortion {
    /// `+0x7d`: the pass is skipped this frame (the game's update said so,
    /// `graphicSetDistortionSkipped`); the next frame then resets.
    pub skipped: bool,
    /// `+0x7e`: this frame's copy is a plain one (just switched on, or after
    /// a skipped frame): the buffer starts again from the colour.
    pub reset: bool,
    /// `+0x78`, as `graphicSetDistortionBlur` stores it (0.25 when switched
    /// on; the field's ripple sets `+0xbe0a0 / 128`).
    pub blur: f32,
    /// The grid's 81 vertices in normalised screen space: (0, 0) top left,
    /// (1, 1) bottom right (from `+0x68`, clip space `(x, y)`:
    /// `((x + 1) / 2, (1 - y) / 2)`).
    pub positions: Vec<[f32; 2]>,
    /// Where each vertex samples the buffer, in the same space (from
    /// `+0x70` `(u, v)`: `(u, 1 - v)`, what `phyredistortionshader.fx`
    /// samples with `UvScaleBias` (1, 1, 0, 0)). Undistorted, the same as
    /// the position.
    pub uvs: Vec<[f32; 2]>,
    /// Vertex colours (R, G, B, A) from `+0x6c`, multiplying the sample (1.0
    /// is the PS2's `0x80`: the field's tint bytes / 128).
    pub colours: Vec<[f32; 4]>,
    /// Triangle-list indices (from `+0x74`, `u32`s in the game).
    pub indices: Vec<u16>,
}

impl Distortion {
    /// The copy's alpha, `min(blur, 0.8)` (`ColorScale.a`).
    #[must_use]
    pub fn copy_alpha(&self) -> f32 {
        if self.blur >= COPY_ALPHA_MAX {
            COPY_ALPHA_MAX
        } else {
            self.blur
        }
    }
}

/// One `DrawMostVFX` call.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    /// The updates done when it drew: the [`ffx_state::Snapshot::update`]
    /// of the state it shows.
    pub update: u32,
    /// The draws, in draw order: the main pass's, then the other passes'
    /// in frame order ([`Pass`]).
    pub draws: Vec<Draw>,
    /// The main pass's draw events, in the order they run.
    pub events: Vec<Event>,
    /// List entries (of every list) whose mesh instance is in no geometry
    /// of the table (the game draws them; not captured).
    pub unresolved: u32,
    /// The screen distortion while it is on (`+0x7c`); `None` when it is
    /// off or not set up. The game also leaves it out while the broken
    /// screen effect draws (not read here).
    pub distortion: Option<Distortion>,
}

/// The capture's switch and its newest frames (`Shared::vfx`).
#[derive(Debug, Default)]
pub struct Capture {
    on: AtomicBool,
    frames: Mutex<Vec<Arc<Frame>>>,
}

impl Capture {
    /// Switches the capture on or off (off: the frames kept are dropped).
    pub fn set_enabled(&self, on: bool) {
        self.on.store(on, Ordering::Release);
        if !on {
            self.frames
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clear();
        }
    }

    /// Whether the capture is on.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.on.load(Ordering::Acquire)
    }

    /// The newest frame.
    #[must_use]
    pub fn latest(&self) -> Option<Arc<Frame>> {
        let f = self.frames.lock().unwrap_or_else(PoisonError::into_inner);
        f.last().cloned()
    }

    /// The frame of update `update` (a snapshot's), among the last few.
    #[must_use]
    pub fn frame(&self, update: u32) -> Option<Arc<Frame>> {
        let f = self.frames.lock().unwrap_or_else(PoisonError::into_inner);
        f.iter().rev().find(|x| x.update == update).cloned()
    }

    /// Keeps `frame` as the newest.
    pub fn push(&self, frame: Frame) {
        let mut f = self.frames.lock().unwrap_or_else(PoisonError::into_inner);
        // switched off while this frame was read: keep nothing (set_enabled
        // cleared the list under the same lock)
        if !self.enabled() {
            return;
        }
        if f.len() >= KEPT {
            f.remove(0);
        }
        f.push(Arc::new(frame));
    }
}

/// Registers the wrapper on `PhyreScene_DrawMostVFX`.
pub fn register(t: &mut Table) {
    // push ebp; mov ebp, esp; sub esp, 0x14
    t.wrap(
        DRAW_MOST_VFX,
        Entry {
            name: "PhyreScene_DrawMostVFX",
            conv: Conv::Thiscall,
            words: 0,
            port: Port::Exec(draw_most_vfx),
        },
        &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x14],
    );
}

/// `PhyreScene_DrawMostVFX(scene)`: with the capture on, the frame is read
/// first; then the original (its port) draws.
fn draw_most_vfx(e: &mut Exec, args: &Args) -> Ret {
    let shared = e.shared();
    if shared.vfx.enabled() {
        let frame = read_frame(&Reader(shared.arena()), args.ecx, shared.clock.frame());
        shared.vfx.push(frame);
    }
    e.call_original(DRAW_MOST_VFX, Conv::Thiscall, args)
        .unwrap_or(Ret::eax(0))
}

/// Game memory, read-only; a read of unmapped memory fails.
pub struct Reader<'a>(pub &'a Arena);

impl Reader<'_> {
    pub(crate) fn bytes(&self, at: u32, n: usize) -> Option<Vec<u8>> {
        if at == 0 || !self.0.is_mapped(at, n) {
            return None;
        }
        let mut v = vec![0; n];
        self.0.read_bytes(at, &mut v);
        Some(v)
    }

    pub(crate) fn u32(&self, at: u32) -> Option<u32> {
        let b = self.bytes(at, 4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub(crate) fn u8(&self, at: u32) -> Option<u8> {
        self.bytes(at, 1).map(|b| b[0])
    }

    pub(crate) fn ptr(&self, at: u32) -> Option<u32> {
        self.u32(at).filter(|&p| p != 0)
    }

    /// A NUL-terminated string at `at` (at most 256 bytes).
    pub(crate) fn c_str(&self, at: u32) -> Option<String> {
        let mut s = Vec::new();
        for i in 0..256 {
            let b = self.u8(at.wrapping_add(i))?;
            if b == 0 {
                return Some(String::from_utf8_lossy(&s).into_owned());
            }
            s.push(b);
        }
        None
    }

    /// `n` groups of `N` floats at `at` (none when unreadable).
    pub(crate) fn f32s<const N: usize>(&self, at: u32, n: u32) -> Vec<[f32; N]> {
        let Some(b) = self.bytes(at, 4 * N * n as usize) else {
            return Vec::new();
        };
        b.chunks_exact(4 * N)
            .map(|c| {
                std::array::from_fn(|k| {
                    f32::from_le_bytes([c[4 * k], c[4 * k + 1], c[4 * k + 2], c[4 * k + 3]])
                })
            })
            .collect()
    }

    pub(crate) fn u16s(&self, at: u32, n: u32) -> Vec<u16> {
        self.bytes(at, 2 * n as usize)
            .map(|b| {
                b.as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_le_bytes(*c))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// A geometry of the table: its key, address, whether pooled.
#[derive(Debug, Clone, Copy)]
struct Found {
    key: u64,
    geometry: u32,
    pooled: bool,
}

/// The table's geometries by mesh instance (`+0xc`): the map (an
/// in-order walk of the MSVC tree: `+0` left, `+8` right, `+0xd` is-nil,
/// `+0x10` key, `+0x18` geometry), then the delete pool (count `+8`, bit 31
/// a flag; keys `+0xc`, geometries `+0x1c`).
fn geometries(m: &Reader) -> HashMap<u32, Found> {
    let mut out = HashMap::new();
    let Some(table) = m.ptr(TABLE) else {
        return out;
    };
    let mut add = |key: u64, g: u32, pooled: bool| {
        if let Some(mi) = m.ptr(g.wrapping_add(0xc)) {
            out.entry(mi).or_insert(Found {
                key,
                geometry: g,
                pooled,
            });
        }
    };
    if let Some(head) = m.ptr(table) {
        let mut stack = Vec::new();
        let mut node = m.u32(head.wrapping_add(4)).unwrap_or(head);
        let mut guard = 0;
        loop {
            while node != head
                && node != 0
                && m.u8(node.wrapping_add(0xd)) == Some(0)
                && guard < 100_000
            {
                guard += 1;
                stack.push(node);
                node = m.u32(node).unwrap_or(head);
            }
            let Some(n) = stack.pop() else { break };
            let lo = m.u32(n.wrapping_add(0x10)).unwrap_or(0);
            let hi = m.u32(n.wrapping_add(0x14)).unwrap_or(0);
            if let Some(g) = m.ptr(n.wrapping_add(0x18)) {
                add((u64::from(hi) << 32) | u64::from(lo), g, false);
            }
            node = m.u32(n.wrapping_add(8)).unwrap_or(head);
        }
    }
    let n = m.u32(table.wrapping_add(8)).unwrap_or(0) & 0x7fff_ffff;
    if let (Some(keys), Some(geoms)) = (
        m.ptr(table.wrapping_add(0xc)),
        m.ptr(table.wrapping_add(0x1c)),
    ) {
        for i in 0..n.min(0x1_0000) {
            let lo = m.u32(keys.wrapping_add(i.wrapping_mul(8))).unwrap_or(0);
            let hi = m
                .u32(keys.wrapping_add(i.wrapping_mul(8).wrapping_add(4)))
                .unwrap_or(0);
            if lo == 0 && hi == 0 {
                continue;
            }
            if let Some(g) = m.ptr(geoms.wrapping_add(i.wrapping_mul(4))) {
                add((u64::from(hi) << 32) | u64::from(lo), g, true);
            }
        }
    }
    out
}

/// `p` placed by a world matrix laid out as [`Draw::world`].
#[must_use]
pub fn to_render(world: &[f32; 12], local: [f32; 3]) -> [f32; 3] {
    let axis_x = [world[3], world[7], world[11]];
    let axis_y = [world[0], world[1], world[2]];
    let axis_z = [world[4], world[5], world[6]];
    std::array::from_fn(|k| {
        axis_x[k] * local[0] + axis_y[k] * local[1] + axis_z[k] * local[2] + world[8 + k]
    })
}

/// Where a captured uv ([`Draw::uvs`]) samples an upright image of the
/// draw's texture (top row first: what `ffx_phyre`'s decoders, its DDS
/// export and `ffx-import`'s PNGs hold): `(u, 1 - v)`.
///
/// The VFX shaders sample `TEXCOORD0` as the arrays hold it (no transform,
/// no constant, no second stream; docs/formats/effects-draw.md, "Texture
/// coordinates"), with D3D's convention (`v = 0` the first row in memory),
/// and the game uploads a texture's rows as the file stores them, bottom
/// row first. So `v` counts up from the image's bottom (file textures
/// only: not the DTT's `FrameBuffer` render target). A frontend that
/// uploads the stored rows instead (`Texture::pixels`, unflipped) uses the
/// uv as it is.
#[must_use]
pub fn upright_uv(uv: [f32; 2]) -> [f32; 2] {
    [uv[0], 1.0 - uv[1]]
}

/// The draws of one list entry: each segment with indices (`+8` > 0) of
/// its geometry's render data (`+0x24`: `+0x90` count, `+0x94` segments of
/// `0x6c` bytes), with the segment's own arrays (`+4` vertices, `+8`
/// indices, `+0xc` positions, `+0x14` colours, `+0x18` uvs, `+0x1c`
/// indices; `docs/formats/render.md`), placed by the mesh instance's world
/// matrix (`*(mi +4)`).
fn entry_draws(m: &Reader, pass: Pass, place: u32, mi: u32, found: Found, out: &mut Vec<Draw>) {
    let g = found.geometry;
    let flags = m.u32(g.wrapping_add(0x3c)).unwrap_or(0);
    let world = m
        .ptr(mi.wrapping_add(4))
        .and_then(|p| m.f32s::<12>(p, 1).first().copied())
        .unwrap_or(IDENTITY);
    let Some(data) = m.ptr(g.wrapping_add(0x24)) else {
        return;
    };
    let n = m
        .u32(data.wrapping_add(0x90))
        .map_or(0, |n| n.cast_signed().clamp(0, SEGMENTS_MAX));
    let Some(base) = m.ptr(data.wrapping_add(0x94)) else {
        return;
    };
    let materials = Materials::of(m, mi);
    for i in 0..n.cast_unsigned() {
        let s = base.wrapping_add(0x6c * i);
        let indices = m.u32(s.wrapping_add(8)).unwrap_or(0);
        if indices.cast_signed() <= 0 {
            continue;
        }
        let indices = indices.min(INDICES_MAX);
        let (technique, blends) = materials
            .as_ref()
            .and_then(|x| x.blends(m, i))
            .map_or((None, Vec::new()), |(t, b)| (Some(t), b));
        let sampler = materials.as_ref().and_then(|x| x.sampler(m, i));
        let vertices = m.u32(s.wrapping_add(4)).unwrap_or(0).min(VERTICES_MAX);
        let positions: Vec<[f32; 3]> = m
            .f32s::<3>(m.u32(s.wrapping_add(0xc)).unwrap_or(0), vertices)
            .iter()
            .map(|p| to_render(&world, *p))
            .collect();
        let n = positions.len();
        let mut colours = m.f32s::<4>(m.u32(s.wrapping_add(0x14)).unwrap_or(0), vertices);
        colours.resize(n, [0.0; 4]);
        let mut uvs = m.f32s::<2>(m.u32(s.wrapping_add(0x18)).unwrap_or(0), vertices);
        uvs.resize(n, [0.0; 2]);
        out.push(Draw {
            pass,
            place,
            key: found.key,
            geometry: g,
            flags,
            pooled: found.pooled,
            segment: u16::try_from(i).unwrap_or(u16::MAX),
            technique,
            blends,
            sampler,
            world,
            texture: m
                .ptr(s.wrapping_add(0x20))
                .and_then(|p| m.c_str(p))
                .unwrap_or_default(),
            positions,
            colours,
            uvs,
            indices: m.u16s(m.u32(s.wrapping_add(0x1c)).unwrap_or(0), indices),
        });
    }
}

/// A mesh instance's materials, as `PRendererBase::renderMeshInstance`
/// (`0x005b9a90`) reaches them: the mesh (`*mi`, segments of 0x6c bytes
/// at `+4`, count `+0`; segment `+0` its material's index), the
/// instance's materials (`+0x10`, a `PSharray`: count `+0` with bit 31 a
/// flag, the one element inline at `+4` or else a pointer there) and its
/// per-segment draw data (`+0x28`, 0x24 bytes each: `+0x18` the effect,
/// `+0x14` its context variant once picked).
struct Materials {
    mi: u32,
    segments: u32,
    count: u32,
    list: u32,
}

impl Materials {
    fn of(m: &Reader, mi: u32) -> Option<Self> {
        let mesh = m.ptr(mi)?;
        let contexts = m.ptr(mi.wrapping_add(0x10))?;
        let n = m.u32(contexts)? & 0x7fff_ffff;
        let list = match n {
            0 => return None,
            1 => contexts.wrapping_add(4),
            _ => m.ptr(contexts.wrapping_add(4))?,
        };
        Some(Self {
            mi,
            segments: m.ptr(mesh.wrapping_add(4))?,
            count: m.u32(mesh)? & 0x7fff_ffff,
            list,
        })
    }

    /// The technique and blends of segment `i`, the way `selectTechniquePass`
    /// (`0x005ba690`) picks its shader pass for [`PASS_TRANSPARENT`]
    /// (`ffx_prender::mesh`): the material (a `PMaterial`: `+0` the effect,
    /// `+8`/`+0xc` a pass remapping), the effect's technique for the pass
    /// (by the pass's id `+0xc` into the effect's table `+0x18`, `+0x14`
    /// long, else the technique entries `+0x10` (40 bytes, `+0xc` of them)
    /// whose first word is the pass), the context variant (the segment's
    /// cached one; a technique of one variant has only variant 0), the
    /// shader (the technique's `+8` plus 32 bytes a variant) and its
    /// shader passes (`+4` count, `+8` the first pass state, the others
    /// after it), each drawn with a blend state made from its
    /// `D3D11_BLEND_DESC`.
    fn blends(&self, m: &Reader, i: u32) -> Option<(Technique, Vec<Blend>)> {
        let material = self.material(m, i)?;
        let effect = m.ptr(material)?;
        let (pass, technique) = if m.u32(material.wrapping_add(8)) == Some(PASS_TRANSPARENT) {
            let to = m.u32(material.wrapping_add(0xc))?;
            let t = match to {
                PASS_TRANSPARENT => Technique::Transparent,
                PASS_OPTIMIZED_TRANS => Technique::OptimizedTrans,
                _ => Technique::Other,
            };
            (to, t)
        } else {
            (PASS_TRANSPARENT, Technique::Transparent)
        };
        if pass == 0 {
            return None;
        }
        let tech = technique_for(m, effect, pass)?;
        let data = m.u32(self.mi.wrapping_add(0x28))?.wrapping_add(0x24 * i);
        let variants = m.u32(tech.wrapping_add(4))? & 0x7fff_ffff;
        // the renderer's cached variant, else (an assumption: the game asks
        // `PEffect_findContextVariant`, which may answer -1 and draw nothing)
        // the only one of a one-variant technique, as every VFX technique seen
        let variant = if m.u32(data.wrapping_add(0x18)) == Some(effect) {
            m.u32(data.wrapping_add(0x14))?
        } else if variants == 1 {
            0
        } else {
            return None;
        };
        if variant >= variants {
            return None;
        }
        let shader = m
            .ptr(tech.wrapping_add(8))?
            .wrapping_add(variant.wrapping_shl(5));
        let n = m.u32(shader.wrapping_add(4))? & 0x7fff_ffff;
        if n == 0 || n > SHADER_PASSES_MAX {
            return None;
        }
        let first = m.ptr(shader.wrapping_add(8))?;
        let blends = (0..n)
            .map(|k| pass_blend(m, first.wrapping_add(SHADER_PASS_SIZE * k)))
            .collect::<Option<Vec<_>>>()?;
        Some((technique, blends))
    }
}

impl Materials {
    /// Segment `i`'s material (`list[segment +0]`).
    fn material(&self, m: &Reader, i: u32) -> Option<u32> {
        if i >= self.count {
            return None;
        }
        let index = m.u32(self.segments.wrapping_add(0x6c * i))?;
        if index >= 0x1_0000 {
            return None;
        }
        m.ptr(self.list.wrapping_add(4 * index))
    }

    /// The sampler of segment `i`'s texture: the tweakable definition
    /// [`TEXTURE_SAMPLER_STATE`] of the material's effect (`+0x20` count,
    /// `+0x24` 16-byte definitions: `+2` the type, 0x47 for a sampler state,
    /// `+4` the name with bit 0 a flag, `+8` the `u16` offset in the
    /// parameter buffer), whose slot in the material's parameter buffer
    /// (`+4`; `{0x47, 0, state, 0}`) holds the `PSamplerState` (`+8`);
    /// `PShaderProgramBindingD3D11::setTextureParameters` (`0x0059b3d0`)
    /// binds the state's D3D11 object (`+0x20`) to the pixel shader's `s0`.
    /// The VFX effects also declare `GlobalSamplerState`,
    /// `NormalMapSamplerState` and `ZBufferSamplerState`, which their pixel
    /// shaders do not sample the texture with. Then its description
    /// ([`sampler_desc`]).
    fn sampler(&self, m: &Reader, i: u32) -> Option<Sampler> {
        let material = self.material(m, i)?;
        let effect = m.ptr(material)?;
        let buffer = m.ptr(material.wrapping_add(4))?;
        let n = m.u32(effect.wrapping_add(0x20))? & 0x7fff_ffff;
        let defs = m.ptr(effect.wrapping_add(0x24))?;
        for k in 0..n.min(PARAMETERS_MAX) {
            let def = m.bytes(defs.wrapping_add(16 * k), 16)?;
            if def[2] != PARAM_SAMPLER_STATE {
                continue;
            }
            let name = u32::from_le_bytes([def[4], def[5], def[6], def[7]]);
            if m.c_str(name & !1).as_deref() != Some(TEXTURE_SAMPLER_STATE) {
                continue;
            }
            let slot = buffer.wrapping_add(u32::from(u16::from_le_bytes([def[8], def[9]])));
            return sampler_desc(m, m.ptr(slot.wrapping_add(8))?);
        }
        None
    }
}

/// A `PSamplerState`'s description as `PSamplerStateD3D11::create`
/// (`0x00595bf0`, `ffx_prender::h2_more9`) makes it: the filter by
/// `0x0058aa10` (min filter `+0`: 0 and 2 point, 1 and 3 linear, 4 point
/// with linear mips, 5 linear with linear mips; mag filter `+1` non-zero
/// linear; [`MAX_ANISOTROPY`] over 1 anisotropic; the comparison bit 6 of
/// `+0x1c` adds `0x80`), the wrap bytes `+2..+4` ([`Address::from_phyre`]),
/// the bias `+8`, the anisotropy `+0xc`, the border bytes `+0x10..+0x13`.
fn sampler_desc(m: &Reader, state: u32) -> Option<Sampler> {
    let b = m.bytes(state, 0x20)?;
    let word = |k: usize| u32::from_le_bytes([b[k], b[k + 1], b[k + 2], b[k + 3]]);
    let mag = 4 * u32::from(b[1] != 0);
    let mut filter = match b[0] {
        0 | 2 => mag,
        1 | 3 => 0x10 + mag,
        4 => 1 + mag,
        5 => 0x11 + mag,
        _ => 0,
    };
    let max = f32::from_bits(m.u32(MAX_ANISOTROPY).unwrap_or(0));
    if max > 1.0 {
        filter = 0x55;
    }
    if (word(0x1c) >> 6) & 1 != 0 {
        filter |= 0x80;
    }
    let aniso = if max > 1.0 {
        max
    } else {
        f32::from_bits(word(0xc))
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // truncation, as `_ftol2`
    let max_anisotropy = aniso as u32;
    Some(Sampler {
        filter,
        address: [2, 3, 4].map(|k| Address::from_phyre(b[k])),
        mip_lod_bias: f32::from_bits(word(8)),
        max_anisotropy,
        border: [b[0x10], b[0x11], b[0x12], b[0x13]],
    })
}

/// The blend of the shader pass state at `state` (its `D3D11_BLEND_DESC`:
/// alpha to coverage, independent blend, then render target 0's enable,
/// source, destination, operation, the alpha's three and the write mask,
/// a word each).
fn pass_blend(m: &Reader, state: u32) -> Option<Blend> {
    let d = m.bytes(state.wrapping_add(PASS_BLEND_DESC), 40)?;
    let word = |k: usize| u32::from_le_bytes([d[4 * k], d[4 * k + 1], d[4 * k + 2], d[4 * k + 3]]);
    let factor = |k: usize| BlendFactor::from_u8(u8::try_from(word(k)).ok()?);
    let op = |k: usize| BlendOp::from_u8(u8::try_from(word(k)).ok()?);
    Some(Blend {
        enabled: word(2) != 0,
        src: factor(3)?,
        dest: factor(4)?,
        op: op(5)?,
        src_alpha: factor(6)?,
        dest_alpha: factor(7)?,
        op_alpha: op(8)?,
        write_mask: d[36],
    })
}

/// The effect's technique for `pass` (`selectTechniquePass`): by the pass's
/// id, else by the pass itself (`0x004e71d0`).
fn technique_for(m: &Reader, effect: u32, pass: u32) -> Option<u32> {
    let id = m.u32(pass.wrapping_add(0xc))?;
    if id != u32::MAX && id < m.u32(effect.wrapping_add(0x14))? & 0x7fff_ffff {
        let table = m.ptr(effect.wrapping_add(0x18))?;
        if let Some(t) = m.ptr(table.wrapping_add(id.wrapping_mul(4))) {
            return Some(t);
        }
    }
    let n = m.u32(effect.wrapping_add(0xc))? & 0x7fff_ffff;
    let mut e = m.ptr(effect.wrapping_add(0x10))?;
    for _ in 0..n.min(0x1000) {
        if m.u32(e)? == pass {
            return Some(e);
        }
        e = e.wrapping_add(40);
    }
    None
}

/// The frame `DrawMostVFX` is about to draw for `scene`, in its order: the
/// list's entries `0..count` (and up to the last event's place, as the
/// events may extend it), each non-zero entry's segments; while magic
/// [`LAST_DRAWN_MAGIC`] plays, the entry of geometry [`LAST_DRAWN_KEY`]
/// last. Then the other lists' draws, pass by pass ([`other_lists`]).
#[must_use]
pub fn read_frame(m: &Reader, scene: u32, update: u32) -> Frame {
    let mut frame = Frame {
        update,
        ..Frame::default()
    };
    let count = m
        .u32(scene.wrapping_add(0xf8cc))
        .unwrap_or(0)
        .cast_signed()
        .max(0);
    let n_events = m
        .u32(scene.wrapping_add(0xfb54))
        .unwrap_or(0)
        .min(EVENTS_MAX);
    let mut end = count.cast_unsigned();
    if let Some(list) = m.ptr(scene.wrapping_add(0xfb5c)) {
        for i in 0..n_events {
            let at = list.wrapping_add(16 * i);
            let mut ev = Event {
                kind: m.u32(at).unwrap_or(u32::MAX),
                at: m.u32(at.wrapping_add(4)).unwrap_or(0),
                words: [
                    m.u32(at.wrapping_add(8)).unwrap_or(0),
                    m.u32(at.wrapping_add(12)).unwrap_or(0),
                ],
                actor: -1,
            };
            if matches!(ev.kind, 3..=6) {
                ev.actor = actor_of(m, ev.words[0]);
            }
            end = end.max(ev.at);
            frame.events.push(ev);
        }
    }
    let end = end.min(LIST_MAX);
    let table = geometries(m);
    let held = m.u32(MAGIC_ID) == Some(LAST_DRAWN_MAGIC);
    let mut last = Vec::new();
    for place in 0..end {
        let Some(mi) = m.ptr(scene.wrapping_add(0x38cc + 4 * place)) else {
            continue;
        };
        let Some(&found) = table.get(&mi) else {
            frame.unresolved += 1;
            continue;
        };
        if held && found.key == LAST_DRAWN_KEY && !found.pooled {
            entry_draws(m, Pass::Main, place, mi, found, &mut last);
        } else {
            entry_draws(m, Pass::Main, place, mi, found, &mut frame.draws);
        }
    }
    frame.draws.append(&mut last);
    other_lists(m, scene, &table, &mut frame);
    frame.distortion = read_distortion(m);
    frame
}

/// The `g_ChrArray` slot of the character whose instance is `instance`
/// (its `+0x18`, what `PhyreScene_vfxEventMeshPass_006673b0` passes to
/// `Ch_GetId`), -1 when it is no record.
fn actor_of(m: &Reader, instance: u32) -> i16 {
    let (Some(ch), Some(base)) = (m.ptr(instance.wrapping_add(0x18)), m.ptr(CHR_ARRAY)) else {
        return -1;
    };
    let count = m.u32(CHR_COUNT).unwrap_or(0).cast_signed().max(0);
    let Some(off) = ch.checked_sub(base) else {
        return -1;
    };
    if !off.is_multiple_of(CHR_SIZE) || off / CHR_SIZE >= count.cast_unsigned() {
        return -1;
    }
    i16::try_from(off / CHR_SIZE).unwrap_or(-1)
}

/// Whether `DrawUI2DVFX` draws this frame: the scene fades (`+0x10dcc`) or
/// a movie does (in the UI step, or on maps 0x163/0x164 the late step).
fn ui2d_on(m: &Reader, scene: u32) -> bool {
    m.u8(scene.wrapping_add(0x1_0dcc)).unwrap_or(0) != 0 || m.u32(MOVIE_FADING).unwrap_or(0) != 0
}

/// `Phyre_PRendering_NameIs(mesh +0x2c, name)`: the same pointer (bit 0
/// off), or both set and the same string.
fn name_is(m: &Reader, mi: u32, name: u32) -> bool {
    let own = m.u32(mi.wrapping_add(0x2c)).unwrap_or(0) & !1;
    if own == name {
        return true;
    }
    own != 0 && name != 0 && m.c_str(own).is_some_and(|a| m.c_str(name) == Some(a))
}

/// The other lists' draws, appended in frame order: each list's non-zero
/// entries `0..count` (a `PArray`: count, data pointer), each entry's
/// segments as the main list's. The DTT list only when the scene step runs
/// its post chain (menu capture state not 2); the UI 2D list only when its
/// pass draws ([`ui2d_on`]), without the entries of the skipped name, and
/// on maps 0x163/0x164 before the after-post list (the late step).
fn other_lists(m: &Reader, scene: u32, table: &HashMap<u32, Found>, frame: &mut Frame) {
    let skip = m.u32(UI2D_SKIP_NAME).unwrap_or(0) & !1;
    let dtt = (Pass::Dtt, 0x3854, 0x385c);
    let after = (Pass::AfterPost, 0x388c, 0x3888);
    let ui2d = (Pass::Ui2d, 0x3860, 0x3868);
    let undrawn = (Pass::Undrawn, 0x3878, 0x3880);
    let order = if m.u32(MAP_163_OR_164).unwrap_or(0) == 0 {
        [dtt, after, ui2d, undrawn]
    } else {
        [dtt, ui2d, after, undrawn]
    };
    let post_chain = m.u32(MENU_CAPTURE_STATE).unwrap_or(0) != 2;
    for (pass, count, data) in order {
        if pass == Pass::Ui2d && !ui2d_on(m, scene) || pass == Pass::Dtt && !post_chain {
            continue;
        }
        let n = m
            .u32(scene.wrapping_add(count))
            .unwrap_or(0)
            .cast_signed()
            .clamp(0, OTHER_MAX.cast_signed())
            .cast_unsigned();
        let Some(list) = m.ptr(scene.wrapping_add(data)) else {
            continue;
        };
        for place in 0..n {
            let Some(mi) = m.ptr(list.wrapping_add(4 * place)) else {
                continue;
            };
            if pass == Pass::Ui2d && name_is(m, mi, skip) {
                continue;
            }
            let Some(&found) = table.get(&mi) else {
                frame.unresolved += 1;
                continue;
            };
            entry_draws(m, pass, place, mi, found, &mut frame.draws);
        }
    }
}

/// The distortion's state (`Distortion_GetInstance`, [`DISTORTION`]) when
/// it is on (`+0x7c`) and its grid arrays are there (`+0x68` positions
/// `x, y, z`, `+0x6c` colours, `+0x70` uvs, `+0x74` `u32` indices;
/// `Distortion_setupDistortionGrid`).
#[must_use]
pub fn read_distortion(m: &Reader) -> Option<Distortion> {
    let d = m.ptr(DISTORTION)?;
    if m.u8(d.wrapping_add(0x7c))? == 0 {
        return None;
    }
    let n = u32::try_from(GRID_VERTICES).ok()?;
    let positions = m.f32s::<3>(m.ptr(d.wrapping_add(0x68))?, n);
    let colours = m.f32s::<4>(m.ptr(d.wrapping_add(0x6c))?, n);
    let uvs = m.f32s::<2>(m.ptr(d.wrapping_add(0x70))?, n);
    let at = m.ptr(d.wrapping_add(0x74))?;
    let indices = m
        .bytes(at, 4 * GRID_INDICES)?
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u16::try_from(u32::from_le_bytes(*c)).unwrap_or(u16::MAX))
        .collect::<Vec<_>>();
    if positions.len() != GRID_VERTICES
        || colours.len() != GRID_VERTICES
        || uvs.len() != GRID_VERTICES
        || indices.iter().any(|&i| usize::from(i) >= GRID_VERTICES)
    {
        return None;
    }
    let blur = m.bytes(d.wrapping_add(0x78), 4)?;
    Some(Distortion {
        skipped: m.u8(d.wrapping_add(0x7d))? != 0,
        reset: m.u8(d.wrapping_add(0x7e))? != 0,
        blur: f32::from_le_bytes([blur[0], blur[1], blur[2], blur[3]]),
        positions: positions
            .iter()
            .map(|p| [f32::midpoint(p[0], 1.0), (1.0 - p[1]) * 0.5])
            .collect(),
        uvs: uvs.iter().map(|t| [t[0], 1.0 - t[1]]).collect(),
        colours,
        indices,
    })
}

// ---- the encoding ----------------------------------------------------------

/// Appends little-endian values.
pub(crate) struct Out(pub(crate) Vec<u8>);

impl Out {
    pub(crate) fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub(crate) fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub(crate) fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
}

/// Reads little-endian values.
pub(crate) struct In<'a>(pub(crate) &'a [u8]);

impl In<'_> {
    pub(crate) fn take(&mut self, n: usize) -> Option<&[u8]> {
        if self.0.len() < n {
            return None;
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Some(a)
    }
    pub(crate) fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
    pub(crate) fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }
    pub(crate) fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    pub(crate) fn u64(&mut self) -> Option<u64> {
        let lo = self.u32()?;
        let hi = self.u32()?;
        Some((u64::from(hi) << 32) | u64::from(lo))
    }
    pub(crate) fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }
    pub(crate) fn f32s<const N: usize>(&mut self, n: usize) -> Option<Vec<[f32; N]>> {
        let mut v = Vec::with_capacity(n.min(self.0.len() / (4 * N)));
        for _ in 0..n {
            let mut a = [0.0; N];
            for x in &mut a {
                *x = self.f32()?;
            }
            v.push(a);
        }
        Some(v)
    }
}

impl Frame {
    /// The frame as bytes (docs/state-api.md, "Effect draws"): little
    /// endian; a header (`FXDR`, version, update, unresolved, event and
    /// draw counts), the events (with their actor slot), each draw (with
    /// its pass, technique and blends) and its arrays, then the distortion
    /// (flags, blur, counts, arrays; flags 0 and no arrays when off).
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut o = Out(Vec::new());
        o.0.extend_from_slice(&MAGIC);
        o.u16(VERSION);
        o.u16(0);
        o.u32(self.update);
        o.u32(self.unresolved);
        o.u32(u32::try_from(self.events.len()).unwrap_or(u32::MAX));
        o.u32(u32::try_from(self.draws.len()).unwrap_or(u32::MAX));
        for e in &self.events {
            o.u32(e.kind);
            o.u32(e.at);
            o.u32(e.words[0]);
            o.u32(e.words[1]);
            o.u16(e.actor.cast_unsigned());
            o.u16(0);
        }
        for d in &self.draws {
            o.u32(d.place);
            o.u64(d.key);
            o.u32(d.geometry);
            o.u32(d.flags);
            o.u16(d.segment);
            o.u8(u8::from(d.pooled));
            o.u8(d.pass as u8);
            put_blends(&mut o, d.technique, &d.blends);
            put_sampler(&mut o, d.sampler.as_ref());
            for &w in &d.world {
                o.f32(w);
            }
            let name = d.texture.as_bytes();
            let len = name.len().min(usize::from(u16::MAX));
            o.u16(u16::try_from(len).unwrap_or(u16::MAX));
            o.0.extend_from_slice(&name[..len]);
            let n = d.positions.len();
            o.u32(u32::try_from(n).unwrap_or(u32::MAX));
            o.u32(u32::try_from(d.indices.len()).unwrap_or(u32::MAX));
            for &v in d.positions.iter().flatten() {
                o.f32(v);
            }
            for k in 0..n {
                for &v in d.colours.get(k).unwrap_or(&[0.0; 4]) {
                    o.f32(v);
                }
            }
            for k in 0..n {
                for &v in d.uvs.get(k).unwrap_or(&[0.0; 2]) {
                    o.f32(v);
                }
            }
            for &i in &d.indices {
                o.u16(i);
            }
        }
        match &self.distortion {
            None => {
                o.u32(0);
                o.f32(0.0);
                o.u32(0);
                o.u32(0);
            }
            Some(d) => {
                o.u32(1 | u32::from(d.skipped) << 1 | u32::from(d.reset) << 2);
                o.f32(d.blur);
                let n = d.positions.len();
                o.u32(u32::try_from(n).unwrap_or(u32::MAX));
                o.u32(u32::try_from(d.indices.len()).unwrap_or(u32::MAX));
                for &v in d.positions.iter().flatten() {
                    o.f32(v);
                }
                for k in 0..n {
                    for &v in d.uvs.get(k).unwrap_or(&[0.0; 2]) {
                        o.f32(v);
                    }
                }
                for k in 0..n {
                    for &v in d.colours.get(k).unwrap_or(&[0.0; 4]) {
                        o.f32(v);
                    }
                }
                for &i in &d.indices {
                    o.u16(i);
                }
            }
        }
        o.0
    }

    /// A frame from [`Frame::encode`]'s bytes; `None` when they are not one
    /// (another magic or version, an unknown pass or blend value, cut
    /// short).
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut i = In(bytes);
        if i.take(4)? != MAGIC || i.u16()? != VERSION {
            return None;
        }
        i.u16()?;
        let update = i.u32()?;
        let unresolved = i.u32()?;
        let n_events = i.u32()? as usize;
        let n_draws = i.u32()? as usize;
        let mut events = Vec::with_capacity(n_events.min(EVENTS_MAX as usize));
        for _ in 0..n_events {
            let kind = i.u32()?;
            let at = i.u32()?;
            let words = [i.u32()?, i.u32()?];
            let actor = i.u16()?.cast_signed();
            i.u16()?;
            events.push(Event {
                kind,
                at,
                words,
                actor,
            });
        }
        let mut draws = Vec::with_capacity(n_draws.min(LIST_MAX as usize));
        for _ in 0..n_draws {
            draws.push(take_draw(&mut i)?);
        }
        let flags = i.u32()?;
        let blur = i.f32()?;
        let n = i.u32()? as usize;
        let m = i.u32()? as usize;
        let distortion = if flags & 1 == 0 {
            if n != 0 || m != 0 {
                return None;
            }
            None
        } else {
            let positions = i.f32s::<2>(n)?;
            let uvs = i.f32s::<2>(n)?;
            let colours = i.f32s::<4>(n)?;
            let mut indices = Vec::with_capacity(m.min(i.0.len() / 2));
            for _ in 0..m {
                indices.push(i.u16()?);
            }
            Some(Distortion {
                skipped: flags & 2 != 0,
                reset: flags & 4 != 0,
                blur,
                positions,
                uvs,
                colours,
                indices,
            })
        };
        Some(Self {
            update,
            draws,
            events,
            unresolved,
            distortion,
        })
    }
}

/// One draw of [`Frame::encode`]'s bytes back.
fn take_draw(i: &mut In) -> Option<Draw> {
    let place = i.u32()?;
    let key = i.u64()?;
    let geometry = i.u32()?;
    let flags = i.u32()?;
    let segment = i.u16()?;
    let pooled = i.u8()? != 0;
    let pass = Pass::from_u8(i.u8()?)?;
    let (technique, blends) = take_blends(i)?;
    let sampler = take_sampler(i)?;
    let mut world = [0.0; 12];
    for w in &mut world {
        *w = i.f32()?;
    }
    let len = usize::from(i.u16()?);
    let texture = String::from_utf8_lossy(i.take(len)?).into_owned();
    let n = i.u32()? as usize;
    let m = i.u32()? as usize;
    let positions = i.f32s::<3>(n)?;
    let colours = i.f32s::<4>(n)?;
    let uvs = i.f32s::<2>(n)?;
    let mut indices = Vec::with_capacity(m.min(i.0.len() / 2));
    for _ in 0..m {
        indices.push(i.u16()?);
    }
    Some(Draw {
        pass,
        place,
        key,
        geometry,
        flags,
        pooled,
        segment,
        technique,
        blends,
        sampler,
        world,
        texture,
        positions,
        colours,
        uvs,
        indices,
    })
}

/// A draw's sampler ([`Frame::encode`]): 0, or 1 and the address modes U,
/// V, W (a byte each), the filter, the bias (`f32`), the maximum
/// anisotropy and the border bytes.
fn put_sampler(o: &mut Out, s: Option<&Sampler>) {
    let Some(s) = s else {
        o.u8(0);
        return;
    };
    o.u8(1);
    for a in s.address {
        o.u8(a as u8);
    }
    o.u32(s.filter);
    o.f32(s.mip_lod_bias);
    o.u32(s.max_anisotropy);
    o.0.extend_from_slice(&s.border);
}

/// [`put_sampler`]'s bytes back (the outer `None` when malformed: another
/// flag, an address mode outside D3D11's).
#[allow(clippy::option_option)] // cut short or bad vs. no sampler
fn take_sampler(i: &mut In) -> Option<Option<Sampler>> {
    match i.u8()? {
        0 => return Some(None),
        1 => {}
        _ => return None,
    }
    let a = i.take(3)?;
    let address = [
        Address::from_u8(a[0])?,
        Address::from_u8(a[1])?,
        Address::from_u8(a[2])?,
    ];
    let filter = i.u32()?;
    let mip_lod_bias = i.f32()?;
    let max_anisotropy = i.u32()?;
    let b = i.take(4)?;
    Some(Some(Sampler {
        filter,
        address,
        mip_lod_bias,
        max_anisotropy,
        border: [b[0], b[1], b[2], b[3]],
    }))
}

/// A draw's technique and blends ([`Frame::encode`]): the technique (0
/// none), the pass count, eight bytes a pass.
fn put_blends(o: &mut Out, technique: Option<Technique>, blends: &[Blend]) {
    let Some(t) = technique else {
        o.0.extend_from_slice(&[0, 0]);
        return;
    };
    let n = blends.len().min(usize::from(u8::MAX));
    o.0.extend_from_slice(&[t as u8, u8::try_from(n).unwrap_or(u8::MAX)]);
    for b in &blends[..n] {
        o.0.extend_from_slice(&[
            u8::from(b.enabled),
            b.src as u8,
            b.dest as u8,
            b.op as u8,
            b.src_alpha as u8,
            b.dest_alpha as u8,
            b.op_alpha as u8,
            b.write_mask,
        ]);
    }
}

/// [`put_blends`]'s bytes back; `None` for a value outside the enums or
/// passes without a technique.
fn take_blends(i: &mut In) -> Option<(Option<Technique>, Vec<Blend>)> {
    let t = i.u8()?;
    let n = i.u8()?;
    let technique = match t {
        0 if n == 0 => None,
        0 => return None,
        t => Some(Technique::from_u8(t)?),
    };
    let mut blends = Vec::with_capacity(usize::from(n));
    for _ in 0..n {
        let b = i.take(8)?;
        blends.push(Blend {
            enabled: b[0] != 0,
            src: BlendFactor::from_u8(b[1])?,
            dest: BlendFactor::from_u8(b[2])?,
            op: BlendOp::from_u8(b[3])?,
            src_alpha: BlendFactor::from_u8(b[4])?,
            dest_alpha: BlendFactor::from_u8(b[5])?,
            op_alpha: BlendOp::from_u8(b[6])?,
            write_mask: b[7],
        });
    }
    Some((technique, blends))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (src alpha, one, add), alpha (one, zero, add), all channels.
    const ADDITIVE: Blend = Blend {
        enabled: true,
        src: BlendFactor::SrcAlpha,
        dest: BlendFactor::One,
        op: BlendOp::Add,
        src_alpha: BlendFactor::One,
        dest_alpha: BlendFactor::Zero,
        op_alpha: BlendOp::Add,
        write_mask: 15,
    };

    fn draw(place: u32) -> Draw {
        Draw {
            pass: Pass::Main,
            place,
            key: (0xd2_a318 << 32) | 6,
            geometry: 0x32c1_dbbc,
            flags: 0x8,
            pooled: place == 2,
            segment: 1,
            technique: Some(Technique::OptimizedTrans),
            blends: vec![ADDITIVE],
            sampler: Some(Sampler {
                filter: 0x15,
                address: [Address::Wrap, Address::Clamp, Address::MirrorOnce],
                mip_lod_bias: -0.25,
                max_anisotropy: 1,
                border: [0, 0x80, 0xff, 7],
            }),
            world: [0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.5, 309.7, 14.9, -476.9, 0.0],
            texture: "../../../FFX_Data/GameData/PS3Data/map/bsil/bsil00/fp/tex/D3D11/14336_19_0_0_128_128.dds.phyre".into(),
            positions: vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]],
            colours: vec![[0.5, 0.875, 1.0, 0.125]; 3],
            uvs: vec![[0.0, 1.0], [1.0, 1.0], [0.5, 0.0]],
            indices: vec![0, 1, 2],
        }
    }

    /// The save sphere's sparkle block at Besaid: `v` 0.4385..0.499 counts
    /// up from the upright image's bottom (the game data side of this is
    /// `tests/vfx_uv.rs`).
    #[test]
    fn upright_uv_counts_v_from_the_bottom() {
        assert_eq!(upright_uv([0.25, 0.0]), [0.25, 1.0]);
        assert_eq!(upright_uv([0.25, 1.0]), [0.25, 0.0]);
        assert_eq!(upright_uv([0.0039, 0.4385]), [0.0039, 1.0 - 0.4385]);
        assert_eq!(upright_uv(upright_uv([0.375, 0.625])), [0.375, 0.625]);
    }

    #[test]
    fn a_frame_encodes_and_decodes_unchanged() {
        let f = Frame {
            update: 155,
            draws: vec![
                draw(19),
                draw(2),
                Draw::default(),
                Draw {
                    pass: Pass::Ui2d,
                    ..draw(0)
                },
                Draw {
                    pass: Pass::Undrawn,
                    ..draw(1)
                },
                Draw {
                    technique: None,
                    blends: Vec::new(),
                    sampler: None,
                    ..draw(3)
                },
                Draw {
                    sampler: Some(Sampler {
                        filter: 0xd5,
                        address: [Address::Border, Address::Mirror, Address::Clamp],
                        mip_lod_bias: 0.0,
                        max_anisotropy: 16,
                        border: [1, 2, 3, 4],
                    }),
                    ..draw(5)
                },
                Draw {
                    technique: Some(Technique::Other),
                    blends: vec![
                        Blend {
                            enabled: false,
                            src: BlendFactor::InvSrc1Alpha,
                            dest: BlendFactor::BlendFactor,
                            op: BlendOp::Max,
                            src_alpha: BlendFactor::Zero,
                            dest_alpha: BlendFactor::DestColour,
                            op_alpha: BlendOp::RevSubtract,
                            write_mask: 0,
                        },
                        ADDITIVE,
                    ],
                    ..draw(4)
                },
            ],
            events: vec![
                Event {
                    kind: 2,
                    at: 1,
                    words: [3, 4],
                    actor: -1,
                },
                Event {
                    kind: 5,
                    at: 3,
                    words: [0x0300_1000, 1],
                    actor: 12,
                },
            ],
            unresolved: 1,
            distortion: None,
        };
        let bytes = f.encode();
        assert_eq!(&bytes[..4], b"FXDR");
        assert_eq!(Frame::decode(&bytes), Some(f));
    }

    #[test]
    fn the_layout_is_the_documented_one() {
        let f = Frame {
            update: 7,
            draws: vec![Draw {
                pass: Pass::AfterPost,
                ..draw(19)
            }],
            events: vec![Event {
                kind: 5,
                at: 9,
                words: [0x0a0b_0c0d, 1],
                actor: 3,
            }],
            ..Frame::default()
        };
        let b = f.encode();
        // header 24, event 20, draw head 4+8+4+4+2+1+1 = 24, technique and
        // pass count 2, one pass 8, sampler 20, world 48, name 2 + its
        // bytes, counts 8, arrays 3 * (12 + 16 + 8) + 3 * 2, no distortion 16
        let name = draw(19).texture.len();
        assert_eq!(
            b.len(),
            24 + 20 + 24 + 2 + 8 + 20 + 48 + 2 + name + 8 + 3 * 36 + 6 + 16
        );
        assert_eq!(&b[4..6], &[5, 0], "version 5");
        assert_eq!(&b[b.len() - 16..], &[0; 16]);
        let word = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        assert_eq!(word(8), 7);
        assert_eq!(word(16), 1, "events");
        assert_eq!(word(20), 1, "draws");
        // the event: kind, place, words, actor (i16), 0
        assert_eq!(
            (word(24), word(28), word(32), word(36)),
            (5, 9, 0x0a0b_0c0d, 1)
        );
        assert_eq!(&b[40..44], &[3, 0, 0, 0]);
        // the draw: place, the key low word first, geometry, flags, segment,
        // pooled, pass
        let d = 44;
        assert_eq!(word(d), 19);
        assert_eq!(&b[d + 4..d + 12], &[6, 0, 0, 0, 0x18, 0xa3, 0xd2, 0]);
        assert_eq!(word(d + 16), 0x8);
        assert_eq!(&b[d + 20..d + 24], &[1, 0, 0, 2]);
        // the technique, the pass count, each pass: enabled, src, dest, op,
        // alpha's three, the write mask (D3D11's numbers); the world after
        assert_eq!(&b[d + 24..d + 34], &[2, 1, 1, 5, 2, 1, 2, 1, 1, 15]);
        // the sampler: 1, the address modes U, V, W, the filter, the bias,
        // the maximum anisotropy, the border bytes; the world after
        assert_eq!(&b[d + 34..d + 38], &[1, 1, 3, 5]);
        assert_eq!(word(d + 38), 0x15);
        assert_eq!(&b[d + 42..d + 46], &(-0.25f32).to_le_bytes());
        assert_eq!(word(d + 46), 1);
        assert_eq!(&b[d + 50..d + 54], &[0, 0x80, 0xff, 7]);
        assert_eq!(&b[d + 54..d + 58], &0.5f32.to_le_bytes());
        // none read: 0, 0 and no passes
        let unknown = Frame {
            draws: vec![Draw {
                technique: None,
                blends: Vec::new(),
                sampler: None,
                ..draw(19)
            }],
            ..Frame::default()
        }
        .encode();
        assert_eq!(&unknown[24 + 24..24 + 27], &[0, 0, 0]);
        assert_eq!(&unknown[24 + 27..24 + 31], &0.5f32.to_le_bytes());
        // an event without an actor: -1
        let none = Frame {
            events: vec![Event::default()],
            ..Frame::default()
        }
        .encode();
        assert_eq!(&none[40..44], &[0xff, 0xff, 0, 0]);
    }

    #[test]
    fn the_events_decode_as_their_makers_write_them() {
        let ev = |kind, w0, w1| Event {
            kind,
            at: 0,
            words: [w0, w1],
            actor: -1,
        };
        // addChrDrawSituation: (i8) layer
        assert_eq!(
            ev(0, u32::MAX, 0).info(),
            EventInfo::Characters { layer: -1 }
        );
        // 0x006568a0(b0, b1, x, h16): b0 | b1 << 8 | h16 << 16, x;
        // graphicVFXAddCopyBufferEventWithClearRGB(1, 0) = (1, 0, 1, 1)
        assert_eq!(
            ev(1, 1 | 1 << 16, 1).info(),
            EventInfo::CopyBuffer {
                from: Buffer::Copy,
                to: Buffer::Colour,
                rgb: true,
                clear: true
            }
        );
        // graphicVFXAddCopyBufferEvent(0, 1): the colour into the copy
        assert_eq!(
            ev(1, 1 << 8, 0).info(),
            EventInfo::CopyBuffer {
                from: Buffer::Colour,
                to: Buffer::Copy,
                rgb: false,
                clear: false
            }
        );
        assert_eq!(
            ev(1, 2 | 2 << 8, 0).info(),
            EventInfo::CopyBuffer {
                from: Buffer::CrossFade,
                to: Buffer::CrossFade,
                rgb: false,
                clear: false
            }
        );
        // graphicVFXAddScreenFadeEvent(r, g, b, a, alpha): the bytes, then
        // calculateVFXSP_AlphaFunction's bits
        assert_eq!(
            ev(2, u32::from_le_bytes([0x80, 0x40, 0, 0xff]), 2).info(),
            EventInfo::ScreenFade {
                colour: [1.0, 0.5, 0.0, 255.0 / 128.0],
                blend: FadeBlend::Additive
            }
        );
        for (w, blend) in [
            (0x10, FadeBlend::Subtract),
            (0, FadeBlend::Alpha),
            (0x20, FadeBlend::Alpha),
        ] {
            assert!(
                matches!(ev(2, 0, w).info(), EventInfo::ScreenFade { blend: b, .. } if b == blend)
            );
        }
        assert_eq!(
            ev(3, 0x1234, 7).info(),
            EventInfo::Ps2Lighting { instance: 0x1234 }
        );
        assert_eq!(
            ev(4, 0x40, 0).info(),
            EventInfo::Specular { instance: 0x40 }
        );
        // the flag is a byte; the bytes above it are the maker's junk
        assert_eq!(
            ev(5, 0x40, 0xcdcd_cd01).info(),
            EventInfo::DrawCharacter {
                instance: 0x40,
                clip: true
            }
        );
        assert_eq!(
            ev(5, 0x40, 0xcdcd_cd00).info(),
            EventInfo::DrawCharacter {
                instance: 0x40,
                clip: false
            }
        );
        assert_eq!(
            ev(6, 0x40, 9).info(),
            EventInfo::ZPrePass { instance: 0x40 }
        );
        assert_eq!(ev(7, 0, 0).info(), EventInfo::Other);
    }

    /// A scene in a fresh arena for [`read_frame`]: the table's delete pool
    /// holds one geometry per mesh instance (a triangle of one segment, its
    /// flags the instance's low byte), the lists hold the given instances,
    /// `g_ChrArray` three records.
    struct Fixture {
        arena: Arena,
        scene: u32,
        next: u32,
        pool: Vec<u32>,
    }

    const HEAP: u32 = 0x0200_0000;
    const CHRS: u32 = 0x0300_0000;

    impl Fixture {
        fn new() -> Self {
            let arena = Arena::new().unwrap();
            for (at, name) in [
                (TABLE & !0xfff, "table global"),
                (UI2D_SKIP_NAME & !0xfff, "ui globals"),
                (MOVIE_FADING & !0xfff, "movie"),
                (CHR_ARRAY & !0xfff, "chr globals"),
                (PASS_TRANSPARENT & !0xfff, "render passes"),
            ] {
                arena.map(at, 0x1000, name).unwrap();
            }
            arena.map(HEAP, 0x10_0000, "heap").unwrap();
            arena.map(CHRS, 0x1_0000, "chrs").unwrap();
            let scene = HEAP;
            arena.write_bytes(CHR_ARRAY, &CHRS.to_le_bytes());
            arena.write_bytes(CHR_COUNT, &3u32.to_le_bytes());
            Self {
                arena,
                scene,
                next: HEAP + 0x2_0000,
                pool: Vec::new(),
            }
        }

        fn alloc(&mut self, n: u32) -> u32 {
            let at = self.next;
            self.next += n.next_multiple_of(16);
            at
        }

        fn put(&self, at: u32, w: u32) {
            self.arena.write_bytes(at, &w.to_le_bytes());
        }

        /// A mesh instance with its geometry in the table.
        fn instance(&mut self, flags: u32) -> u32 {
            let mi = self.alloc(0x40);
            let g = self.alloc(0x40);
            let data = self.alloc(0xa0);
            let seg = self.alloc(0x6c);
            let pos = self.alloc(36);
            let idx = self.alloc(8);
            let name = self.alloc(16);
            self.put(g + 0xc, mi);
            self.put(g + 0x24, data);
            self.put(g + 0x3c, flags);
            self.put(data + 0x90, 1);
            self.put(data + 0x94, seg);
            self.put(seg + 4, 3);
            self.put(seg + 8, 3);
            self.put(seg + 0xc, pos);
            self.put(seg + 0x1c, idx);
            self.put(seg + 0x20, name);
            self.arena.write_bytes(name, b"NoTexture\0");
            self.arena.write_bytes(idx, &[0, 0, 1, 0, 2, 0]);
            self.pool.push(g);
            mi
        }

        /// An effect with two techniques, by pass id: `Transparent` (id
        /// 0x86) with one shader of one pass whose blend is (one, one,
        /// add), and `OptimizedTrans` found by pointer (its id past the
        /// table) whose shader's pass blends (src alpha, one, add) and
        /// whose technique has `variants` shaders (each its own pass, the
        /// source factor 5 + k for variant k > 0).
        fn effect(&mut self, variants: u32) -> u32 {
            self.put(PASS_TRANSPARENT + 0xc, 0x86);
            self.put(PASS_OPTIMIZED_TRANS + 0xc, 0x200);
            let pass = |f: &mut Self, src: u32, dest: u32| {
                let state = f.alloc(0x254);
                let desc = [0, 0, 1, src, dest, 1, 2, 1, 1, 15];
                for (k, w) in desc.iter().enumerate() {
                    f.put(state + PASS_BLEND_DESC + 4 * u32::try_from(k).unwrap(), *w);
                }
                state
            };
            let shaders = |f: &mut Self, blends: &[(u32, u32)]| {
                let n = u32::try_from(blends.len()).unwrap();
                let at = f.alloc(32 * n);
                for (k, &(src, dest)) in blends.iter().enumerate() {
                    let k = u32::try_from(k).unwrap();
                    let state = pass(f, src, dest);
                    f.put(at + 32 * k + 4, 0x8000_0001);
                    f.put(at + 32 * k + 8, state);
                }
                at
            };
            let effect = self.alloc(0x40);
            let transparent = self.alloc(40);
            let s = shaders(self, &[(2, 2)]);
            self.put(transparent + 4, 0x8000_0001);
            self.put(transparent + 8, s);
            let table = self.alloc(0x88 * 4);
            self.put(table + 0x86 * 4, transparent);
            self.put(effect + 0x14, 0x87);
            self.put(effect + 0x18, table);
            // the technique entries searched by pointer: one before it
            let entries = self.alloc(80);
            self.put(entries, PASS_TRANSPARENT);
            self.put(entries + 40, PASS_OPTIMIZED_TRANS);
            let blends: Vec<(u32, u32)> = (0..variants)
                .map(|k| (if k == 0 { 5 } else { 5 + k }, 2))
                .collect();
            let s = shaders(self, &blends);
            self.put(entries + 40 + 4, 0x8000_0000 | variants);
            self.put(entries + 40 + 8, s);
            self.put(effect + 0xc, 2);
            self.put(effect + 0x10, entries);
            effect
        }

        /// Gives `mi` a mesh of `n` segments, each with its own material
        /// of `effect` (remapped to `remap`, 0 for none) and its draw data.
        fn materials(&mut self, mi: u32, n: u32, effect: u32, remap: &[u32]) {
            let mesh = self.alloc(8);
            let segs = self.alloc(0x6c * n);
            self.put(mi, mesh);
            self.put(mesh, n);
            self.put(mesh + 4, segs);
            let contexts = self.alloc(8);
            let list = if n == 1 {
                contexts + 4
            } else {
                let l = self.alloc(4 * n);
                self.put(contexts + 4, l);
                l
            };
            self.put(contexts, 0x8000_0000 | n);
            self.put(mi + 0x10, contexts);
            let data = self.alloc(0x24 * n);
            self.put(mi + 0x28, data);
            for i in 0..n {
                // segment i draws material n - 1 - i
                let index = n - 1 - i;
                self.put(segs + 0x6c * i, index);
                let material = self.alloc(0x18);
                self.put(material, effect);
                let to = remap[index as usize];
                if to != 0 {
                    self.put(material + 8, PASS_TRANSPARENT);
                    self.put(material + 0xc, to);
                }
                self.put(list + 4 * index, material);
            }
            // the data render data the capture reads: n segments
            let g = self.pool[self.pool.len() - 1];
            let rd = self.arena_u32(g + 0x24);
            let seg = self.arena_u32(rd + 0x94);
            let more = self.alloc(0x6c * n);
            let bytes = {
                let mut b = vec![0; 0x6c];
                self.arena.read_bytes(seg, &mut b);
                b
            };
            for i in 0..n {
                self.arena.write_bytes(more + 0x6c * i, &bytes);
            }
            self.put(rd + 0x90, n);
            self.put(rd + 0x94, more);
        }

        fn arena_u32(&self, at: u32) -> u32 {
            let mut b = [0; 4];
            self.arena.read_bytes(at, &mut b);
            u32::from_le_bytes(b)
        }

        /// Writes the table (the delete pool only, keys `(i + 1, 0)`).
        fn table(&mut self) {
            let n = u32::try_from(self.pool.len()).unwrap();
            let table = self.alloc(0x20);
            let head = self.alloc(0x20);
            let keys = self.alloc(8 * n);
            let geoms = self.alloc(4 * n);
            // an empty map: the head's +4 (root) is the head, +0xd is-nil
            self.put(table, head);
            self.put(head + 4, head);
            self.arena.write_bytes(head + 0xd, &[1]);
            self.put(table + 8, n);
            self.put(table + 0xc, keys);
            self.put(table + 0x1c, geoms);
            for (i, &g) in self.pool.iter().enumerate() {
                let i = u32::try_from(i).unwrap();
                self.put(keys + 8 * i, i + 1);
                self.put(geoms + 4 * i, g);
            }
            self.put(TABLE, table);
        }

        /// A `PArray` list: `count` at `+c`, its data pointer at `+d`.
        fn list(&mut self, c: u32, d: u32, entries: &[u32]) {
            let data = self.alloc(4 * u32::try_from(entries.len()).unwrap().max(1));
            for (i, &e) in entries.iter().enumerate() {
                self.put(data + 4 * u32::try_from(i).unwrap(), e);
            }
            self.put(self.scene + c, u32::try_from(entries.len()).unwrap());
            self.put(self.scene + d, data);
        }

        fn frame(&self) -> Frame {
            read_frame(&Reader(&self.arena), self.scene, 5)
        }
    }

    #[test]
    fn the_other_lists_are_read_in_frame_order_with_their_pass() {
        let mut f = Fixture::new();
        let main = f.instance(0x8);
        let dtt = f.instance(0xa);
        let after = f.instance(0x2);
        let ui = f.instance(0x18);
        let undrawn = f.instance(0x9);
        let lost = f.alloc(0x40);
        f.table();
        f.put(f.scene + 0x38cc, main);
        f.put(f.scene + 0xf8cc, 1);
        f.list(0x3854, 0x385c, &[0, dtt]);
        f.list(0x388c, 0x3888, &[after, lost]);
        f.list(0x3860, 0x3868, &[ui]);
        f.list(0x3878, 0x3880, &[undrawn]);
        // the name the UI 2D pass skips (the game always has one: with both
        // names null, NameIs holds and the entry is skipped)
        let skip = f.alloc(16);
        f.arena.write_bytes(skip, b"stencil\0");
        f.put(UI2D_SKIP_NAME, skip | 1);
        let got: Vec<(Pass, u32, u32)> = f
            .frame()
            .draws
            .iter()
            .map(|d| (d.pass, d.place, d.flags))
            .collect();
        // not fading: no UI 2D draws
        assert_eq!(
            got,
            [
                (Pass::Main, 0, 0x8),
                (Pass::Dtt, 1, 0xa),
                (Pass::AfterPost, 0, 0x2),
                (Pass::Undrawn, 0, 0x9),
            ]
        );
        assert_eq!(f.frame().unresolved, 1, "the instance in no geometry");
        let d = &f.frame().draws[1];
        assert_eq!(
            (d.key, d.indices.len(), d.texture.as_str()),
            (2, 3, "NoTexture")
        );
        // the scene fades: the UI 2D list is drawn
        f.arena.write_bytes(f.scene + 0x1_0dcc, &[1]);
        let passes: Vec<Pass> = f.frame().draws.iter().map(|d| d.pass).collect();
        assert_eq!(
            passes,
            [
                Pass::Main,
                Pass::Dtt,
                Pass::AfterPost,
                Pass::Ui2d,
                Pass::Undrawn
            ]
        );
        // on maps 0x163/0x164 too, in the late step before the after-post VFX
        f.put(MAP_163_OR_164, 1);
        let passes: Vec<Pass> = f.frame().draws.iter().map(|d| d.pass).collect();
        assert_eq!(
            passes,
            [
                Pass::Main,
                Pass::Dtt,
                Pass::Ui2d,
                Pass::AfterPost,
                Pass::Undrawn
            ]
        );
        f.put(MAP_163_OR_164, 0);
        // the menu capture (state 2) skips the post chain and the DTT VFX
        f.put(MENU_CAPTURE_STATE, 2);
        assert!(f.frame().draws.iter().all(|d| d.pass != Pass::Dtt));
        f.put(MENU_CAPTURE_STATE, 0);
        // a movie's fade alone is enough
        f.arena.write_bytes(f.scene + 0x1_0dcc, &[0]);
        f.put(MOVIE_FADING, 1);
        assert!(f.frame().draws.iter().any(|d| d.pass == Pass::Ui2d));
        // the skipped name: the same pointer, or the same string elsewhere
        f.put(ui + 0x2c, skip);
        assert!(f.frame().draws.iter().all(|d| d.pass != Pass::Ui2d));
        let a = f.alloc(16);
        f.arena.write_bytes(a, b"stencil\0");
        f.put(ui + 0x2c, a);
        assert!(f.frame().draws.iter().all(|d| d.pass != Pass::Ui2d));
        f.arena.write_bytes(a, b"another\0");
        assert!(f.frame().draws.iter().any(|d| d.pass == Pass::Ui2d));
    }

    #[test]
    fn the_sampler_is_the_material_s_texture_sampler_state() {
        let mut f = Fixture::new();
        let effect = f.effect(1);
        let mi = f.instance(0xa);
        f.materials(mi, 2, effect, &[0, 0]);
        // the effect's definitions: GlobalSamplerState (slot 0x60, empty)
        // first, then TextureSamplerState (slot 0x10), a texture between
        let names = f.alloc(64);
        f.arena.write_bytes(names, b"GlobalSamplerState\0");
        f.arena.write_bytes(names + 32, b"TextureSamplerState\0");
        let defs = f.alloc(48);
        for (k, (kind, name, offset)) in [
            (0x47u32, names, 0x60u32),
            (0x42, 0, 0x20),
            (0x47, names + 32, 0x10),
        ]
        .into_iter()
        .enumerate()
        {
            let d = defs + 16 * u32::try_from(k).unwrap();
            f.put(d, 0x3400_0000 | kind << 16);
            f.put(d + 4, name | 1); // bit 0: a flag
            f.put(d + 8, 0x0010_0000 | offset);
        }
        f.put(effect + 0x20, 0x8000_0003);
        f.put(effect + 0x24, defs);
        // segment 0 draws material 1, segment 1 material 0
        let contexts = f.arena_u32(mi + 0x10);
        let list = f.arena_u32(contexts + 4);
        for (index, wrap, min) in [(0u32, 1u8, 5u8), (1, 2, 1)] {
            let state = f.alloc(0x24);
            f.arena.write_bytes(state, &[min, 1, wrap, wrap, 4]);
            f.put(state + 8, (-0.5f32).to_bits());
            f.put(state + 0xc, 1.0f32.to_bits());
            f.arena.write_bytes(state + 0x10, &[1, 2, 3, 4]);
            let buffer = f.alloc(0xb0);
            f.put(buffer, 0xb0);
            f.put(buffer + 0x10, 0x47);
            f.put(buffer + 0x18, state);
            f.put(f.arena_u32(list + 4 * index) + 4, buffer);
        }
        f.table();
        f.put(f.scene + 0x38cc, mi);
        f.put(f.scene + 0xf8cc, 1);
        let fr = f.frame();
        let want = |address: [Address; 3], filter: u32| {
            Some(Sampler {
                filter,
                address,
                mip_lod_bias: -0.5,
                max_anisotropy: 1,
                border: [1, 2, 3, 4],
            })
        };
        let (clamp, wrap, mirror) = (Address::Clamp, Address::Wrap, Address::Mirror);
        let got: Vec<_> = fr.draws.iter().map(|d| d.sampler).collect();
        // the wrap bytes 1, 2 (clamp), 4 (mirror); the filters (min 1
        // linear, mag linear) 0x14, (min 5 linear with linear mips) 0x15
        assert_eq!(
            got,
            [
                want([clamp, clamp, mirror], 0x14),
                want([wrap, wrap, mirror], 0x15)
            ]
        );
        // the renderer's anisotropy over 1: anisotropic, its integer part
        f.arena
            .map(MAX_ANISOTROPY & !0xfff, 0x1000, "renderer globals")
            .unwrap();
        f.put(MAX_ANISOTROPY, 4.5f32.to_bits());
        let fr = f.frame();
        let s = fr.draws[0].sampler.unwrap();
        assert_eq!((s.filter, s.max_anisotropy), (0x55, 4));
    }

    #[test]
    fn the_blend_is_read_from_the_material_s_technique() {
        let mut f = Fixture::new();
        let effect = f.effect(1);
        let mi = f.instance(0xa);
        // segment 0 draws material 2 (not remapped), 1 material 1
        // (OptimizedTrans), 2 material 0 (remapped to Transparent itself)
        f.materials(mi, 3, effect, &[PASS_TRANSPARENT, PASS_OPTIMIZED_TRANS, 0]);
        f.table();
        f.put(f.scene + 0x38cc, mi);
        f.put(f.scene + 0xf8cc, 1);
        let fr = f.frame();
        let got: Vec<_> = fr
            .draws
            .iter()
            .map(|d| {
                let b = d.blends.first()?;
                Some((d.technique?, b.src, b.dest, b.op, d.blends.len()))
            })
            .collect();
        let (one, alpha, add) = (BlendFactor::One, BlendFactor::SrcAlpha, BlendOp::Add);
        assert_eq!(
            got,
            [
                Some((Technique::Transparent, one, one, add, 1)),
                Some((Technique::OptimizedTrans, alpha, one, add, 1)),
                Some((Technique::Transparent, one, one, add, 1)),
            ]
        );
        let b = fr.draws[1].blends[0];
        assert_eq!(
            (
                b.enabled,
                b.src_alpha,
                b.dest_alpha,
                b.op_alpha,
                b.write_mask
            ),
            (true, BlendFactor::One, BlendFactor::Zero, BlendOp::Add, 15)
        );
    }

    #[test]
    fn a_blend_that_cannot_be_read_is_none() {
        let mut f = Fixture::new();
        // two variants, none picked yet: the renderer's choice is unknown
        let effect = f.effect(2);
        let mi = f.instance(0x8);
        f.materials(mi, 1, effect, &[PASS_OPTIMIZED_TRANS]);
        let bare = f.instance(0x8);
        f.table();
        f.put(f.scene + 0x38cc, mi);
        f.put(f.scene + 0x38d0, bare);
        f.put(f.scene + 0xf8cc, 2);
        let got: Vec<_> = f
            .frame()
            .draws
            .iter()
            .map(|d| (d.technique, d.blends.len()))
            .collect();
        assert_eq!(got, [(None, 0), (None, 0)], "no variant; no materials");
        // the segment's cached variant 1: its pass
        let data = f.arena_u32(mi + 0x28);
        f.put(data + 0x18, effect);
        f.put(data + 0x14, 1);
        let d = &f.frame().draws[0];
        assert_eq!(
            (d.technique, d.blends[0].src),
            (Some(Technique::OptimizedTrans), BlendFactor::InvSrcAlpha)
        );
        // a factor outside D3D11's
        f.put(data + 0x14, 0);
        let entries = f.arena_u32(effect + 0x10);
        let shader = f.arena_u32(entries + 48);
        let state = f.arena_u32(shader + 8);
        f.put(state + PASS_BLEND_DESC + 12, 13);
        assert_eq!(f.frame().draws[0].technique, None);
        f.put(state + PASS_BLEND_DESC + 12, 5);
        assert!(f.frame().draws[0].technique.is_some());
        // two shader passes (as the DTT shader's): both, in order; the
        // second (one, src alpha, subtract)
        f.put(shader + 4, 0x8000_0002);
        let second = state + SHADER_PASS_SIZE + PASS_BLEND_DESC;
        for (k, w) in [0, 0, 1, 2, 5, 2, 2, 1, 1, 15].iter().enumerate() {
            f.put(second + 4 * u32::try_from(k).unwrap(), *w);
        }
        let d = &f.frame().draws[0];
        assert_eq!(
            d.blends
                .iter()
                .map(|b| (b.src, b.dest, b.op))
                .collect::<Vec<_>>(),
            [
                (BlendFactor::SrcAlpha, BlendFactor::One, BlendOp::Add),
                (BlendFactor::One, BlendFactor::SrcAlpha, BlendOp::Subtract),
            ]
        );
        f.put(shader + 4, 0x8000_0001);
        // remapped to another pass the effect has no technique for
        let other = f.alloc(16);
        f.put(other + 0xc, u32::MAX);
        let contexts = f.arena_u32(mi + 0x10);
        let material = f.arena_u32(contexts + 4);
        f.put(material + 0xc, other);
        assert_eq!(f.frame().draws[0].technique, None);
    }

    #[test]
    fn character_events_name_the_actor_slot() {
        let mut f = Fixture::new();
        f.table();
        // character instances: +0x18 the record (slot 2, slot 5 past the
        // count, and an address between records)
        let inst = [f.alloc(0x20), f.alloc(0x20), f.alloc(0x20)];
        f.put(inst[0] + 0x18, CHRS + 2 * CHR_SIZE);
        f.put(inst[1] + 0x18, CHRS + 5 * CHR_SIZE);
        f.put(inst[2] + 0x18, CHRS + 0x10);
        let events = f.alloc(16 * 6);
        for (k, (kind, w0)) in [
            (5, inst[0]),
            (6, inst[0]),
            (4, inst[0]),
            (3, inst[1]),
            (5, inst[2]),
            (0, inst[0]),
        ]
        .into_iter()
        .enumerate()
        {
            let at = events + 16 * u32::try_from(k).unwrap();
            f.put(at, kind);
            f.put(at + 4, 0);
            f.put(at + 8, w0);
        }
        f.put(f.scene + 0xfb54, 6);
        f.put(f.scene + 0xfb5c, events);
        let actors: Vec<i16> = f.frame().events.iter().map(|e| e.actor).collect();
        assert_eq!(actors, [2, 2, 2, -1, -1, -1]);
    }

    /// The game's grid (`Distortion_setupDistortionGrid`) at `base` in a
    /// fresh arena: `+0x68` positions, `+0x6c` colours, `+0x70` uvs,
    /// `+0x74` indices, then `updateDistortionVertex(row, col, ..., u, v)`
    /// for each vertex with `f(row, col)`'s `(u, v)` and colour.
    fn game_grid(
        on: u8,
        flags: [u8; 2],
        blur: f32,
        vertex: &dyn Fn(u32, u32) -> ([f32; 2], [f32; 4]),
    ) -> Arena {
        let arena = Arena::new().unwrap();
        let base = 0x0100_0000;
        arena.map(base, 0x1_0000, "distortion").unwrap();
        arena.map(DISTORTION & !0xfff, 0x1000, "global").unwrap();
        let (pos, col, uv, idx) = (base + 0x1000, base + 0x2000, base + 0x3000, base + 0x4000);
        arena.write_bytes(DISTORTION, &base.to_le_bytes());
        for (off, p) in [(0x68, pos), (0x6c, col), (0x70, uv), (0x74, idx)] {
            arena.write_bytes(base + off, &p.to_le_bytes());
        }
        arena.write_bytes(base + 0x78, &blur.to_le_bytes());
        arena.write_bytes(base + 0x7c, &[on, flags[0], flags[1], 1]);
        let put = |at: u32, v: &[f32]| {
            for (k, x) in v.iter().enumerate() {
                arena.write_bytes(at + 4 * u32::try_from(k).unwrap(), &x.to_le_bytes());
            }
        };
        // 0x006c2ca0, 0x006c2bb0: vertex 9 i + j at (-1 + i/4, -1 + j/4, 1),
        // uv (i/8, j/8); 0x006c28e0: colours 1
        #[allow(clippy::cast_precision_loss)]
        for i in 0..9u32 {
            for j in 0..9u32 {
                let k = 9 * i + j;
                put(
                    pos + 12 * k,
                    &[i as f32 * 0.25 - 1.0, j as f32 * 0.25 - 1.0, 1.0],
                );
                put(uv + 8 * k, &[i as f32 * 0.125, j as f32 * 0.125]);
                put(col + 16 * k, &[1.0; 4]);
            }
        }
        // 0x006c2990: quad t = 9 r + c, (t, t+10, t+1), (t, t+9, t+10)
        let mut n = 0;
        for r in 0..8u32 {
            for c in 0..8u32 {
                let t = 9 * r + c;
                for x in [t, t + 10, t + 1, t, t + 9, t + 10] {
                    arena.write_bytes(idx + 4 * n, &x.to_le_bytes());
                    n += 1;
                }
            }
        }
        // 0x006c3240: k = 9 row - col + 8; colour, then uv (u, 1 - v)
        for row in 0..9u32 {
            for c in 0..9u32 {
                let ([u, v], rgba) = vertex(row, c);
                let k = 9 * row + 8 - c;
                put(col + 16 * k, &rgba);
                put(uv + 8 * k, &[u, 1.0 - v]);
            }
        }
        arena
    }

    #[allow(clippy::cast_precision_loss)]
    fn undistorted(row: u32, col: u32) -> ([f32; 2], [f32; 4]) {
        ([row as f32 / 8.0, col as f32 / 8.0], [1.0; 4])
    }

    #[test]
    fn the_distortion_grid_reads_in_screen_space() {
        let a = game_grid(1, [0, 1], 0.25, &undistorted);
        let d = read_distortion(&Reader(&a)).expect("on");
        assert!(!d.skipped && d.reset);
        assert!((d.blur - 0.25).abs() < f32::EPSILON);
        assert_eq!(d.positions.len(), GRID_VERTICES);
        assert_eq!(d.indices.len(), GRID_INDICES);
        // the undistorted ripple samples where it draws
        assert_eq!(d.uvs, d.positions);
        // the field's (row, col) is screen (row / 8, col / 8), y down:
        // vertex 9 row + 8 - col
        assert_eq!(d.positions[8], [0.0, 0.0]);
        assert_eq!(d.positions[0], [0.0, 1.0]);
        assert_eq!(d.positions[80], [1.0, 0.0]);
        assert_eq!(d.positions[9 * 3 + 8 - 5], [0.375, 0.625]);
        assert_eq!(&d.indices[..6], &[0, 10, 1, 0, 9, 10]);
    }

    #[test]
    fn a_rippled_vertex_keeps_its_place_and_moves_its_sample() {
        #[allow(clippy::cast_precision_loss)]
        let a = game_grid(1, [1, 0], 1.0, &|row, col| {
            let ([u, v], _) = undistorted(row, col);
            if (row, col) == (4, 2) {
                ([u + 0.03, v - 0.02], [0.5, 0.75, 1.0, 1.0])
            } else {
                ([u, v], [1.0; 4])
            }
        });
        let d = read_distortion(&Reader(&a)).expect("on");
        assert!(d.skipped && !d.reset);
        assert!((d.copy_alpha() - COPY_ALPHA_MAX).abs() < f32::EPSILON);
        let k = 9 * 4 + 8 - 2;
        assert_eq!(d.positions[k], [0.5, 0.25]);
        assert!((d.uvs[k][0] - 0.53).abs() < 1e-6 && (d.uvs[k][1] - 0.23).abs() < 1e-6);
        assert_eq!(d.colours[k], [0.5, 0.75, 1.0, 1.0]);
    }

    #[test]
    fn the_distortion_is_none_while_off() {
        let a = game_grid(0, [0, 0], 0.5, &undistorted);
        assert_eq!(read_distortion(&Reader(&a)), None);
    }

    #[test]
    fn a_frame_with_the_distortion_encodes_and_decodes_unchanged() {
        let a = game_grid(1, [0, 1], 0.5, &undistorted);
        let f = Frame {
            update: 3,
            draws: vec![draw(1)],
            distortion: read_distortion(&Reader(&a)),
            ..Frame::default()
        };
        let b = f.encode();
        assert_eq!(Frame::decode(&b), Some(f.clone()));
        // the distortion: flags, blur, counts 16, 81 * (8 + 8 + 16), 384 * 2
        let tail = 16 + GRID_VERTICES * 32 + GRID_INDICES * 2;
        let none = Frame {
            distortion: None,
            ..f
        };
        assert_eq!(b.len(), none.encode().len() - 16 + tail);
        let t = &b[b.len() - tail..];
        assert_eq!(u32::from_le_bytes([t[0], t[1], t[2], t[3]]), 1 | 4);
        assert_eq!(f32::from_le_bytes([t[4], t[5], t[6], t[7]]), 0.5);
        assert_eq!(u32::from_le_bytes([t[8], t[9], t[10], t[11]]), 81);
        assert_eq!(u32::from_le_bytes([t[12], t[13], t[14], t[15]]), 0x180);
        // the first position, (0, 1)
        assert_eq!(f32::from_le_bytes([t[20], t[21], t[22], t[23]]), 1.0);
    }

    #[test]
    fn a_cut_or_foreign_blob_is_refused() {
        let b = Frame {
            draws: vec![draw(0)],
            ..Frame::default()
        }
        .encode();
        assert_eq!(Frame::decode(&b[..b.len() - 1]), None);
        for v in [3, 4] {
            let mut other = b.clone();
            other[4] = v;
            assert_eq!(Frame::decode(&other), None, "version {v}");
        }
        let mut other = b.clone();
        // the draw's pass byte (header 24, draw head byte 23)
        other[24 + 23] = 5;
        assert_eq!(Frame::decode(&other), None, "an unknown pass");
        // the blend's bytes (draw head byte 24 on: technique, count, the
        // pass): a technique, factor or operation outside the enums, passes
        // without a technique
        for (at, v) in [(24, 4), (24, 0), (27, 12), (29, 6), (31, 0), (32, 9)] {
            let mut other = b.clone();
            other[24 + at] = v;
            assert_eq!(Frame::decode(&other), None, "blend byte {at} = {v}");
        }
        // the sampler's (draw head byte 34 on, after one pass): the flag,
        // an address mode outside D3D11's
        for (at, v) in [(34, 2), (35, 0), (36, 6), (37, 0)] {
            let mut other = b.clone();
            other[24 + at] = v;
            assert_eq!(Frame::decode(&other), None, "sampler byte {at} = {v}");
        }
        let mut other = b.clone();
        let n = other.len();
        other[n - 8] = 1;
        assert_eq!(Frame::decode(&other), None, "counts without the flag");
        assert_eq!(Frame::decode(b"FXD"), None);
    }

    #[test]
    fn the_world_matrix_places_as_the_camera_layout() {
        // the field-to-render flip of Besaid's sea (X = -x, Y = -y, Z = z)
        // and a lift of 1.8 down
        let w = [
            0.0, -1.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.8, 0.0, 0.0,
        ];
        assert_eq!(to_render(&w, [10.0, 1.0, -20.0]), [-10.0, -2.8, -20.0]);
        assert_eq!(to_render(&IDENTITY, [1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn frames_are_kept_by_update_and_dropped_when_off() {
        let c = Capture::default();
        assert!(!c.enabled());
        c.set_enabled(true);
        for u in 0..6 {
            c.push(Frame {
                update: u,
                ..Frame::default()
            });
        }
        assert_eq!(c.latest().map(|f| f.update), Some(5));
        assert_eq!(c.frame(2).map(|f| f.update), Some(2));
        assert_eq!(c.frame(1), None);
        c.set_enabled(false);
        assert_eq!(c.latest(), None);
    }
}
