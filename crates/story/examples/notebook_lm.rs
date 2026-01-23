//! NotebookLM - A clone of Google NotebookLM UI
//!
//! This component demonstrates:
//! - Top navigation bar with logo, settings, and user avatar
//! - Tab navigation for filtering notebooks
//! - View toggle between grid and list views
//! - Featured notebook cards with images and metadata
//! - Recent notebooks section

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Icon, IconName, Selectable, Sizable, StyledExt as _,
    Root,
    button::{Button, ButtonGroup, ButtonVariants as _},
    scroll::ScrollableElement as _,
    tab::{Tab, TabBar},
};
use gpui_component_assets::Assets;

#[cfg(feature = "story")]
use gpui_component_story::Story;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Clone, Debug)]
pub struct Notebook {
    pub id: String,
    pub title: String,
    pub category: String,
    pub date: String,
    pub sources: u32,
    pub background_color: Hsla,
    pub is_featured: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Grid,
    List,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TabFilter {
    All,
    MyNotebooks,
    Featured,
}

impl TabFilter {
    fn label(&self) -> &'static str {
        match self {
            TabFilter::All => "全部",
            TabFilter::MyNotebooks => "我的笔记本",
            TabFilter::Featured => "精选笔记本",
        }
    }
}

// ============================================================================
// Mock Data
// ============================================================================

fn mock_notebooks() -> Vec<Notebook> {
    vec![
        Notebook {
            id: "1".to_string(),
            title: "聊天机器人可以为医生和患者提供服务吗?".to_string(),
            category: "Google Research".to_string(),
            date: "2025年7月3日".to_string(),
            sources: 24,
            background_color: hsla(250.0 / 360.0, 0.7, 0.3, 1.0),
            is_featured: true,
        },
        Notebook {
            id: "2".to_string(),
            title: "科学家如何得知基因组中的信息?".to_string(),
            category: "Google Research".to_string(),
            date: "2025年7月10日".to_string(),
            sources: 36,
            background_color: hsla(270.0 / 360.0, 0.6, 0.4, 1.0),
            is_featured: true,
        },
        Notebook {
            id: "3".to_string(),
            title: "科学迷的黄石游玩指南".to_string(),
            category: "旅游".to_string(),
            date: "2025年5月12日".to_string(),
            sources: 17,
            background_color: hsla(30.0 / 360.0, 0.7, 0.5, 1.0),
            is_featured: true,
        },
        Notebook {
            id: "4".to_string(),
            title: "威廉·莎士比亚：戏剧全集".to_string(),
            category: "艺术与文化".to_string(),
            date: "2025年4月26日".to_string(),
            sources: 45,
            background_color: hsla(140.0 / 360.0, 0.3, 0.3, 1.0),
            is_featured: true,
        },
        Notebook {
            id: "5".to_string(),
            title: "Cognitive Digital Twins: Architecture,...".to_string(),
            category: "".to_string(),
            date: "2026年1月21日".to_string(),
            sources: 1,
            background_color: hsla(330.0 / 360.0, 0.6, 0.7, 1.0),
            is_featured: false,
        },
        Notebook {
            id: "6".to_string(),
            title: "The 2025 AI Agent Landscape:...".to_string(),
            category: "".to_string(),
            date: "2026年1月21日".to_string(),
            sources: 40,
            background_color: hsla(200.0 / 360.0, 0.5, 0.7, 1.0),
            is_featured: false,
        },
        Notebook {
            id: "7".to_string(),
            title: "Untitled notebook".to_string(),
            category: "".to_string(),
            date: "2026年1月21日".to_string(),
            sources: 0,
            background_color: hsla(260.0 / 360.0, 0.4, 0.5, 1.0),
            is_featured: false,
        },
    ]
}

// ============================================================================
// Main Story Component
// ============================================================================

pub struct NotebookLMStory {
    focus_handle: FocusHandle,
    notebooks: Vec<Notebook>,
    view_mode: ViewMode,
    active_tab_ix: usize,
}

