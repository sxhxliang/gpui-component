//! Chat session example with collapsible thoughts, tool calls, files, and streaming markdown.
//!
//! Highlights:
//! - Virtual list with measured dynamic heights
//! - Markdown rendering with cached MarkdownState
//! - Simulated streaming updates
//! - Collapsible sections for thoughts and tool calls

use std::{collections::HashMap, rc::Rc, sync::Arc, thread};

use agent_client_protocol::{
    Agent, AgentSideConnection, AuthMethodId, AuthenticateRequest, Client, ClientCapabilities,
    ClientSideConnection, ContentBlock, EmbeddedResourceResource, Error, Implementation,
    InitializeRequest, NewSessionRequest, PermissionOptionKind, PromptRequest, ProtocolVersion,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionNotification, SessionUpdate, StopReason, ToolCall,
    ToolCallContent, ToolCallStatus, ToolCallUpdate,
};
use codex_acp::CodexAgent;
use codex_core::config::{Config, ConfigOverrides};
use gpui::*;
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable, StyledExt, VirtualListScrollHandle,
    button::Button,
    button::ButtonVariants,
    collapsible::Collapsible,
    h_flex,
    input::{InputGroup, InputGroupAddon, InputGroupTextarea, InputState},
    label::Label,
    scroll::{ScrollableElement, ScrollbarAxis},
    tag::Tag,
    text::{MarkdownState, MarkdownView},
    v_flex, v_virtual_list,
};
use gpui_component_assets::Assets;
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChatRole {
    User,
    Assistant,
}

#[derive(Clone, Copy)]
enum ToolStatus {
    Running,
    Success,
    Failed,
}

#[derive(Clone)]
struct ChatToolCall {
    id: String,
    name: String,
    status: ToolStatus,
    duration: String,
    args: String,
    output: String,
}

#[derive(Clone)]
struct FileAttachment {
    name: String,
    size: String,
    kind: String,
}

#[derive(Clone)]
struct ChatMessage {
    id: usize,
    role: ChatRole,
    author: String,
    badge: Option<String>,
    content: String,
    thinking: Option<String>,
    tool_calls: Vec<ChatToolCall>,
    attachments: Vec<FileAttachment>,
    thoughts_open: bool,
    tools_open: bool,
}

#[derive(Clone)]
enum ChatItem {
    Message(ChatMessage),
}

