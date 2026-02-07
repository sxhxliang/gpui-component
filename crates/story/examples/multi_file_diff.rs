use std::collections::HashSet;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, StyledExt,
    button::{Button, ButtonVariants as _},
    collapsible::Collapsible,
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};
use gpui_component_assets::Assets;
use similar::{Algorithm, ChangeTag, TextDiff};

const MAX_CONTEXT_LINES: usize = 10;

#[derive(Clone, Copy)]
struct FileSpec {
    path: &'static str,
    old_text: &'static str,
    new_text: &'static str,
    open: bool,
}

const FILE_SPECS: [FileSpec; 6] = [
    FileSpec {
        path: ".claude/settings.local.json",
        old_text: r#"{
  "theme": "light",
  "telemetry": true,
  "model": "gpt-4.1",
  "ui": {
    "fontSize": 13
  }
}
"#,
        new_text: r#"{
  "theme": "light",
  "telemetry": false,
  "model": "gpt-4.1",
  "ui": {
    "fontSize": 13,
    "lineHeight": 1.5
  }
}
"#,
        open: false,
    },
    FileSpec {
        path: ".gitignore",
        old_text: r#"/target
/.env
/.env.local
"#,
        new_text: r#"/target
/.env
/.env.local
.DS_Store
*.log
"#,
        open: false,
    },
    FileSpec {
        path: "Cargo.toml",
        old_text: r#"[package]
name = "gpui-component"
version = "0.5.1"
edition = "2021"

[dependencies]
gpui = { version = "0.12", default-features = false }
tracing = "0.1"
"#,
        new_text: r#"[package]
name = "gpui-component"
version = "0.5.2"
edition = "2021"

[dependencies]
gpui = { version = "0.12", default-features = false }
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
"#,
        open: true,
    },
    FileSpec {
        path: "crates/agentx-acp-ui/Cargo.toml",
        old_text: r#"[package]
name = "agentx-acp-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
gpui = { workspace = true }
"#,
        new_text: r#"[package]
name = "agentx-acp-ui"
version = "0.1.1"
edition = "2021"

[dependencies]
gpui = { workspace = true }
serde = { version = "1", features = ["derive"] }
"#,
        open: false,
    },
    FileSpec {
        path: "crates/agentx-acp-ui/README.md",
        old_text: r#"# AgentX ACP UI

UI components for the AgentX ACP prototype.

## Usage

Run `cargo run --example acp_ui_story`.
"#,
        new_text: r#"# AgentX ACP UI

UI components for the AgentX ACP prototype.

## Usage

Run `cargo run --example acp_ui_story`.

## Development

- `cargo fmt`
- `cargo clippy`
"#,
        open: false,
    },
    FileSpec {
        path: "crates/agentx-acp-ui/src/agent_message.rs",
        old_text: r#"use std::sync::Arc;

use agent_client_protocol::{ContentBlock, ContentChunk, SessionId};
use gpui::{App, AppContext, Context, ElementId, Entity, IntoElement, ParentElement};

use crate::assets::get_agent_icon;

pub type AgentIconProvider = Arc<dyn Fn(&str) -> Icon + Send + Sync>;

pub struct AgentMessage {
    id: ElementId,
    session_id: SessionId,
    content: Vec<ContentBlock>,
}

impl AgentMessage {
    pub fn new(id: impl Into<ElementId>, session_id: SessionId) -> Self {
        Self {
            id: id.into(),
            session_id,
            content: Vec::new(),
        }
    }

    pub fn push_chunk(&mut self, chunk: ContentChunk) {
        match chunk {
            ContentChunk::Text(text) => {
                self.content.push(ContentBlock::Text(text));
            }
            _ => {}
        }
    }
}
"#,
        new_text: r#"use std::sync::Arc;

use agent_client_protocol::{ContentBlock, ContentChunk, SessionId};
use gpui::{App, AppContext, Context, ElementId, Entity, IntoElement, ParentElement};

use crate::assets::get_agent_icon;

pub type AgentIconProvider = Arc<dyn Fn(&str) -> Icon + Send + Sync>;

pub struct AgentMessage {
    id: ElementId,
    session_id: SessionId,
    content: Vec<ContentBlock>,
    source: Option<String>,
}

impl AgentMessage {
    pub fn new(id: impl Into<ElementId>, session_id: SessionId) -> Self {
        Self {
            id: id.into(),
            session_id,
            content: Vec::new(),
            source: None,
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn push_chunk(&mut self, chunk: ContentChunk) {
        match chunk {
            ContentChunk::Text(text) => {
                self.content.push(ContentBlock::Text(text));
            }
            _ => {}
        }
    }
}
"#,
        open: true,
    },
];

#[derive(Clone, Copy, PartialEq)]
enum DiffLineKind {
    Unchanged,
    Added,
    Removed,
}

#[allow(dead_code)]
#[derive(Clone)]
struct DiffLine {
    kind: DiffLineKind,
    old_line_no: Option<usize>,
    new_line_no: Option<usize>,
    text: String,
}

#[derive(Clone, Copy, PartialEq)]
enum DiffViewMode {
    SideBySide,
    Unified,
}

#[derive(Clone)]
enum DiffDisplayItem {
    Line(DiffLine),
    Collapsed {
        start_old: usize,
        start_new: usize,
        count: usize,
        start_index: usize,
    },
}

#[derive(Clone)]
struct FileDiff {
    path: String,
    diff_lines: Vec<DiffLine>,
    display_items: Vec<DiffDisplayItem>,
    side_by_side_rows: Vec<SideBySideDisplayRow>,
    expanded_collapses: HashSet<usize>,
    show_all: bool,
    open: bool,
}

impl FileDiff {
    fn new(
        path: &str,
        old_text: &str,
        new_text: &str,
        context_lines: usize,
        show_edge_collapsed: bool,
        open: bool,
    ) -> Self {
        let diff_lines = compute_diff(old_text, new_text);
        let expanded_collapses = HashSet::new();
        let show_all = false;
        let display_items = apply_context_collapsing(
            &diff_lines,
            context_lines,
            show_edge_collapsed,
            &expanded_collapses,
            show_all,
        );
        let side_by_side_rows = build_side_by_side_rows(&display_items);

        Self {
            path: path.to_string(),
            diff_lines,
            display_items,
            side_by_side_rows,
            expanded_collapses,
            show_all,
            open,
        }
    }

    fn recompute(&mut self, context_lines: usize, show_edge_collapsed: bool) {
        self.display_items = apply_context_collapsing(
            &self.diff_lines,
            context_lines,
            show_edge_collapsed,
            &self.expanded_collapses,
            self.show_all,
        );
        self.side_by_side_rows = build_side_by_side_rows(&self.display_items);
    }
}

fn compute_diff(old_text: &str, new_text: &str) -> Vec<DiffLine> {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_lines(old_text, new_text);

    let mut lines = Vec::new();
    let mut old_line_no = 1usize;
    let mut new_line_no = 1usize;

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            ChangeTag::Equal => {
                lines.push(DiffLine {
                    kind: DiffLineKind::Unchanged,
                    old_line_no: Some(old_line_no),
                    new_line_no: Some(new_line_no),
                    text,
                });
                old_line_no += 1;
                new_line_no += 1;
            }
            ChangeTag::Delete => {
                lines.push(DiffLine {
                    kind: DiffLineKind::Removed,
                    old_line_no: Some(old_line_no),
                    new_line_no: None,
                    text,
                });
                old_line_no += 1;
            }
            ChangeTag::Insert => {
                lines.push(DiffLine {
                    kind: DiffLineKind::Added,
                    old_line_no: None,
                    new_line_no: Some(new_line_no),
                    text,
                });
                new_line_no += 1;
            }
        }
    }

    lines
}

