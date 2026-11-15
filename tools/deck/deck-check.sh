#!/usr/bin/env bash
# Is the Steam Deck ready for our runs? (docs/m6.md, "The Steam Deck")
#
# Usage: tools/deck/deck-check.sh
#
# Prints, from the Deck: SteamOS's version, free disk, memory, CPU and GPU,
# Game Mode or Desktop Mode (and the session's display), gamescope and its
# headless back end, whether the user plays right now, the sound tools, the
# Steam libraries (libraryfolders.vdf), the game's folder and build, the
# Protons under steamapps/common and compatibilitytools.d, which one Steam
# uses for app 359870 and which one we use, and our own folder on the Deck.
# Reads only; changes nothing on the Deck.
# Exit: 0 reachable and checked, 2 no Deck set up here, 3 not reachable.
set -euo pipefail
# shellcheck source=tools/deck/lib.sh
. "$(dirname "$0")/lib.sh"
deck_load_env

echo "== this machine"
echo "deck: ${DECK_USER}@${DECK_HOST:-<not set>}:${DECK_SSH_PORT}, key $DECK_KEY ($([[ -f "$DECK_KEY" ]] && echo present || echo MISSING))"
echo "settings: $deck_tools/deck.env, then ${DECK_ENV:-$HOME/ffx-work/deck/deck.env} ($([[ -f "${DECK_ENV:-$HOME/ffx-work/deck/deck.env}" ]] && echo present || echo absent))"
deck_need_host
if ! deck_local && ! deck_dry; then
    deck_set_ssh_opts
    if ! ssh "${deck_ssh_opts[@]}" "$(deck_target)" true < /dev/null; then
        echo "the Deck does not answer over SSH: is it awake, on the network, in Desktop Mode with sshd running, and is the key authorised? (docs/m6.md)" >&2
        exit 3
    fi
fi
deck_remote check
