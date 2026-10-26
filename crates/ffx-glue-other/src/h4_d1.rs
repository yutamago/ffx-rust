//! Continuation round 2 of list `h4-glue-other`, batch 1: the particle
//! fill-buffer jobs, the Iggy manager's render and menu insert, the `GDraw`
//! render-thread setup, the sync file read, the data directory, the menu
//! textures' kind dispatch and the sub-menu preload, the distortion
//! target, the extended blur's parameter copy, `SfxEvent_isPlaying` and
//! the Atel shape's VFX setup.
//!
//! Bridges that hand callees a local take `frame`, the block laid out as
//! the original's frame from its lowest local up.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

use crate::frames::ERRCHECK;

// ---- the particle fill-buffer jobs

/// The fill routine of each part kind (`byte +0` of a 0x214-byte part),
/// cdecl `(part)`: `drawMdl`, `drawShp_00727110`, `drawModelScroll`,
/// `drawMdlDtt`, `drawShp_007287b0`, `drawShp`, `md0_ref_0279`.
pub const FILL_PART: [u32; 7] = [
    0x0071_f320,
    0x0072_7110,
    0x0072_69a0,
    0x0072_56c0,
    0x0072_87b0,
    0x0071_ee80,
    0x0072_82d0,
];
/// Bytes of a part record.
pub const PART_SIZE: u32 = 0x214;

/// `JobSchedule_pppPart_fillBuffer(parts, &start, &end)` (`0x0071ed10`,
/// cdecl): each part `start..end` (signed) of the 0x214-byte array
/// `parts` filled by its kind's routine ([`FILL_PART`]; kinds outside
/// 0..6 skipped). Returns what the original leaves in `eax`: `*start`
/// for an empty range, else the last part's routine result or its
/// sign-extended kind.
pub fn fill_buffer(h: &mut impl Host, parts: u32, start: u32, end: u32) -> u32 {
    let last: u32 = h.read(end);
    let first: u32 = h.read(start);
    if first.cast_signed() >= last.cast_signed() {
        return first;
    }
    let mut part = first.wrapping_mul(PART_SIZE).wrapping_add(parts);
    let mut n = last.wrapping_sub(first);
    let mut eax;
    loop {
        let kind = i32::from(h.read::<i8>(part)).cast_unsigned();
        eax = kind;
        if let Some(&f) = FILL_PART.get(kind as usize) {
            eax = h.call_sym(f, Conv::Cdecl, &[part]);
        }
        part = part.wrapping_add(PART_SIZE);
        n = n.wrapping_sub(1);
        if n == 0 {
            return eax;
        }
    }
}

/// The six part arrays (`0x00cec1a8`) and their request counts
/// (`g_FillBufferRequests`, `0x00cec1c0`), then six more counts
/// (`0x00cec1d8`).
pub const FILL_BUFFERS: u32 = 0x00ce_c1a8;
/// `g_FillBufferRequests`.
pub const FILL_REQUESTS: u32 = 0x00ce_c1c0;
/// The second six counts.
pub const FILL_WEIGHTS: u32 = 0x00ce_c1d8;
/// The job scheduler (`0x00cdede4`, a pointer).
pub const JOB_SCHEDULER: Global<u32> = Global::new(0x00cd_ede4);
/// `0x0071e720(scheduler; fn, a, b, c, d)` (thiscall, ret 0x14): queues
/// a job.
pub const QUEUE_JOB: u32 = 0x0071_e720;
/// `JobSchedule_pppPart_fillBuffer`.
pub const FILL_BUFFER: u32 = 0x0071_ed10;
/// Bytes of [`fill_buffer_kick`]'s frame (the start word).
pub const KICK_FRAME: u32 = 4;

/// `JobSchedule_pppPart_fillBuffer_Kick(inline)` (`0x0071edb0`, cdecl;
/// `frame` its start word): with any weight (the second six counts sum
/// non-zero), and the weights at 500 or more or the requests at 50 or
/// more (signed) without `inline`, one job per buffer
/// (`QUEUE_JOB(scheduler; fillBuffer, parts, 0, requests, 0)`); else
/// each buffer filled here (`fillBuffer(parts, &0, &requests)`).
/// Returns what the original leaves in `eax`: the requests' sum when
/// there is no weight, else the last call's result.
pub fn fill_buffer_kick(h: &mut impl Host, frame: u32, inline: u32) -> u32 {
    let requests = h.rebase(FILL_REQUESTS);
    let weights = h.rebase(FILL_WEIGHTS);
    let buffers = h.rebase(FILL_BUFFERS);
    let mut sum = 0u32;
    let mut weight = 0u32;
    for k in 0..6 {
        sum = sum.wrapping_add(h.read(requests + 4 * k));
        weight = weight.wrapping_add(h.read(weights + 4 * k));
    }
    if weight == 0 {
        return sum;
    }
    let mut eax = 0;
    if (weight.cast_signed() >= 500 || sum.cast_signed() >= 50) && inline == 0 {
        let job = h.rebase(FILL_BUFFER);
        for k in 0..6 {
            let scheduler = JOB_SCHEDULER.get(h);
            let parts: u32 = h.read(buffers + 4 * k);
            let count: u32 = h.read(requests + 4 * k);
            eax = h.call_sym(
                QUEUE_JOB,
                Conv::Thiscall(scheduler),
                &[job, parts, 0, count, 0],
            );
        }
    } else {
        h.write(frame, 0u32);
        for k in 0..6 {
            let parts: u32 = h.read(buffers + 4 * k);
            eax = h.call_sym(FILL_BUFFER, Conv::Cdecl, &[parts, frame, requests + 4 * k]);
        }
    }
    eax
}

