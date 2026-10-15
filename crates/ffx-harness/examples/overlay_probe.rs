//! `overlay_probe <out dir> <step>...`: loads save 1 with the renderer and
//! runs the steps, to try the overlay. Steps: `shot:<name>` (a PNG in the
//! out dir), `wait:<frames>`, `click:<x>,<y>` (a left click in the
//! overlay, back buffer pixels), `overlay` (F10), `cmd:<harness command>`.
use std::path::PathBuf;

fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().expect("out dir"));
    std::fs::create_dir_all(&out)?;
    let options = ffx_harness::Options {
        renderer: true,
        ..ffx_harness::Options::default()
    };
    let mut game = ffx_harness::Game::launch(&options)?;
    println!("in {}", ffx_harness::screens::load_save(&mut game, 1)?);
    game.wait(240)?;
    for step in args {
        let (what, arg) = step.split_once(':').unwrap_or((step.as_str(), ""));
        match what {
            "shot" => game
                .screenshot()?
                .save_png(&out.join(format!("{arg}.png")))?,
            "wait" => game.wait(arg.parse().unwrap_or(30))?,
            "click" => {
                let (x, y) = arg.split_once(',').expect("x,y");
                println!("{}", game.command(&format!("overlayclick {x} {y}"))?);
                game.wait(10)?;
            }
            "overlay" => {
                println!("{}", game.command("overlay")?);
                game.wait(10)?;
            }
            "cmd" => println!("{}", game.command(arg)?),
            _ => eprintln!("unknown step {step}"),
        }
    }
    game.quit()
}
