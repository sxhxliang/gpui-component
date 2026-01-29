//! Collapsible List with virtualized rendering.
//!
//! A high-performance list component that supports collapsible items with virtual rendering.
//! Only visible items are rendered for optimal performance with large datasets.

use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, Context, ElementId, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, Pixels, Render, RenderOnce, ScrollStrategy, SharedString, Size,
    StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};

use crate::{
    IconName, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    list::ListItem,
    scroll::{ScrollableElement, ScrollbarAxis},
    v_flex, v_virtual_list, virtual_list::VirtualListScrollHandle,
};

/// Create a [`CollapsibleList`].
///
/// # Arguments
///
/// * `state` - The shared state managing the collapsible list items.
/// * `render_header` - A closure to render the header of each collapsible item.
/// * `render_content` - A closure to render the content of each collapsible item when expanded.
///
/// # Example
///
/// ```ignore
/// let state = cx.new(|cx| {
///     CollapsibleListState::new(cx).items(vec![
///         CollapsibleListItem::new("section-1", "Section 1")
///             .content_height(px(100.)),
///         CollapsibleListItem::new("section-2", "Section 2")
///             .content_height(px(150.))
///             .expanded(true),
///     ])
/// });
///
/// collapsible_list(&state, |ix, item, expanded, window, cx| {
///     ListItem::new(ix).child(item.label.clone())
/// }, |ix, item, window, cx| {
///     div().child("Content here...")
/// })
/// ```
pub fn collapsible_list<H, C>(
    state: &Entity<CollapsibleListState>,
    render_header: H,
    render_content: C,
) -> CollapsibleList
where
    H: Fn(usize, &CollapsibleListItem, bool, &mut Window, &mut App) -> ListItem + 'static,
    C: Fn(usize, &CollapsibleListItem, &mut Window, &mut App) -> AnyElement + 'static,
{
    CollapsibleList::new(state, render_header, render_content)
}

/// Configuration for an item in the collapsible list.
#[derive(Clone)]
pub struct CollapsibleListItem {
    /// Unique identifier for this item.
    pub id: SharedString,
    /// Label text for the header.
    pub label: SharedString,
    /// Height of the header portion.
    header_height: Pixels,
    /// Height of the content when expanded.
    content_height: Pixels,
    /// Internal state for expanded/disabled.
    state: Rc<RefCell<CollapsibleListItemState>>,
}

#[derive(Clone)]
struct CollapsibleListItemState {
    expanded: bool,
    disabled: bool,
}

impl CollapsibleListItem {
    /// Create a new collapsible list item.
    ///
    /// - `id` - Unique identifier for this item.
    /// - `label` - Display label for the header.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            header_height: px(36.),
            content_height: px(100.),
            state: Rc::new(RefCell::new(CollapsibleListItemState {
                expanded: false,
                disabled: false,
            })),
        }
    }

    /// Set the header height. Default is 36px.
    pub fn header_height(mut self, height: Pixels) -> Self {
        self.header_height = height;
        self
    }

    /// Set the content height when expanded. Default is 100px.
    pub fn content_height(mut self, height: Pixels) -> Self {
        self.content_height = height;
        self
    }

    /// Set the initial expanded state.
    pub fn expanded(self, expanded: bool) -> Self {
        self.state.borrow_mut().expanded = expanded;
        self
    }

    /// Set the disabled state.
    pub fn disabled(self, disabled: bool) -> Self {
        self.state.borrow_mut().disabled = disabled;
        self
    }

    /// Check if this item is expanded.
    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.state.borrow().expanded
    }

    /// Check if this item is disabled.
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.state.borrow().disabled
    }

    /// Get the total height of this item (header + content if expanded).
    #[inline]
    pub fn total_height(&self) -> Pixels {
        if self.is_expanded() {
            self.header_height + self.content_height
        } else {
            self.header_height
        }
    }

    /// Toggle the expanded state.
    fn toggle(&self) {
        let mut state = self.state.borrow_mut();
        state.expanded = !state.expanded;
    }

    /// Set the expanded state.
    fn set_expanded(&self, expanded: bool) {
        self.state.borrow_mut().expanded = expanded;
    }
}

/// State for managing collapsible list items.
pub struct CollapsibleListState {
    focus_handle: FocusHandle,
    items: Vec<CollapsibleListItem>,
    scroll_handle: VirtualListScrollHandle,
    selected_ix: Option<usize>,
    render_header: Rc<dyn Fn(usize, &CollapsibleListItem, bool, &mut Window, &mut App) -> ListItem>,
    render_content: Rc<dyn Fn(usize, &CollapsibleListItem, &mut Window, &mut App) -> AnyElement>,
}

