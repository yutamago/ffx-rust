#!/usr/bin/env bash
# Compiles crates/ffx-host/src/gfx/composite.hlsl (ffx-host's Iggy
# composite, docs/host.md "Iggy") with the Windows SDK's fxc and prints the
# two Rust arrays that crates/ffx-host/src/gfx/composite.rs holds
# (VS_DXBC, PS_DXBC). Needs a Windows SDK (any 10.0.* under
# "Program Files (x86)/Windows Kits/10/bin") reachable through /mnt/c.
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
fxc="$(ls -1d "/mnt/c/Program Files (x86)/Windows Kits/10/bin/"10.*/x64/fxc.exe 2>/dev/null | sort -V | tail -1)"
[[ -x "$fxc" ]] || { echo "no fxc.exe in the Windows SDK" >&2; exit 2; }
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cp "$repo/crates/ffx-host/src/gfx/composite.hlsl" "$work/"
cd "$work"
"$fxc" /nologo /T vs_4_0 /E vs_main /O3 /Fo vs.dxbc composite.hlsl | tr -d '\r' >&2
"$fxc" /nologo /T ps_4_0 /E ps_main /O3 /Fo ps.dxbc composite.hlsl | tr -d '\r' >&2
for n in vs ps; do
    python3 -I - "$n" <<'EOF'
import sys
n = sys.argv[1]
b = open(n + ".dxbc", "rb").read()
print(f"const {n.upper()}_DXBC: [u8; {len(b)}] = [")
for i in range(0, len(b), 12):
    print("    " + " ".join(f"0x{x:02x}," for x in b[i:i + 12]))
print("];")
EOF
done
