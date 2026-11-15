//! Hitches (docs/host.md, "Hitches"): updates and flips that take far
//! longer than a frame, with where the time went.
//!
//! `FFX_HOST_HITCH_MS` is the limit (default [`DEFAULT_MS`]; 0 turns the
//! check off). An update is the host time in `FFXApplication_animate`, a
//! flip the time in `Renderer_PresentFlip` (the harness's `timing`
//! measures the same, `crate::clock`). While one has run for half the
//! limit, the watchdog notes, every `WATCH` (20 ms), where its thread
//! stands: its host entries and what it waits for
//! (`Shared::sample_hitches`). One that ends over the limit writes
//! `ffx-host: hitch: update U took N ms; main thread: <place> (k of n
//! samples), ...` to stderr (`ffx-rust.log`; the first [`PRINTED`]), and
//! the run's end writes [`summary_line`] (`hitches: …`), which
//! `tools/replays/run-all.sh` prints for each replay.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The limit without `FFX_HOST_HITCH_MS`: three frames of the game's 30
/// a second.
pub const DEFAULT_MS: u64 = 100;
/// Hitches written as they happen; the rest are only counted.
pub const PRINTED: u64 = 200;
/// Hitches the summary keeps (the longest).
pub const WORST: usize = 8;
/// Places a hitch's line names (the most sampled).
const PLACES: usize = 3;

/// What took long.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An update (`FFXApplication_animate`, the main thread).
    Update,
    /// A flip (`Renderer_PresentFlip`, the render thread).
    Flip,
}

impl Kind {
    fn slot(self) -> usize {
        match self {
            Self::Update => 0,
            Self::Flip => 1,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Update => "update",
            Self::Flip => "flip",
        }
    }

    fn thread(self) -> &'static str {
        match self {
            Self::Update => "main thread",
            Self::Flip => "render thread",
        }
    }
}

/// One update or flip over the limit.
#[derive(Debug, Clone, PartialEq)]
pub struct Hitch {
    pub kind: Kind,
    /// The update (frame boundaries before it) or flip (flips before it).
    pub index: u32,
    pub ms: f64,
    /// Where the thread stood at the samples, most sampled first.
    pub places: Vec<(String, u32)>,
    pub samples: u32,
}

impl Hitch {
    /// `update U took N ms; main thread: <place> (k of n samples), …`.
    #[must_use]
    pub fn line(&self) -> String {
        let places = if self.samples == 0 {
            "no sample (shorter than half the limit plus a poll)".to_owned()
        } else {
            let mut v: Vec<String> = self
                .places
                .iter()
                .take(PLACES)
                .map(|(p, n)| format!("{p} ({n} of {} samples)", self.samples))
                .collect();
            if self.places.len() > PLACES {
                v.push(format!("{} more places", self.places.len() - PLACES));
            }
            v.join("; ")
        };
        format!(
            "{} {} took {:.1} ms; {}: {places}",
            self.kind.name(),
            self.index,
            self.ms,
            self.kind.thread()
        )
    }

    /// `update U N ms (in <the most sampled place>)`, for the summary.
    fn short(&self) -> String {
        let at = self
            .places
            .first()
            .map_or_else(|| "no sample".to_owned(), |(p, _)| format!("in {p}"));
        format!(
            "{} {} {:.1} ms ({} {at})",
            self.kind.name(),
            self.index,
            self.ms,
            self.kind.thread()
        )
    }
}

/// An update or flip under way.
#[derive(Debug, Default)]
struct Running {
    index: u32,
    since: Option<Instant>,
    places: Vec<(String, u32)>,
    samples: u32,
}

/// The check's state: the limit, what runs, the counts and the longest.
#[derive(Debug)]
pub struct Hitches {
    limit: Duration,
    running: [Running; 2],
    count: [u64; 2],
    worst: Vec<Hitch>,
}

impl Hitches {
    #[must_use]
    pub fn new(limit: Duration) -> Self {
        Self {
            limit,
            running: Default::default(),
            count: [0; 2],
            worst: Vec::new(),
        }
    }

    /// An update or flip starts at `now`.
    pub fn begin(&mut self, kind: Kind, index: u32, now: Instant) {
        self.running[kind.slot()] = Running {
            index,
            since: Some(now),
            places: Vec::new(),
            samples: 0,
        };
    }

