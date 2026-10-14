//! Writes probe edits of a motion group: `edit_probe <in> <out> keys|zero`.
use ffx_motion::{Channel, MotionGroup};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut group = MotionGroup::parse(&std::fs::read(&args[1]).unwrap()).unwrap();
    let mut changed = 0;
    for clip in group.sequences.iter_mut().flat_map(|s| &mut s.clips) {
        let ry = &mut clip.clip.channels[1];
        match (args[3].as_str(), &mut *ry) {
            ("keys", Channel::Keys(k)) => {
                k.iter_mut().for_each(|v| *v += 1024);
                changed += 1;
            }
            ("zero", Channel::Zero) => {
                *ry = Channel::Constant(0);
                changed += 1;
            }
            _ => {}
        }
    }
    let out = group.write().unwrap();
    eprintln!("{changed} clips changed, {} bytes", out.len());
    std::fs::write(&args[2], out).unwrap();
}
