# Text encoding

All in-game text uses one byte-oriented encoding with per-language
character tables.

Codec: [`crates/ffx-text`](../../crates/ffx-text).

## Character tables

`Text_LoadCharset` (`0x00646920`) loads
`ffx_ps2/ffx/master/jppc/ffx_encoding/ffxsjistbl_<t>.bin`, with *t* = `jp`
for Japanese, `kr` for Korean, `ch` for Chinese and `us` for every western
language. The file is UTF-8 text listing the characters in code order:
character index *n* is the *n*th character (`ffxsjistbl_us.bin` starts
`0123456789 !”#$%&’()…`).

The decoders (`0x006460a0` for jp/kr/ch, `0x006461f0` for western
languages) hard-code byte offsets into the table, which only track UTF-8
character widths; they agree with "the *n*th character" everywhere checked.
Two display substitutions exist: in Japanese, codes `0x5c`–`0x5e` show
character 10; in Korean, character 10 is shown as an ASCII space.

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

## Open questions

- What each control code does (colors, inserted names, pauses, …).
