//! Sparse, directional enemy effects. The terrain and actor silhouette stay visible.
use bevy::prelude::*;

#[derive(Clone, Copy)]
pub(super) enum GroundMark {
    Fracture,
    Arcane,
    Shackles,
    Frost,
    Blight,
    Seal,
    Wave,
}

fn line(points: &mut Vec<[f32; 3]>, a: Vec2, b: Vec2, width: f32) {
    let normal = (b - a).normalize_or_zero().perp() * width * 0.5;
    for point in [
        a + normal,
        b + normal,
        b - normal,
        a + normal,
        b - normal,
        a - normal,
    ] {
        points.push([point.x, 0.0, point.y]);
    }
}

fn polar(radius: f32, angle: f32) -> Vec2 {
    Vec2::new(angle.cos(), angle.sin()) * radius
}

fn finish(points: Vec<[f32; 3]>) -> Mesh {
    let count = points.len();
    let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count]);
    mesh
}

/// Fine outer boundary communicates the damage footprint; interior marks
/// distinguish the hazard without filling the footprint with opaque geometry.
pub(super) fn ground_mark(radius: f32, kind: GroundMark) -> Mesh {
    let mut points = Vec::new();
    let width = 0.018;
    for segment in 0..64 {
        let angle = segment as f32 * std::f32::consts::TAU / 64.0;
        let next = (segment + 1) as f32 * std::f32::consts::TAU / 64.0;
        line(
            &mut points,
            polar(radius - width, angle),
            polar(radius - width, next),
            width,
        );
    }
    let spokes = match kind {
        GroundMark::Fracture | GroundMark::Blight => 5,
        GroundMark::Frost | GroundMark::Arcane => 6,
        GroundMark::Shackles | GroundMark::Seal => 4,
        GroundMark::Wave => 0,
    };
    for spoke in 0..spokes {
        let angle = spoke as f32 * std::f32::consts::TAU / spokes as f32;
        match kind {
            GroundMark::Fracture | GroundMark::Blight => {
                let a = polar(radius * 0.15, angle);
                let b = polar(radius * 0.44, angle + 0.19);
                let c = polar(radius * 0.72, angle - 0.10);
                line(&mut points, a, b, 0.025);
                line(&mut points, b, c, 0.020);
                line(&mut points, b, polar(radius * 0.57, angle + 0.38), 0.015);
            }
            GroundMark::Frost => {
                line(
                    &mut points,
                    polar(radius * 0.12, angle),
                    polar(radius * 0.70, angle),
                    width,
                );
                let junction = polar(radius * 0.43, angle);
                for offset in [-0.24, 0.24] {
                    line(
                        &mut points,
                        junction,
                        polar(radius * 0.60, angle + offset),
                        width,
                    );
                }
            }
            GroundMark::Arcane | GroundMark::Shackles | GroundMark::Seal => {
                // Engraved chevrons, with a crossbar for the binding seal.
                let tip = polar(radius * 0.80, angle);
                line(&mut points, polar(radius * 0.56, angle - 0.16), tip, width);
                line(&mut points, tip, polar(radius * 0.56, angle + 0.16), width);
                if matches!(kind, GroundMark::Shackles | GroundMark::Seal) {
                    line(
                        &mut points,
                        polar(radius * 0.65, angle - 0.16),
                        polar(radius * 0.65, angle + 0.16),
                        width,
                    );
                }
            }
            GroundMark::Wave => {}
        }
    }
    finish(points)
}

/// A narrow bolt, pointed along local +Z, with visible tip and paired fletching.
pub(super) fn projectile_bolt() -> Mesh {
    let mut points = Vec::new();
    line(
        &mut points,
        Vec2::new(0.0, -0.29),
        Vec2::new(0.0, 0.17),
        0.035,
    );
    points.extend([[-0.085, 0.0, 0.10], [0.0, 0.0, 0.34], [0.085, 0.0, 0.10]]);
    for side in [-1.0, 1.0] {
        line(
            &mut points,
            Vec2::new(0.0, -0.21),
            Vec2::new(side * 0.07, -0.32),
            0.025,
        );
    }
    // Mirrored winding makes the bolt readable above and below the camera plane.
    let reverse = points
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|tri| [tri[2], tri[1], tri[0]])
        .collect::<Vec<_>>();
    points.extend(reverse);
    finish(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    #[test]
    fn hazard_marks_leave_ground_visible_and_stay_inside_damage_boundary() {
        for kind in [
            GroundMark::Fracture,
            GroundMark::Arcane,
            GroundMark::Shackles,
            GroundMark::Frost,
            GroundMark::Blight,
            GroundMark::Seal,
            GroundMark::Wave,
        ] {
            let radius = 1.35;
            let mesh = ground_mark(radius, kind);
            let Some(VertexAttributeValues::Float32x3(points)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("missing positions")
            };
            let mut area = 0.0;
            for triangle in points.as_chunks::<3>().0 {
                let [a, b, c] = [triangle[0], triangle[1], triangle[2]].map(Vec3::from_array);
                area += (b - a).cross(c - a).length() * 0.5;
                for point in [a, b, c] {
                    assert_eq!(point.y, 0.0);
                    assert!(point.length() <= radius);
                }
            }
            assert!(
                area < std::f32::consts::PI * radius * radius * 0.12,
                "hazard must not become a filled disc"
            );
        }
    }

    #[test]
    fn projectile_has_directional_tip_instead_of_spherical_silhouette() {
        let mesh = projectile_bolt();
        let Some(VertexAttributeValues::Float32x3(points)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("missing positions")
        };
        assert!(points.iter().all(|p| p[0].abs() <= 0.09 && p[1] == 0.0));
        assert!(points.iter().any(|p| p[2] >= 0.34));
        assert!(points.iter().any(|p| p[2] <= -0.30));
    }
}
