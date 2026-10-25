//! The census on a synthetic frame and game state: an effect draw whose
//! ring bytes are a geometry's segment arrays is traced to that geometry,
//! its key's ppp program and its place in `DrawMostVFX`'s list; other
//! bytes are not; scene, post and UI draws are told apart.

use ffx_fxdraw::capture::Capture;
use ffx_fxdraw::census::{self, Producer};
use ffx_fxdraw::game::{self, List};
use ffx_fxdraw::mem::Mem;
use ffx_fxdraw::shaders::Families;
use ffx_gfx::{Cmd, Draw, DrawKind, Element, Frame, Msg};

const RING_VB: u32 = 0x100;
const RING_IB: u32 = 0x104;
const VS_VFX: u32 = 0x200;
const PS_VFX: u32 = 0x204;
const VS_POST: u32 = 0x208;
const LAYOUT: u32 = 0x300;
const TARGET: u32 = 0x400;
const DEPTH: u32 = 0x404;

fn shader_bytes(tag: u8) -> Vec<u8> {
    // Not real DXBC: the census only keys shaders by their bytes.
    vec![b'S', tag, 1, 2, 3]
}

fn draw(vertex_offset: u32, index_offset: u32, count: u32, vs: u32, ps: u32) -> Draw {
    Draw {
        kind: DrawKind::Indexed,
        count,
        instances: 0,
        start: 0,
        base: 0,
        first_instance: 0,
        topology: 4,
        layout: LAYOUT,
        vs,
        ps,
        vertex: vec![
            (0, RING_VB, 12, vertex_offset),
            (1, RING_VB, 12, vertex_offset + 0x1000),
            (2, RING_VB, 16, vertex_offset + 0x2000),
            (3, RING_VB, 8, vertex_offset + 0x3000),
        ],
        index: (RING_IB, 57, index_offset),
        vs_cb: Vec::new(),
        ps_cb: Vec::new(),
        vs_srv: Vec::new(),
        ps_srv: Vec::new(),
        vs_samplers: Vec::new(),
        ps_samplers: Vec::new(),
        blend: 0,
        blend_factor: [0.0; 4],
        sample_mask: u32::MAX,
        depth: 0,
        stencil_ref: 0,
        raster: 0,
        viewport: [0.0, 0.0, 1280.0, 720.0, 0.0, 1.0],
        scissor: [0; 4],
        rtv: vec![(0, TARGET)],
        dsv: DEPTH,
    }
}

/// The four vertex arrays of 4 vertices: positions, normals, colours, uvs.
fn positions(seed: u8) -> Vec<Vec<u8>> {
    [48u8, 48, 64, 32]
        .iter()
        .enumerate()
        .map(|(k, &n)| {
            (0..n)
                .map(|i| {
                    i.wrapping_mul(7)
                        .wrapping_add(seed)
                        .wrapping_add(31u8.wrapping_mul(u8::try_from(k).unwrap_or(0)))
                })
                .collect()
        })
        .collect()
}

