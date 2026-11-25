//! Full-body articulated procedural motion engine with 114 distinct actions
//! covering locomotion, charge/windup, class combos, evades, hitstun/stagger,
//! status ailments, consumables/interactions, bosses, summons, companions,
//! and co-op/NPC emotes.
use crate::{GameState, not_paused};
use bevy::prelude::*;

/// High-level grouping for the 114 actions in [`MotionId`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MotionCategory {
    Locomotion,
    ChargeWindup,
    ClassCombat,
    EvadeRecovery,
    HitstunStagger,
    StatusAilment,
    BuffInteraction,
    EnemyBoss,
    SummonCompanion,
    CoopSocialNpc,
}

/// Complete catalog of 114 distinct character motions across all gameplay and
/// cooperative scenarios.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum MotionId {
    // --- 1. Locomotion & Stance (14) ---
    #[default]
    IdleBreathe,
    IdleBattleGuard,
    IdleExhausted,
    IdleWardenHeroic,
    IdleStalkerCrouch,
    IdleEmberHover,
    IdleSoulbinderChannel,
    WalkNormal,
    RunSprint,
    WalkExhausted,
    StrafeLeft,
    StrafeRight,
    TurnPivotLean,
    StopSkidRecover,

    // --- 2. Charge, Windup & Channel (12) ---
    ChargeHeavyMelee,
    ChargeOverheadCleave,
    ChargeBowDraw,
    ChargeBowSkyward,
    ChargeStaffSkyward,
    ChargeGroundSlam,
    ChargeShieldBrace,
    ChargeSummonRitual,
    ChargePortalChannel,
    ChargeSurgeAwaken,
    ChargeBossGuillotine,
    ChargeBossCrescendo,

    // --- 3. Class Combat & Combos (24) ---
    WardenSlash1,
    WardenSlash2Reverse,
    WardenSlash3Overhead,
    WardenWhirlwindSpin,
    WardenLeapSlamAir,
    WardenSolarCleave,
    StalkerQuickShot,
    StalkerTwinSnap,
    StalkerPiercingRelease,
    StalkerVaultBackflip,
    StalkerShadowLunge,
    StalkerPhantomVolley,
    EmberStaffFlick,
    EmberTwinBoltThrust,
    EmberCinderfallSlam,
    EmberFrostRingBurst,
    EmberAshStepBlink,
    EmberOverloadLevitate,
    SoulbinderReapArc,
    SoulbinderCommandPoint,
    SoulbinderRaiseGuard,
    SoulbinderSpiritSurge,
    SoulbinderSacrificeClasp,
    SoulbinderRecallPulse,

    // --- 4. Evade, Mobility & Recovery (10) ---
    EvadeForwardRoll,
    EvadeBackstepHop,
    EvadeSideWeaveLeft,
    EvadeSideWeaveRight,
    EvadeWardenBarge,
    EvadeStalkerTumble,
    EvadeEmberPhaseShift,
    EvadeSoulbinderGlide,
    AttackRecoveryLight,
    AttackRecoveryHeavy,

    // --- 5. Hitstun, Stagger & Knockback (12) ---
    HitFlinchFront,
    HitFlinchBack,
    HitFlinchLeft,
    HitFlinchRight,
    HitHeavyRecoil,
    HitGuardBlock,
    HitEvadeGraze,
    StaggerDazedLoop,
    StaggerKneelCollapse,
    KnockbackLaunch,
    KnockbackLandRecover,
    HitStopFreezeFrame,

    // --- 6. Status Ailments (12) ---
    StatusBurningFlail,
    StatusChilledShiver,
    StatusFrozenRigid,
    StatusJailedStrain,
    StatusBleedingClutch,
    StatusVulnerableExposed,
    StatusPoisonedHunch,
    StatusCursedTremor,
    StatusFearedCower,
    StatusSilencedClutchThroat,
    StatusExhaustedGasp,
    StatusBlindGrope,

    // --- 7. Buffs, Consumables & World Interactions (10) ---
    DrinkHealthPotion,
    DrinkElixirFlask,
    ActivateSurgeRoar,
    ActivateGloryStride,
    ActivateConduitHover,
    InteractChestReach,
    InteractShrinePray,
    InteractLoreInspect,
    LevelUpTriumph,
    DeathCollapse,

    // --- 8. Enemy & Boss Specialized Motions (8) ---
    EnemyAlertStartle,
    EnemyMeleeSwing,
    EnemyHeavyOverhead,
    EnemyRangedDraw,
    EnemyShockwaveSlam,
    BossEntranceRoar,
    BossPhaseShiftAscend,
    BossBellTollSwing,

    // --- 9. Summon, Companion & Co-op Actions (8) ---
    SummonSpiritHoverSlash,
    SummonSpiritPhantomLunge,
    SummonSpiritSpectralWave,
    SummonBoneGuardBrace,
    SummonBoneTauntSlam,
    CompanionVanguardCharge,
    CompanionGuardianAegis,
    CoopRallyCall,

    // --- 10. Social Emotes & NPC Actions (4) ---
    CoopSalute,
    CoopCheer,
    CoopPointTarget,
    NpcMerchantBeckon,
}

/// Static metadata describing each action in [`MotionId::ALL`].
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct MotionSpec {
    pub id: MotionId,
    pub category: MotionCategory,
    pub name_zh: &'static str,
    pub name_en: &'static str,
    pub duration_secs: f32,
    pub looping: bool,
    /// Higher priority actions preempt lower priority actions (e.g. Death > Stagger > Charge > Emote > Idle).
    pub priority: u8,
}

impl MotionId {
    /// All 114 authored actions in canonical catalog order.
    #[allow(dead_code)]
    pub const ALL: [MotionId; 114] = [
        Self::IdleBreathe,
        Self::IdleBattleGuard,
        Self::IdleExhausted,
        Self::IdleWardenHeroic,
        Self::IdleStalkerCrouch,
        Self::IdleEmberHover,
        Self::IdleSoulbinderChannel,
        Self::WalkNormal,
        Self::RunSprint,
        Self::WalkExhausted,
        Self::StrafeLeft,
        Self::StrafeRight,
        Self::TurnPivotLean,
        Self::StopSkidRecover,
        Self::ChargeHeavyMelee,
        Self::ChargeOverheadCleave,
        Self::ChargeBowDraw,
        Self::ChargeBowSkyward,
        Self::ChargeStaffSkyward,
        Self::ChargeGroundSlam,
        Self::ChargeShieldBrace,
        Self::ChargeSummonRitual,
        Self::ChargePortalChannel,
        Self::ChargeSurgeAwaken,
        Self::ChargeBossGuillotine,
        Self::ChargeBossCrescendo,
        Self::WardenSlash1,
        Self::WardenSlash2Reverse,
        Self::WardenSlash3Overhead,
        Self::WardenWhirlwindSpin,
        Self::WardenLeapSlamAir,
        Self::WardenSolarCleave,
        Self::StalkerQuickShot,
        Self::StalkerTwinSnap,
        Self::StalkerPiercingRelease,
        Self::StalkerVaultBackflip,
        Self::StalkerShadowLunge,
        Self::StalkerPhantomVolley,
        Self::EmberStaffFlick,
        Self::EmberTwinBoltThrust,
        Self::EmberCinderfallSlam,
        Self::EmberFrostRingBurst,
        Self::EmberAshStepBlink,
        Self::EmberOverloadLevitate,
        Self::SoulbinderReapArc,
        Self::SoulbinderCommandPoint,
        Self::SoulbinderRaiseGuard,
        Self::SoulbinderSpiritSurge,
        Self::SoulbinderSacrificeClasp,
        Self::SoulbinderRecallPulse,
        Self::EvadeForwardRoll,
        Self::EvadeBackstepHop,
        Self::EvadeSideWeaveLeft,
        Self::EvadeSideWeaveRight,
        Self::EvadeWardenBarge,
        Self::EvadeStalkerTumble,
        Self::EvadeEmberPhaseShift,
        Self::EvadeSoulbinderGlide,
        Self::AttackRecoveryLight,
        Self::AttackRecoveryHeavy,
        Self::HitFlinchFront,
        Self::HitFlinchBack,
        Self::HitFlinchLeft,
        Self::HitFlinchRight,
        Self::HitHeavyRecoil,
        Self::HitGuardBlock,
        Self::HitEvadeGraze,
        Self::StaggerDazedLoop,
        Self::StaggerKneelCollapse,
        Self::KnockbackLaunch,
        Self::KnockbackLandRecover,
        Self::HitStopFreezeFrame,
        Self::StatusBurningFlail,
        Self::StatusChilledShiver,
        Self::StatusFrozenRigid,
        Self::StatusJailedStrain,
        Self::StatusBleedingClutch,
        Self::StatusVulnerableExposed,
        Self::StatusPoisonedHunch,
        Self::StatusCursedTremor,
        Self::StatusFearedCower,
        Self::StatusSilencedClutchThroat,
        Self::StatusExhaustedGasp,
        Self::StatusBlindGrope,
        Self::DrinkHealthPotion,
        Self::DrinkElixirFlask,
        Self::ActivateSurgeRoar,
        Self::ActivateGloryStride,
        Self::ActivateConduitHover,
        Self::InteractChestReach,
        Self::InteractShrinePray,
        Self::InteractLoreInspect,
        Self::LevelUpTriumph,
        Self::DeathCollapse,
        Self::EnemyAlertStartle,
        Self::EnemyMeleeSwing,
        Self::EnemyHeavyOverhead,
        Self::EnemyRangedDraw,
        Self::EnemyShockwaveSlam,
        Self::BossEntranceRoar,
        Self::BossPhaseShiftAscend,
        Self::BossBellTollSwing,
        Self::SummonSpiritHoverSlash,
        Self::SummonSpiritPhantomLunge,
        Self::SummonSpiritSpectralWave,
        Self::SummonBoneGuardBrace,
        Self::SummonBoneTauntSlam,
        Self::CompanionVanguardCharge,
        Self::CompanionGuardianAegis,
        Self::CoopRallyCall,
        Self::CoopSalute,
        Self::CoopCheer,
        Self::CoopPointTarget,
        Self::NpcMerchantBeckon,
    ];

    pub const COOP_EMOTES: [MotionId; 4] = [
        Self::CoopRallyCall,
        Self::CoopSalute,
        Self::CoopCheer,
        Self::CoopPointTarget,
    ];

