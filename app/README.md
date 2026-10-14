# FFX Studio

The desktop app: a launcher for the game and editors for everything the
`ffx` tools can change, for people who would rather not use a command line.

- `ui/`: the interface (Svelte 5, TypeScript, Vite).
- `src-tauri/`: the window (Tauri 2), a thin wrapper that passes every
  command to [`crates/ffx-studio`](../crates/ffx-studio), where all the
  logic lives (and its tests).

## Building

```bash
tools/build-app.sh      # → app/src-tauri/target/x86_64-pc-windows-msvc/release/ffx-studio.exe
```

It builds the mod loader (`DINPUT8.dll`, carried inside the app and
installed into the game folder by **Play**), the UI and the 64-bit Windows
app (cargo-xwin from WSL). The app needs the WebView2 runtime, which
Windows 10 and 11 have.

## Developing the UI

```bash
cargo run --release -p ffx-studio --example serve     # the commands on :8787
cd app/ui && pnpm dev                                  # the UI on :5173, /api proxied
```

In a browser the UI talks to the development server; in the window it
calls Tauri. Both reach the same `Studio::call`.
