#!/usr/bin/env bash
# Builds the player's package: ffx-rust (64-bit, crates/ffx-host), on
# Windows also ffx-launch.exe and Steam's steam_api64.dll, the licences and
# the player's README (docs/player.md), staged, checked and archived (M5
# stage 5; docs/host.md, "The release package"). The release workflow
# (.github/workflows/release.yml) runs it once per platform.
#
#   tools/package-release.sh [--target TRIPLE] [--dev] [--dry-run]
#                            [--prebuilt DIR] [--without-steam] [--out DIR]
#
# --target TRIPLE  the platform (default x86_64-pc-windows-msvc):
#              x86_64-pc-windows-msvc     ffx-rust-...-win64.zip
#              x86_64-unknown-linux-gnu   ffx-rust-...-linux-x64.tar.gz
#              aarch64-unknown-linux-gnu  ffx-rust-...-linux-arm64.tar.gz
#              aarch64-apple-darwin       ffx-rust-...-macos-arm64.tar.gz
#              Windows builds from Linux through cargo-xwin
#              (tools/build-host.sh) or natively on Windows (Git Bash, the
#              workflow); the others build natively only (on a machine of
#              that platform), else --prebuilt.
# --dev        a development package: ffx-rust with dev-unicorn (Unicorn,
#              GPL-2.0), archive named ...-dev-not-for-release, Unicorn's
#              licence inside. Never publish it as a release.
# --dry-run    no builds and no archive: stages into a temporary folder with
#              placeholder programs (or --prebuilt's), runs every check and
#              the checks' self-test, and removes the folder (FFX_KEEP_STAGE=1
#              keeps it).
# --prebuilt DIR  take the programs from DIR instead of building them (they
#              still go through the checks; build them with the same
#              --remap-path-prefix, or the check finds the build paths).
# --without-steam  a package without Steam's library (steam_api64.dll,
#              libsteam_api.so or libsteam_api.dylib; no Steamworks SDK here:
#              the game runs, achievements stay off).
# --out DIR    where the archive goes (default target/package/dist).
#
# FFX_RELEASE_FEATURES: ffx-host's features for the package (default "swf":
# Iggy's screens on Ruffle), built with --no-default-features. dev-unicorn in
# it is refused unless --dev; --dev adds it. Since WP9 (2026-11-12) ffx-host
# builds without dev-unicorn: the release's executor has no emulator, and a
# call to an address with no Rust side stops the run naming it (docs/host.md,
# "Without Unicorn").
# FFX_RELEASE_LABEL: replaces "<version>-<rev>" in the archive's name (the
# workflow passes the tag, v0.2.0).
# STEAMWORKS_SDK: the Steamworks SDK's sdk folder (default
# ~/tools/steamworks_sdk_165/sdk), for its redistributable_bin: win64/
# steam_api64.dll, linux64/ and linuxarm64/libsteam_api.so,
# osx/libsteam_api.dylib.
# Its licence (the Steamworks SDK Access Agreement) is the user's to accept;
# the DLL is never committed.
# PYTHON: the Python 3 to run the checks with (default python3).
#
# The builds go to target/package (CARGO_TARGET_DIR) with every build path
# remapped (--remap-path-prefix), so the development build in target/ stays
# as it is and no path of this machine ends up in the binaries; the check
# (tools/release-files.py check) fails the package if one does.
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
target=x86_64-pc-windows-msvc
dev=0 dry=0 prebuilt="" out="$repo/target/package/dist" steam=1
while [[ $# -gt 0 ]]; do
    case "$1" in
        --target) target="${2:?--target TRIPLE}"; shift ;;
        --dev) dev=1 ;;
        --dry-run) dry=1 ;;
        --prebuilt) prebuilt="$(cd "${2:?--prebuilt DIR}" && pwd)"; shift ;;
        --without-steam) steam=0 ;;
        --out) out="${2:?--out DIR}"; shift ;;
        -h|--help) sed -n '2,52p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "package-release.sh: unknown argument $1" >&2; exit 2 ;;
    esac
    shift
done
cd "$repo"
export CARGO_NET_OFFLINE=true
py="${PYTHON:-python3}"
case "$target" in
    x86_64-pc-windows-msvc) platform=win64 ;;
    x86_64-unknown-linux-gnu) platform=linux-x64 ;;
    aarch64-unknown-linux-gnu) platform=linux-arm64 ;;
    aarch64-apple-darwin) platform=macos-arm64 ;;
    *) echo "package-release.sh: no package for $target (see --help)" >&2; exit 2 ;;
