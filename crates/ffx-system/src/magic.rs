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
}
