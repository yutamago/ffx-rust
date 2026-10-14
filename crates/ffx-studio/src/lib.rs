//! The logic of FFX Studio, the desktop app: finding the game, mods, the
//! editors and the launcher. The app's window (Tauri) and the development
//! server only pass JSON commands to [`Studio::call`]; everything else is
//! here, so it can be tested without a window.
//!
//! Edits go into the *active mod*, a folder under `<game>/mods/` that the
//! ffx-hook proxy loads at start-up (`docs/hooking.md`). An editor reads a
//! file from the active mod if it has one, else from the game's archive,
//! so changes build on each other.

mod battles;
mod config;
mod dialogue;
mod files;
mod game;
mod kernel;
mod launch;
mod models;
mod mods;
mod monsters;
mod scripts;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub use config::Config;
pub use game::Game;

/// The app's state: its settings and the opened game.
pub struct Studio {
    config: Config,
    config_path: PathBuf,
    game: Option<Game>,
    /// The hook DLL the launcher installs as `DINPUT8.dll`, if the app
    /// carries one.
    hook_dll: Option<Vec<u8>>,
}

/// Reads argument `key` of a command.
fn arg<'a>(args: &'a Value, key: &str) -> Result<&'a Value> {
    args.get(key)
        .with_context(|| format!("missing argument `{key}`"))
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    arg(args, key)?
        .as_str()
        .with_context(|| format!("argument `{key}` must be a string"))
}

fn arg_u64(args: &Value, key: &str) -> Result<u64> {
    arg(args, key)?
        .as_u64()
        .with_context(|| format!("argument `{key}` must be a number"))
}

impl Studio {
    /// Starts with the settings stored at `config_path` (created when
    /// missing), opening the game if they name one.
    #[must_use]
    pub fn new(config_path: PathBuf, hook_dll: Option<Vec<u8>>) -> Self {
        let mut config = Config::load(&config_path);
        if config.game_dir.is_none() {
            config.game_dir = game::find_install();
        }
        let game = config
            .game_dir
            .as_ref()
            .and_then(|d| Game::open(d, &config.language).ok());
        Self {
            config,
            config_path,
            game,
            hook_dll,
        }
    }

    fn game(&self) -> Result<&Game> {
        self.game
            .as_ref()
            .context("no game folder yet: choose the folder that contains FFX.exe")
    }

    fn save_config(&self) -> Result<()> {
        self.config.save(&self.config_path)
    }

    /// The active mod's folder, created (with a first mod) if needed.
    fn active_mod(&mut self) -> Result<PathBuf> {
        let game = self.game()?;
        let id = match &self.config.active_mod {
            Some(id) if game.mods_dir().join(id).is_dir() => id.clone(),
            _ => {
                let id = mods::create(&game.mods_dir(), "My mod", "Made with FFX Studio")?;
                self.config.active_mod = Some(id.clone());
                self.save_config()?;
                id
            }
        };
        Ok(self.game()?.mods_dir().join(id))
    }

