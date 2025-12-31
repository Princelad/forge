use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BranchManagerMode {
    List,
    CreateBranch,
}

#[derive(Debug)]
pub struct BranchManager;

impl BranchManager {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        branches: &[BranchInfo],
        selected: usize,
        scroll: usize,
        mode: BranchManagerMode,
        input_buffer: &str,
    ) {
        match mode {
            BranchManagerMode::List => {
                self.render_branch_list(frame, area, branches, selected, scroll);
            }
            BranchManagerMode::CreateBranch => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(7)])
                    .split(area);

                self.render_branch_list(frame, layout[0], branches, selected, scroll);
                self.render_create_form(frame, layout[1], input_buffer);
            }
        }
    }

    fn render_branch_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        branches: &[BranchInfo],
        selected: usize,
        scroll: usize,
    ) {
        let items: Vec<ListItem> = branches
            .iter()
            .map(|b| {
                let prefix = if b.is_current {
                    Span::styled("* ", Style::new().fg(Color::Green).bold())
                } else {
                    Span::raw("  ")
                };

                let branch_type = if b.is_remote {
                    Span::styled(" [remote]", Style::new().fg(Color::Cyan))
                } else {
                    Span::raw("")
                };

                let name = Span::styled(
                    &b.name,
                    if b.is_current {
                        Style::new().fg(Color::Green).bold()
                    } else {
                        Style::new()
                    },
                );

                ListItem::new(Line::from(vec![prefix, name, branch_type]))
            })
            .collect();

        let mut state = ListState::default()
            .with_selected(Some(selected.min(items.len().saturating_sub(1))))
            .with_offset(scroll);

        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title("Branches | ↵ Switch | n New | d Delete"))
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">> "),
            area,
            &mut state,
        );
    }

    fn render_create_form(&self, frame: &mut Frame, area: Rect, input: &str) {
        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled("Branch name:", Style::new().yellow())),
            Line::from(Span::raw(format!("> {}", input))),
            Line::from(""),
            Line::from(Span::styled(
                "Press Enter to create | Esc to cancel",
                Style::new().gray(),
            )),
        ];

        frame.render_widget(
            Paragraph::new(help_text).block(Block::bordered().title("Create New Branch")),
            area,
        );
    }
}
