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
Our names are never replaced; their name for the same address is appended to
our comment as `fh: <name>`. Rerun the script after refreshing either source.
When naming something new, use their name if they have one, so both projects
mean the same function by the same word.

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
