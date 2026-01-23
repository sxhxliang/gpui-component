//! Knowledge Hub - UI example inspired by the provided mobile layout.
//!
//! This component demonstrates:
//!//! - A two-column quick-action tile grid with tinted icon buttons
//! - A recent items list with metadata and action button//! - A primary action button at the bottom of the scroll area

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Colorize as _, Icon, IconName, IndexPath, Root, Sizable, StyledExt as _,
    WindowExt,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
    divider::Divider,
    h_flex,
    list::{List, ListDelegate, ListItem, ListState},
    scroll::ScrollableElement as _,
    v_flex,
};
use gpui_component_assets::Assets;

#[cfg(feature = "story")]
use gpui_component_story::Story;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Clone)]
pub struct QuickTile {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: IconName,
    pub bg: Hsla,
    pub fg: Hsla,
}

#[derive(Clone)]
pub struct RecentItem {
    pub id: &'static str,
    pub title: &'static str,
    pub meta: &'static str,
    pub icon: IconName,
    pub accent: Hsla,
    pub show_play: bool,
}

struct RecentListDelegate {
    items: Vec<RecentItem>,
    selected_index: Option<IndexPath>,
}

impl RecentListDelegate {
    fn new(items: Vec<RecentItem>) -> Self {
        Self {
            items,
            selected_index: None,
        }
    }

    fn render_row(&self, item: &RecentItem, cx: &mut Context<ListState<Self>>) -> impl IntoElement {
        let theme = cx.theme();
        let play_variant = ButtonCustomVariant::new(cx)
            .color(item.accent)
            .foreground(gpui::white())
            .hover(item.accent.opacity(0.9))
            .active(item.accent.opacity(0.8))
            .border(item.accent.opacity(0.9));

        h_flex()
            .w_full()
            .h(px(64.))
            .gap_3()
            .items_center()
            .justify_between()
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .size_9()
                            .rounded_lg()
                            .bg(item.accent.opacity(0.12))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(item.icon.clone())
                                    .size_4()
                                    .text_color(item.accent),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(theme.foreground)
                                    .overflow_x_hidden()
                                    .text_ellipsis()
                                    .child(item.title),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(item.meta),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .when(item.show_play, |this| {
                        this.child(
                            Button::new(format!("play-{}", item.id))
                                .custom(play_variant)
                                .xsmall()
                                .rounded(px(999.))
                                .icon(IconName::Play),
                        )
                    })
                    .child(
                        Button::new(format!("menu-{}", item.id))
                            .ghost()
                            .xsmall()
                            .icon(IconName::EllipsisVertical)
                            .text_color(theme.muted_foreground),
                    ),
            )
    }
}

impl ListDelegate for RecentListDelegate {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        self.items.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let item = self.items.get(ix.row)?;

        Some(ListItem::new(ix).p_0().child(self.render_row(item, cx)))
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
    }
}

// ============================================================================
// Mock Data
// ============================================================================

fn quick_tiles() -> Vec<QuickTile> {
    vec![
        QuickTile {
            id: "audio",
            title: "音频概览",
            icon: IconName::Loader,
            bg: gpui::rgb(0xe8ecff).into(),
            fg: gpui::rgb(0x3347b2).into(),
        },
        QuickTile {
            id: "video",
            title: "视频概览",
            icon: IconName::Play,
            bg: gpui::rgb(0xe7f6ed).into(),
            fg: gpui::rgb(0x1f7a3a).into(),
        },
        QuickTile {
            id: "mindmap",
            title: "思维导图",
            icon: IconName::Map,
            bg: gpui::rgb(0xf6e9f1).into(),
            fg: gpui::rgb(0x8a2b74).into(),
        },
        QuickTile {
            id: "report",
            title: "报告",
            icon: IconName::File,
            bg: gpui::rgb(0xf7f1e2).into(),
            fg: gpui::rgb(0x94692a).into(),
        },
        QuickTile {
            id: "flashcards",
            title: "闪卡",
            icon: IconName::BookOpen,
            bg: gpui::rgb(0xffefe6).into(),
            fg: gpui::rgb(0xb35a2c).into(),
        },
        QuickTile {
            id: "quiz",
            title: "测验",
            icon: IconName::CircleCheck,
            bg: gpui::rgb(0xe6f3ff).into(),
            fg: gpui::rgb(0x2c6a96).into(),
        },
        QuickTile {
            id: "infographic",
            title: "信息图",
            icon: IconName::ChartPie,
            bg: gpui::rgb(0xf3eaff).into(),
            fg: gpui::rgb(0x6a3aa2).into(),
        },
        QuickTile {
            id: "slides",
            title: "演示文稿",
            icon: IconName::Frame,
            bg: gpui::rgb(0xf2f4ff).into(),
            fg: gpui::rgb(0x3b4fb3).into(),
        },
        QuickTile {
            id: "datatable",
            title: "数据表格",
            icon: IconName::LayoutDashboard,
            bg: gpui::rgb(0xeef6ff).into(),
            fg: gpui::rgb(0x2f6fae).into(),
        },
    ]
}

