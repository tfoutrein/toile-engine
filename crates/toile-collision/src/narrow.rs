use glam::Vec2;

use crate::shape::{Collider, Shape};

/// Test overlap between two colliders at given positions.
/// Returns the minimum translation vector (MTV) to push `a` out of `b`, or `None`.
pub fn overlap_test(pos_a: Vec2, col_a: &Collider, pos_b: Vec2, col_b: &Collider) -> Option<Vec2> {
    let ca = pos_a + col_a.offset;
    let cb = pos_b + col_b.offset;

    match (&col_a.shape, &col_b.shape) {
        (Shape::Aabb { half_extents: ha }, Shape::Aabb { half_extents: hb }) => {
            aabb_vs_aabb(ca, *ha, cb, *hb)
        }
        (Shape::Circle { radius: ra }, Shape::Circle { radius: rb }) => {
            circle_vs_circle(ca, *ra, cb, *rb)
        }
        (Shape::Aabb { half_extents: ha }, Shape::Circle { radius: rb }) => {
            aabb_vs_circle(ca, *ha, cb, *rb)
        }
        (Shape::Circle { radius: ra }, Shape::Aabb { half_extents: hb }) => {
            // Flip: compute MTV pushing B out of A, then negate
            aabb_vs_circle(cb, *hb, ca, *ra).map(|mtv| -mtv)
        }
    }
}

fn aabb_vs_aabb(ca: Vec2, ha: Vec2, cb: Vec2, hb: Vec2) -> Option<Vec2> {
    let d = ca - cb;
    let ox = ha.x + hb.x - d.x.abs();
    let oy = ha.y + hb.y - d.y.abs();

    if ox <= 0.0 || oy <= 0.0 {
        return None;
    }

    if ox < oy {
        Some(Vec2::new(ox * d.x.signum(), 0.0))
    } else {
        Some(Vec2::new(0.0, oy * d.y.signum()))
    }
}

fn circle_vs_circle(ca: Vec2, ra: f32, cb: Vec2, rb: f32) -> Option<Vec2> {
    let d = ca - cb;
    let dist_sq = d.length_squared();
    let min_dist = ra + rb;

    if dist_sq >= min_dist * min_dist {
        return None;
    }

    let dist = dist_sq.sqrt();
    let dir = if dist < 1e-6 { Vec2::X } else { d / dist };

    Some(dir * (min_dist - dist))
}

fn aabb_vs_circle(aabb_center: Vec2, half: Vec2, circle_center: Vec2, radius: f32) -> Option<Vec2> {
    let aabb_min = aabb_center - half;
    let aabb_max = aabb_center + half;

    // Closest point on AABB to circle center
    let closest = Vec2::new(
        circle_center.x.clamp(aabb_min.x, aabb_max.x),
        circle_center.y.clamp(aabb_min.y, aabb_max.y),
    );

    let d = circle_center - closest;
    let dist_sq = d.length_squared();

    if dist_sq >= radius * radius {
        return None;
    }

    if dist_sq < 1e-10 {
        // Circle center is inside AABB — push to nearest edge
        let dx_left = circle_center.x - aabb_min.x;
        let dx_right = aabb_max.x - circle_center.x;
        let dy_bottom = circle_center.y - aabb_min.y;
        let dy_top = aabb_max.y - circle_center.y;

        let min_d = dx_left.min(dx_right).min(dy_bottom).min(dy_top);

        if min_d == dx_left {
            Some(Vec2::new(-(dx_left + radius), 0.0))
        } else if min_d == dx_right {
            Some(Vec2::new(dx_right + radius, 0.0))
        } else if min_d == dy_bottom {
            Some(Vec2::new(0.0, -(dy_bottom + radius)))
        } else {
            Some(Vec2::new(0.0, dy_top + radius))
        }
    } else {
        let dist = dist_sq.sqrt();
        let dir = d / dist;
        // MTV pushes circle (A) out of AABB (B)
        Some(dir * (radius - dist))
    }
}

