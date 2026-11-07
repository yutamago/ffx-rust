# The PC platform: window, input, settings, Steam and movies

This is the census of the last M4 parts (`docs/rewrite-plan.md`, M4: movies,
input and the window), written the way `docs/formats/scene.md` is for the
scene: every place `FFX.exe` reaches the operating system, Steam or the
movie codecs, what it does there, and what a Rust replacement has to give
back to stay identical. Audio (FMOD) is `docs/formats/audio-platform.md`'s;
the D3D11 drawing is `scene.md`'s and its successors'. Crate:
[`crates/ffx-platform`](../../crates/ffx-platform).

Everything here is static (the Ghidra dump, `objdump`, `tools/refs.py`, the
game's data); nothing was measured in a running game. "Guess" marks what
the code does not prove.

```bash
# the import census (the appendix below): every import of the platform DLLs
# and every import the movie and codec code calls, with each calling function
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-platform --example census
# the gate: every such import and caller is named in this file
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test --release -p ffx-platform
```

The census finds an import through its IAT slot: a base relocation in
`.text` whose dword is the slot is an instruction that reads it (`call
[slot]`, `jmp [slot]`, `mov reg, [slot]`), which is exact because `FFX.exe`
has a relocation table. A `jmp [slot]` is an import thunk, and its callers
are the `call rel32`s aimed at it. A site belongs to the nearest function
start at or before it: `symbols/ffx.sym`'s functions, code pointers in data,
`push offset` immediates, and 16-byte boundaries after int3 padding that
follows a `ret`/`jmp`, or that open with `push ebp; mov ebp, esp`. On
the 276 sites the Ghidra dump also covers, the census and the dump agree
except where the dump is wrong (code it left outside every function body,
such as `0x006f17d0` and `0x00623500`).

## What the platform is

| Part | Code | DLLs | Replaced by |
|---|---|---|---|
| Window and main loop | PhyreEngine's Win32 framework (`Phyre_PFramework_*`, `Game_Main`) | USER32, IMM32, SHELL32, SHLWAPI, WINMM, KERNEL32 | Rust window and loop |
| Input | Phyre's `PInputDevice*` over DirectInput 8 and XInput 9.1.0; Virtuos' `InputManager` | DINPUT8, XINPUT9_1_0, ole32/OLEAUT32 (WMI) | Rust input |
| Settings | `Settings_*` (`GameSetting.ini`) | MSVCR110 (`_wfopen`, iostreams) | M5 (system) |
| Steam | `Steam_Init`, `StatsAndAchievements` | steam_api | Rust, optional |
| Movies | `PhyFMVPlayerManager` (Virtuos), `PVideoPlaybackWin32` (Phyre), libwebm, libvpx 1.4.0, libVorbis 1.3.5, libogg | fmodex (a stream), KERNEL32, MSVCR110 | Rust player |
| Crash and diagnostics | `CrashDump_UnhandledExceptionFilter`, a StackWalker copy | dbghelp, VERSION, ADVAPI32 | not needed for parity |

ADVAPI32's `Crypt*` imports hash VBF names (MD5, `FileSys_Exists`,
`BigFileStream_openFile`, `BigFileHandle_initialize`): that is the
archive's, not the platform's (`docs/formats/vbf.md`).

## The window

### Start-up (`Game_Main` `0x0062ee50`, the WinMain)

1. `SetThreadUILanguage`, `SetUnhandledExceptionFilter(CrashDump_UnhandledExceptionFilter
   0x006268e0)`, `CreateDocumentsFolders_0062ac90` (Documents\Square Enix\FINAL
   FANTASY X&X-2 HD Remaster\, `SHGetFolderPathW(CSIDL_PERSONAL)`).
2. The settings (`Settings_Construct` `0x00401880`, `Settings_Load` `0x00401cb0`, below),
   then `Settings_SetThreadUILanguage` (`0x0062cad0`: jp 0x411, en 0x409, fr 0x40c,
   de 0x407, it 0x410, es 0x40a, kr 0x412, ch 0x7c04). Message boxes read
   their text from `FFX.exe`'s RT_STRING resources, so they follow the language.
3. Single instance: `CreateMutexW(NULL, TRUE, L"9B8F7468-6DE0-4C5A-9209-89D363F8FCC9")`;
   `ERROR_ALREADY_EXISTS` shows string 0x7835 ("Same program is already
   running.", caption L"error", `MB_ICONHAND`) and returns.
4. `CommandLineToArgvW`, the window description from the settings (below),
   and with arguments `Phyre_PFramework_PApplication_parseCommandLine` (`0x0062adc0`):
   `-script= -log= -input= -timestep= -resolution= -msaa= -vsync= -host=
   -fullscreen= -borderless=`, `-h`/`-help`/`-?` print the usage and exit.
   They change the application object, never the settings file.
5. `Steam_Init` (below), then `PApplication_Startup_00629a40`: icon, hook and
   drag globals 0, `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED
   | ES_DISPLAY_REQUIRED)` (no sleep, no screen saver), `Phyre_PhyreInit`
   (which runs `Phyre_PTimerWin32_Initialize` `0x0042fe10`: `timeBeginPeriod(5)`,
   `timeGetTime`, QPC, `GetSystemTimePreciseAsFileTime` by `GetProcAddress`),
   then `PApplication_InitWindowAndApp_00629ab0`: create the window, fill
   `g_WindowDisplayInfo` (`0x00cca310`: +0 HWND, +4 sample count 1, +8
   fullscreen, +9 vsync; `app+0x264` points at it), attach to a host window
   if any, then `PApplication onInit`, which creates the D3D11 device and
   `Input_InitDirectInput`.

The window description is `PApplication +0x26c..` (width +0x26c, height
+0x270 from `Resolution`; MSAA +0x284; vsync +0x288; an existing HWND +0x294;
fullscreen +0x298 = `ScreenMode == 1`); `g_WindowBorderless` `0x00cc9cd1` =
`ScreenMode == 2`. The title is set by `Phyre_PFramework_PApplication_setWindowTitle`
(`0x0062cf60`, from `FFXApplication_initApplication`): "FINAL FANTASY X", or
"FINAL FANTASY X ETERNAL CALM" when `argv[1] == "_ECalm"`.

### `Phyre_PFramework_PWindowWin32Base_createWindow` (`0x00626cb0`)

- `RegisterClassA`: class "PhyreFrameworkClass", style 0, WndProc `0x00625130`,
  icon `LoadIconA(module, app+0x39c)` (`g_hWindowIcon` `0x00cc9cec`),
  cursor `IDC_ARROW`, brush `COLOR_WINDOW + 1`, no menu.
- Fullscreen: `ChangeDisplaySettingsA({32 bpp, w, h}, CDS_FULLSCREEN)`.
- Style: hosted (a readable host client rect) or borderless or fullscreen:
  `WS_POPUP`, else `0x00ca0000` (`WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX`: no
  resize, no maximize), plus `WS_CLIPSIBLINGS | WS_CLIPCHILDREN`.
  `AdjustWindowRect({0, 0, w, h}, style, FALSE)` unless hosted.
- `CreateWindowExA(0, class, title, style, CW_USEDEFAULT, CW_USEDEFAULT,
  w', h', 0, 0, GetModuleHandleA(0), window)`; a popup lands at 0, 0, a
  window wherever Windows puts it. Then `ImmAssociateContext(hwnd, NULL)`
  (IME off), `SetWindowLongA(GWL_USERDATA, window)`, and
  `RegisterDeviceNotificationA(hwnd, {DBT_DEVTYP_DEVICEINTERFACE,
  GUID_DEVCLASS_HIDCLASS}, DEVICE_NOTIFY_ALL_INTERFACE_CLASSES)` (the flag
  makes the GUID moot: every device arrival is reported).

### The swap chain (`PRendererD3D11_initWindow_00595750`, `PRendererD3D11_createDeviceAndSwapChain_00595800`)

`GetClientRect(hwnd)` gives the size. A `D3D11CreateDevice` probe (hardware,
SDK 7) needs feature level 10.0 or more; on 10.0 with MSAA and HDAO both
on, MSAA is set to 0. `D3D11CreateDeviceAndSwapChain(NULL, HARDWARE, 0, 0,
{11_0, 10_1, 10_0}, 3, 7, ...)`: `R8G8B8A8_UNORM`, 60/1 Hz, one buffer,
`DISCARD`, `ALLOW_MODE_SWITCH`, **`Windowed = TRUE` always**, then
`MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER)`. Fullscreen is a mode
change plus a popup window, never DXGI fullscreen.

### Mode switches

- `Phyre_PFramework_PWindowWin32Base_applyScreenMode` (`0x0062bf00`, through
  the thunk `0x0062bef0` from the video options' apply `FFEscMenu_applyVideoSettings`
  `0x006ff290`), from `g_Settings` `ScreenMode` and `Resolution`:
  - 0 window: `ChangeDisplaySettingsA(NULL, CDS_RESET)`, style `0x00ca0000`,
    `SetWindowPos(HWND_NOTOPMOST, 0, 0, w', h', SWP_SHOWWINDOW)`, icon reload,
    `PostMessageA(WM_SETICON, ICON_BIG)`;
  - 1 fullscreen: mode change `CDS_FULLSCREEN`, `WS_POPUP`, `HWND_TOP`;
  - 2 borderless: `CDS_RESET`, `WS_POPUP`, `HWND_NOTOPMOST` at 0, 0 with
    the `Resolution` size (not the desktop's).
- `Phyre_PFramework_PWindowWin32Base_setFocus` (`0x0062a460`, WM_ACTIVATE,
  "[PWindowWin32Base] has focus %d"): fullscreen only; losing focus restores
  the desktop mode (`CDS_RESET`, `HWND_NOTOPMOST`), gaining it sets the mode
  again (`HWND_TOP`).
- `Renderer_SetDisplayMode` (`0x0058df00`, from `PApplication_frame` when
  `app+0x17`): a DXGI mode-list match and `ResizeTarget`.

### The WndProc (`Phyre_PFramework_PWindowWin32Base_WndProc` `0x00625130`)

The window object is `CREATESTRUCT.lpCreateParams` on `WM_CREATE`, then
`GWL_USERDATA`. A registered message "PHYREMSG_HOSTHWND" (`g_MsgHostHwnd`
`0x00cca308`) makes the game a child of the host window it names
(`PWindowWin32_AttachToHost` `0x00623d50`: `SetParent`, then
"PHYREMSG_SENDHWND" back to the host).

| Message | Handling | Returns |
|---|---|---|
| `WM_CREATE` | `g_hWnd` (`0x00cc9ce8`) = hwnd; `ShowWindow(SW_SHOWNORMAL)` | 1 |
| `WM_DESTROY` | `PostQuitMessage(0)` | 1 |
| `WM_SIZE` | unless minimized: `GetClientRect` into `app+0x26c/+0x270`, `app+0x14` = 1 | 1 |
| `WM_ACTIVATE` | `setFocus(WA_ACTIVE or WA_CLICKACTIVE)` | 1 |
| `WM_CLOSE` | `restoreDisplay` (`0x006270a0`: `ChangeDisplaySettingsA(NULL, 0)` in fullscreen, `ReleaseDC`), `DestroyWindow`, then `ShellExecuteW` of `FFX&X-2_LAUNCHER.exe` next to the exe (`GetModuleFileNameW`, `PathRemoveFileSpecW`, `PathCombineW`) | 1 |
| `WM_ERASEBKGND` | nothing | 1 |
| `WM_GETMINMAXINFO` | `DefWindowProcA`, then `ptMaxTrackSize` = (65536, 65536) | DefWindowProc |
| `WM_NCLBUTTONDOWN`, `WM_NCRBUTTONDOWN` | start of a move/size drag: `g_WindowNcDrag` = 1, `PApplication_NotifySuspend_00627850`, `SetWindowsHookExA(WH_MOUSE, PWindowWin32_MouseHookProc 0x006250e0, 0, this thread)`; `DefWindowProcA` | DefWindowProc |
| `WM_NCLBUTTONUP`, `WM_NCRBUTTONUP` | end of the drag: `UnhookWindowsHookEx`, `PApplication_NotifyResume_006278b0`; `DefWindowProcA` | DefWindowProc |
| `WM_SYSKEYDOWN`/`UP` | Alt+F4: `ShowCursor(1)`, `MessageBoxW` string 0x78ac ("Do you want to quit the game?", `MB_OKCANCEL | MB_ICONEXCLAMATION`), `ShowCursor(0)`, OK closes; `VK_F10` and `VK_MENU` are swallowed (no system menu); other keys `DefWindowProcA` | 1 / DefWindowProc |
| `WM_DEVICECHANGE` | `DBT_DEVICEARRIVAL`: `g_InputRescan` (`0x00cc9cd0`) = 1; removal is not handled | 1 |
| others | `DefWindowProcA` | |

The mouse hook posts `WM_NCLBUTTONUP` when it sees `WM_LBUTTONUP` during a
drag (the modal move loop eats the real one), then `CallNextHookEx`.

### The main loop and the way out

`Game_Main` (`0x0062f262..0x0062f3ce`): `PeekMessageA(PM_REMOVE)`; a
message goes through `TranslateMessage`/`DispatchMessageA`; with none, the
frame limiter: the interval is `g_FrameVsyncs` (`0x00c30e88`) x 0.0166833 s
(1/59.94); early, `Sleep`; on time, `App_Frame` (`0x0062e340`), and
`PostMessageA(WM_CLOSE)` when it returns non-zero or the application asks to
quit. `WM_QUIT` ends the loop and turns off the crash dump
(`g_CrashDumpEnabled` `0x00c30f90`). After it: the renderer's sync, the
framework's terminate, `Phyre_PhyreExit` (`Phyre_PTimerWin32_Terminate`
`0x00430040`: `timeEndPeriod(5)`), `SteamAPI_Shutdown`, the settings freed.

Every quit is `WM_CLOSE`, so it always starts the launcher: senders are
`Game_Main`, `ResetToLauncher` (`0x0042f4d0`, after the fatal file-error
boxes), `PostReset` (`0x0042fa70`), the `FFXApplication` destructors
(`0x0042f0f0`, `0x0042f150`) and the static one (`FFXApplication_atexit_dtor`
`0x00af9ba0`), when `0x00c8fae4` is 1.

`graphicProcessWindowsMessage` (`0x00642860`, `PeekMessageW`/`TranslateMessage`/`DispatchMessageW`
one message at a time) pumps messages during the loading and opening
screens (`PhyreScene_renderLoadingScreen`, `PhyreScene_renderOpeningScreen`),
which `ExitProcess(0)` on `WM_QUIT`.

Error boxes, all `MessageBoxW` after `ShowCursor(1)` with caption L"error"
and `MB_ICONHAND`: 0x7836 "Broken game data, please repair or restore the
data." (`BigFileStream_fileTaskUncompress` `0x0061ba80`, `Game_InitFileSystem`
`0x006795b0`, `fiosOpen` `0x00679730`); 0x7833 "The file $name$ is missing or
damaged. ..." with the UTF-8 file name (`PhyreScene_textureVideoLoad`
`0x0066fb70`, `PhyreFIOS_LoadMagicfilePrx` `0x006793e0`,
`PhyFMVPlayerManager_fmvPlayer_play` `0x006d8fc0`, `0x006da9b0` (0 refs));
then `ResetToLauncher`.

