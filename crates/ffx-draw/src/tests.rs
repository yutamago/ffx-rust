//! The replayer against a recording backend: synthetic captures, and a
//! real one when `FFX_DRAW_CAPTURE` names a `capture.ffxgfx`.

#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)] // test data

use std::collections::HashMap;

use ffx_gfx::{Cmd, Frame, Msg};

use crate::backend::{
    Backend, Blend, Buffer, DepthState, InitData, Layout, Raster, Readback, Result, Sampler,
    Shader, Texture, View,
};
use crate::desc::{
    BlendDesc, BufferDesc, DepthStencilDesc, InputElement, RasterDesc, SamplerDesc, Stage,
    TextureDesc, ViewDesc, ViewKind,
};
use crate::replay::{Capture, Replayer, compare_draws, select_scene};
use crate::state::{DrawCall, DrawState};

/// Records what it is asked to do.
#[derive(Default)]
struct Mock {
    next: u32,
    log: Vec<String>,
    /// Last bytes written per buffer.
    written: HashMap<Buffer, Vec<u8>>,
    /// The states of the draws, as the backend saw them.
    draws: Vec<(DrawState, DrawCall)>,
    view_textures: HashMap<View, Texture>,
}

impl Mock {
    fn id(&mut self) -> u32 {
        self.next += 1;
        self.next
    }
}

impl Backend for Mock {
    fn begin(&mut self) {
        self.log.push("begin".into());
    }
    fn end(&mut self) {
        self.log.push("end".into());
    }
    fn create_shader(&mut self, stage: Stage, dxbc: &[u8]) -> Result<Shader> {
        self.log.push(format!("shader {stage:?} {}", dxbc.len()));
        Ok(Shader(self.id()))
    }
    fn create_layout(&mut self, e: &[InputElement], _vs: &[u8]) -> Result<Layout> {
        self.log.push(format!("layout {}", e.len()));
        Ok(Layout(self.id()))
    }
    fn create_blend(&mut self, _: &BlendDesc) -> Result<Blend> {
        self.log.push("blend".into());
        Ok(Blend(self.id()))
    }
    fn create_depth_state(&mut self, _: &DepthStencilDesc) -> Result<DepthState> {
        self.log.push("depth".into());
        Ok(DepthState(self.id()))
    }
    fn create_raster(&mut self, _: &RasterDesc) -> Result<Raster> {
        self.log.push("raster".into());
        Ok(Raster(self.id()))
    }
    fn create_sampler(&mut self, _: &SamplerDesc) -> Result<Sampler> {
        self.log.push("sampler".into());
        Ok(Sampler(self.id()))
    }
    fn create_buffer(&mut self, d: &BufferDesc, data: Option<&[u8]>) -> Result<Buffer> {
        self.log
            .push(format!("buffer {} {}", d.size, data.is_some()));
        let b = Buffer(self.id());
        if let Some(data) = data {
            self.written.insert(b, data.to_vec());
        }
        Ok(b)
    }
    fn create_texture(&mut self, d: &TextureDesc, init: &[InitData]) -> Result<Texture> {
        self.log
            .push(format!("texture {}x{} {}", d.width, d.height, init.len()));
        Ok(Texture(self.id()))
    }
    fn create_view(&mut self, t: Texture, k: ViewKind, _: Option<&ViewDesc>) -> Result<View> {
        self.log.push(format!("view {k:?}"));
        let v = View(self.id());
        self.view_textures.insert(v, t);
        Ok(v)
    }
    fn write_buffer(&mut self, b: Buffer, offset: u32, data: &[u8]) {
        self.log
            .push(format!("write {} {offset} {}", b.0, data.len()));
        self.written.insert(b, data.to_vec());
    }
    fn clear_target(&mut self, v: View, c: [f32; 4]) {
        self.log.push(format!("clear {} {c:?}", v.0));
    }
    fn clear_depth(&mut self, v: View, flags: u32, depth: f32, stencil: u8) {
        self.log
            .push(format!("clear_depth {} {flags} {depth} {stencil}", v.0));
    }
    fn copy_texture(&mut self, d: Texture, s: Texture) {
        self.log.push(format!("copy {} {}", d.0, s.0));
    }
    fn draw(&mut self, state: &DrawState, call: DrawCall) {
        self.log.push(format!("draw {call:?}"));
        self.draws.push((*state, call));
    }
    fn view_texture(&self, v: View) -> Option<Texture> {
        self.view_textures.get(&v).copied()
    }
    fn read_texture(&mut self, _: Texture) -> Result<Readback> {
        Err(crate::Error("mock".into()))
    }
}

