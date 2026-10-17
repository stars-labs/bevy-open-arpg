//! Runs the shipping summon behavior and damage systems without a renderer.
use super::*;
use std::time::Duration;

fn fixture(bone: u32, spirit: u32) -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .init_resource::<crate::dungeon::DungeonLayout>()
        .init_resource::<Bestiary>()
        .insert_resource(crate::ui::localization::UiLanguage::Chinese)
        .insert_resource(SelectedClass(HeroClass::Soulbinder))
        .init_resource::<GameAssets>()
        .init_resource::<SummonStatus>()
        .insert_resource(
            ron::from_str::<Tuning>(include_str!("../../../assets/data/summons.ron")).unwrap(),
        )
        .insert_resource(Art {
            arc: default(),
            ring: default(),
            cyan: default(),
            gold: default(),
        })
        .add_message::<ContactFeedback>()
        .add_message::<CombatEvent>()
        .add_message::<FloatingCombatTextEvent>()
        .add_message::<ScreenShakeEvent>()
        .add_message::<HitStopEvent>()
        .add_message::<SummonOrder>()
        .add_systems(Update, (orders, behavior).chain());
    let hero = app
        .world_mut()
        .spawn((
            Player,
            Transform::default(),
            Health {
                current: 100.0,
                max: 100.0,
            },
            SummonTalents { bone, spirit },
            DamageBonus(0.0),
            PlayerLevel {
                level: 20,
                xp: 0,
                next_level_xp: 100,
            },
            Barrier {
                current: 0.0,
                max: 100.0,
            },
            Fury {
                current: 0.0,
                max: 100.0,
                basic_gain: 0.0,
                regen_per_sec: 0.0,
                cost_discount: 0.0,
            },
        ))
        .id();
    (app, hero)
}

fn frames(app: &mut App, count: usize) {
    for _ in 0..count {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f64(1.0 / 60.0));
        app.update();
    }
}

fn enemy(app: &mut App, position: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            crate::combat::tests::test_enemy(vec![]),
            Transform::from_translation(position),
            Health {
                current: 10_000.0,
                max: 10_000.0,
            },
        ))
        .id()
}

fn minion(app: &mut App, slot: usize, bone: bool, position: Vec3) -> Entity {
    let tuning = app.world().resource::<Tuning>();
    let max = (if bone {
        tuning.bone_health
    } else {
        tuning.spirit_health
    }) * (1.0 + 19.0 * tuning.level_health_growth);
    app.world_mut()
        .spawn((
            Minion,
            Transform::from_translation(position),
            Visibility::Visible,
            SummonVitality {
                current: max,
                max,
                dead_secs: 0.0,
                guard_secs: 0.0,
                guard_multiplier: 0.45,
            },
            State {
                slot,
                bone,
                attack: None,
                cooldown: 100.0,
                phase: 0.0,
                moving: false,
                progress: 1.0,
                heavy: false,
                volley: 0.0,
                retreating: false,
                rebirth: false,
                echo: None,
                rune: Rune::Expose,
            },
        ))
        .id()
}

fn mark(app: &mut App, target: Entity) {
    let mut session = app.world_mut().resource_mut::<SummonStatus>();
    session.bond.target = Some(target);
    session.bond.remaining = 30.0;
}

fn heavy(app: &mut App, minion: Entity, target: Entity) {
    app.world_mut().get_mut::<State>(minion).unwrap().attack = Some((target, 0.25, true));
}

fn health(app: &App, target: Entity) -> f32 {
    app.world().get::<Health>(target).unwrap().current
}

#[test]
fn spirit_four_heavy_lands_then_echoes_once_and_new_mark_cancels_pending_echo() {
    let (mut app, _) = fixture(0, 4);
    let target = enemy(&mut app, Vec3::Z);
    let other = enemy(&mut app, Vec3::X);
    let pet = minion(&mut app, 0, false, Vec3::Z);
    mark(&mut app, target);
    heavy(&mut app, pet, target);
    frames(&mut app, 5);
    let after_heavy = health(&app, target);
    assert!(after_heavy < 10_000.0);
    assert!(app.world().get::<State>(pet).unwrap().echo.is_some());
    frames(&mut app, 6);
    assert_eq!(health(&app, target), after_heavy);
    frames(&mut app, 12);
    let after_echo = health(&app, target);
    assert!(after_echo < after_heavy);
    frames(&mut app, 30);
    assert_eq!(health(&app, target), after_echo);
    heavy(&mut app, pet, target);
    frames(&mut app, 5);
    let before_cancel = health(&app, target);
    mark(&mut app, other);
    frames(&mut app, 30);
    assert_eq!(health(&app, target), before_cancel);
    assert!(app.world().get::<State>(pet).unwrap().echo.is_none());
}

#[test]
fn spirit_five_two_heavy_contacts_share_one_eight_soul_siphon() {
    let (mut app, hero) = fixture(1, 5);
    let target = enemy(&mut app, Vec3::Z);
    mark(&mut app, target);
    for slot in 0..2 {
        let pet = minion(&mut app, slot, true, Vec3::Z);
        heavy(&mut app, pet, target);
    }
    frames(&mut app, 5);
    assert!(health(&app, target) < 10_000.0);
    assert_eq!(app.world().get::<Fury>(hero).unwrap().current, 8.0);
    assert!(app.world().resource::<SummonStatus>().siphon_cooldown > 0.0);
}

