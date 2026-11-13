//! M5 finish, WP5 part 5 (wp5f): the last `FFX.exe` frontier: dlmalloc's
//! heap growth and release (`ffx_sg::m5main4_lib`; parity tests in
//! `crates/ffx-oracle/tests/m5_main4.rs`, `dlmalloc_*`), the menu screens'
//! layout reads and the main menu's help bar (`ffx_menu::wp5f_menu`;
//! `crates/ffx-oracle/tests/m5_menu.rs`, `wp5f_*`).
//!
//! Threads: the heaps are the game's `mspace`s (the cloth heap on the
//! cloth workers, the graphics heap on the main thread); the menu runs on
//! the main thread. No shim keeps state; the region queries' buffers are
//! `StackFrame`s (7 words).

use super::{StackFrame, mem, ported};
use ffx_menu::wp5f_menu::bridge as menu;
use ffx_prender::wp5f::bridge as pr;
use ffx_sg::m5main4_lib::bridge as lib;
use ffx_system::wp5f_sstream::bridge as ss;

ported! {
    /// `init_top(m, p, psize)`.
    dlmalloc_init_top_006ea140 @ 0x006e_a140 => extern "C" fn init_top(m: u32, p: u32, psize: u32) {
        lib::init_top(&mut mem(), m, p, psize);
    }
    /// `segment_holding(m, p)` -> the segment record or 0.
    dlmalloc_segment_holding_006eb1a0 @ 0x006e_b1a0 => extern "C" fn segment_holding(m: u32, p: u32) -> u32 {
        lib::segment_holding(&mut mem(), m, p)
    }
    /// `mmap_alloc(m, nb)` -> the chunk's memory or 0.
    dlmalloc_mmap_alloc_006eaa30 @ 0x006e_aa30 => extern "C" fn mmap_alloc(m: u32, nb: u32) -> u32 {
        lib::mmap_alloc(&mut mem(), m, nb)
    }
    /// `prepend_alloc(m, newbase, oldbase, nb)` -> the chunk's memory.
    dlmalloc_prepend_alloc_006eab10 @ 0x006e_ab10 => extern "C" fn prepend_alloc(m: u32, newbase: u32, oldbase: u32, nb: u32) -> u32 {
        lib::prepend_alloc(&mut mem(), m, newbase, oldbase, nb)
    }
    /// `add_segment(m, tbase, tsize, mmapped)`.
    dlmalloc_add_segment_006e9ae0 @ 0x006e_9ae0 => extern "C" fn add_segment(m: u32, tbase: u32, tsize: u32, mmapped: u32) {
        lib::add_segment(&mut mem(), m, tbase, tsize, mmapped);
    }
    /// `sys_alloc(m, nb)` -> the chunk's memory or 0.
    dlmalloc_sys_alloc_006eb1d0 @ 0x006e_b1d0 => extern "C" fn sys_alloc(m: u32, nb: u32) -> u32 {
        lib::sys_alloc(&mut mem(), m, nb)
    }
    /// `sys_trim(m, pad)` -> 1 when memory went back; the region query in
    /// a frame.
    dlmalloc_sys_trim_006eb3e0 @ 0x006e_b3e0 => extern "C" fn sys_trim(m: u32, pad: u32) -> u32 {
        let mut f = StackFrame::<7>::new();
        lib::sys_trim(&mut mem(), m, pad, f.at())
    }
    /// `release_unused_segments(m)` -> the bytes released; the region
    /// query in a frame.
    dlmalloc_release_unused_segments_006eae70 @ 0x006e_ae70 => extern "C" fn release_unused_segments(m: u32) -> u32 {
        let mut f = StackFrame::<7>::new();
        lib::release_unused_segments(&mut mem(), m, f.at())
    }
    /// `tmalloc_large(m, nb)` -> the chunk's memory or 0.
    dlmalloc_tmalloc_large_006eb540 @ 0x006e_b540 => extern "C" fn tmalloc_large(m: u32, nb: u32) -> u32 {
        lib::tmalloc_large(&mut mem(), m, nb)
    }

    /// `MenuConfig_InitLayout_008cc050()` -> the last colour.
    MenuConfig_InitLayout_008cc050 @ 0x008c_c050 => extern "C" fn config_init_layout() -> u32 {
        menu::config_init_layout(&mut mem())
    }
    /// `MenuEquip_InitLayout_008cfc40()` -> the last call's `eax`.
    MenuEquip_InitLayout_008cfc40 @ 0x008c_fc40 => extern "C" fn equip_init_layout() -> u32 {
        menu::equip_init_layout(&mut mem())
    }
    /// `MenuLimit_InitLayout_008d0cd0()` -> the last call's `eax`.
    MenuLimit_InitLayout_008d0cd0 @ 0x008d_0cd0 => extern "C" fn limit_init_layout() -> u32 {
        menu::limit_init_layout(&mut mem())
    }
    /// `MenuKaizou_InitLayout_008d6be0()` -> the last call's `eax`.
    MenuKaizou_InitLayout_008d6be0 @ 0x008d_6be0 => extern "C" fn kaizou_init_layout() -> u32 {
        menu::kaizou_init_layout(&mut mem())
    }
    /// `TkMenuMain_DrawHelpBar_008a9350(window)` -> the mask or
    /// `TkVU1SyncPath`'s `eax`.
    TkMenuMain_DrawHelpBar_008a9350 @ 0x008a_9350 => extern "C" fn draw_help_bar(_window: u32) -> u32 {
        menu::draw_help_bar(&mut mem())
    }

    /// `std::stringstream()` (`this; 2 unread words`) -> `this`.
    StdStringstream_ctor_00402f00 @ 0x0040_2f00 => extern "thiscall" fn stringstream_ctor(this: u32, _a: u32, _b: u32) -> u32 {
        ss::stringstream_ctor(&mut mem(), this)
    }
    /// The stringstream's vbase destructor (`this` = its `basic_ios`).
    StdStringstream_vbaseDtor_00402fc0 @ 0x0040_2fc0 => extern "thiscall" fn stringstream_vbase_dtor(this: u32) -> u32 {
        ss::stream_vbase_dtor(&mut mem(), this, ss::STRINGSTREAM)
    }
    /// `stringstream::str()` into `out` -> `out`.
    StdStringstream_str_00402ff0 @ 0x0040_2ff0 => extern "thiscall" fn stringstream_str(this: u32, out: u32) -> u32 {
        ss::stream_str(&mut mem(), this, 0x18, out)
    }
    /// `~basic_stringbuf()`'s body -> `~basic_streambuf`'s `eax`.
    StdStringbuf_tidy_00403010 @ 0x0040_3010 => extern "thiscall" fn stringbuf_tidy(this: u32) -> u32 {
        ss::stringbuf_tidy(&mut mem(), this)
    }
    /// `basic_stringbuf::overflow(c)`.
    StdStringbuf_overflow_00403080 @ 0x0040_3080 => extern "thiscall" fn stringbuf_overflow(this: u32, c: u32) -> u32 {
        ss::stringbuf_overflow(&mut mem(), this, c)
    }
    /// `basic_stringbuf::pbackfail(c)`.
    StdStringbuf_pbackfail_00403260 @ 0x0040_3260 => extern "thiscall" fn stringbuf_pbackfail(this: u32, c: u32) -> u32 {
        ss::stringbuf_pbackfail(&mut mem(), this, c)
    }
    /// `basic_stringbuf::underflow()`.
    StdStringbuf_underflow_004032c0 @ 0x0040_32c0 => extern "thiscall" fn stringbuf_underflow(this: u32) -> u32 {
        ss::stringbuf_underflow(&mut mem(), this)
    }
    /// `basic_stringbuf::seekoff(out; off, way, which)` -> `out`.
    StdStringbuf_seekoff_00403320 @ 0x0040_3320 => extern "thiscall" fn stringbuf_seekoff(this: u32, out: u32, off_lo: u32, off_hi: u32, way: u32, which: u32) -> u32 {
        let off = u64::from(off_hi) << 32 | u64::from(off_lo);
        ss::stringbuf_seekoff(&mut mem(), this, out, off, way, which)
    }
    /// `basic_stringbuf::seekpos(out; pos, which)` -> `out` (the
    /// position's state words are not read).
    StdStringbuf_seekpos_004034f0 @ 0x0040_34f0 => extern "thiscall" fn stringbuf_seekpos(this: u32, out: u32, off_lo: u32, off_hi: u32, fpos_lo: u32, fpos_hi: u32, _state: u32, _pad: u32, which: u32) -> u32 {
        let off = u64::from(off_hi) << 32 | u64::from(off_lo);
        let fpos = u64::from(fpos_hi) << 32 | u64::from(fpos_lo);
        ss::stringbuf_seekpos(&mut mem(), this, out, off, fpos, which)
    }
    /// `std::istringstream(str)` (`this; str, 2 unread words`) -> `this`.
    StdIstringstream_ctor_00403620 @ 0x0040_3620 => extern "thiscall" fn istringstream_ctor(this: u32, s: u32, _a: u32, _b: u32) -> u32 {
        ss::istringstream_ctor(&mut mem(), this, s)
    }
    /// The istringstream's vbase destructor.
    StdIstringstream_vbaseDtor_00403700 @ 0x0040_3700 => extern "thiscall" fn istringstream_vbase_dtor(this: u32) -> u32 {
        ss::stream_vbase_dtor(&mut mem(), this, ss::ISTRINGSTREAM)
    }
    /// `std::ostringstream()` (`this; 2 unread words`) -> `this`.
    StdOstringstream_ctor_00403730 @ 0x0040_3730 => extern "thiscall" fn ostringstream_ctor(this: u32, _a: u32, _b: u32) -> u32 {
        ss::ostringstream_ctor(&mut mem(), this)
    }
    /// The ostringstream's vbase destructor.
    StdOstringstream_vbaseDtor_004037e0 @ 0x0040_37e0 => extern "thiscall" fn ostringstream_vbase_dtor(this: u32) -> u32 {
        ss::stream_vbase_dtor(&mut mem(), this, ss::OSTRINGSTREAM)
    }
    /// `ostringstream::str()` into `out` -> `out`.
    StdOstringstream_str_00403810 @ 0x0040_3810 => extern "thiscall" fn ostringstream_str(this: u32, out: u32) -> u32 {
        ss::stream_str(&mut mem(), this, 4, out)
    }
    /// The stringstream's deleting destructor (`this` = its `basic_ios`).
    StdStringstream_deletingDtor_00403830 @ 0x0040_3830 => extern "thiscall" fn stringstream_deleting_dtor(this: u32, flags: u32) -> u32 {
        ss::stream_deleting_dtor(&mut mem(), this, flags, ss::STRINGSTREAM)
    }
    /// The stringbuf's deleting destructor -> `this`.
    StdStringbuf_deletingDtor_00403890 @ 0x0040_3890 => extern "thiscall" fn stringbuf_deleting_dtor(this: u32, flags: u32) -> u32 {
        ss::stringbuf_deleting_dtor(&mut mem(), this, flags)
    }
    /// The istringstream's deleting destructor.
    StdIstringstream_deletingDtor_004038c0 @ 0x0040_38c0 => extern "thiscall" fn istringstream_deleting_dtor(this: u32, flags: u32) -> u32 {
        ss::stream_deleting_dtor(&mut mem(), this, flags, ss::ISTRINGSTREAM)
    }
    /// The ostringstream's deleting destructor.
    StdOstringstream_deletingDtor_00403920 @ 0x0040_3920 => extern "thiscall" fn ostringstream_deleting_dtor(this: u32, flags: u32) -> u32 {
        ss::stream_deleting_dtor(&mut mem(), this, flags, ss::OSTRINGSTREAM)
    }
    /// `basic_stringbuf::str()` into `out` -> `out`; the temporary string
    /// in a frame.
    StdStringbuf_str_00403ec0 @ 0x0040_3ec0 => extern "thiscall" fn stringbuf_str(this: u32, out: u32) -> u32 {
        let mut tmp = StackFrame::<6>::new();
        ss::stringbuf_str(&mut mem(), this, out, tmp.at())
    }
    /// `std::string::push_back(c)` (`this; unread word, c`) -> `this`.
    StdString_pushBack_004041c0 @ 0x0040_41c0 => extern "thiscall" fn string_push_back(this: u32, _a: u32, c: u32) -> u32 {
        ss::string_push_back(&mut mem(), this, c)
    }
    /// `basic_stringbuf::_Init(p, n)`.
    StdStringbuf_init_004045d0 @ 0x0040_45d0 => extern "thiscall" fn stringbuf_init(this: u32, p: u32, n: u32) {
        ss::stringbuf_init(&mut mem(), this, p, n);
    }
    /// `use_facet<ctype<char>>(loc)` -> the facet; the lock, the
    /// `bad_cast` and the cache copy in a frame.
    StdUseFacet_ctype_004049a0 @ 0x0040_49a0 => extern "fastcall" fn use_ctype(loc: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        ss::use_ctype(&mut mem(), loc, f.at())
    }
    /// `os << ','` (`ecx` = the stream) -> `os`.
    StdOstream_putComma_00404a90 @ 0x0040_4a90 => extern "thiscall" fn put_comma(os: u32) -> u32 {
        ss::put_comma(&mut mem(), os)
    }
    /// `os << s` (a C string) -> `os`.
    StdOstream_putCstr_00404ca0 @ 0x0040_4ca0 => extern "fastcall" fn put_cstr(os: u32, s: u32) -> u32 {
        ss::put_cstr(&mut mem(), os, s)
    }
    /// `os << s` (a `std::string`) -> `os`.
    StdOstream_putString_00404ed0 @ 0x0040_4ed0 => extern "fastcall" fn put_string(os: u32, s: u32) -> u32 {
        ss::put_string(&mut mem(), os, s)
    }
    /// `is >> s` (a `std::string`) -> `is`; the locale in a frame.
    StdIstream_getString_00405800 @ 0x0040_5800 => extern "fastcall" fn get_string(is: u32, s: u32) -> u32 {
        let mut loc = StackFrame::<1>::new();
        ss::get_string(&mut mem(), is, s, loc.at())
    }
    /// `Facet_Register(f)` -> the list node.
    Facet_Register @ 0x0094_a684 => extern "C" fn facet_register(facet: u32) -> u32 {
        ss::facet_register(&mut mem(), facet)
    }

    /// `0x006b60f0()`: the singleton at `0x00cccad0` freed.
    GraphicFinalize_freeSingleton_006b60f0 @ 0x006b_60f0 => extern "C" fn free_singleton_cccad0() {
        pr::free_singleton_cccad0(&mut mem());
    }
    /// `0x006b6180()`: the singleton at `0x00cccae0` freed.
    GraphicFinalize_freeSingleton_006b6180 @ 0x006b_6180 => extern "C" fn free_singleton_6180() {
        pr::free_singleton(&mut mem(), pr::SINGLETONS[0].1);
    }
    /// `0x006b61b0()`: the singleton at `0x00cccad8` freed.
    GraphicFinalize_freeSingleton_006b61b0 @ 0x006b_61b0 => extern "C" fn free_singleton_61b0() {
        pr::free_singleton(&mut mem(), pr::SINGLETONS[1].1);
    }
    /// `0x006b61e0()`: the singleton at `0x00cccadc` freed.
    GraphicFinalize_freeSingleton_006b61e0 @ 0x006b_61e0 => extern "C" fn free_singleton_61e0() {
        pr::free_singleton(&mut mem(), pr::SINGLETONS[2].1);
    }
    /// `0x006b6210()`: the singleton at `0x00cccac4` freed.
    GraphicFinalize_freeSingleton_006b6210 @ 0x006b_6210 => extern "C" fn free_singleton_6210() {
        pr::free_singleton(&mut mem(), pr::SINGLETONS[3].1);
    }
    /// `0x006b6240()`: the singleton at `0x00cccad4` freed.
    GraphicFinalize_freeSingleton_006b6240 @ 0x006b_6240 => extern "C" fn free_singleton_6240() {
        pr::free_singleton(&mut mem(), pr::SINGLETONS[4].1);
    }
    /// `0x006b6270()`: the singleton at `0x00cccae4` freed.
    GraphicFinalize_freeSingleton_006b6270 @ 0x006b_6270 => extern "C" fn free_singleton_6270() {
        pr::free_singleton(&mut mem(), pr::SINGLETONS[5].1);
    }
    /// `0x006b62a0()`: the singleton at `0x00cccac8` freed.
    GraphicFinalize_freeSingleton_006b62a0 @ 0x006b_62a0 => extern "C" fn free_singleton_62a0() {
        pr::free_singleton(&mut mem(), pr::SINGLETONS[6].1);
    }
    /// `0x006b62d0()`: the object at `0x00cccac0` deleted.
    GraphicFinalize_freeObject_006b62d0 @ 0x006b_62d0 => extern "C" fn free_object_cccac0() {
        pr::free_object_cccac0(&mut mem());
    }
    /// `0x006dc870()` -> `Input_ShutdownDevice`'s `eax` (0 without a block).
    FreeBlock_cdede8_006dc870 @ 0x006d_c870 => extern "C" fn free_block_cdede8() -> u32 {
        pr::free_block_cdede8(&mut mem())
    }
    /// `0x0041e6a0(this)`: the `IMkvReader` vtable stored.
    MkvReader_baseDtor_0041e6a0 @ 0x0041_e6a0 => extern "thiscall" fn mkv_reader_base_dtor(this: u32) {
        pr::mkv_reader_base_dtor(&mut mem(), this);
    }
    /// `SqsSePlayState()` -> 0.
    SqsSePlayState @ 0x0081_e530 => extern "C" fn sqs_se_play_state() -> u32 {
        pr::sqs_se_play_state(&mut mem())
    }
    /// `0x005f0950(array, n)`.
    ClearRecords_005f0950 @ 0x005f_0950 => extern "C" fn clear_records(array: u32, n: u32) {
        pr::clear_records(&mut mem(), array, n);
    }
    /// `0x006058b0(this; a, xyz, f)` -> `0x006058e0`'s `eax`.
    SetPositionThen_006058b0 @ 0x0060_58b0 => extern "thiscall" fn set_position_then(this: u32, a: u32, xyz: u32, f: u32) -> u32 {
        pr::set_position_then(&mut mem(), this, a, xyz, f)
    }
    /// `SectionVector_grow_006b6500(this; n)` -> `_Reallocate`'s `eax` (the
    /// spare count when there is room).
    SectionVector_grow_006b6500 @ 0x006b_6500 => extern "thiscall" fn section_vector_grow(this: u32, n: u32) -> u32 {
        pr::section_vector_grow(&mut mem(), this, n)
    }
    /// `std::string::find_first_of(set, pos, n)` -> the position or -1.
    std_string_findFirstOf_006b68c0 @ 0x006b_68c0 => extern "thiscall" fn find_first_of(this: u32, set: u32, pos: u32, n: u32) -> u32 {
        pr::find_first_of(&mut mem(), this, set, pos, n)
    }
    /// `0x006b6730(this; name)`: the map name set and changed.
    SetMapName_006b6730 @ 0x006b_6730 => extern "thiscall" fn set_map_name(this: u32, name: u32) {
        pr::set_map_name(&mut mem(), this, name);
    }
    /// `0x00807120(tree)`: the opus tree cleared.
    opusTree_clear_00807120 @ 0x0080_7120 => extern "thiscall" fn opus_tree_clear(tree: u32) {
        pr::opus_tree_clear(&mut mem(), tree);
    }
    /// `0x0094a641()`: the facet list freed.
    FacetList_free_0094a641 @ 0x0094_a641 => extern "C" fn facet_list_free() {
        pr::facet_list_free(&mut mem());
    }
}
