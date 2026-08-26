use std::sync::Arc;

use glam::{Mat4, Quat, Vec2, Vec3};

use runa_asset::TextureAsset;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

/// Per-instance data for sprite/tile rendering.
/// Contains transform, UV coordinates, and flip information.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    pub position: [f32; 3],
    pub rotation: f32,
    pub scale: [f32; 3],
    pub color: [f32; 4],
    pub uv_offset: [f32; 2],
    pub uv_size: [f32; 2],
    pub flip: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct DirectionalLightData {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct PointLightData {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub radius: f32,
    pub falloff: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum BackgroundModeData {
    SolidColor {
        color: Vec3,
    },
    VerticalGradient {
        zenith_color: Vec3,
        horizon_color: Vec3,
        ground_color: Vec3,
        horizon_height: f32,
        smoothness: f32,
    },
    Sky,
}

#[derive(Clone, Copy, Debug)]
pub struct AtmosphereData {
    pub ambient_color: Vec3,
    pub ambient_intensity: f32,
    pub background_intensity: f32,
    pub background: BackgroundModeData,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScreenEffectData {
    /// 0.0 = fully transparent overlay, 1.0 = fully opaque
    pub fade_color: [f32; 4],
    /// Vignette: 0.0 = no effect, 1.0 = full vignette
    pub vignette_strength: f32,
    pub vignette_radius: f32,
    pub vignette_softness: f32,
    /// Color distortion (RGB shift): offset in UV coordinates
    pub rgb_shift: [f32; 2],
    /// Screen-wide color tint applied as multiply
    pub tint_color: [f32; 4],
    /// Screen-wide brightness/gamma adjustment
    pub brightness: f32,
    pub contrast: f32,
    /// Effect enabled flags
    pub enabled: ScreenEffectFlags,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScreenEffectFlags {
    pub fade: bool,
    pub vignette: bool,
    pub rgb_shift: bool,
    pub tint: bool,
    pub color_adjust: bool,
}

impl ScreenEffectFlags {
    pub fn has_any(&self) -> bool {
        self.fade || self.vignette || self.rgb_shift || self.tint || self.color_adjust
    }

    pub fn to_u32(&self) -> u32 {
        let mut flags = 0u32;
        if self.fade {
            flags |= 1;
        }
        if self.vignette {
            flags |= 2;
        }
        if self.rgb_shift {
            flags |= 4;
        }
        if self.tint {
            flags |= 8;
        }
        if self.color_adjust {
            flags |= 16;
        }
        flags
    }
}

impl Default for AtmosphereData {
    fn default() -> Self {
        Self {
            ambient_color: Vec3::ONE,
            ambient_intensity: 0.15,
            background_intensity: 1.0,
            background: BackgroundModeData::VerticalGradient {
                zenith_color: Vec3::new(0.2, 0.4, 0.8),
                horizon_color: Vec3::new(0.8, 0.9, 1.0),
                ground_color: Vec3::new(0.6, 0.6, 0.7),
                horizon_height: 0.5,
                smoothness: 0.25,
            },
        }
    }
}

pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct Mesh3dParams {
    /// Stable identifier for GPU cache deduplication (Arc::as_ptr).
    pub mesh_id: u64,
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<u32>,
    pub model_matrix: Mat4,
    pub color: [f32; 4],
    pub emission: [f32; 3],
    pub use_vertex_color: bool,
    pub order: i32,
    pub depth: f32,
}

pub struct TileParams {
    pub texture: Arc<TextureAsset>,
    pub position: Vec3,
    pub size: Vec2,
    pub uv_rect: [f32; 4],
    pub flip_x: bool,
    pub flip_y: bool,
    pub color: [f32; 4],
    pub order: i32,
}

#[derive(Clone, Debug)]
pub struct TextOutline {
    pub color: [f32; 4],
    pub width: f32,
}

/// A segment of rich text with its own formatting.
#[derive(Clone, Debug)]
pub struct RichTextSegment {
    pub text: String,
    pub color: [f32; 4],
    pub bold: bool,
}

/// Font identifier for selecting among loaded fonts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontId(pub usize);

impl FontId {
    /// Always the first font loaded (built-in pixel font).
    pub const DEFAULT: FontId = FontId(0);
}

pub enum RenderCommands {
    Sprite {
        texture: std::sync::Arc<TextureAsset>,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
        color: [f32; 4],
        uv_rect: [f32; 4],
        order: i32,
        replace_color: bool,
        flip_x: bool,
        flip_y: bool,
    },
    Text {
        text: String,
        position: Vec2,
        color: [f32; 4],
        size: f32,
        outline: Option<TextOutline>,
    },
    DebugRect {
        position: Vec3,
        size: Vec2,
        color: [f32; 4],
    },
    DebugLine {
        start: Vec2,
        end: Vec2,
        color: [f32; 4],
        width: f32,
    },
    Tile(TileParams),
    TileBatch {
        texture: Arc<TextureAsset>,
        instances: Vec<InstanceData>,
        order: i32,
        depth: f32,
    },
    Mesh3D(Mesh3dParams),
    // IU
    UiRect {
        rect: UiRect,
        color: [f32; 4],
        z_index: i16,
    },
    UiImage {
        texture: Arc<TextureAsset>,
        rect: UiRect,
        tint: [f32; 4],
        uv_rect: [f32; 4],
        z_index: i16,
    },
    UiText {
        text: String,
        rect: UiRect,
        color: [f32; 4],
        font_size: u16,
        z_index: i16,
        font_id: Option<FontId>,
        segments: Vec<RichTextSegment>,
    },
}
