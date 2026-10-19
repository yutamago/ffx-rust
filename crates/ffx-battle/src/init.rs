//! The battle module's start-up (`docs/rewrite-plan.md`, M2): leaves of
//! `MsBattleInit` (`0x00781700`), which runs once when the game starts and
//! reads the battle's kernel tables, texts and names.
//!
//! The CD object, the heap (`user_malloc`, `Sg_MemSetAlign`), the kernel
//! file reader `getExcelBinSize`, the 32-bit pointer table
//! (`is_ptr32_null`, `set_ptr32`, `get_ptr32_func`), map's functions and
//! `MsBattleError` (it hangs) go to the game's code through `call`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::exec::bridge::Call;

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        call(mem, f, args)
    }

    /// The CD object's method at `slot` (`GetCD()`'s table) with `args`.
    fn cd<M: Mem>(mem: &mut M, call: Call<'_, M>, slot: u32, args: &[u32]) -> u32 {
        let obj = game(mem, &mut *call, crate::getmem::bridge::GET_CD, &[]);
        let f: u32 = mem.read(obj.wrapping_add(slot));
        call(mem, f, args)
    }

    /// `user_malloc(n)`, `user_free(p)` (the heap), `Sg_MemSetAlign(n)`
    /// (main), `getExcelBinSize(name, buffer)` (the kernel file reader:
    /// the size), `MsBattleError(msg)` (it hangs).
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const SG_MEM_SET_ALIGN: u32 = 0x0083_c4a0;
    pub const GET_EXCEL_BIN_SIZE: u32 = 0x0078_2df0;
    pub const MS_BATTLE_ERROR: u32 = 0x0079_f210;
    /// The 32-bit pointer table's `is_ptr32_null(h)`, `set_ptr32(p)` (a
    /// handle) and `get_ptr32_func(h)` (the pointer).
    pub const IS_PTR32_NULL: u32 = 0x00a4_46a0;
    pub const SET_PTR32: u32 = 0x00a4_4700;
    pub const GET_PTR32_FUNC: u32 = 0x00a4_4650;
    /// `Yn_PSimTest()`, `Yn_SeaCreate()` and `yiDisposeAnimDObj(i, n)`
    /// (map).
    pub const YN_PSIM_TEST: u32 = 0x0091_9f30;
    pub const YN_SEA_CREATE: u32 = 0x0091_ade0;
    pub const YI_DISPOSE_ANIM_D_OBJ: u32 = 0x0090_84b0;

    /// `TOGetFFXLang()` itself (system; [`TO_GET_FFX_LANG`] is its jump
    /// stub), `sceOpen(path, mode)`,
    /// `sceLseek(fd, offset, whence)`, `sceRead(fd, buf, n)`,
    /// `sceClose(fd)` (the file layer), `TkMalloc(n)`, `TkFree(p)` (the
    /// menu's heap).
    pub const TO_GET_FFX_LANG_FN: u32 = 0x0064_10e0;
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_LSEEK: u32 = 0x0062_f8e0;
    pub const SCE_READ: u32 = 0x0062_fc00;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const TK_MALLOC: u32 = 0x0088_e650;
    pub const TK_FREE: u32 = 0x0088_d520;
    /// "WJ: there is no DE message.bin [%s]\n", `getExcelBinSize`'s
    /// message for a file that does not open.
    pub const NO_KERNEL_FILE: u32 = 0x00b4_1e3c;
    /// `getExcelBinSize`'s path buffer (a stack array in the original).
    pub const KERNEL_PATH_SIZE: u32 = 0x80;

    /// The language directory `getExcelBinSize` reads the kernel from, by
    /// `TOGetFFXLang` (unsigned; English for the others).
    #[must_use]
    pub fn kernel_lang_dir(lang: u32) -> &'static str {
        match lang {
            0 => "jp",
            2 => "fr",
            3 => "sp",
            4 => "de",
            5 => "it",
            9 => "kr",
            10 => "ch",
            _ => "us",
        }
    }

    /// `getExcelBinSize(name, buffer)` (`0x00782df0`): the kernel file
    /// `host0:/ffx/master/new_<lang>pc/battle/kernel/<name>.bin` (built in
    /// `path`, [`KERNEL_PATH_SIZE`] bytes; the original's `sprintf` into a
    /// stack array of that size; a longer path is cut here, where the
    /// original would overrun its stack) read whole: its size rounded up
    /// to 16 (`sceLseek` to the end, signed), a `TkMalloc` block of it
    /// (first word cleared) read and copied into `buffer`, then freed.
    /// Returns the rounded size; 0 (after `dbgPrintf`) when the file does
    /// not open. Ghidra calls it void; its callers read the size.
    pub fn get_excel_bin_size<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        name: u32,
        buffer: u32,
        path: u32,
    ) -> u32 {
        let lang = game(mem, &mut *call, TO_GET_FFX_LANG_FN, &[]);
        let mut text = format!(
            "host0:/ffx/master/new_{}pc/battle/kernel/",
            kernel_lang_dir(lang)
        )
        .into_bytes();
        let mut at = name;
        loop {
            let c: u8 = mem.read(at);
            if c == 0 || text.len() >= KERNEL_PATH_SIZE as usize {
                break;
            }
            text.push(c);
            at = at.wrapping_add(1);
        }
        text.extend_from_slice(b".bin");
        text.truncate(KERNEL_PATH_SIZE as usize - 1);
        text.push(0);
        mem.write_bytes(path, &text);
        let fd = game(mem, &mut *call, SCE_OPEN, &[path, 1]);
        if fd == 0 {
            let fmt = code(mem, NO_KERNEL_FILE);
            game(mem, call, DBG_PRINTF, &[fmt, path]);
            return 0;
        }
        let end = game(mem, &mut *call, SCE_LSEEK, &[fd, 0, 2]).cast_signed();
        let t = end.wrapping_add(0xf);
        let size = (t.wrapping_add((t >> 31) & 0xf) & !0xf).cast_unsigned();
        game(mem, &mut *call, SCE_LSEEK, &[fd, 0, 0]);
        let block = game(mem, &mut *call, TK_MALLOC, &[size]);
        mem.write(block, 0u32);
        game(mem, &mut *call, SCE_READ, &[fd, block, size]);
        game(mem, &mut *call, SCE_CLOSE, &[fd]);
        copy(mem, buffer, block, size);
        game(mem, call, TK_FREE, &[block]);
        size
    }

    /// A kernel file's read buffer (`user_malloc`, freed after the copy).
    pub const READ_BUFFER: u32 = 0x25800;

    /// `memcpy(to, from, n)` in pieces (`n` comes from a file).
    fn copy(mem: &mut impl Mem, to: u32, from: u32, n: u32) {
        let mut done = 0u32;
        let mut piece = vec![0u8; 0x1000];
        while done < n {
            let k = (n - done).min(0x1000);
            let part = &mut piece[..k as usize];
            mem.read_bytes(from.wrapping_add(done), part);
            mem.write_bytes(to.wrapping_add(done), part);
            done += k;
        }
    }

    /// `"eff_id"` and the message `MsMemCheckSum` reports.
    pub const EFF_ID_ERROR: u32 = 0x00b5_4d18;
    pub const MEM_CHECK_ERROR: u32 = 0x00b5_4c90;

    /// `MsMemCheckSum(top)` (`0x00799f20`): the room left below `top` past
    /// the booked work memory (`g_BtlMemCheck` and 0x10), rounded down to
    /// 64 bytes, goes to a block of its own (`user_malloc`,
    /// `g_BtlMemCheck2`); booked past `top` is an error (`MsBattleError`).
    /// EAX as the original leaves it: the room when there is none, else the
    /// block (or the error's).
    pub fn ms_mem_check_sum<M: Mem>(mem: &mut M, call: Call<'_, M>, top: i32) -> u32 {
        use crate::state::bridge::{MEM_CHECK, MEM_CHECK2};
        let room = top.wrapping_sub(MEM_CHECK.get(mem)).wrapping_sub(0x10);
        let mut eax = room.cast_unsigned();
        if room > 0 {
            let n = (room & !0x3f).cast_unsigned();
            eax = game(mem, &mut *call, USER_MALLOC, &[n]);
            MEM_CHECK2.set(mem, eax.cast_signed());
        }
        if MEM_CHECK.get(mem) > top {
            let msg = code(mem, MEM_CHECK_ERROR);
            eax = game(mem, call, MS_BATTLE_ERROR, &[msg]);
        }
        eax
    }

    /// `MsBtlGetClear_007993e0(p, kind)`: clears the 0x1b0-byte record at
    /// `p` and numbers its eight halfwords from `+0xee` `kind << 12` + i.
    /// EAX 8 (the loop's count).
    pub fn ms_btl_get_clear(mem: &mut impl Mem, p: u32, kind: u32) -> u32 {
        crate::util::bridge::ms_struct_clear(mem, p, 0x1b0);
        for i in 0..8u32 {
            #[allow(clippy::cast_possible_truncation)] // the original's halfword
            let v = (kind << 12).wrapping_add(i) as u16;
            mem.write(p.wrapping_add(0xee + 2 * i), v);
        }
        8
    }

    /// `MsBtlGetInit()` (`0x00798b60`): the battle's gains record
    /// (`g_BtlCountsBattle` first) cleared with kind 7. EAX 8.
    pub fn ms_btl_get_init(mem: &mut impl Mem) -> u32 {
        let p = crate::flow::bridge::COUNTS_BATTLE.ptr(mem).addr;
        ms_btl_get_clear(mem, p, 7)
    }

    /// `MsEtEffectNumber_0079f1c0(table, n)`: numbers a status effect
    /// table's entries (4 bytes, up to one of three zero bytes): each whose
    /// byte 0 is not 0xff gets `n` (then counted up) in byte 2, past 0x18
    /// an error (`MsBattleError("eff_id")`). The next number.
    pub fn ms_et_effect_number<M: Mem>(mem: &mut M, call: Call<'_, M>, table: u32, n: u32) -> u32 {
        let mut n = n;
        let mut e = table;
        loop {
            let [b0, b1, b2]: [u8; 3] = mem.read(e);
            if b0 == 0 && b1 == 0 && b2 == 0 {
                return n;
            }
            if b0 != 0xff {
                #[allow(clippy::cast_possible_truncation)] // the original's byte store
                mem.write(e.wrapping_add(2), n as u8);
                n = n.wrapping_add(1);
                if n.cast_signed() > 0x18 {
                    let msg = code(mem, EFF_ID_ERROR);
                    game(mem, &mut *call, MS_BATTLE_ERROR, &[msg]);
                }
            }
            e = e.wrapping_add(4);
        }
    }

    /// `MsEtEffectInit()` (`0x0079e670`): the status effects' numbers, on
    /// from 0 over `g_StatusTable`, `g_BtlStatCountTable` and
    /// `g_StatusTable2`. The last.
    pub fn ms_et_effect_init<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        use crate::state::bridge::{STATUS_TABLE, STATUS_TABLE2};
        let tables = [
            STATUS_TABLE.ptr(mem).addr,
            crate::damage::bridge::STAT_COUNT_TABLE.ptr(mem).addr,
            STATUS_TABLE2.ptr(mem).addr,
        ];
        tables
            .into_iter()
            .fold(0, |n, t| ms_et_effect_number(mem, &mut *call, t, n))
    }

    /// The field's item tables' sizes and buffers (`MsFieldItemRead`;
    /// `g_BtlFieldItem`, `g_BtlFieldItem2` hold the buffers).
    pub const FIELD_ITEM_SIZE: Global<u32> = Global::new(0x0113_5ff0);
    pub const FIELD_ITEM2_SIZE: Global<u32> = Global::new(0x0113_5ff8);

    /// A kernel file into a new block: its size by the CD object
    /// (`+0x14`), the block aligned (`Sg_MemSetAlign(0x40)`, `user_malloc`,
    /// booked by `MsMemCheckRegist`) and read (`+0`). The read's result.
    fn read_into<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        file: u32,
        size: Global<u32>,
        buffer: Global<u32>,
    ) -> u32 {
        let n = cd(mem, &mut *call, 0x14, &[file]);
        size.set(mem, n);
        game(mem, &mut *call, SG_MEM_SET_ALIGN, &[0x40]);
        let p = game(mem, &mut *call, USER_MALLOC, &[size.get(mem)]);
        buffer.set(mem, p);
        crate::state::bridge::ms_mem_check_regist(mem, size.get(mem).cast_signed());
        cd(mem, call, 0, &[file, buffer.get(mem), 0, 0])
    }

    /// `MsFieldItemRead()` (`0x00799020`): directory 0xd's files 0x19 and
    /// 0x1a into their blocks (`g_BtlFieldItem`, `g_BtlFieldItem2`). EAX
    /// the second read's.
    pub fn ms_field_item_read<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        use crate::free::bridge::{FIELD_ITEM, FIELD_ITEM2};
        cd(mem, &mut *call, 0x10, &[0xd]);
        read_into(mem, &mut *call, 0x19, FIELD_ITEM_SIZE, FIELD_ITEM);
        read_into(mem, call, 0x1a, FIELD_ITEM2_SIZE, FIELD_ITEM2)
    }

    /// A kernel table read whole (`getExcelBinSize(name, buffer)`, a
    /// [`READ_BUFFER`] block) and copied into its table (made the first
    /// time: aligned, `user_malloc`, booked), the buffer freed. The free's
    /// result.
    fn read_table<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        name: u32,
        size: Global<u32>,
        table: Global<u32>,
    ) -> u32 {
        let buf = game(mem, &mut *call, USER_MALLOC, &[READ_BUFFER]);
        let name = code(mem, name);
        let n = game(mem, &mut *call, GET_EXCEL_BIN_SIZE, &[name, buf]);
        size.set(mem, n);
        if table.get(mem) == 0 {
            game(mem, &mut *call, SG_MEM_SET_ALIGN, &[0x40]);
            let p = game(mem, &mut *call, USER_MALLOC, &[size.get(mem)]);
            table.set(mem, p);
            crate::state::bridge::ms_mem_check_regist(mem, size.get(mem).cast_signed());
        }
        let n = size.get(mem);
        copy(mem, table.get(mem), buf, n);
        game(mem, call, USER_FREE, &[buf])
    }

    /// `"w_name"`, `"important"`.
    pub const WEAPON_NAME_FILE: u32 = 0x00b5_4e3c;
    pub const IMPORTANT_FILE: u32 = 0x00b5_4b4c;
    /// The weapon names' and key items' sizes (`g_BtlItemMem3`,
    /// `0x011334bc`).
    pub const WEAPON_NAMES_SIZE: Global<u32> = Global::new(0x0113_63b0);
    pub const IMPORTANT_SIZE: Global<u32> = Global::new(0x0113_34bc);

    /// `MsWeaponNameRead()` (`0x007a2600`): the weapon names (`w_name`)
    /// into `g_WeaponNameTable`. EAX the free's.
    pub fn ms_weapon_name_read<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        let table = crate::menu::bridge::WEAPON_NAMES;
        read_table(mem, call, WEAPON_NAME_FILE, WEAPON_NAMES_SIZE, table)
    }

    /// `MsImportantRead()` (`0x007908c0`): the key items (`important`) into
    /// `g_KernelImportant`. 0.
    pub fn ms_important_read<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        let table = crate::save::bridge::KERNEL_IMPORTANT;
        read_table(mem, call, IMPORTANT_FILE, IMPORTANT_SIZE, table);
        0
    }

    /// The menu texts' kernel files (`MsMenuReadText`'s first switch): the
    /// text's number and its name. The others (2, 0xd, 0xf, 0x10) are CD
    /// files of directory 0x27 read asynchronously.
    pub const MENU_TEXT_FILES: [(u32, u32); 13] = [
        (0, 0x00b5_4ad8),
        (1, 0x00b5_4b10),
        (3, 0x00b5_4b34),
        (4, 0x00b5_4ae0),
        (5, 0x00b5_4acc),
        (6, 0x00b5_4aec),
        (7, 0x00b5_4af8),
        (8, 0x00b5_4b04),
        (9, 0x00b5_4b1c),
        (10, 0x00b5_4b28),
        (11, 0x00b5_4ac0),
        (12, 0x00b5_4b40),
        (0xe, 0x00b5_4af8),
    ];
    /// The async read's completion (`0x0078fed0`: clears the text's busy
    /// byte, number at the request's `+0xc`).
    pub const MENU_TEXT_READ_DONE: u32 = 0x0078_fed0;
    /// The request block the async read gets (`+0xc` the text's number).
    pub const MENU_TEXT_REQUEST: u32 = 0x10;

    /// `MsMenuReadText(i, load)` (`0x0078fef0`): menu text `i` (below 0x11;
    /// else -1) marked busy (`g_BtlMenuText`'s `+0`), its size (`+4`) from
    /// its kernel file (read into a [`READ_BUFFER`] block) or, for the CD
    /// texts, `MsGetFileSize(0x27, load ? 9 : 0x27, i)`; its block made the
    /// first time (`+8`, a 32-bit pointer handle: aligned, `user_malloc`,
    /// booked, `set_ptr32`). A kernel text is copied in, not busy, its
    /// buffer freed; a CD text is read asynchronously (CD `+0`, with
    /// [`MENU_TEXT_READ_DONE`] and `request`, [`MENU_TEXT_REQUEST`] bytes
    /// of the caller's whose `+0xc` gets `i`). 0.
    pub fn ms_menu_read_text<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        load: u32,
        request: u32,
    ) -> u32 {
        if i > 0x10 {
            return u32::MAX;
        }
        let row = crate::flow::bridge::MENU_TEXT
            .ptr(mem)
            .addr
            .wrapping_add(12 * i);
        mem.write(row, 1u8);
        let file = MENU_TEXT_FILES
            .iter()
            .find(|&&(k, _)| k == i)
            .map(|&(_, f)| f);
        let mut buf = 0;
        let size = match file {
            Some(name) => {
                buf = game(mem, &mut *call, USER_MALLOC, &[READ_BUFFER]);
                let name = code(mem, name);
                game(mem, &mut *call, GET_EXCEL_BIN_SIZE, &[name, buf])
            }
            None => {
                let dir2 = if load != 0 { 9 } else { 0x27 };
                crate::getmem::bridge::ms_get_file_size(mem, &mut *call, 0x27, dir2, i)
            }
        };
        mem.write(row.wrapping_add(4), size);
        let handle: u32 = mem.read(row.wrapping_add(8));
        if game(mem, &mut *call, IS_PTR32_NULL, &[handle]) != 0 {
            game(mem, &mut *call, SG_MEM_SET_ALIGN, &[0x40]);
            let n: u32 = mem.read(row.wrapping_add(4));
            let p = game(mem, &mut *call, USER_MALLOC, &[n]);
            let n: u32 = mem.read(row.wrapping_add(4));
            crate::state::bridge::ms_mem_check_regist(mem, n.cast_signed());
            let h = game(mem, &mut *call, SET_PTR32, &[p]);
            mem.write(row.wrapping_add(8), h);
        }
        let handle: u32 = mem.read(row.wrapping_add(8));
        if file.is_some() {
            let n: u32 = mem.read(row.wrapping_add(4));
            let to = game(mem, &mut *call, GET_PTR32_FUNC, &[handle]);
            copy(mem, to, buf, n);
            mem.write(row, 0u8);
            game(mem, call, USER_FREE, &[buf]);
        } else {
            mem.write(request.wrapping_add(0xc), i);
            let obj = game(mem, &mut *call, crate::getmem::bridge::GET_CD, &[]);
            let to = game(mem, &mut *call, GET_PTR32_FUNC, &[handle]);
            let f: u32 = mem.read(obj);
            let done = code(mem, MENU_TEXT_READ_DONE);
            call(mem, f, &[i, to, done, request]);
        }
        0
    }

    /// The menu texts read at the start (`0x00782b70`'s list, `i8`, ending
    /// with a negative).
    pub const START_TEXTS: u32 = 0x00c4_21b4;

    /// `MsMenuReadStartText_00782b70(k)`: start text `k`
    /// ([`START_TEXTS`]): 0 at the list's end, else -1 after reading it
    /// ([`ms_menu_read_text`] with `load`).
    pub fn ms_menu_read_start_text<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        k: u32,
        request: u32,
    ) -> u32 {
        let t: i8 = mem.read(code(mem, START_TEXTS).wrapping_add(k));
        if t < 0 {
            return 0;
        }
        ms_menu_read_text(mem, call, i32::from(t).cast_unsigned(), 1, request);
        u32::MAX
    }

    /// The boat stage's roll step (`f32` 0.002, `.rdata` 0x00b54ca4).
    pub const SPECIAL_ROLL_STEP: u32 = 0x00b5_4ca4;

    /// `MsBattleSpecialInit(kind)` (`0x0079de60`): the special stage's work
    /// (`g_BtlSpecialMem`, 0x870 bytes, `user_malloc`) and kind
    /// (`g_BtlSpecialKind`): kind 1 (the boat) with the particle test and
    /// the sea (`Yn_PSimTest`, `Yn_SeaCreate`), its roll step and the rest
    /// of its state zero, map objects 0x1c-0x1e' animations disposed; kinds
    /// 2 and 3 (the sea) with the up vector `+0x810` and the swell `+0x844`
    /// cleared; else none (kind 0). EAX as the original leaves it.
    pub fn ms_battle_special_init<M: Mem>(mem: &mut M, call: Call<'_, M>, kind: u32) -> u32 {
        use crate::free::bridge::{SPECIAL_KIND, SPECIAL_MEM};
        SPECIAL_MEM.set(mem, 0);
        #[allow(clippy::cast_possible_truncation)] // the original's byte store
        let k = (kind as u8).cast_signed();
        match kind {
            1 => {
                game(mem, &mut *call, YN_PSIM_TEST, &[]);
                game(mem, &mut *call, YN_SEA_CREATE, &[]);
                let w = game(mem, &mut *call, USER_MALLOC, &[0x870]);
                let step: u32 = mem.read(code(mem, SPECIAL_ROLL_STEP));
                mem.write(w.wrapping_add(0x838), step);
                for o in [
                    0x810, 0x854, 0x83c, 0x850, 0x840, 0x858, 0x864, 0x85c, 0x834, 0x844,
                ] {
                    mem.write(w.wrapping_add(o), 0u32);
                }
                SPECIAL_MEM.set(mem, w);
                for o in [
                    0x848, 0x84c, 0x814, 0x824, 0x818, 0x828, 0x81c, 0x82c, 0x820, 0x830,
                ] {
                    mem.write(w.wrapping_add(o), 0u32);
                }
                let mut eax = 0;
                for i in 0..3 {
                    eax = game(
                        mem,
                        &mut *call,
                        YI_DISPOSE_ANIM_D_OBJ,
                        &[0x1c + i, u32::MAX],
                    );
                }
                SPECIAL_KIND.set(mem, k);
                eax
            }
            2 | 3 => {
                let w = game(mem, call, USER_MALLOC, &[0x870]);
                mem.write(w.wrapping_add(0x810), 0u32);
                mem.write(w.wrapping_add(0x844), 0u32);
                SPECIAL_KIND.set(mem, k);
                SPECIAL_MEM.set(mem, w);
                w
            }
            _ => {
                SPECIAL_KIND.set(mem, 0);
                kind.wrapping_sub(3)
            }
        }
    }

    /// `dbgPrintf(fmt, ...)` (main), `Sg_MallocA(n, align, pool, name,
    /// flags)`, `Sg_FreeA(p, pool)` (main), `TOGetFFXLang()` (system, its
    /// jump), `TkGetTotalSyncVCounter()` (menu), `MsRandMize(n)`,
    /// `MsSetSaveConfigEnglish(on)` (battle, through the game for their
    /// other modules' callees), `MsWeaponNameNum(record)` (battle, not
    /// ported), `TkInitSaveWork()` (menu) and `eiAbmDataInit(map)`
    /// (sphere grid).
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const SG_MALLOC_A: u32 = 0x0083_f0a0;
    pub const SG_FREE_A: u32 = 0x0083_f090;
    pub const TO_GET_FFX_LANG: u32 = 0x008a_c2f0;
    pub const TK_GET_TOTAL_SYNC_V_COUNTER: u32 = 0x0088_7e10;
    pub const MS_RAND_MIZE: u32 = 0x0079_8890;
    pub const MS_SET_SAVE_CONFIG_ENGLISH: u32 = 0x0078_5ed0;
    pub const MS_WEAPON_NAME_NUM: u32 = 0x007a_0cf0;
    pub const TK_INIT_SAVE_WORK: u32 = 0x0088_df40;
    pub const EI_ABM_DATA_INIT: u32 = 0x00a5_3de0;

    /// `"-- SAVE RAM CLEAR -- ..."` and the kernel files' names (`Sg_MallocA`
    /// tags): directory 0xd files 3 (the menu's start), 4 `party_data`, 5
    /// `wep_data`, 0x1f `menu_panel`; the `ply_save` kernel table.
    pub const SAVE_CLEAR_MESSAGE: u32 = 0x00b5_4984;
    pub const MENU_DATA_NAME: u32 = 0x00b5_49c0;
    pub const PARTY_DATA_NAME: u32 = 0x00b5_49c8;
    pub const WEP_DATA_NAME: u32 = 0x00b5_4934;
    pub const PLY_SAVE_NAME: u32 = 0x00b5_49d4;
    pub const MENU_PANEL_NAME: u32 = 0x00b5_49e0;

    /// The save (`g_SaveData`, 0x68c0 bytes) and the parts a new game fills:
    /// the menu's start (7 words), the party's (gil on, 0x21 words), the
    /// menu panel's (8 words), the weapons (200 of 0x16 bytes), the 18
    /// characters' `ply_save` records (0x94 bytes) and names (0x14 bytes,
    /// kept again as the default names, 400 bytes cleared first), the
    /// starting items (8 IDs and counts) and the item list (0x100 IDs and
    /// counts), the sphere grid's map, the scenario.
    pub const SAVE_DATA: u32 = 0x0112_ca90;
    pub const SAVE_DATA_SIZE: i32 = 0x68c0;
    pub const SAVE_MENU: u32 = 0x0113_079c;
    pub const SAVE_GIL: u32 = 0x0113_07d8;
    pub const SAVE_MENU_PANEL: u32 = 0x0113_07b8;
    pub const SAVE_WEAPONS: u32 = 0x0113_0f2c;
    pub const SAVE_WEAPON_SIZE: u32 = 0x16;
    pub const SAVE_WEAPON_COUNT: u32 = 200;
    pub const PLY_SAVE: u32 = 0x0113_205c;
    pub const PLY_SAVE_SIZE: u32 = 0x94;
    pub const SAVE_NAMES: u32 = 0x0113_2ddc;
    pub const DEFAULT_NAMES: u32 = 0x0231_10a0;
    pub const NAME_SIZE: u32 = 0x14;
    pub const CHARACTERS: u32 = 18;
    pub const START_ITEM_IDS: u32 = 0x0113_081c;
    pub const START_ITEM_COUNTS: u32 = 0x0113_082c;
    pub const SAVE_ITEM_IDS: u32 = 0x0113_095c;
    pub const SAVE_ITEM_COUNTS: u32 = 0x0113_0b5c;
    pub const SAVE_ABILITY_MAP: u32 = 0x0112_ec7c;
    /// The random seed's save word (added to the frame counter).
    pub const SAVE_SEED: Global<u32> = Global::new(0x0113_07dc);
    /// Every weapon of the start file, not only the first 0x23 (a debug
    /// flag, `u8`).
    pub const ALL_WEAPONS: Global<u8> = Global::new(0x0112_a920);
    /// `FFXIsDebug`'s flag (`i8`) and the scenario it sets to 5.
    pub const FFX_IS_DEBUG: Global<i8> = Global::new(0x0133_c910);

    /// Directory 0xd's `file` read into a fresh block (`Sg_MallocA(n, 0x10,
    /// 5, name, -1)`, CD `+0x14`, `+0`, then `+8` to wait).
    fn read_block<M: Mem>(mem: &mut M, call: Call<'_, M>, file: u32, name: u32) -> u32 {
        let n = cd(mem, &mut *call, 0x14, &[file]);
        let name = code(mem, name);
        let p = game(mem, &mut *call, SG_MALLOC_A, &[n, 0x10, 5, name, u32::MAX]);
        cd(mem, &mut *call, 0, &[file, p, 0, 0]);
        cd(mem, call, 8, &[0]);
        p
    }

    /// The first record of `file` (`MsGetExcelData(0, ...)`), `words` of it
    /// to `to`, the block freed (`Sg_FreeA(p, 5)`).
    fn first_record<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        file: u32,
        name: u32,
        to: u32,
        words: u32,
    ) {
        let p = read_block(mem, &mut *call, file, name);
        let r = ffx_kernel::bridge::excel_data(mem, 0, p).0;
        copy(mem, code(mem, to), r, 4 * words);
        game(mem, call, SG_FREE_A, &[p, 5]);
    }

    /// `MsSetSaveStartGame()` (`0x00786b00`): a new game's save. The save
    /// cleared; from directory 0xd the menu's start, the party's and (after
    /// the characters) the menu panel's first records copied in; the
    /// weapons from `wep_data` (records until one comes again as the
    /// first, at most 0x23 unless [`ALL_WEAPONS`]; a character's, byte 4
    /// below 7, gets its name number from `MsWeaponNameNum` and its name's
    /// icon, `MsWeaponName`); the characters' `ply_save` records and names
    /// (`getExcelBinSize` into a block of its own; each name's bytes 18 and
    /// 19 the language, `TOGetFFXLang`, and 1); the random seed
    /// (`MsRandMize` of the frame counter plus [`SAVE_SEED`]); the starting
    /// items (an ID not 0xffff with a count) listed and marked found
    /// (`MsSetItemCheck`), the rest of the list empty (0xff, 0); then
    /// `MsSetSaveConfigEnglish(0)`, the weapons' equip flags, the menu's
    /// save work, the sphere grid's map and, on a debug build, scenario 5.
    /// EAX 5 then, else `eiAbmDataInit`'s.
    #[allow(clippy::too_many_lines)]
    pub fn ms_set_save_start_game<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        use ffx_kernel::bridge::excel_data;
        let msg = code(mem, SAVE_CLEAR_MESSAGE);
        game(mem, &mut *call, DBG_PRINTF, &[msg]);
        crate::util::bridge::ms_struct_clear(mem, code(mem, SAVE_DATA), SAVE_DATA_SIZE);
        cd(mem, &mut *call, 0x10, &[0xd]);
        first_record(mem, &mut *call, 3, MENU_DATA_NAME, SAVE_MENU, 7);
        first_record(mem, &mut *call, 4, PARTY_DATA_NAME, SAVE_GIL, 0x21);
        // the weapons
        let wep = read_block(mem, &mut *call, 5, WEP_DATA_NAME);
        let weapons = code(mem, SAVE_WEAPONS);
        let mut first = 0;
        for k in 0..SAVE_WEAPON_COUNT {
            let r = excel_data(mem, k.cast_signed(), wep).0;
            if first == 0 {
                first = r;
            } else if first == r {
                break;
            }
            if k >= 0x23 && ALL_WEAPONS.get(mem) == 0 {
                break;
            }
            let e = weapons.wrapping_add(SAVE_WEAPON_SIZE * k);
            copy(mem, e, r, SAVE_WEAPON_SIZE);
            let owner: u8 = mem.read(e.wrapping_add(4));
            if owner < 7 {
                let n = game(mem, &mut *call, MS_WEAPON_NAME_NUM, &[r]);
                #[allow(clippy::cast_possible_truncation)] // the original's halfword
                let n = n as u16;
                mem.write(e, n);
                crate::menu::bridge::ms_weapon_name(
                    mem,
                    u32::from(n),
                    u32::from(owner),
                    0,
                    e.wrapping_add(0xc),
                );
            }
        }
        game(mem, &mut *call, SG_FREE_A, &[wep, 5]);
        // the characters' records and names
        let buf = game(mem, &mut *call, USER_MALLOC, &[READ_BUFFER]);
        let name = code(mem, PLY_SAVE_NAME);
        let n = game(mem, &mut *call, GET_EXCEL_BIN_SIZE, &[name, buf]);
        let ply = game(mem, &mut *call, SG_MALLOC_A, &[n, 0x10, 5, name, u32::MAX]);
        copy(mem, ply, buf, n);
        game(mem, &mut *call, USER_FREE, &[buf]);
        let defaults = code(mem, DEFAULT_NAMES);
        mem.write_bytes(defaults, &[0; 400]);
        let (records, names) = (code(mem, PLY_SAVE), code(mem, SAVE_NAMES));
        for i in 0..CHARACTERS {
            let (r, strings) = excel_data(mem, i.cast_signed(), ply);
            copy(
                mem,
                records.wrapping_add(PLY_SAVE_SIZE * i),
                r,
                PLY_SAVE_SIZE,
            );
            let from = u32::from(mem.read::<u16>(r)).wrapping_add(strings);
            let mut bytes = [0u8; NAME_SIZE as usize];
            mem.read_bytes(from, &mut bytes);
            mem.write_bytes(names.wrapping_add(NAME_SIZE * i), &bytes);
            mem.write_bytes(defaults.wrapping_add(NAME_SIZE * i), &bytes);
            let lang = game(mem, &mut *call, TO_GET_FFX_LANG_FN, &[]);
            let at = names.wrapping_add(NAME_SIZE * i + 0x12);
            mem.write(at, lang.to_le_bytes()[0]);
            mem.write(at.wrapping_add(1), 1u8);
        }
        game(mem, &mut *call, SG_FREE_A, &[ply, 5]);
        first_record(mem, &mut *call, 0x1f, MENU_PANEL_NAME, SAVE_MENU_PANEL, 8);
        // the random seed, the starting items
        let frames = game(mem, &mut *call, TK_GET_TOTAL_SYNC_V_COUNTER, &[]);
        let seed = frames.wrapping_add(SAVE_SEED.get(mem));
        game(mem, &mut *call, MS_RAND_MIZE, &[seed]);
        let (ids, counts) = (code(mem, SAVE_ITEM_IDS), code(mem, SAVE_ITEM_COUNTS));
        let (start_ids, start_counts) = (code(mem, START_ITEM_IDS), code(mem, START_ITEM_COUNTS));
        let mut n = 0u32;
        for j in 0..8 {
            let id: u16 = mem.read(start_ids.wrapping_add(2 * j));
            let count: u8 = mem.read(start_counts.wrapping_add(j));
            if id == 0xffff || count == 0 {
                continue;
            }
            mem.write(ids.wrapping_add(2 * n), id);
            mem.write(counts.wrapping_add(n), count);
            let table = crate::save::bridge::ms_get_item_check(mem);
            let id: u16 = mem.read(start_ids.wrapping_add(2 * j));
            crate::save::bridge::ms_set_item_check(mem, u32::from(id), table);
            n += 1;
        }
        for k in n..0x100 {
            mem.write(ids.wrapping_add(2 * k), 0xffu16);
            mem.write(counts.wrapping_add(k), 0u8);
        }
        game(mem, &mut *call, MS_SET_SAVE_CONFIG_ENGLISH, &[0]);
        crate::minor::bridge::ms_init_save_weapon_equip_flag(mem);
        game(mem, &mut *call, TK_INIT_SAVE_WORK, &[]);
        let map = code(mem, SAVE_ABILITY_MAP);
        let eax = game(mem, call, EI_ABM_DATA_INIT, &[map]);
        if FFX_IS_DEBUG.get(mem) != 0 {
            crate::save::bridge::SCENARIO.set(mem, 5);
            return 5;
        }
        eax
    }

    /// `FFXGetAsiaMode(english)` (system), the start's battle inits run
    /// through the game (`MsInitParamData`, `MsBtlReadInit`,
    /// `MsBtlInitItemMem`, `MsSEPtransInit`, `MsInitChrAbilityMap`,
    /// `MsMenuInitText`, `MsAtelRegistInit`, `MsNumberInit`; after the
    /// ported two `MsRegMotionInit`, `MsEventEffectInit`,
    /// `MsSetGlobals_00790840`, `MsBtlInitItemMem_007a0cd0`,
    /// `MsEffectEnvInit`; after `MsBtlGetInit` `MsMemCheckInit`).
    pub const FFX_GET_ASIA_MODE: u32 = 0x0078_26a0;
    pub const START_INITS: [u32; 8] = [
        0x0078_5960,
        0x0078_3000,
        0x0079_a000,
        0x007a_0500,
        0x0079_8820,
        0x0078_fe60,
        0x0079_7ae0,
        0x0079_f9f0,
    ];
    pub const START_INITS2: [u32; 5] = [
        0x0079_ccd0,
        0x0078_8be0,
        0x0079_0840,
        0x007a_0cd0,
        0x0078_7a60,
    ];
    pub const MS_MEM_CHECK_INIT: u32 = 0x0079_9ee0;
    /// `Kernel_LoadTables(i)` (non-zero while there are more),
    /// `MsGetCheckEnglish()`, `MsBtlListInit()`, `FFXIsDebug()` (system),
    /// `MsSetSaveParty()`, `MsBtlPosInit()`, `MsBattleExe(mode, field,
    /// group, slot)`, `MsBtlReadSetScene()`, `MsBattleInitAtEvent(on)`,
    /// `Sg_EffectRoutineEnable(on)` (main), `MsBattleLabelExe(word, a, b)`,
    /// `MsDebugSetScene(field, group, slot, hi, lo)` and the debug fills
    /// (`MsDebugFullItem`, `MsDebugFullCommand`, `MsDebugFullMember`,
    /// `MsDebugFullGill`): through the game.
    pub const KERNEL_LOAD_TABLES: u32 = 0x0078_1d40;
    pub const MS_GET_CHECK_ENGLISH: u32 = 0x0078_4e60;
    pub const MS_BTL_LIST_INIT: u32 = 0x0079_d250;
    pub const FFX_IS_DEBUG_FN: u32 = 0x0088_7c50;
    pub const MS_SET_SAVE_PARTY: u32 = 0x0078_6930;
    pub const MS_BTL_POS_INIT: u32 = 0x007c_93d0;
    pub const MS_BATTLE_EXE: u32 = 0x0078_1020;
    pub const MS_BTL_READ_SET_SCENE: u32 = 0x0078_3e20;
    pub const MS_BATTLE_INIT_AT_EVENT: u32 = 0x0078_1c50;
    pub const SG_EFFECT_ROUTINE_ENABLE: u32 = 0x0082_0380;
    pub const MS_BATTLE_LABEL_EXE: u32 = 0x0078_1c90;
    pub const MS_DEBUG_SET_SCENE: u32 = 0x007c_5f60;
    pub const MS_DEBUG_FULL: [u32; 4] = [0x0078_49d0, 0x0078_48b0, 0x0078_4ac0, 0x0078_49c0];

    /// `"* BTL INIT %x %x %x %x ASIA %d:%d\n"` and the four sizes it prints
    /// (an actor, a weapon record, and two more).
    pub const INIT_MESSAGE: u32 = 0x00b5_4740;
    pub const INIT_SIZES: [u32; 4] = [0xf90, 0x390, 0x1ec, 0x2c];

    /// `MsGetSaveConfigEnglish()` kept (`u32`).
    pub const CONFIG_ENGLISH: Global<u32> = Global::new(0x0112_ca34);

    /// The kernel tables `MsBattleInit` reads whole from the CD (directory
    /// 0xd, then 0x2f from the seventh): file, size (`u16`; the first's
    /// read signed), block.
    pub const START_TABLES: [(u32, u32, u32); 8] = [
        (0, 0x0112_a9d2, 0x0112_a9c4),
        (0xb, 0x0112_a974, 0x0112_a94c),
        (0xe, 0x0112_a972, 0x0112_a948),
        (0x1c, 0x0112_a976, 0x0112_a950),
        (0x1d, 0x0112_a978, 0x0112_a958),
        (0x1e, 0x0112_a97a, 0x0112_a954),
        (5, 0x0112_a980, 0x0112_a964),
        (6, 0x0112_a97e, 0x0112_a960),
    ];

    /// The menu texts read at the start (`load` 0), and the English ones.
    pub const START_MENU_TEXTS: [u32; 3] = [7, 9, 3];
    pub const ENGLISH_MENU_TEXTS: [u32; 3] = [0xe, 0xf, 0x10];

    /// The ground's height (`f32` -14.0 from 0x00b548dc into `g_BtlGround`),
    /// the idle rounds' limit (1), the memory word (`g_BtlMemWord`, its high
    /// half `0x0112ca32`), the debug menu's full fills toggle.
    pub const GROUND_INIT: u32 = 0x00b5_48dc;
    pub const GROUND: Global<u32> = Global::new(0x0112_bde8);
    pub const IDLE_ROUNDS_LIMIT: Global<u8> = Global::new(0x0112_bde3);
    pub const MEM_WORD: Global<u32> = Global::new(0x0112_ca30);
    pub const DEBUG_FULL: Global<u8> = Global::new(0x0112_a905);
    /// The battle's field index (`i16`), group and slot (`i8`, `MsBattleExe`
    /// sets them) that a debug build's `MsDebugSetScene` gets.
    pub const SCENE_FIELD_INDEX: Global<i16> = Global::new(0x0112_c256);
    pub const SCENE_GROUP_INDEX: Global<i8> = Global::new(0x0112_c258);
    pub const SCENE_SLOT_INDEX: Global<i8> = Global::new(0x0112_c259);

    /// `MsBattleInit()` (`0x00781700`): the battle module at the game's
    /// start: the sizes printed, the English config kept, the inits
    /// ([`START_INITS`], `MsEtEffectInit`, `MsBattleSpecialInit(0)`,
    /// [`START_INITS2`], `MsBtlGetInit`, `MsMemCheckInit`); the kernel
    /// tables ([`START_TABLES`]: size by CD `+0x14`, aligned block
    /// booked, read and waited for, `+8`) and the rest
    /// (`Kernel_LoadTables` until it is done, a wait after each); the menu
    /// texts ([`START_MENU_TEXTS`], [`ENGLISH_MENU_TEXTS`] while
    /// `MsGetCheckEnglish`, the start list with a wait after each); the key
    /// items, field items and weapon names, each waited for; a new game's
    /// save; the ground, the idle limit, no script first strike; the
    /// encounter list (`MsBtlListInit`, on a debug build the party and the
    /// battle positions); an empty battle (`MsBattleExe(-1, 0, 0, 1)`),
    /// waited for, its scene and scripts; the event battle off, effects
    /// on; `g_BtlExe` from `g_BtlExeKeep` after the memory word's label
    /// (`MsBattleLabelExe`, on a debug build its scene), else 0; the memory
    /// check against 0x70000 and, with the debug menu's toggle, the full
    /// fills. `request` is [`MENU_TEXT_REQUEST`] bytes for the CD texts'
    /// reads. EAX as the original leaves it.
    #[allow(clippy::too_many_lines)]
    pub fn ms_battle_init<M: Mem>(mem: &mut M, call: Call<'_, M>, request: u32) -> u32 {
        let english = crate::getmem::bridge::ms_get_save_config_english();
        let asia = game(mem, &mut *call, FFX_GET_ASIA_MODE, &[english]);
        let fmt = code(mem, INIT_MESSAGE);
        let [actor, weapon, third, fourth] = INIT_SIZES;
        game(
            mem,
            &mut *call,
            DBG_PRINTF,
            &[fmt, actor, weapon, third, fourth, asia],
        );
        CONFIG_ENGLISH.set(mem, crate::getmem::bridge::ms_get_save_config_english());
        for f in START_INITS {
            game(mem, &mut *call, f, &[]);
        }
        ms_et_effect_init(mem, &mut *call);
        ms_battle_special_init(mem, &mut *call, 0);
        for f in START_INITS2 {
            game(mem, &mut *call, f, &[]);
        }
        ms_btl_get_init(mem);
        game(mem, &mut *call, MS_MEM_CHECK_INIT, &[]);
        cd(mem, &mut *call, 0x10, &[0xd]);
        for (k, (file, size, block)) in START_TABLES.into_iter().enumerate() {
            if k == 6 {
                cd(mem, &mut *call, 0x10, &[0x2f]);
            }
            let size = Global::<u16>::new(size);
            let block = Global::<u32>::new(block);
            let got = cd(mem, &mut *call, 0x14, &[file]);
            #[allow(clippy::cast_possible_truncation)] // the original's halfword store
            size.set(mem, got as u16);
            game(mem, &mut *call, SG_MEM_SET_ALIGN, &[0x40]);
            let n = if k == 0 {
                i32::from(size.get(mem).cast_signed()).cast_unsigned()
            } else {
                u32::from(size.get(mem))
            };
            let p = game(mem, &mut *call, USER_MALLOC, &[n]);
            block.set(mem, p);
            crate::state::bridge::ms_mem_check_regist(mem, n.cast_signed());
            cd(mem, &mut *call, 0, &[file, block.get(mem), 0, 0]);
            cd(mem, &mut *call, 8, &[0]);
        }
        let mut i = 0;
        while game(mem, &mut *call, KERNEL_LOAD_TABLES, &[i]) != 0 {
            i += 1;
            cd(mem, &mut *call, 8, &[0]);
        }
        for t in START_MENU_TEXTS {
            ms_menu_read_text(mem, &mut *call, t, 0, request);
        }
        if game(mem, &mut *call, MS_GET_CHECK_ENGLISH, &[]) != 0 {
            for t in ENGLISH_MENU_TEXTS {
                ms_menu_read_text(mem, &mut *call, t, 0, request);
            }
        }
        let mut k = 0;
        while ms_menu_read_start_text(mem, &mut *call, k, request) != 0 {
            k += 1;
            cd(mem, &mut *call, 8, &[0]);
        }
        ms_important_read(mem, &mut *call);
        cd(mem, &mut *call, 8, &[0]);
        ms_field_item_read(mem, &mut *call);
        cd(mem, &mut *call, 8, &[0]);
        ms_weapon_name_read(mem, &mut *call);
        cd(mem, &mut *call, 8, &[0]);
        ms_set_save_start_game(mem, &mut *call);
        let ground: u32 = mem.read(code(mem, GROUND_INIT));
        GROUND.set(mem, ground);
        IDLE_ROUNDS_LIMIT.set(mem, 1);
        crate::read::bridge::FIRST_ATTACK_SET.set(mem, -1);
        game(mem, &mut *call, MS_BTL_LIST_INIT, &[]);
        if game(mem, &mut *call, FFX_IS_DEBUG_FN, &[]) != 0 {
            game(mem, &mut *call, MS_SET_SAVE_PARTY, &[]);
            game(mem, &mut *call, MS_BTL_POS_INIT, &[]);
        }
        game(mem, &mut *call, MS_BATTLE_EXE, &[u32::MAX, 0, 0, 1]);
        cd(mem, &mut *call, 8, &[0]);
        game(mem, &mut *call, MS_BTL_READ_SET_SCENE, &[]);
        crate::getmem::bridge::ms_btl_script_record_shift(mem);
        game(mem, &mut *call, MS_BATTLE_INIT_AT_EVENT, &[0]);
        game(mem, &mut *call, SG_EFFECT_ROUTINE_ENABLE, &[1]);
        let exe = crate::state::bridge::EXE;
        let word = MEM_WORD.get(mem);
        if word == 0 {
            exe.set(mem, 0);
        } else {
            game(mem, &mut *call, MS_BATTLE_LABEL_EXE, &[word, 0, 0]);
            if game(mem, &mut *call, FFX_IS_DEBUG_FN, &[]) != 0 {
                let field = i32::from(SCENE_FIELD_INDEX.get(mem));
                let group = i32::from(SCENE_GROUP_INDEX.get(mem));
                let slot = i32::from(SCENE_SLOT_INDEX.get(mem));
                let [lo, hi] = [word & 0xffff, word >> 16];
                let args = [field, group, slot].map(i32::cast_unsigned);
                game(
                    mem,
                    &mut *call,
                    MS_DEBUG_SET_SCENE,
                    &[args[0], args[1], args[2], hi, lo],
                );
            }
            exe.set(mem, crate::state::bridge::EXE_KEEP.get(mem).cast_signed());
        }
        let eax = ms_mem_check_sum(mem, &mut *call, 0x7_0000);
        if DEBUG_FULL.get(mem) == 0 {
            return eax;
        }
        let mut eax = 0;
        for f in MS_DEBUG_FULL {
            eax = game(mem, &mut *call, f, &[]);
        }
        eax
    }
}
