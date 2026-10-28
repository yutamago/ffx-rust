//! Replays an Iggy call log (`FFX_IGGY_LOG`, `crates/ffx-hook/src/iggy_log.rs`)
//! into Ruffle and writes what Ruffle shows at each of the log's
//! screenshots:
//!
//! ```text
//! iggy-replay <log> <out dir> [--vbf <FFX_Data.vbf>] [--no-pace] [--smooth]
//!     [--quality <q>] [--ss <k>] [--force-smooth] [--mipmaps] [--virtual-clock]
//!     [--no-font]
//! ```
//!
//! Each Iggy player of the log becomes a Ruffle player of the same movie
//! (matched by size and hash among the `.swf` files of the player's
//! `FFX_Data.vbf`, `FFX_GAME_DIR/data/FFX_Data.vbf` by default); the game's
//! AS3 calls, the ticks and the display size are replayed in log order, the
//! movie's external calls are answered with the game's recorded answers.
//! At each draw of an update a screenshot was taken of, the movie's image
//! is kept; at the screenshot the kept images are put over each other in
//! draw order into `<out>/<shot>-ruffle.png` (straight alpha, transparent
//! where nothing is drawn). `<out>/replay.txt` lists the movies, the calls
//! Ruffle did not know, and the external calls Ruffle made against the
//! recorded ones.
//!
//! `--quality` sets Ruffle's stage quality; `--ss k` draws at k times the
//! size and box-filters down (`ffx_swf::stage::Render`); `--force-smooth`
//! draws every bitmap smoothed, `--mipmaps` mipmaps them and `--virtual-clock` puts `getTimer` on the
//! player's clock (both need the Ruffle fork build; the latter replaces
//! pacing). `--smooth` marks every bitmap fill smoothed before Ruffle loads the movie
//! (`ffx_swf::patch`), as Iggy draws them.
//!
//! Ruffle's `getTimer` reads the wall clock, so the replay is paced to the
//! game's time (the update count at 30 a second, the game's update rate) unless `--no-pace`.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use ffx_swf::log::{Event, shot_name};
use ffx_swf::stage::{GameFont, Gpu, Stage};
use image::RgbaImage;

