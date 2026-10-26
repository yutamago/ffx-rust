//! Continuation round 7 of list `h4-glue-other`, batch 2: the movie
//! player's play and render, the PS2 layer's draw environment and split
//! image load, `IggyMenu`'s constructor, the render thread's command
//! queue start and an escape-menu dialog callback.

use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;
use crate::h4_i1::{EXEC_LOAD_IMAGE, FLUSH_CACHE, MEMSET, SET_DEF_LOAD_IMAGE, SYNC_PATH};

// ---- PhyFMVPlayerManager_fmvPlayer_play

/// `g_FMVPlayerManager` (`0x00cded3c`).
pub const FMV_MANAGER: u32 = 0x00cd_ed3c;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// `"sFMVPlayerManager singleton is not created yet."` (`0x00b49b98`).
pub const NOT_CREATED: u32 = 0x00b4_9b98;
/// `PVideoPlaybackWin32_open(player; path, b)` (thiscall, ret 8).
pub const VIDEO_OPEN: u32 = 0x00a2_7f20;
/// `PVideoPlaybackWin32_setVolume(player; volume)` (thiscall, ret 4):
/// `al` set when done.
pub const SET_VOLUME: u32 = 0x00a2_8330;
/// ATL's default string manager (`0x00c64bf0`; its vtable's `+0xc` is
/// `GetNilString`, the data's manager's `+4` is `Free`).
pub const STRING_MANAGER: u32 = 0x00c6_4bf0;
/// `FUN_0064f970(id, 0)` (cdecl): the resource module holding string
/// `id`, or 0.
pub const FIND_RESOURCE: u32 = 0x0064_f970;
/// `FUN_00652310(str; module, id)` (thiscall, ret 8): loads string `id`
/// (an ATL `CStringW::LoadString` shape).
pub const LOAD_STRING: u32 = 0x0065_2310;
/// `FUN_00651800(conv; text, code page)` (thiscall, ret 8): converts to
/// UTF-16 (an ATL `CA2W` shape); `[conv]` starts as its inline buffer
/// (`conv + 4`, 0x100 bytes).
pub const A2W: u32 = 0x0065_1800;
/// `FUN_00654070(str; old, new)` (thiscall, ret 8): replaces `old` by
/// `new` (an ATL `CStringW::Replace` shape).
pub const STRING_REPLACE: u32 = 0x0065_4070;
/// `free`'s import slot (cdecl).
pub const FREE_IAT: u32 = 0x00b0_c370;
/// `ShowCursor`'s import slot (stdcall, 1 word).
pub const SHOW_CURSOR_IAT: u32 = 0x00b0_c688;
/// `MessageBoxW`'s import slot (stdcall, 4 words).
pub const MESSAGE_BOX_IAT: u32 = 0x00b0_c684;
/// `ResetToLauncher()` (cdecl).
pub const RESET_TO_LAUNCHER: u32 = 0x0042_f4d0;
/// `InterlockedExchangeAdd`'s import slot (stdcall, 2 words): the
/// original's `lock xadd`.
pub const INTERLOCKED_ADD_IAT: u32 = 0x00b0_c128;
/// `L"$name$"` (`0x00b42498`).
pub const NAME_TOKEN: u32 = 0x00b4_2498;
/// `L"error"` (`0x00b3f9fc`).
pub const ERROR_TITLE: u32 = 0x00b3_f9fc;
/// The movie error message's string id.
pub const MOVIE_ERROR_ID: u32 = 0x7833;
/// Bytes of [`fmv_play`]'s frame, from `ebp - 0x118`: the message
/// string (`+0`), the conversion's pointer (`+4`) and inline buffer
/// (`+8`, 0x100 bytes).
pub const PLAY_FRAME: u32 = 0x108;

/// The manager, after logging when it is not created yet.
fn manager(h: &mut impl Host) -> u32 {
    let at = h.rebase(FMV_MANAGER);
    if h.read::<u32>(at) == 0 {
        let msg = h.rebase(NOT_CREATED);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, msg]);
    }
    h.read(at)
}

