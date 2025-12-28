use std::collections::HashMap;
use std::path::PathBuf;

use ratatui::{DefaultTerminal, Frame};

pub mod data;
pub mod git;
pub mod key_handler;
pub mod pages;
pub mod screen;
use data::ModuleStatus;
use key_handler::{ActionContext, ActionProcessor, ActionStateUpdate, KeyAction, KeyHandler};
use pages::merge_visualizer::MergePaneFocus;
use screen::Screen;

// UI constants
#[allow(dead_code)]
const PAGE_SIZE: usize = 5;
const WINDOW_SIZE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Default,
    HighContrast,
}

#[derive(Debug, Clone, Copy)]
pub struct AppSettings {
    pub theme: Theme,
    pub notifications: bool,
    pub autosync: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Menu,
    View,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

pub struct App {
    running: bool,
    screen: Screen,
    key_handler: KeyHandler,
    current_view: AppMode,
    focus: Focus,
    menu_selected_index: usize,
    status_message: String,
    store: data::FakeStore,
    selected_project_index: usize,
    selected_change_index: usize,
    commit_message: String,
    selected_board_column: usize,
    selected_board_item: usize,
    selected_merge_file_index: usize,
    merge_focus: MergePaneFocus,
    selected_setting_index: usize,
    show_help: bool,
    // Scroll positions for list views
    project_scroll: usize,
    changes_scroll: usize,
    merge_scroll: usize,
    // Search functionality
    search_active: bool,
    search_buffer: String,
    settings: AppSettings,
    merge_resolutions: HashMap<(usize, usize), MergePaneFocus>,
    git_client: Option<git::GitClient>,
    git_workdir: Option<PathBuf>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            running: false,
            screen: Screen::new(),
            key_handler: KeyHandler::new(),
            current_view: AppMode::Dashboard,
            focus: Focus::View,
            menu_selected_index: 0,
            status_message: String::from("Ready | Press ? for help"),
            store: data::FakeStore::new(),
            selected_project_index: 0,
            selected_change_index: 0,
            commit_message: String::new(),
            selected_board_column: 1,
            selected_board_item: 0,
            selected_merge_file_index: 0,
            merge_focus: MergePaneFocus::Files,
            selected_setting_index: 0,
            show_help: false,
            project_scroll: 0,
            changes_scroll: 0,
            merge_scroll: 0,
            search_active: false,
            search_buffer: String::new(),
            settings: AppSettings {
                theme: Theme::Default,
                notifications: true,
                autosync: false,
            },
            merge_resolutions: HashMap::new(),
            git_client: None,
            git_workdir: None,
        };

