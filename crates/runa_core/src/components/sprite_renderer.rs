use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use runa_asset::{Handle, TextureAsset};
use runa_macros::Scriptable;

pub const DEFAULT_SPRITE_PIXELS_PER_UNIT: f32 = 100.0;

/// 2D sprite component — draws a textured quad.
#[derive(Clone, Scriptable)]
#[script(crate = "::runa_script_api", builtin)]
pub struct SpriteRenderer {
    #[script(skip)]
    pub texture: OnceLock<Option<Handle<TextureAsset>>>,
    pub texture_path: Option<String>,
    pub pixels_per_unit: f32,
    pub uv_rect: [f32; 4],
    pub color: [f32; 4],
    /// When true, output `color` directly (using texture alpha for transparency).
    /// When false, multiply texture with `color` (legacy behavior).
    pub replace_color: bool,
    /// Flip the sprite horizontally when rendering.
    pub flip_x: bool,
    /// Flip the sprite vertically when rendering.
    pub flip_y: bool,
}

impl SpriteRenderer {
    pub const FULL_UV_RECT: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

    /// Create a new sprite renderer with optional texture.
    pub fn new(texture: Option<Handle<TextureAsset>>) -> Self {
        let texture_path = texture
            .as_ref()
            .map(|handle| handle.inner.path.to_string_lossy().to_string());
        let lock = OnceLock::new();
        if let Some(t) = texture {
            let _ = lock.set(Some(t));
        }
        Self {
            texture: lock,
            texture_path,
            pixels_per_unit: DEFAULT_SPRITE_PIXELS_PER_UNIT,
            uv_rect: Self::FULL_UV_RECT,
            color: [1.0; 4],
            replace_color: false,
            flip_x: false,
            flip_y: false,
        }
    }

    /// Create from file path (lazy-load on first access).
    pub fn from_path(path: &str) -> Self {
        Self {
            texture: OnceLock::new(),
            texture_path: Some(path.into()),
            pixels_per_unit: DEFAULT_SPRITE_PIXELS_PER_UNIT,
            uv_rect: Self::FULL_UV_RECT,
            color: [1.0; 4],
            replace_color: false,
            flip_x: false,
            flip_y: false,
        }
    }

    /// Get the texture handle, loading from `texture_path` on first access if needed.
    pub fn texture(&self) -> Option<&Handle<TextureAsset>> {
        self.texture
            .get_or_init(|| {
                let path = self.texture_path.as_ref()?;
                TextureAsset::load(&PathBuf::from(path))
                    .ok()
                    .map(|asset| Handle {
                        inner: Arc::new(asset),
                    })
            })
            .as_ref()
    }

    /// Consume and return the texture handle, loading if needed.
    pub fn texture_owned(&self) -> Option<Handle<TextureAsset>> {
        self.texture().cloned()
    }

    /// Unwrap the texture handle (panics if missing and cannot be loaded).
    pub fn get_texture_handle(&self) -> Handle<TextureAsset> {
        self.texture_owned().unwrap()
    }

    /// Replace both the texture and its path.
    pub fn set_texture(&mut self, texture: Option<Handle<TextureAsset>>) {
        let texture_path = texture
            .as_ref()
            .map(|handle| handle.inner.path.to_string_lossy().to_string());
        let lock = OnceLock::new();
        if let Some(h) = texture {
            let _ = lock.set(Some(h));
        }
        self.texture = lock;
        self.texture_path = texture_path;
    }

    pub fn pixels_per_unit(&self) -> f32 {
        self.pixels_per_unit.max(f32::EPSILON)
    }

    pub fn set_uv_rect(&mut self, uv_rect: [f32; 4]) {
        self.uv_rect = [
            uv_rect[0].clamp(0.0, 1.0),
            uv_rect[1].clamp(0.0, 1.0),
            uv_rect[2].clamp(0.0, 1.0),
            uv_rect[3].clamp(0.0, 1.0),
        ];
    }

    pub fn frame_size_pixels(&self) -> Option<[f32; 2]> {
        let texture = self.texture()?;
        Some([
            texture.inner.width as f32 * self.uv_rect[2].max(f32::EPSILON),
            texture.inner.height as f32 * self.uv_rect[3].max(f32::EPSILON),
        ])
    }
}

impl Default for SpriteRenderer {
    fn default() -> Self {
        Self {
            texture: OnceLock::new(),
            texture_path: None,
            pixels_per_unit: DEFAULT_SPRITE_PIXELS_PER_UNIT,
            uv_rect: Self::FULL_UV_RECT,
            color: [1.0; 4],
            replace_color: false,
            flip_x: false,
            flip_y: false,
        }
    }
}
