//! FFX Studio's window: the web UI (`app/ui`) calls one command, `call`,
//! which hands it to `ffx_studio::Studio`.
#![windows_subsystem = "windows"]

use std::sync::Mutex;

use ffx_studio::Studio;
use serde_json::Value;
use tauri::Manager;

/// The mod loader the launcher installs, built by `tools/build-hook.sh`
/// (empty when the app was built without it).
const HOOK: &[u8] = include_bytes!(env!("FFX_STUDIO_HOOK"));

#[tauri::command]
fn call(state: tauri::State<'_, Mutex<Studio>>, cmd: String, args: Value) -> Result<Value, String> {
    let mut studio = state.lock().map_err(|_| "busy".to_owned())?;
    studio.call(&cmd, &args).map_err(|e| format!("{e:#}"))
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let config = app.path().app_config_dir()?.join("config.json");
            let hook = (!HOOK.is_empty()).then(|| HOOK.to_vec());
            app.manage(Mutex::new(Studio::new(config, hook)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![call])
        .run(tauri::generate_context!())
        .expect("FFX Studio failed to start");
}
