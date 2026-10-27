//! `PhyreScene_render`'s 3D scene (`0x00668ed3..0x0066a733`, M4 list
//! `h3-glue-render`, agent h3e): shadows, the scene into the colour and
//! normal targets with the camera lifted, the HDAO and `postOpaque`
//! passes, `postTransparent`, the glow, the effects and the post-effects
//! chain. `docs/formats/postfx.md` ("Order") lists the steps.
//!
//! The stage keeps three surfaces in the original's frame (colour at
//! `ebp - 0x34`, normal at `- 0xb4`, depth at `- 0x44`) and destroys them
//! on every way out; the HDAO and `postOpaque` surfaces (`- 0x114`,
//! `- 0x134`) and the camera matrices (`- 0x194` product, `- 0xf4` saved
//! matrix, `- 0x84` lift) are there too.

#[allow(clippy::missing_errors_doc)] // `Err` is the renderer's error code, documented once
pub mod bridge {
    //! The stage over game memory.

    use crate::h3e_util::bridge::{
        Step, al, begin, camera_arg, clear_color, clear_scene, copied, copy_buffer, copy_color,
        drop_surface, end, free_camera_mode, global, instance, magic_id, motion_blur, ok, post,
        post_word, render_world, renderer, scene_clear_color, screen_fade, set_camera_matrix,
        set_color, set_depth, set_pass, set_targets, special_magic, surface, texture,
        viewport_degraded, viewport_to,
    };
    #[allow(clippy::wildcard_imports)] // the addresses
    use crate::h3e_util::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The surfaces and matrices (`ebp -` these).
    pub const COLOR: u32 = 0x34;
    pub const NORMAL: u32 = 0xb4;
    pub const DEPTH: u32 = 0x44;
    pub const HDAO: u32 = 0x114;
    pub const POST_OPAQUE: u32 = 0x134;
    pub const PRODUCT: u32 = 0x194;
    pub const SAVED: u32 = 0xf4;
    pub const LIFT: u32 = 0x84;

    /// The 3D scene stage (`0x00668ed3`), entered when
    /// `PhyreScene_check1195` answers and no movie fades: shadows unless
    /// `+0x93e`, the HDAO depth pass, the before-scene delegations, then
    /// [`scene_body`] on the colour (pp `+0x8c`), normal (`+0x94`) and
    /// depth (`+0x90`) surfaces, destroyed after it whatever it answers.
    /// `Err` is the first failing renderer call's code, which
    /// `PhyreScene_render` returns.
    pub fn scene_stage(h: &mut impl Host, ebp: u32, scene: u32) -> Step {
        if h.read::<u8>(scene + 0x93e) == 0 {
            h.call_sym(RENDER_SHADOW, Conv::Thiscall(scene), &[]);
        }
        h.call_sym(RENDER_HDAO, Conv::Thiscall(scene), &[]);
        h.call_sym(
            BEFORE_SCENE,
            Conv::Thiscall(global(h, DELEGATION_HELPER)),
            &[],
        );
        let color = post_word(h, scene, 0x8c);
        let normal = post_word(h, scene, 0x94);
        let depth = post_word(h, scene, 0x90);
        surface(h, ebp - COLOR, color);
        surface(h, ebp - NORMAL, normal);
        surface(h, ebp - DEPTH, depth);
        let r = scene_body(h, ebp, scene);
        drop_surface(h, ebp - DEPTH);
        drop_surface(h, ebp - NORMAL);
        drop_surface(h, ebp - COLOR);
        r
    }

