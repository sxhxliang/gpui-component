use std::collections::HashSet;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Disableable, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{InputEvent, InputState, TabSize},
    resizable::{h_resizable, resizable_panel},
    scroll::ScrollableElement,
    v_flex,
};
use gpui_component_assets::Assets;
use similar::{Algorithm, ChangeTag, TextDiff};

/// Old version of the sample code for diff demonstration
const OLD_TEXT: &str = r#"use std::sync::Arc;
use tray_icon::{
    MouseButton, TrayIcon,
    TrayIconBuilder,
    TrayIconEvent,
    menu::{Menu, MenuEvent,
    MenuId, MenuItem,
    PredefinedMenuItem},
};

// 定义菜单项ID常量
const MENU_SHOW_ID: &str = "show_window";
const MENU_QUIT_ID: &str = "quit_app";
// 定义菜单项ID常量
const TRAY_ICON_ID: &str = "plus.agentx.app.tray";
"#;

/// New version with modifications
const NEW_TEXT: &str = r#"use std::sync::Arc;
use tray_icon::{
    MouseButton, TrayIcon,
    TrayIconBuilder,
    TrayIconEvent, TrayIconId,
    menu::{Menu, MenuEvent,
    MenuId, MenuItem,
    PredefinedMenuItem},
};

// 定义菜单项ID常量
const MENU_SHOW_ID: &str = "show_window";
const MENU_QUIT_ID: &str = "quit_app";

// 定义唯一的托盘图标 ID, 避免与其他应用冲突
const TRAY_ICON_ID: &str = "plus.agentx.app.tray";

/// 在 Linux 平台上初始化 GTK
#[cfg(target_os = "linux")]
fn init_gtk() {
    gtk::init().expect("Failed to initialize GTK");
}
"#;

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

const MAX_CONTEXT_LINES: usize = 10;

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
        // Indicator column (- or +)
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
        // Line number column
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
        // Code content
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

pub struct DiffExample {
    #[allow(dead_code)]
    old_editor: Entity<InputState>,
    #[allow(dead_code)]
    new_editor: Entity<InputState>,
    diff_lines: Vec<DiffLine>,
    display_items: Vec<DiffDisplayItem>,
    side_by_side_rows: Vec<SideBySideDisplayRow>,
    view_mode: DiffViewMode,
    old_text: String,
    context_lines: usize,
    show_edge_collapsed: bool,
    show_all: bool,
    expanded_collapses: HashSet<usize>,
    // Scroll handles for sync scrolling
    old_scroll_handle: ScrollHandle,
    new_scroll_handle: ScrollHandle,
    last_old_scroll: Point<Pixels>,
    last_new_scroll: Point<Pixels>,
    _subscriptions: Vec<Subscription>,
}

impl DiffExample {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let old_text = OLD_TEXT.to_string();
        let new_text = NEW_TEXT.to_string();
        let context_lines = 3;
        let show_edge_collapsed = true;
        let diff_lines = compute_diff(&old_text, &new_text);
        let show_all = false;
        let expanded_collapses = HashSet::new();
        let display_items = apply_context_collapsing(
            &diff_lines,
            context_lines,
            show_edge_collapsed,
            &expanded_collapses,
            show_all,
        );
        let side_by_side_rows = build_side_by_side_rows(&display_items);