/// `PhyFMVPlayerManager_fmvPlayer_play(path, b, wait)` (`0x006d8fc0`,
/// stdcall, ret 0xc; `frame` see [`PLAY_FRAME`]; its only caller,
/// `PhyFMVPlayerManager_PlayVideo`, does not read `eax`; the SEH frame
/// is not kept): the manager's `+0x6d8` and `+0x6e0` cleared and the
/// movie opened (`+0x38`). An open failing with -3 shows the error: the
/// message (string 0x7833, with `$name$` replaced by the path in UTF-16)
/// in a message box, then `ResetToLauncher`, and the string released.
/// With `wait` (a byte) the volume is set to 0 until that succeeds, at
/// most 10 times. Then `+0x6d0` set.
pub fn fmv_play(h: &mut impl Host, frame: u32, path: u32, b: u32, wait: u32) {
    let m = manager(h);
    h.write(m.wrapping_add(0x6d8), 0u32);
    h.write(m.wrapping_add(0x6e0), 0u32);
    let r = h.call_sym(VIDEO_OPEN, Conv::Thiscall(m.wrapping_add(0x38)), &[path, b]);
    if r == 0xffff_fffd {
        open_error(h, frame, path);
    }
    if wait as u8 != 0 {
        for _ in 0..10 {
            let m = manager(h);
            let done = h.call_sym(SET_VOLUME, Conv::Thiscall(m.wrapping_add(0x38)), &[0]);
            if done as u8 != 0 {
                break;
            }
        }
    }
    h.write(m.wrapping_add(0x6d0), 1u8);
}

/// The open error's message box (see [`fmv_play`]).
fn open_error(h: &mut impl Host, frame: u32, path: u32) {
    let string_at = frame;
    let conv_at = frame.wrapping_add(4);
    let inline = frame.wrapping_add(8);
    let strings = h.rebase(STRING_MANAGER);
    let nil = h.call_virtual(strings, 3, &[]);
    h.write(string_at, nil.wrapping_add(0x10));
    let module = h.call_sym(FIND_RESOURCE, Conv::Cdecl, &[MOVIE_ERROR_ID, 0]);
    if module != 0 {
        h.call_sym(
            LOAD_STRING,
            Conv::Thiscall(string_at),
            &[module, MOVIE_ERROR_ID],
        );
    }
    h.write(conv_at, inline);
    h.call_sym(A2W, Conv::Thiscall(conv_at), &[path, 0xfde9]);
    let converted: u32 = h.read(conv_at);
    let token = h.rebase(NAME_TOKEN);
    h.call_sym(
        STRING_REPLACE,
        Conv::Thiscall(string_at),
        &[token, converted],
    );
    let converted: u32 = h.read(conv_at);
    if converted != inline {
        let free: u32 = h.read(h.rebase(FREE_IAT));
        h.call(free, Conv::Cdecl, &[converted]);
    }
    let show: u32 = h.read(h.rebase(SHOW_CURSOR_IAT));
    h.call(show, Conv::Stdcall, &[1]);
    let text: u32 = h.read(string_at);
    let box_: u32 = h.read(h.rebase(MESSAGE_BOX_IAT));
    let title = h.rebase(ERROR_TITLE);
    h.call(box_, Conv::Stdcall, &[0, text, title, 0x10]);
    h.call_sym(RESET_TO_LAUNCHER, Conv::Cdecl, &[]);
    let data = text.wrapping_sub(0x10);
    let add: u32 = h.read(h.rebase(INTERLOCKED_ADD_IAT));
    let old = h.call(add, Conv::Stdcall, &[data.wrapping_add(0xc), u32::MAX]);
    if old.wrapping_sub(1) as i32 <= 0 {
        let owner: u32 = h.read(data);
        h.call_virtual(owner, 1, &[data]);
    }
}

// ---- PhyFMVPlayerManager_Render

/// `PVideoPlaybackWin32_isPlaying(player)` (thiscall): `al`.
pub const IS_PLAYING: u32 = 0x00a2_7c50;
/// Whether movies are suppressed (`0x00cded31`, a byte).
pub const MOVIES_OFF: u32 = 0x00cd_ed31;
/// `Phyre_PCamera_updateViewMatrices(camera)` (thiscall).
pub const UPDATE_VIEW: u32 = 0x0067_20c0;
/// `Phyre_PRendering_PRendererBase_setCamera(renderer; camera)`
/// (thiscall, ret 4).
pub const SET_CAMERA: u32 = 0x005b_abd0;
/// `Phyre_PRendering_PRendererBase_setClearColor(renderer; r, g, b, a)`
/// (thiscall, ret 0x10).
pub const SET_CLEAR_COLOR: u32 = 0x005b_ada0;
/// `Phyre_PRendering_PRenderer_beginScene(renderer; flags)` (thiscall,
/// ret 4).
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
/// `Phyre_PRendering_PRendererBase_renderMeshInstance(renderer; instance,
/// materials, 0, pass)` (thiscall, ret 0x10).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
/// `Phyre_PRendering_PRenderer_endScene(renderer; flag)` (thiscall, ret
/// 4).
pub const END_SCENE: u32 = 0x005b_6950;
/// The movie's render pass (`0x00cded44`).
pub const MOVIE_PASS: u32 = 0x00cd_ed44;
/// `g_PhyreScene` (`0x00ccb9d8`); the skip prompt at `+0x10de0`.
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// `FUN_006e8540(prompt)` (thiscall): `al`, whether the skip prompt is
/// still showing.
pub const PROMPT_VISIBLE: u32 = 0x006e_8540;