`CrashDump_UnhandledExceptionFilter` (`0x006268e0`) writes `CoreDump.dmp` in
the current folder (`MiniDumpWriteDump` type 0x1824) and returns
`EXCEPTION_EXECUTE_HANDLER`. The StackWalker copy (`StackWalker_LoadModule`
`0x00687c10` with VERSION, `0x00687850` with `GetUserNameA`) is not reached
from the window code.

### Cursor

- The OS cursor stays visible from the start; only the free camera and the
  Esc menu hide it.
- `FreeCamera_handleInput` (`0x00680030`): turning the camera on centres the
  cursor (`GetClientRect`, `ClientToScreen`, `SetCursorPos`), hides it, and
  clips it to the client rect (4 px less on the right and bottom in window
  mode); off, or during a movie wait, `ClipCursor(NULL)`, `ShowCursor(1)`.
  `FreeCamera_setType` (`0x00680410`) type 0 does the same.
- `FFEscMenu_update` (`0x0068ffb0`): opening saves the clip (`GetClipCursor`),
  unclips and shows; closing restores the clip and hides; each frame
  `ShowCursor` follows the menu's state. `FFEscMenu_showMouseCursor`,
  `FFEscMenu_resume`, `FFEscMenu_QuitConfirmed_0068fcf0` show it.
- `Phyre_PFramework_PInputDeviceMouse_update` (`0x0062d6f0`): in fullscreen
  it clips the cursor to the window each update; see Input.
- `MouseState_createInstance_00a25990` (`0x00a25990`, from `0x00630f40`) and `0x00a25910`
  read `GetSystemMetrics(SM_SWAPBUTTON)` into a mouse singleton
  (`0x01984c68`, +0x78).

### Save paths

`wsprintfW(L"%s%s%03d", g_SaveFolder 0x00ce6ee0, L"ffx_", slot)` names the
saves `...\FINAL FANTASY X\ffx_NNN` (slots 0..199) in
`SaveDataMachine_MergeFileFlags_006efd10` (`0x006efd10`), `SaveDataMachine_ReadFile`
(`0x006f0070`), `SaveDataManager_debugSave_Internal_6f0650` (`0x006f0510`),
`SaveDataMachine_WriteFile` (`0x006f0860`), `SaveDataMachine_ScanFiles`
(`0x006f0a70`) and `SaveDataMachine_ReadHeaders` (`0x006f0c60`);
`SetUpDefaultSaveFolder` (`0x006f0330`) builds the folder with `L"%s%s"` and
`CreateDirectoryW` (`docs/formats/save.md`).

## Input

### Devices

`Input_InitDirectInput` (`0x00624350`, from `PApplication onInit`):
`DirectInput8Create(GetModuleHandleA(0), 0x800, IID_IDirectInput8A, &g_DirectInput8
0x00cc9cd4, 0)`, then `EnumDevices(DI8DEVCLASS_ALL, Input_EnumDevicesCallback 0x00623330,
0, DIEDFL_ATTACHEDONLY)`. A failure ends the start with 9.
`Input_InitDirectInput_copy_006243a0` is an unreferenced copy. The callback
sends mice (type 0x12) to `Input_AddMouse` (`0x00624020`), keyboards (0x13) to
`Input_AddKeyboard` (`0x00623fa0`), and joysticks, gamepads and 1st-person
devices (0x14, 0x15, 0x18) to `Input_AddPadDevice` (`0x006240e0`). On
`g_InputRescan` (a device arrived), `Phyre_PFramework_PInput_Update`
(`0x00625770`) enumerates again with `Input_EnumPadsCallback` (`0x00624d40`, pads
only); `Input_AddPadDevice` skips pads it has (by `guidInstance`, up to 18).

| Interface | Off | Method | Use |
|---|---|---|---|
| `IDirectInput8A` | 0x0c | `CreateDevice` | keyboard, mouse, DirectInput pads |
| | 0x10 | `EnumDevices` | start-up, rescan |
| | 0x14 | `GetDeviceStatus` | DirectInput pad update: connected? |
| | 0x08 | `Release` | `Input_Shutdown` (`0x00625680`) |
| `IDirectInputDevice8A` | 0x2c | `SetDataFormat` | `c_dfDIKeyboard` `0x00b6a41c`, `c_dfDIMouse2` `0x00b6a214`, `c_dfDIJoystick2` `0x00b6a624` |
| | 0x34 | `SetCooperativeLevel` | (`g_hWnd`, `DISCL_NONEXCLUSIVE | DISCL_FOREGROUND`) |
| | 0x10 | `EnumObjects` | pads: `Input_EnumAxesSetRange` (`0x006233a0`) |
| | 0x18 | `SetProperty` | `DIPROP_RANGE` -255..255 on every pad axis |
| | 0x1c / 0x20 | `Acquire` / `Unacquire` | once at creation (and after `DIERR_INPUTLOST`) / destructors |
| | 0x24 | `GetDeviceState` | keyboard 256 bytes, mouse `DIMOUSESTATE2`, pad `DIJOYSTATE2` |
| | 0x64 | `Poll` | pads, before `GetDeviceState` |

No buffered input, no deadzone property, no force feedback through
DirectInput; focus loss never unacquires.

