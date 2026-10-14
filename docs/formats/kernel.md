# Battle kernel tables

`ffx_ps2/ffx/master/new_<lang>pc/battle/kernel/*.bin` hold the game's
gameplay tables: items, battle commands, abilities, monster stats and
battle text. There are 26 per language, 8 languages
(`jp us fr sp de it kr ch`).

Reader/writer: [`crates/ffx-kernel`](../../crates/ffx-kernel). Text:
[text.md](text.md).

## Loading

`Kernel_LoadFile` (`0x00782df0`) builds the path from the language
(0 `jp`, 1 `us`, 2 `fr`, 3 `sp`, 4 `de`, 5 `it`, 9 `kr`, 10 `ch`) and reads
the whole file. `Kernel_LoadTables` (`0x00781d40`) keeps each table in a
global: `command` → `0x0112a92c`, `item` → `0x0112a940`, `a_ability` →
`0x0112a944`, `ply_rom` → `0x0112a938`, `monmagic1` → `0x0112a930`, …

## Container

`Kernel_FindRecord(id, table, &strings)` (`0x007ab870`) defines it. All
integers little-endian.

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u16` | Segment count *S* (1 in every shipped table) |
| `0x02` | 6 bytes | Zero |
| `0x08` | 12 × *S* | Segment descriptors |
| … | | Records of each segment, back to back |
| … | | String area, to the end of the file |

Segment descriptor:

| Offset | Type | Meaning |
|---|---|---|
| `0x0` | `u16` | First record ID |
| `0x2` | `u16` | Last record ID (inclusive) |
| `0x4` | `u16` | Record size |
| `0x6` | `u16` | Data length = size × count |
| `0x8` | `u32` | Data offset |

Record *id* lives at `offset + size × (id − first)`. An ID outside every
segment returns the first segment's first record. The third output of
`Kernel_FindRecord` is `offset + length`: the start of the string area.

Record IDs need not start at 0: `monster2` covers 101–180 and `monster3`
181–365, continuing `monster1`.

## Strings

Each record starts with *n* string references of 4 bytes: a `u16` offset
into the string area, then a `u16` the game's string lookups do not read
(for example `*(u16*)(record + 4·i) + strings` in `0x007ab870`'s callers).
*n* depends on the table and is the same in all languages.

The string area is pooled: strings share suffixes, so one offset may point
into the middle of another string. `Table::set_string` therefore keeps the
area as is, reuses an identical existing string, or appends.

## Known fields

Battle commands (`command.bin`) and items (`item.bin`) share a 96-byte
record:

| Offset | Type | Field | Evidence |
|---|---|---|---|
| `0x00` | 4 × ref | name, ?, description, ? | names and help texts |
| `0x1c` | `u32` | flags | bit `0x2` puts an item in the usable list, bit `0x400` in the second list (`0x007906a0`); bit `0x40000`: formula and power come from the user's weapon (`0x007898d0`, `0x0078e630`) |
| `0x20` | `u8` | crit_flags | bit `0x4` can be critical; bit `0x8` takes the crit bonus from the equipment (actor `+0x5d8`) instead of `crit_bonus` (`0x00789690`) |
| `0x27` | `u8` | crit_bonus | added to the critical chance |
| `0x28` | `u8` | formula | `Cmd_HealAmount` (`0x008c6920`): 6 = power × 50, 7 = (stat + power) / 2 × power, 8 = max HP × power / 16 |
| `0x2a` | `u8` | power | Potion 4 (200 HP), Hi-Potion 20 (1000 HP), X-Potion formula 8, power 16 (full) |

All other bytes are exported as hex runs (`bytes_<offset>`) until they are
identified.

| Table | Record size | Strings |
|---|---|---|
| `a_ability` | 108 | 4 |
| `command`, `item` | 96 | 4 |
| `monmagic1`, `monmagic2` | 92 | 4 |
| `monster1`–`3` | 128 | 5 |
| `w_name` | 72 | 14 |
| `ply_save` | 148 | 1 |
| `ply_rom` | 44 | 4 |
| `panel` | 24 | 4 |
| `important` | 20 | 4 |
| `*_txt`, `sphere` | 16 (`btl_txt` 8) | 2 or 4 |

## Tables shared by all languages

Some tables exist once, in `ffx_ps2/ffx/master/jppc/battle/kernel/`, and
the game reads them as PS2 files by number (category `0xd` index *n* is
file `cdrom.fid[0xd] + n` of the name table `cdrom.fnd` in
`ffx_ps2/ffx/proj/battle/jp/cddata/`): `weapon` (5), `takara` (25),
`buki_get` (26); category `0x2f`: `shop_arms` (4), `kaizou` (5),
`sum_grow` (6), `arms_shop` (2), `item_shop` (3). `ffx kernel` reads
`buki_get`, `shop_arms`, `takara` and `weapon` with these fields:

**Equipment** (`weapon`, the starting inventory, "wep_data", copied into
the 200 inventory slots at `0x01130f2c` by `SaveData_Clear`, `0x00786b00`;
`shop_arms`, what shops sell), 22 bytes, the inventory's own layout:

| Offset | Type | Field | Evidence |
|---|---|---|---|
| `0x00` | `u16` | name | `w_name` record, chosen when the equipment is made (`0x007a0cf0`); 0 in the files |
| `0x02` | `u8` | in_use | slot holds equipment (`0x007abc60`) |
| `0x03` | `u8` | flags | bit `0x2`: not counted as owned (`0x007abc60`), skipped by `0x0079bb50` |
| `0x04` | `u8` | character | `ply_save` record: 0–6 the party, 7 Seymour, 8–17 aeons; only 0–6 get a name and model |
| `0x05` | `u8` | armor | 0 weapon, 1 armor |
| `0x06` | `u8` | equipped_by | wearer, `0xff` none; rebuilt from `ply_save` `+0x2d`/`+0x2e` (`0x007abca0`) |
| `0x08` | `u8` | formula | weapon: actor `+0x5c1`, the formula of commands with flag `0x40000` (`0x0079c5f0`) |
| `0x09` | `u8` | power | weapon: actor `+0x5c7` |
| `0x0a` | `u8` | crit_bonus | weapon + armor → actor `+0x5d8` |
| `0x0b` | `u8` | slots | ability slots |
| `0x0c` | `u16` | model | `w_name` `+0x38 + 2 × character` (`0x007a0c50`) |
| `0x0e` | `u16` × 4 | ability_1–4 | `0x8000` + `a_ability` ID, `0x00ff` empty; each applies its `a_ability` record to the wearer (`0x0079c5f0`) |

**Treasure** (`takara`, `Treasure_Give`, `0x00799420`), 4 bytes: `kind`
(0 gil = amount × 100, 2 item, 5 equipment from `buki_get`, 10 key item),
`amount`, `item` (`u16`: `0x2000` + item, `buki_get` record, `0xa000` +
key item).

**Found equipment** (`buki_get`), 16 bytes, turned into an equipment slot
by `Treasure_Give`: `flags`, `character`, `armor`, ?, `formula`, `power`,
`crit_bonus`, `slots` (at least the ability count), then four `u16`
abilities (0 none).

The save test checks this in the running game: Tidus's and Rikku's worn
weapon and armor (inventory slots from `ply_save`) are in use, theirs and
of the right kind, and their battle actors carry the weapon's formula and
power and the summed crit bonus.

## Editing

```bash
ffx kernel tables                                   # what is known per table
ffx kernel export item -o item.toml                 # English by default; --lang de, ...
ffx kernel export --all -o kernel/                  # every table
ffx kernel import item edits.toml --mod-dir mods/   # applies edits onto the original
```

An import starts from the game's original table and changes only the
records and keys the TOML lists:

```toml
[[record]]
id = 0
name = "Mega Potion"
power = 20
```

## Verification

`cargo test -p ffx-kernel` (with `FFX_GAME_DIR`) parses and rewrites all
208 tables (26 × 8 languages) byte for byte, decodes and re-encodes all
92,224 strings, and round-trips each table through TOML unchanged.

## Open questions

- The second `u16` of each string reference.
- Most record fields; `a_ability` (fed into dozens of actor fields), the
  shop lists and `kaizou`.
