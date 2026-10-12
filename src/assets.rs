use crate::{GameState, HeadlessSmokeExit, class::HeroClass};
use bevy::asset::{LoadState, UntypedAssetId, UntypedHandle};
use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy::prelude::*;
use bevy::time::{Timer, TimerMode};
use bevy::world_serialization::WorldAsset;

/// Loads a color texture with a WebGPU-safe, linearly filtered repeat sampler.
///
/// World materials use UV transforms larger than one so the artwork keeps a
/// consistent texel density instead of stretching once across an entire
/// streamed chunk.
pub fn load_repeating_texture(asset_server: &AssetServer, path: &'static str) -> Handle<Image> {
    asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                mipmap_filter: ImageFilterMode::Linear,
                ..default()
            });
        })
        .load(path)
}

#[derive(Resource)]
pub struct GameAssets {
    pub hero: Handle<WorldAsset>,
    pub hero_arm: Handle<WorldAsset>,
    pub hero_leg: Handle<WorldAsset>,
    pub hero_mantle: Handle<WorldAsset>,
    pub hero_stalker: Handle<WorldAsset>,
    pub hero_stalker_arm: Handle<WorldAsset>,
    pub hero_stalker_leg: Handle<WorldAsset>,
    pub hero_stalker_mantle: Handle<WorldAsset>,
    pub hero_embercaller: Handle<WorldAsset>,
    pub hero_embercaller_arm: Handle<WorldAsset>,
    pub hero_embercaller_leg: Handle<WorldAsset>,
    pub hero_embercaller_mantle: Handle<WorldAsset>,
    pub hero_soulbinder: Handle<WorldAsset>,
    pub hero_soulbinder_arm: Handle<WorldAsset>,
    pub hero_soulbinder_leg: Handle<WorldAsset>,
    pub hero_soulbinder_mantle: Handle<WorldAsset>,
    pub soul_spirit: Handle<WorldAsset>,
    pub soul_spirit_arm: Handle<WorldAsset>,
    pub soul_spirit_leg: Handle<WorldAsset>,
    pub soul_spirit_mantle: Handle<WorldAsset>,
    pub bone_sentry: Handle<WorldAsset>,
    pub bone_sentry_arm: Handle<WorldAsset>,
    pub bone_sentry_leg: Handle<WorldAsset>,
    pub bone_sentry_mantle: Handle<WorldAsset>,
    pub soul_staff: Handle<WorldAsset>,
    pub soul_blade: Handle<WorldAsset>,
    pub skeleton: Handle<WorldAsset>,
    pub ashbone_guard: Handle<WorldAsset>,
    pub cultist: Handle<WorldAsset>,
    pub butcher: Handle<WorldAsset>,
    pub bone_stalker: Handle<WorldAsset>,
    pub ashen_marksman: Handle<WorldAsset>,
    pub treasure_imp: Handle<WorldAsset>,
    pub seal_warden: Handle<WorldAsset>,
    pub keeper: Handle<WorldAsset>,
    pub nemesis: Handle<WorldAsset>,
    pub hollow_thrall: Handle<WorldAsset>,
    pub hollow_stalker: Handle<WorldAsset>,
    pub choir_acolyte: Handle<WorldAsset>,
    pub choir_marksman: Handle<WorldAsset>,
    pub hollow_reaver: Handle<WorldAsset>,
    pub bell_warden: Handle<WorldAsset>,
    pub hollow_matron: Handle<WorldAsset>,
    pub sword: Handle<WorldAsset>,
    pub bone_bow: Handle<WorldAsset>,
    pub ember_staff: Handle<WorldAsset>,
    pub bone_arrow: Handle<WorldAsset>,
    pub ember_bolt: Handle<WorldAsset>,
    pub ash_lance: Handle<WorldAsset>,
    pub sunder_wave: Handle<WorldAsset>,
    pub ground_sigil: Handle<WorldAsset>,
    pub chest: Handle<WorldAsset>,
    pub altar: Handle<WorldAsset>,
    pub soul_font: Handle<WorldAsset>,
    pub quartermaster: Handle<WorldAsset>,
    pub fortune_shrine: Handle<WorldAsset>,
    pub storm_shrine: Handle<WorldAsset>,
    pub healing_well: Handle<WorldAsset>,
    pub cursed_shrine: Handle<WorldAsset>,
    pub blood_obelisk: Handle<WorldAsset>,
    pub reliquary_vault: Handle<WorldAsset>,
    pub ember_rift_prop: Handle<WorldAsset>,
    pub ashen_pylon: Handle<WorldAsset>,
    pub lore_page: Handle<WorldAsset>,
    pub breakable_urn: Handle<WorldAsset>,
    pub breakable_coffer: Handle<WorldAsset>,
    pub slash_arc: Handle<WorldAsset>,
    pub hit_spark: Handle<WorldAsset>,
    pub contact_steel: Handle<WorldAsset>,
    pub contact_expose: Handle<WorldAsset>,
    pub contact_blood: Handle<WorldAsset>,
    pub contact_cleanse: Handle<WorldAsset>,
    pub contact_reap: Handle<WorldAsset>,
    pub contact_ember: Handle<WorldAsset>,
    pub contact_frost: Handle<WorldAsset>,
    pub proc_vigor: Handle<WorldAsset>,
    pub proc_focus: Handle<WorldAsset>,