    /// The stage with its surfaces built: [`draw_scene`], the HDAO and
    /// `postOpaque` passes unless the menu capture is done (state 2),
    /// [`post_transparent`], the glow before the effects, [`effects`] and,
    /// unless the capture is done, [`post_chain`].
    pub fn scene_body(h: &mut impl Host, ebp: u32, scene: u32) -> Step {
        draw_scene(h, ebp, scene)?;
        if global(h, MENU_CAPTURE_STATE) != 2 {
            let broken = {
                let obj = instance(h, BROKEN_SCREEN_INSTANCE);
                h.call_sym(BROKEN_SCREEN_ACTIVE, Conv::Thiscall(obj), &[])
            };
            if !al(broken) {
                hdao_pass(h, ebp, scene)?;
            }
            if global(h, MENU_CAPTURE_STATE) != 2 {
                post_opaque(h, ebp, scene)?;
            }
        }
        post_transparent(h, ebp, scene)?;
        if !al(h.call_sym(IS_GLOW_AFTER_VFX, Conv::Thiscall(scene), &[])) {
            glow(h, ebp, scene)?;
        }
        effects(h, ebp, scene)?;
        if global(h, MENU_CAPTURE_STATE) != 2 {
            post_chain(h, ebp, scene)?;
        }
        Ok(())
    }

    /// The camera lifted: its matrix (camera `+0x34`) saved at `ebp -
    /// 0xf4`, multiplied with a translation by scene `+0xfa34`, `+0xfa38`
    /// (built at `ebp - 0x84`; `PMatrix4_TimesRows` into `ebp - 0x194`,
    /// copied back), written to the camera through the x87 stack.
    pub fn lift_camera(h: &mut impl Host, ebp: u32, scene: u32) {
        let camera: u32 = h.read(scene + 0xdc);
        let matrix = camera.wrapping_add(0x34);
        for i in 0..16 {
            let w: u32 = h.read(matrix.wrapping_add(4 * i));
            h.write(ebp - SAVED + 4 * i, w);
        }
        let x = copied(h.read(scene + 0xfa34));
        let y = copied(h.read(scene + 0xfa38));
        let lift: [u32; 16] = [
            ONE, 0, 0, 0, //
            0, ONE, 0, 0, //
            0, 0, ONE, 0, //
            x, y, 0, ONE,
        ];
        for (i, w) in (0u32..).zip(lift) {
            h.write(ebp - LIFT + 4 * i, w);
        }
        let product = h.call_sym(
            MATRIX_TIMES_ROWS,
            Conv::Thiscall(ebp - LIFT),
            &[ebp - PRODUCT, matrix],
        );
        h.call_sym(MATRIX_COPY, Conv::Thiscall(ebp - LIFT), &[product]);
        set_camera_matrix(h, scene, ebp - LIFT);
    }

