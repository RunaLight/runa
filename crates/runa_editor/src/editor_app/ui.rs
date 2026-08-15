use super::*;

impl<'window> EditorApp<'window> {
    pub(super) fn draw_ui(&mut self) -> egui::FullOutput {
        let window = self.window.as_ref().unwrap();
        let egui_state = self.egui_state.as_mut().unwrap();
        let raw_input = egui_state.take_egui_input(window);
        let egui_ctx = self.egui_ctx.clone();
        egui_ctx.run_ui(raw_input, |ctx| {
            self.build_ui(ctx);
        })
    }

    #[allow(deprecated)]
    fn build_ui(&mut self, ctx: &egui::Context) {
        let ui_scale = ctx.zoom_factor().max(0.75);
        egui::Panel::top("editor_top_bar").show(ctx, |ui| {
            ui.set_min_height(38.0 * ui_scale);
            ui.spacing_mut().button_padding = egui::vec2(9.0 * ui_scale, 5.0 * ui_scale);
            ui.spacing_mut().interact_size.y = 28.0 * ui_scale;
            ui.columns(3, |columns| {
                columns[0].with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                    egui::MenuBar::new().ui(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("New Project...").clicked() {
                                self.project_dialog.open = true;
                                ui.close();
                            }
                            if ui.button("Open Project...").clicked() {
                                self.open_project_dialog();
                                ui.close();
                            }
                            ui.separator();
                            let project_open = self.project_session.is_some();
                            if ui
                                .add_enabled(project_open, egui::Button::new("New World"))
                                .clicked()
                            {
                                self.new_world();
                                ui.close();
                            }
                            if ui
                                .add_enabled(project_open, egui::Button::new("Open World..."))
                                .clicked()
                            {
                                self.open_world_dialog();
                                ui.close();
                            }
                            if ui
                                .add_enabled(project_open, egui::Button::new("Save World"))
                                .clicked()
                            {
                                self.save_current_world();
                                ui.close();
                            }
                            if ui
                                .add_enabled(project_open, egui::Button::new("Save World As..."))
                                .clicked()
                            {
                                self.save_world_as_dialog();
                                ui.close();
                            }
                            ui.separator();
                            if ui
                                .add_enabled(project_open, egui::Button::new("UI Editor"))
                                .clicked()
                            {
                                if let Some(session) = self.project_session.as_ref() {
                                    self.ui_editor.open_new(&session.project);
                                }
                                ui.close();
                            }
                            ui.separator();
                            if ui
                                .add_enabled(project_open, egui::Button::new("Return to Welcome"))
                                .clicked()
                            {
                                self.return_to_welcome();
                                ui.close();
                            }
                        });
                        ui.menu_button("Edit", |ui| {
                            if ui.button("Editor Settings").clicked() {
                                self.editor_settings_open = true;
                                ui.close();
                            }
                            if ui.button("Project Settings").clicked() {
                                self.project_settings_open = true;
                                ui.close();
                            }
                        });
                        ui.menu_button("Build", |ui| {
                            if ui.button("Build Settings").clicked() {
                                self.build_settings_open = true;
                                ui.close();
                            }
                            let build_enabled =
                                self.project_session.is_some() && self.build_process.is_none();
                            let build_label = if self.build_process.is_some() {
                                "Building..."
                            } else {
                                "Build Game"
                            };
                            if ui
                                .add_enabled(build_enabled, egui::Button::new(build_label))
                                .clicked()
                            {
                                self.build_project();
                                ui.close();
                            }
                        });
                        ui.menu_button("View", |ui| {
                            ui.checkbox(&mut self.panels.hierarchy, "Hierarchy");
                            ui.checkbox(&mut self.panels.inspector, "Inspector");
                            ui.checkbox(&mut self.panels.bottom_bar, "Bottom Bar");
                        });
                        ui.separator();

                        let refresh_label = if self.place_object.refresh_in_progress {
                            "Refreshing..."
                        } else {
                            "Refresh Project Metadata"
                        };
                        let refresh_response = ui.add_enabled(
                            self.project_session.is_some()
                                && !self.place_object.refresh_in_progress,
                            egui::Button::new(refresh_label),
                        );
                        if refresh_response.clicked() {
                            self.refresh_project_metadata(true);
                        }
                    });
                });

                let column_rect = columns[1].max_rect();
                let button_size = egui::vec2(28.0 * ui_scale, 28.0 * ui_scale);
                let button_rect = egui::Rect::from_center_size(column_rect.center(), button_size);
                if self.runtime_process.is_some() {
                    let stop_icon = crate::editor_textures::load_editor_icon(
                        columns[1].ctx(),
                        "top_bar_stop_icon",
                        "Stop",
                    );
                    let response =
                        columns[1].add_enabled_ui(self.project_session.is_some(), |ui| {
                            ui.put(
                                button_rect,
                                egui::Button::image(
                                    egui::Image::new(&stop_icon).fit_to_exact_size(egui::vec2(
                                        14.0 * ui_scale,
                                        14.0 * ui_scale,
                                    )),
                                )
                                .min_size(button_size),
                            )
                        });
                    if response.inner.clicked() {
                        self.stop_project();
                    }
                    response.response.on_hover_text("Stop");
                } else {
                    let play_icon = crate::editor_textures::load_editor_icon(
                        columns[1].ctx(),
                        "top_bar_play_icon",
                        "Play",
                    );
                    let response =
                        columns[1].add_enabled_ui(self.project_session.is_some(), |ui| {
                            ui.put(
                                button_rect,
                                egui::Button::image(
                                    egui::Image::new(&play_icon).fit_to_exact_size(egui::vec2(
                                        14.0 * ui_scale,
                                        14.0 * ui_scale,
                                    )),
                                )
                                .min_size(button_size),
                            )
                        });
                    if response.inner.clicked() {
                        self.play_project();
                    }
                    response.response.on_hover_text("Play In Window");
                }

