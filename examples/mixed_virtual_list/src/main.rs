//! Mixed Virtual List Example with Dynamic Height Measurement
//!
//! This example demonstrates a virtual list with various content types:
//! - Collapsible sections
//! - Markdown content (using MarkdownState for accurate measurement)
//! - Images
//! - Plain text
//! - Code blocks
//! - Cards with different heights
//!
//! The heights are dynamically measured using `layout_as_root()` for accurate rendering.
//!
//! ## Key Pattern for Markdown in Virtual Lists
//!
//! Markdown content is rendered asynchronously, which causes height measurement issues.
//! The solution is to use `MarkdownState` (externally managed state):
//!
//! 1. Cache `MarkdownState` at the list level (not inside items)
//! 2. Use the same state for both measurement and rendering
//! 3. State is created once and reused across renders

use std::{collections::HashMap, rc::Rc};

use gpui::*;
use gpui_component::{
    ActiveTheme as _, IconName, Sizable, StyledExt, VirtualListScrollHandle,
    button::{Button, ButtonVariants},
    collapsible::Collapsible,
    h_flex,
    label::Label,
    scroll::{ScrollableElement, ScrollbarAxis},
    tag::Tag,
    text::{MarkdownState, MarkdownView},
    v_flex, v_virtual_list,
};

/// Types of content that can appear in the mixed list
#[derive(Clone)]
enum ContentType {
    /// A collapsible section with header and content
    Collapsible {
        title: String,
        content: String,
        expanded: bool,
    },
    /// Markdown text content
    Markdown { source: String },
    /// A simple text card
    TextCard { title: String, description: String },
    /// An image card (using placeholder for demo)
    ImageCard { title: String, image_url: String },
    /// A code snippet
    CodeBlock { language: String, code: String },
    /// A divider/separator
    Divider,
    /// Stats card with numbers
    StatsCard {
        title: String,
        value: String,
        change: String,
        positive: bool,
    },
}

impl ContentType {
    /// Get an estimated height (fallback when measurement isn't available)
    fn estimated_height(&self) -> Pixels {
        match self {
            ContentType::Collapsible { expanded, .. } => {
                if *expanded {
                    px(180.)
                } else {
                    px(60.)
                }
            }
            ContentType::Markdown { source } => {
                let lines = source.lines().count().max(1);
                px(60. + lines as f32 * 24.)
            }
            ContentType::TextCard { .. } => px(100.),
            ContentType::ImageCard { .. } => px(200.),
            ContentType::CodeBlock { code, .. } => {
                let lines = code.lines().count().max(1);
                px(80. + lines as f32 * 20.)
            }
            ContentType::Divider => px(20.),
            ContentType::StatsCard { .. } => px(100.),
        }
    }

    /// Measure the actual height of this item using layout_as_root()
    #[allow(clippy::too_many_arguments)]
    fn measure(
        &self,
        ix: usize,
        available_width: Pixels,
        background: Hsla,
        secondary: Hsla,
        border: Hsla,
        muted_foreground: Hsla,
        markdown_state: Option<&MarkdownState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Size<Pixels> {
        // Build the layout proxy element (same structure as render)
        let element = build_item_element(
            ix,
            self,
            background,
            secondary,
            border,
            muted_foreground,
            markdown_state,
        );

        // Measure using layout_as_root
        let mut any_element = element.into_any_element();
        let available_space = size(
            AvailableSpace::Definite(available_width),
            AvailableSpace::MinContent,
        );
        any_element.layout_as_root(available_space, window, cx)
    }
}

/// State for the mixed virtual list view
pub struct MixedVirtualListView {
    items: Vec<ContentType>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
    /// Track if we've measured items yet
    measured: bool,
    /// Last known container width for re-measurement
    last_width: Option<Pixels>,
    /// Items that need re-measurement (deferred to render phase)
    pending_remeasure: Vec<usize>,
    /// Cached MarkdownState for each markdown item (key: item index as string)
    /// This is the key to solving async markdown measurement issues!
    markdown_states: HashMap<String, MarkdownState>,
    /// Track if markdown content has been updated (async parsing completed)
    /// When true, we need to re-measure all markdown items
    markdown_needs_remeasure: bool,
}

impl MixedVirtualListView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = Self::generate_sample_data();
        // Use estimated heights initially
        let item_sizes = Self::estimate_sizes(&items);

