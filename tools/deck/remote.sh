#!/usr/bin/env bash
# The Steam Deck's side of tools/deck/*.sh (docs/m6.md, "The Steam Deck"). It
# reaches the Deck on stdin of `ssh deck@<ip> bash -s -- FUNCTION ARG...`
# (tools/deck/lib.sh, deck_remote), so nothing is installed for it.
#
# Rules it keeps: it writes only under ~/$DECK_WORK (instances, the Proton
# prefix, homes, runs, logs); the game folder, the user's saves, settings and
# app 359870's prefix are only read; a game of ours never starts while the
# user plays (a `reaper` process: Steam starts every game under one) unless
# DECK_FORCE=1; no sound reaches the speakers unless DECK_SOUND=1; Steam is
# kept away from our games unless DECK_STEAM=1.
#
# Functions: check, paths, install, instance N, launch ..., status, stop,
# fetchlog N NAME.
set -euo pipefail

W="$HOME/${DECK_WORK:-ffx-deck}"
APP=359870
dry() { [[ "${DECK_DRY_RUN:-}" == 1 ]]; }
die() { echo "deck: $*" >&2; exit 2; }

# Steam's root folder (its steamapps holds libraryfolders.vdf).
steam_root() {
    local d
    for d in "$HOME/.steam/root" "$HOME/.local/share/Steam" "$HOME/.steam/steam"; do
        if [[ -f "$d/steamapps/libraryfolders.vdf" || -d "$d/steamapps" ]]; then
            realpath "$d"
            return 0
        fi
    done
    return 1
}

# Every library folder, one per line: libraryfolders.vdf's "path" values, the
# root's own library first.
libraries() {
    local root
    root="$(steam_root)" || return 0
    printf '%s\n' "$root"
    [[ -f "$root/steamapps/libraryfolders.vdf" ]] || return 0
    # "path"		"/run/media/mmcblk0p1" (VDF: quoted key, quoted value; \\ escapes)
    sed -n 's/^[[:space:]]*"path"[[:space:]]*"\(.*\)"[[:space:]]*$/\1/p' \
        "$root/steamapps/libraryfolders.vdf" | sed 's/\\\\/\\/g' | while IFS= read -r p; do
        [[ -d "$p" ]] && [[ "$(realpath "$p")" != "$root" ]] && realpath "$p"
    done
    return 0
}

# A VDF file's value for KEY (the first one).
vdf_value() { # file key
    sed -n "s/^[[:space:]]*\"$2\"[[:space:]]*\"\\(.*\\)\"[[:space:]]*\$/\\1/p" "$1" | head -1
}

# The game folder: DECK_GAME_DIR, else the library whose
# appmanifest_359870.acf names it (its "installdir").
game_dir() {
    if [[ -n "${DECK_GAME_DIR:-}" ]]; then
        [[ -f "$DECK_GAME_DIR/FFX.exe" ]] || die "DECK_GAME_DIR has no FFX.exe: $DECK_GAME_DIR"
        realpath "$DECK_GAME_DIR"
        return 0
    fi
    local lib acf dir
    while IFS= read -r lib; do
        acf="$lib/steamapps/appmanifest_$APP.acf"
        [[ -f "$acf" ]] || continue
        dir="$lib/steamapps/common/$(vdf_value "$acf" installdir)"
        if [[ -f "$dir/FFX.exe" ]]; then
            printf '%s\n' "$dir"
            return 0
        fi
    done < <(libraries)
    return 1
}

