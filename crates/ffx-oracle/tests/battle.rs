//! The battle code's `Ms*` functions (`ffx_battle::{save, actor, camera,
//! state, util}`) against the original, on two oracles started alike: one
//! runs `FFX.exe`'s code, the other the Rust bridge on the same memory;
//! results and every written byte must agree.
//!
//! Each group runs on the image alone (synthetic memory) and, with
//! `FFX_SNAPSHOT`, on a snapshot of the running game: first on the game's
//! own state, then on synthetic values written over it. Needs
//! `FFX_GAME_DIR`; skipped otherwise.

// a and b are the two oracles throughout; the checks are long lists
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

use std::path::PathBuf;

use ffx_battle::{actor, camera, save, state, util};
use ffx_bridge::Global;
use ffx_oracle::{Conv, Oracle, Pages};

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

/// The two oracles on the image alone, the x87 as on the game thread
/// (`0x027f`: 53 bits, exceptions masked).
fn synthetic() -> Option<(Oracle, Oracle)> {
    let exe = exe()?;
    let (mut a, mut b) = (Oracle::load(&exe).unwrap(), Oracle::load(&exe).unwrap());
    a.set_fpu_control(0x027f).unwrap();
    b.set_fpu_control(0x027f).unwrap();
    Some((a, b))
}

/// The two oracles on `FFX_SNAPSHOT`.
fn snapshot() -> Option<(Oracle, Oracle)> {
    let exe = exe()?;
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    Some((
        Oracle::from_snapshot(&exe, &pages).unwrap(),
        Oracle::from_snapshot(&exe, &pages).unwrap(),
    ))
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// Below `n`.
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }

    /// In `lo..hi`.
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo).cast_unsigned()).cast_signed()
    }

    /// A byte, often 0 or 0xff.
    fn byte(&mut self) -> u8 {
        match self.below(4) {
            0 => 0,
            1 => 0xff,
            _ => self.next().to_le_bytes()[0],
        }
    }

    /// A word, often small or at a limit.
    fn word(&mut self) -> u32 {
        match self.below(6) {
            0 => 0,
            1 => u32::MAX,
            2 => self.below(16),
            3 => 0x8000_0000,
            _ => self.next(),
        }
    }
}

/// Writes `bytes` at `addr` in both.
fn poke(a: &mut Oracle, b: &mut Oracle, addr: u32, bytes: &[u8]) {
    a.write(addr, bytes).unwrap();
    b.write(addr, bytes).unwrap();
}

/// Random bytes (see [`Xorshift::byte`]) at `addr` in both.
fn fill(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, addr: u32, len: u32) {
    let bytes: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
    poke(a, b, addr, &bytes);
}

/// Sets a global in both.
fn set<T: ffx_bridge::Value>(a: &mut Oracle, b: &mut Oracle, g: Global<T>, v: T) {
    g.set(a, v);
    g.set(b, v);
}

/// Zeroed scratch memory both oracles allocate alike.
fn scratch(a: &mut Oracle, b: &mut Oracle, len: usize) -> u32 {
    let x = a.alloc(len).unwrap();
    assert_eq!(b.alloc(len).unwrap(), x);
    x
}

/// Calls the original at its `ffx.sym` address in `a`, the Rust in `b`;
/// compares the results under `mask`.
fn twin(
    what: &str,
    a: &mut Oracle,
    b: &mut Oracle,
    sym: u32,
    args: &[u32],
    mask: u32,
    rust: impl FnOnce(&mut Oracle) -> u32,
) {
    let ra = a.call(a.rebase(sym), Conv::Cdecl, args).unwrap().eax & mask;
    let rb = rust(b) & mask;
    assert_eq!(ra, rb, "{what} {args:x?}: result");
}

/// Both wrote the same.
fn same(what: &str, a: &mut Oracle, b: &mut Oracle) {
    assert_eq!(a.diff(b).unwrap(), [], "{what}: memory");
}

/// Whether `len` bytes at `addr` can be read (a pointer in a snapshot).
fn readable(o: &mut Oracle, addr: u32, len: usize) -> bool {
    addr != 0 && o.read(addr, len).is_ok()
}

const ALL: u32 = u32::MAX;