fn draw(rtv: Vec<(u8, u32)>, dsv: u32, count: u32) -> ffx_gfx::Draw {
    ffx_gfx::Draw {
        kind: ffx_gfx::DrawKind::Indexed,
        count,
        instances: 1,
        start: 3,
        base: -2,
        first_instance: 0,
        topology: 4,
        layout: 0x40,
        vs: 0x10,
        ps: 0x20,
        vertex: vec![(0, 0x50, 12, 0), (2, 0x51, 8, 4)],
        index: (0x52, 57, 0),
        vs_cb: vec![(0, vec![1; 64]), (3, vec![2; 20])],
        ps_cb: vec![(1, vec![3; 32])],
        vs_srv: vec![],
        ps_srv: vec![(0, 0x70), (2, 0x71)],
        vs_samplers: vec![],
        ps_samplers: vec![(0, 0x63), (2, 0x64)],
        blend: 0x60,
        blend_factor: [0.5, 0.0, -0.0, 1.0],
        sample_mask: u32::MAX,
        depth: 0x61,
        stencil_ref: 3,
        raster: 0x62,
        viewport: [0.0, 0.0, 1280.0, 720.0, 0.0, 1.0],
        scissor: [0, 0, 1280, 720],
        rtv,
        dsv,
    }
}

fn state(id: u32, kind: ffx_gfx::StateKind, size: usize, seed: u8) -> Msg {
    let mut desc = vec![0u8; size];
    desc[0] = 1;
    desc[4] = seed;
    Msg::State { id, kind, desc }
}