XInput pads: `Input_AddPadDevice` asks `Input_IsXInputDevice` (`0x006244f0`,
Microsoft's "IsXInputDevice" sample: WMI `CoInitialize`,
`CoCreateInstance(CLSID_WbemLocator)`, `root\cimv2`, `CoSetProxyBlanket`,
`Win32_PNPEntity` device IDs with "IG_" and `VID_`/`PID_` equal to the
product GUID's first dword; `CoUninitialize`). A match makes a
`PInputDevicePadXInput` (vtable `0x00b3ff9c`) with user index
`g_XInputPadCount` (`0x00cca30c`) while it is below 4: **the index is the
enumeration order, not the controller's XInput slot**. Without a pad the
game never calls XInput.

| Class | vtable | Update | State |
|---|---|---|---|
| Mouse | `0x00b3fc14` | `Phyre_PFramework_PInputDeviceMouse_update` `0x0062d6f0` | x, y, wheel x 255 (+0x10c..); left +0x118, middle +0x119, right +0x11a |
| Keyboard | `0x00b3fc30` | `Keyboard_Update` `0x0062d5c0` | raw [Phyre key] +0x218, previous +0x118, latched +0x18 |
| Pad (DirectInput) | `0x00b3ff80` | `PInputDevicePadDirectInput_update` `0x0062d9d0` | axes +0x370.. (±255), buttons +0x398[16] |
| Pad (XInput) | `0x00b3ff9c` | `PadXInput_Update` `0x0062dba0` | the same, user index +0x3b0, last `XINPUT_STATE` +0x3b4 |
| Touch | `0x00b3fc4c` | `0x0062de50` | pointer events (`docs/formats/pc-ui.md`) |

Mouse absolute mode (+0x108 == 1): `GetCursorPos` less the window rect
(`GetWindowRect`) and, from `GetWindowInfo`'s style, the frame
(`SM_CXFRAME`/`SM_CYFRAME`) and caption (`SM_CYCAPTION`), x 255; buttons
count only inside the client rect. The mouse and touch serve only the PC
screens and the free camera; the game logic gets no mouse input.

### Pad buttons and axes

Phyre pad button `i` (id `0xb + i`) and its sources:

| i | PS2 | XInput `wButtons` | DirectInput |
|---|---|---|---|
| 0 | Square | X 0x4000 | button 0 |
| 1 | Cross | A 0x1000 | 1 |
| 2 | Circle | B 0x2000 | 2 |
| 3 | Triangle | Y 0x8000 | 3 |
| 4, 5 | L1, R1 | LB 0x100, RB 0x200 | 4, 5 |
| 6, 7 | L2, R2 | left/right trigger != 0 | 6, 7 |
| 8, 9 | Select, Start | BACK 0x20, START 0x10 | 8, 9 |
| 10, 11 | L3, R3 | LTHUMB 0x40, RTHUMB 0x80 | 10, 11 |
| 12-15 | Up, Right, Down, Left | DPAD 0x1, 0x8, 0x2, 0x4 | POV (each direction ±45°) |

XInput (`PadXInput_Update`): `XInputGetState(index)` every frame, decoded
only when `dwPacketNumber` changes: `LX = sThumbLX / 128` (C division,
toward 0), `LY = -(sThumbLY / 128)`, the same for the right stick; a failure
zeroes everything. DirectInput: lX, lY to the left stick, lZ, lRz to the
right. Deadzone (`PInputMapper_getStickAxis` `0x00628470`, threshold 0.2 per
axis mapping from `PInputMapper_attachPads` `0x00626aa0`): an axis reads 0
below 0.2 unless the stick's other axis is already past 0.2.

Vibration: `XInputSetState(index, {L = (u16)(byte +0x58 * 65535.0), R =
(u16)(byte +0x59 * 65535.0)})` every frame. Bytes 1..255 overflow the u16
(the result is 0x10000 - v), so in effect the motors are on or off. The
bytes come from the PS2 code: `Pad_ReadAndUpdate` `0x00888f90` ->
`inputSetVibration` `0x00630e20` -> `Virtuos_InputManager_setVibration`
`0x006329c0` -> `PInputDevicePad_setVibration(work+0xfe, work+0xff)`, only
while the last input came from the pad (`InputManager +0xa8`). DirectInput
pads never rumble.

### The pad word

`Input_BuildPadMap` (`0x006315c0`) pairs bindings with bits
(`InputManager +0x9c`, 75 pairs). A binding of `GameSetting.ini`'s
`ffxKeyBinding` is a Phyre key `v - 0x1c` when `v >= 0x1c` (keys 0-25 A-Z,
26-35 digits, 36 Esc, 37-48 F1-F12, 49 Enter, 50 Backspace, 51 Tab, 52
Space, 63-66 arrows, 67 Home, 68 End, 69 PgUp, 70 PgDn, 73-88 the keypad,
89-94 Ctrl/Shift/Alt, 95 grave; DIK codes `g_PhyreKeyToDik` `0x00c30e90`), a
pad button `v - 0xb` for 0xb..0x1a, nothing for 0.

| Pairs | Default (ffx) | Bits |
|---|---|---|
| 0-3 | Up, Down, Left, Right | 0x1000, 0x4000, 0x8000, 0x2000 |
| 4-7 | C, X, V, Z | Circle 0x20, Cross 0x40, Triangle 0x10, Square 0x80 |
| 8-11 | S, PgUp, D, PgDn | L1 0x4, L2 0x1, R1 0x8, R2 0x2 |
| 12-13 | B, N | Start 0x800, Select 0x100 |
| 14-27 | the pad's d-pad, face buttons, L1 L2 R1 R2, Start, Select | the same bits (shoulders also 0x08000000, 0x10000000, 0x02000000, 0x04000000) |
| 28-29, 33-34 | F1, F4, F2, F3 | 0x10000, 0x20000, 0x200000, 0x400000 (F1: speed-up) |
| 31 | `ffx2KeyBinding[31]` (F5) | 0x80000 |
| 36-39 | Home, End, A, Q | 0x2000000, 0x4000000, 0x8000000, 0x10000000 |
| 40 | Esc | 0x1000000 (PC menu) |
| 49-62, 63-74 | second keys for 0-13 and 28-40 (Enter = Circle, Backspace = Cross) | as their first keys |

The bits 0-15 are the PS2 pad word with its bytes swapped
(`docs/harness.md`, "Input facts"). L3 and R3 have no binding: the pad's
stick clicks never reach the game. `Virtuos_InputManager_updateInput`
(`0x00632a30`) builds the word once a frame:

1. key aliases (`g_KeyAliases` `0x00c32270`: right Alt/Ctrl/Shift and
   keypad Enter count as the left ones and Enter);
2. an override callback (+0xb4) may veto the update;
3. with the free camera on (Esc menu closed, not paused) only the
   booster/Esc pairs count and the sticks are 0;
4. else: the sticks (`PInputMapper_getStickAxis`, pad 0); the pad pairs
   14-27 (`+0xa8` = 1); only when no pad bit is set and the left stick is
   centred, the keyboard pairs 0-13 and 49-62 (`+0xa8` = 0); always 28-40
   and 63-70; Eternal Calm scenes zero the word unless the Esc menu is open
   or L1+R1+Start are held; while paused the booster bits are cleared;
5. `+4` = old word, `+0` = word, sticks to +0x8c..+0x98 (or, with
   `0x0133c930` set, to the `AsyncControllerTaskManager` queue);
6. hold timers (`InputManager_UpdateHoldTimers` `0x006330e0`): repeat after
   0.2333 s, then every 0.1 s.

The PS2 code reads it through `Pad_ReadAndUpdate` (`0x00888f90`:
`Pad_ReadCurrDevice` cancels opposite directions; `Pad_UpdateAnalog`
`0x00889570` turns the sticks into PS2 bytes, `0x80 - ftol(v * -127)`;
pressure bytes are fixed), `TkScanControler` and the `TkGetControler*`
family; the PC screens through `inputIsPressButton` and the raw-key
`Input_KeyHeld` (`0x00629d50`) / `Input_KeyDown` (`0x00629ca0`, really "pad
button held"). The `scePad*` stubs (`0x0062fb10..`) return 0.

### When

All on the main thread, once per frame, in `App_Frame`:
`Phyre_PFramework_PInput_Update` (the rescan, then each device's update:
every `GetDeviceState`, `Poll`, `XInputGetState` and `XInputSetState`), the
mapper (`PInputMapper_update` `0x0062e040`), then in
`Phyre_PFramework_PApplication_frame` the latch (`PInputMapper_latchButtons`
`0x00628320`: a button's latched byte changes only when its raw value
changes, which is why injected presses persist) and
`FFXApplication_handleInputs` -> `Input_UpdateAll_00630e80`, before
`FFXApplication_animate` runs the logic. The vibration chosen in frame n is
sent in frame n + 1. The `AsyncControllerTaskManager` thread
(`AsyncControllerTaskManager_ThreadMain` `0x0068d740`) re-runs the mapper at
60 Hz only while `0x0133c930` is 1 (Lulu's overdrive keyboard mode,
`setLuluODKeyboardInput`), on device state the main thread polled.

## The settings file

`Documents\Square Enix\FINAL FANTASY X&X-2 HD Remaster\GameSetting.ini`,
an object of 0x63c bytes at `*g_Settings` (`0x022fb504`). No
`GetPrivateProfileString`: `Settings_Load` (`0x00401cb0`) reads the file
with `_wfopen(L"rb")` and `Settings_Parse` (`0x00401e00`) splits it with an
`istringstream` and `getline`, `key=value`, keys matched with `_stricmp`
(`g_SettingsKeyNames` `0x00c8a248`), values by `Settings_SetValue`
(`0x00401930`: the enum names at `g_SettingsValueStrings` `0x00c89f48 + 0x20 key`,
else `atoi`; `Resolution` `W*H` packed `W << 16 | H`; binding lists comma
separated, validated by `Settings_ValidateBindings` `0x00405bb0`). The
buffer is not NUL-terminated before `strlen`, so stack bytes after the file
can make one more (invalid) line. Messages go to stdout ("[ConfigFile]
invalid key at [%s].").

| # | Key | Values (stored index) | Preset L/M/H/C | Read by |
|---|---|---|---|---|
| 0 | `Language` | jp en fr de it es kr ch | 0 | `Settings_SetThreadUILanguage`, `LocalizationManager_Initialize` |
| 1 | `Resolution` | `W*H` | 1280*720 | the window description; `Renderer_SetDisplayMode`; mode switches |
| 2 | `ScreenMode` | SM_WINDOW SM_FULLSCREEN SM_BORDERLESS | 0 | window creation and switches, cursor clipping |
| 3 | `Quality` | VQ_LOW VQ_MEDIUM VQ_HIGH VQ_CUSTOM | | picks the preset for 11-18 |
| 4 | `Mute` | Off On | 0 | `iSoundUpdate` |
| 5-8 | `MasterVolume`, `VoiceVolume`, `MusicVolume`, `SFXVolume` | 0..100 | 100 | `iSoundUpdate` (master / 100 also to the movie volume) |
| 9 | `Brightness` | int | 50 | `PhyreScene_ApplySettings` |
| 10 | `VSync` | Off On | 0 | `app+0x288`, `Renderer_SetVSync` |
| 11 | `Anisotropic` | 1 2 4 8 16 | 0/2/4/0 | `Renderer_SetAnisotropy` (`0x0058de30`, given the index) |
| 12 | `Shadow` | 0-3 | 0/3/1/0 | `PhyreScene_ApplySettings` (shadow manager) |
| 13 | `TextureQuality` | TQ_HIGH TQ_MEDIUM TQ_LOW | 2/1/0/0 | `Phyre_PRenderInterface_setTextureQuality` (`0x0058e000`) |
| 14 | `MSAA` | samples | 0/2/8/0 | `app+0x284`, `PhyreScene_ApplySettings`, the device creation |
| 15 | `ColorCorrection` | Off On | 1 | the video options page only |
| 16 | `PostAA` | PAA_OFF PAA_FXAA PAA_SMAA1..4 | 0/1/5/0 | `PhyreScene_ApplySettings` |
| 17 | `UnSharpMask` | Off On | 0/1/1/0 | `PhyreScene_ApplySettings` |
| 18 | `HDAO` | Off Low Medium High | 0/1/3/0 | `PhyreScene_ApplySettings`, the device creation |
| 19 | `Invincible` | Off On | 0 | nothing in `FFX.exe` |
| 20 | `EncounterRate` | ER_OFF ER_NORMAL ER_HIGH | 1 | nothing in `FFX.exe` |
| 21 | `ffxKeyBinding` | 81 ints | `0x00c8a538` | `Input_BuildPadMap`, button icons, the binding pages |
| 22 | `ffx2KeyBinding` | 81 ints | `0x00c8a3f0` | kept and written back (slot 31 is used, above) |
| 23 | `lmKeyBinding` | 81 ints | `0x00c8a2a8` | kept and written back (slots 41-48 reach the pad map, bits 0) |

Presets are `g_SettingsPresets` (`0x00b7cba8`, [key][LOW, MEDIUM, HIGH,
CUSTOM]). A missing key gets the preset of the file's `Quality`, and **keys
11-18 take the preset whatever the file says unless `Quality=VQ_CUSTOM`**.
Unless the language is jp, kr or ch, the default binding tables swap slots
18 and 19 (confirm and cancel); a missing binding key resets all three
lists to the defaults.

Writing (`Settings_Save` `0x00401d80`, `Settings_Format` `0x004022f0`:
`Key=Value` with LF line ends, in key order): when the file is missing
(defaults: jp, 1280*720, window, VQ_LOW), when the ffx bindings were reset
(`PhyreScene_initScene`), and when the Esc menu closes or quits
(`FFEscMenu_resume`, `FFEscMenu_update`, `FFEscMenu_QuitConfirmed_0068fcf0`). The
options pages change the object in memory (`SetWordAt_00402a00`;
`FFEscMenu_applyVideoSettings`, then `Settings_DetectQuality` `0x00402a20`).

The user's file has CRLF line ends (the launcher writes it; the game would
write LF). Whether the `\r` stays in the values is open (below); with
`Quality=VQ_LOW` (the harness sandbox) the question does not change the
result for keys 11-18.

## Steam

`steam_api.dll` (interfaces `SteamUser017`, `SteamUtils007`,
`STEAMUSERSTATS_INTERFACE_VERSION011`). `Steam_Init` (`0x00623470`, once from
`Game_Main`): `SteamAPI_RestartAppIfNecessary(0)` (app id 0: never
restarts), `SteamAPI_Init()`; on success `g_SteamReady` (`0x00cc9cf5`) = 1,
`SteamUtils()->SetOverlayNotificationPosition(top right)` (+0x28), and
`SteamUser()->BLoggedOn()` (+0x04) for a log line. Every other Steam call
waits on `g_SteamReady`, except `SteamAPI_Shutdown` at the end of
`Game_Main`. `FFXApplication_animate` calls `SteamAPI_RunCallbacks` first
thing each frame.

`StatsAndAchievements` (0x50 bytes, `g_StatsAndAchievements` `0x00ce81a8`),
created by `StatsAndAchievements_Create` (`0x006f1430`) from
`Game_InitSystems_00647fe0` only with Steam. Its constructor
(`StatsAndAchievements_ctor` `0x006f11a0`) takes the app id
(`ISteamUtils::GetAppID`, +0x24) and registers three callbacks
(`SteamAPI_RegisterCallback`): `UserStatsReceived_t` (1101,
`StatsAndAchievements_OnUserStatsReceived` `0x006f15f0`: `+0x4c` = 1, then
per record `GetAchievement` (+0x18) and the display name and description
(`GetAchievementDisplayAttribute`, +0x30)), `UserStatsStored_t` (1102) and
`UserAchievementStored_t` (1103), both empty; then `RequestCurrentStats`
(+0x00) when logged on. The `CCallback` constructors, `Register` and
destructors are `0x006f1060..0x006f1350` and `0x006f16e0..0x006f1780`;
`StatsAndAchievements_Destroy` (`0x006f14a0`) is never called, so the
callbacks stay registered until `SteamAPI_Shutdown`.

Unlocking: `achievementUnlockAchievement` (`0x00822380`) ->
`SaveDataUnlockAchievement` (`0x006488c0`, with Steam) ->
`StatsAndAchievements_Unlock_006f1510` -> `StatsAndAchievements_SetAchievement_006f1900`:
`SetAchievement(API name)` (+0x1c) and `StoreStats` (+0x28), then
`AchievementTryObtainPlat`. There are 34 records (`0x00c35790`: id 0
`ACH_COMPLETION_OF_FFX`, the platinum, to 0x21 `ACH_A_JOURNEY_CATALYST`).
The debug tweak menu's item 0xc calls `StatsAndAchievements_ResetAll`
(`0x006f1420`: `ResetAllStats(true)`, +0x54). Without Steam the game runs
the same, minus the achievements; nothing outside Steam keeps them.

## Movies

The PC player is PhyreEngine's `PVideoPlaybackWin32` driven by Virtuos'
`PhyFMVPlayerManager`; the PS2 movie API (`movie_*`) is dead code
(`docs/formats/movie.md` has the PS2 parts that remain).

### Code

| Range | What |
|---|---|
| `0x00409370..0x0041e5b0` | libvpx 1.4.0, C: the `vpx_codec` API, the VP8 decoder (its unused multithreaded part included), `vpx_scale`, `vpx_mem`, run-time CPU detection (`vpx_setup_rtcd_0040d2c0`). VP8 only ("WebM Project VP8 Decoder v1.4.0") |
| `0x0041e5b0..0x0042e1d0` | libwebm's `mkvparser` (libvpx's `third_party` copy, asserts compiled in) |
| `0x0042e1d0..0x0042ee40` | libogg's `bitwise.c` only |
| `0x006d6970..0x006daec0` | `PhyFMVPlayerManager` |
| `0x00a26e70..0x00a28890` | `PVideoPlaybackWin32` |
| `0x00a3dd20..0x00a3e3e0` | `CDataFrameQueue` (blocking ring buffers) and `VbfMkvReader` (an `IMkvReader` over Phyre's `PStreamFile`) |
| `0x00a7c920..0x00a8d6be` | libvpx's x86 DSP routines (MMX/SSE2/SSSE3) |
| `0x00a8ee40..0x00aa51e0` | libVorbis 1.3.5 |

Phyre replaces the allocators of libvpx (`vpx_mem_set_functions`
`0x0040a5d0`), mkvparser (`mkvparser_SetAllocator` `0x0041e5b0`) and libogg
(`0x0042e1d0`). The files are ffmpeg-muxed WebM: VP8 1280 x 720 at 29.97 fps
with Vorbis stereo 48 kHz; the `*_480*.webm` texture videos are 640 x 480
without sound.

### From the game to the player

Script natives `0xb000`-`0xb00c` (`0x0076e910..0x0076ed10`) and the event
code call the `graphic*` wrappers, which call the manager
(`g_FMVPlayerManager` `[0x00cded3c]`, 0x750 bytes):

| Wrapper | Manager | Player |
|---|---|---|
| `graphicInitScene` | `PhyFMVPlayerManager_createVideoIndexMap` `0x006d82f0` (reads `FFX_VideoList.txt` of JP, US or Asia by the localization) | |
| `graphicInitFMVPlayer(movie, loop)` `0x00641690` | `PhyFMVPlayerManager_initialize` `0x006d9c80` -> `PhyFMVPlayerManager_fmvPlayer_initialize` `0x006d8f30` | `PVideoPlaybackWin32_initialize` `0x00a27cf0` (FMOD system, 30-frame queue, reading thread) |
| `graphicPlayVideo` `0x006427c0` | `PlayVideo` `0x006d7430` -> `PhyFMVPlayerManager_fmvPlayer_play` `0x006d8fc0` | `PVideoPlaybackWin32_open(path, loop)` `0x00a27f20` -> 0, -1, -2, -3 (file: error box 0x7833 and `ResetToLauncher`) |
| `graphicVideoUpdate` `0x00645df0` (each `Sg_MainLoop` update) | `handleInput` `0x006d9460` (skip), `UpdateTexture` `0x006d7680` | `PVideoPlaybackWin32_isPlaying` `0x00a27c50`, `updateTexture` `0x00a284a0`, `getCurrentTime` `0x00a27bd0`, `getEndTime` `0x00a27c40` |
| `PhyreScene_RenderVideoAndItsUI` `0x00653960` | `Render` `0x006d7570` | |
| `graphicVideoPause` `0x00645de0` | `pauseMovie` `0x006da4b0`, `PhyFMVPlayerManager_resumeMovie` `0x006d8f80` | `PVideoPlaybackWin32_togglePause` `0x00a27e90` |
| `iSoundUpdate` `0x0067a600` | `PhyFMVPlayerManager_SetVolume_006dabf0` (master volume / 100) | `PVideoPlaybackWin32_setVolume` `0x00a28330` |
| `graphicDestroyFmv` `0x0063df30` | `PhyFMVPlayerManager_DestroyFMVPlayer` `0x006d71b0` | `PVideoPlaybackWin32_terminate` `0x00a28430` |
| `graphicIsVideoPlaying` `0x00641cf0`, `graphicGetCurrentFrame` `0x006410d0`, `graphicisVideoAtEndFrame` `0x00645e50` | `+0x6d0`, `+0x6d8`, `PhyFMVPlayerManager_isVideoAtEndFrame` `0x006da240` | |

`graphicInitFMVPlayer` returns `PhyFMVPlayerManager_initialize`'s `eax
!= 0`, and `initialize` returns 0 on every path that sets the player up
(`0x006da184: xor eax, eax`); only `PTextureD3D11_register_0058cf80`'s
error skips that store (`0x006d9e67`). The normal movie natives ignore the
result. `Native_b00a_MovieStartPicture` (`0x0076eb80`) does not: at
`0x0076ebb4` (`74 28`, `je`) a 0 skips `graphicPlayVideo(0)`,
`g_WaitFramesByTime`, `g_MovieDone` and `g_MoviePictureOn`, so a loaded
picture movie is never played. Its only real user is the Farplane
(`ikai0800`, worker 25 entry 0: movie 104, flags `0x480`, `0x80` the
loop; the others are the dev maps `test01`/`test11`), whose background
video therefore never shows on PC. ffx-demaster (`docs/patches.md`) and
Nexus mod 355 replace the branch with `90 90`.

Texture videos (video on map geometry, no sound) use a second player in
the scene (`scene+0xde8`, `PhyreScene_textureVideoLoad` `0x0066fb70`,
`PhyreScene_UpdateTextureVideo` `0x00654960`).

The manager's state the game reads: `+0x6d0` playing, `+0x6d8` the current
frame (0xfffe when ended or skipped), `+0x6e0` the previous one, `+0x6fc`
paused, `+0x710` ended, `+0x720` hold the last frame, `+0x74d` skippable (16
ids are not: `g_FMVUnskippableIds` `0x00b49b08`); `g_FMVSkipped`
`0x00cded31`, `g_MovieVolume` `0x00c34420`, `g_WaitFramesByTime`
`0x0112a008`.

### Threads, timing and sound

| Thread | Proc | Work |
|---|---|---|
| reading | `PVideoPlaybackWin32_initialize` `0x00a27880` | `PVideoPlaybackWin32_DemuxLoop` `0x00a27650`: mkvparser clusters and blocks, video packets to one queue, Vorbis packets to another; loops from the first cluster when looping |
| decoding | `PVideoPlaybackWin32_open` `0x00a27b90` | `PVideoPlaybackWin32_DecodeLoop` `0x00a279b0`: `vpx_codec_decode`, one `vpx_codec_get_frame`, the Y, U, V planes into a 30-slot I420 queue |
| FMOD stream | `PVideoPlaybackWin32_FmodReadCallback_00a27ee0` | `PVideoPlaybackWin32_AudioRead_00a27170`: 0x1000 frames per call, `vorbis_synthesis_pcmout`, s16 = clamp(floor(f x 32767.0 + 0.5)) interleaved; silent while prebuffering |
| main or render | `UpdateTexture` / `updateVideoCallback` `0x006dae00` | `updateTexture`: one decoded frame into the next of three Y, U, V texture sets (`Map`/`Unmap` or `UpdateSubresource`) |

The sound is an FMOD Ex user stream (`PVideoPlaybackWin32_AudioInit_00a273b0`:
`System::createSound(0, FMOD_OPENUSER | CREATESTREAM | SOFTWARE | 2D,
{PCM16, the track's rate and channels, decode buffer 0x1000, length from the
WebM duration, read callback})`, `playSound`, `setPriority(0)`); the
movie's FMOD calls are in the appendix.

There is no A/V clock. Nothing shows until 30 frames are decoded; then the
sound starts and the video advances one frame per `updateTexture` call (one
per game update, or per render command for ids 0x20, 0x2e, 0x48 and loops),
and taking a frame blocks while the decoder is behind. `getCurrentTime` =
trunc((frames shown + 1) x 1000 / 29.97002997) ms; `+0x6d8` = ftol(f32(ms) x
29.97 x 0.001) (both f32 constants), which the scripts and movie cameras key
on. The end comes when the decoder is done, the stream is `READY` and the
queue is empty, or (not looping) when the time reaches the WebM duration.

Starting a preloaded movie early (replay-flakes7, `docs/harness.md`,
"Movies in lockstep"): `PhyFMVPlayerManager_PlayVideo(0)` (`0x006d7430`)
resumes the preload only once its first frame is taken (`+0x6e4`, set
by `UpdateTexture` `0x006d7680` when `updateTexture` returns a frame,
after the 30-frame prebuffer); before that it opens the movie again
(`0x006d8fc0` -> `PVideoPlaybackWin32_open` `0x00a27f20`,
which first closes the player, `0x00a28350`). The close ends the demux,
which leaves the demux state (player `+0xe8`) at 2, and the reading
thread (`0x00a27880`) runs the demux only while that state is not 2;
only `PVideoPlaybackWin32_initialize` (`0x00a27cf0`) sets it back to 0.
So the reopened movie gets no packets and never shows a frame. Whether
the game's own scripts can start a movie that early is not checked.

### Other files in the player (measured, ffx-remix M10)

Measured 2026-11-07 with ffx-remix's runtime in harness games (its
`tools/probe-movie.sh`): movie 2 as `tools/replays/movie-fmv.txt` plays it
(save `ffx_003`, bsil0000's worker 24 entry 5, the preload held until its
30 frames are decoded), in lockstep: the test-file runs below with the
real clock (its `--realtime`), the open hook's runs with the virtual clock
(two) and the real one (one).

- `PVideoPlaybackWin32_open` (`0x00a27f20`, thiscall(player, path,
  loop)) got the path
  `/FFX_Data/GameData/PS3Data/Video/IWK_us.webm` and loop 0; after it
  returns 0 the width is at `+0x14c` and the height at `+0x150` (1280 and
  720 for `IWK_us.webm`, as ffx-remix's open hook logged them).
- With Fahrenheit serving a test file in place of `IWK_us.webm` (a frame
  number drawn as a barcode in each frame, 29.97 fps, a silent Vorbis
  track): a 1920 x 1080 VP8 file played on to the last shot. In the shots
  taken when `+0x6d8` was 64, 128, 192, 256 and 320 the barcode read 63,
  127, 191, 255 and 319: the frame shown is `+0x6d8` - 1. From the code:
  `updateTexture` uploads frame b while the player's `+0x100` is b and then
  adds one; `updateVideoCallback` (`0x006dae00`) sets the manager's
  `+0x6d8` right after from
  `getCurrentTime`'s (`+0x100` + 1) frames, and the truncation to ms and
  `ftol` of ms x 29.97 x 0.001 bring that down to b + 1 (computed for b =
  0 to 400).
- A 3840 x 2160 VP8 file crashed the game when the script's MovieStart
  loaded it (the preload, before any frame was decoded): an access
  violation writing to address 0 in `MSVCR110.dll` (`+0x31323`, one
  minidump). The main thread's stack: `memset` (the fill `0xcd`) <-
  `user_malloc` (`0x006fb910`, returning to `0x006fb97d`) <- `PhyreMalloc`
  (`0x0042fb40`, returning to `0x0042fb50`) <- the frame queue's set-up
  (`0x00a3ddf0`, returning to `0x00a3de52`) <- `PVideoPlaybackWin32_open`
  (its second queue call, returning to `0x00a28260`) <-
  `PhyFMVPlayerManager_fmvPlayer_play` (`0x006d8fc0`) <- `PlayVideo`
  (`0x006d7430`) <- `graphicPlayVideo(1)` (`0x006427c0`) <-
  `Native_b000_MovieStart` (`0x0076e910`, returning to `0x0076e948`).
  The queue asked for 30 slots of (1.5 x width x height + 4)
  bytes, 373,248,120 in all, and `user_malloc` for 385,689,751 (total plus
  one 1.5 x width x height frame plus `0x1f`); the user heap could not give it, and
  `user_malloc` filled the null block with `0xcd`. At 1080p the block
  would be about 96 MB (computed).

### Drawing

`Render` clears to black and draws one quad with `video.fx`
(`VideoScreen`): `uv = (u, 1 - v)`, three single-channel textures, and the
full-range BT.601 conversion in the pixel shader: R = Y + 1.402 V - 0.701,
G = Y - 0.344 U - 0.714 V + 0.529, B = Y + 1.772 U - 0.886, A = 1. The 3D
scene and effects are drawn over it when the movie has a camera
(`movie_have_camera`, `g_MoviePictureOn`); the UI and the screen fade
follow. Texture videos use `vfxtexturevideo.fx` with the same constants.

### The libraries in Rust (M5 stage 2)

Since m5-movie the three libraries linked into FFX.exe are no longer
called: their entry points are hooked onto `crates/ffx-movie`
(`ported/movie.rs`), and the player around them (`PhyFMVPlayerManager`,
`PVideoPlaybackWin32` but its demux loop, `CDataFrameQueue`,
`VbfMkvReader`, FMOD) stays original for now.

#### What the player calls

| Library | Calls (caller) | What goes in and out |
|---|---|---|
| mkvparser | `SetAllocator(malloc, free)` `0x0041e5b0` (`open`), `ResetAllocator()` `0x0041e5d0` (`close`) | PhyreEngine's addition: `g_mkvparser_malloc`/`_free` `0x022fb510`/`14` |
| | `Segment::CreateInstance(reader, 0, &seg)` `0x0041fd50`, `Load()` `0x00421450`, `GetTracks()` `0x004251b0` (`open`) | the reader is the player's `VbfMkvReader` (`+0x118`): `Read(pos, len, buf)` slot 0, `Length(&total, &avail)` slot 1; the segment at `+0x110` |
| | `Tracks::GetTracksCount()` `0x004294c0`, `GetTrackByIndex(i)` `0x00429de0`, `Track::GetType()` `0x00427b10` (`open`) | type 1 video (kept at `+0x290`), 2 audio (`+0x294`) |
| | `VideoTrack::GetWidth/GetHeight()` `0x00428db0`/`dc0` (i64), `GetFrameRate()` `0x00428dd0` (f64 in `ST0`; 0 means 29.97002997), `Segment::GetDuration()` `0x00425200` (ns, i64; `open` and `AudioInit`) | `+0x14c`/`+0x150` the size, `+0x108` 1000 / fps, `+0xf8` the end time |
| | `Track::GetCodecPrivate(&size)` `0x00427b80`, `AudioTrack::GetSamplingRate()` `0x00429170` (f64), `GetChannels()` `0x00429180` (i64) (`AudioInit`) | the three Vorbis headers; the FMOD stream's format |
| | `DemuxLoop` `0x00a27650`: `Segment::GetFirst/GetNext`, `Cluster::EOS/GetFirst/GetNext`, `BlockEntry::EOS`, `GetBlock` (vtable slot 1), `Block::GetTrackNumber/GetFrameCount/GetFrame(i)`, `Tracks::GetTrackByNumber` | each frame `(pos, len)` is read with the reader into `CDataFrameQueue_Reserve(len)` + 4 of the video queue (player) or the audio queue (`+0x80`), then `Commit`; `delete seg` `0x0041fd10` at `close` |
| libvpx 1.4.0 | `vpx_mem_set_functions` `0x0040a5d0` (Phyre's heap and three copy callbacks), `vpx_codec_vp8_dx()`, `vpx_codec_dec_init_ver(ctx, iface, NULL, 0, 9)` (`open`; ctx at `+0x130`) | |
| | `vpx_codec_decode(ctx, data, size, NULL, 0)` then one `vpx_codec_get_frame(ctx, &iter)` per packet (`DecodeLoop` `0x00a279b0`) | a `vpx_image_t` (I420; planes `+0x24`, strides `+0x34`) for a shown frame, none for a hidden one; the loop copies the visible Y (w x h), U and V (w/2 x h/2) into one 1.5 w h slot of the 30-frame queue |
| | `vpx_codec_destroy`, `vpx_mem_unset_functions` (`close`) | |
| libVorbis 1.3.5 | `vorbis_info_init`, `vorbis_comment_init`, `vorbis_synthesis_headerin` x3, `vorbis_synthesis_init`, `vorbis_block_init` (`AudioInit`) | the headers walked as `CodecPrivate + 3`, then `+ r1`, then `+ r2 + 4` with PhyreEngine's return values (30; the comment header's size - 4; 0); `vorbis_info.channels` (`+4`) is read back by `AudioRead` |
| | per packet `vorbis_synthesis` + `vorbis_synthesis_blockin` (packet `+0x160`: `b_o_s` 1, `e_o_s` 0, granule position 0, packet numbers 1, 2, ...; headers 0), `vorbis_synthesis_pcmout`, `vorbis_synthesis_read` (`AudioRead` `0x00a27170`) | planar float, converted to interleaved s16 by `AudioRead` (`ffx_movie::vorbis_to_s16`) |
| | `vorbis_block_clear`, `vorbis_dsp_clear`, `vorbis_comment_clear`, `vorbis_info_clear` (`AudioShutdown` `0x00a278f0`) | |
| libogg | `SetGlobals_0042e1d0`, `SeveralStoresCopyIntoGlobal_0042e200` (its allocator set and reset) | already ported (auto) |

#### The choice per library, measured

All movies of `FFX_Data.vbf` (95 WebM files: 89 with sound, six texture
videos without), decoded by the candidate and compared with the original.
The original's output comes from FFX.exe's own code run in the oracle
(`cargo run --release -p ffx-oracle --example movie_ref`), set up and fed
as the player does; `cargo run --release -p ffx-movie --example moviecmp`
compares (`vorbis`, `vp8`, `demux` modes).

- **VP8: an existing crate**, zvidlib-vp8 0.5.0 (MIT), vendored in
  `third_party/zvidlib-vp8` because the published crate hands out only RGBA
  (`PATCHES.md`: `pub` on its decoder and picture, the hardware backends
  off). Every frame of every movie, 179,118 frames, is bit-identical
  (per-frame MD5 of the visible I420 planes) to libvpx's (ffmpeg's libvpx
  1.16 decoder; ffmpeg's own VP8 decoder and oximedia-codec 0.2.1, Apache-2.0,
  gave the same on the 74 movies measured with them). Against FFX.exe's
  libvpx 1.4.0 itself, in the oracle (1 to 4 s a frame): 3,805 frames, every
  frame of the six texture videos and of `bsc_jp`, `bgr_jp` and `tht_us`,
  and the first 90 of `sin_us`, `bbb_jp`, `end_us`, `ill_jp`, `dat_us` and
  `fin_jp`, identical to both. The original was not
  run on every frame: the emulator would take days; VP8 decoding is exactly
  specified and libvpx's output has not changed between versions.
  oximedia-codec was not taken: it pulls tokio, serde and rayon in.
- **Vorbis: our own port** of the libVorbis 1.3.5 decoder and libogg's
  bit reader (`ffx_movie::vorbis`, BSD-3-Clause, `crates/ffx-movie/NOTICE`).
  The crates are not exact: lewton 0.10.2 (MIT OR Apache-2.0) differs from
  the original in 331 of `bgr_jp`'s 862,848 s16 samples (and in 70 % of
  the float samples, by at most 1.8e-7), rusty_vorbis 0.1.2 (Apache-2.0) in
  810; symphonia's decoder is MPL-2.0 and was not measured. The port gives
  the original's float samples bit for bit on all 89 movies with sound:
  565,386,368 interleaved samples, f32 and the s16 FMOD gets. Float facts
  of the game's build (MSVC, SSE2 scalar): each float operation rounds to
  float; `_book_unquantize` computes `fabs(q) * delta + mindel + last` in
  double (`0x00a92823..0x00a92871`); libVorbis' `os.h` defines `M_PI` as
  the float `3.1415926536f`, so `mdct_init` (`0x00a9d680`) computes each
  angle as a float product and only `cos`/`sin` in double. Rust's `libm`
  gives the same table floats as MSVCR110's `cos`/`sin` on every block
  size the movies use. Floor type 0 is not ported (no movie uses it; a
  setup with it is refused).
- **WebM: our own demuxer** (`ffx_movie::webm`), reading through the
  game's `IMkvReader` a cluster at a time (the largest movie is 618 MiB).
  Every packet of every movie, 495,903, has the track, size and bytes
  ffmpeg's demuxer gives. matroska-demuxer (Zlib/MIT/Apache) would also
  do; its API reads frames, not `mkvparser`'s objects, so it saves nothing.

#### What is hooked

35 hooks (`ported/movie.rs`, on `ffx_movie::api` and `ffx_movie::mkv`):
the seven libvpx entry points (the allocator table kept as the original
keeps it, so `vpx_mem_set_functions`/`unset` are byte-exact: oracle test
`movie_codecs`), the thirteen libVorbis ones (the decoders keyed by their
`vorbis_info`/`vorbis_dsp_state`/`vorbis_block` addresses), and for
mkvparser `SetAllocator`, `ResetAllocator`, `CreateInstance`, `Load`,
`delete`, `GetDuration`, `GetTracksCount`, `GetTrackByIndex`,
`GetCodecPrivate`, `GetWidth`, `GetHeight`, `GetFrameRate`,
`GetSamplingRate`, `GetChannels` and the player's `DemuxLoop` itself
(PhyreEngine code that is nothing but mkvparser calls). `Segment::GetTracks`
and `Track::GetType` are four bytes, too short for a hook: they run on the
objects `ffx_movie::mkv` lays out like `mkvparser`'s (`+0x74` and `+0x20`),
the other accessors are hooked onto the same fields. The demux loop skips
the frames the original reads into a scratch buffer (`0x01984c80`, audio
without an audio track) and does not print the original's two error lines.

In game: `tools/replays/run-all.sh --extra --hits movie-fmv` (Wakka's kick,
`IWK_us.webm`) runs all 35 hooks (none of the libraries' code), the traces
have the same events and the four shots are identical to the original's.
Its sound: `tools/replays/run-all.sh --audio movie-fmv` (marks from the
movie's start to the third shot) gives 510,400 frames (10.6 s) of FMOD's
float mix identical to both original captures, which agree with each other.
The other replays play no FMV, and none plays a texture video (the scene's
second player, the same hooks).

## What the Rust side must reproduce

- **Window**: the class, styles and sizes above, the three screen modes by
  `ChangeDisplaySettings` and a popup (never DXGI fullscreen), the swap
  chain (one buffer, discard, R8G8B8A8, windowed, no Alt+Enter), the
  WndProc's answers (Alt+F4 box, F10/Alt swallowed, `WM_CLOSE` starting the
  launcher), the frame limiter (`g_FrameVsyncs` x 1/59.94 s with `Sleep`),
  `SetThreadExecutionState`, `timeBeginPeriod(5)`, IME off, the cursor rules.
- **Input**: the device order (DirectInput enumeration order gives the pad
  and XInput indices), the per-frame poll on the main thread before the
  logic, the axis scale (x 255 DirectInput range, /128 XInput) and the 0.2
  cross deadzone, the latch semantics, the pad map and its priority (pad
  first; the keyboard only with no pad bit and a centred stick), the u16
  overflow in the rumble, no rumble on DirectInput pads.
- **Settings**: the parser's tolerance (unknown keys and values keep the
  default, presets override 11-18 unless VQ_CUSTOM, the binding swap),
  and the writer's format and moments. Only at M5.
- **Steam**: only if Steam stays: the same calls and moments; without it,
  nothing.
- **Movies**: bit-exact VP8 (libvpx 1.4.0, deadline 0, no post-processing,
  one thread) and Vorbis 1.3.5, the I420 planes into three 8-bit textures
  with the shader above, the 30-frame prebuffer, one frame per update with
  a blocking take, the `+0x6d8` arithmetic, the PCM16 stream with its
  rounding and the volume rules (preload muted and paused; master volume
  only while playing with `g_WaitFramesByTime == 1`), the end and skip rules.

## Open questions

1. GameSetting.ini line ends: does `getline` leave `\r` in the values of a
   CRLF file (the user's is CRLF), so that every enum value fails and keeps
   its default? Check with `tools/frida/peek.py 0x022fb504 0x60 --deref`
   under the harness with a CRLF file.
2. Movies: does the update rate during a movie match 29.97 Hz, or does the
   video drift against the FMOD stream? Trace `updateTexture` `0x00a284a0`
   and the read callback in a movie replay. Also confirm libvpx's worker
   threads never start (`vp8mt_decoder_create` `0x0040c350` not reached), the
   plane format (`0x00ca2fcc`, assumed R8) and `video.fx`'s sampler filter.
3. Input: `ffxKeyBinding` slots 41-48 and 75-80 are outside the pad map
   (used elsewhere?); the roles of the PC bits 0x20000..0x10000000; the
   harness doc says Esc also gives 0x1, the map gives only 0x1000000.
4. `g_BtlForcedEncount2` (`0x0112ca20`) is written with "pad 0 R1 held" by
   `Virtuos_InputManager_updateInput`: check the name.
5. `g_AchievementRecords` is at `0x00c35794` in `ffx.sym` (the API name
   pointer), while the record table's id field starts at `0x00c35790`.
6. Achievements 2, 7, 8, 9, 0xc and 0x11 have no constant caller; they
   probably come through `Native_025b_UnlockAchievement`.
7. Auto-port names inside the movie code stay as they are because hooks use
   them: `ReturnFieldU32PointerChain_0042e060` (mkvparser
   `Block::GetTrackNumber`), `SetGlobals_0042e1d0` and
   `SeveralStoresCopyIntoGlobal_0042e200` (libogg's allocator set/reset).

## Appendix: import census

Generated by `cargo run --release -p ffx-platform --example census`
(2026-10-25). "via" names the import thunk a call goes through. Calls
through COM vtables (DirectInput, D3D11, Steam interfaces, FMOD through
`fmod_event`) are not imports and are in the sections above.

#### `USER32.dll`

| Import | Slot | Callers (function (sites)) |
|---|---|---|
| `PeekMessageW` | `0x00b0c64c` | `0x00642860` graphicProcessWindowsMessage (0x00642879) |
| `DispatchMessageW` | `0x00b0c650` | `0x00642860` graphicProcessWindowsMessage (0x00642891) |
| `GetWindowInfo` | `0x00b0c654` | `0x0062d6f0` Phyre_PFramework_PInputDeviceMouse_update (0x0062d7d9) |
| `ChangeDisplaySettingsA` | `0x00b0c658` | `0x00626cb0` Phyre_PFramework_PWindowWin32Base_createWindow (0x00626dc8); `0x006270a0` Phyre_PFramework_PWindowWin32Base_restoreDisplay (0x006270af); `0x0062a460` Phyre_PFramework_PWindowWin32Base_setFocus (0x0062a4e9, 0x0062a564); `0x0062bf00` Phyre_PFramework_PWindowWin32Base_applyScreenMode (0x0062bf7c, 0x0062c009, 0x0062c09f) |
| `LoadIconA` | `0x00b0c65c` | `0x00626cb0` Phyre_PFramework_PWindowWin32Base_createWindow (0x00626d1d); `0x0062bf00` Phyre_PFramework_PWindowWin32Base_applyScreenMode (0x0062c13c) |
| `LoadCursorA` | `0x00b0c660` | `0x00626cb0` Phyre_PFramework_PWindowWin32Base_createWindow (0x00626d35) |
| `CallNextHookEx` | `0x00b0c664` | `0x006250e0` PWindowWin32_MouseHookProc (0x0062511f) |
| `UnhookWindowsHookEx` | `0x00b0c668` | `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x006253be) |
| `ClientToScreen` | `0x00b0c66c` | `0x00680030` FreeCamera_handleInput (0x006801f1) |
| `GetClipCursor` | `0x00b0c670` | `0x0068ffb0` FFEscMenu_update (0x0069001c) |
| `wsprintfW` | `0x00b0c674` | `0x006efd10` SaveDataMachine_MergeFileFlags_006efd10 (0x006efd56); `0x006f0070` SaveDataMachine_ReadFile (0x006f00b2); `0x006f0330` SetUpDefaultSaveFolder (0x006f037a); `0x006f0510` SaveDataManager_debugSave_Internal_6f0650 (0x006f0557); `0x006f0860` SaveDataMachine_WriteFile (0x006f08cb); `0x006f0a70` SaveDataMachine_ScanFiles (0x006f0ac9); `0x006f0c60` SaveDataMachine_ReadHeaders (0x006f0cf9) |
| `SetCursorPos` | `0x00b0c678` | `0x00680030` FreeCamera_handleInput (0x0068022d) |
| `SendMessageW` | `0x00b0c67c` | `0x0042f0f0` FFXApplication_dtor_FFXApplication (0x0042f10e); `0x0042f150` FFXApplication_scalar_deleting_dtor (0x0042f171); `0x0042f4d0` ResetToLauncher (0x0042f4dc); `0x0042fa70` PostReset (0x0042fa86); `0x00af9ba0` FFXApplication_atexit_dtor (0x00af9bbf) |
| `GetClientRect` | `0x00b0c680` | `0x00595750` PRendererD3D11_initWindow_00595750 (0x005957b8); `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x006252be); `0x00626cb0` Phyre_PFramework_PWindowWin32Base_createWindow (0x00626de1); `0x0062a460` Phyre_PFramework_PWindowWin32Base_setFocus (0x0062a51a, 0x0062a595); `0x0062bf00` Phyre_PFramework_PWindowWin32Base_applyScreenMode (0x0062bfad, 0x0062c03a, 0x0062c0d0); `0x0062d6f0` Phyre_PFramework_PInputDeviceMouse_update (0x0062d7b1); `0x0062e3a0` PApplication_updateMouseCamera_0062e3a0 (0x0062e423); `0x00680030` FreeCamera_handleInput (0x006801c8) |
| `MessageBoxW` | `0x00b0c684` | `0x0061ba80` BigFileStream_fileTaskUncompress (0x0061bd86); `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x00625448); `0x0062ee50` Game_Main (0x0062ef55); `0x0066fb70` PhyreScene_textureVideoLoad (0x0067004e); `0x006793e0` PhyreFIOS_LoadMagicfilePrx (0x006794b2); `0x006795b0` Game_InitFileSystem (0x0067964a); `0x00679730` fiosOpen (0x006797cc); `0x006d8fc0` PhyFMVPlayerManager_fmvPlayer_play (0x006d90e6); `0x006da9b0` (0x006daaf4) |
| `ShowCursor` | `0x00b0c688` | `0x0061ba80` BigFileStream_fileTaskUncompress (0x0061bd76); `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x00625435); `0x0066fb70` PhyreScene_textureVideoLoad (0x00670038); `0x006793e0` PhyreFIOS_LoadMagicfilePrx (0x0067949c); `0x006795b0` Game_InitFileSystem (0x0067963a); `0x00679730` fiosOpen (0x006797bc); `0x00680030` FreeCamera_handleInput (0x00680152, 0x0068023e); `0x00680410` FreeCamera_setType (0x00680431); `0x0068fcf0` FFEscMenu_QuitConfirmed_0068fcf0 (0x0068fd23); `0x0068fe60` FFEscMenu_resume (0x0068fe82); `0x0068ffb0` FFEscMenu_update (0x0068ffcf, 0x00690384); `0x006d8fc0` PhyFMVPlayerManager_fmvPlayer_play (0x006d90d0); `0x006da9b0` (0x006daade) |
| `RegisterWindowMessageA` | `0x00b0c68c` | `0x00623d50` PWindowWin32_AttachToHost (0x00623d6a); `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x0062517f); `0x00629ab0` PApplication_InitWindowAndApp_00629ab0 (0x00629b52) |
| `TranslateMessage` | `0x00b0c690` | `0x0062ee50` Game_Main (0x0062f2d4); `0x00642860` graphicProcessWindowsMessage (0x00642887) |
| `DispatchMessageA` | `0x00b0c694` | `0x0062ee50` Game_Main (0x0062f2e1) |
| `SetWindowsHookExA` | `0x00b0c698` | `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x00625389) |
| `SetParent` | `0x00b0c69c` | `0x00623d50` PWindowWin32_AttachToHost (0x00623d5f); `0x00629ab0` PApplication_InitWindowAndApp_00629ab0 (0x00629b47) |
| `PeekMessageA` | `0x00b0c6a0` | `0x0062ee50` Game_Main (0x0062f265) |
| `SendMessageA` | `0x00b0c6a4` | `0x00623d50` PWindowWin32_AttachToHost (0x00623d77); `0x00629ab0` PApplication_InitWindowAndApp_00629ab0 (0x00629b5d) |
| `RegisterDeviceNotificationA` | `0x00b0c6a8` | `0x00626cb0` Phyre_PFramework_PWindowWin32Base_createWindow (0x00626faf) |
| `PostMessageA` | `0x00b0c6ac` | `0x006250e0` PWindowWin32_MouseHookProc (0x0062510a); `0x0062bf00` Phyre_PFramework_PWindowWin32Base_applyScreenMode (0x0062c155); `0x0062ee50` Game_Main (0x0062f293) |
| `DefWindowProcA` | `0x00b0c6b0` | `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x0062533c, 0x0062539d, 0x006253dc, 0x0062547b, 0x006254ae, 0x006254e6) |
| `PostQuitMessage` | `0x00b0c6b4` | `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x006252a9) |
| `RegisterClassA` | `0x00b0c6b8` | `0x00626cb0` Phyre_PFramework_PWindowWin32Base_createWindow (0x00626d66) |
| `SetWindowLongA` | `0x00b0c6bc` | `0x00626cb0` Phyre_PFramework_PWindowWin32Base_createWindow (0x00626f55); `0x0062a460` Phyre_PFramework_PWindowWin32Base_setFocus (0x0062a50a, 0x0062a585); `0x0062bf00` Phyre_PFramework_PWindowWin32Base_applyScreenMode (0x0062bf9d, 0x0062c02a, 0x0062c0c0) |
| `GetWindowLongA` | `0x00b0c6c0` | `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x006251aa); `0x0062a460` Phyre_PFramework_PWindowWin32Base_setFocus (0x0062a4f7, 0x0062a572); `0x0062bf00` Phyre_PFramework_PWindowWin32Base_applyScreenMode (0x0062bf8a, 0x0062c017, 0x0062c0ad) |
| `ClipCursor` | `0x00b0c6c4` | `0x0062d6f0` Phyre_PFramework_PInputDeviceMouse_update (0x0062d7ce); `0x00680030` FreeCamera_handleInput (0x00680159, 0x0068026f); `0x00680410` FreeCamera_setType (0x00680420); `0x0068ffb0` FFEscMenu_update (0x00690028, 0x0069007c) |
| `GetCursorPos` | `0x00b0c6c8` | `0x0062d6f0` Phyre_PFramework_PInputDeviceMouse_update (0x0062d785) |
| `AdjustWindowRect` | `0x00b0c6cc` | `0x00626cb0` Phyre_PFramework_PWindowWin32Base_createWindow (0x00626e2c); `0x0062a460` Phyre_PFramework_PWindowWin32Base_setFocus (0x0062a53f, 0x0062a5ba); `0x0062bf00` Phyre_PFramework_PWindowWin32Base_applyScreenMode (0x0062bfc9, 0x0062c05f, 0x0062c0f5) |
| `GetWindowRect` | `0x00b0c6d0` | `0x0062d6f0` Phyre_PFramework_PInputDeviceMouse_update (0x0062d79e) |
| `SetWindowTextA` | `0x00b0c6d4` | `0x0062cf60` Phyre_PFramework_PApplication_setWindowTitle (0x0062cf85) |
| `ReleaseDC` | `0x00b0c6d8` | `0x006270a0` Phyre_PFramework_PWindowWin32Base_restoreDisplay (0x006270be) |
| `GetSystemMetrics` | `0x00b0c6dc` | `0x0062d6f0` Phyre_PFramework_PInputDeviceMouse_update (0x0062d7e2, 0x0062d7f7, 0x0062d803); `0x00a25910` (0x00a25938); `0x00a25990` MouseState_createInstance_00a25990 (0x00a259cf) |
| `SetWindowPos` | `0x00b0c6e0` | `0x0062a460` Phyre_PFramework_PWindowWin32Base_setFocus (0x0062a5dc); `0x0062bf00` Phyre_PFramework_PWindowWin32Base_applyScreenMode (0x0062bfe9, 0x0062c081, 0x0062c117) |
| `ShowWindow` | `0x00b0c6e4` | `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x0062523b) |
| `DestroyWindow` | `0x00b0c6e8` | `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x0062524e) |
| `CreateWindowExA` | `0x00b0c6ec` | `0x00626cb0` Phyre_PFramework_PWindowWin32Base_createWindow (0x00626f35) |

