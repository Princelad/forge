use crate::data::{Change, Project};
use crate::ui_utils::create_list_state;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, List, ListItem, Paragraph},
    Frame,
};

/// Parameters for Changes page rendering
#[derive(Debug, Clone)]
pub struct ChangesParams<'a> {
    pub area: Rect,
    pub project: &'a Project,
    pub selected: usize,
    pub commit_msg: &'a str,
    pub scroll: usize,
    pub pane_ratio: u16,
}

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

    pub fn render(&self, frame: &mut Frame, params: ChangesParams) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(params.area);

        let left = params.pane_ratio.clamp(20, 80);
        let right = 100u16.saturating_sub(left);
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(left), Constraint::Percentage(right)])
            .split(layout[0]);

        // Left: file list
        let items: Vec<ListItem> = params
            .project
            .changes
            .iter()
            .map(|c| ListItem::new(Self::fmt_change(c)))
            .collect();
        let mut state = create_list_state(params.selected, params.scroll, items.len());
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title(format!(
                    "Branch: {} | Space: stage/unstage | f: fetch | p: push | Ctrl+l: pull",
                    params.project.branch
                )))
                .highlight_style(ratatui::style::Style::new().reversed())
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true),
            cols[0],
            &mut state,
        );

        // Right: diff preview for selected
        let preview = params
            .project
            .changes
            .get(params.selected)
            .map(|c| c.diff_preview.clone())
            .unwrap_or_else(|| "Select a file".into());
        frame.render_widget(
            Paragraph::new(preview).block(Block::bordered().title("Diff Preview")),
            cols[1],
        );

        // Bottom: commit message input
        frame.render_widget(
            Paragraph::new(format!("Commit message: {}", params.commit_msg))
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
