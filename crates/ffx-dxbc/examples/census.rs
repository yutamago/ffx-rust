//! `cargo run --release -p ffx-dxbc --example census -- <game dir>`: what
//! the game's shaders use, for the choice of a native graphics back end
//! (docs/m6.md): the shader files in `data/FFX_Data.vbf`
//! (`shaders/d3d11/*.phyre`) and the DXBC containers inside `FFX.exe` and
//! the DLLs beside it, with their stages and shader models, the
//! instructions, resource dimensions, samplers and system values they use.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ffx_dxbc::sm4::{Decl, Dim};
use ffx_dxbc::{Container, find_all, hash};

#[derive(Default)]
struct Census {
    unique: BTreeSet<u64>,
    models: BTreeMap<String, usize>,
    ops: BTreeMap<&'static str, usize>,
    dims: BTreeMap<String, usize>,
    comparison_samplers: usize,
    system_values: BTreeMap<u32, usize>,
    special_outputs: BTreeMap<String, usize>,
    indexable_temps: usize,
    failed: usize,
    sizes: Vec<usize>,
}

impl Census {
    fn add(&mut self, blob: &[u8]) {
        if !self.unique.insert(hash(blob)) {
            return;
        }
        self.sizes.push(blob.len());
        let Ok(c) = Container::parse(blob) else {
            self.failed += 1;
            return;
        };
        let (kind, major, minor) = c.program().unwrap_or(("?", 0, 0));
        *self
            .models
            .entry(format!("{kind}_{major}_{minor}"))
            .or_default() += 1;
        let Ok(p) = c.program_code() else {
            self.failed += 1;
            return;
        };
        let mut ops = BTreeSet::new();
        for i in &p.code {
            ops.insert(i.op.name());
        }
        for o in ops {
            *self.ops.entry(o).or_default() += 1;
        }
        for d in &p.decls {
            match d {
                Decl::Resource { dim, .. } => {
                    let name = match dim {
                        Dim::Buffer => "Buffer".to_owned(),
                        Dim::Texture1D => "Texture1D".to_owned(),
                        Dim::Texture2D => "Texture2D".to_owned(),
                        Dim::Texture2DMs(n) => format!("Texture2DMS<{n}>"),
                        Dim::Texture3D => "Texture3D".to_owned(),
                        Dim::TextureCube => "TextureCube".to_owned(),
                        Dim::Other(n) => format!("other {n}"),
                    };
                    *self.dims.entry(name).or_default() += 1;
                }
                Decl::Sampler {
                    comparison: true, ..
                } => self.comparison_samplers += 1,
                Decl::Input {
                    system: Some(s), ..
                }
                | Decl::Output {
                    system: Some(s), ..
                } => *self.system_values.entry(*s).or_default() += 1,
                Decl::OutputSpecial(r) => {
                    *self.special_outputs.entry(format!("{r:?}")).or_default() += 1;
                }
                Decl::IndexableTemp { .. } => self.indexable_temps += 1,
                _ => {}
            }
        }
    }

    fn print(&self, title: &str) {
        println!("== {title}: {} distinct containers", self.unique.len());
        if self.unique.is_empty() {
            return;
        }
        let total: usize = self.sizes.iter().sum();
        println!(
            "  bytes {total}, largest {}",
            self.sizes.iter().max().unwrap_or(&0)
        );
        println!("  models {:?}", self.models);
        println!("  failed to decode {}", self.failed);
        println!("  resource dims (declarations) {:?}", self.dims);
        println!("  comparison samplers {}", self.comparison_samplers);
        println!("  system values (D3D_NAME, declarations) {:?}", self.system_values);
        println!("  special outputs {:?}", self.special_outputs);
        println!("  shaders with indexable temps {}", self.indexable_temps);
        let ops: Vec<String> = self.ops.iter().map(|(o, n)| format!("{o} {n}")).collect();
        println!("  instructions (shaders using each): {}", ops.join(", "));
    }
}

fn main() {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .or_else(|| std::env::var("FFX_GAME_DIR").ok())
            .expect("usage: census <game dir>"),
    );
    let archive = ffx_vbf::Archive::open(dir.join("data/FFX_Data.vbf")).expect("FFX_Data.vbf");
    let mut all = Census::default();
    let mut files: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for e in archive.entries() {
        if !e.path.contains("/shaders/d3d11/") || !e.path.ends_with(".phyre") {
            continue;
        }
        let bytes = archive.read(e).expect("read");
        let blobs = find_all(&bytes);
        let name = e.path.rsplit('/').next().unwrap_or(&e.path).to_owned();
        let entry = files.entry(name).or_default();
        entry.0 += 1;
        entry.1 += blobs.len();
        for (_, blob) in blobs {
            all.add(blob);
        }
    }
    println!(
        "shader files: {} names, {} copies (languages, folders)",
        files.len(),
        files.values().map(|v| v.0).sum::<usize>()
    );
    for (name, (copies, blobs)) in &files {
        println!("  {name}: {copies} copies, {blobs} containers");
    }
    all.print("shaders/d3d11/*.phyre");
    for exe in ["FFX.exe", "iggy_w32.dll", "fmodex.dll", "fmod_event.dll"] {
        let Ok(bytes) = std::fs::read(dir.join(exe)) else {
            continue;
        };
        let mut c = Census::default();
        for (_, blob) in find_all(&bytes) {
            c.add(blob);
        }
        c.print(exe);
    }
}