// ---- the Iggy manager

/// `Phyre_PRendering_PRenderer_setColorTarget(renderer; a, b)`.
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
/// `Phyre_PRendering_PRenderer_setDepthStencilTarget(renderer; a)`.
pub const SET_DEPTH_TARGET: u32 = 0x005b_b5b0;
/// `Phyre_PRendering_PRenderer_beginScene(renderer; a)`.
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
/// `Phyre_PRendering_PRenderer_endScene(renderer; a)`.
pub const END_SCENE: u32 = 0x005b_6950;
/// `IggyMenu_Draw_00a28eb0(menu)` (thiscall).
pub const MENU_DRAW: u32 = 0x00a2_8eb0;
/// `IggyMenuManager_EndDraw_00a362e0(gdraw)` (thiscall).
pub const END_DRAW: u32 = 0x00a3_62e0;

/// `IggyMenuManager_Render_00a267f0(this)` (thiscall): when enabled (byte
/// `+0x30`), on the renderer `+0`: no colour or depth target, a scene;
/// with a `GDraw` context (`+4`) the menus of the array (`+0xc`, count
/// `+8` without its top bit) drawn layer by layer (`menu+0x98` = 0, 1,
/// 2), then the `GDraw` end; the scene ended (`endScene(1)`). Returns the
/// last call's `eax`; when disabled the original leaves its caller's
/// `eax`, which none of its four callers reads.
pub fn iggy_manager_render(h: &mut impl Host, this: u32) -> u32 {
    if h.read::<u8>(this + 0x30) == 0 {
        return 0;
    }
    let r: u32 = h.read(this);
    h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(r), &[0, 0]);
    let r: u32 = h.read(this);
    h.call_sym(SET_DEPTH_TARGET, Conv::Thiscall(r), &[0]);
    let r: u32 = h.read(this);
    h.call_sym(BEGIN_SCENE, Conv::Thiscall(r), &[0]);
    if h.read::<u32>(this + 4) != 0 {
        for layer in 0..3u32 {
            let mut i = 0u32;
            while i < h.read::<u32>(this + 8) & 0x7fff_ffff {
                let menus: u32 = h.read(this + 0xc);
                let menu: u32 = h.read(menus.wrapping_add(4 * i));
                if menu != 0 && h.read::<u32>(menu.wrapping_add(0x98)) == layer {
                    h.call_sym(MENU_DRAW, Conv::Thiscall(menu), &[]);
                }
                i += 1;
            }
        }
        let gdraw: u32 = h.read(this + 4);
        h.call_sym(END_DRAW, Conv::Thiscall(gdraw), &[]);
    }
    let r: u32 = h.read(this);
    h.call_sym(END_SCENE, Conv::Thiscall(r), &[1])
}

/// `SetField68_00a28f30(menu; manager)` (thiscall, ret 4).
pub const MENU_SET_MANAGER: u32 = 0x00a2_8f30;
/// `0x00a28f40(menu; x, width, y, height)` (thiscall, ret 0x10): the
/// menu's viewport.
pub const MENU_SET_VIEWPORT: u32 = 0x00a2_8f40;
/// `Phyre_PhyreMalloc_0042fb80(bytes, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `0x00a26480(array; storage, &value)` (thiscall, ret 8): the menu
/// array's append into new storage.
pub const MENU_ARRAY_APPEND: u32 = 0x00a2_6480;

fn set_viewport(h: &mut impl Host, this: u32, menu: u32) -> u32 {
    let height: u32 = h.read(this + 0x24);
    let width = h
        .read::<u32>(this + 0x2c)
        .wrapping_sub(h.read(this + 0x1c))
        .wrapping_sub(height);
    let x: u32 = h.read(this + 0x18);
    let y: u32 = h.read(this + 0x20);
    h.call_sym(
        MENU_SET_VIEWPORT,
        Conv::Thiscall(menu),
        &[x, width, y, height],
    )
}

