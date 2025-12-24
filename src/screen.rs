use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::{Line, Span},
    widgets::Block,
};

use crate::data::FakeStore;
use crate::key_handler::KeyAction;
use crate::pages::changes::ChangesPage;
use crate::pages::dashboard::Dashboard;
use crate::pages::help::HelpPage;
use crate::pages::main_menu::MainMenu;
use crate::pages::merge_visualizer::{MergePaneFocus, MergeVisualizer};
use crate::pages::project_board::ProjectBoard;
use crate::pages::settings::SettingsPage;
use crate::{AppMode, AppSettings, Focus, Theme};

#[derive(Debug)]
pub struct Screen {
    main_menu: MainMenu,
    dashborard: Dashboard,
    changes: ChangesPage,
    merge: MergeVisualizer,
    board: ProjectBoard,
    settings: SettingsPage,
    help: HelpPage,
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
            help: HelpPage::new(),
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
        show_help: bool,
        project_scroll: usize,
        changes_scroll: usize,
        merge_scroll: usize,
        search_active: bool,
        search_buffer: &str,
        filtered_projects: &[&crate::data::Project],
        settings_options: &[String],
        total_projects: usize,
        settings: &AppSettings,
        accepted_merge: Option<MergePaneFocus>,
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
        let focus_style = if matches!(_focus, Focus::Menu) {
            ratatui::style::Style::new().yellow().bold()
        } else {
            ratatui::style::Style::new()
        };

        for (idx, item) in self.main_menu.menu_items.iter().enumerate() {
            if idx == menu_selected_index {
                menu_line.push(Span::styled(
                    format!(" {} ", item),
                    ratatui::style::Style::new().reversed().patch(focus_style),
                ));
            } else {
                menu_line.push(Span::styled(
                    format!(" {} ", item),
                    if matches!(_focus, Focus::Menu) {
                        ratatui::style::Style::new().bold()
                    } else {
                        ratatui::style::Style::new()
                    },
                ));
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
            AppMode::Dashboard => self.dashborard.render(
                frame,
                content_area,
                filtered_projects,
                selected_project,
                project_scroll,
                search_active,
                search_buffer,
                total_projects,
            ),
            AppMode::Changes => {
                let proj = store.projects.get(selected_project);
                if let Some(p) = proj {
                    self.changes.render(
                        frame,
                        content_area,
                        p,
                        selected_change,
                        commit_msg,
                        changes_scroll,
                    );
                }
            }
            AppMode::MergeVisualizer => {
                let proj = store.projects.get(selected_project);
                if let Some(p) = proj {
                    self.merge.render(
                        frame,
                        content_area,
                        p,
                        merge_file_index,
                        merge_focus,
                        merge_scroll,
                        accepted_merge,
                    );
                }
            }
            AppMode::ProjectBoard => {
                let proj = store.projects.get(selected_project);
                if let Some(p) = proj {
                    self.board.render(
                        frame,
                        content_area,
                        p,
                        selected_board_column,
                        selected_board_item,
                        project_scroll,
                    );
                }
            }
            AppMode::Settings => self.settings.render(
                frame,
                content_area,
                selected_setting,
                project_scroll,
                settings_options,
            ),
        }

        // Render the status bar on bottom
        let focus_label = match _focus {
            Focus::Menu => "Focus: Menu",
            Focus::View => "Focus: View",
        };
        let settings_badge = format!(
            "Theme: {} | Notif: {} | Auto: {}",
            match settings.theme {
                Theme::Default => "Default",
                Theme::HighContrast => "HighContrast",
            },
            if settings.notifications { "On" } else { "Off" },
            if settings.autosync { "On" } else { "Off" }
        );

        let status_line = Line::from(format!(
            "{}  |  {}  |  {}  |  Tab: Switch View  Enter: Open  ?: Help  Esc/q: Quit",
            status, focus_label, settings_badge
        ));
        let status_line = match settings.theme {
            Theme::HighContrast => status_line.on_yellow().black(),
            Theme::Default => status_line.on_dark_gray().white(),
        };
        frame.render_widget(status_line, vlayout[1]);

        // Render help overlay if needed
        if show_help {
            let popup_area = self.centered_rect(90, 90, frame.area());
            frame.render_widget(
                Block::bordered()
                    .style(ratatui::style::Style::new().bg(ratatui::style::Color::Black)),
                popup_area,
            );
            let inner = Block::bordered().inner(popup_area);
            self.help.render(frame, inner);
        }
    }

    fn centered_rect(
        &self,
        percent_x: u16,
        percent_y: u16,
        r: ratatui::layout::Rect,
    ) -> ratatui::layout::Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
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
