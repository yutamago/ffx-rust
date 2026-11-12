//! What `ffx-launch` starts, decided without touching Windows: the
//! arguments, the game folder, the files it needs, `ffx-rust.exe`'s
//! command line and environment, and the messages a player sees.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

/// Steam's app id of FINAL FANTASY X/X-2 HD Remaster. Steam sets
/// `SteamAppId` for a game it starts; started any other way,
/// `steam_api64.dll` needs it in the environment (docs/host.md, "Steam").
pub const STEAM_APP_ID: &str = "359870";

/// `ffx-rust.exe`'s run budget in seconds. Without `--budget` it stops
/// after 60 s (a development default); a player's run is not meant to
/// end on a timer: one year.
pub const PLAY_BUDGET: &str = "31536000";

/// The program's file name, beside `ffx-launch.exe`.
pub const RUST_EXE: &str = "ffx-rust.exe";

/// Steam's 64-bit library, beside `ffx-rust.exe` (or `FFX_STEAM_API`).
pub const STEAM_DLL: &str = "steam_api64.dll";

/// `ffx-rust.exe`'s report, beside it.
pub const LOG: &str = "ffx-rust.log";

/// The report of the run before, kept for a problem report.
pub const PREVIOUS_LOG: &str = "ffx-rust.previous.log";

/// What `ffx-rust.exe` reads from the game folder, each with the spellings
/// the game ships (`ffx-host` tries both for the runtime DLLs). The two
/// runtime DLLs are needed while the development build runs them
/// (docs/host.md, "The C runtime DLLs"); they leave this list when the C
/// runtime is Rust (finish plan WP5, WP9).
pub const NEEDED: &[&[&str]] = &[
    &["FFX.exe"],
    &["data/FFX_Data.vbf"],
    &["magicFiles/FFX"],
    &["MSVCR110.dll", "msvcr110.dll"],
    &["MSVCP110.dll", "msvcp110.dll"],
];

/// Which program to start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// `FFX.exe` with bottom-up ASLR off (the default: the test tools'
    /// launcher, `tools/run-game.sh --fixed-addresses`).
    Original,
    /// `ffx-rust.exe` on the player's game folder.
    Rust,
}

/// The parsed command line.
#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub mode: Mode,
    /// The first argument that is no option: `FFX.exe`'s path for
    /// [`Mode::Original`]; for [`Mode::Rust`] the game folder or any file
    /// in it (Steam's `%command%` is the game's launcher).
    pub game: Option<PathBuf>,
    /// Arguments after it (Steam's own, from `%command%`): ignored.
    pub ignored: Vec<OsString>,
    /// `--no-dialog`: messages on stderr only, no message box.
    pub no_dialog: bool,
}

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Launch(Args),
    Help,
}

pub const USAGE: &str = "\
ffx-launch [--rust | --original] [--no-dialog] [GAME]

  (no option)  starts FFX.exe (GAME, default FFX.exe in the current
               folder) with bottom-up ASLR off and prints its process ID
  --rust       starts ffx-rust.exe (beside ffx-launch.exe) on the game:
               GAME is the game folder or any file in it (FFX.exe, the
               Steam launcher); without it the current folder, then the
               folder of ffx-launch.exe
  --original   the same as no option
  --no-dialog  problems on the console only, no message box

As a Steam launch option: \"<folder>\\ffx-launch.exe\" --rust %command%";

/// Parses the arguments after the program name.
///
/// # Errors
/// An option this program doesn't know.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, Problem> {
    let mut mode = Mode::Original;
    let mut game = None;
    let mut ignored = Vec::new();
    let mut no_dialog = false;
    for a in args {
        if game.is_some() {
            ignored.push(a);
            continue;
        }
        match a.to_str() {
            Some("--rust") => mode = Mode::Rust,
            Some("--original") => mode = Mode::Original,
            Some("--no-dialog") => no_dialog = true,
            Some("--help" | "-h" | "/?") => return Ok(Command::Help),
            Some(s) if s.starts_with("--") => return Err(Problem::UnknownOption(s.into())),
            _ => game = Some(PathBuf::from(a)),
        }
    }
    Ok(Command::Launch(Args {
        mode,
        game,
        ignored,
        no_dialog,
    }))
}

/// Whether `dir` holds `FFX.exe`.
fn has_game(dir: &Path) -> bool {
    dir.join("FFX.exe").is_file()
}

