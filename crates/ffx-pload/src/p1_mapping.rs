//! List `p1-load`, round 4: the namespace mapping between a cluster
//! file's types and classes and the runtime's
//! (`PSerialization::PBinary::PInternal::PNamespaceMapping`, Fahrenheit's
//! names from the Switch build).
//!
//! The mapping (`map`):
//! - `+0`: the file's namespace block `F`;
//! - `+4`: the type count (bit 31 a flag), `+8` the type entries, 12
//!   bytes each: the address of the name's offset word in `F`, the
//!   runtime type, the name;
//! - `+0xc`: the class count (bit 31 a flag), `+0x10` the class
//!   entries, 32 bytes each (below);
//! - `+0x18`: the member entries, 32 bytes each (below);
//! - `+0x1c`: the mismatch flags of all classes, or'ed.
//!
//! `F`: `+8` type count `t`, `+0xc` class count `c`, `+0x10` member
//! count `m`, `+0x14` the default-data offset; at `+0x20` the types'
//! name offsets (`t` words), then the class records (9 words), then the
//! member records (6 words), then the strings (all name offsets are
//! from there), then (`+0x14` on) the default data.
//!
//! A class record: `+0` super class (1-based class index, 0 for none),
//! `+4` size (low 28 bits), `+8` name, `+0xc` member count, `+0x1c` bit 0
//! "can be made at load" (a dynamic class), `+0x20` its default data
//! (1-based offset, 0 for none). A member record: `+0` name, `+4` type
//! (a type index, or past the types a 1-based class index), `+8` offset,
//! `+0xc` size, `+0x10` flags, `+0x14` element count.
//!
//! A class entry: `+0` the super class's entry, `+4` the record, `+8` the
//! runtime class descriptor, `+0xc` the name, `+0x10` its member
//! entries, `+0x14` its mismatch flags (with its super class's), `+0x18`
//! its default data. A member entry: `+0` its class's entry, `+4` the
//! record, `+8` the runtime member, `+0xc` the name, `+0x10` the runtime
//! type, `+0x14` the class entry of a class-typed member, `+0x18` its
//! mismatch flag.

use ffx_graphic::host::{Conv, Host};

/// `PhyrePrintf(level, format, ...)` (cdecl).
const PRINTF: u32 = 0x0043_52f0;
/// `PTypeDispenser`'s instance (cdecl `()`).
const DISPENSER: u32 = 0x0043_9040;
/// The dispenser's lookup by `{hash, name}` key (thiscall `(&key)`).
const DISPENSER_FIND: u32 = 0x0043_75c0;
/// `PNamespace::getClassDescriptor(name)` (thiscall).
const GET_CLASS: u32 = 0x0043_e950;
/// The dynamic-class allocator's `(name)` (thiscall on it).
const ALLOCATE_CLASS: u32 = 0x0047_d330;
/// `PClassDescriptor::findMember(name)` with the parents (thiscall).
const FIND_MEMBER: u32 = 0x0043_5ca0;
/// The class entry's mismatch handler `(ctx)` (thiscall on the entry).
const ON_MISMATCH: u32 = 0x0047_e180;
/// `PClassDataMemberArrayFixedSize::getFixedSize()` (thiscall).
const FIXED_SIZE: u32 = 0x0057_6050;
/// The flags a check starts from (a global, 0 in the image).
const START_FLAGS: u32 = 0x00c9_75e8;

/// The messages (`checkIfMatches` at level 1, `build` at 1 and 2).
const RESIZED: u32 = 0x00b1_34e0;
const SUPER_MISSING: u32 = 0x00b1_3530;
const SUPER_CHANGED: u32 = 0x00b1_3580;
const SUPER_NEW: u32 = 0x00b1_35d0;
const SUPER_GONE: u32 = 0x00b1_3628;
const MEMBER_GONE: u32 = 0x00b1_3670;
const MEMBER_MOVED: u32 = 0x00b1_36b8;
const MEMBER_SIZE: u32 = 0x00b1_3728;
const UNKNOWN_TYPE: u32 = 0x00b1_3780;
const TYPE_UNREGISTERED: u32 = 0x00b1_3790;
const TYPE_FROM_UNKNOWN: u32 = 0x00b1_3838;
const TYPE_CHANGED: u32 = 0x00b1_3898;
const MEMBER_FLAGS: u32 = 0x00b1_38e8;
const ARRAY_SIZE: u32 = 0x00b1_3940;
const MEMBER_NEW: u32 = 0x00b1_39a8;
const NO_TYPE: u32 = 0x00b1_39f0;
const NO_CLASS: u32 = 0x00b1_3a38;
const NO_MEMBER_TYPE: u32 = 0x00b1_3a88;

