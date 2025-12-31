use crate::{AppMode, Focus};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction {
    Quit,
    Back,
    NextView,
    NavigateUp,
    NavigateDown,
    NavigateLeft,
    NavigateRight,
    ScrollPageUp,
    ScrollPageDown,
    Select,
    Help,
    Search,
    InputChar(char),
    Backspace,
    None,
}

#[derive(Debug)]
pub struct KeyHandler;

impl KeyHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_crossterm_events(&mut self) -> color_eyre::Result<KeyAction> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => Ok(self.on_key_event(key)),
            Event::Mouse(_) => Ok(KeyAction::None),
            Event::Resize(_, _) => Ok(KeyAction::None),
            _ => Ok(KeyAction::None),
        }
    }

    pub fn on_key_event(&mut self, key: KeyEvent) -> KeyAction {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => KeyAction::Back,
            (_, KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => KeyAction::Quit,
            (_, KeyCode::Char('?')) => KeyAction::Help,
            (KeyModifiers::CONTROL, KeyCode::Char('f') | KeyCode::Char('F')) => KeyAction::Search,
            (KeyModifiers::NONE, KeyCode::Tab) => KeyAction::NextView,
            (KeyModifiers::NONE, KeyCode::Up | KeyCode::Char('k')) => KeyAction::NavigateUp,
            (KeyModifiers::NONE, KeyCode::Down | KeyCode::Char('j')) => KeyAction::NavigateDown,
            (KeyModifiers::NONE, KeyCode::Left | KeyCode::Char('h')) => KeyAction::NavigateLeft,
            (KeyModifiers::NONE, KeyCode::Right | KeyCode::Char('l')) => KeyAction::NavigateRight,
            (KeyModifiers::NONE, KeyCode::PageUp) => KeyAction::ScrollPageUp,
            (KeyModifiers::NONE, KeyCode::PageDown) => KeyAction::ScrollPageDown,
            (KeyModifiers::NONE, KeyCode::Enter) => KeyAction::Select,
            (KeyModifiers::NONE, KeyCode::Backspace) => KeyAction::Backspace,
            (KeyModifiers::NONE, KeyCode::Char(c)) => KeyAction::InputChar(c),
            _ => KeyAction::None,
        }
    }
}

/// Action handler result: (should_quit, side_effects_callback)
pub struct ActionResult {
    pub should_quit: bool,
    pub status_message: Option<String>,
}

/// Context passed to action handlers to enable decision-making
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub focus: Focus,
    pub current_view: AppMode,
    pub show_help: bool,
    pub search_active: bool,
    pub menu_selected_index: usize,
    pub selected_project_index: usize,
    pub selected_change_index: usize,
    pub selected_board_column: usize,
    pub selected_board_item: usize,
    pub selected_merge_file_index: usize,
    pub selected_setting_index: usize,
    pub commit_message_empty: bool,
    pub has_git_client: bool,
    // New view indices
    pub selected_commit_index: usize,
    pub selected_branch_index: usize,
    pub selected_module_index: usize,
    pub selected_developer_index: usize,
    pub cached_commits_len: usize,
    pub cached_branches_len: usize,
}

/// Stateless action processor: takes action + context, returns result + modified state
pub struct ActionProcessor;