fn fnv(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
    }
    h
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let log_path = PathBuf::from(
        args.next()
            .context("usage: iggy-replay <log> <out dir> [--vbf F] [--no-pace]")?,
    );
    let out = PathBuf::from(args.next().context("out dir")?);
    let mut vbf =
        std::env::var_os("FFX_GAME_DIR").map(|d| PathBuf::from(d).join("data/FFX_Data.vbf"));
    let mut pace = true;
    let mut smooth = false;
    let mut no_font = false;
    let mut render = ffx_swf::stage::Render::default();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--vbf" => vbf = args.next().map(PathBuf::from),
            "--no-pace" => pace = false,
            "--smooth" => smooth = true,
            "--force-smooth" => ffx_swf::stage::force_bitmap_smoothing(true),
            "--no-font" => no_font = true,
            "--mipmaps" => ffx_swf::stage::gdraw_mipmaps(true),
            "--virtual-clock" => {
                ffx_swf::stage::deterministic_get_timer(true);
                pace = false;
            }
            "--quality" => render.quality = args.next(),
            "--ss" => render.ss = args.next().and_then(|v| v.parse().ok()).unwrap_or(1),
            _ => bail!("unknown argument {a}"),
        }
    }
    let vbf = vbf.context("no FFX_Data.vbf (set FFX_GAME_DIR or pass --vbf)")?;
    std::fs::create_dir_all(&out)?;
    let movies = ffx_swf::flash_movies(&vbf)?;
    let font = movies
        .iter()
        .filter(|_| !no_font)
        .find(|(n, _)| n == "box.swf")
        .map(|(_, d)| GameFont::from_swf(d))
        .transpose()?;
    let events = ffx_swf::log::parse(&std::fs::read_to_string(&log_path)?)?;
    let shots: HashSet<u32> = events
        .iter()
        .filter_map(|e| match e {
            Event::Shot { update, .. } => *update,
            _ => None,
        })
        .collect();
    // the game's answers per player and name, in order
    let mut answers: HashMap<u32, HashMap<String, VecDeque<Option<String>>>> = HashMap::new();
    let mut recorded: HashMap<u32, Vec<String>> = HashMap::new();
    for e in &events {
        if let Event::Callback {
            player,
            name,
            answer,
            ..
        } = e
        {
            answers
                .entry(*player)
                .or_default()
                .entry(name.clone())
                .or_default()
                .push_back(answer.clone());
            recorded.entry(*player).or_default().push(name.clone());
        }
    }
    let gpu = Gpu::new()?;
    let mut report = String::new();
    let mut stages: HashMap<u32, (String, Stage)> = HashMap::new();
    let mut finished: Vec<(u32, String, Stage)> = Vec::new();
    let mut kept: HashMap<u32, Vec<(u32, RgbaImage)>> = HashMap::new();
    let mut unknown: Vec<String> = Vec::new();
    let start = Instant::now();
    let mut first_update: Option<u32> = None;
    for e in &events {
        match e {
            Event::Create { player, size, hash } => {
                let Some((name, data)) = movies
                    .iter()
                    .find(|(_, d)| d.len() == *size as usize && fnv(d) == *hash)
                else {
                    let _ = writeln!(report, "player {player:x}: no movie of {size} bytes");
                    continue;
                };
                // The start-up logos: the harness skips them (docs/harness.md).
                if name == "openingscreen.swf" {
                    let _ = writeln!(
                        report,
                        "player {player:x}: {name} (skipped, the start-up logos)"
                    );
                    continue;
                }
                let patched;
                let data = if smooth {
                    let (d, n) = ffx_swf::patch::smooth_bitmap_fills(data)?;
                    let _ = writeln!(report, "{name}: {n} bitmap fills marked smoothed");
                    patched = d;
                    &patched
                } else {
                    data
                };
                let stage = Stage::new(
                    &gpu,
                    data,
                    &format!("file:///{name}"),
                    (1280, 720),
                    font.as_ref(),
                    &render,
                )?;
                if let Some(a) = answers.remove(player) {
                    stage.answers.borrow_mut().queued = a;
                }
                let _ = writeln!(report, "player {player:x}: {name}");
                stages.insert(*player, (name.clone(), stage));
            }
            Event::Destroy { player } => {
                if let Some((name, s)) = stages.remove(player) {
                    finished.push((*player, name, s));
                }
            }
            Event::DisplaySize { player, w, h } => {
                if let Some((_, s)) = stages.get_mut(player) {
                    s.set_size(*w, *h);
                }
            }
            Event::Tick { player, update, .. } => {
                if pace && let Some(u) = update {
                    let first = *first_update.get_or_insert(*u);
                    let due = start + Duration::from_secs_f64(f64::from(u - first.min(*u)) / 30.0);
                    if let Some(wait) = due.checked_duration_since(Instant::now()) {
                        std::thread::sleep(wait);
                    }
                }
                if let Some((_, s)) = stages.get_mut(player) {
                    s.tick();
                }
            }
            Event::Draw { player, update } => {
                if let Some(u) = update
                    && shots.contains(u)
                    && let Some((_, s)) = stages.get_mut(player)
                {
                    let img = s.capture()?;
                    let list = kept.entry(*u).or_default();
                    list.retain(|(p, _)| p != player);
                    list.push((*player, img));
                }
            }
            Event::Call {
                player, name, args, ..
            } => {
                if let Some((movie, s)) = stages.get_mut(player) {
                    let r = s.call(name, args);
                    if r == ruffle_core::external::Value::Null && !unknown.iter().any(|u| u == name)
                    {
                        // Ruffle answers Null for a name with no callback (and
                        // logs "Calling unknown internal interface").
                        unknown.push(format!("{name} ({movie})"));
                    }
                }
            }
            Event::MouseMove { player, x, y } => {
                if let Some((_, s)) = stages.get_mut(player) {
                    s.mouse_move(*x, *y);
                }
            }
            Event::Callback { .. } => {}
            Event::Shot { path, update } => {
                let name = shot_name(path);
                let mut img = RgbaImage::new(1280, 720);
                if let Some(u) = update
                    && let Some(list) = kept.get(u)
                {
                    for (_, layer) in list {
                        if layer.dimensions() == img.dimensions() {
                            ffx_swf::compose::over(&mut img, layer);
                        }
                    }
                }
                img.save(out.join(format!("{name}-ruffle.png")))?;
                eprintln!(
                    "{name}: update {update:?}, {} layers",
                    kept.get(&update.unwrap_or(u32::MAX)).map_or(0, Vec::len)
                );
            }
        }
    }
    let _ = writeln!(
        report,
        "calls Ruffle answered with null (no callback, or one returning null): {unknown:?}"
    );
    for (player, name, s) in finished
        .iter()
        .map(|(p, n, s)| (p, n, s))
        .chain(stages.iter().map(|(p, (n, s))| (p, n, s)))
    {
        let asked: Vec<String> = s
            .answers
            .borrow()
            .asked
            .iter()
            .map(|c| c.name.clone())
            .collect();
        let rec = recorded.get(player).cloned().unwrap_or_default();
        let _ = writeln!(
            report,
            "{name} ({player:x}): {} ticks; external calls: Ruffle {} / Iggy {}{}",
            s.ticks,
            asked.len(),
            rec.len(),
            if asked == rec {
                " (same names, same order)"
            } else {
                ""
            }
        );
        if asked != rec {
            let _ = writeln!(report, "  Ruffle: {asked:?}");
            let _ = writeln!(report, "  Iggy:   {rec:?}");
        }
    }
    std::fs::write(out.join("replay.txt"), &report)?;
    print!("{report}");
    Ok(())
}
