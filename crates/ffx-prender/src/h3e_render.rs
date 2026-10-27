//! `PhyreScene_render` (`0x00668930`, 10 KB; M4 list `h3-glue-render`,
//! agent h3e): the scene's frame. The 3D scene is [`crate::h3e_scene`];
//! this module holds the stages around it and the function itself. The
//! exception frame and the cookie are left out, as in [`crate::glue10`].
//!
//! The frame is the original's (`ebp - 0x194 .. ebp`): the surfaces and
//! camera matrices live at the original's offsets (callees get their
//! addresses). The spills at `ebp - 0x24 .. - 0x10` (clear colours, the
//! free camera's allocation, the broken-screen and blur bytes) stay in
//! Rust locals.

#[allow(clippy::missing_errors_doc)] // `Err` is the renderer's error code, documented once
pub mod bridge {
    //! The stages over game memory.

    use crate::h3e_scene::bridge::scene_stage;
    use crate::h3e_util::bridge::{
        Step, al, begin, camera_arg, clear_color, clear_scene, copied, copy_buffer, copy_color,
        degrade, drop_surface, end, global, instance, ok, post, post_word, renderer,
        scene_clear_color, screen_fade, set_color, set_depth, slot_rect, special_magic, surface,
        viewport, viewport_to, window_rect,
    };
    #[allow(clippy::wildcard_imports)] // the addresses
    use crate::h3e_util::*;
    use ffx_graphic::host::{Conv, Host};

    /// The late stage's surfaces (`ebp -` these).
    pub const AFTER_COLOR: u32 = 0x94;
    pub const AFTER_DEPTH: u32 = 0x124;
    pub const UI_COLOR: u32 = 0xa4;
    pub const UI_DEPTH: u32 = 0x104;
    pub const FADE_COLOR: u32 = 0x144;
    pub const FADE_DEPTH: u32 = 0x154;

    /// `PhyreScene_render(scene)` (thiscall, plain `ret`; frame from
    /// `ebp - 0x194`) -> 0, or the first failing renderer call's code
    /// (then the Iggy menus, the cluster release and the loading screen
    /// are skipped).
    ///
    /// [`prologue`]; on the sphere grid (`g_GraphicAbmap` with state
    /// below 4) [`abmap_frame`]; else [`main_camera`], the FMV checks
    /// (the video frame, a held FMV, the pause word `0x0134082c`), the
    /// scene's clear colour, then the 3D scene ([`scene_stage`] and
    /// [`late`]) when `PhyreScene_check1195` answers and no movie fades,
    /// only [`late`] when it does not, [`ui`] and [`present`]. Last
    /// [`tail`] and the Iggy menus.
    pub fn render(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        prologue(h, scene);
        let abmap = global(h, GRAPHIC_ABMAP);
        let r = if abmap != 0 && h.read::<i32>(abmap.wrapping_add(4)) < 4 {
            abmap_frame(h, scene)
        } else {
            match main_frame(h, ebp, scene) {
                Ok(Frame::Done) => return 0,
                Ok(Frame::Iggy) => {
                    iggy(h);
                    return 0;
                }
                r => r.map(|_| ()),
            }
        };
        if let Err(code) = r {
            return code;
        }
        tail(h, scene);
        iggy(h);
        0
    }

    /// How [`main_frame`] ends, past the common [`tail`].
    pub enum Frame {
        /// The tail and the Iggy menus.
        Tail,
        /// Straight to the Iggy menus (a video frame, a held FMV).
        Iggy,
        /// Nothing more (the pause word: only the cloud UI).
        Done,
    }

