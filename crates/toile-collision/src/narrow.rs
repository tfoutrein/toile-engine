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
