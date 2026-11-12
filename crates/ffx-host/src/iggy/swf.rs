//! The players on Ruffle (`ffx_swf::stage::Stage`), feature `swf`.
//!
//! Ruffle's players are not `Send` and the game calls Iggy from the main
//! thread (create, calls, events) and the render thread (ticks, draws), so
//! every stage lives on one thread of its own, which runs the requests in
//! order and answers each.
//!
//! - **AS3 external calls** (`ExternalInterface.call`) reach the game while
//!   the movie waits, as Iggy runs its callback: the stage thread hands the
//!   call to the game thread whose request is running (the one waiting in
//!   [`Slot::ask`]), which runs the game's callback ([`super::as3_call`])
//!   and sends the answer back; that is the call's result in the movie.
//!   Iggy calls the game makes from inside its callback are nested
//!   requests: the player is busy (Ruffle's player is locked during the
//!   call), so they are answered at once and run right after the request
//!   that made the call, their own external calls going to the same game
//!   thread. Other threads' requests wait their turn.
//! - **`IggyPlayerDraw`**: the image, premultiplied, for the D3D11 layer to
//!   composite (`crate::gfx` `Gfx::composite`). With `FFX_HOST_IGGY_OUT=<dir>`
//!   every 30th draw of a player is also written there as
//!   `<player>-<draw>.png` (straight alpha).

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::ThreadId;

use ffx_swf::stage::RuffleValue;
use ffx_swf::stage::{GameFont, Gpu, Render, Stage};

use super::Value;
use crate::exec::Exec;

/// A premultiplied image: width, height, RGBA rows.
pub type Image = (u32, u32, Vec<u8>);

enum Req {
    Create(u32, Vec<u8>, (u32, u32)),
    Destroy(u32),
    Size(u32, (u32, u32)),
    Mouse(u32, i32, i32),
    Tick(u32),
    Call(u32, String, Vec<Value>),
    Draw(u32, Option<std::path::PathBuf>),
}

/// What reaches the stage thread.
enum ToStage {
    /// A request from a game thread, with where to answer.
    Req(Req, ThreadId, Sender<Reply>),
    /// The game's answer to the external call the stage waits on.
    Answer(Option<String>),
}

/// What a game thread waiting on a request gets.
enum Reply {
    /// The request is done (a draw's image).
    Done(Option<Image>),
    /// An external call to run now (player, name, arguments).
    External(u32, String, Vec<Value>),
}

/// The stage thread's handle.
#[derive(Default)]
pub struct Slot {
    tx: Mutex<Option<Sender<ToStage>>>,
    draws: Mutex<HashMap<u32, u64>>,
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("iggy::swf::Slot")
    }
}

impl Slot {
    /// The stage thread's inbox (started on first use).
    fn sender(&self) -> Option<Sender<ToStage>> {
        let mut tx = self.tx.lock().ok()?;
        if tx.is_none() {
            let (stx, srx) = channel::<ToStage>();
            std::thread::Builder::new()
                .name("ffx-host swf".into())
                .spawn(move || stage_thread(srx))
                .ok()?;
            *tx = Some(stx);
        }
        tx.clone()
    }
}

/// Sends `req` to the stage thread, runs the external calls it makes on
/// this game thread and waits for it to be done.
fn ask(e: &mut Exec, req: Req) -> Option<Image> {
    let tx = super::iggy(e).swf.sender()?;
    {
        let (rtx, rrx) = channel::<Reply>();
        tx.send(ToStage::Req(req, std::thread::current().id(), rtx))
            .ok()?;
        loop {
            match rrx.recv().ok()? {
                Reply::Done(image) => return image,
                Reply::External(p, name, args) => {
                    let answer = super::as3_call(e, p, &name, &args);
                    if tx.send(ToStage::Answer(answer)).is_err() {
                        return None;
                    }
                }
            }
        }
    }
}

pub(super) fn create(e: &mut Exec, p: u32, swf: &[u8], size: (u32, u32)) {
    ask(e, Req::Create(p, swf.to_vec(), size));
}

pub(super) fn destroy(e: &mut Exec, p: u32) {
    ask(e, Req::Destroy(p));
}

