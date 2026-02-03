//! Chat session example with Codex-style UI.
//!
//! Highlights:
//! - Virtual list with measured dynamic heights
//! - Markdown rendering with cached MarkdownState
//! - Simulated streaming updates
//! - Compact tool call display

#![allow(unexpected_cfgs)]

mod bridge;
mod session;
mod types;
mod ui;
mod utils;
mod view;

use gpui::*;
use gpui_component_assets::Assets;

use view::ChatSessionView;

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1200.), px(800.)), cx)),
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
