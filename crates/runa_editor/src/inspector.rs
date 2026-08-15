use std::any::TypeId;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use egui::{Color32, RichText, Ui};
use rfd::FileDialog;
use runa_asset::loader::load_image;
use runa_asset::AudioAsset;
use runa_core::components::{
    ActiveCamera, AudioListener, AudioSource, BuiltinMeshPrimitive, Camera, Collider2D,
    ComponentRuntimeKind, CursorInteractable, DirectionalLight, MeshRenderer, PhysicsCollision,
    PointLight, ProjectionType, SerializedField, SerializedFieldValue, SerializedTypeKind,
    SerializedTypeStorage, Sorting, SpriteAnimationClip, SpriteAnimator, SpriteRenderer, Tilemap,
    TilemapRenderer, Transform, UiRenderer,
};
use runa_core::glam::{EulerRot, Quat, USizeVec2, Vec3};
use runa_core::ocs::Object;
use runa_project::runa3d::RunaModel;

use crate::editor_textures::{load_component_icon, load_editor_icon};
use crate::style;

#[derive(Debug, Default)]
pub struct InspectorActions {
    pub removals: Vec<InspectorRemoval>,
    pub open_ui_editor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TilePaintMode {
    None,
    Paint,
    Erase,
}

impl Default for TilePaintMode {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Default)]
pub struct TilePaintToolState {
    pub mode: TilePaintMode,
    pub layer: u32,
    pub palette_open: bool,
}

#[derive(Debug, Clone)]
pub struct InspectorRemoval {
    pub target: InspectorRemovalTarget,
}

#[derive(Debug, Clone)]
pub enum InspectorRemovalTarget {
    RuntimeType {
        type_id: TypeId,
        type_name: String,
    },
    SerializedType {
        kind: SerializedTypeKind,
        type_name: String,
    },
}

fn type_count(object: &Object, kind: SerializedTypeKind) -> usize {
    let runtime_count = object
        .component_infos()
        .iter()
        .filter(|info| {
            matches!(
                (kind, info.kind()),
                (
                    SerializedTypeKind::Component,
                    ComponentRuntimeKind::Component
                ) | (SerializedTypeKind::Script, ComponentRuntimeKind::Script)
            )
        })
        .count();
    let serialized_count = object
        .get_component::<SerializedTypeStorage>()
        .map(|storage| storage.entries_of_kind(kind).count())
        .unwrap_or(0);
    runtime_count + serialized_count
}

pub fn inspector_ui(
    ui: &mut Ui,
    object: &mut Object,
    project_root: Option<&Path>,
    scripts_dir: Option<&Path>,
    editor_settings: &crate::editor_settings::EditorSettings,
    tile_paint: &mut TilePaintToolState,
) -> InspectorActions {
    let mut actions = InspectorActions::default();

    object_section(ui, object);
    ui.separator();
    transform_section(ui, object);
    ui.separator();
    components_section(
        ui,
        object,
        project_root,
        editor_settings,
        tile_paint,
        &mut actions,
    );
    ui.separator();
    scripts_section(ui, object, scripts_dir, editor_settings, &mut actions);

    actions
}

fn object_section(ui: &mut Ui, object: &mut Object) {
    ui.heading("Object");
    property_row(ui, "Id", |ui| {
        ui.label(
            object
                .id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "Unassigned".to_string()),
        );
    });
    property_row(ui, "Name", |ui| {
        ui.add_sized(
            [ui.available_width().max(120.0), 22.0],
            egui::TextEdit::singleline(&mut object.name),
        );
    });
}

fn transform_section(ui: &mut Ui, object: &mut Object) {
    if let Some(transform) = object.get_component_mut::<Transform>() {
        component_card(
            ui,
            TypeId::of::<Transform>(),
            ComponentRuntimeKind::Component,
            "Transform",
            false,
            None,
            None,
            None,
            &crate::editor_settings::EditorSettings::default(),
            |ui| {
                vec3_editor(ui, "Position", &mut transform.position);
                quat_editor(ui, transform);
                vec3_editor(ui, "Scale", &mut transform.scale);
            },
        );
    }
}