    pub bone_shatter: Handle<WorldAsset>,
    pub bone_impact: Handle<WorldAsset>,
    pub blood_spray: Handle<WorldAsset>,
    pub execution_burst: Handle<WorldAsset>,
    pub arcane_impact: Handle<WorldAsset>,
    pub holy_impact: Handle<WorldAsset>,
    pub ember_impact: Handle<WorldAsset>,
    pub frost_impact: Handle<WorldAsset>,
    pub void_impact: Handle<WorldAsset>,
    pub frenzy_impact: Handle<WorldAsset>,
    pub vampiric_siphon: Handle<WorldAsset>,
    pub desecrator_burst: Handle<WorldAsset>,
    pub guard_clash: Handle<WorldAsset>,
    pub armor_break: Handle<WorldAsset>,
    pub soul_ward_hit: Handle<WorldAsset>,
    pub hit_bone_rune: Handle<WorldAsset>,
    pub hit_bone_lock: Handle<WorldAsset>,
    pub marrow_flash: Handle<WorldAsset>,
    pub bone_fracture_echo: Handle<WorldAsset>,
    pub elite_affix_break: Handle<WorldAsset>,
    pub shadow_burst: Handle<WorldAsset>,
    pub headshot_burst: Handle<WorldAsset>,
    pub crit_bone_crown: Handle<WorldAsset>,
    pub crit_burst: Handle<WorldAsset>,
    pub stagger_burst: Handle<WorldAsset>,
    pub shadow_trail: Handle<WorldAsset>,
    pub health_pickup: Handle<WorldAsset>,
    pub fury_pickup: Handle<WorldAsset>,
    pub death_remains: Handle<WorldAsset>,
    pub loot_prism: Handle<WorldAsset>,
    pub objective_sigil: Handle<WorldAsset>,
    pub ember_vent: Handle<WorldAsset>,
    pub soul_fissure: Handle<WorldAsset>,
    pub boss_summon_portal: Handle<WorldAsset>,
    pub keeper_cleaver_wave: Handle<WorldAsset>,
    pub keeper_chain_cage: Handle<WorldAsset>,
    pub matron_soul_lance: Handle<WorldAsset>,
    pub matron_crescendo: Handle<WorldAsset>,
    pub affix_ember_aura: Handle<WorldAsset>,
    pub affix_arcane_aura: Handle<WorldAsset>,
    pub affix_frost_aura: Handle<WorldAsset>,
    pub affix_blood_aura: Handle<WorldAsset>,
    pub affix_ward_aura: Handle<WorldAsset>,
    pub ashen_ground_albedo: Handle<Image>,
    pub hollow_ground_albedo: Handle<Image>,
    pub ashen_road_albedo: Handle<Image>,
    pub hollow_road_albedo: Handle<Image>,
    pub scenery_rock: Handle<WorldAsset>,
    pub scenery_dead_tree: Handle<WorldAsset>,
    pub scenery_crystal: Handle<WorldAsset>,
    pub scenery_ruin: Handle<WorldAsset>,
    pub sanctum_gate: Handle<WorldAsset>,
    pub stone_detail: Handle<Image>,
    pub ui_main_menu: Handle<Image>,
    pub ui_panel_frame: Handle<Image>,
    pub ui_slot_frame: Handle<Image>,
    pub ui_resource_orb_frame: Handle<Image>,
    pub ui_skill_icons: Handle<Image>,
    /// Seven columns (basic, Q a/b, E a/b, R a/b) by one row per class.
    pub ui_class_skill_icons: Handle<Image>,
    pub ui_soulbinder_skill_icons: Handle<Image>,
    /// Class-select portraits, indexed by `HeroClass::index`.
    pub ui_class_portraits: [Handle<Image>; 4],
    pub ui_gear_icons: Handle<Image>,
    pub ui_talent_sigils: Handle<Image>,
    pub ui_utility_icons: Handle<Image>,
    pub ui_font_regular: Handle<Font>,
    pub ui_font_bold: Handle<Font>,
    pub ui_font_chinese: Handle<Font>,
}

