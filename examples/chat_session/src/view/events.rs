use std::rc::Rc;

use agent_client_protocol::{SessionInfo, SessionUpdate, ToolCall, ToolCallUpdate};
#[cfg(feature = "unstable_session_info_update")]
use agent_client_protocol::{MaybeUndefined, SessionInfoUpdate};
use gpui::{Context, Size, px};
use gpui_component::text::MarkdownState;

use crate::bridge::UiEvent;
use crate::types::{ChatItem, ChatMessage, ChatRole};
use crate::utils::{content_block_to_text, map_tool_call};

use super::ChatSessionView;

impl ChatSessionView {
    pub(super) fn handle_codex_event(&mut self, event: UiEvent, cx: &mut Context<Self>) {
        match event {
            UiEvent::SessionUpdate { session_id, update } => {
                self.apply_session_update(&session_id, update, cx);
            }
            UiEvent::PromptFinished {
                session_id,
                stop_reason,
            } => {
                let session = self.ensure_session_state(&session_id);
                session.is_generating = false;
                session.status_line = format!("Completed: {stop_reason:?}");
                cx.notify();
            }
            UiEvent::SessionsListed(sessions) => {
                self.update_session_list(sessions);
                cx.notify();
            }
            UiEvent::SessionCreated { session_id, cwd } => {
                let session = self.ensure_session_state(&session_id);
                session.status_line = "Ready".to_string();
                self.active_session_id = Some(session_id.clone());
                self.session_list
                    .retain(|session| session.session_id.to_string() != session_id);
                self.session_list.insert(
                    0,
                    SessionInfo::new(session_id.clone(), cwd).title("New session"),
                );
                self.scroll_handle.scroll_to_bottom();
                cx.notify();
            }
            UiEvent::SessionLoaded { session_id } => {
                let session = self.ensure_session_state(&session_id);
                session.status_line = "Session loaded".to_string();
                self.active_session_id = Some(session_id);
                self.scroll_handle.scroll_to_item(0, gpui::ScrollStrategy::Top);
                cx.notify();
            }
            UiEvent::SystemMessage(message) => {
                self.app_status = message;
                cx.notify();
            }
        }
    }

    pub(super) fn apply_session_update(
        &mut self,
        session_id: &str,
        update: SessionUpdate,
        cx: &mut Context<Self>,
    ) {
        match update {
            SessionUpdate::UserMessageChunk(chunk) => {
                let text = content_block_to_text(&chunk.content);
                if !text.is_empty() {
                    self.append_message_chunk(session_id, ChatRole::User, &text, cx);
                }
            }
            SessionUpdate::AgentMessageChunk(chunk) => {
                let text = content_block_to_text(&chunk.content);
                if !text.is_empty() {
                    self.append_message_chunk(session_id, ChatRole::Assistant, &text, cx);
                }
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                let text = content_block_to_text(&chunk.content);
                if !text.is_empty() {
                    self.append_thought_chunk(session_id, &text, cx);
                }
            }
            SessionUpdate::ToolCall(tool_call) => {
                self.upsert_tool_call(session_id, tool_call, cx);
            }
            SessionUpdate::ToolCallUpdate(update) => {
                self.apply_tool_call_update(session_id, update, cx);
            }
            #[cfg(feature = "unstable_session_info_update")]
            SessionUpdate::SessionInfoUpdate(update) => {
                self.apply_session_info_update(session_id, update);
                cx.notify();
            }
            _ => {}
        }
    }

    #[cfg(feature = "unstable_session_info_update")]
    pub(super) fn apply_session_info_update(&mut self, session_id: &str, update: SessionInfoUpdate) {
        let Some(session) = self
            .session_list
            .iter_mut()
            .find(|session| session.session_id.to_string() == session_id)
        else {
            return;
        };

        match update.title {
            MaybeUndefined::Undefined => {}
            MaybeUndefined::Null => session.title = None,
            MaybeUndefined::Value(value) => session.title = Some(value),
        }

        match update.updated_at {
            MaybeUndefined::Undefined => {}
            MaybeUndefined::Null => session.updated_at = None,
            MaybeUndefined::Value(value) => session.updated_at = Some(value),
        }
    }

    pub(super) fn append_message_chunk(
        &mut self,
        session_id: &str,
        role: ChatRole,
        text: &str,
        cx: &mut Context<Self>,
    ) {
        let (streaming_role, target_index) = {
            let session = self.ensure_session_state(session_id);
            let target_index = match role {
                ChatRole::User => session.active_user_index,
                ChatRole::Assistant => session.active_assistant_index,
            };
            (session.streaming_role, target_index)
        };

        let index = if let (Some(role_match), Some(idx)) = (streaming_role, target_index) {
            if role_match == role {
                idx
            } else {
                self.create_new_streaming_message(session_id, role, cx)
            }
        } else {
            self.create_new_streaming_message(session_id, role, cx)
        };

        self.append_to_message(session_id, index, text, cx);
        if role == ChatRole::Assistant {
            self.scroll_handle.scroll_to_bottom();
        }
    }

    pub(super) fn create_new_streaming_message(
        &mut self,
        session_id: &str,
        role: ChatRole,
        cx: &mut Context<Self>,
    ) -> usize {
        let next_id = self.next_message_id(session_id);
        let message = Self::build_message(next_id, role, String::new());
        let new_index = self.push_message(session_id, message, cx);
        let session = self.ensure_session_state(session_id);
        session.streaming_role = Some(role);
        match role {
            ChatRole::User => session.active_user_index = Some(new_index),
            ChatRole::Assistant => {
                session.active_assistant_index = Some(new_index);
                session.tool_call_index.clear();
                session.tool_call_cache.clear();
            }
        }
        new_index
    }

