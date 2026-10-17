//! Loads a save one way and writes memory snapshots, to compare the title
//! menu's Load with the direct load (`loadsave`, docs/formats/save.md):
//!
//! ```text
//! load_compare menu <entry> <out prefix (windows)> [frames]
//! load_compare direct <slot or windows path> <out prefix (windows)> [frames]
//! ```
//!
//! Runs with the virtual clock; once the save's event opens, lockstep
//! holds the frames, and `<prefix>-0.snap` is taken there, `<prefix>-1.snap`
//! after the map's first fade-in (`frames`, default 120, more frames). Prints
//! the seconds from launch to the event and the frame numbers.
use std::time::Instant;

use ffx_harness::{Game, Options, Result, screens};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: load_compare menu|direct <entry|save> <out prefix> [frames]";
    let (mode, save, out) = match args.as_slice() {
        [m, s, o, ..] => (m.as_str(), s.as_str(), o.as_str()),
        _ => panic!("{usage}"),
    };
    let frames: u32 = args.get(3).map_or(120, |f| f.parse().expect("frames"));
    let options = Options {
        virtual_clock: true,
        ..Options::default()
    };
    let launched = Instant::now();
    let mut game = Game::launch(&options)?;
    let booted = launched.elapsed();
    let start = Instant::now();
    let opened = match mode {
        "menu" => screens::load_save(&mut game, save.parse().expect("entry"))?,
        "direct" => {
            game.wait_for_open("titl00/bin/mapout.vpa", 2, 3000)?;
            println!(
                "title map open after {:.1} s, frame {}",
                start.elapsed().as_secs_f64(),
                game.frame()?
            );
            screens::load_save_direct(&mut game, save)?
        }
        _ => panic!("{usage}"),
    };
    let loaded = start.elapsed();
    let at = game.command("lockstep on")?;
    println!(
        "{mode} {save}: {opened}; launch {:.1} s, load {:.1} s, event at frame {at}",
        booted.as_secs_f64(),
        loaded.as_secs_f64()
    );
    println!(
        "snapshot 0: {}",
        game.command(&format!("snapshot {out}-0.snap"))?
    );
    game.wait(frames)?;
    println!(
        "snapshot 1 at {}: {}",
        game.frame()?,
        game.command(&format!("snapshot {out}-1.snap"))?
    );
    game.quit()
}
