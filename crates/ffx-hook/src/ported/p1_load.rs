//! `ffx-pload`: `PhyreEngine`'s serialization and cluster loading (M4
//! port round 1, list `p1-load`).
//!
//! Threads: the type system runs at start-up on the main thread; the
//! loader (`PBinary`, the streams) on the loader thread too. No shim keeps
//! state: each works on the game's objects through [`mem`].
//!
//! The descriptors' vtable slots that are the same few bytes in many
//! classes are each their own original, so each gets its own line.

use super::{StackFrame, mem, ported};
use ffx_pload::{cluster, descriptor, stream};

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
    /// `PClassDataMember::listsClass` copy 1.
    Phyre_PClassDataMember_listsClass_00465830 @ 0x0046_5830 => extern "thiscall" fn lists_class_5830(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 2.
    Phyre_PClassDataMember_listsClass_00465860 @ 0x0046_5860 => extern "thiscall" fn lists_class_5860(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 3.
    Phyre_PClassDataMember_listsClass_00465890 @ 0x0046_5890 => extern "thiscall" fn lists_class_5890(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 4.
    Phyre_PClassDataMember_listsClass_004658c0 @ 0x0046_58c0 => extern "thiscall" fn lists_class_58c0(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 5.
    Phyre_PClassDataMember_listsClass_004658f0 @ 0x0046_58f0 => extern "thiscall" fn lists_class_58f0(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 6.
    Phyre_PClassDataMember_listsClass_00465920 @ 0x0046_5920 => extern "thiscall" fn lists_class_5920(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 7.
    Phyre_PClassDataMember_listsClass_00465950 @ 0x0046_5950 => extern "thiscall" fn lists_class_5950(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 8.
    Phyre_PClassDataMember_listsClass_00465980 @ 0x0046_5980 => extern "thiscall" fn lists_class_5980(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 9.
    Phyre_PClassDataMember_listsClass_004659b0 @ 0x0046_59b0 => extern "thiscall" fn lists_class_59b0(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 10.
    Phyre_PClassDataMember_listsClass_004659e0 @ 0x0046_59e0 => extern "thiscall" fn lists_class_59e0(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 11.
    Phyre_PClassDataMember_listsClass_00465a10 @ 0x0046_5a10 => extern "thiscall" fn lists_class_5a10(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// `PClassDataMember::listsClass` copy 12.
    Phyre_PClassDataMember_listsClass_00465a40 @ 0x0046_5a40 => extern "thiscall" fn lists_class_5a40(this: u32) -> u8 {
        descriptor::bridge::member_lists_class(&mem(), this)
    }
    /// Registers `PClusterHeader`'s type and adds it to its namespace.
    Phyre_PSerialization_PBinary_PClusterHeader_Register @ 0x0046_8870 => extern "C" fn register_cluster_header() -> u32 {
        descriptor::bridge::register_cluster_header(&mut mem())
    }
    /// `PAnimationKeyDataType`'s parser `(out, name)`.
    Phyre_PAnimation_PAnimationKeyDataType_parse @ 0x0050_7b50 => extern "C" fn parse_key_data_type(out: u32, name: u32) -> u32 {
        descriptor::bridge::parse_key_data_type(&mut mem(), out, name)
    }

    // ---- streams, start-up ----
    /// `PNamespacePacked::needsEndianSwap()`.
    Phyre_PSerialization_PBinary_PInternal_PNamespacePacked_needsEndianSwap @ 0x0042_b830 => extern "thiscall" fn needs_endian_swap(this: u32) -> u32 {
        stream::bridge::needs_endian_swap(&mem(), this)
    }
    /// `PStreamReaderFile::close()`.
    Phyre_PSerialization_PStreamReaderFile_close @ 0x0057_6930 => extern "thiscall" fn reader_close(this: u32) -> u32 {
        stream::bridge::reader_close(&mut mem(), this)
    }
    /// `PStreamReaderFile::isOpen()`.
    Phyre_PSerialization_PStreamReaderFile_isOpen @ 0x0057_6980 => extern "thiscall" fn reader_is_open(this: u32) -> u32 {
        stream::bridge::reader_is_open(&mut mem(), this)
    }
    /// `PStreamReaderFile::read(buffer, size)`.
    Phyre_PSerialization_PStreamReaderFile_read @ 0x0057_6950 => extern "thiscall" fn reader_file_read(this: u32, buffer: u32, size: u32) -> u32 {
        stream::bridge::reader_read(&mut mem(), this, buffer, size)
    }
    /// `PStreamWriterFile::close()`.
    Phyre_PSerialization_PStreamWriterFile_close @ 0x0043_1840 => extern "thiscall" fn writer_close(this: u32) -> u32 {
        stream::bridge::writer_close(&mut mem(), this)
    }
    /// `PStream::~PStream()`.
    Phyre_PSerialization_PStream_dtor @ 0x0060_f0c0 => extern "thiscall" fn stream_dtor(this: u32) {
        stream::bridge::stream_dtor(&mut mem(), this);
    }
    /// `PStreamReader::~PStreamReader()`.
    Phyre_PSerialization_PStreamReader_dtor @ 0x0057_dc70 => extern "thiscall" fn reader_dtor(this: u32) {
        stream::bridge::reader_dtor(&mut mem(), this);
    }
    /// `PStreamReader::PStreamReader(name)`.
    Phyre_PSerialization_PStreamReader_PStreamReader @ 0x0057_dc40 => extern "thiscall" fn reader_ctor(this: u32, name: u32) -> u32 {
        stream::bridge::reader_ctor(&mut mem(), this, name)
    }
    /// `PStreamReader::read(buffer, size)`.
    Phyre_PSerialization_PStreamReader_read @ 0x0057_dd10 => extern "thiscall" fn reader_read(this: u32, buffer: u32, size: u32) -> u32 {
        stream::bridge::reader_read_counted(&mut mem(), this, buffer, size)
    }
    /// `PStreamReader::readExact(buffer, size)`.
    Phyre_PSerialization_PStreamReader_readExact @ 0x0057_dd30 => extern "thiscall" fn reader_read_exact(this: u32, buffer: u32, size: u32) -> u32 {
        stream::bridge::reader_read_exact(&mut mem(), this, buffer, size)
    }
    /// `PStreamFile::SetStreamPrefix(prefix)`.
    Phyre_PSerialization_PStreamFile_SetStreamPrefix @ 0x0060_7d40 => extern "C" fn set_stream_prefix(prefix: u32) -> u32 {
        stream::bridge::set_stream_prefix(&mut mem(), prefix)
    }
    /// `PUtilitySerialization::IsInitialized()`.
    Phyre_PSerialization_PUtilitySerialization_IsInitialized @ 0x0043_2020 => extern "C" fn serialization_ready() -> u8 {
        stream::bridge::serialization_ready(&mem())
    }
    /// `PUtilitySerialization::initialize()`.
    Phyre_PSerialization_PUtilitySerialization_initialize @ 0x0043_20e0 => extern "C" fn serialization_init() -> u32 {
        stream::bridge::serialization_init(&mut mem())
    }
    /// `PAssetReference::InitializeAssetTree()`.
    Phyre_PAssetReference_InitializeAssetTree @ 0x0045_d600 => extern "C" fn init_asset_tree() -> u32 {
        stream::bridge::init_asset_tree(&mut mem())
    }
    /// `PUtilityGame::initialize()`.
    Phyre_PGame_PUtilityGame_initialize @ 0x009f_4c80 => extern "C" fn game_init() -> u32 {
        stream::bridge::game_init(&mut mem())
    }
    /// `PPlatform::GetPlatformId()`: `'TLLB'`.
    Phyre_PPlatform_PPlatform_GetPlatformId @ 0x005f_0490 => extern "C" fn platform_id() -> u32 {
        stream::PLATFORM_ID
    }

    // ---- clusters ----
    /// `PNamespace::PNamespace()`.
    Phyre_PNamespace_PNamespace @ 0x0043_dd80 => extern "thiscall" fn namespace_ctor(this: u32) -> u32 {
        cluster::bridge::namespace_ctor(&mut mem(), this)
    }
    /// `0x00576630`: `PClusterBase::PClusterBase()`, returns `this`.
    Phyre_PCluster_operator_new @ 0x0057_6630 => extern "thiscall" fn cluster_base_init(this: u32) -> u32 {
        cluster::bridge::cluster_base_init(&mut mem(), this)
    }
    /// `PCluster::PCluster()`.
    Phyre_PCluster_PCluster @ 0x0043_f800 => extern "thiscall" fn cluster_ctor(this: u32) -> u32 {
        cluster::bridge::cluster_ctor(&mut mem(), this)
    }
    /// `PClusterHeader::PClusterHeader()`.
    Phyre_PSerialization_PBinary_PClusterHeader_PClusterHeader @ 0x0046_9980 => extern "thiscall" fn header_ctor(this: u32) -> u32 {
        cluster::bridge::header_ctor(&mut mem(), this)
    }
    /// `PNamespaceMapping` set-up `(a, b, c)`.
    Phyre_PSerialization_PBinary_PInternal_PNamespaceMapping_init @ 0x0047_87e0 => extern "thiscall" fn mapping_init(this: u32, a: u32, b: u32, c: u32) -> u32 {
        cluster::bridge::mapping_init(&mut mem(), this, a, b, c)
    }
    /// `0x00465aa0(a, b, c)`: `loadClusterTextures`.
    Phyre_PBinary_loadClusterTextures_thunk @ 0x0046_5aa0 => extern "thiscall" fn load_cluster_textures(this: u32, a: u32, b: u32, c: u32) -> u32 {
        cluster::bridge::load_cluster_textures(&mut mem(), this, a, b, c)
    }
    /// `0x00593bd0`: `+0x20`'s low 31 bits, at least 1.
    Phyre_countAtLeastOne_00593bd0 @ 0x0059_3bd0 => extern "thiscall" fn count_at_least_one(this: u32) -> u32 {
        cluster::bridge::count_at_least_one(&mem(), this)
    }
    /// `0x00595fa0`: whether `+0x64` is set.
    Phyre_hasWord64_00595fa0 @ 0x0059_5fa0 => extern "thiscall" fn has_word_64_a(this: u32) -> u32 {
        cluster::bridge::has_word_64(&mem(), this)
    }
    /// `0x00595fb0`: whether `+0x64` is set.
    Phyre_hasWord64_00595fb0 @ 0x0059_5fb0 => extern "thiscall" fn has_word_64_b(this: u32) -> u32 {
        cluster::bridge::has_word_64(&mem(), this)
    }
    // ---- batch 3: cluster lists, arrays, files ----
    /// `PCluster::allocateInstanceList(n, desc, c)`.
    Phyre_PCluster_allocateInstanceList @ 0x0044_0b70 => extern "thiscall" fn allocate_instance_list(this: u32, n: u32, desc: u32, c: u32) -> u32 {
        cluster::bridge::allocate_instance_list(&mut mem(), this, n, desc, c)
    }
    /// The cluster arena's allocator `(size, align)`.
    Phyre_PCluster_arenaAlloc_00440cf0 @ 0x0044_0cf0 => extern "thiscall" fn arena_alloc(this: u32, size: u32, align: u32) -> u32 {
        cluster::bridge::arena_alloc(&mut mem(), this, size, align)
    }
    /// A list's entry by key `(key)`.
    Phyre_PList_findByKey_00575d40 @ 0x0057_5d40 => extern "thiscall" fn find_by_key(this: u32, key: u32) -> u32 {
        cluster::bridge::find_by_key(&mem(), this, key)
    }
    /// Unlinks every entry of a list.
    Phyre_PList_clear_005758e0 @ 0x0057_58e0 => extern "thiscall" fn clear_list(this: u32) -> u32 {
        cluster::bridge::clear_list(&mut mem(), this)
    }
    /// The cluster header's magic and size check.
    Phyre_PSerialization_PBinary_PClusterHeader_isValid_0047f9d0 @ 0x0047_f9d0 => extern "thiscall" fn header_is_valid(this: u32) -> u8 {
        cluster::bridge::header_is_valid(&mem(), this)
    }
    /// `(a, header, c, d)`: the header's three words.
    Phyre_PBinary_copyHeaderWords_004647a0 @ 0x0046_47a0 => extern "thiscall" fn copy_header_words(this: u32, _a: u32, header: u32, _c: u32, _d: u32) -> u32 {
        cluster::bridge::copy_header_words(&mut mem(), this, header)
    }
    /// Red-black tree lookup `(&key)`, copy 1.
    Phyre_PRedBlackTree_find_00576ef0 @ 0x0057_6ef0 => extern "thiscall" fn tree_find_a(this: u32, key: u32) -> u32 {
        cluster::bridge::tree_find(&mem(), this, key)
    }
    /// Red-black tree lookup `(&key)`, copy 2.
    Phyre_PRedBlackTree_find_00581620 @ 0x0058_1620 => extern "thiscall" fn tree_find_b(this: u32, key: u32) -> u32 {
        cluster::bridge::tree_find(&mem(), this, key)
    }
    /// `PArray<u8>::setData(data, count)`.
    Phyre_PArray_u8_setData_0044a2d0 @ 0x0044_a2d0 => extern "thiscall" fn array_set_data_u8(this: u32, data: u32, count: i32) -> u32 {
        cluster::bridge::array_set_data(&mut mem(), this, data, count)
    }
    /// `PArray<PBitmapFontCharInfo>::setData(data, count)`.
    Phyre_PArray_PBitmapFontCharInfo_setData_00a2db70 @ 0x00a2_db70 => extern "thiscall" fn array_set_data_char_info(this: u32, data: u32, count: i32) -> u32 {
        cluster::bridge::array_set_data(&mut mem(), this, data, count)
    }
    /// `PClassDescriptor<PArray<u8>>::destructInstance(p)`.
    Phyre_PClassDescriptor_PArray_u8_destructInstance @ 0x0044_9540 => extern "stdcall" fn array_destroy_9540(p: u32) -> u8 {
        cluster::bridge::array_destroy(&mut mem(), p)
    }
    /// `PClassDescriptor<PArray<PTypedObject>>::destructInstance(p)`.
    Phyre_PClassDescriptor_PArray_PTypedObject_destructInstance @ 0x0045_9670 => extern "stdcall" fn array_destroy_9670(p: u32) -> u8 {
        cluster::bridge::array_destroy(&mut mem(), p)
    }
    /// `PClassDescriptor<PMorphModifierWeightsUserDataObject>::destructInstance(p)`.
    Phyre_PClassDescriptor_PMorphModifierWeightsUserDataObject_destructInstance @ 0x005c_baa0 => extern "stdcall" fn array_destroy_baa0(p: u32) -> u8 {
        cluster::bridge::array_destroy(&mut mem(), p)
    }
    /// `PClassDescriptor<PArray<PBitmapFontCharInfo>>::destructInstance(p)`.
    Phyre_PClassDescriptor_PArray_PBitmapFontCharInfo_destructInstance @ 0x00a2_d5d0 => extern "stdcall" fn array_destroy_d5d0(p: u32) -> u8 {
        cluster::bridge::array_destroy(&mut mem(), p)
    }
    /// `PClassDescriptorConcrete<PBitmapFontCharInfo>::constructInstance(p)`.
    Phyre_PClassDescriptorConcrete_PBitmapFontCharInfo_constructInstance @ 0x00a2_d520 => extern "stdcall" fn char_info_ctor(p: u32) -> u32 {
        cluster::bridge::char_info_ctor(&mut mem(), p)
    }
    /// `PDepthOfField::PDepthOfField()`.
    Phyre_PPostProcessing_PDepthOfField_PDepthOfField @ 0x009f_bd40 => extern "thiscall" fn depth_of_field_ctor(this: u32) -> u32 {
        cluster::bridge::depth_of_field_ctor(&mut mem(), this)
    }
    /// Creates the object's data when its first word is 0.
    Phyre_ensureCreated_0058dba0 @ 0x0058_dba0 => extern "thiscall" fn ensure_created(this: u32) -> u32 {
        cluster::bridge::ensure_created(&mut mem(), this)
    }
    /// `0x0043be30(out, name)`: the class descriptor named `name`.
    Phyre_PClassDescriptor_findByName_0043be30 @ 0x0043_be30 => extern "C" fn find_class_descriptor(out: u32, name: u32) -> u32 {
        descriptor::bridge::find_class_descriptor(&mut mem(), out, name)
    }
    /// An enum value table's registration `(entry)`.
    Phyre_PTypeEnum_registerEntry_00507ad0 @ 0x0050_7ad0 => extern "C" fn register_enum_entry(entry: u32) -> u32 {
        descriptor::bridge::register_enum_entry(&mut mem(), entry)
    }
    /// Fills `n` 0x14-byte entries `(at, n)`.
    Phyre_PClassDescriptor_initEntries_0043bcf0 @ 0x0043_bcf0 => extern "C" fn init_entries(at: u32, n: u32) {
        descriptor::bridge::init_entries(&mut mem(), at, n);
    }
    /// Deletes a namespace's owned class descriptors.
    Phyre_PNamespace_releaseOwned_0043dde0 @ 0x0043_dde0 => extern "thiscall" fn release_owned(this: u32) {
        descriptor::bridge::release_owned(&mut mem(), this);
    }
    /// `PClassDataMemberArray::PClassDataMemberArray(a, b, c, d, e, f, g)`.
    Phyre_PClassDataMemberArray_PClassDataMemberArray @ 0x0057_5ef0 => extern "thiscall" fn member_array_ctor(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32) -> u32 {
        descriptor::bridge::member_array_ctor(&mut mem(), this, [a, b, c, d, e, f, g])
    }
    /// `PClassDataMemberArrayFixedSize::PClassDataMemberArrayFixedSize(a, ty, c, d, e, size)`.
    Phyre_PClassDataMemberArrayFixedSize_PClassDataMemberArrayFixedSize @ 0x0057_5f30 => extern "thiscall" fn member_array_fixed_ctor(this: u32, a: u32, ty: u32, c: u32, d: u32, e: u32, size: u32) -> u32 {
        descriptor::bridge::member_array_fixed_ctor(&mut mem(), this, [a, ty, c, d, e, size])
    }
    /// An instance list's instance data `(i)`.
    Phyre_PInstanceList_instanceData_005765e0 @ 0x0057_65e0 => extern "thiscall" fn instance_data(this: u32, i: u32) -> u32 {
        descriptor::bridge::instance_data(&mem(), this, i)
    }
    /// `PAssetReferenceImport::ResolveFixup(slot, id, c, name)`.
    Phyre_PAssetReferenceImport_ResolveFixup @ 0x0045_d9b0 => extern "C" fn resolve_asset_fixup(slot: u32, id: u32, c: u32, name: u32) -> u32 {
        descriptor::bridge::resolve_asset_fixup(&mut mem(), slot, id, c, name)
    }
    /// `PStreamFile::PStreamFile(path, b, c, d, e)`.
    Phyre_PSerialization_PStreamFile_ctor @ 0x0060_7bc0 => extern "thiscall" fn file_ctor(this: u32, path: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        stream::bridge::file_ctor(&mut mem(), this, [path, b, c, d, e])
    }
    /// `PStreamFile::closeFile()`.
    Phyre_PSerialization_PStreamFile_closeFile @ 0x0060_7d80 => extern "thiscall" fn close_file(this: u32) -> u32 {
        stream::bridge::close_file(&mut mem(), this)
    }
    /// `PStreamFile::read(buffer, size)`; `ReadFile`'s count in this frame.
    Phyre_PSerialization_PStreamFile_read @ 0x0060_8090 => extern "thiscall" fn file_read(this: u32, buffer: u32, size: u32) -> u32 {
        let mut count = StackFrame::<1>::new();
        stream::bridge::file_read(&mut mem(), count.at(), this, buffer, size)
    }
}
