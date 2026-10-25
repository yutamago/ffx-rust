//! Constructors and destructors of the animation controllers.
//!
//! The hierarchy (vtables from Fahrenheit's labels):
//!
//! ```text
//! PAnimationHierarchyNode            vtable, +4 (zero)
//! ├── PTimeController                (no fields of its own)
//! │   ├── PTimeIntervalController    +8 = 0, f32 +0xc, +0x10, +0x14 = 0
//! │   └── PTimeScaleOffsetController +8 = 0, f32 +0xc = 1 (scale), +0x10 = 0
//! └── PAnimationDataSource           +8 = 0
//!     ├── PAnimationEventController
//!     ├── PAnimationController       +0xc, +0x10 = 0
//!     └── PAnimationBlenderController +0xc, +0x10 = 0
//!         ├── PAnimationWeightedBlenderController +0x14, +0x18 = 0
//!         ├── PAnimationSpuTargetBlenderController
//!         └── PAnimationTargetBlenderController   +0xc = 0
//! ```
//!
//! All are `thiscall`. A constructor returns `this`; a destructor sets its
//! class's vtable and runs the base's (tail jump), leaving `eax` as the
//! caller had it; a scalar deleting destructor (vtable slot 0,
//! `thiscall(this, flags)`) runs the destructor, frees the object when
//! bit 0 of `flags` is set and returns `this`.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::descriptor::{bridge as desc, free};

/// The vtables (`ffx.sym` addresses).
pub mod vtable {
    pub const HIERARCHY_NODE: u32 = 0x00b2_8150;
    pub const TIME_CONTROLLER: u32 = 0x00b2_82c0;
    pub const TIME_INTERVAL: u32 = 0x00b2_84a0;
    pub const TIME_SCALE_OFFSET: u32 = 0x00b2_86b8;
    pub const DATA_SOURCE: u32 = 0x00b2_8870;
    pub const EVENT_CONTROLLER: u32 = 0x00b2_8a10;
    pub const CONTROLLER: u32 = 0x00b2_8c48;
    pub const BLENDER: u32 = 0x00b2_96d0;
    pub const WEIGHTED_BLENDER: u32 = 0x00b2_9ae0;
    pub const SPU_TARGET_BLENDER: u32 = 0x00b2_9f50;
    pub const TARGET_BLENDER: u32 = 0x00b2_a180;
    pub const ADDITIVE_BLENDER: u32 = 0x00b2_9d20;
    pub const SLOT_FILTER: u32 = 0x00b2_8f50;
}

/// Destructors the deleting destructors call at their address
/// (`thiscall(this)`, `ffx.sym` addresses; `PAnimationSlotFilter`'s is
/// [`bridge::slot_filter_dtor`], the others are not ported).
pub mod dtor {
    /// `PAnimationSlotFilter`'s.
    pub const SLOT_FILTER: u32 = 0x0052_7ba0;
    /// `PAnimationNetworkInstance`'s.
    pub const NETWORK_INSTANCE: u32 = 0x0053_4b10;
    /// `PAnimatableComponent`'s.
    pub const ANIMATABLE_COMPONENT: u32 = 0x0053_ebd0;
}

fn set_vtable(m: &mut impl Mem, this: u32, vt: u32) {
    let v = m.rebase(vt);
    m.write(this, v);
}

pub mod bridge {
    use super::{Conv, Host, Mem, desc, free, set_vtable, vtable as vt};

    /// `PAnimationHierarchyNode::PAnimationHierarchyNode()` (`0x005211f0`).
    pub fn hierarchy_node_ctor(m: &mut impl Mem, this: u32) -> u32 {
        set_vtable(m, this, vt::HIERARCHY_NODE);
        m.write(this + 4, 0u32);
        this
    }

    /// `PAnimationHierarchyNode::~PAnimationHierarchyNode()` (`0x00521220`).
    pub fn hierarchy_node_dtor(m: &mut impl Mem, this: u32) {
        set_vtable(m, this, vt::HIERARCHY_NODE);
    }

    /// `PTimeController::PTimeController()` (`0x00521a50`).
    pub fn time_controller_ctor(m: &mut impl Mem, this: u32) -> u32 {
        hierarchy_node_ctor(m, this);
        set_vtable(m, this, vt::TIME_CONTROLLER);
        this
    }

    /// `PTimeIntervalController::PTimeIntervalController()` (`0x005225a0`).
    pub fn time_interval_ctor(m: &mut impl Mem, this: u32) -> u32 {
        time_controller_ctor(m, this);
        m.write(this + 0xc, 0.0f32.to_bits());
        set_vtable(m, this, vt::TIME_INTERVAL);
        m.write(this + 0x10, 0.0f32.to_bits());
        m.write(this + 8, 0u32);
        m.write(this + 0x14, 0.0f32.to_bits());
        this
    }

