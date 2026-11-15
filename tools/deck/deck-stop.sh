#!/usr/bin/env bash
# Ends our games on the Steam Deck (docs/m6.md, "The Steam Deck").
#
# Usage: tools/deck/deck-stop.sh [--status]
#
# Ends what runs in our Proton prefix (its wineserver -k) and the process
# groups deck-run.sh started, removes our null sink and closes the SSH master
# connection with its port forwards. The user's games, Steam and app
# 359870's prefix are not touched. --status only lists our running games.
set -euo pipefail
# shellcheck source=tools/deck/lib.sh
. "$(dirname "$0")/lib.sh"
deck_load_env
deck_need_host
if [[ "${1:-}" == --status ]]; then
    deck_remote status
    exit 0
fi
deck_remote stop
if ! deck_local && ! deck_dry; then
    deck_set_ssh_opts
    ssh "${deck_ssh_opts[@]}" -O exit "$(deck_target)" 2>/dev/null || true
fi
