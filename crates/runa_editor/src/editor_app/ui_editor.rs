use std::path::PathBuf;

use egui::{Align2, Color32, Rect, RichText, StrokeKind, Ui};
use runa_core::components::ui::{
    AnchorAsset, ContainerKindAsset, ImagePropsAsset, LayoutPropsAsset, SliderPropsAsset,
    StylePropsAsset, TextAlignAsset, TextPropsAsset, UiAssetFile, UiNodeAsset, UiNodeKindAsset,
};
use runa_project::ui_asset as project_ui_asset;
use runa_project::ProjectPaths;

#[derive(Clone)]
pub struct UiEditorPanel {
    pub open: bool,
    pub asset: UiAssetFile,
    pub path: Option<PathBuf>,
    pub selected_node: Option<u32>,
    pub dirty: bool,
    pub current_name: String,
}

impl Default for UiEditorPanel {
    fn default() -> Self {
        Self {
            open: false,
            asset: UiAssetFile::empty(),
            path: None,
            selected_node: None,
            dirty: false,
            current_name: String::new(),
        }
    }
}

impl UiEditorPanel {
    pub fn open_new(&mut self, _project_root: &ProjectPaths) {
        self.asset = UiAssetFile::empty();
        self.path = None;
        self.selected_node = Some(0);
        self.dirty = false;
        self.current_name = String::new();
        self.open = true;
    }

    pub fn open_asset(&mut self, path: PathBuf) {
        if let Ok(asset) = project_ui_asset::load_ui_asset(&path) {
            self.asset = asset;
            self.path = Some(path);
            self.selected_node = Some(0);
            self.dirty = false;
            self.current_name = String::new();
            self.open = true;
        }
    }

    pub fn save(&mut self, project: &ProjectPaths) {
        let path = if let Some(existing) = &self.path {
            existing.clone()
        } else {
            let name = if self.current_name.is_empty() {
                "NewUI"
            } else {
                &self.current_name
            };
            project_ui_asset::ui_asset_path(project, name)
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) =
            ron::ser::to_string_pretty(&self.asset, ron::ser::PrettyConfig::default())
        {
            if std::fs::write(&path, content).is_ok() {
                self.path = Some(path);
                self.dirty = false;
            }
        }
    }

