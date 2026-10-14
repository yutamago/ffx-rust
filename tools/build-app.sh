#!/usr/bin/env bash
# Builds FFX Studio for Windows: the mod loader (32-bit DINPUT8.dll, carried
# inside the app), the web UI, and the 64-bit app.
# Output: app/src-tauri/target/x86_64-pc-windows-msvc/release/ffx-studio.exe
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
"$repo/tools/build-hook.sh" >/dev/null
(cd "$repo/app/ui" && pnpm install --frozen-lockfile >/dev/null && pnpm build >/dev/null)
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86_64
export FFX_STUDIO_HOOK="$repo/target/i686-pc-windows-msvc/release/dinput8.dll"
cd "$repo/app/src-tauri"
cargo xwin build --release --target x86_64-pc-windows-msvc "$@"
echo "built: $repo/app/src-tauri/target/x86_64-pc-windows-msvc/release/ffx-studio.exe"
