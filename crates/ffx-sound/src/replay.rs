//! Replays an FMOD call log (`FFX_FMOD_LOG`, `crates/ffx-hook/src/fmod_log.rs`)
//! through [`crate::mixer`]: the offline engine M5's mixer is measured
//! with. The log must carry the capture's update notes
//! (`# update <n> <dsp clock> <result>`, written with `FFX_AUDIO_WAV`): each
//! one mixes one block, after the calls logged before it.
//!
//! What it follows: projects loaded (`EventSystem::load`), groups
//! (`EventProject::getGroup`), event instances (`EventGroup::getEventByIndex`,
//! `getEvent`), their `start`, `stop`, `setVolume`, `setPaused`, the channel
//! pan (`getChannelGroup` -> `getChannel` -> `Channel::setPan`) and the Shout
//! bank (`System::createSound`, `Sound::getSubSound`, `System::playSound`,
//! `Channel::setVolume`). The movie's user stream is left out.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::fev::Project;
use crate::fsb::Bank;
use crate::mixer::{BLOCK, Voice, Wave};

/// Reads a game file by its path below `Sound_PC/` (lower case, `/`).
pub trait Files {
    fn read(&mut self, path: &str) -> Option<Arc<Vec<u8>>>;
}

/// One logged call.
#[derive(Debug, Clone, Default)]
pub struct Call {
    pub frame: u32,
    pub function: String,
    pub args: Vec<u32>,
    pub result: u32,
    /// `o<i>`: the word argument `i` pointed at after the call.
    pub outs: Vec<(usize, u32)>,
    /// `s<i>`: the string argument `i` pointed at.
    pub strings: Vec<(usize, String)>,
}

impl Call {
    fn out(&self, i: usize) -> Option<u32> {
        self.outs.iter().find(|(k, _)| *k == i).map(|(_, v)| *v)
    }
    fn string(&self, i: usize) -> Option<&str> {
        self.strings
            .iter()
            .find(|(k, _)| *k == i)
            .map(|(_, v)| v.as_str())
    }
    fn arg(&self, i: usize) -> u32 {
        self.args.get(i).copied().unwrap_or(0)
    }
}

/// A log entry.
#[derive(Debug, Clone)]
pub enum Entry {
    Call(Call),
    /// `# update <n> <dsp clock> <result>`.
    Update {
        n: u64,
        clock: u64,
    },
}

/// The short name of a decorated FMOD export: `Event::start`.
fn short(name: &str) -> String {
    let Some(rest) = name.strip_prefix('?') else {
        return name.to_owned();
    };
    let mut parts = rest.split('@');
    match (parts.next(), parts.next()) {
        (Some(m), Some(c)) if !c.is_empty() => format!("{c}::{m}"),
        _ => name.to_owned(),
    }
}

/// Parses a log.
#[must_use]
pub fn parse(text: &str) -> Vec<Entry> {
    let mut names: HashMap<u32, String> = HashMap::new();
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# fn ") {
            let mut w = rest.split(' ');
            if let (Some(i), Some(name)) = (w.next().and_then(|i| i.parse().ok()), w.next_back()) {
                names.insert(i, short(name));
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("# update ") {
            let mut w = rest.split(' ').map(str::parse::<u64>);
            if let (Some(Ok(n)), Some(Ok(clock))) = (w.next(), w.next()) {
                out.push(Entry::Update { n, clock });
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let w: Vec<&str> = line.splitn(10, ' ').collect();
        if w.len() < 9 {
            continue;
        }
        let hex = |s: &str| u32::from_str_radix(s, 16).unwrap_or(0);
        let mut call = Call {
            frame: w[2].parse().unwrap_or(0),
            function: w[5]
                .parse()
                .ok()
                .and_then(|i| names.get(&i).cloned())
                .unwrap_or_default(),
            args: if w[7] == "-" {
                Vec::new()
            } else {
                w[7].split(',').map(hex).collect()
            },
            result: hex(w[8]),
            ..Call::default()
        };
        // each value follows a space (the first one too)
        let rest = format!(" {}", w.get(9).copied().unwrap_or(""));
        let mut r = rest.as_str();

        while let Some(at) = r.find(' ') {
            r = &r[at + 1..];
            if let Some(o) = r.strip_prefix('o') {
                if let Some((k, v)) = o.split_once('=') {
                    let v = v.split(' ').next().unwrap_or("");
                    if let Ok(k) = k.parse() {
                        call.outs.push((k, hex(v)));
                    }
                }
            } else if let Some(s) = r.strip_prefix('s')
                && let Some((k, v)) = s.split_once("=\"")
                && let Ok(k) = k.parse()
            {
                let mut text = String::new();
                let mut chars = v.chars();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => text.extend(chars.next()),
                        '"' => break,
                        c => text.push(c),
                    }
                }
                call.strings.push((k, text));
            }
        }
        out.push(Entry::Call(call));
    }
    out
}