impl ActionProcessor {
    pub fn process(action: KeyAction, ctx: &ActionContext) -> (ActionResult, ActionStateUpdate) {
        match action {
            KeyAction::Quit => (
                ActionResult {
                    should_quit: true,
                    status_message: None,
                },
                ActionStateUpdate::none(),
            ),
            KeyAction::Help => (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate {
                    show_help: Some(!ctx.show_help),
                    ..Default::default()
                },
            ),
            KeyAction::Back => {
                if ctx.show_help {
                    return (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate {
                            show_help: Some(false),
                            ..Default::default()
                        },
                    );
                }
                if ctx.search_active {
                    return (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Exited search".into()),
                        },
                        ActionStateUpdate {
                            search_active: Some(false),
                            search_buffer: Some(String::new()),
                            selected_project_index: Some(0),
                            ..Default::default()
                        },
                    );
                }
                if ctx.focus == Focus::Menu {
                    return (
                        ActionResult {
                            should_quit: true,
                            status_message: None,
                        },
                        ActionStateUpdate::none(),
                    );
                }
                (
                    ActionResult {
                        should_quit: false,
                        status_message: Some(
                            "Menu: Tab to navigate, ↵ to select, q to quit".into(),
                        ),
                    },
                    ActionStateUpdate {
                        focus: Some(Focus::Menu),
                        ..Default::default()
                    },
                )
            }
            KeyAction::NextView => {
                if ctx.focus == Focus::Menu {
                    let menu_len = 5; // Fixed: 5 menu items
                    let next_idx = (ctx.menu_selected_index + 1) % menu_len;
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate {
                            menu_selected_index: Some(next_idx),
                            ..Default::default()
                        },
                    )
                } else {
                    let next_view = ctx.current_view.next();
                    let next_idx = next_view.menu_index();
                    let update = ActionStateUpdate {
                        current_view: Some(next_view),
                        menu_selected_index: Some(next_idx),
                        search_active: if matches!(next_view, AppMode::Dashboard) {
                            None
                        } else {
                            Some(false)
                        },
                        search_buffer: if matches!(next_view, AppMode::Dashboard) {
                            None
                        } else {
                            Some(String::new())
                        },
                        ..Default::default()
                    };
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        update,
                    )
                }
            }
            KeyAction::Select => Self::handle_select(ctx),
            KeyAction::NavigateUp => Self::handle_navigate_up(ctx),
            KeyAction::NavigateDown => Self::handle_navigate_down(ctx),
            KeyAction::NavigateLeft => Self::handle_navigate_left(ctx),
            KeyAction::NavigateRight => Self::handle_navigate_right(ctx),
            KeyAction::InputChar(c) => {
                if ctx.search_active {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate {
                            search_buffer_append: Some(c),
                            ..Default::default()
                        },
                    )
                } else if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::Changes) {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate {
                            commit_message_append: Some(c),
                            ..Default::default()
                        },
                    )
                } else {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate::none(),
                    )
                }
            }
            KeyAction::Backspace => {
                if ctx.search_active {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate {
                            search_buffer_pop: Some(()),
                            ..Default::default()
                        },
                    )
                } else if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::Changes) {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate {
                            commit_message_pop: Some(()),
                            ..Default::default()
                        },
                    )
                } else {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate::none(),
                    )
                }
            }
            KeyAction::ScrollPageUp => {
                let update = match ctx.current_view {
                    AppMode::Dashboard => ActionStateUpdate {
                        project_scroll_up: Some(5),
                        ..Default::default()
                    },
                    AppMode::Changes => ActionStateUpdate {
                        changes_scroll_up: Some(5),
                        ..Default::default()
                    },
                    AppMode::MergeVisualizer => ActionStateUpdate {
                        merge_scroll_up: Some(5),
                        ..Default::default()
                    },
                    _ => ActionStateUpdate::none(),
                };
                (
                    ActionResult {
                        should_quit: false,
                        status_message: None,
                    },
                    update,
                )
            }
            KeyAction::ScrollPageDown => {
                let update = match ctx.current_view {
                    AppMode::Dashboard => ActionStateUpdate {
                        project_scroll_down: Some(5),
                        ..Default::default()
                    },
                    AppMode::Changes => ActionStateUpdate {
                        changes_scroll_down: Some(5),
                        ..Default::default()
                    },
                    AppMode::MergeVisualizer => ActionStateUpdate {
                        merge_scroll_down: Some(5),
                        ..Default::default()
                    },
                    _ => ActionStateUpdate::none(),
                };
                (
                    ActionResult {
                        should_quit: false,
                        status_message: None,
                    },
                    update,
                )
            }
            KeyAction::Search => {
                if ctx.focus == Focus::View {
                    if !matches!(ctx.current_view, AppMode::Dashboard) {
                        (
                            ActionResult {
                                should_quit: false,
                                status_message: Some(
                                    "Search is available only in Dashboard".into(),
                                ),
                            },
                            ActionStateUpdate::none(),
                        )
                    } else {
                        let next_active = !ctx.search_active;
                        let status = if next_active {
                            "Search projects (type to filter, Esc to exit)".to_string()
                        } else {
                            String::new() // Will be set by update_status_message
                        };
                        (
                            ActionResult {
                                should_quit: false,
                                status_message: if next_active { Some(status) } else { None },
                            },
                            ActionStateUpdate {
                                search_active: Some(next_active),
                                search_buffer: Some(String::new()),
                                selected_project_index: if next_active { Some(0) } else { None },
                                ..Default::default()
                            },
                        )
                    }
                } else {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate::none(),
                    )
                }
            }
            KeyAction::None => (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate::none(),
            ),
        }
    }

    fn handle_select(ctx: &ActionContext) -> (ActionResult, ActionStateUpdate) {
        if ctx.focus == Focus::Menu {
            // Menu selection will be handled by main.rs looking at menu_selected_index
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate {
                    focus: Some(Focus::View),
                    ..Default::default()
                },
            )
        } else if matches!(ctx.current_view, AppMode::Changes) {
            if ctx.commit_message_empty {
                (
                    ActionResult {
                        should_quit: false,
                        status_message: Some("Commit message cannot be empty".into()),
                    },
                    ActionStateUpdate::none(),
                )
            } else if ctx.has_git_client {
                // Real commit handler - will be called in main.rs
                (
                    ActionResult {
                        should_quit: false,
                        status_message: Some("Attempting commit...".into()),
                    },
                    ActionStateUpdate {
                        commit_requested: Some(()),
                        ..Default::default()
                    },
                )
            } else {
                // Mock commit
                (
                    ActionResult {
                        should_quit: false,
                        status_message: Some(
                            "Committed (mock only; no Git repository detected)".into(),
                        ),
                    },
                    ActionStateUpdate {
                        commit_message_clear: Some(()),
                        ..Default::default()
                    },
                )
            }
        } else if matches!(ctx.current_view, AppMode::ProjectBoard) {
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate {
                    move_board_item: Some(()),
                    ..Default::default()
                },
            )
        } else if matches!(ctx.current_view, AppMode::MergeVisualizer) {
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate {
                    accept_merge_pane: Some(()),
                    ..Default::default()
                },
            )
        } else if matches!(ctx.current_view, AppMode::Settings) {
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate {
                    toggle_setting: Some(()),
                    ..Default::default()
                },
            )
        } else {
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate::none(),
            )
        }
    }

    fn handle_navigate_up(ctx: &ActionContext) -> (ActionResult, ActionStateUpdate) {
        if ctx.focus == Focus::Menu {
            let next_idx = if ctx.menu_selected_index > 0 {
                ctx.menu_selected_index - 1
            } else {
                ctx.menu_selected_index
            };
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate {
                    menu_selected_index: Some(next_idx),
                    ..Default::default()
                },
            )
        } else {
            let update = match ctx.current_view {
                AppMode::Dashboard => {
                    if ctx.selected_project_index > 0 {
                        ActionStateUpdate {
                            selected_project_index: Some(ctx.selected_project_index - 1),
                            clamp_selections: Some(()),
                            ..Default::default()
                        }
                    } else {
                        ActionStateUpdate::none()
                    }
                }
                AppMode::Changes => {
                    if ctx.selected_change_index > 0 {
                        ActionStateUpdate {
                            selected_change_index: Some(ctx.selected_change_index - 1),
                            ..Default::default()
                        }
                    } else {
                        ActionStateUpdate::none()
                    }
                }
                AppMode::CommitHistory => {
                    if ctx.selected_commit_index > 0 {
                        ActionStateUpdate {
                            selected_commit_index: Some(ctx.selected_commit_index - 1),
                            ..Default::default()
                        }
                    } else {
                        ActionStateUpdate::none()
                    }
                }
                AppMode::BranchManager => {
                    if ctx.selected_branch_index > 0 {
                        ActionStateUpdate {
                            selected_branch_index: Some(ctx.selected_branch_index - 1),
                            ..Default::default()
                        }
                    } else {
                        ActionStateUpdate::none()
                    }
                }
                AppMode::ProjectBoard => ActionStateUpdate {
                    navigate_board_up: Some(()),
                    ..Default::default()
                },
                AppMode::MergeVisualizer => {
                    if ctx.selected_merge_file_index > 0 {
                        ActionStateUpdate {
                            selected_merge_file_index: Some(ctx.selected_merge_file_index - 1),
                            ..Default::default()
                        }
                    } else {
                        ActionStateUpdate::none()
                    }
                }
                AppMode::ModuleManager => {
                    if ctx.selected_module_index > 0 {
                        ActionStateUpdate {
                            selected_module_index: Some(ctx.selected_module_index - 1),
                            ..Default::default()
                        }
                    } else {
                        ActionStateUpdate::none()
                    }
                }
                AppMode::Settings => {
                    if ctx.selected_setting_index > 0 {
                        ActionStateUpdate {
                            selected_setting_index: Some(ctx.selected_setting_index - 1),
                            ..Default::default()
                        }
                    } else {
                        ActionStateUpdate::none()
                    }
                }
            };
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                update,
            )
        }
    }

    fn handle_navigate_down(ctx: &ActionContext) -> (ActionResult, ActionStateUpdate) {
        if ctx.focus == Focus::Menu {
            let next_idx = if ctx.menu_selected_index < 7 {
                ctx.menu_selected_index + 1
            } else {
                ctx.menu_selected_index
            };
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate {
                    menu_selected_index: Some(next_idx),
                    ..Default::default()
                },
            )
        } else {
            let update = match ctx.current_view {
                AppMode::Dashboard => ActionStateUpdate {
                    navigate_project_down: Some(()),
                    ..Default::default()
                },
                AppMode::Changes => ActionStateUpdate {
                    navigate_change_down: Some(()),
                    ..Default::default()
                },
                AppMode::CommitHistory => {
                    if ctx.selected_commit_index < ctx.cached_commits_len.saturating_sub(1) {
                        ActionStateUpdate {
                            selected_commit_index: Some(ctx.selected_commit_index + 1),
                            ..Default::default()
                        }
                    } else {
                        ActionStateUpdate::none()
                    }
                }
                AppMode::BranchManager => {
                    if ctx.selected_branch_index < ctx.cached_branches_len.saturating_sub(1) {
                        ActionStateUpdate {
                            selected_branch_index: Some(ctx.selected_branch_index + 1),
                            ..Default::default()
                        }
                    } else {
                        ActionStateUpdate::none()
                    }
                }
                AppMode::ProjectBoard => ActionStateUpdate {
                    navigate_board_down: Some(()),
                    ..Default::default()
                },
                AppMode::MergeVisualizer => ActionStateUpdate {
                    navigate_merge_down: Some(()),
                    ..Default::default()
                },
                AppMode::ModuleManager => {
                    // Get module count from context would require passing more data
                    // For now, increment and let main.rs clamp it
                    ActionStateUpdate {
                        selected_module_index: Some(ctx.selected_module_index + 1),
                        ..Default::default()
                    }
                }
                AppMode::Settings => ActionStateUpdate {
                    navigate_settings_down: Some(()),
                    ..Default::default()
                },
            };
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                update,
            )
        }
    }

    fn handle_navigate_left(ctx: &ActionContext) -> (ActionResult, ActionStateUpdate) {
        if ctx.focus == Focus::View {
            let update = match ctx.current_view {
                AppMode::ProjectBoard => ActionStateUpdate {
                    navigate_board_left: Some(()),
                    ..Default::default()
                },
                AppMode::MergeVisualizer => ActionStateUpdate {
                    merge_focus_prev: Some(()),
                    ..Default::default()
                },
                _ => ActionStateUpdate::none(),
            };
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                update,
            )
        } else {
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate::none(),
            )
        }
    }

    fn handle_navigate_right(ctx: &ActionContext) -> (ActionResult, ActionStateUpdate) {
        if ctx.focus == Focus::View {
            let update = match ctx.current_view {
                AppMode::ProjectBoard => ActionStateUpdate {
                    navigate_board_right: Some(()),
                    ..Default::default()
                },
                AppMode::MergeVisualizer => ActionStateUpdate {
                    merge_focus_next: Some(()),
                    ..Default::default()
                },
                _ => ActionStateUpdate::none(),
            };
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                update,
            )
        } else {
            (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate::none(),
            )
        }
    }
}

