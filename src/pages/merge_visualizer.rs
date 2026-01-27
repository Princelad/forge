use crate::data::Project;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergePaneFocus {
    #[default]
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

/// Parameters for MergeVisualizer rendering
#[derive(Debug, Clone)]
pub struct MergeVisualizerParams<'a> {
    pub area: Rect,
    pub project: &'a Project,
    pub selected_file: usize,
    pub pane_focus: MergePaneFocus,
    pub scroll: usize,
    pub accepted: Option<MergePaneFocus>,
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

    pub fn render(&self, frame: &mut Frame, params: MergeVisualizerParams) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(28),
                Constraint::Percentage(36),
                Constraint::Percentage(36),
            ])
            .split(params.area);

        // Files list
        let file_items: Vec<ListItem> = params
            .project
            .changes
            .iter()
            .map(|c| ListItem::new(format!("{} ({:?})", c.path, c.status)))
            .collect();
        let mut state = ListState::default()
            .with_selected(Some(
                params.selected_file.min(file_items.len().saturating_sub(1)),
            ))
            .with_offset(params.scroll);
        let files_block = Block::bordered().title("Files");
        let files_block = if params.pane_focus == MergePaneFocus::Files {
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

        let local_block = match (params.pane_focus, params.accepted) {
            (MergePaneFocus::Local, _) => local_block.border_style(Style::new().yellow()),
            (_, Some(MergePaneFocus::Local)) => local_block.border_style(Style::new().green()),
            _ => local_block,
        };
        let incoming_block = match (params.pane_focus, params.accepted) {
            (MergePaneFocus::Incoming, _) => incoming_block.border_style(Style::new().yellow()),
            (_, Some(MergePaneFocus::Incoming)) => {
                incoming_block.border_style(Style::new().green())
            }
            _ => incoming_block,
        };

        let (local_preview, incoming_preview) =
            match params.project.changes.get(params.selected_file) {
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
