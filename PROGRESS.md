# Progress

- Pending:
  - [ ] Upstream tracking display
    - [ ] Detect upstream branch
    - [ ] Show upstream status (ahead/behind)
    - [ ] Handle untracked branches
  - [ ] Handle corrupted Git repos
    - [ ] Add validation on repo open
    - [ ] Implement recovery options
    - [ ] Display user-friendly errors
  - [ ] Terminal resize handling
    - [ ] Listen for resize events
    - [ ] Redraw UI on resize
    - [ ] Test with various terminal sizes
  - [ ] Large repo performance optimization
    - [ ] Profile hot paths
    - [ ] Optimize git operations
    - [ ] Cache expensive computations
  - [ ] Error message improvements
    - [ ] Standardize error formatting
    - [ ] Add context to messages
    - [ ] Test user comprehension

- Ongoing: [ ]

- Completed:
  - [x] Remote branch tracking
    - [x] Parse remote branches from git2
    - [x] Display remote branches in UI
    - [x] Handle branch deletion
  - [x] Remote fetch/pull/push with progress
  - [x] State extraction into page structs
  - [x] Background task improvements
