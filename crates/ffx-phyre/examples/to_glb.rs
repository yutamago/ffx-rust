//! Converts a model cluster file to glTF without textures:
//! `to_glb <file.dae.phyre> <out.glb>`.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).unwrap();
    let model = ffx_phyre::Model::parse(&bytes).unwrap();
    let n: usize = model.segments.iter().map(|s| s.positions.len()).sum();
    eprintln!("{} segments, {n} vertices", model.segments.len());
    std::fs::write(&args[2], model.to_glb_with(true, |_| None, &[])).unwrap();
}
