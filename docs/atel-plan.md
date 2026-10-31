# Atel language plan

Goal: turn every compiled ATEL script of FFX (events, monster AI, battle
scripts) back into source code that reads like what the developers meant,
in a new C#-like language called **Atel**, and compile that source back into
the same bytes. Atel then becomes the language mods are written in.

This replaces the Lua plan of the same day and stage L2 ("Atel to Luau") in
`docs/rewrite-plan.md`, "Scripting for modders (Luau)"; that section is to
be updated once this plan is accepted.

## Decisions (user, 2026-10-31)

- **A custom language, close to C#.** Most modders, including the Fahrenheit
  community, know C#. Custom syntax is fine where ATEL needs it, but the
  language should feel like C#. Files end in `.atel`.
- **Byte-identical round trip** is the acceptance bar: decompile, compile,
  and get the original script block and containing file back, for every
  script.
- **The decompiled source lives in a private GitHub repo,
  `yutamago/ffx-atel-src`**, versioned. Claude may create it when the time
  comes. The tools (decompiler, compiler, libraries) live in `ffx-rust`; no
  decompiled output goes into `ffx-rust`.
- **Mods change things, they don't replace files.** Vanilla files stay
  untouched; mods are composed in load order (as the size tables already
  are, `docs/hooking.md`).
- Work starts when the user says so.

## What the bytecode says about the original language

From all 758 scripts (17.3 million instructions; survey 2026-10-31):

- C expressions and types: `+ - * / %`, comparisons, `&& ||` (not
  short-circuit), `& | !`, `<<`; separate signed and unsigned compares;
  variables typed `u8`…`f32` in storage areas.
- Arrays with nested indexing (742k element reads), `&var` passed to
  natives (2k).
- `if` (every conditional forward jump, 405k), loops (64k backward jumps),
  `switch` (selector in register B, compare chain after the case bodies,
  598k backward case jumps; every chain ends in a `jmp`, so `default`
  likely exists).
- Locals in 4 int and 10 float registers per worker; shared subroutines
  (`call`, argument in `i0`); `return value`.
- Workers with numbered entries; requests between workers with 9 priority
  levels (`req` 79k, `reqe` 25k, `reqw` 2.9k).
- Constant folding, small immediates, a breakpoint opcode (a debugger).
- No strings (messages are IDs), no pointers beyond `&var`, no structs.

## The language

### Files and order

- Every file starts with the script it belongs to, like a C# file-scoped
  namespace: `event Bjyt0400;`, `monster M001;`, `battle Cdsp00_00;`.
- A script is a folder, one file per worker, plus a main file:
  `event/bj/bjyt0400/Bjyt0400.atel`, `Wakka.atel`, `Lulu.atel`.
- The main file declares the worker order once, `workers { Director,
  Wakka, Lulu }`; a worker's position is its number.
- File-level variables are shared by all workers of the script.
- `partial class` splits a big worker over several files.
- `using Ffx.Battle;` imports the generated libraries.

### Workers and entries

- A worker is a `class`. Its base class says what it is (`Npc`, `Monster`,
  …; one per worker type, see "Worker types"); function workers are
  `static class`es.
- Entries the engine calls by number are base class members and use
  `override`: `Init` (entry 0, run at priority 7), `Main` (entry 1,
  priority 0), talk, touch, battle turns, … Other entries are `async`;
  plain methods are helpers (see "Three kinds of code").
- Entry numbers are implicit (base class first, then files in path order,
  then declaration order). `[Entry(n)]` only where a specific number is
  wanted; the decompiler writes it only where the rule would differ.
- A base class can provide default entries (a monster without `Main`
  waits for ever), emitted only when the bytes match.

### Worker types

Seven types (worker header `+0x00`), in the 758 scripts:

