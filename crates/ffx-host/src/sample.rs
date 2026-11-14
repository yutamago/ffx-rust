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
//!
//! A development aid only: nothing samples unless the variable is set.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::exec::Shared;
use crate::exec::spot::Kind;

/// A running sampler ([`start`]).
#[derive(Debug)]
pub struct Sampler {
    prefix: String,
    done: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<HashMap<(u8, u32), (u64, u64)>>>,
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

/// The host-entry sampler's loop: (kind, what) -> (self, inclusive).
fn entries(
    shared: &std::sync::Weak<Shared>,
    done: &AtomicBool,
) -> HashMap<(u8, u32), (u64, u64)> {
    let mut counts: HashMap<(u8, u32), (u64, u64)> = HashMap::new();
    while !done.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(1));
        let Some(s) = shared.upgrade() else { break };
        let spots = s.spots.lock().map(|l| l.clone()).unwrap_or_default();
        drop(s);
        for spot in spots {
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
                    shared
                        .import_name(what as usize)
                        .unwrap_or("")
                        .to_string(),
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

    extern "C" fn on_prof(_: i32, _: *mut libc::siginfo_t, ctx: *mut libc::c_void) {
        // SAFETY: the kernel hands a SA_SIGINFO handler its ucontext.
        let rip = unsafe {
            let uc = ctx.cast::<libc::ucontext_t>();
            #[allow(clippy::cast_sign_loss)] // a register's bits
            let r = (*uc).uc_mcontext.gregs[libc::REG_RIP as usize] as u64;
            r
        };
        if rip == 0 {
            return;
        }
        #[allow(clippy::cast_possible_truncation)] // a hash
        let mut i = (rip.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 46) as usize % SLOTS;
        for _ in 0..64 {
            let k = KEYS[i].load(Ordering::Relaxed);
            if k == rip
                || (k == 0
                    && KEYS[i]
                        .compare_exchange(0, rip, Ordering::Relaxed, Ordering::Relaxed)
                        .map_or_else(|now| now == rip, |_| true))
            {
                COUNTS[i].fetch_add(1, Ordering::Relaxed);
                return;
            }
            i = (i + 1) % SLOTS;
        }
        LOST.fetch_add(1, Ordering::Relaxed);
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
        // SAFETY: installs a handler that only touches atomics.
        unsafe {
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = on_prof as usize;
            sa.sa_flags = libc::SA_SIGINFO | libc::SA_RESTART;
            libc::sigemptyset(&raw mut sa.sa_mask);
            libc::sigaction(libc::SIGPROF, &raw const sa, std::ptr::null_mut());
        }
        timer(1000);
    }

    pub(super) fn stop() {
        timer(0);
    }

    /// `object\toffset\tsymbol\tcount` rows, most samples first.
    pub(super) fn report() -> Option<String> {
        let mut rows: Vec<(u64, u64)> = (0..SLOTS)
            .filter_map(|i| {
                let k = KEYS[i].load(Ordering::Relaxed);
                (k != 0).then(|| (k, COUNTS[i].load(Ordering::Relaxed)))
            })
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        let mut out = format!(
            "# object\toffset\tsymbol\tcount (lost {})\n",
            LOST.load(Ordering::Relaxed)
        );
        for (rip, n) in rows {
            // SAFETY: dladdr only reads the loader's tables.
            let (obj, off, sym) = unsafe {
                let mut info: libc::Dl_info = std::mem::zeroed();
                if libc::dladdr(rip as *const libc::c_void, &raw mut info) == 0 {
                    (String::from("?"), rip, String::new())
                } else {
                    let obj = if info.dli_fname.is_null() {
                        String::from("?")
                    } else {
                        std::ffi::CStr::from_ptr(info.dli_fname)
                            .to_string_lossy()
                            .into_owned()
                    };
                    let sym = if info.dli_sname.is_null() {
                        String::new()
                    } else {
                        std::ffi::CStr::from_ptr(info.dli_sname)
                            .to_string_lossy()
                            .into_owned()
                    };
                    (obj, rip - info.dli_fbase as u64, sym)
                }
            };
            let _ = writeln!(out, "{obj}\t{off:#x}\t{sym}\t{n}");
        }
        Some(out)
    }
}

/// No CPU-time sampler off Linux.
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod pc {
    pub(super) fn start() {}
    pub(super) fn stop() {}
    pub(super) fn report() -> Option<String> {
        None
    }
}