#### `DINPUT8.dll`

| Import | Slot | Callers (function (sites)) |
|---|---|---|
| `DirectInput8Create` | `0x00b0c020` | `0x00624350` Input_InitDirectInput (0x0062436a via 0x0094a914); `0x006243a0` Input_InitDirectInput_copy_006243a0 (0x006243ba via 0x0094a914) |

#### `XINPUT9_1_0.dll`

| Import | Slot | Callers (function (sites)) |
|---|---|---|
| `XInputSetState` | `0x00b0c714` | `0x0062dba0` PadXInput_Update (0x0062de11 via 0x009da260) |
| `XInputGetState` | `0x00b0c718` | `0x0062dba0` PadXInput_Update (0x0062dbcb via 0x009da25a) |

#### `steam_api.dll`

| Import | Slot | Callers (function (sites)) |
|---|---|---|
| `SteamUserStats` | `0x00b0c898` | `0x006f11a0` StatsAndAchievements_ctor (0x006f12c9, 0x006f12ee); `0x006f1420` StatsAndAchievements_ResetAll (0x006f1420); `0x006f15f0` StatsAndAchievements_OnUserStatsReceived (0x006f1635, 0x006f164c, 0x006f1679); `0x006f17d0` StatsAndAchievements_RequestStats (0x006f17d9, 0x006f1800); `0x006f18b0` StatsAndAchievements_SetAchievementByName (0x006f18bf, 0x006f18cf); `0x006f1900` StatsAndAchievements_SetAchievement_006f1900 (0x006f1930, 0x006f193e) |
| `SteamUtils` | `0x00b0c89c` | `0x00623470` Steam_Init (0x006234b5); `0x006f11a0` StatsAndAchievements_ctor (0x006f128c) |
| `SteamAPI_UnregisterCallback` | `0x00b0c8a0` | `0x006f1310` CCallback_UserAchievementStored_dtor (0x006f131d); `0x006f1330` CCallback_UserStatsReceived_dtor (0x006f133d); `0x006f1350` CCallback_UserStatsStored_dtor (0x006f135d); `0x006f1370` StatsAndAchievements_dtor (0x006f1378); `0x006f13c0` StatsAndAchievements_scalar_deleting_dtor (0x006f13cb); `0x006f14a0` StatsAndAchievements_Destroy (0x006f14b3); `0x006f16e0` CCallback_UserAchievementStored_Register (0x006f16fd); `0x006f1730` CCallback_UserStatsReceived_Register (0x006f174d); `0x006f1780` CCallback_UserStatsStored_Register (0x006f179d); `0x006f1900` StatsAndAchievements_SetAchievement_006f1900 (0x006f1951); `0x006f1960` (0x006f1961); `0x006f1970` (0x006f1971) |
| `SteamUser` | `0x00b0c8a4` | `0x00623470` Steam_Init (0x006234c4); `0x006f11a0` StatsAndAchievements_ctor (0x006f12d3); `0x006f17d0` StatsAndAchievements_RequestStats (0x006f17e3, 0x006f17ed) |
| `SteamAPI_Init` | `0x00b0c8a8` | `0x00623470` Steam_Init (0x0062347f) |
| `SteamAPI_RestartAppIfNecessary` | `0x00b0c8ac` | `0x00623470` Steam_Init (0x00623472) |
| `SteamAPI_Shutdown` | `0x00b0c8b0` | `0x00623500` Steam_Shutdown (0x00623500); `0x0062ee50` Game_Main (0x0062f45d) |
| `SteamAPI_RegisterCallback` | `0x00b0c8b4` | `0x006f1060` CCallback_UserAchievementStored_ctor (0x006f1097); `0x006f10b0` CCallback_UserStatsReceived_ctor (0x006f10e7); `0x006f1100` CCallback_UserStatsStored_ctor (0x006f1137); `0x006f11a0` StatsAndAchievements_ctor (0x006f11ca); `0x006f16e0` CCallback_UserAchievementStored_Register (0x006f1712); `0x006f1730` CCallback_UserStatsReceived_Register (0x006f1762); `0x006f1780` CCallback_UserStatsStored_Register (0x006f17b2) |
| `SteamAPI_RunCallbacks` | `0x00b0c8b8` | `0x0042f520` FFXApplication_animate (0x0042f52c) |

