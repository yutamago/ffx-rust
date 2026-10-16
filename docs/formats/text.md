# Text encoding

All in-game text uses one byte-oriented encoding with per-language
character tables.

Codec: [`crates/ffx-text`](../../crates/ffx-text).

## Character tables

`MallocAndReadGamecode` (`0x00646920`) loads
`ffx_ps2/ffx/master/jppc/ffx_encoding/ffxsjistbl_<t>.bin`, with *t* = `jp`
for Japanese, `kr` for Korean, `ch` for Chinese and `us` for every western
language. The file is UTF-8 text listing the characters in code order:
character index *n* is the *n*th character (`ffxsjistbl_us.bin` starts
`0123456789 !”#$%&’()…`).

The decoders (`DecodingGamecodeJPCHKR`, `0x006460a0`, for jp/kr/ch;
`Text_DecodeCharWestern`, `0x006461f0`, for western languages) hard-code
byte offsets into the table, which only track UTF-8 character widths; they
agree with "the *n*th character" everywhere checked. Both return the output
position plus the bytes they wrote (`ffx_text::gamecode`, checked against
the originals on every code in `crates/ffx-oracle/tests/text.rs`):

- jp/kr/ch: every character is 3 bytes except characters 17 and 40
  (2 bytes); Chinese has no 2-byte character 40, so its offsets past 17
  are all `3n − 1`. A `04` prefix adds `0x410` (second bank).
- western: only codes `0x00`–`0xbf` (the first 192 characters, 1 to 3
  bytes each, back to back: `0x136` bytes); any other code, so every
  two-byte code, writes nothing.
- Codes below the first lead byte (`2b` and lower, never passed by
  `Text_ToUtf8`) come out negative and read before the table.

Two display substitutions exist: in Japanese, codes `0x5c`–`0x5e` show
character 10; in Korean, character 10 is shown as an ASCII space.
`Text_ToUtf8` picks the jp/kr/ch decoder for languages 0 and 9–11
(`TOGetFFXLang`), the western one otherwise; a `04` always goes to the
jp/kr/ch decoder, whatever the language. `ConvertGameCodeToUTF8`
(`0x00645f30`) is `Text_ToUtf8` in the game's language.

## Byte grammar

From `Text_ToUtf8` (`0x00645f50`):

| Bytes | Meaning |
|---|---|
| `00` | End of string |
| `01`, `02`, `05` | Control code, no argument |
| `03` | Line break |
| `04` *c* | Japanese second bank: character *c* + `0x410` |
| `06`–`2b` *x* | Control code with one argument byte |
| `2c`–`2f` *t* | Character `(lead − 0x2b) × 0xd0 + (t − 0x30)` |
| `30`–`ff` | Character `byte − 0x30` |

So `0x30` = "0", `0x3a` = space, `0x50` = "A", `0x70` = "a".

`Text_ToUtf8` writes a line break as `\r\n`, drops control codes and
their arguments, and skips an argument byte even when it is `00` (a string
ending in a bare control code runs on past its terminator).

## Readable form

`Charset::decode` produces text that `Charset::encode` turns back into the
same bytes:

| Written | Bytes |
|---|---|
| characters | their codes |
| `{{`, `}}` | literal braces |
| newline | `03` |
| `{01}` | control code |
| `{0a:41}` | control code with argument |
| `{c:1a2}` | character index without a unique character (a duplicate in the table) |
| `{raw:2c}` | a byte that does not start a valid sequence |

## Message tables

An event's dialogue and a battle's texts are message tables, one per
language, loaded by `TKMallocAndReadEventMessage` (`0x0088cac0`):

- `ffx_ps2/ffx/master/new_<lang>pc/event/obj_ps3/<xx>/<event>/<event>.bin`,
  `<xx>` being the event's first two letters (`zn/znkd0600`). The
  `obj_psv` copies are the Vita's; the PC reads `obj_ps3`.
- `ffx_ps2/ffx/master/new_<lang>pc/battle/btl/<name>/<name>.bin` for
  battle `<name>` (`cdsp00_00`).

`AtelEventSetUp` (`0x00872ef0`) puts the event's table at script context
`+0x34`; without one, section 1 of the `.ebp` (the original Japanese table,
same layout) stands in.

| Offset | Type | Meaning |
|---|---|---|
| `8i + 0` | `u16` | Text offset of message *i*; 0 = message *i* of the `.ebp`'s table |
| `8i + 2` | `u16` | Flags (`Msg_Flags`, `0x0086bf10`) |
| `8i + 4` | `u16` | Alternate text offset (`Msg_TextAlt`, `0x0086ba90`) |
| `8i + 6` | `u16` | Alternate flags |
| … | | The strings, each ending in `00` |

Offsets are from the file start. The first string follows the entries, so
the first offset gives the count; 13 battle tables are entries only, all 0.
In all 3,882 shipped tables (171,253 messages) the strings follow each
other in entry order and every alternate equals its text.

Flags: the high byte is the number of options of a choice, whose options
the text marks with `{10:30}`, `{10:31}`, …; the low byte is 0 except on
number selectors (`0xc1`–`0xc7`), some empty messages (`0x80`) and the
credits (`0xf0`).

`ffx text export znkd0600 --lang en` writes a table as TOML, one
`[[message]]` per message (`index`, `text`, and `flags` when not 0);
`ffx text import` applies edited messages onto the original. Texts can grow
or shrink, and messages can be added at the end for an edited script to
show. The in-game test changes the first line of the opening ("Listen to
my story.") and finds the modded table in the running game; a screenshot
(`examples/mod_probe.rs` with a screenshot folder) showed the new
subtitle.

## Open questions

- What each control code does (colors, inserted names, pauses, …).
