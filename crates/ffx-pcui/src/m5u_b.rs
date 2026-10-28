//! M5 stage 1, list `pc-ui`, batch 2: the menus' texture loads and the
//! rest of their unloads (`MenuTextureManager`, by name lists in
//! `.data`), the menu manager's mesh lookup and font name buffers, the
//! mouse state singleton, the button icon scale, a character's world
//! matrix and shader parameter setters, a `std::string` reset.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' argument words
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    use crate::m5u_a::bridge::{
        CLUSTER_MANAGER, FFX_LANG_JMP, LOCALIZATION_MANAGER, MEMSET_THUNK, MENU_UNLOAD_TEXTURE,
        OPERATOR_DELETE, OPERATOR_NEW, PHYRE_SCENE, SET_TEXTURE_UNLOADED, default_names,
        unload_named,
    };

    /// `KERNEL32`/`USER32` import slots.
    pub const GET_SYSTEM_METRICS_IAT: u32 = 0x00b0_c6dc;
    /// `g_MouseState` (name ours): the 0x7c-byte mouse state.
    pub const MOUSE_STATE: u32 = 0x0198_4c68;
    /// `g_ChrClothFlip` (`f32` -1.0), the mouse state's initial `+4`/`+8`.
    pub const MINUS_ONE: u32 = 0x00b9_2340;
    pub const SCENE_LOAD_MENU_TEXTURE: u32 = 0x0066_51d0;
    pub const LOAD_PCLUSTER: u32 = 0x0069_b940;
    pub const LOAD_PCLUSTER_JUMP: u32 = 0x0069_bb00;
    pub const GET_APPLICATION: u32 = 0x0062_3b90;
    pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
    pub const LOAD_SA_MENU: u32 = 0x0067_ce50;
    pub const LOAD_TEXTURE_CLUSTERS: u32 = 0x0068_4ef0;
    pub const MENU_LOAD_TEXTURE: u32 = 0x0067_dbb0;
    pub const SHADER_PARAM_WRITE: u32 = 0x0064_26e0;
    pub const SET_WORLD_MATRIX: u32 = 0x0063_c3e0;
    pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;
    pub const DYNGEO_BUFFER_ID: u32 = 0x0063_ff50;
    /// `"RenderBeforeVFX"` and `"RenderAfterVFX"`, the menu mesh's pass
    /// names (`MenuManager_GetMenuMeshInstace`).
    pub const RENDER_BEFORE_VFX: u32 = 0x00b0_da38;
    pub const RENDER_AFTER_VFX: u32 = 0x00b4_5b6c;
    /// The shader parameter names `0x00637c80` writes, in order.
    pub const PARAM_NAMES: [u32; 8] = [
        0x00b4_1c00,
        0x00b4_1c0c,
        0x00b4_1c18,
        0x00b4_1c24,
        0x00b4_1c30,
        0x00b4_1c3c,
        0x00b4_1c4c,
        0x00b4_1c58,
    ];
    /// Their value offsets in the record.
    pub const PARAM_OFFSETS: [u32; 8] = [4, 0x44, 0x84, 0x88, 0xc8, 0x108, 0x10c, 0x11c];
    /// The cloud-save UI texture ids (`0x00c5b44c..`, for screens 11..=15).
    pub const CLOUD_SAVE_IDS: u32 = 0x00c5_b44c;
    /// The dream map's five texture names (`0x00c59ebc..0x00c59ed0`).
    pub const DREAM_MAP_NAMES: u32 = 0x00c5_9ebc;
    /// The Eternal Calm copyright texture: Japanese, others.
    pub const COPYRIGHT_JP: u32 = 0x00c5_9ed0;
    pub const COPYRIGHT: u32 = 0x00c5_9ed4;
    /// The scene event font suffixes `LoadSceneEventFontTexture` appends.
    pub const FONT_SUFFIXES: [u32; 4] = [0x00c5_9bc0, 0x00c5_9bc4, 0x00c5_9c00, 0x00c5_9c04];

    fn g(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn rd(h: &impl Host, at: u32) -> u32 {
        h.read(at)
    }

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `PhyreScene_loadMenuTexture(g_PhyreScene; *sym)`; its `eax`.
    fn scene_load(h: &mut impl Host, sym: u32) -> u32 {
        let id = g(h, sym);
        let scene = g(h, PHYRE_SCENE);
        h.call_sym(SCENE_LOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[id])
    }

    /// An `f32` word as `fld`/`fstp` leave it: a signalling NaN quieted.
    fn quiet(bits: u32) -> u32 {
        if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
            bits | 0x0040_0000
        } else {
            bits
        }
    }

    // ---- singletons and small setters ----

    /// `MouseState_createInstance_00a25990()`: when [`MOUSE_STATE`] is
    /// unset, a new 0x7c-byte state: `+4`, `+8` = -1.0, `+0xc`, `+0x74`
    /// = 0, the word `+0` and the byte `+2` 0, `+0x78` =
    /// `GetSystemMetrics(SM_SWAPBUTTON) != 0` (0 on failure).
    pub fn mouse_state_create(h: &mut impl Host) {
        if g(h, MOUSE_STATE) != 0 {
            return;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x7c]);
        let at = h.rebase(MOUSE_STATE);
        if p == 0 {
            h.write(at, 0u32);
            return;
        }
        let one = quiet(g(h, MINUS_ONE));
        h.write(p + 4, one);
        h.write(p + 8, one);
        h.write(p + 0xc, 0u32);
        h.write(p + 0x74, 0u32);
        h.write(p, 0u16);
        h.write(p + 2, 0u8);
        let swap = import(h, GET_SYSTEM_METRICS_IAT, Conv::Stdcall, &[0x17]);
        h.write(p + 0x78, u8::from(swap != 0));
        h.write(at, p);
    }

    /// `RemapButtonIcon_resetButtonIconScale(this)`: both scales 1.0.
    pub fn reset_button_icon_scale(h: &mut impl Host, this: u32) {
        h.write(this + 0x160, 1.0f32.to_bits());
        h.write(this + 0x164, 1.0f32.to_bits());
    }

    /// `RemapButtonIcon_setButtonIconScale(this; x, y)` (`f32` words).
    pub fn set_button_icon_scale(h: &mut impl Host, this: u32, x: u32, y: u32) {
        h.write(this + 0x160, quiet(x));
        h.write(this + 0x164, quiet(y));
    }

    /// `0x00637c80(rec)` (a render delegation callback): with a record
    /// and its parameter block `*rec`, the eight named shader parameters
    /// at their offsets (`ShaderParam_Write_006426e0(block, name, rec +
    /// off)`). `eax`: the last write's; `rec + 4` without a block; with no
    /// record the original leaves the caller's `eax`, here 0.
    pub fn write_shader_params(h: &mut impl Host, rec: u32) -> u32 {
        if rec == 0 {
            return 0;
        }
        let block = rd(h, rec);
        if block == 0 {
            return rec + 4;
        }
        let mut eax = 0;
        for (name, off) in PARAM_NAMES.into_iter().zip(PARAM_OFFSETS) {
            let name = h.rebase(name);
            eax = h.call_sym(SHADER_PARAM_WRITE, Conv::Cdecl, &[block, name, rec + off]);
        }
        eax
    }

    /// `0x00637d40(rec)`: with a record, its matrix `+4` (16 words)
    /// copied to `frame` (the original's `ebp - 0x44`) and handed to
    /// `graphicCharacterInstanceSetWorldMatrix(*rec, frame)`.
    pub fn set_world_matrix(h: &mut impl Host, rec: u32, frame: u32) {
        if rec == 0 {
            return;
        }
        for i in 0..16 {
            let w = rd(h, rec + 4 + 4 * i);
            h.write(frame + 4 * i, w);
        }
        let list = rd(h, rec);
        h.call_sym(SET_WORLD_MATRIX, Conv::Cdecl, &[list, frame]);
    }

    /// `MenuManager_SetMenuUiMaterialUnloaded(this; name)`: a tail jump to
    /// `setTextureInMaterialUnloaded(**(this + 0x2bd4); name)`.
    pub fn set_menu_ui_material_unloaded(h: &mut impl Host, this: u32, name: u32) -> u32 {
        let geo = rd(h, rd(h, this + 0x2bd4));
        h.call_sym(SET_TEXTURE_UNLOADED, Conv::Thiscall(geo), &[name])
    }

    /// `MenuManager_ResetPauseVertexInfo(this)`:
    /// `graphicDynGeoMemManagerGetCurrentBufferId()` (its `eax`), then
    /// `+0x2c54` = 0.
    pub fn reset_pause_vertex_info(h: &mut impl Host, this: u32) -> u32 {
        let eax = h.call_sym(DYNGEO_BUFFER_ID, Conv::Cdecl, &[]);
        h.write(this + 0x2c54, 0u32);
        eax
    }

    /// `MenuManager_GetMenuMeshInstace(this; i)` -> the mesh's `+0xc`
    /// (0 when the menus aren't up, `+0x5b9d`, or neither table has entry
    /// `i`): the first table `+0x15f0` unless `+0x5b7c`, else the second
    /// `+0x4248`; the mesh's pass name set first (`"RenderBeforeVFX"` when
    /// its material's `+0xb4` is 1, else `"RenderAfterVFX"`).
    pub fn get_menu_mesh_instance(h: &mut impl Host, this: u32, i: u32) -> u32 {
        if h.read::<u8>(this + 0x5b9d) == 0 {
            return 0;
        }
        let mut e = 0;
        if rd(h, this + 0x5b7c) == 0 {
            e = rd(h, this.wrapping_add(i.wrapping_mul(4)).wrapping_add(0x15f0));
        }
        if e == 0 {
            e = rd(h, this.wrapping_add(i.wrapping_mul(4)).wrapping_add(0x4248));
            if e == 0 {
                return 0;
            }
        }
        let mesh = rd(h, e);
        let mat = rd(h, mesh + 0x24);
        let name = rd(h, mesh + 0xc) + 0x2c;
        let pass = if h.read::<u16>(mat + 0xb4) == 1 {
            RENDER_BEFORE_VFX
        } else {
            RENDER_AFTER_VFX
        };
        let pass = h.rebase(pass);
        h.call_sym(PSTRING_ASSIGN, Conv::Thiscall(name), &[pass]);
        rd(h, mesh + 0xc)
    }

    /// `0x0067b180(this)` -> `this` (a font name record's constructor):
    /// `+0` 0, the word `+4` 0, `+0x208` -1, `+0x20c` 0, the two 256-byte
    /// names `+6`, `+0x106` cleared (`memset`).
    pub fn font_names_init(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u16);
        h.write(this + 0x208, 0xffff_ffffu32);
        h.write(this + 0x20c, 0u32);
        h.call_sym(MEMSET_THUNK, Conv::Cdecl, &[this + 6, 0, 0x100]);
        h.call_sym(MEMSET_THUNK, Conv::Cdecl, &[this + 0x106, 0, 0x100]);
        this
    }

    /// Copies the C string at `src` to `dst` (with its terminator).
    fn str_copy(h: &mut impl Host, dst: u32, src: u32) {
        let mut i = 0;
        loop {
            let c: u8 = h.read(src.wrapping_add(i));
            h.write(dst.wrapping_add(i), c);
            if c == 0 {
                break;
            }
            i += 1;
        }
    }

    /// Appends the C string at `src` to the one at `dst`.
    fn str_cat(h: &mut impl Host, dst: u32, src: u32) {
        let mut end = dst;
        while h.read::<u8>(end) != 0 {
            end = end.wrapping_add(1);
        }
        str_copy(h, end, src);
    }

    /// `MenuTextureManager_LoadSceneEventFontTexture(this; name)`: for
    /// each of the four suffixes, the name buffer `+6` cleared, set to
    /// `name` + suffix and loaded; then cleared and set to `name`, and
    /// `+4` = 1.
    pub fn load_scene_event_font(h: &mut impl Host, this: u32, name: u32) {
        let buf = this + 6;
        h.call_sym(MEMSET_THUNK, Conv::Cdecl, &[buf, 0, 0x100]);
        for sym in FONT_SUFFIXES {
            str_copy(h, buf, name);
            let suffix = g(h, sym);
            str_cat(h, buf, suffix);
            let scene = g(h, PHYRE_SCENE);
            h.call_sym(SCENE_LOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[buf]);
            h.call_sym(MEMSET_THUNK, Conv::Cdecl, &[buf, 0, 0x100]);
        }
        str_copy(h, buf, name);
        h.write(this + 4, 1u8);
    }

    // ---- texture loads ----

    /// `0x0067c7a0(this)` (`MenuTextureManager`'s default loads): the
    /// language's nine clusters (`loadPClusterJump`), seven scene
    /// textures, the save menu's (`LoadSaMenuTexture(this; 5)`), then a
    /// tail jump to `GraphicAbmap_loadTextureClusters_00684ef0`; its
    /// `eax`.
    pub fn load_default_menu(h: &mut impl Host, this: u32) -> u32 {
        let loc = h.call_sym(LOCALIZATION_MANAGER, Conv::Cdecl, &[]);
        let names = h.rebase(default_names(rd(h, loc + 4)));
        for i in 0..9 {
            let name = rd(h, names + 4 * i);
            let m = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
            h.call_sym(LOAD_PCLUSTER_JUMP, Conv::Thiscall(m), &[name]);
        }
        for sym in [
            0x00c5_9d78,
            0x00c5_9d7c,
            0x00c5_9d74,
            0x00c5_9d68,
            0x00c5_9d64,
            0x00c5_9d84,
            0x00c5_9d6c,
        ] {
            scene_load(h, sym);
        }
        h.call_sym(LOAD_SA_MENU, Conv::Thiscall(this), &[5]);
        h.call_sym(LOAD_TEXTURE_CLUSTERS, Conv::Cdecl, &[])
    }

    /// `MenuTextureManager_LoadBGWaterTexture()`: `0x00c59d68`, `0x00c59d64`.
    pub fn load_bg_water(h: &mut impl Host) -> u32 {
        scene_load(h, 0x00c5_9d68);
        scene_load(h, 0x00c5_9d64)
    }

    /// `0x0067d650`, `0x0067d670`, `0x0067d690`: one scene texture each.
    pub fn load_one(h: &mut impl Host, sym: u32) -> u32 {
        scene_load(h, sym)
    }

    /// `MenuTextureManager_MenuLoadTexture(this; id)` (`ret 4`): the
    /// cluster loaded (`ClusterManager::loadPCluster`) into `frame` (the
    /// original's `ebp - 4`) and fixed up (`PApplication_FixupClusters(
    /// frame, 1)`); its `eax`.
    pub fn menu_load_texture(h: &mut impl Host, id: u32, frame: u32) -> u32 {
        h.write(frame, 0u32);
        load_cluster_into(h, id, frame)
    }

    /// The cluster `id` loaded into `cell`, then fixed up.
    fn load_cluster_into(h: &mut impl Host, id: u32, cell: u32) -> u32 {
        let m = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        let c = h.call_sym(LOAD_PCLUSTER, Conv::Thiscall(m), &[id]);
        h.write(cell, c);
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[cell, 1])
    }

    /// `MenuTextureManager_LoadCloudSaveUITexture(screen)` (`ret 4`, the
    /// caller's `ecx` handed on): screens 11..=15 load their texture
    /// (`MenuLoadTexture`); its `eax`, else `screen - 11`.
    pub fn load_cloud_save_ui(h: &mut impl Host, this: u32, screen: u32) -> u32 {
        let k = screen.wrapping_sub(11);
        if k > 4 {
            return k;
        }
        let id = g(h, CLOUD_SAVE_IDS + 4 * k);
        h.call_sym(MENU_LOAD_TEXTURE, Conv::Thiscall(this), &[id])
    }

    /// `MenuTextureManager_UnloadCloudSaveUITexture(screen)` (`ret 4`):
    /// as [`load_cloud_save_ui`] with `MenuUnloadTexture`.
    pub fn unload_cloud_save_ui(h: &mut impl Host, this: u32, screen: u32) -> u32 {
        let k = screen.wrapping_sub(11);
        if k > 4 {
            return k;
        }
        let id = g(h, CLOUD_SAVE_IDS + 4 * k);
        h.call_sym(MENU_UNLOAD_TEXTURE, Conv::Thiscall(this), &[id])
    }

    /// `MenuTextureManager_LoadDreamMapTexture()`: the five dream map
    /// textures; the last `eax`.
    pub fn load_dream_map(h: &mut impl Host) -> u32 {
        let mut eax = 0;
        for i in 0..5 {
            eax = scene_load(h, DREAM_MAP_NAMES + 4 * i);
        }
        eax
    }

    /// `MenuTextureManager_UnLoadDreamMapTexture()`: the five names
    /// unloaded ([`unload_named`], `buf` the original's `ebp - 0x104`).
    pub fn unload_dream_map(h: &mut impl Host, buf: u32) {
        for i in 0..5 {
            let name = g(h, DREAM_MAP_NAMES + 4 * i);
            if name != 0 {
                unload_named(h, name, buf);
            }
        }
    }

    /// `0x0067f920`, `0x0067f9a0`, `0x0067fa20`: the name in `sym`
    /// unloaded when set.
    pub fn unload_one(h: &mut impl Host, sym: u32, buf: u32) {
        let name = g(h, sym);
        if name != 0 {
            unload_named(h, name, buf);
        }
    }

    /// `MenuTextureManager_LoadEternalCalmCopyRightTexture()`: the
    /// Japanese (language 0) or the other copyright texture.
    pub fn load_copyright(h: &mut impl Host) -> u32 {
        let lang = h.call_sym(FFX_LANG_JMP, Conv::Cdecl, &[]);
        scene_load(h, if lang == 0 { COPYRIGHT_JP } else { COPYRIGHT })
    }

    /// `MenuTextureManager_UnloadEternalCalmCopyRightTexture(this)`.
    pub fn unload_copyright(h: &mut impl Host, this: u32) -> u32 {
        let lang = h.call_sym(FFX_LANG_JMP, Conv::Cdecl, &[]);
        let id = g(h, if lang == 0 { COPYRIGHT_JP } else { COPYRIGHT });
        h.call_sym(MENU_UNLOAD_TEXTURE, Conv::Thiscall(this), &[id])
    }

    /// `MenuTextureManager_LoadGameMenuTexture(kind)` (`ret 4`): kind 0
    /// loads `0x00c59d78`, `0x00c59d7c`, `0x00c59d74`; 1 the first; 2 the
    /// first and the third; the last `eax`, else `kind`.
    pub fn load_game_menu(h: &mut impl Host, kind: u32) -> u32 {
        let syms: &[u32] = match kind {
            0 => &[0x00c5_9d78, 0x00c5_9d7c, 0x00c5_9d74],
            1 => &[0x00c5_9d78],
            2 => &[0x00c5_9d78, 0x00c5_9d74],
            _ => return kind,
        };
        let mut eax = 0;
        for &sym in syms {
            eax = scene_load(h, sym);
        }
        eax
    }

    /// `MenuTextureManager_LoadSaSubMenuTexture(this; name)` (`ret 4`):
    /// the save sub-menu texture `+0x20c` replaced by `name` unless it is
    /// the same string (then `eax` 0): the old one unloaded, the new
    /// cluster loaded into the argument slot `slot` (the original reuses
    /// it), or with no old one into `frame` (`ebp - 4`); `FixupClusters`'
    /// `eax`.
    pub fn load_sa_sub_menu(h: &mut impl Host, this: u32, name: u32, slot: u32, frame: u32) -> u32 {
        let cur = rd(h, this + 0x20c);
        if cur == 0 {
            h.write(frame, 0u32);
            let eax = load_cluster_into(h, name, frame);
            h.write(this + 0x20c, name);
            return eax;
        }
        if str_eq(h, cur, name) {
            return 0;
        }
        h.call_sym(MENU_UNLOAD_TEXTURE, Conv::Thiscall(this), &[cur]);
        h.write(slot, 0u32);
        let eax = load_cluster_into(h, name, slot);
        h.write(this + 0x20c, name);
        eax
    }

    /// Whether the C strings at `a` and `b` are equal (the inlined
    /// `strcmp`).
    fn str_eq(h: &impl Host, s: u32, t: u32) -> bool {
        let mut i = 0u32;
        loop {
            let x: u8 = h.read(s.wrapping_add(i));
            let y: u8 = h.read(t.wrapping_add(i));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            i += 1;
        }
    }

    /// `0x0067f3c0(this)`: the save sub-menu texture `+0x20c` unloaded and
    /// cleared when set.
    pub fn unload_sa_sub_menu(h: &mut impl Host, this: u32) {
        let cur = rd(h, this + 0x20c);
        if cur != 0 {
            h.call_sym(MENU_UNLOAD_TEXTURE, Conv::Thiscall(this), &[cur]);
            h.write(this + 0x20c, 0u32);
        }
    }

    /// `0x00681800(this)` (`std::string::_Tidy(true)`): the heap buffer
    /// deleted when the capacity `+0x14` is 16 or more; capacity 15,
    /// size `+0x10` 0, the first byte 0.
    pub fn string_tidy(h: &mut impl Host, this: u32) {
        if rd(h, this + 0x14) >= 0x10 {
            let p = rd(h, this);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
        h.write(this + 0x14, 0xfu32);
        h.write(this + 0x10, 0u32);
        h.write(this, 0u8);
    }
}
