//! `capture_gaps <capture.ffxgfx>`: the draws of a full-mode capture whose
//! vertex or index data the capture does not hold, one line each:
//! `<draw number, from 1> <buffer id, hex>`.
//!
//! A dynamic buffer (`D3D11_USAGE_DYNAMIC`) holds what the game last wrote
//! with `Map`; the capture sends a write as an `UpdateBuffer` only once it
//! is armed. A draw that reads a dynamic buffer with no `UpdateBuffer`
//! before it in the stream draws from whatever the buffer's `Buffer`
//! message carried (nothing, for the post-processing quads: a replay draws
//! zeros). `tools/m6/wgpu-parity.py` leaves those draws, and what is made
//! from them, out of the comparison (docs/m6.md).

use std::collections::{BTreeSet, HashMap};

const USAGE_DYNAMIC: u32 = 2;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: capture_gaps <capture.ffxgfx>");
    let bytes = std::fs::read(&path).expect("read the capture");
    let body = bytes
        .strip_prefix(&ffx_gfx::CAPTURE_MAGIC[..])
        .expect("not a capture file");
    let msgs = ffx_gfx::read_all(body).expect("capture stream");
    // Dynamic buffers, and whether the stream has written them yet.
    let mut dynamic: HashMap<u32, bool> = HashMap::new();
    let mut draw = 0usize;
    for m in &msgs {
        match m {
            ffx_gfx::Msg::Buffer { id, desc, .. } if desc[1] == USAGE_DYNAMIC => {
                dynamic.insert(*id, false);
            }
            ffx_gfx::Msg::UpdateBuffer { id, .. } => {
                dynamic.entry(*id).and_modify(|w| *w = true);
            }
            ffx_gfx::Msg::Frame(f) => {
                for c in &f.cmds {
                    match c {
                        ffx_gfx::Cmd::UpdateBuffer { id, .. } => {
                            dynamic.entry(*id).and_modify(|w| *w = true);
                        }
                        ffx_gfx::Cmd::Draw(d) => {
                            draw += 1;
                            let indexed = matches!(
                                d.kind,
                                ffx_gfx::DrawKind::Indexed | ffx_gfx::DrawKind::IndexedInstanced
                            );
                            let mut read: BTreeSet<u32> = d.vertex.iter().map(|v| v.1).collect();
                            if indexed {
                                read.insert(d.index.0);
                            }
                            for id in read {
                                if dynamic.get(&id) == Some(&false) {
                                    println!("{draw} {id:x}");
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
