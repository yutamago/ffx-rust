//! `translate <vs> <ps>`: prints the linked WGSL of a shader pair.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let vb = std::fs::read(&a[0]).expect("vs");
    let pb = std::fs::read(&a[1]).expect("ps");
    let vc = ffx_dxbc::Container::parse(&vb).expect("vs dxbc");
    let pc = ffx_dxbc::Container::parse(&pb).expect("ps dxbc");
    let v = ffx_dxbc::wgsl::link(&vc, &pc).expect("link");
    println!("// varyings: {v:?}");
    println!(
        "{}",
        ffx_dxbc::wgsl::translate(&vc, &v, 0).expect("vs").wgsl
    );
    println!(
        "{}",
        ffx_dxbc::wgsl::translate(&pc, &v, 1).expect("ps").wgsl
    );
}