impl ChatItem {
    fn estimated_height(&self) -> Pixels {
        match self {
            ChatItem::Message(message) => {
                let mut height = px(120.);
                let line_count = message.content.lines().count().max(1) as f32;
                height += px(line_count * 20.);

                if !message.attachments.is_empty() {
                    height += px(70.);
                }

                if message.thinking.is_some() {
                    height += if message.thoughts_open {
                        px(120.)
                    } else {
                        px(36.)
                    };
                }

                if !message.tool_calls.is_empty() {
                    height += if message.tools_open {
                        px(180.)
                    } else {
                        px(36.)
                    };
                }

                height
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn measure(
        &self,
        available_width: Pixels,
        theme: &gpui_component::Theme,
        markdown_state: Option<&MarkdownState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Size<Pixels> {
        let element = match self {
            ChatItem::Message(message) => {
                build_message_element(message, theme, markdown_state, None, None)
            }
        };

        let mut any_element = element.into_any_element();
        let available_space = size(
            AvailableSpace::Definite(available_width),
            AvailableSpace::MinContent,
        );
        any_element.layout_as_root(available_space, window, cx)
    }
}

enum CodexCommand {
    Prompt(String),
}

enum UiEvent {
    SessionUpdate(SessionUpdate),
    PromptFinished(StopReason),
    SystemMessage(String),
}

struct UiClient {
    updates: smol::channel::Sender<UiEvent>,
}

#[async_trait::async_trait(?Send)]
impl Client for UiClient {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse, Error> {
        let preferred = args.options.iter().find(|option| {
            matches!(
                option.kind,
                PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
            )
        });
        let selected = preferred.or_else(|| args.options.first());
        let response = if let Some(option) = selected {
            RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
                SelectedPermissionOutcome::new(option.option_id.clone()),
            ))
        } else {
            RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled)
        };
        Ok(response)
    }

    async fn session_notification(&self, args: SessionNotification) -> Result<(), Error> {
        let _ = self.updates.try_send(UiEvent::SessionUpdate(args.update));
        Ok(())
    }
}

struct CodexBridge {
    commands: tokio::sync::mpsc::UnboundedSender<CodexCommand>,
    updates: smol::channel::Receiver<UiEvent>,
}

fn spawn_codex_bridge() -> CodexBridge {
    let (updates_tx, updates_rx) = smol::channel::unbounded::<UiEvent>();
    let (commands_tx, mut commands_rx) = tokio::sync::mpsc::unbounded_channel::<CodexCommand>();

    thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(err) => {
                let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                    "Failed to start runtime: {err}"
                )));
                return;
            }
        };

        LocalSet::new().block_on(&runtime, async move {
            let config = match Config::load_with_cli_overrides_and_harness_overrides(
                vec![],
                ConfigOverrides::default(),
            )
            .await
            {
                Ok(config) => config,
                Err(err) => {
                    let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                        "Failed to load Codex config: {err}"
                    )));
                    return;
                }
            };

            let (agent_read, client_write) = tokio::io::duplex(64 * 1024);
            let (client_read, agent_write) = tokio::io::duplex(64 * 1024);

            let agent = Rc::new(CodexAgent::new(config));
            let (acp_client, agent_io_task) = AgentSideConnection::new(
                agent.clone(),
                agent_write.compat_write(),
                agent_read.compat(),
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );

            if codex_acp::ACP_CLIENT.set(Arc::new(acp_client)).is_err() {
                let _ = updates_tx.try_send(UiEvent::SystemMessage(
                    "Codex ACP client already initialized".to_string(),
                ));
            }

            let ui_client = Rc::new(UiClient {
                updates: updates_tx.clone(),
            });
            let (client_conn, client_io_task) = ClientSideConnection::new(
                ui_client,
                client_write.compat_write(),
                client_read.compat(),
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );

            let updates_tx_agent = updates_tx.clone();
            tokio::task::spawn_local(async move {
                if let Err(err) = agent_io_task.await {
                    let _ = updates_tx_agent
                        .try_send(UiEvent::SystemMessage(format!("Agent I/O error: {err:?}")));
                }
            });

            let updates_tx_client = updates_tx.clone();
            tokio::task::spawn_local(async move {
                if let Err(err) = client_io_task.await {
                    let _ = updates_tx_client
                        .try_send(UiEvent::SystemMessage(format!("Client I/O error: {err:?}")));
                }
            });

            let init_request = InitializeRequest::new(ProtocolVersion::V1)
                .client_capabilities(ClientCapabilities::new())
                .client_info(Implementation::new("gpui-chat", "0.1.0"));

            if let Err(err) = client_conn.initialize(init_request).await {
                let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                    "Initialize failed: {err:?}"
                )));
                return;
            }

            let auth_method = if std::env::var("CODEX_API_KEY").is_ok() {
                AuthMethodId::new("codex-api-key")
            } else if std::env::var("OPENAI_API_KEY").is_ok() {
                AuthMethodId::new("openai-api-key")
            } else {
                AuthMethodId::new("chatgpt")
            };

            if let Err(err) = client_conn
                .authenticate(AuthenticateRequest::new(auth_method))
                .await
            {
                let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                    "Authentication failed: {err:?}"
                )));
            }

            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let session_response = match client_conn.new_session(NewSessionRequest::new(cwd)).await
            {
                Ok(response) => response,
                Err(err) => {
                    let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                        "Failed to create session: {err:?}"
                    )));
                    return;
                }
            };

            let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                "Connected to Codex ACP (session {})",
                session_response.session_id
            )));

            while let Some(command) = commands_rx.recv().await {
                match command {
                    CodexCommand::Prompt(prompt) => {
                        let request = PromptRequest::new(
                            session_response.session_id.clone(),
                            vec![ContentBlock::from(prompt)],
                        );
                        match client_conn.prompt(request).await {
                            Ok(response) => {
                                let _ = updates_tx
                                    .try_send(UiEvent::PromptFinished(response.stop_reason));
                            }
                            Err(err) => {
                                let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                                    "Prompt failed: {err:?}"
                                )));
                                let _ = updates_tx
                                    .try_send(UiEvent::PromptFinished(StopReason::Cancelled));
                            }
                        }
                    }
                }
            }
        });
    });

    CodexBridge {
        commands: commands_tx,
        updates: updates_rx,
    }
}