/// `allocateMissingClassDescriptors`' callees: `checkIfMatches` (by
/// address), `PhyreMalloc(size)` (cdecl), the dynamic descriptor's
/// constructor `(ns, name, base)`, `addToNamespace`, its member adder
/// `(name, type, flag, 0)`, `PhyreMalloc(size, align)` and `PhyreFree`,
/// `getDefaultBuffer()`, `memcpy`, the default-data copy `(&desc, src,
/// dst)` (thiscall on the class entry), the descriptor's refresh
/// `(buffer)` and the mapping's mismatch pass `()`.
const CHECK: u32 = 0x0047_cc80;
const NEW: u32 = 0x0042_fb40;
const DYNAMIC_CTOR: u32 = 0x0044_7690;
const ADD_TO_NAMESPACE: u32 = 0x0043_c140;
const ADD_MEMBER: u32 = 0x0044_9250;
const MALLOC: u32 = 0x0042_fb80;
const FREE: u32 = 0x0042_fb20;
const DEFAULT_BUFFER: u32 = 0x0043_cb80;
const MEMCPY: u32 = 0x0094_9256;
const COPY_DEFAULTS: u32 = 0x0047_c610;
const REFRESH: u32 = 0x0044_94a0;
const MISMATCHED: u32 = 0x0047_c080;
const NO_SUPER: u32 = 0x00b1_3ad8;
const BAD_SUPER: u32 = 0x00b1_3b20;
const ALREADY_THERE: u32 = 0x00b1_3b5c;
const NO_MEMBER_CLASS: u32 = 0x00b1_3b94;
/// A dynamic class still unmapped after the allocation, and no block.
const MISSING_CLASS: u32 = 0x13;
const NO_BLOCK: u32 = 0xd;

/// A class entry without a file record.
pub const NO_RECORD: u32 = 7;

