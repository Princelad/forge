use crate::data::Project;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergePaneFocus {
    Files,
    Local,
    Incoming,
}

impl MergePaneFocus {
    pub fn next(self) -> Self {
        match self {
            MergePaneFocus::Files => MergePaneFocus::Local,
            MergePaneFocus::Local => MergePaneFocus::Incoming,
            MergePaneFocus::Incoming => MergePaneFocus::Files,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            MergePaneFocus::Files => MergePaneFocus::Incoming,
            MergePaneFocus::Local => MergePaneFocus::Files,
            MergePaneFocus::Incoming => MergePaneFocus::Local,
        }
    }
}

#[derive(Debug)]
pub struct MergeVisualizer;

impl Default for MergeVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

impl MergeVisualizer {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        project: &Project,
        selected_file: usize,
        pane_focus: MergePaneFocus,
        scroll: usize,
        accepted: Option<MergePaneFocus>,
    ) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(28),
                Constraint::Percentage(36),
                Constraint::Percentage(36),
            ])
            .split(area);

        // Files list
        let file_items: Vec<ListItem> = project
            .changes
            .iter()
            .map(|c| ListItem::new(format!("{} ({:?})", c.path, c.status)))
            .collect();
        let mut state = ListState::default()
            .with_selected(Some(selected_file.min(file_items.len().saturating_sub(1))))
            .with_offset(scroll);
        let files_block = Block::bordered().title("Files");
        let files_block = if pane_focus == MergePaneFocus::Files {
            files_block.border_style(Style::new().yellow())
        } else {
            files_block
        };
        frame.render_stateful_widget(
            List::new(file_items)
                .block(files_block)
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true),
            cols[0],
            &mut state,
        );

        // Local / Incoming panes
        let local_block = Block::bordered().title("Local change");
        let incoming_block = Block::bordered().title("Incoming change");

        let local_block = match (pane_focus, accepted) {
            (MergePaneFocus::Local, _) => local_block.border_style(Style::new().yellow()),
            (_, Some(MergePaneFocus::Local)) => local_block.border_style(Style::new().green()),
            _ => local_block,
        };
        let incoming_block = match (pane_focus, accepted) {
            (MergePaneFocus::Incoming, _) => incoming_block.border_style(Style::new().yellow()),
            (_, Some(MergePaneFocus::Incoming)) => {
                incoming_block.border_style(Style::new().green())
            }
            _ => incoming_block,
        };

        let (local_preview, incoming_preview) = match project.changes.get(selected_file) {
            Some(c) => {
                let local = c
                    .local_preview
                    .as_deref()
                    .unwrap_or(c.diff_preview.as_str());
                let incoming = c
                    .incoming_preview
                    .as_deref()
                    .unwrap_or("(no incoming preview)");
                (
                    format!("(local)\n{}", local),
                    format!("(incoming)\n{}", incoming),
                )
            }
            None => (
                "(local)\n(no diff preview)".to_string(),
                "(incoming)\n(no diff preview)".to_string(),
            ),
        };

        frame.render_widget(Paragraph::new(local_preview).block(local_block), cols[1]);
        frame.render_widget(
            Paragraph::new(incoming_preview).block(incoming_block),
            cols[2],
        );
    }
}
