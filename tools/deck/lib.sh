# shellcheck shell=bash
# Shared by tools/deck/*.sh (docs/m6.md, "The Steam Deck"): the settings, SSH
# to the Deck, and tools/deck/remote.sh, the Deck's side of every script.
#
# DECK_DRY_RUN=1: nothing reaches the Deck; every SSH, rsync and port forward
#   is printed instead (and the Deck's answers are stand-ins).
# DECK_LOCAL=<folder>: the Deck's side runs on this machine with HOME=<folder>
#   (a stand-in Deck tree: tools/deck/deck-selftest.sh). With DECK_DRY_RUN=1
#   as well, remote.sh prints the commands it would start.

deck_repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
deck_tools="$deck_repo/tools/deck"

# Settings: tools/deck/deck.env, then the local file, then whatever the
# environment set before (it wins).
deck_load_env() {
    local saved
    # -g: re-declared inside this function, they must stay global
    saved="$(declare -px | grep -E '^declare -x DECK_[A-Z_]+=' | sed 's/^declare -x /declare -gx /' || true)"
    # shellcheck source=tools/deck/deck.env
    . "$deck_tools/deck.env"
    local own="${DECK_ENV:-$HOME/ffx-work/deck/deck.env}"
    if [[ -f "$own" ]]; then
        # shellcheck source=/dev/null
        . "$own"
    fi
    eval "$saved"
    export DECK_HOST DECK_USER DECK_SSH_PORT DECK_KEY DECK_WORK DECK_GAME_DIR DECK_PROTON \
        DECK_SLR DECK_DISPLAY DECK_SOUND DECK_NICE DECK_STEAM DECK_FIXED DECK_HARNESS_PORT \
        DECK_LOCAL_WORK
    mkdir -p "$DECK_LOCAL_WORK/ctl"
}

deck_dry() { [[ "${DECK_DRY_RUN:-}" == 1 ]]; }
deck_local() { [[ -n "${DECK_LOCAL:-}" ]]; }

# Stops unless a Deck is named (or the run is a dry or local one).
deck_need_host() {
    deck_local && return 0
    if [[ -z "$DECK_HOST" ]]; then
        if deck_dry; then
            DECK_HOST="<deck-ip>"
            return 0
        fi
        echo "no Deck: set DECK_HOST in ~/ffx-work/deck/deck.env (docs/m6.md, \"The Steam Deck\")" >&2
        exit 2
    fi
    if [[ ! -f "$DECK_KEY" ]] && ! deck_dry; then
        echo "no SSH key at $DECK_KEY" >&2
        exit 2
    fi
}

deck_target() { printf '%s@%s' "$DECK_USER" "$DECK_HOST"; }

# One master connection, reused by every call for 10 minutes; its socket is
# ours (not /tmp, which WSL empties). accept-new: the first connection
# records the Deck's host key, a changed key later is refused.
deck_ssh_opts=()
deck_set_ssh_opts() {
    deck_ssh_opts=(-i "$DECK_KEY" -p "$DECK_SSH_PORT" -o IdentitiesOnly=yes -o BatchMode=yes
        -o ConnectTimeout=10 -o ServerAliveInterval=30 -o StrictHostKeyChecking=accept-new
        -o ControlMaster=auto -o "ControlPath=$DECK_LOCAL_WORK/ctl/%C" -o ControlPersist=600)
}

# The settings remote.sh reads, as VAR=value words for the remote shell.
deck_remote_vars=(DECK_WORK DECK_GAME_DIR DECK_PROTON DECK_SLR DECK_DISPLAY DECK_SOUND DECK_NICE
    DECK_STEAM DECK_FIXED DECK_FORCE DECK_LINKS DECK_DRY_RUN)
deck_remote_env() {
    local v
    for v in "${deck_remote_vars[@]}"; do
        printf '%s=%q ' "$v" "${!v:-}"
    done
}

