//! `extern "C"` shims with the exact signatures of the originals. On i686
//! Windows, `extern "C"` is cdecl and preserves EBX, ESI, EDI and EBP, as
//! the callers expect.

use std::ffi::CStr;

use ffx_core::zlib::Z_OK;

/// Replaces `zlib_uncompress` (`0x00405e20`):
/// `int uncompress(Bytef *dst, uLongf *dst_len, const Bytef *src, uLong src_len)`.
/// `*dst_len` is updated only on success, like the original.
///
/// # Safety
/// `dst_len` must be valid; `dst` must be writable for `*dst_len` bytes and
/// `src` readable for `src_len` bytes.
pub unsafe extern "C" fn zlib_uncompress(
    dst: *mut u8,
    dst_len: *mut u32,
    src: *const u8,
    src_len: u32,
) -> i32 {
    // SAFETY: guaranteed by the caller, as for the original.
    let (dst, src) = unsafe {
        let capacity = *dst_len as usize;
        (
            if capacity == 0 {
                &mut [][..]
            } else {
                std::slice::from_raw_parts_mut(dst, capacity)
            },
            if src_len == 0 {
                &[][..]
            } else {
                std::slice::from_raw_parts(src, src_len as usize)
            },
        )
    };
    match ffx_core::zlib::uncompress(dst, src) {
        Ok(written) => {
            // SAFETY: as above.
            unsafe { *dst_len = written as u32 };
            Z_OK
        }
        Err(code) => code,
    }
}

/// Replaces `Path_ToLowerAscii` (`0x00607d50`). Like the original, returns
/// the address of the terminating NUL, or `s` itself when it is NULL.
///
/// # Safety
/// `s` must be NULL or a writable NUL-terminated string.
pub unsafe extern "C" fn path_to_lower_ascii(s: *mut u8) -> *mut u8 {
    if s.is_null() {
        return s;
    }
    // SAFETY: the caller guarantees a NUL-terminated, writable string.
    unsafe {
        let len = CStr::from_ptr(s.cast()).to_bytes().len();
        ffx_core::path::to_lower_ascii(std::slice::from_raw_parts_mut(s, len));
        s.add(len)
    }
}
