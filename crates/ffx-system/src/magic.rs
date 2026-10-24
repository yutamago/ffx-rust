//! The magic files' preload table (`MagicFile_PreloadPrx_009dae90`): which
//! effect PRX files (magic `*.dll` on PC) to read ahead for a magic and
//! file pair. The loads themselves are `PhyreFIOS`'s, called through the
//! game.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// Preloading is on (`u32 0x0193fd40`).
    pub const PRELOAD_ON: u32 = 0x0193_fd40;
    /// The preload slots' handles (`0x0193fd44`, 12 words), their FIOS
    /// operations (`0x0193fd78`, 12 words) and paths (`0x0193fdb0`, 12 x
    /// 0x100 bytes).
    pub const PRELOAD_HANDLES: u32 = 0x0193_fd44;
    pub const PRELOAD_OPS: u32 = 0x0193_fd78;
    pub const PRELOAD_PATHS: u32 = 0x0193_fdb0;
    /// The preload table (`0x00c64c10`): 12 entries of (magic, file,
    /// handle id) words; a negative magic or file matches any.
    pub const PRELOAD_TABLE: u32 = 0x00c6_4c10;
    /// The slots the table has.
    pub const PRELOAD_SLOTS: u32 = 12;
    /// `graphicSetMagicFileAsyncLoadEnable`, `PhyreFIOS_UnloadMagicfilePrx`,
    /// `PhyreFIOS_LoadMagicfilePrx` (`ffx.sym` addresses).
    pub const ASYNC_LOAD_ENABLE: u32 = 0x0064_35e0;
    pub const UNLOAD_PRX: u32 = 0x0067_94f0;
    pub const LOAD_PRX: u32 = 0x0067_93e0;
    /// The C runtime imports the original calls through: `__iob_func`
    /// (`0x00b0c350`) and `fprintf` (`0x00b0c38c`).
    pub const IOB_FUNC: u32 = 0x00b0_c350;
    pub const FPRINTF: u32 = 0x00b0_c38c;
    /// Its three messages: "non-null preload handle found" (`stderr`),
    /// "preload for id=%d" (`stdout`), "skiped for id=%d" (`stderr`).
    pub const MSG_NON_NULL: u32 = 0x00b7_0780;
    pub const MSG_PRELOAD: u32 = 0x00b7_07c4;
    pub const MSG_SKIPPED: u32 = 0x00b7_0800;

    /// `MagicFile_PreloadPrx_009dae90(magic, file)` (`0x009dae90`; caller
    /// `MagicFile_Load`, which drops `EAX`): with [`PRELOAD_ON`],
    /// `graphicSetMagicFileAsyncLoadEnable(2)`, then for each
    /// [`PRELOAD_TABLE`] entry `i` whose magic is `magic` (or either is
    /// negative), whose file is `file` (or negative) and whose handle id
    /// `n` is positive: a slot holding another file's handle (`n != file`)
    /// is unloaded (`PhyreFIOS_UnloadMagicfilePrx(handle, &op)` after the
    /// "non-null" message, only for a non-negative handle) and cleared
    /// with its operation; an empty slot is loaded
    /// (`PhyreFIOS_LoadMagicfilePrx(path_i, &handle_i, &op_i)`, then the
    /// "preload" message with `n, i, op_i`), a full one reported ("skiped"
    /// with `file, i, n`). `call` gets the rebased function and its words:
    /// the messages go to `fprintf(__iob_func() + 0x40 or + 0x20, ..)`
    /// through the C runtime's import slots.
    pub fn magic_file_preload_prx<M: Mem>(
        mem: &mut M,
        magic: u32,
        file: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if mem.read::<u32>(mem.rebase(PRELOAD_ON)) == 0 {
            return;
        }
        call(mem, mem.rebase(ASYNC_LOAD_ENABLE), &[2]);
        let print = |mem: &mut M,
                     call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
                     out: u32,
                     words: &[u32]| {
            let iob = mem.read::<u32>(mem.rebase(IOB_FUNC));
            let stream = call(mem, iob, &[]).wrapping_add(out);
            let fprintf = mem.read::<u32>(mem.rebase(FPRINTF));
            let mut args = vec![stream];
            args.extend_from_slice(words);
            call(mem, fprintf, &args);
        };
        let table = mem.rebase(PRELOAD_TABLE);
        let handles = mem.rebase(PRELOAD_HANDLES);
        let ops = mem.rebase(PRELOAD_OPS);
        let paths = mem.rebase(PRELOAD_PATHS);
        for i in 0..PRELOAD_SLOTS {
            let e = table + i * 0xc;
            let e_magic = mem.read::<i32>(e);
            let e_file = mem.read::<i32>(e + 4);
            let n = mem.read::<i32>(e + 8);
            let magic_ok = e_magic == magic.cast_signed() || e_magic < 0 || magic.cast_signed() < 0;
            let file_ok = e_file == file.cast_signed() || e_file < 0;
            if !(magic_ok && file_ok && n > 0) {
                continue;
            }
            let handle_at = handles + i * 4;
            let op_at = ops + i * 4;
            let handle = mem.read::<i32>(handle_at);
            if handle != 0 && n != file.cast_signed() {
                if handle >= 0 {
                    let msg = mem.rebase(MSG_NON_NULL);
                    print(mem, call, 0x40, &[msg, file, i]);
                    let handle = mem.read::<u32>(handle_at);
                    call(mem, mem.rebase(UNLOAD_PRX), &[handle, op_at]);
                }
                mem.write(handle_at, 0u32);
                mem.write(op_at, 0u32);
            }
            if mem.read::<u32>(handle_at) == 0 {
                call(
                    mem,
                    mem.rebase(LOAD_PRX),
                    &[paths + i * 0x100, handle_at, op_at],
                );
                let op = mem.read::<u32>(op_at);
                let msg = mem.rebase(MSG_PRELOAD);
                print(mem, call, 0x20, &[msg, n.cast_unsigned(), i, op]);
            } else {
                let msg = mem.rebase(MSG_SKIPPED);
                print(mem, call, 0x40, &[msg, file, i, n.cast_unsigned()]);
            }
        }
    }

    /// `g_MagicFile_MagicId`, `g_MagicFile_PreviousId`, `g_MagicFile_Handle`,
    /// the previous magic's handle (`g_BtlCommandWait`) and the list of
    /// magic ids that must never load (`0x00c64cb4`, ended by a negative
    /// word: 0, 693..=696, 44, 712, 713, 725).
    pub const MAGIC_ID: u32 = 0x00c6_4ca0;
    pub const PREVIOUS_ID: u32 = 0x00c6_4ca8;
    pub const HANDLE: u32 = 0x00c6_4cac;
    pub const PREVIOUS_HANDLE: u32 = 0x00c6_4cb0;
    pub const FORBIDDEN: u32 = 0x00c6_4cb4;
    /// The C runtime's `printf` import slot.
    pub const PRINTF: u32 = 0x00b0_c380;

    /// `MagicFile_Load()` (`0x009da430`; callers `MsEffectRead`,
    /// `MsEventEffectRead`, which read `EAX`, and two tail jumps after
    /// `MagicFile_SetAsyncLoad`): loads the magic overlay
    /// `g_MagicFile_MagicId`. Clears `0x01940ab4` and
    /// `g_MagicFile_IsNexData`; with no id (-1) or a handle already
    /// loaded it prints why and returns 0. Magic 0x92 zeroes
    /// `g_CamFinishCount`, any other sets it to -1 (unless it is). The
    /// magic waiting for deletion (`g_MagicFile_PreviousId`) is taken back
    /// instead: the previous id and handle become current (both previous
    /// words -1), `graphicMagicSetPath(graphicMagicGetToBeDeletePath())`,
    /// `graphicMagicSetToBeDeletePath("")`, and `0x01940ab4` = `handle >=
    /// 0`, which it returns. Otherwise an id in [`FORBIDDEN`] prints the
    /// "unknown magic file" warning and writes a byte at address 0 (the
    /// original's deliberate crash, kept); then
    /// `MagicFile_PreloadPrx(graphicLevelGetNameHash(id), id)` (the id
    /// word is still on the stack), the path
    /// `"magicFiles\FFX\magic_%04d.dll"` into `path` (256 bytes, the
    /// original's frame), `MagicFile_SetAsyncLoad`, "Load Magic",
    /// `MagicFile_Unload` when both previous words are set,
    /// `graphicMagicVFXLoadSpecialShaderTable(id)`, `0x01940ab0` = 0 and
    /// `PhyreFIOS_LoadMagicfilePrx(path, 0, onMagicDLLLoaded)`: 0 prints
    /// the failure, sets the handle -1 and returns 0. Loaded:
    /// `graphicMagicSetPath("magic_%04d")` (in `path`), the textures
    /// (`graphicMagicVFXLoadTexture(id)`) unless
    /// `graphicIsMagicFileAsyncLoadAvailable`, "completed", magic 0x1ae and
    /// 600 set `g_MagicHide3017`, the camera and screen matrices are
    /// copied to `0x022fb660` and `0x022fb620`, `f32(graphicCameraGetFOVy())`
    /// to `0x022fb600`, magic 0x275 sets `g_MagicMatrixSwap` = 0xf; the
    /// result is 1. The id is read again after each call, as the original
    /// does.
    /// `call` gets absolute addresses; `fcall` returns `ST0`.
    #[allow(clippy::too_many_lines)] // the load in turn
    pub fn magic_file_load<M: Mem>(
        mem: &mut M,
        path: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let id = |mem: &M| mem.read::<u32>(mem.rebase(MAGIC_ID));
        let printf =
            |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, words: &[u32]| {
                let f = mem.read::<u32>(mem.rebase(PRINTF));
                call(mem, f, words);
            };
        let sprintf_id = |mem: &mut M, text: &str| {
            let mut bytes = text.as_bytes().to_vec();
            bytes.push(0);
            mem.write_bytes(path, &bytes);
        };
        mem.write(r(mem, 0x0194_0ab4), 0_u32);
        mem.write(r(mem, 0x0193_fdac), 0_u32);
        let magic = id(mem);
        if magic == u32::MAX {
            let msg = r(mem, 0x00b7_0440);
            printf(mem, call, &[msg]);
            return 0;
        }
        let cam = r(mem, 0x00c4_94c8);
        if magic == 0x92 {
            mem.write(cam, 0_u32);
        } else if mem.read::<u32>(cam) != u32::MAX {
            mem.write(cam, u32::MAX);
        }
        let handle_at = r(mem, HANDLE);
        if mem.read::<u32>(handle_at) != u32::MAX {
            let msg = r(mem, 0x00b7_0478);
            printf(mem, call, &[msg]);
            return 0;
        }
        let prev_at = r(mem, PREVIOUS_ID);
        let prev_handle_at = r(mem, PREVIOUS_HANDLE);
        if magic == mem.read::<u32>(prev_at) {
            let msg = r(mem, 0x00b7_04b8);
            printf(mem, call, &[msg]);
            let prev = mem.read::<u32>(prev_at);
            mem.write(r(mem, MAGIC_ID), prev);
            let h = mem.read::<u32>(prev_handle_at);
            mem.write(prev_at, u32::MAX);
            mem.write(handle_at, h);
            mem.write(prev_handle_at, u32::MAX);
            let old = call(mem, r(mem, 0x0064_2040), &[]);
            call(mem, r(mem, 0x0064_2050), &[old]);
            let empty = r(mem, 0x00b8_fbeb);
            call(mem, r(mem, 0x0064_2070), &[empty]);
            let loaded = u32::from(mem.read::<i32>(handle_at) >= 0);
            mem.write(r(mem, 0x0194_0ab4), loaded);
            return loaded;
        }
        let mut magic = magic;
        let list = r(mem, FORBIDDEN);
        let mut i = 0u32;
        while mem.read::<i32>(list.wrapping_add(i * 4)) >= 0 {
            if magic == mem.read::<u32>(list.wrapping_add(i * 4)) {
                let msg = r(mem, 0x00b7_0530);
                printf(mem, call, &[msg, magic]);
                // the original's deliberate fault: mov byte [0], 0
                mem.write(0, 0_u8);
                magic = id(mem);
            }
            i += 1;
        }
        let hash = call(mem, r(mem, 0x0064_1da0), &[magic]);
        call(mem, r(mem, 0x009d_ae90), &[hash, magic]);
        let n = id(mem).cast_signed();
        sprintf_id(mem, &format!("magicFiles\\FFX\\magic_{n:04}.dll"));
        call(mem, r(mem, 0x009d_afe0), &[]);
        let msg = r(mem, 0x00b7_0570);
        printf(mem, call, &[msg, path]);
        if mem.read::<u32>(prev_handle_at) != u32::MAX && mem.read::<u32>(prev_at) != u32::MAX {
            call(mem, r(mem, 0x009d_a960), &[]);
        }
        let magic = id(mem);
        call(mem, r(mem, 0x0064_2090), &[magic]);
        mem.write(r(mem, 0x0194_0ab0), 0_u32);
        let done = r(mem, 0x009d_b110);
        if call(mem, r(mem, 0x0067_93e0), &[path, 0, done]) == 0 {
            let msg = r(mem, 0x00b7_0580);
            printf(mem, call, &[msg]);
            mem.write(handle_at, u32::MAX);
            return 0;
        }
        let n = id(mem).cast_signed();
        sprintf_id(mem, &format!("magic_{n:04}"));
        call(mem, r(mem, 0x0064_2050), &[path]);
        if call(mem, r(mem, 0x0064_1c60), &[]) == 0 {
            let magic = id(mem);
            call(mem, r(mem, 0x0064_20b0), &[magic]);
        }
        let magic = id(mem);
        let msg = r(mem, 0x00b7_059c);
        printf(mem, call, &[msg, magic]);
        let magic = id(mem);
        if magic == 0x1ae || magic == 600 {
            mem.write(r(mem, 0x0194_0ab8), 1_u32);
        }
        for (from, to) in [(0x0231_1440, 0x022f_b660), (0x0231_1480, 0x022f_b620)] {
            let m = mem.read::<[u32; 16]>(r(mem, from));
            mem.write(r(mem, to), m);
        }
        let fov = fcall(mem, r(mem, 0x0063_9570), &[]);
        let magic = id(mem);
        mem.write(r(mem, 0x022f_b600), ffx_bridge::x87::store(fov));
        if magic == 0x275 {
            mem.write(r(mem, 0x0194_0ac0), 0xf_u32);
        }
        1
    }
}
