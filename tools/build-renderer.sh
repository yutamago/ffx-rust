#!/usr/bin/env bash
# Builds the 64-bit external renderer (ffx-render.exe) for Windows.
# Usage: tools/build-renderer.sh [extra cargo args]
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86,x86_64
cd "$repo"
cargo xwin build --release --target x86_64-pc-windows-msvc -p ffx-render "$@"
out="$repo/target/x86_64-pc-windows-msvc/release"
# DXC compiles the Direct3D 12 shaders far faster than the built-in FXC:
# when its release (github.com/microsoft/DirectXShaderCompiler) is
# unpacked in FFX_DXC_DIR, its DLLs go next to the renderer.
dxc="${FFX_DXC_DIR:-$HOME/tools/dxc}/bin/x64"
if [[ -f "$dxc/dxcompiler.dll" ]]; then
    install -m 755 "$dxc/dxcompiler.dll" "$dxc/dxil.dll" "$out/"
fi
echo "built: $out/ffx-render.exe"
