//! Common status symbols used throughout the application

/// Success indicator (✓)
pub const SUCCESS: &str = "✓";

/// Error indicator (✗)
pub const ERROR: &str = "✗";

/// Progress/loading indicator (⟳)
pub const PROGRESS: &str = "⟳";

/// Information indicator (ℹ)
pub const INFO: &str = "ℹ";

/// Helper function to format success messages
pub fn success(msg: &str) -> String {
    format!("{} {}", SUCCESS, msg)
}

/// Helper function to format error messages
pub fn error(msg: &str) -> String {
    let trimmed = msg.trim();
    let normalized = if trimmed.is_empty() {
        "Unknown error"
    } else {
        trimmed
    };
    let with_prefix = if normalized.to_lowercase().starts_with("error:") {
        normalized.to_string()
    } else {
        format!("Error: {}", normalized)
    };

    format!("{} {}", ERROR, with_prefix)
}

/// Helper function to format progress messages
pub fn progress(msg: &str) -> String {
    format!("{} {}...", PROGRESS, msg)
}

/// Helper function to format info messages
pub fn info(msg: &str) -> String {
    format!("{} {}", INFO, msg)
}