/// The scenes that assemble one class's hero.
pub struct HeroParts {
    pub body: Handle<WorldAsset>,
    pub arm: Handle<WorldAsset>,
    pub leg: Handle<WorldAsset>,
    pub mantle: Handle<WorldAsset>,
    pub weapon: Handle<WorldAsset>,
}

impl GameAssets {
    pub fn spirit_parts(&self) -> HeroParts {
        HeroParts {
            body: self.soul_spirit.clone(),
            arm: self.soul_spirit_arm.clone(),
            leg: self.soul_spirit_leg.clone(),
            mantle: self.soul_spirit_mantle.clone(),
            weapon: self.soul_blade.clone(),
        }
    }
    pub fn bone_guard_parts(&self) -> HeroParts {
        HeroParts {
            body: self.bone_sentry.clone(),
            arm: self.bone_sentry_arm.clone(),
            leg: self.bone_sentry_leg.clone(),
            mantle: self.bone_sentry_mantle.clone(),
            weapon: self.soul_blade.clone(),
        }
    }

    pub fn hero_parts(&self, class: HeroClass) -> HeroParts {
        let (body, arm, leg, mantle, weapon) = match class {
            HeroClass::Warden => (
                &self.hero,
                &self.hero_arm,
                &self.hero_leg,
                &self.hero_mantle,
                &self.sword,
            ),
            HeroClass::Stalker => (
                &self.hero_stalker,
                &self.hero_stalker_arm,
                &self.hero_stalker_leg,
                &self.hero_stalker_mantle,
                &self.bone_bow,
            ),
            HeroClass::Soulbinder => (
                &self.hero_soulbinder,
                &self.hero_soulbinder_arm,
                &self.hero_soulbinder_leg,
                &self.hero_soulbinder_mantle,
                &self.soul_staff,
            ),
            HeroClass::Embercaller => (
                &self.hero_embercaller,
                &self.hero_embercaller_arm,
                &self.hero_embercaller_leg,
                &self.hero_embercaller_mantle,
                &self.ember_staff,
            ),
        };
        HeroParts {
            body: body.clone(),
            arm: arm.clone(),
            leg: leg.clone(),
            mantle: mantle.clone(),
            weapon: weapon.clone(),
        }
    }
}

#[derive(Resource, Default)]
pub struct AssetLoadingProgress {
    timer: Timer,
    checks: u64,
    pub timed_out: bool,
}

impl AssetLoadingProgress {
    fn tick(&mut self, delta: std::time::Duration) {
        self.timer.tick(delta);
        self.checks += 1;
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.timer.elapsed_secs()
    }

    pub fn finished(&self) -> bool {
        self.timer.is_finished()
    }

    pub fn checks(&self) -> u64 {
        self.checks
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AssetLoadSummary {
    pub total: usize,
    pub loaded: usize,
    pub loading: usize,
    pub failed: usize,
    pub not_loaded: usize,
}

impl AssetLoadSummary {
    pub fn percent_ready(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        (self.loaded as f32 / self.total as f32) * 100.0
    }

    pub fn settled(&self) -> bool {
        self.loaded + self.failed == self.total
    }

    pub fn ready(&self) -> bool {
        self.loaded == self.total && self.failed == 0
    }
}

pub struct GameAssetsPlugin;

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), load_assets)
            .add_systems(Update, finish_loading.run_if(in_state(GameState::Loading)));
    }
}