    pub fn spec(self) -> MotionSpec {
        use MotionCategory as C;
        let (category, name_zh, name_en, duration_secs, looping, priority) = match self {
            // Locomotion & Stance
            Self::IdleBreathe => (C::Locomotion, "待机呼吸", "Idle Breathe", 2.4, true, 1),
            Self::IdleBattleGuard => (C::Locomotion, "战备戒备", "Battle Guard", 1.8, true, 2),
            Self::IdleExhausted => (C::Locomotion, "重伤喘息", "Exhausted Idle", 1.6, true, 3),
            Self::IdleWardenHeroic => {
                (C::Locomotion, "守卫持剑立姿", "Warden Stance", 2.2, true, 2)
            }
            Self::IdleStalkerCrouch => (
                C::Locomotion,
                "猎手潜伏蹲姿",
                "Stalker Crouch",
                2.0,
                true,
                2,
            ),
            Self::IdleEmberHover => (C::Locomotion, "余烬悬浮冥想", "Ember Hover", 2.4, true, 2),
            Self::IdleSoulbinderChannel => (
                C::Locomotion,
                "魂缚通灵立姿",
                "Soulbinder Stance",
                2.3,
                true,
                2,
            ),
            Self::WalkNormal => (C::Locomotion, "稳步前行", "Walk", 0.9, true, 4),
            Self::RunSprint => (C::Locomotion, "疾奔冲刺", "Sprint", 0.6, true, 5),
            Self::WalkExhausted => (C::Locomotion, "重伤蹒跚", "Exhausted Walk", 1.1, true, 5),
            Self::StrafeLeft => (C::Locomotion, "左翼侧步", "Strafe Left", 0.7, true, 5),
            Self::StrafeRight => (C::Locomotion, "右翼侧步", "Strafe Right", 0.7, true, 5),
            Self::TurnPivotLean => (C::Locomotion, "急转压弯", "Turn Pivot", 0.4, false, 5),
            Self::StopSkidRecover => (C::Locomotion, "急停稳姿", "Skid Stop", 0.32, false, 6),

            // Charge, Windup & Channel
            Self::ChargeHeavyMelee => (C::ChargeWindup, "重剑蓄力", "Heavy Charge", 0.85, true, 35),
            Self::ChargeOverheadCleave => (
                C::ChargeWindup,
                "高举劈斩蓄力",
                "Overhead Charge",
                0.90,
                true,
                35,
            ),
            Self::ChargeBowDraw => (C::ChargeWindup, "满弓蓄力", "Full Bow Draw", 0.85, true, 35),
            Self::ChargeBowSkyward => (
                C::ChargeWindup,
                "箭雨仰天蓄力",
                "Skyward Draw",
                0.75,
                true,
                35,
            ),
            Self::ChargeStaffSkyward => (
                C::ChargeWindup,
                "陨星法杖蓄力",
                "Meteor Channel",
                0.85,
                true,
                35,
            ),
            Self::ChargeGroundSlam => (C::ChargeWindup, "蓄劲砸地", "Slam Windup", 0.70, true, 35),
            Self::ChargeShieldBrace => (
                C::ChargeWindup,
                "举盾格挡蓄势",
                "Shield Brace",
                0.80,
                true,
                35,
            ),
            Self::ChargeSummonRitual => (
                C::ChargeWindup,
                "通灵召骸蓄力",
                "Summon Ritual",
                0.85,
                true,
                35,
            ),
            Self::ChargePortalChannel => (
                C::ChargeWindup,
                "回城传送引导",
                "Portal Channel",
                1.20,
                true,
                36,
            ),
            Self::ChargeSurgeAwaken => {
                (C::ChargeWindup, "奔涌蓄能", "Surge Charge", 0.65, false, 38)
            }
            Self::ChargeBossGuillotine => (
                C::ChargeWindup,
                "处刑刀高举蓄力",
                "Guillotine Windup",
                0.95,
                true,
                40,
            ),
            Self::ChargeBossCrescendo => (
                C::ChargeWindup,
                "空洞咏叹浮空蓄力",
                "Crescendo Channel",
                1.40,
                true,
                40,
            ),

            // Class Combat
            Self::WardenSlash1 => (
                C::ClassCombat,
                "守卫·斜斩",
                "Warden Slash I",
                0.38,
                false,
                30,
            ),
            Self::WardenSlash2Reverse => (
                C::ClassCombat,
                "守卫·回身反劈",
                "Warden Reverse II",
                0.40,
                false,
                30,
            ),
            Self::WardenSlash3Overhead => (
                C::ClassCombat,
                "守卫·力劈华山",
                "Warden Overhead III",
                0.48,
                false,
                32,
            ),
            Self::WardenWhirlwindSpin => (
                C::ClassCombat,
                "守卫·旋风斩",
                "Whirlwind Spin",
                0.56,
                false,
                34,
            ),
            Self::WardenLeapSlamAir => (
                C::ClassCombat,
                "守卫·跳劈猛击",
                "Leap Slam",
                0.52,
                false,
                34,
            ),
            Self::WardenSolarCleave => (
                C::ClassCombat,
                "守卫·圣焰重劈",
                "Solar Cleave",
                0.54,
                false,
                36,
            ),
            Self::StalkerQuickShot => (C::ClassCombat, "猎手·速射", "Quick Shot", 0.34, false, 30),
            Self::StalkerTwinSnap => (
                C::ClassCombat,
                "猎手·双箭连发",
                "Twin Snap",
                0.38,
                false,
                30,
            ),
            Self::StalkerPiercingRelease => (
                C::ClassCombat,
                "猎手·穿刺强射",
                "Piercing Shot",
                0.46,
                false,
                32,
            ),
            Self::StalkerVaultBackflip => (
                C::ClassCombat,
                "猎手·后空翻跃",
                "Vault Backflip",
                0.44,
                false,
                34,
            ),
            Self::StalkerShadowLunge => (
                C::ClassCombat,
                "猎手·影袭突进",
                "Shadow Lunge",
                0.42,
                false,
                34,
            ),
            Self::StalkerPhantomVolley => (
                C::ClassCombat,
                "猎手·追魂连矢",
                "Phantom Volley",
                0.48,
                false,
                36,
            ),
            Self::EmberStaffFlick => (
                C::ClassCombat,
                "余烬·挥杖弹射",
                "Staff Flick",
                0.36,
                false,
                30,
            ),
            Self::EmberTwinBoltThrust => (
                C::ClassCombat,
                "余烬·双焰推进",
                "Twin Bolt Thrust",
                0.40,
                false,
                30,
            ),
            Self::EmberCinderfallSlam => (
                C::ClassCombat,
                "余烬·陨星顿杖",
                "Cinderfall Slam",
                0.52,
                false,
                34,
            ),
            Self::EmberFrostRingBurst => (
                C::ClassCombat,
                "余烬·霜环爆发",
                "Frost Ring Burst",
                0.48,
                false,
                34,
            ),
            Self::EmberAshStepBlink => (
                C::ClassCombat,
                "余烬·灰步瞬移",
                "Ash Step Blink",
                0.34,
                false,
                34,
            ),
            Self::EmberOverloadLevitate => (
                C::ClassCombat,
                "余烬·过载浮空",
                "Cinder Overload",
                0.58,
                false,
                36,
            ),
            Self::SoulbinderReapArc => {
                (C::ClassCombat, "魂缚·镰弧收割", "Reap Arc", 0.42, false, 30)
            }
            Self::SoulbinderCommandPoint => (
                C::ClassCombat,
                "魂缚·指敌号令",
                "Soul Command",
                0.38,
                false,
                32,
            ),
            Self::SoulbinderRaiseGuard => (
                C::ClassCombat,
                "魂缚·唤骨起阵",
                "Raise Bone Guard",
                0.56,
                false,
                34,
            ),
            Self::SoulbinderSpiritSurge => (
                C::ClassCombat,
                "魂缚·灵潮涌动",
                "Spirit Surge",
                0.54,
                false,
                34,
            ),
            Self::SoulbinderSacrificeClasp => (
                C::ClassCombat,
                "魂缚·献祭合掌",
                "Sacrifice Clasp",
                0.46,
                false,
                34,
            ),
            Self::SoulbinderRecallPulse => (
                C::ClassCombat,
                "魂缚·召回脉动",
                "Recall Pulse",
                0.40,
                false,
                32,
            ),

            // Evade & Recovery
            Self::EvadeForwardRoll => (
                C::EvadeRecovery,
                "前滚翻闪避",
                "Forward Roll",
                0.28,
                false,
                42,
            ),
            Self::EvadeBackstepHop => (
                C::EvadeRecovery,
                "后撤步闪避",
                "Backstep Hop",
                0.26,
                false,
                42,
            ),
            Self::EvadeSideWeaveLeft => (
                C::EvadeRecovery,
                "左侧滑步",
                "Side Weave L",
                0.26,
                false,
                42,
            ),
            Self::EvadeSideWeaveRight => (
                C::EvadeRecovery,
                "右侧滑步",
                "Side Weave R",
                0.26,
                false,
                42,
            ),
            Self::EvadeWardenBarge => (
                C::EvadeRecovery,
                "守卫·铁肩冲撞",
                "Warden Barge",
                0.28,
                false,
                42,
            ),
            Self::EvadeStalkerTumble => (
                C::EvadeRecovery,
                "猎手·低身翻滚",
                "Stalker Tumble",
                0.26,
                false,
                42,
            ),
            Self::EvadeEmberPhaseShift => (
                C::EvadeRecovery,
                "余烬·相位折跃",
                "Ember Phase Shift",
                0.26,
                false,
                42,
            ),
            Self::EvadeSoulbinderGlide => (
                C::EvadeRecovery,
                "魂缚·幽影滑掠",
                "Spectral Glide",
                0.28,
                false,
                42,
            ),
            Self::AttackRecoveryLight => (
                C::EvadeRecovery,
                "轻击收招",
                "Light Recovery",
                0.22,
                false,
                16,
            ),
            Self::AttackRecoveryHeavy => (
                C::EvadeRecovery,
                "重击硬直收招",
                "Heavy Recovery",
                0.36,
                false,
                20,
            ),

            // Hitstun, Stagger & Knockback
            Self::HitFlinchFront => (
                C::HitstunStagger,
                "正面受击后仰",
                "Hit Flinch Front",
                0.26,
                false,
                52,
            ),
            Self::HitFlinchBack => (
                C::HitstunStagger,
                "背后受击前倾",
                "Hit Flinch Back",
                0.26,
                false,
                52,
            ),
            Self::HitFlinchLeft => (
                C::HitstunStagger,
                "左侧受击歪身",
                "Hit Flinch Left",
                0.26,
                false,
                52,
            ),
            Self::HitFlinchRight => (
                C::HitstunStagger,
                "右侧受击歪身",
                "Hit Flinch Right",
                0.26,
                false,
                52,
            ),
            Self::HitHeavyRecoil => (
                C::HitstunStagger,
                "重创僵直踉跄",
                "Heavy Hitstun",
                0.42,
                false,
                58,
            ),
            Self::HitGuardBlock => (
                C::HitstunStagger,
                "护盾格挡震退",
                "Ward Block Recoil",
                0.24,
                false,
                50,
            ),
            Self::HitEvadeGraze => (
                C::HitstunStagger,
                "险象擦身闪过",
                "Evade Graze",
                0.22,
                false,
                48,
            ),
            Self::StaggerDazedLoop => (
                C::HitstunStagger,
                "眩晕僵直晃动",
                "Stagger Dazed",
                1.10,
                true,
                68,
            ),
            Self::StaggerKneelCollapse => (
                C::HitstunStagger,
                "破防单膝跪倒",
                "Stagger Kneel",
                1.30,
                true,
                72,
            ),
            Self::KnockbackLaunch => (
                C::HitstunStagger,
                "击飞后仰腾空",
                "Knockback Launch",
                0.38,
                false,
                64,
            ),
            Self::KnockbackLandRecover => (
                C::HitstunStagger,
                "击退落地稳姿",
                "Knockback Recover",
                0.34,
                false,
                54,
            ),
            Self::HitStopFreezeFrame => (
                C::HitstunStagger,
                "命中顿帧定格",
                "HitStop Freeze",
                0.12,
                false,
                60,
            ),

            // Status Ailments
            Self::StatusBurningFlail => (
                C::StatusAilment,
                "燃烧·扑火挣扎",
                "Burning Flail",
                0.55,
                true,
                62,
            ),
            Self::StatusChilledShiver => (
                C::StatusAilment,
                "冰寒·抱臂颤抖",
                "Chilled Shiver",
                0.72,
                true,
                56,
            ),
            Self::StatusFrozenRigid => (
                C::StatusAilment,
                "冰封·僵直冻滞",
                "Frozen Rigid",
                0.90,
                true,
                70,
            ),
            Self::StatusJailedStrain => (
                C::StatusAilment,
                "禁锢·锁链挣扎",
                "Jailed Strain",
                0.68,
                true,
                64,
            ),
            Self::StatusBleedingClutch => (
                C::StatusAilment,
                "流血·捂胸蹒跚",
                "Bleeding Clutch",
                0.85,
                true,
                55,
            ),
            Self::StatusVulnerableExposed => (
                C::StatusAilment,
                "破甲·门户大开",
                "Vulnerable Exposed",
                0.80,
                true,
                54,
            ),
            Self::StatusPoisonedHunch => (
                C::StatusAilment,
                "中毒·佝偻抽搐",
                "Poisoned Hunch",
                0.90,
                true,
                55,
            ),
            Self::StatusCursedTremor => (
                C::StatusAilment,
                "诅咒·诡异震颤",
                "Cursed Tremor",
                0.76,
                true,
                56,
            ),
            Self::StatusFearedCower => (
                C::StatusAilment,
                "恐惧·抱头退缩",
                "Feared Cower",
                0.70,
                true,
                62,
            ),
            Self::StatusSilencedClutchThroat => (
                C::StatusAilment,
                "沉默·抬手扼喉",
                "Silenced Clutch",
                0.82,
                true,
                58,
            ),
            Self::StatusExhaustedGasp => (
                C::StatusAilment,
                "力竭·拄兵喘息",
                "Exhausted Gasp",
                1.25,
                true,
                46,
            ),
            Self::StatusBlindGrope => (
                C::StatusAilment,
                "致盲·盲目挥手",
                "Blind Grope",
                0.88,
                true,
                56,
            ),

            // Buffs, Consumables & World Interactions
            Self::DrinkHealthPotion => (
                C::BuffInteraction,
                "饮用生命药水",
                "Drink Potion",
                0.48,
                false,
                44,
            ),
            Self::DrinkElixirFlask => (
                C::BuffInteraction,
                "畅饮强化灵药",
                "Drink Elixir",
                0.52,
                false,
                44,
            ),
            Self::ActivateSurgeRoar => (
                C::BuffInteraction,
                "奔涌战吼爆发",
                "Surge Roar",
                0.58,
                false,
                48,
            ),
            Self::ActivateGloryStride => (
                C::BuffInteraction,
                "荣耀威严昂首",
                "Glory Stride",
                0.50,
                false,
                24,
            ),
            Self::ActivateConduitHover => (
                C::BuffInteraction,
                "导能雷光浮空",
                "Conduit Hover",
                0.55,
                false,
                26,
            ),
            Self::InteractChestReach => (
                C::BuffInteraction,
                "俯身开启宝箱",
                "Open Chest",
                0.50,
                false,
                40,
            ),
            Self::InteractShrinePray => (
                C::BuffInteraction,
                "祭坛祈祷颔首",
                "Shrine Pray",
                0.65,
                false,
                40,
            ),
            Self::InteractLoreInspect => (
                C::BuffInteraction,
                "研读遗迹典籍",
                "Inspect Lore",
                0.62,
                false,
                38,
            ),
            Self::LevelUpTriumph => (
                C::BuffInteraction,
                "升级举兵欢呼",
                "Level Up Triumph",
                0.76,
                false,
                49,
            ),
            Self::DeathCollapse => (
                C::BuffInteraction,
                "战败力竭倒地",
                "Death Collapse",
                0.90,
                false,
                99,
            ),

            // Enemy & Boss Specialized
            Self::EnemyAlertStartle => (
                C::EnemyBoss,
                "敌人·惊觉起势",
                "Enemy Alert",
                0.36,
                false,
                32,
            ),
            Self::EnemyMeleeSwing => (
                C::EnemyBoss,
                "敌人·凶猛挥砍",
                "Enemy Melee Swing",
                0.44,
                false,
                36,
            ),
            Self::EnemyHeavyOverhead => (
                C::EnemyBoss,
                "敌人·重锤砸击",
                "Enemy Heavy Overhead",
                0.56,
                false,
                38,
            ),
            Self::EnemyRangedDraw => (
                C::EnemyBoss,
                "敌人·瞄准齐射",
                "Enemy Ranged Draw",
                0.48,
                false,
                36,
            ),
            Self::EnemyShockwaveSlam => (
                C::EnemyBoss,
                "敌人·震地冲击",
                "Enemy Shockwave Slam",
                0.60,
                false,
                38,
            ),
            Self::BossEntranceRoar => (
                C::EnemyBoss,
                "Boss·登场威压怒吼",
                "Boss Entrance Roar",
                0.95,
                false,
                78,
            ),
            Self::BossPhaseShiftAscend => (
                C::EnemyBoss,
                "Boss·二阶段升空爆气",
                "Boss Phase Ascend",
                1.05,
                false,
                80,
            ),
            Self::BossBellTollSwing => (
                C::EnemyBoss,
                "钟楼守卫·重锤敲钟",
                "Boss Bell Toll",
                0.68,
                false,
                44,
            ),

            // Summons, Companion & Co-op
            Self::SummonSpiritHoverSlash => (
                C::SummonCompanion,
                "契灵·浮空横斩",
                "Spirit Hover Slash",
                0.40,
                false,
                36,
            ),
            Self::SummonSpiritPhantomLunge => (
                C::SummonCompanion,
                "契灵·幽影突刺",
                "Spirit Phantom Lunge",
                0.44,
                false,
                38,
            ),
            Self::SummonSpiritSpectralWave => (
                C::SummonCompanion,
                "契灵·双臂灵波",
                "Spirit Spectral Wave",
                0.50,
                false,
                38,
            ),
            Self::SummonBoneGuardBrace => (
                C::SummonCompanion,
                "骸卫·举盾守御",
                "Bone Guard Brace",
                0.80,
                true,
                30,
            ),
            Self::SummonBoneTauntSlam => (
                C::SummonCompanion,
                "骸卫·嘲讽震地",
                "Bone Taunt Slam",
                0.54,
                false,
                40,
            ),
            Self::CompanionVanguardCharge => (
                C::SummonCompanion,
                "伙伴·先锋冲锋",
                "Vanguard Charge",
                0.46,
                false,
                38,
            ),
            Self::CompanionGuardianAegis => (
                C::SummonCompanion,
                "伙伴·圣盾守护",
                "Guardian Aegis",
                0.52,
                false,
                38,
            ),
            Self::CoopRallyCall => (
                C::SummonCompanion,
                "协作·集结号令",
                "Co-op Rally Call",
                0.62,
                false,
                42,
            ),

            // Social Emotes & NPC
            Self::CoopSalute => (
                C::CoopSocialNpc,
                "协作·战友致敬",
                "Co-op Salute",
                0.78,
                false,
                28,
            ),
            Self::CoopCheer => (
                C::CoopSocialNpc,
                "协作·胜利欢呼",
                "Co-op Cheer",
                0.84,
                false,
                28,
            ),
            Self::CoopPointTarget => (
                C::CoopSocialNpc,
                "协作·集火标点",
                "Co-op Point Target",
                0.64,
                false,
                28,
            ),
            Self::NpcMerchantBeckon => (
                C::CoopSocialNpc,
                "军需官·招手迎客",
                "Merchant Beckon",
                1.80,
                true,
                14,
            ),
        };
        MotionSpec {
            id: self,
            category,
            name_zh,
            name_en,
            duration_secs,
            looping,
            priority,
        }
    }

    pub fn label(self, chinese: bool) -> &'static str {
        let s = self.spec();
        if chinese { s.name_zh } else { s.name_en }
    }
}

/// Full-body procedural pose delta applied on top of or blended with base rigs.
/// All angles are in radians; all translations are in local meters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArticulatedPose {
    pub body_y: f32,
    pub body_pitch: f32,
    pub body_yaw: f32,
    pub body_roll: f32,
    pub body_scale_delta: Vec3,
    pub shoulder_pitch: f32,
    pub shoulder_yaw: f32,
    pub shoulder_roll: f32,
    pub left_arm_rot: Vec3,
    pub right_arm_rot: Vec3,
    pub left_leg_rot: Vec3,
    pub right_leg_rot: Vec3,
    pub weapon_offset: Vec3,
    pub weapon_rot: Vec3,
}

impl Default for ArticulatedPose {
    fn default() -> Self {
        Self::ZERO
    }
}

impl ArticulatedPose {
    pub const ZERO: Self = Self {
        body_y: 0.0,
        body_pitch: 0.0,
        body_yaw: 0.0,
        body_roll: 0.0,
        body_scale_delta: Vec3::ZERO,
        shoulder_pitch: 0.0,
        shoulder_yaw: 0.0,
        shoulder_roll: 0.0,
        left_arm_rot: Vec3::ZERO,
        right_arm_rot: Vec3::ZERO,
        left_leg_rot: Vec3::ZERO,
        right_leg_rot: Vec3::ZERO,
        weapon_offset: Vec3::ZERO,
        weapon_rot: Vec3::ZERO,
    };

    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            body_y: self.body_y.lerp(other.body_y, t),
            body_pitch: self.body_pitch.lerp(other.body_pitch, t),
            body_yaw: self.body_yaw.lerp(other.body_yaw, t),
            body_roll: self.body_roll.lerp(other.body_roll, t),
            body_scale_delta: self.body_scale_delta.lerp(other.body_scale_delta, t),
            shoulder_pitch: self.shoulder_pitch.lerp(other.shoulder_pitch, t),
            shoulder_yaw: self.shoulder_yaw.lerp(other.shoulder_yaw, t),
            shoulder_roll: self.shoulder_roll.lerp(other.shoulder_roll, t),
            left_arm_rot: self.left_arm_rot.lerp(other.left_arm_rot, t),
            right_arm_rot: self.right_arm_rot.lerp(other.right_arm_rot, t),
            left_leg_rot: self.left_leg_rot.lerp(other.left_leg_rot, t),
            right_leg_rot: self.right_leg_rot.lerp(other.right_leg_rot, t),
            weapon_offset: self.weapon_offset.lerp(other.weapon_offset, t),
            weapon_rot: self.weapon_rot.lerp(other.weapon_rot, t),
        }
    }

    pub fn add_weighted(self, layer: Self, weight: f32) -> Self {
        let w = weight.clamp(0.0, 1.5);
        Self {
            body_y: self.body_y + layer.body_y * w,
            body_pitch: self.body_pitch + layer.body_pitch * w,
            body_yaw: self.body_yaw + layer.body_yaw * w,
            body_roll: self.body_roll + layer.body_roll * w,
            body_scale_delta: self.body_scale_delta + layer.body_scale_delta * w,
            shoulder_pitch: self.shoulder_pitch + layer.shoulder_pitch * w,
            shoulder_yaw: self.shoulder_yaw + layer.shoulder_yaw * w,
            shoulder_roll: self.shoulder_roll + layer.shoulder_roll * w,
            left_arm_rot: self.left_arm_rot + layer.left_arm_rot * w,
            right_arm_rot: self.right_arm_rot + layer.right_arm_rot * w,
            left_leg_rot: self.left_leg_rot + layer.left_leg_rot * w,
            right_leg_rot: self.right_leg_rot + layer.right_leg_rot * w,
            weapon_offset: self.weapon_offset + layer.weapon_offset * w,
            weapon_rot: self.weapon_rot + layer.weapon_rot * w,
        }
    }
}

