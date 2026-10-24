//! The drawdiff gate on synthetic captures (`docs/drawdiff.md`, "Gate"):
//! each change is caught and classed, and the changes that do not alter
//! the output (an unused constant, a ring offset with the same bytes) are
//! not reported.

use ffx_drawdiff::capture::Capture;
use ffx_drawdiff::diff::{Comparison, Options, Part, compare};
use ffx_drawdiff::{Class, Value, rules};
use ffx_gfx::{Cmd, Draw, DrawKind, Element, Frame, Msg, StateKind, Subresource, ViewKind};

// ---- a shader with reflection ----

/// A DXBC container with an `RDEF` chunk (shader model 4): constant buffer
/// `Material` at `b0` with these variables (name, offset, size, used),
/// texture `t0` and sampler `s0`; and a one-token `SHDR` of `kind`.
fn shader(kind: u32, vars: &[(&str, u32, u32, bool)]) -> Vec<u8> {
    let mut strings: Vec<u8> = Vec::new();
    let mut names = Vec::new();
    let add = |s: &str, strings: &mut Vec<u8>| {
        let at = strings.len();
        strings.extend(s.as_bytes());
        strings.push(0);
        at
    };
    let cb_name = add("Material", &mut strings);
    let tex_name = add("Diffuse", &mut strings);
    let samp_name = add("DiffuseSampler", &mut strings);
    let creator = add("synthetic", &mut strings);
    for v in vars {
        names.push(add(v.0, &mut strings));
    }
    // Layout: header 28, bindings 3 x 32, cbuffer 24, variables 24 each,
    // one float4 type 16, strings.
    let bind_off = 28;
    let cb_off = bind_off + 3 * 32;
    let var_off = cb_off + 24;
    let type_off = var_off + 24 * vars.len();
    let str_off = type_off + 16;
    let w = |v: &mut Vec<u8>, x: usize| v.extend(u32::try_from(x).unwrap().to_le_bytes());
    let mut r = Vec::new();
    w(&mut r, 1);
    w(&mut r, cb_off);
    w(&mut r, 3);
    w(&mut r, bind_off);
    r.extend([0, 4]); // minor, major
    r.extend([0xff, 0xff]); // program type
    w(&mut r, 0); // flags
    w(&mut r, str_off + creator);
    for (name, kind, dim) in [(cb_name, 0, 0), (tex_name, 2, 4), (samp_name, 3, 0)] {
        for x in [str_off + name, kind, 0, dim, 0, 0, 1, 0] {
            w(&mut r, x);
        }
    }
    for x in [str_off + cb_name, vars.len(), var_off, 64, 0, 0] {
        w(&mut r, x);
    }
    for (v, name) in vars.iter().zip(&names) {
        let flags = if v.3 { 2 } else { 0 };
        for x in [
            str_off + name,
            v.1 as usize,
            v.2 as usize,
            flags,
            type_off,
            0,
        ] {
            w(&mut r, x);
        }
    }
    // float4: class vector, base float, 1 row, 4 columns.
    for x in [1u16, 3, 1, 4, 0, 0] {
        r.extend(x.to_le_bytes());
    }
    w(&mut r, 0);
    r.extend(&strings);
    let shdr = (kind << 16 | 0x40).to_le_bytes().to_vec();
    let chunks: [(&[u8; 4], &[u8]); 2] = [(b"RDEF", &r), (b"SHDR", &shdr)];
    let mut out = b"DXBC".to_vec();
    out.extend([0u8; 16]);
    w(&mut out, 1);
    let total_at = out.len();
    w(&mut out, 0);
    w(&mut out, chunks.len());
    let mut at = out.len() + 4 * chunks.len();
    for (_, data) in &chunks {
        w(&mut out, at);
        at += 8 + data.len();
    }
    for (tag, data) in chunks {
        out.extend(tag);
        w(&mut out, data.len());
        out.extend(data);
    }
    let total = out.len();
    out[total_at..total_at + 4].copy_from_slice(&u32::try_from(total).unwrap().to_le_bytes());
    out
}

