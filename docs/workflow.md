# Reverse engineering workflow

## Target

All addresses refer to one build of `FFX.exe`:

- sha256 `0537b2a1047f3266e73495cd4e35f63f0777f4231d417699f979954686da686d`
- PE32, x86, image base `0x400000`, built with Visual Studio 2012
  (`msvcr110.dll`)
- `.text` `0x401000`–`0xb0bc00`; the current dump (`functions.tsv`) lists
  55,871 functions

## Tools

| Tool | Use |
|---|---|
| [REA](https://github.com/morluto/rea) MCP + Ghidra 12.1.4 (Linux) | Interactive questions: decompile, xrefs, strings, callers |
| `tools/ghidra/dump.sh` | Bulk: decompiles every function into greppable text under `$FFX_WORK/decomp` |
| x64dbg (Windows, `x32`) | Stepping the running game |
| frida-server 17.23.2 (Windows) | Tracing the running game; REA 6.4.0 has no Frida tools yet, so use `uv run --with frida==17.23.2` |

## Symbols

`symbols/ffx.sym` is the only lasting record of recovered names. REA's Ghidra
sessions are ephemeral and forget renames when they close, so:

1. Add every name you recover to `symbols/ffx.sym`, with a comment giving the
   evidence.
2. At the start of an REA session, pass `tools/sym2json.py` output to
   `set_addresses_names`.
3. Rerun `tools/ghidra/dump.sh` now and then; it reapplies the map to the
   persistent project in `$FFX_WORK/ghidra` and regenerates the dump with the
   new names.

### Fahrenheit names

The [Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit) team let us
use their names and findings (permission from peppy, 2026-10-16); their code
stays theirs, we don't copy it. `tools/fh-import.py` merges into
`symbols/ffx.sym`:

- `~/ffx-work/ghidra-remote/export/leads.tsv`, built by `merge.py` there from
  their Ghidra server (read-only; their databases load FFX.exe at 0, so their
  address + `0x400000` is ours): names in their database of this build,
  names carried over from their older Steam build and from the Switch build's
  symbols through their version-tracking sessions;
- the names, addresses and calling conventions in their hand-checked FhCall
  tables (`src/core/call.cs`, `src/core/ffx/call.cs`, pinned commit in the
  script, copied to `~/ffx-work/fahrenheit/<commit>/`).

Their `A::B::c` becomes `A_B_c`, the convention their own FhCall tables use.
Where both projects had named a function, we switched to their name
(2026-10-16); our old name follows `was` in the comment. The script never
renames our lines: if they later name one of ours differently, their name is
appended to our comment as `fh: <name>`. Rerun the script after refreshing
either source. When naming something new, use their name if they have one,
so both projects mean the same function by the same word.

Naming from local data, without the server (`tools/naming/`; each writes a
proposals TSV that `apply.py` merges, skipping named addresses and taken
names):

```bash
tools/naming/strmatch.py --out /tmp/str.tsv     # sole user of a string held once by both builds
tools/naming/import_self.py --out /tmp/self.tsv # sole user of its own __FUNCTION__ string
tools/naming/bracket.py --out /tmp/br.tsv       # alphabetical layout vs the Switch file's names
tools/naming/bracket.py --holdout 0.1           # measure bracket's precision first
tools/naming/apply.py /tmp/str.tsv /tmp/self.tsv /tmp/br.tsv
```

`tools/cg-match.py --out /tmp/cg.tsv` reruns the call-graph match against the
current anchors; take leads with 3+ votes (about 99% on holdouts; 2-vote
leads are about 91%).

Branches that name things in the same range conflict in `ffx.sym`; during
the merge, `tools/sym-resolve.py` merges it per symbol against the common
ancestor (a line only one side changed takes that side; both changed: ours,
reported) and reports a name given to two addresses.

## Verifying a merge batch

`tools/verify-batch.py [BASE]` (BASE: the last pushed state, `origin/main`)
runs only what the batch can affect: unit tests of the changed crates and of
every crate depending on them; the `ffx-oracle` tests of the changed crates,
of the changed test files and of the modules whose hook shims changed, on the
image alone and on both snapshots side by side (all of them when
`ffx-oracle/src` or `ffx-bridge` changed). When anything under `crates/`
changed it then runs the static port checks (check-ported, check-clobber,
check-callwidth, check-thisarg, check-argslots, check-callconv,
check-tailargs; below) and
`tools/build-hook.sh`, and when `crates/`, `tools/replays/` or the
comparison scripts changed, `tools/replays/run-all.sh` (`--no-replays`
skips it). It does not run the scene gates or the audio comparison: run
those yourself when the batch can affect them. check-clobber and check-ported share one disassembly cache
(`tools/disasm_cache.py`, `~/ffx-work/cache/clobber-<key>.pickle`, keyed on
FFX.exe's hash and the objdump version): check-clobber takes about 6 s warm
and 30 s when FFX.exe or objdump changed, check-ported about 4 s warm and
15 s cold (it took 2-3 min with one objdump per shim). Every fourth run, or with `--full`, the oracle tests of every
dependent crate run too. The game-data tests that read the whole archive run
separately, now and then.

Before building, it fails when a function is hooked twice: two `ported!`
entries for one address (two branches porting the same function, now
merged), or a `ported!` entry for an address hooked by hand in
`crates/ffx-hook/src`. Keep one port and drop the other.

### The port checks

Each is a static scan of `FFX.exe` against the shims and bridges, so it
also covers paths no test reaches. Run them with
`FFX_GAME_DIR=~/steam-games/FFX-X2-HD python3 -I tools/<check>.py`.

| Check | Finds | Where |
|---|---|---|
| `check-ported` | a shim that can panic, a `static` scratch buffer, an `in(reg)` operand after a `call`, an address that is no function start, under five bytes, hooked twice or named otherwise than in `ffx.sym`, a shim that pops other than the original's `ret` | `docs/porting.md`, "Install it" |
| `check-clobber` | callers that read the `eax` (or `x87` result) the original leaves and a shim drops; void shims of functions called through pointers | `docs/porting.md`, "Install it" |
| `check-callwidth` | calls wider than the game's host passes | `docs/hooking.md`, "Calling originals from a port" |
| `check-thisarg` | port calls to originals that read `ecx` before writing it, passing no `ecx` | `docs/porting.md`, "Library code" |
| `check-callconv` | port calls against the callee's `ret n` or the import's convention; test stubs whose `pops` differ from the original's | `docs/porting.md`, "Library code" |
| `check-tailargs` | a shim declaring fewer stack words than the original reads (forwarded words and tail jumps included) | `docs/porting.md`, "Install it" |
| `check-argslots` | unhooked callers that read an argument slot the original stores into and the shim does not | below |

check-ported, check-callwidth, check-thisarg, check-callconv and
check-tailargs report "0 problems" when clean; check-clobber and
check-argslots exit 0, and check-clobber reports no "reads the eax the
original returns".

### Test stubs

An oracle test replaces the callees a port calls through the game with
stubs. A stub must behave like the callee where the port can tell:

- **It pops like the original** (`pops`, `docs/porting.md`, "Calling
  convention"), or a port calling a stdcall callee as cdecl passes.
- **It logs every argument word the original passes**, the forwarded ones
  too (`MsMusicReadStart`'s victory fanfare id, check-tailargs), and `ecx`
  for library callees (`this = true`).
- **It writes what the callee writes.** When a port passes a pointer to its
  own buffer, the stub writes the full size the original callee writes,
  and the buffer is sized from the callee's code, not from Ghidra's type.
  `Sg_DrawSetEnvironmentInit`'s port gave `yiGetBackColor` a 3-word buffer;
  the callee writes 4 (RGBA), and the game crashed at boot. The stub wrote
  nothing, so the parity test passed (fixed in `73c9d4de`).

### Gates beyond the tests (M4)

A platform port (M4, `docs/rewrite-plan.md`) passes the oracle test and the
checks above, and then the gates for what it can change:

- **Draw streams.** `tools/scene-gate-capture.sh <replay> <run>
  original|only-m4|all` captures a gate replay, `tools/scene-gate.sh
  <scene> <reference run> <run>` compares it with the original's
  (`docs/drawdiff.md`, "Scene gates"). Six scenes: draw-field, bsil0000,
  bsil0300, kami0100, menu and battle. A round that touches rendering
  gates at least one scene besides draw-field.
- **Replays.** `tools/replays/run-all.sh`, and `--extra` for the graphics
  settings, a movie, the Iggy screens and a summon (`docs/harness.md`,
  "Coverage replays").
- **Audio.** `tools/replays/run-all.sh --audio` compares FMOD's mix sample
  for sample on four replays (`docs/harness.md`, "Audio capture").

The M4 port lists live outside the repo, in `~/ffx-work/m4/port/`: one
address per line in `<list>.list` (`h1-render-a`, `p6-render-c`, ...,
`coverage2`), cut from the pilot's coverage run; `tools/coverage-new.py`
finds reached platform functions that no list or `ported!` line holds yet.
Each list's shims go into `crates/ffx-hook/src/ported/<list>*.rs`, which is
what `scene-gate-capture.sh only-m4` installs. The functions that can't be
hooked are in `not-portable.tsv` there (`docs/hooking.md`, "What can't be
hooked").

### Argument slots

`FFX_GAME_DIR=... python3 -I tools/check-argslots.py [--module M] [--all] [NAME...]`
sits next to check-clobber and uses the same disassembly cache (about 5 s).
It catches a second kind of thing a caller can depend on. cdecl leaves the
argument words to the caller, and MSVC reuses them as scratch: `op_random`
(0x7e6700) stores its result into its own argument slot (`fstp [ebp+8]`),
and `opp_md_break_pieces` and `opus_mdc_break_make` then pass that word on
as a stale argument word of `op_random_per` / `op_irandom_p`. The thunk
gives a shim a copy of the arguments, so behind a plain shim the caller
finds the operand there instead of the result.

The check scans every hooked function's original for stores to `[ebp+8+k]`
(ebp frame) or `[esp+4+k]` at the entry depth (aligned `and esp` frames and
`mov ebx,esp` bases included), and for `lea` of such a slot. It then follows
each call site from `call_sites.tsv` from the return address, on both sides
of every branch and through jump tables, for up to 200 instructions or a
`ret`/tail jump, tracking the caller's stack depth and any copies of `esp`
in other registers. It reports a call site when the slot word is read, has
its address taken, or is passed as an argument word to a later call whose
callee reads that word (measured from the callee's code, tail jumps
followed), before the caller overwrites or pops it. A shim reproduces the
stores when it takes the caller's argument area: `fn shim(x: f32; slots: u32)`
in `ported!` makes the thunk pass the address of the caller's first
argument as an extra last stack word (cdecl and stdcall only), and the
shim writes through it (`rand::bridge::op_random_in_slot`). To test such a
shim, run a real original caller in both oracles and replace the callee in
`b` by the bridge with `Oracle::stand_in` (vfx_hi.rs
`op_random_shim_through_a_real_caller`).

It exits 1 when an unhooked caller reads a slot whose shim does not
reproduce the store. Callers that are hooked themselves never run their
original code, so they are listed as notes (`[caller hooked]`). `--all`
also lists the stores that no caller reads (about 1,200 hooked functions
store into their argument slots). Limits: calls through function pointers
are not in `call_sites.tsv`; indirect calls and imports are taken to pop
nothing, so `[esp+X]` reads after a stdcall API or vtable call may be
misplaced (`[ebp+D]` reads are exact).

## Build and test timing

Builds and tests log their wall time, so a round can show how much of an
agent's time went to waiting on the machine. Each run appends one line to
`~/ffx-work/timing/<worktree>.tsv` (tab-separated): start time (ISO), wall
seconds, exit code, tool, the command line, the 1-minute load average at
start and at end, the worktree, and the parent tool (`-` for a top-level
run, else e.g. `mutants.py`).

What logs:

- `cargo build/check/test/clippy/run/bench/doc/rustc` and `cargo xwin ...`,
  through the shim `~/.local/bin/cargo` (`tools/timing/cargo-shim.c`;
  `~/.local/bin` comes before `~/.cargo/bin` in PATH). It logs only inside an
  ffx-decomp checkout or worktree (the cwd, `-C` or `--manifest-path` has
  `symbols/ffx.sym` and `tools/mutants.py` above it) and only those
  subcommands. Everything else, and anything outside ffx-decomp, is `exec`ed
  straight to `~/.cargo/bin/cargo`. Inside, it runs cargo as a child with the
  same stdio, forwards SIGINT/SIGTERM/SIGHUP/SIGQUIT, returns its exit code
  (or dies of its signal), and costs under a millisecond. When it can't log,
  the command still runs. The cargo inside `cargo xwin` isn't logged twice.
- `tools/mutants.py`, `check-clobber.py`, `check-ported.py` and
  `verify-batch.py`, through `tools/timing_log.py` (one line at exit, nothing
  printed); `tools/replays/run-all.sh`, through `tools/timing/timing.sh`.
  Their cargo runs carry them as parent, and mutants.py's builds in
  `~/ffx-work/mutants/<wt>/tree` log under the worktree.

```bash
tools/timing-report.py all --since 6h      # per worktree and kind: total, count, mean, max,
tools/timing-report.py agent-a94...        # time at load > nproc vs not, the 10 slowest
```

The last line, `waiting on builds/tests: N min`, counts top-level runs only
(a nested cargo is inside its parent's time); the coordinator appends it to
`~/ffx-work/agents/metrics.tsv` each round.

Switch off: `FFX_TIMING=0` (per command or exported) makes the shim exec
cargo at once and the tools skip the log. Uninstall the shim with
`tools/timing/install.sh --remove` (removes `~/.local/bin/cargo`); install
or rebuild it with `tools/timing/install.sh`, which runs
`tools/timing/test-shim.sh` (pass-through tests against a fake cargo) first.
`FFX_TIMING_DIR` moves the log, `FFX_TIMING_REAL_CARGO` points the shim at
another cargo. sccache stays the `rustc-wrapper`; there is no per-crate log.

## Oracle test speed

Per-test times: `RUSTC_BOOTSTRAP=1 <test binary> -Z unstable-options
--report-time` (the binary from `cargo test --release -p ffx-oracle --test
<file> --no-run`). Two kinds of waste made the slowest tests slow
(2026-10-21):

- Diffs re-reading old pages. `Oracle::diff` compares every page either
  side wrote since `clear_dirty`, so a long test whose helper never clears
  rereads every page each earlier case wrote, at every case
  (`main::control_synthetic` 198 s, now 3 s). Once a diff passed, clear both
  sides' dirty pages (`settle` in main.rs, script.rs, system.rs;
  `battle_common::same`; the helpers of vfx.rs and menu.rs did so already):
  a page neither side writes again stays equal, and a write marks it again.
  Write new compare helpers the same way.
- Originals that spin. A call that waits on a thread the emulator does not
  run (a file read, a semaphore) loops until the instruction budget runs
  out: 200M instructions, 5-20 s per case. `Oracle::set_max_instructions`
  lowers it per oracle; script.rs's `run_pair` uses 20M (`RUN_BUDGET`). Only
  lower it where the whole file still stops exactly the same cases.

script.rs's tests that loop over the ctrl works in use run in three shards
(`natives_match_the_original::<group>::shard0..2`,
`round19_natives_match_the_original::shard0..2`, `CTRL_SHARDS`): shard `k`
takes ctrl works `k`, `k + 3`, .. and skips the others' random draws, so
every case gets the same arguments and state as in one run. The battle
snapshot has three ctrl works, the field snapshot one (shards 1 and 2 pass
at once there). The old names still select all shards as filters.

## Claims and work packets

Agents pick work from one global list and claim each function first, so two
never port the same one:

```bash
tools/worklist.py --global --unclaimed --limit 20   # leaves first, every game module
tools/claim.py 0x007ab910 0x007abeb0 --agent battle-r40   # all or nothing
tools/claim.py --list                                # stale claims (>12 h) marked
tools/claim.py --release --all --agent battle-r40    # on commit
tools/packet.py 0x007ab910 --out ~/ffx-work/packets/0x007ab910.md
tools/packet.py --batch "--global --unclaimed --limit 10" --dir ~/ffx-work/packets/r40
```

A claim is a file in `~/ffx-work/claims` (`$FFX_CLAIMS`) made with
`O_EXCL`; `--force` takes over a stale one. The port still goes into the
function's module crate.

A packet (`tools/packet.py`, needs `FFX_GAME_DIR`) is one markdown file with
what porting the function needs: module, category, callers and claim; the
measured convention; every callee with its convention, stack words, x87
result and ported state (the Rust path behind its shim) or "call through the
game"; the globals and labels it touches (relocations, `data_refs.tsv`,
`ffx.sym` comments, `.rdata` constants decoded); a trap scan (stack slots
read on a path that never writes them, x87 use, calls with fewer words set
up than the callee reads, eax returned where Ghidra says void, pushed
image-range constants without a relocation); Ghidra's C and the annotated
asm; for x87-heavy code a `tools/x87-lift.py` draft (reference only,
rewrite by hand); and a test skeleton for the module's oracle file. The
machine-code facts come from linear scans and are heuristics: check them
against the asm.

## Mutation testing

`tools/mutants.py` is the mutation runner. It takes the usual mutant list
(a Python or JSON list of `(name, old, new)`, or `[old, new]` pairs named
`#1`, `#2`, ...) and the files the mutants live in:

```bash
MUT_TEST=battle_cue MUT_FILTER=cues MUT_SNAPS=battle,field \
  python3 tools/mutants.py <worktree> muts.py crates/ffx-battle/src/cue.rs
```

It never edits the worktree: it syncs it into a private copy
(`~/ffx-work/mutants/<worktree name>`, or `MUT_WORK`) and rewrites every
mutation site there into a schema,
`if ffx_core::mutant(k) { new } else { old }` (in parentheses for an
expression, in braces for a match arm's body), around the smallest
expression or statement run holding the change (`tools/mutants-rs` finds
those with `syn`, and keeps only nodes whose function still parses with the
schema in place and that are not in a const context; a `let` change keeps
the rest of its block inside the schema). The `ffx-oracle` test binary is
built once with `--features ffx-core/mutants`, and each mutant runs that
binary with `FFX_MUTANT=k` on the image and each snapshot, `MUT_JOBS`
(default cores / 2) at a time. A schema with a compiler error inside it
moves to a larger node; one that still fails falls back to a rebuild for
that mutant alone, so none is lost. Errors whose spans point into no schema
(a type that leaks out of one, say) are traced by building halves of the
schema set, and only the schemata behind them are dropped to the rebuild
path; the rest still build as one. A mutant that does not build on its own
is an ERROR, never a kill. Without the feature `ffx_core::mutant` is a
constant `false`; release builds never enable it.
`python3 tools/mutants.py --self-test` checks the rewrite with rustc alone
(match arms with and without braces, overlapping deletions, a tail
insertion, a const): one build, and every schema behaves as its mutant
rebuilt. Run it after changing `tools/mutants.py` or `tools/mutants-rs`.

The schema wraps the smallest node, so an operator mutant keeps the
original grouping: `a & b` -> `a | b` inside `a & b & c` runs as
`(a | b) & c`, while the same text rebuilt would read `a | (b & c)`. The
grouped one can be equivalent only because of the grouping. Add the
top-level variant by hand, with explicit parentheses so the change spans
the whole expression: old `a & b & c`, new `a | (b & c)`.

Verdicts are `KILLED`, `SURVIVED`, `ERROR` or `NOMATCH`, a tab and the name,
in list order. A failing test or a timeout (`MUT_TIMEOUT`, default 20x the
unmutated run, at least 120 s) kills; a run without "test result" or with
fewer passes than `MUT_PASSED` (default: what the unmutated run passes) is an
ERROR, and an unmutated run that fails stops everything. `old` must match
once in one file, exactly or with any whitespace between tokens. `--dry-run`
prints the schema diff; `MUT_SCHEMA=0` rebuilds every mutant (the old way).
Validation (2026-10-20): on battle r27 g6, effects r31 a and system r38 a
(102 mutants) it gave exactly the old `mut22.py` verdicts, in 146 s instead
of 2417 s (4-29x per group); 1 mutant needed a rebuild. Most of what's left
is the one build. After the match-arm and bisection fixes (2026-10-20) the
same sets give the same verdicts in 161 s, and menu round 24 group 1, whose
one unbuildable schema used to send all 46 mutants to rebuilds (483 s), runs
in 37 s with none rebuilt.

## Progress and ownership

```bash
tools/progress.py             # named and replaced, per module (game and library)
tools/progress.py --unknown   # named functions no module rule places
tools/ownership.py summary    # globals per module: its own, and shared with whom
tools/ownership.py shared 40  # the globals the most modules touch
tools/ownership.py function MsCalcDamageCommand
tools/ownership.py global 0x00c8f508
tools/status.sh               # the status page, regenerated in the background
```

The status page (`$FFX_WORK/status/index.html`) shows the progress.py
numbers for `main`, read through git, so every worktree writes the same page:
the binary as a Hilbert-curve map (each function in address order, coloured
replaced / logic to port / the rest), the modules, the ports per module per
time bucket, the share replaced at every commit, the logic left to port and
the latest ports. `tools/status.sh` returns at once and runs
`tools/status.py` behind a lock; calls during a run fold into one more run.
`tools/verify-batch.py` calls it when a batch passes. The first run replays
the whole history (about a minute); later ones reuse `history.json` and take
under a second. The open page reloads `data.js` every 30 s.

The page opens with "Current work": the phase, the current round's tasks
(owner branch, state, one-line result) and the past rounds, from
`~/ffx-work/agents/tasks.json`. The coordinator keeps that file by hand
(format at the top of `tools/status.py`): update it when a round starts,
a task changes state (running, merged, verified, pushed) or a round ends,
then run `tools/status.sh`. Below it: in-game coverage (the ported hooks
that ran, from `symbols/hook-hits.txt`, per module and over time), "left to
port" split by why (to port, held unhooked on purpose, held for M5, too
short to hook; a new `symbols/classes.tsv` category shows up as its own
group) and the replays `tools/replays/run-all.sh` plays.

Both progress.py and ownership.py read the dump below (`functions.tsv`, `data_refs.tsv`). Modules come
from name prefixes (the original PS2 ones the Switch symbols carry, listed in
`tools/progress.py`); unnamed functions take their named neighbours' module,
so module sizes are estimates until most of the code is named. Ownership only
sees absolute references into `.data`: state reached through pointers is
invisible to it.

## Duplicate functions

Check `tools/dupes.py` before porting: a copy of a ported function only
needs a `ported!` line and a test against its own original.

```bash
FFX_GAME_DIR=~/steam-games/FFX-X2-HD tools/dupes.py --shims ~/ffx-work/speedup/dupes/shims.txt
tools/dupes.py --family 0x0074bff0   # the families of one address
```

Families: `exact` (same bytes once calls and jumps out of the function are
replaced by their resolved target), `masked` (same once the operands
`.reloc` patches are masked too: other globals) and `near` (>= 95 % of the
instructions alike with numbers masked and registers renamed: a lead, read
the difference). `families.tsv` lists every member with module, class and
port status; `--shims` prints the `ported!` entries for unported logic
copies of ported functions, copied from the port, and the oracle test to
extend. About 30 s.

## References and dead verdicts

The dump's `calls.tsv` and `call_sites.tsv` only list calls from code
Ghidra made a function of. Code it left as a label calls functions too
(`0x00a58ff0` from `0x00a44ef5`, `0x00a45012` and `0x00a452d5`;
`0x0084d790` from `0x008532bc`), so "0 callers" there proves nothing.
`tools/refs.py` scans FFX.exe itself:

```bash
FFX_GAME_DIR=~/steam-games/FFX-X2-HD tools/refs.py 0x00a58ff0 AbmapDrawPanels_00a58660
```

It lists every E8/E9/0F 8x rel32 branch to the address in `.text` (each
checked by disassembling around it: "raw (decoded)", or "ghidra" when
call_sites.tsv has it too; "(raw, not decoded)" is byte noise) and every
relocated pointer to it: `data` (vtables, native tables, callbacks in
`.data`/`.rdata`) and `code` (an immediate such as `push offset f`). The
image has a base relocation table, so the pointer list is exact. The index
is cached in `$FFX_WORK/cache/rawrefs-<hash>-v<n>.pickle` (about 1 s to
build, 0.1 s to load). `tools/worklist.py` and `tools/packet.py` count these
raw callers and tail-jump callees together with call_sites.tsv.

A dead verdict (an OVERRIDES entry in `tools/classify.py`, or "no callers"
in a report) must cite `tools/refs.py`: "tools/refs.py: 0 refs", or the
refs it lists and why each one never runs. A function stored in a vtable
(`data` ref) is reached through it even with no direct caller.

## Bulk dump

```bash
export GHIDRA_INSTALL_DIR=~/tools/ghidra_12.1.4_PUBLIC
tools/ghidra/dump.sh          # first run imports and analyzes FFX.exe
```

Output in `$FFX_WORK/decomp` (default `~/ffx-work/decomp`):

- `code/0xNNNN0000.c`: pseudocode, one file per 64 KiB of address space
- `functions.tsv`: address, name, size, caller count
- `calls.tsv`: call graph edges
- `strings.tsv`: every defined string and the functions that use it

This is generated from the game binary, so it stays outside the repository.

## Clean-room rule

Rust code is written from what `FFX.exe` and the data files show. Fahrenheit
and VBFExtract may be consulted for facts (formats, addresses, meanings) to
check findings, but their code is never copied.
