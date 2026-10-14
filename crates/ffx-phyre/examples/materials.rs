//! Prints a model's materials: `materials <file.dae.phyre>`.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).unwrap();
    let model = ffx_phyre::Model::parse(&bytes).unwrap();
    for (i, m) in model.materials.iter().enumerate() {
        println!("material {i}");
        for t in &m.textures {
            println!("  {} = {}", t.sampler, t.path);
        }
        for p in &m.parameters {
            if p.values.len() <= 4 {
                println!("  {} = {:?}", p.name, p.values);
            }
        }
    }
}