#[test]
fn bone_four_retreats_to_owner_heals_and_returns_to_target() {
    let (mut app, _) = fixture(4, 0);
    let target = enemy(&mut app, Vec3::Z * 5.0);
    mark(&mut app, target);
    let pet = minion(&mut app, 0, true, Vec3::Z * 5.0);
    let max = app.world().get::<SummonVitality>(pet).unwrap().max;
    app.world_mut()
        .get_mut::<SummonVitality>(pet)
        .unwrap()
        .current = max * 0.2;
    frames(&mut app, 30);
    assert!(app.world().get::<State>(pet).unwrap().retreating);
    assert!(app.world().get::<Transform>(pet).unwrap().translation.z < 5.0);
    frames(&mut app, 1800);
    assert!(app.world().get::<SummonVitality>(pet).unwrap().current >= max * 0.75);
    assert!(!app.world().get::<State>(pet).unwrap().retreating);
    // Refresh the mark after the long healing interval to test renewed pursuit.
    mark(&mut app, target);
    frames(&mut app, 120);
    assert!(app.world().get::<Transform>(pet).unwrap().translation.z > 3.0);
}

#[test]
fn bone_six_first_death_reforms_half_health_second_and_sacrifice_use_normal_delay() {
    let (mut app, _) = fixture(6, 0);
    let pet = minion(&mut app, 0, true, Vec3::ZERO);
    app.world_mut()
        .get_mut::<SummonVitality>(pet)
        .unwrap()
        .damage(100_000.0);
    frames(&mut app, 1);
    assert!(app.world().get::<State>(pet).unwrap().rebirth);
    assert!(app.world().get::<SummonVitality>(pet).unwrap().dead_secs <= 1.2);
    frames(&mut app, 70);
    assert_eq!(app.world().get::<SummonVitality>(pet).unwrap().current, 0.0);
    frames(&mut app, 2);
    let vitality = app.world().get::<SummonVitality>(pet).unwrap();
    assert!((vitality.current / vitality.max - 0.5).abs() < 0.01);
    app.world_mut()
        .get_mut::<SummonVitality>(pet)
        .unwrap()
        .damage(100_000.0);
    frames(&mut app, 1);
    assert!(!app.world().get::<State>(pet).unwrap().rebirth);
    assert!(app.world().get::<SummonVitality>(pet).unwrap().dead_secs > 5.9);
    frames(&mut app, 74);
    assert_eq!(app.world().get::<SummonVitality>(pet).unwrap().current, 0.0);
    frames(&mut app, 290);
    assert!(app.world().get::<SummonVitality>(pet).unwrap().current > 0.0);
    // Reset the passive cooldown: a deliberate sacrifice must still bypass it.
    app.world_mut()
        .resource_mut::<SummonStatus>()
        .rebirth_cooldown = [0.0; 2];
    app.world_mut().write_message(SummonOrder {
        command: SummonCommand::Sacrifice,
        rune: Rune::Expose,
        damage: 10.0,
        impact: 1.0,
    });
    frames(&mut app, 1);
    assert!(!app.world().get::<State>(pet).unwrap().rebirth);
    assert!(app.world().get::<SummonVitality>(pet).unwrap().dead_secs > 5.9);
}

#[test]
fn spirit_six_overdrive_relays_dead_mark_to_nearby_living_enemy() {
    let (mut app, _) = fixture(0, 6);
    let dead = enemy(&mut app, Vec3::Z);
    let next = enemy(&mut app, Vec3::Z * 2.0);
    let far = enemy(&mut app, Vec3::Z * 10.0);
    mark(&mut app, dead);
    frames(&mut app, 1);
    app.world_mut().get_mut::<Health>(dead).unwrap().current = 0.0;
    app.world_mut().resource_mut::<SummonStatus>().overdrive = 3.0;
    frames(&mut app, 1);
    let session = app.world().resource::<SummonStatus>();
    assert_eq!(session.marked, Some(next));
    assert!(session.overdrive < 3.0);
    assert_eq!(health(&app, next), 10_000.0);
    assert_eq!(health(&app, far), 10_000.0);
}

#[test]
fn flanking_volley_adds_exposure_without_replacing_equipped_bleed_rune() {
    let (mut app, _) = fixture(5, 0);
    let target = enemy(&mut app, Vec3::Z);
    minion(&mut app, 0, true, Vec3::Z);
    mark(&mut app, target);
    app.world_mut().write_message(SummonOrder {
        command: SummonCommand::Volley,
        rune: Rune::Hemorrhage,
        damage: 34.0,
        impact: 1.3,
    });
    frames(&mut app, 25);
    assert!(health(&app, target) < 10_000.0);
    assert!(app.world().get::<Vulnerable>(target).is_some());
    assert!(
        app.world().get::<Burning>(target).is_some(),
        "the equipped rune must survive the talent's extra status"
    );
}
