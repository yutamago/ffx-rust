//! M5 stage 1, list `small`, round 4: the FMOD glue left over
//! (`FmodSfx::unloadProjectBySeq`, `FmodShout::start`).

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers and indices
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    /// `Phyre_PRedBlackTree_successor(tree; node)` and
    /// `Phyre_PRedBlackTree_remove(tree; node)` (thiscall, `ret 4`).
    pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
    pub const TREE_REMOVE: u32 = 0x0057_5000;
    /// `Phyre_PFreeList_deallocate(list; item)` (thiscall, `ret 4`).
    pub const FREELIST_DEALLOCATE: u32 = 0x0049_c980;
    /// `SfxBank_appendEventById(bank; seq, out)` (thiscall, `ret 8`) -> `al`.
    pub const APPEND_EVENT_BY_ID: u32 = 0x0070_fa80;
    /// `SfxBank_stopAllEvents(bank; immediate)` (thiscall, `ret 4`).
    pub const STOP_ALL_EVENTS: u32 = 0x0070_fcd0;
    /// `SfxBank_getNumPlayingEvents(bank)` (thiscall).
    pub const PLAYING_EVENTS: u32 = 0x0070_fb20;
    /// `SfxBank_dtor_SfxBank(bank)` (thiscall).
    pub const BANK_DTOR: u32 = 0x0070_f930;
    /// `operator delete(p)`, `user_free_thunk(p)` (cdecl).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `vfxIntTree_add(tree; &key, &value)` (thiscall, `ret 8`).
    pub const INT_TREE_ADD: u32 = 0x0070_cd60;

    /// `FmodSfx_unloadProjectBySeq(sfx; seq)` (`0x0070dbb0`, thiscall,
    /// `ret 4`). `list` is the original's event list (`ebp - 0x1c`, three
    /// words, zeroed here, filled by `SfxBank_appendEventById`) and `slot`
    /// its own `seq` slot (`ebp + 8`), which gets the bank whose address
    /// goes to `vfxIntTree_add`.
    ///
    /// The bank tree (head `sfx + 0x34`, root `+0x40`, key `+0xc`, bank
    /// `+0x10`) is walked from its leftmost node until a bank has events
    /// of `seq`; that bank stops its events, is destroyed when none plays
    /// on (else parked in the tree `sfx + 0x94` under its key), and its
    /// node leaves the tree. The list's third word is freed at the end.
    /// Returns `user_free_thunk`'s `eax`, or 0.
    pub fn unload_project_by_seq(
        h: &mut impl Host,
        sfx: u32,
        seq: u32,
        list: u32,
        slot: u32,
    ) -> u32 {
        for i in 0..3 {
            h.write(at(list, 4 * i), 0u32);
        }
        let head = at(sfx, 0x34);
        let root: u32 = h.read(at(head, 0xc));
        let mut node = root;
        if root != head && root != 0 {
            let mut p = root;
            while p != head {
                node = p;
                p = h.read(p);
            }
        }
        if node != head {
            let mut next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(head), &[node]);
            loop {
                let bank: u32 = h.read(at(node, 0x10));
                let found = h.call_sym(APPEND_EVENT_BY_ID, Conv::Thiscall(bank), &[seq, list]);
                if found & 0xff != 0 {
                    remove_bank(h, sfx, head, node, slot);
                    break;
                }
                node = next;
                if next != head {
                    next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(head), &[next]);
                }
                if node == head {
                    break;
                }
            }
        }
        let third: u32 = h.read(at(list, 8));
        if third == 0 {
            return 0;
        }
        h.call_sym(USER_FREE, Conv::Cdecl, &[third])
    }

    fn remove_bank(h: &mut impl Host, sfx: u32, head: u32, node: u32, slot: u32) {
        let bank: u32 = h.read(at(node, 0x10));
        h.write(slot, bank);
        h.call_sym(STOP_ALL_EVENTS, Conv::Thiscall(bank), &[0]);
        if h.call_sym(PLAYING_EVENTS, Conv::Thiscall(bank), &[]) == 0 {
            if bank != 0 {
                h.call_sym(BANK_DTOR, Conv::Thiscall(bank), &[]);
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[bank]);
            }
        } else {
            h.call_sym(
                INT_TREE_ADD,
                Conv::Thiscall(at(sfx, 0x94)),
                &[at(node, 0xc), slot],
            );
        }
        let key: i32 = h.read(at(node, 0xc));
        let mut p: u32 = h.read(at(head, 0xc));
        while p != head {
            let k: i32 = h.read(at(p, 0xc));
            if k == key {
                h.call_sym(TREE_REMOVE, Conv::Thiscall(head), &[p]);
                h.call_sym(FREELIST_DEALLOCATE, Conv::Thiscall(at(head, 0x18)), &[p]);
                return;
            }
            // left (+0) when the node's key is larger, else right (+4)
            p = h.read(if k > key { p } else { at(p, 4) });
        }
    }

    /// FMOD's `Sound::getSubSound(sound, i, &sub)`, `Sound::getName(sound,
    /// buf, n)` and `System::playSound(system, index, sound, paused,
    /// &channel)` (import thunks, stdcall with `this` first).
    pub const GET_SUB_SOUND: u32 = 0x0094_8d00;
    pub const GET_NAME: u32 = 0x0094_8d06;
    pub const PLAY_SOUND: u32 = 0x0094_8cf4;
    /// `ERRCHECK(result, file, line)` (cdecl) and the file name it gets.
    pub const ERRCHECK: u32 = 0x0070_79d0;
    pub const FMOD_SHOUT_FILE: u32 = 0x00b4_fd34;
    /// `memset`'s thunk (cdecl).
    pub const MEMSET: u32 = 0x0094_964c;
    /// `g_FmodMusicNoLoad`.
    pub const NO_LOAD: u32 = 0x00ce_c174;

    /// The arguments of [`shout_start`] after `this`.
    #[derive(Clone, Copy, Debug)]
    pub struct ShoutStart {
        /// The name id looked up in the `ML` table.
        pub id: u32,
        /// The FMOD system.
        pub system: u32,
        /// 1 skips the start.
        pub mute: u32,
        /// Two status words zeroed on start.
        pub status0: u32,
        pub status1: u32,
        /// The `ML` blob: `"ML\0\0"`, a count, then `(id, name offset)`
        /// pairs.
        pub blob: u32,
    }

    /// `FmodShout_start(shout; id, _, _, system, mute, &status0, &status1,
    /// blob)` (`0x0070e480`, thiscall, `ret 0x20`; the second and third
    /// words are not read). `buf` is the original's 256-byte name buffer
    /// (`ebp - 0x104`), which `Sound::getName` fills.
    ///
    /// With a system, not muted, a sound (`+0`) and sub-sounds (`+0xc`)
    /// and `g_FmodMusicNoLoad` not 1: the status words are zeroed; for an
    /// `ML` blob holding `id`, the sub-sounds are fetched into `+4` in
    /// turn until one's name is the table's (the others leave `+4` 0).
    /// Then `+4`, when set, is played on a free channel into `+8`.
    /// Returns 0.
    pub fn shout_start(h: &mut impl Host, shout: u32, a: ShoutStart, buf: u32) -> u32 {
        if a.system == 0
            || h.read::<u32>(h.rebase(NO_LOAD)) == 1
            || a.mute == 1
            || h.read::<u32>(shout) == 0
            || h.read::<u32>(at(shout, 0xc)) == 0
        {
            return 0;
        }
        h.write(a.status0, 0u32);
        h.write(a.status1, 0u32);
        let blob = a.blob;
        let file = h.rebase(FMOD_SHOUT_FILE);
        if h.read::<u8>(blob) == b'M'
            && h.read::<u8>(at(blob, 1)) == b'L'
            && h.read::<u8>(at(blob, 2)) == 0
            && h.read::<u8>(at(blob, 3)) == 0
        {
            h.write(buf, 0u8);
            h.call_sym(MEMSET, Conv::Cdecl, &[at(buf, 1), 0, 0xff]);
            let n: u32 = h.read(at(blob, 4));
            let mut e = at(blob, 8);
            let mut found = None;
            for _ in 0..n {
                if h.read::<u32>(e) == a.id {
                    found = Some(h.read::<u32>(at(e, 4)));
                    break;
                }
                e = at(e, 8);
            }
            if let Some(off) = found {
                let name = at(blob, off);
                let mut i = 0i32;
                while i < h.read::<i32>(at(shout, 0xc)) {
                    let sound: u32 = h.read(shout);
                    #[allow(clippy::cast_sign_loss)] // the sub-sound index
                    let r = h.call_sym(
                        GET_SUB_SOUND,
                        Conv::Stdcall,
                        &[sound, i as u32, at(shout, 4)],
                    );
                    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0xb5]);
                    let sub: u32 = h.read(at(shout, 4));
                    let r = h.call_sym(GET_NAME, Conv::Stdcall, &[sub, buf, 0x100]);
                    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0xb8]);
                    if same_string(h, buf, name) {
                        break;
                    }
                    h.call_sym(MEMSET, Conv::Cdecl, &[buf, 0, 0x100]);
                    h.write(at(shout, 4), 0u32);
                    i += 1;
                }
            }
        }
        let sub: u32 = h.read(at(shout, 4));
        if sub != 0 {
            let r = h.call_sym(
                PLAY_SOUND,
                Conv::Stdcall,
                &[a.system, 0xffff_fffe, sub, 0, at(shout, 8)],
            );
            h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0xcc]);
        }
        0
    }

    /// `strcmp(a, b) == 0`.
    fn same_string(h: &impl Host, a: u32, b: u32) -> bool {
        let mut i = 0;
        loop {
            let (x, y) = (h.read::<u8>(at(a, i)), h.read::<u8>(at(b, i)));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            i += 1;
        }
    }
}
