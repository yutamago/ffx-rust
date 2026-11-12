#!/usr/bin/env bash
# Builds ffx-rust.exe (crate ffx-host) for 64-bit Windows, x86_64-pc-windows-msvc
# (docs/host.md, "Building for Windows"; the i686 build was dropped 2026-10-31).
# Usage: tools/build-host.sh [build|clippy] [extra cargo args]
# Features: FFX_HOST_FEATURES (default "swf": Iggy's Flash screens on Ruffle,
# composited through the D3D11 layer; docs/host.md, "Iggy"); empty for none.
# FFX_HOST_RUSTFLAGS: added to the target's rustflags after cargo-xwin's
# (tools/package-release.sh: --remap-path-prefix, so no build path ends up
# in the release binary); FFX_HOST_CFLAGS likewise to the C and C++ flags
# (Unicorn's __FILE__ paths: -ffile-prefix-map); CARGO_TARGET_DIR is honoured.
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
target=x86_64-pc-windows-msvc
if [[ -n "${FFX_HOST_TARGET:-}" && "$FFX_HOST_TARGET" != "$target" ]]; then
    echo "build-host.sh: ffx-rust.exe is built for $target only (FFX_HOST_TARGET=$FFX_HOST_TARGET)" >&2
    exit 2
fi
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
if ! command -v ml64 >/dev/null; then
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
# cargo-xwin's environment, its CRT cache set up under a lock (tools/xwin-env.sh)
eval "$(tools/xwin-env.sh "$target")"
if [[ -n "${FFX_HOST_RUSTFLAGS:-}" ]]; then
    export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="${CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS:-} $FFX_HOST_RUSTFLAGS"
fi
if [[ -n "${FFX_HOST_CFLAGS:-}" ]]; then
    export CFLAGS_x86_64_pc_windows_msvc="${CFLAGS_x86_64_pc_windows_msvc:-} $FFX_HOST_CFLAGS"
    export CXXFLAGS_x86_64_pc_windows_msvc="${CXXFLAGS_x86_64_pc_windows_msvc:-} $FFX_HOST_CFLAGS"
fi
features=(--features "${FFX_HOST_FEATURES-swf}")
[[ -z "${FFX_HOST_FEATURES-swf}" ]] && features=()
env "${toolchain_env[@]}" cargo "$cmd" --release --target "$target" -p ffx-host "${features[@]}" "$@"
[[ "$cmd" == build ]] && echo "built: ${CARGO_TARGET_DIR:-$repo/target}/$target/release/ffx-rust.exe"
exit 0
