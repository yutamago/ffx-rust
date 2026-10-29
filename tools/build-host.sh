#!/usr/bin/env bash
# Builds ffx-rust.exe (crate ffx-host) for 32-bit Windows (docs/host.md).
# Usage: tools/build-host.sh [build|clippy] [extra cargo args]
#
# Unicorn (unicorn-engine-sys) builds with cmake for the MSVC target, which
# needs three things cargo-xwin doesn't give it:
# - cmake and ninja: through uvx when not installed (~/.local/bin/cmake is a
#   broken pip wrapper on this machine);
# - llvm-lib, the toolchain's archiver: the versioned binary when only that
#   exists (llvm-lib-21);
# - a toolchain file without CMAKE_MSVC_RUNTIME_LIBRARY: unicorn's
#   CMakeLists refuses it next to the -MT that cc passes in CMAKE_C_FLAGS.
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
cmd=build
# FFX_HOST_TARGET=x86_64-pc-windows-msvc: the 64-bit program (docs/host.md,
# "Building for Windows")
target="${FFX_HOST_TARGET:-i686-pc-windows-msvc}"
if [[ "${1:-}" == build || "${1:-}" == clippy ]]; then
    cmd=$1
    shift
fi
cd "$repo"
shims="$repo/target/host-shims"
mkdir -p "$shims"
if ! cmake --version >/dev/null 2>&1; then
    printf '#!/bin/bash\nexec uvx --from cmake cmake "$@"\n' >"$shims/cmake"
    chmod +x "$shims/cmake"
fi
if ! command -v ninja >/dev/null; then
    printf '#!/bin/bash\nexec uvx --from ninja ninja "$@"\n' >"$shims/ninja"
    chmod +x "$shims/ninja"
fi
if ! command -v llvm-lib >/dev/null; then
    lib="$(compgen -c llvm-lib- | grep -E '^llvm-lib-[0-9]+$' | sort -t- -k3 -n | tail -1)"
    [[ -n "$lib" ]] && ln -sf "$(command -v "$lib")" "$shims/llvm-lib"
fi
# ml64 (MASM) for unicorn's x86_64 setjmp wrapper: LLVM's llvm-ml64
if [[ "$target" == x86_64-* ]] && ! command -v ml64 >/dev/null; then
    ml="$(compgen -c llvm-ml64- | grep -E '^llvm-ml64-[0-9]+$' | sort -t- -k3 -n | tail -1)"
    [[ -n "$ml" ]] && ln -sf "$(command -v "$ml")" "$shims/ml64"
fi
export PATH="$repo/tools/shims:$shims:$PATH"
export XWIN_ARCH=x86,x86_64
toolchain_env=()
xwin_toolchain="${XDG_CACHE_HOME:-$HOME/.cache}/cargo-xwin/cmake/clang-cl/$target-toolchain.cmake"
if [[ -f "$xwin_toolchain" ]]; then
    toolchain="$shims/$target-toolchain.cmake"
    grep -v CMAKE_MSVC_RUNTIME_LIBRARY "$xwin_toolchain" |
        sed "s|\${CMAKE_CURRENT_LIST_DIR}|$(dirname "$xwin_toolchain")|" >"$toolchain"
    toolchain_env=("CMAKE_TOOLCHAIN_FILE_$target=$toolchain")
fi
# One cargo-xwin at a time (tools/build-hook.sh).
mkdir -p "${XDG_CACHE_HOME:-$HOME/.cache}"
flock "${XDG_CACHE_HOME:-$HOME/.cache}/ffx-xwin.lock" \
    env "${toolchain_env[@]}" cargo xwin "$cmd" --release --target $target -p ffx-host "$@"
[[ "$cmd" == build ]] && echo "built: $repo/target/$target/release/ffx-rust.exe"
exit 0
