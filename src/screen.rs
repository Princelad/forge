use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::{Line, Span},
    widgets::Block,
};

use crate::key_handler::KeyAction;
use crate::AppMode;
use crate::Focus;
use crate::data::FakeStore;
use crate::pages::dashboard::Dashboard;
use crate::pages::main_menu::MainMenu;
use crate::pages::changes::ChangesPage;
use crate::pages::merge_visualizer::{MergeVisualizer, MergePaneFocus};
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

    pub fn render(
        &self,
        frame: &mut Frame,
        mode: AppMode,
        status: &str,
        store: &FakeStore,
        selected_project: usize,
        selected_change: usize,
        commit_msg: &str,
        menu_selected_index: usize,
        _focus: Focus,
        selected_board_column: usize,
        selected_board_item: usize,
        merge_file_index: usize,
        merge_focus: MergePaneFocus,
        selected_setting: usize,
    ) {
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

        // Create menu bar for the content block header
        let mut menu_line = Vec::new();
        for (idx, item) in self.main_menu.menu_items.iter().enumerate() {
            if idx == menu_selected_index {
                menu_line.push(Span::styled(
                    format!(" {} ", item),
                    ratatui::style::Style::new().reversed(),
                ));
            } else {
                menu_line.push(Span::raw(format!(" {} ", item)));
            }
            if idx < self.main_menu.menu_items.len() - 1 {
                menu_line.push(Span::raw("|"));
            }
        }
        
        let content_title = Line::from(menu_line).left_aligned();
        let content_block = Block::bordered().title(content_title);
        let content_area = content_block.inner(vlayout[0]);
        frame.render_widget(content_block, vlayout[0]);

        // Render the content page based on mode
        match mode {
            AppMode::Dashboard => self.dashborard.render(frame, content_area, &store.projects, selected_project),
            AppMode::Changes => {
                let proj = store.projects.get(selected_project);
                if let Some(p) = proj { self.changes.render(frame, content_area, p, selected_change, commit_msg); }
            }
            AppMode::MergeVisualizer => {
                let proj = store.projects.get(selected_project);
                if let Some(p) = proj { self.merge.render(frame, content_area, p, merge_file_index, merge_focus); }
            }
            AppMode::ProjectBoard => {
                let proj = store.projects.get(selected_project);
                if let Some(p) = proj { self.board.render(frame, content_area, p, selected_board_column, selected_board_item); }
            }
            AppMode::Settings => self.settings.render(frame, content_area, selected_setting),
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
