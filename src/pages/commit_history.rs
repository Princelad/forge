use crate::ui_utils::create_list_state;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub files_changed: Vec<String>,
}

#[derive(Debug)]
pub struct CommitHistory;

impl Default for CommitHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CommitHistory {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        commits: &[CommitInfo],
        selected: usize,
        scroll: usize,
        pane_ratio: u16,
    ) {
        let left = pane_ratio.clamp(20, 80);
        let right = 100u16.saturating_sub(left);
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(left), Constraint::Percentage(right)])
            .split(area);

        // Left: commit list
        self.render_commit_list(frame, layout[0], commits, selected, scroll);

        // Right: commit details
        if let Some(commit) = commits.get(selected) {
            self.render_commit_details(frame, layout[1], commit);
        } else {
            frame.render_widget(Block::bordered().title("Commit Details"), layout[1]);
        }
    }

    fn render_commit_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        commits: &[CommitInfo],
        selected: usize,
        scroll: usize,
    ) {
        let items: Vec<ListItem> = commits
            .iter()
            .map(|c| {
                let hash_short = if c.hash.len() > 7 {
                    c.hash[0..7].to_string()
                } else {
                    c.hash.clone()
                };

                let message_oneline = c.message.lines().next().unwrap_or("");
                let message_display = if message_oneline.len() > 50 {
                    format!("{}...", &message_oneline[0..47])
                } else {
                    message_oneline.to_string()
                };

                let author_display = c.author.clone();
                let date_display = format!(" on {}", c.date);

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(hash_short, Style::new().fg(Color::Yellow).bold()),
                        Span::raw(" "),
                        Span::raw(message_display),
                    ]),
                    Line::from(vec![
                        Span::styled("  by ", Style::new().gray()),
                        Span::styled(author_display, Style::new().cyan()),
                        Span::styled(date_display, Style::new().gray()),
                    ]),
                ])
            })
            .collect();

        let mut state = create_list_state(selected, scroll, items.len());

        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title("Commit History"))
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">> "),
            area,
            &mut state,
        );
    }

    fn render_commit_details(&self, frame: &mut Frame, area: Rect, commit: &CommitInfo) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Commit: ", Style::new().bold()),
                Span::styled(&commit.hash, Style::new().yellow()),
            ]),
            Line::from(vec![
                Span::styled("Author: ", Style::new().bold()),
                Span::raw(&commit.author),
            ]),
            Line::from(vec![
                Span::styled("Date:   ", Style::new().bold()),
                Span::raw(&commit.date),
            ]),
            Line::from(""),
        ];

        // Add commit message
        for msg_line in commit.message.lines() {
            lines.push(Line::from(msg_line));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Files Changed:",
            Style::new().bold(),
        )));

        // Add files changed
        if commit.files_changed.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (no files changed)",
                Style::new().gray(),
            )));
        } else {
            for file in &commit.files_changed {
                lines.push(Line::from(format!("  {}", file)));
            }
        }

        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::bordered().title("Commit Details"))
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}
