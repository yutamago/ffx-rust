# PS2 emulation (`ffx_ps2`)

The PC port turned the PS2's hand-written R5900 and VU0 assembly into C
that keeps the registers in globals (`docs/rewrite-plan.md`, "The PS2
emulation registers"), and kept the VU0 vector library (`FFXVu0*`).

## Register set-up (`AsmEmuInit`, `0x0042ee40`)

| Address | | Set to |
|---|---|---|
| `0x00c0a004..0x00c0a013` | `g_Vu0_Vf00`, 4 x f32 | (0, 0, 0, 1.0): `fldz` x3, `fld1` |
| `0x00c8f6d8` | `g_Ee_Gpr` + 0x1d0, `sp`'s low word | the address of `g_Ee_Gpr` (relocated) |

Two `dbgPrintf` lines follow ("Sizeof 64 bit int = %d", "PS2 Assembly
emulation initilization completed!"); `dbgPrintf` (`0x0062f500`) is a bare
`ret` in this build, so `eax` stays the caller's.
`restoreVf00Register` (`0x0042ef00`) stores the same `vf00`.

## Library code in the module (not ported)

The classification files the movie player's Vorbis bit reader here (its
code sits next to `AsmEmu*`): libogg's `oggpack_write` (`0x0042e350`),
`oggpack_look` (`0x0042e840`), `oggpack_adv` (`0x0042ea30`),
`oggpack_read` (`0x0042eb10`) over `{endbyte, endbit, buffer, ptr,
storage}` with libogg's mask table at `0x00b8f4b8`, and libwebm's
`mkvparser::Block::GetFrame` (`0x0042e0d0`, named by its asserts). They are
library code and stay original.