# deck_remote FUNCTION ARG...: runs remote.sh's FUNCTION on the Deck, its
# output on stdout. remote.sh goes over on stdin, so nothing of ours is
# installed outside DECK_WORK.
deck_remote() {
    if deck_local; then
        local v set=()
        for v in "${deck_remote_vars[@]}"; do set+=("$v=${!v:-}"); done
        env HOME="$DECK_LOCAL" "${set[@]}" DECK_LOCAL_TEST=1 bash "$deck_tools/remote.sh" "$@"
        return
    fi
    local cmd
    cmd="$(deck_remote_env)bash -s --$(printf ' %q' "$@")"
    if deck_dry; then
        echo "[dry] ssh $(deck_target): $(deck_remote_env)remote.sh$(printf ' %q' "$@")" >&2
        # stand-in answers for the callers that read one
        [[ "$1" == paths ]] && printf 'HOME=/home/%s\nWORK=/home/%s/%s\n' "$DECK_USER" "$DECK_USER" "$DECK_WORK"
        return 0
    fi
    deck_set_ssh_opts
    # shellcheck disable=SC2029 # $cmd is quoted for the Deck's shell on purpose
    ssh "${deck_ssh_opts[@]}" "$(deck_target)" "$cmd" < "$deck_tools/remote.sh"
}

# deck_ssh_run COMMAND: one shell command on the Deck (stdin passed on).
deck_ssh_run() {
    if deck_local; then
        (cd "$DECK_LOCAL" && HOME="$DECK_LOCAL" bash -c "$1")
        return
    fi
    deck_set_ssh_opts
    # shellcheck disable=SC2029 # quoted for the Deck's shell on purpose
    ssh "${deck_ssh_opts[@]}" "$(deck_target)" "$1"
}

# Our folder's paths only: anything the copies write on the Deck is below
# ~/$DECK_WORK.
deck_own_path() {
    if [[ "$1" == "$DECK_WORK"/* && "$1" != *..* ]]; then
        return 0
    fi
    echo "deck: refusing a path outside ~/$DECK_WORK: $1" >&2
    exit 2
}

# deck_put DIR REL: the Deck's ~/REL becomes a copy of DIR (what was there
# goes). tar over SSH: nothing beyond tar is needed on the Deck.
deck_put() {
    local src="$1" rel="$2" q
    deck_own_path "$rel"
    if deck_dry; then
        echo "[dry] copy $src/ to $(deck_target):~/$rel/ (replacing it)" >&2
        return 0
    fi
    q="\"\$HOME\"/$(printf '%q' "$rel")"
    tar -C "$src" -cf - . | deck_ssh_run "rm -rf $q && mkdir -p $q && tar -C $q -xf -"
}

# deck_get REL DIR: the Deck's ~/REL copied into DIR (added to it). A
# missing REL copies nothing.
deck_get() {
    local rel="$1" dst="$2" q
    deck_own_path "$rel"
    if deck_dry; then
        echo "[dry] copy $(deck_target):~/$rel/ to $dst/" >&2
        return 0
    fi
    mkdir -p "$dst"
    q="\"\$HOME\"/$(printf '%q' "$rel")"
    deck_ssh_run "if [ -d $q ]; then tar -C $q -cf - .; fi" < /dev/null | tar -C "$dst" -xf - 2>/dev/null || true
}

# deck_mkdir REL: makes ~/REL on the Deck (below our folder).
deck_mkdir() {
    deck_own_path "$1"
    if deck_dry; then
        echo "[dry] mkdir $(deck_target):~/$1" >&2
        return 0
    fi
    deck_ssh_run "mkdir -p \"\$HOME\"/$(printf '%q' "$1")" < /dev/null
}

# deck_forward PORT: this machine's 127.0.0.1:PORT reaches the Deck's
# 127.0.0.1:PORT (the harness listens there) through the master connection.
deck_forward() {
    local port="$1"
    if deck_local; then
        return 0
    fi
    if deck_dry; then
        echo "[dry] ssh -O forward -L 127.0.0.1:$port:127.0.0.1:$port $(deck_target)" >&2
        return 0
    fi
    deck_set_ssh_opts
    # the master first (forwards hang off it), then the forward; a forward
    # left from the run before is cancelled and made again
    ssh "${deck_ssh_opts[@]}" -O check "$(deck_target)" 2>/dev/null ||
        ssh "${deck_ssh_opts[@]}" -fN "$(deck_target)"
    ssh "${deck_ssh_opts[@]}" -O cancel -L "127.0.0.1:$port:127.0.0.1:$port" "$(deck_target)" 2>/dev/null || true
    ssh "${deck_ssh_opts[@]}" -O forward -L "127.0.0.1:$port:127.0.0.1:$port" "$(deck_target)"
}
