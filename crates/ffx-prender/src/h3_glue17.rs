//! The render-side glue, seventeenth batch (M4 port round 5, list
//! `h3-glue-render`): the texture video's frame update, the graphics
//! start-up (`graphicInitialize`) and the 16:9 viewport fit (x87).

use ffx_bridge::Global;

pub use crate::glue::SCENE;

/// `operator delete(p)` / `operator new(size)` (cdecl), `g_PauseOn`,
/// `PVideoPlaybackWin32_togglePause(this)` / `isPlaying(this)`,
/// `PRenderCommandQueue_queueCallback(this, f, data)` with
/// `PhyreScene_updateTextureVideoCallback`, `graphicVFXUpdate(lo, hi)`
/// and `graphicDynamicGeometryUpdateTexture(lo, hi)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const PAUSE_ON: Global<u32> = Global::new(0x0134_0828);
pub const VIDEO_TOGGLE_PAUSE: u32 = 0x00a2_7e90;
pub const VIDEO_IS_PLAYING: u32 = 0x00a2_7c50;
pub const QUEUE_CALLBACK: u32 = 0x005b_5880;
pub const TEXTURE_VIDEO_CALLBACK: u32 = 0x0067_2070;
pub const VFX_UPDATE: u32 = 0x0064_58a0;
pub const DYNAMIC_GEOMETRY_UPDATE_TEXTURE: u32 = 0x0064_0060;

/// `graphicInitialize`'s messages (`SaveDataTtyWrite2(level, text)`),
/// its singletons and their creators, in call order.
pub const TTY_WRITE: u32 = 0x0064_8850;
pub const INIT_BEGIN: u32 = 0x00b4_15c4;
pub const INIT_END: u32 = 0x00b4_15e4;
pub const INIT_FIRST: u32 = 0x006d_c800;
/// The singleton at `0x00cdede4` (0x238 bytes, `0x006db670(this)`) and
/// its set-up `0x006dba20(this)`.
pub const FIRST_SINGLETON: Global<u32> = Global::new(0x00cd_ede4);
pub const FIRST_SINGLETON_CTOR: u32 = 0x006d_b670;
pub const FIRST_SINGLETON_SETUP: u32 = 0x006d_ba20;
/// The creators between the first singleton and the delegation helper.
pub const CREATORS_A: [u32; 9] = [
    0x006e_56a0,
    0x006d_eb90,
    0x0069_f440,
    0x0069_a060,
    0x0068_db00,
    0x0069_1a80,
    0x0069_f4c0,
    0x0069_2a90,
    0x006a_be40,
];
/// `g_RenderDelegationHelper` (0x208 bytes,
/// `RenderDelegationHelper_RenderDelegationHelper(this)`).
pub const RENDER_DELEGATION_HELPER: Global<u32> = Global::new(0x00cc_ca98);
pub const RENDER_DELEGATION_HELPER_CTOR: u32 = 0x006a_d0d0;
/// The creators between the delegation helper and the loading screen.
pub const CREATORS_B: [u32; 26] = [
    0x006a_6cd0,
    0x006a_6d50,
    0x006b_60c0,
    0x0069_2b10,
    0x006b_4950,
    0x006b_cc40,
    0x006b_00a0,
    0x006c_1420,
    0x006c_2190,
    0x006c_34d0,
    0x006c_49a0,
    0x006e_76c0,
    0x006c_5030,
    0x006c_5790,
    0x006c_9500,
    0x006d_4d30,
    0x006d_7130,
    0x006b_5eb0,
    0x006b_6040,
    0x006b_5f30,
    0x006b_5e30,
    0x006b_5cb0,
    0x006b_5db0,
    0x006b_5d30,
    0x006b_5fc0,
    0x006e_3240,
];
/// The loading screen (0x44 bytes, `LoadingScreen_LoadingScreen(this)`).
pub const LOADING_SCREEN: Global<u32> = Global::new(0x00ce_517c);
pub const LOADING_SCREEN_CTOR: u32 = 0x006e_45e0;
pub const INIT_AFTER_LOADING_SCREEN: u32 = 0x006e_5490;
/// The scene (0x10de8 bytes, `PhyreScene_PhyreScene(this)`), its
/// post-process set-up and the monster names.
pub const SCENE_SIZE: u32 = 0x1_0de8;
pub const SCENE_CTOR: u32 = 0x0064_dc00;
pub const SCENE_INIT_POST_PROCESS: u32 = 0x0066_56e0;
pub const READ_MONSTER_NAMES: u32 = 0x0078_44c0;
/// The cloth heap: `user_aligned_malloc(size, align)`,
/// `create_mspace_with_base(base, size, locked)`, its base, mspace and
/// the "graphics up" flag.
pub const ALIGNED_MALLOC: u32 = 0x0068_6ff0;
pub const CREATE_MSPACE: u32 = 0x006e_bd30;
pub const CLOTH_BASE: Global<u32> = crate::h3_glue15::CLOTH_BASE;
pub const CLOTH_MSPACE: Global<u32> = crate::h3_glue15::CLOTH_MSPACE;
pub const GRAPHICS_UP: Global<u32> = Global::new(0x00cc_b45c);

