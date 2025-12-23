use ratatui::{DefaultTerminal, Frame};

pub mod data;
pub mod key_handler;
pub mod pages;
pub mod screen;
use data::ModuleStatus;
use key_handler::{KeyAction, KeyHandler};
use pages::merge_visualizer::MergePaneFocus;
use screen::Screen;

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

#[derive(Debug)]
pub struct App {
    running: bool,
    screen: Screen,
    key_handler: KeyHandler,
    current_view: AppMode,
    prev_view: AppMode,
    focus: Focus,
    menu_selected_index: usize,
    selected_project: Option<String>,
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
}

impl App {
    pub fn new() -> Self {
        Self {
            running: false,
            screen: Screen::new(),
            key_handler: KeyHandler::new(),
            current_view: AppMode::Dashboard,
            prev_view: AppMode::Dashboard,
            focus: Focus::View,
            menu_selected_index: 0,
            selected_project: None,
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
        }
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
                ["Pending", "Current", "Completed"][self.selected_board_column]
            ),
            AppMode::MergeVisualizer => format!(
                "Merge: {} (←→ Pane, ↑↓ File)",
                ["Files", "Local", "Incoming"][match self.merge_focus {
                    MergePaneFocus::Files => 0,
                    MergePaneFocus::Local => 1,
                    MergePaneFocus::Incoming => 2,
                }]
            ),
            AppMode::Settings => format!(
                "Settings: {} (↑↓ Select)",
                crate::pages::settings::SETTINGS_OPTIONS
                    .get(self.selected_setting_index)
                    .unwrap_or(&"N/A")
            ),
        };
    }

    fn handle_action(&mut self, action: KeyAction) -> bool {
        match action {
            KeyAction::Quit => true,
            KeyAction::Help => {
                self.show_help = !self.show_help;
                false
            }
            KeyAction::Back => {
                if self.show_help {
                    self.show_help = false;
                    return false;
                }
                if self.search_active {
                    self.search_active = false;
                    self.search_buffer.clear();
                    self.selected_project_index = 0;
                    self.update_status_message();
                    return false;
                }
                if self.focus == Focus::Menu {
                    return true; // Exit from main menu
                }
                self.focus = Focus::Menu;
                self.status_message = "Menu: Tab to navigate, ↵ to select, q to quit".to_string();
                false
            }
            KeyAction::NextView => {
                if self.focus == Focus::Menu {
                    // Tab cycles menu items
                    let menu_len = self.screen.get_menu_items_count();
                    self.menu_selected_index = (self.menu_selected_index + 1) % menu_len;
                } else {
                    // Tab cycles views (but only when in View focus)
                    self.prev_view = self.current_view;
                    self.current_view = self.current_view.next();
                    // Sync menu selection to current view
                    self.menu_selected_index = self.current_view.menu_index();
                    self.update_status_message();
                }
                false
            }
            KeyAction::Select => {
                if self.focus == Focus::Menu {
                    // Enter on menu switches to that view
                    if let Some(item) = self
                        .screen
                        .get_selected_menu_item_by_index(self.menu_selected_index)
                    {
                        match item {
                            "Dashboard" => self.current_view = AppMode::Dashboard,
                            "Changes" => self.current_view = AppMode::Changes,
                            "Merge" => self.current_view = AppMode::MergeVisualizer,
                            "Board" => self.current_view = AppMode::ProjectBoard,
                            "Settings" => self.current_view = AppMode::Settings,
                            "Exit" => return true,
                            _ => {}
                        }
                    }
                    self.focus = Focus::View;
                    self.update_status_message();
                } else if matches!(self.current_view, AppMode::Changes) {
                    // Enter on Changes view commits
                    self.store
                        .bump_progress_on_commit(self.selected_project_index);
                    self.status_message = format!("✓ Committed: {}", self.commit_message);
                    self.commit_message.clear();
                    self.update_status_message();
                } else if matches!(self.current_view, AppMode::ProjectBoard) {
                    // Enter on Board moves item to next column
                    self.move_board_item_to_next_status();
                } else if matches!(self.current_view, AppMode::MergeVisualizer) {
                    // Enter on Merge accepts current pane's version
                    self.accept_merge_pane();
                } else if matches!(self.current_view, AppMode::Settings) {
                    // Enter on Settings toggles setting
                    self.toggle_setting();
                }
                false
            }
            // Navigation within views
            KeyAction::NavigateUp => {
                if self.focus == Focus::Menu {
                    if self.menu_selected_index > 0 {
                        self.menu_selected_index -= 1;
                    }
                } else {
                    match self.current_view {
                        AppMode::Dashboard => {
                            if self.selected_project_index > 0 {
                                self.selected_project_index -= 1;
                                self.clamp_selections_for_project();
                            }
                        }
                        AppMode::Changes => {
                            if self.selected_change_index > 0 {
                                self.selected_change_index -= 1;
                            }
                        }
                        AppMode::ProjectBoard => {
                            let len = self.board_column_len(self.selected_board_column);
                            if len == 0 {
                                self.selected_board_item = 0;
                            } else if self.selected_board_item > 0 {
                                self.selected_board_item -= 1;
                            } else {
                                self.selected_board_item = len - 1;
                            }
                        }
                        AppMode::MergeVisualizer => {
                            if self.selected_merge_file_index > 0 {
                                self.selected_merge_file_index -= 1;
                            }
                        }
                        AppMode::Settings => {
                            if self.selected_setting_index > 0 {
                                self.selected_setting_index -= 1;
                            }
                        }
                    }
                    self.update_status_message();
                }
                false
            }
            KeyAction::NavigateDown => {
                if self.focus == Focus::Menu {
                    let menu_len = self.screen.get_menu_items_count();
                    if self.menu_selected_index < menu_len.saturating_sub(1) {
                        self.menu_selected_index += 1;
                    }
                } else {
                    match self.current_view {
                        AppMode::Dashboard => {
                            let max = self.store.projects.len().saturating_sub(1);
                            if self.selected_project_index < max {
                                self.selected_project_index += 1;
                                self.clamp_selections_for_project();
                            }
                        }
                        AppMode::Changes => {
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
                        AppMode::ProjectBoard => {
                            let len = self.board_column_len(self.selected_board_column);
                            if len == 0 {
                                self.selected_board_item = 0;
                            } else if self.selected_board_item < len.saturating_sub(1) {
                                self.selected_board_item += 1;
                            }
                        }
                        AppMode::MergeVisualizer => {
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
                        AppMode::Settings => {
                            let max = crate::pages::settings::SETTINGS_OPTIONS
                                .len()
                                .saturating_sub(1);
                            if self.selected_setting_index < max {
                                self.selected_setting_index += 1;
                            }
                        }
                    }
                    self.update_status_message();
                }
                false
            }
            KeyAction::NavigateLeft => {
                if self.focus == Focus::View {
                    match self.current_view {
                        AppMode::ProjectBoard => {
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
                        AppMode::MergeVisualizer => {
                            self.merge_focus = self.merge_focus.prev();
                        }
                        _ => {}
                    }
                    self.update_status_message();
                }
                false
            }
            KeyAction::NavigateRight => {
                if self.focus == Focus::View {
                    match self.current_view {
                        AppMode::ProjectBoard => {
                            self.selected_board_column = (self.selected_board_column + 1) % 3;
                            let len = self.board_column_len(self.selected_board_column);
                            self.selected_board_item = if len == 0 {
                                0
                            } else {
                                self.selected_board_item.min(len - 1)
                            };
                        }
                        AppMode::MergeVisualizer => {
                            self.merge_focus = self.merge_focus.next();
                        }
                        _ => {}
                    }
                    self.update_status_message();
                }
                false
            }
            KeyAction::InputChar(c) => {
                if self.search_active {
                    // In search mode, add character to search buffer
                    self.search_buffer.push(c);
                } else if self.focus == Focus::View && matches!(self.current_view, AppMode::Changes) {
                    self.commit_message.push(c);
                }
                false
            }
            KeyAction::Backspace => {
                if self.search_active {
                    self.search_buffer.pop();
                } else if self.focus == Focus::View && matches!(self.current_view, AppMode::Changes) {
                    self.commit_message.pop();
                }
                false
            }
            KeyAction::ScrollPageUp => {
                if self.focus == Focus::View {
                    match self.current_view {
                        AppMode::Dashboard => {
                            self.project_scroll = self.project_scroll.saturating_sub(5);
                        }
                        AppMode::Changes => {
                            self.changes_scroll = self.changes_scroll.saturating_sub(5);
                        }
                        AppMode::MergeVisualizer => {
                            self.merge_scroll = self.merge_scroll.saturating_sub(5);
                        }
                        _ => {}
                    }
                }
                false
            }
            KeyAction::ScrollPageDown => {
                if self.focus == Focus::View {
                    match self.current_view {
                        AppMode::Dashboard => {
                            let max = self.store.projects.len();
                            if max > 10 {
                                self.project_scroll = (self.project_scroll + 5).min(max - 10);
                            }
                        }
                        AppMode::Changes => {
                            let max = self
                                .store
                                .projects
                                .get(self.selected_project_index)
                                .map(|p| p.changes.len())
                                .unwrap_or(0);
                            if max > 10 {
                                self.changes_scroll = (self.changes_scroll + 5).min(max - 10);
                            }
                        }
                        AppMode::MergeVisualizer => {
                            let max = self
                                .store
                                .projects
                                .get(self.selected_project_index)
                                .map(|p| p.changes.len())
                                .unwrap_or(0);
                            if max > 10 {
                                self.merge_scroll = (self.merge_scroll + 5).min(max - 10);
                            }
                        }
                        _ => {}
                    }
                }
                false
            }
            KeyAction::Search => {
                if self.focus == Focus::View {
                    self.search_active = !self.search_active;
                    if self.search_active {
                        self.search_buffer.clear();
                        self.selected_project_index = 0;
                        self.status_message = "Search projects (type to filter, Esc to exit)".to_string();
                    } else {
                        self.search_buffer.clear();
                        self.update_status_message();
                    }
                }
                false
            }
            _ => false,
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
        self.status_message = match self.merge_focus {
            MergePaneFocus::Files => "Selected file for merge".to_string(),
            MergePaneFocus::Local => "✓ Accepted local version".to_string(),
            MergePaneFocus::Incoming => "✓ Accepted incoming version".to_string(),
        };
    }

    fn toggle_setting(&mut self) {
        let setting = crate::pages::settings::SETTINGS_OPTIONS
            .get(self.selected_setting_index)
            .unwrap_or(&"");
        self.status_message = format!("⚙ Toggled: {}", setting);
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
