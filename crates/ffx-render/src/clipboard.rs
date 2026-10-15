//! The Windows clipboard, as text (the overlay shares profiles through it).

/// Puts `text` on the clipboard.
#[cfg(windows)]
#[must_use]
pub fn set(text: &str) -> bool {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{
        GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock,
    };
    const CF_UNICODETEXT: u32 = 13;
    let wide: Vec<u16> = text.encode_utf16().chain([0]).collect();
    // SAFETY: plain Win32 clipboard calls; the memory goes to the clipboard
    // once SetClipboardData succeeds.
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return false;
        }
        EmptyClipboard();
        let mem = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2);
        let mut ok = false;
        if !mem.is_null() {
            let p = GlobalLock(mem).cast::<u16>();
            if !p.is_null() {
                std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
                GlobalUnlock(mem);
                ok = !SetClipboardData(CF_UNICODETEXT, mem).is_null();
            }
        }
        CloseClipboard();
        ok
    }
}

/// The clipboard's text.
#[cfg(windows)]
#[must_use]
pub fn get() -> Option<String> {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, OpenClipboard,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    const CF_UNICODETEXT: u32 = 13;
    // SAFETY: plain Win32 clipboard calls; the text is NUL-terminated.
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let mem = GetClipboardData(CF_UNICODETEXT);
        let mut out = None;
        if !mem.is_null() {
            let p = GlobalLock(mem).cast::<u16>();
            if !p.is_null() {
                let mut n = 0;
                while *p.add(n) != 0 {
                    n += 1;
                }
                out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, n)));
                GlobalUnlock(mem);
            }
        }
        CloseClipboard();
        out
    }
}

#[cfg(not(windows))]
#[must_use]
pub fn set(_text: &str) -> bool {
    false
}

#[cfg(not(windows))]
#[must_use]
pub fn get() -> Option<String> {
    None
}