/// No arguments, a value or an address.
/// A function to compare: name, `ffx.sym` address, result mask, the Rust.
type Getter = (&'static str, u32, u32, fn(&mut Oracle) -> u32);

/// A function on an actor: name, `ffx.sym` address, the Rust.
type Unit = (&'static str, u32, fn(&mut Oracle, u32));

const GETTERS: [Getter; 23] = [
    ("MsGetSaveEventAddress", 0x0078_5240, ALL, |m| {
        save::bridge::ms_get_save_event_address(m)
    }),
    ("MsGetSaveMenu", 0x0078_5290, ALL, |m| {
        save::bridge::ms_get_save_menu(m)
    }),
    ("MsGetSaveConfigCursorSave", 0x0078_51c0, ALL, |m| {
        save::bridge::ms_get_save_config_cursor_save(m)
    }),
    ("MsGetSaveConfigHiragana", 0x0078_51f0, ALL, |m| {
        save::bridge::ms_get_save_config_hiragana(m)
    }),
    ("getScenerioFlag", 0x0078_7360, 0xffff, |m| {
        u32::from(save::bridge::get_scenerio_flag(m))
    }),
    ("getInterValefor", 0x0078_7340, 0xff, |m| {
        u32::from(save::bridge::get_inter_valefor(m))
    }),
    ("MsGetGIL", 0x0078_4e80, ALL, |m| {
        save::bridge::ms_get_gil(m)
    }),
    ("MsGetSaveBattleCount", 0x0078_4f50, ALL, |m| {
        save::bridge::ms_get_save_battle_count(m)
    }),
    ("MsGetSaveAbilityMap", 0x0078_4f40, ALL, |m| {
        save::bridge::ms_get_save_ability_map(m)
    }),
    ("MsGetSaveWalkMap", 0x0078_5400, ALL, |m| {
        save::bridge::ms_get_save_walk_map(m)
    }),
    ("MsGetSaveEventFlag", 0x0078_5250, ALL, |m| {
        save::bridge::ms_get_save_event_flag(m)
    }),
    ("MsGetItemCheck", 0x0079_04a0, ALL, |m| {
        save::bridge::ms_get_item_check(m)
    }),
    ("MsGetSaveItemUse", 0x0079_0500, ALL, |m| {
        save::bridge::ms_get_save_item_use(m)
    }),
    ("MsGetSaveMonsterMeet", 0x0079_0b20, ALL, |m| {
        save::bridge::ms_get_save_monster_meet(m)
    }),
    ("MsGetSaveMonsterDefeat", 0x0079_0b10, ALL, |m| {
        save::bridge::ms_get_save_monster_defeat(m)
    }),
    ("MsGetChrTop", 0x0079_5970, ALL, |m| {
        actor::bridge::ms_get_chr_top(m)
    }),
    ("MsBattleGetStatus", 0x0078_16f0, ALL, |m| {
        state::bridge::ms_battle_get_status(m).cast_unsigned()
    }),
    ("MsEventEffectGetStat", 0x0078_8bd0, ALL, |m| {
        state::bridge::ms_event_effect_get_stat(m).cast_unsigned()
    }),
    ("MsBtlReadCueLevel", 0x007a_d180, ALL, |m| {
        state::bridge::ms_btl_read_cue_level(m)
    }),
    ("MsGetCameraMatrix", 0x007c_04b0, ALL, |m| {
        camera::bridge::ms_get_camera_matrix(m)
    }),
    ("MsBattleExeClear", 0x0078_1530, ALL, |m| {
        state::bridge::ms_battle_exe_clear(m).cast_unsigned()
    }),
    ("MsSetSaveParty", 0x0078_6930, 0, |m| {
        save::bridge::ms_set_save_party(m);
        0
    }),
    ("MsDebugFullGill", 0x0078_49c0, 0, |m| {
        save::bridge::ms_debug_full_gill(m);
        0
    }),
];

fn getters(a: &mut Oracle, b: &mut Oracle) {
    for (name, sym, mask, rust) in GETTERS {
        twin(name, a, b, sym, &[], mask, rust);
        same(name, a, b);
    }
}

fn randomize_getters(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    for g in [
        save::bridge::MENU,
        save::bridge::GIL,
        save::bridge::BATTLE_COUNT,
        actor::bridge::CHR_TOP,
    ] {
        let v = rng.word();
        set(a, b, g, v);
    }
    #[allow(clippy::cast_possible_truncation)]
    {
        set(a, b, save::bridge::SCENARIO, rng.next() as u16);
        set(a, b, save::bridge::BATTLE_PARTY, rng.next() as u16);
    }
    for g in [
        save::bridge::INTER_VALEFOR,
        save::bridge::BATTLE_PARTY3,
        state::bridge::READ_CUE_LEVEL,
        state::bridge::EXE_KEEP,
    ] {
        let v = rng.byte();
        set(a, b, g, v);
    }
    for g in [
        state::bridge::STATUS,
        state::bridge::EVENT_EFFECT_STAT,
        state::bridge::EXE,
    ] {
        let v = rng.byte().cast_signed();
        set(a, b, g, v);
    }
}

fn check_save(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use save::bridge as s;
    let out = scratch(a, b, 0x400);
    if synthetic {
        let ply = s::PLY_SAVE.ptr(a).addr;
        fill(a, b, rng, ply - 0x200, 0xb00 + 0x600);
        let shared = s::SHARED_COMMANDS.ptr(a).addr;
        fill(a, b, rng, shared, 0x200);
        let menu = s::MENU.ptr(a).addr;
        fill(a, b, rng, menu, 0x60);
        let caps = s::CAPTURES.ptr(a).addr;
        fill(a, b, rng, caps, 0x2c0);
    }
    for _ in 0..400 {
        let chr = rng.range(-2, 20).cast_unsigned();
        let cmd = match rng.below(4) {
            0 => [0x3000, 0x305f, 0x3060, 0x3fff, 0x4000, 0x1_3000, 0x2fff][rng.below(7) as usize],
            1 => rng.next(),
            _ => 0x3000 + rng.below(0x1000),
        };
        twin(
            "MsGetSaveCommand",
            a,
            b,
            0x0078_5020,
            &[chr, cmd],
            ALL,
            |m| s::ms_get_save_command(m, chr.cast_signed(), cmd),
        );
        twin(
            "MsGetSaveCommandUse",
            a,
            b,
            0x0078_5140,
            &[chr, cmd],
            ALL,
            |m| s::ms_get_save_command_use(m, chr.cast_signed(), cmd),
        );
    }
    for _ in 0..300 {
        let i = if rng.below(2) == 0 {
            rng.below(0x20)
        } else {
            rng.next()
        };
        twin("MsGetSavePlayerPtr", a, b, 0x0078_5330, &[i], ALL, |m| {
            s::ms_get_save_player_ptr(m, i)
        });
        twin("MsGetSavePlyJoin", a, b, 0x0078_5380, &[i], ALL, |m| {
            s::ms_get_save_ply_join(m, i)
        });
        twin("MsGetSavePlyJoined", a, b, 0x0078_53a0, &[i], ALL, |m| {
            s::ms_get_save_ply_joined(m, i)
        });
        twin("MsGetSaveSP", a, b, 0x0078_53e0, &[i], ALL, |m| {
            s::ms_get_save_sp(m, i)
        });
        let n = rng.next();
        twin("getDefaultChrName", a, b, 0x0078_7320, &[n], ALL, |m| {
            s::get_default_chr_name(m, n)
        });
        twin("MsGetMonsterCapture", a, b, 0x0079_0af0, &[n], ALL, |m| {
            s::ms_get_monster_capture(m, n)
        });
        twin("MsImportantGet", a, b, 0x0079_0800, &[i], ALL, |m| {
            s::ms_important_get(m, i)
        });
    }
    same("save lookups", a, b);

    // the inventory
    let in_use_item = |rng: &mut Xorshift| {
        if rng.below(3) == 0 {
            rng.next()
        } else {
            0x2000 + rng.below(8)
        }
    };
    for round in 0..40 {
        if synthetic || round > 0 {
            let ids: Vec<u8> = (0..save::ITEM_SLOTS)
                .flat_map(|_| (0x2000 + rng.below(10)).to_le_bytes()[..2].to_vec())
                .collect();
            let at = s::ITEM_IDS.ptr(a).addr;
            poke(a, b, at, &ids);
            let counts: Vec<u8> = (0..save::ITEM_SLOTS)
                .map(|_| rng.below(3).to_le_bytes()[0])
                .collect();
            let at = s::ITEM_COUNTS.ptr(a).addr;
            poke(a, b, at, &counts);
            let used = in_use_item(rng);
            twin("MsSetUseItem", a, b, 0x0079_0790, &[used], 0, |m| {
                s::ms_set_use_item(m, used);
                0
            });
        }
        for _ in 0..12 {
            let item = in_use_item(rng);
            twin("MsGetSaveItemNum", a, b, 0x0079_04b0, &[item], ALL, |m| {
                s::ms_get_save_item_num(m, item)
            });
        }
    }
    same("items", a, b);

    // buffers with their counts
    for p in [0, out] {
        for (name, sym, rust) in [
            (
                "MsSaveItemTypeBuffer",
                0x0079_0530,
                s::ms_save_item_type_buffer as fn(&mut Oracle, u32) -> u32,
            ),
            (
                "MsSaveItemNumBuffer",
                0x0079_0510,
                s::ms_save_item_num_buffer,
            ),
            ("MsGetSaveInParty", 0x0078_5270, s::ms_get_save_in_party),
            ("MsGetSaveOutParty", 0x0078_52d0, s::ms_get_save_out_party),
            (
                "MsGetSaveWeaponAmount",
                0x007a_bc60,
                s::ms_get_save_weapon_amount,
            ),
        ] {
            twin(name, a, b, sym, &[p], ALL, |m| rust(m, p));
            same(name, a, b);
        }
    }
    for mask in 0..8u32 {
        let ptr = |i: u32| if mask & (1 << i) != 0 { out + 4 * i } else { 0 };
        let args = [ptr(0), ptr(1), ptr(2)];
        twin("MsGetSavePartyMember", a, b, 0x0078_52f0, &args, 0, |m| {
            s::ms_get_save_party_member(m, args[0], args[1], args[2]);
            0
        });
    }
    same("MsGetSavePartyMember", a, b);

    // gil
    for _ in 0..200 {
        let gil = match rng.below(3) {
            0 => save::GIL_MAX - rng.below(100),
            1 => rng.next(),
            _ => rng.below(1000),
        };
        set(a, b, s::GIL, gil);
        let n = rng.word();
        twin("MsDebugAddGill", a, b, 0x0078_4830, &[n], 0, |m| {
            s::ms_debug_add_gill(m, n.cast_signed());
            0
        });
    }
    same("MsDebugAddGill", a, b);

    // captures, key items, flag words
    let flags = scratch(a, b, 0x200);
    for _ in 0..400 {
        let id = match rng.below(4) {
            0 => 0xff | (rng.below(16) << 8),
            1 => rng.next(),
            _ => rng.below(0x200),
        };
        let n = match rng.below(3) {
            0 => rng.word(),
            _ => rng.range(-12, 12).cast_unsigned(),
        };
        twin("AddMonsterCapture", a, b, 0x0079_0b90, &[id, n], ALL, |m| {
            s::add_monster_capture(m, id, n.cast_signed()).cast_unsigned()
        });
        let k = if rng.below(2) == 0 {
            rng.below(0x100)
        } else {
            rng.next()
        };
        twin("MsImportantSet", a, b, 0x0079_0930, &[k], 0, |m| {
            s::ms_important_set(m, k);
            0
        });
        twin("MsImportantGet", a, b, 0x0079_0800, &[k], ALL, |m| {
            s::ms_important_get(m, k)
        });
        let f = rng.next();
        twin("MsSetItemCheck", a, b, 0x0079_0670, &[f, flags], 0, |m| {
            s::ms_set_item_check(m, f, flags);
            0
        });
        let f = rng.next();
        twin(
            "MsSetMonsterCheck",
            a,
            b,
            0x0079_0be0,
            &[f, flags + 0x100],
            0,
            |m| {
                s::ms_set_monster_check(m, f, flags + 0x100);
                0
            },
        );
    }
    same("captures and flags", a, b);

    // affection
    for _ in 0..200 {
        let love = s::LOVE.ptr(a).addr;
        let v = match rng.below(3) {
            0 => 0x0fff_fffe - rng.below(4),
            1 => rng.next(),
            _ => rng.below(100),
        };
        let chr = rng.range(-1, 9);
        if (1..8).contains(&chr) {
            poke(a, b, love + 4 * chr.cast_unsigned(), &v.to_le_bytes());
        }
        let n = rng.word();
        twin(
            "MsAddLoveParam",
            a,
            b,
            0x0078_45e0,
            &[chr.cast_unsigned(), n],
            ALL,
            |m| s::ms_add_love_param(m, chr, n.cast_signed()),
        );
    }
    same("MsAddLoveParam", a, b);

    // equipment
    for round in 0..30 {
        if synthetic || round > 0 {
            let eq = s::EQUIPMENT.ptr(a).addr;
            let mut bytes = vec![0u8; (save::EQUIPMENT_SLOTS * save::EQUIPMENT_SIZE) as usize];
            for slot in bytes.chunks_mut(save::EQUIPMENT_SIZE as usize) {
                slot[2] = u8::from(rng.below(4) != 0);
                slot[3] = rng.byte();
                slot[4] = rng.below(12).to_le_bytes()[0];
                slot[5] = rng.below(3).to_le_bytes()[0];
                slot[6] = if rng.below(2) == 0 { 0xff } else { rng.byte() };
            }
            poke(a, b, eq, &bytes);
        }
        for _ in 0..10 {
            let args = [
                rng.range(-2, 12).cast_unsigned(),
                rng.range(-1, 3).cast_unsigned(),
                rng.below(3),
                if rng.below(2) == 0 { 0 } else { out },
            ];
            twin("MsGetSaveWeaponList", a, b, 0x007a_bd10, &args, ALL, |m| {
                s::ms_get_save_weapon_list(
                    m,
                    args[0].cast_signed(),
                    args[1].cast_signed(),
                    args[2].cast_signed(),
                    args[3],
                )
            });
        }
        for p in [0, out] {
            twin("MsGetSaveWeaponAmount", a, b, 0x007a_bc60, &[p], ALL, |m| {
                s::ms_get_save_weapon_amount(m, p)
            });
        }
        // an equipment of our own
        let mut eq = [0u8; 22];
        for x in &mut eq {
            *x = rng.byte();
        }
        for i in 0..4 {
            let v: u16 = match rng.below(3) {
                0 => 0,
                1 => 0xff,
                _ => 0x8000 + u16::from(rng.byte()),
            };
            eq[0xe + 2 * i..0x10 + 2 * i].copy_from_slice(&v.to_le_bytes());
        }
        eq[0xb] = rng.below(6).to_le_bytes()[0];
        poke(a, b, out + 0x200, &eq);
        twin(
            "MsCalcWeaponCap",
            a,
            b,
            0x0079_8eb0,
            &[out + 0x200],
            0,
            |m| {
                s::ms_calc_weapon_cap(m, out + 0x200);
                0
            },
        );
    }
    same("equipment", a, b);

    // the kernel tables are there only in a snapshot
    if s::KERNEL_IMPORTANT.get(b) != 0 {
        eprintln!("the key item table is loaded");
        for id in (0..0x100).chain([0x1000, 0xa000, 0xffff_ffff]) {
            for p in [0, out] {
                twin("MsImportantPtr", a, b, 0x0079_08a0, &[id, p], ALL, |m| {
                    s::ms_important_ptr(m, id, p)
                });
            }
        }
        same("MsImportantPtr", a, b);
    }
    if s::KERNEL_PLY_ROM.get(b) != 0 {
        eprintln!("ply_rom is loaded");
        for chr in 0..0x24 {
            twin("MsGetSaveChrMan", a, b, 0x0078_4f90, &[chr], ALL, |m| {
                s::ms_get_save_chr_man(m, chr)
            });
            twin("MsGetSaveChrFemale", a, b, 0x0078_4f60, &[chr], ALL, |m| {
                s::ms_get_save_chr_female(m, chr)
            });
            twin("MsGetSaveChrSummon", a, b, 0x0078_4ff0, &[chr], ALL, |m| {
                s::ms_get_save_chr_summon(m, chr)
            });
        }
        same("ply_rom flags", a, b);
    }
}

#[test]
fn save_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x2545_f491);
    getters(&mut a, &mut b);
    for _ in 0..50 {
        randomize_getters(&mut a, &mut b, &mut rng);
        getters(&mut a, &mut b);
    }
    check_save(&mut a, &mut b, &mut rng, true);
}

