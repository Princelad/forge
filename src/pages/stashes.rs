use crate::ui_utils::{create_list_state, render_input_form};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Parameters for Stashes page rendering
#[derive(Debug, Clone)]
pub struct StashesParams<'a> {
    pub area: Rect,
    pub stashes: &'a [StashInfo],
    pub selected: usize,
    pub scroll: usize,
    pub mode: StashesMode,
    pub input_buffer: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum StashesMode {
    #[default]
    List,
    Create,
}

#[derive(Debug, Clone)]
pub struct StashInfo {
    pub index: usize,
    pub name: String,
    pub oid: String,
}

#[derive(Debug)]
pub struct StashesPage;

impl Default for StashesPage {
    fn default() -> Self {
        Self::new()
    }
}

impl StashesPage {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, params: StashesParams) {
        match params.mode {
            StashesMode::List => {
                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(params.area);

                self.render_stash_list(
                    frame,
                    layout[0],
                    params.stashes,
                    params.selected,
                    params.scroll,
                );

                if let Some(stash) = params.stashes.get(params.selected) {
                    self.render_stash_details(frame, layout[1], stash);
                } else {
                    frame.render_widget(Block::bordered().title("Stash Details"), layout[1]);
                }
            }
            StashesMode::Create => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(7)])
                    .split(params.area);

                self.render_stash_list(
                    frame,
                    layout[0],
                    params.stashes,
                    params.selected,
                    params.scroll,
                );
                render_input_form(
                    frame,
                    layout[1],
                    "Create Stash",
                    "Message",
                    params.input_buffer,
                );
            }
        }
    }

    fn render_stash_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        stashes: &[StashInfo],
        selected: usize,
        scroll: usize,
    ) {
        let items: Vec<ListItem> = stashes
            .iter()
            .map(|stash| {
                let label = format!("stash@{{{}}}: {}", stash.index, stash.name);
                ListItem::new(Line::from(vec![
                    Span::styled("- ", Style::new().fg(Color::DarkGray)),
                    Span::raw(label),
                ]))
            })
            .collect();

        let mut state = create_list_state(selected, scroll, items.len());
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title("Stashes | n New"))
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">> "),
            area,
            &mut state,
        );
    }

    fn render_stash_details(&self, frame: &mut Frame, area: Rect, stash: &StashInfo) {
        let lines = vec![
            Line::from(vec![
                Span::styled("Stash: ", Style::new().bold()),
                Span::styled(format!("stash@{{{}}}", stash.index), Style::new().yellow()),
            ]),
            Line::from(vec![
                Span::styled("Name:  ", Style::new().bold()),
                Span::raw(&stash.name),
            ]),
            Line::from(vec![
                Span::styled("OID:   ", Style::new().bold()),
                Span::raw(&stash.oid),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::bordered().title("Stash Details"))
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}
