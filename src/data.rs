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
    // Optional previews for merge visualizer panes; fall back to diff_preview
    pub local_preview: Option<String>,
    pub incoming_preview: Option<String>,
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
}
