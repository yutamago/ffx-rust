//! Iggy's 41 imports (`iggy_w32.dll`, the Flash screens) on `ffx-swf`
//! (docs/host.md, "Iggy"; the calls: docs/formats/pc-ui.md, "Iggy calls").
//!
//! The layer keeps what Iggy would know: each player (the `.swf` the game
//! read, its properties block, its tick clock), the fast names, the AS3
//! callback. With the `swf` feature a Ruffle player plays each movie
//! (`ffx_swf::stage::Stage`, on a thread of its own: Ruffle's players are
//! not `Send`, and the game calls Iggy from the main and the render
//! thread); without it the players are null (no image, calls do nothing),
//! which is enough for the boot: the title is not Flash.
//!
//! | Import | Here |
//! |---|---|
//! | `IggyPlayerCreateFromMemory(data, size, config)` | a player: a 0x100-byte block on the heap (its handle), the movie's bytes kept |
//! | `IggyPlayerProperties(p)` | the block's `+0x40`: width, height, frame rate (`f32`, twice) |
//! | `IggyPlayerCreateFastNameUTF8(p, s, len)` | a non-zero name id (`IggyMenu_callIggyAS3External` waits for one) |
//! | `IggyPlayerCallFunctionRS(p, result, name, argc, args)` | the AS3 function `name` with the `IggyDataValue`s; `result` undefined |
//! | `IggyPlayerReadyToTick(p)` / `TickRS` | the player's accumulator on the virtual clock: ready while it holds a frame period |
//! | `IggyPlayerSetDisplaySize`, `DispatchEventRS` (mouse moves), `Draw` | the Ruffle stage's size, events, image |
//! | `IggySetAS3ExternalFunctionCallbackUTF8(cb, user)` | an AS3 `ExternalInterface.call` runs `cb(user, player, call)` (stdcall); the answer is what the game sets with `IggyPlayerCallbackResultPath` + `IggyValueSetStringUTF8RS` |
//! | the rest | 0 |

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use ffx_bridge::Mem as _;

use crate::exec::Exec;
use crate::imports::{ImportFn, a};
use crate::table::{Args, Conv, Ret};

/// A player's block (its handle): 0x100 bytes, properties at `+0x40`, the
/// callback result path at `+0x80`.
const BLOCK: u32 = 0x100;
const PROPS: u32 = 0x40;
const RESULT_PATH: u32 = 0x80;

/// A value passed to or from AS3.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `undefined` (and every type the layer doesn't read).
    Undefined,
    /// Type 3.
    Bool(bool),
    /// Type 4.
    Number(f64),
    /// Types 5 (UTF-8) and 6 (UTF-16).
    String(String),
}

/// One player.
#[derive(Debug, Clone)]
pub struct Player {
    /// The movie's bytes.
    pub swf: Vec<u8>,
    /// Stage size and frame rate (from the SWF header).
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    /// The tick clock: the game time last seen and the time not ticked.
    last: Option<f64>,
    owed: f64,
    /// Ticks and draws so far.
    pub ticks: u64,
    pub draws: u64,
}

/// The layer's state.
#[derive(Debug, Default)]
pub struct Iggy {
    players: Mutex<HashMap<u32, Player>>,
    names: Mutex<Vec<String>>,
    callback: AtomicU32,
    user: AtomicU32,
    /// The answer the game set during an AS3 callback.
    answer: Mutex<Option<String>>,
    /// Calls the game made, by function (for the report).
    pub calls: Mutex<std::collections::BTreeMap<String, u64>>,
    #[cfg(feature = "swf")]
    swf: swf::Slot,
}

impl Iggy {
    /// The players: handle, movie size, ticks, draws.
    #[must_use]
    pub fn players(&self) -> Vec<(u32, u32, u64, u64)> {
        self.players.lock().map_or_else(
            |_| Vec::new(),
            |p| {
                let mut v: Vec<_> = p
                    .iter()
                    .map(|(&h, p)| (h, u32::try_from(p.swf.len()).unwrap_or(0), p.ticks, p.draws))
                    .collect();
                v.sort_unstable();
                v
            },
        )
    }

