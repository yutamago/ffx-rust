//! Native functions: what `func` calls.
//!
//! A function ID splits into a group (`id >> 12`) and an index
//! (`id & 0xfff`) into the group's table of 16-byte entries.
//! `AtelInitTotal` (`0x0086d6d0`) registers the tables below; other
//! groups get an empty one. An entry has four handler slots, used by the
//! `func` opcodes in `Script_Run`:
//!
//! | Slot | Use |
//! |---|---|
//! | 0 | start, called once (`Script_CallNative`, `0x008777a0`) |
//! | 1 | polled until it reports completion (`0x00877730`); lets a call span frames |
//! | 2 | returns a float result (`0x008777f0`) |
//! | 3 | returns an integer result |
//!
//! The handler addresses are read from the player's `FFX.exe`.

/// (group, table address in `FFX.exe`), from `AtelInitTotal`.
pub const GROUP_TABLES: [(u16, u32); 11] = [
    (0x0, 0x00c5_0050),
    (0x1, 0x00c5_2be0),
    (0x4, 0x00c8_8d88),
    (0x5, 0x00c8_91f8),
    (0x6, 0x00c4_3998),
    (0x7, 0x00c4_2628),
    (0x8, 0x00c5_dc90),
    (0x9, 0x00c5_d8c0),
    (0xb, 0x00c4_0e30),
    (0xc, 0x00c5_2dd8),
    (0xd, 0x00c8_5eb0),
];

const IMAGE_BASE: u32 = 0x40_0000;
const ENTRY_LEN: u32 = 16;

/// Resolves native function IDs to handler addresses in an `FFX.exe`.
pub struct Natives {
    exe: Vec<u8>,
    /// (virtual address, size, file offset) of each section.
    sections: Vec<(u32, u32, u32)>,
}

impl Natives {
    /// Reads the section table of a PE32 image.
    #[must_use]
    pub fn from_exe(exe: Vec<u8>) -> Option<Self> {
        let u16_at = |o: usize| exe.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        let u32_at = |o: usize| {
            exe.get(o..o + 4)
                .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
        };
        let nt = u32_at(0x3c)? as usize;
        let count = usize::from(u16_at(nt + 6)?);
        let table = nt + 0x18 + usize::from(u16_at(nt + 0x14)?);
        let sections = (0..count)
            .map(|i| {
                let s = table + i * 40;
                Some((u32_at(s + 12)?, u32_at(s + 16)?, u32_at(s + 20)?))
            })
            .collect::<Option<_>>()?;
        Some(Self { exe, sections })
    }

    fn read_u32(&self, va: u32) -> Option<u32> {
        let rva = va.checked_sub(IMAGE_BASE)?;
        let &(start, _, file) = self
            .sections
            .iter()
            .find(|&&(s, len, _)| (s..s + len).contains(&rva))?;
        let at = (file + rva - start) as usize;
        self.exe
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
    }

    /// The four handler slots of `func id` (start, poll, float result,
    /// integer result), as `FFX.exe` addresses.
    #[must_use]
    pub fn slots(&self, id: u16) -> Option<[Option<u32>; 4]> {
        let &(_, table) = GROUP_TABLES.iter().find(|(g, _)| *g == id >> 12)?;
        let entry = table + u32::from(id & 0xfff) * ENTRY_LEN;
        let mut slots = [None; 4];
        for (i, slot) in slots.iter_mut().enumerate() {
            let handler = self.read_u32(entry + 4 * u32::try_from(i).expect("4 slots"))?;
            // Only addresses in .text are handlers; past a table's end lies other data.
            *slot = (0x0040_1000..0x00b0_bc00)
                .contains(&handler)
                .then_some(handler);
        }
        Some(slots)
    }

    /// The handler that does the call's work: the result function if there
    /// is one, else the start function.
    #[must_use]
    pub fn handler(&self, id: u16) -> Option<u32> {
        let [start, poll, float, int] = self.slots(id)?;
        int.or(float).or(start).or(poll)
    }
}