    /// `0x00668f3b..0x0066944e`: the targets (colour, none, depth), the
    /// stencil clear value 0, [`lift_camera`]; the colour cleared to the
    /// scene's clear colour (flags 0xb), the normal target to white; the
    /// prerender scene (with `+0xd4` and `+0x93c`, not `+0x93d`), the 3D
    /// world, and without `+0xd4` the prerender scene again and, with
    /// world kind `+0xf9cc` 0, the `Transparent` pass of world `+0xbc`.
    #[allow(clippy::too_many_lines)] // one straight sequence of passes
    pub fn draw_scene<H: Host>(h: &mut H, ebp: u32, scene: u32) -> Step {
        let color = ebp - COLOR;
        let normal = ebp - NORMAL;
        let depth = ebp - DEPTH;
        ok(set_color(h, scene, color, 0))?;
        ok(set_color(h, scene, 0, 1))?;
        ok(set_depth(h, scene, depth))?;
        viewport_to(h, scene, color);
        h.call_sym(SET_CLEAR_STENCIL, Conv::Thiscall(renderer(h, scene)), &[0]);
        lift_camera(h, ebp, scene);
        ok(begin(h, scene, 0))?;
        viewport_degraded(h, scene, color);
        scene_clear_color(h, scene);
        clear_scene(h, scene, 0xb);
        ok(end(h, scene, 1))?;
        ok(set_color(h, scene, normal, 0))?;
        ok(set_color(h, scene, 0, 1))?;
        ok(set_depth(h, scene, depth))?;
        viewport_to(h, scene, color);
        ok(begin(h, scene, 0))?;
        clear_color(h, scene, [ONE; 4]);
        clear_scene(h, scene, 1);
        ok(end(h, scene, 1))?;
        ok(set_color(h, scene, color, 0))?;
        ok(set_color(h, scene, normal, 1))?;
        ok(set_depth(h, scene, depth))?;
        scene_clear_color(h, scene);
        viewport_to(h, scene, color);
        ok(begin(h, scene, 0))?;
        let flag = |h: &H, off| h3e_byte(h, scene, off);
        if flag(h, 0xd4) && flag(h, 0x93c) && !flag(h, 0x93d) {
            h.call_sym(
                RUN_FLAG_10,
                Conv::Thiscall(global(h, DELEGATION_HELPER)),
                &[],
            );
            h.call_sym(DRAW_NON_UI_PRERENDER, Conv::Thiscall(scene), &[]);
        }
        h.call_sym(
            RUN_FLAG_20,
            Conv::Thiscall(global(h, DELEGATION_HELPER)),
            &[],
        );
        h.call_sym(DRAW_3D_WORLD, Conv::Thiscall(scene), &[1]);
        if !flag(h, 0xd4) {
            viewport_degraded(h, scene, color);
            if flag(h, 0x93c) && !flag(h, 0x93d) {
                h.call_sym(
                    RUN_FLAG_10,
                    Conv::Thiscall(global(h, DELEGATION_HELPER)),
                    &[],
                );
                h.call_sym(DRAW_NON_UI_PRERENDER, Conv::Thiscall(scene), &[]);
                if h.read::<u32>(scene + 0xf9cc) == 0 {
                    h.call_sym(FUN_0065A590, Conv::Thiscall(scene), &[]);
                    set_pass(h, scene, TRANSPARENT_NAME);
                    render_world(h, scene, 0xbc, u32::MAX);
                }
            }
        }
        h.call_sym(
            RUN_FLAG_4000,
            Conv::Thiscall(global(h, DELEGATION_HELPER)),
            &[],
        );
        ok(end(h, scene, 1))?;
        ok(set_color(h, scene, 0, 1))
    }

    fn h3e_byte(h: &impl Host, scene: u32, off: u32) -> bool {
        h.read::<u8>(scene + off) != 0
    }

    /// The HDAO pass (`0x00669479`): pp `+0x9c` in a surface at `ebp -
    /// 0x114` with the scene depth, cleared to 0 (`beginScene(1)`,
    /// `endScene(3)`), then `PostProcessManager_renderHDAOPass(post;
    /// renderer, camera, colour, normal)`.
    pub fn hdao_pass(h: &mut impl Host, ebp: u32, scene: u32) -> Step {
        let target = post_word(h, scene, 0x9c);
        surface(h, ebp - HDAO, target);
        let cleared = (|| {
            set_targets(h, scene, ebp - HDAO, ebp - DEPTH)?;
            ok(clear_color(h, scene, [0; 4]))?;
            ok(begin(h, scene, 1))?;
            ok(end(h, scene, 3))
        })();
        if cleared.is_ok() {
            let camera = camera_arg(h, scene);
            let color = post_word(h, scene, 0x8c);
            let normal = post_word(h, scene, 0x94);
            let r = renderer(h, scene);
            h.call_sym(
                RENDER_HDAO_PASS,
                Conv::Thiscall(post(h, scene)),
                &[r, camera, color, normal],
            );
        }
        drop_surface(h, ebp - HDAO);
        cleared
    }

