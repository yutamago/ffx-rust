//! Adds a copy of a model's largest segment as a new segment, moved by an
//! offset: `add_segment <in.phyre> <out.phyre> [dx dy dz [segment]]`.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).expect("read");
    let d: Vec<f32> = args[3..args.len().min(6)]
        .iter()
        .map(|a| a.parse().expect("number"))
        .collect();
    let d = if d.len() == 3 {
        [d[0], d[1], d[2]]
    } else {
        [80.0, 0.0, 0.0]
    };
    let model = ffx_phyre::Model::parse(&bytes).expect("model");
    let template = args.get(6).map_or_else(
        || {
            model
                .segments
                .iter()
                .enumerate()
                .max_by_key(|(_, s)| s.positions.len())
                .expect("a segment")
                .0
        },
        |s| s.parse().expect("segment"),
    );
    let mut edit = ffx_phyre::edit::ClusterEdit::parse(&bytes).expect("cluster");
    let new = edit
        .add_segment(u32::try_from(template).unwrap())
        .expect("add");
    let grown = edit.write().expect("write");
    let mut model = ffx_phyre::Model::parse(&grown).expect("model again");
    eprintln!(
        "segment {template} copied to {new}: {} segments now",
        model.segments.len()
    );
    let s = &mut model.segments[new as usize];
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &s.positions {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    eprintln!("bounds {lo:?} .. {hi:?}");
    for p in &mut s.positions {
        for k in 0..3 {
            p[k] += d[k];
        }
    }
    let out = model.write(&grown).expect("geometry");
    let back = ffx_phyre::Model::parse(&out).expect("parse result");
    assert_eq!(
        back.segments[new as usize].positions,
        model.segments[new as usize].positions
    );
    std::fs::write(&args[2], out).expect("write file");
}
