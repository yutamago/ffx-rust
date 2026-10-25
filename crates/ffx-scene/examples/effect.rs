//! Dumps an effect variant cluster's render passes, shaders, pass states
//! and parameter tables: `effect <path filter>`, with `FFX_GAME_DIR` set.
#![allow(clippy::many_single_char_names)] // a dump tool's short names
use ffx_scene::phyre::{Graph, Obj};

fn hex(b: &[u8]) -> String {
    b.chunks(4)
        .map(|w| {
            let mut x = [0u8; 4];
            x[..w.len()].copy_from_slice(w);
            format!("{:08x}", u32::from_le_bytes(x))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let filter = std::env::args().nth(1).expect("usage: effect <filter>");
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    for e in archive.entries() {
        if !e.path.contains(filter.as_str()) || !e.path.ends_with(".phyre") {
            continue;
        }
        let g = Graph::parse(archive.read(e).expect("read")).expect("graph");
        println!("== {}", e.path);
        for (l, list) in g.c.lists.iter().enumerate() {
            println!("list {l} {} x{}", g.class_of(l), list.count);
        }
        for (k, f) in g.pointers.iter().enumerate() {
            for p in f {
                println!("ptr list {k} {:x?}", p.0);
            }
        }
        for (k, f) in g.arrays.iter().enumerate() {
            for p in f {
                println!("arr list {k} {:x?}", p.0);
            }
        }
        for v in g.objects("PEffectVariant") {
            dump_variant(&g, v);
        }
    }
}

fn dump_variant(g: &Graph, v: Obj) {
    let c = "PEffectVariant";
    println!(
        "variant {:?} largest {:?} tweak size {:?} untweak size {:?}",
        v,
        g.u16(v, c, "m_largestShaderPassCount"),
        g.u16(v, c, "m_tweakableParameterBufferSize"),
        g.u16(v, c, "m_untweakableParameterBufferSize")
    );
    for sw in g.targets(v, c, "m_switches", 4) {
        println!(
            "  switch {:?} = {:?}",
            g.string(sw, "PMaterialSwitch", "m_name"),
            g.string(sw, "PMaterialSwitch", "m_value")
        );
    }
    for (name, member) in [
        ("tweakable", "m_tweakableShaderParameterDefinitions"),
        ("untweakable", "m_untweakableShaderParameterDefinitions"),
    ] {
        for d in g.targets(v, c, member, 4) {
            println!("  {name} {}", def(g, d));
        }
    }
    for rp in g.targets(v, c, "m_sceneRenderPasses", 4) {
        let r = "PSceneRenderPass";
        println!(
            "  pass {:?} raw {}",
            g.user_text(rp, r, "m_passType"),
            hex(g.field(rp, r, "m_passType", 40).unwrap_or(&[]))
        );
        for f in g.targets(rp, r, "m_variantsFoldingTable", 4) {
            println!("    folding {}", hex(&g.bytes[f.at..f.at + 20]));
        }
        for sh in g.targets(rp, r, "m_shaders", 4) {
            let s = "PShader";
            println!(
                "    shader {:?} freq {:?} size {:?}",
                sh,
                g.u32(sh, s, "m_parameterBufferFrequenciesRequired"),
                g.u32(sh, s, "m_parameterBufferSize")
            );
            for d in g.targets(sh, s, "m_parameterDefinitionsForPasses", 4) {
                println!("      def {}", def(g, d));
            }
            for d in g.targets(sh, s, "m_streamDefinitionsForPasses", 4) {
                let t = "PShaderStreamDefinition";
                println!(
                    "      stream {:?} render {:?} loc {:?} type {:?} index {:?}",
                    g.string(d, t, "m_name"),
                    g.user_text(d, t, "m_renderType"),
                    g.u32(d, t, "m_bufferLoc").map(|x| format!("{x:#x}")),
                    g.u8(d, t, "m_dataType"),
                    g.u8(d, t, "m_index")
                );
            }
            for p in g.targets(sh, s, "m_passes", 4) {
                dump_pass(g, p);
            }
        }
    }
}

fn def(g: &Graph, d: Obj) -> String {
    let t = "PShaderParameterDefinition";
    format!(
        "{:?} ptype {:#x} dtype {:#x} count {:?} loc {:#x} cbloc {:#x}",
        g.string(d, t, "m_name"),
        g.u8(d, t, "m_parameterType").unwrap_or(0),
        g.u8(d, t, "m_dataType").unwrap_or(0),
        g.u16(d, t, "m_arrayElementCount"),
        g.u32(d, t, "m_bufferLoc").unwrap_or(0),
        g.u32(d, t, "m_constantBufferLocation").unwrap_or(0)
    )
}

fn dump_pass(g: &Graph, p: Obj) {
    let c = "PShaderPassD3D11";
    let st = g.offset(c, "m_state").unwrap_or(0) as usize;
    println!(
        "      pass {:?} state {}",
        p,
        hex(&g.bytes[p.at + st..p.at + st + 380])
    );
    for (name, member) in [
        ("vp", "m_vertexProgram"),
        ("fp", "m_fragmentProgram"),
        ("gp", "m_geometryProgram"),
    ] {
        if let Some(prog) = g.target_obj(p, "PShaderPassBase", member) {
            let pc = "PShaderProgramD3D11";
            let code = g
                .ptr(prog, pc, "m_compiledCode", 4)
                .map(|t| format!("{t:?}"));
            let arr = g.array_bytes(prog, pc, "m_compiledCode", 4).map(|b| {
                let n = g.u32(prog, pc, "m_compiledCode").unwrap_or(0) as usize;
                let b = &b[..n.min(b.len())];
                format!("{} bytes key {:016x}", b.len(), ffx_dxbc::hash(b))
            });
            println!(
                "        {name} {:?} cbsize {:?} global {:?} code ptr {code:?} arr {arr:?}",
                prog,
                g.u32(prog, pc, "m_constantBufferSize"),
                g.u32(prog, pc, "m_globalConstantBufferIndex")
            );
            if name == "vp" {
                for d in g.targets(prog, "PStreamInputLayoutD3D11", "m_inputLayout", 4) {
                    let t = "PStreamInputDescD3D11";
                    println!(
                        "          input {:?} {:?} idx {:?} fmt {:?} slot {:?}",
                        g.string(d, t, "m_semantic"),
                        g.user_text(d, t, "m_renderType"),
                        g.u32(d, t, "m_semanticIndex"),
                        g.u32(d, t, "m_d3dFormat"),
                        g.u32(d, t, "m_inputSlot")
                    );
                }
                let il = g
                    .offset("PShaderVertexProgramD3D11", "m_inputLayout")
                    .unwrap_or(0) as usize;
                println!(
                    "          layout raw {}",
                    hex(&g.bytes[prog.at + il..prog.at + il + 12])
                );
            }
        }
    }
    for (name, member) in [
        ("vs", "m_vertexParameterLocation"),
        ("ps", "m_fragmentParameterLocation"),
        ("vstex", "m_vertexTexParameterLocation"),
        ("pstex", "m_fragmentTexParameterLocation"),
    ] {
        let off = g.offset(c, member).unwrap_or(0) as usize;
        println!(
            "        {name} raw {}",
            hex(&g.bytes[p.at + off..p.at + off + 24])
        );
        let t = u32::try_from(off + 0x14).ok().and_then(|o| g.ptr_at(p, o));
        if let Some(t) = t {
            for i in t.first..t.first + t.count {
                let o = g.obj(t.list, i);
                println!(
                    "          loc {} ({})",
                    hex(&g.bytes[o.at..o.at + g.c.lists[t.list].object_size()]),
                    g.class_of(t.list)
                );
            }
        }
    }
}