/// Game memory: the scene pointer, its `DrawMostVFX` list with one mesh
/// instance, the table with one geometry (key: object 0x50000, program
/// 1), its render data with one segment of 4 vertices and 6 indices, the
/// ppp object's program list naming record 2 of `g_pppProgramTable`.
#[allow(clippy::many_single_char_names)]
fn memory(pos: &[Vec<u8>]) -> Mem {
    let scene = 0x0100_0000u32;
    let table = 0x0200_0000u32;
    let head = 0x0200_1000u32;
    let node = 0x0200_2000u32;
    let geom = 0x0300_0000u32;
    let rd = 0x0300_1000u32;
    let seg = 0x0300_2000u32;
    let arrays = 0x0400_0000u32;
    let mi = 0x0500_0000u32;
    let object = 0x0005_0000u32;
    let def = 0x0006_0000u32;
    let prog = 0x0007_0000u32;
    let mut pieces: Vec<(u32, Vec<u8>)> = Vec::new();
    let w = |v: &mut Vec<u8>, at: usize, x: u32| v[at..at + 4].copy_from_slice(&x.to_le_bytes());
    // globals
    let mut g = vec![0u8; 0x10];
    w(&mut g, 0, scene);
    pieces.push((game::SCENE, g));
    let mut t = vec![0u8; 4];
    w(&mut t, 0, table);
    pieces.push((game::TABLE, t));
    // the program table: record 2 names "pppTest"
    let mut pt = vec![0u8; 0x28 * 4 + 16];
    let rec = game::PROGRAM_TABLE + 0x28 * 2;
    let name_at = game::PROGRAM_TABLE + 0x28 * 4;
    w(&mut pt, 0x28 * 2, name_at);
    pt[0x28 * 4..0x28 * 4 + 8].copy_from_slice(b"pppTest\0");
    pieces.push((game::PROGRAM_TABLE, pt));
    // the scene: DrawMostVFX holds [0, mi]
    let mut s = vec![0u8; 0x10000];
    w(&mut s, 0x38cc + 4, mi);
    w(&mut s, 0xf8cc, 2);
    pieces.push((scene, s));
    // the table and its tree: head <-> one node
    let mut tb = vec![0u8; 0x30];
    w(&mut tb, 0, head);
    pieces.push((table, tb));
    let mut h = vec![0u8; 0x20];
    w(&mut h, 0, node);
    w(&mut h, 4, node);
    w(&mut h, 8, node);
    h[0xd] = 1;
    pieces.push((head, h));
    let mut n = vec![0u8; 0x20];
    w(&mut n, 0, head);
    w(&mut n, 4, head);
    w(&mut n, 8, head);
    w(&mut n, 0x10, 1);
    w(&mut n, 0x14, object);
    w(&mut n, 0x18, geom);
    pieces.push((node, n));
    // the geometry, render data, segment
    let mut ge = vec![0u8; 0x100];
    w(&mut ge, 0, game::VFX_GEOMETRY_VTABLE);
    w(&mut ge, 0xc, mi);
    w(&mut ge, 0x24, rd);
    w(&mut ge, 0x3c, 0xb);
    pieces.push((geom, ge));
    let mut r = vec![0u8; 0x100];
    w(&mut r, 0x90, 1);
    w(&mut r, 0x94, seg);
    pieces.push((rd, r));
    let mut sg = vec![0u8; 0x6c];
    for (at, v) in [
        (4, 4),
        (8, 6),
        (0xc, arrays),
        (0x10, arrays + 0x100),
        (0x14, arrays + 0x200),
        (0x18, arrays + 0x300),
        (0x1c, arrays + 0x400),
    ] {
        w(&mut sg, at, v);
    }
    pieces.push((seg, sg));
    let mut a = vec![0u8; 0x500];
    for (k, v) in pos.iter().enumerate() {
        a[0x100 * k..0x100 * k + v.len()].copy_from_slice(v);
    }
    for (i, ix) in [0u16, 1, 2, 2, 1, 3].iter().enumerate() {
        a[0x400 + 2 * i..0x402 + 2 * i].copy_from_slice(&ix.to_le_bytes());
    }
    pieces.push((arrays, a));
    // the ppp object -> def -> prog -> entry 1 -> record 2
    let mut o = vec![0u8; 0x10];
    w(&mut o, 8, def);
    pieces.push((object, o));
    let mut d = vec![0u8; 0x10];
    w(&mut d, 0, prog);
    pieces.push((def, d));
    let mut p = vec![0u8; 0x60];
    p[0x26] = 2;
    w(&mut p, 0x28 + 0x10, rec);
    pieces.push((prog, p));
    Mem::from_pieces(pieces)
}

