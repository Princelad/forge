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
        let alice = Developer { id: Uuid::nil(), name: "Alice".into() };
        let bob = Developer { id: Uuid::nil(), name: "Bob".into() };
        let carol = Developer { id: Uuid::nil(), name: "Carol".into() };

        let mk_change = |path: &str, status: FileStatus| Change {
            path: path.into(),
            status,
            diff_preview: format!("--- a/{p}\n+++ b/{p}\n@@\n- old line\n+ new line", p = path),
        };

        let m1 = Module {
            id: Uuid::nil(),
            name: "Auth".into(),
            owner: Some(alice.id),
            status: ModuleStatus::Current,
            progress_score: 42,
        };
        let m2 = Module {
            id: Uuid::nil(),
            name: "Dashboard".into(),
            owner: Some(bob.id),
            status: ModuleStatus::Pending,
            progress_score: 10,
        };
        let m3 = Module {
            id: Uuid::nil(),
            name: "Merge UI".into(),
            owner: Some(carol.id),
            status: ModuleStatus::Completed,
            progress_score: 100,
        };

        let p1 = Project {
            id: Uuid::nil(),
            name: "Forge".into(),
            description: "Terminal-native Git-aware project manager (mock)".into(),
            branch: "main".into(),
            changes: vec![
                mk_change("src/lib.rs", FileStatus::Modified),
                mk_change("README.md", FileStatus::Added),
                mk_change("scripts/setup.sh", FileStatus::Deleted),
            ],
            modules: vec![m1.clone(), m2.clone(), m3.clone()],
            developers: vec![alice.clone(), bob.clone(), carol.clone()],
        };

        let p2 = Project {
            id: Uuid::nil(),
            name: "Atlas".into(),
            description: "Internal tooling demos".into(),
            branch: "develop".into(),
            changes: vec![mk_change("atlas/core.rs", FileStatus::Modified)],
            modules: vec![m2, m1, m3],
            developers: vec![alice, bob, carol],
        };

        Self { projects: vec![p1, p2] }
    }

    pub fn bump_progress_on_commit(&mut self, project_idx: usize) {
        if let Some(project) = self.projects.get_mut(project_idx) {
            // bump first Current module by 5, cap at 100
            if let Some(m) = project.modules.iter_mut().find(|m| m.status == ModuleStatus::Current)
            {
                m.progress_score = (m.progress_score.saturating_add(5)).min(100);
            }
        }
    }
}
