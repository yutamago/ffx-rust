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
//! | `IggySetAS3ExternalFunctionCallbackUTF8(cb, user)` | an AS3 `ExternalInterface.call` runs `cb(user, player, call)` (stdcall) while the movie waits; the answer is what the game sets with `IggyPlayerCallbackResultPath` + `IggyValueSetStringUTF8RS`, the call's result in the movie |
//! | `IggyValueRefCreateArray`, `RefFromTempRef`, `RefFree` | arrays the layer keeps by handle (string arrays the game passes as type 9 values; arrays AS3 passes to the callback as type 10 temporary refs) |
//! | `IggyValuePathFromRef`, `PathMakeArrayRef`, `PathSetArrayIndex` | a value path in the game's 24-byte frame: player, parent path, name, ref, index, kind (Iggy's own words, as the hook's Iggy log shows them) |
//! | `IggyValueSetStringUTF8RS`, `GetS32RS`, `GetStringUTF8RS` | an array element through its path (or the callback's answer) |
//! | `IggyPlayerDraw` | the movie's image composited over `GDraw`'s target (`crate::gfx` `Gfx::composite`) |
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

/// `GDraw`'s D3D11 state (`DAT_01a858b4`, a pointer): the target view
/// `+0x780` and the draw's origin `+0x6c8`, `+0x6cc` that
/// `gdraw_setRenderTargetView_00a38300` stores right before
/// `IggyPlayerDraw` (`IggyPlayer_tickDraw_00a3d9f0`).
const GDRAW_STATE: u32 = 0x01a8_58b4;

/// A value path's kinds (word 5 of the 24-byte path): from a ref, an
/// array element (Iggy's own numbers), and the layer's callback result.
const PATH_REF: u32 = 4;
const PATH_ELEMENT: u32 = 1;
const PATH_RESULT: u32 = 0x100;

/// `IggyDataValue` types the layer reads and writes beyond 3-6: a
/// value ref (the game's string arrays, `IggyValue_setType9_00a28dd0`) and
/// a temporary ref (an array AS3 passes to the callback).
const TYPE_REF: u32 = 9;
const TYPE_TEMP_REF: u32 = 10;

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
    /// An array: type 9 (a ref the game built) into AS3, type 10 (a
    /// temporary ref) from AS3.
    List(Vec<Value>),
}

/// The arrays the layer keeps for value refs.
#[derive(Debug, Default)]
struct Refs {
    next: u32,
    arrays: HashMap<u32, Vec<Value>>,
}

impl Refs {
    fn add(&mut self, values: Vec<Value>) -> u32 {
        // non-zero, never reused
        self.next = self.next.wrapping_add(1).max(0x100);
        self.arrays.insert(self.next, values);
        self.next
    }
}

/// `ToInt32` of a number, as AS3 converts (wraps modulo 2^32).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_int32(n: f64) -> i32 {
    if !n.is_finite() {
        return 0;
    }
    let t = n.trunc().rem_euclid(4_294_967_296.0);
    (t as u64 as u32).cast_signed()
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
    /// Value refs (arrays).
    refs: Mutex<Refs>,
    /// Images composited, and draws that had none (for the report).
    pub composited: AtomicU32,
    pub not_composited: AtomicU32,
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
    swf::create(e, p, &swf, (width, height));
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
    swf::destroy(e, p);
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
                TYPE_REF => iggy(e)
                    .refs
                    .lock()
                    .ok()
                    .and_then(|r| r.arrays.get(&text).cloned())
                    .map_or(Value::Undefined, Value::List),
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
            if pl.ticks.is_power_of_two() {
                crate::log::note(&format!("iggy: {p:#x} tick {}", pl.ticks));
            }
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
    swf::set_size(e, a(args, 0), (a(args, 1), a(args, 2)));
    #[cfg(not(feature = "swf"))]
    let _ = (e, args);
    Ret::VOID
}

/// `IggyPlayerDraw(player)`: the movie's image over `GDraw`'s target, at
/// the origin `IggyPlayer_tickDraw_00a3d9f0` set, in the order the game
/// draws its menus (on the render thread, between its own draws).
fn draw(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    if let Ok(mut ps) = iggy(e).players.lock()
        && let Some(pl) = ps.get_mut(&p)
    {
        pl.draws += 1;
        if pl.draws.is_power_of_two() {
            crate::log::note(&format!("iggy: {p:#x} draw {}", pl.draws));
        }
    }
    #[cfg(feature = "swf")]
    {
        let image = swf::draw(e, p);
        composite(e, image.as_ref());
    }
    Ret::VOID
}

