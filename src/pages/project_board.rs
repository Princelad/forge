use ratatui::{Frame, layout::{Rect, Layout, Direction, Constraint}, widgets::{Block, Paragraph, List, ListItem}, style::Stylize};
use crate::data::{Project, ModuleStatus};

#[derive(Debug)]
pub struct ProjectBoard;

impl ProjectBoard {
    pub fn new() -> Self { Self }

    pub fn render(&self, frame: &mut Frame, area: Rect, project: &Project) {
        let block = Block::bordered().title("Project Board (mock)").green();
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(33), Constraint::Percentage(34), Constraint::Percentage(33)])
            .split(inner);

        let mk = |status: ModuleStatus| {
            let items: Vec<ListItem> = project
                .modules
                .iter()
                .filter(|m| m.status == status)
                .map(|m| ListItem::new(format!("{} ({}) - {}%", m.name, m.owner.as_ref().map(|_| "owner").unwrap_or("unassigned"), m.progress_score)))
                .collect();
            List::new(items)
        };

        frame.render_widget(mk(ModuleStatus::Pending).block(Block::bordered().title("Pending")), cols[0]);
        frame.render_widget(mk(ModuleStatus::Current).block(Block::bordered().title("Current")), cols[1]);
        frame.render_widget(mk(ModuleStatus::Completed).block(Block::bordered().title("Completed")), cols[2]);
    }
}
