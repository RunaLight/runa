use std::sync::Mutex;

use glam::Vec2;
use runa_render_api::FontId;

use crate::{
    components::UiRenderer,
    ui::{
        Anchor, ContainerHandle, ImageHandle, InteractionState, LayoutProps, SliderHandle,
        StyleProps, TextHandle, UiNodeId, UiNodeKind,
    },
};

pub struct UiNodeBuilder<'a> {
    pub renderer: &'a mut UiRenderer,
    id: UiNodeId,
}

impl<'a> UiNodeBuilder<'a> {
    pub fn new(renderer: &'a mut UiRenderer, id: UiNodeId) -> Self {
        Self { renderer, id }
    }

    pub fn new_with_click(
        renderer: &'a mut UiRenderer,
        id: UiNodeId,
        on_click: Option<Box<dyn FnMut() + Send>>,
    ) -> Self {
        if let Some(mut cb) = on_click {
            if let Some(node) = renderer.node_mut(id) {
                node.interaction_callback = Some(Mutex::new(Box::new(move |state| {
                    if state == InteractionState::Clicked {
                        cb();
                    }
                })));
            }
        }
        Self { renderer, id }
    }

    pub fn id(&self) -> UiNodeId {
        self.id
    }

    /// Finish building and return the raw node id.
    pub fn build(self) -> UiNodeId {
        self.id
    }

    /// Finish building and return a typed text handle.
    pub fn into_text(self) -> TextHandle {
        TextHandle(self.id)
    }

    /// Finish building and return a typed image handle.
    pub fn into_image(self) -> ImageHandle {
        ImageHandle(self.id)
    }

    /// Finish building and return a typed slider handle.
    pub fn into_slider(self) -> SliderHandle {
        SliderHandle(self.id)
    }

    /// Finish building and return a typed container/panel handle.
    pub fn into_container(self) -> ContainerHandle {
        ContainerHandle(self.id)
    }

    pub fn named(self, name: impl Into<String>) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.name = name.into();
        }
        self
    }

    pub fn with_layout(self, layout: LayoutProps) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.layout = layout;
        }
        self
    }

    pub fn with_style(self, style: StyleProps) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.style = style;
        }
        self
    }

    /// Apply a StyleSheet to this node (background, opacity, z_index, padding, margin)
    pub fn with_style_sheet(self, sheet: &crate::ui::StyleSheet) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            sheet.apply_to(node);
        }
        self
    }

    pub fn with_anchor(self, anchor: Anchor) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.layout.anchor = anchor;
        }
        self
    }

    /// Fill parent in both axes (sets Anchor::Stretch).
    /// Combine with `with_margin()` to inset from parent edges.
    pub fn with_fill(self) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.layout.anchor = Anchor::Stretch;
        }
        self
    }

    pub fn with_pos(self, x: f32, y: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.layout.position = Vec2::new(x, y);
        }
        self
    }

    pub fn with_size(self, w: f32, h: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.layout.min_size = Vec2::new(w, h);
            node.layout.max_size = Vec2::new(w, h);
        }
        self
    }

    pub fn with_min_size(self, w: f32, h: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.layout.min_size = Vec2::new(w, h);
        }
        self
    }

    pub fn with_max_size(self, w: f32, h: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.layout.max_size = Vec2::new(w, h);
        }
        self
    }

    pub fn with_background(self, r: f32, g: f32, b: f32, a: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.style.background = Some([r, g, b, a]);
        }
        self
    }

    pub fn with_z_index(self, z: i16) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.style.z_index = z;
        }
        self
    }

    pub fn with_opacity(self, opacity: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.style.opacity = opacity;
        }
        self
    }

    pub fn with_gap(self, gap: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.layout.gap = gap;
        }
        self
    }

    pub fn with_padding(self, l: f32, t: f32, r: f32, b: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.layout.padding = crate::ui::EdgeInsets {
                left: l,
                top: t,
                right: r,
                bottom: b,
            };
        }
        self
    }

    pub fn with_margin(self, l: f32, t: f32, r: f32, b: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.layout.margin = crate::ui::EdgeInsets {
                left: l,
                top: t,
                right: r,
                bottom: b,
            };
        }
        self
    }

    /// For text nodes: set font size (in pixels, or world units for World-space UI)
    pub fn with_font_size(self, size: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            if let UiNodeKind::Text(ref mut props) = node.kind {
                props.font_size = size;
            }
        }
        self
    }

    /// For text nodes: set custom font
    pub fn with_font(self, font: FontId) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            if let UiNodeKind::Text(ref mut props) = node.kind {
                props.font = Some(font);
            }
        }
        self
    }

    /// For text nodes: set color
    pub fn with_text_color(self, r: f32, g: f32, b: f32, a: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            if let UiNodeKind::Text(ref mut props) = node.kind {
                let c = [r, g, b, a];
                props.color = c;
                for seg in &mut props.segments {
                    seg.color = c;
                }
            }
        }
        self
    }

    /// For slider nodes: set value
    pub fn with_slider_value(self, value: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            if let UiNodeKind::Slider(ref mut props) = node.kind {
                props.value = value.clamp(props.min, props.max);
            }
        }
        self
    }

    /// For slider nodes: set range
    pub fn with_slider_range(self, min: f32, max: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            if let UiNodeKind::Slider(ref mut props) = node.kind {
                props.min = min;
                props.max = max;
                props.value = props.value.clamp(min, max);
            }
        }
        self
    }

    /// Set interaction callback (called when interaction state changes)
    pub fn with_on_interact<F>(self, callback: F) -> Self
    where
        F: FnMut(InteractionState) + Send + 'static,
    {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.interaction_callback = Some(Mutex::new(Box::new(callback)));
        }
        self
    }

    /// Set click callback (fires once on mouse release over the node)
    pub fn with_on_click<F>(self, mut callback: F) -> Self
    where
        F: FnMut() + Send + 'static,
    {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.interaction_callback = Some(Mutex::new(Box::new(move |state| {
                if state == InteractionState::Clicked {
                    callback();
                }
            })));
        }
        self
    }

    /// For image nodes: set tint
    pub fn with_tint(self, r: f32, g: f32, b: f32, a: f32) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            if let UiNodeKind::Image(ref mut props) = node.kind {
                props.tint = [r, g, b, a];
            }
        }
        self
    }

    /// For image nodes: set texture handle
    pub fn with_texture(self, texture: runa_asset::Handle<runa_asset::TextureAsset>) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            if let UiNodeKind::Image(ref mut props) = node.kind {
                props.texture = Some(texture);
            }
        }
        self
    }

    /// Set visibility
    pub fn visible(self, visible: bool) -> Self {
        if let Some(node) = self.renderer.node_mut(self.id) {
            node.visible = visible;
        }
        self
    }

    /// Returns the node's computed rect (must call layout() first)
    pub fn rect(&self) -> Option<crate::ui::UiRect> {
        self.renderer.node(self.id).map(|n| n.computed.rect)
    }

    /// Pop parent stack (only valid for container/vbox/hbox nodes that were pushed)
    pub fn end(self) {
        // pop parent stack — only if this node is the current top
        if self.renderer.parent_stack.last() == Some(&self.id) {
            self.renderer.parent_stack.pop();
        }
    }
}