# The library holding a folder (for the runtime container's mounts).
library_of() {
    local lib
    while IFS= read -r lib; do
        [[ "$1" == "$lib"/* ]] && { printf '%s\n' "$lib"; return 0; }
    done < <(libraries)
    return 1
}

# Every Proton, "name<TAB>folder": the libraries' steamapps/common/Proton*,
# then compatibilitytools.d (user and system).
protons() {
    local lib d root
    while IFS= read -r lib; do
        for d in "$lib"/steamapps/common/*/; do
            d="${d%/}"
            [[ -f "$d/proton" ]] && printf '%s\t%s\n' "${d##*/}" "$d"
        done
    done < <(libraries)
    root="$(steam_root)" || root=""
    for d in ${root:+"$root"/compatibilitytools.d/*/} /usr/share/steam/compatibilitytools.d/*/; do
        d="${d%/}"
        [[ -f "$d/proton" ]] && printf '%s\t%s\n' "${d##*/}" "$d"
    done
    return 0
}

# The Proton folder to use: DECK_PROTON (a name or a path), else the newest
# numbered "Proton N.N", else Experimental, else any.
proton_dir() {
    local want="${DECK_PROTON:-}" list
    list="$(protons)"
    if [[ -n "$want" ]]; then
        if [[ -f "$want/proton" ]]; then realpath "$want"; return 0; fi
        if [[ "$want" == */proton && -f "$want" ]]; then dirname "$(realpath "$want")"; return 0; fi
        awk -F'\t' -v w="$want" '$1 == w { print $2; f = 1; exit } END { exit !f }' <<< "$list" ||
            die "no Proton named \"$want\" (deck-check.sh lists them)"
        return 0
    fi
    local pick
    pick="$(awk -F'\t' '$1 ~ /^Proton [0-9]+\.[0-9]+$/' <<< "$list" | sort -t$'\t' -k1,1V | tail -1 | cut -f2)"
    [[ -z "$pick" ]] && pick="$(awk -F'\t' '$1 == "Proton - Experimental" { print $2 }' <<< "$list" | head -1)"
    [[ -z "$pick" ]] && pick="$(head -1 <<< "$list" | cut -f2)"
    [[ -n "$pick" ]] || return 1
    printf '%s\n' "$pick"
}

# The Steam Linux Runtime entry point Proton asks for (toolmanifest.vdf's
# require_tool_appid: 1628350 sniper, 1391110 soldier), or nothing.
slr_entry() { # proton folder
    [[ "${DECK_SLR:-auto}" == 0 ]] && return 0
    local id name lib e
    id="$(vdf_value "$1/toolmanifest.vdf" require_tool_appid 2>/dev/null || true)"
    case "$id" in
        1628350) name=SteamLinuxRuntime_sniper ;;
        1391110) name=SteamLinuxRuntime_soldier ;;
        *) name="" ;;
    esac
    if [[ -n "$name" ]]; then
        while IFS= read -r lib; do
            e="$lib/steamapps/common/$name/_v2-entry-point"
            [[ -x "$e" ]] && { printf '%s\n' "$e"; return 0; }
        done < <(libraries)
    fi
    [[ "${DECK_SLR:-auto}" == 1 ]] && die "Proton asks for runtime ${id:-none}, which is not installed"
    return 0
}

# Processes of this user named exactly NAME, ours (FFX_DECK_RUN in their
# environment) left out.
user_procs() {
    local pid
    for pid in $(pgrep -u "$(id -u)" -x "$1" 2>/dev/null || true); do
        tr '\0' '\n' < "/proc/$pid/environ" 2>/dev/null | grep -qx 'FFX_DECK_RUN=1' && continue
        printf '%s\n' "$pid"
    done
}

# game (Game Mode: gamescope's session), desktop (Plasma) or none.
session_mode() {
    if [[ -n "$(user_procs gamescope)$(user_procs gamescope-wl)" ]]; then
        echo game
    elif [[ -n "$(user_procs plasmashell)" ]]; then
        echo desktop
    else
        echo none
    fi
}

# The display variables of the running session, as VAR=value lines, read
# from the Steam client's environment (Game Mode) or Plasma's (Desktop Mode).
session_env() {
    local pid
    case "$(session_mode)" in
        game) pid="$(user_procs steam | head -1)" ;;
        desktop) pid="$(user_procs plasmashell | head -1)" ;;
        *) return 0 ;;
    esac
    [[ -n "$pid" ]] || return 0
    tr '\0' '\n' < "/proc/$pid/environ" 2>/dev/null |
        grep -E '^(DISPLAY|XAUTHORITY|WAYLAND_DISPLAY|XDG_RUNTIME_DIR)=' || true
}