#### `IMM32.dll`

| Import | Slot | Callers (function (sites)) |
|---|---|---|
| `ImmAssociateContext` | `0x00b0c028` | `0x00626cb0` Phyre_PFramework_PWindowWin32Base_createWindow (0x00626f47 via 0x009da254) |

#### `WINMM.dll`

| Import | Slot | Callers (function (sites)) |
|---|---|---|
| `timeBeginPeriod` | `0x00b0c704` | `0x0042fe10` Phyre_PTimerWin32_Initialize (0x0042fe18) |
| `timeEndPeriod` | `0x00b0c708` | `0x00430040` Phyre_PTimerWin32_Terminate (0x00430042) |
| `timeGetTime` | `0x00b0c70c` | `0x0042fe10` Phyre_PTimerWin32_Initialize (0x0042fe2f) |

#### `SHELL32.dll`

| Import | Slot | Callers (function (sites)) |
|---|---|---|
| `SHGetFolderPathW` | `0x00b0c628` | `0x0062ac90` CreateDocumentsFolders_0062ac90 (0x0062acb7); `0x0062ee50` Game_Main (0x0062ee9c); `0x006f0330` SetUpDefaultSaveFolder (0x006f0363) |
| `CommandLineToArgvW` | `0x00b0c62c` | `0x0062ee50` Game_Main (0x0062ef8b) |
| `ShellExecuteW` | `0x00b0c630` | `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x0062529f) |

#### `SHLWAPI.dll`

| Import | Slot | Callers (function (sites)) |
|---|---|---|
| `PathRemoveFileSpecW` | `0x00b0c638` | `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x0062526f) |
| `PathCombineW` | `0x00b0c63c` | `0x00625130` Phyre_PFramework_PWindowWin32Base_WndProc (0x00625288) |
| `PathRemoveFileSpecA` | `0x00b0c640` | `0x0042f6d0` LauncherPath_Init_0042f6d0 (0x0042f707) |
| `PathCombineA` | `0x00b0c644` | `0x0042f6d0` LauncherPath_Init_0042f6d0 (0x0042f71e) |

