//! Mark lifetime, contact timing and leash rules used by the shipping Soulbinder.
use bevy::prelude::*;
use std::collections::VecDeque;

pub(super) const MARK_SECONDS: f32 = 4.0;
pub(super) const LEASH: f32 = 10.0;
pub(super) const REACH: f32 = 1.55;
pub(super) const WINDUP: f32 = 0.32;
pub(super) const CONTACT: f32 = 0.44;
pub(super) const HEAVY_WINDUP: f32 = 0.50;
pub(super) const HEAVY_CONTACT: f32 = 0.58;

#[derive(Default, Debug)]
pub(super) struct Bond {
    pub target: Option<Entity>,
    pub remaining: f32,
    pub empowered: bool,
    seen: VecDeque<u32>,
}
impl Bond {
    /// Only resolved living-target contacts enter this method. One command per cast.
    pub fn observe(&mut self, cast: u32, target: Entity, skill: bool) -> bool {
        if self.seen.contains(&cast) {
            return false;
        }
        self.seen.push_back(cast);
        if self.seen.len() > 64 {
            self.seen.pop_front();
        }
        if skill {
            if self.target == Some(target) && self.remaining > 0.0 {
                self.empowered = true;
                return true;
            }
            return false;
        }
        if self.target != Some(target) {
            self.empowered = false;
        }
        self.target = Some(target);
        self.remaining = MARK_SECONDS;
        true
    }
    pub fn tick(&mut self, dt: f32, valid: bool) {
        self.remaining = (self.remaining - dt.max(0.0)).max(0.0);
        if !valid || self.remaining == 0.0 {
            self.target = None;
            self.empowered = false;
        }
    }
    pub fn take_empowered(&mut self) -> bool {
        std::mem::take(&mut self.empowered)
    }
}

pub(super) fn step_toward(origin: Vec3, goal: Vec3, step: f32) -> Vec3 {
    let flat = Vec3::new(goal.x - origin.x, 0.0, goal.z - origin.z);
    origin + flat.normalize_or_zero() * step.max(0.0).min(flat.length())
}

/// True only once while crossing the contact frame, including a long frame.
pub(super) fn contact_crossed(before: f32, after: f32, heavy: bool) -> bool {
    let point = if heavy {
        HEAVY_WINDUP * HEAVY_CONTACT
    } else {
        WINDUP * CONTACT
    };
    before < point && after >= point
}

pub(super) fn within_leash(owner: Vec3, target: Vec3) -> bool {
    Vec2::new(owner.x - target.x, owner.z - target.z).length() <= LEASH
}
