//! Async task management for background Git operations
//!
//! This module provides a simple background task executor that runs Git operations
//! (fetch, push, pull) in separate threads without blocking the UI event loop.
//!
//! # Architecture
//!
//! Uses a channel-based approach rather than full async/await because:
//! 1. **ratatui compatibility**: ratatui's event loop is synchronous
//! 2. **Simplicity**: Easier to integrate with existing event handling
//! 3. **Resource efficiency**: Thread pool avoids spawning many threads
//!
//! # Usage
//!
//! ```no_run
//! use std::path::PathBuf;
//! use forge::async_task::{TaskManager, GitOperation};
//!
//! // Create a task manager
//! let mut tm = TaskManager::new();
//!
//! // Spawn a background fetch task
//! tm.spawn_operation(PathBuf::from("/path/to/repo"), GitOperation::Fetch("origin".into()));
//!
//! // Poll for completion in your event loop
//! if let Some(result) = tm.try_recv() {
//!     match result.result {
//!         Ok(status) => println!("Success: {}", status),
//!         Err(e) => println!("Error: {}", e),
//!     }
//! }
//! ```

use std::{path::PathBuf, thread};

use crossbeam::channel::{unbounded, Receiver, Sender};

use crate::{git, git::GitClient};

/// Result of a Git operation
pub type OpResult = Result<String, String>;

/// Result with operation metadata
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub op: GitOperation,
    pub result: OpResult,
}

/// Git operations that can be performed asynchronously
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitOperation {
    Fetch(String), // remote name
    Push(String),  // remote name
    Pull(String),  // remote name
}

/// Task manager for background Git operations
///
/// Handles spawning, tracking, and receiving results from background Git tasks
pub struct TaskManager {
    sender: Sender<OperationResult>,
    receiver: Receiver<OperationResult>,
    pending: usize,
}

impl TaskManager {
    /// Create a new task manager
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            sender,
            receiver,
            pending: 0,
        }
    }

    /// Spawn a background Git operation
    ///
    /// Returns immediately; result can be polled with `try_recv()`
    pub fn spawn_operation(&mut self, workdir: PathBuf, op: GitOperation) {
        self.pending += 1;
        let sender = self.sender.clone();

        thread::spawn(move || {
            let op_clone = op.clone();
            let result = run_git_operation(&workdir, &op_clone);

            // Send result back to main thread
            let _ = sender.send(OperationResult {
                op: op_clone,
                result,
            });
        });
    }

    /// Check if there's a completed operation result
    ///
    /// Returns `Some(result)` if an operation completed, `None` if no operations
    /// are available yet or all are still pending
    pub fn try_recv(&mut self) -> Option<OperationResult> {
        if self.pending == 0 {
            return None;
        }

        match self.receiver.try_recv() {
            Ok(result) => {
                self.pending -= 1;
                Some(result)
            }
            Err(_) => None,
        }
    }

    /// Get number of pending operations
    pub fn pending_count(&self) -> usize {
        self.pending
    }

    /// Check if any operations are currently pending
    pub fn has_pending(&self) -> bool {
        self.pending > 0
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

fn run_git_operation(workdir: &PathBuf, op: &GitOperation) -> OpResult {
    let client = match GitClient::discover(workdir) {
        Ok(client) => client,
        Err(e) => {
            return Err(git::GitClient::explain_error(&e));
        }
    };

    match op {
        GitOperation::Fetch(remote) => client
            .fetch(remote)
            .map(|count| format!("Fetched {} objects from {}", count, remote))
            .map_err(|e| git::GitClient::explain_error(&e)),
        GitOperation::Push(remote) => client
            .push(remote, None)
            .map(|_| format!("Pushed to {}", remote))
            .map_err(|e| git::GitClient::explain_error(&e)),
        GitOperation::Pull(remote) => client
            .pull(remote, None)
            .map(|_| format!("Pulled from {}", remote))
            .map_err(|e| git::GitClient::explain_error(&e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_temp_repo() -> TempDir {
        let temp = TempDir::new().expect("Failed to create temp dir");
        git2::Repository::init(temp.path()).expect("Failed to init repo");
        temp
    }

    #[test]
    fn test_task_manager_creation() {
        let tm = TaskManager::new();
        assert_eq!(tm.pending_count(), 0);
        assert!(!tm.has_pending());
    }

    #[test]
    fn test_spawn_operation() {
        let mut tm = TaskManager::new();
        let _repo = init_temp_repo();
        let repo_path = _repo.path().to_path_buf();
        tm.spawn_operation(repo_path, GitOperation::Fetch("origin".to_string()));
        assert_eq!(tm.pending_count(), 1);
        assert!(tm.has_pending());

        // Wait for background thread to complete before TempDir is dropped
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    #[test]
    fn test_try_recv_completes() {
        let mut tm = TaskManager::new();
        let _repo = init_temp_repo();
        let repo_path = _repo.path().to_path_buf();
        tm.spawn_operation(repo_path, GitOperation::Fetch("origin".to_string()));

        // Wait a bit for the thread to complete
        std::thread::sleep(std::time::Duration::from_millis(150));

        let result = tm.try_recv();
        assert!(result.is_some());
        assert_eq!(tm.pending_count(), 0);
    }
}
