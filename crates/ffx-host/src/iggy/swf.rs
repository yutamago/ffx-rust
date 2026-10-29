//! The players on Ruffle (`ffx_swf::stage::Stage`), feature `swf`.
//!
//! Ruffle's players are not `Send` and the game calls Iggy from the main
//! thread (create, calls, events) and the render thread (ticks, draws), so
//! every stage lives on one thread of its own, which runs the requests in
//! order and answers each.
//!
//! - AS3 `ExternalInterface.call`s a request made are handed back with its
//!   answer and run on the calling game thread afterwards
//!   ([`super::as3_call`]): the movie sees `undefined` as their result
//!   (`ffx_swf`'s provider answers from a recording, it has no live game
//!   to ask yet).
//! - `IggyPlayerDraw`: with `FFX_HOST_IGGY_OUT=<dir>` every 30th draw of a
//!   player is written there as `<player>-<draw>.png` (straight alpha):
//!   the image the layer is to composite over the back buffer; the
//!   compositing through the D3D11 layer is the next step (docs/host.md,
//!   "Iggy").

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

use ffx_swf::log::Arg;
use ffx_swf::stage::RuffleValue;
use ffx_swf::stage::{GameFont, Gpu, Render, Stage};

use super::Value;
use crate::exec::Exec;

enum Req {
    Create(u32, Vec<u8>, (u32, u32)),
    Destroy(u32),
    Size(u32, (u32, u32)),
    Mouse(u32, i32, i32),
    Tick(u32),
    Call(u32, String, Vec<Value>),
    Draw(u32, Option<std::path::PathBuf>),
}

/// What a request left for the game: the AS3 external calls it made.
type Done = Vec<(String, Vec<Value>)>;

/// The stage thread's handle.
#[derive(Default)]
pub struct Slot {
    chan: Mutex<Option<(Sender<Req>, Receiver<Done>)>>,
    draws: Mutex<HashMap<u32, u64>>,
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("iggy::swf::Slot")
    }
}

impl Slot {
    /// Sends `req` and waits for its answer (starting the thread first).
    fn ask(&self, req: Req) -> Done {
        let Ok(mut c) = self.chan.lock() else {
            return Vec::new();
        };
        if c.is_none() {
            let (tx, rx) = channel::<Req>();
            let (dtx, drx) = channel::<Done>();
            let spawned = std::thread::Builder::new()
                .name("ffx-host swf".into())
                .spawn(move || stage_thread(&rx, &dtx));
            if spawned.is_err() {
                return Vec::new();
            }
            *c = Some((tx, drx));
        }
        let Some((tx, rx)) = c.as_ref() else {
            return Vec::new();
        };
        if tx.send(req).is_err() {
            return Vec::new();
        }
        rx.recv().unwrap_or_default()
    }

    pub(super) fn create(&self, p: u32, swf: &[u8], size: (u32, u32)) {
        self.ask(Req::Create(p, swf.to_vec(), size));
    }

    pub(super) fn destroy(&self, p: u32) {
        self.ask(Req::Destroy(p));
    }

    pub(super) fn set_size(&self, p: u32, size: (u32, u32)) {
        self.ask(Req::Size(p, size));
    }

    pub(super) fn mouse_move(&self, p: u32, x: i32, y: i32) {
        self.ask(Req::Mouse(p, x, y));
    }
}

/// Runs the external calls a request made, on the game thread.
fn externals(e: &mut Exec, p: u32, done: Done) {
    for (name, args) in done {
        let _ = super::as3_call(e, p, &name, &args);
    }
}

pub(super) fn tick(e: &mut Exec, p: u32) {
    let done = super::iggy(e).swf.ask(Req::Tick(p));
    externals(e, p, done);
}

pub(super) fn call(e: &mut Exec, p: u32, name: &str, args: Vec<Value>) {
    let done = super::iggy(e).swf.ask(Req::Call(p, name.to_owned(), args));
    externals(e, p, done);
}

