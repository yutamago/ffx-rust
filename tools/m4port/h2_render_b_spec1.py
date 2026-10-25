# batch 1: descriptor slots, array members, small leaves
FREE = (0x0042fb20, 1, 0)
LOOKUP = (0x0043cbb0, 0, 0)
SPEC = []


def add(*r):
    SPEC.append(r)


# fixObjectsAfterLoading / vtable slots returning 0
for a in [0x004d8760, 0x004d8770, 0x004e0150, 0x004e0160, 0x004f7830, 0x004f7840, 0x004f7850,
          0x004fc740, 0x00500710, 0x0056ccd0, 0x0056cce0, 0x00593700, 0x004fa980, 0x00598a60]:
    add(a, 'T', 'pv', 'u32', '0', [], None)
for a in [0x004fa9c0, 0x00598a70]:
    add(a, 'T', 'pvv', 'u32', '0', [], None)
add(0x00598a20, 'T', 'pvvv', 'u32', '0', [], None)
add(0x0058de10, 'C', '', 'u32', '0x17', [], None)
# destructInstance with nothing to do: al = 1
for a in [0x004e00a0, 0x004fde50, 0x00500700, 0x00501550, 0x00502930, 0x005623b0, 0x005623e0,
          0x0056cb60, 0x005701a0, 0x005929d0]:
    add(a, 'T', 'pv', 'u8', '1u8', [], None)


def construct(a, fields):
    fs = ', '.join(fields)
    add(a, 'T', 'pn', 'u8', f'd::construct(M, a1, &[{fs}])', [], None)


def w(off, v=0):
    vs = f'0x{v >> 16:04x}_{v & 0xffff:04x}' if v > 0xffff else f'{v:#x}'
    return f'Init::Word({off:#x}, {vs})'


for a in [0x004dfeb0, 0x004dfed0, 0x004dfef0, 0x004e69c0, 0x004e69e0, 0x004e6a00, 0x004f75f0,
          0x004f7610, 0x004fc610, 0x004fde30, 0x005422c0, 0x00561cc0, 0x00561ce0, 0x00561d00,
          0x0056c8c0, 0x0058e5e0]:
    construct(a, [w(0), w(4)])
for a in [0x004dff10, 0x00561d20, 0x00561db0, 0x0058e790]:
    construct(a, [w(o) for o in (0, 4, 8, 0xc, 0x10)])
for a in [0x004f7670, 0x0056ca40, 0x0058ea90]:
    construct(a, [w(o) for o in (0, 4, 8)])
construct(0x004f7630, [w(0x10), w(0), w(4), w(8), w(0xc), w(0x14, 0xffffffff), w(0x18), w(0x1c), w(0x20)])
construct(0x00500660, [w(o) for o in (0xc, 0x10, 0x1c, 0, 0x14, 4, 0x18, 8)])
construct(0x00502890, [w(0), w(4), w(8), w(0xc), 'Init::Byte(0x10, 0)'])
construct(0x0056c8e0, [w(0), 'Init::Float(0x8, 0x00b3_04f4)', w(4), w(0xc), w(0x10)])
construct(0x0056ca10, [w(0), w(4), w(8), w(0xc, 0x3f800000), w(0x10, 0x3f800000)])

add(0x004e6a20, 'T', 'pn', 'u32', 'd::construct_by(M, a1, 0x004e_4740)', [(0x004e4740, 1, 0)], None)
add(0x004e6d70, 'T', 'pn', 'u32', 'd::construct_by(M, a1, 0x004e_4ec0)', [(0x004e4ec0, 1, 0)], None)
add(0x004fa8c0, 'T', 'pn', 'u8', 'd::construct_owned(M, a1, 0x00cb_b300, 0x00b2_3700)', [LOOKUP], None)
for a in [0x0058e950, 0x0058e990, 0x0058e9d0]:
    add(a, 'T', 'pn', 'u8', 'd::construct_registered(M, a1, 0x00c9_1048, 0x0058_3f20, &[0x4e0])',
        [LOOKUP, (0x00583f20, 0, 0)], None)
add(0x0058e770, 'T', 'pn', 'u8', 'd::destruct_with_if(M, a1, 0x0058_3e20)', [(0x00583e20, 0, 0)], None)
add(0x0058ea10, 'T', 'pn', 'u8', 'd::destruct_with_if(M, a1, 0x0058_3f20)', [(0x00583f20, 0, 0)], None)

for a, f in [(0x004e0040, 0x004db710), (0x004e00b0, 0x004db9d0), (0x004e00d0, 0x004dbaf0),
             (0x004e6d00, 0x004e3830), (0x004e6d20, 0x004e39b0), (0x004f7790, 0x004f2c50),
             (0x005622f0, 0x00433900), (0x005623c0, 0x0055dfa0), (0x005623f0, 0x0055e060),
             (0x0056cb40, 0x00567620), (0x0056cb70, 0x005676c0), (0x00592760, 0x00584dc0),
             (0x005929b0, 0x005853c0), (0x005929e0, 0x00585610), (0x00592a00, 0x005856b0),
             (0x00592a80, 0x005857b0), (0x00592aa0, 0x00585840), (0x00592ac0, 0x005858d0),
             (0x00592ae0, 0x00585960)]:
    add(a, 'T', 'pn', 'u8', f'd::destruct_with(M, a1, 0x{f >> 16:04x}_{f & 0xffff:04x})', [(f, 0, 0)], None)
