//! Rebuilds a motion group file: `rebuild <in.mgrp> <out.mgrp>`.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).unwrap();
    let group = ffx_motion::MotionGroup::parse(&bytes).unwrap();
    std::fs::write(&args[2], group.write().unwrap()).unwrap();
}
