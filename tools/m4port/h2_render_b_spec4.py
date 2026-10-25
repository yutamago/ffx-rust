# round 2, batch 2: small render methods (ffx_prender::small), several with
# frames their callees see (the 8th field: bytes below ebp; the 9th: the
# cookie gap)
SPEC = []


def add(*r):
    SPEC.append(r)


add(0x004fc8e0, 'T', 'pp', 'u32', 'sm::network_forward1(M, a0, a1, FRAME)', [(0x005aead0, 2, 2)], '|e, r, a| prep_network(e, r, a, 1)', 0x10)
add(0x004fc9d0, 'T', 'pvp', 'u32', 'sm::network_forward2(M, a0, a1, a2, FRAME)', [(0x005aece0, 3, 3)], '|e, r, a| prep_network(e, r, a, 2)', 0x10)
add(0x0058cf20, 'T', 'p', 'u32', 'sm::bind_if_unset(M, a0)', [(0x0058ee20, 2, 2)],
    '|e, r, a| prep_sharray_first(e, r, a, 0xc)')
add(0x0058db00, 'T', 'p', 'u32', 'sm::texture_bind(M, a0)', [(0x0058fc20, 4, 4)],
    '|e, r, a| prep_sharray_first(e, r, a, 0x24)')
add(0x00592b60, 'T', 'pvvn', 'u8', 'sm::draw(M, a0, a1, a2, a3)', [(0x0058ab60, 1, 0)], 'prep_context')
add(0x005008c0, 'T', 'pvvv', 'u32', 'sm::bounds_forward(M, a0, a1, a2, a3, FRAME)', [(0x004ffe40, 5, 0)], None, 0x20)
add(0x00593e30, 'T', 'pi', 'u32', 'sm::element_word1(M, a0, a1)', [], '|e, r, a| { prep_sharray_first(e, r, a, 0x20); a[1] = r.below(4); if r.chance(5) { e.w32(a[0] + 0x20, 0); } }')
add(0x0059aaf0, 'T', 'pvvvv', 'u32', 'sm::set_scissor(M, a0, [a1, a2, a3, a4], FRAME)', [], 'prep_context', 0x10, 4)
add(0x00598a80, 'S', 'pvv', 'u32', 'sm::unbind_all(M, a0, a1, a2)', [(0x00598b50, 2, 2)], None)
add(0x00598b50, 'T', 'pvv', 'u32', 'sm::unbind_table(M, a0, a2)', [], 'prep_owner_table')
add(0x0058ab60, 'C', 'i', 'u32', 'sm::topology(M, a0)', [(0x004352f0, 2, 0)], 'prep_once')
# the factor map runs as it is (two calls must give two values)
add(0x005998f0, 'T', 'pvww', 'u32', 'sm::set_blend_pair(M, a0, a1, a2, a3)', [], None)
add(0x0058a8f0, 'C', 'w', 'u32', 'sm::blend_factor(a0)', [], None)
add(0x005940d0, 'T', 'pvv', 'u32', 'sm::query_and_set(M, a0, a1, a2, FRAME)', [], 'prep_query_chain', 8)
add(0x00563900, 'T', 'pip', 'u32', 'sm::element_offset(M, a0, a1, a2, FRAME)', [(0x00560e90, 3, 0, 0)],
    '|e, r, a| prep_sharray_first(e, r, a, 0x14)', 4)