for a in [0x004fa900, 0x004fc700, 0x005423f0, 0x00558500]:
    add(a, 'T', 'pn', 'u8', 'd::destruct_virtual(M, a1)', [], 'prep_virtual_dtor')

add(0x004e0000, 'T', 'pp', 'u8', 'd::free_array(M, a1, 0, None, true)', [FREE], None)
for a in [0x004e6c80, 0x004fc6c0]:
    add(a, 'T', 'pp', 'u8', 'd::free_array(M, a1, 0, None, false)', [FREE], None)
for a, f in [(0x004e0060, 0x004dd770), (0x004e6cc0, 0x004e4df0), (0x004f7710, 0x004f4fc0),
             (0x005423b0, 0x00540b80), (0x0056ca70, 0x0056a700), (0x00592720, 0x00588190),
             (0x00592a40, 0x00588190)]:
    add(a, 'T', 'pp', 'u8', f'd::free_array(M, a1, 0, Some(0x{f >> 16:04x}_{f & 0xffff:04x}), false)', [(f, 2, 0)], None)
add(0x0056cbb0, 'T', 'pp', 'u8', 'd::free_array(M, a1, 4, Some(0x0056_a700), false)', [(0x0056a700, 2, 0)], None)
add(0x004f77f0, 'T', 'pp', 'u8', 'd::free_if_owned(M, a1, 4)', [FREE], None)
for a in [0x0056cb90, 0x00592a20]:
    add(a, 'T', 'pp', 'u8', 'd::free_if_owned(M, a1, 0)', [FREE], None)
for a in [0x0056a700, 0x00588190]:
    add(a, 'C', 'nc', '', 'd::free_owned_buffers(M, a0, a1)', [FREE], None)

for a in [0x004e1690, 0x004e16c0, 0x004e79f0, 0x004e7a20, 0x004f0470, 0x004f04a0, 0x004f04d0,
          0x004f0500, 0x004f0530, 0x004fcc50, 0x00564380, 0x0059ba20]:
    add(a, 'T', 'pvvv', 'u32', 'd::member_forward(M, a0, a1, a2, a3)', [], 'prep_member')
for a in [0x004eef20, 0x004eef80, 0x004f7c10, 0x004fc780]:
    add(a, 'T', 'pv', 'u32', 'd::member_element(M, a0, a1)', [], 'prep_member_element')
add(0x004f7e20, 'T', 'pv', 'u32', 'd::member_handler34(M, a0, a1)', [], 'prep_member')
for a, f in [(0x004e15f0, 0x004e13a0), (0x004e79a0, 0x004e7780), (0x005642c0, 0x00434e40)]:
    add(a, 'T', 'pvv', 'u32', f'd::set_borrowed(M, a0, a1, a2, 0x{f >> 16:04x}_{f & 0xffff:04x})', [(f, 2, 2)], None)
for a, g in [(0x004fd620, 0x00ca8298), (0x005020e0, 0x00ca8dc8), (0x00503490, 0x00ca90d0),
             (0x005034b0, 0x00ca9038), (0x005619b0, 0x00cb11b8), (0x005619e0, 0x00cb1088)]:
    add(a, 'C', '', 'u32', f'd::register_type(M, 0x{g >> 16:04x}_{g & 0xffff:04x})',
        [(0x004395b0, 1, 0), (0x0043c140, 0, 0)], None)

# misc
add(0x004d8940, 'T', 'pvv', 'u32', 'x::set_pair(M, a0, 0, a1, a2)', [], None)
add(0x004d8960, 'T', 'pvv', 'u32', 'x::set_pair(M, a0, 8, a1, a2)', [], None)
add(0x004df2d0, 'C', '', 'u32', 'x::invalidate_lookup_indices(M)', [], 'prep_pass_types')
add(0x004df9c0, 'C', 'pv', 'u32', 'x::lookup_into(M, a0, a1, 0x004d_d910)', [(0x004dd910, 1, 0)], None)
add(0x004f30d0, 'T', 'ppf', 'u32', 'x::scale4(M, a0, a1, a2)', [], 'prep_scale')
for a, o in [(0x004f7d50, 0xc), (0x00593ba0, 0x14), (0x00593bb0, 0x24), (0x00593bc0, 0x28)]:
    add(a, 'T', 'p', 'u32', f'x::count_or_one(M, a0, {o:#x})', [], None)
