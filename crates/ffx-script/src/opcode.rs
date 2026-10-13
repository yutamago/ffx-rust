//! The instruction set of the script interpreter (`Script_Run`,
//! `0x008641e0`).
//!
//! An instruction is one byte; if its high bit is set, a `u16` operand
//! follows (`Script_Fetch`, `0x00869d60`). The opcode is the low 7 bits.
//! The interpreter is a stack machine with two integer registers (A, B),
//! four integer and ten float registers, and per-worker variables.
//!
//! Names are ours. Each comes from the opcode's case in `Script_Run`; where
//! the case only forwards to a helper whose purpose is still open, the name
//! says so (`req0`, `op_39`, …).

/// What an operand refers to; only affects how it is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// Index into the worker's jump label table (`l3`).
    Label,
    /// Index into the integer constant table (`c3`).
    Int,
    /// Index into the float constant table (`f3`).
    Float,
    /// Signed 16-bit immediate.
    Imm,
    /// Index into the worker's variable table (`v3`).
    Var,
    /// Native function ID.
    Func,
    /// Worker index (`w3`).
    Worker,
    /// A value whose meaning is not known.
    Raw,
}

/// One opcode.
#[derive(Debug, Clone, Copy)]
pub struct Opcode {
    pub name: &'static str,
    pub operand: Operand,
    pub doc: &'static str,
}

const fn op(name: &'static str, operand: Operand, doc: &'static str) -> Opcode {
    Opcode { name, operand, doc }
}

use Operand::{Float, Func, Imm, Int, Label, Raw, Var, Worker};

