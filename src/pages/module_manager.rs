use crate::data::{Developer, Module, ModuleStatus, Project};
use crate::ui_utils::{create_list_state, render_input_form};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem},
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

/// Parameters for ModuleManager rendering
#[derive(Debug, Clone)]
pub struct ModuleManagerParams<'a> {
    pub area: Rect,
    pub project: &'a Project,
    pub mode: ModuleManagerMode,
    pub selected_module: usize,
    pub selected_developer: usize,
    pub input_buffer: &'a str,
    pub scroll: usize,
    pub pane_ratio: u16,
}

/// Parameters for ModuleList rendering
#[derive(Debug, Clone)]
pub struct ModuleListParams<'a> {
    pub area: Rect,
    pub modules: &'a [Module],
    pub developers: &'a [Developer],
    pub selected: usize,
    pub scroll: usize,
    pub is_focused: bool,
}

#[derive(Debug)]
pub struct ModuleManager;

impl Default for ModuleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleManager {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, frame: &mut Frame, params: ModuleManagerParams) {
        let left = params.pane_ratio.clamp(20, 80);
        let right = 100u16.saturating_sub(left);
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(left), Constraint::Percentage(right)])
            .split(params.area);

        // Left: Module list
        let list_params = ModuleListParams {
            area: layout[0],
            modules: &params.project.modules,
            developers: &params.project.developers,
            selected: params.selected_module,
            scroll: params.scroll,
            is_focused: params.mode == ModuleManagerMode::ModuleList,
        };
        self.render_module_list(frame, list_params);

        // Right: Developer list or input form
        match params.mode {
            ModuleManagerMode::CreateModule | ModuleManagerMode::EditModule => {
                self.render_module_form(frame, layout[1], params.input_buffer, params.mode);
            }
            ModuleManagerMode::CreateDeveloper => {
                self.render_developer_form(frame, layout[1], params.input_buffer);
            }
            _ => {
                self.render_developer_list(
                    frame,
                    layout[1],
                    &params.project.developers,
                    params.selected_developer,
                    0,
                    params.mode == ModuleManagerMode::DeveloperList,
                );
            }
        }
    }

    fn render_module_list(&self, frame: &mut Frame, params: ModuleListParams) {
        let items: Vec<ListItem> = params
            .modules
            .iter()
            .map(|m| {
                let owner_name = m
                    .owner
                    .and_then(|id| params.developers.iter().find(|d| d.id == id))
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

        let mut state = create_list_state(params.selected, params.scroll, items.len());

        let title = if params.is_focused {
            "Modules [FOCUSED]"
        } else {
            "Modules"
        };

        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title(title).style(if params.is_focused {
                    Style::new().fg(Color::Cyan)
                } else {
                    Style::new()
                }))
                .highlight_style(Style::new().reversed())
                .highlight_symbol(">> "),
            params.area,
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

        let mut state = create_list_state(selected, scroll, items.len());

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

        render_input_form(frame, area, title, "Enter module name", input);
    }

    fn render_developer_form(&self, frame: &mut Frame, area: Rect, input: &str) {
        render_input_form(frame, area, "Create New Developer", "Enter developer name", input);
    }
}
