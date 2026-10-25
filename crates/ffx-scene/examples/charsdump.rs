//! Dumps the character-world scene inputs of a capture:
//! `charsdump <capture.ffxgfx>` (records, instances, joints, cloth, the
//! dynamic geometry lists).
use ffx_scene::chars;
use ffx_scene::inputs;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: charsdump <capture.ffxgfx>");
    let bytes = std::fs::read(path).expect("read");
    let all = inputs::read(&bytes).expect("inputs");
    for s in &all {
        println!("{}", inputs::summary(s).lines().next().unwrap_or(""));
    }
    let frame = all.iter().map(|s| s.frame).find(|&f| f != u32::MAX);
    let Some(frame) = frame else { return };
    for s in chars::records(&all, frame) {
        println!("== world {:#x} pass {} ==", s.world, s.pass);
        for (k, c) in s.clusters.iter().enumerate() {
            println!(
                "  cluster {k}: index {} {:?} lists {:?}",
                c.index, c.name, c.lists
            );
        }
        for (k, b) in s.bounds.iter().enumerate() {
            println!(
                "  bounds {k} cluster {} slot {} list {} order {} instance {:?} world {:?}",
                b.cluster, b.slot, b.list, b.render_order, b.instance, b.world
            );
        }
        let cw = s.chars.as_ref().expect("chars");
        for (k, i) in s.instances.iter().enumerate() {
            let c = &cw.instances[k];
            println!(
                "  instance {k}: cluster {} slot {} segments {} materials {:?} flags {:#x} joints {} skeleton {} world {:?}",
                i.cluster,
                i.slot,
                i.segments.len(),
                i.materials,
                c.flags,
                c.joints.len(),
                c.skeleton.len(),
                i.world
            );
            for (n, g) in i.segments.iter().enumerate() {
                println!(
                    "    segment {n}: material {} matrix {} hidden {} id {} order {} bones {}",
                    g.material,
                    g.matrix.cast_signed(),
                    g.hidden,
                    g.id,
                    g.render_order,
                    c.skin_bones.get(n).map_or(0, Vec::len)
                );
            }
            println!("    joint parents {:?}", c.joint_parent);
        }
        for m in &s.materials {
            println!(
                "  material {:?}->{:?} {} bytes",
                m.remap_from,
                m.remap_to,
                m.parameters.len()
            );
        }
        for c in &cw.cloth {
            println!(
                "  cloth instance {} segment {} slot {} list {} count {} bytes {} matrix {:?}",
                c.instance,
                c.segment,
                c.slot,
                c.list_index,
                c.count,
                c.vertices.len(),
                c.matrix
            );
        }
        if let Some(d) = &cw.dyn_geo {
            println!(
                "  dyn geo id {} vertices {} indices {} arrays {:?} index bytes {:?}",
                d.id,
                d.vertices,
                d.indices,
                d.arrays.iter().map(Vec::len).collect::<Vec<_>>(),
                d.index_bytes
            );
            for a in &d.arrays {
                let f: Vec<f32> = a
                    .chunks(4)
                    .map(|w| f32::from_le_bytes([w[0], w[1], w[2], w[3]]))
                    .collect();
                println!("    {f:?}");
            }
        }
    }
}