/// Evaluate the deterministic articulated pose for any of the 114 [`MotionId`]
/// clips at normalized progress `p` in `0.0..=1.0` (or continuous phase for loops).
pub fn sample_motion_pose(id: MotionId, progress: f32, elapsed_secs: f32) -> ArticulatedPose {
    let p = progress.clamp(0.0, 1.0);
    let tau = std::f32::consts::TAU;
    let pi = std::f32::consts::PI;
    // Smooth bell curve peaking at 0.5, zero at endpoints
    let bell = (p * pi).sin();
    // Sharp attack impulse peaking near 0.35
    let snap = if p < 0.35 {
        (p / 0.35).powf(0.65)
    } else {
        ((1.0 - p) / 0.65).max(0.0).powf(1.35)
    };
    // Windup-to-strike curve: negative coil (<0.38), positive release (>0.38)
    let coil_strike = if p < 0.38 {
        -(p / 0.38 * pi * 0.5).sin()
    } else {
        let r = ((p - 0.38) / 0.62).clamp(0.0, 1.0);
        (r * pi).sin()
    };
    let cyc = (elapsed_secs * tau).sin();
    let cyc_cos = (elapsed_secs * tau).cos();
    let fast = (elapsed_secs * 18.0).sin();
    let shiver = (elapsed_secs * 34.0).sin();

    let mut pose = ArticulatedPose::ZERO;
    match id {
        // ============================================================
        // 1. LOCOMOTION & STANCE (14)
        // ============================================================
        MotionId::IdleBreathe => {
            let b = (elapsed_secs * 2.2).sin();
            pose.body_y = b * 0.018;
            pose.body_pitch = b * 0.025;
            pose.left_arm_rot = Vec3::new(0.06 + b * 0.04, 0.0, 0.05);
            pose.right_arm_rot = Vec3::new(-0.08 + b * 0.04, 0.0, -0.05);
            pose.body_scale_delta = Vec3::new(b * 0.012, b * 0.015, b * 0.012);
        }
        MotionId::IdleBattleGuard => {
            let b = (elapsed_secs * 3.4).sin();
            pose.body_y = -0.06 + b * 0.016;
            pose.body_pitch = -0.14 + b * 0.03;
            pose.body_yaw = 0.16;
            pose.left_arm_rot = Vec3::new(-0.62, 0.25, 0.22);
            pose.right_arm_rot = Vec3::new(-0.48, -0.18, -0.15);
            pose.left_leg_rot = Vec3::new(-0.18, 0.0, 0.08);
            pose.right_leg_rot = Vec3::new(0.20, 0.0, -0.08);
            pose.weapon_offset = Vec3::new(-0.06, 0.10, 0.14);
        }
        MotionId::IdleExhausted => {
            let pant = (elapsed_secs * 4.6).sin();
            pose.body_y = -0.14 + pant * 0.028;
            pose.body_pitch = -0.34 + pant * 0.06;
            pose.body_roll = (elapsed_secs * 1.9).sin() * 0.07;
            pose.shoulder_pitch = -0.18;
            pose.left_arm_rot = Vec3::new(0.32 + pant * 0.08, 0.0, 0.14);
            pose.right_arm_rot = Vec3::new(0.45, 0.0, -0.12);
            pose.left_leg_rot = Vec3::new(-0.22, 0.0, 0.12);
            pose.right_leg_rot = Vec3::new(0.18, 0.0, -0.12);
            pose.weapon_offset = Vec3::new(0.06, -0.22, 0.08);
            pose.weapon_rot = Vec3::new(0.42, 0.0, -0.18);
        }
        MotionId::IdleWardenHeroic => {
            let b = (elapsed_secs * 2.0).sin();
            pose.body_y = b * 0.016;
            pose.body_yaw = -0.10;
            pose.shoulder_yaw = -0.08;
            pose.left_arm_rot = Vec3::new(-0.26, 0.15, 0.18);
            pose.right_arm_rot = Vec3::new(-0.32, -0.12, -0.10);
            pose.weapon_offset = Vec3::new(0.06, 0.06 + b * 0.02, 0.04);
            pose.weapon_rot = Vec3::new(-0.14, 0.08, -0.08);
        }
        MotionId::IdleStalkerCrouch => {
            let b = (elapsed_secs * 2.9).sin();
            pose.body_y = -0.10 + b * 0.015;
            pose.body_pitch = -0.22;
            pose.body_yaw = 0.24;
            pose.left_arm_rot = Vec3::new(-0.52, 0.30, 0.15);
            pose.right_arm_rot = Vec3::new(-0.38, -0.25, -0.12);
            pose.left_leg_rot = Vec3::new(-0.26, 0.0, 0.10);
            pose.right_leg_rot = Vec3::new(0.24, 0.0, -0.10);
            pose.weapon_offset = Vec3::new(-0.08, -0.04, 0.12);
        }
        MotionId::IdleEmberHover => {
            let h = (elapsed_secs * 2.5).sin();
            pose.body_y = 0.08 + h * 0.045;
            pose.body_pitch = 0.05 + h * 0.03;
            pose.left_arm_rot = Vec3::new(-0.35 + h * 0.10, 0.20, 0.34);
            pose.right_arm_rot = Vec3::new(-0.42 - h * 0.08, -0.15, -0.24);
            pose.left_leg_rot = Vec3::new(0.14, 0.0, 0.06);
            pose.right_leg_rot = Vec3::new(0.22, 0.0, -0.06);
            pose.weapon_offset = Vec3::new(0.04, 0.16 + h * 0.04, 0.06);
        }
        MotionId::IdleSoulbinderChannel => {
            let h = (elapsed_secs * 2.1).sin();
            pose.body_y = 0.04 + h * 0.035;
            pose.body_roll = (elapsed_secs * 1.4).cos() * 0.045;
            pose.left_arm_rot = Vec3::new(-0.56, 0.28, 0.28 + h * 0.08);
            pose.right_arm_rot = Vec3::new(-0.30, -0.14, -0.16);
            pose.weapon_offset = Vec3::new(0.02, 0.12 + h * 0.03, 0.08);
            pose.weapon_rot = Vec3::new(-0.18, 0.0, 0.10 * h);
        }
        MotionId::WalkNormal => {
            let s = (elapsed_secs * 7.2).sin();
            pose.body_y = s.abs() * 0.032;
            pose.body_pitch = -0.07;
            pose.body_roll = s * 0.045;
            pose.left_leg_rot = Vec3::new(s * 0.42, 0.0, 0.0);
            pose.right_leg_rot = Vec3::new(-s * 0.42, 0.0, 0.0);
            pose.left_arm_rot = Vec3::new(-s * 0.36, 0.0, 0.06);
            pose.right_arm_rot = Vec3::new(s * 0.32, 0.0, -0.06);
        }
        MotionId::RunSprint => {
            let s = (elapsed_secs * 11.4).sin();
            pose.body_y = -0.04 + s.abs() * 0.065;
            pose.body_pitch = -0.26;
            pose.body_roll = s * 0.085;
            pose.shoulder_yaw = -s * 0.16;
            pose.left_leg_rot = Vec3::new(s * 0.72, 0.0, 0.0);
            pose.right_leg_rot = Vec3::new(-s * 0.72, 0.0, 0.0);
            pose.left_arm_rot = Vec3::new(-s * 0.68 - 0.18, 0.0, 0.12);
            pose.right_arm_rot = Vec3::new(s * 0.56 - 0.14, 0.0, -0.14);
            pose.weapon_offset = Vec3::new(0.08, -0.04, -0.12);
        }
        MotionId::WalkExhausted => {
            let s = (elapsed_secs * 4.8).sin();
            pose.body_y = -0.11 + s.abs() * 0.025;
            pose.body_pitch = -0.32;
            pose.body_roll = 0.12 + s * 0.14;
            pose.left_leg_rot = Vec3::new(s * 0.28, 0.0, 0.08);
            pose.right_leg_rot = Vec3::new(-s * 0.22, 0.0, -0.08);
            pose.left_arm_rot = Vec3::new(0.28, 0.35, -0.22);
            pose.right_arm_rot = Vec3::new(0.35 + s * 0.12, 0.0, -0.14);
            pose.weapon_offset = Vec3::new(0.05, -0.18, 0.0);
        }
        MotionId::StrafeLeft => {
            let s = (elapsed_secs * 8.6).sin();
            pose.body_roll = 0.18;
            pose.body_yaw = 0.22;
            pose.left_leg_rot = Vec3::new(s * 0.35, 0.0, 0.24);
            pose.right_leg_rot = Vec3::new(-s * 0.35, 0.0, -0.12);
            pose.left_arm_rot = Vec3::new(-0.30, 0.20, 0.26);
        }
        MotionId::StrafeRight => {
            let s = (elapsed_secs * 8.6).sin();
            pose.body_roll = -0.18;
            pose.body_yaw = -0.22;
            pose.left_leg_rot = Vec3::new(s * 0.35, 0.0, 0.12);
            pose.right_leg_rot = Vec3::new(-s * 0.35, 0.0, -0.24);
            pose.right_arm_rot = Vec3::new(-0.30, -0.20, -0.26);
        }
        MotionId::TurnPivotLean => {
            pose.body_roll = 0.24 * bell;
            pose.body_yaw = -0.28 * bell;
            pose.body_y = -0.05 * bell;
            pose.left_arm_rot = Vec3::new(-0.20 * bell, 0.0, 0.32 * bell);
            pose.right_arm_rot = Vec3::new(-0.20 * bell, 0.0, -0.26 * bell);
        }
        MotionId::StopSkidRecover => {
            pose.body_y = -0.09 * bell;
            pose.body_pitch = 0.22 * bell;
            pose.left_leg_rot = Vec3::new(-0.42 * bell, 0.0, 0.10 * bell);
            pose.right_leg_rot = Vec3::new(0.32 * bell, 0.0, -0.10 * bell);
            pose.left_arm_rot = Vec3::new(0.35 * bell, 0.0, 0.42 * bell);
            pose.right_arm_rot = Vec3::new(0.25 * bell, 0.0, -0.35 * bell);
        }

        // ============================================================
        // 2. CHARGE, WINDUP & CHANNEL (12) — 蓄力动作！
        // ============================================================
        MotionId::ChargeHeavyMelee => {
            let tension = p.clamp(0.15, 1.0);
            let tremble = fast * 0.028 * tension;
            pose.body_y = -0.14 * tension;
            pose.body_pitch = -0.16 * tension;
            pose.body_yaw = -0.68 * tension + tremble;
            pose.body_roll = -0.12 * tension;
            pose.body_scale_delta = Vec3::new(0.06 * tension, -0.08 * tension, 0.06 * tension);
            pose.shoulder_yaw = -0.38 * tension;
            pose.left_arm_rot = Vec3::new(-0.85 * tension, 0.45 * tension, 0.22 * tension);
            pose.right_arm_rot = Vec3::new(-1.35 * tension, -0.45 * tension, -0.42 * tension);
            pose.left_leg_rot = Vec3::new(-0.34 * tension, 0.0, 0.14 * tension);
            pose.right_leg_rot = Vec3::new(0.36 * tension, 0.0, -0.14 * tension);
            pose.weapon_offset =
                Vec3::new(0.28 * tension, 0.26 * tension + tremble, -0.36 * tension);
            pose.weapon_rot =
                Vec3::new(-0.68 * tension, -0.55 * tension, -0.92 * tension + tremble);
        }
        MotionId::ChargeOverheadCleave => {
            let tension = p.clamp(0.15, 1.0);
            let tremble = fast * 0.032 * tension;
            pose.body_y = -0.08 * tension;
            pose.body_pitch = 0.28 * tension + tremble;
            pose.body_scale_delta = Vec3::new(0.05 * tension, -0.06 * tension, 0.05 * tension);
            pose.left_arm_rot = Vec3::new(-2.15 * tension, 0.24 * tension, -0.18 * tension);
            pose.right_arm_rot = Vec3::new(-2.25 * tension, -0.24 * tension, 0.18 * tension);
            pose.left_leg_rot = Vec3::new(-0.28 * tension, 0.0, 0.12 * tension);
            pose.right_leg_rot = Vec3::new(0.30 * tension, 0.0, -0.12 * tension);
            pose.weapon_offset = Vec3::new(0.04, 0.68 * tension, -0.26 * tension);
            pose.weapon_rot = Vec3::new(-1.45 * tension + tremble, 0.0, 0.0);
        }
        MotionId::ChargeBowDraw => {
            let draw = p.clamp(0.15, 1.0);
            let tremble = fast * 0.022 * draw;
            pose.body_y = -0.09 * draw;
            pose.body_yaw = 0.68 * draw;
            pose.body_pitch = -0.08 * draw;
            pose.shoulder_yaw = 0.34 * draw;
            // Bow arm rigid forward, draw arm pulled tight to cheek
            pose.left_arm_rot = Vec3::new(-1.38 * draw, 0.22 * draw, 0.18 * draw);
            pose.right_arm_rot = Vec3::new(-1.24 * draw + tremble, -0.78 * draw, -0.62 * draw);
            pose.left_leg_rot = Vec3::new(-0.32 * draw, 0.0, 0.16 * draw);
            pose.right_leg_rot = Vec3::new(0.32 * draw, 0.0, -0.16 * draw);
            pose.weapon_offset = Vec3::new(-0.14 * draw, 0.20 * draw, 0.26 * draw);
            pose.weapon_rot = Vec3::new(-0.10 * draw, 0.38 * draw, 0.28 * draw + tremble);
        }
        MotionId::ChargeBowSkyward => {
            let draw = p.clamp(0.15, 1.0);
            pose.body_y = -0.06 * draw;
            pose.body_pitch = 0.34 * draw;
            pose.body_yaw = 0.45 * draw;
            pose.left_arm_rot = Vec3::new(-2.12 * draw, 0.18 * draw, 0.12 * draw);
            pose.right_arm_rot = Vec3::new(-1.62 * draw, -0.65 * draw, -0.45 * draw);
            pose.weapon_offset = Vec3::new(-0.08 * draw, 0.56 * draw, 0.18 * draw);
            pose.weapon_rot = Vec3::new(-0.92 * draw, 0.24 * draw, 0.18 * draw);
        }
        MotionId::ChargeStaffSkyward => {
            let ch = p.clamp(0.15, 1.0);
            let hover = (elapsed_secs * 9.0).sin() * 0.04 * ch;
            pose.body_y = 0.14 * ch + hover;
            pose.body_pitch = 0.22 * ch;
            pose.body_scale_delta = Vec3::new(-0.03 * ch, 0.06 * ch, -0.03 * ch);
            pose.left_arm_rot = Vec3::new(-1.75 * ch, 0.42 * ch, 0.62 * ch);
            pose.right_arm_rot = Vec3::new(-2.15 * ch, -0.20 * ch, -0.26 * ch);
            pose.left_leg_rot = Vec3::new(0.18 * ch, 0.0, 0.08 * ch);
            pose.right_leg_rot = Vec3::new(0.28 * ch, 0.0, -0.08 * ch);
            pose.weapon_offset = Vec3::new(0.10 * ch, 0.54 * ch + hover, 0.12 * ch);
            pose.weapon_rot = Vec3::new(-0.35 * ch, elapsed_secs * 3.0 * ch, 0.18 * ch);
        }
        MotionId::ChargeGroundSlam => {
            let ch = p.clamp(0.15, 1.0);
            pose.body_y = -0.22 * ch;
            pose.body_pitch = -0.34 * ch;
            pose.body_scale_delta = Vec3::new(0.09 * ch, -0.14 * ch, 0.09 * ch);
            pose.left_arm_rot = Vec3::new(-1.45 * ch, 0.22 * ch, 0.45 * ch);
            pose.right_arm_rot = Vec3::new(-1.45 * ch, -0.22 * ch, -0.45 * ch);
            pose.left_leg_rot = Vec3::new(-0.48 * ch, 0.0, 0.22 * ch);
            pose.right_leg_rot = Vec3::new(-0.48 * ch, 0.0, -0.22 * ch);
            pose.weapon_offset = Vec3::new(0.0, 0.34 * ch, -0.22 * ch);
        }
        MotionId::ChargeShieldBrace => {
            let ch = p.clamp(0.15, 1.0);
            pose.body_y = -0.12 * ch;
            pose.body_pitch = -0.16 * ch;
            pose.body_yaw = 0.28 * ch;
            pose.left_arm_rot = Vec3::new(-1.15 * ch, 0.58 * ch, -0.24 * ch);
            pose.right_arm_rot = Vec3::new(-0.62 * ch, -0.32 * ch, -0.35 * ch);
            pose.left_leg_rot = Vec3::new(-0.36 * ch, 0.0, 0.16 * ch);
            pose.right_leg_rot = Vec3::new(0.32 * ch, 0.0, -0.16 * ch);
            pose.weapon_offset = Vec3::new(-0.18 * ch, 0.12 * ch, 0.24 * ch);
        }
        MotionId::ChargeSummonRitual => {
            let ch = p.clamp(0.15, 1.0);
            let pulse = (elapsed_secs * 10.0).sin() * 0.05 * ch;
            pose.body_y = -0.16 * ch + pulse * 0.4;
            pose.body_pitch = -0.24 * ch;
            pose.left_arm_rot = Vec3::new(-1.28 * ch, 0.52 * ch, 0.72 * ch + pulse);
            pose.right_arm_rot = Vec3::new(-1.28 * ch, -0.52 * ch, -0.72 * ch - pulse);
            pose.left_leg_rot = Vec3::new(-0.44 * ch, 0.0, 0.18 * ch);
            pose.right_leg_rot = Vec3::new(0.38 * ch, 0.0, -0.18 * ch);
            pose.weapon_offset = Vec3::new(0.0, 0.38 * ch + pulse, 0.24 * ch);
            pose.weapon_rot = Vec3::new(-0.48 * ch, 0.0, pulse * 2.0);
        }
        MotionId::ChargePortalChannel => {
            let sway = (elapsed_secs * 5.0).sin();
            pose.body_y = 0.12 + sway * 0.05;
            pose.body_pitch = 0.14;
            pose.body_yaw = (elapsed_secs * 1.6).sin() * 0.18;
            pose.left_arm_rot = Vec3::new(-1.55, 0.35, 0.85 + sway * 0.12);
            pose.right_arm_rot = Vec3::new(-1.55, -0.35, -0.85 - sway * 0.12);
            pose.weapon_offset = Vec3::new(0.0, 0.42, 0.22);
        }
        MotionId::ChargeSurgeAwaken => {
            pose.body_y = -0.14 * (1.0 - p) + 0.18 * p;
            pose.body_pitch = -0.26 * (1.0 - p) + 0.34 * p;
            pose.body_scale_delta = Vec3::splat(0.12 * bell);
            pose.left_arm_rot = Vec3::new(-1.65 * bell, -0.45 * bell, 1.15 * bell);
            pose.right_arm_rot = Vec3::new(-1.65 * bell, 0.45 * bell, -1.15 * bell);
        }
        MotionId::ChargeBossGuillotine => {
            let ch = p.clamp(0.15, 1.0);
            pose.body_y = 0.08 * ch;
            pose.body_pitch = 0.32 * ch;
            pose.body_yaw = -0.52 * ch;
            pose.right_arm_rot = Vec3::new(-2.35 * ch, -0.35 * ch, -0.45 * ch);
            pose.left_arm_rot = Vec3::new(-1.15 * ch, 0.45 * ch, 0.35 * ch);
            pose.body_scale_delta = Vec3::new(0.07 * ch, 0.05 * ch, 0.07 * ch);
        }
        MotionId::ChargeBossCrescendo => {
            let ch = p.clamp(0.15, 1.0);
            let hover = (elapsed_secs * 7.5).sin() * 0.08;
            pose.body_y = 0.38 * ch + hover;
            pose.body_pitch = 0.24 * ch;
            pose.left_arm_rot = Vec3::new(-2.05 * ch, 0.45 * ch, 0.95 * ch);
            pose.right_arm_rot = Vec3::new(-2.05 * ch, -0.45 * ch, -0.95 * ch);
            pose.left_leg_rot = Vec3::new(0.24 * ch, 0.0, 0.10 * ch);
            pose.right_leg_rot = Vec3::new(0.32 * ch, 0.0, -0.10 * ch);
        }

        // ============================================================
        // 3. CLASS COMBAT & COMBOS (24)
        // ============================================================
        MotionId::WardenSlash1 => {
            pose.body_yaw = coil_strike * 0.42;
            pose.body_pitch = -0.12 * bell;
            pose.shoulder_yaw = coil_strike * 0.36;
            pose.right_arm_rot = Vec3::new(-0.95 + coil_strike * 1.45, coil_strike * 0.65, -0.55);
            pose.left_arm_rot = Vec3::new(-0.45, 0.35, 0.25);
            pose.weapon_offset = Vec3::new(0.22 * coil_strike, 0.14 * bell, 0.36 * snap);
            pose.weapon_rot =
                Vec3::new(-0.42 * coil_strike, 0.85 * coil_strike, 1.35 * coil_strike);
        }
        MotionId::WardenSlash2Reverse => {
            pose.body_yaw = -coil_strike * 0.48;
            pose.body_pitch = -0.15 * bell;
            pose.shoulder_yaw = -coil_strike * 0.42;
            pose.right_arm_rot = Vec3::new(-0.85 + coil_strike * 1.35, -coil_strike * 0.78, 0.62);
            pose.left_arm_rot = Vec3::new(-0.52, -0.25, 0.38);
            pose.weapon_offset = Vec3::new(-0.28 * coil_strike, 0.18 * bell, 0.40 * snap);
            pose.weapon_rot = Vec3::new(
                -0.36 * coil_strike,
                -0.92 * coil_strike,
                -1.45 * coil_strike,
            );
        }
        MotionId::WardenSlash3Overhead => {
            pose.body_y = if p < 0.4 {
                0.18 * (p / 0.4)
            } else {
                -0.14 * snap
            };
            pose.body_pitch = if p < 0.38 {
                0.32 * (p / 0.38)
            } else {
                -0.38 * snap
            };
            pose.right_arm_rot = Vec3::new(-2.10 + p * 3.10, 0.0, 0.0);
            pose.left_arm_rot = Vec3::new(-1.85 + p * 2.75, 0.25, -0.22);
            pose.weapon_offset = Vec3::new(0.0, 0.56 * (1.0 - p) - 0.18 * snap, 0.52 * snap);
            pose.weapon_rot = Vec3::new(-1.35 + p * 2.55, 0.0, 0.0);
        }
        MotionId::WardenWhirlwindSpin => {
            pose.body_yaw = p * tau * 2.0;
            pose.body_y = -0.06 * bell;
            pose.body_pitch = -0.14 * bell;
            pose.right_arm_rot = Vec3::new(-0.18, 0.0, -1.42);
            pose.left_arm_rot = Vec3::new(-0.18, 0.0, 1.32);
            pose.weapon_offset = Vec3::new(0.45, 0.10, 0.20);
            pose.weapon_rot = Vec3::new(0.0, 0.0, -1.45);
        }
        MotionId::WardenLeapSlamAir => {
            let arc_y = (p * pi).sin() * 0.82;
            pose.body_y = arc_y - 0.12 * (1.0 - bell);
            pose.body_pitch = -0.34 * bell - 0.18 * snap;
            pose.right_arm_rot = Vec3::new(-2.25 + p * 2.95, -0.18, -0.20);
            pose.left_arm_rot = Vec3::new(-1.85 + p * 2.40, 0.22, 0.24);
            pose.left_leg_rot = Vec3::new(-0.55 * bell, 0.0, 0.16);
            pose.right_leg_rot = Vec3::new(0.42 * bell, 0.0, -0.16);
            pose.weapon_offset = Vec3::new(0.04, 0.44 * bell, 0.46 * snap);
            pose.weapon_rot = Vec3::new(-1.25 + p * 2.45, 0.0, 0.0);
        }
        MotionId::WardenSolarCleave => {
            pose.body_y = 0.12 * bell;
            pose.body_yaw = coil_strike * 0.62;
            pose.body_pitch = -0.24 * snap;
            pose.body_scale_delta = Vec3::splat(0.08 * snap);
            pose.right_arm_rot = Vec3::new(-1.35 + coil_strike * 2.05, coil_strike * 0.85, -0.68);
            pose.left_arm_rot = Vec3::new(-1.10 + coil_strike * 1.45, 0.35, 0.35);
            pose.weapon_offset = Vec3::new(0.32 * coil_strike, 0.24 * bell, 0.56 * snap);
            pose.weapon_rot =
                Vec3::new(-0.55 * coil_strike, 1.10 * coil_strike, 1.75 * coil_strike);
        }
        MotionId::StalkerQuickShot => {
            pose.body_yaw = 0.44 * bell;
            pose.left_arm_rot = Vec3::new(-1.32 * bell, 0.16, 0.12);
            pose.right_arm_rot = Vec3::new(-1.18 * bell, -0.64 * (1.0 - snap), -0.42);
            pose.weapon_offset = Vec3::new(-0.10 * bell, 0.16 * bell, 0.26 * bell);
            pose.weapon_rot = Vec3::new(-0.08, 0.32 * bell, 0.22 * bell);
        }
        MotionId::StalkerTwinSnap => {
            let twin = (p * tau * 2.0).sin().abs();
            pose.body_yaw = 0.48 * bell;
            pose.body_pitch = -0.12 * twin;
            pose.left_arm_rot = Vec3::new(-1.38 * bell, 0.20, 0.18);
            pose.right_arm_rot = Vec3::new(-1.25 * bell, -0.72 * twin, -0.52);
            pose.weapon_offset = Vec3::new(-0.12 * bell, 0.18 * bell, 0.30 * twin);
        }
        MotionId::StalkerPiercingRelease => {
            pose.body_yaw = 0.56 * bell;
            pose.body_pitch = 0.18 * snap - 0.12 * (1.0 - snap);
            pose.body_y = -0.10 * bell;
            pose.left_arm_rot = Vec3::new(-1.45 * bell, 0.18, 0.10);
            pose.right_arm_rot = Vec3::new(-1.25 * bell, -0.88 + snap * 1.10, -0.62);
            pose.weapon_offset = Vec3::new(-0.14 * bell, 0.20 * bell, 0.32 * bell - 0.12 * snap);
        }
        MotionId::StalkerVaultBackflip => {
            pose.body_y = (p * pi).sin() * 0.62;
            pose.body_pitch = p * tau;
            pose.left_arm_rot = Vec3::new(-1.10, 0.0, 0.65);
            pose.right_arm_rot = Vec3::new(-1.10, 0.0, -0.65);
            pose.left_leg_rot = Vec3::new(-0.85 * bell, 0.0, 0.0);
            pose.right_leg_rot = Vec3::new(-0.65 * bell, 0.0, 0.0);
        }
        MotionId::StalkerShadowLunge => {
            pose.body_y = -0.16 * bell;
            pose.body_pitch = -0.52 * bell;
            pose.body_roll = -0.22 * bell;
            pose.left_arm_rot = Vec3::new(0.65 * bell, 0.0, 0.45 * bell);
            pose.right_arm_rot = Vec3::new(-1.35 * bell, -0.20, -0.15);
            pose.weapon_offset = Vec3::new(0.08, -0.06, 0.58 * bell);
        }
        MotionId::StalkerPhantomVolley => {
            let rapid = (p * pi * 4.0).sin().abs();
            pose.body_y = 0.14 * bell;
            pose.body_yaw = 0.52 * bell;
            pose.body_roll = -0.16 * bell;
            pose.left_arm_rot = Vec3::new(-1.42 * bell, 0.24, 0.15);
            pose.right_arm_rot = Vec3::new(-1.28 * bell, -0.78 * rapid, -0.48);
            pose.weapon_offset = Vec3::new(-0.12 * bell, 0.24 * bell, 0.34 * bell);
        }
        MotionId::EmberStaffFlick => {
            pose.body_yaw = coil_strike * 0.34;
            pose.right_arm_rot = Vec3::new(-1.15 + coil_strike * 0.85, -0.25 * coil_strike, -0.25);
            pose.left_arm_rot = Vec3::new(-0.65 * bell, 0.35, 0.45 * bell);
            pose.weapon_offset = Vec3::new(0.12 * coil_strike, 0.24 * bell, 0.34 * snap);
            pose.weapon_rot = Vec3::new(-0.48 * snap, 0.0, 0.32 * coil_strike);
        }
        MotionId::EmberTwinBoltThrust => {
            pose.body_pitch = -0.20 * snap;
            pose.left_arm_rot = Vec3::new(-1.48 * snap, 0.22, 0.18);
            pose.right_arm_rot = Vec3::new(-1.48 * snap, -0.22, -0.18);
            pose.weapon_offset = Vec3::new(0.0, 0.20 * bell, 0.46 * snap);
            pose.weapon_rot = Vec3::new(-0.72 * snap, 0.0, 0.0);
        }
        MotionId::EmberCinderfallSlam => {
            pose.body_y = if p < 0.45 {
                0.24 * (p / 0.45)
            } else {
                -0.12 * snap
            };
            pose.body_pitch = if p < 0.45 { 0.26 } else { -0.28 * snap };
            pose.right_arm_rot = Vec3::new(-2.20 + p * 2.65, -0.15, -0.15);
            pose.left_arm_rot = Vec3::new(-1.75 + p * 1.95, 0.32, 0.42);
            pose.weapon_offset = Vec3::new(0.06, 0.52 * (1.0 - p) - 0.16 * snap, 0.32 * snap);
        }
        MotionId::EmberFrostRingBurst => {
            let expand = if p < 0.35 { -0.45 } else { snap };
            pose.body_y = 0.08 * bell;
            pose.left_arm_rot = Vec3::new(-0.95 * bell, 0.25, 1.35 * expand);
            pose.right_arm_rot = Vec3::new(-0.95 * bell, -0.25, -1.35 * expand);
            pose.body_scale_delta = Vec3::new(0.10 * snap, -0.06 * snap, 0.10 * snap);
            pose.weapon_offset = Vec3::new(0.18 * snap, 0.26 * bell, 0.14);
        }
        MotionId::EmberAshStepBlink => {
            pose.body_y = 0.16 * bell;
            pose.body_pitch = -0.36 * bell;
            pose.body_scale_delta = Vec3::new(-0.22 * bell, 0.26 * bell, -0.22 * bell);
            pose.left_arm_rot = Vec3::new(0.45 * bell, 0.0, 0.38 * bell);
            pose.right_arm_rot = Vec3::new(0.45 * bell, 0.0, -0.38 * bell);
        }
        MotionId::EmberOverloadLevitate => {
            pose.body_y = 0.34 * bell;
            pose.body_pitch = 0.22 * bell;
            pose.body_yaw = (p * tau).sin() * 0.24;
            pose.left_arm_rot = Vec3::new(-1.85 * bell, 0.38, 1.05 * bell);
            pose.right_arm_rot = Vec3::new(-1.95 * bell, -0.38, -1.05 * bell);
            pose.left_leg_rot = Vec3::new(0.26 * bell, 0.0, 0.10);
            pose.right_leg_rot = Vec3::new(0.36 * bell, 0.0, -0.10);
            pose.weapon_offset = Vec3::new(0.0, 0.56 * bell, 0.22 * bell);
        }
        MotionId::SoulbinderReapArc => {
            pose.body_yaw = coil_strike * 0.46;
            pose.body_y = 0.06 * bell;
            pose.right_arm_rot = Vec3::new(-1.05 + coil_strike * 1.35, coil_strike * 0.62, -0.45);
            pose.left_arm_rot = Vec3::new(-0.75 * bell, 0.38, 0.42 * bell);
            pose.weapon_offset = Vec3::new(0.24 * coil_strike, 0.18 * bell, 0.38 * snap);
            pose.weapon_rot =
                Vec3::new(-0.35 * coil_strike, 0.78 * coil_strike, 1.15 * coil_strike);
        }
        MotionId::SoulbinderCommandPoint => {
            pose.body_pitch = -0.16 * bell;
            pose.body_yaw = -0.22 * bell;
            pose.left_arm_rot = Vec3::new(-1.52 * bell, 0.14, 0.08);
            pose.right_arm_rot = Vec3::new(-0.45 * bell, -0.22, -0.24);
            pose.weapon_offset = Vec3::new(0.08, 0.16 * bell, 0.12);
        }
        MotionId::SoulbinderRaiseGuard => {
            pose.body_y = -0.14 * (1.0 - p) + 0.14 * snap;
            pose.body_pitch = -0.22 * (1.0 - p) + 0.24 * snap;
            pose.left_arm_rot = Vec3::new(-0.65 - snap * 1.45, 0.28, 0.62 * snap);
            pose.right_arm_rot = Vec3::new(-0.65 - snap * 1.45, -0.28, -0.62 * snap);
            pose.weapon_offset = Vec3::new(0.0, 0.42 * snap, 0.22 * snap);
        }
        MotionId::SoulbinderSpiritSurge => {
            pose.body_y = 0.24 * bell;
            pose.body_pitch = 0.18 * bell;
            pose.left_arm_rot = Vec3::new(-1.42 * bell, 0.35, 1.18 * bell);
            pose.right_arm_rot = Vec3::new(-1.42 * bell, -0.35, -1.18 * bell);
            pose.weapon_offset = Vec3::new(0.12, 0.38 * bell, 0.18);
        }
        MotionId::SoulbinderSacrificeClasp => {
            pose.body_pitch = -0.24 * bell;
            pose.body_y = -0.08 * bell;
            pose.left_arm_rot = Vec3::new(-1.25 * bell, 0.72 * bell, -0.32 * bell);
            pose.right_arm_rot = Vec3::new(-1.25 * bell, -0.72 * bell, 0.32 * bell);
            pose.weapon_offset = Vec3::new(0.0, 0.20 * bell, 0.26 * bell);
        }
        MotionId::SoulbinderRecallPulse => {
            pose.body_y = 0.10 * bell;
            pose.left_arm_rot = Vec3::new(-1.12 * bell, 0.55 * (1.0 - p), 0.75 * (1.0 - p));
            pose.right_arm_rot = Vec3::new(-1.12 * bell, -0.55 * (1.0 - p), -0.75 * (1.0 - p));
        }

        // ============================================================
        // 4. EVADE, MOBILITY & RECOVERY (10)
        // ============================================================
        MotionId::EvadeForwardRoll => {
            pose.body_y = -0.18 * bell;
            pose.body_pitch = -p * tau;
            pose.left_arm_rot = Vec3::new(-0.95, 0.35, 0.25);
            pose.right_arm_rot = Vec3::new(-0.95, -0.35, -0.25);
        }
        MotionId::EvadeBackstepHop => {
            pose.body_y = 0.22 * bell;
            pose.body_pitch = 0.26 * bell;
            pose.left_leg_rot = Vec3::new(-0.48 * bell, 0.0, 0.12);
            pose.right_leg_rot = Vec3::new(0.35 * bell, 0.0, -0.12);
        }
        MotionId::EvadeSideWeaveLeft => {
            pose.body_y = -0.10 * bell;
            pose.body_roll = 0.58 * bell;
            pose.body_yaw = 0.36 * bell;
            pose.left_arm_rot = Vec3::new(-0.45 * bell, 0.0, 0.52 * bell);
        }
        MotionId::EvadeSideWeaveRight => {
            pose.body_y = -0.10 * bell;
            pose.body_roll = -0.58 * bell;
            pose.body_yaw = -0.36 * bell;
            pose.right_arm_rot = Vec3::new(-0.45 * bell, 0.0, -0.52 * bell);
        }
        MotionId::EvadeWardenBarge => {
            pose.body_y = -0.12 * bell;
            pose.body_pitch = -0.44 * bell;
            pose.body_yaw = 0.52 * bell;
            pose.body_roll = -0.24 * bell;
            pose.left_arm_rot = Vec3::new(-0.95 * bell, 0.65 * bell, -0.15);
            pose.right_arm_rot = Vec3::new(0.42 * bell, -0.25, -0.35 * bell);
            pose.weapon_offset = Vec3::new(0.24 * bell, -0.04, -0.20 * bell);
        }
        MotionId::EvadeStalkerTumble => {
            pose.body_y = -0.22 * bell;
            pose.body_pitch = -0.68 * bell;
            pose.body_roll = 0.72 * bell;
            pose.left_leg_rot = Vec3::new(-0.62 * bell, 0.0, 0.22);
            pose.right_leg_rot = Vec3::new(0.54 * bell, 0.0, -0.22);
        }
        MotionId::EvadeEmberPhaseShift => {
            pose.body_y = 0.14 * bell;
            pose.body_pitch = -0.38 * bell;
            pose.body_scale_delta = Vec3::new(-0.26 * bell, 0.22 * bell, 0.32 * bell);
            pose.left_arm_rot = Vec3::new(0.52 * bell, 0.0, 0.45 * bell);
            pose.right_arm_rot = Vec3::new(0.52 * bell, 0.0, -0.45 * bell);
        }
        MotionId::EvadeSoulbinderGlide => {
            pose.body_y = 0.20 * bell;
            pose.body_pitch = -0.28 * bell;
            pose.body_roll = 0.24 * (p * tau).sin();
            pose.left_arm_rot = Vec3::new(0.32 * bell, 0.0, 0.78 * bell);
            pose.right_arm_rot = Vec3::new(0.32 * bell, 0.0, -0.78 * bell);
            pose.left_leg_rot = Vec3::new(0.45 * bell, 0.0, 0.0);
            pose.right_leg_rot = Vec3::new(0.52 * bell, 0.0, 0.0);
        }
        MotionId::AttackRecoveryLight => {
            let r = 1.0 - p;
            pose.body_pitch = -0.10 * r;
            pose.body_yaw = 0.14 * r;
            pose.right_arm_rot = Vec3::new(0.22 * r, 0.18 * r, -0.16 * r);
        }
        MotionId::AttackRecoveryHeavy => {
            let r = (1.0 - p).powf(1.2);
            pose.body_y = -0.12 * r;
            pose.body_pitch = -0.24 * r;
            pose.body_yaw = 0.28 * r;
            pose.right_arm_rot = Vec3::new(0.48 * r, 0.32 * r, -0.28 * r);
            pose.left_arm_rot = Vec3::new(0.24 * r, 0.0, 0.28 * r);
            pose.weapon_offset = Vec3::new(0.14 * r, -0.20 * r, 0.22 * r);
        }

        // ============================================================
        // 5. HITSTUN, STAGGER & KNOCKBACK (12) — 僵直与受击硬直！
        // ============================================================
        MotionId::HitFlinchFront => {
            pose.body_y = -0.09 * snap;
            pose.body_pitch = 0.46 * snap;
            pose.body_roll = -0.14 * snap;
            pose.left_arm_rot = Vec3::new(0.35 * snap, 0.0, 0.56 * snap);
            pose.right_arm_rot = Vec3::new(0.42 * snap, 0.0, -0.52 * snap);
            pose.weapon_offset = Vec3::new(-0.14 * snap, -0.10 * snap, -0.22 * snap);
            pose.weapon_rot = Vec3::new(0.42 * snap, 0.0, -0.38 * snap);
        }
        MotionId::HitFlinchBack => {
            pose.body_y = -0.08 * snap;
            pose.body_pitch = -0.48 * snap;
            pose.left_arm_rot = Vec3::new(-0.62 * snap, 0.0, 0.42 * snap);
            pose.right_arm_rot = Vec3::new(-0.55 * snap, 0.0, -0.42 * snap);
            pose.weapon_offset = Vec3::new(0.08 * snap, -0.08 * snap, 0.24 * snap);
        }
        MotionId::HitFlinchLeft => {
            pose.body_y = -0.07 * snap;
            pose.body_roll = -0.46 * snap;
            pose.body_yaw = -0.32 * snap;
            pose.left_arm_rot = Vec3::new(-0.28 * snap, 0.0, 0.72 * snap);
            pose.right_arm_rot = Vec3::new(0.34 * snap, 0.0, -0.24 * snap);
        }
        MotionId::HitFlinchRight => {
            pose.body_y = -0.07 * snap;
            pose.body_roll = 0.46 * snap;
            pose.body_yaw = 0.32 * snap;
            pose.left_arm_rot = Vec3::new(0.34 * snap, 0.0, 0.24 * snap);
            pose.right_arm_rot = Vec3::new(-0.28 * snap, 0.0, -0.72 * snap);
        }
        MotionId::HitHeavyRecoil => {
            let stumble = (p * pi * 2.0).sin() * (1.0 - p);
            pose.body_y = -0.18 * snap;
            pose.body_pitch = 0.64 * snap;
            pose.body_roll = -0.34 * snap + stumble * 0.22;
            pose.body_yaw = -0.36 * snap;
            pose.body_scale_delta = Vec3::new(0.08 * snap, -0.10 * snap, 0.08 * snap);
            pose.left_arm_rot = Vec3::new(-0.58 * snap, 0.0, 0.88 * snap);
            pose.right_arm_rot = Vec3::new(-0.48 * snap, 0.0, -0.82 * snap);
            pose.left_leg_rot = Vec3::new(-0.38 * snap, 0.0, 0.20 * snap);
            pose.right_leg_rot = Vec3::new(0.46 * snap, 0.0, -0.20 * snap);
            pose.weapon_offset = Vec3::new(-0.22 * snap, -0.16 * snap, -0.32 * snap);
            pose.weapon_rot = Vec3::new(0.55 * snap, -0.24 * snap, -0.68 * snap);
        }
        MotionId::HitGuardBlock => {
            pose.body_y = -0.06 * snap;
            pose.body_pitch = 0.16 * snap;
            pose.body_yaw = 0.18 * snap;
            pose.left_arm_rot = Vec3::new(-1.18 * snap, 0.56 * snap, -0.24 * snap);
            pose.right_arm_rot = Vec3::new(-0.78 * snap, -0.24 * snap, -0.22 * snap);
            pose.weapon_offset = Vec3::new(0.08 * snap, 0.16 * snap, 0.14 * snap);
            pose.weapon_rot = Vec3::new(-0.32 * snap, 0.0, 0.36 * snap);
        }
        MotionId::HitEvadeGraze => {
            pose.body_y = 0.06 * snap;
            pose.body_yaw = 0.48 * snap;
            pose.body_roll = 0.44 * snap;
            pose.left_arm_rot = Vec3::new(-0.32 * snap, 0.0, 0.62 * snap);
        }
        MotionId::StaggerDazedLoop => {
            let sway = (elapsed_secs * 4.2).sin();
            let wobble = (elapsed_secs * 2.8).cos();
            pose.body_y = -0.16 + wobble.abs() * 0.04;
            pose.body_pitch = -0.22 + wobble * 0.18;
            pose.body_roll = sway * 0.32;
            pose.body_yaw = sway * 0.22;
            pose.left_arm_rot = Vec3::new(0.38 + sway * 0.22, 0.0, 0.32);
            pose.right_arm_rot = Vec3::new(0.38 - sway * 0.22, 0.0, -0.32);
            pose.left_leg_rot = Vec3::new(-0.24, 0.0, 0.16);
            pose.right_leg_rot = Vec3::new(0.24, 0.0, -0.16);
            pose.weapon_offset = Vec3::new(0.08 * sway, -0.26, 0.04);
            pose.weapon_rot = Vec3::new(0.55, 0.0, sway * 0.35);
        }
        MotionId::StaggerKneelCollapse => {
            let gasp = (elapsed_secs * 5.2).sin();
            pose.body_y = -0.42 + gasp * 0.025;
            pose.body_pitch = -0.44 + gasp * 0.07;
            pose.body_roll = (elapsed_secs * 2.5).sin() * 0.10;
            pose.left_arm_rot = Vec3::new(0.58, 0.20, 0.28);
            pose.right_arm_rot = Vec3::new(0.62, -0.20, -0.28);
            pose.left_leg_rot = Vec3::new(-0.95, 0.0, 0.18);
            pose.right_leg_rot = Vec3::new(0.75, 0.0, -0.18);
            pose.weapon_offset = Vec3::new(0.12, -0.36, 0.10);
            pose.weapon_rot = Vec3::new(0.75, 0.0, -0.28);
        }
        MotionId::KnockbackLaunch => {
            pose.body_y = 0.34 * bell;
            pose.body_pitch = 0.74 * bell;
            pose.left_arm_rot = Vec3::new(-1.35 * bell, 0.0, 0.95 * bell);
            pose.right_arm_rot = Vec3::new(-1.35 * bell, 0.0, -0.95 * bell);
            pose.left_leg_rot = Vec3::new(-0.55 * bell, 0.0, 0.20);
            pose.right_leg_rot = Vec3::new(-0.35 * bell, 0.0, -0.20);
        }
        MotionId::KnockbackLandRecover => {
            let r = 1.0 - p;
            pose.body_y = -0.22 * r;
            pose.body_pitch = -0.36 * r;
            pose.left_leg_rot = Vec3::new(-0.52 * r, 0.0, 0.22 * r);
            pose.right_leg_rot = Vec3::new(0.44 * r, 0.0, -0.22 * r);
            pose.left_arm_rot = Vec3::new(0.45 * r, 0.0, 0.38 * r);
            pose.right_arm_rot = Vec3::new(0.45 * r, 0.0, -0.38 * r);
        }
        MotionId::HitStopFreezeFrame => {
            pose.body_scale_delta = Vec3::new(0.05, -0.05, 0.07);
            pose.body_roll = fast * 0.06;
        }

        // ============================================================
        // 6. STATUS AILMENTS (12) — 各种异常状态专属动作！
        // ============================================================
        MotionId::StatusBurningFlail => {
            // Frantic flailing / patting flames off arms & shoulders + heat twitch
            let flail = (elapsed_secs * 15.0).sin();
            let flail2 = (elapsed_secs * 15.0).cos();
            pose.body_y = flail.abs() * 0.06;
            pose.body_pitch = 0.12 + flail * 0.14;
            pose.body_roll = flail2 * 0.22;
            pose.body_yaw = flail * 0.20;
            pose.left_arm_rot = Vec3::new(-1.15 + flail * 0.72, 0.35, 0.65 + flail2 * 0.45);
            pose.right_arm_rot = Vec3::new(-1.15 - flail * 0.72, -0.35, -0.65 - flail2 * 0.45);
            pose.left_leg_rot = Vec3::new(flail * 0.24, 0.0, 0.08);
            pose.right_leg_rot = Vec3::new(-flail * 0.24, 0.0, -0.08);
            pose.weapon_offset = Vec3::new(flail * 0.12, 0.14, 0.0);
        }
        MotionId::StatusChilledShiver => {
            // Hunched shoulders, crossed arms close to torso, rapid cold shiver
            pose.body_y = -0.08;
            pose.body_pitch = -0.22;
            pose.body_roll = shiver * 0.065;
            pose.body_yaw = shiver * 0.045;
            pose.shoulder_pitch = -0.20;
            pose.left_arm_rot = Vec3::new(-0.92, 0.72, -0.36 + shiver * 0.08);
            pose.right_arm_rot = Vec3::new(-0.88, -0.68, 0.34 - shiver * 0.08);
            pose.left_leg_rot = Vec3::new(-0.14, 0.0, -0.06);
            pose.right_leg_rot = Vec3::new(-0.14, 0.0, 0.06);
            pose.weapon_offset = Vec3::new(-0.08, -0.06, -0.06);
        }
        MotionId::StatusFrozenRigid => {
            // Locked stiff posture with faint micro-vibration
            pose.body_pitch = -0.10;
            pose.body_roll = shiver * 0.025;
            pose.body_scale_delta = Vec3::new(0.03, 0.03, 0.03);
            pose.left_arm_rot = Vec3::new(-0.65, 0.35, 0.18);
            pose.right_arm_rot = Vec3::new(-0.65, -0.35, -0.18);
        }
        MotionId::StatusJailedStrain => {
            // Legs locked wide, torso pulling fiercely against binding chains
            let tug = (elapsed_secs * 9.5).sin();
            pose.body_y = -0.14;
            pose.body_pitch = -0.18 + tug * 0.18;
            pose.body_yaw = (elapsed_secs * 6.2).cos() * 0.26;
            pose.body_roll = tug * 0.14;
            pose.left_arm_rot = Vec3::new(-0.42 + tug * 0.34, -0.35, 0.92 + tug * 0.28);
            pose.right_arm_rot = Vec3::new(-0.42 - tug * 0.34, 0.35, -0.92 - tug * 0.28);
            pose.left_leg_rot = Vec3::new(-0.26, 0.0, 0.28);
            pose.right_leg_rot = Vec3::new(-0.26, 0.0, -0.28);
        }
        MotionId::StatusBleedingClutch => {
            // Clutching ribs with off-hand, leaning heavily to wounded side
            let throb = (elapsed_secs * 4.8).sin();
            pose.body_y = -0.12 + throb * 0.02;
            pose.body_pitch = -0.28;
            pose.body_roll = 0.22 + throb * 0.06;
            pose.left_arm_rot = Vec3::new(-0.85, 0.88, -0.45);
            pose.right_arm_rot = Vec3::new(0.28, 0.0, -0.18);
            pose.weapon_offset = Vec3::new(0.06, -0.16, -0.06);
        }
        MotionId::StatusVulnerableExposed => {
            // Guard blown wide open, chest thrust back & stumbling off-balance
            let sway = (elapsed_secs * 5.6).sin();
            pose.body_y = -0.06;
            pose.body_pitch = 0.26 + sway * 0.08;
            pose.body_roll = sway * 0.16;
            pose.left_arm_rot = Vec3::new(0.22, -0.35, 1.02);
            pose.right_arm_rot = Vec3::new(0.22, 0.35, -1.02);
            pose.weapon_offset = Vec3::new(0.22, -0.12, -0.18);
        }
        MotionId::StatusPoisonedHunch => {
            let retch = (elapsed_secs * 5.4).sin().max(0.0);
            pose.body_y = -0.16 - retch * 0.06;
            pose.body_pitch = -0.42 - retch * 0.18;
            pose.body_roll = (elapsed_secs * 3.1).sin() * 0.12;
            pose.left_arm_rot = Vec3::new(-0.55, 0.65, -0.28);
            pose.right_arm_rot = Vec3::new(0.32, -0.25, -0.22);
        }
        MotionId::StatusCursedTremor => {
            let twitch = (elapsed_secs * 22.0).sin() * (elapsed_secs * 3.2).sin().abs();
            pose.body_pitch = 0.18 + twitch * 0.22;
            pose.body_roll = twitch * 0.28;
            pose.body_yaw = -twitch * 0.24;
            pose.left_arm_rot = Vec3::new(-1.35 + twitch * 0.45, 0.40, 0.55);
            pose.right_arm_rot = Vec3::new(-1.35 - twitch * 0.45, -0.40, -0.55);
        }
        MotionId::StatusFearedCower => {
            let shake = (elapsed_secs * 14.0).sin();
            pose.body_y = -0.18;
            pose.body_pitch = -0.30;
            pose.body_roll = shake * 0.10;
            pose.left_arm_rot = Vec3::new(-1.85, 0.58, -0.34);
            pose.right_arm_rot = Vec3::new(-1.85, -0.58, 0.34);
            pose.left_leg_rot = Vec3::new(-0.32, 0.0, 0.12);
            pose.right_leg_rot = Vec3::new(0.32, 0.0, -0.12);
        }
        MotionId::StatusSilencedClutchThroat => {
            let gasp = (elapsed_secs * 6.5).sin();
            pose.body_pitch = 0.14 + gasp * 0.08;
            pose.left_arm_rot = Vec3::new(-1.58, 0.65, -0.42);
            pose.right_arm_rot = Vec3::new(-1.42, -0.58, 0.38);
            pose.body_roll = gasp * 0.12;
        }
        MotionId::StatusExhaustedGasp => {
            let gasp = (elapsed_secs * 4.2).sin();
            pose.body_y = -0.20 + gasp * 0.03;
            pose.body_pitch = -0.46 + gasp * 0.08;
            pose.left_arm_rot = Vec3::new(0.45, 0.18, 0.18);
            pose.right_arm_rot = Vec3::new(0.55, -0.18, -0.18);
            pose.weapon_offset = Vec3::new(0.08, -0.28, 0.14);
        }
        MotionId::StatusBlindGrope => {
            let wave = (elapsed_secs * 4.8).sin();
            pose.body_yaw = wave * 0.34;
            pose.body_pitch = -0.12;
            pose.left_arm_rot = Vec3::new(-1.28, 0.25 + wave * 0.38, 0.35);
            pose.right_arm_rot = Vec3::new(-1.28, -0.25 + wave * 0.38, -0.35);
        }

        // ============================================================
        // 7. BUFFS, CONSUMABLES & WORLD INTERACTIONS (10)
        // ============================================================
        MotionId::DrinkHealthPotion => {
            pose.body_pitch = 0.24 * bell;
            pose.body_roll = 0.08 * bell;
            pose.left_arm_rot = Vec3::new(-1.72 * bell, 0.52 * bell, -0.24 * bell);
            pose.right_arm_rot = Vec3::new(0.18 * bell, 0.0, -0.18 * bell);
        }
        MotionId::DrinkElixirFlask => {
            pose.body_pitch = 0.32 * bell;
            pose.body_y = 0.05 * bell;
            pose.left_arm_rot = Vec3::new(-1.92 * bell, 0.45 * bell, -0.18 * bell);
            pose.right_arm_rot = Vec3::new(-0.35 * bell, 0.0, -0.35 * bell);
            pose.body_scale_delta = Vec3::splat(0.05 * bell);
        }
        MotionId::ActivateSurgeRoar => {
            pose.body_y = 0.14 * bell;
            pose.body_pitch = 0.36 * bell;
            pose.body_scale_delta = Vec3::splat(0.12 * bell);
            pose.left_arm_rot = Vec3::new(-1.45 * bell, -0.45 * bell, 1.18 * bell);
            pose.right_arm_rot = Vec3::new(-1.45 * bell, 0.45 * bell, -1.18 * bell);
            pose.weapon_offset = Vec3::new(0.18 * bell, 0.32 * bell, -0.08);
        }
        MotionId::ActivateGloryStride => {
            pose.body_y = 0.08 * bell;
            pose.body_pitch = 0.16 * bell;
            pose.right_arm_rot = Vec3::new(-1.85 * bell, 0.0, -0.25 * bell);
            pose.weapon_offset = Vec3::new(0.08, 0.44 * bell, 0.06);
        }
        MotionId::ActivateConduitHover => {
            pose.body_y = 0.28 * bell;
            pose.body_roll = cyc * 0.08 * bell;
            pose.left_arm_rot = Vec3::new(-1.25 * bell, 0.0, 0.92 * bell);
            pose.right_arm_rot = Vec3::new(-1.25 * bell, 0.0, -0.92 * bell);
        }
        MotionId::InteractChestReach => {
            pose.body_y = -0.22 * bell;
            pose.body_pitch = -0.52 * bell;
            pose.left_arm_rot = Vec3::new(-1.15 * bell, 0.22 * bell, 0.12);
            pose.right_arm_rot = Vec3::new(-1.15 * bell, -0.22 * bell, -0.12);
            pose.left_leg_rot = Vec3::new(-0.42 * bell, 0.0, 0.14);
            pose.right_leg_rot = Vec3::new(0.32 * bell, 0.0, -0.14);
        }
        MotionId::InteractShrinePray => {
            pose.body_y = -0.12 * bell;
            pose.body_pitch = -0.26 * bell;
            pose.left_arm_rot = Vec3::new(-1.18 * bell, 0.65 * bell, -0.35 * bell);
            pose.right_arm_rot = Vec3::new(-1.18 * bell, -0.65 * bell, 0.35 * bell);
        }
        MotionId::InteractLoreInspect => {
            pose.body_pitch = -0.18 * bell;
            pose.body_yaw = 0.16 * bell;
            pose.left_arm_rot = Vec3::new(-1.05 * bell, 0.38 * bell, 0.15);
            pose.right_arm_rot = Vec3::new(-0.85 * bell, -0.22 * bell, -0.12);
        }
        MotionId::LevelUpTriumph => {
            pose.body_y = 0.20 * bell;
            pose.body_pitch = 0.22 * bell;
            pose.body_scale_delta = Vec3::splat(0.09 * bell);
            pose.left_arm_rot = Vec3::new(-2.15 * bell, 0.18, 0.42 * bell);
            pose.right_arm_rot = Vec3::new(-2.25 * bell, -0.18, -0.42 * bell);
            pose.weapon_offset = Vec3::new(0.06, 0.56 * bell, 0.0);
        }
        MotionId::DeathCollapse => {
            let fall = p.powf(1.4);
            pose.body_y = -0.62 * fall;
            pose.body_pitch = 1.42 * fall;
            pose.body_roll = 0.32 * fall;
            pose.left_arm_rot = Vec3::new(-0.65 * fall, 0.0, 1.15 * fall);
            pose.right_arm_rot = Vec3::new(-0.45 * fall, 0.0, -1.05 * fall);
            pose.left_leg_rot = Vec3::new(0.28 * fall, 0.0, 0.22 * fall);
            pose.right_leg_rot = Vec3::new(-0.18 * fall, 0.0, -0.22 * fall);
            pose.weapon_offset = Vec3::new(0.32 * fall, -0.42 * fall, 0.14 * fall);
        }

        // ============================================================
        // 8. ENEMY & BOSS SPECIALIZED MOTIONS (8)
        // ============================================================
        MotionId::EnemyAlertStartle => {
            pose.body_y = 0.14 * snap;
            pose.body_pitch = 0.24 * snap;
            pose.left_arm_rot = Vec3::new(-0.95 * snap, 0.0, 0.55 * snap);
            pose.right_arm_rot = Vec3::new(-1.10 * snap, 0.0, -0.55 * snap);
        }
        MotionId::EnemyMeleeSwing => {
            pose.body_yaw = coil_strike * 0.45;
            pose.body_pitch = -0.18 * bell;
            pose.right_arm_rot = Vec3::new(-1.35 + coil_strike * 2.15, coil_strike * 0.45, -0.35);
            pose.left_arm_rot = Vec3::new(-0.45 * bell, 0.25, 0.30);
        }
        MotionId::EnemyHeavyOverhead => {
            pose.body_y = if p < 0.42 {
                0.16 * (p / 0.42)
            } else {
                -0.16 * snap
            };
            pose.body_pitch = if p < 0.42 { 0.34 } else { -0.44 * snap };
            pose.right_arm_rot = Vec3::new(-2.35 + p * 3.45, 0.0, -0.18);
            pose.left_arm_rot = Vec3::new(-1.95 + p * 2.85, 0.22, 0.18);
        }
        MotionId::EnemyRangedDraw => {
            pose.body_yaw = 0.42 * bell;
            pose.right_arm_rot = Vec3::new(-1.28 * bell, -0.45 * (1.0 - snap), -0.24);
            pose.left_arm_rot = Vec3::new(-1.18 * bell, 0.30, 0.18);
        }
        MotionId::EnemyShockwaveSlam => {
            pose.body_y = if p < 0.45 {
                0.32 * (p / 0.45)
            } else {
                -0.22 * snap
            };
            pose.body_pitch = if p < 0.45 { 0.28 } else { -0.48 * snap };
            pose.left_arm_rot = Vec3::new(-2.15 + p * 3.10, 0.18, 0.35);
            pose.right_arm_rot = Vec3::new(-2.15 + p * 3.10, -0.18, -0.35);
            pose.body_scale_delta = Vec3::new(0.12 * snap, -0.12 * snap, 0.12 * snap);
        }
        MotionId::BossEntranceRoar => {
            pose.body_y = 0.18 * bell;
            pose.body_pitch = 0.42 * bell;
            pose.body_scale_delta = Vec3::splat(0.14 * bell);
            pose.left_arm_rot = Vec3::new(-1.68 * bell, -0.42 * bell, 1.24 * bell);
            pose.right_arm_rot = Vec3::new(-1.82 * bell, 0.42 * bell, -1.24 * bell);
        }
        MotionId::BossPhaseShiftAscend => {
            let pulse = (p * pi * 3.0).sin().abs();
            pose.body_y = 0.54 * bell;
            pose.body_pitch = 0.28 * bell;
            pose.body_yaw = cyc_cos * 0.25 * bell;
            pose.body_scale_delta = Vec3::splat(0.16 * pulse);
            pose.left_arm_rot = Vec3::new(-2.15 * bell, 0.45, 1.15 * bell);
            pose.right_arm_rot = Vec3::new(-2.15 * bell, -0.45, -1.15 * bell);
        }
        MotionId::BossBellTollSwing => {
            pose.body_roll = coil_strike * 0.34;
            pose.body_yaw = -coil_strike * 0.42;
            pose.left_arm_rot = Vec3::new(-1.85 + coil_strike * 2.65, 0.25, 0.42);
            pose.right_arm_rot = Vec3::new(-1.10 * bell, -0.20, -0.35);
        }

        // ============================================================
        // 9. SUMMONS, COMPANION & CO-OP ACTIONS (8)
        // ============================================================
        MotionId::SummonSpiritHoverSlash => {
            pose.body_y = 0.16 + 0.08 * bell;
            pose.body_yaw = coil_strike * 0.56;
            pose.right_arm_rot = Vec3::new(-1.15 + coil_strike * 1.65, 0.55 * coil_strike, -0.58);
            pose.left_arm_rot = Vec3::new(-0.55 * bell, 0.22, 0.52);
        }
        MotionId::SummonSpiritPhantomLunge => {
            pose.body_y = 0.20;
            pose.body_pitch = -0.56 * bell;
            pose.right_arm_rot = Vec3::new(-1.45 * bell, 0.0, -0.22);
            pose.left_arm_rot = Vec3::new(0.45 * bell, 0.0, 0.65 * bell);
        }
        MotionId::SummonSpiritSpectralWave => {
            pose.body_y = 0.24 * bell;
            pose.body_pitch = 0.18 * bell;
            pose.left_arm_rot = Vec3::new(-1.55 * bell, 0.35, 0.95 * bell);
            pose.right_arm_rot = Vec3::new(-1.55 * bell, -0.35, -0.95 * bell);
        }
        MotionId::SummonBoneGuardBrace => {
            let b = (elapsed_secs * 3.0).sin();
            pose.body_y = -0.10 + b * 0.015;
            pose.body_pitch = -0.16;
            pose.left_arm_rot = Vec3::new(-1.12, 0.56, -0.24);
            pose.right_arm_rot = Vec3::new(-0.54, -0.18, -0.20);
        }
        MotionId::SummonBoneTauntSlam => {
            pose.body_y = if p < 0.42 {
                0.18 * (p / 0.42)
            } else {
                -0.16 * snap
            };
            pose.body_pitch = if p < 0.42 { 0.26 } else { -0.38 * snap };
            pose.left_arm_rot = Vec3::new(-1.95 + p * 2.65, 0.22, 0.32);
            pose.right_arm_rot = Vec3::new(-1.95 + p * 2.65, -0.22, -0.32);
        }
        MotionId::CompanionVanguardCharge => {
            pose.body_pitch = -0.42 * bell;
            pose.body_y = 0.08 * bell;
            pose.right_arm_rot = Vec3::new(-1.45 * bell, 0.0, -0.28);
            pose.left_arm_rot = Vec3::new(-0.75 * bell, 0.35, 0.22);
        }
        MotionId::CompanionGuardianAegis => {
            pose.body_y = -0.10 * bell;
            pose.body_pitch = -0.14 * bell;
            pose.left_arm_rot = Vec3::new(-1.32 * bell, 0.62 * bell, -0.22);
            pose.right_arm_rot = Vec3::new(-1.32 * bell, -0.62 * bell, 0.22);
            pose.body_scale_delta = Vec3::splat(0.08 * bell);
        }
        MotionId::CoopRallyCall => {
            let wave = (p * pi * 3.0).sin();
            pose.body_y = 0.08 * bell;
            pose.body_pitch = 0.16 * bell;
            pose.right_arm_rot = Vec3::new(-2.15 * bell, 0.0, -0.32 + wave * 0.28);
            pose.left_arm_rot = Vec3::new(-0.55 * bell, 0.25, 0.45 * bell);
            pose.weapon_offset = Vec3::new(0.10, 0.48 * bell, 0.0);
        }

        // ============================================================
        // 10. SOCIAL EMOTES & NPC ACTIONS (4)
        // ============================================================
        MotionId::CoopSalute => {
            pose.body_pitch = -0.18 * bell;
            pose.right_arm_rot = Vec3::new(-1.55 * bell, -0.62 * bell, 0.34 * bell);
            pose.left_arm_rot = Vec3::new(-0.35 * bell, 0.25 * bell, 0.12);
        }
        MotionId::CoopCheer => {
            let hop = (p * pi * 2.0).sin().abs();
            pose.body_y = 0.24 * hop;
            pose.body_pitch = 0.16 * bell;
            pose.left_arm_rot = Vec3::new(-2.22 * bell, 0.0, 0.48 * bell);
            pose.right_arm_rot = Vec3::new(-2.22 * bell, 0.0, -0.48 * bell);
            pose.weapon_offset = Vec3::new(0.08, 0.52 * bell, 0.0);
        }
        MotionId::CoopPointTarget => {
            pose.body_pitch = -0.16 * bell;
            pose.body_yaw = -0.26 * bell;
            pose.left_arm_rot = Vec3::new(-1.54 * bell, 0.08, 0.06);
            pose.right_arm_rot = Vec3::new(-0.42 * bell, -0.20, -0.25);
        }
        MotionId::NpcMerchantBeckon => {
            let wave = (elapsed_secs * 5.2).sin();
            pose.body_yaw = (elapsed_secs * 1.4).sin() * 0.16;
            pose.body_pitch = -0.06;
            pose.right_arm_rot = Vec3::new(-1.25 + wave * 0.36, -0.24, -0.32);
            pose.left_arm_rot = Vec3::new(-0.32, 0.28, 0.14);
        }
    }
    pose
}

