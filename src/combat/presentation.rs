//! Sparse contact artwork, emitted by resolved damage rather than input or casts.
use super::SkillCast;
use crate::{
    GameState,
    assets::GameAssets,
    class::{HeroClass, Rune, SelectedClass, SkillSlot},
    feedback::{ContactSound, ContactSoundEvent},
    not_paused,
    player::TalentBranch,
};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

#[derive(Message, Clone, Copy, Debug)]
pub(super) struct ContactFeedback {
    pub target: Entity,
    #[cfg_attr(not(feature = "dev_tools"), allow(dead_code))]
    pub cast: u32,
    pub position: Vec3,
    pub source: Vec3,
    pub skill: Option<SkillCast>,
    pub strength: f32,
    pub critical: bool,
    pub killed: bool,
    pub material: ContactSound,
}

#[derive(Message, Clone, Copy, Debug)]
pub(super) struct TalentProcFeedback {
    pub position: Vec3,
    pub branch: TalentBranch,
    pub amount: f32,
}

pub(super) struct CombatPresentationPlugin;
impl Plugin for CombatPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ContactFeedback>()
            .add_message::<TalentProcFeedback>()
            .add_systems(
                Update,
                (
                    show_contacts,
                    show_talent_procs,
                    animate_contacts,
                    animate_impact_sparks,
                    tick_enemy_mesh_hit_flashes,
                )
                    .chain()
                    .after(super::CombatSet::PlayerDamage)
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), clear_contacts);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContactArt {
    Steel,
    Expose,
    Blood,
    Cleanse,
    Reap,
    Ember,
    Frost,
    Vigor,
    Focus,
}

impl ContactArt {
    fn for_skill(skill: Option<SkillCast>) -> Self {
        match skill.map(|cast| cast.rune) {
            None => Self::Steel,
            Some(Rune::Expose) => Self::Expose,
            Some(Rune::Hemorrhage) => Self::Blood,
            Some(Rune::Cleanse) => Self::Cleanse,
            Some(Rune::Reap) => Self::Reap,
            Some(Rune::Ember) => Self::Ember,
            Some(Rune::Frost) => Self::Frost,
        }
    }
    fn model(self, assets: &GameAssets) -> Handle<WorldAsset> {
        match self {
            Self::Steel => &assets.contact_steel,
            Self::Expose => &assets.contact_expose,
            Self::Blood => &assets.contact_blood,
            Self::Cleanse => &assets.contact_cleanse,
            Self::Reap => &assets.contact_reap,
            Self::Ember => &assets.contact_ember,
            Self::Frost => &assets.contact_frost,
            Self::Vigor => &assets.proc_vigor,
            Self::Focus => &assets.proc_focus,
        }
        .clone()
    }
    fn sound(self) -> ContactSound {
        match self {
            Self::Steel => ContactSound::Steel,
            Self::Expose | Self::Vigor => ContactSound::Expose,
            Self::Blood => ContactSound::Blood,
            Self::Cleanse => ContactSound::Cleanse,
            Self::Reap | Self::Focus => ContactSound::Reap,
            Self::Ember => ContactSound::Ember,
            Self::Frost => ContactSound::Frost,
        }
    }
    fn lifetime(self) -> f32 {
        match self {
            Self::Steel => 0.18,
            Self::Expose => 0.24,
            Self::Blood => 0.28,
            Self::Cleanse | Self::Vigor => 0.32,
            Self::Reap | Self::Focus => 0.38,
            Self::Ember => 0.26,
            Self::Frost => 0.36,
        }
    }
    fn ground(self) -> bool {
        matches!(
            self,
            Self::Cleanse | Self::Frost | Self::Vigor | Self::Focus
        )
    }
}

#[derive(Component)]
struct ContactParticle {
    target: Option<Entity>,
    art: ContactArt,
    origin: Vec3,
    destination: Vec3,
    age: f32,
    size: f32,
}

fn contact_size(strength: f32) -> f32 {
    (0.82 + strength.clamp(0.0, 2.0) * 0.22).clamp(0.82, 1.26)
}

