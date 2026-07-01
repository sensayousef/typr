# CLAUDE.md

## Session Start
1. Read `INDEX.md` first. It is the master file map.
2. Read `.context/current.md` for the current project state.
3. Read `tasks/active.md` for the active sprint list.
4. Read each sprint file listed in `tasks/active.md`.
5. Read `tasks/lessons.md`, focusing on `## Active`.

## Lookup Rule
Before searching the filesystem for any file, scan `INDEX.md`. The Quick "Where do I look for..." table covers the common case. Fall back to search only when the index does not list the target. If a file is missing from the index, update the index in the same task.

## Workflow
- Any task: plan, implement, verify, update the relevant sprint file.
- Non-trivial work: write down the plan before coding when the tool supports it.
- After any correction: add an entry to `tasks/lessons.md` under `## Active` immediately.
- Sprint ends: move the sprint file to `tasks/archive/`, remove it from `tasks/active.md`, and update `.context/current.md` only when the user explicitly says to close that sprint.
- Lesson internalized: move it from `## Active` to `## Internalized` after it has stayed useful across multiple sprints.

## Sprint File Update Rule

After every implementation session, append a dated entry to the active sprint file before the session ends.

Which sprint is current: the lowest-numbered sprint still listed in `tasks/active.md`. A sprint stays current until the user explicitly closes it, regardless of what later sprints exist or what `.context/current.md` highlights.

Append entries under a `## Session Log` section using this format:

```markdown
### YYYY-MM-DD - <one-line summary>
- What changed: <bullet list of files/components modified>
- Why: <brief reason or test failure that prompted the work>
- Status: <what is now working / what is still open>
```

Enforcement for Claude Code lives in `.claude/settings.json`. The PostToolUse hook `scripts/sprint-log/record-edit.mjs` tracks source edits, and the Stop hook `scripts/sprint-log/check-sprint-log.mjs` blocks session end until a sprint file is updated.

## Folder Ownership

One source of truth per concern. Do not add a folder without updating this table and `INDEX.md`.

| Concern | Location |
|---------|----------|
| Master file map | `INDEX.md` |
| Current project focus | `.context/current.md` |
| Active sprint list | `tasks/active.md` |
| Sprint task files | `tasks/sprints/sprint-NN-name.md` |
| Completed sprints | `tasks/archive/` |
| Lessons | `tasks/lessons.md` |
| Docs routing index | `docs/INDEX.md` |
| Architecture reference | `docs/architecture.md` |
| Decision log | `docs/decisions.md` |
| Env variable template | `.env.example` |
| Build and utility scripts | `scripts/` |
| Frontend source | `src/` |
| Tauri/Rust source | `src-tauri/` |

## Docs / Index Rule
When any file is created, moved, renamed, or changes scope, update `INDEX.md` in the same task. If the change is doc-only, also update `docs/INDEX.md`.

## Code Rules
- Preserve user-owned work in the dirty worktree.
- Keep app behavior changes scoped to the feature or bug being touched.
- Validate user input and external data at boundaries.
- Handle errors explicitly; do not swallow failures silently.
- Prefer existing Typr/Robin patterns over new abstractions.
- Keep generated build artifacts out of version control.

## Browser / System Testing Protocol

When running a browser test, E2E run, or any "test the app / system test" task, the testing pass is find-and-report unless the user explicitly asks for fixes in the same task.

- Keep an explicit checklist of items to verify.
- When an item passes or fails, close that item.
- On failure, record the finding and stop the test pass instead of debugging in a loop.
- End with what passed, what failed, and what could not be verified.

## Read Aloud Report

Cross-tool development rule. Mirrored in `AGENTS.md`.

End every coding-assistant response that completes meaningful repository work, and every development session wrap-up, with a final section headed exactly `## 🔊 Read Aloud`.

- Plain spoken prose only. No bullets, no markdown symbols, no code blocks, no backticks, no emoji inside the spoken text, no raw file paths.
- Spell things out for speech: say "the App dot tsx file" not `App.tsx`, "sprint seven" not `sprint-07`.
- Cumulative and self-contained: cover what was asked, what changed and why, what was verified, and what is still open.
- Place it last, after all other output. If a response did no real work, the section may be skipped.

## Before Done
- [ ] Behavior tested when behavior changed
- [ ] No new lint/type/build errors introduced
- [ ] No duplicate folders or files created
- [ ] Relevant sprint file updated
- [ ] `.context/current.md` updated if sprint, stage, or stack changed
- [ ] `INDEX.md` updated if any file was added, moved, renamed, or changed scope
- [ ] `docs/INDEX.md` updated for docs-only changes
