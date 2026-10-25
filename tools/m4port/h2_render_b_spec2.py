# batch 2: array helpers, more descriptor slots, small methods
FREE = (0x0042fb20, 1, 0)
LOOKUP = (0x0043cbb0, 0, 0)
SPEC = []


def add(*r):
    SPEC.append(r)


def h(a):
    return f'0x{a >> 16:04x}_{a & 0xffff:04x}'


def w(off, v=0):
    vs = h(v) if v > 0xffff else f'{v:#x}'
    return f'Init::Word({off:#x}, {vs})'


ONE = 0x3f800000

# constructInstance through a constructor, behind an SEH frame (dropped:
# nothing on these paths throws)
for a, f in [(0x00502840, 0x005841a0), (0x00503a80, 0x00583420), (0x005422e0, 0x0053e720),
             (0x005584b0, 0x005572e0), (0x00561d60, 0x0055dc40), (0x00561df0, 0x0055dd30),
             (0x00561e40, 0x004337d0), (0x0056c9c0, 0x00567110), (0x0058e600, 0x00583240),
             (0x0058e720, 0x00583c00), (0x0058e7d0, 0x005841a0), (0x0058e8b0, 0x005842c0),
             (0x0058e900, 0x00584400)]:
    add(a, 'T', 'pn', 'u8', f'd::destruct_with_if(M, a1, {h(f)})', [(f, 0, 0)], None)
add(0x0058e6c0, 'T', 'pn', 'u8', 'd::construct_with_ctor(M, a1, 0x004b_e870, &[0x20])',
    [(0x004be870, 0, 0)], None)
add(0x0058ea30, 'T', 'pn', 'u8', 'd::construct_registered(M, a1, 0x00c9_1048, 0x0058_3f20, &[0x4e0, 0x4e4, 0x4e8, 0x4ec])',
    [LOOKUP, (0x00583f20, 0, 0)], None)
add(0x004e4740, 'C', 'n', 'u8',
    'd::construct_typed(M, a0, 0x00ca_66e8, &[' + ', '.join([w(o) for o in (4, 8, 0xc, 0x10, 0x14, 0x18)]
                                                           + ['Init::Half(0x1c, 0)'] + [w(o) for o in (0x20, 0x24, 0x28, 0x2c, 0x30)]) + '])',
    [LOOKUP], None)
add(0x00501500, 'T', 'pn', 'u8', 'd::construct_two_types(M, a1, 0x00c9_2090, 0x00ca_7228)', [], 'prep_two_types')
for a, f1, f2 in [(0x00592830, 0x0059c770, 0x004be970), (0x00592890, 0x0059c830, 0x00585610),
                  (0x005928f0, 0x0059c890, 0x00585610), (0x00592950, 0x0059c8f0, 0x00585610)]:
    add(a, 'T', 'pp', 'u8', f'd::destruct_with2(M, a1, {h(f1)}, {h(f2)})', [(f1, 0, 0), (f2, 0, 0)], None)
add(0x005028c0, 'T', 'pp', 'u8', 'd::destruct_bound(M, a1, 0x0059_5f80, 0x0059_ca60, 0x0058_57b0)',
    [(0x00595f80, 0, 0), (0x0059ca60, 0, 0), (0x005857b0, 0, 0)], None)

# plain constructInstance
add(0x004dffb0, 'T', 'pn', 'u8', 'd::construct(M, a1, &[' + ', '.join(w(4 * i) for i in range(8)) + '])', [], None)
add(0x004dff50, 'T', 'pn', 'u8', 'd::construct(M, a1, &[Init::Address(0, 0x00c1_6bd4), ' + ', '.join(w(4 * i) for i in range(1, 9)) + ', Init::Byte(0x24, 0)])', [], None)
add(0x0056c920, 'T', 'pn', 'u8', 'd::construct(M, a1, &[' + ', '.join([w(0), 'Init::Float(0x8, 0x00b3_04f4)'] + [w(o) for o in (4, 0xc, 0x10, 0x14, 0x18, 0x1c, 0x20, 0x24)]) + '])', [], None)
add(0x0056c970, 'T', 'pn', 'u8', 'd::construct(M, a1, &[' + ', '.join([w(0), w(4), w(8), w(0xc, ONE), w(0x18, ONE), w(0x1c, ONE), w(0x10), w(0x14),
                                                                       'Init::Float(0x20, 0x00b3_04f8)', 'Init::Float(0x24, 0x00b3_04f8)',
                                                                       'Init::Float(0x28, 0x00b3_04fc)', w(0x2c)]) + '])', [], None)
add(0x00570150, 'T', 'pn', 'u8', 'd::construct(M, a1, &[' + ', '.join([w(0), 'Init::Float(0x14, 0x00b3_0710)', w(4), w(8), w(0x18), w(0x1c),
                                                                       w(0xc, 0xffffffff), w(0x10), 'Init::Half(0x20, 0)']) + '])', [], None)

# PSharray frees (destructInstance slots)
add(0x004f7750, 'T', 'pp', 'u8', 'a::destruct_sharray(M, a1, 0, false)', [FREE], None)
add(0x004f77b0, 'T', 'pp', 'u8', 'a::destruct_sharray(M, a1, 0x1c, false)', [FREE], None)
for a in [0x00562310, 0x00562360]:
    add(a, 'T', 'pp', 'u8', 'a::destruct_sharray(M, a1, 0, true)', [FREE], None)