/// `IggyMenuManager_addMenu_00a265f0(this; menu)` (thiscall, ret 4;
/// `frame` its argument slot, `menu` written there): a menu goes into the
/// first empty slot of the array (`+0xc`, count `+8` without its top
/// bit), else is appended (new storage of `(count + 1) * 4` bytes from
/// `PhyreMalloc(.., 4)`, `MENU_ARRAY_APPEND(this + 8; storage, &menu)`);
/// either way it learns its manager (`+0`) and its viewport (x `+0x18`,
/// width `+0x2c - +0x1c - +0x24`, y `+0x20`, height `+0x24`). Returns the
/// last call's `eax`; with no menu the original leaves its caller's,
/// which its one caller does not read.
pub fn iggy_manager_add_menu(h: &mut impl Host, frame: u32, this: u32, menu: u32) -> u32 {
    h.write(frame, menu);
    if menu == 0 {
        return 0;
    }
    let n = h.read::<u32>(this + 8) & 0x7fff_ffff;
    let slots: u32 = h.read(this + 0xc);
    for i in 0..n {
        if h.read::<u32>(slots.wrapping_add(4 * i)) != 0 {
            continue;
        }
        h.write(slots.wrapping_add(4 * i), menu);
        let manager: u32 = h.read(this);
        let m: u32 = h.read(h.read::<u32>(this + 0xc).wrapping_add(4 * i));
        let eax = h.call_sym(MENU_SET_MANAGER, Conv::Thiscall(m), &[manager]);
        let m: u32 = h.read(h.read::<u32>(this + 0xc).wrapping_add(4 * i));
        if m == 0 {
            return eax;
        }
        return set_viewport(h, this, m);
    }
    let manager: u32 = h.read(this);
    h.call_sym(MENU_SET_MANAGER, Conv::Thiscall(menu), &[manager]);
    set_viewport(h, this, menu);
    let n = h.read::<u32>(this + 8) & 0x7fff_ffff;
    let storage = h.call_sym(
        PHYRE_MALLOC,
        Conv::Cdecl,
        &[n.wrapping_add(1).wrapping_mul(4), 4],
    );
    h.call_sym(
        MENU_ARRAY_APPEND,
        Conv::Thiscall(this + 8),
        &[storage, frame],
    )
}

// ---- `GDraw`'s render-thread setup

/// `_IggyAudioInstallMP3Decoder@0` (import slot `0x00b0c864`).
pub const IGGY_INSTALL_MP3: Global<u32> = Global::new(0x00b0_c864);
/// `_IggyAudioUseDirectSound@0` (import slot `0x00b0c860`).
pub const IGGY_USE_DIRECT_SOUND: Global<u32> = Global::new(0x00b0_c860);
/// `Phyre_PRendering_PRenderer_syncRender(renderer)` (thiscall).
pub const SYNC_RENDER: u32 = 0x005b_c3f0;
/// `PRenderCommandQueue_queueCallback(renderer; fn, data)` (thiscall,
/// ret 8).
pub const QUEUE_CALLBACK: u32 = 0x005b_5880;
/// `Call_00a3baf0_00a34ae0`, the queued callback.
pub const SETUP_CALLBACK: u32 = 0x00a3_4ae0;
/// Bytes of [`gdraw_render_setup`]'s frame: the callback's record
/// (`this`, `arg`, its result word).
pub const GDRAW_SETUP_FRAME: u32 = 0xc;

/// The renderer's virtual `+8` (thiscall), then `syncRender`: the first
/// non-zero result.
fn flush(h: &mut impl Host, renderer: u32) -> u32 {
    let vtable: u32 = h.read(renderer);
    let f: u32 = h.read(vtable.wrapping_add(8));
    let r = h.call(f, Conv::Thiscall(renderer), &[]);
    if r != 0 {
        return r;
    }
    h.call_sym(SYNC_RENDER, Conv::Thiscall(renderer), &[])
}

/// `IggyGDraw_renderSetup_00a3ba50(this; renderer, &audio, arg)`
/// (thiscall, ret 0xc; `frame` the callback's record): with the audio
/// flag set, Iggy's MP3 decoder and `DirectSound` installed; the
/// renderer flushed (its virtual `+8`, `syncRender`), the setup callback
/// queued on the render thread with the record `{this, arg, 0}`, flushed
/// again; then the renderer is kept (`+4`) and the record's result word
/// returned. The first failing call's code is returned instead.
pub fn gdraw_render_setup(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    renderer: u32,
    audio: u32,
    arg: u32,
) -> u32 {
    if h.read::<u8>(audio) != 0 {
        let f = IGGY_INSTALL_MP3.get(h);
        h.call(f, Conv::Stdcall, &[]);
        let f = IGGY_USE_DIRECT_SOUND.get(h);
        h.call(f, Conv::Stdcall, &[]);
    }
    let r = flush(h, renderer);
    if r != 0 {
        return r;
    }
    h.write(frame + 4, arg);
    h.write(frame, this);
    h.write(frame + 8, 0u32);
    let callback = h.rebase(SETUP_CALLBACK);
    let r = h.call_sym(QUEUE_CALLBACK, Conv::Thiscall(renderer), &[callback, frame]);
    if r != 0 {
        return r;
    }
    let r = flush(h, renderer);
    if r != 0 {
        return r;
    }
    h.write(this + 4, renderer);
    h.read(frame + 8)
}

// ---- files

