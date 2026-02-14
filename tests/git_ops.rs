mod common;

use forge::{FileStatus, GitClient};
use git2::BranchType;
use std::fs;

use common::RepoFixture;

fn find_change<'a>(changes: &'a [forge::Change], path: &str) -> &'a forge::Change {
    changes
        .iter()
        .find(|change| change.path == path)
        .expect("change not found")
}

#[test]
fn status_reports_untracked_file() {
    let fixture = RepoFixture::new().expect("fixture init failed");
    fixture
        .write_file("foo.txt", "hello")
        .expect("write file failed");

    let client = GitClient::discover(fixture.path()).expect("discover failed");
    let changes = client.list_changes_summary().expect("list changes failed");

    let change = find_change(&changes, "foo.txt");
    assert_eq!(change.status, FileStatus::Added);
    assert!(!change.staged);
}

#[test]
fn stage_file_marks_change_staged() {
    let fixture = RepoFixture::new().expect("fixture init failed");
    fixture
        .write_file("foo.txt", "hello")
        .expect("write file failed");

    let client = GitClient::discover(fixture.path()).expect("discover failed");
    client.stage_file("foo.txt").expect("stage failed");

    let changes = client.list_changes_summary().expect("list changes failed");

    let change = find_change(&changes, "foo.txt");
    assert_eq!(change.status, FileStatus::Added);
    assert!(change.staged);
}

#[test]
fn status_reports_modified_file() {
    let fixture = RepoFixture::new().expect("fixture init failed");
    fixture
        .commit_file("foo.txt", "hello", "initial")
        .expect("commit file failed");
    fixture
        .write_file("foo.txt", "hello again")
        .expect("write file failed");

    let client = GitClient::discover(fixture.path()).expect("discover failed");
    let changes = client.list_changes_summary().expect("list changes failed");

    let change = find_change(&changes, "foo.txt");
    assert_eq!(change.status, FileStatus::Modified);
    assert!(!change.staged);
}

#[test]
fn commit_all_clears_changes() {
    let fixture = RepoFixture::new().expect("fixture init failed");
    fixture
        .write_file("foo.txt", "hello")
        .expect("write file failed");

    let client = GitClient::discover(fixture.path()).expect("discover failed");
    client.stage_file("foo.txt").expect("stage failed");
    client.commit_all("initial commit").expect("commit failed");

    let changes = client.list_changes_summary().expect("list changes failed");
    assert!(changes.is_empty());
}

#[test]
fn fixture_helpers_create_branch() {
    let fixture = RepoFixture::new().expect("fixture init failed");
    fixture
        .write_file("foo.txt", "hello")
        .expect("write file failed");
    fixture.add_all().expect("add all failed");
    fixture.commit("initial").expect("commit failed");
    fixture
        .create_branch("feature")
        .expect("create branch failed");

    let branch = fixture
        .repo()
        .find_branch("feature", BranchType::Local)
        .expect("branch missing");
    assert!(!branch.is_head());
}

#[test]
fn stash_apply_keeps_entry_and_restores_changes() {
    let fixture = RepoFixture::new().expect("fixture init failed");
    fixture
        .commit_file("foo.txt", "base", "initial")
        .expect("commit failed");
    fixture
        .write_file("foo.txt", "work in progress")
        .expect("write failed");

    let client = GitClient::discover(fixture.path()).expect("discover failed");
    client.create_stash("wip").expect("stash failed");

    let changes = client.list_changes_summary().expect("list changes failed");
    assert!(changes.is_empty());

    let stashes = client.list_stashes().expect("list stashes failed");
    assert_eq!(stashes.len(), 1);

    client.apply_stash(0).expect("apply failed");
    let changes = client.list_changes_summary().expect("list changes failed");
    assert_eq!(changes.len(), 1);

    let stashes = client.list_stashes().expect("list stashes failed");
    assert_eq!(stashes.len(), 1);
}

#[test]
fn stash_pop_drops_entry_and_restores_changes() {
    let fixture = RepoFixture::new().expect("fixture init failed");
    fixture
        .commit_file("foo.txt", "base", "initial")
        .expect("commit failed");
    fixture
        .write_file("foo.txt", "work in progress")
        .expect("write failed");

    let client = GitClient::discover(fixture.path()).expect("discover failed");
    client.create_stash("wip").expect("stash failed");

    client.pop_stash(0).expect("pop failed");
    let changes = client.list_changes_summary().expect("list changes failed");
    assert_eq!(changes.len(), 1);

    let stashes = client.list_stashes().expect("list stashes failed");
    assert!(stashes.is_empty());
}

#[test]
fn cherry_pick_applies_commit_on_current_branch() {
    let fixture = RepoFixture::new().expect("fixture init failed");
    fixture
        .commit_file("foo.txt", "base", "initial")
        .expect("commit failed");

    let client = GitClient::discover(fixture.path()).expect("discover failed");
    let repo = fixture.repo();
    let mut config = repo.config().expect("config failed");
    config
        .set_str("user.name", "Forge Test")
        .expect("config name failed");
    config
        .set_str("user.email", "forge@example.com")
        .expect("config email failed");

    let base_branch = client.head_branch().unwrap_or_else(|| "main".into());
    fixture.create_branch("feature").expect("branch failed");
    client.checkout_branch("feature").expect("checkout failed");

    fixture
        .write_file("foo.txt", "feature change")
        .expect("write failed");
    fixture.add_path("foo.txt").expect("add failed");
    let feature_oid = fixture.commit("feature change").expect("commit failed");

    client
        .checkout_branch(&base_branch)
        .expect("checkout back failed");

    client
        .cherry_pick_commit(&feature_oid.to_string())
        .expect("cherry-pick failed");

    let contents = fs::read_to_string(fixture.path().join("foo.txt")).expect("read failed");
    assert_eq!(contents, "feature change");
}