    /// Whether the one under way has run long enough at `now` to be
    /// sampled (half the limit).
    #[must_use]
    pub fn wants_sample(&self, kind: Kind, now: Instant) -> bool {
        self.running[kind.slot()]
            .since
            .is_some_and(|t| now.saturating_duration_since(t) >= self.limit / 2)
    }

    /// Where its thread stands now.
    pub fn sample(&mut self, kind: Kind, place: String) {
        let r = &mut self.running[kind.slot()];
        if r.since.is_none() {
            return;
        }
        r.samples += 1;
        match r.places.iter_mut().find(|(p, _)| *p == place) {
            Some((_, n)) => *n += 1,
            None => r.places.push((place, 1)),
        }
    }

    /// It ends at `now`: the hitch, when it took the limit or longer.
    pub fn end(&mut self, kind: Kind, now: Instant) -> Option<Hitch> {
        let r = std::mem::take(&mut self.running[kind.slot()]);
        let took = now.saturating_duration_since(r.since?);
        if took < self.limit {
            return None;
        }
        let mut places = r.places;
        // most sampled first; equal counts keep the order first seen
        places.sort_by_key(|p| std::cmp::Reverse(p.1));
        let h = Hitch {
            kind,
            index: r.index,
            ms: took.as_secs_f64() * 1e3,
            places,
            samples: r.samples,
        };
        self.count[kind.slot()] += 1;
        let at = self.worst.partition_point(|w| w.ms >= h.ms);
        if at < WORST {
            self.worst.insert(at, h.clone());
            self.worst.truncate(WORST);
        }
        Some(h)
    }

    /// Hitches so far (updates, flips).
    #[must_use]
    pub fn counts(&self) -> (u64, u64) {
        (self.count[0], self.count[1])
    }

    /// `hitches: N over L ms (updates U, flips F); worst: …`.
    #[must_use]
    pub fn summary(&self) -> String {
        let (u, f) = self.counts();
        let mut line = format!(
            "hitches: {} over {} ms (updates {u}, flips {f})",
            u + f,
            self.limit.as_millis()
        );
        if !self.worst.is_empty() {
            let worst: Vec<String> = self.worst.iter().map(Hitch::short).collect();
            line.push_str("; worst: ");
            line.push_str(&worst.join(", "));
        }
        line
    }
}

/// The run's check (`None` until the first use, or when off).
static STATE: Mutex<Option<Hitches>> = Mutex::new(None);

/// The limit: `FFX_HOST_HITCH_MS` (0: off), else [`DEFAULT_MS`].
#[must_use]
pub fn limit() -> Option<Duration> {
    static LIMIT: std::sync::OnceLock<Option<Duration>> = std::sync::OnceLock::new();
    *LIMIT.get_or_init(|| parse_limit(std::env::var("FFX_HOST_HITCH_MS").ok().as_deref()))
}

/// `FFX_HOST_HITCH_MS`'s value as a limit.
fn parse_limit(v: Option<&str>) -> Option<Duration> {
    match v.and_then(|v| v.trim().parse::<u64>().ok()) {
        Some(0) => None,
        Some(ms) => Some(Duration::from_millis(ms)),
        None => Some(Duration::from_millis(DEFAULT_MS)),
    }
}

/// Runs `f` on the run's check, when it is on.
fn with<R>(f: impl FnOnce(&mut Hitches) -> R) -> Option<R> {
    let limit = limit()?;
    let mut s = STATE.lock().ok()?;
    Some(f(s.get_or_insert_with(|| Hitches::new(limit))))
}

/// An update (`index`: the frame boundaries before it) or a flip starts.
pub fn begin(kind: Kind, index: u32) {
    with(|h| h.begin(kind, index, Instant::now()));
}

/// It ends: a hitch is written to stderr (the first [`PRINTED`]).
pub fn end(kind: Kind) {
    let Some((hitch, n)) = with(|h| {
        let hitch = h.end(kind, Instant::now())?;
        let (u, f) = h.counts();
        Some((hitch, u + f))
    })
    .flatten() else {
        return;
    };
    if n <= PRINTED {
        eprintln!("ffx-host: hitch: {}", hitch.line());
    }
    if n == PRINTED {
        eprintln!("ffx-host: hitch: {PRINTED} written; the rest are only counted (the summary)");
    }
}

