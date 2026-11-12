#!/usr/bin/env bash
# Builds the player's package: ffx-rust.exe (64-bit, crates/ffx-host) and
# ffx-launch.exe, Steam's steam_api64.dll, the licences and the player's
# README (docs/player.md), staged, checked and zipped (M5 stage 5;
# docs/host.md, "The release package").
#
#   tools/package-release.sh [--dev] [--dry-run] [--prebuilt DIR] [--out DIR]
#
# --dev        a development package: ffx-rust.exe with dev-unicorn (Unicorn,
#              GPL-2.0), zip named ...-dev-not-for-release.zip, Unicorn's
#              licence inside. Never publish it as a release.
# --dry-run    no builds and no zip: stages into a temporary folder with
#              placeholder programs (or --prebuilt's), runs every check and
#              the checks' self-test, and removes the folder (FFX_KEEP_STAGE=1
#              keeps it).
# --prebuilt DIR  take ffx-rust.exe and ffx-launch.exe from DIR instead of
#              building them (they still go through the checks).
# --out DIR    where the zip goes (default target/package/dist).
#
# FFX_RELEASE_FEATURES: ffx-host's features for the package (default "swf":
# Iggy's screens on Ruffle), built with --no-default-features. dev-unicorn in
# it is refused unless --dev; --dev adds it. Today (2026-11-12) ffx-host
# refuses to build without dev-unicorn (a compile_error!: its executor still
# needs Unicorn for code without a port; finish plan WP9 removes it), so the
# only package that builds is the --dev one.
# STEAMWORKS_SDK: the Steamworks SDK's sdk folder (default
# ~/tools/steamworks_sdk_165/sdk), for redistributable_bin/win64/steam_api64.dll.
# Its licence (the Steamworks SDK Access Agreement) is the user's to accept;
# the DLL is never committed.
#
# The builds go to target/package (CARGO_TARGET_DIR) with every build path
# remapped (--remap-path-prefix), so the development build in target/ stays
# as it is and no path of this machine ends up in the binaries; the check
# (tools/release-files.py check) fails the package if one does.
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
dev=0 dry=0 prebuilt="" out="$repo/target/package/dist"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dev) dev=1 ;;
        --dry-run) dry=1 ;;
        --prebuilt) prebuilt="$(cd "${2:?--prebuilt DIR}" && pwd)"; shift ;;
        --out) out="${2:?--out DIR}"; shift ;;
        -h|--help) sed -n '2,32p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "package-release.sh: unknown argument $1" >&2; exit 2 ;;
    esac
    shift
done
cd "$repo"
export CARGO_NET_OFFLINE=true
target=x86_64-pc-windows-msvc
features="${FFX_RELEASE_FEATURES-swf}"
has_unicorn() { [[ ",$features," == *,dev-unicorn,* ]]; }
if [[ $dev == 0 ]] && has_unicorn; then
    echo "package-release.sh: FFX_RELEASE_FEATURES has dev-unicorn (Unicorn, GPL-2.0): a release must not link it. Use --dev for a development package." >&2
    exit 2
fi
if [[ $dev == 1 ]] && ! has_unicorn; then
    features="dev-unicorn${features:+,$features}"
fi
sdk="${STEAMWORKS_SDK:-$HOME/tools/steamworks_sdk_165/sdk}"
steam_dll="$sdk/redistributable_bin/win64/steam_api64.dll"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
rev="$(git rev-parse --short=9 HEAD)"
git diff --quiet HEAD -- || rev="$rev-modified"
name="ffx-rust-$version-$rev-win64"
[[ $dev == 1 ]] && name="$name-dev-not-for-release"
echo "package: $name (ffx-host features: ${features:-none}$([[ $dry == 1 ]] && echo '; dry run'))"

# the release dependency trees: no GPL-family package unless --dev
specs=("ffx-host:$target:$features" "ffx-launch:$target")
if [[ $dev == 0 ]]; then
    python3 -I tools/check-licences.py --crate "${specs[0]}" --crate "${specs[1]}"
fi

bins="$repo/target/package/$target/release"
if [[ -n "$prebuilt" ]]; then
    bins="$prebuilt"