fn synthetic() -> Capture {
    use ffx_gfx::{StateKind as K, ViewKind as V};
    let tex = |id, w| Msg::Texture {
        id,
        desc: [w, 4, 1, 1, 28, 1, 0, 0, 0x28, 0, 0],
        subresources: vec![ffx_gfx::Subresource {
            row_pitch: w * 4,
            data: vec![7; (w * 16) as usize],
        }],
    };
    let view = |id, kind, resource| Msg::View {
        id,
        kind,
        resource,
        desc: None,
    };
    let objects = vec![
        Msg::Shader {
            id: 0x10,
            kind: b'v',
            bytecode: vec![1, 2, 3],
        },
        Msg::Shader {
            id: 0x20,
            kind: b'p',
            bytecode: vec![4, 5],
        },
        Msg::Layout {
            id: 0x40,
            elements: vec![ffx_gfx::Element {
                semantic: "POSITION".into(),
                index: 0,
                format: 6,
                slot: 0,
                offset: 0,
                per_instance: false,
                step: 0,
            }],
        },
        state(0x60, K::Blend, 264, 1),
        state(0x61, K::Depth, 52, 1),
        state(0x62, K::Raster, 40, 3),
        state(0x63, K::Sampler, 52, 21),
        // The same description as 0x63: one backend object for both.
        state(0x64, K::Sampler, 52, 21),
        Msg::Buffer {
            id: 0x50,
            desc: [48, 1, 1, 0, 0, 0],
            data: vec![9; 48],
        },
        Msg::Buffer {
            id: 0x51,
            desc: [32, 2, 1, 0x1_0000, 0, 0],
            data: vec![],
        },
        Msg::Buffer {
            id: 0x52,
            desc: [12, 1, 2, 0, 0, 0],
            data: vec![0; 12],
        },
        tex(0x80, 4),
        tex(0x81, 8),
        tex(0x90, 16),
        tex(0x91, 16),
        tex(0x92, 16),
        tex(0x93, 16),
        view(0x70, V::ShaderResource, 0x80),
        view(0x71, V::ShaderResource, 0x81),
        view(0xa0, V::RenderTarget, 0x90),
        view(0xa1, V::RenderTarget, 0x91),
        view(0xa2, V::DepthStencil, 0x92),
        view(0xa3, V::RenderTarget, 0x93),
    ];
    let scene = || vec![(0u8, 0xa0u32), (1, 0xa1)];
    let cmds = vec![
        Cmd::ClearTarget {
            view: 0xa3,
            color: [0.0; 4],
        },
        Cmd::ClearTarget {
            view: 0xa0,
            color: [0.25, 0.0, 0.0, 1.0],
        },
        Cmd::ClearDepth {
            view: 0xa2,
            flags: 3,
            depth: 1.0,
            stencil: 0,
        },
        Cmd::Draw(Box::new(draw(vec![(0, 0xa3)], 0, 6))),
        Cmd::Draw(Box::new(draw(scene(), 0xa2, 30))),
        Cmd::UpdateBuffer {
            id: 0x51,
            offset: 0,
            data: vec![5; 32],
        },
        Cmd::CopyTexture {
            dst: 0x81,
            src: 0x90,
        },
        Cmd::Draw(Box::new(draw(scene(), 0xa2, 60))),
        // After the scene: not replayed.
        Cmd::Draw(Box::new(draw(vec![(0, 0xa3)], 0, 6))),
        Cmd::Draw(Box::new(draw(scene(), 0xa2, 90))),
    ];
    let mut bytes = ffx_gfx::CAPTURE_MAGIC.to_vec();
    for m in &objects {
        m.write(&mut bytes);
    }
    Msg::Frame(Frame {
        number: 0,
        width: 1280,
        height: 720,
        cmds,
    })
    .write(&mut bytes);
    Capture::parse(&bytes).unwrap()
}

/// Runs the scene of `cap` on a mock with its targets brought in; returns
/// the mock and the draws seen, in the capture's terms, against the
/// originals.
fn run(cap: &Capture) -> (Mock, Vec<(ffx_gfx::Draw, ffx_gfx::Draw)>) {
    let sel = select_scene(&cap.frame).unwrap();
    let mut mock = Mock {
        next: 1000,
        ..Mock::default()
    };
    let mut r = Replayer::new();
    for (i, v) in [sel.targets.color, sel.targets.normal, sel.targets.depth]
        .into_iter()
        .enumerate()
    {
        let rid = cap.view_resource(v).unwrap();
        r.import(v, View(i as u32 + 1), Some((rid, Texture(i as u32 + 11))));
    }
    mock.begin();
    let mut seen = Vec::new();
    r.run(&mut mock, cap, &sel.cmds, |r, b, i, _| {
        if let Cmd::Draw(d) = &cap.frame.cmds[i] {
            let (s, call) = *b.draws.last().unwrap();
            let replayed = r.captured_draw(&s, call, |buf| {
                b.written.get(&buf).cloned().unwrap_or_default()
            });
            seen.push(((**d).clone(), replayed));
        }
    });
    mock.end();
    (mock, seen)
}

#[test]
fn scene_selection() {
    let cap = synthetic();
    let sel = select_scene(&cap.frame).unwrap();
    assert_eq!(
        (sel.targets.color, sel.targets.normal, sel.targets.depth),
        (0xa0, 0xa1, 0xa2)
    );
    // Clears of the scene's targets, the update, the copy and the two
    // scene draws of the first run (not the clear of 0xa3, not the draw
    // into it before the scene, not what follows the scene).
    assert_eq!(sel.cmds, vec![1, 2, 4, 5, 6, 7]);
    assert_eq!(sel.draws, 2);
    assert_eq!(sel.draw_numbers, vec![2, 3]);
    assert_eq!(sel.other_draws, 0);
}

