use crate::data::{Change, Project};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};

#[derive(Debug)]
pub struct ChangesPage;

impl Default for ChangesPage {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangesPage {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        project: &Project,
        selected: usize,
        commit_msg: &str,
        scroll: usize,
    ) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(36), Constraint::Min(0)])
            .split(layout[0]);

        // Left: file list
        let items: Vec<ListItem> = project
            .changes
            .iter()
            .map(|c| ListItem::new(Self::fmt_change(c)))
            .collect();
        let mut state = ListState::default()
            .with_selected(Some(selected))
            .with_offset(scroll);
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title(format!(
                    "Branch: {} | Space to stage/unstage",
                    project.branch
                )))
                .highlight_style(ratatui::style::Style::new().reversed())
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true),
            cols[0],
            &mut state,
        );

        // Right: diff preview for selected
        let preview = project
            .changes
            .get(selected)
            .map(|c| c.diff_preview.clone())
            .unwrap_or_else(|| "Select a file".into());
        frame.render_widget(
            Paragraph::new(preview).block(Block::bordered().title("Diff Preview")),
            cols[1],
        );

        // Bottom: commit message input
        frame.render_widget(
            Paragraph::new(format!("Commit message: {}", commit_msg))
                .block(Block::bordered().title("Type and press Enter to commit")),
            layout[1],
        );
    }

    fn fmt_change(c: &Change) -> String {
        let status = match c.status {
            crate::data::FileStatus::Modified => "M",
            crate::data::FileStatus::Added => "A",
            crate::data::FileStatus::Deleted => "D",
        };
        let staged_marker = if c.staged { "✓" } else { " " };
        format!("[{staged_marker}] [{status}] {}", c.path)
    }
}
