// Library for testable modules
pub mod data;
pub mod git;

// Re-export main types used in tests
pub use data::{Change, Developer, FakeStore, FileStatus, Module, ModuleStatus, Project};
pub use git::GitClient;
