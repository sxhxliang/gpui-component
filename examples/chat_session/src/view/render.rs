use agent_client_protocol::SessionInfo;
use gpui::prelude::FluentBuilder;
use gpui::{Context, ElementId, Pixels, Render, Size, Window, div, px};
use gpui::{InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled};
use gpui_component::{
    ActiveTheme as _, ElementExt, Icon, IconName, Sizable, StyledExt,
    button::Button,
    button::ButtonVariants,
    h_flex,
    input::{InputGroup, InputGroupAddon, InputGroupTextarea},
    label::Label,
    scroll::{ScrollableElement, ScrollbarAxis},
    v_flex, v_virtual_list,
};
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::types::ChatItem;
use crate::ui::{build_chat_item_element, format_elapsed_time, project_group_name, session_title};

use super::ChatSessionView;

const SIDEBAR_WIDTH: f32 = 260.0;
const CHAT_LIST_HORIZONTAL_PADDING: f32 = 32.0;
const CHAT_SCROLL_GUTTER: f32 = 12.0;
const MAX_CHAT_WIDTH: f32 = 640.0;

impl ChatSessionView {
    fn measure_width(&self, window: &Window) -> Pixels {
        let base_width = self.list_content_width.unwrap_or_else(|| {
            let viewport_width = window.viewport_size().width;
            (viewport_width - px(SIDEBAR_WIDTH) - px(CHAT_LIST_HORIZONTAL_PADDING)).max(px(0.))
        });
        (base_width - px(CHAT_SCROLL_GUTTER))
            .max(px(0.))
            .min(px(MAX_CHAT_WIDTH))
    }

    fn session_render_state(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Rc<Vec<Size<Pixels>>>, String, Option<String>) {
        let measure_width = self.measure_width(window);
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

        (item_sizes, status_line, active_session_id)
    }

    fn grouped_sessions(&self) -> BTreeMap<String, Vec<&SessionInfo>> {
        let mut grouped: BTreeMap<String, Vec<&SessionInfo>> = BTreeMap::new();
        for session in &self.session_list {
            let group = project_group_name(&session.cwd);
            grouped.entry(group).or_default().push(session);
        }
        grouped
    }

