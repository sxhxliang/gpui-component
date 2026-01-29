use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    ScrollStrategy, Styled, Window, prelude::FluentBuilder as _, px,
};

use gpui_component::{
    ActiveTheme as _, IconName, Sizable, StyledExt,
    button::{Button, ButtonGroup, ButtonVariants},
    collapsible_list::{CollapsibleListItem, CollapsibleListState, collapsible_list},
    divider::Divider,
    h_flex,
    label::Label,
    list::ListItem,
    v_flex,
};

use crate::section;

pub struct CollapsibleListStory {
    focus_handle: FocusHandle,
    state: Entity<CollapsibleListState>,
}

impl super::Story for CollapsibleListStory {
    fn title() -> &'static str {
        "CollapsibleList"
    }

    fn description() -> &'static str {
        "A virtualized list with collapsible items for high-performance rendering of large datasets."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl CollapsibleListStory {
    pub(crate) fn new(_: &mut Window, cx: &mut App) -> Self {
        // Create a large number of items to demonstrate virtualization
        let items: Vec<CollapsibleListItem> = (0..1000)
            .map(|i| {
                CollapsibleListItem::new(format!("section-{}", i), format!("Section {}", i))
                    .header_height(px(40.))
                    .content_height(px(120.))
                    .expanded(i % 10 == 0) // Expand every 10th item
                    .disabled(i % 50 == 25) // Disable some items
            })
            .collect();

        let state = cx.new(|cx| CollapsibleListState::new(cx).items(items));

        Self {
            focus_handle: cx.focus_handle(),
            state,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Focusable for CollapsibleListStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CollapsibleListStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let item_count = self.state.read(cx).len();

        v_flex()
            .gap_6()
            .size_full()
            .child(
                section("Controls").child(
                    h_flex()
                        .gap_4()
                        .child(
                            ButtonGroup::new("actions")
                                .outline()
                                .compact()
                                .child(Button::new("expand-all").label("Expand All"))
                                .child(Button::new("collapse-all").label("Collapse All"))
                                .on_click(cx.listener(|this, clicks: &Vec<usize>, _, cx| {
                                    if clicks.contains(&0) {
                                        this.state.update(cx, |state, cx| {
                                            state.expand_all(cx);
                                        });
                                    } else if clicks.contains(&1) {
                                        this.state.update(cx, |state, cx| {
                                            state.collapse_all(cx);
                                        });
                                    }
                                })),
                        )
                        .child(Divider::vertical().px_2())
                        .child(
                            h_flex()
                                .gap_2()
                                .child(
                                    Button::new("scroll-top")
                                        .small()
                                        .outline()
                                        .label("Scroll to Top")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.update(cx, |state, _| {
                                                state.scroll_to_item(0, ScrollStrategy::Top);
                                            });
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("scroll-middle")
                                        .small()
                                        .outline()
                                        .label("Scroll to 500")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.update(cx, |state, _| {
                                                state.scroll_to_item(500, ScrollStrategy::Center);
                                            });
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("scroll-bottom")
                                        .small()
                                        .outline()
                                        .label("Scroll to Bottom")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.state.update(cx, |state, _| {
                                                state.scroll_to_item(999, ScrollStrategy::Top);
                                            });
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(Divider::vertical().px_2())
                        .child(Label::new(format!("Total items: {}", item_count)).text_sm()),
                ),
            )
            .child(
                section("Collapsible Virtual List").child(
                    v_flex()
                        .w_full()
                        .flex_1()
                        .min_h_96()
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded_md()
                        .child(
                            collapsible_list(
                                &self.state,
                                // Render header
                                |ix, item, expanded, _window, cx| {
                                    let icon = if expanded {
                                        IconName::ChevronDown
                                    } else {
                                        IconName::ChevronRight
                                    };

                                    ListItem::new(ix)
                                        .py_1()
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    Button::new(format!("toggle-{}", ix))
                                                        .icon(icon)
                                                        .ghost()
                                                        .xsmall(),
                                                )
                                                .child(
                                                    Label::new(item.label.clone())
                                                        .font_semibold()
                                                        .when(item.is_disabled(), |this| {
                                                            this.text_color(cx.theme().muted_foreground)
                                                        }),
                                                )
                                                .when(item.is_disabled(), |this| {
                                                    this.child(
                                                        Label::new("(disabled)")
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground),
                                                    )
                                                }),
                                        )
                                },
                                // Render content
                                |ix, item, _window, cx| {
                                    v_flex()
                                        .p_4()
                                        .pl_8()
                                        .gap_2()
                                        .bg(cx.theme().secondary)
                                        .child(
                                            Label::new(format!("Content for {}", item.label))
                                                .text_sm(),
                                        )
                                        .child(
                                            Label::new(format!(
                                                "This is the expanded content area for item {}. \
                                                You can put any content here including text, images, \
                                                forms, or other UI elements.",
                                                ix
                                            ))
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .child(
                                                    Button::new(format!("action1-{}", ix))
                                                        .label("Action 1")
                                                        .xsmall()
                                                        .outline(),
                                                )
                                                .child(
                                                    Button::new(format!("action2-{}", ix))
                                                        .label("Action 2")
                                                        .xsmall()
                                                        .outline(),
                                                ),
                                        )
                                        .into_any_element()
                                },
                            )
                            .size_full(),
                        ),
                ),
            )
    }
}
