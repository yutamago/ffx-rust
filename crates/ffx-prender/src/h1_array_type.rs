//! Three more `GetType<PArray<T>>` getters (M4 port round 3,
//! h1-render-a). They build the same four pieces as
//! [`crate::registry::bridge::array_type`]'s copies (the descriptor, a data
//! member, its annotation, the array member), but one passes the array
//! member an element argument other than its size, and two declare the
//! array member first and give it a second callback and flag 0x40.

use crate::class_slots::{ADD_TO_NAMESPACE, REGISTER_TYPE};
use crate::registry::{
    ADD_ANNOTATION, ANNOTATION_CTOR, ATEXIT, DATA_MEMBER_ARRAY_CTOR, DATA_MEMBER_CTOR, TYPE_U32,
};

/// The array member's element type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element {
    /// This descriptor.
    Const(u32),
    /// What this getter (cdecl, no arguments) returns.
    Call(u32),
}

/// One getter's constants (image addresses).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayKind {
    /// The stage bits.
    pub guard: u32,
    /// The descriptor, returned.
    pub obj: u32,
    pub desc_ctor: u32,
    pub vtable: u32,
    pub atexit_desc: u32,
    /// The class's name global.
    pub name_global: u32,
    /// The data member (`GetType<unsigned int>()`).
    pub member: u32,
    pub member_name: u32,
    pub atexit_member: u32,
    pub annotation_arg: u32,
    pub annotation: u32,
    pub annotation_vtable: u32,
    pub atexit_annotation: u32,
    /// The array member.
    pub array: u32,
    pub array_name: u32,
    pub element: Element,
    /// The array member constructor's last four words.
    pub array_words: [u32; 4],
    pub array_vtable: u32,
    /// `+0x34`.
    pub count_fn: u32,
    /// `+0x3c`.
    pub set_fn: u32,
    pub atexit_array: u32,
    /// `+0x2c`, written after the array member's stage.
    pub size: u32,
    /// The array member's stage comes first, and after it `+0x14` gets
    /// flag 0x40 and `+0x38` this function; else `None` (last, `+0x38`
    /// stays 0).
    pub array_first: Option<u32>,
}

/// The getters, by address.
pub const ARRAY_KINDS: &[(u32, ArrayKind)] = &[
    (
        0x0049_af10,
        ArrayKind {
            guard: 0x00c9_addc,
            obj: 0x00c9_ad48,
            desc_ctor: 0x0049_9b00,
            vtable: 0x00b1_6f6c,
            atexit_desc: 0x00af_cd40,
            name_global: 0x00c9_aecc,
            member: 0x00c9_ade0,
            member_name: 0x00b0_fd34,
            atexit_member: 0x00af_cd60,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_ae0c,
            annotation_vtable: 0x00b0_fd40,
            atexit_annotation: 0x00af_cd30,
            array: 0x00c9_ae28,
            array_name: 0x00b0_fd78,
            element: Element::Call(0x0043_7d30),
            array_words: [4, 0, 2, 1],
            array_vtable: 0x00b1_6fb0,
            count_fn: 0x0043_20c0,
            set_fn: 0x0049_c100,
            atexit_array: 0x00af_cd70,
            size: 4,
            array_first: None,
        },
    ),
    (
        0x0048_acf0,
        ArrayKind {
            guard: 0x00c9_9ac4,
            obj: 0x00c9_9a30,
            desc_ctor: 0x0048_a310,
            vtable: 0x00b1_5624,
            atexit_desc: 0x00af_c840,
            name_global: 0x00c9_9b98,
            member: 0x00c9_9b08,
            member_name: 0x00b0_fd34,
            atexit_member: 0x00af_c8e0,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_9b34,
            annotation_vtable: 0x00b0_fd40,
            atexit_annotation: 0x00af_c830,
            array: 0x00c9_9ac8,
            array_name: 0x00b0_edd0,
            element: Element::Const(0x00ca_5268),
            array_words: [4, 2, 2, 4],
            array_vtable: 0x00b1_5668,
            count_fn: 0x0043_20c0,
            set_fn: 0x0048_c6e0,
            atexit_array: 0x00af_c900,
            size: 4,
            array_first: Some(0x0048_c260),
        },
    ),
    (
        0x004c_f8c0,
        ArrayKind {
            guard: 0x00ca_4d2c,
            obj: 0x00ca_4c98,
            desc_ctor: 0x004c_f1d0,
            vtable: 0x00b1_f57c,
            atexit_desc: 0x00af_e6c0,
            name_global: 0x00ca_4de8,
            member: 0x00ca_4d70,
            member_name: 0x00b0_fd34,
            atexit_member: 0x00af_e6e0,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00ca_4d9c,
            annotation_vtable: 0x00b0_fd40,
            atexit_annotation: 0x00af_e6b0,
            array: 0x00ca_4d30,
            array_name: 0x00b0_edd0,
            element: Element::Call(TYPE_U32),
            array_words: [4, 0, 2, 4],
            array_vtable: 0x00b1_f5c0,
            count_fn: 0x0043_20c0,
            set_fn: 0x004d_0ba0,
            atexit_array: 0x00af_e700,
            size: 4,
            array_first: Some(0x004d_0820),
        },
    ),
];

