//! zlib, which FFX.exe links statically.

use miniz_oxide::inflate::stream::{InflateState, inflate};
use miniz_oxide::{DataFormat, MZError, MZFlush, MZStatus};

/// zlib return codes, as `uncompress` returns them.
pub const Z_OK: i32 = 0;
pub const Z_DATA_ERROR: i32 = -3;
pub const Z_BUF_ERROR: i32 = -5;

/// Inflates one complete zlib stream from `src` into `dst` and returns the
/// number of bytes written, or a zlib error code.
///
/// Replaces `zlib_uncompress` (`0x00405e20`), which is zlib 1.2.x's
/// `uncompress`: a single `inflate(Z_FINISH)` call. Its error mapping is
/// kept: a stream that needs a dictionary, or that runs out of input before
/// it ends, is `Z_DATA_ERROR`; a stream that does not fit in `dst` is
/// `Z_BUF_ERROR`.
///
/// # Errors
/// Returns `Z_DATA_ERROR` or `Z_BUF_ERROR` as described above.
pub fn uncompress(dst: &mut [u8], src: &[u8]) -> Result<usize, i32> {
    let mut state = InflateState::new_boxed(DataFormat::Zlib);
    let result = inflate(&mut state, src, dst, MZFlush::Finish);
    match result.status {
        Ok(MZStatus::StreamEnd) => Ok(result.bytes_written),
        Ok(MZStatus::Ok) | Err(MZError::Buf) => {
            if result.bytes_consumed == src.len() {
                Err(Z_DATA_ERROR)
            } else {
                Err(Z_BUF_ERROR)
            }
        }
        Ok(MZStatus::NeedDict) | Err(_) => Err(Z_DATA_ERROR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // zlib.compress(b"hello hello hello hello", 9)
    const HELLO: &[u8] = &[
        0x78, 0xda, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0xc8, 0x40, 0x27, 0x01, 0x68, 0x03, 0x08,
        0xb1,
    ];

    #[test]
    fn inflates_a_stream() {
        let mut out = [0u8; 64];
        let n = uncompress(&mut out, HELLO).unwrap();
        assert_eq!(&out[..n], b"hello hello hello hello");
    }

    #[test]
    fn small_output_is_a_buffer_error() {
        let mut out = [0u8; 4];
        assert_eq!(uncompress(&mut out, HELLO), Err(Z_BUF_ERROR));
    }

    #[test]
    fn truncated_input_is_a_data_error() {
        let mut out = [0u8; 64];
        assert_eq!(uncompress(&mut out, &HELLO[..8]), Err(Z_DATA_ERROR));
    }

    #[test]
    fn corrupt_input_is_a_data_error() {
        let mut bad = HELLO.to_vec();
        bad[0] = 0x79;
        let mut out = [0u8; 64];
        assert_eq!(uncompress(&mut out, &bad), Err(Z_DATA_ERROR));
    }
}
