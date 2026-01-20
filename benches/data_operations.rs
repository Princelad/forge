use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use forge::{Developer, Store, Module, ModuleStatus, Project};
use uuid::Uuid;

/// Create a store with N modules and M developers
fn create_store_with_data(num_modules: usize, num_developers: usize) -> Store {
    let mut store = Store::new();

    let project = Project {
        id: Uuid::new_v4(),
        name: "Test Project".to_string(),
        description: "Test".to_string(),
        branch: "main".to_string(),
        changes: Vec::new(),
        modules: (0..num_modules)
            .map(|i| Module {
                id: Uuid::new_v4(),
                name: format!("Module {}", i),
                owner: if i < num_developers {
                    Some(Uuid::new_v4())
                } else {
                    None
                },
                status: if i % 3 == 0 {
                    ModuleStatus::Pending
                } else if i % 3 == 1 {
                    ModuleStatus::Current
                } else {
                    ModuleStatus::Completed
                },
                progress_score: (i * 25) as u8 % 100,
            })
            .collect(),
        developers: (0..num_developers)
            .map(|i| Developer {
                id: Uuid::new_v4(),
                name: format!("Developer {}", i),
            })
            .collect(),
    };

    store.projects.push(project);
    store
}

/// Benchmark bumping progress on commit
fn bench_bump_progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("bump_progress_on_commit");

    for module_count in [10, 100, 1000].iter() {
        let mut store = create_store_with_data(*module_count, 5);

        group.bench_with_input(
            BenchmarkId::from_parameter(module_count),
            module_count,
            |b, _| {
                b.iter(|| {
                    store.bump_progress_on_commit(black_box(0));
                });
            },
        );
    }
    group.finish();
}

/// Benchmark adding a developer
fn bench_add_developer(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_developer");

    for dev_count in [10, 100, 1000].iter() {
        let mut store = create_store_with_data(50, *dev_count);

        group.bench_with_input(BenchmarkId::from_parameter(dev_count), dev_count, |b, _| {
            b.iter(|| {
                store.add_developer(
                    black_box(0),
                    black_box(format!("New Dev {}", Uuid::new_v4())),
                );
            });
        });
    }
    group.finish();
}

/// Benchmark removing a developer
fn bench_delete_developer(c: &mut Criterion) {
    let store = create_store_with_data(50, 100);
    let dev_id = store.projects[0]
        .developers
        .first()
        .map(|d| d.id)
        .expect("No developers in store");

    let mut store = create_store_with_data(50, 100);

    c.bench_function("delete_developer", |b| {
        b.iter(|| {
            store.delete_developer(black_box(0), black_box(dev_id));
        });
    });
}

/// Benchmark auto-populating developers
fn bench_auto_populate_developers(c: &mut Criterion) {
    let mut group = c.benchmark_group("auto_populate_developers");

    for committer_count in [10, 100, 1000].iter() {
        let mut store = create_store_with_data(50, 0);
        let committers: Vec<String> = (0..*committer_count)
            .map(|i| format!("Developer {}", i))
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(committer_count),
            committer_count,
            |b, _| {
                b.iter(|| {
                    store.auto_populate_developers_from_git(
                        black_box(0),
                        black_box(committers.clone()),
                    );
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_bump_progress,
    bench_add_developer,
    bench_delete_developer,
    bench_auto_populate_developers,
);

criterion_main!(benches);