/// The viewport fit: `Phyre_PFramework_PApplication_GetApplication`
/// (the window `+0x26c` x `+0x270`), 2^32 (the unsigned fix-up), the
/// target aspect, 9/16, 0.5, `Phyre_PRendering_PRendererBase_setViewport(this, x, y, w, h)`,
/// `g_IggyMenuManager` and `IggyMenuManager_setViewportRect_00a26bd0(this, x, y, w, h, fw, fh)`.
pub const GET_APPLICATION: u32 = 0x0062_3b90;
pub const TWO_POW_32: Global<f32> = Global::new(0x00b1_1f14);
pub const TARGET_ASPECT: Global<f32> = Global::new(0x00c3_4154);
pub const NINE_SIXTEENTHS: Global<f64> = Global::new(0x00b4_7c90);
pub const HALF: Global<f64> = Global::new(0x00b9_2220);
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const IGGY_MENU_MANAGER: Global<u32> = Global::new(0x0198_4c6c);
pub const IGGY_SET_VIEWPORT_RECT: u32 = 0x00a2_6bd0;
/// The x87 control word the game runs at (the fitted cases store it).
pub const CONTROL_WORD: u16 = 0x027f;

/// The character lighting: `Phyre_CharacterLighting_GetInstance`, its
/// `setIntensity(this, i, v)`, `setRotation(this, i, x, y)`, the tweak
/// utility's instance and `setCoefficient(this, i, v)`,
/// `getScenerioFlag`, `0x006aa650(this, flag)`, `setMapName(this, name)`,
/// `balanceLumWithGlow(this)`, `strstr`'s import slot and the name
/// that skips the balance.
pub const LIGHT_INSTANCE: u32 = 0x006a_6e30;
pub const LIGHT_SET_INTENSITY: u32 = 0x006a_a8c0;
pub const LIGHT_SET_ROTATION: u32 = 0x006a_aea0;
pub const LIGHT_TWEAK_INSTANCE: u32 = 0x006a_6e40;
pub const LIGHT_SET_COEFFICIENT: u32 = 0x006a_a920;
pub const GET_SCENARIO_FLAG: u32 = 0x0078_7360;
pub const LIGHT_SET_FLAG: u32 = 0x006a_a650;
pub const LIGHT_SET_MAP_NAME: u32 = 0x006a_b0d0;
pub const LIGHT_BALANCE_GLOW: u32 = 0x006a_7550;
pub const STRSTR_SLOT: u32 = crate::h3_glue15::STRSTR_SLOT;
pub const NO_GLOW_BALANCE: u32 = 0x00b4_2670;

