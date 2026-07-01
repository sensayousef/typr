# Decision Log

Append-only. Each entry is an Architecture Decision Record. Never edit past entries; add a new one when decisions change.

## Format

```markdown
### [YYYY-MM-DD] Decision title
**Context:** Why this choice was needed  
**Decision:** What was chosen  
**Alternatives:** What was ruled out and why  
**Consequences:** What this enables or constrains
```

---

### [2026-07-01] Integrate starter workflow as metadata, not app behavior
**Context:** The Project Starter Template contains repo workflow files, sprint tracking, docs routing, and assistant instructions. Robin/Typr already has product code, app docs, scripts, and a dirty worktree.  
**Decision:** Add and adapt the starter workflow layer while preserving existing app source and scripts. Do not overwrite product files or import placeholder CI as if it were a working app pipeline.  
**Alternatives:** Blindly copy the template over the repo; ruled out because it could overwrite real app files or introduce placeholder guidance. Skip the starter files; ruled out because the requested system would not be available in this project.  
**Consequences:** Future sessions have a consistent project map and sprint log system, while app behavior remains unchanged.