fn capture(pos: &[Vec<u8>]) -> Capture {
    let ring = |id| Msg::Buffer {
        id,
        desc: [0x10000, 2, 1, 0x10000, 0, 0],
        data: Vec::new(),
    };
    let elements = |slot: u32, semantic: &str| Element {
        semantic: semantic.into(),
        index: 0,
        format: 6,
        slot,
        offset: 0,
        per_instance: false,
        step: 0,
    };
    let mut cmds = vec![
        // the effect draw: ring bytes = the segment's arrays (vertices 0..3)
        Cmd::UpdateBuffer {
            id: RING_VB,
            offset: 0x40,
            data: pos[0].clone(),
        },
        Cmd::UpdateBuffer {
            id: RING_VB,
            offset: 0x1040,
            data: pos[1].clone(),
        },
        Cmd::UpdateBuffer {
            id: RING_VB,
            offset: 0x2040,
            data: pos[2].clone(),
        },
        Cmd::UpdateBuffer {
            id: RING_VB,
            offset: 0x3040,
            data: pos[3].clone(),
        },
        Cmd::UpdateBuffer {
            id: RING_IB,
            offset: 0x20,
            data: [5u16, 6, 7, 7, 6, 8]
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect(),
        },
    ];
    // indices 5..8 with base -5: vertices 0..3 of the binding
    let mut d = draw(0x40, 0x20, 6, VS_VFX, PS_VFX);
    d.base = -5;
    cmds.push(Cmd::Draw(Box::new(d)));
    // the same shaders with other bytes: an unmatched effect
    cmds.push(Cmd::UpdateBuffer {
        id: RING_VB,
        offset: 0x800,
        data: vec![0xee; 48],
    });
    let mut d = draw(0x800, 0x20, 6, VS_VFX, PS_VFX);
    d.base = -5;
    cmds.push(Cmd::Draw(Box::new(d)));
    // a post pass
    let mut d = draw(0x40, 0x20, 6, VS_POST, VS_POST);
    d.vertex.truncate(2);
    d.dsv = 0;
    d.base = -5;
    cmds.push(Cmd::Draw(Box::new(d)));
    let msgs = vec![
        ring(RING_VB),
        ring(RING_IB),
        Msg::Shader {
            id: VS_VFX,
            kind: b'v',
            bytecode: shader_bytes(1),
        },
        Msg::Shader {
            id: PS_VFX,
            kind: b'p',
            bytecode: shader_bytes(2),
        },
        Msg::Shader {
            id: VS_POST,
            kind: b'v',
            bytecode: shader_bytes(3),
        },
        Msg::Layout {
            id: LAYOUT,
            elements: vec![
                elements(0, "POSITION"),
                elements(1, "NORMAL"),
                elements(2, "COLOR"),
                elements(3, "TEXCOORD"),
            ],
        },
        Msg::World {
            scene: "test0000".into(),
            camera: None,
        },
        Msg::Frame(Frame {
            number: 1,
            width: 1280,
            height: 720,
            cmds,
        }),
    ];
    Capture::from_msgs(&msgs)
}

fn families() -> Families {
    let mut f = Families::default();
    f.by_key.insert(
        ffx_drawdiff::key::shader(&shader_bytes(1)),
        "vfxzmatadditiveshader.fx".into(),
    );
    f.by_key.insert(
        ffx_drawdiff::key::shader(&shader_bytes(2)),
        "vfxzmatadditiveshader.fx".into(),
    );
    f.by_key.insert(
        ffx_drawdiff::key::shader(&shader_bytes(3)),
        "copybuffer.fx".into(),
    );
    f
}

#[test]
fn an_effect_draw_is_traced_to_its_geometry_program_and_list() {
    let pos = positions(3);
    let lines = census::census(&capture(&pos), 0, &memory(&pos), &families(), None).lines;
    assert_eq!(lines.len(), 3);
    let Producer::Vfx(v) = &lines[0].producer else {
        panic!("draw 0: {:?}", lines[0].producer)
    };
    assert_eq!(v.key, 0x0005_0000_0000_0001);
    assert_eq!((v.segment, v.buffer), (0, 0));
    assert_eq!((v.list, v.place), (List::Most, 1));
    assert_eq!(v.flags, 0xb);
    let ffx_fxdraw::game::Maker::Ppp {
        object,
        index,
        program: Some((_, name)),
        ..
    } = &v.maker
    else {
        panic!("{:?}", v.maker)
    };
    assert_eq!(
        (*object, *index, name.as_str()),
        (0x0005_0000, 1, "pppTest")
    );
}

#[test]
fn other_bytes_are_an_unmatched_effect_and_a_copy_is_post() {
    let pos = positions(3);
    let lines = census::census(&capture(&pos), 0, &memory(&pos), &families(), None).lines;
    assert!(
        matches!(lines[1].producer, Producer::UnmatchedEffect(_)),
        "{:?}",
        lines[1].producer
    );
    assert_eq!(lines[2].producer, Producer::Post);
    let c = census::census(&capture(&pos), 0, &memory(&pos), &families(), None);
    assert!(!c.traced());
    assert!(c.missing.is_empty(), "the one expected draw was taken");
}

#[test]
fn a_rebased_image_moves_the_globals_only() {
    let mut m = Mem::from_pieces(vec![(0x0050_0000, vec![1, 0, 0, 0])]);
    m.image_base = 0x0050_0000;
    assert_eq!(m.image(0x0040_0000), 0x0050_0000);
    assert_eq!(m.u32(m.image(0x0040_0000)), Some(1));
    assert_eq!(m.unrebase(0x0050_0010), 0x0040_0010);
    assert_eq!(m.unrebase(0x0800_0000), 0x0800_0000);
}
