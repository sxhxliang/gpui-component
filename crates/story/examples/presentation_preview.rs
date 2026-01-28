//! AI Presentation Preview Tool - NotebookLM Style
//!
//! This component demonstrates:
//! - Top navigation bar with editable title and action icons
//! - Main preview area with image display and zoom controls
//! - Feedback buttons (Good/Bad content)
//! - Right sidebar with thumbnail navigation using virtual list
//! - Keyboard navigation support (Arrow keys, +/- zoom)
//! - Click to select and preview different slides

use std::{ops::Range, rc::Rc};

use gpui::*;
use gpui_component::{
    ActiveTheme, Disableable as _, IconName, Sizable, StyledExt as _, VirtualListScrollHandle,
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::{ScrollableElement as _, ScrollbarAxis},
    v_flex, v_virtual_list,
};
use gpui_component_assets::Assets;

// ============================================================================
// Actions
// ============================================================================

actions!(
    presentation,
    [
        NextSlide,
        PrevSlide,
        ZoomIn,
        ZoomOut,
        ResetZoom,
        FirstSlide,
        LastSlide,
    ]
);

const CONTEXT: &str = "PresentationPreview";

fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", NextSlide, Some(CONTEXT)),
        KeyBinding::new("right", NextSlide, Some(CONTEXT)),
        KeyBinding::new("up", PrevSlide, Some(CONTEXT)),
        KeyBinding::new("left", PrevSlide, Some(CONTEXT)),
        KeyBinding::new("=", ZoomIn, Some(CONTEXT)),
        KeyBinding::new("-", ZoomOut, Some(CONTEXT)),
        KeyBinding::new("0", ResetZoom, Some(CONTEXT)),
        KeyBinding::new("home", FirstSlide, Some(CONTEXT)),
        KeyBinding::new("end", LastSlide, Some(CONTEXT)),
    ]);
}

// ============================================================================
// Data Models
// ============================================================================

#[derive(Clone, Debug)]
pub struct Slide {
    pub id: usize,
    pub title: String,
    pub image_url: String,
}

// ============================================================================
// Mock Data
// ============================================================================

fn mock_slides() -> Vec<Slide> {
    let base_path = std::env::current_dir().unwrap().join("images");

    vec![
        Slide {
            id: 1,
            title: "技术架构: 网关-节点框架".to_string(),
            image_url: base_path.join("unnamed.png").to_string_lossy().to_string(),
        },
        Slide {
            id: 2,
            title: "核心模块设计: 系统重构模式".to_string(),
            image_url: base_path.join("unnamed2.png").to_string_lossy().to_string(),
        },
        Slide {
            id: 3,
            title: "技术架构详解".to_string(),
            image_url: base_path.join("unnamed.png").to_string_lossy().to_string(),
        },
        Slide {
            id: 4,
            title: "通讯机制: 轮询 vs 消息".to_string(),
            image_url: base_path.join("unnamed2.png").to_string_lossy().to_string(),
        },
        Slide {
            id: 5,
            title: "系统部署与配置".to_string(),
            image_url: base_path.join("unnamed.png").to_string_lossy().to_string(),
        },
        Slide {
            id: 6,
            title: "调试与故障排查技术".to_string(),
            image_url: base_path.join("unnamed2.png").to_string_lossy().to_string(),
        },
        Slide {
            id: 7,
            title: "实战场景应用".to_string(),
            image_url: base_path.join("unnamed.png").to_string_lossy().to_string(),
        },
    ]
}

// ============================================================================
// Thumbnail Item Component
// ============================================================================

struct ThumbnailItem {
    slide: Slide,
    is_selected: bool,
}

impl ThumbnailItem {
    fn new(slide: Slide, is_selected: bool) -> Self {
        Self { slide, is_selected }
    }

    fn render(&self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();

        let (bg_color, border_color, number_bg, number_fg) = if self.is_selected {
            (
                theme.accent.opacity(0.1),
                theme.accent,
                theme.accent,
                gpui::white(),
            )
        } else {
            (
                theme.transparent,
                theme.border,
                theme.muted,
                theme.muted_foreground,
            )
        };

        h_flex()
            .w_full()
            .h(px(80.))
            .px_3()
            .py_2()
            .gap_3()
            .items_center()
            .cursor_pointer()
            .rounded_md()
            .mx_2()
            .bg(bg_color)
            .hover(|s| s.bg(theme.secondary))
            .child(
                // Slide number badge
                div()
                    .w(px(28.))
                    .h(px(28.))
                    .rounded_md()
                    .bg(number_bg)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .font_semibold()
                    .text_color(number_fg)
                    .child(format!("{}", self.slide.id)),
            )
            .child(
                // Thumbnail preview
                div()
                    .flex_1()
                    .h_full()
                    .rounded_md()
                    .border_2()
                    .border_color(border_color)
                    .overflow_hidden()
                    .shadow_sm()
                    .child(
                        img(std::path::PathBuf::from(&self.slide.image_url))
                            .size_full()
                            .object_fit(ObjectFit::Cover),
                    ),
            )
    }
}

