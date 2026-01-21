use crate::data::{ModuleStatus, Project};
use crate::ui_utils::{create_list_state, focused_block};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{List, ListItem},
    Frame,
};

/// Parameters for ProjectBoard page rendering
#[derive(Debug, Clone)]
pub struct ProjectBoardParams<'a> {
    pub area: Rect,
    pub project: &'a Project,
    pub selected_column: usize,
    pub selected_item: usize,
    pub scroll: usize,
}

#[derive(Debug)]
pub struct ProjectBoard;

impl Default for ProjectBoard {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectBoard {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, params: ProjectBoardParams) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(params.area);

        let mk = |status: ModuleStatus| -> Vec<ListItem> {
            params.project
                .modules
                .iter()
                .filter(|m| m.status == status)
                .map(|m| {
                    let owner_name = m
                        .owner
                        .and_then(|oid| params.project.developers.iter().find(|d| d.id == oid))
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| "unassigned".to_string());
                    ListItem::new(format!(
                        "{} ({}) - {}%",
                        m.name, owner_name, m.progress_score
                    ))
                })
                .collect()
        };

        let pending_items = mk(ModuleStatus::Pending);
        let current_items = mk(ModuleStatus::Current);
        let done_items = mk(ModuleStatus::Completed);

        // Create states using utility - only select if this is the active column
        let mut pending_state = if params.selected_column == 0 {
            create_list_state(params.selected_item, params.scroll, pending_items.len())
        } else {
            create_list_state(0, params.scroll, 0)
        };
        
        let mut current_state = if params.selected_column == 1 {
            create_list_state(params.selected_item, params.scroll, current_items.len())
        } else {
            create_list_state(0, params.scroll, 0)
        };
        
        let mut done_state = if params.selected_column == 2 {
            create_list_state(params.selected_item, params.scroll, done_items.len())
        } else {
            create_list_state(0, params.scroll, 0)
        };

        frame.render_stateful_widget(
            List::new(pending_items)
                .block(focused_block("Pending", params.selected_column == 0))
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true)
                .highlight_style(Style::new().reversed()),
            cols[0],
            &mut pending_state,
        );

        frame.render_stateful_widget(
            List::new(current_items)
                .block(focused_block("Current", params.selected_column == 1))
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true)
                .highlight_style(Style::new().reversed()),
            cols[1],
            &mut current_state,
        );

        frame.render_stateful_widget(
            List::new(done_items)
                .block(focused_block("Completed", params.selected_column == 2))
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true)
                .highlight_style(Style::new().reversed()),
            cols[2],
            &mut done_state,
        );
    }
}