    fn build_sidebar(
        &self,
        theme: &gpui_component::Theme,
        grouped_sessions: BTreeMap<String, Vec<&SessionInfo>>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        v_flex()
            .w(px(SIDEBAR_WIDTH))
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
                            .child(Label::new("Docs").text_sm()),
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
                                Icon::new(IconName::Settings)
                                    .xsmall()
                                    .text_color(theme.muted_foreground),
                            )
                            .child(Label::new("Settings").text_sm()),
                    ),
            )
            .child(
                // Threads header
                h_flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(Label::new("Threads").text_sm().font_medium())
                    .child(div().flex_1())
                    .child(
                        Button::new("collapse")
                            .xsmall()
                            .ghost()
                            .icon(IconName::ChevronUp),
                    ),
            )
            .child(
                // Thread list grouped by project
                v_flex()
                    .id("sidebar-thread-list")
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(
                        v_flex()
                            .id("sidebar-scroll-content")
                            .size_full()
                            .px_2()
                            .overflow_y_scroll()
                            .track_scroll(&self.sidebar_scroll_handle)
                            .children(grouped_sessions.into_iter().map(
                                |(group_name, sessions)| {
                                    let group_theme = theme.clone();
                                    // Get the cwd from the first session in this group for creating new threads
                                    let group_cwd = sessions.first().map(|s| s.cwd.clone());
                                    let group_name_for_id = group_name.clone();
                                    v_flex()
                                        .gap_0p5()
                                        .pb_2()
                                        .child(
                                            // Project group header - clickable to create new thread
                                            div()
                                                .id(ElementId::Name(
                                                    format!("group-{group_name_for_id}").into(),
                                                ))
                                                .w_full()
                                                .cursor_pointer()
                                                .rounded_md()
                                                .hover(|s| s.bg(group_theme.secondary))
                                                .on_click(cx.listener(move |this, _, _, _| {
                                                    if let Some(cwd) = group_cwd.clone() {
                                                        let _ = this.codex_commands.send(
                                                            crate::bridge::CodexCommand::NewSession { cwd },
                                                        );
                                                    }
                                                }))
                                                .child(
                                                    h_flex()
                                                        .items_center()
                                                        .gap_2()
                                                        .px_2()
                                                        .py_1p5()
                                                        .child(
                                                            Icon::new(IconName::FolderOpen)
                                                                .xsmall()
                                                                .text_color(
                                                                    group_theme.muted_foreground,
                                                                ),
                                                        )
                                                        .child(
                                                            Label::new(group_name)
                                                                .text_sm()
                                                                .font_medium()
                                                                .text_color(
                                                                    group_theme.foreground,
                                                                ),
                                                        ),
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
                                                    format!("session-{session_id}").into(),
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
                                                    this.select_session(
                                                        session_id_for_click.clone(),
                                                        cx,
                                                    );
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
                                                                Label::new(elapsed)
                                                                    .text_xs()
                                                                    .text_color(
                                                                        item_theme.muted_foreground,
                                                                    ),
                                                            )
                                                        }),
                                                )
                                        }))
                                },
                            )),
                    )
                    .scrollbar(&self.sidebar_scroll_handle, ScrollbarAxis::Vertical),
            )
            .into_any_element()
    }

    fn build_chat_header(&self, theme: &gpui_component::Theme) -> gpui::AnyElement {
        let chat_header_title = self
            .active_session_info()
            .map(session_title)
            .unwrap_or_else(|| "Chat".to_string());
        let chat_header_project = self
            .active_session_info()
            .map(|s| project_group_name(&s.cwd));

        h_flex()
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
            )
            .into_any_element()
    }

    fn build_chat_list(
        &self,
        theme: &gpui_component::Theme,
        item_sizes: Rc<Vec<Size<Pixels>>>,
        active_session_id: Option<String>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if let Some(session_id) = active_session_id.clone() {
            let list_id = format!("chat-items-{session_id}");
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

                                        let view_entity = cx.entity().clone();
                                        elements.push(
                                            div()
                                                .id(ElementId::Name(
                                                    format!("chat-item-{ix}").into(),
                                                ))
                                                .w_full()
                                                .child(element)
                                                .on_prepaint(move |bounds, _, cx| {
                                                    let width = bounds.size.width;
                                                    view_entity.update(cx, |this, cx| {
                                                        if this.list_content_width != Some(width)
                                                        {
                                                            this.list_content_width = Some(width);
                                                            cx.notify();
                                                        }
                                                    });
                                                }),
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
            // Get current project name for the dropdown
            let current_project = project_group_name(&self.cwd);

            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .child(
                    v_flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .gap_4()
                        .child(
                            // Cloud/terminal icon
                            div()
                                .size(px(64.))
                                .rounded_xl()
                                .border_1()
                                .border_color(theme.border)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new(IconName::Bot)
                                        .size(px(32.))
                                        .text_color(theme.foreground),
                                ),
                        )
                        .child(
                            Label::new("Let's build")
                                .text_xl()
                                .font_semibold()
                                .text_color(theme.foreground),
                        )
                        .child(
                            // Project selector button
                            div()
                                .id("project-selector")
                                .cursor_pointer()
                                .px_3()
                                .py_2()
                                .rounded_lg()
                                .border_1()
                                .border_color(theme.border)
                                .hover(|s| s.bg(theme.secondary))
                                .on_click(cx.listener(|this, _, _, _| {
                                    this.create_new_session();
                                }))
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            Icon::new(IconName::FolderOpen)
                                                .xsmall()
                                                .text_color(theme.muted_foreground),
                                        )
                                        .child(
                                            Label::new(current_project)
                                                .text_sm()
                                                .text_color(theme.foreground),
                                        )
                                        .child(
                                            Icon::new(IconName::ChevronDown)
                                                .xsmall()
                                                .text_color(theme.muted_foreground),
                                        ),
                                ),
                        ),
                )
                .into_any_element()
        }
    }

    fn build_input_area(
        &self,
        theme: &gpui_component::Theme,
        status_line: String,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        v_flex()
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
            )
            .into_any_element()
    }
}

impl Render for ChatSessionView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let (item_sizes, status_line, active_session_id) = self.session_render_state(window, cx);
        let grouped_sessions = self.grouped_sessions();

        let sidebar = self.build_sidebar(&theme, grouped_sessions, cx);
        let chat_header = self.build_chat_header(&theme);
        let chat_list = self.build_chat_list(&theme, item_sizes, active_session_id, cx);
        let input_area = self.build_input_area(&theme, status_line, cx);

        let chat_area = v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(theme.background)
            .child(chat_header)
            .child(chat_list)
            .child(input_area);

        h_flex().size_full().child(sidebar).child(chat_area)
    }
}
