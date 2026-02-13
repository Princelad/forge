use std::error::Error;
use std::fs;
use std::path::Path;

use git2::{IndexAddOption, Repository, Signature};
use tempfile::TempDir;

pub type FixtureResult<T> = Result<T, Box<dyn Error>>;

pub struct RepoFixture {
    temp: TempDir,
    repo: Repository,
}

impl RepoFixture {
    pub fn new() -> FixtureResult<Self> {
        let temp = TempDir::new()?;
        let repo = Repository::init(temp.path())?;
        Ok(Self { temp, repo })
    }

    pub fn path(&self) -> &Path {
        self.temp.path()
    }

    pub fn repo(&self) -> &Repository {
        &self.repo
    }

    pub fn write_file(&self, relative: impl AsRef<Path>, contents: &str) -> std::io::Result<()> {
        let path = self.temp.path().join(relative.as_ref());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }

    pub fn add_all(&self) -> FixtureResult<()> {
        let mut index = self.repo.index()?;
        index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    pub fn add_path(&self, relative: impl AsRef<Path>) -> FixtureResult<()> {
        let mut index = self.repo.index()?;
        index.add_path(relative.as_ref())?;
        index.write()?;
        Ok(())
    }

    pub fn commit(&self, message: &str) -> FixtureResult<git2::Oid> {
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let sig = Signature::now("Forge Test", "forge@example.com")?;

        let parents = match self.repo.head() {
            Ok(head) => vec![head.peel_to_commit()?],
            Err(_) => Vec::new(),
        };

        let oid = if parents.is_empty() {
            self.repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?
        } else {
            let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
            self.repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)?
        };

        Ok(oid)
    }

    pub fn commit_file(
        &self,
        relative: impl AsRef<Path>,
        contents: &str,
        message: &str,
    ) -> FixtureResult<git2::Oid> {
        self.write_file(relative.as_ref(), contents)?;
        self.add_path(relative)?;
        self.commit(message)
    }

    pub fn create_branch(&self, name: &str) -> FixtureResult<()> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        self.repo.branch(name, &commit, false)?;
        Ok(())
    }
}
