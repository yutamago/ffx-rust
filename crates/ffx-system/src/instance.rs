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
}
