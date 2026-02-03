//! Conversion utilities for chat data.

use agent_client_protocol::{
    ContentBlock, EmbeddedResourceResource, ToolCall, ToolCallContent, ToolCallStatus,
};

use crate::types::{ChatToolCall, ToolStatus};

/// Truncates a label to a maximum number of characters.
pub fn truncate_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_string();
    }

    let mut out = label
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

/// Maps an ACP ToolCall to a ChatToolCall.
pub fn map_tool_call(id: &str, tool_call: &ToolCall) -> ChatToolCall {
    ChatToolCall {
        id: id.to_string(),
        name: tool_call.title.clone(),
        status: map_tool_status(tool_call.status),
        duration: "—".to_string(),
        args: tool_call
            .raw_input
            .as_ref()
            .map(format_json)
            .unwrap_or_else(|| tool_call_content_to_text(&tool_call.content)),
        output: tool_call
            .raw_output
            .as_ref()
            .map(format_json)
            .unwrap_or_default(),
    }
}

/// Maps an ACP ToolCallStatus to a ToolStatus.
pub fn map_tool_status(status: ToolCallStatus) -> ToolStatus {
    match status {
        ToolCallStatus::Pending | ToolCallStatus::InProgress => ToolStatus::Running,
        ToolCallStatus::Completed => ToolStatus::Success,
        ToolCallStatus::Failed => ToolStatus::Failed,
        _ => ToolStatus::Running,
    }
}

/// Formats a JSON value as a pretty-printed string.
pub fn format_json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// Converts tool call content to text.
pub fn tool_call_content_to_text(content: &[ToolCallContent]) -> String {
    let mut output = String::new();
    for item in content {
        let chunk = match item {
            ToolCallContent::Content(content) => content_block_to_text(&content.content),
            ToolCallContent::Diff(diff) => format!(
                "Diff: {} ({} chars)",
                diff.path.display(),
                diff.new_text.len()
            ),
            ToolCallContent::Terminal(terminal) => format!("Terminal: {}", terminal.terminal_id),
            _ => String::new(),
        };
        if !chunk.is_empty() {
            output.push_str(&chunk);
            if !output.ends_with('\n') {
                output.push('\n');
            }
        }
    }
    output.trim_end().to_string()
}

/// Converts a content block to text.
pub fn content_block_to_text(content: &ContentBlock) -> String {
    match content {
        ContentBlock::Text(text) => text.text.clone(),
        ContentBlock::Image(image) => format!("[image: {}]", image.mime_type),
        ContentBlock::Audio(audio) => format!("[audio: {}]", audio.mime_type),
        ContentBlock::ResourceLink(link) => format!("{} ({})", link.name, link.uri),
        ContentBlock::Resource(resource) => match &resource.resource {
            EmbeddedResourceResource::TextResourceContents(text) => text.text.clone(),
            EmbeddedResourceResource::BlobResourceContents(blob) => {
                format!("[resource: {}]", blob.uri)
            }
            _ => String::new(),
        },
        _ => String::new(),
    }
}