elif [[ $dry == 0 ]]; then
    # every build path, remapped: the repo, cargo's sources, the home folder
    # (the last matching mapping wins, so the most specific goes last)
    remap="--remap-path-prefix=$HOME=~ --remap-path-prefix=$HOME/.cargo=cargo --remap-path-prefix=$HOME/.rustup=rustup --remap-path-prefix=$repo=ffx-rust"
    # the same for C code's __FILE__ (Unicorn, in a --dev package), clang-cl's spelling
    cmap="/clang:-ffile-prefix-map=$HOME/.cargo=cargo /clang:-ffile-prefix-map=$repo=ffx-rust /clang:-ffile-prefix-map=$HOME=~"
    export CARGO_TARGET_DIR="$repo/target/package"
    echo "building ffx-launch.exe"
    (
        eval "$(tools/xwin-env.sh "$target")"
        export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="${CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS:-} $remap"
        cargo build -q --release --target "$target" -p ffx-launch
    )
    echo "building ffx-rust.exe (features: ${features:-none})"
    if ! FFX_HOST_FEATURES="$features" FFX_HOST_RUSTFLAGS="$remap" FFX_HOST_CFLAGS="$cmap" \
            nice tools/build-host.sh build --no-default-features; then
        if ! has_unicorn; then
            echo "package-release.sh: ffx-rust.exe does not build without dev-unicorn yet (the executor's compile_error!, finish plan WP9). Only a --dev package can be made today." >&2
        fi
        exit 1
    fi
fi

stage_parent="$(mktemp -d "${TMPDIR:-/tmp}/ffx-package.XXXXXX")"
stage="$stage_parent/$name"
mkdir -p "$stage/licences"
cleanup() { [[ "${FFX_KEEP_STAGE:-}" == 1 ]] && echo "kept: $stage" || rm -rf "$stage_parent"; }
trap cleanup EXIT

for exe in ffx-rust.exe ffx-launch.exe; do
    if [[ -f "$bins/$exe" ]]; then
        cp "$bins/$exe" "$stage/$exe"
    elif [[ $dry == 1 ]]; then
        echo "placeholder for $exe (dry run: not built)" >"$stage/$exe"
    else
        echo "package-release.sh: no $bins/$exe" >&2; exit 1
    fi
done
if [[ -f "$steam_dll" ]]; then
    cp "$steam_dll" "$stage/steam_api64.dll"
elif [[ $dry == 1 ]]; then
    echo "placeholder for steam_api64.dll (dry run: no Steamworks SDK at STEAMWORKS_SDK)" >"$stage/steam_api64.dll"
else
    echo "package-release.sh: no steam_api64.dll at $steam_dll (set STEAMWORKS_SDK to the SDK's sdk folder)" >&2
    exit 1
fi
# the report names functions with it (ffx-rust.exe looks beside itself)
cp symbols/ffx.sym "$stage/ffx.sym"
crlf() { sed 's/\r$//; s/$/\r/' "$1" >"$2"; }
crlf docs/player.md "$stage/README.txt"
crlf tools/release/play-ffx-rust.cmd "$stage/play-ffx-rust.cmd"

lic="$stage/licences"
cp LICENSE-MIT LICENSE-APACHE "$lic/"
cp third_party/zvidlib-vp8/LICENSE "$lic/zvidlib-vp8-LICENSE"
cp third_party/zvidlib-vp8/THIRD_PARTY_NOTICES.md "$lic/zvidlib-vp8-THIRD_PARTY_NOTICES.md"
cp crates/ffx-movie/NOTICE "$lic/ffx-movie-NOTICE"
ruffle="$(python3 -I tools/release-files.py pkgdir ruffle_core)"
cp "$ruffle/LICENSE.md" "$lic/ruffle-LICENSE.md"
cp tools/release/steam_api64-NOTICE.txt "$lic/"
gpl=()
if [[ $dev == 1 ]]; then
    gpl=(--allow-gpl)
    cp tools/release/unicorn-NOTICE.txt "$lic/"
    cp "$(python3 -I tools/release-files.py pkgdir unicorn-engine-sys)/COPYING" "$lic/unicorn-COPYING"
fi
python3 -I tools/release-files.py notices "$lic/THIRD-PARTY.txt" "${specs[@]}" "${gpl[@]}"
(cd "$stage" && find . -type f ! -name SHA256SUMS.txt -printf '%P\n' | LC_ALL=C sort | xargs -d '\n' sha256sum) >"$stage/SHA256SUMS.txt"

# no game data, nothing of this machine: the Windows user name too
names=()
if command -v cmd.exe >/dev/null 2>&1; then
    w="$(cd /mnt/c 2>/dev/null && cmd.exe /c 'echo %USERNAME%' 2>/dev/null | tr -d '\r' || true)"
    [[ -n "$w" && "$w" != "%USERNAME%" ]] && names+=(--name "$w")
fi
devflag=()
[[ $dev == 1 ]] && devflag=(--dev)
if [[ $dry == 1 ]]; then
    python3 -I tools/release-files.py selftest "$stage" "${devflag[@]}" "${names[@]}"
    echo "dry run: passed ($(find "$stage" -type f | wc -l) files staged, $(du -sh "$stage" | cut -f1))"
    exit 0
fi
python3 -I tools/release-files.py check "$stage" "${devflag[@]}" "${names[@]}"

mkdir -p "$out"
zip="$(cd "$out" && pwd)/$name.zip"
rm -f "$zip"
(cd "$stage_parent" && python3 -I -m zipfile -c "$zip" "$name")
echo "package: $zip ($(du -h "$zip" | cut -f1))"
