//! `TkMn2ArmsRefreshArms` (`0x008cfd70`): the callback (queued through
//! `0x008751b0` by `TkMenuCtrlEquip`, the weapon shop and the
//! customisation menu) that swaps the field models' weapon and armour
//! after the equipment changed, one step per call.

/// The callees, by address.
pub mod callee {
    pub const CHANGE_CURRENT_CTRL: u32 = 0x0086_2d30;
    pub const TOTAL_ACTOR_NUMBER2: u32 = 0x0086_c510;
    pub const GET_BASIC_WORK: u32 = 0x0086_a830;
    pub const GET_BIND_PC: u32 = 0x0086_adf0;
    pub const GET_WEAPON: u32 = 0x008a_9c70;
    pub const GET_ARMOR: u32 = 0x008a_9820;
    pub const CH_DISPOSE: u32 = 0x0087_22f0;
    pub const DATA_DISPOSE: u32 = 0x0082_4f00;
    pub const WEAPON_MODEL: u32 = 0x007a_bba0;
    pub const ARMOR_MODEL: u32 = 0x007a_bb70;
    pub const DATA_READ: u32 = 0x0082_9ef0;
    pub const DATA_READ_SYNC: u32 = 0x0082_a040;
    pub const CH_ALLOCATE: u32 = 0x0087_2190;
    pub const SOUND_SET_ID: u32 = 0x0083_5b50;
    pub const BIND_TO_ELEM: u32 = 0x0087_22b0;
    pub const EN_CLIP: u32 = 0x0082_aa30;
    pub const VU1_SET_CLIPMODE: u32 = 0x0082_b970;
    pub const SET_CURRENT_CTRL: u32 = 0x0086_f730;
}

/// `g_TkMn2ArmsActor`: the actor being refreshed (`i32`).
pub const ACTOR: u32 = 0x0186_a6d4;
/// `g_TkMenuEquipShownWeapon`: per character, the weapon its model wears.
pub const SHOWN_WEAPON: u32 = 0x0186_a654;
/// `g_TkMenuEquipShownArmor`: per character, the armour its model wears.
pub const SHOWN_ARMOR: u32 = 0x0186_a674;

