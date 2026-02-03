use gpui::{App, Context, Pixels, Size, Window};
use gpui_component::{ActiveTheme as _, text::MarkdownState};
use std::rc::Rc;

use crate::session::SessionState;
use crate::types::ChatItem;
use crate::ui::measure_chat_item_layout_proxy;

use super::ChatSessionView;

impl ChatSessionView {
    pub(super) fn measure_all_items(
        session: &mut SessionState,
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme = cx.theme().clone();

        let sizes: Vec<Size<Pixels>> = session
            .items
            .iter()
            .enumerate()
            .map(|(ix, item)| {
                let markdown_state = session.markdown_states.get(&ix.to_string());
                item.measure(width, &theme, markdown_state, window, cx)
            })
            .collect();

        session.item_sizes = Rc::new(sizes);
        session.measured = true;
        session.last_width = Some(width);
    }

    pub(super) fn remeasure_item(
        session: &mut SessionState,
        ix: usize,
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme = cx.theme().clone();
        let markdown_state = session.markdown_states.get(&ix.to_string());
        let new_size = session.items[ix].measure(width, &theme, markdown_state, window, cx);

        let mut sizes = (*session.item_sizes).clone();
        sizes[ix] = new_size;
        session.item_sizes = Rc::new(sizes);
    }
}

impl ChatItem {
    fn measure(
        &self,
        available_width: Pixels,
        theme: &gpui_component::Theme,
        markdown_state: Option<&MarkdownState>,
        window: &mut Window,
        cx: &mut App,
    ) -> Size<Pixels> {
        match self {
            ChatItem::Message(message) => measure_chat_item_layout_proxy(
                message,
                theme,
                markdown_state,
                available_width,
                window,
                cx,
            ),
        }
    }
}
