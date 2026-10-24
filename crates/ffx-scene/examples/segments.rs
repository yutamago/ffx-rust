//! Scratch: a model cluster's segments (vertices, indices, blocks, material textures).
fn main() {
    let path = std::env::args().nth(1).expect("model file");
    let bytes = std::fs::read(path).expect("read");
    let m = ffx_phyre::Model::parse(&bytes).expect("model");
    for (i, (s, b)) in m.segments.iter().zip(m.buffers()).enumerate() {
        let strides: Vec<String> = b
            .blocks
            .iter()
            .map(|x| format!("{}@{:#x}{:?}", x.stride, x.bytes.start, x.streams))
            .collect();
        let tex = s
            .material
            .and_then(|k| m.materials.get(k))
            .and_then(|mat| mat.base_color())
            .map_or(String::new(), |t| t.path.clone());
        println!(
            "seg {i}: v {} i {} bones {} ib@{:#x} {} {tex}",
            s.positions.len(),
            s.indices.len(),
            s.bones.len(),
            b.indices.start,
            strides.join(" ")
        );
    }
    for inst in &m.instances {
        println!(
            "instance {} node {:?} skel {:?} segs {:?}",
            inst.name, inst.node, inst.skeleton, inst.segments
        );
    }
}
