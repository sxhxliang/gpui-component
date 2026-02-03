//! Codex ACP bridge for communication between UI and Codex agent.

use std::{path::PathBuf, rc::Rc, sync::Arc, thread};

use agent_client_protocol::{
    Agent, AgentSideConnection, AuthMethodId, AuthenticateRequest, Client, ClientCapabilities,
    ClientSideConnection, ContentBlock, Error, Implementation, InitializeRequest,
    ListSessionsRequest, LoadSessionRequest, NewSessionRequest, PermissionOptionKind,
    PromptRequest, ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionInfo, SessionNotification,
    SessionUpdate, StopReason,
};
use codex_acp::CodexAgent;
use codex_core::config::{Config, ConfigOverrides};
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

/// Commands sent from UI to Codex.
pub enum CodexCommand {
    Prompt { session_id: String, text: String },
    ListSessions,
    NewSession { cwd: PathBuf },
    LoadSession { session_id: String, cwd: PathBuf },
}

/// Events sent from Codex to UI.
#[allow(clippy::large_enum_variant)]
pub enum UiEvent {
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

/// Client that receives notifications from Codex agent.
pub struct UiClient {
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

/// Bridge between UI and Codex agent.
pub struct CodexBridge {
    pub commands: tokio::sync::mpsc::UnboundedSender<CodexCommand>,
    pub updates: smol::channel::Receiver<UiEvent>,
}

/// Spawns the Codex bridge in a background thread.
pub fn spawn_codex_bridge() -> CodexBridge {
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
