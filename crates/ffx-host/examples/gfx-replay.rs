//! Plays a back-end trace (`FFX_HOST_GFX_TRACE`, `ffx_host::gfx::trace`)
//! into a back end and saves the back buffer at chosen `Present`s, for
//! pixel parity between back ends on one GPU (docs/host.md, "The wgpu back
//! end"; `tools/m6/host-parity.py`).
//!
//! ```text
//! gfx-replay <trace> <d3d11|wgpu|null> <out dir> [--every N] [--at N,N,...] [--stop N]
//! ```
//!
//! `wgpu` takes `FFX_HOST_WGPU_BACKEND` (vulkan, dx12, metal, gl) and
//! `FFX_HOST_WGPU_ADAPTER` (a part of the adapter's name). No window is
//! made: both back ends draw offscreen. The back end's problems (wgpu) go
//! to `<out dir>/problems.txt`.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("usage: gfx-replay <trace> <d3d11|wgpu|null> <out dir> [--every N] [--at N,N,...] [--stop N]");
        return ExitCode::FAILURE;
    }
    let (trace, name, out) = (PathBuf::from(&args[0]), args[1].as_str(), PathBuf::from(&args[2]));
    let mut every = 0u32;
    let mut at: Vec<u32> = Vec::new();
    let mut stop = None;
    let mut i = 3;
    while i < args.len() {
        let v = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--every" => every = v.parse().unwrap_or(0),
            "--at" => at = v.split(',').filter_map(|x| x.parse().ok()).collect(),
            "--stop" => stop = v.parse().ok(),
            other => {
                eprintln!("gfx-replay: unknown option {other}");
                return ExitCode::FAILURE;
            }
        }
        i += 2;
    }
    let mut backend: Box<dyn ffx_host::gfx::Backend> = match name {
        "null" => Box::new(ffx_host::gfx::Null::default()),
        #[cfg(windows)]
        "d3d11" => Box::new(ffx_host::d3d11::D3d11::default()),
        "wgpu" => ffx_host::gfx::wgpu_backend(None),
        other => {
            eprintln!("gfx-replay: no back end {other} here");
            return ExitCode::FAILURE;
        }
    };
    let bytes = match ffx_host::gfx::trace::read_file(&trace) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}: {e}", trace.display());
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("{}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    let shot = |n: u32| (every > 0 && n.is_multiple_of(every)) || at.contains(&n);
    let t = std::time::Instant::now();
    let r = ffx_host::gfx::trace::replay(&bytes, backend.as_mut(), &out, &shot, stop);
    let notes = ffx_host::log::notes().join("\n");
    let _ = std::fs::write(out.join("problems.txt"), format!("{notes}\n"));
    match r {
        Ok(r) => {
            println!(
                "{}: {} calls, {} presents, {} shots in {:.1} s",
                backend.name(),
                r.calls,
                r.presents,
                r.shots.len(),
                t.elapsed().as_secs_f64()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gfx-replay: {e}");
            ExitCode::FAILURE
        }
    }
}
