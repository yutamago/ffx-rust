//! The render-side glue, twenty-third batch (M4 port round 6, list
//! `h3-glue-render`): the shadow manager's shader-bin load and the VFX
//! shader lookup by texture name. The cookie is left out, as in
//! [`crate::glue10`].

/// The shadow shader bins: their path table, `fiosUnifyFilename(path,
/// out, size)`, `Phyre_PCluster_LoadAssetFile(&cluster, path, 0)`,
/// `PApplication::GetApplication`, `PApplication_FixupClusters(&cluster,
/// 1)` (cdecl), `PhyreSceneCursor_Seek3(this)` and the class it seeks.
pub const SHADOW_BIN_PATHS: u32 = 0x00c3_43fc;
pub const SHADOW_BIN_COUNT: u32 = 1;
pub const FIOS_UNIFY_FILENAME: u32 = 0x0067_9820;
pub const LOAD_ASSET_FILE: u32 = 0x0044_06c0;
pub const GET_APPLICATION: u32 = crate::glue::GET_APPLICATION;
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
pub const CURSOR_SEEK3: u32 = 0x0065_ec40;
pub const SHADER_CLASS: u32 = 0x00ca_5ff0;

/// The VFX shader lookup: `Phyre_PhyreMalloc_0042fb80(size, align)`,
/// `Phyre_PhyreFree(p)` (cdecl), `Phyre_PString_PString(this, text)`,
/// `PStringTree_Find(head, &key)` and `UIntTree_Find(head, &key)`.
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const PSTRING_CTOR: u32 = 0x0042_f090;
pub const PSTRING_TREE_FIND: u32 = 0x006b_e750;
pub const UINT_TREE_FIND: u32 = 0x006b_e700;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// `ShadowManager_loadShadowShaderBinFiles(this)` (`ecx`; frame: the
    /// unified path at `ebp - 0x104`, a cursor at `ebp - 0x124`): each
    /// shader bin (one) is loaded into the cluster word `this + 4k` and
    /// fixed up, and must hold an object of the shader class. `eax`: 0,
    /// the loader's or the fix-up's error, or 0x13 (no cluster, or no
    /// shader in it).
    pub fn shadow_load_shader_bins(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let paths = h.rebase(SHADOW_BIN_PATHS);
        let path = ebp - 0x104;
        let cursor = ebp - 0x124;
        for k in 0..SHADOW_BIN_COUNT {
            let slot = this.wrapping_add(4 * k);
            let name: u32 = h.read(paths + 4 * k);
            h.call_sym(FIOS_UNIFY_FILENAME, Conv::Cdecl, &[name, path, 0x100]);
            let error = h.call_sym(LOAD_ASSET_FILE, Conv::Cdecl, &[slot, path, 0]);
            if error != 0 {
                return error;
            }
            h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            let error = h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[slot, 1]);
            if error != 0 {
                return error;
            }
            let cluster: u32 = h.read(slot);
            if cluster == 0 {
                return 0x13;
            }
            crate::h3_glue14::bridge::cursor_over(h, cursor, cluster, SHADER_CLASS);
            h.call_sym(CURSOR_SEEK3, Conv::Thiscall(cursor), &[]);
            if h.read::<u32>(cursor + 0xc) == 0 {
                return 0x13;
            }
            let at = h
                .read::<u32>(cursor + 0x10)
                .wrapping_add(h.read::<u32>(cursor));
            if at == 0 {
                return 0x13;
            }
        }
        0
    }

    /// An entry of a tree node (`+0x10`), or 0 without a node.
    fn entry(node: u32) -> u32 {
        if node == 0 {
            0
        } else {
            node.wrapping_add(0x10)
        }
    }

    /// Frees a `PString`'s text unless its low bit marks it borrowed.
    fn free_text(h: &mut impl Host, text: u32) {
        if text & 1 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[text]);
        }
    }

    /// `VfxShaderManager_FindShader(this, name, flags; slots)` (`ret 8`;
    /// frame: `this`, the copy's size, then the copy, at `ebp - 4`): the special
    /// shaders by name, first in the copied-name table (`+0x30`, with
    /// `+0x60`), then in the `PString` one (`+0x68`, with `+0x98`; the
    /// `PString` built over `name`'s slot): a hit sets `*flags` to -1 and
    /// gives the shader's first word. Otherwise the table by number
    /// (`this`, keyed by `*flags`, which goes over `flags`' slot) gives
    /// its entry's word, or 0.
    pub fn vfx_find_shader(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        name: u32,
        flags: u32,
    ) -> u32 {
        // the frame word is reserved with `push ecx`
        h.write(ebp - 4, this);
        if h.read::<u8>(this + 0x60) != 0 {
            let copy = if name == 0 {
                0
            } else {
                let mut len = 0u32;
                while h.read::<u8>(name.wrapping_add(len)) != 0 {
                    len += 1;
                }
                let size = len + 1;
                h.write(ebp - 4, size);
                let copy = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 2]);
                if copy != 0 {
                    let mut bytes = vec![0; size as usize];
                    h.read_bytes(name, &mut bytes);
                    h.write_bytes(copy, &bytes);
                }
                copy
            };
            h.write(ebp - 4, copy);
            let node = h.call_sym(PSTRING_TREE_FIND, Conv::Thiscall(this + 0x30), &[ebp - 4]);
            let found = entry(node);
            free_text(h, copy);
            if found != 0 {
                h.write(flags, u32::MAX);
                let shader: u32 = h.read(found);
                return h.read(shader);
            }
        }
        if h.read::<u8>(this + 0x98) != 0 {
            let key = h.call_sym(PSTRING_CTOR, Conv::Thiscall(slots), &[name]);
            let node = h.call_sym(PSTRING_TREE_FIND, Conv::Thiscall(this + 0x68), &[key]);
            let found = entry(node);
            let text: u32 = h.read(slots);
            free_text(h, text);
            if found != 0 {
                h.write(flags, u32::MAX);
                let shader: u32 = h.read(found);
                return h.read(shader);
            }
        }
        let number: u32 = h.read(flags);
        h.write(slots + 4, number);
        let node = h.call_sym(UINT_TREE_FIND, Conv::Thiscall(this), &[slots + 4]);
        let found = entry(node);
        if found == 0 {
            return 0;
        }
        h.read(found)
    }
}
