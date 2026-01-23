//! WorkspaceSidebar - A workspace sidebar component similar to VS Code or IDE project explorers
//!
//! This component demonstrates a sidebar with:
//! - Project groups that can be expanded/collapsed
//! - Worktree items with git stats and status badges
//! - View toggle between tree view and timeline view
//! - Header with home icon and view toggle
//! - Footer with action buttons

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Icon, IconName, Selectable, Sizable, StyledExt as _,
    button::{Button, ButtonGroup, ButtonVariants as _},
    scroll::ScrollableElement as _,
};
use gpui_component_assets::Assets;

#[cfg(feature = "story")]
use gpui_component_story::Story;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkspaceStatus {
    Ready,
    Conflict,
    Archive,
    None,
}

#[derive(Clone, Debug)]
pub struct Worktree {
    pub id: String,
    pub branch_name: String,
    pub base_branch: String,
    pub additions: u32,
    pub deletions: u32,
    pub status: WorkspaceStatus,
    pub shortcut: Option<String>,
    pub time_ago: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub worktrees: Vec<Worktree>,
    pub is_expanded: bool,
    pub has_more_icon: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Tree,
    Timeline,
}

// ============================================================================
// Mock Data
// ============================================================================

fn mock_projects() -> Vec<Project> {
    vec![
        Project {
            id: "1".to_string(),
            name: "conductor".to_string(),
            is_expanded: true,
            has_more_icon: false,
            worktrees: vec![
                Worktree {
                    id: "1-1".to_string(),
                    branch_name: "archive-in-repo-details".to_string(),
                    base_branch: "kampala-v3".to_string(),
                    additions: 312,
                    deletions: 332,
                    status: WorkspaceStatus::Ready,
                    shortcut: Some("Ctrl+1".to_string()),
                    time_ago: None,
                },
                Worktree {
                    id: "1-2".to_string(),
                    branch_name: "system-tray-status".to_string(),
                    base_branch: "caracas-v2".to_string(),
                    additions: 611,
                    deletions: 1,
                    status: WorkspaceStatus::Conflict,
                    shortcut: Some("Ctrl+2".to_string()),
                    time_ago: None,
                },
            ],
        },
        Project {
            id: "2".to_string(),
            name: "melty_home".to_string(),
            is_expanded: true,
            has_more_icon: false,
            worktrees: vec![
                Worktree {
                    id: "2-1".to_string(),
                    branch_name: "update-instructions-codex".to_string(),
                    base_branch: "papeete-v1".to_string(),
                    additions: 1,
                    deletions: 1,
                    status: WorkspaceStatus::Ready,
                    shortcut: Some("Ctrl+3".to_string()),
                    time_ago: None,
                },
                Worktree {
                    id: "2-2".to_string(),
                    branch_name: "add-agent-workspaces-txt".to_string(),
                    base_branch: "maputo-v2".to_string(),
                    additions: 1,
                    deletions: 0,
                    status: WorkspaceStatus::Archive,
                    shortcut: Some("Ctrl+4".to_string()),
                    time_ago: None,
                },
                Worktree {
                    id: "2-3".to_string(),
                    branch_name: "cbh123/melty-labs-ho...".to_string(),
                    base_branch: "austin".to_string(),
                    additions: 1037,
                    deletions: 96,
                    status: WorkspaceStatus::None,
                    shortcut: Some("Ctrl+5".to_string()),
                    time_ago: Some("2mo ago".to_string()),
                },
            ],
        },
        Project {
            id: "3".to_string(),
            name: "swipe".to_string(),
            is_expanded: false,
            has_more_icon: false,
            worktrees: vec![],
        },
        Project {
            id: "4".to_string(),
            name: "conductor-docs".to_string(),
            is_expanded: false,
            has_more_icon: false,
            worktrees: vec![],
        },
        Project {
            id: "5".to_string(),
            name: "conductor_api".to_string(),
            is_expanded: false,
            has_more_icon: true,
            worktrees: vec![],
        },
        Project {
            id: "6".to_string(),
            name: "chorus".to_string(),
            is_expanded: false,
            has_more_icon: true,
            worktrees: vec![],
        },
        Project {
            id: "7".to_string(),
            name: "api".to_string(),
            is_expanded: false,
            has_more_icon: true,
            worktrees: vec![],
        },
        Project {
            id: "8".to_string(),
            name: "metarquiz-2".to_string(),
            is_expanded: false,
            has_more_icon: true,
            worktrees: vec![],
        },
    ]
}

// ============================================================================
// Main Story Component
// ============================================================================

pub struct WorkspaceSidebarStory {
    focus_handle: FocusHandle,
    projects: Vec<Project>,
    selected_worktree_id: Option<String>,
    view_mode: ViewMode,
}