fn particle_pose(particle: &ContactParticle, progress: f32) -> (Vec3, f32) {
    let t = progress.clamp(0.0, 1.0);
    // Opaque geometry shrinks to nothing; no translucent material or stacked flashes.
    let fade = if t > 0.64 { (1.0 - t) / 0.36 } else { 1.0 };
    let travel = match particle.art {
        ContactArt::Reap => t * t,
        ContactArt::Steel | ContactArt::Blood | ContactArt::Ember => t,
        _ => 0.0,
    };
    let spread = match particle.art {
        ContactArt::Cleanse | ContactArt::Focus => 1.1 - t * 0.72,
        ContactArt::Frost => 0.55 + t * 0.7,
        ContactArt::Expose | ContactArt::Vigor => 0.8 + t * 0.30,
        _ => 0.7 + t * 0.65,
    };
    (
        particle.origin.lerp(particle.destination, travel),
        particle.size * spread * fade,
    )
}

fn spawn_particle(
    commands: &mut Commands,
    assets: &GameAssets,
    particle: ContactParticle,
    yaw: f32,
) {
    let (position, scale) = particle_pose(&particle, 0.0);
    commands.spawn((
        WorldAssetRoot(particle.art.model(assets)),
        Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(scale)),
        particle,
        Name::new("Resolved contact artwork"),
    ));
}

pub(super) fn basic_material_sound(enemy: Option<&str>) -> ContactSound {
    let id = enemy.unwrap_or("");
    if ["skeleton", "bone", "thrall"]
        .iter()
        .any(|part| id.contains(part))
    {
        ContactSound::Bone
    } else if [
        "warden", "keeper", "guard", "marksman", "knight", "sentinel",
    ]
    .iter()
    .any(|part| id.contains(part))
    {
        ContactSound::Steel
    } else {
        ContactSound::Blood
    }
}

fn contact_art(event: &ContactFeedback, class: HeroClass) -> ContactArt {
    if event.skill.is_none() && event.cast != 0 {
        match class {
            HeroClass::Embercaller => return ContactArt::Ember,
            HeroClass::Soulbinder => return ContactArt::Reap,
            _ => {}
        }
    }
    ContactArt::for_skill(event.skill)
}

#[derive(Component)]
struct ImpactSpark {
    velocity: Vec3,
    gravity: f32,
    age: f32,
    lifetime: f32,
    base_scale: Vec3,
    /// When true, scales radially like a shockwave ring rather than stretching along velocity.
    ring_expand: bool,
}

