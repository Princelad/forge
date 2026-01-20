use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use forge::GitClient;
use std::cell::Cell;
use std::fs;
use tempfile::TempDir;

/// Create a test repository with the given number of commits
fn create_test_repo_with_commits(num_commits: usize) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    let repo = git2::Repository::init(repo_path).expect("Failed to initialize repo");

    // Set up git config for commits
    let mut config = repo.config().expect("Failed to get config");
    config.set_str("user.name", "Test User").ok();
    config.set_str("user.email", "test@example.com").ok();

    // Create initial commit
    let file = repo_path.join("file.txt");
    fs::write(&file, "initial").expect("Failed to write file");

    let mut index = repo.index().expect("Failed to get index");
    index.add_path(std::path::Path::new("file.txt")).ok();
    index.write().expect("Failed to write index");

    let sig = git2::Signature::now("Test User", "test@example.com").expect("Failed to create sig");
    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");

    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .expect("Failed to create initial commit");

    // Create additional commits
    for i in 1..num_commits {
        let content = format!("modification {}", i);
        fs::write(&file, &content).expect("Failed to write file");

        index = repo.index().expect("Failed to get index");
        index.add_path(std::path::Path::new("file.txt")).ok();
        index.write().expect("Failed to write index");

        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");

        let head = repo
            .head()
            .ok()
            .and_then(|h| h.target().and_then(|oid| repo.find_commit(oid).ok()));

        let parents = if let Some(commit) = head {
            vec![commit]
        } else {
            vec![]
        };

        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("Commit {}", i),
            &tree,
            &parent_refs,
        )
        .ok();
    }

    temp_dir
}

/// Benchmark repository discovery
fn bench_discover(c: &mut Criterion) {
    c.bench_function("discover_repo", |b| {
        let temp_dir = create_test_repo_with_commits(1);
        b.iter(|| GitClient::discover(black_box(temp_dir.path())));
    });
}

/// Benchmark getting head branch
fn bench_head_branch(c: &mut Criterion) {
    let temp_dir = create_test_repo_with_commits(1);
    let client = GitClient::discover(temp_dir.path()).expect("Failed to create client");

    c.bench_function("head_branch", |b| {
        b.iter(|| client.head_branch());
    });
}

/// Benchmark listing changes with varying file counts
fn bench_list_changes(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_changes");

    for file_count in [1, 10, 50].iter() {
        let temp_dir = create_test_repo_with_commits(1);
        let repo_path = temp_dir.path();

        // Create multiple files
        for i in 0..*file_count {
            let file = repo_path.join(format!("file{}.txt", i));
            fs::write(&file, format!("content {}", i)).expect("Failed to write file");
        }

        let client = GitClient::discover(repo_path).expect("Failed to create client");

        let error_count = Cell::new(0);
        group.bench_with_input(
            BenchmarkId::from_parameter(file_count),
            file_count,
            |b, _| {
                b.iter(|| {
                    if client.list_changes().is_err() {
                        error_count.set(error_count.get() + 1);
                    }
                });
            },
        );
        if error_count.get() > 0 {
            eprintln!(
                "Warning: list_changes had {} errors during benchmark",
                error_count.get()
            );
        }
    }
    group.finish();
}

/// Benchmark getting commit history with varying commit counts
fn bench_get_commit_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_commit_history");

    for commit_count in [10, 50, 100].iter() {
        let temp_dir = create_test_repo_with_commits(*commit_count);
        let client = GitClient::discover(temp_dir.path()).expect("Failed to create client");

        let error_count = Cell::new(0);
        group.bench_with_input(
            BenchmarkId::from_parameter(commit_count),
            commit_count,
            |b, _| {
                b.iter(|| {
                    if client.get_commit_history(black_box(50)).is_err() {
                        error_count.set(error_count.get() + 1);
                    }
                });
            },
        );
        if error_count.get() > 0 {
            eprintln!(
                "Warning: get_commit_history had {} errors during benchmark",
                error_count.get()
            );
        }
    }
    group.finish();
}

/// Benchmark listing branches
fn bench_list_branches(c: &mut Criterion) {
    let temp_dir = create_test_repo_with_commits(1);
    let client = GitClient::discover(temp_dir.path()).expect("Failed to create client");

    let error_count_local = Cell::new(0);
    c.bench_function("list_branches_local", |b| {
        b.iter(|| {
            if client
                .list_branches(black_box(true), black_box(false))
                .is_err()
            {
                error_count_local.set(error_count_local.get() + 1);
            }
        });
    });
    if error_count_local.get() > 0 {
        eprintln!(
            "Warning: list_branches_local had {} errors during benchmark",
            error_count_local.get()
        );
    }

    let error_count_remote = Cell::new(0);
    c.bench_function("list_branches_remote", |b| {
        b.iter(|| {
            if client
                .list_branches(black_box(false), black_box(true))
                .is_err()
            {
                error_count_remote.set(error_count_remote.get() + 1);
            }
        });
    });
    if error_count_remote.get() > 0 {
        eprintln!(
            "Warning: list_branches_remote had {} errors during benchmark",
            error_count_remote.get()
        );
    }
}

/// Benchmark staging a file
fn bench_stage_file(c: &mut Criterion) {
    let temp_dir = create_test_repo_with_commits(1);
    let repo_path = temp_dir.path();

    // Create a file to stage
    let file = repo_path.join("test_stage.txt");
    fs::write(&file, "content to stage").expect("Failed to write file");

    let client = GitClient::discover(repo_path).expect("Failed to create client");

    let error_count = Cell::new(0);
    c.bench_function("stage_file", |b| {
        b.iter(|| {
            if client.stage_file("test_stage.txt").is_err() {
                error_count.set(error_count.get() + 1);
            }
        });
    });
    if error_count.get() > 0 {
        eprintln!(
            "Warning: stage_file had {} errors during benchmark",
            error_count.get()
        );
    }
}

/// Benchmark unstaging a file
fn bench_unstage_file(c: &mut Criterion) {
    let temp_dir = create_test_repo_with_commits(1);
    let repo_path = temp_dir.path();

    // Create and stage a file
    let file = repo_path.join("test_unstage.txt");
    fs::write(&file, "content").expect("Failed to write file");

    let client = GitClient::discover(repo_path).expect("Failed to create client");
    client.stage_file("test_unstage.txt").ok();

    let error_count = Cell::new(0);
    c.bench_function("unstage_file", |b| {
        b.iter(|| {
            if client.unstage_file("test_unstage.txt").is_err() {
                error_count.set(error_count.get() + 1);
            }
        });
    });
    if error_count.get() > 0 {
        eprintln!(
            "Warning: unstage_file had {} errors during benchmark",
            error_count.get()
        );
    }
}

criterion_group!(
    benches,
    bench_discover,
    bench_head_branch,
    bench_list_changes,
    bench_get_commit_history,
    bench_list_branches,
    bench_stage_file,
    bench_unstage_file,
);

criterion_main!(benches);
