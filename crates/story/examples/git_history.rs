use gpui::{*, prelude::FluentBuilder as _};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    Icon,
    menu::DropdownMenu,
    table::{Column, ColumnFixed, Table, TableDelegate, TableState},
    v_flex, ActiveTheme as _, IconName, Root, Sizable, StyledExt,
};
use gpui_component_assets::Assets;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

actions!(git_history, [NoAction]);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UncommittedFile {
    path: String,
    added: usize,
    removed: usize,
}

// 树节点：可以是文件夹或文件
#[derive(Clone, Debug)]
enum FileTreeNode {
    Folder {
        name: String,
        children: Vec<FileTreeNode>,
    },
    File {
        name: String,
        added: usize,
        removed: usize,
    },
}

impl FileTreeNode {
    fn name(&self) -> &str {
        match self {
            FileTreeNode::Folder { name, .. } => name,
            FileTreeNode::File { name, .. } => name,
        }
    }

    fn is_folder(&self) -> bool {
        matches!(self, FileTreeNode::Folder { .. })
    }
}

// 将文件路径列表转换为树形结构
fn build_file_tree(files: &[UncommittedFile]) -> Vec<FileTreeNode> {
    let mut root: HashMap<String, Vec<(Vec<String>, &UncommittedFile)>> = HashMap::new();

    // 按路径分组
    for file in files {
        let parts: Vec<String> = file.path.split('/').map(|s| s.to_string()).collect();
        if !parts.is_empty() {
            let first = parts[0].clone();
            root.entry(first).or_default().push((parts, file));
        }
    }

    // 构建树
    let mut tree = Vec::new();
    for (name, items) in root {
        tree.push(build_tree_recursive(name, items));
    }

    tree.sort_by(|a, b| {
        // 文件夹优先
        match (a.is_folder(), b.is_folder()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name().cmp(b.name()),
        }
    });

    tree
}

