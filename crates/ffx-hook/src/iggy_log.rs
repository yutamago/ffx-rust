//! Iggy call log: every call `FFX.exe` makes into `iggy_w32.dll`, with its
//! arguments and results decoded, every AS3 external call Iggy makes into
//! the game with the game's answer, and the harness's screenshots, in the
//! order they happened (`docs/formats/pc-ui.md`, "Iggy calls"). The M5 SWF
//! pilot replays it into Ruffle (`crates/ffx-swf`).
//!
//! With `FFX_IGGY_LOG=<file>` (a Windows path; `tools/run-game.sh` passes it
//! through, `/p` in `WSLENV`; `tools/replays/run-all.sh --iggy-log`) the
//! hook points the game's Iggy imports at typed wrappers. Each wrapper calls
//! the original with the same words and returns its `eax`; it only reads
//! what the call passes and returns, so nothing the game sees changes.
//! Without the variable nothing is patched. Not wrapped: the varargs
//! `IggyGDrawSendWarning` (cdecl, no fixed frame) and Iggy's warning and
//! trace callbacks (handed to Iggy unchanged).
//!
//! The file holds one JSON object per line:
//!
//! ```text
//! {"seq":12,"f":"CallFunctionRS","fr":340,"rl":339,"t":5124,"d":0,"a":["..."],"r":"0",...}
//! {"seq":13,"f":"cb","fr":340,"rl":-1,"t":5124,"d":1,"name":"getLocByStr","args":[...]}
//! {"seq":14,"f":"cbret","of":13,"r":"1"}
//! {"seq":15,"f":"shot","path":"...","label":341}
//! ```
//!
//! `seq` is taken when a call begins (lines are written when it returns, so
//! a call that ran callbacks follows them in the file; sort by `seq`), `fr`
//! is the harness's update count, `rl` the update whose flip the render
//! thread runs next (-1 off the render thread's flips), `t` the thread, `d`
//! the nesting depth on that thread, `a` the argument words and `r` the
//! result word (hex). Decoded extras by function: `s` (strings), `args` /
//! `res` (`IggyDataValue`s: `{"t":type,...}`), `w` (words a pointer
//! argument points at after the call), `size`/`hash` (a movie's bytes).

use std::cell::Cell;
use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use windows_sys::Win32::System::Threading::GetCurrentThreadId;

use crate::log::log;

static ON: AtomicBool = AtomicBool::new(false);
static SEQ: AtomicU64 = AtomicU64::new(0);

struct Out {
    file: std::io::BufWriter<std::fs::File>,
    flushed: Instant,
}

