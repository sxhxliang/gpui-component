//! A Typora-style WYSIWYG Markdown Editor built with GPUI Component.
//!
//! Features:
//! - Real-time markdown rendering
//! - Block-level syntax reveal: click on block to edit source
//! - Support for headings, lists, code blocks, tables, blockquotes, etc.

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::ActiveTheme;
use gpui_component::*;
use gpui_component_assets::Assets;

const SAMPLE_MARKDOWN: &str = r#"# Welcome to WYSIWYG Markdown Editor

This is a **Typora-style** markdown editor built with GPUI.

## Features

- **Real-time rendering**: See your markdown rendered instantly
- *Inline syntax reveal*: Click on formatted text to see the markdown
- ~~Strikethrough~~ support
- `Inline code` rendering

### Code Blocks

```rust
fn main() {
    println!("Hello, WYSIWYG!");
}
```

### Lists

1. First ordered item
2. Second item
3. Third item

### Blockquotes

> This is a blockquote.
> It can span multiple lines.

### Tables

| Feature | Status |
|---------|--------|
| Headings | Done |
| Lists | Done |
| Code | Done |

### Links

Check out [GPUI Component](https://github.com/longbridge/gpui-component) for more!

---

*Start editing to see the magic happen!*
"#;

/// Represents the span of a block in source text (byte offsets)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BlockSpan {
    start: usize,
    end: usize,
}

/// Parsed block information
#[derive(Debug, Clone)]
struct ParsedBlock {
    span: BlockSpan,
    block_type: BlockType,
    source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BlockType {
    Paragraph,
    Heading(u8),
    CodeBlock { lang: Option<String> },
    List { ordered: bool },
    Blockquote,
    Table,
    ThematicBreak,
}

/// Parse markdown source into blocks with spans
fn parse_blocks(source: &str) -> Vec<ParsedBlock> {
    use markdown::{mdast::Node, to_mdast, ParseOptions};

    let mut blocks = Vec::new();

    if let Ok(Node::Root(root)) = to_mdast(source, &ParseOptions::gfm()) {
        for node in &root.children {
            if let Some(block) = node_to_block(node, source) {
                blocks.push(block);
            }
        }
    }

    blocks
}

fn node_to_block(node: &markdown::mdast::Node, source: &str) -> Option<ParsedBlock> {
    use markdown::mdast::Node;

    let pos = node.position()?;
    let span = BlockSpan {
        start: pos.start.offset,
        end: pos.end.offset,
    };
    let block_source = source.get(span.start..span.end).unwrap_or("").to_string();

    let block_type = match node {
        Node::Paragraph(_) => BlockType::Paragraph,
        Node::Heading(h) => BlockType::Heading(h.depth),
        Node::Code(code) => BlockType::CodeBlock {
            lang: code.lang.clone(),
        },
        Node::List(list) => BlockType::List {
            ordered: list.ordered,
        },
        Node::Blockquote(_) => BlockType::Blockquote,
        Node::Table(_) => BlockType::Table,
        Node::ThematicBreak(_) => BlockType::ThematicBreak,
        _ => return None,
    };

    Some(ParsedBlock {
        span,
        block_type,
        source: block_source,
    })
}

/// Action to start editing a block
#[derive(Clone, PartialEq)]
struct EditBlock(usize);

impl EventEmitter<EditBlock> for WysiwygEditor {}

/// Action to finish editing
#[derive(Clone, PartialEq)]
struct FinishEdit;

impl EventEmitter<FinishEdit> for WysiwygEditor {}

/// The main WYSIWYG Editor view
pub struct WysiwygEditor {
    /// Full source text
    source: String,
    /// Parsed blocks
    blocks: Vec<ParsedBlock>,
    /// Currently editing block index
    editing_block: Option<usize>,
    /// Input state for block editing
    block_input_state: Entity<InputState>,
    /// Subscriptions
    _subscriptions: Vec<Subscription>,
}

impl WysiwygEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let block_input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .soft_wrap(true)
                .placeholder("Edit block...")
        });

        // Subscribe to input events
        let _subscriptions = vec![cx.subscribe(&block_input_state, |this, _, event: &InputEvent, cx| {
            match event {
                InputEvent::Blur => {
                    this.finish_editing(cx);
                }
                _ => {}
            }
        })];

        let source = SAMPLE_MARKDOWN.to_string();
        let blocks = parse_blocks(&source);

        Self {
            source,
            blocks,
            editing_block: None,
            block_input_state,
            _subscriptions,
        }
    }

    fn start_editing(&mut self, block_index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(block) = self.blocks.get(block_index) {
            self.editing_block = Some(block_index);
            self.block_input_state.update(cx, |state, cx| {
                state.set_value(block.source.clone(), window, cx);
                // Focus the input
                state.focus(window, cx);
            });
            cx.notify();
        }
    }

    fn finish_editing(&mut self, cx: &mut Context<Self>) {
        if let Some(block_index) = self.editing_block.take() {
            let new_block_source = self.block_input_state.read(cx).value().to_string();

            if let Some(block) = self.blocks.get(block_index) {
                // Rebuild the full source by replacing the block
                let before = &self.source[..block.span.start];
                let after = &self.source[block.span.end..];
                self.source = format!("{}{}{}", before, new_block_source, after);

                // Re-parse blocks
                self.blocks = parse_blocks(&self.source);
            }

            cx.notify();
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for WysiwygEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let blocks = self.blocks.clone();
        let editing_block = self.editing_block;
        let block_count = blocks.len();

        div()
            .id("wysiwyg-editor")
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .child(self.render_toolbar(block_count, cx))
            .child(self.render_editor(blocks, editing_block, window, cx))
    }
}

