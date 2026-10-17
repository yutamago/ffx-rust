//! FFX's battles: which ones a map can start, and what each one holds.
//!
//! - [`EncounterTable`] reads `ffx_ps2/ffx/master/jppc/battle/kernel/btl.bin`,
//!   the battle fields with their encounter groups (random encounter rate,
//!   battle map, formations and their weights).
//! - [`FormationFile`] reads `ffx_ps2/ffx/master/jppc/battle/btl/<name>/<name>.bin`,
//!   one battle: its script, the monsters in its eight slots and a few
//!   flags.
//!
//! Both are described in `docs/formats/battle.md`.
//!
//! The battle code of `FFX.exe` (the PS2 `Ms*` functions) is ported module
//! by module, each with a `bridge` onto the game's memory
//! (`docs/porting.md`): [`rng`], [`save`], [`actor`], [`camera`],
//! [`state`], [`util`], [`damage`], [`command`], [`vector`], [`misc`], [`minor`], [`body`], [`menu`], [`cue`], [`turn`], [`flow`] and [`pos`].

pub mod ability;
pub mod action;
pub mod actor;
pub mod after_damage;
pub mod atel;
pub mod battle_end;
pub mod body;
pub mod camera;
pub mod chr_damage;
pub mod command;
pub mod cue;
pub mod damage;
pub mod damage_end;
pub mod death;
pub mod effect;
mod encounters;
pub mod exchange;
pub mod exec;
pub mod flow;
mod formation;
pub mod input;
pub mod menu;
pub mod minor;
pub mod misc;
pub mod parse;
pub mod pos;
pub mod process;
pub mod rng;
pub mod save;
pub mod save_param;
pub mod stat;
pub mod state;
pub mod status;
pub mod target;
pub mod turn;
pub mod util;
pub mod vector;
pub mod visible;

pub use encounters::{EncounterTable, Field, Formation, Group};
pub use formation::{FORMATION, FormationFile, MONSTER_SLOTS};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("malformed {0}")]
    Malformed(&'static str),
    #[error("TOML: {0}")]
    Toml(String),
    #[error("{0}")]
    Value(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Archive path of the encounter table.
pub const ENCOUNTER_TABLE: &str = "ffx_ps2/ffx/master/jppc/battle/kernel/btl.bin";

/// Archive path of formation file `name` (`cdsp00_00`).
#[must_use]
pub fn formation_path(name: &str) -> String {
    format!("ffx_ps2/ffx/master/jppc/battle/btl/{name}/{name}.bin")
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    b.get(o..o + 2)
        .map(|s| u16::from_le_bytes(s.try_into().expect("2 bytes")))
}

fn u32_at(b: &[u8], o: usize) -> Option<usize> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")) as usize)
}

fn parse_toml(text: &str) -> Result<toml::Table> {
    text.parse()
        .map_err(|e: toml::de::Error| Error::Toml(e.to_string()))
}

fn int<T: TryFrom<i64>>(v: &toml::Value, what: &str) -> Result<T> {
    v.as_integer()
        .and_then(|i| T::try_from(i).ok())
        .ok_or_else(|| Error::Value(format!("{what}: expected an integer in range, got {v}")))
}
