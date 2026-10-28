//! Disabled semantics for plain elements. GPUI has no `aria_disabled` setter and gpui-kit keeps its
//! own wrapper private, so this one marks the accessibility node disabled and changes nothing else:
//! no layout node, no focus stop, the element stays clickable (an unavailable swatch explains itself).
//! It has to sit inside gpui-kit's test observation (`test_support()`), or the headless tests could not see the flag.

use gpui_kit::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, InteractiveElement, Interactivity, IntoElement, LayoutId, ParentElement,
    Pixels, StatefulInteractiveElement, StyleRefinement, Styled, Window, accesskit,
};

pub struct Disabled<E> {
    inner: E,
    disabled: bool,
}

impl<E> Disabled<E> {
    pub fn new(inner: E, disabled: bool) -> Self {
        Self { inner, disabled }
    }
}

impl<E: InteractiveElement> InteractiveElement for Disabled<E> {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.inner.interactivity()
    }
}

impl<E: StatefulInteractiveElement> StatefulInteractiveElement for Disabled<E> {}

impl<E: Styled> Styled for Disabled<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl<E: ParentElement> ParentElement for Disabled<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(elements);
    }
}

impl<E: Element> IntoElement for Disabled<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Element> Element for Disabled<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.inner.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.inner.source_location()
    }

    fn request_layout(&mut self, id: Option<&GlobalElementId>, inspector: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, Self::RequestLayoutState) {
        self.inner.request_layout(id, inspector, window, cx)
    }

    fn prepaint(&mut self, id: Option<&GlobalElementId>, inspector: Option<&InspectorElementId>, bounds: Bounds<Pixels>, layout: &mut Self::RequestLayoutState, window: &mut Window, cx: &mut App) -> Self::PrepaintState {
        self.inner.prepaint(id, inspector, bounds, layout, window, cx)
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(&mut self, id: Option<&GlobalElementId>, inspector: Option<&InspectorElementId>, bounds: Bounds<Pixels>, layout: &mut Self::RequestLayoutState, prepaint: &mut Self::PrepaintState, window: &mut Window, cx: &mut App) {
        self.inner.paint(id, inspector, bounds, layout, prepaint, window, cx)
    }

    fn a11y_role(&self) -> Option<gpui_kit::Role> {
        self.inner.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.inner.write_a11y_info(node);
        if self.disabled {
            node.set_disabled();
        }
    }

    fn a11y_synthetic_children(&mut self, prepaint: &mut Self::PrepaintState, builder: &mut gpui_kit::A11ySubtreeBuilder) {
        self.inner.a11y_synthetic_children(prepaint, builder);
    }
}