fn components_section(
    ui: &mut Ui,
    object: &mut Object,
    project_root: Option<&Path>,
    editor_settings: &crate::editor_settings::EditorSettings,
    tile_paint: &mut TilePaintToolState,
    actions: &mut InspectorActions,
) {
    ui.heading(format!(
        "Components - {}",
        type_count(object, SerializedTypeKind::Component)
    ));

    let mut component_infos: Vec<_> = object
        .component_infos()
        .into_iter()
        .filter(|info| {
            info.kind() == ComponentRuntimeKind::Component
                && info.type_id() != TypeId::of::<Transform>()
                && info.type_id() != TypeId::of::<Tilemap>()
        })
        .collect();
    component_infos.sort_by(|left, right| left.type_name().cmp(right.type_name()));

    let has_serialized_components = object
        .get_component::<SerializedTypeStorage>()
        .map(|storage| {
            storage
                .entries_of_kind(SerializedTypeKind::Component)
                .next()
                .is_some()
        })
        .unwrap_or(false);
    if component_infos.is_empty() && !has_serialized_components {
        ui.label("No extra components attached.");
    }

    if let Some(camera) = object.get_component_mut::<Camera>() {
        component_block(
            ui,
            "Camera",
            true,
            actions,
            TypeId::of::<Camera>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                let previous_projection = camera.projection;
                vec3_editor(ui, "Position", &mut camera.position);
                vec3_editor(ui, "Target", &mut camera.target);
                property_row(ui, "Projection", |ui| {
                    ui.selectable_value(
                        &mut camera.projection,
                        ProjectionType::Orthographic,
                        "Orthographic",
                    );
                    ui.selectable_value(
                        &mut camera.projection,
                        ProjectionType::Perspective,
                        "Perspective",
                    );
                });
                if previous_projection != camera.projection
                    && camera.projection == ProjectionType::Orthographic
                    && camera.orthographic_size.length_squared() <= f32::EPSILON
                {
                    camera.orthographic_size = runa_core::glam::Vec2::new(320.0, 180.0);
                }
                if camera.projection == ProjectionType::Orthographic {
                    property_row(ui, "Orthographic Size", |ui| {
                        ui.add_sized(
                            [78.0, 22.0],
                            egui::DragValue::new(&mut camera.orthographic_size.x).speed(1.0),
                        );
                        ui.add_sized(
                            [78.0, 22.0],
                            egui::DragValue::new(&mut camera.orthographic_size.y).speed(1.0),
                        );
                    });
                }
                property_row(ui, "FOV", |ui| {
                    let mut degrees = camera.fov.to_degrees();
                    let response = ui.add_enabled(
                        camera.projection == ProjectionType::Perspective,
                        egui::DragValue::new(&mut degrees).speed(0.25),
                    );
                    if response.changed() {
                        camera.fov = degrees.to_radians();
                    }
                });
                property_row(ui, "Near", |ui| {
                    ui.add_sized(
                        [96.0, 22.0],
                        egui::DragValue::new(&mut camera.near).speed(0.01),
                    );
                });
                property_row(ui, "Far", |ui| {
                    ui.add_sized(
                        [96.0, 22.0],
                        egui::DragValue::new(&mut camera.far).speed(1.0),
                    );
                });
            },
        );
    }

    if object.get_component::<ActiveCamera>().is_some() {
        component_block(
            ui,
            "Active Camera",
            true,
            actions,
            TypeId::of::<ActiveCamera>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                property_row(ui, "State", |ui| {
                    ui.label("Selected runtime camera");
                });
            },
        );
    }

    if let Some(mesh_renderer) = object.get_component_mut::<MeshRenderer>() {
        component_block(
            ui,
            "Mesh Renderer",
            true,
            actions,
            TypeId::of::<MeshRenderer>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                // let mut error_message = None;
                editable_asset_path(ui, "Mesh", &mut mesh_renderer.mesh_path);
                property_row(ui, "Actions", |ui| {
                    if ui.button("Choose PNG").clicked() {
                        let _ = pick_asset_file(project_root, &["png", "jpg", "jpeg", "webp"]);
                    }
                    if ui.button("Clear").clicked() {
                        mesh_renderer.set_mesh(None, None);
                    }
                });
                property_row(ui, "Vertices", |ui| {
                    ui.label(
                        mesh_renderer
                            .get_mesh_handle()
                            .inner
                            .vertices
                            .len()
                            .to_string(),
                    );
                });
                property_row(ui, "Indices", |ui| {
                    ui.label(
                        mesh_renderer
                            .get_mesh_handle()
                            .inner
                            .indices
                            .len()
                            .to_string(),
                    );
                });
                color_editor(ui, "Tint", &mut mesh_renderer.color);
            },
        );
    }

    if let Some(sprite) = object.get_component_mut::<SpriteRenderer>() {
        component_block(
            ui,
            "Sprite Renderer",
            true,
            actions,
            TypeId::of::<SpriteRenderer>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                let mut error_message = None;
                editable_asset_path(ui, "Sprite", &mut sprite.texture_path);
                property_row(ui, "Actions", |ui| {
                    if ui.button("Choose PNG").clicked() {
                        if let Some(path) =
                            pick_asset_file(project_root, &["png", "jpg", "jpeg", "webp"])
                        {
                            match load_texture_from_path(project_root, &path) {
                                Ok(texture) => sprite.set_texture(Some(texture), Some(path)),
                                Err(error) => error_message = Some(error),
                            }
                        }
                    }
                    if ui.button("Clear").clicked() {
                        sprite.set_texture(None, None);
                    }
                });
                property_row(ui, "Texture", |ui| {
                    ui.label(if sprite.texture.is_some() {
                        "assigned"
                    } else {
                        "none"
                    });
                });
                property_row(ui, "Pixels Per Unit", |ui| {
                    let response = ui.add_sized(
                        [96.0, 22.0],
                        egui::DragValue::new(&mut sprite.pixels_per_unit).speed(0.25),
                    );
                    if response.changed() {
                        sprite.pixels_per_unit = sprite.pixels_per_unit.max(f32::EPSILON);
                    }
                });
                if let Some(error) = error_message {
                    ui.colored_label(style::ERROR_COLOR, error);
                }
            },
        );
    }

    if let Some(sorting) = object.get_component_mut::<Sorting>() {
        component_block(
            ui,
            "Sorting",
            true,
            actions,
            TypeId::of::<Sorting>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                property_row(ui, "Order", |ui| {
                    ui.add_sized(
                        [96.0, 22.0],
                        egui::DragValue::new(&mut sorting.order).speed(1.0),
                    );
                });
            },
        );
    }

    let sprite_texture_size = object
        .get_component::<SpriteRenderer>()
        .and_then(|sprite| sprite.texture.as_ref())
        .map(|texture| [texture.inner.width, texture.inner.height]);
    let has_sprite_renderer = object.get_component::<SpriteRenderer>().is_some();
    let mut sprite_animator_uv_rect = None;
    if let Some(animator) = object.get_component_mut::<SpriteAnimator>() {
        component_block(
            ui,
            "Sprite Animator",
            true,
            actions,
            TypeId::of::<SpriteAnimator>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                if !has_sprite_renderer {
                    ui.colored_label(
                        Color32::from_rgb(235, 178, 72),
                        "Requires SpriteRenderer on the same object.",
                    );
                }

                property_row(ui, "Columns", |ui| {
                    let mut columns = animator.sheet.columns.max(1);
                    if ui
                        .add_sized(
                            [96.0, 22.0],
                            egui::DragValue::new(&mut columns).range(1..=512),
                        )
                        .changed()
                    {
                        animator.set_sheet(columns, animator.sheet.rows);
                    }
                });
                property_row(ui, "Rows", |ui| {
                    let mut rows = animator.sheet.rows.max(1);
                    if ui
                        .add_sized([96.0, 22.0], egui::DragValue::new(&mut rows).range(1..=512))
                        .changed()
                    {
                        animator.set_sheet(animator.sheet.columns, rows);
                    }
                });
                if let Some([texture_width, texture_height]) = sprite_texture_size {
                    property_row(ui, "Frame Pixels", |ui| {
                        let mut frame_width =
                            (texture_width / animator.sheet.columns.max(1)).max(1);
                        let mut frame_height = (texture_height / animator.sheet.rows.max(1)).max(1);
                        let width_changed = ui
                            .add_sized(
                                [78.0, 22.0],
                                egui::DragValue::new(&mut frame_width)
                                    .range(1..=texture_width.max(1)),
                            )
                            .changed();
                        let height_changed = ui
                            .add_sized(
                                [78.0, 22.0],
                                egui::DragValue::new(&mut frame_height)
                                    .range(1..=texture_height.max(1)),
                            )
                            .changed();
                        if width_changed || height_changed {
                            animator.set_sheet(
                                (texture_width / frame_width.max(1)).max(1),
                                (texture_height / frame_height.max(1)).max(1),
                            );
                        }
                    });
                }
                property_row(ui, "Frames", |ui| {
                    ui.label(animator.sheet.frame_count().to_string());
                });
                property_row(ui, "Playing", |ui| {
                    ui.checkbox(&mut animator.playing, "");
                });
                property_row(ui, "Current Clip", |ui| {
                    let mut selected = animator.current_clip.clone().unwrap_or_default();
                    egui::ComboBox::from_id_salt("sprite_animator_current_clip")
                        .selected_text(if selected.is_empty() {
                            "None"
                        } else {
                            selected.as_str()
                        })
                        .show_ui(ui, |ui| {
                            for clip in &animator.clips {
                                ui.selectable_value(
                                    &mut selected,
                                    clip.name.clone(),
                                    clip.name.as_str(),
                                );
                            }
                        });
                    if !selected.is_empty()
                        && animator.current_clip.as_deref() != Some(selected.as_str())
                    {
                        let _ = animator.play_clip(&selected);
                    }
                });
                property_row(ui, "Current Frame", |ui| {
                    ui.add_sized(
                        [96.0, 22.0],
                        egui::DragValue::new(&mut animator.current_frame)
                            .range(0..=animator.sheet.frame_count().saturating_sub(1)),
                    );
                });

                ui.separator();
                property_row(ui, "Clips", |ui| {
                    if ui.button("Add Clip").clicked() {
                        let name = format!("Clip {}", animator.clips.len() + 1);
                        let max_frame = animator.sheet.frame_count().saturating_sub(1);
                        animator.clips.push(SpriteAnimationClip::new(
                            name.clone(),
                            0,
                            max_frame,
                            12.0,
                        ));
                        if animator.current_clip.is_none() {
                            animator.current_clip = Some(name);
                        }
                    }
                });

                let mut remove_clip = None;
                let mut move_clip = None;
                let mut play_clip = None;
                let max_frame = animator.sheet.frame_count().saturating_sub(1);
                let clip_count = animator.clips.len();
                let can_remove_clip = animator.clips.len() > 1;
                for (index, clip) in animator.clips.iter_mut().enumerate() {
                    egui::CollapsingHeader::new(clip.name.clone())
                        .id_salt(("sprite_animator_clip", index))
                        .default_open(index == 0)
                        .show(ui, |ui| {
                            property_row(ui, "Name", |ui| {
                                ui.add_sized(
                                    [ui.available_width().max(120.0), 22.0],
                                    egui::TextEdit::singleline(&mut clip.name),
                                );
                            });
                            property_row(ui, "Start Frame", |ui| {
                                ui.add_sized(
                                    [96.0, 22.0],
                                    egui::DragValue::new(&mut clip.start_frame)
                                        .range(0..=max_frame),
                                );
                            });
                            property_row(ui, "End Frame", |ui| {
                                ui.add_sized(
                                    [96.0, 22.0],
                                    egui::DragValue::new(&mut clip.end_frame).range(0..=max_frame),
                                );
                            });
                            property_row(ui, "FPS", |ui| {
                                ui.add_sized(
                                    [96.0, 22.0],
                                    egui::DragValue::new(&mut clip.fps)
                                        .range(0.0..=240.0)
                                        .speed(0.25),
                                );
                            });
                            bool_row(ui, "Loop", &mut clip.looping);
                            property_row(ui, "Actions", |ui| {
                                if ui.add_enabled(index > 0, egui::Button::new("Up")).clicked() {
                                    move_clip = Some((index, index - 1));
                                }
                                if ui
                                    .add_enabled(index + 1 < clip_count, egui::Button::new("Down"))
                                    .clicked()
                                {
                                    move_clip = Some((index, index + 1));
                                }
                                if ui.button("Play").clicked() {
                                    play_clip = Some((clip.name.clone(), clip.start_frame));
                                }
                                if can_remove_clip && ui.button("Remove").clicked() {
                                    remove_clip = Some(index);
                                }
                            });
                        });
                    clip.end_frame = clip.end_frame.max(clip.start_frame).min(max_frame);
                    clip.start_frame = clip.start_frame.min(clip.end_frame);
                }

                if let Some((name, start_frame)) = play_clip {
                    animator.current_clip = Some(name);
                    animator.current_frame = start_frame;
                    animator.playing = true;
                }
                if let Some((from, to)) = move_clip {
                    animator.clips.swap(from, to);
                }
                if let Some(index) = remove_clip {
                    let removed = animator.clips.remove(index);
                    if animator.current_clip.as_deref() == Some(removed.name.as_str()) {
                        animator.current_clip =
                            animator.clips.first().map(|clip| clip.name.clone());
                    }
                }

                sprite_animator_uv_rect =
                    Some(animator.sheet.uv_rect_for_frame(animator.current_frame));
            },
        );
    }
    if let Some(uv_rect) = sprite_animator_uv_rect {
        if let Some(sprite) = object.get_component_mut::<SpriteRenderer>() {
            sprite.set_uv_rect(uv_rect);
        }
    }

    let has_tilemap_renderer = object.get_component::<TilemapRenderer>().is_some();
    if has_tilemap_renderer {
        if let Some(tilemap) = object.get_component_mut::<Tilemap>() {
            component_block(
                ui,
                "Tilemap Renderer",
                true,
                actions,
                TypeId::of::<TilemapRenderer>(),
                ComponentRuntimeKind::Component,
                None,
                None,
                editor_settings,
                |ui| {
                    let mut error_message = None;
                    property_row(ui, "Map Size", |ui| {
                        ui.label(format!("{} x {}", tilemap.width, tilemap.height));
                    });
                    property_row(ui, "Atlas", |ui| {
                        let mut atlas_path = tilemap
                            .atlas
                            .as_ref()
                            .and_then(|atlas| atlas.texture_path.clone());
                        editable_asset_path_inline(ui, &mut atlas_path);
                    });
                    property_row(ui, "Actions", |ui| {
                        if ui.button("Choose PNG").clicked() {
                            if let Some(path) =
                                pick_asset_file(project_root, &["png", "jpg", "jpeg", "webp"])
                            {
                                match load_texture_from_path(project_root, &path) {
                                    Ok(texture) => {
                                        tilemap.set_atlas(Some(texture), Some(path), 1, 1);
                                        sync_tilemap_tile_size_from_atlas(tilemap);
                                    }
                                    Err(error) => error_message = Some(error),
                                }
                            }
                        }
                        if ui.button("Clear").clicked() {
                            tilemap.atlas = None;
                        }
                    });
                    if let Some(atlas) = tilemap.atlas.as_ref() {
                        let texture_width = atlas.texture.width;
                        let texture_height = atlas.texture.height;
                        let mut columns = atlas.columns.max(1);
                        let mut rows = atlas.rows.max(1);
                        let mut changed_grid = false;
                        property_row(ui, "Columns", |ui| {
                            if ui
                                .add_sized(
                                    [96.0, 22.0],
                                    egui::DragValue::new(&mut columns).range(1..=512),
                                )
                                .changed()
                            {
                                changed_grid = true;
                            }
                        });
                        property_row(ui, "Rows", |ui| {
                            if ui
                                .add_sized(
                                    [96.0, 22.0],
                                    egui::DragValue::new(&mut rows).range(1..=512),
                                )
                                .changed()
                            {
                                changed_grid = true;
                            }
                        });
                        property_row(ui, "Tile Pixels", |ui| {
                            let mut frame_width = (texture_width / columns.max(1)).max(1);
                            let mut frame_height = (texture_height / rows.max(1)).max(1);
                            let width_changed = ui
                                .add_sized(
                                    [78.0, 22.0],
                                    egui::DragValue::new(&mut frame_width)
                                        .range(1..=texture_width.max(1)),
                                )
                                .changed();
                            let height_changed = ui
                                .add_sized(
                                    [78.0, 22.0],
                                    egui::DragValue::new(&mut frame_height)
                                        .range(1..=texture_height.max(1)),
                                )
                                .changed();
                            if width_changed || height_changed {
                                columns = (texture_width / frame_width.max(1)).max(1);
                                rows = (texture_height / frame_height.max(1)).max(1);
                                changed_grid = true;
                            }
                        });
                        if changed_grid {
                            if let Some(atlas) = tilemap.atlas.as_mut() {
                                atlas.columns = columns.max(1);
                                atlas.rows = rows.max(1);
                            }
                            sync_tilemap_tile_size_from_atlas(tilemap);
                        }
                        let max_selected_tile = tilemap.atlas_frame_count().saturating_sub(1);
                        property_row(ui, "Selected Tile", |ui| {
                            ui.add_sized(
                                [96.0, 22.0],
                                egui::DragValue::new(&mut tilemap.selected_tile)
                                    .range(0..=max_selected_tile),
                            );
                        });
                    } else {
                        ui.colored_label(
                            Color32::from_rgb(235, 178, 72),
                            "Assign an atlas to paint tiles.",
                        );
                    }
                    property_row(ui, "Offset", |ui| {
                        ui.add_sized(
                            [78.0, 22.0],
                            egui::DragValue::new(&mut tilemap.offset.x).speed(1.0),
                        );
                        ui.add_sized(
                            [78.0, 22.0],
                            egui::DragValue::new(&mut tilemap.offset.y).speed(1.0),
                        );
                    });
                    property_row(ui, "Pixels Per Unit", |ui| {
                        ui.add_sized(
                            [96.0, 22.0],
                            egui::DragValue::new(&mut tilemap.pixels_per_unit)
                                .range(0.001..=4096.0)
                                .speed(0.25),
                        );
                    });
                    property_row(ui, "Tile Size", |ui| {
                        ui.label(format!("{} x {}", tilemap.tile_size.x, tilemap.tile_size.y));
                    });
                    tilemap_paint_ui(ui, tilemap, tile_paint);
                    property_row(ui, "Layers", |ui| {
                        ui.label(tilemap.layers.len().to_string());
                        ui.separator();
                        if ui.button("Add Layer").clicked() {
                            let name = format!("Layer {}", tilemap.layers.len() + 1);
                            tilemap
                                .layers
                                .push(runa_core::components::TilemapLayer::new(
                                    name,
                                    tilemap.width,
                                    tilemap.height,
                                ));
                        }
                    });
                    let layer_count = tilemap.layers.len();
                    let mut move_layer = None;
                    for (index, layer) in tilemap.layers.iter_mut().enumerate() {
                        egui::CollapsingHeader::new(layer.name.clone())
                            .id_salt(("tilemap_layer", index))
                            .default_open(true)
                            .show(ui, |ui| {
                                property_row(ui, "Name", |ui| {
                                    ui.add_sized(
                                        [ui.available_width().max(120.0), 22.0],
                                        egui::TextEdit::singleline(&mut layer.name),
                                    );
                                });
                                property_row(ui, "Visible", |ui| {
                                    ui.checkbox(&mut layer.visible, "");
                                });
                                property_row(ui, "Opacity", |ui| {
                                    ui.add_sized(
                                        [96.0, 22.0],
                                        egui::DragValue::new(&mut layer.opacity)
                                            .range(0.0..=1.0)
                                            .speed(0.01),
                                    );
                                });
                                property_row(ui, "Render Order", |ui| {
                                    ui.add_sized(
                                        [96.0, 22.0],
                                        egui::DragValue::new(&mut layer.self_order).speed(1),
                                    );
                                });
                                property_row(ui, "Actions", |ui| {
                                    if ui.add_enabled(index > 0, egui::Button::new("Up")).clicked()
                                    {
                                        move_layer = Some((index, index - 1));
                                    }
                                    if ui
                                        .add_enabled(
                                            index + 1 < layer_count,
                                            egui::Button::new("Down"),
                                        )
                                        .clicked()
                                    {
                                        move_layer = Some((index, index + 1));
                                    }
                                });
                            });
                    }
                    if let Some((from, to)) = move_layer {
                        tilemap.layers.swap(from, to);
                    }
                    if let Some(error) = error_message {
                        ui.colored_label(style::ERROR_COLOR, error);
                    }
                },
            );
        }
    }

    if let Some(audio) = object.get_component_mut::<AudioSource>() {
        component_block(
            ui,
            "Audio Source",
            true,
            actions,
            TypeId::of::<AudioSource>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                let mut error_message = None;
                editable_asset_path(ui, "Source", &mut audio.source_path);
                property_row(ui, "Actions", |ui| {
                    if ui.button("Choose OGG").clicked() {
                        if let Some(path) = pick_asset_file(project_root, &["ogg"]) {
                            match load_audio_from_path(project_root, &path) {
                                Ok(asset) => {
                                    audio.set_asset_with_path(Some(asset), Some(path));
                                }
                                Err(error) => error_message = Some(error),
                            }
                        }
                    }
                    if ui.button("Clear").clicked() {
                        audio.set_asset_with_path(None, None);
                    }
                });
                property_row(ui, "Volume", |ui| {
                    ui.add(
                        egui::DragValue::new(&mut audio.volume)
                            .range(0.0..=1.0)
                            .speed(0.01),
                    );
                });
                bool_row(ui, "Looped", &mut audio.looped);
                bool_row(ui, "Play On Awake", &mut audio.play_on_awake);
                bool_row(ui, "Spatial", &mut audio.spatial);
                property_row(ui, "Min Distance", |ui| {
                    ui.add_sized(
                        [96.0, 22.0],
                        egui::DragValue::new(&mut audio.min_distance).speed(0.1),
                    );
                });
                property_row(ui, "Max Distance", |ui| {
                    ui.add_sized(
                        [96.0, 22.0],
                        egui::DragValue::new(&mut audio.max_distance).speed(0.1),
                    );
                });
                if let Some(error) = error_message {
                    ui.colored_label(style::ERROR_COLOR, error);
                }
            },
        );
    }

    if let Some(listener) = object.get_component::<AudioListener>() {
        component_block(
            ui,
            "Audio Listener",
            true,
            actions,
            TypeId::of::<AudioListener>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                property_row(ui, "Active", |ui| {
                    ui.label(listener.active.to_string());
                });
                property_row(ui, "Volume", |ui| {
                    ui.label(format!("{:.2}", listener.volume));
                });
                property_row(ui, "Stereo Separation", |ui| {
                    ui.label(format!("{:.2}", listener.stereo_separation));
                });
            },
        );
    }

    if let Some(interactable) = object.get_component::<CursorInteractable>() {
        component_block(
            ui,
            "Cursor Interactable",
            true,
            actions,
            TypeId::of::<CursorInteractable>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                property_row(ui, "Bounds", |ui| {
                    ui.label(format!(
                        "{:.2}, {:.2}, {:.2}",
                        interactable.bounds_size.x,
                        interactable.bounds_size.y,
                        interactable.bounds_size.z
                    ));
                });
                property_row(ui, "Hovered", |ui| {
                    ui.label(interactable.is_hovered.to_string());
                });
                property_row(ui, "Pressed", |ui| {
                    ui.label(interactable.is_pressed.to_string());
                });
            },
        );
    }

    if let Some(collider) = object.get_component_mut::<Collider2D>() {
        component_block(
            ui,
            "Collider 2D",
            true,
            actions,
            TypeId::of::<Collider2D>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                bool_row(ui, "Enabled", &mut collider.enabled);
                bool_row(ui, "Is Trigger", &mut collider.is_trigger);
                property_row(ui, "Half Size", |ui| {
                    ui.add_sized(
                        [78.0, 22.0],
                        egui::DragValue::new(&mut collider.half_size.x)
                            .range(0.0..=100000.0)
                            .speed(0.05),
                    );
                    ui.add_sized(
                        [78.0, 22.0],
                        egui::DragValue::new(&mut collider.half_size.y)
                            .range(0.0..=100000.0)
                            .speed(0.05),
                    );
                });
                property_row(ui, "Size", |ui| {
                    let mut width = collider.half_size.x * 2.0;
                    let mut height = collider.half_size.y * 2.0;
                    let width_changed = ui
                        .add_sized(
                            [78.0, 22.0],
                            egui::DragValue::new(&mut width)
                                .range(0.0..=200000.0)
                                .speed(0.1),
                        )
                        .changed();
                    let height_changed = ui
                        .add_sized(
                            [78.0, 22.0],
                            egui::DragValue::new(&mut height)
                                .range(0.0..=200000.0)
                                .speed(0.1),
                        )
                        .changed();
                    if width_changed {
                        collider.half_size.x = (width * 0.5).max(0.0);
                    }
                    if height_changed {
                        collider.half_size.y = (height * 0.5).max(0.0);
                    }
                });
            },
        );
    }

    if let Some(collision) = object.get_component_mut::<PhysicsCollision>() {
        component_block(
            ui,
            "Physics Collision",
            true,
            actions,
            TypeId::of::<PhysicsCollision>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                bool_row(ui, "Enabled", &mut collision.enabled);
                property_row(ui, "Size", |ui| {
                    ui.add_sized(
                        [78.0, 22.0],
                        egui::DragValue::new(&mut collision.size.x).speed(0.05),
                    );
                    ui.add_sized(
                        [78.0, 22.0],
                        egui::DragValue::new(&mut collision.size.y).speed(0.05),
                    );
                });
            },
        );
    }

    if let Some(ui_renderer) = object.get_component_mut::<UiRenderer>() {
        let mut open_ui_editor: Option<String> = None;
        component_block(
            ui,
            "UI Renderer",
            true,
            actions,
            TypeId::of::<UiRenderer>(),
            ComponentRuntimeKind::Component,
            None,
            None,
            editor_settings,
            |ui| {
                editable_asset_path(ui, "UI Asset", &mut ui_renderer.ui_asset_path);
                property_row(ui, "Actions", |ui| {
                    if ui.button("Choose UI...").clicked() {
                        let path = pick_asset_file(project_root, &["ron"]);
                        if let Some(p) = path {
                            // Only accept .ui.ron files
                            if p.ends_with(".ui.ron") {
                                ui_renderer.ui_asset_path = Some(p);
                            } else {
                                ui_renderer.ui_asset_path = Some(p);
                            }
                        }
                    }
                    if ui.button("Clear").clicked() {
                        ui_renderer.ui_asset_path = None;
                        ui_renderer.clear();
                    }
                    if ui.button("Open In Editor").clicked() {
                        if let Some(path) = &ui_renderer.ui_asset_path {
                            open_ui_editor = Some(path.clone());
                        }
                    }
                    if ui.button("New UI...").clicked() {
                        open_ui_editor = Some(String::new());
                    }
                });
                ui.label("Space: Screen (fixed)");
            },
        );
        if let Some(path) = open_ui_editor {
            actions.open_ui_editor = Some(path);
        }
    }

    for info in component_infos {
        if is_supported_component_type(info.type_id()) {
            continue;
        }

        generic_serialized_component_block(
            ui,
            object,
            short_type_name(info.type_name()),
            true,
            actions,
            info.type_id(),
            ComponentRuntimeKind::Component,
            false,
            None,
            None,
            editor_settings,
        );
    }

    serialized_storage_entries_section(
        ui,
        object,
        actions,
        SerializedTypeKind::Component,
        ComponentRuntimeKind::Component,
        false,
        None,
        editor_settings,
    );
}

