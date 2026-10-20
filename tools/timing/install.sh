#!/bin/bash
# Installs the cargo timing shim as ~/.local/bin/cargo (docs/workflow.md,
# "Build and test timing"). ~/.local/bin comes before ~/.cargo/bin in PATH;
# outside ffx-decomp the shim execs the real cargo at once.
#   tools/timing/install.sh            build, test, install (atomic rename)
#   tools/timing/install.sh --remove   uninstall
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
dest="$HOME/.local/bin/cargo"
if [ "${1:-}" = --remove ]; then
    if [ -e "$dest" ] && ! grep -q FFX_TIMING "$dest"; then
        echo "$dest is not the timing shim; left alone" >&2
        exit 1
    fi
    rm -f "$dest"
    echo "removed $dest"
    exit 0
fi
if [ -e "$dest" ] && ! grep -q FFX_TIMING "$dest"; then
    echo "$dest exists and is not the timing shim; not overwriting" >&2
    exit 1
fi
mkdir -p "$HOME/.local/bin"
tmp="$dest.new.$$"
cc -O2 -Wall -o "$tmp" "$here/cargo-shim.c"
mkdir -p "$HOME/ffx-work/scratch"
"$here/test-shim.sh" "$HOME/ffx-work/scratch" "$tmp" >/dev/null 2>&1 || { rm -f "$tmp"; echo "shim tests failed" >&2; exit 1; }
"$here/test-shim.sh" "$here" "$tmp" >/dev/null 2>&1 || { rm -f "$tmp"; echo "shim tests failed" >&2; exit 1; }
mv -f "$tmp" "$dest"
echo "installed $dest -> $("$dest" --version)"