| Type | Workers | Known |
|---|---|---|
| 0 | 441 (events) | **function worker**: every `call` (5,560) targets one, and each has exactly one entry; becomes a `static class` |
| 1 | 6,707 (events) | |
| 2 | 648 (events), 898 (monsters: every monster worker) | |
| 3 | 486 (events) | |
| 4 | 1,944 (events) | no position block |
| 5 | 1,382 (events) | position block and walk-line table (characters) |
| 6 | 116 (events) | as 5 |

Phase 0 names the rest from the natives each type calls; the `battle/btl`
scripts are counted then too.

### Three kinds of code

ATEL entries are not functions: each runs on its own worker when that
worker gets to it. The language keeps the original's three kinds apart:

| Kind | Declared as | Runs |
|---|---|---|
| Function worker (type 0) | `static class Common { static void SetupModels(int mode) { … } }` | synchronously on the caller's worker (`call`) |
| Helper method (new code) | a plain method in a class: `void FaceThePlayer() { … }` | synchronously; the compiler makes it a hidden function worker and `call`s it |
| Entry | `override` (engine-run) or `async Task NameAsync()` (requestable) | on its own worker, through a request |

`call` limits apply to static functions and helpers: at most 3 deep, no
recursion (compile error), no `await` on them.

### Parameters and return values

ATEL passes neither: `call` shares the caller's registers, and a request
carries only entry, worker and priority. The compiler provides both:

- **Functions and helpers:** parameters in registers (up to 4 `int` and 10
  `float`, minus those the caller still needs, which the compiler saves),
  the return value in `i0` or `f0`. Synchronous, so always safe.
- **Entries:** parameters and the return value go through hidden fields of
  the receiving worker, written at the request and read after
  `.Finished`. Safe while one request at a time is pending for that entry.
  The compiler sees every caller (requests stay within one script, and
  mods are composed before compiling), so it warns when two callers could
  have the same entry pending at once. A return value requires waiting
  for `.Finished`.
- Decompiled code has neither (the original had none); this is for new
  code only.

### Requests between workers

- An entry other workers may run is declared `async Task NameAsync()`
  (`async Task<int> NameAsync(int x)` with a parameter and a result).
- `_ = Lulu.LookAtWakkaAsync();` queues it and carries on (`req`).
- `await Lulu.LookAtWakkaAsync();` or `await Lulu.LookAtWakkaAsync().Finished;`
  waits until it ends (`reqe`).
- `await Lulu.LookAtWakkaAsync().Started;` waits until it begins (`reqw`).
- A request without `await` or `_ =` gets a hint. Requesting an entry that
  is not `async` is an error.
- A worker can request its own entries (641 `req`, 130 `reqe` in the
  shipped scripts). Awaiting one works only at a priority above the one
  the worker runs at: it interrupts, runs, and the caller continues. At the
  same or a lower priority it cannot start while the caller waits (to
  confirm in Phase 0); the compiler reports that as an error where it
  knows the caller's priority (`Main` P0, `Init` P7) and warns elsewhere.
  A helper method is usually the better choice.