/// All 128 opcodes. Opcodes `0x1b`, `0x41`–`0x44` and `0x7b`–`0x7f` stop the
/// interpreter with an error; they never occur in shipped scripts.
pub const OPCODES: [Opcode; 128] = [
    op("nop", Raw, "no operation"),
    op("lor", Raw, "pop b, a; push a || b"),
    op("land", Raw, "pop b, a; push a && b"),
    op("or", Raw, "pop b, a; push a | b"),
    op("xor", Raw, "pop b, a; push a ^ b"),
    op("and", Raw, "pop b, a; push a & b"),
    op("eq", Raw, "pop b, a; push a == b (int or float)"),
    op("ne", Raw, "pop b, a; push a != b"),
    op("gtu", Raw, "pop b, a; push a > b, unsigned"),
    op("geu", Raw, "pop b, a; push a >= b, unsigned"),
    op("gt", Raw, "pop b, a; push a > b, signed"),
    op("ge", Raw, "pop b, a; push a >= b, signed"),
    op("ltu", Raw, "pop b, a; push a < b, unsigned"),
    op("leu", Raw, "pop b, a; push a <= b, unsigned"),
    op("lt", Raw, "pop b, a; push a < b, signed"),
    op("le", Raw, "pop b, a; push a <= b, signed"),
    op("btst", Raw, "pop bit, a; push (a >> bit) & 1"),
    op("btstn", Raw, "pop bit, a; push !((a >> bit) & 1)"),
    op("shl", Raw, "pop n, a; push a << n"),
    op("sar", Raw, "pop n, a; push a >> n (arithmetic)"),
    op("add", Raw, "pop b, a; push a + b (int or float)"),
    op("sub", Raw, "pop b, a; push a - b"),
    op("mul", Raw, "pop b, a; push a * b"),
    op("div", Raw, "pop b, a; push a / b"),
    op("mod", Raw, "pop b, a; push a % b"),
    op("not", Raw, "pop a; push !a"),
    op("neg", Raw, "pop a; push -a (float)"),
    op("op_1b", Raw, "invalid"),
    op("bnot", Raw, "pop a; push ~a"),
    op("nop_1d", Raw, "no operation"),
    op("nop_1e", Raw, "no operation"),
    op("pushv", Var, "push variable"),
    op("popv", Var, "pop into variable"),
    op("popv_1", Var, "pop into variable, store mode 1"),
    op("pushva", Var, "pop index; push array element"),
    op("popva", Var, "pop value, index; store array element"),
    op(
        "popva_1",
        Var,
        "pop value, index; store array element, mode 1",
    ),
    op(
        "popfacc",
        Raw,
        "pop into the float accumulator (worker +0x28)",
    ),
    op("pushfacc", Raw, "push the float accumulator"),
    op(
        "pushvref",
        Var,
        "pop index; push the address of an array element",
    ),
    op("pusha", Raw, "push register A"),
    op("pushb", Raw, "push register B"),
    op("popa", Raw, "pop into register A"),
    op("op_2b", Raw, "push the result of 0x0086a000"),
    op("popb", Raw, "pop into register B"),
    op("pushc", Int, "push integer constant"),
    op("pushimm", Imm, "push signed 16-bit immediate"),
    op("pushfc", Float, "push float constant"),
    op("jmp", Label, "jump"),
    op("jnz", Label, "jump if register A != 0"),
    op("jz", Label, "jump if register A == 0"),
    op(
        "call",
        Worker,
        "call the worker's first entry; up to 3 deep",
    ),
    op("ret", Raw, "return from call"),
    op(
        "func",
        Func,
        "call native function; its float (slot 2) or integer (slot 3) result is pushed",
    ),
    op("req0", Raw, "worker request (0x00867230)"),
    op("reqw0", Raw, "worker request (0x00867570)"),
    op("reqe0", Raw, "worker request (0x008673d0)"),
    op("op_39", Raw, "pop 3; worker operation"),
    op("op_3a", Raw, "pop 3; worker operation"),
    op("op_3b", Raw, "pop 3; worker operation"),
    op("end", Raw, "end the entry"),
    op("endp", Raw, "pop; end the entry"),
    op("end_1", Raw, "end the entry, mode 1"),
    op("endp_1", Raw, "pop; end the entry, mode 1"),
    op("op_40", Raw, "wait"),
    op("op_41", Raw, "invalid"),
    op("op_42", Raw, "invalid"),
    op("op_43", Raw, "invalid"),
    op("op_44", Raw, "invalid"),
    op("req1", Raw, "worker request (0x00867230)"),
    op("req2", Raw, "worker request (0x00867230)"),
    op("req3", Raw, "worker request (0x00867230)"),
    op("req4", Raw, "worker request (0x00867230)"),
    op("req5", Raw, "worker request (0x00867230)"),
    op("reqw1", Raw, "worker request (0x00867570)"),
    op("reqw2", Raw, "worker request (0x00867570)"),
    op("reqw3", Raw, "worker request (0x00867570)"),
    op("reqw4", Raw, "worker request (0x00867570)"),
    op("reqw5", Raw, "worker request (0x00867570)"),
    op("reqe1", Raw, "worker request (0x008673d0)"),
    op("reqe2", Raw, "worker request (0x008673d0)"),
    op("reqe3", Raw, "worker request (0x008673d0)"),
    op("reqe4", Raw, "worker request (0x008673d0)"),
    op("reqe5", Raw, "worker request (0x008673d0)"),
    op("op_54", Raw, "halt the worker"),
    op("popa_jmp", Label, "pop into A; jump"),
    op("popa_jnz", Label, "pop into A; jump if A != 0"),
    op("popa_jz", Label, "pop into A; jump if A == 0"),
    op(
        "func_facc",
        Func,
        "call native function, then pop its result into the float accumulator",
    ),
    op("popi0", Raw, "pop into integer register 0"),
    op("popi1", Raw, "pop into integer register 1"),
    op("popi2", Raw, "pop into integer register 2"),
    op("popi3", Raw, "pop into integer register 3"),
    op("popf0", Raw, "pop into float register 0"),
    op("popf1", Raw, "pop into float register 1"),
    op("popf2", Raw, "pop into float register 2"),
    op("popf3", Raw, "pop into float register 3"),
    op("popf4", Raw, "pop into float register 4"),
    op("popf5", Raw, "pop into float register 5"),
    op("popf6", Raw, "pop into float register 6"),
    op("popf7", Raw, "pop into float register 7"),
    op("popf8", Raw, "pop into float register 8"),
    op("popf9", Raw, "pop into float register 9"),
    op("pushi0", Raw, "push integer register 0"),
    op("pushi1", Raw, "push integer register 1"),
    op("pushi2", Raw, "push integer register 2"),
    op("pushi3", Raw, "push integer register 3"),
    op("pushf0", Raw, "push float register 0"),
    op("pushf1", Raw, "push float register 1"),
    op("pushf2", Raw, "push float register 2"),
    op("pushf3", Raw, "push float register 3"),
    op("pushf4", Raw, "push float register 4"),
    op("pushf5", Raw, "push float register 5"),
    op("pushf6", Raw, "push float register 6"),
    op("pushf7", Raw, "push float register 7"),
    op("pushf8", Raw, "push float register 8"),
    op("pushf9", Raw, "push float register 9"),
    op("pushg", Raw, "push global constant (table 0x00c52704)"),
    op("nop_76", Raw, "no operation"),
    op("op_77", Raw, "pop 2; worker operation (0x00863570)"),
    op("op_78", Raw, "pop 2; worker operation (0x00863570)"),
    op("op_79", Raw, "pop 3; 0x00870d30"),
    op("op_7a", Raw, "pop 2; push 0x0086fab0"),
    op("op_7b", Raw, "invalid"),
    op("op_7c", Raw, "invalid"),
    op("op_7d", Raw, "invalid"),
    op("op_7e", Raw, "invalid"),
    op("op_7f", Raw, "invalid"),
];

/// Looks up an opcode by name.
#[must_use]
pub fn by_name(name: &str) -> Option<u8> {
    OPCODES
        .iter()
        .position(|o| o.name == name)
        .map(|i| u8::try_from(i).expect("128 opcodes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique() {
        for (i, o) in OPCODES.iter().enumerate() {
            assert_eq!(
                by_name(o.name),
                Some(u8::try_from(i).unwrap()),
                "{}",
                o.name
            );
        }
    }
}