pub(super) fn set_size(e: &mut Exec, p: u32, size: (u32, u32)) {
    ask(e, Req::Size(p, size));
}

pub(super) fn mouse_move(e: &mut Exec, p: u32, x: i32, y: i32) {
    ask(e, Req::Mouse(p, x, y));
}

pub(super) fn tick(e: &mut Exec, p: u32) {
    ask(e, Req::Tick(p));
}

pub(super) fn call(e: &mut Exec, p: u32, name: &str, args: Vec<Value>) {
    ask(e, Req::Call(p, name.to_owned(), args));
}

/// The player's image now (premultiplied), for the layer to composite.
pub(super) fn draw(e: &mut Exec, p: u32) -> Option<Image> {
    let slot = &super::iggy(e).swf;
    let n = slot.draws.lock().map_or(0, |mut d| {
        let n = d.entry(p).or_default();
        *n += 1;
        *n
    });
    let out = std::env::var_os("FFX_HOST_IGGY_OUT")
        .filter(|_| n % 30 == 1)
        .map(|d| std::path::PathBuf::from(d).join(format!("{p:08x}-{n:06}.png")));
    ask(e, Req::Draw(p, out))
}

fn to_ruffle(v: &Value) -> RuffleValue {
    match v {
        Value::Undefined => RuffleValue::Undefined,
        Value::Bool(b) => RuffleValue::Bool(*b),
        Value::Number(n) => RuffleValue::Number(*n),
        Value::String(s) => RuffleValue::String(s.clone()),
        Value::List(items) => RuffleValue::List(items.iter().map(to_ruffle).collect()),
    }
}

fn from_ruffle(v: &RuffleValue) -> Value {
    match v {
        RuffleValue::Bool(b) => Value::Bool(*b),
        RuffleValue::Number(n) => Value::Number(*n),
        RuffleValue::String(s) => Value::String(s.clone()),
        RuffleValue::List(items) => Value::List(items.iter().map(from_ruffle).collect()),
        _ => Value::Undefined,
    }
}

/// The game thread a request runs for, and where its replies go.
type Current = Rc<RefCell<Option<(ThreadId, Sender<Reply>)>>>;
/// Requests waiting: (request, thread, reply).
type Queue = Rc<RefCell<VecDeque<(Req, ThreadId, Sender<Reply>)>>>;

/// The stage's live answer: hands the call to the current game thread and
/// waits for its answer, keeping the requests that come meanwhile.
fn live(
    player: u32,
    inbox: &Rc<Receiver<ToStage>>,
    current: &Current,
    nested: &Queue,
    backlog: &Queue,
) -> ffx_swf::stage::Live {
    let (inbox, current, nested, backlog) = (
        inbox.clone(),
        current.clone(),
        nested.clone(),
        backlog.clone(),
    );
    Box::new(move |name: &str, args: &[RuffleValue]| {
        let Some((thread, reply)) = current.borrow().clone() else {
            return RuffleValue::Undefined;
        };
        let args = args.iter().map(from_ruffle).collect();
        if reply
            .send(Reply::External(player, name.to_owned(), args))
            .is_err()
        {
            return RuffleValue::Undefined;
        }
        loop {
            match inbox.recv() {
                Ok(ToStage::Answer(a)) => {
                    return a.map_or(RuffleValue::Undefined, RuffleValue::String);
                }
                Ok(ToStage::Req(req, from, r)) if from == thread => {
                    // the game's callback calling Iggy: the player is busy
                    let _ = r.send(Reply::Done(None));
                    nested.borrow_mut().push_back((req, from, r));
                }
                Ok(ToStage::Req(req, from, r)) => backlog.borrow_mut().push_back((req, from, r)),
                Err(_) => return RuffleValue::Undefined,
            }
        }
    })
}

/// Every player's Ruffle stage, one request at a time.
struct Stages {
    gpu: Option<Gpu>,
    font: Option<GameFont>,
    players: HashMap<u32, Stage>,
    inbox: Rc<Receiver<ToStage>>,
    current: Current,
    nested: Queue,
    backlog: Queue,
}

