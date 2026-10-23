//! `Ch_DrawMain` (`0x0082ed50`; caller `Sg_MainLoop`): the frame's
//! character drawing. Four walks of the character array:
//!
//! 1. world matrices for the renderer instances (`+0x830`, `+0x838`);
//! 2. the "draw" flags (`+0x198 |= 0x50`) of the visible characters;
//! 3. shadows, visibility and the mesh colours (shade and alpha);
//! 4. skinning in the magic files that need it, the cloth jobs, the
//!    renderer pipeline, point lights, the effect-on signal and the
//!    specular parameters of the specular segments.
//!
//! The original keeps its matrices and vectors in a 0x420-byte frame whose
//! addresses it passes to its callees; the bridge takes that frame as
//! [`FRAME`] bytes at `frame` (laid out as the original's, `[ebp - x]` at
//! `frame + FRAME - x`).

/// The originals, on the characters.
pub mod bridge {
    use ffx_bridge::x87::{Reg, round_trip};
    use ffx_bridge::{Global, Mem};

    use crate::handle::bridge::{CHR_ARRAY, CHR_COUNT, GS_REGISTER_SINK};
    use crate::layout as l;
    use crate::sys::bridge::DRAW_DETACHED;

    /// Bytes of the original's frame (`sub esp, 0x420`).
    pub const FRAME: u32 = 0x420;

    /// The callees, all through the game: `getInterValefor`,
    /// `graphicLevelGetNameHash`, `MagicFile_GetMagicId`,
    /// `getScenerioFlag`, `AtelGetCurCtrlWork`, `MsBattleCheck`,
    /// `Ch_SysGetScreenMatrixPtr`, `Ch_SysGetCameraMatrixPtr`, the
    /// renderer's frame start (`0x00640df0`), `Ch_GetPos`, the renderer's
    /// Tidus position (`0x00641080`), the VU0 matrix helpers
    /// (`FFXVu0UnitMatrix`, `FFXVu0MulMatrix`, `FFXVu0CopyMatrix`,
    /// `FFXVu0ApplyMatrix`, `FFXVu0Normalize`, the transpose `0x006ed990`),
    /// `get_ptr32_func`, the renderer's instance calls, `Ch_*` helpers,
    /// the cloth job scheduler and the user heap.
    pub const GET_INTER_VALEFOR: u32 = 0x0078_7340;
    pub const LEVEL_NAME_HASH: u32 = 0x0064_1da0;
    pub const MAGIC_GET_ID: u32 = 0x009d_a3b0;
    pub const GET_SCENARIO_FLAG: u32 = 0x0078_7360;
    pub const ATEL_GET_CUR_CTRL_WORK: u32 = 0x0086_aff0;
    pub const MS_BATTLE_CHECK: u32 = 0x0078_0c90;
    pub const SCREEN_MATRIX_PTR: u32 = 0x0083_5b70;
    pub const CAMERA_MATRIX_PTR: u32 = 0x0083_5ba0;
    pub const VU0_MUL_MATRIX: u32 = 0x006e_d5c0;
    pub const RENDER_FRAME_START: u32 = 0x0064_0df0;
    pub const VU0_UNIT_MATRIX: u32 = 0x006e_d9a0;
    pub const GET_POS: u32 = 0x0082_ac90;
    pub const RENDER_TIDUS_POS: u32 = 0x0064_1080;
    pub const VU0_COPY_MATRIX: u32 = 0x006e_d360;
    pub const GET_PTR32: u32 = 0x00a4_4650;
    pub const SET_WORLD_MATRIX_BY_MDL_ID: u32 = 0x0063_c430;
    pub const UPDATE_SKELETON_BOUNDS: u32 = 0x0063_c500;
    pub const OUTPUT_ALL_VERTEX_BUFFERS: u32 = 0x0063_b290;
    pub const SET_WORLD_MATRIX: u32 = 0x0063_c3e0;
    pub const DRAW_RESERVE_CALC_NORM: u32 = 0x0083_36a0;
    pub const RENDER_SHADOW_START: u32 = 0x0064_3dc0;
    pub const UPDATE_SHADOW_VISIBILITY: u32 = 0x0064_4ab0;
    pub const UPDATE_CLOTH_MATRIX: u32 = 0x0083_14c0;
    pub const CULL_BY_DISTANCE: u32 = 0x0082_e220;
    pub const UPDATE_CLOTH_HIDE_FLAG: u32 = 0x0083_1480;
    pub const MAGIC_0146_KEY: u32 = 0x0064_1310;
    pub const SET_VISIBLE: u32 = 0x0063_c390;
    pub const DRAW_SINGLE_SHADOW: u32 = 0x0083_0240;
    pub const SET_MESH_COLOR: u32 = 0x0063_bb30;
    pub const SET_MESH_SEGMENT_COLOR: u32 = 0x0063_bc80;
    pub const SET_MESH_SEGMENT_ALPHA: u32 = 0x0063_bc50;
    pub const RENDER_SHADOW_END: u32 = 0x0064_3dd0;
    pub const SET_FUKA: u32 = 0x0083_57d0;
    pub const EMPTY_00638DD0: u32 = 0x0063_8dd0;
    pub const DEBUG_CLEAR_CLOTH_DEBUG_DRAW: u32 = 0x0063_8de0;
    pub const CLOTH_JOB_WAIT: u32 = 0x00a4_4b30;
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const APPLY_ENVELOPE: u32 = 0x0083_2110;
    pub const DRAW_CLOTH_CLEAR: u32 = 0x0083_0b90;
    pub const ADD_TO_PIPELINE: u32 = 0x0063_a010;
    pub const PLIGHT_CALC: u32 = 0x0083_05a0;
    pub const ET_CHR_DRAW_ON: u32 = 0x007f_c060;
    pub const CAMERA_GET_POS: u32 = 0x0083_f260;
    pub const TRANSPOSE: u32 = 0x006e_d990;
    pub const VU0_APPLY_MATRIX: u32 = 0x006e_d2e0;
    pub const VU0_NORMALIZE: u32 = 0x006e_d6f0;
    pub const MATRIX_FROM_DIR: u32 = 0x0084_35a0;
    pub const SET_SPECULAR: u32 = 0x0064_3cc0;
    pub const IS_SHADOW_EXCEPTION_ID: u32 = 0x0082_e200;
    pub const CLOTH_JOB_SCHEDULE: u32 = 0x00a4_4b40;