                columns[2].with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(self.window_title()).strong());
                });
            });
        });

        if self.project_session.is_none() {
            self.welcome_screen(ctx);
            self.project_dialog_window(ctx);
            self.project_loading_overlay(ctx);
            self.project_version_prompt_window(ctx);
            return;
        }

        egui::Panel::bottom("status_bar")
            .resizable(false)
            .exact_size(24.0 * ui_scale)
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.label(&self.status_line);
            });

        if self.panels.bottom_bar {
            let max_bottom_bar_height = (ctx.content_rect().height() - 140.0).max(120.0);
            self.bottom_bar_height = self.bottom_bar_height.clamp(80.0, max_bottom_bar_height);

            // Position overlay above status bar with gap
            let screen_rect = ctx.screen_rect();
            let side_margin = 12.0;
            let bottom_gap = 28.0 * ui_scale;
            let right_reserved = if self.panels.inspector {
                self.inspector_panel_width + side_margin
            } else {
                0.0
            };
            let overlay_width =
                (screen_rect.width() - side_margin * 2.0 - right_reserved).max(240.0);
            let overlay_top = screen_rect.bottom() - bottom_gap - self.bottom_bar_height;
            let overlay_pos = egui::pos2(screen_rect.left() + side_margin, overlay_top);
            egui::Area::new("bottom_bar_overlay".into())
                .order(egui::Order::Middle)
                .fixed_pos(overlay_pos)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(ui.visuals().panel_fill)
                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                        .corner_radius(egui::CornerRadius::same(style::spacing::CORNER_RADIUS))
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(overlay_width, self.bottom_bar_height));
                            ui.set_max_size(egui::vec2(overlay_width, self.bottom_bar_height));

                            let handle_height = 10.0;
                            let (handle_rect, handle_response) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), handle_height),
                                egui::Sense::click_and_drag(),
                            );
                            if handle_response.hovered() || handle_response.dragged() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                            }
                            if handle_response.dragged() {
                                self.bottom_bar_height = (self.bottom_bar_height
                                    - handle_response.drag_delta().y)
                                    .clamp(80.0, max_bottom_bar_height);
                            }
                            let handle_fill = if handle_response.dragged() {
                                ui.visuals().widgets.active.bg_fill
                            } else if handle_response.hovered() {
                                ui.visuals().widgets.hovered.bg_fill
                            } else {
                                ui.visuals().widgets.inactive.bg_fill
                            };
                            ui.painter().rect_filled(
                                handle_rect,
                                4.0,
                                handle_fill.gamma_multiply(0.35),
                            );
                            ui.painter().rect_filled(
                                handle_rect.shrink2(egui::vec2(48.0, 3.0)),
                                4.0,
                                handle_fill,
                            );
                            ui.horizontal(|ui| {
                                let browser_selected = self.bottom_tab == BottomTab::ContentBrowser;
                                if ui
                                    .selectable_label(browser_selected, "Content Browser")
                                    .clicked()
                                {
                                    self.bottom_tab = BottomTab::ContentBrowser;
                                }
                                let console_selected = self.bottom_tab == BottomTab::Console;
                                if ui.selectable_label(console_selected, "Console").clicked() {
                                    self.bottom_tab = BottomTab::Console;
                                }
                                ui.separator();
                                match self.bottom_tab {
                                    BottomTab::ContentBrowser => {
                                        if ui.button("Import").clicked() {
                                            self.content_browser
                                                .import_into_current_dir(&self.settings);
                                            if let Some(message) =
                                                self.content_browser.take_message()
                                            {
                                                self.status_line = message;
                                            }
                                        }
                                        ui.separator();
                                        ui.label(self.content_browser.current_dir_display());
                                    }
                                    BottomTab::Console => {
                                        if ui.button("Clear").clicked() {
                                            self.console.clear_messages();
                                        }
                                        ui.separator();
                                        if ui.button("Clear History").clicked() {
                                            // History stays, just scroll to bottom
                                        }
                                    }
                                }
                            });
                            ui.separator();
                            let body_height = (ui.available_height() - 8.0).max(0.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), body_height),
                                Layout::top_down(egui::Align::Min),
                                |ui| match self.bottom_tab {
                                    BottomTab::ContentBrowser => {
                                        self.content_browser.ui(ui, &self.settings);
                                        if let Some(path) =
                                            self.content_browser.take_pending_world_open()
                                        {
                                            self.open_world_from_path(path);
                                        }
                                        if let Some(message) = self.content_browser.take_message() {
                                            self.status_line = message;
                                        }
                                    }
                                    BottomTab::Console => {
                                        let body_rect = ui.max_rect();
                                        let input_h = 24.0;
                                        let input_y = body_rect.bottom() - input_h;

                                        // Messages area (fills from top to input line)
                                        let messages_rect = egui::Rect::from_min_max(
                                            body_rect.left_top(),
                                            egui::pos2(body_rect.right(), input_y),
                                        );

                                        // Suggestion panel (floats above messages, will be painted as overlay)
                                        let suggestions = self.console.matching_commands();
                                        let has_suggestions = !suggestions.is_empty();
                                        let suggestion_h = 24.0;

                                        // Reserve space for input
                                        ui.allocate_ui_at_rect(messages_rect, |ui| {
                                            let scroll_h = if has_suggestions {
                                                ui.available_height() - suggestion_h - 2.0
                                            } else {
                                                ui.available_height()
                                            };

                                            // Scrollable messages
                                            egui::ScrollArea::vertical()
                                                .auto_shrink([false, false])
                                                .stick_to_bottom(true)
                                                .id_salt("editor_console_scroll")
                                                .show(ui, |ui| {
                                                    ui.set_min_height(scroll_h.max(0.0));
                                                    for msg in self.console.messages() {
                                                        ui.monospace(msg);
                                                    }
                                                });

                                            // Suggestion overlay at bottom of messages area
                                            if has_suggestions {
                                                let sel = self.console.selected_suggestion();
                                                let max = ui.max_rect();
                                                let suggest_rect = egui::Rect::from_min_max(
                                                    egui::pos2(
                                                        max.left(),
                                                        max.bottom() - suggestion_h - 2.0,
                                                    ),
                                                    egui::pos2(max.right(), max.bottom()),
                                                );
                                                let painter = ui.painter_at(suggest_rect);
                                                painter.rect_filled(
                                                    suggest_rect,
                                                    2.0,
                                                    egui::Color32::from_black_alpha(230),
                                                );
                                                painter.rect_stroke(
                                                    suggest_rect,
                                                    2.0,
                                                    egui::Stroke::new(
                                                        1.0,
                                                        egui::Color32::from_rgb(60, 100, 180),
                                                    ),
                                                    egui::StrokeKind::Inside,
                                                );
                                                let mut x = suggest_rect.left() + 4.0;
                                                let text_y = suggest_rect.top() + 2.0;
                                                for (i, s) in
                                                    suggestions.iter().enumerate().take(20)
                                                {
                                                    let text = if sel == Some(i) {
                                                        format!("[{}]", s)
                                                    } else {
                                                        s.clone()
                                                    };
                                                    let color = if sel == Some(i) {
                                                        egui::Color32::from_rgb(200, 220, 255)
                                                    } else {
                                                        egui::Color32::from_rgb(150, 200, 255)
                                                    };
                                                    let font_id = egui::FontId::monospace(14.0);
                                                    let galley = painter
                                                        .layout_no_wrap(text, font_id, color);
                                                    let w = galley.size().x;
                                                    painter.galley(
                                                        egui::pos2(x, text_y),
                                                        galley,
                                                        color,
                                                    );
                                                    x += w + 8.0;
                                                }
                                            }
                                        });

                                        // Input line at the very bottom
                                        ui.allocate_ui_at_rect(
                                            egui::Rect::from_min_max(
                                                egui::pos2(body_rect.left(), input_y),
                                                egui::pos2(body_rect.right(), body_rect.bottom()),
                                            ),
                                            |ui| {
                                                let input_id = ui.id().with("editor_console_input");

                                                // Consume Tab/Enter BEFORE widget (prevents focus-steal)
                                                let tab_consumed = ui
                                                    .memory(|mem| mem.has_focus(input_id))
                                                    && ui.input_mut(|i| {
                                                        i.consume_key(
                                                            egui::Modifiers::NONE,
                                                            egui::Key::Tab,
                                                        )
                                                    });
                                                let enter_consumed = ui
                                                    .memory(|mem| mem.has_focus(input_id))
                                                    && ui.input_mut(|i| {
                                                        i.consume_key(
                                                            egui::Modifiers::NONE,
                                                            egui::Key::Enter,
                                                        )
                                                    });

                                                // Consume Left/Right for suggestion navigation (only when suggestions exist)
                                                let left_consumed = has_suggestions
                                                    && ui.memory(|mem| mem.has_focus(input_id))
                                                    && ui.input_mut(|i| {
                                                        i.consume_key(
                                                            egui::Modifiers::NONE,
                                                            egui::Key::ArrowLeft,
                                                        )
                                                    });
                                                let right_consumed = has_suggestions
                                                    && ui.memory(|mem| mem.has_focus(input_id))
                                                    && ui.input_mut(|i| {
                                                        i.consume_key(
                                                            egui::Modifiers::NONE,
                                                            egui::Key::ArrowRight,
                                                        )
                                                    });

                                                let resp = ui.add_sized(
                                                    egui::vec2(ui.available_width(), input_h),
                                                    egui::TextEdit::singleline(
                                                        &mut self.console.input_buffer,
                                                    )
                                                    .hint_text("Type a command...")
                                                    .font(egui::TextStyle::Monospace)
                                                    .desired_width(f32::INFINITY)
                                                    .id(input_id),
                                                );

                                                let mut needs_focus = false;
                                                if tab_consumed {
                                                    self.console.advance_suggestion();
                                                    needs_focus = true;
                                                }
                                                if enter_consumed {
                                                    self.execute_console_input();
                                                }
                                                if left_consumed {
                                                    self.console.select_previous_suggestion();
                                                }
                                                if right_consumed {
                                                    self.console.select_next_suggestion();
                                                }
                                                if needs_focus {
                                                    resp.request_focus();
                                                }

                                                if self.console_focus_requested {
                                                    resp.request_focus();
                                                    self.console_focus_requested = false;
                                                }

                                                // Handle up/down arrows after widget
                                                if resp.has_focus() {
                                                    let egui_ctx = ui.ctx().clone();
                                                    if egui_ctx.input_mut(|i| {
                                                        i.consume_key(
                                                            egui::Modifiers::NONE,
                                                            egui::Key::ArrowUp,
                                                        )
                                                    }) {
                                                        self.console.navigate_history_up();
                                                    }
                                                    if egui_ctx.input_mut(|i| {
                                                        i.consume_key(
                                                            egui::Modifiers::NONE,
                                                            egui::Key::ArrowDown,
                                                        )
                                                    }) {
                                                        self.console.navigate_history_down();
                                                    }
                                                }
                                            },
                                        );
                                    }
                                },
                            );
                        });
                });
        }

        if self.panels.hierarchy {
            egui::Panel::left("hierarchy_panel")
                .resizable(true)
                .default_size(240.0)
                .min_size(180.0)
                .show(ctx, |ui| {
                    ui.heading(self.current_world_title());
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .scroll_source(egui::containers::scroll_area::ScrollSource {
                            drag: false,
                            ..Default::default()
                        })
                        .id_salt("hierarchy_scroll")
                        .show(ui, |ui| {
                            let scroll_start = ui.cursor().min;
                            let mut root_ids = self.world.borrow().root_object_ids();
                            root_ids.sort_unstable();
                            let mut visited = HashSet::new();
                            for object_id in root_ids {
                                self.hierarchy_object_row(ui, object_id, 0, &mut visited);
                            }
                            for object_id in self.world_object_ids() {
                                let has_valid_parent = self
                                    .world
                                    .borrow()
                                    .object(object_id)
                                    .and_then(|object| object.parent())
                                    .is_some_and(|parent_id| {
                                        self.world.borrow().object(parent_id).is_some()
                                    });
                                if !visited.contains(&object_id) && !has_valid_parent {
                                    self.hierarchy_object_row(ui, object_id, 0, &mut visited);
                                }
                            }
                            let blank_height = (ui.clip_rect().bottom() - ui.cursor().min.y)
                                .max(24.0)
                                .max(ui.available_height());
                            let blank_response = ui.allocate_response(
                                egui::vec2(ui.available_width(), blank_height),
                                egui::Sense::click(),
                            );
                            if blank_response.clicked() && ui.cursor().min.y > scroll_start.y {
                                self.clear_selection();
                            }
                            if blank_response.hovered()
                                && ui.input(|input| input.pointer.any_released())
                            {
                                if self.content_browser.is_dragging_asset() {
                                    let path = self.content_browser.take_dragging_asset_path();
                                    if let Some(path) = path {
                                        self.spawn_from_asset_path(&path, None);
                                    }
                                } else if let Some(dragged_id) =
                                    self.hierarchy_dragging_object.take()
                                {
                                    if self.world.borrow_mut().set_parent(dragged_id, None) {
                                        self.status_line =
                                            "Moved object to hierarchy root.".to_string();
                                    }
                                }
                            }
                            blank_response.context_menu(|ui| {
                                self.hierarchy_context_menu_ui(ui, None);
                            });
                        });
                });
        }

        if self.panels.inspector {
            egui::Panel::right("inspector_panel")
                .resizable(true)
                .default_size(320.0)
                .min_size(320.0)
                .show(ctx, |ui| {
                    self.inspector_panel_width = ui.max_rect().width();
                    ui.heading("Inspector");
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .id_salt("inspector_scroll")
                        .show(ui, |ui| {
                            if let Some(object_id) = self.selection {
                                if let Some(object) = self.world.borrow_mut().object_mut(object_id) {
                                    let project_root = self
                                        .project_session
                                        .as_ref()
                                        .map(|session| session.project.root_dir.as_path());
                                    let scripts_dir = self
                                        .project_session
                                        .as_ref()
                                        .map(|session| session.project.scripts_dir())
                                        .map(|path| path.into_boxed_path());
                                    let scripts_dir = scripts_dir.as_deref();
                                    let inspector_actions = inspector_ui(
                                        ui,
                                        object,
                                        project_root,
                                        scripts_dir,
                                        &self.settings,
                                        &mut self.tile_paint,
                                    );
                                    for removal in inspector_actions.removals {
                                        match removal.target {
                                            crate::inspector::InspectorRemovalTarget::RuntimeType {
                                                type_id,
                                                type_name,
                                            } => {
                                                if object.remove_component_type_id(type_id) {
                                                    self.status_line =
                                                        format!("Removed {}.", type_name);
                                                }
                                            }
                                            crate::inspector::InspectorRemovalTarget::SerializedType {
                                                kind,
                                                type_name,
                                            } => {
                                                if let Some(storage) = object
                                                    .get_component_mut::<SerializedTypeStorage>()
                                                {
                                                    if storage.remove(kind, &type_name) {
                                                        self.status_line =
                                                            format!("Removed {}.", type_name);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if let Some(ui_path) = &inspector_actions.open_ui_editor {
                                        if let Some(session) = self.project_session.as_ref() {
                                            if ui_path.is_empty() {
                                                self.ui_editor.open_new(&session.project);
                                            } else {
                                                let full_path = session.project.root_dir.join(ui_path);
                                                self.ui_editor.open_asset(full_path);
                                            }
                                        }
                                    }

                                    ui.separator();
                                } else {
                                    ui.label("Selection is out of bounds.");
                                }
                                self.inspector_actions_ui(ui, object_id);
                            } else {
                                ui.label("Select an object in the hierarchy.");
                            }
                            ui.allocate_space(egui::vec2(ui.available_width(), 8.0));
                        });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.viewport_edit_mode_button(
                    ui,
                    ViewportEditMode::Position,
                    "position-icon",
                    "Position",
                );
                self.viewport_edit_mode_button(
                    ui,
                    ViewportEditMode::Rotation,
                    "rotation-icon",
                    "Rotation",
                );
                self.viewport_edit_mode_button(ui, ViewportEditMode::Scale, "scale-icon", "Scale");

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Gizmo").on_hover_text("Gizmo Settings").clicked() {
                        self.gizmo_settings_open = !self.gizmo_settings_open;
                    }

                    let camera_icon = crate::editor_textures::load_component_icon(
                        ui.ctx(),
                        "viewport_toolbar_camera_icon",
                        "c-Camera",
                    );
                    let camera_button = ui.add(
                        egui::Button::image(
                            egui::Image::new(&camera_icon)
                                .fit_to_exact_size(egui::vec2(18.0, 18.0)),
                        )
                        .frame(true),
                    );
                    if camera_button.clicked() {
                        self.view_settings_open = !self.view_settings_open;
                    }

                    if ui.button("Rendering").clicked() {
                        self.rendering_settings_open = !self.rendering_settings_open;
                    }
                    if ui.button("Frame Selected").clicked() {
                        self.frame_selected_object();
                    }
                });
            });
            ui.separator();

            let desired_size = ui.available_size().max(egui::vec2(64.0, 64.0));
            let pixels_per_point = ctx.pixels_per_point();
            self.pending_viewport_size = (
                (desired_size.x * pixels_per_point).round().max(1.0) as u32,
                (desired_size.y * pixels_per_point).round().max(1.0) as u32,
            );

            let frame = egui::Frame::canvas(ui.style())
                .fill(style::VIEWPORT_BACKGROUND)
                .inner_margin(egui::Margin::same(0));

            frame.show(ui, |ui| {
                if let Some(texture_id) = self.viewport_texture_id {
                    let response = ui.add(
                        egui::Image::new((texture_id, desired_size))
                            .sense(egui::Sense::click_and_drag()),
                    );
                    self.viewport_hovered = response.hovered();
                    self.handle_viewport_interaction(ui, ctx, &response);
                } else {
                    self.viewport_hovered = false;
                    ui.allocate_ui_with_layout(
                        desired_size,
                        Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.label("Viewport is initializing...");
                        },
                    );
                }
            });
        });

        self.viewport_settings_window(ctx);
        self.rendering_settings_window(ctx);
        self.gizmo_settings_window(ctx);
        self.tile_palette_window(ctx);
        self.editor_settings_window(ctx);
        self.project_settings_window(ctx);
        self.build_settings_window(ctx);
        self.project_dialog_window(ctx);
        self.project_loading_overlay(ctx);
        self.project_version_prompt_window(ctx);
        self.ui_editor.window(
            ctx,
            self.project_session
                .as_ref()
                .map(|session| &session.project),
        );
    }

    #[allow(deprecated)]
    fn welcome_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(style::VIEWPORT_BACKGROUND))
            .show(ctx, |ui| {
                let version = env!("CARGO_PKG_VERSION");
                let recent_projects = self.settings.recent_projects.clone();
                egui::Area::new("welcome_center_panel".into())
                    .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ui.ctx(), |ui| {
                        egui::Frame::new()
                            .fill(ui.visuals().panel_fill)
                            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                            .corner_radius(egui::CornerRadius::same(
                                style::spacing::CORNER_RADIUS,
                            ))
                            .inner_margin(egui::Margin::same(28))
                            .show(ui, |ui| {
                                ui.set_width(940.0);
                                ui.set_height(640.0);
                                ui.horizontal_top(|ui| {
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(220.0, ui.available_height()),
                                        Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.heading("Welcome");
                                            ui.add_space(6.0);
                                            ui.label(
                                                "Open an existing project or create a new one.",
                                            );
                                            ui.add_space(14.0);
                                            if ui
                                                .add_sized(
                                                    [ui.available_width(), 34.0],
                                                    egui::Button::new("Open Project..."),
                                                )
                                                .clicked()
                                            {
                                                self.open_project_dialog();
                                            }
                                            ui.add_space(8.0);
                                            if ui
                                                .add_sized(
                                                    [ui.available_width(), 34.0],
                                                    egui::Button::new("Create New Project..."),
                                                )
                                                .clicked()
                                            {
                                                self.project_dialog.open = true;
                                            }
                                            ui.add_space(16.0);
                                            ui.separator();
                                            ui.add_space(10.0);
                                            ui.small(format!("Engine version: {version}"));
                                            if self.project_load.is_some() {
                                                ui.add_space(12.0);
                                                ui.horizontal(|ui| {
                                                    ui.spinner();
                                                    ui.label("Loading project...");
                                                });
                                            }
                                        },
                                    );

                                    ui.add_space(16.0);
                                    ui.separator();
                                    ui.add_space(16.0);

                                    ui.allocate_ui_with_layout(
                                        egui::vec2(ui.available_width(), ui.available_height()),
                                        Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.horizontal(|ui| {
                                                ui.heading("Recent Projects");
                                                ui.label(
                                                    RichText::new(format!("{}", recent_projects.len()))
                                                        .weak(),
                                                );
                                            });
                                            ui.small("Project previews are updated on world save.");
                                            ui.add_space(8.0);

                                            if recent_projects.is_empty() {
                                                ui.label("No recent projects yet.");
                                            } else {
                                                egui::ScrollArea::vertical()
                                                    .auto_shrink([false, false])
                                                    .max_height(ui.available_height())
                                                    .id_salt("welcome_recent_projects")
                                                    .show(ui, |ui| {
                                                        for (index, entry) in recent_projects
                                                            .iter()
                                                            .enumerate()
                                                        {
                                                            let frame = egui::Frame::new()
                                                                .fill(ui.visuals().faint_bg_color)
                                                                .stroke(
                                                                    ui.visuals()
                                                                        .widgets
                                                                        .noninteractive
                                                                        .bg_stroke,
                                                                )
                                                                .corner_radius(
                                                                    egui::CornerRadius::same(10),
                                                                )
                                                                .inner_margin(
                                                                    egui::Margin::same(12),
                                                                );
                                                            frame.show(ui, |ui| {
                                                                ui.set_width(ui.available_width());
                                                                ui.horizontal(|ui| {
                                                                    let preview_path =
                                                                        project::project_preview_path(
                                                                            entry
                                                                                .manifest_path
                                                                                .parent()
                                                                                .unwrap_or(
                                                                                    entry
                                                                                        .manifest_path
                                                                                        .as_path(),
                                                                                ),
                                                                        );
                                                                    if let Ok(texture) =
                                                                        crate::editor_textures::load_texture_from_path(
                                                                            ui.ctx(),
                                                                            &format!(
                                                                                "welcome_recent_preview_{}_{}",
                                                                                index,
                                                                                entry.manifest_path.display()
                                                                            ),
                                                                            &preview_path,
                                                                            Some(64),
                                                                        )
                                                                    {
                                                                        ui.add(
                                                                            egui::Image::new(
                                                                                &texture,
                                                                            )
                                                                            .fit_to_exact_size(
                                                                                egui::vec2(
                                                                                    64.0, 64.0,
                                                                                ),
                                                                            ),
                                                                        );
                                                                    } else {
                                                                        let icon = crate::editor_textures::load_editor_icon(
                                                                            ui.ctx(),
                                                                            &format!(
                                                                                "welcome_recent_world_icon_{}",
                                                                                index
                                                                            ),
                                                                            "world",
                                                                        );
                                                                        ui.add(
                                                                            egui::Image::new(&icon)
                                                                                .fit_to_exact_size(
                                                                                    egui::vec2(
                                                                                        48.0,
                                                                                        48.0,
                                                                                    ),
                                                                                ),
                                                                        );
                                                                    }

                                                                    ui.vertical(|ui| {
                                                                        let display_name = if entry
                                                                            .project_name
                                                                            .trim()
                                                                            .is_empty()
                                                                        {
                                                                            entry
                                                                                .manifest_path
                                                                                .file_stem()
                                                                                .map(|stem| stem
                                                                                    .to_string_lossy()
                                                                                    .to_string())
                                                                                .unwrap_or_else(|| {
                                                                                    "Unknown Project"
                                                                                        .to_string()
                                                                                })
                                                                        } else {
                                                                            entry.project_name.clone()
                                                                        };
                                                                        ui.label(
                                                                            RichText::new(display_name)
                                                                                .strong(),
                                                                        );
                                                                        ui.small(
                                                                            entry.manifest_path.display().to_string(),
                                                                        );
                                                                        ui.horizontal(|ui| {
                                                                            if let Ok(project) =
                                                                                load_project(
                                                                                    &entry.manifest_path,
                                                                                )
                                                                            {
                                                                                let project_version =
                                                                                    project
                                                                                        .manifest
                                                                                        .engine_version;
                                                                                if project_version
                                                                                    != version
                                                                                {
                                                                                    version_warning_badge(ui);
                                                                                }
                                                                                ui.small(format!(
                                                                                    "Version: {}",
                                                                                    project_version
                                                                                ));
                                                                            } else {
                                                                                ui.small(
                                                                                    "Version: unavailable",
                                                                                );
                                                                            }
                                                                        });
                                                                    });

                                                                    ui.with_layout(
                                                                        Layout::right_to_left(
                                                                            egui::Align::Center,
                                                                        ),
                                                                        |ui| {
                                                                            let project_root = entry
                                                                                .manifest_path
                                                                                .parent()
                                                                                .map(PathBuf::from);
                                                                            if ui.button("Open").clicked()
                                                                            {
                                                                                if entry
                                                                                    .manifest_path
                                                                                    .is_file()
                                                                                {
                                                                                    self.begin_load_project(
                                                                                        entry.manifest_path.clone(),
                                                                                    );
                                                                                } else {
                                                                                    self.settings.remove_recent_project(
                                                                                        &entry.manifest_path,
                                                                                    );
                                                                                    let _ = self
                                                                                        .settings
                                                                                        .save();
                                                                                    self.status_line = format!(
                                                                                        "Recent project is missing: {}",
                                                                                        entry.manifest_path.display()
                                                                                    );
                                                                                }
                                                                            }
                                                                            if ui.button("Explorer").clicked() {
                                                                                if let Some(project_root) =
                                                                                    project_root.as_deref()
                                                                                {
                                                                                    self.open_project_in_explorer(
                                                                                        project_root,
                                                                                    );
                                                                                }
                                                                            }
                                                                        },
                                                                    );
                                                                });
                                                            });
                                                            ui.add_space(8.0);
                                                        }
                                                    });
                                            }
                                        },
                                    );
                                });
                            });
                    });
            });
    }

    pub(super) fn render_frame(&mut self) {
        let full_output = self.draw_ui();
        self.ensure_viewport_target();
        self.update_scene_preview();

        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(egui_state) = self.egui_state.as_mut() else {
            return;
        };
        egui_state.handle_platform_output(window, full_output.platform_output);

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let Some(egui_renderer) = self.egui_renderer.as_mut() else {
            return;
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            egui_renderer.update_texture(renderer.device(), renderer.queue(), *id, image_delta);
        }

        let size = window.inner_size();
        let pixels_per_point = window.scale_factor() as f32;
        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, pixels_per_point);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [size.width.max(1), size.height.max(1)],
            pixels_per_point,
        };

        let surface_texture = match renderer.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            _ => return,
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Editor UI Encoder"),
                });

        let mut command_buffers = egui_renderer.update_buffers(
            renderer.device(),
            renderer.queue(),
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Editor UI Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(style::RENDER_CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            let mut render_pass = render_pass.forget_lifetime();
            egui_renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        command_buffers.push(encoder.finish());
        renderer.queue().submit(command_buffers);
        surface_texture.present();

        for id in &full_output.textures_delta.free {
            egui_renderer.free_texture(id);
        }
    }

    fn project_loading_overlay(&mut self, ctx: &egui::Context) {
        if let Some(error) = &self.project_load_error {
            let mut open = true;
            let mut close_clicked = false;
            let error_text = error.clone();
            let log_path = self.global_log_path.clone();
            egui::Window::new("Failed to Open Project")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.colored_label(style::ERROR_COLOR, &error_text);
                    ui.add_space(8.0);
                    ui.label(RichText::new("Check the editor log for details:").size(12.0));
                    ui.add_space(4.0);
                    if ui.button("Open Log Folder").clicked() {
                        let _ = std::process::Command::new("explorer")
                            .arg("/select,")
                            .arg(&log_path)
                            .spawn();
                    }
                    if ui.button("Close").clicked() {
                        close_clicked = true;
                    }
                });
            if !open || close_clicked {
                self.project_load_error = None;
            }
            return;
        }

        if self.project_load.is_none() {
            return;
        }

        egui::Window::new("Loading Project")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Project is loading...");
                });
                ui.add_space(8.0);

                // Show recent console messages (up to 5) for progress feedback
                let all_msgs: Vec<&str> = self.console.messages().collect();
                let start = all_msgs.len().saturating_sub(5);
                for line in all_msgs.iter().skip(start) {
                    ui.small(*line);
                }

                ui.add_space(6.0);
                ui.label("The first build compiles all dependencies and may take several minutes.");
            });
    }

    fn project_version_prompt_window(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.project_version_prompt.as_ref() else {
            return;
        };

        let project_name = prompt.project_name.clone();
        let project_root = prompt.project_root.clone();
        let project_version = prompt.project_version.clone();
        let editor_version = prompt.editor_version.clone();
        let mut open = true;
        let mut cancel_open = false;
        let mut create_backup = false;
        let mut open_project = false;

        egui::Window::new("Project Version Mismatch")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    version_warning_badge(ui);
                    ui.label(RichText::new("Project version differs from editor version").strong());
                });
                ui.add_space(8.0);
                ui.label(format!("Project: {project_name}"));
                ui.label(format!("Project version: {project_version}"));
                ui.label(format!("Editor version: {editor_version}"));
                ui.small(project_root.display().to_string());
                ui.add_space(12.0);
                ui.label(
                    "This project was created or last saved with a different engine/editor version. Create a backup before opening if you want a safe rollback point.",
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Create Backup").clicked() {
                        create_backup = true;
                    }
                    if ui.button("Open Project").clicked() {
                        open_project = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel_open = true;
                    }
                });
            });

        if create_backup {
            match self.create_project_backup(&project_root) {
                Ok(path) => {
                    self.status_line = format!("Created backup at {}.", path.display());
                }
                Err(error) => {
                    self.status_line = format!("Failed to create backup: {error}");
                }
            }
        }

        if open_project {
            if let Some(prompt) = self.project_version_prompt.take() {
                self.apply_loaded_project(prompt.pending_result);
            }
            return;
        }

        if cancel_open || !open {
            self.project_version_prompt = None;
            self.status_line = "Project opening cancelled.".to_string();
        }
    }

    fn current_world_title(&self) -> String {
        self.current_world_path()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "Unsaved world".to_string())
    }

    fn viewport_edit_mode_button(
        &mut self,
        ui: &mut egui::Ui,
        mode: ViewportEditMode,
        icon_name: &str,
        tooltip: &str,
    ) {
        let icon = crate::editor_textures::load_editor_icon(
            ui.ctx(),
            &format!("viewport_{icon_name}"),
            icon_name,
        );
        let mut button =
            egui::Button::image(egui::Image::new(&icon).fit_to_exact_size(egui::vec2(18.0, 18.0)))
                .frame(true)
                .min_size(egui::vec2(28.0, 28.0));
        if self.viewport_edit_mode == mode {
            button = button.fill(ui.visuals().selection.bg_fill);
        }
        if ui.add(button).on_hover_text(tooltip).clicked() {
            self.viewport_edit_mode = mode;
            self.gizmo_drag = None;
        }
    }

    fn hierarchy_context_menu_ui(&mut self, ui: &mut egui::Ui, target_id: Option<ObjectId>) {
        if ui.button("Create Empty Object").clicked() {
            if let Some(parent_id) = target_id {
                self.create_empty_child_object(parent_id);
            } else {
                self.create_empty_object();
            }
            ui.close();
            return;
        }
        self.create_from_archetype_menu_ui(ui);
        ui.separator();

        if let Some(object_id) = target_id {
            if ui.button("Rename").clicked() {
                self.begin_hierarchy_rename(object_id);
                ui.close();
            }
            ui.separator();
            if ui.button("Copy").clicked() {
                self.copy_object(object_id, false);
                ui.close();
            }
            if ui.button("Cut").clicked() {
                self.copy_object(object_id, true);
                ui.close();
            }
            if ui
                .add_enabled(
                    self.hierarchy_clipboard.is_some(),
                    egui::Button::new("Paste"),
                )
                .clicked()
            {
                self.paste_object(target_id);
                ui.close();
            }
            if ui.button("Delete").clicked() {
                self.delete_object(object_id);
                ui.close();
            }
            ui.separator();
        } else if ui
            .add_enabled(
                self.hierarchy_clipboard.is_some(),
                egui::Button::new("Paste"),
            )
            .clicked()
        {
            self.paste_object(None);
            ui.close();
            return;
        }
    }

    fn hierarchy_object_row(
        &mut self,
        ui: &mut egui::Ui,
        object_id: ObjectId,
        depth: usize,
        visited: &mut HashSet<ObjectId>,
    ) {
        if !visited.insert(object_id) {
            return;
        }
        let (title, children, has_children) = {
            let world_ref = self.world.borrow();

            let Some(object) = world_ref.object(object_id) else {
                return;
            };

            let title = helpers::object_title(object);
            let mut children = object.children().to_vec();
            children.sort_unstable();
            let has_children = !children.is_empty();

            (title, children, has_children)
        };

        let mut expanded = self.hierarchy_expanded.contains(&object_id);
        let mut child_state = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            ui.make_persistent_id(("hierarchy_children", object_id)),
            expanded,
        );
        child_state.set_open(expanded);

        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            if has_children {
                let arrow = if child_state.openness(ui.ctx()) > 0.5 {
                    "v"
                } else {
                    ">"
                };
                let arrow_response = ui.add_sized(
                    egui::vec2(16.0, 18.0),
                    egui::Button::new(arrow).frame(false),
                );
                if arrow_response.clicked() {
                    if expanded {
                        self.hierarchy_expanded.remove(&object_id);
                    } else {
                        self.hierarchy_expanded.insert(object_id);
                    }
                    expanded = !expanded;
                    child_state.set_open(expanded);
                }
            } else {
                ui.add_space(16.0);
            }

            let selected = self.selected_objects.contains(&object_id);
            let response = if self.hierarchy_renaming == Some(object_id) {
                let response = ui.add_sized(
                    egui::vec2(ui.available_width().max(80.0), 22.0),
                    egui::TextEdit::singleline(&mut self.hierarchy_rename_buffer)
                        .desired_width(f32::INFINITY),
                );
                response.request_focus();
                let commit =
                    response.lost_focus() || ui.input(|input| input.key_pressed(egui::Key::Enter));
                let cancel = ui.input(|input| input.key_pressed(egui::Key::Escape));
                if commit {
                    self.commit_hierarchy_rename(object_id);
                } else if cancel {
                    self.hierarchy_renaming = None;
                }
                response
            } else {
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width().max(80.0), 22.0),
                    egui::Sense::click_and_drag(),
                );
                let pointer_hovered = ui
                    .input(|input| input.pointer.hover_pos())
                    .is_some_and(|position| rect.contains(position));
                let is_drag_target = self.hierarchy_dragging_object.is_some()
                    && self.hierarchy_dragging_object != Some(object_id)
                    && pointer_hovered;
                let bg_fill = if is_drag_target || (!selected && pointer_hovered) {
                    style::HOVER_BACKGROUND
                } else if selected {
                    style::SELECTION_BACKGROUND
                } else {
                    egui::Color32::TRANSPARENT
                };
                if bg_fill != egui::Color32::TRANSPARENT {
                    ui.painter().rect_filled(rect, 3.0, bg_fill);
                }
                ui.painter().text(
                    rect.left_center() + egui::vec2(4.0, 0.0),
                    Align2::LEFT_CENTER,
                    title,
                    egui::TextStyle::Button.resolve(ui.style()),
                    ui.visuals().text_color(),
                );
                response
            };
            if response.clicked() {
                let additive = ui.input(|input| input.modifiers.shift);
                self.select_object(object_id, additive);
            }
            if response.double_clicked() {
                self.begin_hierarchy_rename(object_id);
            }
            if response.drag_started() {
                self.hierarchy_dragging_object = Some(object_id);
            }
            if response.hovered() && ui.input(|input| input.pointer.any_released()) {
                if self.content_browser.is_dragging_asset() {
                    let path = self.content_browser.take_dragging_asset_path();
                    if let Some(path) = path {
                        self.spawn_from_asset_path(&path, Some(object_id));
                    }
                } else if let Some(dragged_id) = self.hierarchy_dragging_object.take() {
                    if dragged_id != object_id
                        && self
                            .world
                            .borrow_mut()
                            .set_parent(dragged_id, Some(object_id))
                    {
                        self.set_primary_selection(Some(dragged_id));
                        self.status_line = "Reparented object.".to_string();
                    }
                }
            }
            response.context_menu(|ui| {
                self.select_object(object_id, false);
                self.hierarchy_context_menu_ui(ui, Some(object_id));
            });
        });

        if has_children {
            child_state.show_body_unindented(ui, |ui| {
                for child_id in children {
                    self.hierarchy_object_row(ui, child_id, depth + 1, visited);
                }
            });
        } else if expanded {
            for child_id in children {
                self.hierarchy_object_row(ui, child_id, depth + 1, visited);
            }
        }
    }

    fn begin_hierarchy_rename(&mut self, object_id: ObjectId) {
        let world = self.world.borrow();
        let Some(object) = world.object(object_id) else {
            return;
        };
        self.hierarchy_renaming = Some(object_id);
        self.hierarchy_rename_buffer = helpers::object_title(object);
    }

    fn commit_hierarchy_rename(&mut self, object_id: ObjectId) {
        let name = self.hierarchy_rename_buffer.trim();
        if let Some(object) = self.world.borrow_mut().object_mut(object_id) {
            object.name = if name.is_empty() {
                "Object".to_string()
            } else {
                name.to_string()
            };
        }
        self.hierarchy_renaming = None;
        self.status_line = "Renamed object.".to_string();
    }

    fn inspector_actions_ui(&mut self, ui: &mut egui::Ui, object_id: ObjectId) {
        ui.horizontal_wrapped(|ui| {
            self.add_registered_type_menu_ui(ui, object_id, RegisteredTypeKind::Component);
            self.add_registered_type_menu_ui(ui, object_id, RegisteredTypeKind::Script);
        });
    }

    fn add_registered_type_menu_ui(
        &mut self,
        ui: &mut egui::Ui,
        object_id: ObjectId,
        kind: RegisteredTypeKind,
    ) {
        let label = match kind {
            RegisteredTypeKind::Component => "Add Component",
            RegisteredTypeKind::Script => "Add Script",
        };

        // =========================================================
        // Gather all world/object data BEFORE building UI.
        // This prevents RefCell borrow conflicts inside egui closures.
        // =========================================================

        let (object_component_type_ids, serialized_entries) = {
            let world = self.world.borrow();

            let Some(object) = world.object(object_id) else {
                ui.menu_button(label, |ui| {
                    ui.label("Object not found.");
                });
                return;
            };

            let object_component_type_ids: std::collections::HashSet<TypeId> =
                object.component_type_ids().into_iter().collect();

            let serialized_entries = object
                .get_component::<SerializedTypeStorage>()
                .map(|storage| storage.entries.clone())
                .unwrap_or_default();

            (object_component_type_ids, serialized_entries)
        };

        // =========================================================
        // Build addable type list OUTSIDE egui closures
        // =========================================================

        let runtime_registry = self.runtime_registry();

        let mut registered_types: Vec<(
            Option<TypeId>,
            String,
            bool,
            Option<ProjectRegisteredTypeRecord>,
        )> = runtime_registry
            .types()
            .registered_types()
            .into_iter()
            .filter(|metadata| metadata.kind() == kind)
            .filter(|metadata| metadata.type_id() != TypeId::of::<Tilemap>())
            .filter(|metadata| !object_component_type_ids.contains(&metadata.type_id()))
            .map(|metadata| {
                (
                    Some(metadata.type_id()),
                    helpers::short_type_name(metadata.type_name()).to_string(),
                    runtime_registry
                        .types()
                        .has_object_factory(metadata.type_id()),
                    None,
                )
            })
            .collect();

        let project_kind = match kind {
            RegisteredTypeKind::Component => ProjectRegisteredTypeKind::Component,
            RegisteredTypeKind::Script => ProjectRegisteredTypeKind::Script,
        };

        for metadata in self
            .place_object
            .registered_types
            .iter()
            .filter(|metadata| metadata.kind == project_kind)
            .filter(|metadata| metadata.source == runa_project::ProjectRegistrationSource::User)
        {
            let serialized_kind = match metadata.kind {
                ProjectRegisteredTypeKind::Component => SerializedTypeKind::Component,
                ProjectRegisteredTypeKind::Script => SerializedTypeKind::Script,
            };

            let already_attached_as_stub = serialized_entries.iter().any(|entry| {
                entry.kind == serialized_kind && entry.type_name == metadata.type_name
            });

            if already_attached_as_stub {
                continue;
            }

            let short_name = helpers::short_type_name(&metadata.type_name).to_string();

            let already_listed = registered_types
                .iter()
                .any(|(_, existing_name, _, _)| existing_name == &short_name);

            if !already_listed {
                registered_types.push((None, short_name, true, Some(metadata.clone())));
            }
        }

        registered_types.sort_by(|left, right| left.1.cmp(&right.1));

        // =========================================================
        // UI
        // =========================================================

        ui.menu_button(label, |ui| {
            if registered_types.is_empty() {
                ui.label("No registered types available.");
                return;
            }

            let mut addable_count = 0usize;

            for (
                type_id,
                name,
                is_addable,
                project_metadata,
            ) in &registered_types
            {
                let icon_name = type_id
                    .map(|type_id| {
                        helpers::component_icon_name(
                            type_id,
                            registered_kind_to_runtime_kind(kind),
                        )
                    })
                    .unwrap_or(match kind {
                        RegisteredTypeKind::Component => "c-Object",
                        RegisteredTypeKind::Script => "c-Script",
                    });

                let icon =
                    crate::editor_textures::load_component_icon(
                        ui.ctx(),
                        &format!("add_type_icon_{icon_name}"),
                        icon_name,
                    );

                let docs_url = type_id
                    .and_then(crate::inspector::component_docs_url)
                    .unwrap_or(
                        "https://github.com/RunaLight/runa/blob/main/docs/tutorials/README.md",
                    );

                if !*is_addable {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Image::new(&icon)
                                .fit_to_exact_size(egui::vec2(16.0, 16.0)),
                        );

                        ui.add_enabled(
                            false,
                            egui::Button::new(format!(
                                "{name} (TODO: no runtime factory)"
                            )),
                        );

                        ui.with_layout(
                            Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                help_icon_button(
                                    ui,
                                    docs_url,
                                    &mut self.status_line,
                                );
                            },
                        );
                    });

                    continue;
                }

                // =====================================================
                // Project serialized type
                // =====================================================

                if let Some(project_metadata) = project_metadata {
                    addable_count += 1;

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Image::new(&icon)
                                .fit_to_exact_size(egui::vec2(16.0, 16.0)),
                        );

                        if ui.button(name).clicked() {
                            let success = self
                                .add_project_serialized_type_to_object(
                                    object_id,
                                    project_metadata,
                                );

                            if success {
                                self.status_line =
                                    format!("Added {name}.");
                            } else {
                                self.status_line =
                                    format!("Failed to add {name}.");
                            }

                            ui.close();
                        }

                        ui.with_layout(
                            Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                help_icon_button(
                                    ui,
                                    docs_url,
                                    &mut self.status_line,
                                );
                            },
                        );
                    });

                    continue;
                }

                // =====================================================
                // Runtime registered type
                // =====================================================

                let Some(type_id) = *type_id else {
                    continue;
                };

                addable_count += 1;

                ui.horizontal(|ui| {
                    ui.add(
                        egui::Image::new(&icon)
                            .fit_to_exact_size(egui::vec2(16.0, 16.0)),
                    );

                    if ui.button(name).clicked() {
                        let success = self
                            .add_registered_type_to_object(
                                object_id,
                                type_id,
                            );

                        if success {
                            self.status_line =
                                format!("Added {name}.");
                        } else {
                            self.status_line = format!(
                                "Failed to add {name}: runtime registry did not provide a usable factory."
                            );
                        }

                        ui.close();
                    }

                    ui.with_layout(
                        Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            help_icon_button(
                                ui,
                                docs_url,
                                &mut self.status_line,
                            );
                        },
                    );
                });
            }

            if addable_count == 0 {
                ui.separator();
                ui.label("No editor-addable registered types.");
            }
        });
    }

    fn editor_settings_window(&mut self, ctx: &egui::Context) {
        if !self.editor_settings_open {
            return;
        }

        let mut open = self.editor_settings_open;
        egui::Window::new("Editor Settings")
            .open(&mut open)
            .resizable(true)
            .anchor(Align2::CENTER_CENTER, [0., 0.])
            .movable(true)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.heading("Code Editor");
                property_row_like(ui, "Executable", |ui| {
                    ui.text_edit_singleline(&mut self.settings.external_editor_executable);
                });
                ui.label("File Arguments");
                ui.small("Use one argument per line. Use {file} as placeholder.");
                ui.add(
                    egui::TextEdit::multiline(&mut self.settings.external_editor_args)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);
                ui.label("Project Arguments");
                ui.small("Use one argument per line. Use {project} as placeholder.");
                ui.add(
                    egui::TextEdit::multiline(&mut self.settings.external_editor_project_args)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );
                property_row_like(ui, "Presets", |ui| {
                    if ui.button("Zed").clicked() {
                        self.settings.external_editor_executable = "zed".to_string();
                        self.settings.external_editor_args = "{file}".to_string();
                        self.settings.external_editor_project_args = "{project}".to_string();
                    }
                    if ui.button("VS Code").clicked() {
                        self.settings.external_editor_executable = "code".to_string();
                        self.settings.external_editor_args = "--goto\n{file}".to_string();
                        self.settings.external_editor_project_args = "{project}".to_string();
                    }
                });

                ui.separator();
                ui.heading("Interface");
                const UI_SCALE_OPTIONS: &[(f32, &str)] = &[
                    (0.75, "75%"),
                    (0.90, "90%"),
                    (1.00, "100%"),
                    (1.10, "110%"),
                    (1.15, "115%"),
                    (1.25, "125%"),
                    (1.50, "150%"),
                    (1.75, "175%"),
                    (2.00, "200%"),
                ];
                property_row_like(ui, "UI Scale", |ui| {
                    egui::ComboBox::from_id_salt("editor_ui_scale")
                        .selected_text(
                            UI_SCALE_OPTIONS
                                .iter()
                                .find(|(value, _)| (*value - self.settings.ui_scale).abs() < 0.001)
                                .map(|(_, label)| *label)
                                .unwrap_or("100%"),
                        )
                        .show_ui(ui, |ui| {
                            for (value, label) in UI_SCALE_OPTIONS {
                                if ui
                                    .selectable_label(
                                        (self.settings.ui_scale - *value).abs() < 0.001,
                                        *label,
                                    )
                                    .clicked()
                                {
                                    self.settings.ui_scale = *value;
                                    ctx.set_zoom_factor(*value);
                                    style::apply_editor_style(ctx);
                                    ctx.request_repaint();
                                    ui.close();
                                }
                            }
                        });
                });
                property_row_like(ui, "Icon Size", |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.settings.content_icon_size, 32.0..=96.0)
                            .step_by(1.0),
                    );
                });
                property_row_like(ui, "Hidden Files", |ui| {
                    if ui
                        .checkbox(&mut self.settings.show_hidden_files, "Show")
                        .changed()
                    {
                        self.content_browser.refresh(&self.settings);
                    }
                });

                ui.separator();
                if ui.button("Save Settings").clicked() {
                    match self.settings.save() {
                        Ok(()) => {
                            self.content_browser.refresh(&self.settings);
                            self.status_line = "Editor settings saved.".to_string();
                        }
                        Err(error) => {
                            self.status_line = format!("Failed to save settings: {error}");
                        }
                    }
                }
            });
        self.editor_settings_open = open;
    }

    fn project_settings_window(&mut self, ctx: &egui::Context) {
        if !self.project_settings_open {
            return;
        }

        let Some(session) = self.project_session.as_mut() else {
            self.project_settings_open = false;
            return;
        };

        let window_size = egui::vec2(540., 400.);

        let mut open = self.project_settings_open;
        let mut open_in_code_editor = false;
        egui::Window::new("Project Settings")
            .open(&mut open)
            .resizable(true)
            .default_pos(ctx.viewport_rect().center() - window_size * 0.5)
            .default_width(window_size.x)
            .show(ctx, |ui| {
                let project_root = session.project.root_dir.clone();
                ui.heading("Project");
                property_row_like(ui, "Name", |ui| {
                    ui.text_edit_singleline(&mut session.project.manifest.name);
                });
                property_row_like(ui, "Binary", |ui| {
                    ui.text_edit_singleline(&mut session.project.manifest.binary_name);
                });
                property_row_like(ui, "Startup World", |ui| {
                    ui.label(session.project.manifest.startup_world.as_str());
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = FileDialog::new()
                            .set_directory(session.project.worlds_dir())
                            .add_filter("Runa World", &["ron"])
                            .pick_file()
                        {
                            session.project.manifest.startup_world =
                                project::path_relative_to_project(&project_root, &path);
                        }
                    }
                });
                property_row_like(ui, "Assets Dir", |ui| {
                    ui.label(session.project.manifest.assets_dir.as_str());
                    if ui.button("Browse...").clicked() {
                        if let Some(path) =
                            FileDialog::new().set_directory(&project_root).pick_folder()
                        {
                            session.project.manifest.assets_dir =
                                project::path_relative_to_project(&project_root, &path);
                        }
                    }
                });
                property_row_like(ui, "Worlds Dir", |ui| {
                    ui.label(session.project.manifest.worlds_dir.as_str());
                    if ui.button("Browse...").clicked() {
                        if let Some(path) =
                            FileDialog::new().set_directory(&project_root).pick_folder()
                        {
                            session.project.manifest.worlds_dir =
                                project::path_relative_to_project(&project_root, &path);
                        }
                    }
                });
                property_row_like(ui, "Scripts Dir", |ui| {
                    ui.label(session.project.manifest.scripts_dir.as_str());
                    if ui.button("Browse...").clicked() {
                        if let Some(path) =
                            FileDialog::new().set_directory(&project_root).pick_folder()
                        {
                            session.project.manifest.scripts_dir =
                                project::path_relative_to_project(&project_root, &path);
                        }
                    }
                });

                ui.separator();
                ui.heading("RunaAppConfig");
                if ui.button("Open Project In Code Editor").clicked() {
                    open_in_code_editor = true;
                }
                ui.add_space(8.0);
                property_row_like(ui, "Window Title", |ui| {
                    ui.text_edit_singleline(&mut session.project.manifest.app.window_title);
                });
                property_row_like(ui, "Window Size", |ui| {
                    ui.add_sized(
                        [90.0, 22.0],
                        egui::DragValue::new(&mut session.project.manifest.app.width)
                            .range(1..=8192),
                    );
                    ui.add_sized(
                        [90.0, 22.0],
                        egui::DragValue::new(&mut session.project.manifest.app.height)
                            .range(1..=8192),
                    );
                });
                property_row_like(ui, "Fullscreen", |ui| {
                    ui.checkbox(&mut session.project.manifest.app.fullscreen, "");
                });
                property_row_like(ui, "VSync", |ui| {
                    ui.checkbox(&mut session.project.manifest.app.vsync, "");
                });
                property_row_like(ui, "Show FPS", |ui| {
                    ui.checkbox(&mut session.project.manifest.app.show_fps_in_title, "");
                });
                property_row_like(ui, "Window Icon", |ui| {
                    let icon = session
                        .project
                        .manifest
                        .app
                        .window_icon
                        .clone()
                        .unwrap_or_default();
                    if icon.is_empty() {
                        ui.label("None");
                    } else {
                        ui.label(icon);
                    }
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = FileDialog::new()
                            .set_directory(session.project.assets_dir())
                            .add_filter("Images", &["png", "jpg", "jpeg", "svg", "ico"])
                            .pick_file()
                        {
                            session.project.manifest.app.window_icon =
                                Some(project::path_relative_to_project(&project_root, &path));
                        }
                    }
                    if ui.button("Clear").clicked() {
                        session.project.manifest.app.window_icon = None;
                    }
                });

                ui.separator();
                if ui.button("Save Project Settings").clicked() {
                    match session.project.save_manifest() {
                        Ok(()) => {
                            self.status_line = "Project settings saved.".to_string();
                        }
                        Err(error) => {
                            self.status_line = format!("Failed to save project settings: {error}");
                        }
                    }
                }
            });
        self.project_settings_open = open;
        if open_in_code_editor {
            self.open_project_in_code_editor();
        }
    }

    fn build_settings_window(&mut self, ctx: &egui::Context) {
        if !self.build_settings_open {
            return;
        }

        let Some(session) = self.project_session.as_mut() else {
            self.build_settings_open = false;
            return;
        };

        let mut open = self.build_settings_open;
        let mut save_clicked = false;
        let mut build_clicked = false;
        egui::Window::new("Build Settings")
            .open(&mut open)
            .resizable(true)
            .anchor(Align2::CENTER_CENTER, [0., 0.])
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.heading("Build");
                property_row_like(ui, "Profile", |ui| {
                    ui.selectable_value(&mut session.project.manifest.build.release, true, "Release");
                    ui.selectable_value(&mut session.project.manifest.build.release, false, "Debug");
                });
                property_row_like(ui, "Output Dir", |ui| {
                    ui.text_edit_singleline(&mut session.project.manifest.build.output_dir);
                });
                property_row_like(ui, "Hide Console", |ui| {
                    ui.checkbox(
                        &mut session.project.manifest.build.hide_console_window_on_windows,
                        "Windows release build",
                    );
                });
                ui.small("Built executable will be copied from cargo target output into the configured output directory.");

                ui.separator();
                if ui.button("Save Build Settings").clicked() {
                    save_clicked = true;
                }
                let build_enabled = self.build_process.is_none();
                let build_label = if self.build_process.is_some() {
                    "Build Running..."
                } else {
                    "Build Game"
                };
                if ui
                    .add_enabled(build_enabled, egui::Button::new(build_label))
                    .clicked()
                {
                    build_clicked = true;
                }
            });
        self.build_settings_open = open;

        if save_clicked {
            let result = ensure_release_windows_subsystem(
                &session.project.root_dir,
                session
                    .project
                    .manifest
                    .build
                    .hide_console_window_on_windows,
            )
            .and_then(|_| session.project.save_manifest());
            match result {
                Ok(()) => {
                    self.status_line = "Build settings saved.".to_string();
                }
                Err(error) => {
                    self.status_line = format!("Failed to save build settings: {error}");
                }
            }
        }
        if build_clicked {
            self.build_project();
        }
    }

    fn project_dialog_window(&mut self, ctx: &egui::Context) {
        if !self.project_dialog.open && self.project_dialog.error.is_none() {
            return;
        }

        let mut open = self.project_dialog.open || self.project_dialog.error.is_some();
        let mut is_dismissed = false;
        let dialog_error = self.project_dialog.error.clone();
        let log_path = self.global_log_path.clone();
        egui::Window::new("New Project")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                if let Some(error) = &dialog_error {
                    ui.colored_label(style::ERROR_COLOR, error);
                    ui.add_space(8.0);
                    ui.label(RichText::new("Check the editor log for details:").size(12.0));
                    ui.horizontal(|ui| {
                        if ui.button("Open Log Folder").clicked() {
                            let _ = std::process::Command::new("explorer")
                                .arg("/select,")
                                .arg(&log_path)
                                .spawn();
                        }
                        if ui.button("Close").clicked() {
                            is_dismissed = true;
                        }
                    });
                    return;
                }

                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.project_dialog.name);
                });
                ui.horizontal(|ui| {
                    ui.label("Location");
                    ui.text_edit_singleline(&mut self.project_dialog.location);
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = FileDialog::new().pick_folder() {
                            self.project_dialog.location = path.to_string_lossy().to_string();
                        }
                    }
                });

                ui.separator();
                if ui.button("Create Project").clicked() {
                    self.create_project_from_dialog();
                }
            });
        if !open || is_dismissed {
            self.project_dialog.error = None;
            self.project_dialog.open = false;
        }
    }

    fn viewport_settings_window(&mut self, ctx: &egui::Context) {
        if !self.view_settings_open {
            return;
        }

        let mut open = self.view_settings_open;
        egui::Window::new("View Settings")
            .open(&mut open)
            .default_width(360.0)
            .resizable(false)
            .anchor(Align2::RIGHT_TOP, [-16.0, 64.0])
            .show(ctx, |ui| {
                property_row_like(ui, "Projection", |ui| {
                    let mut projection = self.editor_camera.projection();
                    ui.selectable_value(
                        &mut projection,
                        runa_core::components::ProjectionType::Orthographic,
                        "Orthographic",
                    );
                    ui.selectable_value(
                        &mut projection,
                        runa_core::components::ProjectionType::Perspective,
                        "Perspective",
                    );
                    if projection != self.editor_camera.projection() {
                        self.editor_camera.set_projection(projection);
                    }
                });

                if self.editor_camera.is_orthographic() {
                    property_row_like(ui, "Zoom", |ui| {
                        let mut zoom = self.editor_camera.get_zoom();
                        if ui
                            .add_sized(
                                [120.0, 22.0],
                                egui::DragValue::new(&mut zoom)
                                    .speed(0.25)
                                    .range(1.0..=500.0),
                            )
                            .changed()
                        {
                            self.editor_camera.set_zoom(zoom);
                        }
                    });
                } else {
                    property_row_like(ui, "FOV", |ui| {
                        let mut fov = self.editor_camera.get_fov();
                        if ui
                            .add_sized(
                                [120.0, 22.0],
                                egui::DragValue::new(&mut fov)
                                    .speed(1.0)
                                    .range(20.0..=130.0),
                            )
                            .changed()
                        {
                            self.editor_camera.set_fov(fov);
                        }
                    });
                    property_row_like(ui, "Near", |ui| {
                        let mut near = self.editor_camera.near();
                        if ui
                            .add_sized(
                                [120.0, 22.0],
                                egui::DragValue::new(&mut near)
                                    .speed(0.01)
                                    .range(0.001..=999.0),
                            )
                            .changed()
                        {
                            self.editor_camera.set_near(near);
                        }
                    });
                    property_row_like(ui, "Far", |ui| {
                        let mut far = self.editor_camera.far();
                        if ui
                            .add_sized(
                                [120.0, 22.0],
                                egui::DragValue::new(&mut far)
                                    .speed(1.0)
                                    .range(1.0..=10000.0),
                            )
                            .changed()
                        {
                            self.editor_camera.set_far(far);
                        }
                    });
                    property_row_like(ui, "Sensitivity", |ui| {
                        let mut sensitivity = self.editor_camera.get_sensitivity();
                        if ui
                            .add_sized(
                                [120.0, 22.0],
                                egui::DragValue::new(&mut sensitivity)
                                    .speed(0.01)
                                    .range(0.01..=999.0),
                            )
                            .changed()
                        {
                            self.editor_camera.set_sensitivity(sensitivity);
                        }
                    });
                    property_row_like(ui, "Speed", |ui| {
                        let mut speed = self.editor_camera.get_speed();
                        if ui
                            .add_sized(
                                [120.0, 22.0],
                                egui::DragValue::new(&mut speed)
                                    .speed(0.05)
                                    .range(0.01..=999.0),
                            )
                            .changed()
                        {
                            self.editor_camera.set_speed(speed);
                        }
                    });
                    ui.label("Right mouse: look. WASD move, Space/Ctrl vertical, Shift boost.");
                }
            });
        self.view_settings_open = open;
    }

    fn rendering_settings_window(&mut self, ctx: &egui::Context) {
        if !self.rendering_settings_open {
            return;
        }

        let mut open = self.rendering_settings_open;
        egui::Window::new("Rendering Settings")
            .open(&mut open)
            .default_width(360.0)
            .resizable(true)
            .anchor(Align2::RIGHT_TOP, [-16.0, 64.0])
            .show(ctx, |ui| {
                ui.label("World Atmosphere");
                ui.separator();

                let mut world = self.world.borrow_mut();
                let atmosphere = world.atmosphere_mut();
                color_vec3_row(ui, "Ambient Color", &mut atmosphere.ambient_color);
                property_row_like(ui, "Ambient Power", |ui| {
                    ui.add_sized(
                        [120.0, 22.0],
                        egui::DragValue::new(&mut atmosphere.ambient_intensity)
                            .speed(0.01)
                            .range(0.0..=10.0),
                    );
                });
                property_row_like(ui, "Background Power", |ui| {
                    ui.add_sized(
                        [120.0, 22.0],
                        egui::DragValue::new(&mut atmosphere.background_intensity)
                            .speed(0.01)
                            .range(0.0..=10.0),
                    );
                });

                ui.separator();
                let mut mode = match atmosphere.background {
                    BackgroundMode::SolidColor { .. } => 0,
                    BackgroundMode::VerticalGradient { .. } => 1,
                    BackgroundMode::Sky => 2,
                };

                property_row_like(ui, "Background", |ui| {
                    egui::ComboBox::from_id_salt("world_atmosphere_background_mode")
                        .selected_text(match mode {
                            0 => "Solid Color",
                            1 => "Vertical Gradient",
                            _ => "Sky (reserved)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut mode, 0, "Solid Color");
                            ui.selectable_value(&mut mode, 1, "Vertical Gradient");
                            ui.selectable_value(&mut mode, 2, "Sky (reserved)");
                        });
                });

                atmosphere.background = match (mode, atmosphere.background) {
                    (0, BackgroundMode::SolidColor { color }) => {
                        BackgroundMode::SolidColor { color }
                    }
                    (0, _) => BackgroundMode::SolidColor {
                        color: Vec3::new(0.08, 0.09, 0.11),
                    },
                    (
                        1,
                        BackgroundMode::VerticalGradient {
                            zenith_color,
                            horizon_color,
                            ground_color,
                            horizon_height,
                            smoothness,
                        },
                    ) => BackgroundMode::VerticalGradient {
                        zenith_color,
                        horizon_color,
                        ground_color,
                        horizon_height,
                        smoothness,
                    },
                    (1, _) => BackgroundMode::default(),
                    _ => BackgroundMode::Sky,
                };

                match &mut atmosphere.background {
                    BackgroundMode::SolidColor { ref mut color } => {
                        color_vec3_row(ui, "Color", color);
                    }
                    BackgroundMode::VerticalGradient {
                        ref mut zenith_color,
                        ref mut horizon_color,
                        ref mut ground_color,
                        ref mut horizon_height,
                        ref mut smoothness,
                    } => {
                        color_vec3_row(ui, "Zenith", zenith_color);
                        color_vec3_row(ui, "Horizon", horizon_color);
                        color_vec3_row(ui, "Ground", ground_color);
                        property_row_like(ui, "Horizon Height", |ui| {
                            ui.add_sized(
                                [120.0, 22.0],
                                egui::Slider::new(horizon_height, 0.0..=1.0),
                            );
                        });
                        property_row_like(ui, "Smoothness", |ui| {
                            ui.add_sized([120.0, 22.0], egui::Slider::new(smoothness, 0.001..=1.0));
                        });
                    }
                    BackgroundMode::Sky => {
                        ui.label("Sky rendering is reserved for future skybox/HDRI support.");
                    }
                }
            });
        self.rendering_settings_open = open;
    }

    fn gizmo_settings_window(&mut self, ctx: &egui::Context) {
        if !self.gizmo_settings_open {
            return;
        }

        let mut open = self.gizmo_settings_open;
        egui::Window::new("Gizmo")
            .open(&mut open)
            .default_width(320.0)
            .resizable(false)
            .anchor(Align2::RIGHT_TOP, [-16.0, 64.0])
            .show(ctx, |ui| {
                ui.checkbox(&mut self.gizmo_enabled, "Enabled");
                ui.checkbox(&mut self.show_viewport_grid, "Show Grid");
                ui.checkbox(&mut self.show_component_icons, "Component Icons");
                property_row_like(ui, "Icon Size", |ui| {
                    ui.add_sized(
                        [120.0, 22.0],
                        egui::DragValue::new(&mut self.viewport_component_icon_size)
                            .speed(0.5)
                            .range(8.0..=64.0),
                    );
                });
                ui.checkbox(&mut self.snap_enabled, "Snap");
                property_row_like(ui, "Snap Step", |ui| {
                    ui.add_sized(
                        [120.0, 22.0],
                        egui::DragValue::new(&mut self.snap_step)
                            .speed(0.1)
                            .range(0.1..=100.0),
                    );
                });
                if self.editor_camera.is_perspective() {
                    ui.label("Transform gizmo is currently available in orthographic mode.");
                    ui.label("3D mode keeps grid + framing + component icons.");
                }
            });
        self.gizmo_settings_open = open;
    }

    fn tile_palette_window(&mut self, ctx: &egui::Context) {
        if !self.tile_paint.palette_open {
            return;
        }

        let mut open = self.tile_paint.palette_open;
        egui::Window::new("Tile Palette")
            .open(&mut open)
            .default_width(300.0)
            .default_height(300.0)
            .resizable(true)
            .show(ctx, |ui| {
                let Some(object_id) = self.selection else {
                    ui.label("Select a Tilemap object to choose tiles.");
                    return;
                };
                let mut world = self.world.borrow_mut();
                let Some(object) = world.object_mut(object_id) else {
                    ui.label("Selected object is no longer available.");
                    return;
                };
                let Some(tilemap) = object.get_component_mut::<Tilemap>() else {
                    ui.label("Selected object has no Tilemap data.");
                    return;
                };
                let Some(atlas) = tilemap.atlas.as_ref() else {
                    ui.label("Assign an atlas texture in TilemapRenderer first.");
                    return;
                };

                ui.horizontal(|ui| {
                    ui.label(format!("Mode: {:?}", self.tile_paint.mode));
                    ui.separator();
                    ui.label(format!("Layer: {}", self.tile_paint.layer));
                    ui.separator();
                    ui.label(format!("Selected: {}", tilemap.selected_tile));
                });
                ui.separator();

                let texture_handle = atlas.texture_path.as_ref().and_then(|path| {
                    self.project_session.as_ref().and_then(|session| {
                        let full_path = session.project.root_dir.join(path);
                        crate::editor_textures::load_texture_from_path(
                            ui.ctx(),
                            "tile_palette_atlas_texture",
                            &full_path,
                            None,
                        )
                        .ok()
                    })
                });

                let Some(texture_handle) = texture_handle else {
                    ui.colored_label(
                        style::ERROR_COLOR,
                        "Palette preview needs a project-relative atlas path.",
                    );
                    return;
                };

                let columns = atlas.columns.max(1);
                let rows = atlas.rows.max(1);
                let frame_count = atlas.frame_count();
                let tile_cell_size = egui::vec2(40.0, 40.0);

                egui::ScrollArea::vertical()
                    .id_salt("tile_palette_scroll")
                    .show(ui, |ui| {
                        for row in 0..rows {
                            ui.horizontal(|ui| {
                                for column in 0..columns {
                                    let frame = row * columns + column;
                                    if frame >= frame_count {
                                        continue;
                                    }
                                    let uv = atlas.uv_rect_for_frame(frame);
                                    let selected = tilemap.selected_tile == frame;
                                    let (rect, response) = ui
                                        .allocate_exact_size(tile_cell_size, egui::Sense::click());
                                    let fill = if selected {
                                        egui::Color32::from_rgb(35, 77, 116)
                                    } else if response.hovered() {
                                        ui.visuals().widgets.hovered.bg_fill
                                    } else {
                                        ui.visuals().widgets.inactive.bg_fill
                                    };
                                    let stroke = if selected {
                                        egui::Stroke::new(
                                            2.0,
                                            egui::Color32::from_rgb(96, 180, 255),
                                        )
                                    } else {
                                        ui.visuals().widgets.inactive.bg_stroke
                                    };
                                    ui.painter().rect(
                                        rect,
                                        4.0,
                                        fill,
                                        stroke,
                                        egui::StrokeKind::Inside,
                                    );
                                    let image_rect = rect.shrink(4.0);
                                    ui.painter().image(
                                        texture_handle.id(),
                                        image_rect,
                                        egui::Rect::from_min_size(
                                            egui::pos2(uv.x, uv.y),
                                            egui::vec2(uv.width, uv.height),
                                        ),
                                        egui::Color32::WHITE,
                                    );
                                    if response.on_hover_text(format!("Tile {frame}")).clicked() {
                                        tilemap.selected_tile = frame;
                                    }
                                }
                            });
                        }
                    });
            });
        self.tile_paint.palette_open = open;
    }
}

