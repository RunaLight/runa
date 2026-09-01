use std::sync::Arc;

use crate::components::{Camera, Transform};
use crate::resources::input::InputState;
use crate::ui::UiNodeBuilder;
use crate::ui::{
    Anchor, CanvasSpace, ContainerKind, EdgeInsets, ImageProps, InteractionState, RichTextSegment,
    SliderProps, TextProps, UiNode, UiNodeId, UiNodeKind,
};
use glam::Vec2;
use runa_render_api::{command::UiRect as RenderUiRect, RenderQueue};

pub struct UiRenderer {
    pub root_node_path: Option<String>,
    pub ui_asset_path: Option<String>,
    pub space: CanvasSpace,
    pub nodes: Vec<UiNode>,
    pub root: UiNodeId,
    pub dirty_layout: bool,
    pub debug_show_bounds: bool,
    pub parent_stack: Vec<UiNodeId>,
    interaction_pressed_node: Option<UiNodeId>,
    interaction_was_pressed: bool,
    screen_scale: Vec2,
}

impl UiRenderer {
    pub fn new(space: CanvasSpace) -> Self {
        let root = UiNodeId(0);
        let root_node =
            UiNode::new(root, None, UiNodeKind::Container(ContainerKind::Free)).named("root");

        Self {
            root_node_path: None,
            ui_asset_path: None,
            space,
            nodes: vec![root_node],
            root,
            dirty_layout: true,
            debug_show_bounds: false,
            parent_stack: Vec::new(),
            interaction_pressed_node: None,
            interaction_was_pressed: false,
            screen_scale: Vec2::new(1.0, 1.0),
        }
    }
    pub fn clear(&mut self) {
        let root_node =
            UiNode::new(self.root, None, UiNodeKind::Container(ContainerKind::Free)).named("root");
        self.nodes.clear();
        self.nodes.push(root_node);
        self.parent_stack.clear();
        self.dirty_layout = true;
        self.screen_scale = Vec2::new(1.0, 1.0);
    }

    pub fn root(&self) -> UiNodeId {
        self.root
    }

    // ── Parent stack ─────────────────────────────────────────────

    fn current_parent(&self) -> UiNodeId {
        self.parent_stack.last().copied().unwrap_or(self.root)
    }

    pub fn push_parent(&mut self, id: UiNodeId) {
        self.parent_stack.push(id);
    }

    /// Pop parent stack and return the popped node's id.
    /// Call this after done adding children to a container created with `begin_*`.
    pub fn pop_parent(&mut self) -> Option<UiNodeId> {
        self.parent_stack.pop()
    }

    // ── Builder API (parent stack version) ───────────────────────

