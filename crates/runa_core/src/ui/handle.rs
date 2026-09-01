use crate::components::UiRenderer;
use crate::ui::{
    Anchor, ContainerKind, EdgeInsets, ImageProps, SliderProps, TextProps, UiNode, UiNodeId,
    UiNodeKind,
};
use glam::Vec2;
use runa_asset::{Handle, TextureAsset};

/// Typed handle to a text node. Cheap (`Copy`): store it anywhere and edit
/// the node at runtime via `&mut UiRenderer`. No strings, no dynamic dispatch.
#[derive(Clone, Copy, Debug)]
pub struct TextHandle(pub UiNodeId);

/// Typed handle to an image node. `set_color`/`set_tint` edit the image tint.
#[derive(Clone, Copy, Debug)]
pub struct ImageHandle(pub UiNodeId);

/// Typed handle to a slider node.
#[derive(Clone, Copy, Debug)]
pub struct SliderHandle(pub UiNodeId);

/// Typed handle to a container/panel/vbox/hbox node.
#[derive(Clone, Copy, Debug)]
pub struct ContainerHandle(pub UiNodeId);

macro_rules! impl_handle_common {
    ($h:ty) => {
        impl $h {
            pub fn id(&self) -> UiNodeId {
                self.0
            }

            pub fn set_visible(&self, ui: &mut UiRenderer, v: bool) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.visible = v;
                }
            }

            pub fn set_opacity(&self, ui: &mut UiRenderer, o: f32) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.style.opacity = o;
                }
            }

            pub fn set_background(&self, ui: &mut UiRenderer, c: [f32; 4]) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.style.background = Some(c);
                }
            }

            pub fn set_z_index(&self, ui: &mut UiRenderer, z: i16) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.style.z_index = z;
                }
            }

            pub fn set_pos(&self, ui: &mut UiRenderer, x: f32, y: f32) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.layout.position = Vec2::new(x, y);
                    ui.dirty_layout = true;
                }
            }

            pub fn set_size(&self, ui: &mut UiRenderer, w: f32, h: f32) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.layout.min_size = Vec2::new(w, h);
                    n.layout.max_size = Vec2::new(w, h);
                    ui.dirty_layout = true;
                }
            }

            pub fn set_min_size(&self, ui: &mut UiRenderer, w: f32, h: f32) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.layout.min_size = Vec2::new(w, h);
                    ui.dirty_layout = true;
                }
            }

            pub fn set_max_size(&self, ui: &mut UiRenderer, w: f32, h: f32) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.layout.max_size = Vec2::new(w, h);
                    ui.dirty_layout = true;
                }
            }

            pub fn set_anchor(&self, ui: &mut UiRenderer, a: Anchor) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.layout.anchor = a;
                    ui.dirty_layout = true;
                }
            }

            pub fn set_padding(&self, ui: &mut UiRenderer, l: f32, t: f32, r: f32, b: f32) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.layout.padding = EdgeInsets {
                        left: l,
                        top: t,
                        right: r,
                        bottom: b,
                    };
                    ui.dirty_layout = true;
                }
            }

            pub fn set_margin(&self, ui: &mut UiRenderer, l: f32, t: f32, r: f32, b: f32) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.layout.margin = EdgeInsets {
                        left: l,
                        top: t,
                        right: r,
                        bottom: b,
                    };
                    ui.dirty_layout = true;
                }
            }

            pub fn set_gap(&self, ui: &mut UiRenderer, g: f32) {
                if let Some(n) = ui.node_mut(self.0) {
                    n.layout.gap = g;
                    ui.dirty_layout = true;
                }
            }

            /// Raw node access (read or mutate `style`/`layout`/`visible`).
            pub fn get_node<'a>(&self, ui: &'a UiRenderer) -> Option<&'a UiNode> {
                ui.node(self.0)
            }

            pub fn get_node_mut<'a>(&self, ui: &'a mut UiRenderer) -> Option<&'a mut UiNode> {
                ui.node_mut(self.0)
            }
        }
    };
}

