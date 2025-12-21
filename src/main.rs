use ratatui::{DefaultTerminal, Frame};

pub mod key_handler;
pub mod screen;
pub mod pages;
pub mod data;
use key_handler::{KeyAction, KeyHandler};
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
}

impl App {
    pub fn new() -> Self {
        Self {
            running: false,
            screen: Screen::new(),
            key_handler: KeyHandler::new(),
            current_view: AppMode::Dashboard,
            prev_view: AppMode::Dashboard,
            focus: Focus::Menu,
            menu_selected_index: 0,
            selected_project: None,
            status_message: String::from("Ready"),
            store: data::FakeStore::new(),
            selected_project_index: 0,
            selected_change_index: 0,
            commit_message: String::new(),
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
        );
    }

    fn handle_action(&mut self, action: KeyAction) -> bool {
        match action {
            KeyAction::Quit => true,
            KeyAction::Back => {
                if self.focus == Focus::Menu {
                    return true; // Exit from main menu
                }
                self.focus = Focus::Menu;
                self.status_message = "Back to menu".to_string();
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
                    self.status_message = format!("View: {:?}", self.current_view);
                    // Sync menu selection to current view
                    self.menu_selected_index = self.current_view.menu_index();
                }
                false
            }
            KeyAction::Select => {
                if self.focus == Focus::Menu {
                    // Enter on menu switches to that view
                    if let Some(item) = self.screen.get_selected_menu_item_by_index(self.menu_selected_index) {
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
                    self.status_message = format!("View: {:?}", self.current_view);
                } else if matches!(self.current_view, AppMode::Changes) {
                    // Enter on Changes view commits
                    self.store.bump_progress_on_commit(self.selected_project_index);
                    self.status_message = format!("Committed: {}", self.commit_message);
                    self.commit_message.clear();
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
                            if self.selected_project_index > 0 { self.selected_project_index -= 1; }
                        }
                        AppMode::Changes => {
                            if self.selected_change_index > 0 { self.selected_change_index -= 1; }
                        }
                        _ => {}
                    }
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
                            if self.selected_project_index < max { self.selected_project_index += 1; }
                        }
                        AppMode::Changes => {
                            let max = self
                                .store
                                .projects
                                .get(self.selected_project_index)
                                .map(|p| p.changes.len().saturating_sub(1))
                                .unwrap_or(0);
                            if self.selected_change_index < max { self.selected_change_index += 1; }
                        }
                        _ => {}
                    }
                }
                false
            }
            KeyAction::InputChar(c) => {
                if self.focus == Focus::View && matches!(self.current_view, AppMode::Changes) {
                    self.commit_message.push(c);
                }
                false
            }
            KeyAction::Backspace => {
                if self.focus == Focus::View && matches!(self.current_view, AppMode::Changes) {
                    self.commit_message.pop();
                }
                false
            }
            _ => false,
        }
    }

    fn quit(&mut self) {
        self.running = false;
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