/// The game folder: `arg` (a folder, or a file in it), else the current
/// folder when it holds `FFX.exe`, else the launcher's folder when it does.
///
/// # Errors
/// `arg` names nothing, or neither folder holds the game.
pub fn game_folder(
    arg: Option<&Path>,
    cwd: &Path,
    launcher_dir: Option<&Path>,
) -> Result<PathBuf, Problem> {
    if let Some(a) = arg {
        let a = if a.is_absolute() {
            a.to_path_buf()
        } else {
            cwd.join(a)
        };
        if a.is_dir() {
            return Ok(a);
        }
        if a.is_file() {
            return Ok(a
                .parent()
                .map_or_else(|| cwd.to_path_buf(), Path::to_path_buf));
        }
        return Err(Problem::NotFound(a));
    }
    if has_game(cwd) {
        return Ok(cwd.to_path_buf());
    }
    if let Some(d) = launcher_dir
        && has_game(d)
    {
        return Ok(d.to_path_buf());
    }
    Err(Problem::NoGameFolder)
}

/// The first of [`NEEDED`] missing from `game`.
///
/// # Errors
/// [`Problem::MissingGameFile`] naming it.
pub fn check_game(game: &Path) -> Result<(), Problem> {
    for spellings in NEEDED {
        if !spellings.iter().any(|f| game.join(f).exists()) {
            return Err(Problem::MissingGameFile {
                folder: game.to_path_buf(),
                file: spellings[0].replace('/', "\\"),
            });
        }
    }
    Ok(())
}

/// How to start `ffx-rust.exe`.
#[derive(Debug, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    /// The game folder, as Steam starts `FFX.exe` in it.
    pub cwd: PathBuf,
    /// Set in the child's environment (on top of the launcher's).
    pub env: Vec<(OsString, OsString)>,
    /// stdout and stderr go here.
    pub log: PathBuf,
    /// Not in the way of a start, but the player should know.
    pub warnings: Vec<Problem>,
}

/// `ffx-rust.exe`'s command line on `game`: the game folder, the documents
/// folder `FFX.exe` would use (`SHGetFolderPathW(CSIDL_PERSONAL)`: the
/// settings and saves under `Square Enix\FINAL FANTASY X&X-2 HD
/// Remaster`, shared with the original), no time limit; `SteamAppId`
/// when Steam didn't start us. `env` reads the launcher's environment.
///
/// # Errors
/// No `ffx-rust.exe` beside the launcher, no documents folder, or a game
/// file missing.
pub fn rust_launch(
    game: &Path,
    launcher_dir: &Path,
    documents: Option<&Path>,
    env: impl Fn(&str) -> Option<OsString>,
) -> Result<Launch, Problem> {
    let program = launcher_dir.join(RUST_EXE);
    if !program.is_file() {
        return Err(Problem::NoRustExe(launcher_dir.to_path_buf()));
    }
    check_game(game)?;
    let documents = documents.ok_or(Problem::NoDocuments)?;
    let set = |k: &str| env(k).is_some_and(|v| !v.is_empty());
    let mut vars = Vec::new();
    if !set("SteamAppId") {
        vars.push(("SteamAppId".into(), STEAM_APP_ID.into()));
    }
    let steam_dll = env("FFX_STEAM_API")
        .filter(|v| !v.is_empty())
        .map_or_else(|| launcher_dir.join(STEAM_DLL), PathBuf::from);
    let mut warnings = Vec::new();
    if set("FFX_NO_STEAM") {
        // asked for: no note
    } else if !steam_dll.is_file() {
        warnings.push(Problem::NoSteamDll(launcher_dir.to_path_buf()));
    }
    let arg = |s: &str| OsString::from(s);
    Ok(Launch {
        args: vec![
            arg("--game-dir"),
            game.as_os_str().to_owned(),
            arg("--documents"),
            documents.as_os_str().to_owned(),
            arg("--budget"),
            arg(PLAY_BUDGET),
        ],
        program,
        cwd: game.to_path_buf(),
        env: vars,
        log: launcher_dir.join(LOG),
        warnings,
    })
}

