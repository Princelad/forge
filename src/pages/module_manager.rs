use crate::data::{Developer, Module, ModuleStatus, Project};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModuleManagerMode {
    ModuleList,
    DeveloperList,
    CreateModule,
    CreateDeveloper,
    EditModule,
}

#[derive(Debug)]
pub struct ModuleManager;

impl ModuleManager {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        project: &Project,
        mode: ModuleManagerMode,
        selected_module: usize,
        selected_developer: usize,
        input_buffer: &str,
        scroll: usize,
    ) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: Module list
        self.render_module_list(
            frame,
            layout[0],
            &project.modules,
            &project.developers,
            selected_module,
            scroll,
            mode == ModuleManagerMode::ModuleList,
        );

        // Right: Developer list or input form
        match mode {
            ModuleManagerMode::CreateModule | ModuleManagerMode::EditModule => {
                self.render_module_form(frame, layout[1], input_buffer, mode);
            }
            ModuleManagerMode::CreateDeveloper => {
                self.render_developer_form(frame, layout[1], input_buffer);
            }
            _ => {
                self.render_developer_list(
                    frame,
                    layout[1],
                    &project.developers,
                    selected_developer,
                    0,
                    mode == ModuleManagerMode::DeveloperList,
                );
            }
        }
    }

    fn render_module_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        modules: &[Module],
        developers: &[Developer],
        selected: usize,
        scroll: usize,
        is_focused: bool,
    ) {
        let items: Vec<ListItem> = modules
            .iter()
            .map(|m| {
                let owner_name = m
                    .owner
                    .and_then(|id| developers.iter().find(|d| d.id == id))
                    .map(|d| d.name.as_str())
                    .unwrap_or("Unassigned");

                let status_icon = match m.status {
                    ModuleStatus::Pending => "⏸",
                    ModuleStatus::Current => "▶",
                    ModuleStatus::Completed => "✓",
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", status_icon),
                            Style::new().fg(match m.status {
                                ModuleStatus::Pending => Color::Gray,
                                ModuleStatus::Current => Color::Yellow,
                                ModuleStatus::Completed => Color::Green,
                            }),
                        ),
                        Span::styled(&m.name, Style::new().bold()),
                    ]),
                    Line::from(vec![
                        Span::raw("  Owner: "),
                        Span::styled(owner_name, Style::new().cyan()),
                        Span::raw(format!(" | Progress: {}%", m.progress_score)),
                    ]),
                ])
            })
            .collect();

        let mut state = ListState::default()
            .with_selected(Some(selected.min(items.len().saturating_sub(1))))
            .with_offset(scroll);

        let title = if is_focused {
            "Modules [FOCUSED]"
        } else {
            "Modules"
        };

        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title(title).style(if is_focused {
                    Style::new().fg(Color::Cyan)
                } else {
                    Style::new()
                }))
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">> "),
            area,
            &mut state,
        );
    }

    fn render_developer_list(
        &self,
        frame: &mut Frame,
        area: Rect,
        developers: &[Developer],
        selected: usize,
        scroll: usize,
        is_focused: bool,
    ) {
        let items: Vec<ListItem> = developers
            .iter()
            .map(|d| {
                ListItem::new(vec![
                    Line::from(Span::styled(&d.name, Style::new().bold())),
                    Line::from(Span::styled(format!("  ID: {}", d.id), Style::new().gray())),
                ])
            })
            .collect();

        let mut state = ListState::default()
            .with_selected(Some(selected.min(items.len().saturating_sub(1))))
            .with_offset(scroll);

        let title = if is_focused {
            "Developers [FOCUSED]"
        } else {
            "Developers"
        };

        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title(title).style(if is_focused {
                    Style::new().fg(Color::Cyan)
                } else {
                    Style::new()
                }))
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">> "),
            area,
            &mut state,
        );
    }

    fn render_module_form(
        &self,
        frame: &mut Frame,
        area: Rect,
        input: &str,
        mode: ModuleManagerMode,
    ) {
        let title = match mode {
            ModuleManagerMode::CreateModule => "Create New Module",
            ModuleManagerMode::EditModule => "Edit Module",
            _ => "Module Form",
        };

        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled("Enter module name:", Style::new().yellow())),
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

    fn render_developer_form(&self, frame: &mut Frame, area: Rect, input: &str) {
        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled("Enter developer name:", Style::new().yellow())),
            Line::from(""),
            Line::from(Span::raw(format!("> {}", input))),
            Line::from(""),
            Line::from(Span::styled("Press Enter to confirm", Style::new().gray())),
            Line::from(Span::styled("Press Esc to cancel", Style::new().gray())),
        ];

        frame.render_widget(
            Paragraph::new(help_text).block(Block::bordered().title("Create New Developer")),
            area,
        );
    }
}
