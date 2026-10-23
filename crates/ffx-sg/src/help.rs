//! The help menu's list drawer `0x0076f850` (`menu_list_draw`, name ours),
//! the draw slot (`0x00c416b0`) next to [`crate::state::bridge::menu_list_step`]'s
//! (`0x00c416ac`) in the help menu's function table. It walks the current
//! list's records (0x138 bytes, the lists of `menu_list_step`), moves them
//! (scrolling or sliding positions, x87 floats at `+0x118`/`+0x11c`) and
//! draws each kind into the GS packet through the PS2 DMA stand-ins
//! (`FlushCache`, `sceDmaSend`, `sceGsSyncPath`).

/// The original over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem, x87};

    use crate::state::bridge::{
        CDLIB_READ, CDLIB_SYNC, CDLIB_TABLE, EXPAND_CODES, ML_BASE, ML_BUSY, ML_CURRENT,
        ML_IS_TARGET, ML_LISTS, ML_OUT, ML_SLOTS, ML_TARGET, ML_TEXT_BASE, ML_TEXTS, ML_TRG,
        SA_MENU_SBIN,
    };

    /// `Sg_PacketGetPtr()`, `hdd_kabe_disp()`, `RelayoutMenuRecords(screen,
    /// list, first, n)`, `sceGsSyncPath(0, 0)`, `sceDmaGetChan(2)`,
    /// `IsLangSpecialRecord(rec)`, `MenuRecordCondition(rec)`,
    /// `FlushCache(0)`, `sceDmaSend(chan, packet)`.
    pub const PACKET_GET_PTR: u32 = 0x0083_b650;
    pub const HDD_KABE_DISP: u32 = 0x0077_93c0;
    pub const RELAYOUT_RECORDS: u32 = 0x0077_5a20;
    pub const GS_SYNC_PATH: u32 = 0x0062_f880;
    pub const DMA_GET_CHAN: u32 = 0x0062_f6e0;
    pub const IS_LANG_SPECIAL: u32 = 0x0077_4680;
    pub const RECORD_CONDITION: u32 = 0x0077_4a50;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const DMA_SEND: u32 = 0x0062_f710;
    /// Text records: `TOGetMessageSize(text, &w, &h)`, the frame
    /// `0x00775330(packet, x, y, w, h)`, the text by screen (0xb:
    /// `MenuMes_AddPktFont1612IconsNoEdge`, 8: `MenuMes_AddPktFont1612NoEdge`,
    /// 5: `DrawMesFontLClutTypeRGBA_ForHelp` without the cursor, else
    /// `TOAddPktBaseMesFontLClutRGBA`), `TOGetFFXLangJmp()` and the digits
    /// (one: `TOMkpBtlDigitRight_008f3300` on screen 8, else `0x008f3280`;
    /// two: `TOMkpBtlDigitRight_008f3420` on screen 8, else `0x008f3390`).
    pub const MESSAGE_SIZE: u32 = 0x008b_7130;
    pub const TEXT_FRAME: u32 = 0x0077_5330;
    pub const FONT_ICONS: u32 = 0x008e_52e0;
    pub const FONT_NO_EDGE: u32 = 0x008e_52b0;
    pub const FONT_FOR_HELP: u32 = 0x008e_75c0;
    pub const FONT_BASE: u32 = 0x008e_e210;
    pub const FFX_LANG: u32 = 0x008a_c2f0;
    pub const DIGIT_ONE_8: u32 = 0x008f_3300;
    pub const DIGIT_ONE: u32 = 0x008f_3280;
    pub const DIGIT_TWO_8: u32 = 0x008f_3420;
    pub const DIGIT_TWO: u32 = 0x008f_3390;
    /// Kinds 1 and 2: `0x00774cb0(packet, rec, x, y)`. Kind 3: the value
    /// getters (`*0x00c416d0`, by `i16 +0x16`), `TOAdpValueFontLClutRight`
    /// and `TOAddPktMesValue`. Kind 4: `SndSepPlaySimple(sfx)`.
    pub const RECORD_DRAW: u32 = 0x0077_4cb0;
    pub const SCRIPT_GETTERS: Global<u32> = Global::new(0x00c4_16d0);
    pub const VALUE_RIGHT: u32 = 0x008f_30b0;
    pub const MES_VALUE: u32 = 0x008f_2ff0;
    pub const SND_SEP_PLAY_SIMPLE: u32 = 0x0088_6e80;
    /// Kind 6 (pictures): `getSa_MenuTextureName(out, n)`,
    /// `TOInternName(name)`, `graphicMenuLoadSpecialTexture(2, name, 0)`,
    /// `0x00775720(buffer, &w, &h)`, `MenuPictureSize(screen, list, n, &w,
    /// &h)`, `dbgPrintf("w=%d h=%d\n", w, h)`, `0x00774520(x, y, w, h,
    /// texture)`.
    pub const TEXTURE_NAME: u32 = 0x0077_4ae0;
    pub const INTERN_NAME: u32 = 0x008a_b440;
    pub const LOAD_SPECIAL_TEXTURE: u32 = 0x0064_2300;
    pub const PICTURE_READ_SIZE: u32 = 0x0077_5720;
    pub const PICTURE_SIZE: u32 = 0x0077_5820;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const DBG_FORMAT: u32 = 0x00b5_3348;
    pub const PICTURE_DRAW: u32 = 0x0077_4520;
    /// The end: `0x00779450()` with [`HDD_TABLE_A`] set.
    pub const HDD_DRAW: u32 = 0x0077_9450;

    /// Two of the HDD-copy tables (`sa_menu`'s clean-up zeroes them): with
    /// `0x0112a04c` set `hdd_kabe_disp` runs first, with `0x0112a048` set
    /// `0x00779450` last.
    pub const HDD_TABLE_A: Global<u32> = Global::new(0x0112_a048);
    pub const HDD_TABLE_B: Global<u32> = Global::new(0x0112_a04c);
    /// The picture read buffer (`g_CdLib` reads into it).
    pub const PICTURE_BUF: Global<u32> = Global::new(0x0112_a030);
    /// The sound pick of kind 4: `i8 +0xfa` and a slot value.
    pub const PICK_KIND: Global<u32> = Global::new(0x0112_a058);
    pub const PICK_VALUE: Global<u32> = Global::new(0x0112_a054);
    /// The digits `MsgExpandCodes` asks for (`0xfa`-style codes): their
    /// count (1 or 2), value, and the colour flag (`0x25` when set;
    /// `EXPAND_FLAG_F9`).
    pub const DIGIT_MODE: Global<u32> = Global::new(0x0112_a06c);
    pub const DIGIT_VALUE: Global<u32> = Global::new(0x0112_a070);
    pub const DIGIT_COLOUR: Global<u32> = Global::new(0x0112_a074);
    /// The interned special picture texture of kind 6.
    pub const PICTURE_TEXTURE: Global<u32> = Global::new(0x0112_a4ac);

    /// The original's frame words that callees write through pointers:
    /// `MsgExpandCodes`' flag, `TOGetMessageSize`'s `w`/`h`, the picture
    /// `w`/`h` and a 256-byte texture name.
    pub const FLAG: u32 = 0;
    pub const MSG_W: u32 = 4;
    pub const MSG_H: u32 = 8;
    pub const PIC_W: u32 = 0xc;
    pub const PIC_H: u32 = 0x10;
    pub const NAME: u32 = 0x14;
    /// The size of `work` for [`menu_list_draw`].
    pub const DRAW_WORK: u32 = NAME + 0x100;

    /// The screens of [`SA_MENU_SBIN`] the text and pictures switch on.
    const SCREEN_5: u32 = 5;
    const SCREEN_8: u32 = 8;
    const SCREEN_B: u32 = 0xb;

    /// What one record draw needs besides the record.
    struct Pass {
        packet: u32,
        chan: u32,
        x: i32,
        y: i32,
        /// The slot values of `+0x18` and `+0x24`.
        slot18: u32,
        slot24: u32,
    }

    /// A value slot by an `i16` index; 0 for a negative one.
    fn slot<M: Mem>(mem: &M, index: i16) -> u32 {
        if index < 0 {
            return 0;
        }
        let k = i32::from(index).cast_unsigned();
        mem.read(ML_SLOTS.get(mem).wrapping_add(k.wrapping_mul(4)))
    }

    fn i16_at<M: Mem>(mem: &M, at: u32) -> i32 {
        i32::from(mem.read::<i16>(at))
    }

    /// `FlushCache(0)`, `sceDmaSend(chan, packet)`, `sceGsSyncPath(0, 0)`.
    fn send<M, C>(mem: &mut M, call: &mut C, chan: u32, packet: u32)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let f = mem.rebase(FLUSH_CACHE);
        call(mem, f, &[0]);
        let f = mem.rebase(DMA_SEND);
        call(mem, f, &[chan, packet]);
        let f = mem.rebase(GS_SYNC_PATH);
        call(mem, f, &[0, 0]);
    }

    /// The DMA tags around what was drawn at `packet + 0x10 .. end`: an
    /// end tag (`0x70000000`) at `end`, a count tag (`0x10000000 |
    /// qwords - 1`) at `packet`.
    fn close<M: Mem>(mem: &mut M, packet: u32, end: u32) {
        for o in [4, 8, 0xc] {
            mem.write(end.wrapping_add(o), 0u32);
        }
        mem.write(end, 0x7000_0000u32);
        let tag = (end.wrapping_sub(packet) >> 4).wrapping_sub(1) | 0x1000_0000;
        for o in [4, 8, 0xc] {
            mem.write(packet.wrapping_add(o), 0u32);
        }
        mem.write(packet, tag);
    }

    /// `0x0076f850()` (`menu_list_draw`, name ours; its only reference is
    /// the help menu table slot `0x00c416b0`): draws the current list of
    /// `menu_list_step`. With `0x0112a04c` set `hdd_kabe_disp` runs; with
    /// lists, the current one's records (`i16` count) are laid out again
    /// (`RelayoutMenuRecords`), and each record:
    ///
    /// - takes its slot values (`+0x18`, `+0x24` and the offsets `+0x26`,
    ///   `+0x28`), notes whether it is the target (`0x0112a078`);
    /// - kind 2 sits at the target's position `+0x10/+0x12` plus `+0xc/+0xe`
    ///   (0, 0 without a target), others at `+4/+6` plus `+0xc/+0xe` plus
    ///   the truncated floats `+0x118/+0x11c`;
    /// - moves by `+0x110`: 0 clears the floats, 1 scrolls them by
    ///   `+0x124/+0x128` wrapping the position around 512 x 416 by the size
    ///   `+8/+0xa`, 2 slides the position by `+0x124/+0x128` times the
    ///   count `u16 +0x116` (counting down a delay `+0x114` first, then the
    ///   count);
    /// - stores the position at `+0x10/+0x12` (a language-special list puts
    ///   kind 2 at -100, -100 after that) and draws by kind: 0 text (with a
    ///   frame for a `MsgExpandCodes` flag, then one or two digits it asks
    ///   for), 1 and 2 (with a target) `0x00774cb0`, 3 a value through the
    ///   getter table, 4 a sound pick with pad bit 0x40, 6 a picture read
    ///   and sized once (`+0x2a` 0, 1, 2), then drawn; each under
    ///   `MenuRecordCondition` (0 draws) except kind 4.
    ///
    /// With `0x0112a048` set `0x00779450` runs last. `work` is
    /// [`DRAW_WORK`] bytes standing for the original's frame. The x87
    /// values follow the original: f32 loads, f64 sums, f32 stores.
    pub fn menu_list_draw<M, C>(mem: &mut M, work: u32, call: &mut C)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let f = mem.rebase(PACKET_GET_PTR);
        let packet = call(mem, f, &[]);
        if HDD_TABLE_B.get(mem) != 0 {
            let f = mem.rebase(HDD_KABE_DISP);
            call(mem, f, &[]);
        }
        let lists = ML_LISTS.get(mem);
        if lists != 0 {
            let current = ML_CURRENT.get(mem);
            let at = lists.wrapping_add(current.wrapping_mul(8));
            let count = i16_at(mem, at.wrapping_add(4));
            let mut rec = mem.read::<u32>(at).wrapping_add(ML_BASE.get(mem));
            let screen = SA_MENU_SBIN.get(mem);
            let f = mem.rebase(RELAYOUT_RECORDS);
            call(mem, f, &[screen, current, rec, count.cast_unsigned()]);
            let f = mem.rebase(GS_SYNC_PATH);
            call(mem, f, &[0, 0]);
            let f = mem.rebase(DMA_GET_CHAN);
            let chan = call(mem, f, &[2]);
            let f = mem.rebase(IS_LANG_SPECIAL);
            let target = ML_TARGET.get(mem);
            let special = call(mem, f, &[target]) != 0;
            for _ in 0..count.max(0) {
                draw_record(mem, call, work, rec, packet, chan, special);
                rec = rec.wrapping_add(0x138);
            }
        }
        if HDD_TABLE_A.get(mem) != 0 {
            let f = mem.rebase(HDD_DRAW);
            call(mem, f, &[]);
        }
    }

    /// One record of [`menu_list_draw`]: its position, movement and draw.
    fn draw_record<M, C>(
        mem: &mut M,
        call: &mut C,
        work: u32,
        rec: u32,
        packet: u32,
        chan: u32,
        special: bool,
    ) where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let at = |o: u32| rec.wrapping_add(o);
        let slot18 = slot(mem, mem.read(at(0x18)));
        let slot24 = slot(mem, mem.read(at(0x24)));
        let dx = slot(mem, mem.read(at(0x26))).cast_signed();
        let dy = slot(mem, mem.read(at(0x28))).cast_signed();
        let target = ML_TARGET.get(mem);
        ML_IS_TARGET.set(mem, u32::from(rec == target));
        let kind: u8 = mem.read(rec);
        let (mut x, mut y) = if kind != 2 {
            let fx = x87::ftol(mem, x87::load(mem.read(at(0x118)))).cast_signed();
            let x = i16_at(mem, at(4))
                .wrapping_add(i16_at(mem, at(0xc)).wrapping_add(dx).wrapping_add(fx));
            let fy = x87::ftol(mem, x87::load(mem.read(at(0x11c)))).cast_signed();
            let y = i16_at(mem, at(6))
                .wrapping_add(i16_at(mem, at(0xe)).wrapping_add(dy).wrapping_add(fy));
            (x, y)
        } else if target == 0 {
            (0, 0)
        } else {
            let tx = i16_at(mem, target.wrapping_add(0x10)).wrapping_add(dx);
            let ty = i16_at(mem, target.wrapping_add(0x12)).wrapping_add(dy);
            (
                i16_at(mem, at(0xc)).wrapping_add(tx),
                i16_at(mem, at(0xe)).wrapping_add(ty),
            )
        };
        let width = i16_at(mem, at(8));
        let height = i16_at(mem, at(0xa));
        match mem.read::<i8>(at(0x110)) {
            0 => {
                mem.write(at(0x11c), 0.0f32);
                mem.write(at(0x118), 0.0f32);
            }
            1 => (x, y) = scroll(mem, rec, x, y, width, height),
            2 => {
                let n = mem.read::<u16>(at(0x116));
                if n != 0 {
                    let k = f64::from(n);
                    let sx = x87::mul(x87::load(mem.read(at(0x124))), k);
                    x = x87::ftol(mem, x87::add(sx, f64::from(x))).cast_signed();
                    let sy = x87::mul(k, x87::load(mem.read(at(0x128))));
                    y = x87::ftol(mem, x87::add(sy, f64::from(y))).cast_signed();
                    let delay = mem.read::<u16>(at(0x114));
                    if delay == 0 {
                        mem.write(at(0x116), n.wrapping_sub(1));
                    } else {
                        mem.write(at(0x114), delay.wrapping_sub(1));
                    }
                }
            }
            _ => {}
        }
        mem.write(at(0x10), crate::lo16(x.cast_unsigned()));
        mem.write(at(0x12), crate::lo16(y.cast_unsigned()));
        if special && kind == 2 {
            (x, y) = (-100, -100);
        }
        let pass = Pass {
            packet,
            chan,
            x,
            y,
            slot18,
            slot24,
        };
        match kind {
            0 => draw_text(mem, call, work, rec, &pass),
            1 => draw_plain(mem, call, rec, &pass),
            2 if ML_TARGET.get(mem) != 0 => draw_plain(mem, call, rec, &pass),
            3 => draw_value(mem, call, rec, &pass),
            4 => pick_sound(mem, call, rec),
            6 => draw_picture(mem, call, work, rec, &pass, (width, height)),
            _ => {}
        }
    }

    /// `+0x110` 1: the floats `+0x118/+0x11c` step by `+0x124/+0x128`
    /// (rounded to f32 and stored); a position past the screen by the size
    /// wraps around 512 x 416, the float with it. Returns the position.
    fn scroll<M: Mem>(
        mem: &mut M,
        rec: u32,
        mut x: i32,
        mut y: i32,
        width: i32,
        height: i32,
    ) -> (i32, i32) {
        let at = |o: u32| rec.wrapping_add(o);
        let load = |mem: &M, o: u32| x87::load(mem.read(at(o)));
        let fx = x87::store(x87::add(load(mem, 0x124), load(mem, 0x118)));
        mem.write(at(0x118), fx);
        let fy = x87::store(x87::add(load(mem, 0x128), load(mem, 0x11c)));
        mem.write(at(0x11c), fy);
        let (wf, hf) = (f64::from(width), f64::from(height));
        if x < width.wrapping_neg() {
            x = x.wrapping_add(0x200).wrapping_add(width);
            let sum = x87::add(x87::load(fx), x87::add(wf, 512.0));
            mem.write(at(0x118), x87::store(sum));
        }
        if x > 0x200 {
            x = x.wrapping_add((-0x200i32).wrapping_sub(width));
            let sum = x87::sub(load(mem, 0x118), x87::add(512.0, wf));
            mem.write(at(0x118), x87::store(sum));
        }
        if y < height.wrapping_neg() {
            y = y.wrapping_add(0x1a0).wrapping_add(height);
            let sum = x87::add(x87::load(fy), x87::add(hf, 416.0));
            mem.write(at(0x11c), x87::store(sum));
        }
        if y > 0x1a0 {
            y = y.wrapping_add((-0x1a0i32).wrapping_sub(height));
            let sum = x87::sub(load(mem, 0x11c), x87::add(416.0, hf));
            mem.write(at(0x11c), x87::store(sum));
        }
        (x, y)
    }

    fn condition<M, C>(mem: &mut M, call: &mut C, rec: u32) -> bool
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let f = mem.rebase(RECORD_CONDITION);
        call(mem, f, &[rec]) == 0
    }

    /// Kind 0: the text expanded into `*0x0112a02c` (texts `*0x0112a024`
    /// by `i16 +0x16` + slot `+0x18`, with the base `*0x0112a040`), drawn
    /// unless empty (a frame first when `MsgExpandCodes` raised its flag),
    /// then the digits it asked for (`0x0112a06c` 1 or 2).
    fn draw_text<M, C>(mem: &mut M, call: &mut C, work: u32, rec: u32, p: &Pass)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        if !condition(mem, call, rec) {
            return;
        }
        let index = i16_at(mem, rec.wrapping_add(0x16))
            .cast_unsigned()
            .wrapping_add(p.slot18);
        DIGIT_COLOUR.set(mem, 0);
        DIGIT_MODE.set(mem, 0);
        let text: u32 = mem.read(ML_TEXTS.get(mem).wrapping_add(index.wrapping_mul(4)));
        let text = text.wrapping_add(ML_TEXT_BASE.get(mem));
        let flag = work.wrapping_add(FLAG);
        let f = mem.rebase(EXPAND_CODES);
        let out = ML_OUT.get(mem);
        call(mem, f, &[out, text, p.slot24, 0, flag]);
        let (x, y) = (p.x.cast_unsigned(), p.y.cast_unsigned());
        let out = ML_OUT.get(mem);
        if mem.read::<u8>(out) != 0 {
            if mem.read::<u32>(flag) != 0 {
                let (mw, mh) = (work.wrapping_add(MSG_W), work.wrapping_add(MSG_H));
                let f = mem.rebase(MESSAGE_SIZE);
                call(mem, f, &[out, mw, mh]);
                let size = [mem.read::<u32>(mw), mem.read::<u32>(mh)];
                let f = mem.rebase(TEXT_FRAME);
                call(mem, f, &[p.packet, x, y, size[0], size[1]]);
                send(mem, call, p.chan, p.packet);
            }
            let text = ML_OUT.get(mem);
            let cursor = p.packet.wrapping_add(0x10);
            let rest = [7, text, x, y, 0, 0x80, 0x80, 0x80, 0x80];
            let font = match SA_MENU_SBIN.get(mem) {
                SCREEN_B => Some(FONT_ICONS),
                SCREEN_8 => Some(FONT_NO_EDGE),
                SCREEN_5 => None,
                _ => Some(FONT_BASE),
            };
            let end = if let Some(font) = font {
                let mut args = [0u32; 10];
                args[0] = cursor;
                args[1..].copy_from_slice(&rest);
                let f = mem.rebase(font);
                call(mem, f, &args)
            } else {
                let f = mem.rebase(FONT_FOR_HELP);
                call(mem, f, &rest);
                cursor
            };
            close(mem, p.packet, end);
            send(mem, call, p.chan, p.packet);
        }
        let colour = if DIGIT_COLOUR.get(mem) == 0 { 0 } else { 0x25 };
        let screen8 = SA_MENU_SBIN.get(mem) == SCREEN_8;
        let (f, x) = match DIGIT_MODE.get(mem) {
            1 if screen8 => {
                let mut x = p.x;
                if x == 0xb9 && p.y.wrapping_sub(0x5e) % 0x26 == 0 {
                    let f = mem.rebase(FFX_LANG);
                    let lang = call(mem, f, &[]);
                    let wide = i32::from(DIGIT_VALUE.get(mem).cast_signed() > 9);
                    x = 0xb2 + if lang == 0 { 3 * wide } else { 2 * wide };
                }
                (DIGIT_ONE_8, x)
            }
            1 => (DIGIT_ONE, p.x),
            2 => (if screen8 { DIGIT_TWO_8 } else { DIGIT_TWO }, p.x),
            _ => return,
        };
        let value = DIGIT_VALUE.get(mem);
        let args = [
            p.packet.wrapping_add(0x10),
            value,
            x.wrapping_add(0x3c).cast_unsigned(),
            p.y.cast_unsigned(),
            colour,
        ];
        let f = mem.rebase(f);
        let end = call(mem, f, &args);
        close(mem, p.packet, end);
        send(mem, call, p.chan, p.packet);
    }

    /// Kinds 1 and 2: `0x00774cb0(packet, rec, x, y)`.
    fn draw_plain<M, C>(mem: &mut M, call: &mut C, rec: u32, p: &Pass)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        if !condition(mem, call, rec) {
            return;
        }
        let f = mem.rebase(RECORD_DRAW);
        call(
            mem,
            f,
            &[p.packet, rec, p.x.cast_unsigned(), p.y.cast_unsigned()],
        );
        send(mem, call, p.chan, p.packet);
    }

    /// Kind 3: the getter `(*0x00c416d0)[i16 +0x16](slot +0x18)`'s value
    /// drawn right-aligned (`+0x2a` 0) or as a message value (1).
    fn draw_value<M, C>(mem: &mut M, call: &mut C, rec: u32, p: &Pass)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        if !condition(mem, call, rec) {
            return;
        }
        let k = i16_at(mem, rec.wrapping_add(0x16)).cast_unsigned();
        let getter: u32 = mem.read(SCRIPT_GETTERS.get(mem).wrapping_add(k.wrapping_mul(4)));
        let value = call(mem, getter, &[p.slot18]);
        let f = match mem.read::<u8>(rec.wrapping_add(0x2a)) {
            0 => VALUE_RIGHT,
            1 => MES_VALUE,
            _ => return,
        };
        let args = [
            p.packet.wrapping_add(0x10),
            value,
            p.x.cast_unsigned(),
            p.y.cast_unsigned(),
            0,
        ];
        let f = mem.rebase(f);
        let end = call(mem, f, &args);
        close(mem, p.packet, end);
        send(mem, call, p.chan, p.packet);
    }

    /// Kind 4: with pad bit 0x40 and not busy, the pick (`i8 +0xfa`, slot
    /// `+0xfe` or 0 for -1) and its sound (`i16 +0xf2`, flagged); busy
    /// after.
    fn pick_sound<M, C>(mem: &mut M, call: &mut C, rec: u32)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        if ML_TRG.get(mem) & 0x40 == 0 || ML_BUSY.get(mem) != 0 {
            return;
        }
        let at = |o: u32| rec.wrapping_add(o);
        PICK_KIND.set(mem, i32::from(mem.read::<i8>(at(0xfa))).cast_unsigned());
        let index: i16 = mem.read(at(0xfe));
        let value = if index == -1 {
            0
        } else {
            let k = i32::from(index).cast_unsigned();
            mem.read(ML_SLOTS.get(mem).wrapping_add(k.wrapping_mul(4)))
        };
        PICK_VALUE.set(mem, value);
        let sfx: i16 = mem.read(at(0xf2));
        if sfx != 0 {
            let f = mem.rebase(SND_SEP_PLAY_SIMPLE);
            call(mem, f, &[i32::from(sfx).cast_unsigned() | 0x8000_0000]);
        }
        ML_BUSY.set(mem, 1);
    }

    /// Kind 6: a picture by slot `+0x18` (a new one restarts at state 0);
    /// state `+0x2a` 0 reads it (`g_CdLib`, unless screen 0xb or 5) and
    /// interns a special texture for `+0x14` 0x30, 1 waits for the read
    /// and takes its size (`+8/+0xa`), then `MenuPictureSize`; state 2
    /// draws it.
    fn draw_picture<M, C>(
        mem: &mut M,
        call: &mut C,
        work: u32,
        rec: u32,
        p: &Pass,
        size: (i32, i32),
    ) where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let at = |o: u32| rec.wrapping_add(o);
        if i16_at(mem, at(0x1a)) != p.slot18.cast_signed() {
            mem.write(at(0x1a), crate::lo16(p.slot18));
            mem.write(at(0x2a), 0u8);
        }
        if !condition(mem, call, rec) {
            return;
        }
        let reads = |mem: &M| !matches!(SA_MENU_SBIN.get(mem), SCREEN_B | SCREEN_5);
        let sync = |mem: &mut M, call: &mut C| {
            let f = CDLIB_SYNC.get(mem);
            call(mem, f, &[1]) == 0
        };
        let state: u8 = mem.read(at(0x2a));
        if state > 2 {
            return;
        }
        // the picture number, `i16 +0x16` read again at each use
        let picture = |mem: &M| i16_at(mem, at(0x16)).cast_unsigned().wrapping_add(p.slot18);
        if state < 2 {
            if state == 0 {
                if reads(mem) {
                    if !sync(mem, call) {
                        return;
                    }
                    let f = CDLIB_TABLE.get(mem);
                    let table = i16_at(mem, at(0x14)).cast_unsigned();
                    call(mem, f, &[table]);
                    let f = CDLIB_READ.get(mem);
                    let buf = PICTURE_BUF.get(mem);
                    let n = picture(mem);
                    call(mem, f, &[n, buf, 0, 0]);
                }
                let special = mem.read::<u16>(at(0x14)) == 0x30;
                mem.write(at(0x2a), 1u8);
                if special {
                    let name = work.wrapping_add(NAME);
                    mem.write(name, [0u8; 0x100]);
                    let f = mem.rebase(TEXTURE_NAME);
                    let k = picture(mem).wrapping_sub(0xe);
                    call(mem, f, &[name, k]);
                    let f = mem.rebase(INTERN_NAME);
                    let texture = call(mem, f, &[name]);
                    PICTURE_TEXTURE.set(mem, texture);
                    if reads(mem) {
                        let f = mem.rebase(LOAD_SPECIAL_TEXTURE);
                        call(mem, f, &[2, texture, 0]);
                    }
                }
            }
            let (pw, ph) = (work.wrapping_add(PIC_W), work.wrapping_add(PIC_H));
            mem.write(pw, size.0);
            mem.write(ph, size.1);
            if reads(mem) {
                if !sync(mem, call) {
                    return;
                }
                let f = mem.rebase(PICTURE_READ_SIZE);
                let buf = PICTURE_BUF.get(mem);
                call(mem, f, &[buf, pw, ph]);
                mem.write(at(8), mem.read::<u16>(pw));
                mem.write(at(0xa), mem.read::<u16>(ph));
            }
            let screen = SA_MENU_SBIN.get(mem);
            let list = ML_CURRENT.get(mem);
            let f = mem.rebase(PICTURE_SIZE);
            let k = picture(mem).wrapping_sub(0xe);
            call(mem, f, &[screen, list, k, at(8), at(0xa)]);
            mem.write(at(0x2a), 2u8);
            let f = mem.rebase(DBG_PRINTF);
            let format = mem.rebase(DBG_FORMAT);
            let args = [format, mem.read(pw), mem.read(ph)];
            call(mem, f, &args);
        }
        let texture = PICTURE_TEXTURE.get(mem);
        if texture != 0 {
            let w = i16_at(mem, at(8)).cast_unsigned();
            let h = i16_at(mem, at(0xa)).cast_unsigned();
            let f = mem.rebase(PICTURE_DRAW);
            call(
                mem,
                f,
                &[p.x.cast_unsigned(), p.y.cast_unsigned(), w, h, texture],
            );
        }
        send(mem, call, p.chan, p.packet);
    }
}