fn words(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn floats(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

// ---- the scene ----

const VS: u32 = 0x100;
const PS: u32 = 0x101;
const LAYOUT: u32 = 0x102;
const VB: u32 = 0x200;
const IB: u32 = 0x201;
const RING: u32 = 0x202;
const TEX_A: u32 = 0x300;
const TEX_B: u32 = 0x301;
const SRV_A: u32 = 0x310;
const SRV_B: u32 = 0x311;
const COLOR: u32 = 0x320;
const NORMAL: u32 = 0x321;
const DEPTH: u32 = 0x322;
const RTV0: u32 = 0x330;
const RTV1: u32 = 0x331;
const DSV: u32 = 0x332;
const BLEND: u32 = 0x400;
const BLEND_ADD: u32 = 0x401;
const DEPTH_STATE: u32 = 0x402;
const RASTER: u32 = 0x403;
const SAMPLER: u32 = 0x404;

/// The ring's contents: 8 vertices of 16 bytes at `at`, zeros elsewhere.
fn ring_bytes(at: usize) -> Vec<u8> {
    let mut b = vec![0u8; 512];
    for i in 0..128 {
        b[at + i] = u8::try_from(i * 3 % 251).unwrap();
    }
    b
}

fn texture(id: u32, seed: u8, target: bool) -> Msg {
    let bind = if target { 0x28 } else { 0x8 };
    Msg::Texture {
        id,
        desc: [4, 4, 1, 1, 28, 1, 0, 0, bind, 0, 0],
        subresources: vec![Subresource {
            row_pitch: 16,
            data: (0..64u8).map(|i| i.wrapping_mul(seed)).collect(),
        }],
    }
}

fn blend(src: u32) -> Vec<u8> {
    let mut b = words(&[0, 0]);
    for i in 0..8 {
        let enable = u32::from(i == 0);
        b.extend(words(&[enable, src, 6, 1, 2, 1, 1]));
        b.extend([0xf, 0, 0, 0]);
    }
    b
}

fn objects() -> Vec<Msg> {
    let used = [
        ("MaterialColour", 0, 16, true),
        ("Unused", 16, 16, false),
        ("Scale", 32, 16, true),
    ];
    let view = |id, kind, resource| Msg::View {
        id,
        kind,
        resource,
        desc: None,
    };
    vec![
        Msg::Shader {
            id: VS,
            kind: b'v',
            bytecode: shader(1, &[]),
        },
        Msg::Shader {
            id: PS,
            kind: b'p',
            bytecode: shader(0, &used),
        },
        Msg::Layout {
            id: LAYOUT,
            elements: vec![Element {
                semantic: "POSITION".into(),
                index: 0,
                format: 2,
                slot: 0,
                offset: 0,
                per_instance: false,
                step: 0,
            }],
        },
        Msg::Buffer {
            id: VB,
            desc: [1024, 0, 1, 0, 0, 0],
            data: (0..1024u32)
                .map(|i| u8::try_from(i % 256).unwrap())
                .collect(),
        },
        Msg::Buffer {
            id: IB,
            desc: [64, 0, 2, 0, 0, 0],
            data: words(&[0x0001_0000, 0x0003_0002, 0x0005_0004, 0x0007_0006]),
        },
        Msg::Buffer {
            id: RING,
            desc: [512, 2, 1, 0x1_0000, 0, 0],
            data: ring_bytes(0),
        },
        texture(TEX_A, 3, false),
        texture(TEX_B, 5, false),
        texture(COLOR, 0, true),
        texture(NORMAL, 0, true),
        Msg::Texture {
            id: DEPTH,
            desc: [4, 4, 1, 1, 40, 1, 0, 0, 0x40, 0, 0],
            subresources: vec![],
        },
        view(SRV_A, ViewKind::ShaderResource, TEX_A),
        view(SRV_B, ViewKind::ShaderResource, TEX_B),
        view(RTV0, ViewKind::RenderTarget, COLOR),
        view(RTV1, ViewKind::RenderTarget, NORMAL),
        view(DSV, ViewKind::DepthStencil, DEPTH),
        Msg::State {
            id: BLEND,
            kind: StateKind::Blend,
            desc: blend(5),
        },
        Msg::State {
            id: BLEND_ADD,
            kind: StateKind::Blend,
            desc: blend(2),
        },
        Msg::State {
            id: DEPTH_STATE,
            kind: StateKind::Depth,
            desc: {
                let mut d = words(&[1, 1, 4, 0]);
                d.extend([0xff, 0xff, 0, 0]);
                d.extend(words(&[1, 1, 1, 8, 1, 1, 1, 8]));
                d
            },
        },
        Msg::State {
            id: RASTER,
            kind: StateKind::Raster,
            desc: words(&[3, 1, 0, 0, 0, 0, 1, 0, 0, 0]),
        },
        Msg::State {
            id: SAMPLER,
            kind: StateKind::Sampler,
            desc: words(&[0x15, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0x7f7f_ffff]),
        },
    ]
}

/// The constant window: colour, an unused float4, a scale.
fn constants(colour: [f32; 4], unused: [f32; 4], scale: f32) -> Vec<u8> {
    let mut c = floats(&colour);
    c.extend(floats(&unused));
    c.extend(floats(&[scale, scale, scale, 1.0]));
    c.extend([0u8; 16]);
    c
}

fn draw(start: u32, count: u32, scale: f32) -> Draw {
    Draw {
        kind: DrawKind::Indexed,
        count,
        instances: 1,
        start,
        base: 0,
        first_instance: 0,
        topology: 4,
        layout: LAYOUT,
        vs: VS,
        ps: PS,
        vertex: vec![(0, VB, 12, 0)],
        index: (IB, 57, 0),
        vs_cb: vec![],
        ps_cb: vec![(0, constants([1.0, 0.5, 0.25, 1.0], [0.0; 4], scale))],
        vs_srv: vec![],
        ps_srv: vec![(0, SRV_A)],
        vs_samplers: vec![],
        ps_samplers: vec![(0, SAMPLER)],
        blend: BLEND,
        blend_factor: [0.0; 4],
        sample_mask: !0,
        depth: DEPTH_STATE,
        stencil_ref: 0,
        raster: RASTER,
        viewport: [0.0, 0.0, 4.0, 4.0, 0.0, 1.0],
        scissor: [0, 0, 4, 4],
        rtv: vec![(0, RTV0), (1, RTV1)],
        dsv: DSV,
    }
}

/// A ring draw: 8 plain vertices of 16 bytes from the ring.
fn ring_draw(vb_offset: u32, start: u32) -> Draw {
    let mut d = draw(start, 8, 1.0);
    d.kind = DrawKind::Plain;
    d.vertex = vec![(0, RING, 16, vb_offset)];
    d.index = (0, 0, 0);
    d
}

/// The frame: clears, six draws (the third and fourth the same mesh with
/// other constants), a ring draw.
fn cmds() -> Vec<Cmd> {
    let mut c = vec![
        Cmd::ClearTarget {
            view: RTV0,
            color: [0.0, 0.0, 0.0, 1.0],
        },
        Cmd::ClearDepth {
            view: DSV,
            flags: 3,
            depth: 1.0,
            stencil: 0,
        },
    ];
    for (start, count, scale) in [
        (0, 6, 1.0),
        (2, 6, 1.0),
        (0, 3, 1.0),
        (0, 3, 2.0),
        (4, 3, 1.0),
        (1, 6, 1.0),
    ] {
        c.push(Cmd::Draw(Box::new(draw(start, count, scale))));
    }
    c.push(Cmd::UpdateBuffer {
        id: RING,
        offset: 0,
        data: ring_bytes(0),
    });
    c.push(Cmd::Draw(Box::new(ring_draw(0, 0))));
    c
}

fn capture(objs: &[Msg], cmds: Vec<Cmd>) -> Capture {
    let mut msgs = objs.to_vec();
    msgs.push(Msg::World {
        scene: "test0000".into(),
        camera: None,
    });
    msgs.push(Msg::Frame(Frame {
        number: 0,
        width: 4,
        height: 4,
        cmds,
    }));
    // Through the file format, as drawdiff reads it.
    let mut bytes = ffx_gfx::CAPTURE_MAGIC.to_vec();
    for m in &msgs {
        m.write(&mut bytes);
    }
    Capture::parse(&bytes).unwrap()
}

fn base() -> Capture {
    capture(&objects(), cmds())
}

fn run(b: &Capture) -> Comparison {
    compare(&base(), b, &Options::default())
}

/// The (class, field) of every difference.
fn classes(c: &Comparison) -> Vec<(Class, String)> {
    c.frames[0]
        .diffs
        .iter()
        .map(|d| (d.class, d.field.clone()))
        .collect()
}

fn draw_at(cmds: &mut [Cmd], i: usize) -> &mut Draw {
    match &mut cmds[i] {
        Cmd::Draw(d) => d,
        _ => panic!("not a draw"),
    }
}

#[test]
fn a_capture_equals_itself() {
    let c = run(&base());
    assert_eq!(c.unallowed(), 0);
    assert!(c.frames[0].diffs.is_empty());
    assert_eq!(c.frames[0].a_items, 9);
    assert_eq!(c.frames[0].equal, 9);
}

#[test]
fn a_dropped_draw_is_missing() {
    let mut cmds = cmds();
    // The fourth draw: the same mesh as the third, other constants.
    cmds.remove(5);
    let c = run(&capture(&objects(), cmds));
    assert_eq!(classes(&c), vec![(Class::Missing, String::new())]);
    assert_eq!(c.frames[0].diffs[0].a, Some(5));
}

#[test]
fn two_swapped_draws_are_moved() {
    let mut cmds = cmds();
    cmds.swap(3, 6);
    let c = run(&capture(&objects(), cmds));
    let found = classes(&c);
    assert_ne!(found, vec![]);
    assert!(found.iter().all(|(k, _)| *k == Class::Moved), "{found:?}");
}

#[test]
fn one_ulp_in_a_used_constant() {
    let mut cmds = cmds();
    let d = draw_at(&mut cmds, 2);
    d.ps_cb[0].1[4..8].copy_from_slice(&f32::from_bits(0.5f32.to_bits() + 1).to_le_bytes());
    let c = run(&capture(&objects(), cmds.clone()));
    assert_eq!(
        classes(&c),
        vec![(Class::Constant, "ps.cb0.MaterialColour[0].y".into())]
    );
    let d = &c.frames[0].diffs[0];
    assert_eq!(d.var.as_deref(), Some("MaterialColour"));
    assert_eq!(d.va, Value::F(0.5f32.to_bits()));
    // A tolerance with its cause covers it; without the cause it is refused.
    let tol = rules::parse("tol", "ulp 1 var=MaterialColour cause: test").unwrap();
    let opts = Options {
        tolerances: tol,
        ..Options::default()
    };
    let c = compare(&base(), &capture(&objects(), cmds), &opts);
    assert_eq!(c.unallowed(), 0);
    assert_eq!(c.frames[0].tolerated, 1);
    assert!(rules::parse("tol", "ulp 1 var=MaterialColour").is_err());
}

#[test]
fn an_unused_constant_is_not_compared() {
    let mut cmds = cmds();
    let d = draw_at(&mut cmds, 2);
    d.ps_cb[0].1[16..20].copy_from_slice(&123.0f32.to_le_bytes());
    // Past the used variables too.
    d.ps_cb[0].1[60] = 0x55;
    let c = run(&capture(&objects(), cmds));
    assert!(classes(&c).is_empty(), "{:?}", classes(&c));
}

#[test]
fn signed_zero_and_nan_compare_by_bits() {
    let mut cmds = cmds();
    let d = draw_at(&mut cmds, 2);
    d.ps_cb[0].1[32..36].copy_from_slice(&(-0.0f32).to_le_bytes());
    let d = draw_at(&mut cmds, 3);
    d.viewport[0] = f32::from_bits(0x7fc0_0001);
    let mut a_cmds = self::cmds();
    draw_at(&mut a_cmds, 2).ps_cb[0].1[32..36].copy_from_slice(&0.0f32.to_le_bytes());
    draw_at(&mut a_cmds, 3).viewport[0] = f32::from_bits(0x7fc0_0000);
    let c = compare(
        &capture(&objects(), a_cmds),
        &capture(&objects(), cmds),
        &Options::default(),
    );
    assert_eq!(
        classes(&c),
        vec![
            (Class::Constant, "ps.cb0.Scale[0].x".into()),
            (Class::State, "viewport.x".into()),
        ]
    );
}

#[test]
fn a_blend_state_field() {
    let mut objs = objects();
    // The same id, SrcBlend 5 -> 4.
    for m in &mut objs {
        if let Msg::State {
            id: BLEND, desc, ..
        } = m
        {
            *desc = blend(4);
        }
    }
    let c = run(&capture(&objs, cmds()));
    let found = classes(&c);
    assert_eq!(found.len(), 7, "{found:?}");
    assert!(
        found
            .iter()
            .all(|f| *f == (Class::State, "blend.rt0.src".to_owned()))
    );
    // Another state object with the same fields is no difference.
    let mut cmds = cmds();
    for c in &mut cmds {
        if let Cmd::Draw(d) = c {
            d.blend = BLEND_ADD;
        }
    }
    let mut objs = objects();
    for m in &mut objs {
        if let Msg::State {
            id: BLEND_ADD,
            desc,
            ..
        } = m
        {
            *desc = blend(5);
        }
    }
    assert_eq!(classes(&run(&capture(&objs, cmds))), vec![]);
}

#[test]
fn a_swapped_texture() {
    let mut cmds = cmds();
    draw_at(&mut cmds, 4).ps_srv = vec![(0, SRV_B)];
    let c = run(&capture(&objects(), cmds));
    assert_eq!(classes(&c), vec![(Class::Resource, "ps.srv0".into())]);
}

#[test]
fn a_texture_at_another_address_is_the_same() {
    // Every object moved: ids are addresses, keys are contents.
    let mut objs = objects();
    let mut cmds = cmds();
    for m in &mut objs {
        if let Msg::Texture { id, .. } = m
            && *id == TEX_A
        {
            *id = 0x9000;
        }
        if let Msg::View { resource, .. } = m
            && *resource == TEX_A
        {
            *resource = 0x9000;
        }
    }
    draw_at(&mut cmds, 4).ps_srv = vec![(0, SRV_A)];
    assert_eq!(classes(&run(&capture(&objs, cmds))), vec![]);
}

#[test]
fn a_shifted_ring_offset_with_the_same_bytes() {
    // The same 8 vertices 96 bytes further into the ring, by the binding's
    // offset, and by the draw's start.
    for (vb_offset, start) in [(96, 0), (0, 6)] {
        let mut cmds = cmds();
        cmds[8] = Cmd::UpdateBuffer {
            id: RING,
            offset: 0,
            data: ring_bytes(96),
        };
        cmds[9] = Cmd::Draw(Box::new(ring_draw(vb_offset, start)));
        let c = run(&capture(&objects(), cmds));
        assert!(classes(&c).is_empty(), "{:?}", classes(&c));
    }
    // Other bytes there are a resource difference of the same draw.
    let mut cmds = cmds();
    cmds[8] = Cmd::UpdateBuffer {
        id: RING,
        offset: 0,
        data: vec![7; 512],
    };
    let c = run(&capture(&objects(), cmds));
    assert_eq!(classes(&c), vec![(Class::Resource, "vb0".into())]);
}

#[test]
fn a_static_buffer_offset_is_compared() {
    let mut cmds = cmds();
    draw_at(&mut cmds, 2).vertex = vec![(0, VB, 12, 12)];
    let c = run(&capture(&objects(), cmds));
    assert_eq!(classes(&c), vec![(Class::State, "vb0.offset".into())]);
}

#[test]
fn the_scene_part_and_the_allow_list() {
    let mut cmds = cmds();
    // A draw outside the scene targets (one render target, no depth).
    let mut ui = draw(0, 6, 1.0);
    ui.rtv = vec![(0, RTV0)];
    ui.dsv = 0;
    cmds.push(Cmd::Draw(Box::new(ui)));
    let b = capture(&objects(), cmds);
    let all = run(&b);
    assert_eq!(classes(&all), vec![(Class::Extra, String::new())]);
    let scene = compare(
        &base(),
        &b,
        &Options {
            part: Part::Scene,
            ..Options::default()
        },
    );
    assert_eq!(scene.unallowed(), 0);
    let allow = rules::parse("allow", "[test0000]\nextra kind=draw cause: a test\n").unwrap();
    let allowed = compare(
        &base(),
        &b,
        &Options {
            allow: allow.clone(),
            ..Options::default()
        },
    );
    assert_eq!(allowed.unallowed(), 0);
    assert_eq!(allowed.frames[0].diffs[0].allowed, Some(0));
    // Another scene's rule does not apply.
    let other = rules::parse("allow", "[cdsp0000]\nextra cause: a test\n").unwrap();
    let c = compare(
        &base(),
        &b,
        &Options {
            allow: other,
            ..Options::default()
        },
    );
    assert_eq!(c.unallowed(), 1);
}
