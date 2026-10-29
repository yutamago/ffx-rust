//! `ADVAPI32`'s `CryptoAPI` as the game uses it: MD5 of file paths
//! (`BigFileStream_ctor` acquires a `PROV_RSA_FULL` verify context,
//! `FileSys_Exists` hashes each path with `CALG_MD5`), and
//! `GetUserNameA`. Exact: the hash is MD5 (`md-5`), whatever the
//! provider.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ffx_bridge::Mem as _;
use md5::Digest as _;

use crate::exec::Exec;
use crate::imports::ImportFn;
use crate::table::{Args, Conv, Ret};

/// `CALG_MD5`.
const CALG_MD5: u32 = 0x8003;
/// `HP_HASHVAL`.
const HP_HASHVAL: u32 = 2;
/// `HP_HASHSIZE`.
const HP_HASHSIZE: u32 = 4;
/// `NTE_BAD_ALGID`.
const NTE_BAD_ALGID: u32 = 0x8009_0008;
/// `NTE_BAD_HASH`.
const NTE_BAD_HASH: u32 = 0x8009_0002;
/// `ERROR_MORE_DATA`.
const ERROR_MORE_DATA: u32 = 234;

/// Open hashes, by run (the `Shared` they belong to) and handle.
static HASHES: Mutex<Option<HashMap<(usize, u32), md5::Md5>>> = Mutex::new(None);

fn run_key(e: &Exec) -> usize {
    Arc::as_ptr(e.shared()) as usize
}

fn with_hashes<T>(f: impl FnOnce(&mut HashMap<(usize, u32), md5::Md5>) -> T) -> T {
    let mut g = HASHES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(g.get_or_insert_with(HashMap::new))
}

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

fn fail(e: &mut Exec, code: u32) -> Ret {
    e.crt.last_error = code;
    Ret::eax(0)
}

/// `CryptAcquireContextA(&prov, container, provider, type, flags)`.
fn acquire_context(e: &mut Exec, args: &Args) -> Ret {
    let h = e.new_handle();
    if a(args, 0) != 0 {
        e.mem().write(a(args, 0), h);
    }
    Ret::eax(1)
}

/// `CryptCreateHash(prov, algorithm, key, flags, &hash)`.
fn create_hash(e: &mut Exec, args: &Args) -> Ret {
    if a(args, 1) != CALG_MD5 {
        return fail(e, NTE_BAD_ALGID);
    }
    let h = e.new_handle();
    let key = run_key(e);
    with_hashes(|m| m.insert((key, h), md5::Md5::new()));
    if a(args, 4) != 0 {
        e.mem().write(a(args, 4), h);
    }
    Ret::eax(1)
}

/// `CryptHashData(hash, data, len, flags)`.
fn hash_data(e: &mut Exec, args: &Args) -> Ret {
    let mut data = vec![0u8; a(args, 2) as usize];
    e.mem().read_bytes(a(args, 1), &mut data);
    let key = run_key(e);
    let ok = with_hashes(|m| {
        m.get_mut(&(key, a(args, 0)))
            .map(|h| h.update(&data))
            .is_some()
    });
    if ok {
        Ret::eax(1)
    } else {
        fail(e, NTE_BAD_HASH)
    }
}

/// `CryptGetHashParam(hash, param, data, &len, flags)`.
fn get_hash_param(e: &mut Exec, args: &Args) -> Ret {
    let key = run_key(e);
    let (data, len_at) = (a(args, 2), a(args, 3));
    let value: Option<Vec<u8>> = match a(args, 1) {
        HP_HASHVAL => with_hashes(|m| {
            m.get(&(key, a(args, 0)))
                .map(|h| h.clone().finalize().to_vec())
        }),
        HP_HASHSIZE => Some(16u32.to_le_bytes().to_vec()),
        _ => None,
    };
    let Some(v) = value else {
        return fail(e, NTE_BAD_HASH);
    };
    let mut mem = e.mem();
    let room: u32 = if len_at == 0 { 0 } else { mem.read(len_at) };
    let n = u32::try_from(v.len()).unwrap_or(0);
    if len_at != 0 {
        mem.write(len_at, n);
    }
    if data == 0 {
        return Ret::eax(1);
    }
    if room < n {
        return fail(e, ERROR_MORE_DATA);
    }
    mem.write_bytes(data, &v);
    Ret::eax(1)
}

/// `CryptDestroyHash(hash)`.
fn destroy_hash(e: &mut Exec, args: &Args) -> Ret {
    let key = run_key(e);
    with_hashes(|m| m.remove(&(key, a(args, 0))));
    Ret::eax(1)
}

/// `GetUserNameA(buffer, &len)`: a fixed name.
fn user_name(e: &mut Exec, args: &Args) -> Ret {
    let name = b"Player\0";
    let mut mem = e.mem();
    let room: u32 = mem.read(a(args, 1));
    mem.write(a(args, 1), 7u32);
    if room < 7 {
        return fail(e, 122);
    }
    mem.write_bytes(a(args, 0), name);
    Ret::eax(1)
}

const ADV: &str = "ADVAPI32.dll";

/// The stand-ins: DLL, name, convention, stack words.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (
        ADV,
        "CryptAcquireContextA",
        Conv::Stdcall,
        5,
        acquire_context,
    ),
    (ADV, "CryptCreateHash", Conv::Stdcall, 5, create_hash),
    (ADV, "CryptHashData", Conv::Stdcall, 4, hash_data),
    (ADV, "CryptGetHashParam", Conv::Stdcall, 5, get_hash_param),
    (ADV, "CryptDestroyHash", Conv::Stdcall, 1, destroy_hash),
    (ADV, "CryptReleaseContext", Conv::Stdcall, 2, |_, _| {
        Ret::eax(1)
    }),
    (ADV, "GetUserNameA", Conv::Stdcall, 2, user_name),
];
