//! Controlled measurements of FMOD Ex's mixer through the game's own
//! `fmod_event.dll`/`fmodex.dll` (32-bit Windows console program; no game,
//! no window, no sound device: FMOD's WAV writer in non-real-time mode, set
//! up as the harness's capture does, `crates/ffx-hook/src/audio_wav.rs`).
//!
//! ```bash
//! PATH=tools/shims:$PATH XWIN_ARCH=x86,x86_64 cargo xwin build --release \
//!   --target i686-pc-windows-msvc -p ffx-sound --example fmod-probe
//! fmod-probe.exe <game dir> <media dir> <script> <out.wav>
//! ```
//!
//! `<media dir>` holds the `.fev` and `.fsb` files the script loads (copied
//! from the archive; FMOD opens them itself, with `setMediaPath`). The
//! script, one command a line (`#` comments):
//!
//! ```text
//! load P <file.fev>            project P
//! group G P <name>             group G of project P (getGroup, cacheevents)
//! event E G <index>            an instance E (getEventByIndex, mode 0) with the end callback
//! start E | stop E <0|1> | pause E <0|1> | vol E <f> | pan E <f>
//! sound S <file.fsb> <mode>    System::createSound (0x200 = compressed sample)
//! sub S2 S <index>             Sound::getSubSound
//! play C S2                    System::playSound(CHANNEL_REUSE) into channel C
//! cvol C <f> | cpan C <f> | cpause C <0|1> | cstop C
//! update [n]                   n EventSystem::update calls (one block each)
//! ```
//!
//! Every update prints `update <n> clock <dsp clock>`, every event callback
//! `callback <update> <type> <instance>` (the update it ran in: the update
//! counter is raised before the call) and every event's `getState` after
//! each update that changed it: `state <update> <instance> <state>`.

