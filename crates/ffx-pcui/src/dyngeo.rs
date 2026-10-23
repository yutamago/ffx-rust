//! Two shader setters of the remaster's `ClassDynamicGeometry` (vtable
//! slots at `0x00b47e44` and `0x00b47e74`): the shader looked up in
//! `VfxShaderManager` by the geometry's key, then handed to `0x006a46e0`.

/// The maps whose dynamic geometry sets the scene flag `g_PhyreScene +
/// 0xf9c6` (`0x006a4800`): 0x10a, 0xdf, 0xcd, 0xdd.
#[must_use]
pub fn flag_map(map: u32) -> bool {
    matches!(map, 0x10a | 0xdf | 0xcd | 0xdd)
}

/// The shader name `0x006a48d0` makes of a material path: what follows
/// its first `"GL/"`, or `None` without one.
#[must_use]
pub fn gl_name(path: &[u8]) -> Option<&[u8]> {
    let at = path.windows(3).position(|w| w == b"GL/")?;
    Some(&path[at + 3..])
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    use super::{flag_map, gl_name};

    /// `VfxShaderManager_sGetInstance()` and its lookup `0x006c06e0(this,
    /// name, &key)` (`thiscall`, unnamed: by name for a manager keyed by
    /// name, else by the key word).
    pub const SHADER_MANAGER: u32 = 0x006c_14c0;
    pub const SHADER_FIND: u32 = 0x006c_06e0;
    /// `0x006a46e0(this, shader, target)` (`thiscall`, unnamed).
    pub const SET_SHADER: u32 = 0x006a_46e0;
    /// `g_PhyreScene` and `g_CurrentMapNo`.
    pub const PHYRE_SCENE: Global<u32> = Global::new(0x00cc_b9d8);
    pub const CURRENT_MAP: Global<u32> = Global::new(0x012f_bc40);
    /// Bytes of work the two setters need, laid out as the original's
    /// frame from `ebp - 0x88`: the fallback key, then the name buffer
    /// (0x80 bytes).
    pub const WORK: u32 = 0x84;
    const NAME: u32 = 4;

    /// The lookup both setters share: by the key `+0x3c`, else by 8 or 0
    /// (bit 3 of the key as it was before the first lookup); the result
    /// into `+0x20`.
    fn find_shader(mem: &mut impl Host, this: u32, key: u32, work: u32) {
        let name = work + NAME;
        let mgr = mem.call_sym(SHADER_MANAGER, Conv::Cdecl, &[]);
        let at = this.wrapping_add(0x3c);
        let mut shader = mem.call_sym(SHADER_FIND, Conv::Thiscall(mgr), &[name, at]);
        mem.write(this.wrapping_add(0x20), shader);
        if shader == 0 {
            mem.write(work, if key & 8 == 0 { 0u32 } else { 8 });
            let mgr = mem.call_sym(SHADER_MANAGER, Conv::Cdecl, &[]);
            shader = mem.call_sym(SHADER_FIND, Conv::Thiscall(mgr), &[name, work]);
            mem.write(this.wrapping_add(0x20), shader);
        }
    }

    /// `ClassDynamicGeometry_SetShader_006a4800(this, _, target)` (name
    /// ours; `thiscall`, `ret 8`): the shader looked up (the name buffer is
    /// never filled: the original passes its stack as it is, a zeroed
    /// buffer here; only a manager keyed by name reads it), the scene flag
    /// `+0xf9c6` set for the maps of [`flag_map`] and cleared for the
    /// rest, then `0x006a46e0(this, shader, target)`, `target` 0 meaning
    /// `g_PhyreScene + 0x2874`. `eax` is the stack cookie check's.
    pub fn set_shader(mem: &mut impl Host, this: u32, target: u32, work: u32) {
        let key: u32 = mem.read(this.wrapping_add(0x3c));
        find_shader(mem, this, key, work);
        let scene = PHYRE_SCENE.get(mem);
        let target = if target == 0 {
            scene.wrapping_add(0x2874)
        } else {
            target
        };
        let on = flag_map(CURRENT_MAP.get(mem));
        mem.write(scene.wrapping_add(0xf9c6), u8::from(on));
        let shader: u32 = mem.read(this.wrapping_add(0x20));
        mem.call_sym(SET_SHADER, Conv::Thiscall(this), &[shader, target]);
    }

    /// `ClassDynamicGeometry_SetShaderGl_006a48d0(this, _, target)` (name
    /// ours; `thiscall`, `ret 8`): the material path (`[[+0x24] + 0x94] +
    /// 0x20`) after its first `"GL/"` ([`gl_name`]) as the lookup's name
    /// (left as it was without one; at most 0x7f bytes here, where the
    /// original overruns its frame), the shader looked up, then
    /// `0x006a46e0(this, shader, target)`, `target` 0 meaning
    /// `g_PhyreScene + 0x12d4`. `eax` is the stack cookie check's.
    pub fn set_shader_gl(mem: &mut impl Host, this: u32, target: u32, work: u32) {
        let key: u32 = mem.read(this.wrapping_add(0x3c));
        let mat: u32 = mem.read(this.wrapping_add(0x24));
        let info: u32 = mem.read(mat.wrapping_add(0x94));
        let path: u32 = mem.read(info.wrapping_add(0x20));
        let mut text = Vec::new();
        loop {
            let c: u8 = mem.read(path.wrapping_add(u32::try_from(text.len()).unwrap_or(u32::MAX)));
            if c == 0 {
                break;
            }
            text.push(c);
        }
        if let Some(name) = gl_name(&text) {
            let n = name.len().min(0x7f);
            let mut out = name[..n].to_vec();
            out.push(0);
            mem.write_bytes(work + NAME, &out);
        }
        find_shader(mem, this, key, work);
        let target = if target == 0 {
            PHYRE_SCENE.get(mem).wrapping_add(0x12d4)
        } else {
            target
        };
        let shader: u32 = mem.read(this.wrapping_add(0x20));
        mem.call_sym(SET_SHADER, Conv::Thiscall(this), &[shader, target]);
    }
}