/// `dbgPrintf(fmt, ...)` (cdecl).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// `fiosIsFileExist(path)` (cdecl).
pub const FIOS_EXISTS: u32 = 0x0067_96f0;
/// `fiosOpen(path, mode)` (cdecl).
pub const FIOS_OPEN: u32 = 0x0067_9730;
/// `fiosLseek(file, offset, whence)` (cdecl).
pub const FIOS_SEEK: u32 = 0x0067_9710;
/// `fiosRead(file, buf, n)` (cdecl).
pub const FIOS_READ: u32 = 0x0067_9800;
/// `fiosClose(file)` (cdecl).
pub const FIOS_CLOSE: u32 = 0x0067_9510;
/// `user_aligned_malloc(bytes, align)` (cdecl).
pub const ALIGNED_MALLOC: u32 = 0x0068_6ff0;
/// The two messages: no path; the file is missing or does not open.
pub const SYNC_FILE_MESSAGES: [u32; 2] = [0x00b4_c448, 0x00b4_c45c];

/// `SyncManager_readSyncFile(path)` (stdcall): the whole file read into
/// a 16-aligned block (`fiosLseek` to the end for the size, back to 0;
/// `user_aligned_malloc`, `fiosRead`, `fiosClose`); returns the block, or
/// 0 after a `dbgPrintf` when there is no path or the file is missing or
/// does not open.
pub fn read_sync_file(h: &mut impl Host, path: u32) -> u32 {
    if path == 0 {
        let msg = h.rebase(SYNC_FILE_MESSAGES[0]);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg]);
        return 0;
    }
    if h.call_sym(FIOS_EXISTS, Conv::Cdecl, &[path]) != 0 {
        let file = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[path, 1]);
        if file != 0 {
            let size = h.call_sym(FIOS_SEEK, Conv::Cdecl, &[file, 0, 2]);
            h.call_sym(FIOS_SEEK, Conv::Cdecl, &[file, 0, 0]);
            let block = h.call_sym(ALIGNED_MALLOC, Conv::Cdecl, &[size, 0x10]);
            h.call_sym(FIOS_READ, Conv::Cdecl, &[file, block, size]);
            h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[file]);
            return block;
        }
    }
    let msg = h.rebase(SYNC_FILE_MESSAGES[1]);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg]);
    0
}

/// `crt_memset_thunk(p, c, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `fiosUnifyFilename(name, out, n)` (cdecl).
pub const FIOS_UNIFY: u32 = 0x0067_9820;
/// `sprintf_s` (the import slot `0x00b0c394`; cdecl).
pub const SPRINTF_S: Global<u32> = Global::new(0x00b0_c394);
/// The `"%s%s"`-style format of [`data_directory`] (`0x00b0e160`).
pub const DATA_DIRECTORY_FORMAT: u32 = 0x00b0_e160;
/// `Phyre_PSerialization_PStreamFile_GetStreamPrefix()` (cdecl).
pub const STREAM_PREFIX: u32 = 0x0060_7d30;
/// `Phyre_PhyrePrintf(level, fmt, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// The too-long warning (`0x00b44dbc`).
pub const DATA_DIRECTORY_WARNING: u32 = 0x00b4_4dbc;
/// Bytes of [`data_directory`]'s frame (the unified name, the cookie).
pub const DATA_DIRECTORY_FRAME: u32 = 0x104;

fn strlen(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `FmodFile_getDataDirectory_00679a10(name, out, size)` (cdecl; `frame`
/// its 0x100-byte name buffer): `out` cleared (`memset`), `name` unified
/// into the frame (`fiosUnifyFilename`), `sprintf_s(out, size, FORMAT,
/// unified)`; when `size` (signed) is below the stream prefix's length
/// plus the name's, a `PhyrePrintf(1, WARNING, name)`. Returns that
/// length, or the print's result.
pub fn data_directory(h: &mut impl Host, frame: u32, name: u32, out: u32, size: u32) -> u32 {
    h.call_sym(MEMSET, Conv::Cdecl, &[out, 0, size]);
    h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[name, frame, 0x100]);
    let f = SPRINTF_S.get(h);
    let fmt = h.rebase(DATA_DIRECTORY_FORMAT);
    h.call(f, Conv::Cdecl, &[out, size, fmt, frame]);
    let name_len = strlen(h, name);
    let prefix = h.call_sym(STREAM_PREFIX, Conv::Cdecl, &[]);
    let len = strlen(h, prefix).wrapping_add(name_len);
    if size.cast_signed() < len.cast_signed() {
        let warning = h.rebase(DATA_DIRECTORY_WARNING);
        return h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, warning, name]);
    }
    len
}

// ---- menu textures

/// What a menu-texture kind loads or unloads (thiscall on the texture
/// manager), by [`special_menu_texture`]'s argument rule.
#[derive(Clone, Copy)]
enum Arg {
    /// No argument.
    None,
    /// The first extra argument.
    A,
    /// The second extra argument.
    B,
    /// The kind itself.
    Kind,
}

