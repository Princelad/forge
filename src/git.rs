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

            changes.push(Change {
                path,
                status: file_status,
                diff_preview,
                local_preview,
                incoming_preview,
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
}
