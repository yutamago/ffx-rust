#!/usr/bin/env bash
# The Deck scripts against a stand-in Deck on this machine (no Deck, no game,
# no Proton): tools/deck/*.sh with DECK_LOCAL=<fake home>, which runs
# remote.sh here. The stand-in has Steam's folders (libraryfolders.vdf with a
# second library, appmanifest_359870.acf), a game folder with a user's
# DINPUT8 mod, log, steam_appid.txt and mods/, three Protons (the newest
# numbered one asks for the sniper runtime, as Valve's do) whose `proton`
# records how it was called and checks what Valve's script requires
# (STEAM_COMPAT_DATA_PATH, STEAM_COMPAT_CLIENT_INSTALL_PATH, a verb).
#
# Usage: tools/deck/deck-selftest.sh      (exit 0: every check passed)
#
# It checks: the game and Proton found; instances made of links, without the
# user's files, with ours copied in, by hard and by symbolic links; the
# prefix is ours; every start's environment (Steam kept out, app ids unset,
# the null sink, the home and harness, Z: paths, dinput8 overrides) and
# command (nice, ionice, the runtime's entry point, proton waitforexitandrun,
# ffx-launch for fixed addresses); no start while a `reaper` runs; the
# user's game folder byte for byte the same afterwards; dry runs print and
# start nothing.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
ok() { echo "ok   $*"; }
bad() { echo "FAIL $*"; fails=$((fails + 1)); }
check() { # description, command...
    local what="$1"
    shift
    if "$@"; then ok "$what"; else bad "$what"; fi
}

# --- the stand-in Deck ----------------------------------------------------
deck="$tmp/home/deck"
steam="$deck/.local/share/Steam"
sd="$tmp/run/media/mmcblk0p1"
game="$sd/steamapps/common/FINAL FANTASY FFX&FFX-2 HD Remaster"
calls="$tmp/calls"
mkdir -p "$steam/steamapps/common" "$sd/steamapps/common" "$game/data" "$game/mods/someones-mod" "$calls"
mkdir -p "$deck/.steam"
ln -s "$steam" "$deck/.steam/root"
cat > "$steam/steamapps/libraryfolders.vdf" <<EOF
"libraryfolders"
{
	"0"
	{
		"path"		"$steam"
		"apps"
		{
		}
	}
	"1"
	{
		"path"		"$sd"
		"apps"
		{
			"359870"		"40000000000"
		}
	}
}
EOF
cat > "$sd/steamapps/appmanifest_359870.acf" <<'EOF'
"AppState"
{
	"appid"		"359870"
	"name"		"FINAL FANTASY X/X-2 HD Remaster"
	"StateFlags"		"4"
	"installdir"		"FINAL FANTASY FFX&FFX-2 HD Remaster"
	"buildid"		"1234567"
}
EOF
echo "game program" > "$game/FFX.exe"
echo "archive" > "$game/data/FFX_Data.vbf"
echo "user's mod" > "$game/dinput8.dll"
echo "user's log" > "$game/ffx-hook.log"
echo 359870 > "$game/steam_appid.txt"
echo "mod" > "$game/mods/someones-mod/mod.txt"
echo "32-bit steam" > "$game/steam_api.dll"
game_sum() { (cd "$game" && find . -printf '%P %s %i %y\n' | LC_ALL=C sort && find . -type f -exec cat {} + ) | sha256sum; }
before="$(game_sum)"

# Protons: `proton` records the call and checks what Valve's script needs.
fake_proton() { # folder, runtime app id or ""
    mkdir -p "$1"
    cat > "$1/proton" <<EOF
#!/usr/bin/env bash
n=\$(find "$calls" -name '*.call' | wc -l)
f="$calls/\$n.call"
{ echo "proton=\$0"; echo "cwd=\$PWD"; printf 'arg=%s\n' "\$@"; env | grep -E '^(STEAM|Steam|FFX_|PULSE_|WINE|DISPLAY|SLR_)' | LC_ALL=C sort; } > "\$f"
[[ -n "\${STEAM_COMPAT_DATA_PATH:-}" ]] || { echo "No compat data path?"; exit 1; }
[[ -n "\${STEAM_COMPAT_CLIENT_INSTALL_PATH+x}" ]] || { echo "KeyError: 'STEAM_COMPAT_CLIENT_INSTALL_PATH'"; exit 1; }
mkdir -p "\$STEAM_COMPAT_DATA_PATH/pfx"
echo 9.0-fake > "\$STEAM_COMPAT_DATA_PATH/version"
case "\$1" in
    getcompatpath) p="\$2"; echo "Z:\${p//\//\\\\}" ;;
    run|waitforexitandrun) ;;
    *) echo "Need a verb."; exit 1 ;;
