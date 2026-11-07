//! The wide call path (`Host::call_wide`) on the game's host
//! (`ffx_bridge::RawMem`), through a recording router: the words the
//! hooked game and `ffx-host`'s executor are handed.

use std::ptr::NonNull;

use ffx_bridge::route::{self, Args, Float, Raw, Router, Scratch};
use ffx_graphic::host::{Host, WIDE_WORDS};

/// Records the stack words of each call.
struct Record(Vec<Vec<u32>>);

impl Router for Record {
    fn call(&mut self, _: u32, _: route::Conv, args: &Args, _: Float) -> Raw {
        self.0.push(args.stack.clone());
        Raw {
            eax: 7,
            ..Raw::default()
        }
    }
    fn call_x87(&mut self, _: u32, _: &[f64], _: Float) -> f64 {
        0.0
    }
    fn read_bytes(&mut self, _: u32, out: &mut [u8]) {
        out.fill(0);
    }
    fn write_bytes(&mut self, _: u32, _: &[u8]) {}
    fn scratch(&mut self, _: Scratch) -> u32 {
        0
    }
    fn frame_alloc(&mut self, _: u32) -> u32 {
        0
    }
    fn frame_free(&mut self, _: u32) {}
}

/// `call_wide` passes `Phyre_PScripting_Print`'s 36-word `sprintf_s`
/// (buffer, size, format, the matrix, 16 doubles) whole, padded with
/// zeros to [`WIDE_WORDS`], and refuses a longer list without calling.
#[test]
fn call_wide_passes_forty_words() {
    let mut rec = Record(Vec::new());
    let mut h = ffx_bridge::RawMem { image: 0x0040_0000 };
    let words: Vec<u32> = (1..=41).collect();
    {
        let p = NonNull::from(&mut rec as &mut dyn Router);
        // SAFETY: `rec` outlives the guard and is used only through it.
        let _g = unsafe { route::enter(p) };
        assert_eq!(h.call_wide(0x1000, &words[..36]), 7);
        assert_eq!(h.call_wide(0x1000, &words[..WIDE_WORDS]), 7);
        assert_eq!(h.call_wide(0x1000, &words), 0);
    }
    assert_eq!(WIDE_WORDS, 40);
    assert_eq!(rec.0.len(), 2, "the 41-word call is refused");
    assert_eq!(rec.0[0].len(), WIDE_WORDS);
    assert_eq!(rec.0[0][..36], words[..36]);
    assert!(rec.0[0][36..].iter().all(|&w| w == 0));
    assert_eq!(rec.0[1], words[..WIDE_WORDS]);
}