#[cfg(all(windows, target_arch = "x86"))]
mod probe {
    use std::collections::HashMap;
    use std::ffi::{CString, c_void};
    use std::sync::atomic::{AtomicU32, Ordering};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryA(name: *const u8) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
        fn SetDllDirectoryA(path: *const u8) -> i32;
    }

    const EV: bool = true;
    const EX: bool = false;

    type F1 = extern "system" fn(usize) -> u32;
    type F2 = extern "system" fn(usize, usize) -> u32;
    type F3 = extern "system" fn(usize, usize, usize) -> u32;
    type F4 = extern "system" fn(usize, usize, usize, usize) -> u32;
    type F5 = extern "system" fn(usize, usize, usize, usize, usize) -> u32;
    type F6 = extern "system" fn(usize, usize, usize, usize, usize, usize) -> u32;
    type F7 = extern "system" fn(usize, usize, usize, usize, usize, usize, usize) -> u32;

    /// Calls a stdcall function of 1 to 7 words.
    fn call(f: usize, w: &[usize]) -> u32 {
        // SAFETY: an FMOD export or vtable entry called with the words its
        // signature takes (stdcall, `this` first).
        unsafe {
            match *w {
                [a] => std::mem::transmute::<usize, F1>(f)(a),
                [a, b] => std::mem::transmute::<usize, F2>(f)(a, b),
                [a, b, c] => std::mem::transmute::<usize, F3>(f)(a, b, c),
                [a, b, c, d] => std::mem::transmute::<usize, F4>(f)(a, b, c, d),
                [a, b, c, d, e] => std::mem::transmute::<usize, F5>(f)(a, b, c, d, e),
                [a, b, c, d, e, g] => std::mem::transmute::<usize, F6>(f)(a, b, c, d, e, g),
                [a, b, c, d, e, g, h] => std::mem::transmute::<usize, F7>(f)(a, b, c, d, e, g, h),
                _ => panic!("arity"),
            }
        }
    }

    static UPDATE: AtomicU32 = AtomicU32::new(0);

    extern "system" fn callback(
        _event: usize,
        kind: u32,
        _p1: usize,
        _p2: usize,
        user: usize,
    ) -> u32 {
        println!("callback {} {kind} {user}", UPDATE.load(Ordering::Relaxed));
        0
    }

    struct Fmod {
        event: *mut c_void,
        ex: *mut c_void,
    }

    impl Fmod {
        fn x(&self, dll: bool, name: &str, w: &[usize]) -> u32 {
            let m = if dll { self.event } else { self.ex };
            let c = CString::new(name).expect("name");
            // SAFETY: a loaded module and a NUL-terminated name.
            let f = unsafe { GetProcAddress(m, c.as_ptr().cast()) } as usize;
            assert!(f != 0, "missing export {name}");
            let r = call(f, w);
            if r != 0 {
                println!("error {name} {r}");
            }
            r
        }
    }

    fn vcall(this: usize, slot: usize, w: &[usize]) -> u32 {
        // SAFETY: `this` is an FMOD interface object; its first word is the
        // vtable.
        let f = unsafe { *((*(this as *const usize) + slot) as *const usize) };
        let mut all = vec![this];
        all.extend_from_slice(w);
        let r = call(f, &all);
        if r != 0 {
            println!("error vtable+{slot:#x} {r}");
        }
        r
    }

    fn f32w(s: &str) -> usize {
        s.parse::<f32>().expect("float").to_bits() as usize
    }

    #[allow(clippy::too_many_lines)]
    pub fn main() {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let [game, media, script, out] = args.as_slice() else {
            eprintln!("usage: fmod-probe <game dir> <media dir> <script> <out.wav>");
            std::process::exit(2);
        };
        let g = CString::new(game.as_str()).expect("dir");
        // SAFETY: plain Win32 calls with NUL-terminated strings.
        let fm = unsafe {
            SetDllDirectoryA(g.as_ptr().cast());
            Fmod {
                ex: LoadLibraryA(c"fmodex.dll".as_ptr().cast()),
                event: LoadLibraryA(c"fmod_event.dll".as_ptr().cast()),
            }
        };
        assert!(!fm.ex.is_null() && !fm.event.is_null(), "FMOD DLLs");
        let a = |p: *mut usize| p as usize;
        let mut es = 0usize;
        fm.x(EV, "FMOD_EventSystem_Create", &[a(&raw mut es)]);
        let mut sys = 0usize;
        fm.x(
            EV,
            "?getSystemObject@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@PAPAVSystem@2@@Z",
            &[es, a(&raw mut sys)],
        );
        // the game's and the capture's set-up (audio_wav.rs), in their order
        fm.x(
            EX,
            "?setStreamBufferSize@System@FMOD@@QAG?AW4FMOD_RESULT@@II@Z",
            &[sys, 0x10000, 8],
        );
        fm.x(
            EX,
            "?setOutput@System@FMOD@@QAG?AW4FMOD_RESULT@@W4FMOD_OUTPUTTYPE@@@Z",
            &[sys, 5],
        );
        let mut f = [0usize; 6];
        fm.x(
            EX,
            "?getSoftwareFormat@System@FMOD@@QAG?AW4FMOD_RESULT@@PAHPAW4FMOD_SOUND_FORMAT@@00PAW4FMOD_DSP_RESAMPLER@@0@Z",
            &[sys, a(&raw mut f[0]), a(&raw mut f[1]), a(&raw mut f[2]), a(&raw mut f[3]), a(&raw mut f[4]), a(&raw mut f[5])],
        );
        println!("software format before {f:?}");
        fm.x(
            EX,
            "?setSoftwareFormat@System@FMOD@@QAG?AW4FMOD_RESULT@@HW4FMOD_SOUND_FORMAT@@HHW4FMOD_DSP_RESAMPLER@@@Z",
            &[sys, f[0], 5, f[2], f[3], f[4]],
        );
        fm.x(
            EX,
            "?setDSPBufferSize@System@FMOD@@QAG?AW4FMOD_RESULT@@IH@Z",
            &[sys, f[0] / 30, 4],
        );
        let path = CString::new(out.as_str()).expect("out");
        fm.x(
            EV,
            "?init@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@HIPAXI@Z",
            &[es, 128, 1, path.as_ptr() as usize, 0],
        );
        let mut m = media.clone();
        if !m.ends_with('\\') {
            m.push('\\');
        }
        let mc = CString::new(m).expect("media");
        fm.x(
            EV,
            "?setMediaPath@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@PBD@Z",
            &[es, mc.as_ptr() as usize],
        );

        let mut h: HashMap<String, usize> = HashMap::new();
        let mut users: HashMap<String, usize> = HashMap::new();
        let mut states: HashMap<String, u32> = HashMap::new();
        let mut keep: Vec<CString> = Vec::new();
        let text = std::fs::read_to_string(script).expect("script");
        for line in text.lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            if w.is_empty() || w[0].starts_with('#') {
                continue;
            }
            let get = |h: &HashMap<String, usize>, k: &str| {
                *h.get(k).unwrap_or_else(|| panic!("unknown {k}"))
            };
            match w[0] {
                "load" => {
                    let c = CString::new(w[2]).expect("name");
                    let mut p = 0usize;
                    fm.x(
                        EV,
                        "?load@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@PBDPAUFMOD_EVENT_LOADINFO@@PAPAVEventProject@2@@Z",
                        &[es, c.as_ptr() as usize, 0, a(&raw mut p)],
                    );
                    h.insert(w[1].into(), p);
                    keep.push(c);
                }
                "group" => {
                    let c = CString::new(w[3]).expect("name");
                    let mut grp = 0usize;
                    // EventProject::getGroup (+8)
                    vcall(get(&h, w[2]), 8, &[c.as_ptr() as usize, 1, a(&raw mut grp)]);
                    h.insert(w[1].into(), grp);
                    keep.push(c);
                }
                "event" => {
                    let mut e = 0usize;
                    // EventGroup::getEventByIndex (+0x24)
                    vcall(
                        get(&h, w[2]),
                        0x24,
                        &[w[3].parse().expect("index"), 0, a(&raw mut e)],
                    );
                    let user = users.len() + 1;
                    users.insert(w[1].into(), user);
                    fm.x(
                        EV,
                        "?setCallback@Event@FMOD@@QAG?AW4FMOD_RESULT@@P6G?AW43@PAUFMOD_EVENT@@W4FMOD_EVENT_CALLBACKTYPE@@PAX22@Z2@Z",
                        &[e, callback as *const () as usize, user],
                    );
                    println!("event {} {} handle {e:x} user {user}", w[1], w[3]);
                    h.insert(w[1].into(), e);
                }
                "start" => {
                    fm.x(
                        EV,
                        "?start@Event@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
                        &[get(&h, w[1])],
                    );
                }
                "stop" => {
                    fm.x(
                        EV,
                        "?stop@Event@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z",
                        &[get(&h, w[1]), w[2].parse().expect("bool")],
                    );
                }
                "pause" => {
                    fm.x(
                        EV,
                        "?setPaused@Event@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z",
                        &[get(&h, w[1]), w[2].parse().expect("bool")],
                    );
                }
                "vol" => {
                    fm.x(
                        EV,
                        "?setVolume@Event@FMOD@@QAG?AW4FMOD_RESULT@@M@Z",
                        &[get(&h, w[1]), f32w(w[2])],
                    );
                }
                "pan" => {
                    let e = get(&h, w[1]);
                    let mut cg = 0usize;
                    fm.x(
                        EV,
                        "?getChannelGroup@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAPAVChannelGroup@2@@Z",
                        &[e, a(&raw mut cg)],
                    );
                    let mut n = 0usize;
                    fm.x(
                        EX,
                        "?getNumChannels@ChannelGroup@FMOD@@QAG?AW4FMOD_RESULT@@PAH@Z",
                        &[cg, a(&raw mut n)],
                    );
                    for i in 0..n {
                        let mut ch = 0usize;
                        fm.x(
                            EX,
                            "?getChannel@ChannelGroup@FMOD@@QAG?AW4FMOD_RESULT@@HPAPAVChannel@2@@Z",
                            &[cg, i, a(&raw mut ch)],
                        );
                        fm.x(
                            EX,
                            "?setPan@Channel@FMOD@@QAG?AW4FMOD_RESULT@@M@Z",
                            &[ch, f32w(w[2])],
                        );
                    }
                    if n != 1 {
                        println!("pan {}: {n} channels", w[1]);
                    }
                }
                "sound" => {
                    let c = CString::new(format!("{media}\\{}", w[2])).expect("name");
                    let mut s = 0usize;
                    fm.x(
                        EX,
                        "?createSound@System@FMOD@@QAG?AW4FMOD_RESULT@@PBDIPAUFMOD_CREATESOUNDEXINFO@@PAPAVSound@2@@Z",
                        &[sys, c.as_ptr() as usize, usize::from_str_radix(w[3].trim_start_matches("0x"), 16).expect("mode"), 0, a(&raw mut s)],
                    );
                    h.insert(w[1].into(), s);
                    keep.push(c);
                }
                "sub" => {
                    let mut s = 0usize;
                    fm.x(
                        EX,
                        "?getSubSound@Sound@FMOD@@QAG?AW4FMOD_RESULT@@HPAPAV12@@Z",
                        &[get(&h, w[2]), w[3].parse().expect("index"), a(&raw mut s)],
                    );
                    h.insert(w[1].into(), s);
                }
                "play" => {
                    let mut ch = h.get(w[1]).copied().unwrap_or(0);
                    fm.x(
                        EX,
                        "?playSound@System@FMOD@@QAG?AW4FMOD_RESULT@@W4FMOD_CHANNELINDEX@@PAVSound@2@_NPAPAVChannel@2@@Z",
                        &[sys, (-2i32) as usize, get(&h, w[2]), 0, a(&raw mut ch)],
                    );
                    h.insert(w[1].into(), ch);
                }
                "cvol" => {
                    fm.x(
                        EX,
                        "?setVolume@Channel@FMOD@@QAG?AW4FMOD_RESULT@@M@Z",
                        &[get(&h, w[1]), f32w(w[2])],
                    );
                }
                "cpan" => {
                    fm.x(
                        EX,
                        "?setPan@Channel@FMOD@@QAG?AW4FMOD_RESULT@@M@Z",
                        &[get(&h, w[1]), f32w(w[2])],
                    );
                }
                "cpause" => {
                    fm.x(
                        EX,
                        "?setPaused@Channel@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z",
                        &[get(&h, w[1]), w[2].parse().expect("bool")],
                    );
                }
                "cstop" => {
                    fm.x(
                        EX,
                        "?stop@Channel@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
                        &[get(&h, w[1])],
                    );
                }
                "update" => {
                    let n: u32 = w.get(1).map_or(1, |n| n.parse().expect("count"));
                    for _ in 0..n {
                        let u = UPDATE.fetch_add(1, Ordering::Relaxed) + 1;
                        fm.x(
                            EV,
                            "?update@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
                            &[es],
                        );
                        let (mut hi, mut lo) = (0usize, 0usize);
                        fm.x(
                            EX,
                            "?getDSPClock@System@FMOD@@QAG?AW4FMOD_RESULT@@PAI0@Z",
                            &[sys, a(&raw mut hi), a(&raw mut lo)],
                        );
                        println!("update {u} clock {}", ((hi as u64) << 32) | lo as u64);
                        for (k, &user) in &users {
                            let mut st = 0usize;
                            let e = h[k];
                            let r = call_state(&fm, e, &mut st);
                            let st = if r == 0 {
                                u32::try_from(st).unwrap_or(0)
                            } else {
                                0xdead_0000 | r
                            };
                            if states.get(k) != Some(&st) {
                                println!("state {u} {user} {st:#x}");
                                states.insert(k.clone(), st);
                            }
                        }
                    }
                }
                other => panic!("unknown command {other}"),
            }
        }
        fm.x(
            EV,
            "?release@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
            &[es],
        );
    }

    fn call_state(fm: &Fmod, e: usize, st: &mut usize) -> u32 {
        let c = c"?getState@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAI@Z";
        // SAFETY: a loaded module and a NUL-terminated name.
        let f = unsafe { GetProcAddress(fm.event, c.as_ptr().cast()) } as usize;
        call(f, &[e, std::ptr::from_mut(st) as usize])
    }
}

#[cfg(all(windows, target_arch = "x86"))]
fn main() {
    probe::main();
}

#[cfg(not(all(windows, target_arch = "x86")))]
fn main() {
    eprintln!(
        "fmod-probe runs as a 32-bit Windows program only (cargo xwin, i686-pc-windows-msvc)"
    );
}
