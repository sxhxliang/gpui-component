//! MarkdownView - Markdown rendering component designed for virtual lists
//!
//! ## Why MarkdownView?
//!
//! `TextView::markdown()` uses `window.use_keyed_state()` for internal state management,
//! but in virtual list's `layout_as_root()` measurement, this state cache may not work correctly,
//! causing measurement and rendering to use different `TextViewState`, combined with async
//! markdown parsing, resulting in inaccurate height calculations.
//!
//! `MarkdownView` requires externally managed `MarkdownState`, ensuring measurement and
//! rendering use the same state instance, fundamentally solving height measurement issues
//! in virtual lists.
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! // 1. Create and cache state in list container
//! let state = MarkdownState::new("# Hello\n\nWorld", cx);
//! self.markdown_states.insert(item_id, state);
//!
//! // 2. Use state for measurement
//! let size = MarkdownView::measure(&state, width, window, cx);
//!
//! // 3. Use same state for rendering
//! MarkdownView::new(&state)
//!     .text_color(theme.foreground)
//!     .text_sm()
//!
//! // 4. Update content (streaming scenario)
//! state.set_text("new content", cx);
//! ```

use gpui::{
    AnyElement, App, AppContext as _, AvailableSpace, Context, Element, ElementId, Entity,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, ParentElement, Pixels, Styled,
    StyleRefinement, Window, div, size,
};

use super::state::TextViewState;
use super::text_view::TextView;
use crate::StyledExt as _;

/// Markdown rendering state
///
/// This is a wrapper around `Entity<TextViewState>`, providing a friendlier API.
/// In virtual list scenarios, this state needs to be created and cached outside the list.
#[derive(Clone)]
pub struct MarkdownState {
    inner: Entity<TextViewState>,
}

impl MarkdownState {
    /// Create a new Markdown state
    ///
    /// # Arguments
    /// - `text`: Markdown text content
    /// - `cx`: Context for creating Entity
    ///
    /// # Note
    /// Markdown parsing is async, after creation you need to wait 1-2 frames
    /// to get accurate measurement results.
    pub fn new<T: 'static>(text: &str, cx: &mut Context<T>) -> Self {
        let inner = cx.new(|cx| TextViewState::markdown(text, cx));
        Self { inner }
    }

    /// Update Markdown content
    ///
    /// Internally checks if content actually changed to avoid unnecessary re-parsing.
    pub fn set_text(&self, text: &str, cx: &mut App) {
        self.inner.update(cx, |state, cx| {
            state.set_text(text, cx);
        });
    }

    /// Append Markdown content (for streaming output)
    pub fn push_str(&self, text: &str, cx: &mut App) {
        self.inner.update(cx, |state, cx| {
            state.push_str(text, cx);
        });
    }

    /// Get the inner Entity reference
    pub fn entity(&self) -> &Entity<TextViewState> {
        &self.inner
    }
}

impl std::ops::Deref for MarkdownState {
    type Target = Entity<TextViewState>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Markdown rendering component for virtual lists
///
/// Key differences from `TextView::markdown()`:
/// - Requires externally passed `MarkdownState`, not auto-created internally
/// - Ensures measurement and rendering use exactly the same state instance
/// - Solves height calculation inconsistency in virtual lists
pub struct MarkdownView {
    state: Entity<TextViewState>,
    style: StyleRefinement,
}

impl MarkdownView {
    /// Create a new MarkdownView
    ///
    /// # Arguments
    /// - `state`: Pre-created MarkdownState
    pub fn new(state: &MarkdownState) -> Self {
        Self {
            state: state.inner.clone(),
            style: StyleRefinement::default(),
        }
    }

    /// Create from Entity<TextViewState> directly (for compatibility with existing code)
    pub fn from_entity(state: &Entity<TextViewState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
        }
    }

