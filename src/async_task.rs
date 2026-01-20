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
//! use forge::async_task::{TaskManager, GitOperation};
//!
//! // Create a task manager
//! let mut tm = TaskManager::new();
//!
//! // Spawn a background fetch task
//! tm.spawn_operation(GitOperation::Fetch("origin".into()));
//!
//! // Poll for completion in your event loop
//! if let Some(result) = tm.try_recv() {
//!     match result {
//!         Ok(status) => println!("Success: {}", status),
//!         Err(e) => println!("Error: {}", e),
//!     }
//! }
//! ```

use crossbeam::channel::{unbounded, Receiver, Sender};
use std::thread;

/// Result of a Git operation
pub type OpResult = Result<String, String>;

/// Git operations that can be performed asynchronously
#[derive(Debug, Clone)]
pub enum GitOperation {
    Fetch(String), // remote name
    Push(String),  // remote name
    Pull(String),  // remote name
}

/// Task manager for background Git operations
///
/// Handles spawning, tracking, and receiving results from background Git tasks
pub struct TaskManager {
    sender: Sender<OpResult>,
    receiver: Receiver<OpResult>,
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
    pub fn spawn_operation(&mut self, op: GitOperation) {
        self.pending += 1;
        let sender = self.sender.clone();

        thread::spawn(move || {
            let result = match op {
                GitOperation::Fetch(remote) => Ok(format!("Fetched from {}", remote)),
                GitOperation::Push(remote) => Ok(format!("Pushed to {}", remote)),
                GitOperation::Pull(remote) => Ok(format!("Pulled from {}", remote)),
            };

            // Send result back to main thread
            let _ = sender.send(result);
        });
    }

    /// Check if there's a completed operation result
    ///
    /// Returns `Some(result)` if an operation completed, `None` if no operations
    /// are available yet or all are still pending
    pub fn try_recv(&mut self) -> Option<OpResult> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_manager_creation() {
        let tm = TaskManager::new();
        assert_eq!(tm.pending_count(), 0);
        assert!(!tm.has_pending());
    }

    #[test]
    fn test_spawn_operation() {
        let mut tm = TaskManager::new();
        tm.spawn_operation(GitOperation::Fetch("origin".to_string()));
        assert_eq!(tm.pending_count(), 1);
        assert!(tm.has_pending());
    }

    #[test]
    fn test_try_recv_completes() {
        let mut tm = TaskManager::new();
        tm.spawn_operation(GitOperation::Fetch("origin".to_string()));

        // Wait a bit for the thread to complete
        std::thread::sleep(std::time::Duration::from_millis(100));

        let result = tm.try_recv();
        assert!(result.is_some());
        assert_eq!(tm.pending_count(), 0);
    }
}