const LOADING_TIMEOUT_SECONDS: f32 = 45.0;

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(AssetLoadingProgress {
        timer: Timer::from_seconds(LOADING_TIMEOUT_SECONDS, TimerMode::Once),
        checks: 0,
        timed_out: false,
    });
    commands.insert_resource(GameAssets {
        hero: asset_server.load("models/hero.glb#Scene0"),
        hero_arm: asset_server.load("models/hero_arm.glb#Scene0"),
        hero_leg: asset_server.load("models/hero_leg.glb#Scene0"),
        hero_mantle: asset_server.load("models/hero_mantle.glb#Scene0"),
        hero_stalker: asset_server.load("models/hero_stalker.glb#Scene0"),
        hero_stalker_arm: asset_server.load("models/hero_stalker_arm.glb#Scene0"),
        hero_stalker_leg: asset_server.load("models/hero_stalker_leg.glb#Scene0"),
        hero_stalker_mantle: asset_server.load("models/hero_stalker_mantle.glb#Scene0"),
        hero_embercaller: asset_server.load("models/hero_embercaller.glb#Scene0"),
        hero_embercaller_arm: asset_server.load("models/hero_embercaller_arm.glb#Scene0"),
        hero_embercaller_leg: asset_server.load("models/hero_embercaller_leg.glb#Scene0"),
        hero_embercaller_mantle: asset_server.load("models/hero_embercaller_mantle.glb#Scene0"),
        hero_soulbinder: asset_server.load("models/hero_soulbinder.glb#Scene0"),
        hero_soulbinder_arm: asset_server.load("models/hero_soulbinder_arm.glb#Scene0"),
        hero_soulbinder_leg: asset_server.load("models/hero_soulbinder_leg.glb#Scene0"),
        hero_soulbinder_mantle: asset_server.load("models/hero_soulbinder_mantle.glb#Scene0"),
        soul_spirit: asset_server.load("models/soul_spirit.glb#Scene0"),
        soul_spirit_arm: asset_server.load("models/soul_spirit_arm.glb#Scene0"),
        soul_spirit_leg: asset_server.load("models/soul_spirit_leg.glb#Scene0"),
        soul_spirit_mantle: asset_server.load("models/soul_spirit_mantle.glb#Scene0"),
        bone_sentry: asset_server.load("models/bone_sentry.glb#Scene0"),
        bone_sentry_arm: asset_server.load("models/bone_sentry_arm.glb#Scene0"),
        bone_sentry_leg: asset_server.load("models/bone_sentry_leg.glb#Scene0"),
        bone_sentry_mantle: asset_server.load("models/bone_sentry_mantle.glb#Scene0"),
        soul_staff: asset_server.load("models/soul_staff.glb#Scene0"),
        soul_blade: asset_server.load("models/soul_blade.glb#Scene0"),
        skeleton: asset_server.load("models/skeleton.glb#Scene0"),
        ashbone_guard: asset_server.load("models/ashbone_guard.glb#Scene0"),
        cultist: asset_server.load("models/cultist.glb#Scene0"),
        butcher: asset_server.load("models/butcher.glb#Scene0"),
        bone_stalker: asset_server.load("models/bone_stalker.glb#Scene0"),
        ashen_marksman: asset_server.load("models/ashen_marksman.glb#Scene0"),
        treasure_imp: asset_server.load("models/treasure_imp.glb#Scene0"),
        seal_warden: asset_server.load("models/seal_warden.glb#Scene0"),
        keeper: asset_server.load("models/keeper.glb#Scene0"),
        nemesis: asset_server.load("models/nemesis.glb#Scene0"),
        hollow_thrall: asset_server.load("models/hollow_thrall.glb#Scene0"),
        hollow_stalker: asset_server.load("models/hollow_stalker.glb#Scene0"),
        choir_acolyte: asset_server.load("models/choir_acolyte.glb#Scene0"),
        choir_marksman: asset_server.load("models/choir_marksman.glb#Scene0"),
        hollow_reaver: asset_server.load("models/hollow_reaver.glb#Scene0"),
        bell_warden: asset_server.load("models/bell_warden.glb#Scene0"),
        hollow_matron: asset_server.load("models/hollow_matron.glb#Scene0"),
        sword: asset_server.load("models/sword.glb#Scene0"),
        bone_bow: asset_server.load("models/bone_bow.glb#Scene0"),
        ember_staff: asset_server.load("models/ember_staff.glb#Scene0"),
        bone_arrow: asset_server.load("models/bone_arrow.glb#Scene0"),
        ember_bolt: asset_server.load("models/ember_bolt.glb#Scene0"),
        ash_lance: asset_server.load("models/ash_lance.glb#Scene0"),
        sunder_wave: asset_server.load("models/sunder_wave.glb#Scene0"),
        ground_sigil: asset_server.load("models/ground_sigil.glb#Scene0"),
        chest: asset_server.load("models/chest.glb#Scene0"),
        altar: asset_server.load("models/altar.glb#Scene0"),
        soul_font: asset_server.load("models/soul_font.glb#Scene0"),
        quartermaster: asset_server.load("models/quartermaster.glb#Scene0"),
        fortune_shrine: asset_server.load("models/fortune_shrine.glb#Scene0"),
        storm_shrine: asset_server.load("models/storm_shrine.glb#Scene0"),
        healing_well: asset_server.load("models/healing_well.glb#Scene0"),
        cursed_shrine: asset_server.load("models/cursed_shrine.glb#Scene0"),
        blood_obelisk: asset_server.load("models/blood_obelisk.glb#Scene0"),
        reliquary_vault: asset_server.load("models/reliquary_vault.glb#Scene0"),
        ember_rift_prop: asset_server.load("models/ember_rift_prop.glb#Scene0"),
        ashen_pylon: asset_server.load("models/ashen_pylon.glb#Scene0"),
        lore_page: asset_server.load("models/lore_page.glb#Scene0"),
        breakable_urn: asset_server.load("models/breakable_urn.glb#Scene0"),
        breakable_coffer: asset_server.load("models/breakable_coffer.glb#Scene0"),
        slash_arc: asset_server.load("models/slash_arc.glb#Scene0"),
        hit_spark: asset_server.load("models/hit_spark.glb#Scene0"),
        contact_steel: asset_server.load("models/contact_steel.glb#Scene0"),
        contact_expose: asset_server.load("models/contact_expose.glb#Scene0"),
        contact_blood: asset_server.load("models/contact_blood.glb#Scene0"),
        contact_cleanse: asset_server.load("models/contact_cleanse.glb#Scene0"),
        contact_reap: asset_server.load("models/contact_reap.glb#Scene0"),
        contact_ember: asset_server.load("models/contact_ember.glb#Scene0"),
        contact_frost: asset_server.load("models/contact_frost.glb#Scene0"),
        proc_vigor: asset_server.load("models/proc_vigor.glb#Scene0"),
        proc_focus: asset_server.load("models/proc_focus.glb#Scene0"),

        bone_shatter: asset_server.load("models/bone_shatter.glb#Scene0"),
        bone_impact: asset_server.load("models/bone_impact.glb#Scene0"),
        blood_spray: asset_server.load("models/blood_spray.glb#Scene0"),
        execution_burst: asset_server.load("models/execution_burst.glb#Scene0"),
        arcane_impact: asset_server.load("models/arcane_impact.glb#Scene0"),
        holy_impact: asset_server.load("models/holy_impact.glb#Scene0"),
        ember_impact: asset_server.load("models/ember_impact.glb#Scene0"),
        frost_impact: asset_server.load("models/frost_impact.glb#Scene0"),
        void_impact: asset_server.load("models/void_impact.glb#Scene0"),
        frenzy_impact: asset_server.load("models/frenzy_impact.glb#Scene0"),
        vampiric_siphon: asset_server.load("models/vampiric_siphon.glb#Scene0"),
        desecrator_burst: asset_server.load("models/desecrator_burst.glb#Scene0"),
        guard_clash: asset_server.load("models/guard_clash.glb#Scene0"),
        armor_break: asset_server.load("models/armor_break.glb#Scene0"),
        soul_ward_hit: asset_server.load("models/soul_ward_hit.glb#Scene0"),
        hit_bone_rune: asset_server.load("models/hit_bone_rune.glb#Scene0"),
        hit_bone_lock: asset_server.load("models/hit_bone_lock.glb#Scene0"),
        marrow_flash: asset_server.load("models/marrow_flash.glb#Scene0"),
        bone_fracture_echo: asset_server.load("models/bone_fracture_echo.glb#Scene0"),
        elite_affix_break: asset_server.load("models/elite_affix_break.glb#Scene0"),
        shadow_burst: asset_server.load("models/shadow_burst.glb#Scene0"),
        headshot_burst: asset_server.load("models/headshot_burst.glb#Scene0"),
        crit_bone_crown: asset_server.load("models/crit_bone_crown.glb#Scene0"),
        crit_burst: asset_server.load("models/crit_burst.glb#Scene0"),
        stagger_burst: asset_server.load("models/stagger_burst.glb#Scene0"),
        shadow_trail: asset_server.load("models/shadow_trail.glb#Scene0"),
        health_pickup: asset_server.load("models/health_pickup.glb#Scene0"),
        fury_pickup: asset_server.load("models/fury_pickup.glb#Scene0"),
        death_remains: asset_server.load("models/death_remains.glb#Scene0"),
        loot_prism: asset_server.load("models/loot_prism.glb#Scene0"),
        objective_sigil: asset_server.load("models/objective_sigil.glb#Scene0"),
        ember_vent: asset_server.load("models/ember_vent.glb#Scene0"),
        soul_fissure: asset_server.load("models/soul_fissure.glb#Scene0"),
        boss_summon_portal: asset_server.load("models/boss_summon_portal.glb#Scene0"),
        keeper_cleaver_wave: asset_server.load("models/keeper_cleaver_wave.glb#Scene0"),
        keeper_chain_cage: asset_server.load("models/keeper_chain_cage.glb#Scene0"),
        matron_soul_lance: asset_server.load("models/matron_soul_lance.glb#Scene0"),
        matron_crescendo: asset_server.load("models/matron_crescendo.glb#Scene0"),
        affix_ember_aura: asset_server.load("models/affix_ember_aura.glb#Scene0"),
        affix_arcane_aura: asset_server.load("models/affix_arcane_aura.glb#Scene0"),
        affix_frost_aura: asset_server.load("models/affix_frost_aura.glb#Scene0"),
        affix_blood_aura: asset_server.load("models/affix_blood_aura.glb#Scene0"),
        affix_ward_aura: asset_server.load("models/affix_ward_aura.glb#Scene0"),
        ashen_ground_albedo: load_repeating_texture(
            &asset_server,
            "images/generated/world/ashen-ground-albedo.jpg",
        ),
        hollow_ground_albedo: load_repeating_texture(
            &asset_server,
            "images/generated/world/hollow-ground-albedo.jpg",
        ),
        ashen_road_albedo: load_repeating_texture(
            &asset_server,
            "images/generated/world/ashen-road-albedo.jpg",
        ),
        hollow_road_albedo: load_repeating_texture(
            &asset_server,
            "images/generated/world/hollow-road-albedo.jpg",
        ),
        scenery_rock: asset_server.load("models/scenery_rock.glb#Scene0"),
        scenery_dead_tree: asset_server.load("models/scenery_dead_tree.glb#Scene0"),
        scenery_crystal: asset_server.load("models/scenery_crystal.glb#Scene0"),
        scenery_ruin: asset_server.load("models/scenery_ruin.glb#Scene0"),
        sanctum_gate: asset_server.load("models/sanctum_gate.glb#Scene0"),
        stone_detail: load_repeating_texture(&asset_server, "textures/materials/stone-detail.jpg"),
        ui_main_menu: asset_server.load("images/generated/ui/main-menu-reliquary-v2.png"),
        ui_panel_frame: asset_server.load("images/generated/ui/panel-frame-reliquary.png"),
        ui_slot_frame: asset_server.load("images/generated/ui/slot-frame-reliquary.png"),
        ui_resource_orb_frame: asset_server
            .load("images/generated/ui/resource-orb-frame-reliquary.png"),
        ui_skill_icons: asset_server.load("images/generated/ui/skill-icons-reliquary.png"),
        ui_class_skill_icons: asset_server
            .load("images/generated/ui/class-skill-icons-reliquary.png"),
        ui_soulbinder_skill_icons: asset_server
            .load("images/generated/ui/soulbinder-skill-icons.png"),
        ui_class_portraits: [
            asset_server.load("images/generated/ui/class-portrait-warden.png"),
            asset_server.load("images/generated/ui/class-portrait-stalker.png"),
            asset_server.load("images/generated/ui/class-portrait-embercaller.png"),
            asset_server.load("images/generated/ui/class-portrait-soulbinder.png"),
        ],
        ui_gear_icons: asset_server.load("images/generated/ui/gear-icons-reliquary.png"),
        ui_talent_sigils: asset_server.load("images/generated/ui/talent-sigils-reliquary.png"),
        ui_utility_icons: asset_server.load("images/generated/ui/utility-icons-reliquary.png"),
        ui_font_regular: asset_server.load("fonts/CrimsonPro.ttf"),
        ui_font_bold: asset_server.load("fonts/Cinzel.ttf"),
        ui_font_chinese: asset_server.load("fonts/NotoSansSC.ttf"),
    });
}

