//! Ground surface meshes: per-vertex tinted chunks, feathered roads and clearings.
//!
//! Everything here is plain mesh data so the same look ships on native and
//! WebGPU without a custom shader. Vertex colors carry the variation: chunks
//! sample one continuous world-space field (no seams between tiles), roads and
//! clearings darken toward a ragged alpha-masked edge. Alpha *blending* is not
//! available: the WebGPU build has no transparent PBR pipeline, so the edge is
//! a cutout whose outline wanders at two scales instead of a straight block.

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

/// Quads per chunk edge. 2 m cells keep 49 loaded chunks near 30k vertices.
pub const GROUND_CELLS: usize = 24;
/// World metres covered by one repeat of the ground texture.
pub const GROUND_TEXTURE_SPAN: f32 = 4.0;
const ROAD_TEXTURE_SPAN: f32 = 3.2;
const CLEARING_TEXTURE_SPAN: f32 = 6.5;
const ROAD_ROW_SPACING: f32 = 0.5;
/// Width of the masked band outside the paved core.
pub const ROAD_FEATHER: f32 = 0.8;
/// Width of the darkening band just inside the edge.
const ROAD_SHOULDER: f32 = 0.9;
/// Slow meander plus stone-to-stone raggedness of the edge line.
const ROAD_EDGE_WOBBLE: f32 = 0.45;
const ROAD_EDGE_CHIP: f32 = 0.30;
/// Brightness at the edge: a little dirt over the outermost stones. Much darker
/// reads as a black outline around the road.
const EDGE_SHADE: f32 = 0.84;
const CLEARING_SEGMENTS: usize = 48;

#[derive(Default)]
pub struct SurfaceBuffers {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl SurfaceBuffers {
    fn push(&mut self, position: Vec3, uv: Vec2, color: [f32; 4]) -> u32 {
        self.positions.push(position.to_array());
        self.uvs.push(uv.to_array());
        self.colors.push(color);
        (self.positions.len() - 1) as u32
    }

    /// Two triangles wound so the surface faces up.
    fn quad(&mut self, near_left: u32, near_right: u32, far_left: u32, far_right: u32) {
        self.indices.extend_from_slice(&[
            near_left, far_left, near_right, near_right, far_left, far_right,
        ]);
    }