    /// The camera times the screen matrix, made at the start.
    pub const SCREEN_CAMERA_MATRIX: u32 = 0x0130_0820;
    /// A byte cleared at the start.
    pub const DRAW_BYTE_8BC: Global<u8> = Global::new(0x0130_08bc);
    /// Magic `0x275`'s matrix swap: bit 3 on, bits 0-2 for characters
    /// `0x10b2`..`0x10b4` (set: the character's matrix goes to its slot of
    /// [`MAGIC_275_MATRICES`] and the bit clears; else back).
    pub const MAGIC_MATRIX_SWAP: Global<u32> = Global::new(0x0194_0ac0);
    /// Three matrices for [`MAGIC_MATRIX_SWAP`].
    pub const MAGIC_275_MATRICES: u32 = 0x022f_b540;
    /// Set by `MagicFile_Stop` for magic `0x158`, `0x1b5`, `0x1b6`: the
    /// next drawn character `0x1081`, `0x1063`, `0x1064`, `0x1114` or
    /// `0x108b` is hidden (`+0x180 = 1`) and it clears.
    pub const MAGIC_STOP_HIDE: Global<u32> = Global::new(0x0194_0ac4);
    /// When set, character `0x3017` is hidden (`+0x180 = 0x12`) and it
    /// clears.
    pub const HIDE_3017: Global<u32> = Global::new(0x0194_0ab8);
    /// A debug switch and the character id it dumps the vertex buffers of
    /// (`graphicCharacterInstanceOutputAllVertexBuffers`, once).
    pub const DUMP_VERTEX: Global<u32> = Global::new(0x0130_07e0);
    pub const DUMP_VERTEX_ID: Global<u32> = Global::new(0x0130_07dc);
    /// The cloth job list (characters, a heap block) and its size in
    /// bytes.
    pub const CLOTH_JOBS: Global<u32> = Global::new(0x01a8_5f50);
    pub const CLOTH_JOBS_SIZE: Global<u32> = Global::new(0x01a8_5f54);
    /// The specular segment's world matrix.
    pub const SPECULAR_MATRIX: u32 = 0x023c_3b60;
    /// `g_MagicHiddenChr`.
    pub const MAGIC_HIDDEN_CHR: Global<u32> = Global::new(0x022f_b3dc);
    /// `g_CurrentMapNo`.
    pub const CURRENT_MAP: Global<u32> = Global::new(0x012f_bc40);

    /// The level hashes, scenario values and magic files the walks test.
    const HASH_1B84: u32 = 0x1b84;
    const HASH_VALEFOR: u32 = 0x24f9;

    /// Where the original's `[ebp - x]` is in `frame`.
    const fn fr(frame: u32, x: u32) -> u32 {
        frame.wrapping_add(FRAME - x)
    }

    /// What the walks need from the start.
    struct Scene {
        valefor: u8,
        hash: u32,
        magic: u32,
        scenario: u32,
        atel: bool,
        battle: u32,
    }

