//! Tapered skill geometry shared by runtime and regression checks.
use bevy::prelude::*;

/// A tapered ribbon on local XZ. No opaque discs, balls or cylinder particles.
pub(super) fn ribbon(points: &mut Vec<[f32; 3]>, a: Vec2, b: Vec2, wa: f32, wb: f32) {
    let n = (b - a).normalize_or_zero().perp();
    for p in [
        a + n * wa,
        b + n * wb,
        b - n * wb,
        a + n * wa,
        b - n * wb,
        a - n * wa,
    ] {
        points.push([p.x, 0.0, p.y]);
    }
}
pub(super) fn mesh(points: Vec<[f32; 3]>) -> Mesh {
    let count = points.len();
    let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count]);
    mesh
}
pub(super) fn arc(sweep: f32, width: f32) -> Mesh {
    let mut points = Vec::new();
    for i in 0..48 {
        let t = i as f32 / 48.0;
        let u = (i + 1) as f32 / 48.0;
        let polar = |v: f32| {
            let a = (v - 0.5) * sweep;
            Vec2::new(a.sin(), a.cos())
        };
        let taper = |v: f32| (std::f32::consts::PI * v).sin().max(0.0).powf(0.6) * width;
        ribbon(&mut points, polar(t), polar(u), taper(t), taper(u));
    }
    mesh(points)
}
