//! Chat session example with collapsible thoughts, tool calls, files, and streaming markdown.
//!
//! Highlights:
//! - Virtual list with measured dynamic heights
//! - Markdown rendering with cached MarkdownState
//! - Simulated streaming updates
//! - Collapsible sections for thoughts and tool calls

use std::{collections::HashMap, path::PathBuf, rc::Rc, sync::Arc, thread};

use agent_client_protocol::{
    Agent, AgentSideConnection, AuthMethodId, AuthenticateRequest, Client, ClientCapabilities,
    ClientSideConnection, ContentBlock, EmbeddedResourceResource, Error, Implementation,
    InitializeRequest, ListSessionsRequest, LoadSessionRequest, MaybeUndefined, NewSessionRequest,
    PermissionOptionKind, PromptRequest, ProtocolVersion, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome, SessionInfo,
    SessionInfoUpdate, SessionNotification, SessionUpdate, StopReason, ToolCall, ToolCallContent,
    ToolCallStatus, ToolCallUpdate,
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

                if !message.attachments.is_empty()
                    || !message.tool_calls.is_empty()
                    || message.thinking.is_some()
                {
                    height += px(24.);
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

struct SessionState {
    items: Vec<ChatItem>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    measured: bool,
    last_width: Option<Pixels>,
    pending_remeasure: Vec<usize>,
    markdown_states: HashMap<String, MarkdownState>,
    status_line: String,
    is_generating: bool,
    streaming_role: Option<ChatRole>,
    active_user_index: Option<usize>,
    active_assistant_index: Option<usize>,
    tool_call_index: HashMap<String, usize>,
    tool_call_cache: HashMap<String, ToolCall>,
}

impl SessionState {
    fn new(status_line: impl Into<String>) -> Self {
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

    fn clear_stream_state(&mut self) {
        self.streaming_role = None;
        self.active_user_index = None;
        self.active_assistant_index = None;
        self.tool_call_index.clear();
        self.tool_call_cache.clear();
    }
}

enum CodexCommand {
    Prompt { session_id: String, text: String },
    ListSessions,
    NewSession { cwd: PathBuf },
    LoadSession { session_id: String, cwd: PathBuf },
}

enum UiEvent {
    SessionUpdate {
        session_id: String,
        update: SessionUpdate,
    },
    PromptFinished {
        session_id: String,
        stop_reason: StopReason,
    },
    SessionsListed(Vec<SessionInfo>),
    SessionCreated {
        session_id: String,
        cwd: PathBuf,
    },
    SessionLoaded {
        session_id: String,
    },
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
        let _ = self.updates.try_send(UiEvent::SessionUpdate {
            session_id: args.session_id.to_string(),
            update: args.update,
        });
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

            match client_conn.list_sessions(ListSessionsRequest::new()).await {
                Ok(list_response) => {
                    let _ = updates_tx.try_send(UiEvent::SessionsListed(list_response.sessions));
                }
                Err(err) => {
                    let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                        "Failed to list sessions: {err:?}"
                    )));
                }
            }

            let session_response = match client_conn
                .new_session(NewSessionRequest::new(cwd.clone()))
                .await
            {
                Ok(response) => response,
                Err(err) => {
                    let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                        "Failed to create session: {err:?}"
                    )));
                    return;
                }
            };

            let _ = updates_tx.try_send(UiEvent::SessionCreated {
                session_id: session_response.session_id.to_string(),
                cwd: cwd.clone(),
            });

            let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                "Connected to Codex ACP (session {})",
                session_response.session_id
            )));

            while let Some(command) = commands_rx.recv().await {
                match command {
                    CodexCommand::Prompt { session_id, text } => {
                        let request =
                            PromptRequest::new(session_id.clone(), vec![ContentBlock::from(text)]);
                        match client_conn.prompt(request).await {
                            Ok(response) => {
                                let _ = updates_tx.try_send(UiEvent::PromptFinished {
                                    session_id,
                                    stop_reason: response.stop_reason,
                                });
                            }
                            Err(err) => {
                                let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                                    "Prompt failed: {err:?}"
                                )));
                                let _ = updates_tx.try_send(UiEvent::PromptFinished {
                                    session_id,
                                    stop_reason: StopReason::Cancelled,
                                });
                            }
                        }
                    }
                    CodexCommand::ListSessions => {
                        match client_conn.list_sessions(ListSessionsRequest::new()).await {
                            Ok(list_response) => {
                                let _ = updates_tx
                                    .try_send(UiEvent::SessionsListed(list_response.sessions));
                            }
                            Err(err) => {
                                let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                                    "Failed to list sessions: {err:?}"
                                )));
                            }
                        }
                    }
                    CodexCommand::NewSession { cwd } => {
                        match client_conn
                            .new_session(NewSessionRequest::new(cwd.clone()))
                            .await
                        {
                            Ok(response) => {
                                let _ = updates_tx.try_send(UiEvent::SessionCreated {
                                    session_id: response.session_id.to_string(),
                                    cwd,
                                });
                            }
                            Err(err) => {
                                let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                                    "Failed to create session: {err:?}"
                                )));
                            }
                        }
                    }
                    CodexCommand::LoadSession { session_id, cwd } => {
                        match client_conn
                            .load_session(LoadSessionRequest::new(session_id.clone(), cwd))
                            .await
                        {
                            Ok(_) => {
                                let _ = updates_tx.try_send(UiEvent::SessionLoaded { session_id });
                            }
                            Err(err) => {
                                let _ = updates_tx.try_send(UiEvent::SystemMessage(format!(
                                    "Failed to load session: {err:?}"
                                )));
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

    fn toggle_thoughts(
        &mut self,
        session_id: String,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session = self.ensure_session_state(&session_id);
        let ChatItem::Message(message) = &mut session.items[index];
        if message.thinking.is_some() {
            message.thoughts_open = !message.thoughts_open;
            if !session.pending_remeasure.contains(&index) {
                session.pending_remeasure.push(index);
            }
            cx.notify();
        }
    }

    fn toggle_tools(
        &mut self,
        session_id: String,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session = self.ensure_session_state(&session_id);
        let ChatItem::Message(message) = &mut session.items[index];
        if !message.tool_calls.is_empty() {
            message.tools_open = !message.tools_open;
            if !session.pending_remeasure.contains(&index) {
                session.pending_remeasure.push(index);
            }
            cx.notify();
        }
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
            thoughts_open: false,
            tools_open: false,
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

        let index = if streaming_role == Some(role) && target_index.is_some() {
            target_index.unwrap()
        } else {
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
                thoughts_open: false,
                tools_open: false,
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
        };

        self.append_to_message(session_id, index, text, cx);
        if role == ChatRole::Assistant {
            self.scroll_handle.scroll_to_bottom();
        }
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
            thoughts_open: false,
            tools_open: false,
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

impl Render for ChatSessionView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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

        let sidebar = {
            let theme = cx.theme().clone();
            v_flex()
                .w(px(260.))
                .min_w(px(220.))
                .max_w(px(320.))
                .gap_2()
                .p_3()
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .bg(theme.secondary)
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    Label::new("Sessions")
                                        .text_sm()
                                        .font_semibold()
                                        .text_color(theme.muted_foreground),
                                )
                                .child(
                                    Tag::secondary()
                                        .small()
                                        .child(format!("{}", self.session_list.len())),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .child(
                                    Button::new("refresh-sessions")
                                        .xsmall()
                                        .ghost()
                                        .icon(IconName::Redo)
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.request_sessions();
                                        })),
                                )
                                .child(
                                    Button::new("new-session")
                                        .xsmall()
                                        .ghost()
                                        .icon(IconName::Plus)
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.create_new_session();
                                        })),
                                ),
                        ),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_h_0()
                        .py_1()
                        .overflow_y_scrollbar()
                        .children(self.session_list.iter().map(|session| {
                            let session_id = session.session_id.to_string();
                            let is_active =
                                self.active_session_id.as_deref() == Some(session_id.as_str());
                            let title = session_title(session);
                            let subtitle = session_subtitle(session);
                            let short_id = short_session_id(&session_id);
                            let bg_color = if is_active {
                                theme.accent.opacity(0.18)
                            } else {
                                theme.background
                            };
                            let session_id_for_click = session_id.clone();
                            let indicator_color = if is_active {
                                theme.primary
                            } else {
                                theme.border
                            };

                            div()
                                .id(ElementId::Name(format!("session-{}", session_id).into()))
                                .w_full()
                                .cursor_pointer()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .bg(bg_color)
                                .hover(|style| style.bg(theme.accent.opacity(0.08)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.select_session(session_id_for_click.clone(), cx);
                                }))
                                .child(
                                    v_flex()
                                        .gap_0p5()
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .size_2()
                                                        .rounded_full()
                                                        .bg(indicator_color),
                                                )
                                                .child(Label::new(title).text_sm().font_medium())
                                                .child(div().flex_1())
                                                .child(Tag::secondary().small().child(short_id)),
                                        )
                                        .child(
                                            Label::new(subtitle)
                                                .text_xs()
                                                .text_color(theme.muted_foreground),
                                        ),
                                )
                        })),
                )
        };

        let chat_list = if let Some(session_id) = active_session_id.clone() {
            let list_id = format!("chat-items-{}", session_id);
            let session_id_for_list = session_id.clone();
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

                                        let thoughts_header = message.thinking.as_ref().map(|_| {
                                            let header = build_collapsible_header(
                                                "思考",
                                                message.thoughts_open,
                                                theme.muted_foreground,
                                            );
                                            let session_id_for_toggle = session_id_for_list.clone();
                                            div()
                                                .cursor_pointer()
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(move |this, _, window, cx| {
                                                        this.toggle_thoughts(
                                                            session_id_for_toggle.clone(),
                                                            ix,
                                                            window,
                                                            cx,
                                                        );
                                                    }),
                                                )
                                                .child(header)
                                                .into_any_element()
                                        });

                                        let tools_header = if message.tool_calls.is_empty() {
                                            None
                                        } else {
                                            let title =
                                                format!("工具调用 ({})", message.tool_calls.len());
                                            let header = build_collapsible_header(
                                                &title,
                                                message.tools_open,
                                                theme.muted_foreground,
                                            );
                                            let session_id_for_toggle = session_id_for_list.clone();
                                            Some(
                                                div()
                                                    .cursor_pointer()
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(move |this, _, window, cx| {
                                                            this.toggle_tools(
                                                                session_id_for_toggle.clone(),
                                                                ix,
                                                                window,
                                                                cx,
                                                            );
                                                        }),
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
                )
                .into_any_element()
        } else {
            div()
                .flex_1()
                .w_full()
                .border_1()
                .border_color(cx.theme().border)
                .rounded_lg()
                .bg(cx.theme().background)
                .child(
                    v_flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(Icon::new(IconName::PanelLeft).large())
                        .child(Label::new("Select a session").text_sm())
                        .child(
                            Label::new("选择左侧会话以查看历史")
                                .text_xs()
                                .text_color(cx.theme().muted_foreground),
                        ),
                )
                .into_any_element()
        };

        let header_title = self
            .active_session_info()
            .map(session_title)
            .unwrap_or_else(|| "Chat Session".to_string());
        let header_meta = self.active_session_info().map(session_subtitle);
        let header_id = self
            .active_session_info()
            .map(|session| short_session_id(&session.session_id.to_string()));

        h_flex().size_full().gap_4().p_4().child(sidebar).child(
            v_flex()
                .flex_1()
                .gap_3()
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            Label::new(header_title)
                                                .text_xl()
                                                .font_semibold(),
                                        )
                                        .child(if let Some(header_id) = header_id {
                                            Tag::secondary().small().child(header_id)
                                        } else {
                                            Tag::secondary().small().child("—")
                                        }),
                                )
                                .child(
                                    Label::new(status_line)
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .children(header_meta.map(|meta| {
                                    Label::new(meta)
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                })),
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
                .child(chat_list)
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
                ),
        )
    }
}

