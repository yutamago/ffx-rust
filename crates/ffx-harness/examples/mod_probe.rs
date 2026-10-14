//! Plays into the first scene with one mod file in place and reports
//! whether the game read it and survived ten seconds:
//! `mod_probe <archive path> <local file> [screenshot dir]`; with a
//! directory, a screenshot every 30 frames goes there.
fn main() -> ffx_harness::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (path, local) = (&args[1], &args[2]);
    let dir = std::path::PathBuf::from(std::env::var_os("FFX_GAME_DIR").expect("FFX_GAME_DIR"))
        .join("mods/zz-harness-probe");
    let file = dir.join(path);
    std::fs::create_dir_all(file.parent().expect("parent"))?;
    std::fs::copy(local, &file)?;
    let result = (|| -> ffx_harness::Result<String> {
        let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
        ffx_harness::screens::first_scene(&mut game)?;
        match args.get(3) {
            Some(shots) => {
                std::fs::create_dir_all(shots)?;
                for i in 0..40 {
                    game.wait(30)?;
                    game.screenshot()?
                        .save_png(&std::path::Path::new(shots).join(format!("{i:02}.png")))?;
                }
            }
            None => game.wait(600)?,
        }
        game.events()?;
        let read = game.opened().iter().any(|e| *e == format!("mod {path}"));
        game.quit()?;
        Ok(format!("survived; read from the mod: {read}"))
    })();
    std::fs::remove_dir_all(&dir)?;
    println!("{}", result.unwrap_or_else(|e| format!("failed: {e}")));
    Ok(())
}
