//! Skills Page - UI clone of a skills marketplace page.
//!
//! This example demonstrates:
//! - A toolbar with search input, refresh and new skill buttons
//! - Section headers with dividers
//! - Two-column card grid layout with icons and action buttons
//! - Scrollable content area

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, Sizable, StyledExt as _,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    scroll::ScrollableElement as _,
    v_flex,
};
use gpui_component_assets::Assets;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Clone)]
pub struct SkillItem {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub icon: IconName,
    pub icon_bg: Hsla,
    pub icon_fg: Hsla,
}

// ============================================================================
// Mock Data
// ============================================================================

fn installed_skills() -> Vec<SkillItem> {
    vec![
        SkillItem {
            id: "skill-creator",
            title: "Skill Creator",
            description: "Create or update a skill",
            icon: IconName::Settings2,
            icon_bg: gpui::rgb(0xfef3c7).into(),
            icon_fg: gpui::rgb(0xd97706).into(),
        },
        SkillItem {
            id: "skill-installer",
            title: "Skill Installer",
            description: "Install curated skills from openai/skills or other repos",
            icon: IconName::Inbox,
            icon_bg: gpui::rgb(0xfee2e2).into(),
            icon_fg: gpui::rgb(0xdc2626).into(),
        },
    ]
}

fn recommended_skills() -> Vec<SkillItem> {
    vec![
        SkillItem {
            id: "cloudflare-deploy",
            title: "Cloudflare Deploy",
            description: "Deploy Workers, Pages, and platform services on...",
            icon: IconName::Globe,
            icon_bg: gpui::rgb(0xfff7ed).into(),
            icon_fg: gpui::rgb(0xf97316).into(),
        },
        SkillItem {
            id: "develop-web-game",
            title: "Develop Web Game",
            description: "Web game dev + Playwright test loop",
            icon: IconName::Play,
            icon_bg: gpui::rgb(0xede9fe).into(),
            icon_fg: gpui::rgb(0x7c3aed).into(),
        },
        SkillItem {
            id: "doc",
            title: "Doc",
            description: "Edit and review docx files",
            icon: IconName::File,
            icon_bg: gpui::rgb(0xf3f4f6).into(),
            icon_fg: gpui::rgb(0x6b7280).into(),
        },
        SkillItem {
            id: "figma",
            title: "Figma",
            description: "Use Figma MCP for design-to-code work",
            icon: IconName::Frame,
            icon_bg: gpui::rgb(0xfce7f3).into(),
            icon_fg: gpui::rgb(0xdb2777).into(),
        },
        SkillItem {
            id: "figma-implement-design",
            title: "Figma Implement Design",
            description: "Turn Figma designs into production-ready code",
            icon: IconName::Inspector,
            icon_bg: gpui::rgb(0xede9fe).into(),
            icon_fg: gpui::rgb(0x6d28d9).into(),
        },
        SkillItem {
            id: "gh-address-comments",
            title: "GH Address Comments",
            description: "Address comments in a GitHub PR review",
            icon: IconName::GitHub,
            icon_bg: gpui::rgb(0xf3f4f6).into(),
            icon_fg: gpui::rgb(0x374151).into(),
        },
        SkillItem {
            id: "gh-fix-ci",
            title: "GH Fix CI",
            description: "Debug failing GitHub Actions CI",
            icon: IconName::GitHub,
            icon_bg: gpui::rgb(0xf3f4f6).into(),
            icon_fg: gpui::rgb(0x111827).into(),
        },
        SkillItem {
            id: "imagegen",
            title: "Imagegen",
            description: "Generate and edit images using OpenAI",
            icon: IconName::Map,
            icon_bg: gpui::rgb(0xfef9c3).into(),
            icon_fg: gpui::rgb(0xeab308).into(),
        },
        SkillItem {
            id: "jupyter-notebook",
            title: "Jupyter Notebook",
            description: "Create Jupyter notebooks for experiments and tutorials",
            icon: IconName::BookOpen,
            icon_bg: gpui::rgb(0xf3f4f6).into(),
            icon_fg: gpui::rgb(0xf97316).into(),
        },
        SkillItem {
            id: "linear",
            title: "Linear",
            description: "Manage Linear issues in Codex",
            icon: IconName::Loader,
            icon_bg: gpui::rgb(0xede9fe).into(),
            icon_fg: gpui::rgb(0x4338ca).into(),
        },
        SkillItem {
            id: "netlify-deploy",
            title: "Netlify Deploy",
            description: "Deploy web projects to Netlify with the Netlify CLI",
            icon: IconName::Network,
            icon_bg: gpui::rgb(0xd1fae5).into(),
            icon_fg: gpui::rgb(0x059669).into(),
        },
        SkillItem {
            id: "notion-knowledge-capture",
            title: "Notion Knowledge Capture",
            description: "Capture conversations into structured Notion pages",
            icon: IconName::File,
            icon_bg: gpui::rgb(0xf3f4f6).into(),
            icon_fg: gpui::rgb(0x111827).into(),
        },
    ]
}

// ============================================================================
// Main Component
// ============================================================================

pub struct SkillsPage {
    focus_handle: FocusHandle,
    search_state: Entity<InputState>,
    installed: Vec<SkillItem>,
    recommended: Vec<SkillItem>,
}

