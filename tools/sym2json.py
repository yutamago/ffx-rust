#!/usr/bin/env python3
"""Prints symbols/ffx.sym as {"0xaddr": "name"} JSON, the input REA's
set_addresses_names tool takes. REA's Ghidra sessions forget names when they
close, so apply this at the start of each session."""
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else pathlib.Path(__file__).parent.parent / "symbols/ffx.sym")
names = {}
for line in path.read_text().splitlines():
    line = line.split("#", 1)[0].split()
    if len(line) == 3:
        names[hex(int(line[0], 16))] = line[2]
json.dump(names, sys.stdout, indent=1)
print()