fn finish_loading(
    assets: Option<Res<GameAssets>>,
    asset_server: Res<AssetServer>,
    headless_smoke: Option<Res<HeadlessSmokeExit>>,
    time: Res<Time>,
    mut loading_progress: ResMut<AssetLoadingProgress>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(assets) = assets else {
        return;
    };

    loading_progress.tick(time.delta());
    if headless_smoke.is_some() {
        // The logic smoke intentionally has no renderer. Textured GLBs can
        // continue decoding in the asset task pool, but gameplay validation
        // must not depend on how much wall time those workers receive during a
        // fixed number of extremely fast headless updates.
        info!("headless asset handles registered, entering MainMenu");
        next_state.set(GameState::MainMenu);
        return;
    }
    let summary = game_assets_load_summary(&asset_server, &assets);

    if summary.ready() {
        info!("asset loading complete, entering MainMenu");
        next_state.set(GameState::MainMenu);
        return;
    }

    if summary.settled() {
        warn!(
            "asset loading settled but incomplete ({} failed / {} total). Entering MainMenu anyway.",
            summary.failed, summary.total
        );
        next_state.set(GameState::MainMenu);
        return;
    }

    if loading_progress.finished() {
        loading_progress.timed_out = true;
        warn!(
            "asset loading timeout after {:.1}s ({} loaded, {} loading, {} failed). Entering MainMenu to avoid permanent splash.",
            loading_progress.elapsed_secs(),
            summary.loaded,
            summary.loading + summary.not_loaded,
            summary.failed
        );
        next_state.set(GameState::MainMenu);
        return;
    }
    let pending = summary.loading + summary.not_loaded;
    if pending <= 3 || loading_progress.checks.is_multiple_of(60) {
        debug!(
            "loading assets: {:.1}% ready={} loading={} failed={} checks={}",
            summary.percent_ready(),
            summary.loaded,
            pending,
            summary.failed,
            loading_progress.checks
        );
    }
}

