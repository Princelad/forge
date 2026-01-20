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
    pub fn discover(start: impl AsRef<Path>) -> Result<Self> {
        let repo = Repository::discover(start)?;
        let workdir = repo
            .workdir()
            .map(Path::to_path_buf)
            .or_else(|| repo.path().parent().map(Path::to_path_buf))
            .ok_or_else(|| color_eyre::eyre::eyre!("Unable to determine workdir"))?;
        Ok(Self { repo, workdir })
    }

    pub fn head_branch(&self) -> Option<String> {
        self.repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()))
    }

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
                .clone()
                .or_else(|| incoming_preview.clone())
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
}
