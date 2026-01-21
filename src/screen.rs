use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Clear},
    Frame,
};
use throbber_widgets_tui::{Throbber, ThrobberState};

use crate::key_handler::KeyAction;
use crate::pages::branch_manager::BranchManager;
use crate::pages::changes::ChangesPage;
use crate::pages::commit_history::CommitHistory;
use crate::pages::dashboard::Dashboard;
use crate::pages::help::HelpPage;
use crate::pages::main_menu::MainMenu;
use crate::pages::merge_visualizer::MergeVisualizer;
use crate::pages::module_manager::ModuleManager;
use crate::pages::project_board::ProjectBoard;
use crate::pages::settings::SettingsPage;
use crate::{AppMode, AppSettings, Focus, Theme};

#[derive(Debug)]
pub struct Screen {
    main_menu: MainMenu,
    dashboard: Dashboard,
    changes: ChangesPage,
    commit_history: CommitHistory,
    branch_manager: BranchManager,
    merge: MergeVisualizer,
    board: ProjectBoard,
    module_manager: ModuleManager,
    settings: SettingsPage,
    help: HelpPage,
    spinner_state: ThrobberState,
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen {
    pub fn new() -> Self {
        Self {
            main_menu: MainMenu::new(),
            dashboard: Dashboard::new(),
            changes: ChangesPage::new(),
            commit_history: CommitHistory::new(),
            branch_manager: BranchManager::new(),
            merge: MergeVisualizer::new(),
            board: ProjectBoard::new(),
            module_manager: ModuleManager::new(),
            settings: SettingsPage::new(),
            help: HelpPage::new(),
            spinner_state: ThrobberState::default(),
        }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        mode: AppMode,
        status: &str,
        store: &crate::data::Store,
        selected_project: usize,
        selected_change: usize,
        commit_msg: &str,
        changes_pane_ratio: u16,
        commit_pane_ratio: u16,
        dashboard_pane_ratio: u16,
        menu_selected_index: usize,
        _focus: Focus,
        selected_board_column: usize,
        selected_board_item: usize,
        merge_file_index: usize,
        merge_focus: crate::pages::merge_visualizer::MergePaneFocus,
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
        accepted_merge: Option<crate::pages::merge_visualizer::MergePaneFocus>,
        workdir: Option<&std::path::Path>,
        // New page parameters
        module_manager_mode: crate::pages::module_manager::ModuleManagerMode,
        selected_module: usize,
        selected_developer: usize,
        module_input_buffer: &str,
        module_scroll: usize,
        module_pane_ratio: u16,
        branch_manager_mode: crate::pages::branch_manager::BranchManagerMode,
        selected_branch: usize,
        branch_input_buffer: &str,
        branch_scroll: usize,
        cached_branches: &[crate::pages::branch_manager::BranchInfo],
        selected_commit: usize,
        commit_scroll: usize,
        cached_commits: &[crate::pages::commit_history::CommitInfo],
        pending_git_ops_count: usize,
    ) {
        // Tick spinner if there are pending operations
        if pending_git_ops_count > 0 {
            self.spinner_state.calc_next();
        }

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
            AppMode::Dashboard => {
                let params = crate::pages::dashboard::DashboardParams {
                    area: content_area,
                    projects: filtered_projects,
                    selected: selected_project,
                    scroll: project_scroll,
                    search_active,
                    search_buffer,
                    total_count: total_projects,
                    pane_ratio: dashboard_pane_ratio,
                };
                self.dashboard.render(frame, params);
            }
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
                        changes_pane_ratio,
                    );
                }
            }
            AppMode::CommitHistory => {
                self.commit_history.render(
                    frame,
                    content_area,
                    cached_commits,
                    selected_commit,
                    commit_scroll,
                    commit_pane_ratio,
                );
            }
            AppMode::BranchManager => {
                let params = crate::pages::branch_manager::BranchManagerParams {
                    area: content_area,
                    branches: cached_branches,
                    selected: selected_branch,
                    scroll: branch_scroll,
                    mode: branch_manager_mode,
                    input_buffer: branch_input_buffer,
                };
                self.branch_manager.render(frame, params);
            }
            AppMode::MergeVisualizer => {
                let proj = store.projects.get(selected_project);
                if let Some(p) = proj {
                    let params = crate::pages::merge_visualizer::MergeVisualizerParams {
                        area: content_area,
                        project: p,
                        selected_file: merge_file_index,
                        pane_focus: merge_focus,
                        scroll: merge_scroll,
                        accepted: accepted_merge,
                    };
                    self.merge.render(frame, params);
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
            AppMode::ModuleManager => {
                let proj = store.projects.get(selected_project);
                if let Some(p) = proj {
                    let params = crate::pages::module_manager::ModuleManagerParams {
                        area: content_area,
                        project: p,
                        mode: module_manager_mode,
                        selected_module,
                        selected_developer,
                        input_buffer: module_input_buffer,
                        scroll: module_scroll,
                        pane_ratio: module_pane_ratio,
                    };
                    self.module_manager.render(frame, params);
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
        let repo_badge = workdir
            .map(|p| format!("Repo: {}", p.display()))
            .unwrap_or_else(|| "Repo: n/a".to_string());

        let status_text = format!(
            "{}  |  {}  |  Tab: Switch View  Enter: Open  ?: Help  Esc/q: Quit",
            status, repo_badge
        );

        if pending_git_ops_count > 0 {
            let status_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(16), Constraint::Min(0)])
                .split(vlayout[1]);

            let spinner_style = match settings.theme {
                Theme::HighContrast => ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::Yellow),
                Theme::Default => ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Cyan)
                    .bg(ratatui::style::Color::DarkGray),
            };

            let spinner_widget = Throbber::default()
                .label(format!(" {} ops", pending_git_ops_count))
                .style(spinner_style);
            frame.render_stateful_widget(spinner_widget, status_layout[0], &mut self.spinner_state);

            let status_line = Line::from(status_text);
            let status_line = match settings.theme {
                Theme::HighContrast => status_line.on_yellow().black(),
                Theme::Default => status_line.on_dark_gray().white(),
            };
            frame.render_widget(status_line, status_layout[1]);
        } else {
            let status_line = Line::from(status_text);
            let status_line = match settings.theme {
                Theme::HighContrast => status_line.on_yellow().black(),
                Theme::Default => status_line.on_dark_gray().white(),
            };
            frame.render_widget(status_line, vlayout[1]);
        }

        // Render help overlay if needed
        if show_help {
            let popup_area = self.centered_rect(90, 90, frame.area());
            frame.render_widget(Clear, popup_area);
            frame.render_widget(
                Block::bordered()
                    .style(ratatui::style::Style::new().bg(ratatui::style::Color::Black)),
                popup_area,
            );
            let inner = Block::bordered().inner(popup_area);
            frame.render_widget(Clear, inner);
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