    /// `Ch_DrawMain()`. `junk` is what the original's frame holds where it
    /// reads words it never wrote: `[ebp-0x3c]` and `[ebp-0x2c]`, the third
    /// words of the specular light and camera vectors it passes to
    /// `graphicSgSetSpecular`, and `[ebp-0x3c8]`, the camera position's w
    /// for the frame's first specular segment (`FFXVu0ApplyMatrix` takes
    /// it in; later segments get the previous material's `+0x58`). The
    /// shim passes 0s.
    #[allow(clippy::too_many_lines)]
    pub fn ch_draw_main<M: Mem>(
        mem: &mut M,
        frame: u32,
        junk: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, f: u32| mem.rebase(f);
        mem.write(fr(frame, 0x3c), junk[0]);
        mem.write(fr(frame, 0x2c), junk[1]);
        mem.write(fr(frame, 0x3c8), junk[2]);
        #[allow(clippy::cast_possible_truncation)] // al
        let valefor = call(mem, g(mem, GET_INTER_VALEFOR), &[]) as u8;
        let hash = call(mem, g(mem, LEVEL_NAME_HASH), &[]);
        let magic = call(mem, g(mem, MAGIC_GET_ID), &[]);
        let scenario = call(mem, g(mem, GET_SCENARIO_FLAG), &[]) & 0xffff;
        let ctrl = call(mem, g(mem, ATEL_GET_CUR_CTRL_WORK), &[]);
        let atel = ctrl != 0 && mem.read::<u8>(ctrl) & 2 != 0;
        let battle = call(mem, g(mem, MS_BATTLE_CHECK), &[]);
        let s = Scene {
            valefor,
            hash,
            magic,
            scenario,
            atel,
            battle,
        };
        let screen = call(mem, g(mem, SCREEN_MATRIX_PTR), &[]);
        let camera = call(mem, g(mem, CAMERA_MATRIX_PTR), &[screen]);
        let view = g(mem, SCREEN_CAMERA_MATRIX);
        call(mem, g(mem, VU0_MUL_MATRIX), &[view, camera, screen]);
        let mut drawn = 0u32;
        DRAW_BYTE_8BC.set(mem, 0);
        call(mem, g(mem, RENDER_FRAME_START), &[]);
        let scale = fr(frame, 0x84);
        call(mem, g(mem, VU0_UNIT_MATRIX), &[scale]);
        for k in [0u32, 5, 10] {
            let v: f32 = mem.read(scale + 4 * k);
            mem.write(scale + 4 * k, Reg::constant(1000.0).mul_m32(v).store());
        }
        // 1: world matrices
        each(mem, |mem, c| {
            let tidus = crate::mapping::bridge::TIDUS.get(mem);
            if tidus != 0 && mem.read::<u8>(tidus + l::ACTIVE) != 0 {
                let p = fr(frame, 0x24);
                call(mem, g(mem, GET_POS), &[tidus, p, p + 4, p + 8]);
                mem.write(p + 12, 1f32);
                call(mem, g(mem, RENDER_TIDUS_POS), &[p]);
            }
            if mem.read::<u8>(c + l::ACTIVE) == 0 || mem.read::<u8>(c + 0x180) != 0 {
                return;
            }
            let f = mem.read::<u32>(c + l::FLAGS2) & !0x40;
            mem.write(c + l::FLAGS2, f);
            if f & 0x20 != 0 {
                return;
            }
            mem.write(c + l::FLAGS2, f & !0x10);
            if s.magic == 0x275 {
                magic_swap(mem, c, call);
            }
            if mem.read::<u32>(c + l::RENDER_INSTANCE) != 0 {
                world(mem, c, frame, call);
            }
            drawn += 1;
        });
        // 2: the draw flags
        each(mem, |mem, c| {
            let f: u32 = mem.read(c + l::FLAGS2);
            let alpha: f32 = mem.read(c + 0x354);
            if mem.read::<u8>(c + l::ACTIVE) != 0
                && mem.read::<u8>(c + 0x180) == 0
                && f & 0x20 == 0
                && mem.read::<u32>(c + l::FLAGS) & 0x2000 == 0
                && alpha != 0.0
            {
                mem.write(c + l::FLAGS2, f | 0x50);
            }
        });
        call(mem, g(mem, DRAW_RESERVE_CALC_NORM), &[]);
        call(mem, g(mem, RENDER_SHADOW_START), &[]);
        call(mem, g(mem, UPDATE_SHADOW_VISIBILITY), &[]);
        // 3: shadows, visibility, colours
        each(mem, |mem, c| {
            let hidden = mem.read::<u8>(c + 0x180);
            if mem.read::<u8>(c + l::ACTIVE) == 0 {
                return;
            }
            let f = mem.read::<u32>(c + l::FLAGS2) & !0x40;
            mem.write(c + l::FLAGS2, f);
            if mem.read::<u8>(c + l::FLAGS2) & 0x20 != 0 {
                return;
            }
            if show(mem, c, &s, hidden, call) {
                drawn += 1;
            } else {
                let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
                call(mem, g(mem, SET_VISIBLE), &[inst, 0]);
            }
        });
        call(mem, g(mem, RENDER_SHADOW_END), &[]);
        if drawn == 0 {
            let sink = GS_REGISTER_SINK.get(mem);
            call(mem, g(mem, SET_FUKA), &[4, sink]);
            return;
        }
        call(mem, g(mem, EMPTY_00638DD0), &[]);
        call(mem, g(mem, DEBUG_CLEAR_CLOTH_DEBUG_DRAW), &[]);
        let need = CHR_COUNT.get(mem).cast_unsigned() << 2;
        let size = CLOTH_JOBS_SIZE.get(mem);
        if size < need || size == 0 {
            call(mem, g(mem, CLOTH_JOB_WAIT), &[]);
            let old = CLOTH_JOBS.get(mem);
            if old != 0 {
                call(mem, g(mem, USER_FREE), &[old]);
            }
            let n = CHR_COUNT.get(mem).cast_unsigned() << 2;
            let p = call(mem, g(mem, USER_MALLOC), &[n]);
            CLOTH_JOBS.set(mem, p);
            CLOTH_JOBS_SIZE.set(mem, CHR_COUNT.get(mem).cast_unsigned() << 2);
        }
        // 4: skinning, cloth jobs, pipeline, lights, specular
        let mut jobs = 0u32;
        each(mem, |mem, c| {
            let flags: u32 = mem.read(c + l::FLAGS);
            let alpha: f32 = mem.read(c + 0x354);
            if mem.read::<u8>(c + l::ACTIVE) == 0
                || mem.read::<u8>(c + 0x180) != 0
                || flags & 0x1000 != 0
                || alpha == 0.0
                || mem.read::<u8>(c + l::FLAGS2) & 0x20 != 0
            {
                return;
            }
            mem.write(c + l::FLAGS, flags | 0x1000);
            draw(mem, c, frame, s.magic, &mut jobs, call);
        });
        if jobs.cast_signed() > 0 {
            let mut list = CLOTH_JOBS.get(mem);
            for exception in [true, false] {
                for i in 0..jobs {
                    let e: u32 = mem.read(list.wrapping_add(4 * i));
                    if e != 0 && (call(mem, g(mem, IS_SHADOW_EXCEPTION_ID), &[e]) != 0) == exception
                    {
                        call(
                            mem,
                            g(mem, CLOTH_JOB_SCHEDULE),
                            &[list.wrapping_add(4 * i), 1],
                        );
                        list = CLOTH_JOBS.get(mem);
                    }
                }
            }
        }
    }

