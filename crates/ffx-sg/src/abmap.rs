//! Sphere grid (ability map) draws in the main module's range, called by
//! the grid's draw pass `0x00a54660`. They hand each draw to the effect
//! model drawers (`op1_md_draw_eiabm`, `op1_md_draw_eiabm_sphe`) with a
//! request built in their own frame; the bridges build it in `work`, a
//! block laid out as that frame (see [`bridge::NODE_WORK`]).

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
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
}