#[test]
fn save_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x1234_5678);
    getters(&mut a, &mut b);
    check_save(&mut a, &mut b, &mut rng, false);
    for _ in 0..20 {
        randomize_getters(&mut a, &mut b, &mut rng);
        getters(&mut a, &mut b);
    }
    check_save(&mut a, &mut b, &mut rng, true);
}

/// The functions on one actor at `p`.
fn actor_fns(a: &mut Oracle, b: &mut Oracle, p: u32) {
    use actor::bridge as x;
    for mode in [0u32, 1, 2, 3, 0xffff_ffff] {
        twin(
            "MsCheckSetMotion",
            a,
            b,
            0x007a_f320,
            &[p, mode],
            ALL,
            |m| x::ms_check_set_motion(m, p, mode.cast_signed()),
        );
    }
    for flags in 0..8u32 {
        let args = [p, flags & 1, (flags >> 1) & 1, (flags >> 2) & 1];
        twin(
            "MsAutoCommandStatusCheck",
            a,
            b,
            0x007b_24a0,
            &args,
            ALL,
            |m| {
                x::ms_auto_command_status_check(
                    m,
                    p,
                    args[1].cast_signed(),
                    args[2].cast_signed(),
                    args[3].cast_signed(),
                )
            },
        );
    }
    let unit: [Unit; 8] = [
        ("MsPushBtlPos", 0x007a_c650, x::ms_push_btl_pos),
        ("MsPopBtlPos", 0x007a_c600, x::ms_pop_btl_pos),
        ("MsSetAutoStatus", 0x0079_b290, x::ms_set_auto_status),
        ("MsResetAutoStatus", 0x0079_b1a0, x::ms_reset_auto_status),
        ("MsSetEternalRelife", 0x0078_deb0, x::ms_set_eternal_relife),
        ("MsMotionSpeedReset", 0x007a_b730, x::ms_motion_speed_reset),
        ("MsTerminateMotion", 0x007a_b6c0, x::ms_terminate_motion),
        ("msSetEndDir", 0x0079_3260, x::ms_set_end_dir),
    ];
    // in an order that keeps state for the next: set, reset, set again
    for i in [0, 2, 3, 2, 1, 4, 5, 6, 7, 3] {
        let (name, sym, rust) = unit[i];
        twin(name, a, b, sym, &[p], 0, |m| {
            rust(m, p);
            0
        });
        same(name, a, b);
    }
}