impl WysiwygEditor {
    fn render_toolbar(&self, block_count: usize, cx: &App) -> impl IntoElement {
        let editing_info = self
            .editing_block
            .map(|i| format!("Editing block {}", i))
            .unwrap_or_else(|| "Click to edit".to_string());

        h_flex()
            .id("toolbar")
            .w_full()
            .h_10()
            .px_4()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().title_bar)
            .child(
                h_flex()
                    .gap_2()
                    .child("WYSIWYG Markdown Editor")
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                h_flex()
                    .gap_4()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("Blocks: {}", block_count))
                    .child(editing_info),
            )
    }

    fn render_editor(
        &self,
        blocks: Vec<ParsedBlock>,
        editing_block: Option<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let input_state = self.block_input_state.clone();

        div()
            .id("editor-container")
            .flex_1()
            .size_full()
            .overflow_y_scroll()
            .p_6()
            .child(
                div()
                    .max_w(rems(48.))
                    .mx_auto()
                    .children(blocks.into_iter().enumerate().map(|(i, block)| {
                        let is_editing = editing_block == Some(i);

                        if is_editing {
                            // Show input for editing
                            div()
                                .id(("block-edit", i))
                                .w_full()
                                .mb_2()
                                .p_2()
                                .rounded_md()
                                .bg(hsla(0.6, 0.1, 0.5, 0.1))
                                .border_1()
                                .border_color(hsla(0.6, 0.5, 0.5, 0.5))
                                .child(
                                    Input::new(&input_state)
                                        .w_full()
                                        .border_0()
                                        .p_0()
                                        .focus_bordered(false)
                                        .font_family("monospace")
                                        .text_size(px(14.)),
                                )
                                .into_any_element()
                        } else {
                            // Show rendered block with click to edit
                            div()
                                .id(("block-display", i))
                                .w_full()
                                .cursor_pointer()
                                .rounded_md()
                                .hover(|s| s.bg(hsla(0.0, 0.0, 0.0, 0.03)))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.start_editing(i, window, cx);
                                }))
                                .child(render_block(&block))
                                .into_any_element()
                        }
                    })),
            )
    }
}

/// Render a block as formatted content
fn render_block(block: &ParsedBlock) -> impl IntoElement {
    match &block.block_type {
        BlockType::Heading(level) => render_heading(&block.source, *level),
        BlockType::Paragraph => render_paragraph(&block.source),
        BlockType::CodeBlock { lang } => render_code_block(&block.source, lang.clone()),
        BlockType::List { ordered } => render_list(&block.source, *ordered),
        BlockType::Blockquote => render_blockquote(&block.source),
        BlockType::Table => render_table(&block.source),
        BlockType::ThematicBreak => render_thematic_break(),
    }
}

