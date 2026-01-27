//! Page state modules for the Forge TUI application.
//!
//! This module contains isolated state structs for each page/view in the application.
//! Extracting page-specific state from the monolithic `App` struct improves:
//! - Testability: Each page's state can be unit tested independently
//! - Maintainability: Clear separation of concerns
//! - Readability: Reduced cognitive load when working with specific pages
//!
//! # Architecture
//!
//! ```text
//! App
//! ├── DashboardState      - Project list navigation
//! ├── ChangesState        - Git staging and commit interface
//! ├── BoardState          - Kanban board navigation
//! ├── MergeState          - Conflict resolution state
//! ├── ModuleManagerState  - Module/developer management
//! ├── BranchManagerState  - Branch operations
//! └── CommitHistoryState  - Commit history navigation
//! ```

mod board;
mod branch_manager;
mod changes;
mod commit_history;
mod dashboard;
mod merge;
mod module_manager;

pub use board::BoardState;
pub use branch_manager::BranchManagerState;
pub use changes::ChangesState;
pub use commit_history::CommitHistoryState;
pub use dashboard::DashboardState;
pub use merge::MergeState;
pub use module_manager::ModuleManagerState;