        // Pre-create MarkdownState for each markdown item
        // This ensures consistent state between measurement and rendering
        let mut markdown_states = HashMap::new();
        for (ix, item) in items.iter().enumerate() {
            if let ContentType::Markdown { source } = item {
                let state = MarkdownState::new(source, cx);
                // Subscribe to state changes to detect when async parsing completes
                cx.observe(state.entity(), |this: &mut Self, _, cx| {
                    // Mark that we need to re-measure markdown items
                    this.markdown_needs_remeasure = true;
                    cx.notify();
                })
                .detach();
                markdown_states.insert(ix.to_string(), state);
            }
        }

        Self {
            items,
            item_sizes: Rc::new(item_sizes),
            scroll_handle: VirtualListScrollHandle::new(),
            measured: false,
            last_width: None,
            pending_remeasure: Vec::new(),
            markdown_states,
            markdown_needs_remeasure: false,
        }
    }

    /// Get estimated sizes (fallback)
    fn estimate_sizes(items: &[ContentType]) -> Vec<Size<Pixels>> {
        items
            .iter()
            .map(|item| Size {
                width: px(0.),
                height: item.estimated_height(),
            })
            .collect()
    }

    /// Measure all items with actual layout
    fn measure_all_items(
        &mut self,
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let background = cx.theme().background;
        let secondary = cx.theme().secondary;
        let border = cx.theme().border;
        let muted_foreground = cx.theme().muted_foreground;

        let sizes: Vec<Size<Pixels>> = self
            .items
            .iter()
            .enumerate()
            .map(|(ix, item)| {
                // Get cached MarkdownState for markdown items
                let markdown_state = self.markdown_states.get(&ix.to_string());
                item.measure(
                    ix,
                    width,
                    background,
                    secondary,
                    border,
                    muted_foreground,
                    markdown_state,
                    window,
                    cx,
                )
            })
            .collect();

        self.item_sizes = Rc::new(sizes);
        self.measured = true;
        self.last_width = Some(width);
    }

    /// Re-measure a single item (used after toggle)
    fn remeasure_item(
        &mut self,
        ix: usize,
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let background = cx.theme().background;
        let secondary = cx.theme().secondary;
        let border = cx.theme().border;
        let muted_foreground = cx.theme().muted_foreground;

        // Get cached MarkdownState for markdown items
        let markdown_state = self.markdown_states.get(&ix.to_string());
        let new_size = self.items[ix].measure(
            ix,
            width,
            background,
            secondary,
            border,
            muted_foreground,
            markdown_state,
            window,
            cx,
        );

        // Clone the sizes, update the one item, and replace
        let mut sizes = (*self.item_sizes).clone();
        sizes[ix] = new_size;
        self.item_sizes = Rc::new(sizes);
    }

    fn generate_sample_data() -> Vec<ContentType> {
        let mut items = Vec::new();

        // Add various content types
        for i in 0i32..100 {
            match i % 7 {
                0 => {
                    items.push(ContentType::Collapsible {
                        title: format!("Section {} - Click to expand", i + 1),
                        content: format!(
                            "This is the expanded content for section {}.\n\
                            It can contain multiple lines of text.\n\
                            The collapsible component is great for FAQs.",
                            i + 1
                        ),
                        expanded: i % 14 == 0,
                    });
                }
                1 => {
                    items.push(ContentType::Markdown {
                        source: format!(
                            "## Markdown Content {}\n\n\
                            This is **bold** and *italic* text. Here's some `inline code`.\n\n\
                            - List item 1\n\
                            - List item 2\n\
                            - List item 3",
                            i + 1
                        ),
                    });
                }
                2 => {
                    items.push(ContentType::TextCard {
                        title: format!("Text Card {}", i + 1),
                        description: "A simple card with title and description. Great for displaying brief information.".to_string(),
                    });
                }
                3 => {
                    items.push(ContentType::ImageCard {
                        title: format!("Image Card {}", i + 1),
                        image_url: "https://pub.lbkrs.com/files/202503/vEnnmgUM6bo362ya/sdk.svg"
                            .to_string(),
                    });
                }
                4 => {
                    items.push(ContentType::CodeBlock {
                        language: "rust".to_string(),
                        code: format!(
                            "fn example_{}() {{\n    println!(\"Hello, World!\");\n    let x = {};\n}}",
                            i + 1,
                            i * 10
                        ),
                    });
                }
                5 => {
                    items.push(ContentType::StatsCard {
                        title: format!("Metric {}", i + 1),
                        value: format!("{}", (i + 1) * 123),
                        change: format!("{}%", (i * 7 % 20) - 10),
                        positive: i % 2 == 0,
                    });
                }
                6 => {
                    items.push(ContentType::Divider);
                }
                _ => {}
            }
        }

        items
    }

    fn toggle_collapsible(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        if let ContentType::Collapsible { expanded, .. } = &mut self.items[index] {
            *expanded = !*expanded;

            // Mark this item for re-measurement (will be done in render phase)
            if !self.pending_remeasure.contains(&index) {
                self.pending_remeasure.push(index);
            }

            cx.notify();
        }
    }
}