pub struct ChatSessionView {
    items: Vec<ChatItem>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
    measured: bool,
    last_width: Option<Pixels>,
    pending_remeasure: Vec<usize>,
    markdown_states: HashMap<String, MarkdownState>,
    input_state: Entity<InputState>,
    status_line: String,
    codex_commands: tokio::sync::mpsc::UnboundedSender<CodexCommand>,
    is_generating: bool,
    streaming_role: Option<ChatRole>,
    active_user_index: Option<usize>,
    active_assistant_index: Option<usize>,
    tool_call_index: HashMap<String, usize>,
    tool_call_cache: HashMap<String, ToolCall>,
    _codex_task: Task<()>,
}

impl ChatSessionView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = Vec::new();
        let item_sizes = Vec::new();

        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Ask Codex...")
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

        Self {
            items,
            item_sizes: Rc::new(item_sizes),
            scroll_handle,
            measured: false,
            last_width: None,
            pending_remeasure: Vec::new(),
            markdown_states: HashMap::new(),
            input_state,
            status_line: "Connecting to Codex ACP...".to_string(),
            codex_commands,
            is_generating: false,
            streaming_role: None,
            active_user_index: None,
            active_assistant_index: None,
            tool_call_index: HashMap::new(),
            tool_call_cache: HashMap::new(),
            _codex_task,
        }
    }

    fn measure_all_items(&mut self, width: Pixels, window: &mut Window, cx: &mut Context<Self>) {
        let theme = cx.theme().clone();

        let sizes: Vec<Size<Pixels>> = self
            .items
            .iter()
            .enumerate()
            .map(|(ix, item)| {
                let markdown_state = self.markdown_states.get(&ix.to_string());
                item.measure(width, &theme, markdown_state, window, cx)
            })
            .collect();

        self.item_sizes = Rc::new(sizes);
        self.measured = true;
        self.last_width = Some(width);
    }

    fn remeasure_item(
        &mut self,
        ix: usize,
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme = cx.theme().clone();
        let markdown_state = self.markdown_states.get(&ix.to_string());
        let new_size = self.items[ix].measure(width, &theme, markdown_state, window, cx);

        let mut sizes = (*self.item_sizes).clone();
        sizes[ix] = new_size;
        self.item_sizes = Rc::new(sizes);
    }

    fn toggle_thoughts(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        let ChatItem::Message(message) = &mut self.items[index];
        if message.thinking.is_some() {
            message.thoughts_open = !message.thoughts_open;
            if !self.pending_remeasure.contains(&index) {
                self.pending_remeasure.push(index);
            }
            cx.notify();
        }
    }

    fn toggle_tools(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        let ChatItem::Message(message) = &mut self.items[index];
        if !message.tool_calls.is_empty() {
            message.tools_open = !message.tools_open;
            if !self.pending_remeasure.contains(&index) {
                self.pending_remeasure.push(index);
            }
            cx.notify();
        }
    }

    fn clear_stream_state(&mut self) {
        self.streaming_role = None;
        self.active_user_index = None;
        self.active_assistant_index = None;
        self.tool_call_index.clear();
        self.tool_call_cache.clear();
    }

    fn handle_codex_event(&mut self, event: UiEvent, cx: &mut Context<Self>) {
        match event {
            UiEvent::SessionUpdate(update) => self.apply_session_update(update, cx),
            UiEvent::PromptFinished(stop_reason) => {
                self.is_generating = false;
                self.status_line = format!("Completed: {stop_reason:?}");
                cx.notify();
            }
            UiEvent::SystemMessage(message) => {
                self.status_line = message;
                cx.notify();
            }
        }
    }

    fn apply_session_update(&mut self, update: SessionUpdate, cx: &mut Context<Self>) {
        match update {
            SessionUpdate::UserMessageChunk(chunk) => {
                let text = content_block_to_text(&chunk.content);
                if !text.is_empty() {
                    self.append_message_chunk(ChatRole::User, &text, cx);
                }
            }
            SessionUpdate::AgentMessageChunk(chunk) => {
                let text = content_block_to_text(&chunk.content);
                if !text.is_empty() {
                    self.append_message_chunk(ChatRole::Assistant, &text, cx);
                }
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                let text = content_block_to_text(&chunk.content);
                if !text.is_empty() {
                    self.append_thought_chunk(&text, cx);
                }
            }
            SessionUpdate::ToolCall(tool_call) => {
                self.upsert_tool_call(tool_call, cx);
            }
            SessionUpdate::ToolCallUpdate(update) => {
                self.apply_tool_call_update(update, cx);
            }
            _ => {}
        }
    }

    fn send_message(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_generating {
            return;
        }

        let text = self.input_state.read(cx).value();
        let content = text.trim();
        if content.is_empty() {
            return;
        }

        let user_message = ChatMessage {
            id: self.items.len(),
            role: ChatRole::User,
            author: "You".to_string(),
            badge: None,
            content: content.to_string(),
            thinking: None,
            tool_calls: Vec::new(),
            attachments: Vec::new(),
            thoughts_open: false,
            tools_open: false,
        };
        self.push_message(user_message, cx);

        self.input_state
            .update(cx, |input, cx| input.set_value("", window, cx));

        self.status_line = "Sending to Codex...".to_string();
        self.is_generating = true;
        self.clear_stream_state();

        if self
            .codex_commands
            .send(CodexCommand::Prompt(content.to_string()))
            .is_err()
        {
            self.status_line = "Failed to send prompt to Codex".to_string();
            self.is_generating = false;
        }

        self.scroll_handle.scroll_to_bottom();
        cx.notify();
    }

    fn append_message_chunk(&mut self, role: ChatRole, text: &str, cx: &mut Context<Self>) {
        let target_index = match role {
            ChatRole::User => self.active_user_index,
            ChatRole::Assistant => self.active_assistant_index,
        };

        let index = if self.streaming_role == Some(role) && target_index.is_some() {
            target_index.unwrap()
        } else {
            let message = ChatMessage {
                id: self.items.len(),
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
                thoughts_open: false,
                tools_open: false,
            };
            let new_index = self.push_message(message, cx);
            self.streaming_role = Some(role);
            match role {
                ChatRole::User => self.active_user_index = Some(new_index),
                ChatRole::Assistant => {
                    self.active_assistant_index = Some(new_index);
                    self.tool_call_index.clear();
                    self.tool_call_cache.clear();
                }
            }
            new_index
        };

        self.append_to_message(index, text, cx);
        if role == ChatRole::Assistant {
            self.scroll_handle.scroll_to_bottom();
        }
    }

    fn append_thought_chunk(&mut self, text: &str, cx: &mut Context<Self>) {
        let index = self.ensure_assistant_message(cx);
        let ChatItem::Message(message) = &mut self.items[index];
        let thinking = message.thinking.get_or_insert_with(String::new);
        thinking.push_str(text);
        if !self.pending_remeasure.contains(&index) {
            self.pending_remeasure.push(index);
        }
        cx.notify();
    }

    fn ensure_assistant_message(&mut self, cx: &mut Context<Self>) -> usize {
        if let Some(index) = self.active_assistant_index {
            return index;
        }

        let message = ChatMessage {
            id: self.items.len(),
            role: ChatRole::Assistant,
            author: "codex".to_string(),
            badge: Some("ACP".to_string()),
            content: String::new(),
            thinking: None,
            tool_calls: Vec::new(),
            attachments: Vec::new(),
            thoughts_open: false,
            tools_open: false,
        };
        let index = self.push_message(message, cx);
        self.active_assistant_index = Some(index);
        self.streaming_role = Some(ChatRole::Assistant);
        self.tool_call_index.clear();
        self.tool_call_cache.clear();
        index
    }

    fn upsert_tool_call(&mut self, tool_call: ToolCall, cx: &mut Context<Self>) {
        let id = tool_call.tool_call_id.0.to_string();
        self.tool_call_cache.insert(id.clone(), tool_call.clone());
        self.update_tool_call_ui(&id, tool_call, cx);
    }

    fn apply_tool_call_update(&mut self, update: ToolCallUpdate, cx: &mut Context<Self>) {
        let id = update.tool_call_id.0.to_string();
        let updated = if let Some(mut existing) = self.tool_call_cache.remove(&id) {
            existing.update(update.fields.clone());
            existing
        } else {
            ToolCall::try_from(update.clone()).unwrap_or_else(|_| {
                let mut fallback = ToolCall::new(update.tool_call_id.clone(), "Tool Call");
                fallback.update(update.fields.clone());
                fallback
            })
        };
        self.tool_call_cache.insert(id.clone(), updated.clone());
        self.update_tool_call_ui(&id, updated, cx);
    }

    fn update_tool_call_ui(&mut self, id: &str, tool_call: ToolCall, cx: &mut Context<Self>) {
        let index = self.ensure_assistant_message(cx);
        let ChatItem::Message(message) = &mut self.items[index];
        let ui_tool_call = map_tool_call(id, &tool_call);

        if let Some(existing_index) = self.tool_call_index.get(id).copied() {
            if let Some(existing) = message.tool_calls.get_mut(existing_index) {
                *existing = ui_tool_call;
            }
        } else {
            message.tool_calls.push(ui_tool_call);
            self.tool_call_index
                .insert(id.to_string(), message.tool_calls.len() - 1);
        }

        if !self.pending_remeasure.contains(&index) {
            self.pending_remeasure.push(index);
        }
        cx.notify();
    }

    fn push_message(&mut self, message: ChatMessage, cx: &mut Context<Self>) -> usize {
        let index = self.items.len();
        let item = ChatItem::Message(message);

        let ChatItem::Message(message) = &item;
        if message.role == ChatRole::Assistant {
            let state = MarkdownState::new(&message.content, cx);
            let message_id = index;
            cx.observe(state.entity(), move |this: &mut Self, _, cx| {
                if !this.pending_remeasure.contains(&message_id) {
                    this.pending_remeasure.push(message_id);
                }
                cx.notify();
            })
            .detach();
            self.markdown_states.insert(message_id.to_string(), state);
        }

        self.items.push(item);
        let mut sizes = (*self.item_sizes).clone();
        sizes.push(Size {
            width: px(0.),
            height: self.items[index].estimated_height(),
        });
        self.item_sizes = Rc::new(sizes);
        self.measured = false;
        index
    }

    fn append_to_message(&mut self, index: usize, text: &str, cx: &mut Context<Self>) {
        let ChatItem::Message(message) = &mut self.items[index];
        message.content.push_str(text);

        if message.role == ChatRole::Assistant {
            if let Some(state) = self.markdown_states.get(&index.to_string()) {
                state.update(cx, |state, cx| {
                    state.push_str(text, cx);
                });
            }
        }

        if !self.pending_remeasure.contains(&index) {
            self.pending_remeasure.push(index);
        }
        cx.notify();
    }
}

