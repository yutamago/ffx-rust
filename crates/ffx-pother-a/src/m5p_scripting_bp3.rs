//! M5 stage 1, list `phyre-scripting`, round big-phyre3
//! (`ffx_pother_a::m5p_scripting_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_scripting.rs` (`bp3_*`).

/// `0x0061a2a0`'s callees: `PScheduler::copyBlock` (`ecx` the context,
/// the Lua state in `eax`), `loadScript(L, script)` (`al`), `lua_pcallk`
/// (six words), `lua_getglobal`, `lua_type`, `pushBase(L, &object)`,
/// `PTypeDispenser::GetType<bool>`, `luaL_checkstack`-like `0x0094ac00`
/// (L, n), `lua_pushboolean`, `lua_tolstring`-like `0x0094bfc0` (L, i,
/// 0), `lua_settop`, `PhyrePrintf`, `PBase::GetClassDescriptor`,
/// `PObjectAccessor<PBase>::Get(L)`.
pub const VCALL_CALLS: [u32; 14] = [
    0x0061_8130,
    0x0043_6750,
    0x0094_b4d0,
    0x0094_b080,
    0x0094_c1e0,
    0x0043_5580,
    0x0043_8390,
    0x0094_ac00,
    0x0094_b5a0,
    0x0094_bfc0,
    0x0094_bdc0,
    0x0043_52f0,
    0x0043_5740,
    0x0043_5650,
];
/// The argument pushers `pushVararg<T>(type, &va, L)` (cdecl, `al`), in
/// the order tried: `PChar`, `PUInt8`, `PInt16`, `PUInt16`, `PInt32`,
/// `unsigned int`, `PUInt64`, `PInt64`, `float`, `double`, `int`,
/// `PBase*`.
pub const VCALL_PUSHERS: [u32; 12] = [
    0x0061_8630,
    0x0061_8670,
    0x0061_86b0,
    0x0061_86f0,
    0x0061_8730,
    0x0061_8770,
    0x0061_8940,
    0x0061_88f0,
    0x0061_87b0,
    0x0061_8800,
    0x0061_88b0,
    0x0061_8850,
];
/// The result poppers `popValue<T>(type, L, &value)` (cdecl, `al`), in
/// the order tried: `bool`, `PChar`, `PUInt8`, `PInt16`, `PUInt16`,
/// `PInt32`, `unsigned int`, `PUInt64`, `PInt64`, `float`, `double`.
pub const VCALL_POPPERS: [u32; 11] = [
    0x0061_8da0,
    0x0061_8a40,
    0x0061_8a90,
    0x0061_8ae0,
    0x0061_8b30,
    0x0061_8b80,
    0x0061_8bd0,
    0x0061_8d50,
    0x0061_8d00,
    0x0061_8c20,
    0x0061_8c70,
];
/// The two schedulers whose context pools a call borrows from, and the
/// messages: an argument that can't be copied, `lua_pcall` failing, no
/// return value context.
pub const VCALL_GLOBALS: [u32; 5] = [
    0x00cc_9a38,
    0x00cb_d9b8,
    0x00b3_f610,
    0x00b2_d298,
    0x00b3_f64c,
];

pub mod vcall {
    //! `0x0061a2a0` over game memory.

    use super::{VCALL_CALLS, VCALL_GLOBALS, VCALL_POPPERS, VCALL_PUSHERS};
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// A context borrowed from the first scheduler pool (`[pool + 0x30]`
    /// used of `[pool + 0x28] & 0x7fffffff`, 0x14-byte contexts at
    /// `[pool + 0x2c]`) that has one free and gives a Lua state
    /// (`copyBlock`): `(pool, context, L)`; the last pool and context
    /// tried even when no state came.
    fn borrow(h: &mut impl Host) -> (u32, u32, u32) {
        let copy_block = VCALL_CALLS[0];
        let mut taken = (0, 0);
        for at in [VCALL_GLOBALS[0], VCALL_GLOBALS[1]] {
            let pool: u32 = h.read(h.rebase(at));
            if pool == 0 {
                continue;
            }
            let used: u32 = h.read(pool + 0x30);
            if used >= h.read::<u32>(pool + 0x28) & 0x7fff_ffff {
                continue;
            }
            h.write(pool + 0x30, used + 1);
            let ctx = h.read::<u32>(pool + 0x2c).wrapping_add(0x14 * used);
            if ctx == 0 {
                continue;
            }
            taken = (pool, ctx);
            let l = h.call_sym(copy_block, Conv::Thiscall(ctx), &[]);
            if l != 0 {
                return (pool, ctx, l);
            }
        }
        (taken.0, taken.1, 0)
    }

