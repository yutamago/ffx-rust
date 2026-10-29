//! The words a function pushes for its calls to slots, as a port passes
//! them (docs/formats/magic.md, "Porting the overlays": each call passes
//! the words on the original's stack at that call, earlier pushes included).
//!
//! The overlays push a call's arguments and leave them there: the original
//! cleans up with one `add esp, n` at the end. A callee sees the most recent
//! push as its first argument and every older push below it, so
//! [`Pushes::words`] lists the words top first, the order a cdecl call reads
//! them in.

use crate::{Game, Out};

/// The words a function has pushed, most recent first.
#[derive(Debug, Default)]
pub struct Pushes(Vec<u32>);

/// How many words a call passes. A slot callee reads only its own
/// parameters, which are the most recent pushes; the largest argument count
/// among the calls made through [`Pushes`] is 9 (`magic_0187`'s 0x1420), so
/// the words below the first 12 are never read. The parity harness logs 12
/// words of the original's stack.
const CALL_WORDS: usize = 12;

impl Pushes {
    /// Pushes `word` (the next call's argument, or a word under it).
    pub fn push(&mut self, word: u32) {
        self.0.insert(0, word);
    }

    /// The words a call sees, most recent first (at most [`CALL_WORDS`]).
    #[must_use]
    pub fn words(&self) -> &[u32] {
        &self.0[..self.0.len().min(CALL_WORDS)]
    }

    /// Calls the cdecl function at `target` with [`Pushes::words`]; `ecx` is
    /// the entry's (the callees are cdecl, so it is not read).
    pub fn call<G: Game>(&self, g: &mut G, target: u32, ecx: u32) -> Out {
        // call width: 12 (the most `words` holds)
        g.call(target, ecx, self.words())
    }
}
