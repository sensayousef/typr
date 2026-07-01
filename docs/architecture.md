# Architecture

Immutable reference. Describes what was chosen and why. Add new sections when the stack fundamentally changes.

## Stack

| Layer | Technology | Reason |
|-------|------------|--------|
| Desktop shell | Tauri 2 | Native desktop app with Rust backend and web UI. |
| Frontend | TypeScript + Vite | Fast local UI iteration and typed browser-side code. |
| Native backend | Rust | Audio capture, transcription orchestration, hotkeys, paste integration, settings/history, and platform services. |
| Audio/transcription | Whisper/local modules plus optional Groq-related modules | Voice-first dictation and cleanup workflows. |
| Persistence | Local files/settings/history | Local-first desktop behavior without a database server. |
| Testing | Not fully documented yet | Add the official strategy once the app's test suite is settled. |
| CI/CD | Not active | Add a real workflow after build/test commands are confirmed. |

## Folder Structure

```text
src/
  main.ts                 # Frontend boot and UI wiring
  ipc.ts                  # Tauri command wrappers
  types.ts                # Shared frontend types
  ui/                     # UI panels and controls

src-tauri/
  src/                    # Rust app backend and Tauri commands
  capabilities/           # Tauri permissions
  icons/                  # App icons

scripts/
  build-env.mjs           # Windows native build environment discovery
  tauri.mjs               # Tauri CLI wrapper
  run-app.mjs             # Built app launcher
  sprint-log/             # Starter workflow hook scripts

docs/
  superpowers/            # Original product specs/plans
```

## Key Patterns

- Frontend UI is organized by focused modules under `src/ui/`.
- Frontend-to-backend calls should go through typed wrappers in `src/ipc.ts`.
- Native behavior lives in small Rust modules under `src-tauri/src/`.
- Build environment discovery is centralized in `scripts/build-env.mjs`.
- Workflow metadata is kept in `INDEX.md`, `.context/`, `tasks/`, and `docs/`.

## External Services

| Service | Purpose | Notes |
|---------|---------|-------|
| Groq-related modules | Optional transcription/TTS paths | Confirm runtime configuration before documenting required secrets. |
