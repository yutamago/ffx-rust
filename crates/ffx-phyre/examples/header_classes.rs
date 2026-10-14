//! Prints each instance list with the header class count it lines up with:
//! `header_classes <file.phyre>`.
fn main() {
    let path = std::env::args().nth(1).expect("file");
    let e =
        ffx_phyre::edit::ClusterEdit::parse(&std::fs::read(path).expect("read")).expect("parse");
    println!(
        "{} header classes: {:?}",
        e.header_class_counts.len(),
        e.header_class_counts
    );
    for (i, l) in e.lists.iter().enumerate() {
        let name = &e.cluster.classes[l.class].name;
        let word8 = l
            .objects
            .get(8..12)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        println!(
            "{i:3} {name:<32} count {:4} size {:5} word8 {word8:?}",
            l.count(),
            l.object_size
        );
    }
}
