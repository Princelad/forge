use crate::ui_utils::create_list_state;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, List, ListItem},
    Frame,
};

/// Parameters for Settings page rendering
#[derive(Debug, Clone)]
pub struct SettingsParams<'a> {
    pub area: Rect,
    pub selected: usize,
    pub scroll: usize,
    pub options: &'a [String],
}

#[derive(Debug)]
pub struct SettingsPage;

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPage {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, params: SettingsParams) {
        let items: Vec<ListItem> = params
            .options
            .iter()
            .map(|o| ListItem::new(o.clone()))
            .collect();
        let mut state = create_list_state(params.selected, params.scroll, items.len());
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title("Settings").style(Style::new()))
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true),
            params.area,
            &mut state,
        );
    }
}
