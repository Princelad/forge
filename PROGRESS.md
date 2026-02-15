# Progress

- Pending:

- AI/ML foundations: commit message suggestions MVP.
	- Define MVP goals, non-goals, and acceptance criteria.
	- Inventory current commit flow and integration points.
	- Identify data sources for suggestions (staged diff, status, previous messages).
	- Decide on on-device heuristic vs. local ML baseline for MVP.
	- Draft prompt/template format for suggestions output.
	- Create suggestion generation API surface (trait or function) in codebase.
	- Implement diff summarization (files, hunks, scopes).
	- Normalize diff text (strip binaries, limit size, redact secrets).
	- Add commit context builder (branch name, issue keys, scope).
	- Implement baseline suggestion engine (rule-based).
	- Add ranking/deduping logic for multiple suggestions.
	- Add configurable max suggestions and length limits.
	- Wire into UI: new panel/section for suggestions.
	- Add keybinding to accept and edit a suggestion.
	- Add fallback messaging when no suggestions available.
	- Add settings for enable/disable and behavior.
	- Add telemetry/logging hooks for debug (local only).
	- Add unit tests for diff summarization and suggestion rules.
	- Add integration test for suggestion flow in commit screen.
	- Update docs with usage and limitations.
	- Add entry in CHANGELOG for MVP feature.
- Docs: video tutorials.
- Docs: use case examples.
- Docs: architecture deep-dive.


- Completed:

- Testing: expand integration tests and add UI workflow coverage.
- Repo health UX: surface recovery actions inline for common Git failures.
- Remote branches: tracking + switch/manage remote-only branches.
- Upstream tracking: ahead/behind display and sync status.
- Docs audit: fix Features/Architecture inconsistencies and stale dependency versions.
- Remote ops: wire fetch/push keybindings to actions.
- Remote ops: surface remote selection (beyond hardcoded origin).
- Merge: apply accepted pane as real conflict resolution flow.
- Merge: add conflict list from real merge state instead of general changes.
- Settings: persist theme/notifications/autosync to config.
- Settings: implement notifications/autosync behavior (currently placeholders).
- Keybindings: load config file (TOML).
- Keybindings: validate and report errors.
- Keybindings: detect conflicting bindings across actions.
- Stash: list existing stashes.
- Stash: create stash with message.
- Stash: apply and pop stash.
- Stash: drop stash entry.
- Cherry-pick: single commit flow.
- Cherry-pick: conflict handling UX.
- Troubleshooting guide: common Git errors.
- Troubleshooting guide: recovery steps.
- Integration tests: repo fixture helpers.
- Integration tests: core Git ops (status, stage, commit).
