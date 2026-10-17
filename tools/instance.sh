#!/usr/bin/env bash
# Builds (or refreshes) game instance N: a copy of the game folder made of
# hard links, so several harness games run side by side, each with its own
# DINPUT8.dll and ffx-hook.log (docs/harness.md, "Parallel games").
#
# Usage: tools/instance.sh N [--mod DIR]...   prints the instance folder
#
# The folder is $FFX_INSTANCES_DIR/N (default ~/ffx-work/instances/N). The
# hard links are rebuilt only when the game folder's listing (names, sizes,
# times, inodes) changes; mods/ is linked afresh every call, plus every
# --mod DIR as mods/<name of DIR>. Files that are written in place, or that
# each launch installs itself, are never shared with the game folder.
# The game folder and the instances must be on the same filesystem.
set -euo pipefail
n="${1:?usage: tools/instance.sh N [--mod DIR]...}"
shift
[[ "$n" =~ ^[0-9]+$ ]] || { echo "instance must be a number: $n" >&2; exit 2; }
src="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
root="${FFX_INSTANCES_DIR:-$HOME/ffx-work/instances}"
inst="$root/$n"
mods=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --mod) mods+=("$2"); shift ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
    shift
done

# Per launch or written by the game, the hook or the renderer: never linked.
# Small files the renderer reads or rewrites (tools write ffx-render.toml
# before a launch): copied afresh each call instead of linked.
copied=(ffx-render.toml ffx-render-shaders.tsv)
own='^(ffx-hook\.log|ffx-render\.log|CoreDump\.dmp|DINPUT8\.dll|ffx-launch\.exe|ffx-render\.exe|ffx-render\.toml|ffx-render-shaders\.tsv|\.ffx-instance)$'

signature() {
    (cd "$src" && find . \( -path ./mods -o -path ./captures -o -path ./profiles \) -prune \
        -o \( -type f -o -type l \) -printf '%P\t%s\t%T@\t%i\n') \
        | OWN="$own" awk -F'\t' '$1 !~ ENVIRON["OWN"]' | LC_ALL=C sort | sha256sum | cut -d' ' -f1
}

mkdir -p "$root"
sig="$(signature)"
if [[ "$(cat "$inst/.ffx-instance" 2>/dev/null)" != "$sig" ]]; then
    tmp="$root/.$n.building"
    rm -rf "$tmp"
    mkdir -p "$tmp"
    # Everything but mods/ (linked below, every time) and the renderer's
    # captures and profiles (its output).
    for entry in "$src"/* "$src"/.[!.]*; do
        [[ -e "$entry" ]] || continue
        case "${entry##*/}" in mods|captures|profiles) continue ;; esac
        [[ "${entry##*/}" =~ $own ]] && continue
        cp -al "$entry" "$tmp/"
    done
    if [[ -d "$src/profiles" ]]; then
        cp -r "$src/profiles" "$tmp/profiles"
    fi
    # Keep the instance's own log across rebuilds.
    if [[ -f "$inst/ffx-hook.log" ]]; then
        mv "$inst/ffx-hook.log" "$tmp/ffx-hook.log"
    fi
    printf '%s\n' "$sig" > "$tmp/.ffx-instance"
    rm -rf "$inst"
    mv "$tmp" "$inst"
fi

for f in "${copied[@]}"; do
    rm -f "$inst/$f"
    if [[ -f "$src/$f" ]]; then
        cp "$src/$f" "$inst/$f"
    fi
done

# Mods: the game folder's, then the extra ones; linked each time, so a mod
# written into the game folder (or removed) is seen by the next launch.
rm -rf "$inst/mods"
if [[ -d "$src/mods" ]]; then
    cp -al "$src/mods" "$inst/mods"
else
    mkdir -p "$inst/mods"
fi
for m in "${mods[@]}"; do
    name="$(basename "$m")"
    rm -rf "${inst:?}/mods/$name"
    cp -al "$m" "$inst/mods/$name" 2>/dev/null || cp -r "$m" "$inst/mods/$name"
done
echo "$inst"