/// The magic VFX texture load: `fiosUnifyFilename`, the magic folder,
/// `Array_Reserve_00667e00(this, n)`, `LocalizationManager_GetInstance`,
/// `PhyreScene_isSpecialMagicId_0065efd0(id)` (stdcall),
/// `TOGetFFXLangStr`, `sprintf`'s slot and the two list formats,
/// `graphicIsMagicFileAsyncLoadAvailable`,
/// `PhyreScene_loadMagicFileVFXTextureAsync(this, kind, path)` and
/// `PhyreScene_loadVFXTexture(this, kind, path, 0)`.
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
pub const MAGIC_DIRECTORY: u32 = 0x00b4_3b4c;
pub const ARRAY_RESERVE_667E00: u32 = 0x0066_7e00;
pub const LOCALIZATION_INSTANCE: u32 = 0x006d_b070;
pub const IS_SPECIAL_MAGIC: u32 = 0x0065_efd0;
pub const LANGUAGE_NAME: u32 = 0x008a_c300;
pub const SPRINTF_SLOT: u32 = crate::glue9::SPRINTF;
pub const TEX_LIST_FORMAT: u32 = 0x00b4_3b70;
pub const TEX_LIST_LANGUAGE_FORMAT: u32 = 0x00b4_3b88;
pub const MAGIC_ASYNC_AVAILABLE: u32 = 0x0064_1c60;
pub const LOAD_MAGIC_FILE_ASYNC: u32 = 0x0066_4d10;
pub const LOAD_VFX_TEXTURE: u32 = 0x0066_5770;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The function in the import slot `slot`.
    fn import_at(h: &impl Host, slot: u32) -> u32 {
        h.read(h.rebase(slot))
    }

    /// `PhyreScene_setupCharacterLighting_0066eb20(this, flag)` (`ret 4`):
    /// with the map's camera (`+0x4f8`), the character lighting takes its
    /// four intensities (`+0x120`), rotations (`+0x130`, `+0x140`) and
    /// the tweak utility its four coefficients (`+0x120` again); level
    /// 0x12ea before the scenario flag sets them all to 1. Then the
    /// lighting gets `flag` (`0x006aa650`) and the map name (`+0xfaf0`,
    /// low bit a tag), and with `flag` (its low byte) the glow balance
    /// unless the name holds the string at `0x00b42670`.
    pub fn scene_setup_character_lighting(h: &mut impl Host, this: u32, flag: u32) {
        let map: u32 = h.read(this + 0x4f8);
        if map == 0 {
            return;
        }
        let light = h.call_sym(LIGHT_INSTANCE, Conv::Cdecl, &[]);
        for k in 0..4u32 {
            let v = x87::round_trip(h.read(map + 0x120 + 4 * k)).to_bits();
            h.call_sym(LIGHT_SET_INTENSITY, Conv::Thiscall(light), &[k, v]);
        }
        for k in 0..4u32 {
            let x = x87::round_trip(h.read(map + 0x130 + 4 * k)).to_bits();
            let y = x87::round_trip(h.read(map + 0x140 + 4 * k)).to_bits();
            h.call_sym(LIGHT_SET_ROTATION, Conv::Thiscall(light), &[k, x, y]);
        }
        let tweak = h.call_sym(LIGHT_TWEAK_INSTANCE, Conv::Cdecl, &[]);
        for k in 0..4u32 {
            let v = x87::round_trip(h.read(map + 0x120 + 4 * k)).to_bits();
            h.call_sym(LIGHT_SET_COEFFICIENT, Conv::Thiscall(tweak), &[k, v]);
        }
        if h.read::<u32>(this + 0xfaf4) == 0x12ea
            && h.call_sym(GET_SCENARIO_FLAG, Conv::Cdecl, &[])
                .to_le_bytes()[..2]
                == [0, 0]
        {
            let one = 1f32.to_bits();
            for k in 0..4u32 {
                h.call_sym(LIGHT_SET_INTENSITY, Conv::Thiscall(light), &[k, one]);
                h.call_sym(LIGHT_SET_COEFFICIENT, Conv::Thiscall(tweak), &[k, one]);
            }
        }
        h.call_sym(LIGHT_SET_FLAG, Conv::Thiscall(light), &[flag]);
        let tag = u32::from(h.read::<u8>(this + 0xfaf0) & 1);
        let name = h.read::<u32>(this + 0xfaf0).wrapping_sub(tag);
        h.call_sym(LIGHT_SET_MAP_NAME, Conv::Thiscall(light), &[name]);
        if flag.to_le_bytes()[0] == 0 {
            return;
        }
        if name != 0 {
            let strstr = import_at(h, STRSTR_SLOT);
            let key = h.rebase(NO_GLOW_BALANCE);
            if h.call(strstr, Conv::Cdecl, &[name, key]) != 0 {
                return;
            }
        }
        h.call_sym(LIGHT_BALANCE_GLOW, Conv::Thiscall(light), &[]);
    }

    /// `PhyreScene_loadMagicVFXTexture(this, id)` (`ret 4`; frame: the
    /// count at `ebp - 0x208`, the directory at `ebp - 0x204`, the list's
    /// path at `ebp - 0x104`): nothing (0) for an id already listed
    /// (`+0x28` count, `+0x30` ids); else it takes the last free (0) slot
    /// or is appended (grown by an eighth and 2). Without a magic file
    /// name (`+0xfafc`, low bit a tag) `+0x24` = 1 and 5; else the
    /// texture list's path (a language folder for special magic outside
    /// languages 0, 9 and 10) is loaded, asynchronously when that is
    /// available, with `+0x24` = 0. `eax`: the load's.
    pub fn scene_load_magic_vfx_texture(h: &mut impl Host, ebp: u32, this: u32, id: u32) -> u32 {
        let directory = ebp - 0x204;
        let path = ebp - 0x104;
        let root = h.rebase(MAGIC_DIRECTORY);
        h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[root, directory, 0x100]);
        let n: u32 = h.read(this + 0x28);
        h.write(ebp - 0x208, n);
        let mut free = -1i32;
        if n != 0 {
            let ids: u32 = h.read(this + 0x30);
            let mut k = 0u32;
            while k < h.read::<u32>(this + 0x28) {
                let v: u32 = h.read(ids.wrapping_add(4 * k));
                if v == 0 {
                    free = k as i32;
                } else if v == id {
                    return 0;
                }
                k += 1;
            }
        }
        if free >= 0 {
            let ids: u32 = h.read(this + 0x30);
            h.write(ids.wrapping_add(4 * free as u32), id);
        } else {
            let n = (h.read::<i32>(ebp - 0x208)).wrapping_add(1);
            h.write(this + 0x28, n);
            if n > h.read::<i32>(this + 0x2c) {
                let grow = n.wrapping_add(2).wrapping_add(n / 8);
                h.call_sym(
                    ARRAY_RESERVE_667E00,
                    Conv::Thiscall(this + 0x28),
                    &[grow as u32],
                );
            }
            let n: u32 = h.read(this + 0x28);
            let ids: u32 = h.read(this + 0x30);
            let at = ids.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4);
            if at != 0 {
                h.write(at, id);
            }
        }
        let tag = u32::from(h.read::<u8>(this + 0xfafc) & 1);
        let file = h.read::<u32>(this + 0xfafc).wrapping_sub(tag);
        if file == 0 {
            h.write(this + 0x24, 1u8);
            return 5;
        }
        let sprintf = import_at(h, SPRINTF_SLOT);
        let mut special = true;
        for skip in [0u32, 9, 10] {
            let manager = h.call_sym(LOCALIZATION_INSTANCE, Conv::Cdecl, &[]);
            if h.read::<u32>(manager + 4) == skip {
                special = false;
                break;
            }
        }
        if special {
            special = h.call_sym(IS_SPECIAL_MAGIC, Conv::Stdcall, &[id]) & 0xff != 0;
        }
        if special {
            let tag = u32::from(h.read::<u8>(this + 0xfafc) & 1);
            let file = h.read::<u32>(this + 0xfafc).wrapping_sub(tag);
            let language = h.call_sym(LANGUAGE_NAME, Conv::Cdecl, &[]);
            let format = h.rebase(TEX_LIST_LANGUAGE_FORMAT);
            h.call(
                sprintf,
                Conv::Cdecl,
                &[path, format, directory, file, language],
            );
        } else {
            let tag = u32::from(h.read::<u8>(this + 0xfafc) & 1);
            let file = h.read::<u32>(this + 0xfafc).wrapping_sub(tag);
            let format = h.rebase(TEX_LIST_FORMAT);
            h.call(sprintf, Conv::Cdecl, &[path, format, directory, file]);
        }
        h.write(this + 0x24, 0u8);
        if h.call_sym(MAGIC_ASYNC_AVAILABLE, Conv::Cdecl, &[]) != 0 {
            h.call_sym(LOAD_MAGIC_FILE_ASYNC, Conv::Thiscall(this), &[2, path])
        } else {
            h.call_sym(LOAD_VFX_TEXTURE, Conv::Thiscall(this), &[2, path, 0])
        }
    }

    /// `PhyreScene_UpdateTextureVideo(this)`: the four decoded-frame
    /// records (`+0xd68`, 0x18 bytes) in use age, and past 16 frames
    /// drop their five buffers; with a texture video (`+0xdd0`): the
    /// pause follows `g_PauseOn` while playing (`+0x10c9`, last state
    /// `+0x10dc4`); while the playback runs, a callback record (the
    /// playback, the three textures, `+0xdc8`, `+0xdcc`) is queued on
    /// the render queue (`+0xd0`), and with `+0xdd1` the playback is
    /// paused (`+0x10ca`) when it should not play and the video's
    /// `+0x10c0` effects (`0x5bc:0x78 + k`) are updated.
    pub fn scene_update_texture_video(h: &mut impl Host, this: u32) {
        for k in 0..4 {
            let record = this + 0xd68 + 0x18 * k;
            let age: i32 = h.read(record);
            if age > 0 {
                let age = age + 1;
                h.write(record, age);
                if age > 0x10 {
                    for b in 1..=5 {
                        let p: u32 = h.read(record + 4 * b);
                        if p != 0 {
                            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
                            h.write(record + 4 * b, 0u32);
                        }
                    }
                    h.write(record, 0u32);
                }
            }
        }
        if h.read::<u8>(this + 0xdd0) == 0 {
            return;
        }
        let playback = this + 0xde8;
        if h.read::<u8>(this + 0x10c9) != 0 {
            let pause = PAUSE_ON.get(h);
            if pause != h.read::<u32>(this + 0x1_0dc4) {
                h.write(this + 0x1_0dc4, pause);
                h.call_sym(VIDEO_TOGGLE_PAUSE, Conv::Thiscall(playback), &[]);
            }
        }
        if h.read::<u8>(this + 0xdd0) == 0 {
            return;
        }
        if h.call_sym(VIDEO_IS_PLAYING, Conv::Thiscall(playback), &[])
            .to_le_bytes()[0]
            == 0
        {
            return;
        }
        let record = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x18]);
        if record != 0 {
            for k in 0..6 {
                h.write(record + 4 * k, 0u32);
            }
        }
        h.write(record, playback);
        for (k, off) in (1u32..).zip([0xdd8, 0xddc, 0xde0]) {
            let texture: u32 = h.read(this + off);
            h.write(record.wrapping_add(4 * k), texture);
        }
        h.write(record.wrapping_add(0x10), this + 0xdc8);
        h.write(record.wrapping_add(0x14), this + 0xdcc);
        let queue: u32 = h.read(this + 0xd0);
        let callback = h.rebase(TEXTURE_VIDEO_CALLBACK);
        h.call_sym(QUEUE_CALLBACK, Conv::Thiscall(queue), &[callback, record]);
        if h.read::<u8>(this + 0xdd1) == 0 {
            return;
        }
        let playing = h.read::<u8>(this + 0x10c9) != 0;
        let paused = h.read::<u8>(this + 0x10ca) != 0;
        if playing == paused {
            h.call_sym(VIDEO_TOGGLE_PAUSE, Conv::Thiscall(playback), &[]);
            h.write(this + 0x10ca, u8::from(!playing));
        }
        if h.read::<u8>(this + 0x10c9) == 0 || h.read::<i32>(this + 0x10c0) <= 0 {
            return;
        }
        let mut k = 0x78i32;
        loop {
            let lo = k as u32;
            let hi = 0x5bcu32.wrapping_add(u32::from(k < 0).wrapping_neg());
            h.call_sym(VFX_UPDATE, Conv::Cdecl, &[lo, hi]);
            h.call_sym(DYNAMIC_GEOMETRY_UPDATE_TEXTURE, Conv::Cdecl, &[lo, hi]);
            k += 1;
            if k - 0x78 >= h.read::<i32>(this + 0x10c0) {
                break;
            }
        }
    }

    /// Makes the singleton in `global` (`size` bytes, `ctor`) when there
    /// is none; the instance.
    fn singleton(h: &mut impl Host, global: Global<u32>, size: u32, ctor: u32) -> u32 {
        let have = global.get(h);
        if have != 0 {
            return have;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[size]);
        let made = if p == 0 {
            0
        } else {
            h.call_sym(ctor, Conv::Thiscall(p), &[])
        };
        global.set(h, made);
        made
    }

    /// `graphicInitialize()`: between its two messages, every graphics
    /// singleton is made in the original's order (the first one set up
    /// at once, the delegation helper, the loading screen and the scene
    /// made here when missing, the scene's post-process set up), the
    /// monster names read and the cloth heap made (1 MB, 16-aligned);
    /// the "graphics up" flag is set. Returns 0.
    pub fn graphic_initialize(h: &mut impl Host) -> u32 {
        let begin = h.rebase(INIT_BEGIN);
        h.call_sym(TTY_WRITE, Conv::Cdecl, &[10, begin]);
        h.call_sym(INIT_FIRST, Conv::Cdecl, &[]);
        let first = singleton(h, FIRST_SINGLETON, 0x238, FIRST_SINGLETON_CTOR);
        h.call_sym(FIRST_SINGLETON_SETUP, Conv::Thiscall(first), &[]);
        for f in CREATORS_A {
            h.call_sym(f, Conv::Cdecl, &[]);
        }
        singleton(
            h,
            RENDER_DELEGATION_HELPER,
            0x208,
            RENDER_DELEGATION_HELPER_CTOR,
        );
        for f in CREATORS_B {
            h.call_sym(f, Conv::Cdecl, &[]);
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x44]);
        let screen = if p == 0 {
            0
        } else {
            h.call_sym(LOADING_SCREEN_CTOR, Conv::Thiscall(p), &[])
        };
        LOADING_SCREEN.set(h, screen);
        h.call_sym(INIT_AFTER_LOADING_SCREEN, Conv::Cdecl, &[]);
        let scene = singleton(h, SCENE, SCENE_SIZE, SCENE_CTOR);
        h.call_sym(SCENE_INIT_POST_PROCESS, Conv::Thiscall(scene), &[]);
        h.call_sym(READ_MONSTER_NAMES, Conv::Cdecl, &[]);
        let base = h.call_sym(ALIGNED_MALLOC, Conv::Cdecl, &[0x10_0000, 0x10]);
        CLOTH_BASE.set(h, base);
        let mspace = h.call_sym(CREATE_MSPACE, Conv::Cdecl, &[base, 0x10_0000, 1]);
        CLOTH_MSPACE.set(h, mspace);
        let end = h.rebase(INIT_END);
        h.call_sym(TTY_WRITE, Conv::Cdecl, &[10, end]);
        GRAPHICS_UP.set(h, 1);
        0
    }

    /// `fistp qword` under truncation, its low word (the "integer
    /// indefinite" for NaN and out-of-range values).
    fn fistp_trunc_low(x: f64) -> u32 {
        const LIMIT: f64 = 9_223_372_036_854_775_808.0;
        if x.is_nan() || x >= LIMIT || x < -LIMIT {
            0
        } else {
            #[allow(clippy::cast_possible_truncation)] // in range: checked
            let v = x.trunc() as i64;
            v as u32
        }
    }

    /// `PostProcessManager_setNon16By9Viewport(renderer, keep)` (stdcall;
    /// `slots` is the address of the argument slots: `keep`'s ends with
    /// the last ratio stored, its upper half the control word (`fnstcw`)
    /// in the fitted cases; the frame's words are left out): unless `keep`,
    /// the window (unsigned width and height, as floats) is fitted to the
    /// target aspect (`R = w / h`): equal, the whole window; narrower,
    /// letterboxed (`S = R * 9/16`, the height `h * S`, centred); wider,
    /// pillarboxed (`S = A * (1 / R)`, the width `w * S`). Both the
    /// renderer's viewport (`fistp` toward zero) and the Iggy menu's
    /// rectangle (`ftol`, then the window size) are set. `eax`: the
    /// Iggy call's; 0 with `keep` (the original leaves its caller's).
    pub fn post_set_non_16_by_9_viewport(
        h: &mut impl Host,
        slots: u32,
        renderer: u32,
        keep: u32,
    ) -> u32 {
        if keep != 0 {
            return 0;
        }
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let w: u32 = h.read(app + 0x26c);
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let ht: u32 = h.read(app + 0x270);
        // fild, plus 2^32 for a "negative" (unsigned) size, fstp dword
        let fix = TWO_POW_32.get(h);
        let to_f32 = |v: u32| -> u32 {
            let x = f64::from(v as i32);
            let x = if (v as i32) < 0 {
                x87::add(x, f64::from(fix))
            } else {
                x
            };
            x87::store(x)
        };
        let width = to_f32(w);
        let height = to_f32(ht);
        let wf = x87::load(width);
        let hf = x87::load(height);
        let ratio = x87::store(x87::div(wf, hf));
        let r = x87::load(ratio);
        let a = x87::load(TARGET_ASPECT.get(h).to_bits());
        #[allow(clippy::float_cmp)] // fucomp: exact equality
        if a == r {
            h.write(slots + 4, ratio);
            h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &[0, 0, w, ht]);
            let iggy = IGGY_MENU_MANAGER.get(h);
            return h.call_sym(
                IGGY_SET_VIEWPORT_RECT,
                Conv::Thiscall(iggy),
                &[0, 0, w, ht, w, ht],
            );
        }
        let half = HALF.get(h);
        if a > r {
            let s = x87::store(x87::mul(r, NINE_SIXTEENTHS.get(h)));
            h.write(slots + 4, s);
            h.write(slots + 6, CONTROL_WORD);
            let s = x87::load(s);
            let y = x87::ftol(&*h, x87::mul(x87::mul(x87::sub(1.0, s), hf), half));
            let d = x87::mul(hf, s);
            let fh = fistp_trunc_low(d);
            h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &[0, y, w, fh]);
            let fh = x87::ftol(&*h, d);
            let iggy = IGGY_MENU_MANAGER.get(h);
            return h.call_sym(
                IGGY_SET_VIEWPORT_RECT,
                Conv::Thiscall(iggy),
                &[0, y, w, fh, w, ht],
            );
        }
        let s = x87::store(x87::mul(a, x87::div(1.0, r)));
        h.write(slots + 4, s);
        h.write(slots + 6, CONTROL_WORD);
        let s = x87::load(s);
        let x = x87::ftol(&*h, x87::mul(x87::mul(x87::sub(1.0, s), wf), half));
        let d = x87::mul(wf, s);
        let fw = fistp_trunc_low(d);
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &[x, 0, fw, ht]);
        let fw = x87::ftol(&*h, d);
        let iggy = IGGY_MENU_MANAGER.get(h);
        h.call_sym(
            IGGY_SET_VIEWPORT_RECT,
            Conv::Thiscall(iggy),
            &[x, 0, fw, ht, w, ht],
        )
    }
}
