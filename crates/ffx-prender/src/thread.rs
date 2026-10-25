//! The render thread's side of the map path (docs/draw.md, "The map
//! path"): the mesh render-command handlers `PRenderThread_runMeshCommand`
//! `0x006144f0` and `PRenderThread_runMeshCommand2` `0x00614610`
//! (cdecl `(ctx, cmd)`, called from `processRenderCommands` with the
//! renderer context `0x00c94f00` and the 72-byte command's `+8`), and the
//! shader pass state's bind `PShaderPassStateD3D11_bind` `0x0058c6c0`
//! (thiscall `(state, ctx)`, `ret 4`, returns 0).
//!
//! `Direct3D 11` stays a library: the bind's three context calls are made
//! through the context's vtable (`ID3D11DeviceContext` slots 43
//! `RSSetState`, 35 `OMSetBlendState`, 36 `OMSetDepthStencilState`,
//! `stdcall` with the context first), as the original makes them, and the
//! handlers call the original draw wrappers (`0x00592b00`, `0x00592b60`,
//! `0x00592c10`, `0x00592c60`, `0x00592de0`) and a dynamic mesh's own
//! virtual draw.
//!
//! A command (`cmd`, the 72-byte command's `+8`), as the handlers read it:
//! `+0x00` the mesh instance, `+0x04` a dynamic mesh (with its own draw)
//! or 0, `+0x08` its data, `+0x14` the segment (`+0xc`: an indirect draw),
//! `+0x18` the index stream (`+0x10` the format), `+0x1c` the data-block
//! kind (`u16`), `+0x1e` the instance count (`u16`), `+0x34` the start
//! vertex or 0, `+0x38` the index count, `+0x3c` the second handler's
//! extra word.

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PGeometry_PDynamicMesh_getDataBlockCount` (thiscall `(mesh,
/// kind)`: the blocks of that kind in the mesh).
pub const DATA_BLOCK_COUNT: u32 = 0x0049_8770;
/// The block-list destructor (thiscall `(list)`).
pub const BLOCK_LIST_DTOR: u32 = 0x0061_41a0;
/// Draw wrappers (thiscall on the renderer context).
pub const DRAW_INDIRECT: u32 = 0x0059_2b00;
pub const DRAW_INDEXED: u32 = 0x0059_2b60;
pub const DRAW_INDEXED_INSTANCED: u32 = 0x0059_2c60;
pub const DRAW_INDEXED_2: u32 = 0x0059_2c10;
pub const DRAW_INDEXED_INSTANCED_2: u32 = 0x0059_2de0;

/// Frame bytes the handlers need (the block list they pass by address).
pub const HANDLER_FRAME: u32 = 8;
/// Frame bytes the bind needs (the blend factor).
pub const BIND_FRAME: u32 = 16;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BLOCK_LIST_DTOR, Conv, DATA_BLOCK_COUNT, DRAW_INDEXED, DRAW_INDEXED_2,
        DRAW_INDEXED_INSTANCED, DRAW_INDEXED_INSTANCED_2, DRAW_INDIRECT, Host,
    };

    /// A dynamic mesh's draw: the block list `[count | 0x80000000, data]`
    /// in `frame`, the virtual call at `slot` with `args` after `(ctx,
    /// instance, stream, &list)`, then the list's destructor.
    fn dynamic(h: &mut impl Host, frame: u32, ctx: u32, cmd: u32, slot: u32, tail: &[u32]) {
        let dynamic: u32 = h.read(cmd + 4);
        let kind = u32::from(h.read::<u16>(cmd + 0x1c));
        let inst: u32 = h.read(cmd);
        let stream: u32 = h.read(cmd + 0x18);
        let f = h.rebase(DATA_BLOCK_COUNT);
        // thiscall: the original leaves the dynamic mesh in ecx (Ghidra
        // shows a plain function)
        let n = h.call(f, Conv::Thiscall(dynamic), &[kind]);
        let data: u32 = h.read(cmd + 8);
        h.write(frame + 4, data);
        h.write(frame, n | 0x8000_0000);
        let mut args = vec![ctx, inst, stream, frame];
        args.extend_from_slice(tail);
        let vtable: u32 = h.read(dynamic);
        let m: u32 = h.read(vtable + 4 * slot);
        h.call(m, Conv::Thiscall(dynamic), &args);
        let f = h.rebase(BLOCK_LIST_DTOR);
        h.call(f, Conv::Thiscall(frame), &[]);
    }

    /// `PRenderThread_runMeshCommand(ctx, cmd)`.
    pub fn run_mesh_command(h: &mut impl Host, frame: u32, ctx: u32, cmd: u32) {
        let instances = u32::from(h.read::<u16>(cmd + 0x1e));
        let start: u32 = h.read(cmd + 0x34);
        if h.read::<u32>(cmd + 4) != 0 {
            dynamic(h, frame, ctx, cmd, 3, &[start, instances]);
            return;
        }
        let segment: u32 = h.read(cmd + 0x14);
        let indirect = h.read::<u32>(segment + 0xc) != 0;
        let stream: u32 = h.read(cmd + 0x18);
        let format: u32 = h.read(stream + 0x10);
        let count: u32 = h.read(cmd + 0x38);
        if indirect {
            // ceil(count / 64)
            let groups = (count >> 6) + u32::from((count >> 6) << 6 < count);
            let f = h.rebase(DRAW_INDIRECT);
            h.call(f, Conv::Thiscall(ctx), &[groups, 1, 1]);
        } else if start != 0 {
            let f = h.rebase(DRAW_INDEXED_INSTANCED);
            h.call(f, Conv::Thiscall(ctx), &[format, count, start, instances]);
        } else {
            let f = h.rebase(DRAW_INDEXED);
            h.call(f, Conv::Thiscall(ctx), &[format, 0, count]);
        }
    }

    /// `PRenderThread_runMeshCommand2(ctx, cmd)`: as
    /// [`run_mesh_command`] with the command's `+0x3c` word passed on and
    /// no indirect case.
    pub fn run_mesh_command2(h: &mut impl Host, frame: u32, ctx: u32, cmd: u32) {
        let instances = u32::from(h.read::<u16>(cmd + 0x1e));
        let start: u32 = h.read(cmd + 0x34);
        let extra: u32 = h.read(cmd + 0x3c);
        if h.read::<u32>(cmd + 4) != 0 {
            dynamic(h, frame, ctx, cmd, 4, &[start, instances, extra]);
            return;
        }
        let stream: u32 = h.read(cmd + 0x18);
        let format: u32 = h.read(stream + 0x10);
        let count: u32 = h.read(cmd + 0x38);
        if start != 0 {
            let f = h.rebase(DRAW_INDEXED_INSTANCED_2);
            h.call(
                f,
                Conv::Thiscall(ctx),
                &[format, count, start, instances, extra],
            );
        } else {
            let f = h.rebase(DRAW_INDEXED_2);
            h.call(f, Conv::Thiscall(ctx), &[format, 0, count, extra]);
        }
    }

    /// A context method: `vtable[slot](ctx, args...)` (`stdcall`).
    fn com(h: &mut impl Host, ctx: u32, slot: u32, args: &[u32]) {
        let vtable: u32 = h.read(ctx);
        let m: u32 = h.read(vtable + 4 * slot);
        let mut all = vec![ctx];
        all.extend_from_slice(args);
        h.call(m, Conv::Stdcall, &all);
    }

    /// `PShaderPassStateD3D11_bind(state, ctx)` -> 0: the pass's
    /// rasterizer state (`+0x168`, or `+0x16c` when the context's
    /// `+0x574` is 2 and it has one), blend state (`+0x174`, factor 0,
    /// mask all ones) and depth-stencil state (`+0x170`, the stencil
    /// reference byte `+0x178`), each when set, on the context's
    /// `ID3D11DeviceContext` (`ctx +0x70`). `frame` holds the blend factor.
    pub fn bind_pass_state(h: &mut impl Host, frame: u32, state: u32, ctx: u32) -> u32 {
        let rs: u32 = h.read(state + 0x168);
        if rs != 0 {
            let alt: u32 = h.read(state + 0x16c);
            let pick = if h.read::<u32>(ctx + 0x574) == 2 && alt != 0 {
                alt
            } else {
                rs
            };
            let d3d: u32 = h.read(ctx + 0x70);
            com(h, d3d, 43, &[pick]);
        }
        let blend: u32 = h.read(state + 0x174);
        if blend != 0 {
            h.write(frame, [0f32; 4]);
            let d3d: u32 = h.read(ctx + 0x70);
            com(h, d3d, 35, &[blend, frame, 0xffff_ffff]);
        }
        let depth: u32 = h.read(state + 0x170);
        if depth != 0 {
            let stencil = u32::from(h.read::<u8>(state + 0x178));
            let d3d: u32 = h.read(ctx + 0x70);
            com(h, d3d, 36, &[depth, stencil]);
        }
        0
    }
}