    /// `PTimeScaleOffsetController::PTimeScaleOffsetController()`
    /// (`0x005231e0`).
    pub fn time_scale_offset_ctor(m: &mut impl Mem, this: u32) -> u32 {
        time_controller_ctor(m, this);
        m.write(this + 0xc, 1.0f32.to_bits());
        set_vtable(m, this, vt::TIME_SCALE_OFFSET);
        m.write(this + 8, 0u32);
        m.write(this + 0x10, 0.0f32.to_bits());
        this
    }

    /// `PAnimationDataSource::PAnimationDataSource()` (`0x00523d20`).
    pub fn data_source_ctor(m: &mut impl Mem, this: u32) -> u32 {
        hierarchy_node_ctor(m, this);
        set_vtable(m, this, vt::DATA_SOURCE);
        m.write(this + 8, 0u32);
        this
    }

    /// `PAnimationController::PAnimationController()` (`0x00525240`).
    pub fn controller_ctor(m: &mut impl Mem, this: u32) -> u32 {
        data_source_ctor(m, this);
        set_vtable(m, this, vt::CONTROLLER);
        m.write(this + 0xc, 0u32);
        m.write(this + 0x10, 0u32);
        this
    }

    /// `PAnimationBlenderController::PAnimationBlenderController()`
    /// (`0x0052b820`).
    pub fn blender_ctor(m: &mut impl Mem, this: u32) -> u32 {
        data_source_ctor(m, this);
        set_vtable(m, this, vt::BLENDER);
        m.write(this + 0xc, 0u32);
        m.write(this + 0x10, 0u32);
        this
    }

    /// `PAnimationWeightedBlenderController::PAnimationWeightedBlenderController()`
    /// (`0x0052d660`).
    pub fn weighted_blender_ctor(m: &mut impl Mem, this: u32) -> u32 {
        blender_ctor(m, this);
        set_vtable(m, this, vt::WEIGHTED_BLENDER);
        m.write(this + 0x14, 0u32);
        m.write(this + 0x18, 0u32);
        this
    }

    /// `PAnimationTargetBlenderController::PAnimationTargetBlenderController()`
    /// (`0x00530dd0`).
    pub fn target_blender_ctor(m: &mut impl Mem, this: u32) -> u32 {
        data_source_ctor(m, this);
        set_vtable(m, this, vt::TARGET_BLENDER);
        m.write(this + 0xc, 0u32);
        this
    }

    /// A destructor that sets the class's vtable `vt` and runs its base's
    /// (`PTimeIntervalController`'s `0x00522600`, `PTimeScaleOffsetController`'s
    /// `0x00523240` over `PAnimationHierarchyNode`; `PAnimationDataSource`'s
    /// `0x00523d60` the same with its own vtable).
    pub fn dtor_over_node(m: &mut impl Mem, this: u32, vt: u32) {
        set_vtable(m, this, vt);
        hierarchy_node_dtor(m, this);
    }

    /// `PAnimationDataSource::~PAnimationDataSource()` (`0x00523d60`).
    pub fn data_source_dtor(m: &mut impl Mem, this: u32) {
        dtor_over_node(m, this, vt::DATA_SOURCE);
    }

    /// A destructor over `PAnimationDataSource`'s (`PAnimationController`'s
    /// `0x005252c0`, `PAnimationTargetBlenderController`'s `0x00530e40`).
    pub fn dtor_over_data_source(m: &mut impl Mem, this: u32, vt: u32) {
        set_vtable(m, this, vt);
        data_source_dtor(m, this);
    }

    /// The end of every scalar deleting destructor: frees `this` when bit
    /// 0 of `flags` is set; returns `this`.
    pub fn delete_if(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        if flags & 1 != 0 {
            free(h, this);
        }
        this
    }

