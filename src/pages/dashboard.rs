use ratatui::{Frame, layout::{Rect, Layout, Direction, Constraint}, widgets::{Block, Paragraph, List, ListItem, ListState}, style::Stylize};
use crate::data::Project;

#[derive(Debug)]
pub struct Dashboard;

impl Dashboard {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame:&mut Frame, area: Rect, projects: &[Project], selected: usize) {
        let block = Block::bordered().title("Dashboard").blue();
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(32), Constraint::Min(0)])
            .split(inner);

        // Left: project list
        let items: Vec<ListItem> = projects
            .iter()
            .map(|p| ListItem::new(p.name.clone()))
            .collect();
        let mut state = ListState::default().with_selected(Some(selected));
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title("Projects"))
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true)
                .highlight_style(ratatui::style::Style::new().reversed()),
            cols[0],
            &mut state,
        );

        // Right: details
        let details = projects.get(selected).map(|p| {
            format!(
                "Name: {}\nBranch: {}\n\nModules: {}\nDevelopers: {}\n\n{}",
                p.name,
                p.branch,
                p.modules.len(),
                p.developers.len(),
                p.description
            )
        }).unwrap_or_else(|| "No project".into());
        frame.render_widget(Paragraph::new(details).block(Block::bordered().title("Info")), cols[1]);
    }
}