/// Something in the way, in the player's words.
#[derive(Debug, PartialEq, Eq)]
pub enum Problem {
    UnknownOption(String),
    /// The path given on the command line does not exist.
    NotFound(PathBuf),
    /// No path given and no `FFX.exe` here or beside the launcher.
    NoGameFolder,
    MissingGameFile {
        folder: PathBuf,
        file: String,
    },
    /// No `ffx-rust.exe` in this folder.
    NoRustExe(PathBuf),
    NoDocuments,
    /// No `steam_api64.dll` in this folder: Steam is off, the game runs.
    NoSteamDll(PathBuf),
    /// Starting the program failed.
    Start(PathBuf, String),
}

impl Problem {
    /// The message box's title.
    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Problem::NoSteamDll(_) => "ffx-rust: Steam is off",
            _ => "ffx-rust cannot start",
        }
    }

    /// A warning: the game was started anyway.
    #[must_use]
    pub fn is_warning(&self) -> bool {
        matches!(self, Problem::NoSteamDll(_))
    }
}

const POINT_AT_GAME: &str = "Put ffx-launch.exe in the game folder (the folder with FFX.exe), \
or drag the game folder onto ffx-launch.exe, or start it with the folder: \
ffx-launch.exe --rust \"D:\\SteamLibrary\\steamapps\\common\\FINAL FANTASY FFX&FFX-2 HD Remaster\". \
In Steam: right-click the game, Properties, Installed Files, Browse shows the folder.";

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::UnknownOption(o) => write!(f, "Unknown option {o}.\n\n{USAGE}"),
            Problem::NotFound(p) => {
                write!(f, "There is nothing at {}.\n\n{POINT_AT_GAME}", p.display())
            }
            Problem::NoGameFolder => write!(
                f,
                "The game folder was not found: there is no FFX.exe in this folder or next to \
                 ffx-launch.exe.\n\n{POINT_AT_GAME}"
            ),
            Problem::MissingGameFile { folder, file } => write!(
                f,
                "{file} is missing from the game folder {}.\n\nIs this the folder of FINAL \
                 FANTASY X/X-2 HD Remaster? If it is, let Steam repair it: right-click the game, \
                 Properties, Installed Files, Verify integrity of game files.\n\n{POINT_AT_GAME}",
                folder.display()
            ),
            Problem::NoRustExe(dir) => write!(
                f,
                "{RUST_EXE} is missing from {}. Unpack the whole download again: \
                 ffx-launch.exe and {RUST_EXE} belong in the same folder.",
                dir.display()
            ),
            Problem::NoDocuments => write!(
                f,
                "Windows did not say where your Documents folder is, so the game would have \
                 nowhere to keep its settings and saves."
            ),
            Problem::NoSteamDll(dir) => write!(
                f,
                "{STEAM_DLL} is missing from {}, so Steam is off: no achievements in this run. \
                 The game runs anyway. Unpack the whole download again to get it back.",
                dir.display()
            ),
            Problem::Start(p, e) => write!(f, "Windows could not start {}: {e}", p.display()),
        }
    }
}

