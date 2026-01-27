//! Module manager page state.
//!
//! Manages module and developer lists, creation, editing, and assignment.

use crate::pages::module_manager::ModuleManagerMode;

/// State for the Module Manager view.
///
/// Handles module/developer list navigation, creation, editing, and assignment modes.
#[derive(Debug, Clone, Default)]
pub struct ModuleManagerState {
    /// Current mode (list, create, edit, assign).
    pub mode: ModuleManagerMode,
    /// Currently selected module index.
    pub selected_module: usize,
    /// Currently selected developer index.
    pub selected_developer: usize,
    /// Input buffer for module/developer name.
    pub input_buffer: String,
    /// Scroll offset for module list.
    pub module_scroll: usize,
    /// Scroll offset for developer list.
    pub developer_scroll: usize,
    /// ID of module being edited (if in edit mode).
    pub editing_module_id: Option<uuid::Uuid>,
    /// Whether assignment mode is active.
    pub assign_mode: bool,
    /// Pane ratio for module/developer split (percentage).
    pub pane_ratio: u16,
}

impl ModuleManagerState {
    /// Creates a new module manager state with default values.
    pub fn new() -> Self {
        Self {
            mode: ModuleManagerMode::ModuleList,
            selected_module: 0,
            selected_developer: 0,
            input_buffer: String::new(),
            module_scroll: 0,
            developer_scroll: 0,
            editing_module_id: None,
            assign_mode: false,
            pane_ratio: 50,
        }
    }

    /// Toggles between module list and developer list views.
    pub fn toggle_list(&mut self) {
        self.mode = if matches!(self.mode, ModuleManagerMode::ModuleList) {
            ModuleManagerMode::DeveloperList
        } else {
            ModuleManagerMode::ModuleList
        };
    }

    /// Returns `true` if currently viewing the developer list.
    pub fn is_developer_list(&self) -> bool {
        matches!(self.mode, ModuleManagerMode::DeveloperList)
    }

    /// Returns `true` if in any create mode.
    pub fn is_create_mode(&self) -> bool {
        matches!(
            self.mode,
            ModuleManagerMode::CreateModule | ModuleManagerMode::CreateDeveloper
        )
    }

    /// Returns `true` if in edit mode.
    pub fn is_edit_mode(&self) -> bool {
        matches!(self.mode, ModuleManagerMode::EditModule)
    }

    /// Enters module creation mode.
    pub fn enter_create_module(&mut self) {
        self.mode = ModuleManagerMode::CreateModule;
        self.input_buffer.clear();
    }

    /// Enters developer creation mode.
    pub fn enter_create_developer(&mut self) {
        self.mode = ModuleManagerMode::CreateDeveloper;
        self.input_buffer.clear();
    }

    /// Enters module edit mode with the given module's data.
    pub fn enter_edit_module(&mut self, module_id: uuid::Uuid, module_name: &str) {
        self.mode = ModuleManagerMode::EditModule;
        self.editing_module_id = Some(module_id);
        self.input_buffer = module_name.to_string();
    }

    /// Enters assignment mode.
    pub fn enter_assign_mode(&mut self) {
        self.assign_mode = true;
    }

    /// Exits current mode back to the appropriate list view.
    pub fn exit_current_mode(&mut self) {
        match self.mode {
            ModuleManagerMode::CreateModule | ModuleManagerMode::EditModule => {
                self.mode = ModuleManagerMode::ModuleList;
            }
            ModuleManagerMode::CreateDeveloper => {
                self.mode = ModuleManagerMode::DeveloperList;
            }
            _ => {}
        }
        self.input_buffer.clear();
        self.editing_module_id = None;
        self.assign_mode = false;
    }

    /// Navigates up in the current list.
    ///
    /// Returns `true` if the selection changed.
    pub fn navigate_up(&mut self) -> bool {
        if self.is_developer_list() {
            if self.selected_developer > 0 {
                self.selected_developer -= 1;
                self.ensure_developer_visible();
                return true;
            }
        } else if self.selected_module > 0 {
            self.selected_module -= 1;
            self.ensure_module_visible();
            return true;
        }
        false
    }

