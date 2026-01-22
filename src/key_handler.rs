use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::ui_utils::adjust_pane_ratio;
use crate::{AppMode, Focus};

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
    SwitchModuleList,
    ToggleStaging,
    Fetch,
    Push,
    Pull,
    PaneNarrow,
    PaneWiden,
    TerminalResized,
    None,
}

#[derive(Debug)]
pub struct KeyHandler;

impl Default for KeyHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_crossterm_events(&mut self) -> color_eyre::Result<KeyAction> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => Ok(self.on_key_event(key)),
            Event::Mouse(_) => Ok(KeyAction::None),
            Event::Resize(_, _) => Ok(KeyAction::TerminalResized),
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
            (KeyModifiers::CONTROL, KeyCode::Char('l') | KeyCode::Char('L')) => KeyAction::Pull,
            (KeyModifiers::NONE, KeyCode::Tab) => KeyAction::NextView,
            (KeyModifiers::NONE, KeyCode::Up | KeyCode::Char('k')) => KeyAction::NavigateUp,
            (KeyModifiers::NONE, KeyCode::Down | KeyCode::Char('j')) => KeyAction::NavigateDown,
            (KeyModifiers::NONE, KeyCode::Left | KeyCode::Char('h')) => KeyAction::NavigateLeft,
            (KeyModifiers::NONE, KeyCode::Right | KeyCode::Char('l')) => KeyAction::NavigateRight,
            (KeyModifiers::ALT, KeyCode::Left) => KeyAction::PaneNarrow,
            (KeyModifiers::ALT, KeyCode::Right) => KeyAction::PaneWiden,
            (KeyModifiers::NONE, KeyCode::PageUp) => KeyAction::ScrollPageUp,
            (KeyModifiers::NONE, KeyCode::PageDown) => KeyAction::ScrollPageDown,
            (KeyModifiers::NONE, KeyCode::Enter) => KeyAction::Select,
            (KeyModifiers::NONE, KeyCode::Backspace) => KeyAction::Backspace,
            (KeyModifiers::NONE, KeyCode::Char(' ')) => KeyAction::ToggleStaging,
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

