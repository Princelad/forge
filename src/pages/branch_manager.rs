use crate::ui_utils::{create_list_state, render_input_form};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem},
    Frame,
};

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BranchManagerMode {
    #[default]
    List,
    CreateBranch,
}

/// Parameters for BranchManager rendering
#[derive(Debug, Clone)]
pub struct BranchManagerParams<'a> {
    pub area: Rect,
    pub branches: &'a [BranchInfo],
    pub selected: usize,
    pub scroll: usize,
    pub mode: BranchManagerMode,
    pub input_buffer: &'a str,
}

#[derive(Debug)]
pub struct BranchManager;

impl Default for BranchManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BranchManager {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, params: BranchManagerParams) {
        match params.mode {
            BranchManagerMode::List => {
                self.render_branch_list(
                    frame,
                    params.area,
                    params.branches,
                    params.selected,
                    params.scroll,
                );
            }
            BranchManagerMode::CreateBranch => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(7)])
                    .split(params.area);

                self.render_branch_list(
                    frame,
                    layout[0],
                    params.branches,
                    params.selected,
                    params.scroll,
                );
                self.render_create_form(frame, layout[1], params.input_buffer);
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

        let mut state = create_list_state(selected, scroll, items.len());

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
        render_input_form(frame, area, "Create New Branch", "Branch name", input);
    }
}
