//! Message UI builder functions for Codex-style layout.

use gpui::{AnyElement, IntoElement, ParentElement, Styled};
use gpui_component::{
    Icon, IconName, Sizable, StyledExt, h_flex, label::Label, text::MarkdownState,
    text::MarkdownView, v_flex,
};

use crate::types::{ChatMessage, ChatRole, ChatToolCall, ToolStatus};
use crate::ui::components::build_attachments;

/// Builds a compact tool call line (icon + bold name + detail + status).
pub fn build_tool_call_line(tool: &ChatToolCall, theme: &gpui_component::Theme) -> AnyElement {
    let (icon, action_text) = tool_call_icon_and_action(&tool.name, &tool.args);

    let status_element: AnyElement = match tool.status {
        ToolStatus::Running => Icon::new(IconName::LoaderCircle)
            .xsmall()
            .text_color(theme.info)
            .into_any_element(),
        ToolStatus::Success => Icon::new(IconName::Check)
            .xsmall()
            .text_color(theme.success)
            .into_any_element(),
        ToolStatus::Failed => Icon::new(IconName::Close)
            .xsmall()
            .text_color(theme.danger)
            .into_any_element(),
    };

    h_flex()
        .items_center()
        .gap_2()
        .py_1()
        .child(Icon::new(icon).xsmall().text_color(theme.muted_foreground))
        .child(
            Label::new(action_text)
                .text_sm()
                .font_medium()
                .text_color(theme.foreground),
        )
        .child(status_element)
        .into_any_element()
}

/// Returns the appropriate icon and action text for a tool call.
fn tool_call_icon_and_action(name: &str, args: &str) -> (IconName, String) {
    let name_lower = name.to_lowercase();

    // Extract file path from args if available
    let file_path = extract_file_path(args);

    if name_lower.contains("thought") || name_lower.contains("think") {
        (IconName::LoaderCircle, "Thought".to_string())
    } else if name_lower.contains("read") {
        let label = file_path
            .map(|p| format!("Read {}", truncate_path(&p, 40)))
            .unwrap_or_else(|| "Read".to_string());
        (IconName::Eye, label)
    } else if name_lower.contains("edit") || name_lower.contains("write") {
        let label = file_path
            .map(|p| format!("Edited {}", truncate_path(&p, 40)))
            .unwrap_or_else(|| "Edited".to_string());
        (IconName::File, label)
    } else if name_lower.contains("glob")
        || name_lower.contains("grep")
        || name_lower.contains("search")
        || name_lower.contains("explore")
    {
        (IconName::Search, "Explored".to_string())
    } else if name_lower.contains("bash")
        || name_lower.contains("command")
        || name_lower.contains("terminal")
    {
        let label = extract_command(args)
            .map(|c| format!("Ran {}", truncate_path(&c, 40)))
            .unwrap_or_else(|| "Ran command".to_string());
        (IconName::SquareTerminal, label)
    } else if name_lower.contains("task") {
        (IconName::Loader, "Task".to_string())
    } else {
        (IconName::LayoutDashboard, name.to_string())
    }
}

/// Extracts file path from JSON args.
fn extract_file_path(args: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(args).ok()?;
    let obj = value.as_object()?;

    let path_keys = ["path", "file_path", "file", "filename", "target", "uri"];
    for key in path_keys {
        if let Some(path) = obj.get(key).and_then(|v| v.as_str()) {
            return Some(path.to_string());
        }
    }
    None
}

/// Extracts command from JSON args.
fn extract_command(args: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(args).ok()?;
    let obj = value.as_object()?;

    let cmd_keys = ["command", "cmd", "shell", "run"];
    for key in cmd_keys {
        if let Some(cmd) = obj.get(key).and_then(|v| v.as_str()) {
            return Some(cmd.to_string());
        }
    }
    None
}

/// Truncates a path for display.
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    let suffix = &path[path.len().saturating_sub(max_len - 3)..];
    format!("...{}", suffix)
}

/// Builds a user message (right-aligned, subtle bubble, no avatar).
pub fn build_user_message(message: &ChatMessage, theme: &gpui_component::Theme) -> AnyElement {
    use gpui::px;

    let content = Label::new(message.content.clone())
        .text_sm()
        .text_color(theme.foreground);

    let mut bubble = v_flex()
        .gap_2()
        .max_w(px(560.))
        .px_4()
        .py_3()
        .bg(theme.secondary)
        .rounded_lg()
        .child(content);

    if !message.attachments.is_empty() {
        bubble = bubble.child(build_attachments(&message.attachments, theme));
    }

    h_flex()
        .w_full()
        .justify_end()
        .child(bubble)
        .into_any_element()
}

/// Builds an assistant message (left-aligned, no bubble, plain markdown).
pub fn build_assistant_message(
    message: &ChatMessage,
    theme: &gpui_component::Theme,
    markdown_state: Option<&MarkdownState>,
) -> AnyElement {
    use gpui::px;

    let mut content_stack = v_flex().gap_2().max_w(px(640.));

    // Render thinking as a compact line if present
    if message.thinking.is_some() {
        content_stack = content_stack.child(
            h_flex()
                .items_center()
                .gap_2()
                .py_1()
                .child(
                    Icon::new(IconName::LoaderCircle)
                        .xsmall()
                        .text_color(theme.muted_foreground),
                )
                .child(
                    Label::new("Thought")
                        .text_sm()
                        .font_medium()
                        .text_color(theme.foreground),
                )
                .child(
                    Icon::new(IconName::Check)
                        .xsmall()
                        .text_color(theme.success),
                ),
        );
    }

    // Render tool calls as compact lines
    for tool in &message.tool_calls {
        content_stack = content_stack.child(build_tool_call_line(tool, theme));
    }

    // Render main content
    if !message.content.is_empty() {
        let text_content: AnyElement = if let Some(state) = markdown_state {
            MarkdownView::new(state)
                .text_sm()
                .text_color(theme.foreground)
                .into_any_element()
        } else {
            Label::new(message.content.clone())
                .text_sm()
                .text_color(theme.foreground)
                .into_any_element()
        };
        content_stack = content_stack.child(text_content);
    }

    if !message.attachments.is_empty() {
        content_stack = content_stack.child(build_attachments(&message.attachments, theme));
    }

    h_flex()
        .w_full()
        .justify_start()
        .child(content_stack)
        .into_any_element()
}

/// Builds the chat item element wrapper.
pub fn build_chat_item_element(
    message: &ChatMessage,
    theme: &gpui_component::Theme,
    markdown_state: Option<&MarkdownState>,
) -> AnyElement {
    use gpui::px;

    let content = if message.role == ChatRole::User {
        build_user_message(message, theme)
    } else {
        build_assistant_message(message, theme, markdown_state)
    };

    gpui::div()
        .w_full()
        .py_2()
        .px(px(8.))
        .child(content)
        .into_any_element()
}
