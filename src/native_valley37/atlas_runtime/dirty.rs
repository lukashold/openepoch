//! Bounded original pathing dirty-event accumulation and grid dispatch.

use super::atlas_reader::{quantized_floor, Precision53};

pub type WorldRect = [f32; 4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterDispatch {
    pub half_cells: [i32; 4],
    pub tree_bounds: WorldRect,
    pub whole_tiles: [i32; 4],
}

const RESET: WorldRect = [f32::MAX, f32::MAX, -f32::MAX, -f32::MAX];

#[derive(Debug, PartialEq)]
pub struct DirtyState {
    pub batching: bool,
    pub pending: WorldRect,
}

impl Default for DirtyState {
    fn default() -> Self {
        Self {
            batching: false,
            pending: RESET,
        }
    }
}

impl DirtyState {
    pub fn begin(&mut self, force: bool, blocking: bool) {
        if force || !blocking {
            self.pending = RESET;
            self.batching = true;
        }
    }

    pub fn event(&mut self, has_atlas: bool, rect: WorldRect) -> Option<WorldRect> {
        if !has_atlas {
            return None;
        }
        if !self.batching {
            return Some(rect);
        }
        for [x, y] in [[rect[0], rect[1]], [rect[2], rect[3]]] {
            for (axis, value) in [x, y].into_iter().enumerate() {
                if value < self.pending[axis] {
                    self.pending[axis] = value;
                }
                if value > self.pending[axis + 2] {
                    self.pending[axis + 2] = value;
                }
            }
        }
        None
    }

    pub fn flush(&mut self, has_atlas: bool) -> Option<WorldRect> {
        let send = self.batching && self.pending[0] != f32::MAX;
        self.batching = false;
        (send && has_atlas).then_some(self.pending)
    }
}

fn bounded(rect: WorldRect) -> bool {
    rect.into_iter()
        .all(|value| value.is_finite() && value.abs() <= 32768.0)
}

pub fn raster_dispatch(rect: WorldRect, precision: Precision53) -> Option<RasterDispatch> {
    if !bounded(rect) {
        return None;
    }
    let [min_x, min_y, max_x, max_y] = rect;
    let shift = |value, offset| precision.add(f64::from(value), offset);
    let y_max = shift(max_y, 6.0);
    let tree_bounds = [
        precision.store_f32(shift(min_x, -6.0)),
        precision.store_f32(shift(min_y, -6.0)),
        precision.store_f32(shift(max_x, 6.0)),
        precision.store_f32(y_max),
    ];
    let floor = |value| quantized_floor(f64::from(value), precision);
    let ceil = |value: f64| value.ceil() as i32;
    Some(RasterDispatch {
        half_cells: [
            2 * floor(tree_bounds[0]),
            2 * floor(tree_bounds[1]),
            2 * ceil(f64::from(tree_bounds[2])),
            2 * ceil(y_max),
        ],
        tree_bounds,
        whole_tiles: [
            floor(min_x),
            floor(min_y),
            ceil(f64::from(max_x)),
            ceil(f64::from(max_y)),
        ],
    })
}

pub fn forest_event(bounds: WorldRect, precision: Precision53) -> Option<WorldRect> {
    if !bounded(bounds) {
        return None;
    }
    let [x0, y0, x1, y1] = bounds.map(|axis| quantized_floor(f64::from(axis), precision));
    Some([
        (x0 - 4) as f32,
        (y0 - 4) as f32,
        (x1 + 4) as f32,
        (y1 + 4) as f32,
    ])
}

pub fn obstacle_event(bounds: WorldRect, [width, height]: [u16; 2]) -> Option<WorldRect> {
    if !bounded(bounds) || width == 0 || height == 0 {
        return None;
    }
    let [mut x0, mut y0, mut x1, mut y1] = bounds;
    if x0 < 0.0 {
        x0 = 0.0;
    }
    if y0 < 0.0 {
        y0 = 0.0;
    }
    let max_x = f32::from(width - 1);
    let max_y = f32::from(height - 1);
    if x1 > max_x {
        x1 = max_x;
    }
    if y1 > max_y {
        y1 = max_y;
    }
    Some([x0, y0, x1, y1])
}