impl WorkspaceSidebarStory {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            projects: mock_projects(),
            selected_worktree_id: Some("1-1".to_string()),
            view_mode: ViewMode::Tree,
        }
    }

    fn toggle_project(&mut self, project_id: String, cx: &mut Context<Self>) {
        if let Some(project) = self.projects.iter_mut().find(|p| p.id == project_id) {
            project.is_expanded = !project.is_expanded;
            cx.notify();
        }
    }

    fn select_worktree(&mut self, worktree_id: String, cx: &mut Context<Self>) {
        self.selected_worktree_id = Some(worktree_id);
        cx.notify();
    }

    fn set_view_mode(&mut self, mode: ViewMode, cx: &mut Context<Self>) {
        self.view_mode = mode;
        cx.notify();
    }

    fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let view_mode = self.view_mode;

        h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .px_3()
            .py_3()
            .border_b_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    // Use Inbox icon as a substitute for Home (not available)
                    .child(Icon::new(IconName::Inbox).size_4().text_color(theme.muted_foreground))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.foreground)
                            .child("Home"),
                    ),
            )
            .child(
                ButtonGroup::new("view-toggle")
                    .small()
                    .child(
                        Button::new("tree-view")
                            // Use LayoutDashboard as a substitute for LayoutList
                            .icon(IconName::LayoutDashboard)
                            .ghost()
                            .xsmall()
                            .selected(view_mode == ViewMode::Tree)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_view_mode(ViewMode::Tree, cx);
                            })),
                    )
                    .child(
                        Button::new("timeline-view")
                            // Use Menu as a substitute for List
                            .icon(IconName::Menu)
                            .ghost()
                            .xsmall()
                            .selected(view_mode == ViewMode::Timeline)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_view_mode(ViewMode::Timeline, cx);
                            })),
                    ),
            )
    }

    fn render_footer(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .px_3()
            .py_2()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Button::new("add-repo")
                    .ghost()
                    .small()
                    // Use FolderOpen as a substitute for FolderPlus
                    .icon(IconName::FolderOpen)
                    .label("Add repository"),
            )
            .child(
                h_flex()
                    .gap_1()
                    // Use Delete as a substitute for Trash
                    .child(Button::new("trash").ghost().small().icon(IconName::Delete))
                    // Use SquareTerminal as a substitute for Monitor
                    .child(Button::new("monitor").ghost().small().icon(IconName::SquareTerminal))
                    .child(Button::new("settings").ghost().small().icon(IconName::Settings)),
            )
    }

    fn render_tree_view(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_h_0()
            .py_1()
            .overflow_y_scrollbar()
            .children(self.projects.iter().map(|project| {
                self.render_project_group(project, cx)
            }))
    }

    fn render_project_group(&self, project: &Project, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let project_id = project.id.clone();
        let project_id_for_toggle = project_id.clone();
        let is_expanded = project.is_expanded;
        let has_worktrees = !project.worktrees.is_empty();
        let can_expand = has_worktrees || is_expanded;
        let has_more_icon = project.has_more_icon;
        let project_name = project.name.clone();

        v_flex()
            .w_full()
            .child(
                h_flex()
                    .id(SharedString::from(format!("project-{}", project_id)))
                    .w_full()
                    .justify_between()
                    .items_center()
                    .px_3()
                    .py_2()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.accent.opacity(0.3)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if can_expand {
                            this.toggle_project(project_id_for_toggle.clone(), cx);
                        }
                    }))
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(if can_expand {
                                if is_expanded {
                                    Icon::new(IconName::ChevronDown)
                                        .size_4()
                                        .text_color(theme.muted_foreground)
                                        .into_any_element()
                                } else {
                                    Icon::new(IconName::ChevronRight)
                                        .size_4()
                                        .text_color(theme.muted_foreground)
                                        .into_any_element()
                                }
                            } else {
                                div().w_4().into_any_element()
                            })
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(theme.foreground)
                                    .child(project_name),
                            ),
                    )
                    .child(if has_more_icon {
                        Icon::new(IconName::ChevronRight)
                            .size_4()
                            .text_color(theme.muted_foreground)
                            .into_any_element()
                    } else {
                        Icon::new(IconName::ChevronDown)
                            .size_4()
                            .text_color(theme.muted_foreground)
                            .opacity(0.)
                            .into_any_element()
                    }),
            )
            .when(is_expanded, |this| {
                this.child(self.render_new_workspace_button(cx))
                    .children(project.worktrees.iter().map(|worktree| {
                        self.render_worktree_item(worktree, cx)
                    }))
            })
    }

    fn render_new_workspace_button(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .px_3()
            .py_1p5()
            .cursor_pointer()
            .hover(|s| s.bg(theme.accent.opacity(0.3)))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .pl_5()
                    .child(
                        Icon::new(IconName::Plus)
                            .size_3p5()
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("New workspace"),
                    ),
            )
            .child(
                Icon::new(IconName::Ellipsis)
                    .size_4()
                    .text_color(theme.muted_foreground)
                    .opacity(0.),
            )
    }

    fn render_worktree_item(&self, worktree: &Worktree, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let worktree_id = worktree.id.clone();
        let worktree_id_for_click = worktree_id.clone();
        let is_selected = self.selected_worktree_id.as_ref() == Some(&worktree_id);
        let is_archive = worktree.status == WorkspaceStatus::Archive;

        v_flex()
            .id(SharedString::from(format!("worktree-{}", worktree_id)))
            .w_full()
            .gap_0p5()
            .px_3()
            .py_2()
            .cursor_pointer()
            .when(is_selected, |s| s.bg(theme.accent))
            .when(!is_selected, |s| s.hover(|s| s.bg(theme.accent.opacity(0.5))))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_worktree(worktree_id_for_click.clone(), cx);
            }))
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .min_w_0()
                            .flex_1()
                            .child(if is_archive {
                                // Use Folder as a substitute for GitMerge
                                Icon::new(IconName::Folder)
                                    .size_4()
                                    .text_color(theme.muted_foreground)
                            } else {
                                // Use File as a substitute for GitBranch
                                Icon::new(IconName::File)
                                    .size_4()
                                    .text_color(theme.muted_foreground)
                            })
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .overflow_x_hidden()
                                    .text_ellipsis()
                                    .child(worktree.branch_name.clone()),
                            ),
                    )
                    .child(self.render_git_stats(worktree.additions, worktree.deletions, cx)),
            )
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .gap_2()
                    .pl_6()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .min_w_0()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(
                                div()
                                    .overflow_x_hidden()
                                    .text_ellipsis()
                                    .child(worktree.base_branch.clone()),
                            )
                            .when_some(worktree.time_ago.clone(), |this, time_ago| {
                                this.child(div().child("·")).child(div().child(time_ago))
                            })
                            .when(worktree.time_ago.is_none(), |this| {
                                this.child(div().child("·"))
                                    .child(self.render_status_badge(worktree.status, cx))
                            }),
                    )
                    .when_some(worktree.shortcut.clone(), |this, shortcut| {
                        this.child(
                            div()
                                .text_xs()
                                .font_family("Consolas")
                                .text_color(theme.muted_foreground)
                                .child(shortcut),
                        )
                    }),
            )
    }

    fn render_git_stats(&self, additions: u32, deletions: u32, _cx: &Context<Self>) -> impl IntoElement {
        let git_add = gpui::rgb(0x22c55e); // green-500
        let git_delete = gpui::rgb(0xef4444); // red-500

        h_flex()
            .gap_1()
            .items_center()
            .font_family("Consolas")
            .text_xs()
            .when(additions > 0, |this| {
                this.child(
                    div()
                        .text_color(git_add)
                        .child(format!("+{}", additions)),
                )
            })
            .when(deletions > 0, |this| {
                this.child(
                    div()
                        .text_color(git_delete)
                        .child(format!("-{}", deletions)),
                )
            })
    }

    fn render_status_badge(&self, status: WorkspaceStatus, _cx: &Context<Self>) -> impl IntoElement {
        let git_ready = gpui::rgb(0x22c55e); // green-500
        let git_conflict = gpui::rgb(0xef4444); // red-500
        let git_archive = gpui::rgb(0x6b7280); // gray-500

        match status {
            WorkspaceStatus::Ready => h_flex()
                .gap_1()
                .items_center()
                .text_xs()
                .text_color(git_ready)
                .child("Ready to merge")
                .into_any_element(),
            WorkspaceStatus::Conflict => h_flex()
                .gap_1()
                .items_center()
                .text_xs()
                .text_color(git_conflict)
                .child("Merge conflicts")
                .into_any_element(),
            WorkspaceStatus::Archive => h_flex()
                .gap_1()
                .items_center()
                .text_xs()
                .text_color(git_archive)
                // Use FolderClosed as a substitute for Archive
                .child(Icon::new(IconName::FolderClosed).size_3())
                .child("Archive")
                .into_any_element(),
            WorkspaceStatus::None => div().into_any_element(),
        }
    }

    fn render_timeline_view(&self, cx: &Context<Self>) -> impl IntoElement {
        // Flatten all worktrees with project names and group by time
        let mut all_worktrees: Vec<(String, Worktree)> = Vec::new();
        for project in &self.projects {
            for worktree in &project.worktrees {
                all_worktrees.push((project.name.clone(), worktree.clone()));
            }
        }

        // Group worktrees by time labels
        let today: Vec<_> = all_worktrees
            .iter()
            .filter(|(_, w)| w.time_ago.is_none() || is_today(&w.time_ago))
            .collect();

        let older: Vec<_> = all_worktrees
            .iter()
            .filter(|(_, w)| w.time_ago.is_some() && !is_today(&w.time_ago))
            .collect();

        v_flex()
            .flex_1()
            .min_h_0()
            .overflow_y_scrollbar()
            .when(!today.is_empty(), |this| {
                this.child(self.render_time_group("Today", &today, cx))
            })
            .when(!older.is_empty(), |this| {
                this.child(self.render_time_group("Older", &older, cx))
            })
    }

    fn render_time_group(
        &self,
        label: &str,
        worktrees: &[&(String, Worktree)],
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .w_full()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .bg(theme.sidebar.opacity(0.95))
                    .border_b_1()
                    .border_color(theme.border.opacity(0.5))
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .text_color(theme.muted_foreground)
                            .child(label.to_uppercase()),
                    ),
            )
            .children(worktrees.iter().map(|(project_name, worktree)| {
                self.render_timeline_item(project_name, worktree, cx)
            }))
    }

    fn render_timeline_item(
        &self,
        project_name: &str,
        worktree: &Worktree,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let worktree_id = worktree.id.clone();
        let worktree_id_for_click = worktree_id.clone();
        let is_selected = self.selected_worktree_id.as_ref() == Some(&worktree_id);

        v_flex()
            .id(SharedString::from(format!("timeline-{}", worktree_id)))
            .w_full()
            .gap_1()
            .px_3()
            .py_2p5()
            .cursor_pointer()
            .border_b_1()
            .border_color(theme.border.opacity(0.5))
            .when(is_selected, |s| s.bg(theme.accent))
            .when(!is_selected, |s| s.hover(|s| s.bg(theme.accent.opacity(0.5))))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_worktree(worktree_id_for_click.clone(), cx);
            }))
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(theme.foreground)
                            .overflow_x_hidden()
                            .text_ellipsis()
                            .child(worktree.branch_name.clone()),
                    )
                    .child(self.render_git_stats(worktree.additions, worktree.deletions, cx)),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(
                        div()
                            .overflow_x_hidden()
                            .text_ellipsis()
                            .child(project_name.to_string()),
                    )
                    .child(div().child("·"))
                    .child(
                        div()
                            .overflow_x_hidden()
                            .text_ellipsis()
                            .child(worktree.base_branch.clone()),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .gap_2()
                    .mt_0p5()
                    .child(self.render_status_badge(worktree.status, cx))
                    .when_some(worktree.shortcut.clone(), |this, shortcut| {
                        this.child(
                            div()
                                .text_xs()
                                .font_family("Consolas")
                                .text_color(theme.muted_foreground)
                                .child(shortcut),
                        )
                    }),
            )
    }
}