/// Runtime full-body motion state machine attached to heroes, enemies, summons,
/// companions, and NPCs. Smoothly blends active action clips + status layers.
#[derive(Component, Debug, Clone)]
pub struct MotionState {
    pub action: Option<MotionId>,
    pub action_elapsed: f32,
    pub action_duration: f32,
    pub action_looping: bool,
    pub status_motion: Option<MotionId>,
    pub status_elapsed: f32,
    pub charge_ratio: f32,
    pub combo_step: u8,
    pub combo_reset_timer: f32,
    pub blended_pose: ArticulatedPose,
    pub emote_cursor: usize,
}

impl Default for MotionState {
    fn default() -> Self {
        Self {
            action: None,
            action_elapsed: 0.0,
            action_duration: 0.0,
            action_looping: false,
            status_motion: None,
            status_elapsed: 0.0,
            charge_ratio: 0.0,
            combo_step: 0,
            combo_reset_timer: 0.0,
            blended_pose: ArticulatedPose::ZERO,
            emote_cursor: 0,
        }
    }
}

impl MotionState {
    /// Request playing an action clip if its priority meets or exceeds any
    /// currently running clip (or if `force` is true).
    pub fn play(&mut self, id: MotionId, duration_override: Option<f32>, force: bool) {
        let spec = id.spec();
        if !force
            && let Some(current) = self.action
            && current.spec().priority > spec.priority
            && (self.action_looping || self.action_elapsed < self.action_duration)
        {
            return;
        }
        self.action = Some(id);
        self.action_elapsed = 0.0;
        self.action_duration = duration_override.unwrap_or(spec.duration_secs).max(0.05);
        self.action_looping = spec.looping;
    }

