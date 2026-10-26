//! The render-side glue, eighteenth batch (M4 port round 5, list
//! `h3-glue-render`): the scene's pre-render target clears and the shadow
//! manager's buffers. The exception frame and cookie are left out, as in
//! [`crate::glue10`].

use ffx_bridge::Global;

/// `Phyre_PRendering_PRenderSurface_PRenderSurface(this, texture, 0)`,
/// its destructor `0x005b3640(this)`,
/// `Phyre_PRendering_PRenderer_setColorTarget(this, surface, 0)`,
/// `beginScene(this, 1)`, `endScene(this, 1)`, `ShadowManager_getInstance`
/// and the shadow pass `0x006ce130(this, renderer)`.
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
pub const SURFACE_DTOR: u32 = 0x005b_3640;
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
pub const END_SCENE: u32 = 0x005b_6950;
pub const SHADOW_INSTANCE: u32 = 0x006d_2390;
pub const SHADOW_PRE_RENDER: u32 = 0x006c_e130;
/// The four post-process targets cleared, with their surfaces' frame
/// offsets (below `ebp`).
pub const PRE_RENDER_TARGETS: [(u32, u32); 4] =
    [(0x8c, 0x4c), (0x1_232c, 0x3c), (0xa8, 0x2c), (0xa4, 0x1c)];

/// The shadow buffers: `Phyre_PCluster_create(this, 1, class, 0)`,
/// `PTexture2DBase_setDimensions(this, w, h, format, 0)`,
/// `updateMaxLevel(this)`, the render target's maker `0x0064adf0(this,
/// texture)`, `g_ShadowSize`, the texture class and format, and the
/// targets' names.
pub const CLUSTER_CREATE: u32 = crate::glue9::CLUSTER_CREATE;
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
pub const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
pub const MAKE_TARGET: u32 = 0x0064_adf0;
pub const SHADOW_SIZE: Global<u32> = Global::new(0x00c3_4408);
pub const TEXTURE_CLASS: u32 = 0x00ca_37d0;
pub const TEXTURE_FORMAT: u32 = 0x00ca_310c;
/// Per buffer: the texture's field, the target's field and its name.
pub const SHADOW_BUFFERS: [(u32, u32, u32); 4] = [
    (0x74, 0x6c, 0x00b4_98c8),
    (0x78, 0x70, 0x00b4_98c8),
    (0x7c, 0x60, 0x00b4_98ec),
    (0x80, 0x64, 0x00b4_9910),
];
pub const DUMMY_NAME: u32 = 0x00b4_9934;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// `PhyreScene_preRender(this)` (frame: four render surfaces at
    /// `ebp - 0x4c`, `- 0x3c`, `- 0x2c`, `- 0x1c`): each of the
    /// post-process manager's (`+0x10cc`) targets `+0x8c`, `+0x1232c`,
    /// `+0xa8`, `+0xa4` in turn gets a surface, is bound as the colour
    /// target and has an empty scene begun and ended on the renderer
    /// (`+0xd0`); after all four the shadow manager pre-renders. The
    /// first error stops it; the surfaces made are destroyed in reverse.
    /// `eax`: the error, or 0.
    pub fn scene_pre_render(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let mut made = vec![];
        let mut error = 0;
        for (target, at) in PRE_RENDER_TARGETS {
            let post: u32 = h.read(this + 0x10cc);
            let texture: u32 = h.read(post.wrapping_add(target));
            let surface = ebp - at;
            h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[texture, 0]);
            made.push(surface);
            let renderer: u32 = h.read(this + 0xd0);
            error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[surface, 0]);
            if error != 0 {
                break;
            }
            let renderer: u32 = h.read(this + 0xd0);
            error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[1]);
            if error != 0 {
                break;
            }
            let renderer: u32 = h.read(this + 0xd0);
            error = h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
            if error != 0 {
                break;
            }
        }
        if error == 0 {
            let renderer: u32 = h.read(this + 0xd0);
            let shadows = h.call_sym(SHADOW_INSTANCE, Conv::Cdecl, &[]);
            h.call_sym(SHADOW_PRE_RENDER, Conv::Thiscall(shadows), &[renderer]);
        }
        for surface in made.into_iter().rev() {
            h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
        }
        error
    }

    /// A shadow texture in the manager's cluster (`+4`): made, sized
    /// `size` x `size`, its flag 2 (`+0x14`) cleared and its levels
    /// updated. `Err` with `setDimensions`' error, `Ok(None)` when the
    /// cluster gives none.
    fn shadow_texture(
        h: &mut impl Host,
        this: u32,
        size: u32,
        field: Option<u32>,
    ) -> Result<Option<u32>, u32> {
        let class = h.rebase(TEXTURE_CLASS);
        let texture = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(this + 4), &[1, class, 0]);
        if let Some(field) = field {
            h.write(this + field, texture);
        }
        if texture == 0 {
            return Ok(None);
        }
        let format = h.rebase(TEXTURE_FORMAT);
        let error = h.call_sym(
            SET_DIMENSIONS,
            Conv::Thiscall(texture),
            &[size, size, format, 0],
        );
        if error != 0 {
            return Err(error);
        }
        let texture = match field {
            Some(field) => h.read(this + field),
            None => texture,
        };
        let flags = h.read::<u32>(texture + 0x14) & !2;
        h.write(texture + 0x14, flags);
        h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(texture), &[]);
        Ok(Some(texture))
    }

    /// `ShadowManager_createshadowbuffer(this, a, b)` (`ret 8`, the two
    /// words unused): four `g_ShadowSize` textures (`+0x74`, `+0x78`,
    /// `+0x7c`, `+0x80`) with their named render targets (`+0x6c`,
    /// `+0x70`, `+0x60`, `+0x64`), then an 8 x 8 dummy (`+0x68`); the
    /// two current maps (`+0x58`, `+0x5c`) become the second advanced one.
    /// `eax`: 0, 0xd when a texture or target is missing, or
    /// `setDimensions`' error.
    pub fn shadow_create_buffers(h: &mut impl Host, this: u32) -> u32 {
        for (texture_at, target_at, name) in SHADOW_BUFFERS {
            let size = SHADOW_SIZE.get(h);
            match shadow_texture(h, this, size, Some(texture_at)) {
                Err(error) => return error,
                Ok(None) => return 0xd,
                Ok(Some(_)) => {}
            }
            let texture: u32 = h.read(this + texture_at);
            let target = h.call_sym(MAKE_TARGET, Conv::Thiscall(this + 4), &[texture]);
            h.write(this + target_at, target);
            if target == 0 {
                return 0xd;
            }
            let name = h.rebase(name);
            h.write(target + 0xc, name);
        }
        let texture = match shadow_texture(h, this, 8, None) {
            Err(error) => return error,
            Ok(None) => return 0xd,
            Ok(Some(t)) => t,
        };
        let target = h.call_sym(MAKE_TARGET, Conv::Thiscall(this + 4), &[texture]);
        h.write(this + 0x68, target);
        if target == 0 {
            return 0xd;
        }
        let name = h.rebase(DUMMY_NAME);
        h.write(target + 0xc, name);
        let second: u32 = h.read(this + 0x64);
        h.write(this + 0x5c, second);
        h.write(this + 0x58, second);
        0
    }
}
