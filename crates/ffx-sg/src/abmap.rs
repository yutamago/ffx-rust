//! Sphere grid (ability map) draws in the main module's range, called by
//! the grid's draw pass `0x00a54660`. They hand each draw to the effect
//! model drawers (`op1_md_draw_eiabm`, `op1_md_draw_eiabm_sphe`) with a
//! request built in their own frame; the bridges build it in `work`, a
//! block laid out as that frame (see [`bridge::NODE_WORK`]).

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use std::cmp::Ordering;

    use ffx_bridge::{Global, Mem, x87};

    /// The sphere grid data (`g_Abmap`, `LoadAbmap`): `i16 +2` node count,
    /// nodes of 0x28 bytes from `+0x808`, node kinds of 0x30 bytes from
    /// `+0xf834`.
    pub const ABMAP: Global<u32> = Global::new(0x0230_5834);
    /// The grid file (`g_AbmapFile`), the draw requests' target.
    pub const ABMAP_FILE: u32 = 0x0174_0870;
    /// The halo model (`0x023057f4`).
    pub const HALO_MODEL: u32 = 0x0230_57f4;
    /// The emulated EE zero register (`g_Ee_Gpr`, four words).
    pub const EE_R0: u32 = 0x00c8_f508;
    /// `AbmapMakeNodeMatrix(out, node, scale)`.
    pub const MAKE_NODE_MATRIX: u32 = 0x00a5_ad30;
    /// `FFXVu0MulMatrix(out, a, b)`.
    pub const MUL_MATRIX: u32 = 0x006e_d5c0;
    /// `op1_md_draw_eiabm(model, request, n, layer)`.
    pub const MD_DRAW: u32 = 0x00a6_57c0;
    /// `.rdata`: 65536 (`f64`), 0.5 (`f64`), 1.3 (`f64`, from `f32`), 5
    /// (`f32`).
    const K65536: u32 = 0x00b5_1558;
    const KHALF: u32 = 0x00b9_2220;
    const K1_3: u32 = 0x00b4_bf00;
    const KFIVE: u32 = 0x00b3_eb98;

    /// The node sphere model (`0x023057ec`).
    pub const SPHERE_MODEL: u32 = 0x0230_57ec;
    /// `AbmapMakeLinkMatrix(out, node, offset, scale)`.
    pub const MAKE_LINK_MATRIX: u32 = 0x00a5_a360;
    /// `op1_md_draw_eiabm_sphe(model, request, node, part)`.
    pub const MD_DRAW_SPHE: u32 = 0x00a6_8140;
    /// The parts' colours with the panel bit set (`0x00c8659c`) and clear
    /// (`0x00c86644`), seven RGBA words each.
    const LIT_COLORS: u32 = 0x00c8_659c;
    const DARK_COLORS: u32 = 0x00c8_6644;
    /// `g_pppSinTable`: 4096 `f32` sines over a turn.
    const SIN_TABLE: u32 = 0x00c4_4bf0;
    /// `.rdata`: 0.15 and -96 (`f64`), 6, -1 and 0.0008 (`f32`).
    const K0_15: u32 = 0x00b5_9870;
    const KMINUS96: u32 = 0x00b7_b718;
    const KSIX: u32 = 0x00b9_2308;
    const KMINUS1: u32 = 0x00b9_2340;
    const KLINK: u32 = 0x00b7_b5d4;

    /// The work block of [`abmap_draw_node_halos`] and
    /// [`abmap_draw_node_spheres`]: the original's frame
    /// from `ebp - 0x140`. The draw request (0x50 bytes) at 0, the view
    /// times node matrix at `+0x78`, the node matrix at `+0xb8`, the
    /// vertex block (four vectors) at `+0xf8`. 16-byte aligned, as the
    /// original aligns `ebp`.
    pub const NODE_WORK: u32 = 0x138;
    const VIEW_MATRIX: u32 = 0x78;
    const NODE_MATRIX: u32 = 0xb8;
    const VERTS: u32 = 0xf8;

    fn read4(mem: &impl Mem, at: u32) -> [u32; 4] {
        [0, 4, 8, 12].map(|o| mem.read::<u32>(at.wrapping_add(o)))
    }

    fn rdata_f64(mem: &impl Mem, at: u32) -> f64 {
        mem.read::<f64>(mem.rebase(at))
    }

    /// An `.rdata` `f32` through `fld`/`fstp`.
    fn rdata_f32(mem: &impl Mem, at: u32) -> u32 {
        x87::store(x87::load(mem.read(mem.rebase(at))))
    }

    /// The request's and vertex block's fixed fields, as both draws set
    /// them up: the vertex block is the EE zero register four times, the
    /// last with w 1.0; the request has the target `g_AbmapFile` at `+4`,
    /// zeros at `+0x0a` (byte), `+0x0c..+0x14`, `+0x1c`, `+0x38..+0x40`,
    /// the matrices at `+0x20` and `+0x2c`, `g + 0x11560` at `+0x24`, `g +
    /// 0x11520` at `+0x28`, the vertex block at `+0x44`.
    fn init_request<M: Mem>(mem: &mut M, work: u32, g: u32) {
        let (request, verts) = (work, work + VERTS);
        let r0 = read4(mem, mem.rebase(EE_R0));
        for k in 0..4 {
            for (i, w) in (0u32..).zip(r0) {
                mem.write(verts + 0x10 * k + 4 * i, w);
            }
        }
        mem.write(verts + 0x3c, 1f32.to_bits());
        for off in [0x0c, 0x10, 0x14, 0x1c, 0x38, 0x3c, 0x40] {
            mem.write(request + off, 0u32);
        }
        mem.write(request + 0xa, 0u8);
        mem.write(request + 4, mem.rebase(ABMAP_FILE));
        mem.write(request + 0x20, work + VIEW_MATRIX);
        mem.write(request + 0x2c, work + NODE_MATRIX);
        mem.write(request + 0x44, verts);
        mem.write(request + 0x28, g.wrapping_add(0x11520));
        mem.write(request + 0x24, g.wrapping_add(0x11560));
    }

    /// `AbmapDrawNodeHalos()` (`0x00a50290`, name ours; from the grid's
    /// draw pass `0x00a54660`): a halo (model `[0x023057f4]`) on the nodes
    /// near the cursor.
    ///
    /// Each node (`i16 g_Abmap[+2]` of them from `+0x808`) whose kind
    /// (`u16 +6`) is not `0xffff` gets `AbmapMakeNodeMatrix(m, node,
    /// 1.3 * size)` (size `f32 +0xf838` of the kind's 0x30-byte record).
    /// With `d2` the squared distance of `m`'s translation from the cursor
    /// (`+0x11520`, three floats) and `r` the cursor's range (`+0x115a0`):
    /// below `r * r * 0.5`, `v = trunc(sqrt(d2) * (65536 / r))`; `e = v`
    /// from 0x4000 on, else `(0x5000 - v) * 4`, and no halo when that
    /// reaches 0x10000 (the nodes right under the cursor). At most 25
    /// halos: the 26th ends the pass. A halo's strength is
    /// `a = ((0x10000 - e) * 0x60) >> 16`: alpha `a >> 3`, request byte
    /// `+0x4d` `2a`.
    ///
    /// The request (`work`, [`NODE_WORK`]): kind 0x44, the target
    /// `g_AbmapFile`, 0x48, colour 0x808080 and the alpha at `+0x18`, the
    /// view times node matrix (`FFXVu0MulMatrix` of `+0x113e0` and `m`) at
    /// `+0x20`, `g_Abmap + 0x11560` at `+0x24`, `+ 0x11520` at `+0x28`,
    /// `m` at `+0x2c`, the vertex block at `+0x44` (the EE zero register
    /// four times, but `(.., 5, 1)` in the last); `+0x0b`, `+0x30`,
    /// `+0x34`, `+0x48` stay unset. Draws through `op1_md_draw_eiabm(model,
    /// request, 4, g_Abmap[+0x116a4])`.
    #[allow(clippy::many_single_char_names)] // g, f, v, e, a: as documented
    pub fn abmap_draw_node_halos<M, C>(mem: &mut M, work: u32, call: &mut C)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let request = work;
        let (view_m, node_m, verts) = (work + VIEW_MATRIX, work + NODE_MATRIX, work + VERTS);
        let g = ABMAP.get(mem);
        let count = i32::from(mem.read::<i16>(g.wrapping_add(2)));
        let model: u32 = mem.read(mem.rebase(HALO_MODEL));
        init_request(mem, work, g);

        let range = x87::load(mem.read(g.wrapping_add(0x115a0)));
        let per_unit = x87::load(x87::store(x87::div(rdata_f64(mem, K65536), range)));
        let near = x87::load(x87::store(x87::mul(
            x87::mul(range, range),
            rdata_f64(mem, KHALF),
        )));
        let k13 = rdata_f64(mem, K1_3);
        let mut node = g.wrapping_add(0x808);
        let mut drawn = 0;
        for _ in 0..count.cast_unsigned() {
            let kind = mem.read::<u16>(node + 6);
            if kind != 0xffff {
                let g = ABMAP.get(mem);
                let size = x87::load(mem.read(g.wrapping_add(0xf838 + 0x30 * u32::from(kind))));
                let scale = x87::store(x87::mul(k13, size));
                let f = mem.rebase(MAKE_NODE_MATRIX);
                call(mem, f, &[node_m, node, scale]);
                let g = ABMAP.get(mem);
                let diff = |mem: &M, o: u32| {
                    let cursor = x87::load(mem.read(g.wrapping_add(0x11520 + o)));
                    x87::sub(cursor, x87::load(mem.read(node_m + 0x30 + o)))
                };
                let (dx, dy, dz) = (diff(mem, 0), diff(mem, 4), diff(mem, 8));
                let sum = x87::add(
                    x87::add(x87::mul(dx, dx), x87::mul(dy, dy)),
                    x87::mul(dz, dz),
                );
                let d2 = x87::load(x87::store(sum));
                // fcomp: the halo only when near > d2 (not for NaN)
                if near > d2 {
                    mem.write(request, 0x44u32);
                    let dist = x87::load(x87::store(x87::sqrt(d2.abs())));
                    let v = x87::ftol(mem, x87::load(x87::store(x87::mul(dist, per_unit))))
                        .cast_signed();
                    let e = if v >= 0x4000 {
                        v
                    } else {
                        (0x5000i32.wrapping_sub(v)).wrapping_mul(4)
                    };
                    if v >= 0x4000 || e < 0x1_0000 {
                        drawn += 1;
                        if drawn > 25 {
                            return;
                        }
                        mem.write(verts + 0x38, rdata_f32(mem, KFIVE));
                        mem.write(request + 0x18, 0x8080_8080u32);
                        let strength = 0x1_0000i32.wrapping_sub(e).wrapping_mul(0x60) >> 16;
                        let [a, ..] = strength.to_le_bytes();
                        mem.write(request + 0x1b, a >> 3);
                        mem.write(request + 8, 0x48u16);
                        mem.write(request + 0x4c, 0u8);
                        mem.write(request + 0x4d, a.wrapping_add(a));
                        let g = ABMAP.get(mem);
                        let f = mem.rebase(MUL_MATRIX);
                        call(mem, f, &[view_m, g.wrapping_add(0x113e0), node_m]);
                        let g = ABMAP.get(mem);
                        let layer: u32 = mem.read(g.wrapping_add(0x116a4));
                        let f = mem.rebase(MD_DRAW);
                        call(mem, f, &[model, request, 4, layer]);
                    }
                } else {
                    mem.write(request + 0x4d, 0u8);
                }
            }
            node = node.wrapping_add(0x28);
        }
    }

    /// `AbmapDrawNodeSpheres()` (`0x00a505e0`, name ours; from the grid's
    /// draw pass `0x00a54660`): seven parts `e` of every node `i` through
    /// `op1_md_draw_eiabm_sphe([0x023057ec], request, i, e)`, with the
    /// request and vertex block of [`abmap_draw_node_halos`] (`work`,
    /// [`NODE_WORK`]); the model matrix `+0x20` is the view `+0x113e0`
    /// times the part's matrix `m` (`+0x2c`, `FFXVu0MulMatrix`).
    ///
    /// - An empty node (kind `0xffff`): `m = AbmapMakeNodeMatrix(node,
    ///   1.0)`, colour 0, depth (vertex block `+0x38`) -1.
    /// - A kind whose first part offset (`i16` at `+0xf83c` of its 0x30-byte
    ///   record) is 0x1000: `m` at the kind's size (`f32 +0xf838`), colour
    ///   0, depth -1.
    /// - Else the node's panel bits `b` (`+0x21` AND `g_Abmap[+0x115c9]`)
    ///   pick each part's colour: bit `e` set, the word `0x00c8659c[e]` with
    ///   each byte times `c` over 128 (`c = (0x6c - trunc(0.15 sin * -96))
    ///   & 0xff`, `sin` of `(node u16 +0x26 + g_Abmap[+0x11698] << 11) >> 4`
    ///   in `g_pppSinTable`; `c` stays in 94..=122, so no byte carries into the
    ///   next), alpha 0x80, kind 0x2c, depth 1; clear,
    ///   `0x00c86644[e]`, kind 0x24, depth 6. Request `+8` is 0x8064 and
    ///   `m` the node matrix at the kind's size moved by part `e`'s offset
    ///   (`AbmapMakeLinkMatrix(m, node, +0xf83c + 4e, 0.0008)`).
    ///
    /// The request's kind (`+0`) and `+8` are only set on the last path,
    /// so the other two leave what was there.
    #[allow(clippy::too_many_lines)]
    pub fn abmap_draw_node_spheres<M, C>(mem: &mut M, work: u32, call: &mut C)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let request = work;
        let (view_m, node_m, verts) = (work + VIEW_MATRIX, work + NODE_MATRIX, work + VERTS);
        let g = ABMAP.get(mem);
        let count = i32::from(mem.read::<i16>(g.wrapping_add(2)));
        let phase = mem.read::<u32>(g.wrapping_add(0x11698)) << 11;
        init_request(mem, work, g);

        let one = 1f32.to_bits();
        // draws part `e` of node `index` with the part matrix in `node_m`
        let draw = |mem: &mut M, call: &mut C, view: u32, index: i32, e: u32| {
            let f = mem.rebase(MUL_MATRIX);
            call(mem, f, &[view_m, view, node_m]);
            let g = ABMAP.get(mem);
            let nodes = i32::from(mem.read::<i16>(g.wrapping_add(2)));
            let model: u32 = mem.read(mem.rebase(SPHERE_MODEL));
            let f = mem.rebase(MD_DRAW_SPHE);
            let i = nodes.wrapping_sub(index).wrapping_sub(1);
            call(mem, f, &[model, request, i.cast_unsigned(), e]);
        };
        let mut node = g.wrapping_add(0x808);
        let mut left = count;
        // the colour scale of the last lit node (set before each use)
        let mut scale = 0u32;
        while left != 0 {
            left = left.wrapping_sub(1);
            let g = ABMAP.get(mem);
            let kind = mem.read::<u16>(node + 6);
            if kind == 0xffff {
                for e in 0..7 {
                    let g = ABMAP.get(mem);
                    let f = mem.rebase(MAKE_NODE_MATRIX);
                    call(mem, f, &[node_m, node, one]);
                    mem.write(verts + 0x38, rdata_f32(mem, KMINUS1));
                    mem.write(request + 0x18, 0u32);
                    draw(mem, call, g.wrapping_add(0x113e0), left, e);
                }
            } else {
                let rec = g.wrapping_add(0x30 * u32::from(kind));
                let size_at = rec.wrapping_add(0xf838);
                let mut offset = rec.wrapping_add(0xf83c);
                if mem.read::<u16>(offset) == 0x1000 {
                    for e in 0..7 {
                        let g = ABMAP.get(mem);
                        let size = x87::store(x87::load(mem.read(size_at)));
                        let f = mem.rebase(MAKE_NODE_MATRIX);
                        call(mem, f, &[node_m, node, size]);
                        mem.write(verts + 0x38, rdata_f32(mem, KMINUS1));
                        mem.write(request + 0x18, 0u32);
                        draw(mem, call, g.wrapping_add(0x113e0), left, e);
                    }
                } else {
                    let mut bits =
                        mem.read::<u8>(g.wrapping_add(0x115c9)) & mem.read::<u8>(node + 0x21);
                    if bits != 0 {
                        let at = (u32::from(mem.read::<u16>(node + 0x26)).wrapping_add(phase) >> 4)
                            & 0xfff;
                        let sin = x87::load(mem.read(mem.rebase(SIN_TABLE) + 4 * at));
                        let t = x87::mul(
                            x87::mul(rdata_f64(mem, K0_15), sin),
                            rdata_f64(mem, KMINUS96),
                        );
                        scale = 0x6cu32.wrapping_sub(x87::ftol(mem, t)) & 0xff;
                    }
                    mem.write(request + 8, 0x8064u16);
                    for e in 0..7u32 {
                        let g = ABMAP.get(mem);
                        let depth = if bits & 1 == 0 {
                            let color: u32 = mem.read(mem.rebase(DARK_COLORS) + 4 * e);
                            mem.write(request + 0x18, color);
                            mem.write(request, 0x24u32);
                            rdata_f32(mem, KSIX)
                        } else {
                            let base: u32 = mem.read(mem.rebase(LIT_COLORS) + 4 * e);
                            let [b0, b1, b2, b3] =
                                base.to_le_bytes().map(|b| (u32::from(b) * scale) >> 7);
                            let color = ((((b3 << 8).wrapping_add(b2)) << 8).wrapping_add(b1) << 8)
                                .wrapping_add(b0);
                            mem.write(request + 0x18, color);
                            mem.write(request + 0x1b, 0x80u8);
                            mem.write(request, 0x2cu32);
                            one
                        };
                        mem.write(verts + 0x38, depth);
                        let size = x87::store(x87::load(mem.read(size_at)));
                        let f = mem.rebase(MAKE_NODE_MATRIX);
                        call(mem, f, &[node_m, node, size]);
                        let link = rdata_f32(mem, KLINK);
                        let f = mem.rebase(MAKE_LINK_MATRIX);
                        call(mem, f, &[node_m, node, offset, link]);
                        draw(mem, call, g.wrapping_add(0x113e0), left, e);
                        offset = offset.wrapping_add(4);
                        bits >>= 1;
                    }
                }
            }
            node = node.wrapping_add(0x28);
        }
    }

    /// `FFXVu0ApplyMatrix(dst, m, v)`.
    pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
    /// `FFXVu0DivVectorXYZ(dst, v, s)`.
    pub const DIV_VECTOR_XYZ: u32 = 0x006e_d420;
    /// `graphicDrawUIAbmapElement(verts, scratch, kind)`.
    pub const DRAW_UI_ELEMENT: u32 = 0x0063_e930;
    /// The GS packet the strip patches (`[0x01a860f0]`).
    pub const STRIP_PACKET: u32 = 0x01a8_60f0;
    /// The word the emulated GS register writes land in; the strip waits
    /// while its bit 8 is set.
    pub const GS_SINK: u32 = 0x0230_56e8;
    /// [`abmap_draw_link_strip`]'s work: the four vertices (`x, y, u, v`
    /// and four colour words each, 0x20 bytes) it hands to
    /// `graphicDrawUIAbmapElement`.
    pub const STRIP_WORK: u32 = 0x80;

    /// `.rdata` (`f64` unless noted): 0.6 and 0.1 (the strip's divisor
    /// from `g_Abmap[+0x11350]`), 128 (`f32`) and 128, 16
    /// (`g_Vu0FixedScale`), 1/16, 2048, 256, 208, 12, 13, 1/64, 20, 19, 11.
    const K0_6: u32 = 0x00b4_8320;
    const K0_1: u32 = 0x00b9_21f0;
    const K128_F32: u32 = 0x00b5_01bc;
    const K128: u32 = 0x00b4_c170;
    const K16: u32 = 0x00b3_13a0;
    const K1_16: u32 = 0x00b9_21e0;
    const K2048: u32 = 0x00b5_1bc0;
    const K256: u32 = 0x00b9_22f0;
    const K208: u32 = 0x00b5_4c70;
    const K12: u32 = 0x00b4_b468;
    const K13: u32 = 0x00b4_b470;
    const K1_64: u32 = 0x00b5_1648;
    const K20: u32 = 0x00b4_9968;
    const K19: u32 = 0x00b4_b478;
    const K11: u32 = 0x00b5_f1b0;

    /// `x` (a point's coordinate in 1/16) on the screen:
    /// `f32(x) / 16 - 2048 + off`.
    fn screen(mem: &impl Mem, x: i32, off: u32) -> u32 {
        let v = x87::load(x87::store(f64::from(x)));
        let k = |a| rdata_f64(mem, a);
        x87::store(x87::add(x87::sub(x87::mul(v, k(K1_16)), k(K2048)), k(off)))
    }

    /// A texture coordinate: `(tile * step + off) / 64`.
    fn tex(mem: &impl Mem, tile: u8, step: u32, off: f64) -> u32 {
        let k = |a| rdata_f64(mem, a);
        x87::store(x87::mul(
            x87::add(x87::mul(f64::from(tile), k(step)), off),
            k(K1_64),
        ))
    }

    /// The fixed-point screen offsets of the vector at `p + vec` (already
    /// divided by w): `ftol(v * 16)` plus the origin `p + origin` into
    /// `p + outs[0]`, then `v` doubled (stored) twice, each time `ftol(f32(
    /// ftol(v * 16)) / w)` plus the origin into `outs[1]`, `outs[2]`.
    #[allow(clippy::many_single_char_names)] // p, w, x, y, v, q: as documented
    fn offsets(mem: &mut impl Mem, p: u32, vec: u32, origin: u32, outs: [u32; 3], w: f64) {
        let k16 = rdata_f64(mem, K16);
        let fixed =
            |mem: &mut _, at: u32| x87::ftol(mem, x87::mul(x87::load(Mem::read(mem, at)), k16));
        let (x, y) = (p + vec, p + vec + 4);
        let out = p + outs[0];
        let v = fixed(mem, x);
        mem.write(out, v);
        let v = fixed(mem, y);
        mem.write(out + 4, v);
        for (at, o) in [(out, origin), (out + 4, origin + 4)] {
            let v = mem.read::<u32>(at).wrapping_add(mem.read(p + o));
            mem.write(at, v);
        }
        for &k in &outs[1..] {
            let out = p + k;
            for at in [x, y] {
                let doubled = x87::load(mem.read(at));
                mem.write(at, x87::store(x87::add(doubled, doubled)));
            }
            let v = fixed(mem, x);
            mem.write(out, v);
            let v = fixed(mem, y);
            mem.write(out + 4, v);
            for (at, o) in [(out, origin), (out + 4, origin + 4)] {
                let int = f64::from(mem.read::<i32>(at));
                let q = x87::ftol(mem, x87::div(x87::load(x87::store(int)), w));
                mem.write(at, q.wrapping_add(mem.read(p + o)));
            }
        }
    }

    /// `AbmapDrawLinkStrip(p, points, n)` (`0x00a521a0`, name ours; from
    /// `0x00a4b4b0`): a sphere grid link drawn as a strip of textured
    /// quads, `p` its work record (at least `0x1ec` bytes), `points` `n`
    /// triples of 2D points (0x18 bytes each: the centre `a`, the offsets
    /// `b`, `c`).
    ///
    /// For each triple: `p[0..4] = (a, 0, 1)` through the view `g_Abmap +
    /// 0x113e0` into `p + 0x30` (`FFXVu0ApplyMatrix`); it ends when that w
    /// (`p + 0x3c`) is 0 or below (a NaN goes on). Then the point divided by w
    /// (`FFXVu0DivVectorXYZ`), `b` and `c` (`p + 0x10`, `p + 0x20`, with
    /// z 0, w 1) through `g_Abmap + 0x114e0` into `p + 0x40`, `p + 0x50`,
    /// divided by the same w. In 1/16 (`ftol(v * 16)`): the centre at `p + 0x60`
    /// (the first triple) or `p + 0xd0` (the rest); `b` and `c` added to
    /// it once, then doubled and quadrupled and divided by `W =
    /// f32(g_Abmap[+0x11350] * 0.6 + 0.1)`.
    ///
    /// After the first triple, the GS packet at `[0x01a860f0]` gets its
    /// header and fixed words (twice over, after waiting for bit 8 of
    /// `0x023056e8` to clear): words `0x8007`, `p[0x1e0] << 15 |
    /// 0x30004000`, `0x55e`, 0; `p + 0x140..0x158` sign-extended at `+0x10`
    /// and every 0x30 bytes after, each followed by 1, 0; `p[0x1e4]` and
    /// `p[0x1e8]` at the vertices' third words. The colour is `ftol(128)`,
    /// or `ftol(f32(g_Abmap[+0x11620] * 128))` when byte `+0x13` of the
    /// packet is 1.
    ///
    /// Each later triple fills the packet's positions from the previous
    /// (`p + 0x60..`) and the current (`p + 0xd0..`) offsets, then draws
    /// two quads through `graphicDrawUIAbmapElement(work, 0, 9)` and `(work,
    /// 0, 12)`: corners `(x / 16 - 2048 + 256, y / 16 - 2048 + 208)` of
    /// packet words `+0x80, +0x90, +0xb0, +0xc0` and `+0xe0, +0xf0, +0x110,
    /// +0x120`; texture tile `(s, t)` = packet bytes `+0x11`, `+0x10`: `u =
    /// (12 s + 13 | 20) / 64`, `v = (20 t + 12 | 19) / 64` for the first
    /// quad, `(20 t + 0 | 11) / 64` for the second. Then the current
    /// offsets become the previous ones (`p + 0xd0..0x140` to `p + 0x60`).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn abmap_draw_link_strip<M, C>(
        mem: &mut M,
        p: u32,
        points: u32,
        n: u32,
        work: u32,
        call: &mut C,
    ) where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let g = ABMAP.get(mem);
        let w = x87::store(x87::add(
            x87::mul(
                x87::load(mem.read(g.wrapping_add(0x11350))),
                rdata_f64(mem, K0_6),
            ),
            rdata_f64(mem, K0_1),
        ));
        let w = x87::load(w);
        let mut color = rdata_f32(mem, K128_F32);
        // one triple's projection (the first also sets the points' z and
        // w); false when its w is not above 0
        let project = |mem: &mut M, call: &mut C, q: u32, first: bool| -> bool {
            for (i, o) in [(0u32, 0u32), (1, 8), (2, 0x10)] {
                let at = p + 0x10 * i;
                mem.write(at, mem.read::<u32>(q + o));
                mem.write(at + 4, mem.read::<u32>(q + o + 4));
                if first {
                    mem.write(at + 8, 0u32);
                    mem.write(at + 0xc, 1f32.to_bits());
                }
                let (to, m) = if i == 0 {
                    (0x30, 0x113e0)
                } else {
                    (0x30 + 0x10 * i, 0x114e0)
                };
                let m = ABMAP.get(mem).wrapping_add(m);
                let f = mem.rebase(APPLY_MATRIX);
                call(mem, f, &[p + to, m, at]);
                // fcomp sets C0 for 0 < w and for NaN: only 0 and below end
                let w_order = x87::load(mem.read(p + 0x3c)).partial_cmp(&0.0);
                if i == 0 && matches!(w_order, Some(Ordering::Less | Ordering::Equal)) {
                    return false;
                }
                let pw = x87::store(x87::load(mem.read(p + 0x3c)));
                let f = mem.rebase(DIV_VECTOR_XYZ);
                call(mem, f, &[p + to, p + to, pw]);
            }
            true
        };

        if !project(mem, call, points, true) {
            return;
        }
        let k16 = rdata_f64(mem, K16);
        for (at, from) in [(0x60u32, 0x30u32), (0x64, 0x34)] {
            let v = x87::ftol(mem, x87::mul(x87::load(mem.read(p + from)), k16));
            mem.write(p + at, v);
        }
        offsets(mem, p, 0x40, 0x60, [0x70, 0x90, 0xb0], w);
        offsets(mem, p, 0x50, 0x60, [0x80, 0xa0, 0xc0], w);

        // spins as the original does (the tests keep the bit clear)
        while mem.read::<u32>(mem.rebase(GS_SINK)) & 0x100 != 0 {}
        let mut packet = 0;
        for _ in 0..2 {
            packet = mem.read::<u32>(mem.rebase(STRIP_PACKET));
            let tag = mem.read::<u32>(p + 0x1e0) << 15 | 0x3000_4000;
            mem.write(packet, 0x8007u32);
            let again = mem.read::<u32>(mem.rebase(STRIP_PACKET));
            mem.write(packet + 8, 0x55eu32);
            mem.write(again + 4, tag);
            mem.write(packet + 0xc, 0u32);
            for k in 0..7 {
                let v = mem.read::<i32>(p + 0x140 + 4 * k);
                let at = packet + 0x10 + 0x30 * k;
                mem.write(at, v);
                mem.write(at + 4, v >> 31);
                mem.write(at + 8, 1u32);
                mem.write(at + 0xc, 0u32);
            }
            let first = [(0x28u32, 0x8000u32), (0x38, 0), (0x58, 0), (0x68, 0)];
            let middle = [0x88u32, 0x98, 0xb8, 0xc8, 0xe8, 0xf8, 0x118, 0x128];
            for (o, next) in first {
                mem.write(packet + o, mem.read::<u32>(p + 0x1e4));
                mem.write(packet + o + 4, next);
            }
            for o in middle {
                mem.write(packet + o, mem.read::<u32>(p + 0x1e8));
                mem.write(packet + o + 4, 0u32);
            }
            for o in [0x148u32, 0x158] {
                mem.write(packet + o, mem.read::<u32>(p + 0x1e4));
                mem.write(packet + o + 4, 0u32);
            }
        }
        if mem.read::<u8>(packet + 0x13) == 1 {
            let g = ABMAP.get(mem);
            let level = x87::load(mem.read(g.wrapping_add(0x11620)));
            color = x87::store(x87::mul(level, rdata_f64(mem, K128)));
        }

        let mut left = n.wrapping_sub(1);
        let mut q = points;
        loop {
            q = q.wrapping_add(0x18);
            if !project(mem, call, q, false) {
                return;
            }
            for (at, from) in [(0xd0u32, 0x30u32), (0xd4, 0x34)] {
                let v = x87::ftol(mem, x87::mul(x87::load(mem.read(p + from)), k16));
                mem.write(p + at, v);
            }
            offsets(mem, p, 0x40, 0xd0, [0xe0, 0x100, 0x120], w);
            offsets(mem, p, 0x50, 0xd0, [0xf0, 0x110, 0x130], w);

            let packet = mem.read::<u32>(mem.rebase(STRIP_PACKET));
            mem.write(packet + 0x20, mem.read::<u32>(p + 0x70));
            let again = mem.read::<u32>(mem.rebase(STRIP_PACKET));
            mem.write(again + 0x24, mem.read::<u32>(p + 0x74));
            for (to, from) in [
                (0x30u32, 0xe0u32),
                (0x50, 0x80),
                (0x60, 0xf0),
                (0x80, 0x90),
                (0x90, 0x100),
                (0xb0, 0xa0),
                (0xc0, 0x110),
                (0xe0, 0xb0),
                (0xf0, 0x120),
                (0x110, 0xc0),
                (0x120, 0x130),
                (0x140, 0x60),
                (0x150, 0xd0),
            ] {
                mem.write(packet + to, mem.read::<u32>(p + from));
                mem.write(packet + to + 4, mem.read::<u32>(p + from + 4));
            }

            // the two quads: corners from packet words, texture tile
            // (packet bytes +0x11, +0x10)
            let tint = x87::ftol(mem, x87::load(color));
            for v in 0..4 {
                for c in 0..4 {
                    mem.write(work + 0x20 * v + 0x10 + 4 * c, tint);
                }
            }
            for (corners, v_off, kind) in [
                ([0x80u32, 0x90, 0xb0, 0xc0], [K12, K19], 9u32),
                ([0xe0, 0xf0, 0x110, 0x120], [0, K11], 0xc),
            ] {
                for (k, &c) in (0u32..).zip(&corners) {
                    let vert = work + 0x20 * k;
                    let x = screen(mem, mem.read(packet + c), K256);
                    let y = screen(mem, mem.read(packet + c + 4), K208);
                    mem.write(vert, x);
                    mem.write(vert + 4, y);
                }
                for k in 0..4u32 {
                    let vert = work + 0x20 * k;
                    let s = mem.read::<u8>(packet + 0x11);
                    let u_off = rdata_f64(mem, if k % 2 == 0 { K13 } else { K20 });
                    mem.write(vert + 8, tex(mem, s, K12, u_off));
                    let t = mem.read::<u8>(packet + 0x10);
                    let off = v_off[(k / 2) as usize];
                    let v_add = if off == 0 { 0.0 } else { rdata_f64(mem, off) };
                    mem.write(vert + 0xc, tex(mem, t, K20, v_add));
                }
                let f = mem.rebase(DRAW_UI_ELEMENT);
                call(mem, f, &[work, 0, kind]);
            }

            for k in 0..0x1c {
                mem.write(p + 0x60 + 4 * k, mem.read::<u32>(p + 0xd0 + 4 * k));
            }
            left = left.wrapping_sub(1);
            if left == 0 {
                return;
            }
        }
    }

    /// `g_PppPacketPos`: the packet write position the menu text goes to.
    pub const PACKET_POS: Global<u32> = Global::new(0x0230_fd2c);
    /// `TOAddPktCommandIconName_008f0590(pkt, command, x, y, colour)`.
    pub const ADD_COMMAND_ICON_NAME: u32 = 0x008f_0590;
    /// `TOAddPktBaseMesFontLAbmap(pkt, kind, text, x, y, ...)` (12 words).
    pub const ADD_MES_FONT: u32 = 0x008e_8fb0;
    /// `AbmapHelpLineDraw(text, x, y)`.
    pub const HELP_LINE_DRAW: u32 = 0x00a5_94c0;
    /// `MsGetSavePlayerPtr(chr)`.
    pub const SAVE_PLAYER_PTR: u32 = 0x0078_5330;
    /// `ToMakeBtlEasyDigitRightAbmap(n, x, y, d, sx, sy)`.
    pub const DIGIT_RIGHT: u32 = 0x0090_5660;
    /// The stat bytes of kinds 2..=9 in a save player record.
    const STAT_BYTES: [u32; 8] = [0x2f, 0x33, 0x30, 0x34, 0x31, 0x35, 0x32, 0x36];
    /// Non-zero for the wider stat column (`0x01a85f74`).
    pub const STAT_LAYOUT: Global<u32> = Global::new(0x01a8_5f74);
    /// `.rdata` `f32`: 0.8 and 0.5, the digits' scale.
    const DIGIT_SCALE: [u32; 2] = [0x00b3_4254, 0x00b9_2194];

    /// The menu text alpha (`g_Abmap[+0x115b9]`).
    fn text_alpha(mem: &impl Mem) -> u32 {
        u32::from(mem.read::<u8>(ABMAP.get(mem).wrapping_add(0x115b9)))
    }

    /// `TOAddPktBaseMesFontLAbmap` at the packet position (+0x10), the
    /// new position stored.
    fn mes_font<M, C>(
        mem: &mut M,
        call: &mut C,
        kind: u32,
        text: u32,
        x: u32,
        y: u32,
        a: u32,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let alpha = text_alpha(mem);
        let pkt = PACKET_POS.get(mem).wrapping_add(0x10);
        let f = mem.rebase(ADD_MES_FONT);
        let end = call(
            mem,
            f,
            &[pkt, kind, text, x, y, a, 0, 0x80, 0x80, 0x80, alpha, 0],
        );
        PACKET_POS.set(mem, end);
        end
    }

    /// The pad entry draw callbacks (`panel +0x38`, called by
    /// `AbmapPadCtrl`'s list as `f(panel, entry, index, x, y)`; no caller
    /// reads their `eax`): `entry` holds the text (`+0`), a locked byte
    /// (`+4`) and a kind (`+8`).
    ///
    /// `AbmapPadEntryDrawIcon(_, entry, _, x, y)` (`0x00a4f250`, and the
    /// same code at `0x00a4f9a0`; names ours): unless locked, the
    /// command's icon and name (`TOAddPktCommandIconName_008f0590(pos +
    /// 0x10, [entry], x - 0x1c, y - 1, 0)`) at the packet position.
    pub fn abmap_pad_entry_draw_icon<M, C>(
        mem: &mut M,
        entry: u32,
        x: u32,
        y: u32,
        call: &mut C,
    ) -> Option<u32>
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        if mem.read::<u8>(entry.wrapping_add(4)) != 0 {
            return None;
        }
        let pkt = PACKET_POS.get(mem).wrapping_add(0x10);
        let text: u32 = mem.read(entry);
        let f = mem.rebase(ADD_COMMAND_ICON_NAME);
        let end = call(
            mem,
            f,
            &[pkt, text, x.wrapping_sub(0x1c), y.wrapping_sub(1), 0],
        );
        PACKET_POS.set(mem, end);
        Some(end)
    }

    /// `AbmapPadEntryDrawText(_, entry, _, x, y)` (`0x00a4f9e0`, name
    /// ours): the entry's text in font kind 7 at `(x + 10, y)`.
    pub fn abmap_pad_entry_draw_text<M, C>(
        mem: &mut M,
        entry: u32,
        x: u32,
        y: u32,
        call: &mut C,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let text: u32 = mem.read(entry);
        mes_font(mem, call, 7, text, x.wrapping_add(0xa), y, 0)
    }

    /// `AbmapPadEntryDrawHelp(_, entry, _, x, y)` (`0x00a4f930` with
    /// `dx` 14, `0x00a4fb70` with 5; names ours): a locked entry's text in
    /// kind -1 at `(x + dx, y)`, else `AbmapHelpLineDraw(text, x + dx, y)`.
    pub fn abmap_pad_entry_draw_help<M, C>(
        mem: &mut M,
        entry: u32,
        x: u32,
        y: u32,
        dx: u32,
        call: &mut C,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let x = x.wrapping_add(dx);
        let text: u32 = mem.read(entry);
        if mem.read::<u8>(entry.wrapping_add(4)) != 0 {
            mes_font(mem, call, u32::MAX, text, x, y, 1)
        } else {
            let f = mem.rebase(HELP_LINE_DRAW);
            call(mem, f, &[text, x, y])
        }
    }

    /// `AbmapPadEntryDrawStat(_, entry, _, x, y)` (`0x00a4fa30`, name
    /// ours): the entry's name (`AbmapHelpLineDraw(text, x - 15, y)`) and
    /// the current character's (`g_Abmap[+0x115bc]`, `MsGetSavePlayerPtr`)
    /// stat by the entry's kind (`+8`): 0, 1 the words `+0x24`, `+0x28`;
    /// 2..=9 the bytes `+0x2f, +0x33, +0x30, +0x34, +0x31, +0x35, +0x32,
    /// +0x36`; another kind shows `x` itself. Its digits at `(f32(x + 0x30
    /// or 0x50 with 0x01a85f74 clear), f32(y - 1))`, scale 0.8, 0.5.
    pub fn abmap_pad_entry_draw_stat<M, C>(
        mem: &mut M,
        entry: u32,
        x: u32,
        y: u32,
        call: &mut C,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let chr = u32::from(mem.read::<u8>(ABMAP.get(mem).wrapping_add(0x115bc)));
        let f = mem.rebase(SAVE_PLAYER_PTR);
        let player = call(mem, f, &[chr]);
        let text: u32 = mem.read(entry);
        let f = mem.rebase(HELP_LINE_DRAW);
        call(mem, f, &[text, x.wrapping_sub(0xf), y]);
        let kind: u32 = mem.read(entry.wrapping_add(8));
        let value = match kind {
            0 => mem.read(player.wrapping_add(0x24)),
            1 => mem.read(player.wrapping_add(0x28)),
            2..=9 => u32::from(mem.read::<u8>(player.wrapping_add(STAT_BYTES[kind as usize - 2]))),
            _ => x,
        };
        let [sx, sy] = DIGIT_SCALE.map(|a| x87::store(x87::load(mem.read(mem.rebase(a)))));
        let fy = x87::store(f64::from(y.wrapping_sub(1).cast_signed()));
        let dx = if STAT_LAYOUT.get(mem) != 0 {
            0x30
        } else {
            0x50
        };
        let fx = x87::store(f64::from(x.wrapping_add(dx).cast_signed()));
        let f = mem.rebase(DIGIT_RIGHT);
        call(mem, f, &[value, fx, fy, 0, sx, sy])
    }

    /// `MsGetSaveConfigEnglish()` (`0x007851d0`).
    pub const GET_SAVE_CONFIG_ENGLISH: u32 = 0x0078_51d0;
    /// `g_CdLib + 0x10` and `+ 8`: the CD library's table read and read
    /// wait, function pointers.
    pub const CDLIB_READ_TABLE: Global<u32> = Global::new(0x0231_0c50);
    pub const CDLIB_READ_SYNC: Global<u32> = Global::new(0x0231_0c48);
    /// `user_malloc(size)`, `user_free_thunk(p)`, `getExcelBinSize(name,
    /// buffer)`, `FlushCache(0)`.
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const GET_EXCEL_BIN_SIZE: u32 = 0x0078_2df0;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    /// The two kernel files and where each goes (the pointers at
    /// `g_AbmapNodeKinds + 4` and `g_AbmapNodeKinds`): `"sphere"`,
    /// `"panel"`.
    pub const LANG_FILES: [(u32, u32); 2] =
        [(0x00b7_b5c4, 0x01a8_60e4), (0x00b7_b5cc, 0x01a8_60e0)];
    /// The read buffer's size.
    pub const LANG_BUFFER: u32 = 0x2_5800;

    /// `eiAbmSetLang()` (`0x00a54ab0`, from `eiAbmLoad` and
    /// `MsBattleReadDataNB`): the CD library's table 7 (English save
    /// setting) or 0xd, then `sphere.bin` and `panel.bin` of the current
    /// language each read into a new block (`getExcelBinSize`) and copied
    /// (the C runtime's `memcpy`, here through `Mem`) over the block its
    /// pointer names, the block freed; then the CD read wait and
    /// `FlushCache(0)`, whose `eax` it returns.
    pub fn ei_abm_set_lang(h: &mut impl ffx_bridge::Host) -> u32 {
        let f = h.rebase(GET_SAVE_CONFIG_ENGLISH);
        let table = if h.call(f, &[]) == 0 { 0xd } else { 7 };
        let f = CDLIB_READ_TABLE.get(h);
        h.call(f, &[table]);
        for (name, dst) in LANG_FILES {
            let f = h.rebase(USER_MALLOC);
            let buf = h.call(f, &[LANG_BUFFER]);
            let (f, name) = (h.rebase(GET_EXCEL_BIN_SIZE), h.rebase(name));
            let len = h.call(f, &[name, buf]);
            let dst: u32 = Global::new(dst).get(h);
            let mut bytes = vec![0u8; len as usize];
            h.read_bytes(buf, &mut bytes);
            h.write_bytes(dst, &bytes);
            let f = h.rebase(USER_FREE);
            h.call(f, &[buf]);
        }
        let f = CDLIB_READ_SYNC.get(h);
        h.call(f, &[0]);
        let f = h.rebase(FLUSH_CACHE);
        h.call(f, &[0])
    }

    /// `MsInitChrAbilityMap()`, `MsGetAbilityMap(chr)`,
    /// `MsGetSaveAbilityMap()`, `MsGetExcelData(id, table, &strings)`,
    /// `MsAbilityMapSetCommand(chr, command)`, `MsSetSaveParamAll()`.
    pub const INIT_CHR_ABILITY_MAP: u32 = 0x0079_8820;
    pub const GET_ABILITY_MAP: u32 = 0x0079_87f0;
    pub const GET_SAVE_ABILITY_MAP: u32 = 0x0078_4f40;
    pub const GET_EXCEL_DATA: u32 = 0x007a_b870;
    pub const ABILITY_MAP_SET_COMMAND: u32 = 0x0079_8840;
    pub const SET_SAVE_PARAM_ALL: u32 = 0x0078_6900;
    /// `g_AbmapNodeKinds`: the node kinds' excel table.
    pub const NODE_KINDS: Global<u32> = Global::new(0x01a8_60e0);
    /// The save grid's node count (two bytes each: kind, a bit per
    /// character that activated it).
    pub const SAVE_NODES: u32 = 0x400;

    /// `eiAbmParaGet()` (`0x00a54860`, from the menus' load paths,
    /// `AbmapState_ChangingNode`, `AbmapGetNodePanel`, `eiAbmDataInit`):
    /// after `MsInitChrAbilityMap`, for each of the seven characters the
    /// stat bonuses of the grid nodes it activated. Each activated node's
    /// kind record (`MsGetExcelData`) adds its byte `+0x14` to the stats
    /// its `u16 +0x10` bits name and hands its command (`u16 +0x12`) to
    /// `MsAbilityMapSetCommand`. Into the character's ability map record:
    /// bit 0x100 (u32 `+0`) and 0x200 (u32 `+4`) unclamped, bits 0..7 as
    /// bytes `+8..+0xf` clamped at 0xff (`+8..+0xb` from bits 1, 2, 4, 8
    /// and so on). Then `MsSetSaveParamAll`, whose `eax` it returns
    /// (`eiAbmDataInit` passes it on). `MsGetExcelData`'s third argument
    /// is a frame word it never reads: `strings` here (one word).
    pub fn ei_abm_para_get(h: &mut impl ffx_bridge::Host, strings: u32) -> u32 {
        let f = h.rebase(INIT_CHR_ABILITY_MAP);
        h.call(f, &[]);
        for chr in 0..7u32 {
            // bits 0..7, then 0x100 and 0x200
            let mut sums = [0u32; 10];
            let f = h.rebase(GET_ABILITY_MAP);
            let out = h.call(f, &[chr]);
            let f = h.rebase(GET_SAVE_ABILITY_MAP);
            let save = h.call(f, &[]);
            let bit = 1u8 << chr;
            for node in 0..SAVE_NODES {
                let at = save.wrapping_add(2 * node);
                if h.read::<u8>(at.wrapping_add(1)) & bit == 0 {
                    continue;
                }
                let kind = u32::from(h.read::<u8>(at));
                let (f, table) = (h.rebase(GET_EXCEL_DATA), NODE_KINDS.get(h));
                let record = h.call(f, &[kind, table, strings]);
                let stats: u16 = h.read(record.wrapping_add(0x10));
                let amount = u32::from(h.read::<u8>(record.wrapping_add(0x14)));
                for (k, sum) in sums.iter_mut().enumerate() {
                    if stats & (1 << k) != 0 {
                        *sum = sum.wrapping_add(amount);
                    }
                }
                let command = u32::from(h.read::<u16>(record.wrapping_add(0x12)));
                let f = h.rebase(ABILITY_MAP_SET_COMMAND);
                h.call(f, &[chr, command]);
            }
            h.write(out, sums[8]);
            h.write(out.wrapping_add(4), sums[9]);
            for (k, sum) in (0u32..).zip(&sums[..8]) {
                // the sums stay below 0x400 * 0xff: the signed clamp
                // reads as an unsigned one
                let byte = u8::try_from(*sum).unwrap_or(0xff);
                h.write(out.wrapping_add(8 + k), byte);
            }
        }
        let f = h.rebase(SET_SAVE_PARAM_ALL);
        h.call(f, &[])
    }

    /// `0x00a45ab0(side, x, y)` (the panel frame, `f32` x and y),
    /// `AbmapDrawChrFace(x, y)`, `TOGetSaveChrName(chr)`,
    /// `TOGetMessageSize(text, &w, &h)`, `AbmapHelpLineDraw(text, x, y)`,
    /// `MsGetSaveSP(chr)`.
    pub const PANEL_FRAME: u32 = 0x00a4_5ab0;
    pub const DRAW_CHR_FACE: u32 = 0x00a4_61b0;
    pub const SAVE_CHR_NAME: u32 = 0x008a_c850;
    pub const MESSAGE_SIZE: u32 = 0x008b_7130;
    pub const GET_SAVE_SP: u32 = 0x0078_53e0;
    /// `.rdata`: 4 and 172 (`f32`).
    const KFOUR: u32 = 0x00b9_22f8;
    const K172: u32 = 0x00b7_b6d8;
    /// The work block of [`abmap_draw_chr_panel`]: the message size's
    /// width and height words.
    pub const CHR_PANEL_WORK: u32 = 8;

    /// `0x00a4fbe0()` (`AbmapDrawChrPanel`, name ours; from `eiAbmDraw`
    /// and the grid draw pass `0x00a54660`): the current character's
    /// panel (`g_Abmap + 0x115bc`), left (`x` 0xa0) or, with `+0x115c0`,
    /// right (`x` 0). While `+0x115c2` slides it, frame `+0x115c1` (or 3
    /// minus it, to the right) puts `x` at `frame * 160 / 4 + 0x20` and
    /// steps (a fourth step ends the slide), both panels drawn where they
    /// show (the left one for `x > 0`, the right one for `x < 0xa0`). A
    /// panel: its frame (`0x00a45ab0(side, x - 0x70 or x + 0x17c, 4 or
    /// 0)`), the face, the name centred (`TOGetMessageSize` into `work`),
    /// the SP left (`MsGetSaveSP` - `+0x1161c`) as digits at 172.
    /// Returns the last digits call's `eax`.
    pub fn abmap_draw_chr_panel(h: &mut impl ffx_bridge::Host, work: u32) -> u32 {
        /// `(side, frame x, frame y, face x, name x, digits x)` offsets.
        fn panel(h: &mut impl ffx_bridge::Host, work: u32, x: i32, right: bool) -> u32 {
            let f32_of = |v: i32| x87::store(f64::from(v));
            let (side, frame_x, frame_y, face, name_x, digits) = if right {
                (1, x + 0x17c, 0, x + 0x18d, x + 0x169 + 0x3d, x + 0x1bd)
            } else {
                (
                    0,
                    x - 0x70,
                    rdata_f32(h, KFOUR),
                    x - 0x5f,
                    x - 2 - 0x44,
                    x - 0x2f,
                )
            };
            let f = h.rebase(PANEL_FRAME);
            h.call(f, &[side, f32_of(frame_x), frame_y]);
            let f = h.rebase(DRAW_CHR_FACE);
            h.call(f, &[face.cast_unsigned(), 0x50]);
            let chr = u32::from(h.read::<u8>(ABMAP.get(h).wrapping_add(0x115bc)));
            let f = h.rebase(SAVE_CHR_NAME);
            let name = h.call(f, &[chr]);
            let f = h.rebase(MESSAGE_SIZE);
            h.call(f, &[name, work, work + 4]);
            let width: i32 = h.read(work);
            let at = name_x.wrapping_sub(width / 2);
            let f = h.rebase(HELP_LINE_DRAW);
            h.call(f, &[name, at.cast_unsigned(), 0x9e]);
            let chr = u32::from(h.read::<u8>(ABMAP.get(h).wrapping_add(0x115bc)));
            let f = h.rebase(GET_SAVE_SP);
            let sp = h.call(f, &[chr]);
            let used: u32 = h.read(ABMAP.get(h).wrapping_add(0x1161c));
            let (k172, [sx, sy]) = (rdata_f32(h, K172), DIGIT_SCALE.map(|a| rdata_f32(h, a)));
            let f = h.rebase(DIGIT_RIGHT);
            h.call(f, &[sp.wrapping_sub(used), f32_of(digits), k172, 0, sx, sy])
        }
        let g = ABMAP.get(h);
        let sliding = h.read::<u8>(g.wrapping_add(0x115c2)) != 0;
        let to_right = h.read::<u8>(g.wrapping_add(0x115c0)) != 0;
        let mut eax = 0;
        let x = if sliding {
            let frame: u8 = h.read(g.wrapping_add(0x115c1));
            let step = if to_right {
                3 - i32::from(frame)
            } else {
                i32::from(frame)
            };
            let x = step.wrapping_mul(160) / 4 + 0x20;
            let next = frame.wrapping_add(1);
            h.write(g.wrapping_add(0x115c1), next);
            if next >= 4 {
                let g = ABMAP.get(h);
                h.write(g.wrapping_add(0x115c2), 0u8);
                let g = ABMAP.get(h);
                h.write(g.wrapping_add(0x115c1), 0u8);
            }
            if x > 0 {
                eax = panel(h, work, x, false);
            }
            x
        } else if to_right {
            0
        } else {
            return panel(h, work, 0xa0, false);
        };
        if x < 0xa0 {
            eax = panel(h, work, x, true);
        }
        eax
    }

    /// `MsGetSaveItemNum(item)`, `TOAdpMenuIconItemAbmap(pkt, item, x, y,
    /// dim)`, `AbmapDigitRight(n, x, y, dim)`.
    pub const SAVE_ITEM_NUM: u32 = 0x0079_04b0;
    pub const ICON_ITEM: u32 = 0x008f_0700;
    pub const ABMAP_DIGIT_RIGHT: u32 = 0x00a5_9630;
    /// The row's texture coordinates (`.rdata` `f32`): u left and right,
    /// v top for even and odd rows, v bottom for even and odd rows.
    const ITEM_U: [u32; 2] = [0x00b7_b5f0, 0x00b7_b614];
    const ITEM_V_TOP: [u32; 2] = [0x00b7_b5dc, 0x00b6_5610];
    const ITEM_V_BOTTOM: [u32; 2] = [0x00b7_b5e0, 0x00b7_b5f8];
    /// The work of [`abmap_pad_entry_draw_item`]: the four vertices
    /// (`x, y, u, v` and four colour words each) and, from `+0x80`, the
    /// 256-byte name buffer `graphicDrawUIAbmapElement` gets.
    pub const ITEM_WORK: u32 = 0x180;

    /// `0x00a4f630(panel, entry, index, x, y)` (`AbmapPadEntryDrawItem`,
    /// name ours; a pad entry draw callback stored at panel `+0x38` by
    /// `0x00a55036`): the item row of `entry` (item `u16 +8`, dimmed by
    /// byte `+4`). Rows `y - 2 .. y + 0x12` inside the panel's clip
    /// (`i16 +0xa` - 1 .. `+0xa` + `+0xe` - 3; a row above the top is cut
    /// at the top only) get a background quad from `x - +0x20` to `x +
    /// +0xc - +0x20` (texture rows alternating with `index`'s parity, a
    /// signed remainder) through `graphicDrawUIAbmapElement(quad, name,
    /// 6)`, the item icon at `(x, y - 1)` (`TOAdpMenuIconItemAbmap` from
    /// `g_PppPacketPos + 0x10`, the new position stored) and the count
    /// (`MsGetSaveItemNum`, read first) right-aligned at `x + +0xc -
    /// 0x24`. Returns the original's `eax`: the count's draw, else the
    /// row bottom or the clip bottom it failed on. The original's name
    /// buffer is uninitialised stack; `work`'s is the caller's.
    pub fn abmap_pad_entry_draw_item(
        h: &mut impl ffx_bridge::Host,
        work: u32,
        [panel, entry, index, x, y]: [u32; 5],
    ) -> u32 {
        let item = u32::from(h.read::<u16>(entry.wrapping_add(8)));
        let f = h.rebase(SAVE_ITEM_NUM);
        let count = h.call(f, &[item]);
        let half = |h: &mut _, o: u32| i32::from(Mem::read::<i16>(h, panel.wrapping_add(o)));
        let (x, y) = (x.cast_signed(), y.cast_signed());
        let origin = half(h, 0x20);
        let left = x87::store(f64::from(x.wrapping_sub(origin)));
        let right = half(h, 0xc).wrapping_sub(origin).wrapping_add(x);
        let right = x87::store(f64::from(right));
        let (mut top, mut bottom) = (y.wrapping_sub(2), y.wrapping_add(0x12));
        let clip_top = half(h, 0xa).wrapping_sub(1);
        if bottom <= clip_top {
            return bottom.cast_unsigned();
        }
        let clip_bottom = half(h, 0xa).wrapping_sub(3).wrapping_add(half(h, 0xe));
        if top >= clip_bottom {
            return clip_bottom.cast_unsigned();
        }
        if top < clip_top {
            top = clip_top;
        } else if bottom > clip_bottom {
            bottom = clip_bottom;
        }
        let (top, bottom) = (x87::store(f64::from(top)), x87::store(f64::from(bottom)));
        let odd = usize::from(index.cast_signed() % 2 != 0);
        let [u0, u1] = ITEM_U.map(|a| rdata_f32(h, a));
        let (v0, v1) = (
            rdata_f32(h, ITEM_V_TOP[odd]),
            rdata_f32(h, ITEM_V_BOTTOM[odd]),
        );
        for (k, vertex) in (0u32..).zip([
            [left, top, u0, v0],
            [left, bottom, u0, v1],
            [right, top, u1, v0],
            [right, bottom, u1, v1],
        ]) {
            let at = work.wrapping_add(0x20 * k);
            for (o, word) in (0u32..).zip(vertex) {
                h.write(at.wrapping_add(4 * o), word);
            }
            for o in 4..8 {
                h.write(at.wrapping_add(4 * o), 0x80u32);
            }
        }
        let f = h.rebase(DRAW_UI_ELEMENT);
        h.call(f, &[work, work.wrapping_add(0x80), 6]);
        let y = y.wrapping_sub(1).cast_unsigned();
        let dim = u32::from(h.read::<u8>(entry.wrapping_add(4)) != 0);
        let pkt = PACKET_POS.get(h).wrapping_add(0x10);
        let f = h.rebase(ICON_ITEM);
        let end = h.call(f, &[pkt, item, x.cast_unsigned(), y, dim]);
        PACKET_POS.set(h, end);
        let at = half(h, 0xc).wrapping_sub(0x24).wrapping_add(x);
        let f = h.rebase(ABMAP_DIGIT_RIGHT);
        h.call(f, &[count, at.cast_unsigned(), y, dim])
    }

    /// `g_AbmapPad`: the pad state (`i16 +0x2700` the pressed command,
    /// negative while a state runs; twelve slots of 0x340 bytes with a
    /// callback at `+0x34`).
    pub const ABMAP_PAD: Global<u32> = Global::new(0x01a8_6108);
    /// A word `eiAbmCalc` sets to 1 (`0x01a85f70`).
    pub const CALC_RAN: Global<u32> = Global::new(0x01a8_5f70);
    /// The matrix scale scratch (`0x00c8f820`, 16 words: the scales, then
    /// the rotation's three rows).
    pub const SCALE_ROWS: u32 = 0x00c8_f820;
    /// `AbmapReadPad(no_stick, pad)`, `dbgPrintf(text)`,
    /// `AbmapCtrlZoom()`, `AbmapCtrlTilt()`, `AbmapTryChangePlayer()`,
    /// `AbmapState_HomeOnNode` (a state), `AbmapPadCtrl()`,
    /// `AbmapPulseCursor()`, `FFXVu0CopyVector(dst, src)`,
    /// `FFXVu0ScaleVector(dst, v, s)`, `FFXSamp0UnitMatrix(m)`,
    /// `pppGetRotMatrixXYZ(out, angles)`, `FFXVu0CopyMatrix(dst, src)`,
    /// `graphicSetAbmap(sin, cos, x, y, scale)`, `AbmapSpreadPlyTags()`.
    pub const READ_PAD: u32 = 0x00a5_7520;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const CTRL_ZOOM: u32 = 0x00a4_90c0;
    pub const CTRL_TILT: u32 = 0x00a4_8f50;
    pub const TRY_CHANGE_PLAYER: u32 = 0x00a5_8ec0;
    pub const STATE_HOME_ON_NODE: u32 = 0x00a5_9e80;
    pub const PAD_CTRL: u32 = 0x00a5_83e0;
    pub const PULSE_CURSOR: u32 = 0x00a4_7440;
    pub const COPY_VECTOR: u32 = 0x006e_d370;
    pub const SCALE_VECTOR: u32 = 0x006e_d8f0;
    pub const UNIT_MATRIX: u32 = 0x0093_d900;
    pub const ROT_MATRIX_XYZ: u32 = 0x0072_c800;
    pub const COPY_MATRIX: u32 = 0x006e_d360;
    pub const SET_ABMAP: u32 = 0x0064_2da0;
    pub const SPREAD_PLY_TAGS: u32 = 0x00a5_a4b0;
    /// `"Got ABMAP pad input!!!\n"`.
    const STR_PAD_INPUT: u32 = 0x00b7_b58c;
    /// `.rdata`: 15, 10, 512 (`f64`); 341, 208, 96 (`f32`); 128 is
    /// [`K128`].
    const K15: u32 = 0x00b9_22c0;
    const K10: u32 = 0x00b9_22b0;
    const K512: u32 = 0x00b4_1d38;
    const K341: u32 = 0x00b7_b6e8;
    const K208_F32: u32 = 0x00b6_56d8;
    const K96: u32 = 0x00b4_3db8;
    /// The work of [`ei_abm_calc`]: the screen vector it scales (16
    /// bytes).
    pub const CALC_WORK: u32 = 0x10;

    /// `eiAbmCalc()` (`0x00a53570`, from `TkMenuCtrlAbilityMap2`): one
    /// frame of the sphere grid. For its first 21 frames (`g_Abmap +
    /// 0x115b8`, counting) the fade-in: `+0x115e8`, `+0x115ec`, `+0x115f0`
    /// the frame over 15 (0 and 1 at the ends), `+0x115f4` the frame past
    /// 10 over 10 and `+0x115b9` that times 128 (`ftol`; 0 and 0x80 at the
    /// ends); the first frame also sets `g_AbmapPad + 0x13a3`. After that
    /// the pad: `AbmapReadPad(0, +0x1166c)` (a `dbgPrintf` on input),
    /// zoom and tilt while a state runs or their counters (`i16 +0x115d0`,
    /// `+0x115ce`) are positive, then the state (`+0x115a8`) while one
    /// runs, else `AbmapTryChangePlayer` for command 7 (the state then
    /// only if it is `AbmapState_HomeOnNode`) or `AbmapPadCtrl`, and the
    /// twelve pad slots' callbacks. Then the cursor pulse, the three frame
    /// counters (the last capped at 100), the camera target (`+0x11318`
    /// from `+0x11308`) checked against the edges (`+0x115fc..+0x11608`)
    /// with the screen (341, 208) over the scale (`+0x11350`): leaving
    /// the left 128 flips the panel side `+0x115c0` (a slide `+0x115c2`
    /// unless `+0x115ba`), and the target clamped inside the edges less
    /// (341, 208) - 96 scaled (0 where they cross). Then the matrices:
    /// unit `+0x11360`, the rotation `+0x11420` of the angles `+0x11340`
    /// scaled by rows (through `0x00c8f820`), the products `+0x114e0` and
    /// `+0x113e0`, the camera `+0x11390` and target `+0x11450` points
    /// (x, y negated, z from 512), and `graphicSetAbmap` with the angle's
    /// sine and cosine. The name tags spread when `+0x115c7` asks.
    /// Returns `g_Abmap` (the original's `eax`).
    #[allow(clippy::too_many_lines)] // one frame, in the original's order
    pub fn ei_abm_calc(h: &mut impl ffx_bridge::Host, work: u32) -> u32 {
        let call = |h: &mut _, f: u32, args: &[u32]| {
            let f = Mem::rebase(h, f);
            ffx_bridge::Host::call(h, f, args)
        };
        let f32_at = |h: &mut _, o: u32| -> f64 {
            let at = ABMAP.get(h).wrapping_add(o);
            x87::load(Mem::read(h, at))
        };
        let set_f32 = |h: &mut _, o: u32, v: f64| {
            let at = ABMAP.get(h).wrapping_add(o);
            Mem::write(h, at, x87::store(v));
        };
        CALC_RAN.set(h, 1);
        let mut first = ABMAP.get(h);
        if h.read::<u8>(first.wrapping_add(0x115b8)) == 0 {
            let pad = ABMAP_PAD.get(h);
            h.write(pad.wrapping_add(0x13a3), 1u8);
            first = ABMAP.get(h);
        }
        let frame: u8 = h.read(first.wrapping_add(0x115b8));
        if frame <= 0x14 {
            let fade = match frame {
                0 => 0.0,
                1..=14 => {
                    let t = x87::load(x87::store(f64::from(frame)));
                    x87::load(x87::store(x87::div(t, rdata_f64(h, K15))))
                }
                _ => 1.0,
            };
            h.write(first.wrapping_add(0x115e8), x87::store(fade));
            let fade = x87::load(h.read(first.wrapping_add(0x115e8)));
            let fade = x87::load(x87::store(fade));
            set_f32(h, 0x115f0, fade);
            set_f32(h, 0x115ec, fade);
            let late = i32::from(h.read::<u8>(first.wrapping_add(0x115b8))) - 10;
            let (shade, alpha) = if late <= 0 {
                (0.0, 0u8)
            } else if late < 10 {
                let t = x87::load(x87::store(f64::from(late)));
                let shade = x87::load(x87::store(x87::div(t, rdata_f64(h, K10))));
                let k128 = rdata_f64(h, K128);
                (shade, crate::lo8(x87::ftol(h, x87::mul(shade, k128))))
            } else {
                (1.0, 0x80)
            };
            set_f32(h, 0x115f4, shade);
            let at = ABMAP.get(h).wrapping_add(0x115b9);
            h.write(at, alpha);
            let at = ABMAP.get(h).wrapping_add(0x115b8);
            let n: u8 = h.read(at);
            h.write(at, n.wrapping_add(1));
        } else {
            call(h, READ_PAD, &[0, first.wrapping_add(0x1166c)]);
            if h.read::<u16>(ABMAP.get(h).wrapping_add(0x1166e)) != 0 {
                let text = h.rebase(STR_PAD_INPUT);
                call(h, DBG_PRINTF, &[text]);
            }
            let pad = ABMAP_PAD.get(h);
            let command = |h: &mut _| Mem::read::<i16>(h, pad.wrapping_add(0x2700));
            if command(h) < 0 || h.read::<i16>(ABMAP.get(h).wrapping_add(0x115d0)) > 0 {
                call(h, CTRL_ZOOM, &[]);
            }
            if command(h) < 0 || h.read::<i16>(ABMAP.get(h).wrapping_add(0x115ce)) > 0 {
                call(h, CTRL_TILT, &[]);
            }
            let command = command(h);
            let state = |h: &mut _| -> u32 { Mem::read(h, ABMAP.get(h).wrapping_add(0x115a8)) };
            if command < 0 {
                let f = state(h);
                if f != 0 {
                    h.call(f, &[]);
                }
            } else if command == 7 && {
                call(h, TRY_CHANGE_PLAYER, &[]);
                state(h) == h.rebase(STATE_HOME_ON_NODE)
            } {
                let f = state(h);
                h.call(f, &[]);
            } else {
                call(h, PAD_CTRL, &[]);
            }
            let mut slot = ABMAP_PAD.get(h);
            for _ in 0..12 {
                let f: u32 = h.read(slot.wrapping_add(0x34));
                if f != 0 {
                    h.call(f, &[slot]);
                }
                slot = slot.wrapping_add(0x340);
            }
        }
        call(h, PULSE_CURSOR, &[]);
        for o in [0x11698, 0x1169c, 0x116a0] {
            let at = ABMAP.get(h).wrapping_add(o);
            let n: u32 = h.read(at);
            h.write(at, n.wrapping_add(1));
        }
        let at = ABMAP.get(h).wrapping_add(0x116a0);
        if h.read::<i32>(at) > 100 {
            h.write(at, 100u32);
        }
        let at = ABMAP.get(h);
        call(
            h,
            COPY_VECTOR,
            &[at.wrapping_add(0x11318), at.wrapping_add(0x11308)],
        );
        let screen = |h: &mut _, z: u32| {
            let words = [
                rdata_f32(h, K341),
                rdata_f32(h, K208_F32),
                z,
                x87::store(1.0),
            ];
            for (k, w) in (0u32..).zip(words) {
                Mem::write(h, work.wrapping_add(4 * k), w);
            }
            let s = x87::store(x87::div(1.0, f32_at(h, 0x11350)));
            call(h, SCALE_VECTOR, &[work, work, s]);
        };
        // the panel side from the target against the left and right edges
        screen(h, 0);
        let x = f32_at(h, 0x11318);
        let half = x87::load(h.read(work));
        let left = x87::add(x87::add(f32_at(h, 0x115fc), rdata_f64(h, K128)), half);
        let side = ABMAP.get(h).wrapping_add(0x115c0);
        let flip = if x < left {
            (h.read::<u8>(side) != 1).then_some(1u8)
        } else {
            let right = x87::sub(x87::sub(f32_at(h, 0x11604), rdata_f64(h, K128)), half);
            (f32_at(h, 0x11318) > right && h.read::<u8>(side) != 0).then_some(0)
        };
        if let Some(to) = flip {
            h.write(side, to);
            let g0 = ABMAP.get(h);
            if h.read::<u8>(g0.wrapping_add(0x115ba)) == 0 {
                h.write(g0.wrapping_add(0x115c2), 1u8);
            }
        }
        // the target kept inside the edges less the scaled screen
        screen(h, rdata_f32(h, K96));
        let z = x87::load(h.read(work.wrapping_add(8)));
        let wx = x87::load(x87::store(x87::sub(x87::load(h.read(work)), z)));
        let wy = x87::load(x87::store(x87::sub(
            x87::load(h.read(work.wrapping_add(4))),
            z,
        )));
        h.write(work, x87::store(wx));
        h.write(work.wrapping_add(4), x87::store(wy));
        for (o, w, lo, hi) in [
            (0x11318, wx, 0x115fc, 0x11604),
            (0x1131c, wy, 0x11600, 0x11608),
        ] {
            let lo = x87::add(f32_at(h, lo), w);
            let hi = x87::sub(f32_at(h, hi), w);
            if hi < lo {
                set_f32(h, o, 0.0);
            } else if f32_at(h, o) < lo {
                set_f32(h, o, lo);
            } else if f32_at(h, o) > hi {
                set_f32(h, o, hi);
            }
        }
        // the matrices
        let at = ABMAP.get(h);
        call(h, UNIT_MATRIX, &[at.wrapping_add(0x11360)]);
        let at = ABMAP.get(h);
        call(
            h,
            ROT_MATRIX_XYZ,
            &[at.wrapping_add(0x11420), at.wrapping_add(0x11340)],
        );
        let at = ABMAP.get(h);
        call(
            h,
            COPY_MATRIX,
            &[at.wrapping_add(0x11460), at.wrapping_add(0x11360)],
        );
        let at = ABMAP.get(h);
        call(
            h,
            MUL_MATRIX,
            &[
                at.wrapping_add(0x114e0),
                at.wrapping_add(0x114a0),
                at.wrapping_add(0x11460),
            ],
        );
        let at = ABMAP.get(h);
        let rows = h.rebase(SCALE_ROWS);
        for k in 0..16u32 {
            let from = if k < 4 {
                0x11350 + 4 * k
            } else {
                0x11420 + 4 * (k - 4)
            };
            let w: u32 = h.read(at.wrapping_add(from));
            h.write(rows.wrapping_add(4 * k), w);
        }
        for row in 0..3u32 {
            let scale = x87::load(h.read(rows.wrapping_add(4 * row)));
            for col in 0..3u32 {
                let at = rows.wrapping_add(0x10 + 0x10 * row + 4 * col);
                let v = x87::mul(scale, x87::load(h.read(at)));
                h.write(at, x87::store(v));
            }
        }
        for k in 0..12u32 {
            let w: u32 = h.read(rows.wrapping_add(0x10 + 4 * k));
            // `g_Abmap` read again before the second and third rows
            let at = if k % 4 == 0 { ABMAP.get(h) } else { at };
            h.write(at.wrapping_add(0x11420 + 4 * k), w);
        }
        set_f32(h, 0x11324, 1.0);
        let at = ABMAP.get(h);
        call(
            h,
            APPLY_MATRIX,
            &[
                at.wrapping_add(0x11390),
                at.wrapping_add(0x11360),
                at.wrapping_add(0x11328),
            ],
        );
        let at = ABMAP.get(h);
        call(
            h,
            APPLY_MATRIX,
            &[
                at.wrapping_add(0x11450),
                at.wrapping_add(0x11420),
                at.wrapping_add(0x11318),
            ],
        );
        let k512 = rdata_f64(h, K512);
        for point in [0x11390, 0x11450] {
            let x = f32_at(h, point);
            set_f32(h, point, -x);
            let y = f32_at(h, point + 4);
            set_f32(h, point + 4, -y);
            let z = f32_at(h, point + 8);
            set_f32(h, point + 8, x87::sub(k512, z));
        }
        let at = ABMAP.get(h);
        call(
            h,
            MUL_MATRIX,
            &[
                at.wrapping_add(0x113e0),
                at.wrapping_add(0x113a0),
                at.wrapping_add(0x11360),
            ],
        );
        let at = ABMAP.get(h);
        let angle: i32 = h.read(at.wrapping_add(0x11340));
        let table = |h: &mut _, a: i32| {
            let k = (a >> 4).cast_unsigned() & 0xfff;
            let at = Mem::rebase(h, SIN_TABLE).wrapping_add(4 * k);
            x87::store(x87::load(Mem::read(h, at)))
        };
        let through = |h: &mut _, o: u32| x87::store(x87::load(Mem::read(h, at.wrapping_add(o))));
        let args = [
            table(h, angle),
            table(h, angle.wrapping_add(0x4000)),
            through(h, 0x11318),
            through(h, 0x1131c),
            through(h, 0x11350),
        ];
        call(h, SET_ABMAP, &args);
        let mut at = ABMAP.get(h);
        if h.read::<u8>(at.wrapping_add(0x115c7)) != 0 {
            call(h, SPREAD_PLY_TAGS, &[]);
            let g1 = ABMAP.get(h);
            h.write(g1.wrapping_add(0x115c7), 0u8);
            at = ABMAP.get(h);
        }
        h.write(at.wrapping_add(0x115ba), 0u8);
        at
    }

    /// `eiAbmLoad()` (`0x00a54810`, from `MsBattleReadDataNB`'s menu
    /// side): the sizes of `sphere.bin` and `panel.bin` (`getExcelBinSize`
    /// into a [`LANG_BUFFER`] block, freed after), a block of each size
    /// into `g_AbmapNodeKinds + 4` and `g_AbmapNodeKinds`, then (a tail
    /// jump) [`ei_abm_set_lang`], whose `eax` it returns.
    pub fn ei_abm_load(h: &mut impl ffx_bridge::Host) -> u32 {
        let f = h.rebase(USER_MALLOC);
        let buf = h.call(f, &[LANG_BUFFER]);
        let mut sizes = [0; 2];
        for (size, (name, _)) in sizes.iter_mut().zip(LANG_FILES) {
            let (f, name) = (h.rebase(GET_EXCEL_BIN_SIZE), h.rebase(name));
            *size = h.call(f, &[name, buf]);
        }
        let f = h.rebase(USER_FREE);
        h.call(f, &[buf]);
        for (size, (_, at)) in sizes.into_iter().zip(LANG_FILES) {
            let f = h.rebase(USER_MALLOC);
            let block = h.call(f, &[size]);
            Global::<u32>::new(at).set(h, block);
        }
        ei_abm_set_lang(h)
    }

    /// `MsGetSaveMenu()` (the menu settings word's address; bits 14-15
    /// pick the grid: 0 standard, 1 expert, else the third).
    pub const GET_SAVE_MENU: u32 = 0x0078_5290;
    /// Set once the 70 grid data files are loaded (`eiAbmDataInit`).
    pub const DATA_LOADED: Global<u32> = Global::new(0x01a8_5f60);
    /// Zeroed by every `eiAbmDataInit`.
    pub const DATA_INIT_CLEAR: Global<u32> = Global::new(0x01a8_5f70);
    /// The C runtime's `sprintf` and `sprintf_s` (import slots).
    pub const SPRINTF_SLOT: Global<u32> = Global::new(0x00b0_c3dc);
    pub const SPRINTF_S_SLOT: Global<u32> = Global::new(0x00b0_c394);
    /// `"%s/%s"`, `"/ffx_ps2/ffx/eiichi_abmap_data"`, `"%s"`, the
    /// not-found and the loaded `dbgPrintf` formats.
    pub const FMT_PATH: u32 = 0x00b5_85c8;
    pub const DATA_DIR: u32 = 0x00b7_b5a4;
    pub const FMT_S: u32 = 0x00b0_e160;
    pub const FMT_NOT_FOUND: u32 = 0x00b5_85ec;
    pub const FMT_LOADED: u32 = 0x00b5_862c;
    /// The data files' names (`"par_dna.pdt"`, ...): [`DATA_FILES`]
    /// pointers.
    pub const DATA_NAMES: u32 = 0x00c8_5ef0;
    pub const DATA_FILES: u32 = 70;
    /// `fiosUnifyFilename(src, out, size)`, `fiosOpen(path, mode)`,
    /// `fiosLseek(file, offset, whence)`, `user_aligned_malloc(size,
    /// align)`, `fiosRead(file, buffer, size)`, `fiosClose(file)`; the
    /// reports go to `dbgPrintf` ([`DBG_PRINTF`]).
    pub const FIOS_UNIFY: u32 = 0x0067_9820;
    pub const FIOS_OPEN: u32 = 0x0067_9730;
    pub const FIOS_LSEEK: u32 = 0x0067_9710;
    pub const ALIGNED_MALLOC: u32 = 0x0068_6ff0;
    pub const FIOS_READ: u32 = 0x0067_9800;
    pub const FIOS_CLOSE: u32 = 0x0067_9510;
    /// `g_CdLib + 0`: the CD library's read (`(table, buffer, 0, 0)`).
    pub const CDLIB_READ: Global<u32> = Global::new(0x0231_0c40);
    /// The grid's activation lists by grid (bits 14-15 of the menu word,
    /// 0, 1, else): seven pointers each, one per character, to `u16` node
    /// numbers ending in 0xffff.
    pub const ACTIVE_LISTS: [u32; 3] = [0x00c8_6c5c, 0x00c8_6ca4, 0x00c8_6cec];
    /// The characters' start nodes by grid: seven `u16` each.
    pub const START_NODES: [u32; 3] = [0x00c8_6c00, 0x00c8_6c10, 0x00c8_6c20];
    /// `eiAbmParaGet()`.
    pub const EI_ABM_PARA_GET: u32 = 0x00a5_4860;
    /// Where the loaded data blocks go: block `i` (but 16) to
    /// `0x02305800 - 4i`; blocks 6..=13 also to `0x01a85f78 + 4(i - 6)`,
    /// 0x1e..=0x30 to `0x01a85f98 + 4(i - 0x1e)`, 0x31..=0x43 to
    /// `0x01a85fe8 + 4(i - 0x31)`, block 14 to `0x01a85fe4`.
    pub const DATA_BLOCKS: u32 = 0x0230_5800;
    pub const DATA_COPIES: [(u32, std::ops::RangeInclusive<u32>); 4] = [
        (0x01a8_5f78, 6..=13),
        (0x01a8_5f98, 0x1e..=0x30),
        (0x01a8_5fe4, 14..=14),
        (0x01a8_5fe8, 0x31..=0x43),
    ];
    /// The block [`ei_abm_data_init`] works in, laid out as the original's
    /// frame from `ebp - 0x684`: the table read's buffer
    /// ([`DATA_INIT_HEADER`] bytes, 4 spare), the unified path and the
    /// path (0x100 bytes each).
    pub const DATA_INIT_WORK: u32 = 0x680;
    pub const DATA_INIT_HEADER: u32 = 0x47c;
    pub const DATA_INIT_UNIFIED: u32 = 0x480;
    pub const DATA_INIT_PATH: u32 = 0x580;

    /// `eiAbmDataInit(save)` (`0x00a53de0`, Fahrenheit's name; from
    /// `MsSetSaveStartGame` and `Native_0254`): the save's sphere grid
    /// reset for the grid the menu word picks (bits 14-15).
    ///
    /// Once ([`DATA_LOADED`]): each of the 70 grid data files
    /// `"/ffx_ps2/ffx/eiichi_abmap_data/<name>"` (`sprintf`,
    /// `fiosUnifyFilename`, `sprintf_s "%s"`) opened, its size taken
    /// (`fiosLseek` to the end and back), read into a new 16-aligned block
    /// and closed, with a `dbgPrintf` either way; the blocks stored as
    /// [`DATA_BLOCKS`] says. A file that does not open leaves its block
    /// word as the frame held it (garbage in the original): 0 here.
    ///
    /// Then [`DATA_INIT_CLEAR`] 0, the CD library's table (`g_CdLib +
    /// 0x10`) 0x25, read of table 0x11, 0x12 or 0x13 by grid into the
    /// work block's buffer, the read wait (`g_CdLib + 8`), `save`'s 0x1320
    /// bytes zeroed. A buffer whose `u16 +0` is 0x31 gives the nodes'
    /// kinds: `u16 +2` bytes from `+8` to the even bytes of `save`
    /// (beyond 0x678 the original reads past its frame; the data's count
    /// is far below). Each character `k` (0..7) sets bit `k` of the odd
    /// byte of each node of its activation list ([`ACTIVE_LISTS`]), and
    /// the seven start nodes ([`START_NODES`]) go to `save + 0xf00`. The
    /// grid is re-read from the menu word at each step. Ends with
    /// `eiAbmParaGet()`, whose `eax` it returns.
    pub fn ei_abm_data_init<H: ffx_bridge::Host>(h: &mut H, save: u32, work: u32) -> u32 {
        let f = h.rebase(GET_SAVE_MENU);
        let menu = h.call(f, &[]);
        // 0, 1, or 2 for the rest
        let grid = |h: &H| -> usize { (h.read::<u32>(menu) >> 14 & 3).min(2) as usize };
        if DATA_LOADED.get(h) == 0 {
            load_data(h, work);
        }
        let g = grid(h);
        DATA_INIT_CLEAR.set(h, 0);
        let f = CDLIB_READ_TABLE.get(h);
        h.call(f, &[0x25]);
        let table = [0x11, 0x12, 0x13][g];
        let header = work;
        let f = CDLIB_READ.get(h);
        h.call(f, &[table, header, 0, 0]);
        let f = CDLIB_READ_SYNC.get(h);
        h.call(f, &[0]);
        h.write_bytes(save, &[0u8; 0x1320]);
        if h.read::<u16>(header) == 0x31 {
            let count = u32::from(h.read::<u16>(header + 2));
            for i in 0..count {
                let kind = h.read::<u8>(header + 8 + i);
                h.write(save.wrapping_add(2 * i), kind);
            }
        }
        for k in 0..7u32 {
            let g = grid(h);
            let lists = h.rebase(ACTIVE_LISTS[g]);
            let mut at: u32 = h.read(lists + 4 * k);
            loop {
                let node: u16 = h.read(at);
                if node == 0xffff {
                    break;
                }
                let byte = save.wrapping_add(2 * u32::from(node) + 1);
                let v = h.read::<u8>(byte) | 1 << k;
                h.write(byte, v);
                at = at.wrapping_add(2);
            }
        }
        for k in 0..7u32 {
            let g = grid(h);
            let starts = h.rebase(START_NODES[g]);
            let node: u16 = h.read(starts + 2 * k);
            h.write(save.wrapping_add(0xf00 + 2 * k), node);
        }
        let f = h.rebase(EI_ABM_PARA_GET);
        h.call(f, &[])
    }

    /// [`ei_abm_data_init`]'s one-time load of the grid data files.
    fn load_data(h: &mut impl ffx_bridge::Host, work: u32) {
        let (path, unified) = (work + DATA_INIT_PATH, work + DATA_INIT_UNIFIED);
        let mut blocks = [0u32; DATA_FILES as usize];
        for (i, block) in (0u32..).zip(blocks.iter_mut()) {
            let name: u32 = h.read(h.rebase(DATA_NAMES) + 4 * i);
            let (fmt, dir) = (h.rebase(FMT_PATH), h.rebase(DATA_DIR));
            let f = SPRINTF_SLOT.get(h);
            h.call(f, &[path, fmt, dir, name]);
            let f = h.rebase(FIOS_UNIFY);
            h.call(f, &[path, unified, 0x100]);
            let (f, fmt) = (SPRINTF_S_SLOT.get(h), h.rebase(FMT_S));
            h.call(f, &[path, 0xfe, fmt, unified]);
            let f = h.rebase(FIOS_OPEN);
            let file = h.call(f, &[path, 1]);
            if file == 0 {
                let (f, fmt) = (h.rebase(DBG_PRINTF), h.rebase(FMT_NOT_FOUND));
                h.call(f, &[fmt, path]);
                continue;
            }
            let f = h.rebase(FIOS_LSEEK);
            let size = h.call(f, &[file, 0, 2]);
            h.call(f, &[file, 0, 0]);
            let f = h.rebase(ALIGNED_MALLOC);
            *block = h.call(f, &[size, 0x10]);
            let f = h.rebase(FIOS_READ);
            h.call(f, &[file, *block, size]);
            let f = h.rebase(FIOS_CLOSE);
            h.call(f, &[file]);
            let (f, fmt) = (h.rebase(DBG_PRINTF), h.rebase(FMT_LOADED));
            h.call(f, &[fmt, path, size]);
        }
        for (i, &block) in (0u32..).zip(&blocks) {
            if i != 16 {
                Global::<u32>::new(DATA_BLOCKS - 4 * i).set(h, block);
            }
        }
        for (at, range) in DATA_COPIES {
            for (k, i) in (0u32..).zip(range) {
                Global::<u32>::new(at + 4 * k).set(h, blocks[i as usize]);
            }
        }
        DATA_LOADED.set(h, 1);
    }

    /// `op_shape_draw_eamap(shape, key, slot, layer)` (`0x007f49e0`) and
    /// the `.rdata` 16384.0 (`0x00b7b708`) the area-map key's vertex block
    /// ends with.
    pub const SHAPE_DRAW_EAMAP: u32 = 0x007f_49e0;
    pub const K16384: u32 = 0x00b7_b708;
    /// Bytes of work [`abmap_draw_nodes_eamap`] needs: the key
    /// (`[ebp-0x100]`), the node matrix (`[ebp-0xc8]`), the view times
    /// node matrix (`[ebp-0x88]`) and the vertex block (`[ebp-0x48]`).
    pub const EAMAP_WORK: u32 = 0x100;

    /// `AbmapDrawNodesEamap_00a51340()` (name ours; from the grid's draw
    /// pass `0x00a54660`): every node of the grid into the area map.
    /// The key (`work`, read by `op_shape_draw_eamap`): `u16 +2` 0x14,
    /// colour `+4`, `+8` and `+0xc` 0, the matrix at `+0x10`, byte `+0x18`
    /// 0, the vertex block at `+0x1c` (the EE zero register three times,
    /// then `(r0.x, r0.y, 16384, 1)`). For node `n` (counted down from the
    /// `i16` count `+2`, `i` the nodes left): with a kind (`u16 +6`, not
    /// `0xffff`) its matrix (`AbmapMakeNodeMatrix(.., 1.0)`, times the view
    /// `+0x113e0`), then the kind's (0x30-byte records from `+0xf828`)
    /// first shape (`+0`, colour `0x80454545`) when the node's panel bits
    /// (`+0x21`) hold bit `[+0x115bc]`, else its second (`+4`, colour
    /// `0x80121212`) into slot `count - i + 0x35c`, and its third (`+8`,
    /// colour 0) appended (slot -0x35e); without a kind only an empty quad
    /// (shape 0, slot 0xffff). The layer is `[+0x116a4]`; the grid pointer
    /// and its count are read again for each call.
    pub fn abmap_draw_nodes_eamap<M, C>(mem: &mut M, work: u32, call: &mut C)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let (key, node_m, m) = (work, work + 0x38, work + 0x78);
        let g = ABMAP.get(mem);
        let mut left = i32::from(mem.read::<i16>(g.wrapping_add(2)));
        let bit = mem.read::<u8>(g.wrapping_add(0x115bc));
        let bit = if u32::from(bit & 0x1f) < 8 {
            1u8 << (bit & 0x1f)
        } else {
            0
        };
        let kinds = g.wrapping_add(0xf828);
        let mut node = g.wrapping_add(0x808);
        eamap_key(mem, key, m, work + 0xb8);
        let draw = |mem: &mut M, call: &mut C, shape: u32, slot: u32| {
            let g = ABMAP.get(mem);
            let layer: u32 = mem.read(g.wrapping_add(0x116a4));
            let f = mem.rebase(SHAPE_DRAW_EAMAP);
            call(mem, f, &[shape, key, slot, layer]);
        };
        while left != 0 {
            left = left.wrapping_sub(1);
            let kind = mem.read::<u16>(node.wrapping_add(6));
            if kind == 0xffff {
                draw(mem, call, 0, 0xffff);
            } else {
                let f = mem.rebase(MAKE_NODE_MATRIX);
                call(mem, f, &[node_m, node, 1f32.to_bits()]);
                let view = ABMAP.get(mem).wrapping_add(0x113e0);
                let f = mem.rebase(MUL_MATRIX);
                call(mem, f, &[m, view, node_m]);
                let k = kinds.wrapping_add(u32::from(kind) * 0x30);
                let (colour, shape) = if mem.read::<u8>(node.wrapping_add(0x21)) & bit != 0 {
                    (0x8045_4545u32, mem.read::<u32>(k))
                } else {
                    (0x8012_1212, mem.read::<u32>(k.wrapping_add(4)))
                };
                mem.write(key + 4, colour);
                let g = ABMAP.get(mem);
                let count = i32::from(mem.read::<i16>(g.wrapping_add(2)));
                let slot = count.wrapping_sub(left).wrapping_add(0x35c).cast_unsigned();
                draw(mem, call, shape, slot);
                mem.write(key + 4, 0u32);
                let third: u32 = mem.read(k.wrapping_add(8));
                draw(mem, call, third, (-0x35ei32).cast_unsigned());
            }
            node = node.wrapping_add(0x28);
        }
    }

    /// The area-map key the eamap draws hand `op_shape_draw_eamap`: see
    /// [`abmap_draw_nodes_eamap`]; `+0`, `+4` and the bytes after `+0x18`
    /// are left as they were (the original's stack).
    fn eamap_key(mem: &mut impl Mem, key: u32, m: u32, verts: u32) {
        let r0: [u32; 4] = mem.read(mem.rebase(EE_R0));
        mem.write(key + 2, 0x14u16);
        mem.write(key + 8, [0u32, 0, m]);
        mem.write(key + 0x18, 0u8);
        mem.write(key + 0x1c, verts);
        for row in 0..3 {
            mem.write(verts + 16 * row, r0);
        }
        let k: f32 = mem.read(mem.rebase(K16384));
        mem.write(
            verts + 0x30,
            [r0[0], r0[1], x87::round_trip(k).to_bits(), 1f32.to_bits()],
        );
    }
}
