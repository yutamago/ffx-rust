//! M5 `system-b`: small pieces of the PC glue that belong to no larger
//! area of this list.

/// `FFEscMenu_vftable` and the vtable of its second base at `+0x9c`.
pub const ESC_MENU_VFTABLE: u32 = 0x00b4_6c20;
pub const ESC_MENU_LISTENER_VFTABLE: u32 = 0x00b4_6c2c;
/// `0x0062bce0(listener)` (cdecl): erases the listener from the set at
/// `0x00cc9ee4`.
pub const REMOVE_LISTENER: u32 = 0x0062_bce0;
/// `IggyMenu_dtor_IggyMenu` (thiscall).
pub const IGGY_MENU_DTOR: u32 = 0x00a2_8a40;

/// `LocalizationManager_readLocalizationKit`'s strings: the directory
/// (35 bytes with its NUL), `"PS3"`, the per-language file formats (the
/// language at `this + 4`: 0..=5, 9, 10 have one) and the error.
pub const LOC_KIT_DIR: u32 = 0x00b4_a210;
pub const LOC_KIT_DIR_LEN: u32 = 35;
pub const PS3: u32 = 0x00b4_a20c;
pub const LOC_KIT_FORMATS: [u32; 11] = [
    0x00b4_a234,
    0x00b4_a24c,
    0x00b4_a264,
    0x00b4_a27c,
    0x00b4_a294,
    0x00b4_a2ac,
    0,
    0,
    0,
    0x00b4_a2c4,
    0x00b4_a2dc,
];
pub const LOC_KIT_ERROR: u32 = 0x00b4_a2f4;
/// The frame: the file name at `ebp - 0x104` (0x100 bytes), the unified
/// path at `ebp - 0x304` (0x200).
pub const LOC_KIT_FRAME: u32 = 0x304;
/// `fiosUnifyFilename`, `fiosOpen`, `fiosLseek`, `fiosRead`, `fiosClose`
/// (cdecl), `operator new[]`, `crt_memset_thunk`; `MSVCR110` `sprintf`,
/// `printf`, `strstr` (cdecl).
pub const FIOS_UNIFY: u32 = 0x0067_9820;
pub const FIOS_OPEN: u32 = 0x0067_9730;
pub const FIOS_LSEEK: u32 = 0x0067_9710;
pub const FIOS_READ: u32 = 0x0067_9800;
pub const FIOS_CLOSE: u32 = 0x0067_9510;
pub const NEW_ARRAY: u32 = 0x0063_04c0;
pub const MEMSET: u32 = 0x0094_964c;
pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
pub const PRINTF_IAT: u32 = 0x00b0_c380;
pub const STRSTR_IAT: u32 = 0x00b0_c378;
/// `DepthBiasHooks_onSceneLoaded`'s maps (`"djyt04"`, `"djyt05"`,
/// `"lchb18"`, `"lchb00"`, `"mcfr12"`, `"mcfr14"`), `g_PhyreScene`,
/// `PhyreScene_getCurrentLevelName` (thiscall), `Sg_SetMapAlphaFast`
/// (cdecl), `AssetListReader_dtor_AssetListReader` (thiscall) and
/// `user_free_thunk` (cdecl).
pub const DEPTH_BIAS_MAPS: [u32; 6] = [
    0x00b4_ada4,
    0x00b4_adac,
    0x00b4_adb4,
    0x00b4_adbc,
    0x00b4_adc4,
    0x00b4_adcc,
];
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
pub const LEVEL_NAME: u32 = 0x0065_de70;
pub const SET_MAP_ALPHA_FAST: u32 = 0x0082_1b00;
pub const ASSET_LIST_DTOR: u32 = 0x0063_4610;
pub const USER_FREE: u32 = 0x0068_7010;