        // Attempt to discover a Git repository from the current directory
        if let Ok(cwd) = std::env::current_dir() {
            if let Ok(client) = git::GitClient::discover(&cwd) {
                let workdir = client.workdir.clone();
                let branch = client.head_branch().unwrap_or_else(|| "HEAD".into());
                let repo_name = workdir
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "repository".into());

                let changes = client.list_changes().unwrap_or_default();
                let project = data::Project {
                    id: uuid::Uuid::nil(),
                    name: repo_name.clone(),
                    description: format!("Git repo at {}", workdir.display()),
                    branch,
                    changes,
                    modules: Vec::new(),
                    developers: Vec::new(),
                };
                app.store.projects = vec![project];
                app.status_message = format!("Git: loaded status from {}", workdir.display());
                app.git_workdir = Some(workdir);
                app.git_client = Some(client);
                // Load persisted progress if available
                if let Some(wd) = app.git_workdir.as_ref() {
                    let _ = app.store.load_progress(wd);
                }
            }
        }

        app
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            let action = self.key_handler.handle_crossterm_events()?;
            if self.handle_action(action) {
                self.quit();
            }
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let filtered_projects = self.get_filtered_projects();
        let settings_options = self.settings_options();
        let accepted_merge = self
            .merge_resolutions
            .get(&(self.selected_project_index, self.selected_merge_file_index))
            .copied();
        let workdir = self.git_workdir.as_deref();
        self.screen.render(
            frame,
            self.current_view,
            &self.status_message,
            &self.store,
            self.selected_project_index,
            self.selected_change_index,
            &self.commit_message,
            self.menu_selected_index,
            self.focus,
            self.selected_board_column,
            self.selected_board_item,
            self.selected_merge_file_index,
            self.merge_focus,
            self.selected_setting_index,
            self.show_help,
            self.project_scroll,
            self.changes_scroll,
            self.merge_scroll,
            self.search_active,
            &self.search_buffer,
            &filtered_projects,
            &settings_options,
            self.store.projects.len(),
            &self.settings,
            accepted_merge,
            workdir,
        );
    }

    fn board_column_len(&self, column: usize) -> usize {
        let status = match column {
            0 => ModuleStatus::Pending,
            1 => ModuleStatus::Current,
            _ => ModuleStatus::Completed,
        };

        self.store
            .projects
            .get(self.selected_project_index)
            .map(|p| p.modules.iter().filter(|m| m.status == status).count())
            .unwrap_or(0)
    }

    fn update_status_message(&mut self) {
        self.status_message = match self.current_view {
            AppMode::Dashboard => format!(
                "Project: {} (↑↓ Select, ↵ Open)",
                self.store
                    .projects
                    .get(self.selected_project_index)
                    .map(|p| &p.name)
                    .unwrap_or(&"N/A".to_string())
            ),
            AppMode::Changes => format!(
                "Changes: {} (↑↓ Select file, ↵ Commit)",
                self.store
                    .projects
                    .get(self.selected_project_index)
                    .and_then(|p| p.changes.get(self.selected_change_index))
                    .map(|c| &c.path)
                    .unwrap_or(&"N/A".to_string())
            ),
            AppMode::ProjectBoard => format!(
                "Board: {} (←→ Column, ↑↓ Item)",
                match self.selected_board_column {
                    0 => "Pending",
                    1 => "Current",
                    _ => "Completed",
                }
            ),
            AppMode::MergeVisualizer => format!(
                "Merge: {} (←→ Pane, ↑↓ File)",
                match self.merge_focus {
                    MergePaneFocus::Files => "Files",
                    MergePaneFocus::Local => "Local",
                    MergePaneFocus::Incoming => "Incoming",
                }
            ),
            AppMode::Settings => {
                let opts = self.settings_options();
                let label = opts
                    .get(self.selected_setting_index)
                    .map(|s| s.as_str())
                    .unwrap_or("N/A");
                format!("Settings: {} (↑↓ Select, ↵ Toggle)", label)
            }
        };
    }

    fn handle_action(&mut self, action: KeyAction) -> bool {
        // Build context for stateless processor
        let ctx = ActionContext {
            focus: self.focus,
            current_view: self.current_view,
            show_help: self.show_help,
            search_active: self.search_active,
            menu_selected_index: self.menu_selected_index,
            selected_project_index: self.selected_project_index,
            selected_change_index: self.selected_change_index,
            selected_board_column: self.selected_board_column,
            selected_board_item: self.selected_board_item,
            selected_merge_file_index: self.selected_merge_file_index,
            selected_setting_index: self.selected_setting_index,
            commit_message_empty: self.commit_message.trim().is_empty(),
            has_git_client: self.git_client.is_some(),
        };

        // Process action (stateless)
        let (result, update) = ActionProcessor::process(action, &ctx);

        // Apply state updates
        self.apply_action_updates(update);

        // Set status if provided
        if let Some(msg) = result.status_message {
            self.status_message = msg;
            self.update_status_message(); // Will be overwritten only if msg is generic
        } else {
            self.update_status_message();
        }

        result.should_quit
    }

    fn apply_action_updates(&mut self, update: ActionStateUpdate) {
        // Apply all optional state updates
        if let Some(focus) = update.focus {
            self.focus = focus;
        }
        if let Some(view) = update.current_view {
            self.current_view = view;
        }
        if let Some(help) = update.show_help {
            self.show_help = help;
        }
        if let Some(search) = update.search_active {
            self.search_active = search;
        }
        if let Some(buf) = update.search_buffer {
            self.search_buffer = buf;
        }
        if let Some(c) = update.search_buffer_append {
            self.search_buffer.push(c);
        }
        if update.search_buffer_pop.is_some() {
            self.search_buffer.pop();
        }
        if let Some(idx) = update.menu_selected_index {
            self.menu_selected_index = idx;
        }
        if let Some(idx) = update.selected_project_index {
            self.selected_project_index = idx;
        }
        if let Some(idx) = update.selected_change_index {
            self.selected_change_index = idx;
        }
        if let Some(idx) = update.selected_board_column {
            self.selected_board_column = idx;
        }
        if let Some(idx) = update.selected_board_item {
            self.selected_board_item = idx;
        }
        if let Some(idx) = update.selected_merge_file_index {
            self.selected_merge_file_index = idx;
        }
        if let Some(idx) = update.selected_setting_index {
            self.selected_setting_index = idx;
        }
        if let Some(c) = update.commit_message_append {
            self.commit_message.push(c);
        }
        if update.commit_message_pop.is_some() {
            self.commit_message.pop();
        }
        if update.commit_message_clear.is_some() {
            self.commit_message.clear();
        }
        if let Some(amount) = update.project_scroll_up {
            self.project_scroll = self.project_scroll.saturating_sub(amount);
        }
        if let Some(amount) = update.project_scroll_down {
            let max = self.store.projects.len();
            if max > WINDOW_SIZE {
                self.project_scroll = (self.project_scroll + amount).min(max - WINDOW_SIZE);
            }
        }
        if let Some(amount) = update.changes_scroll_up {
            self.changes_scroll = self.changes_scroll.saturating_sub(amount);
        }
        if let Some(amount) = update.changes_scroll_down {
            let max = self
                .store
                .projects
                .get(self.selected_project_index)
                .map(|p| p.changes.len())
                .unwrap_or(0);
            if max > WINDOW_SIZE {
                self.changes_scroll = (self.changes_scroll + amount).min(max - WINDOW_SIZE);
            }
        }
        if let Some(amount) = update.merge_scroll_up {
            self.merge_scroll = self.merge_scroll.saturating_sub(amount);
        }
        if let Some(amount) = update.merge_scroll_down {
            let max = self
                .store
                .projects
                .get(self.selected_project_index)
                .map(|p| p.changes.len())
                .unwrap_or(0);
            if max > WINDOW_SIZE {
                self.merge_scroll = (self.merge_scroll + amount).min(max - WINDOW_SIZE);
            }
        }

        // Complex navigation handlers
        if update.clamp_selections.is_some() {
            self.clamp_selections_for_project();
        }
        if update.navigate_project_down.is_some() {
            let max = self.store.projects.len().saturating_sub(1);
            if self.selected_project_index < max {
                self.selected_project_index += 1;
                self.clamp_selections_for_project();
            }
        }
        if update.navigate_change_down.is_some() {
            let max = self
                .store
                .projects
                .get(self.selected_project_index)
                .map(|p| p.changes.len().saturating_sub(1))
                .unwrap_or(0);
            if self.selected_change_index < max {
                self.selected_change_index += 1;
            }
        }
        if update.navigate_board_up.is_some() {
            let len = self.board_column_len(self.selected_board_column);
            if len == 0 {
                self.selected_board_item = 0;
            } else if self.selected_board_item > 0 {
                self.selected_board_item -= 1;
            } else {
                self.selected_board_item = len - 1;
            }
        }
        if update.navigate_board_down.is_some() {
            let len = self.board_column_len(self.selected_board_column);
            if len == 0 {
                self.selected_board_item = 0;
            } else if self.selected_board_item < len.saturating_sub(1) {
                self.selected_board_item += 1;
            }
        }
        if update.navigate_board_left.is_some() {
            if self.selected_board_column == 0 {
                self.selected_board_column = 2;
            } else {
                self.selected_board_column -= 1;
            }
            let len = self.board_column_len(self.selected_board_column);
            self.selected_board_item = if len == 0 {
                0
            } else {
                self.selected_board_item.min(len - 1)
            };
        }
        if update.navigate_board_right.is_some() {
            self.selected_board_column = (self.selected_board_column + 1) % 3;
            let len = self.board_column_len(self.selected_board_column);
            self.selected_board_item = if len == 0 {
                0
            } else {
                self.selected_board_item.min(len - 1)
            };
        }
        if update.navigate_merge_down.is_some() {
            let max = self
                .store
                .projects
                .get(self.selected_project_index)
                .map(|p| p.changes.len().saturating_sub(1))
                .unwrap_or(0);
            if self.selected_merge_file_index < max {
                self.selected_merge_file_index += 1;
            }
        }
        if update.navigate_settings_down.is_some() {
            let max = self.settings_options().len().saturating_sub(1);
            if self.selected_setting_index < max {
                self.selected_setting_index += 1;
            }
        }
        if update.merge_focus_next.is_some() {
            self.merge_focus = self.merge_focus.next();
        }
        if update.merge_focus_prev.is_some() {
            self.merge_focus = self.merge_focus.prev();
        }

        // Action-specific handlers
        if update.move_board_item.is_some() {
            self.move_board_item_to_next_status();
        }
        if update.accept_merge_pane.is_some() {
            self.accept_merge_pane();
        }
        if update.toggle_setting.is_some() {
            self.toggle_setting();
        }
        if update.commit_requested.is_some() {
            self.perform_commit();
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }

    fn get_filtered_projects(&self) -> Vec<&crate::data::Project> {
        if self.search_buffer.is_empty() {
            return self.store.projects.iter().collect();
        }
        let query = self.search_buffer.to_lowercase();
        self.store
            .projects
            .iter()
            .filter(|p| p.name.to_lowercase().contains(&query))
            .collect()
    }

    fn clamp_selections_for_project(&mut self) {
        // When switching projects, ensure selections are valid for the new project
        if let Some(project) = self.store.projects.get(self.selected_project_index) {
            self.selected_change_index = self
                .selected_change_index
                .min(project.changes.len().saturating_sub(1));
            self.selected_merge_file_index = self
                .selected_merge_file_index
                .min(project.changes.len().saturating_sub(1));
            self.selected_board_item = self.selected_board_item.min(
                self.board_column_len(self.selected_board_column)
                    .saturating_sub(1),
            );
        }
    }

    fn move_board_item_to_next_status(&mut self) {
        if let Some(project) = self.store.projects.get_mut(self.selected_project_index) {
            let status = match self.selected_board_column {
                0 => ModuleStatus::Pending,
                1 => ModuleStatus::Current,
                _ => ModuleStatus::Completed,
            };

            let modules_in_col: Vec<usize> = project
                .modules
                .iter()
                .enumerate()
                .filter(|(_, m)| m.status == status)
                .map(|(i, _)| i)
                .collect();

            if let Some(&module_idx) = modules_in_col.get(self.selected_board_item) {
                let next_status = match status {
                    ModuleStatus::Pending => ModuleStatus::Current,
                    ModuleStatus::Current => ModuleStatus::Completed,
                    ModuleStatus::Completed => ModuleStatus::Completed,
                };
                project.modules[module_idx].status = next_status;
                self.status_message = format!(
                    "✓ Moved {} to {:?}",
                    project.modules[module_idx].name, next_status
                );
            }
        }
    }

    fn accept_merge_pane(&mut self) {
        match self.merge_focus {
            MergePaneFocus::Files => {
                self.status_message = "Selected file for merge".to_string();
            }
            MergePaneFocus::Local | MergePaneFocus::Incoming => {
                self.merge_resolutions.insert(
                    (self.selected_project_index, self.selected_merge_file_index),
                    self.merge_focus,
                );
                self.status_message = match self.merge_focus {
                    MergePaneFocus::Local => "✓ Accepted local version".to_string(),
                    MergePaneFocus::Incoming => "✓ Accepted incoming version".to_string(),
                    _ => unreachable!(),
                };
            }
        }
    }

    fn toggle_setting(&mut self) {
        match self.selected_setting_index {
            0 => {
                // Cycle theme
                self.settings.theme = match self.settings.theme {
                    Theme::Default => Theme::HighContrast,
                    Theme::HighContrast => Theme::Default,
                };
                self.status_message = format!(
                    "⚙ Theme set to {}",
                    match self.settings.theme {
                        Theme::Default => "Default",
                        Theme::HighContrast => "High Contrast",
                    }
                );
            }
            1 => {
                self.settings.notifications = !self.settings.notifications;
                self.status_message = format!(
                    "⚙ Notifications: {}",
                    if self.settings.notifications {
                        "On"
                    } else {
                        "Off"
                    }
                );
            }
            2 => {
                self.settings.autosync = !self.settings.autosync;
                self.status_message = format!(
                    "⚙ Autosync: {}",
                    if self.settings.autosync { "On" } else { "Off" }
                );
            }
            _ => {}
        }
    }

    fn perform_commit(&mut self) {
        let msg = self.commit_message.trim();
        if let Some(client) = &self.git_client {
            match client.stage_all() {
                Ok(()) => match client.commit_all(msg) {
                    Ok(_oid) => {
                        // Refresh changes and bump progress
                        if let Ok(changes) = client.list_changes() {
                            if let Some(project) =
                                self.store.projects.get_mut(self.selected_project_index)
                            {
                                project.changes = changes;
                            }
                        }
                        self.store
                            .bump_progress_on_commit(self.selected_project_index);
                        self.status_message = format!("✓ Committed: {}", msg);
                        self.commit_message.clear();
                        if let Some(wd) = self.git_workdir.as_ref() {
                            let _ = self.store.save_progress(wd);
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("✗ Commit failed: {}", e);
                    }
                },
                Err(e) => {
                    self.status_message = format!("✗ Stage failed: {}", e);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AppMode {
    Dashboard,
    Changes,
    MergeVisualizer,
    ProjectBoard,
    Settings,
}

impl App {
    fn settings_options(&self) -> Vec<String> {
        vec![
            format!(
                "Theme: {}",
                match self.settings.theme {
                    Theme::Default => "Default",
                    Theme::HighContrast => "High Contrast",
                }
            ),
            format!(
                "Notifications: {}",
                if self.settings.notifications {
                    "On"
                } else {
                    "Off"
                }
            ),
            format!(
                "Autosync: {}",
                if self.settings.autosync { "On" } else { "Off" }
            ),
        ]
    }
}

impl AppMode {
    pub fn next(self) -> Self {
        use AppMode::*;
        match self {
            Dashboard => Changes,
            Changes => MergeVisualizer,
            MergeVisualizer => ProjectBoard,
            ProjectBoard => Settings,
            Settings => Dashboard,
        }
    }

    pub fn menu_index(self) -> usize {
        match self {
            AppMode::Dashboard => 0,
            AppMode::Changes => 1,
            AppMode::MergeVisualizer => 2,
            AppMode::ProjectBoard => 3,
            AppMode::Settings => 4,
        }
    }
}