    /// Runs one command. The result is JSON for the UI; errors are
    /// messages for the user.
    ///
    /// # Errors
    /// Fails on unknown commands, bad arguments and failed operations.
    #[allow(clippy::too_many_lines)] // the command table
    pub fn call(&mut self, cmd: &str, args: &Value) -> Result<Value> {
        // Uploaded files (`data`, base64, with a `name`) become a file on
        // disk the command reads as `file`.
        let uploaded;
        let args = if let Some(data) = args.get("data").and_then(Value::as_str) {
            let name = args
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("upload.bin");
            let name = std::path::Path::new(name)
                .file_name()
                .map_or_else(|| "upload.bin".into(), |n| n.to_string_lossy().into_owned());
            let dir = std::env::temp_dir().join("ffx-studio-uploads");
            std::fs::create_dir_all(&dir)?;
            let file = dir.join(name);
            std::fs::write(&file, files::base64(data)?)?;
            let mut a = args.clone();
            a["file"] = json!(file.to_string_lossy());
            uploaded = a;
            &uploaded
        } else {
            args
        };
        match cmd {
            "status" => Ok(self.status()),
            "set_game_dir" => {
                let dir = PathBuf::from(arg_str(args, "path")?);
                let game = Game::open(&dir, &self.config.language)?;
                self.game = Some(game);
                self.config.game_dir = Some(dir);
                self.config.active_mod = None;
                self.save_config()?;
                Ok(self.status())
            }
            "set_language" => {
                let code = arg_str(args, "language")?.to_owned();
                ffx_text::Language::from_code(&code)
                    .with_context(|| format!("unknown language {code}"))?;
                self.config.language = code;
                if let Some(dir) = self.config.game_dir.clone() {
                    self.game = Some(Game::open(&dir, &self.config.language)?);
                }
                self.save_config()?;
                Ok(self.status())
            }
            "set_advanced" => {
                self.config.advanced = arg(args, "on")?.as_bool().unwrap_or(false);
                self.save_config()?;
                Ok(self.status())
            }
            "mod_create" => {
                let game = self.game()?;
                let id = mods::create(
                    &game.mods_dir(),
                    arg_str(args, "name")?,
                    args.get("description")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                )?;
                self.config.active_mod = Some(id);
                self.save_config()?;
                Ok(self.status())
            }
            "mod_select" => {
                let id = arg_str(args, "id")?;
                if !self.game()?.mods_dir().join(id).is_dir() {
                    bail!("there is no mod {id}");
                }
                self.config.active_mod = Some(id.to_owned());
                self.save_config()?;
                Ok(self.status())
            }
            "mod_update" => {
                let game = self.game()?;
                mods::update(&game.mods_dir(), arg_str(args, "id")?, args)?;
                Ok(self.status())
            }
            "mod_files" => {
                let game = self.game()?;
                Ok(json!(mods::files(
                    &game.mods_dir().join(arg_str(args, "id")?)
                )))
            }
            "mod_remove_file" => {
                let game = self.game()?;
                mods::remove_file(
                    &game.mods_dir().join(arg_str(args, "id")?),
                    arg_str(args, "path")?,
                )?;
                Ok(json!(mods::files(
                    &game.mods_dir().join(arg_str(args, "id")?)
                )))
            }
            "launch" => {
                let game = self.game()?;
                let with_mods = args.get("mods").and_then(Value::as_bool).unwrap_or(true);
                launch::launch(
                    game,
                    with_mods,
                    self.hook_dll.as_deref(),
                    &self.config.language,
                )?;
                Ok(json!({"started": true}))
            }
            "editors" => Ok(Self::editors()),
            // Kernel tables (items, abilities, equipment, treasure…).
            "table_rows" => kernel::rows(
                self.game()?,
                self.overlay()?.as_deref(),
                arg_str(args, "table")?,
            ),
            "table_record" => kernel::record(
                self.game()?,
                self.overlay()?.as_deref(),
                arg_str(args, "table")?,
                arg_u64(args, "id")?,
            ),
            "table_save" => {
                let dir = self.active_mod()?;
                kernel::save(
                    self.game()?,
                    &dir,
                    arg_str(args, "table")?,
                    arg_u64(args, "id")?,
                    arg(args, "values")?,
                )
            }
            "monster_rows" => monsters::rows(self.game()?, self.overlay()?.as_deref()),
            "monster_record" => monsters::record(
                self.game()?,
                self.overlay()?.as_deref(),
                arg_u64(args, "id")?,
            ),
            "monster_save" => {
                let dir = self.active_mod()?;
                monsters::save(
                    self.game()?,
                    &dir,
                    arg_u64(args, "id")?,
                    arg(args, "values")?,
                )
            }
            "encounter_fields" => battles::fields(self.game()?, self.overlay()?.as_deref()),
            "encounter_field" => battles::field(
                self.game()?,
                self.overlay()?.as_deref(),
                arg_u64(args, "field")?,
            ),
            "encounter_save_group" => {
                let dir = self.active_mod()?;
                battles::save_group(self.game()?, &dir, args)
            }
            "formation_save" => {
                let dir = self.active_mod()?;
                battles::save_formation(self.game()?, &dir, args)
            }
            "monster_names" => monsters::names(self.game()?, self.overlay()?.as_deref()),
            "dialogue_list" => Ok(dialogue::list(self.game()?)),
            "dialogue_get" => dialogue::get(
                self.game()?,
                self.overlay()?.as_deref(),
                arg_str(args, "name")?,
            ),
            "dialogue_save" => {
                let dir = self.active_mod()?;
                dialogue::save(
                    self.game()?,
                    &dir,
                    arg_str(args, "name")?,
                    arg(args, "messages")?,
                )
            }
            "model_list" => Ok(models::list(
                self.game()?,
                args.get("filter").and_then(Value::as_str).unwrap_or(""),
            )),
            "model_export" => models::export(
                self.game()?,
                self.overlay()?.as_deref(),
                &self.exports_dir()?,
                arg_str(args, "path")?,
            ),
            "model_import" => {
                let dir = self.active_mod()?;
                models::import(
                    self.game()?,
                    &dir,
                    arg_str(args, "path")?,
                    arg_str(args, "file")?,
                )
            }
            "texture_export" => models::export_texture(
                self.game()?,
                self.overlay()?.as_deref(),
                &self.exports_dir()?,
                arg_str(args, "path")?,
            ),
            "texture_import" => {
                let dir = self.active_mod()?;
                models::import_texture(
                    self.game()?,
                    &dir,
                    arg_str(args, "path")?,
                    arg_str(args, "file")?,
                )
            }
            "script_list" => Ok(scripts::list(
                self.game()?,
                args.get("filter").and_then(Value::as_str).unwrap_or(""),
            )),
            "script_get" => scripts::get(
                self.game()?,
                self.overlay()?.as_deref(),
                arg_str(args, "path")?,
            ),
            "script_save" => {
                let dir = self.active_mod()?;
                scripts::save(
                    self.game()?,
                    &dir,
                    arg_str(args, "path")?,
                    arg_str(args, "source")?,
                )
            }
            "file_list" => Ok(files::list(
                self.game()?,
                args.get("prefix").and_then(Value::as_str).unwrap_or(""),
            )),
            "file_extract" => files::extract(
                self.game()?,
                self.overlay()?.as_deref(),
                &self.exports_dir()?,
                arg_str(args, "path")?,
            ),
            "file_add" => {
                let dir = self.active_mod()?;
                files::add(&dir, arg_str(args, "path")?, arg_str(args, "file")?)
            }
            "open_folder" => {
                launch::open_folder(&PathBuf::from(arg_str(args, "path")?))?;
                Ok(json!({}))
            }
            other => bail!("unknown command {other}"),
        }
    }

