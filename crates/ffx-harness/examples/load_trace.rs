//! Loads a save through the title menu (`load_trace [entry]`), or with
//! `loadsave` (`load_trace direct [save] [early]`, frame by frame in
//! lockstep: once the title scene is set up as `screens::load_save_direct`
//! waits, or with `early` as soon as the title event opens, which the
//! pending boot jump undoes), and prints every change of the
//! save/load state globals, with the frame it was seen on, until the save's
//! map script opens and a while after: which steps each path takes
//! (docs/formats/save.md).
use ffx_harness::{Game, Result, screens};

const WATCH: &[(&str, u32, usize)] = &[
    ("screen_state 0x00ccb994", 0x00cc_b994, 4),
    ("dialog_state 0x00ccb998", 0x00cc_b998, 4),
    ("is_save 0x00ccb99c", 0x00cc_b99c, 4),
    ("machine 0x00ce72e8", 0x00ce_72e8, 4),
    ("machine_on 0x00ce7310", 0x00ce_7310, 4),
    ("cursor 0x00ce72ec/f0", 0x00ce_72ec, 8),
    ("menu_reserve 0x012fbc38", 0x012f_bc38, 8),
    ("loaded_flag 0x012fb8c0", 0x012f_b8c0, 4),
    ("sl_active 0x018663c4", 0x0186_63c4, 4),
    ("jump_req 0x0133084c", 0x0133_084c, 4),
    ("fadeout 0x01330850", 0x0133_0850, 8),
    ("map 0x0112ca90", 0x0112_ca90, 4),
    ("entrance 0x0112ca9c", 0x0112_ca9c, 2),
    ("saved_loc 0x0112cb48", 0x0112_cb48, 4),
    ("cur_scene 0x00c33480", 0x00c3_3480, 4),
    ("scene 0x01330844", 0x0133_0844, 4),
    ("buf_head 0x01597f70", 0x0159_7f70, 0x20),
    ("scenario 0x0112d67c", 0x0112_d67c, 2),
    ("ctrl0 flags 0x01325ba0", 0x0132_5ba0, 4),
    ("ctrl0 +0x210 0x01325db0", 0x0132_5db0, 4),
    ("fade 0x01330854", 0x0133_0854, 4),
];

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("direct") {
        return direct(
            args.get(1).map_or("1", String::as_str),
            args.get(2).map(String::as_str) == Some("early"),
        );
    }
    let entry: u32 = args.first().map_or(1, |e| e.parse().expect("entry"));
    let mut game = Game::launch(&ffx_harness::Options::default())?;
    screens::to_title(&mut game)?;
    let mut last: Vec<Vec<u8>> = vec![Vec::new(); WATCH.len()];
    let mut snap = |game: &mut Game, tag: &str| -> Result<()> {
        let f = game.frame()?;
        for (i, (name, at, len)) in WATCH.iter().enumerate() {
            let v = game.read(*at, *len)?;
            if v != last[i] {
                let hex: String = v.iter().map(|b| format!("{b:02x}")).collect();
                println!("{f:6} {tag:8} {name:28} {hex}");
                last[i] = v;
            }
        }
        for e in game.events()? {
            println!("{f:6} {tag:8} event {e}");
        }
        Ok(())
    };
    snap(&mut game, "title")?;
    game.command("key 1c 3 30")?;
    snap(&mut game, "enter")?;
    for _ in 0..40 {
        game.wait(2)?;
        snap(&mut game, "list")?;
    }
    for _ in 0..entry {
        game.command("key d0 3 20")?;
        snap(&mut game, "down")?;
    }
    game.command("key 1c 3 40")?;
    snap(&mut game, "pick")?;
    game.command("key 1c 3 1")?;
    snap(&mut game, "yes")?;
    let mut seen_ebp = 0;
    for _ in 0..1500 {
        game.wait(1)?;
        snap(&mut game, "load")?;
        if game.opened().iter().any(|e| e.contains(".ebp")) {
            seen_ebp += 1;
            if seen_ebp > 200 {
                break;
            }
        }
    }
    game.quit()
}

fn direct(save: &str, early: bool) -> Result<()> {
    let mut game = Game::launch(&ffx_harness::Options {
        virtual_clock: true,
        ..ffx_harness::Options::default()
    })?;
    game.wait_for_open("test20.ebp", 1, 3000)?;
    if !early {
        game.wait_for_open("titl00/bin/mapout.vpa", 1, 3000)?;
        game.command("waitfor 0x0133084c:4 00000000 ffffffff 600")?;
        game.wait(30)?;
    }
    game.command("lockstep on")?;
    let mut last: Vec<Vec<u8>> = vec![Vec::new(); WATCH.len()];
    let mut snap = |game: &mut Game, tag: &str| -> Result<()> {
        let f = game.frame()?;
        for (i, (name, at, len)) in WATCH.iter().enumerate() {
            let v = game.read(*at, *len)?;
            if v != last[i] {
                let hex: String = v.iter().map(|b| format!("{b:02x}")).collect();
                println!("{f:6} {tag:8} {name:28} {hex}");
                last[i] = v;
            }
        }
        for e in game.events()? {
            if e.contains(".ebp") || e.contains("mapout") {
                println!("{f:6} {tag:8} event {e}");
            }
        }
        Ok(())
    };
    snap(&mut game, "open")?;
    println!("loadsave: {}", game.command(&format!("loadsave {save}"))?);
    snap(&mut game, "loaded")?;
    for _ in 0..600 {
        game.wait(1)?;
        snap(&mut game, "frame")?;
    }
    game.quit()
}