fn scripts_section(
    ui: &mut Ui,
    object: &mut Object,
    scripts_dir: Option<&Path>,
    editor_settings: &crate::editor_settings::EditorSettings,
    actions: &mut InspectorActions,
) {
    ui.heading(format!(
        "Scripts - {}",
        type_count(object, SerializedTypeKind::Script)
    ));

    let mut script_infos: Vec<_> = object
        .component_infos()
        .into_iter()
        .filter(|info| info.kind() == ComponentRuntimeKind::Script)
        .collect();
    script_infos.sort_by(|left, right| left.type_name().cmp(right.type_name()));

    let has_serialized_scripts = object
        .get_component::<SerializedTypeStorage>()
        .map(|storage| {
            storage
                .entries_of_kind(SerializedTypeKind::Script)
                .next()
                .is_some()
        })
        .unwrap_or(false);
    if script_infos.is_empty() && !has_serialized_scripts {
        ui.label("No scripts attached.");
    }

    for info in script_infos {
        generic_serialized_component_block(
            ui,
            object,
            short_type_name(info.type_name()),
            true,
            actions,
            info.type_id(),
            ComponentRuntimeKind::Script,
            true,
            Some(info.type_name()),
            scripts_dir,
            editor_settings,
        );
    }

    serialized_storage_entries_section(
        ui,
        object,
        actions,
        SerializedTypeKind::Script,
        ComponentRuntimeKind::Script,
        true,
        scripts_dir,
        editor_settings,
    );
}

