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
        let mut state = create_list_state(params.selected, params.scroll, items.len());

        let title = if params.search_active {
            format!(
                "Projects (search: {} · {}/{} matches) | Esc to exit search",
                params.search_buffer,
                params.projects.len(),
                params.total_count
            )
        } else {
            "Projects (Ctrl+F: search, f: fetch)".to_string()
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
            .unwrap_or_else(|| "No project".into());
        frame.render_widget(
            Paragraph::new(details).block(Block::bordered().title("Info")),
            cols[1],
        );
    }
}
