# Save files and loading a save

How the PC build reads a save and starts the game from it, recovered from
FFX.exe (`symbols/ffx.sym` has the names) and checked on the running game
(`crates/ffx-harness/examples/load_trace.rs`, `load_compare.rs`). The
harness command `loadsave` (`crates/ffx-hook/src/loadsave.rs`) runs the
same path without the title menu.

## Files

`SetUpDefaultSaveFolder` (`0x006f0330`) builds the folder from Documents
(`SHGetFolderPathW`, `CSIDL_PERSONAL`) and `Square Enix\FINAL FANTASY
X&X-2 HD Remaster\FINAL FANTASY X\` into `g_SaveFolder` (`0x00ce6ee0`,
UTF-16) and sets `g_SaveFolderSet` (`0x00ce72e0`). A slot `n` (0..199) is
the file `%s%s%03d` of that folder, `ffx_` (`*0x00c3577c`) and `n`:
`ffx_000` is the autosave, `ffx_001` up the manual saves.

Every file is `0x6900` bytes, the size the save manager was made with
(`SaveData_Init`, `0x00647fb0`: buffer `g_SaveFileBuffer` `0x01597f70`,
size `0x6900`, kept at manager `+0x350`/`+0x354`):

| Offset | Size | |
|---|---|---|
| `0x00` | `0x40` | header; `+0x1a` `u16` CRC |
| `0x40` | `0x68c0` | the game state block, copied to and from `g_SaveData` (`0x0112ca90`): `+0` map, `+2` previous map, `+0xc` entrance, `+0xba`/`+0xb8` the map and entrance to return to (`u16` each); the whole layout is under "The game state block" below, `+0xbec` story progress (`g_SaveScenario`), the party records at `0x0113205c` (block `+0x55cc`) |
| `0x64f4` | 4 | the CRC again |
| `0x64f8` | `0x408` | the tail: block `+0x64b8`..`+0x68c0`, copied with the block but outside the CRC; nothing reads it ("The tail" below) |

`SaveDataCheckCrc` (`0x00647d70`) passes a buffer when the `u16` at `+0x1a`
equals the word at `+0x64f4` and `TkCRC(0x64b8, buffer + 0x40)` gives it
(the word is zeroed first), or always with `g_SaveData_CrcCheckOff`
(`0x00ccb9a4`).

### The tail

The block the game copies is `0x68c0` bytes, but the CRC and the last
field (`character_names`, `g_SaveChrNames` `0x01132ddc`, ends at block
`+0x64b4`) stop at `+0x64b8`. The `0x408` bytes after them (`0x01132f48`..
`0x01133350`, file `0x64f8`..`0x6900`) go everywhere the block goes and are
never read:

- `MenuSave_PackSave` (`0x008b3e60`) copies all `0x68c0` bytes from
  `g_SaveData` into the file buffer. It stores a CRC over `0x68c0` bytes, but
  every writer then calls `SaveDataWriteCrc` (`0x00648f20`), which
  recomputes it over `0x64b8`: `SaveDataMachine_StartSave_006f07a0` (menu
  save) and `SaveDataManager_debugSave_Internal_6f0650` (`0x006f0510`,
  autosave). Fahrenheit's save module calls it too. `debugSave`
  (`0x00646df0`) uses `0x64b8` directly.
- `SaveDataCheckCrc` checks `0x64b8` bytes, so the tail can hold anything
  and the save still loads. `TkMenuCopyLoadedSave` copies it into
  `g_SaveData` with the rest.
- `MsSetSaveStartGame` clears it with the block on a new game.
  `TkMenuCtrlTuto` (`0x008db4e0`) keeps all `0x68c0` bytes while a tutorial
  runs on its own save and copies them back afterwards.
- The Ghidra dump's only reference into the range is a loop end
  (`resetDefaultPlayerNameStatus`, one past the names' `+0x12` bytes). The
  next label is the rename flag at `0x01133350`.
- Five PC saves (the autosave and four manual saves) have all `0x408` bytes
  zero. No shipped script declares a storage 0 variable there: storage 0
  is block `+0x1ec`, so the tail is storage 0 `0x62cc`..`0x66d3`, and the
  last storage 0 variable of the 1,621 scripts ends at `0x1a00`
  ([Atel#](https://github.com/yutamago/ffx-atel-sharp) `docs/research/mod-saves.md`,
  which uses the tail for mods' saved variables).

## The PS2 save file

Source: ffx-demaster `docs/compare.md` (MIPS disassembly of `SCES_504.92`,
the German PAL disc; saves made this way load in PCSX2). PS2 addresses are
ELF addresses (`docs/formats/ps2-elf.md`).

The memory card holds folder and file `BESCES-50492FF0906nn/BESCES-50492FF0906nn`
(nn = slot 0..98, listed in the game as file nn + 1), with the icon
`010906nn.ico` and `icon.sys` beside it. The region prefixes are a table
at `0x00326e50..0x00326ec0` (`BISLPS-25088`, `BASLUS-20312`,
`BESCES-50490`..`50494`, `BISLPM-65115`, picked by `0x0020b858`), built
into `0x00326f58` by `0x002413e0` with `FF0906%02d` (`0x00326ee0`). Path
builders: `0x0023cbd8` data file, `0x0023cc58` icon, `0x0023cd38`
`icon.sys`, `0x0023cdb8` folder.

The file is `0x64f8` bytes: the `0x40` header and `0x64b8` bytes of game
state (buffer `0x01d57b00`, state `0x01d57b40`), with the CRC as `u16` at
header `+0x1a` and as a word at file `+0x64f4` (zeroed while computing).
The CRC (`0x002510a8`, check at `0x00240f1c`, written at `0x0023d478`) is
CRC-16 with polynomial `0x1021`, start `0xffff` and the result inverted,
over a table built for entries 0..254 only (entry `0xff` stays 0). It is
the PC's `TkCRC(0x64b8, buffer + 0x40)` (above): the PC's `ffx_NNN` files
checked (slots 1 and 5) are this file zero-padded to `0x6900`, pass the
PS2's check and load in PCSX2.

On the PS2, `g_SaveData` is `0x003194d0` (cleared with size `0x64b8` at
`0x001bc440`); `g_BtlFieldIndex` is `0x00318c76`, followed by the group
(`+2`), the formation (`+3`) and the field name (`+4`): `22 00 00 01`
`kami00_01` in a Thunder Plains encounter (PINE reads in PCSX2 against the
loaded save).

## The game state block (`g_SaveData`, `0x0112ca90`)

The `0x68c0` bytes the file keeps at `0x40`. The names are Fahrenheit's
(`SaveData` in `core/ffx/savedata.cs`, `PlySave` in `plysave.cs`; used with
permission, Fahrenheit 4ec1919). The evidence column gives what in FFX.exe
reads or writes the field (our labels and accessors, `symbols/ffx.sym`);
"Fahrenheit, not checked" marks their fields we have not found in the code
(most of those are flags that only the event scripts touch, through their
save variables).

| Offset | Type | Fahrenheit | Evidence |
|---|---|---|---|
| `0x00`, `0x02` | `u16` | `current_room_id`, `last_room_id` | map and previous map (`TkSetEventJump` `0x0088ea60`) |
| `0x04`, `0x06` | `u16` | `now_eventjump_map_no`, `last_eventjump_map_no` | `+4` is the map `MsBtlReadSetScene` compares the battle map with; `+6` Fahrenheit, not checked |
| `0x08`, `0x0a` | `u16` | `now_eventjump_map_id`, `last_eventjump_map_id` | Fahrenheit, not checked |
| `0x0c`, `0x0d` | `u8` | `current_spawnpoint`, `last_spawnpoint` | entrance and previous entrance (`TkSetEventJump`) |
| `0x0e` | `u16` | `atel_save_dic_index` | Fahrenheit, not checked |
| `0x10` | `u8` | `atel_battle_scene_group` | `AtelGetBattleSceneGroup` (`0x0086a9a0`) |
| `0x11`, `0x12` | `u8` | `fade_mode`, `fade_time` | `AtelSetMapFadeType`, `AtelSetMapFadeFrames` (`0x008700c0`, `0x008700d0`) |
| `0x13` | `u8` | `battle_status` | bit 1 `AtelGetBattleStatusSys`, bit 2 `AtelGetBattleStatusDummyEnc` |
| `0x18` | 2 × `u32` | `flying_ship_pos` | Fahrenheit, not checked |
| `0x20` | `u8` | `atel_is_push_member` | `AtelPushMember` (`0x0086e310`) sets it, `AtelPopMember` (`0x0086ddb0`) clears it; `AtelGetSave20` (`0x0086d9d0`) reads it |
| `0x21`-`0x23` | 3 × `u8` | `atel_push_frontline` | the party `AtelPushMember` saves |
| `0x24` | `u8` | `atel_push_party` | join bits of characters 0-6 (`AtelPushMember`) |
| `0x28`, `0x29` | `u8` | `is_cam_underwater`, `is_map_underwater` | Fahrenheit, not checked |
| `0x2b` | `u8` | `tk_event_new_game` | `TkGetEventNewGame`, `TkSetEventNewGame` (`0x0088d5d0`, `0x0088eb40`) |
| `0x2c` | 8 × `u32` | `affection` | `g_SaveLove`, `MsAddLoveParam` (`0x007845e0`) |
| `0x4c` | 20 × `u32` | `affection_room_flags` | Fahrenheit, not checked: a bit per map below `0x280` once its affection was gained |
| `0xa8`, `0xaa` | `u16` | - | the save's music and texture video mode, read from the loaded file (`SaveDataSaveLoadSucceed`, see "The load machine") |
| `0xb4`, `0xb6` | `u16` | `item_map_x`, `item_map_y` | the world map marker (`Native_SetMapMarkerPos` 0x016e, `AtelGetMapMarkerX`/`Y` `0x00873ae0`/`0x00873af0`, drawn by `0x008e2b60`); Fahrenheit calls their meaning unknown |
| `0xb8`, `0xba` | `u16` | `saved_current_spawnpoint`, `saved_current_room_id` | the entrance and map a loaded game returns to (`TkSetEventJump` with a negative map) |
| `0xbc` | `u32` | `time` | play time (`Atel_PlayTimeUpdate_0086ca20`, capped at 3599999; `TkMenuGetTime` `0x008a9c50`) |
| `0xc0`-`0xce` | | `saved_*` | `+0xc2` previous map and `+0xce` previous entrance (`TkSetEventJump`); the rest Fahrenheit, not checked |
| `0xd1` | `u8` | `albhed_rikku` | `AtelGetAlbhedRikku` (`0x0086a7e0`) |
| `0xd2` | `u8` | `drop_shadow_mode` | Fahrenheit, not checked |
| `0xd4`, `0xd6` | `u16`, `u8` | `atel_force_place_id_value`, `atel_force_place_id` | `AtelSetForcePlaceId` (`0x00870140`) writes both, `AtelClearForcePlaceId` the flag |
| `0xd7` | `u8` | `atel_water_btl_effect` | `AtelGetWaterBattleEffect` (`0x0086c8a0`) |
| `0xd8`, `0xdc`, `0xe0` | `u32` | `on_memory_movie_file_no`, `on_memory_movie_mode`, `on_memory_movie` | movie, flags and state (`AtelSetOnMemoryMovieFileWait` `0x00873f70`: `+0xe0` = 1, `+0xd8` the movie, `+0xdc` the flags) |
| `0xe4` | `u32` | `rand_encounter_modifiers` | `AtelGetRandomEncountStatus` (`0x00873b40`); bits 8-10 no fanfare, no game over, no music (Fahrenheit, not checked) |
| `0xe8` | `u16` | `btl_end_tag_always` | `AtelGetBattleEndTagAlways` (`0x00873a90`) |
| `0xea` | `u16` | `sphere_monitor` | `AtelGetSphereMonitor` (`0x0086c4b0`) |
| `0x279`-`0x3f1` | `u8` | `progression_flags_*`, `luca_controllable_character` | Fahrenheit, not checked (script variables) |
| `0x3fc`-`0x400` | 3 × `u16` | `lightning_dodging_*` | Fahrenheit, not checked |
| `0x5ec` | `u8` | `soundtrack_type` | 0 arranged, 1 original: `iMusicRefreshBgmType` (`0x008cc170`) passes bit 0 to `iMuiscSetBgmType`, and FMOD music set 1 is `Music_PS2/ffx_music_PS2.fev` (`docs/formats/audio-platform.md`). Written by `MenuConfig_SetBgmType` (`0x008cb0b0`) and natives 0x0264/0x0267; `MsSetSaveStartGame` (`0x00786b00`) clears it with the rest of `g_SaveData`, so a new game starts arranged and a loaded save keeps its own choice (ffx-demaster `docs/patches.md`) |
| `0xbec` | `u16` | `story_progress` | `g_SaveScenario`, `AtelGetScenarioFlag` |
| `0xc38`-`0xc91` | | celestials, airship, Jecht spheres, aeons, dark aeons | `+0xc89` is `g_SaveInterValefor` (`getInterValefor`); the rest Fahrenheit, not checked |
| `0xcd7` | 42 × `u8` | `battle_dialog_lines_seen` | Fahrenheit, not checked |
| `0x1984` | | `blitzball_data` | Fahrenheit, not checked; its `player_uncovered_techs_page_1` is 60 `u32`, not 240 ("Fields the event scripts explain" below) |
| `0x3d0c` | `u32` | `config` | `g_SaveMenu`, read by the `MsGetSaveConfig*` getters (`0x007851a0..0x00785230`, alphabetical, each the mirror of its `MsSetSaveConfig*` setter's mask): bit 0 stereo, 1 cursor memory, 2 controller, 3 hiragana, 4 vibrate, 5 credit name, 6 guide map, 9 subtitles (`MsGetSaveConfigSubTitle` `0x00785210`, 0 while `g_BtlMuteSubtitle` is set; Fahrenheit's `config_subtitles`), 16 English (`MsSetSaveConfigEnglish`; its getter returns 0 on PC). Bit 9 is set in the new-game word (`0x00002f61`, read after `MsSetSaveStartGame`) and in the PC saves measured, and nothing on PC clears it (`MsSetSaveConfigSubTitle` has no caller): scripted battle lines (native 0x70de) are subtitled (ffx-demaster `docs/patches.md`) |
| `0x3d10` | `u32` | `unlocked_primers` | Fahrenheit, not checked |
| `0x3d14` | `u32` | `battle_count` | `g_SaveBattleCount` |
| `0x3d48` | `u32` | `gil` | `g_SaveGil` |
| `0x3d58`, `0x3d5b` | 3 + 17 × `u8` | `party_frontline`, `party_backline` | `g_SaveInParty`, `g_SaveOutParty` |
| `0x3d6c` | | `ability_map_limit` | `g_SaveSharedCommands`: the flag words of commands `0x3060` and up, for everybody |
| `0x3da4` | `u32` | `yojimbo_compatibility` | `g_SaveYoujinbouCom` |
| `0x3da8` | `u32` | `yojimbo_type` | Fahrenheit, not checked |
| `0x3dac` | `u32` | `tidus_limit_uses` | `g_SaveTidusLimitUses` (`MsLimitTidusLearn`; not counted in `g_BtlBattleType` 2) |
| `0x3db0` | `u32` | `successful_rikku_steals` | `g_StealCount` (`Battle_Steal`; achievement 4 at 200) |
| `0x3db4` | `u32` | `bribe_gil_spent` | `g_BtlGilThrown`: gil thrown with command `0x302a` (`MsParseCommand`, achievement `0x13` at 100000) |
| `0x3dcc` | `u8[0x100]` | `event_flags` | `g_SaveEventFlags` |
| `0x3ecc`, `0x40cc` | | `inventory_ids`, `inventory_counts` | `g_SaveItemIds`, `g_SaveItemCounts` |
| `0x41cc`, `0x41ec` | 16 × `u16` | `items_acquired`, `items_used` | `g_SaveItemCheck`, `g_SaveItemUse` |
| `0x420c` | `u8[0x200]` | `monsters_captured` | `g_SaveCaptures` |
| `0x440c`, `0x444c` | 32 × `u16` | `monsters_seen`, `monsters_defeated` | `g_SaveMonsterMeet`, `g_SaveMonsterDefeat` |
| `0x448c` | 8 × `u16` | `key_items` | `g_SaveImportant` |
| `0x449c` | 200 × 22 | `equipment` | `g_Equipment` (`docs/formats/kernel.md`) |
| `0x55cc` | 18 × `0x94` | `ply_saves` | `g_PlySave` (below) |
| `0x6034` | 18 × 22 × `u16` | `commands_used` | `g_SaveCommandUse` |
| `0x634c` | 18 × 20 | `character_names` | `g_SaveChrNames` |

**`ply_save`** (`0x94` bytes, one per character; `MsSetSaveParam` in
`docs/formats/battle.md` rebuilds most of it):

| Offset | Type | Fahrenheit | Evidence |
|---|---|---|---|
| `0x00` | | `name` | Fahrenheit, not checked |
| `0x04`, `0x08` | `u32` | `base_hp`, `base_mp` | `MsSetSaveParam` |
| `0x0c`-`0x13` | 8 × `u8` | `base_strength` ... `base_accuracy` | `MsSetSaveParam` |
| `0x14`, `0x18` | `u32` | `total_ap`, `ap` | `MsSaveAddAP_00784610` adds to both |
| `0x1c`-`0x28` | 4 × `u32` | `hp`, `mp`, `max_hp`, `max_mp` | `MsSetSaveParam` |
| `0x2c` | `u8` | `ply_flags` | bit 0 joined (`MsGetSavePlyJoin`); bit 4 `joined` Fahrenheit, not checked |
| `0x2d`, `0x2e` | `u8` | `wpn_inv_idx`, `arm_inv_idx` | weapon and armour slots |
| `0x2f`-`0x36` | 8 × `u8` | stats | `MsSetSaveParam` |
| `0x37` | `u8` | `poison_dmg` | Fahrenheit, not checked |
| `0x38`-`0x3a` | `u8` | `limit_mode_index`, `limit_charge`, `limit_charge_max` | Fahrenheit, not checked |
| `0x3b`, `0x3c` | `u8` | `slv_available`, `slv_spent` | `+0x3b` the S-level `MsSaveAddAP_00784610` raises (to 99) |
| `0x3d` | `u8` | `battles_until_recovery` | Fahrenheit, not checked |
| `0x3e` | 6 × `u16` | `abi_map` | commands learnt (`MsSetSaveParam`) |
| `0x4a` | 3 × `u16` | `auto_ability_effects` | the equipment's auto-ability bits (`MsSetSaveParam`) |
| `0x50`-`0x5c` | 4 × `u32` | `battle_count`, `enemies_defeated`, `deaths`, `limits_charged` | `MsSaveAddInfoData` (`0x00785a00`, kinds 0-3; nothing in `g_BtlBattleType` 2) |
| `0x60` | 20 × `u16` | `limit_mode_counters` | Fahrenheit, not checked |
| `0x88` | `u32` | `obtained_limit_modes` | Fahrenheit, not checked |

### Fields the event scripts explain

No FFX.exe function reads these fields; the event scripts do, through
variable storage 0, which starts at `g_SaveData + 0x1ec` (a script's storage 0
offset `x` is block offset `x + 0x1ec`). The meanings come from
[Atel#](https://github.com/yutamago/ffx-atel-sharp) (`docs/research/save-fields.md`,
`src/Ffx.Atel.Game/Save.cs`), which decompiles all 1,621 shipped scripts: a
field is listed only when the dialogue shown next to it, or the only code
that writes and reads it, says what it is. The names are Atel#'s.

| Offset | Storage 0 | Type | Atel# name | What the scripts show |
|---|---|---|---|---|
| `0x3f4` | `0x0208` | bits | `LightningDodgingRewards` | Thunder Plains (`kami0000`, `kami0300`): bits 0-5 set when the highest dodge streak (`lightning_dodging_*`, `0x3fc`-`0x400`) reaches 5, 10, 20, 50, 100, 150; bits 6, 7 when the bolts struck reach 30, 80 (200 in a row sets a Thunder Plains progression bit instead). The agency chest (`kami0400`) gives one reward per set bit, with a message naming it |
| `0x402` | `0x0216` | bits | `LightningDodgingRewardsClaimed` | the same bits: the chest sets the one it gives and gives only those clear here |
| `0xc36` | `0x0a4a` | bits | `SaveSphereTipsShown` | each save sphere shows the "Save Sphere Level 1/2/3" tip while bit 0/1/2 is clear, then sets it |
| `0xc4c` | `0x0a60` | `u8` | `AlBhedPrimersFound` | each primer adds 1 and picks the rank message by it |
| `0xc54` | `0x0a68` | `u32` | `LastSaveSphereMap` | each save sphere stores the map (`GetNowEventJumpId`); the blitzball exit returns there with native `0x00ab` (map, entrance), the airship's scripts set its location from it |
| `0xc58` | `0x0a6c` | `u32` | `LastSaveSphereEntrance` | the save sphere's own constant (1-8); read only as that entrance |
| `0xc74` | `0x0a88` | 6 × `u8` | `BlitzballTeamSizes` | players per team; team 5 is the Besaid Aurochs (the stadium is refused while `[5] < 6`, signing adds 1, at most 8) |
| `0xc86` | `0x0a9a` | `u8` | `AlBhedPrimerTutorialShown` | the first primer shows its introduction while 0, then sets 1 |
| `0x11ec` | `0x1000` | 60 × `u32` | `BlitzballPlayerTechsLearnedPage1` | per player, bit n: tech n known (the scout screen lists it by name) |
| `0x12dc` | `0x10f0` | 60 × `u32` | `BlitzballPlayerTechsLearnedPage2` | the same for techs 31-60 |
| `0x1452` | `0x1266` | 60 × 5 × `u8` | `BlitzballPlayerTechs` | each player's five tech slots on the scout screen |
| `0x157e` | `0x1392` | 60 × `u8` | `BlitzballPlayerTechSlots` | slots in use, set after a match from the level: 0 below 3, then 1, 2, 3, 4, 5 from 3, 7, 12, 20, 30 |
| `0x15ba` | `0x13ce` | 60 × `u8` | `BlitzballPlayerLevels` | the "Level:" line; the stats are computed from it |
| `0x1606` | `0x141a` | 6 × 8 × `u8` | `BlitzballTeamRosters` | player numbers per team (team 5 the Aurochs) |
| `0x163e`, `0x163f` | `0x1452`, `0x1453` | `u8` | `BlitzballAurochsScore`, `BlitzballOpponentScore` | the scoreboard; a win is `0x163e > 0x163f` |
| `0x1716` | `0x152a` | 60 × `u8` | `BlitzballPlayerContracts` | games left on the contract; 255 is permanent ("Cannot release that player") |
| `0x1752` | `0x1566` | `u16` | `BlitzballAurochsWins` | wins, up to 65000; the team level shown is wins / 5 + 1 |
| `0x1a0c` | `0x1820` | 60 × `u32` | `BlitzballPlayerUncoveredTechsPage1` | techs seen but not learnt (shown as unknown) |
| `0x1afc` | `0x1910` | 60 × `u32` | `BlitzballPlayerUncoveredTechsPage2` | the same for techs 31-60 |

Fahrenheit declares `player_uncovered_techs_page_1` as 240 × `u32`, which
puts page 2 at `0x1dcc`; the scripts declare page 1 as 60 × `u32` (240
bytes) and read page 2 at `0x1afc`, right after it, ORing it into the scout
screen's list for techs 31-60 as page 1 for techs 1-30.

The dark aeon bytes in `0xc38`-`0xc91` (`g_SaveInterValefor` among them):
bit 7 is set by each area's battle event when its dark aeon is beaten, and
the Penance unlock checks all eight.

## The load machine

`SaveData_Update` (`0x00648480`, every frame from `Sg_MainLoop`) runs the
save/load state machine `SaveDataMachine_Update` (`0x006f0950`) on
`g_SaveMachineState` (`0x00ce72e8`). Its Load states, as the title's Load
menu goes through them (`load_trace`):

| State | Function | Does |
|---|---|---|
| `0xb` | `SaveDataMachine_BuildList` (`0x006f0220`) | `SaveDataMachine_ScanFiles` (`0x006f0a70`: opens `ffx_000`..`ffx_199`, notes each one's modification date and time), sorts entries 1..199 newest first (`SortSaveTable_006f0fb0`, `g_SortedSaveTable` `0x00ce7318`, 3 words an entry), fills the Flash list; then `0xc` |
| `0xc` | `0x006f0140` | the list's input (`0x006ef320`): a pick opens the Yes/No (`0xd`), cancel ends (`0x11` with no file) |
| `0xd` | `0x006effa0` | Yes/No; Yes: `0xe` |
| `0xe` | `SaveDataMachine_ReadFile` (`0x006f0070`) | `_wfopen` + `fread` of the picked entry into the buffer (list entry `top + cursor`, `0x00ce72ec` + `0x00ce72f0`, through `g_SortedSaveTable`; one more when there is no autosave); a 30-frame timer (`0x00ce72e4`); then `0xf` |
| `0xf` | `SaveDataMachine_CheckFile` (`0x006efbd0`) | after the timer: `SaveDataCheckCrc`; a bad file is zeroed and reported (`0x10`) |
| `0x11` | `SaveDataMachine_LoadDone` (`0x006f0020`) | closes the Flash screen, `SaveDataSaveLoadSucceed(3)` (`0x00648540`: manager state 3 -> 4, result 3; queues the save's music, buffer `+0xe8` = block `+0xa8`, in `0x00c526d0` and sets the texture video mode from buffer `+0xea` = block `+0xaa`: `SaveDataManager_getRefBuffer` returns the file buffer, header included), `SaveDataSetDialogState(0)`; then `0xa` (idle) |

So list entry 1 is the newest manual save, not `ffx_001`. The harness
sandbox copied the saves with new modification times, all in about the
same second, which made the order of the copies (and the sort's ties)
decide it: in the sandbox, entry 1 was `ffx_001` (2026-10-17), or another
save when the copies fell across a second (menu-save's trace of entry 1,
2026-10-27). Since 2026-10-27 the sandbox keeps the source files' times
(`ffx_harness` `copy_file`), so the list's order is the user's own.
`loadsave` takes the slot (the file) instead.

## From the loaded buffer into the game

`HandleSaveDataScreen` (`0x0081ff80`, every frame from `Sg_MainLoop`)
follows the screen state `g_SaveData_ScreenState` (`0x00ccb994`): 1 starts
the load (`SaveDataToLoad`, `0x00648760` -> `SaveDataManager_doLoad`
`0x006f1ce0`: manager `+0xc` = -1, state 3, `SaveDataMachine_StartLoad`
`0x006f01d0` sets the machine's buffer and size, `0x00ce7308`/`0x00ce7304`,
and state `0xb`), then 2. In 2, once loading finished and succeeded, the Al
Bhed check (`SaveDataIsAlbhedLoaded` -1: `g_SaveLoadedPending`
`0x012fb8c0` = 1 and `0x00647e40`, manager `+0` = 1), state 3:
`SaveDataResetToIdle`, `AtelClearForcePlaceId`, state 4: with the pending
flag, `TkMenuJumpToLoadedScene` (`0x008b4ec0`) and state 0.

`TkMenuJumpToLoadedScene` copies the block (`TkMenuCopyLoadedSave`,
`0x008b54a0`: `memcpy(g_SaveData, buffer + 0x40, 0x68c0)`, then
`setDefaultPlayerName`), refreshes the music, sound and HDD settings from
it, sets `g_TkMenuSaveLoadActive` (`0x018663c4`) and the menu help's load
result (`g_MenuHelpExtArg`, `0x00c5a088`) to 1 and puts the save menu to
sleep. It does not change the map.

The title script does. The title is event `test20` (map `0x17`, which
`Sg_MainInit` `0x00820860` jumps to at boot), on map `titl00`. Its Load
choice:

```
0x00fa(1) 0x00e2(1) 0x00aa(1) 0xc02c(0) AtelSetEventMode(2) FadeOut(15)
Native_MenuOpen_Result(0x40100000)      ; 0x401d: the save list (Sg_MenuSetReserve)
if 0x01fa() != 0:                       ; the load result, g_MenuHelpExtArg
    Native_MusicFadeOut(60)             ; the title music out, the save's queued music in
    Native_FadeOut(1)
    0x01c2()                            ; 0x00873d30: block +0xe0 of 2..5 -> 1
    Native_SetMapFadeFrames(0) Native_SetMapFadeType(0)
    Native_MapJump(-1, 0)
