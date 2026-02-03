use std::path::PathBuf;

use agent_client_protocol::SessionInfo;

use crate::session::SessionState;
use crate::types::{ChatMessage, ChatRole};

use super::ChatSessionView;

impl ChatSessionView {
    pub(super) fn ensure_session_state(&mut self, session_id: &str) -> &mut SessionState {
        self.sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionState::new("Ready"))
    }

    pub(super) fn session_cwd(&self, session_id: &str) -> Option<PathBuf> {
        self.session_list
            .iter()
            .find(|session| session.session_id.to_string() == session_id)
            .map(|session| session.cwd.clone())
    }

    pub(super) fn active_session_info(&self) -> Option<&SessionInfo> {
        let active_id = self.active_session_id.as_ref()?;
        self.session_list
            .iter()
            .find(|session| session.session_id.to_string() == *active_id)
    }

    pub(super) fn update_session_list(&mut self, sessions: Vec<SessionInfo>) {
        let active_id = self.active_session_id.clone();
        self.session_list = sessions;

        if let Some(active_id) = active_id {
            let has_active = self
                .session_list
                .iter()
                .any(|session| session.session_id.to_string() == active_id);
            if !has_active {
                self.session_list.insert(
                    0,
                    SessionInfo::new(active_id.clone(), self.cwd.clone()).title("Current session"),
                );
            }
        }

        if self.active_session_id.is_none() {
            if let Some(first) = self.session_list.first() {
                self.active_session_id = Some(first.session_id.to_string());
                self.ensure_session_state(&first.session_id.to_string());
            }
        }
    }

    pub(super) fn next_message_id(&self, session_id: &str) -> usize {
        self.sessions
            .get(session_id)
            .map(|session| session.items.len())
            .unwrap_or(0)
    }

    pub(super) fn build_message(id: usize, role: ChatRole, content: String) -> ChatMessage {
        let (author, badge) = match role {
            ChatRole::User => ("You", None),
            ChatRole::Assistant => ("codex", Some("ACP")),
        };

        ChatMessage {
            id,
            role,
            author: author.to_string(),
            badge: badge.map(|value| value.to_string()),
            content,
            thinking: None,
            tool_calls: Vec::new(),
            attachments: Vec::new(),
        }
    }
}