/// The name hash the type dispenser keys on: `0x7b5`, then `33 h + (b &
/// 0x1f)` per byte up to the terminator.
fn name_hash(h: &impl Host, name: u32) -> u32 {
    let mut hash = 0x7b5u32;
    let mut p = name;
    loop {
        let b: u8 = h.read(p);
        if b == 0 {
            return hash;
        }
        hash = hash.wrapping_mul(33).wrapping_add(u32::from(b & 0x1f));
        p = p.wrapping_add(1);
    }
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        ADD_MEMBER, ADD_TO_NAMESPACE, ALREADY_THERE, BAD_SUPER, CHECK, COPY_DEFAULTS,
        DEFAULT_BUFFER, DYNAMIC_CTOR, FREE, MALLOC, MEMCPY, MISMATCHED, MISSING_CLASS, NEW,
        NO_BLOCK, NO_MEMBER_CLASS, NO_SUPER, REFRESH,
    };
    use super::{
        ALLOCATE_CLASS, ARRAY_SIZE, Conv, DISPENSER, DISPENSER_FIND, FIND_MEMBER, FIXED_SIZE,
        GET_CLASS, Host, MEMBER_FLAGS, MEMBER_GONE, MEMBER_MOVED, MEMBER_NEW, MEMBER_SIZE,
        NO_CLASS, NO_MEMBER_TYPE, NO_RECORD, NO_TYPE, ON_MISMATCH, PRINTF, RESIZED, START_FLAGS,
        SUPER_CHANGED, SUPER_GONE, SUPER_MISSING, SUPER_NEW, TYPE_CHANGED, TYPE_FROM_UNKNOWN,
        TYPE_UNREGISTERED, UNKNOWN_TYPE, name_hash,
    };

    /// `PhyrePrintf(level, f, args...)`.
    fn print(h: &mut impl Host, level: u32, f: u32, args: &[u32]) {
        let f = h.rebase(f);
        let mut words = [0u32; 6];
        words[0] = level;
        words[1] = f;
        let n = args.len().min(4);
        words[2..2 + n].copy_from_slice(&args[..n]);
        h.call_sym(PRINTF, Conv::Cdecl, &words[..2 + n]);
    }

    /// The kind word (`+0x30`) of what a data member's slot 1 returns, 0
    /// for nothing.
    fn member_kind(h: &mut impl Host, member: u32) -> u32 {
        let k = h.call_virtual(member, 1, &[]);
        if k == 0 { 0 } else { h.read(k + 0x30) }
    }

    /// The size a data member takes: 4 for a pointer (flag bit 1, or kind
    /// 2 without flag bit 6), else its type's size.
    fn member_size(h: &mut impl Host, member: u32) -> u32 {
        let ty: u32 = h.read(member + 0x18);
        let type_size: u32 = h.read(ty + 0x1c);
        let kind2 = member_kind(h, member) == 2;
        let flags: u32 = h.read(member + 0x14);
        let pointer = (kind2 && flags & 0x40 == 0) || flags & 2 != 0;
        if pointer { 4 } else { type_size }
    }

    /// `PClassDescriptorMapping::checkIfMatches(verbose)` (`0x0047cc80`,
    /// thiscall on a class entry `cm`, `ret 4`): how the runtime class
    /// differs from the file's: bit 2 the size, bit 1 a member the file
    /// lacks (counted only when `verbose`), bit 0 a member that differs
    /// or is gone (each member entry's `+0x18` gets its own bit 0), over
    /// the start flags; [`NO_RECORD`] without a record. `verbose` (its low
    /// byte) prints each difference at level 1.
    pub fn check_if_matches(h: &mut impl Host, cm: u32, verbose: u32) -> u32 {
        let verbose = verbose & 0xff != 0;
        let record: u32 = h.read(cm + 4);
        if record == 0 {
            return NO_RECORD;
        }
        let count: u32 = h.read(record + 0xc);
        let class: u32 = h.read(cm + 8);
        let name: u32 = h.read(class + 0x18);
        let size: u32 = h.read(class + 0x1c);
        let start = h.rebase(START_FLAGS);
        let mut flags: u32 = h.read(start);
        let file_size = h.read::<u32>(record + 4) & 0x0fff_ffff;
        if size != file_size {
            if verbose {
                print(h, 1, RESIZED, &[name, file_size, size]);
            }
            flags |= 4;
        }
        let now: u32 = h.read(cm + 8);
        let parent: u32 = h.read(now + 0x40);
        let sup: u32 = h.read(cm);
        if verbose {
            if parent != 0 {
                if sup != 0 {
                    let found: u32 = h.read(sup + 8);
                    let sup_name: u32 = h.read(sup + 0xc);
                    if found == 0 {
                        print(h, 1, SUPER_MISSING, &[name, sup_name]);
                    } else if found != parent {
                        let parent_name: u32 = h.read(parent + 0x18);
                        print(h, 1, SUPER_CHANGED, &[name, sup_name, parent_name]);
                    }
                } else {
                    let parent_name: u32 = h.read(parent + 0x18);
                    print(h, 1, SUPER_NEW, &[name, parent_name]);
                }
            } else if sup != 0 {
                let sup_name: u32 = h.read(sup + 0xc);
                print(h, 1, SUPER_GONE, &[name, sup_name]);
            }
        }
        let mut e: u32 = h.read(cm + 0x10);
        for _ in 0..count {
            let member: u32 = h.read(e + 8);
            let bad = if member == 0 {
                if verbose {
                    let member_name: u32 = h.read(e + 0xc);
                    print(h, 1, MEMBER_GONE, &[name, member_name]);
                }
                1
            } else {
                check_member(h, e, member, name, verbose)
            };
            let old: u32 = h.read(e + 0x18);
            h.write(e + 0x18, old | bad);
            flags |= bad;
            e = e.wrapping_add(0x20);
        }
        // the runtime's members the file lacks
        let head = class.wrapping_add(0x44);
        let mut node: u32 = h.read(head);
        while node != head && node != 0 && node != 4 {
            let member = node - 4;
            let entries: u32 = h.read(cm + 0x10);
            let record: u32 = h.read(cm + 4);
            let n: u32 = h.read(record + 0xc);
            let end = entries.wrapping_add(n << 5);
            let mut at = entries;
            let mut found = false;
            while at < end {
                if h.read::<u32>(at + 8) == member {
                    found = true;
                    break;
                }
                at = at.wrapping_add(0x20);
            }
            if !found && verbose {
                let member_name: u32 = h.read(member + 0x10);
                print(h, 1, MEMBER_NEW, &[name, member_name]);
                flags |= 2;
            }
            node = h.read(member + 4);
        }
        flags
    }

    /// One member entry `e` with its runtime member: 1 when it moved,
    /// changed size, type or flags, or (a fixed-size array) its element
    /// count.
    fn check_member(h: &mut impl Host, e: u32, member: u32, name: u32, verbose: bool) -> u32 {
        let member_flags: u32 = h.read(member + 0x14);
        let fixed = member_kind(h, member) == 1;
        let offset: u32 = h.read(member + 0x24);
        let record: u32 = h.read(e + 4);
        let member_name: u32 = h.read(member + 0x10);
        let member_flags = member_flags & !1;
        let mut bad = 0;
        let file_offset: u32 = h.read(record + 8);
        if offset != file_offset {
            if verbose {
                print(
                    h,
                    1,
                    MEMBER_MOVED,
                    &[name, member_name, file_offset, offset],
                );
            }
            bad = 1;
        }
        let size = member_size(h, member);
        let file_size: u32 = h.read(record + 0xc);
        if size != file_size {
            if verbose {
                let size = member_size(h, member);
                let file_size: u32 = h.read(record + 0xc);
                print(h, 1, MEMBER_SIZE, &[name, member_name, file_size, size]);
            }
            bad = 1;
        }
        let ty: u32 = h.read(member + 0x18);
        let file_ty: u32 = h.read(e + 0x10);
        if ty != file_ty {
            let ty_name: u32 = h.read(ty + 0x18);
            let file_ty_name: u32 = if file_ty == 0 {
                0
            } else {
                h.read(file_ty + 0x18)
            };
            if verbose {
                if file_ty_name != 0 {
                    print(
                        h,
                        1,
                        TYPE_CHANGED,
                        &[name, member_name, file_ty_name, ty_name],
                    );
                } else {
                    let class: u32 = h.read(e + 0x14);
                    let class_name = if class == 0 {
                        h.rebase(UNKNOWN_TYPE)
                    } else {
                        h.read(class + 0xc)
                    };
                    if class_name == 0 {
                        print(h, 1, TYPE_FROM_UNKNOWN, &[name, member_name, ty_name]);
                    } else {
                        print(
                            h,
                            1,
                            TYPE_UNREGISTERED,
                            &[name, member_name, class_name, ty_name],
                        );
                    }
                }
            }
            bad = 1;
        }
        let file_flags: u32 = h.read(record + 0x10);
        if member_flags != file_flags {
            if verbose {
                print(
                    h,
                    1,
                    MEMBER_FLAGS,
                    &[name, member_name, file_flags, member_flags],
                );
            }
            bad = 1;
        }
        if fixed {
            let file_count: u32 = h.read(record + 0x14);
            let count = h.call_sym(FIXED_SIZE, Conv::Thiscall(member), &[]);
            if count != file_count {
                if verbose {
                    let file_count: u32 = h.read(record + 0x14);
                    let count = h.call_sym(FIXED_SIZE, Conv::Thiscall(member), &[]);
                    print(h, 1, ARRAY_SIZE, &[name, member_name, file_count, count]);
                }
                bad = 1;
            }
        }
        bad
    }

    /// The runtime type of the name at `name` through the type dispenser,
    /// its `{hash, name, dispenser}` key at `key` (the original's frame
    /// words the lookup gets).
    fn find_type(h: &mut impl Host, key: u32, name: u32) -> u32 {
        let dispenser = h.call_sym(DISPENSER, Conv::Cdecl, &[]);
        h.write(key + 8, dispenser);
        h.write(key + 4, name);
        let hash = name_hash(h, name);
        h.write(key, hash);
        h.call_sym(DISPENSER_FIND, Conv::Thiscall(dispenser), &[key])
    }

    /// `PNamespaceMapping::build(ns, allocator, ctx, verbose)`
    /// (`0x0047c8b0`, thiscall on the mapping, `ret 0x10`): maps the
    /// file's types (through the type dispenser), its classes (the type
    /// dispenser, then `ns`, keeping only classes of `ns`; a missing
    /// dynamic class made by `allocator` when there is one), and their
    /// members (`findMember` on the runtime class), then checks each
    /// mapped class (`checkIfMatches`; a mismatch goes to `+0x1c`, and
    /// to the entry's handler with `ctx`). Messages when `verbose` (low
    /// byte). `key` is the original's frame at `ebp - 0x28` (3 words)
    /// whose address the dispenser's lookup gets.
    ///
    /// `eax`: the mapping when a class was checked, else what the
    /// original leaves (the class records' address without classes, the
    /// low word of its record-count division with them).
    pub fn build(h: &mut impl Host, map: u32, args: [u32; 4], key: u32) -> u32 {
        let [ns, allocator, ctx, verbose_word] = args;
        let verbose = verbose_word & 0xff != 0;
        let file: u32 = h.read(map);
        let types: u32 = h.read(file + 8);
        let classes: u32 = h.read(file + 0xc);
        let members: u32 = h.read(file + 0x10);
        let strings = file.wrapping_add(
            classes
                .wrapping_mul(9)
                .wrapping_add(members.wrapping_mul(6))
                .wrapping_add(8)
                .wrapping_add(types)
                .wrapping_mul(4),
        );
        map_types(h, map, strings, key, verbose);
        let (first, end) = map_classes(h, map, (strings, key), (ns, allocator), verbose);
        map_members(h, map, strings, (first, end), verbose);
        check_classes(h, map, (first, end), ctx, verbose_word)
    }

    /// `build`'s type entries.
    fn map_types(h: &mut impl Host, map: u32, strings: u32, key: u32, verbose: bool) {
        let file: u32 = h.read(map);
        let n = h.read::<u32>(map + 4) & 0x7fff_ffff;
        let mut out: u32 = h.read(map + 8);
        let mut offset_at = file.wrapping_add(0x20);
        for _ in 0..n {
            let name = h.read::<u32>(offset_at).wrapping_add(strings);
            let ty = if name == 0 {
                0
            } else {
                find_type(h, key, name)
            };
            if ty == 0 && verbose {
                print(h, 2, NO_TYPE, &[name]);
            }
            h.write(out, offset_at);
            h.write(out + 8, name);
            h.write(out + 4, ty);
            offset_at = offset_at.wrapping_add(4);
            out = out.wrapping_add(12);
        }
    }

    /// `build`'s class entries: the class records' `[first, end)`.
    fn map_classes(
        h: &mut impl Host,
        map: u32,
        (strings, key): (u32, u32),
        (ns, allocator): (u32, u32),
        verbose: bool,
    ) -> (u32, u32) {
        let file: u32 = h.read(map);
        let first = file.wrapping_add(h.read::<u32>(file + 8).wrapping_add(8).wrapping_mul(4));
        let class_count = h.read::<u32>(map + 0xc) & 0x7fff_ffff;
        let end = first.wrapping_add(class_count.wrapping_mul(36));
        let mut out: u32 = h.read(map + 0x10);
        let types: u32 = h.read(file + 8);
        let classes: u32 = h.read(file + 0xc);
        let members: u32 = h.read(file + 0x10);
        let data_offset: u32 = h.read(file + 0x14);
        let defaults = data_offset
            .wrapping_add(
                classes
                    .wrapping_mul(9)
                    .wrapping_add(members.wrapping_mul(6))
                    .wrapping_add(types)
                    .wrapping_mul(4),
            )
            .wrapping_add(0x20)
            .wrapping_add(file);
        let mut record = first;
        while record < end {
            let name = h.read::<u32>(record + 8).wrapping_add(strings);
            let mut class = 0;
            if name != 0 {
                let ty = find_type(h, key, name);
                if ty != 0 {
                    let c = h.call_virtual(ty, 1, &[]);
                    if c != 0 && h.read::<u32>(c + 0x3c) == ns {
                        class = c;
                    }
                }
            }
            if class == 0 {
                let c = h.call_sym(GET_CLASS, Conv::Thiscall(ns), &[name]);
                if c != 0 && h.read::<u32>(c + 0x3c) == ns {
                    class = c;
                }
            }
            let dynamic = h.read::<u8>(record + 0x1c) & 1 != 0;
            if class == 0 {
                if !dynamic {
                    if verbose {
                        print(h, 1, NO_CLASS, &[name]);
                    }
                } else if allocator != 0 {
                    class = h.call_sym(ALLOCATE_CLASS, Conv::Thiscall(allocator), &[name]);
                }
            }
            let data: u32 = h.read(record + 0x20);
            let data = if data == 0 {
                0
            } else {
                defaults.wrapping_sub(1).wrapping_add(data)
            };
            let sup: u32 = h.read(record);
            let sup = if sup as i32 > 0 {
                let entries: u32 = h.read(map + 0x10);
                entries.wrapping_sub(0x20).wrapping_add(sup << 5)
            } else {
                0
            };
            h.write(out + 4, record);
            h.write(out, sup);
            h.write(out + 8, class);
            h.write(out + 0xc, name);
            h.write(out + 0x18, data);
            out = out.wrapping_add(0x20);
            record = record.wrapping_add(36);
        }
        (first, end)
    }

    /// `build`'s member entries.
    fn map_members(
        h: &mut impl Host,
        map: u32,
        strings: u32,
        (first, end): (u32, u32),
        verbose: bool,
    ) {
        let file: u32 = h.read(map);
        let classes: u32 = h.read(file + 0xc);
        let types: u32 = h.read(file + 8);
        let mut cm: u32 = h.read(map + 0x10);
        let mut mm: u32 = h.read(map + 0x18);
        let mut member_record = file
            .wrapping_add(0x20)
            .wrapping_add(classes.wrapping_mul(9).wrapping_add(types).wrapping_mul(4));
        let mut record = first;
        while record < end {
            let class: u32 = h.read(cm + 8);
            h.write(cm + 0x10, mm);
            let n: u32 = h.read(record + 0xc);
            for _ in 0..n {
                let name = h.read::<u32>(member_record).wrapping_add(strings);
                let ti: u32 = h.read(member_record + 4);
                let member = if class == 0 {
                    0
                } else {
                    h.call_sym(FIND_MEMBER, Conv::Thiscall(class), &[name])
                };
                let type_count = h.read::<u32>(map + 4) & 0x7fff_ffff;
                let (entry, class_entry, quiet) = if ti < type_count {
                    let entries: u32 = h.read(map + 8);
                    (entries.wrapping_add(ti.wrapping_mul(12)), 0, false)
                } else {
                    let entries: u32 = h.read(map + 0x10);
                    let c = ti
                        .wrapping_sub(type_count)
                        .wrapping_sub(1)
                        .wrapping_shl(5)
                        .wrapping_add(entries);
                    let quiet = c != 0 && {
                        let r: u32 = h.read(c + 4);
                        h.read::<u8>(r + 0x1c) & 1 != 0
                    };
                    (0, c, quiet)
                };
                h.write(mm, cm);
                h.write(mm + 4, member_record);
                h.write(mm + 0xc, name);
                h.write(mm + 0x14, class_entry);
                let ty = if class_entry != 0 {
                    h.read(class_entry + 8)
                } else if entry != 0 {
                    h.read(entry + 4)
                } else {
                    0
                };
                h.write(mm + 0x10, ty);
                if ty == 0 && verbose && !quiet {
                    let class_name: u32 = h.read(cm + 0xc);
                    print(h, 2, NO_MEMBER_TYPE, &[class_name, name]);
                }
                h.write(mm + 8, member);
                member_record = member_record.wrapping_add(0x18);
                mm = mm.wrapping_add(0x20);
            }
            record = record.wrapping_add(36);
            cm = cm.wrapping_add(0x20);
        }
    }

    /// `build`'s checks of the mapped classes; its `eax`.
    fn check_classes(
        h: &mut impl Host,
        map: u32,
        (first, end): (u32, u32),
        ctx: u32,
        verbose_word: u32,
    ) -> u32 {
        if first >= end {
            return first;
        }
        let mut eax = end
            .wrapping_sub(first)
            .wrapping_sub(1)
            .wrapping_mul(0x38e3_8e39);
        let count = (end - first - 1) / 36 + 1;
        let mut cm: u32 = h.read(map + 0x10);
        for _ in 0..count {
            if h.read::<u32>(cm + 8) != 0 {
                let sup: u32 = h.read(cm);
                let r = check_if_matches(h, cm, verbose_word);
                let inherited = if sup == 0 { 0 } else { h.read(sup + 0x14) };
                h.write(cm + 0x14, inherited | r);
                if r != 0 {
                    h.call_sym(ON_MISMATCH, Conv::Thiscall(cm), &[ctx]);
                }
                let old: u32 = h.read(map + 0x1c);
                h.write(map + 0x1c, old | r);
                eax = map;
            }
            cm = cm.wrapping_add(0x20);
        }
        eax
    }

    /// The bytes of `allocateMissingClassDescriptors`' frame the port
    /// keeps at the original's offsets (`ebp - 0x20` to `ebp - 0x10`):
    /// the default-data copy's description `{1, file size, size, byte
    /// 1}` whose address `0x0047c610` gets.
    pub const ALLOCATE_FRAME: u32 = 0x10;

    /// `PNamespaceMapping::allocateMissingClassDescriptors(ns, ctx,
    /// verbose)` (`0x0047c190`, thiscall on the mapping, `ret 0xc`): makes
    /// a dynamic class descriptor (`PClassDescriptorDynamic`, `0x00447690`
    /// on a 0xb8-byte `PhyreMalloc` block, then `addToNamespace`) for each
    /// unmapped dynamic class whose super class is mapped to a descriptor
    /// without flags 1 and 3 and whose name `ns` doesn't know yet, pass
    /// after pass while the last attempt of a pass succeeds. When any was
    /// made: 0x13 if a dynamic class is still unmapped; else each
    /// dynamic class's members are added (`0x00449250`), the classes
    /// checked (`checkIfMatches`, as `build` does), and their default
    /// buffers filled from the file's default data (`0x0047c610`,
    /// `0x004494a0`; 0xd without a block). Then, with mismatches,
    /// `0x0047c080` on the mapping. 0 or the error. `frame` is
    /// [`ALLOCATE_FRAME`] bytes at the original's `ebp - 0x20`.
    pub fn allocate_missing(h: &mut impl Host, map: u32, args: [u32; 3], frame: u32) -> u32 {
        let [ns, ctx, verbose] = args;
        let n = h.read::<u32>(map + 0xc) & 0x7fff_ffff;
        let mut any = false;
        // passes while the last attempt of a pass succeeds
        let mut made = n != 0;
        while made {
            made = false;
            for i in 0..n {
                if let Some(ok) = make_one(h, map, ns, i) {
                    made = ok;
                    any |= ok;
                }
            }
        }
        if !any {
            return finish(h, map);
        }
        let classes: u32 = h.read(map + 0x10);
        for i in 0..n {
            let e = classes.wrapping_add(i << 5);
            let record: u32 = h.read(e + 4);
            if h.read::<u32>(e + 8) == 0 && h.read::<u8>(record + 0x1c) & 1 != 0 {
                return MISSING_CLASS;
            }
        }
        for i in 0..n {
            let classes: u32 = h.read(map + 0x10);
            add_members(h, classes.wrapping_add(i << 5));
        }
        for i in 0..n {
            let classes: u32 = h.read(map + 0x10);
            let e = classes.wrapping_add(i << 5);
            let record: u32 = h.read(e + 4);
            if h.read::<u8>(record + 0x1c) & 1 == 0 {
                continue;
            }
            let sup: u32 = h.read(e);
            let r = h.call_sym(CHECK, Conv::Thiscall(e), &[verbose]);
            let inherited = if sup == 0 { 0 } else { h.read(sup + 0x14) };
            h.write(e + 0x14, inherited | r);
            if r != 0 {
                h.call_sym(ON_MISMATCH, Conv::Thiscall(e), &[ctx]);
            }
            let old: u32 = h.read(map + 0x1c);
            h.write(map + 0x1c, old | r);
        }
        for i in 0..n {
            let classes: u32 = h.read(map + 0x10);
            if !fill_defaults(h, classes.wrapping_add(i << 5), frame) {
                return NO_BLOCK;
            }
        }
        finish(h, map)
    }

    /// The end: `0x0047c080` on a mapping with mismatches; 0.
    fn finish(h: &mut impl Host, map: u32) -> u32 {
        if h.read::<u32>(map + 0x1c) != 0 {
            h.call_sym(MISMATCHED, Conv::Thiscall(map), &[]);
        }
        0
    }

    /// One pass's class `i`: `None` when nothing was tried, else whether
    /// its descriptor was made.
    fn make_one(h: &mut impl Host, map: u32, ns: u32, i: u32) -> Option<bool> {
        let classes: u32 = h.read(map + 0x10);
        let e = classes.wrapping_add(i << 5);
        if h.read::<u32>(e + 8) != 0 {
            return None;
        }
        let record: u32 = h.read(e + 4);
        if h.read::<u8>(record + 0x1c) & 1 == 0 {
            return None;
        }
        let sup: u32 = h.read(record);
        let name: u32 = h.read(e + 0xc);
        let count = h.read::<u32>(map + 0xc) & 0x7fff_ffff;
        let si = sup.wrapping_sub(1);
        let se = classes.wrapping_add(si << 5);
        if si >= count || se == 0 {
            print(h, 1, NO_SUPER, &[si, count, name]);
            return None;
        }
        let base: u32 = h.read(se + 8);
        if base == 0 {
            return None;
        }
        if h.read::<u8>(base + 0x90) & 0xa != 0 {
            let super_name: u32 = h.read(se + 0xc);
            print(h, 1, BAD_SUPER, &[super_name, name]);
            return None;
        }
        if h.call_sym(GET_CLASS, Conv::Thiscall(ns), &[name]) != 0 {
            print(h, 1, ALREADY_THERE, &[name]);
            return None;
        }
        let block = h.call_sym(NEW, Conv::Cdecl, &[0xb8]);
        let desc = if block == 0 {
            0
        } else {
            h.call_sym(DYNAMIC_CTOR, Conv::Thiscall(block), &[ns, name, base])
        };
        if desc != 0 {
            h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(desc), &[]);
        }
        h.write(e + 8, desc);
        h.write(e, se);
        let start = h.rebase(START_FLAGS);
        let flags: u32 = h.read(start);
        h.write(e + 0x14, flags);
        Some(h.read::<u32>(e + 8) != 0)
    }

    /// A made dynamic class's members (`0x00449250(name, type, flag bit
    /// 1, 0)` for each one not mapped yet).
    fn add_members(h: &mut impl Host, e: u32) {
        let desc: u32 = h.read(e + 8);
        if desc == 0 {
            return;
        }
        let record: u32 = h.read(e + 4);
        if h.read::<u8>(record + 0x1c) & 1 == 0 {
            return;
        }
        let count: u32 = h.read(record + 0xc);
        for j in 0..count {
            let members: u32 = h.read(e + 0x10);
            let me = members.wrapping_add(j << 5);
            if h.read::<u32>(me + 8) != 0 {
                continue;
            }
            let file: u32 = h.read(me + 4);
            let class_entry: u32 = h.read(me + 0x14);
            let flag = h.read::<u32>(file + 0x10) & 2;
            let mut ty: u32 = h.read(me + 0x10);
            let name: u32 = h.read(me + 0xc);
            if ty == 0 && class_entry != 0 {
                ty = h.read(class_entry + 8);
                h.write(me + 0x10, ty);
            }
            if ty == 0 {
                let class_name: u32 = h.read(desc + 0x18);
                print(h, 1, NO_MEMBER_CLASS, &[class_name, name]);
            } else {
                let member = h.call_sym(ADD_MEMBER, Conv::Thiscall(desc), &[name, ty, flag, 0]);
                h.write(me + 8, member);
            }
            h.write(me + 0x14, class_entry);
        }
    }

    /// A made dynamic class's default buffer: false without a block.
    fn fill_defaults(h: &mut impl Host, e: u32, frame: u32) -> bool {
        let desc: u32 = h.read(e + 8);
        if desc == 0 {
            return true;
        }
        let record: u32 = h.read(e + 4);
        if h.read::<u8>(record + 0x1c) & 1 == 0 {
            return true;
        }
        let size: u32 = h.read(desc + 0x1c);
        if size == 0 {
            return true;
        }
        let data: u32 = h.read(e + 0x18);
        let block = h.call_sym(MALLOC, Conv::Cdecl, &[size, 1]);
        let owned = if block == 0 { 0 } else { size };
        if block == 0 {
            return false;
        }
        let record: u32 = h.read(e + 4);
        h.write(frame, 1u32);
        let file = h.read::<u32>(record + 4) & 0x0fff_ffff;
        h.write(frame + 8, size);
        h.write(frame + 4, file);
        h.write(frame + 0xc, 1u8);
        let defaults = h.call_sym(DEFAULT_BUFFER, Conv::Thiscall(desc), &[]);
        h.call_sym(MEMCPY, Conv::Cdecl, &[block, defaults, size]);
        let dst = block
            .wrapping_sub(h.read(desc + 0x88))
            .wrapping_add(h.read(desc + 0x8c));
        let src = h
            .read::<u32>(record + 0x18)
            .wrapping_sub(h.read(record + 0x14))
            .wrapping_add(data);
        h.call_sym(COPY_DEFAULTS, Conv::Thiscall(e), &[frame, src, dst]);
        h.call_sym(REFRESH, Conv::Thiscall(desc), &[block]);
        if owned >> 31 == 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        true
    }
}
