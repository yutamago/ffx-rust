fn main() {
    let path = std::env::args().nth(1).expect("file");
    let bytes = std::fs::read(path).expect("read");
    let c = ffx_dxbc::Container::parse(&bytes).expect("dxbc");
    let p = c.program_code().expect("code");
    for d in &p.decls {
        println!("{d:?}");
    }
    for i in &p.code {
        println!("{i:?}");
    }
}