fn serialized_storage_entries_section(
    ui: &mut Ui,
    object: &mut Object,
    actions: &mut InspectorActions,
    kind: SerializedTypeKind,
    runtime_kind: ComponentRuntimeKind,
    read_only: bool,
    scripts_dir: Option<&Path>,
    editor_settings: &crate::editor_settings::EditorSettings,
) {
    let entries = object
        .get_component::<SerializedTypeStorage>()
        .map(|storage| storage.entries_of_kind(kind).cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    for entry in entries {
        let title = short_type_name(&entry.type_name).to_string();
        let remove_id = egui::Id::new((
            "remove_serialized_component",
            kind as u8,
            entry.type_name.clone(),
        ));
        let icon_name = component_icon_name(TypeId::of::<SerializedTypeStorage>(), runtime_kind);
        let icon = load_component_icon(
            ui.ctx(),
            &format!("inspector_component_icon_{icon_name}"),
            icon_name,
        );
        let card_id = ui.make_persistent_id((
            "serialized_component_card",
            kind as u8,
            entry.type_name.clone(),
        ));
        let state = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            card_id,
            true,
        );

        let frame = egui::Frame {
            fill: style::COMPONENT_BACKGROUND,
            ..egui::Frame::group(ui.style())
        };
        frame.show(ui, |ui| {
            state
                .show_header(ui, |ui| {
                    ui.add(
                        egui::Image::new(&icon)
                            .fit_to_exact_size(egui::vec2(18.0, 18.0))
                            .sense(egui::Sense::hover()),
                    );
                    ui.label(
                        RichText::new(&title)
                            .text_style(egui::TextStyle::Name("component_title".into()))
                            .strong()
                            .color(style::COMPONENT_TITLE_COLOR),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if runtime_kind == ComponentRuntimeKind::Script
                            && icon_action_button(
                                ui,
                                "serialized_edit_icon",
                                "edit-icon",
                                "Open Script",
                            )
                            .clicked()
                        {
                            let _ = open_script_type_in_editor(
                                &entry.type_name,
                                scripts_dir,
                                editor_settings,
                            );
                        }
                        if icon_action_button(ui, "serialized_delete_icon", "cross-icon", "Delete")
                            .clicked()
                        {
                            ui.memory_mut(|memory| memory.data.insert_temp(remove_id, true));
                        }
                    });
                })
                .body(|ui| {
                    ui.separator();
                    if let Some(storage) = object.get_component_mut::<SerializedTypeStorage>() {
                        if let Some(target) = storage.entries.iter_mut().find(|target| {
                            target.kind == kind && target.type_name == entry.type_name
                        }) {
                            if target.fields.is_empty() {
                                ui.colored_label(Color32::GRAY, "No serialized inspector fields.");
                            } else {
                                for field in &mut target.fields {
                                    if read_only {
                                        serialized_field_asset_read_only_row(ui, field);
                                    } else {
                                        serialized_field_asset_row(ui, field);
                                    }
                                }
                            }
                        }
                    }
                });
        });

        if ui.memory(|memory| memory.data.get_temp::<bool>(remove_id).unwrap_or(false)) {
            actions.removals.push(InspectorRemoval {
                target: InspectorRemovalTarget::SerializedType {
                    kind,
                    type_name: entry.type_name.clone(),
                },
            });
            ui.memory_mut(|memory| memory.data.remove::<bool>(remove_id));
        }
    }
}