add(0x004ec310, 'C', 'nc', '', 'a::free_items(M, a0, a1, 8, 0, true)', [FREE], None)
add(0x004f4fc0, 'C', 'nc', '', 'a::free_items(M, a0, a1, 0x24, 0x1c, false)', [FREE], None)
add(0x004f6380, 'C', 'pc', '', 'a::init_items(M, a0, a1, 0x24, &[' + ', '.join([w(0x10), w(0), w(4), w(8), w(0xc), w(0x14, 0xffffffff), w(0x18), w(0x1c), w(0x20)]) + '])', [], None)
add(0x0058b070, 'C', 'pc', '', 'a::init_items(M, a0, a1, 0x1c, &[' + ', '.join(w(4 * i) for i in range(7)) + '])', [], None)
for a, f in [(0x004e1610, 0x004dd770), (0x004e7950, 0x004e4df0), (0x004f0420, 0x004ec310), (0x0059b9d0, 0x00588190)]:
    add(a, 'T', 'pvv', 'u32', f'a::set_storage(M, a0, a1, a2, Some({h(f)}))', [(f, 2, 0)], 'prep_storage')
for a in [0x004f02e0, 0x004f0330, 0x004f0380, 0x004f03d0, 0x004fcc00]:
    add(a, 'T', 'pvv', 'u32', 'a::set_storage(M, a0, a1, a2, None)', [FREE], 'prep_storage')
add(0x004db710, 'T', 'p', '', 'a::clear_with_dtor(M, a0, 0x004b_bf50, 0x20)', [(0x004bbf50, 0, 0), FREE], 'prep_small_count')
add(0x004e3830, 'T', 'p', '', 'a::clear_with_dtor(M, a0, 0x004d_b9d0, 0x28)', [(0x004db9d0, 0, 0), FREE], 'prep_small_count')
add(0x0055e0f0, 'T', 'pv', 'u32', 'a::element(M, a0, a1, 0x14)', [], None)
add(0x00585c90, 'T', 'pv', 'u32', 'a::element(M, a0, a1, 0x1c)', [], None)
add(0x00593af0, 'T', 'pi', 'u32', 'a::bound_element_word(M, a0, a1)', [], 'prep_bound_element')

# misc
add(0x004dd860, 'C', 'n', 'u32', 'x::find_pass_type(M, a0)', [], 'prep_pass_names')
add(0x004dd8c0, 'C', 'n', 'u32', 'x::find_pass_type_by(M, a0, 0x00b0_c398)', [], 'prep_pass_names_import')
add(0x004db230, 'T', 'pnb', 'u32', 'x::pass_type_init(M, a0, a1, a2, 0x004d_d860)', [(0x004dd860, 1, 0)], 'prep_pass_types')
add(0x004db280, 'T', 'pvn', 'u32', 'x::pass_type_init(M, a0, a2, 1, 0x004d_d860)', [(0x004dd860, 1, 0)], 'prep_pass_types')
add(0x004ea7a0, 'T', 'pn', 'u8', 'x::name_is(M, a0, a1)', [], 'prep_name_is')
add(0x00500bd0, 'T', 'ppp', 'u32', 'x::set_min_and_max(M, a0, a1, a2)', [], None)
add(0x00504ec0, 'T', 'p', 'u32', 'x::init_fields(M, a0, &[' + ', '.join(w(4 * i, ONE if 4 * i in (0x10, 0x24, 0x38, 0x4c) else 0) for i in range(0x15)) + '])', [], None)
add(0x00583100, 'T', 'pv', 'u32', 'x::init_fields(M, a0, &[' + ', '.join([w(0, 0x15), w(0x10), w(4, 3), w(8, 3), w(0xc, 3), w(0x14, 1), w(0x18, 1),
                                                                         w(0x1c, ONE), w(0x20, ONE), w(0x24, ONE), w(0x28, ONE),
                                                                         'Init::Float(0x2c, 0x00b1_f2d0)', 'Init::Float(0x30, 0x00b1_7504)']) + '])', [], None)
add(0x00583a60, 'T', 'pv', 'u32', 'x::ctor_arg_then_zero(M, a0, a1, 0x004c_78a0, &[0x10, 0x14, 0x18, 0x1c, 0x20, 0x24])', [(0x004c78a0, 1, 1)], None)
add(0x00583b80, 'T', 'pv', 'u32', 'x::first_then_zero(M, a0, a1, 7)', [], None)
add(0x004fedc0, 'T', 'ppn', 'u32', 'x::set_value(M, a0, a1, a2, 0x0094_9256)', [(0x00949256, 3, 0)], 'prep_set_value')
add(0x004ef3f0, 'T', 'pvvvvvv', 'u32', 'x::build_node_context_key(M, a0, [a1, a2, a3, a4, a5, a6])', [], 'prep_context_switches')
add(0x00581670, 'T', 'pp', 'u32', 'x::tree_find(M, a0, a1)', [], 'prep_tree')
add(0x0058a9a0, 'C', 'i', 'u32', 'x::map_mode8(a0)', [], None)
add(0x0055bc40, 'C', 'w', 'u32', 'x::format_size(a0)', [], None)
