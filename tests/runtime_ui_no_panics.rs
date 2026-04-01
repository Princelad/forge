use std::fs;
use std::path::{Path, PathBuf};

fn runtime_region(path: &Path) -> String {
    let content = fs::read_to_string(path).expect("failed to read source file");
    content
        .split("#[cfg(test)]")
        .next()
        .unwrap_or_default()
        .to_string()
}

fn assert_no_panic_calls(path: &Path) {
    let runtime = runtime_region(path);
    assert!(
        !runtime.contains("unwrap("),
        "runtime UI code contains unwrap() in {}",
        path.display()
    );
    assert!(
        !runtime.contains("expect("),
        "runtime UI code contains expect() in {}",
        path.display()
    );
}

#[test]
fn runtime_ui_paths_avoid_unwrap_and_expect() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paths = [
        "src/main.rs",
        "src/key_handler.rs",
        "src/ui_utils.rs",
        "src/pages/branch_manager.rs",
        "src/pages/changes.rs",
        "src/pages/commit_history.rs",
        "src/pages/dashboard.rs",
        "src/pages/help.rs",
        "src/pages/main_menu.rs",
        "src/pages/merge_visualizer.rs",
        "src/pages/module_manager.rs",
        "src/pages/project_board.rs",
        "src/pages/settings.rs",
        "src/pages/stashes.rs",
        "src/state/branch_manager.rs",
        "src/state/changes.rs",
        "src/state/commit_history.rs",
        "src/state/dashboard.rs",
        "src/state/merge.rs",
        "src/state/module_manager.rs",
        "src/state/stashes.rs",
    ];

    for rel_path in paths {
        assert_no_panic_calls(&root.join(rel_path));
    }
}