/// Synthetic actor contents, biased to reach every branch.
fn fill_actor(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, p: u32) {
    fill(a, b, rng, p, actor::ACTOR_SIZE);
    let small = |rng: &mut Xorshift| u8::from(rng.below(3) == 0);
    for off in [0x508, 0x608, 0x432, 0xdce, 0xdc8, 0x3b, 0x612, 0x613] {
        let v = small(rng);
        poke(a, b, p + off, &[v]);
    }
    let motion = [0x28, 0x29, rng.byte()][rng.below(3) as usize];
    poke(a, b, p + 0x3f0, &[motion]);
    poke(a, b, p + 0x63e, &[rng.below(4).to_le_bytes()[0]]);
    poke(a, b, p + 0xdfb, &[rng.below(4).to_le_bytes()[0]]);
    let dir = rng.range(-6, 6).to_le_bytes()[0];
    poke(a, b, p + 0x511, &[dir]);
    for i in 0..6 {
        let f = match rng.below(5) {
            0 => 0x7f80_0001 | rng.below(0x40_0000), // signalling NaN
            1 => [0x7f80_0000, 0xff80_0000, 0x7f7f_ffff, 1][rng.below(4) as usize],
            _ => (rng.next() & 0x3fff_ffff).wrapping_sub(0x2000_0000) ^ (rng.below(2) << 31),
        };
        poke(a, b, p + 0x500 + 4 * i, &f.to_le_bytes());
    }
    for i in 0..6 {
        let f = (f32::from(rng.below(2000).to_le_bytes()[0]) - 100.0) / 7.0;
        poke(a, b, p + 0x518 + 4 * i, &f.to_le_bytes());
    }
}

