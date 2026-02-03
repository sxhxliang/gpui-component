//! Tool call and attachment UI components.

use gpui::{AnyElement, InteractiveElement, IntoElement, ParentElement, StyleRefinement, Styled};
use gpui_component::{
    Icon, IconName, Sizable, StyledExt, h_flex, label::Label, scroll::ScrollableElement, v_flex,
};

use crate::types::{DiffFile, DiffLine, DiffLineKind, FileAttachment};

/// Builds the attachments list.
pub fn build_attachments(
    attachments: &[FileAttachment],
    theme: &gpui_component::Theme,
) -> AnyElement {
    let cards = attachments.iter().map(|file| {
        h_flex()
            .items_center()
            .gap_3()
            .p_3()
            .bg(theme.secondary)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .child(Icon::new(IconName::File).small())
            .child(
                v_flex()
                    .gap_0p5()
                    .child(Label::new(file.name.clone()).text_sm().font_medium())
                    .child(
                        Label::new(format!("{} • {}", file.kind, file.size))
                            .text_xs()
                            .text_color(theme.muted_foreground),
                    ),
            )
            .into_any_element()
    });

    v_flex().gap_2().children(cards).into_any_element()
}

/// Builds a diff file header line with +/- stats.
pub fn build_diff_file_header(file: &DiffFile, theme: &gpui_component::Theme) -> AnyElement {
    use gpui::px;

    h_flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            Icon::new(IconName::File)
                .xsmall()
                .text_color(theme.muted_foreground),
        )
        .child(
            Label::new(file.path.clone())
                .text_xs()
                .font_medium()
                .text_color(theme.foreground),
        )
        .child(gpui::div().flex_1())
        .child(
            Label::new(format!("+{}", file.additions))
                .text_xs()
                .text_color(theme.success),
        )
        .child(
            Label::new(format!("-{}", file.deletions))
                .text_xs()
                .text_color(theme.danger),
        )
        .child(
            h_flex()
                .gap_1()
                .ml(px(4.))
                .child(
                    gpui::div()
                        .size_5()
                        .rounded_sm()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(|s: StyleRefinement| s.bg(theme.secondary))
                        .child(
                            Icon::new(IconName::Check)
                                .xsmall()
                                .text_color(theme.success),
                        ),
                )
                .child(
                    gpui::div()
                        .size_5()
                        .rounded_sm()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(|s: StyleRefinement| s.bg(theme.secondary))
                        .child(Icon::new(IconName::Close).xsmall().text_color(theme.danger)),
                ),
        )
        .into_any_element()
}

/// Builds a single diff line.
pub fn build_diff_line(line: &DiffLine, theme: &gpui_component::Theme) -> AnyElement {
    let (bg, prefix, text_color) = match line.kind {
        DiffLineKind::Added => (theme.success.opacity(0.15), "+", theme.success),
        DiffLineKind::Removed => (theme.danger.opacity(0.15), "-", theme.danger),
        DiffLineKind::Context => (gpui::transparent_black(), " ", theme.muted_foreground),
    };

    h_flex()
        .w_full()
        .bg(bg)
        .px_3()
        .child(
            Label::new(format!("{} {}", prefix, line.text))
                .text_xs()
                .font_family("monospace")
                .text_color(text_color),
        )
        .into_any_element()
}

/// Builds the right-side diff panel.
pub fn build_diff_panel(files: &[DiffFile], theme: &gpui_component::Theme) -> AnyElement {
    use gpui::px;

    let total_additions: usize = files.iter().map(|f| f.additions).sum();
    let total_deletions: usize = files.iter().map(|f| f.deletions).sum();

    let mut panel = v_flex()
        .w(px(320.))
        .min_w(px(280.))
        .h_full()
        .border_1()
        .border_color(theme.border)
        .rounded_lg()
        .bg(theme.background)
        .overflow_hidden();

    // Header
    panel = panel.child(
        h_flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(theme.border)
            .child(
                Label::new(format!("{} files changed", files.len()))
                    .text_sm()
                    .font_semibold(),
            )
            .child(gpui::div().flex_1())
            .child(
                Label::new(format!("+{}", total_additions))
                    .text_xs()
                    .text_color(theme.success),
            )
            .child(
                Label::new(format!("-{}", total_deletions))
                    .text_xs()
                    .text_color(theme.danger),
            ),
    );

    // File list with diffs
    let mut file_list = v_flex().flex_1().min_h_0().overflow_y_scrollbar();
    for file in files {
        file_list = file_list.child(build_diff_file_header(file, theme));
        for line in &file.lines {
            file_list = file_list.child(build_diff_line(line, theme));
        }
    }

    panel.child(file_list).into_any_element()
}
