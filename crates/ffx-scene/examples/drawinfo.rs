//! Prints what scene draws of a capture bind: `drawinfo <capture> [n...]`
//! (scene draw numbers; all when none). States as D3D11 words, samplers,
//! buffers, views and constant buffer sizes.
use ffx_gfx::StateKind;
use ffx_scene::capture::Capture;

fn words(b: &[u8]) -> String {
    b.chunks(4)
        .map(|w| {
            let mut x = [0u8; 4];
            x[..w.len()].copy_from_slice(w);
            format!("{:x}", u32::from_le_bytes(x))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: drawinfo <capture> [n...]");
    let want: Vec<usize> = args.filter_map(|a| a.parse().ok()).collect();
    let cap = Capture::read(&std::fs::read(path).expect("read")).expect("capture");
    let scene: Vec<_> = cap
        .draws()
        .filter(|(_, d)| Capture::scene_targets(d).is_some())
        .collect();
    for (n, (cmd, d)) in scene.iter().enumerate() {
        if !want.is_empty() && !want.contains(&n) {
            continue;
        }
        println!(
            "draw {n} cmd {cmd} {:?} count {} start {} base {} topo {}",
            d.kind, d.count, d.start, d.base, d.topology
        );
        let st = |id, k| cap.state(id, k).map(words).unwrap_or_default();
        println!(
            "  blend {} factor {:?} mask {:x}",
            st(d.blend, StateKind::Blend),
            d.blend_factor,
            d.sample_mask
        );
        println!(
            "  depth {} ref {}",
            st(d.depth, StateKind::Depth),
            d.stencil_ref
        );
        println!("  raster {}", st(d.raster, StateKind::Raster));
        println!("  viewport {:?} scissor {:?}", d.viewport, d.scissor);
        for (s, id) in &d.ps_samplers {
            println!("  ps sampler {s}: {}", st(*id, StateKind::Sampler));
        }
        for (s, id) in &d.vs_samplers {
            println!("  vs sampler {s}: {}", st(*id, StateKind::Sampler));
        }
        for (s, id) in &d.ps_srv {
            let v = cap.objects.get(id);
            let desc = match v {
                Some(ffx_gfx::Msg::View { desc, .. }) => format!("{desc:?}"),
                _ => "?".into(),
            };
            let t = cap.view_texture(*id).map(|t| format!("{:?}", t.desc));
            println!("  ps srv {s}: view {desc} tex {t:?}");
        }
        for (s, id) in &d.vs_srv {
            println!("  vs srv {s}: {id:#x}");
        }
        for (s, id, stride, off) in &d.vertex {
            let b = cap
                .buffer(*id)
                .map(|(desc, data)| (desc.to_vec(), data.len()));
            println!("  vb {s}: stride {stride} off {off} {b:?}");
        }
        let b = cap
            .buffer(d.index.0)
            .map(|(desc, data)| (desc.to_vec(), data.len()));
        println!("  ib fmt {} off {} {b:?}", d.index.1, d.index.2);
        if let Some(l) = cap.layout(d.layout) {
            for e in l {
                println!("  layout {e:?}");
            }
        }
        for (s, data) in &d.vs_cb {
            println!("  vs cb {s}: {} bytes", data.len());
        }
        for (s, data) in &d.ps_cb {
            println!("  ps cb {s}: {} bytes", data.len());
        }
        println!("  rtv {:x?} dsv {:#x}", d.rtv, d.dsv);
    }
}