fn build_tree_recursive(
    name: String,
    items: Vec<(Vec<String>, &UncommittedFile)>,
) -> FileTreeNode {
    // 如果所有项都只有一个部分，说明是文件
    if items.iter().all(|(parts, _)| parts.len() == 1) {
        // 应该只有一个文件
        if let Some((_, file)) = items.first() {
            return FileTreeNode::File {
                name,
                added: file.added,
                removed: file.removed,
            };
        }
    }

    // 否则是文件夹，继续递归
    let mut children_map: HashMap<String, Vec<(Vec<String>, &UncommittedFile)>> = HashMap::new();

    for (parts, file) in items {
        if parts.len() > 1 {
            let next = parts[1].clone();
            let remaining: Vec<String> = parts.iter().skip(1).cloned().collect();
            children_map.entry(next).or_default().push((remaining, file));
        }
    }

    let mut children = Vec::new();
    for (child_name, child_items) in children_map {
        children.push(build_tree_recursive(child_name, child_items));
    }

    children.sort_by(|a, b| {
        match (a.is_folder(), b.is_folder()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name().cmp(b.name()),
        }
    });

    FileTreeNode::Folder { name, children }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct GitCommit {
    id: String,
    short_id: String,
    description: String,
    author: String,
    date: String,
    branch: Option<String>,
    is_uncommitted: bool,
    uncommitted_files: Vec<UncommittedFile>,
}

impl GitCommit {
    fn new(
        id: &str,
        description: &str,
        author: &str,
        date: &str,
        branch: Option<&str>,
    ) -> Self {
        Self {
            id: id.to_string(),
            short_id: id.chars().take(7).collect(),
            description: description.to_string(),
            author: author.to_string(),
            date: date.to_string(),
            branch: branch.map(|s| s.to_string()),
            is_uncommitted: false,
            uncommitted_files: vec![],
        }
    }

    fn uncommitted(files: Vec<UncommittedFile>) -> Self {
        let count = files.len();
        Self {
            id: String::new(),
            short_id: String::new(),
            description: format!("Uncommitted Changes ({})", count),
            author: "*".to_string(),
            date: "22 Dec 2025 17:48".to_string(),
            branch: None,
            is_uncommitted: true,
            uncommitted_files: files,
        }
    }
}

// 模拟 git 历史数据
fn mock_git_data() -> Vec<GitCommit> {
    vec![
        GitCommit::uncommitted(vec![
            UncommittedFile {
                path: ".claude/settings.local.json".to_string(),
                added: 2,
                removed: 1,
            },
            UncommittedFile {
                path: "src/components/agent_message.rs".to_string(),
                added: 54,
                removed: 82,
            },
            UncommittedFile {
                path: "src/components/chat_input_box.rs".to_string(),
                added: 1,
                removed: 1,
            },
            UncommittedFile {
                path: "src/components/mod.rs".to_string(),
                added: 3,
                removed: 0,
            },
            UncommittedFile {
                path: "src/components/status_indicator.rs".to_string(),
                added: 0,
                removed: 0,
            },
            UncommittedFile {
                path: "src/components/tool_call_item.rs".to_string(),
                added: 16,
                removed: 10,
            },
            UncommittedFile {
                path: "src/core/services/agent_service.rs".to_string(),
                added: 5,
                removed: 1,
            },
        ]),
        GitCommit::new(
            "99d0373a",
            "update claude.md",
            "sxhxliang",
            "22 Dec 2025 10:38",
            None,
        ),
        GitCommit::new(
            "e3bf49a2",
            "WIP on (no branch): 6e9c76f feat(chat): refactor command suggestions popover to use v_flex layout",
            "sxhxliang",
            "22 Dec 2025 10:23",
            Some("stash@{0}"),
        ),
        GitCommit::new(
            "9a384766",
            "WIP on dev: 6e9c76f feat(chat): refactor command suggestions popover to use v_flex layout",
            "sxhxliang",
            "22 Dec 2025 10:15",
            Some("stash@{1}"),
        ),
        GitCommit::new(
            "6e9c76f8",
            "feat(chat): refactor command suggestions popover to use v_flex layout",
            "Shihua Liang",
            "22 Dec 2025 01:13",
            Some("dev"),
        ),
        GitCommit::new(
            "125a750c",
            "为了解决 AvailableCommandsUpdate 到达时找不到 session 对应 agent 的问题，我把 session_id 从 agent 中移除",
            "Shihua Liang",
            "22 Dec 2025 00:08",
            None,
        ),
        GitCommit::new(
            "4fd7a250",
            "删除模拟数据",
            "Shihua Liang",
            "21 Dec 2025 23:54",
            None,
        ),
        GitCommit::new(
            "7b012446",
            "fix bug",
            "Shihua Liang",
            "20 Dec 2025 21:20",
            None,
        ),
        GitCommit::new(
            "2507a20e",
            "input: Update paint argument for GPUI API change. (#1839)",
            "contributors",
            "19 Dec 2025 15:42",
            None,
        ),
        GitCommit::new(
            "c47cbfc1",
            "example: Add a system_monitor example. (#1835)",
            "contributors",
            "18 Dec 2025 10:30",
            None,
        ),
        GitCommit::new(
            "969df4d4",
            "table: Add `min_width`, `max_width` for limit column resizing. (#1831)",
            "contributors",
            "17 Dec 2025 09:15",
            None,
        ),
        GitCommit::new(
            "d53f6f53",
            "dock: Fix the active index not working when restoring from cache (#1832)",
            "contributors",
            "16 Dec 2025 14:20",
            None,
        ),
        GitCommit::new(
            "1a80c43d",
            "button: Use variant color for outline button hover bg. (#1830)",
            "contributors",
            "15 Dec 2025 11:45",
            None,
        ),
    ]
}

// 扁平化树节点，用于表格显示
#[derive(Clone, Debug)]
struct FlatTreeNode {
    name: String,
    depth: usize,
    is_folder: bool,
    added: usize,
    removed: usize,
}

fn flatten_tree(nodes: &[FileTreeNode], depth: usize) -> Vec<FlatTreeNode> {
    let mut result = Vec::new();

    for node in nodes {
        match node {
            FileTreeNode::Folder { name, children } => {
                result.push(FlatTreeNode {
                    name: name.clone(),
                    depth,
                    is_folder: true,
                    added: 0,
                    removed: 0,
                });
                result.extend(flatten_tree(children, depth + 1));
            }
            FileTreeNode::File {
                name,
                added,
                removed,
            } => {
                result.push(FlatTreeNode {
                    name: name.clone(),
                    depth,
                    is_folder: false,
                    added: *added,
                    removed: *removed,
                });
            }
        }
    }

    result
}

struct GitHistoryDelegate {
    commits: Vec<GitCommit>,
    columns: Vec<Column>,
    expanded_row: Option<usize>,
    file_tree_cache: Option<Vec<FlatTreeNode>>,
}

impl GitHistoryDelegate {
    fn new() -> Self {
        Self {
            commits: mock_git_data(),
            columns: vec![
                Column::new("graph", "Graph")
                    .width(60.)
                    .fixed(ColumnFixed::Left),
                Column::new("description", "Description")
                    .width(600.)
                    .fixed(ColumnFixed::Left),
                Column::new("date", "Date").width(180.),
                Column::new("author", "Author").width(120.),
                Column::new("commit", "Commit").width(100.),
            ],
            expanded_row: None,
            file_tree_cache: None,
        }
    }

    fn toggle_row(&mut self, row_ix: usize) {
        if self.expanded_row == Some(row_ix) {
            self.expanded_row = None;
            self.file_tree_cache = None;
        } else {
            self.expanded_row = Some(row_ix);
            // 构建树并缓存
            if let Some(commit) = self.commits.get(row_ix) {
                if commit.is_uncommitted {
                    let tree = build_file_tree(&commit.uncommitted_files);
                    self.file_tree_cache = Some(flatten_tree(&tree, 0));
                }
            }
        }
    }

    fn get_expanded_row_count(&self) -> usize {
        if let Some(_) = self.expanded_row {
            self.file_tree_cache.as_ref().map_or(0, |t| t.len())
        } else {
            0
        }
    }
}

impl TableDelegate for GitHistoryDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.commits.len() + self.get_expanded_row_count()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let col = &self.columns[col_ix];
        div()
            .px_2()
            .py_1()
            .text_sm()
            .font_semibold()
            .child(col.name.clone())
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        // 计算实际的提交索引和展开行索引
        let expanded_count = self.get_expanded_row_count();
        let first_expanded_row = self.expanded_row.unwrap_or(usize::MAX);

        if self.expanded_row.is_some() && row_ix > first_expanded_row && row_ix <= first_expanded_row + expanded_count {
            // 这是展开的文件行
            let tree_ix = row_ix - first_expanded_row - 1;
            return self.render_tree_node_row(tree_ix, col_ix, cx);
        }

        // 普通提交行
        let actual_commit_ix = if row_ix > first_expanded_row {
            row_ix - expanded_count
        } else {
            row_ix
        };

        let commit = &self.commits[actual_commit_ix];
        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "graph" => div()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(12.))
                        .h(px(12.))
                        .rounded_full()
                        .when(commit.is_uncommitted, |this| {
                            this.border_2().border_color(cx.theme().border)
                        })
                        .when(!commit.is_uncommitted, |this| {
                            this.bg(cx.theme().primary)
                        }),
                )
                .into_any_element(),
            "description" => {
                let is_expanded = self.expanded_row == Some(actual_commit_ix);
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(commit.is_uncommitted, |this| {
                        this.child(
                            Button::new("expand-toggle")
                                .xsmall()
                                .ghost()
                                .icon(if is_expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                })
                                .on_click(cx.listener(move |table, _, _, cx| {
                                    table.delegate_mut().toggle_row(actual_commit_ix);
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        div()
                            .text_sm()
                            .when(commit.is_uncommitted, |this| {
                                this.font_semibold()
                            })
                            .child(commit.description.clone()),
                    )
                    .when(commit.branch.is_some(), |this| {
                        this.child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded_sm()
                                .bg(cx.theme().primary.alpha(0.1))
                                .text_color(cx.theme().primary)
                                .text_xs()
                                .child(commit.branch.as_ref().unwrap().clone()),
                        )
                    })
                    .into_any_element()
            }
            "date" => div()
                .h_full()
                .flex()
                .items_center()
                .text_sm()
                .child(commit.date.clone())
                .into_any_element(),
            "author" => div()
                .h_full()
                .flex()
                .items_center()
                .text_sm()
                .child(commit.author.clone())
                .into_any_element(),
            "commit" => div()
                .h_full()
                .flex()
                .items_center()
                .text_sm()
                .when(!commit.short_id.is_empty(), |this| {
                    this.child(commit.short_id.clone())
                })
                .when(commit.is_uncommitted, |this| this.child("*"))
                .into_any_element(),
            _ => div().into_any_element(),
        }
    }
}

