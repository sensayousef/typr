//! "To Markdown" conversion service.
//!
//! Wraps Microsoft's MarkItDown CLI (`python -m markitdown <file-or-url>`) so the
//! frontend can convert documents, spreadsheets, web pages, and more into
//! Markdown. MarkItDown lives in a specific Python interpreter rather than the
//! one on `PATH`, so the interpreter is auto-resolved once and cached.

use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tokio::sync::OnceCell;

/// Hard cap on a single conversion, mirroring the original GUI's 120s limit.
const CONVERT_TIMEOUT_SECS: u64 = 120;

/// `CREATE_NO_WINDOW` — keeps the spawned Python process from flashing a console
/// window, since Robin itself runs as a GUI-subsystem binary.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A resolved Python interpreter that can import `markitdown`.
#[derive(Clone)]
struct Interpreter {
    program: String,
    leading_args: Vec<String>,
}

static INTERPRETER: OnceCell<Interpreter> = OnceCell::const_new();

/// Candidate interpreters to probe, in priority order. The Windows Store Python
/// 3.13 (where MarkItDown is installed as an editable package) is reachable both
/// through the `py` launcher and via its per-user app-execution alias; plain
/// `python` is the last-resort fallback.
fn interpreter_candidates() -> Vec<Interpreter> {
    let mut candidates = vec![Interpreter {
        program: "py".to_string(),
        leading_args: vec!["-3.13".to_string()],
    }];

    if let Some(local) = dirs::data_local_dir() {
        let store_python = local
            .join("Microsoft")
            .join("WindowsApps")
            .join("PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0")
            .join("python.exe");
        candidates.push(Interpreter {
            program: store_python.to_string_lossy().into_owned(),
            leading_args: Vec::new(),
        });
    }

    candidates.push(Interpreter {
        program: "python".to_string(),
        leading_args: Vec::new(),
    });

    candidates
}

/// Configure UTF-8 I/O and (on Windows) suppress the console window.
fn base_command(interp: &Interpreter) -> Command {
    let mut cmd = Command::new(&interp.program);
    cmd.args(&interp.leading_args)
        // Force UTF-8 so Markdown with non-ASCII content survives the pipe on
        // Windows, where the default stdout encoding is the legacy code page.
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Returns true if the given interpreter can `import markitdown`.
///
/// `creation_flags` above is `tokio::process::Command`'s own Windows-only
/// inherent method, so no `CommandExt` import is needed.
async fn can_import_markitdown(interp: &Interpreter) -> bool {
    let mut cmd = base_command(interp);
    cmd.args(["-c", "import markitdown"]);
    matches!(cmd.output().await, Ok(out) if out.status.success())
}

async fn resolve_interpreter() -> Result<Interpreter, String> {
    INTERPRETER
        .get_or_try_init(|| async {
            for candidate in interpreter_candidates() {
                if can_import_markitdown(&candidate).await {
                    return Ok(candidate);
                }
            }
            Err("Could not find a Python interpreter with MarkItDown installed. \
                 Install it with: py -3.13 -m pip install markitdown[all]"
                .to_string())
        })
        .await
        .cloned()
}

/// Convert a local file path or public URL to Markdown.
#[tauri::command]
pub async fn convert_markitdown(input: String) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("No file or URL provided.".to_string());
    }

    let interp = resolve_interpreter().await?;
    let mut cmd = base_command(&interp);
    cmd.args(["-m", "markitdown"]).arg(input);

    let output = match tokio::time::timeout(
        Duration::from_secs(CONVERT_TIMEOUT_SECS),
        cmd.output(),
    )
    .await
    {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(format!("Failed to run MarkItDown: {e}")),
        Err(_) => {
            return Err(format!(
                "Conversion timed out after {CONVERT_TIMEOUT_SECS}s."
            ))
        }
    };

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        Err(if message.is_empty() {
            "MarkItDown failed with no error output.".to_string()
        } else {
            message.to_string()
        })
    }
}

/// Write converted Markdown to a user-chosen path (selected via the save dialog).
#[tauri::command]
pub fn save_markdown(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| format!("Failed to save file: {e}"))
}
