use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, Sizable, Size,
    avatar::Avatar,
    button::Button,
    h_flex,
    input::{Input, InputState},
    switch::Switch,
    v_flex,
};
use gpui_component_assets::Assets;

// ── Login View ──────────────────────────────────────────────────────────────

struct LoginView {
    show_network_settings: bool,
    proxy_enabled: bool,
    proxy_address_input: Entity<InputState>,
    proxy_port_input: Entity<InputState>,
    proxy_account_input: Entity<InputState>,
    proxy_password_input: Entity<InputState>,
}

impl LoginView {
    fn compact_window_size() -> gpui::Size<Pixels> {
        size(px(283.), px(379.))
    }

    fn expanded_window_size() -> gpui::Size<Pixels> {
        size(px(283.), px(440.))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let proxy_address_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入地址"));
        let proxy_port_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入端口"));
        let proxy_account_input = cx.new(|cx| InputState::new(window, cx).placeholder("输入账户"));
        let proxy_password_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入密码")
                .masked(true)
        });

        Self {
            show_network_settings: false,
            proxy_enabled: true,
            proxy_address_input,
            proxy_port_input,
            proxy_account_input,
            proxy_password_input,
        }
    }

    fn render_login(&self, cx: &mut Context<Self>) -> AnyElement {
        let top_text = rgb(0x9C9C9C);
        let icon_color = rgb(0x555555);
        let name_color = rgb(0x222222);
        let action_green = rgb(0x07C160);
        let action_green_hover = rgb(0x06B457);
        let action_green_active = rgb(0x059D4C);
        let link_color = rgb(0x667AA0);
        let split_color = rgb(0xD1D3D8);
        let avatar_placeholder = rgb(0xC5CBD4);
        let top_hover_bg = rgb(0xE2E2E2);
        let top_active_bg = rgb(0xD8D8D8);
        let close_hover_bg = rgb(0xE81123);
        let close_active_bg = rgb(0xC50F1F);

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .w_full()
                    .h(px(32.))
                    .items_start()
                    .justify_between()
                    .child(
                        div()
                            .h_full()
                            .pl(px(9.))
                            .pt(px(9.))
                            .text_sm()
                            .text_color(top_text)
                            .child("微信"),
                    )
                    .child(
                        h_flex()
                            .h_full()
                            .items_start()
                            .child(
                                div()
                                    .id("settings-btn")
                                    .cursor_default()
                                    .w(px(33.))
                                    .h_full()
                                    .flex()
                                    .justify_center()
                                    .pt(px(7.))
                                    .text_color(icon_color)
                                    .hover(|this: StyleRefinement| this.bg(top_hover_bg))
                                    .active(|this: StyleRefinement| this.bg(top_active_bg))
                                    .child(Icon::new(IconName::Settings).size_4())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.show_network_settings = true;
                                        window.resize(if this.proxy_enabled {
                                            Self::expanded_window_size()
                                        } else {
                                            Self::compact_window_size()
                                        });
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("close-btn")
                                    .cursor_default()
                                    .w(px(37.))
                                    .h_full()
                                    .flex()
                                    .justify_center()
                                    .pt(px(7.))
                                    .text_color(icon_color)
                                    .hover(|this: StyleRefinement| {
                                        this.bg(close_hover_bg).text_color(gpui::white())
                                    })
                                    .active(|this: StyleRefinement| {
                                        this.bg(close_active_bg).text_color(gpui::white())
                                    })
                                    .child(Icon::new(IconName::Close).size_4())
                                    .on_click(|_, window, _| {
                                        window.remove_window();
                                    }),
                            ),
                    ),
            )
            .child(
                v_flex().flex_1().w_full().items_center().child(
                    v_flex()
                        .items_center()
                        .mt(px(40.))
                        .child(
                            div()
                                .relative()
                                .size(px(75.))
                                .rounded(px(8.))
                                .overflow_hidden()
                                .bg(avatar_placeholder)
                                .child(
                                    img("examples/wechat/assets/login-avatar.jpg")
                                        .size_full()
                                        .object_fit(ObjectFit::Cover),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .right(px(-3.))
                                        .bottom(px(-3.))
                                        .w(px(23.))
                                        .h(px(16.))
                                        .rounded(px(5.))
                                        .bg(gpui::white())
                                        .border_1()
                                        .border_color(rgb(0xD9D9D9))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_sm()
                                        .child("🇨🇳"),
                                ),
                        )
                        .child(
                            div()
                                .mt(px(22.))
                                // .text_()
                                .text_color(name_color)
                                .child("仕华"),
                        ),
                ),
            )
            .child(
                v_flex()
                    .w_full()
                    .items_center()
                    .pb(px(30.))
                    .child(
                        div()
                            .id("login-btn")
                            .w(px(180.))
                            .h(px(36.))
                            .rounded(px(4.))
                            .bg(action_green)
                            .cursor_default()
                            .hover(|this: StyleRefinement| this.bg(action_green_hover))
                            .active(|this: StyleRefinement| this.bg(action_green_active))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(div().text_sm().text_color(gpui::white()).child("进入微信"))
                            .on_click(|_, window, cx| {
                                window.remove_window();
                                open_chat_window(cx);
                            }),
                    )
                    .child(
                        h_flex()
                            .mt(px(20.))
                            .items_center()
                            .gap(px(9.))
                            .child(
                                div()
                                    .id("switch-account")
                                    .cursor_default()
                                    .text_sm()
                                    .text_color(link_color)
                                    .hover(|this: StyleRefinement| this.opacity(0.75))
                                    .child("切换账号"),
                            )
                            .child(div().w(px(1.)).h(px(15.)).bg(split_color))
                            .child(
                                div()
                                    .id("file-transfer")
                                    .cursor_default()
                                    .text_sm()
                                    .text_color(link_color)
                                    .hover(|this: StyleRefinement| this.opacity(0.75))
                                    .child("仅传输文件"),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_network_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let page_bg = rgb(0xECECEC);
        let icon_color = rgb(0x555555);
        let title_color = rgb(0x07C160);
        let card_bg = rgb(0xF5F5F5);
        let card_border = rgb(0xD0D0D0);
        let text_color = rgb(0x2E2E2E);
        let field_bg = rgb(0xECECEC);
        let line_color = rgb(0xD9D9D9);
        let action_green = rgb(0x07C160);
        let action_green_hover = rgb(0x06B457);
        let top_hover_bg = rgb(0xE2E2E2);
        let top_active_bg = rgb(0xD8D8D8);
        let close_hover_bg = rgb(0xE81123);
        let close_active_bg = rgb(0xC50F1F);

        let field_row =
            |id: &'static str, title: &'static str, state: &Entity<InputState>, with_line: bool| {
                v_flex()
                    .w_full()
                    .child(
                        h_flex()
                            .id(id)
                            .w_full()
                            .h(px(48.))
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .w(px(45.))
                                    .text_size(px(13.))
                                    .text_color(text_color)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .w(px(119.))
                                    .h(px(23.))
                                    .rounded(px(5.))
                                    .bg(field_bg)
                                    .px(px(5.))
                                    .child(Input::new(state).appearance(false).small().w_full()),
                            ),
                    )
                    .when(with_line, |this: Div| {
                        this.child(div().w_full().h(px(1.)).bg(line_color))
                    })
            };

        v_flex()
            .size_full()
            .bg(page_bg)
            .child(
                h_flex()
                    .w_full()
                    .h(px(32.))
                    .items_start()
                    .justify_between()
                    .child(
                        div()
                            .id("network-back")
                            .cursor_default()
                            .w(px(33.))
                            .h_full()
                            .flex()
                            .justify_center()
                            .pt(px(7.))
                            .text_color(icon_color)
                            .hover(|this: StyleRefinement| this.bg(top_hover_bg))
                            .active(|this: StyleRefinement| this.bg(top_active_bg))
                            .child(Icon::new(IconName::ChevronLeft).size_4())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.show_network_settings = false;
                                window.resize(Self::compact_window_size());
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("network-close")
                            .cursor_default()
                            .w(px(37.))
                            .h_full()
                            .flex()
                            .justify_center()
                            .pt(px(7.))
                            .text_color(icon_color)
                            .hover(|this: StyleRefinement| {
                                this.bg(close_hover_bg).text_color(gpui::white())
                            })
                            .active(|this: StyleRefinement| {
                                this.bg(close_active_bg).text_color(gpui::white())
                            })
                            .child(Icon::new(IconName::Close).size_4())
                            .on_click(|_, window, _| {
                                window.remove_window();
                            }),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .w_full()
                    .items_center()
                    .child(
                        div()
                            .mt(px(18.))
                            .text_color(title_color)
                            .child("网络代理设置"),
                    )
                    .child(
                        div()
                            .mt(px(25.))
                            .w(px(231.))
                            .rounded(px(9.))
                            .border_1()
                            .border_color(card_border)
                            .bg(card_bg)
                            .px(px(15.))
                            .py(px(10.))
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .child(div().text_sm().text_color(text_color).child("使用代理"))
                                    .child(
                                        Switch::new("proxy-switch")
                                            .small()
                                            .checked(self.proxy_enabled)
                                            .on_click(cx.listener(|this, checked, window, cx| {
                                                this.proxy_enabled = *checked;
                                                window.resize(if *checked {
                                                    Self::expanded_window_size()
                                                } else {
                                                    Self::compact_window_size()
                                                });
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    )
                    .when(self.proxy_enabled, |this| {
                        this.child(
                            div()
                                .mt(px(17.))
                                .w(px(231.))
                                .rounded(px(9.))
                                .border_1()
                                .border_color(card_border)
                                .bg(card_bg)
                                .px(px(15.))
                                .child(field_row(
                                    "proxy-address",
                                    "地址",
                                    &self.proxy_address_input,
                                    true,
                                ))
                                .child(field_row(
                                    "proxy-port",
                                    "端口",
                                    &self.proxy_port_input,
                                    true,
                                ))
                                .child(field_row(
                                    "proxy-account",
                                    "账户",
                                    &self.proxy_account_input,
                                    true,
                                ))
                                .child(field_row(
                                    "proxy-password",
                                    "密码",
                                    &self.proxy_password_input,
                                    false,
                                )),
                        )
                    })
                    .child(
                        div()
                            .mt(if self.proxy_enabled { px(24.) } else { px(41.) })
                            .w(px(76.))
                            .h(px(29.))
                            .rounded(px(4.))
                            .bg(action_green)
                            .cursor_default()
                            .hover(|this: StyleRefinement| this.bg(action_green_hover))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(gpui::white())
                                    .child("保存"),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl Render for LoginView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.show_network_settings {
            self.render_network_settings(cx)
        } else {
            self.render_login(cx)
        }
    }
}

fn open_chat_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(900.), px(640.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|cx| ChatView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        },
    )
    .unwrap();
}

// ── Chat View ───────────────────────────────────────────────────────────────

struct Contact {
    name: &'static str,
    last_msg: &'static str,
    time: &'static str,
}

struct ChatMessage {
    sender: &'static str,
    content: &'static str,
    is_me: bool,
}

struct ChatView {
    contacts: Vec<Contact>,
    messages: Vec<ChatMessage>,
    active_contact: usize,
}

impl ChatView {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            contacts: vec![
                Contact {
                    name: "文件传输助手",
                    last_msg: "图片",
                    time: "昨天",
                },
                Contact {
                    name: "张三",
                    last_msg: "明天见！",
                    time: "10:30",
                },
                Contact {
                    name: "李四",
                    last_msg: "收到，谢谢",
                    time: "09:15",
                },
                Contact {
                    name: "工作群",
                    last_msg: "[图片]",
                    time: "昨天",
                },
                Contact {
                    name: "王五",
                    last_msg: "好的，没问题",
                    time: "周一",
                },
                Contact {
                    name: "家人群",
                    last_msg: "晚上一起吃饭",
                    time: "周日",
                },
            ],
            messages: vec![
                ChatMessage {
                    sender: "张三",
                    content: "你好，最近忙吗？",
                    is_me: false,
                },
                ChatMessage {
                    sender: "我",
                    content: "还好，有什么事吗？",
                    is_me: true,
                },
                ChatMessage {
                    sender: "张三",
                    content: "想约你明天出来聊聊项目的事情",
                    is_me: false,
                },
                ChatMessage {
                    sender: "我",
                    content: "可以啊，什么时间？",
                    is_me: true,
                },
                ChatMessage {
                    sender: "张三",
                    content: "下午两点怎么样？",
                    is_me: false,
                },
                ChatMessage {
                    sender: "我",
                    content: "没问题，明天见！",
                    is_me: true,
                },
                ChatMessage {
                    sender: "张三",
                    content: "明天见！",
                    is_me: false,
                },
            ],
            active_contact: 1, // 张三
        }
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(260.))
            .h_full()
            .bg(cx.theme().background)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                // Search bar placeholder
                div().px(px(12.)).py(px(10.)).child(
                    h_flex()
                        .w_full()
                        .h(px(28.))
                        .rounded(px(4.))
                        .bg(cx.theme().muted)
                        .items_center()
                        .justify_center()
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .child(
                                    Icon::new(IconName::Search)
                                        .size_3()
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("搜索"),
                                ),
                        ),
                ),
            )
            // Contact list
            .child(
                v_flex()
                    .id("contact-list")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(self.contacts.iter().enumerate().map(|(i, contact)| {
                        let is_active = i == self.active_contact;
                        h_flex()
                            .id(ElementId::Name(format!("contact-{i}").into()))
                            .w_full()
                            .px(px(12.))
                            .py(px(10.))
                            .gap(px(10.))
                            .items_center()
                            .when(is_active, |el: Stateful<Div>| el.bg(cx.theme().muted))
                            .hover(|el: StyleRefinement| el.bg(cx.theme().muted))
                            .cursor_pointer()
                            .child(
                                Avatar::new()
                                    .name(contact.name)
                                    .with_size(Size::Size(px(36.))),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .overflow_hidden()
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().foreground)
                                                    .child(contact.name.to_string()),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(contact.time.to_string()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .text_ellipsis()
                                            .child(contact.last_msg.to_string()),
                                    ),
                            )
                    })),
            )
    }

    fn render_chat_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let wechat_green = rgb(0x07C160);
        let contact = &self.contacts[self.active_contact];

        v_flex()
            .flex_1()
            .h_full()
            .bg(cx.theme().background)
            .child(
                // Chat header
                h_flex()
                    .w_full()
                    .h(px(50.))
                    .px(px(16.))
                    .items_center()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_base()
                            .text_color(cx.theme().foreground)
                            .child(contact.name.to_string()),
                    ),
            )
            // Messages area
            .child(
                v_flex()
                    .id("messages")
                    .flex_1()
                    .overflow_y_scroll()
                    .p(px(16.))
                    .gap(px(12.))
                    .children(self.messages.iter().enumerate().map(|(i, msg)| {
                        let bubble_bg = if msg.is_me {
                            wechat_green
                        } else {
                            cx.theme().muted.into()
                        };
                        let text_color = if msg.is_me {
                            gpui::white()
                        } else {
                            cx.theme().foreground
                        };

                        h_flex()
                            .id(ElementId::Name(format!("msg-{i}").into()))
                            .w_full()
                            .when(msg.is_me, |el: Stateful<Div>| el.flex_row_reverse())
                            .gap(px(8.))
                            .child(
                                Avatar::new()
                                    .name(msg.sender)
                                    .with_size(Size::Size(px(32.))),
                            )
                            .child(
                                div()
                                    .px(px(12.))
                                    .py(px(8.))
                                    .rounded(px(6.))
                                    .bg(bubble_bg)
                                    .max_w(px(400.))
                                    .text_sm()
                                    .text_color(text_color)
                                    .child(msg.content.to_string()),
                            )
                    })),
            )
            // Input area
            .child(
                v_flex()
                    .w_full()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .w_full()
                            .h(px(120.))
                            .p(px(12.))
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("在此输入消息..."),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .px(px(12.))
                            .pb(px(10.))
                            .justify_end()
                            .child(Button::new("send").label("发送(S)").with_size(Size::Small)),
                    ),
            )
    }
}

impl Render for ChatView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .child(self.render_sidebar(cx))
            .child(self.render_chat_area(cx))
    }
}

// ── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, LoginView::compact_window_size(), cx);
        let window_options = WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };

        cx.open_window(window_options, |window, cx| {
            let view = cx.new(|cx| LoginView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        })
        .unwrap();

        cx.activate(true);
    });
}
