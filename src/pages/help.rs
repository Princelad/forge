use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

#[derive(Debug)]
pub struct HelpPage;

impl Default for HelpPage {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpPage {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(9),
                Constraint::Length(8),
                Constraint::Min(0),
            ])
            .split(area);

        // Fill the overlay to avoid a transparent background bleeding through
        frame.render_widget(Block::default().style(Style::new().bg(Color::Black)), area);

        // Navigation section
        let nav_help = vec![
            Line::from(vec![
                Span::styled("↑↓ / k j", Style::new().bold().cyan()),
                Span::raw("     Navigate items"),
            ]),
            Line::from(vec![
                Span::styled("← → / h l", Style::new().bold().cyan()),
                Span::raw("     Change columns/panes"),
            ]),
            Line::from(vec![
                Span::styled("Tab", Style::new().bold().cyan()),
                Span::raw("          Cycle through views"),
            ]),
            Line::from(vec![
                Span::styled("Enter / ↵", Style::new().bold().cyan()),
                Span::raw("     Select/Confirm action"),
            ]),
            Line::from(vec![
                Span::styled("Esc", Style::new().bold().cyan()),
                Span::raw("          Back to menu"),
            ]),
            Line::from(vec![
                Span::styled("q / Ctrl-C", Style::new().bold().cyan()),
                Span::raw("     Quit"),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(nav_help)
                .block(Block::bordered().title("Navigation"))
                .style(Style::new()),
            sections[0],
        );

        // View-specific section
        let view_help = vec![
            Line::from(vec![
                Span::styled("Dashboard", Style::new().bold().yellow()),
                Span::raw("  View projects & modules"),
            ]),
            Line::from(vec![
                Span::styled("Changes", Style::new().bold().yellow()),
                Span::raw("    Browse & commit changes"),
            ]),
            Line::from(vec![
                Span::styled("Board", Style::new().bold().yellow()),
                Span::raw("      Move tasks between columns"),
            ]),
            Line::from(vec![
                Span::styled("Merge", Style::new().bold().yellow()),
                Span::raw("       Resolve merge conflicts"),
            ]),
            Line::from(vec![
                Span::styled("Settings", Style::new().bold().yellow()),
                Span::raw("    Configure app behavior"),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(view_help)
                .block(Block::bordered().title("Views"))
                .style(Style::new()),
            sections[1],
        );

        // Actions section
        let action_help = vec![
            Line::from(vec![
                Span::styled("Dashboard", Style::new().bold().magenta()),
                Span::raw("  Press "),
                Span::styled("Enter", Style::new().bold()),
                Span::raw(" to open Changes view"),
            ]),
            Line::from(vec![
                Span::styled("Changes", Style::new().bold().magenta()),
                Span::raw("    Press "),
                Span::styled("Space", Style::new().bold()),
                Span::raw(" to stage/unstage file"),
            ]),
            Line::from(vec![
                Span::styled("Remote", Style::new().bold().magenta()),
                Span::raw("      "),
                Span::styled("f", Style::new().bold()),
                Span::raw(": Fetch,  "),
                Span::styled("p", Style::new().bold()),
                Span::raw(": Push,  "),
                Span::styled("Ctrl+l", Style::new().bold()),
                Span::raw(": Pull"),
            ]),
            Line::from(vec![
                Span::styled("Branches", Style::new().bold().magenta()),
                Span::raw("   n: New, d: Delete, Enter: Switch"),
            ]),
            Line::from(vec![
                Span::styled("Modules", Style::new().bold().magenta()),
                Span::raw("    n: New, e: Edit, d: Delete, a: Assign"),
            ]),
            Line::from(vec![
                Span::styled("Board", Style::new().bold().magenta()),
                Span::raw("       Press "),
                Span::styled("Enter", Style::new().bold()),
                Span::raw(" to move item to next column"),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(action_help)
                .block(Block::bordered().title("Actions"))
                .style(Style::new()),
            sections[2],
        );

        // Tips section
        let tips = vec![
            Line::from("💡 Status bar shows contextual hints for your current view and selection"),
            Line::from("💡 Git commits are real—module progress persists to .forge/progress.json"),
            Line::from("💡 Use Tab to quickly navigate between different parts of the app"),
        ];
        frame.render_widget(
            Paragraph::new(tips)
                .block(Block::bordered().title("Tips"))
                .style(Style::new()),
            sections[3],
        );
    }
}