impl NotebookLMStory {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            notebooks: mock_notebooks(),
            view_mode: ViewMode::Grid,
            active_tab_ix: 0,
        }
    }

    fn set_view_mode(&mut self, mode: ViewMode, cx: &mut Context<Self>) {
        self.view_mode = mode;
        cx.notify();
    }

    fn set_active_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.active_tab_ix = ix;
        cx.notify();
    }

    fn render_top_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .px_6()
            .py_4()
            .border_b_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(Icon::new(IconName::Network).size_6().text_color(theme.foreground))
                    .child(
                        div()
                            .text_xl()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child("NotebookLM"),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        Button::new("settings")
                            .ghost()
                            .icon(IconName::Settings)
                            .label("设置"),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(gpui::rgb(0xfbbf24))
                            .text_xs()
                            .font_semibold()
                            .text_color(gpui::rgb(0x000000))
                            .child("PRO"),
                    )
                    .child(Button::new("apps").ghost().icon(IconName::Menu))
                    .child(
                        div()
                            .size_8()
                            .rounded_full()
                            .bg(theme.accent)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(IconName::User)
                                    .size_5()
                                    .text_color(theme.accent_foreground),
                            ),
                    ),
            )
    }

    fn render_tab_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .px_6()
            .py_4()
            .child(
                TabBar::new("notebook-tabs")
                    .underline()
                    .selected_index(self.active_tab_ix)
                    .on_click(cx.listener(|this, ix: &usize, _, cx| {
                        this.set_active_tab(*ix, cx);
                    }))
                    .child(Tab::new().label("全部"))
                    .child(Tab::new().label("我的笔记本"))
                    .child(Tab::new().label("精选笔记本")),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        ButtonGroup::new("view-toggle")
                            .child(
                                Button::new("grid-view")
                                    .icon(IconName::LayoutDashboard)
                                    .ghost()
                                    .small()
                                    .selected(self.view_mode == ViewMode::Grid)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_view_mode(ViewMode::Grid, cx);
                                    })),
                            )
                            .child(
                                Button::new("list-view")
                                    .icon(IconName::Menu)
                                    .ghost()
                                    .small()
                                    .selected(self.view_mode == ViewMode::List)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_view_mode(ViewMode::List, cx);
                                    })),
                            ),
                    )
                    .child(
                        Button::new("sort")
                            .ghost()
                            .small()
                            .label("最近")
                            .icon(IconName::ChevronDown),
                    )
                    .child(
                        Button::new("new")
                            .label("新建")
                            .icon(IconName::Plus),
                    ),
            )
    }


    fn render_featured_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let featured: Vec<_> = self
            .notebooks
            .iter()
            .filter(|n| n.is_featured)
            .collect();

        v_flex()
            .w_full()
            .gap_4()
            .px_6()
            .py_6()
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child("精选笔记本"),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.foreground))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("查看全部"),
                            )
                            .child(
                                Icon::new(IconName::ChevronRight)
                                    .size_4()
                                    .text_color(theme.muted_foreground),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .gap_4()
                    .children(featured.iter().map(|notebook| {
                        self.render_featured_card(notebook, cx)
                    })),
            )
    }

    fn render_featured_card(&self, notebook: &Notebook, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .min_w(px(240.))
            .max_w(px(320.))
            .h(px(240.))
            .rounded_lg()
            .overflow_hidden()
            .cursor_pointer()
            .hover(|s| s.shadow_lg())
            .relative()
            .child(
                // Background image
                div()
                    .absolute()
                    .size_full()
                    .bg(notebook.background_color)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(IconName::File)
                            .size(px(64.))
                            .text_color(gpui::white().opacity(0.3)),
                    ),
            )
            .child(
                // Gradient overlay
                div()
                    .absolute()
                    .size_full()
                    .bg(gpui::black())
                    .opacity(0.4),
            )
            .child(
                // Content overlay
                v_flex()
                    .absolute()
                    .size_full()
                    .p_5()
                    .justify_between()
                    .child(
                        // Top section: Category
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .w(px(32.))
                                    .h(px(32.))
                                    .rounded_md()
                                    .bg(gpui::white().opacity(0.2))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::new(IconName::Network)
                                            .size_4()
                                            .text_color(gpui::white()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(gpui::white())
                                    .child(notebook.category.clone()),
                            ),
                    )
                    .child(
                        // Bottom section: Title and metadata
                        v_flex()
                            .w_full()
                            .gap_3()
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(gpui::white())
                                    .line_height(rems(1.4))
                                    .child(notebook.title.clone()),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(gpui::white().opacity(0.9))
                                            .child(format!(
                                                "{} · {} 个来源",
                                                notebook.date, notebook.sources
                                            )),
                                    )
                                    .child(
                                        div()
                                            .p_1p5()
                                            .rounded_full()
                                            .bg(gpui::white().opacity(0.2))
                                            .child(
                                                Icon::new(IconName::Globe)
                                                    .size_4()
                                                    .text_color(gpui::white()),
                                            ),
                                    ),
                            ),
                    ),
            )
    }

    fn render_recent_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let recent: Vec<_> = self
            .notebooks
            .iter()
            .filter(|n| !n.is_featured)
            .collect();

        v_flex()
            .w_full()
            .gap_4()
            .px_6()
            .py_6()
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .text_color(theme.foreground)
                    .child("最近打开过的笔记本"),
            )
            .child(
                h_flex()
                    .w_full()
                    .gap_4()
                    .flex_wrap()
                    .children(recent.iter().map(|notebook| {
                        self.render_recent_card(notebook, cx)
                    })),
            )
    }

    fn render_recent_card(&self, notebook: &Notebook, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .w(px(280.))
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .cursor_pointer()
            .hover(|s| s.shadow_lg().border_color(theme.accent))
            .child(
                v_flex()
                    .w_full()
                    .p_4()
                    .gap_3()
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_start()
                            .child(
                                div()
                                    .size_12()
                                    .rounded_lg()
                                    .bg(notebook.background_color)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::new(IconName::File)
                                            .size_6()
                                            .text_color(gpui::white().opacity(0.9)),
                                    ),
                            )
                            .child(
                                Button::new(format!("menu-{}", notebook.id))
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Ellipsis),
                            ),
                    )
                    .child(
                        div()
                            .text_base()
                            .font_medium()
                            .text_color(theme.foreground)
                            .child(notebook.title.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "{} · {} 个来源",
                                notebook.date, notebook.sources
                            )),
                    ),
            )
    }

    fn render_content(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_h_0()
            .overflow_y_scrollbar()
            .child(self.render_featured_section(cx))
            .child(self.render_recent_section(cx))
    }
}

impl Focusable for NotebookLMStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NotebookLMStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .id("notebook-lm")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(theme.background)
            .child(self.render_top_bar(cx))
            .child(self.render_tab_bar(cx))
            .child(self.render_content(cx))
    }
}

#[cfg(feature = "story")]
impl Story for NotebookLMStory {
    fn title() -> &'static str {
        "NotebookLM"
    }

    fn description() -> &'static str {
        "A clone of Google NotebookLM UI with featured notebooks and recent notebooks"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn paddings() -> Pixels {
        px(0.)
    }
}

// ============================================================================
// Example Wrapper
// ============================================================================

pub struct Example {
    root: Entity<Root>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let story = cx.new(|cx| NotebookLMStory::new(window, cx));
        let root = cx.new(|cx| Root::new(story, window, cx));
        Self { root }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.root.clone()
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        cx.activate(true);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1400.0), px(900.0)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("NotebookLM Example")),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                ..Default::default()
            },
            |window, cx| Example::view(window, cx),
        )
        .unwrap();
    });
}