#[test]
fn replay_issues_the_captured_draws() {
    let cap = synthetic();
    let (mock, seen) = run(&cap);
    assert_eq!(seen.len(), 2);
    for (orig, replayed) in &seen {
        assert_eq!(compare_draws(&cap, orig, replayed), Vec::<String>::new());
    }
    // Targets are the host's: no texture or view made for them. One
    // sampler for two captured ids with the same description.
    let made = |what: &str| mock.log.iter().filter(|l| l.starts_with(what)).count();
    assert_eq!(made("sampler"), 1);
    assert_eq!(made("view"), 2, "{:?}", mock.log);
    assert_eq!(made("texture"), 2);
    assert_eq!(made("shader"), 2);
    assert!(
        mock.log
            .contains(&"clear 1 [0.25, 0.0, 0.0, 1.0]".to_owned())
    );
    assert!(mock.log.contains(&"clear_depth 3 3 1 0".to_owned()));
    assert!(
        mock.log
            .iter()
            .any(|l| l.starts_with("copy ") && l.ends_with(" 11"))
    );
    // The window past 16 bytes is padded for the API (20 → 32).
    assert!(mock.log.iter().any(|l| l.ends_with(" 0 32")));
    assert_eq!(mock.log.first().map(String::as_str), Some("begin"));
    assert_eq!(mock.log.last().map(String::as_str), Some("end"));
}

#[test]
fn compare_catches_changes() {
    let cap = synthetic();
    let a = draw(vec![(0, 0xa0), (1, 0xa1)], 0xa2, 30);
    let mut b = a.clone();
    b.ps_cb[0].1[5] ^= 1;
    b.blend_factor[2] = 0.0; // -0 → +0
    b.ps_samplers[1].1 = 0x63; // same description: no difference
    let diff = compare_draws(&cap, &a, &b);
    assert_eq!(diff.len(), 2, "{diff:?}");
    assert!(diff[0].starts_with("ps_cb"));
    assert!(diff[1].starts_with("blend_factor"));
}

/// `FFX_DRAW_CAPTURE=<capture.ffxgfx>`: the scene of a real capture
/// replays draw for draw, and every state description survives the
/// field-by-field decoding.
#[test]
fn real_capture() {
    let Ok(path) = std::env::var("FFX_DRAW_CAPTURE") else {
        eprintln!("FFX_DRAW_CAPTURE unset: skipped");
        return;
    };
    let cap = Capture::parse(&std::fs::read(path).unwrap()).unwrap();
    for m in cap.objects.values() {
        if let Msg::State { kind, desc, .. } = m {
            let k = match kind {
                ffx_gfx::StateKind::Blend => crate::desc::StateKind::Blend,
                ffx_gfx::StateKind::Depth => crate::desc::StateKind::Depth,
                ffx_gfx::StateKind::Raster => crate::desc::StateKind::Raster,
                ffx_gfx::StateKind::Sampler => crate::desc::StateKind::Sampler,
            };
            let clean = crate::desc::without_padding(k, desc).unwrap();
            assert_eq!(clean.len(), desc.len());
        }
    }
    let (_, seen) = run(&cap);
    let sel = select_scene(&cap.frame).unwrap();
    eprintln!(
        "{} scene draws, {} commands, {} other draws inside",
        sel.draws,
        sel.cmds.len(),
        sel.other_draws
    );
    assert_eq!(seen.len(), sel.draws);
    for (i, (orig, replayed)) in seen.iter().enumerate() {
        assert_eq!(
            compare_draws(&cap, orig, replayed),
            Vec::<String>::new(),
            "scene draw {i}"
        );
    }
}