esac
windows=0
[[ $platform == win64 ]] && windows=1
exe=""
[[ $windows == 1 ]] && exe=.exe
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
case "$platform" in
    win64) steam_lib=steam_api64.dll steam_dir=win64 steam_notice=steam_api64-NOTICE.txt ;;
    linux-x64) steam_lib=libsteam_api.so steam_dir=linux64 steam_notice=libsteam_api-NOTICE.txt ;;
    linux-arm64) steam_lib=libsteam_api.so steam_dir=linuxarm64 steam_notice=libsteam_api-NOTICE.txt ;;
    macos-arm64) steam_lib=libsteam_api.dylib steam_dir=osx steam_notice=libsteam_api-NOTICE.txt ;;
esac
steam_dll="$sdk/redistributable_bin/$steam_dir/$steam_lib"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
rev="$(git rev-parse --short=9 HEAD)"
git diff --quiet HEAD -- || rev="$rev-modified"
name="ffx-rust-${FFX_RELEASE_LABEL:-$version-$rev}-$platform"
[[ $dev == 1 ]] && name="$name-dev-not-for-release"
echo "package: $name (ffx-host features: ${features:-none}$([[ $steam == 0 ]] && echo '; without Steam')$([[ $dry == 1 ]] && echo '; dry run'))"

# the release dependency trees: no GPL-family package unless --dev
specs=("ffx-host:$target:$features")
[[ $windows == 1 ]] && specs+=("ffx-launch:$target")
if [[ $dev == 0 ]]; then
    crates=()
    for s in "${specs[@]}"; do crates+=(--crate "$s"); done
    "$py" -I tools/check-licences.py "${crates[@]}"
fi

# a path as the compiler sees it (Git Bash on Windows: C:\...)
native_path() { if command -v cygpath >/dev/null 2>&1; then cygpath -w "$1"; else printf '%s\n' "$1"; fi; }
bins="$repo/target/package/$target/release"
if [[ -n "$prebuilt" ]]; then
    bins="$prebuilt"
elif [[ $dry == 0 ]]; then
    host="$(rustc -vV | sed -n 's/^host: //p')"
    # every build path, remapped: the repo, cargo's sources, the home folder
    # (the last matching mapping wins, so the most specific goes last)
    h="$(native_path "$HOME")"
    remap=("--remap-path-prefix=$h=~"
        "--remap-path-prefix=$(native_path "${CARGO_HOME:-$HOME/.cargo}")=cargo"
        "--remap-path-prefix=$(native_path "${RUSTUP_HOME:-$HOME/.rustup}")=rustup"
        "--remap-path-prefix=$(native_path "$repo")=ffx-rust")
    export CARGO_TARGET_DIR="$repo/target/package"
    if [[ "$host" == "$target" ]]; then
        # a native build (the workflow's runners); the flags as one
        # unit-separated list, so no path is split at a space
        flags="$(printf '%s\x1f' "${remap[@]}")"
        export CARGO_ENCODED_RUSTFLAGS="${flags%$'\x1f'}"
        unset RUSTFLAGS
        feat=(--features "$features")
        [[ -z "$features" ]] && feat=()
        if [[ $windows == 1 ]]; then
            echo "building ffx-launch.exe"
            cargo build -q --release --locked --target "$target" -p ffx-launch
        fi
        echo "building ffx-rust$exe (features: ${features:-none})"
        cargo build -q --release --locked --target "$target" -p ffx-host --bin ffx-rust \
            --no-default-features "${feat[@]}"
    elif [[ $windows == 1 ]]; then
        # the same for C code's __FILE__ (Unicorn, in a --dev package), clang-cl's spelling
        cmap="/clang:-ffile-prefix-map=$HOME/.cargo=cargo /clang:-ffile-prefix-map=$repo=ffx-rust /clang:-ffile-prefix-map=$HOME=~"
        echo "building ffx-launch.exe"
        (
            eval "$(tools/xwin-env.sh "$target")"
            # cargo-xwin sets RUSTFLAGS (empty), which wins over the target's rustflags
            export RUSTFLAGS="${RUSTFLAGS:-} ${remap[*]}"
            cargo build -q --release --target "$target" -p ffx-launch
        )
        echo "building ffx-rust.exe (features: ${features:-none})"
        if ! FFX_HOST_FEATURES="$features" FFX_HOST_RUSTFLAGS="${remap[*]}" FFX_HOST_CFLAGS="$cmap" \
                nice tools/build-host.sh build --no-default-features; then
            exit 1
        fi
    else
        echo "package-release.sh: $target builds natively only (this is $host): build it on a $target machine, or pass --prebuilt" >&2
        exit 2
    fi
fi

stage_parent="$(mktemp -d "${TMPDIR:-/tmp}/ffx-package.XXXXXX")"
stage="$stage_parent/$name"
mkdir -p "$stage/licences"
cleanup() { [[ "${FFX_KEEP_STAGE:-}" == 1 ]] && echo "kept: $stage" || rm -rf "$stage_parent"; }
trap cleanup EXIT