/// `0x0059e520(texture; sampler)` (thiscall, `ret 4`: binds the pair) and
/// `memcpy` (`0x00949256`, cdecl).
pub const BIND_TEXTURE: u32 = 0x0059_e520;
pub const MEMCPY: u32 = 0x0094_9256;
/// `Sys_Thread_vftable`, `"JobWorker"`, `SysThread_create_00679cb0(this;
/// arg, a, b, name)` (thiscall, `ret 0x10`), `operator_delete`; `KERNEL32`
/// `TerminateThread`, `CloseHandle`, `SetThreadPriority`, `ResumeThread`
/// (stdcall).
pub const THREAD_VFTABLE: u32 = 0x00b4_4e2c;
pub const JOB_WORKER_NAME: u32 = 0x00b4_cf38;
pub const THREAD_CREATE: u32 = 0x0067_9cb0;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const TERMINATE_THREAD_IAT: u32 = 0x00b0_c1d8;
pub const CLOSE_HANDLE_IAT: u32 = 0x00b0_c0dc;
pub const SET_THREAD_PRIORITY_IAT: u32 = 0x00b0_c10c;
pub const RESUME_THREAD_IAT: u32 = 0x00b0_c1dc;

/// The originals over game memory.
pub mod bridge {
    use super::{
        ASSET_LIST_DTOR, BIND_TEXTURE, CLOSE_HANDLE_IAT, DEPTH_BIAS_MAPS,
        ESC_MENU_LISTENER_VFTABLE, ESC_MENU_VFTABLE, FIOS_CLOSE, FIOS_LSEEK, FIOS_OPEN, FIOS_READ,
        FIOS_UNIFY, IGGY_MENU_DTOR, JOB_WORKER_NAME, LEVEL_NAME, LOC_KIT_DIR, LOC_KIT_DIR_LEN,
        LOC_KIT_ERROR, LOC_KIT_FORMATS, MEMCPY, MEMSET, NEW_ARRAY, OPERATOR_DELETE, PHYRE_SCENE,
        PRINTF_IAT, PS3, REMOVE_LISTENER, RESUME_THREAD_IAT, SET_MAP_ALPHA_FAST,
        SET_THREAD_PRIORITY_IAT, SPRINTF_IAT, STRSTR_IAT, TERMINATE_THREAD_IAT, THREAD_CREATE,
        THREAD_VFTABLE, USER_FREE,
    };
    use ffx_graphic::host::{Conv, Host};

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    fn import_std(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// `LocalizationManager_readLocalizationKit(this)` (`0x006db2f0`,
    /// thiscall; `ebp` the original's frame pointer, the cookie left out):
    /// the kit directory copied into the name buffer (the rest cleared),
    /// the language's file name formatted over it (`sprintf(name, fmt,
    /// name, "PS3")`; languages 6..8 and past 10 keep the directory),
    /// unified into the path; without the file the error is printed. Else
    /// `+0x18` = its size (`fiosLseek(f, 0, 2)`), back to 0, `+0x14` =
    /// `new[](size)`, cleared and read until a read returns the whole size
    /// (signed), then closed.
    pub fn read_localization_kit(h: &mut impl Host, ebp: u32, this: u32) {
        let name = ebp.wrapping_sub(0x104);
        let path = ebp.wrapping_sub(0x304);
        let mut dir = vec![0u8; LOC_KIT_DIR_LEN as usize];
        h.read_bytes(h.rebase(LOC_KIT_DIR), &mut dir);
        h.write_bytes(name, &dir);
        h.call_sym(
            MEMSET,
            Conv::Cdecl,
            &[
                name.wrapping_add(LOC_KIT_DIR_LEN),
                0,
                0x100 - LOC_KIT_DIR_LEN,
            ],
        );
        let lang: u32 = h.read(this.wrapping_add(4));
        if let Some(&fmt) = LOC_KIT_FORMATS.get(lang as usize)
            && fmt != 0
        {
            let (fmt, ps3) = (h.rebase(fmt), h.rebase(PS3));
            import(h, SPRINTF_IAT, &[name, fmt, name, ps3]);
        }
        h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[name, path, 0x200]);
        let f = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[path, 1]);
        if f == 0 {
            let msg = h.rebase(LOC_KIT_ERROR);
            import(h, PRINTF_IAT, &[msg, path]);
            return;
        }
        let size = h.call_sym(FIOS_LSEEK, Conv::Cdecl, &[f, 0, 2]);
        h.write(this.wrapping_add(0x18), size);
        h.call_sym(FIOS_LSEEK, Conv::Cdecl, &[f, 0, 0]);
        let size: u32 = h.read(this.wrapping_add(0x18));
        let buf = h.call_sym(NEW_ARRAY, Conv::Cdecl, &[size]);
        h.write(this.wrapping_add(0x14), buf);
        loop {
            let size: u32 = h.read(this.wrapping_add(0x18));
            let buf: u32 = h.read(this.wrapping_add(0x14));
            h.call_sym(MEMSET, Conv::Cdecl, &[buf, 0, size]);
            let size: u32 = h.read(this.wrapping_add(0x18));
            let buf: u32 = h.read(this.wrapping_add(0x14));
            let got = h.call_sym(FIOS_READ, Conv::Cdecl, &[f, buf, size]);
            let size: u32 = h.read(this.wrapping_add(0x18));
            if got.cast_signed() >= size.cast_signed() {
                break;
            }
        }
        h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[f]);
    }

    /// `DepthBiasHooks_onSceneLoaded(_, a, b)` (`0x006e4470`, cdecl; `args`
    /// the caller's argument words: a word, then two 0x50-byte objects by
    /// value, an `AssetListReader` and a block pointer each; without its
    /// SEH frame): on the six maps whose names the current level's name
    /// contains (`strstr`), `Sg_SetMapAlphaFast(0)`; then the two objects
    /// destroyed (`AssetListReader`'s destructor, `user_free` of the
    /// block when set), as the callee owns them.
    pub fn depth_bias_on_scene_loaded(h: &mut impl Host, args: u32) {
        for name in DEPTH_BIAS_MAPS {
            let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
            let level = h.call_sym(LEVEL_NAME, Conv::Thiscall(scene), &[]);
            let name = h.rebase(name);
            if import(h, STRSTR_IAT, &[level, name]) != 0 {
                h.call_sym(SET_MAP_ALPHA_FAST, Conv::Cdecl, &[0]);
            }
        }
        for obj in [args.wrapping_add(4), args.wrapping_add(0x54)] {
            h.call_sym(ASSET_LIST_DTOR, Conv::Thiscall(obj), &[]);
            let block: u32 = h.read(obj.wrapping_add(0x4c));
            if block != 0 {
                h.call_sym(USER_FREE, Conv::Cdecl, &[block]);
            }
        }
    }

    /// `FFEscMenu::~FFEscMenu(this)` (`0x0068e1f0`, thiscall; without its
    /// SEH frame): the two vtables, the listener at `+0x9c` removed, then
    /// `IggyMenu::~IggyMenu(this)`, whose `eax` it returns.
    pub fn esc_menu_destruct(h: &mut impl Host, this: u32) -> u32 {
        let listener = this.wrapping_add(0x9c);
        h.write(this, h.rebase(ESC_MENU_VFTABLE));
        h.write(listener, h.rebase(ESC_MENU_LISTENER_VFTABLE));
        h.call_sym(REMOVE_LISTENER, Conv::Cdecl, &[listener]);
        h.call_sym(IGGY_MENU_DTOR, Conv::Thiscall(this), &[])
    }

    /// A post effect's texture parameter setter `(this; sampler, texture)`
    /// (thiscall, `ret 8`; `frame` its 0x10-byte value block; the record
    /// at `this + record`, the buffer holder at `this + holder`; the code
    /// of `BlurEffectEx_setTextureParam_006c44e0`): no record gives 0x13; a
    /// record that is no texture type (byte `+2`: 0x42, 0xd9..=0xdc, 0x12)
    /// gives 5; the texture bound to the sampler when both are set (its
    /// failure returned); a record past the buffer's size word or below
    /// offset 4 gives 3; else `{0x42, 0, texture, sampler}` copied in and
    /// 0.
    #[allow(clippy::too_many_arguments)]
    pub fn post_set_texture_param(
        h: &mut impl Host,
        frame: u32,
        this: u32,
        record: u32,
        holder: u32,
        sampler: u32,
        texture: u32,
    ) -> u32 {
        let rec: u32 = h.read(this.wrapping_add(record));
        if rec == 0 {
            return 0x13;
        }
        let holder: u32 = h.read(this.wrapping_add(holder));
        let block: u32 = h.read(holder.wrapping_add(4));
        let kind: u8 = h.read(rec.wrapping_add(2));
        h.write(frame, 0x42u32);
        h.write(frame.wrapping_add(0xc), sampler);
        h.write(frame.wrapping_add(8), texture);
        h.write(frame.wrapping_add(4), 0u32);
        if !matches!(kind, 0x42 | 0xd9..=0xdc | 0x12) {
            return 5;
        }
        if texture != 0 && sampler != 0 {
            let r = h.call_sym(BIND_TEXTURE, Conv::Thiscall(texture), &[sampler]);
            if r != 0 {
                return r;
            }
        }
        let len = u32::from(h.read::<u16>(rec.wrapping_add(0xa)) & 0x1fff);
        let offset = u32::from(h.read::<u16>(rec.wrapping_add(8)));
        if offset.wrapping_add(len) > h.read::<u32>(block) || offset < 4 {
            return 3;
        }
        h.call_sym(
            MEMCPY,
            Conv::Cdecl,
            &[offset.wrapping_add(block), frame, len],
        );
        0
    }

    /// `Sys::Thread::~Thread(this)` (`0x00703990`, thiscall): the vtable;
    /// a running thread terminated (`TerminateThread(h, 0)`) and closed,
    /// its handle and id cleared.
    pub fn thread_destruct(h: &mut impl Host, this: u32) {
        let handle: u32 = h.read(this.wrapping_add(4));
        h.write(this, h.rebase(THREAD_VFTABLE));
        if handle != 0 {
            import_std(h, TERMINATE_THREAD_IAT, &[handle, 0]);
            let handle: u32 = h.read(this.wrapping_add(4));
            import_std(h, CLOSE_HANDLE_IAT, &[handle]);
            h.write(this.wrapping_add(4), 0u32);
            h.write(this.wrapping_add(8), 0u32);
        }
    }

    /// The deleting destructor `(this; flags)` (`0x007039d0`): as
    /// [`thread_destruct`], `operator delete` when `flags & 1`; `this`.
    pub fn thread_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        thread_destruct(h, this);
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `JobWorker_start_00703a50(this; queue, arg)` (thiscall, `ret 8`):
    /// `+0x10` = `queue`; the thread created suspended
    /// (`SysThread_create_00679cb0(this; arg, 0, 0, "JobWorker")`), its
    /// priority 0, resumed. Returns `ResumeThread`'s result.
    pub fn job_worker_start(h: &mut impl Host, this: u32, queue: u32, arg: u32) -> u32 {
        h.write(this.wrapping_add(0x10), queue);
        let name = h.rebase(JOB_WORKER_NAME);
        h.call_sym(THREAD_CREATE, Conv::Thiscall(this), &[arg, 0, 0, name]);
        let handle: u32 = h.read(this.wrapping_add(4));
        import_std(h, SET_THREAD_PRIORITY_IAT, &[handle, 0]);
        let handle: u32 = h.read(this.wrapping_add(4));
        import_std(h, RESUME_THREAD_IAT, &[handle])
    }
}