pub(super) fn draw(e: &mut Exec, p: u32) {
    let slot = &super::iggy(e).swf;
    let n = slot.draws.lock().map_or(0, |mut d| {
        let n = d.entry(p).or_default();
        *n += 1;
        *n
    });
    let out = std::env::var_os("FFX_HOST_IGGY_OUT")
        .filter(|_| n % 30 == 1)
        .map(|d| std::path::PathBuf::from(d).join(format!("{p:08x}-{n:06}.png")));
    let done = slot.ask(Req::Draw(p, out));
    externals(e, p, done);
}

fn to_arg(v: &Value) -> Arg {
    match v {
        Value::Undefined => Arg::Undefined,
        Value::Bool(b) => Arg::Bool(*b),
        Value::Number(n) => Arg::Number(*n),
        Value::String(s) => Arg::String(s.clone()),
    }
}

fn from_ruffle(v: &RuffleValue) -> Value {
    match v {
        RuffleValue::Bool(b) => Value::Bool(*b),
        RuffleValue::Number(n) => Value::Number(*n),
        RuffleValue::String(s) => Value::String(s.clone()),
        _ => Value::Undefined,
    }
}

/// The stage thread: every player's Ruffle stage, one request at a time.
fn stage_thread(rx: &Receiver<Req>, done: &Sender<Done>) {
    let gpu = match Gpu::new() {
        Ok(g) => Some(g),
        Err(e) => {
            crate::log::note(&format!("iggy: no GPU for Ruffle ({e}): null players"));
            None
        }
    };
    ffx_swf::stage::gdraw_mipmaps(true);
    ffx_swf::stage::force_bitmap_smoothing(true);
    ffx_swf::stage::deterministic_get_timer(true);
    let mut font: Option<GameFont> = None;
    let mut stages: HashMap<u32, Stage> = HashMap::new();
    while let Ok(req) = rx.recv() {
        let p = match &req {
            Req::Create(p, ..)
            | Req::Destroy(p)
            | Req::Size(p, _)
            | Req::Mouse(p, ..)
            | Req::Tick(p)
            | Req::Call(p, ..)
            | Req::Draw(p, _) => *p,
        };
        match req {
            Req::Create(p, swf, size) => {
                if font.is_none() {
                    font = GameFont::from_swf(&swf).ok();
                }
                if let Some(gpu) = &gpu {
                    match Stage::new(
                        gpu,
                        &swf,
                        &format!("file:///{p:08x}.swf"),
                        size,
                        font.as_ref(),
                        &Render::default(),
                    ) {
                        Ok(s) => {
                            stages.insert(p, s);
                        }
                        Err(e) => crate::log::note(&format!("iggy: Ruffle refused {p:#x}: {e}")),
                    }
                }
            }
            Req::Destroy(p) => {
                stages.remove(&p);
            }
            Req::Size(p, (w, h)) => {
                if let Some(s) = stages.get_mut(&p) {
                    s.set_size(w, h);
                }
            }
            Req::Mouse(p, x, y) => {
                if let Some(s) = stages.get_mut(&p) {
                    s.mouse_move(x, y);
                }
            }
            Req::Tick(p) => {
                if let Some(s) = stages.get_mut(&p) {
                    s.tick();
                }
            }
            Req::Call(p, name, args) => {
                if let Some(s) = stages.get_mut(&p) {
                    let args: Vec<Arg> = args.iter().map(to_arg).collect();
                    s.call(&name, &args);
                }
            }
            Req::Draw(p, out) => {
                if let (Some(s), Some(path)) = (stages.get_mut(&p), out) {
                    match s.capture() {
                        Ok(img) => {
                            let _ = img.save(&path);
                        }
                        Err(e) => crate::log::note(&format!("iggy: no image of {p:#x}: {e}")),
                    }
                }
            }
        }
        let calls = stages.get_mut(&p).map_or_else(Vec::new, |s| {
            std::mem::take(&mut s.answers.borrow_mut().asked)
                .into_iter()
                .map(|c| (c.name, c.args.iter().map(from_ruffle).collect()))
                .collect()
        });
        if done.send(calls).is_err() {
            return;
        }
    }
}