    /// `0x00668961..0x00668b93`: the map objects rebuilt, `preRender`
    /// once (byte `+0xf9c8`), the shadow scale and bias, the broken
    /// screen's frame capture, the cross-fade reset, the back buffer
    /// cleared black (viewport the window), the four cameras' view
    /// matrices (`+0xdc`, `+0x394`, `+0x238` with `+0x10d8c` 1, `+0x654`;
    /// their `+0xb4` pointed at the scene's matrices), [`free_camera`].
    pub fn prologue(h: &mut impl Host, scene: u32) {
        h.call_sym(REBUILD_MAP_OBJECTS, Conv::Thiscall(scene), &[]);
        if h.read::<u8>(scene + 0xf9c8) == 0 {
            h.call_sym(PRE_RENDER, Conv::Thiscall(scene), &[]);
            h.write(scene + 0xf9c8, 1_u8);
        }
        h.call_sym(SET_SHADOW_SCALE_BIAS, Conv::Thiscall(scene), &[]);
        {
            let obj = instance(h, BROKEN_SCREEN_INSTANCE);
            h.call_sym(BROKEN_SCREEN_CAPTURE, Conv::Thiscall(obj), &[])
        };
        h.call_sym(RESET_CROSS_FADE, Conv::Thiscall(post(h, scene)), &[]);
        set_color(h, scene, 0, 0);
        let rect = window_rect(h);
        viewport(h, scene, rect);
        clear_color(h, scene, [0, 0, 0, ONE]);
        begin(h, scene, 1);
        end(h, scene, 1);
        view_matrices(h, scene, 0xdc, 0xfa04);
        view_matrices(h, scene, 0x394, 0xfa3c);
        if h.read::<u32>(scene + 0x1_0d8c) == 1 && h.read::<u32>(scene + 0x238) != 0 {
            view_matrices(h, scene, 0x238, 0x1_0d94);
        }
        view_matrices(h, scene, 0x654, 0x7b0);
        free_camera(h, scene);
    }

    /// The camera at scene `+camera` pointed at the matrices at scene
    /// `+matrices` (its `+0xb4`), its view matrices updated.
    fn view_matrices(h: &mut impl Host, scene: u32, camera: u32, matrices: u32) {
        let c: u32 = h.read(scene + camera);
        h.write(c.wrapping_add(0xb4), scene + matrices);
        let c: u32 = h.read(scene + camera);
        h.call_sym(UPDATE_VIEW_MATRICES, Conv::Thiscall(c.wrapping_add(4)), &[]);
    }