    pub(super) fn append_thought_chunk(&mut self, session_id: &str, text: &str, cx: &mut Context<Self>) {
        let index = self.ensure_assistant_message(session_id, cx);
        let session = self.ensure_session_state(session_id);
        let ChatItem::Message(message) = &mut session.items[index];
        let thinking = message.thinking.get_or_insert_with(String::new);
        thinking.push_str(text);
        if !session.pending_remeasure.contains(&index) {
            session.pending_remeasure.push(index);
        }
        cx.notify();
    }

    pub(super) fn ensure_assistant_message(&mut self, session_id: &str, cx: &mut Context<Self>) -> usize {
        if let Some(index) = self
            .sessions
            .get(session_id)
            .and_then(|session| session.active_assistant_index)
        {
            return index;
        }

        let next_id = self.next_message_id(session_id);
        let message = Self::build_message(next_id, ChatRole::Assistant, String::new());
        let index = self.push_message(session_id, message, cx);
        let session = self.ensure_session_state(session_id);
        session.active_assistant_index = Some(index);
        session.streaming_role = Some(ChatRole::Assistant);
        session.tool_call_index.clear();
        session.tool_call_cache.clear();
        index
    }

    pub(super) fn upsert_tool_call(&mut self, session_id: &str, tool_call: ToolCall, cx: &mut Context<Self>) {
        let id = tool_call.tool_call_id.0.to_string();
        let session = self.ensure_session_state(session_id);
        session
            .tool_call_cache
            .insert(id.clone(), tool_call.clone());
        self.update_tool_call_ui(session_id, &id, tool_call, cx);
    }

    pub(super) fn apply_tool_call_update(
        &mut self,
        session_id: &str,
        update: ToolCallUpdate,
        cx: &mut Context<Self>,
    ) {
        let id = update.tool_call_id.0.to_string();
        let session = self.ensure_session_state(session_id);
        let updated = if let Some(mut existing) = session.tool_call_cache.remove(&id) {
            existing.update(update.fields.clone());
            existing
        } else {
            ToolCall::try_from(update.clone()).unwrap_or_else(|_| {
                let mut fallback = ToolCall::new(update.tool_call_id.clone(), "Tool Call");
                fallback.update(update.fields.clone());
                fallback
            })
        };
        session.tool_call_cache.insert(id.clone(), updated.clone());
        self.update_tool_call_ui(session_id, &id, updated, cx);
    }

    pub(super) fn update_tool_call_ui(
        &mut self,
        session_id: &str,
        id: &str,
        tool_call: ToolCall,
        cx: &mut Context<Self>,
    ) {
        let index = self.ensure_assistant_message(session_id, cx);
        let session = self.ensure_session_state(session_id);
        let ChatItem::Message(message) = &mut session.items[index];
        let ui_tool_call = map_tool_call(id, &tool_call);

        if let Some(existing_index) = session.tool_call_index.get(id).copied() {
            if let Some(existing) = message.tool_calls.get_mut(existing_index) {
                *existing = ui_tool_call;
            }
        } else {
            message.tool_calls.push(ui_tool_call);
            session
                .tool_call_index
                .insert(id.to_string(), message.tool_calls.len() - 1);
        }

        if !session.pending_remeasure.contains(&index) {
            session.pending_remeasure.push(index);
        }
        cx.notify();
    }

    pub(super) fn push_message(
        &mut self,
        session_id: &str,
        message: ChatMessage,
        cx: &mut Context<Self>,
    ) -> usize {
        let session = self.ensure_session_state(session_id);
        let index = session.items.len();
        let item = ChatItem::Message(message);

        let ChatItem::Message(message) = &item;
        if message.role == ChatRole::Assistant {
            let state = MarkdownState::new(&message.content, cx);
            let message_id = index;
            let session_id = session_id.to_string();
            cx.observe(state.entity(), move |this: &mut Self, _, cx| {
                if let Some(session) = this.sessions.get_mut(&session_id) {
                    if !session.pending_remeasure.contains(&message_id) {
                        session.pending_remeasure.push(message_id);
                    }
                }
                cx.notify();
            })
            .detach();
            session
                .markdown_states
                .insert(message_id.to_string(), state);
        }

        session.items.push(item);
        let mut sizes = (*session.item_sizes).clone();
        sizes.push(Size {
            width: px(0.),
            height: session.items[index].estimated_height(),
        });
        session.item_sizes = Rc::new(sizes);
        session.measured = false;
        index
    }

    pub(super) fn append_to_message(
        &mut self,
        session_id: &str,
        index: usize,
        text: &str,
        cx: &mut Context<Self>,
    ) {
        let session = self.ensure_session_state(session_id);
        let ChatItem::Message(message) = &mut session.items[index];
        message.content.push_str(text);

        if message.role == ChatRole::Assistant {
            if let Some(state) = session.markdown_states.get(&index.to_string()) {
                state.update(cx, |state, cx| {
                    state.push_str(text, cx);
                });
            }
        }

        if !session.pending_remeasure.contains(&index) {
            session.pending_remeasure.push(index);
        }
        cx.notify();
    }
}
