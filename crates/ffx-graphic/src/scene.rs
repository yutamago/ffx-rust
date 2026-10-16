//! The remaster's scene object (`PhyreScene`, one instance at
//! `g_PhyreScene`) and the `graphic*` calls of the game that read and set
//! its state: the current level, the map flags, the minimap switches, the
//! screen offset (`docs/formats/render.md` lists the fields).
//!
//! Functions that leave a byte in `al` and the caller's value in the rest
//! of `eax` return `u8`.

/// The originals, over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem, x87};

    /// `g_PhyreScene`.
    pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
    /// `g_PhyreScene3D`: whether a 3D scene is up (`PhyreScene_is3DScene`).
    pub const IS_3D: Global<u8> = Global::new(0x00cc_b9c0);
    /// `g_PhyreSceneOpeningPlaying` (`PhyreScene_isOpeningScreenPlaying`).
    pub const OPENING_PLAYING: Global<u8> = Global::new(0x00cc_b9c2);
    /// `g_GraphicMagicFileAsyncLoad` (`graphicIsMagicFileAsyncLoadAvailable`).
    pub const MAGIC_ASYNC_LOAD: Global<u32> = Global::new(0x00cc_b464);
    /// `g_GraphicGameOverNotify` (`graphicSetGameOverNotify`).
    pub const GAME_OVER_NOTIFY: Global<u32> = Global::new(0x00cc_b468);
    /// `g_GraphicTextureVideoInfo` (`graphicGetTextureVideoInfo`, a `u16`).
    pub const TEXTURE_VIDEO_INFO: Global<u16> = Global::new(0x00c3_22a8);
    /// `g_GraphicCharacterLoading`: bit 31 a flag, the rest a count
    /// (`graphicIsCharacterLoadingOrInstanciating` masks it off).
    pub const CHARACTER_LOADING: Global<u32> = Global::new(0x00cc_b6d4);
    /// `g_GraphicScreenDttOffset`: two floats.
    pub const DTT_OFFSET: Global<u32> = Global::new(0x00cc_b228);
    /// `g_MovieFading`: the flag `PhyreScene_isFading` reads beside the
    /// scene's own fade byte.
    pub const MOVIE_FADING: Global<u32> = Global::new(0x0134_0824);

    // ---- PhyreScene fields ----
    /// The level name (a `PString`: its pointer, bit 0 a flag).
    pub const LEVEL_NAME: u32 = 0xfaf0;
    /// The current level's name hash.
    pub const LEVEL_HASH: u32 = 0xfaf4;
    /// The previous level's name hash.
    pub const PREVIOUS_LEVEL_HASH: u32 = 0xfaf8;
    /// The current magic (effect) path (`PString`).
    pub const MAGIC_PATH: u32 = 0xfafc;
    /// The magic path to delete (`PString`).
    pub const DELETE_MAGIC_PATH: u32 = 0xfb00;

    /// The `PString` at `at`: its pointer without the flag bit.
    fn pstring(m: &impl Mem, at: u32) -> u32 {
        let p: u32 = m.read(at);
        p.wrapping_sub(p & 1)
    }

    /// `PhyreScene_getCurrentLevelNameHash(this)`.
    pub fn current_level_name_hash(m: &impl Mem, scene: u32) -> u32 {
        m.read(scene + LEVEL_HASH)
    }

    /// `PhyreScene_getPreviousLevelNameHash(this)`.
    pub fn previous_level_name_hash(m: &impl Mem, scene: u32) -> u32 {
        m.read(scene + PREVIOUS_LEVEL_HASH)
    }

    /// `PhyreScene_setCurrentLevelNameHash(this, hash)`: the current one
    /// becomes the previous one. Returns the new hash (`eax`).
    pub fn set_current_level_name_hash(m: &mut impl Mem, scene: u32, hash: u32) -> u32 {
        let old: u32 = m.read(scene + LEVEL_HASH);
        m.write(scene + PREVIOUS_LEVEL_HASH, old);
        m.write(scene + LEVEL_HASH, hash);
        hash
    }

    /// `PhyreScene_getCurrentLevelName(this)`.
    pub fn current_level_name(m: &impl Mem, scene: u32) -> u32 {
        pstring(m, scene + LEVEL_NAME)
    }

    /// `PhyreScene_getCurrentMagicPath(this)`.
    pub fn current_magic_path(m: &impl Mem, scene: u32) -> u32 {
        pstring(m, scene + MAGIC_PATH)
    }

    /// `PhyreScene_getToBeDeleteMagicPath(this)`.
    pub fn to_be_delete_magic_path(m: &impl Mem, scene: u32) -> u32 {
        pstring(m, scene + DELETE_MAGIC_PATH)
    }

    /// `graphicLevelGetNameHash()`.
    pub fn level_get_name_hash(m: &impl Mem) -> u32 {
        current_level_name_hash(m, SCENE.get(m))
    }

    /// A `u32` field of `this` (the one-line getters).
    pub fn field(m: &impl Mem, this: u32, offset: u32) -> u32 {
        m.read(this + offset)
    }

    /// `this + offset` (the getters returning an embedded object).
    #[must_use]
    pub fn embedded(this: u32, offset: u32) -> u32 {
        this.wrapping_add(offset)
    }

    /// The getters `movzx eax, byte [g_PhyreScene + offset]`.
    pub fn scene_byte(m: &impl Mem, offset: u32) -> u32 {
        let scene = SCENE.get(m);
        u32::from(m.read::<u8>(scene + offset))
    }

    /// `graphicGetMiniMapType()` (scene `+0x74`) and the other `u32`
    /// fields read through `g_PhyreScene`.
    pub fn scene_u32(m: &impl Mem, offset: u32) -> u32 {
        let scene = SCENE.get(m);
        m.read(scene + offset)
    }

    /// `graphicSetMiniMapType(v)` and the other `u32` setters through
    /// `g_PhyreScene`; returns the scene (`eax`).
    pub fn set_scene_u32(m: &mut impl Mem, offset: u32, v: u32) -> u32 {
        let scene = SCENE.get(m);
        m.write(scene + offset, v);
        scene
    }

    /// `graphicMinimapSetEnable(v)` (`+0x78`), `graphicMinimapFogSetEnable`
    /// (`+0x79`), `graphicSetMapAlphaFast` (`+0xd4`): the byte `v != 0`;
    /// returns the scene (`eax`).
    pub fn set_scene_flag(m: &mut impl Mem, offset: u32, v: u32) -> u32 {
        let scene = SCENE.get(m);
        m.write(scene + offset, u8::from(v != 0));
        scene
    }

    /// `graphicDoneResetMapParam()` (`+0x10d2d` = 0) and the other byte
    /// stores through `g_PhyreScene`; returns the scene (`eax`).
    pub fn set_scene_byte(m: &mut impl Mem, offset: u32, v: u8) -> u32 {
        let scene = SCENE.get(m);
        m.write(scene + offset, v);
        scene
    }

    /// `graphicSetIsBattleMap(v)`: the battle-map byte (`+0x1196`) moves to
    /// `+0x1197` and becomes `v != 0`. Returns the flag (`al`).
    pub fn set_is_battle_map(m: &mut impl Mem, v: u32) -> u8 {
        let scene = SCENE.get(m);
        let old: u8 = m.read(scene + 0x1196);
        m.write(scene + 0x1197, old);
        let flag = u8::from(v != 0);
        m.write(scene + 0x1196, flag);
        flag
    }

    /// `graphicIsAnyBattleMap()` (`FUN_00641bd0`): whether either
    /// battle-map byte is set.
    pub fn is_any_battle_map(m: &impl Mem) -> u32 {
        let scene = SCENE.get(m);
        u32::from(scene_is_any_battle_map(m, scene) != 0)
    }

    /// `PhyreScene_isAnyBattleMap(this)` (`FUN_00660330`): as
    /// [`is_any_battle_map`] on `this`, in `al`.
    pub fn scene_is_any_battle_map(m: &impl Mem, scene: u32) -> u8 {
        u8::from(m.read::<u8>(scene + 0x1196) != 0 || m.read::<u8>(scene + 0x1197) != 0)
    }

    /// `PhyreScene_isFading(this)` (`FUN_006523e0`): the scene's fade byte
    /// (`+0x10dcc`) or the movie layer's flag; in `al`.
    pub fn scene_is_fading(m: &impl Mem, scene: u32) -> u8 {
        u8::from(m.read::<u8>(scene + 0x10dcc) != 0 || MOVIE_FADING.get(m) != 0)
    }

    /// `PhyreScene_isDrawingNonUI2DScene(this)`.
    pub fn is_drawing_non_ui_2d_scene(m: &impl Mem, scene: u32) -> u32 {
        u32::from(m.read::<u8>(scene + 0xf9c7) == 0 && IS_3D.get(m) == 0)
    }

    /// `PhyreScene_setMeshVisible(mesh, visible)` (stdcall): the mesh's
    /// node (`+0x1c`) gets the mesh at `+0x1c` when visible, else null.
    pub fn set_mesh_visible(m: &mut impl Mem, mesh: u32, visible: u32) {
        if mesh == 0 {
            return;
        }
        let node: u32 = m.read(mesh + 0x1c);
        if node != 0 {
            m.write(node + 0x1c, if visible & 0xff != 0 { mesh } else { 0 });
        }
    }

    /// `PhyreScene_SetSplitScreenStat(this, v)` (`+0x10d8c`) and
    /// `SetSplitScreenType` (`+0x10d90`): stores `v` when it differs.
    /// Returns `v` (`eax`).
    pub fn set_if_changed(m: &mut impl Mem, this: u32, offset: u32, v: u32) -> u32 {
        if m.read::<u32>(this + offset) != v {
            m.write(this + offset, v);
        }
        v
    }

    /// The `thiscall` setters of one `u32` field; return `v` (`eax`).
    pub fn set_field(m: &mut impl Mem, this: u32, offset: u32, v: u32) -> u32 {
        m.write(this + offset, v);
        v
    }

    /// The `thiscall` setters of one byte field (`mov al, [arg]`): return
    /// the byte (`al`).
    pub fn set_byte_field(m: &mut impl Mem, this: u32, offset: u32, v: u32) -> u8 {
        #[allow(clippy::cast_possible_truncation)] // the byte argument
        let b = v as u8;
        m.write(this + offset, b);
        b
    }

    /// `PhyreScene_getLinearSampler(this, which)` (`FUN_0065e050`): the
    /// embedded object at `+0xfacc` for 2, else `+0xfaa8`.
    #[must_use]
    pub fn linear_sampler(scene: u32, which: u32) -> u32 {
        scene.wrapping_add(if which == 2 { 0xfacc } else { 0xfaa8 })
    }

    /// The "live pointer" getters (`FUN_0065ded0`, `FUN_0065e2f0`): the
    /// object at `this + offset` holds a pointer at `+0` and a dead flag
    /// at `+4`.
    pub fn live_pointer(m: &impl Mem, this: u32, offset: u32) -> u32 {
        let p: u32 = m.read(this + offset);
        if m.read::<u8>(p + 4) == 0 {
            m.read(p)
        } else {
            0
        }
    }

    /// `graphicGetCut2DSceneBias()`: the float at scene `+0xf9bc`.
    pub fn scene_float(m: &impl Mem, offset: u32) -> f32 {
        let scene = SCENE.get(m);
        f32::from_bits(m.read::<u32>(scene + offset))
    }

    /// `graphicCameraGetFOVy()` (`+0x154`) and `getNearPlane()` (`+0xb8`):
    /// a float of the scene's camera (`+0xdc`).
    pub fn camera_float(m: &impl Mem, offset: u32) -> f32 {
        let scene = SCENE.get(m);
        let camera: u32 = m.read(scene + 0xdc);
        f32::from_bits(m.read::<u32>(camera + offset))
    }

    /// `graphicScreenSetOffset(x, y)`: the scene's screen offset
    /// (`+0xfa34`, `+0xfa38`), through the x87 (`fld`/`fstp`). Returns
    /// the scene (`eax`).
    pub fn screen_set_offset(m: &mut impl Mem, x: f32, y: f32) -> u32 {
        let scene = SCENE.get(m);
        m.write(scene + 0xfa34, x87::round_trip(x).to_bits());
        let scene = SCENE.get(m);
        m.write(scene + 0xfa38, x87::round_trip(y).to_bits());
        scene
    }

    /// `graphicScreenSetDttOffset(x, y)`.
    pub fn screen_set_dtt_offset(m: &mut impl Mem, x: f32, y: f32) {
        let at = DTT_OFFSET.ptr(m);
        at.set(m, x87::round_trip(x).to_bits());
        at.field::<u32>(4).set(m, x87::round_trip(y).to_bits());
    }

    /// `graphicScreenGetDttOffset(&x, &y)`; returns `y`'s pointer (`eax`).
    pub fn screen_get_dtt_offset(m: &mut impl Mem, x: u32, y: u32) -> u32 {
        let at = DTT_OFFSET.ptr(m);
        let vx = x87::round_trip(f32::from_bits(at.get(m)));
        m.write(x, vx.to_bits());
        let vy = x87::round_trip(f32::from_bits(at.field::<u32>(4).get(m)));
        m.write(y, vy.to_bits());
        y
    }

    /// `graphicCharacterInstanceSetDrawSituation(chr, v)`: the byte `+0x15`
    /// of a non-null character instance.
    pub fn character_set_draw_situation(m: &mut impl Mem, chr: u32, v: u32) {
        if chr != 0 {
            #[allow(clippy::cast_possible_truncation)] // the byte argument
            m.write(chr + 0x15, v as u8);
        }
    }

    /// `PhyreScene_getYIQParam(this, &y0, &y1, &y2, &o0, &o1, &o2)`: the
    /// three YIQ scales (`+0xfcac..`) and offsets (`+0xfcbc..`); while the
    /// override (`+0xfc88`) is on, scaled by `+0xfc8c..` with `+0xfc9c..`
    /// added to the offsets. Every value goes through the x87 on its way
    /// out. Returns the last pointer (`eax`).
    #[allow(clippy::many_single_char_names)]
    pub fn yiq_param(m: &mut impl Mem, scene: u32, out: [u32; 6]) -> u32 {
        use x87::Reg;
        let f = |m: &dyn Fn(u32) -> u32, at: u32| f32::from_bits(m(at));
        let rd = |at: u32| -> u32 { m.read(scene + at) };
        let mut v = [0f32; 6];
        for (i, off) in [0xfcac, 0xfcb0, 0xfcb4, 0xfcbc, 0xfcc0, 0xfcc4]
            .into_iter()
            .enumerate()
        {
            v[i] = f(&rd, off);
        }
        if m.read::<u8>(scene + 0xfc88) != 0 {
            let s = [f(&rd, 0xfc8c), f(&rd, 0xfc90), f(&rd, 0xfc94)];
            let o = [f(&rd, 0xfc9c), f(&rd, 0xfca0), f(&rd, 0xfca4)];
            // the original's operand orders: scale first for 0 and 2
            v[0] = Reg::load(s[0]).mul_m32(v[0]).store();
            v[3] = Reg::load(s[0]).mul_m32(v[3]).add_m32(o[0]).store();
            v[1] = Reg::load(v[1]).mul_m32(s[1]).store();
            v[4] = Reg::load(v[4]).mul_m32(s[1]).add_m32(o[1]).store();
            v[2] = Reg::load(s[2]).mul_m32(v[2]).store();
            v[5] = Reg::load(s[2]).mul_m32(v[5]).add_m32(o[2]).store();
        }
        for (at, x) in out.into_iter().zip(v) {
            m.write(at, x87::round_trip(x).to_bits());
        }
        out[5]
    }

    /// `PhyreScene_removeChrFromPipeline(this, chr)`: clears the first slot
    /// holding `chr` among the pipeline's characters (`+0xfb50`, count
    /// `+0xfb48`). Returns its index, else the count (`eax`).
    pub fn remove_chr_from_pipeline(m: &mut impl Mem, scene: u32, chr: u32) -> u32 {
        let count: u32 = m.read(scene + 0xfb48);
        let list: u32 = m.read(scene + 0xfb50);
        for i in 0..count {
            if m.read::<u32>(list + 4 * i) == chr {
                m.write(list + 4 * i, 0u32);
                return i;
            }
        }
        count
    }

    /// `graphicCharacterInstanceSetMeshSegmentHide(inst, id, hide)`: every
    /// segment (`0x6c` bytes, id at `+0x18`) of every mesh of the instance
    /// whose id matches gets the hide byte (`+0x14`). Returns `inst`.
    pub fn set_mesh_segment_hide(m: &mut impl Mem, inst: u32, id: u32, hide: u32) -> u32 {
        if inst == 0 {
            return inst;
        }
        let mut i = 0u32;
        while i < m.read::<u32>(inst) & 0x7fff_ffff {
            let meshes: u32 = m.read(inst + 4);
            let mesh_ref: u32 = m.read(meshes + 4 * i);
            let mesh: u32 = m.read(mesh_ref);
            let n = m.read::<u32>(mesh) & 0x7fff_ffff;
            for k in 0..n {
                let segs: u32 = m.read(mesh + 4);
                let at = segs.wrapping_add(0x6c * k);
                if m.read::<u32>(at + 0x18) == id {
                    m.write(at + 0x14, u8::from(hide != 0));
                }
            }
            i += 1;
        }
        inst
    }

    /// `g_FFEscMenu`: the PC escape menu object.
    pub const ESC_MENU: Global<u32> = Global::new(0x00cc_c870);

    /// `graphicIsEscMenuActive()` (`FUN_0063da80`): the escape menu exists
    /// and is open (`+0x70`) or has a selection (`+0xe4 >= 0`).
    pub fn is_esc_menu_active(m: &impl Mem) -> u32 {
        let menu = ESC_MENU.get(m);
        u32::from(menu != 0 && (m.read::<u8>(menu + 0x70) != 0 || m.read::<i32>(menu + 0xe4) >= 0))
    }

    /// The render target height the minimap viewport scales with: the
    /// live pointer of `*(scene + 0x10cc) + 0x8c`, its `+0x1c`.
    fn target_height(m: &impl Mem, scene: u32) -> u32 {
        let owner: u32 = m.read(scene + 0x10cc);
        let cell: u32 = m.read(owner + 0x8c);
        let target = if m.read::<u8>(cell + 4) == 0 {
            m.read(cell)
        } else {
            0u32
        };
        m.read(target + 0x1c)
    }

    /// `fild` of a word read as unsigned (`+ 2^32` when negative).
    fn unsigned(v: u32) -> f64 {
        f64::from(v)
    }

    /// `fistp qword` under truncation (`or cw, 0xc00`): the low word.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn chop(v: f64) -> u32 {
        // in range for every height the game has; out of range the x87
        // gives the integer indefinite
        let t = v.trunc();
        if t.is_nan() || t >= 9.223_372_036_854_776e18 || t < -9.223_372_036_854_776e18 {
            0
        } else {
            t as i64 as u32
        }
    }

    /// `PhyreScene_getMinimapViewportSize(this)` (`FUN_0065e320`): the
    /// minimap's size for the target height: `h * 120.25 / 16 * 9 / 416`
    /// in blitzball (`this[0]`), else `f32(h * 104) / 16 * 9 / 416`.
    pub fn minimap_viewport_size(m: &impl Mem, scene: u32) -> u32 {
        let h = target_height(m, scene);
        if m.read::<u8>(scene) != 0 {
            let v = x87::mul(unsigned(h), 120.25);
            let v = x87::div(x87::mul(x87::mul(v, 0.0625), 9.0), 416.0);
            chop(v)
        } else {
            let w = f64::from(f32::from_bits(x87::store(unsigned(h.wrapping_mul(0x68)))));
            chop(x87::div(x87::mul(x87::mul(w, 0.0625), 9.0), 416.0))
        }
    }

    /// The viewport offsets: `f32(h) * base / 512`, truncated.
    fn scaled(h: u32, base: f32) -> u32 {
        let d = f64::from(f32::from_bits(x87::store(unsigned(h))));
        chop(x87::mul(x87::mul(d, f64::from(base)), 0.001_953_125))
    }

    /// `PhyreScene_getMinimapViewportX(this)` (`FUN_0065e3f0`): by the
    /// minimap type (`+0x74`): 1, 2: 39; 3, 4: 368.75 in blitzball else
    /// 377; else 0; scaled by the target height / 512.
    pub fn minimap_viewport_x(m: &impl Mem, scene: u32) -> u32 {
        let base = match m.read::<u32>(scene + 0x74) {
            1 | 2 => 39.0,
            3 | 4 if m.read::<u8>(scene) != 0 => 368.75,
            3 | 4 => 377.0,
            _ => 0.0,
        };
        scaled(target_height(m, scene), base)
    }

    /// `PhyreScene_getMinimapViewportY(this)` (`FUN_0065e560`): 111 in
    /// blitzball else 96, scaled by the target height / 512.
    pub fn minimap_viewport_y(m: &impl Mem, scene: u32) -> u32 {
        let base = if m.read::<u8>(scene) != 0 {
            111.0
        } else {
            96.0
        };
        scaled(target_height(m, scene), base)
    }

    /// `graphicScreenGetOffset(&x, &y)`: the scene's screen offset, through
    /// the x87. Returns `y`'s pointer (`eax`).
    pub fn screen_get_offset(m: &mut impl Mem, x: u32, y: u32) -> u32 {
        let scene = SCENE.get(m);
        let vx = x87::round_trip(f32::from_bits(m.read(scene + 0xfa34)));
        m.write(x, vx.to_bits());
        let scene = SCENE.get(m);
        let vy = x87::round_trip(f32::from_bits(m.read(scene + 0xfa38)));
        m.write(y, vy.to_bits());
        y
    }

    /// `PhyreScene_setTextureVideoInfo(this, info, playing)`: unless `info`
    /// is 0x6b, the video info (`+0x10c4`) and its flag (`+0x10c8`).
    /// Returns `eax` (`info`, with the flag in `al` when stored).
    pub fn set_texture_video_info(m: &mut impl Mem, scene: u32, info: u32, playing: u32) -> u32 {
        if info == 0x6b {
            return info;
        }
        m.write(scene + 0x10c4, info);
        let flag = u8::from(playing != 0);
        m.write(scene + 0x10c8, flag);
        (info & !0xff) | u32::from(flag)
    }

    /// `graphicCharacterInstanceGetMaxiumBoundingBox(inst, &x, &z, &y)`: the
    /// first mesh's (`*(+4)[0] + 0x1c`) bounding-box maximum, `+0x10` to
    /// `x`, `+0x18` to `z`, `+0x14` to `y`, for an instance with meshes.
    #[allow(clippy::many_single_char_names, clippy::verbose_bit_mask)]
    pub fn character_max_bounding_box(m: &mut impl Mem, inst: u32, x: u32, z: u32, y: u32) {
        if inst == 0 || m.read::<u32>(inst) & 0x7fff_ffff == 0 {
            return;
        }
        for (off, to) in [(0x10, x), (0x14, y), (0x18, z)] {
            let meshes: u32 = m.read(inst + 4);
            let mesh: u32 = m.read(meshes);
            let bounds: u32 = m.read(mesh + 0x1c);
            let v = x87::round_trip(f32::from_bits(m.read(bounds + off)));
            m.write(to, v.to_bits());
        }
    }
}
