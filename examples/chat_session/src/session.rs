//! Session state management.

use std::{collections::HashMap, rc::Rc};

use agent_client_protocol::ToolCall;
use gpui::{Pixels, Size};
use gpui_component::text::MarkdownState;

use crate::types::{ChatItem, ChatRole};

/// State for a single chat session.
pub struct SessionState {
    pub items: Vec<ChatItem>,
    pub item_sizes: Rc<Vec<Size<Pixels>>>,
    pub measured: bool,
    pub last_width: Option<Pixels>,
    pub pending_remeasure: Vec<usize>,
    pub markdown_states: HashMap<String, MarkdownState>,
    pub status_line: String,
    pub is_generating: bool,
    pub streaming_role: Option<ChatRole>,
    pub active_user_index: Option<usize>,
    pub active_assistant_index: Option<usize>,
    pub tool_call_index: HashMap<String, usize>,
    pub tool_call_cache: HashMap<String, ToolCall>,
}

impl SessionState {
    pub fn new(status_line: impl Into<String>) -> Self {
        Self {
            items: Vec::new(),
            item_sizes: Rc::new(Vec::new()),
            measured: false,
            last_width: None,
            pending_remeasure: Vec::new(),
            markdown_states: HashMap::new(),
            status_line: status_line.into(),
            is_generating: false,
            streaming_role: None,
            active_user_index: None,
            active_assistant_index: None,
            tool_call_index: HashMap::new(),
            tool_call_cache: HashMap::new(),
        }
    }

    pub fn clear_stream_state(&mut self) {
        self.streaming_role = None;
        self.active_user_index = None;
        self.active_assistant_index = None;
        self.tool_call_index.clear();
        self.tool_call_cache.clear();
    }
}