/// Structural representation of state changes requested by action handlers
#[derive(Debug, Default, Clone)]
pub struct ActionStateUpdate {
    // Focus and mode
    pub focus: Option<Focus>,
    pub current_view: Option<AppMode>,
    pub show_help: Option<bool>,

    // Search state
    pub search_active: Option<bool>,
    pub search_buffer: Option<String>,
    pub search_buffer_append: Option<char>,
    pub search_buffer_pop: Option<()>,

    // Selection state
    pub menu_selected_index: Option<usize>,
    pub selected_project_index: Option<usize>,
    pub selected_change_index: Option<usize>,
    pub selected_board_column: Option<usize>,
    pub selected_board_item: Option<usize>,
    pub selected_merge_file_index: Option<usize>,
    pub selected_setting_index: Option<usize>,
    // New view selections
    pub selected_commit_index: Option<usize>,
    pub selected_branch_index: Option<usize>,
    pub selected_module_index: Option<usize>,
    pub selected_developer_index: Option<usize>,

    // Commit message
    pub commit_message_append: Option<char>,
    pub commit_message_pop: Option<()>,
    pub commit_message_clear: Option<()>,

    // Scroll state
    pub project_scroll_up: Option<usize>,
    pub project_scroll_down: Option<usize>,
    pub changes_scroll_up: Option<usize>,
    pub changes_scroll_down: Option<usize>,
    pub merge_scroll_up: Option<usize>,
    pub merge_scroll_down: Option<usize>,

    // Complex actions
    pub clamp_selections: Option<()>,
    pub navigate_project_down: Option<()>,
    pub navigate_change_down: Option<()>,
    pub navigate_board_up: Option<()>,
    pub navigate_board_down: Option<()>,
    pub navigate_board_left: Option<()>,
    pub navigate_board_right: Option<()>,
    pub navigate_merge_down: Option<()>,
    pub merge_focus_next: Option<()>,
    pub merge_focus_prev: Option<()>,
    pub navigate_settings_down: Option<()>,

    // Commands
    pub move_board_item: Option<()>,
    pub accept_merge_pane: Option<()>,
    pub toggle_setting: Option<()>,
    pub commit_requested: Option<()>,
}

impl ActionStateUpdate {
    pub fn none() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_basic_keys() {
        let mut kh = KeyHandler::new();

        let quit = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('q'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(quit, KeyAction::Quit);

        let help = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('?'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(help, KeyAction::Help);

        let tab = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Tab,
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(tab, KeyAction::NextView);

        let up = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Up,
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(up, KeyAction::NavigateUp);

        let ch = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('x'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(ch, KeyAction::InputChar('x'));
    }
}
