//! Gear names (`docs/rewrite-plan.md`, M2): `MsWeaponNameNum`
//! (`0x007a0cf0`) picks the name a piece of gear gets from its
//! auto-abilities, a weapon's in `0x007a1c00` and armour's in `0x007a0d10`.
//! The rules are a fixed order of tests; the first that holds names the
//! gear (`docs/formats/battle.md`, "Gear names").
//!
//! A gear record (the save's and the spoils' `0x16` bytes) as these read
//! it: `+3` flags (weapons: 4 and 8 the two named-weapon kinds), `+5` 0 for
//! a weapon, `+0xb` the slot count, `+0xe` four `u16` auto-abilities
//! (`0x8000 | n`, 0xff for an empty slot).

/// The auto-abilities the rules name (`0x8000 | n`, the kernel's
/// `a_ability.bin` record `n`).
#[allow(missing_docs)]
pub mod ability {
    pub const SENSOR: u16 = 0x8000;
    pub const FIRST_STRIKE: u16 = 0x8001;
    pub const INITIATIVE: u16 = 0x8002;
    pub const COUNTERATTACK: u16 = 0x8003;
    pub const EVADE_COUNTER: u16 = 0x8004;
    pub const MAGIC_COUNTER: u16 = 0x8005;
    pub const MAGIC_BOOSTER: u16 = 0x8006;
    pub const ALCHEMY: u16 = 0x8007;
    pub const AUTO_POTION: u16 = 0x8008;
    pub const AUTO_MED: u16 = 0x8009;
    pub const AUTO_PHOENIX: u16 = 0x800a;
    pub const PIERCING: u16 = 0x800b;
    pub const HALF_MP_COST: u16 = 0x800c;
    pub const ONE_MP_COST: u16 = 0x800d;
    pub const DOUBLE_OVERDRIVE: u16 = 0x800e;
    pub const TRIPLE_OVERDRIVE: u16 = 0x800f;
    pub const SOS_OVERDRIVE: u16 = 0x8010;
    pub const OVERDRIVE_TO_AP: u16 = 0x8011;
    pub const DOUBLE_AP: u16 = 0x8012;
    pub const TRIPLE_AP: u16 = 0x8013;
    pub const PICKPOCKET: u16 = 0x8015;
    pub const MASTER_THIEF: u16 = 0x8016;
    pub const BREAK_HP_LIMIT: u16 = 0x8017;
    pub const BREAK_MP_LIMIT: u16 = 0x8018;
    pub const BREAK_DAMAGE_LIMIT: u16 = 0x8019;
    pub const GILLIONAIRE: u16 = 0x801a;
    pub const HP_STROLL: u16 = 0x801b;
    pub const MP_STROLL: u16 = 0x801c;
    pub const NO_ENCOUNTERS: u16 = 0x801d;
    /// The four elements' strike, ward, proof and eater (fire, ice,
    /// lightning, water) follow each other from here.
    pub const FIRESTRIKE: u16 = 0x801e;
    pub const DEATHSTRIKE: u16 = 0x802e;
    /// The status strikes and touches (death, zombie, stone, poison,
    /// sleep, silence, dark, slow: strike, touch, proof, ward) follow
    /// each other from [`DEATHSTRIKE`]; `CONFUSEPROOF` on come proof and
    /// ward pairs (confuse, berserk, curse).
    pub const CONFUSEPROOF: u16 = 0x804e;
    pub const BERSERKPROOF: u16 = 0x8050;
    pub const CURSEPROOF: u16 = 0x8052;
    pub const AUTO_SHELL: u16 = 0x8054;
    pub const AUTO_PROTECT: u16 = 0x8055;
    pub const AUTO_HASTE: u16 = 0x8056;
    pub const AUTO_REGEN: u16 = 0x8057;
    pub const AUTO_REFLECT: u16 = 0x8058;
    pub const SOS_SHELL: u16 = 0x8059;
    pub const SOS_PROTECT: u16 = 0x805a;
    pub const SOS_HASTE: u16 = 0x805b;
    pub const SOS_REGEN: u16 = 0x805c;
    pub const SOS_REFLECT: u16 = 0x805d;
    pub const SOS_NULTIDE: u16 = 0x805e;
    pub const SOS_NULFROST: u16 = 0x805f;
    pub const SOS_NULSHOCK: u16 = 0x8060;
    pub const SOS_NULBLAZE: u16 = 0x8061;
    /// Strength, Magic, Defense, Magic Def +3/+5/+10/+20% and HP, MP
    /// +5/+10/+20/+30%, four each from here.
    pub const STRENGTH_3: u16 = 0x8062;
    pub const MAGIC_3: u16 = 0x8066;
    pub const DEFENSE_3: u16 = 0x806a;
    pub const MAGIC_DEF_3: u16 = 0x806e;
    pub const HP_5: u16 = 0x8072;
    pub const MP_5: u16 = 0x8076;
    pub const CAPTURE: u16 = 0x807a;
    pub const DISTILL_POWER: u16 = 0x807c;
    pub const DISTILL_MANA: u16 = 0x807d;
    pub const DISTILL_SPEED: u16 = 0x807e;
    pub const DISTILL_ABILITY: u16 = 0x807f;
    pub const RIBBON: u16 = 0x8080;
}

