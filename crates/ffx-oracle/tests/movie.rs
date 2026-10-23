//! The movie module (`ffx_movie`: the PS2 sample library's
//! `FFXSamp0UnitMatrix`) against the original, on two oracles started
//! alike: one runs `FFX.exe`'s code, the other the Rust bridge on the same
//! memory; results and every written byte must agree. Synthetic cases need
//! `FFX_GAME_DIR`; the snapshot cases also `FFX_SNAPSHOT` (both kept
//! snapshots).

// a, b: the two oracles; m, f, v, x, y: matrix, floats, arguments, results
#![allow(clippy::many_single_char_names)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_movie::{ViewScreen, bridge as mv};
use ffx_oracle::{Conv, Oracle, Pages};

/// The game thread's x87 control word.
const GAME_FPU: u32 = 0x027f;

const SAMP0_UNIT_MATRIX: u32 = 0x0093_d900;
const SAMP0_VIEW_SCREEN_MATRIX: u32 = 0x0093_d960;

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

/// The worlds a test runs in: the image alone, and the snapshot when
/// `FFX_SNAPSHOT` is set.
fn worlds() -> Vec<(&'static str, Oracle, Oracle)> {
    let Some(exe) = exe() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut a = Oracle::load(&exe).unwrap();
    let mut b = Oracle::load(&exe).unwrap();
    a.set_fpu_control(GAME_FPU).unwrap();
    b.set_fpu_control(GAME_FPU).unwrap();
    out.push(("image", a, b));
    if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        out.push((
            "snapshot",
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
        ));
    } else {
        eprintln!("FFX_SNAPSHOT is not set; image cases only");
    }
    out
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// Ordinary floats of mixed magnitudes, special values, quiet NaNs
    /// (the oracle keeps a loaded signalling NaN signalling; the CPU
    /// quiets it, as the port does).
    fn float(&mut self) -> f32 {
        const SPECIAL: [u32; 9] = [
            0,
            0x8000_0000,
            0x7f80_0000,
            0xff80_0000,
            0x7fc0_0000,
            0xffc1_2345,
            1,
            0x7f7f_ffff,
            0x3f80_0000,
        ];
        let bits = match self.next() % 12 {
            0 => self.next(),
            1 => SPECIAL[(self.next() % 9) as usize],
            2 => self.next() | 0x7fc0_0000,
            _ => self.next() & 0x807f_ffff | (self.next() % 40 + 107) << 23,
        };
        f32::from_bits(if f32::from_bits(bits).is_nan() {
            bits | 0x0040_0000
        } else {
            bits
        })
    }
}

/// The bridges' host: the oracle's memory and, for the callees, its
/// emulator.
struct H<'a>(&'a mut Oracle);

impl Mem for H<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

impl ffx_bridge::Host for H<'_> {
    fn call(&mut self, addr: u32, args: &[u32]) -> u32 {
        self.0
            .call(addr, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }
}

#[test]
fn samp0_unit_matrix() {
    let mut rng = Xorshift(0x0e1e_0001);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        // two matrices and a word on each side, all random
        let block = a.alloc(0x88).unwrap();
        assert_eq!(b.alloc(0x88).unwrap(), block);
        for off in [0, 4, 0x40] {
            for i in 0..0x88 / 4 {
                let v = rng.next();
                Mem::write(a, block + 4 * i, v);
                Mem::write(b, block + 4 * i, v);
            }
            let m = block + off;
            let at = a.rebase(SAMP0_UNIT_MATRIX);
            let ret = a.call(at, Conv::Cdecl, &[m]).unwrap();
            assert!(ret.st0.is_none(), "{world}: ST0 left set");
            assert_eq!(ret.eax, mv::samp0_unit_matrix(b, m), "{world}: eax");
            assert_eq!(a.diff(b).unwrap(), [], "{world}: memory");
            assert_eq!(Mem::read::<[f32; 16]>(a, m), ffx_movie::unit_matrix());
        }
        eprintln!("{world}: FFXSamp0UnitMatrix agrees");
    }
}

