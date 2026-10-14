//! Adds a copy of segment `s` moved by (dx, 0, 0) with a new material (a
//! copy of the segment's own) whose base texture is `import`:
//! `material_probe <in> <out> <s> <dx> <import>`.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).expect("read");
    let (segment, dx): (u32, f32) = (args[3].parse().unwrap(), args[4].parse().unwrap());
    let mut edit = ffx_phyre::edit::ClusterEdit::parse(&bytes).expect("cluster");
    let segs = edit.list_of("PMeshSegment").expect("segments");
    let set_index = u32::from_le_bytes(edit.lists[segs].object(segment)[..4].try_into().unwrap());
    // The segment's material: element `set_index` of the mesh's set.
    let materials = edit.list_of("PMaterial").expect("materials");
    let meshes = edit.list_of("PMesh").expect("meshes");
    let material = edit.lists[meshes]
        .pointer_fixups
        .iter()
        .find(|f| f.0[0] == 0x8000_0034 && f.0[5] == set_index && f.0[3] as usize == materials)
        .expect("material set element")
        .0[2];
    // FFX_PROBE=notex: new material, same texture; FFX_PROBE=texonly: only
    // the texture of the segment's own material changes.
    let mode = std::env::var("FFX_PROBE").unwrap_or_default();
    if mode == "dupimport" {
        // A second import with the same id as an existing one.
        let existing = edit
            .import_index(&args[5])
            .expect("imports")
            .expect("an existing import");
        let dup = edit.add_import(existing, &args[5]).expect("import");
        edit.set_texture_import(material, "TextureSampler", dup)
            .expect("texture");
        std::fs::write(&args[2], edit.write().expect("write")).expect("write file");
        return;
    }
    if mode == "texonly" {
        edit.set_texture(material, "TextureSampler", &args[5])
            .expect("texture");
        std::fs::write(&args[2], edit.write().expect("write")).expect("write file");
        return;
    }
    let new = edit.add_segment(segment).expect("segment");
    let (m, index) = edit.add_material(material, new).expect("material");
    let segs = edit.list_of("PMeshSegment").expect("segments");
    edit.lists[segs].object_mut(new)[..4].copy_from_slice(&index.to_le_bytes());
    if mode != "notex" {
        edit.set_texture(m, "TextureSampler", &args[5])
            .expect("texture");
    }
    let grown = edit.write().expect("write");
    let mut model = ffx_phyre::Model::parse(&grown).expect("again");
    for p in &mut model.segments[new as usize].positions {
        p[0] += dx;
    }
    let out = model.write(&grown).expect("geometry");
    let check = ffx_phyre::Model::parse(&out).expect("result");
    let mat = check.segments[new as usize].material.expect("material");
    eprintln!(
        "segment {new}: material {mat} (set index {index}), textures {:?}",
        check.materials[mat]
            .textures
            .iter()
            .map(|t| (&t.sampler, &t.path))
            .collect::<Vec<_>>()
    );
    std::fs::write(&args[2], out).expect("write file");
}
