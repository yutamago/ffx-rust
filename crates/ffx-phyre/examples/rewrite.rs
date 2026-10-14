//! Rewrites a cluster through `ClusterEdit` (fixups re-encoded):
//! `rewrite <in.phyre> <out.phyre>`.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).expect("read");
    let edit = ffx_phyre::edit::ClusterEdit::parse(&bytes).expect("parse");
    let out = edit.write().expect("write");
    eprintln!("{} -> {} bytes", bytes.len(), out.len());
    std::fs::write(&args[2], out).expect("write");
}
