//! `drawdiff A B [--part scene|all] [--tolerances F] [--allow F]
//! [--limit N] [--show-allowed] [--suggest-allow]`: compares two captures
//! (`capture.ffxgfx` files, or folders holding one) draw by draw.
//!
//! Exit status: 0 when every difference is allowed, 1 when some are not,
//! 2 on bad arguments or unreadable files.

use std::path::PathBuf;
use std::process::ExitCode;

use ffx_drawdiff::capture::Capture;
use ffx_drawdiff::diff::{Options, Part, compare};
use ffx_drawdiff::report::{Style, render, suggest_allow};
use ffx_drawdiff::rules;

const USAGE: &str = "usage: drawdiff A B [--part scene|all] [--tolerances F] [--allow F] \
                     [--limit N] [--show-allowed] [--suggest-allow]";

fn run() -> Result<bool, String> {
    let mut args = std::env::args().skip(1);
    let mut paths = Vec::new();
    let mut opts = Options::default();
    let mut style = Style::default();
    let mut suggest = false;
    let read_rules = |path: Option<String>| -> Result<Vec<rules::Rule>, String> {
        let path = path.ok_or(USAGE)?;
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        rules::parse(&path, &text)
    };
    while let Some(a) = args.next() {
        match a.as_str() {
            "--part" => {
                opts.part = match args.next().as_deref() {
                    Some("scene") => Part::Scene,
                    Some("all") => Part::All,
                    _ => return Err(USAGE.into()),
                };
            }
            "--tolerances" => {
                let r = read_rules(args.next())?;
                if let Some(bad) = r.iter().find(|r| r.ulp.is_none()) {
                    return Err(format!(
                        "{}: a tolerance file holds only `ulp` rules",
                        bad.at
                    ));
                }
                opts.tolerances.extend(r);
            }
            "--allow" => {
                let r = read_rules(args.next())?;
                if let Some(bad) = r.iter().find(|r| r.ulp.is_some()) {
                    return Err(format!(
                        "{}: `ulp` rules go into the tolerance file",
                        bad.at
                    ));
                }
                opts.allow.extend(r);
            }
            "--limit" => {
                style.limit = args.next().and_then(|n| n.parse().ok()).ok_or(USAGE)?;
            }
            "--show-allowed" => style.show_allowed = true,
            "--suggest-allow" => suggest = true,
            "-h" | "--help" => return Err(USAGE.into()),
            _ if a.starts_with("--") => return Err(USAGE.into()),
            _ => paths.push(PathBuf::from(a)),
        }
    }
    let [pa, pb] = paths.as_slice() else {
        return Err(USAGE.into());
    };
    let t = std::time::Instant::now();
    let a = Capture::read(pa).map_err(|e| e.to_string())?;
    let b = Capture::read(pb).map_err(|e| e.to_string())?;
    let read = t.elapsed();
    let cmp = compare(&a, &b, &opts);
    print!(
        "{}",
        render(
            (&pa.display().to_string(), &pb.display().to_string()),
            &a,
            &b,
            &cmp,
            &opts,
            style
        )
    );
    eprintln!(
        "drawdiff: read in {:.2} s, compared in {:.2} s",
        read.as_secs_f64(),
        t.elapsed().saturating_sub(read).as_secs_f64()
    );
    if suggest {
        print!(
            "\n# suggested allow-list entries (write each cause):\n{}",
            suggest_allow(&cmp)
        );
    }
    Ok(cmp.unallowed() == 0)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("drawdiff: {e}");
            ExitCode::from(2)
        }
    }
}
