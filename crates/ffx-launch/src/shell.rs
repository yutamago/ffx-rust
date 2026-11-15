//! The player's Windows shell: the documents folder `FFX.exe` uses and a
//! message box. One file for two crates: `ffx-launch` uses it, and
//! `ffx-host`'s Windows platform compiles it too (`#[path]`, as the hook's
//! `rust_audio_out.rs`).

/// The folder `FFX.exe` asks Windows for: `SHGetFolderPathW(CSIDL_PERSONAL)`.
#[must_use]
pub fn documents() -> Option<std::path::PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::UI::Shell::{CSIDL_PERSONAL, SHGetFolderPathW};
    let mut buf = [0u16; 260];
    // SAFETY: MAX_PATH wide characters, as the call requires.
    let hr = unsafe {
        SHGetFolderPathW(
            std::ptr::null_mut(),
            i32::try_from(CSIDL_PERSONAL).ok()?,
            std::ptr::null_mut(),
            0,
            buf.as_mut_ptr(),
        )
    };
    let len = buf.iter().position(|&c| c == 0)?;
    (hr == 0 && len > 0).then(|| std::ffi::OsString::from_wide(&buf[..len]).into())
}

/// A message box with `text` under `title`, an OK button and the warning
/// or the error icon; returns when the player closed it.
pub fn message_box(title: &str, text: &str, warning: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_ICONWARNING, MB_OK, MessageBoxW,
    };
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let text = wide(text);
    let title = wide(title);
    let icon = if warning {
        MB_ICONWARNING
    } else {
        MB_ICONERROR
    };
    // SAFETY: two NUL-terminated wide strings that outlive the call.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | icon,
        );
    }
}