    /// Where reads look first: the active mod's folder, if any.
    fn overlay(&self) -> Result<Option<PathBuf>> {
        let game = self.game()?;
        Ok(self
            .config
            .active_mod
            .as_ref()
            .map(|id| game.mods_dir().join(id))
            .filter(|d| d.is_dir()))
    }

    /// Where exports go: `<game>/studio-exports`, outside every mod.
    fn exports_dir(&self) -> Result<PathBuf> {
        let dir = self.game()?.dir.join("studio-exports");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn status(&self) -> Value {
        let mods = self
            .game
            .as_ref()
            .map(|g| mods::list(&g.mods_dir()))
            .unwrap_or_default();
        json!({
            "game_dir": self.config.game_dir,
            "game_ok": self.game.is_some(),
            "language": self.config.language,
            "languages": ffx_text::Language::ALL.iter().map(|l| json!({
                "code": l.code(),
                "name": format!("{l:?}"),
            })).collect::<Vec<_>>(),
            "advanced": self.config.advanced,
            "mods": mods,
            "active_mod": self.config.active_mod,
            "hook_bundled": self.hook_dll.is_some(),
            "hook_installed": self.game.as_ref().is_some_and(|g| g.dir.join("DINPUT8.dll").exists()),
            "running": self.game.as_ref().is_some_and(|_| launch::running()),
        })
    }

    fn editors() -> Value {
        json!([
            {"id": "items", "kind": "table", "table": "item", "group": "Items and abilities", "title": "Items", "icon": "potion", "simple": true,
             "about": "Names, descriptions and effects of usable items"},
            {"id": "commands", "kind": "table", "table": "command", "group": "Items and abilities", "title": "Abilities", "icon": "spark", "simple": true,
             "about": "Battle commands, skills, spells and special abilities"},
            {"id": "a_ability", "kind": "table", "table": "a_ability", "group": "Items and abilities", "title": "Auto-abilities", "icon": "gear", "simple": true,
             "about": "Abilities on weapons and armor"},
            {"id": "important", "kind": "table", "table": "important", "group": "Items and abilities", "title": "Key items", "icon": "key", "simple": true,
             "about": "Names and descriptions of key items"},
            {"id": "equipment", "kind": "table", "table": "weapon", "group": "Equipment and treasure", "title": "Starting equipment", "icon": "sword", "simple": true,
             "about": "Weapons and armor the party has at a new game"},
            {"id": "shop_arms", "kind": "table", "table": "shop_arms", "group": "Equipment and treasure", "title": "Shop equipment", "icon": "shop", "simple": true,
             "about": "Weapons and armor sold in shops"},
            {"id": "buki_get", "kind": "table", "table": "buki_get", "group": "Equipment and treasure", "title": "Found equipment", "icon": "sword", "simple": true,
             "about": "Weapons and armor found in chests"},
            {"id": "takara", "kind": "table", "table": "takara", "group": "Equipment and treasure", "title": "Treasure", "icon": "chest", "simple": true,
             "about": "What each chest and reward gives"},
            {"id": "monsters", "kind": "monsters", "group": "Battles", "title": "Monsters", "icon": "monster", "simple": true,
             "about": "HP, stats, drops, steal and bribe of every monster"},
            {"id": "encounters", "kind": "encounters", "group": "Battles", "title": "Encounters", "icon": "swords", "simple": true,
             "about": "Which monsters appear where, and how often"},
            {"id": "monmagic", "kind": "table", "table": "monmagic1", "group": "Battles", "title": "Monster abilities", "icon": "spark", "simple": false,
             "about": "Attacks and spells monsters use"},
            {"id": "characters", "kind": "table", "table": "ply_save", "group": "Characters", "title": "Characters", "icon": "person", "simple": false,
             "about": "Party members, aeons and guests"},
            {"id": "dialogue", "kind": "dialogue", "group": "Story", "title": "Dialogue", "icon": "chat", "simple": true,
             "about": "Every line of every scene and battle, in the chosen language"},
            {"id": "scripts", "kind": "scripts", "group": "Story", "title": "Event scripts", "icon": "code", "simple": false,
             "about": "The bytecode behind events and monster AI, as editable assembly"},
            {"id": "models", "kind": "models", "group": "Graphics", "title": "Models and textures", "icon": "cube", "simple": true,
             "about": "Export characters, monsters and maps to glTF and textures to PNG/DDS; import edits"},
            {"id": "files", "kind": "files", "group": "Advanced", "title": "Game files", "icon": "folder", "simple": false,
             "about": "Browse the archive, extract any file, add your own"},
        ])
    }
}
