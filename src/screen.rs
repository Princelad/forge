use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Clear},
    Frame,
};
use throbber_widgets_tui::{Throbber, ThrobberState};

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

/// Context for rendering the UI
///
/// Bundles all parameters needed for rendering to reduce function signature complexity
/// and improve maintainability when adding new views or features.
#[derive(Debug)]
pub struct RenderContext<'a> {
    pub mode: AppMode,
    pub status: &'a str,
    pub store: &'a crate::data::Store,
    pub selected_project: usize,
    pub selected_change: usize,
    pub commit_msg: &'a str,
    pub changes_pane_ratio: u16,
    pub commit_pane_ratio: u16,
    pub dashboard_pane_ratio: u16,
    pub menu_selected_index: usize,
    pub focus: Focus,
    pub selected_board_column: usize,
    pub selected_board_item: usize,
    pub merge_file_index: usize,
    pub merge_focus: crate::pages::merge_visualizer::MergePaneFocus,
    pub selected_setting: usize,
    pub show_help: bool,
    pub project_scroll: usize,
    pub changes_scroll: usize,
    pub merge_scroll: usize,
    pub search_active: bool,
    pub search_buffer: &'a str,
    pub filtered_projects: &'a [&'a crate::data::Project],
    pub settings_options: &'a [String],
    pub total_projects: usize,
    pub settings: &'a AppSettings,
    pub accepted_merge: Option<crate::pages::merge_visualizer::MergePaneFocus>,
    pub workdir: Option<&'a std::path::Path>,
    pub module_manager_mode: crate::pages::module_manager::ModuleManagerMode,
    pub selected_module: usize,
    pub selected_developer: usize,
    pub module_input_buffer: &'a str,
    pub module_scroll: usize,
    pub module_pane_ratio: u16,
    pub branch_manager_mode: crate::pages::branch_manager::BranchManagerMode,
    pub selected_branch: usize,
    pub branch_input_buffer: &'a str,
    pub branch_scroll: usize,
    pub cached_branches: &'a [crate::pages::branch_manager::BranchInfo],
    pub selected_commit: usize,
    pub commit_scroll: usize,
    pub cached_commits: &'a [crate::pages::commit_history::CommitInfo],
    pub pending_git_ops_count: usize,
}

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

    pub fn render(&mut self, frame: &mut Frame, ctx: &RenderContext) {
        // Tick spinner if there are pending operations
        if ctx.pending_git_ops_count > 0 {
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
        let focus_style = if matches!(ctx.focus, Focus::Menu) {
            ratatui::style::Style::new().yellow().bold()
        } else {
            ratatui::style::Style::new()
        };

        for (idx, item) in self.main_menu.menu_items.iter().enumerate() {
            if idx == ctx.menu_selected_index {
                menu_line.push(Span::styled(
                    format!(" {} ", item),
                    ratatui::style::Style::new().reversed().patch(focus_style),
                ));
            } else {
                menu_line.push(Span::styled(
                    format!(" {} ", item),
                    if matches!(ctx.focus, Focus::Menu) {
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
        match ctx.mode {
            AppMode::Dashboard => {
                let params = crate::pages::dashboard::DashboardParams {
                    area: content_area,
                    projects: ctx.filtered_projects,
                    selected: ctx.selected_project,
                    scroll: ctx.project_scroll,
                    search_active: ctx.search_active,
                    search_buffer: ctx.search_buffer,
                    total_count: ctx.total_projects,
                    pane_ratio: ctx.dashboard_pane_ratio,
                };
                self.dashboard.render(frame, params);
            }
            AppMode::Changes => {
                let proj = ctx.store.projects.get(ctx.selected_project);
                if let Some(p) = proj {
                    let params = crate::pages::changes::ChangesParams {
                        area: content_area,
                        project: p,
                        selected: ctx.selected_change,
                        commit_msg: ctx.commit_msg,
                        scroll: ctx.changes_scroll,
                        pane_ratio: ctx.changes_pane_ratio,
                    };
                    self.changes.render(frame, params);
                }
            }
            AppMode::CommitHistory => {
                let params = crate::pages::commit_history::CommitHistoryParams {
                    area: content_area,
                    commits: ctx.cached_commits,
                    selected: ctx.selected_commit,
                    scroll: ctx.commit_scroll,
                    pane_ratio: ctx.commit_pane_ratio,
                };
                self.commit_history.render(frame, params);
            }
            AppMode::BranchManager => {
                let params = crate::pages::branch_manager::BranchManagerParams {
                    area: content_area,
                    branches: ctx.cached_branches,
                    selected: ctx.selected_branch,
                    scroll: ctx.branch_scroll,
                    mode: ctx.branch_manager_mode,
                    input_buffer: ctx.branch_input_buffer,
                };
                self.branch_manager.render(frame, params);
            }
            AppMode::MergeVisualizer => {
                let proj = ctx.store.projects.get(ctx.selected_project);
                if let Some(p) = proj {
                    let params = crate::pages::merge_visualizer::MergeVisualizerParams {
                        area: content_area,
                        project: p,
                        selected_file: ctx.merge_file_index,
                        pane_focus: ctx.merge_focus,
                        scroll: ctx.merge_scroll,
                        accepted: ctx.accepted_merge,
                    };
                    self.merge.render(frame, params);
                }
            }
            AppMode::ProjectBoard => {
                let proj = ctx.store.projects.get(ctx.selected_project);
                if let Some(p) = proj {
                    let params = crate::pages::project_board::ProjectBoardParams {
                        area: content_area,
                        project: p,
                        selected_column: ctx.selected_board_column,
                        selected_item: ctx.selected_board_item,
                        scroll: ctx.project_scroll,
                    };
                    self.board.render(frame, params);
                }
            }
            AppMode::ModuleManager => {
                let proj = ctx.store.projects.get(ctx.selected_project);
                if let Some(p) = proj {
                    let params = crate::pages::module_manager::ModuleManagerParams {
                        area: content_area,
                        project: p,
                        mode: ctx.module_manager_mode,
                        selected_module: ctx.selected_module,
                        selected_developer: ctx.selected_developer,
                        input_buffer: ctx.module_input_buffer,
                        scroll: ctx.module_scroll,
                        pane_ratio: ctx.module_pane_ratio,
                    };
                    self.module_manager.render(frame, params);
                }
            }
            AppMode::Settings => {
                let params = crate::pages::settings::SettingsParams {
                    area: content_area,
                    selected: ctx.selected_setting,
                    scroll: ctx.project_scroll,
                    options: ctx.settings_options,
                };
                self.settings.render(frame, params);
            }
        }

        // Render the status bar on bottom
        let repo_badge = ctx
            .workdir
            .map(|p| format!("Repo: {}", p.display()))
            .unwrap_or_else(|| "Repo: n/a".to_string());

        let status_text = format!(
            "{}  |  {}  |  Tab: Switch View  Enter: Open  ?: Help  Esc/q: Quit",
            ctx.status, repo_badge
        );

        if ctx.pending_git_ops_count > 0 {
            let status_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(16), Constraint::Min(0)])
                .split(vlayout[1]);

            let spinner_style = match ctx.settings.theme {
                Theme::HighContrast => ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::Yellow),
                Theme::Default => ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Cyan)
                    .bg(ratatui::style::Color::DarkGray),
            };

            let spinner_widget = Throbber::default()
                .label(format!(" {} ops", ctx.pending_git_ops_count))
                .style(spinner_style);
            frame.render_stateful_widget(spinner_widget, status_layout[0], &mut self.spinner_state);

            let status_line = Line::from(status_text);
            let status_line = match ctx.settings.theme {
                Theme::HighContrast => status_line.on_yellow().black(),
                Theme::Default => status_line.on_dark_gray().white(),
            };
            frame.render_widget(status_line, status_layout[1]);
        } else {
            let status_line = Line::from(status_text);
            let status_line = match ctx.settings.theme {
                Theme::HighContrast => status_line.on_yellow().black(),
                Theme::Default => status_line.on_dark_gray().white(),
            };
            frame.render_widget(status_line, vlayout[1]);
        }

        // Render help overlay if needed
        if ctx.show_help {
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
}
