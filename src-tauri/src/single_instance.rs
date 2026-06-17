//! "Newest launch wins" single-instance enforcement.
//!
//! Tauri's official single-instance plugin keeps the *first* instance alive and
//! discards new launches. Typr wants the opposite: launching the app should
//! terminate any older instance — including one running invisibly in the system
//! tray — and let the new process take over (tray icon, global hotkeys, etc.).
//!
//! This runs before the Tauri app is built, so the surviving process is always
//! the one the user just started.

/// Terminate every other running instance of this executable, leaving only the
/// current process alive. Best-effort: failures are ignored so a launch is
/// never blocked by a stuck enumeration or a permission error.
#[cfg(target_os = "windows")]
pub fn kill_other_instances() {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Don't pop up a console window for the helper process.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "typr.exe".to_string());

    let current_pid = std::process::id();

    // taskkill forcibly ends every process with our image name *except* this
    // one (PID filter). This frees the global hotkey and tray registrations the
    // old instance was holding so the new instance can claim them.
    let _ = Command::new("taskkill")
        .args([
            "/F",
            "/IM",
            &exe_name,
            "/FI",
            &format!("PID ne {current_pid}"),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

/// No-op on non-Windows platforms. Tauri's single-instance plugin can be wired
/// up here later if Typr ever ships beyond Windows.
#[cfg(not(target_os = "windows"))]
pub fn kill_other_instances() {}
