use nalgebra::Point2;
use std::collections::VecDeque;

/// A simple log-odds occupancy grid with dynamic doubling growth, standing in
/// for Cartographer's `Grid2D` (which uses a fixed-point `ValueConversionTables`
/// lookup instead of float log-odds math -- simpler here, and if anything more
/// expensive per cell, so this is a conservative simplification for timing).
pub struct ProbabilityGrid {
    pub min_x: f64,
    pub min_y: f64,
    pub resolution: f64,
    pub width: usize,
    pub height: usize,
    cells: Vec<f32>,
}

const L_OCC: f32 = 0.85;
const L_FREE: f32 = -0.4;
const L_CLAMP: f32 = 10.0;

impl ProbabilityGrid {
    pub fn new(resolution: f64) -> Self {
        let width = 100;
        let height = 100;
        let half = (width as f64) / 2.0 * resolution;
        ProbabilityGrid {
            min_x: -half,
            min_y: -half,
            resolution,
            width,
            height,
            cells: vec![0.0; width * height],
        }
    }

    fn cell_index(&self, p: Point2<f64>) -> (i64, i64) {
        let ix = ((p.x - self.min_x) / self.resolution).floor() as i64;
        let iy = ((p.y - self.min_y) / self.resolution).floor() as i64;
        (ix, iy)
    }

    fn contains(&self, ix: i64, iy: i64) -> bool {
        ix >= 0 && iy >= 0 && (ix as usize) < self.width && (iy as usize) < self.height
    }

    /// Doubles-and-copies the grid (mirroring `Grid2D::GrowLimits`) until `p` is
    /// within bounds. Returns whether any growth occurred.
    pub fn grow_to_include(&mut self, p: Point2<f64>) -> bool {
        let mut grew = false;
        while {
            let (ix, iy) = self.cell_index(p);
            !self.contains(ix, iy)
        } {
            grew = true;
            let old_w = self.width;
            let old_h = self.height;
            let new_w = old_w * 2;
            let new_h = old_h * 2;
            let offset_x = old_w / 2;
            let offset_y = old_h / 2;
            let new_min_x = self.min_x - self.resolution * offset_x as f64;
            let new_min_y = self.min_y - self.resolution * offset_y as f64;
            let mut new_cells = vec![0.0f32; new_w * new_h];
            for iy in 0..old_h {
                for ix in 0..old_w {
                    new_cells[(iy + offset_y) * new_w + (ix + offset_x)] =
                        self.cells[iy * old_w + ix];
                }
            }
            self.cells = new_cells;
            self.width = new_w;
            self.height = new_h;
            self.min_x = new_min_x;
            self.min_y = new_min_y;
        }
        grew
    }

    fn apply(&mut self, ix: i64, iy: i64, delta: f32) {
        if self.contains(ix, iy) {
            let idx = (iy as usize) * self.width + (ix as usize);
            self.cells[idx] = (self.cells[idx] + delta).clamp(-L_CLAMP, L_CLAMP);
        }
    }