impl Render for ChatSessionView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let measure_width = px(860.);
        if !self.measured || self.last_width != Some(measure_width) {
            self.measure_all_items(measure_width, window, cx);
        }

        if !self.pending_remeasure.is_empty() {
            let indices: Vec<usize> = self.pending_remeasure.drain(..).collect();
            if let Some(width) = self.last_width {
                for ix in indices {
                    self.remeasure_item(ix, width, window, cx);
                }
            }
        }

        let item_sizes = self.item_sizes.clone();

        v_flex()
            .size_full()
            .gap_3()
            .p_4()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Label::new("Chat Session").text_xl().font_semibold())
                            .child(
                                Label::new(self.status_line.clone())
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Tag::secondary().small().child("Codex ACP"))
                            .child(
                                Button::new("scroll-bottom")
                                    .small()
                                    .ghost()
                                    .icon(IconName::ChevronDown)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.scroll_handle.scroll_to_bottom();
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_lg()
                    .bg(cx.theme().background)
                    .overflow_hidden()
                    .child(
                        v_flex()
                            .id("chat-list-container")
                            .size_full()
                            .relative()
                            .child(
                                v_virtual_list(
                                    cx.entity().clone(),
                                    "chat-items",
                                    item_sizes,
                                    move |view, visible_range, _window, cx| {
                                        let theme = cx.theme().clone();
                                        let mut elements = Vec::with_capacity(visible_range.len());

                                        for ix in visible_range {
                                            let ChatItem::Message(message) = &view.items[ix];
                                            let markdown_state =
                                                view.markdown_states.get(&ix.to_string());

                                            let thoughts_header =
                                                message.thinking.as_ref().map(|_| {
                                                    let header = build_collapsible_header(
                                                        "思考",
                                                        message.thoughts_open,
                                                        theme.muted_foreground,
                                                    );
                                                    div()
                                                        .cursor_pointer()
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            cx.listener(
                                                                move |this, _, window, cx| {
                                                                    this.toggle_thoughts(
                                                                        ix, window, cx,
                                                                    );
                                                                },
                                                            ),
                                                        )
                                                        .child(header)
                                                        .into_any_element()
                                                });

                                            let tools_header = if message.tool_calls.is_empty() {
                                                None
                                            } else {
                                                let title = format!(
                                                    "工具调用 ({})",
                                                    message.tool_calls.len()
                                                );
                                                let header = build_collapsible_header(
                                                    &title,
                                                    message.tools_open,
                                                    theme.muted_foreground,
                                                );
                                                Some(
                                                    div()
                                                        .cursor_pointer()
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            cx.listener(
                                                                move |this, _, window, cx| {
                                                                    this.toggle_tools(
                                                                        ix, window, cx,
                                                                    );
                                                                },
                                                            ),
                                                        )
                                                        .child(header)
                                                        .into_any_element(),
                                                )
                                            };

                                            let element = build_message_element(
                                                message,
                                                &theme,
                                                markdown_state,
                                                thoughts_header,
                                                tools_header,
                                            );

                                            elements.push(
                                                div()
                                                    .id(ElementId::Name(
                                                        format!("chat-item-{}", ix).into(),
                                                    ))
                                                    .child(element),
                                            );
                                        }

                                        elements
                                    },
                                )
                                .track_scroll(&self.scroll_handle)
                                .p_4()
                                .gap_4(),
                            )
                            .scrollbar(&self.scroll_handle, ScrollbarAxis::Vertical),
                    ),
            )
            .child(
                InputGroup::new()
                    .flex_col()
                    .h_auto()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_lg()
                    .bg(cx.theme().background)
                    .child(
                        InputGroupTextarea::new(&self.input_state)
                            .min_h(px(96.))
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
                                Button::new("tools")
                                    .xsmall()
                                    .ghost()
                                    .icon(IconName::SquareTerminal),
                            )
                            .child(div().flex_1())
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
            )
    }
}