fn render_heading(source: &str, level: u8) -> AnyElement {
    let text = source
        .trim_start_matches(|c: char| c == '#' || c.is_whitespace());

    let (text_size, font_weight) = match level {
        1 => (rems(2.0), FontWeight::BOLD),
        2 => (rems(1.5), FontWeight::SEMIBOLD),
        3 => (rems(1.25), FontWeight::SEMIBOLD),
        4 => (rems(1.1), FontWeight::SEMIBOLD),
        5 => (rems(1.0), FontWeight::SEMIBOLD),
        _ => (rems(0.9), FontWeight::MEDIUM),
    };

    div()
        .w_full()
        .py_2()
        .text_size(text_size)
        .font_weight(font_weight)
        .child(render_inline_text(text))
        .into_any_element()
}

fn render_paragraph(source: &str) -> AnyElement {
    div()
        .w_full()
        .py_2()
        .child(render_inline_text(source))
        .into_any_element()
}

fn render_code_block(source: &str, lang: Option<String>) -> AnyElement {
    let lines: Vec<&str> = source.lines().collect();
    let code = if lines.len() > 2 {
        lines[1..lines.len() - 1].join("\n")
    } else {
        source.to_string()
    };

    let mut container = div()
        .w_full()
        .my_2()
        .p_3()
        .rounded_md()
        .bg(hsla(0.0, 0.0, 0.1, 0.05));

    if let Some(lang) = lang {
        container = container.child(
            div()
                .text_xs()
                .text_color(hsla(0.0, 0.0, 0.5, 1.0))
                .pb_1()
                .child(lang),
        );
    }

    container
        .child(div().font_family("monospace").text_sm().child(code))
        .into_any_element()
}

fn render_list(source: &str, ordered: bool) -> AnyElement {
    let items: Vec<&str> = source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    div()
        .w_full()
        .py_2()
        .pl_4()
        .children(items.iter().enumerate().map(|(i, item)| {
            let content = item
                .trim_start()
                .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == '-' || c == '*')
                .trim_start();

            let prefix = if ordered {
                format!("{}. ", i + 1)
            } else {
                "• ".to_string()
            };

            div()
                .flex()
                .flex_row()
                .gap_1()
                .child(div().w_5().flex_shrink_0().child(prefix))
                .child(render_inline_text(content))
        }))
        .into_any_element()
}

fn render_blockquote(source: &str) -> AnyElement {
    let content = source
        .lines()
        .map(|line| line.trim_start_matches('>').trim_start())
        .collect::<Vec<_>>()
        .join("\n");

    div()
        .w_full()
        .my_2()
        .pl_4()
        .border_l_4()
        .border_color(hsla(0.0, 0.0, 0.5, 0.3))
        .text_color(hsla(0.0, 0.0, 0.4, 1.0))
        .child(render_inline_text(&content))
        .into_any_element()
}

fn render_table(source: &str) -> AnyElement {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return div().into_any_element();
    }

    div()
        .w_full()
        .my_2()
        .border_1()
        .border_color(hsla(0.0, 0.0, 0.8, 0.3))
        .rounded_md()
        .overflow_hidden()
        .children(lines.iter().enumerate().filter_map(|(i, line)| {
            if line.contains("---") || line.contains(":-") {
                return None;
            }

            let cells: Vec<&str> = line
                .split('|')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim())
                .collect();

            let is_header = i == 0;

            Some(
                div()
                    .flex()
                    .w_full()
                    .when(is_header, |d: Div| {
                        d.bg(hsla(0.0, 0.0, 0.0, 0.05))
                            .font_weight(FontWeight::SEMIBOLD)
                    })
                    .border_b_1()
                    .border_color(hsla(0.0, 0.0, 0.8, 0.2))
                    .children(cells.iter().map(|cell| {
                        div()
                            .flex_1()
                            .px_2()
                            .py_1()
                            .border_r_1()
                            .border_color(hsla(0.0, 0.0, 0.8, 0.1))
                            .child(render_inline_text(cell))
                    })),
            )
        }))
        .into_any_element()
}

