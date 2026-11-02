# PS2 music (`musicNNN.bgm`, `wave_wd.dat`, `iopsound.irx`)

The PC's "Original" soundtrack (`sound_pc/music_ps2/ffx_music_ps2_bank00.fsb`,
`docs/formats/sound.md`) is a recording of the PS2's music output, mastered
again (each track peak-normalised and limited) and IMA-ADPCM encoded. The
PS2 game instead played sequences on the SPU2 through its IOP sound driver.
None of that driver is in FFX.exe; this page records the PS2 formats and
the driver's behaviour from the PS2 disc (source: ffx-demaster
`docs/music.md`, which renders the tracks again from the disc; disc layout
in `docs/formats/ps2-disc.md`). Addresses marked "IRX" are offsets in
`iopsound.irx` relocated to `0x100000`, not FFX.exe addresses.

## Files

Under `_ffx/proj/sound/<region>/` (`us` on the German disc too):

| File | What |
|---|---|
| `music/data/musicNNN.bgm` | 114 sequences, `NNN` = the game's music id |
| `music/data/music.dat` + `.sec` | the same sequences in one file; `.sec`: `u32 sector, u32 size` per id |
| `wave/wave_wd.dat` + `.sec` | the instrument banks (`WD`); `.sec`: `i32` sector per bank id, -1 none |
| `jppc/irx/iopsound.irx` | the IOP sound driver (R3000) |
| `jppc/irx/libsd.irx` | Sony's SPU2 library (the reverb presets) |

The PS2 file number is the music id, so FFX.exe's music id table
(`g_FmodMusic_EventByMusicId`, `0x00b4f1c8`, read by `FmodMusic_getState`
`0x00708be0`; `docs/formats/sound.md`, "Music") maps every PC music event
to its PS2 sequence. Where two ids share an event the renders tell them
apart (the hymn of the fayth is 160, not 149).

## BGM sequence

Header, 0x20 bytes:

| Offset | Type | |
|---|---|---|
| `0x00` | 4 | `BGM ` |
| `0x04` | `u16` | music id |
| `0x06` | `u16` | WD bank id |
| `0x08` | `u8` | tracks |
| `0x09` | `u8` | reverb mode (libsd: 5 hall, 4 studio C, 0xff none) |
| `0x0a` | `i16` | reverb depth (EVOL; 0 keeps the current one) |
| `0x0c` | `u8` | volume (`& 0x7f`) |
| `0x0e` | `u16` | ticks per quarter note (48) |
| `0x10` | `u32` | size |
| `0x14` | `u32` | flags (`0x10000`: notes key off one tick before the next event) |

Then per track a `u32` length and its events: a variable-length delta
(MIDI style), then `command, arguments, delta` repeated. The driver's
command table (IRX `0x17500`, 128 handlers) takes these arguments (b a
byte, v a variable-length number); any other command ends the track:

| Arguments | Commands |
|---|---|
| none | `00`-`04`, `06`, `10`, `18`, `29`-`2b`, `2e`, `2f`, `30`, `41`, `49`, `51`, `60`, `61`, `64`, `65`, `68`, `69`, `7f` |
| b | `05`, `08`, `0a`, `0d`, `12`, `13`, `1a`, `1b`, `20`, `22`, `24`, `26`, `31`-`38`, `3c`-`3f`, `42`, `44`, `46`, `4a`, `4c`, `4e`, `52`, `54`, `56`, `58`-`5b`, `5d`, `6c`, `70`, `71`, `79` |
| b b | `09`, `0b`, `0c`, `11`, `19`, `21`, `39`, `47`, `4f`, `5c`, `62`, `78` |
| b b b | `40`, `48`, `50` |
| v b | `23`, `25`, `27`, `2c`, `43`, `45`, `4b`, `4d`, `53`, `55` |
| v | `28` |

What they do:

| Command | Meaning |
|---|---|
| `00` | end of track |
| `02` | loop point (track 1): snapshot of every track's position, wait, flags and key, and the tempo |
| `03` | jump back to the loop point |
| `04`, `05 n`, `06` | counted loop start, loop end (`n` times, 0 = 256; 4 deep), jump to the loop start |
| `08 b` | tempo (BPM) |
| `09` | tempo slide (kept, never applied by the driver) |
| `0a b`, `0b n b` | reverb depth (EVOL = value << 8), slide of it |
| `10`-`13` | key on (with key and/or velocity) |
| `18`-`1b` | key off |
| `20 p` | program (instrument of the WD bank) |
| `21` | another bank |
| `22`/`24 b` | track volume / expression; `23`/`25 v b` slides |
| `26 b`, `27 v b` | pan (+0x40), slide |
| `28 v`, `29`, `2c v b` | portamento time, off, pitch slide |
| `2a`, `2b` | tie on, off |
| `2e`, `2f` | no automatic key-off on, off |
| `30` | ADSR back to the region's; `31`-`39` override ADSR fields |
| `3c b` | sustain pedal |
| `3d b` | first voice to allocate |
| `40`/`48`/`50 d r w` | pitch / volume / pan LFO (delay, rate, wave); `41`-`47`, `49`-`4f`, `51`-`56` off, depth, rate, wave, delay+fade and their slides |
| `58`, `59` | pitch scale set, add |
| `5a`, `5b` | detune set, add (`<< 8`) |
| `5c lsb msb` | pitch bend; `5d` bend range |
| `60`, `61`, `62 a b` | the track's voices dry + wet, dry only, chosen per core |
| `64`/`65`, `68`/`69` | noise on/off, pitch modulation on/off |
| `70 b` | delta offset (clamped to +-9999) |
| `78` | random variation |