fn collect_game_asset_handles(assets: &GameAssets) -> Vec<Handle<WorldAsset>> {
    vec![
        assets.hero_soulbinder.clone(),
        assets.hero_soulbinder_arm.clone(),
        assets.hero_soulbinder_leg.clone(),
        assets.hero_soulbinder_mantle.clone(),
        assets.soul_spirit.clone(),
        assets.soul_spirit_arm.clone(),
        assets.soul_spirit_leg.clone(),
        assets.soul_spirit_mantle.clone(),
        assets.bone_sentry.clone(),
        assets.bone_sentry_arm.clone(),
        assets.bone_sentry_leg.clone(),
        assets.bone_sentry_mantle.clone(),
        assets.soul_staff.clone(),
        assets.soul_blade.clone(),
        assets.hero.clone(),
        assets.hero_arm.clone(),
        assets.hero_leg.clone(),
        assets.hero_mantle.clone(),
        assets.hero_stalker.clone(),
        assets.hero_stalker_arm.clone(),
        assets.hero_stalker_leg.clone(),
        assets.hero_stalker_mantle.clone(),
        assets.hero_embercaller.clone(),
        assets.hero_embercaller_arm.clone(),
        assets.hero_embercaller_leg.clone(),
        assets.hero_embercaller_mantle.clone(),
        assets.bone_bow.clone(),
        assets.ember_staff.clone(),
        assets.bone_arrow.clone(),
        assets.ember_bolt.clone(),
        assets.ash_lance.clone(),
        assets.sunder_wave.clone(),
        assets.ground_sigil.clone(),
        assets.skeleton.clone(),
        assets.ashbone_guard.clone(),
        assets.cultist.clone(),
        assets.butcher.clone(),
        assets.bone_stalker.clone(),
        assets.ashen_marksman.clone(),
        assets.treasure_imp.clone(),
        assets.seal_warden.clone(),
        assets.keeper.clone(),
        assets.nemesis.clone(),
        assets.hollow_thrall.clone(),
        assets.hollow_stalker.clone(),
        assets.choir_acolyte.clone(),
        assets.choir_marksman.clone(),
        assets.hollow_reaver.clone(),
        assets.bell_warden.clone(),
        assets.hollow_matron.clone(),
        assets.sword.clone(),
        assets.chest.clone(),
        assets.altar.clone(),
        assets.soul_font.clone(),
        assets.quartermaster.clone(),
        assets.fortune_shrine.clone(),
        assets.storm_shrine.clone(),
        assets.healing_well.clone(),
        assets.cursed_shrine.clone(),
        assets.blood_obelisk.clone(),
        assets.reliquary_vault.clone(),
        assets.ember_rift_prop.clone(),
        assets.ashen_pylon.clone(),
        assets.lore_page.clone(),
        assets.breakable_urn.clone(),
        assets.breakable_coffer.clone(),
        assets.slash_arc.clone(),
        assets.hit_spark.clone(),
        assets.contact_steel.clone(),
        assets.contact_expose.clone(),
        assets.contact_blood.clone(),
        assets.contact_cleanse.clone(),
        assets.contact_reap.clone(),
        assets.contact_ember.clone(),
        assets.contact_frost.clone(),
        assets.proc_vigor.clone(),
        assets.proc_focus.clone(),
        assets.bone_shatter.clone(),
        assets.bone_impact.clone(),
        assets.blood_spray.clone(),
        assets.execution_burst.clone(),
        assets.arcane_impact.clone(),
        assets.holy_impact.clone(),
        assets.ember_impact.clone(),
        assets.frost_impact.clone(),
        assets.void_impact.clone(),
        assets.frenzy_impact.clone(),
        assets.vampiric_siphon.clone(),
        assets.desecrator_burst.clone(),
        assets.guard_clash.clone(),
        assets.armor_break.clone(),
        assets.soul_ward_hit.clone(),
        assets.hit_bone_rune.clone(),
        assets.hit_bone_lock.clone(),
        assets.marrow_flash.clone(),
        assets.bone_fracture_echo.clone(),
        assets.elite_affix_break.clone(),
        assets.shadow_burst.clone(),
        assets.headshot_burst.clone(),
        assets.crit_bone_crown.clone(),
        assets.crit_burst.clone(),
        assets.stagger_burst.clone(),
        assets.shadow_trail.clone(),
        assets.scenery_rock.clone(),
        assets.scenery_dead_tree.clone(),
        assets.scenery_crystal.clone(),
        assets.scenery_ruin.clone(),
        assets.sanctum_gate.clone(),
        assets.health_pickup.clone(),
        assets.fury_pickup.clone(),
        assets.death_remains.clone(),
        assets.loot_prism.clone(),
        assets.objective_sigil.clone(),
        assets.ember_vent.clone(),
        assets.soul_fissure.clone(),
        assets.boss_summon_portal.clone(),
        assets.keeper_cleaver_wave.clone(),
        assets.keeper_chain_cage.clone(),
        assets.matron_soul_lance.clone(),
        assets.matron_crescendo.clone(),
        assets.affix_ember_aura.clone(),
        assets.affix_arcane_aura.clone(),
        assets.affix_frost_aura.clone(),
        assets.affix_blood_aura.clone(),
        assets.affix_ward_aura.clone(),
    ]
}

