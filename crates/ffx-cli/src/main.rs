//! `ffx`: command-line tools for FFX game data.

use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use ffx_kernel::{Schema, Table};
use ffx_monster::MonsterFile;
use ffx_script::{Container, Natives, disassemble_annotated};
use ffx_text::{Charset, Language};
use ffx_vbf::{Archive, Entry};
use rayon::prelude::*;

#[derive(Parser)]
#[command(name = "ffx", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Work with VBF archives.
    #[command(subcommand)]
    Vbf(VbfCommand),
    /// Export and import the battle kernel tables (items, commands, monsters, ...).
    #[command(subcommand)]
    Kernel(KernelCommand),
    /// Export and import monster stats and rewards (battle/mon/_mNNN/mNNN.bin).
    #[command(subcommand)]
    Monster(MonsterCommand),
    /// Disassemble and assemble event and monster AI scripts.
    #[command(subcommand)]
    Script(ScriptCommand),
}

#[derive(Subcommand)]
enum ScriptCommand {
    /// List the files that contain scripts.
    List {
        #[command(flatten)]
        game: GameArgs,
        /// Only paths containing this text.
        #[arg(default_value = "")]
        filter: String,
    },
    /// Disassemble a script: an archive path (`.../znkd0900.ebp`) or a monster (`m001`).
    Disasm {
        #[command(flatten)]
        game: GameArgs,
        file: String,
        /// Output file; stdout if omitted.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Assemble edited text onto the original script and write the file.
    Asm {
        #[command(flatten)]
        game: GameArgs,
        file: String,
        /// The edited disassembly.
        source: PathBuf,
        /// Write the file here.
        #[arg(short, long, conflicts_with = "mod_dir")]
        out: Option<PathBuf>,
        /// Write the file into a mod folder, at its path inside the archive.
        #[arg(long)]
        mod_dir: Option<PathBuf>,
    },
}

/// Our recovered names, for annotating native function calls.
const SYMBOLS: &str = include_str!("../../../symbols/ffx.sym");

fn symbol_names() -> std::collections::HashMap<u32, &'static str> {
    SYMBOLS
        .lines()
        .filter_map(|l| {
            let mut w = l.split('#').next()?.split_whitespace();
            let addr = u32::from_str_radix(w.next()?.trim_start_matches("0x"), 16).ok()?;
            let _kind = w.next()?;
            Some((addr, w.next()?))
        })
        .collect()
}

/// Turns `m001` into the monster's archive path; other names are paths.
fn script_path(file: &str) -> String {
    file.strip_prefix('m')
        .and_then(|n| n.parse::<u16>().ok())
        .map_or_else(|| file.to_string(), ffx_monster::archive_path)
}

fn script(cmd: ScriptCommand) -> Result<()> {
    match cmd {
        ScriptCommand::List { game, filter } => {
            let game = Game::open(&game)?;
            let mut paths: Vec<&str> = game
                .archive
                .entries()
                .iter()
                .map(|e| e.path.as_str())
                .filter(|p| {
                    p.ends_with(".ebp") || (p.contains("/battle/mon/_m") && p.ends_with(".bin"))
                })
                .filter(|p| p.contains(&filter))
                .collect();
            paths.sort_unstable();
            let mut out = std::io::stdout().lock();
            for p in paths {
                writeln!(out, "{p}")?;
            }
        }
        ScriptCommand::Disasm { game, file, out } => {
            let args = game;
            let game = Game::open(&args)?;
            let path = script_path(&file);
            let container = Container::parse(read_file(&game.archive, &path)?)?;
            let names = symbol_names();
            let natives = std::fs::read(args.game_dir.join("FFX.exe"))
                .ok()
                .and_then(Natives::from_exe);
            let annotate = |id: u16| {
                let handler = natives.as_ref()?.handler(id)?;
                Some(match names.get(&handler) {
                    Some(name) => format!("{name} 0x{handler:08x}"),
                    None => format!("0x{handler:08x}"),
                })
            };
            let text = format!(
                "; {path}\n{}",
                disassemble_annotated(&container.script, &annotate)?
            );
            match out {
                Some(p) => std::fs::write(p, text)?,
                None => std::io::stdout().lock().write_all(text.as_bytes())?,
            }
        }
        ScriptCommand::Asm {
            game,
            file,
            source,
            out,
            mod_dir,
        } => {
            let game = Game::open(&game)?;
            let path = script_path(&file);
            let container = Container::parse(read_file(&game.archive, &path)?)?;
            let text = std::fs::read_to_string(&source)
                .with_context(|| format!("reading {}", source.display()))?;
            let rebuilt =
                container.replace_script(&ffx_script::assemble(&container.script, &text)?)?;
            let dest = match (out, mod_dir) {
                (Some(p), _) => p,
                (None, Some(dir)) => dir.join(safe_relative(&path)?),
                (None, None) => bail!("pass --out <file> or --mod-dir <dir>"),
            };
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest, rebuilt.bytes())?;
            eprintln!("wrote {}", dest.display());
        }
    }
    Ok(())
}

