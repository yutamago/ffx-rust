//! `ffx`: command-line tools for FFX game data.

use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use ffx_kernel::{Schema, Table};
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
}

/// Where the game is and which language to use.
#[derive(clap::Args)]
struct GameArgs {
    /// Game folder (contains FFX.exe); defaults to $FFX_GAME_DIR.
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
    match Cli::parse().command {
        Command::Vbf(cmd) => vbf(cmd),
        Command::Kernel(cmd) => kernel(cmd),
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