    /// The `postOpaque` pass (`0x0066956d`): the flag-0x40 delegations,
    /// pp `+0x9c` in a surface at `ebp - 0x134` with the scene depth,
    /// cleared to 0, and unless `+0x93e` the `postOpaque` pass of worlds
    /// `+0xac` and `+0x7c`.
    pub fn post_opaque(h: &mut impl Host, ebp: u32, scene: u32) -> Step {
        h.call_sym(
            RUN_FLAG_40,
            Conv::Thiscall(global(h, DELEGATION_HELPER)),
            &[],
        );
        let target = post_word(h, scene, 0x9c);
        surface(h, ebp - POST_OPAQUE, target);
        let r = (|| {
            set_targets(h, scene, ebp - POST_OPAQUE, ebp - DEPTH)?;
            ok(clear_color(h, scene, [0; 4]))?;
            if h3e_byte(h, scene, 0x93e) {
                return Ok(());
            }
            ok(begin(h, scene, 1))?;
            set_pass(h, scene, POST_OPAQUE_NAME);
            render_world(h, scene, 0xac, 1);
            render_world(h, scene, 0x7c, 1);
            ok(end(h, scene, 1))
        })();
        drop_surface(h, ebp - POST_OPAQUE);
        r
    }

    /// `0x006696a2`: the scene clear colour, the colour and depth targets,
    /// and unless `+0x93e` the `postTransparent` pass of worlds `+0x7c`
    /// and `+0xac` (`+0xac` first with `+0xd4`).
    pub fn post_transparent(h: &mut impl Host, ebp: u32, scene: u32) -> Step {
        ok(scene_clear_color(h, scene))?;
        set_targets(h, scene, ebp - COLOR, ebp - DEPTH)?;
        viewport_degraded(h, scene, ebp - COLOR);
        if h3e_byte(h, scene, 0x93e) {
            return Ok(());
        }
        ok(begin(h, scene, 0))?;
        set_pass(h, scene, POST_TRANSPARENT_NAME);
        let order = if h3e_byte(h, scene, 0xd4) {
            [0xac, 0x7c]
        } else {
            [0x7c, 0xac]
        };
        for world in order {
            render_world(h, scene, world, u32::MAX);
        }
        ok(end(h, scene, 1))
    }

    /// The glow (`PostProcessManager_render`, with the flag-0x100 and
    /// 0x200 delegations around it) unless the menu capture is done, then
    /// the colour and depth targets again.
    pub fn glow(h: &mut impl Host, ebp: u32, scene: u32) -> Step {
        if global(h, MENU_CAPTURE_STATE) != 2 {
            h.call_sym(
                RUN_FLAG_100,
                Conv::Thiscall(global(h, DELEGATION_HELPER)),
                &[],
            );
            let camera = camera_arg(h, scene);
            let color = post_word(h, scene, 0x8c);
            let r = renderer(h, scene);
            h.call_sym(
                POST_RENDER,
                Conv::Thiscall(post(h, scene)),
                &[r, camera, color],
            );
            h.call_sym(
                RUN_FLAG_200,
                Conv::Thiscall(global(h, DELEGATION_HELPER)),
                &[],
            );
        }
        set_targets(h, scene, ebp - COLOR, ebp - DEPTH)
    }

    /// The colour pass (`0x006698c2`) begun: with `+0xfb45` (cleared) the
    /// previous frame copied to pp `+0xa8` (mode 0 and no motion blur),
    /// cleared black (flags 0xb); with byte `+3` 1 flags 10; else 0.
    fn begin_world_9c(h: &mut impl Host, ebp: u32, scene: u32) -> Step {
        let color = ebp - COLOR;
        let flags = if h3e_byte(h, scene, 0xfb45) {
            if free_camera_mode(h) == 0
                && !al(h.call_sym(IS_MOTION_BLUR_ON, Conv::Thiscall(scene), &[]))
            {
                copy_color(h, scene, 0xa8);
            }
            h.write(scene + 0xfb45, 0_u8);
            set_targets(h, scene, color, ebp - DEPTH)?;
            viewport_degraded(h, scene, color);
            clear_color(h, scene, [0; 4]);
            0xb
        } else {
            set_targets(h, scene, color, ebp - DEPTH)?;
            viewport_degraded(h, scene, color);
            if h.read::<u8>(scene + 3) == 1 { 10 } else { 0 }
        };
        ok(begin(h, scene, flags))
    }

