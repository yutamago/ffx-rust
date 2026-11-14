//! A sampling profiler for the headless boot (m5-perf1; docs/host.md,
//! "Speed"): no `perf` on the development machine, so the run samples
//! itself. `FFX_HOST_SAMPLE=<prefix>` turns it on ([`start`]); [`Sampler::finish`]
//! writes two TSV files:
//!
//! - `<prefix>.entries.tsv`: every 1 ms of wall time, each game thread's
//!   host entries ([`crate::exec::spot`]): `self` counts the innermost
//!   entry, `incl` every entry on the chain once. Where the threads are,
//!   waiting or not, by port, import and trap.
//! - `<prefix>.pc.tsv` (Linux): every 1 ms of CPU time (`ITIMER_PROF`),
//!   the interrupted instruction's address, as an offset into the object
//!   that holds it (`tools/host-profile.py` names them with `nm`). Where
//!   the CPU goes, function by function, the executor's own code included.
//!   `FFX_HOST_SAMPLE_MAIN=1` keeps only the main thread's samples (the
//!   thread that started the sampler), the one the frame rate waits on;
//!   `FFX_HOST_SAMPLE_THREAD=<id>` one game thread's ([`thread_started`]).
//!
//! With the sampler on, every game thread notes its CPU time as it ends
//! (`sample: thread ... used ... s of CPU`).
//!
//! A development aid only: nothing samples unless the variable is set.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::exec::Shared;
use crate::exec::spot::Kind;

/// Host-entry samples: (kind, what) -> (self, inclusive).
type Counts = HashMap<(u8, u32), (u64, u64)>;

/// A running sampler ([`start`]).
#[derive(Debug)]
pub struct Sampler {
    prefix: String,
    done: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<Counts>>,
}

/// Starts sampling when `FFX_HOST_SAMPLE` is set.
#[must_use]
pub fn start(shared: &Arc<Shared>) -> Option<Sampler> {
    let prefix = std::env::var("FFX_HOST_SAMPLE").ok()?;
    let done = Arc::new(AtomicBool::new(false));
    let weak = Arc::downgrade(shared);
    let stop = Arc::clone(&done);
    let thread = std::thread::Builder::new()
        .name("ffx-sample".into())
        .spawn(move || entries(&weak, &stop))
        .ok();
    pc::start();
    Some(Sampler {
        prefix,
        done,
        thread,
    })
}

/// The calling thread's CPU time so far, when sampling (Linux): the
/// game threads note theirs as they end, the main thread at
/// [`Sampler::finish`].
#[must_use]
pub fn thread_cpu() -> Option<Duration> {
    std::env::var_os("FFX_HOST_SAMPLE")?;
    pc::thread_cpu()
}

/// A game thread starts on this host thread: with
/// `FFX_HOST_SAMPLE_THREAD=<id>` (hex, `0x1014`) the CPU sampler keeps only
/// that thread's samples.
pub fn thread_started(id: u32) {
    let Ok(want) = std::env::var("FFX_HOST_SAMPLE_THREAD") else {
        return;
    };
    if u32::from_str_radix(want.trim_start_matches("0x"), 16) == Ok(id) {
        pc::only_this_thread();
    }
}

/// The host-entry sampler's loop: (kind, what) -> (self, inclusive).
fn entries(shared: &std::sync::Weak<Shared>, done: &AtomicBool) -> Counts {
    let mut counts = Counts::new();
    // the threads the CPU sampler keeps (all, the main one, or one)
    let only: Option<u32> = if std::env::var("FFX_HOST_SAMPLE_MAIN").is_ok_and(|v| v == "1") {
        Some(crate::exec::MAIN_THREAD)
    } else {
        std::env::var("FFX_HOST_SAMPLE_THREAD")
            .ok()
            .and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok())
    };
    while !done.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(1));
        let Some(s) = shared.upgrade() else { break };
        let spots = s.spots.lock().map(|l| l.clone()).unwrap_or_default();
        drop(s);
        for spot in spots {
            if only.is_some_and(|id| id != spot.id) {
                continue;
            }
            let (frames, _) = spot.frames();
            let Some(inner) = frames.last() else {
                counts.entry((0, spot.id)).or_default().0 += 1;
                continue;
            };
            counts.entry((kind(inner.kind), inner.what)).or_default().0 += 1;
            let mut seen: Vec<(u8, u32)> = Vec::with_capacity(frames.len());
            for f in &frames {
                let k = (kind(f.kind), f.what);
                if !seen.contains(&k) {
                    seen.push(k);
                    counts.entry(k).or_default().1 += 1;
                }
            }
        }
    }
    counts
}