fn impact_accent_color(art: ContactArt, critical: bool) -> Color {
    if critical {
        return Color::srgba(1.0, 0.88, 0.36, 0.96);
    }
    match art {
        ContactArt::Steel => Color::srgba(1.0, 0.84, 0.56, 0.94),
        ContactArt::Blood => Color::srgba(1.0, 0.22, 0.16, 0.94),
        ContactArt::Ember => Color::srgba(1.0, 0.44, 0.10, 0.96),
        ContactArt::Frost => Color::srgba(0.32, 0.88, 1.0, 0.94),
        ContactArt::Reap => Color::srgba(0.24, 0.96, 0.82, 0.94),
        ContactArt::Expose => Color::srgba(1.0, 0.72, 0.20, 0.94),
        ContactArt::Cleanse | ContactArt::Vigor => Color::srgba(1.0, 0.96, 0.76, 0.92),
        ContactArt::Focus => Color::srgba(0.68, 0.50, 1.0, 0.92),
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_aaa_impact_burst(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
    direction: Vec3,
    art: ContactArt,
    strength: f32,
    critical: bool,
    killed: bool,
) {
    let dir = if direction.length_squared() > 1e-4 {
        direction.normalize()
    } else {
        Vec3::Z
    };
    let right = Vec3::new(dir.z, 0.0, -dir.x).normalize_or_zero();
    let color = impact_accent_color(art, critical);
    let lin = LinearRgba::from(color);
    let boost = if critical || killed { 5.2 } else { 3.6 };

    // 1. Crisp 85ms hot-white/elemental hit-flash core right at the impact point
    let core_color = Color::linear_rgba(
        (lin.red * 0.45 + 0.55) * boost,
        (lin.green * 0.45 + 0.55) * boost,
        (lin.blue * 0.45 + 0.55) * boost,
        0.96,
    );
    let core_mat = materials.add(StandardMaterial {
        base_color: core_color,
        emissive: LinearRgba::from(core_color),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });
    let flash_radius = (0.24 + strength.clamp(0.5, 2.0) * 0.14) * if critical { 1.35 } else { 1.0 };
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(flash_radius).mesh().ico(1).unwrap())),
        MeshMaterial3d(core_mat.clone()),
        Transform::from_translation(origin),
        NotShadowCaster,
        NotShadowReceiver,
        ImpactSpark {
            velocity: dir * 0.8,
            gravity: 0.0,
            age: 0.0,
            lifetime: if critical { 0.11 } else { 0.085 },
            base_scale: Vec3::new(1.35, 0.95, 1.35),
            ring_expand: false,
        },
        Name::new("AAA Contact Hit Flash"),
    ));

    // 2. Directional razor hit-sparks spraying along the blade exit vector
    let spark_mat = materials.add(StandardMaterial {
        base_color: color,
        emissive: lin * boost,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });
    let spark_mesh = meshes.add(Cone::new(0.032, 0.42));
    let spark_count: usize = if killed {
        8
    } else if critical {
        6
    } else {
        4
    };
    for i in 0..spark_count {
        let frac = (i as f32 / (spark_count.max(1) as f32)) * 2.0 - 1.0;
        let up_bias = 0.24 + ((i * 37) % 10) as f32 * 0.055;
        let spread_dir = (dir * 1.15 + right * (frac * 0.82) + Vec3::Y * up_bias).normalize();
        let speed = (4.6 + ((i * 53) % 7) as f32 * 0.95) * strength.clamp(0.8, 1.6);
        let len_scale = if critical || killed { 1.45 } else { 1.05 };
        commands.spawn((
            Mesh3d(spark_mesh.clone()),
            MeshMaterial3d(spark_mat.clone()),
            Transform::from_translation(origin + spread_dir * 0.12)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, spread_dir))
                .with_scale(Vec3::new(0.9, len_scale, 0.9)),
            NotShadowCaster,
            NotShadowReceiver,
            ImpactSpark {
                velocity: spread_dir * speed,
                gravity: -14.5,
                age: 0.0,
                lifetime: 0.17 + (i % 3) as f32 * 0.045,
                base_scale: Vec3::new(0.9, len_scale, 0.9),
                ring_expand: false,
            },
            Name::new("AAA Directional Hit Spark"),
        ));
    }

    // 3. Critical cross-slash glint planes
    if critical {
        let glint_mesh = meshes.add(Cuboid::new(0.055, 1.35, 0.055));
        for tilt in [-0.72_f32, 0.72] {
            let rot = Quat::from_rotation_y(dir.x.atan2(dir.z)) * Quat::from_rotation_z(tilt);
            commands.spawn((
                Mesh3d(glint_mesh.clone()),
                MeshMaterial3d(core_mat.clone()),
                Transform::from_translation(origin).with_rotation(rot),
                NotShadowCaster,
                NotShadowReceiver,
                ImpactSpark {
                    velocity: Vec3::ZERO,
                    gravity: 0.0,
                    age: 0.0,
                    lifetime: 0.14,
                    base_scale: Vec3::new(1.0, 1.35, 1.0),
                    ring_expand: false,
                },
                Name::new("AAA Crit Cross Glint"),
            ));
        }
    }

    // 4. Finishing-blow kill shockwave ring
    if killed {
        commands.spawn((
            Mesh3d(meshes.add(Torus::new(0.32, 0.44))),
            MeshMaterial3d(spark_mat),
            Transform::from_translation(origin.with_y(0.14)).with_scale(Vec3::new(0.5, 0.4, 0.5)),
            NotShadowCaster,
            NotShadowReceiver,
            ImpactSpark {
                velocity: Vec3::ZERO,
                gravity: 0.0,
                age: 0.0,
                lifetime: 0.34,
                base_scale: Vec3::new(2.6, 0.12, 2.6),
                ring_expand: true,
            },
            Name::new("AAA Kill Finisher Ring"),
        ));
    }
}