esac
EOF
    chmod +x "$1/proton"
    if [[ -n "$2" ]]; then
        printf '"manifest"\n{\n\t"version" "2"\n\t"commandline" "/proton %%verb%%"\n\t"require_tool_appid" "%s"\n}\n' "$2" > "$1/toolmanifest.vdf"
    fi
}
fake_proton "$steam/steamapps/common/Proton 9.0" 1628350
fake_proton "$steam/steamapps/common/Proton 10.0" 1628350
fake_proton "$steam/steamapps/common/Proton - Experimental" 1628350
fake_proton "$steam/compatibilitytools.d/GE-Proton9-20" ""
slr="$steam/steamapps/common/SteamLinuxRuntime_sniper"
mkdir -p "$slr"
cat > "$slr/_v2-entry-point" <<'EOF'
#!/usr/bin/env bash
verb=""
while [[ $# -gt 0 && "$1" != -- ]]; do
    case "$1" in --verb=*) verb="${1#--verb=}" ;; esac
    shift
done
shift
export SLR_VERB="$verb"
exec "$@"
EOF
chmod +x "$slr/_v2-entry-point"

# our programs, as deck-install.sh --from takes them
pkg="$tmp/pkg"
mkdir -p "$pkg"
for f in ffx-rust.exe ffx-launch.exe DINPUT8.dll steam_api64.dll ffx.sym; do echo "ours: $f" > "$pkg/$f"; done

export DECK_LOCAL="$deck" DECK_LOCAL_WORK="$tmp/local" DECK_ENV="$tmp/none.env"
export DECK_DISPLAY=none DECK_SOUND=0 DECK_SLR=auto DECK_PROTON="" DECK_GAME_DIR="" DECK_STEAM=0
W="$deck/ffx-deck"

# --- check ----------------------------------------------------------------
out="$("$here/deck-check.sh" 2>&1)" || { bad "deck-check.sh ran"; echo "$out"; }
check "check finds the game in the second library" grep -qF "game: $game" <<< "$out"
check "check reads the build id" grep -q 'buildid 1234567, StateFlags 4' <<< "$out"
check "check lists the Protons" grep -q 'GE-Proton9-20' <<< "$out"
check "check picks the newest numbered Proton, in the sniper runtime" \
    grep -qF "we use: $steam/steamapps/common/Proton 10.0, in $slr/_v2-entry-point" <<< "$out"

# --- install --------------------------------------------------------------
out="$("$here/deck-install.sh" --from "$pkg" 2>&1)" || { bad "deck-install.sh ran"; echo "$out"; }
inst="$W/instances/0"
check "instance 0 made" test -f "$inst/FFX.exe"
check "the game's files are hard links" test "$inst/FFX.exe" -ef "$game/FFX.exe"
check "the archive too" test "$inst/data/FFX_Data.vbf" -ef "$game/data/FFX_Data.vbf"
check "the user's dinput8.dll is left out, ours is in" bash -c "[[ ! -e '$inst/dinput8.dll' || \$(cat '$inst/DINPUT8.dll') == 'ours: DINPUT8.dll' ]] && [[ \$(ls '$inst' | grep -ci '^dinput8.dll\$') == 1 ]]"
check "the user's log is left out" test ! -e "$inst/ffx-hook.log"
check "steam_appid.txt is left out" test ! -e "$inst/steam_appid.txt"
check "the user's mods are left out" bash -c "[[ -d '$inst/mods' && -z \$(ls -A '$inst/mods') ]]"
check "ffx-rust.exe is our own file" bash -c "[[ \$(cat '$inst/ffx-rust.exe') == 'ours: ffx-rust.exe' && ! '$inst/ffx-rust.exe' -ef '$pkg/ffx-rust.exe' ]]"
check "our prefix made" test -d "$W/compatdata/pfx"
# shellcheck disable=SC1003 # a literal backslash
check "the prefix's Windows path is Z:" grep -qF 'our folder as Proton sees it: Z:\' <<< "$out"
check "install asked getcompatpath through the runtime" grep -qx 'SLR_VERB=waitforexitandrun' "$calls/0.call"
check "install handed Proton no Steam client" grep -qx "STEAM_COMPAT_CLIENT_INSTALL_PATH=$W/no-steam-client" "$calls/0.call"

# --- launches -------------------------------------------------------------
ncalls() { find "$calls" -name "*.call" | wc -l; }
wait_call() { # n

    for _ in $(seq 50); do [[ -s "$calls/$1.call" ]] && return 0; sleep 0.1; done
    return 1
}
export SteamAppId=359870 SteamGameId=359870
"$here/deck-run.sh" --rust --harness 27399 --fast --seed 1 --virtual-clock --background > "$tmp/run1.out" 2>&1 ||
    { bad "deck-run.sh --rust ran"; cat "$tmp/run1.out"; }
if wait_call 1; then
    c="$calls/1.call"
    check "rust: proton waitforexitandrun ffx-rust.exe" bash -c "grep -A2 -x 'arg=waitforexitandrun' '$c' | grep -qx 'arg=ffx-rust.exe'"
    check "rust: Proton 10.0" grep -qx "proton=$steam/steamapps/common/Proton 10.0/proton" "$c"
    check "rust: in the runtime" grep -qx 'SLR_VERB=waitforexitandrun' "$c"
    check "rust: in instance 0" grep -qx "cwd=$inst" "$c"
    check "rust: our prefix" grep -qx "STEAM_COMPAT_DATA_PATH=$W/compatdata" "$c"
    check "rust: no Steam client" grep -qx "STEAM_COMPAT_CLIENT_INSTALL_PATH=$W/no-steam-client" "$c"
    check "rust: FFX_NO_STEAM" grep -qx 'FFX_NO_STEAM=1' "$c"
    check "rust: the app ids unset" bash -c "! grep -qE '^(SteamAppId|SteamGameId)=' '$c'"
    check "rust: the null sink" grep -qx 'PULSE_SINK=ffx_deck_null' "$c"
    check "rust: the harness port" grep -qx 'FFX_HARNESS=27399' "$c"
    check "rust: the harness options" bash -c "grep -qx FFX_HARNESS_FAST=1 '$c' && grep -qx FFX_HARNESS_SEED=1 '$c' && grep -qx FFX_HARNESS_VIRTUAL_CLOCK=1 '$c'"
    check "rust: the home as a Z: path" grep -qxF "FFX_HARNESS_HOME=Z:${W//\//\\}\\homes\\0" "$c"
    check "rust: the home has the sandbox settings" grep -q '^Language=en' "$W/homes/0/Square Enix/FINAL FANTASY X&X-2 HD Remaster/GameSetting.ini"
else
    bad "rust: Proton was not called"; cat "$tmp/run1.out"
fi
FFX_AUDIO_WAV="$W/runs/x/a.wav" "$here/deck-run.sh" --virtual-clock --hook-disable replacements > "$tmp/run2.out" 2>&1 ||
    { bad "deck-run.sh (FFX.exe) ran"; cat "$tmp/run2.out"; }
if wait_call 2; then
    c="$calls/2.call"
    check "ffx: ffx-launch.exe FFX.exe for fixed addresses" bash -c "grep -A2 -x 'arg=waitforexitandrun' '$c' | tr '\n' ' ' | grep -q 'arg=ffx-launch.exe arg=FFX.exe'"
    check "ffx: the hook DLL first" grep -qx 'WINEDLLOVERRIDES=dinput8=n,b' "$c"
    check "ffx: replacements off" grep -qx 'FFX_HOOK_DISABLE=replacements' "$c"
    check "ffx: a path variable as a Z: path" grep -qxF "FFX_AUDIO_WAV=Z:${W//\//\\}\\runs\\x\\a.wav" "$c"
else
    bad "ffx: Proton was not called"; cat "$tmp/run2.out"
fi
DECK_FIXED=direct DECK_PROTON="GE-Proton9-20" "$here/deck-run.sh" --no-hooks --virtual-clock > "$tmp/run3.out" 2>&1 ||
    { bad "deck-run.sh --no-hooks ran"; cat "$tmp/run3.out"; }
if wait_call 3; then
    c="$calls/3.call"
    check "no hooks: Wine's own dinput8" grep -qx 'WINEDLLOVERRIDES=dinput8=b' "$c"
    check "DECK_FIXED=direct: FFX.exe itself" bash -c "grep -A1 -x 'arg=waitforexitandrun' '$c' | grep -qx 'arg=FFX.exe'"
    check "DECK_PROTON by name, no runtime asked" bash -c "grep -qx 'proton=$steam/compatibilitytools.d/GE-Proton9-20/proton' '$c' && ! grep -q '^SLR_VERB' '$c'"
else
    bad "no hooks: Proton was not called"; cat "$tmp/run3.out"
fi
DECK_STEAM=1 "$here/deck-run.sh" --rust > "$tmp/run4.out" 2>&1 || { bad "deck-run.sh DECK_STEAM=1 ran"; cat "$tmp/run4.out"; }
if wait_call 4; then
    c="$calls/4.call"
    check "DECK_STEAM=1: the real Steam client" grep -qx "STEAM_COMPAT_CLIENT_INSTALL_PATH=$(realpath "$steam")" "$c"
    check "DECK_STEAM=1: no FFX_NO_STEAM" bash -c "! grep -q '^FFX_NO_STEAM' '$c'"
else
    bad "DECK_STEAM=1: Proton was not called"
fi

# symbolic links (a library on another filesystem)
DECK_LINKS=symbolic DECK_LOCAL="$deck" DECK_WORK=ffx-deck bash -c ". '$here/lib.sh'; deck_load_env; deck_remote instance 1" > /dev/null
check "symbolic instance: FFX.exe links to the game" bash -c "[[ -L '$W/instances/1/FFX.exe' && '$W/instances/1/FFX.exe' -ef '$game/FFX.exe' ]]"
check "symbolic instance: our DINPUT8.dll is a file" bash -c "[[ -f '$W/instances/1/DINPUT8.dll' && ! -L '$W/instances/1/DINPUT8.dll' ]]"

# the user plays: a `reaper` process
# (a script's process is named after the script)
# shellcheck disable=SC2016 # the script is written as it is
printf '#!/bin/bash\nfor _ in $(seq 300); do sleep 0.1; done\n' > "$tmp/reaper"
chmod +x "$tmp/reaper"
"$tmp/reaper" &
reaper=$!
sleep 0.2
n_before="$(ncalls)"
if "$here/deck-run.sh" --rust > "$tmp/run5.out" 2>&1; then rc=0; else rc=$?; fi
kill "$reaper" 2>/dev/null || true
check "no start while the user plays (exit 4)" test "$rc" = 4
sleep 0.5
check "nothing started then" test "$(ncalls)" = "$n_before"

# dry runs start nothing
n_before="$(ncalls)"
out="$(DECK_DRY_RUN=1 "$here/deck-run.sh" --rust --harness 27399 2>&1)"
check "local dry run prints the start" grep -q '^\[dry\].*proton waitforexitandrun ffx-rust.exe' <<< "$out"
out="$(env -u DECK_LOCAL DECK_DRY_RUN=1 DECK_HOST=192.0.2.1 "$here/deck-run.sh" --rust --harness 27399 2>&1)"
check "remote dry run prints ssh, the copy and the forward" bash -c "grep -q '^\[dry\] ssh deck@192.0.2.1: .*remote.sh launch 0 rust' <<< \"\$1\" && grep -q '^\[dry\] copy ' <<< \"\$1\" && grep -q 'forward -L 127.0.0.1:27399:127.0.0.1:27399' <<< \"\$1\"" _ "$out"
sleep 0.5
check "dry runs started nothing" test "$(ncalls)" = "$n_before"

# stop, and the game folder untouched
"$here/deck-stop.sh" > /dev/null 2>&1 || bad "deck-stop.sh ran"
check "the user's game folder is unchanged" test "$(game_sum)" = "$before"
check "nothing of ours outside ~/ffx-deck" bash -c "cd '$deck' && [[ \$(ls -A | LC_ALL=C sort | tr '\n' ' ') == '.local .steam ffx-deck ' ]]"

echo
if [[ $fails == 0 ]]; then echo "deck-selftest: all passed"; else echo "deck-selftest: $fails failed"; exit 1; fi
