#!/usr/bin/env bash
# Puts our programs on the Steam Deck (docs/m6.md, "The Steam Deck").
#
# Usage: tools/deck/deck-install.sh [--no-build | --from DIR]
#
# Builds (unless --no-build) and stages in $DECK_LOCAL_WORK/bin:
#   ffx-rust.exe     tools/build-host.sh (FFX_HOST_FEATURES as there), or the
#                    file DECK_RUST_EXE names (a release package's, say)
#   ffx-launch.exe   crates/ffx-launch, x86_64
#   DINPUT8.dll      the hook, tools/build-hook.sh (FFX.exe's harness lives in it)
#   steam_api64.dll  the Steamworks SDK's (STEAMWORKS_SDK, default
#                    ~/tools/steamworks_sdk_165/sdk), for ffx-rust.exe
#   ffx.sym          symbols/ffx.sym, beside ffx-rust.exe for its reports
# --from DIR stages DIR's files of these names instead (no build).
# Then copies them to ~/$DECK_WORK/bin on the Deck, builds instance 0 there
# (the game folder linked into ~/$DECK_WORK/instances/0, our programs copied
# in) and makes our Proton prefix (~/$DECK_WORK/compatdata). The game folder,
# the user's saves and app 359870's prefix are never written.
set -euo pipefail
# shellcheck source=tools/deck/lib.sh
. "$(dirname "$0")/lib.sh"
deck_load_env
build=1
from=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-build) build=0 ;;
        --from) from="$(realpath "$2")"; build=0; shift ;;
        *) echo "usage: deck-install.sh [--no-build | --from DIR]" >&2; exit 2 ;;
    esac
    shift
done
deck_need_host
repo="$deck_repo"
rel64="$repo/target/x86_64-pc-windows-msvc/release"
rel32="$repo/target/i686-pc-windows-msvc/release"
if [[ $build == 1 ]]; then
    if deck_dry; then
        echo "[dry] tools/build-hook.sh; tools/build-host.sh; cargo build --release --target x86_64-pc-windows-msvc -p ffx-launch" >&2
    else
        "$repo/tools/build-hook.sh" >/dev/null
        [[ -n "${DECK_RUST_EXE:-}" ]] || "$repo/tools/build-host.sh" >/dev/null
        (eval "$("$repo/tools/xwin-env.sh" x86_64-pc-windows-msvc)" && cargo build -q --release \
            --manifest-path "$repo/Cargo.toml" --target x86_64-pc-windows-msvc -p ffx-launch >/dev/null)
    fi
fi
stage="$DECK_LOCAL_WORK/bin"
rm -rf "$stage"
mkdir -p "$stage"
sdk="${STEAMWORKS_SDK:-$HOME/tools/steamworks_sdk_165/sdk}"
missing=0
put() { # source, name
    if [[ -f "$1" ]]; then
        cp "$1" "$stage/$2"
    else
        echo "deck-install: no $2 at $1" >&2
        missing=1
    fi
}
if [[ -n "$from" ]]; then
    for f in ffx-rust.exe ffx-launch.exe DINPUT8.dll steam_api64.dll ffx.sym; do
        put "$from/$f" "$f"
    done
else
    put "${DECK_RUST_EXE:-$rel64/ffx-rust.exe}" ffx-rust.exe
    put "$rel64/ffx-launch.exe" ffx-launch.exe
    put "$rel32/dinput8.dll" DINPUT8.dll
    put "$sdk/redistributable_bin/win64/steam_api64.dll" steam_api64.dll
    put "$repo/symbols/ffx.sym" ffx.sym  # ffx-rust.exe's report and --profile name functions by it
fi
if [[ $missing == 1 ]] && ! deck_dry; then
    exit 1
fi
deck_put "$stage" "$DECK_WORK/bin"
deck_remote install
