//! Chat session view controller.

use std::{collections::HashMap, path::PathBuf};

use agent_client_protocol::SessionInfo;

use gpui::IntoElement;
use gpui::{AppContext, Context, Entity, ScrollHandle, Task, Window};
use gpui_component::{VirtualListScrollHandle, input::InputState};

use crate::bridge::{CodexCommand, spawn_codex_bridge};
use crate::session::SessionState;

mod actions;
mod events;
mod measure;
mod render;
mod state;

pub struct ChatSessionView {
    sessions: HashMap<String, SessionState>,
    session_list: Vec<SessionInfo>,
    active_session_id: Option<String>,
    scroll_handle: VirtualListScrollHandle,
    sidebar_scroll_handle: ScrollHandle,
    input_state: Entity<InputState>,
    app_status: String,
    codex_commands: tokio::sync::mpsc::UnboundedSender<CodexCommand>,
    cwd: PathBuf,
    list_content_width: Option<gpui::Pixels>,
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
        let sidebar_scroll_handle = ScrollHandle::new();
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
            sidebar_scroll_handle,
            input_state,
            app_status: "Connecting to Codex ACP...".to_string(),
            codex_commands,
            cwd,
            list_content_width: None,
            _codex_task,
        }
    }
}
