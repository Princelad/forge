use std::collections::HashMap;
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use serde::Deserialize;

use crate::ui_utils::adjust_pane_ratio;
use crate::{AppMode, Focus, InputMode};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct KeyChord {
    modifiers: KeyModifiers,
    code: KeyCode,
}

impl KeyChord {
    fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { modifiers, code }
    }

    fn from_event(event: &KeyEvent) -> Self {
        Self {
            modifiers: event.modifiers,
            code: event.code,
        }
    }
}

#[derive(Debug, Deserialize)]
struct KeybindingsConfig {
    bindings: HashMap<String, BindingValue>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BindingValue {
    Single(String),
    Multiple(Vec<String>),
}

impl BindingValue {
    fn entries(&self) -> Vec<&str> {
        match self {
            BindingValue::Single(value) => vec![value.as_str()],
            BindingValue::Multiple(values) => values.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Debug)]
pub struct KeyHandler {
    keymap: HashMap<KeyChord, KeyAction>,
}

impl Default for KeyHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyHandler {
    pub fn new() -> Self {
        Self {
            keymap: default_keymap(),
        }
    }

    pub fn load_keybindings_from(&mut self, workdir: &Path) -> std::io::Result<()> {
        let path = workdir.join(".forge").join("keybindings.toml");
        if !path.exists() {
            return Ok(());
        }

        let contents = std::fs::read_to_string(path)?;
        let config: KeybindingsConfig = toml::from_str(&contents)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        apply_keybindings(&mut self.keymap, config)?;
        Ok(())
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
        let chord = KeyChord::from_event(&key);
        if let Some(action) = self.keymap.get(&chord) {
            return action.clone();
        }

        if matches!(key.code, KeyCode::Char('q')) {
            return KeyAction::Quit;
        }
        if matches!(key.code, KeyCode::Char('?')) {
            return KeyAction::Help;
        }

        match key.code {
            KeyCode::Char(c) => KeyAction::InputChar(c),
            _ => KeyAction::None,
        }
    }
}

fn default_keymap() -> HashMap<KeyChord, KeyAction> {
    let mut map = HashMap::new();
    map.insert(
        KeyChord::new(KeyCode::Esc, KeyModifiers::NONE),
        KeyAction::Back,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('q'), KeyModifiers::NONE),
        KeyAction::Quit,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        KeyAction::Quit,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('C'), KeyModifiers::CONTROL),
        KeyAction::Quit,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('?'), KeyModifiers::NONE),
        KeyAction::Help,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        KeyAction::Search,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('F'), KeyModifiers::CONTROL),
        KeyAction::Search,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
        KeyAction::Pull,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('L'), KeyModifiers::CONTROL),
        KeyAction::Pull,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('f'), KeyModifiers::ALT),
        KeyAction::Fetch,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('F'), KeyModifiers::ALT),
        KeyAction::Fetch,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('p'), KeyModifiers::ALT),
        KeyAction::Push,
    );
    map.insert(
        KeyChord::new(KeyCode::Char('P'), KeyModifiers::ALT),
        KeyAction::Push,
    );
    map.insert(
        KeyChord::new(KeyCode::Tab, KeyModifiers::NONE),
        KeyAction::NextView,
    );
    map.insert(
        KeyChord::new(KeyCode::Up, KeyModifiers::NONE),
        KeyAction::NavigateUp,
    );
    map.insert(
        KeyChord::new(KeyCode::Down, KeyModifiers::NONE),
        KeyAction::NavigateDown,
    );
    map.insert(
        KeyChord::new(KeyCode::Left, KeyModifiers::NONE),
        KeyAction::NavigateLeft,
    );
    map.insert(
        KeyChord::new(KeyCode::Right, KeyModifiers::NONE),
        KeyAction::NavigateRight,
    );
    map.insert(
        KeyChord::new(KeyCode::Left, KeyModifiers::ALT),
        KeyAction::PaneNarrow,
    );
    map.insert(
        KeyChord::new(KeyCode::Right, KeyModifiers::ALT),
        KeyAction::PaneWiden,
    );
    map.insert(
        KeyChord::new(KeyCode::PageUp, KeyModifiers::NONE),
        KeyAction::ScrollPageUp,
    );
    map.insert(
        KeyChord::new(KeyCode::PageDown, KeyModifiers::NONE),
        KeyAction::ScrollPageDown,
    );
    map.insert(
        KeyChord::new(KeyCode::Enter, KeyModifiers::NONE),
        KeyAction::Select,
    );
    map.insert(
        KeyChord::new(KeyCode::Backspace, KeyModifiers::NONE),
        KeyAction::Backspace,
    );
    map.insert(
        KeyChord::new(KeyCode::Char(' '), KeyModifiers::NONE),
        KeyAction::ToggleStaging,
    );
    map
}

