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

## Control codes

What the message windows do with each code, from FFX.exe: `TOGetGroup`
(`0x008df610`) sorts codes into groups, `TOSkipCode` (`0x008dfd80`) steps
over them, `TOSetDrawInfo` (`0x008df8f0`, `ffx_system::tolib::set_draw_info`)
applies the group-2 codes to the pen and `TOGetChangeMacroAddress`
(`0x008b6660`, `ffx_system::tolib::to_get_change_macro_address`) expands
the group-3 ones. The names in the second column are Fahrenheit's
(`FhEncoding` in their `core/encoding.cs`, used with permission).

| Code | Fahrenheit | What the game does |
|---|---|---|
| `00` | `{END}` | end (group 1) |
| `01` | `{BREAK}` | group 1; no argument |
| `02`, `05` | - | no argument; `02` group 1, `05` group 4 |
| `03` | `{LF}` | line break (group 1) |
| `04` *c* | - | second character bank (`TOSkipCode`: 3 bytes when *c* ≤ `0x2f`, else 2) |
| `06` *x* | - | group 4; one argument |
| `07` *n* | `{SPACE:n-30}` | pen position: *n* `0xe0`-`0xef` moves right *n* − `0xe0` pixels; `'1'` (and `'8'` on a window whose `+0x24` is 0) restarts the line (`TOFirstInitMessageNormal`); `'2'` right-aligns what follows (x = window `+0x50` − its width, `TOGetMessageWH`); `0x40`-`0xbf` tabs to window `+0x48` + 4*n* − `0x100` |
| `08` *x* | - | nothing in `TOSetDrawInfo` |
| `09` *n* | `{TIME:n-30}` | `'0'` clears the window's wait (`+0xac`), *n* below `0xc0` sets it to *n* − `'0'`; *n* from `0xc0` sets `+0xb4` to *n* − `0xc0` (the scroll timer `TOTextScrollTick_008df570` counts down) |
| `0a` *n* | `{COLOR:n}` | colour: a low nibble *l* ≠ 0 sets the pen colour byte's (`+0x1b`) low nibble to *l* − 1, a high nibble from 4 its high nibble to (*n* & `0xf0`) − `0x40` |
| `0b` *n* | `{BTN:n}` | button icon *n* (`setButtonData(n, -1)`, drawn at 0.65 or 0.73 by the icon mode); the pen moves by its width |
| `0c`, `0f`, `11` | - | group 4; **`TOSkipCode` steps over one byte**, while `Text_ToUtf8` and Fahrenheit skip an argument too |
| `0d` *x* | - | nothing in `TOSetDrawInfo` |
| `0e` *n* | - | icon mode: `'0'`-`'3'` set mode *n* − `'0'`, `'@'` mode 3, `'A'` mode 1 (with mode 3, the pen moves 5.0) |
| `10` *n* | `{CHOICE:n-30}` | a choice option's mark (`'0'`, `'1'`, …; the message's flags give the count); *n* `0x50`-`0x57` instead writes the window's counter (`+0xc`) as *n* − `0x4f` digits (`TOIntToMes`) |
| `12` *n* | `{VAR:n-30}` | text variable *n* − `'0'` of the window (`g_MsgVars`, set by script natives `0x009e`, `0x009f`, `0x00a0`): a number, an item, weapon, command or ability name (types `0x33`-`0x36`), the field pickup (`0x37`), a dictionary entry (`0x13`-`0x23`) or a number of `type − 0x2a` digits (`0x2b`-`0x32`); *n* from `0x50`: a window string |
| `13` *n* | `{MACRO:00:n-30}` | the name of character *n* − `'0'` (`TOGetSaveChrName`) |
| `14`, `15`, `18` | `{MACRO:…}` | expand to the code itself |
| `16`, `17` *n* | `{MACRO:…}` | item *n* − `'0'`'s text (`MsGetRomItem`) |
| `19`-`22` *n* | `{MACRO:06..0f:n-30}` | the text at `u16` slot window `+0x20` + 2(*n* − `'0'`) of macro dictionary section *code* − `0x13` (`g_MacroDic`, the English set when `MsGetSaveConfigEnglish`) |
| `23` *n* | `{KEY:n}` | the name of key item *n* − `'0'` (`MsImportantName`) |
| `24`-`2b` *x* | - | group 4; one argument |
| `2c`-`2f` *t* | - | characters (see "Byte grammar") |

Groups: 1 = `00`-`03`; 2 = `07`-`0b`, `0d`, `0e`, `10` (`TOSetDrawInfo`);
3 = `12`-`23` (expanded, recursively, by `TODrawChangeMacro2`); 4 = the
rest. Fahrenheit's `{MACRO:i:n}` writes the code as *i* = code − `0x13`.

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

- Codes `02`, `05`, `06`, `08`, `0c`, `0d`, `0f`, `11`, `24`-`2b`: no
  window code does anything with them. 16 events and a battle in English
  and Japanese use none of `0c`, `0f`, `11` (which the two skippers read
  differently), but `2a` with an argument 625 times.
