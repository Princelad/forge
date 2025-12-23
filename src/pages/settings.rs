use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    widgets::{Block, List, ListItem, ListState},
};

pub const SETTINGS_OPTIONS: [&str; 3] = ["Theme: Default", "Notifications: On", "Autosync: Off"];

#[derive(Debug)]
pub struct SettingsPage;

impl SettingsPage {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, selected_index: usize, scroll: usize) {
        let items: Vec<ListItem> = SETTINGS_OPTIONS.iter().map(|o| ListItem::new(*o)).collect();
        let mut state = ListState::default()
            .with_selected(Some(selected_index.min(items.len().saturating_sub(1))))
            .with_offset(scroll);
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title("Settings").style(Style::new()))
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true),
            area,
            &mut state,
        );
    }
}
