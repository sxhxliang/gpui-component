//! Skills Page - UI clone of a skills marketplace page with sidebar.
//!
//! This example demonstrates:
//! - A left sidebar with navigation, collapsible thread folders, and user menu popover
//! - A toolbar with search input, refresh and new skill buttons
//! - Section headers with dividers
//! - Two-column card grid layout with icons and action buttons
//! - Scrollable content area

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Anchor, Icon, IconName, Root, Sizable, StyledExt as _,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    popover::Popover,
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

#[derive(Clone)]
pub struct ThreadItem {
    pub title: &'static str,
    pub time: &'static str,
    pub selected: bool,
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

fn agent_studio_threads() -> Vec<ThreadItem> {
    vec![
        ThreadItem {
            title: "$new-component \u{521b}\u{5efa}agent_cli...",
            time: "8h",
            selected: false,
        },
        ThreadItem {
            title: "\u{9075}\u{5faa}\u{5965}\u{5361}\u{59c6}\u{5243}\u{5200}\u{539f}\u{5219}\u{ff0c}\u{91cd}\u{65b0}\u{8bbe}\u{8ba1}\u{4e00}...",
            time: "10h",
            selected: true,
        },
        ThreadItem {
            title: "$gpui-event \u{4f7f}\u{7528} event \u{673a}\u{5236}\u{4f18}...",
            time: "11h",
            selected: false,
        },
        ThreadItem {
            title: "$skill-installer",
            time: "11h",
            selected: false,
        },
        ThreadItem {
            title: "Checking agentx v0.2.4 (/home...",
            time: "2d",
            selected: false,
        },
        ThreadItem {
            title: "running 1 test test src/core/eve...",
            time: "2d",
            selected: false,
        },
        ThreadItem {
            title: "\u{5c06}\u{8f6f}\u{4ef6}\u{6253}\u{5305}\u{6210} macOS app \u{4e4b}\u{540e}\u{ff0c}...",
            time: "3d",
            selected: false,
        },
        ThreadItem {
            title: "\u{5728} README \u{589e}\u{52a0}\u{652f}\u{6301}\u{7684} Agent \u{6839}...",
            time: "5d",
            selected: false,
        },
        ThreadItem {
            title: "src/panels/task_panel/panel.rs ...",
            time: "1w",
            selected: false,
        },
        ThreadItem {
            title: ".github/workflows/release.yml ...",
            time: "1w",
            selected: false,
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
    threads: Vec<ThreadItem>,
    agent_studio_expanded: bool,
    teleagent_expanded: bool,
    user_menu_open: bool,
}

impl SkillsPage {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_state =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search skills"));

        Self {
            focus_handle: cx.focus_handle(),
            search_state,
            installed: installed_skills(),
            recommended: recommended_skills(),
            threads: agent_studio_threads(),
            agent_studio_expanded: true,
            teleagent_expanded: false,
            user_menu_open: false,
        }
    }

    // ========================================================================
    // Sidebar
    // ========================================================================

    fn render_sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .w(px(280.))
            .h_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(self.render_sidebar_nav(cx))
                    .child(self.render_threads_section(cx)),
            )
            .child(self.render_sidebar_bottom(cx))
    }

    fn render_sidebar_nav(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .px_3()
            .pt_3()
            .pb_1()
            .gap(px(1.))
            .child(self.render_nav_item("New thread", IconName::Copy, false, cx))
            .child(self.render_nav_item(
                "Automations",
                IconName::LoaderCircle,
                false,
                cx,
            ))
            .child(self.render_nav_item("Skills", IconName::Settings, true, cx))
    }

