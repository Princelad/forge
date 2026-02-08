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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_adds_prefix_and_trims() {
        let msg = error("  failed ");
        assert_eq!(msg, format!("{} Error: failed", ERROR));
    }

    #[test]
    fn error_preserves_existing_prefix() {
        let msg = error("Error: boom");
        assert_eq!(msg, format!("{} Error: boom", ERROR));
    }

    #[test]
    fn error_accepts_lowercase_prefix() {
        let msg = error("error: boom");
        assert_eq!(msg, format!("{} error: boom", ERROR));
    }

    #[test]
    fn error_falls_back_on_empty() {
        let msg = error("   ");
        assert_eq!(msg, format!("{} Error: Unknown error", ERROR));
    }
}
