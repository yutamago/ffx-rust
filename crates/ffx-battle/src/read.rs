//! The battle's reads (`docs/rewrite-plan.md`, M2): leaves of
//! `MsBtlReadManage` (`0x00783020`), which loads a battle's scene and
//! actors across frames, and places and shows them when they are in.
//!
//! The CD object, the heap (`Sg_MallocA`) and chara's functions with
//! callbacks go to the game's code through `call`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::exec::bridge::Call;
    use crate::target::bridge::{Dm, Host, Regs};

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        call(mem, f, args)
    }

    /// `Sg_MallocA(n, align, pool, name, flags)` (main) and the scene
    /// buffer's name.
    pub const SG_MALLOC_A: u32 = 0x0083_f0a0;
    pub const SCENE_NAME: u32 = 0x00b5_4940;
    /// The scene read's completion callback (`0x00783dd0`, the CD object
    /// calls it).
    pub const SCENE_READ_DONE: u32 = 0x0078_3dd0;
    /// `Ch_SetHideEvent(chr, flags)` and `Ch_SetPos3(chr, x, y, z)` (chara).
    pub const CH_SET_HIDE_EVENT: u32 = 0x0082_b2c0;
    pub const CH_SET_POS3: u32 = 0x0082_b480;

    /// `MsBtlReadScene(file)` (`0x00783d80`): `file` of directory 0x29 (CD
    /// methods `+0x10`, `+0x14`) into a new block (`Sg_MallocA(n, 0x10, 5,
    /// name, -1)`, `g_BtlRead3`), read asynchronously (method `+0`, with
    /// [`SCENE_READ_DONE`] and 0). The read's result.
    pub fn ms_btl_read_scene<M: Mem>(mem: &mut M, call: Call<'_, M>, file: u32) -> u32 {
        let cd = |mem: &mut M, call: Call<'_, M>, slot: u32, args: &[u32]| {
            let obj = game(mem, &mut *call, crate::getmem::bridge::GET_CD, &[]);
            let f: u32 = mem.read(obj.wrapping_add(slot));
            call(mem, f, args)
        };
        cd(mem, &mut *call, 0x10, &[0x29]);
        let n = cd(mem, &mut *call, 0x14, &[file]);
        let name = code(mem, SCENE_NAME);
        let p = game(mem, &mut *call, SG_MALLOC_A, &[n, 0x10, 5, name, u32::MAX]);
        crate::misc::bridge::BTL_READ3.set(mem, p);
        let done = code(mem, SCENE_READ_DONE);
        cd(mem, call, 0, &[file, p, done, 0])
    }

    /// The motion `MsGetRamChrCTBbasic` waits on.
    pub const CTB_WAIT_MOTION: u8 = 0xd0;

    /// `MsGetRamChrCTBbasic()` (`0x00784220`): the first of the 0x1f actors
    /// still loading (`+0x433`) goes to `g_BtlEffectWait` with motion 0xd0:
    /// -1; 0 when none is.
    pub fn ms_get_ram_chr_ctb_basic(mem: &mut impl Mem) -> i32 {
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if mem.read::<u8>(a.wrapping_add(0x433)) != 0 {
                #[allow(clippy::cast_possible_truncation)] // below 0x1f
                crate::flow::bridge::EFFECT_WAIT.set(mem, [i as u8, CTB_WAIT_MOTION]);
                return -1;
            }
        }
        0
    }

    /// `MsPlaceMonsters_00784420()`: each monster (actors 0x14-0x1b) with
    /// a model is shown to the event (`Ch_SetHideEvent(chr, 0)`), placed at
    /// its battle position (`MsGetChrBtlPos`, `Ch_SetPos3`) and, unless
    /// `+0x407`, made visible (`MsSetChrVisibleAppear(i, 0)`). The count.
    pub fn ms_place_monsters<'d>(mem: &mut Dm<'d>, host: &Host<'_>, call: Call<'_, Dm<'d>>) -> u32 {
        use ffx_bridge::x87::{load, store};
        mem.locals(0x10, |mem, v| {
            let mut n = 0;
            for i in 0x14..0x1c {
                let a = ms_get_chr(mem, i);
                let chr: u32 = mem.read(a);
                if chr == 0 {
                    continue;
                }
                n += 1;
                crate::pos::bridge::ms_get_chr_btl_pos(mem, host.trig, i, v);
                game(mem, &mut *call, CH_SET_HIDE_EVENT, &[chr, 0]);
                let p: [u32; 3] = mem.read(v);
                let p = p.map(|w| store(load(w)));
                game(mem, &mut *call, CH_SET_POS3, &[chr, p[0], p[1], p[2]]);
                if mem.read::<u8>(a.wrapping_add(0x407)) == 0 {
                    crate::visible::bridge::ms_set_chr_visible_appear(mem, &mut *call, i, 0);
                }
            }
            n
        })
    }

    /// `MsCalcChrDirAll()` (`0x00793cb0`): each of the 0x1f actors faces
    /// its direction ([`crate::exchange::bridge::ms_calc_chr_dir`]): its
    /// directions `+0x514`, `+0x518` and `+0x49c`. With the caller's
    /// registers (`regs`) the direction's own caller words are this
    /// function's (`i`, the actor and the caller's `ebx`).
    pub fn ms_calc_chr_dir_all(mem: &mut Dm<'_>, host: &Host<'_>, regs: Option<Regs>) {
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            let inner = regs.map(|r| Regs {
                edi: i,
                esi: a,
                ebx: r.ebx,
            });
            let d = crate::exchange::bridge::ms_calc_chr_dir(mem, host, i, inner);
            for o in [0x514, 0x518, 0x49c] {
                mem.write(a.wrapping_add(o), d);
            }
        }
    }

    /// `MsSetChrVisibleEvent(i, flags)` (`0x007967f0`): actor `i`'s model
    /// (1 when it has one) and its two weapons' models get the event hide
    /// flags (`Ch_SetHideEvent`).
    pub fn ms_set_chr_visible_event<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        flags: u32,
    ) -> u32 {
        let i = i & 0xff;
        let chr: u32 = mem.read(ms_get_chr(mem, i));
        let mut r = 0;
        if chr != 0 {
            game(mem, &mut *call, CH_SET_HIDE_EVENT, &[chr, flags]);
            r = 1;
        }
        for k in 0..2 {
            let w = crate::actor::bridge::ms_get_weapon(mem, i, k);
            if w == 0 {
                continue;
            }
            let m: u32 = mem.read(w);
            if m != 0 {
                game(mem, &mut *call, CH_SET_HIDE_EVENT, &[m, flags]);
            }
        }
        r
    }
}
