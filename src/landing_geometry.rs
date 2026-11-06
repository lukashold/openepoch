//! Original fixed-runway geometry, independent of path execution and admission.

pub type Point = [f32; 2];

const RADIUS_FACTOR: f64 = f32::from_bits(0x3f4c_cccd) as f64;
const TURN_EPSILON: f64 = 0.01;

#[derive(Clone, Copy)]
pub struct Runway {
    pub first: Point,
    pub second: Point,
}

#[derive(Clone, Copy)]
pub struct Aircraft {
    pub position: Point,
    pub velocity: Point,
    pub heading: f32,
    pub turn_rate: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Approach {
    pub turn_left: bool,
    pub point: Point,
}

pub fn approach(runway: Runway, actor: Aircraft) -> Approach {
    let [dx, dy] = difference(runway.second, runway.first);
    let [x, y] = difference(actor.position, runway.first);
    // 007D658A computes the negative runway-side cross product and stores it
    // before checking the sign. Equal points use the right-hand circle.
    let side = (f64::from(dy) * f64::from(x) - f64::from(dx) * f64::from(y)) as f32;
    let turn_left = side > 0.0;
    let [vx, vy] = actor.velocity;
    let offset = if turn_left { [-vy, vx] } else { [vy, -vx] };
    Approach { turn_left, point: add(actor.position, offset) }
}

pub fn aligned_for_landing(approach: Approach, angular_velocity: f32, effective_turn: f32) -> bool {
    let target = if approach.turn_left { effective_turn } else { -effective_turn };
    f64::from((angular_velocity - target).abs()) < TURN_EPSILON
}

fn add([ax, ay]: Point, [bx, by]: Point) -> Point { [ax + bx, ay + by] }

fn difference([ax, ay]: Point, [bx, by]: Point) -> Point { [ax - bx, ay - by] }

fn normalized(dx: f64, dy: f64) -> Point {
    let length = (dx * dx + dy * dy).sqrt();
    if length > 0.0 { [(dx / length) as f32, (dy / length) as f32] }
    else { [0.0, 0.0] }
}

/// Mode 4 starts at cursor 1 in these seven stored path records.
pub fn landing_path(runway: Runway, actor: Aircraft, speed: f32, turn: f32) -> Option<[Point; 7]> {
    if !speed.is_finite() || !turn.is_finite() || speed <= 0.0 || turn <= 0.0 { return None; }
    let [dx, dy] = difference(runway.second, runway.first);
    let [dx, dy] = normalized(f64::from(dx), f64::from(dy));
    if dx == 0.0 && dy == 0.0 { return None; }

    let far = add(runway.second, [dx * 10.0, dy * 10.0]);
    let radius = (f64::from(speed) / f64::from(turn) * RADIUS_FACTOR) as f32;
    if !radius.is_finite() { return None; }
    let (sin, cos) = f64::from(actor.heading).sin_cos();
    let (sin, cos) = (sin as f32, cos as f32);
    let sign = if actor.turn_rate > 0.0 { 1.0f32 } else { -1.0 };

    // 007D68C8 and 007D69BB use the common external tangent of
    // the two same-direction circles. Their X offsets store before
    // translation. The destination center's Y stays on the x87 stack.
    let start_x = actor.position[0] + -sign * sin * radius;
    let start_y = (f64::from(actor.position[1]) + f64::from(sign * cos) * f64::from(radius)) as f32;
    let end_x = far[0] + sign * dy * radius;
    let end_y = f64::from(far[1]) - f64::from(sign * dx) * f64::from(radius);
    let [tx, ty] = normalized(f64::from(end_x) - f64::from(start_x), end_y - f64::from(start_y));
    let offset_x = f64::from(sign * ty) * f64::from(radius);
    let offset_y = -f64::from(sign * tx) * f64::from(radius);
    let entry = [start_x + offset_x as f32, (f64::from(start_y) + offset_y) as f32];
    let exit = [(f64::from(end_x) + offset_x) as f32, (end_y + offset_y) as f32];
    let near = [(f64::from(runway.second[0]) + f64::from(dx) * 5.0) as f32,
                runway.second[1] + dy * 5.0];
    Some([actor.position, entry, exit, far, near, runway.second, runway.first])
}

pub fn landing_speed(speed: f32, mode: u8, remaining: usize) -> f32 {
    match (mode, remaining) {
        (4, 2) => (f64::from(speed) * 0.7) as f32,
        (5, 1) => (f64::from(speed) * 0.3) as f32,
        _ => speed,
    }
}

pub fn landing_height(cruise: f32, deck_offset: f32, remaining: usize, distance: f64) -> Option<(u8, f32)> {
    Some(match remaining {
        0 => return None,
        1 => (5, deck_offset),
        2 if distance < 3.0 => {
            let bottom = f64::from(deck_offset) + f64::from(0.1f32);
            let height = bottom + (f64::from(cruise) - bottom) * distance * f64::from(0.33333334f32);
            (4, height as f32)
        }
        _ => (4, cruise),
    })
}