    fn name(&self, id: u32) -> Option<String> {
        self.names
            .lock()
            .ok()
            .and_then(|n| n.get(id.checked_sub(1)? as usize).cloned())
    }

    fn count(&self, f: &str) {
        if let Ok(mut c) = self.calls.lock() {
            *c.entry(f.to_owned()).or_default() += 1;
        }
    }
}

fn iggy(e: &Exec) -> &Iggy {
    &e.shared().iggy
}

/// A SWF header's stage size (pixels) and frame rate; the body is
/// compressed (`CWS`, zlib) in the game's movies.
fn swf_header(data: &[u8]) -> Option<(u32, u32, f32)> {
    let body: Vec<u8> = match data.get(..3)? {
        b"FWS" => data.get(8..)?.to_vec(),
        b"CWS" => {
            let len = u32::from_le_bytes(data.get(4..8)?.try_into().ok()?) as usize;
            let mut out = vec![0u8; len.saturating_sub(8).min(64)];
            // only the head is needed: a short output buffer ends early
            let mut st = miniz_oxide::inflate::stream::InflateState::new_boxed(
                miniz_oxide::DataFormat::Zlib,
            );
            let r = miniz_oxide::inflate::stream::inflate(
                &mut st,
                data.get(8..)?,
                &mut out,
                miniz_oxide::MZFlush::None,
            );
            out.truncate(r.bytes_written);
            out
        }
        _ => return None,
    };
    // RECT: 5 bits of field size, then xmin, xmax, ymin, ymax (twips)
    let bits = |at: usize, n: usize| -> Option<u32> {
        let mut v = 0u32;
        for i in at..at + n {
            let b = body.get(i / 8)?;
            v = (v << 1) | u32::from((b >> (7 - i % 8)) & 1);
        }
        Some(v)
    };
    let n = bits(0, 5)? as usize;
    let xmax = bits(5 + n, n)?;
    let ymax = bits(5 + 3 * n, n)?;
    let rect = (5 + 4 * n).div_ceil(8);
    let rate = u16::from_le_bytes([*body.get(rect)?, *body.get(rect + 1)?]);
    Some((xmax / 20, ymax / 20, f32::from(rate) / 256.0))
}

fn create_from_memory(e: &mut Exec, args: &Args) -> Ret {
    iggy(e).count("CreateFromMemory");
    let (data, size) = (a(args, 0), a(args, 1));
    let mut swf = vec![0u8; size as usize];
    e.mem().read_bytes(data, &mut swf);
    let (width, height, fps) = swf_header(&swf).unwrap_or((1280, 720, 30.0));
    let p = e.alloc(BLOCK);
    if p == 0 {
        return Ret::eax(0);
    }
    let mut mem = e.mem();
    mem.write_bytes(p, &[0u8; BLOCK as usize]);
    mem.write(p + PROPS, width);
    mem.write(p + PROPS + 4, height);
    mem.write(p + PROPS + 8, fps);
    mem.write(p + PROPS + 12, fps);
    #[cfg(feature = "swf")]
    iggy(e).swf.create(p, &swf, (width, height));
    let player = Player {
        swf,
        width,
        height,
        fps,
        last: None,
        owed: 0.0,
        ticks: 0,
        draws: 0,
    };
    if let Ok(mut ps) = iggy(e).players.lock() {
        ps.insert(p, player);
    }
    crate::log::note(&format!(
        "iggy: player {p:#x}: a {size}-byte movie, {width}x{height} at {fps} fps"
    ));
    Ret::eax(p)
}

fn destroy(e: &mut Exec, args: &Args) -> Ret {
    iggy(e).count("Destroy");
    let p = a(args, 0);
    if let Ok(mut ps) = iggy(e).players.lock() {
        ps.remove(&p);
    }
    #[cfg(feature = "swf")]
    iggy(e).swf.destroy(p);
    Ret::VOID
}

fn properties(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    let known = iggy(e).players.lock().is_ok_and(|ps| ps.contains_key(&p));
    Ret::eax(if known { p + PROPS } else { 0 })
}

