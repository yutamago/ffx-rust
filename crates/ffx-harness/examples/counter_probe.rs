//! Looks for per-frame counters in FFX.exe's `.data`/`.bss` after a field
//! load: `counter_probe [save] [frames]` (default 1, 120) loads the save the
//! way the field replay does (`load direct`, lockstep, original code), reads
//! the image's data when the load returns, one frame later, `frames` frames
//! later and one after that, and prints every aligned u32 that counted up
//! by one over the last frame, with how far it counted since the load
//! (fewer than `frames`: it started late, with the event or the map). The
//! field-walk screenshots are pinned to one found this way, ctrl work 0's
//! frame count `0x01325d94` (`tools/replays/field-walk.txt`).
const DATA: u32 = 0x00c0_a000;
const DATA_LEN: u32 = 0x019c_c2c4; // .data's virtual size, .bss included
const CHUNK: u32 = 0x10_0000;

/// The image's data; unreadable chunks read as zeros.
fn dump(game: &mut ffx_harness::Game) -> Vec<u8> {
    let mut out = Vec::with_capacity(DATA_LEN as usize);
    let mut at = DATA;
    while at < DATA + DATA_LEN {
        let len = CHUNK.min(DATA + DATA_LEN - at) as usize;
        match game.read(at, len) {
            Ok(bytes) => out.extend_from_slice(&bytes),
            Err(_) => out.resize(out.len() + len, 0),
        }
        at += CHUNK;
    }
    out
}

fn word(bytes: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]])
}

fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let save = args.next().unwrap_or_else(|| "1".into());
    let frames: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(120);
    let options = ffx_harness::Options {
        virtual_clock: true,
        hook_disable: Some("replacements".into()),
        ..ffx_harness::Options::default()
    };
    let mut game = ffx_harness::Game::launch(&options)?;
    let script = format!("load direct {save}\nlockstep on\n");
    eprintln!(
        "{:?}",
        ffx_harness::replay::play(&mut game, &script, "C:\\nul")?
    );
    let start = game.frame()?;
    let at_load = dump(&mut game);
    game.wait(frames)?;
    let before = dump(&mut game);
    game.wait(1)?;
    let after = dump(&mut game);
    eprintln!("frames {start} .. {}", game.frame()?);
    for i in (0..at_load.len() - 3).step_by(4) {
        let (first, last) = (word(&before, i), word(&after, i));
        if last.wrapping_sub(first) == 1 {
            println!(
                "{:#010x} counted {} since the load ({} then, {last} now)",
                DATA as usize + i,
                last.wrapping_sub(word(&at_load, i)),
                word(&at_load, i)
            );
        }
    }
    game.quit()
}