impl SkillsPage {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search skills")
        });

        Self {
            focus_handle: cx.focus_handle(),
            search_state,
            installed: installed_skills(),
            recommended: recommended_skills(),
        }
    }

    /// Top toolbar: Refresh | Search | + New skill
    fn render_toolbar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let new_skill_variant = ButtonCustomVariant::new(cx)
            .color(gpui::rgb(0x111827).into())
            .foreground(gpui::white())
            .hover(gpui::rgb(0x1f2937).into())
            .active(gpui::rgb(0x0f172a).into())
            .border(gpui::rgb(0x111827).into());

        h_flex()
            .w_full()
            .items_center()
            .justify_end()
            .gap_3()
            .pb_2()
            .child(
                Button::new("refresh")
                    .ghost()
                    .small()
                    .icon(IconName::LoaderCircle)
                    .label("Refresh")
                    .text_color(theme.muted_foreground),
            )
            .child(
                div().w(px(200.)).child(
                    Input::new(&self.search_state)
                        .small()
                        .prefix(
                            Icon::new(IconName::Search)
                                .size_3()
                                .text_color(theme.muted_foreground),
                        ),
                ),
            )
            .child(
                Button::new("new-skill")
                    .custom(new_skill_variant)
                    .small()
                    .rounded(px(8.))
                    .icon(IconName::Plus)
                    .label("New skill"),
            )
    }

    /// Title section: "Skills" heading + subtitle
    fn render_title(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .gap_1()
            .child(
                div()
                    .text_size(px(28.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.foreground)
                    .child("Skills"),
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Give Codex superpowers."),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0x2563eb))
                            .cursor_pointer()
                            .child("Learn more"),
                    ),
            )
    }

    /// Section header with divider
    fn render_section_header(
        &self,
        label: &'static str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .text_color(theme.foreground)
                    .child(label),
            )
            .child(
                div()
                    .w_full()
                    .h(px(1.))
                    .bg(theme.border),
            )
    }

    /// Render a single skill card (installed variant with edit icon)
    fn render_installed_card(
        &self,
        skill: &SkillItem,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id(SharedString::from(format!("card-{}", skill.id)))
            .h_flex()
            .flex_1()
            .min_w(px(0.))
            .h(px(76.))
            .rounded_lg()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .px_3()
            .py_2()
            .gap_3()
            .items_center()
            .cursor_pointer()
            .hover(|this| this.bg(theme.muted.opacity(0.3)))
            .child(
                // Icon container
                div()
                    .size(px(40.))
                    .rounded(px(10.))
                    .bg(skill.icon_bg)
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(skill.icon.clone())
                            .size_5()
                            .text_color(skill.icon_fg),
                    ),
            )
            .child(
                // Text content
                v_flex()
                    .gap(px(2.))
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .overflow_x_hidden()
                            .text_ellipsis()
                            .child(skill.title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .overflow_x_hidden()
                            .text_ellipsis()
                            .child(skill.description),
                    ),
            )
            .child(
                // Edit action
                div()
                    .flex_shrink_0()
                    .child(
                        Icon::new(IconName::Settings2)
                            .size(px(16.))
                            .text_color(theme.muted_foreground.opacity(0.6)),
                    ),
            )
    }

    /// Render a single skill card (recommended variant with + icon)
    fn render_recommended_card(
        &self,
        skill: &SkillItem,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id(SharedString::from(format!("card-{}", skill.id)))
            .h_flex()
            .flex_1()
            .min_w(px(0.))
            .h(px(76.))
            .rounded_lg()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .px_3()
            .py_2()
            .gap_3()
            .items_center()
            .cursor_pointer()
            .hover(|this| this.bg(theme.muted.opacity(0.3)))
            .child(
                // Icon container
                div()
                    .size(px(40.))
                    .rounded(px(10.))
                    .bg(skill.icon_bg)
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(skill.icon.clone())
                            .size_5()
                            .text_color(skill.icon_fg),
                    ),
            )
            .child(
                // Text content
                v_flex()
                    .gap(px(2.))
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .overflow_x_hidden()
                            .text_ellipsis()
                            .child(skill.title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .overflow_x_hidden()
                            .text_ellipsis()
                            .child(skill.description),
                    ),
            )
            .child(
                // Plus action
                div()
                    .flex_shrink_0()
                    .child(
                        Icon::new(IconName::Plus)
                            .size(px(16.))
                            .text_color(theme.muted_foreground.opacity(0.6)),
                    ),
            )
    }

    /// Render installed section: 2 cards side by side
    fn render_installed_section(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .child(self.render_section_header("Installed", cx))
            .child(
                h_flex()
                    .w_full()
                    .gap_3()
                    .children(
                        self.installed
                            .iter()
                            .map(|skill| self.render_installed_card(skill, cx)),
                    ),
            )
    }

    /// Render recommended section: 2-column grid
    fn render_recommended_section(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .child(self.render_section_header("Recommended", cx))
            .children(self.recommended.chunks(2).map(|row| {
                h_flex()
                    .w_full()
                    .gap_3()
                    .children(
                        row.iter()
                            .map(|skill| self.render_recommended_card(skill, cx)),
                    )
                    // If odd number of items, add empty spacer for last row
                    .when(row.len() == 1, |this| {
                        this.child(div().flex_1().min_w(px(0.)))
                    })
            }))
    }
}

impl Focusable for SkillsPage {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SkillsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .id("skills-page")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(theme.background)
            .child(
                // Toolbar area (non-scrolling)
                div()
                    .w_full()
                    .px_8()
                    .pt_6()
                    .child(self.render_toolbar(cx)),
            )
            .child(
                // Scrollable content area
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .px_8()
                    .pt_4()
                    .pb_8()
                    .gap_6()
                    .child(self.render_title(cx))
                    .child(self.render_installed_section(cx))
                    .child(self.render_recommended_section(cx)),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        cx.activate(true);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1100.0), px(800.0)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("Skills")),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                ..Default::default()
            },
            |window, cx| {
                let page = cx.new(|cx| SkillsPage::new(window, cx));
                cx.new(|cx| Root::new(page, window, cx))
            },
        )
        .unwrap();
    });
}