/// Context passed to action handlers to enable decision-making.
///
/// # Input Mode Handling
///
/// The `commit_message_empty` field enables context-aware keybindings in the Changes view.
/// This solves the conflict between shortcut keys ('f' for fetch, 'p' for push) and
/// typing those same characters in the commit message input.
///
/// ## Current Design (Phase 6c - Jan 2026)
///
/// **Pattern**: Conditional shortcut activation based on input state
/// - When commit message is empty: 'f' and 'p' trigger fetch/push
/// - When commit message has content: 'f' and 'p' are typed literally
///
/// **Pros**:
/// - Simple implementation (single boolean flag)
/// - Intuitive UX: shortcuts work when input is unused
/// - Minimal state management overhead
/// - Works well for current two-shortcut use case
///
/// **Cons**:
/// - Scales poorly with more shortcuts (would need more conditionals)
/// - Implicit behavior - not obvious from code structure
/// - Mixed concerns: input state affects command routing
///
/// ## Alternative: Explicit Input Mode State Machine
///
/// **Pattern**: Dedicated `InputMode` enum with explicit transitions
/// ```rust
/// enum InputMode {
///     Normal,        // Shortcuts active, no text input
///     Typing,        // All chars go to input buffer
///     Search,        // Search-specific input mode
/// }
/// ```
///
/// **Pros**:
/// - Explicit state transitions (escape to exit typing mode)
/// - Scales better with more shortcuts and input contexts
/// - Clear separation of concerns
/// - Common pattern in TUI apps (vim-style modal editing)
///
/// **Cons**:
/// - Requires user to explicitly enter/exit typing mode (less intuitive)
/// - More state to manage (mode transitions, visual indicators)
/// - Breaking UX change from current behavior
///
/// ## Recommendation
///
/// **Keep current design** for now because:
/// 1. Only 2 shortcuts ('f'/'p') currently conflict with commit message input
/// 2. Current UX is intuitive: "shortcuts work when I'm not typing"
/// 3. No plans for many more single-character shortcuts in Changes view
/// 4. Alternative would require modal editing learning curve
///
/// **Future consideration**: If more shortcuts are needed, evaluate:
/// - Using multi-key chords (Ctrl+F, Ctrl+P) instead of single chars
/// - Adding explicit "focus commit message" action (Tab key?)
/// - Implementing full modal editing if app grows significantly
///
/// See: CHANGELOG.md Phase 6c, src/key_handler.rs lines 292, 302, 819
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
    pub changes_pane_ratio: u16,
    pub commit_pane_ratio: u16,
    pub module_pane_ratio: u16,
    pub dashboard_pane_ratio: u16,
    // New view indices
    pub selected_commit_index: usize,
    pub selected_branch_index: usize,
    pub selected_module_index: usize,
    pub selected_developer_index: usize,
    pub cached_commits_len: usize,
    pub cached_branches_len: usize,
    pub branch_create_mode: bool,
    pub branch_input_empty: bool,
    pub module_manager_in_developer_list: bool,
    pub module_create_mode: bool,
    pub module_edit_mode: bool,
    pub developer_create_mode: bool,
    pub module_assign_mode: bool,
    pub module_input_empty: bool,
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
                if ctx.branch_create_mode {
                    return (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Cancelled branch creation".into()),
                        },
                        ActionStateUpdate {
                            branch_create_mode: Some(false),
                            branch_input_clear: Some(()),
                            ..Default::default()
                        },
                    );
                }
                if ctx.module_create_mode
                    || ctx.module_edit_mode
                    || ctx.developer_create_mode
                    || ctx.module_assign_mode
                {
                    return (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Cancelled".into()),
                        },
                        ActionStateUpdate {
                            module_create_mode: Some(false),
                            module_edit_mode: Some(false),
                            developer_create_mode: Some(false),
                            module_assign_mode: Some(false),
                            module_input_clear: Some(()),
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
                    match c {
                        // Conditional shortcuts: only active when commit message is empty
                        // This prevents 'f' and 'p' from triggering fetch/push while typing
                        // Alternative considered: explicit input mode (see ActionContext docs)
                        'f' if ctx.commit_message_empty => (
                            ActionResult {
                                should_quit: false,
                                status_message: Some("Fetching from origin...".into()),
                            },
                            ActionStateUpdate {
                                fetch_requested: Some(()),
                                ..Default::default()
                            },
                        ),
                        'p' if ctx.commit_message_empty => (
                            ActionResult {
                                should_quit: false,
                                status_message: Some("Pushing to origin...".into()),
                            },
                            ActionStateUpdate {
                                push_requested: Some(()),
                                ..Default::default()
                            },
                        ),
                        _ => (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate {
                                commit_message_append: Some(c),
                                ..Default::default()
                            },
                        ),
                    }
                } else if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::Dashboard)
                {
                    match c {
                        'f' => (
                            ActionResult {
                                should_quit: false,
                                status_message: Some("Fetching from origin...".into()),
                            },
                            ActionStateUpdate {
                                fetch_requested: Some(()),
                                ..Default::default()
                            },
                        ),
                        _ => (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate::none(),
                        ),
                    }
                } else if ctx.focus == Focus::View
                    && matches!(ctx.current_view, AppMode::BranchManager)
                {
                    // Handle branch-specific actions
                    match c {
                        'n' if !ctx.branch_create_mode => (
                            ActionResult {
                                should_quit: false,
                                status_message: Some(
                                    "Enter branch name (Enter to create, Esc to cancel)".into(),
                                ),
                            },
                            ActionStateUpdate {
                                branch_create_mode: Some(true),
                                ..Default::default()
                            },
                        ),
                        'd' if !ctx.branch_create_mode => (
                            ActionResult {
                                should_quit: false,
                                status_message: Some("Deleting branch...".into()),
                            },
                            ActionStateUpdate {
                                branch_delete_requested: Some(()),
                                ..Default::default()
                            },
                        ),
                        _ if ctx.branch_create_mode => (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate {
                                branch_input_append: Some(c),
                                ..Default::default()
                            },
                        ),
                        _ => (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate::none(),
                        ),
                    }
                } else if ctx.focus == Focus::View
                    && matches!(ctx.current_view, AppMode::ModuleManager)
                {
                    // Handle module manager specific actions
                    match c {
                        'a' if !ctx.module_create_mode
                            && !ctx.module_edit_mode
                            && !ctx.developer_create_mode
                            && !ctx.module_manager_in_developer_list =>
                        {
                            (
                                ActionResult {
                                    should_quit: false,
                                    status_message: Some(
                                        "Press ↑↓ to select developer, Enter to assign".into(),
                                    ),
                                },
                                ActionStateUpdate {
                                    module_assign_mode: Some(true),
                                    ..Default::default()
                                },
                            )
                        }
                        'n' if !ctx.module_create_mode
                            && !ctx.module_edit_mode
                            && !ctx.developer_create_mode =>
                        {
                            let in_developer_list = ctx.module_manager_in_developer_list;
                            (
                                ActionResult {
                                    should_quit: false,
                                    status_message: Some(if in_developer_list {
                                        "Enter developer name (Enter to create, Esc to cancel)"
                                            .into()
                                    } else {
                                        "Enter module name (Enter to create, Esc to cancel)".into()
                                    }),
                                },
                                ActionStateUpdate {
                                    module_create_mode: if !in_developer_list {
                                        Some(true)
                                    } else {
                                        None
                                    },
                                    developer_create_mode: if in_developer_list {
                                        Some(true)
                                    } else {
                                        None
                                    },
                                    ..Default::default()
                                },
                            )
                        }
                        'e' if !ctx.module_create_mode
                            && !ctx.module_edit_mode
                            && !ctx.developer_create_mode
                            && !ctx.module_manager_in_developer_list =>
                        {
                            (
                                ActionResult {
                                    should_quit: false,
                                    status_message: Some(
                                        "Edit module name (Enter to save, Esc to cancel)".into(),
                                    ),
                                },
                                ActionStateUpdate {
                                    module_edit_mode: Some(true),
                                    module_load_selected: Some(()),
                                    ..Default::default()
                                },
                            )
                        }
                        'd' if !ctx.module_create_mode
                            && !ctx.module_edit_mode
                            && !ctx.developer_create_mode =>
                        {
                            let in_developer_list = ctx.module_manager_in_developer_list;
                            (
                                ActionResult {
                                    should_quit: false,
                                    status_message: Some("Deleting...".into()),
                                },
                                ActionStateUpdate {
                                    module_delete_requested: if !in_developer_list {
                                        Some(())
                                    } else {
                                        None
                                    },
                                    developer_delete_requested: if in_developer_list {
                                        Some(())
                                    } else {
                                        None
                                    },
                                    ..Default::default()
                                },
                            )
                        }
                        _ if ctx.module_create_mode
                            || ctx.module_edit_mode
                            || ctx.developer_create_mode =>
                        {
                            (
                                ActionResult {
                                    should_quit: false,
                                    status_message: None,
                                },
                                ActionStateUpdate {
                                    module_input_append: Some(c),
                                    ..Default::default()
                                },
                            )
                        }
                        _ => (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate::none(),
                        ),
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
                } else if ctx.focus == Focus::View
                    && matches!(ctx.current_view, AppMode::BranchManager)
                    && ctx.branch_create_mode
                {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate {
                            branch_input_pop: Some(()),
                            ..Default::default()
                        },
                    )
                } else if ctx.focus == Focus::View
                    && matches!(ctx.current_view, AppMode::ModuleManager)
                    && (ctx.module_create_mode || ctx.module_edit_mode || ctx.developer_create_mode)
                {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate {
                            module_input_pop: Some(()),
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
            KeyAction::SwitchModuleList => {
                if matches!(ctx.current_view, AppMode::ModuleManager)
                    && !ctx.module_create_mode
                    && !ctx.module_edit_mode
                    && !ctx.developer_create_mode
                {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate {
                            toggle_module_list: Some(()),
                            ..Default::default()
                        },
                    )
                } else {
                    // Fall back to NextView if not in module manager
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
            KeyAction::ToggleStaging => {
                if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::Changes) {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Toggling staging...".into()),
                        },
                        ActionStateUpdate {
                            toggle_staging_requested: Some(()),
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
            KeyAction::PaneNarrow => {
                if ctx.focus == Focus::View {
                    let update = match ctx.current_view {
                        AppMode::Changes => ActionStateUpdate {
                            changes_pane_ratio: Some(adjust_pane_ratio(ctx.changes_pane_ratio, -5)),
                            ..Default::default()
                        },
                        AppMode::CommitHistory => ActionStateUpdate {
                            commit_pane_ratio: Some(adjust_pane_ratio(ctx.commit_pane_ratio, -5)),
                            ..Default::default()
                        },
                        AppMode::ModuleManager => ActionStateUpdate {
                            module_pane_ratio: Some(adjust_pane_ratio(ctx.module_pane_ratio, -5)),
                            ..Default::default()
                        },
                        AppMode::Dashboard => ActionStateUpdate {
                            dashboard_pane_ratio: Some(adjust_pane_ratio(
                                ctx.dashboard_pane_ratio,
                                -5,
                            )),
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
            KeyAction::PaneWiden => {
                if ctx.focus == Focus::View {
                    let update = match ctx.current_view {
                        AppMode::Changes => ActionStateUpdate {
                            changes_pane_ratio: Some(adjust_pane_ratio(ctx.changes_pane_ratio, 5)),
                            ..Default::default()
                        },
                        AppMode::CommitHistory => ActionStateUpdate {
                            commit_pane_ratio: Some(adjust_pane_ratio(ctx.commit_pane_ratio, 5)),
                            ..Default::default()
                        },
                        AppMode::ModuleManager => ActionStateUpdate {
                            module_pane_ratio: Some(adjust_pane_ratio(ctx.module_pane_ratio, 5)),
                            ..Default::default()
                        },
                        AppMode::Dashboard => ActionStateUpdate {
                            dashboard_pane_ratio: Some(adjust_pane_ratio(
                                ctx.dashboard_pane_ratio,
                                5,
                            )),
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
            KeyAction::Fetch => {
                if ctx.focus == Focus::View
                    && (matches!(ctx.current_view, AppMode::Dashboard | AppMode::Changes))
                {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Fetching from origin...".into()),
                        },
                        ActionStateUpdate {
                            fetch_requested: Some(()),
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
            KeyAction::Push => {
                if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::Changes) {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Pushing to origin...".into()),
                        },
                        ActionStateUpdate {
                            push_requested: Some(()),
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
            KeyAction::Pull => {
                if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::Changes) {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Pulling from origin...".into()),
                        },
                        ActionStateUpdate {
                            pull_requested: Some(()),
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
            KeyAction::TerminalResized => (
                ActionResult {
                    should_quit: false,
                    status_message: None,
                },
                ActionStateUpdate::none(),
            ),
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
        } else if matches!(ctx.current_view, AppMode::Dashboard) {
            // Switch to Changes view when pressing Enter on a project
            (
                ActionResult {
                    should_quit: false,
                    status_message: Some("Switching to Changes...".into()),
                },
                ActionStateUpdate {
                    current_view: Some(AppMode::Changes),
                    menu_selected_index: Some(AppMode::Changes.menu_index()),
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
        } else if matches!(ctx.current_view, AppMode::BranchManager) {
            if ctx.branch_create_mode {
                // Create branch
                if ctx.branch_input_empty {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Branch name cannot be empty".into()),
                        },
                        ActionStateUpdate::none(),
                    )
                } else {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Creating branch...".into()),
                        },
                        ActionStateUpdate {
                            branch_create_requested: Some(()),
                            ..Default::default()
                        },
                    )
                }
            } else {
                // Switch branch
                (
                    ActionResult {
                        should_quit: false,
                        status_message: Some("Switching branch...".into()),
                    },
                    ActionStateUpdate {
                        branch_switch_requested: Some(()),
                        ..Default::default()
                    },
                )
            }
        } else if matches!(ctx.current_view, AppMode::ModuleManager) {
            if ctx.module_assign_mode {
                (
                    ActionResult {
                        should_quit: false,
                        status_message: Some("Assigning developer to module...".into()),
                    },
                    ActionStateUpdate {
                        module_assign_requested: Some(()),
                        ..Default::default()
                    },
                )
            } else if ctx.module_create_mode {
                if ctx.module_input_empty {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Module name cannot be empty".into()),
                        },
                        ActionStateUpdate::none(),
                    )
                } else {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Creating module...".into()),
                        },
                        ActionStateUpdate {
                            module_create_requested: Some(()),
                            ..Default::default()
                        },
                    )
                }
            } else if ctx.module_edit_mode {
                if ctx.module_input_empty {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Module name cannot be empty".into()),
                        },
                        ActionStateUpdate::none(),
                    )
                } else {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Updating module...".into()),
                        },
                        ActionStateUpdate {
                            module_update_requested: Some(()),
                            ..Default::default()
                        },
                    )
                }
            } else if ctx.developer_create_mode {
                if ctx.module_input_empty {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Developer name cannot be empty".into()),
                        },
                        ActionStateUpdate::none(),
                    )
                } else {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Creating developer...".into()),
                        },
                        ActionStateUpdate {
                            developer_create_requested: Some(()),
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
            let next_idx = ctx.menu_selected_index.saturating_sub(1);
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
                    selected_project_index: Some(ctx.selected_project_index.saturating_sub(1)),
                    clamp_selections: Some(()),
                    ..Default::default()
                },
                AppMode::Changes => ActionStateUpdate {
                    selected_change_index: Some(ctx.selected_change_index.saturating_sub(1)),
                    ..Default::default()
                },
                AppMode::CommitHistory => ActionStateUpdate {
                    selected_commit_index: Some(ctx.selected_commit_index.saturating_sub(1)),
                    ..Default::default()
                },
                AppMode::BranchManager => ActionStateUpdate {
                    selected_branch_index: Some(ctx.selected_branch_index.saturating_sub(1)),
                    ..Default::default()
                },
                AppMode::ProjectBoard => ActionStateUpdate {
                    navigate_board_up: Some(()),
                    ..Default::default()
                },
                AppMode::MergeVisualizer => ActionStateUpdate {
                    selected_merge_file_index: Some(
                        ctx.selected_merge_file_index.saturating_sub(1),
                    ),
                    ..Default::default()
                },
                AppMode::ModuleManager => {
                    if ctx.module_assign_mode {
                        ActionStateUpdate {
                            selected_developer_index: Some(
                                ctx.selected_developer_index.saturating_sub(1),
                            ),
                            ..Default::default()
                        }
                    } else {
                        ActionStateUpdate {
                            selected_module_index: Some(
                                ctx.selected_module_index.saturating_sub(1),
                            ),
                            ..Default::default()
                        }
                    }
                }
                AppMode::Settings => ActionStateUpdate {
                    selected_setting_index: Some(ctx.selected_setting_index.saturating_sub(1)),
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

    fn handle_navigate_down(ctx: &ActionContext) -> (ActionResult, ActionStateUpdate) {
        if ctx.focus == Focus::Menu {
            let next_idx = (ctx.menu_selected_index + 1).min(7);
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
                    if ctx.module_assign_mode {
                        // Navigate developer list in assign mode
                        ActionStateUpdate {
                            selected_developer_index: Some(ctx.selected_developer_index + 1),
                            ..Default::default()
                        }
                    } else {
                        // Get module count from context would require passing more data
                        // For now, increment and let main.rs clamp it
                        ActionStateUpdate {
                            selected_module_index: Some(ctx.selected_module_index + 1),
                            ..Default::default()
                        }
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

    // Layout adjustments
    pub changes_pane_ratio: Option<u16>,
    pub commit_pane_ratio: Option<u16>,
    pub module_pane_ratio: Option<u16>,
    pub dashboard_pane_ratio: Option<u16>,

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

    // Branch operations
    pub branch_create_mode: Option<bool>,
    pub branch_input_append: Option<char>,
    pub branch_input_pop: Option<()>,
    pub branch_input_clear: Option<()>,
    pub branch_switch_requested: Option<()>,
    pub branch_create_requested: Option<()>,
    pub branch_delete_requested: Option<()>,

    // Module operations
    pub toggle_module_list: Option<()>,
    pub module_create_mode: Option<bool>,
    pub module_edit_mode: Option<bool>,
    pub developer_create_mode: Option<bool>,
    pub module_input_append: Option<char>,
    pub module_input_pop: Option<()>,
    pub module_input_clear: Option<()>,
    pub module_load_selected: Option<()>,
    pub module_create_requested: Option<()>,
    pub module_update_requested: Option<()>,
    pub module_delete_requested: Option<()>,
    pub developer_create_requested: Option<()>,
    pub developer_delete_requested: Option<()>,
    pub module_assign_mode: Option<bool>,
    pub module_assign_requested: Option<()>,

    // File staging
    pub toggle_staging_requested: Option<()>,

    // Remote operations
    pub fetch_requested: Option<()>,
    pub push_requested: Option<()>,
    pub pull_requested: Option<()>,
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

    #[test]
    fn test_navigation_keys() {
        let mut kh = KeyHandler::new();

        let down = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Down,
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(down, KeyAction::NavigateDown);

        let left = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Left,
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(left, KeyAction::NavigateLeft);

        let right = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Right,
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(right, KeyAction::NavigateRight);
    }

    #[test]
    fn test_vim_keybindings() {
        let mut kh = KeyHandler::new();

        let up_vim = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('k'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(up_vim, KeyAction::NavigateUp);

        let down_vim = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('j'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(down_vim, KeyAction::NavigateDown);

        let left_vim = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('h'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(left_vim, KeyAction::NavigateLeft);

        let right_vim = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('l'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(right_vim, KeyAction::NavigateRight);
    }

    #[test]
    fn test_modifier_keys() {
        let mut kh = KeyHandler::new();

        let ctrl_c = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('c'),
            modifiers: crossterm::event::KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(ctrl_c, KeyAction::Quit);

        let ctrl_f = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('f'),
            modifiers: crossterm::event::KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(ctrl_f, KeyAction::Search);
    }

    #[test]
    fn test_staging_and_enter() {
        let mut kh = KeyHandler::new();

        let space = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char(' '),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(space, KeyAction::ToggleStaging);

        let enter = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Enter,
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(enter, KeyAction::Select);
    }

    #[test]
    fn test_page_keys() {
        let mut kh = KeyHandler::new();

        let page_up = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::PageUp,
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(page_up, KeyAction::ScrollPageUp);

        let page_down = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::PageDown,
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(page_down, KeyAction::ScrollPageDown);
    }

    #[test]
    fn test_backspace() {
        let mut kh = KeyHandler::new();

        let backspace = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Backspace,
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(backspace, KeyAction::Backspace);
    }

    #[test]
    fn test_action_result_creation() {
        let result = ActionResult {
            should_quit: false,
            status_message: Some("Test message".to_string()),
        };

        assert!(!result.should_quit);
        assert_eq!(result.status_message, Some("Test message".to_string()));
    }

    #[test]
    fn test_action_state_update_none() {
        let update = ActionStateUpdate::none();
        assert!(update.focus.is_none());
        assert!(update.current_view.is_none());
        assert!(update.show_help.is_none());
    }
}