/// `PhyFMVPlayerManager_Render(this; renderer)` (`0x006d7570`, thiscall,
/// ret 4): 9 unless the manager is on (`+0x6d2`), its movie playing
/// (`+0x38`), it has a frame (`+0x6e4`), movies are not suppressed and
/// it has its quad (`+0x388`). Then `+0x6cc` kept at `+0x6c4`
/// (`fld`/`fstp`), the camera (`+0x574`) updated (its virtual `+4` and
/// `updateViewMatrices` at `+0x578`) and set, the clear colour black,
/// the scene begun (3), the movie pass set (`renderer + 0x48`), the quad
/// drawn and the scene ended: the first non-zero result is returned. On
/// success the skip prompt's flag `+0x74c` follows its visibility,
/// `+0x6e5` is set and 0 returned.
pub fn fmv_render(h: &mut impl Host, this: u32, renderer: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    if h.read::<u8>(at(0x6d2)) == 0
        || h.call_sym(IS_PLAYING, Conv::Thiscall(at(0x38)), &[]) as u8 == 0
        || h.read::<u8>(at(0x6e4)) == 0
        || h.read::<u8>(h.rebase(MOVIES_OFF)) != 0
        || h.read::<u32>(at(0x388)) == 0
    {
        return 9;
    }
    let v = pass(h.read(at(0x6cc)));
    h.write(at(0x6c4), v);
    h.call_virtual(at(0x574), 1, &[]);
    h.call_sym(UPDATE_VIEW, Conv::Thiscall(at(0x578)), &[]);
    let camera = if at(0x574) == 0 { 0 } else { at(0x578) };
    h.call_sym(SET_CAMERA, Conv::Thiscall(renderer), &[camera]);
    let one = 1.0f32.to_bits();
    let r = h.call_sym(SET_CLEAR_COLOR, Conv::Thiscall(renderer), &[0, 0, 0, one]);
    if r != 0 {
        return r;
    }
    let r = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[3]);
    if r != 0 {
        return r;
    }
    let render_pass: u32 = h.read(renderer.wrapping_add(0x13c));
    let pass_at = h.rebase(MOVIE_PASS);
    h.write(renderer.wrapping_add(0x48), pass_at);
    let quad: u32 = h.read(at(0x388));
    let materials: u32 = h.read(quad.wrapping_add(0x10));
    let r = h.call_sym(
        RENDER_MESH_INSTANCE,
        Conv::Thiscall(renderer),
        &[quad, materials, 0, render_pass],
    );
    if r != 0 {
        return r;
    }
    let r = h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
    if r != 0 {
        return r;
    }
    if h.read::<u8>(at(0x74c)) != 0 {
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        let prompt: u32 = h.read(scene.wrapping_add(0x1_0de0));
        if h.call_sym(PROMPT_VISIBLE, Conv::Thiscall(prompt), &[]) as u8 == 0 {
            h.write(at(0x74c), 0u8);
        }
    }
    h.write(at(0x6e5), 1u8);
    0
}

// ---- Sg_DrawSetEnvironment