impl GitHistoryDelegate {
    fn render_tree_node_row(
        &self,
        tree_ix: usize,
        col_ix: usize,
        cx: &mut Context<TableState<Self>>,
    ) -> AnyElement {
        let Some(tree_nodes) = &self.file_tree_cache else {
            return div().into_any_element();
        };

        let Some(node) = tree_nodes.get(tree_ix) else {
            return div().into_any_element();
        };

        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "graph" => div()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(2.))
                        .h_full()
                        .bg(cx.theme().border.alpha(0.3)),
                )
                .into_any_element(),
            "description" => {
                let indent = node.depth * 20 + 40;

                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .pl(px(indent as f32))
                    .gap_2()
                    .child(
                        div()
                            .w(px(16.))
                            .h(px(16.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(if node.is_folder {
                                    IconName::Folder
                                } else {
                                    IconName::File
                                })
                                .text_color(cx.theme().muted_foreground)
                                .xsmall(),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(node.name.clone()),
                    )
                    .when(!node.is_folder && (node.added > 0 || node.removed > 0), |this| {
                        this.child(
                            div()
                                .flex()
                                .gap_1()
                                .text_xs()
                                .when(node.added > 0, |this| {
                                    this.child(
                                        div()
                                            .text_color(cx.theme().green)
                                            .child(format!("+{}", node.added)),
                                    )
                                })
                                .when(node.removed > 0, |this| {
                                    this.child(
                                        div()
                                            .text_color(cx.theme().red)
                                            .child(format!("-{}", node.removed)),
                                    )
                                }),
                        )
                    })
                    .into_any_element()
            }
            _ => div().into_any_element(),
        }
    }
}