gamescope_headless() {
    command -v gamescope >/dev/null && gamescope --help 2>&1 | grep -q -- 'headless'
}

# The user plays: a game Steam started (`reaper SteamLaunch AppId=N ...`).
playing() {
    local pid out=""
    for pid in $(user_procs reaper); do
        out+="$(tr '\0' ' ' < "/proc/$pid/cmdline" 2>/dev/null | grep -o 'AppId=[0-9]*' || echo reaper) "
    done
    printf '%s' "${out% }"
}

# A Deck path as Proton's programs see it (drive Z: is /).
winpath() { local p="$1"; printf 'Z:%s' "${p//\//\\}"; }

cmd_paths() {
    printf 'HOME=%s\nWORK=%s\n' "$HOME" "$W"
}

cmd_check() {
    echo "== the Deck"
    echo "host: $(uname -n), kernel $(uname -r)"
    if [[ -f /etc/os-release ]]; then
        # shellcheck source=/dev/null
        echo "system: $(. /etc/os-release && echo "${PRETTY_NAME:-$NAME} ${VERSION_ID:-} ${BUILD_ID:-}")"
    fi
    echo "user: $(id -un) ($(id -u)), home $HOME"
    echo "free disk: home $(df -h --output=avail "$HOME" | tail -1 | tr -d ' ')"
    local lib
    while IFS= read -r lib; do
        echo "free disk: library $lib $(df -h --output=avail "$lib" | tail -1 | tr -d ' ')"
    done < <(libraries)
    echo "memory: $(free -h | awk '/^Mem:/ { print $2 " total, " $7 " available" }')"
    echo "cpu: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //')"
    local gpu
    gpu="$(for d in /sys/class/drm/card*/device; do
        if [[ -f "$d/vendor" ]]; then printf '%s:%s ' "$(cat "$d/vendor")" "$(cat "$d/device")"; fi
    done)"
    echo "gpu (pci ids): ${gpu:-none found}"
    if command -v vulkaninfo >/dev/null; then
        echo "vulkan: $(vulkaninfo --summary 2>/dev/null | grep -m2 -E 'deviceName|driverInfo' | sed 's/^[[:space:]]*//' | tr '\n' ' ')"
    fi
    echo "session: $(session_mode)"
    session_env | sed 's/^/  /'
    if command -v gamescope >/dev/null; then
        echo "gamescope: $(gamescope --version 2>&1 | head -1), headless back end: $(gamescope_headless && echo yes || echo no)"
    else
        echo "gamescope: none"
    fi
    echo "user plays now: $(playing || true)"
    echo "sound: pactl $(command -v pactl >/dev/null && echo yes || echo no), default sink $(pactl get-default-sink 2>/dev/null || echo '?')"
    echo "sshd: $(systemctl is-active sshd 2>/dev/null || echo '?')"
    echo "== Steam"
    echo "root: $(steam_root || echo 'not found')"
    libraries | sed 's/^/library: /'
    local g
    if g="$(game_dir)"; then
        echo "game: $g ($(du -sh "$g" 2>/dev/null | cut -f1))"
        local acf
        acf="$(library_of "$g")/steamapps/appmanifest_$APP.acf"
        echo "  buildid $(vdf_value "$acf" buildid), StateFlags $(vdf_value "$acf" StateFlags) (4 = installed)"
        echo "  same filesystem as $W: $([[ "$(stat -c %d "$g")" == "$(stat -c %d "$HOME")" ]] && echo 'yes (hard links)' || echo 'no (symbolic links)')"
        echo "  the user's prefix (read only, never used): $(library_of "$g")/steamapps/compatdata/$APP $([[ -d "$(library_of "$g")/steamapps/compatdata/$APP" ]] && echo exists || echo missing)"
    else
        echo "game: not installed (no appmanifest_$APP.acf with FFX.exe)"
    fi
    echo "protons:"
    protons | while IFS=$'\t' read -r n d; do
        echo "  $n  ($d)$( [[ -f "$d/version" ]] && echo ", version $(cat "$d/version")")$( [[ -n "$(vdf_value "$d/toolmanifest.vdf" require_tool_appid 2>/dev/null || true)" ]] && echo ", runtime $(vdf_value "$d/toolmanifest.vdf" require_tool_appid)")"
    done
    local root map
    root="$(steam_root || true)"
    if [[ -n "$root" && -f "$root/config/config.vdf" ]]; then
        map="$(awk -v app="\"$APP\"" '$1 == app { f = 1 } f && /"name"/ { print $2; exit }' "$root/config/config.vdf" | tr -d '"')"
        echo "Steam's choice for $APP (config.vdf CompatToolMapping): ${map:-none set, so the default}"
    fi
    local p
    if p="$(proton_dir)"; then
        echo "we use: $p$( [[ -n "$(slr_entry "$p")" ]] && echo ", in $(slr_entry "$p")")"
    else
        echo "we use: no Proton installed"
    fi
    echo "== ours ($W)"
    if [[ -d "$W" ]]; then
        echo "size: $(du -sh "$W" | cut -f1)"
        find "$W/bin" -maxdepth 1 -type f -printf 'bin: %f\n' 2>/dev/null | sort || true
        find "$W/instances" -mindepth 1 -maxdepth 1 -type d ! -name '.*' -printf 'instance: %p\n' 2>/dev/null | sort || true
        if [[ -d "$W/compatdata/pfx" ]]; then
            echo "prefix: $W/compatdata (version $(cat "$W/compatdata/version" 2>/dev/null || echo '?'))"
        fi
    else
        echo "not made yet (deck-install.sh)"
    fi
    cmd_status
}