fn generic_serialized_component_block(
    ui: &mut Ui,
    object: &mut Object,
    title: &str,
    removable: bool,
    actions: &mut InspectorActions,
    type_id: TypeId,
    kind: ComponentRuntimeKind,
    read_only: bool,
    source_type_name: Option<&str>,
    scripts_dir: Option<&Path>,
    editor_settings: &crate::editor_settings::EditorSettings,
) {
    component_block(
        ui,
        title,
        removable,
        actions,
        type_id,
        kind,
        source_type_name,
        scripts_dir,
        editor_settings,
        |ui| {
            let Some(fields) = object
                .with_component_by_type_id(type_id, |component| component.serialized_fields())
            else {
                ui.colored_label(style::ERROR_COLOR, "Component is no longer attached.");
                return;
            };

            if fields.is_empty() {
                ui.colored_label(Color32::GRAY, "No serialized inspector fields.");
                return;
            }

            for field in fields {
                if read_only {
                    serialized_field_read_only_row(ui, field);
                } else {
                    serialized_field_row(ui, object, type_id, field);
                }
            }
        },
    );
}

fn serialized_field_row(ui: &mut Ui, object: &mut Object, type_id: TypeId, field: SerializedField) {
    match field.value {
        SerializedFieldValue::Bool(mut value) => {
            let mut changed = false;
            property_row(ui, &field.name, |ui| {
                changed = ui.checkbox(&mut value, "").changed();
            });
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component.set_serialized_field(&field.name, SerializedFieldValue::Bool(value))
                });
            }
        }
        SerializedFieldValue::I32(mut value) => {
            let mut changed = false;
            property_row(ui, &field.name, |ui| {
                changed = ui
                    .add_sized([96.0, 22.0], egui::DragValue::new(&mut value).speed(1.0))
                    .changed();
            });
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component.set_serialized_field(&field.name, SerializedFieldValue::I32(value))
                });
            }
        }
        SerializedFieldValue::I64(mut value) => {
            let mut changed = false;
            property_row(ui, &field.name, |ui| {
                changed = ui
                    .add_sized([96.0, 22.0], egui::DragValue::new(&mut value).speed(1.0))
                    .changed();
            });
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component.set_serialized_field(&field.name, SerializedFieldValue::I64(value))
                });
            }
        }
        SerializedFieldValue::U32(mut value) => {
            let mut changed = false;
            property_row(ui, &field.name, |ui| {
                changed = ui
                    .add_sized([96.0, 22.0], egui::DragValue::new(&mut value).speed(1.0))
                    .changed();
            });
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component.set_serialized_field(&field.name, SerializedFieldValue::U32(value))
                });
            }
        }
        SerializedFieldValue::U64(mut value) => {
            let mut changed = false;
            property_row(ui, &field.name, |ui| {
                changed = ui
                    .add_sized([96.0, 22.0], egui::DragValue::new(&mut value).speed(1.0))
                    .changed();
            });
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component.set_serialized_field(&field.name, SerializedFieldValue::U64(value))
                });
            }
        }
        SerializedFieldValue::F32(mut value) => {
            let mut changed = false;
            property_row(ui, &field.name, |ui| {
                changed = ui
                    .add_sized([96.0, 22.0], egui::DragValue::new(&mut value).speed(0.05))
                    .changed();
            });
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component.set_serialized_field(&field.name, SerializedFieldValue::F32(value))
                });
            }
        }
        SerializedFieldValue::F64(mut value) => {
            let mut changed = false;
            property_row(ui, &field.name, |ui| {
                changed = ui
                    .add_sized([96.0, 22.0], egui::DragValue::new(&mut value).speed(0.05))
                    .changed();
            });
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component.set_serialized_field(&field.name, SerializedFieldValue::F64(value))
                });
            }
        }
        SerializedFieldValue::String(mut value) => {
            let mut changed = false;
            property_row(ui, &field.name, |ui| {
                changed = ui
                    .add_sized(
                        [ui.available_width().max(120.0), 22.0],
                        egui::TextEdit::singleline(&mut value),
                    )
                    .changed();
            });
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component.set_serialized_field(&field.name, SerializedFieldValue::String(value))
                });
            }
        }
        SerializedFieldValue::Vec2(mut value) => {
            let mut changed = false;
            property_row(ui, &field.name, |ui| {
                changed |= axis_drag(ui, "X", &mut value[0], 0.05);
                changed |= axis_drag(ui, "Y", &mut value[1], 0.05);
            });
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component.set_serialized_field(&field.name, SerializedFieldValue::Vec2(value))
                });
            }
        }
        SerializedFieldValue::Vec3(mut value) => {
            let mut changed = false;
            if is_color_field(&field.name) {
                color_vec3_editor(ui, &field.name, &mut value, &mut changed);
            } else {
                property_row(ui, &field.name, |ui| {
                    changed |= axis_drag(ui, "X", &mut value[0], 0.05);
                    changed |= axis_drag(ui, "Y", &mut value[1], 0.05);
                    changed |= axis_drag(ui, "Z", &mut value[2], 0.05);
                });
            }
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component.set_serialized_field(&field.name, SerializedFieldValue::Vec3(value))
                });
            }
        }
        SerializedFieldValue::ObjectRef(mut value) => {
            let mut changed = false;
            property_row(ui, &field.name, |ui| {
                let world = match object.get_world() {
                    Some(w) => w.borrow().object_names_sorted(),
                    None => Vec::new(),
                };
                let current_label = if value.is_empty() {
                    "None".to_string()
                } else {
                    value.clone()
                };
                egui::ComboBox::from_id_salt(format!("objref_{}", &field.name))
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(value.is_empty(), "None").clicked() {
                            value.clear();
                            changed = true;
                        }
                        for obj_name in &world {
                            let selected = obj_name == &value;
                            if ui.selectable_label(selected, obj_name).clicked() {
                                value = obj_name.clone();
                                changed = true;
                            }
                        }
                    });
            });
            if changed {
                let _ = object.with_component_mut_by_type_id(type_id, |component| {
                    component
                        .set_serialized_field(&field.name, SerializedFieldValue::ObjectRef(value))
                });
            }
        }
    }
}