fn kind(k: Kind) -> u8 {
    match k {
        Kind::Port => 1,
        Kind::Import => 2,
        Kind::Trap => 3,
    }
}

impl Sampler {
    /// Stops sampling and writes the files (names from `shared`).
    pub fn finish(mut self, shared: &Shared) {
        if let Some(t) = thread_cpu() {
            eprintln!("sample: main thread used {:.2} s of CPU", t.as_secs_f64());
        }
        self.done.store(true, Ordering::Relaxed);
        pc::stop();
        let counts = self
            .thread
            .take()
            .and_then(|t| t.join().ok())
            .unwrap_or_default();
        let mut rows: Vec<_> = counts.into_iter().collect();
        rows.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(b.1.1.cmp(&a.1.1)));
        let mut out = String::from("# kind\twhat\tname\tself\tincl\n");
        for ((k, what), (own, incl)) in rows {
            let (kind, name) = match k {
                1 => ("port", shared.port_name(what).unwrap_or("").to_string()),
                2 => (
                    "import",
                    shared.import_name(what as usize).unwrap_or("").to_string(),
                ),
                3 => ("trap", String::new()),
                _ => ("none", format!("thread {what:#x} outside host entries")),
            };
            let _ = writeln!(out, "{kind}\t{what:#010x}\t{name}\t{own}\t{incl}");
        }
        let _ = std::fs::write(format!("{}.entries.tsv", self.prefix), out);
        if let Some(pcs) = pc::report() {
            let _ = std::fs::write(format!("{}.pc.tsv", self.prefix), pcs);
        }
    }
}

