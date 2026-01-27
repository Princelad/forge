use std::path::PathBuf;

use ratatui::{DefaultTerminal, Frame};

pub mod async_task;
pub mod data;
pub mod git;
pub mod key_handler;
pub mod pages;
pub mod screen;
pub mod state;
pub mod status_symbols;
pub mod ui_utils;
use async_task::{GitOperation, TaskManager};
use data::ModuleStatus;
use key_handler::{ActionContext, ActionProcessor, ActionStateUpdate, KeyAction, KeyHandler};
use pages::branch_manager::BranchInfo;
use pages::commit_history::CommitInfo;
use pages::merge_visualizer::MergePaneFocus;
use screen::Screen;
use state::{
    BoardState, BranchManagerState, ChangesState, CommitHistoryState, DashboardState, MergeState,
    ModuleManagerState,
};
use status_symbols::{error, progress, success};

// UI constants
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

/// Main application state container
///
/// # Architecture Note
///
/// **v0.2.0 Refactoring Complete**: Page-specific state has been extracted into dedicated
/// structs in `src/state/`:
///
/// - `DashboardState`: Project list navigation
/// - `ChangesState`: Git staging and commit interface  
/// - `BoardState`: Kanban board navigation
/// - `MergeState`: Conflict resolution state
/// - `ModuleManagerState`: Module/developer management
/// - `BranchManagerState`: Branch operations
/// - `CommitHistoryState`: Commit history navigation
///
/// **Benefits Achieved**:
/// - Page logic is now unit testable in isolation
/// - Clear separation of concerns
/// - Reduced cognitive load when working with specific pages
/// - Foundation for v0.3.0 full state machine
pub struct App {
    // ====================================================================
    // Core Application State
    // ====================================================================
    running: bool,
    screen: Screen,
    key_handler: KeyHandler,
    status_message: String,
    progress_message: Option<String>,
    last_completion_message: Option<String>,
    store: data::Store,
    settings: AppSettings,
    git_client: Option<git::GitClient>,
    git_workdir: Option<PathBuf>,
    task_manager: TaskManager,
    pending_git_ops: Vec<GitOperation>,

    // ====================================================================
    // Navigation & Focus State
    // ====================================================================
    current_view: AppMode,
    focus: Focus,
    menu_selected_index: usize,
    show_help: bool,
    search_active: bool,
    search_buffer: String,

    // ====================================================================
    // Page State (extracted into dedicated structs)
    // ====================================================================
    /// Dashboard view state (project list navigation)
    dashboard: DashboardState,
    /// Changes view state (Git staging/commit interface)
    changes: ChangesState,
    /// Project board view state (Kanban board)
    board: BoardState,
    /// Merge visualizer view state (conflict resolution)
    merge: MergeState,
    /// Module manager view state (modules & developers)
    module_manager: ModuleManagerState,
    /// Branch manager view state (branch operations)
    branch_manager: BranchManagerState,
    /// Commit history view state
    commit_history: CommitHistoryState,