/// `IggyPlayerCreateFastNameUTF8(player, s, len)`: the name's id (from 1).
fn fast_name(e: &mut Exec, args: &Args) -> Ret {
    let (s, len) = (a(args, 1), a(args, 2));
    let mut b = vec![0u8; len.min(4096) as usize];
    e.mem().read_bytes(s, &mut b);
    let name = String::from_utf8_lossy(&b).into_owned();
    let Ok(mut names) = iggy(e).names.lock() else {
        return Ret::eax(0);
    };
    let i = names.iter().position(|n| *n == name).unwrap_or_else(|| {
        names.push(name);
        names.len() - 1
    });
    Ret::eax(u32::try_from(i + 1).unwrap_or(0))
}

/// The `IggyDataValue`s at `at` (16 bytes each).
fn read_values(e: &Exec, at: u32, count: u32) -> Vec<Value> {
    let mem = e.mem();
    (0..count.min(64))
        .map(|i| {
            let value = at + 16 * i;
            let (text, len) = (mem.read::<u32>(value + 8), mem.read::<u32>(value + 12));
            match mem.read::<u32>(value) {
                3 => Value::Bool(mem.read::<u8>(value + 8) != 0),
                4 => Value::Number(mem.read::<f64>(value + 8)),
                5 => {
                    let mut bytes = vec![0u8; len.min(1 << 16) as usize];
                    mem.read_bytes(text, &mut bytes);
                    Value::String(String::from_utf8_lossy(&bytes).into_owned())
                }
                6 => {
                    let units: Vec<u16> = (0..len.min(1 << 15))
                        .map(|k| mem.read(text + 2 * k))
                        .collect();
                    Value::String(String::from_utf16_lossy(&units))
                }
                _ => Value::Undefined,
            }
        })
        .collect()
}

/// `IggyPlayerCallFunctionRS(player, result, name, argc, args)`.
fn call_function(e: &mut Exec, args: &Args) -> Ret {
    iggy(e).count("CallFunctionRS");
    // the result (word 1) is left alone: the game never reads it, and its
    // buffers are smaller than an IggyDataValue
    let (p, id, count, at) = (a(args, 0), a(args, 2), a(args, 3), a(args, 4));
    let name = iggy(e).name(id).unwrap_or_default();
    let values = read_values(e, at, count);
    crate::log::note(&format!("iggy: {p:#x} {name}{values:?}"));
    #[cfg(feature = "swf")]
    swf::call(e, p, &name, values);
    #[cfg(not(feature = "swf"))]
    let _ = (p, values);
    Ret::eax(0)
}

/// `IggyPlayerReadyToTick(player)`: whether the player owes a frame by the
/// virtual clock (the update's game time: every check of one draw sees
/// the same time, as on the harness's virtual clock).
fn ready_to_tick(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    let now = e.shared().clock.now();
    let Ok(mut ps) = iggy(e).players.lock() else {
        return Ret::eax(0);
    };
    let Some(pl) = ps.get_mut(&p) else {
        return Ret::eax(0);
    };
    let last = pl.last.replace(now).unwrap_or(now);
    pl.owed += now - last;
    let period = 1.0 / f64::from(pl.fps.max(1.0));
    Ret::eax(u32::from(pl.owed >= period))
}

fn tick(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    let ticked = iggy(e).players.lock().is_ok_and(|mut ps| {
        ps.get_mut(&p).is_some_and(|pl| {
            let period = 1.0 / f64::from(pl.fps.max(1.0));
            pl.owed = (pl.owed - period).max(0.0);
            pl.ticks += 1;
            true
        })
    });
    if ticked {
        #[cfg(feature = "swf")]
        swf::tick(e, p);
    }
    Ret::VOID
}

fn initialize_and_tick(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    if let Ok(mut ps) = iggy(e).players.lock()
        && let Some(pl) = ps.get_mut(&p)
    {
        pl.ticks += 1;
    }
    #[cfg(feature = "swf")]
    swf::tick(e, p);
    Ret::VOID
}

fn set_display_size(e: &mut Exec, args: &Args) -> Ret {
    #[cfg(feature = "swf")]
    iggy(e).swf.set_size(a(args, 0), (a(args, 1), a(args, 2)));
    #[cfg(not(feature = "swf"))]
    let _ = (e, args);
    Ret::VOID
}

