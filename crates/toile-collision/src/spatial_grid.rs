use std::collections::{HashMap, HashSet};

use glam::Vec2;

/// Uniform spatial grid for broad-phase collision detection.
///
/// Insert entities with their bounding box, then query for candidate pairs.
/// Pairs are deduplicated across cells.
pub struct SpatialGrid {
    cell_size: f32,
    inv_cell_size: f32,
    cells: HashMap<(i32, i32), Vec<u32>>,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            inv_cell_size: 1.0 / cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        for cell in self.cells.values_mut() {
            cell.clear();
        }
    }

    /// Insert an entity into all cells its bounding box overlaps.
    pub fn insert(&mut self, index: u32, center: Vec2, half_extents: Vec2) {
        let min = center - half_extents;
        let max = center + half_extents;

        let x0 = (min.x * self.inv_cell_size).floor() as i32;
        let y0 = (min.y * self.inv_cell_size).floor() as i32;
        let x1 = (max.x * self.inv_cell_size).floor() as i32;
        let y1 = (max.y * self.inv_cell_size).floor() as i32;

        for cx in x0..=x1 {
            for cy in y0..=y1 {
                self.cells.entry((cx, cy)).or_default().push(index);
            }
        }
    }

    /// Return all unique candidate pairs (i, j) where i < j.
    pub fn query_pairs(&self) -> Vec<(u32, u32)> {
        let mut seen = HashSet::new();

        for cell in self.cells.values() {
            if cell.len() < 2 {
                continue;
            }
            for i in 0..cell.len() {
                for j in (i + 1)..cell.len() {
                    let a = cell[i].min(cell[j]);
                    let b = cell[i].max(cell[j]);
                    seen.insert((a, b));
                }
            }
        }

        seen.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_pairs() {
        let mut grid = SpatialGrid::new(100.0);
        // Two overlapping entities in the same cell
        grid.insert(0, Vec2::new(50.0, 50.0), Vec2::new(10.0, 10.0));
        grid.insert(1, Vec2::new(60.0, 50.0), Vec2::new(10.0, 10.0));
        // One entity far away
        grid.insert(2, Vec2::new(500.0, 500.0), Vec2::new(10.0, 10.0));

        let pairs = grid.query_pairs();
        assert_eq!(pairs.len(), 1);
        assert!(pairs.contains(&(0, 1)));
    }
}
