//! Destructor chains: a derived class's destructor that stores its own
//! vtable and jumps to its parent's, the parent passed in by the caller
//! (game code not ported yet, or a Rust port).

/// The `Phyre::PClassDescriptorForType<T>` destructors (`thiscall`):
/// `*this` = the vtable, then `Phyre::PClassDescriptor`'s destructor
/// (`0x0043b3e0`), whose `eax` they return. (address, vtable), the type in
/// the comment.
pub const CLASS_DESCRIPTOR_DTORS: [(u32, u32); 31] = [
    (0x0067_5f30, 0x00b4_466c), // RadialLineDynamicMesh
    (0x0067_6ba0, 0x00b4_483c), // BrokenScreenPolygonDynamicMeshInstance
    (0x0067_7590, 0x00b4_49bc), // BrokenScreenPolygonDynamicMesh
    (0x0067_82f0, 0x00b4_4ba4), // ShadowDynamicMesh
    (0x009e_2160, 0x00b7_3a2c), // Phyre_PArray_Phyre_PInputs_PInputAction_4
    (0x009e_2170, 0x00b7_38f4), // Phyre_PArray_Phyre_PInputs_PInputSource_4
    (0x009e_2180, 0x00b7_1fac), // Phyre_PInputs_PInputAction
    (0x009e_2190, 0x00b7_209c), // Phyre_PInputs_PInputMap
    (0x009e_21a0, 0x00b7_08ac), // Phyre_PInputs_PInputSource
    (0x009e_21b0, 0x00b7_0b54), // Phyre_PInputs_PInputSourceJoypadAxis
    (0x009e_21c0, 0x00b7_0a54), // Phyre_PInputs_PInputSourceJoypadButton
    (0x009e_21d0, 0x00b7_095c), // Phyre_PInputs_PInputSourceKey
    (0x009e_21e0, 0x00b7_1964), // Phyre_PInputs_PInputSourceMotionAngularVelocityX
    (0x009e_21f0, 0x00b7_1a6c), // Phyre_PInputs_PInputSourceMotionAngularVelocityY
    (0x009e_2200, 0x00b7_1b74), // Phyre_PInputs_PInputSourceMotionAngularVelocityZ
    (0x009e_2210, 0x00b7_1c7c), // Phyre_PInputs_PInputSourceMotionLinearAccelerationX
    (0x009e_2220, 0x00b7_1d8c), // Phyre_PInputs_PInputSourceMotionLinearAccelerationY
    (0x009e_2230, 0x00b7_1e9c), // Phyre_PInputs_PInputSourceMotionLinearAccelerationZ
    (0x009e_2240, 0x00b7_1864), // Phyre_PInputs_PInputSourceMotionQuatW
    (0x009e_2250, 0x00b7_1564), // Phyre_PInputs_PInputSourceMotionQuatX
    (0x009e_2260, 0x00b7_1664), // Phyre_PInputs_PInputSourceMotionQuatY
    (0x009e_2270, 0x00b7_1764), // Phyre_PInputs_PInputSourceMotionQuatZ
    (0x009e_2280, 0x00b7_0c54), // Phyre_PInputs_PInputSourceMouseButton
    (0x009e_2290, 0x00b7_0d54), // Phyre_PInputs_PInputSourceMouseDeltaX
    (0x009e_22a0, 0x00b7_0e54), // Phyre_PInputs_PInputSourceMouseDeltaY
    (0x009e_22b0, 0x00b7_1364), // Phyre_PInputs_PInputSourceTouchDragX
    (0x009e_22c0, 0x00b7_1464), // Phyre_PInputs_PInputSourceTouchDragY
    (0x009e_22d0, 0x00b7_1054), // Phyre_PInputs_PInputSourceTouchPinch
    (0x009e_22e0, 0x00b7_0f54), // Phyre_PInputs_PInputSourceTouchRotate
    (0x009e_22f0, 0x00b7_1154), // Phyre_PInputs_PInputSourceTouchTwoFingerDragX
    (0x009e_2300, 0x00b7_125c), // Phyre_PInputs_PInputSourceTouchTwoFingerDragY
];

/// `Phyre::PClassDescriptor::~PClassDescriptor` (`0x0043b3e0`).
pub const CLASS_DESCRIPTOR_DTOR: u32 = 0x0043_b3e0;

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// A derived destructor: `*this` = `vtable`, then `parent(this)`, whose
    /// result it returns (the original's tail jump).
    pub fn chain<M: Mem>(
        mem: &mut M,
        this: u32,
        vtable: u32,
        parent: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        mem.write(this, vtable);
        parent(mem, this)
    }
}
