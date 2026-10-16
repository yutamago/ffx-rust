//! Compares two replay traces: `replay_diff <a> <b>` (WSL paths). Exits 1
//! at the first frame where they differ.
fn main() {
    let mut args = std::env::args().skip(1);
    let a = std::fs::read_to_string(args.next().expect("usage: replay_diff <a> <b>")).expect("a");
    let b = std::fs::read_to_string(args.next().expect("b")).expect("b");
    let frames = a.lines().count().min(b.lines().count());
    match ffx_harness::replay::first_difference(&a, &b) {
        None => println!("same on all {frames} frames"),
        Some((frame, range)) => {
            println!("differ from frame {frame}, range {range}");
            std::process::exit(1);
        }
    }
}
