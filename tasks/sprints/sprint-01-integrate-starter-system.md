# Sprint 01 - Integrate Starter System

**Goal:** Integrate the Project Starter Template workflow system into the existing Robin/Typr repo.  
**Start:** 2026-07-01  
**End:** Open

## In Progress
- [ ] Use the new workflow docs for future implementation sessions.

## Todo
- [ ] Decide whether to add a real CI workflow after the app's build/test expectations are confirmed.
- [ ] Document the automated test strategy once it exists.

## Done
- [x] Add cross-tool assistant instructions.
- [x] Add master project index and current context snapshot.
- [x] Add sprint tracking and lessons files.
- [x] Add docs routing, architecture reference, and decision log.
- [x] Add sprint-log hook scripts and Claude Code hook configuration.

## Session Log

Append one dated entry per implementation session. Newest entries go at the bottom.

### 2026-07-01 - Integrated starter workflow system
- What changed: Added root assistant docs, project index, current context, task tracking, docs routing, decision log, architecture note, env template, Claude hook settings, and sprint-log scripts.
- Why: The Project Starter Template system needed to be copied into Robin/Typr and adapted to the existing app instead of replacing product files.
- Status: Workflow metadata is installed. No app behavior was changed; verification is limited to file presence and hook syntax checks.

### 2026-07-01 - Added "To Markdown" page (MarkItDown integration)
- What changed: New `src-tauri/src/markitdown.rs` (auto-resolves the Python interpreter that has MarkItDown, `convert_markitdown` + `save_markdown` commands); registered them and the `tauri-plugin-dialog` plugin in `main.rs`; added `tauri-plugin-dialog` to `Cargo.toml` and `dialog:default` to `capabilities/default.json`. Frontend: `To Markdown` nav item + `#section-markitdown` in `index.html`, new `src/ui/markitdown-panel.ts`, IPC wrappers in `src/ipc.ts`, wired into `main.ts`, styles in `style.css`. Pinned `@tauri-apps/api` to `~2.10.1` (+overrides) to match the Rust crate.
- Why: User wants Typr to host the AI tools they use; their standalone MarkItDown GUI (`markitdown_gui.pyw`, launched by the desktop shortcut) is now a first-class page. The engine is Microsoft MarkItDown 0.1.6 in Store Python 3.13.
- Status: Frontend builds clean. Rust build re-run after aligning the Tauri JS/Rust versions (the dialog plugin had bumped `@tauri-apps/api` to 2.11, tripping the CLI's mismatch guard). Build then surfaced two non-logic issues: an unused `CommandExt` import (tokio's `Command` has its own `creation_flags`, so the import was removed) and a locked `robin.exe` from a running tray instance (killed before rebuilding). Final compile/verification in progress.
