//! `FFXApplication`'s methods (`FFXApplication.cpp`, the game's
//! `Phyre::PFramework::PApplication`): the ones that only pass the call
//! on to the engine.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// `0x00642cd0`: `PhyreScene_resize` on the game's scene
    /// (`mov ecx, [0x00ccb9d8]` and a jump; cdecl, no arguments).
    pub const SCENE_RESIZE: u32 = 0x0064_2cd0;
    /// `0x0062c260`: the base class's resize (`PApplication::resize`,
    /// thiscall without stack arguments): the viewport and the render
    /// targets at the new size, its result in `eax`.
    pub const APPLICATION_RESIZE: u32 = 0x0062_c260;

    /// `FFXApplication::resize(this)` (`0x0042f9a0`, vtable slot 9 of
    /// `FFXApplication_vftable`; name ours, after `render` in the class's
    /// method order): the scene's resize, then the base class's on `this`
    /// (a tail jump), whose `eax` it returns. `call` runs a cdecl
    /// original, `thiscall` a thiscall one (`this`, stack words).
    pub fn resize<M: Mem>(
        mem: &mut M,
        this: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        thiscall: &mut impl FnMut(&mut M, u32, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(SCENE_RESIZE);
        call(mem, f, &[]);
        let f = mem.rebase(APPLICATION_RESIZE);
        thiscall(mem, f, this, &[])
    }
}