/// The constants of getter `f` (compile time).
#[must_use]
pub const fn kind_of(f: u32) -> ArrayKind {
    let mut i = 0;
    while i < ARRAY_KINDS.len() {
        if ARRAY_KINDS[i].0 == f {
            return ARRAY_KINDS[i].1;
        }
        i += 1;
    }
    panic!("not an array-type getter of h1_array_type")
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_ANNOTATION, ADD_TO_NAMESPACE, ANNOTATION_CTOR, ATEXIT, ArrayKind,
        DATA_MEMBER_ARRAY_CTOR, DATA_MEMBER_CTOR, Element, REGISTER_TYPE, TYPE_U32,
    };
    use ffx_graphic::host::{Conv, Host};

    /// Registers `f` (an image address) with `atexit`.
    fn at_exit(h: &mut impl Host, f: u32) {
        let f = h.rebase(f);
        h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
    }

    /// The array member's stage (when its bit is clear) and the writes
    /// after it.
    fn array_stage(h: &mut impl Host, k: &ArrayKind, guard: u32, flags: u32, bit: u32) -> u32 {
        let obj = h.rebase(k.obj);
        let array = h.rebase(k.array);
        let mut flags = flags;
        if flags & bit == 0 {
            h.write(guard, flags | bit);
            let element = match k.element {
                Element::Const(d) => h.rebase(d),
                Element::Call(f) => h.call_sym(f, Conv::Cdecl, &[]),
            };
            let [a, b, c, d] = k.array_words;
            let name = h.rebase(k.array_name);
            h.call_sym(
                DATA_MEMBER_ARRAY_CTOR,
                Conv::Thiscall(array),
                &[obj, element, name, a, b, c, d],
            );
            let vtable = h.rebase(k.array_vtable);
            h.write(array, vtable);
            let count = h.rebase(k.count_fn);
            h.write(array + 0x34, count);
            h.write(array + 0x38, 0u32);
            let set = h.rebase(k.set_fn);
            h.write(array + 0x3c, set);
            at_exit(h, k.atexit_array);
            flags = h.read(guard);
        }
        if let Some(second) = k.array_first {
            let f: u32 = h.read(array + 0x14);
            h.write(array + 0x14, f | 0x40);
            let second = h.rebase(second);
            h.write(array + 0x38, second);
        }
        h.write(array + 0x2c, k.size);
        flags
    }

    /// `GetType<PArray<T>>(name)` (cdecl) -> the descriptor `k.obj`, built
    /// in stages, each once (bits of `k.guard`): bit 0 the descriptor (its
    /// constructor, `+0x90` bit 1 cleared, its vtable); then, given a
    /// `name` while the descriptor has none (`+0x18`): the name (the
    /// class's name global first, set to `name` when empty) and the next
    /// three bits, in order:
    /// - with `k.array_first`: the array member, then the data member,
    ///   then the annotation;
    /// - else the data member, the annotation, then the array member.
    ///
    /// The annotation is added to the data member while the annotation's
    /// list is empty (`+4` pointing at itself). Last `RegisterType` and
    /// `addToNamespace`.
    pub fn array_type(h: &mut impl Host, k: &ArrayKind, name: u32) -> u32 {
        let guard = h.rebase(k.guard);
        let obj = h.rebase(k.obj);
        let mut flags: u32 = h.read(guard);
        if flags & 1 == 0 {
            h.write(guard, flags | 1);
            h.call_sym(k.desc_ctor, Conv::Thiscall(obj), &[]);
            let f: u32 = h.read(obj + 0x90);
            h.write(obj + 0x90, f & !2);
            let vtable = h.rebase(k.vtable);
            h.write(obj, vtable);
            at_exit(h, k.atexit_desc);
            flags = h.read(guard);
        }
        if name == 0 || h.read::<u32>(obj + 0x18) != 0 {
            return obj;
        }
        let name_global = h.rebase(k.name_global);
        let mut class_name: u32 = h.read(name_global);
        if class_name == 0 {
            class_name = name;
            h.write(name_global, name);
        }
        h.write(obj + 0x18, class_name);
        let (member_bit, annotation_bit) = if k.array_first.is_some() {
            flags = array_stage(h, k, guard, flags, 2);
            (4, 8)
        } else {
            (2, 4)
        };
        let member = h.rebase(k.member);
        if flags & member_bit == 0 {
            h.write(guard, flags | member_bit);
            let u32_type = h.call_sym(TYPE_U32, Conv::Cdecl, &[]);
            let member_name = h.rebase(k.member_name);
            h.call_sym(
                DATA_MEMBER_CTOR,
                Conv::Thiscall(member),
                &[obj, u32_type, member_name, 0, 0, 0],
            );
            at_exit(h, k.atexit_member);
            flags = h.read(guard);
        }
        let annotation = h.rebase(k.annotation);
        if flags & annotation_bit == 0 {
            h.write(guard, flags | annotation_bit);
            let u32_type = h.call_sym(TYPE_U32, Conv::Cdecl, &[]);
            let arg = h.rebase(k.annotation_arg);
            h.call_sym(
                ANNOTATION_CTOR,
                Conv::Thiscall(annotation),
                &[arg, u32_type, 0],
            );
            let vtable = h.rebase(k.annotation_vtable);
            h.write(annotation, vtable);
            h.write(annotation + 0x18, 0x7fff_ffffu32);
            at_exit(h, k.atexit_annotation);
            flags = h.read(guard);
        }
        if h.read::<u32>(annotation + 4) == annotation + 4 {
            h.call_sym(ADD_ANNOTATION, Conv::Thiscall(member + 0x1c), &[annotation]);
            flags = h.read(guard);
        }
        if k.array_first.is_none() {
            array_stage(h, k, guard, flags, 8);
        }
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[obj]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(obj), &[]);
        obj
    }
}