fn draw(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    if let Ok(mut ps) = iggy(e).players.lock()
        && let Some(pl) = ps.get_mut(&p)
    {
        pl.draws += 1;
    }
    #[cfg(feature = "swf")]
    swf::draw(e, p);
    Ret::VOID
}

/// `IggyMakeEventMouseMove(ev, x, y)`: the layer's own event layout
/// (24 bytes, as `IggyMenuManager_dispatchMouse_00a26ce0`'s; type 7 at
/// `+0`, x and y at `+8`, `+0xc`), read back by
/// `DispatchEventRS`.
fn event_mouse_move(e: &mut Exec, args: &Args) -> Ret {
    let ev = a(args, 0);
    let mut mem = e.mem();
    mem.write_bytes(ev, &[0u8; 24]);
    mem.write(ev, 7u32);
    mem.write(ev + 8, a(args, 1));
    mem.write(ev + 12, a(args, 2));
    Ret::VOID
}

fn event_none(e: &mut Exec, args: &Args) -> Ret {
    e.mem().write_bytes(a(args, 0), &[0u8; 24]);
    Ret::VOID
}

fn dispatch_event(e: &mut Exec, args: &Args) -> Ret {
    let (p, ev, out) = (a(args, 0), a(args, 1), a(args, 2));
    let mem = e.mem();
    if mem.read::<u32>(ev) == 7 {
        #[cfg(feature = "swf")]
        iggy(e).swf.mouse_move(
            p,
            mem.read::<u32>(ev + 8).cast_signed(),
            mem.read::<u32>(ev + 12).cast_signed(),
        );
        #[cfg(not(feature = "swf"))]
        let _ = p;
    }
    if out != 0 {
        e.mem().write_bytes(out, &[0u8; 12]);
    }
    Ret::VOID
}

fn set_as3_callback(e: &mut Exec, args: &Args) -> Ret {
    iggy(e).callback.store(a(args, 0), Ordering::Release);
    iggy(e).user.store(a(args, 1), Ordering::Release);
    Ret::VOID
}

fn callback_result_path(_: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    Ret::eax(if p == 0 { 0 } else { p + RESULT_PATH })
}

/// `IggyValueSetStringUTF8RS(path, sub, sub2, s, len)`: on a player's
/// callback result path, the answer to the AS3 call in progress.
fn value_set_string(e: &mut Exec, args: &Args) -> Ret {
    let (s, len) = (a(args, 3), a(args, 4));
    let mut b = vec![0u8; len.min(1 << 16) as usize];
    e.mem().read_bytes(s, &mut b);
    if let Ok(mut ans) = iggy(e).answer.lock() {
        *ans = Some(String::from_utf8_lossy(&b).into_owned());
    }
    Ret::eax(1)
}

fn background_color(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    mem.write(a(args, 1), [0f32; 3]);
    Ret::VOID
}

