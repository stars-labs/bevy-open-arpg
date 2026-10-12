//! Exercise the same command state and contact timing used by the shipping controller.
#[path = "../src/combat/summon_logic.rs"]
mod logic;
use bevy::prelude::*;
use logic::*;

#[test]
fn spirit_has_no_damage_command_until_a_real_contact_and_expires() {
    let mut bond = Bond::default();
    assert_eq!(bond.target, None);
    let target = Entity::from_bits(1);
    assert!(bond.observe(1, target, false));
    bond.tick(MARK_SECONDS - 0.1, true);
    assert_eq!(bond.target, Some(target));
    bond.tick(0.2, true);
    assert_eq!(bond.target, None);
    assert!(!bond.empowered);
}
#[test]
fn coordinated_strike_requires_the_marked_target_and_consumes_once() {
    let a = Entity::from_bits(1);
    let b = Entity::from_bits(2);
    let mut bond = Bond::default();
    assert!(!bond.observe(1, a, true));
    assert!(bond.observe(2, a, false));
    assert!(!bond.observe(3, b, true));
    assert!(bond.observe(4, a, true));
    assert!(bond.take_empowered());
    assert!(!bond.take_empowered());
}
#[test]
fn scattered_hits_of_one_cast_never_switch_targets_or_rearm() {
    let a = Entity::from_bits(1);
    let b = Entity::from_bits(2);
    let mut bond = Bond::default();
    assert!(bond.observe(11, a, false));
    assert!(!bond.observe(11, b, false));
    assert_eq!(bond.target, Some(a));
    assert!(bond.observe(12, a, true));
    assert!(bond.take_empowered());
    assert!(!bond.observe(12, a, true));
    assert!(!bond.empowered);
}
#[test]
fn target_switch_death_or_leash_invalidates_pending_empowerment() {
    let a = Entity::from_bits(1);
    let b = Entity::from_bits(2);
    let mut bond = Bond::default();
    bond.observe(1, a, false);
    bond.observe(2, a, true);
    bond.observe(3, b, false);
    assert_eq!(bond.target, Some(b));
    assert!(!bond.empowered);
    bond.observe(4, b, true);
    bond.tick(0.01, false);
    assert_eq!(bond.target, None);
    assert!(!bond.empowered);
}
#[test]
fn attack_contact_is_crossed_once_even_with_a_long_frame() {
    let contact = WINDUP * CONTACT;
    assert!(!contact_crossed(0.0, contact - 0.001, false));
    assert!(contact_crossed(contact - 0.001, WINDUP, false));
    assert!(!contact_crossed(contact, WINDUP, false));
}
#[test]
fn chase_never_overshoots_and_preserves_ground_height() {
    let origin = Vec3::new(0.0, 0.0, 0.0);
    let goal = Vec3::new(0.1, 5.0, -0.1);
    assert_eq!(step_toward(origin, goal, 100.0), Vec3::new(0.1, 0.0, -0.1));
    assert_eq!(step_toward(origin, goal, -2.0), origin);
    assert_eq!(step_toward(origin, origin, 1.0), origin);
    assert!(within_leash(origin, Vec3::X * REACH));
    assert!(!within_leash(origin, Vec3::X * (LEASH + 0.1)));
}

#[test]
fn empowered_cut_contacts_later_than_the_basic_swing() {
    let basic = WINDUP * CONTACT;
    let heavy = HEAVY_WINDUP * HEAVY_CONTACT;
    assert!(contact_crossed(0.0, basic, false));
    assert!(!contact_crossed(0.0, basic, true));
    assert!(contact_crossed(basic, heavy, true));
    assert!(!contact_crossed(heavy, HEAVY_WINDUP, true));
}
