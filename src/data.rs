use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
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
    // Optional previews for merge visualizer panes; fall back to diff_preview
    pub local_preview: Option<String>,
    pub incoming_preview: Option<String>,
    pub staged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ModuleStatus {
    Pending,
    Current,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Developer {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct Store {
    pub projects: Vec<Project>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            projects: Vec::new(),
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

    // Minimal persistence of module progress to .git/forge/progress.txt
    pub fn save_progress(&self, workdir: &std::path::Path) -> std::io::Result<()> {
        use std::fs::{create_dir_all, File};
        use std::io::Write;
        let dir = workdir.join(".git/forge");
        create_dir_all(&dir)?;
        let mut f = File::create(dir.join("progress.txt"))?;
        for p in &self.projects {
            for m in &p.modules {
                let owner = m
                    .owner
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "".to_string());
                writeln!(
                    f,
                    "{}|{}|{:?}|{}|{}",
                    p.name, m.name, m.status, m.progress_score, owner
                )?;
            }
        }
        Ok(())
    }

    // JSON persistence - save modules and developers
    pub fn save_to_json(&self, workdir: &std::path::Path) -> std::io::Result<()> {
        use std::fs::{create_dir_all, File};
        use std::io::Write;

        let dir = workdir.join(".forge");
        create_dir_all(&dir)?;

        if let Some(project) = self.projects.first() {
            // Save modules
            let modules_json = serde_json::to_string_pretty(&project.modules)?;
            let mut f = File::create(dir.join("modules.json"))?;
            f.write_all(modules_json.as_bytes())?;

            // Save developers
            let devs_json = serde_json::to_string_pretty(&project.developers)?;
            let mut f = File::create(dir.join("developers.json"))?;
            f.write_all(devs_json.as_bytes())?;
        }

        Ok(())
    }

    // JSON persistence - load modules and developers
    pub fn load_from_json(&mut self, workdir: &std::path::Path) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Read;

        let dir = workdir.join(".forge");

        if let Some(project) = self.projects.first_mut() {
            // Load modules
            let modules_path = dir.join("modules.json");
            if modules_path.exists() {
                let mut f = File::open(&modules_path)?;
                let mut contents = String::new();
                f.read_to_string(&mut contents)?;
                if let Ok(modules) = serde_json::from_str(&contents) {
                    project.modules = modules;
                }
            }

            // Load developers
            let devs_path = dir.join("developers.json");
            if devs_path.exists() {
                let mut f = File::open(&devs_path)?;
                let mut contents = String::new();
                f.read_to_string(&mut contents)?;
                if let Ok(developers) = serde_json::from_str(&contents) {
                    project.developers = developers;
                }
            }
        }

        Ok(())
    }

    pub fn load_progress(&mut self, workdir: &std::path::Path) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        let path = workdir.join(".git/forge/progress.txt");
        if !path.exists() {
            return Ok(());
        }
        let reader = BufReader::new(File::open(path)?);
        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 4 {
                continue;
            }
            let (proj_name, module_name, status_str, progress_str) =
                (parts[0], parts[1], parts[2], parts[3]);
            let parsed_status = match status_str {
                "Pending" => ModuleStatus::Pending,
                "Current" => ModuleStatus::Current,
                "Completed" => ModuleStatus::Completed,
                _ => continue,
            };
            let progress: u8 = progress_str.parse().unwrap_or(0);

            if let Some(project) = self.projects.iter_mut().find(|p| p.name == proj_name) {
                if let Some(module) = project.modules.iter_mut().find(|m| m.name == module_name) {
                    module.status = parsed_status;
                    module.progress_score = progress;
                }
            }
        }
        Ok(())
    }

    // CRUD operations for modules
    pub fn add_module(&mut self, project_idx: usize, name: String) -> Option<Uuid> {
        if let Some(project) = self.projects.get_mut(project_idx) {
            let module = Module {
                id: Uuid::new_v4(),
                name,
                owner: None,
                status: ModuleStatus::Pending,
                progress_score: 0,
            };
            let id = module.id;
            project.modules.push(module);
            Some(id)
        } else {
            None
        }
    }

    pub fn update_module(&mut self, project_idx: usize, module_id: Uuid, name: String) -> bool {
        if let Some(project) = self.projects.get_mut(project_idx) {
            if let Some(module) = project.modules.iter_mut().find(|m| m.id == module_id) {
                module.name = name;
                return true;
            }
        }
        false
    }

    pub fn delete_module(&mut self, project_idx: usize, module_id: Uuid) -> bool {
        if let Some(project) = self.projects.get_mut(project_idx) {
            let len_before = project.modules.len();
            project.modules.retain(|m| m.id != module_id);
            project.modules.len() < len_before
        } else {
            false
        }
    }

    pub fn assign_module_owner(
        &mut self,
        project_idx: usize,
        module_id: Uuid,
        developer_id: Option<Uuid>,
    ) -> bool {
        if let Some(project) = self.projects.get_mut(project_idx) {
            if let Some(module) = project.modules.iter_mut().find(|m| m.id == module_id) {
                module.owner = developer_id;
                return true;
            }
        }
        false
    }

    pub fn set_module_status(
        &mut self,
        project_idx: usize,
        module_id: Uuid,
        status: ModuleStatus,
    ) -> bool {
        if let Some(project) = self.projects.get_mut(project_idx) {
            if let Some(module) = project.modules.iter_mut().find(|m| m.id == module_id) {
                module.status = status;
                return true;
            }
        }
        false
    }

    // CRUD operations for developers
    pub fn add_developer(&mut self, project_idx: usize, name: String) -> Option<Uuid> {
        if let Some(project) = self.projects.get_mut(project_idx) {
            let developer = Developer {
                id: Uuid::new_v4(),
                name,
            };
            let id = developer.id;
            project.developers.push(developer);
            Some(id)
        } else {
            None
        }
    }

    pub fn delete_developer(&mut self, project_idx: usize, developer_id: Uuid) -> bool {
        if let Some(project) = self.projects.get_mut(project_idx) {
            let len_before = project.developers.len();
            project.developers.retain(|d| d.id != developer_id);
            // Also unassign from any modules
            for module in &mut project.modules {
                if module.owner == Some(developer_id) {
                    module.owner = None;
                }
            }
            project.developers.len() < len_before
        } else {
            false
        }
    }

    // Auto-populate developers from Git committers
    pub fn auto_populate_developers_from_git(
        &mut self,
        project_idx: usize,
        committer_names: Vec<String>,
    ) {
        if let Some(project) = self.projects.get_mut(project_idx) {
            for name in committer_names {
                // Only add if not already exists
                if !project.developers.iter().any(|d| d.name == name) {
                    project.developers.push(Developer {
                        id: Uuid::new_v4(),
                        name,
                    });
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_status_creation() {
        let _status_modified = FileStatus::Modified;
        let _status_added = FileStatus::Added;
        let _status_deleted = FileStatus::Deleted;

        // Verify Copy trait works
        let status_copy = _status_modified;
        assert!(matches!(status_copy, FileStatus::Modified));
    }

    #[test]
    fn test_change_struct_creation() {
        let change = Change {
            path: "src/main.rs".to_string(),
            status: FileStatus::Modified,
            diff_preview: "--- a/src/main.rs\n+++ b/src/main.rs".to_string(),
            local_preview: Some("local changes".to_string()),
            incoming_preview: Some("incoming changes".to_string()),
            staged: true,
        };

        assert_eq!(change.path, "src/main.rs");
        assert_eq!(change.status, FileStatus::Modified);
        assert!(change.staged);
    }

    #[test]
    fn test_module_status_variants() {
        let pending = ModuleStatus::Pending;
        let current = ModuleStatus::Current;
        let completed = ModuleStatus::Completed;

        assert!(matches!(pending, ModuleStatus::Pending));
        assert!(matches!(current, ModuleStatus::Current));
        assert!(matches!(completed, ModuleStatus::Completed));
    }

    #[test]
    fn test_developer_creation() {
        let dev = Developer {
            id: Uuid::new_v4(),
            name: "Alice".to_string(),
        };

        assert_eq!(dev.name, "Alice");
        let id_string = dev.id.to_string();
        assert!(!id_string.is_empty());
    }

    #[test]
    fn test_module_creation_and_progress() {
        let module_id = Uuid::new_v4();
        let dev_id = Uuid::new_v4();

        let module = Module {
            id: module_id,
            name: "Authentication".to_string(),
            owner: Some(dev_id),
            status: ModuleStatus::Current,
            progress_score: 50,
        };

        assert_eq!(module.name, "Authentication");
        assert_eq!(module.progress_score, 50);
        assert_eq!(module.status, ModuleStatus::Current);
        assert_eq!(module.owner, Some(dev_id));
    }

    #[test]
    fn test_project_creation() {
        let project = Project {
            id: Uuid::new_v4(),
            name: "MyProject".to_string(),
            description: "Test project".to_string(),
            branch: "main".to_string(),
            changes: Vec::new(),
            modules: Vec::new(),
            developers: Vec::new(),
        };

        assert_eq!(project.name, "MyProject");
        assert_eq!(project.branch, "main");
        assert_eq!(project.changes.len(), 0);
    }

    #[test]
    fn test_fake_store_new() {
        let store = Store::new();
        assert_eq!(store.projects.len(), 0);
    }

    #[test]
    fn test_fake_store_add_project() {
        let mut store = Store::new();
        let project = Project {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            description: "".to_string(),
            branch: "main".to_string(),
            changes: Vec::new(),
            modules: Vec::new(),
            developers: Vec::new(),
        };

        store.projects.push(project);
        assert_eq!(store.projects.len(), 1);
    }

    #[test]
    fn test_bump_progress_on_commit() {
        let mut store = Store::new();
        let module = Module {
            id: Uuid::new_v4(),
            name: "Core".to_string(),
            owner: None,
            status: ModuleStatus::Current,
            progress_score: 50,
        };

        let project = Project {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            description: "".to_string(),
            branch: "main".to_string(),
            changes: Vec::new(),
            modules: vec![module],
            developers: Vec::new(),
        };

        store.projects.push(project);
        store.bump_progress_on_commit(0);

        let bumped_score = store.projects[0].modules[0].progress_score;
        assert!(bumped_score > 50, "Progress should increase");
        assert!(bumped_score <= 100, "Progress should not exceed 100");
    }

    #[test]
    fn test_bump_progress_on_commit_caps_at_100() {
        let mut store = Store::new();
        let module = Module {
            id: Uuid::new_v4(),
            name: "Core".to_string(),
            owner: None,
            status: ModuleStatus::Current,
            progress_score: 95,
        };

        let project = Project {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            description: "".to_string(),
            branch: "main".to_string(),
            changes: Vec::new(),
            modules: vec![module],
            developers: Vec::new(),
        };

        store.projects.push(project);
        store.bump_progress_on_commit(0);

        let bumped_score = store.projects[0].modules[0].progress_score;
        assert_eq!(bumped_score, 100, "Progress should be capped at 100");
    }

    #[test]
    fn test_bump_progress_ignores_non_current_modules() {
        let mut store = Store::new();
        let module_pending = Module {
            id: Uuid::new_v4(),
            name: "Pending".to_string(),
            owner: None,
            status: ModuleStatus::Pending,
            progress_score: 0,
        };

        let project = Project {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            description: "".to_string(),
            branch: "main".to_string(),
            changes: Vec::new(),
            modules: vec![module_pending],
            developers: Vec::new(),
        };

        store.projects.push(project);
        store.bump_progress_on_commit(0);

        let score = store.projects[0].modules[0].progress_score;
        assert_eq!(score, 0, "Pending module should not be bumped");
    }

    #[test]
    fn test_add_developer() {
        let mut store = Store::new();
        let project = Project {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            description: "".to_string(),
            branch: "main".to_string(),
            changes: Vec::new(),
            modules: Vec::new(),
            developers: Vec::new(),
        };

        store.projects.push(project);
        let added = store.add_developer(0, "Bob".to_string());
        assert!(added.is_some(), "Should successfully add developer");
        assert_eq!(store.projects[0].developers.len(), 1);
    }

    #[test]
    fn test_add_developer_duplicate() {
        let mut store = Store::new();
        let developer = Developer {
            id: Uuid::new_v4(),
            name: "Bob".to_string(),
        };

        let project = Project {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            description: "".to_string(),
            branch: "main".to_string(),
            changes: Vec::new(),
            modules: Vec::new(),
            developers: vec![developer],
        };

        store.projects.push(project);
        let added = store.add_developer(0, "Bob".to_string());
        assert!(
            added.is_some(),
            "add_developer should still return Some even for duplicate names"
        );
        assert_eq!(
            store.projects[0].developers.len(),
            2,
            "Duplicate is added (no deduplication in add_developer)"
        );
    }

    #[test]
    fn test_auto_populate_developers_from_git() {
        let mut store = Store::new();
        let project = Project {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            description: "".to_string(),
            branch: "main".to_string(),
            changes: Vec::new(),
            modules: Vec::new(),
            developers: Vec::new(),
        };

        store.projects.push(project);

        let committers = vec![
            "Alice <alice@example.com>".to_string(),
            "Bob <bob@example.com>".to_string(),
        ];

        store.auto_populate_developers_from_git(0, committers);
        assert_eq!(store.projects[0].developers.len(), 2);
    }

    #[test]
    fn test_auto_populate_developers_no_duplicates() {
        let mut store = Store::new();
        let developer = Developer {
            id: Uuid::new_v4(),
            name: "Alice <alice@example.com>".to_string(),
        };

        let project = Project {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            description: "".to_string(),
            branch: "main".to_string(),
            changes: Vec::new(),
            modules: Vec::new(),
            developers: vec![developer],
        };

        store.projects.push(project);

        let committers = vec![
            "Alice <alice@example.com>".to_string(),
            "Bob <bob@example.com>".to_string(),
        ];

        store.auto_populate_developers_from_git(0, committers);
        assert_eq!(
            store.projects[0].developers.len(),
            2,
            "Only new developer should be added"
        );
    }
}