- Priorities: `Priority.P0_Main` … `Priority.P8_…`, zero-based, sorted by
  level, names after the number from Phase 0. P1–P6 are for scripts; P0
  (`Main`'s level), P7 (`Init`, battle system) and P8 (unused) compile with
  a warning. A common default makes most calls omit it.
- `await` on something that cannot be awaited (a native that waits anyway,
  a function or helper) is dropped, with a hint.

### Variables

- Fields live in the script's data (storage 2, 3, 4), with initial values;
  file-level variables in the shared area (storage 6).
- `[Register] int Count;` / `[Register(2)] int Count;` puts a field in one
  of the worker's registers; the compiler assigns slots in declaration
  order unless pinned. Function and helper parameters also use registers
  (see "Parameters and return values").
- Locals survive waits, as in C#: where a local is still needed after a
  wait, the compiler saves and restores it through hidden storage.
  Decompiled locals that do not cross a wait stay plain locals; registers
  the original kept across a wait become `[Register]` fields.
- Register fields used across a wait while another entry of the same
  worker writes them give warning ATL0001, suppressible
  (`#pragma warning disable ATL0001`).
- The save game is the predefined `Save` library (`Save.StoryProgress`),
  from `docs/formats/save.md` and Fahrenheit's names; unnamed offsets are
  `Save.Unknown_0123`.

### Readability

- Natives are grouped static classes and methods (`Msg.Say`, `Cam.Move`,
  `Chr.PlayMotion`); on the worker itself without a prefix (`PlayMotion`).
- Actor properties are properties: `Radius = 35.0;`, `if (Hp < 100)`,
  `Party.Alive.Defense = 0;`. `Position = (x, y, z);`. Named arguments
  where they help (`frames: 15`).
- Values are typed by use: `Item[] Rewards = { Item.Potion, … }`,
  `Motion.Idle`, `Command.Bite`, `Target.Party & Target.Alive`.
- Story progress: `Story.S1340_WakkaJoinsParty`, the value zero-padded in
  the name so alphabetical order is progress order; auto-named from the
  script that sets each value, renamed by hand.
- Shared behaviour: workers with identical code across scripts (2,238 of
  12,622 are exact copies of another) become library base classes
  (`class Chest3 : TreasureChest { }`); the compiler copies the code back.
  Near-duplicates may become base classes with parameters.

### Text

- Messages by ID: `Msg.Say(Text[12]);`. The language server shows the text
  (inlay hint, hover) in a chosen language; the formatter can keep an
  optional `// "…"` comment for reading outside an editor.
- New messages: `Text.New("Nice throw!")`. A script holds one index for
  all languages, so the compiler adds the message at the same index in
  every language's table: the mod's source language (`mod.toml`) gets the
  text, the others a copy marked untranslated. The formatter rewrites the
  call to `Text[173]`; translators work in the per-language TOML
  (`ffx text export/import`).
- Debug text is a real `string`, allowed only for `Debug.*`:
  `Debug.Print($"hp = {Hp}");`. It calls new natives in a group the game
  leaves empty (2, 3, a, e or f), provided by our hook or a Fahrenheit
  mod; without them the compiler refuses it or drops it on request.

### Control flow and semantics

- `if`/`else`, `while`, `do`, `switch` with `case`, `default`, `break`,
  `goto case`; `goto` for flow that has no structured form. Ternary and
  `for` if Phase 0 shows the original compiler produced them; the compiler
  accepts both for mods either way.
- ATEL semantics, documented: `&&` and `||` do not short-circuit; `int` is
  32-bit with wrap-around; `float` is `f32` (`35.0` is a float, `35.0f`
  also accepted); unsigned compares follow the operand types.
- No objects, strings (outside `Debug`), generics, exceptions or LINQ:
  "not available in Atel".
- Escape hatches for the round trip: `asm { … }` (the existing assembly
  syntax) for code the decompiler cannot express yet, and attributes that
  pin layout details (constant-pool order, a variable's storage area and
  offset, unknown header data).

## Compiler output

- One script file (`.ebp`, monster or battle file) with the rebuilt script
  block, through `Script::rebuild`.
- A package for runtime injection: the bytecode plus any message additions,
  so a debugger or mod can load it into a running game and extend the
  event's message table in memory (ctrl work `+0x34`).

## Mods

- The launcher builds on the player's machine: it decompiles the player's
  own vanilla scripts, applies the enabled mods in load order, compiles
  each affected script once and writes the results to a generated folder
  that the hook serves like any mod file. Vanilla files are never edited;
  disabling a mod is a rebuild without it. No mod contains game code.
- A mod changes an existing worker with `partial class Wakka { … }` and adds
  new workers, entries, fields and messages by name; every number is
  assigned at build time.
- Patches on entries, after Harmony and Mixins: `[Prefix(nameof(OnTalk))]`,
  `[Postfix(…)]`, `[Inject(…)]` at a point inside an entry, `[Replace(…)]`;
  ordering with `[Before("mod")]` / `[After("mod")]`.
- Data edits per field (kernel tables, messages), after RimWorld and the
  Bethesda merge tools.
- `mod.toml`: `source_language`, `after`, `requires`, `incompatible`.
- The launcher shows a conflict report before the game starts: what two
  mods both touch and who wins. Additions and prefix/postfix chains combine
  automatically; replacements and same-field edits are reported conflicts.
- Open: saved data for mods. The save has no known free space; storage 3
  can be redirected through the ctrl work's `+0x54` callback, which nothing
  uses, so the hook might give each mod its own saved variables in a side
  file. Needs research.
- File sizes are solved: the hook wraps `CDfileSize_CD` and `CDfileSize_PC`,
  and a mod file reports its own size; the vanilla tables are never shipped.
  To check: whether the PC build calls `CDfileSize_CD_EX` (not wrapped).
  Worth offering the wrapper to Fahrenheit.

## Phases

Each phase ends with a corpus test in `ffx-rust` (`FFX_GAME_DIR`, skipped
without it) and numbers: scripts that round-trip, instructions inside `asm`,
`goto` count, names placed.

0. **Measure the corpus** (2–3 days). Stack depth at labels and jumps; jump
   shapes; encoding choices (immediate vs pool, `func` vs `func_facc`,
   store modes, end variants); table and label order against code order;
   ternary and `for` patterns; `default` in switches; worker type meanings
   (types 1-6, and the `battle/btl` scripts); what an awaited request to
   the worker's own entry does at the same or a lower priority (oracle);
   fixed entry numbers per type; what each priority is used for; story
   values by the script that sets them; whether `UseCommand` checks the
   monster's command list; near-duplicate workers; whether anything stores
   entry numbers (saves); unreachable code; `CDfileSize_CD_EX` on PC.
1. **Native signatures and libraries** (3–5 days). Argument count, kinds,
   result and polling for every native, from the Rust ports and from stack
   balance, compared. Generated from it: the Atel library declarations
   (natives, enums, `Save`, `Story`, `Priority`, base classes).
2. **Skeleton round trip** (1 week). New crate `crates/ffx-atel`;
   `ffx script decompile` and `ffx script compile`. Declarations plus every
   entry as `asm { … }`; a hand-written parser; every script round-trips.
3. **Expressions and statements** (1 week). Stack code to expressions and
   calls, registers to locals and `[Register]` fields.
4. **Control flow** (1–2 weeks). `if`/`while`/`switch`/`goto case`, label
   order and branch polarity matching the original. Exit: no `asm` left.
5. **Typing and naming** (ongoing). Enums by use, properties, text IDs,
   story constants, base classes and engine entries, Fahrenheit's names.
6. **Source layout.** Folders, one file per worker, `partial`, `using`,
   library base classes for duplicated workers.
7. **The private repo.** `yutamago/ffx-atel-src`; a `generated` branch
   written only by the tool and a `main` branch for hand edits, merged with
   git; the round-trip check runs locally (it needs the game). Commits as
   Yutamago, no user paths.
8. **Tooling.** TextMate grammar; a language server (completion of natives
   and enums, hover, text inlays, warnings, saved-locals and register
   hints); the formatter (the decompiler's printer).
9. **Mod system.** `mod.toml`, patch attributes, the launcher's build step,
   per-field data edits, the conflict report.
10. **Debug text and runtime packages.** The `Debug.*` natives in a free
    group (hook and Fahrenheit), the injection package.
11. **Documentation.** A language reference, the encoding rules, a
    first-mod tutorial.

Phases 0–4: about 5–7 weeks to a fully structured, byte-identical decompile
of every script. 5–11 follow and partly run alongside.

## Out of scope for now

- Running something other than the ATEL VM (the old L2 idea of running
  Luau). Atel compiles to ATEL bytecode, so the existing Rust interpreter
  and the oracle's byte parity keep working.