/// `Sg_GetSpadPtr()` (cdecl).
pub const GET_SPAD: u32 = 0x0084_3220;
/// `MsGetScreenData(&x, &y, &w, &h, flags)` (cdecl): `al`.
pub const SCREEN_DATA: u32 = 0x007b_bd00;
/// `Sg_GsSetPacket(p)` (cdecl).
pub const SET_PACKET: u32 = 0x0084_1cf0;
/// `Sg_GsStartVIF()` (cdecl).
pub const START_VIF: u32 = 0x0084_1d50;
/// `Sg_DrawSetFrame()` (cdecl).
pub const SET_FRAME: u32 = 0x0082_cf80;
/// `Sg_GsXYOFFSET2(x, y)` (cdecl).
pub const XYOFFSET2: u32 = 0x0084_2070;
/// `Sg_GsSCISSOR2(x0, x1, y0, y1)` (cdecl).
pub const SCISSOR2: u32 = 0x0084_1c40;
/// `Sg_GsTEST2(...)` (cdecl, 8 words).
pub const TEST2: u32 = 0x0084_1e30;
/// `Sg_GsPRIM(prim)` (cdecl).
pub const PRIM: u32 = 0x0084_1a60;
/// `Sg_GsXYZF2(x, y)` (cdecl).
pub const XYZF2: u32 = 0x0084_2140;
/// `Sg_GsXYOFFSET(x, y)` (cdecl).
pub const XYOFFSET: u32 = 0x0084_2000;
/// `Sg_GsSCISSOR(x0, x1, y0, y1)` (cdecl).
pub const SCISSOR: u32 = 0x0084_1b90;
/// `Sg_GsEnd2()` (cdecl).
pub const GS_END2: u32 = 0x0084_1800;
/// `SendSpadToMFIFO(p)` (cdecl).
pub const SEND_SPAD: u32 = 0x0084_3200;
/// The screen mode `MsGetScreenData` returns (`0x01300775`, a byte).
pub const SCREEN_MODE: u32 = 0x0130_0775;
/// The mode the last environment was set for (`0x01300770`, a byte).
pub const LAST_SCREEN_MODE: u32 = 0x0130_0770;
/// `MsGetScreenData`'s flags output (`0x0130077c`).
pub const SCREEN_FLAGS: u32 = 0x0130_077c;
/// Bytes of [`draw_set_environment`]'s frame, from `ebp - 0x10`: the
/// width (`+0`), height (`+4`), x (`+8`), y (`+0xc`).
pub const ENV_FRAME: u32 = 0x10;

/// `Sg_DrawSetEnvironment()` (`0x0082c7b0`, cdecl; `frame` see
/// [`ENV_FRAME`]): the screen rectangle (`MsGetScreenData`; its mode
/// kept at `0x01300775`), width and height one larger, a GS packet on
/// the scratchpad; when the mode changed since the last call, the whole
/// 512x417 area is cleared with a sprite first. Then the context 2 and 1
/// offsets and scissors set to the rectangle, and the packet sent.
/// Returns `SendSpadToMFIFO`'s `eax`.
pub fn draw_set_environment(h: &mut impl Host, frame: u32) -> u32 {
    let (w_at, h_at, x_at, y_at) = (
        frame,
        frame.wrapping_add(4),
        frame.wrapping_add(8),
        frame.wrapping_add(0xc),
    );
    let spad = h.call_sym(GET_SPAD, Conv::Cdecl, &[]);
    let flags = h.rebase(SCREEN_FLAGS);
    let mode = h.call_sym(SCREEN_DATA, Conv::Cdecl, &[x_at, y_at, w_at, h_at, flags]);
    let w: u32 = h.read(w_at);
    h.write(w_at, w.wrapping_add(1));
    let hh: u32 = h.read(h_at);
    h.write(h_at, hh.wrapping_add(1));
    let mode_at = h.rebase(SCREEN_MODE);
    h.write(mode_at, mode as u8);
    h.call_sym(SET_PACKET, Conv::Cdecl, &[spad]);
    h.call_sym(START_VIF, Conv::Cdecl, &[]);
    let mode: u8 = h.read(mode_at);
    h.call_sym(SET_FRAME, Conv::Cdecl, &[]);
    let last_at = h.rebase(LAST_SCREEN_MODE);
    if mode != h.read::<u8>(last_at) {
        h.call_sym(XYOFFSET2, Conv::Cdecl, &[0, 0]);
        h.call_sym(SCISSOR2, Conv::Cdecl, &[0, 0x200, 0, 0x1a1]);
        h.call_sym(TEST2, Conv::Cdecl, &[1, 0, 0, 2, 0, 0, 1, 1]);
        h.call_sym(PRIM, Conv::Cdecl, &[0x206]);
        let (x, y) = (word(h, x_at), word(h, y_at));
        h.call_sym(XYZF2, Conv::Cdecl, &[x, y]);
        let (x, y, w, hh) = (word(h, x_at), word(h, y_at), word(h, w_at), word(h, h_at));
        h.call_sym(XYZF2, Conv::Cdecl, &[x.wrapping_add(w), y.wrapping_add(hh)]);
    }
    h.write(last_at, mode);
    let (x, y) = (word(h, x_at), word(h, y_at));
    h.call_sym(XYOFFSET2, Conv::Cdecl, &[x, y]);
    let (x, y, w, hh) = (word(h, x_at), word(h, y_at), word(h, w_at), word(h, h_at));
    h.call_sym(
        SCISSOR2,
        Conv::Cdecl,
        &[x, x.wrapping_add(w), y, y.wrapping_add(hh)],
    );
    let (x, y) = (word(h, x_at), word(h, y_at));
    h.call_sym(XYOFFSET, Conv::Cdecl, &[x, y]);
    let (x, y, w, hh) = (word(h, x_at), word(h, y_at), word(h, w_at), word(h, h_at));
    h.call_sym(
        SCISSOR,
        Conv::Cdecl,
        &[x, x.wrapping_add(w), y, y.wrapping_add(hh)],
    );
    let packet = h.call_sym(GS_END2, Conv::Cdecl, &[]);
    h.call_sym(SEND_SPAD, Conv::Cdecl, &[packet])
}

