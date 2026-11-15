//! The bytes a draw reads from its dynamic vertex buffers, as drawdiff
//! keys them: `ring <capture> <frame> <item> [slot]` prints, per ring slot
//! of item `item` (drawdiff's `A#n`/`B#n`), the window's range in the
//! buffer and its records as hex words (`stride` bytes a line).
use ffx_drawdiff::capture::{Objects, ring_window};
use ffx_gfx::{Cmd, Msg};
use std::collections::HashMap;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: ring <capture> <frame> <item> [slot]");
    let frame_no: usize = args.next().and_then(|n| n.parse().ok()).expect("frame");
    let item_no: usize = args.next().and_then(|n| n.parse().ok()).expect("item");
    let only: Option<u8> = args.next().and_then(|n| n.parse().ok());
    let path = std::path::Path::new(&path);
    let path = if path.is_dir() {
        path.join("capture.ffxgfx")
    } else {
        path.to_owned()
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let body = bytes
        .strip_prefix(ffx_gfx::CAPTURE_MAGIC)
        .expect("not a capture");
    let msgs = ffx_gfx::read_all(body).unwrap_or_else(|e| panic!("{e}"));
    let mut objects = Objects::default();
    let mut frames = 0;
    for msg in &msgs {
        let Msg::Frame(frame) = msg else {
            objects.apply(msg);
            continue;
        };
        if frames != frame_no {
            frames += 1;
            continue;
        }
        let mut contents: HashMap<ffx_gfx::Id, Vec<u8>> = HashMap::new();
        let mut item = 0;
        for c in &frame.cmds {
            match c {
                Cmd::UpdateBuffer { id, offset, data } => {
                    let buf = contents.entry(*id).or_insert_with(|| {
                        objects
                            .buffers
                            .get(id)
                            .map(|x| x.data.clone())
                            .unwrap_or_default()
                    });
                    let at = *offset as usize;
                    if buf.len() < at + data.len() {
                        buf.resize(at + data.len(), 0);
                    }
                    buf[at..at + data.len()].copy_from_slice(data);
                }
                Cmd::Draw(draw) => {
                    if item == item_no {
                        let bytes = |id: ffx_gfx::Id| {
                            contents
                                .get(&id)
                                .map(Vec::as_slice)
                                .or_else(|| objects.buffers.get(&id).map(|b| b.data.as_slice()))
                        };
                        for (slot, id, stride, from, to) in ring_window(&objects, draw, &bytes) {
                            if only.is_some_and(|s| s != slot) {
                                continue;
                            }
                            println!("vb{slot}: buffer {id:#x} stride {stride} bytes {from}..{to}");
                            let Some(window) = bytes(id).and_then(|b| b.get(from..to)) else {
                                println!("  out of bounds");
                                continue;
                            };
                            let line = (stride as usize).max(4);
                            for (r, rec) in window.chunks(line).enumerate() {
                                let words: Vec<String> = rec
                                    .chunks(4)
                                    .map(|w| {
                                        let mut x = [0u8; 4];
                                        x[..w.len()].copy_from_slice(w);
                                        format!("{:08x}", u32::from_le_bytes(x))
                                    })
                                    .collect();
                                println!("  {r:4} {}", words.join(" "));
                            }
                        }
                        return;
                    }
                    item += 1;
                }
                Cmd::ClearTarget { .. } | Cmd::ClearDepth { .. } | Cmd::CopyTexture { .. } => {
                    if item == item_no {
                        println!("item {item_no} is not a draw");
                        return;
                    }
                    item += 1;
                }
            }
        }
        println!("frame {frame_no} has {item} items");
        return;
    }
    println!("{frames} frames");
}