/// Test overlap between two colliders with rotation (radians).
/// For AABB shapes with rotation, uses SAT (Separating Axis Theorem) for OBB.
/// Circles ignore rotation. Returns MTV or None.
pub fn overlap_test_rotated(
    pos_a: Vec2, col_a: &Collider, rot_a: f32,
    pos_b: Vec2, col_b: &Collider, rot_b: f32,
) -> Option<Vec2> {
    let ca = pos_a + col_a.offset;
    let cb = pos_b + col_b.offset;

    match (&col_a.shape, &col_b.shape) {
        (Shape::Aabb { half_extents: ha }, Shape::Aabb { half_extents: hb }) => {
            if rot_a.abs() < 0.001 && rot_b.abs() < 0.001 {
                // No rotation — use fast AABB path
                aabb_vs_aabb(ca, *ha, cb, *hb)
            } else {
                // OBB vs OBB via SAT
                obb_vs_obb(ca, *ha, rot_a, cb, *hb, rot_b)
            }
        }
        // Circles don't care about rotation
        (Shape::Circle { radius: ra }, Shape::Circle { radius: rb }) => {
            circle_vs_circle(ca, *ra, cb, *rb)
        }
        (Shape::Aabb { half_extents: ha }, Shape::Circle { radius: rb }) => {
            if rot_a.abs() < 0.001 {
                aabb_vs_circle(ca, *ha, cb, *rb)
            } else {
                obb_vs_circle(ca, *ha, rot_a, cb, *rb)
            }
        }
        (Shape::Circle { radius: ra }, Shape::Aabb { half_extents: hb }) => {
            if rot_b.abs() < 0.001 {
                aabb_vs_circle(cb, *hb, ca, *ra).map(|v| -v)
            } else {
                obb_vs_circle(cb, *hb, rot_b, ca, *ra).map(|v| -v)
            }
        }
    }
}

/// OBB vs OBB using Separating Axis Theorem.
fn obb_vs_obb(ca: Vec2, ha: Vec2, rot_a: f32, cb: Vec2, hb: Vec2, rot_b: f32) -> Option<Vec2> {
    let (sin_a, cos_a) = rot_a.sin_cos();
    let (sin_b, cos_b) = rot_b.sin_cos();

    // Local axes for each OBB
    let axes = [
        Vec2::new(cos_a, sin_a),   // A x-axis
        Vec2::new(-sin_a, cos_a),  // A y-axis
        Vec2::new(cos_b, sin_b),   // B x-axis
        Vec2::new(-sin_b, cos_b),  // B y-axis
    ];

    let d = cb - ca;
    let mut min_overlap = f32::MAX;
    let mut min_axis = Vec2::ZERO;

    // Corners of A
    let a_corners = obb_corners(ca, ha, sin_a, cos_a);
    let b_corners = obb_corners(cb, hb, sin_b, cos_b);

    for axis in &axes {
        let (a_min, a_max) = project_corners(&a_corners, *axis);
        let (b_min, b_max) = project_corners(&b_corners, *axis);

        let overlap = (a_max.min(b_max)) - (a_min.max(b_min));
        if overlap <= 0.0 {
            return None; // separating axis found
        }
        if overlap < min_overlap {
            min_overlap = overlap;
            min_axis = *axis;
        }
    }

    // Ensure MTV pushes A away from B
    if d.dot(min_axis) < 0.0 {
        min_axis = -min_axis;
    }
    Some(min_axis * min_overlap)
}

fn obb_corners(center: Vec2, half: Vec2, sin: f32, cos: f32) -> [Vec2; 4] {
    let dx = Vec2::new(cos, sin) * half.x;
    let dy = Vec2::new(-sin, cos) * half.y;
    [
        center - dx - dy,
        center + dx - dy,
        center + dx + dy,
        center - dx + dy,
    ]
}

fn project_corners(corners: &[Vec2; 4], axis: Vec2) -> (f32, f32) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for c in corners {
        let p = c.dot(axis);
        if p < min { min = p; }
        if p > max { max = p; }
    }
    (min, max)
}