#[allow(clippy::wildcard_imports)] // the rules read as the abilities' names
use ability::*;

/// The elements in the order of their abilities: fire, ice, lightning,
/// water (strike at `+0`, ward `+1`, proof `+2`, eater `+3`).
const ELEMENTS: [u16; 4] = [FIRESTRIKE, FIRESTRIKE + 4, FIRESTRIKE + 8, FIRESTRIKE + 12];
/// The statuses of the strikes and touches: death, zombie, stone, poison,
/// sleep, silence, dark, slow (strike `+0`, touch `+1`, proof `+2`, ward
/// `+3`).
const STATUSES: [u16; 8] = [
    DEATHSTRIKE,
    DEATHSTRIKE + 4,
    DEATHSTRIKE + 8,
    DEATHSTRIKE + 12,
    DEATHSTRIKE + 16,
    DEATHSTRIKE + 20,
    DEATHSTRIKE + 24,
    DEATHSTRIKE + 28,
];
const DEATH: u16 = STATUSES[0];
const STONE: u16 = STATUSES[2];
const POISON: u16 = STATUSES[3];
const SLEEP: u16 = STATUSES[4];
const SILENCE: u16 = STATUSES[5];
const DARK: u16 = STATUSES[6];
const SLOW: u16 = STATUSES[7];

/// What the naming rules read of a gear record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Gear {
    /// `+3`: a weapon's named kinds (4, 8).
    pub flags: u8,
    /// `+0xb`: the slot count.
    pub slots: u8,
    /// `+0xe`: the four auto-abilities.
    pub abilities: [u16; 4],
}

impl Gear {
    /// `MsGearHasAbility_007a0c20(gear, id)`: one of the four slots holds
    /// `id`.
    #[must_use]
    pub fn has(&self, id: u16) -> bool {
        self.abilities.contains(&id)
    }

    /// How many of `ids` the gear holds.
    fn count(&self, ids: &[u16]) -> u32 {
        ids.iter().map(|&id| u32::from(self.has(id))).sum()
    }

    /// The four abilities from `first` on.
    fn count4(&self, first: u16) -> u32 {
        self.count(&[first, first + 1, first + 2, first + 3])
    }

    /// The gear holds either of two abilities.
    fn either(&self, a: u16, b: u16) -> bool {
        self.has(a) || self.has(b)
    }

    /// How many of the pairs `(first + k, first + k + 1)` for `first` in
    /// `firsts` the gear holds either of.
    fn pairs(&self, firsts: &[u16], k: u16) -> u32 {
        firsts
            .iter()
            .map(|&f| u32::from(self.either(f + k, f + k + 1)))
            .sum()
    }
}

