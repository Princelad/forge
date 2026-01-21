use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, List, ListState},
    Frame,
};

use crate::Focus;

#[derive(Debug)]
pub struct MainMenu {
    pub menu_items: Vec<&'static str>,
}

impl Default for MainMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl MainMenu {
    pub fn new() -> Self {
        Self {
            menu_items: vec![
                "Dashboard",
                "Changes",
                "History",
                "Branches",
                "Merge",
                "Board",
                "Modules",
                "Settings",
            ],
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, selected_index: usize, focus: Focus) {
        let mut state = ListState::default().with_selected(Some(selected_index));

        let block_style = if focus == Focus::Menu {
            Block::bordered().title("Menu").style(Style::new().green())
        } else {
            Block::bordered().title("Menu")
        };

        frame.render_stateful_widget(
            List::new(self.menu_items.clone())
                .block(block_style)
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">> ")
                .repeat_highlight_symbol(true),
            area,
            &mut state,
        );
    }
}