fn collect_visual_asset_handles(assets: &GameAssets) -> Vec<UntypedHandle> {
    vec![
        assets.ashen_ground_albedo.clone().untyped(),
        assets.hollow_ground_albedo.clone().untyped(),
        assets.ashen_road_albedo.clone().untyped(),
        assets.hollow_road_albedo.clone().untyped(),
        assets.stone_detail.clone().untyped(),
        assets.ui_main_menu.clone().untyped(),
        assets.ui_panel_frame.clone().untyped(),
        assets.ui_slot_frame.clone().untyped(),
        assets.ui_resource_orb_frame.clone().untyped(),
        assets.ui_skill_icons.clone().untyped(),
        assets.ui_class_skill_icons.clone().untyped(),
        assets.ui_soulbinder_skill_icons.clone().untyped(),
        assets.ui_class_portraits[3].clone().untyped(),
        assets.ui_class_portraits[0].clone().untyped(),
        assets.ui_class_portraits[1].clone().untyped(),
        assets.ui_class_portraits[2].clone().untyped(),
        assets.ui_gear_icons.clone().untyped(),
        assets.ui_talent_sigils.clone().untyped(),
        assets.ui_utility_icons.clone().untyped(),
        assets.ui_font_regular.clone().untyped(),
        assets.ui_font_bold.clone().untyped(),
        assets.ui_font_chinese.clone().untyped(),
    ]
}