```

`Native_MapJump` (`0x00858460`) calls `AtelSetEventJump2(-1, 0, 0)`
(`0x0086ff40`), which needs the running event to hold control (ctrl work
byte 0 bit 7) and calls `TkSetEventJump` (`0x0088ea60`): a negative map
means the one the block keeps to return to, so map `+0` = `+0xba`, entrance
`+0xc` = `+0xb8` (then 0), previous map `+2` = `+0xc2`, `+0xd` = `+0xce`,
and the request `g_TkEventJumpRequest` (`0x0133084c`) = 1. The scene step
`TkUpdateEventJump` (`0x0088d3d0`, every frame) waits out the fade and calls
`TkInitScene(map)` (`0x0088e070`), which opens the map's event.

The writes the script makes before the menu to the block (`0x00fa`
`TkSetEventNewGame`, `0x00e2` `+0x13` bit 1) are overwritten by the copy;
`0x00aa(1)` sets bit 0 of ctrl work 0 `+0x210`, `AtelSetEventMode(2)` the
event mode in `g_AtelMapIdPage` `+3`.

A developer path does the same in one step: `TkDebugLoadViewerSave`
(`0x008b55e0`) reads a file with `debugLoadForViewer`, copies the block and
jumps with `0x0086fec0` to the map and entrance in `+0xb8`.

## The boot and the title

`Sg_MainInit` jumps to map `0x17` (`test20.ebp`, on `titl00`). That first
visit's script set-up starts a new game's state (`MsSetSaveStartGame`
territory: party, HP and MP natives) and jumps to `memochek` (map `0x15c`,
on `titl02`: the logos and the opening), which waits for Start or the end
of the opening and comes back to map `0x17` (previous map `0x15c`): the
title proper, where the title menu and its Load run.

## `loadsave` (harness)

`loadsave <slot|path>` runs, between two frames on the game thread: the
file into the buffer, manager `+0xc` = -1 and state 3 (what `doLoad` sets),
`SaveDataCheckCrc`, `SaveDataSaveLoadSucceed(3)`, `SaveDataSetDialogState(0)`,
the Al Bhed check, `SaveDataResetToIdle`, `AtelClearForcePlaceId`,
`TkMenuJumpToLoadedScene`, `SaveDataSetScreenState(0)`, then the title
script's natives above (`0x0086f3d0(1)`, `AtelSetEventMode(2)` and the
post-menu ones, called with a script stack laid out as
`AtelPopStackInteger` reads it) with ctrl work 0 as the current one. The
title holds control there, so the jump goes through without forcing it.

When it runs matters (`load_trace direct [early]`):

- at the first visit's open (frame 8), the boot's jump to map `0x17` is
  still pending (`g_TkEventJumpRequest` 1): the scene step enters the
  title map instead, and the title script's set-up then resets the state;
  the save's map never opens;
- later in the first visit the title script's own jump to `memochek`
  overtakes it (the load "succeeds" and `memochek` opens);
- from `memochek` (frame 66) the save's map opened and the game crashed on
  the next frame;
- in the title proper it works every time.

So `screens::load_save_direct` taps Start until `g_SaveData` reads map
`0x17` with previous map `0x15c` and no jump is pending, waits 30 frames
for the title script's set-up, drains the file events (the title's
`test20.ebp` is read again there) and sends `loadsave`; the menus never
open. In fast mode that is about frame 290, against about 600 to 730 for
the menu path's event.

## Menu path against `loadsave`

Same file (`ffx_001`, list entry 1 and slot 1), snapshots 120 frames
after the save's event opened (`load_compare`, virtual clock, lockstep),
image `.data`/`.bss` compared with pointers into the image normalized: one
menu run against one direct run, and the bytes that differ there but
neither between two menu runs nor between two direct runs.

- Identical: `g_SaveData` (`0x0112ca90`, `0x68c0` bytes, with the party
  records at `0x0113205c`), the global random generator (`0x00c42200`), the
  68 generators (`0x01135ee0`), the save/load globals (`0x00ccb990`..), the
  scene variables (`0x01330840`..), the save file buffer, and the ctrl
  works and script caches apart from heap pointers (which differ between
  any two runs).
- Only between the paths (3,668 bytes): what the menus leave behind. The
  save machine's state `0xa`, timer, list cursor and file count, Yes/No
  choice, buffer and size (set again by every save,
  `SaveDataManager_doSave`), the sorted table and the list's dates and
  texts (`g_SortedSaveTable`.., the Flash screens' strings "Load", "Yes",
  "No", play time, after `g_UserHeapArenaSize`), the save manager's UI
  fields past `+0x560`, the pad history of the keys the menus took
  (`g_TkControlerWork` `+0xa8`, `+0x1a8`), the menu module
  (`g_TkMenuSaveLoadActive` `+0x3a0`, `+0x3b1`, a message window flag,
  `g_MenuWaitFrames` -1 against 0: the `0x401d` menu ran), and single
  counters of caches and heaps that grew with the extra screens
  (`g_CacheEntries`, `g_OpSprStack`, `g_BtlEtEffectRead`).

The replays (`tools/replays`) gave the recorded original events and
screenshots with `load direct 1 cdsp0000` (see the commit that switched
them).