# Instance N: the game folder linked into $W/instances/N (hard links on the
# same filesystem, else symbolic links), without the files our runs write
# or install, without the user's mods and without steam_appid.txt; then our
# programs from $W/bin copied in. Rebuilt only when the game folder changes.
own_re='^(ffx-hook\.log|ffx-render\.log|ffx-rust\.log|ffx-rust\.previous\.log|ffx-rust\.profile\.tsv|ffx-rust\.profile\.part|ffx-rust\.entries\.tsv|ffx-rust\.exe|ffx-launch\.exe|steam_api64\.dll|dinput8\.dll|steam_appid\.txt|coredump\.dmp|\.ffx-instance)$'
cmd_instance() {
    local n="${1:?instance number}"
    [[ "$n" =~ ^[0-9]+$ ]] || die "instance must be a number: $n"
    local src inst
    src="$(game_dir)" || die "the game is not installed (deck-check.sh)"
    inst="$W/instances/$n"
    case "$W/" in "$src"/*) die "our folder $W is inside the game folder" ;; esac
    mkdir -p "$W/instances"
    local mode=hard
    [[ "$(stat -c %d "$src")" == "$(stat -c %d "$W/instances")" ]] || mode=symbolic
    # DECK_LINKS=hard|symbolic: deck-selftest.sh tries both on one filesystem
    case "${DECK_LINKS:-auto}" in hard|symbolic) mode="$DECK_LINKS" ;; esac
    local sig
    sig="$(cd "$src" && find . -path ./mods -prune -o \( -type f -o -type l \) -printf '%P\t%s\t%T@\t%i\n' |
        LC_ALL=C sort | sha256sum | cut -d' ' -f1) $mode"
    if [[ "$(cat "$inst/.ffx-instance" 2>/dev/null)" != "$sig" ]]; then
        local tmp="$W/instances/.$n.building" entry name
        rm -rf "$tmp"
        mkdir -p "$tmp"
        shopt -s nocasematch dotglob
        for entry in "$src"/*; do
            name="${entry##*/}"
            [[ "$name" == mods || "$name" =~ $own_re ]] && continue
            if [[ $mode == hard ]]; then cp -al "$entry" "$tmp/"; else cp -as "$entry" "$tmp/"; fi
        done
        shopt -u nocasematch dotglob
        # the same names below the top level (a stray log in a subfolder) are
        # never shared either: drop the links, the game writes new files
        find "$tmp" -mindepth 2 \( -iname 'ffx-*.log' -o -iname 'coredump.dmp' -o -iname 'steam_appid.txt' \) -delete
        [[ -f "$inst/ffx-hook.log" ]] && mv "$inst/ffx-hook.log" "$tmp/ffx-hook.log"
        printf '%s\n' "$sig" > "$tmp/.ffx-instance"
        rm -rf "$inst"
        mv "$tmp" "$inst"
    fi
    rm -rf "$inst/mods"
    mkdir -p "$inst/mods"
    local f
    for f in "$W"/bin/*; do
        [[ -f "$f" ]] || continue
        rm -f "$inst/${f##*/}"  # never write through a link into the game folder
        cp "$f" "$inst/${f##*/}"
    done
    printf '%s\n' "$inst"
}