fn asset_has_failed(asset_server: &AssetServer, id: UntypedAssetId) -> bool {
    let root_failed = matches!(asset_server.get_load_state(id), Some(LoadState::Failed(_)));
    let dep_failed = matches!(
        asset_server.get_dependency_load_state(id),
        Some(state) if state.is_failed()
    );
    let rec_dep_failed = matches!(
        asset_server.get_recursive_dependency_load_state(id),
        Some(state) if state.is_failed()
    );
    root_failed || dep_failed || rec_dep_failed
}

fn record_asset_load_state(
    asset_server: &AssetServer,
    id: UntypedAssetId,
    summary: &mut AssetLoadSummary,
) {
    if asset_server.is_loaded_with_dependencies(id) {
        summary.loaded += 1;
        return;
    }

    if asset_has_failed(asset_server, id) {
        summary.failed += 1;
        return;
    }

    if matches!(
        asset_server.get_load_state(id),
        Some(state) if state.is_loading()
    ) || matches!(
        asset_server.get_dependency_load_state(id),
        Some(state) if state.is_loading()
    ) || matches!(
        asset_server.get_recursive_dependency_load_state(id),
        Some(state) if state.is_loading()
    ) {
        summary.loading += 1;
        return;
    }

    summary.not_loaded += 1;
}

pub fn game_assets_load_summary(
    asset_server: &AssetServer,
    assets: &GameAssets,
) -> AssetLoadSummary {
    let world_handles = collect_game_asset_handles(assets);
    let visual_handles = collect_visual_asset_handles(assets);
    let mut summary = AssetLoadSummary {
        total: world_handles.len() + visual_handles.len(),
        ..Default::default()
    };

    for handle in world_handles {
        record_asset_load_state(asset_server, handle.id().untyped(), &mut summary);
    }
    for handle in visual_handles {
        record_asset_load_state(asset_server, handle.id(), &mut summary);
    }

    summary
}

#[allow(dead_code)]
pub fn game_assets_ready(asset_server: &AssetServer, assets: &GameAssets) -> bool {
    game_assets_load_summary(asset_server, assets).ready()
}