/// `MenuTextureManager_UnloadSpecialMenuTexture`'s kinds 0..16: the
/// routine (0: nothing) and its argument, plus a second routine.
const UNLOAD_KINDS: [(u32, Arg, u32); 17] = [
    (0x0067_f610, Arg::None, 0),
    (0x0067_f230, Arg::None, 0),
    (0x0067_f3c0, Arg::None, 0),
    (0x0067_f9a0, Arg::None, 0),
    (0x0067_fa20, Arg::None, 0),
    (0x0067_ecf0, Arg::None, 0x0067_f920),
    (0x0067_ee70, Arg::None, 0),
    (0x0067_efa0, Arg::None, 0),
    (0x0067_f3e0, Arg::None, 0),
    (0, Arg::None, 0),
    (0x0067_ebc0, Arg::None, 0),
    (0x0067_edc0, Arg::Kind, 0),
    (0x0067_edc0, Arg::Kind, 0),
    (0x0067_edc0, Arg::Kind, 0),
    (0x0067_edc0, Arg::Kind, 0),
    (0x0067_edc0, Arg::Kind, 0),
    (0x0067_ee40, Arg::None, 0),
];

/// `MenuTextureManager_LoadSpecialMenuTexture`'s kinds 0..16.
const LOAD_KINDS: [(u32, Arg, u32); 17] = [
    (0x0067_d370, Arg::A, 0),
    (0x0067_ce50, Arg::B, 0),
    (0x0067_d0b0, Arg::A, 0),
    (0x0067_d670, Arg::None, 0),
    (0x0067_d690, Arg::None, 0),
    (0x0067_c6f0, Arg::None, 0x0067_d650),
    (0x0067_cb30, Arg::B, 0),
    (0x0067_cba0, Arg::None, 0),
    (0x0067_d180, Arg::A, 0),
    (0, Arg::None, 0),
    (0x0067_c8c0, Arg::None, 0),
    (0x0067_c720, Arg::Kind, 0),
    (0x0067_c720, Arg::Kind, 0),
    (0x0067_c720, Arg::Kind, 0),
    (0x0067_c720, Arg::Kind, 0),
    (0x0067_c720, Arg::Kind, 0),
    (0x0067_c8f0, Arg::None, 0),
];

/// The kind dispatch of both: kinds 0..16 (signed) call their routine on
/// `this` (each callee pops its own argument); the others and kind 9 do
/// nothing. Returns the last routine's `eax`, or the kind.
fn special_menu_texture(
    h: &mut impl Host,
    table: &[(u32, Arg, u32); 17],
    this: u32,
    kind: u32,
    a: u32,
    b: u32,
) -> u32 {
    let Some(&(f, arg, then)) = table.get(kind as usize) else {
        return kind;
    };
    if f == 0 {
        return kind;
    }
    let args: &[u32] = match arg {
        Arg::None => &[],
        Arg::A => &[a],
        Arg::B => &[b],
        Arg::Kind => &[kind],
    };
    let eax = h.call_sym(f, Conv::Thiscall(this), args); // call width: 1 at most
    if then == 0 {
        return eax;
    }
    h.call_sym(then, Conv::Thiscall(this), &[])
}

/// `MenuTextureManager_UnloadSpecialMenuTexture(this; kind)` (thiscall,
/// ret 4): kind 0 the event font, 1 the Sa menu, 2 `0x0067f3c0`, 3
/// `0x0067f9a0`, 4 `0x0067fa20`, 5 the water background then
/// `0x0067f920`, 6 the game menu, 7 the main menu's wave, 8 the battle
/// font, 10 the dream map, 11..15 the cloud-save UI (`(kind)`), 16 the
/// Eternal Calm copyright. Returns the last routine's `eax`, or the kind.
pub fn unload_special_menu_texture(h: &mut impl Host, this: u32, kind: u32) -> u32 {
    special_menu_texture(h, &UNLOAD_KINDS, this, kind, 0, 0)
}

/// `MenuTextureManager_LoadSpecialMenuTexture(this; kind, a, b)`
/// (thiscall, ret 0xc): kind 0 the event font `(a)`, 1 the Sa menu
/// `(b)`, 2 the Sa sub-menu `(a)`, 3 `0x0067d670`, 4 `0x0067d690`, 5 the
/// water background then `0x0067d650`, 6 the game menu `(b)`, 7 the main
/// menu's wave, 8 the battle font `(a)`, 10 the dream map, 11..15 the
/// cloud-save UI `(kind)`, 16 the Eternal Calm copyright. Returns the
/// last routine's `eax`, or the kind.
pub fn load_special_menu_texture(h: &mut impl Host, this: u32, kind: u32, a: u32, b: u32) -> u32 {
    special_menu_texture(h, &LOAD_KINDS, this, kind, a, b)
}

/// `getSa_MenuTextureName(out, id)` (cdecl).
pub const SA_TEXTURE_NAME: u32 = 0x0077_4ae0;
/// `PhyreScene_loadMenuTexture(scene; name)` (thiscall, ret 4).
pub const LOAD_MENU_TEXTURE: u32 = 0x0066_51d0;
/// `g_PhyreScene`.
pub const PHYRE_SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// Bytes of [`preload_sa_sub_menu`]'s frame: the twelve ids, the
/// 0x100-byte name, the cookie.
pub const PRELOAD_FRAME: u32 = 0x134;

