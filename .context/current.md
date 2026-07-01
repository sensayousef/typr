# Current State

Keep this under 40 lines. Update whenever the sprint, stage, stack, or major decision changes.

## Project
**Name:** Robin, in the `typr` repo  
**Description:** Local Tauri desktop sidekick for voice-first AI workflows: global hotkey recording, transcription, cleanup, paste, history, console, and text-to-speech.  
**Stage:** MVP / active hardening

## Active Sprint
**Sprint:** Sprint 1 - Integrate starter system  
**Goal:** Bring the Project Starter Template workflow, docs map, context, sprint tracking, and hook scripts into this app without disturbing existing product work.  
**Blocking:** None for metadata integration. App verification depends on the current dirty product worktree and local Windows build environment.

## Tech Stack
TypeScript + Vite frontend, Tauri 2 desktop shell, Rust native backend, Whisper/local audio pipeline, optional Groq-related transcription/TTS modules.

## Last Decision
[2026-07-01] Integrate starter workflow as metadata, not app behavior

## Status Flags
- [x] Desktop app scaffolded
- [x] Tauri build scripts present
- [x] Project workflow docs installed
- [ ] CI/CD active
- [ ] Automated test suite documented
