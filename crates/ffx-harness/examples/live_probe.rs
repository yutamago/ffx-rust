//! Boots save `entry` (default 1) with the external renderer on, waits
//! for the field and saves screenshots: `live_probe <out dir> [entry]
//! [frames between shots] [shots]`. `FFX_RENDER_TEST=invert` makes the
//! renderer's image stand out.
fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().expect("out dir"));
    let entry = args.next().map_or(1, |e| e.parse().expect("entry"));
    let gap = args.next().map_or(60, |e| e.parse().expect("frames"));
    let shots: u32 = args.next().map_or(3, |e| e.parse().expect("shots"));
    std::fs::create_dir_all(&out)?;
    let options = ffx_harness::Options {
        renderer: true,
        ..ffx_harness::Options::default()
    };
    let mut game = ffx_harness::Game::launch(&options)?;
    println!("in {}", ffx_harness::screens::load_save(&mut game, entry)?);
    game.wait(240)?;
    for i in 0..shots {
        game.screenshot()?
            .save_png(&out.join(format!("shot{i}.png")))?;
        println!("{}", game.command("gfxstats")?);
        game.wait(gap)?;
    }
    game.quit()
}
