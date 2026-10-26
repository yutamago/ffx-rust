#!/usr/bin/env python3
"""The hook map: every `ported!` entry with where its port lives.

One row per hook: the FFX.exe address and name, the calling convention,
the shim (file:line and function), the Rust function the shim calls (the
first `alias::path(...)` call in its body, with the alias resolved through
the file's `use ... as alias;` lines; for a shim that returns a constant,
the constant's path), and the ffx-oracle test files that name the address
as a literal (tests that reach it through a symbol constant are not found). Hand-written hooks in crates/ffx-hook/src/lib.rs are not
`ported!` entries and are not listed.

  tools/hook-map.py                 TSV on stdout
  tools/hook-map.py --find NAME|ADDR  only rows whose name or address matches
  tools/hook-map.py --html FILE     a searchable page
"""
import argparse
import html
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
PORTED = REPO / "crates/ffx-hook/src/ported"
TESTS = REPO / "crates/ffx-oracle/tests"

ENTRY = re.compile(
    r'^\s*([A-Za-z_][A-Za-z_0-9]*) @ (0x[0-9a-fA-F_]+) => extern "(\w+)" fn ([A-Za-z_][A-Za-z_0-9]*)\s*\(')
USE_AS = re.compile(r'^\s*use ([A-Za-z_][\w:]*) as ([A-Za-z_]\w*);')
USE_PLAIN = re.compile(r'^\s*use ([A-Za-z_][\w:]*::([A-Za-z_]\w*));')
CALL = re.compile(r'\b([A-Za-z_]\w*(?:::[A-Za-z_]\w*)+)\s*(?:::<[^>]*>)?\(')
PATH = re.compile(r'\b([A-Za-z_]\w*(?:::[A-Za-z_]\w*)+)')
SKIP = ("std::", "core::", "super::", "crate::", "Self::")


def entries():
    for path in sorted(PORTED.glob("*.rs")):
        lines = path.read_text().splitlines()
        alias = {}
        for line in lines:
            if m := USE_AS.match(line):
                alias[m.group(2)] = m.group(1)
            elif m := USE_PLAIN.match(line):
                alias[m.group(2)] = m.group(1)
        starts = [i for i, line in enumerate(lines) if ENTRY.match(line)]
        for k, i in enumerate(starts):
            m = ENTRY.match(lines[i])
            name, addr, conv, shim = m.groups()
            # the body: the rest of the entry's line (one-line shims) and
            # the lines up to the next entry
            end = starts[k + 1] if k + 1 < len(starts) else len(lines)
            body = "\n".join([lines[i][m.end():], *lines[i + 1:end]])
            target = ""
            # the first call; else the first path (shims returning a constant)
            for m in [*CALL.finditer(body), *PATH.finditer(body)]:
                path_ = m.group(1)
                if path_.startswith(SKIP):
                    continue
                head, _, rest = path_.partition("::")
                target = f"{alias[head]}::{rest}" if head in alias else path_
                break
            target = target or "(in the shim)"
            yield {
                "address": "0x" + addr[2:].replace("_", "").lower().zfill(8),
                "name": name,
                "conv": conv,
                "shim": f"{path.relative_to(REPO)}:{i + 1}",
                "shim_fn": shim,
                "port": target,
            }


def test_index():
    """address -> test files naming it (as 0x0079_fff0 or 0x0079fff0)."""
    found = {}
    lit = re.compile(r"0x([0-9a-fA-F]{4})_?([0-9a-fA-F]{4})\b")
    for path in sorted(TESTS.glob("*.rs")):
        rel = path.name
        for m in lit.finditer(path.read_text()):
            found.setdefault(f"0x{(m.group(1) + m.group(2)).lower()}", set()).add(rel)
    return found


COLUMNS = ["address", "name", "conv", "shim", "shim_fn", "port", "tests"]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--find")
    ap.add_argument("--html")
    args = ap.parse_args()
    tests = test_index()
    rows = []
    for e in entries():
        e["tests"] = " ".join(sorted(tests.get(e["address"], ())))
        rows.append(e)
    rows.sort(key=lambda r: r["address"])
    if args.find:
        q = args.find.lower()
        rows = [r for r in rows if q in r["name"].lower() or q in r["address"]]
    if args.html:
        body = "\n".join(
            "<tr>" + "".join(f"<td>{html.escape(r[c])}</td>" for c in COLUMNS) + "</tr>" for r in rows)
        pathlib.Path(args.html).write_text(f"""<!doctype html><meta charset=utf-8>
<title>ffx-decomp hook map</title>
<style>body{{font:13px system-ui,sans-serif;margin:1em}}table{{border-collapse:collapse}}
td,th{{padding:2px 8px;border-bottom:1px solid #ddd;text-align:left;font-family:ui-monospace,monospace}}
input{{font-size:14px;width:30em;margin-bottom:.5em}}</style>
<p>{len(rows)} hooks. <input id=q placeholder="filter by name, address, file or port" autofocus>
<table><thead><tr>{"".join(f"<th>{c}</th>" for c in COLUMNS)}</tr></thead><tbody id=t>
{body}
</tbody></table>
<script>q.oninput=()=>{{const v=q.value.toLowerCase();for(const r of t.rows)r.hidden=v&&!r.textContent.toLowerCase().includes(v)}}</script>
""")
        print(f"{len(rows)} hooks -> {args.html}", file=sys.stderr)
        return
    print("\t".join(COLUMNS))
    for r in rows:
        print("\t".join(r[c] for c in COLUMNS))


if __name__ == "__main__":
    main()