/// `MsWeaponNameNum_Weapon_007a1c00(gear)`: a weapon's name number
/// (`0x5000`-`0x5046`).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn weapon_name(g: &Gear) -> u16 {
    let strength = g.count4(STRENGTH_3);
    let magic = g.count4(MAGIC_3);
    let strikes = g.count(&ELEMENTS);
    // the status strikes but zombie's
    let status_strikes = g.count(&[SLOW, DARK, SILENCE, SLEEP, POISON, STONE, DEATH]);
    if g.flags & 4 != 0 {
        return 0x5000;
    }
    if g.flags & 8 != 0 {
        return 0x5001;
    }
    if g.has(CAPTURE) {
        return 0x5002;
    }
    if strikes > 3 {
        return 0x5003;
    }
    if g.has(BREAK_DAMAGE_LIMIT) {
        return 0x5004;
    }
    let (od3, od2, ap3, ap2, od_ap) = (
        g.has(TRIPLE_OVERDRIVE),
        g.has(DOUBLE_OVERDRIVE),
        g.has(TRIPLE_AP),
        g.has(DOUBLE_AP),
        g.has(OVERDRIVE_TO_AP),
    );
    for (holds, name) in [
        (od3 && ap3 && od_ap, 0x5005),
        (od3 && od_ap, 0x5006),
        (od2 && ap2, 0x5007),
        (od3, 0x5008),
        (od2, 0x5009),
        (ap3, 0x500a),
        (ap2, 0x500b),
        (od_ap, 0x500c),
        (g.has(SOS_OVERDRIVE), 0x500d),
        (g.has(ONE_MP_COST), 0x500f),
        (status_strikes > 3, 0x5010),
        (strength > 3, 0x5013),
        (magic > 3, 0x5014),
        (magic > 2 && g.has(MAGIC_BOOSTER), 0x5015),
        (g.has(HALF_MP_COST), 0x5016),
        (g.has(GILLIONAIRE), 0x5017),
        (strikes > 2, 0x5018),
        (status_strikes > 2, 0x5019),
        (
            g.either(COUNTERATTACK, EVADE_COUNTER) && g.has(MAGIC_COUNTER),
            0x501a,
        ),
        (g.either(COUNTERATTACK, EVADE_COUNTER), 0x501b),
        (g.has(MAGIC_COUNTER), 0x5020),
        (g.has(MAGIC_BOOSTER), 0x5021),
        (g.has(ALCHEMY), 0x5022),
        (g.has(FIRST_STRIKE), 0x5023),
        (g.has(INITIATIVE), 0x5024),
        (g.has(DEATH), 0x5025),
        (g.has(SLOW), 0x5026),
        (g.has(STONE), 0x5027),
        (g.has(POISON), 0x5028),
        (g.has(SLEEP), 0x5029),
        (g.has(SILENCE), 0x502a),
        (g.has(DARK), 0x502b),
        (strength > 2, 0x502c),
        (magic > 2, 0x502d),
        (strikes > 1, 0x502e),
        // strikes or touches of two statuses (zombie's not counted)
        (
            g.pairs(&[DEATH, SLOW, STONE, POISON, SLEEP, SILENCE, DARK], 0) > 1,
            0x502f,
        ),
        (g.has(DEATH + 1), 0x5030),
        (g.has(SLOW + 1), 0x5031),
        (g.has(STONE + 1), 0x5032),
        (g.has(POISON + 1), 0x5033),
        (g.has(SLEEP + 1), 0x5034),
        (g.has(SILENCE + 1), 0x5035),
        (g.has(DARK + 1), 0x5036),
        (g.has(SENSOR), 0x5037),
        (g.has(ELEMENTS[0]), 0x5038),
        (g.has(ELEMENTS[1]), 0x5039),
        (g.has(ELEMENTS[2]), 0x503a),
        (g.has(ELEMENTS[3]), 0x503b),
        (g.has(DISTILL_POWER), 0x501c),
        (g.has(DISTILL_MANA), 0x501d),
        (g.has(DISTILL_SPEED), 0x501e),
        (g.has(DISTILL_ABILITY), 0x501f),
        (g.slots > 3, 0x503c),
        (magic > 0 && strength > 0, 0x503d),
        (g.slots > 1, 0x503e),
        (g.either(MAGIC_3 + 2, MAGIC_3 + 3), 0x503f),
        (g.either(STRENGTH_3 + 2, STRENGTH_3 + 3), 0x5040),
        (g.has(MAGIC_3 + 1), 0x5041),
        (g.has(MAGIC_3), 0x5042),
        (g.has(STRENGTH_3 + 1), 0x5043),
        (g.has(STRENGTH_3), 0x5044),
        (g.has(PIERCING), 0x5045),
    ] {
        if holds {
            return name;
        }
    }
    0x5046
}