    /// Calls `f` on every character, the array's end read again after each.
    fn each<M: Mem>(mem: &mut M, mut f: impl FnMut(&mut M, u32)) {
        let end = |mem: &M| {
            let n = CHR_COUNT.get(mem).cast_unsigned();
            CHR_ARRAY.get(mem).wrapping_add(n.wrapping_mul(l::SIZE))
        };
        let mut c = CHR_ARRAY.get(mem);
        while c != end(mem) {
            f(mem, c);
            c = c.wrapping_add(l::SIZE);
        }
    }

    /// Magic `0x275`'s matrix swap for characters `0x10b2`..`0x10b4`.
    fn magic_swap<M: Mem>(mem: &mut M, c: u32, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) {
        let swap = MAGIC_MATRIX_SWAP.get(mem);
        if swap & 8 == 0 {
            return;
        }
        let slot = match mem.read::<u16>(c) {
            0x10b2 => 0,
            0x10b3 => 1,
            0x10b4 => 2,
            _ => return,
        };
        let bit = 1u32 << slot;
        let saved = mem.rebase(MAGIC_275_MATRICES) + 0x40 * slot;
        let own = c + 0x1d0;
        if swap & bit != 0 {
            call(mem, mem.rebase(VU0_COPY_MATRIX), &[saved, own]);
            MAGIC_MATRIX_SWAP.set(mem, MAGIC_MATRIX_SWAP.get(mem) & !bit);
        } else {
            call(mem, mem.rebase(VU0_COPY_MATRIX), &[own, saved]);
        }
    }

    /// The model count of `c`'s header (read again each time).
    fn models(mem: &impl Mem, c: u32) -> u32 {
        let hdr: u32 = mem.read(c + l::MODEL_HEADER);
        u32::from(mem.read::<u16>(hdr.wrapping_add(6)))
    }