fn apply_context_collapsing(
    diff_lines: &[DiffLine],
    context_lines: usize,
    show_edge_collapsed: bool,
    expanded_collapses: &HashSet<usize>,
    show_all: bool,
) -> Vec<DiffDisplayItem> {
    if show_all {
        return diff_lines
            .iter()
            .cloned()
            .map(DiffDisplayItem::Line)
            .collect();
    }

    let has_changes = diff_lines
        .iter()
        .any(|line| line.kind != DiffLineKind::Unchanged);
    if !has_changes {
        return diff_lines
            .iter()
            .cloned()
            .map(DiffDisplayItem::Line)
            .collect();
    }

    let min_collapse_size = context_lines * 2 + 1;
    let mut display_items: Vec<DiffDisplayItem> = Vec::new();
    let mut context_buffer: Vec<DiffLine> = Vec::new();
    let mut last_change_index: Option<usize> = None;

    for (i, line) in diff_lines.iter().enumerate() {
        match line.kind {
            DiffLineKind::Unchanged => {
                context_buffer.push(line.clone());
            }
            DiffLineKind::Added | DiffLineKind::Removed => {
                if !context_buffer.is_empty() {
                    let buffer_start_index = i - context_buffer.len();
                    if let Some(last_idx) = last_change_index {
                        let distance = i - last_idx - 1;

                        if distance >= min_collapse_size {
                            let collapsed_count = distance.saturating_sub(context_lines * 2);
                            let collapsed_start_index = buffer_start_index + context_lines;
                            let is_expanded = expanded_collapses.contains(&collapsed_start_index);

                            if is_expanded || collapsed_count == 0 {
                                for ctx in &context_buffer {
                                    display_items.push(DiffDisplayItem::Line(ctx.clone()));
                                }
                            } else {
                                for ctx in context_buffer.iter().take(context_lines) {
                                    display_items.push(DiffDisplayItem::Line(ctx.clone()));
                                }

                                if let Some((start_old, start_new)) =
                                    context_line_numbers(&context_buffer[context_lines])
                                {
                                    display_items.push(DiffDisplayItem::Collapsed {
                                        start_old,
                                        start_new,
                                        count: collapsed_count,
                                        start_index: collapsed_start_index,
                                    });
                                }

                                let start = context_buffer.len().saturating_sub(context_lines);
                                for ctx in context_buffer.iter().skip(start) {
                                    display_items.push(DiffDisplayItem::Line(ctx.clone()));
                                }
                            }
                        } else {
                            for ctx in &context_buffer {
                                display_items.push(DiffDisplayItem::Line(ctx.clone()));
                            }
                        }
                    } else {
                        if context_buffer.len() > context_lines {
                            if show_edge_collapsed {
                                let collapsed_count = context_buffer.len() - context_lines;
                                let collapsed_start_index = buffer_start_index;
                                let is_expanded =
                                    expanded_collapses.contains(&collapsed_start_index);

                                if is_expanded {
                                    for ctx in &context_buffer {
                                        display_items.push(DiffDisplayItem::Line(ctx.clone()));
                                    }
                                } else if let Some((start_old, start_new)) =
                                    context_line_numbers(&context_buffer[0])
                                {
                                    display_items.push(DiffDisplayItem::Collapsed {
                                        start_old,
                                        start_new,
                                        count: collapsed_count,
                                        start_index: collapsed_start_index,
                                    });
                                }
                            }

                            if !show_edge_collapsed
                                || !expanded_collapses.contains(&buffer_start_index)
                            {
                                let start = context_buffer.len() - context_lines;
                                for ctx in context_buffer.iter().skip(start) {
                                    display_items.push(DiffDisplayItem::Line(ctx.clone()));
                                }
                            }
                        } else {
                            for ctx in &context_buffer {
                                display_items.push(DiffDisplayItem::Line(ctx.clone()));
                            }
                        }
                    }

                    context_buffer.clear();
                }

                display_items.push(DiffDisplayItem::Line(line.clone()));
                last_change_index = Some(i);
            }
        }
    }

    if !context_buffer.is_empty() {
        if context_buffer.len() > context_lines {
            let buffer_start_index = diff_lines.len() - context_buffer.len();
            for ctx in context_buffer.iter().take(context_lines) {
                display_items.push(DiffDisplayItem::Line(ctx.clone()));
            }

            if show_edge_collapsed {
                let collapsed_count = context_buffer.len() - context_lines;
                let collapsed_start_index = buffer_start_index + context_lines;
                let is_expanded = expanded_collapses.contains(&collapsed_start_index);
                if is_expanded {
                    for ctx in context_buffer.iter().skip(context_lines) {
                        display_items.push(DiffDisplayItem::Line(ctx.clone()));
                    }
                    return display_items;
                }
                if let Some((start_old, start_new)) =
                    context_line_numbers(&context_buffer[context_lines])
                {
                    display_items.push(DiffDisplayItem::Collapsed {
                        start_old,
                        start_new,
                        count: collapsed_count,
                        start_index: collapsed_start_index,
                    });
                }
            }
        } else {
            for ctx in &context_buffer {
                display_items.push(DiffDisplayItem::Line(ctx.clone()));
            }
        }
    }

    display_items
}

