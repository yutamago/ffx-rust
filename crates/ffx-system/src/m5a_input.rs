//! M5 `system-a` (round 3): `DirectInput`'s start (`Input_InitDirectInput`,
//! the device enumeration callback and the axis range set-up), the
//! archive handles' constructor and registration, the command-line
//! prefix match and the copy of a global intrusive list.

/// The originals over game memory and the original callees.
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `KERNEL32!GetModuleHandleA`, `KERNEL32!InitializeCriticalSection`
    /// (stdcall); `MSVCR110!strncmp` (cdecl).
    pub const GET_MODULE_HANDLE_IAT: u32 = 0x00b0_c114;
    pub const INIT_CS_IAT: u32 = 0x00b0_c150;
    pub const STRNCMP_IAT: u32 = 0x00b0_c3e4;
    /// `DINPUT8!DirectInput8Create` through its jump thunk (stdcall).
    pub const DIRECT_INPUT8_CREATE: u32 = 0x0094_a914;
    /// The `IDirectInput8` the game keeps, `IID_IDirectInput8A`, and the
    /// enumeration callback (`Input_EnumDevicesCallback`).
    pub const DIRECT_INPUT: u32 = 0x00cc_9cd4;
    pub const IID_DIRECT_INPUT8: u32 = 0x00b6_bddc;
    pub const ENUM_DEVICES_CALLBACK: u32 = 0x0062_3330;
    /// `Input_AddMouse`, `Input_AddKeyboard`, `Input_AddPadDevice` (cdecl).
    pub const ADD_MOUSE: u32 = 0x0062_4020;
    pub const ADD_KEYBOARD: u32 = 0x0062_3fa0;
    pub const ADD_PAD: u32 = 0x0062_40e0;
    /// `Phyre_PFreeList_PFreeList` (thiscall, `ret 0x10`) and the name it
    /// gets.
    pub const FREE_LIST_CTOR: u32 = 0x0049_c480;
    pub const FREE_LIST_NAME: u32 = 0x00b1_3bc8;
    /// `BigFileHandle_BigFileHandle`, `BigFileHandle_initialize` (`ret
    /// 4`, `al`), `BigFileHandle_dtor_BigFileHandle` (thiscall);
    /// `operator new` / `operator delete` (cdecl).
    pub const HANDLE_CTOR: u32 = 0x0061_d460;
    pub const HANDLE_INIT: u32 = 0x0061_dac0;
    pub const HANDLE_DTOR: u32 = 0x0061_d670;
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// The list head `0x00623420` walks.
    pub const LIST_HEAD: u32 = 0x00c3_0fa8;

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `Input_InitDirectInput()` (cdecl): `DirectInput8Create(
    /// GetModuleHandleA(0), 0x800, IID_IDirectInput8A, &di, 0)` (9 when
    /// it fails), then `di->EnumDevices(DI8DEVCLASS_ALL,
    /// Input_EnumDevicesCallback, 0, DIEDFL_ATTACHEDONLY)`: 0 when that
    /// succeeds, else 9.
    pub fn init_direct_input(h: &mut impl Host) -> u32 {
        let module = import(h, GET_MODULE_HANDLE_IAT, Conv::Stdcall, &[0]);
        let iid = h.rebase(IID_DIRECT_INPUT8);
        let out = h.rebase(DIRECT_INPUT);
        let hr = h.call_sym(
            DIRECT_INPUT8_CREATE,
            Conv::Stdcall,
            &[module, 0x800, iid, out, 0],
        );
        if hr.cast_signed() < 0 {
            return 9;
        }
        let di: u32 = h.read(out);
        let vt: u32 = h.read(di);
        let enum_devices: u32 = h.read(vt.wrapping_add(0x10));
        let cb = h.rebase(ENUM_DEVICES_CALLBACK);
        let hr = h.call(enum_devices, Conv::Stdcall, &[di, 0, cb, 0, 1]);
        if hr.cast_signed() < 0 { 9 } else { 0 }
    }

    /// `Input_EnumDevicesCallback(instance, ctx)` (stdcall, `ret 8`): by
    /// the device type (`+0x24`): 0x12 a mouse, 0x13 a keyboard, 0x14,
    /// 0x15 and 0x18 a pad added; 1 (continue).
    pub fn enum_devices_callback(h: &mut impl Host, instance: u32) -> u32 {
        let add = match h.read::<u8>(instance.wrapping_add(0x24)) {
            0x12 => Some(ADD_MOUSE),
            0x13 => Some(ADD_KEYBOARD),
            0x14 | 0x15 | 0x18 => Some(ADD_PAD),
            _ => None,
        };
        if let Some(add) = add {
            h.call_sym(add, Conv::Cdecl, &[instance]);
        }
        1
    }

    /// The frame `Input_EnumAxesSetRange` needs: a `DIPROPRANGE` at `ebp -
    /// 0x1c`.
    pub const RANGE_FRAME: u32 = 0x1c;

    /// `Input_EnumAxesSetRange(object, device)` (stdcall, `ret 8`): for an
    /// axis (type bits 0-1 set) the range -255..255 set through
    /// `device->SetProperty(DIPROP_RANGE, &range)` (vtable `+0x18`, the
    /// `DIPROPRANGE` in the frame, by id); 0 when that fails, else 1.
    pub fn enum_axes_set_range(h: &mut impl Host, ebp: u32, object: u32, device: u32) -> u32 {
        let ty: u32 = h.read(object.wrapping_add(0x18));
        if ty.is_multiple_of(4) {
            return 1;
        }
        let range = ebp.wrapping_sub(RANGE_FRAME);
        for (k, v) in (0u32..).zip([0x18u32, 0x10, ty, 2, 0xffff_ff01, 0xff]) {
            h.write(range.wrapping_add(4 * k), v);
        }
        let vt: u32 = h.read(device);
        let set_property: u32 = h.read(vt.wrapping_add(0x18));
        let hr = h.call(set_property, Conv::Stdcall, &[device, 4, range]);
        u32::from(hr.cast_signed() >= 0)
    }

    /// `0x00623420(out)` (cdecl): the nodes of the intrusive list at
    /// `0x00c30fa8` (each linked by its `+4`, the node 4 bytes before the
    /// link) into `out`, until the head or a null link; their count.
    pub fn copy_list(h: &mut impl Host, out: u32) -> u32 {
        let head = h.rebase(LIST_HEAD);
        let mut link: u32 = h.read(head);
        let mut n = 0u32;
        while link != head && link != 0 {
            let node = link.wrapping_sub(4);
            if node == 0 {
                break;
            }
            h.write(out.wrapping_add(4 * n), node);
            link = h.read(node.wrapping_add(4));
            n = n.wrapping_add(1);
        }
        n
    }

    /// `0x00623da0(s, prefix)` (cdecl): the rest of `s` after `prefix`
    /// when `s` starts with it (`strncmp`), else 0.
    pub fn after_prefix(h: &mut impl Host, s: u32, prefix: u32) -> u32 {
        let mut len = 0u32;
        while h.read::<u8>(prefix.wrapping_add(len)) != 0 {
            len = len.wrapping_add(1);
        }
        if import(h, STRNCMP_IAT, Conv::Cdecl, &[s, prefix, len]) == 0 {
            len.wrapping_add(s)
        } else {
            0
        }
    }

    /// `BigFileHandle_BigFileHandle(this)` (thiscall): `+0` = 0, the tree
    /// head at `+4` linked to itself (six words), the node free list at
    /// `+0x1c` (0x28-byte nodes, 8 per block, alignment 8), `+0x38`,
    /// `+0x3c`, `+0x40`, `+0x5c` = 0, the lock at `+0x44` initialised.
    /// Returns `this`.
    pub fn handle_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        let head = this.wrapping_add(4);
        for off in [0u32, 4, 8, 0x14, 0x10, 0xc] {
            h.write(head.wrapping_add(off), head);
        }
        let name = h.rebase(FREE_LIST_NAME);
        h.call_sym(
            FREE_LIST_CTOR,
            Conv::Thiscall(this.wrapping_add(0x1c)),
            &[0x28, 8, 8, name],
        );
        for off in [0x38u32, 0x3c, 0x40, 0x5c] {
            h.write(this.wrapping_add(off), 0u32);
        }
        import(h, INIT_CS_IAT, Conv::Stdcall, &[this.wrapping_add(0x44)]);
        this
    }

    /// `BigFileStream_registerBigFile(this; path)` (thiscall, `ret 4`) ->
    /// `al`: the first free of the five archive slots (`+0x10`; none: 0)
    /// gets a new `BigFileHandle` (0x60 bytes; 0 when the allocation
    /// fails) initialised from `path`; when that fails the handle is
    /// destroyed, deleted and the slot cleared. `BigFileHandle_initialize`'s
    /// answer.
    pub fn register_big_file(h: &mut impl Host, this: u32, path: u32) -> u8 {
        let Some(i) = (0..5u32).find(|&i| h.read::<u32>(this.wrapping_add(0x10 + 4 * i)) == 0)
        else {
            return 0;
        };
        let slot = this.wrapping_add(0x10 + 4 * i);
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x60]);
        let handle = if p == 0 {
            0
        } else {
            h.call_sym(HANDLE_CTOR, Conv::Thiscall(p), &[])
        };
        h.write(slot, handle);
        if handle == 0 {
            return 0;
        }
        let ok = h
            .call_sym(HANDLE_INIT, Conv::Thiscall(handle), &[path])
            .to_le_bytes()[0];
        if ok == 0 {
            let handle: u32 = h.read(slot);
            if handle != 0 {
                h.call_sym(HANDLE_DTOR, Conv::Thiscall(handle), &[]);
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[handle]);
            }
            h.write(slot, 0u32);
        }
        ok
    }

    /// `steam_api` imports (cdecl): `SteamAPI_RestartAppIfNecessary`,
    /// `SteamAPI_Init`, `SteamUtils`, `SteamUser`; `MSVCR110!printf`.
    pub const STEAM_RESTART_IAT: u32 = 0x00b0_c8ac;
    pub const STEAM_INIT_IAT: u32 = 0x00b0_c8a8;
    pub const STEAM_UTILS_IAT: u32 = 0x00b0_c89c;
    pub const STEAM_USER_IAT: u32 = 0x00b0_c8a4;
    pub const PRINTF_IAT: u32 = 0x00b0_c380;
    /// `g_SteamReady` (byte) and the four messages.
    pub const STEAM_READY: u32 = 0x00cc_9cf5;
    pub const MSG_INIT_FAILED: u32 = 0x00b4_0e40;
    pub const MSG_INIT_OK: u32 = 0x00b4_0e58;
    pub const MSG_NOT_LOGGED_ON: u32 = 0x00b4_0e74;
    pub const MSG_LOGGED_ON: u32 = 0x00b4_0e94;
    /// `MSVCR110!_stricmp` (cdecl); `KERNEL32!SetThreadUILanguage`
    /// (stdcall).
    pub const STRICMP_IAT: u32 = 0x00b0_c398;
    pub const SET_THREAD_UI_LANGUAGE_IAT: u32 = 0x00b0_c1b8;
    /// `g_Settings` (a pointer; its first word the language index).
    pub const SETTINGS: u32 = 0x022f_b504;
    /// The key code to key slot table the script key commands use.
    pub const KEY_SLOTS: u32 = 0x00c3_0e08;
    /// `0x00624da0` (cdecl).
    pub const SCRIPT_624DA0: u32 = 0x0062_4da0;

    /// `Steam_Init()` (cdecl): 1 when Steam restarts the game
    /// (`SteamAPI_RestartAppIfNecessary(0)`) or `SteamAPI_Init` fails (a
    /// message printed); else `g_SteamReady` set, a message,
    /// `SteamUtils()->SetOverlayNotificationPosition(1)` (vtable
    /// `+0x28`), and by `SteamUser()->BLoggedOn()` (`+4`) a message: not
    /// logged on 1, else that `printf`'s `eax`.
    pub fn steam_init(h: &mut impl Host) -> u32 {
        if import(h, STEAM_RESTART_IAT, Conv::Cdecl, &[0]) & 0xff != 0 {
            return 1;
        }
        if import(h, STEAM_INIT_IAT, Conv::Cdecl, &[]).to_le_bytes()[0] == 0 {
            let msg = h.rebase(MSG_INIT_FAILED);
            import(h, PRINTF_IAT, Conv::Cdecl, &[msg]);
            return 1;
        }
        let ready = h.rebase(STEAM_READY);
        h.write(ready, 1u8);
        let msg = h.rebase(MSG_INIT_OK);
        import(h, PRINTF_IAT, Conv::Cdecl, &[msg]);
        let utils = import(h, STEAM_UTILS_IAT, Conv::Cdecl, &[]);
        h.call_virtual(utils, 0x28 / 4, &[1]);
        let user = import(h, STEAM_USER_IAT, Conv::Cdecl, &[]);
        if h.call_virtual(user, 1, &[]).to_le_bytes()[0] == 0 {
            let msg = h.rebase(MSG_NOT_LOGGED_ON);
            import(h, PRINTF_IAT, Conv::Cdecl, &[msg]);
            return 1;
        }
        let msg = h.rebase(MSG_LOGGED_ON);
        import(h, PRINTF_IAT, Conv::Cdecl, &[msg])
    }

    /// `0x006255d0(table, name, default)` (cdecl): the value slot (`+4`)
    /// of the first 8-byte `{name, value}` entry whose name equals `name`
    /// ignoring case (`_stricmp`), up to a null name; `default` without
    /// a match or a table.
    pub fn find_option(h: &mut impl Host, table: u32, name: u32, default: u32) -> u32 {
        if table == 0 {
            return default;
        }
        let mut entry = table;
        loop {
            let key: u32 = h.read(entry);
            if key == 0 {
                return default;
            }
            if import(h, STRICMP_IAT, Conv::Cdecl, &[key, name]) == 0 {
                return entry.wrapping_add(4);
            }
            entry = entry.wrapping_add(8);
        }
    }

    /// `Input_Shutdown()` (cdecl): every device of `g_PInputDeviceList`
    /// unlinked (its links pointed at itself) and destroyed (vtable slot
    /// 0, flags 1), until the list is empty or a link is null; then the
    /// `IDirectInput8` released (`+8`) and cleared. Returns 0.
    pub fn input_shutdown(h: &mut impl Host) -> u32 {
        let head = h.rebase(LIST_HEAD);
        loop {
            let link: u32 = h.read(head);
            if link == head || link == 0 || link == 4 {
                break;
            }
            let node = link.wrapping_sub(4);
            let next: u32 = h.read(node.wrapping_add(4));
            let prev: u32 = h.read(node.wrapping_add(8));
            h.write(next.wrapping_add(4), prev);
            h.write(prev, next);
            h.write(link, link);
            h.write(link.wrapping_add(4), link);
            h.call_virtual(node, 0, &[1]);
        }
        let at = h.rebase(DIRECT_INPUT);
        let di: u32 = h.read(at);
        if di != 0 {
            let vt: u32 = h.read(di);
            let release: u32 = h.read(vt.wrapping_add(8));
            h.call(release, Conv::Stdcall, &[di]);
            h.write(at, 0u32);
        }
        0
    }

    /// `Settings_SetThreadUILanguage()` (cdecl): `SetThreadUILanguage` of
    /// the settings' language (1 English 0x409, 2 French 0x40c, 3 German
    /// 0x407, 4 Italian 0x410, 5 Spanish 0x40a, 6 Korean 0x412, 7 Chinese
    /// 0x7c04, else Japanese 0x411); its `eax`.
    pub fn set_thread_ui_language(h: &mut impl Host) -> u32 {
        let settings: u32 = h.read(h.rebase(SETTINGS));
        let lang = match h.read::<u32>(settings) {
            1 => 0x409,
            2 => 0x40c,
            3 => 0x407,
            4 => 0x410,
            5 => 0x40a,
            6 => 0x412,
            7 => 0x7c04,
            _ => 0x411,
        };
        import(h, SET_THREAD_UI_LANGUAGE_IAT, Conv::Stdcall, &[lang])
    }

    /// `0x00624810` / `0x00624850(app, key)` (cdecl, script key
    /// commands): with an application, a key and the keyboard state
    /// (`[app + 0x2b8] + 0x80`), the key's slot byte (the table at
    /// `0x00c30e08` by the key's first character's low 7 bits) set to
    /// `down`, 4 bytes before the state. Returns 0.
    pub fn set_script_key(h: &mut impl Host, app: u32, key: u32, down: u8) -> u32 {
        if app == 0 || key == 0 {
            return 0;
        }
        let devices: u32 = h.read(app.wrapping_add(0x2b8));
        let state: u32 = h.read(devices.wrapping_add(0x80));
        if state == 0 {
            return 0;
        }
        let c = u32::from(h.read::<u8>(key)) & 0x7f;
        let slot = u32::from(h.read::<u8>(h.rebase(KEY_SLOTS).wrapping_add(c)));
        h.write(slot.wrapping_add(state).wrapping_sub(4), down);
        0
    }

    /// `0x00624d70(obj, arg)` (cdecl, a script command): the object's
    /// vtable slot 8 (thiscall) when that is not 0; else 0 without `arg`,
    /// else `0x00624da0(obj, arg)`.
    pub fn script_624d70(h: &mut impl Host, obj: u32, arg: u32) -> u32 {
        let r = h.call_virtual(obj, 8, &[]);
        if r != 0 || arg == 0 {
            return r;
        }
        h.call_sym(SCRIPT_624DA0, Conv::Cdecl, &[obj, arg])
    }

    /// `Phyre_PhyreMalloc` (cdecl), `PInputDeviceKeyboard_ctor` and
    /// `PInputDeviceMouse_ctor` (thiscall, `ret 4`); `c_dfDIKeyboard`,
    /// `c_dfDIMouse2`, `g_hWnd`; `MSVCR110!sscanf_s` (cdecl) and its
    /// format.
    pub const PHYRE_MALLOC: u32 = 0x0042_fb40;
    pub const KEYBOARD_CTOR: u32 = 0x0061_fff0;
    pub const MOUSE_CTOR: u32 = 0x0062_00d0;
    pub const DF_KEYBOARD: u32 = 0x00b6_a41c;
    pub const DF_MOUSE: u32 = 0x00b6_a214;
    pub const HWND: u32 = 0x00cc_9ce8;
    pub const SSCANF_S_IAT: u32 = 0x00b0_c418;
    pub const MOUSE_FORMAT: u32 = 0x00b4_02e8;

    /// A COM call through `obj`'s vtable slot at `off` (stdcall, `obj`
    /// first).
    fn com(h: &mut impl Host, obj: u32, off: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt.wrapping_add(off));
        let mut words = vec![obj];
        words.extend_from_slice(args);
        h.call(f, Conv::Stdcall, &words)
    }

    /// The frame of the device adders: the device pointer at `ebp - 4`
    /// (`Input_AddKeyboard`) or `ebp - 0x10` (`Input_AddMouse`).
    pub const KEYBOARD_FRAME: u32 = 4;
    pub const MOUSE_FRAME: u32 = 0x10;

    /// The device of `instance` (`IDirectInput8::CreateDevice(guid at
    /// +4, slot, 0)`), its data format `format` set, the cooperative
    /// level (`g_hWnd`, foreground | non-exclusive) set and the device
    /// acquired; the device, or `None` when a step fails (the level only
    /// on `E_NOTIMPL`).
    fn open_device(h: &mut impl Host, instance: u32, slot: u32, format: u32) -> Option<u32> {
        let di: u32 = h.read(h.rebase(DIRECT_INPUT));
        if com(h, di, 0xc, &[instance.wrapping_add(4), slot, 0]).cast_signed() < 0 {
            return None;
        }
        let dev: u32 = h.read(slot);
        let format = h.rebase(format);
        if com(h, dev, 0x2c, &[format]).cast_signed() < 0 {
            return None;
        }
        let dev: u32 = h.read(slot);
        let hwnd: u32 = h.read(h.rebase(HWND));
        if com(h, dev, 0x34, &[hwnd, 6]) == 0x8000_4001 {
            return None;
        }
        let dev: u32 = h.read(slot);
        com(h, dev, 0x1c, &[]);
        Some(h.read(slot))
    }

    /// `Input_AddKeyboard(instance)` (cdecl; its only caller,
    /// `Input_EnumDevicesCallback`, drops `eax`): the device opened
    /// (`c_dfDIKeyboard`), then a `PInputDeviceKeyboard` (0x318 bytes from
    /// `Phyre_PhyreMalloc`, none when that fails) built on it.
    pub fn add_keyboard(h: &mut impl Host, ebp: u32, instance: u32) {
        let slot = ebp.wrapping_sub(KEYBOARD_FRAME);
        let Some(dev) = open_device(h, instance, slot, DF_KEYBOARD) else {
            return;
        };
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x318]);
        if p != 0 {
            h.call_sym(KEYBOARD_CTOR, Conv::Thiscall(p), &[dev]);
        }
    }

    /// `Input_AddMouse(instance)` (cdecl; `eax` dropped by its caller):
    /// the device opened (`c_dfDIMouse2`), an 8-byte record (`+0` = 0,
    /// `+4` the device; `Phyre_PhyreMalloc`, used unchecked) and a
    /// `PInputDeviceMouse` (0x11c bytes, none when that allocation fails)
    /// built on it.
    pub fn add_mouse(h: &mut impl Host, ebp: u32, instance: u32) {
        let slot = ebp.wrapping_sub(MOUSE_FRAME);
        if open_device(h, instance, slot, DF_MOUSE).is_none() {
            return;
        }
        let rec = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[8]);
        h.write(rec, 0u8);
        let dev: u32 = h.read(slot);
        h.write(rec.wrapping_add(4), dev);
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x11c]);
        if p != 0 {
            h.call_sym(MOUSE_CTOR, Conv::Thiscall(p), &[rec]);
        }
    }

    /// `0x00624890(app, key)` (cdecl, a script command): the key pressed
    /// (as [`set_script_key`]), the application's vtable slot 7
    /// (thiscall) run; its `eax` when not 0, else the key released and 0.
    pub fn script_key_press(h: &mut impl Host, app: u32, key: u32) -> u32 {
        if app == 0 || key == 0 {
            return 0;
        }
        set_script_key(h, app, key, 1);
        let r = h.call_virtual(app, 7, &[]);
        if r != 0 {
            return r;
        }
        set_script_key(h, app, key, 0)
    }

    /// The frame `0x00624910` needs: the two coordinates at `ebp - 8`
    /// and `ebp - 4`.
    pub const MOUSE_SET_FRAME: u32 = 8;

    /// `0x00624910(app, text)` (cdecl, a script command): `sscanf_s(text,
    /// "%d %d"-like format, &x, &y)` into the frame, then with the mouse
    /// state (`[app + 0x2b8]`) flagged (`+0xa0`, `+0xa7`) and its position
    /// set (`+0xa8` x, `+0xac` y, `+0xb0`, `+0xb4` 0) the application's
    /// vtable slot 7 run, `+0xa0` cleared. Returns the vtable call's
    /// `eax`.
    pub fn script_mouse_set(h: &mut impl Host, ebp: u32, app: u32, text: u32) -> u32 {
        let y_at = ebp.wrapping_sub(8);
        let x_at = ebp.wrapping_sub(4);
        let fmt = h.rebase(MOUSE_FORMAT);
        import(h, SSCANF_S_IAT, Conv::Cdecl, &[text, fmt, x_at, y_at]);
        let state_at = app.wrapping_add(0x2b8);
        let x: u32 = h.read(x_at);
        let y: u32 = h.read(y_at);
        let fields: [(u32, Option<u32>); 6] = [
            (0xa0, None),
            (0xa7, None),
            (0xb0, Some(0)),
            (0xa8, Some(x)),
            (0xb4, Some(0)),
            (0xac, Some(y)),
        ];
        for (off, v) in fields {
            let state: u32 = h.read(state_at);
            match v {
                None => h.write(state.wrapping_add(off), 1u8),
                Some(v) => h.write(state.wrapping_add(off), v),
            }
        }
        let r = h.call_virtual(app, 7, &[]);
        let state: u32 = h.read(state_at);
        h.write(state.wrapping_add(0xa0), 0u8);
        r
    }

    /// The script pad buttons: each name (a string address) and its
    /// button byte (the pad state's `+0xd + button`).
    pub const BUTTONS: [(u32, u32); 10] = [
        (0x00b4_0290, 0xd),
        (0x00b2_bf3c, 0xc),
        (0x00b4_0298, 0xe),
        (0x00b4_02a4, 0xb),
        (0x00b8_f9bc, 0x14),
        (0x00b4_02ac, 0x13),
        (0x00b8_f8a0, 0x17),
        (0x00b8_f8a4, 0x19),
        (0x00b8_f890, 0x1a),
        (0x00b8_f898, 0x18),
    ];
    /// `Phyre_PhyrePrintf` (cdecl) and its "unknown button" message.
    pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
    pub const MSG_UNKNOWN_BUTTON: u32 = 0x00b4_02b4;
    /// `0x0062c730(app, name, 0)` (cdecl), the press command's fallback.
    pub const BUTTON_FALLBACK: u32 = 0x0062_c730;

    /// The button of `name` (`_stricmp` against [`BUTTONS`] in order).
    fn button(h: &mut impl Host, name: u32) -> Option<u32> {
        for (text, button) in BUTTONS {
            let text = h.rebase(text);
            if import(h, STRICMP_IAT, Conv::Cdecl, &[name, text]) == 0 {
                return Some(button);
            }
        }
        None
    }

    /// An unknown button reported (`Phyre_PhyrePrintf(2, msg, name)`); 5.
    fn unknown_button(h: &mut impl Host, name: u32) -> u32 {
        let msg = h.rebase(MSG_UNKNOWN_BUTTON);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, msg, name]);
        5
    }

    /// `PApplicationScriptProxy_buttonRelease(app, name)` (cdecl): with
    /// an application, a name and a pad (`[app + 0x2b8] + 0x38`), the
    /// named button released (0) or, for an unknown name, 5 reported;
    /// else 0.
    pub fn button_release(h: &mut impl Host, app: u32, name: u32) -> u32 {
        if app == 0 || name == 0 {
            return 0;
        }
        let devices: u32 = h.read(app.wrapping_add(0x2b8));
        let pad: u32 = h.read(devices.wrapping_add(0x38));
        if pad == 0 {
            return 0;
        }
        let Some(b) = button(h, name) else {
            return unknown_button(h, name);
        };
        h.write(pad.wrapping_add(b + 0xd), 0u8);
        0
    }

    /// `0x00623060(app, name)` (cdecl, the press command): with an
    /// application and a name: with a pad, the button pressed (1) or, for
    /// an unknown name, 5 reported; then the application's vtable slot 7
    /// (its `eax` when not 0), else `0x0062c730(app, name, 0)`'s.
    pub fn button_press(h: &mut impl Host, app: u32, name: u32) -> u32 {
        if app == 0 || name == 0 {
            return 0;
        }
        let devices: u32 = h.read(app.wrapping_add(0x2b8));
        let pad: u32 = h.read(devices.wrapping_add(0x38));
        if pad != 0 {
            let Some(b) = button(h, name) else {
                return unknown_button(h, name);
            };
            h.write(pad.wrapping_add(b + 0xd), 1u8);
        }
        let r = h.call_virtual(app, 7, &[]);
        if r != 0 {
            return r;
        }
        h.call_sym(BUTTON_FALLBACK, Conv::Cdecl, &[app, name, 0])
    }

    /// `Input_IsXInputDevice(&guid)` (cdecl, `al`), `PInputDevicePad_ctor`
    /// (thiscall, `ret 4`), the two pad vtables, `c_dfDIJoystick2`,
    /// `g_XInputPadCount`, the two messages, `Input_EnumAxesSetRange`.
    pub const IS_XINPUT: u32 = 0x0062_44f0;
    pub const PAD_CTOR: u32 = 0x0062_01a0;
    pub const XINPUT_PAD_VTABLE: u32 = 0x00b3_ff9c;
    pub const DI_PAD_VTABLE: u32 = 0x00b3_ff80;
    pub const DF_JOYSTICK: u32 = 0x00b6_a624;
    pub const XINPUT_COUNT: u32 = 0x00cc_a30c;
    pub const MSG_XINPUT_PAD: u32 = 0x00b4_1058;
    pub const MSG_DI_PAD: u32 = 0x00b4_1078;
    pub const ENUM_AXES: u32 = 0x0062_33a0;
    /// The frame of `Input_AddPadDevice`: the device at `ebp - 0x10`.
    pub const PAD_FRAME: u32 = 0x10;

    /// Whether the pad numbered `index` among the type-2 devices of
    /// `g_PInputDeviceList` (in list order) has `instance`'s product
    /// GUID (`+4`, 16 bytes) at its `+0x130`. A device of another index
    /// is skipped; a mismatch ends the walk.
    fn pad_known(h: &mut impl Host, index: u32, instance: u32) -> bool {
        let head = h.rebase(LIST_HEAD);
        let mut link: u32 = h.read(head);
        let mut seen = 0u32;
        while link != head && link != 0 && link != 4 {
            let node = link.wrapping_sub(4);
            if h.call_virtual(node, 1, &[]) == 2 {
                if seen == index {
                    return (0..4u32).all(|k| {
                        h.read::<u32>(instance.wrapping_add(4 + 4 * k))
                            == h.read::<u32>(node.wrapping_add(0x130 + 4 * k))
                    });
                }
                seen = seen.wrapping_add(1);
            }
            link = h.read(node.wrapping_add(4));
        }
        false
    }

    /// The device instance (0x244 bytes) copied into the pad at `+0x12c`.
    fn copy_instance(h: &mut impl Host, pad: u32, instance: u32) {
        for k in 0..0x91u32 {
            let w: u32 = h.read(instance.wrapping_add(4 * k));
            h.write(pad.wrapping_add(0x12c + 4 * k), w);
        }
    }

    /// `Input_AddPadDevice(instance)` (cdecl; both callers drop `eax`):
    /// nothing when one of the first 18 pads already is this device
    /// ([`pad_known`]). An `XInput` device (`Input_IsXInputDevice(+0x14)`)
    /// is counted (`g_XInputPadCount`), and while fewer than 4 are,
    /// reported and given a `PInputDevicePadXInput` (0x3c4 bytes: its
    /// index `+0x3b0`, `+0x3b4..=+0x3c0` 0, the instance). Any other is
    /// reported and opened through `DirectInput` (`c_dfDIJoystick2`, the
    /// cooperative level, the axis ranges by `EnumObjects`, acquired) as
    /// a `PInputDevicePadDirectInput` (0x3b0 bytes, the instance).
    pub fn add_pad_device(h: &mut impl Host, ebp: u32, instance: u32) {
        for index in 0..0x12u32 {
            if pad_known(h, index, instance) {
                return;
            }
        }
        if h.call_sym(IS_XINPUT, Conv::Cdecl, &[instance.wrapping_add(0x14)])
            .to_le_bytes()[0]
            != 0
        {
            let count_at = h.rebase(XINPUT_COUNT);
            let count: u32 = h.read(count_at);
            if count < 4 {
                let msg = h.rebase(MSG_XINPUT_PAD);
                let g = |k: u32| instance.wrapping_add(4 * k);
                let guid: [u32; 4] = [h.read(g(1)), h.read(g(2)), h.read(g(3)), h.read(g(4))];
                h.call_sym(
                    PHYRE_PRINTF,
                    Conv::Cdecl,
                    &[2, msg, guid[0], guid[1], guid[2], guid[3]],
                );
                let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x3c4]);
                if p != 0 {
                    let index: u32 = h.read(count_at);
                    h.call_sym(PAD_CTOR, Conv::Thiscall(p), &[0]);
                    h.write(p.wrapping_add(0x3b0), index);
                    let vt = h.rebase(XINPUT_PAD_VTABLE);
                    h.write(p, vt);
                    for off in [0x3b4u32, 0x3b8, 0x3bc, 0x3c0] {
                        h.write(p.wrapping_add(off), 0u32);
                    }
                    copy_instance(h, p, instance);
                }
            }
            let count: u32 = h.read(count_at);
            h.write(count_at, count.wrapping_add(1));
            return;
        }
        let msg = h.rebase(MSG_DI_PAD);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, msg]);
        let slot = ebp.wrapping_sub(PAD_FRAME);
        let di: u32 = h.read(h.rebase(DIRECT_INPUT));
        if com(h, di, 0xc, &[instance.wrapping_add(4), slot, 0]).cast_signed() < 0 {
            return;
        }
        let dev: u32 = h.read(slot);
        let format = h.rebase(DF_JOYSTICK);
        if com(h, dev, 0x2c, &[format]).cast_signed() < 0 {
            return;
        }
        let dev: u32 = h.read(slot);
        let hwnd: u32 = h.read(h.rebase(HWND));
        if com(h, dev, 0x34, &[hwnd, 6]) == 0x8000_4001 {
            return;
        }
        let dev: u32 = h.read(slot);
        let cb = h.rebase(ENUM_AXES);
        if com(h, dev, 0x10, &[cb, dev, 0]).cast_signed() < 0 {
            return;
        }
        let dev: u32 = h.read(slot);
        com(h, dev, 0x1c, &[]);
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x3b0]);
        if p != 0 {
            let dev: u32 = h.read(slot);
            h.call_sym(PAD_CTOR, Conv::Thiscall(p), &[dev]);
            let vt = h.rebase(DI_PAD_VTABLE);
            h.write(p, vt);
            copy_instance(h, p, instance);
        }
    }
}
