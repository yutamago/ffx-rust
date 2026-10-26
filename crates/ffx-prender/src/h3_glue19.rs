//! The render-side glue, nineteenth batch (M4 port round 6, list
//! `h3-glue-render`): the broken screen's index fill and colour-buffer
//! parameter, the shadow manager's per-frame cascade dispatch, the VFX
//! render-data table's constructor, the scene's VFX draw loop and the
//! global camera search. The exception frame and cookie are left out, as
//! in [`crate::glue10`].

pub use crate::glue::SCENE;

/// `TrophyManager_Install(this, a)` (`ret 4`; the shared `xor eax, eax`
/// body the broken screen calls on its texture).
pub const TROPHY_INSTALL: u32 = 0x0059_e520;

/// `MsGetEncountType()`, `g_ShadowManager`, the battle shadow pass
/// `0x006d00d0(this, a, battle)`, the cascade fit `0x006c8090(this, a,
/// box, splits, flag)` and the cascade pass `0x006cebd0(this, a, b)`.
pub const ENCOUNT_TYPE: u32 = 0x0063_8e20;
pub const SHADOW_MANAGER: u32 = 0x00cd_ec28;
pub const SHADOW_BATTLE_PASS: u32 = 0x006d_00d0;
pub const SHADOW_FIT: u32 = 0x006c_8090;
pub const SHADOW_CASCADE_PASS: u32 = 0x006c_ebd0;
/// The map ids whose shadows always take the battle pass.
pub const SHADOW_BATTLE_MAPS: [u32; 3] = [0x11f7, 0x1eda, 0x4a6];

/// The VFX render-data table's constructor: the map head maker
/// `0x006a0390()`, `user_aligned_malloc(size, align)`,
/// `operator_new_006304c0(size)` (cdecl), `eh_vector_constructor_iterator`
/// (stdcall) with `VFXDynamicGeometry`'s constructor and destructor,
/// `__iob_func` / `fprintf` (import slots) and the format.
pub const MAP_HEAD: u32 = 0x006a_0390;
pub const ALIGNED_MALLOC: u32 = 0x0068_6ff0;
pub const OPERATOR_NEW: u32 = 0x0063_04c0;
pub const VECTOR_CTOR: u32 = 0x0094_966a;
pub const GEOMETRY_CTOR: u32 = 0x0069_e3e0;
pub const GEOMETRY_DTOR: u32 = 0x0069_ea10;
pub const IOB_FUNC: u32 = crate::glue4::IOB_FUNC;
pub const FPRINTF: u32 = crate::glue4::FPRINTF;
pub const POOLS_FORMAT: u32 = 0x00b4_7fc4;
/// The pools: the data pool's size and alignment, the geometry count and
/// size, the segment, name and flag pools' sizes.
pub const DATA_POOL: (u32, u32) = (0x9_8580, 0x10);
pub const GEOMETRY_COUNT: u32 = 3000;
pub const GEOMETRY_SIZE: u32 = 0xf8;
pub const SEGMENT_FLAGS: u32 = 3000;
pub const SEGMENT_POOL: u32 = 0x6_9780;
pub const NAME_POOL: u32 = 0xf_a000;
pub const NAME_FLAGS: u32 = 4000;

/// The VFX draw loop: `g_VFXRenderDataTable`,
/// `GetDynamicGeometryByInstance(this, 0x78, 0x5bc)`,
/// `MagicFile_GetMagicId()`, `renderMeshInstance(renderer, mesh, data, 0,
/// context)`, `PhyreScene_processVFXDrawEvent(this, event)` and the
/// renderer's pass name `"Transparent"`'s entry.
pub const VFX_RENDER_TABLE: u32 = 0x00cc_ca6c;
pub const GEOMETRY_BY_INSTANCE: u32 = 0x0069_f620;
pub const MAGIC_ID: u32 = 0x009d_a3b0;
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
pub const PROCESS_VFX_DRAW_EVENT: u32 = 0x0066_6e40;
pub const TRANSPARENT_PASS: u32 = 0x00c1_69d4;
/// The magic whose one geometry is drawn last.
pub const LAST_DRAWN_MAGIC: u32 = 0x296;