/// `MenuTextureManager_PreLoadSaSubMenu(unused)` (stdcall; `frame` its
/// locals from `ebp - 0x134`): the Sa menu textures 5..16 loaded into the
/// scene, each named into a cleared 0x100-byte buffer
/// (`getSa_MenuTextureName`, `PhyreScene_loadMenuTexture`). Returns the
/// last load's `eax`.
pub fn preload_sa_sub_menu(h: &mut impl Host, frame: u32) -> u32 {
    for k in 0..12u32 {
        h.write(frame + 4 * k, k + 5);
    }
    let name = frame + 0x30;
    let mut eax = 0;
    for k in 0..12u32 {
        h.call_sym(MEMSET, Conv::Cdecl, &[name, 0, 0x100]);
        let id: u32 = h.read(frame + 4 * k);
        h.call_sym(SA_TEXTURE_NAME, Conv::Cdecl, &[name, id]);
        let scene = PHYRE_SCENE.get(h);
        eax = h.call_sym(LOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[name]);
    }
    eax
}

// ---- effects

/// `Distortion_resetRenderTarget_006c24b0(this)` (thiscall).
pub const DISTORTION_RESET: u32 = 0x006c_24b0;
/// `Phyre_PCluster_create(cluster; n, class, a)` (thiscall, ret 0xc).
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
/// The texture class descriptor (`0x00ca37d0`).
pub const TEXTURE_CLASS: u32 = 0x00ca_37d0;
/// `Phyre_PRendering_PTexture2DBase_setDimensions(tex; w, h, format, a)`
/// (thiscall, ret 0x10).
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
/// The texture format (`0x00ca310c`).
pub const DISTORTION_FORMAT: u32 = 0x00ca_310c;
/// `Phyre_PRendering_PTexture2DBase_updateMaxLevel(tex)` (thiscall).
pub const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
/// `0x0064adf0(cluster; texture)` (thiscall, ret 4): the render target
/// over a texture.
pub const CREATE_TARGET: u32 = 0x0064_adf0;
/// The name the target gets (`+0xc`, `0x00b49350`).
pub const DISTORTION_TARGET_NAME: u32 = 0x00b4_9350;
/// `ShaderParams_FindByName(shader; name)` (thiscall, ret 4).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// The distortion texture parameter's name (`0x00b425d8`).
pub const DISTORTION_PARAM: u32 = 0x00b4_25d8;
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670(params; param,
/// &value)` (thiscall, ret 8).
pub const SET_TEXTURE: u32 = 0x0056_e670;
/// Bytes of [`distortion_create_target`]'s frame (the parameter value).
pub const DISTORTION_CREATE_FRAME: u32 = 0x10;

/// `Distortion_createRenderTarget(this)` (thiscall; `frame` its value
/// block): the old target reset; a texture created in the cluster `+0x1c`
/// (`+0x24`; none: 0xd), sized `+0xc` x `+0x10` in the distortion format
/// (a failure's code returned), its flag bit 1 cleared and its levels
/// updated; a render target over it (`+0x28`, named; none: 0xd); with a
/// shader (`+0x2c`) that has the distortion parameter, the parameter set
/// on `(+0x30)+4` to `{0x42, 0, this + 0x44, texture}`; ready byte
/// `+0x7f` set. Returns 0.
pub fn distortion_create_target(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    h.call_sym(DISTORTION_RESET, Conv::Thiscall(this), &[]);
    let cluster: u32 = h.read(this + 0x1c);
    let class = h.rebase(TEXTURE_CLASS);
    let tex = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, class, 0]);
    h.write(this + 0x24, tex);
    if tex == 0 {
        return 0xd;
    }
    let width: u32 = h.read(this + 0xc);
    let height: u32 = h.read(this + 0x10);
    let format = h.rebase(DISTORTION_FORMAT);
    let r = h.call_sym(
        SET_DIMENSIONS,
        Conv::Thiscall(tex),
        &[width, height, format, 0],
    );
    if r != 0 {
        return r;
    }
    let tex: u32 = h.read(this + 0x24);
    let flags: u32 = h.read(tex.wrapping_add(0x14));
    h.write(tex.wrapping_add(0x14), flags & !2);
    h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(tex), &[]);
    let tex: u32 = h.read(this + 0x24);
    let cluster: u32 = h.read(this + 0x1c);
    let target = h.call_sym(CREATE_TARGET, Conv::Thiscall(cluster), &[tex]);
    h.write(this + 0x28, target);
    if target == 0 {
        return 0xd;
    }
    let name = h.rebase(DISTORTION_TARGET_NAME);
    h.write(target.wrapping_add(0xc), name);
    let shader: u32 = h.read(this + 0x2c);
    if shader != 0 {
        let param = h.call_sym(
            FIND_PARAM,
            Conv::Thiscall(shader),
            &[h.rebase(DISTORTION_PARAM)],
        );
        if param != 0 {
            let tex: u32 = h.read(this + 0x24);
            h.write(frame + 0xc, tex);
            h.write(frame + 8, this + 0x44);
            let owner: u32 = h.read(this + 0x30);
            let params: u32 = h.read(owner.wrapping_add(4));
            h.write(frame, 0x42u32);
            h.write(frame + 4, 0u32);
            h.call_sym(SET_TEXTURE, Conv::Thiscall(params), &[param, frame]);
        }
    }
    h.write(this + 0x7f, 1u8);
    0
}

