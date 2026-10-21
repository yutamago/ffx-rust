//! The battle's sound loading and voices (`docs/rewrite-plan.md`, M2),
//! round 32: the sound effect and wave transfers queued from a SEP file
//! (`MsRegistSEdata`, `MsSEPtransRegist`; `MsSEPtransProcess` sends them,
//! [`crate::battle_end`]), a voice line through the scripts
//! (`MsSetVoice`) and a shout stream (`MsSoundStreamPlay`).
//!
//! `dbgPrintf` (`0x0062f500`) is a bare `ret` in this build: the ports
//! leave its calls out, but keep the reads the originals make for its
//! arguments.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::battle_end::bridge::{SEP_TRANS_QUEUE, SEP_TRANS_SCENE, ms_sep_trans_process};
    use crate::exec::bridge::Call;
    use crate::minor::bridge::{SEP_TRANS_STATE, SEP_TRANS_TABLE};
    use crate::state::bridge::SEP_TRANS;

    /// `SndGetSepNumber(sep)` (sound): the number of a SEP block.
    pub const SND_GET_SEP_NUMBER: u32 = 0x0088_6410;
    /// `SndIsReadShoutVoiceStatus()` (sound): a shout still loading.
    pub const SND_IS_READ_SHOUT_VOICE_STATUS: u32 = 0x0088_6640;
    /// `SndStartShoutStream(stream, volume, pan)` (sound).
    pub const SND_START_SHOUT_STREAM: u32 = 0x0088_75d0;
    /// `MsAtelRequestExe(kind, i, n, flags, actor, out)`.
    pub const MS_ATEL_REQUEST_EXE: u32 = 0x0079_7d50;
    /// The shout streams of [`ms_sound_stream_play`]: 8 rows of 8 bytes
    /// (0xff none), by speaker and line (`0x00c42510`, data).
    pub const SHOUT_STREAMS: Global<[[u8; 8]; 8]> = Global::new(0x00c4_2510);
    /// The battle debug menu's shout trace (`dbgPrintf` of the speaker,
    /// line and stream in [`ms_sound_stream_play`]; `0x0112a90b`).
    pub const DEBUG_SHOUT_TRACE: Global<u8> = Global::new(0x0112_a90b);
    /// How many SEP numbers an actor keeps (`+0xeac`, words).
    pub const ACTOR_SEPS: u32 = 0x20;
    /// How many the scene keeps ([`SEP_TRANS_TABLE`]).
    pub const SCENE_SEPS: u32 = 0x80;
    /// Transfers the queue holds.
    pub const QUEUE: u32 = 16;

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        call(mem, f, args)
    }

    fn word(mem: &impl Mem, p: u32, k: u32) -> u32 {
        mem.read(p.wrapping_add(k.wrapping_mul(4)))
    }

    /// `MsSEPtransRegist(actor, data, size, kind)` (`0x007a06b0`): with
    /// the queue full ([`SEP_TRANS`] 16 or more) the transfers go on
    /// (`MsSEPtransProcess`) until one ends; then the record after the
    /// last one queued ([`SEP_TRANS_QUEUE`] at the first one's index,
    /// [`SEP_TRANS_STATE`], plus the count; the original's signed `% 16`)
    /// gets `data`, `size`, the kind (`+8`, byte), the actor (`+9`, byte;
    /// 0xff the scene's) and state 0 (`+0xa`), and the count goes up.
    /// `eax`: `size` with the kind in its low byte (never read: its one
    /// caller drops it).
    pub fn ms_sep_trans_regist<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        actor: u32,
        data: u32,
        size: u32,
        kind: u32,
    ) -> u32 {
        while SEP_TRANS.get(mem).cast_signed() >= QUEUE.cast_signed() {
            ms_sep_trans_process(mem, &mut *call);
        }
        let n = SEP_TRANS.get(mem);
        let first = SEP_TRANS_STATE.get(mem)[0];
        let k = first.wrapping_add(n).cast_signed() % QUEUE.cast_signed();
        SEP_TRANS.set(mem, n.wrapping_add(1));
        let r = SEP_TRANS_QUEUE
            .ptr(mem)
            .addr
            .wrapping_add(k.wrapping_mul(12).cast_unsigned());
        mem.write(r, data);
        mem.write(r.wrapping_add(4), size);
        mem.write(r.wrapping_add(9), actor.to_le_bytes()[0]);
        mem.write(r.wrapping_add(8), kind.to_le_bytes()[0]);
        mem.write(r.wrapping_add(0xa), 0u8);
        size & !0xff | kind & 0xff
    }

    /// The first and last offsets of a SEP file section's blocks (the
    /// words after its count; 0 none): the first non-zero one and how many
    /// are non-zero.
    fn blocks(mem: &impl Mem, section: u32, count: i32) -> (u32, u32) {
        let (mut first, mut used) = (0u32, 0u32);
        for k in 0..count.cast_unsigned() {
            let o = word(mem, section, k + 1);
            if o != 0 {
                used += 1;
                if first == 0 {
                    first = o;
                }
            }
        }
        (first, used)
    }

    /// `MsRegistSEdata(actor, sep)` (`0x007a01b0`): the sound file `sep`
    /// (a count, then that many section offsets from `sep`, 0 none) of an
    /// actor (`actor` its record, 0 the scene's). Each section is a count
    /// and that many block offsets (0 none) and one more, the end. Section
    /// 0 holds the sound effects: each block's number
    /// (`SndGetSepNumber(sep + offset)`, through the game) goes to the
    /// actor's `+0xeac` words (the first 32) or [`SEP_TRANS_TABLE`] (the
    /// first 128) at the block's index. A section with blocks is queued as
    /// one transfer ([`ms_sep_trans_regist`]) from its first block to the
    /// end, kind = the section's index, with the actor's `+0xc` (its
    /// transfers `+0xf2e` and `+0xf2d` counted up) or 0xff for the scene
    /// ([`SEP_TRANS_SCENE`] up). -1 without a file, else 0.
    pub fn ms_regist_se_data<M: Mem>(mem: &mut M, call: Call<'_, M>, actor: u32, sep: u32) -> i32 {
        if sep == 0 {
            return -1;
        }
        let sections: i32 = mem.read(sep);
        for s in 0..sections.max(0).cast_unsigned() {
            let o = word(mem, sep, s + 1);
            if o == 0 {
                continue;
            }
            let section = sep.wrapping_add(o);
            let count: i32 = mem.read(section);
            if count <= 0 {
                continue;
            }
            let (first, used) = blocks(mem, section, count);
            if s == 0 {
                for k in 0..count.cast_unsigned() {
                    let b = word(mem, section, k + 1);
                    let at = if actor == 0 {
                        (k < SCENE_SEPS).then(|| SEP_TRANS_TABLE.ptr(mem).addr.wrapping_add(4 * k))
                    } else {
                        (k < ACTOR_SEPS).then(|| actor.wrapping_add(0xeac + 4 * k))
                    };
                    if b != 0
                        && let Some(at) = at
                    {
                        let n = game(mem, &mut *call, SND_GET_SEP_NUMBER, &[sep.wrapping_add(b)]);
                        mem.write(at, n);
                    }
                }
            }
            if used == 0 {
                continue;
            }
            let end = word(mem, section, count.cast_unsigned() + 1);
            let size = end.wrapping_sub(first);
            let data = sep.wrapping_add(first);
            let who = if actor == 0 {
                SEP_TRANS_SCENE.set(mem, SEP_TRANS_SCENE.get(mem).wrapping_add(1));
                0xff
            } else {
                for c in [0xf2e, 0xf2d] {
                    let v: u8 = mem.read(actor.wrapping_add(c));
                    mem.write(actor.wrapping_add(c), v.wrapping_add(1));
                }
                u32::from(mem.read::<u8>(actor.wrapping_add(0xc)))
            };
            ms_sep_trans_regist(mem, &mut *call, who, data, size, s);
            // (dbgPrintf's argument)
            let _: u8 = mem.read(data);
        }
        0
    }

    /// `MsSetVoice(i, line, flags, actor, out)` (`0x007a0720`): a voice
    /// line through the battle scripts: for a line not negative, the
    /// request queue reset (`MsAtelRequestInit`) and four requests of kind
    /// 2 for the line: of the speaker (`0x4f + i`) and of the voice script
    /// (0x4e for a monster, else 0x4d), each once with 1 and once with 0;
    /// then `MsAtelRequestExe(2, 0, 0x4f + i, flags, actor, out)` (through
    /// the game): 1 when that was not negative, else 0. (`MsGetChr(i)` is
    /// called and dropped.)
    pub fn ms_set_voice<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        line: u32,
        [flags, actor, out]: [u32; 3],
    ) -> u32 {
        use crate::state::bridge::{ms_atel_request_add, ms_atel_request_init};
        let voice = 0x4d + crate::actor::bridge::ms_get_ram_chr_monster(i);
        if line.cast_signed() < 0 {
            return 0;
        }
        let speaker = i.wrapping_add(0x4f);
        // (the request words are 16 bits)
        let low = |v: u32| {
            let [b0, b1, _, _] = v.to_le_bytes();
            u16::from_le_bytes([b0, b1])
        };
        ms_atel_request_init(mem);
        for on in [1, 0] {
            ms_atel_request_add(mem, [2, on, low(speaker), low(line)]);
            ms_atel_request_add(mem, [2, on, low(voice), low(line)]);
        }
        let r = game(
            mem,
            call,
            MS_ATEL_REQUEST_EXE,
            &[2, 0, speaker, flags, actor, out],
        );
        u32::from(r.cast_signed() >= 0)
    }

    /// `MsSoundStreamPlay(speaker, line)` (`0x007a0820`): a shout, unless
    /// one is still loading (`SndIsReadShoutVoiceStatus`, -1): a negative
    /// speaker plays stream `line` itself; speakers 0-7 with lines 0-7
    /// play [`SHOUT_STREAMS`]' entry (none for 0xff), others -1. The
    /// stream starts with `SndStartShoutStream(stream, 0x3f, 0x7f)`
    /// (through the game). 0 once past the checks.
    pub fn ms_sound_stream_play<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        speaker: i32,
        line: u32,
    ) -> i32 {
        if game(mem, &mut *call, SND_IS_READ_SHOUT_VOICE_STATUS, &[]) != 0 {
            return -1;
        }
        if speaker < 0 {
            game(mem, call, SND_START_SHOUT_STREAM, &[line, 0x3f, 0x7f]);
            return 0;
        }
        if speaker >= 8 || line > 7 {
            return -1;
        }
        let at = SHOUT_STREAMS
            .ptr(mem)
            .addr
            .wrapping_add(speaker.cast_unsigned() * 8 + line);
        let stream: u8 = mem.read(at);
        if stream != 0xff {
            game(
                mem,
                call,
                SND_START_SHOUT_STREAM,
                &[u32::from(stream), 0x3f, 0x7f],
            );
        }
        0
    }
}