    /// The `Opaque`, `Transparent` (and `postTransparent`) passes of
    /// world `+0x9c`.
    fn world_9c(h: &mut impl Host, scene: u32, post_transparent: bool) {
        set_pass(h, scene, OPAQUE_NAME);
        render_world(h, scene, 0x9c, 1);
        set_pass(h, scene, TRANSPARENT_NAME);
        render_world(h, scene, 0x9c, u32::MAX);
        if post_transparent {
            set_pass(h, scene, POST_TRANSPARENT_NAME);
            render_world(h, scene, 0x9c, u32::MAX);
        }
    }

    /// `0x006698c2..0x00669f67`: world `+0x9c` (unless `+0xfb44` or magic
    /// 0x14, and only when `FUN_006589e0(0)` answers), the focus frame on
    /// maps 0x189-0x18e, the before-VFX delegations, the screen fade, the
    /// blur and world `+0x9c` again for magic 0x14, the effects merged and
    /// drawn (copies of the colour for magic 0x110/0x111 and word
    /// `0x00cec19c`), the glow after them, the cluster moved, the arrays
    /// cleared, the before-effects delegations.
    #[allow(clippy::too_many_lines)] // one straight sequence of passes
    pub fn effects(h: &mut impl Host, ebp: u32, scene: u32) -> Step {
        let color = ebp - COLOR;
        let depth = ebp - DEPTH;
        begin_world_9c(h, ebp, scene)?;
        if !h3e_byte(h, scene, 0xfb44) && magic_id(h) != 0x14 {
            if al(h.call_sym(FUN_006589E0, Conv::Thiscall(scene), &[0])) {
                world_9c(h, scene, true);
            }
            h.call_sym(FUN_0066C770, Conv::Thiscall(scene), &[]);
        }
        ok(end(h, scene, 1))?;
        if global(h, CURRENT_MAP_NO).wrapping_sub(0x189) <= 5 {
            let r = renderer(h, scene);
            let target = post_word(h, scene, 0xa4);
            h.call_sym(
                RENDER_FOCUS_FRAME,
                Conv::Thiscall(post(h, scene)),
                &[r, target],
            );
        }
        h.call_sym(
            BEFORE_VFX,
            Conv::Thiscall(global(h, DELEGATION_HELPER)),
            &[],
        );
        if h3e_byte(h, scene, 0xf9c5) {
            let drawn = h.call_sym(
                FUN_006AE0C0,
                Conv::Thiscall(global(h, DELEGATION_HELPER)),
                &[color],
            );
            let at = h.rebase(SCREEN_FADE_DELEGATED);
            h.write(at, drawn as u8);
            if !al(drawn) {
                screen_fade(h, scene, 0);
            }
        }
        if !h3e_byte(h, scene, 0xfb44) && magic_id(h) == 0x14 {
            viewport_degraded(h, scene, color);
            {
                let obj = instance(h, BLUR_EX_INSTANCE);
                h.call_sym(BLUR_EX_RENDER, Conv::Thiscall(obj), &[])
            };
            set_targets(h, scene, color, depth)?;
            viewport_degraded(h, scene, color);
            ok(begin(h, scene, 0))?;
            world_9c(h, scene, false);
            ok(end(h, scene, 1))?;
        }
        viewport_degraded(h, scene, color);
        h.call_sym(PPP_FILL_WAIT, Conv::Cdecl, &[]);
        h.call_sym(MERGE_VFX_ARRAYS, Conv::Thiscall(scene), &[]);
        if magic_id(h) == 0x110 || magic_id(h) == 0x111 {
            copy_color(h, scene, 0x9c);
        }
        if global(h, WORD_CEC19C) != 0 {
            copy_color(h, scene, 0xac);
        }
        set_targets(h, scene, color, depth)?;
        ok(begin(h, scene, 0))?;
        h.call_sym(DRAW_MOST_VFX, Conv::Thiscall(scene), &[]);
        ok(end(h, scene, 1))?;
        if al(h.call_sym(IS_GLOW_AFTER_VFX, Conv::Thiscall(scene), &[])) {
            glow(h, ebp, scene)?;
        }
        h.call_sym(MOVE_CLUSTER, Conv::Thiscall(scene), &[]);
        h.call_sym(CLEAR_ARRAYS, Conv::Thiscall(scene), &[]);
        h.call_sym(
            BEFORE_EFFECTS,
            Conv::Thiscall(global(h, DELEGATION_HELPER)),
            &[],
        );
        Ok(())
    }