#[test]
fn samp0_view_screen_matrix() {
    let mut rng = Xorshift(0x0e1e_0002);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        let m = a.alloc(0x40).unwrap();
        assert_eq!(b.alloc(0x40).unwrap(), m);
        // the bridge's stand-in for the original's frame: the bottom of the
        // emulated stack, which the diff leaves out as it does the frames
        let tmp = b.stack_range().start + 0x100;
        for i in 0..3000 {
            for k in 0..16 {
                let v = rng.next();
                Mem::write(a, m + 4 * k, v);
                Mem::write(b, m + 4 * k, v);
            }
            let mut f = [0f32; 9];
            for x in &mut f {
                *x = rng.float();
            }
            if i % 3 == 0 {
                // a typical camera: scrz 512, 640x448 around 2048, depth
                f = [
                    512.0,
                    1.0,
                    0.47,
                    2048.0,
                    2048.0,
                    1.0,
                    16_777_215.0,
                    0.1,
                    65_536.0,
                ];
                f[i % 9] = rng.float();
            } else if i % 3 == 1 {
                // farz * zmin close to nearz * zmax: [14] cancels
                let x = f32::from_bits(rng.next() & 0x007f_ffff | 0x4000_0000);
                let y = f32::from_bits(rng.next() & 0x007f_ffff | 0x4380_0000);
                f[5] = x;
                f[7] = x;
                f[8] = y;
                f[6] = f32::from_bits(y.to_bits() + rng.next() % 5 - 2);
            }
            let v = ViewScreen {
                scrz: f[0],
                ax: f[1],
                ay: f[2],
                cx: f[3],
                cy: f[4],
                zmin: f[5],
                zmax: f[6],
                nearz: f[7],
                farz: f[8],
            };
            let mut args = vec![m];
            args.extend(f.iter().map(|x| x.to_bits()));
            let at = a.rebase(SAMP0_VIEW_SCREEN_MATRIX);
            let ret = a.call(at, Conv::Cdecl, &args).unwrap();
            assert!(ret.st0.is_none(), "{world}: ST0 left set");
            mv::samp0_view_screen_matrix(&mut H(b), m, tmp, &v);
            let (x, y) = (Mem::read::<[u32; 16]>(a, m), Mem::read::<[u32; 16]>(b, m));
            assert_eq!(x, y, "{world} {f:?}: matrix");
            assert_eq!(a.diff(b).unwrap(), [], "{world} {f:?}: memory");
        }
        eprintln!("{world}: FFXSamp0ViewScreenMatrix agrees");
    }
}

// ---- pc round 4: the movie camera (0x0076edc0) ------------------------

/// Free image bytes for the stand-ins (`.reloc`, which the code never
/// reads): code, then the log (a count, then 8-byte entries: tag, first
/// argument) and one answer per stand-in.
const REC_CODE: u32 = 0x0270_0000;
const REC_LOG: u32 = 0x0270_f000;
const REC_RETS: u32 = 0x0270_fc00;

/// Replaces the cdecl original at `sym` with a stand-in (slot `k`) that
/// logs `sym` and its argument (0 for none) and returns its answer.
fn recorder(o: &mut Oracle, k: u32, sym: u32, takes_arg: bool) {
    let log = o.rebase(REC_LOG);
    let ret = o.rebase(REC_RETS) + 4 * k;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0x25, 0xff, 0x00, 0x00, 0x00]); // and eax, 0xff
    c.extend([0x8d, 0x04, 0xc5]);
    c.extend((log + 4).to_le_bytes()); // lea eax, [eax*8 + log + 4]
    c.extend([0xc7, 0x00]);
    c.extend(sym.to_le_bytes()); // mov dword [eax], sym
    if takes_arg {
        c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
    } else {
        c.extend([0x31, 0xd2]); // xor edx, edx
    }
    c.extend([0x89, 0x50, 0x04]); // mov [eax+4], edx
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xa1);
    c.extend(ret.to_le_bytes()); // mov eax, [ret]
    c.push(0xc3);
    let code = o.rebase(REC_CODE) + 0x40 * k;
    o.write(code, &c).unwrap();
    let at = o.rebase(sym);
    let mut j = vec![0xe9];
    j.extend(code.wrapping_sub(at + 5).to_le_bytes());
    o.write(at, &j).unwrap();
}

fn put<T: ffx_bridge::Value>(a: &mut Oracle, b: &mut Oracle, at: u32, v: T) {
    Mem::write(a, at, v);
    Mem::write(b, at, v);
}