/// The game's path of a sound file, below `Sound_PC/`, lower case.
fn sound_path(path: &str) -> String {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    match lower.find("sound_pc/") {
        Some(at) => lower[at + "sound_pc/".len()..].to_owned(),
        None => lower,
    }
}

struct LoadedProject {
    project: Project,
    /// Bank path (below `Sound_PC/`).
    bank: String,
}

#[derive(Clone)]
struct Instance {
    wave: Arc<Wave>,
    oneshot: bool,
    /// The event's own volume (FEV).
    base: f32,
    /// The game's `setVolume`.
    volume: f32,
    pan: f32,
    paused: bool,
    voice: Option<u64>,
}

/// The engine: the log's FMOD state and the voices.
pub struct Engine<F: Files> {
    files: F,
    projects: HashMap<u32, LoadedProject>,
    groups: HashMap<u32, (u32, String)>,
    events: HashMap<u32, Instance>,
    channel_groups: HashMap<u32, u32>,
    channels: HashMap<u32, Target>,
    sounds: HashMap<u32, (String, Option<u32>)>,
    waves: HashMap<(String, u32), Arc<Wave>>,
    banks: HashMap<String, (Arc<Vec<u8>>, Bank)>,
    /// Playing voices by start order (the mix order).
    voices: BTreeMap<u64, Voice>,
    next_voice: u64,
    /// FMOD channel index per voice, and the free ones.
    slots: HashMap<u64, u16>,
    free: std::collections::BTreeSet<u16>,
    /// `getChannel` handles whose index matched the allocator, and didn't.
    pub channel_checks: (u32, u32),
    /// Channels of ended voices, free from the update after the first word.
    pending_free: Vec<(u64, u16)>,
    /// The mixed output, interleaved stereo.
    pub out: Vec<f32>,
    /// Updates mixed.
    pub updates: u64,
    /// Calls the engine did not follow, by function (for the report).
    pub ignored: HashMap<String, u32>,
}

#[derive(Clone, Copy)]
enum Target {
    Event(u32),
    /// A channel the game played a Shout subsound on: its voice.
    Shout(Option<u64>),
}

impl<F: Files> Engine<F> {
    pub fn new(files: F) -> Self {
        Self {
            files,
            projects: HashMap::new(),
            groups: HashMap::new(),
            events: HashMap::new(),
            channel_groups: HashMap::new(),
            channels: HashMap::new(),
            sounds: HashMap::new(),
            waves: HashMap::new(),
            banks: HashMap::new(),
            voices: BTreeMap::new(),
            next_voice: 0,
            slots: HashMap::new(),
            free: (0..128).collect(),
            channel_checks: (0, 0),
            pending_free: Vec::new(),
            out: Vec::new(),
            updates: 0,
            ignored: HashMap::new(),
        }
    }

    fn bank(&mut self, path: &str) -> Option<&(Arc<Vec<u8>>, Bank)> {
        if !self.banks.contains_key(path) {
            let bytes = self.files.read(path)?;
            let bank = Bank::parse(&bytes).ok()?;
            self.banks.insert(path.to_owned(), (bytes, bank));
        }
        self.banks.get(path)
    }

    fn wave(&mut self, bank: &str, index: u32) -> Option<Arc<Wave>> {
        let key = (bank.to_owned(), index);
        if let Some(w) = self.waves.get(&key) {
            return Some(w.clone());
        }
        let (bytes, b) = self.bank(bank)?;
        let s = b.samples.get(index as usize)?;
        let pcm = crate::ima::decode(&bytes[s.data.clone()], s.channels, s.frames);
        let w = Arc::new(Wave {
            frames: u32::try_from(pcm.len() / usize::from(s.channels)).unwrap_or(0),
            pcm,
            channels: s.channels,
            rate: s.rate,
            looped: s.looped,
        });
        self.waves.insert(key, w.clone());
        Some(w)
    }

    fn new_instance(&mut self, project: u32, group: &str, index: u32, handle: u32) {
        let Some(p) = self.projects.get(&project) else {
            return;
        };
        let Some(ev) = p
            .project
            .group(group)
            .and_then(|g| g.events.get(index as usize))
        else {
            return;
        };
        let Some((wave, oneshot)) = p.project.wave(ev) else {
            return;
        };
        let (bank, sample, base) = (p.bank.clone(), wave.index, ev.volume);
        let Some(wave) = self.wave(&bank, sample) else {
            return;
        };
        // a handle the game had already (FMOD hands the instance back): its
        // voice keeps playing until the next `start` or a `stop`
        self.events.insert(
            handle,
            Instance {
                wave,
                oneshot,
                base,
                volume: 1.0,
                pan: 0.0,
                paused: false,
                voice: self.events.get(&handle).and_then(|i| i.voice),
            },
        );
    }

