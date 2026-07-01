# Project Index

Canonical map of the important files in this repo. Look here first instead of guessing or searching.

Update rule: any time a file is added, moved, renamed, or changes scope, update this index in the same task.

## Quick "Where do I look for..."

| If you need... | Go to |
|---|---|
| Project rules / workflow | `CLAUDE.md` |
| Cross-tool assistant rules / Read Aloud | `AGENTS.md` |
| Current project state | `.context/current.md` |
| Active sprint and goal | `tasks/active.md` then `tasks/sprints/sprint-NN-*.md` |
| AI self-correction lessons | `tasks/lessons.md` |
| Tech stack / folder structure / patterns | `docs/architecture.md` |
| Architectural decision history | `docs/decisions.md` |
| Docs-only routing | `docs/INDEX.md` |
| App overview and commands | `README.md` |
| Frontend entry point | `src/main.ts` |
| Frontend UI modules | `src/ui/` |
| IPC wrappers and shared types | `src/ipc.ts`, `src/types.ts` |
| Frontend styles | `src/style.css` |
| Tauri app setup | `src-tauri/src/lib.rs`, `src-tauri/src/main.rs` |
| Tauri commands | `src-tauri/src/commands.rs` |
| Recording/audio code | `src-tauri/src/recording/`, `src-tauri/src/audio/` |
| Transcription engines | `src-tauri/src/engine.rs`, `src-tauri/src/transcribe_local.rs`, `src-tauri/src/transcribe_groq.rs` |
| Text-to-speech | `src-tauri/src/tts.rs`, `src-tauri/src/tts_groq.rs` |
| App settings and history | `src-tauri/src/settings.rs`, `src-tauri/src/history.rs` |
| Startup/autostart UI and backend | `src/ui/startup-panel.ts`, `src-tauri/src/single_instance.rs` |
| Build environment discovery | `scripts/build-env.mjs` |
| Tauri wrapper scripts | `scripts/tauri.mjs`, `scripts/run-app.mjs`, `scripts/free-port.mjs` |
| Sprint-log hook scripts | `scripts/sprint-log/` |
| Env variable template | `.env.example` |

## Root

| File | What's inside |
|---|---|
| `AGENTS.md` | Cross-tool instructions for AI coding assistants. |
| `CLAUDE.md` | Session workflow, sprint logging, folder ownership, code rules, and testing protocol. |
| `INDEX.md` | This master file map. |
| `README.md` | Human-facing app overview and primary npm commands. |
| `package.json` | Node scripts and frontend/Tauri dependencies. |
| `tsconfig.json` | TypeScript compiler settings. |
| `vite.config.ts` | Vite dev server/build config. |
| `index.html` | Frontend HTML shell. |
| `.env.example` | Env template. No runtime app secrets are required yet. |
| `.gitignore` | Version-control exclusions. |

## `.context/`

| File | What's inside |
|---|---|
| `current.md` | Current project snapshot: name, stage, active sprint, stack, last decision, status flags. |

## `tasks/`

| File | What's inside |
|---|---|
| `active.md` | Active sprint routing table. |
| `lessons.md` | Active and internalized AI self-correction lessons. |
| `sprints/sprint-01-integrate-starter-system.md` | Sprint tracking the starter-system integration and immediate repo workflow setup. |
| `archive/` | Closed sprint files. |

## `docs/`

| File | What's inside |
|---|---|
| `INDEX.md` | Docs-only sub-index. |
| `architecture.md` | Stack, folder structure, key patterns, and external services. |
| `decisions.md` | Append-only decision log. |
| `superpowers/specs/2026-04-08-typr-dictation-app-design.md` | Original product/design spec. |
| `superpowers/plans/2026-04-08-typr-dictation-app.md` | Original implementation plan and notes. |

## `src/`

| Path | What's inside |
|---|---|
| `main.ts` | Frontend bootstrapping and UI initialization. |
| `ipc.ts` | Tauri command wrappers used by the frontend. |
| `types.ts` | Shared frontend type definitions. |
| `store.ts` | Frontend state helpers. |
| `style.css` | App styles. |
| `overlay.html`, `overlay.ts` | Overlay window UI. |
| `ui/` | Panels and UI components for console, engine, history, hotkeys, mic, modes, onboarding, startup, status, TTS, To Markdown, and window controls. |
| `ui/markitdown-panel.ts` | "To Markdown" page: file/URL pick, convert via MarkItDown, copy/save result. |

## `src-tauri/`

| Path | What's inside |
|---|---|
| `tauri.conf.json` | Tauri product/build/window/bundle config. |
| `Cargo.toml`, `Cargo.lock` | Rust package manifest and lockfile. |
| `build.rs` | Tauri build script. |
| `capabilities/default.json` | Tauri capability permissions. |
| `src/main.rs` | Native entrypoint. |
| `src/lib.rs` | Tauri app builder/plugin setup. |
| `src/commands.rs` | Commands exposed to the frontend. |
| `src/app_state.rs` | Shared app state. |
| `src/audio/`, `src/recording/` | Capture, recording state, notifications, and DSP. |
| `src/engine.rs`, `src/transcribe_local.rs`, `src/transcribe_groq.rs` | Transcription engine selection and implementations. |
| `src/tts.rs`, `src/tts_groq.rs` | Text-to-speech support. |
| `src/settings.rs`, `src/history.rs` | Settings and history persistence. |
| `src/hotkey.rs`, `src/paste.rs`, `src/cleanup.rs`, `src/console.rs`, `src/downloader.rs`, `src/single_instance.rs` | Platform helpers and app services. |
| `icons/` | App icons. |

## `scripts/`

| File | What's inside |
|---|---|
| `build-env.mjs` | Discovers Windows Rust/native build dependencies and writes Cargo env config. |
| `tauri.mjs` | Tauri CLI wrapper that applies build environment setup. |
| `run-app.mjs` | Launches the built desktop app. |
| `free-port.mjs` | Frees the Vite dev port before dev startup. |
| `sprint-log/record-edit.mjs` | Claude Code PostToolUse hook that records source edits. |
| `sprint-log/check-sprint-log.mjs` | Claude Code Stop hook that enforces sprint-log updates after source edits. |

## `.claude/`

| File | What's inside |
|---|---|
| `settings.json` | Claude Code permissions and sprint-log hooks. |
| `scheduled_tasks.lock` | Existing local lock file. |
| `sprint-sessions/` | Gitignored transient hook state. |

## Maintenance

If you cannot find what you need above, the index is stale. Search, then update this file before finishing the task.