/// The word at `at`.
fn word(h: &impl Host, at: u32) -> u32 {
    h.read(at)
}

// ---- the split image load

/// The load-image record the PS2 effect layer uses (`0x011a08b0`).
pub const LOAD_IMAGE_RECORD: u32 = 0x011a_08b0;

/// `op_gs_loadImage_007e4420(image, data)` (cdecl): the image's `u16`
/// fields (dbp `+0`, dbw `+2`, psm `+4`, x `+6`, y `+8`, w `+0xa`, h
/// `+0xc`) loaded into GS memory: whole when `i16 w * i16 h` is below
/// 0x40000; else in 256-row strips (rows counted against the `i16` h,
/// read again each strip), each at dbp advanced by `w` (as the original's
/// `(w << 8) / 256` gives it, cut to 16 bits) and data by `w << 8`.
/// Each load is `sceGsSetDefLoadImage`, `FlushCache(0)`, a path sync,
/// `sceGsExecLoadImage` and a path sync. The strip counter lives in the
/// original's `image` argument slot, which neither caller reads after
/// (`check-argslots`).
pub fn load_image_split<H: Host>(h: &mut H, image: u32, data: u32) {
    let record = h.rebase(LOAD_IMAGE_RECORD);
    let field = |h: &mut H, off: u32| u32::from(h.read::<u16>(image.wrapping_add(off)));
    let load = |h: &mut H, args: &[u32], data: u32| {
        h.call_sym(SET_DEF_LOAD_IMAGE, Conv::Cdecl, args);
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0]);
        h.call_sym(EXEC_LOAD_IMAGE, Conv::Cdecl, &[record, data]);
        h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0]);
    };
    let w = field(h, 0xa);
    let hh = field(h, 0xc);
    let area = i32::from(w as u16 as i16).wrapping_mul(i32::from(hh as u16 as i16));
    if area < 0x4_0000 {
        let args = [
            record,
            field(h, 0),
            field(h, 2),
            field(h, 4),
            field(h, 6),
            field(h, 8),
            w,
            hh,
        ];
        load(h, &args, data);
        return;
    }
    let mut dbp = field(h, 0);
    let scaled = i32::from(w as u16 as i16).wrapping_shl(8);
    let bias = (scaled >> 31) & 0xff;
    let step = (scaled.wrapping_add(bias) >> 8) as u32 & 0xffff;
    let data_step = step << 8;
    let mut data = data;
    let mut row = 0i32;
    if i32::from(hh as u16 as i16) <= 0 {
        return;
    }
    loop {
        let args = [
            record,
            dbp,
            field(h, 2),
            field(h, 4),
            field(h, 6),
            0,
            field(h, 0xa),
            0x100,
        ];
        load(h, &args, data);
        dbp = dbp.wrapping_add(step);
        data = data.wrapping_add(data_step);
        row = row.wrapping_add(0x100);
        if row >= i32::from(h.read::<i16>(image.wrapping_add(0xc))) {
            break;
        }
    }
}

// ---- IggyMenu's constructor