    /// The script call once the state is loaded: the function's global,
    /// the object (`pushBase(L, &object)`, the caller's argument slot)
    /// and each argument (types from `*types`, values from the `va`
    /// cell) pushed, `lua_pcallk(L, n, 1)`, the result popped into
    /// `holder` by its type; `true` when it came back.
    fn call(
        h: &mut impl Host,
        l: u32,
        name: u32,
        slots: u32,
        ret: u32,
        holder: u32,
        va: u32,
    ) -> bool {
        let [
            _,
            _,
            pcall,
            getglobal,
            lua_type,
            push_base,
            bool_type,
            check,
            push_bool,
            tostring,
            settop,
            printf,
            base_desc,
            get_base,
        ] = VCALL_CALLS;
        let [_, _, cannot_copy, failed, no_context] = VCALL_GLOBALS;
        let err = cdecl(h, pcall, &[l, 0, 0, 0, 0, 0]);
        let err = if err == 0 {
            cdecl(h, getglobal, &[l, name]);
            if cdecl(h, lua_type, &[l, 0xffff_ffff]) == 0 {
                return false;
            }
            cdecl(h, push_base, &[l, slots]);
            let mut types: u32 = h.read(holder);
            let mut count = 1u32;
            loop {
                let t: u32 = h.read(types);
                types = types.wrapping_add(4);
                if t == 0 {
                    break;
                }
                if t == cdecl(h, bool_type, &[]) {
                    let at = h.read::<u32>(va).wrapping_add(4);
                    h.write(va, at);
                    let v: u32 = h.read(at.wrapping_sub(4));
                    cdecl(h, check, &[l, 1]);
                    cdecl(h, push_bool, &[l, u32::from(v != 0)]);
                } else if !VCALL_PUSHERS
                    .iter()
                    .any(|&p| cdecl(h, p, &[t, va, l]) & 0xff != 0)
                {
                    let text = h.rebase(cannot_copy);
                    cdecl(h, printf, &[1, text, count]);
                }
                count += 1;
            }
            let err = cdecl(h, pcall, &[l, count, 1, 0, 0, 0]);
            if err == 0 {
                if VCALL_POPPERS
                    .iter()
                    .any(|&p| cdecl(h, p, &[ret, l, holder]) & 0xff != 0)
                {
                    return true;
                }
                if ret == cdecl(h, base_desc, &[]) | 1 {
                    let object = cdecl(h, get_base, &[l]);
                    h.write(holder, [0u32, object]);
                    return true;
                }
                let text = h.rebase(no_context);
                cdecl(h, printf, &[1, text]);
                return false;
            }
            err
        } else {
            err
        };
        let message = cdecl(h, tostring, &[l, 0xffff_ffff, 0]);
        let text = h.rebase(failed);
        cdecl(h, printf, &[3, text, err, message]);
        cdecl(h, settop, &[l, 0xffff_fffe]);
        false
    }

    /// `Phyre_PScripting_PClassCallableMethodScript_PCallerScript_vcall(this;
    /// object, ret, holder, va)` (thiscall, ret 0x10; Fahrenheit's name;
    /// `slots` the caller's argument words): 0 without argument types
    /// (`holder` or `*holder` 0) or a function name (`[this + 4]`'s
    /// tagged `+0x30`); else a scheduler context borrowed ([`borrow`]),
    /// the script (`+0x28`) loaded into its state and called
    /// ([`call`]); the context given back when it is still the pool's
    /// last. 1 when the call returned a value, else 0.
    pub fn vcall(h: &mut impl Host, slots: u32, this: u32, ret: u32, holder: u32, va: u32) -> u32 {
        let load_script = VCALL_CALLS[1];
        if holder == 0 || h.read::<u32>(holder) == 0 {
            return 0;
        }
        let script: u32 = h.read(this + 4);
        let source: u32 = h.read(script + 0x28);
        let tagged: u32 = h.read(script + 0x30);
        let name = tagged.wrapping_sub(u32::from(h.read::<u8>(script + 0x30) & 1));
        if name == 0 {
            return 0;
        }
        let (pool, ctx, l) = borrow(h);
        let ok = l != 0
            && cdecl(h, load_script, &[l, source]) & 0xff != 0
            && call(h, l, name, slots, ret, holder, va);
        if pool != 0 && ctx != 0 {
            let used: u32 = h.read(pool + 0x30);
            if used != 0
                && h.read::<u32>(pool + 0x2c)
                    .wrapping_add(0x14 * used)
                    .wrapping_sub(0x14)
                    == ctx
            {
                h.write(pool + 0x30, used - 1);
            }
        }
        u32::from(ok)
    }
}
