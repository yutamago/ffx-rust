# Scripts

Events, NPCs, story progression and monster AI all run on one bytecode
interpreter, `Script_Run` (`0x008641e0`). Scripts live in:

- event files, `ffx_ps2/ffx/master/jppc/event/**/*.ebp` (397), section 0;
- monster files, `ffx_ps2/ffx/master/jppc/battle/mon/_mNNN/mNNN.bin` (361),
  section 0 (registered by `Mon_LoadAll` through `0x00797a40`).

Disassembler/assembler: [`crates/ffx-script`](../../crates/ffx-script).

## Instructions

`Script_Fetch` (`0x00869d60`): a byte below `0x80` is an opcode alone; a
byte with the high bit set is opcode `byte & 0x7f` followed by a `u16`
operand. The program counter advances by `((byte | 0x40) >> 6)`, 1 or 3.
Opcodes reach their cases through the byte table at `0x0086536c`; Ghidra
resolves it, so its `case` labels are opcodes.

The machine is a stack machine with registers A and B, a float
accumulator, four integer and ten float registers, and per-worker
variables. The full table with the evidence for each opcode is
[`crates/ffx-script/src/opcode.rs`](../../crates/ffx-script/src/opcode.rs).
In summary:

| Opcodes | Meaning |
|---|---|
| `00`, `1d`, `1e`, `76` | no-op |
| `01`–`05` | logical or/and; bitwise or/xor/and |
| `06`–`0f` | comparisons: eq, ne, gt/ge/lt/le, signed and unsigned |
| `10`–`13` | bit test, inverted bit test, shift left, arithmetic shift right |
| `14`–`18`, `1a` | add, sub, mul, div, mod (int or float), float negate |
| `19`, `1c` | logical not, bitwise not |
| `1f`–`24`, `27` | variables: push, pop (two store modes), array element push/pop, element address |
| `25`, `26` | float accumulator pop/push |
| `28`, `29`, `2a`, `2c` | registers A, B push/pop |
| `2d`, `2e`, `2f` | push integer constant, 16-bit immediate, float constant |
| `30`, `31`, `32` | jump to label: always, if A ≠ 0, if A = 0 (`55`–`57`: pop into A first) |
| `33`, `34` | call another worker's first entry (3 deep), return |
| `35`, `58` | call a native function (`58` then pops the result into the float accumulator) |
| `36`–`38`, `45`–`53` | requests to other workers (three families of six) |
| `3c`–`3f`, `40`, `54` | end the entry (four variants), wait, halt |
| `59`–`74` | integer and float register pop/push |
| `75` | push global constant (table `0x00c52704`) |
| `1b`, `41`–`44`, `7b`–`7f` | invalid: stop with an error; never used |

### Comparisons

With `a` pushed first and `b` on top:

| | `==` | `!=` | `>` | `>=` | `<` | `<=` |
|---|---|---|---|---|---|---|
| signed | `06` | `07` | `0a` | `0b` | `0e` | `0f` |
| unsigned | | | `08` | `09` | `0c` | `0d` |

Floats use the same opcodes.

## Native functions

`func id` splits the ID into a group (`id >> 12`) and an index; groups
0, 1, 4–9, b, c, d have tables registered by `Script_InitNatives`
(`0x0086d6d0`). Each 16-byte entry holds four handlers: start (called once),
poll (called until done, so a call can span frames), float result and
integer result. The shipped scripts call 948 distinct native functions,
2.3 million times. `ffx script disasm` annotates each call with its handler's
address (and name, when `symbols/ffx.sym` has one); for example
`0x70ab` → `0x007a5360`, which sets a battle actor property
(`0x007b4b70`).

## Script block

All offsets are relative to the block start (`B`). Code offsets in tables
are relative to the code start.

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u32` | Code length |
| `0x10` | `u32` | Block length |
| `0x30` | `u32` | Code start; the only header field `Script_Run` reads |
| `0x34` | `u16` | Worker count |
| `0x38` | `u32` × *n* | Worker header offsets |

Other header words (`0x04`–`0x2c`) point to tables before and after the
code and are kept as they are.

Worker header (`0x34` bytes):

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u16` | Worker type; `Script_Run` treats types 5 and 6 specially |
| `0x02` | `u16` | Variable count; table at `0x14`, 8 bytes each (`u32` type in the top nibble and location, `u16` array length) |
| `0x04` | `u16` | Integer constant count; table at `0x18` |
| `0x06` | `u16` | Float constant count; table at `0x1c`, ending at the code start |
| `0x08` | `u16` | Entry point count; table at `0x20` |
| `0x0a` | `u16` | Jump label count; table at `0x24` (`Script_Jump`, `0x008726f0`) |
| `0x30` | `u32` | End of the code, rounded up to 4 |

Every worker's entries and labels lie inside its own contiguous code range,
and the ranges follow worker order. This holds for all 758 shipped scripts,
which is what lets the disassembler name jump targets per worker.

## Assembly

```bash
ffx script list znkd                     # files with scripts, filtered
ffx script disasm m001 -o raldo.asm      # a monster's AI
ffx script disasm ffx_ps2/ffx/master/jppc/event/obj/zn/znkd0900/znkd0900.ebp
ffx script asm m001 raldo.asm --mod-dir mods/my-mod
```

```text
worker 1 type 2
e0:
    pushimm 158
    pushimm 16650
    func_facc 0x70ab         ; 0x007a5360
```

`eN:` and `lN:` place entry points and jump labels; jumps name labels
(`jmp l3`). Assembly applies onto the original script: constants,
variables and data stay as they are, and the original entry and label
counts must be kept. Code that fits in a worker's original range is written
there and padded with `nop`; longer code is appended to the end of the
block, and the containing file's later sections shift by a multiple of
`0x40`. That works because `Script_Run` only resolves code through the code
start at `+0x30`; nothing else in the block moves.

## Verification

`cargo test -p ffx-script` (with `FFX_GAME_DIR`) disassembles and
reassembles all 758 scripts, 17.3 million instructions, and checks that
script blocks and their files come back byte for byte. A second test grows
a worker of `m001` past its range and checks the relocated code, the
shifted sections and that the result round-trips again.

## Open questions

- The meaning of most native functions.
- The three worker request families (`0x00867230`, `0x00867570`,
  `0x008673d0`) and opcodes `39`–`3b`, `40`, `54`, `77`–`7a`.
- The block header words `0x04`–`0x2c` and the tables they point to.