add(0x004fa420, 'T', 'pv', 'u32', 'x::dynamic_mesh_instance(M, a0, a1)', [], None)
add(0x004fa470, 'T', 'p', '', 'x::dynamic_mesh_instance_dtor(M, a0)', [], None)
add(0x004fa510, 'T', 'pb', 'u32', 'x::scalar_deleting_dtor(M, a0, a1, None)', [FREE], None)
add(0x00557670, 'T', 'pb', 'u32', 'x::scalar_deleting_dtor(M, a0, a1, Some(0x0055_7410))',
    [(0x00557410, 0, 0), FREE], None)
add(0x004fee50, 'T', 'p', 'u32', 'x::zero_init(M, a0, 4, None)', [], None)
add(0x00501c40, 'T', 'p', 'u32', 'x::zero_init(M, a0, 4, Some(0x10))', [], None)
add(0x005031e0, 'T', 'p', 'u32', 'x::zero_init(M, a0, 2, None)', [], None)
add(0x004fee70, 'T', 'p', 'u32', 'x::call_return_this(M, a0, 0x0058_3580)', [(0x00583580, 0, 0)], None)
add(0x00500c20, 'T', 'ppp', 'u32', 'x::set_min_and_size(M, a0, a1, a2)', [], None)
add(0x00562da0, 'T', 'p', 'u32', 'x::field_bit0(M, a0, 0x34)', [], None)
add(0x00562db0, 'T', 'p', 'u32', 'x::field_bit0(M, a0, 0x30)', [], None)
add(0x005947d0, 'T', 'p', 'u32', 'x::field(M, a0, 0x1340)', [], None)
add(0x00594f40, 'T', 'pppp', 'u32', 'x::viewport_size(M, a0, a1, a2, a3)', [], None)
for a, o in [(0x00595f70, 0x38), (0x00595f80, 0x44), (0x00595f90, 0x70)]:
    add(a, 'T', 'p', 'u32', f'x::field_nonzero(M, a0, {o:#x})', [], None)
add(0x005980c0, 'T', 'pv', 'u32', 'x::set_target(M, a0, a1)', [], None)
add(0x00598a30, 'T', 'pp', 'u32', 'x::release_shader_pass(M, a1)', [], None)
add(0x00599d50, 'T', 'pffff', '', 'x::set_clear_color(M, a0, [a1, a2, a3, a4])', [], None)
add(0x00599eb0, 'T', 'pf', '', 'x::set_clear_depth(M, a0, a1)', [], None)
add(0x00599ec0, 'T', 'pv', 'u32', 'x::set_field(M, a0, 0x390, a1)', [], None)
add(0x0058e170, 'T', 'pv', 'u32', 'x::set_byte_ret0(M, a0, 0x131c, a1)', [], None)
add(0x0058ab10, 'C', 'i', 'u32', 'x::map_mode(a0)', [], None)
add(0x0058af90, 'C', 'pc', '', 'x::zero_array16(M, a0, a1)', [], None)
add(0x0058e5b0, 'T', 'pv', 'u32', 'x::forward_593930(M, a0, a1, 0x0059_3930)', [(0x00593930, 2, 2)], None)
add(0x00593db0, 'T', 'pi', 'u32', 'x::element_field(M, a0, a1, 0x0058_5c90, 8)', [(0x00585c90, 1, 1)], 'prep_element_field')
add(0x00594060, 'T', 'pi', 'u32', 'x::element_field(M, a0, a1, 0x0058_5c90, 4)', [(0x00585c90, 1, 1)], 'prep_element_field')
add(0x0059a120, 'T', 'pvv', 'u32', 'x::set_mode(M, a0, a1, a2, 0x0058_a9a0)', [(0x0058a9a0, 1, 0)], None)
add(0x0059aff0, 'S', 'pv', 'u32', 'x::forward_if_bound(M, a0, a1, 0x0059_b140)', [(0x0059b140, 2, 2)], None)
add(0x0059bbf0, 'T', 'pvvvv', 'u32', 'x::set_viewport(M, a0, [a1, a2, a3, a4], 0x0058_c790)', [(0x0058c790, 4, 4)], None)
add(0x00583420, 'T', 'p', 'u32', 'x::zero_init21(M, a0)', [(0x005031e0, 0, 0)], None)
add(0x00583b20, 'T', 'p', 'u32', 'x::ctor_then_zero(M, a0, 0x004b_e870, &[0x20])', [(0x004be870, 0, 0)], None)
for a in [0x00583b40, 0x00583bc0, 0x00583be0]:
    add(a, 'T', 'pv', 'u32', 'x::ctor_then_zero(M, a0, 0x0058_3f20, &[0x4e0])', [(0x00583f20, 0, 0)], None)
add(0x00584060, 'T', 'pv', 'u32', 'x::ctor_then_zero(M, a0, 0x0058_3f20, &[0x4e0, 0x4e4, 0x4e8, 0x4ec])',
    [(0x00583f20, 0, 0)], None)
add(0x005844a0, 'T', 'pvvvvvvvv', 'u32', 'x::ctor8(M, a0, [a1, a2, a3, a4, a5, a6, a7, a8], 0x004c_0c00)',
    [(0x004c0c00, 3, 3)], None)