/// Temporary 75ms opaque emissive hit-flash applied directly to a struck enemy's
/// descendant mesh materials (skipping health-bar/UI decorations) and restored cleanly.
#[derive(Component)]
struct EnemyMeshHitFlash {
    timer: Timer,
    original: Handle<StandardMaterial>,
}

fn queue_enemy_mesh_hit_flash(
    commands: &mut Commands,
    target: Entity,
    art: ContactArt,
    critical: bool,
) {
    let color = impact_accent_color(art, critical);
    commands.queue(move |world: &mut World| {
        if world.get_entity(target).is_err()
            || world.get_resource::<Assets<StandardMaterial>>().is_none()
        {
            return;
        }
        let lin = LinearRgba::from(color);
        let boost = if critical { 4.8 } else { 3.2 };
        let flash_mat = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.96, 0.88),
                emissive: LinearRgba::new(
                    (lin.red * 0.65 + 0.35) * boost,
                    (lin.green * 0.65 + 0.35) * boost,
                    (lin.blue * 0.65 + 0.35) * boost,
                    1.0,
                ),
                alpha_mode: AlphaMode::Opaque,
                perceptual_roughness: 0.35,
                ..default()
            });

        // Walk up to 4 levels of descendants, skipping DeathDecoration (health bars/focus rings)
        let mut stack = vec![(target, 0_u8)];
        let mut mesh_entities = Vec::new();
        while let Some((node, depth)) = stack.pop() {
            if world
                .get::<crate::enemy_death::DeathDecoration>(node)
                .is_some()
            {
                continue;
            }
            if world
                .get::<MeshMaterial3d<StandardMaterial>>(node)
                .is_some()
            {
                mesh_entities.push(node);
            }
            if depth < 4
                && let Some(children) = world.get::<Children>(node)
            {
                for child in children.iter() {
                    stack.push((child, depth + 1));
                }
            }
        }
        let duration = if critical { 0.09 } else { 0.072 };
        for mesh_entity in mesh_entities {
            // Preserve the true original material if already mid-flash!
            let original = if let Some(existing) = world.get::<EnemyMeshHitFlash>(mesh_entity) {
                existing.original.clone()
            } else if let Some(mat) = world.get::<MeshMaterial3d<StandardMaterial>>(mesh_entity) {
                mat.0.clone()
            } else {
                continue;
            };
            if let Ok(mut em) = world.get_entity_mut(mesh_entity) {
                em.insert((
                    MeshMaterial3d(flash_mat.clone()),
                    EnemyMeshHitFlash {
                        timer: Timer::from_seconds(duration, TimerMode::Once),
                        original,
                    },
                ));
            }
        }
    });
}