impl Render for MixedVirtualListView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Measure items on first render or when width changes significantly
        // We use a fixed width for measurement (could be made dynamic with window size tracking)
        let measure_width = px(800.); // Approximate container width
        if !self.measured || self.last_width != Some(measure_width) {
            self.measure_all_items(measure_width, window, cx);
        }

        // Re-measure markdown items when async parsing completes
        // This is the key to accurate height measurement for async content!
        if self.markdown_needs_remeasure {
            self.markdown_needs_remeasure = false;
            if let Some(width) = self.last_width {
                // Collect indices first to avoid borrow conflict
                let markdown_indices: Vec<usize> = self
                    .markdown_states
                    .keys()
                    .filter_map(|key| key.parse::<usize>().ok())
                    .collect();
                // Re-measure all markdown items
                for ix in markdown_indices {
                    self.remeasure_item(ix, width, window, cx);
                }
            }
        }

        // Process any pending re-measurements (from toggle operations)
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
            .gap_4()
            .p_4()
            // Header
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .child(
                                Label::new("Mixed Virtual List (Dynamic Height)")
                                    .text_2xl()
                                    .font_semibold(),
                            )
                            .child(
                                Label::new("Heights are measured dynamically using layout_as_root()")
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("scroll-top")
                                    .small()
                                    .outline()
                                    .label("Top")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.scroll_handle
                                            .scroll_to_item(0, ScrollStrategy::Top);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("scroll-middle")
                                    .small()
                                    .outline()
                                    .label("Middle")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.scroll_handle
                                            .scroll_to_item(50, ScrollStrategy::Center);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("scroll-bottom")
                                    .small()
                                    .outline()
                                    .label("Bottom")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.scroll_handle.scroll_to_bottom();
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            // Virtual list container
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        v_flex()
                            .id("mixed-list-container")
                            .size_full()
                            .relative()
                            .child(
                                v_virtual_list(
                                    cx.entity().clone(),
                                    "mixed-items",
                                    item_sizes,
                                    move |view, visible_range, _window, cx| {
                                        let background = cx.theme().background;
                                        let secondary = cx.theme().secondary;
                                        let border = cx.theme().border;
                                        let muted_foreground = cx.theme().muted_foreground;

                                        let mut elements = Vec::with_capacity(visible_range.len());

                                        for ix in visible_range {
                                            let item = &view.items[ix];
                                            let is_collapsible =
                                                matches!(item, ContentType::Collapsible { .. });
                                            // Get cached MarkdownState for markdown items
                                            let markdown_state =
                                                view.markdown_states.get(&ix.to_string());
                                            let el = build_item_element(
                                                ix,
                                                item,
                                                background,
                                                secondary,
                                                border,
                                                muted_foreground,
                                                markdown_state,
                                            );

                                            if is_collapsible {
                                                elements.push(
                                                    div()
                                                        .id(ElementId::Name(
                                                            format!("collapsible-click-{}", ix)
                                                                .into(),
                                                        ))
                                                        .cursor_pointer()
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            cx.listener(move |this, _, window, cx| {
                                                                this.toggle_collapsible(
                                                                    ix, window, cx,
                                                                );
                                                            }),
                                                        )
                                                        .child(el),
                                                );
                                            } else {
                                                elements.push(
                                                    div()
                                                        .id(ElementId::Name(
                                                            format!("item-wrapper-{}", ix).into(),
                                                        ))
                                                        .child(el),
                                                );
                                            }
                                        }

                                        elements
                                    },
                                )
                                .track_scroll(&self.scroll_handle)
                                .p_4()
                                .gap_3(),
                            )
                            .scrollbar(&self.scroll_handle, ScrollbarAxis::Vertical),
                    ),
            )
    }
}