    /// Navigates down in the current list.
    ///
    /// Returns `true` if the selection changed.
    pub fn navigate_down(&mut self, max_modules: usize, max_developers: usize) -> bool {
        if self.is_developer_list() {
            let max_index = max_developers.saturating_sub(1);
            if self.selected_developer < max_index {
                self.selected_developer += 1;
                self.ensure_developer_visible();
                return true;
            }
        } else {
            let max_index = max_modules.saturating_sub(1);
            if self.selected_module < max_index {
                self.selected_module += 1;
                self.ensure_module_visible();
                return true;
            }
        }
        false
    }

    /// Appends a character to the input buffer.
    pub fn append_input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    /// Removes the last character from the input buffer.
    ///
    /// Returns `true` if a character was removed.
    pub fn pop_input_char(&mut self) -> bool {
        self.input_buffer.pop().is_some()
    }

    /// Clears the input buffer.
    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
    }

    /// Returns `true` if the input buffer is empty or whitespace-only.
    pub fn is_input_empty(&self) -> bool {
        self.input_buffer.trim().is_empty()
    }

    /// Gets the trimmed input value.
    pub fn get_input_value(&self) -> &str {
        self.input_buffer.trim()
    }

    /// Adjusts the pane ratio.
    ///
    /// # Returns
    /// The new ratio value.
    pub fn adjust_pane_ratio(&mut self, delta: i16) -> u16 {
        let new_ratio = (self.pane_ratio as i16 + delta).clamp(10, 90) as u16;
        self.pane_ratio = new_ratio;
        new_ratio
    }

    /// Ensures the current module selection is visible.
    fn ensure_module_visible(&mut self) {
        const WINDOW_SIZE: usize = 10;
        if self.selected_module < self.module_scroll {
            self.module_scroll = self.selected_module;
        } else if self.selected_module >= self.module_scroll + WINDOW_SIZE {
            self.module_scroll = self.selected_module.saturating_sub(WINDOW_SIZE - 1);
        }
    }

    /// Ensures the current developer selection is visible.
    fn ensure_developer_visible(&mut self) {
        const WINDOW_SIZE: usize = 10;
        if self.selected_developer < self.developer_scroll {
            self.developer_scroll = self.selected_developer;
        } else if self.selected_developer >= self.developer_scroll + WINDOW_SIZE {
            self.developer_scroll = self.selected_developer.saturating_sub(WINDOW_SIZE - 1);
        }
    }

    /// Resets selections to valid ranges.
    pub fn clamp_selections(&mut self, max_modules: usize, max_developers: usize) {
        self.selected_module = self.selected_module.min(max_modules.saturating_sub(1));
        self.selected_developer = self.selected_developer.min(max_developers.saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default_values() {
        let state = ModuleManagerState::new();
        assert!(matches!(state.mode, ModuleManagerMode::ModuleList));
        assert_eq!(state.selected_module, 0);
        assert_eq!(state.selected_developer, 0);
        assert!(state.input_buffer.is_empty());
        assert_eq!(state.pane_ratio, 50);
        assert!(!state.assign_mode);
    }

    #[test]
    fn test_toggle_list() {
        let mut state = ModuleManagerState::new();
        
        assert!(!state.is_developer_list());
        state.toggle_list();
        assert!(state.is_developer_list());
        state.toggle_list();
        assert!(!state.is_developer_list());
    }

    #[test]
    fn test_enter_create_module() {
        let mut state = ModuleManagerState::new();
        state.input_buffer = "existing".to_string();
        
        state.enter_create_module();
        
        assert!(matches!(state.mode, ModuleManagerMode::CreateModule));
        assert!(state.input_buffer.is_empty());
        assert!(state.is_create_mode());
    }

    #[test]
    fn test_enter_create_developer() {
        let mut state = ModuleManagerState::new();
        
        state.enter_create_developer();
        
        assert!(matches!(state.mode, ModuleManagerMode::CreateDeveloper));
        assert!(state.is_create_mode());
    }

    #[test]
    fn test_enter_edit_module() {
        let mut state = ModuleManagerState::new();
        let module_id = uuid::Uuid::new_v4();
        
        state.enter_edit_module(module_id, "Test Module");
        
        assert!(state.is_edit_mode());
        assert_eq!(state.editing_module_id, Some(module_id));
        assert_eq!(state.input_buffer, "Test Module");
    }

    #[test]
    fn test_exit_current_mode_from_create_module() {
        let mut state = ModuleManagerState::new();
        state.mode = ModuleManagerMode::CreateModule;
        state.input_buffer = "test".to_string();
        
        state.exit_current_mode();
        
        assert!(matches!(state.mode, ModuleManagerMode::ModuleList));
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn test_exit_current_mode_from_create_developer() {
        let mut state = ModuleManagerState::new();
        state.mode = ModuleManagerMode::CreateDeveloper;
        
        state.exit_current_mode();
        
        assert!(matches!(state.mode, ModuleManagerMode::DeveloperList));
    }

    #[test]
    fn test_navigate_up_module_list() {
        let mut state = ModuleManagerState {
            selected_module: 3,
            ..Default::default()
        };
        
        assert!(state.navigate_up());
        assert_eq!(state.selected_module, 2);
    }

    #[test]
    fn test_navigate_up_at_top() {
        let mut state = ModuleManagerState::new();
        
        assert!(!state.navigate_up());
        assert_eq!(state.selected_module, 0);
    }

    #[test]
    fn test_navigate_up_developer_list() {
        let mut state = ModuleManagerState {
            mode: ModuleManagerMode::DeveloperList,
            selected_developer: 3,
            ..Default::default()
        };
        
        assert!(state.navigate_up());
        assert_eq!(state.selected_developer, 2);
    }

    #[test]
    fn test_navigate_down_module_list() {
        let mut state = ModuleManagerState {
            selected_module: 3,
            ..Default::default()
        };
        
        assert!(state.navigate_down(10, 5));
        assert_eq!(state.selected_module, 4);
    }

    #[test]
    fn test_navigate_down_at_bottom() {
        let mut state = ModuleManagerState {
            selected_module: 9,
            ..Default::default()
        };
        
        assert!(!state.navigate_down(10, 5));
        assert_eq!(state.selected_module, 9);
    }

    #[test]
    fn test_input_operations() {
        let mut state = ModuleManagerState::new();
        
        assert!(state.is_input_empty());
        
        state.append_input_char('H');
        state.append_input_char('i');
        assert_eq!(state.input_buffer, "Hi");
        assert!(!state.is_input_empty());
        
        assert!(state.pop_input_char());
        assert_eq!(state.input_buffer, "H");
        
        state.clear_input();
        assert!(state.is_input_empty());
    }

    #[test]
    fn test_whitespace_is_empty() {
        let mut state = ModuleManagerState::new();
        state.input_buffer = "   ".to_string();
        
        assert!(state.is_input_empty());
        assert_eq!(state.get_input_value(), "");
    }

    #[test]
    fn test_adjust_pane_ratio() {
        let mut state = ModuleManagerState::new();
        
        let ratio = state.adjust_pane_ratio(10);
        assert_eq!(ratio, 60);
        
        let ratio = state.adjust_pane_ratio(-20);
        assert_eq!(ratio, 40);
    }

    #[test]
    fn test_clamp_selections() {
        let mut state = ModuleManagerState {
            selected_module: 15,
            selected_developer: 10,
            ..Default::default()
        };
        
        state.clamp_selections(5, 3);
        assert_eq!(state.selected_module, 4);
        assert_eq!(state.selected_developer, 2);
    }

    #[test]
    fn test_assign_mode() {
        let mut state = ModuleManagerState::new();
        
        assert!(!state.assign_mode);
        state.enter_assign_mode();
        assert!(state.assign_mode);
        state.exit_current_mode();
        assert!(!state.assign_mode);
    }
}