    /// Clear a looping action (such as an active charge hold or portal channel).
    pub fn stop_action(&mut self, id: MotionId) {
        if self.action == Some(id) {
            self.action = None;
            self.action_elapsed = 0.0;
            self.action_looping = false;
        }
    }

    pub fn advance_combo(&mut self) -> u8 {
        let step = self.combo_step;
        self.combo_step = (self.combo_step + 1) % 3;
        self.combo_reset_timer = 1.65;
        step
    }

    pub fn active_display_motion(&self) -> Option<MotionId> {
        if self.charge_ratio > 0.05
            && let Some(a) = self.action
        {
            return Some(a);
        }
        match (self.action, self.status_motion) {
            (Some(a), Some(s)) => {
                if s.spec().priority > a.spec().priority {
                    Some(s)
                } else {
                    Some(a)
                }
            }
            (Some(a), None) => Some(a),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        }
    }

    pub fn tick_and_sample(
        &mut self,
        dt: f32,
        elapsed_secs: f32,
        base_idle: Option<MotionId>,
    ) -> ArticulatedPose {
        if self.combo_reset_timer > 0.0 {
            self.combo_reset_timer = (self.combo_reset_timer - dt).max(0.0);
            if self.combo_reset_timer <= 0.0 {
                self.combo_step = 0;
            }
        }
        if self.status_motion.is_some() {
            self.status_elapsed += dt;
        } else {
            self.status_elapsed = 0.0;
        }

        let mut target = match base_idle {
            Some(idle) => sample_motion_pose(idle, 0.0, elapsed_secs),
            None => ArticulatedPose::ZERO,
        };

        if let Some(id) = self.action {
            self.action_elapsed += dt;
            let progress = if self.action_looping {
                if self.charge_ratio > 0.0 {
                    self.charge_ratio
                } else {
                    (self.action_elapsed / self.action_duration.max(0.05)).fract()
                }
            } else {
                (self.action_elapsed / self.action_duration.max(0.05)).clamp(0.0, 1.0)
            };
            let action_pose = sample_motion_pose(id, progress, elapsed_secs);
            target = action_pose;
            if !self.action_looping && self.action_elapsed >= self.action_duration {
                // Transition heavy attacks automatically into brief recovery motion!
                let follow = match id {
                    MotionId::WardenSlash3Overhead
                    | MotionId::WardenSolarCleave
                    | MotionId::WardenLeapSlamAir
                    | MotionId::EmberCinderfallSlam
                    | MotionId::StalkerPiercingRelease => Some(MotionId::AttackRecoveryHeavy),
                    MotionId::KnockbackLaunch => Some(MotionId::KnockbackLandRecover),
                    _ => None,
                };
                if let Some(next_id) = follow {
                    let next_spec = next_id.spec();
                    self.action = Some(next_id);
                    self.action_elapsed = 0.0;
                    self.action_duration = next_spec.duration_secs;
                    self.action_looping = false;
                } else {
                    self.action = None;
                }
            }
        }

        // Layer status ailment posture seamlessly on top of movement / idle / actions
        if let Some(status_id) = self.status_motion {
            let status_pose =
                sample_motion_pose(status_id, 0.5, self.status_elapsed + elapsed_secs);
            let weight = if matches!(
                status_id,
                MotionId::StaggerKneelCollapse
                    | MotionId::StaggerDazedLoop
                    | MotionId::StatusFrozenRigid
            ) {
                1.0
            } else if self.action.is_some() {
                0.48
            } else {
                0.88
            };
            target = if weight >= 0.99 {
                status_pose
            } else {
                target.add_weighted(status_pose, weight)
            };
        }

        // Exponential critically-damped smoothing eliminates hard pops between clips
        let blend_alpha = (1.0 - (-18.0 * dt).exp()).clamp(0.0, 1.0);
        self.blended_pose = self.blended_pose.lerp(target, blend_alpha);
        self.blended_pose
    }
}