# The environment and command every launch shares, in the arrays run_env
# and run_cmd: our prefix, Steam kept out, the runtime container, the
# display, the sound, the priority.
run_env=()
run_cmd=()
prepare_run() { # instance folder
    local inst="$1" proton entry root lib libs=""
    proton="$(proton_dir)" || die "no Proton installed"
    entry="$(slr_entry "$proton")"
    root="$(steam_root)" || die "no Steam folder"
    mkdir -p "$W/compatdata" "$W/logs" "$W/run" "$W/no-steam-client"
    while IFS= read -r lib; do libs+="${libs:+:}$lib"; done < <(libraries)
    run_env=(
        -u SteamAppId -u SteamGameId -u SteamOverlayGameId -u STEAM_COMPAT_APP_ID
        FFX_DECK_RUN=1
        "STEAM_COMPAT_DATA_PATH=$W/compatdata"
        "STEAM_COMPAT_INSTALL_PATH=$inst"
        "STEAM_COMPAT_LIBRARY_PATHS=$libs"
        "STEAM_COMPAT_MOUNTS=$libs"
    )
    if [[ "${DECK_STEAM:-0}" == 1 ]]; then
        run_env+=("STEAM_COMPAT_CLIENT_INSTALL_PATH=$root")
    else
        # an empty client folder: Proton copies no steamclient into our prefix
        run_env+=("STEAM_COMPAT_CLIENT_INSTALL_PATH=$W/no-steam-client" FFX_NO_STEAM=1)
    fi
    [[ -n "${XDG_RUNTIME_DIR:-}" ]] || run_env+=("XDG_RUNTIME_DIR=/run/user/$(id -u)")
    if [[ "${DECK_SOUND:-0}" != 1 ]]; then
        sink_ready || die "no null sink for the sound (pactl); DECK_SOUND=1 plays on the Deck's output"
        run_env+=(PULSE_SINK=ffx_deck_null)
    fi
    local display="${DECK_DISPLAY:-auto}" gs=()
    if [[ "$display" == auto ]]; then
        if gamescope_headless; then display=headless; else display=session; fi
    fi
    case "$display" in
        headless)
            gamescope_headless || die "this gamescope has no headless back end (DECK_DISPLAY=session shows the window)"
            gs=(gamescope --backend headless -w 1280 -h 720 -W 1280 -H 720 --)
            ;;
        session)
            local line
            while IFS= read -r line; do [[ -n "$line" ]] && run_env+=("$line"); done < <(session_env)
            ;;
        none) ;;  # the stand-in Deck of deck-selftest.sh
        *) die "DECK_DISPLAY: auto, headless or session, not $display" ;;
    esac
    run_cmd=(nice -n "${DECK_NICE:-19}" ionice -c 3 "${gs[@]}")
    if [[ -n "$entry" ]]; then
        run_cmd+=("$entry" --verb=waitforexitandrun --)
    fi
    run_cmd+=("$proton/proton" waitforexitandrun)
}