    pub fn save_as(&mut self, path: PathBuf) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) =
            ron::ser::to_string_pretty(&self.asset, ron::ser::PrettyConfig::default())
        {
            if std::fs::write(&path, content).is_ok() {
                self.path = Some(path);
                self.dirty = false;
            }
        }
    }

    pub fn window(&mut self, ctx: &egui::Context, project: Option<&ProjectPaths>) {
        if !self.open {
            return;
        }

        let mut open = self.open;
        const LEFT_WIDTH: f32 = 180.0;
        const RIGHT_WIDTH: f32 = 220.0;

        let viewport = ctx.viewport_rect();

        egui::Window::new("UI Editor")
            .open(&mut open)
            .resizable(true)
            .default_width(viewport.width() * 0.75)
            .default_height(viewport.height() * 0.75)
            .min_width(640.0)
            .min_height(400.0)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if let Some(project) = project {
                            self.save(project);
                        }
                    }
                    if ui.button("Save As...").clicked() {
                        if let Some(project) = project {
                            let suggested = project_ui_asset::ui_asset_path(project, &self.current_name);
                            if let Some(path) = rfd::FileDialog::new()
                                .set_directory(suggested.parent().unwrap_or(&project.root_dir))
                                .set_file_name(
                                    suggested.file_name().unwrap_or_default().to_str().unwrap_or("NewUI.ui.ron"),
                                )
                                .add_filter("UI Asset", &["ron"])
                                .save_file()
                            {
                                self.save_as(path);
                            }
                        }
                    }
                    if self.dirty {
                        ui.label(RichText::new("* Unsaved").color(Color32::YELLOW));
                    }
                    if let Some(path) = &self.path {
                        ui.label(format!("File: {}", path.display()));
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(LEFT_WIDTH, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.label(RichText::new("Nodes").strong());
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .id_salt("ui_editor_tree")
                                .show(ui, |ui| {
                                    self.node_tree_ui(ui, 0, 0);
                                });
                        },
                    );

                    ui.separator();

                    ui.vertical(|ui| {
                        ui.label(RichText::new("Canvas").strong());
                        ui.separator();
                        let canvas_size = ui.available_size();
                        let (_id, canvas_rect) = ui.allocate_space(canvas_size);
                        let canvas_rect = canvas_rect.intersect(ui.clip_rect());
                        self.canvas_preview(ui, canvas_rect);
                    });

                    ui.separator();

                    ui.allocate_ui_with_layout(
                        egui::vec2(RIGHT_WIDTH, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.label(RichText::new("Properties").strong());
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .id_salt("ui_editor_properties")
                                .show(ui, |ui| {
                                    if let Some(node_id) = self.selected_node {
                                        self.properties_ui(ui, node_id);
                                    } else {
                                        ui.label("Select a node");
                                    }
                                });
                        },
                    );
                });
            });
        self.open = open;
    }

    fn node_tree_ui(&mut self, ui: &mut Ui, node_id: u32, depth: usize) {
        let node_index = self.asset.nodes.iter().position(|n| n.id == node_id);
        let Some(i) = node_index else {
            return;
        };

        let name = self.asset.nodes[i].name.clone();
        let kind = self.asset.nodes[i].kind.clone();
        let children = self.asset.nodes[i].children.clone();
        let has_parent = self.asset.nodes[i].parent.is_some();

        let label = if name.is_empty() {
            format!("{} (id:{})", node_kind_name(&kind), node_id)
        } else {
            format!("{} ({})", name, node_kind_name(&kind))
        };

        let selected = self.selected_node == Some(node_id);

        let response = ui
            .add_sized(
                egui::vec2(ui.available_width(), 20.0),
                egui::Button::selectable(selected, label),
            )
            .on_hover_text(format!("id: {}", node_id));

        if response.clicked() {
            self.selected_node = Some(node_id);
        }

        // Context menu
        response.context_menu(|ui| {
            if ui.button("Add Container Child").clicked() {
                self.add_child_node(node_id, UiNodeKindAsset::Container(ContainerKindAsset::Free));
                ui.close();
            }
            if ui.button("Add Text Child").clicked() {
                self.add_child_node(
                    node_id,
                    UiNodeKindAsset::Text(TextPropsAsset {
                        text: "Text".to_string(),
                        font_size: 16.0,
                        color: [1.0, 1.0, 1.0, 1.0],
                        line_height: None,
                        align: TextAlignAsset::Left,
                    }),
                );
                ui.close();
            }
            if ui.button("Add Image Child").clicked() {
                self.add_child_node(
                    node_id,
                    UiNodeKindAsset::Image(ImagePropsAsset::default()),
                );
                ui.close();
            }
            if ui.button("Add Slider Child").clicked() {
                self.add_child_node(
                    node_id,
                    UiNodeKindAsset::Slider(SliderPropsAsset::default()),
                );
                ui.close();
            }
            ui.separator();
            if has_parent {
                if ui.button("Delete Node").clicked() {
                    self.delete_node(node_id);
                    ui.close();
                }
            }
        });

        for child_id in &children {
            self.node_tree_ui(ui, *child_id, depth + 1);
        }
    }

    fn add_child_node(&mut self, parent_id: u32, kind: UiNodeKindAsset) {
        let parent_index = self.asset.nodes.iter().position(|n| n.id == parent_id);
        let Some(pi) = parent_index else {
            return;
        };

        let new_id = self.asset.nodes.len() as u32;

        let node = UiNodeAsset {
            id: new_id,
            parent: Some(parent_id),
            children: Vec::new(),
            kind,
            layout: LayoutPropsAsset::default(),
            style: StylePropsAsset::default(),
            visible: true,
            name: String::new(),
        };
        self.asset.nodes[pi].children.push(new_id);
        self.asset.nodes.push(node);
        self.selected_node = Some(new_id);
        self.dirty = true;
    }

    fn delete_node(&mut self, node_id: u32) {
        let Some(node_index) = self.asset.nodes.iter().position(|n| n.id == node_id) else {
            return;
        };

        // Remove from parent's children
        if let Some(parent) = self.asset.nodes.get(node_index).and_then(|n| n.parent) {
            if let Some(parent_node) = self.asset.nodes.iter_mut().find(|n| n.id == parent) {
                parent_node.children.retain(|c| *c != node_id);
            }
        }

        // Recursively collect all descendants
        let mut to_remove = vec![node_id];
        let mut i = 0;
        while i < to_remove.len() {
            let id = to_remove[i];
            if let Some(node) = self.asset.nodes.iter().find(|n| n.id == id) {
                for child in &node.children {
                    to_remove.push(*child);
                }
            }
            i += 1;
        }

        self.asset.nodes.retain(|n| !to_remove.contains(&n.id));
        if self.selected_node == Some(node_id) {
            self.selected_node = None;
        }
        self.dirty = true;
    }

    fn canvas_preview(&mut self, ui: &mut Ui, canvas_rect: Rect) {
        let painter = ui.painter_at(canvas_rect);
        let bg = ui.visuals().extreme_bg_color;
        painter.rect_filled(canvas_rect, 0.0, bg);

        let scale_x = canvas_rect.width() / self.asset.viewport_width.max(1.0);
        let scale_y = canvas_rect.height() / self.asset.viewport_height.max(1.0);
        let scale = scale_x.min(scale_y);

        let viewport_w = self.asset.viewport_width * scale;
        let viewport_h = self.asset.viewport_height * scale;
        let offset_x = canvas_rect.center().x - viewport_w * 0.5;
        let offset_y = canvas_rect.center().y - viewport_h * 0.5;

        // Draw viewport area
        let viewport_rect = Rect::from_min_size(
            egui::pos2(offset_x, offset_y),
            egui::vec2(viewport_w, viewport_h),
        );
        painter.rect_stroke(viewport_rect, 0.0, (1.0, Color32::WHITE.gamma_multiply(0.3)), StrokeKind::Inside);

        self.render_node_preview(&painter, 0, scale, offset_x, offset_y);
    }

    fn render_node_preview(
        &self,
        painter: &egui::Painter,
        node_id: u32,
        scale: f32,
        offset_x: f32,
        offset_y: f32,
    ) {
        let Some(node) = self.asset.nodes.iter().find(|n| n.id == node_id) else {
            return;
        };
        if !node.visible {
            return;
        }

        let pos = node.layout.position;
        let min = node.layout.min_size;
        let max = node.layout.max_size;
        let mut w = if max[0].is_finite() { max[0] } else { min[0].max(100.0) };
        let mut h = if max[1].is_finite() { max[1] } else { min[1].max(40.0) };
        w = w.max(min[0]);
        h = h.max(min[1]);

        let anchor_factor = match node.layout.anchor {
            AnchorAsset::TopLeft | AnchorAsset::Left | AnchorAsset::BottomLeft => 0.0,
            AnchorAsset::TopCenter | AnchorAsset::Center | AnchorAsset::BottomCenter => 0.5,
            AnchorAsset::TopRight | AnchorAsset::Right | AnchorAsset::BottomRight => 1.0,
            AnchorAsset::Stretch => 0.5,
        };

        let cx = pos[0] + w * anchor_factor;
        let cy = pos[1] + h * 0.5;

        let rect = Rect::from_center_size(
            egui::pos2(offset_x + cx * scale, offset_y + cy * scale),
            egui::vec2(w * scale, h * scale),
        );

        let is_selected = self.selected_node == Some(node_id);

        // Background
        if let Some(bg) = node.style.background {
            let color = Color32::from_rgba_premultiplied(
                (bg[0] * 255.0) as u8,
                (bg[1] * 255.0) as u8,
                (bg[2] * 255.0) as u8,
                (bg[3] * 255.0) as u8,
            );
            painter.rect_filled(rect, 0.0, color);
        } else {
            let fill = if node.parent.is_none() {
                Color32::from_rgba_premultiplied(30, 30, 40, 200)
            } else {
                Color32::from_rgba_premultiplied(
                    50 + (node.id * 30 % 100) as u8,
                    50,
                    60,
                    180,
                )
            };
            painter.rect_filled(rect, 2.0, fill);
        }

        // Border for selection
        if is_selected {
            painter.rect_stroke(rect, 2.0, (2.0, Color32::YELLOW), StrokeKind::Inside);
        } else {
            painter.rect_stroke(rect, 1.0, (1.0, Color32::WHITE.gamma_multiply(0.3)), StrokeKind::Inside);
        }

        // Label
        let label = if node.name.is_empty() {
            node_kind_name(&node.kind)
        } else {
            &node.name
        };
        let text_color = Color32::WHITE.gamma_multiply(0.8);
        let font_id = egui::FontId::proportional(12.0);
        painter.text(
            rect.min + egui::vec2(2.0, 0.0),
            Align2::LEFT_TOP,
            label,
            font_id,
            text_color,
        );

        // Children
        for child_id in &node.children {
            self.render_node_preview(painter, *child_id, scale, offset_x, offset_y);
        }
    }

    fn get_node(&self, node_id: u32) -> Option<&UiNodeAsset> {
        self.asset.nodes.iter().find(|n| n.id == node_id)
    }

    fn get_node_mut(&mut self, node_id: u32) -> Option<&mut UiNodeAsset> {
        self.asset.nodes.iter_mut().find(|n| n.id == node_id)
    }

    fn properties_ui(&mut self, ui: &mut Ui, node_id: u32) {
        let Some(node) = self.get_node(node_id).cloned() else {
            ui.label("Node not found");
            return;
        };

        ui.label(RichText::new(format!("Node {}", node_id)).strong());
        ui.separator();

        // Name
        let mut name = node.name.clone();
        if ui
            .horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut name).changed()
            })
            .inner
        {
            if let Some(n) = self.get_node_mut(node_id) {
                if name != n.name {
                    n.name = name;
                    self.dirty = true;
                }
            }
        }

        // Kind label
        ui.label(format!("Kind: {}", node_kind_name(&node.kind)));

        match &node.kind {
            UiNodeKindAsset::Container(kind) => {
                let mut current = match kind {
                    ContainerKindAsset::Free => 0usize,
                    ContainerKindAsset::HorizontalBox => 1,
                    ContainerKindAsset::VerticalBox => 2,
                };
                let options = ["Free", "Horizontal", "Vertical"];
                if ui
                    .horizontal(|ui| {
                        ui.label("Layout:");
                        egui::ComboBox::from_id_salt(("ui_container_kind", node_id))
                            .selected_text(options[current])
                            .show_ui(ui, |ui| {
                                for (i, opt) in options.iter().enumerate() {
                                    if ui.selectable_label(current == i, *opt).clicked() {
                                        current = i;
                                    }
                                }
                            });
                    })
                    .response
                    .changed()
                {
                    let new_kind = match current {
                        1 => ContainerKindAsset::HorizontalBox,
                        2 => ContainerKindAsset::VerticalBox,
                        _ => ContainerKindAsset::Free,
                    };
                    if let Some(n) = self.get_node_mut(node_id) {
                        n.kind = UiNodeKindAsset::Container(new_kind);
                        self.dirty = true;
                    }
                }
            }
            UiNodeKindAsset::Text(props) => {
                let mut props = props.clone();
                if ui
                    .horizontal(|ui| {
                        ui.label("Text:");
                        ui.add_sized(
                            [ui.available_width().max(80.0), 20.0],
                            egui::TextEdit::singleline(&mut props.text),
                        )
                        .changed()
                    })
                    .inner
                {
                    if let Some(n) = self.get_node_mut(node_id) {
                        n.kind = UiNodeKindAsset::Text(props.clone());
                        self.dirty = true;
                    }
                }
                // Font size
                let mut font_size = props.font_size;
                if ui
                    .horizontal(|ui| {
                        ui.label("Font Size:");
                        ui.add(egui::Slider::new(&mut font_size, 1.0..=256.0))
                            .changed()
                    })
                    .inner
                {
                    if let Some(n) = self.get_node_mut(node_id) {
                        if let UiNodeKindAsset::Text(ref mut p) = n.kind {
                            p.font_size = font_size;
                            self.dirty = true;
                        }
                    }
                }
            }
            UiNodeKindAsset::Slider(props) => {
                let mut props = props.clone();
                let mut changed = false;
                let mut value = props.value;
                if ui
                    .horizontal(|ui| {
                        ui.label("Value:");
                        ui.add(egui::Slider::new(&mut value, props.min..=props.max))
                            .changed()
                    })
                    .inner
                {
                    props.value = value;
                    changed = true;
                }
                let mut min = props.min;
                if ui
                    .horizontal(|ui| {
                        ui.label("Min:");
                        ui.add(egui::DragValue::new(&mut min).speed(0.1).range(-10000.0..=10000.0))
                            .changed()
                    })
                    .inner
                {
                    props.min = min;
                    changed = true;
                }
                let mut max = props.max;
                if ui
                    .horizontal(|ui| {
                        ui.label("Max:");
                        ui.add(egui::DragValue::new(&mut max).speed(0.1).range(-10000.0..=10000.0))
                            .changed()
                    })
                    .inner
                {
                    props.max = max;
                    changed = true;
                }
                if changed {
                    if let Some(n) = self.get_node_mut(node_id) {
                        n.kind = UiNodeKindAsset::Slider(props);
                        self.dirty = true;
                    }
                }
            }
            UiNodeKindAsset::Image(props) => {
                let mut texture_path = props.texture_path.clone().unwrap_or_default();
                if ui
                    .horizontal(|ui| {
                        ui.label("Texture:");
                        ui.add_sized(
                            [ui.available_width().max(80.0), 20.0],
                            egui::TextEdit::singleline(&mut texture_path),
                        )
                        .changed()
                    })
                    .inner
                {
                    if let Some(n) = self.get_node_mut(node_id) {
                        if let UiNodeKindAsset::Image(ref mut p) = n.kind {
                            let trimmed = texture_path.trim().to_string();
                            p.texture_path = if trimmed.is_empty() { None } else { Some(trimmed) };
                            self.dirty = true;
                        }
                    }
                }
            }
        }

        ui.separator();
        ui.label("Layout");
        let mut layout = node.layout.clone();
        let mut layout_changed = false;

        // Anchor
        let anchor_names = [
            "TopLeft", "TopCenter", "TopRight", "Left", "Center", "Right", "BottomLeft",
            "BottomCenter", "BottomRight", "Stretch",
        ];
        let anchor_idx = match layout.anchor {
            AnchorAsset::TopLeft => 0,
            AnchorAsset::TopCenter => 1,
            AnchorAsset::TopRight => 2,
            AnchorAsset::Left => 3,
            AnchorAsset::Center => 4,
            AnchorAsset::Right => 5,
            AnchorAsset::BottomLeft => 6,
            AnchorAsset::BottomCenter => 7,
            AnchorAsset::BottomRight => 8,
            AnchorAsset::Stretch => 9,
        };
        let mut new_anchor = anchor_idx;
        ui.horizontal(|ui| {
            ui.label("Anchor:");
            egui::ComboBox::from_id_salt(("ui_anchor", node_id))
                .selected_text(anchor_names[anchor_idx])
                .show_ui(ui, |ui| {
                    for (i, name) in anchor_names.iter().enumerate() {
                        if ui.selectable_label(anchor_idx == i, *name).clicked() {
                            new_anchor = i;
                        }
                    }
                });
        });
        if new_anchor != anchor_idx {
            layout.anchor = match new_anchor {
                1 => AnchorAsset::TopCenter,
                2 => AnchorAsset::TopRight,
                3 => AnchorAsset::Left,
                4 => AnchorAsset::Center,
                5 => AnchorAsset::Right,
                6 => AnchorAsset::BottomLeft,
                7 => AnchorAsset::BottomCenter,
                8 => AnchorAsset::BottomRight,
                9 => AnchorAsset::Stretch,
                _ => AnchorAsset::TopLeft,
            };
            layout_changed = true;
        }

        // Position
        layout_changed |= property_f32_pair(ui, "Position", &mut layout.position, 0, 1);
        layout_changed |= property_f32_pair(ui, "Min Size", &mut layout.min_size, 0, 1);
        if layout.max_size[0].is_finite() || layout.max_size[1].is_finite() {
            let mut max_w = if layout.max_size[0].is_finite() {
                layout.max_size[0] as i32
            } else {
                0
            };
            let mut max_h = if layout.max_size[1].is_finite() {
                layout.max_size[1] as i32
            } else {
                0
            };
            if ui
                .horizontal(|ui| {
                    ui.label("Max Size:");
                    ui.add(egui::DragValue::new(&mut max_w).speed(1.0).range(0..=10000))
                        .changed()
                        || ui
                            .add(egui::DragValue::new(&mut max_h).speed(1.0).range(0..=10000))
                            .changed()
                })
                .inner
            {
                layout.max_size = [if max_w > 0 { max_w as f32 } else { f32::INFINITY }, if max_h > 0 { max_h as f32 } else { f32::INFINITY }];
                layout_changed = true;
            }
        }

        if layout_changed {
            if let Some(n) = self.get_node_mut(node_id) {
                n.layout = layout;
                self.dirty = true;
            }
        }

        ui.separator();
        ui.label("Style");
        let mut style = node.style.clone();
        let mut style_changed = false;

        let mut has_bg = style.background.is_some();
        if ui.checkbox(&mut has_bg, "Background").changed() {
            if has_bg {
                style.background = Some([0.2, 0.2, 0.3, 0.8]);
            } else {
                style.background = None;
            }
            style_changed = true;
        }

        let mut opacity = style.opacity;
        if ui
            .horizontal(|ui| {
                ui.label("Opacity:");
                ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0)).changed()
            })
            .inner
        {
            style.opacity = opacity;
            style_changed = true;
        }

        if style_changed {
            if let Some(n) = self.get_node_mut(node_id) {
                n.style = style;
                self.dirty = true;
            }
        }
    }
}

fn node_kind_name(kind: &UiNodeKindAsset) -> &'static str {
    match kind {
        UiNodeKindAsset::Container(c) => match c {
            ContainerKindAsset::Free => "Container",
            ContainerKindAsset::HorizontalBox => "HBox",
            ContainerKindAsset::VerticalBox => "VBox",
        },
        UiNodeKindAsset::Image(_) => "Image",
        UiNodeKindAsset::Text(_) => "Text",
        UiNodeKindAsset::Slider(_) => "Slider",
    }
}

fn property_f32_pair(ui: &mut Ui, label: &str, values: &mut [f32; 2], _idx0: usize, _idx1: usize) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        let c0 = ui
            .add(egui::DragValue::new(&mut values[0]).speed(1.0).range(-5000.0..=5000.0))
            .changed();
        let c1 = ui
            .add(egui::DragValue::new(&mut values[1]).speed(1.0).range(-5000.0..=5000.0))
            .changed();
        c0 || c1
    })
    .inner
}