/// Cross-module event to play any of the 114 motions on a specific entity.
#[allow(dead_code)]
#[derive(Message, Debug, Clone, Copy)]
pub struct PlayMotionEvent {
    pub entity: Entity,
    pub motion: MotionId,
    pub duration_override: Option<f32>,
    pub force: bool,
}

/// Safely trigger any of the 114 motions on `entity` via a deferred command,
/// working even in minimal test worlds where [`MotionPlugin`] is not installed.
pub fn queue_play_motion(
    commands: &mut Commands,
    entity: Entity,
    motion: MotionId,
    duration_override: Option<f32>,
    force: bool,
) {
    commands.queue(move |world: &mut World| {
        if let Ok(mut entity_mut) = world.get_entity_mut(entity)
            && let Some(mut state) = entity_mut.get_mut::<MotionState>()
        {
            state.play(motion, duration_override, force);
        }
    });
}

/// Tracks active player hold-to-charge state (Hold `LeftAlt` or `Middle Mouse`).
#[derive(Resource, Debug, Default, Clone)]
pub struct PlayerChargeState {
    pub charging: bool,
    pub elapsed_secs: f32,
    pub ratio: f32,
    /// When the player releases a charge >= 0.22s, combat consumes this release!
    pub pending_release_ratio: Option<f32>,
}