fn serialized_field_read_only_row(ui: &mut Ui, field: SerializedField) {
    property_row(ui, &humanize_field_name(&field.name), |ui| {
        ui.label(serialized_field_value_text(&field.value));
    });
}

fn serialized_field_asset_row(ui: &mut Ui, field: &mut SerializedField) {
    match &mut field.value {
        SerializedFieldValue::Bool(value) => {
            property_row(ui, &field.name, |ui| {
                ui.checkbox(value, "");
            });
        }
        SerializedFieldValue::I32(value) => {
            property_row(ui, &field.name, |ui| {
                ui.add_sized([96.0, 22.0], egui::DragValue::new(value).speed(1.0));
            });
        }
        SerializedFieldValue::I64(value) => {
            property_row(ui, &field.name, |ui| {
                ui.add_sized([96.0, 22.0], egui::DragValue::new(value).speed(1.0));
            });
        }
        SerializedFieldValue::U32(value) => {
            property_row(ui, &field.name, |ui| {
                ui.add_sized([96.0, 22.0], egui::DragValue::new(value).speed(1.0));
            });
        }
        SerializedFieldValue::U64(value) => {
            property_row(ui, &field.name, |ui| {
                ui.add_sized([96.0, 22.0], egui::DragValue::new(value).speed(1.0));
            });
        }
        SerializedFieldValue::F32(value) => {
            property_row(ui, &field.name, |ui| {
                ui.add_sized([96.0, 22.0], egui::DragValue::new(value).speed(0.05));
            });
        }
        SerializedFieldValue::F64(value) => {
            property_row(ui, &field.name, |ui| {
                ui.add_sized([96.0, 22.0], egui::DragValue::new(value).speed(0.05));
            });
        }
        SerializedFieldValue::String(value) => {
            property_row(ui, &field.name, |ui| {
                ui.add_sized(
                    [ui.available_width().max(120.0), 22.0],
                    egui::TextEdit::singleline(value),
                );
            });
        }
        SerializedFieldValue::Vec2(value) => {
            property_row(ui, &field.name, |ui| {
                axis_drag(ui, "X", &mut value[0], 0.05);
                axis_drag(ui, "Y", &mut value[1], 0.05);
            });
        }
        SerializedFieldValue::Vec3(value) => {
            if is_color_field(&field.name) {
                let mut changed = false;
                color_vec3_editor(ui, &field.name, value, &mut changed);
            } else {
                property_row(ui, &field.name, |ui| {
                    axis_drag(ui, "X", &mut value[0], 0.05);
                    axis_drag(ui, "Y", &mut value[1], 0.05);
                    axis_drag(ui, "Z", &mut value[2], 0.05);
                });
            }
        }
        SerializedFieldValue::ObjectRef(value) => {
            let selected_text = if value.is_empty() {
                "None".to_string()
            } else {
                value.clone()
            };
            property_row(ui, &field.name, |ui| {
                egui::ComboBox::from_id_salt(format!("objref_asset_{}", &field.name))
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(value.is_empty(), "None").clicked() {
                            *value = String::new();
                        }
                    });
            });
        }
    }
}

fn serialized_field_asset_read_only_row(ui: &mut Ui, field: &SerializedField) {
    property_row(ui, &humanize_field_name(&field.name), |ui| {
        ui.label(serialized_field_value_text(&field.value));
    });
}

fn vec3_editor(ui: &mut Ui, label: &str, value: &mut Vec3) {
    property_row(ui, label, |ui| {
        axis_drag(ui, "X", &mut value.x, 0.05);
        axis_drag(ui, "Y", &mut value.y, 0.05);
        axis_drag(ui, "Z", &mut value.z, 0.05);
    });
}

fn quat_editor(ui: &mut Ui, transform: &mut Transform) {
    let (mut x, mut y, mut z) = transform.rotation.to_euler(EulerRot::XYZ);
    x = x.to_degrees();
    y = y.to_degrees();
    z = z.to_degrees();

    let mut changed = false;
    property_row(ui, "Rotation", |ui| {
        changed |= axis_drag(ui, "X", &mut x, 0.5);
        changed |= axis_drag(ui, "Y", &mut y, 0.5);
        changed |= axis_drag(ui, "Z", &mut z, 0.5);
    });

    if changed {
        transform.rotation = Quat::from_euler(
            EulerRot::XYZ,
            x.to_radians(),
            y.to_radians(),
            z.to_radians(),
        );
    }
}

fn color_editor(ui: &mut Ui, label: &str, color: &mut [f32; 4]) {
    property_row(ui, label, |ui| {
        ui.color_edit_button_rgba_unmultiplied(color);
    });
}

fn color_vec3_editor(ui: &mut Ui, label: &str, color: &mut [f32; 3], changed: &mut bool) {
    property_row(ui, label, |ui| {
        *changed |= ui.color_edit_button_rgb(color).changed();
    });
}

fn is_color_field(name: &str) -> bool {
    name.to_ascii_lowercase().contains("color")
}

fn component_badge(ui: &mut Ui, label: &str, description: &str) {
    ui.group(|ui| {
        ui.label(RichText::new(label).strong());
        ui.label(description);
    });
}

fn component_block(
    ui: &mut Ui,
    title: &str,
    removable: bool,
    actions: &mut InspectorActions,
    type_id: TypeId,
    kind: ComponentRuntimeKind,
    source_type_name: Option<&str>,
    scripts_dir: Option<&Path>,
    editor_settings: &crate::editor_settings::EditorSettings,
    body: impl FnOnce(&mut Ui),
) {
    let removal = if removable {
        Some((type_id, title.to_string()))
    } else {
        None
    };
    component_card(
        ui,
        type_id,
        kind,
        title,
        removable,
        removal,
        source_type_name,
        scripts_dir,
        editor_settings,
        body,
    );
    if removable {
        if ui.memory(|memory| {
            memory
                .data
                .get_temp::<bool>(egui::Id::new(("remove_component", type_id)))
                .unwrap_or(false)
        }) {
            actions.removals.push(InspectorRemoval {
                target: InspectorRemovalTarget::RuntimeType {
                    type_id,
                    type_name: title.to_string(),
                },
            });
            ui.memory_mut(|memory| {
                memory
                    .data
                    .remove::<bool>(egui::Id::new(("remove_component", type_id)))
            });
        }
    }
}