impl CollapsibleListState {
    /// Create a new empty collapsible list state.
    pub fn new(cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            items: Vec::new(),
            scroll_handle: VirtualListScrollHandle::new(),
            selected_ix: None,
            render_header: Rc::new(|_, _, _, _, _| ListItem::new(0)),
            render_content: Rc::new(|_, _, _, _| div().into_any_element()),
        }
    }

    /// Set the items for this list.
    pub fn items(mut self, items: impl Into<Vec<CollapsibleListItem>>) -> Self {
        self.items = items.into();
        self
    }

    /// Set the items for this list (mutable).
    pub fn set_items(&mut self, items: impl Into<Vec<CollapsibleListItem>>, cx: &mut Context<Self>) {
        self.items = items.into();
        self.selected_ix = None;
        cx.notify();
    }

    /// Get all items.
    pub fn items_ref(&self) -> &[CollapsibleListItem] {
        &self.items
    }

    /// Get the number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the currently selected index.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_ix
    }

    /// Set the selected index.
    pub fn set_selected_index(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
        self.selected_ix = ix;
        cx.notify();
    }

    /// Get the selected item.
    pub fn selected_item(&self) -> Option<&CollapsibleListItem> {
        self.selected_ix.and_then(|ix| self.items.get(ix))
    }

    /// Toggle the expanded state of an item at the given index.
    pub fn toggle_expand(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(item) = self.items.get(ix) {
            if !item.is_disabled() {
                item.toggle();
                cx.notify();
            }
        }
    }

    /// Expand an item at the given index.
    pub fn expand(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(item) = self.items.get(ix) {
            if !item.is_disabled() {
                item.set_expanded(true);
                cx.notify();
            }
        }
    }

    /// Collapse an item at the given index.
    pub fn collapse(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(item) = self.items.get(ix) {
            item.set_expanded(false);
            cx.notify();
        }
    }

    /// Expand all items.
    pub fn expand_all(&mut self, cx: &mut Context<Self>) {
        for item in &self.items {
            if !item.is_disabled() {
                item.set_expanded(true);
            }
        }
        cx.notify();
    }

    /// Collapse all items.
    pub fn collapse_all(&mut self, cx: &mut Context<Self>) {
        for item in &self.items {
            item.set_expanded(false);
        }
        cx.notify();
    }

    /// Scroll to an item at the given index.
    pub fn scroll_to_item(&mut self, ix: usize, strategy: ScrollStrategy) {
        self.scroll_handle.scroll_to_item(ix, strategy);
    }

    /// Get the scroll handle.
    pub fn scroll_handle(&self) -> &VirtualListScrollHandle {
        &self.scroll_handle
    }

    /// Calculate item sizes for virtual list rendering.
    fn item_sizes(&self) -> Rc<Vec<Size<Pixels>>> {
        Rc::new(
            self.items
                .iter()
                .map(|item| Size {
                    width: px(0.), // Width will be determined by container
                    height: item.total_height(),
                })
                .collect(),
        )
    }

    fn on_item_click(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_ix = Some(ix);
        self.toggle_expand(ix, cx);
    }
}

impl Render for CollapsibleListState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let render_header = self.render_header.clone();
        let render_content = self.render_content.clone();
        let item_sizes = self.item_sizes();

        div().id("collapsible-list-state").size_full().relative().child(
            v_virtual_list(
                cx.entity().clone(),
                "collapsible-items",
                item_sizes,
                move |state, visible_range: Range<usize>, window, cx| {
                    let mut elements = Vec::with_capacity(visible_range.len());

                    for ix in visible_range {
                        let item = &state.items[ix];
                        let selected = Some(ix) == state.selected_ix;
                        let expanded = item.is_expanded();
                        let disabled = item.is_disabled();

                        let header = (render_header)(ix, item, expanded, window, cx);
                        let content = if expanded {
                            Some((render_content)(ix, item, window, cx))
                        } else {
                            None
                        };

                        let header_height = item.header_height;
                        let content_height = item.content_height;

                        let el = div()
                            .id(ix)
                            .w_full()
                            .child(
                                v_flex()
                                    .w_full()
                                    .child(
                                        div()
                                            .h(header_height)
                                            .w_full()
                                            .child(header.disabled(disabled).selected(selected)),
                                    )
                                    .when_some(content, |this, content| {
                                        this.child(div().h(content_height).w_full().child(content))
                                    }),
                            )
                            .when(!disabled, |this| {
                                this.on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener({
                                        move |this, _, window, cx| {
                                            this.on_item_click(ix, window, cx);
                                        }
                                    }),
                                )
                            });

                        elements.push(el);
                    }

                    elements
                },
            )
            .flex_grow()
            .size_full()
            .track_scroll(&self.scroll_handle)
            .into_any_element(),
        )
    }
}

/// A collapsible list with virtualized rendering.
#[derive(IntoElement)]
pub struct CollapsibleList {
    id: ElementId,
    state: Entity<CollapsibleListState>,
    style: StyleRefinement,
    scrollbar_axis: ScrollbarAxis,
    render_header: Rc<dyn Fn(usize, &CollapsibleListItem, bool, &mut Window, &mut App) -> ListItem>,
    render_content: Rc<dyn Fn(usize, &CollapsibleListItem, &mut Window, &mut App) -> AnyElement>,
}