#[test]
fn actors_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x9e37_79b9);
    check_actor_tables(&mut a, &mut b, &mut rng, true);
    let p = scratch(&mut a, &mut b, actor::ACTOR_SIZE as usize);
    for _ in 0..300 {
        fill_actor(&mut a, &mut b, &mut rng, p);
        actor_fns(&mut a, &mut b, p);
    }
}

#[test]
fn actors_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0bad_cafe);
    check_actor_tables(&mut a, &mut b, &mut rng, false);
    // the game's own actors, when there is a battle
    let mut real = 0;
    for i in 0..actor::ACTORS {
        let p = actor::bridge::ms_get_chr(&b, i);
        if readable(&mut a, p, actor::ACTOR_SIZE as usize) {
            actor_fns(&mut a, &mut b, p);
            real += 1;
        }
    }
    eprintln!("{real} actors in the snapshot");
    let p = scratch(&mut a, &mut b, actor::ACTOR_SIZE as usize);
    for _ in 0..100 {
        fill_actor(&mut a, &mut b, &mut rng, p);
        actor_fns(&mut a, &mut b, p);
    }
    check_actor_tables(&mut a, &mut b, &mut rng, true);
}

fn check_actor_tables(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use actor::bridge as x;
    if synthetic {
        for g in [x::CHR_TOP, x::MON_TOP, x::WEAPON_TOP] {
            let v = rng.next();
            set(a, b, g, v);
        }
    }
    for _ in 0..600 {
        let i = if rng.below(2) == 0 {
            rng.below(0x100)
        } else {
            rng.next()
        };
        twin("MsGetChr", a, b, 0x0079_4020, &[i], ALL, |m| {
            x::ms_get_chr(m, i)
        });
        twin("MsGetMon", a, b, 0x0079_5aa0, &[i], ALL, |m| {
            x::ms_get_mon(m, i)
        });
        let second = rng.word();
        twin("MsGetWeapon", a, b, 0x0079_5b10, &[i, second], ALL, |m| {
            x::ms_get_weapon(m, i, second.cast_signed())
        });
        twin("MsBtlChrNumCheck", a, b, 0x0079_3650, &[i], ALL, |_| {
            x::ms_btl_chr_num_check(i)
        });
        twin("MsGetRamChrMonster", a, b, 0x0079_aef0, &[i], ALL, |_| {
            x::ms_get_ram_chr_monster(i)
        });
        let (hp, max) = (rng.word(), rng.word());
        let (hp, max) = if rng.below(2) == 0 {
            (rng.below(10000), rng.below(10000))
        } else {
            (hp, max)
        };
        // i32::MIN / -1 faults in the original
        if !(max == u32::MAX && hp.wrapping_mul(4) == 0x8000_0000) {
            twin(
                "MsCalcWeakLevel",
                a,
                b,
                0x0078_bf00,
                &[hp, max],
                ALL,
                |_| x::ms_calc_weak_level(hp.cast_signed(), max.cast_signed()),
            );
        }
    }
    same("actor tables", a, b);
}

