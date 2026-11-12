//! `ffx-host`: runs FFX without loading `FFX.exe` (docs/host.md; M5
//! stages 3 and 4 in docs/rewrite-plan.md).
//!
//! - [`arena`]: the game's 32-bit address space, one block on `x86_64`;
//! - [`image`]: the player's `FFX.exe` mapped into it, read as data;
//! - [`table`]: the function table, every port under its original address;
//! - [`exec`]: the executor, ports in Rust and (development only, feature
//!   `dev-unicorn`) the rest from `FFX.exe`'s `.text` in Unicorn, on the
//!   same memory; without the feature a call to an address with no Rust
//!   side is a hard stop that names it ([`Error::Unported`]);
//! - [`imports`]: the original's imports, their Rust sides;
//! - [`ports`]: what this crate registers in the table;
//! - [`report`]: what a run reached, for the log and the tests.

#[cfg(not(target_pointer_width = "64"))]
compile_error!(
    "ffx-host is 64-bit only (x86_64-pc-windows-msvc, or a 64-bit Linux for the tests): the \
     i686 ffx-rust.exe was dropped (docs/host.md, \"Building for Windows\")"
);

pub mod arena;
pub mod audio;
pub mod clock;
pub mod com;
pub mod com_methods;
pub mod crt;
pub mod crt_dll;
pub mod crt_format;
pub mod crt_start;
pub mod crt_stdio;
pub mod crypt;
#[cfg(windows)]
pub mod d3d11;
pub mod devsave;
pub mod dll;
pub mod exec;
pub mod files;
pub mod gfx;
pub mod harness;
pub mod iggy;
pub mod image;
pub mod imports;
pub mod leaf;
pub mod library;
pub mod log;
pub mod ports;
pub mod replay;
pub mod report;
pub mod rsrc;
pub mod stacks;
pub mod stand_ins;
pub mod steam;
pub mod sync;
pub mod table;
#[cfg(all(windows, feature = "dev-unicorn"))]
pub mod tcg_commit;
pub mod win32;
#[cfg(windows)]
pub mod win_forward;

use std::path::Path;
use std::sync::Arc;

/// What can go wrong.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reading a file.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Parsing the PE.
    #[error("pe: {0}")]
    Pe(#[from] goblin::error::Error),
    /// Not the supported `FFX.exe`.
    #[error("image: {0}")]
    Image(String),
    /// An arena mapping was refused.
    #[error("map {0:#010x}+{1:#x}: {2}")]
    Map(u32, u32, String),
    /// Unicorn refused something.
    #[cfg(feature = "dev-unicorn")]
    #[error("unicorn: {0:?}")]
    Unicorn(unicorn_engine::uc_error),
    /// A call reached an address with no Rust side (no port, import,
    /// trap or one-instruction leaf) in a build without the emulator: the
    /// address, then its name and the callers (`exec`'s chain).
    #[error("unported address {addr:#010x}{what}")]
    Unported {
        /// The address called.
        addr: u32,
        /// ` (name) from caller <- caller ...`.
        what: String,
    },
    /// Emulated code called an import without a Rust side.
    #[error("unstubbed import {0}")]
    UnstubbedImport(String),
    /// Emulated code touched unmapped memory.
    #[error("bad {} at {addr:#010x} (eip {eip:#010x})", if *write { "write" } else { "read" })]
    BadAccess {
        /// The address.
        addr: u32,
        /// Where.
        eip: u32,
        /// Whether it was a write.
        write: bool,
    },
    /// An interrupt the executor doesn't handle (`int3` at an address with
    /// no port or stub).
    #[error("interrupt {0} at {1:#010x}")]
    Interrupt(u32, u32),
    /// The emulator stopped with no event.
    #[error("emulation ended at {0:#010x}")]
    Ended(u32),
    /// A call from Rust found no room on the stack.
    #[error("stack overflow calling {0:#010x}")]
    StackOverflow(u32),
    /// The run was ended (a milestone, the budget, `exit`).
    #[error("stopped")]
    Stopped,
}

#[cfg(feature = "dev-unicorn")]
impl From<unicorn_engine::uc_error> for Error {
    fn from(e: unicorn_engine::uc_error) -> Self {
        Self::Unicorn(e)
    }
}

/// Results.
pub type Result<T> = std::result::Result<T, Error>;

/// Maps `FFX.exe` from `exe` and builds the executor with every port this
/// build knows ([`ports::table`]).
///
/// # Errors
/// When the file can't be read, isn't the supported build, or a mapping
/// fails.
pub fn boot(exe: &Path) -> Result<exec::Exec> {
    boot_with(exe, ports::table())
}

/// [`boot`] with a function table of the caller's.
///
/// # Errors
/// As [`boot`].
pub fn boot_with(exe: &Path, table: table::Table) -> Result<exec::Exec> {
    boot_opts(exe, table, &Options::default())
}

/// How to boot.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Leave the player's `MSVCR110.dll` and `MSVCP110.dll` out (Rust
    /// stand-ins only, as the first round ran). Always so without
    /// `dev-unicorn`.
    pub no_dlls: bool,
    /// The folder the game sees as its documents folder
    /// (`SHGetFolderPathW`), for `GameSetting.ini` and the saves; `None`:
    /// none (every file there is missing).
    pub documents: Option<std::path::PathBuf>,
    /// The D3D11 layer's back end and stop (`crate::gfx`).
    pub gfx: gfx::Config,
}

/// [`boot_with`] with [`Options`]: maps the image and (unless
/// `no_dlls`) the C runtime DLLs from beside it, builds the main thread's
/// executor and runs the DLLs' `DllMain(DLL_PROCESS_ATTACH)`.
///
/// # Errors
/// As [`boot`]; a DLL whose `DllMain` fails stops the boot.
pub fn boot_opts(exe: &Path, table: table::Table, opts: &Options) -> Result<exec::Exec> {
    let bytes = std::fs::read(exe)?;
    let arena = Arc::new(arena::Arena::new()?);
    let image = image::load(&arena, &bytes)?;
    let dir = exe.parent().unwrap_or(Path::new("."));
    // Without the emulator the DLLs' code can't run: every C runtime
    // import is its Rust side or missing (a hard stop when called).
    let dlls = if opts.no_dlls || cfg!(not(feature = "dev-unicorn")) {
        Vec::new()
    } else {
        dll::load_runtime(&arena, dir)?
    };
    let mut files = files::Files::new(dir);
    files.documents.clone_from(&opts.documents);
    let shared = exec::Shared::new(arena, image, dlls, table, files)?;
    shared.gfx.configure(opts.gfx.clone());
    let mut exec = exec::Exec::new(shared)?;
    match exec.attach_dlls(1) {
        Ok(()) => Ok(exec),
        Err(Error::Stopped) => Err(Error::Image(format!(
            "the C runtime DLLs did not start: {}",
            exec.stopped().map(ToString::to_string).unwrap_or_default()
        ))),
        Err(e) => Err(e),
    }
}
