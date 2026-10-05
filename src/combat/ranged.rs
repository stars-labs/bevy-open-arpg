//! Missiles and telegraphed ground bursts: strikes that land after the hero has moved on.
use super::*;
use crate::class::ProjectileArt;

/// Height missiles fly at, so they cross enemy chests rather than feet.
const PROJECTILE_HEIGHT: f32 = 0.95;
/// Added to a missile's half-width so it connects with a body, not a point.
const TARGET_BODY_RADIUS: f32 = 0.38;
/// A sigil draws on the ground, just above the burst artwork's own lift.
const SIGIL_HEIGHT: f32 = 0.03;

#[derive(Component, Debug)]
pub(super) struct PlayerProjectile {
    velocity: Vec3,
    remaining: f32,
    width: f32,
    pierce: bool,
    blast_radius: f32,
    /// Where it was loosed: recoil direction and Long Shot distance both measure from here.
    source: Vec3,
    profile: StrikeProfile,
    struck: Vec<Entity>,
}

#[derive(Component, Debug)]
pub(super) struct PlayerGroundBlast {
    timer: Timer,
    radius: f32,
    source: Vec3,
    profile: StrikeProfile,
}

/// Evenly fans `count` directions across `spread_deg`, centred on `facing`.
pub(super) fn projectile_directions(facing: Vec3, count: u32, spread_deg: f32) -> Vec<Vec3> {
    let count = count.max(1);
    (0..count)
        .map(|index| {
            let offset = if count == 1 {
                0.0
            } else {
                (index as f32 / (count - 1) as f32 - 0.5) * spread_deg.to_radians()
            };
            Quat::from_rotation_y(offset) * facing
        })
        .collect()
}

fn projectile_scene(assets: &GameAssets, art: ProjectileArt) -> Handle<WorldAsset> {
    match art {
        ProjectileArt::BoneArrow => assets.bone_arrow.clone(),
        ProjectileArt::EmberBolt => assets.ember_bolt.clone(),
        ProjectileArt::AshLance => assets.ash_lance.clone(),
        ProjectileArt::SunderWave => assets.sunder_wave.clone(),
    }
}

fn projectile_height(art: ProjectileArt) -> f32 {
    if art == ProjectileArt::SunderWave {
        0.42
    } else {
        PROJECTILE_HEIGHT
    }
}

pub(super) fn launch_projectiles(
    commands: &mut Commands,
    assets: &GameAssets,
    origin: Vec3,
    facing: Vec3,
    shape: StrikeShape,
    profile: StrikeProfile,
) {
    let StrikeShape::Projectiles {
        art,
        count,
        spread_deg,
        speed,
        range,
        width,
        pierce,
        blast_radius,
    } = shape
    else {
        return;
    };
    let forward = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    for direction in projectile_directions(forward, count, spread_deg) {
        let start = origin + direction * 0.6 + Vec3::Y * projectile_height(art);
        let missile = commands
            .spawn((
                WorldAssetRoot(projectile_scene(assets, art)),
                Transform::from_translation(start)
                    .with_rotation(Quat::from_rotation_y(direction.x.atan2(direction.z))),
                PlayerProjectile {
                    velocity: direction * speed,
                    remaining: range,
                    width,
                    pierce,
                    blast_radius,
                    source: origin,
                    profile,
                    struck: Vec::new(),
                },
                Name::new("Player Projectile"),
            ))
            .id();
        if let Some(cast) = profile.skill {
            commands.write_message(skill_vfx::ProjectileVisual {
                entity: missile,
                rune: cast.rune,
                width,
            });
        }
    }
}

pub(super) fn mark_ground_blast(
    commands: &mut Commands,
    origin: Vec3,
    target: Vec3,
    radius: f32,
    delay: f32,
    profile: StrikeProfile,
) {
    if let Some(cast) = profile.skill {
        commands.write_message(skill_vfx::SkillVisual {
            origin: target,
            facing: (target - origin).normalize_or_zero(),
            shape: StrikeShape::GroundBlast {
                radius,
                delay,
                cast_range: origin.distance(target),
            },
            cast,
            strength: profile.impact_scale,
        });
    }
    commands.spawn((
        Transform::from_translation(Vec3::new(target.x, SIGIL_HEIGHT, target.z))
            .with_scale(Vec3::splat(radius * 1.12)),
        PlayerGroundBlast {
            timer: Timer::from_seconds(delay.max(0.05), TimerMode::Once),
            radius,
            source: origin,
            profile,
        },
        Name::new("Player Ground Blast"),
    ));
}

/// Fraction of `step` at which a missile of `reach` touches `target`, ignoring height.
pub(super) fn sweep_hit(start: Vec3, step: Vec3, target: Vec3, reach: f32) -> Option<f32> {
    let to_target = Vec2::new(target.x - start.x, target.z - start.z);
    let segment = Vec2::new(step.x, step.z);
    let length_squared = segment.length_squared();
    let t = if length_squared <= f32::EPSILON {
        0.0
    } else {
        (to_target.dot(segment) / length_squared).clamp(0.0, 1.0)
    };
    ((to_target - segment * t).length() <= reach).then_some(t)
}

type ProjectileQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Transform,
        &'static mut PlayerProjectile,
    ),
    (Without<Enemy>, Without<Player>, Without<Breakable>),
>;
type GroundBlastQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Transform, &'static mut PlayerGroundBlast),
    (Without<Enemy>, Without<Player>, Without<Breakable>),
>;