fn recent_items() -> Vec<RecentItem> {
    vec![
        RecentItem {
            id: "rec-1",
            title: "数字孪生操作系统的认知觉醒",
            meta: "1 个来源 · 1 小时前",
            icon: IconName::Cpu,
            accent: gpui::rgb(0x3a57ff).into(),
            show_play: true,
        },
        RecentItem {
            id: "rec-2",
            title: "The Anatomy of Digital Autonomy",
            meta: "1 个来源 · 1 小时前",
            icon: IconName::BookOpen,
            accent: gpui::rgb(0x916c2b).into(),
            show_play: false,
        },
        RecentItem {
            id: "rec-3",
            title: "孪生闪卡",
            meta: "1 个来源 · 1 小时前",
            icon: IconName::File,
            accent: gpui::rgb(0xb35a2c).into(),
            show_play: false,
        },
        RecentItem {
            id: "rec-4",
            title: "数字孪生操作系统与认知智能体架构",
            meta: "1 个来源 · 1 小时前",
            icon: IconName::Network,
            accent: gpui::rgb(0x8a2b74).into(),
            show_play: false,
        },
    ]
}

// ============================================================================
// Main Story Component
// ============================================================================

pub struct KnowledgeHubStory {
    focus_handle: FocusHandle,
    tiles: Vec<QuickTile>,
    recent_list: Entity<ListState<RecentListDelegate>>,
    pressed_tile: Option<&'static str>,
}

impl KnowledgeHubStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let recent_list = cx.new(|cx| {
            ListState::new(RecentListDelegate::new(recent_items()), window, cx).selectable(false)
        });

        Self {
            focus_handle: cx.focus_handle(),
            tiles: quick_tiles(),
            recent_list,
            pressed_tile: None,
        }
    }

    fn render_device(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .w(px(360.))
            .h(px(720.))
            .rounded(px(28.))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border.opacity(0.5))
            .shadow_lg()
            .relative()
            .child(self.render_content(cx))
    }

    fn render_content(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex().size_full().child(
            v_flex()
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .px_4()
                .pt_4()
                .pb_6()
                .gap_5()
                .child(self.render_tiles(cx))
                .child(Divider::horizontal().color(theme.border.opacity(0.4)))
                .child(self.render_recent_list(cx))
                .child(self.render_add_button(cx)),
        )
    }

    fn render_tiles(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .children(self.tiles.chunks(2).map(|row| {
                h_flex()
                    .w_full()
                    .gap_3()
                    .children(row.iter().map(|tile| self.render_tile(tile, cx)))
            }))
    }

    fn render_tile(&self, tile: &QuickTile, cx: &Context<Self>) -> impl IntoElement {
        let edit_variant = ButtonCustomVariant::new(cx)
            .color(tile.fg.opacity(0.12))
            .foreground(tile.fg)
            .hover(tile.fg.opacity(0.18))
            .active(tile.fg.opacity(0.24))
            .border(tile.fg.opacity(0.08));
        let tile_title = tile.title;
        let tile_id = tile.id;
        let is_pressed = self.pressed_tile == Some(tile_id);

        div()
            .h_flex()
            .flex_1()
            .min_w(px(0.))
            .id(SharedString::from(format!("quick-tile-{}", tile_id)))
            .h(px(72.))
            .rounded_lg()
            .bg(tile.bg)
            .border_1()
            .border_color(tile.fg.opacity(0.08))
            .px_3()
            .py_2()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .hover(|this| {
                this.bg(tile.bg.lighten(0.02))
                    .border_color(tile.fg.opacity(0.16))
            })
            .on_click(cx.listener(move |_, _, window, cx| {
                println!("打开：{}", tile_title);
            }))
            .child(
                v_flex()
                    .gap_1()
                    .child(Icon::new(tile.icon.clone()).size_5().text_color(tile.fg))
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(tile.fg)
                            .child(tile.title),
                    ),
            )
            .child(
                Button::new(format!("tile-edit-{}", tile.id))
                    .custom(edit_variant)
                    .xsmall()
                    .rounded(px(999.))
                    .icon(IconName::Settings2),
            )
    }

    fn render_recent_list(&self, cx: &Context<Self>) -> impl IntoElement {
        let items_count = self.recent_list.read(cx).delegate().items.len().max(1);
        let list_height = px(64.0 * items_count as f32);

        List::new(&self.recent_list)
            .scrollbar_visible(false)
            .p_0()
            .w_full()
            .h(list_height)
    }

    fn render_add_button(&self, cx: &Context<Self>) -> impl IntoElement {
        let button_variant = ButtonCustomVariant::new(cx)
            .color(gpui::rgb(0x111111).into())
            .foreground(gpui::white())
            .hover(gpui::rgb(0x1f2937).into())
            .active(gpui::rgb(0x0f172a).into())
            .border(gpui::rgb(0x111111).into());

        h_flex().w_full().justify_center().pt_3().child(
            Button::new("add-note")
                .custom(button_variant)
                .rounded(px(999.))
                .icon(IconName::Plus)
                .label("添加笔记")
                .small(),
        )
    }
}

impl Focusable for KnowledgeHubStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for KnowledgeHubStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .id("knowledge-hub")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(theme.background)
            .items_center()
            .justify_center()
            .child(self.render_device(cx))
    }
}

#[cfg(feature = "story")]
impl Story for KnowledgeHubStory {
    fn title() -> &'static str {
        "Knowledge Hub"
    }

    fn description() -> &'static str {
        "A mobile-style grid and list layout inspired by the provided UI screenshot"
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
        let story = cx.new(|cx| KnowledgeHubStory::new(window, cx));
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
                    size(px(520.0), px(900.0)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("Knowledge Hub Example")),
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
