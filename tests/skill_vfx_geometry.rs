//! Check the actual runtime artwork without compiling the unmigrated class unit tests.
#[path = "../src/combat/skill_vfx_art.rs"]
mod art;
use bevy::{mesh::VertexAttributeValues, prelude::*};

fn positions(mesh: &Mesh) -> &Vec<[f32; 3]> {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
        VertexAttributeValues::Float32x3(points) => points,
        _ => panic!("expected 3D positions"),
    }
}

#[test]
fn heavy_cut_stays_in_front_of_the_hero_and_leaves_the_centre_open() {
    let artwork = art::arc(2.65, 0.065);
    for point in positions(&artwork) {
        assert!(point.iter().all(|v| v.is_finite()));
        assert!(
            point[2] > 0.0,
            "a forward cut must not appear behind the hero"
        );
        let radius = Vec2::new(point[0], point[2]).length();
        assert!(
            (0.90..1.10).contains(&radius),
            "blade ribbon must not become an opaque disc"
        );
    }
}

#[test]
fn shockwave_boundary_matches_its_radius_and_does_not_cover_the_enemy() {
    let artwork = art::arc(std::f32::consts::TAU, 0.017);
    let points = positions(&artwork);
    for point in points {
        let radius = Vec2::new(point[0], point[2]).length();
        assert!((0.97..1.03).contains(&radius));
        assert_eq!(point[1], 0.0);
    }
    for axis in [0, 2] {
        assert!(points.iter().any(|p| p[axis] > 0.97));
        assert!(points.iter().any(|p| p[axis] < -0.97));
    }
}

#[test]
fn retreat_ribbon_tapers_at_the_start_and_stays_on_the_backward_path() {
    let mut points = Vec::new();
    art::ribbon(&mut points, Vec2::ZERO, -Vec2::Y * 4.0, 0.0, 0.08);
    let artwork = art::mesh(points);
    let p = positions(&artwork);
    assert!(p.iter().any(|v| v[2] == 0.0 && v[0] == 0.0));
    assert!(p.iter().any(|v| v[2] == -4.0 && v[0].abs() == 0.08));
    assert!(p.iter().all(|v| v[2] <= 0.0 && v[1] == 0.0));
    let a = Vec3::from_array(p[0]);
    let b = Vec3::from_array(p[1]);
    let c = Vec3::from_array(p[2]);
    assert!((b - a).cross(c - a).length() > 0.01);
}
