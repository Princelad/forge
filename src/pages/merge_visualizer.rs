use ratatui::{Frame, layout::{Rect, Layout, Direction, Constraint}, widgets::{Block, Paragraph}, style::Stylize};

#[derive(Debug)]
pub struct MergeVisualizer;

impl MergeVisualizer {
    pub fn new() -> Self { Self }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().title("Merge Visualizer (mock)").magenta();
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let cols = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Length(24), Constraint::Percentage(38), Constraint::Percentage(38)]).split(inner);
        frame.render_widget(Paragraph::new("Changed Files\n- src/lib.rs\n- README.md").block(Block::bordered().title("Files")), cols[0]);
        frame.render_widget(Paragraph::new("Local version\n\nfn add(a,b){a+b}<<<<<").block(Block::bordered().title("Local")), cols[1]);
        frame.render_widget(Paragraph::new("Incoming version\n\nfn add(a,b){a-b}>>>>>").block(Block::bordered().title("Incoming")), cols[2]);
    }
}
