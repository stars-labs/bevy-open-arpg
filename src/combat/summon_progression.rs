//! Shared eligibility rules for the Soulbinder's two summon paths.
pub(crate) const TIER_LEVELS: [u32; 6] = [2, 4, 6, 10, 14, 18];
pub(crate) const MAX_POINTS: u32 = 10;
pub(crate) const LEVEL_CAP: u32 = 20;

#[derive(serde::Deserialize)]
struct ProgressionTuning {
    xp_growth: f32,
}

pub(crate) fn next_level_xp(current: u32) -> u32 {
    let tuning: ProgressionTuning =
        ron::from_str(include_str!("../../assets/data/summon_progression.ron"))
            .expect("valid Soulbinder progression tuning");
    (current as f32 * tuning.xp_growth).round() as u32
}

pub(crate) fn can_unlock(
    bone: u32,
    spirit: u32,
    is_bone: bool,
    tier: u32,
    points: u32,
    level: u32,
) -> bool {
    let Some(required_level) = tier
        .checked_sub(1)
        .and_then(|index| TIER_LEVELS.get(index as usize))
    else {
        return false;
    };
    let (path, other) = if is_bone {
        (bone, spirit)
    } else {
        (spirit, bone)
    };
    points > 0
        && bone.saturating_add(spirit) < MAX_POINTS
        && path.checked_add(1) == Some(tier)
        && level >= *required_level
        && (tier != 6 || other < 6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_twenty_xp_budget_is_reachable_and_thresholds_always_increase() {
        let mut threshold = 60;
        let mut total = 0;
        for _ in 2..=LEVEL_CAP {
            total += threshold;
            let next = next_level_xp(threshold);
            assert!(next > threshold);
            threshold = next;
        }
        assert_eq!(total, 7_444);
        assert!((7_000..=12_000).contains(&total));
    }

    #[test]
    fn each_tier_requires_its_level_and_previous_tier() {
        for (index, level) in TIER_LEVELS.into_iter().enumerate() {
            let tier = index as u32 + 1;
            assert!(can_unlock(tier - 1, 0, true, tier, 1, level));
            assert!(!can_unlock(tier - 1, 0, true, tier, 1, level - 1));
            assert!(can_unlock(0, tier - 1, false, tier, 1, level));
            assert!(!can_unlock(tier, 0, true, tier, 1, level));
        }
        assert!(!can_unlock(0, 0, true, 2, 1, LEVEL_CAP));
        assert!(!can_unlock(0, 0, true, 0, 1, LEVEL_CAP));
        assert!(!can_unlock(0, 0, true, 7, 1, LEVEL_CAP));
    }

    #[test]
    fn spending_and_specialization_limits_apply_to_both_paths() {
        assert!(!can_unlock(0, 0, true, 1, 0, LEVEL_CAP));
        assert!(can_unlock(5, 4, true, 6, 1, LEVEL_CAP));
        assert!(can_unlock(4, 5, false, 6, 1, LEVEL_CAP));
        assert!(!can_unlock(5, 5, true, 6, 1, LEVEL_CAP));
        assert!(!can_unlock(5, 6, true, 6, 1, LEVEL_CAP));
        assert!(!can_unlock(6, 5, false, 6, 1, LEVEL_CAP));
        assert!(!can_unlock(u32::MAX, 1, false, 2, 1, LEVEL_CAP));
    }
}
