# Reverse engineering workflow

## Target

All addresses refer to one build of `FFX.exe`:

- sha256 `0537b2a1047f3266e73495cd4e35f63f0777f4231d417699f979954686da686d`
- PE32, x86, image base `0x400000`, built with Visual Studio 2012
  (`msvcr110.dll`)
- `.text` `0x401000`–`0xb0bc00`; Ghidra finds 56,903 functions

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
`ffx-oracle/src` or `ffx-bridge` changed); the port checks, the hook build
and the replays. Every fourth run, or with `--full`, the oracle tests of every
dependent crate run too. The game-data tests that read the whole archive run
separately, now and then.

Before building, it fails when a function is hooked twice: two `ported!`
entries for one address (two branches porting the same function, now
merged), or a `ported!` entry for an address hooked by hand in
`crates/ffx-hook/src`. Keep one port and drop the other.

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
```

Both read the dump below (`functions.tsv`, `data_refs.tsv`). Modules come
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