impl PlayerChargeState {
    pub const MAX_CHARGE_SECS: f32 = 0.85;
    pub const MIN_RELEASE_RATIO: f32 = 0.22;

    pub fn damage_multiplier(ratio: f32) -> f32 {
        1.0 + ratio.clamp(0.0, 1.0) * 0.62
    }

    pub fn impact_multiplier(ratio: f32) -> f32 {
        1.0 + ratio.clamp(0.0, 1.0) * 0.75
    }
}

/// Compute directional hit flinch motion from hit source relative to character facing.
pub fn directional_hit_motion(facing: Vec3, source_to_victim: Vec3, heavy: bool) -> MotionId {
    if heavy {
        return MotionId::HitHeavyRecoil;
    }
    let fwd = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let hit_dir = Vec3::new(source_to_victim.x, 0.0, source_to_victim.z).normalize_or_zero();
    if fwd.length_squared() < 1e-4 || hit_dir.length_squared() < 1e-4 {
        return MotionId::HitFlinchFront;
    }
    let right = Vec3::new(fwd.z, 0.0, -fwd.x);
    let forward_dot = hit_dir.dot(fwd);
    let right_dot = hit_dir.dot(right);
    if forward_dot.abs() >= right_dot.abs() {
        if forward_dot < 0.0 {
            MotionId::HitFlinchFront
        } else {
            MotionId::HitFlinchBack
        }
    } else if right_dot > 0.0 {
        MotionId::HitFlinchLeft
    } else {
        MotionId::HitFlinchRight
    }
}

/// Select the class-specific charge stance for the active hero class.
pub fn class_charge_motion(class: crate::class::HeroClass) -> MotionId {
    match class {
        crate::class::HeroClass::Warden => MotionId::ChargeHeavyMelee,
        crate::class::HeroClass::Stalker => MotionId::ChargeBowDraw,
        crate::class::HeroClass::Embercaller => MotionId::ChargeStaffSkyward,
        crate::class::HeroClass::Soulbinder => MotionId::ChargeSummonRitual,
    }
}

/// Select the class-specific idle stance.
pub fn class_idle_motion(class: crate::class::HeroClass, low_hp: bool) -> MotionId {
    if low_hp {
        return MotionId::IdleExhausted;
    }
    match class {
        crate::class::HeroClass::Warden => MotionId::IdleWardenHeroic,
        crate::class::HeroClass::Stalker => MotionId::IdleStalkerCrouch,
        crate::class::HeroClass::Embercaller => MotionId::IdleEmberHover,
        crate::class::HeroClass::Soulbinder => MotionId::IdleSoulbinderChannel,
    }
}

/// Select the class-specific evade posture.
pub fn class_evade_motion(class: crate::class::HeroClass) -> MotionId {
    match class {
        crate::class::HeroClass::Warden => MotionId::EvadeWardenBarge,
        crate::class::HeroClass::Stalker => MotionId::EvadeStalkerTumble,
        crate::class::HeroClass::Embercaller => MotionId::EvadeEmberPhaseShift,
        crate::class::HeroClass::Soulbinder => MotionId::EvadeSoulbinderGlide,
    }
}

/// Select the 3-hit basic attack combo motion for the given class & combo step (`0..=2`).
pub fn class_basic_combo_motion(class: crate::class::HeroClass, combo_step: u8) -> MotionId {
    match (class, combo_step % 3) {
        (crate::class::HeroClass::Warden, 0) => MotionId::WardenSlash1,
        (crate::class::HeroClass::Warden, 1) => MotionId::WardenSlash2Reverse,
        (crate::class::HeroClass::Warden, _) => MotionId::WardenSlash3Overhead,
        (crate::class::HeroClass::Stalker, 0) => MotionId::StalkerQuickShot,
        (crate::class::HeroClass::Stalker, 1) => MotionId::StalkerTwinSnap,
        (crate::class::HeroClass::Stalker, _) => MotionId::StalkerPiercingRelease,
        (crate::class::HeroClass::Embercaller, 0) => MotionId::EmberStaffFlick,
        (crate::class::HeroClass::Embercaller, 1) => MotionId::EmberTwinBoltThrust,
        (crate::class::HeroClass::Embercaller, _) => MotionId::EmberCinderfallSlam,
        (crate::class::HeroClass::Soulbinder, 0) => MotionId::SoulbinderReapArc,
        (crate::class::HeroClass::Soulbinder, 1) => MotionId::SoulbinderCommandPoint,
        (crate::class::HeroClass::Soulbinder, _) => MotionId::SoulbinderSpiritSurge,
    }
}

/// Select the signature skill motion for a given class + slot + alternate skill flag.
pub fn class_skill_motion(
    class: crate::class::HeroClass,
    slot: crate::class::SkillSlot,
    alt: bool,
) -> MotionId {
    use crate::class::{HeroClass, SkillSlot};
    match (class, slot, alt) {
        (HeroClass::Warden, SkillSlot::Mobility, false) => MotionId::EvadeWardenBarge,
        (HeroClass::Warden, SkillSlot::Mobility, true) => MotionId::WardenLeapSlamAir,
        (HeroClass::Warden, SkillSlot::Area, false) => MotionId::WardenWhirlwindSpin,
        (HeroClass::Warden, SkillSlot::Area, true) => MotionId::WardenSolarCleave,
        (HeroClass::Warden, SkillSlot::Finisher, _) => MotionId::WardenSlash3Overhead,

        (HeroClass::Stalker, SkillSlot::Mobility, false) => MotionId::StalkerVaultBackflip,
        (HeroClass::Stalker, SkillSlot::Mobility, true) => MotionId::StalkerShadowLunge,
        (HeroClass::Stalker, SkillSlot::Area, _) => MotionId::ChargeBowSkyward,
        (HeroClass::Stalker, SkillSlot::Finisher, false) => MotionId::StalkerPiercingRelease,
        (HeroClass::Stalker, SkillSlot::Finisher, true) => MotionId::StalkerPhantomVolley,

        (HeroClass::Embercaller, SkillSlot::Mobility, _) => MotionId::EmberAshStepBlink,
        (HeroClass::Embercaller, SkillSlot::Area, false) => MotionId::EmberFrostRingBurst,
        (HeroClass::Embercaller, SkillSlot::Area, true) => MotionId::EmberCinderfallSlam,
        (HeroClass::Embercaller, SkillSlot::Finisher, _) => MotionId::EmberOverloadLevitate,

        (HeroClass::Soulbinder, SkillSlot::Mobility, false) => MotionId::SoulbinderCommandPoint,
        (HeroClass::Soulbinder, SkillSlot::Mobility, true) => MotionId::SoulbinderRecallPulse,
        (HeroClass::Soulbinder, SkillSlot::Area, false) => MotionId::SoulbinderRaiseGuard,
        (HeroClass::Soulbinder, SkillSlot::Area, true) => MotionId::SoulbinderSacrificeClasp,
        (HeroClass::Soulbinder, SkillSlot::Finisher, _) => MotionId::SoulbinderSpiritSurge,
    }
}

/// Plugin registering motion events, charge state, emote shortcuts, and motion VFX/HUD.
pub struct MotionPlugin;

impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerChargeState>()
            .add_message::<PlayMotionEvent>()
            .add_systems(Startup, spawn_motion_hud_badge)
            .add_systems(
                Update,
                (
                    apply_motion_events,
                    observe_world_motion_triggers,
                    animate_merchant_npc_beckon,
                    update_player_charge_and_emotes,
                    update_motion_status_vfx,
                    update_motion_hud_badge,
                )
                    .chain()
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), cleanup_motion_runtime);
    }
}

fn apply_motion_events(
    mut events: MessageReader<PlayMotionEvent>,
    mut motions: Query<&mut MotionState>,
) {
    for ev in events.read() {
        if let Ok(mut state) = motions.get_mut(ev.entity) {
            state.play(ev.motion, ev.duration_override, ev.force);
        }
    }
}

fn observe_world_motion_triggers(
    mut last_level: Local<u32>,
    mut interactables: MessageReader<crate::chapter::InteractableUsed>,
    mut player: Query<(&crate::player::PlayerLevel, &mut MotionState), With<crate::player::Player>>,
) {
    let Ok((level, mut motion)) = player.single_mut() else {
        interactables.clear();
        return;
    };
    if *last_level > 0 && level.level > *last_level {
        motion.play(MotionId::LevelUpTriumph, None, true);
    }
    *last_level = level.level;

    for ev in interactables.read() {
        use crate::chapter::InteractableKind as K;
        let clip = match ev.kind {
            K::Cache | K::ReliquaryVault => MotionId::InteractChestReach,
            K::LorePage => MotionId::InteractLoreInspect,
            K::Merchant => MotionId::CoopSalute,
            K::Altar
            | K::WeaponShrine
            | K::FortuneShrine
            | K::StormShrine
            | K::CursedShrine
            | K::BloodObelisk
            | K::HealingWell
            | K::EmberRift
            | K::AshenPylon => MotionId::InteractShrinePray,
        };
        motion.play(clip, None, false);
    }
}

