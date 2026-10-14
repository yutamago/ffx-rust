//! Prints a cluster's classes and members: `classes <file.phyre> [class]`.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).unwrap();
    let c = ffx_phyre::cluster::Cluster::parse(&bytes).unwrap();
    for class in &c.classes {
        if args.get(2).is_some_and(|n| *n != class.name) {
            continue;
        }
        let base = class.base.map_or("", |b| c.classes[b].name.as_str());
        println!("{} : {} ({:#x} bytes)", class.name, base, class.size);
        for m in &class.members {
            println!("  +{:#05x} {:#05x} {}", m.offset, m.size, m.name);
        }
    }
}