        let old_editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("rust")
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .default_value(&old_text)
        });

        let new_editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("rust")
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .default_value(&new_text)
        });

        let _subscriptions =
            vec![
                cx.subscribe(&new_editor, |this, editor, ev: &InputEvent, cx| {
                    if matches!(ev, InputEvent::Change) {
                        let new_text = editor.read(cx).value().to_string();
                        this.diff_lines = compute_diff(&this.old_text, &new_text);
                        this.expanded_collapses.clear();
                        this.recompute_display();
                        cx.notify();
                    }
                }),
            ];

        Self {
            old_editor,
            new_editor,
            diff_lines,
            display_items,
            side_by_side_rows,
            view_mode: DiffViewMode::SideBySide,
            old_text,
            context_lines,
            show_edge_collapsed,
            show_all,
            expanded_collapses,
            old_scroll_handle: ScrollHandle::new(),
            new_scroll_handle: ScrollHandle::new(),
            last_old_scroll: point(px(0.), px(0.)),
            last_new_scroll: point(px(0.), px(0.)),
            _subscriptions,
        }
    }

    fn recompute_display(&mut self) {
        self.display_items = apply_context_collapsing(
            &self.diff_lines,
            self.context_lines,
            self.show_edge_collapsed,
            &self.expanded_collapses,
            self.show_all,
        );
        self.side_by_side_rows = build_side_by_side_rows(&self.display_items);
    }

    fn expand_collapsed(&mut self, start_index: usize, cx: &mut Context<Self>) {
        self.show_all = false;
        self.expanded_collapses.insert(start_index);
        self.recompute_display();
        cx.notify();
    }

    fn expand_all(&mut self, cx: &mut Context<Self>) {
        self.show_all = true;
        self.expanded_collapses.clear();
        self.recompute_display();
        cx.notify();
    }

    fn collapse_all(&mut self, cx: &mut Context<Self>) {
        self.show_all = false;
        self.expanded_collapses.clear();
        self.recompute_display();
        cx.notify();
    }

    fn sync_scroll(&mut self, _cx: &mut Context<Self>) {
        let old_offset = self.old_scroll_handle.offset();
        let new_offset = self.new_scroll_handle.offset();

        // Check which side scrolled and sync the other
        if old_offset != self.last_old_scroll && old_offset != new_offset {
            // Old side scrolled, sync new side
            self.new_scroll_handle.set_offset(old_offset);
            self.last_old_scroll = old_offset;
            self.last_new_scroll = old_offset;
        } else if new_offset != self.last_new_scroll && new_offset != old_offset {
            // New side scrolled, sync old side
            self.old_scroll_handle.set_offset(new_offset);
            self.last_old_scroll = new_offset;
            self.last_new_scroll = new_offset;
        }
    }

    #[allow(dead_code)]
    fn build_unified_text(&self) -> String {
        self.diff_lines
            .iter()
            .map(|line| {
                let prefix = match line.kind {
                    DiffLineKind::Added => "+ ",
                    DiffLineKind::Removed => "- ",
                    DiffLineKind::Unchanged => "  ",
                };
                format!("{}{}", prefix, line.text)
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn render_toolbar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stats = compute_stats(&self.diff_lines);
        let has_collapsed = self
            .display_items
            .iter()
            .any(|item| matches!(item, DiffDisplayItem::Collapsed { .. }));

        h_flex()
            .gap_3()
            .p_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .items_center()
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("side-by-side")
                            .label("Side by Side")
                            .small()
                            .when(self.view_mode == DiffViewMode::SideBySide, |b| b.primary())
                            .when(self.view_mode != DiffViewMode::SideBySide, |b| b.ghost())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.view_mode = DiffViewMode::SideBySide;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("unified")
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
            .child(
                h_flex()
                    .gap_1()
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
                                    this.context_lines -= 1;
                                    this.show_all = false;
                                    this.expanded_collapses.clear();
                                    this.recompute_display();
                                    cx.notify();
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
                                    this.context_lines += 1;
                                    this.show_all = false;
                                    this.expanded_collapses.clear();
                                    this.recompute_display();
                                    cx.notify();
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
                            .disabled(self.show_all || !has_collapsed)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.expand_all(cx);
                            })),
                    )
                    .child(
                        Button::new("collapse-all")
                            .label("Collapse")
                            .small()
                            .ghost()
                            .disabled(!self.show_all && self.expanded_collapses.is_empty())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.collapse_all(cx);
                            })),
                    ),
            )
    }

    fn render_side_by_side(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Extract theme values first
        let mono_font = cx.theme().mono_font_family.clone();
        let muted_bg = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;

        // Pre-render all cells to avoid closure issues
        let mut old_cells = Vec::with_capacity(self.side_by_side_rows.len());
        let mut new_cells = Vec::with_capacity(self.side_by_side_rows.len());

        for row in &self.side_by_side_rows {
            match row {
                SideBySideDisplayRow::Line(line_row) => {
                    old_cells.push(render_diff_cell(
                        line_row.old_line_no,
                        line_row.old_text.as_deref(),
                        line_row.old_kind,
                        muted_bg,
                        muted_fg,
                        fg,
                    ));
                    new_cells.push(render_diff_cell(
                        line_row.new_line_no,
                        line_row.new_text.as_deref(),
                        line_row.new_kind,
                        muted_bg,
                        muted_fg,
                        fg,
                    ));
                }
                SideBySideDisplayRow::Collapsed {
                    start_old,
                    start_new,
                    count,
                    start_index,
                } => {
                    let old_start = *start_index;
                    let new_start = *start_index;
                    old_cells.push(
                        render_collapsed_cell(*start_old, *start_new, *count, muted_bg, muted_fg)
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                this.expand_collapsed(old_start, cx);
                            })),
                    );
                    new_cells.push(
                        render_collapsed_cell(*start_old, *start_new, *count, muted_bg, muted_fg)
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                this.expand_collapsed(new_start, cx);
                            })),
                    );
                }
            }
        }

        h_resizable("diff-panels")
            .child(
                resizable_panel().size(px(500.)).child(
                    v_flex()
                        .size_full()
                        .child(
                            h_flex()
                                .px_2()
                                .py_1()
                                .bg(muted_bg)
                                .text_sm()
                                .text_color(muted_fg)
                                .child("Old"),
                        )
                        .child(
                            div()
                                .id("old-scroll")
                                .flex_1()
                                .overflow_y_scroll()
                                .track_scroll(&self.old_scroll_handle)
                                .on_scroll_wheel(cx.listener(
                                    |this, _: &ScrollWheelEvent, _, cx| {
                                        // Trigger re-render to sync scroll
                                        cx.notify();
                                        this.sync_scroll(cx);
                                    },
                                ))
                                .font_family(mono_font.clone())
                                .text_size(px(13.))
                                .children(old_cells),
                        ),
                ),
            )
            .child(
                v_flex()
                    .size_full()
                    .child(
                        h_flex()
                            .px_2()
                            .py_1()
                            .bg(muted_bg)
                            .text_sm()
                            .text_color(muted_fg)
                            .child("New"),
                    )
                    .child(
                        div()
                            .id("new-scroll")
                            .flex_1()
                            .overflow_y_scroll()
                            .track_scroll(&self.new_scroll_handle)
                            .on_scroll_wheel(cx.listener(|this, _: &ScrollWheelEvent, _, cx| {
                                // Trigger re-render to sync scroll
                                cx.notify();
                                this.sync_scroll(cx);
                            }))
                            .font_family(mono_font)
                            .text_size(px(13.))
                            .children(new_cells),
                    )
                    .into_any_element(),
            )
    }

    fn render_unified(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .px_2()
                    .py_1()
                    .bg(cx.theme().muted)
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Unified Diff"),
            )
            .child(
                div()
                    .id("unified-scroll")
                    .flex_1()
                    .overflow_y_scrollbar()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(cx.theme().mono_font_size)
                    .p_2()
                    .children(self.display_items.iter().map(|item| {
                        match item {
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
                                    .child(
                                        div()
                                            .w(px(20.))
                                            .text_color(color)
                                            .child(prefix.to_string()),
                                    )
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
                                render_collapsed_cell(
                                    *start_old,
                                    *start_new,
                                    *count,
                                    cx.theme().muted,
                                    cx.theme().muted_foreground,
                                )
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                    this.expand_collapsed(collapse_index, cx);
                                }))
                                .into_any_element()
                            }
                        }
                    })),
            )
    }

    fn render_status_bar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stats = compute_stats(&self.diff_lines);
        let visible_rows = match self.view_mode {
            DiffViewMode::SideBySide => self.side_by_side_rows.len(),
            DiffViewMode::Unified => self.display_items.len(),
        };

        h_flex()
            .justify_between()
            .text_sm()
            .bg(cx.theme().background)
            .py_1()
            .px_4()
            .border_t_1()
            .border_color(cx.theme().border)
            .text_color(cx.theme().muted_foreground)
            .child(
                h_flex()
                    .gap_4()
                    .child(format!("{} rows", visible_rows))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(gpui::green())
                                    .child(format!("+{} added", stats.added)),
                            )
                            .child(
                                div()
                                    .text_color(gpui::red())
                                    .child(format!("-{} removed", stats.removed)),
                            ),
                    ),
            )
            .child(format!(
                "View: {}",
                match self.view_mode {
                    DiffViewMode::SideBySide => "Side by Side",
                    DiffViewMode::Unified => "Unified",
                }
            ))
    }
}

impl Render for DiffExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Sync scroll positions between left and right panels
        if self.view_mode == DiffViewMode::SideBySide {
            self.sync_scroll(cx);
        }

        v_flex()
            .id("diff-app")
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(window, cx))
            .child(match self.view_mode {
                DiffViewMode::SideBySide => self.render_side_by_side(window, cx).into_any_element(),
                DiffViewMode::Unified => self.render_unified(window, cx).into_any_element(),
            })
            .child(self.render_status_bar(window, cx))
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window_with_size(
            "Diff Viewer",
            Some(size(px(1200.), px(750.))),
            |window, cx| cx.new(|cx| DiffExample::new(window, cx)),
            cx,
        );
    });
}