static OUT: Mutex<Option<Out>> = Mutex::new(None);

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Writes one line (flushed at least every 200 ms: `quit` ends the game
/// with `TerminateProcess`).
fn write_line(line: &str, flush: bool) {
    let Ok(mut out) = OUT.lock() else { return };
    if let Some(o) = out.as_mut() {
        let _ = o.file.write_all(line.as_bytes());
        let _ = o.file.write_all(b"\n");
        if flush || o.flushed.elapsed().as_millis() >= 200 {
            let _ = o.file.flush();
            o.flushed = Instant::now();
        }
    }
}

fn json_str(out: &mut String, s: &[u8]) {
    out.push('"');
    for ch in String::from_utf8_lossy(s).chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `len` bytes at `p`, or up to the first 0 when `len` is negative (at
/// most 4096), when readable.
fn bytes_at(p: u32, len: i32) -> Option<Vec<u8>> {
    let room = crate::fmod_log::readable(p);
    if room == 0 {
        return None;
    }
    let max = room.min(4096);
    // SAFETY: `max` readable bytes from `p`.
    let all = unsafe { std::slice::from_raw_parts(p as usize as *const u8, max) };
    let n = match usize::try_from(len) {
        Ok(n) => n.min(max),
        Err(_) => all.iter().position(|&b| b == 0).unwrap_or(max),
    };
    Some(all[..n].to_vec())
}

fn words_at(p: u32, n: usize) -> Option<Vec<u32>> {
    if crate::fmod_log::readable(p) < 4 * n {
        return None;
    }
    // SAFETY: `4n` readable bytes.
    Some(
        (0..n)
            .map(|i| unsafe { (p as usize as *const u32).add(i).read_unaligned() })
            .collect(),
    )
}

fn push_words(out: &mut String, key: &str, w: &[u32]) {
    let _ = write!(out, ",\"{key}\":[");
    for (i, x) in w.iter().enumerate() {
        let _ = write!(out, "{}\"{x:x}\"", if i > 0 { "," } else { "" });
    }
    out.push(']');
}

/// One `IggyDataValue` (16 bytes: type, pad, value at +8).
fn push_value(out: &mut String, v: u32) {
    let Some(w) = words_at(v, 4) else {
        out.push_str("null");
        return;
    };
    let _ = write!(out, "{{\"t\":{}", w[0]);
    match w[0] {
        3 => {
            let _ = write!(out, ",\"b\":{}", w[2] & 0xff != 0);
        }
        4 => {
            let n = f64::from_bits(u64::from(w[2]) | (u64::from(w[3]) << 32));
            if n.is_finite() {
                let _ = write!(out, ",\"n\":{n:?}");
            } else {
                let _ = write!(out, ",\"nbits\":\"{:x}\"", n.to_bits());
            }
        }
        5 => {
            if let Some(s) = bytes_at(w[2], w[3].cast_signed()) {
                out.push_str(",\"s\":");
                json_str(out, &s);
            }
        }
        6 => {
            // UTF-16: pointer, length in units
            let n = w[3] as usize;
            if let Some(b) = bytes_at(w[2], i32::try_from(2 * n.min(2048)).unwrap_or(0)) {
                let units: Vec<u16> = b
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&c| u16::from_le_bytes(c))
                    .collect();
                out.push_str(",\"s\":");
                json_str(out, String::from_utf16_lossy(&units).as_bytes());
            }
        }
        _ => {}
    }
    push_words(out, "w", &w);
    out.push('}');
}

fn push_values(out: &mut String, key: &str, at: u32, n: u32) {
    let _ = write!(out, ",\"{key}\":[");
    for i in 0..n.min(64) {
        if i > 0 {
            out.push(',');
        }
        push_value(out, at.wrapping_add(16 * i));
    }
    out.push(']');
}

fn push_string(out: &mut String, key: &str, p: u32, len: i32) {
    if let Some(s) = bytes_at(p, len) {
        let _ = write!(out, ",\"{key}\":");
        json_str(out, &s);
    }
}

/// The start of a line: sequence, function, frame, labels, thread, depth.
fn head(seq: u64, f: &str) -> String {
    let rl = crate::harness::render_label().map_or(-1, i64::from);
    // SAFETY: plain call.
    let tid = unsafe { GetCurrentThreadId() };
    let d = DEPTH.with(Cell::get);
    format!(
        "{{\"seq\":{seq},\"f\":\"{f}\",\"fr\":{},\"rl\":{rl},\"t\":{tid},\"d\":{d}",
        crate::harness::frame()
    )
}

/// What a wrapper read before the call (pointed-at data the callee may
/// change).
#[derive(Default)]
struct Pre {
    extra: String,
}

/// Decodes arguments before the call.
fn pre(f: &str, a: &[u32]) -> Pre {
    let mut extra = String::new();
    match f {
        "CreateFromMemory" => {
            let size = a[1];
            let _ = write!(extra, ",\"size\":{size}");
            if let Some(b) = bytes_at(a[0], i32::try_from(size.min(16)).unwrap_or(0)) {
                push_words(
                    &mut extra,
                    "magic",
                    &[u32::from_le_bytes([b[0], b[1], b[2], b[3]])],
                );
            }
            if crate::fmod_log::readable(a[0]) >= size as usize {
                // SAFETY: `size` readable bytes.
                let all = unsafe {
                    std::slice::from_raw_parts(a[0] as usize as *const u8, size as usize)
                };
                let mut h: u64 = 0xcbf2_9ce4_8422_2325;
                for &b in all {
                    h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
                }
                let _ = write!(extra, ",\"hash\":\"{h:016x}\"");
            }
        }
        "CreateFromFileAndPlay" => push_string(&mut extra, "s", a[0], -1),
        "CreateFastNameUTF8" => push_string(&mut extra, "s", a[1], a[2].cast_signed()),
        "CallFunctionRS" => push_values(&mut extra, "args", a[4], a[3]),
        "DispatchEventRS" => {
            if let Some(w) = words_at(a[1], 8) {
                push_words(&mut extra, "ev", &w);
            }
        }
        "ValueSetStringUTF8RS" => {
            push_string(&mut extra, "s", a[3], a[4].cast_signed());
            push_string(&mut extra, "sub", a[2], -1);
            if let Some(w) = words_at(a[0], 8) {
                push_words(&mut extra, "path", &w);
            }
        }
        "ValueGetS32RS" | "ValueGetStringUTF8RS" => {
            push_string(&mut extra, "sub", a[2], -1);
            if let Some(w) = words_at(a[0], 8) {
                push_words(&mut extra, "path", &w);
            }
        }
        "ValuePathMakeArrayRef" => {
            if let Some(w) = words_at(a[1], 8) {
                push_words(&mut extra, "parent", &w);
            }
        }
        _ => {}
    }
    Pre { extra }
}

/// Decodes results after the call.
fn post(f: &str, a: &[u32], r: u32, out: &mut String) {
    match f {
        "CallFunctionRS" if a[1] != 0 => {
            out.push_str(",\"res\":");
            push_value(out, a[1]);
        }
        "DispatchEventRS" => {
            if let Some(w) = words_at(a[2], 4) {
                push_words(out, "evres", &w);
            }
        }
        "MakeEventNone" | "MakeEventMouseMove" | "MakeEventMouseButton" | "MakeEventMouseWheel" => {
            if let Some(w) = words_at(a[0], 8) {
                push_words(out, "ev", &w);
            }
        }
        "ValueGetS32RS" => {
            if let Some(w) = words_at(a[3], 1) {
                let _ = write!(out, ",\"out\":{}", w[0].cast_signed());
            }
        }
        "ValueGetStringUTF8RS" => {
            let len = words_at(a[5], 1).map_or(-1, |w| w[0].cast_signed());
            push_string(out, "out", a[4], len);
        }
        "ValuePathFromRef" | "ValuePathMakeArrayRef" | "ValuePathSetArrayIndex" => {
            if let Some(w) = words_at(a[0], 8) {
                push_words(out, "path", &w);
            }
        }
        "CallbackResultPath" => {
            if let Some(w) = words_at(r, 8) {
                push_words(out, "path", &w);
            }
        }
        "Properties" => {
            if let Some(w) = words_at(r, 24) {
                push_words(out, "props", &w);
            }
        }
        "GetBackgroundColor" => {
            if let Some(w) = words_at(a[1], 3) {
                let c: Vec<String> = w
                    .iter()
                    .map(|&x| format!("{:?}", f32::from_bits(x)))
                    .collect();
                let _ = write!(out, ",\"rgb\":[{}]", c.join(","));
            }
        }
        _ => {}
    }
}

fn finish(seq: u64, f: &str, a: &[u32], r: u32, p: &Pre) {
    let mut line = head(seq, f);
    push_words(&mut line, "a", a);
    let _ = write!(line, ",\"r\":\"{r:x}\"");
    line.push_str(&p.extra);
    post(f, a, r, &mut line);
    line.push('}');
    write_line(&line, false);
}

/// Runs one wrapped call: the original with the same words, its `eax`.
fn wrapped(index: usize, a: &[u32], call: impl FnOnce(usize) -> u32) -> u32 {
    let f = WRAPPERS[index].0;
    let seq = SEQ.fetch_add(1, Ordering::AcqRel);
    let p = pre(f, a);
    DEPTH.with(|d| d.set(d.get() + 1));
    let r = call(ORIGINAL[index].load(Ordering::Acquire));
    DEPTH.with(|d| d.set(d.get() - 1));
    finish(seq, f, a, r, &p);
    r
}

macro_rules! wrappers {
    ($( $index:literal $name:literal $import:literal $fn:ident ($($arg:ident),*); )*) => {
        $(
            unsafe extern "system" fn $fn($($arg: u32),*) -> u32 {
                wrapped($index, &[$($arg),*], |f| {
                    // SAFETY: the original import, with the words the game
                    // passed (its decorated name gives their count).
                    unsafe {
                        let f: unsafe extern "system" fn($(wrappers!(@word $arg)),*) -> u32 =
                            std::mem::transmute(f);
                        f($($arg),*)
                    }
                })
            }
        )*
        /// (log name, decorated import).
        static WRAPPERS: &[(&str, &str)] = &[
            $( ($name, $import), )*
        ];
        /// The wrapper of entry `i` of [`WRAPPERS`].
        fn wrapper(i: usize) -> usize {
            match i {
                $( $index => $fn as *const () as usize, )*
                _ => 0,
            }
        }
        static ORIGINAL: [AtomicUsize; 0 $(+ { let _ = $index; 1 })*] =
            [const { AtomicUsize::new(0) }; 0 $(+ { let _ = $index; 1 })*];
    };
    (@word $arg:ident) => { u32 };
}

wrappers! {
    0 "CreateFromMemory" "_IggyPlayerCreateFromMemory@12" w_create_from_memory(a0, a1, a2);
    1 "Properties" "_IggyPlayerProperties@4" w_properties(a0);
    2 "InitializeAndTickRS" "_IggyPlayerInitializeAndTickRS@4" w_initialize_and_tick(a0);
    3 "ReadyToTick" "_IggyPlayerReadyToTick@4" w_ready_to_tick(a0);
    4 "TickRS" "_IggyPlayerTickRS@4" w_tick(a0);
    5 "Play" "_IggyPlayerPlay@4" w_play(a0);
    6 "GotoFrameRS" "_IggyPlayerGotoFrameRS@12" w_goto_frame(a0, a1, a2);
    7 "GetBackgroundColor" "_IggyPlayerGetBackgroundColor@8" w_background(a0, a1);
    8 "SetDisplaySize" "_IggyPlayerSetDisplaySize@12" w_display_size(a0, a1, a2);
    9 "Draw" "_IggyPlayerDraw@4" w_draw(a0);
    10 "Init" "_IggyInit@4" w_init(a0);
    11 "Shutdown" "_IggyShutdown@0" w_shutdown();
    12 "SetWarningCallback" "_IggySetWarningCallback@8" w_warning_callback(a0, a1);
    13 "SetTraceCallbackUTF8" "_IggySetTraceCallbackUTF8@8" w_trace_callback(a0, a1);
    14 "Destroy" "_IggyPlayerDestroy@4" w_destroy(a0);
    15 "CallbackResultPath" "_IggyPlayerCallbackResultPath@4" w_result_path(a0);
    16 "ValueSetStringUTF8RS" "_IggyValueSetStringUTF8RS@20" w_set_string(a0, a1, a2, a3, a4);
    17 "ValueRefFree" "_IggyValueRefFree@8" w_ref_free(a0, a1);
    18 "ValueRefCreateArray" "_IggyValueRefCreateArray@8" w_ref_create_array(a0, a1);
    19 "ValueRefFromTempRef" "_IggyValueRefFromTempRef@12" w_ref_from_temp(a0, a1, a2);
    20 "ValuePathFromRef" "_IggyValuePathFromRef@12" w_path_from_ref(a0, a1, a2);
    21 "ValuePathMakeArrayRef" "_IggyValuePathMakeArrayRef@12" w_path_array_ref(a0, a1, a2);
    22 "ValuePathSetArrayIndex" "_IggyValuePathSetArrayIndex@8" w_path_set_index(a0, a1);
    23 "ValueGetS32RS" "_IggyValueGetS32RS@16" w_get_s32(a0, a1, a2, a3);
    24 "SetAS3ExternalFunctionCallbackUTF8" "_IggySetAS3ExternalFunctionCallbackUTF8@8" w_set_as3_callback_unused(a0, a1);
    25 "MakeEventNone" "_IggyMakeEventNone@4" w_event_none(a0);
    26 "MakeEventMouseMove" "_IggyMakeEventMouseMove@12" w_event_move(a0, a1, a2);
    27 "MakeEventMouseButton" "_IggyMakeEventMouseButton@8" w_event_button(a0, a1);
    28 "MakeEventMouseWheel" "_IggyMakeEventMouseWheel@8" w_event_wheel(a0, a1);
    29 "CreateFastNameUTF8" "_IggyPlayerCreateFastNameUTF8@12" w_fast_name(a0, a1, a2);
    30 "CallFunctionRS" "_IggyPlayerCallFunctionRS@20" w_call_function(a0, a1, a2, a3, a4);
    31 "ValueGetStringUTF8RS" "_IggyValueGetStringUTF8RS@24" w_get_string(a0, a1, a2, a3, a4, a5);
    32 "DispatchEventRS" "_IggyPlayerDispatchEventRS@12" w_dispatch_event(a0, a1, a2);
    33 "AudioUseDirectSound" "_IggyAudioUseDirectSound@0" w_audio_dsound();
    34 "AudioInstallMP3Decoder" "_IggyAudioInstallMP3Decoder@0" w_audio_mp3();
    35 "SetGDraw" "_IggySetGDraw@4" w_set_gdraw(a0);
    36 "GDrawMallocAnnotated" "_IggyGDrawMallocAnnotated@12" w_gdraw_malloc(a0, a1, a2);
    37 "GDrawFree" "_IggyGDrawFree@4" w_gdraw_free(a0);
    38 "DiscardVertexBufferCallback" "_IggyDiscardVertexBufferCallback@8" w_discard_vb(a0, a1);
    39 "CreateFromFileAndPlay" "_IggyPlayerCreateFromFileAndPlay@8" w_create_from_file(a0, a1);
}

/// Index of `SetAS3ExternalFunctionCallbackUTF8` in [`WRAPPERS`].
const SET_AS3_CALLBACK: usize = 24;

/// The game's AS3 external-function callback
/// (`IggyMenuManager_callback_00a260f0`, stdcall(user, player, call)).
static GAME_CALLBACK: AtomicUsize = AtomicUsize::new(0);

/// Replaces `IggySetAS3ExternalFunctionCallbackUTF8`: Iggy gets
/// [`as3_callback`], which calls the game's.
unsafe extern "system" fn w_set_as3_callback(callback: u32, user: u32) -> u32 {
    GAME_CALLBACK.store(callback as usize, Ordering::Release);
    let mine = if callback == 0 {
        0
    } else {
        as3_callback as *const () as usize as u32
    };
    wrapped(SET_AS3_CALLBACK, &[callback, user], |f| {
        // SAFETY: the original import with the same user word.
        unsafe {
            let f: unsafe extern "system" fn(u32, u32) -> u32 = std::mem::transmute(f);
            f(mine, user)
        }
    })
}

/// An AS3 `ExternalInterface.call` reaching the game: logs the call
/// (`IggyExternalFunctionCallUTF8`: name pointer and length, argument count
/// at +8, `IggyDataValue`s from +0x10) and the game's answer.
unsafe extern "system" fn as3_callback(user: u32, player: u32, call: u32) -> u32 {
    let seq = SEQ.fetch_add(1, Ordering::AcqRel);
    let mut line = head(seq, "cb");
    let _ = write!(line, ",\"player\":\"{player:x}\"");
    if let Some(w) = words_at(call, 4) {
        push_string(&mut line, "name", w[0], w[1].cast_signed());
        push_values(&mut line, "args", call.wrapping_add(16), w[2]);
    }
    line.push('}');
    write_line(&line, false);
    DEPTH.with(|d| d.set(d.get() + 1));
    // SAFETY: the game's callback, as Iggy would have called it.
    let r = unsafe {
        let f: unsafe extern "system" fn(u32, u32, u32) -> u32 =
            std::mem::transmute(GAME_CALLBACK.load(Ordering::Acquire));
        f(user, player, call)
    };
    DEPTH.with(|d| d.set(d.get() - 1));
    let done = SEQ.fetch_add(1, Ordering::AcqRel);
    write_line(
        &format!("{{\"seq\":{done},\"f\":\"cbret\",\"of\":{seq},\"r\":\"{r:x}\"}}"),
        false,
    );
    r
}

/// `GDraw`'s antialiasing ramp texture (`gdraw_createRowTexture_00a3a9f0`,
/// called by Iggy through `GDraw`'s table): `w` RGBA texels.
pub fn note_row_texture(w: u32, texels: u32) {
    if !ON.load(Ordering::Acquire) {
        return;
    }
    let seq = SEQ.fetch_add(1, Ordering::AcqRel);
    let mut line = head(seq, "gdrawRowTexture");
    let _ = write!(line, ",\"w\":{w}");
    if let Some(t) = words_at(texels, (w as usize).min(1024)) {
        push_words(&mut line, "texels", &t);
    }
    line.push('}');
    write_line(&line, true);
}

/// A texture Iggy asks `GDraw` for (`gdraw_makeTextureBegin_00a3a070`):
/// size, format (0 RGBA, else R8), flags (bit 0 mipmaps); `info` is the
/// staging block whose `+0x18` record the draws bind.
pub fn note_texture(owner: u32, w: u32, h: u32, format: u32, flags: u32, info: u32) {
    if !ON.load(Ordering::Acquire) {
        return;
    }
    let seq = SEQ.fetch_add(1, Ordering::AcqRel);
    let mut line = head(seq, "gdrawTexture");
    let _ = write!(
        line,
        ",\"owner\":\"{owner:x}\",\"w\":{w},\"h\":{h},\"format\":{format},\"flags\":{flags},\"info\":\"{info:x}\"}}"
    );
    write_line(&line, false);
}

/// Draw states logged so far ([`note_render_state`] keeps the first ones).
static RENDER_STATES: AtomicU64 = AtomicU64::new(0);

/// One draw's state (`gdraw_setRenderState_00a3c6c0(format, r, _, colour)`):
/// the vertex format and `r`'s first 0x20 bytes (`+8` .. `+0x11`: the
/// shader, address mode, filter and blend bytes; `+0x1c` the texture); the
/// first 4000 draws.
pub fn note_render_state(format: u32, r: u32) {
    if !ON.load(Ordering::Acquire) || RENDER_STATES.fetch_add(1, Ordering::AcqRel) >= 4000 {
        return;
    }
    let seq = SEQ.fetch_add(1, Ordering::AcqRel);
    let mut line = head(seq, "gdrawState");
    let _ = write!(line, ",\"format\":{format}");
    if let Some(w) = words_at(r, 8) {
        push_words(&mut line, "r", &w);
        // the bound texture's record: +0xc w, +0x10 h (gdraw_makeTextureBegin)
        if w[7] != 0
            && let Some(t) = words_at(w[7], 5)
        {
            push_words(&mut line, "tex", &t);
        }
    }
    line.push('}');
    write_line(&line, false);
}

/// Vertex buffers logged so far ([`note_vertex_buffer`] keeps the first
/// few).
static VERTEX_BUFFERS: AtomicU64 = AtomicU64::new(0);

/// A shape's vertices as Iggy hands them to `GDraw`
/// (`gdraw_makeVertexBufferEnd_00a3a5f0`'s staging block `info + 0`, size
/// `info + 8`): the first 64 buffers, up to 256 words each.
pub fn note_vertex_buffer(info: u32) {
    if !ON.load(Ordering::Acquire) || VERTEX_BUFFERS.fetch_add(1, Ordering::AcqRel) >= 64 {
        return;
    }
    let Some(w) = words_at(info, 4) else { return };
    let seq = SEQ.fetch_add(1, Ordering::AcqRel);
    let mut line = head(seq, "gdrawVertexBuffer");
    let _ = write!(line, ",\"bytes\":{},\"ibytes\":{}", w[2], w[3]);
    if let Some(v) = words_at(w[0], (w[2] as usize / 4).min(256)) {
        push_words(&mut line, "vertices", &v);
    }
    line.push('}');
    write_line(&line, false);
}

/// A harness screenshot was saved (render thread, inside `Present`, after
/// the Iggy draws of the image it shows).
pub fn note_shot(path: &str, label: Option<u32>) {
    if !ON.load(Ordering::Acquire) {
        return;
    }
    let seq = SEQ.fetch_add(1, Ordering::AcqRel);
    let mut line = format!("{{\"seq\":{seq},\"f\":\"shot\",\"path\":");
    json_str(&mut line, path.as_bytes());
    let _ = write!(line, ",\"label\":{}}}", label.map_or(-1, i64::from));
    write_line(&line, true);
}

/// Installs the wrappers when `FFX_IGGY_LOG` is set.
///
/// # Safety
/// `base` must be the main module, at start-up (no Iggy player yet); after
/// the harness, whose tick-check wrapper then runs inside this one.
pub unsafe fn install(base: usize) {
    let Ok(path) = std::env::var("FFX_IGGY_LOG") else {
        return;
    };
    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            log(&format!("iggy log: cannot create {path}: {e}"));
            return;
        }
    };
    let mut file = std::io::BufWriter::with_capacity(1 << 20, file);
    let _ = writeln!(file, "{{\"iggylog\":1}}");
    if let Ok(mut out) = OUT.lock() {
        *out = Some(Out {
            file,
            flushed: Instant::now(),
        });
    }
    ON.store(true, Ordering::Release);
    let mut patched = 0;
    for (i, &(_, import)) in WRAPPERS.iter().enumerate() {
        let wrapper = if i == SET_AS3_CALLBACK {
            w_set_as3_callback as *const () as usize
        } else {
            wrapper(i)
        };
        // SAFETY: forwarded from the caller; no thread calls Iggy yet.
        if let Some(original) = unsafe { crate::iat::patch(base, "iggy_w32.dll", import, wrapper) }
        {
            ORIGINAL[i].store(original, Ordering::Release);
            patched += 1;
        }
    }
    log(&format!(
        "iggy log: {patched} of {} Iggy imports logged to {path}",
        WRAPPERS.len()
    ));
}
