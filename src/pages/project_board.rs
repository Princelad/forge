use crate::data::{ModuleStatus, Project};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    widgets::{Block, List, ListItem, ListState},
};

#[derive(Debug)]
pub struct ProjectBoard;

impl ProjectBoard {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        project: &Project,
        selected_column: usize,
        selected_item: usize,
        scroll: usize,
    ) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(area);

        let mk = |status: ModuleStatus| -> Vec<ListItem> {
            project
                .modules
                .iter()
                .filter(|m| m.status == status)
                .map(|m| {
                    ListItem::new(format!(
                        "{} ({}) - {}%",
                        m.name,
                        m.owner.as_ref().map(|_| "owner").unwrap_or("unassigned"),
                        m.progress_score
                    ))
                })
                .collect()
        };

        let pending_items = mk(ModuleStatus::Pending);
        let current_items = mk(ModuleStatus::Current);
        let done_items = mk(ModuleStatus::Completed);

        let mut pending_state = ListState::default().with_offset(scroll);
        let mut current_state = ListState::default().with_offset(scroll);
        let mut done_state = ListState::default().with_offset(scroll);

        if selected_column == 0 {
            if !pending_items.is_empty() {
                pending_state.select(Some(selected_item.min(pending_items.len() - 1)));
            }
        } else if selected_column == 1 {
            if !current_items.is_empty() {
                current_state.select(Some(selected_item.min(current_items.len() - 1)));
            }
        } else if selected_column == 2 {
            if !done_items.is_empty() {
                done_state.select(Some(selected_item.min(done_items.len() - 1)));
            }
        }

        let block_style_focused = Style::new().yellow();

        frame.render_stateful_widget(
            List::new(pending_items)
                .block(
                    Block::bordered()
                        .title("Pending")
                        .border_style(if selected_column == 0 {
                            block_style_focused
                        } else {
                            Style::default()
                        }),
                )
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true)
                .highlight_style(Style::new().reversed()),
            cols[0],
            &mut pending_state,
        );

        frame.render_stateful_widget(
            List::new(current_items)
                .block(
                    Block::bordered()
                        .title("Current")
                        .border_style(if selected_column == 1 {
                            block_style_focused
                        } else {
                            Style::default()
                        }),
                )
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true)
                .highlight_style(Style::new().reversed()),
            cols[1],
            &mut current_state,
        );

        frame.render_stateful_widget(
            List::new(done_items)
                .block(
                    Block::bordered()
                        .title("Completed")
                        .border_style(if selected_column == 2 {
                            block_style_focused
                        } else {
                            Style::default()
                        }),
                )
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true)
                .highlight_style(Style::new().reversed()),
            cols[2],
            &mut done_state,
        );
    }
}
