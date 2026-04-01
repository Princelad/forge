# Commit Suggestions Reference

This file is the source content for README/wiki documentation of Forge commit suggestions.

## Overview

Forge uses a rule-based engine to generate conventional-style commit messages from staged changes.

## Behavior

1. Suggestions are generated only from staged files.
2. Suggestions are scored and sorted by confidence.
3. Users can apply a suggestion with `1-3` and manually edit before committing.
4. Branch context (for example issue keys) may be included in suggestion formatting.

## Limits

1. Maximum suggestion count is controlled by `max_suggestions` (1-5).
2. Message length is limited by `max_length` (20-200).
3. If no staged files exist, suggestions are intentionally empty.

## Configuration

Persisted in `.forge/settings.json` under:

- `suggestions.enabled`
- `suggestions.max_suggestions`
- `suggestions.max_length`

These values are validated on startup. Invalid values produce a startup diagnostic in the status bar.