/// `0x0059e520(texture; sampler)` (thiscall, ret 4; named
/// `TrophyManager_Install` in `ffx.sym`): binds the pair.
pub const BIND_TEXTURE: u32 = 0x0059_e520;
/// `memcpy` (`0x00949256`, the import thunk; cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// Bytes of [`blur_set_texture_param`]'s frame (the value).
pub const BLUR_PARAM_FRAME: u32 = 0x10;

/// `BlurEffectEx_setTextureParam_006c44e0(this; sampler, texture)`
/// (thiscall, ret 8; `frame` its value block): the parameter record
/// `+0x54` (none: 0x13) of a texture type (byte `+2`: 0x42, 0xd9..0xdc or
/// 0x12; else 5) gets `{0x42, 0, texture, sampler}` copied into the
/// constant block `(+0x20)+4` at its offset (word `+8`, at least 4; the
/// length word `+0xa`'s low 13 bits; past the block's size word `+0`: 3),
/// after binding the texture to the sampler when both are set (a failure's
/// code returned). Returns 0.
pub fn blur_set_texture_param(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    sampler: u32,
    texture: u32,
) -> u32 {
    let record: u32 = h.read(this + 0x54);
    if record == 0 {
        return 0x13;
    }
    let block: u32 = h.read(h.read::<u32>(this + 0x20).wrapping_add(4));
    let kind: u8 = h.read(record.wrapping_add(2));
    h.write(frame, 0x42u32);
    h.write(frame + 0xc, sampler);
    h.write(frame + 8, texture);
    h.write(frame + 4, 0u32);
    if !matches!(kind, 0x42 | 0xd9..=0xdc | 0x12) {
        return 5;
    }
    if texture != 0 && sampler != 0 {
        let r = h.call_sym(BIND_TEXTURE, Conv::Thiscall(texture), &[sampler]);
        if r != 0 {
            return r;
        }
    }
    let len = u32::from(h.read::<u16>(record.wrapping_add(0xa)) & 0x1fff);
    let offset = u32::from(h.read::<u16>(record.wrapping_add(8)));
    if offset + len > h.read::<u32>(block) || offset < 4 {
        return 3;
    }
    h.call_sym(
        MEMCPY,
        Conv::Cdecl,
        &[offset.wrapping_add(block), frame, len],
    );
    0
}

// ---- sound

/// `FMOD::Event::getChannelGroup(event, &group)` (stdcall thunk).
pub const EVENT_GET_CHANNEL_GROUP: u32 = 0x0094_8cd0;
/// `FMOD::ChannelGroup::getNumChannels(group, &n)`.
pub const GROUP_NUM_CHANNELS: u32 = 0x0094_8d30;
/// `FMOD::ChannelGroup::getChannel(group, i, &channel)`.
pub const GROUP_GET_CHANNEL: u32 = 0x0094_8d36;
/// `FMOD::Channel::getLoopCount(channel, &n)`.
pub const CHANNEL_LOOP_COUNT: u32 = 0x0094_8d2a;
/// `FMOD::Event::getInfo(event, &index, &name, &info)`.
pub const EVENT_GET_INFO: u32 = 0x0094_8cca;
/// `FmodEventInfo_Init(info)` (thiscall).
pub const EVENT_INFO_INIT: u32 = 0x0070_a140;
/// `SfxEvent.cpp`'s name for `ERRCHECK` (`0x00b4fddc`).
pub const SFX_EVENT_FILE: u32 = 0x00b4_fddc;
/// Bytes of [`sfx_event_is_playing`]'s frame: the event info (0x34
/// bytes, its play state at `+0x28`), the loop count, the channel, the
/// channel count, the group.
pub const IS_PLAYING_FRAME: u32 = 0x44;

