use crate::data::Project;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, List, ListItem, ListState, Paragraph},
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
        let local_block = if pane_focus == MergePaneFocus::Local {
            local_block.border_style(Style::new().yellow())
        } else {
            local_block
        };
        let incoming_block = if pane_focus == MergePaneFocus::Incoming {
            incoming_block.border_style(Style::new().yellow())
        } else {
            incoming_block
        };

        let local_preview = "fn add(a, b) { a + b }";
        let incoming_preview = "fn add(a, b) { a - b }";
        frame.render_widget(Paragraph::new(local_preview).block(local_block), cols[1]);
        frame.render_widget(
            Paragraph::new(incoming_preview).block(incoming_block),
            cols[2],
        );
    }
}