    pub fn into_mesh(self) -> Mesh {
        let normals = vec![[0.0, 1.0, 0.0]; self.positions.len()];
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Deterministic value noise in `0..1`, smooth over one unit.
pub fn value_noise(seed: u64, x: f32, z: f32) -> f32 {
    let x_floor = x.floor();
    let z_floor = z.floor();
    let x0 = x_floor as i64;
    let z0 = z_floor as i64;
    let tx = smoothstep(x - x_floor);
    let tz = smoothstep(z - z_floor);
    let n00 = lattice_value(seed, x0, z0);
    let n10 = lattice_value(seed, x0 + 1, z0);
    let n01 = lattice_value(seed, x0, z0 + 1);
    let n11 = lattice_value(seed, x0 + 1, z0 + 1);
    let north = n00 + (n10 - n00) * tx;
    let south = n01 + (n11 - n01) * tx;
    north + (south - north) * tz
}

fn lattice_value(seed: u64, x: i64, z: i64) -> f32 {
    let hash = mix(seed
        ^ (x as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
        ^ (z as u64)
            .rotate_left(32)
            .wrapping_mul(0xA5A3_564E_27F8_864D));
    ((hash >> 40) as f32) * (1.0 / 16_777_216.0)
}

pub fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

pub fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

/// Blends the four biome tints by a continuous biome value (`0..4`, where
/// `n + 0.5` is the middle of biome `n`) and mottles the result so the tiled
/// albedo never reads as one flat sheet.
pub fn ground_color(seed: u64, palette: &[LinearRgba; 4], biome: f32, world: Vec2) -> [f32; 4] {
    let position = (biome - 0.5).clamp(0.0, 3.0);
    let lower = position.floor() as usize;
    let upper = (lower + 1).min(3);
    let blend = smoothstep(position - lower as f32);
    let tint = palette[lower].mix(&palette[upper], blend);
    let broad = value_noise(seed ^ 0x6D6F_7474_6C65, world.x / 11.0, world.y / 11.0);
    let fine = value_noise(seed ^ 0x6772_6974, world.x / 3.1, world.y / 3.1);
    let shade = 0.72 + 0.34 * broad + 0.14 * (fine - 0.5);
    [tint.red * shade, tint.green * shade, tint.blue * shade, 1.0]
}

/// A flat grid centred on the origin. `color` receives each vertex's world XZ.
pub fn ground_chunk_mesh(center: Vec2, size: f32, color: impl Fn(Vec2) -> [f32; 4]) -> Mesh {
    let mut surface = SurfaceBuffers::default();
    let step = size / GROUND_CELLS as f32;
    let row = GROUND_CELLS as u32 + 1;
    for z in 0..=GROUND_CELLS {
        for x in 0..=GROUND_CELLS {
            let local = Vec2::new(x as f32 * step - size * 0.5, z as f32 * step - size * 0.5);
            let world = center + local;
            surface.push(
                Vec3::new(local.x, 0.0, local.y),
                world / GROUND_TEXTURE_SPAN,
                color(world),
            );
        }
    }
    for z in 0..GROUND_CELLS as u32 {
        for x in 0..GROUND_CELLS as u32 {
            let near = z * row + x;
            let far = near + row;
            surface.quad(near, near + 1, far, far + 1);
        }
    }
    surface.into_mesh()
}

/// Appends a road ribbon from `from` to `to`. The paved core is `width` wide;
/// both edges wander, darken, and drop to zero alpha over [`ROAD_FEATHER`], and
/// each end is a faded cap so consecutive ribbons overlap without a joint. Vertices
/// are relative to `origin`; texture coordinates are world-planar.
pub fn append_road(
    surface: &mut SurfaceBuffers,
    origin: Vec3,
    from: Vec3,
    to: Vec3,
    width: f32,
    seed: u64,
) {
    let start = Vec2::new(from.x, from.z);
    let delta = Vec2::new(to.x - from.x, to.z - from.z);
    let length = delta.length();
    if length <= f32::EPSILON {
        return;
    }
    let along = delta / length;
    let across = Vec2::new(along.y, -along.x);
    let half = width * 0.5;
    let cap = half;
    let rows = ((length + 2.0 * cap) / ROAD_ROW_SPACING).ceil().max(1.0) as usize;
    let mut previous: Option<[u32; 6]> = None;
    for row in 0..=rows {
        let distance = -cap + (length + 2.0 * cap) * row as f32 / rows as f32;
        let end_fade =
            smoothstep((distance + cap) / cap).min(smoothstep((length + cap - distance) / cap));
        let wobble = |salt: u64| {
            (value_noise(seed ^ salt, distance / 3.0, 0.0) - 0.5) * 2.0 * ROAD_EDGE_WOBBLE
                + (value_noise(seed ^ salt.rotate_left(17), distance / 0.6, 7.0) - 0.5)
                    * 2.0
                    * ROAD_EDGE_CHIP
        };
        let left = -half + wobble(0x6C65_6674);
        let right = half + wobble(0x7269_6768);
        let wear = 0.86 + 0.18 * value_noise(seed ^ 0x7765_6172, distance / 5.0, 0.0);
        let edge = wear * EDGE_SHADE;
        let columns = [
            (left - ROAD_FEATHER, edge, 0.0),
            (left, edge, end_fade),
            (left + ROAD_SHOULDER, wear, end_fade),
            (right - ROAD_SHOULDER, wear, end_fade),
            (right, edge, end_fade),
            (right + ROAD_FEATHER, edge, 0.0),
        ];
        let current = columns.map(|(lateral, shade, alpha)| {
            let world = start + along * distance + across * lateral;
            surface.push(
                Vec3::new(world.x - origin.x, 0.0, world.y - origin.z),
                world / ROAD_TEXTURE_SPAN,
                [shade, shade, shade, alpha],
            )
        });
        if let Some(previous) = previous {
            for column in 0..5 {
                surface.quad(
                    previous[column],
                    previous[column + 1],
                    current[column],
                    current[column + 1],
                );
            }
        }
        previous = Some(current);
    }
}

/// Appends an elliptical clearing whose rim wanders and fades to transparent.
pub fn append_clearing(
    surface: &mut SurfaceBuffers,
    origin: Vec3,
    center: Vec3,
    radius: Vec2,
    seed: u64,
) {
    let center_xz = Vec2::new(center.x, center.z);
    let vertex = |surface: &mut SurfaceBuffers, world: Vec2, shade: f32, alpha: f32| {
        surface.push(
            Vec3::new(world.x - origin.x, 0.0, world.y - origin.z),
            world / CLEARING_TEXTURE_SPAN,
            [shade, shade, shade, alpha],
        )
    };
    let hub = vertex(surface, center_xz, 1.0, 1.0);
    let mut previous: Option<[u32; 2]> = None;
    let mut first: Option<[u32; 2]> = None;
    for segment in 0..CLEARING_SEGMENTS {
        let angle = segment as f32 / CLEARING_SEGMENTS as f32 * std::f32::consts::TAU;
        let direction = Vec2::new(angle.cos(), angle.sin());
        // Sampled on a circle so the rim closes without a seam.
        let rim = 0.90 + 0.20 * value_noise(seed, 4.0 + direction.x * 1.7, 4.0 + direction.y * 1.7);
        let shade = 0.90 + 0.14 * value_noise(seed ^ 0x7368_6164, direction.x * 2.3, direction.y);
        let inner = vertex(
            surface,
            center_xz + direction * radius * 0.62 * rim,
            shade,
            1.0,
        );
        let chip =
            0.06 * (value_noise(seed ^ 0x6368_6970, direction.x * 9.0, direction.y * 9.0) - 0.5);
        let outer = vertex(
            surface,
            center_xz + direction * radius * (rim + chip),
            shade * EDGE_SHADE,
            0.0,
        );
        let current = [inner, outer];
        if let Some(previous) = previous {
            surface
                .indices
                .extend_from_slice(&[hub, current[0], previous[0]]);
            surface.quad(previous[0], previous[1], current[0], current[1]);
        }
        first.get_or_insert(current);
        previous = Some(current);
    }
    if let (Some(first), Some(last)) = (first, previous) {
        surface.indices.extend_from_slice(&[hub, first[0], last[0]]);
        surface.quad(last[0], last[1], first[0], first[1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PALETTE: [LinearRgba; 4] = [
        LinearRgba::new(0.8, 0.7, 0.6, 1.0),
        LinearRgba::new(0.9, 0.7, 0.5, 1.0),
        LinearRgba::new(0.7, 0.7, 0.7, 1.0),
        LinearRgba::new(1.0, 0.6, 0.4, 1.0),
    ];

    fn up_facing(surface: &SurfaceBuffers) -> bool {
        surface.indices.chunks(3).all(|triangle| {
            let [a, b, c] = [triangle[0], triangle[1], triangle[2]]
                .map(|index| Vec3::from_array(surface.positions[index as usize]));
            (b - a).cross(c - a).y >= 0.0
        })
    }

    #[test]
    fn neighbouring_chunks_agree_on_their_shared_edge_color() {
        let color = |world: Vec2| ground_color(7, &PALETTE, 1.0 + world.x / 96.0, world);
        let edge = Vec2::new(48.0, 13.0);
        // The same world point is the east edge of one chunk and the west edge of the next.
        assert_eq!(
            color(edge),
            color(Vec2::new(24.0, 13.0) + Vec2::new(24.0, 0.0))
        );
        let step = color(edge + Vec2::X * 0.01);
        for (a, b) in color(edge).iter().zip(step) {
            assert!((a - b).abs() < 0.01, "tint jumps across a chunk border");
        }
    }

    #[test]
    fn ground_tint_varies_inside_one_biome_and_stays_opaque() {
        let samples: Vec<_> = (0..40)
            .map(|i| ground_color(3, &PALETTE, 1.5, Vec2::new(i as f32 * 3.7, i as f32 * 1.9)))
            .collect();
        let reds: Vec<f32> = samples.iter().map(|c| c[0]).collect();
        let spread = reds.iter().cloned().fold(f32::MIN, f32::max)
            - reds.iter().cloned().fold(f32::MAX, f32::min);
        assert!(spread > 0.08, "ground reads as a flat sheet: {spread}");
        assert!(samples.iter().all(|c| c[3] == 1.0));
    }

    #[test]
    fn chunk_mesh_uses_world_planar_uvs_and_faces_up() {
        let mesh = ground_chunk_mesh(Vec2::new(24.0, 72.0), 48.0, |_| [1.0; 4]);
        assert_eq!(
            mesh.count_vertices(),
            (GROUND_CELLS + 1) * (GROUND_CELLS + 1)
        );
        let mut surface = SurfaceBuffers::default();
        append_road(
            &mut surface,
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::new(9.0, 0.0, 4.0),
            4.0,
            1,
        );
        assert!(up_facing(&surface));
    }

    #[test]
    fn road_core_is_opaque_and_every_outer_edge_is_transparent() {
        let mut surface = SurfaceBuffers::default();
        let (from, to, width) = (Vec3::new(2.0, 0.0, 1.0), Vec3::new(30.0, 0.0, -12.0), 4.4);
        append_road(&mut surface, Vec3::ZERO, from, to, width, 11);
        let along = (to - from).normalize();
        let across = Vec3::new(along.z, 0.0, -along.x);
        let mut opaque = 0;
        for (position, color) in surface.positions.iter().zip(&surface.colors) {
            let offset = Vec3::from_array(*position) - from;
            let lateral = offset.dot(across).abs();
            let distance = offset.dot(along);
            if lateral > width * 0.5 + ROAD_EDGE_WOBBLE + ROAD_EDGE_CHIP + 0.01 {
                assert_eq!(color[3], 0.0, "hard edge at lateral {lateral}");
            }
            if distance < -width * 0.5 + 0.01
                || distance > (to - from).length() + width * 0.5 - 0.01
            {
                assert_eq!(color[3], 0.0, "hard end cap at {distance}");
            }
            if color[3] == 1.0 {
                opaque += 1;
                assert!(lateral <= width * 0.5 + ROAD_EDGE_WOBBLE + ROAD_EDGE_CHIP + 0.01);
            }
        }
        assert!(opaque > 20, "road has no solid core");
        assert!(up_facing(&surface));
    }

    #[test]
    fn clearing_rim_is_transparent_and_closed() {
        let mut surface = SurfaceBuffers::default();
        append_clearing(
            &mut surface,
            Vec3::ZERO,
            Vec3::new(5.0, 0.0, 5.0),
            Vec2::new(12.0, 9.0),
            5,
        );
        assert_eq!(surface.positions.len(), 1 + 2 * CLEARING_SEGMENTS);
        // hub fan + ring quads, including the closing segment.
        assert_eq!(surface.indices.len(), CLEARING_SEGMENTS * 9);
        let rim_alpha: Vec<f32> = surface
            .colors
            .iter()
            .skip(2)
            .step_by(2)
            .map(|c| c[3])
            .collect();
        assert!(rim_alpha.iter().all(|alpha| *alpha == 0.0));
        assert!(up_facing(&surface));
    }
}
