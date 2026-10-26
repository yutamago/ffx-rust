//! The render-side glue, forty-fourth batch (M4 port round 11, list
//! `h3-glue-render`): the boot scene set-up.

/// `TrophyManager_Instance()`, its `0x0067ac80(trophy)` (returns 0),
/// `TrophyManager_IsInstallCompleted(trophy)` (`al`) and
/// `TrophyGetError()`; `AsyncLoadManager_sGetInstance()`,
/// `AsyncLoadManager_registAsyncLoadTask(manager, task, 0)` (cdecl) and
/// `AsyncLoadManager_isAllRequestDone(manager)` (`al`); the boot task
/// `0x00636430`.
pub const TROPHY_INSTANCE: u32 = 0x0067_ac90;
pub const TROPHY_CHECK: u32 = 0x0067_ac80;
pub const TROPHY_INSTALLED: u32 = 0x0067_acc0;
pub const TROPHY_ERROR: u32 = 0x0064_8ff0;
pub const ASYNC_MANAGER: u32 = 0x0069_1b40;
pub const REGIST_ASYNC_TASK: u32 = 0x0069_18b0;
pub const ALL_REQUESTS_DONE: u32 = 0x0069_1630;
pub const BOOT_TASK: u32 = 0x0063_6430;

/// `InitIggyHelper()`, `IggyMenuManager_createInstance()`, the menu
/// manager (`g_IggyMenuManager`) and its `0x00a262e0(manager; app +
/// 0x124, w, h)` (`ret 0xc`); the application getter.
pub const INIT_IGGY_HELPER: u32 = 0x006e_96c0;
pub const IGGY_MENU_CREATE: u32 = 0x00a2_6240;
pub const IGGY_MENU_MANAGER: u32 = 0x0198_4c6c;
pub const IGGY_MENU_SET_SIZE: u32 = 0x00a2_62e0;
pub const GET_APPLICATION: u32 = crate::h3_glue12::GET_APPLICATION;

/// The loading screen (`g_LoadingScreen`), its
/// `LoadingScreen_setParameterManually(screen; &time, 0, &value)` (`ret
/// 0xc`) and the time (`f32` 3); the argument that skips the opening
/// (the string at `0x00b0da90`).
pub const LOADING_SCREEN: u32 = 0x00ce_517c;
pub const SET_LOADING_PARAMETER: u32 = 0x006e_4ff0;
pub const LOADING_TIME: u32 = 0x00b3_eb94;
pub const SKIP_OPENING_ARG: u32 = 0x00b0_da90;

/// The scene (`g_PhyreScene`) and its `renderLoadingScreen`,
/// `beginRenderOpeningScreen`, `isOpeningScreenPlaying` (`al`),
/// `renderOpeningScreen`, `deleteIggyMenu_0065b750` and `initScene`
/// (all `this` only); `PInput_Update()`.
pub const SCENE: u32 = 0x00cc_b9d8;
pub const RENDER_LOADING_SCREEN: u32 = 0x0066_b1e0;
pub const BEGIN_OPENING: u32 = 0x0065_79d0;
pub const OPENING_PLAYING: u32 = 0x0066_0400;
pub const RENDER_OPENING: u32 = 0x0066_b2e0;
pub const DELETE_IGGY_MENU: u32 = 0x0065_b750;
pub const INIT_SCENE: u32 = 0x0065_f450;
pub const INPUT_UPDATE: u32 = 0x0062_5770;