# The null sink our games play into (made once; deck-stop.sh removes it).
sink_ready() {
    [[ -n "${DECK_LOCAL_TEST:-}" ]] && return 0
    command -v pactl >/dev/null || return 1
    export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
    pactl list short sinks 2>/dev/null | awk '{ print $2 }' | grep -qx ffx_deck_null && return 0
    dry && { echo "[dry] pactl load-module module-null-sink sink_name=ffx_deck_null" >&2; return 0; }
    mkdir -p "$W/run"
    pactl load-module module-null-sink sink_name=ffx_deck_null \
        sink_properties=device.description=ffx-deck-null > "$W/run/sink.module"
}

# install: instance 0 and the prefix made (the first start of a prefix takes
# a minute; getcompatpath sets it up and answers with our folder's Windows
# path, the drive-Z mapping every path we hand the game relies on).
cmd_install() {
    local inst
    inst="$(cmd_instance 0)"
    echo "instance: $inst"
    DECK_DISPLAY=none prepare_run "$inst"
    local cmd=("${run_cmd[@]}")
    cmd[${#cmd[@]}-1]=getcompatpath
    if dry; then
        echo "[dry] env ${run_env[*]} ${cmd[*]} $W"
        return 0
    fi
    local got
    got="$(cd "$inst" && env "${run_env[@]}" "${cmd[@]}" "$W" 2>"$W/logs/install.log" | tail -1 | tr -d '\r')"
    echo "prefix: $W/compatdata ($(cat "$W/compatdata/version" 2>/dev/null || echo 'version ?'))"
    echo "our folder as Proton sees it: ${got:-? (see $W/logs/install.log)} (expected $(winpath "$W"))"
}

# launch N rust|ffx [--no-hooks] [--fixed] [--home REL] [--harness PORT]
#        [--arg WORD]... [VAR=VALUE]...
# Starts the game in instance N, detached, at low priority; returns at once.
# VALUEs of the path variables (below) that start with / become Z: paths.
path_vars=' FFX_COVERAGE FFX_HOOK_HITS FFX_FMOD_LOG FFX_IGGY_LOG FFX_AUDIO_WAV FFX_HOST_IGGY_OUT '
cmd_launch() {
    local n="${1:?instance}" exe="${2:?rust or ffx}" hooks=1 fixed="" home="" harness="" args=() vars=()
    shift 2
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --no-hooks) hooks=0 ;;
            --fixed) fixed=1 ;;
            --home) home="$2"; shift ;;
            --harness) harness="$2"; shift ;;
            --arg) args+=("$2"); shift ;;
            *=*)
                local k="${1%%=*}" v="${1#*=}"
                [[ "$k" =~ ^FFX_[A-Z0-9_]+$ ]] || die "only FFX_ variables pass: $k"
                [[ "$path_vars" == *" $k "* && "$v" == /* ]] && v="$(winpath "$v")"
                vars+=("$k=$v")
                ;;
            *) die "launch: unknown argument $1" ;;
        esac
        shift
    done
    local who
    who="$(playing || true)"
    if [[ -n "$who" && "${DECK_FORCE:-}" != 1 ]]; then
        echo "deck: the user is playing ($who); not starting a game (DECK_FORCE=1 overrides)" >&2
        exit 4
    fi
    local inst
    inst="$(cmd_instance "$n")"
    prepare_run "$inst"
    run_env+=("${vars[@]}")
    if [[ -n "$home" ]]; then
        mkdir -p "$W/$home"
        run_env+=("FFX_HARNESS_HOME=$(winpath "$W/$home")")
    fi
    [[ -n "$harness" ]] && run_env+=("FFX_HARNESS=$harness")
    local log prog=()
    case "$exe" in
        rust)
            [[ -f "$inst/ffx-rust.exe" ]] || die "no ffx-rust.exe in $inst (deck-install.sh)"
            rm -f "$inst/ffx-rust.log" "$inst/ffx-rust.profile.tsv" "$inst/ffx-rust.profile.part"
            log="$inst/ffx-rust.log"
            prog=(ffx-rust.exe "${args[@]}")
            ;;
        ffx)
            log="$W/logs/$n.log"
            if [[ $hooks == 1 ]]; then
                [[ -f "$inst/DINPUT8.dll" ]] || die "no DINPUT8.dll (the hook) in $inst (deck-install.sh)"
                # Wine prefers its own dinput8 unless told: the folder's first
                run_env+=("WINEDLLOVERRIDES=dinput8=n,b")
            else
                run_env+=("WINEDLLOVERRIDES=dinput8=b")
            fi
            if [[ -n "$fixed" && "${DECK_FIXED:-launch}" == launch ]]; then
                [[ -f "$inst/ffx-launch.exe" ]] || die "no ffx-launch.exe in $inst (deck-install.sh)"
                prog=(ffx-launch.exe FFX.exe)
            else
                prog=(FFX.exe)
            fi
            ;;
        *) die "launch: rust or ffx, not $exe" ;;
    esac
    if dry; then
        echo "[dry] cd $inst"
        printf '[dry] env'; printf ' %q' "${run_env[@]}"; echo
        printf '[dry]'; printf ' %q' "${run_cmd[@]}" "${prog[@]}"; echo " > $log 2>&1 &"
        return 0
    fi
    cd "$inst"
    setsid nohup env "${run_env[@]}" "${run_cmd[@]}" "${prog[@]}" > "$log" 2>&1 < /dev/null &
    echo $! > "$W/run/$n.pid"
    echo "started ${prog[*]} in $inst (process group $!, log $log)"
}

# Our games: the launch process groups still alive.
cmd_status() {
    local f pid any=""
    for f in "$W"/run/*.pid; do
        [[ -f "$f" ]] || continue
        pid="$(cat "$f")"
        if kill -0 "$pid" 2>/dev/null; then
            echo "running: instance $(basename "$f" .pid), process group $pid"
            any=1
        fi
    done
    [[ -n "$any" ]] || echo "running: none of ours"
}

# stop: ends our games (our prefix's wineserver, our process groups) and
# removes our null sink. Nothing else is touched.
cmd_stop() {
    local proton f pid
    if proton="$(proton_dir 2>/dev/null)" && [[ -d "$W/compatdata/pfx" ]]; then
        local ws
        for ws in "$proton/files/bin/wineserver" "$proton/dist/bin/wineserver"; do
            if [[ -x "$ws" ]]; then
                if dry; then echo "[dry] WINEPREFIX=$W/compatdata/pfx $ws -k"; else WINEPREFIX="$W/compatdata/pfx" "$ws" -k 2>/dev/null || true; fi
                break
            fi
        done
    fi
    for f in "$W"/run/*.pid; do
        [[ -f "$f" ]] || continue
        pid="$(cat "$f")"
        if kill -0 "$pid" 2>/dev/null; then
            if dry; then echo "[dry] kill -TERM -- -$pid"; else kill -TERM -- "-$pid" 2>/dev/null || true; fi
        fi
        dry || rm -f "$f"
    done
    if [[ -f "$W/run/sink.module" ]] && command -v pactl >/dev/null; then
        if dry; then echo "[dry] pactl unload-module $(cat "$W/run/sink.module")"; else
            XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}" pactl unload-module "$(cat "$W/run/sink.module")" 2>/dev/null || true
            rm -f "$W/run/sink.module"
        fi
    fi
    echo "stopped"
}

# fetchlog N: instance N's game log (ffx-rust.log or the hook's) on stdout.
cmd_fetchlog() {
    local n="${1:?instance}" name="${2:?ffx-rust.log or ffx-hook.log}"
    [[ "$name" =~ ^(ffx-rust\.log|ffx-hook\.log)$ ]] || die "fetchlog: $name"
    cat "$W/instances/$n/$name" 2>/dev/null || true
}

fn="${1:-}"
shift || true
case "$fn" in
    check) cmd_check ;;
    paths) cmd_paths ;;
    install) cmd_install ;;
    instance) cmd_instance "$@" ;;
    launch) cmd_launch "$@" ;;
    status) cmd_status ;;
    stop) cmd_stop ;;
    fetchlog) cmd_fetchlog "$@" ;;
    *) die "remote.sh: check, paths, install, instance, launch, status, stop or fetchlog" ;;
esac