    /// Walk 1 for a character with a render instance: its matrix
    /// (`+0x1d0`) times the scale is the world matrix (`[ebp-0x104]`).
    /// Every model in the skin list (an entry whose `u16` `+2` names it;
    /// flags in the 0x80 words at `[ebp-0x3c4]`), or none: the instance and
    /// the shadow instance get it whole; else per model, a skinned one as
    /// is, the others times their base element's matrix. Then the
    /// skeleton bounds (some model skinned, attached) and the vertex dump.
    fn world<M: Mem>(
        mem: &mut M,
        c: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, f: u32| mem.rebase(f);
        let mask = fr(frame, 0x3c4);
        for k in 0..0x80 {
            mem.write(mask + 4 * k, 0u32);
        }
        let world = fr(frame, 0x104);
        call(
            mem,
            g(mem, VU0_MUL_MATRIX),
            &[world, c + 0x1d0, fr(frame, 0x84)],
        );
        let mut any = false;
        let mut all = true;
        if models(mem, c) != 0 {
            let mut i = 0u32;
            loop {
                let entries = |mem: &M| {
                    let hdr: u32 = mem.read(c + l::MODEL_HEADER);
                    u32::from(mem.read::<u16>(hdr.wrapping_add(8)))
                };
                if entries(mem) != 0 {
                    let mut j = 0u32;
                    loop {
                        let hdr: u32 = mem.read(c + l::MODEL_HEADER);
                        let list: u32 = mem.read(hdr.wrapping_add(0x14));
                        let p = call(mem, g(mem, GET_PTR32), &[list]);
                        let m = u32::from(mem.read::<u16>(p.wrapping_add(12 * j + 2)));
                        if m == i {
                            mem.write(mask.wrapping_add(4 * i), 1u32);
                            any = true;
                            break;
                        }
                        j += 1;
                        if j.cast_signed() >= entries(mem).cast_signed() {
                            break;
                        }
                    }
                }
                all &= mem.read::<u32>(mask.wrapping_add(4 * i)) != 0;
                i += 1;
                if i.cast_signed() >= models(mem, c).cast_signed() {
                    break;
                }
            }
        }
        if all {
            for o in [l::RENDER_INSTANCE, 0x838] {
                let inst: u32 = mem.read(c + o);
                call(mem, g(mem, SET_WORLD_MATRIX), &[inst, world]);
            }
        } else {
            let part = fr(frame, 0x1c4);
            let mut i = 0u32;
            while i.cast_signed() < models(mem, c).cast_signed() {
                if mem.read::<u32>(mask.wrapping_add(4 * i)) != 0 {
                    let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
                    call(mem, g(mem, SET_WORLD_MATRIX_BY_MDL_ID), &[inst, world, i]);
                } else {
                    let mdl: u32 = mem.read(
                        mem.read::<u32>(c + l::MODELS)
                            .wrapping_add(l::MODEL_SIZE * i),
                    );
                    let base = u32::from(mem.read::<u16>(mdl));
                    let e = mem
                        .read::<u32>(c + 0x328)
                        .wrapping_add(0x88)
                        .wrapping_add(base * l::ELEM_SIZE);
                    call(mem, g(mem, VU0_MUL_MATRIX), &[part, world, e]);
                    let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
                    call(mem, g(mem, SET_WORLD_MATRIX_BY_MDL_ID), &[inst, part, i]);
                }
                i += 1;
            }
        }
        if any && DRAW_DETACHED.get(mem) == 0 {
            let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
            call(mem, g(mem, UPDATE_SKELETON_BOUNDS), &[inst]);
        }
        if DUMP_VERTEX.get(mem) != 0 && DUMP_VERTEX_ID.get(mem) == u32::from(mem.read::<u16>(c)) {
            let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
            call(mem, g(mem, OUTPUT_ALL_VERTEX_BUFFERS), &[inst]);
            DUMP_VERTEX.set(mem, 0);
        }
    }

