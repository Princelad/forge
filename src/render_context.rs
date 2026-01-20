use ratatui::{layout::Rect, Frame};

use crate::pages::branch_manager::{BranchInfo, BranchManagerMode};
use crate::pages::commit_history::CommitInfo;
use crate::{data::Store, AppMode, AppSettings, Focus, Theme};

/// Centralized context for rendering pages, reducing parameter proliferation
pub struct RenderContext<'a> {
    // Frame and layout - Note: Frame is borrowed as immutable for access, actual rendering happens via the frame ref
    pub frame: &'a mut Frame<'a>,
    pub area: Rect,

    // App state
    pub mode: AppMode,
    pub focus: Focus,
    pub show_help: bool,
    pub theme: &'a Theme,
    pub settings: &'a AppSettings,

    // Data store
    pub store: &'a Store,

    // UI state for various pages
    pub selected_project: usize,
    pub selected_change: usize,
    pub selected_branch: usize,
    pub selected_setting: usize,
    pub selected_module: usize,
    pub selected_developer: usize,
    pub selected_commit: usize,
    pub selected_board_column: usize,
    pub selected_board_item: usize,

    // Scroll positions
    pub project_scroll: usize,
    pub changes_scroll: usize,
    pub branch_scroll: usize,
    pub merge_scroll: usize,
    pub module_scroll: usize,
    pub commit_scroll: usize,

    // Text input buffers
    pub commit_msg: &'a str,
    pub branch_input_buffer: &'a str,
    pub module_input_buffer: &'a str,
    pub search_buffer: &'a str,

    // Search and filter state
    pub search_active: bool,
    pub filtered_projects: &'a [&'a crate::data::Project],
    pub total_projects: usize,

    // Merge state
    pub merge_file_index: usize,
    pub merge_focus: crate::pages::merge_visualizer::MergePaneFocus,
    pub accepted_merge: Option<crate::pages::merge_visualizer::MergePaneFocus>,

    // Branch manager state
    pub branch_manager_mode: BranchManagerMode,
    pub cached_branches: &'a [BranchInfo],

    // Module manager state
    pub branch_manager_mode_ref: BranchManagerMode,

    // Commit history state
    pub cached_commits: &'a [CommitInfo],

    // Menu state
    pub menu_selected_index: usize,
    pub status: &'a str,

    // Workspace
    pub workdir: Option<&'a std::path::Path>,
}