fn context_line_numbers(line: &DiffLine) -> Option<(usize, usize)> {
    match (line.old_line_no, line.new_line_no) {
        (Some(old_no), Some(new_no)) => Some((old_no, new_no)),
        _ => None,
    }
}

/// A row in the side-by-side view
#[derive(Clone)]
struct SideBySideRow {
    old_line_no: Option<usize>,
    old_text: Option<String>,
    old_kind: DiffLineKind,
    new_line_no: Option<usize>,
    new_text: Option<String>,
    new_kind: DiffLineKind,
}

#[derive(Clone)]
enum SideBySideDisplayRow {
    Line(SideBySideRow),
    Collapsed {
        start_old: usize,
        start_new: usize,
        count: usize,
        start_index: usize,
    },
}

/// Build side-by-side rows from diff lines
fn build_side_by_side_rows(display_items: &[DiffDisplayItem]) -> Vec<SideBySideDisplayRow> {
    let mut rows = Vec::new();
    let mut i = 0;

    while i < display_items.len() {
        let item = &display_items[i];

        match item {
            DiffDisplayItem::Collapsed {
                start_old,
                start_new,
                count,
                start_index,
            } => {
                rows.push(SideBySideDisplayRow::Collapsed {
                    start_old: *start_old,
                    start_new: *start_new,
                    count: *count,
                    start_index: *start_index,
                });
                i += 1;
            }
            DiffDisplayItem::Line(line) => match line.kind {
                DiffLineKind::Unchanged => {
                    rows.push(SideBySideDisplayRow::Line(SideBySideRow {
                        old_line_no: line.old_line_no,
                        old_text: Some(line.text.clone()),
                        old_kind: DiffLineKind::Unchanged,
                        new_line_no: line.new_line_no,
                        new_text: Some(line.text.clone()),
                        new_kind: DiffLineKind::Unchanged,
                    }));
                    i += 1;
                }
                DiffLineKind::Removed => {
                    if i + 1 < display_items.len() {
                        if let DiffDisplayItem::Line(next_line) = &display_items[i + 1] {
                            if next_line.kind == DiffLineKind::Added {
                                rows.push(SideBySideDisplayRow::Line(SideBySideRow {
                                    old_line_no: line.old_line_no,
                                    old_text: Some(line.text.clone()),
                                    old_kind: DiffLineKind::Removed,
                                    new_line_no: next_line.new_line_no,
                                    new_text: Some(next_line.text.clone()),
                                    new_kind: DiffLineKind::Added,
                                }));
                                i += 2;
                                continue;
                            }
                        }
                    }

                    rows.push(SideBySideDisplayRow::Line(SideBySideRow {
                        old_line_no: line.old_line_no,
                        old_text: Some(line.text.clone()),
                        old_kind: DiffLineKind::Removed,
                        new_line_no: None,
                        new_text: None,
                        new_kind: DiffLineKind::Unchanged,
                    }));
                    i += 1;
                }
                DiffLineKind::Added => {
                    rows.push(SideBySideDisplayRow::Line(SideBySideRow {
                        old_line_no: None,
                        old_text: None,
                        old_kind: DiffLineKind::Unchanged,
                        new_line_no: line.new_line_no,
                        new_text: Some(line.text.clone()),
                        new_kind: DiffLineKind::Added,
                    }));
                    i += 1;
                }
            },
        }
    }

    rows
}