pub(super) fn fly_player_projectiles(
    time: Res<Time>,
    catalog: Res<ClassCatalog>,
    selected: Res<SelectedClass>,
    mut world: StrikeWorld,
    mut heroes: StrikeHeroQuery,
    mut projectiles: ProjectileQuery,
) {
    let resource = catalog.get(selected.0).resource.kind;
    for (entity, mut transform, mut projectile) in &mut projectiles {
        let travel = (projectile.velocity.length() * time.delta_secs()).min(projectile.remaining);
        let step = projectile.velocity.normalize_or_zero() * travel;
        let start = transform.translation;
        let reach = projectile.width + TARGET_BODY_RADIUS;
        let mut touched: Vec<(f32, Entity, Vec3)> = world
            .enemies
            .iter()
            .filter(|(target, _, _, health, ..)| {
                health.current > 0.0 && !projectile.struck.contains(target)
            })
            .filter_map(|(target, target_transform, ..)| {
                sweep_hit(start, step, target_transform.translation, reach)
                    .map(|t| (t, target, target_transform.translation))
            })
            .chain(
                world
                    .breakables
                    .iter()
                    .filter(|(target, _, health)| {
                        health.current > 0.0 && !projectile.struck.contains(target)
                    })
                    .filter_map(|(target, target_transform, _)| {
                        sweep_hit(start, step, target_transform.translation, reach)
                            .map(|t| (t, target, target_transform.translation))
                    }),
            )
            .collect();
        touched.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut spent = false;
        for (_, target, position) in touched {
            let area = if projectile.blast_radius > 0.0 {
                if let Some(cast) = projectile.profile.skill {
                    world.commands.write_message(skill_vfx::SkillVisual {
                        origin: position,
                        facing: projectile.velocity.normalize_or_zero(),
                        shape: StrikeShape::Radial {
                            radius: projectile.blast_radius,
                        },
                        cast,
                        strength: projectile.profile.impact_scale,
                    });
                } else {
                    let vfx = combat_vfx_assets(&world.assets);
                    spawn_burst_vfx(
                        &mut world.commands,
                        &vfx,
                        position,
                        None,
                        projectile.blast_radius,
                    );
                }
                HitArea::Radial {
                    center: position,
                    radius: projectile.blast_radius,
                }
            } else {
                HitArea::Target(target)
            };
            let source = projectile.source;
            let profile = projectile.profile;
            with_hero(&mut heroes, |hero| {
                world.land(hero, resource, area, source, profile);
            });
            projectile.struck.push(target);
            if !projectile.pierce || projectile.blast_radius > 0.0 {
                spent = true;
                break;
            }
        }
        projectile.remaining -= travel;
        transform.translation += step;
        if spent || projectile.remaining <= 0.0 {
            world.commands.entity(entity).despawn();
        }
    }
}

pub(super) fn burst_ground_blasts(
    time: Res<Time>,
    catalog: Res<ClassCatalog>,
    selected: Res<SelectedClass>,
    mut world: StrikeWorld,
    mut heroes: StrikeHeroQuery,
    mut blasts: GroundBlastQuery,
) {
    let resource = catalog.get(selected.0).resource.kind;
    for (entity, transform, mut blast) in &mut blasts {
        blast.timer.tick(time.delta());
        if !blast.timer.is_finished() {
            continue;
        }
        let center = Vec3::new(transform.translation.x, 0.0, transform.translation.z);
        if let Some(cast) = blast.profile.skill {
            world.commands.write_message(skill_vfx::SkillVisual {
                origin: center,
                facing: (center - blast.source).normalize_or_zero(),
                shape: StrikeShape::Radial {
                    radius: blast.radius,
                },
                cast,
                strength: blast.profile.impact_scale,
            });
        }
        let (radius, source, profile) = (blast.radius, blast.source, blast.profile);
        with_hero(&mut heroes, |hero| {
            world.land(
                hero,
                resource,
                HitArea::Radial { center, radius },
                source,
                profile,
            );
        });
        world.commands.entity(entity).despawn();
    }
}

pub(super) fn despawn_ranged_strikes(
    mut commands: Commands,
    projectiles: Query<Entity, With<PlayerProjectile>>,
    blasts: Query<Entity, With<PlayerGroundBlast>>,
) {
    for entity in projectiles.iter().chain(blasts.iter()) {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fans_spread_symmetrically_around_the_facing() {
        let directions = projectile_directions(Vec3::Z, 3, 60.0);
        assert_eq!(directions.len(), 3);
        assert!((directions[1] - Vec3::Z).length() < 1e-5);
        assert!((directions[0].x + directions[2].x).abs() < 1e-5);
        assert!((directions[0].angle_between(directions[2]).to_degrees() - 60.0).abs() < 0.01);
        assert_eq!(projectile_directions(Vec3::Z, 1, 90.0), vec![Vec3::Z]);
    }

    #[test]
    fn sweep_touches_bodies_beside_the_path_and_ignores_height() {
        let start = Vec3::new(0.0, 0.95, 0.0);
        let step = Vec3::Z * 2.0;
        assert!(sweep_hit(start, step, Vec3::new(0.3, 0.0, 1.0), 0.78).is_some());
        assert_eq!(
            sweep_hit(start, step, Vec3::new(0.0, 0.0, 1.0), 0.78),
            Some(0.5)
        );
        assert!(sweep_hit(start, step, Vec3::new(1.0, 0.0, 1.0), 0.78).is_none());
        assert!(sweep_hit(start, step, Vec3::new(0.0, 0.0, 3.5), 0.78).is_none());
    }
}
