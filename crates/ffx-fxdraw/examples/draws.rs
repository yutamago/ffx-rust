//! `draws <capture> [frame]`: one line per draw of a capture's frame
//! (targets, shaders, counts, which slots are rings), for looking at a
//! capture before the census explains it.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("usage: draws <capture> [frame]")?;
    let frame: usize = args.next().map_or(Ok(0), |f| f.parse())?;
    let c = ffx_fxdraw::capture::Capture::read(std::path::Path::new(&path))?;
    let f = c.frames.get(frame).ok_or("no such frame")?;
    // FFX_GAME_DIR set: shader families from the archive.
    let fam = match std::env::var_os("FFX_GAME_DIR") {
        Some(g) => ffx_fxdraw::shaders::Families::from_game(std::path::Path::new(&g))?,
        None => ffx_fxdraw::shaders::Families::default(),
    };
    println!(
        "scene {} frame {} ({}x{}), {} draws",
        c.scene,
        f.number,
        f.width,
        f.height,
        f.draws.len()
    );
    let mut others = f.others.iter().peekable();
    for d in &f.draws {
        while let Some((cmd, what)) = others.peek() {
            if *cmd > d.cmd {
                break;
            }
            println!("      {what}");
            others.next();
        }
        let targets: Vec<String> = d
            .targets
            .iter()
            .map(|&(s, r)| format!("{s}:{}", c.texture_label(r)))
            .collect();
        let slots: Vec<String> = d
            .slots
            .iter()
            .map(|s| {
                format!(
                    "{}{}/{}{}",
                    s.slot,
                    if s.ring { "r" } else { "s" },
                    s.stride,
                    if s.per_instance { "i" } else { "" }
                )
            })
            .collect();
        println!(
            "{:4} {:?} n{} t{} vs {:016x} ps {:016x} {}/{} rt [{}] ds {} vb [{}] ib{}",
            d.n,
            d.draw.kind,
            d.draw.count,
            d.draw.topology,
            d.vs,
            d.ps,
            fam.name(d.vs),
            fam.name(d.ps),
            targets.join(" "),
            if d.depth_target == 0 { "-" } else { "y" },
            slots.join(" "),
            if d.ring_indices { "r" } else { "s" },
        );
    }
    Ok(())
}