    /// The scene's DTT effects (`0x00669f74`): without motion blur, the
    /// colour copied to pp `+0xa8` for the DTT buffer (`+0x3854` set; byte
    /// `+2` clear, or `+1` clear and `+2` set and `g_OpusWaveOmdEnd` 0, or
    /// magic 0x2c2 with the noise filled), then `RenderDTTVFX` on the
    /// colour when `+0x3854` is set.
    fn dtt<H: Host>(h: &mut H, ebp: u32, scene: u32) -> Step {
        let pending = |h: &H| h3e_word(h, scene, 0x3854) != 0;
        let draw = if al(h.call_sym(IS_MOTION_BLUR_ON, Conv::Thiscall(scene), &[])) {
            true
        } else if pending(h) {
            if h.read::<u8>(scene + 2) == 0 {
                copy_color(h, scene, 0xa8);
            }
            if pending(h) {
                if !h3e_byte(h, scene, 1)
                    && h3e_byte(h, scene, 2)
                    && global(h, OPUS_WAVE_OMD_END) == 0
                {
                    copy_color(h, scene, 0xa8);
                }
                if pending(h) {
                    if magic_id(h) == 0x2c2 && global(h, NOISE_FILLED_0706) == 1 {
                        copy_color(h, scene, 0xa8);
                    }
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        if draw && pending(h) {
            set_targets(h, scene, ebp - COLOR, ebp - DEPTH)?;
            viewport_degraded(h, scene, ebp - COLOR);
            h.call_sym(RENDER_DTT_VFX, Conv::Thiscall(scene), &[]);
        }
        Ok(())
    }

    fn h3e_word(h: &impl Host, scene: u32, off: u32) -> u32 {
        h.read(scene + off)
    }

    /// The post-effects chain (`0x00669f74..0x0066a709`), run unless the
    /// menu capture is done: [`dtt`], the specular pass (or the specular
    /// maps reset), the cross-fade capture, the screen fade for special
    /// magic, the DTT copy, the camera matrix put back, the broken screen,
    /// motion blur, blurs, distortion, [`afterimage`], the cross-fade
    /// copy, `RenderMF152Related`, unsharp mask, post AA and brightness.
    #[allow(clippy::too_many_lines)] // one straight sequence of passes
    pub fn post_chain<H: Host>(h: &mut H, ebp: u32, scene: u32) -> Step {
        let color = ebp - COLOR;
        dtt(h, ebp, scene)?;
        let needs = {
            let obj = instance(h, SPECULAR_INSTANCE);
            h.call_sym(NEED_SPECULAR_PASS, Conv::Thiscall(obj), &[])
        };
        if al(needs) {
            set_targets(h, scene, color, ebp - DEPTH)?;
            viewport_degraded(h, scene, color);
            ok(begin(h, scene, 0))?;
            let r = renderer(h, scene);
            {
                let obj = instance(h, SPECULAR_INSTANCE);
                h.call_sym(RENDER_SPECULAR_PASS, Conv::Thiscall(obj), &[r])
            };
            ok(end(h, scene, 1))?;
        } else {
            {
                let obj = instance(h, SPECULAR_INSTANCE);
                h.call_sym(RESET_SPECULAR_MAPS, Conv::Thiscall(obj), &[])
            };
        }
        let capture = |h: &H| {
            let pp = post(h, scene);
            h.read::<u8>(pp.wrapping_add(0x123ba)) == 1
        };
        if capture(h) && !blurring(h) {
            cross_fade_capture(h, scene, 0x1232c);
        }
        if h3e_byte(h, scene, 0xf9c5)
            && special_magic(h)
            && !al(h.call_sym(
                RUN_FLAG_1000,
                Conv::Thiscall(global(h, DELEGATION_HELPER)),
                &[color],
            ))
        {
            let pp = post(h, scene);
            if h.read::<u8>(pp.wrapping_add(0x123a0)) != 0 {
                screen_fade(h, scene, color);
            }
        }
        if al(h.call_sym(COPY_PREVIOUS_FRAME_FOR_DTT, Conv::Thiscall(scene), &[]))
            && free_camera_mode(h) != 1
            && !al(h.call_sym(IS_MOTION_BLUR_ON, Conv::Thiscall(scene), &[]))
        {
            copy_color(h, scene, 0xa8);
        }
        let pending = h3e_word(h, scene, 0x3854) != 0;
        h.write(scene + 2, u8::from(pending));
        set_camera_matrix(h, scene, ebp - SAVED);
        let broken = al({
            let obj = instance(h, BROKEN_SCREEN_INSTANCE);
            h.call_sym(BROKEN_SCREEN_ACTIVE, Conv::Thiscall(obj), &[])
        });
        {
            let obj = instance(h, BROKEN_SCREEN_INSTANCE);
            h.call_sym(BROKEN_SCREEN_RENDER, Conv::Thiscall(obj), &[])
        };
        motion_blur_render(h, scene);
        if capture(h) && blurring(h) {
            cross_fade_capture(h, scene, 0xa8);
        }
        {
            let obj = instance(h, BLUR_INSTANCE);
            h.call_sym(BLUR_RENDER, Conv::Thiscall(obj), &[])
        };
        if magic_id(h) != 0x14 || h3e_byte(h, scene, 0xfb44) {
            {
                let obj = instance(h, BLUR_EX_INSTANCE);
                h.call_sym(BLUR_EX_RENDER, Conv::Thiscall(obj), &[])
            };
        }
        if !broken {
            {
                let obj = instance(h, DISTORTION_INSTANCE);
                h.call_sym(DISTORTION_RENDER, Conv::Thiscall(obj), &[])
            };
        }
        if blurring(h) {
            let pp = post(h, scene);
            let a8: u32 = h.read(pp.wrapping_add(0xa8));
            if h.read::<u8>(pp.wrapping_add(0x123bb)) != 0
                && h.read::<u32>(pp.wrapping_add(0x1_2408)) != a8
            {
                let slot: u32 = h.read(pp.wrapping_add(0x1_232c));
                copy_buffer(h, scene, slot, a8, [0, 0, 0]);
                let pp = post(h, scene);
                let a8: u32 = h.read(pp.wrapping_add(0xa8));
                h.write(pp.wrapping_add(0x1_2408), a8);
            }
            copy_color(h, scene, 0x1_232c);
        }
        let pp = post(h, scene);
        if h.read::<u8>(pp.wrapping_add(0x123bb)) != 0 {
            afterimage(h, scene);
        }
        let pp = post(h, scene);
        if h.read::<u8>(pp.wrapping_add(0x123c4)) != 0 {
            copy_color(h, scene, 0x1_232c);
            let pp = post(h, scene);
            h.write(pp.wrapping_add(0x123c4), 0_u8);
        }
        h.call_sym(RENDER_MF152, Conv::Thiscall(scene), &[]);
        let r = renderer(h, scene);
        if !broken {
            h.call_sym(RENDER_USM, Conv::Thiscall(post(h, scene)), &[r]);
        }
        let mode: u32 = h.read(scene + 0x10dd8);
        h.call_sym(RENDER_POST_AA, Conv::Thiscall(scene), &[mode]);
        let color_slot = post_word(h, scene, 0x8c);
        h.call_sym(
            RENDER_BRIGHT,
            Conv::Thiscall(post(h, scene)),
            &[r, color_slot],
        );
        Ok(())
    }

    /// Whether the motion blur is on (its byte `+0x58`).
    fn blurring(h: &mut impl Host) -> bool {
        let mb = motion_blur(h);
        h.read::<u8>(mb.wrapping_add(0x58)) != 0
    }

    /// The colour copied to the target at pp `+dst` for the cross-fade,
    /// the capture switched off, pp `+0x12408` = that target.
    fn cross_fade_capture(h: &mut impl Host, scene: u32, dst: u32) {
        copy_color(h, scene, dst);
        h.call_sym(SET_CROSS_FADE_CAPTURE, Conv::Thiscall(post(h, scene)), &[0]);
        let pp = post(h, scene);
        let target: u32 = h.read(pp.wrapping_add(dst));
        h.write(pp.wrapping_add(0x1_2408), target);
    }

    /// `MotionBlur_renderMotionBlur(blur; renderer, colour)`.
    fn motion_blur_render(h: &mut impl Host, scene: u32) {
        let color = post_word(h, scene, 0x8c);
        let r = renderer(h, scene);
        let mb = motion_blur(h);
        h.call_sym(MOTION_BLUR_RENDER, Conv::Thiscall(mb), &[r, color]);
    }

    /// The motion-blur afterimage (pp `+0x123bb`, `0x0066a542`): the blur
    /// switched on for one render with weight `*(pp +0x123bc) / 128` (a
    /// short), its state and weight put back after it when it was on
    /// (the colour copied to pp `+0xac` and the buffers swapped around it).
    fn afterimage(h: &mut impl Host, scene: u32) {
        let mb = motion_blur(h);
        let was_on = h.read::<u8>(mb.wrapping_add(0x58)) != 0;
        let mb = motion_blur(h);
        let weight = copied(h.read(mb.wrapping_add(0x5c)));
        if was_on {
            copy_color(h, scene, 0xac);
            let pp = post(h, scene);
            let slot: u32 = h.read(pp.wrapping_add(0x1_2408));
            let t = texture(h, slot);
            let live = h.call_sym(LIVE_POINTER_00AC, Conv::Thiscall(pp), &[]);
            let mb = motion_blur(h);
            h.call_sym(MOTION_BLUR_6C5570, Conv::Thiscall(mb), &[t, live]);
        } else {
            let mb = motion_blur(h);
            h.write(mb.wrapping_add(0x58), 1_u16);
        }
        let pp = post(h, scene);
        let count: u32 = h.read(pp.wrapping_add(0x123bc));
        let n = h.read::<i16>(count);
        // fild, fmul qword 1/128, fstp dword: exact for a short
        let w = x87::store(f64::from(n) * 0.007_812_5);
        let mb = motion_blur(h);
        h.call_sym(MOTION_BLUR_SET_WEIGHT, Conv::Thiscall(mb), &[w]);
        motion_blur_render(h, scene);
        let mb = motion_blur(h);
        if was_on {
            h.write(mb.wrapping_add(0x58), 1_u16);
            let pp = post(h, scene);
            let live = h.call_sym(LIVE_POINTER_00AC, Conv::Thiscall(pp), &[]);
            let live2 = h.call_sym(LIVE_POINTER_1232C, Conv::Thiscall(pp), &[]);
            let mb = motion_blur(h);
            h.call_sym(MOTION_BLUR_6C5570, Conv::Thiscall(mb), &[live2, live]);
            let mb = motion_blur(h);
            h.call_sym(MOTION_BLUR_SET_WEIGHT, Conv::Thiscall(mb), &[weight]);
        } else {
            h.write(mb.wrapping_add(0x58), 0_u8);
        }
        let pp = post(h, scene);
        let count: u32 = h.read(pp.wrapping_add(0x123c0));
        if h.read::<u16>(count) == 0 {
            h.call_sym(SET_BYTE_123BB, Conv::Thiscall(post(h, scene)), &[0]);
        }
    }
}
