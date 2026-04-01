# Terminal Walkthroughs

This directory contains short, scriptable terminal walkthroughs for first-time users.

## Included walkthroughs

1. `quickstart.tape`: init -> stage -> commit -> branch -> sync
2. `troubleshooting.tape`: common failure recovery paths
3. `suggestions.tape`: commit suggestion apply/edit flow

## Generate GIF/video locally

Use [Charm VHS](https://github.com/charmbracelet/vhs) to render `.tape` files.

```bash
# Install VHS (example)
brew install vhs

# Render GIFs
vhs docs/walkthroughs/quickstart.tape
vhs docs/walkthroughs/troubleshooting.tape
vhs docs/walkthroughs/suggestions.tape
```

Generated output files are written next to each `.tape` script.
