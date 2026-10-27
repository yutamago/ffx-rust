//! The render-side glue, fifty-first batch (M4 port round 14, list
//! `h3-glue-render`): the scene's VFX draw events. The exception frame
//! and cookie are left out, as in [`crate::glue10`].

/// `PRenderSurface`'s constructor (`this; texture, word`, `ret 8`) and
/// destructor (a bare `ret`).
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
pub const SURFACE_DTOR: u32 = 0x005b_3640;
/// The renderer's `setCamera(renderer; camera)`, `endScene(renderer;
/// word)` and `beginScene(renderer; flags)` (`ret 4` each).
pub const SET_CAMERA: u32 = 0x005b_abd0;
pub const END_SCENE: u32 = 0x005b_6950;
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
/// `FUN_006673b0(scene; character instance, flag)` (`ret 8`): draws a
/// character instance.
pub const DRAW_CHARACTER: u32 = 0x0066_73b0;
/// `Phyre_PCamera_updateViewMatrices(camera)`.
pub const UPDATE_VIEW_MATRICES: u32 = 0x0067_20c0;
/// `PhyreScene_isMotionBlurOn_006601e0(scene)` (`al`).
pub const IS_MOTION_BLUR_ON: u32 = 0x0066_01e0;
/// `PostProcessManager_copyDepthBuffer(post; 9 words)` (`ret 0x24`),
/// its image data argument, and `copyBuffer(post; 6 words)` (`ret
/// 0x18`).
pub const COPY_DEPTH_BUFFER: u32 = 0x0069_2e90;
pub const DEPTH_COPY_DATA: u32 = 0x00cc_c8ec;
pub const COPY_BUFFER: u32 = 0x0069_2c40;
/// `PhyreScene_setColor4_00638840(post; &colour)` (`ret 4`) and
/// `PostProcessManager_renderScreenFade(post; renderer, camera, &surface,
/// 0)` (`ret 0x10`).
pub const SET_COLOR4: u32 = 0x0063_8840;
pub const RENDER_SCREEN_FADE: u32 = 0x0069_7490;
/// `PS2LightingSetup_apply_006e5a40(mesh)` (cdecl).
pub const PS2_LIGHTING_APPLY: u32 = 0x006e_5a40;
/// `SpecularMapParamsManager_GetInstance()` (cdecl) and
/// `renderCorrespondSpecular(manager; what, renderer)` (`ret 8`).
pub const SPECULAR_INSTANCE: u32 = 0x006c_57e0;
pub const RENDER_CORRESPOND_SPECULAR: u32 = 0x006c_59c0;
/// `graphicCharacterInstanceSetVisible(instance, visible)` (cdecl).
pub const SET_VISIBLE: u32 = 0x0063_c390;
/// The renderer's `setColorTarget(renderer; &surface, 0)` (`ret 8`),
/// `setDepthStencilTarget(renderer; &surface)` (`ret 4`),
/// `setViewport(renderer; x, y, w, h)` (`ret 0x10`) and
/// `setClearColor(renderer; r, g, b, a)` (`ret 0x10`).
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
pub const SET_DEPTH_STENCIL_TARGET: u32 = 0x005b_b5b0;
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const SET_CLEAR_COLOR: u32 = 0x005b_ada0;
/// `PostProcessManager_degradeResolution(post; renderer)` (`ret 4`).
pub const DEGRADE_RESOLUTION: u32 = 0x0069_3530;
/// `renderMeshInstance(renderer; instance, mesh, 0, context)` (`ret
/// 0x10`) and the pass name pointer the Z pre-pass sets (`+0x48`).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
pub const ZPREPASS_NAME: u32 = 0x00c1_6a74;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// A texture slot's texture: none when its byte `+4` is set.
    fn texture(h: &impl Host, slot: u32) -> u32 {
        if h.read::<u8>(slot.wrapping_add(4)) == 0 {
            h.read(slot)
        } else {
            0
        }
    }

    /// The scene camera's `+4`, or 0 without one.
    fn camera(h: &impl Host, scene: u32) -> u32 {
        let camera: u32 = h.read(scene + 0xdc);
        if camera == 0 { 0 } else { camera + 4 }
    }

    /// `PhyreScene_processVFXDrawEvent(scene; event)` (thiscall, `ret 4`;
    /// frame from `ebp - 0xa8`) -> 0 or the first failing renderer
    /// call's code. The post-process targets `+0x8c` and `+0x90` go into
    /// surfaces at `ebp - 0x50` and `ebp - 0xa8`; by the event's kind:
    /// - 0: the camera's matrix (`+0x34`, 16 words) saved at `ebp -
    ///   0x90`, the camera set; each VFX character (`+0xfb50`, count
    ///   `+0xfb48`) of the event's layer (`+0x15`, sign-extended, against
    ///   `+8`) and shown (`+0x14`) drawn, the matrix put back through the
    ///   x87 and the view matrices updated; the scene ended;
    /// - 1 (unless motion blur is on and byte `+9` is 1): a post target
    ///   chosen by byte `+8` and one by byte `+9` (`ebp - 0x14`), the
    ///   scene ended, the first copied into the second (with depth when
    ///   word `+0xa` is set, its colour 1.0 at `ebp - 0x30`); a set `+0xc`
    ///   clears the next scene;
    /// - 2: the scene ended, the bytes `+8..+0xb` / 128 as the post
    ///   colour (`ebp - 0x40`; `+0xc` the fade word unless `+0x123a0`),
    ///   the screen fade drawn;
    /// - 3: the PS2 lighting of `+8`; 4: its specular maps;
    /// - 5: the character `+8` drawn visible (flag `+0xc`), the scene
    ///   ended, hidden again if it was;
    /// - 6: the mesh instances of the array `+8` drawn in the Z pre-pass.
    ///
    /// Kinds 0, 1, 2 and 5 then set the targets back, the viewport to the
    /// colour target's size and begin the scene again.
    #[allow(clippy::too_many_lines)] // the original's switch
    pub fn process_vfx_draw_event(h: &mut impl Host, ebp: u32, scene: u32, event: u32) -> u32 {
        let colour = ebp - 0x50;
        let depth = ebp - 0xa8;
        let post: u32 = h.read(scene + 0x10cc);
        let target: u32 = h.read(post.wrapping_add(0x8c));
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(colour), &[target, 0]);
        let post: u32 = h.read(scene + 0x10cc);
        let target: u32 = h.read(post.wrapping_add(0x90));
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(depth), &[target, 0]);
        h.write(ebp - 0xd, 0u8);
        let kind: u32 = h.read(event);
        let result = match kind {
            0 => {
                let camera: u32 = h.read(scene + 0xdc);
                for k in 0..16 {
                    let w: u32 = h.read(camera.wrapping_add(0x34 + 4 * k));
                    h.write(ebp - 0x90 + 4 * k, w);
                }
                let renderer: u32 = h.read(scene + 0xd0);
                let cam = if camera == 0 { 0 } else { camera + 4 };
                h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[cam]);
                let mut k = 0u32;
                while k < h.read::<u32>(scene + 0xfb48) {
                    let list: u32 = h.read(scene + 0xfb50);
                    let chr: u32 = h.read(list.wrapping_add(4 * k));
                    if chr != 0
                        && i32::from(h.read::<u8>(chr + 0x15) as i8)
                            == h.read::<u32>(event + 8) as i32
                        && h.read::<u8>(chr + 0x14) != 0
                    {
                        h.call_sym(DRAW_CHARACTER, Conv::Thiscall(scene), &[chr, 0]);
                        let cam = h.read::<u32>(scene + 0xdc).wrapping_add(4);
                        for j in 0..16 {
                            let w: u32 = h.read(ebp - 0x90 + 4 * j);
                            h.write(cam.wrapping_add(0x30 + 4 * j), x87::store(x87::load(w)));
                        }
                        h.call_sym(UPDATE_VIEW_MATRICES, Conv::Thiscall(cam), &[]);
                    }
                    k += 1;
                }
                let renderer: u32 = h.read(scene + 0xd0);
                match h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]) {
                    0 => restore(h, ebp, scene),
                    error => error,
                }
            }
            1 => blur_copy(h, ebp, scene, event),
            2 => {
                let renderer: u32 = h.read(scene + 0xd0);
                match h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]) {
                    0 => {
                        fade(h, ebp, scene, event);
                        restore(h, ebp, scene)
                    }
                    error => error,
                }
            }
            3 => {
                let mesh: u32 = h.read(event + 8);
                h.call_sym(PS2_LIGHTING_APPLY, Conv::Cdecl, &[mesh]);
                0
            }
            4 => {
                let renderer: u32 = h.read(scene + 0xd0);
                let what: u32 = h.read(event + 8);
                let manager = h.call_sym(SPECULAR_INSTANCE, Conv::Cdecl, &[]);
                h.call_sym(
                    RENDER_CORRESPOND_SPECULAR,
                    Conv::Thiscall(manager),
                    &[what, renderer],
                );
                0
            }
            5 => {
                let chr: u32 = h.read(event + 8);
                let shown = h.read::<u8>(chr.wrapping_add(0x14)) != 0;
                h.write(ebp - 0x18, chr);
                h.call_sym(SET_VISIBLE, Conv::Cdecl, &[chr, 1]);
                let flag = u32::from(h.read::<u8>(event + 0xc) != 0);
                h.call_sym(DRAW_CHARACTER, Conv::Thiscall(scene), &[chr, flag]);
                let renderer: u32 = h.read(scene + 0xd0);
                match h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]) {
                    0 => {
                        if !shown {
                            h.call_sym(SET_VISIBLE, Conv::Cdecl, &[chr, 0]);
                        }
                        restore(h, ebp, scene)
                    }
                    error => error,
                }
            }
            6 => {
                let array: u32 = h.read(event + 8);
                let renderer: u32 = h.read(scene + 0xd0);
                let name = h.rebase(ZPREPASS_NAME);
                h.write(renderer.wrapping_add(0x48), name);
                let mut k = 0u32;
                while k < h.read::<u32>(array) & 0x7fff_ffff {
                    let renderer: u32 = h.read(scene + 0xd0);
                    let data: u32 = h.read(array.wrapping_add(4));
                    let instance: u32 = h.read(data.wrapping_add(4 * k));
                    let context: u32 = h.read(renderer.wrapping_add(0x13c));
                    let mesh: u32 = h.read(instance.wrapping_add(0x10));
                    h.call_sym(
                        RENDER_MESH_INSTANCE,
                        Conv::Thiscall(renderer),
                        &[instance, mesh, 0, context],
                    );
                    k += 1;
                }
                0
            }
            _ => 0,
        };
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(depth), &[]);
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(colour), &[]);
        result
    }

    /// Kind 1: unless motion blur is on and byte `+9` is 1, the post
    /// target of byte `+8` (0 or other: `+0x8c`, 1: `+0xa8`, 2:
    /// `+0x1232c`) copied into the one of byte `+9` (0: `+0x8c`, 2:
    /// `+0x1232c`, other: `+0xa8`; kept at `ebp - 0x14`) after the scene
    /// ends, then [`restore`].
    fn blur_copy(h: &mut impl Host, ebp: u32, scene: u32, event: u32) -> u32 {
        let blur = h.call_sym(IS_MOTION_BLUR_ON, Conv::Thiscall(scene), &[]) & 0xff;
        if blur != 0 && h.read::<u8>(event + 9) == 1 {
            return 0;
        }
        let post: u32 = h.read(scene + 0x10cc);
        let from = match h.read::<u8>(event + 8) {
            1 => 0xa8,
            2 => 0x1_232c,
            _ => 0x8c,
        };
        let from: u32 = h.read(post.wrapping_add(from));
        let to = match h.read::<u8>(event + 9) {
            0 => 0x8c,
            2 => 0x1_232c,
            _ => 0xa8,
        };
        let to: u32 = h.read(post.wrapping_add(to));
        h.write(ebp - 0x14, to);
        let renderer: u32 = h.read(scene + 0xd0);
        let error = h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
        if error != 0 {
            return error;
        }
        if h.read::<u16>(event + 0xa) == 0 {
            let source = texture(h, from);
            let post: u32 = h.read(scene + 0x10cc);
            let renderer: u32 = h.read(scene + 0xd0);
            h.call_sym(
                COPY_BUFFER,
                Conv::Thiscall(post),
                &[renderer, source, to, 1, 0, 0],
            );
        } else {
            for k in 0..4 {
                h.write(ebp - 0x30 + 4 * k, 1.0f32.to_bits());
            }
            let source = texture(h, from);
            let post: u32 = h.read(scene + 0x10cc);
            let renderer: u32 = h.read(scene + 0xd0);
            let data = h.rebase(DEPTH_COPY_DATA);
            h.call_sym(
                COPY_DEPTH_BUFFER,
                Conv::Thiscall(post),
                &[renderer, source, to, data, ebp - 0x30, 1, 1, 1, 0],
            );
        }
        if h.read::<u32>(event + 0xc) != 0 {
            h.write(ebp - 0xd, 1u8);
        }
        restore(h, ebp, scene)
    }

    /// Kind 2's colour: the bytes `+8..+0xb` times 1/128 (`ebp - 0x14`,
    /// `- 0x20`, `- 0x18`, the last in the argument slot), the original's
    /// conversion temporaries (`ebp - 0x1c`, `ebp - 0x98`) included, then
    /// at `ebp - 0x40` handed to the post-process manager; the fade word
    /// `+0xc` taken unless `+0x123a0`; the screen fade drawn.
    fn fade(h: &mut impl Host, ebp: u32, scene: u32, event: u32) {
        let scale = |b: u8| x87::store(f64::from(b) * 0.007_812_5);
        let c: [u8; 4] = std::array::from_fn(|k| h.read(event + 8 + k as u32));
        // each byte goes through a double at ebp - 0x1c (the last at
        // ebp - 0x98); the third's high word is overwritten below
        h.write(ebp - 0x1c, f64::from(c[2]).to_bits());
        h.write(ebp - 0x14, scale(c[0]));
        h.write(ebp - 0x20, scale(c[1]));
        h.write(ebp - 0x18, scale(c[2]));
        h.write(ebp - 0x98, f64::from(c[3]).to_bits());
        for (k, &b) in c.iter().enumerate() {
            h.write(ebp - 0x40 + 4 * k as u32, scale(b));
        }
        let post: u32 = h.read(scene + 0x10cc);
        let word: u32 = h.read(event + 0xc);
        if h.read::<u8>(post.wrapping_add(0x1_23a0)) == 0 {
            h.write(post.wrapping_add(0x1_239c), word);
        }
        let post: u32 = h.read(scene + 0x10cc);
        h.call_sym(SET_COLOR4, Conv::Thiscall(post), &[ebp - 0x40]);
        let camera = camera(h, scene);
        let post: u32 = h.read(scene + 0x10cc);
        let renderer: u32 = h.read(scene + 0xd0);
        h.call_sym(
            RENDER_SCREEN_FADE,
            Conv::Thiscall(post),
            &[renderer, camera, ebp - 0x50, 0],
        );
    }

    /// The targets set back (colour `ebp - 0x50`, depth `ebp - 0xa8`), the
    /// viewport to the colour texture's size (`+0x1c`, `+0x20`), the
    /// resolution degraded and the scene begun (cleared to 0 first when
    /// the byte at `ebp - 0xd` is set) -> 0 or the first error.
    fn restore(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let renderer: u32 = h.read(scene + 0xd0);
        let error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[ebp - 0x50, 0]);
        if error != 0 {
            return error;
        }
        let renderer: u32 = h.read(scene + 0xd0);
        let error = h.call_sym(
            SET_DEPTH_STENCIL_TARGET,
            Conv::Thiscall(renderer),
            &[ebp - 0xa8],
        );
        if error != 0 {
            return error;
        }
        let slot: u32 = h.read(ebp - 0x50);
        let texture = texture(h, slot);
        let height: u32 = h.read(texture.wrapping_add(0x20));
        let width: u32 = h.read(texture.wrapping_add(0x1c));
        let renderer: u32 = h.read(scene + 0xd0);
        h.call_sym(
            SET_VIEWPORT,
            Conv::Thiscall(renderer),
            &[0, 0, width, height],
        );
        let post: u32 = h.read(scene + 0x10cc);
        let renderer: u32 = h.read(scene + 0xd0);
        h.call_sym(DEGRADE_RESOLUTION, Conv::Thiscall(post), &[renderer]);
        let renderer: u32 = h.read(scene + 0xd0);
        if h.read::<u8>(ebp - 0xd) == 0 {
            h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0])
        } else {
            h.call_sym(SET_CLEAR_COLOR, Conv::Thiscall(renderer), &[0, 0, 0, 0]);
            let renderer: u32 = h.read(scene + 0xd0);
            h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0xb])
        }
    }
}