#[test]
#[allow(clippy::too_many_lines)] // one case loop, in the order the original reads
fn pc_r4_movie_camera_frame() {
    const MOVIE_CAMERA_FRAME: u32 = 0x0076_edc0;
    const STAND_INS: [u32; 4] = [
        mv::HAVE_CAMERA,
        mv::CURRENT_FRAME,
        mv::SCENE_SET_MATRIX_CROSS,
        mv::CAM_APPLY,
    ];
    const FRAMES: [u32; 11] = [
        0,
        1,
        0x47a,
        0x47b,
        0x47c,
        0x4d7,
        0x4d8,
        0x3c9,
        0x3ca,
        0xffff_ffff,
        0x7fff_ffff,
    ];
    let mut rng = Xorshift(0x0e1e_0401);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for (k, &sym) in (0..).zip(&STAND_INS) {
            recorder(a, k, sym, sym == mv::CAM_APPLY);
            recorder(b, k, sym, sym == mv::CAM_APPLY);
        }
        let records = a.alloc(0x500 * 0xa8).unwrap();
        assert_eq!(b.alloc(0x500 * 0xa8).unwrap(), records);
        let work = a.alloc(0x100).unwrap();
        assert_eq!(b.alloc(0x100).unwrap(), work);
        // the original's frame (ebp - 0xac) under its return address and
        // saved ebp
        let frame = a.stack_range().end - 0x100 - 8 - 0xac;
        for n in 0..3000u32 {
            for i in 0..(0x500 * 0xa8 / 4) {
                if i % 42 == 0 || rng.next().is_multiple_of(64) {
                    let v = match rng.next() % 4 {
                        0 => rng.float(),
                        1 => 1.0,
                        2 => f32::from(u8::try_from(rng.next() % 8).unwrap()),
                        _ => 0.0,
                    };
                    put(a, b, records + 4 * i, v);
                } else if n == 0 {
                    put(a, b, records + 4 * i, rng.float());
                }
            }
            // without records the original reads its never-written frame:
            // 0 there (the port's value, brief 2026-10-24); the port writes
            // its 0s itself, so its work block starts as garbage
            for i in 0..0xa8 / 4 {
                Mem::write(a, frame + 4 * i, 0u32);
                Mem::write(b, work + 4 * i, rng.float());
            }
            let id =
                [0x2e, 0x46, 0x20, 0x48, 0x2e, 0x46, rng.next() % 0x80][(rng.next() % 7) as usize];
            put(a, b, a.rebase(mv::MOVIE_ID), id);
            put(
                a,
                b,
                a.rebase(mv::WAIT_FRAMES_BY_TIME),
                u32::from(!rng.next().is_multiple_of(8)),
            );
            let frames = [
                0x4d9,
                0x4d8,
                0x3cb,
                0x3ca,
                0x47c,
                0x100,
                0,
                rng.next() % 0x500,
            ][(rng.next() % 8) as usize];
            let words = frames * 42 + [0, 1, 41, 0][(rng.next() % 4) as usize];
            let words = if rng.next().is_multiple_of(16) {
                rng.next()
            } else {
                words
            };
            put(a, b, a.rebase(mv::CAMERA_DATA_WORDS), words);
            let data = if rng.next().is_multiple_of(5) {
                0
            } else {
                records
            };
            put(a, b, a.rebase(mv::CAMERA_DATA), data);
            let current = if rng.next().is_multiple_of(3) {
                FRAMES[(rng.next() % 11) as usize]
            } else {
                rng.next() % 0x500
            };
            for (k, v) in [
                u32::from(!rng.next().is_multiple_of(6)),
                current,
                rng.next() % 2,
                rng.next(),
            ]
            .into_iter()
            .enumerate()
            {
                put(a, b, a.rebase(REC_RETS) + 4 * u32::try_from(k).unwrap(), v);
            }
            put(a, b, a.rebase(REC_LOG), 0u32);
            put(a, b, a.rebase(0x012f_b8fb), 1u8);
            let at = a.rebase(MOVIE_CAMERA_FRAME);
            let ra = a
                .call(at, Conv::Cdecl, &[])
                .unwrap_or_else(|e| panic!("{world} {n}: {e}"));
            let rb = mv::movie_camera_frame(&mut H(b), work);
            let log = a.rebase(REC_LOG);
            let count: u32 = Mem::read(a, log);
            let entries = |o: &Oracle| -> Vec<u32> {
                (0..=2 * count).map(|i| Mem::read(o, log + 4 * i)).collect()
            };
            assert_eq!(entries(a), entries(b), "{world} {n}: calls");
            // eax is the last callee's on the camera path; elsewhere it is
            // what the frame and bound checks left (nothing reads it)
            if entries(a).contains(&mv::SCENE_SET_MATRIX_CROSS) {
                assert_eq!(ra.eax, rb, "{world} {n}: eax");
            }
            let frame_words: Vec<u32> =
                (0..0xa8 / 4).map(|i| Mem::read(a, frame + 4 * i)).collect();
            let work_words: Vec<u32> = (0..0xa8 / 4).map(|i| Mem::read(b, work + 4 * i)).collect();
            assert_eq!(frame_words, work_words, "{world} {n}: frame");
            // the work block is the original's stack frame: leave it out
            let diff: Vec<_> = a
                .diff(b)
                .unwrap()
                .into_iter()
                .filter(|d| !(work..work + 0xa8).contains(&d.0))
                .collect();
            assert_eq!(diff, [], "{world} {n}: memory");
        }
    }
}
