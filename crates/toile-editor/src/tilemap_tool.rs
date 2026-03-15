use glam::Vec2;
use toile_graphics::texture::TextureHandle;
use toile_scene::{TilemapData, TilemapLayerData};

/// Active painting tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileTool {
    Brush,
    Eraser,
    Fill,
}

/// Tilemap editor state.
pub struct TilemapEditor {
    pub active: bool,
    pub tool: TileTool,
    pub selected_gid: u32,
    pub active_layer: usize,
    pub tileset_tex: Option<TextureHandle>,
    pub tileset_columns: u32,
    pub tileset_rows: u32,
    pub tile_size: u32,
}

impl TilemapEditor {
    pub fn new() -> Self {
        Self {
            active: false,
            tool: TileTool::Brush,
            selected_gid: 1,
            active_layer: 0,
            tileset_tex: None,
            tileset_columns: 4,
            tileset_rows: 1,
            tile_size: 32,
        }
    }

    /// Convert a world position to tile grid coordinates.
    /// The tilemap is centered at the world origin.
    pub fn world_to_tile(&self, world_pos: Vec2, map_width: u32, map_height: u32) -> Option<(u32, u32)> {
        let ts = self.tile_size as f32;
        let map_w = map_width as f32 * ts;
        let map_h = map_height as f32 * ts;

        // Offset so tilemap is centered at origin
        let local_x = world_pos.x + map_w * 0.5;
        let local_y = map_h * 0.5 - world_pos.y;

        let col = (local_x / ts).floor() as i32;
        let row = (local_y / ts).floor() as i32;

        if col >= 0 && col < map_width as i32 && row >= 0 && row < map_height as i32 {
            Some((col as u32, row as u32))
        } else {
            None
        }
    }

    /// Paint a tile at (col, row) on the active layer.
    pub fn paint(&self, tilemap: &mut TilemapData, col: u32, row: u32) {
        if let Some(layer) = tilemap.layers.get_mut(self.active_layer) {
            let idx = (row * tilemap.width + col) as usize;
            if idx < layer.tiles.len() {
                layer.tiles[idx] = self.selected_gid;
            }
        }
    }

    /// Erase a tile (set to 0).
    pub fn erase(&self, tilemap: &mut TilemapData, col: u32, row: u32) {
        if let Some(layer) = tilemap.layers.get_mut(self.active_layer) {
            let idx = (row * tilemap.width + col) as usize;
            if idx < layer.tiles.len() {
                layer.tiles[idx] = 0;
            }
        }
    }

    /// Flood fill from (col, row) with selected_gid.
    pub fn flood_fill(&self, tilemap: &mut TilemapData, start_col: u32, start_row: u32) {
        let Some(layer) = tilemap.layers.get_mut(self.active_layer) else {
            return;
        };

        let w = tilemap.width;
        let h = tilemap.height;
        let start_idx = (start_row * w + start_col) as usize;
        if start_idx >= layer.tiles.len() {
            return;
        }

        let target_gid = layer.tiles[start_idx];
        if target_gid == self.selected_gid {
            return; // already the same
        }

        let mut stack = vec![(start_col, start_row)];
        while let Some((col, row)) = stack.pop() {
            let idx = (row * w + col) as usize;
            if idx >= layer.tiles.len() || layer.tiles[idx] != target_gid {
                continue;
            }
            layer.tiles[idx] = self.selected_gid;

            if col > 0 { stack.push((col - 1, row)); }
            if col + 1 < w { stack.push((col + 1, row)); }
            if row > 0 { stack.push((col, row - 1)); }
            if row + 1 < h { stack.push((col, row + 1)); }
        }
    }

    /// Get UV coordinates for a tile GID in the tileset.
    pub fn tile_uv(&self, gid: u32) -> (Vec2, Vec2) {
        if gid == 0 || self.tileset_columns == 0 {
            return (Vec2::ZERO, Vec2::ZERO);
        }
        let local_id = gid - 1; // GIDs start at 1
        let col = local_id % self.tileset_columns;
        let row = local_id / self.tileset_columns;
        let total_w = self.tileset_columns as f32;
        let total_h = self.tileset_rows as f32;

        let uv_min = Vec2::new(col as f32 / total_w, row as f32 / total_h);
        let uv_max = Vec2::new((col + 1) as f32 / total_w, (row + 1) as f32 / total_h);
        (uv_min, uv_max)
    }
}

/// Create a default empty tilemap.
pub fn create_default_tilemap(width: u32, height: u32, tile_size: u32, tileset_path: &str, columns: u32) -> TilemapData {
    let total_tiles = (width * height) as usize;
    TilemapData {
        tileset_path: tileset_path.to_string(),
        tile_size,
        columns,
        width,
        height,
        layers: vec![TilemapLayerData {
            name: "Ground".to_string(),
            tiles: vec![0; total_tiles],
            visible: true,
        }],
    }
}
