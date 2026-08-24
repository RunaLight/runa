use glam::{Quat, Vec3};

use crate::components::WorldCollider3D;

/// Oriented bounding box in world space, used only inside the narrow phase.
struct Obb {
    center: Vec3,
    half_size: Vec3,
    rotation: Quat,
}

/// Dispatch two already-resolved world colliders to the narrow-phase test.
pub fn intersects_world(a: &WorldCollider3D, b: &WorldCollider3D) -> bool {
    match (a, b) {
        (
            WorldCollider3D::Box {
                center: ca,
                half_size: ha,
                rotation: ra,
            },
            WorldCollider3D::Box {
                center: cb,
                half_size: hb,
                rotation: rb,
            },
        ) => box_vs_box(
            &Obb {
                center: *ca,
                half_size: *ha,
                rotation: *ra,
            },
            &Obb {
                center: *cb,
                half_size: *hb,
                rotation: *rb,
            },
        ),
        (
            WorldCollider3D::Sphere {
                center: ca,
                radius: ra,
            },
            WorldCollider3D::Sphere {
                center: cb,
                radius: rb,
            },
        ) => sphere_vs_sphere(*ca, *ra, *cb, *rb),
        (
            WorldCollider3D::Box {
                center,
                half_size,
                rotation,
            },
            WorldCollider3D::Sphere { center: sc, radius },
        )
        | (
            WorldCollider3D::Sphere { center: sc, radius },
            WorldCollider3D::Box {
                center,
                half_size,
                rotation,
            },
        ) => box_vs_sphere(
            &Obb {
                center: *center,
                half_size: *half_size,
                rotation: *rotation,
            },
            *sc,
            *radius,
        ),
    }
}

fn sphere_vs_sphere(a: Vec3, ra: f32, b: Vec3, rb: f32) -> bool {
    a.distance(b) <= ra + rb
}

/// Closest point on the OBB to the sphere center, in world space, then
/// compare distance to the sphere radius.
fn box_vs_sphere(obb: &Obb, sphere_center: Vec3, sphere_radius: f32) -> bool {
    let local = obb.rotation.inverse() * (sphere_center - obb.center);
    let closest = Vec3::new(
        local.x.clamp(-obb.half_size.x, obb.half_size.x),
        local.y.clamp(-obb.half_size.y, obb.half_size.y),
        local.z.clamp(-obb.half_size.z, obb.half_size.z),
    );
    let closest_world = obb.center + obb.rotation * closest;
    closest_world.distance(sphere_center) <= sphere_radius
}

/// OBB vs OBB via SAT over the 15 axes (3 face axes each + 9 edge cross
/// products). Returns only overlap; depth/normal would be added for a
/// depenetration pass later.
fn box_vs_box(a: &Obb, b: &Obb) -> bool {
    let a_axes = [
        a.rotation * Vec3::X,
        a.rotation * Vec3::Y,
        a.rotation * Vec3::Z,
    ];
    let b_axes = [
        b.rotation * Vec3::X,
        b.rotation * Vec3::Y,
        b.rotation * Vec3::Z,
    ];

    let mut axes: Vec<Vec3> = vec![
        a_axes[0], a_axes[1], a_axes[2], b_axes[0], b_axes[1], b_axes[2],
    ];
    for &ai in &a_axes {
        for &bi in &b_axes {
            let cross = ai.cross(bi);
            if cross.length_squared() > 1e-8 {
                axes.push(cross.normalize());
            }
        }
    }

    for axis in axes {
        let (amin, amax) = project_box(a, &a_axes, axis);
        let (bmin, bmax) = project_box(b, &b_axes, axis);
        if amax < bmin || bmax < amin {
            return false;
        }
    }
    true
}

/// Project an OBB onto an axis. Radius along the axis = sum of component
/// extents times the absolute dot of the axis with each box axis.
fn project_box(obb: &Obb, axes: &[Vec3; 3], axis: Vec3) -> (f32, f32) {
    let r = obb.half_size.x * axis.dot(axes[0]).abs()
        + obb.half_size.y * axis.dot(axes[1]).abs()
        + obb.half_size.z * axis.dot(axes[2]).abs();
    let c = obb.center.dot(axis);
    (c - r, c + r)
}
