//! Sparse contact artwork, emitted by resolved damage rather than input or casts.
use super::SkillCast;
use crate::{
    GameState,
    assets::GameAssets,
    class::Rune,
    feedback::{ContactSound, ContactSoundEvent},
    not_paused,
    player::TalentBranch,
};
use bevy::prelude::*;

#[derive(Message, Clone, Copy, Debug)]
pub(super) struct ContactFeedback {
    pub target: Entity,
    pub position: Vec3,
    pub source: Vec3,
    pub skill: Option<SkillCast>,
    pub strength: f32,
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
                (show_contacts, show_talent_procs, animate_contacts)
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

fn show_contacts(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut events: MessageReader<ContactFeedback>,
    targets: Query<&Transform, Without<ContactParticle>>,
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
            kind: ContactArt::for_skill(event.skill).sound(),
            strength: event.strength,
        });
    }
    for event in contacts {
        for (entity, particle) in &previous {
            if particle.target == Some(event.target) {
                commands.entity(entity).despawn();
            }
        }
        let art = ContactArt::for_skill(event.skill);
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
                size: contact_size(event.strength),
            },
            yaw,
        );
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

fn clear_contacts(mut commands: Commands, particles: Query<Entity, With<ContactParticle>>) {
    for entity in &particles {
        commands.entity(entity).despawn();
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
}
