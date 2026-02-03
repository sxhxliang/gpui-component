//! Type definitions for chat session.

use gpui::Pixels;

/// Role of a chat participant.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

/// Status of a tool call.
#[derive(Clone, Copy)]
pub enum ToolStatus {
    Running,
    Success,
    Failed,
}

/// Represents a tool call within a chat message.
#[derive(Clone)]
pub struct ChatToolCall {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub status: ToolStatus,
    pub duration: String,
    pub args: String,
    pub output: String,
}

/// Represents a file attachment in a chat message.
#[derive(Clone)]
pub struct FileAttachment {
    pub name: String,
    pub size: String,
    pub kind: String,
}

/// Type of diff line.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
}

/// A single line in a diff.
#[derive(Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: String,
}

/// A file with diff information.
#[derive(Clone)]
pub struct DiffFile {
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
    pub lines: Vec<DiffLine>,
}

/// A chat message from a user or assistant.
#[derive(Clone)]
pub struct ChatMessage {
    #[allow(dead_code)]
    pub id: usize,
    pub role: ChatRole,
    pub author: String,
    pub badge: Option<String>,
    pub content: String,
    pub thinking: Option<String>,
    pub tool_calls: Vec<ChatToolCall>,
    pub attachments: Vec<FileAttachment>,
}

/// An item in the chat list.
#[derive(Clone)]
pub enum ChatItem {
    Message(ChatMessage),
}

impl ChatItem {
    /// Returns an estimated height for virtual list layout.
    pub fn estimated_height(&self) -> Pixels {
        use gpui::px;

        match self {
            ChatItem::Message(message) => {
                let mut height = px(60.);
                let line_count = message.content.lines().count().max(1) as f32;
                height += px(line_count * 20.);

                if !message.attachments.is_empty() {
                    height += px(70.);
                }

                // Tool calls are now compact single lines
                if !message.tool_calls.is_empty() {
                    height += px(message.tool_calls.len() as f32 * 28.);
                }

                // Thinking is now inline
                if message.thinking.is_some() {
                    height += px(28.);
                }

                height
            }
        }
    }
}
