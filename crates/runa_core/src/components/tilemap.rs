use glam::{IVec2, USizeVec2, Vec3};
use runa_asset::Handle;
use runa_asset::TextureAsset;
use runa_macros::Scriptable;
use std::sync::Arc;

/// Rectangle used for UV coordinates and placement.
#[derive(Clone, Copy, Debug, Scriptable)]
#[script(crate = "::runa_script_api", not_addable)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// A single tile in a tilemap.
#[derive(Clone, Scriptable)]
#[script(crate = "::runa_script_api", not_addable)]
pub struct Tile {
    #[script(skip)]
    pub texture: Option<Arc<TextureAsset>>, // None means an empty tile
    pub uv_rect: Rect, // Part of the texture atlas
    pub flip_x: bool,
    pub flip_y: bool,
}

impl Tile {
    pub fn new(texture: Arc<TextureAsset>, uv_rect: Rect) -> Self {
        Self {
            texture: Some(texture),
            uv_rect,
            flip_x: false,
            flip_y: false,
        }
    }

    pub fn empty() -> Self {
        Self {
            texture: None,
            uv_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            flip_x: false,
            flip_y: false,
        }
    }
}

/// A single tilemap layer.
#[derive(Clone, Scriptable)]
#[script(crate = "::runa_script_api", not_addable)]
pub struct TilemapLayer {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Tile>, // width * height elements
    pub visible: bool,
    pub opacity: f32,
    pub self_order: i32,
}

impl TilemapLayer {
    pub fn new(name: String, width: u32, height: u32) -> Self {
        Self {
            name,
            width,
            height,
            tiles: vec![Tile::empty(); (width * height) as usize],
            visible: true,
            opacity: 1.0,
            self_order: 0,
        }
    }

    pub fn set_tile(&mut self, x: u32, y: u32, tile: Tile) {
        let index = (y * self.width + x) as usize;
        if index < self.tiles.len() {
            self.tiles[index] = tile;
        }
    }

    pub fn get_tile(&self, x: u32, y: u32) -> Option<&Tile> {
        let index = (y * self.width + x) as usize;
        self.tiles.get(index)
    }
}

/// Tilemap data component.
#[derive(Clone, Scriptable)]
#[script(crate = "::runa_script_api", not_addable)]
pub struct Tilemap {
    /// Map size in tiles.
    pub width: u32,
    pub height: u32,

    /// Tile size in world units.
    /// Example: 16 pixels at pixels_per_unit=16 -> tile_size=1.0
    #[script(skip)]
    pub tile_size: USizeVec2,
    #[script(skip)]
    pub offset: IVec2,

    /// Layers ordered from back to front.
    pub layers: Vec<TilemapLayer>,

    #[script(skip)]
    pub atlas: Option<TilemapAtlas>,
    pub selected_tile: u32,
    pub pixels_per_unit: f32,

    /// Incremented on every tile mutation for dirty tracking.
    #[script(skip)]
    pub generation: u64,
}

impl Tilemap {
    /// Creates a map centered at world origin.
    pub fn centered(width: u32, height: u32, tile_size: USizeVec2) -> Self {
        let offset = IVec2::new(-(width as i32) / 2, -(height as i32) / 2);

        Self {
            width,
            height,
            tile_size,
            offset,
            layers: Vec::new(),
            atlas: None,
            selected_tile: 0,
            pixels_per_unit: 16.0,
            generation: 0,
        }
    }

    pub fn add_layer(&mut self, layer: TilemapLayer) {
        self.layers.push(layer);
    }

    pub fn set_atlas(
        &mut self,
        texture: Option<Handle<TextureAsset>>,
        texture_path: Option<String>,
        columns: u32,
        rows: u32,
    ) {
        self.atlas = texture.map(|texture| TilemapAtlas {
            texture: texture.inner,
            texture_path,
            columns: columns.max(1),
            rows: rows.max(1),
        });
        self.selected_tile = self
            .selected_tile
            .min(self.atlas_frame_count().saturating_sub(1));
    }

    pub fn atlas_frame_count(&self) -> u32 {
        self.atlas
            .as_ref()
            .map(TilemapAtlas::frame_count)
            .unwrap_or(1)
    }

    pub fn atlas_tile(&self, frame: u32) -> Option<Tile> {
        let atlas = self.atlas.as_ref()?;
        Some(Tile::new(
            atlas.texture.clone(),
            atlas.uv_rect_for_frame(frame),
        ))
    }

    pub fn paint_tile(&mut self, layer_index: usize, tile_x: i32, tile_y: i32, frame: u32) {
        let Some(tile) = self.atlas_tile(frame) else {
            return;
        };
        self.ensure_tile_position(tile_x, tile_y);
        let array_x = (tile_x - self.offset.x) as u32;
        let array_y = (tile_y - self.offset.y) as u32;
        if let Some(layer) = self.layers.get_mut(layer_index) {
            layer.set_tile(array_x, array_y, tile);
        }
        self.generation += 1;
    }