/// The CPU-time sampler: `SIGPROF` records the interrupted `rip` in a
/// lock-free table.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod pc {
    use std::fmt::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    const SLOTS: usize = 1 << 18;
    static KEYS: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];
    static COUNTS: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];
    static LOST: AtomicU64 = AtomicU64::new(0);
    /// Only this thread's samples (`FFX_HOST_SAMPLE_MAIN=1`: the thread
    /// that started the sampler, the game's main thread), else 0: all.
    static ONLY: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
    /// (address, the word at `rsp`) pairs: for a leaf function (libc's
    /// `memcpy`, `memset`) the word is its caller's return address.
    static PAIR_KEYS: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];
    static PAIR_RIP: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];
    static PAIR_RET: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];
    static PAIR_COUNTS: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];

    /// Counts `key` in `keys`/`counts`; the slot, or `None` when full.
    fn count(keys: &[AtomicU64; SLOTS], counts: &[AtomicU64; SLOTS], key: u64) -> Option<usize> {
        #[allow(clippy::cast_possible_truncation)] // a hash
        let mut i = (key.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 46) as usize % SLOTS;
        for _ in 0..64 {
            let k = keys[i].load(Ordering::Relaxed);
            if k == key
                || (k == 0
                    && keys[i]
                        .compare_exchange(0, key, Ordering::Relaxed, Ordering::Relaxed)
                        .map_or_else(|now| now == key, |_| true))
            {
                counts[i].fetch_add(1, Ordering::Relaxed);
                return Some(i);
            }
            i = (i + 1) % SLOTS;
        }
        None
    }

    extern "C" fn on_prof(_: i32, _: *mut libc::siginfo_t, ctx: *mut libc::c_void) {
        // SAFETY: the kernel hands a SA_SIGINFO handler its ucontext; the
        // interrupted thread's `rsp` points into its (mapped) stack.
        #[allow(clippy::cast_sign_loss)] // a register's bits
        let (rip, ret) = unsafe {
            let uc = ctx.cast::<libc::ucontext_t>();
            let g = &(*uc).uc_mcontext.gregs;
            let rsp = g[libc::REG_RSP as usize] as u64;
            let ret = if rsp == 0 {
                0
            } else {
                (rsp as *const u64).read_volatile()
            };
            (g[libc::REG_RIP as usize] as u64, ret)
        };
        if rip == 0 {
            return;
        }
        let only = ONLY.load(Ordering::Relaxed);
        // SAFETY: gettid is async-signal-safe.
        if only != 0 && unsafe { libc::syscall(libc::SYS_gettid) } != only {
            return;
        }
        if count(&KEYS, &COUNTS, rip).is_none() {
            LOST.fetch_add(1, Ordering::Relaxed);
        }
        let pair = (rip ^ ret.rotate_left(29)).max(1);
        if let Some(i) = count(&PAIR_KEYS, &PAIR_COUNTS, pair) {
            PAIR_RIP[i].store(rip, Ordering::Relaxed);
            PAIR_RET[i].store(ret, Ordering::Relaxed);
        }
    }

    fn timer(us: i64) {
        let t = libc::itimerval {
            it_interval: libc::timeval {
                tv_sec: 0,
                tv_usec: us,
            },
            it_value: libc::timeval {
                tv_sec: 0,
                tv_usec: us,
            },
        };
        // SAFETY: plain system calls.
        unsafe {
            libc::setitimer(libc::ITIMER_PROF, &raw const t, std::ptr::null_mut());
        }
    }

    pub(super) fn start() {
        if std::env::var("FFX_HOST_SAMPLE_MAIN").is_ok_and(|v| v == "1") {
            only_this_thread();
        }
        // SAFETY: installs a handler that only touches atomics.
        unsafe {
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = on_prof as *const () as usize;
            sa.sa_flags = libc::SA_SIGINFO | libc::SA_RESTART;
            libc::sigemptyset(&raw mut sa.sa_mask);
            libc::sigaction(libc::SIGPROF, &raw const sa, std::ptr::null_mut());
        }
        timer(1000);
    }

    pub(super) fn stop() {
        timer(0);
    }

    pub(super) fn only_this_thread() {
        // SAFETY: a plain system call.
        ONLY.store(
            unsafe { libc::syscall(libc::SYS_gettid) },
            Ordering::Relaxed,
        );
    }

    pub(super) fn thread_cpu() -> Option<std::time::Duration> {
        let mut t = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: a plain system call into a local.
        let ok = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &raw mut t) } == 0;
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // 0..1e9 ns
        ok.then(|| std::time::Duration::new(t.tv_sec as u64, t.tv_nsec as u32))
    }

    /// The object holding `addr`, the offset into it and the nearest
    /// exported symbol.
    fn place(addr: u64) -> (String, u64, String) {
        let text = |p: *const libc::c_char| {
            if p.is_null() {
                String::new()
            } else {
                // SAFETY: dladdr hands NUL-terminated names.
                unsafe { std::ffi::CStr::from_ptr(p) }
                    .to_string_lossy()
                    .into_owned()
            }
        };
        // SAFETY: dladdr only reads the loader's tables.
        unsafe {
            let mut info: libc::Dl_info = std::mem::zeroed();
            if libc::dladdr(addr as *const libc::c_void, &raw mut info) == 0 {
                (String::from("?"), addr, String::new())
            } else {
                (
                    text(info.dli_fname),
                    addr - info.dli_fbase as u64,
                    text(info.dli_sname),
                )
            }
        }
    }

    /// `object\toffset\tsymbol\tcount` rows, most samples first; then,
    /// after a `# pairs` line, `object\toffset\tsymbol\tret object\tret
    /// offset\tcount` rows.
    #[allow(clippy::unnecessary_wraps)] // None off Linux
    pub(super) fn report() -> Option<String> {
        let mut rows: Vec<(u64, u64)> = (0..SLOTS)
            .filter_map(|i| {
                let k = KEYS[i].load(Ordering::Relaxed);
                (k != 0).then(|| (k, COUNTS[i].load(Ordering::Relaxed)))
            })
            .collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.1));
        let mut out = format!(
            "# object\toffset\tsymbol\tcount (lost {})\n",
            LOST.load(Ordering::Relaxed)
        );
        for (rip, n) in rows {
            let (obj, off, sym) = place(rip);
            let _ = writeln!(out, "{obj}\t{off:#x}\t{sym}\t{n}");
        }
        let mut pairs: Vec<(u64, u64, u64)> = (0..SLOTS)
            .filter(|&i| PAIR_KEYS[i].load(Ordering::Relaxed) != 0)
            .map(|i| {
                (
                    PAIR_RIP[i].load(Ordering::Relaxed),
                    PAIR_RET[i].load(Ordering::Relaxed),
                    PAIR_COUNTS[i].load(Ordering::Relaxed),
                )
            })
            .collect();
        pairs.sort_by_key(|r| std::cmp::Reverse(r.2));
        out.push_str("# pairs\n");
        for (rip, ret, n) in pairs.into_iter().take(20_000) {
            let (obj, off, sym) = place(rip);
            let (robj, roff, _) = place(ret);
            let _ = writeln!(out, "{obj}\t{off:#x}\t{sym}\t{robj}\t{roff:#x}\t{n}");
        }
        Some(out)
    }
}

/// No CPU-time sampler off Linux.
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod pc {
    pub(super) fn start() {}
    pub(super) fn stop() {}
    pub(super) fn only_this_thread() {}
    pub(super) fn report() -> Option<String> {
        None
    }
    pub(super) fn thread_cpu() -> Option<std::time::Duration> {
        None
    }
}