impl Stages {
    fn run(&mut self, req: Req) -> Option<Image> {
        match req {
            Req::Create(p, swf, size) => {
                self.create(p, &swf, size);
                None
            }
            Req::Destroy(p) => {
                self.players.remove(&p);
                None
            }
            Req::Size(p, (w, h)) => {
                if let Some(s) = self.players.get_mut(&p) {
                    s.set_size(w, h);
                }
                None
            }
            Req::Mouse(p, x, y) => {
                if let Some(s) = self.players.get_mut(&p) {
                    s.mouse_move(x, y);
                }
                None
            }
            Req::Tick(p) => {
                if let Some(s) = self.players.get_mut(&p) {
                    s.tick();
                }
                None
            }
            Req::Call(p, name, args) => {
                if let Some(s) = self.players.get_mut(&p) {
                    s.call_values(&name, args.iter().map(to_ruffle).collect());
                }
                None
            }
            Req::Draw(p, out) => {
                let s = self.players.get_mut(&p)?;
                if let Some(path) = out {
                    match s.capture() {
                        Ok(img) => {
                            let _ = img.save(&path);
                        }
                        Err(e) => crate::log::note(&format!("iggy: no image of {p:#x}: {e}")),
                    }
                }
                match s.capture_premultiplied() {
                    Ok(img) => Some(img),
                    Err(e) => {
                        crate::log::note(&format!("iggy: no image of {p:#x}: {e}"));
                        None
                    }
                }
            }
        }
    }

    fn create(&mut self, p: u32, swf: &[u8], size: (u32, u32)) {
        let Some(gpu) = &self.gpu else {
            return;
        };
        // the game's font is `box.swf`'s DefineFont3: a movie whose font
        // registers keeps it for the others
        let own = GameFont::from_swf(swf).ok();
        let url = format!("file:///{p:08x}.swf");
        let render = Render::gdraw();
        let made = [own.as_ref(), self.font.as_ref(), None]
            .into_iter()
            .enumerate()
            .find_map(|(i, f)| {
                Stage::new(gpu, swf, &url, size, f, &render)
                    .ok()
                    .map(|s| (i, s))
            });
        match made {
            Some((i, s)) => {
                if i == 0 {
                    self.font = own;
                }
                s.answers.borrow_mut().live = Some(live(
                    p,
                    &self.inbox,
                    &self.current,
                    &self.nested,
                    &self.backlog,
                ));
                self.players.insert(p, s);
            }
            None => crate::log::note(&format!("iggy: Ruffle refused {p:#x}")),
        }
    }

    /// Runs one request and the nested ones it left, then answers it.
    fn serve(&mut self, req: Req, from: ThreadId, reply: &Sender<Reply>) {
        *self.current.borrow_mut() = Some((from, reply.clone()));
        let image = self.run(req);
        loop {
            let next = self.nested.borrow_mut().pop_front();
            let Some((req, ..)) = next else { break };
            self.run(req);
        }
        *self.current.borrow_mut() = None;
        let _ = reply.send(Reply::Done(image));
    }
}

/// The stage thread.
fn stage_thread(inbox: Receiver<ToStage>) {
    let gpu = match Gpu::new() {
        Ok(g) => Some(g),
        Err(e) => {
            crate::log::note(&format!("iggy: no GPU for Ruffle ({e}): null players"));
            None
        }
    };
    ffx_swf::stage::gdraw_like();
    let mut st = Stages {
        gpu,
        font: None,
        players: HashMap::new(),
        inbox: Rc::new(inbox),
        current: Rc::new(RefCell::new(None)),
        nested: Rc::new(RefCell::new(VecDeque::new())),
        backlog: Rc::new(RefCell::new(VecDeque::new())),
    };
    loop {
        let waiting = st.backlog.borrow_mut().pop_front();
        let next = match waiting {
            Some(r) => r,
            None => match st.inbox.recv() {
                Ok(ToStage::Req(req, from, reply)) => (req, from, reply),
                // an answer nobody waits for (a stage gone meanwhile)
                Ok(ToStage::Answer(_)) => continue,
                Err(_) => return,
            },
        };
        let (req, from, reply) = next;
        st.serve(req, from, &reply);
    }
}