fn tick_enemy_mesh_hit_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(
        Entity,
        &mut EnemyMeshHitFlash,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    for (entity, mut flash, mut mat) in &mut flashes {
        flash.timer.tick(time.delta());
        if flash.timer.is_finished() {
            mat.0 = flash.original.clone();
            commands.entity(entity).try_remove::<EnemyMeshHitFlash>();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn show_contacts(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut events: MessageReader<ContactFeedback>,
    targets: Query<&Transform, Without<ContactParticle>>,
    selected: Res<SelectedClass>,
    previous: Query<(Entity, &ContactParticle)>,
    mut sounds: MessageWriter<ContactSoundEvent>,
) {
    // A cleave may report the same victim more than once; retain a single layer.
    let mut contacts: Vec<ContactFeedback> = Vec::new();
    for event in events.read().filter(|event| event.strength > 0.0) {
        if let Some(existing) = contacts
            .iter_mut()
            .find(|entry| entry.target == event.target)
        {
            if event.strength >= existing.strength {
                *existing = *event;
            }
        } else {
            contacts.push(*event);
        }
    }
    let strongest = contacts
        .iter()
        .max_by(|a, b| a.strength.total_cmp(&b.strength));
    if let Some(event) = strongest {
        sounds.write(ContactSoundEvent {
            kind: if event.skill.is_none()
                && !matches!(selected.0, HeroClass::Embercaller | HeroClass::Soulbinder)
            {
                event.material
            } else {
                contact_art(event, selected.0).sound()
            },
            strength: event.strength.max(if event.killed {
                1.7
            } else if event.critical {
                1.4
            } else {
                0.0
            }),
        });
    }
    for event in contacts {
        for (entity, particle) in &previous {
            if particle.target == Some(event.target) {
                commands.entity(entity).despawn();
            }
        }
        if selected.0 == HeroClass::Warden
            && event
                .skill
                .is_some_and(|cast| cast.slot == SkillSlot::Finisher)
        {
            commands.write_message(super::pixelfx::PixelFx::at(
                super::pixelfx::Kind::HeavyImpact,
                event.position,
                event.position - event.source,
                1.35 * event.strength.clamp(0.85, 1.4),
            ));
            continue;
        }
        let art = contact_art(&event, selected.0);
        let direction = (event.position - event.source)
            .with_y(0.0)
            .normalize_or_zero();
        let yaw = direction.x.atan2(direction.z);
        let mut origin = event.position;
        if art.ground() {
            origin.y = targets
                .get(event.target)
                .map_or(event.source.y, |target| target.translation.y)
                + 0.055;
        }
        let destination = match art {
            ContactArt::Reap => event.source + Vec3::Y * 0.65,
            ContactArt::Blood => origin + direction * 0.18 - Vec3::Y * 0.28,
            ContactArt::Steel => origin + direction * 0.22 + Vec3::Y * 0.08,
            ContactArt::Ember => origin + direction * 0.10 + Vec3::Y * 0.28,
            _ => origin,
        };
        spawn_particle(
            &mut commands,
            &assets,
            ContactParticle {
                target: Some(event.target),
                art,
                origin,
                destination,
                age: 0.0,
                size: contact_size(event.strength)
                    * if event.killed {
                        1.14
                    } else if event.critical {
                        1.08
                    } else {
                        1.0
                    },
            },
            yaw,
        );
        if let (Some(mesh_assets), Some(mat_assets)) =
            (meshes.as_deref_mut(), materials.as_deref_mut())
        {
            spawn_aaa_impact_burst(
                &mut commands,
                mesh_assets,
                mat_assets,
                event.position,
                direction,
                art,
                event.strength,
                event.critical,
                event.killed,
            );
            queue_enemy_mesh_hit_flash(&mut commands, event.target, art, event.critical);
        }
    }
}

fn show_talent_procs(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut events: MessageReader<TalentProcFeedback>,
    previous: Query<(Entity, &ContactParticle)>,
) {
    // Only one small effect near the player, even when several victims grant a reward.
    let Some(event) = events
        .read()
        .filter(|event| event.amount > 0.0 && event.branch != TalentBranch::Wrath)
        .max_by(|a, b| a.amount.total_cmp(&b.amount))
        .copied()
    else {
        return;
    };
    for (entity, particle) in &previous {
        if particle.target.is_none() {
            commands.entity(entity).despawn();
        }
    }
    let art = if event.branch == TalentBranch::Vigor {
        ContactArt::Vigor
    } else {
        ContactArt::Focus
    };
    let origin = event.position + Vec3::Y * 0.07;
    spawn_particle(
        &mut commands,
        &assets,
        ContactParticle {
            target: None,
            art,
            origin,
            destination: origin,
            age: 0.0,
            size: 0.9,
        },
        0.0,
    );
}

fn animate_contacts(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut ContactParticle, &mut Transform)>,
) {
    for (entity, mut particle, mut transform) in &mut particles {
        particle.age += time.delta_secs();
        let progress = particle.age / particle.art.lifetime();
        if progress >= 1.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let (position, scale) = particle_pose(&particle, progress);
        transform.translation = position;
        transform.scale = Vec3::splat(scale);
    }
}

fn animate_impact_sparks(
    mut commands: Commands,
    time: Res<Time>,
    mut sparks: Query<(Entity, &mut ImpactSpark, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut spark, mut transform) in &mut sparks {
        spark.age += dt;
        let t = (spark.age / spark.lifetime.max(0.01)).clamp(0.0, 1.0);
        if t >= 1.0 {
            commands.entity(entity).despawn();
            continue;
        }
        if spark.ring_expand {
            let eased = 1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t);
            let r = 0.45 + eased * spark.base_scale.x;
            let y = (1.0 - t).max(0.02) * 0.45;
            transform.scale = Vec3::new(r, y, r);
        } else {
            spark.velocity.y += spark.gravity * dt;
            transform.translation += spark.velocity * dt;
            if spark.velocity.length_squared() > 0.25 {
                let dir = spark.velocity.normalize();
                transform.rotation = Quat::from_rotation_arc(Vec3::Y, dir);
            }
            // Fast punch-in, crisp rapid decay
            let shrink = (1.0 - t * t).max(0.02);
            transform.scale = Vec3::new(
                spark.base_scale.x * shrink,
                spark.base_scale.y * (0.4 + shrink * 0.85),
                spark.base_scale.z * shrink,
            );
        }
    }
}

fn clear_contacts(
    mut commands: Commands,
    particles: Query<Entity, With<ContactParticle>>,
    sparks: Query<Entity, With<ImpactSpark>>,
    mut flashes: Query<(
        Entity,
        &EnemyMeshHitFlash,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    for entity in &particles {
        commands.entity(entity).despawn();
    }
    for entity in &sparks {
        commands.entity(entity).despawn();
    }
    for (entity, flash, mut mat) in &mut flashes {
        mat.0 = flash.original.clone();
        commands.entity(entity).try_remove::<EnemyMeshHitFlash>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn six_runes_have_distinct_contact_art_and_never_outlive_four_tenths() {
        let runes = [
            Rune::Cleanse,
            Rune::Reap,
            Rune::Expose,
            Rune::Hemorrhage,
            Rune::Ember,
            Rune::Frost,
        ];
        let art: Vec<_> = runes
            .into_iter()
            .map(|rune| {
                ContactArt::for_skill(Some(SkillCast {
                    slot: crate::class::SkillSlot::Finisher,
                    rune,
                }))
            })
            .collect();
        for (index, entry) in art.iter().enumerate() {
            assert!(!art[..index].contains(entry));
            assert!(entry.lifetime() <= 0.4);
        }
        assert!(contact_size(1000.0) <= 1.26);
    }
    #[test]
    fn reap_returns_to_source_and_all_art_disappears_without_alpha_blending() {
        for art in [
            ContactArt::Steel,
            ContactArt::Expose,
            ContactArt::Blood,
            ContactArt::Cleanse,
            ContactArt::Reap,
            ContactArt::Ember,
            ContactArt::Frost,
            ContactArt::Vigor,
            ContactArt::Focus,
        ] {
            let particle = ContactParticle {
                target: None,
                art,
                origin: Vec3::X * 3.0,
                destination: Vec3::ZERO,
                age: 0.0,
                size: 1.0,
            };
            let (end, scale) = particle_pose(&particle, 1.0);
            assert_eq!(scale, 0.0);
            if art == ContactArt::Reap {
                assert_eq!(end, Vec3::ZERO);
            }
        }
    }

    #[test]
    fn enemy_mesh_hit_flash_swaps_mesh_material_and_restores_original() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, tick_enemy_mesh_hit_flashes);

        let original_mat = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let bar_mat = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let enemy = app.world_mut().spawn(Transform::default()).id();
        let body_mesh = app
            .world_mut()
            .spawn((MeshMaterial3d(original_mat.clone()), ChildOf(enemy)))
            .id();
        let health_bar = app
            .world_mut()
            .spawn((
                MeshMaterial3d(bar_mat.clone()),
                crate::enemy_death::DeathDecoration,
                ChildOf(enemy),
            ))
            .id();

        {
            let mut cmds = app.world_mut().commands();
            queue_enemy_mesh_hit_flash(&mut cmds, enemy, ContactArt::Steel, false);
        }
        app.world_mut().flush();
        app.update();

        // Body mesh flashed to new emissive handle; health bar decoration untouched!
        assert_ne!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(body_mesh)
                .unwrap()
                .0,
            original_mat
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(health_bar)
                .unwrap()
                .0,
            bar_mat
        );

        // After 80ms the body mesh restores its exact original handle cleanly.
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(std::time::Duration::from_secs_f32(0.08));
        app.update();
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(body_mesh)
                .unwrap()
                .0,
            original_mat
        );
        assert!(app.world().get::<EnemyMeshHitFlash>(body_mesh).is_none());
    }
}