#### `VERSION.dll`

| Import | Slot | Callers (function (sites)) |
|---|---|---|
| `VerQueryValueW` | `0x00b0c6f4` | `0x00687c10` StackWalker_LoadModule (0x00687d37 via 0x009da272) |
| `GetFileVersionInfoA` | `0x00b0c6f8` | `0x00687c10` StackWalker_LoadModule (0x00687d0e via 0x009da26c) |
| `GetFileVersionInfoSizeA` | `0x00b0c6fc` | `0x00687c10` StackWalker_LoadModule (0x00687ce2 via 0x009da266) |

#### `d3d11.dll`

| Import | Slot | Callers (function (sites)) |
|---|---|---|
| `D3D11CreateDevice` | `0x00b0c720` | `0x00595800` PRendererD3D11_createDeviceAndSwapChain_00595800 (0x00595870 via 0x0096838e) |
| `D3D11CreateDeviceAndSwapChain` | `0x00b0c724` | `0x00595800` PRendererD3D11_createDeviceAndSwapChain_00595800 (0x00595940 via 0x00968394) |

#### Imports called from the movie and codec code

| DLL | Import | Callers (function, number of sites when more than one) |
|---|---|---|
| fmodex.dll | `?setPriority@Channel@FMOD@@QAG?AW4FMOD_RESULT@@H@Z` | `0x00a273b0` PVideoPlaybackWin32_AudioInit_00a273b0 |
| fmodex.dll | `?setPaused@Channel@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z` | `0x00a27e90` PVideoPlaybackWin32_togglePause (2 sites); `0x00a282f0` PVideoPlaybackWin32_play |
| fmodex.dll | `?stop@Channel@FMOD@@QAG?AW4FMOD_RESULT@@XZ` | `0x00a278f0` PVideoPlaybackWin32_AudioShutdown |
| fmodex.dll | `?getUserData@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAPAX@Z` | `0x00a27ee0` PVideoPlaybackWin32_FmodReadCallback_00a27ee0 |
| fmodex.dll | `?setUserData@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAX@Z` | `0x00a273b0` PVideoPlaybackWin32_AudioInit_00a273b0 |
| fmodex.dll | `?getOpenState@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAW4FMOD_OPENSTATE@@PAIPA_N2@Z` | `0x00a278f0` PVideoPlaybackWin32_AudioShutdown; `0x00a27c50` PVideoPlaybackWin32_isPlaying; `0x00a27db0` |
| fmodex.dll | `?createSound@System@FMOD@@QAG?AW4FMOD_RESULT@@PBDIPAUFMOD_CREATESOUNDEXINFO@@PAPAVSound@2@@Z` | `0x00a273b0` PVideoPlaybackWin32_AudioInit_00a273b0 |
| fmodex.dll | `?playSound@System@FMOD@@QAG?AW4FMOD_RESULT@@W4FMOD_CHANNELINDEX@@PAVSound@2@_NPAPAVChannel@2@@Z` | `0x00a273b0` PVideoPlaybackWin32_AudioInit_00a273b0 |
| fmodex.dll | `?release@Sound@FMOD@@QAG?AW4FMOD_RESULT@@XZ` | `0x00a278f0` PVideoPlaybackWin32_AudioShutdown |
| fmodex.dll | `?setVolume@Channel@FMOD@@QAG?AW4FMOD_RESULT@@M@Z` | `0x00a28330` PVideoPlaybackWin32_setVolume |
| KERNEL32.dll | `InterlockedIncrement` | `0x0040db70` vpx_once_0040db70; `0x0041b0b0` vpx_once_0041b0b0 |
| KERNEL32.dll | `InterlockedDecrement` | `0x0040db70` vpx_once_0040db70; `0x0041b0b0` vpx_once_0041b0b0 |
| KERNEL32.dll | `GetModuleHandleW` | `0x0041b2b0` get_cpu_count_0041b2b0; `0x0041b320` |
| KERNEL32.dll | `GetTickCount` | `0x00a27f20` PVideoPlaybackWin32_open |
| KERNEL32.dll | `GetSystemInfo` | `0x0041b2b0` get_cpu_count_0041b2b0; `0x0041b320` |
| KERNEL32.dll | `CloseHandle` | `0x0040ce60` |
| KERNEL32.dll | `ReleaseSemaphore` | `0x0040b5b0`; `0x0040ce60`; `0x0040cfa0` |
| KERNEL32.dll | `WaitForSingleObject` | `0x0040c2c0`; `0x0040ce60`; `0x0040cfa0` |
| KERNEL32.dll | `Sleep` | `0x0040b5b0`; `0x00a278f0` PVideoPlaybackWin32_AudioShutdown |
| KERNEL32.dll | `GetProcAddress` | `0x0041b2b0` get_cpu_count_0041b2b0; `0x0041b320` |
| KERNEL32.dll | `InterlockedCompareExchange` | `0x0040db70` vpx_once_0040db70; `0x0041b0b0` vpx_once_0041b0b0; `0x006d73f0`; `0x006dae00` PhyFMVPlayerManager_updateVideoCallback (2 sites) |
| KERNEL32.dll | `InitializeCriticalSection` | `0x0040db70` vpx_once_0040db70; `0x0041b0b0` vpx_once_0041b0b0 |
| KERNEL32.dll | `EnterCriticalSection` | `0x0040db70` vpx_once_0040db70; `0x0041b0b0` vpx_once_0041b0b0 |
| KERNEL32.dll | `LeaveCriticalSection` | `0x0040db70` vpx_once_0040db70; `0x0041b0b0` vpx_once_0041b0b0 |
| KERNEL32.dll | `DeleteCriticalSection` | `0x0040db70` vpx_once_0040db70 (2 sites); `0x0041b0b0` vpx_once_0041b0b0 (2 sites) |
| KERNEL32.dll | `CreateSemaphoreW` | `0x0040c350` vp8mt_decoder_create (2 sites) |
| USER32.dll | `MessageBoxW` | `0x006d8fc0` PhyFMVPlayerManager_fmvPlayer_play; `0x006da9b0` |
| USER32.dll | `ShowCursor` | `0x006d8fc0` PhyFMVPlayerManager_fmvPlayer_play; `0x006da9b0` |
| MSVCR110.dll | `longjmp` | `0x00409460` vpx_internal_error |
| MSVCR110.dll | `_libm_sse2_sqrt_precise` | `0x00412440`; `0x004124b0`; `0x00a95300` (4 sites); `0x00aa4b20` (2 sites); `0x00aa4fa0` |
| MSVCR110.dll | `_libm_sse2_sin_precise` | `0x00a98070`; `0x00a999e0`; `0x00a9af90`; `0x00a9b840` (2 sites); `0x00a9d680` (3 sites) |
| MSVCR110.dll | `memmove` | `0x0042e5f0`; `0x00a911a0`; `0x00a9bda0` |
| MSVCR110.dll | `__iob_func` | `0x006d71b0` PhyFMVPlayerManager_DestroyFMVPlayer; `0x00a27170` PVideoPlaybackWin32_AudioRead_00a27170; `0x00a27650` PVideoPlaybackWin32_DemuxLoop |
| MSVCR110.dll | `memcpy` | `0x0040f930` (3 sites); `0x0040f9f0` (3 sites); `0x004116e0` (2 sites); `0x00411870` (3 sites); `0x004119c0`; `0x00415120` (3 sites); `0x00419780` (2 sites); `0x00427950`; `0x00a26fb0` PVideoPlaybackWin32_memcpy_00a26fb0; `0x00a26fe0` PVideoPlaybackWin32_CopyPlane_00a26fe0 (2 sites); `0x00a279b0` PVideoPlaybackWin32_DecodeLoop (6 sites); `0x00a8ee40` (3 sites); `0x00a8f1b0`; `0x00a911a0`; `0x00a94a90` |
| MSVCR110.dll | `strncpy` | `0x006d82f0` PhyFMVPlayerManager_createVideoIndexMap (5 sites); `0x006d9230`; `0x006d98e0` PhyFMVPlayerManager_initSubtitleTimestamps (4 sites) |
| MSVCR110.dll | `free` | `0x0040db70` vpx_once_0040db70 (2 sites); `0x0041b0b0` vpx_once_0041b0b0 (2 sites); `0x0041e5d0` mkvparser_ResetAllocator; `0x006d8fc0` PhyFMVPlayerManager_fmvPlayer_play; `0x006da9b0` |
| MSVCR110.dll | `strstr` | `0x006d82f0` PhyFMVPlayerManager_createVideoIndexMap (2 sites); `0x006d98e0` PhyFMVPlayerManager_initSubtitleTimestamps |
| MSVCR110.dll | `toupper` | `0x00a900a0` (2 sites); `0x00a90190` (2 sites); `0x00a90290` (2 sites) |
| MSVCR110.dll | `printf` | `0x00a27650` PVideoPlaybackWin32_DemuxLoop (2 sites) |
| MSVCR110.dll | `memset` | `0x0040a680`; `0x0040c350` vp8mt_decoder_create; `0x0040cfa0` (6 sites); `0x0040dc40` (4 sites); `0x0040dda0` (5 sites); `0x0040fab0` (6 sites); `0x0040fbd0` (3 sites); `0x00410830` (2 sites); `0x004116e0` (2 sites); `0x00412340`; `0x00412e00`; `0x00414830` (3 sites); `0x004150d0` (3 sites); `0x00415120` (2 sites); `0x0041c950`; `0x0041caf0`; `0x006d82f0` PhyFMVPlayerManager_createVideoIndexMap; `0x006d98e0` PhyFMVPlayerManager_initSubtitleTimestamps; `0x00a26fd0` PVideoPlaybackWin32_memset_00a26fd0; `0x00a273b0` PVideoPlaybackWin32_AudioInit_00a273b0; `0x00a8fac0` (4 sites); `0x00a8fd80` vorbis_info_clear (2 sites); `0x00a8ffd0` vorbis_info_init; `0x00a914b0`; `0x00a917a0` vorbis_dsp_clear (2 sites); `0x00a91a70`; `0x00a91b30` (6 sites); `0x00a92040` vorbis_block_clear; `0x00a921b0` vorbis_block_init (2 sites); `0x00a926f0`; `0x00a92a60`; `0x00a92d50` (2 sites); `0x00a93410`; `0x00a935b0`; `0x00a94a90`; `0x00a96840`; `0x00a972e0`; `0x00a97330`; `0x00a974b0`; `0x00a97db0` (2 sites); `0x00a9b190` (2 sites); `0x00a9b7a0`; `0x00a9b840` (3 sites); `0x00a9def0`; `0x00a9e050`; `0x00a9e060`; `0x00a9eb80`; `0x00a9edf0`; `0x00a9f3c0` (2 sites); `0x00a9f600`; `0x00a9f7e0`; `0x00a9fea0` (3 sites); `0x00aa00f0` (2 sites); `0x00aa03d0`; `0x00aa0450`; `0x00aa0480`; `0x00aa0790`; `0x00aa15f0` (3 sites); `0x00aa19a0`; `0x00aa19d0`; `0x00aa23b0`; `0x00aa2700`; `0x00aa2a00` (2 sites); `0x00aa2f40`; `0x00aa2f70`; `0x00aa3640`; `0x00aa3c80`; `0x00aa4090`; `0x00aa4200` |
| MSVCR110.dll | `fprintf` | `0x006d71b0` PhyFMVPlayerManager_DestroyFMVPlayer; `0x00a27170` PVideoPlaybackWin32_AudioRead_00a27170 |
| MSVCR110.dll | `sprintf_s` | `0x006d82f0` PhyFMVPlayerManager_createVideoIndexMap (2 sites); `0x006d93e0` PhyFMVPlayerManager_getVideoName; `0x006d98e0` PhyFMVPlayerManager_initSubtitleTimestamps; `0x006da550` PhyFMVPlayerManager_readCreditsText |
| MSVCR110.dll | `qsort` | `0x00a92d50`; `0x00a95300`; `0x00aa2700`; `0x00aa2a00`; `0x00aa4230` |
| MSVCR110.dll | `fflush` | `0x00a27650` PVideoPlaybackWin32_DemuxLoop |
| MSVCR110.dll | `strchr` | `0x006d82f0` PhyFMVPlayerManager_createVideoIndexMap; `0x006d98e0` PhyFMVPlayerManager_initSubtitleTimestamps |
| MSVCR110.dll | `floor` | `0x00a27170` PVideoPlaybackWin32_AudioRead_00a27170; `0x00a27bb0`; `0x00a929c0`; `0x00a92c70` (2 sites); `0x00a93410` (2 sites); `0x00a943e0`; `0x00a95300` (4 sites); `0x00a96840` (2 sites); `0x00a974b0` (2 sites); `0x00a9d680`; `0x00a9d9d0` (3 sites); `0x00a9def0` (3 sites); `0x00aa20f0` (2 sites); `0x00aa3eb0` |
| MSVCR110.dll | `sprintf` | `0x006d98e0` PhyFMVPlayerManager_initSubtitleTimestamps; `0x006d9c80` PhyFMVPlayerManager_initialize; `0x006da550` PhyFMVPlayerManager_readCreditsText |
| MSVCR110.dll | `atoi` | `0x006d82f0` PhyFMVPlayerManager_createVideoIndexMap; `0x006d98e0` PhyFMVPlayerManager_initSubtitleTimestamps |
| MSVCR110.dll | `rand` | `0x00411e80`; `0x004124b0`; `0x00412690`; `0x00a7f0e0`; `0x00a8aec0` |
| MSVCR110.dll | `_libm_sse2_pow_precise` | `0x00a929c0`; `0x00a943e0` |
| MSVCR110.dll | `malloc` | `0x0040db70` vpx_once_0040db70; `0x0041b0b0` vpx_once_0041b0b0; `0x0041e5d0` mkvparser_ResetAllocator |
| MSVCR110.dll | `ceil` | `0x00a96840` |
| MSVCR110.dll | `exit` | `0x00aa2fa0` |
| MSVCR110.dll | `_libm_sse2_log_precise` | `0x00a92c70` (2 sites); `0x00a96840` (2 sites); `0x00a974b0` (6 sites); `0x00a9d680` (2 sites) |
| MSVCR110.dll | `_setjmp3` | `0x00409c70`; `0x0040a680`; `0x0040ab90` |
| MSVCR110.dll | `realloc` | `0x0042e200` SeveralStoresCopyIntoGlobal_0042e200 |
| MSVCR110.dll | `getenv` | `0x0040d190` (2 sites); `0x0040d2c0` vpx_setup_rtcd_0040d2c0; `0x0041aeb0` (2 sites); `0x0041afe0` (2 sites); `0x0041b180` (2 sites); `0x0041b320` (2 sites) |
| MSVCR110.dll | `_beginthreadex` | `0x0040c350` vp8mt_decoder_create |
| MSVCR110.dll | `strtol` | `0x0040d190` (2 sites); `0x0040d2c0` vpx_setup_rtcd_0040d2c0 (2 sites); `0x0041aeb0` (2 sites); `0x0041afe0` (2 sites); `0x0041b180` (2 sites); `0x0041b320` (2 sites) |
| MSVCR110.dll | `_wassert` | `0x0041e710` (2 sites); `0x0041e850` (2 sites); `0x0041e980`; `0x0041eab0` (2 sites); `0x0041ec10`; `0x0041eea0`; `0x0041f120`; `0x0041f480` (4 sites); `0x0041fa80`; `0x0041fd50` mkvparser_Segment_CreateInstance; `0x004200d0` (4 sites); `0x00420730` (13 sites); `0x00420f70`; `0x00421060` (2 sites); `0x00421270` (2 sites); `0x00421450` mkvparser_Segment_Load; `0x004215d0` (5 sites); `0x00421970` (4 sites); `0x00421fe0`; `0x004220c0`; `0x004222b0`; `0x00422390`; `0x00422660`; `0x00422880` (2 sites); `0x004228e0` (2 sites); `0x00422940`; `0x00422ab0` (3 sites); `0x00422cb0`; `0x00422eb0`; `0x00422f40` (2 sites); `0x00423430`; `0x004236a0`; `0x00423710`; `0x00423840` mkvparser_Segment_GetFirst; `0x00423880`; `0x004238d0` mkvparser_Segment_GetNext; `0x00424050` (2 sites); `0x00424490` (16 sites); `0x00424fa0`; `0x00425200` mkvparser_Segment_GetDuration (2 sites); `0x00425380` (2 sites); `0x00425820` (2 sites); `0x00425e10` (2 sites); `0x00426310` (2 sites); `0x004264f0` (3 sites); `0x00426740`; `0x00426a80`; `0x00426ad0`; `0x00426b00`; `0x00426b50`; `0x00426b80` (3 sites); `0x00426cc0` (4 sites); `0x00427090` (3 sites); `0x00427270` (3 sites); `0x00427be0` (2 sites); `0x00427d40` (2 sites); `0x00427f00`; `0x00427f90`; `0x004282d0` (2 sites); `0x00428350` (4 sites); `0x00428700` (5 sites); `0x00428a00`; `0x00428e90` (5 sites); `0x004291f0` (7 sites); `0x004294c0` mkvparser_Tracks_GetTracksCount; `0x004294f0` (3 sites); `0x00429e10` (3 sites); `0x0042a550` (8 sites); `0x0042aca0` (2 sites); `0x0042af10` (3 sites); `0x0042b500`; `0x0042b620`; `0x0042b7c0`; `0x0042b850`; `0x0042b8a0` (5 sites); `0x0042bfd0` (2 sites); `0x0042c070`; `0x0042c0d0`; `0x0042c130`; `0x0042c300`; `0x0042c740`; `0x0042c880` mkvparser_Cluster_GetFirst (2 sites); `0x0042c920` (2 sites); `0x0042c9d0` mkvparser_Cluster_GetNext; `0x0042cb40`; `0x0042cd00`; `0x0042d470` (13 sites); `0x0042def0`; `0x0042df70` (2 sites); `0x0042e0d0` libwebm_mkvparser_Block_GetFrame; `0x0042e160` (2 sites) |
| MSVCR110.dll | `_vsnprintf` | `0x00409460` vpx_internal_error |
| MSVCR110.dll | `ldexp` | `0x00a926f0` (2 sites); `0x00a92c10`; `0x00a92c70`; `0x00a93410` (2 sites) |
| MSVCR110.dll | `_libm_sse2_acos_precise` | `0x00aa4230` (2 sites) |
| MSVCR110.dll | `_libm_sse2_atan_precise` | `0x00a974b0` (6 sites); `0x00aa3eb0` (4 sites) |
| MSVCR110.dll | `_libm_sse2_cos_precise` | `0x00a98070`; `0x00a999e0`; `0x00a9af90`; `0x00a9d680` (3 sites); `0x00aa4fa0` (2 sites) |
| MSVCR110.dll | `_libm_sse2_exp_precise` | `0x00412440`; `0x004124b0`; `0x00a96840` (6 sites); `0x00a974b0`; `0x00aa4fa0` |
| MSVCP110.dll | `?_Xout_of_range@std@@YAXPBD@Z` | `0x006d8a00` std_Tree_erase_006d8a00 |
| MSVCP110.dll | `?_Xlength_error@std@@YAXPBD@Z` | `0x006d7ad0`; `0x006d98e0` PhyFMVPlayerManager_initSubtitleTimestamps; `0x006da500` |
| MSVCP110.dll | `?_Xbad_alloc@std@@YAXXZ` | `0x006d78f0`; `0x006d7930`; `0x006d7e20`; `0x006d7e60`; `0x006d7ea0`; `0x006d7ee0` |