struct DiffStats {
    added: usize,
    removed: usize,
}

fn compute_stats(diff_lines: &[DiffLine]) -> DiffStats {
    let mut added = 0;
    let mut removed = 0;
    for line in diff_lines {
        match line.kind {
            DiffLineKind::Added => added += 1,
            DiffLineKind::Removed => removed += 1,
            DiffLineKind::Unchanged => {}
        }
    }
    DiffStats { added, removed }
}

/// Render a single diff cell (one side of a row)
fn render_diff_cell(
    line_no: Option<usize>,
    text: Option<&str>,
    kind: DiffLineKind,
    muted_bg: Hsla,
    muted_fg: Hsla,
    fg: Hsla,
) -> Div {
    let (bg_color, indicator_color, indicator) = match kind {
        DiffLineKind::Added => (gpui::green().opacity(0.15), gpui::green(), "+"),
        DiffLineKind::Removed => (gpui::red().opacity(0.15), gpui::red(), "-"),
        DiffLineKind::Unchanged => (gpui::transparent_black(), muted_fg, " "),
    };

    let has_content = text.is_some();
    let display_text = text.unwrap_or("").trim_end().to_string();

    h_flex()
        .w_full()
        .h(px(20.))
        .bg(bg_color)
        .when(!has_content && kind == DiffLineKind::Unchanged, |this| {
            this.bg(muted_bg.opacity(0.3))
        })
        .child(
            div()
                .w(px(20.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(indicator_color)
                .when(kind != DiffLineKind::Unchanged, |this| {
                    this.bg(indicator_color.opacity(0.3))
                })
                .child(indicator),
        )
        .child(
            div()
                .w(px(50.))
                .h_full()
                .flex()
                .items_center()
                .justify_end()
                .pr_2()
                .text_color(muted_fg)
                .child(line_no.map(|n| n.to_string()).unwrap_or_default()),
        )
        .child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .pl_2()
                .text_color(fg)
                .overflow_x_hidden()
                .child(display_text),
        )
}

fn render_collapsed_cell(
    start_old: usize,
    start_new: usize,
    count: usize,
    muted_bg: Hsla,
    muted_fg: Hsla,
) -> Div {
    h_flex()
        .w_full()
        .h(px(20.))
        .items_center()
        .justify_center()
        .bg(muted_bg.opacity(0.3))
        .hover(|this| this.bg(muted_bg.opacity(0.45)))
        .text_size(px(11.))
        .text_color(muted_fg)
        .child(format!(
            "... {} unchanged lines hidden ({}..{}, {}..{}) ...",
            count,
            start_old,
            start_old + count - 1,
            start_new,
            start_new + count - 1
        ))
}

fn format_unified_line_numbers(line: &DiffLine) -> String {
    let old_display = line
        .old_line_no
        .map(|n| format!("{:>4}", n))
        .unwrap_or_else(|| "    ".to_string());
    let new_display = line
        .new_line_no
        .map(|n| format!("{:>4}", n))
        .unwrap_or_else(|| "    ".to_string());

    format!("{} {}  ", old_display, new_display)
}

pub struct MultiFileDiffExample {
    files: Vec<FileDiff>,
    view_mode: DiffViewMode,
    context_lines: usize,
    show_edge_collapsed: bool,
}

impl MultiFileDiffExample {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let context_lines = 3;
        let show_edge_collapsed = true;

        let files = FILE_SPECS
            .iter()
            .map(|spec| {
                FileDiff::new(
                    spec.path,
                    spec.old_text,
                    spec.new_text,
                    context_lines,
                    show_edge_collapsed,
                    spec.open,
                )
            })
            .collect();

        Self {
            files,
            view_mode: DiffViewMode::SideBySide,
            context_lines,
            show_edge_collapsed,
        }
    }

    fn toggle_file(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(file) = self.files.get_mut(index) {
            file.open = !file.open;
            cx.notify();
        }
    }

    fn expand_collapsed(&mut self, file_index: usize, start_index: usize, cx: &mut Context<Self>) {
        if let Some(file) = self.files.get_mut(file_index) {
            file.show_all = false;
            file.expanded_collapses.insert(start_index);
            file.recompute(self.context_lines, self.show_edge_collapsed);
            cx.notify();
        }
    }

    fn expand_all(&mut self, cx: &mut Context<Self>) {
        for file in &mut self.files {
            file.show_all = true;
            file.expanded_collapses.clear();
            file.recompute(self.context_lines, self.show_edge_collapsed);
        }
        cx.notify();
    }

    fn collapse_all(&mut self, cx: &mut Context<Self>) {
        for file in &mut self.files {
            file.show_all = false;
            file.expanded_collapses.clear();
            file.recompute(self.context_lines, self.show_edge_collapsed);
        }
        cx.notify();
    }

    fn set_context_lines(&mut self, context_lines: usize, cx: &mut Context<Self>) {
        self.context_lines = context_lines;
        for file in &mut self.files {
            file.show_all = false;
            file.expanded_collapses.clear();
            file.recompute(self.context_lines, self.show_edge_collapsed);
        }
        cx.notify();
    }

    fn total_stats(&self) -> DiffStats {
        let mut added = 0;
        let mut removed = 0;
        for file in &self.files {
            let stats = compute_stats(&file.diff_lines);
            added += stats.added;
            removed += stats.removed;
        }
        DiffStats { added, removed }
    }

    fn render_toolbar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stats = self.total_stats();
        let file_count = self.files.len();
        let has_collapsed = self.files.iter().any(|file| {
            file.display_items
                .iter()
                .any(|item| matches!(item, DiffDisplayItem::Collapsed { .. }))
        });
        let can_collapse = self
            .files
            .iter()
            .any(|file| file.show_all || !file.expanded_collapses.is_empty());

        v_flex()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .px_3()
                    .py_2()
                    .bg(cx.theme().background)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .text_lg()
                            .child("All branch changes")
                            .child(
                                Icon::new(IconName::ChevronDown)
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} files", file_count))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_color(gpui::green())
                                            .child(format!("+{}", stats.added)),
                                    )
                                    .child(
                                        div()
                                            .text_color(gpui::red())
                                            .child(format!("-{}", stats.removed)),
                                    ),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .px_3()
                    .py_2()
                    .bg(cx.theme().background)
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("view-side-by-side")
                                    .label("Side by Side")
                                    .small()
                                    .when(self.view_mode == DiffViewMode::SideBySide, |b| {
                                        b.primary()
                                    })
                                    .when(self.view_mode != DiffViewMode::SideBySide, |b| b.ghost())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.view_mode = DiffViewMode::SideBySide;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("view-unified")
                                    .label("Unified")
                                    .small()
                                    .when(self.view_mode == DiffViewMode::Unified, |b| b.primary())
                                    .when(self.view_mode != DiffViewMode::Unified, |b| b.ghost())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.view_mode = DiffViewMode::Unified;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .text_sm()
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Context: {}", self.context_lines)),
                            )
                            .child(
                                Button::new("context-dec")
                                    .label("-")
                                    .small()
                                    .ghost()
                                    .disabled(self.context_lines == 0)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if this.context_lines > 0 {
                                            this.set_context_lines(this.context_lines - 1, cx);
                                        }
                                    })),
                            )
                            .child(
                                Button::new("context-inc")
                                    .label("+")
                                    .small()
                                    .ghost()
                                    .disabled(self.context_lines >= MAX_CONTEXT_LINES)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if this.context_lines < MAX_CONTEXT_LINES {
                                            this.set_context_lines(this.context_lines + 1, cx);
                                        }
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .text_sm()
                            .child(
                                Button::new("expand-all")
                                    .label("Expand")
                                    .small()
                                    .ghost()
                                    .disabled(!has_collapsed)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.expand_all(cx);
                                    })),
                            )
                            .child(
                                Button::new("collapse-all")
                                    .label("Collapse")
                                    .small()
                                    .ghost()
                                    .disabled(!can_collapse)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.collapse_all(cx);
                                    })),
                            ),
                    ),
            )
    }

    fn render_file_header(
        &self,
        file_index: usize,
        file: &FileDiff,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let stats = compute_stats(&file.diff_lines);
        let is_open = file.open;
        let toggle_index = file_index;

        h_flex()
            .id(format!("file-header-{}", file_index))
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .bg(cx.theme().muted.opacity(0.2))
            .when(is_open, |this| this.bg(cx.theme().muted.opacity(0.35)))
            .hover(|this| this.bg(cx.theme().muted.opacity(0.45)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.toggle_file(toggle_index, cx);
                }),
            )
            .child(
                Icon::new(if is_open {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .text_color(cx.theme().foreground)
                    .font_semibold()
                    .child(file.path.clone()),
            )
            .child(div().flex_1())
            .child(
                h_flex()
                    .gap_2()
                    .text_sm()
                    .child(
                        div()
                            .text_color(gpui::green())
                            .child(format!("+{}", stats.added)),
                    )
                    .child(
                        div()
                            .text_color(gpui::red())
                            .child(format!("-{}", stats.removed)),
                    ),
            )
    }

    fn render_side_by_side_for_file(
        &self,
        file_index: usize,
        file: &FileDiff,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mono_font = cx.theme().mono_font_family.clone();
        let muted_bg = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;
        let border = cx.theme().border;

        let mut rows: Vec<AnyElement> = Vec::with_capacity(file.side_by_side_rows.len());
        for row in &file.side_by_side_rows {
            match row {
                SideBySideDisplayRow::Line(line_row) => {
                    let left = render_diff_cell(
                        line_row.old_line_no,
                        line_row.old_text.as_deref(),
                        line_row.old_kind,
                        muted_bg,
                        muted_fg,
                        fg,
                    );
                    let right = render_diff_cell(
                        line_row.new_line_no,
                        line_row.new_text.as_deref(),
                        line_row.new_kind,
                        muted_bg,
                        muted_fg,
                        fg,
                    );

                    rows.push(
                        h_flex()
                            .w_full()
                            .child(div().flex_1().child(left))
                            .child(
                                div()
                                    .flex_1()
                                    .border_l_1()
                                    .border_color(border)
                                    .child(right),
                            )
                            .into_any_element(),
                    );
                }
                SideBySideDisplayRow::Collapsed {
                    start_old,
                    start_new,
                    count,
                    start_index,
                } => {
                    let collapse_index = *start_index;
                    let collapse_file = file_index;
                    rows.push(
                        render_collapsed_cell(*start_old, *start_new, *count, muted_bg, muted_fg)
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.expand_collapsed(collapse_file, collapse_index, cx);
                                }),
                            )
                            .into_any_element(),
                    );
                }
            }
        }

        v_flex()
            .font_family(mono_font)
            .text_size(px(13.))
            .child(
                h_flex()
                    .w_full()
                    .bg(muted_bg)
                    .text_sm()
                    .text_color(muted_fg)
                    .child(div().flex_1().px_2().py_1().child("Old"))
                    .child(
                        div()
                            .flex_1()
                            .px_2()
                            .py_1()
                            .border_l_1()
                            .border_color(border)
                            .child("New"),
                    ),
            )
            .child(v_flex().children(rows))
    }

    fn render_unified_for_file(
        &self,
        file_index: usize,
        file: &FileDiff,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .font_family(cx.theme().mono_font_family.clone())
            .text_size(cx.theme().mono_font_size)
            .children(file.display_items.iter().map(|item| match item {
                DiffDisplayItem::Line(line) => {
                    let (prefix, color) = match line.kind {
                        DiffLineKind::Added => ("+", gpui::green()),
                        DiffLineKind::Removed => ("-", gpui::red()),
                        DiffLineKind::Unchanged => (" ", cx.theme().muted_foreground),
                    };

                    let bg = match line.kind {
                        DiffLineKind::Added => gpui::green().opacity(0.1),
                        DiffLineKind::Removed => gpui::red().opacity(0.1),
                        DiffLineKind::Unchanged => gpui::transparent_black(),
                    };

                    h_flex()
                        .w_full()
                        .bg(bg)
                        .child(
                            div()
                                .min_w(px(70.))
                                .text_color(color)
                                .child(format_unified_line_numbers(line)),
                        )
                        .child(div().w(px(20.)).text_color(color).child(prefix))
                        .child(
                            div()
                                .flex_1()
                                .text_color(cx.theme().foreground)
                                .child(line.text.trim_end().to_string()),
                        )
                        .into_any_element()
                }
                DiffDisplayItem::Collapsed {
                    start_old,
                    start_new,
                    count,
                    start_index,
                } => {
                    let collapse_index = *start_index;
                    let collapse_file = file_index;
                    render_collapsed_cell(
                        *start_old,
                        *start_new,
                        *count,
                        cx.theme().muted,
                        cx.theme().muted_foreground,
                    )
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.expand_collapsed(collapse_file, collapse_index, cx);
                        }),
                    )
                    .into_any_element()
                }
            }))
    }

    fn render_file_panel(
        &self,
        file_index: usize,
        file: &FileDiff,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let content = match self.view_mode {
            DiffViewMode::SideBySide => self
                .render_side_by_side_for_file(file_index, file, cx)
                .into_any_element(),
            DiffViewMode::Unified => self
                .render_unified_for_file(file_index, file, cx)
                .into_any_element(),
        };

        Collapsible::new()
            .open(file.open)
            .gap_0()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(self.render_file_header(file_index, file, cx))
            .content(
                div()
                    .px_3()
                    .pb_3()
                    .pt_2()
                    .bg(cx.theme().background)
                    .child(content),
            )
    }
}

impl Render for MultiFileDiffExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut panels: Vec<AnyElement> = Vec::with_capacity(self.files.len());
        for (index, file) in self.files.iter().enumerate() {
            panels.push(self.render_file_panel(index, file, cx).into_any_element());
        }

        v_flex()
            .id("multi-file-diff")
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(window, cx))
            .child(div().flex_1().overflow_y_scrollbar().children(panels))
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window_with_size(
            "Multi-file Diff Viewer",
            Some(size(px(1200.), px(750.))),
            |window, cx| cx.new(|cx| MultiFileDiffExample::new(window, cx)),
            cx,
        );
    });
}