fn is_today(time_ago: &Option<String>) -> bool {
    match time_ago {
        None => true,
        Some(t) => {
            let lower = t.to_lowercase();
            lower.contains("min")
                || lower.contains("hour")
                || lower.contains("分钟")
                || lower.contains("小时")
        }
    }
}

impl Focusable for WorkspaceSidebarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WorkspaceSidebarStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .id("workspace-sidebar")
            .track_focus(&self.focus_handle)
            .w(px(320.))
            .h_full()
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .child(self.render_header(cx))
            .child(if self.view_mode == ViewMode::Tree {
                self.render_tree_view(cx).into_any_element()
            } else {
                self.render_timeline_view(cx).into_any_element()
            })
            .child(self.render_footer(cx))
    }
}

#[cfg(feature = "story")]
impl Story for WorkspaceSidebarStory {
    fn title() -> &'static str {
        "Workspace Sidebar"
    }

    fn description() -> &'static str {
        "A workspace sidebar similar to VS Code with project groups, worktrees, and git stats"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn paddings() -> Pixels {
        px(0.)
    }
}

// ============================================================================
// Example Wrapper
// ============================================================================

pub struct Example {
    root: Entity<WorkspaceSidebarStory>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let root = cx.new(|cx| WorkspaceSidebarStory::new(window, cx));
        Self { root }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        h_flex()
            .size_full()
            .bg(theme.background)
            .child(self.root.clone())
            .child(
                // Main content area placeholder
                v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_lg()
                            .text_color(theme.muted_foreground)
                            .child("Main Content Area"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Select a worktree from the sidebar"),
                    ),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window("Workspace Sidebar Example", Example::view, cx);
    });
}
