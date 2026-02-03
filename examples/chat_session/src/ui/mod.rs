//! UI module for chat session components.

mod builders;
mod components;

pub use builders::*;
pub use components::{build_attachments, build_diff_panel};

use agent_client_protocol::SessionInfo;

/// Returns a short session ID (first 8 characters).
pub fn short_session_id(session_id: &str) -> String {
    session_id.chars().take(8).collect()
}

/// Returns the title for a session.
pub fn session_title(session: &SessionInfo) -> String {
    session.title.clone().unwrap_or_else(|| {
        format!(
            "Session {}",
            short_session_id(&session.session_id.to_string())
        )
    })
}

/// Returns the subtitle for a session (cwd and updated_at).
pub fn session_subtitle(session: &SessionInfo) -> String {
    let cwd_label = session
        .cwd
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| session.cwd.display().to_string());

    if let Some(updated_at) = &session.updated_at {
        format!("{cwd_label} · {updated_at}")
    } else {
        cwd_label
    }
}

/// Formats elapsed time in human-readable format.
/// For now, just returns the raw updated_at string if available.
pub fn format_elapsed_time(updated_at: Option<&str>) -> String {
    updated_at
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Extracts the project/folder name from a path.
pub fn project_group_name(cwd: &std::path::Path) -> String {
    cwd.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| cwd.display().to_string())
}