#[test]
fn battle_state_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check_state(&mut a, &mut b, &mut Xorshift(0x5555_1234), true);
}

#[test]
fn battle_state_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check_state(&mut a, &mut b, &mut Xorshift(0x7777_4321), false);
}

fn check_state(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use state::bridge as s;
    let buf = scratch(a, b, 0x100);
    // the game's own values first
    if !synthetic {
        let field = s::SCENE_FIELD.get(b);
        let group = s::SCENE_GROUP.get(b);
        if s::STATUS.get(b) == 0 || (readable(a, field, 2) && readable(a, group, 1)) {
            twin("MsGetBattleScene", a, b, 0x0078_2970, &[], ALL, |m| {
                s::ms_get_battle_scene(m)
            });
        }
        if readable(a, s::VOICE_EVENT.get(b), 1) {
            twin("MsGetVoiceEventFlag", a, b, 0x0079_86a0, &[], ALL, |m| {
                s::ms_get_voice_event_flag(m)
            });
        }
        same("battle state", a, b);
    }
    for _ in 0..300 {
        set(
            a,
            b,
            s::STATUS,
            [0, 1, rng.byte().cast_signed()][rng.below(3) as usize],
        );
        set(a, b, s::SCENE_FIELD, [0, buf][rng.below(2) as usize]);
        set(a, b, s::SCENE_GROUP, buf + 2 + rng.below(4));
        fill(a, b, rng, buf, 8);
        twin("MsGetBattleScene", a, b, 0x0078_2970, &[], ALL, |m| {
            s::ms_get_battle_scene(m)
        });
        set(a, b, s::VOICE_EVENT, buf + 0x10);
        poke(a, b, buf + 0x10, &[u8::from(rng.below(2) == 0)]);
        set(a, b, s::VOICE_MUTE, u8::from(rng.below(2) == 0));
        twin("MsGetVoiceEventFlag", a, b, 0x0079_86a0, &[], ALL, |m| {
            s::ms_get_voice_event_flag(m)
        });
        set(a, b, s::EXE, rng.byte().cast_signed());
        set(a, b, s::EXE_KEEP, rng.byte());
        twin("MsBattleExeClear", a, b, 0x0078_1530, &[], ALL, |m| {
            s::ms_battle_exe_clear(m).cast_unsigned()
        });
        if rng.below(20) == 0 {
            twin("MsMemCheckInit", a, b, 0x0079_9ee0, &[], 0, |m| {
                s::ms_mem_check_init(m);
                0
            });
        }
        let size = rng.word();
        twin(
            "MsMemCheckRegist",
            a,
            b,
            0x0079_9f00,
            &[rng.next(), size],
            0,
            |m| {
                s::ms_mem_check_regist(m, size.cast_signed());
                0
            },
        );
        if rng.below(10) == 0 {
            set(a, b, s::ATEL_COUNT, rng.range(-3, 20));
        }
        if rng.below(30) == 0 {
            twin("MsAtelRequestInit", a, b, 0x0079_7ff0, &[], 0, |m| {
                s::ms_atel_request_init(m);
                0
            });
        }
        let req = [rng.next(), rng.next(), rng.next(), rng.next()];
        twin("MsAtelRequestAdd", a, b, 0x0079_7b70, &req, 0, |m| {
            #[allow(clippy::cast_possible_truncation)]
            s::ms_atel_request_add(m, req.map(|v| v as u16));
            0
        });
    }
    same("battle state", a, b);
}