    /// Which destructor a scalar deleting destructor runs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Dtor {
        /// `PAnimationHierarchyNode`'s (`PTimeController`, `0x00521b80`).
        Node,
        /// The class's vtable, then `PAnimationHierarchyNode`'s
        /// (`0x005226d0`, `0x00523310`).
        OverNode(u32),
        /// The class's vtable, then `PAnimationDataSource`'s
        /// (`0x00524740`, `0x00525390`, `0x00530180`, `0x00530f20`).
        OverDataSource(u32),
        /// An original destructor (`thiscall(this)` at this `ffx.sym`
        /// address: `0x00528180`, `0x005357d0`, `0x0053f3d0`).
        Original(u32),
    }

    /// A scalar deleting destructor (`thiscall(this, flags)` -> `this`).
    pub fn deleting_dtor(h: &mut impl Host, this: u32, flags: u32, dtor: Dtor) -> u32 {
        match dtor {
            Dtor::Node => hierarchy_node_dtor(h, this),
            Dtor::OverNode(vt) => dtor_over_node(h, this, vt),
            Dtor::OverDataSource(vt) => dtor_over_data_source(h, this, vt),
            Dtor::Original(f) => {
                h.call_sym(f, Conv::Thiscall(this), &[]);
            }
        }
        delete_if(h, this, flags)
    }

    /// `PAnimationEventController::PAnimationEventController()` (inlined in
    /// its construct callback `0x00524c00`): over `PAnimationDataSource`,
    /// `+0xc`, `+0x10` zero.
    pub fn event_controller_ctor(m: &mut impl Mem, this: u32) -> u32 {
        data_source_ctor(m, this);
        set_vtable(m, this, vt::EVENT_CONTROLLER);
        m.write(this + 0xc, 0u32);
        m.write(this + 0x10, 0u32);
        this
    }

    /// `PAnimationAdditiveBlenderController`'s constructor (inlined in
    /// `0x0052f670`): over `PAnimationBlenderController`, `+0x14`, `+0x18`
    /// zero.
    pub fn additive_blender_ctor(m: &mut impl Mem, this: u32) -> u32 {
        blender_ctor(m, this);
        set_vtable(m, this, vt::ADDITIVE_BLENDER);
        m.write(this + 0x14, 0u32);
        m.write(this + 0x18, 0u32);
        this
    }

    /// `PAnimationSpuTargetBlenderController`'s constructor (inlined in
    /// `0x005305c0`): over `PAnimationDataSource`, `+0xc` zero.
    pub fn spu_target_blender_ctor(m: &mut impl Mem, this: u32) -> u32 {
        data_source_ctor(m, this);
        set_vtable(m, this, vt::SPU_TARGET_BLENDER);
        m.write(this + 0xc, 0u32);
        this
    }

    /// `PAnimationSlotFilter`'s constructor (inlined in `0x0052a060`): over
    /// `PAnimationDataSource`, `+0xc`..`+0x1c` zero.
    pub fn slot_filter_ctor(m: &mut impl Mem, this: u32) -> u32 {
        data_source_ctor(m, this);
        set_vtable(m, this, vt::SLOT_FILTER);
        for off in [0xc, 0x10, 0x18, 0x1c, 0x14] {
            m.write(this + off, 0u32);
        }
        this
    }

    /// A construct callback of a controller class (`stdcall(p)`;
    /// `0x00522040` and the others): runs `ctor` on `p` when it is not
    /// null. Returns `true` in `al` (the rest of `eax` is a stack address
    /// for a null `p`; callers test `al`).
    pub fn construct<M: Mem>(m: &mut M, p: u32, ctor: impl FnOnce(&mut M, u32) -> u32) -> bool {
        if p != 0 {
            ctor(m, p);
        }
        true
    }

    /// `PAnimationBlenderController::~PAnimationBlenderController()`
    /// (`0x0052b8e0`): frees its owned `PArray<PAnimationDataSource>` at
    /// `+0xc` (no null check), then `PAnimationDataSource`'s destructor.
    pub fn blender_dtor(h: &mut impl Host, this: u32) {
        set_vtable(h, this, vt::BLENDER);
        desc::free_owned(h, this + 0xc, false);
        data_source_dtor(h, this);
    }

    /// A blender subclass's destructor (`PAnimationWeightedBlenderController`'s
    /// `0x0052d6b0`; inlined in `PAnimationAdditiveBlenderController`'s
    /// deleting destructor): its vtable `vt`, frees its owned `PArray` at
    /// `+0x14`, then [`blender_dtor`].
    pub fn blender_subclass_dtor(h: &mut impl Host, this: u32, vt: u32) {
        set_vtable(h, this, vt);
        desc::free_owned(h, this + 0x14, true);
        blender_dtor(h, this);
    }

    /// `PAnimationSlotFilter::~PAnimationSlotFilter()` (`0x00527ba0`):
    /// frees its owned deferred-load entries (`+0x18`, through
    /// `0x00528a30`) and its owned `PArray` at `+0xc`, then
    /// `PAnimationDataSource`'s destructor.
    pub fn slot_filter_dtor(h: &mut impl Host, this: u32) {
        set_vtable(h, this, vt::SLOT_FILTER);
        desc::free_owned_elements(h, this + 0x18, 8, 4);
        desc::free_owned(h, this + 0xc, true);
        data_source_dtor(h, this);
    }

    /// `PAnimationSet::~PAnimationSet()` (`0x00519380`, no vtable): frees
    /// its owned `PArray` at `+0x10`, its owned targets at `+8` (through
    /// `0x0051a890`) and its owned `PSharray<PAnimationClip>` at `+0`.
    pub fn animation_set_dtor(h: &mut impl Host, this: u32) {
        desc::free_owned(h, this + 0x10, true);
        desc::free_owned_elements(h, this + 8, 0x1c, 0x14);
        desc::free_owned_shared(h, this);
    }

    /// The scalar deleting destructor of a blender subclass (`0x0052d940`,
    /// `0x0052edd0`): [`blender_subclass_dtor`], then [`delete_if`].
    pub fn blender_subclass_deleting_dtor(
        h: &mut impl Host,
        this: u32,
        flags: u32,
        vt: u32,
    ) -> u32 {
        blender_subclass_dtor(h, this, vt);
        delete_if(h, this, flags)
    }
}
