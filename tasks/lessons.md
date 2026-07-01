# Lessons

AI self-correction lessons for this repo.

## Active

- Preserve the existing dirty worktree. Treat product-code changes already present in Robin/Typr as user-owned unless the user explicitly asks to replace them.
- For workflow/template imports, adapt docs and tracking to the real app instead of copying placeholder starter text verbatim.
- When adding a `@tauri-apps/plugin-*` JS package, `npm install` can bump `@tauri-apps/api` to a newer minor than the Rust `tauri` crate, and `tauri build` aborts with "Found version mismatched Tauri packages". Pin `@tauri-apps/api` (and an `overrides` entry) to the Rust crate's minor line, or bump the Rust crate to match.
- MarkItDown lives in the Store Python 3.13 (`PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0`), not the `python` on PATH (3.14). Invoke it via `py -3.13 -m markitdown` or the Store python.exe; plain `python -m markitdown` fails with ModuleNotFoundError.

## Internalized

- No internalized lessons yet.