/// OBB vs Circle.
fn obb_vs_circle(obb_center: Vec2, obb_half: Vec2, obb_rot: f32, circle_center: Vec2, radius: f32) -> Option<Vec2> {
    // Transform circle center into OBB local space
    let (sin, cos) = obb_rot.sin_cos();
    let d = circle_center - obb_center;
    let local = Vec2::new(d.x * cos + d.y * sin, -d.x * sin + d.y * cos);

    // Closest point on AABB in local space
    let closest_local = Vec2::new(
        local.x.clamp(-obb_half.x, obb_half.x),
        local.y.clamp(-obb_half.y, obb_half.y),
    );

    let diff = local - closest_local;
    let dist_sq = diff.length_squared();

    if dist_sq >= radius * radius {
        return None;
    }

    // Transform MTV back to world space
    if dist_sq < 1e-10 {
        // Circle center inside OBB — push to nearest edge
        let dx = obb_half.x - local.x.abs();
        let dy = obb_half.y - local.y.abs();
        let (local_mtv, pen) = if dx < dy {
            (Vec2::new(local.x.signum(), 0.0), dx + radius)
        } else {
            (Vec2::new(0.0, local.y.signum()), dy + radius)
        };
        let world_mtv = Vec2::new(
            local_mtv.x * cos - local_mtv.y * sin,
            local_mtv.x * sin + local_mtv.y * cos,
        );
        Some(world_mtv * pen)
    } else {
        let dist = dist_sq.sqrt();
        let dir = diff / dist;
        let pen = radius - dist;
        let world_dir = Vec2::new(
            dir.x * cos - dir.y * sin,
            dir.x * sin + dir.y * cos,
        );
        Some(world_dir * pen)
    }
}

pub fn point_in_aabb(point: Vec2, center: Vec2, half_extents: Vec2) -> bool {
    let d = (point - center).abs();
    d.x <= half_extents.x && d.y <= half_extents.y
}

pub fn point_in_circle(point: Vec2, center: Vec2, radius: f32) -> bool {
    (point - center).length_squared() <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::Collider;

    #[test]
    fn aabb_overlap() {
        let a = Collider::aabb(10.0, 10.0);
        let b = Collider::aabb(10.0, 10.0);
        let mtv = overlap_test(Vec2::new(0.0, 0.0), &a, Vec2::new(15.0, 0.0), &b);
        assert!(mtv.is_some());
        let mtv = mtv.unwrap();
        assert!(mtv.x < 0.0); // push A left (away from B)
    }

    #[test]
    fn aabb_no_overlap() {
        let a = Collider::aabb(5.0, 5.0);
        let b = Collider::aabb(5.0, 5.0);
        assert!(overlap_test(Vec2::ZERO, &a, Vec2::new(20.0, 0.0), &b).is_none());
    }

    #[test]
    fn circle_overlap() {
        let a = Collider::circle(10.0);
        let b = Collider::circle(10.0);
        let mtv = overlap_test(Vec2::ZERO, &a, Vec2::new(15.0, 0.0), &b);
        assert!(mtv.is_some());
    }

    #[test]
    fn circle_no_overlap() {
        let a = Collider::circle(5.0);
        let b = Collider::circle(5.0);
        assert!(overlap_test(Vec2::ZERO, &a, Vec2::new(20.0, 0.0), &b).is_none());
    }

    #[test]
    fn aabb_circle_overlap() {
        let a = Collider::aabb(10.0, 10.0);
        let b = Collider::circle(8.0);
        let mtv = overlap_test(Vec2::ZERO, &a, Vec2::new(15.0, 0.0), &b);
        assert!(mtv.is_some());
    }

    #[test]
    fn point_tests() {
        assert!(point_in_aabb(Vec2::new(1.0, 1.0), Vec2::ZERO, Vec2::new(5.0, 5.0)));
        assert!(!point_in_aabb(Vec2::new(10.0, 0.0), Vec2::ZERO, Vec2::new(5.0, 5.0)));
        assert!(point_in_circle(Vec2::new(1.0, 0.0), Vec2::ZERO, 5.0));
        assert!(!point_in_circle(Vec2::new(10.0, 0.0), Vec2::ZERO, 5.0));
    }
}