impl<'a> RenderContext<'a> {
    /// Create a new RenderContext with all required data
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frame: &'a mut Frame<'a>,
        area: Rect,
        mode: AppMode,
        focus: Focus,
        show_help: bool,
        theme: &'a Theme,
        settings: &'a AppSettings,
        store: &'a Store,
        status: &'a str,
    ) -> Self {
        Self {
            frame,
            area,
            mode,
            focus,
            show_help,
            theme,
            settings,
            store,
            status,
            selected_project: 0,
            selected_change: 0,
            selected_branch: 0,
            selected_setting: 0,
            selected_module: 0,
            selected_developer: 0,
            selected_commit: 0,
            selected_board_column: 0,
            selected_board_item: 0,
            project_scroll: 0,
            changes_scroll: 0,
            branch_scroll: 0,
            merge_scroll: 0,
            module_scroll: 0,
            commit_scroll: 0,
            commit_msg: "",
            branch_input_buffer: "",
            module_input_buffer: "",
            search_buffer: "",
            search_active: false,
            filtered_projects: &[],
            total_projects: 0,
            merge_file_index: 0,
            merge_focus: crate::pages::merge_visualizer::MergePaneFocus::Files,
            accepted_merge: None,
            branch_manager_mode: BranchManagerMode::List,
            cached_branches: &[],
            branch_manager_mode_ref: BranchManagerMode::List,
            cached_commits: &[],
            menu_selected_index: 0,
            workdir: None,
        }
    }

    /// Builder method to set selected_project
    pub fn with_selected_project(mut self, index: usize) -> Self {
        self.selected_project = index;
        self
    }

    /// Builder method to set selected_change
    pub fn with_selected_change(mut self, index: usize) -> Self {
        self.selected_change = index;
        self
    }

    /// Builder method to set selected_branch
    pub fn with_selected_branch(mut self, index: usize) -> Self {
        self.selected_branch = index;
        self
    }

    /// Builder method to set selected_module
    pub fn with_selected_module(mut self, index: usize) -> Self {
        self.selected_module = index;
        self
    }

    /// Builder method to set menu_selected_index
    pub fn with_menu_selected_index(mut self, index: usize) -> Self {
        self.menu_selected_index = index;
        self
    }

    /// Builder method to set commit_msg
    pub fn with_commit_msg(mut self, msg: &'a str) -> Self {
        self.commit_msg = msg;
        self
    }

    /// Builder method to set search state
    pub fn with_search(mut self, active: bool, buffer: &'a str) -> Self {
        self.search_active = active;
        self.search_buffer = buffer;
        self
    }

    /// Builder method to set filtered projects
    pub fn with_filtered_projects(
        mut self,
        projects: &'a [&'a crate::data::Project],
        total: usize,
    ) -> Self {
        self.filtered_projects = projects;
        self.total_projects = total;
        self
    }

    /// Builder method to set branch manager state
    pub fn with_branch_manager(
        mut self,
        mode: BranchManagerMode,
        selected: usize,
        buffer: &'a str,
        scroll: usize,
        branches: &'a [BranchInfo],
    ) -> Self {
        self.branch_manager_mode = mode;
        self.branch_manager_mode_ref = mode;
        self.selected_branch = selected;
        self.branch_input_buffer = buffer;
        self.branch_scroll = scroll;
        self.cached_branches = branches;
        self
    }

    /// Builder method to set module manager state
    pub fn with_module_manager(
        mut self,
        selected_module: usize,
        selected_developer: usize,
        buffer: &'a str,
        scroll: usize,
    ) -> Self {
        self.selected_module = selected_module;
        self.selected_developer = selected_developer;
        self.module_input_buffer = buffer;
        self.module_scroll = scroll;
        self
    }

    /// Builder method to set commit history state
    pub fn with_commit_history(
        mut self,
        selected: usize,
        scroll: usize,
        commits: &'a [CommitInfo],
    ) -> Self {
        self.selected_commit = selected;
        self.commit_scroll = scroll;
        self.cached_commits = commits;
        self
    }

    /// Builder method to set merge state
    pub fn with_merge(
        mut self,
        file_index: usize,
        focus: crate::pages::merge_visualizer::MergePaneFocus,
        scroll: usize,
        accepted: Option<crate::pages::merge_visualizer::MergePaneFocus>,
    ) -> Self {
        self.merge_file_index = file_index;
        self.merge_focus = focus;
        self.merge_scroll = scroll;
        self.accepted_merge = accepted;
        self
    }

    /// Builder method to set board state
    pub fn with_board(mut self, selected_column: usize, selected_item: usize) -> Self {
        self.selected_board_column = selected_column;
        self.selected_board_item = selected_item;
        self
    }

    /// Builder method to set scroll positions
    pub fn with_scrolls(
        mut self,
        project: usize,
        changes: usize,
        branch: usize,
        merge: usize,
    ) -> Self {
        self.project_scroll = project;
        self.changes_scroll = changes;
        self.branch_scroll = branch;
        self.merge_scroll = merge;
        self
    }

    /// Builder method to set workdir
    pub fn with_workdir(mut self, workdir: Option<&'a std::path::Path>) -> Self {
        self.workdir = workdir;
        self
    }

    /// Builder method to set selected_setting
    pub fn with_selected_setting(mut self, index: usize) -> Self {
        self.selected_setting = index;
        self
    }
}
