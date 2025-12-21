use ratatui::{Frame, layout::Rect, widgets::Paragraph};

#[derive(Debug)]
pub struct SettingsPage;

impl SettingsPage {
    pub fn new() -> Self { Self }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Paragraph::new("Theme: Default\nNotifications: On\n\nAll values mocked."), area);
    }
}
