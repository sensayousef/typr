//! "Newest launch wins" single-instance enforcement.
//!
//! Tauri's official single-instance plugin keeps the *first* instance alive and
//! discards new launches. Robin wants the opposite: launching the app should
//! terminate any older instance — including one running invisibly in the system
//! tray — and let the new process take over (tray icon, global hotkeys, etc.).
//!
//! This runs before the Tauri app is built, so the surviving process is always
//! the one the user just started.

/// Terminate every other running Robin instance, leaving only the current
/// process alive. Also retire legacy Typr instances from before the rename so
/// they cannot keep owning global hotkeys or tray state.
#[cfg(target_os = "windows")]
pub fn kill_other_instances() {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Don't pop up a console window for the helper process.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "robin.exe".to_string());

    let current_pid = std::process::id();

    for image_name in [exe_name.as_str(), "typr.exe"] {
        // taskkill forcibly ends every process with the target image name
        // except this one. That frees the global hotkey and tray registrations
        // held by older tray-only instances so the new Robin process can claim
        // them.
        let _ = Command::new("taskkill")
            .args([
                "/F",
                "/IM",
                image_name,
                "/FI",
                &format!("PID ne {current_pid}"),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
}

/// Best-effort cleanup for legacy Typr startup entries left behind before the
/// app was renamed. Robin's own autostart setting is not touched.
#[cfg(target_os = "windows")]
pub fn disable_legacy_typr_autostart() {
    use std::fs;
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    const LEGACY_VALUES: [&str; 4] = ["Typr", "typr", "typr.exe", "com.typr.app"];

    for value_name in LEGACY_VALUES {
        let _ = Command::new("reg")
            .args(["delete", RUN_KEY, "/v", value_name, "/f"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    if let Ok(appdata) = std::env::var("APPDATA") {
        let startup_dir = PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup");
        for file_name in ["Typr.lnk", "typr.lnk", "launch-typr.bat"] {
            let _ = fs::remove_file(startup_dir.join(file_name));
        }
    }
}

/// No-op on non-Windows platforms. Tauri's single-instance plugin can be wired
/// up here later if Robin ever ships beyond Windows.
#[cfg(not(target_os = "windows"))]
pub fn kill_other_instances() {}

#[cfg(not(target_os = "windows"))]
pub fn disable_legacy_typr_autostart() {}
