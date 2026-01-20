use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    widgets::{Block, List, ListState},
    Frame,
};

use crate::key_handler::KeyAction;
use crate::Focus;

#[derive(Debug)]
pub struct MainMenu {
    selected_option: usize,
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
            selected_option: 0,
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

    pub fn handle_key_action(&mut self, action: KeyAction) -> bool {
        match action {
            KeyAction::NavigateUp => {
                if self.selected_option > 0 {
                    self.selected_option -= 1;
                }
                false
            }
            KeyAction::NavigateDown => {
                if self.selected_option < self.menu_items.len().saturating_sub(1) {
                    self.selected_option += 1;
                }
                false
            }
            KeyAction::Select => false,
            KeyAction::Quit => true,
            _ => false,
        }
    }

    pub fn get_selected_option(&self) -> usize {
        self.selected_option
    }

    pub fn get_selected_item(&self) -> Option<&str> {
        self.menu_items.get(self.selected_option).copied()
    }

    pub fn get_items_count(&self) -> usize {
        self.menu_items.len()
    }

    pub fn get_item_by_index(&self, idx: usize) -> Option<&str> {
        self.menu_items.get(idx).copied()
    }
}
