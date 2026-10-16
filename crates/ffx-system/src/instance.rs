//! Singletons and small object accessors of the PC port's managers
//! (clusters, the asset loader, the archive stream, the magic file, asset
//! lists, the ability map, the character cache, the loading screen).

/// `AssetListReader::getAssetCount` for an asset vector `[begin, end)`:
/// the 24-byte records between them (a signed division, toward zero).
#[must_use]
pub const fn asset_count(begin: u32, end: u32) -> i32 {
    end.wrapping_sub(begin).cast_signed() / 24
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `ClusterManager::GetInstance`'s object.
    pub const CLUSTER_MANAGER: Global<u32> = Global::new(0x00cc_ca4c);
    /// `AsyncLoadManager::sGetInstance`'s object.
    pub const ASYNC_LOAD_MANAGER: Global<u32> = Global::new(0x00cc_c88c);
    /// `BigFileStream_sBigFileStreamInstacne` (sic).
    pub const BIG_FILE_STREAM: Global<u32> = Global::new(0x00cc_9c48);
    /// The magic (spell effect) file's ID, -1 when none.
    pub const MAGIC_ID: Global<u32> = Global::new(0x00c6_4ca0);
    /// The magic file's handle, -1 when none.
    pub const MAGIC_HANDLE: Global<u32> = Global::new(0x00c6_4cac);
    /// Whether the magic file is NEX data.
    pub const MAGIC_IS_NEX: Global<u32> = Global::new(0x0193_fdac);

    /// `ClusterManager_GetInstance()` (`0x0069a1a0`).
    pub fn cluster_manager(mem: &impl Mem) -> u32 {
        CLUSTER_MANAGER.get(mem)
    }

    /// `AsyncLoadManager_sGetInstance()` (`0x00691b40`).
    pub fn async_load_manager(mem: &impl Mem) -> u32 {
        ASYNC_LOAD_MANAGER.get(mem)
    }

    /// `AsyncLoadManager::isAllRequestDone` (`0x00691630`, `this` in ECX):
    /// both request counts (`+0x20`, `+0x2c`) zero.
    pub fn is_all_request_done(mem: &impl Mem, this: u32) -> bool {
        mem.read::<u32>(this.wrapping_add(0x20)) == 0
            && mem.read::<u32>(this.wrapping_add(0x2c)) == 0
    }

    /// `BigFileStream_get()` (`0x0061bdb0`).
    pub fn big_file_stream(mem: &impl Mem) -> u32 {
        BIG_FILE_STREAM.get(mem)
    }

    /// `MagicFile_GetMagicId()` (`0x009da3b0`).
    pub fn magic_id(mem: &impl Mem) -> u32 {
        MAGIC_ID.get(mem)
    }

    /// `MagicFile_GetHandle()` (`0x009da390`).
    pub fn magic_handle(mem: &impl Mem) -> u32 {
        MAGIC_HANDLE.get(mem)
    }

    /// `MagicFile_IsNexData()` (`0x009da420`).
    pub fn magic_is_nex(mem: &impl Mem) -> u32 {
        MAGIC_IS_NEX.get(mem)
    }

    /// `AssetListReader::getAssetCount` (`0x006f4170`, `this` in ECX): the
    /// asset vector at `+0x14`.
    pub fn asset_count(mem: &impl Mem, this: u32) -> i32 {
        super::asset_count(
            mem.read(this.wrapping_add(0x14)),
            mem.read(this.wrapping_add(0x18)),
        )
    }

    /// An MSVC `std::string` at `s`: its characters, inline while the
    /// capacity (`+0x14`) is under 16, else through the pointer at `+0`.
    fn string_chars(mem: &impl Mem, s: u32) -> u32 {
        if mem.read::<u32>(s.wrapping_add(0x14)) < 0x10 {
            s
        } else {
            mem.read(s)
        }
    }

    /// `AssetListReader::getAssetPath(i)` (`0x006f41b0`, thiscall): asset
    /// `i`'s path (a `std::string` first in each 24-byte record), or null
    /// out of range.
    pub fn asset_path(mem: &impl Mem, this: u32, i: i32) -> u32 {
        if i < 0 || i >= asset_count(mem, this) {
            return 0;
        }
        let begin: u32 = mem.read(this.wrapping_add(0x14));
        string_chars(mem, begin.wrapping_add(i.cast_unsigned().wrapping_mul(24)))
    }

    /// `AssetListReader::isModel(id)` (`0x006f46b0`, thiscall): `id` is in
    /// the model ID vector at `+8`.
    pub fn is_model(mem: &impl Mem, this: u32, id: u32) -> bool {
        let mut p: u32 = mem.read(this.wrapping_add(8));
        let end: u32 = mem.read(this.wrapping_add(0xc));
        while p != end {
            if mem.read::<u32>(p) == id {
                return true;
            }
            p = p.wrapping_add(4);
        }
        false
    }

    /// `AssetListReader::getSymbolById(id)` (`0x006f42d0`, thiscall): the
    /// name of symbol `id` in the `std::map` whose head `this` points to
    /// (nodes: left `+0`, parent `+4`, right `+8`, nil flag `+0xd`, name
    /// `+0x10`, ID `+0x28`), walked in order; null when absent.
    pub fn symbol_by_id(mem: &impl Mem, this: u32, id: u32) -> u32 {
        let head: u32 = mem.read(this);
        let nil = |n: u32| mem.read::<u8>(n.wrapping_add(0xd)) != 0;
        let mut node: u32 = mem.read(head);
        while node != head {
            if mem.read::<u32>(node.wrapping_add(0x28)) == id {
                return string_chars(mem, node.wrapping_add(0x10));
            }
            if !nil(node) {
                let right: u32 = mem.read(node.wrapping_add(8));
                if nil(right) {
                    // up while coming from the right
                    let mut child = node;
                    let mut up: u32 = mem.read(node.wrapping_add(4));
                    while !nil(up) && child == mem.read::<u32>(up.wrapping_add(8)) {
                        child = up;
                        up = mem.read(up.wrapping_add(4));
                    }
                    node = up;
                } else {
                    // the leftmost below the right child
                    node = right;
                    loop {
                        let left: u32 = mem.read(node);
                        if nil(left) {
                            break;
                        }
                        node = left;
                    }
                }
            }
        }
        0
    }

    /// `AbmapManager::ToDestroy()` (`0x006857b0`, `this` in ECX): state 2.
    pub fn abmap_to_destroy(mem: &mut impl Mem, this: u32) {
        mem.write(this.wrapping_add(4), 2u32);
    }

    /// `ChrResourceCacheManager::isChrDelayedRelease()` (`0x0069b7b0`,
    /// `this` in ECX): the mode at `+0` is not 2.
    pub fn is_chr_delayed_release(mem: &impl Mem, this: u32) -> u32 {
        u32::from(mem.read::<u32>(this) != 2)
    }

    /// The new-character load counter (`AsyncLoadNewCharacter`; negative
    /// = off).
    pub const NEW_CHARACTERS: Global<i32> = Global::new(0x00c4_21dc);

    /// `AsyncLoadNewCharacter()` (`0x00782f80`): counts one when the
    /// counter is on; returns 0.
    pub fn async_load_new_character(mem: &mut impl Mem) -> u32 {
        let n = NEW_CHARACTERS.get(mem);
        if n >= 0 {
            NEW_CHARACTERS.set(mem, n.wrapping_add(1));
        }
        0
    }

    /// `LoadingScreen::setParameterManually(&a, &b, &c)` (`0x006e4ff0`,
    /// thiscall): each non-null one into `+4` (`f32`), `+0x10` (copied
    /// whole), `+0x14` (`f32`).
    pub fn loading_screen_set_parameter(mem: &mut impl Mem, this: u32, a: u32, b: u32, c: u32) {
        if a != 0 {
            let v = ffx_bridge::x87::round_trip(mem.read(a));
            mem.write(this.wrapping_add(4), v);
        }
        if b != 0 {
            let v: u32 = mem.read(b);
            mem.write(this.wrapping_add(0x10), v);
        }
        if c != 0 {
            let v = ffx_bridge::x87::round_trip(mem.read(c));
            mem.write(this.wrapping_add(0x14), v);
        }
    }

    /// `abmCalcChrMoveCurMoveCheck(chr)` (`0x00a45000`): bit 0 of `+0x22`.
    pub fn chr_move_cur_move_check(mem: &impl Mem, chr: u32) -> u32 {
        u32::from(mem.read::<u8>(chr.wrapping_add(0x22)) & 1)
    }

    /// `Virtuos::InputManager`'s object (`getInstance` returns its address).
    pub const INPUT_MANAGER: Global<u32> = Global::new(0x00cc_b170);
    /// `MorphManager::GetInstance`'s object pointer.
    pub const MORPH_MANAGER: Global<u32> = Global::new(0x00cd_ede8);
    /// `TrophyManager::Instance`'s object pointer.
    pub const TROPHY_MANAGER: Global<u32> = Global::new(0x00cc_c7cc);

    /// `Virtuos::InputManager::getInstance()` (`0x00631300`).
    pub fn input_manager(mem: &impl Mem) -> u32 {
        INPUT_MANAGER.ptr(mem).addr
    }

    /// `Virtuos::InputManager::getVibrationFlag()` (`0x00631320`, `this`
    /// in ECX): `+0xa8` non-zero.
    pub fn vibration_flag(mem: &impl Mem, this: u32) -> bool {
        mem.read::<u32>(this.wrapping_add(0xa8)) != 0
    }

    /// The key test at `0x00629ca0` (`thiscall(manager, key, pad)`, `ret
    /// 8`): whether key `key` (11..=26) of pad `pad` (< 0x12) is down, the
    /// pad's state at `+0x18 + key - 11` of the pad object
    /// `[manager+0x2b8]+0x38+4 pad`. The whole `eax`: the result in `al`
    /// over `key - 11` when the key is looked at, else over what the
    /// original had there (the pad table, or `key - 11` when out of range).
    pub fn key_down(mem: &impl Mem, this: u32, key: u32, pad: u32) -> u32 {
        let table: u32 = mem.read(this.wrapping_add(0x2b8));
        if pad >= 0x12 {
            return table & !0xff;
        }
        let p: u32 = mem.read(table.wrapping_add(0x38).wrapping_add(4 * pad));
        if p == 0 {
            return table & !0xff;
        }
        let k = key.wrapping_sub(11);
        if k >= 0x10 {
            return k & !0xff;
        }
        let down = mem.read::<u8>(p.wrapping_add(0x18).wrapping_add(k)) != 0;
        k & !0xff | u32::from(down)
    }

    /// `MorphManager::GetInstance()` (`0x006dc8a0`).
    pub fn morph_manager(mem: &impl Mem) -> u32 {
        MORPH_MANAGER.get(mem)
    }

    /// `TrophyManager::Instance()` (`0x0067ac90`).
    pub fn trophy_manager(mem: &impl Mem) -> u32 {
        TROPHY_MANAGER.get(mem)
    }

    /// `TouchManager::GetInstance`'s object pointer.
    pub const TOUCH_MANAGER: Global<u32> = Global::new(0x00cc_c850);
    /// The achievement manager (`SaveDataUnlockAchievement` asks for it).
    pub const ACHIEVEMENT_MANAGER: Global<u32> = Global::new(0x00ce_81a8);
    /// The magic file's ID to delete (`MagicFile_GetToBeDeleteMagicId`).
    pub const MAGIC_TO_DELETE: Global<u32> = Global::new(0x00c6_4ca8);

    /// The HDD word `get_hdd_word_518` returns (`TkRefreshHdd` reads it).
    pub const HDD_WORD_518: Global<u32> = Global::new(0x0112_a518);
    /// The words the unnamed `TOGet` getters return, by getter address:
    /// `0x008ac2d0`, `0x008ac3f0`, `0x008ac790`, `0x008acf10`.
    pub const TO_GET_WORDS: [Global<u32>; 4] = [
        Global::new(0x0184_1e08),
        Global::new(0x0184_1d98),
        Global::new(0x0184_1e00),
        Global::new(0x0184_1d9c),
    ];

    /// A getter of one global word: `TouchManager_GetInstance`
    /// (`0x006898c0`), `AchievementManager_GetInstance` (`0x006f15b0`),
    /// `MagicFile_GetToBeDeleteMagicId` (`0x009da3d0`), `get_hdd_word_518`
    /// (`0x007787b0`), the `TOGet` getters at `0x008ac2d0`, `0x008ac3f0`,
    /// `0x008ac790`, `0x008acf10`.
    pub fn word(mem: &impl Mem, g: Global<u32>) -> u32 {
        g.get(mem)
    }

    /// `0x006311b0` (`thiscall(this, i)`, `ret 4`): word `i` of the table
    /// at `this + 0x60`.
    pub fn table_60(mem: &impl Mem, this: u32, i: u32) -> u32 {
        mem.read(this.wrapping_add(0x60).wrapping_add(i.wrapping_mul(4)))
    }

    /// `0x006db870(n, _)` (stdcall, `ret 8`): `n / 8` (signed, toward
    /// zero) above 8, else 1.
    #[must_use]
    pub const fn eighth_at_least_one(n: i32) -> i32 {
        if n > 8 { n / 8 } else { 1 }
    }

    /// `LocalizationManager_GetInstance`'s object pointer.
    pub const LOCALIZATION_MANAGER: Global<u32> = Global::new(0x00cd_ed58);
    /// "sLanguageManager singleton is not created yet." (`.rdata`).
    pub const NOT_CREATED_MESSAGE: Global<u8> = Global::new(0x00b4_a1dc);

    /// `LocalizationManager_GetInstance()` (`0x006db070`): the language
    /// manager; while it is null, first `Phyre_PhyrePrintf(1, "...not
    /// created yet.")` through `print(level, format)` (the host's printf),
    /// then read again.
    pub fn localization_manager(mem: &impl Mem, print: &mut impl FnMut(u32, u32)) -> u32 {
        let m = LOCALIZATION_MANAGER.get(mem);
        if m != 0 {
            return m;
        }
        print(1, NOT_CREATED_MESSAGE.ptr(mem).addr);
        LOCALIZATION_MANAGER.get(mem)
    }

    /// `TOGetFFXLang()` (`0x006410e0`): the language (`+4` of the language
    /// manager: 0 jp, 1 us, 2 fr, 3 sp, 4 de, 5 it, 9 kr, 10 ch), 0 while
    /// there is no manager; the manager is asked twice, as the original
    /// does.
    pub fn ffx_lang(mem: &impl Mem, print: &mut impl FnMut(u32, u32)) -> u32 {
        if localization_manager(mem, print) == 0 {
            return 0;
        }
        let m = localization_manager(mem, print);
        mem.read(m.wrapping_add(4))
    }

    /// `TOGetBtlMenuTexAdrs()`'s word.
    pub const BTL_MENU_TEX: Global<u32> = Global::new(0x0184_1e04);
    /// `TOMenuGetYesNoWindow()`'s word.
    pub const YES_NO_WINDOW: Global<u32> = Global::new(0x0184_1c2c);

    /// The entry table `0x00774740` reads (`0x0112a01c`): 8 bytes per
    /// entry, an offset from [`RECORD_BASE`] and an i16 record count.
    pub const RECORD_ENTRIES: Global<u32> = Global::new(0x0112_a01c);
    /// Added to an entry's offset (`0x0112a03c`).
    pub const RECORD_BASE: Global<u32> = Global::new(0x0112_a03c);
    /// The current entry (`0x0112a088`).
    pub const RECORD_ENTRY: Global<u32> = Global::new(0x0112_a088);

    /// `0x00774740()`: the current entry's records (0x138 bytes each)
    /// reset: the halves at `+0x120`, `+0x122` copied to `+0x114`,
    /// `+0x116`, the floats `+0x118`, `+0x11c` zeroed. Returns what `eax`
    /// ends as: the last record's half at `+0x122`, or the table pointer
    /// when there are none.
    pub fn reset_records(mem: &mut impl Mem) -> u32 {
        let table = RECORD_ENTRIES.get(mem);
        let entry = table.wrapping_add(RECORD_ENTRY.get(mem).wrapping_mul(8));
        let count: i16 = mem.read(entry.wrapping_add(4));
        let mut rec = mem.read::<u32>(entry).wrapping_add(RECORD_BASE.get(mem));
        let mut eax = table;
        for _ in 0..count.max(0) {
            let (x, y): (u16, u16) = (
                mem.read(rec.wrapping_add(0x120)),
                mem.read(rec.wrapping_add(0x122)),
            );
            mem.write(rec.wrapping_add(0x114), x);
            mem.write(rec.wrapping_add(0x116), y);
            mem.write(rec.wrapping_add(0x118), 0u64);
            eax = u32::from(y);
            rec = rec.wrapping_add(0x138);
        }
        eax
    }

    /// `0x00641100()`: the first word of the localization manager (through
    /// `LocalizationManager_GetInstance`, twice, as the original), or 0
    /// when there is none.
    pub fn localization_manager_word0(mem: &impl Mem, print: &mut impl FnMut(u32, u32)) -> u32 {
        if localization_manager(mem, print) == 0 {
            return 0;
        }
        mem.read(localization_manager(mem, print))
    }

    /// The word table `0x00402be0` reads (`0x00c8a538`).
    pub const TABLE_00C8A538: Global<u32> = Global::new(0x00c8_a538);
    /// The word table `0x008bdfc0` writes (`0x02310f20`).
    pub const TABLE_02310F20: Global<u32> = Global::new(0x0231_0f20);

    /// `0x00402be0(i)` (`ret 4`): word `i` of [`TABLE_00C8A538`].
    pub fn table_00c8a538(mem: &impl Mem, i: u32) -> u32 {
        mem.read(TABLE_00C8A538.ptr(mem).addr.wrapping_add(i.wrapping_mul(4)))
    }

    /// `0x008bdfc0(i, v)`: word `i` of [`TABLE_02310F20`] set; returns `i`.
    pub fn set_table_02310f20(mem: &mut impl Mem, i: u32, v: u32) -> u32 {
        let at = TABLE_02310F20.ptr(mem).addr.wrapping_add(i.wrapping_mul(4));
        mem.write(at, v);
        i
    }

    /// `0x009dad70(id, &out)`: with the magic PRX preload table on
    /// (`0x0193fd40`), the first of the 12 entries (12 bytes from
    /// `0x00c64c18`) with a positive count (`0x0193fd44 + 4 i`) whose id is
    /// `id` (any id when `id` is negative) is found: its count's address to
    /// `*out` (when given), 1. Else 0.
    pub fn find_preloaded(mem: &mut impl Mem, id: i32, out: u32) -> u32 {
        if Global::<u32>::new(0x0193_fd40).get(mem) == 0 {
            return 0;
        }
        let ids = mem.rebase(0x00c6_4c18);
        let counts = mem.rebase(0x0193_fd44);
        for i in 0..12u32 {
            if id >= 0 && mem.read::<i32>(ids + 12 * i) != id {
                continue;
            }
            if mem.read::<i32>(counts + 4 * i) > 0 {
                if out != 0 {
                    mem.write(out, counts + 4 * i);
                }
                return 1;
            }
        }
        0
    }
}
