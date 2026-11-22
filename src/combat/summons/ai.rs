//! Autonomous summon target acquisition, tactical stance, and active skill rotation.
use super::*;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SummonStance {
    /// Proactively hunts enemies within `auto_acquire_radius` and uses full skill kit.
    #[default]
    Hunt,
    /// Engages threats approaching within `guard_acquire_radius` of the Soulbinder.
    Guard,
    /// Strictly focuses the Soulbinder's marked target while still casting skills on it.
    Focus,
}

impl SummonStance {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Hunt => Self::Guard,
            Self::Guard => Self::Focus,
            Self::Focus => Self::Hunt,
        }
    }

    pub(crate) fn label(self, zh: bool) -> &'static str {
        match (self, zh) {
            (Self::Hunt, true) => "主动猎杀",
            (Self::Guard, true) => "护主警戒",
            (Self::Focus, true) => "聚焦听令",
            (Self::Hunt, false) => "Hunt",
            (Self::Guard, false) => "Guard",
            (Self::Focus, false) => "Focus",
        }
    }

    pub(super) fn acquire_radius(self, tuning: &Tuning) -> f32 {
        match self {
            Self::Hunt => tuning.auto_acquire_radius.min(LEASH),
            Self::Guard => tuning.guard_acquire_radius.min(LEASH),
            Self::Focus => 0.0,
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AutoAction {
    #[default]
    Idle,
    Hunting,
    Lunge,
    Cleave,
    Wave,
    TauntSlam,
}

impl AutoAction {
    pub(super) fn label(self, zh: bool) -> Option<&'static str> {
        match (self, zh) {
            (Self::Lunge, true) => Some("幽影突袭"),
            (Self::Cleave, true) => Some("魂刃旋斩"),
            (Self::Wave, true) => Some("幽魂斩波"),
            (Self::TauntSlam, true) => Some("骨盾震击"),
            (Self::Hunting, true) => Some("猎杀中"),
            (Self::Lunge, false) => Some("Phantom Lunge"),
            (Self::Cleave, false) => Some("Spectral Cleave"),
            (Self::Wave, false) => Some("Spectral Wave"),
            (Self::TauntSlam, false) => Some("Ossuary Slam"),
            (Self::Hunting, false) => Some("Hunting"),
            (Self::Idle, _) => None,
        }
    }
}

/// Piercing spectral energy wave launched autonomously by the Bound Spirit.
#[derive(Component)]
pub(super) struct SummonWave {
    direction: Vec3,
    speed: f32,
    remaining_distance: f32,
    radius: f32,
    damage: f32,
    impact: f32,
    rune: Option<Rune>,
    hit: Vec<Entity>,
}

pub(super) fn is_priority_foe(enemy: &Enemy) -> bool {
    !enemy.affixes.is_empty()
        || matches!(
            enemy.id.as_str(),
            "keeper" | "hollow_matron" | "seal_warden" | "bell_warden" | "nemesis" | "butcher"
        )
}

/// Resolves the active combat target for one minion.
/// Player marks always win; otherwise the minion independently hunts within stance range,
/// prioritizing threats near the owner and spreading Bone Guards across nearby packs.
#[allow(clippy::too_many_arguments)]
pub(super) fn acquire_target(
    stance: SummonStance,
    bone: bool,
    owner: Vec3,
    minion_pos: Vec3,
    marked: Option<(Entity, Vec3)>,
    current_auto: Option<Entity>,
    claimed_by_sibling: &[Entity],
    enemies: &EnemyTargets,
    tuning: &Tuning,
) -> Option<(Entity, Vec3, bool)> {
    if let Some((entity, pos)) = marked {
        return Some((entity, pos, true));
    }
    let radius = stance.acquire_radius(tuning);
    if radius <= 0.0 {
        return None;
    }

    let mut best: Option<(Entity, Vec3, (u8, u8, i32))> = None;
    for (entity, transform, _, health, ..) in enemies.iter() {
        if health.current <= 0.0 {
            continue;
        }
        let pos = transform.translation;
        let owner_dist = Vec2::new(pos.x - owner.x, pos.z - owner.z).length();
        if owner_dist > radius || !within_leash(owner, pos) {
            continue;
        }
        let threat_tier = u8::from(owner_dist > tuning.threat_priority_radius);
        let spread_tier = u8::from(bone && claimed_by_sibling.contains(&entity));
        let mut score_dist = Vec2::new(pos.x - minion_pos.x, pos.z - minion_pos.z).length();
        if current_auto == Some(entity) {
            // Gentle hysteresis prevents jittering between equidistant enemies.
            score_dist = (score_dist - 0.9).max(0.0);
        }
        let key = (threat_tier, spread_tier, (score_dist * 100.0) as i32);
        if best.is_none_or(|(_, _, best_key)| key < best_key) {
            best = Some((entity, pos, key));
        }
    }
    best.map(|(entity, pos, _)| (entity, pos, false))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_cast_autonomous_skill(
    state: &mut State,
    vitality: &mut SummonVitality,
    position: &mut Transform,
    hero_pos: Vec3,
    target: Entity,
    target_pos: Vec3,
    base_damage: f32,
    overdrive_active: bool,
    overdrive_rune: Option<Rune>,
    layout: &crate::dungeon::DungeonLayout,
    damage: &mut Damage,
    commands: &mut Commands,
    art: &Art,
    tuning: &Tuning,
) -> bool {
    let flat_delta = Vec3::new(
        target_pos.x - position.translation.x,
        0.0,
        target_pos.z - position.translation.z,
    );
    let dist = flat_delta.length();
    let dir = flat_delta.normalize_or_zero();
    let overdrive_mult = if overdrive_active {
        tuning.overdrive_damage_multiplier
    } else {
        1.0
    };
    let skill_rune = if overdrive_active {
        overdrive_rune
    } else {
        None
    };

    // Count nearby living enemies and check if target is a priority elite/boss.
    let mut nearby_cleave = 0usize;
    let mut nearby_taunt = 0usize;
    let mut threatening_hero = false;
    let mut target_is_priority = false;
    for (entity, transform, enemy, health, ..) in damage.enemies.iter() {
        if health.current <= 0.0 {
            continue;
        }
        let d_minion = transform.translation.distance(position.translation);
        if d_minion <= tuning.cleave_radius {
            nearby_cleave += 1;
        }
        if d_minion <= tuning.taunt_radius {
            nearby_taunt += 1;
        }
        if transform.translation.distance(hero_pos) <= tuning.taunt_radius
            && d_minion <= tuning.taunt_radius
        {
            threatening_hero = true;
        }
        if entity == target && is_priority_foe(enemy) {
            target_is_priority = true;
        }
    }

    // 1. Bone Guard: Ossuary Taunt Slam when protecting the Soulbinder or surrounded.
    if state.bone
        && state.taunt_cooldown <= 0.0
        && (threatening_hero || nearby_taunt >= 2)
        && dist <= tuning.taunt_radius
    {
        let amount = base_damage * tuning.taunt_damage_multiplier * overdrive_mult;
        damage.strike(
            commands,
            position.translation,
            HitArea::Radial {
                center: position.translation,
                radius: tuning.taunt_radius,
            },
            amount,
            tuning.taunt_impact,
            skill_rune,
        );
        vitality.guard_secs = vitality.guard_secs.max(tuning.taunt_guard_secs);
        flash(commands, art, position.translation, 1.45, true);
        slash(
            commands,
            art,
            position.translation + Vec3::Y * 0.45,
            position.rotation,
            true,
        );
        state.taunt_cooldown = tuning.taunt_cooldown;
        state.cooldown = tuning.attack_cooldown * 0.9;
        state.progress = 0.25;
        state.heavy = true;
        state.action_label = AutoAction::TauntSlam;
        state.action_banner_secs = 1.25;
        info!(
            "SUMMON_AUTO_TAUNT_SLAM slot={} amount={amount:.1}",
            state.slot
        );
        return true;
    }

    // 2. Spectral Cleave: 360° soul-blade nova against packs or elites/bosses in melee range.
    if state.cleave_cooldown <= 0.0
        && dist <= tuning.cleave_radius
        && (nearby_cleave >= 2 || target_is_priority)
    {
        let amount = base_damage * tuning.cleave_damage_multiplier * overdrive_mult;
        damage.strike(
            commands,
            position.translation,
            HitArea::Radial {
                center: position.translation,
                radius: tuning.cleave_radius,
            },
            amount,
            tuning.cleave_impact,
            skill_rune,
        );
        flash(commands, art, position.translation, 1.35, overdrive_active);
        slash(
            commands,
            art,
            position.translation + Vec3::Y * 0.65,
            position.rotation,
            true,
        );
        slash(
            commands,
            art,
            position.translation + Vec3::Y * 0.65,
            position.rotation * Quat::from_rotation_y(std::f32::consts::PI),
            false,
        );
        state.cleave_cooldown = tuning.cleave_cooldown;
        state.cooldown = tuning.attack_cooldown;
        state.progress = 0.2;
        state.heavy = true;
        state.action_label = AutoAction::Cleave;
        state.action_banner_secs = 1.2;
        info!(
            "SUMMON_AUTO_CLEAVE slot={} nearby={} amount={amount:.1}",
            state.slot, nearby_cleave
        );
        return true;
    }

    // 3. Phantom Lunge: phase-dash gap closer + immediate heavy strike for mid-range targets.
    if state.lunge_cooldown <= 0.0
        && dist >= tuning.lunge_min_range
        && dist <= tuning.lunge_max_range
        && dir.length_squared() > 0.01
    {
        let start = position.translation;
        let desired_landing = target_pos - dir * (REACH * 0.65);
        let landing = crate::player::clamp_player_translation(layout, desired_landing);
        if landing.distance(target_pos) <= REACH * 1.15 {
            position.translation = landing;
            position.rotation = Quat::from_rotation_y(dir.x.atan2(dir.z));
            spawn_lunge_trail(commands, art, start, landing, overdrive_active);
            flash(commands, art, start, 0.85, false);
            let amount = base_damage * tuning.lunge_damage_multiplier * overdrive_mult;
            damage.strike(
                commands,
                landing,
                if overdrive_active {
                    HitArea::Radial {
                        center: landing,
                        radius: 2.0,
                    }
                } else {
                    HitArea::Target(target)
                },
                amount,
                tuning.lunge_impact,
                skill_rune,
            );
            slash(
                commands,
                art,
                landing + Vec3::Y * 0.68,
                position.rotation,
                true,
            );
            state.lunge_cooldown = tuning.lunge_cooldown;
            state.cooldown = tuning.attack_cooldown * 0.85;
            state.progress = 0.2;
            state.heavy = true;
            state.action_label = AutoAction::Lunge;
            state.action_banner_secs = 1.15;
            info!(
                "SUMMON_AUTO_LUNGE slot={} dist={dist:.2} amount={amount:.1}",
                state.slot
            );
            return true;
        }
    }

    // 4. Bound Spirit: Spectral Wave ranged piercing crescent when target is mid-range.
    if !state.bone
        && state.wave_cooldown <= 0.0
        && dist >= tuning.wave_min_range
        && dist <= tuning.wave_max_range
        && dir.length_squared() > 0.01
    {
        let yaw = dir.x.atan2(dir.z);
        position.rotation = Quat::from_rotation_y(yaw);
        let amount = base_damage * tuning.wave_damage_multiplier * overdrive_mult;
        commands.spawn((
            SummonWave {
                direction: dir,
                speed: tuning.wave_speed,
                remaining_distance: tuning.wave_max_range + 1.5,
                radius: tuning.wave_radius,
                damage: amount,
                impact: tuning.wave_impact,
                rune: skill_rune,
                hit: Vec::new(),
            },
            Mesh3d(art.arc.clone()),
            MeshMaterial3d(if overdrive_active {
                art.gold.clone()
            } else {
                art.cyan.clone()
            }),
            Transform::from_translation(position.translation + dir * 0.55 + Vec3::Y * 0.72)
                .with_rotation(Quat::from_rotation_y(yaw))
                .with_scale(Vec3::new(1.15, 1.0, 0.95)),
            NotShadowCaster,
            NotShadowReceiver,
            Name::new("Spectral Wave"),
        ));
        slash(
            commands,
            art,
            position.translation + Vec3::Y * 0.72,
            position.rotation,
            false,
        );
        state.wave_cooldown = tuning.wave_cooldown;
        state.cooldown = tuning.attack_cooldown;
        state.progress = 0.25;
        state.heavy = false;
        state.action_label = AutoAction::Wave;
        state.action_banner_secs = 1.1;
        info!(
            "SUMMON_AUTO_WAVE slot={} dist={dist:.2} amount={amount:.1}",
            state.slot
        );
        return true;
    }

    false
}

fn spawn_lunge_trail(commands: &mut Commands, art: &Art, start: Vec3, end: Vec3, heavy: bool) {
    let delta = Vec3::new(end.x - start.x, 0.0, end.z - start.z);
    let len = delta.length();
    if len < 0.2 {
        return;
    }
    let yaw = delta.x.atan2(delta.z);
    let side = Vec3::new(delta.z, 0.0, -delta.x).normalize_or_zero();
    for offset in [-0.22, 0.0, 0.22] {
        commands.spawn((
            LungeTrail(0.22),
            Mesh3d(art.streak.clone()),
            MeshMaterial3d(if heavy || offset == 0.0 {
                art.cyan.clone()
            } else {
                art.idle_cyan.clone()
            }),
            Transform::from_translation(start + side * offset + Vec3::Y * 0.58)
                .with_rotation(Quat::from_rotation_y(yaw))
                .with_scale(Vec3::new(if offset == 0.0 { 1.35 } else { 0.85 }, 1.0, len)),
            NotShadowCaster,
            NotShadowReceiver,
            Name::new("Phantom Lunge Trail"),
        ));
    }
}

pub(super) fn advance_waves(
    time: Res<Time>,
    mut commands: Commands,
    art: Res<Art>,
    mut waves: Query<(Entity, &mut SummonWave, &mut Transform), Without<Enemy>>,
    mut damage: Damage,
) {
    let dt = time.delta_secs();
    for (entity, mut wave, mut transform) in &mut waves {
        let step = (wave.speed * dt).min(wave.remaining_distance.max(0.0));
        transform.translation += wave.direction * step;
        wave.remaining_distance -= step;

        let wave_pos = transform.translation;
        let targets: Vec<(Entity, Vec3)> = damage
            .enemies
            .iter()
            .filter(|(enemy, enemy_tf, _, health, ..)| {
                health.current > 0.0
                    && !wave.hit.contains(enemy)
                    && Vec2::new(
                        enemy_tf.translation.x - wave_pos.x,
                        enemy_tf.translation.z - wave_pos.z,
                    )
                    .length()
                        <= wave.radius
            })
            .map(|(enemy, enemy_tf, ..)| (enemy, enemy_tf.translation))
            .collect();

        for (target, pos) in targets {
            wave.hit.push(target);
            damage.strike(
                &mut commands,
                wave_pos,
                HitArea::Target(target),
                wave.damage,
                wave.impact,
                wave.rune,
            );
            flash(&mut commands, &art, pos + Vec3::Y * 0.5, 0.72, false);
        }

        if wave.remaining_distance <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
