//! FFX's battle kernel tables (`battle/kernel/*.bin` in `FFX_Data.vbf`):
//! items, commands, abilities, monster stats and battle text.
//!
//! - [`Table`] reads and writes the container byte for byte.
//! - [`schema`] describes each table's records.
//! - [`export`] turns a table into editable TOML and applies edits back.
//!
//! The format is described in `docs/formats/kernel.md`.

mod export;
pub mod record;
pub mod schema;
mod table;

pub use export::{export, import};
pub use schema::{Field, Kind, SCHEMAS, SHARED_SCHEMAS, Schema};
pub use table::{Segment, StringRef, Table};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("malformed table: {0}")]
    Malformed(&'static str),
    #[error("the table outgrew the format's 16-bit limits")]
    TooLarge,
    #[error("no record with ID {0}")]
    NoRecord(u16),
    #[error("record has no string field {0}")]
    NoField(usize),
    #[error("records do not match the {0} schema")]
    WrongSchema(&'static str),
    #[error("TOML: {0}")]
    Toml(String),
    #[error("record {id}, `{key}`: {problem}")]
    Value {
        id: u16,
        key: String,
        problem: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