fn component_card(
    ui: &mut Ui,
    type_id: TypeId,
    kind: ComponentRuntimeKind,
    title: &str,
    removable: bool,
    removal: Option<(TypeId, String)>,
    source_type_name: Option<&str>,
    scripts_dir: Option<&Path>,
    editor_settings: &crate::editor_settings::EditorSettings,
    body: impl FnOnce(&mut Ui),
) {
    let icon_name = component_icon_name(type_id, kind);
    let icon = load_component_icon(
        ui.ctx(),
        &format!("inspector_component_icon_{icon_name}"),
        icon_name,
    );
    let card_id = ui.make_persistent_id(("component_card", type_id));
    let state =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), card_id, true);

    let frame = egui::Frame {
        fill: style::COMPONENT_BACKGROUND,
        ..egui::Frame::group(ui.style())
    };
    frame.show(ui, |ui| {
        state
            .show_header(ui, |ui| {
                ui.add(
                    egui::Image::new(&icon)
                        .fit_to_exact_size(egui::vec2(
                            style::spacing::COMPONENT_ICON_SIZE,
                            style::spacing::COMPONENT_ICON_SIZE,
                        ))
                        .sense(egui::Sense::hover()),
                );
                ui.label(
                    RichText::new(title)
                        .text_style(egui::TextStyle::Name("component_title".into()))
                        .strong()
                        .color(style::COMPONENT_TITLE_COLOR),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if kind == ComponentRuntimeKind::Component {
                        if let Some(url) = component_docs_url(type_id) {
                            if icon_action_button(
                                ui,
                                "component_docs_icon",
                                "question-icon",
                                "Open Docs",
                            )
                            .clicked()
                            {
                                let _ = open_external_target(url);
                            }
                        }
                    }
                    if kind == ComponentRuntimeKind::Script {
                        if let Some(type_name) = source_type_name {
                            if icon_action_button(
                                ui,
                                "component_edit_icon",
                                "edit-icon",
                                "Open Script",
                            )
                            .clicked()
                            {
                                let _ = open_script_type_in_editor(
                                    type_name,
                                    scripts_dir,
                                    editor_settings,
                                );
                            }
                        }
                    }
                    if removable
                        && icon_action_button(ui, "component_delete_icon", "cross-icon", "Delete")
                            .clicked()
                    {
                        if let Some((remove_type_id, _)) = &removal {
                            ui.memory_mut(|memory| {
                                memory.data.insert_temp(
                                    egui::Id::new(("remove_component", *remove_type_id)),
                                    true,
                                );
                            });
                        }
                    }
                });
            })
            .body(|ui| {
                ui.separator();
                body(ui);
            });
    });
}

fn axis_drag(ui: &mut Ui, label: &str, value: &mut f32, speed: f64) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed |= ui
            .add_sized([78.0, 22.0], egui::DragValue::new(value).speed(speed))
            .changed();
    });
    changed
}

fn property_row(ui: &mut Ui, label: &str, body: impl FnOnce(&mut Ui)) {
    let width_id = egui::Id::new("inspector_property_label_width");
    let mut label_width = ui
        .ctx()
        .data_mut(|data| data.get_persisted::<f32>(width_id))
        .unwrap_or(120.0)
        .clamp(90.0, 280.0);

    ui.horizontal(|ui| {
        ui.add_sized([label_width, 22.0], egui::Label::new(label));
        let (drag_rect, drag_response) =
            ui.allocate_exact_size(egui::vec2(6.0, 22.0), egui::Sense::click_and_drag());
        if drag_response.hovered() || drag_response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if drag_response.dragged() {
            label_width = (label_width + drag_response.drag_delta().x).clamp(90.0, 280.0);
            ui.ctx()
                .data_mut(|data| data.insert_persisted(width_id, label_width));
        }
        ui.painter().line_segment(
            [drag_rect.center_top(), drag_rect.center_bottom()],
            egui::Stroke::new(1.0, ui.visuals().widgets.inactive.bg_stroke.color),
        );
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_width(ui.available_width());
            body(ui);
        });
    });
}

fn icon_action_button(
    ui: &mut Ui,
    texture_name: &str,
    icon_name: &str,
    tooltip: &str,
) -> egui::Response {
    let icon = load_editor_icon(ui.ctx(), texture_name, icon_name);
    ui.add(
        egui::Button::image(egui::Image::new(&icon).fit_to_exact_size(egui::vec2(14.0, 14.0)))
            .frame(false),
    )
    .on_hover_text(tooltip)
}

fn serialized_field_value_text(value: &SerializedFieldValue) -> String {
    match value {
        SerializedFieldValue::Bool(value) => value.to_string(),
        SerializedFieldValue::I32(value) => value.to_string(),
        SerializedFieldValue::I64(value) => value.to_string(),
        SerializedFieldValue::U32(value) => value.to_string(),
        SerializedFieldValue::U64(value) => value.to_string(),
        SerializedFieldValue::F32(value) => format!("{value:.3}"),
        SerializedFieldValue::F64(value) => format!("{value:.3}"),
        SerializedFieldValue::String(value) => value.clone(),
        SerializedFieldValue::Vec2(value) => {
            format!("X {:.3}  Y {:.3}", value[0], value[1])
        }
        SerializedFieldValue::Vec3(value) => {
            format!("X {:.3}  Y {:.3}  Z {:.3}", value[0], value[1], value[2])
        }
        SerializedFieldValue::ObjectRef(value) => {
            if value.is_empty() {
                "None (ObjectRef)".to_string()
            } else {
                format!("{} (ObjectRef)", value)
            }
        }
    }
}

fn humanize_field_name(name: &str) -> String {
    let mut result = String::new();
    let mut previous_was_space = true;

    for ch in name.chars() {
        if ch == '_' || ch == '-' {
            if !result.ends_with(' ') {
                result.push(' ');
            }
            previous_was_space = true;
            continue;
        }

        if previous_was_space {
            result.extend(ch.to_uppercase());
            previous_was_space = false;
        } else {
            result.push(ch);
        }
    }

    result.trim().to_string()
}

pub(crate) fn component_docs_url(type_id: TypeId) -> Option<&'static str> {
    if type_id == TypeId::of::<Transform>() {
        Some("https://github.com/RunaLight/runa/blob/main/docs/tutorials/components/transform.md")
    } else if type_id == TypeId::of::<SpriteRenderer>() {
        Some("https://github.com/RunaLight/runa/blob/main/docs/tutorials/components/sprite-renderer.md")
    } else if type_id == TypeId::of::<SpriteAnimator>() {
        Some("https://github.com/RunaLight/runa/blob/main/docs/tutorials/components/sprite-animator.md")
    } else if type_id == TypeId::of::<Sorting>() {
        Some("https://github.com/RunaLight/runa/blob/main/docs/tutorials/components/sorting.md")
    } else if type_id == TypeId::of::<CursorInteractable>() {
        Some("https://github.com/RunaLight/runa/blob/main/docs/tutorials/components/cursor-interactable.md")
    } else if type_id == TypeId::of::<Collider2D>() {
        Some("https://github.com/RunaLight/runa/blob/main/docs/tutorials/components/physics-collision.md")
    } else if type_id == TypeId::of::<PhysicsCollision>() {
        Some("https://github.com/RunaLight/runa/blob/main/docs/tutorials/components/physics-collision.md")
    } else {
        Some("https://github.com/RunaLight/runa/blob/main/docs/tutorials/README.md")
    }
}

fn open_script_type_in_editor(
    type_name: &str,
    scripts_dir: Option<&Path>,
    editor_settings: &crate::editor_settings::EditorSettings,
) -> Result<(), String> {
    let Some(scripts_dir) = scripts_dir else {
        return Err("Project scripts directory is unavailable.".to_string());
    };
    let Some(script_path) = find_script_source_path(scripts_dir, short_type_name(type_name)) else {
        return Err(format!(
            "Script source for {} was not found.",
            short_type_name(type_name)
        ));
    };
    open_file_in_external_editor(&script_path, editor_settings)
}

fn open_file_in_external_editor(
    path: &Path,
    editor_settings: &crate::editor_settings::EditorSettings,
) -> Result<(), String> {
    let executable = editor_settings.external_editor_executable.trim();
    if executable.is_empty() {
        return Err("External editor is not configured.".to_string());
    }

    let file = path.to_string_lossy().to_string();
    let args: Vec<String> = editor_settings
        .external_editor_args
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.replace("{file}", &file))
        .collect();

    Command::new(executable)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to open external editor: {error}"))
}

pub(crate) fn open_external_target(target: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", target])
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Failed to open target: {error}"))
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Failed to open target: {error}"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Failed to open target: {error}"))
    }
}

fn find_script_source_path(scripts_dir: &Path, type_name: &str) -> Option<PathBuf> {
    let preferred_name = format!("{}.rs", to_snake_case(type_name));
    let mut files = Vec::new();
    collect_rust_files(scripts_dir, &mut files);

    if let Some(path) = files.iter().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case(&preferred_name))
            .unwrap_or(false)
    }) {
        return Some(path.clone());
    }

    files.into_iter().find(|path| {
        fs::read_to_string(path)
            .map(|content| {
                content.contains(&format!("struct {type_name}"))
                    || content.contains(&format!("impl Script for {type_name}"))
            })
            .unwrap_or(false)
    })
}