fn render_thematic_break() -> AnyElement {
    div()
        .w_full()
        .my_4()
        .h(px(1.))
        .bg(hsla(0.0, 0.0, 0.5, 0.3))
        .into_any_element()
}

/// Render inline text with basic formatting
fn render_inline_text(text: &str) -> impl IntoElement {
    let segments = parse_inline_segments(text);

    div()
        .flex()
        .flex_wrap()
        .children(segments.into_iter().map(|seg| {
            let mut el = div().child(seg.text.clone());

            if seg.bold {
                el = el.font_weight(FontWeight::BOLD);
            }
            if seg.italic {
                el = el.italic();
            }
            if seg.code {
                el = el
                    .font_family("monospace")
                    .bg(hsla(0.0, 0.0, 0.0, 0.08))
                    .px_1()
                    .rounded_sm();
            }
            if seg.strikethrough {
                el = el.text_color(hsla(0.0, 0.0, 0.5, 0.6));
            }
            if seg.link.is_some() {
                el = el.text_color(hsla(0.6, 0.8, 0.5, 1.0)).cursor_pointer();
            }

            el
        }))
}

#[derive(Debug, Clone, Default)]
struct InlineSegment {
    text: String,
    bold: bool,
    italic: bool,
    code: bool,
    strikethrough: bool,
    link: Option<String>,
}

fn parse_inline_segments(text: &str) -> Vec<InlineSegment> {
    let mut segments = Vec::new();
    let mut current = InlineSegment::default();
    let mut chars = text.chars().peekable();
    let mut buffer = String::new();

    while let Some(c) = chars.next() {
        match c {
            '*' | '_' => {
                if chars.peek() == Some(&c) {
                    if !buffer.is_empty() {
                        current.text = buffer.clone();
                        segments.push(current.clone());
                        buffer.clear();
                    }
                    chars.next();
                    current.bold = !current.bold;
                } else {
                    if !buffer.is_empty() {
                        current.text = buffer.clone();
                        segments.push(current.clone());
                        buffer.clear();
                    }
                    current.italic = !current.italic;
                }
            }
            '`' => {
                if !buffer.is_empty() {
                    current.text = buffer.clone();
                    segments.push(current.clone());
                    buffer.clear();
                }
                current.code = !current.code;
            }
            '~' if chars.peek() == Some(&'~') => {
                chars.next();
                if !buffer.is_empty() {
                    current.text = buffer.clone();
                    segments.push(current.clone());
                    buffer.clear();
                }
                current.strikethrough = !current.strikethrough;
            }
            '[' => {
                if !buffer.is_empty() {
                    current.text = buffer.clone();
                    segments.push(current.clone());
                    buffer.clear();
                }

                let mut link_text = String::new();
                let mut found_close = false;

                while let Some(lc) = chars.next() {
                    if lc == ']' {
                        found_close = true;
                        break;
                    }
                    link_text.push(lc);
                }

                if found_close && chars.peek() == Some(&'(') {
                    chars.next();
                    let mut url = String::new();
                    while let Some(uc) = chars.next() {
                        if uc == ')' {
                            break;
                        }
                        url.push(uc);
                    }
                    let mut link_seg = current.clone();
                    link_seg.text = link_text;
                    link_seg.link = Some(url);
                    segments.push(link_seg);
                } else {
                    buffer.push('[');
                    buffer.push_str(&link_text);
                    if found_close {
                        buffer.push(']');
                    }
                }
            }
            _ => {
                buffer.push(c);
            }
        }
    }

    if !buffer.is_empty() {
        current.text = buffer;
        segments.push(current);
    }

    segments.retain(|s| !s.text.is_empty());

    if segments.is_empty() {
        segments.push(InlineSegment {
            text: text.to_string(),
            ..Default::default()
        });
    }

    segments
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        cx.activate(true);

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    window_background: WindowBackgroundAppearance::Blurred,
                    ..Default::default()
                },
                |window, cx| {
                    let view = WysiwygEditor::view(window, cx);
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )?;
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