/// `--rust`'s check of a game argument that is no path at all (an empty
/// one, as a broken shortcut gives): treated as none.
#[must_use]
pub fn game_arg(args: &Args) -> Option<&Path> {
    args.game.as_deref().filter(|p| !p.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn os(v: &[&str]) -> Vec<OsString> {
        v.iter().map(OsString::from).collect()
    }

    fn launch(v: &[&str]) -> Args {
        match parse(os(v)).expect("parses") {
            Command::Launch(a) => a,
            Command::Help => panic!("help"),
        }
    }

    /// A temporary folder, removed at the end.
    struct Tmp(PathBuf);
    impl Tmp {
        fn new(name: &str) -> Tmp {
            let p = std::env::temp_dir().join(format!(
                "ffx-launch-{name}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).expect("tmp");
            Tmp(p)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn touch(p: &Path) {
        fs::create_dir_all(p.parent().expect("parent")).expect("dir");
        fs::write(p, b"x").expect("write");
    }

    /// A game folder with every file `ffx-rust.exe` needs.
    fn game(dir: &Path) {
        touch(&dir.join("FFX.exe"));
        touch(&dir.join("data/FFX_Data.vbf"));
        fs::create_dir_all(dir.join("magicFiles/FFX")).expect("dir");
        touch(&dir.join("MSVCR110.dll"));
        touch(&dir.join("msvcp110.dll"));
    }

    #[test]
    fn default_is_the_original_as_today() {
        let a = launch(&["FFX.exe"]);
        assert_eq!(a.mode, Mode::Original);
        assert_eq!(a.game, Some(PathBuf::from("FFX.exe")));
        assert!(!a.no_dialog);
        assert_eq!(launch(&[]).game, None);
        assert_eq!(launch(&["--original"]).mode, Mode::Original);
    }

    #[test]
    fn rust_and_steam_command() {
        // Steam's launch option: `ffx-launch.exe --rust %command%`
        let a = launch(&[
            "--rust",
            "C:\\Games\\FFX\\FFX&X-2_LAUNCHER.exe",
            "-steam",
            "--x",
        ]);
        assert_eq!(a.mode, Mode::Rust);
        assert_eq!(
            a.game,
            Some(PathBuf::from("C:\\Games\\FFX\\FFX&X-2_LAUNCHER.exe"))
        );
        assert_eq!(a.ignored, os(&["-steam", "--x"]));
        assert!(launch(&["--no-dialog", "--rust"]).no_dialog);
        assert_eq!(launch(&["--rust", "--original"]).mode, Mode::Original);
    }

    #[test]
    fn help_and_unknown_options() {
        assert_eq!(parse(os(&["--help"])), Ok(Command::Help));
        assert_eq!(parse(os(&["/?"])), Ok(Command::Help));
        let e = parse(os(&["--rusty"])).expect_err("unknown");
        assert_eq!(e, Problem::UnknownOption("--rusty".into()));
        assert!(e.to_string().contains("--rust"));
    }

    #[test]
    fn empty_game_argument_is_none() {
        let a = launch(&["--rust", ""]);
        assert_eq!(game_arg(&a), None);
        let a = launch(&["--rust", "x"]);
        assert_eq!(game_arg(&a), Some(Path::new("x")));
    }

    #[test]
    fn game_folder_search_order() {
        let t = Tmp::new("order");
        let cwd = t.0.join("cwd");
        let beside = t.0.join("launcher");
        fs::create_dir_all(&cwd).expect("dir");
        fs::create_dir_all(&beside).expect("dir");
        assert_eq!(
            game_folder(None, &cwd, Some(&beside)),
            Err(Problem::NoGameFolder)
        );
        assert_eq!(game_folder(None, &cwd, None), Err(Problem::NoGameFolder));
        touch(&beside.join("FFX.exe"));
        assert_eq!(game_folder(None, &cwd, Some(&beside)), Ok(beside.clone()));
        touch(&cwd.join("FFX.exe"));
        assert_eq!(game_folder(None, &cwd, Some(&beside)), Ok(cwd.clone()));
    }

    #[test]
    fn game_folder_from_an_argument() {
        let t = Tmp::new("arg");
        let g = t.0.join("FINAL FANTASY FFX&FFX-2 HD Remaster");
        game(&g);
        touch(&g.join("FFX&X-2_LAUNCHER.exe"));
        let other = t.0.join("elsewhere");
        fs::create_dir_all(&other).expect("dir");
        // a folder, a file in it, relative to the current folder
        assert_eq!(game_folder(Some(&g), &other, None), Ok(g.clone()));
        assert_eq!(
            game_folder(Some(&g.join("FFX&X-2_LAUNCHER.exe")), &other, None),
            Ok(g.clone())
        );
        assert_eq!(
            game_folder(
                Some(Path::new("FINAL FANTASY FFX&FFX-2 HD Remaster")),
                &t.0,
                None
            ),
            Ok(t.0.join("FINAL FANTASY FFX&FFX-2 HD Remaster"))
        );
        let missing = t.0.join("nope");
        assert_eq!(
            game_folder(Some(&missing), &other, None),
            Err(Problem::NotFound(missing.clone()))
        );
    }

    #[test]
    fn each_needed_file_is_checked() {
        let t = Tmp::new("needed");
        game(&t.0);
        assert_eq!(check_game(&t.0), Ok(()));
        for (path, shown) in [
            ("FFX.exe", "FFX.exe"),
            ("data/FFX_Data.vbf", "data\\FFX_Data.vbf"),
            ("MSVCR110.dll", "MSVCR110.dll"),
            ("msvcp110.dll", "MSVCP110.dll"),
        ] {
            let p = t.0.join(path);
            fs::remove_file(&p).expect("rm");
            let e = check_game(&t.0).expect_err(path);
            assert_eq!(
                e,
                Problem::MissingGameFile {
                    folder: t.0.clone(),
                    file: shown.into()
                }
            );
            assert!(e.to_string().contains(shown), "{e}");
            touch(&p);
        }
        fs::remove_dir_all(t.0.join("magicFiles")).expect("rm");
        assert!(matches!(
            check_game(&t.0),
            Err(Problem::MissingGameFile { file, .. }) if file == "magicFiles\\FFX"
        ));
    }

    fn no_env(_: &str) -> Option<OsString> {
        None
    }

    #[test]
    fn rust_launch_command_line() {
        let t = Tmp::new("launch");
        let g = t.0.join("game");
        game(&g);
        let l = t.0.join("ffx-rust");
        touch(&l.join(RUST_EXE));
        touch(&l.join(STEAM_DLL));
        let docs = t.0.join("Documents");
        let r = rust_launch(&g, &l, Some(&docs), no_env).expect("launch");
        assert_eq!(r.program, l.join(RUST_EXE));
        assert_eq!(
            r.args,
            vec![
                OsString::from("--game-dir"),
                g.clone().into_os_string(),
                "--documents".into(),
                docs.clone().into_os_string(),
                "--budget".into(),
                PLAY_BUDGET.into(),
            ]
        );
        assert_eq!(r.cwd, g);
        assert_eq!(r.log, l.join(LOG));
        assert_eq!(r.env, vec![("SteamAppId".into(), STEAM_APP_ID.into())]);
        assert_eq!(r.warnings, Vec::new());
        // the budget parses as ffx-rust's does
        assert!(PLAY_BUDGET.parse::<f64>().is_ok_and(|s| s >= 86_400.0));
    }

    #[test]
    fn steam_started_us() {
        let t = Tmp::new("steam");
        let g = t.0.join("game");
        game(&g);
        touch(&t.0.join(RUST_EXE));
        touch(&t.0.join(STEAM_DLL));
        let env = |k: &str| (k == "SteamAppId").then(|| OsString::from("359870"));
        let r = rust_launch(&g, &t.0, Some(&t.0), env).expect("launch");
        assert!(r.env.is_empty(), "{:?}", r.env);
        // an empty one is not set
        let env = |k: &str| (k == "SteamAppId").then(OsString::new);
        let r = rust_launch(&g, &t.0, Some(&t.0), env).expect("launch");
        assert_eq!(r.env.len(), 1);
    }

    #[test]
    fn steam_dll_missing_is_a_warning() {
        let t = Tmp::new("nodll");
        let g = t.0.join("game");
        game(&g);
        touch(&t.0.join(RUST_EXE));
        let r = rust_launch(&g, &t.0, Some(&t.0), no_env).expect("starts anyway");
        assert_eq!(r.warnings, vec![Problem::NoSteamDll(t.0.clone())]);
        assert!(r.warnings[0].is_warning());
        assert!(r.warnings[0].to_string().contains("runs anyway"));
        // FFX_STEAM_API names one elsewhere
        let dll = t.0.join("sdk").join(STEAM_DLL);
        touch(&dll);
        let env = |k: &str| (k == "FFX_STEAM_API").then(|| dll.clone().into_os_string());
        let r = rust_launch(&g, &t.0, Some(&t.0), env).expect("launch");
        assert_eq!(r.warnings, Vec::new());
        // FFX_NO_STEAM: Steam off on purpose, no note
        let env = |k: &str| (k == "FFX_NO_STEAM").then(|| OsString::from("1"));
        let r = rust_launch(&g, &t.0, Some(&t.0), env).expect("launch");
        assert_eq!(r.warnings, Vec::new());
    }

    #[test]
    fn rust_launch_problems() {
        let t = Tmp::new("problems");
        let g = t.0.join("game");
        game(&g);
        assert_eq!(
            rust_launch(&g, &t.0, Some(&t.0), no_env),
            Err(Problem::NoRustExe(t.0.clone()))
        );
        touch(&t.0.join(RUST_EXE));
        assert_eq!(
            rust_launch(&g, &t.0, None, no_env),
            Err(Problem::NoDocuments)
        );
        fs::remove_file(g.join("data/FFX_Data.vbf")).expect("rm");
        assert!(matches!(
            rust_launch(&g, &t.0, Some(&t.0), no_env),
            Err(Problem::MissingGameFile { .. })
        ));
        assert!(!Problem::NoDocuments.is_warning());
        assert_eq!(Problem::NoDocuments.title(), "ffx-rust cannot start");
    }
}
