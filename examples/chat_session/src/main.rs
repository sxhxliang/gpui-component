//! Chat session example with Codex-style UI.
//!
//! Highlights:
//! - Virtual list with measured dynamic heights
//! - Markdown rendering with cached MarkdownState
//! - Simulated streaming updates
//! - Compact tool call display

#![allow(unexpected_cfgs)]

mod bridge;
mod session;
mod types;
mod ui;
mod utils;

#[cfg(feature = "unstable_session_info_update")]
use agent_client_protocol::{MaybeUndefined, SessionInfoUpdate};
use agent_client_protocol::{SessionInfo, SessionUpdate, ToolCall, ToolCallUpdate};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable, StyledExt, VirtualListScrollHandle,
    button::Button,
    button::ButtonVariants,
    h_flex,
    input::{InputGroup, InputGroupAddon, InputGroupTextarea, InputState},
    label::Label,
    scroll::{ScrollableElement, ScrollbarAxis},
    text::MarkdownState,
    v_flex, v_virtual_list,
};
use gpui_component_assets::Assets;
use std::{collections::HashMap, path::PathBuf, rc::Rc};

use bridge::{CodexCommand, UiEvent, spawn_codex_bridge};
use session::SessionState;
use types::{ChatItem, ChatMessage, ChatRole};
use ui::{
    build_chat_item_element, format_elapsed_time, project_group_name, session_title,
    short_session_id,
};
use utils::{content_block_to_text, map_tool_call};

pub struct ChatSessionView {
    sessions: HashMap<String, SessionState>,
    session_list: Vec<SessionInfo>,
    active_session_id: Option<String>,
    scroll_handle: VirtualListScrollHandle,
    input_state: Entity<InputState>,
    app_status: String,
    codex_commands: tokio::sync::mpsc::UnboundedSender<CodexCommand>,
    cwd: PathBuf,
    _codex_task: Task<()>,
}