struct GitHistoryView {
    table_state: Entity<TableState<GitHistoryDelegate>>,
    show_remote_branches: bool,
    selected_branch: String,
}

impl GitHistoryView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let table_state = cx.new(|cx| TableState::new(GitHistoryDelegate::new(), window, cx));

        Self {
            table_state,
            show_remote_branches: true,
            selected_branch: "Show All".to_string(),
        }
    }
}

impl Render for GitHistoryView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // 顶部工具栏
                h_flex()
                    .h(px(40.))
                    .px_4()
                    .gap_4()
                    .items_center()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child("Branches:"),
                    )
                    .child(
                        Button::new("branch-dropdown")
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(self.selected_branch.clone())
                                    .child(Icon::new(IconName::ChevronDown).xsmall()),
                            )
                            .dropdown_menu(|menu, _, _| {
                                menu.menu("Show All", Box::new(NoAction))
                                    .separator()
                                    .menu("main", Box::new(NoAction))
                                    .menu("dev", Box::new(NoAction))
                                    .menu("feature/new-ui", Box::new(NoAction))
                            }),
                    )
                    .child(
                        Checkbox::new("show-remote")
                            .checked(self.show_remote_branches)
                            .label("Show Remote Branches")
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.show_remote_branches = *checked;
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("search")
                            .ghost()
                            .icon(IconName::Search)
                            .small(),
                    )
                    .child(
                        Button::new("filter")
                            .ghost()
                            .icon(IconName::Menu)
                            .small(),
                    )
                    .child(
                        Button::new("settings")
                            .ghost()
                            .icon(IconName::Settings)
                            .small(),
                    )
                    .child(
                        Button::new("refresh")
                            .ghost()
                            .icon(IconName::Redo2)
                            .small(),
                    ),
            )
            .child(
                // Git 历史表格
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(Table::new(&self.table_state).stripe(true)),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        let window_size = size(px(1200.0), px(800.0));
        let window_bounds = Bounds::centered(None, window_size, cx);

        cx.spawn(async move |cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                #[cfg(not(target_os = "linux"))]
                titlebar: Some(gpui_component::TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(800.),
                    height: px(600.),
                }),
                kind: WindowKind::Normal,
                ..Default::default()
            };

            cx.open_window(options, |window, cx| {
                let view = cx.new(|cx| GitHistoryView::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