    /// Walk 3 for an active character not flagged out: the cloth matrix,
    /// the distance cull, the cloth hide flag, the scene's shadow switches
    /// (`+0x183`) and the special hides; false when it is not shown (the
    /// caller hides its instance), else the instance shown or hidden by
    /// `+0x180` and, attached, its colours ([`shade`]).
    #[allow(clippy::too_many_lines)]
    fn show<M: Mem>(
        mem: &mut M,
        c: u32,
        s: &Scene,
        hidden: u8,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        let g = |mem: &M, f: u32| mem.rebase(f);
        let id = |mem: &M| mem.read::<u16>(c);
        let shadow = |mem: &mut M, on: u8| mem.write(c + 0x183, on);
        call(mem, g(mem, UPDATE_CLOTH_MATRIX), &[c]);
        let culled = call(mem, g(mem, CULL_BY_DISTANCE), &[c]);
        call(mem, g(mem, UPDATE_CLOTH_HIDE_FLAG), &[c]);
        let scenario = s.scenario;
        if s.hash == HASH_1B84 && s.atel && (2..=7).contains(&id(mem)) && scenario >= 0x13b {
            shadow(mem, 0);
        }
        if scenario == 0x39a && matches!(id(mem), 2 | 5) {
            shadow(mem, 0);
        }
        if s.hash == HASH_VALEFOR {
            if s.valefor & 4 != 0 {
                let i = id(mem);
                let on = (1..=7).contains(&i) || s.valefor.cast_signed() < 0 || i == 0x2041;
                shadow(mem, u8::from(on));
            }
        } else if s.hash == 0xf99 {
            if matches!(id(mem), 8 | 0x20f1) {
                shadow(mem, 1);
            }
        } else if s.hash == 0x24c9 && id(mem) == 1 {
            shadow(mem, 1);
        }
        let other: u32 = mem.read(c + 0x19c);
        if other != 0 && mem.read::<u16>(other) == 0x207a {
            shadow(mem, 1);
            mem.write(other + 0x183, 1u8);
        }
        let i = id(mem);
        if matches!(i, 0x5009 | 0x500a | 0x501d | 0x5034 | 0x5055 | 0x5056)
            || (i == 0x5033 && s.hash != 0x200f)
            || (i == 0x4029 && s.hash != 0xfc1 && s.hash != 0x1c36)
        {
            shadow(mem, 1);
        }
        if MAGIC_STOP_HIDE.get(mem) == 1 && matches!(i, 0x1081 | 0x1063 | 0x1064 | 0x1114 | 0x108b)
        {
            mem.write(c + 0x180, 1u8);
            MAGIC_STOP_HIDE.set(mem, 0);
        }
        if call(mem, g(mem, MAGIC_0146_KEY), &[]) != 0 {
            shadow(mem, 0);
        }
        let i = id(mem);
        if i == 0x3017 && HIDE_3017.get(mem) != 0 {
            HIDE_3017.set(mem, 0);
            mem.write(c + 0x180, 0x12u8);
            return false;
        }
        if s.magic == 0x13c && matches!(i >> 12, 0 | 3) {
            return false;
        }
        if culled != 0 {
            return false;
        }
        call(mem, g(mem, DRAW_SINGLE_SHADOW), &[c]);
        let alpha: f32 = mem.read(c + 0x354);
        if alpha == 0.0 {
            return false;
        }
        if matches!(s.magic, 0x280 | 0x245) && mem.read::<u32>(c + l::FLAGS) & 0x2000 != 0 {
            return false;
        }
        if s.magic == 0xa8 && id(mem) == 0x10d3 {
            return false;
        }
        if id(mem) == 0x5075 {
            use crate::chr::bridge::{F117_HIDE_COUNT, F117_HIDES};
            let n = F117_HIDE_COUNT.get(mem);
            let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
            let pairs = F117_HIDES.ptr(mem).addr;
            for k in 0..n.max(0).cast_unsigned() {
                if inst == mem.read::<u32>(pairs + 8 * k + 4) {
                    let h: u8 = mem.read(pairs + 8 * k);
                    mem.write(c + 0x180, h);
                    break;
                }
            }
        }
        let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
        if mem.read::<u8>(c + 0x180) != 0 {
            call(mem, g(mem, SET_VISIBLE), &[inst, 0]);
        } else {
            call(mem, g(mem, SET_VISIBLE), &[inst, 1]);
            if DRAW_DETACHED.get(mem) == 0 {
                shade(mem, c, s, call);
            }
        }
        let i = id(mem);
        if i == 0x5075 {
            mem.write(c + 0x180, hidden);
        }
        if i == 0x5033 {
            let models: u32 = mem.read(c + l::MODELS);
            let a = if mem.read::<u8>(models.wrapping_add(0x18)) != 0 {
                0x38d1_b717 // 1e-4
            } else {
                1f32.to_bits()
            };
            let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
            call(mem, g(mem, SET_MESH_SEGMENT_ALPHA), &[inst, 0, 0, a]);
        }
        true
    }

