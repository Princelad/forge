// Library for testable modules
pub mod data;
pub mod git;

// Re-export main types used in tests
pub use data::{Change, Developer, FileStatus, Module, ModuleStatus, Project, Store};
pub use git::GitClient;