    fn render_nav_item(
        &self,
        label: &'static str,
        icon: IconName,
        active: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id(SharedString::from(format!("nav-{}", label)))
            .h_flex()
            .w_full()
            .h(px(34.))
            .px_2()
            .gap_2p5()
            .items_center()
            .rounded(px(6.))
            .cursor_pointer()
            .text_sm()
            .when(active, |this| {
                this.bg(theme.muted.opacity(0.5))
                    .font_medium()
                    .text_color(theme.foreground)
            })
            .when(!active, |this| {
                this.text_color(theme.foreground)
                    .hover(|this| this.bg(theme.muted.opacity(0.3)))
            })
            .child(
                Icon::new(icon)
                    .size(px(16.))
                    .text_color(theme.muted_foreground),
            )
            .child(label)
    }

    fn render_threads_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .w_full()
            .px_3()
            .pt_3()
            .gap_1()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .pb_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Threads"),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Icon::new(IconName::FolderOpen)
                                    .size(px(14.))
                                    .text_color(theme.muted_foreground),
                            )
                            .child(
                                Icon::new(IconName::Menu)
                                    .size(px(14.))
                                    .text_color(theme.muted_foreground),
                            ),
                    ),
            )
            // Folder: agent-studio (collapsible)
            .child(self.render_folder_group(
                "agent-studio",
                self.agent_studio_expanded,
                true,
                cx,
            ))
            // Folder: teleagent (collapsible)
            .child(self.render_folder_group("teleagent", self.teleagent_expanded, false, cx))
    }

    fn render_folder_header(
        &self,
        name: &'static str,
        expanded: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let icon = if expanded {
            IconName::FolderOpen
        } else {
            IconName::Folder
        };

        div()
            .id(SharedString::from(format!("folder-{}", name)))
            .h_flex()
            .w_full()
            .h(px(32.))
            .px_2()
            .gap_2p5()
            .items_center()
            .rounded(px(6.))
            .cursor_pointer()
            .text_sm()
            .text_color(theme.foreground)
            .hover(|this| this.bg(theme.muted.opacity(0.3)))
            .on_click(cx.listener(move |this, _, _, cx| {
                match name {
                    "agent-studio" => this.agent_studio_expanded = !this.agent_studio_expanded,
                    "teleagent" => this.teleagent_expanded = !this.teleagent_expanded,
                    _ => {}
                }
                cx.notify();
            }))
            .child(
                Icon::new(icon)
                    .size(px(15.))
                    .text_color(theme.muted_foreground),
            )
            .child(name)
    }

    fn render_folder_group(
        &self,
        name: &'static str,
        expanded: bool,
        has_threads: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .w_full()
            .gap(px(1.))
            .child(self.render_folder_header(name, expanded, cx))
            .when(expanded && has_threads, |this| {
                this.children(
                    self.threads
                        .iter()
                        .map(|thread| self.render_thread_item(thread, cx)),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("show-more-{}", name)))
                        .h_flex()
                        .w_full()
                        .h(px(30.))
                        .pl(px(34.))
                        .items_center()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .cursor_pointer()
                        .hover(|this| this.text_color(theme.foreground))
                        .child("Show more"),
                )
            })
    }

    fn render_thread_item(
        &self,
        thread: &ThreadItem,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let selected_bg: Hsla = gpui::rgb(0xfef9c3).into();

        div()
            .id(SharedString::from(format!("thread-{}", thread.title)))
            .h_flex()
            .w_full()
            .h(px(32.))
            .pl(px(34.))
            .pr_2()
            .gap_2()
            .items_center()
            .rounded(px(6.))
            .cursor_pointer()
            .when(thread.selected, |this| this.bg(selected_bg))
            .when(!thread.selected, |this| {
                this.hover(|this| this.bg(theme.muted.opacity(0.3)))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .text_color(theme.foreground)
                    .overflow_x_hidden()
                    .text_ellipsis()
                    .child(thread.title),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(thread.time),
            )
    }

    fn render_sidebar_bottom(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .px_3()
            .pb_2()
            .gap_1()
            .child(
                Popover::new("user-menu-popover")
                    .anchor(Anchor::TopLeft)
                    .open(self.user_menu_open)
                    .on_open_change(cx.listener(|this, open, _, cx| {
                        this.user_menu_open = *open;
                        cx.notify();
                    }))
                    .trigger(
                        Button::new("sidebar-settings")
                            .ghost()
                            .icon(IconName::Settings)
                            .label("Settings")
                            .small(),
                    )
                    .content(|_, _, cx| {
                        let theme = cx.theme();

                        v_flex()
                            .w(px(240.))
                            .py_1()
                            // User email
                            .child(
                                div()
                                    .h_flex()
                                    .w_full()
                                    .h(px(34.))
                                    .px_3()
                                    .gap_2p5()
                                    .items_center()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child(
                                        Icon::new(IconName::CircleUser)
                                            .size(px(16.))
                                            .text_color(theme.muted_foreground),
                                    )
                                    .child("mc2liang@gmail.com"),
                            )
                            // Team/org
                            .child(
                                div()
                                    .h_flex()
                                    .w_full()
                                    .h(px(34.))
                                    .px_3()
                                    .gap_2p5()
                                    .items_center()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(
                                        Icon::new(IconName::Settings)
                                            .size(px(16.))
                                            .text_color(theme.muted_foreground),
                                    )
                                    .child("Indevs"),
                            )
                            // Divider
                            .child(div().w_full().h(px(1.)).my_1().bg(theme.border))
                            // Settings
                            .child(
                                div()
                                    .id("menu-settings")
                                    .h_flex()
                                    .w_full()
                                    .h(px(34.))
                                    .px_3()
                                    .gap_2p5()
                                    .items_center()
                                    .rounded(px(4.))
                                    .text_sm()
                                    .font_medium()
                                    .text_color(theme.foreground)
                                    .cursor_pointer()
                                    .hover(|this| this.bg(theme.muted.opacity(0.3)))
                                    .child(
                                        Icon::new(IconName::Settings)
                                            .size(px(16.))
                                            .text_color(theme.muted_foreground),
                                    )
                                    .child("Settings"),
                            )
                            // Log out
                            .child(
                                div()
                                    .id("menu-logout")
                                    .h_flex()
                                    .w_full()
                                    .h(px(34.))
                                    .px_3()
                                    .gap_2p5()
                                    .items_center()
                                    .rounded(px(4.))
                                    .text_sm()
                                    .font_medium()
                                    .text_color(theme.foreground)
                                    .cursor_pointer()
                                    .hover(|this| this.bg(theme.muted.opacity(0.3)))
                                    .child(
                                        Icon::new(IconName::ExternalLink)
                                            .size(px(16.))
                                            .text_color(theme.muted_foreground),
                                    )
                                    .child("Log out"),
                            )
                    }),
            )
    }

    // ========================================================================
    // Main content (Skills page)
    // ========================================================================

    fn render_content(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(theme.background)
            .child(
                div()
                    .w_full()
                    .px_8()
                    .pt_6()
                    .child(self.render_toolbar(cx)),
            )
            .child(
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
            .child(div().w_full().h(px(1.)).bg(theme.border))
    }

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
                div().flex_shrink_0().child(
                    Icon::new(IconName::Settings2)
                        .size(px(16.))
                        .text_color(theme.muted_foreground.opacity(0.6)),
                ),
            )
    }

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
                div().flex_shrink_0().child(
                    Icon::new(IconName::Plus)
                        .size(px(16.))
                        .text_color(theme.muted_foreground.opacity(0.6)),
                ),
            )
    }

    fn render_installed_section(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .child(self.render_section_header("Installed", cx))
            .child(
                h_flex().w_full().gap_3().children(
                    self.installed
                        .iter()
                        .map(|skill| self.render_installed_card(skill, cx)),
                ),
            )
    }

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

        h_flex()
            .id("skills-page")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(theme.background)
            .child(self.render_sidebar(cx))
            .child(self.render_content(cx))
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
                    size(px(1200.0), px(860.0)),
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
