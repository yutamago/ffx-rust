//! `ffx-pload`: `PhyreEngine`'s serialization and cluster loading (M4
//! port round 1, list `p1-load`).
//!
//! Threads: the type system runs at start-up on the main thread; the
//! loader (`PBinary`, the streams) on the loader thread too. No shim keeps
//! state: each works on the game's objects through [`mem`].
//!
//! The descriptors' vtable slots that are the same few bytes in many
//! classes are each their own original, so each gets its own line.

use super::{mem, ported};
use ffx_pload::{cluster, descriptor};

/// `PClusterHeaderBase::PClusterHeaderBase` (slot 10 of its descriptor
/// constructs one in place).
const HEADER_BASE_CTOR: u32 = cluster::HEADER_BASE_CTOR;
/// `PNode::PNode`.
const NODE_CTOR: u32 = 0x0050_4ec0;
/// `PNode::~PNode`.
const NODE_DTOR: u32 = 0x0050_5010;
/// `PBitmapFont::~PBitmapFont`.
const BITMAP_FONT_DTOR: u32 = 0x00a2_bbd0;

ported! {
    // ---- descriptor slots returning a constant ----
    /// `PClassDescriptorWithoutDefaultConstructor<PClassMember>::constructInstance(p)`: false.
    Phyre_PClassDescriptorWithoutDefaultConstructor_PClassMember_constructInstance @ 0x0043_a540 => extern "stdcall" fn construct_false_a540(_p: u32) -> u8 { descriptor::core::CONSTRUCT_REFUSED }
    /// `PClassDescriptorForType<T>::constructInstance(p)` (823 vtables): false.
    Phyre_PClassDescriptorForType_constructInstance @ 0x0043_ca40 => extern "stdcall" fn construct_false_ca40(_p: u32) -> u8 { descriptor::core::CONSTRUCT_REFUSED }
    /// `PClassDescriptorWithoutDefaultConstructor<PRandomGenerator>::constructInstance(p)`: false.
    Phyre_PClassDescriptorWithoutDefaultConstructor_PRandomGenerator_constructInstance @ 0x0045_ede0 => extern "stdcall" fn construct_false_ede0(_p: u32) -> u8 { descriptor::core::CONSTRUCT_REFUSED }
    /// `PClassDescriptor<PTypedObject>::destructInstance(p)`: true.
    Phyre_PClassDescriptor_PTypedObject_destructInstance @ 0x0044_47c0 => extern "stdcall" fn destruct_true_47c0(_p: u32) -> u8 { descriptor::core::DESTRUCT_DONE }
    /// `PClassDescriptor<PMatrix4>::destructInstance(p)`: true.
    Phyre_PClassDescriptor_PMatrix4_destructInstance @ 0x0046_1dc0 => extern "stdcall" fn destruct_true_1dc0(_p: u32) -> u8 { descriptor::core::DESTRUCT_DONE }
    /// `PClassDescriptor<PMatrix4x3>::destructInstance(p)`: true.
    Phyre_PClassDescriptor_PMatrix4x3_destructInstance @ 0x0046_3150 => extern "stdcall" fn destruct_true_3150(_p: u32) -> u8 { descriptor::core::DESTRUCT_DONE }
    /// `PClassDescriptor<PClusterHeader>::destructInstance(p)`: true.
    Phyre_PClassDescriptor_PClusterHeader_destructInstance @ 0x0046_94d0 => extern "stdcall" fn destruct_true_94d0(_p: u32) -> u8 { descriptor::core::DESTRUCT_DONE }
    /// `PClassDescriptor<PClusterHeaderBase>::destructInstance(p)`: true.
    Phyre_PClassDescriptor_PClusterHeaderBase_destructInstance @ 0x0046_94e0 => extern "stdcall" fn destruct_true_94e0(_p: u32) -> u8 { descriptor::core::DESTRUCT_DONE }
    /// `PClassDescriptor<PClusterHeaderD3D11>::destructInstance(p)`: true.
    Phyre_PClassDescriptor_PClusterHeaderD3D11_destructInstance @ 0x0046_9ed0 => extern "stdcall" fn destruct_true_9ed0(_p: u32) -> u8 { descriptor::core::DESTRUCT_DONE }
    /// `PClassDescriptor<PBitmapFontCharInfo>::destructInstance(p)`: true.
    Phyre_PClassDescriptor_PBitmapFontCharInfo_destructInstance @ 0x00a2_d630 => extern "stdcall" fn destruct_true_d630(_p: u32) -> u8 { descriptor::core::DESTRUCT_DONE }
    /// `PClassDescriptorForType<T>` slot 14 `(x)` (435 vtables): 0.
    Phyre_PClassDescriptorForType_slot14 @ 0x0043_cad0 => extern "stdcall" fn slot14_cad0(_x: u32) -> u32 { descriptor::core::SLOT14 }
    /// `PClassDescriptor<PNode>` slot 14 `(x)`: 0.
    Phyre_PClassDescriptor_PNode_slot14 @ 0x0050_6bc0 => extern "stdcall" fn slot14_6bc0(_x: u32) -> u32 { descriptor::core::SLOT14 }
    /// `PClassDescriptor<PMorphModifierWeightsUserDataObject>` slot 14 `(x)`: 0.
    Phyre_PClassDescriptor_PMorphModifierWeightsUserDataObject_slot14 @ 0x005c_bae0 => extern "stdcall" fn slot14_bae0(_x: u32) -> u32 { descriptor::core::SLOT14 }
    /// `PClassDescriptor<PBitmapFont>` slot 14 `(x)`: 0.
    Phyre_PClassDescriptor_PBitmapFont_slot14 @ 0x00a2_d650 => extern "stdcall" fn slot14_d650(_x: u32) -> u32 { descriptor::core::SLOT14 }
    /// `PClassDescriptor<PBitmapFontCharInfo>` slot 14 `(x)`: 0.
    Phyre_PClassDescriptor_PBitmapFontCharInfo_slot14 @ 0x00a2_d660 => extern "stdcall" fn slot14_d660(_x: u32) -> u32 { descriptor::core::SLOT14 }

    // ---- descriptor slots constructing or destroying in place ----
    /// `PClassDescriptorConcrete<PArray<u8>>::constructInstance(p)`.
    Phyre_PClassDescriptorConcrete_PArray_u8_constructInstance @ 0x0044_9400 => extern "stdcall" fn construct_pair_9400(p: u32) -> u32 {
        descriptor::bridge::construct_zero_pair(&mut mem(), p)
    }
    /// `PClassDescriptorConcrete<PArray<PTypedObject>>::constructInstance(p)`.
    Phyre_PClassDescriptorConcrete_PArray_PTypedObject_constructInstance @ 0x0045_95e0 => extern "stdcall" fn construct_pair_95e0(p: u32) -> u32 {
        descriptor::bridge::construct_zero_pair(&mut mem(), p)
    }
    /// `PClassDescriptorConcrete<PMorphModifierWeightsUserDataObject>::constructInstance(p)`.
    Phyre_PClassDescriptorConcrete_PMorphModifierWeightsUserDataObject_constructInstance @ 0x005c_ba80 => extern "stdcall" fn construct_pair_ba80(p: u32) -> u32 {
        descriptor::bridge::construct_zero_pair(&mut mem(), p)
    }
    /// `PClassDescriptorConcrete<PArray<PBitmapFontCharInfo>>::constructInstance(p)`.
    Phyre_PClassDescriptorConcrete_PArray_PBitmapFontCharInfo_constructInstance @ 0x00a2_d500 => extern "stdcall" fn construct_pair_d500(p: u32) -> u32 {
        descriptor::bridge::construct_zero_pair(&mut mem(), p)
    }
    /// `PClassDescriptorConcrete<PClusterHeaderBase>::constructInstance(p)`.
    Phyre_PClassDescriptorConcrete_PClusterHeaderBase_constructInstance @ 0x0046_94b0 => extern "stdcall" fn construct_header_base(p: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), HEADER_BASE_CTOR, p)
    }
    /// `PClassDescriptorConcrete<PNode>::constructInstance(p)`.
    Phyre_PClassDescriptorConcrete_PNode_constructInstance @ 0x0050_6b30 => extern "stdcall" fn construct_node(p: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), NODE_CTOR, p)
    }
    /// `PClassDescriptor<PNode>::destructInstance(p)`: `~PNode` on `p - 4`.
    Phyre_PClassDescriptor_PNode_destructInstance @ 0x0050_6b50 => extern "stdcall" fn destruct_node(p: u32) -> u8 {
        descriptor::bridge::destroy_with(&mut mem(), NODE_DTOR, 4, p)
    }
    /// `PClassDescriptor<PBitmapFont>::destructInstance(p)`: `~PBitmapFont` on `p`.
    Phyre_PClassDescriptor_PBitmapFont_destructInstance @ 0x00a2_d610 => extern "stdcall" fn destruct_bitmap_font(p: u32) -> u8 {
        descriptor::bridge::destroy_with(&mut mem(), BITMAP_FONT_DTOR, 0, p)
    }
    /// `PClassDescriptor<PClassDataMemberDynamic>::destructInstance(p)`: slot 0 of `p - 4`.
    Phyre_PClassDescriptor_PClassDataMemberDynamic_destructInstance @ 0x0044_69a0 => extern "stdcall" fn destruct_member_dynamic(p: u32) -> u8 {
        descriptor::bridge::destroy_virtual(&mut mem(), 0, 4, p)
    }
    /// `PClassDescriptor<PClassDescriptorDynamic>::destructInstance(p)`: slot 7 of `p - 0x18`.
    Phyre_PClassDescriptor_PClassDescriptorDynamic_destructInstance @ 0x0044_9580 => extern "stdcall" fn destruct_descriptor_dynamic(p: u32) -> u8 {
        descriptor::bridge::destroy_virtual(&mut mem(), 7, 0x18, p)
    }
    /// `PClassDescriptorWithoutDefaultConstructor<PClassDataMemberDynamic>::constructInstance(p)`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PClassDataMemberDynamic__constructInstance @ 0x0044_6980 => extern "stdcall" fn construct_member_dynamic(p: u32) -> u32 {
        descriptor::bridge::construct_member_dynamic(&mut mem(), p)
    }

    // ---- PType, PClassDescriptor, members ----
    /// `PType::setFixupFunctions(a, b)` -> `b`.
    Phyre_PType_setFixupFunctions @ 0x0043_79b0 => extern "thiscall" fn set_fixup_functions(this: u32, a: u32, b: u32) -> u32 {
        descriptor::bridge::set_fields(&mut mem(), this, 0x24, &[a, b])
    }
    /// `PType::resolveFixupForAddress(a, b, c, d)`.
    Phyre_PType_resolveFixupForAddress @ 0x0043_7980 => extern "thiscall" fn resolve_fixup(this: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        descriptor::bridge::resolve_fixup(&mut mem(), this, [a, b, c, d])
    }
    /// `PClassDescriptor::setDefaultBuffer(p)` -> `p`.
    Phyre_PClassDescriptor_setDefaultBuffer @ 0x0043_d510 => extern "thiscall" fn set_default_buffer(this: u32, p: u32) -> u32 {
        descriptor::bridge::set_fields(&mut mem(), this, 0x64, &[p])
    }
    /// `PClassDescriptor::setValidationBuffer(p)` -> `p`.
    Phyre_PClassDescriptor_setValidationBuffer @ 0x0043_d5b0 => extern "thiscall" fn set_validation_buffer(this: u32, p: u32) -> u32 {
        descriptor::bridge::set_fields(&mut mem(), this, 0x68, &[p])
    }
    /// `PClassDescriptor::setWriteMaskBuffer(p)` -> `p`.
    Phyre_PClassDescriptor_setWriteMaskBuffer @ 0x0043_d5c0 => extern "thiscall" fn set_write_mask_buffer(this: u32, p: u32) -> u32 {
        descriptor::bridge::set_fields(&mut mem(), this, 0x6c, &[p])
    }
    /// `PClassDescriptor::setOverallocation(n, size)` -> `size`.
    Phyre_PClassDescriptor_setOverallocation @ 0x0043_d590 => extern "thiscall" fn set_overallocation(this: u32, n: u32, size: u32) -> u32 {
        descriptor::bridge::set_fields(&mut mem(), this, 0x74, &[n, size])
    }
    /// `PClassDescriptor::isInNamespace()`.
    Phyre_PClassDescriptor_isInNamespace @ 0x0043_cfb0 => extern "thiscall" fn is_in_namespace(this: u32) -> u32 {
        descriptor::bridge::in_namespace(&mem(), this)
    }
    /// `PClassDescriptor::addToNamespace()`.
    Phyre_PClassDescriptor_addToNamespace @ 0x0043_c140 => extern "thiscall" fn add_to_namespace(this: u32) -> u32 {
        descriptor::bridge::add_to_namespace(&mut mem(), this)
    }
    /// `PClassDescriptor::getDefaultBuffer()`.
    Phyre_PClassDescriptor_getDefaultBuffer @ 0x0043_cb80 => extern "thiscall" fn get_default_buffer(this: u32) -> u32 {
        descriptor::bridge::default_buffer(&mut mem(), this)
    }
    /// `PClassDescriptor::inheritsFrom(other)`.
    Phyre_PClassDescriptor_inheritsFrom @ 0x0043_1420 => extern "thiscall" fn inherits_from(this: u32, other: u32) -> u32 {
        descriptor::bridge::inherits_from(&mem(), this, other)
    }
    /// `PClassDescriptorDynamic` slot 12.
    Phyre_PClassDescriptorDynamic_bindBase @ 0x0044_9380 => extern "thiscall" fn bind_base(this: u32) -> u32 {
        descriptor::bridge::bind_base(&mut mem(), this)
    }
    /// `PClassDescriptorDynamic`'s scalar deleting destructor `(flags)`.
    Phyre_PClassDescriptorDynamic_scalarDeletingDtor @ 0x0044_7cb0 => extern "thiscall" fn delete_descriptor_dynamic(this: u32, flags: u32) -> u32 {
        descriptor::bridge::delete_dynamic(&mut mem(), this, flags)
    }
    /// `PClassDataMember::getDynamicType(x)`.
    Phyre_PClassDataMember_getDynamicType @ 0x0057_5690 => extern "thiscall" fn get_dynamic_type(this: u32, x: u32) -> u32 {
        descriptor::bridge::dynamic_type(&mut mem(), this, x)
    }
    /// `PClassDataMemberDynamic::IsDefaultInstance(member)`.
    Phyre_PClassDataMemberDynamic_IsDefaultInstance @ 0x0044_6870 => extern "C" fn is_default_instance(member: u32) -> u32 {
        descriptor::bridge::is_default_instance(&mem(), member)
    }
    /// `PClassDataMemberArrayDynamic<PArray<u8>>` slot 8 `(array, a, b)`.
    Phyre_PClassDataMemberArrayDynamic_PArray_u8_resize @ 0x0044_a320 => extern "thiscall" fn resize_u8(this: u32, array: u32, a: u32, b: u32) -> u32 {
        descriptor::bridge::resize_array(&mut mem(), this, array, a, b)
    }
    /// `PClassDataMemberArrayDynamic<PArray<PBitmapFontCharInfo>>` slot 8 `(array, a, b)`.
    Phyre_PClassDataMemberArrayDynamic_PArray_PBitmapFontCharInfo_resize @ 0x00a2_dbc0 => extern "thiscall" fn resize_char_info(this: u32, array: u32, a: u32, b: u32) -> u32 {
        descriptor::bridge::resize_array(&mut mem(), this, array, a, b)
    }
    /// `0x00575d70(x)`: the block header of `this`'s array.
    Phyre_PArray_blockHeader_00575d70 @ 0x0057_5d70 => extern "thiscall" fn block_header(this: u32, x: u32) -> u32 {
        descriptor::bridge::block_header(&mem(), this, x)
    }
}
