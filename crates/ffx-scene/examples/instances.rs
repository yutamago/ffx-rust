//! Lists a map model's instances as the builder reads them: `instances
//! <model path filter> [name filter]`, with `FFX_GAME_DIR` set.
use ffx_scene::map::MapModel;

fn main() {
    let mut args = std::env::args().skip(1);
    let filter = args.next().expect("usage: instances <model filter> [name]");
    let name = args.next().unwrap_or_default();
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let e = archive
        .entries()
        .iter()
        .find(|e| e.path.contains(filter.as_str()) && e.path.ends_with(".dae.phyre"))
        .expect("model");
    let m = MapModel::parse(archive.read(e).expect("read")).expect("model");
    for (i, inst) in m.instances.iter().enumerate() {
        if !inst.name.contains(name.as_str()) {
            continue;
        }
        println!(
            "{i} {} kind {} flag {:#x} group {} bounds {:?}",
            inst.name, inst.dobj_kind, inst.dobj_flag, inst.group_id, inst.bounds
        );
        println!("  materials {:?}", inst.materials);
        for &s in &inst.segments {
            let seg = &m.segments[s];
            println!(
                "  segment {s}: material index {} -> {:?}, bucket {}, {} indices, {} blocks",
                seg.material_index,
                m.material(i, s),
                seg.render_order,
                seg.index_count,
                seg.blocks.len()
            );
        }
    }
}