/// Build the element for a single item (used both for rendering and measurement)
fn build_item_element(
    ix: usize,
    item: &ContentType,
    background: Hsla,
    secondary: Hsla,
    border: Hsla,
    muted_foreground: Hsla,
    markdown_state: Option<&MarkdownState>,
) -> AnyElement {
    match item {
        ContentType::Collapsible {
            title,
            content,
            expanded,
        } => {
            let expanded = *expanded;
            div()
                .id(ElementId::Name(format!("collapsible-{}", ix).into()))
                .w_full()
                .p_3()
                .bg(background)
                .border_1()
                .border_color(border)
                .rounded_lg()
                .child(
                    Collapsible::new()
                        .gap_2()
                        .open(expanded)
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    Button::new(format!("toggle-{}", ix))
                                        .icon(if expanded {
                                            IconName::ChevronDown
                                        } else {
                                            IconName::ChevronRight
                                        })
                                        .ghost()
                                        .xsmall(),
                                )
                                .child(Label::new(title.clone()).font_semibold()),
                        )
                        .content(
                            div()
                                .pl_6()
                                .pt_2()
                                .child(Label::new(content.clone()).text_sm()),
                        ),
                )
                .into_any_element()
        }
        ContentType::Markdown { source: _ } => {
            // Use MarkdownView with externally managed state for accurate measurement
            // This is the key pattern for virtual lists with async content!
            let content: AnyElement = if let Some(state) = markdown_state {
                MarkdownView::new(state).into_any_element()
            } else {
                // Fallback: should not happen if state is properly cached
                div()
                    .child("Markdown state not found")
                    .into_any_element()
            };

            div()
                .id(ElementId::Name(format!("markdown-{}", ix).into()))
                .w_full()
                .p_4()
                .bg(background)
                .border_1()
                .border_color(border)
                .rounded_lg()
                .child(content)
                .into_any_element()
        }
        ContentType::TextCard { title, description } => div()
            .id(ElementId::Name(format!("text-card-{}", ix).into()))
            .w_full()
            .p_4()
            .bg(background)
            .border_1()
            .border_color(border)
            .rounded_lg()
            .child(
                v_flex()
                    .gap_2()
                    .child(Label::new(title.clone()).text_lg().font_semibold())
                    .child(
                        Label::new(description.clone())
                            .text_sm()
                            .text_color(muted_foreground),
                    ),
            )
            .into_any_element(),
        ContentType::ImageCard { title, image_url } => div()
            .id(ElementId::Name(format!("image-card-{}", ix).into()))
            .w_full()
            .p_4()
            .bg(background)
            .border_1()
            .border_color(border)
            .rounded_lg()
            .child(
                v_flex()
                    .gap_3()
                    .child(Label::new(title.clone()).text_lg().font_semibold())
                    .child(
                        div()
                            .h_32()
                            .w_full()
                            .rounded_md()
                            .bg(secondary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(img(image_url.clone()).h_24()),
                    ),
            )
            .into_any_element(),
        ContentType::CodeBlock { language, code } => div()
            .id(ElementId::Name(format!("code-block-{}", ix).into()))
            .w_full()
            .p_4()
            .bg(background)
            .border_1()
            .border_color(border)
            .rounded_lg()
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Tag::new().small().child(language.clone())),
                    )
                    .child(
                        div()
                            .p_3()
                            .bg(secondary)
                            .rounded_md()
                            .font_family("monospace")
                            .text_sm()
                            .child(code.clone()),
                    ),
            )
            .into_any_element(),
        ContentType::Divider => div()
            .id(ElementId::Name(format!("divider-{}", ix).into()))
            .w_full()
            .py_2()
            .child(div().h(px(1.)).w_full().bg(border))
            .into_any_element(),
        ContentType::StatsCard {
            title,
            value,
            change,
            positive,
        } => {
            let positive = *positive;
            let tag = if positive {
                Tag::primary().small().child(change.clone())
            } else {
                Tag::danger().small().child(change.clone())
            };

            div()
                .id(ElementId::Name(format!("stats-card-{}", ix).into()))
                .w_full()
                .p_4()
                .bg(background)
                .border_1()
                .border_color(border)
                .rounded_lg()
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            Label::new(title.clone())
                                .text_sm()
                                .text_color(muted_foreground),
                        )
                        .child(
                            h_flex()
                                .items_end()
                                .gap_2()
                                .child(Label::new(value.clone()).text_2xl().font_semibold())
                                .child(tag),
                        ),
                )
                .into_any_element()
        }
    }
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|cx| MixedVirtualListView::new(window, cx));
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