    /// `g_FreeCamera`, created when missing (`operator new(0x340)`,
    /// `FreeCamera_FreeCamera`; 0 when the allocation fails).
    fn free_camera_instance(h: &mut impl Host) -> u32 {
        let at = h.rebase(FREE_CAMERA);
        let fc: u32 = h.read(at);
        if fc != 0 {
            return fc;
        }
        let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x340]);
        let fc = if block == 0 {
            0
        } else {
            h.call_sym(FREE_CAMERA_CTOR, Conv::Thiscall(block), &[])
        };
        h.write(at, fc);
        fc
    }

    /// The free camera synchronised with the scene camera when it is
    /// active (bytes `+0x338`, `+0x33c`, or a positive first word): the
    /// scene camera copied into it (`+0x168`, the floats `+0x150..`
    /// into `+0x2b4..`), then its camera (`+0xc`) back into the scene's.
    fn free_camera(h: &mut impl Host, scene: u32) {
        let fc = free_camera_instance(h);
        let active = h.read::<u8>(fc.wrapping_add(0x338)) != 0
            || h.read::<u8>(fc.wrapping_add(0x33c)) != 0
            || h.read::<i32>(fc) > 0;
        if !active {
            return;
        }
        let camera: u32 = h.read(scene + 0xdc);
        let arg = if camera == 0 {
            0
        } else {
            camera.wrapping_add(4)
        };
        h.call_sym(
            CAMERA_COPY_4533B0,
            Conv::Thiscall(fc.wrapping_add(0x168)),
            &[arg],
        );
        for (from, to) in [(0x150, 0x2b4), (0x154, 0x2b8)] {
            let v = copied(h.read(camera.wrapping_add(from)));
            h.write(fc.wrapping_add(to), v);
        }
        let fc = free_camera_instance(h);
        let camera: u32 = h.read(scene + 0xdc);
        let own = fc.wrapping_add(0xc);
        let arg = if own == 0 { 0 } else { own.wrapping_add(4) };
        h.call_sym(
            CAMERA_COPY_4533B0,
            Conv::Thiscall(camera.wrapping_add(4)),
            &[arg],
        );
        for off in [0x150, 0x154] {
            let v = copied(h.read(own.wrapping_add(off)));
            h.write(camera.wrapping_add(off), v);
        }
    }

    /// The sphere grid's frame (`0x00668ba6`): the target cleared black,
    /// `PhyreScene_RenderABmap` while its state is below 2, the arrays
    /// cleared.
    pub fn abmap_frame(h: &mut impl Host, scene: u32) -> Step {
        ok(clear_color(h, scene, [0, 0, 0, ONE]))?;
        ok(begin(h, scene, 0))?;
        clear_scene(h, scene, 1);
        ok(end(h, scene, 1))?;
        let abmap = global(h, GRAPHIC_ABMAP);
        if h.read::<i32>(abmap.wrapping_add(4)) < 2 {
            h.call_sym(RENDER_ABMAP, Conv::Thiscall(scene), &[]);
        }
        h.call_sym(CLEAR_ARRAYS, Conv::Thiscall(scene), &[]);
        Ok(())
    }

    /// The scene context's matrix (`+0xcc` `+0x40`) set from the camera's
    /// (`+0x34`); with `+0x93d` the camera's `+0xb8` set to the one at
    /// `[+0x7e0] +0xb8` around it (its projection updated each time,
    /// vtable slot 1), then 1.
    pub fn main_camera(h: &mut impl Host, scene: u32) {
        let lifted = h.read::<u8>(scene + 0x93d) != 0;
        if lifted {
            let src: u32 = h.read(scene + 0x7e0);
            let camera: u32 = h.read(scene + 0xdc);
            let v = copied(h.read(src.wrapping_add(0xb8)));
            h.write(camera.wrapping_add(0xb8), v);
            h.call_virtual(camera, 1, &[]);
        }
        let ctx: u32 = h.read(scene + 0xcc);
        let camera: u32 = h.read(scene + 0xdc);
        h.call_sym(
            MATRIX_COPY,
            Conv::Thiscall(ctx.wrapping_add(0x40)),
            &[camera.wrapping_add(0x34)],
        );
        if lifted {
            let camera: u32 = h.read(scene + 0xdc);
            h.write(camera.wrapping_add(0xb8), ONE);
            h.call_virtual(camera, 1, &[]);
        }
    }

    /// `0x00668c23..0x0066b0dd` without the sphere grid.
    pub fn main_frame(h: &mut impl Host, ebp: u32, scene: u32) -> Result<Frame, u32> {
        main_camera(h, scene);
        let fmv = h.call_sym(FMV_INSTANCE, Conv::Cdecl, &[]);
        let playing = h.read::<u8>(fmv.wrapping_add(0x6d2)) != 0;
        let small = if playing {
            Some(0)
        } else {
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            (h.read::<u32>(app.wrapping_add(0x270)) <= 0x2d0).then_some(1)
        };
        if let Some(v) = small {
            {
                let obj = instance(h, POST_INSTANCE);
                h.call_sym(SET_BYTE_123B1, Conv::Thiscall(obj), &[v])
            };
        }
        let shown = (h.call_sym(FMV_HAS_STATE, Conv::Thiscall(fmv), &[1]) != 0
            && h.read::<u8>(fmv.wrapping_add(0x6d2)) != 0)
            || h.read::<u8>(fmv.wrapping_add(0x6d0)) != 0;
        if shown && global(h, WAIT_FRAMES_BY_TIME) != 0 {
            video_frame(h, scene);
            return Ok(Frame::Iggy);
        }
        if h.read::<u8>(scene + 0x1_0d2e) != 0
            && (h.call_sym(FMV_HAS_STATE, Conv::Thiscall(fmv), &[3]) != 0
                || h.call_sym(FMV_HAS_STATE, Conv::Thiscall(fmv), &[4]) != 0)
        {
            h.call_sym(PPP_FILL_WAIT, Conv::Cdecl, &[]);
            return Ok(Frame::Iggy);
        }
        if global(h, PAUSE_WORD_134082C) != 0 {
            h.call_sym(PPP_FILL_WAIT, Conv::Cdecl, &[]);
            h.call_sym(RENDER_CLOUD_UI, Conv::Thiscall(scene), &[]);
            return Ok(Frame::Done);
        }
        ok(scene_clear_color(h, scene))?;
        if !al(h.call_sym(CHECK_1195, Conv::Thiscall(scene), &[])) {
            h.call_sym(PPP_FILL_WAIT, Conv::Cdecl, &[]);
            late(h, ebp, scene)?;
        } else if global(h, MOVIE_FADING) != 0 {
            h.call_sym(PPP_FILL_WAIT, Conv::Cdecl, &[]);
        } else {
            scene_stage(h, ebp, scene)?;
            late(h, ebp, scene)?;
        }
        let captured = ui(h, ebp, scene)?;
        present(h, scene, captured);
        Ok(Frame::Tail)
    }

    /// A movie's frame (`0x00668cfa`): the video pass and its UI, the
    /// skip prompt, the colour copied to the back buffer, and the menu
    /// capture (state 1 -> 2) into pp `+0x9c` while a movie is up.
    pub fn video_frame(h: &mut impl Host, scene: u32) {
        h.call_sym(PPP_FILL_WAIT, Conv::Cdecl, &[]);
        h.call_sym(RENDER_VIDEO_AND_UI, Conv::Thiscall(scene), &[]);
        let vibration = u32::from(
            {
                let obj = instance(h, INPUT_INSTANCE);
                h.call_sym(INPUT_VIBRATION, Conv::Thiscall(obj), &[])
            }
            .to_le_bytes()[0],
        );
        let prompt: u32 = h.read(scene + 0x1_0de0);
        let r = renderer(h, scene);
        h.call_sym(SKIP_PROMPT_RENDER, Conv::Thiscall(prompt), &[r, vibration]);
        let color = post_word(h, scene, 0x8c);
        copy_buffer(h, scene, color, 0, [1, 0, 0]);
        let movie =
            h.call_sym(WITH_3D_SCENE, Conv::Cdecl, &[]) != 0 || global(h, MOVIE_PICTURE_ON) != 0;
        if movie && global(h, MENU_CAPTURE_STATE) == 1 {
            capture_menu_background(h, scene);
        }
    }

    /// The frame copied for the menu background: viewport the window,
    /// then the size of pp `+0x9c`, the colour copied there, state 2.
    fn capture_menu_background(h: &mut impl Host, scene: u32) {
        let rect = window_rect(h);
        viewport(h, scene, rect);
        let slot = post_word(h, scene, 0x9c);
        let rect = slot_rect(h, slot);
        viewport(h, scene, rect);
        copy_color(h, scene, 0x9c);
        let at = h.rebase(MENU_CAPTURE_STATE);
        h.write(at, 2_u32);
    }

    /// `0x0066a73a`: viewport the colour's size; the UI prerender scene
    /// (byte `+0x1195` clear or a battle map, with `+0x93d`); the 2D VFX
    /// on maps 0x163/0x164; the screen fade (`+0xf9c5`, no pending word
    /// `+0x3860` or those maps, no special magic, no `te/test20`, no
    /// flag-0x1000 delegation drawing it, pp `+0x123a0`); with `+0x388c`
    /// positive the effects after the post-process on a copy of the
    /// colour (pp `+0xac`), surfaces at `ebp - 0x94` and `- 0x124`.
    pub fn late(h: &mut impl Host, ebp: u32, scene: u32) -> Step {
        let slot = post_word(h, scene, 0x8c);
        let rect = slot_rect(h, slot);
        viewport(h, scene, rect);
        degrade(h, scene);
        let battle_or_field = h.read::<u8>(scene + 0x1195) == 0
            || al(h.call_sym(IS_ANY_BATTLE_MAP, Conv::Thiscall(scene), &[]));
        if battle_or_field && h.read::<u8>(scene + 0x93d) != 0 {
            h.call_sym(
                RUN_FLAG_400,
                Conv::Thiscall(global(h, DELEGATION_HELPER)),
                &[],
            );
            h.call_sym(RESTORE_DEFAULT_TARGETS, Conv::Thiscall(scene), &[]);
            h.call_sym(DRAW_UI_PRERENDER, Conv::Thiscall(scene), &[]);
        }
        if global(h, MAP_163_OR_164) != 0 {
            h.call_sym(RESTORE_DEFAULT_TARGETS, Conv::Thiscall(scene), &[]);
            h.call_sym(DRAW_UI_2D_VFX, Conv::Thiscall(scene), &[]);
        }
        let maps = global(h, MAP_163_OR_164);
        if h.read::<u8>(scene + 0xf9c5) != 0
            && (h.read::<u32>(scene + 0x3860) == 0 || maps != 0)
            && !special_magic(h)
            && global(h, YN_TEST20_SEEN) == 0
        {
            let drawn = h.call_sym(
                RUN_FLAG_1000,
                Conv::Thiscall(global(h, DELEGATION_HELPER)),
                &[0],
            );
            let at = h.rebase(SCREEN_FADE_DELEGATED);
            h.write(at, drawn as u8);
            let pp = post(h, scene);
            if !al(drawn) && h.read::<u8>(pp.wrapping_add(0x123a0)) != 0 {
                screen_fade(h, scene, 0);
            }
        }
        if h.read::<i32>(scene + 0x388c) > 0 {
            h.call_sym(RESTORE_DEFAULT_TARGETS, Conv::Thiscall(scene), &[]);
            copy_color(h, scene, 0xac);
            let color = post_word(h, scene, 0x8c);
            let depth = post_word(h, scene, 0x90);
            surface(h, ebp - AFTER_COLOR, color);
            surface(h, ebp - AFTER_DEPTH, depth);
            let r = (|| {
                ok(set_color(h, scene, ebp - AFTER_COLOR, 0))?;
                ok(set_depth(h, scene, ebp - AFTER_DEPTH))
            })();
            if r.is_ok() {
                viewport_to(h, scene, ebp - AFTER_COLOR);
                h.call_sym(RENDER_VFX_AFTER_POST, Conv::Thiscall(scene), &[]);
            }
            drop_surface(h, ebp - AFTER_DEPTH);
            drop_surface(h, ebp - AFTER_COLOR);
            r?;
        }
        Ok(())
    }

    /// The UI (`0x0066a983`) on the colour and depth targets (surfaces at
    /// `ebp - 0xa4`, `- 0x104`), when the minimap byte `+0x78`, the
    /// pending word `+0x3860` or open menus ask for it: stencil clear
    /// value 0, the 2D VFX (not on maps 0x163/0x164), the default targets
    /// (or, while a movie fades, pp `+0x9c` and no depth: the frame is
    /// then captured), scissor the colour's size, the minimap stencil
    /// with `+0x78` (viewport and scissor `+0x10d40..`), the normal UI.
    /// `Ok` says whether the frame was captured into pp `+0x9c`.
    pub fn ui(h: &mut impl Host, ebp: u32, scene: u32) -> Result<bool, u32> {
        let color = post_word(h, scene, 0x8c);
        let depth = post_word(h, scene, 0x90);
        surface(h, ebp - UI_COLOR, color);
        surface(h, ebp - UI_DEPTH, depth);
        let mut captured = false;
        let r = ui_body(h, ebp, scene, &mut captured);
        drop_surface(h, ebp - UI_DEPTH);
        drop_surface(h, ebp - UI_COLOR);
        r.map(|()| captured)
    }

    fn wants_ui(h: &mut impl Host, scene: u32) -> bool {
        if h.read::<u8>(scene + 0x78) != 0 || h.read::<u32>(scene + 0x3860) != 0 {
            return true;
        }
        let ui = global(h, GRAPHIC_UI);
        ui != 0 && (h.call_sym(MENU_COUNT, Conv::Thiscall(ui), &[]) as i32) > 0
    }

    fn ui_body(h: &mut impl Host, ebp: u32, scene: u32, captured: &mut bool) -> Step {
        ok(set_color(h, scene, ebp - UI_COLOR, 0))?;
        ok(set_depth(h, scene, ebp - UI_DEPTH))?;
        viewport_to(h, scene, ebp - UI_COLOR);
        degrade(h, scene);
        if !wants_ui(h, scene) {
            return Ok(());
        }
        h.call_sym(SET_CLEAR_STENCIL, Conv::Thiscall(renderer(h, scene)), &[0]);
        if global(h, MAP_163_OR_164) == 0 {
            h.call_sym(RESTORE_DEFAULT_TARGETS, Conv::Thiscall(scene), &[]);
            h.call_sym(DRAW_UI_2D_VFX, Conv::Thiscall(scene), &[]);
        }
        if global(h, MOVIE_FADING) == 0 {
            h.call_sym(RESTORE_DEFAULT_TARGETS, Conv::Thiscall(scene), &[]);
        } else {
            let target = post_word(h, scene, 0x9c);
            surface(h, ebp - FADE_COLOR, target);
            surface(h, ebp - FADE_DEPTH, 0);
            set_color(h, scene, ebp - FADE_COLOR, 0);
            set_color(h, scene, 0, 1);
            set_depth(h, scene, ebp - FADE_DEPTH);
            *captured = true;
            drop_surface(h, ebp - FADE_DEPTH);
            drop_surface(h, ebp - FADE_COLOR);
        }
        let slot = post_word(h, scene, 0x8c);
        let rect = slot_rect(h, slot);
        h.call_sym(SET_SCISSOR, Conv::Thiscall(renderer(h, scene)), &rect);
        let flags = u32::from(global(h, MOVIE_FADING) != 0);
        ok(begin(h, scene, flags))?;
        if h.read::<u8>(scene + 0x78) != 0 {
            let rect: [u32; 4] = [0, 4, 8, 0xc].map(|k| h.read(scene + 0x1_0d40 + k));
            viewport(h, scene, rect);
            h.call_sym(SET_SCISSOR, Conv::Thiscall(renderer(h, scene)), &rect);
            let slot = post_word(h, scene, 0x8c);
            let rect = slot_rect(h, slot);
            viewport(h, scene, rect);
            h.call_sym(MINIMAP_STENCIL, Conv::Thiscall(scene), &[]);
        }
        h.call_sym(DRAW_NORMAL_UI, Conv::Thiscall(scene), &[]);
        ok(end(h, scene, 1))
    }

    /// Whether the flag-0x1000 delegations leave the screen fade to the
    /// scene (they answer 0) and pp `+0x123a0` asks for it.
    fn fade_wanted(h: &mut impl Host, scene: u32) -> bool {
        if al(h.call_sym(
            RUN_FLAG_1000,
            Conv::Thiscall(global(h, DELEGATION_HELPER)),
            &[0],
        )) {
            return false;
        }
        let pp = post(h, scene);
        h.read::<u8>(pp.wrapping_add(0x123a0)) != 0
    }

    /// `0x0066ac8d..0x0066b0dd`: the screen fades after the UI, the debug
    /// 4:3 frame, render command 0x1e, the flag-0x800 delegations, the
    /// colour (or the captured frame) copied to the back buffer, the menu
    /// capture, the targets reset, the 4:3 viewport, the UI again over a
    /// captured menu background, the debug tweak utilities, the video
    /// text and the free camera's matrix into the scene camera.
    pub fn present(h: &mut impl Host, scene: u32, captured: bool) {
        if h.read::<u8>(scene + 0xf9c5) != 0 {
            if h.read::<u32>(scene + 0x3860) != 0
                && global(h, MAP_163_OR_164) == 0
                && !special_magic(h)
                && global(h, YN_TEST20_SEEN) == 0
                && global(h, PAUSE_ON) == 0
                && fade_wanted(h, scene)
            {
                screen_fade(h, scene, 0);
            }
            if h.read::<u8>(scene + 0xf9c5) != 0
                && global(h, YN_TEST20_SEEN) != 0
                && fade_wanted(h, scene)
            {
                screen_fade(h, scene, 0);
            }
        }
        if global(h, WORD_1136FC0) == 1 {
            let r = renderer(h, scene);
            h.call_sym(RENDER_DEBUG_4BY3, Conv::Thiscall(post(h, scene)), &[r]);
        }
        h.call_sym(
            QUEUE_COMMAND_1E,
            Conv::Thiscall(renderer(h, scene)),
            &[1, 1],
        );
        h.call_sym(
            RUN_FLAG_800,
            Conv::Thiscall(global(h, DELEGATION_HELPER)),
            &[],
        );
        let source = post_word(h, scene, if captured { 0x9c } else { 0x8c });
        copy_buffer(h, scene, source, 0, [1, 0, 1]);
        let mut captured = false;
        if global(h, MENU_CAPTURE_STATE) == 1 {
            capture_menu_background(h, scene);
            captured = true;
        }
        set_color(h, scene, 0, 0);
        set_depth(h, scene, 0);
        let r = renderer(h, scene);
        {
            let obj = instance(h, POST_INSTANCE);
            h.call_sym(SET_NON_16BY9_VIEWPORT, Conv::Thiscall(obj), &[r, 0])
        };
        if global(h, MENU_CAPTURE_STATE) == 2 && !captured {
            begin(h, scene, 0);
            let rect = window_rect(h);
            h.call_sym(SET_SCISSOR, Conv::Thiscall(renderer(h, scene)), &rect);
            h.call_sym(DRAW_NORMAL_UI, Conv::Thiscall(scene), &[]);
            end(h, scene, 1);
        }
        tweak_utils(h, scene);
        let text = h.call_sym(INSTANCE_6DC800, Conv::Cdecl, &[]);
        let r = renderer(h, scene);
        h.call_sym(
            VIDEO_TEXT_RENDER,
            Conv::Thiscall(text.wrapping_add(0x110)),
            &[r],
        );
        let mode = {
            let obj = instance(h, FREE_CAMERA_INSTANCE);
            h.call_sym(GET_MODE_OR_2, Conv::Thiscall(obj), &[])
        };
        if (mode as i32) > 0 {
            let fc = h.call_sym(FREE_CAMERA_INSTANCE, Conv::Cdecl, &[]);
            let camera: u32 = h.read(scene + 0xdc);
            h.call_sym(
                CAMERA_FROM_456430,
                Conv::Thiscall(camera),
                &[fc.wrapping_add(0x164)],
            );
        }
    }

    /// The application's float `+0x30`, through the x87 stack.
    fn app_float(h: &mut impl Host) -> u32 {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        copied(h.read(app.wrapping_add(0x30)))
    }

    /// The debug tweak utilities' renders (each draws nothing unless its
    /// debug menu is on).
    fn tweak_utils(h: &mut impl Host, scene: u32) {
        let camera = camera_arg(h, scene);
        let f = app_float(h);
        let r = renderer(h, scene);
        {
            let obj = instance(h, CHARACTER_LIGHTING_INSTANCE);
            h.call_sym(
                CHARACTER_LIGHTING_RENDER,
                Conv::Thiscall(obj),
                &[r, camera, f],
            )
        };
        for (getter, render) in TWEAK_UTILS {
            let f = app_float(h);
            let r = renderer(h, scene);
            {
                let obj = instance(h, getter);
                h.call_sym(render, Conv::Thiscall(obj), &[r, f])
            };
        }
    }

    /// `0x0066b0dd`: the cluster objects released, byte `+0xf9c7`
    /// cleared, the loading screen drawn (10 ms sleep in its state 3).
    pub fn tail(h: &mut impl Host, scene: u32) {
        h.call_sym(RELEASE_CLUSTER_OBJECTS, Conv::Thiscall(scene), &[]);
        h.write(scene + 0xf9c7, 0_u8);
        let screen = global(h, LOADING_SCREEN);
        let r = renderer(h, scene);
        h.call_sym(LOADING_SCREEN_RENDER, Conv::Thiscall(screen), &[r]);
        let screen = global(h, LOADING_SCREEN);
        if h.read::<u32>(screen.wrapping_add(0x10)) == 3 {
            h.call_sym(SLEEP_MS, Conv::Cdecl, &[10]);
        }
    }

    /// `IggyMenuManager_Render(g_IggyMenuManager)`.
    fn iggy(h: &mut impl Host) {
        let manager = global(h, IGGY_MENU_MANAGER);
        h.call_sym(IGGY_RENDER, Conv::Thiscall(manager), &[]);
    }
}