    /// The shade `s` (1; 3 for character `0x5008` in maps `0xf1`/`0xf2`;
    /// else `+0x360` when it differs from `+0x364`; 1 for `0x5002` there
    /// outside battle), then unless character `0x4001` in map `0xc1`
    /// times `0.85·v + 0.15` (`v` = `+0x370` in levels `0x1ca` and `0x1c0e`
    /// at scenario `0x8ac`, else the light `+0x36c` when
    /// `g_ChrShadeByLight`) or `0.8·light + 0.2` (level `0x2fa`); the
    /// colours `+0x330`, `+0x33c`, `+0x348` times it and the alpha
    /// `+0x354` to the mesh, and to each segment whose alpha (`+0x14`) is
    /// under 1 with the alphas multiplied.
    #[allow(clippy::float_cmp)] // fucompp: the exact compare, NaN unequal
    fn shade<M: Mem>(
        mem: &mut M,
        c: u32,
        s: &Scene,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use crate::parts::bridge::SHADE_BY_LIGHT;
        let g = |mem: &M, f: u32| mem.rebase(f);
        let f = |mem: &M, o: u32| mem.read::<f32>(c + o);
        let id = |mem: &M| mem.read::<u16>(c);
        let map = CURRENT_MAP.get(mem);
        let pool = map == 0xf2 || map == 0xf1;
        let mut shade = 1f32;
        if pool && id(mem) == 0x5008 {
            shade = 3.0;
        } else if f(mem, 0x364) != f(mem, 0x360) {
            shade = round_trip(f(mem, 0x360));
        }
        if pool && id(mem) == 0x5002 && s.battle == 0 {
            shade = 1.0;
        }
        if map == 0xc1 && id(mem) == 0x4001 {
            shade = 1.0;
        } else {
            let k85 = |v: f32| {
                Reg::load(v) * Reg::constant(f64::from(0.85f32)) + Reg::constant(f64::from(0.15f32))
            };
            let light = |mem: &M| (SHADE_BY_LIGHT.get(mem) != 0).then(|| k85(f(mem, 0x36c)));
            let v = match s.hash {
                0x1c0e if s.scenario == 0x8ac => Some(k85(f(mem, 0x370))),
                0x1ca => Some(k85(f(mem, 0x370))),
                0x2fa => Some(
                    Reg::load(f(mem, 0x36c)) * Reg::constant(f64::from(0.8f32))
                        + Reg::constant(f64::from(0.2f32)),
                ),
                _ => light(mem),
            };
            if let Some(v) = v {
                shade = v.mul_m32(shade).store();
            }
        }
        let r = (Reg::load(f(mem, 0x330)) * Reg::load(shade)).store();
        let gr = (Reg::load(f(mem, 0x33c)) * Reg::load(shade)).store();
        let b = Reg::load(shade).mul_m32(f(mem, 0x348)).store();
        let a = round_trip(f(mem, 0x354));
        let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
        let rgb = [r.to_bits(), gr.to_bits(), b.to_bits()];
        call(
            mem,
            g(mem, SET_MESH_COLOR),
            &[inst, c, rgb[0], rgb[1], rgb[2], a.to_bits()],
        );
        let mut i = 0u32;
        while i.cast_signed() < models(mem, c).cast_signed() {
            let models: u32 = mem.read(c + l::MODELS);
            let seg = round_trip(mem.read::<f32>(models.wrapping_add(l::MODEL_SIZE * i + 0x14)));
            if seg < 1.0 {
                let sa = Reg::load(seg).mul_m32(a).store();
                let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
                call(
                    mem,
                    g(mem, SET_MESH_SEGMENT_COLOR),
                    &[inst, i, rgb[0], rgb[1], rgb[2], sa.to_bits(), 0],
                );
            }
            i += 1;
        }
    }

    /// Walk 4 for a drawn character (flag bit 12 just set): skinning in
    /// magic `0x1b6` (characters `0x1114`, `0x1064`) and `0x1b5` (`0x1063`),
    /// hidden while `g_MagicHiddenChr`; a cloth character (flag bit 21,
    /// `+0x834`) gets `Ch_DrawClothClear` and a place in the job list; the
    /// pipeline (attached), point lights, the effect-on signal once
    /// (`+0x198` bit 0) and the specular segments ([`specular`]).
    fn draw<M: Mem>(
        mem: &mut M,
        c: u32,
        frame: u32,
        magic: u32,
        jobs: &mut u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, f: u32| mem.rebase(f);
        let mut envelope = |mem: &mut M| {
            if MAGIC_HIDDEN_CHR.get(mem) != 0 {
                let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
                call(mem, g(mem, SET_VISIBLE), &[inst, 0]);
            }
            call(mem, g(mem, APPLY_ENVELOPE), &[c, 0]);
        };
        if magic == 0x1b6 {
            if mem.read::<u16>(c) == 0x1114 {
                envelope(mem);
            }
            if mem.read::<u16>(c) == 0x1064 {
                envelope(mem);
            }
        } else if magic == 0x1b5 && mem.read::<u16>(c) == 0x1063 {
            envelope(mem);
        }
        if mem.read::<u32>(c + l::FLAGS) & 0x0020_0000 != 0 && mem.read::<u32>(c + 0x834) != 0 {
            call(mem, g(mem, DRAW_CLOTH_CLEAR), &[c]);
            let list = CLOTH_JOBS.get(mem);
            mem.write(list.wrapping_add(4 * *jobs), c);
            *jobs += 1;
        }
        if DRAW_DETACHED.get(mem) == 0 {
            let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
            call(mem, g(mem, ADD_TO_PIPELINE), &[inst]);
        }
        call(mem, g(mem, PLIGHT_CALC), &[c]);
        let f: u32 = mem.read(c + l::FLAGS2);
        if f & 1 == 0 {
            mem.write(c + l::FLAGS2, f | 1);
            call(mem, g(mem, ET_CHR_DRAW_ON), &[c]);
        }
        specular(mem, c, frame, call);
    }

