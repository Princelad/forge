use crate::data::Project;
use crate::ui_utils::create_list_state;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, List, ListItem, Paragraph},
    Frame,
};

/// Parameters for Dashboard rendering
#[derive(Debug, Clone)]
pub struct DashboardParams<'a> {
    pub area: Rect,
    pub projects: &'a [&'a Project],
    pub selected: usize,
    pub scroll: usize,
    pub search_active: bool,
    pub search_buffer: &'a str,
    pub total_count: usize,
    pub pane_ratio: u16,
    pub status: &'a str,
}

#[derive(Debug)]
pub struct Dashboard;

impl Default for Dashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Dashboard {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, params: DashboardParams) {
        let left_pct = params.pane_ratio;
        let right_pct = 100 - left_pct;
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(left_pct),
                Constraint::Percentage(right_pct),
            ])
            .split(params.area);

        // Left: project list with scrolling
        let items: Vec<ListItem> = params
            .projects
            .iter()
            .map(|p| ListItem::new(p.name.clone()))
            .collect();
        let items = if items.is_empty() {
            vec![ListItem::new(
                "No projects found. Open a Git repository to get started.",
            )]
        } else {
            items
        };
        let mut state = create_list_state(params.selected, params.scroll, items.len());

        let status_suffix = if params.status.starts_with('⟳') {
            " | Loading"
        } else if params.status.starts_with('✗') {
            " | Error"
        } else {
            ""
        };

        let title = if params.search_active {
            format!(
                "Projects (search: {} · {}/{} matches) | Esc to exit search{}",
                params.search_buffer,
                params.projects.len(),
                params.total_count,
                status_suffix
            )
        } else {
            format!("Projects (Ctrl+F: search, f: fetch){}", status_suffix)
        };

        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title(title))
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true)
                .highlight_style(ratatui::style::Style::new().reversed()),
            cols[0],
            &mut state,
        );

        // Right: details
        let details = params
            .projects
            .get(params.selected)
            .map(|p| {
                format!(
                    "Name: {}\nBranch: {}\n\nModules: {}\nDevelopers: {}\n\n{}",
                    p.name,
                    p.branch,
                    p.modules.len(),
                    p.developers.len(),
                    p.description
                )
            })
            .unwrap_or_else(|| {
                "No active project data.\n\nTry:\n- Open this app from inside a Git repository\n- Press Tab to navigate to other views\n- Press ? for keyboard help"
                    .into()
            });
        frame.render_widget(
            Paragraph::new(details).block(Block::bordered().title("Info")),
            cols[1],
        );
    }
}
