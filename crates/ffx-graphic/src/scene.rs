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

    /// `PhyreScene_getRenderTarget(this, which)` (`FUN_0065e050`): the
    /// embedded object at `+0xfacc` for 2, else `+0xfaa8`.
    #[must_use]
    pub fn render_target(scene: u32, which: u32) -> u32 {
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
}
