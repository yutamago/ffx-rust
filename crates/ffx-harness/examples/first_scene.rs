//! Boots the game into the first scene of a new game and leaves it
//! running, for exploring with `tools/harness.py`.
fn main() -> ffx_harness::Result<()> {
    let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
    println!("{}", ffx_harness::screens::first_scene(&mut game)?);
    game.wait(120)?;
    let base = u32::from_le_bytes(
        game.read_through(0x0132_6b28, 0x2c, 4)?
            .try_into()
            .expect("4"),
    );
    println!(
        "globals at {base:#x}, byte 0xa3c = {:#x}",
        game.read_abs(base + 0xa3c, 1)?[0]
    );
    game.events()?;
    let mut chr: Vec<&String> = game
        .opened()
        .iter()
        .filter(|e| e.contains("/chr/"))
        .collect();
    chr.sort();
    chr.dedup();
    for e in chr {
        println!("{e}");
    }
    game.detach();
    Ok(())
}
