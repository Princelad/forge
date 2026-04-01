use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crossterm::terminal;
use ratatui::{DefaultTerminal, Frame};

pub mod async_task;
pub mod data;
pub mod git;
pub mod key_handler;
pub mod pages;
pub mod screen;
pub mod state;
pub mod status_symbols;
pub mod suggestions;
pub mod ui_utils;
use async_task::{GitOperation, TaskManager};
use data::ModuleStatus;
use key_handler::{ActionContext, ActionProcessor, ActionStateUpdate, KeyAction, KeyHandler};
use pages::branch_manager::{BranchInfo, UpstreamStatus};
use pages::commit_history::CommitInfo;
use pages::merge_visualizer::MergePaneFocus;
use pages::stashes::StashInfo;
use screen::Screen;
use state::{
    BoardState, BranchManagerState, ChangesState, CommitHistoryState, DashboardState, MergeState,
    ModuleManagerState, StashesState,
};
use status_symbols::{error, progress, success};
use suggestions::SuggestionEngine;

// UI constants
const DEFAULT_WINDOW_SIZE: usize = 10;
const AUTOSYNC_INTERVAL: Duration = Duration::from_secs(300);
const DIFF_PREVIEW_PLACEHOLDER: &str = "(diff not loaded)";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Default,
    HighContrast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: Theme,
    pub notifications: bool,
    pub autosync: bool,
    /// Commit message suggestion engine configuration.
    ///
    /// Uses `#[serde(default)]` so that existing config files written
    /// before this field existed will deserialise without error.
    #[serde(default)]
    pub suggestions: suggestions::SuggestionConfig,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Default,
            notifications: true,
            autosync: false,
            suggestions: suggestions::SuggestionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Menu,
    View,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Typing,
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
/// - Foundation for v0.3.1 full state machine
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
    pending_destructive_confirmation: Option<PendingDestructiveAction>,
    store: data::Store,
    settings: AppSettings,
    git_client: Option<git::GitClient>,
    git_workdir: Option<PathBuf>,
    git_health: Option<git::RepoHealthReport>,
    task_manager: TaskManager,
    pending_git_ops: Vec<GitOperation>,
    last_autosync_at: Option<Instant>,
    suggestion_cache: Option<SuggestionCacheEntry>,

    // ====================================================================
    // Navigation & Focus State
    // ====================================================================
    current_view: AppMode,
    focus: Focus,
    input_mode: InputMode,
    menu_selected_index: usize,
    show_help: bool,
    search_active: bool,
    search_buffer: String,
    window_size: usize,

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
    /// Stashes view state
    stashes: StashesState,

    // ====================================================================
    // Settings View State (simple, kept inline)
    // ====================================================================
    selected_setting_index: usize,
    available_remotes: Vec<String>,
    selected_remote_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingDestructiveAction {
    DeleteBranch { name: String },
    DropStash { index: usize, name: String },
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
            input_mode: InputMode::Normal,
            menu_selected_index: 0,
            status_message: String::from("Ready | Press ? for help"),
            progress_message: None,
            last_completion_message: None,
            pending_destructive_confirmation: None,
            store: data::Store::new(),
            show_help: false,
            search_active: false,
            search_buffer: String::new(),
            window_size: DEFAULT_WINDOW_SIZE,
            settings: AppSettings::default(),
            git_client: None,
            git_workdir: None,
            git_health: None,
            task_manager: TaskManager::new(),
            pending_git_ops: Vec::new(),
            last_autosync_at: None,
            suggestion_cache: None,
            // Page state structs
            dashboard: DashboardState::new(),
            changes: ChangesState::new(),
            board: BoardState::new(),
            merge: MergeState::new(),
            module_manager: ModuleManagerState::new(),
            branch_manager: BranchManagerState::new(),
            commit_history: CommitHistoryState::new(),
            stashes: StashesState::new(),
            // Settings (kept inline)
            selected_setting_index: 0,
            available_remotes: Vec::new(),
            selected_remote_index: None,
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

                let health = client.check_repo_health();
                let description = health.format_description(&workdir);

                let mut status_message = format!("Git: loaded status from {}", workdir.display());
                let mut last_completion_message = None;

                let changes = if health.has_blocking_issues() {
                    let mut msg = health.summary();
                    if let Some(hint) = health.inline_recovery_hint() {
                        msg = format!("{} | {}", msg, hint);
                    }
                    let msg = error(&msg);
                    status_message = msg.clone();
                    last_completion_message = Some(msg);
                    Vec::new()
                } else {
                    match client.list_changes_summary() {
                        Ok(changes) => changes,
                        Err(e) => {
                            let msg = error(&git::GitClient::explain_error(&e));
                            status_message = msg.clone();
                            last_completion_message = Some(msg);
                            Vec::new()
                        }
                    }
                };

                let project = data::Project {
                    id: uuid::Uuid::nil(),
                    name: repo_name,
                    description,
                    branch,
                    changes,
                    modules: Vec::new(),
                    developers: Vec::new(),
                };
                app.store.projects = vec![project];
                app.status_message = status_message;
                app.last_completion_message = last_completion_message;
                app.git_client = Some(client);
                app.git_workdir = Some(workdir);
                app.git_health = Some(health);
                app.refresh_remotes();
                app.ensure_change_preview_loaded(app.changes.selected_index);
                app.regenerate_commit_suggestions();
                // Load persisted data if available
                if let Some(wd) = app.git_workdir.as_ref() {
                    let mut startup_diagnostics = Vec::new();

                    let _ = app.store.load_progress(wd);
                    match data::Store::migrate_forge_schema(wd) {
                        Ok(Some(msg)) => startup_diagnostics.push(msg),
                        Ok(None) => {}
                        Err(err) => startup_diagnostics
                            .push(format!(".forge migration diagnostic: {}", err)),
                    }
                    if let Err(err) = app.store.load_from_json(wd) {
                        startup_diagnostics.push(format!(".forge load diagnostic: {}", err));
                    }
                    match app.load_settings_from(wd) {
                        Ok(Some(settings)) => {
                            app.settings = settings;
                        }
                        Ok(None) => {}
                        Err(err) => {
                            startup_diagnostics
                                .push(format!("settings schema diagnostic: {}", err));
                        }
                    }

                    if let Err(err) = key_handler::ensure_default_keybindings_profile(wd) {
                        startup_diagnostics
                            .push(format!("default keymap profile diagnostic: {}", err));
                    }

                    if let Err(err) = app.key_handler.load_keybindings_from(wd) {
                        startup_diagnostics.push(format!("keybindings schema diagnostic: {}", err));
                    }

                    if !startup_diagnostics.is_empty() {
                        let msg = error(&format!(
                            "Startup configuration diagnostics | {}",
                            startup_diagnostics.join(" | ")
                        ));
                        app.status_message = msg.clone();
                        app.last_completion_message = Some(msg);
                    }
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

                if app.settings.autosync {
                    app.maybe_autosync(true);
                }
            }
        }

        app
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;
        self.handle_terminal_resize();
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            let action = self.key_handler.handle_crossterm_events()?;
            if matches!(action, KeyAction::TerminalResized) {
                self.handle_terminal_resize();
            } else if self.handle_action(action) {
                self.quit();
            }

            // Poll for completed background operations
            self.poll_background_tasks();

            // Auto-fetch when autosync is enabled
            self.maybe_autosync(false);
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
                    self.progress_message = None;
                    self.apply_completion_message(msg, false);
                    // Refresh view cache to show updated data
                    self.refresh_view_cache();
                }
                Err(e) => {
                    let label = Self::describe_git_operation(&result.op);
                    let msg = error(&format!("{} failed: {}", label, e));
                    self.progress_message = None;
                    self.apply_completion_message(msg, true);
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

    fn update_repo_health(&mut self, report: git::RepoHealthReport) {
        if let Some(workdir) = self.git_workdir.as_ref() {
            if let Some(project) = self.store.projects.get_mut(self.dashboard.selected_index) {
                project.description = report.format_description(workdir);
            }
        }
        self.git_health = Some(report);
    }

    fn refresh_repo_health(&mut self) -> Option<git::RepoHealthReport> {
        let report = self
            .git_client
            .as_ref()
            .map(|client| client.check_repo_health());
        if let Some(ref report) = report {
            self.update_repo_health(report.clone());
        }
        report
    }

    fn ensure_change_preview_loaded(&mut self, index: usize) {
        let client = match self.git_client.as_ref() {
            Some(client) => client,
            None => return,
        };

        let path = self
            .store
            .projects
            .get(self.dashboard.selected_index)
            .and_then(|project| project.changes.get(index))
            .and_then(|change| {
                if change.diff_preview == DIFF_PREVIEW_PLACEHOLDER {
                    Some(change.path.clone())
                } else {
                    None
                }
            });

        let Some(path) = path else {
            return;
        };

        let (diff_preview, local_preview, incoming_preview) = client.get_change_previews(&path);

        if let Some(change) = self
            .store
            .projects
            .get_mut(self.dashboard.selected_index)
            .and_then(|project| project.changes.get_mut(index))
        {
            change.diff_preview = diff_preview;
            change.local_preview = local_preview;
            change.incoming_preview = incoming_preview;
        }
    }

    fn ensure_merge_conflict_preview_loaded(&mut self, index: usize) {
        let client = match self.git_client.as_ref() {
            Some(client) => client,
            None => return,
        };

        let path = self.merge.conflicts.get(index).and_then(|conflict| {
            if conflict.diff_preview == DIFF_PREVIEW_PLACEHOLDER {
                Some(conflict.path.clone())
            } else {
                None
            }
        });

        let Some(path) = path else {
            return;
        };

        let (diff_preview, local_preview, incoming_preview) = client.get_change_previews(&path);

        if let Some(conflict) = self.merge.conflicts.get_mut(index) {
            conflict.diff_preview = diff_preview;
            conflict.local_preview = local_preview;
            conflict.incoming_preview = incoming_preview;
        }
    }

    fn ensure_selected_commit_files_loaded(&mut self) {
        let client = match self.git_client.as_ref() {
            Some(client) => client,
            None => return,
        };

        let commit_hash = self
            .commit_history
            .cached_commits
            .get(self.commit_history.selected_index)
            .and_then(|commit| {
                if commit.files_loaded {
                    None
                } else {
                    Some(commit.hash.clone())
                }
            });

        let Some(commit_hash) = commit_hash else {
            return;
        };

        match client.get_commit_files_changed(&commit_hash) {
            Ok(files) => {
                if let Some(commit) = self
                    .commit_history
                    .cached_commits
                    .get_mut(self.commit_history.selected_index)
                {
                    commit.files_changed = files;
                    commit.files_loaded = true;
                }
            }
            Err(e) => {
                self.report_git_error("Failed to load commit files", &e);
            }
        }
    }

    fn ensure_repo_ready(&mut self) -> bool {
        if let Some(report) = self.refresh_repo_health() {
            if report.has_blocking_issues() {
                let mut msg = report.summary();
                if let Some(hint) = report.inline_recovery_hint() {
                    msg = format!("{} | {}", msg, hint);
                }
                let msg = error(&msg);
                self.status_message = msg.clone();
                self.last_completion_message = Some(msg);
                return false;
            }
        }
        true
    }

    fn report_git_error(&mut self, context: &str, e: &color_eyre::eyre::Report) {
        let detail = git::GitClient::explain_error(e);
        let summary = detail.lines().next().unwrap_or(detail.as_str());
        let mut msg = format!("{}: {}", context, summary);
        let mut hint = git::GitClient::inline_recovery_hint(e);
        if hint.is_none() {
            if let Some(report) = self.refresh_repo_health() {
                hint = report.inline_recovery_hint();
            }
        }
        if let Some(hint) = hint {
            msg = format!("{} | {}", msg, hint);
        }
        let msg = error(&msg);
        self.status_message = msg.clone();
        self.last_completion_message = Some(msg);
    }

    fn enqueue_git_operation(&mut self, op: GitOperation) {
        if self.git_workdir.is_none() || self.git_client.is_none() {
            self.status_message = error("No Git repository");
            return;
        }

        if !self.ensure_repo_ready() {
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

    fn apply_completion_message(&mut self, msg: String, force: bool) {
        if self.settings.notifications || force {
            self.last_completion_message = Some(msg.clone());
            self.status_message = msg;
        } else {
            self.last_completion_message = None;
        }
    }

    fn notify_success_with_hint(&mut self, message: &str, hint: &str) {
        self.apply_completion_message(success(&format!("{} | Hint: {}", message, hint)), false);
    }

    fn notify_info_with_hint(&mut self, message: &str, hint: &str) {
        self.apply_completion_message(format!("ℹ {} | Hint: {}", message, hint), false);
    }

    fn request_delete_branch_confirmation(&mut self, name: &str) {
        self.pending_destructive_confirmation = Some(PendingDestructiveAction::DeleteBranch {
            name: name.to_string(),
        });
        self.notify_info_with_hint(
            &format!("Delete branch '{}'", name),
            "Press d again to confirm, or press Esc to cancel",
        );
    }

    fn request_drop_stash_confirmation(&mut self, index: usize, name: &str) {
        self.pending_destructive_confirmation = Some(PendingDestructiveAction::DropStash {
            index,
            name: name.to_string(),
        });
        self.notify_info_with_hint(
            &format!("Drop stash@{{{}}}: {}", index, name),
            "Press d again to confirm, or press Esc to cancel",
        );
    }

    fn clear_destructive_confirmation_if_unrelated(&mut self, update: &ActionStateUpdate) {
        let destructive_requested =
            update.branch_delete_requested.is_some() || update.stash_drop_requested.is_some();
        if !destructive_requested {
            self.pending_destructive_confirmation = None;
        }
    }

    fn maybe_autosync(&mut self, force: bool) {
        if !self.settings.autosync {
            return;
        }

        if !force {
            if let Some(last) = self.last_autosync_at {
                if last.elapsed() < AUTOSYNC_INTERVAL {
                    return;
                }
            }
        }

        if !self.pending_git_ops.is_empty() || self.git_client.is_none() {
            return;
        }

        self.refresh_remotes();
        if self.available_remotes.is_empty() {
            self.last_autosync_at = Some(Instant::now());
            return;
        }

        if self.selected_remote_index.is_none() {
            self.selected_remote_index = Some(0);
        }

        if let Some(remote) = self.selected_remote_name().map(|name| name.to_string()) {
            self.last_autosync_at = Some(Instant::now());
            self.enqueue_git_operation(GitOperation::Fetch(remote));
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

    fn handle_terminal_resize(&mut self) {
        let window_size = terminal::size()
            .map(|(_, rows)| Self::window_size_from_rows(rows))
            .unwrap_or(DEFAULT_WINDOW_SIZE);
        if window_size != self.window_size {
            self.window_size = window_size;
            self.reflow_scroll_for_window();
        }
    }

    fn window_size_from_rows(rows: u16) -> usize {
        // Reserve rows for borders, menu header, and status bar.
        let usable_rows = rows.saturating_sub(6);
        usize::from(usable_rows.max(1))
    }

    fn reflow_scroll_for_window(&mut self) {
        let window_size = self.window_size.max(1);

        let project_count = self.store.projects.len();
        self.dashboard.selected_index = self
            .dashboard
            .selected_index
            .min(project_count.saturating_sub(1));
        self.dashboard.scroll = self
            .dashboard
            .scroll
            .min(project_count.saturating_sub(window_size));
        crate::ui_utils::auto_scroll(
            self.dashboard.selected_index,
            &mut self.dashboard.scroll,
            window_size,
        );

        let (changes_len, modules_len, developers_len) = self
            .store
            .projects
            .get(self.dashboard.selected_index)
            .map(|project| {
                (
                    project.changes.len(),
                    project.modules.len(),
                    project.developers.len(),
                )
            })
            .unwrap_or((0, 0, 0));

        self.changes.selected_index = self
            .changes
            .selected_index
            .min(changes_len.saturating_sub(1));
        self.changes.scroll = self
            .changes
            .scroll
            .min(changes_len.saturating_sub(window_size));
        crate::ui_utils::auto_scroll(
            self.changes.selected_index,
            &mut self.changes.scroll,
            window_size,
        );

        self.merge.selected_file_index = self
            .merge
            .selected_file_index
            .min(changes_len.saturating_sub(1));
        self.merge.scroll = self
            .merge
            .scroll
            .min(changes_len.saturating_sub(window_size));
        crate::ui_utils::auto_scroll(
            self.merge.selected_file_index,
            &mut self.merge.scroll,
            window_size,
        );

        let commit_len = self.commit_history.cached_commits.len();
        self.commit_history.selected_index = self
            .commit_history
            .selected_index
            .min(commit_len.saturating_sub(1));
        self.commit_history.scroll = self
            .commit_history
            .scroll
            .min(commit_len.saturating_sub(window_size));
        crate::ui_utils::auto_scroll(
            self.commit_history.selected_index,
            &mut self.commit_history.scroll,
            window_size,
        );

        let stash_len = self.stashes.cached_stashes.len();
        self.stashes.selected_index = self.stashes.selected_index.min(stash_len.saturating_sub(1));
        self.stashes.scroll = self
            .stashes
            .scroll
            .min(stash_len.saturating_sub(window_size));
        crate::ui_utils::auto_scroll(
            self.stashes.selected_index,
            &mut self.stashes.scroll,
            window_size,
        );

        let branch_len = self.branch_manager.cached_branches.len();
        self.branch_manager.selected_index = self
            .branch_manager
            .selected_index
            .min(branch_len.saturating_sub(1));
        self.branch_manager.scroll = self
            .branch_manager
            .scroll
            .min(branch_len.saturating_sub(window_size));
        crate::ui_utils::auto_scroll(
            self.branch_manager.selected_index,
            &mut self.branch_manager.scroll,
            window_size,
        );

        self.module_manager.selected_module = self
            .module_manager
            .selected_module
            .min(modules_len.saturating_sub(1));
        self.module_manager.module_scroll = self
            .module_manager
            .module_scroll
            .min(modules_len.saturating_sub(window_size));
        crate::ui_utils::auto_scroll(
            self.module_manager.selected_module,
            &mut self.module_manager.module_scroll,
            window_size,
        );

        self.module_manager.selected_developer = self
            .module_manager
            .selected_developer
            .min(developers_len.saturating_sub(1));
        self.module_manager.developer_scroll = self
            .module_manager
            .developer_scroll
            .min(developers_len.saturating_sub(window_size));
        crate::ui_utils::auto_scroll(
            self.module_manager.selected_developer,
            &mut self.module_manager.developer_scroll,
            window_size,
        );
    }

    fn render(&mut self, frame: &mut Frame) {
        let settings_options = self.settings_options();
        let accepted_merge = self.merge.get_resolution(
            self.dashboard.selected_index,
            self.merge.selected_file_index,
        );
        let workdir = self.git_workdir.as_deref();
        let pending_git_ops_count = self.pending_git_ops.len();
        let selected_remote = self.selected_remote_name().map(|name| name.to_string());

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
            suggestions: &self.changes.suggestions,
            selected_suggestion: self.changes.selected_suggestion_index,
            no_suggestions_message: &self.changes.no_suggestions_message,
            changes_pane_ratio: self.changes.changes_pane_ratio,
            commit_pane_ratio: self.changes.commit_pane_ratio,
            dashboard_pane_ratio: self.dashboard.pane_ratio,
            menu_selected_index: self.menu_selected_index,
            focus: self.focus,
            selected_board_column: self.board.selected_column,
            selected_board_item: self.board.selected_item,
            merge_file_index: self.merge.selected_file_index,
            merge_focus: self.merge.focus,
            merge_conflicts: &self.merge.conflicts,
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
            selected_remote,
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
            selected_stash: self.stashes.selected_index,
            stash_scroll: self.stashes.scroll,
            cached_stashes: &self.stashes.cached_stashes,
            stash_mode: self.stashes.mode,
            stash_input_buffer: &self.stashes.input_buffer,
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
                format!(
                    "Commit History: {} commits (↑↓ Navigate, c Cherry-pick)",
                    count
                )
            }
            AppMode::Stashes => {
                let count = self.stashes.cached_stashes.len();
                if self.stashes.is_create_mode() {
                    format!("Stashes: {} (↵ Confirm, Esc Cancel)", count)
                } else {
                    format!(
                        "Stashes: {} (↑↓ Select, n New, a Apply, p Pop, d Drop)",
                        count
                    )
                }
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
                "Merge: {} (←→ Pane, ↑↓ File, Enter Accept)",
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
            input_mode: self.input_mode,
            current_view: self.current_view,
            show_help: self.show_help,
            search_active: self.search_active,
            menu_selected_index: self.menu_selected_index,
            menu_len: self.screen.menu_len(),
            selected_project_index: self.dashboard.selected_index,
            selected_change_index: self.changes.selected_index,
            selected_board_column: self.board.selected_column,
            selected_board_item: self.board.selected_item,
            selected_merge_file_index: self.merge.selected_file_index,
            selected_setting_index: self.selected_setting_index,
            commit_message_empty: self.changes.is_commit_message_empty(),
            suggestions_count: self.changes.suggestion_count(),
            has_git_client: self.git_client.is_some(),
            changes_pane_ratio: self.changes.changes_pane_ratio,
            commit_pane_ratio: self.changes.commit_pane_ratio,
            module_pane_ratio: self.module_manager.pane_ratio,
            dashboard_pane_ratio: self.dashboard.pane_ratio,
            // New view context
            selected_commit_index: self.commit_history.selected_index,
            selected_branch_index: self.branch_manager.selected_index,
            selected_stash_index: self.stashes.selected_index,
            selected_module_index: self.module_manager.selected_module,
            selected_developer_index: self.module_manager.selected_developer,
            cached_commits_len: self.commit_history.cached_commits.len(),
            cached_branches_len: self.branch_manager.cached_branches.len(),
            cached_stashes_len: self.stashes.cached_stashes.len(),
            branch_create_mode: matches!(self.branch_manager.mode, BranchManagerMode::CreateBranch),
            branch_input_empty: self.branch_manager.is_input_empty(),
            stash_create_mode: self.stashes.is_create_mode(),
            stash_input_empty: self.stashes.is_input_empty(),
            module_manager_in_developer_list: self.module_manager.is_developer_list(),
            module_create_mode: matches!(self.module_manager.mode, ModuleManagerMode::CreateModule),
            module_edit_mode: matches!(self.module_manager.mode, ModuleManagerMode::EditModule),
            developer_create_mode: matches!(
                self.module_manager.mode,
                ModuleManagerMode::CreateDeveloper
            ),
            module_assign_mode: self.module_manager.assign_mode,
            module_input_empty: self.module_manager.is_input_empty(),
            selected_remote: self.selected_remote_name().map(|name| name.to_string()),
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
        self.clear_destructive_confirmation_if_unrelated(&update);
        let window_size = self.window_size.max(1);
        // Apply all optional state updates
        if let Some(focus) = update.focus {
            self.focus = focus;
        }
        if let Some(mode) = update.input_mode {
            self.input_mode = mode;
        }
        if let Some(view) = update.current_view {
            let old_view = self.current_view;
            self.current_view = view;
            // Refresh caches when entering new views
            if old_view != view {
                self.refresh_view_cache();
            }
        }
        if update.current_view.is_some() || update.focus == Some(Focus::Menu) {
            self.input_mode = InputMode::Normal;
        }
        if let Some(help) = update.show_help {
            self.show_help = help;
        }
        if let Some(search) = update.search_active {
            self.search_active = search;
            if !search {
                self.input_mode = InputMode::Normal;
            }
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
            self.ensure_change_preview_loaded(self.changes.selected_index);
        }
        if let Some(idx) = update.selected_board_column {
            self.board.selected_column = idx;
        }
        if let Some(idx) = update.selected_board_item {
            self.board.selected_item = idx;
        }
        if let Some(idx) = update.selected_merge_file_index {
            self.merge.selected_file_index = idx;
            self.ensure_merge_conflict_preview_loaded(self.merge.selected_file_index);
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
            } else if self.commit_history.selected_index >= self.commit_history.scroll + window_size
            {
                self.commit_history.scroll = self
                    .commit_history
                    .selected_index
                    .saturating_sub(window_size - 1);
            }
            self.ensure_selected_commit_files_loaded();
        }
        if let Some(idx) = update.selected_stash_index {
            self.stashes.selected_index =
                idx.min(self.stashes.cached_stashes.len().saturating_sub(1));
            if self.stashes.selected_index < self.stashes.scroll {
                self.stashes.scroll = self.stashes.selected_index;
            } else if self.stashes.selected_index >= self.stashes.scroll + window_size {
                self.stashes.scroll = self.stashes.selected_index.saturating_sub(window_size - 1);
            }
        }
        if let Some(mode) = update.stash_create_mode {
            if mode {
                self.stashes.enter_create_mode();
            } else {
                self.stashes.exit_create_mode();
            }
        }
        if let Some(c) = update.stash_input_append {
            self.stashes.append_input_char(c);
        }
        if update.stash_input_pop.is_some() {
            self.stashes.pop_input_char();
        }
        if update.stash_input_clear.is_some() {
            self.stashes.clear_input();
        }
        if let Some(idx) = update.selected_branch_index {
            self.branch_manager.selected_index =
                idx.min(self.branch_manager.cached_branches.len().saturating_sub(1));
            // Auto-scroll to keep selection visible
            if self.branch_manager.selected_index < self.branch_manager.scroll {
                self.branch_manager.scroll = self.branch_manager.selected_index;
            } else if self.branch_manager.selected_index >= self.branch_manager.scroll + window_size
            {
                self.branch_manager.scroll = self
                    .branch_manager
                    .selected_index
                    .saturating_sub(window_size - 1);
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
                >= self.module_manager.module_scroll + window_size
            {
                self.module_manager.module_scroll = self
                    .module_manager
                    .selected_module
                    .saturating_sub(window_size - 1);
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
                >= self.module_manager.developer_scroll + window_size
            {
                self.module_manager.developer_scroll = self
                    .module_manager
                    .selected_developer
                    .saturating_sub(window_size - 1);
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
        if let Some(index) = update.apply_suggestion_index {
            if self.changes.apply_suggestion_at(index) {
                self.input_mode = InputMode::Typing;
                self.apply_completion_message(
                    success(&format!(
                        "Applied suggestion {}. Edit message and press Enter to commit",
                        index + 1
                    )),
                    false,
                );
            } else {
                self.apply_completion_message(
                    error(&format!("Suggestion {} is not available", index + 1)),
                    false,
                );
            }
        }
        if let Some(amount) = update.project_scroll_up {
            self.dashboard.scroll_up(amount);
        }
        if let Some(amount) = update.project_scroll_down {
            let max = self.store.projects.len();
            self.dashboard.scroll_down(amount, max, window_size);
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
            self.changes.scroll_down(amount, max, window_size);
        }
        if let Some(ratio) = update.changes_pane_ratio {
            self.changes.changes_pane_ratio = ratio;
            self.apply_completion_message(
                format!(
                    "Changes pane: {}% (Alt+←/→)",
                    self.changes.changes_pane_ratio
                ),
                false,
            );
        }
        if let Some(ratio) = update.commit_pane_ratio {
            self.changes.commit_pane_ratio = ratio;
            self.apply_completion_message(
                format!("Commit pane: {}% (Alt+←/→)", self.changes.commit_pane_ratio),
                false,
            );
        }
        if let Some(ratio) = update.module_pane_ratio {
            self.module_manager.pane_ratio = ratio;
            self.apply_completion_message(
                format!("Module pane: {}% (Alt+←/→)", self.module_manager.pane_ratio),
                false,
            );
        }
        if let Some(ratio) = update.dashboard_pane_ratio {
            self.dashboard.pane_ratio = ratio;
            self.apply_completion_message(
                format!("Dashboard pane: {}% (Alt+←/→)", self.dashboard.pane_ratio),
                false,
            );
        }
        if let Some(amount) = update.merge_scroll_up {
            self.merge.scroll_up(amount);
        }
        if let Some(amount) = update.merge_scroll_down {
            let max = self.merge.conflicts.len();
            self.merge.scroll_down(amount, max, window_size);
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
            let max = self.merge.conflicts.len();
            self.merge.navigate_down(max);
            self.ensure_merge_conflict_preview_loaded(self.merge.selected_file_index);
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
        if update.stash_create_requested.is_some() {
            self.perform_stash_create();
        }
        if update.stash_apply_requested.is_some() {
            self.perform_stash_apply();
        }
        if update.stash_pop_requested.is_some() {
            self.perform_stash_pop();
        }
        if update.stash_drop_requested.is_some() {
            self.perform_stash_drop();
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
        if update.cherry_pick_requested.is_some() {
            self.perform_cherry_pick();
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }

    fn clamp_selections_for_project(&mut self) {
        // When switching projects, ensure selections are valid for the new project
        if let Some(project) = self.store.projects.get(self.dashboard.selected_index) {
            self.changes.clamp_selection(project.changes.len());
            self.merge.clamp_selection(self.merge.conflicts.len());
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
        enum MergeAcceptError {
            Status(String),
            Git(String),
            Abort,
        }

        let mut merge_state = std::mem::take(&mut self.merge);

        let result: Result<&'static str, MergeAcceptError> = (|| {
            if matches!(merge_state.focus, MergePaneFocus::Files) {
                return Err(MergeAcceptError::Status(
                    "Selected file for merge".to_string(),
                ));
            }

            let path = merge_state
                .conflicts
                .get(merge_state.selected_file_index)
                .map(|change| change.path.clone())
                .ok_or_else(|| {
                    MergeAcceptError::Status("No file selected for merge".to_string())
                })?;

            if !self.ensure_repo_ready() {
                return Err(MergeAcceptError::Abort);
            }

            let side = match merge_state.focus {
                MergePaneFocus::Local => git::ConflictSide::Ours,
                MergePaneFocus::Incoming => git::ConflictSide::Theirs,
                MergePaneFocus::Files => {
                    return Err(MergeAcceptError::Status(
                        "Selected file for merge".to_string(),
                    ));
                }
            };

            let resolve_error = if let Some(client) = self.git_client.take() {
                let result = client.resolve_conflict(&path, side);
                self.git_client = Some(client);
                result.err().map(|e| e.to_string())
            } else {
                return Err(MergeAcceptError::Status("No Git repository".to_string()));
            };

            if let Some(err_msg) = resolve_error {
                return Err(MergeAcceptError::Git(err_msg));
            }

            Ok(merge_state
                .accept_current_pane(self.dashboard.selected_index)
                .unwrap_or("Resolved merge conflict"))
        })();

        self.merge = merge_state;

        match result {
            Ok(msg) => {
                self.status_message = success(msg);
                if let Err(e) = self.refresh_changes_summary(false) {
                    self.report_git_error("Failed to list changes", &e);
                }
                if let Err(e) = self.refresh_merge_conflicts() {
                    self.report_git_error("Failed to list merge conflicts", &e);
                }
            }
            Err(MergeAcceptError::Status(msg)) => {
                self.status_message = msg;
            }
            Err(MergeAcceptError::Git(err_msg)) => {
                let report = color_eyre::eyre::eyre!(err_msg);
                self.report_git_error("Merge resolution failed", &report);
            }
            Err(MergeAcceptError::Abort) => {}
        }
    }

    fn toggle_setting(&mut self) {
        match self.selected_setting_index {
            0 => {
                self.cycle_remote();
            }
            1 => {
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
                self.persist_settings();
            }
            2 => {
                self.settings.notifications = !self.settings.notifications;
                self.status_message = format!(
                    "⚙ Notifications: {}",
                    if self.settings.notifications {
                        "On"
                    } else {
                        "Off"
                    }
                );
                if !self.settings.notifications {
                    self.last_completion_message = None;
                }
                self.persist_settings();
            }
            3 => {
                self.settings.autosync = !self.settings.autosync;
                self.status_message = format!(
                    "⚙ Autosync: {}",
                    if self.settings.autosync { "On" } else { "Off" }
                );
                self.persist_settings();
                if self.settings.autosync {
                    self.maybe_autosync(true);
                }
            }
            4 => {
                self.settings.suggestions.enabled = !self.settings.suggestions.enabled;
                self.status_message = format!(
                    "⚙ Suggestions: {}",
                    if self.settings.suggestions.enabled {
                        "On"
                    } else {
                        "Off"
                    }
                );
                self.regenerate_commit_suggestions();
                self.persist_settings();
            }
            5 => {
                self.settings.suggestions.max_suggestions =
                    (self.settings.suggestions.max_suggestions % 5) + 1;
                self.status_message = format!(
                    "⚙ Max suggestions: {}",
                    self.settings.suggestions.max_suggestions
                );
                self.regenerate_commit_suggestions();
                self.persist_settings();
            }
            6 => {
                self.settings.suggestions.max_length = match self.settings.suggestions.max_length {
                    50 => 72,
                    72 => 100,
                    100 => 120,
                    _ => 50,
                };
                self.status_message = format!(
                    "⚙ Suggestion length: {}",
                    self.settings.suggestions.max_length
                );
                self.regenerate_commit_suggestions();
                self.persist_settings();
            }
            _ => {}
        }
    }

    fn perform_commit(&mut self) {
        let msg = self.changes.commit_message.trim().to_string();
        if !self.ensure_repo_ready() {
            return;
        }
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

            match client.commit_all(&msg) {
                Ok(_oid) => {
                    // Refresh changes and bump progress
                    if let Ok(changes) = client.list_changes_summary() {
                        if let Some(project) =
                            self.store.projects.get_mut(self.dashboard.selected_index)
                        {
                            project.changes = changes;
                            self.ensure_change_preview_loaded(self.changes.selected_index);
                            self.regenerate_commit_suggestions();
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
                    self.report_git_error("Commit failed", &e);
                }
            }
        }
    }

    fn refresh_view_cache(&mut self) {
        if !self.ensure_repo_ready() {
            return;
        }
        let mut error: Option<(String, color_eyre::eyre::Report)> = None;

        match self.current_view {
            AppMode::BranchManager => match self.load_branch_infos() {
                Ok(branch_infos) => {
                    self.branch_manager.update_branches(branch_infos);
                }
                Err(e) => {
                    error = Some(("Failed to list branches".to_string(), e));
                }
            },
            AppMode::CommitHistory => match self.load_commit_history(50) {
                Ok(commit_infos) => {
                    self.commit_history.update_commits(commit_infos);
                    self.ensure_selected_commit_files_loaded();
                }
                Err(e) => {
                    error = Some(("Failed to load commit history".to_string(), e));
                }
            },
            AppMode::Stashes => match self.load_stashes() {
                Ok(stashes) => {
                    self.stashes.update_stashes(stashes);
                }
                Err(e) => {
                    error = Some(("Failed to list stashes".to_string(), e));
                }
            },
            AppMode::Changes => {
                // Refresh changes when entering the view
                let refresh_result = self.refresh_changes_summary(true);
                if let Err(e) = refresh_result {
                    error = Some(("Failed to list changes".to_string(), e));
                } else {
                    self.ensure_change_preview_loaded(self.changes.selected_index);
                }
            }
            _ => {}
        }

        if matches!(self.current_view, AppMode::MergeVisualizer) {
            if let Err(e) = self.refresh_merge_conflicts() {
                error = Some(("Failed to list merge conflicts".to_string(), e));
            }
        }

        if let Some((context, err)) = error {
            self.report_git_error(&context, &err);
        }
    }

    fn refresh_merge_conflicts(&mut self) -> color_eyre::Result<()> {
        let conflicts = self.get_merge_conflicts()?;
        self.merge.conflicts = conflicts;
        self.merge.clamp_selection(self.merge.conflicts.len());
        self.ensure_merge_conflict_preview_loaded(self.merge.selected_file_index);
        Ok(())
    }

    fn get_merge_conflicts(&self) -> color_eyre::Result<Vec<crate::data::Change>> {
        let client = match self.git_client.as_ref() {
            Some(client) => client,
            None => return Ok(Vec::new()),
        };
        client.list_merge_conflicts_summary()
    }

    fn refresh_changes_summary(&mut self, update_branch: bool) -> color_eyre::Result<()> {
        let (changes, branch) = match self.git_client.as_ref() {
            Some(client) => (client.list_changes_summary()?, client.head_branch()),
            None => return Ok(()),
        };

        if let Some(project) = self.store.projects.get_mut(self.dashboard.selected_index) {
            project.changes = changes;
            if update_branch {
                project.branch = branch.unwrap_or_default();
            }
        }

        self.regenerate_commit_suggestions();

        Ok(())
    }

    fn regenerate_commit_suggestions(&mut self) {
        if !self.settings.suggestions.enabled {
            self.changes.clear_suggestions();
            self.changes
                .set_no_suggestions_message("Suggestions are disabled in Settings");
            return;
        }

        let Some(project) = self.store.projects.get(self.dashboard.selected_index) else {
            self.changes.clear_suggestions();
            self.changes
                .set_no_suggestions_message("No active project selected");
            return;
        };

        let cache_key = self.build_suggestion_cache_key(project);
        if let Some(cache) = &self.suggestion_cache {
            if cache.key == cache_key {
                if cache.suggestions.is_empty() {
                    self.changes.clear_suggestions();
                    self.changes
                        .set_no_suggestions_message(cache.no_suggestions_message.clone());
                } else {
                    self.changes.set_suggestions(cache.suggestions.clone());
                }
                return;
            }
        }

        let diff_summary = suggestions::DiffSummary::from_changes(&project.changes);
        if diff_summary.is_empty() {
            self.changes.clear_suggestions();
            self.changes
                .set_no_suggestions_message("Stage files to see commit suggestions");
            self.suggestion_cache = Some(SuggestionCacheEntry {
                key: cache_key,
                suggestions: Vec::new(),
                no_suggestions_message: "Stage files to see commit suggestions".to_string(),
            });
            return;
        }

        let branch_name = if project.branch.trim().is_empty() {
            None
        } else {
            Some(project.branch.clone())
        };
        let context = suggestions::CommitContext::new(diff_summary, branch_name);
        let engine = suggestions::RuleBasedEngine::new();
        let suggestions = engine.suggest(&context, &self.settings.suggestions);

        if suggestions.is_empty() {
            self.changes.clear_suggestions();
            self.changes.set_no_suggestions_message(
                "No high-confidence suggestions for current staged changes",
            );
            self.suggestion_cache = Some(SuggestionCacheEntry {
                key: cache_key,
                suggestions: Vec::new(),
                no_suggestions_message: "No high-confidence suggestions for current staged changes"
                    .to_string(),
            });
        } else {
            self.changes.set_suggestions(suggestions.clone());
            self.suggestion_cache = Some(SuggestionCacheEntry {
                key: cache_key,
                suggestions,
                no_suggestions_message: String::new(),
            });
        }
    }

    fn build_suggestion_cache_key(&self, project: &data::Project) -> SuggestionCacheKey {
        let mut hasher = DefaultHasher::new();

        for change in &project.changes {
            if !change.staged {
                continue;
            }

            change.path.hash(&mut hasher);
            change.diff_preview.hash(&mut hasher);
            let status_marker = match change.status {
                data::FileStatus::Modified => "M",
                data::FileStatus::Added => "A",
                data::FileStatus::Deleted => "D",
            };
            status_marker.hash(&mut hasher);
        }

        SuggestionCacheKey {
            project_index: self.dashboard.selected_index,
            branch_name: project.branch.clone(),
            staged_fingerprint: hasher.finish(),
            max_suggestions: self.settings.suggestions.max_suggestions,
            max_length: self.settings.suggestions.max_length,
        }
    }

    fn load_branch_infos(&self) -> color_eyre::Result<Vec<BranchInfo>> {
        let client = self
            .git_client
            .as_ref()
            .ok_or_else(|| color_eyre::eyre::eyre!("No Git repository"))?;
        let branches = client.list_branches_with_upstream(true, true)?;
        let branch_infos = branches
            .into_iter()
            .map(|(name, is_current, is_remote, upstream)| {
                let upstream_status = if !is_remote {
                    upstream.as_ref().and_then(|_| {
                        client
                            .get_ahead_behind(&name)
                            .ok()
                            .flatten()
                            .map(|(ahead, behind)| UpstreamStatus { ahead, behind })
                    })
                } else {
                    None
                };

                BranchInfo {
                    name,
                    is_current,
                    is_remote,
                    upstream,
                    upstream_status,
                }
            })
            .collect();

        Ok(branch_infos)
    }

    fn load_commit_history(&self, limit: usize) -> color_eyre::Result<Vec<CommitInfo>> {
        let client = self
            .git_client
            .as_ref()
            .ok_or_else(|| color_eyre::eyre::eyre!("No Git repository"))?;
        let commits = client.get_commit_history_summary(limit)?;
        Ok(commits
            .into_iter()
            .map(|(hash, author, date, message, files)| CommitInfo {
                hash,
                author,
                date,
                message,
                files_changed: files,
                files_loaded: false,
            })
            .collect())
    }

    fn load_stashes(&self) -> color_eyre::Result<Vec<StashInfo>> {
        let client = self
            .git_client
            .as_ref()
            .ok_or_else(|| color_eyre::eyre::eyre!("No Git repository"))?;
        let stashes = client.list_stashes()?;
        Ok(stashes
            .into_iter()
            .map(|stash| StashInfo {
                index: stash.index,
                name: stash.name,
                oid: stash.oid,
            })
            .collect())
    }

    fn perform_branch_switch(&mut self) {
        if !self.ensure_repo_ready() {
            return;
        }
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
                        self.notify_success_with_hint(
                            &format!("Switched to branch: {}", name),
                            "Press Tab to review changes, or n in Branches to create a new branch",
                        );
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
                        self.report_git_error("Failed to switch branch", &e);
                    }
                }
            }
        }
    }

    fn perform_branch_create(&mut self) {
        if !self.ensure_repo_ready() {
            return;
        }
        let branch_name = self.branch_manager.get_input_value();
        if let Some(client) = &self.git_client {
            match client.create_branch(branch_name) {
                Ok(()) => {
                    self.notify_success_with_hint(
                        &format!("Created branch: {}", branch_name),
                        "Use Enter to switch to it when ready",
                    );
                    self.branch_manager.exit_create_mode();
                    // Refresh branch list
                    self.refresh_view_cache();
                }
                Err(e) => {
                    self.report_git_error("Failed to create branch", &e);
                }
            }
        }
    }

    fn perform_branch_delete(&mut self) {
        if !self.ensure_repo_ready() {
            return;
        }
        let branch_info = self
            .branch_manager
            .selected_branch()
            .map(|b| (b.name.clone(), b.is_current));

        if let Some((name, is_current)) = branch_info {
            if is_current {
                self.notify_info_with_hint(
                    "Cannot delete current branch",
                    "Switch to another branch first, then delete",
                );
                return;
            }

            let confirmed = matches!(
                self.pending_destructive_confirmation.as_ref(),
                Some(PendingDestructiveAction::DeleteBranch { name: pending_name }) if pending_name == &name
            );

            if !confirmed {
                self.request_delete_branch_confirmation(&name);
                return;
            }

            self.pending_destructive_confirmation = None;

            if let Some(client) = &self.git_client {
                match client.delete_branch(&name) {
                    Ok(()) => {
                        self.notify_success_with_hint(
                            &format!("Deleted branch: {}", name),
                            "Press r to refresh branch list if needed",
                        );
                        // Refresh branch list
                        self.refresh_view_cache();
                    }
                    Err(e) => {
                        self.report_git_error("Failed to delete branch", &e);
                    }
                }
            }
        }
    }

    fn perform_stash_create(&mut self) {
        if !self.ensure_repo_ready() {
            return;
        }
        let message = self.stashes.get_input_value().to_string();
        if let Some(client) = &self.git_client {
            match client.create_stash(&message) {
                Ok(_oid) => {
                    self.notify_success_with_hint(
                        &format!("Created stash: {}", message),
                        "Use a to apply or p to pop from Stashes view",
                    );
                    self.stashes.exit_create_mode();
                    self.refresh_view_cache();
                    if let Err(e) = self.refresh_changes_summary(true) {
                        self.report_git_error("Failed to refresh changes", &e);
                    }
                }
                Err(e) => {
                    self.report_git_error("Failed to create stash", &e);
                }
            }
        }
    }

    fn perform_stash_apply(&mut self) {
        if !self.ensure_repo_ready() {
            return;
        }
        let stash = match self.stashes.cached_stashes.get(self.stashes.selected_index) {
            Some(stash) => stash,
            None => {
                self.notify_info_with_hint(
                    "No stash selected",
                    "Use ↑/↓ to select a stash entry first",
                );
                return;
            }
        };

        if let Some(client) = &self.git_client {
            match client.apply_stash(stash.index) {
                Ok(()) => {
                    self.notify_success_with_hint(
                        &format!("Applied stash: stash@{{{}}}: {}", stash.index, stash.name),
                        "Review changes and commit or stash again as needed",
                    );
                    if let Err(e) = self.refresh_changes_summary(false) {
                        self.report_git_error("Failed to refresh changes", &e);
                    }
                }
                Err(e) => {
                    self.report_git_error("Failed to apply stash", &e);
                }
            }
        }
    }

    fn perform_stash_pop(&mut self) {
        if !self.ensure_repo_ready() {
            return;
        }
        let stash = match self.stashes.cached_stashes.get(self.stashes.selected_index) {
            Some(stash) => stash,
            None => {
                self.notify_info_with_hint(
                    "No stash selected",
                    "Use ↑/↓ to select a stash entry first",
                );
                return;
            }
        };

        if let Some(client) = &self.git_client {
            match client.pop_stash(stash.index) {
                Ok(()) => {
                    self.notify_success_with_hint(
                        &format!("Popped stash: stash@{{{}}}: {}", stash.index, stash.name),
                        "The stash is removed; review and commit recovered changes",
                    );
                    self.refresh_view_cache();
                    if let Err(e) = self.refresh_changes_summary(false) {
                        self.report_git_error("Failed to refresh changes", &e);
                    }
                }
                Err(e) => {
                    self.report_git_error("Failed to pop stash", &e);
                }
            }
        }
    }

    fn perform_stash_drop(&mut self) {
        if !self.ensure_repo_ready() {
            return;
        }
        let stash = match self.stashes.cached_stashes.get(self.stashes.selected_index) {
            Some(stash) => (stash.index, stash.name.clone()),
            None => {
                self.notify_info_with_hint(
                    "No stash selected",
                    "Use ↑/↓ to select a stash entry first",
                );
                return;
            }
        };

        let confirmed = matches!(
            self.pending_destructive_confirmation.as_ref(),
            Some(PendingDestructiveAction::DropStash { index, name })
                if *index == stash.0 && name == &stash.1
        );

        if !confirmed {
            self.request_drop_stash_confirmation(stash.0, &stash.1);
            return;
        }

        self.pending_destructive_confirmation = None;

        if let Some(client) = &self.git_client {
            match client.drop_stash(stash.0) {
                Ok(()) => {
                    self.notify_success_with_hint(
                        &format!("Dropped stash: stash@{{{}}}: {}", stash.0, stash.1),
                        "Use n to create a stash before risky changes",
                    );
                    self.refresh_view_cache();
                }
                Err(e) => {
                    self.report_git_error("Failed to drop stash", &e);
                }
            }
        }
    }

    fn perform_cherry_pick(&mut self) {
        if !self.ensure_repo_ready() {
            return;
        }

        let commit = match self
            .commit_history
            .cached_commits
            .get(self.commit_history.selected_index)
        {
            Some(commit) => commit,
            None => {
                self.status_message = "No commit selected".into();
                return;
            }
        };

        if let Some(client) = &self.git_client {
            match client.cherry_pick_commit(&commit.hash) {
                Ok(new_oid) => {
                    self.status_message = success(&format!(
                        "Cherry-picked {} (new {})",
                        &commit.hash[..commit.hash.len().min(7)],
                        &new_oid[..new_oid.len().min(7)]
                    ));
                    if let Err(e) = self.refresh_changes_summary(true) {
                        self.report_git_error("Failed to refresh changes", &e);
                    }
                    self.refresh_view_cache();
                }
                Err(e) => {
                    let message = e.to_string();
                    if message.to_lowercase().contains("conflict") {
                        self.merge.clear_resolutions();
                        self.merge.focus = MergePaneFocus::Files;
                        self.merge.selected_file_index = 0;
                        self.merge.scroll = 0;
                        self.current_view = AppMode::MergeVisualizer;
                        self.menu_selected_index = AppMode::MergeVisualizer.menu_index();
                        self.focus = Focus::View;
                        self.input_mode = InputMode::Normal;
                        if let Err(err) = self.refresh_merge_conflicts() {
                            self.report_git_error("Failed to list merge conflicts", &err);
                        } else {
                            self.status_message = error(
                                "Cherry-pick conflict. Resolve in Merge view, then commit from Changes.",
                            );
                        }
                    } else {
                        self.report_git_error("Cherry-pick failed", &e);
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
            self.status_message = error(&format!("Failed to create module '{}'", module_name));
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
                self.status_message = error(&format!("Failed to update module '{}'", module_name));
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
                    self.status_message =
                        error(&format!("Failed to delete module '{}'", module_name));
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
            self.status_message =
                error(&format!("Failed to create developer '{}'", developer_name));
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
                    self.status_message =
                        error(&format!("Failed to delete developer '{}'", developer_name));
                }
            }
        }
    }

    fn toggle_file_staging(&mut self) {
        if !self.ensure_repo_ready() {
            return;
        }
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
                            match client.list_changes_summary() {
                                Ok(changes) => {
                                    project.changes = changes;
                                    self.ensure_change_preview_loaded(self.changes.selected_index);
                                    self.regenerate_commit_suggestions();
                                    self.status_message = if is_staged {
                                        success(&format!("Unstaged: {}", path))
                                    } else {
                                        success(&format!("Staged: {}", path))
                                    };
                                }
                                Err(e) => {
                                    self.report_git_error("Failed to refresh changes", &e);
                                }
                            }
                        }
                        Err(e) => {
                            let action = if is_staged { "Unstage" } else { "Stage" };
                            self.report_git_error(&format!("{} failed for {}", action, path), &e);
                        }
                    }
                }
            }
        }
    }

    fn perform_fetch(&mut self) {
        if let Some(remote) = self.ensure_remote_selected("Fetch") {
            self.enqueue_git_operation(GitOperation::Fetch(remote));
        }
    }

    fn perform_push(&mut self) {
        if let Some(remote) = self.ensure_remote_selected("Push") {
            self.enqueue_git_operation(GitOperation::Push(remote));
        }
    }

    fn perform_pull(&mut self) {
        if let Some(remote) = self.ensure_remote_selected("Pull") {
            self.enqueue_git_operation(GitOperation::Pull(remote));
        }
    }

    fn perform_module_assignment(&mut self) {
        if let Some(project) = self.store.projects.get_mut(self.dashboard.selected_index) {
            if let Some(module) = project.modules.get(self.module_manager.selected_module) {
                let module_id = module.id;
                let module_name = module.name.clone();
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
                        self.status_message = success(&format!(
                            "Assigned {} to module {}",
                            developer_name, module_name
                        ));
                        self.module_manager.assign_mode = false;
                        if let Some(wd) = self.git_workdir.as_ref() {
                            let _ = self.store.save_to_json(wd);
                        }
                    } else {
                        self.status_message = error(&format!(
                            "Failed to assign {} to module {}",
                            developer_name, module_name
                        ));
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
    Stashes,
    CommitHistory,
    BranchManager,
    MergeVisualizer,
    ProjectBoard,
    ModuleManager,
    Settings,
}

impl App {
    fn settings_options(&self) -> Vec<String> {
        let remote_label = match self.selected_remote_name() {
            Some(name) => format!("Remote: {}", name),
            None => "Remote: (none)".to_string(),
        };
        vec![
            remote_label,
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
            format!(
                "Suggestions: {}",
                if self.settings.suggestions.enabled {
                    "On"
                } else {
                    "Off"
                }
            ),
            format!(
                "Suggestion Max Count: {}",
                self.settings.suggestions.max_suggestions
            ),
            format!(
                "Suggestion Max Length: {}",
                self.settings.suggestions.max_length
            ),
        ]
    }

    fn refresh_remotes(&mut self) {
        let remotes = self
            .git_client
            .as_ref()
            .and_then(|client| client.list_remotes().ok())
            .unwrap_or_default();
        self.available_remotes = remotes;
        self.selected_remote_index = self
            .available_remotes
            .iter()
            .position(|name| name == "origin")
            .or_else(|| {
                if self.available_remotes.is_empty() {
                    None
                } else {
                    Some(0)
                }
            });
    }

    fn selected_remote_name(&self) -> Option<&str> {
        self.selected_remote_index
            .and_then(|idx| self.available_remotes.get(idx))
            .map(String::as_str)
    }

    fn cycle_remote(&mut self) {
        if self.git_client.is_some() {
            self.refresh_remotes();
        }
        if self.available_remotes.is_empty() {
            self.status_message = "⚙ No remotes configured".to_string();
            return;
        }

        let next_index = match self.selected_remote_index {
            Some(idx) => (idx + 1) % self.available_remotes.len(),
            None => 0,
        };
        self.selected_remote_index = Some(next_index);
        if let Some(name) = self.selected_remote_name() {
            self.status_message = format!("⚙ Remote set to {}", name);
        }
    }

    fn ensure_remote_selected(&mut self, action: &str) -> Option<String> {
        if self.git_client.is_some() {
            self.refresh_remotes();
        }
        if self.available_remotes.is_empty() {
            self.status_message = format!("{} failed: no remotes configured", action);
            return None;
        }
        if self.selected_remote_index.is_none() {
            self.selected_remote_index = Some(0);
        }
        self.selected_remote_name().map(|name| name.to_string())
    }

    fn persist_settings(&mut self) {
        if let Some(workdir) = self.git_workdir.as_ref() {
            if let Err(err) = self.save_settings_to(workdir) {
                self.status_message = error(&format!("Failed to save settings: {}", err));
            }
        }
    }

    fn save_settings_to(&self, workdir: &std::path::Path) -> std::io::Result<()> {
        use std::fs;

        let dir = workdir.join(".forge");
        fs::create_dir_all(&dir)?;
        let contents = serde_json::to_string_pretty(&self.settings)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        fs::write(dir.join("settings.json"), contents)?;
        Ok(())
    }

    fn load_settings_from(
        &self,
        workdir: &std::path::Path,
    ) -> std::io::Result<Option<AppSettings>> {
        use std::fs;

        let path = workdir.join(".forge").join("settings.json");
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&contents).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("settings.json is not valid JSON: {}", err),
            )
        })?;

        Self::validate_settings_schema(&value)?;

        let settings = serde_json::from_value(value).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("settings.json has invalid fields: {}", err),
            )
        })?;
        Ok(Some(settings))
    }

    fn validate_settings_schema(value: &serde_json::Value) -> std::io::Result<()> {
        let object = value.as_object().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "settings.json must be a JSON object",
            )
        })?;

        let mut errors = Vec::new();

        match object.get("theme").and_then(serde_json::Value::as_str) {
            Some("default") | Some("high_contrast") => {}
            Some(other) => errors.push(format!(
                "theme must be 'default' or 'high_contrast' (found '{}')",
                other
            )),
            None => errors.push("missing required field: theme".to_string()),
        }

        if !object
            .get("notifications")
            .is_some_and(serde_json::Value::is_boolean)
        {
            errors.push("notifications must be a boolean".to_string());
        }

        if !object
            .get("autosync")
            .is_some_and(serde_json::Value::is_boolean)
        {
            errors.push("autosync must be a boolean".to_string());
        }

        if let Some(suggestions) = object.get("suggestions") {
            if let Some(suggestions_obj) = suggestions.as_object() {
                if !suggestions_obj
                    .get("enabled")
                    .is_some_and(serde_json::Value::is_boolean)
                {
                    errors.push("suggestions.enabled must be a boolean".to_string());
                }

                match suggestions_obj
                    .get("max_suggestions")
                    .and_then(serde_json::Value::as_u64)
                {
                    Some(v) if (1..=5).contains(&v) => {}
                    Some(v) => errors.push(format!(
                        "suggestions.max_suggestions must be between 1 and 5 (found {})",
                        v
                    )),
                    None => {
                        errors.push("suggestions.max_suggestions must be an integer".to_string())
                    }
                }

                match suggestions_obj
                    .get("max_length")
                    .and_then(serde_json::Value::as_u64)
                {
                    Some(v) if (20..=200).contains(&v) => {}
                    Some(v) => errors.push(format!(
                        "suggestions.max_length must be between 20 and 200 (found {})",
                        v
                    )),
                    None => errors.push("suggestions.max_length must be an integer".to_string()),
                }
            } else {
                errors.push("suggestions must be an object".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("settings schema validation failed: {}", errors.join("; ")),
            ))
        }
    }
}

impl AppMode {
    pub fn next(self) -> Self {
        use AppMode::*;
        match self {
            Dashboard => Changes,
            Changes => Stashes,
            Stashes => CommitHistory,
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
            AppMode::Stashes => 2,
            AppMode::CommitHistory => 3,
            AppMode::BranchManager => 4,
            AppMode::MergeVisualizer => 5,
            AppMode::ProjectBoard => 6,
            AppMode::ModuleManager => 7,
            AppMode::Settings => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SuggestionCacheKey {
    project_index: usize,
    branch_name: String,
    staged_fingerprint: u64,
    max_suggestions: usize,
    max_length: usize,
}

#[derive(Debug, Clone)]
struct SuggestionCacheEntry {
    key: SuggestionCacheKey,
    suggestions: Vec<suggestions::CommitSuggestion>,
    no_suggestions_message: String,
}