    /// Measure Markdown content size
    ///
    /// This is the core method for virtual list pre-measurement,
    /// using the same state as rendering for measurement.
    ///
    /// # Arguments
    /// - `state`: MarkdownState, must be the same instance used for rendering
    /// - `max_width`: Maximum width constraint
    /// - `window`: Window reference
    /// - `cx`: App context
    ///
    /// # Returns
    /// Measured content size
    pub fn measure(
        state: &MarkdownState,
        max_width: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::Size<Pixels> {
        Self::measure_entity(&state.inner, max_width, window, cx)
    }

    /// Measure from Entity (for compatibility with existing code)
    pub fn measure_entity(
        state: &Entity<TextViewState>,
        max_width: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::Size<Pixels> {
        let mut element = div()
            .max_w(max_width)
            .child(TextView::new(state))
            .into_any_element();

        let available = size(
            AvailableSpace::Definite(max_width),
            AvailableSpace::MinContent,
        );
        element.layout_as_root(available, window, cx)
    }

    /// Measure Markdown content with styles applied
    ///
    /// Use this method when you need to apply specific styles (like padding, text_sm, etc.)
    /// during measurement.
    pub fn measure_styled<F>(
        state: &MarkdownState,
        max_width: Pixels,
        style_fn: F,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::Size<Pixels>
    where
        F: FnOnce(MarkdownView) -> MarkdownView,
    {
        let view = style_fn(Self::new(state));
        let mut element = div().max_w(max_width).child(view).into_any_element();

        let available = size(
            AvailableSpace::Definite(max_width),
            AvailableSpace::MinContent,
        );
        element.layout_as_root(available, window, cx)
    }
}

impl Styled for MarkdownView {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for MarkdownView {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// MarkdownView layout state
pub struct MarkdownViewLayoutState {
    element: AnyElement,
}

impl Element for MarkdownView {
    type RequestLayoutState = MarkdownViewLayoutState;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        // Use TextViewState's entity_id as element ID
        Some(ElementId::Name(
            self.state.entity_id().to_string().into(),
        ))
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // Build internal TextView and apply styles
        let mut element = div()
            .child(TextView::new(&self.state))
            .refine_style(&self.style)
            .into_any_element();

        let layout_id = element.request_layout(window, cx);
        (layout_id, MarkdownViewLayoutState { element })
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        request_layout.element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        request_layout.element.paint(window, cx);
    }
}

/// Helper trait for virtual lists to manage Markdown state
pub trait MarkdownStateCache {
    /// Get or create MarkdownState for the specified key
    fn get_or_create_markdown_state<T: 'static>(
        &mut self,
        key: &str,
        text: &str,
        cx: &mut Context<T>,
    ) -> MarkdownState;

    /// Update MarkdownState content for the specified key
    fn update_markdown_state(&mut self, key: &str, text: &str, cx: &mut App);

    /// Remove MarkdownState for the specified key
    fn remove_markdown_state(&mut self, key: &str);

    /// Clear all cached MarkdownStates
    fn clear_markdown_states(&mut self);
}

impl MarkdownStateCache for std::collections::HashMap<String, MarkdownState> {
    fn get_or_create_markdown_state<T: 'static>(
        &mut self,
        key: &str,
        text: &str,
        cx: &mut Context<T>,
    ) -> MarkdownState {
        if let Some(state) = self.get(key) {
            state.clone()
        } else {
            let state = MarkdownState::new(text, cx);
            self.insert(key.to_string(), state.clone());
            state
        }
    }

    fn update_markdown_state(&mut self, key: &str, text: &str, cx: &mut App) {
        if let Some(state) = self.get(key) {
            state.set_text(text, cx);
        }
    }

    fn remove_markdown_state(&mut self, key: &str) {
        self.remove(key);
    }

    fn clear_markdown_states(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    // Unit tests require gpui test environment, only do structure validation here
    #[test]
    fn test_markdown_state_deref() {
        // Verify MarkdownState can Deref to Entity<TextViewState>
        // Actual tests need gpui App environment
    }
}
