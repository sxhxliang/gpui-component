use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    h_flex,
    highlighter::Language,
    input::{Input, InputEvent, InputState, TabSize},
    resizable::{h_resizable, resizable_panel},
    text::markdown,
    v_flex,
};
use gpui_component_assets::Assets;
use gpui_component_story::Open;

/// A cache for rendered mermaid diagrams to avoid re-rendering on each frame.
/// Maps mermaid code hash to the path of the rendered SVG file.
struct MermaidCache {
    cache: Mutex<HashMap<u64, PathBuf>>,
    temp_dir: PathBuf,
}

impl MermaidCache {
    fn new() -> Self {
        let temp_dir = std::env::temp_dir().join("gpui-mermaid-cache");
        std::fs::create_dir_all(&temp_dir).ok();
        Self {
            cache: Mutex::new(HashMap::new()),
            temp_dir,
        }
    }

    fn hash_code(code: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        code.hash(&mut hasher);
        hasher.finish()
    }

    fn get_or_render(&self, code: &str) -> Option<PathBuf> {
        let hash = Self::hash_code(code);

        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(path) = cache.get(&hash) {
                if path.exists() {
                    return Some(path.clone());
                }
            }
        }

        // Render mermaid to SVG
        let svg = mermaid_rs_renderer::render(code).ok()?;

        // Save to temp file
        let path = self.temp_dir.join(format!("{}.svg", hash));
        std::fs::write(&path, &svg).ok()?;

        // Update cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(hash, path.clone());
        }

        Some(path)
    }
}

// Global mermaid cache
static MERMAID_CACHE: std::sync::LazyLock<MermaidCache> =
    std::sync::LazyLock::new(MermaidCache::new);

pub struct Example {
    input_state: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

const EXAMPLE: &str = include_str!("./fixtures/test.md");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(Language::Markdown)
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 2,
                    ..Default::default()
                })
                .searchable(true)
                .placeholder("Enter your Markdown here...")
                .default_value(EXAMPLE)
        });

        let _subscriptions = vec![cx.subscribe(&input_state, |_, _, _: &InputEvent, _| {})];

        Self {
            input_state,
            _subscriptions,
        }
    }

    fn on_action_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        let path = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: Some("Select a Markdown file".into()),
        });

        let input_state = self.input_state.clone();
        cx.spawn_in(window, async move |_, window| {
            let path = path.await.ok()?.ok()??.iter().next()?.clone();

            let content = std::fs::read_to_string(&path).ok()?;

            window
                .update(|window, cx| {
                    _ = input_state.update(cx, |this, cx| {
                        this.set_value(content, window, cx);
                    });
                })
                .ok();

            Some(())
        })
        .detach();
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("editor")
            .size_full()
            .on_action(cx.listener(Self::on_action_open))
            .child(
                h_resizable("container")
                    .child(
                        resizable_panel().child(
                            div()
                                .id("source")
                                .size_full()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_size(cx.theme().mono_font_size)
                                .child(
                                    Input::new(&self.input_state)
                                        .h_full()
                                        .p_0()
                                        .border_0()
                                        .focus_bordered(false),
                                ),
                        ),
                    )
                    .child(
                        resizable_panel().child(
                            markdown(self.input_state.read(cx).value().clone())
                                .code_block_renderer(|code_block, _window, cx| {
                                    let lang = code_block.lang();

                                    // Only handle mermaid code blocks
                                    if lang.as_ref().map(|l| l.as_ref()) != Some("mermaid") {
                                        return None;
                                    }

                                    let code = code_block.code();
                                    let svg_path = MERMAID_CACHE.get_or_render(code.as_ref())?;

                                    // Return the rendered mermaid diagram as an image
                                    Some(
                                        v_flex()
                                            .w_full()
                                            .p_3()
                                            .rounded(cx.theme().radius)
                                            .bg(cx.theme().background)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .child(img(svg_path).max_w_full())
                                            .into_any_element(),
                                    )
                                })
                                .code_block_actions(|code_block, _window, _cx| {
                                    let code = code_block.code();
                                    let lang = code_block.lang();

                                    h_flex()
                                        .gap_1()
                                        .child(Clipboard::new("copy").value(code.clone()))
                                        .when_some(lang, |this, lang| {
                                            // Only show run terminal button for certain languages
                                            if lang.as_ref() == "rust" || lang.as_ref() == "python"
                                            {
                                                this.child(
                                                    Button::new("run-terminal")
                                                        .icon(IconName::SquareTerminal)
                                                        .ghost()
                                                        .xsmall()
                                                        .on_click(move |_, _, _cx| {
                                                            println!(
                                                                "Running {} code: {}",
                                                                lang, code
                                                            );
                                                        }),
                                                )
                                            } else {
                                                this
                                            }
                                        })
                                })
                                .flex_none()
                                .p_5()
                                .scrollable(true)
                                .selectable(true),
                        ),
                    ),
            )
    }
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window("Markdown Editor", Example::view, cx);
    });
}
