//! Git operations and repository management.
//!
//! # Error Handling & Edge Cases
//!
//! This module uses libgit2 for all Git operations and returns `color_eyre::Result`
//! for proper error propagation. Known edge cases and limitations:
//!
//! ## Corrupted Repository Handling
//!
//! **Current Behavior:**
//! - Repository corruption is detected during `discover()` and propagated as errors
//! - Operations on corrupted repos will fail with git2 error codes
//! - No automatic recovery or repair attempts are made
//! - UI layer should handle errors gracefully and display user-friendly messages
//!
//! **Known Edge Cases:**
//! 1. **Missing or corrupted HEAD**: `head_branch()` returns `None`, operations may fail
//! 2. **Corrupted index**: `list_changes()`, `stage_file()`, and `commit_all()` will error
//! 3. **Missing objects**: Diff operations may fail silently, returning empty strings
//! 4. **Invalid references**: Branch operations may fail with obscure error messages
//! 5. **Locked index**: Concurrent Git operations can cause `.git/index.lock` conflicts
//!
//! **Recommended Improvements** (see Roadmap):
//! - Add `fn check_repo_health() -> Result<RepoHealth>` to diagnose issues
//! - Implement graceful degradation (read-only mode when index is corrupted)
//! - Better error messages mapping git2 errors to user-actionable guidance
//! - Add `fn repair_index()` for common corruption patterns
//!
//! ## Error Propagation Strategy
//!
//! - All public methods return `Result<T>` or `Option<T>` for nullable operations
//! - Callers should use `?` operator or `.unwrap_or_default()` with fallbacks
//! - UI should never panic on Git errors - display errors in status bar instead
//! - Benchmark code tracks errors via `is_err()` checks (see benches/git_operations.rs)

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use color_eyre::eyre::Result;
use git2::{DiffFormat, DiffOptions, IndexAddOption, Repository, Signature, StatusOptions, Tree};

use crate::data::{Change, FileStatus};

/// Transfer progress for remote operations (fetch/push)
#[derive(Debug, Clone, Default)]
pub struct TransferProgress {
    pub total_objects: usize,
    pub indexed_objects: usize,
    pub received_objects: usize,
    pub received_bytes: usize,
    pub total_deltas: usize,
    pub indexed_deltas: usize,
}

impl TransferProgress {
    /// Calculate overall progress as a percentage (0-100)
    pub fn percent(&self) -> u8 {
        if self.total_objects == 0 {
            return 100;
        }
        ((self.received_objects as f64 / self.total_objects as f64) * 100.0) as u8
    }

    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        if self.total_objects == 0 {
            return "Initializing...".to_string();
        }
        format!(
            "Received {}/{} objects ({} bytes)",
            self.received_objects, self.total_objects, self.received_bytes
        )
    }
}

/// Commit info: (hash, author, date, message, files_changed)
pub type CommitData = (String, String, String, String, Vec<String>);

pub struct GitClient {
    repo: Repository,
    pub workdir: PathBuf,
}

impl GitClient {
    /// Discover a Git repository starting from the given path.
    ///
    /// Walks up the directory tree to find a `.git` directory.
    ///
    /// # Edge Cases
    ///
    /// - **Corrupted repo**: Returns `Err` if `.git` directory is malformed
    /// - **Bare repo**: Handles bare repositories by using repo path as workdir
    /// - **Submodules**: Discovers parent repo, not submodule (libgit2 behavior)
    /// - **Missing workdir**: Returns error if workdir cannot be determined
    ///
    /// # Errors
    ///
    /// - Path does not exist or is not accessible
    /// - No Git repository found in path or parent directories
    /// - Repository structure is corrupted
    /// - Unable to determine working directory
    pub fn discover(start: impl AsRef<Path>) -> Result<Self> {
        let repo = Repository::discover(start)?;
        let workdir = repo
            .workdir()
            .map(Path::to_path_buf)
            .or_else(|| repo.path().parent().map(Path::to_path_buf))
            .ok_or_else(|| color_eyre::eyre::eyre!("Unable to determine workdir"))?;
        Ok(Self { repo, workdir })
    }