impl ChatSessionView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Ask Codex anything...")
                .multi_line(true)
        });

        let scroll_handle = VirtualListScrollHandle::new();
        let codex_bridge = spawn_codex_bridge();
        let codex_commands = codex_bridge.commands.clone();
        let updates_rx = codex_bridge.updates;

        let _codex_task = cx.spawn(async move |this, cx| {
            while let Ok(event) = updates_rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    this.handle_codex_event(event, cx);
                });
            }
        });

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            sessions: HashMap::new(),
            session_list: Vec::new(),
            active_session_id: None,
            scroll_handle,
            input_state,
            app_status: "Connecting to Codex ACP...".to_string(),
            codex_commands,
            cwd,
            _codex_task,
        }
    }

    fn ensure_session_state(&mut self, session_id: &str) -> &mut SessionState {
        self.sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionState::new("Ready"))
    }

    fn session_cwd(&self, session_id: &str) -> Option<PathBuf> {
        self.session_list
            .iter()
            .find(|session| session.session_id.to_string() == session_id)
            .map(|session| session.cwd.clone())
    }

    fn active_session_info(&self) -> Option<&SessionInfo> {
        let active_id = self.active_session_id.as_ref()?;
        self.session_list
            .iter()
            .find(|session| session.session_id.to_string() == *active_id)
    }

    fn update_session_list(&mut self, sessions: Vec<SessionInfo>) {
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

    fn measure_all_items(
        session: &mut SessionState,
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme = cx.theme().clone();

        let sizes: Vec<Size<Pixels>> = session
            .items
            .iter()
            .enumerate()
            .map(|(ix, item)| {
                let markdown_state = session.markdown_states.get(&ix.to_string());
                item.measure(width, &theme, markdown_state, window, cx)
            })
            .collect();

        session.item_sizes = Rc::new(sizes);
        session.measured = true;
        session.last_width = Some(width);
    }

    fn remeasure_item(
        session: &mut SessionState,
        ix: usize,
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme = cx.theme().clone();
        let markdown_state = session.markdown_states.get(&ix.to_string());
        let new_size = session.items[ix].measure(width, &theme, markdown_state, window, cx);

        let mut sizes = (*session.item_sizes).clone();
        sizes[ix] = new_size;
        session.item_sizes = Rc::new(sizes);
    }

    fn handle_codex_event(&mut self, event: UiEvent, cx: &mut Context<Self>) {
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
                self.scroll_handle.scroll_to_item(0, ScrollStrategy::Top);
                cx.notify();
            }
            UiEvent::SystemMessage(message) => {
                self.app_status = message;
                cx.notify();
            }
        }
    }

    fn apply_session_update(
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
    fn apply_session_info_update(&mut self, session_id: &str, update: SessionInfoUpdate) {
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

    fn send_message(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

        let next_id = self
            .sessions
            .get(&session_id)
            .map(|session| session.items.len())
            .unwrap_or(0);
        let user_message = ChatMessage {
            id: next_id,
            role: ChatRole::User,
            author: "You".to_string(),
            badge: None,
            content: content.to_string(),
            thinking: None,
            tool_calls: Vec::new(),
            attachments: Vec::new(),
        };
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

    fn request_sessions(&mut self) {
        let _ = self.codex_commands.send(CodexCommand::ListSessions);
    }

    fn create_new_session(&mut self) {
        let _ = self.codex_commands.send(CodexCommand::NewSession {
            cwd: self.cwd.clone(),
        });
    }

    fn select_session(&mut self, session_id: String, cx: &mut Context<Self>) {
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

    fn append_message_chunk(
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

    fn create_new_streaming_message(
        &mut self,
        session_id: &str,
        role: ChatRole,
        cx: &mut Context<Self>,
    ) -> usize {
        let next_id = self
            .sessions
            .get(session_id)
            .map(|session| session.items.len())
            .unwrap_or(0);
        let message = ChatMessage {
            id: next_id,
            role,
            author: if role == ChatRole::User {
                "You".to_string()
            } else {
                "codex".to_string()
            },
            badge: if role == ChatRole::Assistant {
                Some("ACP".to_string())
            } else {
                None
            },
            content: String::new(),
            thinking: None,
            tool_calls: Vec::new(),
            attachments: Vec::new(),
        };
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

    fn append_thought_chunk(&mut self, session_id: &str, text: &str, cx: &mut Context<Self>) {
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

    fn ensure_assistant_message(&mut self, session_id: &str, cx: &mut Context<Self>) -> usize {
        if let Some(index) = self
            .sessions
            .get(session_id)
            .and_then(|session| session.active_assistant_index)
        {
            return index;
        }

        let next_id = self
            .sessions
            .get(session_id)
            .map(|session| session.items.len())
            .unwrap_or(0);
        let message = ChatMessage {
            id: next_id,
            role: ChatRole::Assistant,
            author: "codex".to_string(),
            badge: Some("ACP".to_string()),
            content: String::new(),
            thinking: None,
            tool_calls: Vec::new(),
            attachments: Vec::new(),
        };
        let index = self.push_message(session_id, message, cx);
        let session = self.ensure_session_state(session_id);
        session.active_assistant_index = Some(index);
        session.streaming_role = Some(ChatRole::Assistant);
        session.tool_call_index.clear();
        session.tool_call_cache.clear();
        index
    }

    fn upsert_tool_call(&mut self, session_id: &str, tool_call: ToolCall, cx: &mut Context<Self>) {
        let id = tool_call.tool_call_id.0.to_string();
        let session = self.ensure_session_state(session_id);
        session
            .tool_call_cache
            .insert(id.clone(), tool_call.clone());
        self.update_tool_call_ui(session_id, &id, tool_call, cx);
    }

    fn apply_tool_call_update(
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

    fn update_tool_call_ui(
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

    fn push_message(
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

    fn append_to_message(
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

impl ChatItem {
    fn measure(
        &self,
        available_width: Pixels,
        theme: &gpui_component::Theme,
        markdown_state: Option<&MarkdownState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Size<Pixels> {
        let element = match self {
            ChatItem::Message(message) => build_chat_item_element(message, theme, markdown_state),
        };

        let mut any_element = element.into_any_element();
        let available_space = size(
            AvailableSpace::Definite(available_width),
            AvailableSpace::MinContent,
        );
        any_element.layout_as_root(available_space, window, cx)
    }
}

impl Render for ChatSessionView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let measure_width = px(640.);
        let mut item_sizes = Rc::new(Vec::new());
        let mut status_line = self.app_status.clone();
        let active_session_id = self.active_session_id.clone();

        if let Some(session_id) = active_session_id.as_deref() {
            let session = self.ensure_session_state(session_id);
            if !session.measured || session.last_width != Some(measure_width) {
                Self::measure_all_items(session, measure_width, window, cx);
            }

            if !session.pending_remeasure.is_empty() {
                let indices: Vec<usize> = session.pending_remeasure.drain(..).collect();
                if let Some(width) = session.last_width {
                    for ix in indices {
                        Self::remeasure_item(session, ix, width, window, cx);
                    }
                }
            }

            item_sizes = session.item_sizes.clone();
            status_line = session.status_line.clone();
        }

        // Group sessions by project (cwd)
        let mut grouped_sessions: HashMap<String, Vec<&SessionInfo>> = HashMap::new();
        for session in &self.session_list {
            let group = project_group_name(&session.cwd);
            grouped_sessions.entry(group).or_default().push(session);
        }

        // === LEFT SIDEBAR ===
        let sidebar = {
            v_flex()
                .w(px(260.))
                .min_w(px(220.))
                .max_w(px(300.))
                .h_full()
                .bg(theme.sidebar)
                .border_r_1()
                .border_color(theme.border)
                .child(
                    // Sidebar header
                    h_flex()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_3()
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            Icon::new(IconName::SquareTerminal)
                                .small()
                                .text_color(theme.foreground),
                        )
                        .child(
                            Button::new("new-thread")
                                .small()
                                .ghost()
                                .label("New thread")
                                .icon(IconName::Plus)
                                .on_click(cx.listener(|this, _, _, _| {
                                    this.create_new_session();
                                })),
                        )
                        .child(div().flex_1())
                        .child(
                            Button::new("refresh")
                                .xsmall()
                                .ghost()
                                .icon(IconName::Redo)
                                .on_click(cx.listener(|this, _, _, _| {
                                    this.request_sessions();
                                })),
                        ),
                )
                .child(
                    // Menu items
                    v_flex()
                        .px_2()
                        .py_2()
                        .gap_0p5()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .py_1p5()
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.secondary))
                                .child(
                                    Icon::new(IconName::Loader)
                                        .xsmall()
                                        .text_color(theme.muted_foreground),
                                )
                                .child(Label::new("Automations").text_sm()),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .py_1p5()
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.secondary))
                                .child(
                                    Icon::new(IconName::BookOpen)
                                        .xsmall()
                                        .text_color(theme.muted_foreground),
                                )
                                .child(Label::new("Skills").text_sm()),
                        ),
                )
                .child(
                    // Threads section header
                    h_flex().items_center().px_3().py_2().child(
                        Label::new("Threads")
                            .text_xs()
                            .font_semibold()
                            .text_color(theme.muted_foreground),
                    ),
                )
                .child(
                    // Thread list grouped by project
                    v_flex()
                        .flex_1()
                        .min_h_0()
                        .px_2()
                        .overflow_y_scrollbar()
                        .children(grouped_sessions.into_iter().map(|(group_name, sessions)| {
                            let group_theme = theme.clone();
                            v_flex()
                                .gap_0p5()
                                .pb_2()
                                .child(
                                    // Project group header
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .px_2()
                                        .py_1()
                                        .child(
                                            Icon::new(IconName::FolderOpen)
                                                .xsmall()
                                                .text_color(group_theme.muted_foreground),
                                        )
                                        .child(
                                            Label::new(group_name)
                                                .text_xs()
                                                .font_medium()
                                                .text_color(group_theme.muted_foreground),
                                        ),
                                )
                                .children(sessions.into_iter().map(|session| {
                                    let session_id = session.session_id.to_string();
                                    let is_active = self.active_session_id.as_deref()
                                        == Some(session_id.as_str());
                                    let title = session_title(session);
                                    let elapsed =
                                        format_elapsed_time(session.updated_at.as_deref());
                                    let session_id_for_click = session_id.clone();
                                    let item_theme = group_theme.clone();

                                    let bg = if is_active {
                                        item_theme.accent.opacity(0.15)
                                    } else {
                                        gpui::transparent_black()
                                    };

                                    div()
                                        .id(ElementId::Name(
                                            format!("session-{}", session_id).into(),
                                        ))
                                        .w_full()
                                        .cursor_pointer()
                                        .px_2()
                                        .py_1p5()
                                        .pl_6()
                                        .rounded_md()
                                        .bg(bg)
                                        .hover(|style| style.bg(item_theme.secondary))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.select_session(session_id_for_click.clone(), cx);
                                        }))
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    Label::new(title)
                                                        .text_sm()
                                                        .text_color(if is_active {
                                                            item_theme.foreground
                                                        } else {
                                                            item_theme.muted_foreground
                                                        })
                                                        .truncate(),
                                                )
                                                .child(div().flex_1())
                                                .when(!elapsed.is_empty(), |this| {
                                                    this.child(
                                                        Label::new(elapsed).text_xs().text_color(
                                                            item_theme.muted_foreground,
                                                        ),
                                                    )
                                                }),
                                        )
                                }))
                        })),
                )
        };

        // === CHAT AREA ===
        let chat_header_title = self
            .active_session_info()
            .map(session_title)
            .unwrap_or_else(|| "Chat".to_string());
        let chat_header_project = self
            .active_session_info()
            .map(|s| project_group_name(&s.cwd));

        let chat_header = h_flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(Label::new(chat_header_title).font_semibold())
                    .when_some(chat_header_project, |this, project| {
                        this.child(
                            Icon::new(IconName::FolderOpen)
                                .xsmall()
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            Label::new(project)
                                .text_sm()
                                .text_color(theme.muted_foreground),
                        )
                    }),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("open")
                            .small()
                            .ghost()
                            .label("Open")
                            .icon(IconName::ExternalLink),
                    )
                    .child(
                        Button::new("commit")
                            .small()
                            .ghost()
                            .label("Commit")
                            .icon(IconName::CircleCheck),
                    ),
            );

        let chat_list = if let Some(session_id) = active_session_id.clone() {
            let list_id = format!("chat-items-{}", session_id);
            let session_id_for_list = session_id.clone();

            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_hidden()
                .child(
                    v_flex()
                        .id("chat-list-container")
                        .size_full()
                        .relative()
                        .child(
                            v_virtual_list(
                                cx.entity().clone(),
                                list_id,
                                item_sizes,
                                move |view, visible_range, _window, cx| {
                                    let theme = cx.theme().clone();
                                    let mut elements = Vec::with_capacity(visible_range.len());
                                    let Some(session) = view.sessions.get(&session_id_for_list)
                                    else {
                                        return elements;
                                    };

                                    for ix in visible_range {
                                        let ChatItem::Message(message) = &session.items[ix];
                                        let markdown_state =
                                            session.markdown_states.get(&ix.to_string());

                                        let element = build_chat_item_element(
                                            message,
                                            &theme,
                                            markdown_state,
                                        );

                                        elements.push(
                                            div()
                                                .id(ElementId::Name(
                                                    format!("chat-item-{}", ix).into(),
                                                ))
                                                .w_full()
                                                .child(element),
                                        );
                                    }

                                    elements
                                },
                            )
                            .track_scroll(&self.scroll_handle)
                            .p_4()
                            .gap_2(),
                        )
                        .scrollbar(&self.scroll_handle, ScrollbarAxis::Vertical),
                )
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .child(
                    v_flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .child(
                            Icon::new(IconName::SquareTerminal)
                                .size(px(48.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            Label::new("Select a thread or start a new one")
                                .text_color(theme.muted_foreground),
                        ),
                )
                .into_any_element()
        };

        // Input area
        let input_area = v_flex()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(theme.border)
            .child(
                InputGroup::new()
                    .flex_col()
                    .h_auto()
                    .w_full()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .bg(theme.background)
                    .child(
                        InputGroupTextarea::new(&self.input_state)
                            .min_h(px(80.))
                            .flex_1(),
                    )
                    .child(
                        InputGroupAddon::new()
                            .block_end()
                            .child(
                                Button::new("attach")
                                    .xsmall()
                                    .ghost()
                                    .icon(IconName::Plus)
                                    .rounded_full(),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.secondary))
                                    .child(
                                        Label::new("GPT-5.2-Codex")
                                            .text_xs()
                                            .text_color(theme.muted_foreground),
                                    )
                                    .child(
                                        Icon::new(IconName::ChevronDown)
                                            .xsmall()
                                            .text_color(theme.muted_foreground),
                                    ),
                            )
                            .child(div().flex_1())
                            .child(
                                Label::new(status_line)
                                    .text_xs()
                                    .text_color(theme.muted_foreground),
                            )
                            .child(
                                Button::new("send")
                                    .xsmall()
                                    .primary()
                                    .icon(IconName::ArrowUp)
                                    .rounded_full()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.send_message(window, cx);
                                    })),
                            ),
                    ),
            );

        let chat_area = v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(theme.background)
            .child(chat_header)
            .child(chat_list)
            .child(input_area);

        // === MAIN LAYOUT ===
        h_flex().size_full().child(sidebar).child(chat_area)
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1200.), px(800.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| ChatSessionView::new(window, cx));
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