/// `IggyMenu_vftable` (`0x00b78c54`).
pub const IGGY_MENU_VTABLE: u32 = 0x00b7_8c54;
/// `operator_new(n)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `Phyre_PSerialization_PStreamFile_ctor(stream; path, 1, 0, 0, 1)`
/// (thiscall, ret 0x14): the stream.
pub const STREAM_CTOR: u32 = 0x0060_7bc0;
/// `Phyre_PSerialization_PStreamFile_getFileSize(stream)` (thiscall).
pub const STREAM_SIZE: u32 = 0x0060_7dc0;
/// `operator_new_006304c0(n)` (cdecl).
pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
/// `Phyre_PSerialization_PStreamFile_read(stream; buffer, n)` (thiscall,
/// ret 8).
pub const STREAM_READ: u32 = 0x0060_8090;
/// `Iggy_createPlayer_00a3e4f0(0, 0, 0, 0, data, n, 0)` (cdecl).
pub const CREATE_PLAYER: u32 = 0x00a3_e4f0;
/// The Iggy menu manager (`0x01984c6c`).
pub const MENU_MANAGER: u32 = 0x0198_4c6c;
/// `IggyMenuManager_addMenu_00a265f0(manager; menu)` (thiscall, ret 4).
pub const ADD_MENU: u32 = 0x00a2_65f0;
/// `operator_delete_00630540(p)` (cdecl).
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
/// `Phyre_PSerialization_PStreamFile_closeFile(stream)` (thiscall).
pub const STREAM_CLOSE: u32 = 0x0060_7d80;
/// `FUN_00607c10(stream)` (thiscall, a bare `ret`): the stream's
/// destructor.
pub const STREAM_DTOR: u32 = 0x0060_7c10;
/// `operator_delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;

/// `IggyMenu_ctor_00a288f0(this; path, x, y, w, h, flags)` (thiscall, ret
/// 0x18; the SEH frame is not kept): the vtable, the rectangle floats
/// (`+0x74`..`+0x80`, each through `fld`/`fstp`), `+0x68` 0, bytes
/// `+0x70` and `+0x94` 1, `flags` at `+0x98`; the `.swf` file read whole
/// through a `PStreamFile` (a null stream when `operator new` fails, as
/// in the original) into a temporary buffer, an Iggy player made from it
/// (`+0x6c`), the menu added to the manager, buffer and stream freed,
/// and `+4`..`+0x68` cleared (100 bytes). Returns `this`.
pub fn iggy_menu_ctor(h: &mut impl Host, this: u32, path: u32, rect: [u32; 4], flags: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    h.write(at(0x74), pass(rect[0]));
    let vtable = h.rebase(IGGY_MENU_VTABLE);
    h.write(this, vtable);
    h.write(at(0x78), pass(rect[1]));
    h.write(at(0x68), 0u32);
    h.write(at(0x70), 1u8);
    h.write(at(0x7c), pass(rect[2]));
    h.write(at(0x94), 1u8);
    h.write(at(0x98), flags);
    h.write(at(0x80), pass(rect[3]));
    let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[8]);
    let stream = if block == 0 {
        0
    } else {
        h.call_sym(STREAM_CTOR, Conv::Thiscall(block), &[path, 1, 0, 0, 1])
    };
    let size = h.call_sym(STREAM_SIZE, Conv::Thiscall(stream), &[]);
    let buffer = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[size]);
    h.call_sym(STREAM_READ, Conv::Thiscall(stream), &[buffer, size]);
    let player = h.call_sym(CREATE_PLAYER, Conv::Cdecl, &[0, 0, 0, 0, buffer, size, 0]);
    h.write(at(0x6c), player);
    let manager: u32 = h.read(h.rebase(MENU_MANAGER));
    h.call_sym(ADD_MENU, Conv::Thiscall(manager), &[this]);
    h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[buffer]);
    h.call_sym(STREAM_CLOSE, Conv::Thiscall(stream), &[]);
    if stream != 0 {
        h.call_sym(STREAM_DTOR, Conv::Thiscall(stream), &[]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[stream]);
    }
    h.call_sym(MEMSET, Conv::Cdecl, &[at(4), 0, 100]);
    this
}

// ---- the render thread's command queue start

/// `CreateSemaphoreW`'s import slot (stdcall, 4 words).
pub const CREATE_SEMAPHORE_IAT: u32 = 0x00b0_c1e0;
/// `WaitForSingleObject`'s import slot (stdcall, 2 words).
pub const WAIT_IAT: u32 = 0x00b0_c0f8;
/// `ReleaseSemaphore`'s import slot (stdcall, 3 words).
pub const RELEASE_SEMAPHORE_IAT: u32 = 0x00b0_c0f4;
/// `FUN_00703a50(thread; 0, stack)` (thiscall, ret 8): starts a thread
/// (`CreateThread` with `stack` bytes).
pub const THREAD_START: u32 = 0x0070_3a50;