/// The worker (`g_RecordState`) and its `0x00633670(worker; job,
/// &status, argc, argv, 0)` (`ret 0x14`) with `asyncFFXInitialize`
/// (`0x00636410`), `RecordStateIs8_006dbf30(worker; 0)` (`ret 4`, `al`);
/// the quit flag byte `0x00ccc7bc`.
pub const WORKER: u32 = 0x00cd_ede4;
pub const WORKER_RUN: u32 = 0x0063_3670;
pub const ASYNC_FFX_INITIALIZE: u32 = 0x0063_6410;
pub const WORKER_DONE: u32 = 0x006d_bf30;
pub const QUIT: u32 = 0x00cc_c7bc;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The C strings at `a` and `b` compare equal (the inlined `strcmp`).
    fn str_eq(h: &impl Host, a: u32, b: u32) -> bool {
        let mut k = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(k));
            if x != h.read::<u8>(b.wrapping_add(k)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            k = k.wrapping_add(1);
        }
    }

    fn global(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn app(h: &mut impl Host) -> u32 {
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[])
    }

    /// A `this`-only call on the scene.
    fn scene_call(h: &mut impl Host, sym: u32) -> u32 {
        let scene = global(h, SCENE);
        h.call_sym(sym, Conv::Thiscall(scene), &[])
    }

    fn trophy_error(h: &mut impl Host) -> u32 {
        h.call_sym(TROPHY_ERROR, Conv::Cdecl, &[])
    }

    fn requests_done(h: &mut impl Host) -> bool {
        let manager = h.call_sym(ASYNC_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(ALL_REQUESTS_DONE, Conv::Thiscall(manager), &[]) as u8 != 0
    }

    fn trophies_installed(h: &mut impl Host) -> bool {
        let trophy = h.call_sym(TROPHY_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(TROPHY_INSTALLED, Conv::Thiscall(trophy), &[]) as u8 != 0
    }

    /// The loading screen, unless the trophies report an error.
    fn loading_frame(h: &mut impl Host) {
        if trophy_error(h) == 0 {
            scene_call(h, RENDER_LOADING_SCREEN);
        }
    }

    /// The game's initialisation started on the worker:
    /// `asyncFFXInitialize` with the command line (`app + 0x3a8` count,
    /// `+ 0x3a4` words) and the status word at `ebp - 4`.
    fn start_init(h: &mut impl Host, ebp: u32) {
        let a = app(h);
        let b = app(h);
        let worker = global(h, WORKER);
        let job = h.rebase(ASYNC_FFX_INITIALIZE);
        let argc: u32 = h.read(b + 0x3a8);
        let argv: u32 = h.read(a + 0x3a4);
        h.call_sym(
            WORKER_RUN,
            Conv::Thiscall(worker),
            &[job, ebp - 4, argc, argv, 0],
        );
    }

    /// `graphicInitScene()` (cdecl; frame: the loading time at `ebp -
    /// 0xc`, its value at `- 8`, the status word the worker writes at
    /// `- 4`).
    ///
    /// Registers the boot task, creates the Iggy helper and menus, sets
    /// the loading screen (3, 0). With the skip argument (`argv[1]`)
    /// the loading screen is drawn 30 times (no trophy error); else the
    /// opening plays (input, frames; the initialisation starts on the
    /// worker once the requests are done) and the Iggy menu is deleted.
    /// Then it waits for the trophy install and the requests (drawing
    /// the loading screen; 0 when the quit flag rises or a trophy error
    /// appears), starts the initialisation if not yet, waits for the
    /// worker, and returns `initScene`'s `eax` when the status is 1,
    /// else 9.
    pub fn graphic_init_scene(h: &mut impl Host, ebp: u32) -> u32 {
        let trophy = h.call_sym(TROPHY_INSTANCE, Conv::Cdecl, &[]);
        if trophy != 0 {
            let trophy = h.call_sym(TROPHY_INSTANCE, Conv::Cdecl, &[]);
            h.call_sym(TROPHY_CHECK, Conv::Thiscall(trophy), &[]);
        }
        let manager = h.call_sym(ASYNC_MANAGER, Conv::Cdecl, &[]);
        let task = h.rebase(BOOT_TASK);
        h.call_sym(REGIST_ASYNC_TASK, Conv::Cdecl, &[manager, task, 0]);
        h.write(ebp - 4, 0u32);
        let mut pending = true;
        h.call_sym(INIT_IGGY_HELPER, Conv::Cdecl, &[]);
        h.call_sym(IGGY_MENU_CREATE, Conv::Cdecl, &[]);
        let a = app(h);
        let height: u32 = h.read(a + 0x270);
        let a = app(h);
        let width: u32 = h.read(a + 0x26c);
        let a = app(h);
        let menus = global(h, IGGY_MENU_MANAGER);
        h.call_sym(
            IGGY_MENU_SET_SIZE,
            Conv::Thiscall(menus),
            &[a + 0x124, width, height],
        );
        let a = app(h);
        h.write(a + 0x3a0, 0u8);
        let time = x87::store(x87::load(global(h, LOADING_TIME)));
        h.write(ebp - 0xc, time);
        h.write(ebp - 8, 0u32);
        let screen = global(h, LOADING_SCREEN);
        h.call_sym(
            SET_LOADING_PARAMETER,
            Conv::Thiscall(screen),
            &[ebp - 0xc, 0, ebp - 8],
        );
        let a = app(h);
        let skip = if h.read::<i32>(a + 0x3a8) > 1 {
            let a = app(h);
            let argv: u32 = h.read(a + 0x3a4);
            let arg1: u32 = h.read(argv + 4);
            str_eq(h, arg1, h.rebase(SKIP_OPENING_ARG))
        } else {
            false
        };
        if skip {
            let a = app(h);
            h.write(a + 0x3a0, 1u8);
            if trophy_error(h) == 0 {
                for _ in 0..0x1e {
                    scene_call(h, RENDER_LOADING_SCREEN);
                }
            }
        } else {
            scene_call(h, BEGIN_OPENING);
            while scene_call(h, OPENING_PLAYING) as u8 != 0 {
                h.call_sym(INPUT_UPDATE, Conv::Cdecl, &[]);
                scene_call(h, RENDER_OPENING);
                if pending && requests_done(h) {
                    start_init(h, ebp);
                    pending = false;
                }
            }
            scene_call(h, DELETE_IGGY_MENU);
            let a = app(h);
            h.write(a + 0x3a0, 1u8);
        }
        wait_and_finish(h, ebp, pending)
    }

    /// The tail of [`graphic_init_scene`].
    fn wait_and_finish(h: &mut impl Host, ebp: u32, pending: bool) -> u32 {
        let trophy = h.call_sym(TROPHY_INSTANCE, Conv::Cdecl, &[]);
        if trophy != 0 && trophy_error(h) == 0 && !trophies_installed(h) {
            loop {
                loading_frame(h);
                if trophies_installed(h) {
                    break;
                }
            }
        }
        if !requests_done(h) {
            loop {
                loading_frame(h);
                if h.read::<u8>(h.rebase(QUIT)) != 0 {
                    return 0;
                }
                if requests_done(h) {
                    break;
                }
            }
        }
        if trophy_error(h) != 0 {
            return 0;
        }
        let a = app(h);
        h.write(a + 0x3a1, 1u8);
        let a = app(h);
        h.write(a + 0x3a0, 0u8);
        if pending {
            start_init(h, ebp);
        }
        loop {
            let worker = global(h, WORKER);
            if h.call_sym(WORKER_DONE, Conv::Thiscall(worker), &[0]) as u8 != 0 {
                break;
            }
            scene_call(h, RENDER_LOADING_SCREEN);
        }
        let a = app(h);
        h.write(a + 0x3a1, 0u8);
        if h.read::<u32>(ebp - 4) == 1 {
            return scene_call(h, INIT_SCENE);
        }
        9
    }
}
