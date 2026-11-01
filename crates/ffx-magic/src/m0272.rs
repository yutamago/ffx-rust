//! `magic_0272.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Sig, sa_summon};

/// The calling thunk at 0x100032b0 (`push ebp; mov ebp, esp; push ecx;
/// mov ecx, [ebp + 8]; call X; pop ecx; pop ebp; ret`, 15 bytes) over
/// [`sa_summon::vector_op`], run by [`crate::s3_wrap::ecx_call`].
pub const CALL_VECTOR_OP: Sig = Sig {
    len: 15,
    hash: crate::s3_wrap::ECX_CALL_HASH,
    calls: &[(8, Callee::Sig(&sa_summon::VECTOR_OP))],
    imports: &[],
};
