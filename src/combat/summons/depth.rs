//! Upper-tier summon decisions; every proc requires a real, living target.
use super::*;

pub(super) struct Echo {
    target: Entity,
    remaining: f32,
    amount: f32,
}

pub(super) fn retreating(previous: bool, current: f32, maximum: f32, tuning: &Tuning) -> bool {
    current > 0.0
        && current / maximum.max(1.0)
            < if previous {
                tuning.retreat_release_fraction
            } else {
                tuning.retreat_trigger_fraction
            }
}

pub(super) fn flank(target: Vec3, owner: Vec3, slot: usize, offset: f32) -> Vec3 {
    let forward = (target - owner).with_y(0.0).normalize_or(Vec3::Z);
    target + Vec3::new(forward.z, 0.0, -forward.x) * if slot == 0 { -offset } else { offset }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn echo(
    state: &mut State,
    dt: f32,
    source: Vec3,
    marked: Option<Entity>,
    damage: &mut Damage,
    commands: &mut Commands,
    art: &Art,
    tuning: &Tuning,
) {
    let Some(mut echo) = state.echo.take() else {
        return;
    };
    if marked != Some(echo.target)
        || !damage
            .enemies
            .get(echo.target)
            .is_ok_and(|e| e.3.current > 0.0 && e.1.translation.distance(source) <= REACH * 1.5)
    {
        return;
    }
    echo.remaining -= dt;
    if echo.remaining > 0.0 {
        state.echo = Some(echo);
        return;
    }
    damage.strike(
        commands,
        source,
        HitArea::Target(echo.target),
        echo.amount,
        tuning.echo_impact,
        None,
    );
    flash(commands, art, source + Vec3::Y * 0.6, 0.8, true);
    info!(
        "SUMMON_AFTERSHOCK target={:?} amount={:.1}",
        echo.target, echo.amount
    );
}

pub(super) fn queue_echo(state: &mut State, target: Entity, amount: f32, tuning: &Tuning) {
    state.echo = Some(Echo {
        target,
        remaining: tuning.aftershock_delay,
        amount: amount * tuning.aftershock_fraction,
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn relay(
    session: &mut SummonStatus,
    talents: &SummonTalents,
    owner: Vec3,
    damage: &Damage,
    tuning: &Tuning,
    commands: &mut Commands,
    art: &Art,
) {
    let Some(target) = session.bond.target else {
        return;
    };
    if let Ok(enemy) = damage.enemies.get(target) {
        session.mark_position = enemy.1.translation;
        if enemy.3.current > 0.0 {
            return;
        }
    }
    if talents.spirit < 6 || session.overdrive <= 0.0 {
        return;
    }
    // Passing the mark deals no damage and does not extend Overdrive or its own lifetime.
    let next = damage
        .enemies
        .iter()
        .filter(|e| {
            e.0 != target
                && e.3.current > 0.0
                && e.1.translation.distance(session.mark_position) <= tuning.relay_radius
                && within_leash(owner, e.1.translation)
        })
        .min_by(|a, b| {
            a.1.translation
                .distance_squared(session.mark_position)
                .total_cmp(&b.1.translation.distance_squared(session.mark_position))
        })
        .map(|e| e.0);
    if let Some(next) = next {
        session.bond.target = Some(next);
        flash(commands, art, session.mark_position, 1.0, true);
        info!("SUMMON_RELAY from={target:?} to={next:?}");
    }
}

pub(super) fn rebirth_delay(unlocked: bool, cooldown: &mut f32, tuning: &Tuning) -> Option<f32> {
    if !unlocked || *cooldown > 0.0 {
        return None;
    }
    *cooldown = tuning.last_stand_cooldown;
    Some(tuning.last_stand_reform_secs)
}

pub(super) fn siphon_gain(rank: u32, cooldown: &mut f32, tuning: &Tuning) -> f32 {
    if rank < 5 || *cooldown > 0.0 {
        return 0.0;
    }
    *cooldown = tuning.siphon_cooldown;
    tuning.siphon_amount
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retreat_uses_hysteresis_and_never_resurrects_dead_guards() {
        let tuning: Tuning =
            ron::from_str(include_str!("../../../assets/data/summons.ron")).unwrap();
        assert!(retreating(false, 29.0, 100.0, &tuning));
        assert!(!retreating(false, 40.0, 100.0, &tuning));
        assert!(retreating(true, 60.0, 100.0, &tuning));
        assert!(!retreating(true, 80.0, 100.0, &tuning));
        assert!(!retreating(true, 0.0, 100.0, &tuning));
    }
    #[test]
    fn rebirth_cannot_bypass_its_cooldown_or_locked_tier() {
        let tuning: Tuning =
            ron::from_str(include_str!("../../../assets/data/summons.ron")).unwrap();
        let mut cooldown = 0.0;
        assert_eq!(rebirth_delay(false, &mut cooldown, &tuning), None);
        assert_eq!(rebirth_delay(true, &mut cooldown, &tuning), Some(1.2));
        assert_eq!(cooldown, 30.0);
        assert_eq!(rebirth_delay(true, &mut cooldown, &tuning), None);
        cooldown = 0.0;
        assert_eq!(rebirth_delay(true, &mut cooldown, &tuning), Some(1.2));
    }
    #[test]
    fn siphon_is_shared_and_rate_limited_across_multiple_guard_hits() {
        let tuning: Tuning =
            ron::from_str(include_str!("../../../assets/data/summons.ron")).unwrap();
        let mut cooldown = 0.0;
        assert_eq!(siphon_gain(4, &mut cooldown, &tuning), 0.0);
        assert_eq!(siphon_gain(5, &mut cooldown, &tuning), 8.0);
        assert_eq!(siphon_gain(6, &mut cooldown, &tuning), 0.0);
        cooldown = 0.0;
        assert_eq!(siphon_gain(6, &mut cooldown, &tuning), 8.0);
    }
    #[test]
    fn two_guards_approach_opposite_flanks_inside_contact_range() {
        let target = Vec3::new(4.0, 0.0, 0.0);
        let a = flank(target, Vec3::ZERO, 0, 0.8);
        let b = flank(target, Vec3::ZERO, 1, 0.8);
        assert!((a.distance(target) - 0.8).abs() < 0.001);
        assert!((a + b).distance(target * 2.0) < 0.001);
        assert!(a.distance(b) > 1.5 && a.distance(target) < REACH);
    }
}
