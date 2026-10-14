//! Adds a copy of segment `s`, moved by (dx, 0, 0) and bound only to
//! hierarchy joint `h` through a new skeleton matrix (the inverse of its
//! world matrix): `bind_probe <in> <out> <s> <h> <dx>`.
fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut m = [0f32; 16];
    for c in 0..4 {
        for r in 0..4 {
            m[c * 4 + r] = (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum();
        }
    }
    m
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).expect("read");
    let (segment, joint, dx): (u32, u16, f32) = (
        args[3].parse().unwrap(),
        args[4].parse().unwrap(),
        args[5].parse().unwrap(),
    );
    let model = ffx_phyre::Model::parse(&bytes).expect("model");
    let name = &model.skeletons[0].names[usize::from(joint)];
    let mut i = model
        .nodes
        .iter()
        .position(|n| &n.name == name)
        .expect("node");
    let mut w = model.nodes[i].local;
    while let Some(p) = model.nodes[i].parent {
        w = mul(&model.nodes[p].local, &w);
        i = p;
    }
    // Rigid inverse: transpose the rotation, rotate the negated translation.
    let mut inv = [0f32; 16];
    for c in 0..3 {
        for r in 0..3 {
            inv[c * 4 + r] = w[r * 4 + c];
        }
    }
    for r in 0..3 {
        inv[12 + r] = -(0..3).map(|k| inv[k * 4 + r] * w[12 + k]).sum::<f32>();
    }
    inv[15] = 1.0;
    // FFX_PROBE_COPY=k: the new matrix is a copy of skeleton matrix k.
    if let Ok(k) = std::env::var("FFX_PROBE_COPY") {
        inv = model.skeletons[0].inverse_bind[k.parse::<usize>().expect("index")];
    }
    eprintln!("world {w:?}\ninverse {inv:?}");
    let mut edit = ffx_phyre::edit::ClusterEdit::parse(&bytes).expect("cluster");
    // FFX_PROBE_EXISTING=k: use existing skeleton matrix k instead of a new one.
    let k = match std::env::var("FFX_PROBE_EXISTING") {
        Ok(k) => k.parse().expect("skeleton index"),
        Err(_) => edit
            .add_skeleton_matrix(segment, joint, &inv)
            .expect("matrix"),
    };
    let new = edit.add_segment(segment).expect("segment");
    edit.set_palette(new, &[(joint, k)]).expect("palette");
    let grown = edit.write().expect("write");
    let mut model = ffx_phyre::Model::parse(&grown).expect("again");
    let seg = &mut model.segments[new as usize];
    for p in &mut seg.positions {
        p[0] += dx;
    }
    seg.joints.fill([0; 4]);
    seg.weights.fill([1.0, 0.0, 0.0, 0.0]);
    std::fs::write(&args[2], model.write(&grown).expect("geometry")).expect("write");
    eprintln!("segment {new} on {name} (skeleton matrix {k})");
}