impl_handle_common!(TextHandle);
impl_handle_common!(ImageHandle);
impl_handle_common!(SliderHandle);
impl_handle_common!(ContainerHandle);

impl TextHandle {
    /// Replace text content (recomputes rich-text segments).
    pub fn set_text(&self, ui: &mut UiRenderer, text: &str) {
        if let Some(n) = ui.node_mut(self.0) {
            if let UiNodeKind::Text(p) = &mut n.kind {
                p.text = text.to_string();
                p.segments = crate::ui::parse_rich_text(text);
            }
        }
    }

    pub fn set_color(&self, ui: &mut UiRenderer, c: [f32; 4]) {
        if let Some(n) = ui.node_mut(self.0) {
            if let UiNodeKind::Text(p) = &mut n.kind {
                p.color = c;
                for seg in &mut p.segments {
                    seg.color = c;
                }
            }
        }
    }

    pub fn set_font_size(&self, ui: &mut UiRenderer, s: f32) {
        if let Some(n) = ui.node_mut(self.0) {
            if let UiNodeKind::Text(p) = &mut n.kind {
                p.font_size = s;
            }
        }
    }

    pub fn get_props<'a>(&self, ui: &'a mut UiRenderer) -> Option<&'a mut TextProps> {
        match ui.node_mut(self.0) {
            Some(n) => match &mut n.kind {
                UiNodeKind::Text(p) => Some(p),
                _ => None,
            },
            None => None,
        }
    }
}

impl ImageHandle {
    /// Edit the image tint.
    pub fn set_color(&self, ui: &mut UiRenderer, c: [f32; 4]) {
        if let Some(n) = ui.node_mut(self.0) {
            if let UiNodeKind::Image(p) = &mut n.kind {
                p.tint = c;
            }
        }
    }

    pub fn set_tint(&self, ui: &mut UiRenderer, c: [f32; 4]) {
        self.set_color(ui, c);
    }

    pub fn set_texture(&self, ui: &mut UiRenderer, texture: Handle<TextureAsset>) {
        if let Some(n) = ui.node_mut(self.0) {
            if let UiNodeKind::Image(p) = &mut n.kind {
                p.texture = Some(texture);
            }
        }
    }

    pub fn get_props<'a>(&self, ui: &'a mut UiRenderer) -> Option<&'a mut ImageProps> {
        match ui.node_mut(self.0) {
            Some(n) => match &mut n.kind {
                UiNodeKind::Image(p) => Some(p),
                _ => None,
            },
            None => None,
        }
    }
}

impl SliderHandle {
    pub fn set_value(&self, ui: &mut UiRenderer, v: f32) {
        if let Some(n) = ui.node_mut(self.0) {
            if let UiNodeKind::Slider(p) = &mut n.kind {
                p.value = v.clamp(p.min, p.max);
            }
        }
    }

    pub fn set_range(&self, ui: &mut UiRenderer, min: f32, max: f32) {
        if let Some(n) = ui.node_mut(self.0) {
            if let UiNodeKind::Slider(p) = &mut n.kind {
                p.min = min;
                p.max = max;
                p.value = p.value.clamp(min, max);
            }
        }
    }

    pub fn get_props<'a>(&self, ui: &'a mut UiRenderer) -> Option<&'a mut SliderProps> {
        match ui.node_mut(self.0) {
            Some(n) => match &mut n.kind {
                UiNodeKind::Slider(p) => Some(p),
                _ => None,
            },
            None => None,
        }
    }
}

impl ContainerHandle {
    pub fn get_props<'a>(&self, ui: &'a mut UiRenderer) -> Option<&'a mut ContainerKind> {
        match ui.node_mut(self.0) {
            Some(n) => match &mut n.kind {
                UiNodeKind::Container(k) => Some(k),
                _ => None,
            },
            None => None,
        }
    }
}
