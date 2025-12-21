use ratatui::{Frame, layout::Rect, widgets::{Block, Paragraph}, style::Stylize};

#[derive(Debug)]
pub struct SettingsPage;

impl SettingsPage {
    pub fn new() -> Self { Self }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title("Settings (mock)").cyan();
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new("Theme: Default\nNotifications: On\n\nAll values mocked."), inner);
    }
}
