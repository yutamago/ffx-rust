//! `ffx`: command-line tools for FFX game data.

use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
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
    }
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