/// A semaphore (8 of 8) in a block of its own, read back: for a null
/// block the original reads address 0 too.
fn boxed_semaphore(h: &mut impl Host) -> u32 {
    let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[4]);
    if block != 0 {
        let create: u32 = h.read(h.rebase(CREATE_SEMAPHORE_IAT));
        let s = h.call(create, Conv::Stdcall, &[0, 8, 8, 0]);
        h.write(block, s);
    }
    h.read(block)
}

/// `SemaphoreQueue_start_006dba20(this)` (thiscall, from
/// `graphicInitialize`): two counting semaphores (8 of 8) at `+0x234`
/// and `+0x230`; the eight 0x30-byte entries from `+0x50` queued in the
/// ring (`+0x1d0`, index `+0x1f4` advanced by 7 mod 8 as a signed
/// remainder, count `+0x1f8`) under the lock `+0x1fc`, each taking
/// `+0x230` and giving `+0x234`; `+0x220`..`+0x228` cleared under the
/// lock `+0x22c`; then four threads started (`this + 0x14 k`, 128 KB
/// stacks).
pub fn semaphore_queue_start(h: &mut impl Host, this: u32) {
    let at = |off: u32| this.wrapping_add(off);
    let s = boxed_semaphore(h);
    h.write(at(0x234), s);
    let s = boxed_semaphore(h);
    h.write(at(0x230), s);
    let wait: u32 = h.read(h.rebase(WAIT_IAT));
    let release: u32 = h.read(h.rebase(RELEASE_SEMAPHORE_IAT));
    let mut entry = at(0x50);
    for _ in 0..8 {
        let lock: u32 = h.read(at(0x1fc));
        h.call(wait, Conv::Stdcall, &[lock, u32::MAX]);
        let mut i = h.read::<u32>(at(0x1f4)).wrapping_add(7) & 0x8000_0007;
        if (i as i32) < 0 {
            i = (i.wrapping_sub(1) | 0xffff_fff8).wrapping_add(1);
        }
        h.write(at(0x1f4), i);
        h.write(at(0x1d0).wrapping_add(i.wrapping_mul(4)), entry);
        let n: u32 = h.read(at(0x1f8));
        h.write(at(0x1f8), n.wrapping_add(1));
        let lock: u32 = h.read(at(0x1fc));
        h.call(release, Conv::Stdcall, &[lock, 1, 0]);
        let free: u32 = h.read(at(0x230));
        h.call(wait, Conv::Stdcall, &[free, u32::MAX]);
        let used: u32 = h.read(at(0x234));
        h.call(release, Conv::Stdcall, &[used, 1, 0]);
        entry = entry.wrapping_add(0x30);
    }
    let lock: u32 = h.read(at(0x22c));
    h.call(wait, Conv::Stdcall, &[lock, u32::MAX]);
    for off in [0x228u32, 0x224, 0x220] {
        h.write(at(off), 0u32);
    }
    let lock: u32 = h.read(at(0x22c));
    h.call(release, Conv::Stdcall, &[lock, 1, 0]);
    for k in 0..4 {
        h.call_sym(THREAD_START, Conv::Thiscall(at(0x14 * k)), &[0, 0x2_0000]);
    }
}

// ---- the escape menu's dialog callback