    fn voice(&mut self, v: Option<u64>) -> Option<&mut Voice> {
        v.and_then(|i| self.voices.get_mut(&i))
    }

    fn add_voice(&mut self, voice: Voice) -> u64 {
        let id = self.next_voice;
        self.next_voice += 1;
        self.voices.insert(id, voice);
        // FMOD's channel: the highest free index (checked against the
        // handles `getChannel` hands out, `channel_checks`)
        if let Some(i) = self.free.pop_last() {
            self.slots.insert(id, i);
        }
        id
    }

    fn remove_voice(&mut self, id: u64) {
        self.voices.remove(&id);
        if let Some(i) = self.slots.remove(&id) {
            self.free.insert(i);
        }
    }

    fn apply_instance(&mut self, handle: u32) {
        let Some(i) = self.events.get(&handle).cloned() else {
            return;
        };
        if let Some(v) = self.voice(i.voice) {
            v.volume = i.base * i.volume;
            v.pan = i.pan;
            v.paused = i.paused;
        }
    }

    /// Applies one call.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one arm per FMOD function
    pub fn call(&mut self, c: &Call) {
        let f = c.function.as_str();
        match f {
            "EventSystem::load" => {
                if c.result != 0 {
                    return;
                }
                let (Some(name), Some(handle)) = (c.string(1), c.out(3)) else {
                    return;
                };
                let path = sound_path(name);
                let Some(bytes) = self.files.read(&path) else {
                    return;
                };
                let Ok(project) = Project::parse(&bytes) else {
                    return;
                };
                let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
                let bank = format!(
                    "{dir}/{}.fsb",
                    project
                        .banks
                        .first()
                        .map_or(String::new(), |b| b.name.to_ascii_lowercase())
                );
                self.projects
                    .insert(handle, LoadedProject { project, bank });
            }
            "EventProjectI::getGroup" => {
                if let (Some(name), Some(g)) = (c.string(1), c.out(3)) {
                    self.groups.insert(g, (c.arg(0), name.to_owned()));
                }
            }
            "EventGroupI::getEventByIndex" => {
                if let (Some((p, g)), Some(h)) = (self.groups.get(&c.arg(0)).cloned(), c.out(3)) {
                    self.new_instance(p, &g, c.arg(1), h);
                }
            }
            "EventGroupI::getEvent" => {
                let (Some((p, g)), Some(name), Some(h)) =
                    (self.groups.get(&c.arg(0)).cloned(), c.string(1), c.out(3))
                else {
                    return;
                };
                let index = self
                    .projects
                    .get(&p)
                    .and_then(|lp| lp.project.group(&g))
                    .and_then(|gr| gr.events.iter().position(|e| e.name == name));
                if let Some(i) = index {
                    self.new_instance(p, &g, u32::try_from(i).unwrap_or(0), h);
                }
            }
            "Event::start" => {
                let h = c.arg(0);
                let Some(i) = self.events.get(&h).cloned() else {
                    return;
                };
                if let Some(old) = i.voice {
                    self.remove_voice(old);
                }
                let v = self.add_voice(Voice::new(i.wave.clone(), !i.oneshot));
                if let Some(e) = self.events.get_mut(&h) {
                    e.voice = Some(v);
                }
                self.apply_instance(h);
            }
            "Event::stop" => {
                if let Some(e) = self.events.get_mut(&c.arg(0))
                    && let Some(v) = e.voice.take()
                {
                    self.remove_voice(v);
                }
            }
            "Event::setVolume" => {
                if let Some(e) = self.events.get_mut(&c.arg(0)) {
                    e.volume = f32::from_bits(c.arg(1));
                }
                self.apply_instance(c.arg(0));
            }
            "Event::setPaused" => {
                if let Some(e) = self.events.get_mut(&c.arg(0)) {
                    e.paused = c.arg(1) & 0xff != 0;
                }
                self.apply_instance(c.arg(0));
            }
            "Event::getChannelGroup" => {
                if let Some(g) = c.out(1) {
                    self.channel_groups.insert(g, c.arg(0));
                }
            }
            "ChannelGroup::getChannel" => {
                if let (Some(&e), Some(ch)) = (self.channel_groups.get(&c.arg(0)), c.out(2)) {
                    self.channels.insert(ch, Target::Event(e));
                    let slot = self
                        .events
                        .get(&e)
                        .and_then(|i| i.voice)
                        .and_then(|v| self.slots.get(&v).copied());
                    let logged = u16::try_from(ch >> 16).ok();
                    if slot == logged {
                        self.channel_checks.0 += 1;
                    } else {
                        self.channel_checks.1 += 1;
                        if std::env::var("FFX_MIX_DEBUG").is_ok() {
                            eprintln!(
                                "chan update {} frame {} event {e:x}: mine {slot:?} logged {logged:?} free top {:?} voices {}",
                                self.updates,
                                c.frame,
                                self.free.iter().rev().take(4).collect::<Vec<_>>(),
                                self.voices.len()
                            );
                        }
                    }
                }
            }
            "Channel::setPan" => match self.channels.get(&c.arg(0)).copied() {
                Some(Target::Event(e)) => {
                    if let Some(i) = self.events.get_mut(&e) {
                        i.pan = f32::from_bits(c.arg(1));
                    }
                    self.apply_instance(e);
                }
                Some(Target::Shout(v)) => {
                    if let Some(v) = self.voice(v) {
                        v.pan = f32::from_bits(c.arg(1));
                    }
                }
                None => {}
            },
            "System::createSound" => {
                if let (Some(name), Some(s)) = (c.string(1), c.out(4)) {
                    self.sounds.insert(s, (sound_path(name), None));
                }
            }
            "Sound::getSubSound" => {
                if let (Some((path, _)), Some(sub)) =
                    (self.sounds.get(&c.arg(0)).cloned(), c.out(2))
                {
                    self.sounds.insert(sub, (path, Some(c.arg(1))));
                }
            }
            "System::playSound" => {
                let (Some((path, Some(index))), Some(ch)) =
                    (self.sounds.get(&c.arg(2)).cloned(), c.out(4))
                else {
                    *self.ignored.entry(f.to_owned()).or_default() += 1;
                    return;
                };
                if let Some(Target::Shout(Some(old))) = self.channels.get(&ch).copied() {
                    self.remove_voice(old);
                }
                let Some(wave) = self.wave(&path, index) else {
                    return;
                };
                let mut voice = Voice::new(wave, false);
                voice.paused = c.arg(3) & 0xff != 0;
                let v = self.add_voice(voice);
                self.channels.insert(ch, Target::Shout(Some(v)));
            }
            "Channel::setVolume" => {
                if let Some(Target::Shout(v)) = self.channels.get(&c.arg(0)).copied()
                    && let Some(v) = self.voice(v)
                {
                    v.volume = f32::from_bits(c.arg(1));
                }
            }
            "Channel::setPaused" => {
                if let Some(Target::Shout(v)) = self.channels.get(&c.arg(0)).copied()
                    && let Some(v) = self.voice(v)
                {
                    v.paused = c.arg(1) & 0xff != 0;
                }
            }
            f if f.starts_with("Event::get")
                || f.starts_with("FMOD_")
                || f.starts_with("ChannelGroup::getNum")
                || f.starts_with("EventProjectI::getInfo")
                || f == "Event::setCallback" => {}
            _ => *self.ignored.entry(f.to_owned()).or_default() += 1,
        }
    }

