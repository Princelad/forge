use ratatui::{Frame, layout::{Rect, Layout, Direction, Constraint}, widgets::{Block, Paragraph, List, ListItem, ListState}, style::Stylize};
use crate::data::{Project, Change};

#[derive(Debug)]
pub struct ChangesPage;

impl ChangesPage {
    pub fn new() -> Self { Self }

    pub fn render(&self, frame: &mut Frame, area: Rect, project: &Project, selected: usize, commit_msg: &str) {
        let block = Block::bordered().title("Changes (mock)").yellow();
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(inner);

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
        let mut state = ListState::default().with_selected(Some(selected));
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title(format!("Branch: {}", project.branch)))
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
        frame.render_widget(Paragraph::new(preview).block(Block::bordered().title("Diff Preview")), cols[1]);

        // Bottom: commit message input
        frame.render_widget(
            Paragraph::new(format!("Commit message: {}", commit_msg))
                .block(Block::bordered().title("Type and press Enter to commit (mock)")),
            layout[1],
        );
    }

    fn fmt_change(c: &Change) -> String {
        let status = match c.status { crate::data::FileStatus::Modified => "M", crate::data::FileStatus::Added => "A", crate::data::FileStatus::Deleted => "D" };
        format!("[{status}] {}", c.path)
    }
}