#[derive(Subcommand)]
enum MonsterCommand {
    /// List monster IDs with their names and HP.
    List {
        #[command(flatten)]
        game: GameArgs,
    },
    /// Export one monster's stats and rewards as TOML.
    Export {
        #[command(flatten)]
        game: GameArgs,
        id: u16,
        /// Output file; stdout if omitted.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Apply TOML onto the original monster file and write the result.
    Import {
        #[command(flatten)]
        game: GameArgs,
        id: u16,
        /// TOML with `[stats]` and/or `[spoils]` edits.
        toml: PathBuf,
        /// Write the file here.
        #[arg(short, long, conflicts_with = "mod_dir")]
        out: Option<PathBuf>,
        /// Write the file into a mod folder, at its path inside the archive.
        #[arg(long)]
        mod_dir: Option<PathBuf>,
    },
}

/// Where the game is and which language to use.
#[derive(clap::Args)]
struct GameArgs {
    /// Game folder (contains FFX.exe); defaults to `$FFX_GAME_DIR`.
    #[arg(long, env = "FFX_GAME_DIR")]
    game_dir: PathBuf,
    /// Language: en, jp, fr, sp, de, it, kr, ch.
    #[arg(long, default_value = "en")]
    lang: String,
}

#[derive(Subcommand)]
enum KernelCommand {
    /// List the tables and their known fields.
    Tables,
    /// Export one table (e.g. `item`), or every table with `--all`, as TOML.
    Export {
        #[command(flatten)]
        game: GameArgs,
        /// Table name, e.g. `item` or `command`.
        table: Option<String>,
        /// Export every table into the `--out` directory.
        #[arg(long)]
        all: bool,
        /// Output file (or directory with `--all`); stdout if omitted.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Apply a TOML file onto the original table and write the result.
    Import {
        #[command(flatten)]
        game: GameArgs,
        /// Table name, e.g. `item`.
        table: String,
        /// TOML with the edits; only the listed records and keys change.
        toml: PathBuf,
        /// Write the table here.
        #[arg(short, long, conflicts_with = "mod_dir")]
        out: Option<PathBuf>,
        /// Write the table into a mod folder, at its path inside the archive.
        #[arg(long)]
        mod_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum VbfCommand {
    /// List files as `size<TAB>offset<TAB>path`, sorted by path.
    List { archive: PathBuf },
    /// Print one file to stdout.
    Cat { archive: PathBuf, path: String },
    /// Write a new archive: the base with files replaced or added from mod
    /// folders (each mirroring archive paths; later folders win).
    Pack {
        /// The original archive.
        base: PathBuf,
        /// Mod folder(s) whose files replace or add archive files.
        #[arg(long = "mod", required = true)]
        mods: Vec<PathBuf>,
        /// The new archive; must differ from the base.
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Extract files whose path starts with an optional prefix.
    Extract {
        archive: PathBuf,
        /// Output directory.
        #[arg(short, long)]
        out: PathBuf,
        /// Only extract paths starting with this prefix.
        #[arg(short, long, default_value = "")]
        prefix: String,
    },
}

fn main() -> Result<()> {
    let result = run(Cli::parse());
    // Piping into `head` closes stdout early; that is not an error.
    if let Err(e) = &result
        && e.downcast_ref::<std::io::Error>()
            .is_some_and(|e| e.kind() == std::io::ErrorKind::BrokenPipe)
    {
        return Ok(());
    }
    result
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Vbf(cmd) => vbf(cmd),
        Command::Kernel(cmd) => kernel(cmd),
        Command::Monster(cmd) => monster(cmd),
        Command::Script(cmd) => script(cmd),
    }
}

/// The game's main archive and one language's charset.
struct Game {
    archive: Archive,
    language: Language,
    charset: Charset,
}

impl Game {
    fn open(args: &GameArgs) -> Result<Self> {
        let language = Language::from_code(&args.lang)
            .with_context(|| format!("unknown language {:?}", args.lang))?;
        let archive = open(&args.game_dir.join("data/FFX_Data.vbf"))?;
        let charset =
            Charset::from_table(&read_file(&archive, &language.charset_path())?, language)?;
        Ok(Self {
            archive,
            language,
            charset,
        })
    }

    fn kernel_path(&self, table: &str) -> String {
        format!("{}{table}.bin", self.language.kernel_dir())
    }
}

fn monster(cmd: MonsterCommand) -> Result<()> {
    match cmd {
        MonsterCommand::List { game } => {
            let game = Game::open(&game)?;
            let tables: Vec<Table> = ["monster1", "monster2", "monster3"]
                .iter()
                .map(|t| {
                    Ok(Table::parse(&read_file(
                        &game.archive,
                        &game.kernel_path(t),
                    )?)?)
                })
                .collect::<Result<_>>()?;
            let mut out = std::io::stdout().lock();
            for table in &tables {
                for id in table.ids() {
                    if game.archive.find(&ffx_monster::archive_path(id)).is_none() {
                        continue;
                    }
                    let name = game.charset.decode(table.string(id, 0).unwrap_or_default());
                    let hp = table.record(id).map_or(0, |r| {
                        u32::from_le_bytes(r[0x14..0x18].try_into().expect("4 bytes"))
                    });
                    writeln!(out, "{id:3}  {hp:>9} HP  {name}")?;
                }
            }
        }
        MonsterCommand::Export { game, id, out } => {
            let game = Game::open(&game)?;
            let file =
                MonsterFile::parse(read_file(&game.archive, &ffx_monster::archive_path(id))?)?;
            let text = format!("# battle/mon/_m{id:03}/m{id:03}.bin\n\n{}", file.export()?);
            match out {
                Some(path) => std::fs::write(path, text)?,
                None => std::io::stdout().lock().write_all(text.as_bytes())?,
            }
        }
        MonsterCommand::Import {
            game,
            id,
            toml,
            out,
            mod_dir,
        } => {
            let game = Game::open(&game)?;
            let archive_path = ffx_monster::archive_path(id);
            let mut file = MonsterFile::parse(read_file(&game.archive, &archive_path)?)?;
            let edits = std::fs::read_to_string(&toml)
                .with_context(|| format!("reading {}", toml.display()))?;
            file.import(&edits)?;
            let dest = match (out, mod_dir) {
                (Some(path), _) => path,
                (None, Some(dir)) => dir.join(safe_relative(&archive_path)?),
                (None, None) => bail!("pass --out <file> or --mod-dir <dir>"),
            };
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest, file.bytes())?;
            eprintln!("wrote {}", dest.display());
        }
    }
    Ok(())
}

fn read_file(archive: &Archive, path: &str) -> Result<Vec<u8>> {
    let entry = archive
        .find(path)
        .with_context(|| format!("{path}: not in archive"))?;
    Ok(archive.read(entry)?)
}

fn schema(table: &str) -> Result<&'static Schema> {
    Schema::by_file(table)
        .with_context(|| format!("unknown table {table:?}; see `ffx kernel tables`"))
}

fn kernel(cmd: KernelCommand) -> Result<()> {
    match cmd {
        KernelCommand::Tables => {
            for s in ffx_kernel::SCHEMAS {
                let fields: Vec<&str> = s
                    .strings
                    .iter()
                    .copied()
                    .chain(s.fields.iter().map(|f| f.name))
                    .collect();
                println!(
                    "{:<12} {:>4} bytes  {}",
                    s.file,
                    s.record_size,
                    fields.join(", ")
                );
            }
        }
        KernelCommand::Export {
            game,
            table,
            all,
            out,
        } => {
            let game = Game::open(&game)?;
            let export_one = |schema: &Schema| -> Result<String> {
                let table =
                    Table::parse(&read_file(&game.archive, &game.kernel_path(schema.file))?)?;
                Ok(ffx_kernel::export(&table, schema, &game.charset)?)
            };
            if all {
                let dir = out.context("--all needs --out <directory>")?;
                std::fs::create_dir_all(&dir)?;
                for s in ffx_kernel::SCHEMAS {
                    std::fs::write(dir.join(format!("{}.toml", s.file)), export_one(s)?)?;
                }
                eprintln!(
                    "exported {} tables to {}",
                    ffx_kernel::SCHEMAS.len(),
                    dir.display()
                );
            } else {
                let text = export_one(schema(&table.context("name a table, or pass --all")?)?)?;
                match out {
                    Some(path) => std::fs::write(path, text)?,
                    None => std::io::stdout().lock().write_all(text.as_bytes())?,
                }
            }
        }
        KernelCommand::Import {
            game,
            table,
            toml,
            out,
            mod_dir,
        } => {
            let game = Game::open(&game)?;
            let schema = schema(&table)?;
            let archive_path = game.kernel_path(schema.file);
            let mut parsed = Table::parse(&read_file(&game.archive, &archive_path)?)?;
            let edits = std::fs::read_to_string(&toml)
                .with_context(|| format!("reading {}", toml.display()))?;
            ffx_kernel::import(&mut parsed, schema, &game.charset, &edits)?;
            let dest = match (out, mod_dir) {
                (Some(path), _) => path,
                (None, Some(dir)) => dir.join(safe_relative(&archive_path)?),
                (None, None) => bail!("pass --out <file> or --mod-dir <dir>"),
            };
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest, parsed.to_bytes()?)?;
            eprintln!("wrote {}", dest.display());
        }
    }
    Ok(())
}

fn vbf(cmd: VbfCommand) -> Result<()> {
    match cmd {
        VbfCommand::List { archive } => {
            let archive = open(&archive)?;
            let mut entries: Vec<&Entry> = archive.entries().iter().collect();
            entries.sort_by(|a, b| a.path.cmp(&b.path));
            let mut out = std::io::stdout().lock();
            for e in entries {
                writeln!(out, "{}\t{}\t{}", e.size, e.offset, e.path)?;
            }
        }
        VbfCommand::Cat { archive, path } => {
            let archive = open(&archive)?;
            let entry = archive
                .find(&path)
                .with_context(|| format!("{path}: not in archive"))?;
            std::io::stdout().lock().write_all(&archive.read(entry)?)?;
        }
        VbfCommand::Pack { base, mods, out } => {
            if out.canonicalize().ok() == base.canonicalize().ok() {
                bail!("refusing to overwrite the base archive");
            }
            let archive = open(&base)?;
            let mut changes = std::collections::BTreeMap::new();
            for dir in &mods {
                let mut files = Vec::new();
                collect_files(dir, &mut files)?;
                for file in files {
                    let rel = file.strip_prefix(dir).expect("under dir");
                    let key = rel.to_string_lossy().replace('\\', "/");
                    changes.insert(key, std::fs::read(&file)?);
                }
            }
            let tmp = out.with_extension("vbf.partial");
            let mut writer = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
            let stats = ffx_vbf::pack(&archive, &changes, &mut writer)?;
            writer.flush()?;
            drop(writer);
            std::fs::rename(&tmp, &out)?;
            eprintln!(
                "wrote {}: {} files ({} replaced, {} added), {} bytes",
                out.display(),
                stats.files,
                stats.replaced,
                stats.added,
                stats.bytes
            );
        }
        VbfCommand::Extract {
            archive,
            out,
            prefix,
        } => {
            let archive = open(&archive)?;
            let selected: Vec<&Entry> = archive
                .entries()
                .iter()
                .filter(|e| e.path.starts_with(&prefix))
                .collect();
            if selected.is_empty() {
                bail!("no files start with {prefix:?}");
            }
            let bytes = AtomicU64::new(0);
            selected.par_iter().try_for_each(|entry| -> Result<()> {
                let dest = out.join(safe_relative(&entry.path)?);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let data = archive.read(entry)?;
                std::fs::write(&dest, &data)
                    .with_context(|| format!("writing {}", dest.display()))?;
                bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
                Ok(())
            })?;
            eprintln!(
                "extracted {} files, {} bytes",
                selected.len(),
                bytes.into_inner()
            );
        }
    }
    Ok(())
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

fn open(path: &Path) -> Result<Archive> {
    Archive::open(path).with_context(|| format!("opening {}", path.display()))
}

/// Rejects archive paths that would escape the output directory.
fn safe_relative(path: &str) -> Result<PathBuf> {
    let rel = PathBuf::from(path);
    if rel.components().all(|c| matches!(c, Component::Normal(_))) {
        Ok(rel)
    } else {
        bail!("refusing unsafe archive path {path:?}")
    }
}
