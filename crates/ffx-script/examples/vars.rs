//! Lists a script's variables: `vars <file.ebp>`.
fn main() {
    let path = std::env::args().nth(1).expect("file");
    let container = ffx_script::Container::parse(std::fs::read(path).unwrap()).unwrap();
    let script = &container.script;
    for w in 0..script.workers().len() {
        let mut i = 0;
        while let Some((loc, len)) = script.variable(w, i) {
            println!(
                "w{w} v{i}: type {} storage {} offset {:#x} len {len}",
                loc >> 28,
                (loc >> 25) & 7,
                loc & 0xff_ffff
            );
            i += 1;
        }
    }
}
