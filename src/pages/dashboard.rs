use crate::data::Project;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

#[derive(Debug)]
pub struct Dashboard;

impl Dashboard {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        projects: &[&Project],
        selected: usize,
        scroll: usize,
        search_active: bool,
        search_buffer: &str,
    ) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(32), Constraint::Min(0)])
            .split(area);

        // Left: project list with scrolling
        let items: Vec<ListItem> = projects
            .iter()
            .map(|p| ListItem::new(p.name.clone()))
            .collect();
        let mut state = ListState::default()
            .with_selected(Some(selected.min(items.len().saturating_sub(1))))
            .with_offset(scroll);

        let title = if search_active {
            format!("Projects (search: {})", search_buffer)
        } else {
            "Projects".to_string()
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
        let details = projects
            .get(selected)
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