## WD instrument bank

Header: `WD`, `u16` bank id, `u32` sample data size, `u32` instruments,
`u32` regions, `u32` SPU address (written by the driver), then at `0x20`
one `u32` offset per instrument to its first region. A region is 0x20
bytes:

| Offset | Type | |
|---|---|---|
| `0x00` | `u32` | flags: bit 0 stereo pair with the next region, `0x100` first, `0x200` last, `0x10000` key/velocity split |
| `0x04` | `u32` | sample offset |
| `0x08` | `u32` | loop offset |
| `0x0c` | `u16` | ADSR1 |
| `0x0e` | `u16` | ADSR2 |
| `0x10` | `u16` | unused by the driver |
| `0x12` | `i16` | tuning in 1/256 semitone (key 60 + tuning plays the sample at 48 kHz) |
| `0x14` | `u8` | highest key |
| `0x15` | `u8` | highest velocity |
| `0x16` | `u8` | volume |
| `0x17` | `u8` | pan (bit 7: use `(pan & 0x7f) + 0x40`) |
| `0x18` | `u8` | reverb: 1 wet only, else wet + dry |

The SPU2 4-bit ADPCM sample data follows the regions.

## The driver (`iopsound.irx`)

- Timing: a hardware timer at 36.864 MHz / 153600 = 240 Hz. Each interrupt
  first sends the key-offs and the voice updates queued by the last tick
  (volumes, pitches, mix switches, `KON`), then subtracts the tempo (1
  until the first `08`) from a counter and runs one tick for each 300 it
  falls below 1: one tick per 1 / (0.8 BPM) seconds at 48 ticks a quarter.
- Loops: at the end of all tracks or at `03` the song restores the `02`
  snapshot and ticks again at once. Volumes, pans and programs are not
  restored.
- Voices: 48 (24 per core). A note takes the first voice whose envelope
  (`ENVX`, read back every interrupt) is 0, else the quietest. Stereo
  regions take two voices, the first panned left.
- Volume: `(song volume + 1) * 511 >> 7`, times expression, velocity,
  track volume and region volume in the driver's fixed point; pan through
  a 256-entry table in the module (0x40 left, 0x80 centre, 0xc0 right).
- Pitch: `key * 256 + detune + bend + tuning` through the module's
  semitone and 1/256-semitone tables (`floor(2^(n/12))`), vibrato from the
  pitch LFO (10-bit phase; sine, triangle, saw, square, random).
- Slides: step `((target - current) << 12 | 0x800) / n` per tick.
- Song-wide modifiers the PC's empty FMOD commands (`musicTempoDirect`,
  `musicPitchDirect`, ...; `docs/formats/sound.md`, "FMOD command queue")
  stood for: the tempo modifier at IRX `0x00222450` and the global pitch at
  IRX `0x00222440`.
- Core 1 mixes core 0's output dry (`MMIX` `0xfc0` / `0xfcc`).
- Reverb: `libsd`'s preset registers and work-area sizes (addresses x4 on
  the SPU2), the PS1/SPU2 algorithm at 24 kHz.

The IRX functions: one tick `0x00100824`, slides and LFO delays
`0x001057a0`, queue a key-on `0x00102d50`, key-off of the current key
`0x00102e70`, pedal-aware key-offs `0x00102bec`, voice allocation
`0x00101140`, region for key and velocity `0x00101c70`, note start
`0x00102468` + `0x00101d20`, the per-interrupt voice update `0x001025e8`,
voice volume registers `0x001014b4`, pitch register with vibrato
`0x00101810`.

Three tracks (Otherworld, Confrontation, Jecht's theme) had guitar and
vocal parts streamed from the disc next to the sequence
(`BattleStreamingStart` in the driver); their sequences alone lack them.

Checked by rendering 86 tracks and comparing them with the PC's recording:
70 correlate >= 0.9 window by window (median 0.97), loop periods agree to a
few ms in 60 (battle theme 78.317 s against 78.318 s). A bank written in
the PC's format with these renders (same names and order, the 32 bytes at
`0x1c` copied) plays in the game in place of the recording.