/// `MsWeaponNameNum_Armor_007a0d10(gear)`: armour's name number
/// (`0x504a`-`0x509e`).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn armor_name(g: &Gear) -> u16 {
    let autos = g.count(&[AUTO_PHOENIX, AUTO_MED, AUTO_POTION]);
    let eaters = g.count(&ELEMENTS.map(|e| e + 3));
    let proofs = g.count(&ELEMENTS.map(|e| e + 2));
    let auto_statuses = g.count(&[AUTO_REFLECT, AUTO_REGEN, AUTO_PROTECT, AUTO_SHELL]);
    // the eleven status proofs (here zombie's counts)
    let status_proofs = g.count(&[
        CURSEPROOF,
        BERSERKPROOF,
        CONFUSEPROOF,
        SLOW + 2,
        DARK + 2,
        SILENCE + 2,
        SLEEP + 2,
        POISON + 2,
        STONE + 2,
        STATUSES[1] + 2,
        DEATH + 2,
    ]);
    let defense = g.count4(DEFENSE_3);
    let magic_def = g.count4(MAGIC_DEF_3);
    let hp = g.count4(HP_5);
    let mp = g.count4(MP_5);
    let sos_nul = g.count4(SOS_NULTIDE);
    let sos = sos_nul + g.count(&[SOS_REFLECT, SOS_REGEN, SOS_HASTE, SOS_PROTECT, SOS_SHELL]);
    let hastes = u32::from(g.has(AUTO_HASTE)) + auto_statuses;
    for (holds, name) in [
        (g.has(BREAK_HP_LIMIT) && g.has(BREAK_MP_LIMIT), 0x504a),
        (g.has(RIBBON), 0x509e),
        (g.has(BREAK_HP_LIMIT), 0x504b),
        (g.has(BREAK_MP_LIMIT), 0x504c),
        (eaters > 3, 0x504d),
        (proofs > 3, 0x504e),
        (auto_statuses > 3, 0x504f),
        (autos > 2, 0x5050),
        (g.has(AUTO_POTION) && g.has(AUTO_MED), 0x5051),
        (status_proofs > 3, 0x5052),
        (defense > 3, 0x5053),
        (magic_def > 3, 0x5054),
        (hp > 3, 0x5055),
        (mp > 3, 0x5056),
        (g.has(MASTER_THIEF), 0x5057),
        (g.has(PICKPOCKET), 0x5058),
        (g.has(HP_STROLL) && g.has(MP_STROLL), 0x5059),
        (hastes > 2, 0x505a),
        (eaters > 2, 0x505b),
        (g.has(HP_STROLL), 0x505c),
        (g.has(MP_STROLL), 0x505d),
        (g.has(AUTO_PHOENIX), 0x505e),
        (g.has(AUTO_MED), 0x505f),
        (sos_nul > 3, 0x5060),
        (sos > 3, 0x5061),
        (status_proofs > 2, 0x5062),
        (g.has(NO_ENCOUNTERS), 0x5063),
        (g.has(AUTO_POTION), 0x5064),
        (proofs > 2, 0x5065),
        (sos > 2, 0x5066),
        (hastes > 1, 0x5067),
        (sos > 1, 0x5068),
        (g.either(AUTO_REGEN, SOS_REGEN), 0x5069),
        (g.either(AUTO_HASTE, SOS_HASTE), 0x506a),
        (g.either(AUTO_REFLECT, SOS_REFLECT), 0x506b),
        (g.either(AUTO_SHELL, SOS_SHELL), 0x506c),
        (g.either(AUTO_PROTECT, SOS_PROTECT), 0x506d),
        (g.has(ALCHEMY), 0x506e),
        (defense > 2, 0x506f),
        (magic_def > 2, 0x5070),
        (hp > 2, 0x5071),
        (mp > 2, 0x5072),
        // proofs or eaters of two elements
        (g.pairs(&ELEMENTS, 2) > 1, 0x5073),
        (status_proofs > 1, 0x5074),
        (g.has(ELEMENTS[0] + 3), 0x5075),
        (g.has(ELEMENTS[1] + 3), 0x5076),
        (g.has(ELEMENTS[2] + 3), 0x5077),
        (g.has(ELEMENTS[3] + 3), 0x5078),
        (g.either(CURSEPROOF, CURSEPROOF + 1), 0x5079),
        (g.either(CONFUSEPROOF, CONFUSEPROOF + 1), 0x507a),
        (g.either(BERSERKPROOF, BERSERKPROOF + 1), 0x507b),
        (g.either(SLOW + 2, SLOW + 3), 0x507c),
        (g.either(DEATH + 2, DEATH + 3), 0x507d),
        (g.either(STATUSES[1] + 2, STATUSES[1] + 3), 0x507e),
        (g.either(STONE + 2, STONE + 3), 0x507f),
        (g.either(POISON + 2, POISON + 3), 0x5080),
        (g.either(SLEEP + 2, SLEEP + 3), 0x5081),
        (g.either(SILENCE + 2, SILENCE + 3), 0x5082),
        (g.either(DARK + 2, DARK + 3), 0x5083),
        // an element's proof or ward
        (g.either(ELEMENTS[0] + 2, ELEMENTS[0] + 1), 0x5084),
        (g.either(ELEMENTS[1] + 2, ELEMENTS[1] + 1), 0x5085),
        (g.either(ELEMENTS[2] + 2, ELEMENTS[2] + 1), 0x5086),
        (g.either(ELEMENTS[3] + 2, ELEMENTS[3] + 1), 0x5087),
        (g.has(SOS_NULTIDE), 0x5088),
        (g.has(SOS_NULBLAZE), 0x5089),
        (g.has(SOS_NULSHOCK), 0x508a),
        (g.has(SOS_NULFROST), 0x508b),
        (hp + mp > 3, 0x508c),
        (g.slots > 3, 0x508d),
        (defense > 0 && magic_def > 0, 0x508e),
        (defense > 1, 0x508f),
        (magic_def > 1, 0x5090),
        (hp > 1, 0x5091),
        (mp > 1, 0x5092),
        (g.either(DEFENSE_3 + 2, DEFENSE_3 + 3), 0x5093),
        (g.either(MAGIC_DEF_3 + 2, MAGIC_DEF_3 + 3), 0x5094),
        (g.either(MP_5 + 2, MP_5 + 3), 0x5095),
        (g.either(HP_5 + 2, HP_5 + 3), 0x5096),
        (g.slots > 2, 0x5097),
        (g.either(DEFENSE_3, DEFENSE_3 + 1), 0x5098),
        (g.either(MAGIC_DEF_3, MAGIC_DEF_3 + 1), 0x5099),
        (g.either(MP_5, MP_5 + 1), 0x509a),
        (g.either(HP_5, HP_5 + 1), 0x509b),
    ] {
        if holds {
            return name;
        }
    }
    if g.slots < 2 { 0x509d } else { 0x509c }
}

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::Gear;

    /// The gear record at `at`.
    pub fn gear(mem: &impl Mem, at: u32) -> Gear {
        Gear {
            flags: mem.read(at.wrapping_add(3)),
            slots: mem.read(at.wrapping_add(0xb)),
            abilities: std::array::from_fn(|k| {
                #[allow(clippy::cast_possible_truncation)] // k < 4
                mem.read(at.wrapping_add(0xe + 2 * k as u32))
            }),
        }
    }

    /// `MsGearHasAbility_007a0c20(gear, id)`: 1 when one of the gear's four
    /// abilities is `id` (its low 16 bits), else 0.
    pub fn ms_gear_has_ability(mem: &impl Mem, at: u32, id: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the original compares words
        u32::from(gear(mem, at).has(id as u16))
    }

    /// `MsWeaponNameNum_Weapon_007a1c00(gear)`.
    pub fn ms_weapon_name_num_weapon(mem: &impl Mem, at: u32) -> u32 {
        super::weapon_name(&gear(mem, at)).into()
    }

    /// `MsWeaponNameNum_Armor_007a0d10(gear)`.
    pub fn ms_weapon_name_num_armor(mem: &impl Mem, at: u32) -> u32 {
        super::armor_name(&gear(mem, at)).into()
    }

    /// `MsWeaponNameNum(gear)` (`0x007a0cf0`): a weapon's name (`+5` 0) or
    /// armour's.
    pub fn ms_weapon_name_num(mem: &impl Mem, at: u32) -> u32 {
        if mem.read::<u8>(at.wrapping_add(5)) == 0 {
            ms_weapon_name_num_weapon(mem, at)
        } else {
            ms_weapon_name_num_armor(mem, at)
        }
    }
}
