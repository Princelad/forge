use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::Line,
    widgets::Block,
};

use crate::key_handler::KeyAction;
use crate::AppMode;
use crate::Focus;
use crate::data::FakeStore;
use crate::pages::dashboard::Dashboard;
use crate::pages::main_menu::MainMenu;
use crate::pages::changes::ChangesPage;
use crate::pages::merge_visualizer::MergeVisualizer;
use crate::pages::project_board::ProjectBoard;
use crate::pages::settings::SettingsPage;

#[derive(Debug)]
pub struct Screen {
    main_menu: MainMenu,
    dashborard: Dashboard,
    changes: ChangesPage,
    merge: MergeVisualizer,
    board: ProjectBoard,
    settings: SettingsPage,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            main_menu: MainMenu::new(),
            dashborard: Dashboard::new(),
            changes: ChangesPage::new(),
            merge: MergeVisualizer::new(),
            board: ProjectBoard::new(),
            settings: SettingsPage::new(),
        }
    }

    pub fn render(&self, frame: &mut Frame, mode: AppMode, status: &str, store: &FakeStore, selected_project: usize, selected_change: usize, commit_msg: &str, menu_selected_index: usize, focus: Focus) {
        let area = frame.area();
        let title = Line::from("Forge - Git Aware Project Management")
            .bold()
            .blue()
            .left_aligned();
        let block = Block::bordered().title(title);
        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // Split into main content and bottom status bar
        let vlayout = Layout::new(
            Direction::Vertical,
            [Constraint::Min(0), Constraint::Length(1)],
        )
        .split(inner_area);

        // Inside main content, split into menu and page area
        let layout = Layout::new(
            Direction::Horizontal,
            [Constraint::Length(30), Constraint::Min(0)],
        )
        .split(vlayout[0]);

        self.main_menu.render(frame, layout[0], menu_selected_index, focus);
        match mode {
            AppMode::Dashboard => self.dashborard.render(frame, layout[1], &store.projects, selected_project),
            AppMode::Changes => {
                let proj = store.projects.get(selected_project);
                if let Some(p) = proj { self.changes.render(frame, layout[1], p, selected_change, commit_msg); }
            }
            AppMode::MergeVisualizer => self.merge.render(frame, layout[1]),
            AppMode::ProjectBoard => {
                let proj = store.projects.get(selected_project);
                if let Some(p) = proj { self.board.render(frame, layout[1], p); }
            }
            AppMode::Settings => self.settings.render(frame, layout[1]),
        }

        // Render the status bar on bottom
        let status_line = Line::from(format!(
            "{}  |  Tab: Switch View  Enter: Open  Esc/q: Quit",
            status
        ))
        .on_dark_gray()
        .white();
        frame.render_widget(status_line, vlayout[1]);
    }

    pub fn handle_key_action(&mut self, action: KeyAction) -> bool {
        self.main_menu.handle_key_action(action)
    }

    pub fn get_selected_menu_item(&self) -> Option<&str> {
        self.main_menu.get_selected_item()
    }

    pub fn get_menu_items_count(&self) -> usize {
        self.main_menu.get_items_count()
    }

    pub fn get_selected_menu_item_by_index(&self, idx: usize) -> Option<&str> {
        self.main_menu.get_item_by_index(idx)
    }
}
