//! Chat session example with collapsible thoughts, tool calls, files, and streaming markdown.
//!
//! Highlights:
//! - Virtual list with measured dynamic heights
//! - Markdown rendering with cached MarkdownState
//! - Simulated streaming updates
//! - Collapsible sections for thoughts and tool calls

use std::{collections::HashMap, rc::Rc};

use gpui::*;
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable, StyledExt, VirtualListScrollHandle,
    button::{Button, ButtonVariants},
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

const STREAM_REPLY: &str = r#"## 相对论的直觉版讲解（流式演示）

以下是一个简洁的直觉框架：

1. **狭义相对论**：光速恒定、同时性是相对的。
2. **时间膨胀**：运动得越快，时间越慢。
3. **长度收缩**：沿运动方向会变短。
4. **广义相对论**：引力可以理解为时空弯曲。

> 这只是概念层面的直觉解释，真正推导需要数学工具（微分几何等）。

### 小结
- 速度影响时间与长度的测量方式
- 引力影响时间流逝与空间几何
- 观察者不同，结论不同
"#;
const STREAMING_MESSAGE_ID: usize = 3;

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
struct ToolCall {
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
    tool_calls: Vec<ToolCall>,
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

pub struct ChatSessionView {
    items: Vec<ChatItem>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
    measured: bool,
    last_width: Option<Pixels>,
    pending_remeasure: Vec<usize>,
    markdown_states: HashMap<String, MarkdownState>,
    stream_tx: smol::channel::Sender<String>,
    streaming_message_id: usize,
    input_state: Entity<InputState>,
    _stream_task: Task<()>,
    _update_task: Task<()>,
}

impl ChatSessionView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = Self::sample_items();
        let item_sizes = Self::estimate_sizes(&items);

        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Ask a follow-up...")
                .multi_line(true)
        });

        let mut markdown_states = HashMap::new();
        let streaming_message_id = STREAMING_MESSAGE_ID;

        for item in &items {
            let ChatItem::Message(message) = item;
            if message.role == ChatRole::Assistant {
                let state = MarkdownState::new(&message.content, cx);
                let message_id = message.id;
                cx.observe(state.entity(), move |this: &mut Self, _, cx| {
                    if !this.pending_remeasure.contains(&message_id) {
                        this.pending_remeasure.push(message_id);
                    }
                    cx.notify();
                })
                .detach();
                markdown_states.insert(message.id.to_string(), state);
            }
        }

        let (tx, rx) = smol::channel::unbounded::<String>();
        let scroll_handle = VirtualListScrollHandle::new();

        let stream_state = markdown_states
            .get(&streaming_message_id.to_string())
            .cloned();

        let _stream_task = if let Some(stream_state) = stream_state {
            let weak_state = stream_state.downgrade();
            let scroll_handle = scroll_handle.clone();
            cx.spawn(async move |_, cx| {
                while let Ok(chunk) = rx.recv().await {
                    _ = weak_state.update(cx, |state, cx| {
                        state.push_str(&chunk, cx);
                        scroll_handle.scroll_to_bottom();
                    });
                }
            })
        } else {
            Task::ready(())
        };

        Self {
            items,
            item_sizes: Rc::new(item_sizes),
            scroll_handle,
            measured: false,
            last_width: None,
            pending_remeasure: Vec::new(),
            markdown_states,
            stream_tx: tx,
            streaming_message_id,
            input_state,
            _stream_task,
            _update_task: Task::ready(()),
        }
    }

    fn estimate_sizes(items: &[ChatItem]) -> Vec<Size<Pixels>> {
        items
            .iter()
            .map(|item| Size {
                width: px(0.),
                height: item.estimated_height(),
            })
            .collect()
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

    fn replay_stream(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(state) = self
            .markdown_states
            .get(&self.streaming_message_id.to_string())
        {
            state.update(cx, |state, cx| {
                state.set_text("", cx);
            });
        }

        let tx = self.stream_tx.clone();
        self._update_task = cx.background_executor().spawn(async move {
            let chars: Vec<char> = STREAM_REPLY.chars().collect();
            let mut current = 0;
            let chunk_size = 12usize;

            while current < chars.len() {
                let take = chunk_size.min(chars.len() - current);
                let chunk: String = chars[current..current + take].iter().collect();
                _ = tx.try_send(chunk);
                current += take;
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
        });
    }

    fn sample_items() -> Vec<ChatItem> {
        vec![
            ChatItem::Message(ChatMessage {
                id: 0,
                role: ChatRole::User,
                author: "男生".to_string(),
                badge: None,
                content: "请用通俗语言解释相对论，并给出一个 5 分钟讲解稿。".to_string(),
                thinking: None,
                tool_calls: Vec::new(),
                attachments: Vec::new(),
                thoughts_open: false,
                tools_open: false,
            }),
            ChatItem::Message(ChatMessage {
                id: 1,
                role: ChatRole::Assistant,
                author: "manus".to_string(),
                badge: Some("Lite".to_string()),
                content: r#"我已经为你生成了音频与讲解稿，内容包含：

1. **狭义相对论**：解释光速不变与惯性系等价。
2. **时间膨胀**：高速运动会让时间“变慢”。
3. **广义相对论**：引力可以理解为时空弯曲。

如果你需要更长版本或配图说明，告诉我即可。"#
                    .to_string(),
                thinking: Some(
                    "先给出直觉解释，再通过例子联系日常经验。保持结构清晰，避免公式。".to_string(),
                ),
                tool_calls: vec![
                    ToolCall {
                        name: "audio.generate".to_string(),
                        status: ToolStatus::Success,
                        duration: "1.4s".to_string(),
                        args: r#"{"voice":"male","style":"lecture","length":"5min"}"#.to_string(),
                        output: "生成 audio/relativity_explanation.wav".to_string(),
                    },
                    ToolCall {
                        name: "doc.summarize".to_string(),
                        status: ToolStatus::Success,
                        duration: "520ms".to_string(),
                        args: r#"{"source":"relativity.md","format":"bullet"}"#.to_string(),
                        output: "提取 3 个核心要点".to_string(),
                    },
                ],
                attachments: vec![
                    FileAttachment {
                        name: "relativity_explanation.wav".to_string(),
                        size: "10.7 MB".to_string(),
                        kind: "Audio".to_string(),
                    },
                    FileAttachment {
                        name: "relativity_script.md".to_string(),
                        size: "3.0 KB".to_string(),
                        kind: "Markdown".to_string(),
                    },
                ],
                thoughts_open: false,
                tools_open: false,
            }),
            ChatItem::Message(ChatMessage {
                id: 2,
                role: ChatRole::User,
                author: "男生".to_string(),
                badge: None,
                content: "再简短一些，用要点列表输出。".to_string(),
                thinking: None,
                tool_calls: Vec::new(),
                attachments: Vec::new(),
                thoughts_open: false,
                tools_open: false,
            }),
            ChatItem::Message(ChatMessage {
                id: 3,
                role: ChatRole::Assistant,
                author: "manus".to_string(),
                badge: Some("Lite".to_string()),
                content: r#"我已经为你生成了音频与讲解稿，内容包含：

1. **狭义相对论**：解释光速不变与惯性系等价。
2. **时间膨胀**：高速运动会让时间“变慢”。
3. **广义相对论**：引力可以理解为时空弯曲。

如果你需要更长版本或配图说明，告诉我即可。"#
                    .to_string(),
                thinking: Some("等待用户触发流式演示，然后逐段输出。".to_string()),
                tool_calls: vec![ToolCall {
                    name: "tools.status".to_string(),
                    status: ToolStatus::Running,
                    duration: "—".to_string(),
                    args: r#"{"mode":"stream","target":"assistant_reply"}"#.to_string(),
                    output: "等待开始...".to_string(),
                }],
                attachments: Vec::new(),
                thoughts_open: false,
                tools_open: true,
            }),
        ]
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
                                Label::new("相对论讲解 / 演示对话")
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Tag::secondary().small().child("GPT-4o mini"))
                            .child(
                                Button::new("stream-reply")
                                    .small()
                                    .outline()
                                    .label("Stream Reply")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.replay_stream(window, cx);
                                    })),
                            )
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
                                    .rounded_full(),
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

fn build_tool_call_card(tool: &ToolCall, theme: &gpui_component::Theme) -> AnyElement {
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