// ============================================================================
// Main Component
// ============================================================================

pub struct PresentationPreview {
    focus_handle: FocusHandle,
    slides: Vec<Slide>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    selected_index: usize,
    zoom_level: f32,
    scroll_handle: VirtualListScrollHandle,
    title: SharedString,
    subtitle: SharedString,
    feedback: Option<bool>, // None = no feedback, Some(true) = good, Some(false) = bad
}

impl PresentationPreview {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let slides = mock_slides();
        let item_count = slides.len();
        let item_sizes = (0..item_count)
            .map(|_| size(px(250.), px(80.)))
            .collect::<Vec<_>>();

        Self {
            focus_handle: cx.focus_handle(),
            slides,
            item_sizes: Rc::new(item_sizes),
            selected_index: 0,
            zoom_level: 1.0,
            scroll_handle: VirtualListScrollHandle::new(),
            title: "Clawdbot Agentic Infrastructure".into(),
            subtitle: "基于 1 个来源".into(),
            feedback: None,
        }
    }

    fn select_slide(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.slides.len() {
            self.selected_index = index;
            // Scroll to the selected item
            self.scroll_handle
                .scroll_to_item(index, ScrollStrategy::Center);
            cx.notify();
        }
    }

    fn next_slide(&mut self, cx: &mut Context<Self>) {
        let new_index = (self.selected_index + 1).min(self.slides.len() - 1);
        self.select_slide(new_index, cx);
    }

    fn prev_slide(&mut self, cx: &mut Context<Self>) {
        let new_index = self.selected_index.saturating_sub(1);
        self.select_slide(new_index, cx);
    }

    fn zoom_in(&mut self, cx: &mut Context<Self>) {
        self.zoom_level = (self.zoom_level + 0.1).min(2.0);
        cx.notify();
    }

    fn zoom_out(&mut self, cx: &mut Context<Self>) {
        self.zoom_level = (self.zoom_level - 0.1).max(0.5);
        cx.notify();
    }

    fn reset_zoom(&mut self, cx: &mut Context<Self>) {
        self.zoom_level = 1.0;
        cx.notify();
    }

    fn set_feedback(&mut self, good: bool, cx: &mut Context<Self>) {
        self.feedback = Some(good);
        cx.notify();
    }

    fn render_top_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        h_flex()
            .w_full()
            .h(px(56.))
            .px_6()
            .items_center()
            .justify_between()
            .bg(theme.background)
            .border_b_1()
            .border_color(theme.border)
            .child(
                // Left: Title section with icon
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .w(px(36.))
                            .h(px(36.))
                            .rounded_lg()
                            .bg(theme.accent.opacity(0.1))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                gpui_component::Icon::new(IconName::GalleryVerticalEnd)
                                    .size_5()
                                    .text_color(theme.accent),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_base()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(theme.accent))
                                    .child(self.title.clone()),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child(self.subtitle.clone()),
                                    )
                                    .child(
                                        div()
                                            .px_1p5()
                                            .py_0p5()
                                            .rounded(px(4.))
                                            .bg(theme.success.opacity(0.1))
                                            .text_xs()
                                            .text_color(theme.success)
                                            .child("已生成"),
                                    ),
                            ),
                    ),
            )
            .child(
                // Right: Action icons with tooltips
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        Button::new("share")
                            .ghost()
                            .icon(IconName::ExternalLink)
                            .tooltip("分享"),
                    )
                    .child(
                        Button::new("download")
                            .ghost()
                            .icon(IconName::Inbox)
                            .tooltip("下载"),
                    )
                    .child(div().w(px(1.)).h(px(24.)).bg(theme.border).mx_2())
                    .child(
                        Button::new("play")
                            .ghost()
                            .icon(IconName::Play)
                            .tooltip("播放幻灯片"),
                    )
                    .child(
                        Button::new("fullscreen")
                            .ghost()
                            .icon(IconName::Maximize)
                            .tooltip("全屏"),
                    )
                    .child(div().w(px(1.)).h(px(24.)).bg(theme.border).mx_2())
                    .child(
                        Button::new("close")
                            .ghost()
                            .icon(IconName::Close)
                            .tooltip("关闭"),
                    ),
            )
    }

    fn render_main_preview(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let current_slide = &self.slides[self.selected_index];
        let is_first = self.selected_index == 0;
        let is_last = self.selected_index == self.slides.len() - 1;

        v_flex()
            .flex_1()
            .bg(theme.secondary.opacity(0.3))
            .child(
                // Main content area
                div()
                    .flex_1()
                    .p_8()
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_h_0()
                    .child(
                        // Outer container with relative positioning for zoom controls
                        div()
                            .relative()
                            .w(px(900.))
                            .h(px(550.))
                            .child(
                                // Image container
                                div()
                                    .size_full()
                                    .bg(gpui::white())
                                    .rounded_lg()
                                    .shadow_2xl()
                                    .overflow_scrollbar()
                                    .child(
                                        div()
                                            .w(px(900. * self.zoom_level))
                                            .h(px(550. * self.zoom_level))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                img(std::path::PathBuf::from(
                                                    &current_slide.image_url,
                                                ))
                                                .size_full()
                                                .object_fit(ObjectFit::Contain)
                                                .with_fallback(|| {
                                                    div()
                                                        .size_full()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .bg(gpui::rgb(0xf5f5f5))
                                                        .child(
                                                            v_flex()
                                                                .gap_2()
                                                                .items_center()
                                                                .child(
                                                                    gpui_component::Icon::new(
                                                                        IconName::File,
                                                                    )
                                                                    .size_12()
                                                                    .text_color(gpui::rgb(0xcccccc)),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_color(gpui::rgb(
                                                                            0x999999,
                                                                        ))
                                                                        .child("图片加载失败"),
                                                                ),
                                                        )
                                                        .into_any_element()
                                                }),
                                            ),
                                    ),
                            )
                            .child(
                                // Zoom controls (bottom right corner)
                                div().absolute().bottom_3().right_3().child(
                                    h_flex()
                                        .gap_1()
                                        .p_1()
                                        .rounded_lg()
                                        .bg(theme.background.opacity(0.95))
                                        .border_1()
                                        .border_color(theme.border)
                                        .shadow_md()
                                        .child(
                                            Button::new("zoom-out")
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Minus)
                                                .tooltip("缩小 (-)")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.zoom_out(cx);
                                                })),
                                        )
                                        .child(
                                            div()
                                                .w(px(48.))
                                                .text_center()
                                                .text_xs()
                                                .font_medium()
                                                .text_color(theme.muted_foreground)
                                                .child(format!(
                                                    "{}%",
                                                    (self.zoom_level * 100.0) as i32
                                                )),
                                        )
                                        .child(
                                            Button::new("zoom-in")
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Plus)
                                                .tooltip("放大 (+)")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.zoom_in(cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("zoom-reset")
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Undo)
                                                .tooltip("重置缩放 (0)")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.reset_zoom(cx);
                                                })),
                                        ),
                                ),
                            ),
                    ),
            )
            .child(
                // Bottom control bar
                h_flex()
                    .w_full()
                    .h(px(56.))
                    .px_6()
                    .items_center()
                    .justify_between()
                    .bg(theme.background)
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        // Left: Feedback buttons
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("这个内容有帮助吗？"),
                            )
                            .child({
                                let btn = Button::new("feedback-good")
                                    .ghost()
                                    .icon(IconName::ThumbsUp)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_feedback(true, cx);
                                    }));
                                if self.feedback == Some(true) {
                                    btn.bg(theme.success.opacity(0.1))
                                } else {
                                    btn
                                }
                            })
                            .child({
                                let btn = Button::new("feedback-bad")
                                    .ghost()
                                    .icon(IconName::ThumbsDown)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_feedback(false, cx);
                                    }));
                                if self.feedback == Some(false) {
                                    btn.bg(theme.danger.opacity(0.1))
                                } else {
                                    btn
                                }
                            }),
                    )
                    .child(
                        // Center: Slide indicator
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(theme.foreground)
                                    .child(format!(
                                        "{} / {}",
                                        self.selected_index + 1,
                                        self.slides.len()
                                    )),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .max_w(px(300.))
                                    .truncate()
                                    .child(current_slide.title.clone()),
                            ),
                    )
                    .child(
                        // Right: Navigation buttons
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Button::new("prev")
                                    .ghost()
                                    .icon(IconName::ChevronLeft)
                                    .tooltip("上一张 (←)")
                                    .disabled(is_first)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.prev_slide(cx);
                                    })),
                            )
                            .child(
                                Button::new("next")
                                    .ghost()
                                    .icon(IconName::ChevronRight)
                                    .tooltip("下一张 (→)")
                                    .disabled(is_last)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.next_slide(cx);
                                    })),
                            ),
                    ),
            )
    }

    fn render_sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .w(px(300.))
            .h_full()
            .border_l_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                v_flex()
                    .size_full()
                    .child(
                        // Sidebar header
                        h_flex()
                            .w_full()
                            .h(px(56.))
                            .px_4()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(theme.border)
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_semibold()
                                            .text_color(theme.foreground)
                                            .child("幻灯片"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .py_0p5()
                                            .rounded(px(10.))
                                            .bg(theme.muted)
                                            .text_xs()
                                            .font_medium()
                                            .text_color(theme.muted_foreground)
                                            .child(format!("{}", self.slides.len())),
                                    ),
                            )
                            .child(
                                Button::new("grid-view")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::LayoutDashboard)
                                    .tooltip("网格视图"),
                            ),
                    )
                    .child(
                        // Thumbnail list with virtual scrolling
                        div()
                            .flex_1()
                            .min_h_0()
                            .relative()
                            .py_2()
                            .child(
                                v_virtual_list(
                                    cx.entity().clone(),
                                    "slides",
                                    self.item_sizes.clone(),
                                    move |this: &mut PresentationPreview,
                                          visible_range: Range<usize>,
                                          _: &mut Window,
                                          cx: &mut Context<PresentationPreview>| {
                                        visible_range
                                            .map(|ix| {
                                                let slide = this.slides[ix].clone();
                                                let is_selected = ix == this.selected_index;
                                                let thumbnail =
                                                    ThumbnailItem::new(slide, is_selected);

                                                div()
                                                    .id(("slide", ix))
                                                    .on_click(cx.listener(
                                                        move |this, _, _, cx| {
                                                            this.select_slide(ix, cx);
                                                        },
                                                    ))
                                                    .child(thumbnail.render(cx))
                                            })
                                            .collect()
                                    },
                                )
                                .track_scroll(&self.scroll_handle)
                                .size_full(),
                            )
                            .scrollbar(&self.scroll_handle, ScrollbarAxis::Vertical),
                    )
                    .child(
                        // Sidebar footer with keyboard hints
                        div()
                            .w_full()
                            .px_4()
                            .py_3()
                            .border_t_1()
                            .border_color(theme.border)
                            .child(
                                h_flex()
                                    .gap_4()
                                    .justify_center()
                                    .child(
                                        h_flex()
                                            .gap_1()
                                            .items_center()
                                            .child(
                                                div()
                                                    .px_1p5()
                                                    .py_0p5()
                                                    .rounded(px(4.))
                                                    .bg(theme.muted)
                                                    .text_xs()
                                                    .font_medium()
                                                    .text_color(theme.muted_foreground)
                                                    .child("←→"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.muted_foreground)
                                                    .child("切换"),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_1()
                                            .items_center()
                                            .child(
                                                div()
                                                    .px_1p5()
                                                    .py_0p5()
                                                    .rounded(px(4.))
                                                    .bg(theme.muted)
                                                    .text_xs()
                                                    .font_medium()
                                                    .text_color(theme.muted_foreground)
                                                    .child("+/-"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.muted_foreground)
                                                    .child("缩放"),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}

impl Focusable for PresentationPreview {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PresentationPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .id("presentation-preview")
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &NextSlide, _, cx| {
                this.next_slide(cx);
            }))
            .on_action(cx.listener(|this, _: &PrevSlide, _, cx| {
                this.prev_slide(cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomIn, _, cx| {
                this.zoom_in(cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOut, _, cx| {
                this.zoom_out(cx);
            }))
            .on_action(cx.listener(|this, _: &ResetZoom, _, cx| {
                this.reset_zoom(cx);
            }))
            .on_action(cx.listener(|this, _: &FirstSlide, _, cx| {
                this.select_slide(0, cx);
            }))
            .on_action(cx.listener(|this, _: &LastSlide, _, cx| {
                let last = this.slides.len().saturating_sub(1);
                this.select_slide(last, cx);
            }))
            .size_full()
            .bg(theme.background)
            .child(self.render_top_bar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_main_preview(cx))
                    .child(self.render_sidebar(cx)),
            )
    }
}

// ============================================================================
// Example Wrapper
// ============================================================================

pub struct Example {
    presentation: Entity<PresentationPreview>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let presentation = cx.new(|cx| PresentationPreview::new(window, cx));
        Self { presentation }
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.presentation.clone()
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        init(cx);
        cx.activate(true);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1400.), px(900.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("AI Presentation Preview - NotebookLM Style".into()),
                    ..Default::default()
                }),
                window_background: WindowBackgroundAppearance::default(),
                focus: true,
                show: true,
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Example::new(window, cx)),
        )
        .unwrap();
    });
}
