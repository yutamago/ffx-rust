//! Prints the pointer-array fixups and header word 11 of a cluster:
//! `ptrarrays <file.phyre>`.
use ffx_phyre::cluster::Cluster;
use ffx_phyre::fixup::unpack_array_fixups;

fn main() {
    let path = std::env::args().nth(1).expect("file");
    let bytes = std::fs::read(path).expect("read");
    let c = Cluster::parse(&bytes).expect("cluster");
    let w = |i: usize| u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
    println!("words 9 10 11: {} {} {}", w(9), w(10), w(11));
    let lists_at = 0x54 + c.header.namespace_size as usize;
    let lists: Vec<(u32, u32)> = (0..c.lists.len())
        .map(|i| {
            let at = lists_at + 0x24 * i;
            let n = u32::from_le_bytes(bytes[at + 0x20..at + 0x24].try_into().unwrap());
            (n, c.lists[i].count)
        })
        .collect();
    let packed = &bytes[c.fixups_offset..c.fixups_offset + w(9) as usize];
    let (recs, used) = unpack_array_fixups(packed, &lists).expect("unpack");
    println!("{} records, {used} bytes", recs.len());
    let mut k = 0;
    for (i, &(n, _)) in lists.iter().enumerate() {
        for r in &recs[k..k + n as usize] {
            println!("list {i} {}: {:x?}", c.classes[c.lists[i].class].name, r.0);
        }
        k += n as usize;
    }
    for (i, l) in c.lists.iter().enumerate() {
        let at = lists_at + 0x24 * i;
        let total = u32::from_le_bytes(bytes[at + 8..at + 12].try_into().unwrap());
        println!("list {i} {} count {} total {total} objs {} arrays {}", c.classes[l.class].name, l.count, l.objects_size, l.arrays_size);
    }
}