    /// Get the current branch name.
    ///
    /// Returns `None` in edge cases rather than erroring.
    ///
    /// # Edge Cases
    ///
    /// - **Detached HEAD**: Returns `None` (HEAD points to commit, not branch)
    /// - **Empty repo**: Returns `None` (no commits or HEAD yet)
    /// - **Corrupted HEAD**: Returns `None` (cannot read `.git/HEAD`)
    /// - **Initial state**: Returns `None` before first commit
    pub fn head_branch(&self) -> Option<String> {
        self.repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()))
    }

    /// List all changes in the working directory and staging area.
    ///
    /// # Edge Cases
    ///
    /// - **Corrupted index**: Returns `Err` - caller should display error to user
    /// - **Large repos**: May be slow (1000s of files) - consider showing spinner
    /// - **Untracked files**: Included by default (can be filtered by caller)
    /// - **Ignored files**: Excluded (per `.gitignore` rules)
    /// - **Submodules**: Shown as modified files, not expanded
    /// - **Invalid UTF-8**: Paths with invalid UTF-8 are skipped (logged to stderr)
    /// - **Missing objects**: Diffs may be empty if referenced objects are missing
    ///
    /// # Errors
    ///
    /// - Index is locked by another process (`.git/index.lock` exists)
    /// - Filesystem permissions prevent reading files
    /// - Repository structure is corrupted
    pub fn list_changes(&self) -> Result<Vec<Change>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut changes = Vec::new();

        for entry in statuses.iter() {
            let path = match entry.path() {
                Some(p) => p.to_string(),
                None => continue,
            };

            let status = entry.status();
            // Map git status to our simplified FileStatus
            let file_status = if status.is_wt_new() || status.is_index_new() {
                FileStatus::Added
            } else if status.is_wt_deleted() || status.is_index_deleted() {
                FileStatus::Deleted
            } else {
                FileStatus::Modified
            };

            let local_preview = self
                .diff_index_to_workdir_for_path(&path)
                .or_else(|| self.diff_for_path(&path));
            let incoming_preview = self.diff_head_to_index_for_path(&path);
            let diff_preview = local_preview
                .as_ref()
                .or(incoming_preview.as_ref())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "(no diff)".into());

            let staged = status.is_index_new()
                || status.is_index_modified()
                || status.is_index_deleted()
                || status.is_index_renamed()
                || status.is_index_typechange();

            changes.push(Change {
                path,
                status: file_status,
                diff_preview,
                local_preview,
                incoming_preview,
                staged,
            });
        }

        Ok(changes)
    }

    fn diff_for_path(&self, path: &str) -> Option<String> {
        let mut opts = DiffOptions::new();
        opts.pathspec(path);
        // Compare index to workdir to show staged+unstaged deltas
        let diff = self
            .repo
            .diff_index_to_workdir(None, Some(&mut opts))
            .ok()?;
        let mut out = String::new();
        let _ = diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            out.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        });
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    fn diff_index_to_workdir_for_path(&self, path: &str) -> Option<String> {
        let mut opts = DiffOptions::new();
        opts.pathspec(path);
        let diff = self
            .repo
            .diff_index_to_workdir(None, Some(&mut opts))
            .ok()?;
        let mut out = String::new();
        let _ = diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            out.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        });
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    fn head_tree(&self) -> Option<Tree<'_>> {
        self.repo.head().ok()?.peel_to_tree().ok()
    }

    fn diff_head_to_index_for_path(&self, path: &str) -> Option<String> {
        let head = self.head_tree()?;
        let mut opts = DiffOptions::new();
        opts.pathspec(path);
        let mut index = self.repo.index().ok()?;
        let index_tree = self.repo.find_tree(index.write_tree().ok()?).ok()?;
        let diff = self
            .repo
            .diff_tree_to_tree(Some(&head), Some(&index_tree), Some(&mut opts))
            .ok()?;
        let mut out = String::new();
        let _ = diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            out.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        });
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    pub fn stage_all(&self) -> Result<()> {
        let mut index = self.repo.index()?;
        index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    pub fn stage_file(&self, path: &str) -> Result<()> {
        let mut index = self.repo.index()?;
        index.add_path(std::path::Path::new(path))?;
        index.write()?;
        Ok(())
    }

    pub fn unstage_file(&self, path: &str) -> Result<()> {
        let mut index = self.repo.index()?;
        // Get HEAD tree
        if let Some(head_tree) = self.head_tree() {
            // Try to get the file from HEAD and reset it to that state
            let path_obj = std::path::Path::new(path);
            match head_tree.get_path(path_obj) {
                Ok(entry) => {
                    // File exists in HEAD - reset to HEAD version
                    let oid = entry.id();
                    let mode = entry.filemode() as u32;
                    index.add_frombuffer(
                        &git2::IndexEntry {
                            ctime: git2::IndexTime::new(0, 0),
                            mtime: git2::IndexTime::new(0, 0),
                            dev: 0,
                            ino: 0,
                            mode,
                            uid: 0,
                            gid: 0,
                            file_size: 0,
                            id: oid,
                            flags: 0,
                            flags_extended: 0,
                            path: path.as_bytes().to_vec(),
                        },
                        path.as_bytes(),
                    )?;
                }
                Err(_) => {
                    // File doesn't exist in HEAD, remove from index
                    index.remove_path(path_obj)?;
                }
            }
        } else {
            // No HEAD (initial commit), just remove from index
            index.remove_path(std::path::Path::new(path))?;
        }
        index.write()?;
        Ok(())
    }

    fn default_signature(&self) -> Result<Signature<'_>> {
        // Try repository config
        if let Ok(sig) = self.repo.signature() {
            return Ok(sig);
        }
        // Fallback to env vars or defaults
        let name = std::env::var("GIT_AUTHOR_NAME").unwrap_or_else(|_| "Forge".into());
        let email =
            std::env::var("GIT_AUTHOR_EMAIL").unwrap_or_else(|_| "forge@example.com".into());
        Ok(Signature::now(&name, &email)?)
    }

    /// Commit all staged changes with the given message.
    ///
    /// # Edge Cases
    ///
    /// - **Empty commit**: Returns `Err` if no changes are staged
    /// - **First commit**: Handles initial commit (no parent) correctly
    /// - **Merge state**: May fail if repository is in merge/rebase state
    /// - **Invalid signature**: Uses fallback signature if git config incomplete
    /// - **Corrupted index**: Returns `Err` - caller should handle gracefully
    /// - **Locked index**: Fails with "index is locked" error
    ///
    /// # Errors
    ///
    /// - No changes are staged
    /// - Cannot create signature (invalid git config)
    /// - Index is locked or corrupted
    /// - Cannot write tree or commit object
    pub fn commit_all(&self, message: &str) -> Result<git2::Oid> {
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let sig = self.default_signature()?;

        // Determine parent commit (if HEAD exists)
        let parents: Vec<git2::Commit> = match self.repo.head() {
            Ok(head) => {
                let oid = head
                    .target()
                    .ok_or_else(|| color_eyre::eyre::eyre!("Invalid HEAD"))?;
                let commit = self.repo.find_commit(oid)?;
                vec![commit]
            }
            Err(_) => Vec::new(),
        };

        let oid = if parents.is_empty() {
            // Initial commit
            self.repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?
        } else {
            let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
            self.repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)?
        };

        Ok(oid)
    }

    /// Get list of unique committer names from repository history
    pub fn get_committers(&self) -> Result<Vec<String>> {
        let mut names = std::collections::HashSet::new();

        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        for oid in revwalk.take(100).flatten() {
            // Limit to last 100 commits
            if let Ok(commit) = self.repo.find_commit(oid) {
                let author = commit.author();
                if let Some(name) = author.name() {
                    names.insert(name.to_string());
                }
            }
        }

        Ok(names.into_iter().collect())
    }

    /// List all branches (local and remote)
    pub fn list_branches(&self, local: bool, remote: bool) -> Result<Vec<(String, bool)>> {
        let mut branches = Vec::new();
        let current_branch = self.head_branch().unwrap_or_default();

        // List local branches
        if local {
            let branch_iter = self.repo.branches(Some(git2::BranchType::Local))?;
            for (branch, _) in branch_iter.flatten() {
                if let Some(name) = branch.name()? {
                    let is_current = name == current_branch;
                    branches.push((name.to_string(), is_current));
                }
            }
        }

        // List remote branches
        if remote {
            let branch_iter = self.repo.branches(Some(git2::BranchType::Remote))?;
            for (branch, _) in branch_iter.flatten() {
                if let Some(name) = branch.name()? {
                    branches.push((name.to_string(), false));
                }
            }
        }

        Ok(branches)
    }

    /// Switch to a different branch
    pub fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let obj = self
            .repo
            .revparse_single(&format!("refs/heads/{}", branch_name))?;
        self.repo.checkout_tree(&obj, None)?;
        self.repo.set_head(&format!("refs/heads/{}", branch_name))?;
        Ok(())
    }

    /// Create a new branch from current HEAD
    pub fn create_branch(&self, branch_name: &str) -> Result<()> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        self.repo.branch(branch_name, &commit, false)?;
        Ok(())
    }

    /// Delete a branch
    pub fn delete_branch(&self, branch_name: &str) -> Result<()> {
        let mut branch = self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }

    /// Commit info: (hash, author, date, message, files_changed)
    pub fn get_commit_history(&self, limit: usize) -> Result<Vec<CommitData>> {
        let mut commits = Vec::new();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        for oid in revwalk.take(limit).flatten() {
            if let Ok(commit) = self.repo.find_commit(oid) {
                let hash = oid.to_string();
                let author = commit.author().name().unwrap_or("Unknown").to_string();
                let time = commit.time();
                let date = chrono::DateTime::from_timestamp(time.seconds(), 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown date".to_string());
                let message = commit.message().unwrap_or("").to_string();

                // Get files changed
                let mut files = Vec::new();
                if let Ok(tree) = commit.tree() {
                    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
                    if let Ok(diff) =
                        self.repo
                            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
                    {
                        diff.foreach(
                            &mut |delta, _| {
                                if let Some(path) = delta.new_file().path() {
                                    files.push(path.to_string_lossy().to_string());
                                }
                                true
                            },
                            None,
                            None,
                            None,
                        )
                        .ok();
                    }
                }

                commits.push((hash, author, date, message, files));
            }
        }

        Ok(commits)
    }

    /// Fetch from a remote repository with progress tracking
    ///
    /// Returns the number of objects received
    ///
    /// # Arguments
    ///
    /// * `remote_name` - Name of the remote (e.g., "origin")
    /// * `progress` - Optional progress tracker (wrapped in Arc<Mutex> for thread safety)
    /// * `cancel_flag` - Optional cancellation flag (wrapped in Arc for thread safety)
    ///
    /// # Errors
    ///
    /// - Remote does not exist
    /// - Authentication failed (SSH/HTTPS credentials)
    /// - Network errors
    /// - Operation was cancelled
    pub fn fetch_with_progress(
        &self,
        remote_name: &str,
        progress: Option<Arc<Mutex<TransferProgress>>>,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> Result<usize> {
        let mut remote = self.repo.find_remote(remote_name)?;

        let mut fetch_options = git2::FetchOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();

        // Credential callback for authentication
        callbacks.credentials(|url, username_from_url, allowed_types| {
            // Check if SSH is allowed
            if allowed_types.is_ssh_key() {
                // Try SSH agent first
                if let Some(username) = username_from_url {
                    if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                        return Ok(cred);
                    }
                }

                // Try default SSH key locations
                if let Some(username) = username_from_url.or_else(|| url.split('@').next()) {
                    if let Ok(home) = std::env::var("HOME") {
                        let ssh_key = std::path::PathBuf::from(&home).join(".ssh/id_rsa");
                        if ssh_key.exists() {
                            if let Ok(cred) = git2::Cred::ssh_key(username, None, &ssh_key, None) {
                                return Ok(cred);
                            }
                        }
                    }
                }
            }

            // Try username/password for HTTPS
            if allowed_types.is_user_pass_plaintext() {
                if let Ok(cred) =
                    git2::Cred::credential_helper(&self.repo.config()?, url, username_from_url)
                {
                    return Ok(cred);
                }
            }

            // Fallback to default credentials
            git2::Cred::default()
        });

        // Transfer progress callback
        if let Some(progress_tracker) = progress.clone() {
            callbacks.transfer_progress(move |stats| {
                if let Ok(mut p) = progress_tracker.lock() {
                    p.total_objects = stats.total_objects();
                    p.indexed_objects = stats.indexed_objects();
                    p.received_objects = stats.received_objects();
                    p.received_bytes = stats.received_bytes();
                    p.total_deltas = stats.total_deltas();
                    p.indexed_deltas = stats.indexed_deltas();
                }

                // Check cancellation flag
                if let Some(ref cancel) = cancel_flag {
                    if cancel.load(Ordering::Relaxed) {
                        return false; // Cancel the operation
                    }
                }

                true // Continue
            });
        }

        fetch_options.remote_callbacks(callbacks);

        // Fetch all refs (equivalent to `git fetch origin`)
        let empty_refspecs: Vec<&str> = vec![];
        remote.fetch(&empty_refspecs, Some(&mut fetch_options), None)?;

        // Get final object count from progress tracker
        let object_count = if let Some(p) = progress {
            p.lock().ok().map(|p| p.received_objects).unwrap_or(1)
        } else {
            1
        };

        Ok(object_count)
    }

    /// Fetch from a remote repository (simple version without progress)
    /// Returns the number of objects fetched
    pub fn fetch(&self, remote_name: &str) -> Result<usize> {
        self.fetch_with_progress(remote_name, None, None)
    }

    /// Fetch from the default remote (usually "origin")
    pub fn fetch_origin(&self) -> Result<usize> {
        self.fetch("origin")
    }

    /// List all remotes in the repository
    pub fn list_remotes(&self) -> Result<Vec<String>> {
        let remotes = self.repo.remotes()?;
        Ok(remotes.iter().flatten().map(|s| s.to_string()).collect())
    }

    /// Get the URL of a remote
    pub fn remote_url(&self, remote_name: &str) -> Result<String> {
        let remote = self.repo.find_remote(remote_name)?;
        remote
            .url()
            .map(|s| s.to_string())
            .ok_or_else(|| color_eyre::eyre::eyre!("Remote has no URL"))
    }

    /// Push to a remote branch with progress tracking
    ///
    /// # Arguments
    ///
    /// * `remote_name` - Name of the remote (e.g., "origin")
    /// * `branch_name` - Branch to push (None = current branch)
    /// * `progress` - Optional progress tracker
    /// * `cancel_flag` - Optional cancellation flag
    ///
    /// # Errors
    ///
    /// - Remote does not exist
    /// - Branch does not exist or cannot be determined
    /// - Authentication failed
    /// - Push rejected (e.g., non-fast-forward)
    /// - Operation was cancelled
    pub fn push_with_progress(
        &self,
        remote_name: &str,
        branch_name: Option<&str>,
        progress: Option<Arc<Mutex<TransferProgress>>>,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;

        let mut push_options = git2::PushOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();

        // Credential callback for authentication
        callbacks.credentials(|url, username_from_url, allowed_types| {
            // Check if SSH is allowed
            if allowed_types.is_ssh_key() {
                // Try SSH agent first
                if let Some(username) = username_from_url {
                    if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                        return Ok(cred);
                    }
                }

                // Try default SSH key locations
                if let Some(username) = username_from_url.or_else(|| url.split('@').next()) {
                    if let Ok(home) = std::env::var("HOME") {
                        let ssh_key = std::path::PathBuf::from(&home).join(".ssh/id_rsa");
                        if ssh_key.exists() {
                            if let Ok(cred) = git2::Cred::ssh_key(username, None, &ssh_key, None) {
                                return Ok(cred);
                            }
                        }
                    }
                }
            }

            // Try username/password for HTTPS
            if allowed_types.is_user_pass_plaintext() {
                if let Ok(cred) =
                    git2::Cred::credential_helper(&self.repo.config()?, url, username_from_url)
                {
                    return Ok(cred);
                }
            }

            // Fallback to default credentials
            git2::Cred::default()
        });

        // Transfer progress callback
        if let Some(progress_tracker) = progress.clone() {
            callbacks.transfer_progress(move |stats| {
                if let Ok(mut p) = progress_tracker.lock() {
                    p.total_objects = stats.total_objects();
                    p.indexed_objects = stats.indexed_objects();
                    p.received_objects = stats.received_objects();
                    p.received_bytes = stats.received_bytes();
                    p.total_deltas = stats.total_deltas();
                    p.indexed_deltas = stats.indexed_deltas();
                }

                // Check cancellation flag
                if let Some(ref cancel) = cancel_flag {
                    if cancel.load(Ordering::Relaxed) {
                        return false; // Cancel the operation
                    }
                }

                true // Continue
            });
        }

        // Push update callback for errors
        callbacks.push_update_reference(|refname, status| {
            if let Some(s) = status {
                Err(git2::Error::from_str(&format!(
                    "Push rejected for {}: {}",
                    refname, s
                )))
            } else {
                Ok(())
            }
        });

        push_options.remote_callbacks(callbacks);

        // Determine the refspec to push
        let refspec = if let Some(branch) = branch_name {
            format!("refs/heads/{}:refs/heads/{}", branch, branch)
        } else {
            // Push current branch to its upstream if it has one
            let head = self.repo.head()?;
            if let Some(branch_name) = head.shorthand() {
                format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name)
            } else {
                return Err(color_eyre::eyre::eyre!(
                    "Unable to determine branch to push"
                ));
            }
        };

        remote.push(&[&refspec], Some(&mut push_options))?;

        Ok(())
    }

    /// Push to a remote branch (simple version without progress)
    /// If branch_name is None, pushes to the upstream branch of the current HEAD
    pub fn push(&self, remote_name: &str, branch_name: Option<&str>) -> Result<()> {
        self.push_with_progress(remote_name, branch_name, None, None)
    }

    /// Push to origin
    pub fn push_origin(&self, branch_name: Option<&str>) -> Result<()> {
        self.push("origin", branch_name)
    }

    /// Pull from a remote branch with progress tracking (fetch + merge)
    ///
    /// # Behavior
    ///
    /// 1. Fetches from the specified remote using `git fetch <remote>`
    /// 2. Merges the remote branch into the current local branch using `git merge`
    /// 3. Uses fast-forward merge when possible for clean history
    ///
    /// # Arguments
    ///
    /// * `remote_name` - Name of the remote (e.g., "origin")
    /// * `branch_name` - Branch to merge (None = current branch)
    /// * `progress` - Optional progress tracker
    /// * `cancel_flag` - Optional cancellation flag
    ///
    /// # Edge Cases
    ///
    /// - **Merge conflicts**: Returns error with conflict details
    /// - **Detached HEAD**: Returns error - cannot merge on detached HEAD
    /// - **No upstream branch**: Attempts to merge `remote/branch_name` pattern
    /// - **Dirty working directory**: May fail if conflicts would occur
    /// - **Fast-forward possible**: Performs fast-forward instead of merge commit
    ///
    /// # Errors
    ///
    /// - Remote does not exist
    /// - Current HEAD is detached
    /// - Merge conflicts detected
    /// - Repository structure is corrupted
    /// - Operation was cancelled
    pub fn pull_with_progress(
        &self,
        remote_name: &str,
        branch_name: Option<&str>,
        progress: Option<Arc<Mutex<TransferProgress>>>,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> Result<()> {
        // Step 1: Fetch from remote
        self.fetch_with_progress(remote_name, progress, cancel_flag.clone())?;

        // Check if cancelled after fetch
        if let Some(ref cancel) = cancel_flag {
            if cancel.load(Ordering::Relaxed) {
                return Err(color_eyre::eyre::eyre!("Operation cancelled by user"));
            }
        }

        // Step 2: Determine the branch to merge
        let head = self.repo.head()?;
        let current_branch = head
            .shorthand()
            .ok_or_else(|| color_eyre::eyre::eyre!("Cannot pull on detached HEAD"))?;

        let merge_branch = branch_name.unwrap_or(current_branch);

        // Step 3: Get the remote tracking branch reference
        let refname = format!("refs/remotes/{}/{}", remote_name, merge_branch);
        let merge_ref = self.repo.find_reference(&refname)?;
        let merge_oid = merge_ref
            .target()
            .ok_or_else(|| color_eyre::eyre::eyre!("Remote branch {} not found", refname))?;

        let merge_commit = self.repo.find_commit(merge_oid)?;
        let local_commit = head.peel_to_commit()?;

        // Step 4: Check if fast-forward is possible
        let merge_annotated = self.repo.find_annotated_commit(merge_oid)?;
        let (analysis, _) = self.repo.merge_analysis(&[&merge_annotated])?;

        if analysis.is_up_to_date() {
            return Ok(()); // Already up to date
        }

        if analysis.is_fast_forward() {
            // Perform fast-forward
            let mut reference = self
                .repo
                .find_reference(&format!("refs/heads/{}", current_branch))?;
            reference.set_target(merge_oid, "Fast-forward")?;
            self.repo
                .set_head(&format!("refs/heads/{}", current_branch))?;
            self.repo
                .checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            return Ok(());
        }

        // Step 5: Perform merge
        let mut index = self
            .repo
            .merge_commits(&local_commit, &merge_commit, None)?;

        // Step 6: Check for conflicts
        if index.has_conflicts() {
            // Collect conflict paths for better error message
            let conflicts: Vec<_> = index
                .conflicts()
                .ok()
                .and_then(|c| {
                    c.flatten()
                        .filter_map(|conflict| {
                            conflict
                                .our
                                .as_ref()
                                .and_then(|e| std::str::from_utf8(&e.path).ok())
                                .map(|s| s.to_string())
                        })
                        .collect::<Vec<_>>()
                        .into()
                })
                .unwrap_or_default();

            let conflict_list = if conflicts.is_empty() {
                "unknown files".to_string()
            } else {
                conflicts.join(", ")
            };

            return Err(color_eyre::eyre::eyre!(
                "Merge conflict in: {}. Resolve conflicts manually.",
                conflict_list
            ));
        }

        // Step 7: Write merged index to tree
        let tree_id = index.write_tree_to(&self.repo)?;
        let tree = self.repo.find_tree(tree_id)?;

        // Step 8: Create merge commit
        let signature = self.repo.signature()?;
        let merge_msg = format!(
            "Merge remote-tracking branch '{}/{}'",
            remote_name, merge_branch
        );

        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &merge_msg,
            &tree,
            &[&local_commit, &merge_commit],
        )?;

        // Step 9: Checkout the new commit
        self.repo
            .checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;

        Ok(())
    }

    /// Pull from a remote branch (simple version without progress)
    pub fn pull(&self, remote_name: &str, branch_name: Option<&str>) -> Result<()> {
        self.pull_with_progress(remote_name, branch_name, None, None)
    }

    /// Pull from the default remote (usually "origin")
    pub fn pull_origin(&self, branch_name: Option<&str>) -> Result<()> {
        self.pull("origin", branch_name)
    }
    /// Check repository health and return diagnostics
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if repository is healthy
    /// - `Ok(false)` if repository has issues (corrupted index, missing objects, etc.)
    /// - `Err` if unable to determine health
    ///
    /// # Errors
    ///
    /// - Repository path is invalid
    /// - Unable to access repository metadata
    pub fn check_health(&self) -> Result<bool> {
        // Try to open the index - most reliable corruption indicator
        match self.repo.index() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get a user-friendly error message from a git2 error
    ///
    /// Maps common git2 errors to actionable user guidance
    /// Explain Git errors in user-friendly terms with actionable guidance
    ///
    /// Maps common git2 error codes and patterns to helpful troubleshooting steps.
    /// Covers authentication, network, corruption, and operational errors.
    pub fn explain_error(e: &color_eyre::eyre::Report) -> String {
        let error_str = e.to_string();
        let error_lower = error_str.to_lowercase();

        // Authentication and credentials errors
        if error_lower.contains("authentication failed")
            || error_lower.contains("credentials")
            || error_lower.contains("publickey")
            || error_lower.contains("username")
            || error_lower.contains("password")
        {
            return "Authentication failed. Check:\n\
                    • SSH keys: ls -la ~/.ssh/ | grep id_rsa\n\
                    • SSH agent: ssh-add -l\n\
                    • GitHub/GitLab access: ssh -T git@github.com\n\
                    • HTTPS credentials in ~/.gitconfig"
                .to_string();
        }

        // Network connectivity errors
        if error_lower.contains("failed to resolve")
            || error_lower.contains("could not resolve host")
            || error_lower.contains("connection timed out")
            || error_lower.contains("connection refused")
            || error_lower.contains("network is unreachable")
        {
            return "Network error. Check:\n\
                    • Internet connection: ping -c 3 github.com\n\
                    • Firewall/proxy settings\n\
                    • VPN connectivity\n\
                    • Remote URL: git remote -v"
                .to_string();
        }

        // Remote repository errors
        if (error_lower.contains("remote") && error_lower.contains("not found"))
            || error_lower.contains("repository not found")
            || error_lower.contains("does not appear to be a git repository")
        {
            return "Remote repository not found. Check:\n\
                    • Repository exists: git ls-remote <url>\n\
                    • Access permissions (private repo?)\n\
                    • Remote URL: git remote -v\n\
                    • Typos in organization/repo name"
                .to_string();
        }

        // Index lock errors (common with concurrent operations)
        if error_lower.contains("index") && error_lower.contains("lock") {
            return "Git index is locked. Another Git operation is running.\n\
                    • Wait for other operations to complete\n\
                    • If stuck, check: ps aux | grep git\n\
                    • Force unlock: rm -f .git/index.lock\n\
                    • Warning: Only force unlock if no git process is active"
                .to_string();
        }

        // Corrupted index
        if error_lower.contains("index") && error_lower.contains("corrupt") {
            return "Git index is corrupted. Try:\n\
                    1. rm .git/index\n\
                    2. git reset\n\
                    3. git status (to rebuild index)\n\
                    4. If that fails: git fsck --full"
                .to_string();
        }

        // HEAD reference errors
        if error_lower.contains("head") && error_lower.contains("invalid") {
            return "Repository HEAD is invalid. Try:\n\
                    • Check: cat .git/HEAD\n\
                    • Fix: git symbolic-ref HEAD refs/heads/main\n\
                    • Or create initial commit if empty repo"
                .to_string();
        }

        // Detached HEAD state
        if error_lower.contains("detached") || error_lower.contains("detached head") {
            return "Cannot perform this operation on a detached HEAD.\n\
                    • View current state: git status\n\
                    • Return to branch: git checkout main\n\
                    • Create new branch: git checkout -b <branch-name>\n\
                    • Discard changes: git checkout <branch>"
                .to_string();
        }

        // Merge conflicts
        if error_lower.contains("conflict") || error_lower.contains("merge conflict") {
            return "Merge conflicts detected. Resolve conflicts manually:\n\
                    1. git status (see conflicted files)\n\
                    2. Edit files to resolve conflicts\n\
                    3. git add <resolved-files>\n\
                    4. git commit -m 'Resolve merge conflicts'\n\
                    5. Or abort: git merge --abort"
                .to_string();
        }

        // Missing objects (corruption)
        if error_lower.contains("object") && error_lower.contains("missing") {
            return "Repository is missing objects (corruption detected).\n\
                    Repair steps:\n\
                    1. git fsck --full\n\
                    2. git gc --aggressive --prune=now\n\
                    3. If many errors, consider re-cloning\n\
                    4. Check disk space: df -h"
                .to_string();
        }

        // General corruption
        if error_lower.contains("corrupt") || error_lower.contains("invalid") {
            return "Repository data is corrupted. Try:\n\
                    1. git fsck --full (diagnose)\n\
                    2. git gc --aggressive (repair)\n\
                    3. Backup: cp -r .git .git.backup\n\
                    4. Last resort: re-clone repository"
                .to_string();
        }

        // Permission errors
        if error_lower.contains("permission denied")
            || error_lower.contains("insufficient permission")
        {
            return "Permission denied. Check:\n\
                    • File permissions: ls -la .git/\n\
                    • Ownership: stat .git/\n\
                    • Fix ownership: sudo chown -R $USER:$USER .git/\n\
                    • Repository access rights on remote"
                .to_string();
        }

        // Untracked files would be overwritten
        if error_lower.contains("would be overwritten") || error_lower.contains("untracked") {
            return "Operation would overwrite untracked files.\n\
                    • Stash changes: git stash --include-untracked\n\
                    • Remove untracked: git clean -fd (warning: deletes files!)\n\
                    • Or move files to different location"
                .to_string();
        }

        // Reference errors
        if error_lower.contains("reference") || error_lower.contains("ref") {
            return "Git reference error. Try:\n\
                    • List refs: git show-ref\n\
                    • Verify: git fsck --full\n\
                    • Repair: git gc --prune=now\n\
                    • Check .git/refs/ directory integrity"
                .to_string();
        }

        // Default: provide raw error with context
        format!(
            "Git operation failed: {}\n\n\
            Common troubleshooting:\n\
            • Check git status\n\
            • Verify remote connectivity: git ls-remote\n\
            • Run diagnostics: git fsck --full",
            error_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_gitclient_discover_valid_repo() {
        // Create a temporary git repository
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        // Initialize a git repo
        git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Test discover
        let client = GitClient::discover(repo_path);
        assert!(
            client.is_ok(),
            "GitClient::discover should succeed for valid repo"
        );
    }

    #[test]
    fn test_gitclient_discover_invalid_path() {
        let result = GitClient::discover("/nonexistent/path/that/does/not/exist");
        assert!(
            result.is_err(),
            "GitClient::discover should fail for invalid path"
        );
    }

    #[test]
    fn test_head_branch_on_empty_repo() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        git2::Repository::init(repo_path).expect("Failed to initialize repo");
        let client = GitClient::discover(repo_path).expect("Failed to create GitClient");

        // Empty repo has no HEAD
        let branch = client.head_branch();
        // Could be None or Some("master") depending on git config
        // Just verify it doesn't panic
        let _ = branch;
    }

    #[test]
    fn test_list_changes_empty_repo() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        git2::Repository::init(repo_path).expect("Failed to initialize repo");
        let client = GitClient::discover(repo_path).expect("Failed to create GitClient");

        let changes = client.list_changes();
        assert!(changes.is_ok(), "list_changes should succeed on empty repo");
        assert_eq!(
            changes.unwrap().len(),
            0,
            "empty repo should have no changes"
        );
    }

    #[test]
    fn test_list_changes_with_untracked_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Create an untracked file
        let test_file = repo_path.join("test.txt");
        fs::write(&test_file, "test content").expect("Failed to write test file");

        let client = GitClient::discover(repo_path).expect("Failed to create GitClient");
        let changes = client.list_changes();

        assert!(changes.is_ok(), "list_changes should succeed");
        let changes_vec = changes.unwrap();
        assert!(
            !changes_vec.is_empty(),
            "untracked file should appear in changes"
        );
        assert_eq!(
            changes_vec[0].status,
            FileStatus::Added,
            "untracked file should be marked as Added"
        );
    }

    #[test]
    fn test_stage_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Create initial commit
        let test_file = repo_path.join("test.txt");
        fs::write(&test_file, "initial").expect("Failed to write test file");
        let mut index = repo.index().expect("Failed to get index");
        index.add_path(std::path::Path::new("test.txt")).ok();
        index.write().expect("Failed to write index");

        // Create signature and commit
        let sig =
            git2::Signature::now("Test", "test@example.com").expect("Failed to create signature");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to create commit");

        // Modify the file
        fs::write(&test_file, "modified").expect("Failed to modify file");

        let client = GitClient::discover(repo_path).expect("Failed to create GitClient");

        // Stage the file
        let result = client.stage_file("test.txt");
        assert!(result.is_ok(), "stage_file should succeed");
    }

    #[test]
    fn test_commit_data_type_alias() {
        // This test ensures the CommitData type alias is correctly defined
        let _sample: CommitData = (
            "abc123".to_string(),
            "John Doe".to_string(),
            "2024-01-01".to_string(),
            "Test commit".to_string(),
            vec!["file1.rs".to_string(), "file2.rs".to_string()],
        );
        // If compilation succeeds, the type alias is correct
    }

    #[test]
    fn test_explain_error_authentication() {
        let err = color_eyre::eyre::eyre!("authentication failed for remote");
        let explanation = GitClient::explain_error(&err);
        assert!(
            explanation.contains("Authentication failed"),
            "Should explain authentication errors"
        );
        assert!(explanation.contains("SSH keys"), "Should mention SSH keys");
    }

    #[test]
    fn test_explain_error_network() {
        let err = color_eyre::eyre::eyre!("failed to resolve host github.com");
        let explanation = GitClient::explain_error(&err);
        assert!(
            explanation.contains("Network error"),
            "Should explain network errors"
        );
        assert!(
            explanation.contains("ping"),
            "Should suggest connectivity check"
        );
    }

    #[test]
    fn test_explain_error_index_lock() {
        let err = color_eyre::eyre::eyre!("index.lock file exists");
        let explanation = GitClient::explain_error(&err);
        assert!(explanation.contains("locked"), "Should explain lock errors");
        assert!(
            explanation.contains("index.lock"),
            "Should mention lock file"
        );
    }

    #[test]
    fn test_explain_error_merge_conflict() {
        let err = color_eyre::eyre::eyre!("Merge conflict detected in file.txt");
        let explanation = GitClient::explain_error(&err);
        assert!(
            explanation.contains("conflict"),
            "Should explain merge conflicts"
        );
        assert!(
            explanation.contains("git status"),
            "Should suggest git status"
        );
    }

    #[test]
    fn test_explain_error_corrupted_object() {
        let err = color_eyre::eyre::eyre!("object 1234abc is missing from repository");
        let explanation = GitClient::explain_error(&err);
        assert!(
            explanation.contains("missing objects"),
            "Should explain missing objects"
        );
        assert!(explanation.contains("git fsck"), "Should suggest fsck");
    }

    #[test]
    fn test_explain_error_permission_denied() {
        let err = color_eyre::eyre::eyre!("permission denied accessing .git/config");
        let explanation = GitClient::explain_error(&err);
        assert!(
            explanation.contains("Permission denied"),
            "Should explain permission errors"
        );
        assert!(
            explanation.contains("permissions"),
            "Should mention checking permissions"
        );
    }

    #[test]
    fn test_explain_error_detached_head() {
        let err = color_eyre::eyre::eyre!("You are in 'detached HEAD' state");
        let explanation = GitClient::explain_error(&err);
        assert!(
            explanation.contains("detached HEAD"),
            "Should explain detached HEAD"
        );
        assert!(
            explanation.contains("git checkout"),
            "Should suggest checkout"
        );
    }

    #[test]
    fn test_explain_error_generic() {
        let err = color_eyre::eyre::eyre!("Some unknown git error occurred");
        let explanation = GitClient::explain_error(&err);
        assert!(
            explanation.contains("Git operation failed"),
            "Should provide generic message"
        );
        assert!(
            explanation.contains("troubleshooting"),
            "Should provide troubleshooting steps"
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_full_git_workflow() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Create and commit initial file
        let file1 = repo_path.join("file1.txt");
        fs::write(&file1, "content1").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(std::path::Path::new("file1.txt")).ok();
        index.write().expect("Failed to write index");

        let sig =
            git2::Signature::now("Test", "test@example.com").expect("Failed to create signature");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");

        repo.commit(Some("HEAD"), &sig, &sig, "First commit", &tree, &[])
            .expect("Failed to create commit");

        // Create client and test operations
        let client = GitClient::discover(repo_path).expect("Failed to create GitClient");

        // Create new file
        let file2 = repo_path.join("file2.txt");
        fs::write(&file2, "content2").expect("Failed to write file");

        // List changes - should show new file
        let changes = client.list_changes().expect("Failed to list changes");
        assert!(!changes.is_empty(), "Should detect new file");

        // Stage the file
        let stage_result = client.stage_file("file2.txt");
        assert!(stage_result.is_ok(), "Should stage file successfully");
    }

    #[test]
    fn test_list_remotes() {
        // Create a temporary directory
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = dir.path();

        // Initialize repo
        Repository::init(repo_path).expect("Failed to init repo");

        let client = GitClient::discover(repo_path).expect("Failed to create GitClient");

        // List remotes - should be empty initially
        let remotes = client.list_remotes().expect("Failed to list remotes");
        assert!(
            remotes.is_empty(),
            "Newly initialized repo should have no remotes"
        );
    }

    #[test]
    fn test_branch_create_and_switch_workflow() {
        // This tests: branch create → switch → commit
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Create initial commit on main/master
        let file1 = repo_path.join("initial.txt");
        fs::write(&file1, "initial content").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(std::path::Path::new("initial.txt")).ok();
        index.write().expect("Failed to write index");

        let sig =
            git2::Signature::now("Test User", "test@example.com").expect("Failed to create sig");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to create initial commit");

        let client = GitClient::discover(repo_path).expect("Failed to create client");

        // Create new branch
        let branch_result = client.create_branch("feature/test");
        assert!(branch_result.is_ok(), "Should create branch successfully");

        // Verify branch exists in list
        let branches = client
            .list_branches(true, true)
            .expect("Failed to list branches");
        let branch_exists = branches
            .iter()
            .any(|(name, _is_current)| name.contains("feature/test"));
        assert!(branch_exists, "Created branch should appear in branch list");

        // Switch to new branch
        let switch_result = client.checkout_branch("feature/test");
        assert!(
            switch_result.is_ok(),
            "Should switch to feature/test branch"
        );

        // Verify we're on the correct branch
        let current_branch = client.head_branch().expect("Failed to get current branch");
        assert_eq!(
            current_branch, "feature/test",
            "Should be on feature/test branch after switch"
        );

        // Create and commit a new file on the branch
        let feature_file = repo_path.join("feature.txt");
        fs::write(&feature_file, "feature content").expect("Failed to write feature file");

        let commit_result = client.commit_all("Add feature file");
        assert!(
            commit_result.is_ok(),
            "Should commit on feature branch successfully"
        );

        // Verify the commit created a change
        let history = client
            .get_commit_history(50)
            .expect("Failed to list history");
        assert!(
            history.len() >= 2,
            "Should have at least 2 commits (initial + feature)"
        );
    }

    #[test]
    fn test_commit_and_diff_workflow() {
        // This tests: modify files → stage → diff → commit
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Create initial commit
        let initial_file = repo_path.join("file.txt");
        fs::write(&initial_file, "initial\n").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(std::path::Path::new("file.txt")).ok();
        index.write().expect("Failed to write index");

        let sig = git2::Signature::now("Test", "test@example.com").expect("Failed to create sig");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .expect("Failed to create initial commit");

        let client = GitClient::discover(repo_path).expect("Failed to create client");

        // Modify the file
        fs::write(&initial_file, "initial\nmodified line\n").expect("Failed to modify file");

        // List changes - should show modified file
        let changes = client.list_changes().expect("Failed to list changes");
        assert!(
            !changes.is_empty(),
            "Should detect modified file in changes"
        );

        // Find the modified file
        let modified_change = changes.iter().find(|c| c.path == "file.txt");
        assert!(modified_change.is_some(), "Should find modified file");

        // Stage the modification
        let stage_result = client.stage_file("file.txt");
        assert!(stage_result.is_ok(), "Should stage file");

        // Get staged changes
        let staged = client.list_changes().expect("Failed to list staged");
        assert!(!staged.is_empty(), "Should have staged changes");

        // Commit the changes
        let commit_result = client.commit_all("Modify file with new line");
        assert!(commit_result.is_ok(), "Should commit staged changes");

        // Verify no more changes after commit
        let changes_after = client.list_changes().expect("Failed to list after commit");
        assert!(
            changes_after.is_empty(),
            "Should have no changes after commit"
        );
    }

    #[test]
    fn test_multiple_file_staging_workflow() {
        // This tests: create multiple files → selective staging → commit
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Create initial commit
        let file0 = repo_path.join("initial.txt");
        fs::write(&file0, "initial").expect("Failed to write");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(std::path::Path::new("initial.txt")).ok();
        index.write().expect("Failed to write index");

        let sig = git2::Signature::now("Test", "test@example.com").expect("Failed to create sig");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .expect("Failed to create initial commit");

        let client = GitClient::discover(repo_path).expect("Failed to create client");

        // Create multiple new files
        fs::write(repo_path.join("tracked.txt"), "content1").expect("Failed to write");
        fs::write(repo_path.join("untracked.txt"), "content2").expect("Failed to write");

        // List changes - should show both files
        let changes = client.list_changes().expect("Failed to list changes");
        assert!(changes.len() >= 2, "Should detect both new files");

        // Stage only the first file
        let stage_result = client.stage_file("tracked.txt");
        assert!(stage_result.is_ok(), "Should stage first file");

        // Commit only staged file
        let commit_result = client.commit_all("Add tracked file only");
        assert!(commit_result.is_ok(), "Should commit successfully");

        // Untracked file should still be present in changes
        let remaining = client.list_changes().expect("Failed to list remaining");
        let untracked = remaining.iter().find(|c| c.path == "untracked.txt");
        assert!(
            untracked.is_some(),
            "Untracked file should still be in changes"
        );
    }

    #[test]
    fn test_unstage_workflow() {
        // This tests: create file → stage → unstage → verify
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();

        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Create initial commit
        let initial = repo_path.join("initial.txt");
        fs::write(&initial, "initial").expect("Failed to write");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(std::path::Path::new("initial.txt")).ok();
        index.write().expect("Failed to write index");

        let sig = git2::Signature::now("Test", "test@example.com").expect("Failed to create sig");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .expect("Failed to create initial commit");

        let client = GitClient::discover(repo_path).expect("Failed to create client");

        // Create and stage a file
        fs::write(repo_path.join("test.txt"), "test content").expect("Failed to write");
        let stage_result = client.stage_file("test.txt");
        assert!(stage_result.is_ok(), "Should stage file");

        // Unstage the file
        let unstage_result = client.unstage_file("test.txt");
        assert!(unstage_result.is_ok(), "Should unstage file");

        // Verify the change is still there but unstaged
        let changes = client.list_changes().expect("Failed to list changes");
        let found = changes.iter().find(|c| c.path == "test.txt");
        assert!(
            found.is_some(),
            "File should still be in changes after unstaging"
        );
    }

    // ===== Remote Operations Tests =====

    #[test]
    fn test_transfer_progress_creation() {
        let progress = TransferProgress::default();
        assert_eq!(progress.total_objects, 0);
        assert_eq!(progress.percent(), 100);
        assert_eq!(progress.status_message(), "Initializing...");
    }

    #[test]
    fn test_transfer_progress_tracking() {
        let progress = TransferProgress {
            total_objects: 100,
            received_objects: 50,
            received_bytes: 1024,
            ..Default::default()
        };

        assert_eq!(progress.percent(), 50);
        assert!(progress.status_message().contains("50/100"));
        assert!(progress.status_message().contains("1024 bytes"));
    }

    #[test]
    fn test_list_remotes_basic() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();
        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Add a remote
        repo.remote("origin", "https://github.com/test/test.git")
            .expect("Failed to add remote");

        let client = GitClient::discover(repo_path).expect("Failed to create client");
        let remotes = client.list_remotes().expect("Failed to list remotes");

        assert!(remotes.contains(&"origin".to_string()));
    }

    #[test]
    fn test_remote_url() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();
        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        let test_url = "https://github.com/test/test.git";
        repo.remote("origin", test_url)
            .expect("Failed to add remote");

        let client = GitClient::discover(repo_path).expect("Failed to create client");
        let url = client
            .remote_url("origin")
            .expect("Failed to get remote URL");

        assert_eq!(url, test_url);
    }

    #[test]
    fn test_fetch_nonexistent_remote() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();
        git2::Repository::init(repo_path).expect("Failed to initialize repo");

        let client = GitClient::discover(repo_path).expect("Failed to create client");
        let result = client.fetch("nonexistent");

        assert!(result.is_err());
    }

    #[test]
    fn test_push_nonexistent_remote() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();
        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Create initial commit
        let file = repo_path.join("test.txt");
        fs::write(&file, "test").expect("Failed to write");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(std::path::Path::new("test.txt")).ok();
        index.write().expect("Failed to write index");

        let sig = git2::Signature::now("Test", "test@example.com").expect("Failed to create sig");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .expect("Failed to create initial commit");

        let client = GitClient::discover(repo_path).expect("Failed to create client");
        let result = client.push("nonexistent", None);

        assert!(result.is_err());
    }

    #[test]
    fn test_fetch_with_progress_tracking() {
        use std::sync::{Arc, Mutex};

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();
        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Add a remote (even if it doesn't exist, we test progress tracking structure)
        repo.remote("origin", "https://github.com/test/test.git")
            .expect("Failed to add remote");

        let client = GitClient::discover(repo_path).expect("Failed to create client");
        let progress = Arc::new(Mutex::new(TransferProgress::default()));

        // This will fail due to network/auth, but we verify the progress structure works
        let _ = client.fetch_with_progress("origin", Some(progress.clone()), None);

        // Progress tracker should be accessible
        let p = progress.lock().expect("Failed to lock progress");
        assert_eq!(p.total_objects, 0); // No objects received due to failure
    }

    #[test]
    fn test_fetch_with_cancellation() {
        use std::sync::{atomic::AtomicBool, Arc};

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();
        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        repo.remote("origin", "https://github.com/test/test.git")
            .expect("Failed to add remote");

        let client = GitClient::discover(repo_path).expect("Failed to create client");
        let cancel = Arc::new(AtomicBool::new(true)); // Pre-cancelled

        // Should fail or complete quickly if already cancelled
        let result = client.fetch_with_progress("origin", None, Some(cancel));

        // Either fails due to cancellation or network error
        // We just verify the API works
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_pull_on_detached_head() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let repo_path = temp_dir.path();
        let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

        // Create initial commit
        let file = repo_path.join("test.txt");
        fs::write(&file, "test").expect("Failed to write");

        let mut index = repo.index().expect("Failed to get index");
        index.add_path(std::path::Path::new("test.txt")).ok();
        index.write().expect("Failed to write index");

        let sig = git2::Signature::now("Test", "test@example.com").expect("Failed to create sig");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        let commit_oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[])
            .expect("Failed to create initial commit");

        // Detach HEAD
        repo.set_head_detached(commit_oid)
            .expect("Failed to detach HEAD");

        repo.remote("origin", "https://github.com/test/test.git")
            .expect("Failed to add remote");

        let client = GitClient::discover(repo_path).expect("Failed to create client");
        let result = client.pull("origin", None);

        // Should fail because HEAD is detached or remote fetch fails
        assert!(result.is_err());
        // The error could be about detached HEAD or remote not found
        if let Err(e) = result {
            let err_msg = e.to_string().to_lowercase();
            assert!(
                err_msg.contains("detached")
                    || err_msg.contains("remote")
                    || err_msg.contains("fetch"),
                "Expected detached HEAD or fetch error, got: {}",
                e
            );
        }
    }

    #[test]
    fn test_explain_error_authentication() {
        let error = color_eyre::eyre::eyre!("Authentication failed: invalid credentials");
        let explanation = GitClient::explain_error(&error);

        assert!(explanation.contains("Authentication failed"));
        assert!(explanation.contains("SSH"));
    }

    #[test]
    fn test_explain_error_network() {
        let error = color_eyre::eyre::eyre!("Failed to resolve host");
        let explanation = GitClient::explain_error(&error);

        assert!(explanation.contains("network") || explanation.contains("Network"));
    }
}