    // ====================================================================
    // Settings View State (simple, kept inline)
    // ====================================================================
    selected_setting_index: usize,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
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
            progress_message: None,
            last_completion_message: None,
            store: data::Store::new(),
            show_help: false,
            search_active: false,
            search_buffer: String::new(),
            settings: AppSettings {
                theme: Theme::Default,
                notifications: true,
                autosync: false,
            },
            git_client: None,
            git_workdir: None,
            task_manager: TaskManager::new(),
            pending_git_ops: Vec::new(),
            // Page state structs
            dashboard: DashboardState::new(),
            changes: ChangesState::new(),
            board: BoardState::new(),
            merge: MergeState::new(),
            module_manager: ModuleManagerState::new(),
            branch_manager: BranchManagerState::new(),
            commit_history: CommitHistoryState::new(),
            // Settings (kept inline)
            selected_setting_index: 0,
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
                    name: repo_name,
                    description: format!("Git repo at {}", workdir.display()),
                    branch,
                    changes,
                    modules: Vec::new(),
                    developers: Vec::new(),
                };
                app.store.projects = vec![project];
                app.status_message = format!("Git: loaded status from {}", workdir.display());
                app.git_client = Some(client);
                app.git_workdir = Some(workdir);
                // Load persisted data if available
                if let Some(wd) = app.git_workdir.as_ref() {
                    let _ = app.store.load_progress(wd);
                    let _ = app.store.load_from_json(wd);
                }
                // Auto-populate developers from Git history
                if let Some(client) = &app.git_client {
                    if let Ok(committers) = client.get_committers() {
                        app.store.auto_populate_developers_from_git(0, committers);
                        // Save to persist auto-populated developers
                        if let Some(wd) = app.git_workdir.as_ref() {
                            let _ = app.store.save_to_json(wd);
                        }
                    }
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

            // Poll for completed background operations
            self.poll_background_tasks();
        }
        Ok(())
    }

    /// Poll for completed background Git operations
    fn poll_background_tasks(&mut self) {
        if let Some(result) = self.task_manager.try_recv() {
            self.remove_pending_git_op(&result.op);
            match result.result {
                Ok(status) => {
                    let msg = success(&status);
                    self.last_completion_message = Some(msg.clone());
                    self.progress_message = None;
                    self.status_message = msg;
                    // Refresh view cache to show updated data
                    self.refresh_view_cache();
                }
                Err(e) => {
                    let msg = error(&e.to_string());
                    self.last_completion_message = Some(msg.clone());
                    self.progress_message = None;
                    self.status_message = msg;
                }
            }
        }
    }

    fn remove_pending_git_op(&mut self, op: &GitOperation) {
        if let Some(pos) = self
            .pending_git_ops
            .iter()
            .position(|existing| existing == op)
        {
            self.pending_git_ops.remove(pos);
        }
    }

    fn enqueue_git_operation(&mut self, op: GitOperation) {
        if self.git_workdir.is_none() || self.git_client.is_none() {
            self.status_message = error("No Git repository");
            return;
        }

        if let Some(workdir) = self.git_workdir.clone() {
            let label = Self::describe_git_operation(&op);
            self.progress_message = Some(progress(&label));
            self.last_completion_message = None;
            self.pending_git_ops.push(op.clone());
            self.task_manager.spawn_operation(workdir, op);
        }
    }

    fn describe_git_operation(op: &GitOperation) -> String {
        match op {
            GitOperation::Fetch(remote) => format!("Fetching from {}", remote),
            GitOperation::Push(remote) => format!("Pushing to {}", remote),
            GitOperation::Pull(remote) => format!("Pulling from {}", remote),
        }
    }

    fn status_bar_text(&self, pending_git_ops_count: usize) -> String {
        if pending_git_ops_count > 0 {
            if let Some(msg) = &self.progress_message {
                return msg.clone();
            }
        }

        if let Some(msg) = &self.last_completion_message {
            return msg.clone();
        }

        self.status_message.clone()
    }

    fn render(&mut self, frame: &mut Frame) {
        let settings_options = self.settings_options();
        let accepted_merge = self.merge.get_resolution(
            self.dashboard.selected_index,
            self.merge.selected_file_index,
        );
        let workdir = self.git_workdir.as_deref();
        let pending_git_ops_count = self.pending_git_ops.len();

        // Capture frequently used fields to avoid borrow conflicts while mutating screen
        let status_message = self.status_bar_text(pending_git_ops_count);
        let commit_message = self.changes.commit_message.clone();
        let search_buffer = self.search_buffer.clone();
        let module_input_buffer = self.module_manager.input_buffer.clone();
        let branch_input_buffer = self.branch_manager.input_buffer.clone();

        let store = &self.store;
        let filtered_projects: Vec<&crate::data::Project> = if search_buffer.is_empty() {
            store.projects.iter().collect()
        } else {
            let query = search_buffer.to_lowercase();
            store
                .projects
                .iter()
                .filter(|p| p.name.to_lowercase().contains(&query))
                .collect()
        };

        let screen = &mut self.screen;

        let render_ctx = crate::screen::RenderContext {
            mode: self.current_view,
            status: &status_message,
            store,
            selected_project: self.dashboard.selected_index,
            selected_change: self.changes.selected_index,
            commit_msg: &commit_message,
            changes_pane_ratio: self.changes.changes_pane_ratio,
            commit_pane_ratio: self.changes.commit_pane_ratio,
            dashboard_pane_ratio: self.dashboard.pane_ratio,
            menu_selected_index: self.menu_selected_index,
            focus: self.focus,
            selected_board_column: self.board.selected_column,
            selected_board_item: self.board.selected_item,
            merge_file_index: self.merge.selected_file_index,
            merge_focus: self.merge.focus,
            selected_setting: self.selected_setting_index,
            show_help: self.show_help,
            project_scroll: self.dashboard.scroll,
            changes_scroll: self.changes.scroll,
            merge_scroll: self.merge.scroll,
            search_active: self.search_active,
            search_buffer: &search_buffer,
            filtered_projects: &filtered_projects,
            settings_options: &settings_options,
            total_projects: self.store.projects.len(),
            settings: &self.settings,
            accepted_merge,
            workdir,
            module_manager_mode: self.module_manager.mode,
            selected_module: self.module_manager.selected_module,
            selected_developer: self.module_manager.selected_developer,
            module_input_buffer: &module_input_buffer,
            module_scroll: self.module_manager.module_scroll,
            module_pane_ratio: self.module_manager.pane_ratio,
            branch_manager_mode: self.branch_manager.mode,
            selected_branch: self.branch_manager.selected_index,
            branch_input_buffer: &branch_input_buffer,
            branch_scroll: self.branch_manager.scroll,
            cached_branches: &self.branch_manager.cached_branches,
            selected_commit: self.commit_history.selected_index,
            commit_scroll: self.commit_history.scroll,
            cached_commits: &self.commit_history.cached_commits,
            pending_git_ops_count,
        };

        screen.render(frame, &render_ctx);
    }

    fn board_column_len(&self, column: usize) -> usize {
        let status = match column {
            0 => ModuleStatus::Pending,
            1 => ModuleStatus::Current,
            _ => ModuleStatus::Completed,
        };

        self.store
            .projects
            .get(self.dashboard.selected_index)
            .map(|p| p.modules.iter().filter(|m| m.status == status).count())
            .unwrap_or(0)
    }

    fn update_status_message(&mut self) {
        self.status_message = match self.current_view {
            AppMode::Dashboard => format!(
                "Project: {} (↑↓ Select, ↵ Open)",
                self.store
                    .projects
                    .get(self.dashboard.selected_index)
                    .map(|p| &p.name)
                    .unwrap_or(&"N/A".to_string())
            ),
            AppMode::Changes => format!(
                "Changes: {} (↑↓ Select file, ↵ Commit)",
                self.store
                    .projects
                    .get(self.dashboard.selected_index)
                    .and_then(|p| p.changes.get(self.changes.selected_index))
                    .map(|c| &c.path)
                    .unwrap_or(&"N/A".to_string())
            ),
            AppMode::CommitHistory => {
                let count = self.commit_history.cached_commits.len();
                format!("Commit History: {} commits (↑↓ Navigate)", count)
            }
            AppMode::BranchManager => {
                let count = self.branch_manager.cached_branches.len();
                format!("Branches: {} (↑↓ Select, ↵ Switch, n New, d Delete)", count)
            }
            AppMode::ProjectBoard => format!(
                "Board: {} (←→ Column, ↑↓ Item)",
                self.board.current_column_name()
            ),
            AppMode::MergeVisualizer => format!(
                "Merge: {} (←→ Pane, ↑↓ File)",
                match self.merge.focus {
                    MergePaneFocus::Files => "Files",
                    MergePaneFocus::Local => "Local",
                    MergePaneFocus::Incoming => "Incoming",
                }
            ),
            AppMode::ModuleManager => {
                use pages::module_manager::ModuleManagerMode;
                let mode_str = match self.module_manager.mode {
                    ModuleManagerMode::ModuleList => "Modules",
                    ModuleManagerMode::DeveloperList => "Developers",
                    ModuleManagerMode::CreateModule => "Creating Module",
                    ModuleManagerMode::CreateDeveloper => "Creating Developer",
                    ModuleManagerMode::EditModule => "Editing Module",
                };
                format!("{} (n New, d Delete, Tab Switch)", mode_str)
            }
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
        use pages::branch_manager::BranchManagerMode;
        use pages::module_manager::ModuleManagerMode;

        // Build context for stateless processor
        let ctx = ActionContext {
            focus: self.focus,
            current_view: self.current_view,
            show_help: self.show_help,
            search_active: self.search_active,
            menu_selected_index: self.menu_selected_index,
            selected_project_index: self.dashboard.selected_index,
            selected_change_index: self.changes.selected_index,
            selected_board_column: self.board.selected_column,
            selected_board_item: self.board.selected_item,
            selected_merge_file_index: self.merge.selected_file_index,
            selected_setting_index: self.selected_setting_index,
            commit_message_empty: self.changes.is_commit_message_empty(),
            has_git_client: self.git_client.is_some(),
            changes_pane_ratio: self.changes.changes_pane_ratio,
            commit_pane_ratio: self.changes.commit_pane_ratio,
            module_pane_ratio: self.module_manager.pane_ratio,
            dashboard_pane_ratio: self.dashboard.pane_ratio,
            // New view context
            selected_commit_index: self.commit_history.selected_index,
            selected_branch_index: self.branch_manager.selected_index,
            selected_module_index: self.module_manager.selected_module,
            selected_developer_index: self.module_manager.selected_developer,
            cached_commits_len: self.commit_history.cached_commits.len(),
            cached_branches_len: self.branch_manager.cached_branches.len(),
            branch_create_mode: matches!(self.branch_manager.mode, BranchManagerMode::CreateBranch),
            branch_input_empty: self.branch_manager.is_input_empty(),
            module_manager_in_developer_list: self.module_manager.is_developer_list(),
            module_create_mode: matches!(self.module_manager.mode, ModuleManagerMode::CreateModule),
            module_edit_mode: matches!(self.module_manager.mode, ModuleManagerMode::EditModule),
            developer_create_mode: matches!(
                self.module_manager.mode,
                ModuleManagerMode::CreateDeveloper
            ),
            module_assign_mode: self.module_manager.assign_mode,
            module_input_empty: self.module_manager.is_input_empty(),
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
            let old_view = self.current_view;
            self.current_view = view;
            // Refresh caches when entering new views
            if old_view != view {
                self.refresh_view_cache();
            }
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
            self.dashboard.selected_index = idx;
        }
        if let Some(idx) = update.selected_change_index {
            self.changes.selected_index = idx;
        }
        if let Some(idx) = update.selected_board_column {
            self.board.selected_column = idx;
        }
        if let Some(idx) = update.selected_board_item {
            self.board.selected_item = idx;
        }
        if let Some(idx) = update.selected_merge_file_index {
            self.merge.selected_file_index = idx;
        }
        if let Some(idx) = update.selected_setting_index {
            self.selected_setting_index = idx;
        }
        // New view selections
        if let Some(idx) = update.selected_commit_index {
            self.commit_history.selected_index =
                idx.min(self.commit_history.cached_commits.len().saturating_sub(1));
            // Auto-scroll to keep selection visible
            if self.commit_history.selected_index < self.commit_history.scroll {
                self.commit_history.scroll = self.commit_history.selected_index;
            } else if self.commit_history.selected_index >= self.commit_history.scroll + WINDOW_SIZE
            {
                self.commit_history.scroll = self
                    .commit_history
                    .selected_index
                    .saturating_sub(WINDOW_SIZE - 1);
            }
        }
        if let Some(idx) = update.selected_branch_index {
            self.branch_manager.selected_index =
                idx.min(self.branch_manager.cached_branches.len().saturating_sub(1));
            // Auto-scroll to keep selection visible
            if self.branch_manager.selected_index < self.branch_manager.scroll {
                self.branch_manager.scroll = self.branch_manager.selected_index;
            } else if self.branch_manager.selected_index >= self.branch_manager.scroll + WINDOW_SIZE
            {
                self.branch_manager.scroll = self
                    .branch_manager
                    .selected_index
                    .saturating_sub(WINDOW_SIZE - 1);
            }
        }
        if let Some(idx) = update.selected_module_index {
            let module_count = self
                .store
                .projects
                .get(self.dashboard.selected_index)
                .map(|p| p.modules.len())
                .unwrap_or(0);
            self.module_manager.selected_module = idx.min(module_count.saturating_sub(1));
            // Auto-scroll to keep selection visible
            if self.module_manager.selected_module < self.module_manager.module_scroll {
                self.module_manager.module_scroll = self.module_manager.selected_module;
            } else if self.module_manager.selected_module
                >= self.module_manager.module_scroll + WINDOW_SIZE
            {
                self.module_manager.module_scroll = self
                    .module_manager
                    .selected_module
                    .saturating_sub(WINDOW_SIZE - 1);
            }
        }
        if let Some(idx) = update.selected_developer_index {
            let dev_count = self
                .store
                .projects
                .get(self.dashboard.selected_index)
                .map(|p| p.developers.len())
                .unwrap_or(0);
            self.module_manager.selected_developer = idx.min(dev_count.saturating_sub(1));
            // Auto-scroll to keep selection visible
            if self.module_manager.selected_developer < self.module_manager.developer_scroll {
                self.module_manager.developer_scroll = self.module_manager.selected_developer;
            } else if self.module_manager.selected_developer
                >= self.module_manager.developer_scroll + WINDOW_SIZE
            {
                self.module_manager.developer_scroll = self
                    .module_manager
                    .selected_developer
                    .saturating_sub(WINDOW_SIZE - 1);
            }
        }
        if let Some(c) = update.commit_message_append {
            self.changes.append_commit_char(c);
        }
        if update.commit_message_pop.is_some() {
            self.changes.pop_commit_char();
        }
        if update.commit_message_clear.is_some() {
            self.changes.clear_commit_message();
        }
        if let Some(amount) = update.project_scroll_up {
            self.dashboard.scroll_up(amount);
        }
        if let Some(amount) = update.project_scroll_down {
            let max = self.store.projects.len();
            self.dashboard.scroll_down(amount, max, WINDOW_SIZE);
        }
        if let Some(amount) = update.changes_scroll_up {
            self.changes.scroll_up(amount);
        }
        if let Some(amount) = update.changes_scroll_down {
            let max = self
                .store
                .projects
                .get(self.dashboard.selected_index)
                .map(|p| p.changes.len())
                .unwrap_or(0);
            self.changes.scroll_down(amount, max, WINDOW_SIZE);
        }
        if let Some(ratio) = update.changes_pane_ratio {
            self.changes.changes_pane_ratio = ratio;
            self.last_completion_message = Some(format!(
                "Changes pane: {}% (Alt+←/→)",
                self.changes.changes_pane_ratio
            ));
        }
        if let Some(ratio) = update.commit_pane_ratio {
            self.changes.commit_pane_ratio = ratio;
            self.last_completion_message = Some(format!(
                "Commit pane: {}% (Alt+←/→)",
                self.changes.commit_pane_ratio
            ));
        }
        if let Some(ratio) = update.module_pane_ratio {
            self.module_manager.pane_ratio = ratio;
            self.last_completion_message = Some(format!(
                "Module pane: {}% (Alt+←/→)",
                self.module_manager.pane_ratio
            ));
        }
        if let Some(ratio) = update.dashboard_pane_ratio {
            self.dashboard.pane_ratio = ratio;
            self.last_completion_message = Some(format!(
                "Dashboard pane: {}% (Alt+←/→)",
                self.dashboard.pane_ratio
            ));
        }
        if let Some(amount) = update.merge_scroll_up {
            self.merge.scroll_up(amount);
        }
        if let Some(amount) = update.merge_scroll_down {
            let max = self
                .store
                .projects
                .get(self.dashboard.selected_index)
                .map(|p| p.changes.len())
                .unwrap_or(0);
            self.merge.scroll_down(amount, max, WINDOW_SIZE);
        }

        // Complex navigation handlers
        if update.clamp_selections.is_some() {
            self.clamp_selections_for_project();
        }
        if update.navigate_project_down.is_some() {
            let max = self.store.projects.len().saturating_sub(1);
            if self.dashboard.selected_index < max {
                self.dashboard.selected_index += 1;
                self.clamp_selections_for_project();
            }
        }
        if update.navigate_change_down.is_some() {
            let max = self
                .store
                .projects
                .get(self.dashboard.selected_index)
                .map(|p| p.changes.len().saturating_sub(1))
                .unwrap_or(0);
            if self.changes.selected_index < max {
                self.changes.selected_index += 1;
            }
        }
        if update.navigate_board_up.is_some() {
            let len = self.board_column_len(self.board.selected_column);
            self.board.navigate_up(len);
        }
        if update.navigate_board_down.is_some() {
            let len = self.board_column_len(self.board.selected_column);
            self.board.navigate_down(len);
        }
        if update.navigate_board_left.is_some() {
            // Calculate new column first
            let new_col = if self.board.selected_column == 0 {
                2
            } else {
                self.board.selected_column - 1
            };
            let new_len = self.board_column_len(new_col);
            self.board.navigate_left(new_len);
        }
        if update.navigate_board_right.is_some() {
            let new_col = (self.board.selected_column + 1) % 3;
            let new_len = self.board_column_len(new_col);
            self.board.navigate_right(new_len);
        }
        if update.navigate_merge_down.is_some() {
            let max = self
                .store
                .projects
                .get(self.dashboard.selected_index)
                .map(|p| p.changes.len())
                .unwrap_or(0);
            self.merge.navigate_down(max);
        }
        if update.navigate_settings_down.is_some() {
            let max = self.settings_options().len().saturating_sub(1);
            if self.selected_setting_index < max {
                self.selected_setting_index += 1;
            }
        }
        if update.merge_focus_next.is_some() {
            self.merge.focus_next();
        }
        if update.merge_focus_prev.is_some() {
            self.merge.focus_prev();
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

        // Branch operations
        if let Some(mode) = update.branch_create_mode {
            use pages::branch_manager::BranchManagerMode;
            self.branch_manager.mode = if mode {
                BranchManagerMode::CreateBranch
            } else {
                BranchManagerMode::List
            };
        }
        if let Some(c) = update.branch_input_append {
            self.branch_manager.append_input_char(c);
        }
        if update.branch_input_pop.is_some() {
            self.branch_manager.pop_input_char();
        }
        if update.branch_input_clear.is_some() {
            self.branch_manager.clear_input();
        }
        if update.branch_switch_requested.is_some() {
            self.perform_branch_switch();
        }
        if update.branch_create_requested.is_some() {
            self.perform_branch_create();
        }
        if update.branch_delete_requested.is_some() {
            self.perform_branch_delete();
        }

        // Module operations
        if update.toggle_module_list.is_some() {
            self.module_manager.toggle_list();
        }
        if let Some(mode) = update.module_create_mode {
            use pages::module_manager::ModuleManagerMode;
            if mode {
                self.module_manager.mode = ModuleManagerMode::CreateModule;
            } else if matches!(self.module_manager.mode, ModuleManagerMode::CreateModule) {
                self.module_manager.mode = ModuleManagerMode::ModuleList;
            }
        }
        if let Some(mode) = update.module_edit_mode {
            use pages::module_manager::ModuleManagerMode;
            if mode {
                self.module_manager.mode = ModuleManagerMode::EditModule;
            } else if matches!(self.module_manager.mode, ModuleManagerMode::EditModule) {
                self.module_manager.mode = ModuleManagerMode::ModuleList;
            }
        }
        if let Some(mode) = update.developer_create_mode {
            use pages::module_manager::ModuleManagerMode;
            if mode {
                self.module_manager.mode = ModuleManagerMode::CreateDeveloper;
            } else if matches!(self.module_manager.mode, ModuleManagerMode::CreateDeveloper) {
                self.module_manager.mode = ModuleManagerMode::DeveloperList;
            }
        }
        if let Some(c) = update.module_input_append {
            self.module_manager.append_input_char(c);
        }
        if update.module_input_pop.is_some() {
            self.module_manager.pop_input_char();
        }
        if update.module_input_clear.is_some() {
            self.module_manager.clear_input();
        }
        if update.module_load_selected.is_some() {
            self.load_selected_module_for_edit();
        }
        if update.module_create_requested.is_some() {
            self.perform_module_create();
        }
        if update.module_update_requested.is_some() {
            self.perform_module_update();
        }
        if update.module_delete_requested.is_some() {
            self.perform_module_delete();
        }
        if update.developer_create_requested.is_some() {
            self.perform_developer_create();
        }
        if update.developer_delete_requested.is_some() {
            self.perform_developer_delete();
        }
        if let Some(mode) = update.module_assign_mode {
            self.module_manager.assign_mode = mode;
        }
        if update.module_assign_requested.is_some() {
            self.perform_module_assignment();
        }
        if update.toggle_staging_requested.is_some() {
            self.toggle_file_staging();
        }
        if update.fetch_requested.is_some() {
            self.perform_fetch();
        }
        if update.push_requested.is_some() {
            self.perform_push();
        }
        if update.pull_requested.is_some() {
            self.perform_pull();
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }

    fn clamp_selections_for_project(&mut self) {
        // When switching projects, ensure selections are valid for the new project
        if let Some(project) = self.store.projects.get(self.dashboard.selected_index) {
            self.changes.clamp_selection(project.changes.len());
            self.merge.clamp_selection(project.changes.len());
            let board_len = self.board_column_len(self.board.selected_column);
            self.board.clamp_selection(board_len);
        }
    }

    fn move_board_item_to_next_status(&mut self) {
        if let Some(project) = self.store.projects.get_mut(self.dashboard.selected_index) {
            let status = self.board.current_status();

            let modules_in_col: Vec<usize> = project
                .modules
                .iter()
                .enumerate()
                .filter(|(_, m)| m.status == status)
                .map(|(i, _)| i)
                .collect();

            if let Some(&module_idx) = modules_in_col.get(self.board.selected_item) {
                let next_status = match status {
                    ModuleStatus::Pending => ModuleStatus::Current,
                    ModuleStatus::Current => ModuleStatus::Completed,
                    ModuleStatus::Completed => ModuleStatus::Completed,
                };
                project.modules[module_idx].status = next_status;
                self.status_message = success(&format!(
                    "Moved {} to {:?}",
                    project.modules[module_idx].name, next_status
                ));
            }
        }
    }

    fn accept_merge_pane(&mut self) {
        if let Some(msg) = self
            .merge
            .accept_current_pane(self.dashboard.selected_index)
        {
            self.status_message = success(msg);
        } else {
            self.status_message = "Selected file for merge".to_string();
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
        let msg = self.changes.commit_message.trim();
        if let Some(client) = &self.git_client {
            // Check if any files are staged
            let has_staged = self
                .store
                .projects
                .get(self.dashboard.selected_index)
                .map(|p| p.changes.iter().any(|c| c.staged))
                .unwrap_or(false);

            if !has_staged {
                self.status_message = "No files staged for commit".into();
                return;
            }

            match client.commit_all(msg) {
                Ok(_oid) => {
                    // Refresh changes and bump progress
                    if let Ok(changes) = client.list_changes() {
                        if let Some(project) =
                            self.store.projects.get_mut(self.dashboard.selected_index)
                        {
                            project.changes = changes;
                        }
                    }
                    self.store
                        .bump_progress_on_commit(self.dashboard.selected_index);
                    self.status_message = success(&format!("Committed: {}", msg));
                    self.changes.clear_commit_message();
                    if let Some(wd) = self.git_workdir.as_ref() {
                        let _ = self.store.save_progress(wd);
                    }
                }
                Err(e) => {
                    self.status_message = error(&format!("Commit failed: {}", e));
                }
            }
        }
    }

    fn refresh_view_cache(&mut self) {
        if let Some(client) = &self.git_client {
            match self.current_view {
                AppMode::BranchManager => {
                    if let Ok(branches) = client.list_branches(true, false) {
                        let branch_infos: Vec<BranchInfo> = branches
                            .into_iter()
                            .map(|(name, is_current)| BranchInfo {
                                name,
                                is_current,
                                is_remote: false,
                            })
                            .collect();
                        self.branch_manager.update_branches(branch_infos);
                    }
                }
                AppMode::CommitHistory => {
                    if let Ok(commits) = client.get_commit_history(50) {
                        let commit_infos: Vec<CommitInfo> = commits
                            .into_iter()
                            .map(|(hash, author, date, message, files)| CommitInfo {
                                hash,
                                author,
                                date,
                                message,
                                files_changed: files,
                            })
                            .collect();
                        self.commit_history.update_commits(commit_infos);
                    }
                }
                AppMode::Changes => {
                    // Refresh changes when entering the view
                    if let Ok(changes) = client.list_changes() {
                        if let Some(project) =
                            self.store.projects.get_mut(self.dashboard.selected_index)
                        {
                            project.changes = changes;
                            project.branch = client.head_branch().unwrap_or_default();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn perform_branch_switch(&mut self) {
        let branch_info = self
            .branch_manager
            .selected_branch()
            .map(|b| (b.name.clone(), b.is_current));

        if let Some((name, is_current)) = branch_info {
            if is_current {
                self.status_message = "Already on this branch".into();
                return;
            }

            if let Some(client) = &self.git_client {
                match client.checkout_branch(&name) {
                    Ok(()) => {
                        self.status_message = success(&format!("Switched to branch: {}", name));
                        // Refresh branch list
                        self.refresh_view_cache();
                        // Update project branch info
                        if let Some(project) =
                            self.store.projects.get_mut(self.dashboard.selected_index)
                        {
                            project.branch = name;
                        }
                    }
                    Err(e) => {
                        self.status_message = error(&format!("Failed to switch branch: {}", e));
                    }
                }
            }
        }
    }

    fn perform_branch_create(&mut self) {
        let branch_name = self.branch_manager.get_input_value();
        if let Some(client) = &self.git_client {
            match client.create_branch(branch_name) {
                Ok(()) => {
                    self.status_message = success(&format!("Created branch: {}", branch_name));
                    self.branch_manager.exit_create_mode();
                    // Refresh branch list
                    self.refresh_view_cache();
                }
                Err(e) => {
                    self.status_message = error(&format!("Failed to create branch: {}", e));
                }
            }
        }
    }

    fn perform_branch_delete(&mut self) {
        let branch_info = self
            .branch_manager
            .selected_branch()
            .map(|b| (b.name.clone(), b.is_current));

        if let Some((name, is_current)) = branch_info {
            if is_current {
                self.status_message = "Cannot delete current branch".into();
                return;
            }

            if let Some(client) = &self.git_client {
                match client.delete_branch(&name) {
                    Ok(()) => {
                        self.status_message = success(&format!("Deleted branch: {}", name));
                        // Refresh branch list
                        self.refresh_view_cache();
                    }
                    Err(e) => {
                        self.status_message = error(&format!("Failed to delete branch: {}", e));
                    }
                }
            }
        }
    }

    fn load_selected_module_for_edit(&mut self) {
        if let Some(project) = self.store.projects.get(self.dashboard.selected_index) {
            if let Some(module) = project.modules.get(self.module_manager.selected_module) {
                self.module_manager
                    .enter_edit_module(module.id, &module.name);
            }
        }
    }

    fn perform_module_create(&mut self) {
        let module_name = self.module_manager.get_input_value().to_string();
        if let Some(_id) = self
            .store
            .add_module(self.dashboard.selected_index, module_name.clone())
        {
            self.status_message = success(&format!("Created module: {}", module_name));
            self.module_manager.exit_current_mode();
            if let Some(wd) = self.git_workdir.as_ref() {
                let _ = self.store.save_to_json(wd);
            }
        } else {
            self.status_message = error("Failed to create module");
        }
    }

    fn perform_module_update(&mut self) {
        let module_name = self.module_manager.get_input_value().to_string();
        if let Some(module_id) = self.module_manager.editing_module_id {
            if self.store.update_module(
                self.dashboard.selected_index,
                module_id,
                module_name.clone(),
            ) {
                self.status_message = success(&format!("Updated module: {}", module_name));
                self.module_manager.exit_current_mode();
                if let Some(wd) = self.git_workdir.as_ref() {
                    let _ = self.store.save_to_json(wd);
                }
            } else {
                self.status_message = error("Failed to update module");
            }
        }
    }

    fn perform_module_delete(&mut self) {
        if let Some(project) = self.store.projects.get(self.dashboard.selected_index) {
            if let Some(module) = project.modules.get(self.module_manager.selected_module) {
                let module_id = module.id;
                let module_name = module.name.clone();
                if self
                    .store
                    .delete_module(self.dashboard.selected_index, module_id)
                {
                    self.status_message = success(&format!("Deleted module: {}", module_name));
                    // Adjust selection
                    let new_count = self.store.projects[self.dashboard.selected_index]
                        .modules
                        .len();
                    self.module_manager.clamp_selections(new_count, 0);
                    if let Some(wd) = self.git_workdir.as_ref() {
                        let _ = self.store.save_to_json(wd);
                    }
                } else {
                    self.status_message = error("Failed to delete module");
                }
            }
        }
    }

    fn perform_developer_create(&mut self) {
        let developer_name = self.module_manager.get_input_value().to_string();
        if let Some(_id) = self
            .store
            .add_developer(self.dashboard.selected_index, developer_name.clone())
        {
            self.status_message = success(&format!("Created developer: {}", developer_name));
            self.module_manager.exit_current_mode();
            if let Some(wd) = self.git_workdir.as_ref() {
                let _ = self.store.save_to_json(wd);
            }
        } else {
            self.status_message = error("Failed to create developer");
        }
    }

    fn perform_developer_delete(&mut self) {
        if let Some(project) = self.store.projects.get(self.dashboard.selected_index) {
            if let Some(developer) = project
                .developers
                .get(self.module_manager.selected_developer)
            {
                let developer_id = developer.id;
                let developer_name = developer.name.clone();
                if self
                    .store
                    .delete_developer(self.dashboard.selected_index, developer_id)
                {
                    self.status_message =
                        success(&format!("Deleted developer: {}", developer_name));
                    // Adjust selection
                    let new_count = self.store.projects[self.dashboard.selected_index]
                        .developers
                        .len();
                    self.module_manager.clamp_selections(0, new_count);
                    if let Some(wd) = self.git_workdir.as_ref() {
                        let _ = self.store.save_to_json(wd);
                    }
                } else {
                    self.status_message = error("Failed to delete developer");
                }
            }
        }
    }

    fn toggle_file_staging(&mut self) {
        if let Some(project) = self.store.projects.get_mut(self.dashboard.selected_index) {
            if let Some(change) = project.changes.get(self.changes.selected_index) {
                let path = change.path.clone();
                let is_staged = change.staged;

                if let Some(client) = &self.git_client {
                    let result = if is_staged {
                        client.unstage_file(&path)
                    } else {
                        client.stage_file(&path)
                    };

                    match result {
                        Ok(()) => {
                            // Refresh changes to update staging status
                            if let Ok(changes) = client.list_changes() {
                                project.changes = changes;
                                self.status_message = if is_staged {
                                    success(&format!("Unstaged: {}", path))
                                } else {
                                    success(&format!("Staged: {}", path))
                                };
                            }
                        }
                        Err(e) => {
                            self.status_message = error(&format!(
                                "Failed to {} {}: {}",
                                if is_staged { "unstage" } else { "stage" },
                                path,
                                e
                            ));
                        }
                    }
                }
            }
        }
    }

    fn perform_fetch(&mut self) {
        self.enqueue_git_operation(GitOperation::Fetch("origin".to_string()));
    }

    fn perform_push(&mut self) {
        self.enqueue_git_operation(GitOperation::Push("origin".to_string()));
    }

    fn perform_pull(&mut self) {
        self.enqueue_git_operation(GitOperation::Pull("origin".to_string()));
    }

    fn perform_module_assignment(&mut self) {
        if let Some(project) = self.store.projects.get_mut(self.dashboard.selected_index) {
            if let Some(module) = project.modules.get(self.module_manager.selected_module) {
                let module_id = module.id;
                if let Some(developer) = project
                    .developers
                    .get(self.module_manager.selected_developer)
                {
                    let developer_id = developer.id;
                    let developer_name = developer.name.clone();
                    if self.store.assign_module_owner(
                        self.dashboard.selected_index,
                        module_id,
                        Some(developer_id),
                    ) {
                        self.status_message =
                            success(&format!("Assigned {} to module", developer_name));
                        self.module_manager.assign_mode = false;
                        if let Some(wd) = self.git_workdir.as_ref() {
                            let _ = self.store.save_to_json(wd);
                        }
                    } else {
                        self.status_message = error("Failed to assign developer");
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Dashboard,
    Changes,
    CommitHistory,
    BranchManager,
    MergeVisualizer,
    ProjectBoard,
    ModuleManager,
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
                "Notifications: {} (placeholder)",
                if self.settings.notifications {
                    "On"
                } else {
                    "Off"
                }
            ),
            format!(
                "Autosync: {} (placeholder)",
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
            Changes => CommitHistory,
            CommitHistory => BranchManager,
            BranchManager => MergeVisualizer,
            MergeVisualizer => ProjectBoard,
            ProjectBoard => ModuleManager,
            ModuleManager => Settings,
            Settings => Dashboard,
        }
    }

    pub fn menu_index(self) -> usize {
        match self {
            AppMode::Dashboard => 0,
            AppMode::Changes => 1,
            AppMode::CommitHistory => 2,
            AppMode::BranchManager => 3,
            AppMode::MergeVisualizer => 4,
            AppMode::ProjectBoard => 5,
            AppMode::ModuleManager => 6,
            AppMode::Settings => 7,
        }
    }
}
