#!/bin/bash
# The saves and settings round trip (M5 stage 5; docs/harness.md, "Saves
# round trip"): what ffx-rust.exe writes, FFX.exe reads, and the other way
# round, byte for byte.
#
#   tools/saves-roundtrip.sh [--out DIR] [step...]   # run the steps, then compare
#   tools/saves-roundtrip.sh --compare DIR           # compare only (no game)
#
# Steps (default all four, in this order):
#   write-orig  FFX.exe, original code, plays tools/replays/saves-write.txt:
#               a menu save (New Save Data) and two settings (VoiceVolume 70,
#               MusicVolume 60) written by the PC menu's close (Settings_Save)
#   write-rust  the same on ffx-rust.exe
#   read-orig   FFX.exe, original code, starts with write-rust's home copied
#               verbatim (Options::home_from) and plays saves-read.txt: the
#               settings read at boot, the save loaded through the title's
#               Load, both written back (a new save, the PC menu's close)
#   read-rust   the same on ffx-rust.exe with write-orig's home
# Every game runs in the harness's background mode, in a game slot, in its
# own sandboxed home DIR/<step>/home (what it started with: DIR/<step>/seed):
# never the user's saves folder or settings, never the game folder. The
# writers' homes start as every test game's (the user's saves copied, or
# FFX_SAVES_FROM on a second machine; tools/sandbox/GameSetting.ini).
#
# Then `saves_roundtrip compare` (crates/ffx-harness/src/saves.rs) reports
# every file each game wrote (saves decoded, CRCs checked), the writers
# against each other, each reader against the writer it read, and the
# readers against each other; only the play time and the CRCs over it may
# differ (docs/formats/save.md, "What a game writes"). The report is
# DIR/report.txt. Exit: 0 pass, 1 a check failed, 2 runs missing, 3 a run
# failed.
#
# DIR defaults to ~/ffx-work/saves-roundtrip/<date-time>. FFX_ROUNDTRIP_FFX=
# hooked runs FFX.exe with the Rust replacements instead of the original
# code. FFX_NO_BUILD=1: use the hook and ffx-rust.exe already in target/
# (PC2, which cannot build Ruffle).
set -u
repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"
export FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}"
out=""
compare_only=""
steps=()
while (($#)); do
    case "$1" in
        --out) out="$2"; shift ;;
        --compare) compare_only=1; out="$2"; shift ;;
        -h|--help) sed -n '2,37p' "$0"; exit 0 ;;
        *) steps+=("$1") ;;
    esac
    shift
done
out="${out:-$HOME/ffx-work/saves-roundtrip/$(date +%Y%m%d-%H%M%S)}"
mkdir -p "$out"
cargo build -q --release -p ffx-harness --example saves_roundtrip || exit 1
bin="$repo/target/release/examples/saves_roundtrip"
if [[ -n "$compare_only" ]]; then
    exec "$bin" compare "$out"
fi
# Build once: run-game.sh would rebuild at every launch.
if [[ "${FFX_NO_BUILD:-}" != 1 ]]; then
    need_hook="" need_host=""
    for s in "${steps[@]:-write-orig write-rust read-orig read-rust}"; do
        [[ "$s" == *-orig* ]] && need_hook=1
        [[ "$s" == *-rust* ]] && need_host=1
    done
    if [[ -n "$need_hook" ]]; then tools/build-hook.sh >/dev/null || exit 1; fi
    if [[ -n "$need_host" ]]; then tools/build-host.sh >/dev/null || exit 1; fi
    export FFX_NO_BUILD=1
fi
# ffx-rust.exe's updates are emulated: a long reply (a waitfor) may take long
export FFX_HARNESS_REPLY_TIMEOUT="${FFX_HARNESS_REPLY_TIMEOUT:-3600}"
echo "saves round trip: $out" >&2
"$bin" run "$out" "${steps[@]}" 2>&1 | tee -a "$out/run.log"
exit "${PIPESTATUS[0]}"