programs=("ffx-rust$exe")
[[ $windows == 1 ]] && programs+=(ffx-launch.exe)
for p in "${programs[@]}"; do
    if [[ ($dry == 0 || -n "$prebuilt") && -f "$bins/$p" ]]; then
        cp "$bins/$p" "$stage/$p"
    elif [[ $dry == 1 && -z "$prebuilt" ]]; then
        echo "placeholder for $p (dry run: not built)" >"$stage/$p"
    else
        echo "package-release.sh: no $bins/$p" >&2; exit 1
    fi
    chmod +x "$stage/$p"
done
if [[ $steam == 1 ]]; then
    if [[ -f "$steam_dll" ]]; then
        cp "$steam_dll" "$stage/$steam_lib"
    elif [[ $dry == 1 ]]; then
        echo "placeholder for $steam_lib (dry run: no Steamworks SDK at STEAMWORKS_SDK)" >"$stage/$steam_lib"
    else
        echo "package-release.sh: no $steam_lib at $steam_dll (set STEAMWORKS_SDK to the SDK's sdk folder, or --without-steam)" >&2
        exit 1
    fi
    cp "tools/release/$steam_notice" "$stage/licences/"
fi
# the report names functions with it (ffx-rust looks beside itself)
cp symbols/ffx.sym "$stage/ffx.sym"
if [[ $windows == 1 ]]; then
    crlf() { sed 's/\r$//; s/$/\r/' "$1" >"$2"; }
    crlf docs/player.md "$stage/README.txt"
    crlf tools/release/play-ffx-rust.cmd "$stage/play-ffx-rust.cmd"
else
    sed 's/\r$//' docs/player.md >"$stage/README.txt"
fi

lic="$stage/licences"
cp LICENSE-MIT LICENSE-APACHE "$lic/"
cp third_party/zvidlib-vp8/LICENSE "$lic/zvidlib-vp8-LICENSE"
cp third_party/zvidlib-vp8/THIRD_PARTY_NOTICES.md "$lic/zvidlib-vp8-THIRD_PARTY_NOTICES.md"
cp crates/ffx-movie/NOTICE "$lic/ffx-movie-NOTICE"
ruffle="$("$py" -I tools/release-files.py pkgdir ruffle_core | tr -d '\r')"
cp "$ruffle/LICENSE.md" "$lic/ruffle-LICENSE.md"
gpl=()
if [[ $dev == 1 ]]; then
    gpl=(--allow-gpl)
    cp tools/release/unicorn-NOTICE.txt "$lic/"
    cp "$("$py" -I tools/release-files.py pkgdir unicorn-engine-sys | tr -d '\r')/COPYING" "$lic/unicorn-COPYING"
fi
"$py" -I tools/release-files.py notices "$lic/THIRD-PARTY.txt" "${specs[@]}" "${gpl[@]}"
(cd "$stage" && find . -type f ! -name SHA256SUMS.txt -printf '%P\n' | LC_ALL=C sort | xargs -d '\n' sha256sum) >"$stage/SHA256SUMS.txt"

# no game data, nothing of this machine: the Windows user name too
names=()
if [[ -n "${USERNAME:-}" ]]; then
    names+=(--name "$USERNAME")
elif grep -qi microsoft /proc/version 2>/dev/null && command -v cmd.exe >/dev/null 2>&1; then
    # WSL: the Windows side's user name
    w="$(cd /mnt/c 2>/dev/null && cmd.exe /c 'echo %USERNAME%' 2>/dev/null | tr -d '\r' || true)"
    [[ -n "$w" && "$w" != "%USERNAME%" ]] && names+=(--name "$w")
fi
checkflags=(--target "$target")
[[ $dev == 1 ]] && checkflags+=(--dev)
[[ $steam == 0 ]] && checkflags+=(--without-steam)
if [[ $dry == 1 ]]; then
    "$py" -I tools/release-files.py selftest "$stage" "${checkflags[@]}" "${names[@]}"
    echo "dry run: passed ($(find "$stage" -type f | wc -l) files staged, $(du -sh "$stage" | cut -f1))"
    exit 0
fi
"$py" -I tools/release-files.py check "$stage" "${checkflags[@]}" "${names[@]}"

mkdir -p "$out"
out="$(cd "$out" && pwd)"
if [[ $windows == 1 ]]; then
    archive="$out/$name.zip"
    rm -f "$archive"
    (cd "$stage_parent" && "$py" -I -m zipfile -c "$(native_path "$archive")" "$name")
else
    # tar keeps ffx-rust's executable bit
    archive="$out/$name.tar.gz"
    rm -f "$archive"
    (cd "$stage_parent" && "$py" -I -m tarfile -c "$archive" "$name")
fi
echo "package: $archive ($(du -h "$archive" | cut -f1))"
