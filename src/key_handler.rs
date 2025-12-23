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
    Select,
    Help,
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
            (KeyModifiers::NONE, KeyCode::Tab) => KeyAction::NextView,
            (KeyModifiers::NONE, KeyCode::Up | KeyCode::Char('k')) => KeyAction::NavigateUp,
            (KeyModifiers::NONE, KeyCode::Down | KeyCode::Char('j')) => KeyAction::NavigateDown,
            (KeyModifiers::NONE, KeyCode::Left | KeyCode::Char('h')) => KeyAction::NavigateLeft,
            (KeyModifiers::NONE, KeyCode::Right | KeyCode::Char('l')) => KeyAction::NavigateRight,
            (KeyModifiers::NONE, KeyCode::Enter) => KeyAction::Select,
            (KeyModifiers::NONE, KeyCode::Backspace) => KeyAction::Backspace,
            (KeyModifiers::NONE, KeyCode::Char(c)) => KeyAction::InputChar(c),
            _ => KeyAction::None,
        }
    }
}
