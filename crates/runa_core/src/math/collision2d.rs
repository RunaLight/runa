use glam::{Quat, Vec2, Vec3, Vec3Swizzles};

use crate::components::WorldCollider2D;

pub fn rect_corners(center: Vec2, quat: Quat, half: Vec2, offset: Vec2) -> [Vec2; 4] {
    let c = center + (quat * Vec3::new(offset.x, offset.y, 0.0)).xy();
    let ax = (quat * Vec3::X).xy(); // world X axis of the box (foreshortened when tilted)
    let ay = (quat * Vec3::Y).xy(); // world Y axis of the box
    [
        c + ax * half.x + ay * half.y,
        c + ax * half.x - ay * half.y,
        c - ax * half.x - ay * half.y,
        c - ax * half.x + ay * half.y,
    ]
}

/// Dispatch two already-resolved world colliders to the narrow-phase test.
pub fn intersects_world(a: &WorldCollider2D, b: &WorldCollider2D) -> bool {
    match (a, b) {
        (WorldCollider2D::Rect { corners: ca }, WorldCollider2D::Rect { corners: cb }) => {
            rect_vs_rect(*ca, *cb)
        }
        (
            WorldCollider2D::Circle {
                center: ca,
                radius: ra,
            },
            WorldCollider2D::Circle {
                center: cb,
                radius: rb,
            },
        ) => circle_vs_circle(*ca, *ra, *cb, *rb),
        (WorldCollider2D::Rect { corners }, WorldCollider2D::Circle { center, radius })
        | (WorldCollider2D::Circle { center, radius }, WorldCollider2D::Rect { corners }) => {
            rect_vs_circle(*corners, *center, *radius)
        }
    }
}

pub fn rect_vs_rect(a: [Vec2; 4], b: [Vec2; 4]) -> bool {
    let axes = [
        edge_normal(a, 0),
        edge_normal(a, 1),
        edge_normal(b, 0),
        edge_normal(b, 1),
    ];
    for axis in axes {
        let (amin, amax) = project(a, axis);
        let (bmin, bmax) = project(b, axis);

        if amax < bmin || bmax < amin {
            return false;
        }
    }
    true
}

fn project(corners: [Vec2; 4], axis: Vec2) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for c in corners {
        let p = c.dot(axis);
        min = min.min(p);
        max = max.max(p);
    }
    (min, max)
}

fn edge_normal(c: [Vec2; 4], i: usize) -> Vec2 {
    let edge = c[i + 1] - c[i];
    Vec2::new(-edge.y, edge.x).normalize()
}

pub fn circle_vs_circle(a: Vec2, ra: f32, b: Vec2, rb: f32) -> bool {
    a.distance(b) <= ra + rb
}

pub fn rect_vs_circle(rect: [Vec2; 4], circle: Vec2, radius: f32) -> bool {
    let mut best = f32::INFINITY;
    for i in 0..4 {
        let p = closest_point_on_segment(rect[i], rect[(i + 1) % 4], circle);
        best = best.min(p.distance(circle));
    }

    best <= radius
}

fn closest_point_on_segment(point: Vec2, point_2: Vec2, circle_pos: Vec2) -> Vec2 {
    circle_pos.clamp(point, point_2)
}