fn apply_keybindings(
    keymap: &mut HashMap<KeyChord, KeyAction>,
    config: KeybindingsConfig,
) -> std::io::Result<()> {
    for (action_name, binding) in config.bindings {
        let action = action_from_name(&action_name).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown keybinding action: {}", action_name),
            )
        })?;
        for entry in binding.entries() {
            let chord = parse_key_chord(entry).map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid keybinding '{}': {}", entry, err),
                )
            })?;
            keymap.insert(chord, action.clone());
        }
    }
    Ok(())
}

fn action_from_name(name: &str) -> Option<KeyAction> {
    let normalized = name.trim().to_ascii_lowercase().replace('-', "_");
    match normalized.as_str() {
        "quit" => Some(KeyAction::Quit),
        "back" => Some(KeyAction::Back),
        "next_view" => Some(KeyAction::NextView),
        "navigate_up" => Some(KeyAction::NavigateUp),
        "navigate_down" => Some(KeyAction::NavigateDown),
        "navigate_left" => Some(KeyAction::NavigateLeft),
        "navigate_right" => Some(KeyAction::NavigateRight),
        "scroll_page_up" => Some(KeyAction::ScrollPageUp),
        "scroll_page_down" => Some(KeyAction::ScrollPageDown),
        "select" => Some(KeyAction::Select),
        "help" => Some(KeyAction::Help),
        "search" => Some(KeyAction::Search),
        "backspace" => Some(KeyAction::Backspace),
        "toggle_staging" => Some(KeyAction::ToggleStaging),
        "fetch" => Some(KeyAction::Fetch),
        "push" => Some(KeyAction::Push),
        "pull" => Some(KeyAction::Pull),
        "pane_narrow" => Some(KeyAction::PaneNarrow),
        "pane_widen" => Some(KeyAction::PaneWiden),
        "switch_module_list" => Some(KeyAction::SwitchModuleList),
        _ => None,
    }
}

fn parse_key_chord(raw: &str) -> Result<KeyChord, String> {
    let mut modifiers = KeyModifiers::NONE;
    let mut code: Option<KeyCode> = None;

    for part in raw.split('+') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let lowered = part.to_ascii_lowercase();
        match lowered.as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "esc" | "escape" => code = assign_key_code(code, KeyCode::Esc)?,
            "enter" | "return" => code = assign_key_code(code, KeyCode::Enter)?,
            "tab" => code = assign_key_code(code, KeyCode::Tab)?,
            "backspace" => code = assign_key_code(code, KeyCode::Backspace)?,
            "space" => code = assign_key_code(code, KeyCode::Char(' '))?,
            "up" => code = assign_key_code(code, KeyCode::Up)?,
            "down" => code = assign_key_code(code, KeyCode::Down)?,
            "left" => code = assign_key_code(code, KeyCode::Left)?,
            "right" => code = assign_key_code(code, KeyCode::Right)?,
            "pageup" | "pgup" => code = assign_key_code(code, KeyCode::PageUp)?,
            "pagedown" | "pgdn" => code = assign_key_code(code, KeyCode::PageDown)?,
            _ => {
                if lowered.len() == 1 {
                    let ch = part.chars().next().ok_or_else(|| "Empty key".to_string())?;
                    code = assign_key_code(code, KeyCode::Char(ch))?;
                } else {
                    return Err(format!("Unsupported key token '{}'", part));
                }
            }
        }
    }

    let code = code.ok_or_else(|| "Missing key code".to_string())?;
    Ok(KeyChord::new(code, modifiers))
}

