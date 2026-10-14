#!/usr/bin/env bash
# Runs the in-game tests (crates/ffx-harness/tests/game.rs) that the changed
# files concern. Each test starts FFX.exe in the background (no focus taken).
#
# Usage: tools/game-tests.sh [--all] [--list] [BASE]
#   BASE    compare with this commit (default: the merge base with origin/main);
#           uncommitted and untracked changes always count
#   --all   run every game test
#   --list  only print which tests would run, and why
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"

all=0 list=0 base=""
for arg in "$@"; do
    case "$arg" in
        --all) all=1 ;;
        --list) list=1 ;;
        -*) echo "unknown option $arg" >&2; exit 2 ;;
        *) base="$arg" ;;
    esac
done

# Each test and the paths that concern it.
start_up=start_up_input_and_kernel_mod
first_scene=first_scene_with_script_model_and_motion_mods
save=save_and_battle_with_monster_mod
seed=random_generators_repeat_with_a_seed
everything="$start_up $first_scene $save $seed"

tests_for() {
    case "$1" in
        # The hook, the harness and what every test uses.
        crates/ffx-hook/*|crates/ffx-dinput8/*|crates/ffx-harness/*|crates/ffx-core/*\
        |crates/ffx-vbf/*|tools/run-game.sh|tools/build-hook.sh|tools/shims/*\
        |tools/game-tests.sh|Cargo.toml|Cargo.lock)
            echo "$everything" ;;
        crates/ffx-kernel/*|crates/ffx-text/*) echo "$start_up" ;;
        crates/ffx-script/*|crates/ffx-phyre/*|crates/ffx-motion/*) echo "$first_scene" ;;
        crates/ffx-monster/*) echo "$save" ;;
        *) ;;
    esac
}

if [[ $all == 1 ]]; then
    selected="$everything"
    [[ $list == 1 ]] && echo "all game tests (--all)"
else
    if [[ -z "$base" ]]; then
        base="$(git merge-base HEAD origin/main 2>/dev/null || git rev-parse HEAD)"
    fi
    changed="$( (git diff --name-only "$base"; git diff --name-only; git diff --name-only --cached;
                 git ls-files --others --exclude-standard) | sort -u)"
    selected=""
    while IFS= read -r file; do
        [[ -z "$file" ]] && continue
        for test in $(tests_for "$file"); do
            if [[ " $selected " != *" $test "* ]]; then
                selected="$selected $test"
                [[ $list == 1 ]] && echo "$test  (because of $file)"
            fi
        done
    done <<< "$changed"
fi

if [[ -z "${selected// /}" ]]; then
    echo "no game tests concern the changed files"
    exit 0
fi
[[ $list == 1 ]] && exit 0

: "${FFX_GAME_DIR:=$HOME/steam-games/FFX-X2-HD}"
export FFX_GAME_DIR
# Test names are filters; --exact keeps one from matching another.
# shellcheck disable=SC2086
exec cargo test --release -p ffx-harness --test game -- --ignored --test-threads 1 --exact $selected
