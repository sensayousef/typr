//! Runtime debug-console management (binary-only).
//!
//! Robin is built as a GUI-subsystem binary (see `windows_subsystem` in
//! `main.rs`), so by default no terminal window appears when the app launches.
//! This module lets the user summon a console at runtime to watch
//! `println!`/`eprintln!` diagnostics — and dismiss it again — toggled from the
//! settings UI. On non-Windows targets every function is a no-op.

#[cfg(windows)]
mod sys {
    use std::ffi::{c_void, OsStr};
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    type Handle = *mut c_void;

    const STD_INPUT_HANDLE: u32 = -10i32 as u32;
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;

    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const OPEN_EXISTING: u32 = 3;
    const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;

    #[link(name = "kernel32")]
    extern "system" {
        fn AllocConsole() -> i32;
        fn FreeConsole() -> i32;
        fn GetConsoleWindow() -> Handle;
        fn SetStdHandle(n_std_handle: u32, handle: Handle) -> i32;
        fn SetConsoleTitleW(title: *const u16) -> i32;
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *mut c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: Handle,
        ) -> Handle;
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Open one of the console pseudo-files (`CONOUT$` / `CONIN$`) so the new
    /// console's buffers can be wired into the process's std handles.
    fn open_console_device(name: &str) -> Handle {
        let wide_name = wide(name);
        unsafe {
            CreateFileW(
                wide_name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null_mut(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        }
    }

    pub fn is_visible() -> bool {
        unsafe { !GetConsoleWindow().is_null() }
    }

    pub fn show() {
        // A console may already be attached (e.g. launched from a terminal).
        // In that case keep the existing one rather than failing.
        if is_visible() {
            return;
        }

        unsafe {
            if AllocConsole() == 0 {
                return;
            }

            // Rust's stdout/stderr resolve their target via `GetStdHandle` on
            // every write, so pointing the std handles at the freshly created
            // console buffers makes `println!`/`eprintln!` land in the window.
            let out = open_console_device("CONOUT$");
            if out != INVALID_HANDLE_VALUE {
                SetStdHandle(STD_OUTPUT_HANDLE, out);
                SetStdHandle(STD_ERROR_HANDLE, out);
            }
            let inp = open_console_device("CONIN$");
            if inp != INVALID_HANDLE_VALUE {
                SetStdHandle(STD_INPUT_HANDLE, inp);
            }

            let title = wide("Robin — Debug Console");
            SetConsoleTitleW(title.as_ptr());
        }

        println!("[Robin] Debug console attached. Diagnostics will appear here.");
    }

    pub fn hide() {
        if !is_visible() {
            return;
        }
        unsafe {
            FreeConsole();
        }
    }
}

#[cfg(not(windows))]
mod sys {
    pub fn is_visible() -> bool {
        false
    }
    pub fn show() {}
    pub fn hide() {}
}

/// Show or hide the debug console, taking effect immediately.
pub fn set_visible(enabled: bool) {
    if enabled {
        sys::show();
    } else {
        sys::hide();
    }
}