/// `SfxEvent_isPlaying(this)` (thiscall; `frame` its locals from `ebp -
/// 0x44`): with a count (`+0`, signed) and a live event (`+8`: handle
/// `+0`, byte `+4`), the channels of its group (`getChannelGroup`,
/// `getNumChannels`, `getChannel`, `getLoopCount`, each `ERRCHECK`ed at
/// lines 0x158, 0x15d, 0x160) are searched for an endless loop (-1); one
/// found, the event's info (`FmodEventInfo_Init`, `getInfo(event, 0, 0,
/// &info)`) says whether it plays (`+0x28 == 1`). Returns 1 or 0 in
/// `al`.
pub fn sfx_event_is_playing(h: &mut impl Host, frame: u32, this: u32) -> u8 {
    let file = h.rebase(SFX_EVENT_FILE);
    let (group, n, channel, loops) = (frame + 0x40, frame + 0x3c, frame + 0x38, frame + 0x34);
    if h.read::<u32>(this).cast_signed() < 1 {
        return 0;
    }
    let event: u32 = h.read(this + 8);
    if h.read::<u8>(event.wrapping_add(4)) == 0 {
        return 0;
    }
    let handle: u32 = h.read(event);
    if h.call_sym(EVENT_GET_CHANNEL_GROUP, Conv::Stdcall, &[handle, group]) != 0 {
        return 0;
    }
    let g: u32 = h.read(group);
    let r = h.call_sym(GROUP_NUM_CHANNELS, Conv::Stdcall, &[g, n]);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x158]);
    let mut i = 0u32;
    loop {
        if i.cast_signed() >= h.read::<u32>(n).cast_signed() {
            return 0;
        }
        let g: u32 = h.read(group);
        let r = h.call_sym(GROUP_GET_CHANNEL, Conv::Stdcall, &[g, i, channel]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x15d]);
        let c: u32 = h.read(channel);
        let r = h.call_sym(CHANNEL_LOOP_COUNT, Conv::Stdcall, &[c, loops]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x160]);
        if h.read::<u32>(loops) == u32::MAX {
            break;
        }
        i += 1;
    }
    if h.read::<u32>(this).cast_signed() < 1 {
        return 0;
    }
    let event: u32 = h.read(this + 8);
    if h.read::<u8>(event.wrapping_add(4)) == 0 {
        return 0;
    }
    h.call_sym(EVENT_INFO_INIT, Conv::Thiscall(frame), &[]);
    let handle: u32 = h.read(event);
    h.call_sym(EVENT_GET_INFO, Conv::Stdcall, &[handle, 0, 0, frame]);
    u8::from(h.read::<u32>(frame + 0x28) == 1)
}

// ---- the Atel shape's VFX

/// `createVFXRenderDataBufferAtelShp(a, b, c, e)` (cdecl).
pub const CREATE_ATEL_SHP: u32 = 0x0071_47b0;
/// `graphicVFXGetRenderDataByInstance(lo, hi)` (cdecl).
pub const VFX_RENDER_DATA: u32 = 0x0064_53d0;
/// `graphicVFX2DSetup(lo, hi)` (cdecl).
pub const VFX_2D_SETUP: u32 = 0x0064_4b50;
/// `updateVFXRenderDataAtelShp(a, b, c, d, e)` (cdecl).
pub const UPDATE_ATEL_SHP: u32 = 0x0071_96f0;
/// `graphicVFX2DUpdate(lo, hi)` (cdecl).
pub const VFX_2D_UPDATE: u32 = 0x0064_4bc0;
/// `graphicVFXDestroy(lo, hi)` (cdecl).
pub const VFX_DESTROY: u32 = 0x0064_5280;

/// The six instance ids of `id` (`id + 10000 k`, 64-bit).
fn instances(lo: u32, hi: u32) -> impl Iterator<Item = [u32; 2]> {
    (0..6u32).map(move |k| {
        let (l, carry) = lo.overflowing_add(10_000 * k);
        [l, hi.wrapping_add(u32::from(carry))]
    })
}

/// `InitVFXRenderDataAtelShp(lo, hi, c, d, e)` (cdecl): the buffers
/// created (`createVFXRenderDataBufferAtelShp(lo, hi, c, e)`); each of the
/// six instances `id + 10000 k` with render data gets mode 2 (`(+0x94) +
/// 0x24`) and its 2D setup; all succeeding, the data updated and each
/// instance's 2D update run, else (or when creation fails) each instance
/// destroyed. Returns the last call's `eax`.
pub fn init_vfx_render_data_atel_shp(h: &mut impl Host, args: [u32; 5]) -> u32 {
    let [lo, hi, c, _, e] = args;
    let mut ok = h.call_sym(CREATE_ATEL_SHP, Conv::Cdecl, &[lo, hi, c, e]) != 0;
    if ok {
        let mut all = 1u32;
        for id in instances(lo, hi) {
            let data = h.call_sym(VFX_RENDER_DATA, Conv::Cdecl, &[id[0], id[1]]);
            if data != 0 {
                let r: u32 = h.read(data.wrapping_add(0x94));
                h.write(r.wrapping_add(0x24), 2u32);
                all &= h.call_sym(VFX_2D_SETUP, Conv::Cdecl, &[id[0], id[1]]);
            }
        }
        ok = all != 0;
    }
    let mut eax = 0;
    if ok {
        h.call_sym(UPDATE_ATEL_SHP, Conv::Cdecl, &args);
        for id in instances(lo, hi) {
            eax = h.call_sym(VFX_2D_UPDATE, Conv::Cdecl, &[id[0], id[1]]);
        }
    } else {
        for id in instances(lo, hi) {
            eax = h.call_sym(VFX_DESTROY, Conv::Cdecl, &[id[0], id[1]]);
        }
    }
    eax
}
