use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct Change {
    pub path: String,
    pub status: FileStatus,
    pub diff_preview: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModuleStatus {
    Pending,
    Current,
    Completed,
}

#[derive(Debug, Clone)]
pub struct Developer {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub id: Uuid,
    pub name: String,
    pub owner: Option<Uuid>,
    pub status: ModuleStatus,
    pub progress_score: u8,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub branch: String,
    pub changes: Vec<Change>,
    pub modules: Vec<Module>,
    pub developers: Vec<Developer>,
}

#[derive(Debug, Default)]
pub struct FakeStore {
    pub projects: Vec<Project>,
}

impl FakeStore {
    pub fn new() -> Self {
        // Create 5 developers
        let alice = Developer {
            id: Uuid::nil(),
            name: "Alice Chen".into(),
        };
        let bob = Developer {
            id: Uuid::nil(),
            name: "Bob Smith".into(),
        };
        let carol = Developer {
            id: Uuid::nil(),
            name: "Carol Davis".into(),
        };
        let dave = Developer {
            id: Uuid::nil(),
            name: "Dave Wilson".into(),
        };
        let eve = Developer {
            id: Uuid::nil(),
            name: "Eve Johnson".into(),
        };

        let mk_change = |path: &str, status: FileStatus| Change {
            path: path.into(),
            status,
            diff_preview: format!("--- a/{p}\n+++ b/{p}\n@@\n- old line\n+ new line", p = path),
        };

        // Project 1: Forge (mature, actively maintained)
        let modules_forge = vec![
            Module {
                id: Uuid::nil(),
                name: "Auth System".into(),
                owner: Some(alice.id),
                status: ModuleStatus::Completed,
                progress_score: 100,
            },
            Module {
                id: Uuid::nil(),
                name: "Dashboard UI".into(),
                owner: Some(bob.id),
                status: ModuleStatus::Current,
                progress_score: 65,
            },
            Module {
                id: Uuid::nil(),
                name: "Merge Visualizer".into(),
                owner: Some(carol.id),
                status: ModuleStatus::Completed,
                progress_score: 100,
            },
            Module {
                id: Uuid::nil(),
                name: "Git Integration".into(),
                owner: Some(dave.id),
                status: ModuleStatus::Current,
                progress_score: 52,
            },
            Module {
                id: Uuid::nil(),
                name: "Settings Panel".into(),
                owner: Some(eve.id),
                status: ModuleStatus::Pending,
                progress_score: 8,
            },
            Module {
                id: Uuid::nil(),
                name: "Notifications Engine".into(),
                owner: None,
                status: ModuleStatus::Pending,
                progress_score: 0,
            },
        ];

        let p1 = Project {
            id: Uuid::nil(),
            name: "Forge".into(),
            description: "Terminal-native Git-aware project manager (TUI)".into(),
            branch: "main".into(),
            changes: vec![
                mk_change("src/main.rs", FileStatus::Modified),
                mk_change("src/pages/help.rs", FileStatus::Added),
                mk_change("src/screen.rs", FileStatus::Modified),
                mk_change("src/data.rs", FileStatus::Modified),
                mk_change("Cargo.toml", FileStatus::Modified),
            ],
            modules: modules_forge,
            developers: vec![
                alice.clone(),
                bob.clone(),
                carol.clone(),
                dave.clone(),
                eve.clone(),
            ],
        };

        // Project 2: Atlas (active development, mid-stage)
        let modules_atlas = vec![
            Module {
                id: Uuid::nil(),
                name: "Core Engine".into(),
                owner: Some(bob.id),
                status: ModuleStatus::Completed,
                progress_score: 98,
            },
            Module {
                id: Uuid::nil(),
                name: "Plugin System".into(),
                owner: Some(carol.id),
                status: ModuleStatus::Current,
                progress_score: 78,
            },
            Module {
                id: Uuid::nil(),
                name: "Performance Tuning".into(),
                owner: Some(dave.id),
                status: ModuleStatus::Current,
                progress_score: 41,
            },
            Module {
                id: Uuid::nil(),
                name: "Benchmarks".into(),
                owner: Some(eve.id),
                status: ModuleStatus::Pending,
                progress_score: 22,
            },
            Module {
                id: Uuid::nil(),
                name: "Documentation".into(),
                owner: None,
                status: ModuleStatus::Pending,
                progress_score: 5,
            },
        ];

        let p2 = Project {
            id: Uuid::nil(),
            name: "Atlas".into(),
            description: "Internal tooling and infrastructure suite for team automation".into(),
            branch: "develop".into(),
            changes: vec![
                mk_change("atlas/core.rs", FileStatus::Modified),
                mk_change("atlas/plugins/mod.rs", FileStatus::Modified),
                mk_change("atlas/plugins/cache.rs", FileStatus::Added),
                mk_change("tests/integration.rs", FileStatus::Modified),
                mk_change("benches/perf.rs", FileStatus::Added),
                mk_change(".github/workflows/bench.yml", FileStatus::Added),
            ],
            modules: modules_atlas,
            developers: vec![
                alice.clone(),
                bob.clone(),
                carol.clone(),
                dave.clone(),
                eve.clone(),
            ],
        };

        // Project 3: Nebula (early stage planning)
        let modules_nebula = vec![
            Module {
                id: Uuid::nil(),
                name: "Design Phase".into(),
                owner: Some(eve.id),
                status: ModuleStatus::Current,
                progress_score: 35,
            },
            Module {
                id: Uuid::nil(),
                name: "Prototyping".into(),
                owner: Some(alice.id),
                status: ModuleStatus::Pending,
                progress_score: 0,
            },
            Module {
                id: Uuid::nil(),
                name: "Market Research".into(),
                owner: None,
                status: ModuleStatus::Pending,
                progress_score: 12,
            },
            Module {
                id: Uuid::nil(),
                name: "API Specification".into(),
                owner: Some(bob.id),
                status: ModuleStatus::Pending,
                progress_score: 0,
            },
        ];

        let p3 = Project {
            id: Uuid::nil(),
            name: "Nebula".into(),
            description: "Next-generation distributed compute platform (early concept)".into(),
            branch: "feature/initial-concept".into(),
            changes: vec![
                mk_change(".github/DESIGN.md", FileStatus::Added),
                mk_change("ROADMAP.md", FileStatus::Added),
                mk_change("docs/architecture.md", FileStatus::Added),
            ],
            modules: modules_nebula,
            developers: vec![eve.clone(), alice.clone(), bob.clone()],
        };

        // Project 4: Skyline (maintenance/legacy)
        let modules_skyline = vec![
            Module {
                id: Uuid::nil(),
                name: "Legacy Codebase".into(),
                owner: Some(carol.id),
                status: ModuleStatus::Completed,
                progress_score: 100,
            },
            Module {
                id: Uuid::nil(),
                name: "Bug Fixes".into(),
                owner: Some(dave.id),
                status: ModuleStatus::Current,
                progress_score: 88,
            },
            Module {
                id: Uuid::nil(),
                name: "Migration to Forge".into(),
                owner: None,
                status: ModuleStatus::Pending,
                progress_score: 10,
            },
        ];

        let p4 = Project {
            id: Uuid::nil(),
            name: "Skyline".into(),
            description: "Legacy system in maintenance mode - gradual migration planned".into(),
            branch: "main".into(),
            changes: vec![
                mk_change("src/legacy/compat.rs", FileStatus::Modified),
                mk_change("tests/regression.rs", FileStatus::Modified),
            ],
            modules: modules_skyline,
            developers: vec![carol.clone(), dave.clone()],
        };

        Self {
            projects: vec![p1, p2, p3, p4],
        }
    }

    pub fn bump_progress_on_commit(&mut self, project_idx: usize) {
        if let Some(project) = self.projects.get_mut(project_idx) {
            // bump first Current module by 5-15, cap at 100
            if let Some(m) = project
                .modules
                .iter_mut()
                .find(|m| m.status == ModuleStatus::Current)
            {
                m.progress_score = (m.progress_score.saturating_add(8)).min(100);
            }
        }
    }
}
