use gpui::{Context, Window};

use crate::bridge::CodexCommand;
use crate::types::ChatRole;

use super::ChatSessionView;

impl ChatSessionView {
    pub(super) fn send_message(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let session_id = match self.active_session_id.clone() {
            Some(session_id) => session_id,
            None => return,
        };
        if let Some(session) = self.sessions.get(&session_id) {
            if session.is_generating {
                return;
            }
        }

        let text = self.input_state.read(cx).value();
        let content = text.trim();
        if content.is_empty() {
            return;
        }

        let next_id = self.next_message_id(&session_id);
        let user_message = Self::build_message(next_id, ChatRole::User, content.to_string());
        self.push_message(&session_id, user_message, cx);

        self.input_state
            .update(cx, |input, cx| input.set_value("", window, cx));

        {
            let session = self.ensure_session_state(&session_id);
            session.status_line = "Sending to Codex...".to_string();
            session.is_generating = true;
            session.clear_stream_state();
        }

        let send_result = self.codex_commands.send(CodexCommand::Prompt {
            session_id: session_id.clone(),
            text: content.to_string(),
        });
        if send_result.is_err() {
            let session = self.ensure_session_state(&session_id);
            session.status_line = "Failed to send prompt to Codex".to_string();
            session.is_generating = false;
        }

        self.scroll_handle.scroll_to_bottom();
        cx.notify();
    }

    pub(super) fn request_sessions(&mut self) {
        let _ = self.codex_commands.send(CodexCommand::ListSessions);
    }

    pub(super) fn create_new_session(&mut self) {
        let _ = self.codex_commands.send(CodexCommand::NewSession {
            cwd: self.cwd.clone(),
        });
    }

    pub(super) fn select_session(&mut self, session_id: String, cx: &mut Context<Self>) {
        if self.active_session_id.as_ref() == Some(&session_id) {
            return;
        }

        {
            let session = self.ensure_session_state(&session_id);
            session.status_line = "Loading session...".to_string();
            session.is_generating = false;
            session.clear_stream_state();
        }
        self.active_session_id = Some(session_id.clone());

        if let Some(cwd) = self.session_cwd(&session_id) {
            let _ = self
                .codex_commands
                .send(CodexCommand::LoadSession { session_id, cwd });
        } else {
            let session = self.ensure_session_state(&session_id);
            session.status_line = "Missing session cwd".to_string();
        }
        cx.notify();
    }
}