fn collect_rust_files(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn to_snake_case(value: &str) -> String {
    let mut result = String::new();
    let mut previous_was_lowercase_or_digit = false;

    for ch in value.chars() {
        if ch.is_ascii_uppercase() {
            if previous_was_lowercase_or_digit && !result.ends_with('_') {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
            previous_was_lowercase_or_digit = false;
        } else if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            previous_was_lowercase_or_digit = true;
        } else if !result.ends_with('_') && !result.is_empty() {
            result.push('_');
            previous_was_lowercase_or_digit = false;
        }
    }

    result.trim_matches('_').to_string()
}

fn bool_row(ui: &mut Ui, label: &str, value: &mut bool) {
    property_row(ui, label, |ui| {
        ui.checkbox(value, "");
    });
}

fn component_icon_name(type_id: TypeId, kind: ComponentRuntimeKind) -> &'static str {
    if type_id == TypeId::of::<Transform>() {
        "c-Transform"
    } else if type_id == TypeId::of::<Camera>() {
        "c-Camera"
    } else if type_id == TypeId::of::<ActiveCamera>() {
        "c-ActiveCamera"
    } else if type_id == TypeId::of::<AudioSource>() {
        "c-AudioSource"
    } else if type_id == TypeId::of::<AudioListener>() {
        "c-AudioListener"
    } else if type_id == TypeId::of::<Collider2D>() {
        "c-Collider2D"
    } else if type_id == TypeId::of::<PhysicsCollision>() {
        "c-PhysicsCollision"
    } else if type_id == TypeId::of::<CursorInteractable>() {
        "c-CursorInteractable"
    } else if type_id == TypeId::of::<UiRenderer>() {
        "c-Canvas"
    } else if type_id == TypeId::of::<MeshRenderer>() {
        "c-MeshRenderer"
    } else if type_id == TypeId::of::<DirectionalLight>() {
        "c-DirectionalLight"
    } else if type_id == TypeId::of::<PointLight>() {
        "c-PointLight"
    } else if type_id == TypeId::of::<SpriteRenderer>() {
        "c-SpriteRenderer"
    } else if type_id == TypeId::of::<SpriteAnimator>() {
        "c-SpriteAnimator"
    } else if type_id == TypeId::of::<Sorting>() {
        "c-Sorting"
    } else if type_id == TypeId::of::<TilemapRenderer>() {
        "c-TilemapRenderer"
    } else if kind == ComponentRuntimeKind::Script {
        "c-Script"
    } else {
        "c-Object"
    }
}

fn is_supported_component_type(type_id: TypeId) -> bool {
    [
        TypeId::of::<Camera>(),
        TypeId::of::<ActiveCamera>(),
        TypeId::of::<MeshRenderer>(),
        TypeId::of::<SpriteRenderer>(),
        TypeId::of::<SpriteAnimator>(),
        TypeId::of::<Sorting>(),
        TypeId::of::<TilemapRenderer>(),
        TypeId::of::<Collider2D>(),
        TypeId::of::<AudioSource>(),
        TypeId::of::<AudioListener>(),
        TypeId::of::<CursorInteractable>(),
        TypeId::of::<PhysicsCollision>(),
        TypeId::of::<UiRenderer>(),
    ]
    .contains(&type_id)
}

fn mesh_extents(mesh: &runa_core::components::Mesh) -> Vec3 {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for vertex in &mesh.vertices {
        let p = Vec3::from_array(vertex.position);
        min = min.min(p);
        max = max.max(p);
    }
    (max - min).abs().max(Vec3::splat(1.0))
}

fn build_builtin_mesh_from_extents(
    primitive: BuiltinMeshPrimitive,
    extents: Vec3,
) -> runa_core::components::Mesh {
    match primitive {
        BuiltinMeshPrimitive::Cube => {
            runa_core::components::Mesh::cube(extents.x.max(extents.y).max(extents.z))
        }
        BuiltinMeshPrimitive::Quad => {
            runa_core::components::Mesh::quad(extents.x.max(0.01), extents.y.max(0.01))
        }
        BuiltinMeshPrimitive::Plane => {
            runa_core::components::Mesh::plane(extents.x.max(0.01), extents.z.max(0.01))
        }
        BuiltinMeshPrimitive::Pyramid => runa_core::components::Mesh::pyramid(
            extents.x.max(0.01),
            extents.y.max(0.01),
            extents.z.max(0.01),
        ),
    }
}

fn short_type_name(type_name: &str) -> &str {
    type_name.rsplit("::").next().unwrap_or(type_name)
}

fn editable_asset_path_inline(ui: &mut Ui, path: &mut Option<String>) {
    let mut buffer = path.clone().unwrap_or_default();
    ui.add_enabled(
        false,
        egui::TextEdit::singleline(&mut buffer).desired_width(ui.available_width().max(120.0)),
    );
}

fn sync_tilemap_tile_size_from_atlas(tilemap: &mut Tilemap) {
    let Some(atlas) = tilemap.atlas.as_ref() else {
        return;
    };
    tilemap.tile_size = USizeVec2::new(
        (atlas.texture.width / atlas.columns.max(1)).max(1) as usize,
        (atlas.texture.height / atlas.rows.max(1)).max(1) as usize,
    );
    tilemap.selected_tile = tilemap
        .selected_tile
        .min(tilemap.atlas_frame_count().saturating_sub(1));
}

fn tilemap_paint_ui(ui: &mut Ui, tilemap: &mut Tilemap, tile_paint: &mut TilePaintToolState) {
    property_row(ui, "Paint Layer", |ui| {
        let max_layer = tilemap.layers.len().saturating_sub(1) as u32;
        tile_paint.layer = tile_paint.layer.min(max_layer);
        ui.add_sized(
            [96.0, 22.0],
            egui::DragValue::new(&mut tile_paint.layer).range(0..=max_layer),
        );
    });
    property_row(ui, "Tool", |ui| {
        ui.selectable_value(&mut tile_paint.mode, TilePaintMode::None, "None");
        ui.selectable_value(&mut tile_paint.mode, TilePaintMode::Paint, "Paint");
        ui.selectable_value(&mut tile_paint.mode, TilePaintMode::Erase, "Erase");
    });
    property_row(ui, "Palette", |ui| {
        if ui
            .add_enabled(tilemap.atlas.is_some(), egui::Button::new("Open Palette"))
            .clicked()
        {
            tile_paint.palette_open = true;
        }
        if tilemap.atlas.is_some() {
            ui.label(format!("Selected {}", tilemap.selected_tile));
        }
    });
}

fn editable_asset_path(ui: &mut Ui, label: &str, path: &mut Option<String>) {
    let mut buffer = path.clone().unwrap_or_default();
    let mut changed = false;
    property_row(ui, label, |ui| {
        changed = ui
            .add_sized(
                [ui.available_width().max(120.0), 22.0],
                egui::TextEdit::singleline(&mut buffer),
            )
            .changed();
    });
    if changed {
        let trimmed = buffer.trim();
        *path = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };
    }
}

fn pick_asset_file(project_root: Option<&Path>, extensions: &[&str]) -> Option<String> {
    let mut dialog = FileDialog::new();
    if let Some(project_root) = project_root {
        dialog = dialog.set_directory(project_root.join("assets"));
    }
    let selected = dialog.add_filter("Assets", extensions).pick_file()?;
    project_root
        .and_then(|root| selected.strip_prefix(root).ok().map(path_to_string))
        .or_else(|| Some(path_to_string(&selected)))
}

fn load_texture_from_path(
    project_root: Option<&Path>,
    relative_path: &str,
) -> Result<runa_asset::Handle<runa_asset::TextureAsset>, String> {
    let Some(project_root) = project_root else {
        return Err("Open a project before assigning sprite assets.".to_string());
    };
    Ok(load_image(
        project_root.to_string_lossy().as_ref(),
        relative_path,
    ))
}

fn load_r3m_from_path(
    _project_root: Option<&Path>,
    _relative_path: &str,
) -> Result<runa_asset::Handle<RunaModel>, String> {
    let Some(_project_root) = _project_root else {
        return Err("Open a project before assigning model assets.".to_string());
    };
    // RunaModel::from_file(project_root.to_string_lossy().as_ref(), relative_path)
    //     .map(runa_asset::Handle::new)
    //     .map_err(|error| error.to_string())
    todo!("Not yet implemented")
}

fn load_audio_from_path(
    project_root: Option<&Path>,
    relative_path: &str,
) -> Result<std::sync::Arc<AudioAsset>, String> {
    let Some(project_root) = project_root else {
        return Err("Open a project before assigning audio assets.".to_string());
    };
    AudioAsset::from_file(project_root.to_string_lossy().as_ref(), relative_path)
        .map(std::sync::Arc::new)
        .map_err(|error| error.to_string())
}

fn path_to_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}
