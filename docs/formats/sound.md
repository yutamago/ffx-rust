# Sound banks (`sound_pc/**/*.fsb`)

The PC version plays sound through FMOD Ex (`fmodex.dll`, `fmod_event.dll`):
each sound folder has an FMOD Designer event file (`.fev`, RIFF `FEV FMT`)
and a sample bank (`<name>_bank00.fsb`). `FFX_Data.vbf` holds 1,260 banks
with 33,362 samples:

| Folder | Banks | What |
|---|---|---|
| `sound_pc/music/` | 1 (89 samples, 554 MB) | the remastered soundtrack, named (`1_09_battle_theme`), with loop points |
| `sound_pc/music_ps2/` | 1 (89 samples) | the original soundtrack: a recording of the PS2's output, each track peak-normalised and limited (the PS2 sequences and banks: `ps2-music.md`) |
| `sound_pc/voice/us/`, `voice/jp/` | 25 each | voice |
| `sound_pc/sfx/us/`, `sfx/jp/` | 604 each | sound effects, one bank per number (`0201_bank00.fsb`) |

Read by `ffx_sound::fsb` and `ffx_sound::ima`; `ffx sound list <bank>` and
`ffx sound wav <bank> -o <dir> [names]` export them. The banks are facts
of the data: the reader is derived from them and checked on every bank
(`crates/ffx-sound/tests/game_data.rs`), not from FMOD.

## FSB5 bank

Little-endian; every bank is version 1, codec 7.

| Offset | Size | |
|---|---|---|
| 0x00 | 4 | `FSB5` |
| 0x04 | 4 | version (1) |
| 0x08 | 4 | samples |
| 0x0c | 4 | size of the sample headers |
| 0x10 | 4 | size of the name table |
| 0x14 | 4 | size of the data |
| 0x18 | 4 | codec (7: IMA ADPCM) |
| 0x1c | 32 | flags and a hash |
| 0x3c | | sample headers, then the name table, then the data |

A sample header is 64 bits: bit 0 chunks follow, bits 1–4 the rate index
(1 8000, 2 11000, 3 11025, 4 16000, 5 22050, 6 24000, 7 32000, 8 44100,
9 48000 Hz), bit 5 stereo, bits 6–33 the data offset / 16, bits 34–63 the
frames. Each chunk is a 32-bit word (bit 0 another follows, bits 1–24 size,
bits 25–31 type) and its bytes; type 3 is the loop (start and end frame,
`u32` each), 1 the channel count, 2 the rate. A sample's data runs to the
next sample's offset (the last to the end of the data).