#[allow(clippy::too_many_arguments)]
fn animate_merchant_npc_beckon(
    time: Res<Time>,
    mut merchants: Query<(&crate::dungeon::Interactable, &mut Transform)>,
) {
    let elapsed = time.elapsed_secs();
    let pose = sample_motion_pose(MotionId::NpcMerchantBeckon, 0.5, elapsed);
    for (interactable, mut transform) in &mut merchants {
        if interactable.kind == crate::chapter::InteractableKind::Merchant {
            transform.rotation = Quat::from_euler(
                EulerRot::YXZ,
                pose.body_yaw,
                pose.body_pitch,
                pose.body_roll,
            );
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_player_charge_and_emotes(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    selected: Res<crate::class::SelectedClass>,
    language: Res<crate::ui::localization::UiLanguage>,
    mut charge: ResMut<PlayerChargeState>,
    mut floating_text: MessageWriter<crate::feedback::FloatingCombatTextEvent>,
    mut player: Query<
        (
            &Transform,
            &crate::player::Health,
            Option<&crate::player::Burning>,
            Option<&crate::player::Jailed>,
            Option<&crate::enemy::Chilled>,
            Option<&crate::enemy::Staggered>,
            &mut MotionState,
        ),
        With<crate::player::Player>,
    >,
) {
    let Ok((transform, health, burning, jailed, chilled, staggered, mut motion)) =
        player.single_mut()
    else {
        return;
    };

    // 1. Evaluate highest-priority player status ailment motion
    let low_hp = health.max > 0.0 && (health.current / health.max) < 0.35;
    motion.status_motion = if health.current <= 0.0 {
        Some(MotionId::DeathCollapse)
    } else if staggered.is_some() {
        Some(MotionId::StaggerDazedLoop)
    } else if jailed.is_some() {
        Some(MotionId::StatusJailedStrain)
    } else if burning.is_some() {
        Some(MotionId::StatusBurningFlail)
    } else if chilled.is_some() {
        Some(MotionId::StatusChilledShiver)
    } else if low_hp {
        Some(MotionId::StatusExhaustedGasp)
    } else {
        None
    };

    // 2. Active Hold-to-Charge (LeftAlt or Middle Mouse)
    let hold_charge = keyboard.pressed(KeyCode::AltLeft) || mouse.pressed(MouseButton::Middle);
    let charge_clip = class_charge_motion(selected.0);
    if hold_charge && staggered.is_none() && health.current > 0.0 {
        if !charge.charging {
            charge.charging = true;
            charge.elapsed_secs = 0.0;
            charge.ratio = 0.0;
            motion.play(charge_clip, Some(PlayerChargeState::MAX_CHARGE_SECS), true);
        }
        charge.elapsed_secs =
            (charge.elapsed_secs + time.delta_secs()).min(PlayerChargeState::MAX_CHARGE_SECS);
        charge.ratio = (charge.elapsed_secs / PlayerChargeState::MAX_CHARGE_SECS).clamp(0.0, 1.0);
        motion.charge_ratio = charge.ratio;
    } else if charge.charging {
        let released = charge.ratio;
        charge.charging = false;
        charge.elapsed_secs = 0.0;
        charge.ratio = 0.0;
        motion.charge_ratio = 0.0;
        motion.stop_action(charge_clip);
        if released >= PlayerChargeState::MIN_RELEASE_RATIO {
            charge.pending_release_ratio = Some(released);
        }
    }

    // 3. Co-op / Social Emotes on Digit0 (cycles Rally -> Salute -> Cheer -> Point Target)
    if keyboard.just_pressed(KeyCode::Digit0) && !charge.charging {
        let emote = MotionId::COOP_EMOTES[motion.emote_cursor % MotionId::COOP_EMOTES.len()];
        motion.emote_cursor = (motion.emote_cursor + 1) % MotionId::COOP_EMOTES.len();
        motion.play(emote, None, false);
        let zh = *language == crate::ui::localization::UiLanguage::Chinese;
        floating_text.write(crate::feedback::FloatingCombatTextEvent {
            text: emote.label(zh).to_string(),
            position: transform.translation + Vec3::Y * 2.15,
            critical: false,
        });
    }
}

#[derive(Component)]
struct ChargeAuraRing;

#[derive(Component)]
struct StatusAuraRing;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_motion_status_vfx(
    mut commands: Commands,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    charge: Res<PlayerChargeState>,
    player: Query<
        (&Transform, &MotionState),
        (
            With<crate::player::Player>,
            Without<ChargeAuraRing>,
            Without<StatusAuraRing>,
        ),
    >,
    mut charge_rings: Query<
        (Entity, &mut Transform, &MeshMaterial3d<StandardMaterial>),
        (With<ChargeAuraRing>, Without<StatusAuraRing>),
    >,
    mut status_rings: Query<
        (Entity, &mut Transform, &MeshMaterial3d<StandardMaterial>),
        (With<StatusAuraRing>, Without<ChargeAuraRing>),
    >,
) {
    let Ok((player_transform, motion)) = player.single() else {
        return;
    };
    let elapsed = time.elapsed_secs();

    // Charge-up glowing additive ring around player feet
    if charge.charging && charge.ratio > 0.04 {
        let radius = 0.55 + charge.ratio * 0.95;
        let pulse = 1.0 + (elapsed * 14.0).sin() * 0.08;
        let color = if charge.ratio >= 0.85 {
            Color::srgba(1.0, 0.78, 0.18, 0.95)
        } else {
            Color::srgba(0.95, 0.45, 0.12, 0.80)
        };
        if let Ok((_, mut tf, mat_handle)) = charge_rings.single_mut() {
            tf.translation = player_transform.translation + Vec3::Y * 0.10;
            tf.scale = Vec3::new(radius * pulse, 0.45 + charge.ratio * 0.7, radius * pulse);
            tf.rotate_y(time.delta_secs() * (2.5 + charge.ratio * 5.0));
            if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                let lin = LinearRgba::from(color) * (2.2 + charge.ratio * 2.4);
                mat.base_color = color;
                mat.emissive = lin;
            }
        } else {
            let lin = LinearRgba::from(color) * 2.8;
            commands.spawn((
                Mesh3d(meshes.add(Torus::new(0.42, 0.55))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: color,
                    emissive: lin,
                    unlit: true,
                    alpha_mode: AlphaMode::Add,
                    ..default()
                })),
                Transform::from_translation(player_transform.translation + Vec3::Y * 0.10),
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
                ChargeAuraRing,
                Name::new("Player Charge Aura Ring"),
            ));
        }
    } else {
        for (e, ..) in &charge_rings {
            commands.entity(e).try_despawn();
        }
    }

    // Status ailment additive indicator ring around player (Burning / Chilled / Jailed / Staggered)
    let status_color = match motion.status_motion {
        Some(MotionId::StatusBurningFlail) => Some(Color::srgba(1.0, 0.32, 0.06, 0.88)),
        Some(MotionId::StatusChilledShiver | MotionId::StatusFrozenRigid) => {
            Some(Color::srgba(0.28, 0.82, 1.0, 0.85))
        }
        Some(MotionId::StatusJailedStrain) => Some(Color::srgba(0.76, 0.34, 1.0, 0.90)),
        Some(MotionId::StaggerDazedLoop | MotionId::StaggerKneelCollapse) => {
            Some(Color::srgba(1.0, 0.88, 0.22, 0.92))
        }
        _ => None,
    };

    if let Some(color) = status_color {
        let y_pos = if matches!(
            motion.status_motion,
            Some(MotionId::StaggerDazedLoop | MotionId::StaggerKneelCollapse)
        ) {
            1.95
        } else {
            0.22
        };
        let pulse = 0.88 + (elapsed * 9.0).sin().abs() * 0.24;
        if let Ok((_, mut tf, mat_handle)) = status_rings.single_mut() {
            tf.translation = player_transform.translation + Vec3::Y * y_pos;
            tf.scale = Vec3::new(pulse, 0.35, pulse);
            tf.rotate_y(-time.delta_secs() * 3.4);
            if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                mat.base_color = color;
                mat.emissive = LinearRgba::from(color) * 2.6;
            }
        } else {
            commands.spawn((
                Mesh3d(meshes.add(Torus::new(0.46, 0.56))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: color,
                    emissive: LinearRgba::from(color) * 2.6,
                    unlit: true,
                    alpha_mode: AlphaMode::Add,
                    ..default()
                })),
                Transform::from_translation(player_transform.translation + Vec3::Y * y_pos),
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
                StatusAuraRing,
                Name::new("Player Status Ailment Ring"),
            ));
        }
    } else {
        for (e, ..) in &status_rings {
            commands.entity(e).try_despawn();
        }
    }
}

#[derive(Component)]
struct MotionHudBadgeRoot;

#[derive(Component)]
struct MotionHudBadgeText;

fn spawn_motion_hud_badge(mut commands: Commands, server: Res<AssetServer>) {
    commands
        .spawn((
            MotionHudBadgeRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(18.0),
                top: Val::Px(254.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.025, 0.03, 0.82)),
            BorderColor::all(Color::srgba(0.82, 0.56, 0.24, 0.72)),
        ))
        .with_children(|parent| {
            parent.spawn((
                MotionHudBadgeText,
                crate::ui::localization::NativeLanguageName,
                Text::new(""),
                TextFont {
                    font: server.load("fonts/NotoSansSC.ttf").into(),
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.98, 0.86, 0.62)),
            ));
        });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_motion_hud_badge(
    language: Res<crate::ui::localization::UiLanguage>,
    charge: Res<PlayerChargeState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    player: Query<&MotionState, With<crate::player::Player>>,
    mut root_q: Query<(&mut Node, &mut BorderColor), With<MotionHudBadgeRoot>>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<MotionHudBadgeText>>,
) {
    let zh = *language == crate::ui::localization::UiLanguage::Chinese;
    let compact = windows.single().is_ok_and(|w| w.width() < 760.0);
    let Ok(motion) = player.single() else {
        for (mut node, _) in &mut root_q {
            node.display = Display::None;
        }
        return;
    };

    let display_info = if charge.charging {
        let pct = (charge.ratio * 100.0).round() as u32;
        let label = motion
            .action
            .map_or(if zh { "蓄力" } else { "Charging" }, |m| m.label(zh));
        Some((
            format!("{label} {pct}%"),
            Color::srgb(1.0, 0.80, 0.28),
            Color::srgba(1.0, 0.72, 0.22, 0.88),
        ))
    } else if let Some(active) = motion.active_display_motion() {
        let spec = active.spec();
        if matches!(
            spec.category,
            MotionCategory::StatusAilment
                | MotionCategory::HitstunStagger
                | MotionCategory::ChargeWindup
                | MotionCategory::BuffInteraction
                | MotionCategory::CoopSocialNpc
                | MotionCategory::SummonCompanion
        ) {
            let (fg, border) = match spec.category {
                MotionCategory::StatusAilment => (
                    Color::srgb(1.0, 0.56, 0.36),
                    Color::srgba(0.95, 0.34, 0.18, 0.85),
                ),
                MotionCategory::HitstunStagger => (
                    Color::srgb(1.0, 0.84, 0.32),
                    Color::srgba(0.95, 0.72, 0.18, 0.85),
                ),
                _ => (
                    Color::srgb(0.86, 0.92, 0.98),
                    Color::srgba(0.45, 0.72, 0.88, 0.80),
                ),
            };
            Some((active.label(zh).to_string(), fg, border))
        } else {
            None
        }
    } else {
        None
    };

    match display_info {
        Some((label, fg, border)) => {
            for (mut node, mut bc) in &mut root_q {
                node.display = Display::Flex;
                node.left = Val::Px(if compact { 10.0 } else { 18.0 });
                node.top = Val::Px(if compact { 158.0 } else { 254.0 });
                *bc = BorderColor::all(border);
            }
            for (mut text, mut tc) in &mut text_q {
                if text.0 != label {
                    text.0 = label.clone();
                }
                tc.0 = fg;
            }
        }
        None => {
            for (mut node, _) in &mut root_q {
                node.display = Display::None;
            }
        }
    }
}

fn cleanup_motion_runtime(
    mut commands: Commands,
    mut charge: ResMut<PlayerChargeState>,
    charge_rings: Query<Entity, With<ChargeAuraRing>>,
    status_rings: Query<Entity, With<StatusAuraRing>>,
    mut badge: Query<&mut Node, With<MotionHudBadgeRoot>>,
) {
    *charge = PlayerChargeState::default();
    for e in &charge_rings {
        commands.entity(e).try_despawn();
    }
    for e in &status_rings {
        commands.entity(e).try_despawn();
    }
    for mut node in &mut badge {
        node.display = Display::None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn motion_catalog_contains_over_one_hundred_unique_valid_actions() {
        assert!(
            MotionId::ALL.len() >= 100,
            "expected at least 100 motions, got {}",
            MotionId::ALL.len()
        );
        let mut seen = HashSet::new();
        for id in MotionId::ALL {
            assert!(seen.insert(id), "duplicate MotionId: {id:?}");
            let spec = id.spec();
            assert_eq!(spec.id, id);
            assert!(!spec.name_zh.is_empty(), "empty zh label for {id:?}");
            assert!(!spec.name_en.is_empty(), "empty en label for {id:?}");
            assert!(spec.duration_secs > 0.0, "invalid duration for {id:?}");
            for &p in &[0.0_f32, 0.25, 0.5, 0.75, 1.0] {
                let pose = sample_motion_pose(id, p, 0.42);
                assert!(pose.body_y.is_finite());
                assert!(pose.body_pitch.is_finite());
                assert!(pose.body_yaw.is_finite());
                assert!(pose.body_roll.is_finite());
                assert!(pose.left_arm_rot.is_finite());
                assert!(pose.right_arm_rot.is_finite());
                assert!(pose.left_leg_rot.is_finite());
                assert!(pose.right_leg_rot.is_finite());
                assert!(pose.weapon_offset.is_finite());
                assert!(pose.weapon_rot.is_finite());
            }
        }
    }

    #[test]
    fn charge_stagger_and_status_ailments_produce_distinct_articulated_poses() {
        let burn = sample_motion_pose(MotionId::StatusBurningFlail, 0.5, 0.23);
        let chill = sample_motion_pose(MotionId::StatusChilledShiver, 0.5, 0.23);
        let jail = sample_motion_pose(MotionId::StatusJailedStrain, 0.5, 0.23);
        let bleed = sample_motion_pose(MotionId::StatusBleedingClutch, 0.5, 0.23);
        let kneel = sample_motion_pose(MotionId::StaggerKneelCollapse, 0.5, 0.23);
        let bow_charge = sample_motion_pose(MotionId::ChargeBowDraw, 0.8, 0.23);
        let sword_charge = sample_motion_pose(MotionId::ChargeHeavyMelee, 0.8, 0.23);

        assert_ne!(burn, chill);
        assert_ne!(chill, jail);
        assert_ne!(jail, bleed);
        assert_ne!(kneel, burn);
        assert_ne!(bow_charge, sword_charge);
        assert!(
            kneel.body_y < -0.30,
            "stagger kneel should visibly drop torso"
        );
        assert!(
            bow_charge.body_yaw > 0.4,
            "bow charge should turn torso side-on"
        );
        assert!(
            sword_charge.body_yaw < -0.4,
            "heavy melee charge should coil back"
        );
    }

    #[test]
    fn directional_hit_motion_distinguishes_front_back_left_right_and_heavy() {
        assert_eq!(
            directional_hit_motion(Vec3::Z, -Vec3::Z, false),
            MotionId::HitFlinchFront
        );
        assert_eq!(
            directional_hit_motion(Vec3::Z, Vec3::Z, false),
            MotionId::HitFlinchBack
        );
        assert_eq!(
            directional_hit_motion(Vec3::Z, Vec3::X, false),
            MotionId::HitFlinchLeft
        );
        assert_eq!(
            directional_hit_motion(Vec3::Z, -Vec3::X, false),
            MotionId::HitFlinchRight
        );
        assert_eq!(
            directional_hit_motion(Vec3::Z, -Vec3::Z, true),
            MotionId::HitHeavyRecoil
        );
    }

    #[test]
    fn motion_state_blends_smoothly_and_transitions_heavy_attacks_into_recovery() {
        let mut state = MotionState::default();
        state.play(MotionId::WardenSlash3Overhead, Some(0.40), true);
        let mid = state.tick_and_sample(0.20, 0.20, None);
        assert!(mid.body_y.is_finite());
        // Finish overhead cleave -> should auto-chain into AttackRecoveryHeavy
        state.tick_and_sample(0.25, 0.45, None);
        assert_eq!(state.action, Some(MotionId::AttackRecoveryHeavy));
        // Status ailment layers cleanly while running
        state.status_motion = Some(MotionId::StatusBurningFlail);
        let burning_pose = state.tick_and_sample(0.08, 0.53, None);
        assert_ne!(burning_pose, ArticulatedPose::ZERO);
    }

    #[test]
    fn player_charge_hold_and_status_ailments_drive_motion_state() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(ButtonInput::<MouseButton>::default())
            .insert_resource(crate::class::SelectedClass(
                crate::class::HeroClass::Stalker,
            ))
            .insert_resource(crate::ui::localization::UiLanguage::Chinese)
            .init_resource::<PlayerChargeState>()
            .add_message::<crate::feedback::FloatingCombatTextEvent>()
            .add_systems(Update, update_player_charge_and_emotes);

        let hero = app
            .world_mut()
            .spawn((
                crate::player::Player,
                Transform::default(),
                crate::player::Health {
                    current: 100.0,
                    max: 100.0,
                },
                MotionState::default(),
            ))
            .id();

        // Hold AltLeft to build Stalker bow charge
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::AltLeft);
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(std::time::Duration::from_secs_f32(0.45));
        app.update();
        assert!(app.world().resource::<PlayerChargeState>().charging);
        assert_eq!(
            app.world().get::<MotionState>(hero).unwrap().action,
            Some(MotionId::ChargeBowDraw)
        );

        // Release AltLeft -> queues charged strike release
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::AltLeft);
        app.update();
        assert!(!app.world().resource::<PlayerChargeState>().charging);
        assert!(
            app.world()
                .resource::<PlayerChargeState>()
                .pending_release_ratio
                .is_some()
        );

        // Burning ailment drives StatusBurningFlail
        app.world_mut()
            .entity_mut(hero)
            .insert(crate::player::Burning {
                dps: 5.0,
                timer: Timer::from_seconds(3.0, TimerMode::Once),
            });
        app.update();
        assert_eq!(
            app.world().get::<MotionState>(hero).unwrap().status_motion,
            Some(MotionId::StatusBurningFlail)
        );

        // Jailed overrides Burning with StatusJailedStrain
        app.world_mut()
            .entity_mut(hero)
            .insert(crate::player::Jailed {
                timer: Timer::from_seconds(1.5, TimerMode::Once),
                speed_multiplier: 0.4,
            });
        app.update();
        assert_eq!(
            app.world().get::<MotionState>(hero).unwrap().status_motion,
            Some(MotionId::StatusJailedStrain)
        );
    }
}