fn property_row_like(ui: &mut egui::Ui, label: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized([110.0, 22.0], egui::Label::new(label));
        body(ui);
    });
}

fn color_vec3_row(ui: &mut egui::Ui, label: &str, color: &mut Vec3) {
    property_row_like(ui, label, |ui| {
        let mut value = [color.x, color.y, color.z];
        if ui.color_edit_button_rgb(&mut value).changed() {
            *color = Vec3::from_array(value);
        }
        for channel in &mut value {
            *channel = channel.clamp(0.0, 1.0);
        }
    });
}

fn registered_kind_to_runtime_kind(kind: RegisteredTypeKind) -> ComponentRuntimeKind {
    match kind {
        RegisteredTypeKind::Component => ComponentRuntimeKind::Component,
        RegisteredTypeKind::Script => ComponentRuntimeKind::Script,
    }
}

fn help_icon_button(ui: &mut egui::Ui, target: &str, status_line: &mut String) {
    let icon =
        crate::editor_textures::load_editor_icon(ui.ctx(), "add_type_help_icon", "question-icon");
    if ui
        .add(
            egui::Button::image(egui::Image::new(&icon).fit_to_exact_size(egui::vec2(14.0, 14.0)))
                .frame(false),
        )
        .on_hover_text("Open documentation")
        .clicked()
    {
        if let Err(error) = crate::inspector::open_external_target(target) {
            *status_line = error;
        }
    }
}

fn version_warning_badge(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
    let painter = ui.painter();
    let fill = egui::Color32::from_rgb(230, 184, 36);
    painter.circle_filled(rect.center(), 7.0, fill);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "!",
        egui::FontId::proportional(12.0),
        egui::Color32::BLACK,
    );
}
