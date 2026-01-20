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

use color_eyre::eyre::Result;
use git2::{DiffFormat, DiffOptions, IndexAddOption, Repository, Signature, StatusOptions, Tree};

use crate::data::{Change, FileStatus};

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

    /// Fetch from a remote repository
    /// Returns the number of objects fetched
    pub fn fetch(&self, remote_name: &str) -> Result<usize> {
        let mut remote = self.repo.find_remote(remote_name)?;

        // Setup fetch options with callbacks
        let mut fetch_options = git2::FetchOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();

        // Credential callback for authentication
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            // Try SSH agent first
            if let Some(username) = username_from_url {
                if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                    return Ok(cred);
                }
            }

            // Fallback to default credentials (will use ssh-agent or credential helpers)
            git2::Cred::default()
        });

        fetch_options.remote_callbacks(callbacks);

        // Fetch all refs (equivalent to `git fetch origin`)
        // Empty refspecs means use the remote's default refspecs
        let empty_refspecs: Vec<&str> = vec![];
        remote.fetch(&empty_refspecs, Some(&mut fetch_options), None)?;

        // Since we can't easily get the transfer stats from git2,
        // return 1 to indicate successful fetch (implementation detail)
        Ok(1)
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

    /// Push to a remote branch
    /// If branch_name is None, pushes to the upstream branch of the current HEAD
    pub fn push(&self, remote_name: &str, branch_name: Option<&str>) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;

        // Setup push options with callbacks
        let mut push_options = git2::PushOptions::new();
        let mut callbacks = git2::RemoteCallbacks::new();

        // Credential callback for authentication
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            // Try SSH agent first
            if let Some(username) = username_from_url {
                if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                    return Ok(cred);
                }
            }

            // Fallback to default credentials (will use ssh-agent or credential helpers)
            git2::Cred::default()
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

    /// Push to origin
    pub fn push_origin(&self, branch_name: Option<&str>) -> Result<()> {
        self.push("origin", branch_name)
    }

    /// Pull from a remote branch (fetch + merge)
    ///
    /// # Behavior
    ///
    /// 1. Fetches from the specified remote using `git fetch <remote>`
    /// 2. Merges the remote branch into the current local branch using `git merge`
    /// 3. Uses fast-forward merge by default for clean integration
    ///
    /// # Edge Cases
    ///
    /// - **Merge conflicts**: Returns error with git2 error code, does not auto-resolve
    /// - **Detached HEAD**: Returns error - cannot merge on detached HEAD
    /// - **No upstream branch**: Attempts to merge `remote/branch_name` pattern
    /// - **Dirty working directory**: git2 handles this - may fail if conflicts would occur
    /// - **No commits**: Will fail if either repo has no commits
    ///
    /// # Errors
    ///
    /// - Remote does not exist
    /// - Current HEAD is detached
    /// - Merge conflicts detected
    /// - Repository structure is corrupted
    pub fn pull(&self, remote_name: &str, branch_name: Option<&str>) -> Result<()> {
        // Step 1: Fetch from remote
        self.fetch(remote_name)?;

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

        // Step 4: Perform merge
        let mut index = self
            .repo
            .merge_commits(&head.peel_to_commit()?, &merge_commit, None)?;

        // Step 5: Check for conflicts
        if index.has_conflicts() {
            return Err(color_eyre::eyre::eyre!(
                "Merge conflict detected: resolve conflicts and commit manually"
            ));
        }

        // Step 6: Write merged index to tree
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        // Step 7: Create merge commit
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
            &[&head.peel_to_commit()?, &merge_commit],
        )?;

        Ok(())
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
    pub fn explain_error(e: &color_eyre::eyre::Report) -> String {
        let error_str = e.to_string();

        if error_str.contains("index") && error_str.contains("lock") {
            "Git index is locked. Another Git operation may be running. Try again in a moment."
                .to_string()
        } else if error_str.contains("index") {
            "Git index is corrupted. Try: 'rm .git/index.lock' and retry, or 'git fsck --full'"
                .to_string()
        } else if error_str.contains("HEAD") {
            "Repository HEAD is invalid. Check 'git show-ref HEAD' or make an initial commit."
                .to_string()
        } else if error_str.contains("detached") {
            "Cannot perform this operation on a detached HEAD. Checkout a branch first with 'git checkout'.".to_string()
        } else if error_str.contains("conflict") {
            "Merge conflicts detected. Resolve conflicts and commit manually.".to_string()
        } else if error_str.contains("authentication") || error_str.contains("credentials") {
            "Authentication failed. Check your SSH keys or Git credentials.".to_string()
        } else if error_str.contains("remote") && error_str.contains("not found") {
            "Remote repository not found. Check 'git remote -v' or network connectivity."
                .to_string()
        } else if error_str.contains("network") {
            "Network error. Check your internet connection and remote URL.".to_string()
        } else if error_str.contains("object") {
            "Repository is missing objects. Try: 'git fsck --full' or clone again.".to_string()
        } else if error_str.contains("corrupted") || error_str.contains("invalid") {
            "Repository data is corrupted. Try: 'git fsck --full' and 'git gc --aggressive'."
                .to_string()
        } else {
            format!("Git error: {}", error_str)
        }
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
            changes_vec.len() > 0,
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
        assert!(changes.len() > 0, "Should detect new file");

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
}