    /// Rasterizes the ray from `origin` to `hit` (Bresenham), applying a free-space
    /// update to traversed cells and an occupied update to the final cell.
    /// Returns whether the grid grew to accommodate either endpoint.
    pub fn insert_ray(&mut self, origin: Point2<f64>, hit: Point2<f64>) -> bool {
        let grew_a = self.grow_to_include(origin);
        let grew_b = self.grow_to_include(hit);
        let (x0, y0) = self.cell_index(origin);
        let (x1, y1) = self.cell_index(hit);

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            if (x, y) == (x1, y1) {
                self.apply(x, y, L_OCC);
                break;
            } else {
                self.apply(x, y, L_FREE);
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
        grew_a || grew_b
    }

    /// Bilinearly-interpolated occupancy probability at a world point (a
    /// simplification of Cartographer's bicubic interpolation in the Ceres cost
    /// function -- see plan's "Assumptions & explicit simplifications").
    pub fn probability_at(&self, p: Point2<f64>) -> f64 {
        let fx = (p.x - self.min_x) / self.resolution - 0.5;
        let fy = (p.y - self.min_y) / self.resolution - 0.5;
        let ix0 = fx.floor();
        let iy0 = fy.floor();
        let tx = fx - ix0;
        let ty = fy - iy0;

        let sample = |ix: i64, iy: i64| -> f32 {
            if self.contains(ix, iy) {
                self.cells[(iy as usize) * self.width + (ix as usize)]
            } else {
                0.0
            }
        };
        let ix0 = ix0 as i64;
        let iy0 = iy0 as i64;
        let l00 = sample(ix0, iy0) as f64;
        let l10 = sample(ix0 + 1, iy0) as f64;
        let l01 = sample(ix0, iy0 + 1) as f64;
        let l11 = sample(ix0 + 1, iy0 + 1) as f64;
        let l0 = l00 * (1.0 - tx) + l10 * tx;
        let l1 = l01 * (1.0 - tx) + l11 * tx;
        let log_odds = l0 * (1.0 - ty) + l1 * ty;
        1.0 / (1.0 + (-log_odds).exp())
    }
}

pub struct Submap {
    pub grid: ProbabilityGrid,
    pub insertions: usize,
}

impl Submap {
    fn new(resolution: f64) -> Self {
        Submap {
            grid: ProbabilityGrid::new(resolution),
            insertions: 0,
        }
    }
}

/// A rolling window of submaps mirroring `ActiveSubmaps2D`: a new submap starts
/// every `num_range_data` insertions, and the oldest is retired once it has
/// received `2 * num_range_data` insertions. The front (oldest/most complete)
/// submap is used for scan matching, matching `active_submaps_.submaps().front()`.
pub struct ActiveSubmaps {
    pub submaps: VecDeque<Submap>,
    num_range_data: usize,
    resolution: f64,
}

impl ActiveSubmaps {
    pub fn new(num_range_data: usize, resolution: f64) -> Self {
        let mut submaps = VecDeque::new();
        submaps.push_back(Submap::new(resolution));
        ActiveSubmaps {
            submaps,
            num_range_data,
            resolution,
        }
    }

    pub fn matching_grid(&self) -> &ProbabilityGrid {
        &self.submaps.front().unwrap().grid
    }

    /// Inserts a scan (single `origin` and a set of hit points, all in map
    /// frame) into every active submap. Returns whether any grid grew.
    pub fn insert(&mut self, origin: Point2<f64>, hits: &[Point2<f64>]) -> bool {
        let mut grew = false;
        for sm in self.submaps.iter_mut() {
            for &h in hits {
                grew |= sm.grid.insert_ray(origin, h);
            }
            sm.insertions += 1;
        }
        if self.submaps.back().unwrap().insertions == self.num_range_data {
            self.submaps.push_back(Submap::new(self.resolution));
        }
        if self.submaps.len() > 1 && self.submaps.front().unwrap().insertions >= 2 * self.num_range_data {
            self.submaps.pop_front();
        }
        grew
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_grows_and_preserves_cells() {
        let mut grid = ProbabilityGrid::new(0.05);
        let origin = Point2::new(0.0, 0.0);
        let near = Point2::new(0.1, 0.1);
        grid.insert_ray(origin, near);
        let old_width = grid.width;
        let far = Point2::new(100.0, 100.0);
        let grew = grid.grow_to_include(far);
        assert!(grew);
        assert!(grid.width > old_width);
        // The originally-hit cell should still read as more occupied than an
        // untouched cell far away.
        let p_hit = grid.probability_at(near);
        let p_unknown = grid.probability_at(Point2::new(50.0, 50.0));
        assert!(p_hit > p_unknown);
    }

    #[test]
    fn ray_rasterization_marks_expected_cells() {
        let mut grid = ProbabilityGrid::new(1.0);
        let origin = Point2::new(0.0, 0.0);
        let hit = Point2::new(5.0, 0.0);
        grid.insert_ray(origin, hit);
        let p_hit = grid.probability_at(hit);
        let p_free = grid.probability_at(Point2::new(2.0, 0.0));
        assert!(p_hit > 0.5);
        assert!(p_free < 0.5);
    }
}
