//! M5 finish, WP5 part 6 (wp5g): small `FFX.exe` functions the script
//! natives jump to or the event loader queues, which sit in gaps of the
//! dump (Ghidra never split them off). The build without Unicorn stopped
//! at the first of them (`0x009085a0`, in `ffx_field::yi`); these are the
//! rest of that kind the native and helper ports name. Read from the
//! machine code; every one is cdecl and reads no stack word (a native's
//! tail jump hands them `(worker, args, stack)`, which they ignore).

pub mod bridge {
    use ffx_bridge::{Mem, x87};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn global<M: Mem>(m: &M, g: u32) -> u32 {
        m.read(m.rebase(g))
    }

    fn set_global<M: Mem>(m: &mut M, g: u32, v: u32) {
        let p = m.rebase(g);
        m.write(p, v);
    }

    fn game<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        m: &mut M,
        call: &mut C,
        f: u32,
        args: &[u32],
    ) -> u32 {
        let f = m.rebase(f);
        call(m, f, args)
    }

    /// `SndStopStream()` (`0x00887780`).
    pub const SND_STOP_STREAM: u32 = 0x0088_7780;

    /// `0x007b2c00()` (native 0x7066's jump target): [`SND_STOP_STREAM`];
    /// 0.
    pub fn stop_stream_0<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        m: &mut M,
        call: &mut C,
    ) -> u32 {
        game(m, call, SND_STOP_STREAM, &[]);
        0
    }

    /// The event loader's flags byte (`0x01325ba3`), the current ctrl
    /// work pointer (`0x01326b28`), the map-change ctrl work
    /// (`0x01325ba0`) and the three words the "DONE" steps reset
    /// (`0x01326b84` = -1, `0x01326b88`, `0x01326b8c`).
    pub const LOADER_FLAGS: u32 = 0x0132_5ba3;
    pub const CUR_WORK: u32 = 0x0132_6b28;
    pub const MAP_WORK: u32 = 0x0132_5ba0;
    pub const DONE_WORDS: [u32; 3] = [0x0132_6b84, 0x0132_6b88, 0x0132_6b8c];
    /// `Atel_TranslateBGSeps_00861020(a, b, c)`, `AtelClearMenuOtakaraFlag()`,
    /// `Yn_GuideMapSetDispProg(on)`.
    pub const TRANSLATE_BG_SEPS: u32 = 0x0086_1020;
    pub const CLEAR_OTAKARA: u32 = 0x0087_2b60;
    pub const GUIDE_MAP_SET_DISP_PROG: u32 = 0x0091_9d70;

    fn reset_done_words<M: Mem>(m: &mut M) {
        set_global(m, DONE_WORDS[0], u32::MAX);
        set_global(m, DONE_WORDS[1], 0);
        set_global(m, DONE_WORDS[2], 0);
    }

    /// The event loader's "DONE" step (`0x00861480`, queued by the map
    /// change): the flags byte's bits 0..1 cleared; with the current work
    /// pointing at the map-change work, `TranslateBGSeps(0x78, 0x7f, 1)`,
    /// its four floats `+0x218..+0x224` copied to `+0x228..+0x234`
    /// (`fld`/`fstp`: a signalling NaN comes out quiet) and
    /// [`CLEAR_OTAKARA`]; the pointer put back, the done words reset; 1.
    pub fn loader_done<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        m: &mut M,
        call: &mut C,
    ) -> u32 {
        let flags = m.rebase(LOADER_FLAGS);
        let f: u8 = m.read(flags);
        m.write(flags, f & 0xfc);
        let saved = global(m, CUR_WORK);
        let work = m.rebase(MAP_WORK);
        set_global(m, CUR_WORK, work);
        game(m, call, TRANSLATE_BG_SEPS, &[0x78, 0x7f, 1]);
        let w = global(m, CUR_WORK);
        for i in 0..4 {
            let v = f32::from_bits(m.read(at(w, 0x218 + 4 * i)));
            m.write(at(w, 0x228 + 4 * i), x87::round_trip(v).to_bits());
        }
        game(m, call, CLEAR_OTAKARA, &[]);
        set_global(m, CUR_WORK, saved);
        reset_done_words(m);
        1
    }

    /// The event loader's "DONE2" step (`0x00861510`):
    /// `Yn_GuideMapSetDispProg(1)`, `TranslateBGSeps(0x78, 0x7f, 1)`, the
    /// done words reset; 1.
    pub fn loader_done2<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        m: &mut M,
        call: &mut C,
    ) -> u32 {
        game(m, call, GUIDE_MAP_SET_DISP_PROG, &[1]);
        game(m, call, TRANSLATE_BG_SEPS, &[0x78, 0x7f, 1]);
        reset_done_words(m);
        1
    }

    /// `TkGetSaveEventAddress()` (`0x0088c830`) and
    /// `MsGetSaveEventAddress()` (`0x00785240`).
    pub const TK_SAVE_EVENT: u32 = 0x0088_c830;
    pub const MS_SAVE_EVENT: u32 = 0x0078_5240;

    /// `0x0086bcc0()` (native 0x014d's jump target): the index 0..7 of
    /// the largest of the save event's eight signed words `+0x2c..+0x48`
    /// (the first of equal ones; the scan starts from -1, so a word below
    /// that is never taken, and index 0 wins when every word is -1 or
    /// less).
    pub fn save_event_max_index<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        m: &mut M,
        call: &mut C,
    ) -> u32 {
        let e = game(m, call, TK_SAVE_EVENT, &[]);
        let word = |m: &M, i: u32| m.read::<u32>(at(e, 0x2c + 4 * i)).cast_signed();
        let mut best = -1i32;
        let first = word(m, 0);
        if first > best {
            best = first;
        }
        let mut index = 0;
        for i in 1..7 {
            let v = word(m, i);
            if best < v {
                index = i;
                best = v;
            }
        }
        if best < word(m, 7) { 7 } else { index }
    }

    /// The movie start (`code_0076f010(a, b, c, d, e)`) and its four
    /// arguments (`0x01327110`, `0x0132710c`, `0x01327118`, `0x01327114`
    /// in push order reversed).
    pub const MOVIE_START: u32 = 0x0076_f010;
    pub const MOVIE_ARGS: [u32; 4] = [0x0132_7110, 0x0132_710c, 0x0132_7118, 0x0132_7114];

    /// `0x00874060()` (native 0x01b5's jump target): when the save
    /// event's state `+0xe0` is 2..=4, the movie `+0xdc` started with the
    /// four words and the state set to 5; 1. Else 0.
    pub fn texture_video_play<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        m: &mut M,
        call: &mut C,
    ) -> u32 {
        let e = game(m, call, TK_SAVE_EVENT, &[]);
        let state: u32 = m.read(at(e, 0xe0));
        if state.wrapping_sub(2) > 2 {
            return 0;
        }
        let e = game(m, call, TK_SAVE_EVENT, &[]);
        let movie: u32 = m.read(at(e, 0xdc));
        let a = MOVIE_ARGS.map(|g| global(m, g));
        game(m, call, MOVIE_START, &[movie, a[0], a[1], a[2], a[3]]);
        let e = game(m, call, TK_SAVE_EVENT, &[]);
        m.write(at(e, 0xe0), 5u32);
        1
    }

    /// The byte `0x00886430` returns (`0x01330092`).
    pub const SND_BYTE: u32 = 0x0133_0092;

    /// `0x00886430()` (native 0xc040's jump target): the byte at
    /// [`SND_BYTE`], zero-extended.
    pub fn snd_byte<M: Mem>(m: &M) -> u32 {
        u32::from(m.read::<u8>(m.rebase(SND_BYTE)))
    }

    /// `0x008cc140()` (native 0x0266's jump target): bit 0 of the battle
    /// save event's byte `+0x5ec`.
    pub fn save_event_bit_5ec<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        m: &mut M,
        call: &mut C,
    ) -> u32 {
        let e = game(m, call, MS_SAVE_EVENT, &[]);
        u32::from(m.read::<u8>(at(e, 0x5ec)) & 1)
    }

    /// The field scene's sound record list (`0x0193bf80`), and
    /// `rcbgSceneWaveDCStop(id)` / `rcbgSceneWaveC0Stop(id)`.
    pub const SCENE_WAVES: u32 = 0x0193_bf80;
    pub const WAVE_DC_STOP: u32 = 0x0092_0650;
    pub const WAVE_C0_STOP: u32 = 0x0092_1200;

    /// `0x0091a8d0()` / `0x0091a900()` (natives 0x802b / 0x8027, with
    /// `stop` [`WAVE_DC_STOP`] / [`WAVE_C0_STOP`]): without the list 0;
    /// else `stop` of its first record's `+8` (0 without a record); its
    /// `eax`.
    pub fn scene_wave_stop<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        m: &mut M,
        call: &mut C,
        stop: u32,
    ) -> u32 {
        let list = global(m, SCENE_WAVES);
        if list == 0 {
            return 0;
        }
        let rec: u32 = m.read(list);
        let id = if rec == 0 { 0 } else { m.read(at(rec, 8)) };
        game(m, call, stop, &[id])
    }
}
