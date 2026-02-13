mod common;

use forge::{FileStatus, GitClient};

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