/// Hands a player's image to the D3D11 layer: premultiplied RGBA rows
/// over `GDraw`'s target view at its origin.
#[cfg_attr(not(feature = "swf"), allow(dead_code))]
fn composite(e: &Exec, image: Option<&(u32, u32, Vec<u8>)>) {
    let Some((width, height, rows)) = image else {
        iggy(e).not_composited.fetch_add(1, Ordering::Relaxed);
        return;
    };
    let mem = e.mem();
    let state = mem.read::<u32>(GDRAW_STATE);
    let (view, x, y) = if state == 0 {
        (0, 0, 0)
    } else {
        (
            mem.read::<u32>(state + 0x780),
            mem.read::<u32>(state + 0x6c8).cast_signed(),
            mem.read::<u32>(state + 0x6cc).cast_signed(),
        )
    };
    let done = view != 0
        && e.shared()
            .gfx
            .lock()
            .as_mut()
            .is_some_and(|g| g.composite(view, (x, y), (*width, *height), rows));
    let counter = if done {
        &iggy(e).composited
    } else {
        &iggy(e).not_composited
    };
    let n = counter.fetch_add(1, Ordering::Relaxed);
    if n < 3 || n.is_power_of_two() {
        let covered = rows.chunks(4).filter(|p| p[3] != 0).count();
        crate::log::note(&format!(
            "iggy: draw {} {}: {width}x{height} at ({x}, {y}) on view {view:#x}, {covered} pixels not transparent",
            if done { "composited" } else { "not composited" },
            n + 1
        ));
    }
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
        let (x, y) = (
            mem.read::<u32>(ev + 8).cast_signed(),
            mem.read::<u32>(ev + 12).cast_signed(),
        );
        #[cfg(feature = "swf")]
        swf::mouse_move(e, p, x, y);
        #[cfg(not(feature = "swf"))]
        let _ = (p, x, y);
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

/// `IggyPlayerCallbackResultPath(player)`: a path (in the player's block)
/// whose value is the answer to the AS3 call in progress.
fn callback_result_path(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    if p == 0 {
        return Ret::eax(0);
    }
    write_path(e, p + RESULT_PATH, [p, 0, 0, 0, 0, PATH_RESULT]);
    Ret::eax(p + RESULT_PATH)
}

/// A value path: 24 bytes in the game's frame, Iggy's six words (player,
/// parent path, name, ref, index, kind).
fn write_path(e: &Exec, at: u32, words: [u32; 6]) {
    if at != 0 {
        e.mem().write(at, words);
    }
}

fn read_path(e: &Exec, at: u32) -> [u32; 6] {
    if at == 0 {
        return [0; 6];
    }
    e.mem().read(at)
}

/// Where a path points: an array ref and an element index.
fn element(e: &Exec, path: u32) -> Option<(u32, usize)> {
    let p = read_path(e, path);
    if p[5] != PATH_ELEMENT {
        return None;
    }
    let parent = read_path(e, p[1]);
    (parent[5] == PATH_REF).then_some((parent[3], p[4] as usize))
}

/// The value at a path (an element of an array ref), or `None`.
fn value_at(e: &Exec, path: u32) -> Option<Value> {
    let (r, i) = element(e, path)?;
    iggy(e).refs.lock().ok()?.arrays.get(&r)?.get(i).cloned()
}

/// `IggyValueRefCreateArray(player, n)`: a ref to an array of `n`
/// undefined values.
fn ref_create_array(e: &mut Exec, args: &Args) -> Ret {
    iggy(e).count("ValueRefCreateArray");
    let n = a(args, 1).min(1 << 16) as usize;
    let h = iggy(e)
        .refs
        .lock()
        .map_or(0, |mut r| r.add(vec![Value::Undefined; n]));
    Ret::eax(h)
}

/// `IggyValueRefFromTempRef(player, temp, flags)`: a ref of its own to a
/// temporary ref's array (the temporary one goes when the callback
/// returns).
fn ref_from_temp(e: &mut Exec, args: &Args) -> Ret {
    let temp = a(args, 1);
    let h = iggy(e).refs.lock().map_or(0, |mut r| {
        let values = r.arrays.get(&temp).cloned().unwrap_or_default();
        r.add(values)
    });
    Ret::eax(h)
}

/// `IggyValueRefFree(player, ref)`.
fn ref_free(e: &mut Exec, args: &Args) -> Ret {
    if let Ok(mut r) = iggy(e).refs.lock() {
        r.arrays.remove(&a(args, 1));
    }
    Ret::eax(0)
}

/// `IggyValuePathFromRef(path, player, ref)` -> path.
fn path_from_ref(e: &mut Exec, args: &Args) -> Ret {
    let path = a(args, 0);
    write_path(e, path, [a(args, 1), 0, 0, a(args, 2), 0, PATH_REF]);
    Ret::eax(path)
}

/// `IggyValuePathMakeArrayRef(path, parent, index)` -> path.
fn path_make_array_ref(e: &mut Exec, args: &Args) -> Ret {
    let (path, parent) = (a(args, 0), a(args, 1));
    let player = read_path(e, parent)[0];
    write_path(e, path, [player, parent, 0, 0, a(args, 2), PATH_ELEMENT]);
    Ret::eax(path)
}

/// `IggyValuePathSetArrayIndex(path, index)` -> path.
fn path_set_array_index(e: &mut Exec, args: &Args) -> Ret {
    let path = a(args, 0);
    if path != 0 {
        e.mem().write(path + 16, a(args, 1));
    }
    Ret::eax(path)
}

/// A UTF-8 argument: `len` bytes, or up to the NUL when `len` is
/// negative.
fn utf8_arg(e: &Exec, s: u32, len: u32) -> String {
    let mem = e.mem();
    let bytes: Vec<u8> = if len.cast_signed() < 0 {
        (0..1u32 << 16)
            .map(|i| mem.read::<u8>(s + i))
            .take_while(|&b| b != 0)
            .collect()
    } else {
        let mut b = vec![0u8; len.min(1 << 16) as usize];
        mem.read_bytes(s, &mut b);
        b
    };
    String::from_utf8_lossy(&bytes).into_owned()
}

/// `IggyValueSetStringUTF8RS(path, sub, sub_utf8, s, len)`: on a
/// player's callback result path, the answer to the AS3 call in progress;
/// on an element path, that element of the array (the game's string
/// arrays, `IggyStringArray_ctor_006fcbc0`). True (1) when set.
fn value_set_string(e: &mut Exec, args: &Args) -> Ret {
    let path = a(args, 0);
    let s = utf8_arg(e, a(args, 3), a(args, 4));
    if read_path(e, path)[5] == PATH_RESULT {
        if let Ok(mut ans) = iggy(e).answer.lock() {
            *ans = Some(s);
        }
        return Ret::eax(1);
    }
    let Some((r, i)) = element(e, path) else {
        return Ret::eax(0);
    };
    let set = iggy(e).refs.lock().is_ok_and(|mut refs| {
        refs.arrays.get_mut(&r).is_some_and(|v| {
            if v.len() <= i {
                v.resize(i + 1, Value::Undefined);
            }
            v[i] = Value::String(s);
            true
        })
    });
    Ret::eax(u32::from(set))
}

/// `IggyValueGetS32RS(path, sub, sub_utf8, out)`: 0 (success) with the
/// value converted as AS3 does, else 1 and `out` left alone.
fn value_get_s32(e: &mut Exec, args: &Args) -> Ret {
    let out = a(args, 3);
    let v = match value_at(e, a(args, 0)) {
        Some(Value::Number(n)) => to_int32(n),
        Some(Value::Bool(b)) => i32::from(b),
        Some(Value::String(s)) => s.trim().parse::<f64>().map_or(0, to_int32),
        _ => return Ret::eax(1),
    };
    if out != 0 {
        e.mem().write(out, v);
    }
    Ret::eax(0)
}

/// `IggyValueGetStringUTF8RS(path, sub, sub_utf8, size, buf, len_out)`:
/// 0 with the string (at most `size - 1` bytes, NUL-terminated) and its
/// length, else 1.
fn value_get_string(e: &mut Exec, args: &Args) -> Ret {
    let (size, buf, len_out) = (a(args, 3), a(args, 4), a(args, 5));
    let s = match value_at(e, a(args, 0)) {
        Some(Value::String(s)) => s,
        Some(Value::Number(n)) => format!("{n}"),
        Some(Value::Bool(b)) => format!("{b}"),
        _ => return Ret::eax(1),
    };
    let n = s.len().min(size.saturating_sub(1) as usize);
    let n32 = u32::try_from(n).unwrap_or(0);
    let mut mem = e.mem();
    if buf != 0 && size != 0 {
        mem.write_bytes(buf, &s.as_bytes()[..n]);
        mem.write(buf + n32, 0u8);
    }
    if len_out != 0 {
        mem.write(len_out, n32);
    }
    Ret::eax(0)
}

/// `IggyGDrawMallocAnnotated(size, file, line)`: `GDraw`'s memory (its
/// state, the quad index buffer's contents, the render-target pool) from
/// the arena heap. With the null stand-in's 0 `IggyGDraw_setup_00a3baf0`
/// failed, `g_GDrawContext` stayed 0 and `IggyMenuManager_Render_00a267f0`
/// drew no menu: no tick, no draw.
fn gdraw_malloc(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(e.alloc(a(args, 0)))
}

/// `IggyGDrawFree(p)`.
fn gdraw_free(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    if p != 0 {
        e.release(p);
    }
    Ret::VOID
}

fn background_color(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    mem.write(a(args, 1), [0f32; 3]);
    Ret::VOID
}

/// An AS3 `ExternalInterface.call(name, args)` reaching the game: the
/// game's callback (`IggyMenuManager_callback_00a260f0`, stdcall(user,
/// player, call); the call: NUL-terminated name pointer, length, argument
/// count, then `IggyDataValue`s from `+0x10`; an array as a temporary ref,
/// type 10 with the ref at `+4`, as Iggy passes them) on this thread;
/// returns the answer the game set.
pub fn as3_call(e: &mut Exec, player: u32, name: &str, args: &[Value]) -> Option<String> {
    let cb = iggy(e).callback.load(Ordering::Acquire);
    if cb == 0 {
        return None;
    }
    let mut temps = Vec::new();
    iggy(e).count(&format!("as3 {name}"));
    crate::log::note(&format!("iggy: {player:#x} calls the game: {name}{args:?}"));
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
            Value::List(items) => {
                let temp = iggy(e).refs.lock().map_or(0, |mut r| r.add(items.clone()));
                temps.push(temp);
                mem.write(at, TYPE_TEMP_REF);
                mem.write(at + 4, temp);
            }
        }
    }
    if let Ok(mut ans) = iggy(e).answer.lock() {
        *ans = None;
    }
    let user = iggy(e).user.load(Ordering::Acquire);
    let _ = e.call_stdcall(cb, &[user, player, block]);
    e.release(block);
    if let Ok(mut r) = iggy(e).refs.lock() {
        for t in temps {
            r.arrays.remove(&t);
        }
    }
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
    (
        IGGY,
        "_IggyGDrawMallocAnnotated@12",
        Conv::Stdcall,
        3,
        gdraw_malloc,
    ),
    (IGGY, "_IggyGDrawFree@4", Conv::Stdcall, 1, gdraw_free),
    (
        IGGY,
        "_IggyValueRefCreateArray@8",
        Conv::Stdcall,
        2,
        ref_create_array,
    ),
    (
        IGGY,
        "_IggyValueRefFromTempRef@12",
        Conv::Stdcall,
        3,
        ref_from_temp,
    ),
    (IGGY, "_IggyValueRefFree@8", Conv::Stdcall, 2, ref_free),
    (
        IGGY,
        "_IggyValuePathFromRef@12",
        Conv::Stdcall,
        3,
        path_from_ref,
    ),
    (
        IGGY,
        "_IggyValuePathMakeArrayRef@12",
        Conv::Stdcall,
        3,
        path_make_array_ref,
    ),
    (
        IGGY,
        "_IggyValuePathSetArrayIndex@8",
        Conv::Stdcall,
        2,
        path_set_array_index,
    ),
    (
        IGGY,
        "_IggyValueGetS32RS@16",
        Conv::Stdcall,
        4,
        value_get_s32,
    ),
    (
        IGGY,
        "_IggyValueGetStringUTF8RS@24",
        Conv::Stdcall,
        6,
        value_get_string,
    ),
];

#[cfg(feature = "swf")]
mod swf;