    /// Mixes one block (one `EventSystem::update`).
    pub fn update(&mut self) {
        let at = self.out.len();
        self.out.resize(at + 2 * BLOCK, 0.0);
        let block = &mut self.out[at..];
        let mut order: Vec<(u64, u16)> = self
            .voices
            .keys()
            .map(|&id| (id, self.slots.get(&id).copied().unwrap_or(0)))
            .collect();
        match std::env::var("FFX_MIX_ORDER").unwrap_or_default().as_str() {
            "asc" => order.sort_by_key(|&(_, s)| s),
            "desc" => order.sort_by_key(|&(_, s)| std::cmp::Reverse(s)),
            _ => {}
        }
        for (id, _) in order {
            if let Some(v) = self.voices.get_mut(&id) {
                v.mix(block);
            }
        }
        let ended: Vec<u64> = self
            .voices
            .iter()
            .filter(|(_, v)| v.ended)
            .map(|(&id, _)| id)
            .collect();
        let delay: u64 = std::env::var("FFX_MIX_FREE_DELAY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        for id in ended {
            self.voices.remove(&id);
            if let Some(i) = self.slots.remove(&id) {
                self.pending_free.push((self.updates + delay, i));
            }
        }
        self.updates += 1;
        let now = self.updates;
        let (due, wait): (Vec<_>, Vec<_>) =
            self.pending_free.drain(..).partition(|&(d, _)| d < now);
        self.pending_free = wait;
        self.free.extend(due.into_iter().map(|(_, i)| i));
    }

    /// Replays a whole log.
    pub fn run(&mut self, log: &[Entry]) {
        for e in log {
            match e {
                Entry::Call(c) => self.call(c),
                Entry::Update { .. } => self.update(),
            }
        }
    }
}