#[test]
fn helpers_match_the_original() {
    use util::bridge as u;
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let mut rng = Xorshift(0x1357_9bdf);
    let rng = &mut rng;
    let buf = scratch(a, b, 0x400);
    for _ in 0..300 {
        fill(a, b, rng, buf, 0x400);
        let n = rng.range(-5, 0x180).cast_unsigned();
        let p = buf + rng.below(0x40);
        twin("MsStructClear", a, b, 0x0079_f230, &[p, n], 0, |m| {
            u::ms_struct_clear(m, p, n.cast_signed());
            0
        });
        let v = rng.next();
        twin(
            "MsStructFill",
            a,
            b,
            0x0079_f260,
            &[p + 0x100, n, v],
            0,
            |m| {
                #[allow(clippy::cast_possible_truncation)]
                u::ms_struct_fill(m, p + 0x100, n.cast_signed(), v as u16);
                0
            },
        );
        // a string with terminators here and there
        let s: Vec<u8> = (0..0x60)
            .map(|_| match rng.below(30) {
                0 => 0,
                1 => 0xff,
                2 => b'\n',
                3 => b'\r',
                _ => 0x20 + rng.below(0x5f).to_le_bytes()[0],
            })
            .chain([0])
            .collect();
        poke(a, b, buf + 0x200, &s);
        let max = rng.range(-2, 0x70).cast_unsigned();
        let (src, dst) = (buf + 0x200, buf + 0x300);
        twin("MsStrCopy", a, b, 0x007c_f6c0, &[src, dst, max], ALL, |m| {
            u::ms_str_copy(m, src, dst, max.cast_signed()).cast_unsigned()
        });
        // overlapping, forward
        twin(
            "MsStrCopy",
            a,
            b,
            0x007c_f6c0,
            &[src, src + 3, max],
            ALL,
            |m| u::ms_str_copy(m, src, src + 3, max.cast_signed()).cast_unsigned(),
        );
        let args = [rng.next(), rng.next(), rng.next()];
        twin("MsGetEffID", a, b, 0x0078_8df0, &args, ALL, |_| {
            u::ms_get_eff_id(args[0], args[1], args[2])
        });
    }
    same("memory helpers", a, b);
    for _ in 0..40 {
        let (first, next) = (
            rng.range(-1, 5).cast_unsigned(),
            rng.range(-1, 3).cast_unsigned(),
        );
        twin(
            "MsAutoRepeatInit",
            a,
            b,
            0x007c_89a0,
            &[buf, first, next],
            0,
            |m| {
                u::ms_auto_repeat_init(m, buf, first.cast_signed(), next.cast_signed());
                0
            },
        );
        for _ in 0..20 {
            let input = [0, 1, 0x10, u32::MAX][rng.below(4) as usize];
            let input = if rng.below(4) == 0 { input } else { 0x10 };
            twin(
                "MsAutoRepeatCheck",
                a,
                b,
                0x007c_8950,
                &[buf, input],
                ALL,
                |m| u::ms_auto_repeat_check(m, buf, input.cast_signed()).cast_unsigned(),
            );
        }
    }
    same("auto repeat", a, b);
}

