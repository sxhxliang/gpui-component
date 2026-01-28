use gpui::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, NoAction, ParentElement,
    Render, StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::section;
use gpui_component::{
    Anchor, button::*, divider::Divider, input::*, popover::Popover, spinner::Spinner, *,
};
use gpui_component::popover::PopoverState;

pub fn init(_: &mut App) {}

pub struct InputGroupStory {
    search_input: Entity<InputState>,
    url_input: Entity<InputState>,
    username_input: Entity<InputState>,
    chat_input: Entity<InputState>,
}

impl super::Story for InputGroupStory {
    fn title() -> &'static str {
        "InputGroup"
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl InputGroupStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            search_input: cx.new(|cx| InputState::new(window, cx).placeholder("Search...")),
            url_input: cx.new(|cx| InputState::new(window, cx).placeholder("example.com")),
            username_input: cx.new(|cx| InputState::new(window, cx).placeholder("@username")),
            chat_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Ask, Search or Chat...")
                    .multi_line(true)
            }),
        }
    }
}

impl Render for InputGroupStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("input-group-story")
            .size_full()
            .overflow_y_scroll()
            .gap_6()
            .child(
                section("Basic Search with Icon and Results").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(InputGroupAddon::new().child(Icon::new(IconName::Search).small()))
                        .child(InputGroupInput::new(&self.search_input))
                        .child(
                            InputGroupAddon::new()
                                .inline_end()
                                .child(InputGroupText::new().child("12 results")),
                        ),
                ),
            )
            .child(
                section("URL Input with Protocol Prefix").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(
                            InputGroupAddon::new().child(InputGroupText::new().child("https://")),
                        )
                        .child(InputGroupInput::new(&self.url_input)),
                ),
            )
            .child(
                section("Username with Validation Icon").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(InputGroupInput::new(&self.username_input))
                        .child(
                            InputGroupAddon::new().inline_end().child(
                                div()
                                    .size_4()
                                    .rounded_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(cx.theme().primary)
                                    .child(Icon::new(IconName::Check).text_sm()),
                            ),
                        ),
                ),
            )
            .child(
                section("Search with Clear Button").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(InputGroupAddon::new().child(Icon::new(IconName::Search).small()))
                        .child(InputGroupInput::new(&self.search_input))
                        .child(
                            InputGroupAddon::new().inline_end().child(
                                Button::new("clear-btn")
                                    .xsmall()
                                    .ghost()
                                    .icon(IconName::Close)
                                    .rounded_full(),
                            ),
                        ),
                ),
            )
            .child(
                section("Search with Loading State").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(InputGroupAddon::new().child(Icon::new(IconName::Search).small()))
                        .child(InputGroupInput::new(&self.search_input))
                        .child(
                            InputGroupAddon::new()
                                .inline_end()
                                .child(Spinner::new().small()),
                        ),
                ),
            )
            .child(
                section("Multiple Icons").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(
                            InputGroupAddon::new()
                                .child(Icon::new(IconName::Search).small())
                                .child(Icon::new(IconName::Settings).small()),
                        )
                        .child(InputGroupInput::new(&self.search_input)),
                ),
            )
            .child(
                section("With Action Button").child(
                    InputGroup::new()
                        .max_w_96()
                        .child(InputGroupInput::new(&self.search_input))
                        .child(
                            InputGroupAddon::new().inline_end().child(
                                Button::new("send-btn")
                                    .xsmall()
                                    .primary()
                                    .label("Send")
                                    .rounded_full(),
                            ),
                        ),
                ),
            )
            .child(
                section("Disabled State").child(
                    InputGroup::new()
                        .max_w_96()
                        .disabled(true)
                        .child(InputGroupAddon::new().child(Icon::new(IconName::Minus).small()))
                        .child(InputGroupInput::new(&self.search_input)),
                ),
            )
            .child(
                section("Invalid/Error State").child(
                    InputGroup::new()
                        .max_w_96()
                        .border_color(gpui::red())
                        .child(
                            InputGroupAddon::new()
                                .child(Icon::new(IconName::TriangleAlert).small()),
                        )
                        .child(InputGroupInput::new(&self.search_input))
                        .child(
                            InputGroupAddon::new()
                                .inline_end()
                                .child(InputGroupText::new().child("Error!")),
                        ),
                ),
            )
            .child(
                section("Chat Input with Toolbar").child(
                    InputGroup::new()
                        .max_w_96()
                        .flex_col()
                        .h_auto()
                        .child(InputGroupTextarea::new(&self.chat_input).flex_1())
                        .child(
                            InputGroupAddon::new()
                                .block_end()
                                .child(
                                    Button::new("attach-btn")
                                        .xsmall()
                                        .ghost()
                                        .icon(IconName::Plus)
                                        .rounded_full(),
                                )
                                .child(
                                    DropdownButton::new("model-dropdown")
                                        .xsmall()
                                        .ghost()
                                        .button(Button::new("model-btn").label("GPT-4o"))
                                        .compact()
                                        .dropdown_menu(|this, _, _| {
                                            this.menu("GPT-4o", Box::new(NoAction))
                                                .menu("GPT-4o Mini", Box::new(NoAction))
                                                .menu("GPT-3.5 Turbo", Box::new(NoAction))
                                                .separator()
                                                .menu("Claude Sonnet", Box::new(NoAction))
                                                .menu("Claude Opus", Box::new(NoAction))
                                        }),
                                )
                                .child(
                                    Popover::new("settings-popover")
                                        .anchor(Anchor::TopLeft)
                                        .trigger(
                                            Button::new("settings-btn")
                                                .xsmall()
                                                .ghost()
                                                .icon(IconName::Settings),
                                        )
                                        .w(px(200.))
                                        .gap_2()
                                        .text_sm()
                                        .child("Chat Settings")
                                        .child(Divider::horizontal())
                                        .child(
                                            h_flex()
                                                .justify_between()
                                                .child("Temperature")
                                                .child("0.7"),
                                        )
                                        .child(
                                            h_flex()
                                                .justify_between()
                                                .child("Max Tokens")
                                                .child("4096"),
                                        )
                                        .child(
                                            h_flex().justify_between().child("Stream").child("On"),
                                        ),
                                )
                                .child(div().flex_1())
                                .child(InputGroupText::new().child("52% used"))
                                .child(div().h_4().w_px().bg(cx.theme().border))
                                .child(
                                    Button::new("send-btn")
                                        .xsmall()
                                        .primary()
                                        .icon(IconName::ArrowUp)
                                        .rounded_full()
                                        .disabled(true),
                                ),
                        ),
                ),
            )
            .child(
                section("Chat Input with DropdownButton").child(
                    InputGroup::new()
                        .max_w_96()
                        .flex_col()
                        .h_auto()
                        .child(InputGroupTextarea::new(&self.chat_input).flex_1())
                        .child(
                            InputGroupAddon::new()
                                .block_end()
                                .child(
                                    DropdownButton::new("attach-dropdown")
                                        .xsmall()
                                        .ghost()
                                        .button(
                                            Button::new("attach-btn")
                                                .icon(IconName::Plus)
                                                .rounded_full(),
                                        )
                                        .compact()
                                        .dropdown_menu(|this, _, _| {
                                            this.menu_with_icon(
                                                "Upload File",
                                                IconName::File,
                                                Box::new(NoAction),
                                            )
                                            .menu_with_icon(
                                                "Upload Image",
                                                IconName::Frame,
                                                Box::new(NoAction),
                                            )
                                            .menu_with_icon(
                                                "Add Link",
                                                IconName::ExternalLink,
                                                Box::new(NoAction),
                                            )
                                            .separator()
                                            .menu_with_icon(
                                                "Code Block",
                                                IconName::SquareTerminal,
                                                Box::new(NoAction),
                                            )
                                        }),
                                )
                                .child(
                                    DropdownButton::new("action-dropdown")
                                        .xsmall()
                                        .outline()
                                        .button(Button::new("action-btn").label("Actions"))
                                        .compact()
                                        .dropdown_menu(|this, _, _| {
                                            this.menu_with_icon(
                                                "Generate",
                                                IconName::Star,
                                                Box::new(NoAction),
                                            )
                                            .menu_with_icon(
                                                "Regenerate",
                                                IconName::Redo,
                                                Box::new(NoAction),
                                            )
                                            .menu_with_icon(
                                                "Copy",
                                                IconName::Copy,
                                                Box::new(NoAction),
                                            )
                                            .separator()
                                            .menu_with_icon(
                                                "Clear Chat",
                                                IconName::Delete,
                                                Box::new(NoAction),
                                            )
                                        }),
                                )
                                .child(div().flex_1())
                                .child(
                                    Button::new("send-btn")
                                        .xsmall()
                                        .primary()
                                        .icon(IconName::ArrowUp)
                                        .rounded_full(),
                                ),
                        ),
                ),
            )
            .child(
                section("Chat Input with Popover").child(
                    InputGroup::new()
                        .max_w_96()
                        .flex_col()
                        .h_auto()
                        .child(InputGroupTextarea::new(&self.chat_input).flex_1())
                        .child(
                            InputGroupAddon::new()
                                .block_end()
                                .child(
                                    Popover::new("emoji-popover")
                                        .anchor(Anchor::BottomLeft)
                                        .trigger(
                                            Button::new("emoji-btn")
                                                .xsmall()
                                                .ghost()
                                                .icon(IconName::Heart),
                                        )
                                        .w(px(180.))
                                        .content(|_state, _, cx| {
                                            div()
                                                .flex()
                                                .flex_wrap()
                                                .gap_1()
                                                .child(emoji_button("e1", "😀", cx))
                                                .child(emoji_button("e2", "😂", cx))
                                                .child(emoji_button("e3", "😍", cx))
                                                .child(emoji_button("e4", "🤔", cx))
                                                .child(emoji_button("e5", "👍", cx))
                                                .child(emoji_button("e6", "👏", cx))
                                                .child(emoji_button("e7", "🎉", cx))
                                                .child(emoji_button("e8", "❤️", cx))
                                        }),
                                )
                                .child(
                                    Popover::new("mention-popover")
                                        .anchor(Anchor::BottomLeft)
                                        .trigger(
                                            Button::new("mention-btn")
                                                .xsmall()
                                                .ghost()
                                                .icon(IconName::User),
                                        )
                                        .w(px(160.))
                                        .content(|_state, _, cx| {
                                            v_flex()
                                                .gap_1()
                                                .text_sm()
                                                .child("Mention")
                                                .child(Divider::horizontal())
                                                .child(mention_button("m1", "@alice", cx))
                                                .child(mention_button("m2", "@bob", cx))
                                                .child(mention_button("m3", "@charlie", cx))
                                        }),
                                )
                                .child(div().flex_1())
                                .child(
                                    Popover::new("format-popover")
                                        .anchor(Anchor::TopRight)
                                        .trigger(
                                            Button::new("format-btn")
                                                .xsmall()
                                                .ghost()
                                                .icon(IconName::ALargeSmall),
                                        )
                                        .content(|_state, _, cx| {
                                            h_flex()
                                                .gap_1()
                                                .child(format_button("bold", "B", cx))
                                                .child(format_button("italic", "I", cx))
                                                .child(format_button("underline", "U", cx))
                                                .child(format_button("strikethrough", "S", cx))
                                        }),
                                )
                                .child(
                                    Button::new("send-btn")
                                        .xsmall()
                                        .primary()
                                        .icon(IconName::ArrowUp)
                                        .rounded_full(),
                                ),
                        ),
                ),
            )
    }
}

fn emoji_button(
    id: &'static str,
    emoji: &'static str,
    cx: &mut Context<PopoverState>,
) -> Button {
    Button::new(id)
        .xsmall()
        .ghost()
        .label(emoji)
        .on_click(cx.listener(|state, _, window, cx| {
            state.dismiss(window, cx);
        }))
}

fn mention_button(
    id: &'static str,
    label: &'static str,
    cx: &mut Context<PopoverState>,
) -> Button {
    Button::new(id)
        .xsmall()
        .ghost()
        .w_full()
        .justify_start()
        .label(label)
        .on_click(cx.listener(|state, _, window, cx| {
            state.dismiss(window, cx);
        }))
}

fn format_button(
    id: &'static str,
    label: &'static str,
    cx: &mut Context<PopoverState>,
) -> Button {
    Button::new(id)
        .xsmall()
        .ghost()
        .label(label)
        .on_click(cx.listener(|state, _, window, cx| {
            state.dismiss(window, cx);
        }))
}