/// An AS3 `ExternalInterface.call(name, args)` reaching the game: the
/// game's callback (`IggyMenuManager_callback_00a260f0`, stdcall(user,
/// player, call); the call: NUL-terminated name pointer, length, argument
/// count, then `IggyDataValue`s from `+0x10`) on this thread; returns the
/// answer the game set.
pub fn as3_call(e: &mut Exec, player: u32, name: &str, args: &[Value]) -> Option<String> {
    let cb = iggy(e).callback.load(Ordering::Acquire);
    if cb == 0 {
        return None;
    }
    iggy(e).count(&format!("as3 {name}"));
    let strings: usize = args
        .iter()
        .map(|v| match v {
            Value::String(s) => s.len() + 1,
            _ => 0,
        })
        .sum();
    let size = u32::try_from(0x10 + 16 * args.len() + name.len() + 1 + strings).ok()?;
    let block = e.alloc(size);
    if block == 0 {
        return None;
    }
    let mut mem = e.mem();
    let mut text = block + 0x10 + 16 * u32::try_from(args.len()).ok()?;
    let name_at = text;
    mem.write_bytes(text, name.as_bytes());
    mem.write(text + u32::try_from(name.len()).ok()?, 0u8);
    text += u32::try_from(name.len() + 1).ok()?;
    mem.write(block, name_at);
    mem.write(block + 4, u32::try_from(name.len()).ok()?);
    mem.write(block + 8, u32::try_from(args.len()).ok()?);
    mem.write(block + 12, 0u32);
    for (i, v) in args.iter().enumerate() {
        let at = block + 0x10 + 16 * u32::try_from(i).ok()?;
        mem.write_bytes(at, &[0u8; 16]);
        match v {
            Value::Undefined => {}
            Value::Bool(b) => {
                mem.write(at, 3u32);
                mem.write(at + 8, u32::from(*b));
            }
            Value::Number(n) => {
                mem.write(at, 4u32);
                mem.write(at + 8, *n);
            }
            Value::String(s) => {
                mem.write(at, 5u32);
                mem.write(at + 8, text);
                mem.write(at + 12, u32::try_from(s.len()).ok()?);
                mem.write_bytes(text, s.as_bytes());
                mem.write(text + u32::try_from(s.len()).ok()?, 0u8);
                text += u32::try_from(s.len() + 1).ok()?;
            }
        }
    }
    if let Ok(mut ans) = iggy(e).answer.lock() {
        *ans = None;
    }
    let user = iggy(e).user.load(Ordering::Acquire);
    let _ = e.call_stdcall(cb, &[user, player, block]);
    e.release(block);
    iggy(e).answer.lock().ok().and_then(|mut a| a.take())
}

const IGGY: &str = "iggy_w32.dll";

/// The layer's imports (the others are null stand-ins: 0).
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (
        IGGY,
        "_IggyPlayerCreateFromMemory@12",
        Conv::Stdcall,
        3,
        create_from_memory,
    ),
    (IGGY, "_IggyPlayerDestroy@4", Conv::Stdcall, 1, destroy),
    (
        IGGY,
        "_IggyPlayerProperties@4",
        Conv::Stdcall,
        1,
        properties,
    ),
    (
        IGGY,
        "_IggyPlayerCreateFastNameUTF8@12",
        Conv::Stdcall,
        3,
        fast_name,
    ),
    (
        IGGY,
        "_IggyPlayerCallFunctionRS@20",
        Conv::Stdcall,
        5,
        call_function,
    ),
    (
        IGGY,
        "_IggyPlayerReadyToTick@4",
        Conv::Stdcall,
        1,
        ready_to_tick,
    ),
    (IGGY, "_IggyPlayerTickRS@4", Conv::Stdcall, 1, tick),
    (
        IGGY,
        "_IggyPlayerInitializeAndTickRS@4",
        Conv::Stdcall,
        1,
        initialize_and_tick,
    ),
    (
        IGGY,
        "_IggyPlayerSetDisplaySize@12",
        Conv::Stdcall,
        3,
        set_display_size,
    ),
    (IGGY, "_IggyPlayerDraw@4", Conv::Stdcall, 1, draw),
    (
        IGGY,
        "_IggyMakeEventMouseMove@12",
        Conv::Stdcall,
        3,
        event_mouse_move,
    ),
    (IGGY, "_IggyMakeEventNone@4", Conv::Stdcall, 1, event_none),
    (
        IGGY,
        "_IggyPlayerDispatchEventRS@12",
        Conv::Stdcall,
        3,
        dispatch_event,
    ),
    (
        IGGY,
        "_IggySetAS3ExternalFunctionCallbackUTF8@8",
        Conv::Stdcall,
        2,
        set_as3_callback,
    ),
    (
        IGGY,
        "_IggyPlayerCallbackResultPath@4",
        Conv::Stdcall,
        1,
        callback_result_path,
    ),
    (
        IGGY,
        "_IggyValueSetStringUTF8RS@20",
        Conv::Stdcall,
        5,
        value_set_string,
    ),
    (
        IGGY,
        "_IggyPlayerGetBackgroundColor@8",
        Conv::Stdcall,
        2,
        background_color,
    ),
];

#[cfg(feature = "swf")]
mod swf;