The name table is one `u32` offset per sample (from the table's start),
then the zero-terminated names.

## IMA ADPCM, Xbox layout

A mono block is 0x24 bytes: predictor (`i16`), step index (`u8`), one
unused byte, then 32 bytes of 4-bit codes, low nibble first: 64 samples,
the predictor itself and then the first 63 codes (the 64th code is never
output). Measured against FMOD's mix (`audio-platform.md`, "FMOD Ex's
mixer"): decoding all 64 codes after the header shifts every sample by
one and shows wherever a block's header differs from the previous block's
last code (menu-browse, SFX 106, frame 448).
A stereo block is 0x48 bytes: both channels' 4-byte headers, then their
codes in alternating 4-byte words (left, right, left, ...). The step and
index tables are the IMA standard's. Decoded with whole 0x24-byte blocks per
channel instead, a stereo bank is noise (mean step between samples 84 and
983 against 23).

# The game's sound layers

From the game's side down (`crates/ffx-sound`: `snd`, `fmod`):

1. **`Snd*`** (`0x00885d00..0x008879d0`), the PS2 sound API the game code
   calls. The PC build keeps these functions in alphabetical order (as the
   PS2 object file had them), which names most of them against the Switch
   build's symbols. Ported in `ffx_sound::snd`.
2. **`Sqs*`** (`0x0081ce70..0x0081f900`), the PS2 sequencer layer; on PC
   mostly thin calls into 3 (`SqsSePlay(kind, id, pan, volume)` ->
   `iSfxStart(id, volume & 0x7f, pan & 0x7f)`), some stubs that print "not
   implemented" (`SqsSePitchDirect`, `SqsSePlayState`).
3. **`i*`** (`0x00679fb0..0x0067a7d0`): `iMusic*`, `iSfx*`, `iVoice*`,
   `iSound*`; each queues one FMOD command. Ported in `ffx_sound::fmod`.
4. **The FMOD thread**: `FmodManager` runs the queue at 30 Hz (33.3 ms,
   `0x006fa700`) through the handler table at `0x00c3a368` (70 commands),
   each calling a `FfxFmod` method, which drives `FmodMusic`, `FmodSfx`
   (`SfxBank`, `SfxEvent`), `FmodVoice` and `FmodShout` over FMOD Designer
   events. Left original (FMOD calls).

## FMOD command queue (`Fmod_PushCommand` 0x006fa300)

`g_FmodManager` (`0x00ce9010`) is a 0x50-byte object: `+8` the `FfxFmod`,
`+0x10` the queue mutex, `+0x14` the sync mutex, `+0x18`/`+0x1c` events,
`+0x28`/`+0x2c` the ring's write/read index, `+0x30` the sync command,
`+0x34..+0x40` its four arguments, `+0x44` (u8) done, `+0x48` its result.

The ring is 150 entries of 0x14 bytes at `0x00ce8458`: `u32 cmd, a, b, c,
d`. `Fmod_PushCommand(mgr; cmd, a, b, c, d)` appends one; commands 0x3b
(sfx volume) and 0x3d (sfx pan) replace a queued one with the same `a`
(the sfx id) instead; 0x31 (unload all sfx) is dropped while `0x01325b7c`
is set; a full ring prints every queued command ("Too many fmod commands at
the same time!!"). `FmodManager_registCommandAsync1/./3/4`
(`0x006fa280..0x006fa2e0`) pass 1-4 words and zeros.

`FmodManager_registCommandSync(mgr; cmd, ...)` (`0x006fa500`) runs a
command and returns its result: 0x11-0x13, 0x16, 0x19 and 0x41 on the
calling thread, the others by waking the FMOD thread and waiting twice 5
ms ("registCommandSync TimeOut"); 0x3f also answers 1 while a 0x33 start
for that id is still queued. Wrappers `FmodManager_registCommandSync1..4`
(`0x006fa480..0x006fa4e0`).

| cmd | queued by | | cmd | queued by |
|---|---|---|---|---|
| 0x0b | `iVoicePanTranslate` | | 0x23 | `iMusicVolumeDirect(id, v, 1)` |
| 0x0c | `iVoicePanDirect` | | 0x24 | `iMusicMuteSet` |
| 0x0d | `iVoiceVolumeTranslate` | | 0x25 | `iMusicClear` |
| 0x0e | `iVoiceVolumeDirect(v, 1)` | | 0x26 | `iMusicStop` |
| 0x0f | `iVoiceStart(pan, v)` | | 0x27 | `iMusicStart` |
| 0x10 | `iVoiceRead(id)` | | 0x28 | `iMusicRead` |
| 0x11 | `iVoicePlayStatus` (sync) | | 0x2c | `iMusicStopAll` |
| 0x14 | `iMusicSyncStandby` (sync) | | 0x2d | `iMusicClearAll` |
| 0x15 | `iMusicDataSend(id, id >> 31)` | | 0x2e | `iMuiscSetBgmType(0/1)` |
| 0x16 | `iMusicPlayState` (sync) | | 0x2f | `iMusicSetPaused` |
| 0x17 | `iMusicCrossPlay(id, frames)` | | 0x30 | `iSfxLoadData(id, 1)` |
| 0x18 | `iMusicPlayFast` | | 0x31 | `iSfxUnloadAllData` |
| 0x19 | `iMusicReplayPointSet` | | 0x33 | `iSfxStart(id, volume, pan)` |
| 0x1a | `iMusicReplayPointClear` | | 0x34 | `iSfxForceStart` |
| 0x1b | `iMusicPhrasePlay` | | 0x35 | `iSfxDelayStart` |
| 0x1c | `iMusicVolumeFadeOut(id, frames)` | | 0x3a | `iSfxPause` |
| 0x1d | `iMusicTempoTranslate` | | 0x3b | `iSfxSetVolume(id, v)` |
| 0x1e | `iMusicTempoDirect` | | 0x3d | `iSfxSetPan(id, pan)` |
| 0x1f | `iMusicPitchTranslate` | | 0x3e | `iSfxSetPanTranslate` |
| 0x20 | `iMusicPitchDirect` | | 0x3f | `iSfxIsPlaying` (sync) |
| 0x21 | `iMusicMultiPlay` | | 0x40 | `iSfxPlayBattleStreaming` |
| 0x22 | `iMusicVolumeTranslate(id, frames, v)` | | 0x41 | `iSfxIsPaused` (sync) |

Volumes and pans are 0..127 on the game side; the FMOD side makes volume
`v / 127` clamped to 0..1 (`0x0070f1a0`) and pan `v/127 + v/127 - 1`
clamped to -1..1 (`0x0070ef80`); a volume translation over `n` frames runs
`n / 30` seconds (`0x0070f000`). `iMusicStart` first records the id at
`FmodMusic +0x18` and raises the start signal at `+0x20`
(`FmodManager_signalMusicStart`, for music ids below 0xb6).

`iSoundUpdate()` (`0x0067a600`) applies the PC settings (`g_Settings`,
`0x022fb504`): the volumes at `+0x14`, `+0x1c`, `+0x18`, `+0x20` (0..100;
all 0 while `+0x10`, sound off, is set) go out as commands 0x42, 0x43,
0x44, 0x45; the first also to the movie player
(`PhyFMVPlayerManager_SetVolume_006dabf0`) as `f32(f32(v) / 100.0)`.

## The FMOD thread (`FfxFmod`, `ffx_sound::ffxfmod`)

The FMOD thread runs each queued command through the handler table at
`0x00c3a368` (`g_FmodCommandHandlers`): 70 cdecl `FmodCmd_*(FfxFmod*,
u32 args[4])` (`0x00707b90..0x00708460`), each calling one `FfxFmod`
method of the same name. Handlers 0x00-0x2f answer 0 (but 0x05, 0x11,
0x12, 0x13 and 0x16, which return the method's result); 0x30-0x45 return
the method's `eax`. Conversions in the handlers: 0x0d and 0x22 turn the
frame count into `ftol(ceil(f32(i32 x) * 0.5) + 0.5)` (f32 stores between
steps, CRT `ceil` at import slot `0x00b0c480`); 0x0e and 0x23 pass the
volume as `f32(i32 x)`; 0x42-0x45 pass `f32(f32(i32 x) / 100.0)`; 0x37
widens `a1:a0` to 64 bits but passes only `a0`.

`FfxFmod` (`FmodManager +8`): `+4` FMOD `EventSystem*`, `+8` low-level
`System*`, `+0xc` `FmodMusic*`, `+0x10` `FmodVoice*`, `+0x14` `FmodSfx*`,
`+0x18/+0x1c/+0x20/+0x24` master/music/voice/sfx volume (f32 0..1). Its
methods (`0x007067d0..0x00707580`, alphabetical like every PC source file,
so named after the Switch build's `FfxFmod::*`) mostly forward to one
object; empty on PC: `musicMuteSet`, `musicPitchDirect/Translate`,
`musicPlayFast`, `musicTempoDirect/Translate`, `voicePanDirect/Translate`
(`xor eax,eax; ret n`). `musicMultiPlay` and `musicPhrasePlay` are a plain
`FmodMusic::start` for ids <= 0xb5. The volume setters clamp to 0..1
(NaN stays), do nothing when the value equals the kept one (exact f32
compare), else keep it and hand `f32(x * master)` to the objects'
`setEntireVolume` (all three for the master). `soundUpdate` (cmd 2) is
`EventSystem::update` + `ERRCHECK(r, "FfxFmod.cpp", 0x121)`; `soundLoop`
(cmd 3) measures the elapsed time and updates the objects. `soundInit`
(cmd 1) reads the language from `LocalizationManager_GetInstance()`:
`FmodSfx +2` = English for languages 1 and 2; `FmodVoice +4` (English) is
0 only for languages 0, 3 and 4.

The objects: `FmodMusic` `+0x10` 0xb6 records of 0x3c bytes (`+0` the FMOD
`Event*`, `+4` replay point, `+8` fading, `+0x20` play state), `+0x18` the
playing music, `+0x20` start signal, `+0x28` the voice's first word.
`FmodVoice` `+0x14` `Event*`, `+0x40/+0x44` play status (`+0x44` start
signal), `+0x50` voice volume, `+0x54` paused, `+0x80` mute id, `+0x84`
`FmodShout*`, `+0x88` entire volume. `FmodSfx` `+0` paused (byte), `+2`
English, `+0x64` a binary tree of the playing `SfxEvent`s by sound id
(`SfxEventTree_Find` 0x0070ba70: head `+0xc` the root, the head the leaf,
node `+0` left (greater ids go left), `+4` right, `+0xc` id, `+0x10`
`{count, _, SfxEvent**}`). `FmodSfx::setPan/setVolume/stop` apply to every
event of the id; `start/forceStart/delayStart` need `canStartNewEvent`
(no event of the id playing; id 1 always) and a new event from
`createSfxEvent`, else print a message. `stopNonBlockingSound` only stops
ids above 0x7f. The FMOD libraries are C++ `__stdcall` members (`this` the
first stack word), called through the jmp thunks at `0x00948c70..`
(`Event::getState` `0x00948ca6` -> slot `0x00b0c750`, `Event::setPaused`
`0x00948cbe` -> `0x00b0c740`, `EventSystem::update` `0x00948c82` ->
`0x00b0c768`); a state is "playing" with flag 8.

## Snd state

| Address | | |
|---|---|---|
| `0x0132f690..0x0132ff10` | `g_SndFootSepWork` | footstep sound ids by kind, below |
| `0x0132ff10` | u32 | voice read in progress (`g_TkSndVoiceFlags`) |
| `0x0132ff14` | u32 | shout stream busy (`SndStartShoutStream`) |
| `0x0132ff18` | u32 | VAG voice stream busy (`SndStartStream`) |
| `0x0132ff1c` | u32 | standby voice |
| `0x0132ff20` | u32 | `SqsDataSendingCompleted` result |
| `0x0132ff24` | u32 | `g_TkSndStatus` (busy bits) |
| `0x0132ff28` | u32 | voice sector length (`SndStreamDVDPos`) |
| `0x0132ff2c`, `34`, `3c`, `44` | u32 each | party bit masks: 3D footsteps, footsteps, cloth (kind 4), events (kind 0xff); each followed by its pushed copy |
| `0x0132ff4c` | u32 | kick stay: kinds 0-7 become 8-15 |
| `0x0132ff50` | ptr | 3D footstep request (`+0` u16 flags, `+0x18` sound, `+0x1c` self, `+0x20` u16 actor) |
| `0x0132ff54` | ptr | sound event table: `u32 count, u32 sound[count]` |
| `0x0132ff58`, `0x0132ff60` | u32, u32[16] | IOP error bits, a status per newly raised bit |
| `0x0132ff5c` | u32 | music playing, -1 none |
| `0x0132ffa0` | i32[16] | actor per party slot |
| `0x0132ffe0`, `0x01330068` | 17 x 8 bytes, i32 | music clear queue, count |
| `0x0133006c..0x01330088` | u32 | status/shout/VAG words: previous and this frame (`SndSqsIopFrame` moves them) |
| `0x01330090..0x01330097` | u8 | music mute, SE lock, sound mode (0-2), sound type (0-1), kick flag, last kick, SPU sort switch, HDD error |
| `0x0133009c` | u32 | battle music data size |
| `0x013300e0` | u8[256] | volume per music |
| `0x00c54dd0` | u32 | last faded-out music (-1 in the image) |
| `0x00c54db0` | i16[16] | default common waves (0, 1, 0x7d5, then -1) |

**BGM type** (`SndSetBgmTypeSave(on)`, `0x008cc150`, from script natives
0x0264 and 0x0267): the save data's byte `+0x5ec`
(`MsGetSaveEventAddress`) becomes `on != 0`; `iMusicRefreshBgmType`
(`0x008cc170`) then queues `iMuiscSetBgmType(byte & 1)` to FMOD.

**Sound-effect ids**: bit 31 clear is a separate SE (kind 1), set a block SE
(kind 0); every `SndSep*` entry passes `(kind, id & 0x7fffffff)` to `Sqs*`.
`SndSepClear` clears only separate SEs.

**Footsteps** (`SndGetFootSepWork(kind, party, variation)`): 16 kinds;
kinds 0, 1, 5, 8, 9, 13 have `[16][4]` u32 slots (four variations), the
others `[16]`. Party clamps to 0..15, variation to 0..3. `SndKickFoot(kind,
party, kick, variation)` plays slot + variation (at most 3; 1 for kinds
4/12; 0 for 6/7/14/15) when the party bit is in the kind's mask; kind 0xff
plays sound event `variation` under the event mask. A party slot in the 3D
mask with a non-negative actor goes through the 3D request instead.

**Music clear queue** (`SndExecMusicClearQue(mode, id)`, every frame with
mode 1): entries `{i16 music (< 0: an SE), i16 frames, u32 sep}` count down
and clear the music (`SndMusicClear`) or stop the SE (`SndSepStop`) at 0;
mode 0 cancels music `id`, -1 SE `id`. The compaction keeps the original's
shift (it copies `count - i` entries past each removed one).

**Music**: ids below 0x100 (0xb6 on the FMOD side). Starting a music sets
its volume byte to 0x7f and registers it as playing; `SndMusicCrossStart`
starts at volume 0 when it is the music last faded out and nothing plays;
every volume is 0 while muted.

**Voice streams**: `SndStreamDVDPos(id)` looks voice `id >> 12` up as three
two-digit indices (`/10000`, `/100`, `%100`) in `g_SndStreamTable`
(`0x023cbec4`: u16 group count and offsets; per group a u16 count and
offsets; per list a u16 count and sectors, 0xffff gaps), returning `(group
<< 16) | sector` and the length to the next sector. `SndStreamPCName(id)`
names the file: `ve%06d` (`vj` for languages above 6) of `id >> 12`, then
`0x60 +` bits 6-10 and 0-4 as two letters over characters 8 and 9, then
`.vs`. `SndCalcVoiceWait` turns the sector length into frames (x2048, -32,
/48000, x16/28, x2016/2048, x4.97, each stored as f32), 0 in video mode 0.

**Positioned sounds** (`Snd3dFootRequest` 0x00876950, `ffx_sound::foot3d`):
a request is `{id, params, position or u16 actor, direction}` with the
result bytes at `+0x20` (pan) and `+0x21` (volume); params are `u16`
flags (1 direction, 2 the event actor's position, 4 the second camera
matrix, 0x8000 no height in the distance), the far and near volume bytes
(`+2`, `+3`), the pan scale (`+4`) and the volume's and pan's near/far
distances (`+8`/`+0xc`, `+0x10`/`+0x14`). The position goes through the
camera matrix in the emulated VU0 registers (`0x00c8f890`, copied from
`0x01328288` or `0x013282c8` when `g_AtelCameraMatrixFlag` changes),
moves toward the direction's nearest point (`Math_SegmentNearestT`), and
its distance picks the volume (the near byte, the far byte, or both mixed,
times `g_AtelSnd3dVolumeTotalRatio`; only the mix is clamped to 0..127).
The pan is `ftol((pi + angle) * 127 / 2pi)` of `Math_Atan2XZ(0, 0, z, x)`
folded into [-pi/2, pi/2] and scaled by the pan ratio. Requests with flag
2 for an actor that is not of type 2 or 5 but flag 1 set read a direction
the function never wrote (its stack frame); the port uses 0 there (brief
2026-10-24).

# FMOD Designer event projects (`*.fev`)

Read by `ffx_sound::fev` (`Project::parse`). The layout below is derived
from the data: every one of the 1,260 `.fev` files in `FFX_Data.vbf` (SFX,
voice and music, US and JP: 33,362 events, one per FSB sample) parses to
the exact end of its `LGCY` chunk, and the constants stated here hold in
all of them (`tests/game_data.rs`, `every_fev_parses`). The samples
themselves are in the FSB banks above.

**Container.** RIFF, form `FEV `; chunks are padded to an even size (the
`PROP` of `0303` is 9 bytes and a pad byte). `FMT ` (4 bytes, `u32`
0x00450000: version 4.45), then `LIST PROJ` with `OBCT` (0x10c bytes: 33
pairs `u32 type, u32 count`), `PROP` (`u32 length`, the project name:
`0303`, `ffx_music`), `LGCY` (below), `EPRP` (4 bytes), `STRR` (`u32
count, u32 offset[count]` from the end of the offsets, zero-terminated
strings; entry 0 is the empty string, then group names, event names and
sound definition names `/__simpleevent_sounddef__/9003_0`), `LANG`
(`default`).

**`LGCY`**, all little-endian; a *string* is `u32 length` (with the NUL)
and the bytes; a *name* is a `u32` index into `STRR`:

| Field | |
|---|---|
| `u32`, `u32` | 24 and 32 times the number of events (sizes; not needed to read on) |
| string | project name |
| `u32` | 1 |
| `u32` count, banks | per bank: `u32` mode (`FMOD_MODE` bits: 0x80 streamed for music and voice, 0x200 compressed sample in memory for SFX), `u32` max streams (10), 8 bytes (a hash), `u32` 0, string name (`<project>_bank00`) |
| category tree | string name, `f32` volume (1), `f32` pitch (0), `u32`, `u32`, `u32` child count, the children: always `master` with one child `music` |
| `u32` count, event groups | per group: name, `u32` user properties (0), `u32` subgroups (0), `u32` events, the subgroups, the events |
| `u32` count, sound definition configs | one per project: 18 `u32` and a `u16`, the same in every file (words 3, 5 and 10 are 1, words 4, 6, 7 and 8 are `f32` 1.0, the rest 0) |
| `u32` count, sound definitions | per definition: name, `u32` config (0), `u32` waves (1), per wave: `u32` type (0), `u32` weight (100), string file (`0303/9003.wav`), `u32` bank (0), `u32` sample index in the bank, `u32` length in ms |
| `u32` | 0 |
| `u32` 8, `comp` | the end (a size-8 record: its size word and its tag) |

An **event** (every one is a simple event):

| Offset | Field | Values in the data |
|---|---|---|
| +0x00 | `u32` type | 0x10 |
| +0x04 | name | the sequence number (`9003`), `1_24_the_summoning`, `210101td` |
| +0x08 | 16 bytes | GUID |
| +0x18 | `f32` volume | 1.0 for every SFX; music and voice dB steps (0.5012 = -6 dB, 0.3548 = -9 dB; 15 values) |
| +0x1c, +0x20, +0x24 | `f32` pitch, pitch and volume randomisation | 0 |
| +0x28 | `u32` mode | 0x80 music and voice, 0x100 SFX group `loop`, 0x200 SFX group `nonloop` |
| +0x2c | `u32` max playbacks | 5 for SFX `nonloop`, 30 for sequence 114, 1 for sequence 1 (the game's own instance limits: "5 instances a sequence, 30 for sequence 114, id 1 always"), 1 for every `loop` SFX, music and voice |
| +0x30 | `u32` | 10000 |
| +0x34 | `u32` flags | 0x0018_0008 |
| +0x38, +0x3c | `f32` | 1.0, 10000.0 (min and max distance by the defaults' look; the game has no 3D) |
| +0x40..+0xa4 | 26 words | constant over the data: `u32` 0, `f32` 1500, `u32` 0x80000, `f32` 1, `f32` 1, six 0, `f32` 360, `f32` 360, `f32` 1, `u32` 1, `f32` 1, five 0, `f32` 1, 0, `f32` 1, 0, 0. The fades and the steal behaviour are among them; since none of them varies, the mixer needs no names for them |
| +0xa8 | `u32` layers | 1 |
| per layer | `u16` sound definition, 14 `u32` | word 3: 1 the wave plays once, 0 it loops (every music event but the seven `*_oneshot` tracks loops, as do 947 of the 984 `loop` SFX of the US data; every `nonloop` SFX and voice plays once); word 4 -1; words 1 and 9 `f32` 1.0; the rest 0 |
| | `u32` 1, string | the category (`master`) |

So a looping event loops its wave between the FSB sample's loop points
(the whole wave when the sample has none), and nothing else in the
projects shapes the sound beyond the event volume: no fades, no pitch, no
randomisation, no 3D. Every SFX project has the groups `loop` and
`nonloop`; music is one group `music` of 89 events, voice one group
`voice`.

**Sound effects.** `FfxFmod::sfxLoad(n)` loads `SFX/US/%04d.fev` (`JP` by
the voice language) as an `SfxBank` (`0x0070f760`), then two side files in
the same folder: `%04d_common.txt`, binary pairs `u32 sequence, u32 event
index` for group `nonloop`, and `%04d_loop.txt`, records `u32 sequence,
u32 count, u32 index[count]` for group `loop` (a looping sound is several
events started together). `FmodSfx_Start(seq, volume, pan)` searches the
loaded banks for the sequence and creates the event by index in its group
(`EventGroup::getEventByIndex`); at most 5 instances per sequence (30 for
sequence 114). The index order is the group's event order (the STRR order:
`1010, 1011, 1012, 2001, 40001, 4010, ...`), and the FSB sample of a simple
event has the event's name: sequence 9003 is sample `9003` of
`0303_bank00.fsb`.

**Music.** `ffx_music.fev` (project `ffx_music`, group `music`); a music id
below 0xb6 maps to an event index through the table at `0x00b4f1c8` (92 ids
mapped, -1 otherwise; `0x00708be0`); the events are the named tracks
(`1_09_battle_theme`). `FmodMusic` keeps 0xb6 records of 0x3c bytes (`+0`
event, `+8` state, `+0x24..+0x38` volume translation: active, frames left,
current, target, step). The PC build also tries `Music/MusicIDMapper.txt`
(`FmodMusic_initList`, never called; not in the data).

The table's ids are also the PS2 music file numbers (`musicNNN.bgm`,
`ps2-music.md`), so it ties each event of `ffx_music_ps2.fev` to its PS2
sequence. A replacement `music_ps2` bank with the same sample names and
order, IMA ADPCM codec 7, and the original's 32 bytes at `0x1c` (flags and
hash) copied plays through the unchanged `.fev` (ffx-demaster
`docs/music.md`: a battle's captured mix matched the replacement sample for
sample).

**Voice.** `Voice/VoiceIDMapper.txt` and `Voice/US/VoiceFevMapper.txt`
(`JP`) are `ML` files: `"ML\0\0", u32 count, count x {u32 key, u32
offset}`, zero-terminated strings at the offsets: voice id -> event name
(`210101td`), and key -> project (`ffx_us_voice20`,
`ffx_us_voice_nagisetsu`).