/// Whether the watchdog should note where the thread of the one under way
/// stands.
#[must_use]
pub fn due(kind: Kind) -> bool {
    with(|h| h.wants_sample(kind, Instant::now())).unwrap_or(false)
}

/// Notes where its thread stands.
pub fn sample(kind: Kind, place: String) {
    with(|h| h.sample(kind, place));
}

/// The run's line: `hitches: …` (or `hitches: off`).
#[must_use]
pub fn summary_line() -> String {
    with(|h| h.summary()).unwrap_or_else(|| "hitches: off (FFX_HOST_HITCH_MS=0)".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn a_frame_under_the_limit_is_no_hitch() {
        let mut h = Hitches::new(100 * MS);
        let t = Instant::now();
        h.begin(Kind::Update, 7, t);
        assert!(!h.wants_sample(Kind::Update, t + 49 * MS));
        assert_eq!(h.end(Kind::Update, t + 99 * MS), None);
        assert_eq!(h.counts(), (0, 0));
        assert_eq!(h.summary(), "hitches: 0 over 100 ms (updates 0, flips 0)");
    }

    #[test]
    fn a_hitch_names_where_its_thread_stood_most() {
        let mut h = Hitches::new(100 * MS);
        let t = Instant::now();
        h.begin(Kind::Update, 36431, t);
        assert!(h.wants_sample(Kind::Update, t + 50 * MS));
        for p in ["Sleep <- a", "Wait <- b", "Wait <- b"] {
            h.sample(Kind::Update, p.to_owned());
        }
        let hitch = h.end(Kind::Update, t + 250 * MS).expect("a hitch");
        assert_eq!(hitch.index, 36431);
        assert_eq!(hitch.samples, 3);
        assert_eq!(
            hitch.places,
            [("Wait <- b".to_owned(), 2), ("Sleep <- a".to_owned(), 1)]
        );
        assert_eq!(
            hitch.line(),
            "update 36431 took 250.0 ms; main thread: Wait <- b (2 of 3 samples); \
             Sleep <- a (1 of 3 samples)"
        );
        // the next one starts clean
        h.begin(Kind::Update, 36432, t);
        assert_eq!(
            h.end(Kind::Update, t + 300 * MS).map(|x| x.samples),
            Some(0)
        );
    }

    #[test]
    fn updates_and_flips_are_kept_apart_and_the_summary_lists_the_longest() {
        let mut h = Hitches::new(100 * MS);
        let t = Instant::now();
        h.begin(Kind::Update, 1, t);
        h.begin(Kind::Flip, 9, t);
        h.sample(Kind::Flip, "Present".to_owned());
        assert!(h.end(Kind::Flip, t + 150 * MS).is_some());
        assert!(h.end(Kind::Update, t + 120 * MS).is_some());
        // a sample with nothing under way is dropped
        h.sample(Kind::Update, "nowhere".to_owned());
        assert_eq!(h.counts(), (1, 1));
        assert_eq!(
            h.summary(),
            "hitches: 2 over 100 ms (updates 1, flips 1); worst: flip 9 150.0 ms \
             (render thread in Present), update 1 120.0 ms (main thread no sample)"
        );
    }

    #[test]
    fn the_summary_keeps_only_the_longest() {
        let mut h = Hitches::new(100 * MS);
        let t = Instant::now();
        for i in 0..20u32 {
            h.begin(Kind::Update, i, t);
            h.end(Kind::Update, t + (100 + i * 7 % 20) * MS);
        }
        assert_eq!(h.counts(), (20, 0));
        assert_eq!(h.worst.len(), WORST);
        assert!(h.worst.windows(2).all(|w| w[0].ms >= w[1].ms));
        assert!((h.worst[0].ms - 119.0).abs() < 1e-6);
    }

    #[test]
    fn the_limit_comes_from_the_variable() {
        assert_eq!(parse_limit(None), Some(Duration::from_millis(DEFAULT_MS)));
        assert_eq!(parse_limit(Some("0")), None);
        assert_eq!(parse_limit(Some("250")), Some(250 * MS));
        assert_eq!(
            parse_limit(Some("junk")),
            Some(Duration::from_millis(DEFAULT_MS))
        );
    }
}