impl CollapsibleList {
    /// Create a new collapsible list.
    pub fn new<H, C>(state: &Entity<CollapsibleListState>, render_header: H, render_content: C) -> Self
    where
        H: Fn(usize, &CollapsibleListItem, bool, &mut Window, &mut App) -> ListItem + 'static,
        C: Fn(usize, &CollapsibleListItem, &mut Window, &mut App) -> AnyElement + 'static,
    {
        Self {
            id: ElementId::Name(format!("collapsible-list-{}", state.entity_id()).into()),
            state: state.clone(),
            style: StyleRefinement::default(),
            scrollbar_axis: ScrollbarAxis::Vertical,
            render_header: Rc::new(move |ix, item, expanded, window, app| {
                render_header(ix, item, expanded, window, app)
            }),
            render_content: Rc::new(move |ix, item, window, app| {
                render_content(ix, item, window, app)
            }),
        }
    }

    /// Set the scrollbar axis. Default is vertical.
    pub fn scrollbar_axis(mut self, axis: ScrollbarAxis) -> Self {
        self.scrollbar_axis = axis;
        self
    }
}

impl Styled for CollapsibleList {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CollapsibleList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let scroll_handle = self.state.read(cx).scroll_handle.clone();

        self.state.update(cx, |state, _| {
            state.render_header = self.render_header;
            state.render_content = self.render_content;
        });

        div()
            .id(self.id)
            .track_focus(&focus_handle)
            .size_full()
            .child(self.state)
            .refine_style(&self.style)
            .scrollbar(&scroll_handle, self.scrollbar_axis)
    }
}

/// A default header renderer that includes an expand/collapse icon.
///
/// Use this helper function if you want a simple header with an icon indicator.
pub fn default_header_with_icon(
    ix: usize,
    item: &CollapsibleListItem,
    expanded: bool,
    _window: &mut Window,
    _cx: &mut App,
) -> ListItem {
    let icon = if expanded {
        IconName::ChevronDown
    } else {
        IconName::ChevronRight
    };

    ListItem::new(ix)
        .child(
            Button::new(format!("header-{}", ix))
                .icon(icon)
                .ghost()
                .xsmall(),
        )
        .child(item.label.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::AppContext as _;

    #[gpui::test]
    fn test_collapsible_list_item_builder(_cx: &mut gpui::TestAppContext) {
        let item = CollapsibleListItem::new("test-id", "Test Label")
            .header_height(px(40.))
            .content_height(px(200.))
            .expanded(true)
            .disabled(false);

        assert_eq!(item.id.as_ref(), "test-id");
        assert_eq!(item.label.as_ref(), "Test Label");
        assert_eq!(item.header_height, px(40.));
        assert_eq!(item.content_height, px(200.));
        assert!(item.is_expanded());
        assert!(!item.is_disabled());
        assert_eq!(item.total_height(), px(240.));
    }

    #[gpui::test]
    fn test_collapsible_list_item_toggle(_cx: &mut gpui::TestAppContext) {
        let item = CollapsibleListItem::new("test", "Test");

        assert!(!item.is_expanded());
        assert_eq!(item.total_height(), px(36.));

        item.toggle();
        assert!(item.is_expanded());
        assert_eq!(item.total_height(), px(136.));

        item.toggle();
        assert!(!item.is_expanded());
    }

    #[gpui::test]
    fn test_collapsible_list_state(cx: &mut gpui::TestAppContext) {
        let state = cx.new(|cx| {
            CollapsibleListState::new(cx).items(vec![
                CollapsibleListItem::new("item-1", "Item 1"),
                CollapsibleListItem::new("item-2", "Item 2").expanded(true),
                CollapsibleListItem::new("item-3", "Item 3").disabled(true),
            ])
        });

        state.update(cx, |state, cx| {
            assert_eq!(state.len(), 3);
            assert!(!state.is_empty());
            assert!(state.selected_index().is_none());

            // Test item access
            assert!(!state.items[0].is_expanded());
            assert!(state.items[1].is_expanded());
            assert!(state.items[2].is_disabled());

            // Test toggle
            state.toggle_expand(0, cx);
            assert!(state.items[0].is_expanded());

            // Test disabled item cannot be toggled
            state.toggle_expand(2, cx);
            assert!(!state.items[2].is_expanded());

            // Test expand/collapse all
            state.expand_all(cx);
            assert!(state.items[0].is_expanded());
            assert!(state.items[1].is_expanded());
            assert!(!state.items[2].is_expanded()); // Disabled items not affected

            state.collapse_all(cx);
            assert!(!state.items[0].is_expanded());
            assert!(!state.items[1].is_expanded());
        });
    }
}
