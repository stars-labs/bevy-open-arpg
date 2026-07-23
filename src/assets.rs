use crate::GameState;
use bevy::asset::LoadState;
use bevy::prelude::*;
use bevy::time::{Timer, TimerMode};
use bevy::world_serialization::WorldAsset;

#[derive(Resource)]
pub struct GameAssets {
    pub hero: Handle<WorldAsset>,
    pub skeleton: Handle<WorldAsset>,
    pub cultist: Handle<WorldAsset>,
    pub butcher: Handle<WorldAsset>,
    pub sword: Handle<WorldAsset>,
    pub chest: Handle<WorldAsset>,
    pub altar: Handle<WorldAsset>,
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
    pub loot_prism: Handle<WorldAsset>,
    pub objective_sigil: Handle<WorldAsset>,
    pub ember_vent: Handle<WorldAsset>,
    pub boss_summon_portal: Handle<WorldAsset>,
    pub affix_ember_aura: Handle<WorldAsset>,
    pub affix_arcane_aura: Handle<WorldAsset>,
    pub affix_frost_aura: Handle<WorldAsset>,
    pub affix_blood_aura: Handle<WorldAsset>,
    pub affix_ward_aura: Handle<WorldAsset>,
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
        skeleton: asset_server.load("models/skeleton.glb#Scene0"),
        cultist: asset_server.load("models/cultist.glb#Scene0"),
        butcher: asset_server.load("models/butcher.glb#Scene0"),
        sword: asset_server.load("models/sword.glb#Scene0"),
        chest: asset_server.load("models/chest.glb#Scene0"),
        altar: asset_server.load("models/altar.glb#Scene0"),
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
        loot_prism: asset_server.load("models/loot_prism.glb#Scene0"),
        objective_sigil: asset_server.load("models/objective_sigil.glb#Scene0"),
        ember_vent: asset_server.load("models/ember_vent.glb#Scene0"),
        boss_summon_portal: asset_server.load("models/boss_summon_portal.glb#Scene0"),
        affix_ember_aura: asset_server.load("models/affix_ember_aura.glb#Scene0"),
        affix_arcane_aura: asset_server.load("models/affix_arcane_aura.glb#Scene0"),
        affix_frost_aura: asset_server.load("models/affix_frost_aura.glb#Scene0"),
        affix_blood_aura: asset_server.load("models/affix_blood_aura.glb#Scene0"),
        affix_ward_aura: asset_server.load("models/affix_ward_aura.glb#Scene0"),
    });
}

fn finish_loading(
    assets: Option<Res<GameAssets>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut loading_progress: ResMut<AssetLoadingProgress>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(assets) = assets else {
        return;
    };

    loading_progress.tick(time.delta());
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
        assets.hero.clone(),
        assets.skeleton.clone(),
        assets.cultist.clone(),
        assets.butcher.clone(),
        assets.sword.clone(),
        assets.chest.clone(),
        assets.altar.clone(),
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
        assets.loot_prism.clone(),
        assets.objective_sigil.clone(),
        assets.ember_vent.clone(),
        assets.boss_summon_portal.clone(),
        assets.affix_ember_aura.clone(),
        assets.affix_arcane_aura.clone(),
        assets.affix_frost_aura.clone(),
        assets.affix_blood_aura.clone(),
        assets.affix_ward_aura.clone(),
    ]
}

fn asset_has_failed(asset_server: &AssetServer, handle: &Handle<WorldAsset>) -> bool {
    let root_failed = matches!(
        asset_server.get_load_state(handle),
        Some(LoadState::Failed(_))
    );
    let dep_failed =
        matches!(asset_server.get_dependency_load_state(handle), Some(state) if state.is_failed());
    let rec_dep_failed = matches!(
        asset_server.get_recursive_dependency_load_state(handle),
        Some(state) if state.is_failed()
    );
    root_failed || dep_failed || rec_dep_failed
}

pub fn game_assets_load_summary(
    asset_server: &AssetServer,
    assets: &GameAssets,
) -> AssetLoadSummary {
    let handles = collect_game_asset_handles(assets);
    let mut summary = AssetLoadSummary {
        total: handles.len(),
        ..Default::default()
    };

    for handle in &handles {
        if asset_server.is_loaded_with_dependencies(handle) {
            summary.loaded += 1;
            continue;
        }

        if asset_has_failed(asset_server, handle) {
            summary.failed += 1;
            continue;
        }

        if matches!(
            asset_server.get_load_state(handle),
            Some(state) if state.is_loading()
        ) || matches!(
            asset_server.get_dependency_load_state(handle),
            Some(state) if state.is_loading()
        ) || matches!(
            asset_server.get_recursive_dependency_load_state(handle),
            Some(state) if state.is_loading()
        ) {
            summary.loading += 1;
            continue;
        }

        summary.not_loaded += 1;
    }

    summary
}

#[allow(dead_code)]
pub fn game_assets_ready(asset_server: &AssetServer, assets: &GameAssets) -> bool {
    let handles = [
        &assets.hero,
        &assets.skeleton,
        &assets.cultist,
        &assets.butcher,
        &assets.sword,
        &assets.chest,
        &assets.altar,
        &assets.quartermaster,
        &assets.fortune_shrine,
        &assets.storm_shrine,
        &assets.healing_well,
        &assets.cursed_shrine,
        &assets.blood_obelisk,
        &assets.reliquary_vault,
        &assets.ember_rift_prop,
        &assets.ashen_pylon,
        &assets.lore_page,
        &assets.breakable_urn,
        &assets.breakable_coffer,
        &assets.slash_arc,
        &assets.hit_spark,
        &assets.bone_shatter,
        &assets.bone_impact,
        &assets.blood_spray,
        &assets.execution_burst,
        &assets.arcane_impact,
        &assets.holy_impact,
        &assets.ember_impact,
        &assets.frost_impact,
        &assets.void_impact,
        &assets.frenzy_impact,
        &assets.vampiric_siphon,
        &assets.desecrator_burst,
        &assets.guard_clash,
        &assets.armor_break,
        &assets.soul_ward_hit,
        &assets.hit_bone_rune,
        &assets.hit_bone_lock,
        &assets.marrow_flash,
        &assets.bone_fracture_echo,
        &assets.elite_affix_break,
        &assets.shadow_burst,
        &assets.headshot_burst,
        &assets.crit_bone_crown,
        &assets.crit_burst,
        &assets.stagger_burst,
        &assets.shadow_trail,
        &assets.loot_prism,
        &assets.objective_sigil,
        &assets.ember_vent,
        &assets.boss_summon_portal,
        &assets.affix_ember_aura,
        &assets.affix_arcane_aura,
        &assets.affix_frost_aura,
        &assets.affix_blood_aura,
        &assets.affix_ward_aura,
    ];
    handles
        .into_iter()
        .all(|handle| asset_server.is_loaded_with_dependencies(handle))
}