/// `__RTDynamicCast(p, 0, from, to, 0)` (cdecl).
pub const DYNAMIC_CAST: u32 = 0x0094_9d86;
/// `IggyMenu`'s RTTI type descriptor (`0x00c33f60`).
pub const IGGY_MENU_RTTI: u32 = 0x00c3_3f60;
/// `FFEscMenu`'s RTTI type descriptor (`0x00c33f78`).
pub const ESC_MENU_RTTI: u32 = 0x00c3_3f78;
/// `GetDerefU32_00a28d10(menu)` (thiscall): `[[menu + 0x6c]]`.
pub const DEREF_PLAYER: u32 = 0x00a2_8d10;
/// `IggyValueRef_create_00a288b0(ref; player, temp)` (thiscall, ret 8).
pub const VALUE_REF_CREATE: u32 = 0x00a2_88b0;
/// `IggyMenu_getArrayS32_00a28b20(ref; i)` (thiscall, ret 4).
pub const GET_ARRAY_S32: u32 = 0x00a2_8b20;
/// `FFEscMenu_addHandler_00702570(menu; i)` (thiscall, ret 4).
pub const ADD_HANDLER: u32 = 0x0070_2570;
/// `IggyMenuHandlers_addCall(menu; f, arg)` (thiscall, ret 8).
pub const ADD_CALL: u32 = 0x00a2_8df0;
/// `FFEscMenu_dialogText_0068e560(id)` (cdecl).
pub const DIALOG_TEXT: u32 = 0x0068_e560;
/// `IggyValueRef_free_00a28a20(ref)` (thiscall).
pub const VALUE_REF_FREE: u32 = 0x00a2_8a20;
/// The dialog table (`0x00c33dc0`, 0x23 entries of 8 bytes: the text id
/// word, then the two `i16` keys at `+4` and `+6`).
pub const DIALOG_TABLE: u32 = 0x00c3_3dc0;
/// The text for keys 0x460/0x461 below 100 (`0x00c33db8`).
pub const DIALOG_LOW: u32 = 0x00c3_3db8;
/// The keys of the last unknown dialog (`0x00ccc87c`, two `u16`).
pub const UNKNOWN_KEYS: u32 = 0x00cc_c87c;
/// Bytes of [`esc_dialog`]'s frame: the value reference at `ebp - 0x48`.
pub const DIALOG_FRAME: u32 = 0x3c;

/// `FFEscMenu_onDialog_0068e5b0(menu, args)` (cdecl, an `IggyMenu`
/// callback registered by `FFEscMenu_registerCallbacks`; `frame` its
/// value reference; the SEH frame is not kept): nothing when the escape
/// menu (`__RTDynamicCast`) is busy (`+0xfc`). Else the two numbers of
/// the call's array (`[args + 4]`) read: (0xc, k) adds handler `k`;
/// otherwise keys 100..=102 (except (0x6e, 0x66)) clear the first, and
/// the dialog's text is looked up: keys 0x460/0x461 with a second below
/// 100 use the fixed text, else the table entry matching both; none
/// found keeps the keys at `0x00ccc87c` and passes 0. The text goes to
/// `FFEscMenu_dialogText` through the menu's call queue. Returns the
/// cast's `eax` when busy, else `IggyValueRef_free`'s.
pub fn esc_dialog(h: &mut impl Host, frame: u32, menu: u32, args: u32) -> u32 {
    let (from, to) = (h.rebase(IGGY_MENU_RTTI), h.rebase(ESC_MENU_RTTI));
    let esc = h.call_sym(DYNAMIC_CAST, Conv::Cdecl, &[menu, 0, from, to, 0]);
    if h.read::<u8>(esc.wrapping_add(0xfc)) != 0 {
        return esc;
    }
    let temp: u32 = h.read(args.wrapping_add(4));
    let player = h.call_sym(DEREF_PLAYER, Conv::Thiscall(menu), &[]);
    h.call_sym(VALUE_REF_CREATE, Conv::Thiscall(frame), &[player, temp]);
    let mut first = h.call_sym(GET_ARRAY_S32, Conv::Thiscall(frame), &[0]) as u16;
    let second = h.call_sym(GET_ARRAY_S32, Conv::Thiscall(frame), &[1]) as u16;
    let mut handled = false;
    if second.wrapping_sub(100) <= 2 && (second != 0x66 || first != 0x6e) {
        first = 0;
    } else if first == 0xc {
        h.call_sym(
            ADD_HANDLER,
            Conv::Thiscall(esc),
            &[i32::from(second as i16) as u32],
        );
        handled = true;
    }
    if !handled {
        let table = h.rebase(DIALOG_TABLE);
        let mut text = None;
        for i in 0..0x23u32 {
            if first == 0x461 || first == 0x460 {
                if (second as i16) < 100 {
                    text = Some(h.rebase(DIALOG_LOW));
                    break;
                }
            } else if h.read::<u16>(table.wrapping_add(8 * i + 4)) == first
                && h.read::<u16>(table.wrapping_add(8 * i + 6)) == second
            {
                text = Some(table.wrapping_add(8 * i));
                break;
            }
        }
        let text = if let Some(t) = text {
            t
        } else {
            let keys = h.rebase(UNKNOWN_KEYS);
            h.write(keys, first);
            h.write(keys.wrapping_add(2), second);
            0
        };
        let f = h.rebase(DIALOG_TEXT);
        h.call_sym(ADD_CALL, Conv::Thiscall(menu), &[f, text]);
    }
    h.call_sym(VALUE_REF_FREE, Conv::Thiscall(frame), &[])
}
