//! Shared combat feel data; pure reads keep native and browser behavior identical.
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
pub(crate) struct FeelTuning {
    pub basic_buffer_secs: f32,
    pub skill_buffer_secs: f32,
    pub evade_buffer_secs: f32,
    pub chain_recovery_fraction: f32,
    pub cancel_recovery_fraction: f32,
    pub basic_contact_fraction: f32,
    pub heavy_contact_fraction: f32,
    pub evade_active_secs: f32,
    pub assist_extra_reach: f32,
    pub assist_aim_cosine: f32,
    pub contact_hold_fraction: f32,
    pub contact_drag_fraction: f32,
    pub camera_max_intensity: f32,
    pub camera_max_duration: f32,
    pub audio_pitch_variation: f32,
    pub audio_gain_min: f32,
    pub basic_pause_secs: f32,
    pub critical_pause_secs: f32,
    pub heavy_pause_secs: f32,
    pub boss_break_pause_secs: f32,
}

pub(crate) fn tuning() -> &'static FeelTuning {
    static TUNING: LazyLock<FeelTuning> = LazyLock::new(|| {
        ron::from_str(include_str!("../../assets/data/combat_feel.ron"))
            .expect("valid combat feel tuning")
    });
    &TUNING
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_buffered_press_can_survive_release_but_cannot_fire_after_expiry() {
        use crate::class::SkillSlot;
        use crate::combat::SkillInputBuffer;
        let mut buffer = SkillInputBuffer {
            secs: [0.0; 3],
            basic_secs: tuning().basic_buffer_secs,
        };
        buffer.capture(SkillSlot::Finisher);
        buffer.tick(0.08);
        assert!(buffer.basic_secs > 0.0 && buffer.requested(SkillSlot::Finisher));
        buffer.tick(0.20);
        assert_eq!(buffer.basic_secs, 0.0);
        assert!(!buffer.requested(SkillSlot::Finisher));
    }

    #[test]
    fn cursor_and_weapon_edge_forgiveness_agree_without_backwards_hits() {
        use crate::combat::{arc_target_hit, assisted_cursor_target, attack_target_in_range};
        use bevy::prelude::*;
        let enemy = crate::combat::tests::test_enemy(vec![]);
        let position = Vec3::Z * 2.3;
        assert!(attack_target_in_range(Vec3::ZERO, position, 2.2, &enemy));
        assert!(arc_target_hit(Vec3::ZERO, Vec3::Z, position, 2.2));
        assert!(assisted_cursor_target(
            position + Vec3::X * 0.8,
            Vec3::ZERO,
            position,
            &enemy
        ));
        assert!(!assisted_cursor_target(
            Vec3::Z * 2.0,
            Vec3::ZERO,
            Vec3::NEG_Z * 2.0,
            &enemy
        ));
        assert!(!arc_target_hit(Vec3::ZERO, Vec3::Z, Vec3::Z * 2.5, 2.2));
    }

    #[test]
    fn load_reset_removes_unreleased_strikes_and_buffered_intent() {
        use crate::combat::*;
        let mut world = World::new();
        world.insert_resource(SkillInputBuffer {
            secs: [0.1; 3],
            basic_secs: 0.1,
        });
        world.insert_resource(BasicAttackIntent {
            target: Some(Entity::PLACEHOLDER),
        });
        let mut animation = PlayerAnimation::default();
        animation.trigger_attack(PlayerAttackMotion::Basic, 0.34);
        let actor = world
            .spawn((
                Player,
                animation,
                queued_strike(
                    0.34,
                    Vec3::Z,
                    Vec3::Z,
                    StrikeShape::Arc { range: 2.2 },
                    PlayerAttackMotion::Basic,
                    StrikeProfile {
                        damage: 22.0,
                        impact_scale: 1.0,
                        crit_chance: 0.0,
                        legendary_powers: LegendaryPowerSet::new(
                            LegendaryPower::None,
                            LegendaryPower::None,
                        ),
                        skill: None,
                        hit_stop: true,
                        long_shot: None,
                        cast: 1,
                        fury_bonus: 0.0,
                        barrier_reward: 0.0,
                    },
                ),
            ))
            .id();
        clear_transient_actions(&mut world);
        assert!(world.get::<PendingPlayerStrike>(actor).is_none());
        assert_eq!(
            world
                .get::<PlayerAnimation>(actor)
                .unwrap()
                .attack_progress(),
            1.0
        );
        assert_eq!(world.resource::<BasicAttackIntent>().target, None);
        assert_eq!(world.resource::<SkillInputBuffer>().secs, [0.0; 3]);
    }

    #[test]
    fn feel_data_preserves_fast_light_contact_and_distinct_heavy_weight() {
        let data = tuning();
        assert!(data.basic_contact_fraction < data.heavy_contact_fraction);
        assert!(data.basic_pause_secs < data.heavy_pause_secs);
        assert!(data.contact_hold_fraction < data.contact_drag_fraction);
        assert!(data.contact_drag_fraction < 1.0);
        assert!(data.cancel_recovery_fraction > data.heavy_contact_fraction);
        assert!(data.chain_recovery_fraction < 1.0);
        assert!(data.assist_extra_reach < 0.25 && data.assist_aim_cosine > 0.9);
    }
}
