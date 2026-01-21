use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, ListState, Paragraph},
    Frame,
};

/// Creates a ListState with proper bounds checking and scrolling
pub fn create_list_state(selected: usize, scroll: usize, item_count: usize) -> ListState {
    ListState::default()
        .with_selected(Some(selected.min(item_count.saturating_sub(1))))
        .with_offset(scroll)
}

/// Creates a block with conditional focus styling (yellow border when focused)
pub fn focused_block(title: &str, is_focused: bool) -> Block<'_> {
    let block = Block::bordered().title(title);
    if is_focused {
        block.border_style(Style::new().yellow())
    } else {
        block
    }
}

/// Renders a common input form with title, label, and input buffer
pub fn render_input_form(frame: &mut Frame, area: Rect, title: &str, label: &str, input: &str) {
    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(format!("{}:", label), Style::new().yellow())),
        Line::from(""),
        Line::from(Span::raw(format!("> {}", input))),
        Line::from(""),
        Line::from(Span::styled("Press Enter to confirm", Style::new().gray())),
        Line::from(Span::styled("Press Esc to cancel", Style::new().gray())),
    ];

    frame.render_widget(
        Paragraph::new(help_text).block(Block::bordered().title(title)),
        area,
    );
}

/// Auto-scrolls a view to keep the selected item visible
/// Call this after changing selected index to adjust scroll position
pub fn auto_scroll(selected: usize, scroll: &mut usize, window_size: usize) {
    if selected < *scroll {
        *scroll = selected;
    } else if selected >= *scroll + window_size {
        *scroll = selected.saturating_sub(window_size - 1);
    }
}