fn check_camera(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    use camera::bridge as c;
    let out = scratch(a, b, 0x20);
    let cam = scratch(a, b, 0x1000);
    if !synthetic {
        for i in 0..4 {
            twin("MsCameraGetNum", a, b, 0x007b_d340, &[i], ALL, |m| {
                c::ms_camera_get_num(m, i)
            });
            twin("MsCameraIDcheck", a, b, 0x007b_d560, &[i], ALL, |m| {
                c::ms_camera_id_check(m, i).cast_unsigned()
            });
        }
        if readable(a, c::CAMERA.get(b), 0xca0) {
            for side in 0..2 {
                twin(
                    "MsCameraGetPosNow",
                    a,
                    b,
                    0x007b_d470,
                    &[side, out],
                    0,
                    |m| {
                        c::ms_camera_get_pos_now(m, side, out);
                        0
                    },
                );
            }
            scr_dpt(a, b);
        }
        same("camera", a, b);
    }
    let slots = c::SLOTS_AT.ptr(a).addr;
    for _ in 0..300 {
        for k in 0..4u32 {
            for i in 0..4u32 {
                let at = slots + k * c::CAMERA_SIZE + 8 * i;
                let level = [0u8, 1, 2, 3, 0x80, 0xff][rng.below(6) as usize];
                let bonus = rng.range(-3, 4).to_le_bytes()[0];
                let off = u8::from(rng.below(4) == 0);
                let done = u8::from(rng.below(4) == 0);
                let id = rng.below(4).to_le_bytes()[0];
                poke(a, b, at, &[id, 0, level, 0, off, done, bonus]);
            }
        }
        let k = rng.below(4) | (rng.below(4) << 8);
        twin("MsCameraGetNum", a, b, 0x007b_d340, &[k], ALL, |m| {
            c::ms_camera_get_num(m, k)
        });
        let id = match rng.below(3) {
            0 => k & 0xff,
            1 => (k & 0xff) | (u32::from([1u8, 2, 3][rng.below(3) as usize]) << 16),
            _ => rng.next(),
        };
        twin("MsCameraIDcheck", a, b, 0x007b_d560, &[id], ALL, |m| {
            c::ms_camera_id_check(m, id).cast_unsigned()
        });
        // the camera in use
        set(a, b, c::CAMERA, cam);
        for i in 0..8 {
            // No signalling NaNs: the x87 makes them quiet as it loads them
            // (camera::x87_copy), the emulator keeps them as they are.
            let f = match rng.below(4) {
                0 => 0x7fc0_0000 | rng.below(0x40_0000),
                1 => 0xff80_0000,
                _ => rng.next(),
            };
            let f = camera::x87_copy(f);
            for at in [0x50, 0x66c, 0xc9c] {
                poke(a, b, cam + at + 4 * (i % 3), &f.to_le_bytes());
            }
            let side = rng.next();
            let p = if rng.below(4) == 0 { 0 } else { out };
            twin("MsCameraGetPosNow", a, b, 0x007b_d470, &[side, p], 0, |m| {
                c::ms_camera_get_pos_now(m, side, p);
                0
            });
            scr_dpt(a, b);
        }
    }
    same("camera", a, b);
}

/// `MsCameraGetScrDpt`, an x87 result.
fn scr_dpt(a: &mut Oracle, b: &mut Oracle) {
    let ra = a
        .call(a.rebase(0x007b_d510), Conv::Cdecl, &[])
        .unwrap()
        .st0
        .expect("a float result");
    let rb = f64::from(camera::bridge::ms_camera_get_scr_dpt(b));
    // the oracle reads every NaN in ST0 as the same NaN
    if ra.is_nan() {
        assert!(rb.is_nan(), "MsCameraGetScrDpt NaN, {rb}");
    } else {
        assert_eq!(ra.to_bits(), rb.to_bits(), "MsCameraGetScrDpt {ra} {rb}");
    }
}

#[test]
fn camera_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check_camera(&mut a, &mut b, &mut Xorshift(0x0f0f_1234), true);
}

#[test]
fn camera_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check_camera(&mut a, &mut b, &mut Xorshift(0x3c3c_5678), false);
}
