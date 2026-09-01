use std::sync::Arc;

use crate::command::{
    AtmosphereData, DirectionalLightData, FontId, InstanceData, Mesh3dParams, PointLightData,
    RichTextSegment, ScreenEffectData, TextOutline, TileParams, UiRect,
};
use crate::RenderCommands;
use glam::{Quat, Vec2, Vec3};
use runa_asset::TextureAsset;

#[derive(Default)]
pub struct RenderQueue {
    pub commands: Vec<RenderCommands>,
    pub directional_lights: Vec<DirectionalLightData>,
    pub point_lights: Vec<PointLightData>,
    pub atmosphere: AtmosphereData,
    pub screen_effects: ScreenEffectData,
}

impl RenderQueue {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            directional_lights: Vec::new(),
            point_lights: Vec::new(),
            atmosphere: AtmosphereData::default(),
            screen_effects: ScreenEffectData::default(),
        }
    }

    pub fn set_atmosphere(&mut self, atmosphere: AtmosphereData) {
        self.atmosphere = atmosphere;
    }

    pub fn set_screen_effects(&mut self, effects: ScreenEffectData) {
        self.screen_effects = effects;
    }

    pub fn add_directional_light(&mut self, light: DirectionalLightData) {
        self.directional_lights.push(light);
    }

    pub fn add_point_light(&mut self, light: PointLightData) {
        self.point_lights.push(light);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_sprite(
        &mut self,
        texture: Arc<TextureAsset>,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
        color: [f32; 4],
        uv_rect: [f32; 4],
        order: i32,
        replace_color: bool,
        flip_x: bool,
        flip_y: bool,
    ) {
        self.commands.push(RenderCommands::Sprite {
            texture,
            position,
            rotation,
            scale,
            color,
            uv_rect,
            order,
            replace_color,
            flip_x,
            flip_y,
        });
    }

    pub fn draw_text(
        &mut self,
        text: String,
        position: Vec2,
        color: [f32; 4],
        size: f32,
        outline: Option<TextOutline>,
    ) {
        self.commands.push(RenderCommands::Text {
            text,
            position,
            color,
            size,
            outline,
        });
    }

    pub fn draw_debug_line(&mut self, start: Vec2, end: Vec2, color: [f32; 4], width: f32) {
        self.commands.push(RenderCommands::DebugLine {
            start,
            end,
            color,
            width,
        });
    }

    pub fn draw_tile(&mut self, params: TileParams) {
        self.commands.push(RenderCommands::Tile(params));
    }

    pub fn draw_tiles_batch(
        &mut self,
        texture: Arc<TextureAsset>,
        instances: Vec<InstanceData>,
        order: i32,
    ) {
        if instances.is_empty() {
            return;
        }
        self.commands.push(RenderCommands::TileBatch {
            texture,
            instances,
            order,
            depth: 0.0,
        });
    }

    pub fn draw_mesh_3d(&mut self, params: Mesh3dParams) {
        self.commands.push(RenderCommands::Mesh3D(params));
    }

    // UI
    pub fn draw_ui_rect(
        &mut self,
        rect: UiRect,
        color: [f32; 4],
        z_index: i16,
        order: i32,
        world: bool,
    ) {
        self.commands.push(RenderCommands::UiRect {
            rect,
            color,
            z_index,
            order,
            world,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_ui_image(
        &mut self,
        texture: Arc<TextureAsset>,
        rect: UiRect,
        tint: [f32; 4],
        uv_rect: [f32; 4],
        z_index: i16,
        order: i32,
        world: bool,
    ) {
        self.commands.push(RenderCommands::UiImage {
            texture,
            rect,
            tint,
            uv_rect,
            z_index,
            order,
            world,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_ui_text(
        &mut self,
        text: String,
        rect: UiRect,
        color: [f32; 4],
        font_size: f32,
        z_index: i16,
        order: i32,
        world: bool,
        font_id: Option<FontId>,
        segments: Vec<RichTextSegment>,
    ) {
        self.commands.push(RenderCommands::UiText {
            text,
            rect,
            color,
            font_size,
            z_index,
            order,
            world,
            font_id,
            segments,
        });
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.directional_lights.clear();
        self.point_lights.clear();
        self.atmosphere = AtmosphereData::default();
        self.screen_effects = ScreenEffectData::default();
    }
}