    pub fn erase_tile(&mut self, layer_index: usize, tile_x: i32, tile_y: i32) {
        if tile_x < self.offset.x
            || tile_y < self.offset.y
            || tile_x >= self.offset.x + self.width as i32
            || tile_y >= self.offset.y + self.height as i32
        {
            return;
        }
        let array_x = (tile_x - self.offset.x) as u32;
        let array_y = (tile_y - self.offset.y) as u32;
        if let Some(layer) = self.layers.get_mut(layer_index) {
            layer.set_tile(array_x, array_y, Tile::empty());
        }
        self.generation += 1;
    }

    fn ensure_tile_position(&mut self, tile_x: i32, tile_y: i32) {
        let min_x = self.offset.x.min(tile_x);
        let min_y = self.offset.y.min(tile_y);
        let max_x = (self.offset.x + self.width as i32 - 1).max(tile_x);
        let max_y = (self.offset.y + self.height as i32 - 1).max(tile_y);
        let new_width = (max_x - min_x + 1).max(1) as u32;
        let new_height = (max_y - min_y + 1).max(1) as u32;
        if new_width == self.width
            && new_height == self.height
            && min_x == self.offset.x
            && min_y == self.offset.y
        {
            return;
        }

        let old_offset = self.offset;
        let old_width = self.width;
        self.offset = IVec2::new(min_x, min_y);
        self.width = new_width;
        self.height = new_height;
        for layer in &mut self.layers {
            let mut resized = vec![Tile::empty(); (new_width * new_height) as usize];
            for y in 0..layer.height {
                for x in 0..layer.width {
                    let old_index = (y * old_width + x) as usize;
                    let new_x = old_offset.x + x as i32 - min_x;
                    let new_y = old_offset.y + y as i32 - min_y;
                    let new_index = (new_y as u32 * new_width + new_x as u32) as usize;
                    if let (Some(source), Some(target)) = (
                        layer.tiles.get(old_index).cloned(),
                        resized.get_mut(new_index),
                    ) {
                        *target = source;
                    }
                }
            }
            layer.width = new_width;
            layer.height = new_height;
            layer.tiles = resized;
        }
        self.generation += 1;
    }

    /// Sets a tile using world tile coordinates.
    pub fn set_tile(&mut self, world_x: i32, world_y: i32, tile: Tile) {
        // Convert world coordinates to array indices
        let array_x = (world_x - self.offset.x) as u32;
        let array_y = (world_y - self.offset.y) as u32;

        // Bounds check
        if array_x < self.width && array_y < self.height {
            for layer in &mut self.layers {
                layer.set_tile(array_x, array_y, tile.clone());
            }
            self.generation += 1;
        }
    }

    /// Converts world coordinates to tile coordinates.
    pub fn world_to_tile(&self, world_pos: Vec3) -> (i32, i32) {
        let tile_size = self.world_tile_size();
        let tile_x = (world_pos.x / tile_size.x).floor() as i32;
        let tile_y = (world_pos.y / tile_size.y).floor() as i32;
        (tile_x, tile_y)
    }

    /// Converts tile coordinates to the world-space tile center.
    pub fn tile_to_world(&self, tile_x: i32, tile_y: i32) -> Vec3 {
        let world_tile_size = self.world_tile_size();
        Vec3::new(
            (tile_x as f32) * world_tile_size.x,
            (tile_y as f32) * world_tile_size.y,
            0.0,
        )
    }

    pub fn world_tile_size(&self) -> glam::Vec2 {
        let ppu = self.pixels_per_unit.max(f32::EPSILON);
        glam::Vec2::new(self.tile_size.x as f32 / ppu, self.tile_size.y as f32 / ppu)
    }
}

/// Rendering Component for Tilemap
#[derive(Clone, Scriptable)]
#[script(crate = "::runa_script_api")]
pub struct TilemapRenderer;

impl TilemapRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TilemapRenderer {
    fn default() -> Self {
        Self
    }
}

#[derive(Clone)]
pub struct TilemapAtlas {
    pub texture: Arc<TextureAsset>,
    pub texture_path: Option<String>,
    pub columns: u32,
    pub rows: u32,
}

impl TilemapAtlas {
    pub fn frame_count(&self) -> u32 {
        self.columns.saturating_mul(self.rows).max(1)
    }

    pub fn uv_rect_for_frame(&self, frame: u32) -> Rect {
        let columns = self.columns.max(1);
        let rows = self.rows.max(1);
        let frame = frame.min(self.frame_count().saturating_sub(1));
        let col = frame % columns;
        let row = frame / columns;
        let width = 1.0 / columns as f32;
        let height = 1.0 / rows as f32;
        Rect::new(col as f32 * width, row as f32 * height, width, height)
    }

    pub fn tile_index_for_uv(&self, uv_rect: Rect) -> Option<u32> {
        let col = (uv_rect.x * self.columns as f32).round() as u32;
        let row = (uv_rect.y * self.rows as f32).round() as u32;
        let index = row.saturating_mul(self.columns).saturating_add(col);
        (index < self.frame_count()).then_some(index)
    }
}
