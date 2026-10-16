#!/usr/bin/env bash
# Imports FFX.exe into a persistent Ghidra project (first run only), applies
# symbols/ffx.sym and dumps decompiled code and data references to
# $FFX_WORK/decomp.
# Usage: tools/ghidra/dump.sh [path/to/FFX.exe]
set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
exe="${1:-${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}/FFX.exe}"
work="${FFX_WORK:-$HOME/ffx-work}"
ghidra="${GHIDRA_INSTALL_DIR:?set GHIDRA_INSTALL_DIR}"
export JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-21-openjdk-amd64}"
export GHIDRA_HEADLESS_MAXMEM="${GHIDRA_HEADLESS_MAXMEM:-16G}"

mkdir -p "$work/ghidra" "$work/decomp"
common=(-scriptPath "$repo/tools/ghidra"
        -postScript ApplySymbols.java "$repo/symbols/ffx.sym"
        -postScript DumpDecomp.java "$work/decomp"
        -postScript DumpDataRefs.java "$work/decomp"
        -postScript DumpCallSites.java "$work/decomp"
        -max-cpu "$(nproc)")

if [[ -d "$work/ghidra/ffx.rep" ]]; then
    "$ghidra/support/analyzeHeadless" "$work/ghidra" ffx -process FFX.exe -noanalysis "${common[@]}"
else
    "$ghidra/support/analyzeHeadless" "$work/ghidra" ffx -import "$exe" "${common[@]}"
fi