/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{ACTOR, SHOWN_ARMOR, SHOWN_WEAPON, callee as c};

    /// One of the two parts a model wears.
    struct Part {
        /// The model handle (`+0xaf4` / `+0xaf8`).
        handle: u32,
        /// The model data id (`u16 +0xb04` / `+0xb06`).
        id: u32,
        /// The element it binds to (`+0xafc` / `+0xb00`).
        elem: u32,
        /// The clip flags (`+0xb08` / `+0xb0c`).
        clip: u32,
        /// `TkCh_BindToElem`'s last argument.
        keep: u32,
    }

    const WEAPON: Part = Part {
        handle: 0xaf4,
        id: 0xb04,
        elem: 0xafc,
        clip: 0xb08,
        keep: 0,
    };
    const ARMOR: Part = Part {
        handle: 0xaf8,
        id: 0xb06,
        elem: 0xb00,
        clip: 0xb0c,
        keep: 1,
    };

    /// `TkMn2ArmsRefreshArms(state)` (`0x008cfd70`): with the current
    /// control switched to 0 (`AtelChangeCurrentCtrl`, restored at the
    /// end), runs the state machine `*state` from where it stopped:
    /// 0 starts over (actor -1), 1 and 9 go to the next actor (`*state`
    /// 9; past the last, `AtelGetTotalActorNumber2`, the result is 1)
    /// and pick it (state 2) when its `+0xaa` byte is 1, `+0x9c` set and
    /// its party member at most 6; 3 swaps the weapon when `+0xb14` bit 0
    /// is set and `TkMenuGetWeapon` differs from the one shown (the old
    /// model disposed, the new read: `*state` 4); 4 and 7 wait for the
    /// read (returning 0 while `Ch_DataReadSync` is 1); 5 and 8 allocate
    /// and bind the new model; 6 does the armour (bit 1; no read for
    /// model 0; `*state` 7 and return). Returns 1 when every actor is
    /// done, else 0; an unknown state returns 0 at once.
    #[allow(clippy::too_many_lines)] // the original's states in turn
    pub fn tk_mn2_arms_refresh_arms<M, C>(mem: &mut M, state: u32, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let mut f = |mem: &mut M, addr: u32, args: &[u32]| {
            let a = mem.rebase(addr);
            call(mem, a, args)
        };
        let actor_at = mem.rebase(ACTOR);
        let saved = f(mem, c::CHANGE_CURRENT_CTRL, &[0]);
        let total = f(mem, c::TOTAL_ACTOR_NUMBER2, &[]);
        let mut s = mem.read::<u32>(state);
        let mut done = 0;
        // the actor's work, and its party member when asked
        let work = |mem: &mut M, f: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let i = mem.read::<u32>(actor_at);
            f(mem, c::GET_BASIC_WORK, &[i])
        };
        // swaps the part's model when the gear shown is stale; whether it
        // started a read
        let swap = |mem: &mut M,
                    f: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
                    w: u32,
                    pc: u32,
                    p: &Part| {
            let at = |o: u32| w.wrapping_add(o);
            let old = mem.read::<u32>(at(p.handle));
            if old != 0 {
                f(mem, c::CH_DISPOSE, &[old]);
            }
            let id = u32::from(mem.read::<u16>(at(p.id)));
            if p.keep == 0 || id != 0 {
                f(mem, c::DATA_DISPOSE, &[id, 0]);
            }
            mem.write(at(p.handle), 0u32);
            let model = if p.keep == 0 {
                c::WEAPON_MODEL
            } else {
                c::ARMOR_MODEL
            };
            let id = f(mem, model, &[pc]) & 0xffff;
            #[allow(clippy::cast_possible_truncation)] // the original's `mov word`
            mem.write(at(p.id), id as u16);
            if p.keep == 0 || id != 0 {
                f(mem, c::DATA_READ, &[id]);
                true
            } else {
                false
            }
        };
        let bind =
            |mem: &mut M, f: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, w: u32, p: &Part| {
                let at = |o: u32| w.wrapping_add(o);
                let id = u32::from(mem.read::<u16>(at(p.id)));
                let h = f(mem, c::CH_ALLOCATE, &[id]);
                mem.write(at(p.handle), h);
                f(mem, c::SOUND_SET_ID, &[h, 0xf]);
                let kind = u32::from(mem.read::<u16>(at(0xa8)));
                let args = [
                    kind,
                    mem.read::<u32>(at(p.handle)),
                    mem.read::<u32>(at(0x9c)),
                    mem.read::<u32>(at(p.elem)),
                    p.keep,
                ];
                f(mem, c::BIND_TO_ELEM, &args);
                let h = mem.read::<u32>(at(p.handle));
                let clip = mem.read::<u32>(at(p.clip));
                f(mem, c::EN_CLIP, &[h, clip & 1]);
                let h = mem.read::<u32>(at(p.handle));
                let clip = mem.read::<u32>(at(p.clip));
                f(mem, c::VU1_SET_CLIPMODE, &[h, (clip >> 1) & 1]);
            };
        // `ja`: states above 10 end at once
        while s <= 10 {
            s = match s {
                0 | 1 | 9 => {
                    if s == 0 {
                        mem.write(actor_at, u32::MAX);
                    }
                    if s != 9 {
                        mem.write(state, 9u32);
                    }
                    let i = mem.read::<u32>(actor_at).wrapping_add(1);
                    mem.write(actor_at, i);
                    if i.cast_signed() >= total.cast_signed() {
                        10
                    } else {
                        let w = f(mem, c::GET_BASIC_WORK, &[i]);
                        let pc = f(mem, c::GET_BIND_PC, &[w]);
                        if mem.read::<u8>(w.wrapping_add(0xaa)) == 1
                            && mem.read::<u32>(w.wrapping_add(0x9c)) != 0
                            && pc <= 6
                        {
                            mem.write(state, 2u32);
                            2
                        } else {
                            9
                        }
                    }
                }
                2 => 3,
                3 => {
                    let w = work(mem, &mut f);
                    let pc = f(mem, c::GET_BIND_PC, &[w]);
                    let swap_it = mem.read::<u8>(w.wrapping_add(0xb14)) & 1 != 0 && {
                        let gear = f(mem, c::GET_WEAPON, &[pc]);
                        let shown = mem.rebase(SHOWN_WEAPON).wrapping_add(pc.wrapping_mul(4));
                        mem.read::<u32>(shown) != gear
                    };
                    if swap_it {
                        swap(mem, &mut f, w, pc, &WEAPON);
                        mem.write(state, 4u32);
                        4
                    } else {
                        6
                    }
                }
                4 | 7 => {
                    let w = work(mem, &mut f);
                    let p = if s == 4 { &WEAPON } else { &ARMOR };
                    let id = u32::from(mem.read::<u16>(w.wrapping_add(p.id)));
                    if f(mem, c::DATA_READ_SYNC, &[id]) == 1 {
                        break;
                    }
                    s + 1
                }
                5 | 8 => {
                    let w = work(mem, &mut f);
                    bind(mem, &mut f, w, if s == 5 { &WEAPON } else { &ARMOR });
                    if s == 5 { 6 } else { 9 }
                }
                6 => {
                    let w = work(mem, &mut f);
                    let pc = f(mem, c::GET_BIND_PC, &[w]);
                    let swap_it = mem.read::<u8>(w.wrapping_add(0xb14)) & 2 != 0 && {
                        let gear = f(mem, c::GET_ARMOR, &[pc]);
                        let shown = mem.rebase(SHOWN_ARMOR).wrapping_add(pc.wrapping_mul(4));
                        mem.read::<u32>(shown) != gear
                    };
                    if swap_it && swap(mem, &mut f, w, pc, &ARMOR) {
                        mem.write(state, 7u32);
                        break;
                    }
                    9
                }
                // 10
                _ => {
                    done = 1;
                    break;
                }
            };
        }
        f(mem, c::SET_CURRENT_CTRL, &[saved]);
        done
    }
}
