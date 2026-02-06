use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    highlighter::Language,
    input::{Input, InputState, TabSize},
    v_flex,
};
use gpui_component_assets::Assets;

const SAMPLE_DIFF: &str = include_str!("./fixtures/test.diff");

pub struct Example {
    diff: Entity<InputState>,
    line_number: bool,
    indent_guides: bool,
    soft_wrap: bool,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let diff = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Language::Diff)
                .line_number(true)
                .indent_guides(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .searchable(true)
                .default_value(SAMPLE_DIFF)
                .placeholder("Unified diff content...")
        });

        Self {
            diff,
            line_number: true,
            indent_guides: true,
            soft_wrap: false,
        }
    }

    fn render_line_number_button(
        &self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new("line-number")
            .when(self.line_number, |this| this.icon(IconName::Check))
            .label("Line Number")
            .ghost()
            .xsmall()
            .on_click(cx.listener(|this, _, window, cx| {
                this.line_number = !this.line_number;
                this.diff.update(cx, |state, cx| {
                    state.set_line_number(this.line_number, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_soft_wrap_button(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("soft-wrap")
            .ghost()
            .xsmall()
            .when(self.soft_wrap, |this| this.icon(IconName::Check))
            .label("Soft Wrap")
            .on_click(cx.listener(|this, _, window, cx| {
                this.soft_wrap = !this.soft_wrap;
                this.diff.update(cx, |state, cx| {
                    state.set_soft_wrap(this.soft_wrap, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_indent_guides_button(
        &self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new("indent-guides")
            .ghost()
            .xsmall()
            .when(self.indent_guides, |this| this.icon(IconName::Check))
            .label("Indent Guides")
            .on_click(cx.listener(|this, _, window, cx| {
                this.indent_guides = !this.indent_guides;
                this.diff.update(cx, |state, cx| {
                    state.set_indent_guides(this.indent_guides, window, cx);
                });
                cx.notify();
            }))
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("diff-viewer")
            .size_full()
            .child(
                v_flex()
                    .w_full()
                    .flex_1()
                    .child(
                        Input::new(&self.diff)
                            .bordered(false)
                            .p_0()
                            .h_full()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_size(cx.theme().mono_font_size)
                            .focus_bordered(false)
                            .into_any_element(),
                    ),
            )
            .child(
                h_flex()
                    .justify_between()
                    .text_sm()
                    .bg(cx.theme().background)
                    .py_1p5()
                    .px_4()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        h_flex()
                            .gap_3()
                            .child(self.render_line_number_button(window, cx))
                            .child(self.render_soft_wrap_button(window, cx))
                            .child(self.render_indent_guides_button(window, cx)),
                    )
                    .child("Unified Diff"),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window("Code Diff", |window, cx| {
            cx.new(|cx| Example::new(window, cx))
        }, cx);
    });
}