fn short_session_id(session_id: &str) -> String {
    session_id.chars().take(8).collect()
}

fn session_title(session: &SessionInfo) -> String {
    session.title.clone().unwrap_or_else(|| {
        format!(
            "Session {}",
            short_session_id(&session.session_id.to_string())
        )
    })
}

fn session_subtitle(session: &SessionInfo) -> String {
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
            .child(Tag::secondary().small().child(message.author.clone()))
            .into_any_element()
    } else {
        let mut header_row = h_flex()
            .w_full()
            .justify_start()
            .items_center()
            .gap_2()
            .child(Label::new(message.author.clone()).font_semibold());
        if let Some(badge) = &message.badge {
            header_row = header_row.child(Tag::secondary().small().child(badge.clone()));
        }
        header_row.into_any_element()
    };

    let mut meta_row = h_flex().items_center().gap_2();
    let mut has_meta = false;
    if !message.attachments.is_empty() {
        meta_row = meta_row.child(
            Tag::secondary()
                .small()
                .child(format!("附件 {}", message.attachments.len())),
        );
        has_meta = true;
    }
    if !message.tool_calls.is_empty() {
        meta_row = meta_row.child(
            Tag::secondary()
                .small()
                .child(format!("工具 {}", message.tool_calls.len())),
        );
        has_meta = true;
    }
    if message.thinking.is_some() {
        meta_row = meta_row.child(Tag::secondary().small().child("思考"));
        has_meta = true;
    }
    let meta_row = if has_meta {
        let row = if is_user {
            meta_row.w_full().justify_end()
        } else {
            meta_row.w_full().justify_start()
        };
        Some(row.into_any_element())
    } else {
        None
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
        .child(content);

    if !message.attachments.is_empty() {
        bubble = bubble.child(build_attachments(&message.attachments, theme));
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
        let mut stack = v_flex().w_full().items_end().gap_1().child(header);
        if let Some(meta_row) = meta_row {
            stack = stack.child(meta_row);
        }
        stack
            .child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .items_start()
                    .gap_3()
                    .child(bubble)
                    .child(avatar_container),
            )
            .into_any_element()
    } else {
        let mut stack = v_flex().w_full().items_start().gap_1().child(header);
        if let Some(meta_row) = meta_row {
            stack = stack.child(meta_row);
        }
        stack
            .child(
                h_flex()
                    .w_full()
                    .justify_start()
                    .items_start()
                    .gap_3()
                    .child(avatar_container)
                    .child(bubble),
            )
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

    let has_args = !tool.args.trim().is_empty();
    let has_output = !tool.output.trim().is_empty();
    let mut details = v_flex().gap_1();
    let mut has_details = false;

    if has_args {
        details = details
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
            );
        has_details = true;
    }

    if has_output {
        details = details
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
            );
        has_details = true;
    }

    if !has_details {
        details = details.child(
            Label::new("Awaiting output")
                .text_xs()
                .text_color(theme.muted_foreground),
        );
    }

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
        .child(details)
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
