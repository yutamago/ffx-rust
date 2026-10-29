#!/usr/bin/env bash
# Builds the 64-bit external renderer (ffx-render.exe) for Windows.
# Usage: tools/build-renderer.sh [extra cargo args]
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86,x86_64
cd "$repo"
# cargo-xwin's environment (tools/xwin-env.sh, as tools/build-hook.sh)
mkdir -p "${XDG_CACHE_HOME:-$HOME/.cache}"
eval "$(tools/xwin-env.sh x86_64-pc-windows-msvc)"  # CRT cache set up under a lock
cargo build --release --target x86_64-pc-windows-msvc -p ffx-render "$@"
out="$repo/target/x86_64-pc-windows-msvc/release"
# DXC compiles the Direct3D 12 shaders far faster than the built-in FXC:
# when its release (github.com/microsoft/DirectXShaderCompiler) is
# unpacked in FFX_DXC_DIR, its DLLs go next to the renderer.
dxc="${FFX_DXC_DIR:-$HOME/tools/dxc}/bin/x64"
if [[ -f "$dxc/dxcompiler.dll" ]]; then
    install -m 755 "$dxc/dxcompiler.dll" "$dxc/dxil.dll" "$out/"
fi
# AMD FidelityFX (FSR 3, MIT licence): the SDK's signed DLLs from
# FFX_SDK_DIR (github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK, v2.0.0:
# Kits/FidelityFX/signedbin) go next to the renderer, with the licence.
ffx="${FFX_SDK_DIR:-$HOME/tools/ffx-sdk}"
if [[ -f "$ffx/Kits/FidelityFX/signedbin/amd_fidelityfx_loader_dx12.dll" ]]; then
    install -m 755 "$ffx/Kits/FidelityFX/signedbin/amd_fidelityfx_loader_dx12.dll" \
        "$ffx/Kits/FidelityFX/signedbin/amd_fidelityfx_upscaler_dx12.dll" "$out/"
    install -m 644 "$ffx/docs/license.md" "$out/LICENSE-FidelityFX.txt"
fi
echo "built: $out/ffx-render.exe"
