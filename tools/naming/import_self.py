#!/usr/bin/env python3
"""Names functions after their own __FUNCTION__ string.

Error prints in Phyre and in the PC code embed the whole C++ function name
("Phyre::PScripting::PScriptAccessors::PObjectAccessor<class Phyre::PTimer *>::Get").
When such a string occurs once and one function uses it, that function
takes the name (Class::method -> Class_method). Strings that are type names
rather than functions (no lower-case or ~ method part) are left out.

  tools/naming/import_self.py --out PROPOSALS.tsv [--strings $FFX_WORK/decomp/strings.tsv]
  then: tools/naming/apply.py PROPOSALS.tsv
"""
import argparse
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import symlib  # noqa: E402

FULL = re.compile(r"^[A-Za-z_][\w:<>,\s\*&~]*::(~?[a-z_][\w]*|[A-Z]\w*Handler|Get\w*)$")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--strings", default=str(symlib.DUMP / "strings.tsv"))
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    _, ent = symlib.load()
    used = {v[1] for v in ent.values()}
    rows = []
    for value, defs in symlib.read_strings(args.strings).items():
        s = value.strip()
        if not FULL.match(s) or len(defs) != 1 or len(defs[0]) != 1:
            continue
        a = defs[0][0]
        name = symlib.token(s.replace("class ", "").replace("struct ", ""))
        if a in ent or name in used:
            continue
        used.add(name)
        rows.append((a, name, f'str: the only user of the string "{s}" (its own __FUNCTION__ in an error print)'))
    rows.sort()
    symlib.write_tsv(args.out, rows)
    print(f"{len(rows)} proposals -> {args.out}")


if __name__ == "__main__":
    main()