fn assign_key_code(existing: Option<KeyCode>, next: KeyCode) -> Result<Option<KeyCode>, String> {
    if existing.is_some() {
        return Err("Multiple key codes in binding".to_string());
    }
    Ok(Some(next))
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
/// The app uses a hybrid input mode: press Enter to start typing in inputs and
/// Esc to stop typing. While in `InputMode::Typing`, character keys are routed
/// to the active input buffer instead of navigation shortcuts.
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub focus: Focus,
    pub input_mode: InputMode,
    pub current_view: AppMode,
    pub show_help: bool,
    pub search_active: bool,
    pub menu_selected_index: usize,
    pub menu_len: usize,
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
    pub selected_remote: Option<String>,
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
                if ctx.input_mode == InputMode::Typing {
                    return (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Stopped typing".into()),
                        },
                        ActionStateUpdate {
                            input_mode: Some(InputMode::Normal),
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
                    let menu_len = ctx.menu_len.max(1);
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
                if ctx.input_mode == InputMode::Normal {
                    match c {
                        'k' => return Self::handle_navigate_up(ctx),
                        'j' => return Self::handle_navigate_down(ctx),
                        'h' => return Self::handle_navigate_left(ctx),
                        'l' => return Self::handle_navigate_right(ctx),
                        _ => {}
                    }
                }

                if ctx.input_mode == InputMode::Typing {
                    if ctx.search_active {
                        return (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate {
                                search_buffer_append: Some(c),
                                ..Default::default()
                            },
                        );
                    }

                    if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::Changes) {
                        return (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate {
                                commit_message_append: Some(c),
                                ..Default::default()
                            },
                        );
                    }

                    if ctx.focus == Focus::View
                        && matches!(ctx.current_view, AppMode::BranchManager)
                        && ctx.branch_create_mode
                    {
                        return (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate {
                                branch_input_append: Some(c),
                                ..Default::default()
                            },
                        );
                    }

                    if ctx.focus == Focus::View
                        && matches!(ctx.current_view, AppMode::ModuleManager)
                        && (ctx.module_create_mode
                            || ctx.module_edit_mode
                            || ctx.developer_create_mode)
                    {
                        return (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate {
                                module_input_append: Some(c),
                                ..Default::default()
                            },
                        );
                    }
                }

                if ctx.search_active {
                    return (
                        ActionResult {
                            should_quit: false,
                            status_message: None,
                        },
                        ActionStateUpdate::none(),
                    );
                }

                if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::Changes) {
                    return match c {
                        'f' => (
                            ActionResult {
                                should_quit: false,
                                status_message: ctx
                                    .selected_remote
                                    .as_deref()
                                    .map(|name| format!("Fetching from {}...", name))
                                    .or_else(|| Some("No remotes configured".into())),
                            },
                            ActionStateUpdate {
                                fetch_requested: ctx.selected_remote.as_ref().map(|_| ()),
                                ..Default::default()
                            },
                        ),
                        'p' => (
                            ActionResult {
                                should_quit: false,
                                status_message: ctx
                                    .selected_remote
                                    .as_deref()
                                    .map(|name| format!("Pushing to {}...", name))
                                    .or_else(|| Some("No remotes configured".into())),
                            },
                            ActionStateUpdate {
                                push_requested: ctx.selected_remote.as_ref().map(|_| ()),
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
                    };
                }

                if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::Dashboard) {
                    return match c {
                        'f' => (
                            ActionResult {
                                should_quit: false,
                                status_message: ctx
                                    .selected_remote
                                    .as_deref()
                                    .map(|name| format!("Fetching from {}...", name))
                                    .or_else(|| Some("No remotes configured".into())),
                            },
                            ActionStateUpdate {
                                fetch_requested: ctx.selected_remote.as_ref().map(|_| ()),
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
                    };
                }

                if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::BranchManager) {
                    return match c {
                        'n' if !ctx.branch_create_mode => (
                            ActionResult {
                                should_quit: false,
                                status_message: Some(
                                    "Press Enter to start typing a branch name".into(),
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
                        _ => (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate::none(),
                        ),
                    };
                }

                if ctx.focus == Focus::View && matches!(ctx.current_view, AppMode::ModuleManager) {
                    return match c {
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
                                        "Press Enter to start typing a developer name".into()
                                    } else {
                                        "Press Enter to start typing a module name".into()
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
                                        "Press Enter to edit the module name".into(),
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
                        _ => (
                            ActionResult {
                                should_quit: false,
                                status_message: None,
                            },
                            ActionStateUpdate::none(),
                        ),
                    };
                }

                (
                    ActionResult {
                        should_quit: false,
                        status_message: None,
                    },
                    ActionStateUpdate::none(),
                )
            }
            KeyAction::Backspace => {
                if ctx.input_mode == InputMode::Typing && ctx.search_active {
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
                } else if ctx.input_mode == InputMode::Typing
                    && ctx.focus == Focus::View
                    && matches!(ctx.current_view, AppMode::Changes)
                {
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
                } else if ctx.input_mode == InputMode::Typing
                    && ctx.focus == Focus::View
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
                } else if ctx.input_mode == InputMode::Typing
                    && ctx.focus == Focus::View
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
                                input_mode: Some(InputMode::Normal),
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
                            status_message: ctx
                                .selected_remote
                                .as_deref()
                                .map(|name| format!("Fetching from {}...", name))
                                .or_else(|| Some("No remotes configured".into())),
                        },
                        ActionStateUpdate {
                            fetch_requested: ctx.selected_remote.as_ref().map(|_| ()),
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
                            status_message: ctx
                                .selected_remote
                                .as_deref()
                                .map(|name| format!("Pushing to {}...", name))
                                .or_else(|| Some("No remotes configured".into())),
                        },
                        ActionStateUpdate {
                            push_requested: ctx.selected_remote.as_ref().map(|_| ()),
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
                            status_message: ctx
                                .selected_remote
                                .as_deref()
                                .map(|name| format!("Pulling from {}...", name))
                                .or_else(|| Some("No remotes configured".into())),
                        },
                        ActionStateUpdate {
                            pull_requested: ctx.selected_remote.as_ref().map(|_| ()),
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
        } else if matches!(ctx.current_view, AppMode::Dashboard) && ctx.search_active {
            if ctx.input_mode == InputMode::Typing {
                (
                    ActionResult {
                        should_quit: false,
                        status_message: Some("Search input stopped".into()),
                    },
                    ActionStateUpdate {
                        input_mode: Some(InputMode::Normal),
                        ..Default::default()
                    },
                )
            } else {
                (
                    ActionResult {
                        should_quit: false,
                        status_message: Some("Type to filter projects".into()),
                    },
                    ActionStateUpdate {
                        input_mode: Some(InputMode::Typing),
                        ..Default::default()
                    },
                )
            }
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
            if ctx.input_mode == InputMode::Normal {
                (
                    ActionResult {
                        should_quit: false,
                        status_message: Some("Start typing a commit message".into()),
                    },
                    ActionStateUpdate {
                        input_mode: Some(InputMode::Typing),
                        ..Default::default()
                    },
                )
            } else if ctx.commit_message_empty {
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
                        input_mode: Some(InputMode::Normal),
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
                        input_mode: Some(InputMode::Normal),
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
                if ctx.input_mode == InputMode::Normal {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Start typing a branch name".into()),
                        },
                        ActionStateUpdate {
                            input_mode: Some(InputMode::Typing),
                            ..Default::default()
                        },
                    )
                } else if ctx.branch_input_empty {
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
                            input_mode: Some(InputMode::Normal),
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
                if ctx.input_mode == InputMode::Normal {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Start typing a module name".into()),
                        },
                        ActionStateUpdate {
                            input_mode: Some(InputMode::Typing),
                            ..Default::default()
                        },
                    )
                } else if ctx.module_input_empty {
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
                            input_mode: Some(InputMode::Normal),
                            ..Default::default()
                        },
                    )
                }
            } else if ctx.module_edit_mode {
                if ctx.input_mode == InputMode::Normal {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Start typing the module name".into()),
                        },
                        ActionStateUpdate {
                            input_mode: Some(InputMode::Typing),
                            ..Default::default()
                        },
                    )
                } else if ctx.module_input_empty {
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
                            input_mode: Some(InputMode::Normal),
                            ..Default::default()
                        },
                    )
                }
            } else if ctx.developer_create_mode {
                if ctx.input_mode == InputMode::Normal {
                    (
                        ActionResult {
                            should_quit: false,
                            status_message: Some("Start typing the developer name".into()),
                        },
                        ActionStateUpdate {
                            input_mode: Some(InputMode::Typing),
                            ..Default::default()
                        },
                    )
                } else if ctx.module_input_empty {
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
                            input_mode: Some(InputMode::Normal),
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
            let max_idx = ctx.menu_len.saturating_sub(1);
            let next_idx = (ctx.menu_selected_index + 1).min(max_idx);
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
    pub input_mode: Option<InputMode>,
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
        assert_eq!(up_vim, KeyAction::InputChar('k'));

        let down_vim = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('j'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(down_vim, KeyAction::InputChar('j'));

        let left_vim = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('h'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(left_vim, KeyAction::InputChar('h'));

        let right_vim = kh.on_key_event(crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('l'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        assert_eq!(right_vim, KeyAction::InputChar('l'));
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