/// The character unload task: `ClassLoadedCharacterTable_GetInstance`,
/// `existPrototype(this, chr)`, `Phyre_PhyrePrintf(level, text)` and its
/// message, `ClassCharacter_clearInstanceList(this)`,
/// `PhyreScene_0x00670b20(this, chr)` and `Tree_eraseValue_006a1d10(this,
/// chr)`.
pub const LOADED_CHARACTER_TABLE: u32 = crate::h3_glue14::LOADED_CHARACTER_TABLE;
pub const EXIST_PROTOTYPE: u32 = 0x006a_1e80;
pub const PHYRE_PRINTF: u32 = crate::glue10::PHYRE_PRINTF;
pub const UNLOAD_MISSING: u32 = 0x00b4_17b0;
pub const CLEAR_INSTANCE_LIST: u32 = 0x006a_1200;
pub const SCENE_UNLOAD_CHARACTER: u32 = 0x0067_0b20;
pub const ERASE_VALUE: u32 = 0x006a_1d10;

/// The global camera search: the far plane `65536.0`.
pub const CAMERA_FAR: u32 = 0x00b4_3ddc;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// The 24 indices of the six-vertex shape, then the 36 of the
    /// eight-vertex one.
    const INDICES_6: [u32; 24] = [
        0, 4, 1, 0, 1, 2, 0, 2, 3, 0, 3, 5, 0, 5, 4, 1, 4, 5, 5, 2, 1, 3, 2, 5,
    ];
    const INDICES_8: [u32; 36] = [
        0, 4, 6, 0, 6, 1, 0, 1, 2, 0, 2, 3, 1, 6, 7, 1, 7, 2, 4, 5, 7, 4, 7, 6, 0, 3, 4, 3, 5, 4,
        3, 2, 7, 3, 7, 5,
    ];

    /// `BrokenScreenEffect_fillIndices_006d41b0(this)` (`ecx`): the shape
    /// of `+8` (6 or 8 vertices) gets its triangle list written to the
    /// index buffer `+0x50` (re-read before each store, as the original
    /// does). `eax`: the buffer pointer last read, or the shape.
    pub fn broken_fill_indices(h: &mut impl Host, this: u32) -> u32 {
        let shape: u32 = h.read(this + 8);
        let indices: &[u32] = match shape {
            6 => &INDICES_6,
            8 => &INDICES_8,
            _ => return shape,
        };
        let mut buffer: u32 = 0;
        for (k, &index) in (0u32..).zip(indices) {
            buffer = h.read(this + 0x50);
            h.write(buffer.wrapping_add(4 * k), index);
        }
        buffer
    }

    /// `BrokenScreenEffect_setColorBuffer_006d4540(this, texture,
    /// sampler)` (`ret 8`; frame: the 16-byte record `{0x42, 0, sampler,
    /// texture}` at `ebp - 0x10`). With the parameter `+0xa8` of a texture
    /// kind (`+2`: 0x42, 0xd9..0xdc or 0x12) the record is copied into the
    /// parameter buffer (`[+0x1c] + 4`, its size the first word) at the
    /// parameter's offset `+8` (16 bits) for its size `+0xa` (13 bits).
    /// With both words set, `TrophyManager_Install(sampler, texture)`'s
    /// error comes first. `eax`: 0, 3 (out of the buffer, or an offset
    /// under 4), 5 (another kind) or 0x13 (no parameter).
    ///
    /// A size over 16 copies what lies above the record: the original's
    /// saved `ebp`, return address and arguments, the port's own frame.
    /// The game's colour buffer is 16 bytes.
    pub fn broken_set_color_buffer(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        texture: u32,
        sampler: u32,
    ) -> u32 {
        let parameter: u32 = h.read(this + 0xa8);
        if parameter == 0 {
            return 0x13;
        }
        let block: u32 = h.read(this + 0x1c);
        let buffer: u32 = h.read(block.wrapping_add(4));
        let kind: u8 = h.read(parameter.wrapping_add(2));
        let record = ebp - 0x10;
        h.write(record, 0x42u32);
        h.write(ebp - 0xc, 0u32);
        h.write(ebp - 8, sampler);
        h.write(ebp - 4, texture);
        if !matches!(kind, 0x42 | 0xd9..=0xdc | 0x12) {
            return 5;
        }
        if sampler != 0 && texture != 0 {
            let error = h.call_sym(TROPHY_INSTALL, Conv::Thiscall(sampler), &[texture]);
            if error != 0 {
                return error;
            }
        }
        let offset = u32::from(h.read::<u16>(parameter.wrapping_add(8)));
        let size = u32::from(h.read::<u16>(parameter.wrapping_add(0xa)) & 0x1fff);
        if offset + size > h.read::<u32>(buffer) || offset < 4 {
            return 3;
        }
        let mut bytes = vec![0; size as usize];
        h.read_bytes(record, &mut bytes);
        h.write_bytes(buffer.wrapping_add(offset), &bytes);
        0
    }

    /// The cascade bytes: `+0x2b28..+0x2b68` set to `first`, the 64 after
    /// them cleared.
    fn clear_cascades(h: &mut impl Host, this: u32, first: u8) {
        for k in 0..64 {
            h.write(this + 0x2b28 + k, first);
            h.write(this + 0x2b68 + k, 0u8);
        }
    }

    /// `ShadowManager_updateCascades_006ce860(this, flag, a, b, map;
    /// slots)` (`ret 0x10`): the 64 cascade bytes `+0x2b28` and the 64 at
    /// `+0x2b68` are cleared. In a battle (`MsGetEncountType() > 0`) or
    /// on one of [`SHADOW_BATTLE_MAPS`], or with the scene's `+0x1196`, a
    /// set `flag` byte makes the first 64 bytes 1 and runs the battle pass
    /// on `g_ShadowManager`; otherwise the cascades are fitted twice
    /// (`+0x290`/`+0x3b0`, then `+0x2a0`/`+0x190`) around the cascade
    /// pass. The battle byte is stored over `map`'s low byte (`slots +
    /// 0xc`), and that word is what the battle pass gets. `eax`: the last
    /// callee's.
    pub fn shadow_update_cascades(
        h: &mut impl Host,
        slots: u32,
        this: u32,
        flag: u32,
        a: u32,
        b: u32,
        map: u32,
    ) -> u32 {
        clear_cascades(h, this, 0);
        let fixed = SHADOW_BATTLE_MAPS.contains(&map);
        let encount = h.call_sym(ENCOUNT_TYPE, Conv::Cdecl, &[]);
        let battle = (encount as i32) > 0 || fixed;
        h.write(slots + 0xc, u8::from(battle));
        let manager_at = h.rebase(SHADOW_MANAGER);
        if flag as u8 != 0 {
            let scene: u32 = SCENE.get(h);
            if h.read::<u8>(scene.wrapping_add(0x1196)) != 0 || battle {
                clear_cascades(h, this, 1);
                let word: u32 = h.read(slots + 0xc);
                let manager: u32 = h.read(manager_at);
                return h.call_sym(SHADOW_BATTLE_PASS, Conv::Thiscall(manager), &[b, word]);
            }
        }
        let manager: u32 = h.read(manager_at);
        h.call_sym(
            SHADOW_FIT,
            Conv::Thiscall(manager),
            &[a, this + 0x290, this + 0x3b0, flag],
        );
        let manager: u32 = h.read(manager_at);
        h.call_sym(SHADOW_CASCADE_PASS, Conv::Thiscall(manager), &[b, a]);
        let manager: u32 = h.read(manager_at);
        h.call_sym(
            SHADOW_FIT,
            Conv::Thiscall(manager),
            &[a, this + 0x2a0, this + 0x190, flag],
        )
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, Conv::Cdecl, args)
    }

    /// `ClassVFXRenderDataTable_ClassVFXRenderDataTable(this)`: the map
    /// head (`+0`, size `+4`), the cached key and geometry (`+8..+0x24`)
    /// cleared, the pools allocated: data `+0x2c`, 3000 geometries
    /// `+0x30` (constructed, after their count word), their flags `+0x34`
    /// (cleared, count `+0x38`), segments `+0x3c`, names `+0x40` and
    /// their flags `+0x44` (cleared, count `+0x48`); the pools are printed
    /// to `stdout`. `eax`: `this`.
    pub fn vfx_table_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        let head = h.call_sym(MAP_HEAD, Conv::Cdecl, &[]);
        h.write(this, head);
        for k in 2..10 {
            h.write(this + 4 * k, 0u32);
        }
        let data = h.call_sym(ALIGNED_MALLOC, Conv::Cdecl, &[DATA_POOL.0, DATA_POOL.1]);
        h.write(this + 0x2c, data);
        let block = h.call_sym(
            OPERATOR_NEW,
            Conv::Cdecl,
            &[4 + GEOMETRY_COUNT * GEOMETRY_SIZE],
        );
        let geometries = if block == 0 {
            0
        } else {
            h.write(block, GEOMETRY_COUNT);
            let ctor = h.rebase(GEOMETRY_CTOR);
            let dtor = h.rebase(GEOMETRY_DTOR);
            h.call_sym(
                VECTOR_CTOR,
                Conv::Stdcall,
                &[block + 4, GEOMETRY_SIZE, GEOMETRY_COUNT, ctor, dtor],
            );
            block + 4
        };
        h.write(this + 0x30, geometries);
        let flags = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[SEGMENT_FLAGS]);
        h.write(this + 0x34, flags);
        h.write_bytes(flags, &vec![0; SEGMENT_FLAGS as usize]);
        h.write(this + 0x38, 0u32);
        let segments = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[SEGMENT_POOL]);
        h.write(this + 0x3c, segments);
        let names = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[NAME_POOL]);
        h.write(this + 0x40, names);
        let name_flags = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[NAME_FLAGS]);
        h.write(this + 0x44, name_flags);
        h.write_bytes(name_flags, &vec![0; NAME_FLAGS as usize]);
        h.write(this + 0x48, 0u32);
        let names: u32 = h.read(this + 0x40);
        let segments: u32 = h.read(this + 0x3c);
        let geometries: u32 = h.read(this + 0x30);
        let data: u32 = h.read(this + 0x2c);
        let format = h.rebase(POOLS_FORMAT);
        let iob = import(h, IOB_FUNC, &[]);
        import(
            h,
            FPRINTF,
            &[
                iob.wrapping_add(0x20),
                format,
                data,
                geometries,
                segments,
                names,
            ],
        );
        this
    }

    /// One VFX mesh drawn: `renderMeshInstance` on the renderer `+0xd0`
    /// with the mesh's data `+0x10` and the renderer's context `+0x13c`.
    fn draw_vfx(h: &mut impl Host, this: u32, mesh: u32) {
        let renderer: u32 = h.read(this + 0xd0);
        let context: u32 = h.read(renderer.wrapping_add(0x13c));
        let data: u32 = h.read(mesh.wrapping_add(0x10));
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[mesh, data, 0, context],
        );
    }

    /// `PhyreScene_DrawMostVFX(this)`: the VFX meshes `+0x38cc` (count
    /// `+0xf8cc`) drawn in the transparent pass, interleaved with the draw
    /// events `+0xfb5c` (16 bytes each, count `+0xfb54`), each run before
    /// the mesh index its `+4` names (and may extend the meshes drawn past
    /// the count). While magic [`LAST_DRAWN_MAGIC`] plays, the mesh of
    /// its dynamic geometry (`+0xc`) is held back and drawn last.
    /// `MagicFile_GetMagicId` gets the bound in `ecx`, as the original
    /// leaves it. `eax`: 0, or the first event's error.
    pub fn scene_draw_most_vfx(h: &mut impl Host, this: u32) -> u32 {
        let table: u32 = h.read(h.rebase(VFX_RENDER_TABLE));
        let geometry = h.call_sym(GEOMETRY_BY_INSTANCE, Conv::Thiscall(table), &[0x78, 0x5bc]);
        let pass = h.rebase(TRANSPARENT_PASS);
        let mut mesh_at = 0u32;
        let mut event = 0u32;
        let mut last = None;
        loop {
            let mut bound: u32 = h.read(this + 0xf8cc);
            let events: u32 = h.read(this + 0xfb54);
            if (mesh_at as i32) >= (bound as i32) && event >= events {
                break;
            }
            if event < events {
                let list: u32 = h.read(this + 0xfb5c);
                bound = h.read(list.wrapping_add(16 * event + 4));
            }
            let renderer: u32 = h.read(this + 0xd0);
            h.write(renderer.wrapping_add(0x48), pass);
            while (mesh_at as i32) < (bound as i32) {
                let slot = this.wrapping_add(0x38cc).wrapping_add(4 * mesh_at);
                let held = geometry != 0
                    && h.call_sym(MAGIC_ID, Conv::Thiscall(bound), &[]) == LAST_DRAWN_MAGIC;
                let mesh: u32 = h.read(slot);
                if held && mesh != 0 && h.read::<u32>(geometry + 0xc) == mesh {
                    last = Some(mesh_at);
                } else if mesh != 0 {
                    draw_vfx(h, this, mesh);
                }
                mesh_at = mesh_at.wrapping_add(1);
            }
            while event < h.read::<u32>(this + 0xfb54) {
                let list: u32 = h.read(this + 0xfb5c);
                let at = list.wrapping_add(16 * event);
                if h.read::<u32>(at + 4) as i32 > mesh_at as i32 {
                    break;
                }
                let error = h.call_sym(PROCESS_VFX_DRAW_EVENT, Conv::Thiscall(this), &[at]);
                if error != 0 {
                    return error;
                }
                event += 1;
            }
        }
        if let Some(k) = last {
            let mesh: u32 = h.read(this.wrapping_add(0x38cc).wrapping_add(4 * k));
            draw_vfx(h, this, mesh);
        }
        0
    }

    /// A character's instance count: its list's bytes (`+8..+0xc`) / 4,
    /// signed.
    fn instance_count(h: &impl Host, chr: u32) -> u32 {
        let end: u32 = h.read(chr + 0xc);
        let begin: u32 = h.read(chr + 8);
        ((end.wrapping_sub(begin) as i32) >> 2) as u32
    }

    /// `graphicCharacterUnloadTask(chr)` (cdecl; frame: the instance at
    /// `ebp - 4`, its character `ebp - 8`, its mesh `ebp - 0xc`): a
    /// loaded character's instances (`+8..+0xc`) are torn down as
    /// `graphicCharacterInstanceDestroy` does (a null one unregisters load
    /// -1); then its instance list is cleared, the scene unloads it, the
    /// loaded table forgets it and it destroys itself (virtual slot 0 with
    /// 1). An unknown one only prints. `eax`: the last callee's (`None`
    /// without a character: the original leaves the caller's).
    pub fn character_unload_task(h: &mut impl Host, ebp: u32, chr: u32) -> Option<u32> {
        if chr == 0 {
            return None;
        }
        let table = h.call_sym(LOADED_CHARACTER_TABLE, Conv::Cdecl, &[]);
        if h.call_sym(EXIST_PROTOTYPE, Conv::Thiscall(table), &[chr]) as u8 == 0 {
            let text = h.rebase(UNLOAD_MISSING);
            return Some(h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, text]));
        }
        let mut k = 0u32;
        if instance_count(h, chr) != 0 {
            loop {
                let list: u32 = h.read(chr + 8);
                let instance: u32 = h.read(list.wrapping_add(4 * k));
                if instance != 0 {
                    let mesh: u32 = h.read(instance + 0x18);
                    h.write(ebp - 0xc, mesh);
                    let character: u32 = h.read(instance + 0x10);
                    h.write(ebp - 8, character);
                }
                crate::h3_glue14::bridge::character_instance_destroy(
                    h,
                    ebp - 4,
                    instance,
                    u32::MAX,
                );
                k += 1;
                if k >= instance_count(h, chr) {
                    break;
                }
            }
        }
        h.call_sym(CLEAR_INSTANCE_LIST, Conv::Thiscall(chr), &[]);
        let scene = SCENE.get(h);
        h.call_sym(SCENE_UNLOAD_CHARACTER, Conv::Thiscall(scene), &[chr]);
        let table = h.call_sym(LOADED_CHARACTER_TABLE, Conv::Cdecl, &[]);
        h.call_sym(ERASE_VALUE, Conv::Thiscall(table), &[chr]);
        Some(h.call_virtual(chr, 0, &[1]))
    }

    /// `PhyreScene_findGlobalCamera(this)` (frame: the cursor at `ebp -
    /// 0x28`, the object at `ebp - 8`, the scene at `ebp - 4`): the camera
    /// pointers `+0xdc`, `+0x238`, `+0x654` set to their objects; with a
    /// map cluster (`+0x10d4`) its cameras are walked and the one named
    /// `"GlobalCamera"` becomes `+0x4f8`, then the two cameras `+0x238`
    /// and `+0x654` get near 1.0 (`+0xb8`) and far 65536.0 (`+0xbc`),
    /// each store followed by their update (virtual slot 1). Returns 0.
    pub fn scene_find_global_camera(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        h.write(ebp - 4, this);
        h.write(this + 0xdc, this + 0xe0);
        h.write(this + 0x238, this + 0x23c);
        h.write(this + 0x654, this + 0x658);
        let map: u32 = h.read(this + 0x10d4);
        if map == 0 {
            return 0;
        }
        crate::h3_glue14::bridge::find_named(h, ebp, 8, 4, map);
        let far: u32 = h.read(h.rebase(CAMERA_FAR));
        for camera_at in [0x238, 0x654] {
            for (field, value) in [(0xb8, 0x3f80_0000), (0xbc, far)] {
                let camera: u32 = h.read(this + camera_at);
                h.write(camera.wrapping_add(field), value);
                h.call_virtual(camera, 1, &[]);
            }
        }
        0
    }
}