    /// The specular parameters: with a specular strength (byte `+0x63` of
    /// the material `+0x1c4`), the colour (`[ebp-0x14]`: bytes `+0x60..`
    /// times the alpha, and the strength, over 128) and, for each segment
    /// with a triangle group flagged 8, its world matrix
    /// ([`SPECULAR_MATRIX`]), the camera position in its space (times
    /// -1000) and the direction to it, normalized, as a matrix
    /// (`Sg_MatrixFromDir(.., 0x12)`, transposed). The vectors handed to
    /// `graphicSgSetSpecular` are (x, z, junk, w): the light
    /// `[ebp-0x44]` (w `+0x5c`) and the camera `[ebp-0x34]` (w `+0x58`);
    /// a NaN in the camera position skips the segment.
    fn specular<M: Mem>(
        mem: &mut M,
        c: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, f: u32| mem.rebase(f);
        let mat: u32 = mem.read(c + 0x1c4);
        let strength = mem.read::<u8>(mat.wrapping_add(0x63));
        if strength == 0 {
            return;
        }
        let k = Reg::constant(0.007_812_5);
        let alpha = round_trip(mem.read::<f32>(c + 0x354));
        let byte = |mem: &M, o: u32| Reg::constant(f64::from(mem.read::<u8>(mat.wrapping_add(o))));
        let colour = fr(frame, 0x14);
        let red = (byte(mem, 0x60) * Reg::load(alpha) * k).store();
        let green = (byte(mem, 0x61) * Reg::load(alpha) * k).store();
        let blue = (Reg::load(alpha) * byte(mem, 0x62) * k).store();
        let power = (Reg::constant(f64::from(strength)) * k).store();
        mem.write(colour, [red, green, blue, power]);
        let (v, w) = (fr(frame, 0x3d4), fr(frame, 0x3e4));
        let (local, unit, dir) = (fr(frame, 0xc4), fr(frame, 0x184), fr(frame, 0x144));
        let mut i = 0u32;
        while i.cast_signed() < models(mem, c).cast_signed() {
            let seg: u32 = mem.read(
                mem.read::<u32>(c + l::MODELS)
                    .wrapping_add(l::MODEL_SIZE * i),
            );
            let groups: u32 = mem.read(seg.wrapping_add(0x10));
            let n = u32::from(mem.read::<u16>(seg.wrapping_add(6)));
            if (0..n).any(|t| mem.read::<u8>(groups.wrapping_add(12 * t)) & 8 != 0) {
                let base = u32::from(mem.read::<u16>(seg));
                let e = mem
                    .read::<u32>(c + 0x328)
                    .wrapping_add(base * l::ELEM_SIZE + 0x88);
                let sm = g(mem, SPECULAR_MATRIX);
                call(mem, g(mem, VU0_MUL_MATRIX), &[sm, c + 0x1d0, e]);
                call(mem, g(mem, VU0_UNIT_MATRIX), &[unit]);
                call(mem, g(mem, VU0_MUL_MATRIX), &[local, sm, unit]);
                call(mem, g(mem, CAMERA_GET_POS), &[v]);
                for k in 0..3 {
                    let d = Reg::load(mem.read::<f32>(v + 4 * k))
                        .sub_m32(mem.read::<f32>(local + 0x30 + 4 * k))
                        .store();
                    mem.write(v + 4 * k, d);
                    mem.write(w + 4 * k, d.to_bits());
                }
                let vw: u32 = mem.read(v + 12);
                mem.write(w + 12, vw);
                call(mem, g(mem, TRANSPOSE), &[local, local]);
                call(mem, g(mem, VU0_APPLY_MATRIX), &[v, local, v]);
                let k1000 = Reg::constant(-1000.0);
                for k in 0..2 {
                    let x = (Reg::load(mem.read::<f32>(v + 4 * k)) * k1000).store();
                    mem.write(v + 4 * k, x);
                }
                let z = k1000.mul_m32(mem.read::<f32>(v + 8)).store();
                mem.write(v + 8, z);
                call(mem, g(mem, VU0_APPLY_MATRIX), &[w, local, w]);
                call(mem, g(mem, VU0_NORMALIZE), &[w, w]);
                call(mem, g(mem, MATRIX_FROM_DIR), &[dir, w, 0x12]);
                call(mem, g(mem, TRANSPOSE), &[dir, dir]);
                let mat: u32 = mem.read(c + 0x1c4);
                let cw = round_trip(mem.read::<f32>(mat.wrapping_add(0x58)));
                let lw = round_trip(mem.read::<f32>(mat.wrapping_add(0x5c)));
                mem.write(v + 12, cw);
                mem.write(w + 12, lw);
                let rd = |mem: &M, a: u32| round_trip(mem.read::<f32>(a));
                let (cam, light) = (fr(frame, 0x34), fr(frame, 0x44));
                let (vx, vz) = (rd(mem, v), rd(mem, v + 8));
                mem.write(cam, vx);
                mem.write(cam + 4, vz);
                mem.write(cam + 12, rd(mem, v + 12));
                let (wx, wz, ww) = (rd(mem, w), rd(mem, w + 8), rd(mem, w + 12));
                mem.write(light, wx);
                mem.write(light + 4, wz);
                mem.write(light + 12, ww);
                let vy: f32 = mem.read(v + 4);
                if !vx.is_nan() && !vy.is_nan() && !vz.is_nan() {
                    let inst: u32 = mem.read(c + l::RENDER_INSTANCE);
                    call(
                        mem,
                        g(mem, SET_SPECULAR),
                        &[inst, i, colour, light, cam, dir],
                    );
                }
            }
            i += 1;
        }
    }
}