fn build_collapsible_header(title: &str, open: bool, muted_foreground: Hsla) -> AnyElement {
    let icon = if open {
        IconName::ChevronDown
    } else {
        IconName::ChevronRight
    };

    h_flex()
        .items_center()
        .gap_2()
        .child(Icon::new(icon).text_color(muted_foreground).small())
        .child(
            Label::new(title.to_string())
                .text_sm()
                .font_medium()
                .text_color(muted_foreground),
        )
        .into_any_element()
}

fn build_message_element(
    message: &ChatMessage,
    theme: &gpui_component::Theme,
    markdown_state: Option<&MarkdownState>,
    thoughts_header: Option<AnyElement>,
    tools_header: Option<AnyElement>,
) -> AnyElement {
    let is_user = message.role == ChatRole::User;
    let bubble_bg = if is_user {
        theme.primary
    } else {
        theme.background
    };
    let bubble_border = if is_user { theme.primary } else { theme.border };
    let bubble_text = if is_user {
        theme.primary_foreground
    } else {
        theme.foreground
    };

    let header = if is_user {
        h_flex()
            .w_full()
            .justify_end()
            .items_center()
            .gap_2()
            .child(Tag::primary().small().child(message.author.clone()))
            .into_any_element()
    } else {
        let mut header_row = h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .child(Icon::new(IconName::Bot).small())
            .child(Label::new(message.author.clone()).font_semibold());
        if let Some(badge) = &message.badge {
            header_row = header_row.child(Tag::secondary().small().child(badge.clone()));
        }
        header_row.into_any_element()
    };

    let content = if is_user {
        Label::new(message.content.clone())
            .text_sm()
            .text_color(bubble_text)
            .into_any_element()
    } else if let Some(state) = markdown_state {
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

    let mut bubble = v_flex()
        .gap_3()
        .max_w(px(720.))
        .p_4()
        .bg(bubble_bg)
        .border_1()
        .border_color(bubble_border)
        .rounded_lg()
        .child(header)
        .child(content);

    if !message.attachments.is_empty() {
        bubble = bubble.child(build_attachments(&message.attachments, theme));
    }

    if let Some(thinking) = &message.thinking {
        let header = thoughts_header.unwrap_or_else(|| {
            build_collapsible_header("思考", message.thoughts_open, theme.muted_foreground)
        });
        let thinking_content = div().pl_6().pt_2().child(
            Label::new(thinking.clone())
                .text_sm()
                .text_color(theme.muted_foreground),
        );
        let collapsible = Collapsible::new()
            .gap_2()
            .open(message.thoughts_open)
            .child(header)
            .content(thinking_content);

        bubble = bubble.child(
            div()
                .bg(theme.secondary)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .p_3()
                .child(collapsible),
        );
    }

    if !message.tool_calls.is_empty() {
        let title = format!("工具调用 ({})", message.tool_calls.len());
        let header = tools_header.unwrap_or_else(|| {
            build_collapsible_header(&title, message.tools_open, theme.muted_foreground)
        });
        let tools_content = v_flex().gap_2().pt_2().children(
            message
                .tool_calls
                .iter()
                .map(|tool| build_tool_call_card(tool, theme)),
        );
        let collapsible = Collapsible::new()
            .gap_2()
            .open(message.tools_open)
            .child(header)
            .content(tools_content);

        bubble = bubble.child(
            div()
                .bg(theme.secondary)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .p_3()
                .child(collapsible),
        );
    }

    let avatar = if is_user {
        Icon::new(IconName::User).small().into_any_element()
    } else {
        Icon::new(IconName::Bot).small().into_any_element()
    };

    let avatar_container = div()
        .size_9()
        .rounded_full()
        .bg(theme.secondary)
        .flex()
        .items_center()
        .justify_center()
        .child(avatar);

    if is_user {
        h_flex()
            .w_full()
            .justify_end()
            .items_start()
            .gap_3()
            .child(bubble)
            .child(avatar_container)
            .into_any_element()
    } else {
        h_flex()
            .w_full()
            .justify_start()
            .items_start()
            .gap_3()
            .child(avatar_container)
            .child(bubble)
            .into_any_element()
    }
}

fn build_attachments(attachments: &[FileAttachment], theme: &gpui_component::Theme) -> AnyElement {
    let cards = attachments.iter().map(|file| {
        h_flex()
            .items_center()
            .gap_3()
            .p_3()
            .bg(theme.secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(Icon::new(IconName::File).small())
            .child(
                v_flex()
                    .gap_0p5()
                    .child(Label::new(file.name.clone()).text_sm().font_medium())
                    .child(
                        Label::new(format!("{} • {}", file.kind, file.size))
                            .text_xs()
                            .text_color(theme.muted_foreground),
                    ),
            )
            .into_any_element()
    });

    v_flex().gap_2().children(cards).into_any_element()
}

fn build_tool_call_card(tool: &ChatToolCall, theme: &gpui_component::Theme) -> AnyElement {
    let status_tag = match tool.status {
        ToolStatus::Running => Tag::info().small().child("Running"),
        ToolStatus::Success => Tag::success().small().child("Success"),
        ToolStatus::Failed => Tag::danger().small().child("Failed"),
    };

    v_flex()
        .gap_2()
        .p_3()
        .bg(theme.background)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .child(Icon::new(IconName::SquareTerminal).small())
                .child(Label::new(tool.name.clone()).font_semibold())
                .child(status_tag)
                .child(div().flex_1())
                .child(
                    Label::new(tool.duration.clone())
                        .text_xs()
                        .text_color(theme.muted_foreground),
                ),
        )
        .child(
            v_flex()
                .gap_1()
                .child(
                    Label::new("Arguments")
                        .text_xs()
                        .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .p_2()
                        .bg(theme.secondary)
                        .rounded_sm()
                        .font_family("monospace")
                        .text_xs()
                        .child(tool.args.clone()),
                )
                .child(
                    Label::new("Result")
                        .text_xs()
                        .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .p_2()
                        .bg(theme.secondary)
                        .rounded_sm()
                        .text_xs()
                        .child(tool.output.clone()),
                ),
        )
        .into_any_element()
}

fn map_tool_call(id: &str, tool_call: &ToolCall) -> ChatToolCall {
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
            .unwrap_or_else(|| String::new()),
    }
}

fn map_tool_status(status: ToolCallStatus) -> ToolStatus {
    match status {
        ToolCallStatus::Pending | ToolCallStatus::InProgress => ToolStatus::Running,
        ToolCallStatus::Completed => ToolStatus::Success,
        ToolCallStatus::Failed => ToolStatus::Failed,
        _ => ToolStatus::Running,
    }
}

fn format_json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn tool_call_content_to_text(content: &[ToolCallContent]) -> String {
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

fn content_block_to_text(content: &ContentBlock) -> String {
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

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(980.), px(780.)), cx)),
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
