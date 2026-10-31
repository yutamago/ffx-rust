//! M5 stage 1, the leftovers of list `phyre-registry-load-a` (round
//! left2): class-descriptor visitors, `PClassDescriptorDynamic`'s layout,
//! name lookups and cluster helpers
//! (`ffx_pload::m5p_registry_load_a_left`). The parity tests are in
//! `crates/ffx-oracle/tests/m5p_registry_load_a.rs` (`left2_*`).
//!
//! Threads: these run where clusters load and the script layer binds
//! (the loader and main threads). The visitors, the registry key and the
//! instance cursor the originals keep in their frames are per-call
//! [`StackFrame`]s here: the swap visitor is read again after calls that
//! can swap nested objects (and so reach this port again).

use ffx_pload::m5p_registry_load_a_left::{self as lf, bridge as lb};
use ffx_pload::m5p_tmpl_fortype::bridge as t;

use super::{StackFrame, mem, ported};

ported! {
    /// The copy visitor's per-member step `(member)`.
    Phyre_PClassDescriptor_copyMemberVisitor_0043d140 @ 0x0043_d140 => extern "thiscall" fn copy_member(visitor: u32, member: u32) {
        lb::copy_member_at(&mut mem(), visitor, member);
    }
    /// The swap visitor's per-member step `(member)`.
    Phyre_PClassDescriptor_swapMemberVisitor_0043d230 @ 0x0043_d230 => extern "thiscall" fn swap_member(visitor: u32, member: u32) {
        lb::swap_member_at(&mut mem(), visitor, member);
    }
    /// The swap walker, forwards `(visitor)`; `eax` the last link, else
    /// the visitor's address (every caller's `eax` at the call).
    Phyre_PClassDescriptor_swapMembersForward_0043ab60 @ 0x0043_ab60 => extern "thiscall" fn swap_forward(desc: u32, visitor: u32) -> u32 {
        lb::swap_walk(&mut mem(), desc, visitor, false, visitor)
    }
    /// The swap walker, backwards `(visitor)`.
    Phyre_PClassDescriptor_swapMembersBackward_0043ad10 @ 0x0043_ad10 => extern "thiscall" fn swap_backward(desc: u32, visitor: u32) -> u32 {
        lb::swap_walk(&mut mem(), desc, visitor, true, visitor)
    }
    /// `PClassDescriptor::swapEndiannessOfObject(object, flag)`.
    Phyre_PClassDescriptor_swapEndiannessOfObject @ 0x0043_d6a0 => extern "thiscall" fn swap_object(desc: u32, object: u32, flag: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        lb::swap_object(&mut mem(), desc, object, flag, f.at())
    }
    /// The copy walker `(visitor)`.
    Phyre_PClassDescriptor_copyMembers_0043abb0 @ 0x0043_abb0 => extern "thiscall" fn copy_walk(desc: u32, visitor: u32) -> u32 {
        lb::copy_walk_at(&mut mem(), desc, visitor)
    }
    /// The object copy `(a, b)`: `b` into `a`.
    Phyre_PClassDescriptor_copyObject_0043ca50 @ 0x0043_ca50 => extern "thiscall" fn copy_object(desc: u32, a: u32, b: u32) -> u32 {
        lb::copy_object(&mut mem(), desc, a, b)
    }
    /// The script function `(L, top, object)` pushing a `PhyreObject`.
    Phyre_PScripting_pushPhyreObject_00435940 @ 0x0043_5940 => extern "C" fn push_phyre_object(l: u32, top: u32, object: u32) -> u32 {
        lb::push_scripted(&mut mem(), l, top, object)
    }
    /// The named-entry lookup of the list at `+0x54` `(name)`.
    Phyre_PClassDescriptor_findInList54_00435b80 @ 0x0043_5b80 => extern "thiscall" fn find_54(desc: u32, name: u32) -> u32 {
        lb::find_named(&mut mem(), const { lf::named_list_of(0x0043_5b80) }, desc, name)
    }
    /// The named-entry lookup of the list at `+0x4c` `(name)`.
    Phyre_PClassDescriptor_findInList4c_00435c10 @ 0x0043_5c10 => extern "thiscall" fn find_4c(desc: u32, name: u32) -> u32 {
        lb::find_named(&mut mem(), const { lf::named_list_of(0x0043_5c10) }, desc, name)
    }
    /// The named-entry lookup of the list at `+0x5c` `(name)`.
    Phyre_PClassDescriptor_findInList5c_00435d30 @ 0x0043_5d30 => extern "thiscall" fn find_5c(desc: u32, name: u32) -> u32 {
        lb::find_named(&mut mem(), const { lf::named_list_of(0x0043_5d30) }, desc, name)
    }
    /// The name/value table's lookup `(name, out)`; `al`.
    Phyre_PNameValueTable_find_005709f0 @ 0x0057_09f0 => extern "thiscall" fn find_pair(this: u32, name: u32, out: u32) -> u8 {
        lb::find_pair(&mut mem(), this, name, out)
    }
    /// The copy of `PNamespace::getClassDescriptor(name)`.
    Phyre_PNamespace_getClassDescriptor_0043ea30 @ 0x0043_ea30 => extern "thiscall" fn namespace_find(ns: u32, name: u32) -> u32 {
        lb::namespace_find(&mut mem(), ns, name)
    }
    /// The function-local static list object.
    Phyre_StaticList_get_0043f090 @ 0x0043_f090 => extern "C" fn static_list() -> u32 {
        lb::static_list(&mut mem())
    }
    /// The instance-list node's release into the pool `(node)`.
    Phyre_PInstanceListPool_release_0043f3c0 @ 0x0043_f3c0 => extern "thiscall" fn release_node(pool: u32, node: u32) -> u32 {
        lb::release_node(&mut mem(), pool, node)
    }
    /// `PCluster`'s scalar deleting destructor `(flags)`.
    Phyre_PCluster_scalarDeletingDtor_0043fed0 @ 0x0043_fed0 => extern "thiscall" fn delete_cluster(this: u32, flags: u32) -> u32 {
        lb::delete_cluster(&mut mem(), this, flags)
    }
    /// `PCluster::createInternal(out, desc)`.
    Phyre_PCluster_createInternal @ 0x0044_1120 => extern "thiscall" fn create_internal(cluster: u32, out: u32, desc: u32) -> u32 {
        lb::create_internal(&mut mem(), cluster, out, desc)
    }
    /// `PCluster::deleteInstancesOfType(desc)`.
    Phyre_PCluster_deleteInstancesOfType @ 0x0044_1180 => extern "thiscall" fn delete_instances_of_type(cluster: u32, desc: u32) {
        lb::delete_instances_of_type(&mut mem(), cluster, desc);
    }
    /// `PArray<PUInt8>`'s `swapEndiannessStatic(p)`, flag 1.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_PUInt8__swapEndiannessStatic1_004470b0 @ 0x0044_70b0 => extern "C" fn ss_004470b0(p: u32) -> u32 {
        t::swap_lazy(&mut mem(), &const { lf::swap_static_of(0x0044_70b0) }, p)
    }
    /// `PArray<PUInt8>`'s `swapEndiannessStatic(p)`, flag 0.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_PUInt8__swapEndiannessStatic0_00447170 @ 0x0044_7170 => extern "C" fn ss_00447170(p: u32) -> u32 {
        t::swap_lazy(&mut mem(), &const { lf::swap_static_of(0x0044_7170) }, p)
    }
    /// `PSharray<PString>`'s `swapEndiannessStatic(p)`, flag 1.
    Phyre_PClassDescriptorConcrete_Phyre_PSharray_PString__swapEndiannessStatic1_0044a990 @ 0x0044_a990 => extern "C" fn ss_0044a990(p: u32) -> u32 {
        t::swap_lazy(&mut mem(), &const { lf::swap_static_of(0x0044_a990) }, p)
    }
    /// `PSharray<PString>`'s `swapEndiannessStatic(p)`, flag 0.
    Phyre_PClassDescriptorConcrete_Phyre_PSharray_PString__swapEndiannessStatic0_0044aa40 @ 0x0044_aa40 => extern "C" fn ss_0044aa40(p: u32) -> u32 {
        t::swap_lazy(&mut mem(), &const { lf::swap_static_of(0x0044_aa40) }, p)
    }
    /// `PArray<PTypedObject>`'s `swapEndiannessStatic(p)`, flag 1.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_PTypedObject__swapEndiannessStatic1_004574f0 @ 0x0045_74f0 => extern "C" fn ss_004574f0(p: u32) -> u32 {
        t::swap_lazy(&mut mem(), &const { lf::swap_static_of(0x0045_74f0) }, p)
    }
    /// `PArray<PTypedObject>`'s `swapEndiannessStatic(p)`, flag 0.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_PTypedObject__swapEndiannessStatic0_004575a0 @ 0x0045_75a0 => extern "C" fn ss_004575a0(p: u32) -> u32 {
        t::swap_lazy(&mut mem(), &const { lf::swap_static_of(0x0045_75a0) }, p)
    }
    /// `PArray<PUInt8>::operator=(src)`.
    Phyre_PArray_PUInt8_assign_004479f0 @ 0x0044_79f0 => extern "thiscall" fn assign_bytes(this: u32, src: u32) -> u32 {
        lb::assign_bytes(&mut mem(), this, src)
    }
    /// `PClassDescriptorDynamic`'s write-mask merge `(src)`.
    Phyre_PClassDescriptorDynamic_mergeBuffer_004494a0 @ 0x0044_94a0 => extern "thiscall" fn merge_buffer(this: u32, src: u32) {
        lb::merge_buffer(&mut mem(), this, src);
    }
    /// `PClassDescriptorDynamic`'s script read `(L, out)`.
    Phyre_PClassDescriptorDynamic_readScriptValue_004498f0 @ 0x0044_98f0 => extern "thiscall" fn read_script_value(this: u32, l: u32, out: u32) {
        lb::read_script_value(&mut mem(), this, l, out);
    }
    /// `PClassDescriptorDynamic::addMember(member)`.
    Phyre_PClassDescriptorDynamic_addMember_00449b90 @ 0x0044_9b90 => extern "thiscall" fn add_member(this: u32, member: u32) -> u32 {
        lb::add_member(&mut mem(), this, member)
    }
    /// `PClassDescriptorDynamic`'s reset.
    Phyre_PClassDescriptorDynamic_reset_00449c80 @ 0x0044_9c80 => extern "thiscall" fn reset(this: u32) -> u32 {
        lb::reset(&mut mem(), this)
    }
    /// `PClassDescriptorDynamic::removeMember(member)`.
    Phyre_PClassDescriptorDynamic_removeMember_00449d50 @ 0x0044_9d50 => extern "thiscall" fn remove_member(this: u32, member: u32) {
        lb::remove_member(&mut mem(), this, member);
    }
    /// `PClassDescriptorDynamic`'s place of a new member `(type, flags, sizer)`.
    Phyre_PClassDescriptorDynamic_placeMember_00449960 @ 0x0044_9960 => extern "thiscall" fn place_new(this: u32, ty: u32, flags: u32, sizer: u32) -> u32 {
        lb::place_new(&mut mem(), this, ty, flags, sizer)
    }
    /// `PClassDescriptorDynamic::addMember(name, type, flags, sizer)`.
    Phyre_PClassDescriptorDynamic_addNamedMember_00449250 @ 0x0044_9250 => extern "thiscall" fn add_named_member(this: u32, name: u32, ty: u32, flags: u32, sizer: u32) -> u32 {
        lb::add_named_member(&mut mem(), this, name, ty, flags, sizer)
    }
    /// The asset registry's lookup `(name)`.
    Phyre_PAssetReference_Registry_find_0045d280 @ 0x0045_d280 => extern "C" fn registry_find(name: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        lb::registry_find(&mut mem(), name, f.at())
    }
    /// The script's instance push `(world, L, name)`.
    Phyre_PWorld_pushInstancesForScript_00442370 @ 0x0044_2370 => extern "C" fn push_instances(world: u32, l: u32, name: u32) -> u32 {
        let mut f = StackFrame::<8>::new();
        lb::push_instances(&mut mem(), world, l, name, f.at())
    }
}
