//! Like `load_save`, but saves a screenshot at every step, to see where
//! booting into a save stops: `load_debug <out dir>`. Renderer on when
//! `FFX_RENDERER=1`.
use ffx_harness::screens;

fn main() -> ffx_harness::Result<()> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("out dir"));
    std::fs::create_dir_all(&out)?;
    let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
    let r = game.wait_for_open("titl00", 30, 3000);
    println!("titl00: {r:?}");
    for i in 0..40 {
        let shot = game.screenshot()?;
        if screens::is_title(&shot) {
            println!("title after {i} presses");
            break;
        }
        if i % 5 == 0 {
            shot.save_png(&out.join(format!("pre{i:02}.png")))?;
        }
        game.command("press start 3 60")?;
    }
    game.command("key 1c 3 30")?;
    for i in 0..20 {
        let shot = game.screenshot()?;
        if screens::is_save_list(&shot) {
            println!("save list after {i}");
            break;
        }
        shot.save_png(&out.join(format!("list{i:02}.png")))?;
        game.wait(30)?;
    }
    game.quit()
}
