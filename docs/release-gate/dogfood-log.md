# Dogfood Log

Release-gate requirement: run Forge against at least 3 real repositories for one week and log friction items.

## Baseline Run (2026-04-01)

| Repo | Check Type | Result | Friction Items |
| --- | --- | --- | --- |
| /home/pixel/Projects/forge | startup smoke (`timeout 3s`) | pass (`startup_ok_timeout`) | none |
| /home/pixel/Projects/IMS | startup smoke (`timeout 3s`) | pass (`startup_ok_timeout`) | none |
| /home/pixel/Projects/Portfolio | startup smoke (`timeout 3s`) | pass (`startup_ok_timeout`) | none |

## Week-Long Tracking Template

| Date | Repo | Workflow Tested | Friction | Severity (P0/P1/P2) | Status |
| --- | --- | --- | --- | --- | --- |
| YYYY-MM-DD | repo-name | stage/commit/branch/sync | description | P1 | open |
