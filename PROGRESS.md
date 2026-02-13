# Progress

- Pending:

- Remote branches: tracking + switch/manage remote-only branches.
	- List remote branches in Branch Manager (distinct marker).
	- Allow checkout of remote branch to create local tracking branch.
	- Show remote branch deletion action (safe-guarded).
	- Cache/refresh remote branch list on view enter.
- Upstream tracking: ahead/behind display and sync status.
- Repo health UX: surface recovery actions inline for common Git failures.
- Docs audit: fix Features/Architecture inconsistencies and stale dependency versions.
- Testing: expand integration tests and add UI workflow coverage.
- AI/ML foundations: commit message suggestions MVP.


- Completed:

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
