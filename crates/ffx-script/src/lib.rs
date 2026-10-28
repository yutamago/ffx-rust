//! FFX's script engine: the bytecode behind events, NPCs, story flags and
//! monster AI.
//!
//! - [`opcode`]: the instruction set, from the interpreter `Script_Run`
//!   (`0x008641e0`).
//! - [`Script`]: a script block (header, workers, tables, code).
//! - [`text`]: disassembly to readable assembly and assembly back.
//! - [`Container`]: monster and event files that hold a script.
//! - [`Natives`]: native function IDs to their handlers in `FFX.exe`.
//! - [`vm`]: the VM's workers, stacks and variables as the game keeps them,
//!   and its accessors ported over game memory.
//!
//! Disassembling and reassembling any shipped script reproduces its bytes
//! exactly. The format is described in `docs/formats/script.md`.

mod container;
pub mod m5_thunks;
pub mod natives;
pub mod opcode;
mod script;
pub mod text;
pub mod vm;

pub use container::{Container, Kind};
pub use natives::Natives;
pub use script::{Instr, Script, Worker};
pub use text::{assemble, disassemble, disassemble_annotated};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("malformed script: {0}")]
    Malformed(&'static str),
    #[error("line {line}: {message}")]
    Syntax { line: usize, message: String },
    #[error("{0}")]
    Shape(String),
}

pub type Result<T> = std::result::Result<T, Error>;
