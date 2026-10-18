//! Swapping the international environment: the language files a language
//! change reloads (`TOSwapInternationalEnvExec*`) and the texture uploads
//! that follow (`TOTransTexture`).
//!
//! The reload reads, from the save region's archive (`0x00c59564`, 4 for
//! English and 0x12 otherwise, `TOSwapInternationalEnvInit`), file 0xb
//! into `[0x01841e00]`, 10 into the battle-menu texture, 0xf into
//! `[0x01841d9c]`, 0xe into `[0x01841d98]`, 8 into `[0x01841e08]`, 6 into
//! `0x025d0d20`, 5 into the Al Bhed table, 7 into `0x025d0dc0`, resets the
//! menu scenes, reads file 0x16 into scene 2's data (`0x01841e10`), and
//! reloads the battle data.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The non-blocking reload's step (`0x01841d28`, 0..=0x1c).
    pub const STEP: Global<u32> = Global::new(0x0184_1d28);
    /// Set once the reload is done (`0x01841d2c`); what every form returns.
    pub const DONE: Global<u32> = Global::new(0x0184_1d2c);
    /// Set while a reload runs (`0x01841d30`, `GetU32_01841d30`).
    pub const BUSY: Global<u32> = Global::new(0x0184_1d30);
    /// The battle-data file the non-blocking reload is at (`0x01841d34`).
    pub const BATTLE_FILE: Global<i32> = Global::new(0x0184_1d34);
    /// The menu reload's step (`0x01841d38`, 0..=6).
    pub const MENU_STEP: Global<u32> = Global::new(0x0184_1d38);
    /// Set once the menu reload is done (`0x01841d3c`).
    pub const MENU_DONE: Global<u32> = Global::new(0x0184_1d3c);
    /// The archive the language files come from (`0x00c59564`).
    pub const ARCHIVE: Global<u32> = Global::new(0x00c5_9564);
    /// The menu reload's archive (`0x00c59978`).
    pub const MENU_ARCHIVE: Global<u32> = Global::new(0x00c5_9978);
    /// `g_DebugPadPrintFlag`.
    pub const DEBUG_PRINT: Global<u8> = Global::new(0x0184_1d43);

    /// `dbgPrintf(format, ...)`.
    const DBG_PRINTF: u32 = 0x0062_f500;
    /// `GetCD()`: the CD library's table.
    const GET_CD: u32 = 0x0076_d0d0;
    /// `MsBattleReadData()` (battle module).
    const BATTLE_READ_DATA: u32 = 0x0078_2050;
    /// `MsBattleReadDataNB(n)` (battle module): non-zero while files remain.
    const BATTLE_READ_DATA_NB: u32 = 0x0078_2090;
    /// `TODapsSetPos(x, y)`.
    const DAPS_SET_POS: u32 = 0x0089_de30;
    /// `TODapsPrintf(format, ...)`.
    const DAPS_PRINTF: u32 = 0x0089_dde0;
    /// `TODVDFileRead(archive, file, buf)`.
    const FILE_READ: u32 = 0x008a_ba30;
    /// `TODVDFileStartRead(archive, file, buf)`.
    const FILE_START_READ: u32 = 0x008a_bab0;
    /// `0x008abb00()`: the CD table's `+8(1)`, non-zero while a read runs.
    const FILE_BUSY: u32 = 0x008a_bb00;
    /// `TOGetBtlMenuTexAdrs()`.
    const BTL_MENU_TEX: u32 = 0x008a_c2c0;
    /// `TOGetWord_01841e08()`.
    const WORD_1E08: u32 = 0x008a_c2d0;
    /// `TOGetFFXLang()`.
    const FFX_LANG: u32 = 0x008a_c2f0;
    /// `TOGetWord_01841d98()`.
    const WORD_1D98: u32 = 0x008a_c3f0;
    /// `TOGetInternationalBaseDicType()`.
    const BASE_DIC_TYPE: u32 = 0x008a_c760;
    /// `TOGetWord_01841e00()`.
    const WORD_1E00: u32 = 0x008a_c790;
    /// `TOGetWord_01841d9c()`.
    const WORD_1D9C: u32 = 0x008a_cf10;
    /// `GetU32_01841d30()`: [`BUSY`].
    const GET_BUSY: u32 = 0x008a_cf20;
    /// `TOSeqInitAllScene(scene)`.
    const SEQ_INIT_ALL_SCENE: u32 = 0x008a_e5a0;
    /// `TOSeqInitParam()`.
    const SEQ_INIT_PARAM: u32 = 0x008a_e6f0;
    /// `TOGetFFXToAlbhedTblAdrs()`.
    const ALBHED_TABLE: u32 = 0x0090_6b20;
    /// `TkMenuSetSceneDataAdrs(scene, data)` (menu module).
    const SET_SCENE_DATA: u32 = 0x00a7_c7f0;
    /// `TOTransImageHL(data, dest, format, x, y, w, h)`: one GS upload.
    const TRANS_IMAGE: u32 = 0x008b_0b10;
    /// `MsBattleGetStatus()` (battle module): non-zero in battle.
    const BATTLE_STATUS: u32 = 0x0078_16f0;
    /// `MsBattleGetFontAddress()` (battle module).
    const BATTLE_FONT: u32 = 0x0078_1680;
    /// `AtelGetExpandFTCAdrs()` (script module).
    const EXPAND_FTC: u32 = 0x0087_3ac0;
    /// `GetU32_0112a010()`.
    const GET_0112A010: u32 = 0x0077_4670;

    /// Scene 2's data (`0x01841e10`), file 0x16.
    const SCENE_DATA: u32 = 0x0184_1e10;
    /// File 6's buffer.
    const FILE6_BUF: u32 = 0x025d_0d20;
    /// File 7's buffer.
    const FILE7_BUF: u32 = 0x025d_0dc0;
    /// The menu reload's buffer, file 0xd.
    const MENU_BUF: u32 = 0x0157_f130;

    /// `"TOSwapInternationalEnvExec ---- Start\n"`.
    const START_TEXT: u32 = 0x00b6_2c40;
    /// `"TOSwapInternationalEnvExec ---- Done\n"`.
    const DONE_TEXT: u32 = 0x00b6_2c18;
    /// `"READ:%d\n"`.
    const READ_TEXT: u32 = 0x00b6_2c68;
    /// `"COUNT:%d\n"`.
    const COUNT_TEXT: u32 = 0x00b6_2c74;
    /// `"TOSwapInternationalEnvExecMenu ---- Start\n"`.
    const MENU_START_TEXT: u32 = 0x00b6_2c80;
    /// `"TOSwapInternationalEnvExecMenu ---- Done\n"`.
    const MENU_DONE_TEXT: u32 = 0x00b6_2cac;

    /// The language files of a reload before scene 2's, in order: file
    /// number and where it goes (a getter's result or a fixed buffer).
    enum Dest {
        Getter(u32),
        Buffer(u32),
    }

    /// The reload's files up to the scene reset: (file, destination).
    const FILES: [(u32, Dest); 8] = [
        (0xb, Dest::Getter(WORD_1E00)),
        (0xa, Dest::Getter(BTL_MENU_TEX)),
        (0xf, Dest::Getter(WORD_1D9C)),
        (0xe, Dest::Getter(WORD_1D98)),
        (0x8, Dest::Getter(WORD_1E08)),
        (0x6, Dest::Buffer(FILE6_BUF)),
        (0x5, Dest::Getter(ALBHED_TABLE)),
        (0x7, Dest::Buffer(FILE7_BUF)),
    ];

    /// Where file `k` of [`FILES`] goes.
    fn dest<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        k: usize,
    ) -> (u32, u32) {
        let (file, ref to) = FILES[k];
        let buf = match *to {
            Dest::Getter(f) => {
                let f = mem.rebase(f);
                call(mem, f, &[])
            }
            Dest::Buffer(b) => mem.rebase(b),
        };
        (file, buf)
    }

    /// Calls the original `sym`.
    fn sym<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        sym: u32,
        args: &[u32],
    ) -> u32 {
        let f = mem.rebase(sym);
        call(mem, f, args)
    }

    /// `TOSwapInternationalEnvExec()` (`0x008b0340`): the whole reload at
    /// once with `TODVDFileRead`: [`BUSY`] set, the [`FILES`], scene reset
    /// (`TOSeqInitAllScene(2)`), file 0x16 into scene 2's data,
    /// `TkMenuSetSceneDataAdrs(2, data)`, `TOSeqInitParam()`,
    /// `MsBattleReadData()`; then [`DONE`] = 1, [`BUSY`] = 0, the "Done"
    /// line; returns [`DONE`]. `call` takes absolute addresses.
    pub fn exec<M: Mem>(mem: &mut M, mut call: impl FnMut(&mut M, u32, &[u32]) -> u32) -> u32 {
        BUSY.set(mem, 1);
        for k in 0..FILES.len() {
            let (file, buf) = dest(mem, &mut call, k);
            let archive = ARCHIVE.get(mem);
            sym(mem, &mut call, FILE_READ, &[archive, file, buf]);
        }
        sym(mem, &mut call, SEQ_INIT_ALL_SCENE, &[2]);
        let (archive, data) = (ARCHIVE.get(mem), mem.rebase(SCENE_DATA));
        sym(mem, &mut call, FILE_READ, &[archive, 0x16, data]);
        sym(mem, &mut call, SET_SCENE_DATA, &[2, data]);
        sym(mem, &mut call, SEQ_INIT_PARAM, &[]);
        sym(mem, &mut call, BATTLE_READ_DATA, &[]);
        DONE.set(mem, 1);
        BUSY.set(mem, 0);
        let text = mem.rebase(DONE_TEXT);
        sym(mem, &mut call, DBG_PRINTF, &[text]);
        DONE.get(mem)
    }

    /// `TOSwapInternationalEnvExecMenu()` (`0x008b0440`): one step of the
    /// menu's reload, file 0xd of [`MENU_ARCHIVE`] into `0x0157f130`:
    /// step 0 prints "Start", 1 waits a frame, 2 starts the read, 3 waits
    /// for it (then step 6), 6 sets [`MENU_DONE`] and prints "Done"; steps
    /// 4, 5 and above 6 do nothing. Returns [`MENU_DONE`].
    pub fn exec_menu<M: Mem>(mem: &mut M, mut call: impl FnMut(&mut M, u32, &[u32]) -> u32) -> u32 {
        match MENU_STEP.get(mem) {
            0 => {
                let text = mem.rebase(MENU_START_TEXT);
                sym(mem, &mut call, DBG_PRINTF, &[text]);
                MENU_STEP.set(mem, 1);
            }
            1 => MENU_STEP.set(mem, 2),
            2 => {
                let (archive, buf) = (MENU_ARCHIVE.get(mem), mem.rebase(MENU_BUF));
                sym(mem, &mut call, FILE_START_READ, &[archive, 0xd, buf]);
                MENU_STEP.set(mem, 3);
            }
            3 => {
                if sym(mem, &mut call, FILE_BUSY, &[]) == 0 {
                    MENU_STEP.set(mem, 6);
                }
            }
            6 => {
                MENU_DONE.set(mem, 1);
                let text = mem.rebase(MENU_DONE_TEXT);
                sym(mem, &mut call, DBG_PRINTF, &[text]);
            }
            _ => {}
        }
        MENU_DONE.get(mem)
    }

    /// `TOSwapInternationalEnvExecNonBlock()` (`0x008b0500`): one step of
    /// [`exec`]'s reload, spread over frames by [`STEP`]. Step 0 prints
    /// "Start" and sets [`BUSY`]; 1 waits; then each of the [`FILES`] takes
    /// two steps (start the read, wait until `0x008abb00` is 0); 0x12
    /// resets the scenes and goes straight on to 0x13, the read of file
    /// 0x16 (0x14 waits); 0x15 registers scene 2's data and runs
    /// `TOSeqInitParam`; 0x16 starts the battle files at 0; 0x17 reads
    /// battle file [`BATTLE_FILE`] (`MsBattleReadDataNB`, then 0x18 while
    /// it returns non-zero, else 0x1a); 0x18 waits for the CD (`GetCD()`'s
    /// `+8(1)` is 0), 0x19 steps to the next battle file; 0x1a and 0x1b
    /// wait; 0x1c sets [`DONE`], clears [`BUSY`] and prints "Done" (every
    /// time it runs). Steps above 0x1c do nothing. With the debug print
    /// flag the step and battle file go to the debug screen. Returns
    /// [`DONE`].
    pub fn exec_non_block<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let step = STEP.get(mem);
        match step {
            0 => {
                let text = mem.rebase(START_TEXT);
                sym(mem, &mut call, DBG_PRINTF, &[text]);
                BUSY.set(mem, 1);
                STEP.set(mem, 1);
            }
            1 => STEP.set(mem, 2),
            // a file read: even steps start it, odd ones wait for it
            2..=0x11 => {
                let k = (step - 2) as usize / 2;
                if step.is_multiple_of(2) {
                    let (file, buf) = dest(mem, &mut call, k);
                    let archive = ARCHIVE.get(mem);
                    sym(mem, &mut call, FILE_START_READ, &[archive, file, buf]);
                    STEP.set(mem, step + 1);
                } else if sym(mem, &mut call, FILE_BUSY, &[]) == 0 {
                    STEP.set(mem, step + 1);
                }
            }
            0x12 | 0x13 => {
                if step == 0x12 {
                    sym(mem, &mut call, SEQ_INIT_ALL_SCENE, &[2]);
                    STEP.set(mem, 0x13);
                }
                let (archive, data) = (ARCHIVE.get(mem), mem.rebase(SCENE_DATA));
                sym(mem, &mut call, FILE_START_READ, &[archive, 0x16, data]);
                STEP.set(mem, 0x14);
            }
            0x14 => {
                if sym(mem, &mut call, FILE_BUSY, &[]) == 0 {
                    STEP.set(mem, 0x15);
                }
            }
            0x15 => {
                let data = mem.rebase(SCENE_DATA);
                sym(mem, &mut call, SET_SCENE_DATA, &[2, data]);
                sym(mem, &mut call, SEQ_INIT_PARAM, &[]);
                STEP.set(mem, 0x16);
            }
            0x16 => {
                BATTLE_FILE.set(mem, 0);
                STEP.set(mem, 0x17);
            }
            0x17 => {
                let n = BATTLE_FILE.get(mem).cast_unsigned();
                let more = sym(mem, &mut call, BATTLE_READ_DATA_NB, &[n]) != 0;
                STEP.set(mem, if more { 0x18 } else { 0x1a });
            }
            0x18 => {
                let cd = sym(mem, &mut call, GET_CD, &[]);
                let busy: u32 = mem.read(cd.wrapping_add(8));
                if call(mem, busy, &[1]) == 0 {
                    STEP.set(mem, 0x19);
                }
            }
            0x19 => {
                let n = BATTLE_FILE.get(mem);
                BATTLE_FILE.set(mem, n.wrapping_add(1));
                STEP.set(mem, 0x17);
            }
            0x1a => STEP.set(mem, 0x1b),
            0x1b => STEP.set(mem, 0x1c),
            0x1c => {
                DONE.set(mem, 1);
                BUSY.set(mem, 0);
                let text = mem.rebase(DONE_TEXT);
                sym(mem, &mut call, DBG_PRINTF, &[text]);
            }
            _ => {}
        }
        if DEBUG_PRINT.get(mem) != 0 {
            sym(mem, &mut call, DAPS_SET_POS, &[0x1e, 0x1e]);
            let (text, step) = (mem.rebase(READ_TEXT), STEP.get(mem));
            sym(mem, &mut call, DAPS_PRINTF, &[text, step]);
            let (text, n) = (mem.rebase(COUNT_TEXT), BATTLE_FILE.get(mem).cast_unsigned());
            sym(mem, &mut call, DAPS_PRINTF, &[text, n]);
        }
        DONE.get(mem)
    }

    /// The loaded textures (`0x01841da0`, 16-byte entries: image data,
    /// CLUT, size, `i16` width at `+0xc`, height at `+0xe`).
    pub const TEXTURES: u32 = 0x0184_1da0;

    /// `TOTransImageHL(data, dest, format, 0, 0, w, h)`; its `eax`.
    fn upload<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        data: u32,
        dest: u32,
        format: u32,
        (w, h): (u32, u32),
    ) -> u32 {
        sym(mem, call, TRANS_IMAGE, &[data, dest, format, 0, 0, w, h])
    }

    /// Texture `k` of [`TEXTURES`] to `dest` (format 0x14), at its own size.
    fn upload_texture<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        k: u32,
        dest: u32,
    ) -> u32 {
        let at = mem.rebase(TEXTURES).wrapping_add(k.wrapping_mul(0x10));
        let data: u32 = mem.read(at);
        let w = i32::from(mem.read::<i16>(at.wrapping_add(0xc))).cast_unsigned();
        let h = i32::from(mem.read::<i16>(at.wrapping_add(0xe))).cast_unsigned();
        upload(mem, call, data, dest, 0x14, (w, h))
    }

    /// The word at `sym` in the image.
    fn word(mem: &impl Mem, sym: u32) -> u32 {
        mem.read(mem.rebase(sym))
    }

    /// `TOTransTexture()` (`0x008b1020`): the language textures to the GS:
    /// the base dictionary's texture (`TOGetInternationalBaseDicType`) to
    /// 0x3c00; outside battle texture 1 to 0x3d00 when
    /// `AtelGetExpandFTCAdrs` is non-zero; in battle texture 2 to 0x3d00
    /// when `MsBattleGetFontAddress` is non-zero, and texture 4 to 0x3d40
    /// for language 0; then `[0x01841d94]` (0x80 x 0x100) to 0x3d80,
    /// `[0x01841d98]` (0x100 x 0x80) to 0x3dc0, `[0x01841e00]` and the
    /// battle-menu texture (0x100 x 0x100, format 0x13) to 0x3e00 and
    /// 0x3f00, `[0x01841e08]` (0x40 x 0x40, format 0) to 0x2ea0. Returns
    /// the last upload's `eax`.
    pub fn trans_texture<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let dic = sym(mem, &mut call, BASE_DIC_TYPE, &[]);
        upload_texture(mem, &mut call, dic, 0x3c00);
        if sym(mem, &mut call, BATTLE_STATUS, &[]) == 0 {
            if sym(mem, &mut call, EXPAND_FTC, &[]) != 0 {
                upload_texture(mem, &mut call, 1, 0x3d00);
            }
        } else {
            if sym(mem, &mut call, BATTLE_FONT, &[]) != 0 {
                upload_texture(mem, &mut call, 2, 0x3d00);
            }
            if sym(mem, &mut call, FFX_LANG, &[]) == 0 {
                upload_texture(mem, &mut call, 4, 0x3d40);
            }
        }
        let data = word(mem, 0x0184_1d94);
        upload(mem, &mut call, data, 0x3d80, 0x14, (0x80, 0x100));
        let data = word(mem, 0x0184_1d98);
        upload(mem, &mut call, data, 0x3dc0, 0x14, (0x100, 0x80));
        let data = word(mem, 0x0184_1e00);
        upload(mem, &mut call, data, 0x3e00, 0x13, (0x100, 0x100));
        let data = sym(mem, &mut call, BTL_MENU_TEX, &[]);
        upload(mem, &mut call, data, 0x3f00, 0x13, (0x100, 0x100));
        let data = word(mem, 0x0184_1e08);
        upload(mem, &mut call, data, 0x2ea0, 0, (0x40, 0x40))
    }

    /// `0x008b1190()`: the menu's form of [`trans_texture`]: for language 0
    /// textures 0 to 0x3c00 and 4 to 0x3d40, otherwise the base
    /// dictionary's texture to 0x3c00 (the type is asked first either
    /// way); texture 3 to 0x3d00 when `GetU32_0112a010` is non-zero;
    /// `[0x01841d94]` to 0x3d80, `[0x01841d98]` to 0x3dc0, `[0x01841d9c]`
    /// (0x200 x 0x80) to 0x3f00; and while no reload runs ([`BUSY`] 0)
    /// `[0x01841e00]` to 0x3e00 and `[0x01841e08]` to 0x2ea0. Returns the
    /// last upload's `eax`, or [`BUSY`] when it stops there.
    pub fn trans_texture_menu<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let dic = sym(mem, &mut call, BASE_DIC_TYPE, &[]);
        if sym(mem, &mut call, FFX_LANG, &[]) == 0 {
            upload_texture(mem, &mut call, 0, 0x3c00);
            upload_texture(mem, &mut call, 4, 0x3d40);
        } else {
            upload_texture(mem, &mut call, dic, 0x3c00);
        }
        if sym(mem, &mut call, GET_0112A010, &[]) != 0 {
            upload_texture(mem, &mut call, 3, 0x3d00);
        }
        let data = word(mem, 0x0184_1d94);
        upload(mem, &mut call, data, 0x3d80, 0x14, (0x80, 0x100));
        let data = word(mem, 0x0184_1d98);
        upload(mem, &mut call, data, 0x3dc0, 0x14, (0x100, 0x80));
        let data = word(mem, 0x0184_1d9c);
        upload(mem, &mut call, data, 0x3f00, 0x14, (0x200, 0x80));
        let busy = sym(mem, &mut call, GET_BUSY, &[]);
        if busy != 0 {
            return busy;
        }
        let data = word(mem, 0x0184_1e00);
        upload(mem, &mut call, data, 0x3e00, 0x13, (0x100, 0x100));
        let data = word(mem, 0x0184_1e08);
        upload(mem, &mut call, data, 0x2ea0, 0, (0x40, 0x40))
    }

    /// `TOMenuTransFacePlyTex()` (`0x00901100`): the menu's face texture
    /// (0x200 x 0x100, format 0x13) to 0x3a00; the upload's `eax`. The
    /// original pushes `0x01c80000` with no relocation entry, so the
    /// address stays literal even in a relocated image (the game runs at
    /// its preferred base, where both agree).
    pub fn trans_face_ply_tex<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let data = 0x01c8_0000;
        upload(mem, &mut call, data, 0x3a00, 0x13, (0x200, 0x100))
    }
}
