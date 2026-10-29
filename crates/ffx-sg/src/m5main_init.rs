//! M5 `main`: `FFXApplication`'s other virtual methods (start-up, the
//! input polls, scene entry and exit, render, shutdown), the start-up
//! helpers they call (the systems, the file system and its language
//! prefix), the settings refresh, the device-reset message and the
//! player's animated-material forwarder.
//!
//! The application's virtual methods are `thiscall` on the
//! `FFXApplication` (vtable `0x00b0d998`); most end in a tail jump to the
//! `PApplication` base method, whose `eax` they return.

/// The originals over game memory.
#[allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::verbose_bit_mask
)] // registers, points; the `al` test as written
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `SaveDataTtyWrite2(fd, fmt, ...)` (cdecl, varargs).
    pub const TTY_WRITE: u32 = 0x0064_8850;
    /// `SKS: exitApplication begin!` / `end!` (`.rdata`).
    pub const MSG_EXIT_BEGIN: u32 = 0x00b0_db0c;
    pub const MSG_EXIT_END: u32 = 0x00b0_db2c;
    /// `FreeBlock_0067ac40()` (cdecl).
    pub const FREE_BLOCK: u32 = 0x0067_ac40;
    /// `graphicFinalize()` (cdecl).
    pub const GRAPHIC_FINALIZE: u32 = 0x0064_03e0;
    /// `jmp TkClearVsyncHandler` (cdecl).
    pub const CLEAR_VSYNC: u32 = 0x0082_2750;
    /// `TrophyGetError()` (cdecl).
    pub const TROPHY_GET_ERROR: u32 = 0x0064_8ff0;
    /// The byte that makes shutdown finalize FMOD even when the trophies
    /// report an error (set by `initApplication` once FMOD is up).
    pub const FMOD_UP: u32 = 0x00c8_fae8;
    /// `iFmodFinalize()`, `iFmodInitialize()` (cdecl).
    pub const FMOD_FINALIZE: u32 = 0x0067_9fb0;
    pub const FMOD_INITIALIZE: u32 = 0x0067_9fc0;
    /// `SyncManager_shutdown_0067a900()`, `iSyncInitialize()` (cdecl).
    pub const SYNC_SHUTDOWN: u32 = 0x0067_a900;
    pub const SYNC_INITIALIZE: u32 = 0x0067_a960;
    /// `fiosFinalize()` (cdecl).
    pub const FIOS_FINALIZE: u32 = 0x0067_9590;
    /// The `PApplication` base methods the overrides tail-jump to
    /// (thiscall, no stack words).
    pub const BASE_EXIT_APPLICATION: u32 = 0x0062_75c0;
    pub const BASE_EXIT_SCENE: u32 = 0x0062_75d0;
    pub const BASE_HANDLE_INPUTS: u32 = 0x0062_9800;
    pub const BASE_INIT_SCENE: u32 = 0x0062_9820;
    pub const BASE_PRE_PHYRE_INIT: u32 = 0x0062_ac70;
    /// `PApplication::setWindowTitle(this; title)` (thiscall, one word).
    pub const SET_WINDOW_TITLE: u32 = 0x0062_cf60;

    /// `FFXApplication::exitApplication(this)` (`0x0042f600`, thiscall):
    /// the shutdown in order (the trophy block, graphics, the vsync
    /// handler, FMOD unless the trophies report an error and FMOD never
    /// came up, the sync manager, FIOS), between two log lines; then the
    /// base method, whose `eax` (0) it returns.
    pub fn exit_application(h: &mut impl Host, this: u32) -> u32 {
        let msg = h.rebase(MSG_EXIT_BEGIN);
        h.call_sym(TTY_WRITE, Conv::Cdecl, &[0xa, msg]);
        h.call_sym(FREE_BLOCK, Conv::Cdecl, &[]);
        h.call_sym(GRAPHIC_FINALIZE, Conv::Cdecl, &[]);
        h.call_sym(CLEAR_VSYNC, Conv::Cdecl, &[]);
        let err = h.call_sym(TROPHY_GET_ERROR, Conv::Cdecl, &[]);
        if err == 0 || h.read::<u8>(h.rebase(FMOD_UP)) != 0 {
            h.call_sym(FMOD_FINALIZE, Conv::Cdecl, &[]);
        }
        h.call_sym(SYNC_SHUTDOWN, Conv::Cdecl, &[]);
        h.call_sym(FIOS_FINALIZE, Conv::Cdecl, &[]);
        let msg = h.rebase(MSG_EXIT_END);
        h.call_sym(TTY_WRITE, Conv::Cdecl, &[0xa, msg]);
        h.call_sym(BASE_EXIT_APPLICATION, Conv::Thiscall(this), &[])
    }

    /// `graphicFieldMapShutdown`, `graphicFieldMapUnload`,
    /// `graphicVFXUnloadLevelData`, `graphicExitScene_006401c0` (cdecl).
    pub const EXIT_SCENE_STEPS: [u32; 4] = [0x0064_03b0, 0x0064_03c0, 0x0064_5860, 0x0064_01c0];

    /// `FFXApplication::exitScene(this)` (`0x0042f660`, thiscall): the
    /// four graphics steps, then the base method's `eax` (0).
    pub fn exit_scene(h: &mut impl Host, this: u32) -> u32 {
        for f in EXIT_SCENE_STEPS {
            h.call_sym(f, Conv::Cdecl, &[]);
        }
        h.call_sym(BASE_EXIT_SCENE, Conv::Thiscall(this), &[])
    }

    /// `Input_UpdateAll_00630e80()` (cdecl).
    pub const INPUT_UPDATE_ALL: u32 = 0x0063_0e80;

    /// `FFXApplication::handleInputs(this)` (`0x0042f770`, thiscall):
    /// the input update unless `+0x3ad` (the utility mode) is set, then
    /// the base method's `eax` (0).
    pub fn handle_inputs(h: &mut impl Host, this: u32) -> u32 {
        if h.read::<u8>(this + 0x3ad) == 0 {
            h.call_sym(INPUT_UPDATE_ALL, Conv::Cdecl, &[]);
        }
        h.call_sym(BASE_HANDLE_INPUTS, Conv::Thiscall(this), &[])
    }

    /// `FUN_006250d0(flag)` (cdecl): stores the byte at `0x00cc9cf4`.
    pub const SET_POLL_FLAG: u32 = 0x0062_50d0;
    /// `PInputMapper::update(this)`, `PInputMapper::latchButtons(this)`
    /// (thiscall).
    pub const MAPPER_UPDATE: u32 = 0x0062_e040;
    pub const MAPPER_LATCH: u32 = 0x0062_8320;
    /// `inputODUpdate()` (cdecl).
    pub const INPUT_OD_UPDATE: u32 = 0x0063_0d50;

    /// The poll both input callbacks share: the flag byte 1, then the
    /// input mapper at `+0x2b8` updated and latched (read again for each).
    fn poll_mapper(h: &mut impl Host, this: u32) {
        h.call_sym(SET_POLL_FLAG, Conv::Cdecl, &[1]);
        let mapper: u32 = h.read(this + 0x2b8);
        h.call_sym(MAPPER_UPDATE, Conv::Thiscall(mapper), &[]);
        let mapper: u32 = h.read(this + 0x2b8);
        h.call_sym(MAPPER_LATCH, Conv::Thiscall(mapper), &[]);
    }

    /// `FFXApplication::pollInputAsync(this)` (`0x0042f790`, thiscall;
    /// the controller thread's callback): the mapper poll, then
    /// `inputODUpdate`'s `eax`.
    pub fn poll_input_async(h: &mut impl Host, this: u32) -> u32 {
        poll_mapper(h, this);
        h.call_sym(INPUT_OD_UPDATE, Conv::Cdecl, &[])
    }

    /// `FFXApplication::pollInputSync(this)` (`0x0042f7c0`, thiscall):
    /// the mapper poll, then virtual slot 7 (`handleInputs`) of `this`
    /// and its `eax`.
    pub fn poll_input_sync(h: &mut impl Host, this: u32) -> u32 {
        poll_mapper(h, this);
        h.call_virtual(this, 7, &[])
    }

    /// `SaveDataManager_create_006f1c10`, `SetUpDefaultSaveFolder`
    /// (cdecl).
    pub const SAVE_MANAGER_CREATE: u32 = 0x006f_1c10;
    pub const SAVE_FOLDER: u32 = 0x006f_0330;
    /// `g_SteamReady`.
    pub const STEAM_READY: u32 = 0x00cc_9cf5;
    /// `StatsAndAchievements_Create`, `TrophyManager_create_0067ac10`,
    /// `SaveDataManager_instance` (cdecl).
    pub const STATS_CREATE: u32 = 0x006f_1430;
    pub const TROPHY_CREATE: u32 = 0x0067_ac10;
    pub const SAVE_MANAGER_INSTANCE: u32 = 0x006f_2070;
    /// `SaveDataManager::initSysModules(this)` (thiscall).
    pub const SAVE_INIT_SYS_MODULES: u32 = 0x006f_2060;

    /// `Game_InitSystems_00647fe0()` (cdecl): the save manager and its
    /// folder, the Steam stats when Steam is up, the trophy manager, then
    /// the save manager's `initSysModules` and its `eax` (0).
    pub fn init_systems(h: &mut impl Host) -> u32 {
        h.call_sym(SAVE_MANAGER_CREATE, Conv::Cdecl, &[]);
        h.call_sym(SAVE_FOLDER, Conv::Cdecl, &[]);
        if h.read::<u8>(h.rebase(STEAM_READY)) != 0 {
            h.call_sym(STATS_CREATE, Conv::Cdecl, &[]);
        }
        h.call_sym(TROPHY_CREATE, Conv::Cdecl, &[]);
        let mgr = h.call_sym(SAVE_MANAGER_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(SAVE_INIT_SYS_MODULES, Conv::Thiscall(mgr), &[])
    }

    /// `PStreamFile::SetStreamPrefix(prefix)`, `GetStreamPrefix()`
    /// (cdecl).
    pub const SET_STREAM_PREFIX: u32 = 0x0060_7d40;
    pub const GET_STREAM_PREFIX: u32 = 0x0060_7d30;
    /// `BigFileStream_createInstance()`, `BigFileStream_get()` (cdecl).
    pub const BIG_FILE_CREATE: u32 = 0x0061_b590;
    pub const BIG_FILE_GET: u32 = 0x0061_bdb0;
    /// `BigFileStream::setStreamPrefix(this; prefix)`,
    /// `BigFileStream::registerBigFile(this; name)` -> `al` (thiscall,
    /// one word).
    pub const BIG_FILE_SET_PREFIX: u32 = 0x0061_c3a0;
    pub const BIG_FILE_REGISTER: u32 = 0x0061_c150;
    /// The stream prefix, the big file's prefix and its name (`.rdata`).
    pub const STR_STREAM_PREFIX: u32 = 0x00b4_4d60;
    pub const STR_BIG_PREFIX: u32 = 0x00b4_4d6c;
    pub const STR_BIG_NAME: u32 = 0x00b4_4d78;
    /// ATL's string manager (its vtable slot 3 gives the empty string's
    /// data block; the characters follow its 16-byte header).
    pub const STRING_MANAGER: u32 = 0x00c6_4bf0;
    /// `AtlFindStringResourceInstance_0064f970(id, lang)` (cdecl),
    /// `CStringW::LoadString(this; instance, id)` (thiscall, two words).
    pub const FIND_STRING_RESOURCE: u32 = 0x0064_f970;
    pub const LOAD_STRING: u32 = 0x0065_2310;
    /// The missing-data message's string resource.
    pub const MSG_NO_DATA: u32 = 0x7836;
    /// `USER32!ShowCursor`, `USER32!MessageBoxW` import slots (stdcall,
    /// one and four words).
    pub const SHOW_CURSOR_IAT: u32 = 0x00b0_c688;
    pub const MESSAGE_BOX_IAT: u32 = 0x00b0_c684;
    /// The message box's caption (`.rdata`, UTF-16).
    pub const STR_CAPTION: u32 = 0x00b3_f9fc;
    /// `ResetToLauncher()` (cdecl).
    pub const RESET_TO_LAUNCHER: u32 = 0x0042_f4d0;
    /// `LocalizationManager_create_006dafc0()`,
    /// `LocalizationManager_GetInstance()` (cdecl),
    /// `LocalizationManager::Initialize(this)` (thiscall).
    pub const LOC_CREATE: u32 = 0x006d_afc0;
    pub const LOC_INSTANCE: u32 = 0x006d_b070;
    pub const LOC_INITIALIZE: u32 = 0x006d_b090;
    /// The FIOS prefixes by the manager's first word (1, 2, 3; anything
    /// else the first): `PCSG`, `PCSE`, `PCSB`, `PCSH`.
    pub const STR_FIOS: [u32; 4] = [0x00b4_4d8c, 0x00b4_4d98, 0x00b4_4da4, 0x00b4_4db0];
    /// `PhyreFIOS_Initialize(&params)` (cdecl).
    pub const FIOS_INITIALIZE: u32 = 0x006f_9b60;
    /// `Game_InitFileSystem`'s frame below `ebp`: the FIOS parameters at
    /// `ebp - 0x10` (prefix, stream prefix, a flag byte) and the message
    /// string at `ebp - 4`.
    pub const INIT_FS_FRAME: u32 = 0x10;

    /// `Game_InitFileSystem()` (`0x006795b0`, cdecl), with its frame at
    /// `ebp` ([`INIT_FS_FRAME`] bytes below it): the stream prefixes and
    /// the big file; when the big file can't be registered, the missing
    /// data message (`MessageBoxW`, cursor shown) and the launcher reset,
    /// the message string released. Then the localization manager and
    /// FIOS with the language's prefix. Returns `PhyreFIOS_Initialize`'s
    /// `eax`.
    pub fn init_file_system(h: &mut impl Host, ebp: u32) -> u32 {
        let params = ebp - 0x10;
        h.write(params, 0u32);
        h.write(params + 4, 0u32);
        h.write(params + 8, 1u8);
        let s = h.rebase(STR_STREAM_PREFIX);
        h.call_sym(SET_STREAM_PREFIX, Conv::Cdecl, &[s]);
        h.call_sym(BIG_FILE_CREATE, Conv::Cdecl, &[]);
        let big = h.call_sym(BIG_FILE_GET, Conv::Cdecl, &[]);
        let s = h.rebase(STR_BIG_PREFIX);
        h.call_sym(BIG_FILE_SET_PREFIX, Conv::Thiscall(big), &[s]);
        let big = h.call_sym(BIG_FILE_GET, Conv::Cdecl, &[]);
        let s = h.rebase(STR_BIG_NAME);
        let ok = h.call_sym(BIG_FILE_REGISTER, Conv::Thiscall(big), &[s]);
        if ok & 0xff == 0 {
            no_data_message(h, ebp - 4);
        }
        h.call_sym(LOC_CREATE, Conv::Cdecl, &[]);
        let loc = h.call_sym(LOC_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(LOC_INITIALIZE, Conv::Thiscall(loc), &[]);
        let loc = h.call_sym(LOC_INSTANCE, Conv::Cdecl, &[]);
        let lang: u32 = h.read(loc);
        let prefix = match lang {
            1..=3 => STR_FIOS[lang as usize],
            _ => STR_FIOS[0],
        };
        h.write(params, h.rebase(prefix));
        h.write(params + 8, 1u8);
        let stream = h.call_sym(GET_STREAM_PREFIX, Conv::Cdecl, &[]);
        h.write(params + 4, stream);
        h.call_sym(FIOS_INITIALIZE, Conv::Cdecl, &[params])
    }

    /// The missing-data path: an ATL `CStringW` at `cell` (the empty
    /// string, then the resource loaded into it), shown in a message box;
    /// the launcher reset; the string's reference dropped and its block
    /// freed through its manager (slot 1) at zero.
    fn no_data_message(h: &mut impl Host, cell: u32) {
        let mgr = h.rebase(STRING_MANAGER);
        let nil = h.call_virtual(mgr, 3, &[]);
        let mut chars = nil.wrapping_add(0x10);
        h.write(cell, chars);
        let inst = h.call_sym(FIND_STRING_RESOURCE, Conv::Cdecl, &[MSG_NO_DATA, 0]);
        if inst != 0 {
            h.call_sym(LOAD_STRING, Conv::Thiscall(cell), &[inst, MSG_NO_DATA]);
            chars = h.read(cell);
        }
        let show: u32 = h.read(h.rebase(SHOW_CURSOR_IAT));
        h.call(show, Conv::Stdcall, &[1]);
        let message_box: u32 = h.read(h.rebase(MESSAGE_BOX_IAT));
        let caption = h.rebase(STR_CAPTION);
        h.call(message_box, Conv::Stdcall, &[0, chars, caption, 0x10]);
        h.call_sym(RESET_TO_LAUNCHER, Conv::Cdecl, &[]);
        let data = chars.wrapping_sub(0x10);
        let refs = data.wrapping_add(0xc);
        let n = h.read::<u32>(refs).wrapping_sub(1);
        h.write(refs, n);
        if n.cast_signed() <= 0 {
            let owner: u32 = h.read(data);
            h.call_virtual(owner, 1, &[data]);
        }
    }

    /// `SKS: initApplication` log lines (`.rdata`).
    pub const MSG_INIT: [u32; 3] = [0x00b0_da74, 0x00b0_dac8, 0x00b0_dae8];
    /// `"_ECalm"`: the first argument that starts at the Eternal Calm.
    pub const STR_ECALM: u32 = 0x00b0_da90;
    /// The window titles: the Eternal Calm's and the game's.
    pub const TITLE_ECALM: u32 = 0x00b0_da98;
    pub const TITLE_GAME: u32 = 0x00b0_dab8;
    /// `Game_InitFileSystem()` (cdecl).
    pub const INIT_FILE_SYSTEM: u32 = 0x0067_95b0;
    /// `InitSysModuleEncoding`, `Game_InitSystems_00647fe0`,
    /// `SaveData_Init`, `graphicInitialize`,
    /// `Virtuos::InputManager::getInstance` (cdecl).
    pub const INIT_ENCODING: u32 = 0x0064_6910;
    pub const INIT_SYSTEMS: u32 = 0x0064_7fe0;
    pub const SAVE_DATA_INIT: u32 = 0x0064_7fb0;
    pub const GRAPHIC_INITIALIZE: u32 = 0x0064_19d0;
    pub const INPUT_MANAGER: u32 = 0x0063_1300;
    /// `Input_BuildPadMap(this)` (thiscall on the input manager).
    pub const BUILD_PAD_MAP: u32 = 0x0063_15c0;

    /// `FFXApplication::initApplication(this; argv, argc)` (`0x0042f7f0`,
    /// thiscall, `ret 8`): keeps `argv` and `argc` (`+0x3a4`, `+0x3a8`);
    /// the window title (the Eternal Calm's when `argv[1]` is `"_ECalm"`);
    /// `+0x3b0` = 0.0; the file system, the encoding, the systems, the
    /// saves, graphics and the pad map; FMOD (and [`FMOD_UP`]) unless the
    /// trophies report an error; the sync manager. Returns 0.
    pub fn init_application(h: &mut impl Host, this: u32, argv: u32, argc: u32) -> u32 {
        h.write(this + 0x3a4, argv);
        h.write(this + 0x3a8, argc);
        let msg = h.rebase(MSG_INIT[0]);
        h.call_sym(TTY_WRITE, Conv::Cdecl, &[0xa, msg]);
        let mut title = TITLE_GAME;
        if h.read::<u32>(this + 0x3a8).cast_signed() > 1 {
            let argv: u32 = h.read(this + 0x3a4);
            let arg: u32 = h.read(argv + 4);
            if same_string(h, arg, h.rebase(STR_ECALM)) {
                title = TITLE_ECALM;
            }
        }
        let title = h.rebase(title);
        h.call_sym(SET_WINDOW_TITLE, Conv::Thiscall(this), &[title]);
        h.write(this + 0x3b0, 0u32);
        h.call_sym(INIT_FILE_SYSTEM, Conv::Cdecl, &[]);
        for m in [MSG_INIT[1], MSG_INIT[2]] {
            let msg = h.rebase(m);
            h.call_sym(TTY_WRITE, Conv::Cdecl, &[0xa, msg]);
        }
        h.call_sym(INIT_ENCODING, Conv::Cdecl, &[]);
        h.call_sym(INIT_SYSTEMS, Conv::Cdecl, &[]);
        h.call_sym(SAVE_DATA_INIT, Conv::Cdecl, &[]);
        h.call_sym(GRAPHIC_INITIALIZE, Conv::Cdecl, &[]);
        let input = h.call_sym(INPUT_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(BUILD_PAD_MAP, Conv::Thiscall(input), &[]);
        if h.call_sym(TROPHY_GET_ERROR, Conv::Cdecl, &[]) == 0 {
            h.call_sym(FMOD_INITIALIZE, Conv::Cdecl, &[]);
            h.write(h.rebase(FMOD_UP), 1u8);
        }
        h.call_sym(SYNC_INITIALIZE, Conv::Cdecl, &[]);
        0
    }

    /// The inlined `strcmp(a, b) == 0`.
    fn same_string(h: &impl Host, a: u32, b: u32) -> bool {
        let mut k = 0;
        loop {
            let x: u8 = h.read(a + k);
            if x != h.read::<u8>(b + k) {
                return false;
            }
            if x == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// `graphicInitScene()` (cdecl).
    pub const GRAPHIC_INIT_SCENE: u32 = 0x0064_16c0;

    /// `FFXApplication::initScene(this)` (`0x0042f8e0`, thiscall):
    /// `graphicInitScene`, then the base method's `eax` (0).
    pub fn init_scene(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(GRAPHIC_INIT_SCENE, Conv::Cdecl, &[]);
        h.call_sym(BASE_INIT_SCENE, Conv::Thiscall(this), &[])
    }

    /// The type registrations `prePhyreInit` runs, in order (cdecl).
    pub const REGISTER_TYPES: [u32; 9] = [
        0x0067_2790,
        0x0067_3710,
        0x0067_40e0,
        0x0067_4d10,
        0x0067_6060,
        0x0067_5630,
        0x0067_76c0,
        0x0067_6c90,
        0x0067_8560,
    ];

    /// `FFXApplication::prePhyreInit(this)` (`0x0042f8f0`, thiscall): the
    /// game's nine type registrations, then the base method's `eax` (0).
    pub fn pre_phyre_init(h: &mut impl Host, this: u32) -> u32 {
        for f in REGISTER_TYPES {
            h.call_sym(f, Conv::Cdecl, &[]);
        }
        h.call_sym(BASE_PRE_PHYRE_INIT, Conv::Thiscall(this), &[])
    }

    /// `graphicRenderCaptureAndIggy_00642be0`,
    /// `graphicCharacterInstanceSetClothVertexBuffers`, `graphicRender`,
    /// `VsyncEmu_Wake`, `graphicIsEscMenuActive`,
    /// `graphicCopyPostBuffer_00639d10` (cdecl).
    pub const RENDER_CAPTURE_IGGY: u32 = 0x0064_2be0;
    pub const SET_CLOTH_BUFFERS: u32 = 0x0063_b740;
    pub const GRAPHIC_RENDER: u32 = 0x0064_2bb0;
    pub const VSYNC_WAKE: u32 = 0x0067_9f80;
    pub const IS_ESC_MENU_ACTIVE: u32 = 0x0063_da80;
    pub const COPY_POST_BUFFER: u32 = 0x0063_9d10;
    /// `g_GraphicAbmap` (the sphere grid's render object).
    pub const GRAPHIC_ABMAP: u32 = 0x00cc_c838;

    /// `FFXApplication::render(this)` (`0x0042f930`, thiscall): in
    /// utility mode (`+0x3ad`) only the capture and Iggy; else, unless
    /// the trophies report error 1, the cloth buffers (not while the
    /// sphere grid's state is below 4), the frame, the vsync wake, and
    /// the post buffer copy while the escape menu is up. Returns 0.
    pub fn render(h: &mut impl Host, this: u32) -> u32 {
        if h.read::<u8>(this + 0x3ad) != 0 {
            h.call_sym(RENDER_CAPTURE_IGGY, Conv::Cdecl, &[]);
            return 0;
        }
        if h.call_sym(TROPHY_GET_ERROR, Conv::Cdecl, &[]) == 1 {
            return 0;
        }
        let abmap: u32 = h.read(h.rebase(GRAPHIC_ABMAP));
        if abmap == 0 || h.read::<u32>(abmap + 4).cast_signed() >= 4 {
            h.call_sym(SET_CLOTH_BUFFERS, Conv::Cdecl, &[]);
        }
        h.call_sym(GRAPHIC_RENDER, Conv::Cdecl, &[]);
        h.call_sym(VSYNC_WAKE, Conv::Cdecl, &[]);
        if h.read::<u8>(this + 0x3ad) == 0 && h.call_sym(IS_ESC_MENU_ACTIVE, Conv::Cdecl, &[]) != 0
        {
            h.call_sym(COPY_POST_BUFFER, Conv::Cdecl, &[]);
        }
        0
    }

    /// `g_PhyreScene`.
    pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
    /// `PhyreScene::ApplySettings(this)` (thiscall).
    pub const SCENE_APPLY_SETTINGS: u32 = 0x0066_f010;

    /// `FFXApplication_ApplySettings()` (`0x0042f9b0`, cdecl): the
    /// settings to the scene when there is one. Returns 0.
    pub fn apply_settings(h: &mut impl Host) -> u32 {
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        if scene != 0 {
            h.call_sym(SCENE_APPLY_SETTINGS, Conv::Thiscall(scene), &[]);
        }
        0
    }

    /// `g_hWnd`.
    pub const HWND: u32 = 0x00cc_9ce8;
    /// The word `PostReset` clears (`0x00ccb45c`).
    pub const RESET_PENDING: u32 = 0x00cc_b45c;
    /// `USER32!SendMessageW`'s import slot (stdcall, four words).
    pub const SEND_MESSAGE_IAT: u32 = 0x00b0_c67c;

    /// `PostReset()` (`0x0042fa70`, cdecl): clears [`RESET_PENDING`] and
    /// sends `WM_CLOSE` (0x10) to the window; `SendMessageW`'s `eax`.
    pub fn post_reset(h: &mut impl Host) -> u32 {
        let hwnd: u32 = h.read(h.rebase(HWND));
        h.write(h.rebase(RESET_PENDING), 0u32);
        let f: u32 = h.read(h.rebase(SEND_MESSAGE_IAT));
        h.call(f, Conv::Stdcall, &[hwnd, 0x10, 0, 0])
    }

    /// `Sg_GetPlayer()` (cdecl).
    pub const GET_PLAYER: u32 = 0x0082_d860;
    /// `graphicSetAnimatedMaterialsChar_006440e0(a0..a10)` (cdecl).
    pub const SET_ANIMATED_MATERIALS: u32 = 0x0064_40e0;

    /// `FUN_00829c00(a0..a9)` (`0x00829c00`, cdecl): the player's
    /// animated materials: `a0..a2`, the six floats `a3..a8` (through
    /// the x87 stack, so a signalling NaN arrives quiet), `a9` and the
    /// player's `+0x830`. Returns the forwarder's `eax`, or 0 without a
    /// player.
    pub fn player_animated_materials(h: &mut impl Host, a: [u32; 10]) -> u32 {
        let player = h.call_sym(GET_PLAYER, Conv::Cdecl, &[]);
        if player == 0 {
            return 0;
        }
        let tag: u32 = h.read(player + 0x830);
        let mut w = [0u32; 11];
        w[..3].copy_from_slice(&a[..3]);
        for k in 3..9 {
            w[k] = x87::store(x87::load(a[k]));
        }
        w[9] = a[9];
        w[10] = tag;
        h.call_sym(SET_ANIMATED_MATERIALS, Conv::Cdecl, &w)
    }
}
