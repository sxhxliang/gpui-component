use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
use image::{ImageBuffer, Rgba};
use smallvec::SmallVec;

/// A cache for rendered mermaid diagrams to avoid re-rendering on each frame.
/// Maps mermaid code hash to the rendered MermaidImage.
struct MermaidCache {
    cache: Mutex<HashMap<u64, MermaidImage>>,
}

impl MermaidCache {
    fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn hash_code(code: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        code.hash(&mut hasher);
        hasher.finish()
    }

    fn get_or_render(&self, code: &str) -> Option<MermaidImage> {
        let hash = Self::hash_code(code);

        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(mermaid_image) = cache.get(&hash) {
                return Some(MermaidImage {
                    image: mermaid_image.image.clone(),
                    display_width: mermaid_image.display_width,
                    display_height: mermaid_image.display_height,
                });
            }
        }

        // Render mermaid to SVG
        let svg_string = mermaid_rs_renderer::render(code).ok()?;

        // Parse and render SVG to pixels using resvg
        let mermaid_image = svg_to_render_image(&svg_string)?;

        // Update cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(
                hash,
                MermaidImage {
                    image: mermaid_image.image.clone(),
                    display_width: mermaid_image.display_width,
                    display_height: mermaid_image.display_height,
                },
            );
        }

        Some(mermaid_image)
    }
}

/// Render scale factor for high-DPI displays (matches GPUI's SMOOTH_SVG_SCALE_FACTOR)
const RENDER_SCALE: f32 = 2.0;

/// Cached usvg Options with system fonts loaded (loading system fonts is expensive).
static USVG_OPTIONS: std::sync::LazyLock<usvg::Options> = std::sync::LazyLock::new(|| {
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    opt
});

/// Rendered mermaid image with its original (pre-scale) dimensions
struct MermaidImage {
    image: Arc<RenderImage>,
    /// Original width before scaling (for display sizing)
    display_width: u32,
    /// Original height before scaling (for display sizing)
    display_height: u32,
}

/// Convert SVG string to RenderImage using resvg, rendered at 2x for sharpness
fn svg_to_render_image(svg: &str) -> Option<MermaidImage> {
    let tree = usvg::Tree::from_str(svg, &USVG_OPTIONS).ok()?;

    // Get SVG size (original display size)
    let size = tree.size().to_int_size();
    let display_width = size.width();
    let display_height = size.height();

    // Render at higher resolution for sharpness on high-DPI displays
    let render_width = (display_width as f32 * RENDER_SCALE) as u32;
    let render_height = (display_height as f32 * RENDER_SCALE) as u32;

    // Create pixmap for rendering at scaled size
    let mut pixmap = resvg::tiny_skia::Pixmap::new(render_width, render_height)?;

    // Fill with white background
    pixmap.fill(resvg::tiny_skia::Color::WHITE);

    // Render SVG to pixmap with scale transform
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(RENDER_SCALE, RENDER_SCALE),
        &mut pixmap.as_mut(),
    );

    // Convert from RGBA (premultiplied alpha) to BGRA format that GPUI expects
    let mut pixels = pixmap.take();
    for pixel in pixels.chunks_exact_mut(4) {
        // resvg outputs premultiplied RGBA, GPUI expects non-premultiplied BGRA
        // First unpremultiply alpha, then swap R and B
        let a = pixel[3];
        if a > 0 && a < 255 {
            let a_f = a as f32 / 255.0;
            pixel[0] = (pixel[0] as f32 / a_f).min(255.0) as u8;
            pixel[1] = (pixel[1] as f32 / a_f).min(255.0) as u8;
            pixel[2] = (pixel[2] as f32 / a_f).min(255.0) as u8;
        }
        // Swap R and B (RGBA -> BGRA)
        pixel.swap(0, 2);
    }

    // Create ImageBuffer from raw pixels (at scaled size)
    let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(render_width, render_height, pixels)?;

    // Create RenderImage from Frame
    let frame = image::Frame::new(buffer);
    let render_image = RenderImage::new(SmallVec::from_elem(frame, 1));

    Some(MermaidImage {
        image: Arc::new(render_image),
        display_width,
        display_height,
    })
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
                                    let mermaid = MERMAID_CACHE.get_or_render(code.as_ref())?;

                                    let w = px(mermaid.display_width as f32);
                                    let h = px(mermaid.display_height as f32);

                                    // Display the 2x rendered image at original SVG dimensions
                                    Some(
                                        v_flex()
                                            .w_full()
                                            .p_3()
                                            .rounded(cx.theme().radius)
                                            .bg(cx.theme().background)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .child(
                                                img(mermaid.image)
                                                    .max_w(w)
                                                    .max_h(h),
                                            )
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
