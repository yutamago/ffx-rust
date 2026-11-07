//! M5 stage 1, round 9 (`tail9`): the ability map's job kicks
//! (`0x00a8ed00`, `0x00a8ed20`, `0x00a8ed40`, `0x00a8ed60`, cdecl, no
//! arguments), which `eiAbmDraw` calls: one job pushed onto queue 0 of
//! the scheduler at `0x00cdede4`; their wait (`0x00a8ecf0`) is a copy of
//! `JobScheduleWait_Ch_Cloth_PerCharacter`
//! ([`crate::m5a_platform::bridge::wait_cloth`]).

/// The originals over game memory.
pub mod bridge {
    use crate::m5a_platform::bridge::{CLOTH_SCHEDULER, push_job};
    use ffx_graphic::host::Host;

    /// The jobs the four kicks push (code addresses), by kick:
    /// `0x00a8ed00` -> `0x00a53450`, `0x00a8ed20` -> `0x00a534b0`,
    /// `0x00a8ed40` -> `0x00a534c0`, `0x00a8ed60` -> `0x00a53510`.
    pub const KICKS: [(u32, u32); 4] = [
        (0x00a8_ed00, 0x00a5_3450),
        (0x00a8_ed20, 0x00a5_34b0),
        (0x00a8_ed40, 0x00a5_34c0),
        (0x00a8_ed60, 0x00a5_3510),
    ];

    /// One kick (cdecl `()`) -> `JobScheduler::pushJob`'s `eax`:
    /// `pushJob(job, 0)` on the scheduler `0x00cdede4` holds; `slot` is
    /// the kick's own argument word for the queue, which `pushJob`
    /// overwrites with the queue's address (`[esp_at_call - 8]`).
    pub fn kick(h: &mut impl Host, job: u32, slot: u32) -> u32 {
        let scheduler: u32 = h.read(h.rebase(CLOTH_SCHEDULER));
        let job = h.rebase(job);
        push_job(h, scheduler, job, 0, Some(slot))
    }
}
