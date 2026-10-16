# Sound banks (`sound_pc/**/*.fsb`)

The PC version plays sound through FMOD Ex (`fmodex.dll`, `fmod_event.dll`):
each sound folder has an FMOD Designer event file (`.fev`, RIFF `FEV FMT`)
and a sample bank (`<name>_bank00.fsb`). `FFX_Data.vbf` holds 1,260 banks
with 33,362 samples:

| Folder | Banks | What |
|---|---|---|
| `sound_pc/music/` | 1 (89 samples, 554 MB) | the remastered soundtrack, named (`1_09_battle_theme`), with loop points |
| `sound_pc/music_ps2/` | 1 (89 samples) | the original soundtrack |
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
unused byte, then 32 bytes of 4-bit codes, low nibble first: 64 samples.
A stereo block is 0x48 bytes: both channels' 4-byte headers, then their
codes in alternating 4-byte words (left, right, left, ...). The step and
index tables are the IMA standard's. Decoded with whole 0x24-byte blocks per
channel instead, a stereo bank is noise (mean step between samples 84 and
983 against 23).
