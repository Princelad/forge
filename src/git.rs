use std::path::{Path, PathBuf};

use color_eyre::eyre::Result;
use git2::{DiffFormat, DiffOptions, IndexAddOption, Repository, Signature, StatusOptions, Tree};

use crate::data::{Change, FileStatus};

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

        for oid in revwalk.take(100) {
            // Limit to last 100 commits
            if let Ok(oid) = oid {
                if let Ok(commit) = self.repo.find_commit(oid) {
                    let author = commit.author();
                    if let Some(name) = author.name() {
                        names.insert(name.to_string());
                    }
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
            for branch in branch_iter {
                if let Ok((branch, _)) = branch {
                    if let Some(name) = branch.name()? {
                        let is_current = name == current_branch;
                        branches.push((name.to_string(), is_current));
                    }
                }
            }
        }

        // List remote branches
        if remote {
            let branch_iter = self.repo.branches(Some(git2::BranchType::Remote))?;
            for branch in branch_iter {
                if let Ok((branch, _)) = branch {
                    if let Some(name) = branch.name()? {
                        branches.push((name.to_string(), false));
                    }
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

    /// Get commit history
    pub fn get_commit_history(
        &self,
        limit: usize,
    ) -> Result<Vec<(String, String, String, String, Vec<String>)>> {
        let mut commits = Vec::new();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        for oid in revwalk.take(limit) {
            if let Ok(oid) = oid {
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
        }

        Ok(commits)
    }
}