    pub fn begin_vbox(&mut self) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::VerticalBox));
        self.push_parent(id);
        UiNodeBuilder::new(self, id)
    }

    pub fn begin_hbox(&mut self) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::HorizontalBox));
        self.push_parent(id);
        UiNodeBuilder::new(self, id)
    }

    pub fn begin_container(&mut self) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::Free));
        self.push_parent(id);
        UiNodeBuilder::new(self, id)
    }

    pub fn add_hbox(&mut self) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::HorizontalBox));
        UiNodeBuilder::new(self, id)
    }

    pub fn add_vbox(&mut self) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::VerticalBox));
        UiNodeBuilder::new(self, id)
    }

    pub fn add_text(&mut self, content: impl Into<String>) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        self.text(parent, content)
    }

    pub fn add_rich_text(&mut self, content: &str) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        self.add_rich_text_in(parent, content)
    }

    pub fn add_image(&mut self) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        self.image(parent)
    }

    pub fn add_button(
        &mut self,
        label: Option<impl Into<String>>,
        on_click: Option<Box<dyn FnMut() + Send>>,
    ) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        self.button(parent, label, on_click)
    }

    pub fn add_slider(&mut self) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        self.slider(parent)
    }

    // ── Closure-based container API (egui-style) ─────────────────
    //
    // These methods create a container, push it onto the parent stack,
    // call the closure (where children are added), and pop back.
    //
    // Usage:
    //   ui.vbox(|ui| {
    //       ui.text("Hello");
    //       ui.hbox(|ui| { ui.text("A"); ui.text("B"); });
    //   });

    pub fn vbox(&mut self, f: impl FnOnce(&mut Self)) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::VerticalBox));
        self.push_parent(id);
        f(self);
        self.pop_parent();
        UiNodeBuilder::new(self, id)
    }

    pub fn hbox(&mut self, f: impl FnOnce(&mut Self)) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::HorizontalBox));
        self.push_parent(id);
        f(self);
        self.pop_parent();
        UiNodeBuilder::new(self, id)
    }

    pub fn container(&mut self, f: impl FnOnce(&mut Self)) -> UiNodeBuilder<'_> {
        let parent = self.current_parent();
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::Free));
        self.push_parent(id);
        f(self);
        self.pop_parent();
        UiNodeBuilder::new(self, id)
    }

    // ── Builder API (explicit parent, low-level) ─────────────────

    pub fn container_in(&mut self, parent: UiNodeId) -> UiNodeBuilder<'_> {
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::Free));
        UiNodeBuilder::new(self, id)
    }

    pub fn hbox_in(&mut self, parent: UiNodeId) -> UiNodeBuilder<'_> {
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::HorizontalBox));
        UiNodeBuilder::new(self, id)
    }

    pub fn vbox_in(&mut self, parent: UiNodeId) -> UiNodeBuilder<'_> {
        let id = self.add_node(parent, UiNodeKind::Container(ContainerKind::VerticalBox));
        UiNodeBuilder::new(self, id)
    }

    pub fn text(&mut self, parent: UiNodeId, content: impl Into<String>) -> UiNodeBuilder<'_> {
        let text = content.into();
        let segments = crate::ui::parse_rich_text(&text);
        let props = TextProps {
            text,
            segments,
            font: None,
            font_size: 16.0,
            color: [1.0, 1.0, 1.0, 1.0],
            line_height: None,
            align: crate::ui::TextAlign::Left,
        };
        let id = self.add_node(parent, UiNodeKind::Text(props));
        UiNodeBuilder::new(self, id)
    }

    pub fn add_rich_text_in(&mut self, parent: UiNodeId, content: &str) -> UiNodeBuilder<'_> {
        let segments = crate::ui::parse_rich_text(content);
        let props = TextProps {
            text: content.to_string(),
            segments,
            font: None,
            font_size: 16.0,
            color: [1.0, 1.0, 1.0, 1.0],
            line_height: None,
            align: crate::ui::TextAlign::Left,
        };
        let id = self.add_node(parent, UiNodeKind::Text(props));
        UiNodeBuilder::new(self, id)
    }

    pub fn slider(&mut self, parent: UiNodeId) -> UiNodeBuilder<'_> {
        let id = self.add_node(parent, UiNodeKind::Slider(SliderProps::default()));
        UiNodeBuilder::new(self, id)
    }

    pub fn image(&mut self, parent: UiNodeId) -> UiNodeBuilder<'_> {
        let props = ImageProps {
            texture: None,
            tint: [1.0, 1.0, 1.0, 1.0],
            uv: [0.0, 0.0, 1.0, 1.0],
        };
        let id = self.add_node(parent, UiNodeKind::Image(props));
        UiNodeBuilder::new(self, id)
    }

    /// Creates a clickable button (container + optional text + optional image)
    pub fn button(
        &mut self,
        parent: UiNodeId,
        label: Option<impl Into<String>>,
        on_click: Option<Box<dyn FnMut() + Send>>,
    ) -> UiNodeBuilder<'_> {
        let btn_id = self.add_node(parent, UiNodeKind::Container(ContainerKind::Free));

        if let Some(text) = label {
            let text_id = self.add_node(
                btn_id,
                UiNodeKind::Text(TextProps {
                    text: text.into(),
                    segments: vec![],
                    font: None,
                    font_size: 16.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                    line_height: None,
                    align: crate::ui::TextAlign::Center,
                }),
            );
            if let Some(node) = self.node_mut(text_id) {
                node.layout.anchor = Anchor::Center;
                node.layout.min_size = Vec2::new(0.0, 0.0);
            }
        }

        UiNodeBuilder::new_with_click(self, btn_id, on_click)
    }

    // ── Node lookup ──────────────────────────────────────────────

    /// Find a node by name, searching recursively
    pub fn find_by_name(&self, name: &str) -> Option<UiNodeId> {
        self.find_by_name_recursive(self.root, name)
    }

    fn find_by_name_recursive(&self, id: UiNodeId, name: &str) -> Option<UiNodeId> {
        if let Some(node) = self.node(id) {
            if node.name == name {
                return Some(id);
            }
            for child in &node.children {
                if let Some(found) = self.find_by_name_recursive(*child, name) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Find nodes by name, returns all matches
    pub fn find_all_by_name(&self, name: &str) -> Vec<UiNodeId> {
        let mut results = Vec::new();
        self.find_all_by_name_recursive(self.root, name, &mut results);
        results
    }

    fn find_all_by_name_recursive(&self, id: UiNodeId, name: &str, results: &mut Vec<UiNodeId>) {
        if let Some(node) = self.node(id) {
            if node.name == name {
                results.push(id);
            }
            for child in &node.children {
                self.find_all_by_name_recursive(*child, name, results);
            }
        }
    }

    // ── Low-level helpers (bypass parent stack) ──────────────────

    pub fn add_container_node(&mut self, parent: UiNodeId, kind: ContainerKind) -> UiNodeId {
        self.add_node(parent, UiNodeKind::Container(kind))
    }

    pub fn add_text_node(&mut self, parent: UiNodeId, props: TextProps) -> UiNodeId {
        self.add_node(parent, UiNodeKind::Text(props))
    }

    /// Update the text content of an existing text node.
    /// Does nothing if `id` is not a text node.
    pub fn set_text(&mut self, id: UiNodeId, content: impl Into<String>) {
        if let Some(UiNodeKind::Text(props)) = self.node_mut(id).map(|n| &mut n.kind) {
            let text = content.into();
            props.text = text.clone();
            props.segments = crate::ui::parse_rich_text(&text);
        }
    }

    pub fn add_image_node(&mut self, parent: UiNodeId, props: ImageProps) -> UiNodeId {
        self.add_node(parent, UiNodeKind::Image(props))
    }

    fn add_node(&mut self, parent: UiNodeId, kind: UiNodeKind) -> UiNodeId {
        let parent_index = parent.0 as usize;
        assert!(
            parent_index < self.nodes.len(),
            "Invalid parent id: {:?}",
            parent
        );

        let id = UiNodeId(self.nodes.len() as u32);
        let node = UiNode::new(id, Some(parent), kind);

        self.nodes.push(node);
        self.nodes[parent_index].children.push(id);
        self.dirty_layout = true;

        id
    }

    pub fn node(&self, id: UiNodeId) -> Option<&UiNode> {
        self.nodes.get(id.0 as usize)
    }

    pub fn node_mut(&mut self, id: UiNodeId) -> Option<&mut UiNode> {
        self.nodes.get_mut(id.0 as usize)
    }

    // ── Layout ───────────────────────────────────────────────────

    pub fn layout(&mut self, viewport_size: Vec2, camera: Option<&Camera>) {
        let (virtual_size, scale) = match self.space {
            CanvasSpace::Screen => (viewport_size, Vec2::new(1.0, 1.0)),
            CanvasSpace::Camera => {
                if let Some(cam) = camera {
                    let vs_height = cam.orthographic_size.y;
                    // virtual width = height * aspect
                    let vs_x = vs_height * (viewport_size.x / viewport_size.y);
                    let virtual_size = Vec2::new(vs_x, vs_height);

                    let s = viewport_size.y / vs_height;
                    (virtual_size, Vec2::splat(s))
                } else {
                    (viewport_size, Vec2::new(1.0, 1.0))
                }
            }
            CanvasSpace::World => {
                // World-space UI uses the orthographic size as layout space
                // then projects to screen via camera matrix during render
                let vs = camera
                    .map(|cam| cam.orthographic_size)
                    .unwrap_or(viewport_size);
                (vs, Vec2::new(1.0, 1.0))
            }
        };

        let font_scale: f32 = match self.space {
            CanvasSpace::Screen => 1.0,
            CanvasSpace::Camera => 1.0 / scale.y,
            // World-space UI lays out in world units: `font_size` is a height in
            // the orthographic space and `world_rect_to_screen` scales it to pixels
            // by the camera zoom. Using 1.0 keeps the layout rect in world units so
            // the text grows/shrinks with the camera (same as sprites) instead of
            // staying a constant pixel size.
            CanvasSpace::World => 1.0,
        };

        // Step 1: collect parent-child relationships and compute sizes (immutable read)
        let node_count = self.nodes.len();
        let mut sizes: Vec<(f32, f32)> = vec![(virtual_size.x, virtual_size.y); node_count];
        let mut parent_ids: Vec<Option<usize>> = vec![None; node_count];
        let mut anchors: Vec<Anchor> = vec![Anchor::TopLeft; node_count];
        let mut positions: Vec<Vec2> = vec![Vec2::ZERO; node_count];
        let mut kinds: Vec<UiNodeKind> = Vec::new();

        let viewport_center = Vec2::new(virtual_size.x * 0.5, virtual_size.y * 0.5);

        for (i, node) in self.nodes.iter().enumerate() {
            if i == 0 {
                sizes[i] = (virtual_size.x, virtual_size.y);
                parent_ids[i] = None;
                anchors[i] = Anchor::TopLeft;
                positions[i] = viewport_center;
                kinds.push(UiNodeKind::Container(ContainerKind::Free));
                continue;
            }

            let mut w = node.computed.rect.w;
            let mut h = node.computed.rect.h;

            match &node.kind {
                UiNodeKind::Image(props) => {
                    h = virtual_size.y * 0.2;
                    if let Some(handle) = &props.texture {
                        let image_arc: Arc<runa_asset::TextureAsset> = handle.clone().into();
                        if image_arc.height > 0 {
                            let aspect = image_arc.width as f32 / image_arc.height as f32;
                            w = (h * aspect).max(1.0);
                        } else {
                            w = h;
                        }
                    } else {
                        w = h;
                    }
                }
                UiNodeKind::Text(props) => {
                    h = props.font_size * font_scale;
                    let char_est = props.font_size * font_scale * 0.5;
                    w = (props.text.len() as f32) * char_est;
                }
                UiNodeKind::Slider(_) => {
                    h = 20.0;
                    w = w.max(100.0);
                }
                UiNodeKind::Container(_) => {
                    if w <= 0.0 || h <= 0.0 {
                        w = virtual_size.x;
                        h = virtual_size.y;
                    }
                }
            }

            let min = node.layout.min_size;
            let max = node.layout.max_size;
            let max_w = max.x.min(virtual_size.x).max(min.x);
            let max_h = max.y.min(virtual_size.y).max(min.y);
            w = w.clamp(min.x, max_w);
            h = h.clamp(min.y, max_h);

            sizes[i] = (w, h);
            parent_ids[i] = node.parent.map(|pid| pid.0 as usize).filter(|&p| p != 0);
            anchors[i] = node.layout.anchor;
            positions[i] = node.layout.position;
            kinds.push(match &node.kind {
                UiNodeKind::Container(ck) => UiNodeKind::Container(*ck),
                UiNodeKind::Text(_) => UiNodeKind::Text(TextProps {
                    text: String::new(),
                    segments: vec![],
                    font: None,
                    font_size: 0.0,
                    color: [0.0; 4],
                    line_height: None,
                    align: crate::ui::TextAlign::Left,
                }),
                UiNodeKind::Image(_) => UiNodeKind::Image(ImageProps {
                    texture: None,
                    tint: [0.0; 4],
                    uv: [0.0; 4],
                }),
                UiNodeKind::Slider(props) => UiNodeKind::Slider(*props),
            });
        }

        // Step 2: build parent rects from computed sizes
        let mut rects: Vec<(f32, f32, f32, f32)> = vec![(0.0, 0.0, 0.0, 0.0); node_count];
        rects[0] = (viewport_center.x, viewport_center.y, sizes[0].0, sizes[0].1);

        for i in 1..node_count {
            let (w, h) = sizes[i];
            let anchor = anchors[i];
            let pos = positions[i];
            let margin = self.nodes[i].layout.margin;

            let parent_idx = parent_ids[i].unwrap_or(0);
            let (pcx, pcy, pw, ph) = rects[parent_idx];

            // Margin offsets from the anchor direction
            let margin_ox = match anchor {
                Anchor::TopLeft | Anchor::Left | Anchor::BottomLeft => margin.left,
                Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => 0.0,
                Anchor::TopRight | Anchor::Right | Anchor::BottomRight => -margin.right,
                Anchor::Stretch => 0.0,
            };
            let margin_oy = match anchor {
                Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => margin.top,
                Anchor::Left | Anchor::Center | Anchor::Right => 0.0,
                Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => -margin.bottom,
                Anchor::Stretch => 0.0,
            };
            let pos = pos + glam::Vec2::new(margin_ox, margin_oy);

            let cx = match anchor {
                Anchor::TopLeft => pcx - pw * 0.5 + pos.x + w * 0.5,
                Anchor::TopCenter => pcx + pos.x,
                Anchor::TopRight => pcx + pw * 0.5 - pos.x - w * 0.5,
                Anchor::Left => pcx - pw * 0.5 + pos.x + w * 0.5,
                Anchor::Center => pcx + pos.x,
                Anchor::Right => pcx + pw * 0.5 - pos.x - w * 0.5,
                Anchor::BottomLeft => pcx - pw * 0.5 + pos.x + w * 0.5,
                Anchor::BottomCenter => pcx + pos.x,
                Anchor::BottomRight => pcx + pw * 0.5 - pos.x - w * 0.5,
                Anchor::Stretch => pcx,
            };

            let cy = match anchor {
                Anchor::TopLeft => pcy - ph * 0.5 + pos.y + h * 0.5,
                Anchor::TopCenter => pcy - ph * 0.5 + pos.y + h * 0.5,
                Anchor::TopRight => pcy - ph * 0.5 + pos.y + h * 0.5,
                Anchor::Left => pcy + pos.y,
                Anchor::Center => pcy + pos.y,
                Anchor::Right => pcy + pos.y,
                Anchor::BottomLeft => pcy + ph * 0.5 - pos.y - h * 0.5,
                Anchor::BottomCenter => pcy + ph * 0.5 - pos.y - h * 0.5,
                Anchor::BottomRight => pcy + ph * 0.5 - pos.y - h * 0.5,
                Anchor::Stretch => pcy,
            };

            let final_w = if matches!(anchor, Anchor::Stretch) {
                pw - margin.left - margin.right - pos.x * 2.0
            } else {
                w
            };
            let final_h = if matches!(anchor, Anchor::Stretch) {
                ph - margin.top - margin.bottom - pos.y * 2.0
            } else {
                h
            };
            rects[i] = (cx, cy, final_w, final_h);
        }

        // Step 3: handle auto-layout for hbox/vbox
        for i in 0..node_count {
            if let UiNodeKind::Container(ck) = &kinds[i] {
                match ck {
                    ContainerKind::HorizontalBox | ContainerKind::VerticalBox => {
                        let children: Vec<usize> = self.nodes[i]
                            .children
                            .iter()
                            .map(|id| id.0 as usize)
                            .collect();
                        if children.is_empty() {
                            continue;
                        }
                        let is_horizontal = matches!(ck, ContainerKind::HorizontalBox);
                        let gap = self.nodes[i].layout.gap;
                        let padding = self.nodes[i].layout.padding;
                        let (pcx, pcy, pw, ph) = rects[i];

                        let available = if is_horizontal {
                            pw - padding.left - padding.right
                        } else {
                            ph - padding.top - padding.bottom
                        };

                        // Compute minimum base size for each child (not inflated rect size)
                        let mut margins: Vec<EdgeInsets> = Vec::with_capacity(children.len());
                        let mut base_sizes = Vec::with_capacity(children.len());
                        let mut total_min = 0.0f32;
                        for &ci in &children {
                            let (min_val, mgn) =
                                if let Some(child) = self.node(UiNodeId(ci as u32)) {
                                    let base =
                                        match &child.kind {
                                            UiNodeKind::Text(props) => {
                                                let raw = if is_horizontal {
                                                    (props.text.len() as f32)
                                                        * props.font_size
                                                        * 0.5
                                                        * font_scale
                                                } else {
                                                    props.font_size * font_scale
                                                };
                                                raw.max(if is_horizontal {
                                                    child.layout.min_size.x
                                                } else {
                                                    child.layout.min_size.y
                                                })
                                                .min(if is_horizontal {
                                                    child.layout.max_size.x
                                                } else {
                                                    child.layout.max_size.y
                                                })
                                            }
                                            UiNodeKind::Slider(_) => {
                                                if is_horizontal {
                                                    child.layout.min_size.x.max(100.0)
                                                } else {
                                                    30.0
                                                }
                                            }
                                            UiNodeKind::Image(_) => {
                                                if is_horizontal {
                                                    child.computed.rect.w.max(1.0)
                                                } else {
                                                    child.computed.rect.h.max(1.0)
                                                }
                                            }
                                            UiNodeKind::Container(_) => {
                                                if is_horizontal {
                                                    child
                                                        .computed
                                                        .rect
                                                        .w
                                                        .max(child.layout.min_size.x)
                                                        .max(1.0)
                                                } else {
                                                    child
                                                        .computed
                                                        .rect
                                                        .h
                                                        .max(child.layout.min_size.y)
                                                        .max(1.0)
                                                }
                                            }
                                        };
                                    let m = child.layout.margin;
                                    (base.max(0.0), m)
                                } else {
                                    (0.0, EdgeInsets::ZERO)
                                };
                            // Include margin in the effective size (space reserved in layout)
                            let effective = if is_horizontal {
                                min_val + mgn.left + mgn.right
                            } else {
                                min_val + mgn.top + mgn.bottom
                            };
                            margins.push(mgn);
                            base_sizes.push(min_val);
                            total_min += effective;
                        }

                        let spacing = if children.len() > 1 {
                            gap * (children.len() - 1) as f32
                        } else {
                            0.0
                        };
                        let remaining = (available - spacing - total_min).max(0.0);
                        let extra = if !children.is_empty() {
                            remaining / children.len() as f32
                        } else {
                            0.0
                        };

                        let mut offset = if is_horizontal {
                            pcx - pw * 0.5 + padding.left
                        } else {
                            pcy - ph * 0.5 + padding.top
                        };

                        for (idx, &ci) in children.iter().enumerate() {
                            let base = base_sizes[idx];
                            let mgn = margins[idx];
                            let effective = if is_horizontal {
                                base + mgn.left + mgn.right
                            } else {
                                base + mgn.top + mgn.bottom
                            };
                            let final_effective = effective + extra;
                            let content_size = (final_effective
                                - (if is_horizontal {
                                    mgn.left + mgn.right
                                } else {
                                    mgn.top + mgn.bottom
                                }))
                            .max(0.0);
                            let content_size = self
                                .node(UiNodeId(ci as u32))
                                .map(|n| {
                                    let main_min = if is_horizontal {
                                        n.layout.min_size.x
                                    } else {
                                        n.layout.min_size.y
                                    };
                                    let main_max = if is_horizontal {
                                        n.layout.max_size.x
                                    } else {
                                        n.layout.max_size.y
                                    };
                                    content_size.max(main_min).min(main_max)
                                })
                                .unwrap_or(content_size);
                            let content_half = content_size * 0.5;

                            if is_horizontal {
                                let child_h = self
                                    .node(UiNodeId(ci as u32))
                                    .map(|n| {
                                        rects[ci]
                                            .3
                                            .max(n.layout.min_size.y)
                                            .min(n.layout.max_size.y)
                                    })
                                    .unwrap_or(rects[ci].3);
                                // offset by margin, then center content within remaining space
                                rects[ci] = (
                                    pcx - pw * 0.5 + offset + mgn.left + content_half,
                                    pcy,
                                    content_size,
                                    child_h,
                                );
                                offset += final_effective + gap;
                            } else {
                                let child_w = self
                                    .node(UiNodeId(ci as u32))
                                    .map(|n| {
                                        rects[ci]
                                            .2
                                            .max(n.layout.min_size.x)
                                            .min(n.layout.max_size.x)
                                    })
                                    .unwrap_or(rects[ci].2);
                                rects[ci] = (
                                    pcx,
                                    pcy - ph * 0.5 + offset + mgn.top + content_half,
                                    child_w,
                                    content_size,
                                );
                                offset += final_effective + gap;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Step 4: write back to nodes
        for (i, node) in self.nodes.iter_mut().enumerate() {
            let (cx, cy, w, h) = rects[i];
            node.computed.rect.x = cx;
            node.computed.rect.y = cy;
            node.computed.rect.w = w;
            node.computed.rect.h = h;
        }

        self.screen_scale = scale;
        self.dirty_layout = false;
    }

    /// Hit-test: find the topmost visible node at (px, py) by reverse iteration.
    /// If that node has no interaction callback, walks up the parent chain
    /// to find the nearest ancestor with a callback (so clicking a text label
    /// inside a button triggers the button).
    pub fn hit_test(&self, px: f32, py: f32) -> Option<UiNodeId> {
        let leaf = self
            .nodes
            .iter()
            .rev()
            .find(|n| n.visible && n.contains_point(px, py))?;
        // If leaf has a callback, return it directly
        if leaf.interaction_callback.is_some() {
            return Some(leaf.id);
        }
        // Walk up parent chain looking for an interactive ancestor
        let mut current = leaf.parent;
        while let Some(pid) = current {
            if let Some(parent) = self.node(pid) {
                if parent.interaction_callback.is_some() {
                    return Some(pid);
                }
                current = parent.parent;
            } else {
                break;
            }
        }
        // Return the leaf anyway so it can at least get hover state
        Some(leaf.id)
    }

    pub fn process_interaction(&mut self, camera: Option<&Camera>, input: &mut InputState) {
        let (screen_x, screen_y) = input.mouse_position;

        let (mx, my) = match self.space {
            CanvasSpace::Screen => (screen_x, screen_y),
            CanvasSpace::Camera => {
                if let Some(cam) = camera {
                    let vp = Vec2::new(cam.viewport_size.0 as f32, cam.viewport_size.1 as f32);
                    let vs = cam.orthographic_size;
                    ((screen_x / vp.x) * vs.x, (screen_y / vp.y) * vs.y)
                } else {
                    (screen_x, screen_y)
                }
            }
            CanvasSpace::World => {
                if let Some(cam) = camera {
                    let wp = cam.screen_to_world((screen_x, screen_y));
                    (wp.x, wp.y)
                } else {
                    (screen_x, screen_y)
                }
            }
        };

        let left_down = input
            .mouse_buttons_pressed
            .contains(&winit::event::MouseButton::Left);
        let left_just_down = left_down && !self.interaction_was_pressed;
        let left_just_up = self.interaction_was_pressed && !left_down;

        let hovered_id = self.hit_test(mx, my);
        let pressed_node = self.interaction_pressed_node;

        // Determine new interaction state for every node
        let mut new_states: Vec<InteractionState> = vec![InteractionState::None; self.nodes.len()];

        if let Some(hid) = hovered_id {
            let idx = hid.0 as usize;
            if left_just_down {
                new_states[idx] = InteractionState::Pressed;
                self.interaction_pressed_node = Some(hid);
            } else if left_down && pressed_node == Some(hid) {
                new_states[idx] = InteractionState::Dragging;
            } else if left_just_up && pressed_node == Some(hid) {
                new_states[idx] = InteractionState::Clicked;
                self.interaction_pressed_node = None;
            } else {
                new_states[idx] = InteractionState::Hovered;
            }
        } else if left_just_up {
            self.interaction_pressed_node = None;
        }

        // Apply new states and fire callbacks only on actual change
        for (i, node) in self.nodes.iter_mut().enumerate() {
            let new = new_states[i];
            if node.interaction != new {
                node.interaction = new;
                if let Some(ref mut cb) = node.interaction_callback {
                    if let Ok(cb) = cb.get_mut() {
                        cb(new);
                    }
                }
            }
        }

        // Handle slider drag (update value while dragging)
        if left_down {
            if let Some(pid) = self.interaction_pressed_node {
                if let Some(node) = self.node_mut(pid) {
                    if let UiNodeKind::Slider(ref mut props) = node.kind {
                        let rect = node.computed.rect;
                        let local_x = mx - (rect.x - rect.w * 0.5);
                        let t = (local_x / rect.w).clamp(0.0, 1.0);
                        props.value = props.min + t * (props.max - props.min);
                    }
                }
            }
        }

        self.interaction_was_pressed = left_down;
    }

    fn to_screen_rect(&self, rect: crate::ui::UiRect) -> RenderUiRect {
        RenderUiRect {
            x: rect.x * self.screen_scale.x,
            y: rect.y * self.screen_scale.y,
            w: rect.w * self.screen_scale.x,
            h: rect.h * self.screen_scale.y,
        }
    }

    fn world_rect_to_screen(
        &self,
        world_rect: crate::ui::UiRect,
        camera: &Camera,
        transform: Option<&Transform>,
    ) -> RenderUiRect {
        let vs = camera.orthographic_size;
        let mut wcx = (world_rect.x - vs.x * 0.5) + camera.position.x;
        let mut wcy = -(world_rect.y - vs.y * 0.5) + camera.position.y;

        if let Some(t) = transform {
            wcx += t.position.x;
            wcy += t.position.y;
        }

        let screen_center = camera.world_to_screen(Vec2::new(wcx, wcy));
        let visible = camera.ortho_visible_size();
        let scale_x = camera.viewport_size.0 as f32 / visible.x;
        let scale_y = camera.viewport_size.1 as f32 / visible.y;

        RenderUiRect {
            x: screen_center.x,
            y: screen_center.y,
            w: world_rect.w * scale_x,
            h: world_rect.h * scale_y,
        }
    }

    pub fn build_render_commands(
        &self,
        render_queue: &mut RenderQueue,
        camera: Option<&Camera>,
        transform: Option<&Transform>,
        order: i32,
    ) {
        let screen_of = |rect: crate::ui::UiRect| -> RenderUiRect {
            match self.space {
                CanvasSpace::World => {
                    if let Some(cam) = camera {
                        self.world_rect_to_screen(rect, cam, transform)
                    } else {
                        self.to_screen_rect(rect)
                    }
                }
                _ => self.to_screen_rect(rect),
            }
        };

        // In World space, text is rendered `by world` (like sprites): its on-screen
        // size follows the camera zoom. The glyph rasterizer is driven by `font_size`
        // (pixels) and ignores `rect.h`, so we multiply it by the camera's zoom factor
        // (viewport height / visible height) so distant text shrinks and close text
        // grows. For Screen/Camera the zoom is identity / handled by scaless, so the
        // raw pixel `font_size` is kept.
        let world_font_zoom = if matches!(self.space, CanvasSpace::World) {
            camera
                .map(|cam| {
                    let visible = cam.ortho_visible_size();
                    cam.viewport_size.1 as f32 / visible.y
                })
                .unwrap_or(1.0)
        } else {
            1.0
        };

        for node in &self.nodes {
            if !node.visible {
                continue;
            }

            if let Some(background) = node.style.background {
                let rect = screen_of(node.computed.rect);
                render_queue.draw_ui_rect(
                    rect,
                    background,
                    node.style.z_index,
                    order,
                    matches!(self.space, CanvasSpace::World),
                );
            }
            match &node.kind {
                UiNodeKind::Container(_) => {}
                UiNodeKind::Image(props) => {
                    if let Some(texture) = &props.texture {
                        let rect = screen_of(node.computed.rect);
                        render_queue.draw_ui_image(
                            Arc::from(texture.clone()),
                            rect,
                            props.tint,
                            props.uv,
                            node.style.z_index,
                            order,
                            matches!(self.space, CanvasSpace::World),
                        );
                    }
                }
                UiNodeKind::Text(props) => {
                    let rect = screen_of(node.computed.rect);
                    let font_size = props.font_size * world_font_zoom;
                    let segments: Vec<RichTextSegment> = props
                        .segments
                        .iter()
                        .map(|s| RichTextSegment {
                            text: s.text.clone(),
                            color: s.color,
                            bold: s.bold,
                        })
                        .collect();
                    render_queue.draw_ui_text(
                        props.text.clone(),
                        rect,
                        props.color,
                        font_size,
                        node.style.z_index,
                        order,
                        matches!(self.space, CanvasSpace::World),
                        props.font,
                        segments,
                    );
                }
                UiNodeKind::Slider(props) => {
                    let rect = screen_of(node.computed.rect);
                    let track_color = if node.interaction == InteractionState::Hovered
                        || node.interaction == InteractionState::Dragging
                    {
                        [0.3, 0.3, 0.5, 1.0]
                    } else {
                        [0.2, 0.2, 0.3, 1.0]
                    };
                    render_queue.draw_ui_rect(
                        RenderUiRect {
                            x: rect.x,
                            y: rect.y,
                            w: rect.w,
                            h: rect.h * 0.4,
                        },
                        track_color,
                        node.style.z_index,
                        order,
                        matches!(self.space, CanvasSpace::World),
                    );
                    let t = ((props.value - props.min) / (props.max - props.min)).clamp(0.0, 1.0);
                    let thumb_x = rect.x - rect.w * 0.5 + t * rect.w;
                    render_queue.draw_ui_rect(
                        RenderUiRect {
                            x: thumb_x,
                            y: rect.y,
                            w: 12.0,
                            h: rect.h,
                        },
                        [0.8, 0.8, 1.0, 1.0],
                        node.style.z_index,
                        order,
                        matches!(self.space, CanvasSpace::World),
                    );
                }
            }
        }

        if self.debug_show_bounds {
            let outline_color = [0.0, 1.0, 0.0, 0.8];
            for node in &self.nodes {
                if !node.visible {
                    continue;
                }
                let rect = screen_of(node.computed.rect);
                let l = rect.x - rect.w * 0.5;
                let t = rect.y - rect.h * 0.5;
                let r = rect.x + rect.w * 0.5;
                let b = rect.y + rect.h * 0.5;
                render_queue.draw_debug_line(Vec2::new(l, t), Vec2::new(r, t), outline_color, 1.5);
                render_queue.draw_debug_line(Vec2::new(r, t), Vec2::new(r, b), outline_color, 1.5);
                render_queue.draw_debug_line(Vec2::new(r, b), Vec2::new(l, b), outline_color, 1.5);
                render_queue.draw_debug_line(Vec2::new(l, b), Vec2::new(l, t), outline_color, 1.5);
            }
        }
    }
}
